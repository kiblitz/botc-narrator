use crate::ability;
use crate::character;
use crate::ctx;
use crate::game;
use crate::player;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "ravenkeeper"
    }

    fn name(&self) -> &'static str {
        "Ravenkeeper"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Townsfolk
    }

    fn night_order(&self) -> character::NightOrder {
        character::NightOrder {
            first: None,
            other: Some(52),
        }
    }

    fn wakes_tonight(&self, state: &game::State, me: &player::Id) -> bool {
        state.deaths_tonight().contains(me)
    }

    fn night_action(&self) -> Option<character::NightAction> {
        Some(character::NightAction::ReadOnly(act))
    }
}

/// On the night they die, chooses a player and learns their character.
fn act(ctx: &mut ctx::Read<'_>, me: &player::Id, _night: u32) {
    let candidates = ctx.state().seat_order();
    ctx.wake(me);
    if let Some(target) = ctx.ask_one(me, "Choose a player to learn about", &candidates) {
        ability::tell_character(ctx, me, &target);
    }
    ctx.sleep(me);
}
