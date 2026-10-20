use crate::ability;
use crate::character;
use crate::ctx;
use crate::player;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "clockmaker"
    }

    fn name(&self) -> &'static str {
        "Clockmaker"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Townsfolk
    }

    fn night_order(&self) -> character::NightOrder {
        character::NightOrder {
            first: Some(60),
            other: None,
        }
    }

    fn night_action(&self) -> Option<character::NightAction> {
        Some(character::NightAction::ReadOnly(act))
    }
}

/// Starts knowing how many steps from the Demon to its nearest Minion,
/// counting the shorter way round (adjacent is 1), as players register.
fn act(ctx: &mut ctx::Read<'_>, me: &player::Id, _night: u32) {
    let seats = ctx.state().seat_order();
    let mut demons = Vec::new();
    let mut minions = Vec::new();
    for (i, p) in seats.iter().enumerate() {
        if ctx.registers_as(me, p, character::Kind::Demon).is_some() {
            demons.push(i);
        }
        if ctx.registers_as(me, p, character::Kind::Minion).is_some() {
            minions.push(i);
        }
    }
    let n = seats.len();
    let steps = |a: usize, b: usize| {
        let d = a.abs_diff(b);
        d.min(n - d)
    };
    let nearest = demons
        .iter()
        .flat_map(|d| minions.iter().filter(move |m| *m != d).map(|m| steps(*d, *m)))
        .min()
        // No Demon and Minion register: nothing true to count, so 0.
        .unwrap_or(0);
    ctx.wake(me);
    ability::tell_count(ctx, me, nearest, n / 2, "steps");
    ctx.sleep(me);
}
