use crate::ability;
use crate::character;
use crate::ctx;
use crate::player;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "drunk"
    }

    fn name(&self) -> &'static str {
        "Drunk"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Outsider
    }

    /// Thinks they are a not-in-play Townsfolk of the storyteller's choice.
    /// They act as that character, but are drunk, so their ability never
    /// works and their information may be false.
    fn on_setup(&self, ctx: &mut ctx::Write<'_>, me: &player::Id) {
        let options = ability::not_in_play(ctx.state(), character::Kind::Townsfolk);
        let question = "which Townsfolk they believe they are";
        if let Some(believed) =
            ability::storyteller_picks_character(&mut ctx.read(), me, &Def, question, options)
        {
            ctx.set_believed(me, Some(believed));
        }
    }
}
