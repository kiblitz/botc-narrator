//! JSON messages exchanged with player browsers.
//!
//! Player-facing by construction: options and public state carry **names**, not
//! the storyteller's "Name(Role)" labels, so a client never receives another
//! player's hidden role.

use serde::{Deserialize, Serialize};

/// A selectable player in a prompt, shown by name only.
#[derive(Debug, Clone, Serialize)]
pub struct Opt {
    pub id: usize,
    pub name: String,
}

/// A player's public state (everything all players may see).
#[derive(Debug, Clone, Serialize)]
pub struct PublicPlayer {
    pub id: usize,
    pub name: String,
    pub alive: bool,
}

/// Final role reveal, sent when the game ends.
#[derive(Debug, Clone, Serialize)]
pub struct FinalRole {
    pub name: String,
    pub role: String,
}

/// Server → client.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMsg {
    /// Your assigned seat id.
    Joined { you: usize },
    /// The lobby roster while waiting to start.
    Lobby {
        players: Vec<String>,
        can_start: bool,
    },
    /// The game has begun.
    Started,
    /// Private: your (believed) role and team.
    Role { role: String, team: String },
    /// Public board state.
    Public {
        phase: String,
        players: Vec<PublicPlayer>,
    },
    /// Private: you have woken.
    Wake,
    /// Private: you may sleep.
    Sleep,
    /// Private: one-way information for you.
    Info { message: String },
    /// Private: choose one of the options.
    Prompt { prompt: String, options: Vec<Opt> },
    /// A nomination is open for voting.
    VoteOpen {
        nominee: usize,
        nominee_name: String,
        threshold: usize,
    },
    /// The result of a nomination's vote.
    VoteResult {
        nominee: usize,
        nominee_name: String,
        votes: usize,
        executed: bool,
    },
    /// The day has begun; nominate or pass.
    DayStarted,
    /// The night has begun.
    NightStarted,
    /// The game is over.
    GameOver {
        winner: String,
        roles: Vec<FinalRole>,
    },
    /// A recoverable error for this client.
    Error { message: String },
}

/// Client → server.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMsg {
    /// Join the lobby with a display name.
    Join { name: String },
    /// Start the game (needs enough players).
    Start,
    /// Answer to the current night [`ServerMsg::Prompt`].
    Answer { target: usize },
    /// Nominate a player for execution.
    Nominate { nominee: usize },
    /// Vote on the currently open nomination.
    Vote { yes: bool },
    /// Decline to nominate this day.
    Pass,
}

/// A client command tagged with the seat it came from.
#[derive(Debug, Clone)]
pub struct PlayerCmd {
    pub player: usize,
    pub msg: ClientMsg,
}

/// Where a game event is delivered.
#[derive(Debug, Clone, Copy)]
pub enum Target {
    One(usize),
    All,
}

/// An outbound event from the game thread to the async relay.
#[derive(Debug, Clone)]
pub struct GameEvent {
    pub to: Target,
    pub msg: ServerMsg,
}
