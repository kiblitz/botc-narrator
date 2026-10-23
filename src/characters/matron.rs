use crate::character;
use crate::ctx;
use crate::player;

pub struct Def;

/// At most this many pairs swap seats each day.
const MAX_SWAPS: usize = 3;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "matron"
    }

    fn name(&self) -> &'static str {
        "Matron"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Traveller
    }

    fn day_action(&self) -> Option<character::DayFn> {
        Some(swap)
    }
}

/// Chooses up to three pairs of players, one after another, who swap seats.
/// Malfunctioning, the swaps do not happen.
fn swap(ctx: &mut ctx::Write<'_>, me: &player::Id, _targets: &[player::Id]) {
    if !ctx.state().is_alive(me) {
        return;
    }
    for i in 0..MAX_SWAPS {
        if i > 0 && !ctx.ask_yes_no(me, "Swap another pair?") {
            break;
        }
        let seats = ctx.state().seat_order();
        let pair = ctx.ask(me, "Which two players swap seats?", &seats, 2);
        if let [a, b] = pair.as_slice()
            && !ctx.state().is_malfunctioning(me)
        {
            ctx.swap_seats(a, b);
        }
    }
}
