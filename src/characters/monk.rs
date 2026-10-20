use crate::ability;
use crate::character;
use crate::ctx;
use crate::game;
use crate::player;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "monk"
    }

    fn name(&self) -> &'static str {
        "Monk"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Townsfolk
    }

    fn night_order(&self) -> character::NightOrder {
        character::NightOrder {
            first: None,
            other: Some(12),
        }
    }

    fn night_action(&self) -> Option<character::NightAction> {
        Some(character::NightAction::ReadWrite(act))
    }
}

/// Chooses a player (not themself) who is safe from the demon tonight.
fn act(ctx: &mut ctx::Write<'_>, me: &player::Id, _night: u32) {
    let candidates = ctx.state().alive_except(me);
    ctx.wake(me);
    if let Some(target) = ctx.ask_one(me, "Who do you want to protect?", &candidates) {
        let (effect, expiry) = (game::Effect::Protected, game::Expiry::Dusk);
        ability::mark(ctx, me, &Def, &target, effect, expiry);
    }
    ctx.sleep(me);
}
