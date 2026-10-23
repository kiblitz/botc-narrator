// On a script with the Atheist (the Midnight Oasis), the Storyteller is a
// legal nominee: the host picks them in the middle of the town square, the
// table votes as on anyone, and executing them ends the game.
import { test, expect } from "@playwright/test";
import { DAY_ACTIONS_PORT } from "../playwright.config.mjs";

const BASE = `http://127.0.0.1:${DAY_ACTIONS_PORT}`;
const PLAYERS = ["Ada", "Bram", "Cleo", "Dev", "Esme"];

async function phone(browser) {
  const context = await browser.newContext();
  return context.newPage();
}

/** Answers the page's prompt, if it has one, with the first legal choice. */
async function answerPrompt(page) {
  const prompt = page.locator("#prompt");
  if (!(await prompt.isVisible())) return;
  if (await prompt.getByText("Answer sent.").isVisible()) return;
  const confirm = prompt.locator("#confirm");
  if (await confirm.count()) {
    const choices = prompt.locator(".choices button");
    for (let i = 0; i < (await choices.count()) && (await confirm.isDisabled()); i++) await choices.nth(i).click();
    await confirm.click();
  } else if (await prompt.locator(".choices button").count()) {
    await prompt.locator(".choices button").first().click();
  } else {
    await prompt.getByRole("button", { name: "No" }).click();
  }
}

test("the host nominates the Storyteller on an Atheist script; executing them ends the game", async ({ browser }) => {
  const host = await phone(browser);
  await host.goto(`${BASE}/`);
  await host.locator("#host-name").fill("Host");
  await host.getByRole("button", { name: "Host a game" }).click();
  const code = (await host.locator("#room-code").innerText()).trim();
  const players = [];
  for (const name of PLAYERS) {
    const page = await phone(browser);
    await page.goto(`${BASE}/?join=${code}`);
    await page.locator("#join-name").fill(name);
    await page.getByRole("button", { name: "Join" }).click();
    await expect(page.locator("#room-code")).toHaveText(code);
    players.push(page);
  }
  await expect(host.locator("#seats .seat")).toHaveCount(6);
  await host.locator("#script").selectOption("the_midnight_oasis");
  await expect(host.locator("#start")).toBeEnabled();
  await host.locator("#start").click();
  const pages = [host, ...players];
  for (let i = 0; i < 400; i++) {
    if ((await host.locator("#phase").innerText().catch(() => "")) === "Day 1") break;
    for (const page of pages) await answerPrompt(page).catch(() => {});
    await host.waitForTimeout(150);
  }
  await expect(host.locator("#phase")).toHaveText("Day 1");

  // The Storyteller is offered only once a nominator is picked.
  await expect(host.locator("#storyteller-seat")).toHaveCount(0);
  await host.locator('#town button.seat.nominable[aria-label$=" nominates"]').first().click();
  await host.getByRole("button", { name: "Nominate the Storyteller" }).click();
  await expect(host.locator("#nomination")).toContainText("nominates Storyteller");
  await host.locator("#confirm-nomination").click();
  for (const page of pages) {
    await expect(page.locator("#vote")).toContainText("Storyteller");
    await expect(page.locator("#storyteller-seat.nominee")).toBeVisible();
  }
  for (const player of players) await expect(player.locator("#storyteller-seat")).not.toHaveAttribute("aria-label", /.+/);
  const voters = await host.locator('#town button[aria-label^="Raise "]').evaluateAll((nodes) => nodes.map((n) => n.dataset.name));
  expect(voters.length).toBeGreaterThan(3);
  for (const name of voters) {
    await host.getByRole("button", { name: `Raise ${name}'s hand` }).click();
    await expect(host.getByRole("button", { name: `Lower ${name}'s hand` })).toBeVisible();
  }
  await host.locator("#close-vote").click();
  await expect(host.locator("#log")).toContainText("votes for the Storyteller");
  host.on("dialog", (d) => d.accept());
  await host.locator("#end-day").click();
  for (const page of pages) {
    await expect(page.locator("#winner")).toBeVisible();
    await expect(page.locator("#log")).toContainText("the Storyteller is executed");
  }
});
