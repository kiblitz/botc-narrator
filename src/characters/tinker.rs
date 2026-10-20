use crate::ability;
use crate::character;
use crate::ctx;
use crate::player;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "tinker"
    }

    fn name(&self) -> &'static str {
        "Tinker"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Outsider
    }

    fn night_order(&self) -> character::NightOrder {
        character::NightOrder {
            first: None,
            other: Some(49),
        }
    }

    fn night_action(&self) -> Option<character::NightAction> {
        Some(character::NightAction::ReadWrite(act))
    }
}

/// Fewest living players at which the storyteller considers killing the
/// Tinker: with fewer, its death would leave two players and end the game.
const MIN_ALIVE: usize = 4;

/// Might die at any time: each night the storyteller decides (never when it
/// would end the game). A malfunctioning Tinker cannot die this way.
fn act(ctx: &mut ctx::Write<'_>, me: &player::Id, _night: u32) {
    if ctx.state().alive_ids().len() < MIN_ALIVE || ctx.state().is_malfunctioning(me) {
        return;
    }
    if ability::storyteller_agrees(&mut ctx.read(), me, &Def, "does the Tinker die tonight?") {
        ability::kill(ctx, me, me);
    }
}
