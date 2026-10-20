//! Game flow: setup, nights in script order, and day actions.

use crate::ability;
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

/// Before the first night: the script is recorded in the grimoire
/// (`game::State::script`), then every player's setup ability runs (the Drunk is
/// told who they believe they are), then the setup ability of each believed
/// character (a Drunk who believes they are the Fortune Teller gets a red
/// herring). Last, the script's Fabled and Loric are put in play and do their
/// storyteller-side setup.
pub fn setup(ctx: &mut ctx::Write<'_>, script: &script::Script) {
    ctx.set_script(script.characters.clone());
    let players: Vec<(player::Id, character::Ref)> = ctx
        .state()
        .players()
        .iter()
        .map(|p| (p.id.clone(), p.character))
        .collect();
    for (id, character) in players {
        character.on_setup(ctx, &id);
    }
    let believers: Vec<(player::Id, character::Ref)> = ctx
        .state()
        .players()
        .iter()
        .filter_map(|p| p.believes.map(|c| (p.id.clone(), c)))
        .collect();
    for (id, character) in believers {
        character.on_setup(ctx, &id);
    }
    let fabled: Vec<character::Ref> = script
        .characters
        .iter()
        .copied()
        .filter(|c| c.kind().storyteller_side())
        .collect();
    for f in &fabled {
        ctx.add_fabled(*f);
    }
    for f in fabled {
        f.fabled_setup(ctx);
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
            let team_shown = evil_info && !evil_info_hidden(ctx.state());
            if team_shown {
                minion_info(ctx);
            }
            run_order(ctx, &before_demon, 1);
            if evil_info {
                demon_info(ctx, script, team_shown);
            }
            run_order(ctx, &after_demon, 1);
        }
        game::Phase::Night(night) => run_order(ctx, &script.other_nights, night),
        _ => {}
    }
}

/// The evil team as the minions and demon learn it: Travellers are not in
/// it, and players whose ability joins it (the Magician) are.
fn evil_team(state: &game::State) -> info::Message {
    info::Message::Team {
        evil: state
            .players()
            .iter()
            .filter(|p| {
                let evil = p.alignment == character::Alignment::Evil
                    && p.character.kind() != character::Kind::Traveller;
                let joins = !state.is_malfunctioning(&p.id)
                    && state.abilities(&p.id).iter().any(|c| c.joins_evil_info());
                evil || joins
            })
            .map(|p| p.id.clone())
            .collect(),
    }
}

/// Whether a living, functioning player's ability keeps the Minions and
/// Demon from learning each other (`character::Def::hides_evil_info`).
fn evil_info_hidden(state: &game::State) -> bool {
    state.alive_ids().iter().any(|id| {
        !state.is_malfunctioning(id) && state.abilities(id).iter().any(|c| c.hides_evil_info())
    })
}

