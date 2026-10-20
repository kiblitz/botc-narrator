use crate::ability;
use crate::character;
use crate::ctx;
use crate::event;
use crate::game;
use crate::player;

pub struct Def;

/// The token on the players who can't die tonight.
const SAFE: game::Effect = game::Effect::Custom("Safe");

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "innkeeper"
    }

    fn name(&self) -> &'static str {
        "Innkeeper"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Townsfolk
    }

    fn night_order(&self) -> character::NightOrder {
        character::NightOrder {
            first: None,
            other: Some(9),
        }
    }

    fn night_action(&self) -> Option<character::NightAction> {
        Some(character::NightAction::ReadWrite(act))
    }

    /// The chosen players can't die tonight, from any cause. (If the
    /// Innkeeper is the one made drunk, the engine skips this: nobody is safe.)
    fn intercept_death(
        &self,
        ctx: &mut ctx::Write<'_>,
        me: &player::Id,
        target: &player::Id,
        _cause: &event::Cause,
    ) -> character::Interception {
        let state = ctx.state();
        if matches!(state.phase(), game::Phase::Night(_))
            && ability::has_mark(state, me, &Def, target, SAFE)
        {
            character::Interception::Prevent
        } else {
            character::Interception::Pass
        }
    }
}

/// Each night*, chooses 2 players: they can't die tonight, but the
/// storyteller makes 1 of them drunk until dusk.
fn act(ctx: &mut ctx::Write<'_>, me: &player::Id, _night: u32) {
    let candidates = ctx.state().alive_ids();
    ctx.wake(me);
    let chosen = ctx.ask(me, "Which 2 players do you want to protect?", &candidates, 2);
    ctx.sleep(me);
    if ctx.state().is_malfunctioning(me) {
        return;
    }
    ability::mark_all(ctx, me, &Def, &chosen, SAFE, game::Expiry::Dusk);
    let drunk =
        ability::storyteller_picks_player(&mut ctx.read(), me, &Def, "who is drunk", chosen);
    if let Some(drunk) = drunk {
        ability::mark(ctx, me, &Def, &drunk, game::Effect::Drunk, game::Expiry::Dusk);
    }
}
