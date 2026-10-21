//! What the server and a bridge say to each other over the bridge's
//! WebSocket (`/ws/bridge`), as JSON text messages.
//!
//! The bridge opens the connection and pairs first; then the server sends
//! each storyteller decision as a ready-made prompt, and the bridge replies
//! with whatever its command printed. The server parses and validates the
//! reply, so a bridge cannot make an illegal choice.

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ToServer {
    /// The first message: the pairing code the host was shown.
    Pair { code: String },
    /// The answer to decision `id`: the command's output, or why it failed.
    Reply {
        id: u64,
        #[serde(default)]
        text: Option<String>,
        #[serde(default)]
        error: Option<String>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ToBridge {
    /// Paired with room `room`.
    Paired { room: String },
    /// The pairing code is unknown (or was replaced); the server closes.
    Refused { reason: String },
    /// A storyteller decision: answer `prompt` with the chosen option numbers.
    Decide { id: u64, prompt: String },
}
