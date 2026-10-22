//! A room's game, on a thread of its own: the engine's narrator runs the
//! nights, and the days follow the host (nominations, votes) and the phones (day
//! abilities) until the host ends them. The host starts each night.

use std::cell;
use std::panic;
use std::rc;
use std::sync;
use std::thread;
use std::time;

use botc_narrator::ability;
use botc_narrator::character;
use botc_narrator::characters;
use botc_narrator::ctx;
use botc_narrator::game;
use botc_narrator::narrator;
use botc_narrator::player;
use botc_narrator::script;
use botc_narrator::sim;
use botc_narrator::storyteller;
use rand::SeedableRng;

use crate::bridged;
use crate::inbox;
use crate::phone;
use crate::room;

/// How long the table waits for people and the bridge.
#[derive(Clone, Copy, Debug)]
pub struct Timing {
    /// A player's answer to a prompt.
    pub answer: time::Duration,
    /// The bridge's answer to a storyteller decision.
    pub decision: time::Duration,
}

impl Default for Timing {
    fn default() -> Self {
        Timing {
            answer: time::Duration::from_secs(90),
            decision: time::Duration::from_secs(90),
        }
    }
}

/// Deals characters to the room's seats and starts the game's thread.
/// `seed` makes the deal and the random storyteller reproducible (tests).
pub fn start(
    room: &sync::Arc<room::Room>,
    timing: Timing,
    seed: Option<u64>,
) -> Result<(), room::Error> {
    let mut rng = match seed {
        Some(seed) => rand::rngs::StdRng::seed_from_u64(seed),
        None => rand::rngs::StdRng::from_os_rng(),
    };
    let fallback = storyteller::Random::seeded(rand::Rng::random(&mut rng));
    let started = room.update(|state| {
        let script = state.startable()?;
        let names: Vec<String> = state.seats.iter().map(|s| s.name.clone()).collect();
        let dealt = sim::setup(&mut rng, names.len(), &script);
        let seats: Vec<(player::Id, character::Ref)> = names
            .iter()
            .zip(dealt)
            .map(|(name, (_, character))| (player::Id::new(name.as_str()), character))
            .collect();
        let grimoire = game::State::new(seats);
        let (sender, inbox) = inbox::Inbox::new();
        state.game = Some(room::Game::new(sender, grimoire.clone()));
        Ok((script, grimoire, inbox))
    });
    let (script, grimoire, inbox) = started?;
    let failed = sync::Arc::clone(room);
    let room = sync::Arc::clone(room);
    tracing::info!(room = %room.code, script = script.id, players = grimoire.players().len(), "game starts");
    let spawned = thread::Builder::new()
        .name(format!("room-{}", room.code))
        .spawn(move || run(&room, &script, grimoire, inbox, timing, fallback));
    if let Err(e) = spawned {
        tracing::error!(error = %e, "could not start a game thread");
        failed.update(|state| {
            if let Some(game) = &mut state.game {
                game.stage = room::Stage::Over;
                game.failure = Some("the server is too busy to run another game".to_string());
            }
        });
    }
    Ok(())
}

/// Plays the game, and if the engine panics, says so to the table rather
/// than leaving it waiting.
fn run(
    room: &sync::Arc<room::Room>,
    script: &script::Script,
    grimoire: game::State,
    inbox: inbox::Inbox,
    timing: Timing,
    fallback: storyteller::Random,
) {
    let played = panic::catch_unwind(panic::AssertUnwindSafe(|| {
        play(room, script, grimoire, inbox, timing, fallback)
    }));
    let failure = played.err().map(|panic| {
        let reason = panic
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| panic.downcast_ref::<&str>().map(|s| s.to_string()))
            .unwrap_or_else(|| "unknown".to_string());
        tracing::error!(room = %room.code, %reason, "the game broke");
        reason
    });
    room.update(|state| {
        if let Some(game) = &mut state.game {
            game.stage = room::Stage::Over;
            game.prompts.clear();
            game.vote = None;
            game.thinking = false;
            game.failure = failure;
        }
    });
    tracing::info!(room = %room.code, "game over");
}

