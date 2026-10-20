use crate::ability;
use crate::character;
use crate::ctx;
use crate::game;
use crate::player;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "leviathan"
    }

    fn name(&self) -> &'static str {
        "Leviathan"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Demon
    }

    /// Its positions only mark the day count; it never wakes.
    fn night_order(&self) -> character::NightOrder {
        character::NightOrder {
            first: Some(79),
            other: Some(99),
        }
    }

    /// All players know you are in play.
    fn on_setup(&self, ctx: &mut ctx::Write<'_>, _me: &player::Id) {
        ctx.announce("The Leviathan is in play");
    }

    /// At the end of each day: a second good player executed, or day 5
    /// ending, and evil wins. Checked at dusk (an execution counts even if
    /// the player survived it).
    fn on_day_end(&self, ctx: &mut ctx::Write<'_>, me: &player::Id) {
        let state = ctx.state();
        if !state.is_alive(me) || state.is_malfunctioning(me) {
            return;
        }
        if let Some(executed) = state.executed_today().cloned()
            && !state.is_evil(&executed)
        {
            let earlier = ability::marked(state, me, &Def, GOOD_EXECUTED);
            if earlier.is_empty() {
                let token = game::Reminder::new(&Def, me, GOOD_EXECUTED, game::Expiry::Never);
                ctx.place_reminder(&executed, token);
            } else {
                ctx.win(character::Alignment::Evil);
                return;
            }
        }
        if matches!(ctx.state().phase(), game::Phase::Day(day) if day >= LAST_DAY) {
            ctx.win(character::Alignment::Evil);
        }
    }
}

const GOOD_EXECUTED: game::Effect = game::Effect::Custom("Good player executed");

/// After this day, evil wins.
const LAST_DAY: u32 = 5;
