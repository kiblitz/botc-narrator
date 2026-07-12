//! Butler (Outsider): each night, choose a master. You may only vote when your
//! master votes. (Voting is a downstream concern; the Butler simply records the
//! master token.)

use crate::ability::Ability;
use crate::ctx::Ctx;
use crate::ids::PlayerId;
use crate::role::{Alignment, CharacterInfo, Kind};
use crate::token::Token;

character!(Butler, "butler", "Butler", Kind::Outsider, Alignment::Good);

impl Ability for Butler {
    fn info(&self) -> CharacterInfo {
        Self::INFO
    }

    fn on_night(&self, ctx: &mut Ctx, me: PlayerId, _night: u32) {
        if !ctx.grim.is_alive(me) {
            return;
        }
        ctx.wake(me);
        let targets = ctx.alive_others(me);
        let master = ctx.ask(me, "Choose your master", &targets);
        // Replace any previous master with tonight's choice.
        ctx.grim.remove_tokens_of_kind(me, Token::Master(me));
        ctx.grim.add_token(me, Token::Master(master));
        ctx.sleep(me);
    }
}
