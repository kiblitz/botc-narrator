use crate::ability;
use crate::character;
use crate::ctx;
use crate::event;
use crate::game;
use crate::player;
use crate::setup;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "vigormortis"
    }

    fn name(&self) -> &'static str {
        "Vigormortis"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Demon
    }

    fn night_order(&self) -> character::NightOrder {
        character::NightOrder {
            first: None,
            other: Some(49),
        }
    }

    fn night_action(&self) -> Option<character::NightAction> {
        Some(character::NightAction::ReadWrite(act))
    }

    /// [-1 Outsider]
    fn setup_counts(&self, counts: setup::Counts) -> Vec<setup::Counts> {
        vec![counts.shift_outsiders(-1)]
    }

    /// Once the Vigormortis is dead its poison ends (kept abilities end by
    /// themselves: `game::Effect::KeepsAbility` needs a living placer).
    fn on_death(
        &self,
        ctx: &mut ctx::Write<'_>,
        me: &player::Id,
        dead: &player::Id,
        _cause: &event::Cause,
    ) {
        if me == dead {
            ability::sync_marks(ctx, me, &Def, game::Effect::Poisoned, POISON, &[]);
        }
    }
}

const POISON: game::Expiry = game::Expiry::Never;

/// Kills each night*; a Minion it kills keeps their ability and poisons one
/// of their Townsfolk neighbours (the storyteller picks which).
fn act(ctx: &mut ctx::Write<'_>, me: &player::Id, _night: u32) {
    let Some(target) = ability::demon_attack(ctx, me) else {
        return;
    };
    let state = ctx.state();
    if state.is_malfunctioning(me)
        || state.is_alive(&target)
        || state.kind(&target) != character::Kind::Minion
    {
        return;
    }
    let keeps = game::Reminder::new(&Def, me, game::Effect::KeepsAbility, game::Expiry::Never);
    ctx.add_reminder(&target, keeps);
    let neighbours = ability::nearest_of_kind(ctx.state(), &target, character::Kind::Townsfolk);
    let question = "which Townsfolk neighbour of the killed Minion is poisoned?";
    let picked = ability::storyteller_picks_player(&mut ctx.read(), me, &Def, question, neighbours);
    if let Some(neighbour) = picked {
        let mut poisoned = ability::marked(ctx.state(), me, &Def, game::Effect::Poisoned);
        poisoned.push(neighbour);
        ability::sync_marks(ctx, me, &Def, game::Effect::Poisoned, POISON, &poisoned);
    }
}
