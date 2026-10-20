use crate::ability;
use crate::character;
use crate::ctx;
use crate::game;
use crate::info;
use crate::player;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "farmer"
    }

    fn name(&self) -> &'static str {
        "Farmer"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Townsfolk
    }

    fn night_order(&self) -> character::NightOrder {
        character::NightOrder {
            first: None,
            other: Some(48),
        }
    }

    fn night_action(&self) -> Option<character::NightAction> {
        Some(character::NightAction::ReadWrite(act))
    }

    /// The night they die (a day death does nothing).
    fn wakes_tonight(&self, state: &game::State, me: &player::Id) -> bool {
        !state.is_alive(me) && state.deaths_tonight().contains(me)
    }
}

/// Died tonight: the storyteller makes an alive good player a Farmer, who is
/// woken and told. Nothing happens if the Farmer was drunk or poisoned.
fn act(ctx: &mut ctx::Write<'_>, me: &player::Id, _night: u32) {
    let state = ctx.state();
    if state.is_malfunctioning(me) {
        return;
    }
    let good: Vec<player::Id> = state
        .alive_ids()
        .into_iter()
        .filter(|p| !state.is_evil(p))
        .collect();
    let question = "which alive good player becomes a Farmer";
    let Some(heir) = ability::storyteller_picks_player(&mut ctx.read(), me, &Def, question, good)
    else {
        return;
    };
    ctx.transform(&heir, &Def);
    ctx.wake(&heir);
    ctx.tell(&heir, &info::Message::Character(character::Def::name(&Def)));
    ctx.sleep(&heir);
}
