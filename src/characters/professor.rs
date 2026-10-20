use crate::ability;
use crate::character;
use crate::ctx;
use crate::game;
use crate::player;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "professor"
    }

    fn name(&self) -> &'static str {
        "Professor"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Townsfolk
    }

    fn night_order(&self) -> character::NightOrder {
        character::NightOrder {
            first: None,
            other: Some(43),
        }
    }

    fn night_action(&self) -> Option<character::NightAction> {
        Some(character::NightAction::ReadWrite(act))
    }

    fn wakes_tonight(&self, state: &game::State, me: &player::Id) -> bool {
        state.is_alive(me) && !ability::used(state, me, &Def)
    }
}

/// Once per game, chooses a dead player: a Townsfolk is resurrected, with
/// their ability back (a used once-per-game ability too); anyone else, or a
/// malfunctioning Professor, wastes it. With nobody dead there is nothing
/// to choose.
fn act(ctx: &mut ctx::Write<'_>, me: &player::Id, _night: u32) {
    let state = ctx.state();
    let dead: Vec<player::Id> = state
        .seat_order()
        .into_iter()
        .filter(|p| !state.is_alive(p))
        .collect();
    ctx.wake(me);
    let target = ctx.ask_one(me, "Whom do you resurrect?", &dead);
    ctx.sleep(me);
    let Some(target) = target else {
        return;
    };
    ability::use_once(ctx, me, &Def);
    let state = ctx.state();
    if state.is_malfunctioning(me) || state.kind(&target) != character::Kind::Townsfolk {
        return;
    }
    ctx.revive(&target);
    let used: Vec<game::Reminder> = ctx
        .state()
        .player(&target)
        .reminders
        .iter()
        .filter(|r| r.placed_by == target && r.effect == game::Effect::AbilityUsed)
        .cloned()
        .collect();
    for reminder in used {
        ctx.remove_reminder(&target, &reminder);
    }
}
