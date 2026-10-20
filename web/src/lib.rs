//! WebAssembly entry point for the simulator site.

use wasm_bindgen::prelude::wasm_bindgen;

use botc_narrator::sim;

/// A random game as JSON (see `sim::Trace`), using the script with `script_id`.
#[wasm_bindgen]
pub fn simulate(seed: u32, players: u32, script_id: &str) -> Result<String, String> {
    sim::simulate_json(u64::from(seed), players as usize, script_id)
}

/// The available scripts as JSON: `[{ "id", "name" }]`.
#[wasm_bindgen]
pub fn scripts() -> String {
    sim::scripts_json()
}
