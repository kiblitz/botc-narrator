use crate::character;
use crate::characters;
use crate::ctx;
use crate::player;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "investigator"
    }

    fn name(&self) -> &'static str {
        "Investigator"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Townsfolk
    }

    fn night_order(&self) -> character::NightOrder {
        character::NightOrder {
            first: Some(35),
            other: None,
        }
    }

    fn night_action(&self) -> Option<character::NightAction> {
        Some(character::NightAction::ReadOnly(act))
    }
}

/// Learns that one of two players is a particular Minion.
fn act(ctx: &mut ctx::Read<'_>, me: &player::Id, _night: u32) {
    characters::inform_one_of(ctx, me, character::Kind::Minion);
}
