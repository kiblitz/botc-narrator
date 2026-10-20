use crate::ability;
use crate::character;
use crate::ctx;
use crate::game;
use crate::player;

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
    fn on_setup(&self, ctx: &mut ctx::Write<'_>, me: &player::Id) {
        let state = ctx.state();
        let good: Vec<player::Id> = state
            .seat_order()
            .into_iter()
            .filter(|p| !state.is_evil(p))
            .collect();
        let question = "which good player is the red herring";
        if let Some(herring) =
            ability::storyteller_picks_player(&mut ctx.read(), me, &Def, question, good)
        {
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
        let herring = ability::has_mark(ctx.state(), me, &Def, p, RED_HERRING);
        yes |= herring || ctx.registers_as(me, p, character::Kind::Demon).is_some();
    }
    ability::tell_yes_no(ctx, me, yes);
    ctx.sleep(me);
}
