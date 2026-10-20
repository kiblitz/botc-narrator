use crate::ability;
use crate::character;
use crate::ctx;
use crate::event;
use crate::player;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "tea_lady"
    }

    fn name(&self) -> &'static str {
        "Tea Lady"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Townsfolk
    }

    /// If both alive neighbours are good, they can't die (by any cause).
    fn intercept_death(
        &self,
        ctx: &mut ctx::Write<'_>,
        me: &player::Id,
        target: &player::Id,
        _cause: &event::Cause,
    ) -> character::Interception {
        let state = ctx.state();
        let neighbours = ability::alive_neighbours(state, me);
        if neighbours.len() == 2
            && neighbours.contains(target)
            && neighbours.iter().all(|p| !state.is_evil(p))
        {
            character::Interception::Prevent
        } else {
            character::Interception::Pass
        }
    }
}
