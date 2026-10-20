use crate::ability;
use crate::character;
use crate::ctx;
use crate::player;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "artist"
    }

    fn name(&self) -> &'static str {
        "Artist"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Townsfolk
    }

    fn day_action(&self) -> Option<character::DayFn> {
        Some(ask)
    }
}

/// Once per game, by day, privately asks the storyteller a yes/no question.
/// Questions are modelled as "is this player this character?", answered
/// from the grimoire (false answers are legal when malfunctioning).
fn ask(ctx: &mut ctx::Write<'_>, me: &player::Id) {
    if !ctx.state().is_alive(me) || !ability::use_once(ctx, me, &Def) {
        return;
    }
    let others = ability::others(ctx.state(), me);
    let characters = ctx.state().script().to_vec();
    ctx.wake(me);
    let target = ctx.ask_one(me, "Whom is your question about?", &others);
    let character = ctx.ask_character(me, "Are they which character?", &characters);
    if let (Some(target), Some(character)) = (target, character) {
        let yes = character::same(ctx.state().character(&target), character);
        ability::tell_yes_no(&mut ctx.read(), me, yes);
    }
    ctx.sleep(me);
}
