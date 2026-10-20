use crate::ability;
use crate::character;
use crate::ctx;
use crate::game;
use crate::info;
use crate::player;

pub struct Def;

/// Marks the Exorcist's latest choice, which they can't choose the next night.
const CHOSEN: game::Effect = game::Effect::Custom("Chosen");

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "exorcist"
    }

    fn name(&self) -> &'static str {
        "Exorcist"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Townsfolk
    }

    fn night_order(&self) -> character::NightOrder {
        character::NightOrder {
            first: None,
            other: Some(21),
        }
    }

    fn night_action(&self) -> Option<character::NightAction> {
        Some(character::NightAction::ReadWrite(act))
    }
}

/// Each night*, chooses a player (not last night's choice): if it is the
/// Demon, they learn who the Exorcist is and don't wake tonight. Drunk or
/// poisoned, the choice is still made (and counts as last night's), to no
/// effect.
fn act(ctx: &mut ctx::Write<'_>, me: &player::Id, _night: u32) {
    let state = ctx.state();
    let last = ability::marked(state, me, &Def, CHOSEN);
    let candidates: Vec<player::Id> = state
        .alive_ids()
        .into_iter()
        .filter(|p| !last.contains(p))
        .collect();
    ctx.wake(me);
    let target = ctx.ask_one(me, "Who do you want to exorcise?", &candidates);
    ctx.sleep(me);
    let Some(target) = target else { return };
    // Placed even when malfunctioning: the choice still happened.
    let chosen = game::Reminder::new(&Def, me, CHOSEN, game::Expiry::Death);
    ctx.place_reminder(&target, chosen);
    if ctx.state().is_malfunctioning(me) || !ctx.state().demons().contains(&target) {
        return;
    }
    let (effect, expiry) = (game::Effect::DoesNotWake, game::Expiry::Dusk);
    ability::mark(ctx, me, &Def, &target, effect, expiry);
    ctx.wake(&target);
    let exorcist = info::Message::Grimoire(vec![(me.clone(), character::Def::name(&Def))]);
    ctx.tell(&target, &exorcist);
    ctx.sleep(&target);
}
