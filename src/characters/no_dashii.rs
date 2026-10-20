use crate::ability;
use crate::character;
use crate::ctx;
use crate::event;
use crate::game;
use crate::player;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "no_dashii"
    }

    fn name(&self) -> &'static str {
        "No Dashii"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Demon
    }

    fn night_order(&self) -> character::NightOrder {
        character::NightOrder {
            first: None,
            other: Some(46),
        }
    }

    fn night_action(&self) -> Option<character::NightAction> {
        Some(character::NightAction::ReadWrite(act))
    }

    fn on_setup(&self, ctx: &mut ctx::Write<'_>, me: &player::Id) {
        poison_neighbours(ctx, me);
    }

    fn on_night_start(&self, ctx: &mut ctx::Write<'_>, me: &player::Id) {
        poison_neighbours(ctx, me);
    }

    fn on_day_start(&self, ctx: &mut ctx::Write<'_>, me: &player::Id) {
        poison_neighbours(ctx, me);
    }

    fn on_death(
        &self,
        ctx: &mut ctx::Write<'_>,
        me: &player::Id,
        dead: &player::Id,
        _cause: &event::Cause,
    ) {
        if me == dead {
            poison_neighbours(ctx, me);
        }
    }
}

/// Kills each night*.
fn act(ctx: &mut ctx::Write<'_>, me: &player::Id, _night: u32) {
    ability::demon_attack(ctx, me);
}

/// The nearest Townsfolk each way (alive or dead) are poisoned while the No
/// Dashii is alive and functioning; re-checked at setup, at each phase change
/// and when it dies.
fn poison_neighbours(ctx: &mut ctx::Write<'_>, me: &player::Id) {
    let state = ctx.state();
    let neighbours = if state.is_alive(me) && !state.is_malfunctioning(me) {
        ability::nearest_of_kind(state, me, character::Kind::Townsfolk)
    } else {
        Vec::new()
    };
    ability::sync_marks(
        ctx,
        me,
        &Def,
        game::Effect::Poisoned,
        game::Expiry::Never,
        &neighbours,
    );
}
