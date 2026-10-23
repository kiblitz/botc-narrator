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

/// `me`'s nearest `kind` players clockwise and anticlockwise, alive or dead
/// (one if both directions find the same player): "your 2 Townsfolk
/// neighbours".
pub fn nearest_of_kind(
    state: &game::State,
    me: &player::Id,
    kind: character::Kind,
) -> Vec<player::Id> {
    let seats = state.seat_order();
    let n = seats.len();
    let Some(i) = seats.iter().position(|p| p == me) else {
        return Vec::new();
    };
    let nearest = |seat: &dyn Fn(usize) -> usize| {
        (1..n)
            .map(|k| &seats[seat(k)])
            .find(|p| state.kind(p) == kind)
            .cloned()
    };
    let mut found: Vec<player::Id> = [nearest(&|k| (i + n - k) % n), nearest(&|k| (i + k) % n)]
        .into_iter()
        .flatten()
        .collect();
    found.dedup();
    found
}

/// The events of the current day (or of the last day, if it is night).
pub fn today(state: &game::State) -> &[event::Event] {
    let history = state.history();
    let day_start = history
        .iter()
        .rposition(|e| matches!(e, event::Event::PhaseBegan(game::Phase::Day(_))));
    let Some(start) = day_start else {
        return &[];
    };
    let day_end = history[start..]
        .iter()
        .position(|e| matches!(e, event::Event::PhaseBegan(game::Phase::Night(_))))
        .map_or(history.len(), |i| start + i);
    &history[start..day_end]
}

/// Players who died since the current day began (or during the last day, if
/// it is night): "if an Outsider died today".
pub fn died_today(state: &game::State) -> Vec<player::Id> {
    today(state)
        .iter()
        .filter_map(|e| match e {
            event::Event::Died { player, .. } => Some(player.clone()),
            _ => None,
        })
        .collect()
}

/// Players who nominated since the current day began (or during the last
/// day, if it is night), in nomination order.
pub fn nominators_today(state: &game::State) -> Vec<player::Id> {
    today(state)
        .iter()
        .filter_map(|e| match e {
            event::Event::Nominated { nominator, .. } => nominator.clone(),
            _ => None,
        })
        .collect()
}

/// Today's nominations as (nominator, nominee), in order; the nominator is
/// `None` when the storyteller nominated.
pub fn nominations_today(state: &game::State) -> Vec<(Option<player::Id>, player::Id)> {
    today(state)
        .iter()
        .filter_map(|e| match e {
            event::Event::Nominated { nominee, nominator } => {
                Some((nominator.clone(), nominee.clone()))
            }
            _ => None,
        })
        .collect()
}

/// What `me` publicly claimed today with `source`'s ability (or during the
/// last day, if it is night): pairs of a player and the name of the
/// character `me` said they are (names, since events compare by value).
pub fn claims_today(
    state: &game::State,
    me: &player::Id,
    source: character::Ref,
) -> Vec<(player::Id, &'static str)> {
    today(state)
        .iter()
        .filter_map(|e| match e {
            event::Event::Claimed {
                player,
                ability,
                claims,
            } if player == me && *ability == source.id() => Some(claims.clone()),
            _ => None,
        })
        .flatten()
        .collect()
}

