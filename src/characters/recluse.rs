//! Recluse (Outsider): you might register as evil, and as a Minion or Demon,
//! even though you are good.

use crate::ability::Ability;
use crate::event::Registration;
use crate::role::{Alignment, CharacterInfo, Kind};

character!(
    Recluse,
    "recluse",
    "Recluse",
    Kind::Outsider,
    Alignment::Good
);

impl Ability for Recluse {
    fn info(&self) -> CharacterInfo {
        Self::INFO
    }

    fn misregistration(&self) -> Option<Registration> {
        Some(Registration::recluse())
    }
}
