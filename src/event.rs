use std::fmt;

use crate::character;
use crate::game;
use crate::player;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Cause {
    Demon,
    Starpass,
    Execution,
    Ability { by: player::Id },
}

/// Something that happened in the game, kept as the game's history.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    PhaseBegan(game::Phase),
    Nominated {
        nominee: player::Id,
        nominator: player::Id,
    },
    Executed(player::Id),
    Died {
        player: player::Id,
        cause: Cause,
    },
    KillPrevented {
        player: player::Id,
        by: &'static str,
    },
    KillRedirected {
        from: player::Id,
        to: player::Id,
    },
    Transformed {
        player: player::Id,
        into: &'static str,
    },
    /// The player is told they are `character`, which they are not.
    Believes {
        player: player::Id,
        character: &'static str,
    },
    AbilityGained {
        player: player::Id,
        ability: &'static str,
    },
    AbilityLost {
        player: player::Id,
        ability: &'static str,
    },
    FabledAdded(&'static str),
    /// The storyteller judged that a mad player broke their madness.
    MadnessBroken {
        player: player::Id,
        about: &'static str,
    },
    ReminderPlaced {
        player: player::Id,
        reminder: game::Reminder,
    },
    Voted {
        nominee: player::Id,
        voters: Vec<player::Id>,
        needed: usize,
    },
    Note(String),
    Won(character::Alignment),
}

impl fmt::Display for Event {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Event::PhaseBegan(phase) => write!(f, "=== {phase} ==="),
            Event::Nominated { nominee, nominator } => write!(f, "{nominator} nominates {nominee}"),
            Event::Executed(p) => write!(f, "{p} is executed"),
            Event::Died { player, cause } => write!(f, "{player} dies ({cause:?})"),
            Event::KillPrevented { player, by } => write!(f, "{player} survives ({by})"),
            Event::KillRedirected { from, to } => write!(f, "kill on {from} redirected to {to}"),
            Event::Transformed { player, into } => write!(f, "{player} becomes the {into}"),
            Event::Believes { player, character } => {
                write!(f, "{player} believes they are the {character}")
            }
            Event::AbilityGained { player, ability } => {
                write!(f, "{player} gains the {ability} ability")
            }
            Event::AbilityLost { player, ability } => {
                write!(f, "{player} loses the {ability} ability")
            }
            Event::FabledAdded(name) => write!(f, "the {name} is in play"),
            Event::MadnessBroken { player, about } => {
                write!(f, "{player} broke madness about the {about}")
            }
            Event::ReminderPlaced { player, reminder } => write!(
                f,
                "{} ({}) marks {player}: {}",
                reminder.placed_by, reminder.source, reminder.effect
            ),
            Event::Voted {
                nominee,
                voters,
                needed,
            } => write!(f, "{} votes for {nominee} ({needed} needed)", voters.len()),
            Event::Note(text) => f.write_str(text),
            Event::Won(alignment) => write!(f, "{alignment} wins!"),
        }
    }
}
