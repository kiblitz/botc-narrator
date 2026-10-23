use crate::ability;
use crate::character;
use crate::ctx;
use crate::event;
use crate::player;
use crate::table;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "psychopath"
    }

    fn name(&self) -> &'static str {
        "Psychopath"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Minion
    }

    fn day_action(&self) -> Option<character::DayFn> {
        Some(kill)
    }

    fn public_claim(&self) -> Option<character::PublicClaim> {
        Some(character::PublicClaim {
            limit: character::ClaimLimit::EachDay,
            announcement: "uses the Psychopath ability",
            resolves_now: true,
            bluff: |ctx, me| {
                target(ctx, me);
            },
        })
    }

    /// If executed, only dies on losing roshambo against the nominator (the
    /// storyteller if they nominated themself).
    fn intercept_death(
        &self,
        ctx: &mut ctx::Write<'_>,
        me: &player::Id,
        target: &player::Id,
        cause: &event::Cause,
    ) -> character::Interception {
        if target != me || *cause != event::Cause::Execution {
            return character::Interception::Pass;
        }
        let against = ability::nominations_today(ctx.state())
            .into_iter()
            .rev()
            .find(|(_, nominee)| nominee == me)
            .and_then(|(nominator, _)| nominator)
            .filter(|nominator| nominator != me);
        let lost = ctx.confirm(me, &table::Question::LostRoshambo { against });
        ctx.announce(format!(
            "{me} {} roshambo",
            if lost { "loses" } else { "does not lose" }
        ));
        if lost {
            character::Interception::Pass
        } else {
            character::Interception::Prevent
        }
    }
}

/// Each day, before nominations, may publicly choose a player: they die.
fn kill(ctx: &mut ctx::Write<'_>, me: &player::Id) {
    if !ctx.state().is_alive(me) {
        return;
    }
    if let Some(target) = target(ctx, me) {
        ability::kill(ctx, me, &target);
    }
}

/// Asks whom `me` chooses to kill.
fn target(ctx: &mut ctx::Write<'_>, me: &player::Id) -> Option<player::Id> {
    let candidates = ctx.state().alive_ids();
    ctx.ask_one(me, "Who do you want to kill?", &candidates)
}