fn play(
    room: &sync::Arc<room::Room>,
    script: &script::Script,
    mut grimoire: game::State,
    inbox: inbox::Inbox,
    timing: Timing,
    fallback: storyteller::Random,
) {
    let inbox = rc::Rc::new(cell::RefCell::new(inbox));
    let mut table = phone::Table::new(sync::Arc::clone(room), rc::Rc::clone(&inbox), timing.answer);
    let mut storyteller = bridged::Storyteller::new(
        sync::Arc::clone(room),
        rc::Rc::clone(&inbox),
        timing.decision,
        fallback,
    );
    let mut ctx = ctx::Write::new(&mut grimoire, &mut table, &mut storyteller);
    narrator::setup(&mut ctx, script);
    let open = |inbox: &rc::Rc<cell::RefCell<inbox::Inbox>>| !inbox.borrow().is_closed();
    while ctx.state().winner().is_none() && open(&inbox) {
        match ctx.advance_phase() {
            game::Phase::Night(_) => {
                set_stage(room, room::Stage::Night);
                narrator::run_night(&mut ctx, script);
            }
            game::Phase::Day(_) => {
                day(&mut ctx, room, &inbox);
                if ctx.state().winner().is_some() || !open(&inbox) {
                    break;
                }
                set_stage(room, room::Stage::Dusk);
                let night = inbox.borrow_mut().wait(None, |input| match input {
                    inbox::Input::StartNight => inbox::Verdict::Take,
                    _ => inbox::Verdict::Drop,
                });
                if night.is_none() {
                    break;
                }
            }
            game::Phase::Setup => {}
        }
    }
    let winner = ctx.state().winner();
    room.update(|state| {
        if let Some(game) = &mut state.game {
            game.winner = winner;
        }
    });
}

fn set_stage(room: &room::Room, stage: room::Stage) {
    room.update(|state| {
        if let Some(game) = &mut state.game {
            game.stage = stage;
            game.end_requested = false;
            game.night_requested = false;
        }
    });
}

/// A day: the host enters nominations and votes, and players use public day
/// abilities, until the host ends it; then the nominee on the block is executed.
fn day(ctx: &mut ctx::Write<'_>, room: &room::Room, inbox: &rc::Rc<cell::RefCell<inbox::Inbox>>) {
    room.update(|state| {
        if let Some(game) = &mut state.game {
            game.stage = room::Stage::Day;
            game.nominators.clear();
            game.nominees.clear();
            game.end_requested = false;
        }
    });
    let mut block = narrator::Block::default();
    // Only the storyteller nominates today (the Bishop): those votes happen
    // at once, and the players only use day abilities.
    let storyteller_nominates = narrator::storyteller_nominations(ctx);
    let players_nominate = storyteller_nominates.is_none();
    for nominee in storyteller_nominates.unwrap_or_default() {
        if ctx.state().winner().is_some() {
            return;
        }
        match nominate_and_vote(ctx, room, inbox, None, &nominee) {
            Voted::Counted(votes) => block.add(nominee, votes, narrator::votes_needed(ctx.state())),
            Voted::NoVote => {}
            Voted::DayEnded => return narrator::end_day(ctx),
        }
    }
    loop {
        refresh(ctx, room, players_nominate);
        let input = inbox.borrow_mut().wait(None, |input| match input {
            inbox::Input::Nominate { .. }
            | inbox::Input::DayAction { .. }
            | inbox::Input::EndDay => inbox::Verdict::Take,
            inbox::Input::StartNight => inbox::Verdict::Drop,
            other => inbox::keep_commands(other),
        });
        match input {
            None => return,
            Some(inbox::Input::EndDay) => break,
            Some(inbox::Input::Nominate { nominator, nominee }) => {
                let nominator = player::Id::new(nominator);
                let nominee = player::Id::new(nominee);
                if !players_nominate || !may_nominate(ctx.state(), room, &nominator, &nominee) {
                    continue;
                }
                match nominate_and_vote(ctx, room, inbox, Some(&nominator), &nominee) {
                    Voted::Counted(votes) => {
                        block.add(nominee, votes, narrator::votes_needed(ctx.state()))
                    }
                    Voted::NoVote => {}
                    // The nomination executed the nominator (the Virgin): the day ends.
                    Voted::DayEnded => return narrator::end_day(ctx),
                }
            }
            Some(inbox::Input::DayAction { player, ability }) => {
                let player = player::Id::new(player);
                if let Some(ability) = usable(ctx, &player, &ability) {
                    narrator::day_action(ctx, &player, ability);
                }
            }
            Some(_) => {}
        }
        if ctx.state().winner().is_some() {
            return;
        }
    }
    narrator::execute_block(ctx, &block);
    if players_nominate && block.nominee().is_some() {
        nominate_again(ctx, room, inbox);
    }
    narrator::finish_day(ctx);
}

