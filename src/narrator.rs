//! Game flow: setup, nights in script order, and day actions.

use crate::character;
use crate::ctx;
use crate::event;
use crate::game;
use crate::info;
use crate::player;
use crate::script;
use crate::storyteller;

pub fn setup(ctx: &mut ctx::Write<'_>) {
    let players: Vec<(player::Id, character::Ref)> = ctx
        .state()
        .players()
        .iter()
        .map(|p| (p.id.clone(), p.character))
        .collect();
    for (id, character) in players {
        character.on_setup(ctx, &id);
    }
}

pub fn run_night(ctx: &mut ctx::Write<'_>, script: &script::Script) {
    match ctx.state().phase() {
        game::Phase::Night(1) => {
            minion_info(ctx);
            demon_info(ctx, script);
            run_order(ctx, &script.first_night, 1);
        }
        game::Phase::Night(night) => run_order(ctx, &script.other_nights, night),
        _ => {}
    }
}

/// Minions learn who is evil.
pub fn minion_info(ctx: &mut ctx::Write<'_>) {
    let minions = ctx.state().alive_of_kind(character::Kind::Minion);
    let team = info::Message::Team {
        evil: ctx.state().evil_ids(),
    };
    let mut read = ctx.read();
    for m in &minions {
        read.wake(m);
        read.tell(m, &team);
        read.sleep(m);
    }
}

/// The demon learns who is evil and three not-in-play good characters to bluff.
pub fn demon_info(ctx: &mut ctx::Write<'_>, script: &script::Script) {
    let Some(demon) = ctx
        .state()
        .alive_of_kind(character::Kind::Demon)
        .into_iter()
        .next()
    else {
        return;
    };
    let not_in_play: Vec<storyteller::Choice> = script
        .characters
        .iter()
        .filter(|c| {
            c.kind().alignment() == character::Alignment::Good && ctx.state().find(c.id()).is_none()
        })
        .map(|c| storyteller::Choice::Character(c.name()))
        .collect();
    let team = info::Message::Team {
        evil: ctx.state().evil_ids(),
    };
    let mut read = ctx.read();
    read.wake(&demon);
    read.tell(&demon, &team);
    let bluffs = read
        .decide(storyteller::Reason::DemonBluffs, not_in_play, 3)
        .into_iter()
        .filter_map(|c| match c {
            storyteller::Choice::Character(name) => Some(name),
            _ => None,
        })
        .collect();
    read.tell(&demon, &info::Message::Bluffs(bluffs));
    read.sleep(&demon);
}

/// Runs night actions in order. Consecutive read-only actions are announced as
/// one batch; read-write actions are sync points.
pub fn run_order(ctx: &mut ctx::Write<'_>, order: &[character::Ref], night: u32) {
    let actions: Vec<(player::Id, character::NightAction)> = order
        .iter()
        .filter_map(|c| {
            let id = ctx.state().find(c.id())?.clone();
            c.night_action(night).map(|action| (id, action))
        })
        .collect();
    let mut i = 0;
    while i < actions.len() && ctx.state().winner().is_none() {
        let end = actions[i..]
            .iter()
            .position(|(_, a)| matches!(a, character::NightAction::ReadWrite(_)))
            .map_or(actions.len(), |offset| i + offset);
        if end == i {
            if let (id, character::NightAction::ReadWrite(f)) = &actions[i] {
                f(ctx, id, night);
            }
            i += 1;
            continue;
        }
        let batch = &actions[i..end];
        if batch.len() > 1 {
            ctx.io().batch_started(batch.len());
        }
        for (id, action) in batch {
            if let character::NightAction::ReadOnly(f) = action {
                f(&mut ctx.read(), id, night);
            }
        }
        if batch.len() > 1 {
            ctx.io().batch_ended();
        }
        i = end;
    }
}

pub fn day_action(ctx: &mut ctx::Write<'_>, player: &player::Id) {
    if let Some(f) = ctx.state().character(player).day_action() {
        f(ctx, player);
    }
}

pub fn nominate(ctx: &mut ctx::Write<'_>, nominator: &player::Id, nominee: &player::Id) {
    ctx.record(event::Event::Nominated {
        nominee: nominee.clone(),
        nominator: nominator.clone(),
    });
    ctx.state()
        .character(nominee)
        .on_nominated(ctx, nominee, nominator);
}
