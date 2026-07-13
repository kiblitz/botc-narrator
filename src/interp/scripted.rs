//! A deterministic storyteller for tests: scripted answers in, transcript out.

use crate::ids::PlayerId;
use crate::storyteller::{Candidate, Storyteller};
use std::collections::VecDeque;

/// Replays queued answers and records everything that happened.
///
/// * `responses` feed [`Storyteller::ask`] — each is the player the asked
///   character points at. When the queue is empty, the first candidate is
///   chosen (matching the original engine's default).
/// * `choices` feed [`Storyteller::choose`] — each is the index the narrator
///   picks. When empty, index `0` is chosen.
///
/// Every interaction is appended to `transcript` in a stable, assertable
/// format; `reveals` additionally captures `(who, message)` pairs so tests can
/// assert on information without parsing text.
#[derive(Default)]
pub struct ScriptedStoryteller {
    responses: VecDeque<PlayerId>,
    choices: VecDeque<usize>,
    transcript: Vec<String>,
    reveals: Vec<(String, String)>,
}

impl ScriptedStoryteller {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Queue the players an asked character will point at, in order.
    #[must_use]
    pub fn with_responses(mut self, responses: impl IntoIterator<Item = PlayerId>) -> Self {
        self.responses.extend(responses);
        self
    }

    /// Queue the discretionary indices the narrator will pick, in order.
    #[must_use]
    pub fn with_choices(mut self, choices: impl IntoIterator<Item = usize>) -> Self {
        self.choices.extend(choices);
        self
    }

    /// Push more scripted `ask` answers onto the queue mid-run.
    pub fn push_responses(&mut self, responses: impl IntoIterator<Item = PlayerId>) {
        self.responses.extend(responses);
    }

    /// Push more scripted `choose` answers onto the queue mid-run.
    pub fn push_choices(&mut self, choices: impl IntoIterator<Item = usize>) {
        self.choices.extend(choices);
    }

    #[must_use]
    pub fn transcript(&self) -> &[String] {
        &self.transcript
    }

    #[must_use]
    pub fn transcript_text(&self) -> String {
        self.transcript.join("\n")
    }

    /// Every `(who, message)` revealed so far.
    #[must_use]
    pub fn reveals(&self) -> &[(String, String)] {
        &self.reveals
    }

    /// The last message revealed to `who`, if any.
    #[must_use]
    pub fn last_reveal_to(&self, who: &str) -> Option<&str> {
        self.reveals
            .iter()
            .rev()
            .find(|(w, _)| w == who)
            .map(|(_, m)| m.as_str())
    }
}

impl Storyteller for ScriptedStoryteller {
    fn wake(&mut self, who: &str) {
        self.transcript.push(format!("narrator->{who}: wake"));
    }

    fn sleep(&mut self, who: &str) {
        self.transcript.push(format!("narrator->{who}: sleep"));
    }

    fn reveal(&mut self, who: &str, message: &str) {
        self.transcript.push(format!("narrator->{who}: {message}"));
        self.reveals.push((who.to_string(), message.to_string()));
    }

    fn ask(&mut self, who: &str, prompt: &str, options: &[Candidate]) -> PlayerId {
        assert!(!options.is_empty(), "ask with no candidates for {who}");
        self.transcript.push(format!("narrator->{who}: {prompt}"));
        let chosen = match self.responses.pop_front() {
            Some(id) => id,
            None => options[0].id,
        };
        let candidate = options
            .iter()
            .find(|c| c.id == chosen)
            .unwrap_or_else(|| panic!("scripted response {chosen} not among candidates for {who}"));
        self.transcript
            .push(format!("{who}->narrator: {}", candidate.label));
        candidate.id
    }

    fn choose(&mut self, prompt: &str, options: &[String]) -> usize {
        assert!(!options.is_empty(), "choose with no options: {prompt}");
        let idx = self.choices.pop_front().unwrap_or(0);
        assert!(
            idx < options.len(),
            "scripted choice index {idx} out of range for '{prompt}'"
        );
        self.transcript
            .push(format!("[narrator picks] {prompt}: {}", options[idx]));
        idx
    }

    fn log(&mut self, message: &str) {
        self.transcript.push(message.to_string());
    }
}
