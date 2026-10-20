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

    /// Good wins if only three players live and no one was executed today.
    fn on_day_end(&self, ctx: &mut ctx::Write<'_>, me: &player::Id) {
        let state = ctx.state();
        if state.alive_ids().len() == 3
            && state.executed_today().is_none()
            && !state.is_malfunctioning(me)
        {
            ctx.win(character::Alignment::Good);
        }
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
