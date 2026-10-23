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

/// What the Artist asks.
enum Question {
    /// Their own words, for a storyteller that reads them.
    Typed(String),
    /// "Is this player this character?", for one that does not.
    Is {
        target: player::Id,
        character: character::Ref,
    },
}

/// Once per game, by day, privately asks the storyteller any yes/no question.
fn ask(ctx: &mut ctx::Write<'_>, me: &player::Id, _targets: &[player::Id]) {
    if !ctx.state().is_alive(me) || !ability::use_once(ctx, me, &Def) {
        return;
    }
    match question(ctx, me) {
        Some(Question::Typed(question)) => {
            ctx.answer(me, &question);
        }
        Some(Question::Is { target, character }) => {
            let registered = ctx.registered_character(me, &target);
            ability::tell_yes_no(&mut ctx.read(), me, character::same(registered, character));
        }
        // Said nothing: the question is spent unasked.
        None => {}
    }
}

/// Asks `me` their question: typed, when the storyteller reads text (an LLM);
/// otherwise "is this player this character?", which the engine can judge.
fn question(ctx: &mut ctx::Write<'_>, me: &player::Id) -> Option<Question> {
    if ctx.storyteller_reads_text() {
        return ctx
            .ask_text(me, "Ask the storyteller any yes/no question")
            .map(Question::Typed);
    }
    let others = ability::others(ctx.state(), me);
    let target = ctx.ask_one(me, "Whom is your question about?", &others)?;
    let characters = ctx.state().script().to_vec();
    let character = ctx.ask_character(me, "Are they which character?", &characters)?;
    Some(Question::Is { target, character })
}
