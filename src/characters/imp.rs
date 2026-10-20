use crate::ability;
use crate::character;
use crate::ctx;
use crate::event;
use crate::player;
use crate::storyteller;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "imp"
    }

    fn name(&self) -> &'static str {
        "Imp"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Demon
    }

    fn night_order(&self) -> character::NightOrder {
        character::NightOrder {
            first: None,
            other: Some(24),
        }
    }

    fn night_action(&self) -> Option<character::NightAction> {
        Some(character::NightAction::ReadWrite(act))
    }

    /// Starpass: if the Imp killed themself and no Scarlet Woman caught the
    /// demon, a minion of the storyteller's choice becomes the Imp.
    fn on_death(
        &self,
        ctx: &mut ctx::Write<'_>,
        me: &player::Id,
        dead: &player::Id,
        cause: &event::Cause,
    ) {
        if me != dead || *cause != event::Cause::Starpass {
            return;
        }
        let state = ctx.state();
        if !state.alive_of_kind(character::Kind::Demon).is_empty() {
            return;
        }
        let minions = state.alive_of_kind(character::Kind::Minion);
        if let Some(heir) = ctx.choose_player(storyteller::Reason::StarpassTarget, minions) {
            ctx.transform(&heir, &Def);
        }
    }
}

/// Kills a player each night after the first; killing themself passes the
/// demon to a minion.
fn act(ctx: &mut ctx::Write<'_>, me: &player::Id, _night: u32) {
    ability::demon_attack(ctx, me);
}
