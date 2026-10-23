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
    /// Some players, e.g. the Demon player, or whom someone chose.
    Players(Vec<player::Id>),
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
    /// "This player is good / evil."
    IsAligned {
        player: player::Id,
        alignment: character::Alignment,
    },
    /// Advice for the listener's team: trust this player, or do not.
    Advice {
        player: player::Id,
        trust: bool,
    },
    /// Several statements given together (the Savant's two).
    Statements(Vec<Message>),
    /// Which way round the circle something is.
    Direction(Direction),
    /// Which alignment is winning; `None` for neither.
    Winning(Option<character::Alignment>),
    /// How close a guess was (the Amnesiac's guess at their ability).
    Accuracy(Accuracy),
    /// A secret word (the Mezepheles's).
    Word(&'static str),
    /// A visit to the storyteller with nothing to learn: what a player who
    /// claims a private day ability they do not have is told.
    Nothing,
    /// The storyteller took note of what a player reported (madness).
    Noted,
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
            Message::Direction(_) | Message::Winning(_) => Vec::new(),
            Message::Player(player)
            | Message::Is { player, .. }
            | Message::IsAligned { player, .. }
            | Message::Advice { player, .. } => vec![player],
            Message::Statements(statements) => {
                statements.iter().flat_map(Message::players).collect()
            }
            Message::Players(players) => players.iter().collect(),
            Message::Count { .. }
            | Message::YesNo(_)
            | Message::Character(_)
            | Message::Characters(_)
            | Message::Bluffs(_)
            | Message::Alignment(_)
            | Message::NotInPlay(_)
            | Message::Selected { .. }
            | Message::Accuracy(_)
            | Message::Word(_)
            | Message::Nothing
            | Message::Noted => Vec::new(),
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
            Message::Statements(statements) => {
                statements.iter().flat_map(Message::characters).collect()
            }
            Message::Count { .. }
            | Message::YesNo(_)
            | Message::Team { .. }
            | Message::Group { .. }
            | Message::Player(_)
            | Message::Direction(_)
            | Message::Alignment(_)
            | Message::Players(_)
            | Message::Winning(_)
            | Message::IsAligned { .. }
            | Message::Advice { .. } => Vec::new(),
            Message::Accuracy(_) => Vec::new(),
            Message::Word(_) | Message::Nothing | Message::Noted => Vec::new(),
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
            Message::Players(players) if players.is_empty() => f.write_str("Nobody"),
            Message::Players(players) => f.write_str(&join(players)),
            Message::Characters(names) if names.is_empty() => f.write_str("None"),
            Message::Characters(names) => f.write_str(&join(names)),
            Message::Group {
                players,
                count,
                noun,
            } => write!(f, "{}: {count} {noun}", join(players)),
            Message::Team { evil } => write!(f, "Evil: {}", join(evil)),
            Message::Bluffs(names) if names.is_empty() => {
                f.write_str("Bluffs: none (every good character on the script is in play)")
            }
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
            Message::IsAligned { player, alignment } => write!(f, "{player} is {alignment}"),
            Message::Advice {
                player,
                trust: true,
            } => write!(f, "Trust {player}"),
            Message::Advice {
                player,
                trust: false,
            } => write!(f, "Do not trust {player}"),
            Message::Statements(statements) => f.write_str(&join_with(statements, "; ")),
            Message::Word(word) => write!(f, "The word is \"{word}\""),
            Message::Nothing => f.write_str("The storyteller has nothing for you"),
            Message::Noted => f.write_str("The storyteller has noted it"),
        }
    }
}

fn join<T: fmt::Display>(items: &[T]) -> String {
    join_with(items, ", ")
}

fn join_with<T: fmt::Display>(items: &[T], separator: &str) -> String {
    items
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(separator)
}
