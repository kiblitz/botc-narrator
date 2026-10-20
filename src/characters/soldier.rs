use crate::character;
use crate::ctx;
use crate::event;
use crate::player;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "soldier"
    }

    fn name(&self) -> &'static str {
        "Soldier"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Townsfolk
    }

    /// Safe from the demon.
    fn intercept_death(
        &self,
        _ctx: &mut ctx::Write<'_>,
        me: &player::Id,
        target: &player::Id,
        cause: &event::Cause,
    ) -> character::Interception {
        if target == me && *cause == event::Cause::Demon {
            character::Interception::Prevent
        } else {
            character::Interception::Pass
        }
    }
}
