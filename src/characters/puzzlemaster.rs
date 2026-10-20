use crate::ability;
use crate::character;
use crate::ctx;
use crate::game;
use crate::info;
use crate::player;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "puzzlemaster"
    }

    fn name(&self) -> &'static str {
        "Puzzlemaster"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Outsider
    }

    /// One player is drunk, even after the Puzzlemaster dies: usually a
    /// Townsfolk, so they are offered first.
    fn on_setup(&self, ctx: &mut ctx::Write<'_>, me: &player::Id) {
        let state = ctx.state();
        let (mut options, others): (Vec<player::Id>, Vec<player::Id>) = ability::others(state, me)
            .into_iter()
            .partition(|p| state.kind(p) == character::Kind::Townsfolk);
        options.extend(others);
        let question = "who is drunk";
        if let Some(target) =
            ability::storyteller_picks_player(&mut ctx.read(), me, &Def, question, options)
        {
            let reminder = game::Reminder::new(&Def, me, game::Effect::Drunk, game::Expiry::Never);
            ctx.place_reminder(&target, reminder);
        }
    }

    fn day_action(&self) -> Option<character::DayFn> {
        Some(guess)
    }
}

/// Once per game, guesses who the drunk player is: if right, learns the
/// Demon player; if wrong, learns a player who is not the Demon (false info).
/// Whether the guess was right is never said.
fn guess(ctx: &mut ctx::Write<'_>, me: &player::Id) {
    if !ctx.state().is_alive(me) || !ability::use_once(ctx, me, &Def) {
        return;
    }
    let candidates = ability::others(ctx.state(), me);
    let Some(guessed) = ctx.ask_one(me, "Who is the drunk player?", &candidates) else {
        return;
    };
    let others = ability::others(ctx.state(), me);
    let (demons, not_demons): (Vec<player::Id>, Vec<player::Id>) = others
        .into_iter()
        .partition(|p| ctx.registers_as(me, p, character::Kind::Demon).is_some());
    let as_message = |players: Vec<player::Id>| -> Vec<info::Message> {
        players
            .into_iter()
            .map(|p| info::Message::Players(vec![p]))
            .collect()
    };
    let right = ability::has_mark(ctx.state(), me, &Def, &guessed, game::Effect::Drunk);
    ctx.wake(me);
    if right {
        ctx.inform(me, as_message(demons), as_message(not_demons));
    } else if let Some(shown) = ability::storyteller_picks_player(
        &mut ctx.read(),
        me,
        &Def,
        "which non-Demon player to show (wrong guess)",
        not_demons,
    ) {
        ctx.tell(me, &info::Message::Players(vec![shown]));
    }
    ctx.sleep(me);
}
