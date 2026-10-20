use crate::ability;
use crate::character;
use crate::ctx;
use crate::info;
use crate::player;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "marionette"
    }

    fn name(&self) -> &'static str {
        "Marionette"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Minion
    }

    fn night_order(&self) -> character::NightOrder {
        character::NightOrder {
            first: Some(12),
            other: None,
        }
    }

    fn night_action(&self) -> Option<character::NightAction> {
        Some(character::NightAction::ReadOnly(act))
    }

    fn seated_next_to_demon(&self) -> bool {
        true
    }

    /// Thinks they are a not-in-play Townsfolk of the storyteller's choice:
    /// they act as it (malfunctioning, like the Drunk) and do not learn the
    /// evil team.
    fn on_setup(&self, ctx: &mut ctx::Write<'_>, me: &player::Id) {
        let options = ability::not_in_play(ctx.state(), character::Kind::Townsfolk);
        let question = "which Townsfolk they believe they are";
        if let Some(believed) =
            ability::storyteller_picks_character(&mut ctx.read(), me, &Def, question, options)
        {
            ctx.set_believed(me, Some(believed));
        }
    }
}

/// The Demon knows who the Marionette is: they are woken and shown them.
fn act(ctx: &mut ctx::Read<'_>, me: &player::Id, _night: u32) {
    let shown = info::Message::Grimoire(vec![(me.clone(), character::Def::name(&Def))]);
    for demon in ctx.state().demons() {
        ctx.wake(&demon);
        ctx.tell(&demon, &shown);
        ctx.sleep(&demon);
    }
}
