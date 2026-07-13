//! Investigator (Townsfolk): on the first night, learn that one of two players
//! is a particular Minion.

use crate::ability::Ability;
use crate::ctx::Ctx;
use crate::ids::PlayerId;
use crate::role::{Alignment, CharacterInfo, Kind};

use super::info_role;

character!(
    Investigator,
    "investigator",
    "Investigator",
    Kind::Townsfolk,
    Alignment::Good
);

impl Ability for Investigator {
    fn info(&self) -> CharacterInfo {
        Self::INFO
    }

    fn on_night(&self, ctx: &mut Ctx, me: PlayerId, night: u32) {
        if night != 1 || !ctx.grim.is_alive(me) {
            return;
        }
        info_role::learn_one_of_two(ctx, me, Kind::Minion, "investigator");
    }
}
