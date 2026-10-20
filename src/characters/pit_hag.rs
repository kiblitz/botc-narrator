use crate::ability;
use crate::character;
use crate::ctx;
use crate::event;
use crate::info;
use crate::player;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "pit_hag"
    }

    fn name(&self) -> &'static str {
        "Pit-Hag"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Minion
    }

    fn night_order(&self) -> character::NightOrder {
        character::NightOrder {
            first: None,
            other: Some(16),
        }
    }

    fn night_action(&self) -> Option<character::NightAction> {
        Some(character::NightAction::ReadWrite(act))
    }
}

/// Each night*, chooses a player and a character they become, if it is not in
/// play (their alignment stays). If a Demon is made, the storyteller may
/// kill a player of their choice tonight.
fn act(ctx: &mut ctx::Write<'_>, me: &player::Id, _night: u32) {
    let candidates = ctx.state().alive_ids();
    let characters = ctx.state().script().to_vec();
    ctx.wake(me);
    let target = ctx.ask_one(me, "Who do you want to change?", &candidates);
    let into = ctx.ask_character(me, "Into which character?", &characters);
    ctx.sleep(me);
    let (Some(target), Some(into)) = (target, into) else {
        return;
    };
    if ctx.state().is_malfunctioning(me) || ability::in_play(ctx.state(), into) {
        return;
    }
    ctx.transform(&target, into);
    // The only Demon may have stopped being one.
    ctx.check_win();
    if ctx.state().winner().is_some() {
        return;
    }
    ctx.wake(&target);
    ctx.tell(&target, &info::Message::Character(into.name()));
    ctx.sleep(&target);
    if into.kind() == character::Kind::Demon
        && ability::storyteller_agrees(&mut ctx.read(), me, &Def, "a Demon was made: kill someone?")
    {
        let living = ctx.state().alive_ids();
        let question = "who dies tonight?";
        if let Some(victim) =
            ability::storyteller_picks_player(&mut ctx.read(), me, &Def, question, living)
        {
            ctx.kill(&victim, event::Cause::Ability { by: me.clone() });
        }
    }
}
