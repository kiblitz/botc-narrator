use crate::character;
use crate::characters;
use crate::ctx;
use crate::info;
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

/// Learns the character of the player executed today.
fn act(ctx: &mut ctx::Read<'_>, me: &player::Id, _night: u32) {
    let Some(executed) = ctx.state().executed_today().cloned() else {
        return;
    };
    let registered = ctx.registered_character(me, &executed);
    let truthful = vec![info::Message::Character(registered.name())];
    let misinformation = characters::any_character(ctx.state());
    ctx.wake(me);
    ctx.inform(me, truthful, misinformation);
    ctx.sleep(me);
}
