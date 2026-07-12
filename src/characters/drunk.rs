//! Drunk (Outsider): you do not know you are the Drunk. You think you are a
//! Townsfolk, but your ability malfunctions.
//!
//! The Drunk needs no hooks: a player whose `believed_role` differs from their
//! true role is impaired by construction (see
//! [`Grimoire::is_impaired`](crate::grimoire::Grimoire::is_impaired)). The
//! engine runs the *believed* Townsfolk's ability for them, and the impairment
//! turns its output into storyteller-chosen noise. This role exists only to
//! carry the "Drunk" identity and its Outsider metadata.

use crate::ability::Ability;
use crate::role::{Alignment, CharacterInfo, Kind};

character!(Drunk, "drunk", "Drunk", Kind::Outsider, Alignment::Good);

impl Ability for Drunk {
    fn info(&self) -> CharacterInfo {
        Self::INFO
    }
}
