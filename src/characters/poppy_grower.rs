use crate::ability;
use crate::character;
use crate::ctx;
use crate::event;
use crate::game;
use crate::narrator;
use crate::player;

pub struct Def;

/// Placed on a Poppy Grower who died sober: evil learn each other that night.
const DIED: game::Effect = game::Effect::Custom("Evil wakes");

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "poppy_grower"
    }

    fn name(&self) -> &'static str {
        "Poppy Grower"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Townsfolk
    }

    fn night_order(&self) -> character::NightOrder {
        character::NightOrder {
            first: Some(4),
            other: Some(3),
        }
    }

    fn night_action(&self) -> Option<character::NightAction> {
        Some(character::NightAction::ReadWrite(act))
    }

    /// The first night after they died sober.
    fn wakes_tonight(&self, state: &game::State, me: &player::Id) -> bool {
        ability::has_mark(state, me, &Def, me, DIED) && !ability::used(state, me, &Def)
    }

    /// Minions and Demons do not know each other (while sober and alive).
    fn hides_evil_info(&self) -> bool {
        true
    }

    fn on_death(
        &self,
        ctx: &mut ctx::Write<'_>,
        me: &player::Id,
        dead: &player::Id,
        _cause: &event::Cause,
    ) {
        if dead == me {
            ability::mark(ctx, me, &Def, me, DIED, game::Expiry::Never);
        }
    }
}

/// They died: the Minions and Demon learn each other tonight.
fn act(ctx: &mut ctx::Write<'_>, me: &player::Id, _night: u32) {
    ability::use_once(ctx, me, &Def);
    narrator::evil_info(ctx);
}
