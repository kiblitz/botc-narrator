use crate::ability;
use crate::character;
use crate::ctx;
use crate::player;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "gambler"
    }

    fn name(&self) -> &'static str {
        "Gambler"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Townsfolk
    }

    fn night_order(&self) -> character::NightOrder {
        character::NightOrder {
            first: None,
            other: Some(10),
        }
    }

    fn night_action(&self) -> Option<character::NightAction> {
        Some(character::NightAction::ReadWrite(act))
    }
}

/// Chooses any player (alive or dead, themself too) and guesses their
/// character: a wrong guess (against their real character) kills the
/// Gambler, unless malfunctioning. They learn nothing.
fn act(ctx: &mut ctx::Write<'_>, me: &player::Id, _night: u32) {
    let players = ctx.state().seat_order();
    let characters = ctx.state().script().to_vec();
    ctx.wake(me);
    let target = ctx.ask_one(me, "Whose character do you guess?", &players);
    let guess = ctx.ask_character(me, "Which character are they?", &characters);
    ctx.sleep(me);
    if let (Some(target), Some(guess)) = (target, guess)
        && !character::same(ctx.state().character(&target), guess)
    {
        ability::kill(ctx, me, me);
    }
}
