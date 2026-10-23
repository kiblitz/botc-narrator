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

/// The evil team as the minions and demon learn it: the evil Minions and
/// Demons (not evil Travellers, nor a Townsfolk turned evil), and players
/// whose ability joins it (the Magician); not evil players who believe they
/// are good (the Marionette).
fn evil_team(state: &game::State) -> info::Message {
    info::Message::Team {
        evil: state
            .players()
            .iter()
            .filter(|p| {
                let evil = p.alignment == character::Alignment::Evil
                    && matches!(
                        p.character.kind(),
                        character::Kind::Minion | character::Kind::Demon
                    )
                    && !p.believes_good();
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

/// Minions learn who is evil; a Minion who believes they are good does not wake.
pub fn minion_info(ctx: &mut ctx::Write<'_>) {
    let minions: Vec<player::Id> = ctx
        .state()
        .alive_of_kind(character::Kind::Minion)
        .into_iter()
        .filter(|m| !ctx.state().player(m).believes_good())
        .collect();
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
                            ctx.resolve_choices();
                        }
                    }
                }
            }
            Some(character::NightAction::ReadOnly(f)) => {
                // Read-only actions cannot change who acts: gather the run.
                // (Their night choices are resolved after each action and may
                // change the grimoire, e.g. a Goon making the chooser drunk,
                // but not who has which ability.)
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
                    ctx.resolve_choices();
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

/// Players who have `c`'s ability now and wake for it tonight, in seat
/// order, except that players who only believe they are `c` go first (the
/// Lunatic wakes before the real Demon, who learns their choices).
fn acting(state: &game::State, c: character::Ref) -> Vec<player::Id> {
    let mut acting: Vec<player::Id> = state
        .seat_order()
        .into_iter()
        .filter(|id| wakes(state, c, id))
        .collect();
    let believes = |id: &player::Id| {
        state
            .player(id)
            .believes
            .is_some_and(|b| character::same(b, c))
    };
    acting.sort_by_key(|id| !believes(id));
    acting
}

/// Who may really use `ability`'s day action now: for each player with the
/// ability active, its `day_action_users` (themself, or every Minion for the
/// Damsel's guess).
pub fn day_action_users(state: &game::State, ability: character::Ref) -> Vec<player::Id> {
    if ability.day_action().is_none() {
        return Vec::new();
    }
    let mut users: Vec<player::Id> = Vec::new();
    for holder in state.seat_order() {
        if state.ability_active(&holder) && state.has_ability(&holder, ability.id()) {
            for user in ability.day_action_users(state, &holder) {
                if !users.contains(&user) {
                    users.push(user);
                }
            }
        }
    }
    users
}

/// The token on a player who has claimed an ability (within its
/// `character::ClaimLimit`).
const CLAIMED: game::Effect = game::Effect::Custom("claimed");

/// Every ability with a day action on the script or in play, in that order.
fn day_abilities(state: &game::State) -> Vec<character::Ref> {
    let mut abilities: Vec<character::Ref> = Vec::new();
    let in_play = state
        .seat_order()
        .into_iter()
        .filter(|p| state.ability_active(p))
        .flat_map(|p| state.abilities(&p));
    for ability in state.script().iter().copied().chain(in_play) {
        if ability.day_action().is_some() && !abilities.iter().any(|a| character::same(*a, ability))
        {
            abilities.push(ability);
        }
    }
    abilities
}

/// The claimable day abilities (`character::Def::claim`) on the script or in
/// play: the same list for every living player, whatever their character,
/// so no phone shows who has which.
pub fn day_menu(state: &game::State) -> Vec<character::Ref> {
    day_abilities(state)
        .into_iter()
        .filter(|a| a.claim().is_some())
        .collect()
}

/// Every day action that may be used now, with who may use it. A claimable
/// ability (see [`day_menu`]) is offered to every living player within its
/// limit, whoever really has it; any other ability to its living users (see
/// [`day_action_users`]) who have not used it.
pub fn day_actions(state: &game::State) -> Vec<(player::Id, character::Ref)> {
    let mut actions = Vec::new();
    for ability in day_abilities(state) {
        let users: Vec<player::Id> = match ability.claim() {
            Some(claim) => state
                .alive_ids()
                .into_iter()
                .filter(|p| may_claim(state, p, ability, claim.limit))
                .collect(),
            None => day_action_users(state, ability)
                .into_iter()
                .filter(|u| state.is_alive(u) && !ability::used(state, u, ability))
                .collect(),
        };
        actions.extend(users.into_iter().map(|u| (u, ability)));
    }
    actions
}

/// Whether `player` may claim `ability` now, within `limit`.
fn may_claim(
    state: &game::State,
    player: &player::Id,
    ability: character::Ref,
    limit: character::ClaimLimit,
) -> bool {
    let allowed_now = match limit {
        character::ClaimLimit::FirstDay => state.phase() == game::Phase::Day(1),
        character::ClaimLimit::OncePerGame | character::ClaimLimit::EachDay => true,
    };
    allowed_now && !ability::has_mark(state, player, ability, player, CLAIMED)
}

/// `ability`'s targets (`character::Def::day_targets`) as `player` chooses
/// them now.
pub struct TargetChoice {
    pub prompt: &'static str,
    pub candidates: Vec<player::Id>,
    /// How many to choose: at most as many as there are candidates.
    pub count: usize,
}

/// Whom `player` may choose for `ability`'s targets now; `None` if it has
/// no targets.
pub fn target_choice(
    state: &game::State,
    player: &player::Id,
    ability: character::Ref,
) -> Option<TargetChoice> {
    let targets = ability.day_targets()?;
    let candidates = (targets.candidates)(state, player);
    let count = targets.count.min(candidates.len());
    Some(TargetChoice {
        prompt: targets.prompt,
        candidates,
        count,
    })
}

/// The player uses `ability`'s day action (the Slayer's shot), if it is one
/// of the [`day_actions`] they may use now, and is asked for its targets.
pub fn day_action(ctx: &mut ctx::Write<'_>, player: &player::Id, ability: character::Ref) {
    day_action_on(ctx, player, ability, &[]);
}

/// The player uses `ability`'s day action on `chosen` (picked on their phone
/// beforehand), if it is one of the [`day_actions`] they may use now; if
/// `chosen` are not legal targets, the player is asked for them.
///
/// The targets come first, then a claimable ability is used: in public, it
/// is announced ("{player} {announcement} {targets}"), works for a real user
/// and is a bluff for anyone else (the same questions, and the same "Nothing
/// happens" as a failed use); in private, the player is called over for the
/// whole visit (their answers are private), and anyone without the ability
/// (or with it spent) is asked the same and told `info::Message::Nothing`.
pub fn day_action_on(
    ctx: &mut ctx::Write<'_>,
    player: &player::Id,
    ability: character::Ref,
    chosen: &[player::Id],
) {
    let offered = day_actions(ctx.state())
        .iter()
        .any(|(p, a)| p == player && character::same(*a, ability));
    let Some(f) = ability.day_action().filter(|_| offered) else {
        return;
    };
    let claim = ability.claim();
    let private = claim.is_some_and(|c| c.visibility == character::Visibility::Private);
    if private {
        ctx.wake(player);
    }
    let targets = match target_choice(ctx.state(), player, ability) {
        Some(choice)
            if storyteller::is_selection(chosen, choice.count, |p| {
                choice.candidates.contains(p)
            }) =>
        {
            chosen.to_vec()
        }
        Some(choice) => ctx.ask(player, choice.prompt, &choice.candidates, choice.count),
        None => Vec::new(),
    };
    let Some(claim) = claim else {
        return f(ctx, player, &targets);
    };
    // Gone if the claimant dies, so a revived player (the Professor's) may
    // claim again, as a revived Slayer shoots again.
    let expiry = match claim.limit {
        character::ClaimLimit::EachDay => game::Expiry::Dusk,
        character::ClaimLimit::OncePerGame | character::ClaimLimit::FirstDay => game::Expiry::Death,
    };
    ctx.add_reminder(
        player,
        game::Reminder::new(ability, player, CLAIMED, expiry),
    );
    let real = day_action_users(ctx.state(), ability).contains(player);
    match claim.visibility {
        character::Visibility::Public {
            announcement,
            resolves_now,
        } => {
            let on: Vec<&str> = targets.iter().map(player::Id::as_str).collect();
            let announcement = if on.is_empty() {
                format!("{player} {announcement}")
            } else {
                format!("{player} {announcement} {}", on.join(", "))
            };
            ctx.announce(announcement);
            let before = ctx.state().history().len();
            if real {
                f(ctx, player, &targets);
            } else {
                (claim.bluff)(ctx, player, &targets);
            }
            let someone_died = ctx.state().history()[before..]
                .iter()
                .any(|e| matches!(e, event::Event::Died { .. }));
            if resolves_now && !someone_died && ctx.state().winner().is_none() {
                ctx.announce("Nothing happens");
            }
        }
        character::Visibility::Private => {
            // A holder whose once-per-game use is spent (offered again
            // after a revival) is answered like anyone without it.
            if real && !ability::used(ctx.state(), player, ability) {
                f(ctx, player, &targets);
            } else {
                (claim.bluff)(ctx, player, &targets);
                ctx.tell(player, &info::Message::Nothing);
            }
            ctx.sleep(player);
        }
    }
}

/// Whether madness is possible in this game: an ability that may make a
/// player mad (`character::Def::causes_madness`) is on the script or in play.
pub fn madness_possible(state: &game::State) -> bool {
    let in_play = state
        .seat_order()
        .into_iter()
        .flat_map(|p| state.abilities(&p));
    state
        .script()
        .iter()
        .copied()
        .chain(in_play)
        .any(|c| c.causes_madness())
}

/// Whether `player` may report breaking madness now: madness is possible, they
/// live and they have not reported it today.
pub fn may_report_madness(state: &game::State, player: &player::Id) -> bool {
    madness_possible(state) && state.is_alive(player) && !ctx::reported_madness(state, player)
}

/// `player` tells the storyteller they broke madness today (or, as the
/// Mutant, claimed to be an Outsider): the storyteller cannot hear the
/// table, so this is on their honour. Anyone may report it, mad or not, and
/// is told the same; it counts when madness is judged at the end of the day
/// (`check_madness`, `ctx::Read::is_mad_about`).
pub fn report_madness(ctx: &mut ctx::Write<'_>, player: &player::Id) {
    if !may_report_madness(ctx.state(), player) {
        return;
    }
    ctx.record(event::Event::MadnessReported(player.clone()));
    ctx.tell(player, &info::Message::Noted);
}

/// Whether `id` has `c`'s ability now and wakes for it tonight (not if a
/// `DoesNotWake` token is on them, as on the Demon the Exorcist chose).
fn wakes(state: &game::State, c: character::Ref, id: &player::Id) -> bool {
    state.has_ability(id, c.id())
        && c.wakes_tonight(state, id)
        && !state.has_effect(id, game::Effect::DoesNotWake)
}

/// Whether every ability of `player` lets them nominate (the Golem only once).
pub fn can_nominate(state: &game::State, player: &player::Id) -> bool {
    state
        .abilities(player)
        .iter()
        .all(|c| c.can_nominate(state, player))
}

/// End of day: active players' day-end abilities (the Mayor) fire, then the
/// Fabled and Loric dusk abilities.
pub fn end_day(ctx: &mut ctx::Write<'_>) {
    for (id, ability) in ctx.active_abilities() {
        if ctx.state().winner().is_some() {
            return;
        }
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
/// nominator's, then everyone's `on_nomination` (the Witch).
///
/// A nomination the nominator may not make ([`can_nominate`]) is refused.
pub fn nominate(ctx: &mut ctx::Write<'_>, nominator: &player::Id, nominee: &player::Id) {
    if !can_nominate(ctx.state(), nominator) {
        ctx.note(format!("{nominator} may not nominate"));
        return;
    }
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
    let everyone = ctx.state().seat_order();
    for (id, ability) in ctx.abilities_of(&everyone) {
        ability.on_nomination(ctx, &id, nominator, nominee);
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
    let voters = counted(ctx.state(), voters);
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

/// The hands that count: a hand that counts for nothing (a Beggar without a
/// token) is no vote.
fn counted(state: &game::State, voters: Vec<player::Id>) -> Vec<player::Id> {
    voters
        .into_iter()
        .filter(|v| weight(state, v) > 0)
        .collect()
}

/// The characters on the script, or in play, that let the town nominate the
/// Storyteller (`character::Def::storyteller_nominable`: the Atheist).
fn storyteller_rules(state: &game::State) -> Vec<character::Ref> {
    let mut rules: Vec<character::Ref> = Vec::new();
    let in_play = state.players().iter().map(|p| p.character);
    for c in state.script().iter().copied().chain(in_play) {
        if c.storyteller_nominable() && !rules.iter().any(|r| character::same(*r, c)) {
            rules.push(c);
        }
    }
    rules
}

/// Whether the Storyteller may be nominated in this game.
pub fn storyteller_nominable(state: &game::State) -> bool {
    !storyteller_rules(state).is_empty()
}

/// `nominator` nominates the Storyteller. Refused (noted, nothing happens)
/// unless the Storyteller is nominable and `nominator` may nominate. No
/// player is nominated, so nomination abilities do not react.
pub fn nominate_storyteller(ctx: &mut ctx::Write<'_>, nominator: &player::Id) {
    if !storyteller_nominable(ctx.state()) || !can_nominate(ctx.state(), nominator) {
        ctx.note(format!("{nominator} may not nominate the Storyteller"));
        return;
    }
    ctx.record(event::Event::StorytellerNominated {
        nominator: nominator.clone(),
    });
}

/// A vote on the Storyteller: counted like any vote ([`vote`]), but no
/// player is on trial, so `on_vote` abilities do not react. Returns the
/// votes counted.
pub fn vote_storyteller(ctx: &mut ctx::Write<'_>, voters: Vec<player::Id>, needed: usize) -> usize {
    let voters = counted(ctx.state(), voters);
    let votes = tally(ctx.state(), &voters);
    ctx.record(event::Event::StorytellerVoted {
        voters,
        votes,
        needed,
    });
    votes
}

/// The Storyteller is executed: every character that made them nominable
/// decides the game (the Atheist: good wins if an Atheist is in play).
pub fn execute_storyteller(ctx: &mut ctx::Write<'_>) {
    ctx.record(event::Event::StorytellerExecuted);
    for rule in storyteller_rules(ctx.state()) {
        if ctx.state().winner().is_some() {
            return;
        }
        rule.on_storyteller_executed(ctx);
    }
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
        if !ctx.state().is_alive(&player) || !ctx.broke_madness(&player, about) {
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

/// Votes needed to execute: half the living players, rounded up.
pub fn votes_needed(state: &game::State) -> usize {
    state.alive_ids().len().div_ceil(2)
}

/// Who is nominated: a player, or the Storyteller where a character allows
/// it ([`storyteller_nominable`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Nominee {
    Player(player::Id),
    Storyteller,
}

/// The day's block: the nominee with the most votes (at least the votes
/// needed), unless another nominee tied with them.
#[derive(Clone, Debug, Default)]
pub struct Block {
    best: Option<(Nominee, usize)>,
    tied: bool,
}

impl Block {
    /// A vote on `nominee` counted `votes` of the `needed`.
    pub fn add(&mut self, nominee: Nominee, votes: usize, needed: usize) {
        if votes < needed {
            return;
        }
        match &self.best {
            Some((_, top)) if votes < *top => {}
            Some((_, top)) if votes == *top => self.tied = true,
            _ => {
                self.best = Some((nominee, votes));
                self.tied = false;
            }
        }
    }

    /// Who is executed at the end of the day, if anyone.
    pub fn nominee(&self) -> Option<&Nominee> {
        self.best.as_ref().filter(|_| !self.tied).map(|(p, _)| p)
    }
}

/// What follows a nomination.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Nominated {
    /// The nomination ended the day (the Virgin executed the nominator) or the game.
    DayEnded,
    /// There is nothing to vote on: the nomination killed the nominee (the Golem).
    NoVote,
    /// The table votes now.
    Vote,
}

/// `nominator` nominates `nominee` (the storyteller does when `None`).
/// A nominator who died nominating (the Witch's curse) still nominated: the
/// vote goes on.
pub fn nominate_by(
    ctx: &mut ctx::Write<'_>,
    nominator: Option<&player::Id>,
    nominee: &player::Id,
) -> Nominated {
    let executed_before = ctx.state().executed_today().is_some();
    let alive_before = ctx.state().is_alive(nominee);
    match nominator {
        Some(nominator) => nominate(ctx, nominator, nominee),
        None => storyteller_nominates(ctx, nominee),
    }
    if ctx.state().winner().is_some()
        || (!executed_before && ctx.state().executed_today().is_some())
    {
        Nominated::DayEnded
    } else if alive_before && !ctx.state().is_alive(nominee) {
        Nominated::NoVote
    } else {
        Nominated::Vote
    }
}

/// Executes the block's nominee, if any, unless the game is over.
pub fn execute_block(ctx: &mut ctx::Write<'_>, block: &Block) {
    if ctx.state().winner().is_some() {
        return;
    }
    if let Some(nominee) = block.nominee().cloned() {
        execute(ctx, &nominee);
    }
}

/// Executes `nominee`: a player (`ctx::Write::execute`) or the Storyteller
/// ([`execute_storyteller`]).
pub fn execute(ctx: &mut ctx::Write<'_>, nominee: &Nominee) {
    match nominee {
        Nominee::Player(player) => {
            ctx.execute(player);
        }
        Nominee::Storyteller => execute_storyteller(ctx),
    }
}

/// The end of a day once executions are done: madness is judged, then the
/// day-end abilities (`end_day`), unless the game is over.
pub fn finish_day(ctx: &mut ctx::Write<'_>) {
    if ctx.state().winner().is_none() {
        check_madness(ctx);
    }
    if ctx.state().winner().is_none() {
        end_day(ctx);
    }
}
