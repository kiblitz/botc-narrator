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

    fn day_targets(&self) -> Option<character::Targets> {
        Some(character::Targets {
            prompt: "Whom is your question about?",
            count: 1,
            candidates: ability::others,
        })
    }

    fn claim(&self) -> Option<character::Claim> {
        Some(character::Claim {
            limit: character::ClaimLimit::OncePerGame,
            visibility: character::Visibility::Private,
            bluff: |ctx, me, _| {
                question(ctx, me);
            },
        })
    }
}

/// Once per game, by day, privately asks the storyteller a yes/no question.
/// Questions are modelled as "is this player this character?", answered
/// from the grimoire (false answers are legal when malfunctioning).
fn ask(ctx: &mut ctx::Write<'_>, me: &player::Id, targets: &[player::Id]) {
    if !ctx.state().is_alive(me) || !ability::use_once(ctx, me, &Def) {
        return;
    }
    let character = question(ctx, me);
    if let (Some(target), Some(character)) = (targets.first(), character) {
        let yes = character::same(ctx.state().character(target), character);
        ability::tell_yes_no(&mut ctx.read(), me, yes);
    }
}

/// Asks `me` which character their question is about.
fn question(ctx: &mut ctx::Write<'_>, me: &player::Id) -> Option<character::Ref> {
    let characters = ctx.state().script().to_vec();
    ctx.ask_character(me, "Are they which character?", &characters)
}
