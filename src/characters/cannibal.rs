use crate::character;
use crate::characters;
use crate::ctx;
use crate::event;
use crate::game;
use crate::player;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "cannibal"
    }

    fn name(&self) -> &'static str {
        "Cannibal"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Townsfolk
    }

    /// Has the ability of the last player to die by execution (replacing the
    /// previous one); a token on the Cannibal names it. After an evil one they
    /// are poisoned, until a good player dies by execution. A Demon's ability
    /// is not taken: executing the Demon ends the game.
    fn on_death(
        &self,
        ctx: &mut ctx::Write<'_>,
        me: &player::Id,
        dead: &player::Id,
        cause: &event::Cause,
    ) {
        let state = ctx.state();
        if *cause != event::Cause::Execution || dead == me || !state.is_alive(me) {
            return;
        }
        let eaten = state.character(dead);
        let evil = state.is_evil(dead);
        if let Some(previous) = meal(state, me) {
            ctx.remove_reminder(me, &previous);
            if let game::Effect::Custom(name) = previous.effect
                && let Some(old) = characters::all().iter().find(|c| c.name() == name)
            {
                ctx.lose_ability(me, *old);
            }
        }
        let poison = game::Reminder::new(&Def, me, game::Effect::Poisoned, game::Expiry::Never);
        if evil {
            ctx.place_reminder(me, poison);
        } else {
            ctx.remove_reminder(me, &poison);
        }
        if eaten.counts_as_demon() || character::same(eaten, &Def) {
            return;
        }
        ctx.gain_ability(me, eaten);
        let token = game::Effect::Custom(eaten.name());
        let meal = game::Reminder::new(&Def, me, token, game::Expiry::Never);
        ctx.place_reminder(me, meal);
    }
}

/// The token naming the ability the Cannibal has eaten, if any.
fn meal(state: &game::State, me: &player::Id) -> Option<game::Reminder> {
    state
        .player(me)
        .reminders
        .iter()
        .find(|r| {
            r.source == character::Def::id(&Def) && matches!(r.effect, game::Effect::Custom(_))
        })
        .cloned()
}
