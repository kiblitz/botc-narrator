use crate::ability;
use crate::character;
use crate::ctx;
use crate::game;
use crate::player;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "golem"
    }

    fn name(&self) -> &'static str {
        "Golem"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Outsider
    }

    /// The Golem may only nominate once per game.
    fn can_nominate(&self, state: &game::State, me: &player::Id) -> bool {
        !ability::used(state, me, &Def)
    }

    /// When the Golem nominates, the nominee dies unless they are the Demon.
    /// The nomination is used up even when malfunctioning.
    fn on_nominates(&self, ctx: &mut ctx::Write<'_>, me: &player::Id, nominee: &player::Id) {
        if ability::use_once(ctx, me, &Def)
            && ctx
                .registers_as(me, nominee, character::Kind::Demon)
                .is_none()
        {
            ability::kill(ctx, me, nominee);
        }
    }
}
