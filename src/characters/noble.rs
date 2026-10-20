use crate::ability;
use crate::character;
use crate::ctx;
use crate::info;
use crate::player;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "noble"
    }

    fn name(&self) -> &'static str {
        "Noble"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Townsfolk
    }

    fn night_order(&self) -> character::NightOrder {
        character::NightOrder {
            first: Some(44),
            other: None,
        }
    }

    fn night_action(&self) -> Option<character::NightAction> {
        Some(character::NightAction::ReadOnly(act))
    }
}

/// Starts knowing 3 players, 1 and only 1 of which is evil.
fn act(ctx: &mut ctx::Read<'_>, me: &player::Id, _night: u32) {
    let others = ability::others(ctx.state(), me);
    let evil: Vec<player::Id> = others
        .iter()
        .filter(|p| ctx.registers_evil(me, p))
        .cloned()
        .collect();
    let message = |players: Vec<player::Id>| info::Message::Group {
        players,
        count: 1,
        noun: "evil",
    };
    let (truthful, misinformation): (Vec<_>, Vec<_>) = ability::groups(&others, 3)
        .into_iter()
        .partition(|group| group.iter().filter(|p| evil.contains(p)).count() == 1);
    ctx.wake(me);
    ctx.inform(
        me,
        truthful.into_iter().map(message).collect(),
        misinformation.into_iter().map(message).collect(),
    );
    ctx.sleep(me);
}
