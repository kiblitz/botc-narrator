use crate::character;
use crate::ctx;
use crate::info;
use crate::player;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "fortune_teller"
    }

    fn name(&self) -> &'static str {
        "Fortune Teller"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Townsfolk
    }

    fn night_action(&self, _night: u32) -> Option<character::NightAction> {
        Some(character::NightAction::ReadOnly(act))
    }
}

/// Chooses two players and learns whether either is the demon.
fn act(ctx: &mut ctx::Read<'_>, me: &player::Id, _night: u32) {
    if !ctx.state().is_alive(me) {
        return;
    }
    let candidates = ctx.state().seat_order();
    ctx.wake(me);
    let chosen = ctx.ask(me, "Choose two players", &candidates, 2);
    let yes = chosen
        .iter()
        .any(|p| ctx.state().kind(p) == character::Kind::Demon);
    let truthful = vec![info::Message::YesNo(yes)];
    let misinformation = vec![info::Message::YesNo(!yes)];
    ctx.inform(me, truthful, misinformation);
    ctx.sleep(me);
}
