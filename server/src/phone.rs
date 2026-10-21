//! The table over phones: the engine's `table::Io`, where waking, telling and
//! asking a player become entries and prompts on that player's phone.
//!
//! Asking blocks the game thread until the player answers or their time runs
//! out; then the engine's default is taken (the first candidates, no, the
//! first character), so a phone left on a table never stalls the game.

use std::cell;
use std::rc;
use std::sync;
use std::time;

use botc_narrator::character;
use botc_narrator::event;
use botc_narrator::game;
use botc_narrator::info;
use botc_narrator::player;
use botc_narrator::storyteller;
use botc_narrator::table;

use crate::inbox;
use crate::room;

pub struct Table {
    room: sync::Arc<room::Room>,
    inbox: rc::Rc<cell::RefCell<inbox::Inbox>>,
    /// How long a player has to answer.
    answer: time::Duration,
    phase: game::Phase,
    /// Night deaths (true) and revivals (false), announced when the day begins.
    unannounced: Vec<(player::Id, bool)>,
    next_prompt: u64,
}

impl Table {
    pub fn new(
        room: sync::Arc<room::Room>,
        inbox: rc::Rc<cell::RefCell<inbox::Inbox>>,
        answer: time::Duration,
    ) -> Self {
        Table {
            room,
            inbox,
            answer,
            phase: game::Phase::Setup,
            unannounced: Vec::new(),
            next_prompt: 1,
        }
    }

    fn log(
        &self,
        audience: room::Audience,
        text: String,
        players: Vec<String>,
        characters: Vec<&'static str>,
    ) {
        let phase = self.phase;
        self.room.update(|state| {
            if let Some(game) = &mut state.game {
                game.log.push(room::Entry {
                    phase,
                    audience,
                    text,
                    players,
                    characters,
                });
            }
        });
    }

    fn private(&self, player: &player::Id, text: String) {
        self.log(
            room::Audience::Player(player.to_string()),
            text,
            Vec::new(),
            Vec::new(),
        );
    }

    /// By day, a player's answer is public unless they are privately visiting
    /// the storyteller.
    fn answered(
        &self,
        player: &player::Id,
        text: String,
        players: Vec<String>,
        characters: Vec<&'static str>,
    ) {
        let awake = self.room.read(|s| {
            s.game
                .as_ref()
                .is_some_and(|g| g.awake.iter().any(|a| a == player.as_str()))
        });
        let audience = match self.phase {
            game::Phase::Day(_) if !awake => room::Audience::Public,
            _ => room::Audience::Player(player.to_string()),
        };
        self.log(audience, text, players, characters);
    }

    /// Shows `player` a prompt and waits for a valid answer, or `None` once
    /// their time is up (or the room closed).
    fn prompt(
        &mut self,
        player: &player::Id,
        text: &str,
        kind: room::PromptKind,
    ) -> Option<inbox::Answer> {
        let id = self.next_prompt;
        self.next_prompt += 1;
        let name = player.to_string();
        let prompt = room::Prompt {
            id,
            player: name.clone(),
            text: text.to_string(),
            kind: kind.clone(),
            seconds: self.answer.as_secs(),
        };
        self.room.update(|state| {
            if let Some(game) = &mut state.game {
                game.prompts.retain(|p| p.player != name);
                game.prompts.push(prompt);
            }
        });
        tracing::debug!(room = %self.room.code, %player, prompt = id, "asking");
        let deadline = time::Instant::now() + self.answer;
        let answer = self
            .inbox
            .borrow_mut()
            .wait(Some(deadline), |input| match input {
                inbox::Input::Answer {
                    prompt,
                    player: from,
                    answer,
                } if *prompt == id && *from == name && valid(&kind, answer) => inbox::Verdict::Take,
                other => inbox::keep_commands(other),
            });
        self.room.update(|state| {
            if let Some(game) = &mut state.game {
                game.prompts.retain(|p| p.id != id);
            }
        });
        match answer {
            Some(inbox::Input::Answer { answer, .. }) => Some(answer),
            _ => {
                tracing::info!(room = %self.room.code, %player, prompt = id, "no answer in time; taking the default");
                None
            }
        }
    }

