//! Assassin (Minion): once per game, at night, choose a player — they die,
//! even if something would normally save them.
//!
//! This character is not part of Trouble Brewing. It exists to demonstrate
//! append-only extension: it is a single new file that plugs into the existing
//! hooks (`on_night`), the existing token vocabulary (`UsedAbility`), and the
//! existing death pipeline. `ability_kill` bypasses protection and immunity, so
//! the Assassin kills through a Soldier or Monk with no change to either.

use crate::ability::Ability;
use crate::ctx::Ctx;
use crate::ids::PlayerId;
use crate::role::{Alignment, CharacterInfo, Kind};
use crate::token::Token;

character!(
    Assassin,
    "assassin",
    "Assassin",
    Kind::Minion,
    Alignment::Evil
);

impl Ability for Assassin {
    fn info(&self) -> CharacterInfo {
        Self::INFO
    }

    fn on_night(&self, ctx: &mut Ctx, me: PlayerId, night: u32) {
        if night <= 1 || !ctx.grim.is_alive(me) {
            return;
        }
        // Once per game, and never while impaired.
        if ctx.grim.has_token(me, Token::UsedAbility) || ctx.is_impaired(me) {
            return;
        }
        ctx.wake(me);
        // Choosing yourself means "hold the ability for another night".
        let mut candidates: Vec<PlayerId> = vec![me];
        candidates.extend(ctx.alive_others(me));
        let target = ctx.ask(me, "Assassinate whom? (yourself = hold)", &candidates);
        if target != me {
            ctx.grim.add_token(me, Token::UsedAbility);
            // An unpreventable death: it skips protection and immunity.
            ctx.ability_kill(target, "assassin");
        }
        ctx.sleep(me);
    }
}
