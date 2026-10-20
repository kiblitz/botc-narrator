use crate::ability;
use crate::character;
use crate::ctx;
use crate::player;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "bishop"
    }

    fn name(&self) -> &'static str {
        "Bishop"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Traveller
    }

    /// Only the storyteller nominates, and at least one player of the
    /// Bishop's opposing alignment each day: the storyteller picks which.
    fn storyteller_nominations(
        &self,
        ctx: &mut ctx::Read<'_>,
        me: &player::Id,
    ) -> Option<Vec<player::Id>> {
        let state = ctx.state();
        let opposing: Vec<player::Id> = state
            .alive_except(me)
            .into_iter()
            .filter(|p| state.alignment(p) != state.alignment(me))
            .collect();
        let question = "which opposing player to nominate";
        let nominee = ability::storyteller_picks_player(ctx, me, &Def, question, opposing);
        Some(nominee.into_iter().collect())
    }
}
