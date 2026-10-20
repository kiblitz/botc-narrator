use crate::ability;
use crate::character;
use crate::ctx;
use crate::player;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "politician"
    }

    fn name(&self) -> &'static str {
        "Politician"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Outsider
    }

    /// If the Politician's team lost and the storyteller judges them the
    /// player most responsible, they change alignment and win, even if dead.
    fn on_win(&self, ctx: &mut ctx::Write<'_>, me: &player::Id, winner: character::Alignment) {
        let question = "was the Politician most responsible for their team losing?";
        if ctx.state().alignment(me) != winner
            && !ctx.state().is_malfunctioning(me)
            && ability::storyteller_agrees(&mut ctx.read(), me, &Def, question)
        {
            ctx.set_alignment(me, winner);
        }
    }
}
