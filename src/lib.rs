//! # botc-narrator
//!
//! A Blood on the Clocktower narrator (storyteller) engine, starting with the
//! Trouble Brewing edition.
//!
//! ## Design in one paragraph
//!
//! State is an open bag of reminder [`token`]s on players in a [`grimoire`];
//! nothing about a specific role is hardcoded into the state. Behaviour lives
//! in stateless [`ability::Ability`] objects held in a [`registry`], selected
//! by a data-only [`script`]. Roles never touch each other: they raise intents
//! through a [`ctx::Ctx`] — "the demon attacks X", "does Y register as evil?" —
//! and the pipeline resolves them by consulting every ability's hooks. Adding a
//! character or a whole script is therefore append-only: implement the trait,
//! register it, list its id. I/O is abstracted behind [`storyteller::Storyteller`]
//! so the same logic drives a console, a UI, or a deterministic test.

pub mod ability;
pub mod ctx;
pub mod engine;
pub mod event;
pub mod grimoire;
pub mod ids;
pub mod interp;
pub mod registry;
pub mod role;
pub mod script;
pub mod storyteller;
pub mod token;
pub mod voting;

pub mod characters;
pub mod scripts;

/// Common imports for defining and using characters.
pub mod prelude {
    pub use crate::ability::Ability;
    pub use crate::ctx::Ctx;
    pub use crate::event::{DeathSource, Registration};
    pub use crate::grimoire::{Grimoire, Phase, Player, PlayerSpec};
    pub use crate::ids::{CharacterId, PlayerId};
    pub use crate::registry::Registry;
    pub use crate::role::{Alignment, CharacterInfo, Kind};
    pub use crate::script::Script;
    pub use crate::token::Token;
}
