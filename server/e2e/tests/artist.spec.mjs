// The Artist's question on a phone: with a storyteller bridge paired, the
// Artist's visit asks for a typed question (anyone may visit as the Artist);
// what is typed survives the view redrawing, and only that phone shows it.
//
// The bridge here is a WebSocket in this test that answers every decision
// with its first option.
import { test, expect } from "@playwright/test";

const PLAYERS = ["Ada", "Bram", "Cleo", "Dev", "Esme"];
const QUESTION = "Is the Demon sitting next to me?";

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

/** A bridge that pairs with `code` and answers every decision with option 0. */
function bridge(baseURL, code) {
  const prompts = [];
  const socket = new WebSocket(`${baseURL.replace("http", "ws")}ws/bridge`);
  socket.addEventListener("open", () => socket.send(JSON.stringify({ type: "pair", code })));
  socket.addEventListener("message", (event) => {
    const message = JSON.parse(event.data);
    if (message.type !== "decide") return;
    prompts.push(message.prompt);
    socket.send(JSON.stringify({ type: "reply", id: message.id, text: "0" }));
  });
  return { prompts, close: () => socket.close() };
}

test("the Artist types a question on the phone, seen only there and by the storyteller", async ({ browser, baseURL }) => {
  const host = await phone(browser);
  await host.goto(baseURL);
  await host.locator("#host-name").fill("Host");
  await host.getByRole("button", { name: "Host a game" }).click();
  const code = (await host.locator("#room-code").innerText()).trim();
  const players = [];
  for (const name of PLAYERS) {
    const page = await phone(browser);
    await page.goto(`${baseURL}?join=${code}`);
    await page.locator("#join-name").fill(name);
    await page.getByRole("button", { name: "Join" }).click();
    await expect(page.locator("#room-code")).toHaveText(code);
    players.push(page);
  }
  await expect(host.locator("#seats .seat")).toHaveCount(6);
  const pairing = (await host.locator("#bridge-command").innerText()).match(/sh -s -- (\S+)/)[1];
  const storyteller = bridge(baseURL, pairing);
  try {
    await expect(host.locator("#bridge .dot.on")).toBeVisible();
    await host.locator("#script").selectOption("laissez_un_faire");
    await host.locator("#start").click();
    const pages = [host, ...players];
    await playUntil(pages, "Day 1");

    // Visit as the Artist (holder or not, the screen is the same).
    const asker = players[0];
    await asker.locator("#day-actions").click();
    await asker.locator("#use-artist").click();
    await asker.locator("#confirm-day").click();
    const field = asker.locator("#question");
    await expect(field).toBeVisible();
    await expect(asker.locator("#ask")).toBeDisabled();
    // Typed in two goes while a view arrives (another phone reconnects, so
    // every phone is redrawn): nothing typed is lost.
    await field.pressSequentially("Is the Demon ");
    await players[1].reload();
    await expect(players[1].locator("#phase")).toHaveText("Day 1");
    await asker.waitForTimeout(500);
    await field.pressSequentially("sitting next to me?");
    await expect(field).toHaveValue(QUESTION);
    await asker.locator("#ask").click();
    await expect(asker.locator("#log")).toContainText(`You asked: "${QUESTION}"`);
    await expect(asker.locator("#log")).toContainText(new RegExp(`"${QUESTION}": Yes|The storyteller has nothing for you`));
    for (const page of pages.slice(1).concat(host).filter((p) => p !== asker)) {
      await expect(page.locator("body")).not.toContainText(QUESTION);
    }
    if (!(await asker.locator("#log").innerText()).includes("nothing for you")) {
      expect(storyteller.prompts.some((p) => p.includes(`asks "${QUESTION}"`))).toBe(true);
    }
  } finally {
    storyteller.close();
  }
});
