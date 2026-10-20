use crate::ability;
use crate::character;
use crate::ctx;
use crate::event;
use crate::game;
use crate::player;

pub struct Def;

const ID: &str = "zombuul";

impl character::Def for Def {
    fn id(&self) -> &'static str {
        ID
    }

    fn name(&self) -> &'static str {
        "Zombuul"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Demon
    }

    fn night_order(&self) -> character::NightOrder {
        character::NightOrder {
            first: None,
            other: Some(41),
        }
    }

    fn night_action(&self) -> Option<character::NightAction> {
        Some(character::NightAction::ReadWrite(act))
    }

    /// Each night*, if no-one died today (its own first death counts).
    fn wakes_tonight(&self, state: &game::State, me: &player::Id) -> bool {
        let seemed_to_die = ability::today(state).iter().any(|e| match e {
            event::Event::KillPrevented { player, by } => player == me && *by == ID,
            _ => false,
        });
        state.is_alive(me) && ability::died_today(state).is_empty() && !seemed_to_die
    }

    /// The first time it dies, it lives (once per game, as `AbilityUsed`).
    /// A malfunctioning Zombuul is not asked, so it dies for real.
    fn intercept_death(
        &self,
        ctx: &mut ctx::Write<'_>,
        me: &player::Id,
        target: &player::Id,
        _cause: &event::Cause,
    ) -> character::Interception {
        if target == me && ability::use_once(ctx, me, &Def) {
            ctx.note(format!("{me} (Zombuul) seems to die but lives"));
            character::Interception::Prevent
        } else {
            character::Interception::Pass
        }
    }
}

/// Kills on nights after a day without deaths.
fn act(ctx: &mut ctx::Write<'_>, me: &player::Id, _night: u32) {
    ability::demon_attack(ctx, me);
}
