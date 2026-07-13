//! Concrete [`Storyteller`](crate::storyteller::Storyteller) backends.

mod auto;
mod console;
mod scripted;

pub use auto::{
    AutoStoryteller, DiscretionPolicy, PlayerAgent, RandomAgent, RandomDiscretion, Rng,
};
pub use console::ConsoleStoryteller;
pub use scripted::ScriptedStoryteller;
