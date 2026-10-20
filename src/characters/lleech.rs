use crate::ability;
use crate::character;
use crate::ctx;
use crate::event;
use crate::game;
use crate::player;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "lleech"
    }

    fn name(&self) -> &'static str {
        "Lleech"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Demon
    }

    fn night_order(&self) -> character::NightOrder {
        character::NightOrder {
            first: Some(31),
            other: Some(52),
        }
    }

    fn night_action(&self) -> Option<character::NightAction> {
        Some(character::NightAction::ReadWrite(act))
    }

    /// The Lleech cannot die while its host lives.
    fn intercept_death(
        &self,
        ctx: &mut ctx::Write<'_>,
        me: &player::Id,
        target: &player::Id,
        _cause: &event::Cause,
    ) -> character::Interception {
        let host_alive = host(ctx.state(), me).is_some_and(|h| ctx.state().is_alive(&h));
        if target == me && host_alive {
            character::Interception::Prevent
        } else {
            character::Interception::Pass
        }
    }

    /// When the host dies, so does the Lleech (unless it malfunctions).
    fn on_death(
        &self,
        ctx: &mut ctx::Write<'_>,
        me: &player::Id,
        dead: &player::Id,
        _cause: &event::Cause,
    ) {
        let state = ctx.state();
        if me != dead
            && state.is_alive(me)
            && !state.is_malfunctioning(me)
            && host(state, me).as_ref() == Some(dead)
        {
            ctx.kill(me, event::Cause::Ability { by: me.clone() });
        }
    }
}

/// The host, chosen on the first night; it never changes.
const HOST: game::Effect = game::Effect::Custom("Host");

fn host(state: &game::State, me: &player::Id) -> Option<player::Id> {
    ability::marked(state, me, &Def, HOST).into_iter().next()
}

/// First night: choose a host, who is poisoned. Other nights: kill.
fn act(ctx: &mut ctx::Write<'_>, me: &player::Id, _night: u32) {
    if host(ctx.state(), me).is_some() {
        ability::demon_attack(ctx, me);
        return;
    }
    let candidates = ctx.state().alive_except(me);
    ctx.wake(me);
    let chosen = ctx.ask_one(me, "Choose your host", &candidates);
    ctx.sleep(me);
    let Some(chosen) = chosen else { return };
    let token = game::Reminder::new(&Def, me, HOST, game::Expiry::Never);
    ctx.place_reminder(&chosen, token);
    let poison = game::Effect::Poisoned;
    ability::mark(ctx, me, &Def, &chosen, poison, game::Expiry::Never);
}
