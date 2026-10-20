use crate::ability;
use crate::character;
use crate::ctx;
use crate::game;
use crate::player;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "chambermaid"
    }

    fn name(&self) -> &'static str {
        "Chambermaid"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Townsfolk
    }

    fn night_order(&self) -> character::NightOrder {
        character::NightOrder {
            first: Some(76),
            other: Some(95),
        }
    }

    fn night_action(&self) -> Option<character::NightAction> {
        Some(character::NightAction::ReadOnly(act))
    }

    /// Not woken when fewer than 2 other living players can be chosen.
    fn wakes_tonight(&self, state: &game::State, me: &player::Id) -> bool {
        state.is_alive(me) && state.alive_except(me).len() >= 2
    }
}

/// Each night, chooses 2 living players (not themself): learns how many woke
/// tonight due to their own ability.
fn act(ctx: &mut ctx::Read<'_>, me: &player::Id, night: u32) {
    let candidates = ctx.state().alive_except(me);
    ctx.wake(me);
    let chosen = ctx.ask(me, "Choose 2 players:", &candidates, 2);
    let woke = chosen
        .iter()
        .filter(|p| ability::wakes_for_ability(ctx.state(), p, night))
        .count();
    ability::tell_count(ctx, me, woke, chosen.len(), "woke");
    ctx.sleep(me);
}
