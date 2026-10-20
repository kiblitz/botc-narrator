use crate::ability;
use crate::character;
use crate::ctx;
use crate::game;
use crate::player;
use crate::table;

pub struct Def;

const CLAIMED: game::Effect = game::Effect::Custom("Claimed");

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "goblin"
    }

    fn name(&self) -> &'static str {
        "Goblin"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Minion
    }

    /// When nominated, may publicly claim to be the Goblin; the claim counts
    /// for that day only.
    fn on_nominated(&self, ctx: &mut ctx::Write<'_>, me: &player::Id, _nominator: &player::Id) {
        if ctx.confirm(me, &table::Question::Claim(character::Def::name(&Def))) {
            ctx.announce(format!("{me} claims to be the Goblin"));
            let claimed = game::Reminder::new(&Def, me, CLAIMED, game::Expiry::Dusk);
            ctx.place_reminder(me, claimed);
        }
    }

    /// Executed on the day they claimed: their team wins.
    fn on_executed(&self, ctx: &mut ctx::Write<'_>, me: &player::Id) {
        let state = ctx.state();
        if ability::has_mark(state, me, &Def, me, CLAIMED) && !state.is_malfunctioning(me) {
            let team = state.alignment(me);
            ctx.win(team);
        }
    }
}
