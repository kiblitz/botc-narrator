//! Collects every character's ability text (`data/characters/<id>.json`
//! specs and the `data/abilities/*.json` maps, as `scripts/abilities.mjs`
//! does for the simulator) into one JSON map for the lobby's script view.
//! Keys are ids without underscores.

use std::collections;
use std::fs;
use std::path;

fn main() {
    let data = path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../data");
    println!(
        "cargo::rerun-if-changed={}",
        data.join("characters").display()
    );
    println!(
        "cargo::rerun-if-changed={}",
        data.join("abilities").display()
    );
    let key = |id: &str| id.replace('_', "");
    let mut abilities = collections::BTreeMap::new();
    for spec in json_files(&data.join("characters")) {
        if let (Some(id), Some(ability)) = (spec["id"].as_str(), spec["ability"].as_str()) {
            abilities.insert(key(id), ability.to_string());
        }
    }
    for map in json_files(&data.join("abilities")) {
        for (id, text) in map.as_object().into_iter().flatten() {
            if let Some(text) = text.as_str() {
                abilities.insert(key(id), text.to_string());
            }
        }
    }
    let out = path::PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("abilities.json");
    fs::write(out, serde_json::to_string(&abilities).unwrap()).unwrap();
}

fn json_files(dir: &path::Path) -> Vec<serde_json::Value> {
    let mut files: Vec<path::PathBuf> = fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "json"))
        .collect();
    files.sort();
    files
        .iter()
        .map(|p| serde_json::from_str(&fs::read_to_string(p).unwrap()).unwrap())
        .collect()
}
