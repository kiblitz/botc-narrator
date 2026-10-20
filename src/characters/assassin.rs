use crate::ability;
use crate::character;
use crate::ctx;
use crate::event;
use crate::game;
use crate::player;
use crate::table;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "assassin"
    }

    fn name(&self) -> &'static str {
        "Assassin"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Minion
    }

    fn night_order(&self) -> character::NightOrder {
        character::NightOrder {
            first: None,
            other: Some(36),
        }
    }

    fn night_action(&self) -> Option<character::NightAction> {
        Some(character::NightAction::ReadWrite(act))
    }

    /// Until the ability is used.
    fn wakes_tonight(&self, state: &game::State, me: &player::Id) -> bool {
        state.is_alive(me) && !ability::used(state, me, &Def)
    }
}

/// Once per game, at night*, may choose a player: they die, even if for some
/// reason they could not. Malfunctioning, the ability is spent for nothing.
fn act(ctx: &mut ctx::Write<'_>, me: &player::Id, _night: u32) {
    ctx.wake(me);
    let target = if ctx.confirm(me, &table::Question::UseAbility(character::Def::name(&Def))) {
        let candidates = ctx.state().alive_ids();
        ctx.ask_one(me, "Who do you want to assassinate?", &candidates)
    } else {
        None
    };
    ctx.sleep(me);
    let Some(target) = target else { return };
    ability::use_once(ctx, me, &Def);
    if !ctx.state().is_malfunctioning(me) {
        ctx.kill_unstoppably(&target, event::Cause::Ability { by: me.clone() });
    }
}
