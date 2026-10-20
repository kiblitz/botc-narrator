use crate::ability;
use crate::character;
use crate::ctx;
use crate::info;
use crate::player;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "knight"
    }

    fn name(&self) -> &'static str {
        "Knight"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Townsfolk
    }

    fn night_order(&self) -> character::NightOrder {
        character::NightOrder {
            first: Some(64),
            other: None,
        }
    }

    fn night_action(&self) -> Option<character::NightAction> {
        Some(character::NightAction::ReadOnly(act))
    }
}

/// Starts knowing 2 players that are not the Demon.
fn act(ctx: &mut ctx::Read<'_>, me: &player::Id, _night: u32) {
    let others = ability::others(ctx.state(), me);
    let demons: Vec<player::Id> = others
        .iter()
        .filter(|p| ctx.registers_as(me, p, character::Kind::Demon).is_some())
        .cloned()
        .collect();
    let message = |players: Vec<player::Id>| info::Message::Group {
        players,
        count: 0,
        noun: ability::noun(character::Kind::Demon),
    };
    let (truthful, misinformation): (Vec<_>, Vec<_>) = ability::groups(&others, 2)
        .into_iter()
        .partition(|pair| pair.iter().all(|p| !demons.contains(p)));
    ctx.wake(me);
    ctx.inform(
        me,
        truthful.into_iter().map(message).collect(),
        misinformation.into_iter().map(message).collect(),
    );
    ctx.sleep(me);
}
