//! Undertaker (Townsfolk): each night after the first, learn which character
//! was executed earlier today.

use crate::ability::Ability;
use crate::ctx::Ctx;
use crate::ids::PlayerId;
use crate::role::{Alignment, CharacterInfo, Kind};
use crate::token::Token;

character!(
    Undertaker,
    "undertaker",
    "Undertaker",
    Kind::Townsfolk,
    Alignment::Good
);

impl Ability for Undertaker {
    fn info(&self) -> CharacterInfo {
        Self::INFO
    }

    fn on_night(&self, ctx: &mut Ctx, me: PlayerId, night: u32) {
        if night <= 1 || !ctx.grim.is_alive(me) {
            return;
        }
        // The player executed today carries the DiedToday token.
        let Some(dead) = ctx
            .grim
            .seats()
            .find(|&id| ctx.grim.has_token(id, Token::DiedToday))
        else {
            return; // no execution today: the Undertaker is not woken.
        };
        ctx.wake(me);
        let shown = if ctx.is_impaired(me) {
            let names = ctx.roster_character_names();
            names[ctx.choose("undertaker false info", &names)].clone()
        } else {
            let cid = ctx.registers_as_character(dead, "undertaker");
            ctx.registry().info(cid).name.to_string()
        };
        ctx.reveal(me, &format!("The {shown} was executed today"));
        ctx.sleep(me);
    }
}
