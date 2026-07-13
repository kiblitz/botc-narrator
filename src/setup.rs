//! Bag-building: pick a legal role composition for a player count and assign
//! roles to seats. This is what a storyteller does before the first night, and
//! the automated storyteller needs it too.

use crate::grimoire::PlayerSpec;
use crate::ids::CharacterId;
use crate::interp::Rng;
use crate::registry::Registry;
use crate::role::{CharacterInfo, Kind};
use crate::script::Script;

/// The Trouble Brewing role counts `(townsfolk, outsider, minion, demon)` for a
/// given player count (5–15). Panics outside that range.
#[must_use]
pub fn tb_counts(players: usize) -> (usize, usize, usize, usize) {
    match players {
        5 => (3, 0, 1, 1),
        6 => (3, 1, 1, 1),
        7 => (5, 0, 1, 1),
        8 => (5, 1, 1, 1),
        9 => (5, 2, 1, 1),
        10 => (7, 0, 2, 1),
        11 => (7, 1, 2, 1),
        12 => (7, 2, 2, 1),
        13 => (9, 0, 3, 1),
        14 => (9, 1, 3, 1),
        15 => (9, 2, 3, 1),
        n => panic!("unsupported player count for Trouble Brewing: {n}"),
    }
}

fn roster_of_kind(reg: &Registry, script: &Script, kind: Kind) -> Vec<CharacterInfo> {
    script
        .roster
        .iter()
        .map(|&c| reg.info(c))
        .filter(|info| info.kind == kind)
        .collect()
}

fn sample(pool: &[CharacterInfo], k: usize, rng: &mut Rng) -> Vec<CharacterInfo> {
    let mut pool = pool.to_vec();
    let mut out = Vec::with_capacity(k);
    for _ in 0..k {
        if pool.is_empty() {
            break;
        }
        out.push(pool.remove(rng.below(pool.len())));
    }
    out
}

/// Build a random legal Trouble Brewing game for the given player names.
///
/// Handles the Baron (which converts two Townsfolk into two Outsiders) and the
/// Drunk (who is seated as the Drunk but believes they are a random Townsfolk
/// not otherwise in play). Seating is shuffled.
#[must_use]
pub fn random_trouble_brewing(
    reg: &Registry,
    script: &Script,
    names: &[String],
    rng: &mut Rng,
) -> Vec<PlayerSpec> {
    let (mut tf, mut out, min, dem) = tb_counts(names.len());

    let townsfolk = roster_of_kind(reg, script, Kind::Townsfolk);
    let outsiders = roster_of_kind(reg, script, Kind::Outsider);
    let minions = roster_of_kind(reg, script, Kind::Minion);
    let demons = roster_of_kind(reg, script, Kind::Demon);

    // Minions first, so the Baron can adjust the Townsfolk/Outsider split.
    let chosen_minions = sample(&minions, min, rng);
    let baron = CharacterId("baron");
    if chosen_minions.iter().any(|c| c.id == baron) {
        let shift = 2.min(tf);
        tf -= shift;
        out += shift;
    }

    let chosen_demons = sample(&demons, dem, rng);
    let chosen_townsfolk = sample(&townsfolk, tf, rng);
    let chosen_outsiders = sample(&outsiders, out, rng);

    // A believed-Townsfolk pool for a possible Drunk: any Townsfolk not in play.
    let in_play: Vec<CharacterId> = chosen_townsfolk.iter().map(|c| c.id).collect();
    let mut believed_pool: Vec<CharacterInfo> = townsfolk
        .iter()
        .copied()
        .filter(|c| !in_play.contains(&c.id))
        .collect();

    // Assemble (role, believed) pairs.
    let drunk = CharacterId("drunk");
    let mut roles: Vec<(CharacterInfo, Option<CharacterInfo>)> = Vec::with_capacity(names.len());
    for info in chosen_townsfolk
        .into_iter()
        .chain(chosen_outsiders)
        .chain(chosen_minions)
        .chain(chosen_demons)
    {
        if info.id == drunk && !believed_pool.is_empty() {
            let believed = believed_pool.remove(rng.below(believed_pool.len()));
            roles.push((info, Some(believed)));
        } else {
            roles.push((info, None));
        }
    }

    // Shuffle seating (Fisher–Yates).
    for i in (1..roles.len()).rev() {
        roles.swap(i, rng.below(i + 1));
    }

    names
        .iter()
        .zip(roles)
        .map(|(name, (role, believed))| {
            let spec = PlayerSpec::new(name.clone(), role);
            match believed {
                Some(b) => spec.believing(b),
                None => spec,
            }
        })
        .collect()
}
