//! The table: how the storyteller talks to players. Implement [`Io`] for a
//! console, a GUI, a networked app, or a test recorder.

use rand::seq::index;

use crate::event;
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

    /// A public or storyteller-facing game event happened.
    fn event(&mut self, _event: &event::Event) {}

    /// The storyteller made a decision (for logging; players never see this).
    fn decided(&mut self, _decision: &storyteller::Decision, _picks: &[usize]) {}

    /// The next `size` night actions are independent reads that may run concurrently.
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

    fn event(&mut self, event: &event::Event) {
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
