use crate::character;
use crate::ctx;
use crate::game;
use crate::player;
use crate::storyteller;

pub struct Def;

/// The first of the two storytellers is the liar (decided at setup).
pub const FIRST_LIES: game::Effect = game::Effect::Custom("first storyteller lies");
/// The storytellers have switched roles (once per game, at dusk).
pub const SWITCHED: game::Effect = game::Effect::Custom("switched");

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "knaves"
    }

    fn name(&self) -> &'static str {
        "Knaves"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Loric
    }

    /// The two storytellers decide which of them lies.
    fn fabled_setup(&self, ctx: &mut ctx::Write<'_>) {
        if decide(&mut ctx.read(), "does the first storyteller lie") {
            ctx.mark_fabled(&Def, FIRST_LIES);
        }
    }

    /// Once per game, at dusk, they might switch.
    fn fabled_dusk(&self, ctx: &mut ctx::Write<'_>) {
        if !ctx.state().fabled_has(&Def, SWITCHED)
            && decide(&mut ctx.read(), "do the storytellers switch")
        {
            ctx.mark_fabled(&Def, SWITCHED);
        }
    }

    /// The receiver chooses which storyteller gives them their information:
    /// asks them (`ctx.ask_yes_no`) each time.
    fn lies_to(&self, ctx: &mut ctx::Read<'_>, receiver: &player::Id) -> bool {
        let state = ctx.state();
        let first_lies = state.fabled_has(&Def, FIRST_LIES) != state.fabled_has(&Def, SWITCHED);
        let asks_first = ctx.ask_yes_no(receiver, "Do you ask the first storyteller?");
        asks_first == first_lies
    }
}

/// A yes/no decision for the storytellers; no by default.
fn decide(ctx: &mut ctx::Read<'_>, question: &'static str) -> bool {
    let reason = storyteller::Reason::Fabled {
        character: character::Def::name(&Def),
        question,
    };
    ctx.choose(reason, vec![false, true], 1, |yes| {
        storyteller::Choice::YesNo(*yes)
    })
    .first()
    .copied()
    .unwrap_or(false)
}
