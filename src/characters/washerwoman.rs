use crate::ability;
use crate::character;
use crate::ctx;
use crate::player;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "washerwoman"
    }

    fn name(&self) -> &'static str {
        "Washerwoman"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Townsfolk
    }

    fn night_order(&self) -> character::NightOrder {
        character::NightOrder {
            first: Some(33),
            other: None,
        }
    }

    fn night_action(&self) -> Option<character::NightAction> {
        Some(character::NightAction::ReadOnly(act))
    }
}

/// Learns that one of two players is a particular Townsfolk.
fn act(ctx: &mut ctx::Read<'_>, me: &player::Id, _night: u32) {
    ctx.wake(me);
    ability::tell_one_of(ctx, me, character::Kind::Townsfolk);
    ctx.sleep(me);
}
