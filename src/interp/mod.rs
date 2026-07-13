//! Concrete [`Storyteller`](crate::storyteller::Storyteller) backends.

mod console;
mod scripted;

pub use console::ConsoleStoryteller;
pub use scripted::ScriptedStoryteller;
