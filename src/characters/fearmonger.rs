use crate::ability;
use crate::character;
use crate::ctx;
use crate::game;
use crate::player;

pub struct Def;

const FEAR: game::Effect = game::Effect::Custom("Fear");

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "fearmonger"
    }

    fn name(&self) -> &'static str {
        "Fearmonger"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Minion
    }

    fn night_order(&self) -> character::NightOrder {
        character::NightOrder {
            first: Some(26),
            other: Some(17),
        }
    }

    fn night_action(&self) -> Option<character::NightAction> {
        Some(character::NightAction::ReadWrite(act))
    }

    /// If the Fearmonger nominated and executes their Fear target, that
    /// player's team loses (even if the player survives the execution).
    fn on_execution(&self, ctx: &mut ctx::Write<'_>, me: &player::Id, executed: &player::Id) {
        let state = ctx.state();
        let nominated = ability::nominations_today(state)
            .iter()
            .any(|(nominator, nominee)| nominator.as_ref() == Some(me) && nominee == executed);
        if state.is_alive(me)
            && !state.is_malfunctioning(me)
            && nominated
            && ability::has_mark(state, me, &Def, executed, FEAR)
        {
            let loser = state.alignment(executed);
            ctx.win(loser.opposite());
        }
    }
}

/// Each night, chooses a player; everyone is told when it is a new player.
fn act(ctx: &mut ctx::Write<'_>, me: &player::Id, _night: u32) {
    let state = ctx.state();
    let candidates = state.seat_order();
    let current = candidates
        .iter()
        .find(|p| ability::has_mark(state, me, &Def, p, FEAR))
        .cloned();
    ctx.wake(me);
    let target = ctx.ask_one(me, "Who do you want to frighten?", &candidates);
    ctx.sleep(me);
    let Some(target) = target else { return };
    if ctx.state().is_malfunctioning(me) {
        return;
    }
    ability::mark(ctx, me, &Def, &target, FEAR, game::Expiry::Never);
    if current.as_ref() != Some(&target) {
        ctx.announce("The Fearmonger has chosen a new player");
    }
}
