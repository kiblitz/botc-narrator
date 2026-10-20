use crate::ability;
use crate::character;
use crate::ctx;
use crate::player;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "oracle"
    }

    fn name(&self) -> &'static str {
        "Oracle"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Townsfolk
    }

    fn night_order(&self) -> character::NightOrder {
        character::NightOrder {
            first: None,
            other: Some(82),
        }
    }

    fn night_action(&self) -> Option<character::NightAction> {
        Some(character::NightAction::ReadOnly(act))
    }
}

/// Each night*, learns how many dead players are evil.
fn act(ctx: &mut ctx::Read<'_>, me: &player::Id, _night: u32) {
    let dead: Vec<player::Id> = ctx
        .state()
        .seat_order()
        .into_iter()
        .filter(|p| !ctx.state().is_alive(p))
        .collect();
    let evil = ability::count_evil(ctx, me, &dead);
    ctx.wake(me);
    ability::tell_count(ctx, me, evil, dead.len(), "dead evil");
    ctx.sleep(me);
}
