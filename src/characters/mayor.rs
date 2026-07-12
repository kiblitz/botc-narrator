//! Mayor (Townsfolk): if you would die at night, the storyteller may choose
//! that another player dies instead. (The Mayor's "3 players, no execution =>
//! good wins" clause is resolved by [`crate::engine::Engine::end_day`].)

use crate::ability::Ability;
use crate::ctx::Ctx;
use crate::event::DeathSource;
use crate::ids::PlayerId;
use crate::role::{Alignment, CharacterInfo, Kind};

character!(Mayor, "mayor", "Mayor", Kind::Townsfolk, Alignment::Good);

impl Ability for Mayor {
    fn info(&self) -> CharacterInfo {
        Self::INFO
    }

    fn redirect_death(&self, ctx: &mut Ctx, me: PlayerId, source: DeathSource) -> Option<PlayerId> {
        // Only the demon's night kill bounces, and only for a healthy Mayor.
        if !source.is_demon() || ctx.is_impaired(me) {
            return None;
        }
        let others = ctx.alive_others(me);
        let mut opts: Vec<String> = others.iter().map(|&id| ctx.label(id)).collect();
        opts.push(format!("{} dies (no bounce)", ctx.label(me)));
        let i = ctx.choose("Mayor: bounce the demon kill to whom?", &opts);
        others.get(i).copied()
    }

    fn on_no_execution(&self, ctx: &mut Ctx, me: PlayerId) {
        // Three players alive and no execution today: good wins.
        if !ctx.is_impaired(me) && ctx.grim.alive_count() == 3 {
            ctx.declare_winner(Alignment::Good, "Mayor: three players and no execution");
        }
    }
}
