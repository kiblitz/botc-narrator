//! Setup counts and choosing a legal set of characters for a game.
//!
//! The base counts depend only on the number of (non-Traveller) players.
//! Characters adjust them through `character::Def::setup_counts` (the Baron
//! adds two Outsiders), so a setup is built by picking characters and
//! re-applying every chosen character's adjustment until the picks match.

use rand::Rng;
use rand::seq::IndexedRandom;

use crate::character;
use crate::script;

pub const MIN_PLAYERS: usize = 5;
pub const MAX_PLAYERS: usize = 15;

/// How many characters of each kind are in play.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Counts {
    pub townsfolk: usize,
    pub outsiders: usize,
    pub minions: usize,
    pub demons: usize,
}

/// Kinds in the order a setup fills them.
const KINDS: [character::Kind; 4] = [
    character::Kind::Demon,
    character::Kind::Minion,
    character::Kind::Outsider,
    character::Kind::Townsfolk,
];

/// Re-applying adjustments stops after this many rounds even if picks keep
/// changing (adjustments that undo each other).
const MAX_ROUNDS: usize = 32;

impl Counts {
    /// The standard counts for 5 to 15 players.
    pub fn base(players: usize) -> Self {
        let players = players.clamp(MIN_PLAYERS, MAX_PLAYERS);
        let (townsfolk, outsiders, minions) = match players {
            5 => (3, 0, 1),
            6 => (3, 1, 1),
            n => {
                let extra = n - 7;
                (5 + 2 * (extra / 3), extra % 3, 1 + extra / 3)
            }
        };
        Self {
            townsfolk,
            outsiders,
            minions,
            demons: 1,
        }
    }

    pub fn get(&self, kind: character::Kind) -> usize {
        match kind {
            character::Kind::Townsfolk => self.townsfolk,
            character::Kind::Outsider => self.outsiders,
            character::Kind::Minion => self.minions,
            character::Kind::Demon => self.demons,
            character::Kind::Traveller | character::Kind::Fabled => 0,
        }
    }

    /// These counts with `kind` set to `n` (Riot: no Minions, more Demons).
    pub fn with(self, kind: character::Kind, n: usize) -> Self {
        let mut counts = self;
        match kind {
            character::Kind::Townsfolk => counts.townsfolk = n,
            character::Kind::Outsider => counts.outsiders = n,
            character::Kind::Minion => counts.minions = n,
            character::Kind::Demon => counts.demons = n,
            character::Kind::Traveller | character::Kind::Fabled => {
                panic!("{kind} is not part of the setup counts")
            }
        }
        counts
    }

    /// Move up to `n` slots from Townsfolk to Outsiders (negative `n` moves
    /// them back), as the Baron (+2) or Vigormortis (-1) do.
    pub fn shift_outsiders(self, n: isize) -> Self {
        let mut counts = self;
        if n >= 0 {
            let n = n.unsigned_abs().min(counts.townsfolk);
            counts.townsfolk -= n;
            counts.outsiders += n;
        } else {
            let n = n.unsigned_abs().min(counts.outsiders);
            counts.outsiders -= n;
            counts.townsfolk += n;
        }
        counts
    }

    pub fn total(&self) -> usize {
        self.townsfolk + self.outsiders + self.minions + self.demons
    }
}

/// A chosen character and which of its `setup_counts` options applies.
#[derive(Clone, Copy)]
struct Pick {
    character: character::Ref,
    option: u32,
}

/// A legal set of `players` characters from `script` (unshuffled): picks
/// characters at random and applies their setup adjustments until the picks
/// match the adjusted counts. Kinds that run out of characters are filled
/// with Townsfolk.
pub fn choose(rng: &mut impl Rng, players: usize, script: &script::Script) -> Vec<character::Ref> {
    let players = players.clamp(MIN_PLAYERS, MAX_PLAYERS);
    let base = Counts::base(players);
    let mut picks: Vec<Pick> = Vec::new();
    for _ in 0..MAX_ROUNDS {
        let counts = adjusted(base, &picks);
        let mut changed = false;
        let mut shortfall = 0;
        for kind in KINDS {
            let mut want = counts.get(kind);
            if kind == character::Kind::Townsfolk {
                want += shortfall;
            }
            let have = picks.iter().filter(|p| p.character.kind() == kind).count();
            if have > want {
                remove(&mut picks, base, kind, have - want);
                changed = true;
            }
            for _ in have..want {
                match next(rng, script, &picks, kind) {
                    Some(character) => {
                        picks.push(Pick {
                            character,
                            option: rng.random(),
                        });
                        changed = true;
                    }
                    None => shortfall += 1,
                }
            }
            // Re-apply adjustments before filling the next kind.
            if changed {
                break;
            }
        }
        if !changed {
            break;
        }
    }
    picks.into_iter().map(|p| p.character).collect()
}

/// `base` with every pick's chosen adjustment applied in pick order.
fn adjusted(base: Counts, picks: &[Pick]) -> Counts {
    picks.iter().fold(base, |counts, pick| {
        let options = pick.character.setup_counts(counts);
        options
            .get(pick.option as usize % options.len().max(1))
            .copied()
            .unwrap_or(counts)
    })
}

/// A not-yet-picked character of `kind`, or a copy of a picked one that
/// duplicates in setup.
fn next(
    rng: &mut impl Rng,
    script: &script::Script,
    picks: &[Pick],
    kind: character::Kind,
) -> Option<character::Ref> {
    let unused: Vec<character::Ref> = script
        .characters
        .iter()
        .copied()
        .filter(|c| c.kind() == kind && !picks.iter().any(|p| character::same(p.character, *c)))
        .collect();
    if let Some(c) = unused.choose(rng) {
        return Some(*c);
    }
    picks
        .iter()
        .map(|p| p.character)
        .find(|c| c.kind() == kind && c.duplicates_in_setup())
}

/// Removes `n` picks of `kind`, the most recent first, preferring picks
/// whose removal leaves the counts unchanged (so an adjustment does not remove
/// its own cause).
fn remove(picks: &mut Vec<Pick>, base: Counts, kind: character::Kind, n: usize) {
    for _ in 0..n {
        let counts = adjusted(base, picks);
        let keeps_counts = |i: usize| {
            let mut rest = picks.clone();
            rest.remove(i);
            adjusted(base, &rest) == counts
        };
        let of_kind: Vec<usize> = (0..picks.len())
            .rev()
            .filter(|&i| picks[i].character.kind() == kind)
            .collect();
        let index = of_kind
            .iter()
            .copied()
            .find(|&i| keeps_counts(i))
            .or(of_kind.first().copied());
        if let Some(i) = index {
            picks.remove(i);
        }
    }
}
