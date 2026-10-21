// Plays the live site end to end from phone-sized browsers: a bot account
// signs in and hosts, five guests join by code, the host reseats and pairs a
// bridge on this computer, and the table plays into day 2.
//
// Usage (from server/e2e):
//   BOT_EMAIL=... BOT_PASSWORD_FILE=... [SITE=https://botc.kiblitz.xyz]
//   [BRIDGE="claude -p"] [SHOTS=dir] node live.mjs
// BRIDGE is the storyteller command (default: the bridge's own, Codex).
import { chromium, devices } from "@playwright/test";
import { spawn } from "node:child_process";
import { readFileSync } from "node:fs";

const SITE = process.env.SITE || "https://botc.kiblitz.xyz";
const PLAYERS = ["Ada", "Bram", "Cleo", "Dev", "Esme"];
const shot = (page, name) => process.env.SHOTS && page.screenshot({ path: `${process.env.SHOTS}/${name}.png`, fullPage: true });
const log = (...a) => console.log(new Date().toISOString().slice(11, 19), ...a);

const browser = await chromium.launch();
const phone = async () => (await browser.newContext({ ...devices["Pixel 5"], baseURL: SITE })).newPage();

const host = await phone();
await host.goto("/");
await host.getByRole("button", { name: "Sign in or sign up" }).click();
await host.locator('form[action="/api/log-in"] input[name=email]').fill(process.env.BOT_EMAIL);
await host.locator('form[action="/api/log-in"] input[name=password]').fill(readFileSync(process.env.BOT_PASSWORD_FILE, "utf8").trim());
await host.locator('form[action="/api/log-in"] button').click();
await host.getByText("Continue to app").click();
await host.waitForURL(`${SITE}/**`);
await host.locator("#host-name").fill("Host");
await host.getByRole("button", { name: "Host a game" }).click();
const code = (await host.locator("#room-code").innerText()).trim();
log("signed in and hosted room", code);

const pages = [host];
for (const name of PLAYERS) {
  const page = await phone();
  await page.goto(`/?join=${code}`);
  await page.locator("#join-name").fill(name);
  await page.getByRole("button", { name: "Join" }).click();
  await page.locator("#room-code").waitFor();
  pages.push(page);
}
log("five guests joined");
await host.getByRole("button", { name: "Move Esme up" }).click();
await host.waitForTimeout(500);

const pairing = (await host.locator("#bridge-command").innerText()).split(" ")[1];
const command = process.env.BRIDGE ? ["--", ...process.env.BRIDGE.split(" ")] : [];
// The built binary itself (cargo build --release -p botc_bridge first), so
// killing it really stops it, on any exit.
const bridge = spawn("../../target/release/botc-bridge", [pairing, "--server", SITE, ...command], { stdio: ["ignore", "inherit", "inherit"] });
process.on("exit", () => bridge.kill());
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
bridge.kill();
await browser.close();
process.exit(0);
