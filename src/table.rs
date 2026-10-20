//! The table: how the storyteller talks to players. Implement [`Io`] for a
//! console, a GUI, a networked app, or a test recorder.

use rand::seq::index;

use crate::character;
use crate::event;
use crate::game;
use crate::info;
use crate::player;
use crate::storyteller;

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
