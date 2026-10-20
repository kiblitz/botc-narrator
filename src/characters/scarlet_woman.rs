use crate::character;
use crate::ctx;
use crate::event;
use crate::player;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "scarlet_woman"
    }

    fn name(&self) -> &'static str {
        "Scarlet Woman"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Minion
    }

    /// Becomes the demon if the demon dies while 5 or more players were alive.
    fn on_death(
        &self,
        ctx: &mut ctx::Write<'_>,
        me: &player::Id,
        dead: &player::Id,
        _cause: &event::Cause,
    ) {
        let state = ctx.state();
        let demon = state.character(dead);
        let alive_before_death = state.alive_ids().len() + 1;
        if me != dead
            && state.is_alive(me)
            && demon.kind() == character::Kind::Demon
            && !state.demon_alive()
            && alive_before_death >= 5
            && !state.is_malfunctioning(me)
        {
            ctx.transform(me, demon);
        }
    }
}
