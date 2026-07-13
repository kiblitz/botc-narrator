//! The character registry: a data-driven map from id to behaviour.
//!
//! Adding a character to the game is `registry.register(MyRole)` — no central
//! enum to extend, no match arm to widen. Scripts are just lists of ids that
//! must exist in the registry.

use crate::ability::Ability;
use crate::ids::CharacterId;
use crate::role::CharacterInfo;
use std::collections::HashMap;

#[derive(Default)]
pub struct Registry {
    abilities: HashMap<CharacterId, Box<dyn Ability>>,
}

impl Registry {
    #[must_use]
    pub fn new() -> Self {
        Registry {
            abilities: HashMap::new(),
        }
    }

    /// Register a character. Keyed by the ability's own [`CharacterInfo::id`].
    /// Panics if two characters claim the same id.
    pub fn register<A: Ability + 'static>(&mut self, ability: A) {
        let id = ability.info().id;
        if self.abilities.insert(id, Box::new(ability)).is_some() {
            panic!("duplicate character id registered: {id}");
        }
    }

    /// The behaviour for `id`. Panics if unregistered — a script referencing an
    /// unknown character is a programming error, surfaced loudly.
    #[must_use]
    pub fn get(&self, id: CharacterId) -> &dyn Ability {
        self.abilities
            .get(&id)
            .map(AsRef::as_ref)
            .unwrap_or_else(|| panic!("unregistered character: {id}"))
    }

    #[must_use]
    pub fn try_get(&self, id: CharacterId) -> Option<&dyn Ability> {
        self.abilities.get(&id).map(AsRef::as_ref)
    }

    #[must_use]
    pub fn info(&self, id: CharacterId) -> CharacterInfo {
        self.get(id).info()
    }
}
