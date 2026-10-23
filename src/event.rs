use std::fmt;

use crate::character;
use crate::characters;
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
        /// `None` when the storyteller nominates (the Bishop).
        nominator: Option<player::Id>,
    },
    Executed(player::Id),
    Died {
        player: player::Id,
        cause: Cause,
    },
    /// A dead player comes back to life (the Shabaloth, the Al-Hadikhia).
    Revived(player::Id),
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
    AlignmentChanged {
        player: player::Id,
        alignment: character::Alignment,
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
    ReminderRemoved {
        player: player::Id,
        reminder: game::Reminder,
    },
    FabledMarked {
        fabled: &'static str,
        effect: game::Effect,
    },
    SeatsSwapped(player::Id, player::Id),
    GhostVoteSpent(player::Id),
    /// A player publicly claimed that these players are these characters
    /// (the Juggler's guesses, the Gossip's statement).
    Claimed {
        player: player::Id,
        /// The id of the character whose ability the claim was made with.
        ability: &'static str,
        claims: Vec<(player::Id, &'static str)>,
    },
    Voted {
        nominee: player::Id,
        voters: Vec<player::Id>,
        /// The votes counted, after vote weights (Bureaucrat, Beggar).
        votes: usize,
        needed: usize,
    },
    Note(String),
    /// Something the storyteller tells everyone ("the Leviathan is in play").
    Announced(String),
    Won(character::Alignment),
}

impl Event {
    /// Players the event names, in order.
    pub fn players(&self) -> Vec<&player::Id> {
        match self {
            Event::Nominated { nominee, nominator } => {
                nominator.iter().chain(std::iter::once(nominee)).collect()
            }
            Event::Died { player, cause } => match cause {
                Cause::Ability { by } => vec![player, by],
                Cause::Demon | Cause::Starpass | Cause::Execution => vec![player],
            },
            Event::Executed(player)
            | Event::Revived(player)
            | Event::GhostVoteSpent(player)
            | Event::KillPrevented { player, .. }
            | Event::Transformed { player, .. }
            | Event::Believes { player, .. }
            | Event::AbilityGained { player, .. }
            | Event::AbilityLost { player, .. }
            | Event::AlignmentChanged { player, .. }
            | Event::MadnessBroken { player, .. } => vec![player],
            Event::ReminderPlaced { player, reminder }
            | Event::ReminderRemoved { player, reminder } => {
                if reminder.placed_by == *player {
                    vec![player]
                } else {
                    vec![&reminder.placed_by, player]
                }
            }
            Event::KillRedirected { from, to } => vec![from, to],
            Event::SeatsSwapped(a, b) => vec![a, b],
            Event::Claimed { player, claims, .. } => std::iter::once(player)
                .chain(claims.iter().map(|(p, _)| p))
                .collect(),
            Event::Voted {
                nominee, voters, ..
            } => std::iter::once(nominee).chain(voters.iter()).collect(),
            Event::PhaseBegan(_)
            | Event::FabledAdded(_)
            | Event::FabledMarked { .. }
            | Event::Note(_)
            | Event::Announced(_)
            | Event::Won(_) => Vec::new(),
        }
    }

    /// Character names the event names, in order.
    pub fn characters(&self) -> Vec<&'static str> {
        match self {
            Event::Transformed { into: c, .. }
            | Event::Believes { character: c, .. }
            | Event::AbilityGained { ability: c, .. }
            | Event::AbilityLost { ability: c, .. }
            | Event::MadnessBroken { about: c, .. }
            | Event::FabledAdded(c)
            | Event::FabledMarked { fabled: c, .. } => vec![*c],
            Event::ReminderPlaced { reminder, .. } | Event::ReminderRemoved { reminder, .. } => {
                // The ability that placed the token, then any character the
                // token itself names (a madness, or a token named after a
                // character, like the Cannibal's gained ability).
                let mut names: Vec<&'static str> = characters::find(reminder.source)
                    .map(|c| c.name())
                    .into_iter()
                    .collect();
                let named = match reminder.effect {
                    game::Effect::Mad(c) => Some(c),
                    game::Effect::Custom(token) => characters::all()
                        .iter()
                        .map(|c| c.name())
                        .find(|name| *name == token),
                    _ => None,
                };
                names.extend(named.filter(|n| !names.contains(n)));
                names
            }
            Event::Claimed { claims, .. } => claims.iter().map(|(_, c)| *c).collect(),
            Event::PhaseBegan(_)
            | Event::Nominated { .. }
            | Event::Executed(_)
            | Event::Died { .. }
            | Event::Revived(_)
            | Event::KillPrevented { .. }
            | Event::KillRedirected { .. }
            | Event::AlignmentChanged { .. }
            | Event::SeatsSwapped(..)
            | Event::GhostVoteSpent(_)
            | Event::Voted { .. }
            | Event::Note(_)
            | Event::Announced(_)
            | Event::Won(_) => Vec::new(),
        }
    }
}

impl fmt::Display for Event {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Event::PhaseBegan(phase) => write!(f, "=== {phase} ==="),
            Event::Nominated {
                nominee,
                nominator: Some(nominator),
            } => write!(f, "{nominator} nominates {nominee}"),
            Event::Nominated {
                nominee,
                nominator: None,
            } => write!(f, "the storyteller nominates {nominee}"),
            Event::Executed(p) => write!(f, "{p} is executed"),
            Event::Died { player, cause } => write!(f, "{player} dies ({cause:?})"),
            Event::Revived(player) => write!(f, "{player} is alive again"),
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
            Event::AlignmentChanged { player, alignment } => {
                write!(f, "{player} is now {alignment}")
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
            Event::ReminderRemoved { player, reminder } => write!(
                f,
                "{} ({}) removes {}'s {} token",
                reminder.placed_by, reminder.source, player, reminder.effect
            ),
            Event::FabledMarked { fabled, effect } => write!(f, "the {fabled} is marked: {effect}"),
            Event::SeatsSwapped(a, b) => write!(f, "{a} and {b} swap seats"),
            Event::GhostVoteSpent(p) => write!(f, "{p} gives away their vote token"),
            Event::Claimed { player, claims, .. } => {
                let claims: Vec<String> = claims
                    .iter()
                    .map(|(p, c)| format!("{p} is the {c}"))
                    .collect();
                write!(f, "{player} claims: {}", claims.join(", "))
            }
            Event::Voted {
                nominee,
                votes,
                needed,
                ..
            } => write!(f, "{votes} votes for {nominee} ({needed} needed)"),
            Event::Note(text) => f.write_str(text),
            Event::Announced(text) => write!(f, "Announcement: {text}"),
            Event::Won(alignment) => write!(f, "{alignment} wins!"),
        }
    }
}
