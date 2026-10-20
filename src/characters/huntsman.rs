use crate::ability;
use crate::character;
use crate::ctx;
use crate::game;
use crate::info;
use crate::player;

/// The character the Huntsman hunts.
const DAMSEL: &str = "damsel";

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "huntsman"
    }

    fn name(&self) -> &'static str {
        "Huntsman"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Townsfolk
    }

    fn night_order(&self) -> character::NightOrder {
        character::NightOrder {
            first: Some(30),
            other: Some(45),
        }
    }

    fn night_action(&self) -> Option<character::NightAction> {
        Some(character::NightAction::ReadWrite(act))
    }

    fn wakes_tonight(&self, state: &game::State, me: &player::Id) -> bool {
        state.is_alive(me) && !ability::used(state, me, &Def)
    }

    /// [+the Damsel]
    fn requires(&self) -> &'static [&'static str] {
        &[DAMSEL]
    }
}

/// Once per game, chooses a living player: if they are the Damsel (even a
/// malfunctioning one), they become a not-in-play Townsfolk of the
/// storyteller's choice and are told so. Any choice uses the ability.
fn act(ctx: &mut ctx::Write<'_>, me: &player::Id, _night: u32) {
    let candidates = ctx.state().alive_except(me);
    ctx.wake(me);
    let target = ctx.ask_one(me, "Whom do you choose?", &candidates);
    ctx.sleep(me);
    let Some(target) = target else {
        return;
    };
    ability::use_once(ctx, me, &Def);
    let state = ctx.state();
    if state.is_malfunctioning(me) || state.character(&target).id() != DAMSEL {
        return;
    }
    let options = ability::not_in_play(state, character::Kind::Townsfolk);
    let question = "which Townsfolk the Damsel becomes";
    let Some(into) = ability::storyteller_picks_character(&mut ctx.read(), me, &Def, question, options)
    else {
        return;
    };
    ctx.transform(&target, into);
    ctx.wake(&target);
    ctx.tell(&target, &info::Message::Character(into.name()));
    ctx.sleep(&target);
}
