use crate::ability;
use crate::character;
use crate::ctx;
use crate::game;
use crate::player;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "courtier"
    }

    fn name(&self) -> &'static str {
        "Courtier"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Townsfolk
    }

    fn night_order(&self) -> character::NightOrder {
        character::NightOrder {
            first: Some(19),
            other: Some(8),
        }
    }

    fn night_action(&self) -> Option<character::NightAction> {
        Some(character::NightAction::ReadWrite(act))
    }

    fn wakes_tonight(&self, state: &game::State, me: &player::Id) -> bool {
        state.is_alive(me) && !ability::used(state, me, &Def)
    }
}

/// Once per game, chooses a character: they are drunk for 3 nights & 3 days
/// (until dusk after the third night). The ability is spent even if the
/// character is not in play or the Courtier is drunk; while the Courtier is
/// drunk or poisoned, their target is sober (`game::State::is_malfunctioning`).
fn act(ctx: &mut ctx::Write<'_>, me: &player::Id, night: u32) {
    let options = ctx.state().script().to_vec();
    ctx.wake(me);
    let chosen = ctx.ask_character(me, "Which character do you want to make drunk?", &options);
    ctx.sleep(me);
    let Some(chosen) = chosen else { return };
    ability::use_once(ctx, me, &Def);
    let state = ctx.state();
    let targets: Vec<player::Id> = state
        .seat_order()
        .into_iter()
        .filter(|p| character::same(state.character(p), chosen))
        .collect();
    let expiry = game::Expiry::Night(night + 3);
    ability::mark_all(ctx, me, &Def, &targets, game::Effect::Drunk, expiry);
}
