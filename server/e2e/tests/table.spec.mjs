// A whole table on phone-sized browsers: the host opens a room with no
// account, five players join by code, the host rearranges the circle and
// starts; players answer night prompts, nominate and vote from their phones.
import { test, expect } from "@playwright/test";

const PLAYERS = ["Ada", "Bram", "Cleo", "Dev", "Esme"];
// SHOTS=<dir> saves a screenshot at each step, to look at the phones.
const shot = (page, name) => process.env.SHOTS && page.screenshot({ path: `${process.env.SHOTS}/${name}.png`, fullPage: true });

async function phone(browser) {
  const context = await browser.newContext();
  return context.newPage();
}

const seatNames = (page, list) => page.locator(`${list} .seat .name`).allInnerTexts();
const clean = (names) => names.map((n) => n.replace(" (you)", "").trim());

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

/** Answers prompts on every phone until the host's phase is `phase` (or the game is over). */
async function playUntil(pages, phase) {
  let answered = 0;
  for (let i = 0; i < 400; i++) {
    const shown = await pages[0].locator("#phase").innerText().catch(() => "");
    if (shown === phase || (await pages[0].locator("#winner").isVisible())) return answered;
    for (const page of pages) if (await answerPrompt(page).catch(() => false)) answered++;
    await pages[0].waitForTimeout(150);
  }
  throw new Error(`never reached ${phase}`);
}

test("a table of phones: host, join, seat, start, night prompt, nominate and vote", async ({ browser }) => {
  const host = await phone(browser);
  await host.goto("/");
  await host.locator("#host-name").fill("Host");
  await host.getByRole("button", { name: "Host a game" }).click();
  const code = (await host.locator("#room-code").innerText()).trim();
  expect(code).toMatch(/^[A-Z]{5}$/);

  // Joining needs only the code and a name: no account either.
  const players = [];
  for (const name of PLAYERS) {
    const page = await phone(browser);
    await page.goto(`/?join=${code}`);
    await expect(page.locator("#join-code")).toHaveValue(code);
    await expect(page.getByRole("button", { name: "Host a game" })).toBeVisible();
    await page.locator("#join-name").fill(name);
    await page.getByRole("button", { name: "Join" }).click();
    await expect(page.locator("#room-code")).toHaveText(code);
    players.push(page);
  }
  await expect(host.locator("#seats .seat")).toHaveCount(6);
  // Only the host gets the controls and the bridge's pairing command.
  await expect(players[0].locator("#start")).toHaveCount(0);
  await expect(players[0].locator("#bridge")).toHaveCount(0);
  await expect(host.locator("#bridge-command")).toContainText("botc-bridge ");

  // The host drags Esme (last) to the top of the circle...
  const handle = host.locator('#seats .seat[data-name="Esme"] .handle');
  const top = host.locator('#seats .seat[data-name="Host"]');
  // A view arriving redraws the list, so a box can be missing for a moment.
  let from, to;
  await expect.poll(async () => {
    [from, to] = [await handle.boundingBox(), await top.boundingBox()];
    return Boolean(from && to);
  }).toBe(true);
  await host.mouse.move(from.x + from.width / 2, from.y + from.height / 2);
  await host.mouse.down();
  await host.mouse.move(from.x + from.width / 2, to.y + 4, { steps: 12 });
  await host.mouse.up();
  await expect.poll(async () => clean(await seatNames(players[2], "#seats"))).toEqual(["Esme", "Host", "Ada", "Bram", "Cleo", "Dev"]);
  // ...and moves Ada up one with the arrow.
  await host.getByRole("button", { name: "Move Ada up" }).click();
  const order = ["Esme", "Ada", "Host", "Bram", "Cleo", "Dev"];
  await expect.poll(async () => clean(await seatNames(players[4], "#seats"))).toEqual(order);

  await shot(host, "1-lobby-host");
  await shot(players[0], "2-lobby-player");
  await host.locator("#start").click();
  const pages = [host, ...players];
  // Every phone shows its own character, hidden until tapped.
  const characters = {};
  for (const [i, page] of pages.entries()) {
    await expect(page.locator("#token")).toBeVisible();
    await page.locator("#token").click();
    characters[i] = (await page.locator("#token").innerText());
    await expect(page.locator("#my-character")).not.toBeEmpty();
  }
  // Nobody sees anyone else's character in the town square.
  await shot(players[1], "3-night");
  const firstAnswered = await playUntil(pages, "Day 1");
  await shot(players[1], "4-day");
  await expect(host.locator("#town .seat")).toHaveCount(6);
  expect(await host.locator("#town").innerText()).not.toMatch(/Imp|Poisoner|Washerwoman/);
  await expect(host.locator("#town")).not.toContainText(" Imp");
  for (const name of order) await expect(host.locator("#town")).toContainText(name);

  // Day 1: someone who is neither the Demon nor the Saint is nominated, and
  // everyone raises a hand; the host ends the day.
  const safe = Object.entries(characters)
    .filter(([, text]) => !/Demon|Saint|Virgin/.test(text))
    .map(([i]) => (Number(i) === 0 ? "Host" : PLAYERS[Number(i) - 1]));
  const nominator = pages.find((p, i) => (i === 0 ? "Host" : PLAYERS[i - 1]) !== safe[0]);
  nominator.on("dialog", (d) => d.accept());
  await nominator.locator("#town .seat", { hasText: safe[0] }).getByRole("button", { name: "Nominate" }).click();
  await expect(host.locator("#vote")).toBeVisible();
  await shot(host, "5-vote");
  for (const page of pages) {
    await expect(page.locator("#vote")).toBeVisible();
    await page.locator("#vote-yes").click();
  }
  await expect(host.locator("#log")).toContainText(`votes for ${safe[0]}`);
  await expect(host.locator("#vote")).toHaveCount(0);
  host.on("dialog", (d) => d.accept());
  await host.locator("#end-day").click();
  await expect(host.locator("#log")).toContainText(`${safe[0]} is executed`);
  await host.locator("#start-night").click();

  // Night 2: the Demon (at least) is woken and answers on their phone.
  for (const page of pages) if (await page.locator("#prompt").isVisible()) await shot(page, "6-prompt");
  const secondAnswered = await playUntil(pages, "Day 2");
  expect(firstAnswered + secondAnswered).toBeGreaterThan(0);
  await expect(host.locator("#log")).toContainText("Night 2");
});
