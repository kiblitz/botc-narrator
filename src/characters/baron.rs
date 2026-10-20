use crate::character;
use crate::ctx;
use crate::player;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "baron"
    }

    fn name(&self) -> &'static str {
        "Baron"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Minion
    }

    fn on_setup(&self, ctx: &mut ctx::Write<'_>, _me: &player::Id) {
        ctx.note("Baron in play: two extra Outsiders replace Townsfolk");
    }
}
