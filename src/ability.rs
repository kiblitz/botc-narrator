//! Building blocks that character abilities compose, so most characters are a
//! few lines: pick players, learn something about them (registration-aware,
//! with misinformation when malfunctioning), mark a player, kill, use an
//! ability once, or leave a choice to the storyteller.
//!
//! A character wakes and sleeps its player itself (`ctx.wake(me)` ...
//! `ctx.sleep(me)`); the `tell_*` blocks give information to an awake player.
//! Information blocks take a `ctx::Read`; from a read-write ability, call them
//! with `&mut ctx.read()`. Effects take a `ctx::Write` and do nothing while
//! the acting player is malfunctioning, as the rules say.

use crate::character;
use crate::ctx;
use crate::event;
use crate::game;
use crate::info;
use crate::player;
use crate::storyteller;

// --- Who ----------------------------------------------------------------------

/// Everyone except `me`, in seat order (dead players too).
pub fn others(state: &game::State, me: &player::Id) -> Vec<player::Id> {
    state.seat_order().into_iter().filter(|p| p != me).collect()
}

/// `me`'s nearest living neighbours on each side (one if only two live).
pub fn alive_neighbours(state: &game::State, me: &player::Id) -> Vec<player::Id> {
    let seats = state.seat_order();
    let n = seats.len();
    let Some(i) = seats.iter().position(|p| p == me) else {
        return Vec::new();
    };
    let nearest = |seat: &dyn Fn(usize) -> usize| {
        (1..n)
            .map(|k| &seats[seat(k)])
            .find(|p| state.is_alive(p))
            .cloned()
    };
    let left = nearest(&|k| (i + n - k) % n);
    let right = nearest(&|k| (i + k) % n);
    let mut neighbours: Vec<player::Id> = [left, right].into_iter().flatten().collect();
    neighbours.dedup();
    neighbours.retain(|p| p != me);
    neighbours
}

/// Players who died since the current day began (or during the last day, if
/// it is night): "if an Outsider died today".
pub fn died_today(state: &game::State) -> Vec<player::Id> {
    let history = state.history();
    let day_start = history
        .iter()
        .rposition(|e| matches!(e, event::Event::PhaseBegan(game::Phase::Day(_))));
    let Some(start) = day_start else {
        return Vec::new();
    };
    let day_end = history[start..]
        .iter()
        .position(|e| matches!(e, event::Event::PhaseBegan(game::Phase::Night(_))))
        .map_or(history.len(), |i| start + i);
    history[start..day_end]
        .iter()
        .filter_map(|e| match e {
            event::Event::Died { player, .. } => Some(player.clone()),
            _ => None,
        })
        .collect()
}

/// Characters of `kind` on the script that nobody is, or believes they are.
pub fn not_in_play(state: &game::State, kind: character::Kind) -> Vec<character::Ref> {
    state
        .script()
        .iter()
        .copied()
        .filter(|c| c.kind() == kind && !in_play(state, *c))
        .collect()
}

/// Whether anyone is, or believes they are, this character.
pub fn in_play(state: &game::State, c: character::Ref) -> bool {
    state
        .players()
        .iter()
        .any(|p| character::same(p.character, c) || character::same(p.believed(), c))
}

/// Every way to pick `size` players from `players`, keeping seat order.
pub fn groups(players: &[player::Id], size: usize) -> Vec<Vec<player::Id>> {
    if size == 0 {
        return vec![Vec::new()];
    }
    let mut out = Vec::new();
    for (i, first) in players.iter().enumerate() {
        for mut rest in groups(&players[i + 1..], size - 1) {
            rest.insert(0, first.clone());
            out.push(rest);
        }
    }
    out
}

// --- Learning -------------------------------------------------------------------

/// Tell `me` a number from 0 to `max` (`count` is true).
pub fn tell_count(
    ctx: &mut ctx::Read<'_>,
    me: &player::Id,
    count: usize,
    max: usize,
    noun: &'static str,
) {
    let truthful = vec![info::Message::Count { count, noun }];
    ctx.inform(me, truthful, counts(0..=max, noun));
}