/// After the day's first execution, the Butcher may nominate again (once: a
/// second execution does not allow a third).
fn nominate_again(
    ctx: &mut ctx::Write<'_>,
    room: &room::Room,
    inbox: &rc::Rc<cell::RefCell<inbox::Inbox>>,
) {
    let Some(butcher) = narrator::nominate_again(ctx.state()).first().cloned() else {
        return;
    };
    if ctx.state().winner().is_some() {
        return;
    }
    let nominated = room.read(|s| {
        s.game
            .as_ref()
            .map(|g| g.nominees.clone())
            .unwrap_or_default()
    });
    let candidates: Vec<player::Id> = ctx
        .state()
        .alive_except(&butcher)
        .into_iter()
        .filter(|p| !nominated.iter().any(|n| n == p.as_str()))
        .collect();
    let chosen = ctx
        .io()
        .ask_up_to(&butcher, "Nominate again? (your ability)", &candidates, 1);
    let Some(nominee) = chosen.first() else {
        return;
    };
    if let Voted::Counted(votes) = nominate_and_vote(ctx, room, inbox, Some(&butcher), nominee)
        && votes >= narrator::votes_needed(ctx.state())
        && ctx.state().winner().is_none()
    {
        ctx.execute(nominee);
    }
}

/// Whether `nominator` may nominate `nominee` now: a living player who may
/// nominate and has not today, of a seated player not yet nominated today.
fn may_nominate(
    state: &game::State,
    room: &room::Room,
    nominator: &player::Id,
    nominee: &player::Id,
) -> bool {
    let today = room.read(|s| {
        s.game.as_ref().is_some_and(|g| {
            !g.nominators.iter().any(|n| n == nominator.as_str())
                && !g.nominees.iter().any(|n| n == nominee.as_str())
        })
    });
    today
        && state.is_alive(nominator)
        && narrator::can_nominate(state, nominator)
        && state.players().iter().any(|p| p.id == *nominee)
}

/// The ability `id`, if `player` may use it as a day action now.
fn usable(ctx: &ctx::Write<'_>, player: &player::Id, id: &str) -> Option<character::Ref> {
    let ability = characters::all().iter().copied().find(|c| c.id() == id)?;
    let state = ctx.state();
    let may = state.is_alive(player)
        && !ability::used(state, player, ability)
        && narrator::day_action_users(state, ability).contains(player);
    may.then_some(ability)
}

