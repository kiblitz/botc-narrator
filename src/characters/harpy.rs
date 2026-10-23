use crate::ability;
use crate::character;
use crate::ctx;
use crate::game;
use crate::info;
use crate::player;

pub struct Def;

/// The first player is mad that the second (marked `SECOND`) is evil.
const MAD: game::Effect = game::Effect::Mad("2nd player being evil");
const SECOND: game::Effect = game::Effect::Custom("2nd");

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "harpy"
    }

    fn name(&self) -> &'static str {
        "Harpy"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Minion
    }

    /// Officially 26.5 / 17.5, between the Fearmonger and the Mezepheles.
    fn night_order(&self) -> character::NightOrder {
        character::NightOrder {
            first: Some(26),
            other: Some(17),
        }
    }

    fn night_action(&self) -> Option<character::NightAction> {
        Some(character::NightAction::ReadWrite(act))
    }

    /// The mad player broke madness: the storyteller may kill either or both.
    fn causes_madness(&self) -> bool {
        true
    }

    fn on_madness_broken(&self, ctx: &mut ctx::Write<'_>, me: &player::Id, mad: &player::Id) {
        let state = ctx.state();
        let second = state
            .seat_order()
            .into_iter()
            .find(|p| state.is_alive(p) && ability::has_mark(state, me, &Def, p, SECOND));
        let kill_mad = ability::storyteller_agrees(&mut ctx.read(), me, &Def, "kill the mad player?");
        let kill_second = second.is_some()
            && ability::storyteller_agrees(&mut ctx.read(), me, &Def, "kill the 2nd player?");
        if kill_mad {
            ability::kill(ctx, me, mad);
        }
        if let Some(second) = second.filter(|_| kill_second) {
            ability::kill(ctx, me, &second);
        }
    }
}

/// Each night, chooses 2 players: tomorrow, the 1st is mad that the 2nd is
/// evil, or one or both might die. The 1st is woken and shown the 2nd.
fn act(ctx: &mut ctx::Write<'_>, me: &player::Id, _night: u32) {
    let candidates = ctx.state().seat_order();
    ctx.wake(me);
    let first = ctx.ask_one(me, "Who do you want to make mad?", &candidates);
    let second = ctx.ask_one(me, "Mad that which player is evil?", &candidates);
    ctx.sleep(me);
    let (Some(first), Some(second)) = (first, second) else {
        return;
    };
    ability::mark(ctx, me, &Def, &first, MAD, game::Expiry::Dusk);
    ability::mark(ctx, me, &Def, &second, SECOND, game::Expiry::Dusk);
    ctx.wake(&first);
    ctx.tell(&first, &info::Message::Team { evil: vec![second] });
    ctx.sleep(&first);
}
