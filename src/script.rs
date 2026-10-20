//! Scripts (editions): character lists loaded from the script JSON files in
//! `data/scripts`, with night orders computed from each character's
//! `character::Def::night_order`.

use std::fmt;

use crate::character;
use crate::characters;

/// The playable scripts: id and script JSON (official character ids).
const SCRIPTS: &[(&str, &str)] = &[(
    "trouble_brewing",
    include_str!("../data/scripts/trouble_brewing.json"),
)];

/// An edition: its characters and night orders.
pub struct Script {
    /// Stable identifier, e.g. for URLs.
    pub id: &'static str,
    pub name: String,
    pub characters: Vec<character::Ref>,
    /// Characters with a first-night position, in night order.
    pub first_night: Vec<character::Ref>,
    /// Characters with an other-nights position, in night order.
    pub other_nights: Vec<character::Ref>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    Json(String),
    /// Ids no registered character (`characters::all`) has.
    UnknownCharacters(Vec<String>),
    MissingName,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Json(e) => write!(f, "invalid script JSON: {e}"),
            Error::UnknownCharacters(ids) => {
                write!(f, "unknown character ids: {}", ids.join(", "))
            }
            Error::MissingName => f.write_str("the script has no _meta name"),
        }
    }
}

impl std::error::Error for Error {}

/// Every playable script.
pub fn all() -> Vec<Script> {
    SCRIPTS
        .iter()
        .map(|(id, json)| parse(id, json).unwrap_or_else(|e| panic!("script {id}: {e}")))
        .collect()
}

pub fn by_id(id: &str) -> Option<Script> {
    all().into_iter().find(|s| s.id == id)
}

/// Parses a script in the standard JSON format: a `{"id": "_meta", "name"}`
/// entry and official character ids, as strings or `{"id"}` objects.
pub fn parse(id: &'static str, json: &str) -> Result<Script, Error> {
    let entries: Vec<serde_json::Value> =
        serde_json::from_str(json).map_err(|e| Error::Json(e.to_string()))?;
    let mut name = None;
    let mut characters = Vec::new();
    let mut unknown = Vec::new();
    for entry in &entries {
        let entry_id = match entry {
            serde_json::Value::String(s) => s.as_str(),
            serde_json::Value::Object(o) => o
                .get("id")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| Error::Json(format!("entry without an id: {entry}")))?,
            other => return Err(Error::Json(format!("unexpected entry: {other}"))),
        };
        if entry_id == "_meta" {
            name = entry.get("name").and_then(serde_json::Value::as_str);
            continue;
        }
        match characters::find(entry_id) {
            Some(c) => characters.push(c),
            None => unknown.push(entry_id.to_string()),
        }
    }
    if !unknown.is_empty() {
        return Err(Error::UnknownCharacters(unknown));
    }
    let name = name.ok_or(Error::MissingName)?;
    Ok(Script::new(id, name, characters))
}

impl Script {
    pub fn new(id: &'static str, name: impl Into<String>, characters: Vec<character::Ref>) -> Self {
        let order = |position: fn(character::NightOrder) -> Option<u32>| {
            let mut order: Vec<(u32, character::Ref)> = characters
                .iter()
                .filter_map(|c| position(c.night_order()).map(|n| (n, *c)))
                .collect();
            order.sort_by_key(|(n, _)| *n);
            order.into_iter().map(|(_, c)| c).collect()
        };
        Self {
            id,
            name: name.into(),
            first_night: order(|o| o.first),
            other_nights: order(|o| o.other),
            characters,
        }
    }

    pub fn find(&self, id: &str) -> Option<character::Ref> {
        self.characters.iter().copied().find(|c| c.id() == id)
    }
}
