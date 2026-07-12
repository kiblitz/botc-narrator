//! Saint (Outsider): if you are executed, your team loses.

use crate::ability::Ability;
use crate::ctx::Ctx;
use crate::ids::PlayerId;
use crate::role::{Alignment, CharacterInfo, Kind};

character!(Saint, "saint", "Saint", Kind::Outsider, Alignment::Good);

impl Ability for Saint {
    fn info(&self) -> CharacterInfo {
        Self::INFO
    }

    fn on_executed(&self, ctx: &mut Ctx, me: PlayerId) {
        // A poisoned/drunk Saint executed is just a dead Saint.
        if !ctx.is_impaired(me) {
            ctx.declare_winner(Alignment::Evil, "the Saint was executed");
        }
    }
}
