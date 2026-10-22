// Seating is drawn as a circle, as the players sit: it fits a small portrait
// phone for any table from 5 to 15 without seats overlapping, and the host
// reorders it by tapping two seats or dragging one onto another.
import { test, expect } from "@playwright/test";

const SMALL_PHONE = { width: 375, height: 667 };

async function room(browser, guests) {
  const context = await browser.newContext({ viewport: SMALL_PHONE });
  const host = await context.newPage();
  await host.goto("/");
  await host.locator("#host-name").fill("Host");
  await host.getByRole("button", { name: "Host a game" }).click();
  const code = (await host.locator("#room-code").innerText()).trim();
  for (const name of guests) {
    const joined = await host.request.post(`/api/rooms/${code}/join`, { data: { name } });
    expect(joined.ok()).toBe(true);
  }
  await expect(host.locator("#seats .seat")).toHaveCount(guests.length + 1);
  return host;
}

const names = (count) => Array.from({ length: count }, (_, i) => `Player ${i + 1}`);
const order = (page) => page.locator("#seats .seat .name").allInnerTexts();

/** Every seat's box, after any redraw settles. */
async function boxes(page, selector) {
  const seats = page.locator(selector);
  const all = [];
  for (let i = 0; i < (await seats.count()); i++) all.push(await seats.nth(i).boundingBox());
  return all;
}

// Seats are round: two overlap when their centres are closer than their radii.
const overlap = (a, b) => Math.hypot(a.x + a.width / 2 - b.x - b.width / 2, a.y + a.height / 2 - b.y - b.height / 2) < (a.width + b.width) / 2;

for (const count of [5, 10, 15]) {
  test(`${count} seats sit around a circle on a small phone without overlapping`, async ({ browser }) => {
    const host = await room(browser, names(count - 1));
    const seats = await boxes(host, "#seats .seat");
    const ring = await host.locator("#seats").boundingBox();
    const centre = { x: ring.x + ring.width / 2, y: ring.y + ring.height / 2 };
    const radii = seats.map((b) => Math.hypot(b.x + b.width / 2 - centre.x, b.y + b.height / 2 - centre.y));
    for (const r of radii) expect(Math.abs(r - radii[0])).toBeLessThan(2);
    // The first seat is at the top and they go clockwise.
    expect(seats[0].y).toBeLessThan(seats[1].y);
    expect(seats[1].x).toBeGreaterThan(seats[0].x);
    for (const [i, a] of seats.entries()) {
      expect(a.x).toBeGreaterThanOrEqual(0);
      expect(a.x + a.width).toBeLessThanOrEqual(SMALL_PHONE.width);
      expect(a.width).toBeGreaterThanOrEqual(44);
      for (const b of seats.slice(i + 1)) expect(overlap(a, b)).toBe(false);
    }
    // Each name fits inside its seat.
    for (const name of await host.locator("#seats .seat .name").all()) {
      expect(await name.evaluate((n) => n.scrollWidth <= n.clientWidth + 1)).toBe(true);
    }
  });
}

test("the host swaps seats by tapping two of them, or by dragging one onto another", async ({ browser }) => {
  const host = await room(browser, ["Ada", "Bram", "Cleo", "Dev", "Esme"]);
  await host.locator('#seats .seat[data-name="Esme"]').click();
  await expect(host.locator('#seats .seat[data-name="Esme"]')).toHaveClass(/selected/);
  await host.locator('#seats .seat[data-name="Host"]').click();
  await expect.poll(() => order(host)).toEqual(["Esme", "Ada", "Bram", "Cleo", "Dev", "Host"]);

  const from = await host.locator('#seats .seat[data-name="Ada"]').boundingBox();
  const to = await host.locator('#seats .seat[data-name="Cleo"]').boundingBox();
  await host.mouse.move(from.x + from.width / 2, from.y + from.height / 2);
  await host.mouse.down();
  await host.mouse.move(to.x + to.width / 2, to.y + to.height / 2, { steps: 10 });
  await host.mouse.up();
  await expect.poll(() => order(host)).toEqual(["Esme", "Cleo", "Bram", "Ada", "Dev", "Host"]);

  // A selected guest can be removed.
  host.on("dialog", (d) => d.accept());
  await host.locator('#seats .seat[data-name="Dev"]').click();
  await host.getByRole("button", { name: "Remove Dev" }).click();
  await expect.poll(() => order(host)).toEqual(["Esme", "Cleo", "Bram", "Ada", "Host"]);
});
