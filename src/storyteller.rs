//! The storyteller interaction boundary.
//!
//! A [`Storyteller`] is *how* narrator interactions actually happen — printing
//! to a console, prompting a UI, or replaying scripted answers in a test. All
//! game logic is written against this trait, so swapping the backend never
//! touches a character. This is the same separation the original engine got
//! from its free monad, achieved here by dependency injection (no reified
//! instruction GADT, no `Obj.magic`).

use crate::ids::PlayerId;

/// One selectable player in an [`Storyteller::ask`], carrying a display label.
#[derive(Debug, Clone)]
pub struct Candidate {
    pub id: PlayerId,
    pub label: String,
}

impl Candidate {
    #[must_use]
    pub fn new(id: PlayerId, label: impl Into<String>) -> Self {
        Candidate {
            id,
            label: label.into(),
        }
    }
}

/// The narrator I/O interface. Implementations decide how each interaction is
/// surfaced and answered.
pub trait Storyteller {
    /// Wake a player (they open their eyes). `who` is a display label.
    fn wake(&mut self, who: &str);

    /// Put a player back to sleep.
    fn sleep(&mut self, who: &str);

    /// Give a player one-way information ("You see 2 evil neighbours").
    fn reveal(&mut self, who: &str, message: &str);

    /// Ask a player to point at one of `options`; returns the chosen player.
    ///
    /// Implementations must return the id of one of the supplied candidates.
    fn ask(&mut self, who: &str, prompt: &str, options: &[Candidate]) -> PlayerId;

    /// A discretionary storyteller choice (which false number, which bluffs).
    /// Returns the chosen index into `options`.
    fn choose(&mut self, prompt: &str, options: &[String]) -> usize;

    /// A narrator-facing log line (not shown to any player).
    fn log(&mut self, message: &str);
}
