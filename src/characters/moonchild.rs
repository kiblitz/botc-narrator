use crate::ability;
use crate::character;
use crate::ctx;
use crate::game;
use crate::player;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "moonchild"
    }

    fn name(&self) -> &'static str {
        "Moonchild"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Outsider
    }

    fn night_order(&self) -> character::NightOrder {
        character::NightOrder {
            first: None,
            other: Some(50),
        }
    }

    fn night_action(&self) -> Option<character::NightAction> {
        Some(character::NightAction::ReadWrite(act))
    }

    /// Acts (dead) on the night after choosing a good player.
    fn wakes_tonight(&self, state: &game::State, me: &player::Id) -> bool {
        target(state, me).is_some()
    }

    /// When the Moonchild learns they died, they publicly choose a living
    /// player; if that player is good, they are marked to die tonight.
    fn on_learns_death(&self, ctx: &mut ctx::Write<'_>, me: &player::Id) {
        let candidates = ctx.state().alive_except(me);
        let Some(chosen) = ctx.ask_one(me, "Choose a living player", &candidates) else {
            return;
        };
        if ctx.registered_alignment(me, &chosen) == character::Alignment::Good {
            ctx.place_reminder(&chosen, curse(me));
        }
    }
}

/// The token on the good player the Moonchild chose, who dies tonight.
const DIES: game::Effect = game::Effect::Custom("dies tonight");

fn curse(me: &player::Id) -> game::Reminder {
    game::Reminder::new(&Def, me, DIES, game::Expiry::Never)
}

/// The player `me` cursed, if any.
fn target(state: &game::State, me: &player::Id) -> Option<player::Id> {
    state
        .seat_order()
        .into_iter()
        .find(|p| ability::has_mark(state, me, &Def, p, DIES))
}

/// The cursed player dies, if the Moonchild is functioning tonight (whether
/// they were when choosing does not matter).
fn act(ctx: &mut ctx::Write<'_>, me: &player::Id, _night: u32) {
    let Some(target) = target(ctx.state(), me) else {
        return;
    };
    ctx.remove_reminder(&target, &curse(me));
    ability::kill(ctx, me, &target);
}
