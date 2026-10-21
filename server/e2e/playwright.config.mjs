import { defineConfig, devices } from "@playwright/test";

const port = 8124;

// Phones at a table: every test runs in a phone-sized, touch-enabled browser.
export default defineConfig({
  testDir: "tests",
  timeout: 180_000,
  workers: 1,
  use: { ...devices["Pixel 5"], baseURL: `http://127.0.0.1:${port}/` },
  webServer: {
    command: `cargo run --quiet -p botc_server --example e2e -- ${port} 4`,
    url: `http://127.0.0.1:${port}/health`,
    timeout: 300_000,
    reuseExistingServer: false,
  },
});