/// Who may nominate and which day abilities can be used, for the phones.
fn refresh(ctx: &ctx::Write<'_>, room: &room::Room, players_nominate: bool) {
    let state = ctx.state();
    let mut actions: Vec<room::DayAction> = Vec::new();
    for (_, ability) in ctx.active_abilities() {
        for user in narrator::day_action_users(state, ability) {
            let fresh = !actions
                .iter()
                .any(|a| a.player == user.as_str() && a.ability == ability.id());
            if fresh && state.is_alive(&user) && !ability::used(state, &user, ability) {
                actions.push(room::DayAction {
                    player: user.to_string(),
                    ability: ability.id(),
                    name: ability.name(),
                });
            }
        }
    }
    let able: Vec<String> = state
        .alive_ids()
        .into_iter()
        .filter(|p| players_nominate && narrator::can_nominate(state, p))
        .map(|p| p.to_string())
        .collect();
    room.update(|s| {
        if let Some(game) = &mut s.game {
            game.may_nominate = able
                .into_iter()
                .filter(|p| !game.nominators.contains(p))
                .collect();
            game.day_actions = actions;
        }
    });
}

/// How a nomination ended.
enum Voted {
    /// The nomination ended the day (the Virgin) or the game.
    DayEnded,
    /// Nothing to vote on (the Golem killed the nominee).
    NoVote,
    /// The votes counted.
    Counted(usize),
}

/// Nominates `nominee` (by the storyteller when `nominator` is `None`) and
/// holds the vote: the host enters the hands and closes it.
fn nominate_and_vote(
    ctx: &mut ctx::Write<'_>,
    room: &room::Room,
    inbox: &rc::Rc<cell::RefCell<inbox::Inbox>>,
    nominator: Option<&player::Id>,
    nominee: &player::Id,
) -> Voted {
    room.update(|s| {
        if let Some(game) = &mut s.game {
            if let Some(nominator) = nominator {
                game.nominators.push(nominator.to_string());
                game.may_nominate.retain(|p| p != nominator.as_str());
            }
            game.nominees.push(nominee.to_string());
        }
    });
    match narrator::nominate_by(ctx, nominator, nominee) {
        narrator::Nominated::DayEnded => return Voted::DayEnded,
        narrator::Nominated::NoVote => return Voted::NoVote,
        narrator::Nominated::Vote => {}
    }
    let needed = narrator::votes_needed(ctx.state());
    let eligible: Vec<String> = ctx
        .state()
        .players()
        .iter()
        .filter(|p| p.alive || p.ghost_vote)
        .map(|p| p.id.to_string())
        .collect();
    room.update(|s| {
        if let Some(game) = &mut s.game {
            game.stage = room::Stage::Vote;
            game.vote = Some(room::Vote {
                nominee: nominee.to_string(),
                nominator: nominator.map(ToString::to_string),
                needed,
                eligible: eligible.clone(),
                hands: Vec::new(),
            });
        }
    });
    // The host raises and lowers hands round the circle, then closes the vote.
    let mut hands: Vec<(String, bool)> = Vec::new();
    loop {
        let input = inbox.borrow_mut().wait(None, |input| match input {
            inbox::Input::Vote { voter, .. } if eligible.contains(voter) => inbox::Verdict::Take,
            inbox::Input::CloseVote => inbox::Verdict::Take,
            other => inbox::keep_commands(other),
        });
        match input {
            Some(inbox::Input::Vote { voter, yes }) => {
                hands.retain(|(p, _)| *p != voter);
                hands.push((voter, yes));
                let shown = hands.clone();
                room.update(|s| {
                    if let Some(vote) = s.game.as_mut().and_then(|g| g.vote.as_mut()) {
                        vote.hands = shown;
                    }
                });
            }
            _ => break,
        }
    }
    room.update(|s| {
        if let Some(game) = &mut s.game {
            game.stage = room::Stage::Day;
            game.vote = None;
        }
    });
    // Raised hands, in seat order; a dead voter spends their ghost vote.
    let voters: Vec<player::Id> = ctx
        .state()
        .seat_order()
        .into_iter()
        .filter(|p| hands.iter().any(|(h, yes)| *yes && h == p.as_str()))
        .collect();
    for voter in &voters {
        if !ctx.state().is_alive(voter) {
            ctx.spend_ghost_vote(voter);
        }
    }
    Voted::Counted(narrator::vote(ctx, nominee, voters, needed))
}
