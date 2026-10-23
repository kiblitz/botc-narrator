// The Day actions screen on phones: every living phone lists the same day
// actions (with their ability text), whatever its character; using one picks
// its targets on the town square, where Cancel sends nothing and Confirm
// uses it; a kill shows on every phone as a death without its cause.
//
// The second e2e server's seed deals the Midnight Oasis to six players:
// Vigormortis, Snake Charmer, Psychopath, Barber, Savant, Farmer.
import { test, expect } from "@playwright/test";
import { DAY_ACTIONS_PORT } from "../playwright.config.mjs";

const BASE = `http://127.0.0.1:${DAY_ACTIONS_PORT}`;
const PLAYERS = ["Ada", "Bram", "Cleo", "Dev", "Esme"];
// SHOTS=<dir> saves screenshots of the Day actions screen and the picker.
const shot = (page, name) => process.env.SHOTS && page.screenshot({ path: `${process.env.SHOTS}/${name}.png`, fullPage: true });

async function phone(browser) {
  const context = await browser.newContext();
  return context.newPage();
}

/** Answers the page's prompt, if it has one, with the first legal choice. */
async function answerPrompt(page) {
  const prompt = page.locator("#prompt");
  if (!(await prompt.isVisible())) return false;
  if (await prompt.getByText("Answer sent.").isVisible()) return false;
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
  return true;
}

async function playUntil(pages, phase) {
  for (let i = 0; i < 400; i++) {
    if ((await pages[0].locator("#phase").innerText().catch(() => "")) === phase) return;
    for (const page of pages) await answerPrompt(page).catch(() => false);
    await pages[0].waitForTimeout(150);
  }
  throw new Error(`never reached ${phase}`);
}

/** The Day actions list as the page shows it: each action's name and text. */
async function listed(page) {
  await page.locator("#day-actions").click();
  await expect(page.locator("#day-list")).toBeVisible();
  const items = await page.locator("#day-list .day-action").evaluateAll((nodes) =>
    nodes.map((n) => ({ ability: n.dataset.ability, text: n.querySelector("div").innerText })));
  return items;
}

test("the Day actions screen: the same list everywhere, targets on the town square, confirm or cancel", async ({ browser }) => {
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
  const names = ["Host", ...PLAYERS];
  for (const page of pages) await page.locator("#token").click();
  await playUntil(pages, "Day 1");
  // Read at dawn: the Snake Charmer may have swapped with the Demon.
  const characters = [];
  for (const page of pages) characters.push(await page.locator("#my-character").innerText());

  // Every phone lists the same actions, each with its ability text.
  const lists = [];
  for (const page of pages) {
    lists.push(await listed(page));
    await expect(page.locator("#honour-note")).toContainText("cannot hear the table");
  }
  for (const page of pages) {
    await page.locator("#close-day").click();
    await expect(page.locator("#town")).toBeVisible();
  }
  for (const list of lists) expect(list).toEqual(lists[0]);
  const abilities = lists[0].map((a) => a.ability);
  expect(abilities).toEqual(expect.arrayContaining(["psychopath", "savant", "fisherman", "damsel"]));
  const psychopathText = lists[0].find((a) => a.ability === "psychopath").text;
  expect(psychopathText).toContain("Psychopath");
  expect(psychopathText.length).toBeGreaterThan(40);

  const psycho = characters.indexOf("Psychopath");
  expect(psycho).toBeGreaterThanOrEqual(0);
  const killer = pages[psycho];
  const victim = names[characters.findIndex((c, i) => i !== psycho && !["Vigormortis", "Psychopath"].includes(c))];
  await killer.locator("#day-actions").click();
  await shot(killer, "day-actions-list");

  // Use, tap a seat, Cancel: nothing is sent, and the list comes back.
  await killer.locator("#use-psychopath").click();
  await expect(killer.locator("#target-circle")).toBeVisible();
  await expect(killer.locator("#confirm-day")).toBeDisabled();
  await killer.getByRole("button", { name: `Pick ${victim}` }).click();
  await expect(killer.locator("#day-targets")).toContainText(victim);
  await expect(killer.locator("#confirm-day")).toBeEnabled();
  await shot(killer, "day-actions-target-picker");
  await killer.locator("#cancel-day").click();
  await expect(killer.locator("#day-list")).toBeVisible();
  await killer.waitForTimeout(500);
  await expect(host.locator("#log")).not.toContainText("Psychopath ability");

  // Use, pick, Confirm: the table is told, and every phone shows the death
  // in the town square, without its cause.
  await killer.locator("#use-psychopath").click();
  await killer.getByRole("button", { name: `Pick ${victim}` }).click();
  await killer.locator("#confirm-day").click();
  for (const page of pages) {
    await expect(page.locator("#log")).toContainText(`${victim} dies`);
    await expect(page.locator("#log")).toContainText(`uses the Psychopath ability on ${victim}`);
    await expect(page.locator("#log")).not.toContainText("Ability {");
    await expect(page.locator(`#town .seat.dead[data-name="${victim}"]`)).toBeVisible();
  }

  // A visit as the Savant by someone who is not the Savant: same screen,
  // private, and they learn nothing; nobody else hears of it.
  const visitorAt = characters.findIndex((c, i) => !["Savant", "Psychopath"].includes(c) && names[i] !== victim);
  const visitor = pages[visitorAt];
  const otherAt = pages.findIndex((_, i) => i !== visitorAt && names[i] !== victim);
  const before = await pages[otherAt].locator("#log li").count();
  await visitor.locator("#day-actions").click();
  await visitor.locator("#use-savant").click();
  await expect(visitor.locator("#day-use")).toContainText("Only you see what follows");
  await visitor.locator("#confirm-day").click();
  await expect(visitor.locator("#log")).toContainText("The storyteller has nothing for you");
  await pages[otherAt].waitForTimeout(500);
  await expect(pages[otherAt].locator("#log li")).toHaveCount(before);
});
