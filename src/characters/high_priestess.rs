use crate::character;
use crate::ctx;
use crate::info;
use crate::player;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "high_priestess"
    }

    fn name(&self) -> &'static str {
        "High Priestess"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Townsfolk
    }

    /// After the Spy (49 / 68) on both nights; the clocktower.online numbering
    /// has no High Priestess, and nothing at 50 / 69 shares a script with her.
    fn night_order(&self) -> character::NightOrder {
        character::NightOrder {
            first: Some(50),
            other: Some(69),
        }
    }

    fn night_action(&self) -> Option<character::NightAction> {
        Some(character::NightAction::ReadOnly(act))
    }
}

/// Learns which player the storyteller believes they should talk to most:
/// any player, alive or dead, is a legal answer whether or not the High
/// Priestess is malfunctioning.
fn act(ctx: &mut ctx::Read<'_>, me: &player::Id, _night: u32) {
    let players = ctx
        .state()
        .seat_order()
        .into_iter()
        .filter(|p| p != me)
        .map(info::Message::Player)
        .collect();
    ctx.wake(me);
    ctx.inform(me, players, Vec::new());
    ctx.sleep(me);
}
