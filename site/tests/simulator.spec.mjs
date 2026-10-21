import { expect, test } from "@playwright/test";

const open = async (page, hash = "#seed=36&players=8") => {
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
  // Seed 36 / 8 players: the minion (Dev, the Spy) is woken and told the evil team.
  await page.locator("#next").click();
  await expect(page.locator('.seat[data-name="Dev"]')).toHaveClass(/awake/);
  await page.locator("#next").click();
  await expect(page.locator("#caption")).toHaveText("Storyteller → Dev: Evil: Cleo, Dev");
  await page.locator('.seat[data-name="Dev"]').click();
  await expect(page.locator("#knowledge-title")).toHaveText("What Dev knows — Spy");
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
  await expect(page.locator('.seat[data-name="Ada"] .token .label')).toHaveText("Washerwoman");
  await expect(page.locator('.seat[data-name="Dev"] .token')).toHaveText("?");
  await page.keyboard.press("End");
  const log = await page.locator("#log").innerText();
  expect(log).not.toContain("Evil: Cleo, Dev");
  expect(log).not.toContain("storyteller");
  expect(log).toContain("Gus and Cleo: one is the Empath");
});

test("loading a seed is reproducible and shareable", async ({ page }) => {
  await open(page, "#seed=123&players=12");
  await expect(page.locator(".seat")).toHaveCount(12);
  const first = await page.locator("#log").innerText();
  await page.locator("#random").click();
  await page.locator("#seed").fill("123");
  await page.locator("#players").selectOption("12");
  await page.locator("#load").click();
  await expect(page).toHaveURL(/#script=trouble_brewing&seed=123&players=12$/);
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
  // Seed 36 / 8 players: Ada dies to the demon on night 2.
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

for (const [width, height, players] of [
  [1080, 1920, 8],
  [1920, 1080, 15],
  [1280, 500, 15],
  [800, 600, 15],
  [375, 667, 15],
]) {
  test(`${players} seats fit a ${width}x${height} screen without overlapping`, async ({ page }) => {
    await page.setViewportSize({ width, height });
    await open(page, `#seed=7&players=${players}`);
    const board = await page.locator("#board").boundingBox();
    expect(board.x).toBeGreaterThanOrEqual(0);
    expect(board.x + board.width).toBeLessThanOrEqual(width);
    const tokens = [];
    for (const token of await page.locator(".seat .token").all()) {
      const box = await token.boundingBox();
      expect(box.x).toBeGreaterThanOrEqual(0);
      expect(box.x + box.width).toBeLessThanOrEqual(width);
      expect(box.y).toBeGreaterThanOrEqual(board.y - box.height);
      expect(box.y + box.height).toBeLessThanOrEqual(board.y + board.height + box.height);
      tokens.push(box);
    }
    for (let i = 0; i < tokens.length; i++) {
      for (let j = i + 1; j < tokens.length; j++) {
        const [a, b] = [tokens[i], tokens[j]];
        const overlap = a.x < b.x + b.width && b.x < a.x + a.width && a.y < b.y + b.height && b.y < a.y + a.height;
        expect(overlap, `tokens ${i} and ${j} overlap`).toBe(false);
      }
    }
  });
}

test("townsfolk and outsiders get different shades of blue", async ({ page }) => {
  await open(page);
  const border = (name) =>
    page.locator(`.seat[data-name="${name}"] .token`).evaluate((e) => getComputedStyle(e).borderTopColor);
  // Seed 36 / 8 players: Ada is the Washerwoman, Esme the Recluse.
  expect(await border("Ada")).toBe("rgb(47, 104, 196)");
  expect(await border("Esme")).toBe("rgb(134, 185, 243)");
});

test("tokens show the official character icon", async ({ page }) => {
  await open(page);
  await expect(page.locator('.seat[data-name="Ada"] .token img')).toHaveAttribute(
    "src",
    /\/icons\/washerwoman\.png$/,
  );
  await page.locator("#pov").selectOption("Ada");
  await expect(page.locator('.seat[data-name="Dev"] .token img')).toHaveCount(0);
  await expect(page.locator('.seat[data-name="Dev"] .token')).toHaveAttribute("title", "Unknown character");
});

test("demon bluffs appear around the demon once told", async ({ page }) => {
  await open(page);
  // Seed 36 / 8 players: Cleo is the Imp and learns the bluffs on step 7.
  await expect(page.locator(".bluff")).toHaveCount(0);
  for (let i = 0; i < 7; i++) await page.keyboard.press("ArrowRight");
  await expect(page.locator('.seat[data-name="Cleo"] .bluff')).toHaveCount(3);
  await expect(page.locator('.seat[data-name="Cleo"] .bluff').first()).toHaveAttribute("title", /^Bluff: /);
  // The demon sees their own bluffs; nobody else does.
  await page.locator("#pov").selectOption("Cleo");
  await expect(page.locator('.seat[data-name="Cleo"] .bluff')).toHaveCount(3);
  await page.locator("#pov").selectOption("Ada");
  await page.keyboard.press("End");
  await expect(page.locator(".bluff")).toHaveCount(0);
});

for (const [width, height] of [[1080, 1920], [1400, 900], [375, 667]]) {
  test(`stepping scrolls the log, not the page, at ${width}x${height}`, async ({ page }) => {
    await page.setViewportSize({ width, height });
    await open(page);
    await page.locator("#next").click();
    await page.keyboard.press("End");
    expect(await page.evaluate(() => window.scrollY)).toBe(0);
    for (const steps of [0, 30]) {
      for (let i = 0; i < steps; i++) await page.keyboard.press("ArrowLeft");
      expect(await page.evaluate(() => window.scrollY)).toBe(0);
      const area = await page.locator("#log-area").boundingBox();
      const row = await page.locator("#log .current").boundingBox();
      expect(row.y).toBeGreaterThanOrEqual(area.y);
      expect(row.y + row.height).toBeLessThanOrEqual(area.y + area.height);
    }
  });
}

test("the stepper has only back and forward", async ({ page }) => {
  await open(page);
  await expect(page.locator("#stepper button")).toHaveCount(2);
  await page.locator("#next").click();
  await page.locator("#next").click();
  await expect(page.locator("#counter")).toHaveText(/^3 \//);
  await page.locator("#prev").click();
  await expect(page.locator("#counter")).toHaveText(/^2 \//);
});

test("votes highlight the nominee and everyone who voted", async ({ page }) => {
  await open(page);
  // Seed 36 / 8 players: on day 1, Ada nominates Bram and three players vote.
  await page.keyboard.press("Shift+ArrowRight");
  await page.keyboard.press("ArrowRight"); // nomination
  await page.keyboard.press("ArrowRight"); // vote
  await expect(page.locator("#caption")).toContainText("3 votes for Bram");
  await expect(page.locator('.seat[data-name="Bram"]')).toHaveClass(/nominee/);
  await expect(page.locator(".seat.voted")).toHaveCount(3);
  await page.keyboard.press("ArrowRight");
  await expect(page.locator(".seat.voted")).toHaveCount(0);
});

test("information highlights the players and characters it names", async ({ page }) => {
  await open(page);
  // Seed 36 / 8 players: step 13 tells Ada "Gus and Cleo: one is the Empath".
  for (let i = 0; i < 12; i++) await page.keyboard.press("ArrowRight");
  await expect(page.locator("#caption")).toContainText("one is the Empath");
  await expect(page.locator(".seat.mentioned")).toHaveCount(2);
  await expect(page.locator('.seat[data-name="Gus"]')).toHaveClass(/mentioned/);
  await expect(page.locator('.seat[data-name="Cleo"]')).toHaveClass(/mentioned/);
  await expect(page.locator("#caption .player-ref")).toHaveText(["Gus", "Cleo"]);
  await expect(page.locator("#caption .char-ref")).toHaveText("Empath");
  await expect(page.locator("#caption .char-ref img")).toHaveAttribute("src", /empath\.png$/);
  await expect(page.locator("#log .current .char-ref")).toHaveText("Empath");
  await page.keyboard.press("ArrowRight");
  await expect(page.locator(".seat.mentioned")).toHaveCount(0);
});

test("a script can be chosen and is kept in the URL", async ({ page }) => {
  await open(page);
  await expect(page.locator("#script option").first()).toHaveText("Trouble Brewing");
  await expect(page.locator("#script")).toHaveValue("trouble_brewing");
  await expect(page).toHaveURL(/#script=trouble_brewing&seed=36&players=8$/);
  // An unknown script in the URL falls back to the first one.
  await open(page, "#script=nope&seed=7&players=8");
  await expect(page.locator("#script")).toHaveValue("trouble_brewing");
});

test("arrows show which way communication goes", async ({ page }) => {
  await open(page);
  // Step 3: the storyteller tells Dev the evil team.
  await page.keyboard.press("ArrowRight");
  await page.keyboard.press("ArrowRight");
  await expect(page.locator("#arrows .flow-arrow.tell")).toHaveCount(1);
  await expect(page.locator("#caption .who")).toHaveText("Storyteller → Dev: ");
  const tell = await page.locator("#arrows .flow-arrow").evaluate((l) => ({
    x1: +l.getAttribute("x1"), y1: +l.getAttribute("y1"), x2: +l.getAttribute("x2"), y2: +l.getAttribute("y2"),
  }));
  const board = await page.locator("#board").boundingBox();
  const dev = await page.locator('.seat[data-name="Dev"] .token').boundingBox();
  const devX = dev.x + dev.width / 2 - board.x;
  // The arrow starts near the centre and ends near Dev.
  expect(Math.abs(tell.x2 - devX)).toBeLessThan(Math.abs(tell.x1 - devX));
  // A wake step has no arrow.
  await page.keyboard.press("ArrowRight");
  await expect(page.locator("#arrows .flow-arrow")).toHaveCount(0);
  // Day 1: Ada nominates Bram, player to player.
  await page.keyboard.press("Shift+ArrowRight");
  await page.keyboard.press("ArrowRight");
  await expect(page.locator("#arrows .flow-arrow.event")).toHaveCount(1);
  await expect(page.locator("#caption .who")).toHaveText("Ada → Bram: ");
});

test("the arrow is drawn above the centre text", async ({ page }) => {
  await open(page);
  // Step 13: the storyteller tells Ada (top seat), so the arrow crosses the caption.
  for (let i = 0; i < 12; i++) await page.keyboard.press("ArrowRight");
  const z = (selector) => page.locator(selector).evaluate((e) => Number(getComputedStyle(e).zIndex));
  expect(await z("#arrows")).toBeGreaterThan(await z("#center"));
  expect(await z("#arrows")).toBeGreaterThan(await z(".seat >> nth=0"));
  // Whatever is at the arrow's midpoint is the arrow, not the caption.
  const hit = await page.evaluate(() => {
    const line = document.querySelector("#arrows .flow-arrow");
    const svg = document.getElementById("arrows").getBoundingClientRect();
    const x = svg.left + (+line.getAttribute("x1") + +line.getAttribute("x2")) / 2;
    const y = svg.top + (+line.getAttribute("y1") + +line.getAttribute("y2")) / 2;
    document.getElementById("arrows").style.pointerEvents = "auto";
    line.style.pointerEvents = "stroke";
    return document.elementFromPoint(x, y)?.getAttribute("class");
  });
  expect(hit).toContain("flow-arrow");
});

test("no text overlaps the communication arrow", async ({ page }) => {
  await open(page, "#seed=7&players=15");
  // Check every tell/ask/nomination step of the first night and day.
  for (let i = 0; i < 60; i++) {
    await page.keyboard.press("ArrowRight");
    if ((await page.locator("#arrows .flow-arrow").count()) === 0) continue;
    const blocked = await page.evaluate(() => {
      const line = document.querySelector("#arrows .flow-arrow");
      const svg = document.getElementById("arrows").getBoundingClientRect();
      const [x1, y1, x2, y2] = ["x1", "y1", "x2", "y2"].map((a) => +line.getAttribute(a));
      // Make the arrow hit-testable, then sample along it: the topmost thing
      // must be the arrow itself, never text or a seat.
      document.getElementById("arrows").style.pointerEvents = "auto";
      line.style.pointerEvents = "stroke";
      try {
        for (let t = 0.1; t <= 0.9; t += 0.1) {
          const el = document.elementFromPoint(svg.left + x1 + (x2 - x1) * t, svg.top + y1 + (y2 - y1) * t);
          if (el && !el.closest("#arrows")) return el.textContent || el.tagName;
        }
        return null;
      } finally {
        document.getElementById("arrows").style.pointerEvents = "";
        line.style.pointerEvents = "";
      }
    });
    expect(blocked, `step ${i + 2}`).toBeNull();
  }
});

test("every script can be picked and played to the end", async ({ page }) => {
  await open(page);
  const options = page.locator("#script option");
  const ids = await options.evaluateAll((os) => os.map((o) => o.value));
  expect(ids.length).toBeGreaterThan(1);
  for (const id of ids) {
    await page.locator("#script").selectOption(id);
    await page.locator("#load").click();
    await expect(page).toHaveURL(new RegExp(`#script=${id}&seed=36&players=8$`));
    // 8 residents, plus any Travellers the script offers.
    expect(await page.locator(".seat").count()).toBeGreaterThanOrEqual(8);
    await page.keyboard.press("End");
    await expect(page.locator("#log .winner")).toHaveText(/^Game over: /);
  }
});

test("the script view lists every character with its ability", async ({ page }) => {
  await open(page);
  await page.locator("#show-script").click();
  await expect(page.locator("#script-view")).toBeVisible();
  await expect(page.locator("#script-title")).toHaveText("Trouble Brewing");
  await expect(page.locator(".script-character")).toHaveCount(22);
  await expect(page.locator(".kind.townsfolk .script-character")).toHaveCount(13);
  await expect(page.locator(".script-character").first()).toContainText("Washerwoman");
  await expect(page.locator(".script-character").first()).toContainText("1 of 2 players is a particular Townsfolk");
  await page.locator("#close-script").click();
  await expect(page.locator("#script-view")).toBeHidden();
  // Another script shows its own list.
  await page.locator("#script").selectOption("bad_moon_rising");
  await page.locator("#load").click();
  await page.locator("#show-script").click();
  await expect(page.locator("#script-title")).toHaveText("Bad Moon Rising");
  await expect(page.locator(".script-character").filter({ hasText: "Zombuul" })).toHaveCount(1);
});

test("custom names are used for the seats and kept in the URL", async ({ page }) => {
  await open(page);
  await page.locator("#names").fill("Alice, Bob, Carol");
  await page.locator("#load").click();
  await expect(page.locator(".seat").nth(0)).toHaveAttribute("data-name", "Alice");
  await expect(page.locator(".seat").nth(2)).toHaveAttribute("data-name", "Carol");
  await expect(page).toHaveURL(/names=Alice%2CBob%2CCarol/);
  await page.reload();
  await expect(page.locator("body")).toHaveAttribute("data-ready", "true");
  await expect(page.locator("#names")).toHaveValue("Alice,Bob,Carol");
  await expect(page.locator(".seat").nth(1)).toHaveAttribute("data-name", "Bob");
});
