//! Librarian (Townsfolk): on the first night, learn that one of two players is
//! a particular Outsider — or that there are no Outsiders in play.

use crate::ability::Ability;
use crate::ctx::Ctx;
use crate::ids::PlayerId;
use crate::role::{Alignment, CharacterInfo, Kind};

use super::info_role;

character!(
    Librarian,
    "librarian",
    "Librarian",
    Kind::Townsfolk,
    Alignment::Good
);

impl Ability for Librarian {
    fn info(&self) -> CharacterInfo {
        Self::INFO
    }

    fn on_night(&self, ctx: &mut Ctx, me: PlayerId, night: u32) {
        if night != 1 || !ctx.grim.is_alive(me) {
            return;
        }
        let has_outsider = ctx
            .grim
            .seats()
            .any(|id| ctx.grim.get(id).kind == Kind::Outsider);
        if !ctx.is_impaired(me) && !has_outsider {
            ctx.wake(me);
            ctx.reveal(me, "There are no Outsiders in play");
            ctx.sleep(me);
        } else {
            info_role::learn_one_of_two(ctx, me, Kind::Outsider, "librarian");
        }
    }
}
