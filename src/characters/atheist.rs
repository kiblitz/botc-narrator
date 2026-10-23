use crate::character;
use crate::ctx;
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

    /// The town may nominate and execute the Storyteller.
    fn storyteller_nominable(&self) -> bool {
        true
    }

    /// Good wins if an Atheist is in play; otherwise (a Drunk who believes
    /// they are the Atheist, or a bluff) evil wins.
    fn on_storyteller_executed(&self, ctx: &mut ctx::Write<'_>) {
        let winner = if ctx.state().demonless() {
            character::Alignment::Good
        } else {
            character::Alignment::Evil
        };
        ctx.win(winner);
    }
}
