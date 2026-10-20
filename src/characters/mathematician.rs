use crate::ability;
use crate::character;
use crate::ctx;
use crate::player;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "mathematician"
    }

    fn name(&self) -> &'static str {
        "Mathematician"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Townsfolk
    }

    fn night_order(&self) -> character::NightOrder {
        character::NightOrder {
            first: Some(77),
            other: Some(96),
        }
    }

    fn night_action(&self) -> Option<character::NightAction> {
        Some(character::NightAction::ReadOnly(act))
    }
}

/// Each night, learns how many players' abilities worked abnormally (since
/// dawn) due to another character's ability.
///
/// Approximation: the engine does not record which abilities misfired, so
/// any number up to how many other players are malfunctioning now is
/// truthful, and the storyteller (who knows) picks.
fn act(ctx: &mut ctx::Read<'_>, me: &player::Id, _night: u32) {
    let others = ability::others(ctx.state(), me);
    let malfunctioning = others
        .iter()
        .filter(|p| ctx.state().is_malfunctioning(p))
        .count();
    ctx.wake(me);
    ctx.inform(
        me,
        ability::counts(0..=malfunctioning, "abnormal"),
        ability::counts(0..=others.len(), "abnormal"),
    );
    ctx.sleep(me);
}
