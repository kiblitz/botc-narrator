use crate::ability;
use crate::character;
use crate::ctx;
use crate::player;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "spy"
    }

    fn name(&self) -> &'static str {
        "Spy"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Minion
    }

    fn night_order(&self) -> character::NightOrder {
        character::NightOrder {
            first: Some(49),
            other: Some(68),
        }
    }

    fn night_action(&self) -> Option<character::NightAction> {
        Some(character::NightAction::ReadOnly(act))
    }

    /// Might register as good and as a Townsfolk or Outsider, even if dead.
    fn misregistration(&self) -> Option<character::Misregistration> {
        Some(character::Misregistration {
            alignment: character::Alignment::Good,
            kinds: &[character::Kind::Townsfolk, character::Kind::Outsider],
        })
    }
}

/// Sees the grimoire.
fn act(ctx: &mut ctx::Read<'_>, me: &player::Id, _night: u32) {
    let grimoire = ability::grimoire(ctx.state());
    ctx.wake(me);
    ctx.inform(me, vec![grimoire], Vec::new());
    ctx.sleep(me);
}
