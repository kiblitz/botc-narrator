//! WebAssembly entry point for the simulator site.

use wasm_bindgen::prelude::wasm_bindgen;

use botc_narrator::sim;

/// A random game as JSON (see `sim::Trace`).
#[wasm_bindgen]
pub fn simulate(seed: u32, players: u32) -> String {
    sim::simulate_json(u64::from(seed), players as usize)
}
