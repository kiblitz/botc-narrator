use crate::ability;
use crate::character;
use crate::ctx;
use crate::player;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "gossip"
    }

    fn name(&self) -> &'static str {
        "Gossip"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Townsfolk
    }

    fn night_order(&self) -> character::NightOrder {
        character::NightOrder {
            first: None,
            other: Some(38),
        }
    }

    fn night_action(&self) -> Option<character::NightAction> {
        Some(character::NightAction::ReadWrite(act))
    }

    fn day_action(&self) -> Option<character::DayFn> {
        Some(gossip)
    }

    /// Anyone may make a public statement; only the Gossip's is judged.
    fn day_targets(&self) -> Option<character::Targets> {
        Some(character::Targets {
            prompt: "Who is your statement about?",
            count: 1,
            candidates: ability::others,
        })
    }

    fn claim(&self) -> Option<character::Claim> {
        Some(character::Claim {
            limit: character::ClaimLimit::EachDay,
            visibility: character::Visibility::Public {
                announcement: "gossips about",
                resolves_now: false,
            },
            bluff: gossip,
        })
    }
}

/// Each day, a public statement: "this player is this character". It is
/// only recorded (`ability::claim`) and judged tonight by the Gossip's own
/// night action, so a bluffer's statement does nothing.
fn gossip(ctx: &mut ctx::Write<'_>, me: &player::Id, targets: &[player::Id]) {
    if !ctx.state().is_alive(me) {
        return;
    }
    let characters = ctx.state().script().to_vec();
    let Some(target) = targets.first().cloned() else {
        return;
    };
    let Some(character) = ctx.ask_character(me, "Which character do you say they are?", &characters)
    else {
        return;
    };
    ability::claim(ctx, me, &Def, vec![(target, character)]);
}

/// Tonight, if today's statement was true, a player of the storyteller's
/// choice dies. Only the Gossip's health tonight matters.
fn act(ctx: &mut ctx::Write<'_>, me: &player::Id, _night: u32) {
    let state = ctx.state();
    let true_statement = ability::claims_today(state, me, &Def)
        .iter()
        .any(|(p, character)| state.character(p).name() == *character);
    if !true_statement || state.is_malfunctioning(me) {
        return;
    }
    let question = "who dies because the Gossip's statement was true";
    let options = state.alive_ids();
    if let Some(target) =
        ability::storyteller_picks_player(&mut ctx.read(), me, &Def, question, options)
    {
        ability::kill(ctx, me, &target);
    }
}
