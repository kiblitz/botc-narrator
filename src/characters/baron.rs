//! Baron (Minion): adds two Outsiders to the game. This is a bag-composition
//! effect resolved before seating, so at runtime the Baron is inert — the extra
//! Outsiders are already present in the roster the game was built with.

use crate::ability::Ability;
use crate::ctx::Ctx;
use crate::ids::PlayerId;
use crate::role::{Alignment, CharacterInfo, Kind};

character!(Baron, "baron", "Baron", Kind::Minion, Alignment::Evil);

impl Ability for Baron {
    fn info(&self) -> CharacterInfo {
        Self::INFO
    }

    fn on_setup(&self, ctx: &mut Ctx, _me: PlayerId) {
        ctx.log("Baron in play: the game was built with two extra Outsiders");
    }
}
