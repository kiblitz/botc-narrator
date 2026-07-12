//! Poisoner (Minion): each night, choose a player; they are poisoned until you
//! act again or die.

use crate::ability::Ability;
use crate::ctx::Ctx;
use crate::event::DeathSource;
use crate::ids::PlayerId;
use crate::role::{Alignment, CharacterInfo, Kind};
use crate::token::Token;

character!(
    Poisoner,
    "poisoner",
    "Poisoner",
    Kind::Minion,
    Alignment::Evil
);

impl Ability for Poisoner {
    fn info(&self) -> CharacterInfo {
        Self::INFO
    }

    fn on_night(&self, ctx: &mut Ctx, me: PlayerId, _night: u32) {
        if !ctx.grim.is_alive(me) {
            return;
        }
        ctx.wake(me);
        let targets = ctx.alive_others(me);
        let target = ctx.ask(me, "Who do you want to poison?", &targets);
        // Move the single poison token from wherever it was to the new target.
        ctx.grim.clear_token_kind_everywhere(Token::Poisoned);
        ctx.grim.add_token(target, Token::Poisoned);
        ctx.sleep(me);
    }

    fn on_any_death(&self, ctx: &mut Ctx, me: PlayerId, dead: PlayerId, _source: DeathSource) {
        // When the Poisoner dies, their poison wears off.
        if me == dead {
            ctx.grim.clear_token_kind_everywhere(Token::Poisoned);
        }
    }
}
