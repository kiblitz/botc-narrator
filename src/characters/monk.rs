use crate::character;
use crate::ctx;
use crate::game;
use crate::player;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "monk"
    }

    fn name(&self) -> &'static str {
        "Monk"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Townsfolk
    }

    fn night_action(&self, night: u32) -> Option<character::NightAction> {
        (night > 1).then_some(character::NightAction::ReadWrite(act))
    }
}

/// Protects another player from the demon tonight.
fn act(ctx: &mut ctx::Write<'_>, me: &player::Id, _night: u32) {
    if !ctx.state().is_alive(me) {
        return;
    }
    let candidates = ctx.state().alive_except(me);
    ctx.wake(me);
    if let Some(target) = ctx.ask_one(me, "Who do you want to protect?", &candidates)
        && !ctx.state().is_malfunctioning(me)
    {
        let reminder = game::Reminder::new(&Def, me, game::Effect::Protected, game::Expiry::Dusk);
        ctx.place_reminder(&target, reminder);
    }
    ctx.sleep(me);
}