/// `me` publicly claims, with `source`'s ability, that each of `claims` is
/// that character.
pub fn claim(
    ctx: &mut ctx::Write<'_>,
    me: &player::Id,
    source: character::Ref,
    claims: Vec<(player::Id, character::Ref)>,
) {
    let claims = claims.into_iter().map(|(p, c)| (p, c.name())).collect();
    ctx.record(event::Event::Claimed {
        player: me.clone(),
        ability: source.id(),
        claims,
    });
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

/// Characters of these kinds that information may name: the script's, or
/// the in-play ones when no script is recorded.
pub fn nameable(state: &game::State, kinds: &[character::Kind]) -> Vec<character::Ref> {
    let pool: Vec<character::Ref> = if state.script().is_empty() {
        state.players().iter().map(|p| p.character).collect()
    } else {
        state.script().to_vec()
    };
    let mut out: Vec<character::Ref> = Vec::new();
    for c in pool {
        if kinds.contains(&c.kind()) && !out.iter().any(|o| character::same(*o, c)) {
            out.push(c);
        }
    }
    out
}

/// Whether `player` wakes tonight (night `night`) for one of their abilities
/// (own, believed or gained): one with a night action and a turn tonight
/// that it wakes for. Judged now, so a turn still to come counts and a player
/// who died after their turn does not (see README, "Not yet modelled").
pub fn wakes_for_ability(state: &game::State, player: &player::Id, night: u32) -> bool {
    state.abilities(player).iter().any(|c| {
        let order = c.night_order();
        let turn = if night == 1 { order.first } else { order.other };
        let on_script =
            state.script().is_empty() || state.script().iter().any(|s| character::same(*s, *c));
        c.night_action().is_some() && turn.is_some() && on_script && c.wakes_tonight(state, player)
    })
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

/// The grimoire as a player sees it (the Spy, the Widow): every seat's character.
pub fn grimoire(state: &game::State) -> info::Message {
    info::Message::Grimoire(
        state
            .players()
            .iter()
            .map(|p| (p.id.clone(), p.character.name()))
            .collect(),
    )
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
        character::Kind::Loric => "loric",
    }
}

// --- Choosing -------------------------------------------------------------------

/// Why the storyteller decides something for `me`'s `character` ability.
pub fn reason(
    me: &player::Id,
    character: character::Ref,
    question: &'static str,
) -> storyteller::Reason {
    storyteller::Reason::Ability {
        player: me.clone(),
        character: character.name(),
        question,
    }
}

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

/// Makes exactly `targets` carry `source`'s `effect` token placed by `me`
/// (several copies, as with the No Dashii's two Poisoned tokens): tokens on
/// other players are removed, missing ones added.
pub fn sync_marks(
    ctx: &mut ctx::Write<'_>,
    me: &player::Id,
    source: character::Ref,
    effect: game::Effect,
    expiry: game::Expiry,
    targets: &[player::Id],
) {
    let reminder = game::Reminder::new(source, me, effect, expiry);
    for p in ctx.state().seat_order() {
        let has = ctx.state().player(&p).reminders.contains(&reminder);
        let wanted = targets.contains(&p);
        if has && !wanted {
            ctx.remove_reminder(&p, &reminder);
        } else if wanted && !has {
            ctx.add_reminder(&p, reminder.clone());
        }
    }
}

/// Like [`mark`], with the same token on each of `targets` (moving the
/// ability's earlier ones): the Innkeeper's two safe players, the Minstrel's
/// drunk table.
pub fn mark_all(
    ctx: &mut ctx::Write<'_>,
    me: &player::Id,
    source: character::Ref,
    targets: &[player::Id],
    effect: game::Effect,
    expiry: game::Expiry,
) {
    if !ctx.state().is_malfunctioning(me) {
        let reminder = game::Reminder::new(source, me, effect, expiry);
        ctx.place_reminders(targets, reminder);
    }
}

/// Players carrying `source`'s `effect` token placed by `me`, in seat order.
pub fn marked(
    state: &game::State,
    me: &player::Id,
    source: character::Ref,
    effect: game::Effect,
) -> Vec<player::Id> {
    state
        .seat_order()
        .into_iter()
        .filter(|p| has_mark(state, me, source, p, effect))
        .collect()
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

/// Whether `me` used `source`'s once-per-game ability during the current night.
pub fn used_tonight(state: &game::State, me: &player::Id, source: character::Ref) -> bool {
    let history = state.history();
    let night_start = history
        .iter()
        .rposition(|e| matches!(e, event::Event::PhaseBegan(game::Phase::Night(_))));
    matches!(state.phase(), game::Phase::Night(_))
        && night_start.is_some_and(|start| {
            history[start..].iter().any(|e| match e {
                event::Event::ReminderPlaced { player, reminder } => {
                    player == me
                        && &reminder.placed_by == me
                        && reminder.source == source.id()
                        && reminder.effect == game::Effect::AbilityUsed
                }
                _ => false,
            })
        })
}

/// Every player `source`'s ability (held by `me`) has ever put an `effect`
/// token on, in the order they were marked.
pub fn ever_marked(
    state: &game::State,
    me: &player::Id,
    source: character::Ref,
    effect: game::Effect,
) -> Vec<player::Id> {
    state
        .history()
        .iter()
        .filter_map(|e| match e {
            event::Event::ReminderPlaced { player, reminder }
                if &reminder.placed_by == me
                    && reminder.source == source.id()
                    && reminder.effect == effect =>
            {
                Some(player.clone())
            }
            _ => None,
        })
        .collect()
}

/// Whether `me` has used `source`'s once-per-game ability.
pub fn used(state: &game::State, me: &player::Id, source: character::Ref) -> bool {
    has_mark(state, me, source, me, game::Effect::AbilityUsed)
}

/// A once-per-game ability: if `me` has not used `source`'s ability, mark it
/// used (even when malfunctioning, as the rules say) and return true.
pub fn use_once(ctx: &mut ctx::Write<'_>, me: &player::Id, source: character::Ref) -> bool {
    if used(ctx.state(), me, source) {
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

/// A demon kill of `target` by `me` (Monk protection applies), unless `me`
/// is malfunctioning. Returns whether someone died.
pub fn demon_kill(ctx: &mut ctx::Write<'_>, me: &player::Id, target: &player::Id) -> bool {
    !ctx.state().is_malfunctioning(me) && ctx.kill(target, event::Cause::Demon)
}

/// The demon's nightly attack: wake, choose a living player, sleep, and kill
/// them as a demon kill (unless malfunctioning). Choosing themself is a
/// starpass. Returns the choice.
pub fn demon_attack(ctx: &mut ctx::Write<'_>, me: &player::Id) -> Option<player::Id> {
    let candidates = ctx.state().alive_ids();
    ctx.wake(me);
    let target = ctx.ask_one(me, "Who do you want to kill?", &candidates);
    ctx.sleep(me);
    match &target {
        Some(target) if target == me => {
            if !ctx.state().is_malfunctioning(me) {
                ctx.kill(target, event::Cause::Starpass);
            }
        }
        Some(target) => {
            demon_kill(ctx, me, target);
        }
        None => {}
    }
    target
}
