use crate::ability;
use crate::character;
use crate::ctx;
use crate::event;
use crate::player;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "pacifist"
    }

    fn name(&self) -> &'static str {
        "Pacifist"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Townsfolk
    }

    /// Executed good players might not die: the storyteller decides.
    fn intercept_death(
        &self,
        ctx: &mut ctx::Write<'_>,
        me: &player::Id,
        target: &player::Id,
        cause: &event::Cause,
    ) -> character::Interception {
        let question = "does the executed good player survive?";
        if *cause == event::Cause::Execution
            && !ctx.state().is_evil(target)
            && ability::storyteller_agrees(&mut ctx.read(), me, &Def, question)
        {
            character::Interception::Prevent
        } else {
            character::Interception::Pass
        }
    }
}
