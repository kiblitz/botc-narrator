use crate::character;
use crate::ctx;
use crate::game;
use crate::info;
use crate::player;
use crate::script;
use crate::storyteller;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "fortune_teller"
    }

    fn name(&self) -> &'static str {
        "Fortune Teller"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Townsfolk
    }

    fn night_order(&self) -> character::NightOrder {
        character::NightOrder {
            first: Some(38),
            other: Some(54),
        }
    }

    fn night_action(&self) -> Option<character::NightAction> {
        Some(character::NightAction::ReadOnly(act))
    }

    /// The storyteller picks a good player as the red herring.
    fn on_setup(&self, ctx: &mut ctx::Write<'_>, me: &player::Id, _script: &script::Script) {
        let state = ctx.state();
        let good: Vec<player::Id> = state
            .seat_order()
            .into_iter()
            .filter(|p| !state.is_evil(p))
            .collect();
        let reason = storyteller::Reason::RedHerring {
            fortune_teller: me.clone(),
        };
        if let Some(herring) = ctx.choose_player(reason, good) {
            let reminder = game::Reminder::new(&Def, me, RED_HERRING, game::Expiry::Never);
            ctx.place_reminder(&herring, reminder);
        }
    }
}

/// Registers as the demon to the Fortune Teller who placed it.
pub const RED_HERRING: game::Effect = game::Effect::Custom("red herring");

/// Chooses two players and learns whether either is the demon (or their red
/// herring).
fn act(ctx: &mut ctx::Read<'_>, me: &player::Id, _night: u32) {
    let candidates = ctx.state().seat_order();
    ctx.wake(me);
    let chosen = ctx.ask(me, "Choose two players", &candidates, 2);
    let mut yes = false;
    for p in &chosen {
        let herring = ctx
            .state()
            .player(p)
            .reminders
            .iter()
            .any(|r| r.effect == RED_HERRING && &r.placed_by == me);
        yes |= herring || ctx.registers_as(me, p, character::Kind::Demon).is_some();
    }
    let truthful = vec![info::Message::YesNo(yes)];
    let misinformation = vec![info::Message::YesNo(!yes)];
    ctx.inform(me, truthful, misinformation);
    ctx.sleep(me);
}
