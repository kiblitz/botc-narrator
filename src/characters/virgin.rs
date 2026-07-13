//! Virgin (Townsfolk): the first time you are nominated, if the nominator is a
//! Townsfolk, they are executed immediately.

use crate::ability::Ability;
use crate::ctx::Ctx;
use crate::ids::PlayerId;
use crate::role::{Alignment, CharacterInfo, Kind};
use crate::token::Token;

character!(Virgin, "virgin", "Virgin", Kind::Townsfolk, Alignment::Good);

impl Ability for Virgin {
    fn info(&self) -> CharacterInfo {
        Self::INFO
    }

    fn on_nominated(&self, ctx: &mut Ctx, me: PlayerId, nominator: PlayerId) {
        // The ability fires on the *first* nomination and is then spent, whether
        // or not it executes anyone.
        if ctx.grim.has_token(me, Token::UsedAbility) {
            return;
        }
        ctx.grim.add_token(me, Token::UsedAbility);
        if ctx.is_impaired(me) {
            return;
        }
        if ctx.registers_as_kind(nominator, Kind::Townsfolk, "virgin") {
            let who = ctx.label(nominator);
            ctx.log(&format!("{who} nominated the Virgin and is executed"));
            ctx.execute(nominator);
        }
    }
}
