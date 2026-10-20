use crate::character;

/// An edition: its characters and night orders.
pub struct Script {
    pub name: &'static str,
    pub characters: Vec<character::Ref>,
    pub first_night: Vec<character::Ref>,
    pub other_nights: Vec<character::Ref>,
}

impl Script {
    pub fn find(&self, id: &str) -> Option<character::Ref> {
        self.characters.iter().copied().find(|c| c.id() == id)
    }
}
