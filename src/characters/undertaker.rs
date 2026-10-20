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

    fn night_action(&self, night: u32) -> Option<character::NightAction> {
        (night > 1).then_some(character::NightAction::ReadOnly(act))
    }
}

/// Learns the character of the player executed today.
fn act(ctx: &mut ctx::Read<'_>, me: &player::Id, _night: u32) {
    let state = ctx.state();
    if !state.is_alive(me) {
        return;
    }
    let Some(executed) = state.executed_today() else {
        return;
    };
    let truthful = vec![info::Message::Character(state.character(executed).name())];
    let misinformation = characters::any_character(state);
    ctx.wake(me);
    ctx.inform(me, truthful, misinformation);
    ctx.sleep(me);
}
