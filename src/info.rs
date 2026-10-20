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
