//! Every rules spec in `data/characters` parses and has every field its README lists.

use std::fs;
use std::path::Path;

const FIELDS: [&str; 15] = [
    "id",
    "name",
    "kind",
    "ability",
    "first_night",
    "other_nights",
    "reminders",
    "setup",
    "wakes",
    "interaction",
    "drunk_or_poisoned",
    "jinxes",
    "rulings",
    "building_blocks",
    "test_scenarios",
];

const KINDS: [&str; 6] = ["townsfolk", "outsider", "minion", "demon", "traveller", "fabled"];

#[test]
fn character_specs_are_complete() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("data/characters");
    let mut problems = Vec::new();
    for entry in fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_none_or(|e| e != "json") {
            continue;
        }
        let name = path.file_stem().unwrap().to_string_lossy().into_owned();
        let text = fs::read_to_string(&path).unwrap();
        let spec: serde_json::Value = match serde_json::from_str(&text) {
            Ok(v) => v,
            Err(e) => {
                problems.push(format!("{name}: {e}"));
                continue;
            }
        };
        for field in FIELDS {
            if spec.get(field).is_none() {
                problems.push(format!("{name}: missing {field}"));
            }
        }
        if spec["id"] != name.as_str() {
            problems.push(format!("{name}: id does not match the file name"));
        }
        if !KINDS.iter().any(|k| spec["kind"] == *k) {
            problems.push(format!("{name}: bad kind {}", spec["kind"]));
        }
        let scenarios = spec["test_scenarios"].as_array().map_or(0, Vec::len);
        if !(3..=6).contains(&scenarios) {
            problems.push(format!("{name}: {scenarios} test scenarios, want 3-6"));
        }
        if spec["building_blocks"].as_array().is_none_or(Vec::is_empty) {
            problems.push(format!("{name}: no building blocks"));
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}
