//! What an ability can do. [`Read`] can talk to players and the storyteller
//! but cannot change the grimoire; [`Write`] can also change it, and resolves
//! rule interactions (protection, redirects, death triggers, win checks).

use crate::character;
use crate::event;
use crate::game;
use crate::info;
use crate::player;
use crate::storyteller;
use crate::table;

pub struct Read<'a> {
    state: &'a game::State,
    io: &'a mut dyn table::Io,
    storyteller: &'a mut dyn storyteller::Strategy,
}

impl<'a> Read<'a> {
    pub fn new(
        state: &'a game::State,
        io: &'a mut dyn table::Io,
        storyteller: &'a mut dyn storyteller::Strategy,
    ) -> Self {
        Self {
            state,
            io,
            storyteller,
        }
    }

    pub fn state(&self) -> &game::State {
        self.state
    }

    pub fn wake(&mut self, player: &player::Id) {
        self.io.wake(player);
    }

    pub fn sleep(&mut self, player: &player::Id) {
        self.io.sleep(player);
    }

    pub fn tell(&mut self, player: &player::Id, message: &info::Message) {
        self.io.tell(player, message);
    }

    /// Ask a player to point at `count` distinct candidates.
    pub fn ask(
        &mut self,
        player: &player::Id,
        prompt: &str,
        candidates: &[player::Id],
        count: usize,
    ) -> Vec<player::Id> {
        let count = count.min(candidates.len());
        if count == 0 {
            return Vec::new();
        }
        let answer = self.io.ask(player, prompt, candidates, count);
        let valid = answer.len() == count
            && answer.iter().all(|a| candidates.contains(a))
            && (1..answer.len()).all(|i| !answer[..i].contains(&answer[i]));
        if valid {
            answer
        } else {
            candidates[..count].to_vec()
        }
    }

    pub fn ask_one(
        &mut self,
        player: &player::Id,
        prompt: &str,
        candidates: &[player::Id],
    ) -> Option<player::Id> {
        self.ask(player, prompt, candidates, 1).into_iter().next()
    }

    /// Let the storyteller choose `count` of `options`. When there is no real
    /// choice (as many options as picks) the storyteller is not consulted.
    pub fn decide(
        &mut self,
        reason: storyteller::Reason,
        options: Vec<storyteller::Choice>,
        count: usize,
    ) -> Vec<storyteller::Choice> {
        let count = count.min(options.len());
        if count == 0 {
            return Vec::new();
        }
        let decision = storyteller::Decision {
            reason,
            options,
            count,
        };
        let picks = if decision.options.len() == count {
            (0..count).collect()
        } else {
            let picks = self.storyteller.decide(&decision, self.state);
            if decision.is_valid(&picks) {
                picks
            } else {
                (0..count).collect()
            }
        };
        self.io.decided(&decision, &picks);
        picks
            .into_iter()
            .map(|i| decision.options[i].clone())
            .collect()
    }

    /// Give `player` information. The storyteller picks one of the `truthful`
    /// messages; if the player is malfunctioning, `misinformation` is legal too.
    pub fn inform(
        &mut self,
        player: &player::Id,
        truthful: Vec<info::Message>,
        misinformation: Vec<info::Message>,
    ) {
        let malfunctioning = self.state.is_malfunctioning(player);
        let mut options: Vec<storyteller::Choice> = truthful
            .into_iter()
            .map(|message| storyteller::Choice::Info {
                message,
                truthful: true,
            })
            .collect();
        if malfunctioning {
            for message in misinformation {
                let duplicate = options.iter().any(
                    |o| matches!(o, storyteller::Choice::Info { message: m, .. } if *m == message),
                );
                if !duplicate {
                    options.push(storyteller::Choice::Info {
                        message,
                        truthful: false,
                    });
                }
            }
        }
        let reason = storyteller::Reason::Information {
            player: player.clone(),
            character: self.state.character(player).name(),
            malfunctioning,
        };
        if let Some(storyteller::Choice::Info { message, .. }) =
            self.decide(reason, options, 1).into_iter().next()
        {
            self.tell(player, &message);
        }
    }
}

pub struct Write<'a> {
    state: &'a mut game::State,
    io: &'a mut dyn table::Io,
    storyteller: &'a mut dyn storyteller::Strategy,
}

impl<'a> Write<'a> {
    pub fn new(
        state: &'a mut game::State,
        io: &'a mut dyn table::Io,
        storyteller: &'a mut dyn storyteller::Strategy,
    ) -> Self {
        Self {
            state,
            io,
            storyteller,
        }
    }

