use crate::character;
use crate::ctx;
use crate::event;
use crate::game;
use crate::info;
use crate::player;

pub struct Def;

/// A vote token a dead player gave the Beggar (placed by the donor).
pub const TOKEN: game::Effect = game::Effect::Custom("vote token");

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "beggar"
    }

    fn name(&self) -> &'static str {
        "Beggar"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Traveller
    }

    /// "You are sober & healthy."
    fn always_sober(&self) -> bool {
        true
    }

    fn day_action(&self) -> Option<character::DayFn> {
        Some(beg)
    }

    /// A living Beggar's vote only counts with a vote token.
    fn vote_weight(
        &self,
        state: &game::State,
        me: &player::Id,
        voter: &player::Id,
        weight: i32,
    ) -> i32 {
        if voter == me && state.is_alive(me) && tokens(state, me).is_empty() {
            0
        } else {
            weight
        }
    }

    /// Voting spends a token.
    fn on_vote(
        &self,
        ctx: &mut ctx::Write<'_>,
        me: &player::Id,
        _nominee: &player::Id,
        voters: &[player::Id],
    ) {
        if voters.contains(me)
            && ctx.state().is_alive(me)
            && let Some(token) = tokens(ctx.state(), me).first()
        {
            ctx.remove_reminder(me, token);
        }
    }

    /// A dead Beggar loses every token they collected (they keep their own
    /// ghost vote).
    fn on_death(
        &self,
        ctx: &mut ctx::Write<'_>,
        me: &player::Id,
        dead: &player::Id,
        _cause: &event::Cause,
    ) {
        if dead == me {
            for token in tokens(ctx.state(), me) {
                ctx.remove_reminder(me, &token);
            }
        }
    }
}

/// The vote tokens the Beggar holds.
fn tokens(state: &game::State, me: &player::Id) -> Vec<game::Reminder> {
    state
        .player(me)
        .reminders
        .iter()
        .filter(|r| r.source == character::Def::id(&Def) && r.effect == TOKEN)
        .cloned()
        .collect()
}

/// A dead player with a vote token gives it to the (living) Beggar, who
/// learns their alignment (as it registers).
fn beg(ctx: &mut ctx::Write<'_>, me: &player::Id) {
    let state = ctx.state();
    if !state.is_alive(me) {
        return;
    }
    let candidates: Vec<player::Id> = state
        .players()
        .iter()
        .filter(|p| !p.alive && p.ghost_vote)
        .map(|p| p.id.clone())
        .collect();
    let prompt = "Which dead player gives you their vote token?";
    let Some(donor) = ctx.ask_one(me, prompt, &candidates) else {
        return;
    };
    ctx.spend_ghost_vote(&donor);
    let reminder = game::Reminder::new(&Def, &donor, TOKEN, game::Expiry::Never);
    ctx.place_reminder(me, reminder);
    let alignment = ctx.registered_alignment(me, &donor);
    ctx.inform(
        me,
        vec![info::Message::Alignment(alignment)],
        vec![info::Message::Alignment(alignment.opposite())],
    );
}
