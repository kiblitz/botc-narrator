use crate::ability;
use crate::character;
use crate::ctx;
use crate::event;
use crate::game;
use crate::player;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "minstrel"
    }

    fn name(&self) -> &'static str {
        "Minstrel"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Townsfolk
    }

    /// When a Minion dies by execution, all other players (except
    /// Travellers) are drunk until dusk tomorrow.
    fn on_death(
        &self,
        ctx: &mut ctx::Write<'_>,
        me: &player::Id,
        dead: &player::Id,
        cause: &event::Cause,
    ) {
        let state = ctx.state();
        let game::Phase::Day(day) = state.phase() else {
            return;
        };
        if *cause != event::Cause::Execution
            || dead == me
            || !state.is_alive(me)
            || state.kind(dead) != character::Kind::Minion
        {
            return;
        }
        let others: Vec<player::Id> = ability::others(state, me)
            .into_iter()
            .filter(|p| state.kind(p) != character::Kind::Traveller)
            .collect();
        // Dusk tomorrow is when night `day + 2` begins.
        let expiry = game::Expiry::Night(day + 2);
        ability::mark_all(ctx, me, &Def, &others, game::Effect::Drunk, expiry);
    }
}
