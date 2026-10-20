use crate::ability;
use crate::character;
use crate::ctx;
use crate::player;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "empath"
    }

    fn name(&self) -> &'static str {
        "Empath"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Townsfolk
    }

    fn night_order(&self) -> character::NightOrder {
        character::NightOrder {
            first: Some(37),
            other: Some(53),
        }
    }

    fn night_action(&self) -> Option<character::NightAction> {
        Some(character::NightAction::ReadOnly(act))
    }
}

/// Learns how many of their two alive neighbours are evil.
fn act(ctx: &mut ctx::Read<'_>, me: &player::Id, _night: u32) {
    let neighbours = ability::alive_neighbours(ctx.state(), me);
    let count = ability::count_evil(ctx, me, &neighbours);
    ctx.wake(me);
    ability::tell_count(ctx, me, count, 2, "evil neighbours");
    ctx.sleep(me);
}
