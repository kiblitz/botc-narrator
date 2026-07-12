//! A storyteller that prints to stdout and always takes the first option.
//!
//! Useful for a non-interactive demo run. A real interactive backend would
//! implement the same trait and actually prompt a human.

use crate::ids::PlayerId;
use crate::storyteller::{Candidate, Storyteller};

#[derive(Default)]
pub struct ConsoleStoryteller;

impl ConsoleStoryteller {
    #[must_use]
    pub fn new() -> Self {
        ConsoleStoryteller
    }
}

impl Storyteller for ConsoleStoryteller {
    fn wake(&mut self, who: &str) {
        println!("  {who}: (wake)");
    }

    fn sleep(&mut self, who: &str) {
        println!("  {who}: (sleep)");
    }

    fn reveal(&mut self, who: &str, message: &str) {
        println!("  {who} <- {message}");
    }

    fn ask(&mut self, who: &str, prompt: &str, options: &[Candidate]) -> PlayerId {
        let chosen = &options[0];
        println!("  {who} ? {prompt} -> {}", chosen.label);
        chosen.id
    }

    fn choose(&mut self, prompt: &str, options: &[String]) -> usize {
        println!("  [narrator] {prompt} -> {}", options[0]);
        0
    }

    fn log(&mut self, message: &str) {
        println!("{message}");
    }
}
