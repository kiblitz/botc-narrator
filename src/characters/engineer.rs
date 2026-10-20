use crate::ability;
use crate::character;
use crate::ctx;
use crate::game;
use crate::info;
use crate::player;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "engineer"
    }

    fn name(&self) -> &'static str {
        "Engineer"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Townsfolk
    }

    fn night_order(&self) -> character::NightOrder {
        character::NightOrder {
            first: Some(13),
            other: Some(5),
        }
    }

    fn night_action(&self) -> Option<character::NightAction> {
        Some(character::NightAction::ReadWrite(act))
    }

    fn wakes_tonight(&self, state: &game::State, me: &player::Id) -> bool {
        state.is_alive(me) && !ability::used(state, me, &Def)
    }
}

/// Once per game, chooses which Demon or which Minions are in play: evil
/// players of that kind become the chosen characters (Minions already one of
/// them stay) and are woken and told. Setup changes of the new characters do
/// not apply. Drunk or poisoned, the ability is spent for nothing.
fn act(ctx: &mut ctx::Write<'_>, me: &player::Id, _night: u32) {
    let evil: Vec<character::Ref> = ctx
        .state()
        .script()
        .iter()
        .copied()
        .filter(|c| c.kind().alignment() == Some(character::Alignment::Evil))
        .collect();
    ctx.wake(me);
    let Some(first) = ctx.ask_character(me, "Which Demon or Minion is in play?", &evil) else {
        ctx.sleep(me);
        return;
    };
    let kind = first.kind();
    let holders: Vec<player::Id> = ctx
        .state()
        .seat_order()
        .into_iter()
        .filter(|p| ctx.state().kind(p) == kind)
        .collect();
    let mut chosen = vec![first];
    if kind == character::Kind::Minion {
        while chosen.len() < holders.len() {
            let rest: Vec<character::Ref> = evil
                .iter()
                .copied()
                .filter(|c| c.kind() == kind && !chosen.iter().any(|x| character::same(*x, *c)))
                .collect();
            match ctx.ask_character(me, "Which other Minion is in play?", &rest) {
                Some(c) => chosen.push(c),
                None => break,
            }
        }
    }
    ctx.sleep(me);
    ability::use_once(ctx, me, &Def);
    if ctx.state().is_malfunctioning(me) {
        return;
    }
    // Holders who already are a chosen character keep it; the others take the
    // remaining choices in seat order.
    let state = ctx.state();
    let (keep, change): (Vec<player::Id>, Vec<player::Id>) = holders
        .into_iter()
        .partition(|p| chosen.iter().any(|c| character::same(state.character(p), *c)));
    chosen.retain(|c| !keep.iter().any(|p| character::same(state.character(p), *c)));
    for (player, into) in change.into_iter().zip(chosen) {
        ctx.transform(&player, into);
        ctx.wake(&player);
        ctx.tell(&player, &info::Message::Character(into.name()));
        ctx.sleep(&player);
    }
}
