use crate::ability;
use crate::character;
use crate::ctx;
use crate::game;
use crate::player;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "vortox"
    }

    fn name(&self) -> &'static str {
        "Vortox"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Demon
    }

    fn night_order(&self) -> character::NightOrder {
        character::NightOrder {
            first: None,
            other: Some(47),
        }
    }

    fn night_action(&self) -> Option<character::NightAction> {
        Some(character::NightAction::ReadWrite(act))
    }

    /// Townsfolk abilities yield false info (also to a player who only
    /// believes they are a Townsfolk).
    fn forces_false_info(
        &self,
        state: &game::State,
        _me: &player::Id,
        receiver: &player::Id,
    ) -> bool {
        state.believed(receiver).kind() == character::Kind::Townsfolk
    }

    /// Each day, if no-one is executed, evil wins.
    fn on_day_end(&self, ctx: &mut ctx::Write<'_>, me: &player::Id) {
        let state = ctx.state();
        if state.is_alive(me) && !state.is_malfunctioning(me) && state.executed_today().is_none()
        {
            ctx.win(character::Alignment::Evil);
        }
    }
}

/// Kills each night*.
fn act(ctx: &mut ctx::Write<'_>, me: &player::Id, _night: u32) {
    ability::demon_attack(ctx, me);
}
