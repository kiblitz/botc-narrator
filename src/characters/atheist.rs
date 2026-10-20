use crate::ability;
use crate::character;
use crate::ctx;
use crate::player;
use crate::setup;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "atheist"
    }

    fn name(&self) -> &'static str {
        "Atheist"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Townsfolk
    }

    /// [No evil characters]: the Minion and Demon slots become Townsfolk.
    fn setup_counts(&self, counts: setup::Counts) -> Vec<setup::Counts> {
        let evil = counts.minions + counts.demons;
        vec![
            counts
                .with(character::Kind::Minion, 0)
                .with(character::Kind::Demon, 0)
                .with(character::Kind::Townsfolk, counts.townsfolk + evil),
        ]
    }

    /// There is no Demon to kill: good wins by executing the Storyteller.
    fn demonless(&self) -> bool {
        true
    }

    fn day_action(&self) -> Option<character::DayFn> {
        Some(execute_storyteller)
    }
}

/// The town nominates and executes the Storyteller, modelled as a day action
/// of whoever believes they are the Atheist: good wins if an Atheist is in
/// play, evil wins otherwise (a Drunk who believes they are the Atheist).
fn execute_storyteller(ctx: &mut ctx::Write<'_>, me: &player::Id) {
    if !ctx.state().is_alive(me) || !ability::use_once(ctx, me, &Def) {
        return;
    }
    ctx.note(format!("{me} has the Storyteller executed"));
    let winner = if ctx.state().demonless() {
        character::Alignment::Good
    } else {
        character::Alignment::Evil
    };
    ctx.win(winner);
}
