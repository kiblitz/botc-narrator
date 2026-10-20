use crate::ability;
use crate::character;
use crate::ctx;
use crate::game;
use crate::info;
use crate::player;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "damsel"
    }

    fn name(&self) -> &'static str {
        "Damsel"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Outsider
    }

    fn night_order(&self) -> character::NightOrder {
        character::NightOrder {
            first: Some(31),
            other: Some(46),
        }
    }

    fn night_action(&self) -> Option<character::NightAction> {
        Some(character::NightAction::ReadWrite(act))
    }

    fn day_action(&self) -> Option<character::DayFn> {
        Some(guess)
    }

    /// Every living Minion may guess the Damsel, once in total.
    fn day_action_users(&self, state: &game::State, me: &player::Id) -> Vec<player::Id> {
        if ability::used(state, me, &Def) {
            return Vec::new();
        }
        state.alive_of_kind(character::Kind::Minion)
    }
}

/// A Minion who has been shown the Damsel token.
const KNOWS: game::Effect = game::Effect::Custom("knows Damsel");

/// Minions who do not know yet (on the first night, or when they became a
/// Minion) are shown that a Damsel is in play.
fn act(ctx: &mut ctx::Write<'_>, me: &player::Id, _night: u32) {
    for minion in ctx.state().alive_of_kind(character::Kind::Minion) {
        if ability::has_mark(ctx.state(), me, &Def, &minion, KNOWS) {
            continue;
        }
        ctx.wake(&minion);
        ctx.tell(&minion, &info::Message::Character(character::Def::name(&Def)));
        ctx.sleep(&minion);
        let reminder = game::Reminder::new(&Def, me, KNOWS, game::Expiry::Never);
        ctx.place_reminder(&minion, reminder);
    }
}

/// A Minion publicly guesses who the Damsel is (the Minions' one guess): if
/// right, and the Damsel is alive and functioning, the Damsel's team loses.
fn guess(ctx: &mut ctx::Write<'_>, minion: &player::Id) {
    let candidates = ctx.state().alive_except(minion);
    let Some(guessed) = ctx.ask_one(minion, "Who is the Damsel?", &candidates) else {
        return;
    };
    let damsels: Vec<player::Id> = ctx
        .state()
        .seat_order()
        .into_iter()
        .filter(|p| ctx.state().has_ability(p, character::Def::id(&Def)))
        .collect();
    for damsel in &damsels {
        ability::use_once(ctx, damsel, &Def);
    }
    let state = ctx.state();
    if damsels.contains(&guessed) && state.is_alive(&guessed) && !state.is_malfunctioning(&guessed)
    {
        let team = state.alignment(&guessed);
        ctx.win(team.opposite());
    }
}