/// Tell `me` yes or no (`yes` is true).
pub fn tell_yes_no(ctx: &mut ctx::Read<'_>, me: &player::Id, yes: bool) {
    let truthful = vec![info::Message::YesNo(yes)];
    ctx.inform(me, truthful, vec![info::Message::YesNo(!yes)]);
}

/// Tell `me` which character `target` is (as they register to `me`); any
/// in-play character is legal misinformation.
pub fn tell_character(ctx: &mut ctx::Read<'_>, me: &player::Id, target: &player::Id) {
    let registered = ctx.registered_character(me, target);
    let truthful = vec![info::Message::Character(registered.name())];
    let misinformation = any_character(ctx.state());
    ctx.inform(me, truthful, misinformation);
}

/// How many of `players` register as evil to `me`.
pub fn count_evil(ctx: &mut ctx::Read<'_>, me: &player::Id, players: &[player::Id]) -> usize {
    players.iter().filter(|p| ctx.registers_evil(me, p)).count()
}

/// Washerwoman-style information: "one of these two players is the X",
/// where X is a `kind` character (as they register to `me`). Shown in both
/// seat orders so the storyteller also picks the order. Misinformation is
/// any two other players and any in-play character of that kind.
pub fn tell_one_of(ctx: &mut ctx::Read<'_>, me: &player::Id, kind: character::Kind) {
    let others = others(ctx.state(), me);
    let registered: Vec<(player::Id, character::Ref)> = others
        .iter()
        .filter_map(|p| ctx.registers_as(me, p, kind).map(|c| (p.clone(), c)))
        .collect();
    let state = ctx.state();
    let pairs_with = |a: &player::Id| -> Vec<[player::Id; 2]> {
        others
            .iter()
            .filter(|b| *b != a)
            .flat_map(|b| [[a.clone(), b.clone()], [b.clone(), a.clone()]])
            .collect()
    };
    let mut truthful = Vec::new();
    for (target, character) in &registered {
        let character = character.name();
        for players in pairs_with(target) {
            truthful.push(info::Message::OneOf { players, character });
        }
    }
    let none = info::Message::Count {
        count: 0,
        noun: noun(kind),
    };
    if truthful.is_empty() {
        truthful.push(none.clone());
    }
    let mut misinformation = vec![none];
    let names: Vec<&'static str> = state
        .players()
        .iter()
        .filter(|p| p.character.kind() == kind)
        .map(|p| p.character.name())
        .collect();
    for (i, a) in others.iter().enumerate() {
        for b in &others[i + 1..] {
            for &character in &names {
                misinformation.push(info::Message::OneOf {
                    players: [a.clone(), b.clone()],
                    character,
                });
            }
        }
    }
    ctx.inform(me, truthful, misinformation);
}

/// Misinformation for "learn a character" abilities: any in-play character.
pub fn any_character(state: &game::State) -> Vec<info::Message> {
    state
        .character_names()
        .into_iter()
        .map(info::Message::Character)
        .collect()
}

/// Every count in `range` of `noun`, as misinformation.
pub fn counts(range: std::ops::RangeInclusive<usize>, noun: &'static str) -> Vec<info::Message> {
    range
        .map(|count| info::Message::Count { count, noun })
        .collect()
}

/// The plural noun for a kind, as in "0 outsiders".
pub fn noun(kind: character::Kind) -> &'static str {
    match kind {
        character::Kind::Townsfolk => "townsfolk",
        character::Kind::Outsider => "outsiders",
        character::Kind::Minion => "minions",
        character::Kind::Demon => "demons",
        character::Kind::Traveller => "travellers",
        character::Kind::Fabled => "fabled",
    }
}

// --- Choosing -------------------------------------------------------------------

