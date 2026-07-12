//! Soldier (Townsfolk): safe from the Demon. Passive — it simply refuses the
//! demon's kill, unless impaired.

use crate::ability::Ability;
use crate::ctx::Ctx;
use crate::ids::PlayerId;
use crate::role::{Alignment, CharacterInfo, Kind};

character!(
    Soldier,
    "soldier",
    "Soldier",
    Kind::Townsfolk,
    Alignment::Good
);

impl Ability for Soldier {
    fn info(&self) -> CharacterInfo {
        Self::INFO
    }

    fn blocks_demon_kill(&self, ctx: &Ctx, me: PlayerId) -> bool {
        // Immune while sober & healthy; a poisoned Soldier can die.
        !ctx.is_impaired(me)
    }
}
