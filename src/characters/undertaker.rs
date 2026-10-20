use crate::ability;
use crate::character;
use crate::ctx;
use crate::player;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "undertaker"
    }

    fn name(&self) -> &'static str {
        "Undertaker"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Townsfolk
    }

    fn night_order(&self) -> character::NightOrder {
        character::NightOrder {
            first: None,
            other: Some(55),
        }
    }

    fn night_action(&self) -> Option<character::NightAction> {
        Some(character::NightAction::ReadOnly(act))
    }
}

/// Learns which character died by execution today.
fn act(ctx: &mut ctx::Read<'_>, me: &player::Id, _night: u32) {
    let Some(executed) = ctx.state().executed_today().cloned() else {
        return;
    };
    ctx.wake(me);
    ability::tell_character(ctx, me, &executed);
    ctx.sleep(me);
}