/// The storyteller answers a yes/no question about `me`'s ability; "no" is
/// the first (default) option.
pub fn storyteller_agrees(
    ctx: &mut ctx::Read<'_>,
    me: &player::Id,
    character: character::Ref,
    question: &'static str,
) -> bool {
    let reason = storyteller::Reason::Ability {
        player: me.clone(),
        character: character.name(),
        question,
    };
    ctx.choose(reason, vec![false, true], 1, |yes| {
        storyteller::Choice::YesNo(*yes)
    })
    .first()
    .copied()
    .unwrap_or(false)
}

/// The storyteller picks one of `options` for `me`'s ability.
pub fn storyteller_picks_player(
    ctx: &mut ctx::Read<'_>,
    me: &player::Id,
    character: character::Ref,
    question: &'static str,
    options: Vec<player::Id>,
) -> Option<player::Id> {
    let reason = storyteller::Reason::Ability {
        player: me.clone(),
        character: character.name(),
        question,
    };
    ctx.choose_player(reason, options)
}

/// The storyteller picks one of `options` (characters) for `me`'s ability.
pub fn storyteller_picks_character(
    ctx: &mut ctx::Read<'_>,
    me: &player::Id,
    character: character::Ref,
    question: &'static str,
    options: Vec<character::Ref>,
) -> Option<character::Ref> {
    let reason = storyteller::Reason::Ability {
        player: me.clone(),
        character: character.name(),
        question,
    };
    ctx.choose(reason, options, 1, |c| {
        storyteller::Choice::Character(c.name())
    })
    .into_iter()
    .next()
}

// --- Effects ----------------------------------------------------------------------

/// `source`'s ability (held by `me`) puts an `effect` token on `target`,
/// unless `me` is malfunctioning: poison, protect, and the like.
pub fn mark(
    ctx: &mut ctx::Write<'_>,
    me: &player::Id,
    source: character::Ref,
    target: &player::Id,
    effect: game::Effect,
    expiry: game::Expiry,
) {
    if !ctx.state().is_malfunctioning(me) {
        let reminder = game::Reminder::new(source, me, effect, expiry);
        ctx.place_reminder(target, reminder);
    }
}

/// Whether `source`'s token placed by `me` is on `target`.
pub fn has_mark(
    state: &game::State,
    me: &player::Id,
    source: character::Ref,
    target: &player::Id,
    effect: game::Effect,
) -> bool {
    state
        .player(target)
        .reminders
        .iter()
        .any(|r| &r.placed_by == me && r.source == source.id() && r.effect == effect)
}

/// A once-per-game ability: if `me` has not used it, mark it used (even when
/// malfunctioning, as the rules say) and return true.
pub fn use_once(ctx: &mut ctx::Write<'_>, me: &player::Id, source: character::Ref) -> bool {
    if ctx.state().has_effect(me, game::Effect::AbilityUsed) {
        return false;
    }
    let reminder = game::Reminder::new(source, me, game::Effect::AbilityUsed, game::Expiry::Never);
    ctx.place_reminder(me, reminder);
    true
}

/// Kill `target` with `me`'s ability, unless `me` is malfunctioning.
/// Returns whether someone died.
pub fn kill(ctx: &mut ctx::Write<'_>, me: &player::Id, target: &player::Id) -> bool {
    !ctx.state().is_malfunctioning(me) && ctx.kill(target, event::Cause::Ability { by: me.clone() })
}

/// The demon's nightly attack: wake, choose a living player, sleep, and kill
/// them as a demon kill (unless malfunctioning). Choosing themself is a
/// starpass. Returns the choice.
pub fn demon_attack(ctx: &mut ctx::Write<'_>, me: &player::Id) -> Option<player::Id> {
    let candidates = ctx.state().alive_ids();
    ctx.wake(me);
    let target = ctx.ask_one(me, "Who do you want to kill?", &candidates);
    ctx.sleep(me);
    if let Some(target) = &target
        && !ctx.state().is_malfunctioning(me)
    {
        let cause = if target == me {
            event::Cause::Starpass
        } else {
            event::Cause::Demon
        };
        ctx.kill(target, cause);
    }
    target
}
