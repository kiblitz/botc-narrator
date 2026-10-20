use crate::ability;
use crate::character;
use crate::ctx;
use crate::event;
use crate::game;
use crate::player;

pub struct Def;

/// Who was chosen last night, kept even when malfunctioning, since the next
/// choice must differ.
const CHOSEN: game::Effect = game::Effect::Custom("Chosen");
const SURVIVES: game::Effect = game::Effect::Custom("Survives execution");

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "devils_advocate"
    }

    fn name(&self) -> &'static str {
        "Devil's Advocate"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Minion
    }

    fn night_order(&self) -> character::NightOrder {
        character::NightOrder {
            first: Some(22),
            other: Some(13),
        }
    }

    fn night_action(&self) -> Option<character::NightAction> {
        Some(character::NightAction::ReadWrite(act))
    }

    /// The chosen player does not die if executed tomorrow.
    fn intercept_death(
        &self,
        ctx: &mut ctx::Write<'_>,
        me: &player::Id,
        target: &player::Id,
        cause: &event::Cause,
    ) -> character::Interception {
        if *cause == event::Cause::Execution
            && ability::has_mark(ctx.state(), me, &Def, target, SURVIVES)
        {
            character::Interception::Prevent
        } else {
            character::Interception::Pass
        }
    }
}

/// Each night, chooses a living player (different to last night): if
/// executed tomorrow, they don't die.
fn act(ctx: &mut ctx::Write<'_>, me: &player::Id, _night: u32) {
    let state = ctx.state();
    let candidates: Vec<player::Id> = state
        .alive_ids()
        .into_iter()
        .filter(|p| !ability::has_mark(state, me, &Def, p, CHOSEN))
        .collect();
    ctx.wake(me);
    let target = ctx.ask_one(me, "Who do you want to save from execution?", &candidates);
    ctx.sleep(me);
    let Some(target) = target else { return };
    let chosen = game::Reminder::new(&Def, me, CHOSEN, game::Expiry::Never);
    ctx.place_reminder(&target, chosen);
    ability::mark(ctx, me, &Def, &target, SURVIVES, game::Expiry::Dusk);
}
