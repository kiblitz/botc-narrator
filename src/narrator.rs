//! Game flow: setup, nights in script order, and day actions.

use crate::character;
use crate::ctx;
use crate::event;
use crate::game;
use crate::info;
use crate::player;
use crate::script;
use crate::storyteller;

/// Below this many players (not counting Travellers), the minions and demon
/// do not learn each other.
pub const EVIL_INFO_MIN_PLAYERS: usize = 7;

/// First-night positions of the minion info and the demon info, in the
/// numbering of `character::NightOrder`.
pub const MINION_INFO_ORDER: u32 = 6;
pub const DEMON_INFO_ORDER: u32 = 9;

/// Before the first night: every player's setup ability runs (the Drunk is
/// told who they believe they are), then the setup ability of each believed
/// character (a Drunk who believes they are the Fortune Teller gets a red
/// herring).
pub fn setup(ctx: &mut ctx::Write<'_>, script: &script::Script) {
    let players: Vec<(player::Id, character::Ref)> = ctx
        .state()
        .players()
        .iter()
        .map(|p| (p.id.clone(), p.character))
        .collect();
    for (id, character) in players {
        character.on_setup(ctx, &id, script);
    }
    let believers: Vec<(player::Id, character::Ref)> = ctx
        .state()
        .players()
        .iter()
        .filter_map(|p| p.believes.map(|c| (p.id.clone(), c)))
        .collect();
    for (id, character) in believers {
        character.on_setup(ctx, &id, script);
    }
}

pub fn run_night(ctx: &mut ctx::Write<'_>, script: &script::Script) {
    match ctx.state().phase() {
        game::Phase::Night(1) => {
            let position = |c: &character::Ref| c.night_order().first.unwrap_or(0);
            let (before_minions, rest): (Vec<character::Ref>, Vec<character::Ref>) = script
                .first_night
                .iter()
                .partition(|c| position(c) < MINION_INFO_ORDER);
            let (before_demon, after_demon): (Vec<character::Ref>, Vec<character::Ref>) =
                rest.iter().partition(|c| position(c) < DEMON_INFO_ORDER);
            let evil_info = ctx.state().resident_count() >= EVIL_INFO_MIN_PLAYERS;
            run_order(ctx, &before_minions, 1);
            if evil_info {
                minion_info(ctx);
            }
            run_order(ctx, &before_demon, 1);
            if evil_info {
                demon_info(ctx, script);
            }
            run_order(ctx, &after_demon, 1);
        }
        game::Phase::Night(night) => run_order(ctx, &script.other_nights, night),
        _ => {}
    }
}

/// The evil team as the minions and demon learn it: Travellers are not in it.
fn evil_team(state: &game::State) -> info::Message {
    info::Message::Team {
        evil: state
            .players()
            .iter()
            .filter(|p| {
                p.alignment == character::Alignment::Evil
                    && p.character.kind() != character::Kind::Traveller
            })
            .map(|p| p.id.clone())
            .collect(),
    }
}

/// Minions learn who is evil.
pub fn minion_info(ctx: &mut ctx::Write<'_>) {
    let minions = ctx.state().alive_of_kind(character::Kind::Minion);
    let team = evil_team(ctx.state());
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
            matches!(
                c.kind(),
                character::Kind::Townsfolk | character::Kind::Outsider
            ) && !in_play(ctx.state(), **c)
        })
        .map(|c| c.name())
        .collect();
    let team = evil_team(ctx.state());
    let mut read = ctx.read();
    read.wake(&demon);
    read.tell(&demon, &team);
    let bluffs = read.choose(storyteller::Reason::DemonBluffs, not_in_play, 3, |name| {
        storyteller::Choice::Character(name)
    });
    read.tell(&demon, &info::Message::Bluffs(bluffs));
    read.sleep(&demon);
}

/// Whether anyone is, or believes they are, this character (so the Drunk's
/// Townsfolk is never a demon bluff).
pub fn in_play(state: &game::State, c: character::Ref) -> bool {
    state
        .players()
        .iter()
        .any(|p| character::same(p.character, c) || character::same(p.believed(), c))
}

