//! Scarlet Woman (Minion): if the Demon dies while 5+ players are alive, she
//! becomes the Demon.

use crate::ability::Ability;
use crate::ctx::Ctx;
use crate::event::DeathSource;
use crate::ids::PlayerId;
use crate::role::{Alignment, CharacterInfo, Kind};

character!(
    ScarletWoman,
    "scarlet_woman",
    "Scarlet Woman",
    Kind::Minion,
    Alignment::Evil
);

/// Living-player threshold for promotion (matches the classic "5 or more
/// players alive" rule, counted after the Demon's death).
const PROMOTION_THRESHOLD: usize = 5;

impl Ability for ScarletWoman {
    fn info(&self) -> CharacterInfo {
        Self::INFO
    }

    fn on_any_death(&self, ctx: &mut Ctx, me: PlayerId, dead: PlayerId, _source: DeathSource) {
        // I must be a living, functioning Minion; the dead player must have been
        // the Demon; and no other Demon may remain (so a starpass, which makes a
        // new Demon first, does not also trigger me).
        let me_ok =
            ctx.grim.is_alive(me) && !ctx.is_impaired(me) && ctx.grim.get(me).kind == Kind::Minion;
        let demon_died = ctx.grim.get(dead).kind.is_demon() && !ctx.living_demon_exists();
        if me_ok && demon_died && ctx.grim.alive_count() >= PROMOTION_THRESHOLD {
            let demon_role = ctx.registry().info(ctx.grim.get(dead).role);
            ctx.transform(me, demon_role);
        }
    }
}
