//! Event vocabulary for the interaction pipeline.
//!
//! Deaths (and, later, other cross-role interactions) flow through the pipeline
//! in [`crate::ctx`] rather than being resolved inline by whichever character
//! initiated them. A character raises an intent — "the demon attacks X" — and
//! the pipeline consults every role's [`crate::ability::Ability`] hooks to
//! resolve it. That is what keeps Soldier/Monk/Mayor/Scarlet Woman append-only.

use crate::role::{Alignment, Kind};

/// Why a player is dying. Roles key their death reactions off this.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeathSource {
    /// The Demon's night kill. Soldier is immune, Monk protection blocks it.
    Demon,
    /// A day execution (including the Virgin's instant execution of a
    /// nominator). The Saint and Undertaker care about this.
    Execution,
    /// Any other ability-driven death (Slayer shot, starpass self-kill). The
    /// tag records the originating character id for logging.
    Ability(&'static str),
}

impl DeathSource {
    #[must_use]
    pub fn is_demon(self) -> bool {
        matches!(self, DeathSource::Demon)
    }

    #[must_use]
    pub fn is_execution(self) -> bool {
        matches!(self, DeathSource::Execution)
    }
}

/// How a player *may* register to information abilities.
///
/// Most roles register exactly as their current kind/alignment ([`None`] from
/// [`crate::ability::Ability::misregistration`]). Roles that can be shown as
/// something else — the Recluse and Spy — return a set of possible alignments
/// and kinds; the storyteller resolves each individual reading from that set,
/// which is exactly the discretionary "the Recluse might ping evil tonight"
/// call.
#[derive(Debug, Clone)]
pub struct Registration {
    /// Alignments this player may register as. Must be non-empty.
    pub alignments: Vec<Alignment>,
    /// Kinds this player may register as. Must be non-empty.
    pub kinds: Vec<Kind>,
}

impl Registration {
    /// The Recluse: good Outsider who may appear evil, and as a Minion or Demon.
    #[must_use]
    pub fn recluse() -> Self {
        Registration {
            alignments: vec![Alignment::Good, Alignment::Evil],
            kinds: vec![Kind::Outsider, Kind::Minion, Kind::Demon],
        }
    }

    /// The Spy: evil Minion who may appear good, and as a Townsfolk or Outsider.
    #[must_use]
    pub fn spy() -> Self {
        Registration {
            alignments: vec![Alignment::Evil, Alignment::Good],
            kinds: vec![Kind::Minion, Kind::Townsfolk, Kind::Outsider],
        }
    }

    #[must_use]
    pub fn may_be_evil(&self) -> bool {
        self.alignments.contains(&Alignment::Evil)
    }

    #[must_use]
    pub fn may_be_good(&self) -> bool {
        self.alignments.contains(&Alignment::Good)
    }

    #[must_use]
    pub fn may_be_kind(&self, kind: Kind) -> bool {
        self.kinds.contains(&kind)
    }
}
