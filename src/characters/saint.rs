use crate::character;
use crate::ctx;
use crate::player;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "saint"
    }

    fn name(&self) -> &'static str {
        "Saint"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Outsider
    }

    /// Evil wins if the Saint is executed.
    fn on_executed(&self, ctx: &mut ctx::Write<'_>, me: &player::Id) {
        if !ctx.state().is_malfunctioning(me) {
            ctx.win(character::Alignment::Evil);
        }
    }
}
