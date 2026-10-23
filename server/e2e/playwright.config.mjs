import { defineConfig, devices } from "@playwright/test";

const port = 8124;
// A second server whose seed deals the Midnight Oasis to six with a
// Psychopath and a Savant (tests/day-actions.spec.mjs).
export const DAY_ACTIONS_PORT = 8125;
const server = (port, seed) => ({
  command: `cargo run --quiet -p botc_server --example e2e -- ${port} ${seed}`,
  url: `http://127.0.0.1:${port}/health`,
  timeout: 300_000,
  reuseExistingServer: false,
});

// Phones at a table: every test runs in a phone-sized, touch-enabled browser.
export default defineConfig({
  testDir: "tests",
  timeout: 180_000,
  workers: 1,
  use: { ...devices["Pixel 5"], baseURL: `http://127.0.0.1:${port}/` },
  webServer: [server(port, 4), server(DAY_ACTIONS_PORT, 10)],
});
