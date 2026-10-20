use crate::character;
use crate::ctx;
use crate::event;
use crate::game;
use crate::player;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "slayer"
    }

    fn name(&self) -> &'static str {
        "Slayer"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Townsfolk
    }

    fn day_action(&self) -> Option<character::DayFn> {
        Some(slay)
    }
}

/// Once per game, publicly chooses a player: if they are the demon, they die.
fn slay(ctx: &mut ctx::Write<'_>, me: &player::Id) {
    let state = ctx.state();
    if !state.is_alive(me) || state.has_effect(me, game::Effect::AbilityUsed) {
        return;
    }
    let reminder = game::Reminder::new(&Def, game::Effect::AbilityUsed, game::Expiry::Never);
    ctx.place_reminder(me, reminder);
    let candidates = ctx.state().alive_ids();
    if let Some(target) = ctx.ask_one(me, "Who do you want to slay?", &candidates)
        && !ctx.state().is_malfunctioning(me)
        && ctx.state().kind(&target) == character::Kind::Demon
    {
        ctx.kill(&target, event::Cause::Ability { by: me.clone() });
    }
}
