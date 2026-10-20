use crate::character;
use crate::trouble_brewing;

/// An edition: its characters and night orders.
pub struct Script {
    /// Stable identifier, e.g. for URLs.
    pub id: &'static str,
    pub name: &'static str,
    pub characters: Vec<character::Ref>,
    pub first_night: Vec<character::Ref>,
    pub other_nights: Vec<character::Ref>,
}

/// Every script the engine knows.
pub fn all() -> Vec<Script> {
    vec![trouble_brewing::script()]
}

pub fn by_id(id: &str) -> Option<Script> {
    all().into_iter().find(|s| s.id == id)
}

impl Script {
    pub fn find(&self, id: &str) -> Option<character::Ref> {
        self.characters.iter().copied().find(|c| c.id() == id)
    }
}
