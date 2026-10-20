//! Storyteller discretion: every choice the rules leave to the storyteller is a
//! [`Decision`] with only legal options, answered by a swappable [`Strategy`].

use std::collections::VecDeque;
use std::fmt;

use rand::SeedableRng;
use rand::seq::index;

use crate::character;
use crate::game;
use crate::info;
use crate::player;

pub mod claude;

/// Why the storyteller is being asked to choose.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reason {
    /// Which information to give a player. When `malfunctioning` (poisoned or
    /// drunk), false options are also legal.
    Information {
        player: player::Id,
        character: &'static str,
        malfunctioning: bool,
    },
    DemonBluffs,
    /// Which minion becomes the demon after the demon kills themself.
    StarpassTarget,
    /// Who dies instead when the demon attacks the Mayor (the Mayor is an option).
    MayorRedirect {
        mayor: player::Id,
    },
    /// How `target` registers to `asker`'s ability: the first option is the
    /// truth, the others are misregistrations (the Recluse, the Spy).
    Registration {
        asker: player::Id,
        target: player::Id,
    },
    /// Any other choice a character's ability leaves to the storyteller:
    /// `question` says what is being decided.
    Ability {
        player: player::Id,
        character: &'static str,
        question: &'static str,
    },
    /// A choice a Fabled or Loric leaves to the storyteller.
    Fabled {
        character: &'static str,
        question: &'static str,
    },
}

impl fmt::Display for Reason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Reason::Information {
                player,
                character,
                malfunctioning,
            } => {
                write!(f, "information for {player} ({character})")?;
                if *malfunctioning {
                    f.write_str(", who is poisoned or drunk: false info is allowed")?;
                }
                Ok(())
            }
            Reason::DemonBluffs => f.write_str("demon bluffs (not-in-play good characters)"),
            Reason::StarpassTarget => f.write_str("which minion becomes the new demon"),
            Reason::MayorRedirect { mayor } => {
                write!(f, "the demon attacked Mayor {mayor}: who dies instead")
            }
            Reason::Registration { asker, target } => {
                write!(
                    f,
                    "how {target} registers to {asker} (first option is true)"
                )
            }
            Reason::Ability {
                player,
                character,
                question,
            } => write!(f, "{character} {player}: {question}"),
            Reason::Fabled {
                character,
                question,
            } => write!(f, "{character}: {question}"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Choice {
    Player(player::Id),
    Character(&'static str),
    Info {
        message: info::Message,
        truthful: bool,
    },
    Alignment(character::Alignment),
    YesNo(bool),
    /// Registers as some kind other than this one.
    NotOfKind(character::Kind),
}

impl Choice {
    /// Players this option names.
    pub fn players(&self) -> Vec<&player::Id> {
        match self {
            Choice::Player(p) => vec![p],
            Choice::Character(_)
            | Choice::Alignment(_)
            | Choice::YesNo(_)
            | Choice::NotOfKind(_) => Vec::new(),
            Choice::Info { message, .. } => message.players(),
        }
    }

    /// Characters this option names.
    pub fn characters(&self) -> Vec<&'static str> {
        match self {
            Choice::Player(_) | Choice::Alignment(_) | Choice::YesNo(_) | Choice::NotOfKind(_) => {
                Vec::new()
            }
            Choice::Character(name) => vec![*name],
            Choice::Info { message, .. } => message.characters(),
        }
    }
}

impl fmt::Display for Choice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Choice::Player(p) => write!(f, "{p}"),
            Choice::Character(name) => f.write_str(name),
            Choice::Alignment(alignment) => write!(f, "{alignment}"),
            Choice::YesNo(true) => f.write_str("Yes"),
            Choice::YesNo(false) => f.write_str("No"),
            Choice::NotOfKind(kind) => match kind {
                character::Kind::Outsider => write!(f, "not an {kind}"),
                _ => write!(f, "not a {kind}"),
            },
            Choice::Info { message, truthful } => {
                write!(f, "\"{message}\"")?;
                if !truthful {
                    f.write_str(" (false)")?;
                }
                Ok(())
            }
        }
    }
}

/// Choose exactly `count` distinct options.
#[derive(Clone, Debug)]
pub struct Decision {
    pub reason: Reason,
    pub options: Vec<Choice>,
    pub count: usize,
}

impl Decision {
    pub fn is_valid(&self, picks: &[usize]) -> bool {
        is_selection(picks, self.count, |&i| i < self.options.len())
    }
}

/// Exactly `count` distinct items, each `allowed`.
pub fn is_selection<T: PartialEq>(items: &[T], count: usize, allowed: impl Fn(&T) -> bool) -> bool {
    items.len() == count
        && items.iter().all(allowed)
        && (1..items.len()).all(|i| !items[..i].contains(&items[i]))
}

/// Makes storyteller decisions. Answers are validated by the engine; an
/// invalid answer falls back to the first `count` options.
pub trait Strategy {
    fn decide(&mut self, decision: &Decision, view: &game::State) -> Vec<usize>;
}

/// Uniformly random legal choices.
pub struct Random {
    rng: rand::rngs::StdRng,
}

impl Random {
    pub fn seeded(seed: u64) -> Self {
        Self {
            rng: rand::rngs::StdRng::seed_from_u64(seed),
        }
    }

    pub fn from_entropy() -> Self {
        Self {
            rng: rand::rngs::StdRng::from_os_rng(),
        }
    }
}

impl Strategy for Random {
    fn decide(&mut self, decision: &Decision, _view: &game::State) -> Vec<usize> {
        index::sample(&mut self.rng, decision.options.len(), decision.count).into_vec()
    }
}

/// Deterministic choices for tests: queued answers in order, then the first
/// `count` options once the queue is empty.
#[derive(Default)]
pub struct Scripted {
    picks: VecDeque<Vec<usize>>,
}

impl Scripted {
    pub fn new(picks: impl IntoIterator<Item = Vec<usize>>) -> Self {
        Self {
            picks: picks.into_iter().collect(),
        }
    }
}

impl Strategy for Scripted {
    fn decide(&mut self, decision: &Decision, _view: &game::State) -> Vec<usize> {
        self.picks
            .pop_front()
            .unwrap_or_else(|| (0..decision.count).collect())
    }
}