    fn choose_players(
        &mut self,
        player: &player::Id,
        prompt: &str,
        candidates: &[player::Id],
        min: usize,
        max: usize,
    ) -> Vec<player::Id> {
        let kind = room::PromptKind::Players {
            candidates: candidates.iter().map(ToString::to_string).collect(),
            min,
            max,
        };
        let chosen: Vec<player::Id> = match self.prompt(player, prompt, kind) {
            Some(inbox::Answer::Players(names)) => names.into_iter().map(player::Id::new).collect(),
            _ => candidates.iter().take(min).cloned().collect(),
        };
        let names: Vec<String> = chosen.iter().map(ToString::to_string).collect();
        let text = if names.is_empty() {
            format!("{player} chose no-one")
        } else {
            format!("{player} chose {}", names.join(", "))
        };
        self.answered(player, text, names, Vec::new());
        chosen
    }

    fn yes_no(&mut self, player: &player::Id, prompt: &str) -> bool {
        let yes = matches!(
            self.prompt(player, prompt, room::PromptKind::YesNo),
            Some(inbox::Answer::YesNo(true))
        );
        let text = format!("{prompt} {}", if yes { "Yes" } else { "No" });
        self.answered(player, text, Vec::new(), Vec::new());
        yes
    }

    /// The deaths and revivals of the night, now that it is day.
    fn announce_night(&mut self) {
        for (player, died) in std::mem::take(&mut self.unannounced) {
            let text = if died {
                format!("{player} died in the night")
            } else {
                format!("{player} is alive again")
            };
            self.announce(&player, died, text);
        }
    }

    /// Tells the table that `player` died (or lives again): only now do the
    /// phones show it.
    fn announce(&self, player: &player::Id, died: bool, text: String) {
        let name = player.to_string();
        let phase = self.phase;
        self.room.update(|state| {
            if let Some(game) = &mut state.game {
                game.known_dead.retain(|d| *d != name);
                if died {
                    game.known_dead.push(name.clone());
                }
                game.log.push(room::Entry {
                    phase,
                    audience: room::Audience::Public,
                    text,
                    players: vec![name],
                    characters: Vec::new(),
                });
            }
        });
    }

    /// The entries an event makes: public ones for what the table sees, the
    /// rest the storyteller's.
    fn record(&mut self, event: &event::Event) {
        let text = event.to_string();
        let players: Vec<String> = event.players().iter().map(ToString::to_string).collect();
        let characters = event.characters();
        let public = |table: &Self, text: String| {
            table.log(
                room::Audience::Public,
                text,
                players.clone(),
                characters.clone(),
            );
        };
        let secret = |table: &Self| {
            table.log(
                room::Audience::Storyteller,
                text.clone(),
                players.clone(),
                characters.clone(),
            );
        };
        match event {
            event::Event::PhaseBegan(phase) => {
                public(self, text.clone());
                if let game::Phase::Day(_) = phase {
                    self.announce_night();
                }
            }
            event::Event::Died { player, .. } => {
                // The cause is the storyteller's; the table only learns who died.
                secret(self);
                if let game::Phase::Night(_) = self.phase {
                    self.unannounced.push((player.clone(), true));
                } else {
                    self.announce(player, true, format!("{player} dies"));
                }
            }
            event::Event::Revived(player) => {
                secret(self);
                if let game::Phase::Night(_) = self.phase {
                    // Died and came back tonight: the table hears of neither.
                    let before = self.unannounced.len();
                    self.unannounced.retain(|(p, died)| !(p == player && *died));
                    if self.unannounced.len() == before {
                        self.unannounced.push((player.clone(), false));
                    }
                } else {
                    self.announce(player, false, format!("{player} is alive again"));
                }
            }
            event::Event::Won(alignment) => {
                // A night kill can end the game before dawn: the winner is
                // set first, so no phone sees a night death announced
                // before the game is over.
                let winner = *alignment;
                self.room.update(|state| {
                    if let Some(game) = &mut state.game {
                        game.winner = Some(winner);
                    }
                });
                self.announce_night();
                public(self, text.clone());
            }
            event::Event::Nominated { .. }
            | event::Event::Executed(_)
            | event::Event::Voted { .. }
            | event::Event::Announced(_)
            | event::Event::Claimed { .. }
            | event::Event::SeatsSwapped(..)
            | event::Event::GhostVoteSpent(_) => public(self, text.clone()),
            // The player is shown their new token.
            event::Event::Transformed { player, into } => {
                secret(self);
                self.log(
                    room::Audience::Player(player.to_string()),
                    format!("You are now the {into}"),
                    Vec::new(),
                    vec![*into],
                );
            }
            event::Event::Believes { player, character } => {
                secret(self);
                self.log(
                    room::Audience::Player(player.to_string()),
                    format!("You are the {character}"),
                    Vec::new(),
                    vec![*character],
                );
            }
            event::Event::AlignmentChanged { player, alignment } => {
                secret(self);
                self.private(player, format!("You are now {alignment}"));
            }
            _ => secret(self),
        }
    }
}