/// The Minions and Demon learn each other now, as on the first night (after
/// the Poppy Grower dies): only with enough players, and not while another
/// ability still hides it.
pub fn evil_info(ctx: &mut ctx::Write<'_>) {
    let state = ctx.state();
    if state.resident_count() < EVIL_INFO_MIN_PLAYERS || evil_info_hidden(state) {
        return;
    }
    minion_info(ctx);
    let demons = ctx.state().demons();
    let team = evil_team(ctx.state());
    let mut read = ctx.read();
    for demon in &demons {
        read.wake(demon);
        read.tell(demon, &team);
        read.sleep(demon);
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

/// The demon learns who is evil (if `team_shown`) and three not-in-play good
/// characters to bluff.
pub fn demon_info(ctx: &mut ctx::Write<'_>, script: &script::Script, team_shown: bool) {
    let demons = ctx.state().demons();
    if demons.is_empty() {
        return;
    }
    let not_in_play: Vec<&'static str> = script
        .characters
        .iter()
        .filter(|c| {
            matches!(
                c.kind(),
                character::Kind::Townsfolk | character::Kind::Outsider
            ) && !ability::in_play(ctx.state(), **c)
        })
        .map(|c| c.name())
        .collect();
    let team = evil_team(ctx.state());
    let mut read = ctx.read();
    // Every demon learns the same bluffs, chosen once.
    let mut bluffs: Option<Vec<&'static str>> = None;
    for demon in &demons {
        read.wake(demon);
        if team_shown {
            read.tell(demon, &team);
        }
        let shown = bluffs.get_or_insert_with(|| {
            read.choose(
                storyteller::Reason::DemonBluffs,
                not_in_play.clone(),
                3,
                |name| storyteller::Choice::Character(name),
            )
        });
        read.tell(demon, &info::Message::Bluffs(shown.clone()));
        read.sleep(demon);
    }
}

/// Runs night actions in order. Consecutive read-only actions are announced
/// to the table as one batch; read-write actions are sync points. When a
/// character's turn comes, every player who has its ability then
/// (`game::State::abilities`, so abilities gained or lost earlier tonight
/// count) acts if it `wakes_tonight`. A player with an `ActsTwice` token
/// (the Barista) acts twice in a row.
pub fn run_order(ctx: &mut ctx::Write<'_>, order: &[character::Ref], night: u32) {
    let mut i = 0;
    while i < order.len() {
        if ctx.state().winner().is_some() {
            return;
        }
        let c = order[i];
        i += 1;
        match c.night_action() {
            None => {}
            Some(character::NightAction::ReadWrite(f)) => {
                for id in acting(ctx.state(), c) {
                    for _ in 0..times(ctx.state(), &id) {
                        let state = ctx.state();
                        if state.winner().is_none() && wakes(state, c, &id) {
                            f(ctx, &id, night);
                        }
                    }
                }
            }
            Some(character::NightAction::ReadOnly(f)) => {
                // Read-only actions cannot change who acts: gather the run.
                let mut batch: Vec<(player::Id, character::ReadOnlyFn)> = Vec::new();
                add_turns(ctx.state(), &mut batch, c, f);
                // Characters nobody acts for do not interrupt the run.
                while let Some(next) = order.get(i) {
                    match next.night_action() {
                        Some(character::NightAction::ReadOnly(g)) => {
                            add_turns(ctx.state(), &mut batch, *next, g)
                        }
                        Some(character::NightAction::ReadWrite(_))
                            if !acting(ctx.state(), *next).is_empty() =>
                        {
                            break;
                        }
                        _ => {}
                    }
                    i += 1;
                }
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

/// How many times `player` acts at each of their turns tonight.
fn times(state: &game::State, player: &player::Id) -> usize {
    if state.has_effect(player, game::Effect::ActsTwice) {
        2
    } else {
        1
    }
}

/// Adds the read-only turns of everyone acting for `c` to `batch`.
fn add_turns(
    state: &game::State,
    batch: &mut Vec<(player::Id, character::ReadOnlyFn)>,
    c: character::Ref,
    f: character::ReadOnlyFn,
) {
    for id in acting(state, c) {
        for _ in 0..times(state, &id) {
            batch.push((id.clone(), f));
        }
    }
}

/// Players who have `c`'s ability now and wake for it tonight, in seat order.
fn acting(state: &game::State, c: character::Ref) -> Vec<player::Id> {
    state
        .seat_order()
        .into_iter()
        .filter(|id| wakes(state, c, id))
        .collect()
}

/// Whether `id` has `c`'s ability now and wakes for it tonight (not if a
/// `DoesNotWake` token is on them, as on the Demon the Exorcist chose).
fn wakes(state: &game::State, c: character::Ref, id: &player::Id) -> bool {
    state.has_ability(id, c.id())
        && c.wakes_tonight(state, id)
        && !state.has_effect(id, game::Effect::DoesNotWake)
}

/// The player uses `ability`'s day action (the Slayer's shot), if it has one
/// and they have the ability.
pub fn day_action(ctx: &mut ctx::Write<'_>, player: &player::Id, ability: character::Ref) {
    if !ctx.state().has_ability(player, ability.id()) {
        return;
    }
    if let Some(f) = ability.day_action() {
        f(ctx, player);
    }
}

/// End of day: active players' day-end abilities (the Mayor) fire, then the
/// Fabled and Loric dusk abilities.
pub fn end_day(ctx: &mut ctx::Write<'_>) {
    for (id, ability) in ctx.active_abilities() {
        ability.on_day_end(ctx, &id);
    }
    for f in ctx.state().fabled().to_vec() {
        f.fabled_dusk(ctx);
    }
}

/// Whom the storyteller nominates today, if a living, functioning player's
/// ability says only the storyteller nominates (the Bishop); `None` if the
/// players nominate.
pub fn storyteller_nominations(ctx: &mut ctx::Write<'_>) -> Option<Vec<player::Id>> {
    for (id, ability) in ctx.active_abilities() {
        if !ctx.state().is_alive(&id) || ctx.state().is_malfunctioning(&id) {
            continue;
        }
        if let Some(nominees) = ability.storyteller_nominations(&mut ctx.read(), &id) {
            return Some(nominees);
        }
    }
    None
}

/// Living, functioning players who may nominate again after the day's first
/// execution (the Butcher), in seat order.
pub fn nominate_again(state: &game::State) -> Vec<player::Id> {
    state
        .alive_ids()
        .into_iter()
        .filter(|id| {
            !state.is_malfunctioning(id) && state.abilities(id).iter().any(|c| c.nominates_again())
        })
        .collect()
}

/// The storyteller nominates `nominee` (when only the storyteller may): the
/// nominee's abilities that react to a nominator do not fire.
pub fn storyteller_nominates(ctx: &mut ctx::Write<'_>, nominee: &player::Id) {
    ctx.record(event::Event::Nominated {
        nominee: nominee.clone(),
        nominator: None,
    });
}

/// A nomination: the nominee's abilities react (the Virgin), then the
/// nominator's.
pub fn nominate(ctx: &mut ctx::Write<'_>, nominator: &player::Id, nominee: &player::Id) {
    ctx.record(event::Event::Nominated {
        nominee: nominee.clone(),
        nominator: Some(nominator.clone()),
    });
    for (id, ability) in ctx.abilities_of(std::slice::from_ref(nominee)) {
        ability.on_nominated(ctx, &id, nominator);
    }
    for (id, ability) in ctx.abilities_of(std::slice::from_ref(nominator)) {
        ability.on_nominates(ctx, &id, nominee);
    }
}

/// A vote: the hands raised are counted with their weights (`tally`),
/// recorded, then every player's `on_vote` abilities react. Returns the votes
/// counted.
pub fn vote(
    ctx: &mut ctx::Write<'_>,
    nominee: &player::Id,
    voters: Vec<player::Id>,
    needed: usize,
) -> usize {
    // A hand that counts for nothing (a Beggar without a token) is no vote.
    let voters: Vec<player::Id> = voters
        .into_iter()
        .filter(|v| weight(ctx.state(), v) > 0)
        .collect();
    let votes = tally(ctx.state(), &voters);
    ctx.record(event::Event::Voted {
        nominee: nominee.clone(),
        voters: voters.clone(),
        votes,
        needed,
    });
    let everyone = ctx.state().seat_order();
    for (id, ability) in ctx.abilities_of(&everyone) {
        ability.on_vote(ctx, &id, nominee, &voters);
    }
    votes
}

/// How many votes `voters` count for: each starts at 1, and every
/// functioning player's abilities may change it
/// (`character::Def::vote_weight`).
pub fn tally(state: &game::State, voters: &[player::Id]) -> usize {
    let total: i32 = voters.iter().map(|v| weight(state, v)).sum();
    usize::try_from(total).unwrap_or(0)
}

/// How much `voter`'s vote counts (see [`tally`]).
fn weight(state: &game::State, voter: &player::Id) -> i32 {
    let holders: Vec<(player::Id, character::Ref)> = state
        .seat_order()
        .into_iter()
        .filter(|id| !state.is_malfunctioning(id))
        .flat_map(|id| {
            state
                .abilities(&id)
                .into_iter()
                .map(move |c| (id.clone(), c))
        })
        .collect();
    holders
        .iter()
        .fold(1, |weight, (id, c)| c.vote_weight(state, id, voter, weight))
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
