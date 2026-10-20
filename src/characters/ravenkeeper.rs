use crate::character;
use crate::characters;
use crate::ctx;
use crate::info;
use crate::player;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "ravenkeeper"
    }

    fn name(&self) -> &'static str {
        "Ravenkeeper"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Townsfolk
    }

    fn night_action(&self, night: u32) -> Option<character::NightAction> {
        (night > 1).then_some(character::NightAction::ReadOnly(act))
    }
}

/// If they died tonight, chooses a player and learns their character.
fn act(ctx: &mut ctx::Read<'_>, me: &player::Id, _night: u32) {
    if !ctx.state().deaths_tonight().contains(me) {
        return;
    }
    let candidates = ctx.state().seat_order();
    ctx.wake(me);
    if let Some(target) = ctx.ask_one(me, "Choose a player to learn about", &candidates) {
        let state = ctx.state();
        let truthful = vec![info::Message::Character(state.character(&target).name())];
        let misinformation = characters::any_character(state);
        ctx.inform(me, truthful, misinformation);
    }
    ctx.sleep(me);
}
