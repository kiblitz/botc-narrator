use crate::ability;
use crate::character;
use crate::ctx;
use crate::game;
use crate::player;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "seamstress"
    }

    fn name(&self) -> &'static str {
        "Seamstress"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Townsfolk
    }

    fn night_order(&self) -> character::NightOrder {
        character::NightOrder {
            first: Some(62),
            other: Some(83),
        }
    }

    fn night_action(&self) -> Option<character::NightAction> {
        Some(character::NightAction::ReadWrite(act))
    }

    /// Wakes each night until the ability is used (including that night).
    fn wakes_tonight(&self, state: &game::State, me: &player::Id) -> bool {
        state.is_alive(me)
            && (!ability::used(state, me, &Def) || ability::used_tonight(state, me, &Def))
    }
}

/// Once per game, at night, chooses 2 players (not themself): learns if they
/// are the same alignment. Pointing at themself passes.
fn act(ctx: &mut ctx::Write<'_>, me: &player::Id, _night: u32) {
    if ability::used(ctx.state(), me, &Def) {
        return;
    }
    let candidates = ctx.state().seat_order();
    ctx.wake(me);
    let chosen = ctx.ask(
        me,
        "Choose 2 players (or yourself, to pass):",
        &candidates,
        2,
    );
    if chosen.len() == 2 && !chosen.contains(me) {
        ability::use_once(ctx, me, &Def);
        let mut read = ctx.read();
        let a = read.registered_alignment(me, &chosen[0]);
        let b = read.registered_alignment(me, &chosen[1]);
        ability::tell_yes_no(&mut read, me, a == b);
    }
    ctx.sleep(me);
}
