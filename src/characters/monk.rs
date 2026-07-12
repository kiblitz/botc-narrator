//! Monk (Townsfolk): each night from the second, protect a player from the
//! Demon. A poisoned/drunk Monk protects no one.

use crate::ability::Ability;
use crate::ctx::Ctx;
use crate::ids::PlayerId;
use crate::role::{Alignment, CharacterInfo, Kind};
use crate::token::Token;

character!(Monk, "monk", "Monk", Kind::Townsfolk, Alignment::Good);

impl Ability for Monk {
    fn info(&self) -> CharacterInfo {
        Self::INFO
    }

    fn on_night(&self, ctx: &mut Ctx, me: PlayerId, night: u32) {
        // The Monk does not act on the first night.
        if night <= 1 || !ctx.grim.is_alive(me) {
            return;
        }
        ctx.wake(me);
        let targets = ctx.alive_others(me);
        let target = ctx.ask(me, "Who do you want to protect?", &targets);
        // A malfunctioning Monk places no real protection.
        if !ctx.is_impaired(me) {
            ctx.grim.add_token(target, Token::Protected);
        }
        ctx.sleep(me);
    }
}
