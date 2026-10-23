use crate::ability;
use crate::character;
use crate::ctx;
use crate::game;
use crate::player;

/// The most guesses the Juggler may make.
const GUESSES: usize = 5;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "juggler"
    }

    fn name(&self) -> &'static str {
        "Juggler"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Townsfolk
    }

    fn night_order(&self) -> character::NightOrder {
        character::NightOrder {
            first: None,
            other: Some(61),
        }
    }

    fn night_action(&self) -> Option<character::NightAction> {
        Some(character::NightAction::ReadOnly(act))
    }

    /// The night after the day they guessed.
    fn wakes_tonight(&self, state: &game::State, me: &player::Id) -> bool {
        state.is_alive(me) && !ability::claims_today(state, me, &Def).is_empty()
    }

    fn day_action(&self) -> Option<character::DayFn> {
        Some(juggle)
    }

    /// Anyone may guess on the first day; only the Juggler learns anything.
    fn public_claim(&self) -> Option<character::PublicClaim> {
        Some(character::PublicClaim {
            limit: character::ClaimLimit::FirstDay,
            announcement: "juggles",
            resolves_now: false,
            bluff: |ctx, me| {
                let guesses = guesses(ctx, me);
                ability::claim(ctx, me, &Def, guesses);
            },
        })
    }
}

/// On the first day, publicly guesses up to five players' characters.
fn juggle(ctx: &mut ctx::Write<'_>, me: &player::Id) {
    if ctx.state().phase() != game::Phase::Day(1)
        || !ctx.state().is_alive(me)
        || !ability::use_once(ctx, me, &Def)
    {
        return;
    }
    let guesses = guesses(ctx, me);
    ability::claim(ctx, me, &Def, guesses);
}

/// Asks `me` for their guesses.
fn guesses(ctx: &mut ctx::Write<'_>, me: &player::Id) -> Vec<(player::Id, character::Ref)> {
    let others = ability::others(ctx.state(), me);
    let characters = ctx.state().script().to_vec();
    let players = ctx.ask(me, "Whose characters do you guess?", &others, GUESSES);
    let mut guesses = Vec::new();
    for player in players {
        let prompt = format!("Which character is {player}?");
        if let Some(character) = ctx.ask_character(me, &prompt, &characters) {
            guesses.push((player, character));
        }
    }
    guesses
}

/// Learns how many of today's guesses were correct (as the players register).
fn act(ctx: &mut ctx::Read<'_>, me: &player::Id, _night: u32) {
    let guesses = ability::claims_today(ctx.state(), me, &Def);
    let correct = guesses
        .iter()
        .filter(|(p, character)| ctx.registered_character(me, p).name() == *character)
        .count();
    ctx.wake(me);
    ability::tell_count(ctx, me, correct, guesses.len(), "correct");
    ctx.sleep(me);
}
