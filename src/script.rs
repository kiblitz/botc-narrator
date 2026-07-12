//! A script: the roster and night orders for an edition (e.g. Trouble Brewing).
//!
//! Purely data. Every id must be registered in the [`crate::registry::Registry`]
//! used to run it.

use crate::ids::CharacterId;

pub struct Script {
    pub id: &'static str,
    pub name: &'static str,
    /// Every character that may appear on this script.
    pub roster: Vec<CharacterId>,
    /// Wake order for the first night.
    pub first_night: Vec<CharacterId>,
    /// Wake order for every subsequent night.
    pub other_nights: Vec<CharacterId>,
}

impl Script {
    #[must_use]
    pub fn night_order(&self, night: u32) -> &[CharacterId] {
        if night <= 1 {
            &self.first_night
        } else {
            &self.other_nights
        }
    }
}
