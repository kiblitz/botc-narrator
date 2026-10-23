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

    fn day_targets(&self) -> Option<character::Targets> {
        Some(character::Targets {
            prompt: "Who do you want to kill?",
            count: 1,
            candidates: |state, _| state.alive_ids(),
        })
    }

    fn claim(&self) -> Option<character::Claim> {
        Some(character::Claim {
            limit: character::ClaimLimit::EachDay,
            visibility: character::Visibility::Public {
                announcement: "uses the Psychopath ability on",
                resolves_now: true,
            },
            bluff: |_, _, _| {},
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
fn kill(ctx: &mut ctx::Write<'_>, me: &player::Id, targets: &[player::Id]) {
    if !ctx.state().is_alive(me) {
        return;
    }
    if let Some(target) = targets.first() {
        ability::kill(ctx, me, target);
    }
}
