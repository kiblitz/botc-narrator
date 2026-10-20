//! Shared test harness: a recording table, scripted answers, and a grimoire printer.
#![allow(dead_code)]

use std::collections::VecDeque;
use std::fmt::Write as _;

use botc_narrator::character;
use botc_narrator::ctx;
use botc_narrator::event;
use botc_narrator::game;
use botc_narrator::info;
use botc_narrator::player;
use botc_narrator::storyteller;
use botc_narrator::table;

pub fn p(name: &str) -> player::Id {
    player::Id::new(name)
}

pub fn state(seats: &[(&str, character::Ref)]) -> game::State {
    game::State::new(seats.iter().map(|(name, c)| (p(name), *c)).collect())
}

/// Records everything said at the table; players answer from a queue, or
/// with the first candidates once the queue is empty.
#[derive(Default)]
pub struct Recorder {
    answers: VecDeque<Vec<player::Id>>,
    pub log: String,
}

impl Recorder {
    fn line(&mut self, text: impl AsRef<str>) {
        self.log.push_str(text.as_ref());
        self.log.push('\n');
    }
}

impl table::Io for Recorder {
    fn wake(&mut self, player: &player::Id) {
        self.line(format!("wake {player}"));
    }

    fn sleep(&mut self, player: &player::Id) {
        self.line(format!("sleep {player}"));
    }

    fn tell(&mut self, player: &player::Id, message: &info::Message) {
        self.line(format!("  -> {player}: {message}"));
    }

    fn ask(
        &mut self,
        player: &player::Id,
        prompt: &str,
        candidates: &[player::Id],
        count: usize,
    ) -> Vec<player::Id> {
        let answer = self
            .answers
            .pop_front()
            .unwrap_or_else(|| candidates[..count].to_vec());
        let names: Vec<&str> = answer.iter().map(player::Id::as_str).collect();
        self.line(format!("  {player}? {prompt} => {}", names.join(", ")));
        answer
    }

    fn event(&mut self, event: &event::Event, _view: &game::State) {
        self.line(format!("* {event}"));
    }

    fn decided(&mut self, decision: &storyteller::Decision, picks: &[usize]) {
        let picked: Vec<String> = picks
            .iter()
            .map(|&i| decision.options[i].to_string())
            .collect();
        self.line(format!(
            "  storyteller [{} of {} options] {}: {}",
            picks.len(),
            decision.options.len(),
            decision.reason,
            picked.join(", ")
        ));
    }

    fn batch_started(&mut self, size: usize) {
        self.line(format!("=== parallel ({size}) ==="));
    }

    fn batch_ended(&mut self) {
        self.line("=== end parallel ===");
    }
}

/// A game under test: the state plus scripted player answers and storyteller picks.
pub struct Game {
    pub state: game::State,
    pub table: Recorder,
    pub storyteller: storyteller::Scripted,
}

impl Game {
    pub fn new(seats: &[(&str, character::Ref)]) -> Self {
        Self {
            state: state(seats),
            table: Recorder::default(),
            storyteller: storyteller::Scripted::default(),
        }
    }

    /// Queue answers for the next player questions, one entry per question.
    pub fn answers(&mut self, answers: &[&[&str]]) -> &mut Self {
        self.table.answers = answers
            .iter()
            .map(|names| names.iter().map(|n| p(n)).collect())
            .collect();
        self
    }

    /// Queue picks for the next storyteller decisions.
    pub fn picks(&mut self, picks: &[&[usize]]) -> &mut Self {
        self.storyteller = storyteller::Scripted::new(picks.iter().map(|p| p.to_vec()));
        self
    }

    pub fn run(&mut self, f: impl FnOnce(&mut ctx::Write<'_>)) -> &mut Self {
        let mut ctx = ctx::Write::new(&mut self.state, &mut self.table, &mut self.storyteller);
        f(&mut ctx);
        self
    }

    /// Everything recorded so far, followed by the grimoire; clears the log.
    pub fn transcript(&mut self) -> String {
        let mut out = std::mem::take(&mut self.table.log);
        out.push_str("--- grimoire ---\n");
        out.push_str(&grimoire(&self.state));
        out
    }

    /// Discard what was recorded so far.
    pub fn quiet(&mut self) -> &mut Self {
        self.table.log.clear();
        self
    }
}

pub fn grimoire(state: &game::State) -> String {
    let mut out = String::new();
    for p in state.players() {
        let _ = write!(out, "{}: {}", p.id, p.character.name());
        if !p.alive {
            out.push_str(" [dead]");
        }
        for r in &p.reminders {
            let _ = write!(out, " [{} by {}]", r.effect, r.source);
        }
        out.push('\n');
    }
    if let Some(winner) = state.winner() {
        let _ = writeln!(out, "winner: {winner}");
    }
    out
}
