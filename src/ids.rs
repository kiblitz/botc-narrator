//! Stable identifiers for players and characters.

use std::fmt;

/// Identifies a player by their seat position (0-based, clockwise).
///
/// Seat position is stable for the whole game: players never change seats, so
/// this doubles as both identity and geometry (neighbour calculations index by
/// it). Death and role changes leave the seat untouched.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PlayerId(pub usize);

impl PlayerId {
    #[must_use]
    pub fn seat(self) -> usize {
        self.0
    }
}

impl fmt::Display for PlayerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "P{}", self.0)
    }
}

/// Identifies a character (role) by a stable, interned string id such as
/// `"imp"` or `"fortune_teller"`.
///
/// Using an opaque string rather than a closed enum keeps the roster
/// data-driven: a script is a list of ids, the registry is a map keyed by id,
/// and homebrew characters can be added without touching a central enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CharacterId(pub &'static str);

impl CharacterId {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        self.0
    }
}

impl fmt::Display for CharacterId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}
