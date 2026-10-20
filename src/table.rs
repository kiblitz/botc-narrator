//! The table: how the storyteller talks to players. Implement [`Io`] for a
//! console, a GUI, a networked app, or a test recorder.

use std::fmt;

use rand::seq::index;

use crate::character;
use crate::event;
use crate::game;
use crate::info;
use crate::player;
use crate::storyteller;

/// A yes/no question for a player (or the table's judgement about them).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Question {
    /// Whether to use an optional ability now (the Assassin's kill).
    UseAbility(&'static str),
    /// Whether to publicly claim to be this character (the Goblin, when nominated).
    Claim(&'static str),
    /// Whether the player said this word today (the Mezepheles's secret word).
    SaidWord(&'static str),
    /// Whether the player lost a game of roshambo against `against` (the
    /// storyteller when `None`).
    LostRoshambo { against: Option<player::Id> },
}

impl fmt::Display for Question {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Question::UseAbility(name) => write!(f, "use the {name} ability?"),
            Question::Claim(name) => write!(f, "claim to be the {name}?"),
            Question::SaidWord(word) => write!(f, "said \"{word}\"?"),
            Question::LostRoshambo { against: Some(p) } => write!(f, "lost roshambo against {p}?"),
            Question::LostRoshambo { against: None } => {
                f.write_str("lost roshambo against the storyteller?")
            }
        }
    }
}

pub trait Io {
    fn wake(&mut self, player: &player::Id);
    fn sleep(&mut self, player: &player::Id);
    fn tell(&mut self, player: &player::Id, message: &info::Message);

    /// Ask a player to point at `count` distinct candidates. Invalid answers
    /// are replaced by the first `count` candidates.
    fn ask(
        &mut self,
        player: &player::Id,
        prompt: &str,
        candidates: &[player::Id],
        count: usize,
    ) -> Vec<player::Id>;

    /// Ask a player who "may choose" to point at up to `max` distinct
    /// candidates; fewer (or none) is a legal answer. The default answers as
    /// to [`Io::ask`], so never chooses fewer: override it to let players
    /// choose no-one.
    fn ask_up_to(
        &mut self,
        player: &player::Id,
        prompt: &str,
        candidates: &[player::Id],
        max: usize,
    ) -> Vec<player::Id> {
        self.ask(player, prompt, candidates, max)
    }

    /// Ask a player a yes/no question (whether they agree); no by default.
    fn ask_yes_no(&mut self, _player: &player::Id, _prompt: &str) -> bool {
        false
    }

    /// Ask a player to choose one of `options` (characters); returns its
    /// index. Invalid answers are replaced by the first option.
    fn ask_character(
        &mut self,
        _player: &player::Id,
        _prompt: &str,
        _options: &[character::Ref],
    ) -> usize {
        0
    }

    /// A game event happened; `view` is the grimoire after it.
    fn event(&mut self, _event: &event::Event, _view: &game::State) {}

    /// The storyteller made a decision (for logging; players never see this).
    fn decided(&mut self, _decision: &storyteller::Decision, _picks: &[usize]) {}

    /// Whether `player`, mad about being the `about` character, broke their
    /// madness (did not try to convince the group of it).
    fn broke_madness(&mut self, _player: &player::Id, _about: &'static str) -> bool {
        false
    }

    /// Whether `player` tried to convince the group they are `about` (the
    /// Mutant must not be mad about being an Outsider).
    fn is_mad_about(&mut self, _player: &player::Id, _about: &'static str) -> bool {
        false
    }

    /// A typed yes/no `question` about `player`: a rules choice (use an
    /// ability, claim a character) or what the table saw (said the word, lost
    /// roshambo). Unlike `ask_yes_no`'s free-text prompt, the question is
    /// typed so a table can answer each kind differently.
    fn confirm(&mut self, _player: &player::Id, _question: &Question) -> bool {
        false
    }

    /// The next `size` night actions are independent reads (announced as a batch).
    fn batch_started(&mut self, _size: usize) {}
    fn batch_ended(&mut self) {}
}

/// Prints everything to stdout; players answer randomly.
pub struct Console {
    rng: rand::rngs::ThreadRng,
}

impl Console {
    pub fn new() -> Self {
        Self { rng: rand::rng() }
    }
}

impl Default for Console {
    fn default() -> Self {
        Self::new()
    }
}

impl Io for Console {
    fn wake(&mut self, player: &player::Id) {
        println!("[wake]  {player}");
    }

    fn sleep(&mut self, player: &player::Id) {
        println!("[sleep] {player}");
    }

    fn tell(&mut self, player: &player::Id, message: &info::Message) {
        println!("[tell -> {player}] {message}");
    }

    fn ask(
        &mut self,
        player: &player::Id,
        prompt: &str,
        candidates: &[player::Id],
        count: usize,
    ) -> Vec<player::Id> {
        let answer: Vec<player::Id> = index::sample(&mut self.rng, candidates.len(), count)
            .into_iter()
            .map(|i| candidates[i].clone())
            .collect();
        let names: Vec<&str> = answer.iter().map(player::Id::as_str).collect();
        println!("[ask {player}] {prompt} -> {}", names.join(", "));
        answer
    }

    fn ask_up_to(
        &mut self,
        player: &player::Id,
        prompt: &str,
        candidates: &[player::Id],
        max: usize,
    ) -> Vec<player::Id> {
        if rand::Rng::random_bool(&mut self.rng, 0.2) {
            println!("[ask {player}] {prompt} -> no-one");
            return Vec::new();
        }
        self.ask(player, prompt, candidates, max)
    }

    fn event(&mut self, event: &event::Event, _view: &game::State) {
        println!("[event] {event}");
    }

    fn decided(&mut self, decision: &storyteller::Decision, picks: &[usize]) {
        let picked: Vec<String> = picks
            .iter()
            .map(|&i| decision.options[i].to_string())
            .collect();
        println!("[storyteller] {}: {}", decision.reason, picked.join(", "));
    }
}
