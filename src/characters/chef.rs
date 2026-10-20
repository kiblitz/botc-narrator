use crate::ability;
use crate::character;
use crate::ctx;
use crate::player;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "chef"
    }

    fn name(&self) -> &'static str {
        "Chef"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Townsfolk
    }

    fn night_order(&self) -> character::NightOrder {
        character::NightOrder {
            first: Some(36),
            other: None,
        }
    }

    fn night_action(&self) -> Option<character::NightAction> {
        Some(character::NightAction::ReadOnly(act))
    }
}

/// Learns how many pairs of evil players sit next to each other.
fn act(ctx: &mut ctx::Read<'_>, me: &player::Id, _night: u32) {
    let seats = ctx.state().seat_order();
    let evil: Vec<bool> = seats.iter().map(|p| ctx.registers_evil(me, p)).collect();
    let pairs = (0..seats.len())
        .filter(|&i| evil[i] && evil[(i + 1) % seats.len()])
        .count();
    let max = ctx.state().evil_ids().len();
    ctx.wake(me);
    ability::tell_count(ctx, me, pairs, max, "evil pairs");
    ctx.sleep(me);
}
