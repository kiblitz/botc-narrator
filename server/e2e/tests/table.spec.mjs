// A whole table on phone-sized browsers: the host opens a room with no
// account, five players join by code, the host rearranges the circle and
// starts; players answer night prompts on their phones; the host enters the
// nomination and the hands.
import { test, expect } from "@playwright/test";

const PLAYERS = ["Ada", "Bram", "Cleo", "Dev", "Esme"];
// SHOTS=<dir> saves a screenshot at each step, to look at the phones.
const shot = (page, name) => process.env.SHOTS && page.screenshot({ path: `${process.env.SHOTS}/${name}.png`, fullPage: true });

async function phone(browser) {
  const context = await browser.newContext();
  return context.newPage();
}

const seatNames = (page, list) => page.locator(`${list} .seat .name`).allInnerTexts();

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

test("a table of phones: host, join, seat, start, night prompt, the host nominates and counts hands", async ({ browser }) => {
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
  await expect(host.locator("#bridge-command")).toContainText("/bridge.sh | sh -s -- ");

  // The host swaps Esme (last) with the first seat by tapping both...
  await host.locator('#seats .seat[data-name="Esme"]').click();
  await host.locator('#seats .seat[data-name="Host"]').click();
  await expect.poll(() => seatNames(players[2], "#seats")).toEqual(["Esme", "Ada", "Bram", "Cleo", "Dev", "Host"]);
  // ...and then the host with Bram.
  await host.locator('#seats .seat[data-name="Host"]').click();
  await host.locator('#seats .seat[data-name="Bram"]').click();
  const order = ["Esme", "Ada", "Host", "Cleo", "Dev", "Bram"];
  await expect.poll(() => seatNames(players[4], "#seats")).toEqual(order);

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

  // Day 1: the host enters a nomination of someone who is neither the Demon
  // nor the Saint; other phones cannot nominate or vote.
  const safe = Object.entries(characters)
    .filter(([, text]) => !/Demon|Saint|Virgin/.test(text))
    .map(([i]) => (Number(i) === 0 ? "Host" : PLAYERS[Number(i) - 1]));
  const nominator = order.find((name) => name !== safe[0]);
  for (const page of players) await expect(page.locator("#town button.seat")).toHaveCount(0);
  // A mistaken nominator is cancelled; then nominator, nominee, confirm.
  await host.getByRole("button", { name: `${safe[0]} nominates` }).click();
  await host.locator("#cancel-nomination").click();
  await expect(host.locator("#nomination")).toHaveCount(0);
  await host.getByRole("button", { name: `${nominator} nominates` }).click();
  await host.getByRole("button", { name: `Nominate ${safe[0]}` }).click();
  await expect(host.locator("#nomination")).toContainText(`${nominator} nominates ${safe[0]}`);
  await host.locator("#confirm-nomination").click();
  await expect(host.locator("#vote")).toBeVisible();
  for (const page of players) {
    await expect(page.locator("#vote")).toBeVisible();
    await expect(page.locator("#vote button")).toHaveCount(0);
    await expect(page.locator("#town button.seat")).toHaveCount(0);
  }
  // The host raises every hand round the circle (lowering one mistake);
  // everyone sees the hands; the host closes the vote.
  await host.getByRole("button", { name: `Raise ${order[0]}'s hand` }).click();
  await host.getByRole("button", { name: `Lower ${order[0]}'s hand` }).click();
  await expect(host.locator("#hand-count")).toContainText("0 hands");
  for (const name of order) await host.getByRole("button", { name: `Raise ${name}'s hand` }).click();
  await expect(host.locator("#hand-count")).toContainText("6 hands");
  await expect(players[1].locator("#town .hand")).toHaveCount(6);
  await shot(host, "5-vote");
  await host.locator("#close-vote").click();
  await expect(host.locator("#log")).toContainText(`6 votes for ${safe[0]}`);
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
