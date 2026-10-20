use crate::ability;
use crate::character;
use crate::ctx;
use crate::event;
use crate::game;
use crate::player;
use crate::setup;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "fang_gu"
    }

    fn name(&self) -> &'static str {
        "Fang Gu"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Demon
    }

    fn night_order(&self) -> character::NightOrder {
        character::NightOrder {
            first: None,
            other: Some(29),
        }
    }

    fn night_action(&self) -> Option<character::NightAction> {
        Some(character::NightAction::ReadWrite(act))
    }

    /// [+1 Outsider]
    fn setup_counts(&self, counts: setup::Counts) -> Vec<setup::Counts> {
        vec![counts.shift_outsiders(1)]
    }
}

/// Kills each night*; the first Outsider it kills becomes an evil Fang Gu
/// and the Fang Gu dies instead (once per game).
fn act(ctx: &mut ctx::Write<'_>, me: &player::Id, _night: u32) {
    let candidates = ctx.state().alive_ids();
    ctx.wake(me);
    let target = ctx.ask_one(me, "Who do you want to kill?", &candidates);
    ctx.sleep(me);
    let Some(target) = target else { return };
    if ctx.state().is_malfunctioning(me) {
        return;
    }
    let jumps = &target != me
        && !jumped(ctx.state())
        && !ctx.state().has_effect(&target, game::Effect::Protected)
        && ctx
            .registers_as(me, &target, character::Kind::Outsider)
            .is_some();
    if !jumps {
        ctx.kill(&target, event::Cause::Demon);
        return;
    }
    ability::use_once(ctx, me, &Def);
    ctx.transform(&target, &Def);
    ctx.set_alignment(&target, character::Alignment::Evil);
    ability::use_once(ctx, &target, &Def);
    ctx.kill(me, event::Cause::Ability { by: me.clone() });
}

/// Whether any Fang Gu has jumped this game.
fn jumped(state: &game::State) -> bool {
    state
        .players()
        .iter()
        .any(|p| ability::used(state, &p.id, &Def))
}
