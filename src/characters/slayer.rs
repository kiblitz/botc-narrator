//! Slayer (Townsfolk): once per game, publicly choose a player. If they
//! register as the Demon, they die.

use crate::ability::Ability;
use crate::ctx::Ctx;
use crate::ids::PlayerId;
use crate::role::{Alignment, CharacterInfo, Kind};
use crate::token::Token;

character!(Slayer, "slayer", "Slayer", Kind::Townsfolk, Alignment::Good);

impl Ability for Slayer {
    fn info(&self) -> CharacterInfo {
        Self::INFO
    }

    fn on_day_ability(&self, ctx: &mut Ctx, me: PlayerId, target: PlayerId) {
        if ctx.grim.has_token(me, Token::UsedAbility) {
            return;
        }
        ctx.grim.add_token(me, Token::UsedAbility);
        let shooter = ctx.label(me);
        let victim = ctx.label(target);
        ctx.log(&format!("{shooter} slays {victim}"));
        if ctx.is_impaired(me) {
            return;
        }
        if ctx.registers_as_demon(target, "slayer") {
            ctx.ability_kill(target, "slayer");
        }
    }
}
