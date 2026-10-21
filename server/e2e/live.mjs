// Plays the live site end to end from phone-sized browsers: an anonymous
// visitor hosts, five guests join by code, the host reseats and pairs a
// bridge on this computer, and the table plays into day 2.
//
// Usage (from server/e2e):
//   [SITE=https://botc.kiblitz.xyz] [BRIDGE="claude -p"] [SHOTS=dir]
//   [VIDEO=dir] [CODEX_HOME=dir] node live.mjs
// The bridge runs from the host's one-line command; BRIDGE is the storyteller
// command (default: Codex, using whichever login CODEX_HOME points at, else
// Claude Code). VIDEO records the host's and the first player's phones.
import { chromium, devices } from "@playwright/test";
import { spawn, spawnSync } from "node:child_process";

const SITE = process.env.SITE || "https://botc.kiblitz.xyz";
const PLAYERS = ["Ada", "Bram", "Cleo", "Dev", "Esme"];
const shot = (page, name) => process.env.SHOTS && page.screenshot({ path: `${process.env.SHOTS}/${name}.png`, fullPage: true });
const log = (...a) => console.log(new Date().toISOString().slice(11, 19), ...a);

const browser = await chromium.launch();
const contexts = [];
const phone = async (record) => {
  const context = await browser.newContext({
    ...devices["Pixel 5"],
    baseURL: SITE,
    ...(record && process.env.VIDEO ? { recordVideo: { dir: process.env.VIDEO, size: { width: 393, height: 851 } } } : {}),
  });
  contexts.push(context);
  return context.newPage();
};

const host = await phone(true);
await host.goto("/");
await host.locator("#host-name").fill("Host");
await host.getByRole("button", { name: "Host a game" }).click();
const code = (await host.locator("#room-code").innerText()).trim();
log("hosted room", code, "without an account");

const pages = [host];
for (const [i, name] of PLAYERS.entries()) {
  const page = await phone(i === 0);
  await page.goto(`/?join=${code}`);
  await page.locator("#join-name").fill(name);
  await page.getByRole("button", { name: "Join" }).click();
  await page.locator("#room-code").waitFor();
  pages.push(page);
}
log("five guests joined");
await host.getByRole("button", { name: "Move Esme up" }).click();
await host.waitForTimeout(500);

// The host's one-line bridge command, run as shown (plus BRIDGE's command).
const line = (await host.locator("#bridge-command").innerText()) + (process.env.BRIDGE ? ` -- ${process.env.BRIDGE}` : "");
const bridge = spawn("sh", ["-c", line], { stdio: ["ignore", "inherit", "inherit"], detached: process.platform !== "win32" });
// The whole tree, on any exit: the shell, Node and the storyteller command.
const stopBridge = () =>
  process.platform === "win32" ? spawnSync("taskkill", ["/T", "/F", "/PID", String(bridge.pid)]) : process.kill(-bridge.pid);
process.on("exit", stopBridge);
await host.locator("#bridge .dot.on").waitFor({ timeout: 300_000 });
log("bridge paired");
await shot(host, "live-1-lobby");
await host.locator("#start").click();

async function answer(page) {
  const prompt = page.locator("#prompt");
  if (!(await prompt.isVisible()) || (await prompt.getByText("Answer sent.").isVisible())) return false;
  const confirm = prompt.locator("#confirm");
  if (await confirm.count()) {
    const choices = prompt.locator(".choices button");
    for (let i = 0; i < (await choices.count()) && (await confirm.isDisabled()); i++) await choices.nth(i).click();
    await confirm.click();
  } else if (await prompt.locator(".choices button").count()) await prompt.locator(".choices button").first().click();
  else await prompt.getByRole("button", { name: "No" }).click();
  return true;
}

async function until(phase) {
  let answered = 0;
  for (let i = 0; i < 4000; i++) {
    if ((await host.locator("#phase").innerText().catch(() => "")) === phase || (await host.locator("#winner").isVisible())) return answered;
    for (const [n, page] of pages.entries()) {
      if (await answer(page).catch(() => false)) {
        answered++;
        log("answered a prompt on phone", n);
      }
    }
    await host.waitForTimeout(300);
  }
  throw new Error(`never reached ${phase}`);
}

let answered = await until("Day 1");
log("day 1");
await shot(pages[1], "live-2-day");
const nominator = pages[1];
nominator.on("dialog", (d) => d.accept());
host.on("dialog", (d) => d.accept());
const target = await nominator.locator("#town .seat").filter({ has: nominator.getByRole("button", { name: "Nominate" }) }).first();
await target.getByRole("button", { name: "Nominate" }).click();
for (const page of pages) {
  await page.locator("#vote").waitFor({ timeout: 120_000 });
  await page.locator("#vote-no").click();
}
await shot(host, "live-3-vote");
await host.locator("#end-day").waitFor({ timeout: 120_000 });
await host.locator("#end-day").click();
await host.locator("#start-night").waitFor({ timeout: 300_000 });
await host.locator("#start-night").click();
answered += await until("Day 2");
await shot(pages[2], "live-4-day2");
const bridged = await host.locator("#dock").innerText();
log("reached day 2; prompts answered:", answered, "dock:", bridged);
const state = await host.evaluate(() => document.body.className);
log("host stage:", state);
// Closing the contexts finishes their videos.
for (const context of contexts) await context.close();
await browser.close();
process.exit(0);