    /// A read-only view of this context, for talking to players and the storyteller.
    pub fn read(&mut self) -> Read<'_> {
        Read::new(self.state, self.io, self.storyteller)
    }

    pub fn state(&self) -> &game::State {
        self.state
    }

    pub(crate) fn io(&mut self) -> &mut dyn table::Io {
        self.io
    }

    pub fn record(&mut self, event: event::Event) {
        self.io.event(&event);
        self.state.record(event);
    }

    /// A storyteller-facing note, kept in the history.
    pub fn note(&mut self, text: impl Into<String>) {
        self.record(event::Event::Note(text.into()));
    }

    pub fn advance_phase(&mut self) -> game::Phase {
        let phase = self.state.advance_phase();
        self.record(event::Event::PhaseBegan(phase));
        phase
    }

    pub fn place_reminder(&mut self, target: &player::Id, reminder: game::Reminder) {
        self.state.place_reminder(target, reminder);
    }

    /// Attempt to kill `target`. Demon kills respect protection, demon-proof
    /// characters and redirects. Returns whether someone died.
    pub fn kill(&mut self, target: &player::Id, cause: event::Cause) -> bool {
        if !self.state.is_alive(target) {
            return false;
        }
        if cause == event::Cause::Demon {
            if let Some(by) = self.state.effect_source(target, game::Effect::Protected) {
                self.record(event::Event::KillPrevented {
                    player: target.clone(),
                    by,
                });
                return false;
            }
            let character = self.state.character(target);
            if !self.state.is_malfunctioning(target) {
                if character.safe_from_demon() {
                    self.record(event::Event::KillPrevented {
                        player: target.clone(),
                        by: character.id(),
                    });
                    return false;
                }
                if let Some(redirect) = character.redirect_demon_kill(self, target)
                    && &redirect != target
                {
                    self.record(event::Event::KillRedirected {
                        from: target.clone(),
                        to: redirect.clone(),
                    });
                    return self.kill(&redirect, cause);
                }
            }
        }
        self.die(target, cause);
        true
    }

    fn die(&mut self, target: &player::Id, cause: event::Cause) {
        self.state.mark_dead(target);
        self.record(event::Event::Died {
            player: target.clone(),
            cause: cause.clone(),
        });
        let mut watchers: Vec<(player::Id, character::Ref)> = self
            .state
            .players()
            .iter()
            .filter(|p| p.alive)
            .map(|p| (p.id.clone(), p.character))
            .collect();
        watchers.push((target.clone(), self.state.character(target)));
        for (id, character) in watchers {
            character.on_death(self, &id, target, &cause);
        }
        self.check_win();
    }

    /// Execute `target`: their execution triggers fire, then they die.
    pub fn execute(&mut self, target: &player::Id) {
        if !self.state.is_alive(target) {
            return;
        }
        self.state.mark_executed(target);
        self.record(event::Event::Executed(target.clone()));
        self.state.character(target).on_executed(self, target);
        self.die(target, event::Cause::Execution);
    }

    pub fn transform(&mut self, target: &player::Id, into: character::Ref) {
        self.state.set_character(target, into);
        self.record(event::Event::Transformed {
            player: target.clone(),
            into: into.name(),
        });
    }

    pub fn win(&mut self, alignment: character::Alignment) {
        if self.state.winner().is_none() {
            self.state.set_winner(alignment);
            self.record(event::Event::Won(alignment));
        }
    }

    /// Good wins when no demon lives; evil wins when two players remain.
    pub fn check_win(&mut self) {
        if !self
            .state
            .players()
            .iter()
            .any(|p| p.alive && p.character.kind() == character::Kind::Demon)
        {
            self.win(character::Alignment::Good);
        } else if self.state.alive_ids().len() <= 2 {
            self.win(character::Alignment::Evil);
        }
    }

    // Read conveniences, so read-write abilities need not reborrow.

    pub fn wake(&mut self, player: &player::Id) {
        self.read().wake(player);
    }

    pub fn sleep(&mut self, player: &player::Id) {
        self.read().sleep(player);
    }

    pub fn ask_one(
        &mut self,
        player: &player::Id,
        prompt: &str,
        candidates: &[player::Id],
    ) -> Option<player::Id> {
        self.read().ask_one(player, prompt, candidates)
    }

    pub fn decide(
        &mut self,
        reason: storyteller::Reason,
        options: Vec<storyteller::Choice>,
        count: usize,
    ) -> Vec<storyteller::Choice> {
        self.read().decide(reason, options, count)
    }
}
