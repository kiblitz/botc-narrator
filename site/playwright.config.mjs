import { defineConfig } from "@playwright/test";

const port = 8123;

export default defineConfig({
  testDir: "tests",
  use: { baseURL: `http://localhost:${port}/` },
  webServer: {
    command: "node serve.mjs",
    env: { PORT: String(port) },
    url: `http://localhost:${port}/`,
    reuseExistingServer: false,
  },
});
