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

    fn day_targets(&self) -> Option<character::Targets> {
        Some(character::Targets {
            prompt: "Who do you want to slay?",
            count: 1,
            candidates: |state, _| state.alive_ids(),
        })
    }

    fn claim(&self) -> Option<character::Claim> {
        Some(character::Claim {
            limit: character::ClaimLimit::OncePerGame,
            visibility: character::Visibility::Public {
                announcement: "shoots",
                resolves_now: true,
            },
            bluff: |ctx, me, targets| {
                aim(ctx, me, targets);
            },
        })
    }
}

/// Once per game, publicly chooses a player: if they are the demon, they die.
fn slay(ctx: &mut ctx::Write<'_>, me: &player::Id, targets: &[player::Id]) {
    // A shot that cannot work (used, or the ability lost) is still aimed,
    // so it looks like any other shot.
    let works = ctx.state().ability_active(me) && ability::use_once(ctx, me, &Def);
    if let Some(target) = aim(ctx, me, targets).filter(|_| works) {
        ability::kill(ctx, me, &target);
    }
}

/// The target, if they register as the Demon to `me`. The storyteller
/// decides the registration even when the shot cannot work (malfunctioning,
/// or a bluff), so no shot gives itself away by the wait.
fn aim(ctx: &mut ctx::Write<'_>, me: &player::Id, targets: &[player::Id]) -> Option<player::Id> {
    let target = targets.first()?.clone();
    let demon = ctx.registers_as(me, &target, character::Kind::Demon).is_some();
    (demon && !ctx.state().is_malfunctioning(me)).then_some(target)
}
