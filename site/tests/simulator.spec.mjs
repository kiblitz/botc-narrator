import { expect, test } from "@playwright/test";

const open = async (page, hash = "#seed=7&players=8") => {
  await page.goto(`/${hash}`);
  await expect(page.locator("body")).toHaveAttribute("data-ready", "true");
};

test("seats every player in a circle", async ({ page }) => {
  await open(page);
  await expect(page.locator(".seat")).toHaveCount(8);
  await expect(page.locator("#phase")).toHaveText("Night 1");
  await expect(page.locator("#counter")).toHaveText(/^1 \/ \d+$/);
});

test("stepping reveals private information to the right player", async ({ page }) => {
  await open(page);
  // Seed 7 / 8 players: the first minion (Cleo, the Spy) is woken and told the evil team.
  await page.locator("#next").click();
  await expect(page.locator('.seat[data-name="Cleo"]')).toHaveClass(/awake/);
  await page.locator("#next").click();
  await expect(page.locator('.seat[data-name="Cleo"] .bubble')).toHaveText("Evil: Cleo, Dev");
  await page.locator('.seat[data-name="Cleo"]').click();
  await expect(page.locator("#knowledge-title")).toHaveText("What Cleo knows — Spy");
  await expect(page.locator("#knowledge-list")).toContainText("Evil: Cleo, Dev");
});

test("keyboard and phase jumps move through the game", async ({ page }) => {
  await open(page);
  await page.keyboard.press("Shift+ArrowRight");
  await expect(page.locator("#phase")).toHaveText("Day 1");
  await page.keyboard.press("End");
  await expect(page.locator("#log .winner")).toHaveText(/^Game over: /);
  await page.keyboard.press("Home");
  await expect(page.locator("#counter")).toHaveText(/^1 \//);
});

test("a player's point of view hides other characters and private steps", async ({ page }) => {
  await open(page);
  await page.locator("#pov").selectOption("Ada");
  await expect(page.locator('.seat[data-name="Ada"] .token')).toHaveText("Washerwoman");
  await expect(page.locator('.seat[data-name="Dev"] .token')).toHaveText("?");
  await page.keyboard.press("End");
  const log = await page.locator("#log").innerText();
  expect(log).not.toContain("Evil: Cleo, Dev");
  expect(log).not.toContain("storyteller");
  expect(log).toContain("Bram and Gus: one is the Investigator");
});

test("loading a seed is reproducible and shareable", async ({ page }) => {
  await open(page, "#seed=123&players=12");
  await expect(page.locator(".seat")).toHaveCount(12);
  const first = await page.locator("#log").innerText();
  await page.locator("#random").click();
  await page.locator("#seed").fill("123");
  await page.locator("#players").selectOption("12");
  await page.locator("#load").click();
  await expect(page).toHaveURL(/#seed=123&players=12$/);
  expect(await page.locator("#log").innerText()).toBe(first);
});

test("in a player's view, other players' private info stays hidden", async ({ page }) => {
  await open(page);
  await page.locator("#pov").selectOption("Ada");
  await page.keyboard.press("End");
  // Clicking another seat must not reveal what they were told.
  await page.locator('.seat[data-name="Cleo"]').click();
  await expect(page.locator("#knowledge-title")).toHaveText("What Ada knows — Washerwoman");
  await expect(page.locator("#knowledge-list")).not.toContainText("Evil: Cleo, Dev");
});

test("night deaths are announced at dawn without their cause", async ({ page }) => {
  await open(page);
  await page.locator("#pov").selectOption("Hana");
  // Seed 7 / 8 players: Ada dies to the demon on night 2.
  await page.keyboard.press("Shift+ArrowRight"); // Day 1
  await page.keyboard.press("Shift+ArrowRight"); // Night 2
  await page.keyboard.press("Shift+ArrowRight"); // Day 2
  await expect(page.locator("#phase")).toHaveText("Day 2");
  await page.keyboard.press("ArrowLeft");
  await expect(page.locator('.seat[data-name="Ada"]')).not.toHaveClass(/dead/);
  await page.keyboard.press("ArrowRight");
  await page.keyboard.press("ArrowRight");
  await expect(page.locator('.seat[data-name="Ada"]')).toHaveClass(/dead/);
  const log = await page.locator("#log").innerText();
  expect(log).toContain("Ada died in the night");
  expect(log).not.toContain("(Demon)");
});

for (const [width, height] of [[1080, 1920], [1920, 1080], [800, 600]]) {
  test(`the board fits a ${width}x${height} screen`, async ({ page }) => {
    await page.setViewportSize({ width, height });
    await open(page);
    const board = await page.locator("#board").boundingBox();
    expect(board.x).toBeGreaterThanOrEqual(0);
    expect(board.x + board.width).toBeLessThanOrEqual(width);
    for (const seat of await page.locator(".seat").all()) {
      const box = await seat.boundingBox();
      expect(box.x).toBeGreaterThanOrEqual(0);
      expect(box.x + box.width).toBeLessThanOrEqual(width);
    }
  });
}
