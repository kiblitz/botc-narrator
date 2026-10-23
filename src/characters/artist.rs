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

/// Once per game, by day, privately asks the storyteller any yes/no question.
fn ask(ctx: &mut ctx::Write<'_>, me: &player::Id, _targets: &[player::Id]) {
    if !ctx.state().is_alive(me) || !ability::use_once(ctx, me, &Def) {
        return;
    }
    // Said nothing: the question is spent unasked.
    if let Some(question) = question(ctx, me) {
        ctx.answer(me, &question);
    }
}

/// Asks `me` to type their question.
fn question(ctx: &mut ctx::Write<'_>, me: &player::Id) -> Option<String> {
    ctx.ask_text(me, "Ask the storyteller any yes/no question")
}
