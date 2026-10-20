//! Game flow: setup, nights in script order, and day actions.

use crate::character;
use crate::ctx;
use crate::event;
use crate::game;
use crate::info;
use crate::player;
use crate::script;
use crate::storyteller;

/// Below this many players, the minions and demon do not learn each other.
pub const EVIL_INFO_MIN_PLAYERS: usize = 7;

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
            if ctx.state().players().len() >= EVIL_INFO_MIN_PLAYERS {
                minion_info(ctx);
                demon_info(ctx, script);
            }
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
    let not_in_play: Vec<&'static str> = script
        .characters
        .iter()
        .filter(|c| {
            c.kind().alignment() == character::Alignment::Good && ctx.state().find(c.id()).is_none()
        })
        .map(|c| c.name())
        .collect();
    let team = info::Message::Team {
        evil: ctx.state().evil_ids(),
    };
    let mut read = ctx.read();
    read.wake(&demon);
    read.tell(&demon, &team);
    let bluffs = read.choose(storyteller::Reason::DemonBluffs, not_in_play, 3, |name| {
        storyteller::Choice::Character(name)
    });
    read.tell(&demon, &info::Message::Bluffs(bluffs));
    read.sleep(&demon);
}

/// Runs night actions in order. Consecutive read-only actions are announced
/// to the table as one batch; read-write actions are sync points.
pub fn run_order(ctx: &mut ctx::Write<'_>, order: &[character::Ref], night: u32) {
    let mut steps: Vec<Step> = Vec::new();
    for c in order {
        let Some(action) = c.night_action(night) else {
            continue;
        };
        // Every holder acts (dead ones too: the action decides whether a dead
        // player wakes), so a demon that moved seats is not shadowed by the
        // dead original.
        let holders: Vec<player::Id> = ctx
            .state()
            .players()
            .iter()
            .filter(|p| p.character.id() == c.id())
            .map(|p| p.id.clone())
            .collect();
        for id in holders {
            match (action, steps.last_mut()) {
                (character::NightAction::ReadOnly(f), Some(Step::Reads(batch))) => {
                    batch.push((id, f))
                }
                (character::NightAction::ReadOnly(f), _) => steps.push(Step::Reads(vec![(id, f)])),
                (character::NightAction::ReadWrite(f), _) => steps.push(Step::Write(id, f)),
            }
        }
    }
    for step in steps {
        if ctx.state().winner().is_some() {
            return;
        }
        match step {
            Step::Write(id, f) => f(ctx, &id, night),
            Step::Reads(batch) => {
                let announce = batch.len() > 1;
                if announce {
                    ctx.io().batch_started(batch.len());
                }
                for (id, f) in batch {
                    f(&mut ctx.read(), &id, night);
                }
                if announce {
                    ctx.io().batch_ended();
                }
            }
        }
    }
}

enum Step {
    Write(player::Id, character::ReadWriteFn),
    Reads(Vec<(player::Id, character::ReadOnlyFn)>),
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
