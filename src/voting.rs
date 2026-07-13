//! Vote tallying for a nomination.
//!
//! This is a pure, read-only tally over the grimoire: given who raised their
//! hand, it works out whose votes actually count and whether the nominee
//! reaches the execution threshold. It respects three rules generically:
//!
//! * living players have one vote;
//! * dead players have a single ghost vote (validity checked here; spending it
//!   is the caller's job so the tally stays side-effect free);
//! * a voting-restricted player (the Butler, via its `Master` token) only
//!   counts when their master also voted — the tally never names the Butler.

use crate::grimoire::Grimoire;
use crate::ids::PlayerId;

/// The outcome of tallying one nomination.
#[derive(Debug, Clone)]
pub struct VoteResult {
    pub nominee: PlayerId,
    /// Players whose votes counted, in the order supplied.
    pub valid_voters: Vec<PlayerId>,
    /// Number of counting votes.
    pub votes: usize,
    /// Votes required to put the nominee on the block (at least half the living
    /// players, rounded up).
    pub threshold: usize,
}

impl VoteResult {
    #[must_use]
    pub fn reached(&self) -> bool {
        self.votes >= self.threshold
    }

    /// Dead players among the valid voters — those whose ghost vote is now spent.
    pub fn ghost_voters<'a>(&'a self, grim: &'a Grimoire) -> impl Iterator<Item = PlayerId> + 'a {
        self.valid_voters
            .iter()
            .copied()
            .filter(move |&v| !grim.is_alive(v))
    }
}

/// The votes required to nominate for execution: at least half the living
/// players, rounded up.
#[must_use]
pub fn threshold(alive_count: usize) -> usize {
    alive_count.div_ceil(2)
}

/// Tally a nomination. `raised_hands` is who voted "yes" (duplicates ignored).
#[must_use]
pub fn tally(grim: &Grimoire, nominee: PlayerId, raised_hands: &[PlayerId]) -> VoteResult {
    let mut seen = Vec::new();
    let mut valid_voters = Vec::new();
    for &voter in raised_hands {
        if seen.contains(&voter) {
            continue;
        }
        seen.push(voter);
        if vote_counts(grim, voter, raised_hands) {
            valid_voters.push(voter);
        }
    }
    VoteResult {
        nominee,
        votes: valid_voters.len(),
        threshold: threshold(grim.alive_count()),
        valid_voters,
    }
}

fn vote_counts(grim: &Grimoire, voter: PlayerId, raised_hands: &[PlayerId]) -> bool {
    // Must have a vote to spend.
    let has_vote = grim.is_alive(voter) || grim.has_ghost_vote(voter);
    if !has_vote {
        return false;
    }
    // A master-restricted voter only counts if their master voted too.
    if let Some(master) = grim.master_of(voter) {
        if !raised_hands.contains(&master) {
            return false;
        }
    }
    true
}
