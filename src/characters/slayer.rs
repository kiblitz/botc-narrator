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

    fn public_claim(&self) -> Option<character::PublicClaim> {
        Some(character::PublicClaim {
            limit: character::ClaimLimit::OncePerGame,
            announcement: "shoots",
            resolves_now: true,
            bluff: |ctx, me| {
                aim(ctx, me);
            },
        })
    }
}

/// Once per game, publicly chooses a player: if they are the demon, they die.
fn slay(ctx: &mut ctx::Write<'_>, me: &player::Id) {
    // A shot that cannot work (used, or the ability lost) is still aimed,
    // so it looks like any other shot.
    let works = ctx.state().ability_active(me) && ability::use_once(ctx, me, &Def);
    if let Some(target) = aim(ctx, me).filter(|_| works) {
        ability::kill(ctx, me, &target);
    }
}

/// Asks whom `me` shoots, and returns them if they register as the Demon.
/// The storyteller decides the registration even when the shot cannot work
/// (malfunctioning, or a bluff), so no shot gives itself away by the wait.
fn aim(ctx: &mut ctx::Write<'_>, me: &player::Id) -> Option<player::Id> {
    let candidates = ctx.state().alive_ids();
    let target = ctx.ask_one(me, "Who do you want to slay?", &candidates)?;
    let demon = ctx.registers_as(me, &target, character::Kind::Demon).is_some();
    (demon && !ctx.state().is_malfunctioning(me)).then_some(target)
}
