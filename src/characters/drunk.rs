use crate::character;
use crate::ctx;
use crate::narrator;
use crate::player;
use crate::script;
use crate::storyteller;

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
    fn on_setup(&self, ctx: &mut ctx::Write<'_>, me: &player::Id, script: &script::Script) {
        let options: Vec<character::Ref> = script
            .characters
            .iter()
            .copied()
            .filter(|c| {
                c.kind() == character::Kind::Townsfolk && !narrator::in_play(ctx.state(), *c)
            })
            .collect();
        let reason = storyteller::Reason::BelievedCharacter { player: me.clone() };
        let believed = ctx
            .choose(reason, options, 1, |c| {
                storyteller::Choice::Character(c.name())
            })
            .into_iter()
            .next();
        if let Some(believed) = believed {
            ctx.set_believed(me, Some(believed));
        }
    }
}
