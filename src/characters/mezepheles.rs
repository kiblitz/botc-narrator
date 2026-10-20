use crate::ability;
use crate::character;
use crate::ctx;
use crate::game;
use crate::info;
use crate::player;
use crate::storyteller;
use crate::table;

pub struct Def;

/// Unusual words the storyteller picks the secret word from.
const WORDS: [&str; 6] = [
    "marmalade",
    "zeppelin",
    "harmonica",
    "walrus",
    "lantern",
    "turnip",
];

/// On the good player who said the word: they turn evil tonight.
const SAID: game::Effect = game::Effect::Custom("Turns evil");

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "mezepheles"
    }

    fn name(&self) -> &'static str {
        "Mezepheles"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Minion
    }

    fn night_order(&self) -> character::NightOrder {
        character::NightOrder {
            first: Some(27),
            other: Some(18),
        }
    }

    fn night_action(&self) -> Option<character::NightAction> {
        Some(character::NightAction::ReadWrite(act))
    }

    /// The first night to learn the word; later, the night after a good
    /// player said it (once).
    fn wakes_tonight(&self, state: &game::State, me: &player::Id) -> bool {
        state.is_alive(me)
            && (state.phase() == game::Phase::Night(1)
                || (!ability::used(state, me, &Def) && said_by(state, me).is_some()))
    }

    /// The storyteller picks the secret word (kept as a token on the Mezepheles).
    fn on_setup(&self, ctx: &mut ctx::Write<'_>, me: &player::Id) {
        let reason = ability::reason(me, &Def, "the secret word");
        let picked = ctx.choose(reason, WORDS.to_vec(), 1, |w| storyteller::Choice::Info {
            message: info::Message::Word(w),
            truthful: true,
        });
        if let Some(word) = picked.first() {
            let token = game::Reminder::new(&Def, me, game::Effect::Custom(word), game::Expiry::Never);
            ctx.place_reminder(me, token);
        }
    }

    /// The table judges whether a living good player said the word today;
    /// the first who did is marked.
    fn on_day_end(&self, ctx: &mut ctx::Write<'_>, me: &player::Id) {
        let state = ctx.state();
        let Some(word) = word(state, me) else { return };
        if ability::used(state, me, &Def) || said_by(state, me).is_some() {
            return;
        }
        let good: Vec<player::Id> = state
            .alive_ids()
            .into_iter()
            .filter(|p| !state.is_evil(p))
            .collect();
        for p in good {
            if ctx.confirm(&p, &table::Question::SaidWord(word)) {
                let reminder = game::Reminder::new(&Def, me, SAID, game::Expiry::Never);
                ctx.place_reminder(&p, reminder);
                return;
            }
        }
    }
}

/// Starts knowing a secret word; the first good player to say it becomes
/// evil that night (unless the Mezepheles is then malfunctioning, which
/// spends the ability).
fn act(ctx: &mut ctx::Write<'_>, me: &player::Id, night: u32) {
    if night == 1 {
        if let Some(word) = word(ctx.state(), me) {
            ctx.wake(me);
            ctx.tell(me, &info::Message::Word(word));
            ctx.sleep(me);
        }
        return;
    }
    let Some(target) = said_by(ctx.state(), me) else {
        return;
    };
    ability::use_once(ctx, me, &Def);
    if ctx.state().is_malfunctioning(me) || ctx.state().is_evil(&target) {
        return;
    }
    ctx.set_alignment(&target, character::Alignment::Evil);
    ctx.wake(&target);
    ctx.tell(&target, &info::Message::Alignment(character::Alignment::Evil));
    ctx.sleep(&target);
}

/// The Mezepheles's word: a `WORDS` token on themself.
fn word(state: &game::State, me: &player::Id) -> Option<&'static str> {
    state.player(me).reminders.iter().find_map(|r| match r.effect {
        game::Effect::Custom(w) if r.source == character::Def::id(&Def) && WORDS.contains(&w) => {
            Some(w)
        }
        _ => None,
    })
}

/// The good player marked for saying the word.
fn said_by(state: &game::State, me: &player::Id) -> Option<player::Id> {
    state
        .seat_order()
        .into_iter()
        .find(|p| ability::has_mark(state, me, &Def, p, SAID))
}
