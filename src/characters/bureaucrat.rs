use crate::ability;
use crate::character;
use crate::ctx;
use crate::game;
use crate::player;

pub struct Def;

pub const THREE_VOTES: game::Effect = game::Effect::Custom("3 votes");

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "bureaucrat"
    }

    fn name(&self) -> &'static str {
        "Bureaucrat"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Traveller
    }

    fn night_order(&self) -> character::NightOrder {
        character::NightOrder {
            first: Some(1),
            other: Some(1),
        }
    }

    fn night_action(&self) -> Option<character::NightAction> {
        Some(character::NightAction::ReadWrite(act))
    }

    /// The chosen player's vote counts three times. The token goes when the
    /// Bureaucrat dies, and does nothing while they malfunction.
    fn vote_weight(
        &self,
        state: &game::State,
        me: &player::Id,
        voter: &player::Id,
        weight: i32,
    ) -> i32 {
        if ability::has_mark(state, me, &Def, voter, THREE_VOTES) {
            weight * 3
        } else {
            weight
        }
    }
}

/// Each night, chooses a player (not themself): their vote counts as 3
/// tomorrow.
fn act(ctx: &mut ctx::Write<'_>, me: &player::Id, _night: u32) {
    let candidates = ability::others(ctx.state(), me);
    ctx.wake(me);
    let target = ctx.ask_one(me, "Whose vote counts as 3 tomorrow?", &candidates);
    ctx.sleep(me);
    if let Some(target) = target {
        ability::mark(ctx, me, &Def, &target, THREE_VOTES, game::Expiry::Dusk);
    }
}