/// Runs night actions in order. Consecutive read-only actions are announced
/// to the table as one batch; read-write actions are sync points. Every
/// player with the ability acts (`game::State::abilities`) if it
/// `wakes_tonight` when their turn comes.
pub fn run_order(ctx: &mut ctx::Write<'_>, order: &[character::Ref], night: u32) {
    let mut steps: Vec<Step> = Vec::new();
    for c in order {
        let Some(action) = c.night_action() else {
            continue;
        };
        let holders: Vec<player::Id> = ctx
            .state()
            .players()
            .iter()
            .filter(|p| ctx.state().has_ability(&p.id, c.id()))
            .map(|p| p.id.clone())
            .collect();
        for id in holders {
            match (action, steps.last_mut()) {
                (character::NightAction::ReadOnly(f), Some(Step::Reads(batch))) => {
                    batch.push((*c, id, f))
                }
                (character::NightAction::ReadOnly(f), _) => {
                    steps.push(Step::Reads(vec![(*c, id, f)]))
                }
                (character::NightAction::ReadWrite(f), _) => steps.push(Step::Write(*c, id, f)),
            }
        }
    }
    for step in steps {
        if ctx.state().winner().is_some() {
            return;
        }
        match step {
            Step::Write(c, id, f) => {
                if c.wakes_tonight(ctx.state(), &id) {
                    f(ctx, &id, night);
                }
            }
            Step::Reads(batch) => {
                let batch: Vec<_> = batch
                    .into_iter()
                    .filter(|(c, id, _)| c.wakes_tonight(ctx.state(), id))
                    .collect();
                let announce = batch.len() > 1;
                if announce {
                    ctx.io().batch_started(batch.len());
                }
                for (_, id, f) in batch {
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
    Write(character::Ref, player::Id, character::ReadWriteFn),
    Reads(Vec<(character::Ref, player::Id, character::ReadOnlyFn)>),
}

/// The player uses their day ability (the Slayer's shot), if they have one.
pub fn day_action(ctx: &mut ctx::Write<'_>, player: &player::Id) {
    let action = ctx
        .state()
        .abilities(player)
        .into_iter()
        .find_map(|c| c.day_action());
    if let Some(f) = action {
        f(ctx, player);
    }
}

/// End of day: living players' day-end abilities (the Mayor) fire.
pub fn end_day(ctx: &mut ctx::Write<'_>) {
    for (id, ability) in ctx.living_abilities() {
        ability.on_day_end(ctx, &id);
    }
}

/// A nomination: the nominee's abilities react (the Virgin), then the
/// nominator's.
pub fn nominate(ctx: &mut ctx::Write<'_>, nominator: &player::Id, nominee: &player::Id) {
    ctx.record(event::Event::Nominated {
        nominee: nominee.clone(),
        nominator: nominator.clone(),
    });
    for (id, ability) in ctx.abilities_of(std::slice::from_ref(nominee)) {
        ability.on_nominated(ctx, &id, nominator);
    }
    for (id, ability) in ctx.abilities_of(std::slice::from_ref(nominator)) {
        ability.on_nominates(ctx, &id, nominee);
    }
}

/// A tallied vote: recorded, then every player's `on_vote` abilities react.
pub fn vote(
    ctx: &mut ctx::Write<'_>,
    nominee: &player::Id,
    voters: Vec<player::Id>,
    needed: usize,
) {
    ctx.record(event::Event::Voted {
        nominee: nominee.clone(),
        voters: voters.clone(),
        needed,
    });
    let everyone = ctx.state().seat_order();
    for (id, ability) in ctx.abilities_of(&everyone) {
        ability.on_vote(ctx, &id, nominee, &voters);
    }
}

/// Asks the table whether each living mad player broke their madness; for
/// each who did, the ability that made them mad reacts (it may have the
/// storyteller execute them).
pub fn check_madness(ctx: &mut ctx::Write<'_>) {
    let mad: Vec<(player::Id, game::Reminder, &'static str)> = ctx
        .state()
        .players()
        .iter()
        .filter(|p| p.alive)
        .flat_map(|p| {
            p.reminders.iter().filter_map(|r| match r.effect {
                game::Effect::Mad(about) => Some((p.id.clone(), r.clone(), about)),
                _ => None,
            })
        })
        .collect();
    for (player, reminder, about) in mad {
        if !ctx.state().is_alive(&player) || !ctx.io().broke_madness(&player, about) {
            continue;
        }
        ctx.record(event::Event::MadnessBroken {
            player: player.clone(),
            about,
        });
        let source = ctx
            .state()
            .abilities(&reminder.placed_by)
            .into_iter()
            .find(|c| c.id() == reminder.source);
        if let Some(source) = source {
            source.on_madness_broken(ctx, &reminder.placed_by, &player);
        }
    }
}
