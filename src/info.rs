use std::fmt;

use crate::player;

/// Information the storyteller gives a player. Typed so engines can render it
/// and storytellers can reason about which values are true.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Message {
    Count {
        count: usize,
        noun: &'static str,
    },
    YesNo(bool),
    OneOf {
        players: [player::Id; 2],
        character: &'static str,
    },
    Character(&'static str),
    Team {
        evil: Vec<player::Id>,
    },
    Bluffs(Vec<&'static str>),
    Grimoire(Vec<(player::Id, &'static str)>),
}

impl Message {
    /// Players the message names, in order.
    pub fn players(&self) -> Vec<&player::Id> {
        match self {
            Message::OneOf { players, .. } => players.iter().collect(),
            Message::Team { evil } => evil.iter().collect(),
            Message::Grimoire(seats) => seats.iter().map(|(p, _)| p).collect(),
            Message::Count { .. }
            | Message::YesNo(_)
            | Message::Character(_)
            | Message::Bluffs(_) => Vec::new(),
        }
    }

    /// Character names the message names, in order.
    pub fn characters(&self) -> Vec<&'static str> {
        match self {
            Message::OneOf { character, .. } | Message::Character(character) => vec![*character],
            Message::Bluffs(names) => names.clone(),
            Message::Grimoire(seats) => seats.iter().map(|(_, c)| *c).collect(),
            Message::Count { .. } | Message::YesNo(_) | Message::Team { .. } => Vec::new(),
        }
    }
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Message::Count { count, noun } => write!(f, "{count} {noun}"),
            Message::YesNo(true) => f.write_str("Yes"),
            Message::YesNo(false) => f.write_str("No"),
            Message::OneOf {
                players: [a, b],
                character,
            } => {
                write!(f, "{a} and {b}: one is the {character}")
            }
            Message::Character(name) => f.write_str(name),
            Message::Team { evil } => write!(f, "Evil: {}", join(evil)),
            Message::Bluffs(names) => write!(f, "Bluffs: {}", join(names)),
            Message::Grimoire(seats) => {
                let seats: Vec<String> = seats.iter().map(|(p, c)| format!("{p}={c}")).collect();
                f.write_str(&seats.join(", "))
            }
        }
    }
}

fn join<T: fmt::Display>(items: &[T]) -> String {
    items
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ")
}
