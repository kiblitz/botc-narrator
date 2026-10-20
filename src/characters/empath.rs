use crate::character;
use crate::characters;
use crate::ctx;
use crate::info;
use crate::player;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "empath"
    }

    fn name(&self) -> &'static str {
        "Empath"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Townsfolk
    }

    fn night_order(&self) -> character::NightOrder {
        character::NightOrder {
            first: Some(37),
            other: Some(53),
        }
    }

    fn night_action(&self) -> Option<character::NightAction> {
        Some(character::NightAction::ReadOnly(act))
    }
}

/// Learns how many of their two alive neighbours are evil.
fn act(ctx: &mut ctx::Read<'_>, me: &player::Id, _night: u32) {
    let state = ctx.state();
    let alive = state.alive_ids();
    let n = alive.len();
    let i = alive.iter().position(|p| p == me).expect("empath is alive");
    let mut neighbours = vec![&alive[(i + n - 1) % n], &alive[(i + 1) % n]];
    neighbours.dedup();
    neighbours.retain(|p| *p != me);
    let neighbours: Vec<player::Id> = neighbours.into_iter().cloned().collect();
    let count = neighbours
        .iter()
        .filter(|p| ctx.registers_evil(me, p))
        .count();
    let noun = "evil neighbours";
    let truthful = vec![info::Message::Count { count, noun }];
    ctx.wake(me);
    ctx.inform(me, truthful, characters::counts(0..=2, noun));
    ctx.sleep(me);
}
