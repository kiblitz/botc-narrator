use crate::ability;
use crate::character;
use crate::ctx;
use crate::player;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "town_crier"
    }

    fn name(&self) -> &'static str {
        "Town Crier"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Townsfolk
    }

    fn night_order(&self) -> character::NightOrder {
        character::NightOrder {
            first: None,
            other: Some(58),
        }
    }

    fn night_action(&self) -> Option<character::NightAction> {
        Some(character::NightAction::ReadOnly(act))
    }
}

/// Learns whether a Minion (as they register) nominated today.
fn act(ctx: &mut ctx::Read<'_>, me: &player::Id, _night: u32) {
    let nominators = ability::nominators_today(ctx.state());
    let minion_nominated = nominators
        .iter()
        .any(|n| ctx.registers_as(me, n, character::Kind::Minion).is_some());
    ctx.wake(me);
    ability::tell_yes_no(ctx, me, minion_nominated);
    ctx.sleep(me);
}
