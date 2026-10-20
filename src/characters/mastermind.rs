use crate::character;
use crate::ctx;
use crate::event;
use crate::game;
use crate::player;

pub struct Def;

/// On the Mastermind: the Demon was executed on the day this was placed.
const EXTRA_DAY: game::Effect = game::Effect::Custom("Extra day");

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "mastermind"
    }

    fn name(&self) -> &'static str {
        "Mastermind"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Minion
    }

    /// If the Demon dies by execution (ending the game), play for 1 more day.
    fn on_death(
        &self,
        ctx: &mut ctx::Write<'_>,
        me: &player::Id,
        dead: &player::Id,
        cause: &event::Cause,
    ) {
        let state = ctx.state();
        if *cause == event::Cause::Execution
            && state.kind(dead) == character::Kind::Demon
            && !state.demon_alive()
            && state.is_alive(me)
            && !state.is_malfunctioning(me)
            && extra_day_from(state, me).is_none()
        {
            let reminder = game::Reminder::new(&Def, me, EXTRA_DAY, game::Expiry::Never);
            ctx.place_reminder(me, reminder);
        }
    }

    /// Nobody wins until the extra day is over, while the Mastermind lives.
    fn vetoes_win(
        &self,
        ctx: &mut ctx::Write<'_>,
        me: &player::Id,
        _winner: character::Alignment,
    ) -> bool {
        let state = ctx.state();
        state.is_alive(me) && extra_day_from(state, me).is_some_and(|day| match state.phase() {
            game::Phase::Day(d) => d == day || d == day + 1,
            game::Phase::Night(n) => n == day + 1,
            game::Phase::Setup => false,
        })
    }

    /// On the extra day, an executed player's team loses.
    fn on_execution(&self, ctx: &mut ctx::Write<'_>, me: &player::Id, executed: &player::Id) {
        if on_extra_day(ctx.state(), me) {
            let loser = ctx.state().alignment(executed);
            ctx.win(loser.opposite());
        }
    }

    /// The extra day ends without an execution: good wins.
    fn on_day_end(&self, ctx: &mut ctx::Write<'_>, me: &player::Id) {
        if on_extra_day(ctx.state(), me) {
            ctx.win(character::Alignment::Good);
        }
    }
}

fn on_extra_day(state: &game::State, me: &player::Id) -> bool {
    extra_day_from(state, me).is_some_and(|day| state.phase() == game::Phase::Day(day + 1))
}

/// The day the Demon was executed, if the Mastermind's ability triggered.
fn extra_day_from(state: &game::State, me: &player::Id) -> Option<u32> {
    let history = state.history();
    let placed = history.iter().position(|e| {
        matches!(e, event::Event::ReminderPlaced { player, reminder }
            if player == me && reminder.source == character::Def::id(&Def) && reminder.effect == EXTRA_DAY)
    })?;
    history[..placed].iter().rev().find_map(|e| match e {
        event::Event::PhaseBegan(game::Phase::Day(d)) => Some(*d),
        _ => None,
    })
}