/// Whether `answer` answers a prompt of this kind.
fn valid(kind: &room::PromptKind, answer: &inbox::Answer) -> bool {
    match (kind, answer) {
        (
            room::PromptKind::Players {
                candidates,
                min,
                max,
            },
            inbox::Answer::Players(names),
        ) => {
            (*min..=*max).contains(&names.len())
                && storyteller::is_selection(names, names.len(), |n| candidates.contains(n))
        }
        (room::PromptKind::Character { options }, inbox::Answer::Option(i)) => *i < options.len(),
        (room::PromptKind::YesNo, inbox::Answer::YesNo(_)) => true,
        _ => false,
    }
}

/// A typed question as the player is asked it.
fn question_text(question: &table::Question) -> String {
    match question {
        table::Question::UseAbility(_) | table::Question::Claim(_) => {
            format!("Do you want to {question}")
        }
        table::Question::SaidWord(_) | table::Question::LostRoshambo { .. } => {
            format!("Have you {question}")
        }
    }
}

impl table::Io for Table {
    fn wake(&mut self, player: &player::Id) {
        let name = player.to_string();
        self.room.update(|state| {
            if let Some(game) = &mut state.game
                && !game.awake.contains(&name)
            {
                game.awake.push(name);
            }
        });
        let text = match self.phase {
            game::Phase::Day(_) => "The storyteller calls you over",
            _ => "You wake up",
        };
        self.private(player, text.to_string());
    }

    fn sleep(&mut self, player: &player::Id) {
        let name = player.to_string();
        self.room.update(|state| {
            if let Some(game) = &mut state.game {
                game.awake.retain(|a| *a != name);
            }
        });
    }

    fn tell(&mut self, player: &player::Id, message: &info::Message) {
        self.log(
            room::Audience::Player(player.to_string()),
            message.to_string(),
            message.players().iter().map(ToString::to_string).collect(),
            message.characters(),
        );
    }

    fn ask(
        &mut self,
        player: &player::Id,
        prompt: &str,
        candidates: &[player::Id],
        count: usize,
    ) -> Vec<player::Id> {
        self.choose_players(player, prompt, candidates, count, count)
    }

    fn ask_up_to(
        &mut self,
        player: &player::Id,
        prompt: &str,
        candidates: &[player::Id],
        max: usize,
    ) -> Vec<player::Id> {
        self.choose_players(player, prompt, candidates, 0, max)
    }

    fn ask_yes_no(&mut self, player: &player::Id, prompt: &str) -> bool {
        self.yes_no(player, prompt)
    }

    fn ask_character(
        &mut self,
        player: &player::Id,
        prompt: &str,
        options: &[character::Ref],
    ) -> usize {
        let names: Vec<&'static str> = options.iter().map(|c| c.name()).collect();
        let kind = room::PromptKind::Character {
            options: names.clone(),
        };
        let chosen = match self.prompt(player, prompt, kind) {
            Some(inbox::Answer::Option(i)) => i,
            _ => 0,
        };
        let name = names.get(chosen).copied().unwrap_or_default();
        self.answered(
            player,
            format!("{player} chose the {name}"),
            Vec::new(),
            vec![name],
        );
        chosen
    }

    fn confirm(&mut self, player: &player::Id, question: &table::Question) -> bool {
        self.yes_no(player, &question_text(question))
    }

    fn event(&mut self, event: &event::Event, view: &game::State) {
        self.phase = view.phase();
        let phase = self.phase;
        let grimoire = view.clone();
        self.room.update(|state| {
            if let Some(game) = &mut state.game {
                game.phase = phase;
                game.grimoire = grimoire;
            }
        });
        self.record(event);
    }

    fn decided(&mut self, decision: &storyteller::Decision, picks: &[usize]) {
        let picked: Vec<String> = picks
            .iter()
            .map(|&i| decision.options[i].to_string())
            .collect();
        let mut players: Vec<String> = decision
            .reason
            .players()
            .iter()
            .map(ToString::to_string)
            .collect();
        let mut characters = decision.reason.characters();
        for &i in picks {
            players.extend(
                decision.options[i]
                    .players()
                    .iter()
                    .map(ToString::to_string),
            );
            characters.extend(decision.options[i].characters());
        }
        self.log(
            room::Audience::Storyteller,
            format!("{}: {}", decision.reason, picked.join(", ")),
            players,
            characters,
        );
    }
}
