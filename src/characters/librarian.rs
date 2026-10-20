use crate::ability;
use crate::character;
use crate::ctx;
use crate::player;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "librarian"
    }

    fn name(&self) -> &'static str {
        "Librarian"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Townsfolk
    }

    fn night_order(&self) -> character::NightOrder {
        character::NightOrder {
            first: Some(34),
            other: None,
        }
    }

    fn night_action(&self) -> Option<character::NightAction> {
        Some(character::NightAction::ReadOnly(act))
    }
}

/// Learns that one of two players is a particular Outsider.
fn act(ctx: &mut ctx::Read<'_>, me: &player::Id, _night: u32) {
    ctx.wake(me);
    ability::tell_one_of(ctx, me, character::Kind::Outsider);
    ctx.sleep(me);
}
