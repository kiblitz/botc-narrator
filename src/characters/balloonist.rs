use crate::ability;
use crate::character;
use crate::ctx;
use crate::game;
use crate::info;
use crate::player;
use crate::setup;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "balloonist"
    }

    fn name(&self) -> &'static str {
        "Balloonist"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Townsfolk
    }

    fn night_order(&self) -> character::NightOrder {
        character::NightOrder {
            first: Some(66),
            other: Some(85),
        }
    }

    fn night_action(&self) -> Option<character::NightAction> {
        Some(character::NightAction::ReadWrite(act))
    }

    /// [+0 or +1 Outsider]
    fn setup_counts(&self, counts: setup::Counts) -> Vec<setup::Counts> {
        vec![counts, counts.shift_outsiders(1)]
    }
}

/// The Know token on the player shown, naming the type they were shown as,
/// so the next night's player can be of a different type.
fn know(kind: character::Kind) -> game::Effect {
    game::Effect::Custom(match kind {
        character::Kind::Townsfolk => "Know: Townsfolk",
        character::Kind::Outsider => "Know: Outsider",
        character::Kind::Minion => "Know: Minion",
        character::Kind::Demon => "Know: Demon",
        character::Kind::Traveller => "Know: Traveller",
        character::Kind::Fabled => "Know: Fabled",
        character::Kind::Loric => "Know: Loric",
    })
}

/// Each night, learns a player of a different character type (as they
/// register) than last night's. Malfunctioning, the type may repeat; either
/// way the next night differs from the player actually shown.
fn act(ctx: &mut ctx::Write<'_>, me: &player::Id, _night: u32) {
    let previous: Option<(player::Id, game::Reminder)> =
        ctx.state().players().iter().find_map(|p| {
            p.reminders
                .iter()
                .find(|r| &r.placed_by == me && r.source == character::Def::id(&Def))
                .map(|r| (p.id.clone(), r.clone()))
        });
    let last_kind = previous
        .as_ref()
        .and_then(|(_, r)| character::Kind::ALL.into_iter().find(|k| know(*k) == r.effect));
    let others = ability::others(ctx.state(), me);
    let mut read = ctx.read();
    let registered: Vec<(player::Id, character::Kind)> = others
        .iter()
        .map(|p| (p.clone(), read.registered_character(me, p).kind()))
        .collect();
    let (truthful, misinformation): (Vec<_>, Vec<_>) = registered
        .iter()
        .partition(|(_, kind)| Some(*kind) != last_kind);
    let message = |(p, _): &&(player::Id, character::Kind)| info::Message::Player(p.clone());
    read.wake(me);
    let shown = read.inform(
        me,
        truthful.iter().map(message).collect(),
        misinformation.iter().map(message).collect(),
    );
    read.sleep(me);
    let Some(info::Message::Player(shown)) = shown else {
        return;
    };
    let Some((_, kind)) = registered.iter().find(|(p, _)| *p == shown) else {
        return;
    };
    if let Some((player, reminder)) = &previous {
        ctx.remove_reminder(player, reminder);
    }
    let reminder = game::Reminder::new(&Def, me, know(*kind), game::Expiry::Never);
    ctx.place_reminder(&shown, reminder);
}
