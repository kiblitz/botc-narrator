//! Fortune Teller (Townsfolk): each night, choose two players and learn if
//! either registers as the Demon. One good player is a "red herring" who always
//! registers as the Demon to you.

use crate::ability::Ability;
use crate::ctx::Ctx;
use crate::ids::PlayerId;
use crate::role::{Alignment, CharacterInfo, Kind};
use crate::token::Token;

character!(
    FortuneTeller,
    "fortune_teller",
    "Fortune Teller",
    Kind::Townsfolk,
    Alignment::Good
);

impl Ability for FortuneTeller {
    fn info(&self) -> CharacterInfo {
        Self::INFO
    }

    fn on_setup(&self, ctx: &mut Ctx, me: PlayerId) {
        // The storyteller marks one good player as the red herring.
        let good: Vec<PlayerId> = ctx
            .grim
            .seats()
            .filter(|&id| !ctx.grim.get(id).alignment.is_evil())
            .collect();
        if good.is_empty() {
            return;
        }
        let labels: Vec<String> = good.iter().map(|&id| ctx.label(id)).collect();
        let i = ctx.choose(&format!("red herring for {}", ctx.label(me)), &labels);
        ctx.grim.add_token(good[i], Token::RedHerring);
    }

    fn on_night(&self, ctx: &mut Ctx, me: PlayerId, _night: u32) {
        if !ctx.grim.is_alive(me) {
            return;
        }
        ctx.wake(me);
        let candidates: Vec<PlayerId> = ctx.grim.alive().collect();
        let p1 = ctx.ask(me, "Choose a player", &candidates);
        let p2 = ctx.ask(me, "Choose a player", &candidates);
        let truth = ctx.registers_as_demon(p1, "fortune_teller")
            || ctx.registers_as_demon(p2, "fortune_teller");
        let shown = ctx.deliver(me, yes_no(truth), &["Yes", "No"]);
        ctx.reveal(me, shown);
        ctx.sleep(me);
    }
}

fn yes_no(b: bool) -> &'static str {
    if b {
        "Yes"
    } else {
        "No"
    }
}
