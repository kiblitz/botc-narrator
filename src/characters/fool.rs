use crate::ability;
use crate::character;
use crate::ctx;
use crate::event;
use crate::player;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "fool"
    }

    fn name(&self) -> &'static str {
        "Fool"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Townsfolk
    }

    /// The first time they would die, they don't. Only asked while sober, and
    /// after other players' protection, so the ability is not spent otherwise.
    fn intercept_death(
        &self,
        ctx: &mut ctx::Write<'_>,
        me: &player::Id,
        target: &player::Id,
        _cause: &event::Cause,
    ) -> character::Interception {
        if target == me && ability::use_once(ctx, me, &Def) {
            character::Interception::Prevent
        } else {
            character::Interception::Pass
        }
    }
}
