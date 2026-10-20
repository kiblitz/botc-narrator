use crate::character;
use crate::characters;
use crate::ctx;
use crate::player;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "librarian"
    }

    fn name(&self) -> &'static str {
        "Librarian"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Townsfolk
    }

    fn night_action(&self, night: u32) -> Option<character::NightAction> {
        (night == 1).then_some(character::NightAction::ReadOnly(act))
    }
}

/// Learns that one of two players is a particular Outsider.
fn act(ctx: &mut ctx::Read<'_>, me: &player::Id, _night: u32) {
    if ctx.state().is_alive(me) {
        characters::inform_one_of(ctx, me, character::Kind::Outsider);
    }
}
