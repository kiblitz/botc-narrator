use crate::character;
use crate::ctx;
use crate::info;
use crate::player;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "shugenja"
    }

    fn name(&self) -> &'static str {
        "Shugenja"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Townsfolk
    }

    fn night_order(&self) -> character::NightOrder {
        character::NightOrder {
            first: Some(67),
            other: None,
        }
    }

    fn night_action(&self) -> Option<character::NightAction> {
        Some(character::NightAction::ReadOnly(act))
    }
}

/// Starts knowing whether the closest evil player (as players register) is
/// clockwise or anticlockwise; if equidistant, either answer is true.
fn act(ctx: &mut ctx::Read<'_>, me: &player::Id, _night: u32) {
    let seats = ctx.state().seat_order();
    let n = seats.len();
    let i = seats.iter().position(|p| p == me).unwrap_or(0);
    let mut truthful = Vec::new();
    for k in 1..=n / 2 {
        let clockwise = &seats[(i + k) % n];
        let anticlockwise = &seats[(i + n - k) % n];
        if ctx.registers_evil(me, clockwise) {
            truthful.push(info::Direction::Clockwise);
        }
        if anticlockwise != clockwise && ctx.registers_evil(me, anticlockwise) {
            truthful.push(info::Direction::Anticlockwise);
        }
        if !truthful.is_empty() {
            break;
        }
    }
    let all = [info::Direction::Clockwise, info::Direction::Anticlockwise];
    if truthful.is_empty() {
        // Nobody evil: nothing true to say, so either direction.
        truthful = all.to_vec();
    }
    let misinformation: Vec<info::Direction> =
        all.into_iter().filter(|d| !truthful.contains(d)).collect();
    ctx.wake(me);
    ctx.inform(
        me,
        truthful.into_iter().map(info::Message::Direction).collect(),
        misinformation.into_iter().map(info::Message::Direction).collect(),
    );
    ctx.sleep(me);
}
