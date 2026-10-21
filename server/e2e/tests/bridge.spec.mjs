// The host's one-line bridge commands, run as shown: each fetches the bridge
// from the server and runs it with Node, which finds a fake `codex` on PATH,
// pairs with the room and answers the storyteller's decisions.
import { test, expect } from "@playwright/test";
import { spawn, spawnSync } from "node:child_process";
import { mkdtempSync, writeFileSync, chmodSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";

const windows = process.platform === "win32";

/** A directory holding a `codex` that ignores its prompt and picks option 0. */
function fakeCodex() {
  const dir = mkdtempSync(path.join(tmpdir(), "fake-codex-"));
  writeFileSync(path.join(dir, "codex"), "#!/bin/sh\ncat >/dev/null\necho 0\n");
  chmodSync(path.join(dir, "codex"), 0o755);
  writeFileSync(path.join(dir, "codex.cmd"), "@echo off\r\nmore >nul\r\necho 0\r\n");
  return dir;
}

/** Runs a shown command line in its shell, with the fake codex first on PATH. */
function run(shell, line) {
  const env = { ...process.env, PATH: `${fakeCodex()}${path.delimiter}${process.env.PATH}` };
  const args = shell === "sh" ? ["-c", line] : ["-NoProfile", "-Command", line];
  const child = spawn(shell, args, { env, detached: !windows, stdio: ["ignore", "pipe", "pipe"] });
  let output = "";
  child.stdout.on("data", (d) => (output += d));
  child.stderr.on("data", (d) => (output += d));
  return {
    output: () => output,
    // The whole tree: the shell, Node, and whatever the bridge started.
    stop: () => (windows ? spawnSync("taskkill", ["/T", "/F", "/PID", String(child.pid)]) : process.kill(-child.pid)),
  };
}

const PLAYERS = ["Ada", "Bram", "Cleo", "Dev"];

for (const [shell, id] of [["sh", "#bridge-command"], ...(windows ? [["powershell", "#bridge-command-powershell"]] : [])]) {
  test(`the ${shell} one-liner pairs a bridge that answers decisions`, async ({ page, request }) => {
    await page.goto("/");
    await page.locator("#host-name").fill("Host");
    await page.getByRole("button", { name: "Host a game" }).click();
    const code = (await page.locator("#room-code").innerText()).trim();
    for (const name of PLAYERS) {
      expect((await request.post(`/api/rooms/${code}/join`, { data: { name } })).ok()).toBe(true);
    }
    await expect(page.locator("#bridge .dot.on")).toHaveCount(0);
    const line = await page.locator(id).innerText();
    expect(line).toContain("--server http://127.0.0.1");

    const bridge = run(shell, line);
    try {
      await expect(page.locator("#bridge .dot.on")).toBeVisible({ timeout: 60_000 });
      expect(bridge.output()).toContain("answering with: codex exec");
      await page.locator("#start").click();
      await expect.poll(bridge.output, { timeout: 60_000 }).toMatch(/decision \d+: answered in [\d.]+s: 0/);
    } finally {
      bridge.stop();
    }
  });
}
