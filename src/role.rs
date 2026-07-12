//! Character classification and static metadata.

use crate::ids::CharacterId;

/// The four (plus Traveller) character categories in Blood on the Clocktower.
///
/// Kind can change during a game (a Scarlet Woman `Minion` becomes a `Demon`),
/// which is why the current kind is tracked per-player rather than derived once.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    Townsfolk,
    Outsider,
    Minion,
    Demon,
    Traveller,
}

impl Kind {
    /// The team a kind belongs to *by default*. Actual alignment is tracked
    /// separately because it can diverge (e.g. a Traveller of either team).
    #[must_use]
    pub fn default_alignment(self) -> Alignment {
        match self {
            Kind::Townsfolk | Kind::Outsider => Alignment::Good,
            Kind::Minion | Kind::Demon => Alignment::Evil,
            // Travellers are assigned an alignment explicitly; default Good.
            Kind::Traveller => Alignment::Good,
        }
    }

    #[must_use]
    pub fn is_demon(self) -> bool {
        matches!(self, Kind::Demon)
    }

    #[must_use]
    pub fn is_minion(self) -> bool {
        matches!(self, Kind::Minion)
    }
}

/// Which team a player is playing for. This is the thing win conditions and
/// most "evil" queries actually care about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Alignment {
    Good,
    Evil,
}

impl Alignment {
    #[must_use]
    pub fn is_evil(self) -> bool {
        matches!(self, Alignment::Evil)
    }
}

/// Static, unchanging facts about a character. Returned by every
/// [`crate::ability::Ability`] so the registry can answer metadata queries
/// (name, starting kind/alignment) without a separate table.
#[derive(Debug, Clone, Copy)]
pub struct CharacterInfo {
    pub id: CharacterId,
    pub name: &'static str,
    /// The kind this character starts as. The *current* kind lives on the
    /// player and may differ after a transformation.
    pub kind: Kind,
    pub alignment: Alignment,
}

impl CharacterInfo {
    #[must_use]
    pub const fn new(
        id: &'static str,
        name: &'static str,
        kind: Kind,
        alignment: Alignment,
    ) -> Self {
        CharacterInfo {
            id: CharacterId(id),
            name,
            kind,
            alignment,
        }
    }
}
