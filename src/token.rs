//! Reminder tokens: the open, generic status system.
//!
//! Every transient fact about a player is a token in a bag, not a hardcoded
//! boolean field. Adding a new status (drunk, red herring, master, …) means
//! adding a variant here — never widening the player record or writing a new
//! `set_/clear_` pair. Queries like "is this player impaired?" are expressed
//! over the token bag so they compose automatically with new roles.

use crate::ids::PlayerId;
use std::mem::discriminant;

/// A reminder token placed on a player.
///
/// Tokens carrying data (e.g. [`Token::Master`]) still compare by *kind* when
/// removed in bulk via [`crate::grimoire::Grimoire::remove_tokens_of_kind`],
/// so "clear the old master, place a new one" is one call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Token {
    /// Placed by the Poisoner. The holder's ability malfunctions. Persists
    /// until the Poisoner acts again or dies (handled by the Poisoner's logic),
    /// so it is *not* cleared automatically at dawn.
    Poisoned,
    /// The player is the Drunk (or is otherwise permanently drunk). Impairs
    /// exactly like poison but is never removed.
    Drunk,
    /// Placed by the Monk: the holder is safe from the Demon tonight. Cleared
    /// at dawn.
    Protected,
    /// Placed by the Fortune Teller's setup: this player registers as the Demon
    /// to the Fortune Teller. Permanent.
    RedHerring,
    /// The Butler's chosen master for tonight. Re-placed each night.
    Master(PlayerId),
    /// The player died (execution or otherwise) during the current day. Read by
    /// the Undertaker; cleared when the next night begins.
    DiedToday,
    /// A once-per-game ability has been spent (Slayer shot, Virgin triggered).
    /// Permanent.
    UsedAbility,
}

impl Token {
    /// Whether holding this token impairs the player's ability.
    #[must_use]
    pub fn impairs(self) -> bool {
        matches!(self, Token::Poisoned | Token::Drunk)
    }

    /// Whether this token is cleared automatically at dawn (night → day).
    ///
    /// `Protected` is placed at night and expires the same dawn. `DiedToday` is
    /// placed during the day, survives the following night (so the Undertaker
    /// can read it), and is cleared at the *next* dawn — both are dawn-scoped.
    #[must_use]
    pub fn clears_at_dawn(self) -> bool {
        matches!(self, Token::Protected | Token::DiedToday)
    }

    /// True when `self` and `other` are the same variant, ignoring any payload.
    #[must_use]
    pub fn same_kind(self, other: Token) -> bool {
        discriminant(&self) == discriminant(&other)
    }
}
