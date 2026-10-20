use crate::character;
use crate::ctx;
use crate::player;
use crate::storyteller;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "mayor"
    }

    fn name(&self) -> &'static str {
        "Mayor"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Townsfolk
    }

    /// The storyteller may have another player die instead (or the Mayor),
    /// but never the demon.
    fn redirect_demon_kill(&self, ctx: &mut ctx::Write<'_>, me: &player::Id) -> Option<player::Id> {
        let state = ctx.state();
        let mut options = vec![me.clone()];
        options.extend(
            state
                .alive_except(me)
                .into_iter()
                .filter(|p| state.kind(p) != character::Kind::Demon),
        );
        let reason = storyteller::Reason::MayorRedirect { mayor: me.clone() };
        ctx.choose_player(reason, options)
    }
}
