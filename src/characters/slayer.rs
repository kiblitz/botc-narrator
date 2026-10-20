use crate::ability;
use crate::character;
use crate::ctx;
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
    if !ctx.state().ability_active(me) || !ability::use_once(ctx, me, &Def) {
        return;
    }
    let candidates = ctx.state().alive_ids();
    if let Some(target) = ctx.ask_one(me, "Who do you want to slay?", &candidates)
        && !ctx.state().is_malfunctioning(me)
        && ctx.registers_as(me, &target, character::Kind::Demon).is_some()
    {
        ability::kill(ctx, me, &target);
    }
}
