use crate::character;
use crate::characters;
use crate::ctx;
use crate::info;
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

    fn night_action(&self, night: u32) -> Option<character::NightAction> {
        (night == 1).then_some(character::NightAction::ReadOnly(act))
    }
}

/// Learns how many pairs of evil players sit next to each other.
fn act(ctx: &mut ctx::Read<'_>, me: &player::Id, _night: u32) {
    let state = ctx.state();
    if !state.is_alive(me) {
        return;
    }
    let seats = state.seat_order();
    let pairs = (0..seats.len())
        .filter(|&i| state.is_evil(&seats[i]) && state.is_evil(&seats[(i + 1) % seats.len()]))
        .count();
    let noun = "evil pairs";
    let truthful = vec![info::Message::Count { count: pairs, noun }];
    let misinformation = characters::counts(0..=state.evil_ids().len(), noun);
    ctx.wake(me);
    ctx.inform(me, truthful, misinformation);
    ctx.sleep(me);
}
