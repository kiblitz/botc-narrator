use crate::ability;
use crate::character;
use crate::ctx;
use crate::event;
use crate::game;
use crate::player;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "sailor"
    }

    fn name(&self) -> &'static str {
        "Sailor"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Townsfolk
    }

    fn night_order(&self) -> character::NightOrder {
        character::NightOrder {
            first: Some(11),
            other: Some(4),
        }
    }

    fn night_action(&self) -> Option<character::NightAction> {
        Some(character::NightAction::ReadWrite(act))
    }

    /// Can't die (while sober).
    fn intercept_death(
        &self,
        _ctx: &mut ctx::Write<'_>,
        me: &player::Id,
        target: &player::Id,
        _cause: &event::Cause,
    ) -> character::Interception {
        if target == me {
            character::Interception::Prevent
        } else {
            character::Interception::Pass
        }
    }
}

/// Chooses an alive player: the storyteller makes either them or the Sailor
/// drunk until dusk (the Sailor, if they chose themself).
fn act(ctx: &mut ctx::Write<'_>, me: &player::Id, _night: u32) {
    let candidates = ctx.state().alive_ids();
    ctx.wake(me);
    let target = ctx.ask_one(me, "Who do you want to drink with?", &candidates);
    ctx.sleep(me);
    let Some(target) = target else { return };
    if ctx.state().is_malfunctioning(me) {
        return;
    }
    let options = if &target == me {
        vec![me.clone()]
    } else {
        vec![target, me.clone()]
    };
    let drunk =
        ability::storyteller_picks_player(&mut ctx.read(), me, &Def, "who is drunk", options);
    if let Some(drunk) = drunk {
        ability::mark(ctx, me, &Def, &drunk, game::Effect::Drunk, game::Expiry::Dusk);
    }
}
