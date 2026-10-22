// Plays the live site end to end from phone-sized browsers: an anonymous
// visitor hosts, five guests join by code, the host reseats and pairs a
// bridge on this computer, and the table plays into day 2.
//
// Usage (from server/e2e):
//   [SITE=https://botc.kiblitz.xyz] [BRIDGE="claude -p"] [SHOTS=dir]
//   [VIDEO=dir] [CODEX_HOME=dir] node live.mjs
// The bridge runs from the host's one-line command; BRIDGE is the storyteller
// command (default: Codex, using whichever login CODEX_HOME points at, else
// Claude Code); it runs through `sh`, so on Windows Git Bash must be on PATH.
// VIDEO records the host's and the first two players' phones. FULL plays on
// until the game is over, executing someone every day.
import { chromium, devices } from "@playwright/test";
import { spawn, spawnSync } from "node:child_process";

const SITE = process.env.SITE || "https://botc.kiblitz.xyz";
const PLAYERS = ["Ada", "Bram", "Cleo", "Dev", "Esme"];
// Phones recorded with VIDEO: the host's and the first two guests'.
const RECORDED = 3;
// While recording, each step stays on screen long enough to follow.
const pause = (page, ms) => (process.env.VIDEO ? page.waitForTimeout(ms) : Promise.resolve());
const shot = async (page, name) => {
  await pause(page, 2500);
  if (process.env.SHOTS) await page.screenshot({ path: `${process.env.SHOTS}/${name}.png`, fullPage: true });
};
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
  const page = await phone(i + 1 < RECORDED);
  await page.goto(`/?join=${code}`);
  await page.locator("#join-name").fill(name);
  await page.getByRole("button", { name: "Join" }).click();
  await page.locator("#room-code").waitFor();
  pages.push(page);
}
log("five guests joined");
await host.locator("#script").selectOption("trouble_brewing");
await pause(host, 1000);
// The first guest reads the script while the host sets up.
await pages[1].locator("#script-view summary").click();
await pages[1].locator("#script-view .script-character").first().waitFor();
await pages[1].waitForTimeout(1500);
await pages[1].locator("#script-view summary").click();
// Tapping two seats swaps them: Esme and Dev change places.
await host.locator('#seats .seat[data-name="Esme"]').click();
await host.waitForTimeout(400);
await host.locator('#seats .seat[data-name="Dev"]').click();
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
      // A prompt on a recorded phone is shown first.
      if (n < RECORDED && (await page.locator("#prompt").isVisible().catch(() => false)) && !(await page.getByText("Answer sent.").isVisible().catch(() => false))) await pause(page, 3500);
      if (await answer(page).catch(() => false)) {
        answered++;
        log("answered a prompt on phone", n);
      }
    }
    await host.waitForTimeout(300);
  }
  throw new Error(`never reached ${phase}`);
}

for (const page of pages) page.on("dialog", (d) => d.accept());
let answered = await until("Day 1");
if (process.env.FULL) {
  // The recorded phones show their characters and what they learned.
  for (const page of pages.slice(0, RECORDED)) await page.locator("#token").click().catch(() => {});
  await pause(host, 5000);
}
log("day 1");
await shot(pages[1], "live-2-day");
if (process.env.FULL) {
  for (let day = 1; !(await host.locator("#winner").isVisible()); day++) {
    // Whoever may still nominate puts someone up; every living hand goes up.
    const first = day % pages.length;
    const ordered = [...pages.slice(first), ...pages.slice(0, first)];
    let nominated = false;
    for (const page of ordered) {
      // A living nominee, so each execution moves the game on.
      const buttons = page.locator("#town button.seat.nominable:not(.dead)");
      if (!(await buttons.count())) continue;
      await pause(page, 1500);
      await buttons.nth(day % (await buttons.count())).click();
      nominated = true;
      break;
    }
    if (nominated) {
      await host.locator("#vote").waitFor({ timeout: 120_000 }).catch(() => {});
      for (const page of pages) if (await page.locator("#vote-yes").isVisible().catch(() => false)) await page.locator("#vote-yes").click().catch(() => {});
      await pause(host, 3000);
      await shot(host, `full-vote-${day}`);
      if (await host.locator("#close-vote").isVisible().catch(() => false)) await host.locator("#close-vote").click();
    }
    await host.locator("#end-day, #winner").first().waitFor({ timeout: 120_000 });
    if (await host.locator("#winner").isVisible()) break;
    await pause(host, 2000);
    await host.locator("#end-day").click();
    await host.locator("#start-night, #winner").first().waitFor({ timeout: 300_000 });
    if (await host.locator("#winner").isVisible()) break;
    log("day", day, "ended");
    await pause(host, 2000);
    await host.locator("#start-night").click();
    answered += await until(`Day ${day + 1}`);
    log("day", day + 1);
    await pause(host, 4000);
  }
  log("winner:", await host.locator("#winner").innerText());
  for (const page of pages.slice(0, RECORDED)) await page.locator("#winner").scrollIntoViewIfNeeded();
  await pause(host, 6000);
} else {
  const nominator = pages[1];
  await nominator.locator("#town button.seat.nominable").first().click();
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
}
// Closing the contexts finishes their videos.
for (const context of contexts) await context.close();
await browser.close();
process.exit(0);
