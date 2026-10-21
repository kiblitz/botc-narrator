//! WebAssembly entry point for the simulator site.

use wasm_bindgen::prelude::wasm_bindgen;

use botc_narrator::sim;

/// A random game as JSON (see `sim::Trace`), using the script with `script_id`.
#[wasm_bindgen]
pub fn simulate(
    seed: u32,
    players: u32,
    script_id: &str,
    names: Vec<String>,
) -> Result<String, String> {
    sim::simulate_named_json(u64::from(seed), players as usize, script_id, &names)
}

/// The available scripts as JSON: `[{ "id", "name" }]`.
#[wasm_bindgen]
pub fn scripts() -> String {
    sim::scripts_json()
}
