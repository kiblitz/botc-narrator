use crate::ability;
use crate::character;
use crate::ctx;
use crate::player;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "virgin"
    }

    fn name(&self) -> &'static str {
        "Virgin"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Townsfolk
    }

    /// The first time she is nominated, a Townsfolk nominator is executed.
    fn on_nominated(&self, ctx: &mut ctx::Write<'_>, me: &player::Id, nominator: &player::Id) {
        if ability::use_once(ctx, me, &Def)
            && !ctx.state().is_malfunctioning(me)
            && ctx
                .registers_as(me, nominator, character::Kind::Townsfolk)
                .is_some()
        {
            ctx.execute(nominator);
        }
    }
}
