use crate::ability;
use crate::character;
use crate::ctx;
use crate::game;
use crate::info;
use crate::player;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "barista"
    }

    fn name(&self) -> &'static str {
        "Barista"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Traveller
    }

    fn night_order(&self) -> character::NightOrder {
        character::NightOrder {
            first: Some(1),
            other: Some(1),
        }
    }

    fn night_action(&self) -> Option<character::NightAction> {
        Some(character::NightAction::ReadWrite(act))
    }
}

/// Each night the storyteller chooses a player: until dusk they are sober,
/// healthy and get true info, or their ability works twice. That player
/// learns which; the Barista does not wake. A malfunctioning Barista's effect
/// does not apply (the player is still told).
fn act(ctx: &mut ctx::Write<'_>, me: &player::Id, _night: u32) {
    let candidates = ctx.state().alive_ids();
    let mut read = ctx.read();
    let Some(target) =
        ability::storyteller_picks_player(&mut read, me, &Def, "who is affected", candidates)
    else {
        return;
    };
    let twice = ability::storyteller_agrees(
        &mut read,
        me,
        &Def,
        "does their ability work twice (instead of sober & healthy)",
    );
    let (effect, told) = if twice {
        (game::Effect::ActsTwice, "your ability works twice")
    } else {
        (game::Effect::Sober, "you are sober, healthy & get true info")
    };
    ability::mark(ctx, me, &Def, &target, effect, game::Expiry::Dusk);
    ctx.wake(&target);
    let message = info::Message::Selected {
        by: character::Def::name(&Def),
        effect: told,
    };
    ctx.tell(&target, &message);
    ctx.sleep(&target);
}
