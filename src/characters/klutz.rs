use crate::character;
use crate::ctx;
use crate::player;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "klutz"
    }

    fn name(&self) -> &'static str {
        "Klutz"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Outsider
    }

    /// When the Klutz learns they died, they publicly choose a living player:
    /// if that player is evil, the Klutz's team loses.
    fn on_learns_death(&self, ctx: &mut ctx::Write<'_>, me: &player::Id) {
        let candidates = ctx.state().alive_ids();
        let Some(target) = ctx.ask_one(me, "Choose a living player", &candidates) else {
            return;
        };
        if !ctx.state().is_malfunctioning(me) && ctx.registers_evil(me, &target) {
            let team = ctx.state().alignment(me);
            ctx.win(team.opposite());
        }
    }
}
