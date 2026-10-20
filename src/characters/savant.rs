use crate::ability;
use crate::character;
use crate::ctx;
use crate::info;
use crate::player;
use crate::storyteller;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "savant"
    }

    fn name(&self) -> &'static str {
        "Savant"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Townsfolk
    }

    fn day_action(&self) -> Option<character::DayFn> {
        Some(visit)
    }
}

/// A statement the Savant may hear, and its opposite (a true statement's
/// opposite is false, and the reverse).
struct Statement {
    message: info::Message,
    opposite: info::Message,
}

/// Each day, privately visits the storyteller to learn two statements, one
/// true and one false, in either order. Statements are about one other
/// player's alignment or character, as they register to the Savant. The
/// storyteller picks the true and the false one; when the Savant is
/// malfunctioning both may instead be true (the false one's opposite) or
/// both false (the true one's opposite).
fn visit(ctx: &mut ctx::Write<'_>, me: &player::Id) {
    if !ctx.state().is_alive(me) {
        return;
    }
    let (truths, lies) = statements(&mut ctx.read(), me);
    ctx.wake(me);
    let mut read = ctx.read();
    let truth = pick(&mut read, me, truths, true);
    let lie = pick(&mut read, me, lies, false);
    let (Some(truth), Some(lie)) = (truth, lie) else {
        read.sleep(me);
        return;
    };
    let pair = |a: &info::Message, b: &info::Message| {
        info::Message::Statements(vec![a.clone(), b.clone()])
    };
    let truthful = vec![
        pair(&truth.message, &lie.message),
        pair(&lie.message, &truth.message),
    ];
    let misinformation = vec![
        pair(&truth.message, &lie.opposite),
        pair(&truth.opposite, &lie.message),
    ];
    read.inform(me, truthful, misinformation);
    read.sleep(me);
}

/// The true and the false statements about the other players.
fn statements(ctx: &mut ctx::Read<'_>, me: &player::Id) -> (Vec<Statement>, Vec<Statement>) {
    let names = ctx.state().character_names();
    let mut truths = Vec::new();
    let mut lies = Vec::new();
    for player in ability::others(ctx.state(), me) {
        let alignment = ctx.registered_alignment(me, &player);
        let as_alignment = |alignment| info::Message::IsAligned {
            player: player.clone(),
            alignment,
        };
        truths.push(Statement {
            message: as_alignment(alignment),
            opposite: as_alignment(alignment.opposite()),
        });
        lies.push(Statement {
            message: as_alignment(alignment.opposite()),
            opposite: as_alignment(alignment),
        });
        let character = ctx.registered_character(me, &player).name();
        let is = |character| info::Message::Is {
            player: player.clone(),
            character,
        };
        let false_characters: Vec<&'static str> =
            names.iter().copied().filter(|c| *c != character).collect();
        for &other in &false_characters {
            lies.push(Statement {
                message: is(other),
                opposite: is(character),
            });
        }
        if let Some(&other) = false_characters.first() {
            truths.push(Statement {
                message: is(character),
                opposite: is(other),
            });
        }
    }
    (truths, lies)
}

/// The storyteller picks one of `statements` for `me`.
fn pick(
    ctx: &mut ctx::Read<'_>,
    me: &player::Id,
    statements: Vec<Statement>,
    truthful: bool,
) -> Option<Statement> {
    let question = if truthful {
        "the Savant's true statement"
    } else {
        "the Savant's false statement"
    };
    let reason = storyteller::Reason::Ability {
        player: me.clone(),
        character: character::Def::name(&Def),
        question,
    };
    let picks = ctx.choose(reason, (0..statements.len()).collect(), 1, |&i| {
        storyteller::Choice::Info {
            message: statements[i].message.clone(),
            truthful,
        }
    });
    let i = *picks.first()?;
    statements.into_iter().nth(i)
}
