use crate::ability;
use crate::character;
use crate::ctx;
use crate::player;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "mutant"
    }

    fn name(&self) -> &'static str {
        "Mutant"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Outsider
    }

    /// If the Mutant tried to convince the group they are an Outsider, the
    /// storyteller may execute them (if nobody was executed today).
    fn on_day_end(&self, ctx: &mut ctx::Write<'_>, me: &player::Id) {
        if ctx.state().executed_today().is_none()
            && !ctx.state().is_malfunctioning(me)
            && ctx.is_mad_about(me, "Outsider")
            && ability::storyteller_agrees(&mut ctx.read(), me, &Def, "execute the Mutant?")
        {
            ctx.execute(me);
        }
    }
}
