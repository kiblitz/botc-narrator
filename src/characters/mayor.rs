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

    /// The storyteller may have another player die instead (or the Mayor).
    fn redirect_demon_kill(&self, ctx: &mut ctx::Write<'_>, me: &player::Id) -> Option<player::Id> {
        let mut options = vec![storyteller::Choice::Player(me.clone())];
        options.extend(
            ctx.state()
                .alive_except(me)
                .into_iter()
                .map(storyteller::Choice::Player),
        );
        let reason = storyteller::Reason::MayorRedirect { mayor: me.clone() };
        match ctx.decide(reason, options, 1).into_iter().next() {
            Some(storyteller::Choice::Player(p)) => Some(p),
            _ => None,
        }
    }
}
