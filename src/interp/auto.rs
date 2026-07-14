//! The automated storyteller.
//!
//! In multiplayer with no human running the game, the computer must play the
//! storyteller. That job splits cleanly across the [`Storyteller`] trait:
//!
//! * **`ask`** is a *player's* decision (who to poison, who to kill). It is
//!   delegated to a [`PlayerAgent`] — a network-backed one per human, or a
//!   random one for simulation.
//! * **`choose`** is the *storyteller's* discretionary call (false info under
//!   poison, demon bluffs, Recluse/Spy misregistration). An automated
//!   storyteller decides it by a [`DiscretionPolicy`].
//! * **`wake`/`sleep`/`reveal`** are private, per-player events forwarded to
//!   that seat's agent.

use crate::ids::PlayerId;
use crate::storyteller::{Candidate, Storyteller};

/// A tiny deterministic PRNG (xorshift64*), so simulated games and automated
/// decisions are reproducible from a seed without pulling in a dependency.
#[derive(Debug, Clone)]
pub struct Rng {
    state: u64,
}

impl Rng {
    #[must_use]
    pub fn new(seed: u64) -> Self {
        // Avoid the all-zero state, which xorshift cannot escape.
        Rng {
            state: seed ^ 0x9E37_79B9_7F4A_7C15 | 1,
        }
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// A uniform index in `0..n` (returns 0 if `n == 0`).
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % n as u64) as usize
        }
    }
}

/// Supplies the decision a *player* makes when the storyteller asks them to
/// point at someone, and receives their private night events.
pub trait PlayerAgent {
    /// Player `who` is asked `prompt`; return one of the candidates' ids.
    fn decide(&mut self, who: PlayerId, prompt: &str, options: &[Candidate]) -> PlayerId;

    fn on_wake(&mut self, _who: PlayerId) {}
    fn on_sleep(&mut self, _who: PlayerId) {}
    fn on_reveal(&mut self, _who: PlayerId, _message: &str) {}
}

/// Supplies the storyteller's discretionary choices.
pub trait DiscretionPolicy {
    fn choose(&mut self, prompt: &str, options: &[String]) -> usize;
}

/// A [`PlayerAgent`] that points at a uniformly random candidate — used to
/// simulate players.
#[derive(Debug, Clone)]
pub struct RandomAgent {
    rng: Rng,
}

impl RandomAgent {
    #[must_use]
    pub fn new(seed: u64) -> Self {
        RandomAgent {
            rng: Rng::new(seed),
        }
    }
}

impl PlayerAgent for RandomAgent {
    fn decide(&mut self, _who: PlayerId, _prompt: &str, options: &[Candidate]) -> PlayerId {
        options[self.rng.below(options.len())].id
    }
}

/// A [`DiscretionPolicy`] that makes each storyteller call at random. A smarter
/// automated storyteller would weigh choices to balance the game; random is a
/// correct, if unsubtle, baseline.
#[derive(Debug, Clone)]
pub struct RandomDiscretion {
    rng: Rng,
}

impl RandomDiscretion {
    #[must_use]
    pub fn new(seed: u64) -> Self {
        RandomDiscretion {
            rng: Rng::new(seed),
        }
    }
}

impl DiscretionPolicy for RandomDiscretion {
    fn choose(&mut self, _prompt: &str, options: &[String]) -> usize {
        self.rng.below(options.len())
    }
}

/// The automated storyteller: players' `ask`s go to `agent`, discretionary
/// `choose`s go to `discretion`, and log lines are collected.
pub struct AutoStoryteller<A: PlayerAgent, D: DiscretionPolicy> {
    pub agent: A,
    pub discretion: D,
    log: Vec<String>,
}

impl<A: PlayerAgent, D: DiscretionPolicy> AutoStoryteller<A, D> {
    pub fn new(agent: A, discretion: D) -> Self {
        AutoStoryteller {
            agent,
            discretion,
            log: Vec::new(),
        }
    }

    #[must_use]
    pub fn log(&self) -> &[String] {
        &self.log
    }
}

impl AutoStoryteller<RandomAgent, RandomDiscretion> {
    /// A fully random automated storyteller — random players, random discretion
    /// — seeded reproducibly. Handy for simulation and tests.
    #[must_use]
    pub fn random(seed: u64) -> Self {
        AutoStoryteller::new(
            RandomAgent::new(seed),
            RandomDiscretion::new(seed ^ 0xD1B5_4A32_D192_ED03),
        )
    }
}

impl<A: PlayerAgent, D: DiscretionPolicy> Storyteller for AutoStoryteller<A, D> {
    fn wake(&mut self, who_id: PlayerId, _who: &str) {
        self.agent.on_wake(who_id);
    }

    fn sleep(&mut self, who_id: PlayerId, _who: &str) {
        self.agent.on_sleep(who_id);
    }

    fn reveal(&mut self, who_id: PlayerId, _who: &str, message: &str) {
        self.agent.on_reveal(who_id, message);
    }

    fn ask(
        &mut self,
        who_id: PlayerId,
        _who: &str,
        prompt: &str,
        options: &[Candidate],
    ) -> PlayerId {
        self.agent.decide(who_id, prompt, options)
    }

    fn choose(&mut self, prompt: &str, options: &[String]) -> usize {
        self.discretion.choose(prompt, options)
    }

    fn log(&mut self, message: &str) {
        self.log.push(message.to_string());
    }
}
