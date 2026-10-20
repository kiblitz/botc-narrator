use std::fmt;

use crate::character;
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
    /// Several characters, e.g. which Outsiders are in play.
    Characters(Vec<&'static str>),
    /// "`count` of these players are `noun`", e.g. 1 of 3 is evil.
    Group {
        players: Vec<player::Id>,
        count: usize,
        noun: &'static str,
    },
    Team {
        evil: Vec<player::Id>,
    },
    Bluffs(Vec<&'static str>),
    Alignment(character::Alignment),
    /// The character is not in play.
    NotInPlay(&'static str),
    /// "This character selected you", and what it did.
    Selected {
        by: &'static str,
        effect: &'static str,
    },
    Grimoire(Vec<(player::Id, &'static str)>),
    /// A player, pointed at.
    Player(player::Id),
    /// "This player is this character."
    Is {
        player: player::Id,
        character: &'static str,
    },
    /// Which way round the circle something is.
    Direction(Direction),
    /// Which alignment is winning; `None` for neither.
    Winning(Option<character::Alignment>),
    /// How close a guess was (the Amnesiac's guess at their ability).
    Accuracy(Accuracy),
}

/// Around the circle: clockwise is increasing seat order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Clockwise,
    Anticlockwise,
}

/// How close a guess is, from far off to exact.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Accuracy {
    Cold,
    Warm,
    Hot,
    Bingo,
}

impl Message {
    /// Players the message names, in order.
    pub fn players(&self) -> Vec<&player::Id> {
        match self {
            Message::OneOf { players, .. } => players.iter().collect(),
            Message::Team { evil } => evil.iter().collect(),
            Message::Group { players, .. } => players.iter().collect(),
            Message::Grimoire(seats) => seats.iter().map(|(p, _)| p).collect(),
            Message::Player(player) | Message::Is { player, .. } => vec![player],
            Message::Direction(_) | Message::Winning(_) => Vec::new(),
            Message::Count { .. }
            | Message::YesNo(_)
            | Message::Character(_)
            | Message::Characters(_)
            | Message::Bluffs(_)
            | Message::Alignment(_)
            | Message::NotInPlay(_)
            | Message::Selected { .. }
            | Message::Accuracy(_) => Vec::new(),
        }
    }

    /// Character names the message names, in order.
    pub fn characters(&self) -> Vec<&'static str> {
        match self {
            Message::OneOf { character, .. }
            | Message::Character(character)
            | Message::Is { character, .. } => vec![*character],
            Message::Selected { by, .. } | Message::NotInPlay(by) => vec![*by],
            Message::Bluffs(names) | Message::Characters(names) => names.clone(),
            Message::Grimoire(seats) => seats.iter().map(|(_, c)| *c).collect(),
            Message::Count { .. }
            | Message::YesNo(_)
            | Message::Team { .. }
            | Message::Group { .. }
            | Message::Player(_)
            | Message::Direction(_)
            | Message::Alignment(_)
            | Message::Winning(_) => Vec::new(),
            Message::Accuracy(_) => Vec::new(),
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
            Message::Characters(names) if names.is_empty() => f.write_str("None"),
            Message::Characters(names) => f.write_str(&join(names)),
            Message::Group {
                players,
                count,
                noun,
            } => write!(f, "{}: {count} {noun}", join(players)),
            Message::Team { evil } => write!(f, "Evil: {}", join(evil)),
            Message::Bluffs(names) => write!(f, "Bluffs: {}", join(names)),
            Message::Alignment(alignment) => write!(f, "{alignment}"),
            Message::NotInPlay(name) => write!(f, "The {name} is not in play"),
            Message::Selected { by, effect } => write!(f, "The {by} selected you: {effect}"),
            Message::Grimoire(seats) => {
                let seats: Vec<String> = seats.iter().map(|(p, c)| format!("{p}={c}")).collect();
                f.write_str(&seats.join(", "))
            }
            Message::Player(player) => write!(f, "{player}"),
            Message::Is { player, character } => write!(f, "{player} is the {character}"),
            Message::Direction(Direction::Clockwise) => f.write_str("Clockwise"),
            Message::Direction(Direction::Anticlockwise) => f.write_str("Anticlockwise"),
            Message::Winning(Some(alignment)) => write!(f, "{alignment} is winning"),
            Message::Winning(None) => f.write_str("Neither team is winning"),
            Message::Accuracy(accuracy) => fmt::Debug::fmt(accuracy, f),
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
