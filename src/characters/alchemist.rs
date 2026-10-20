use crate::ability;
use crate::character;
use crate::ctx;
use crate::info;
use crate::player;

pub struct Def;

/// Minions whose ability an Alchemist does not get (their jinxes).
const JINXED: [&str; 3] = ["spy", "marionette", "mastermind"];

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "alchemist"
    }

    fn name(&self) -> &'static str {
        "Alchemist"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Townsfolk
    }

    fn night_order(&self) -> character::NightOrder {
        character::NightOrder {
            first: Some(3),
            other: None,
        }
    }

    fn night_action(&self) -> Option<character::NightAction> {
        Some(character::NightAction::ReadOnly(act))
    }

    /// The storyteller gives them a not-in-play Minion's ability.
    fn on_setup(&self, ctx: &mut ctx::Write<'_>, me: &player::Id) {
        let options: Vec<character::Ref> =
            ability::not_in_play(ctx.state(), character::Kind::Minion)
                .into_iter()
                .filter(|c| !JINXED.contains(&c.id()))
                .collect();
        let question = "which not-in-play Minion ability they have";
        if let Some(minion) =
            ability::storyteller_picks_character(&mut ctx.read(), me, &Def, question, options)
        {
            ctx.gain_ability(me, minion);
        }
    }
}

/// Learns which Minion ability they have.
fn act(ctx: &mut ctx::Read<'_>, me: &player::Id, _night: u32) {
    let minion = ctx
        .state()
        .player(me)
        .gained
        .iter()
        .copied()
        .find(|c| c.kind() == character::Kind::Minion);
    let Some(minion) = minion else { return };
    ctx.wake(me);
    ctx.tell(me, &info::Message::Character(minion.name()));
    ctx.sleep(me);
}
