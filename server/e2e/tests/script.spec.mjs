// Everyone in the lobby can read the chosen script: its characters by kind,
// with icons and abilities, following the host's choice as it changes.
import { test, expect } from "@playwright/test";

test("the lobby shows the chosen script to everyone and follows the host's choice", async ({ browser }) => {
  const host = await (await browser.newContext()).newPage();
  await host.goto("/");
  await host.locator("#host-name").fill("Host");
  await host.getByRole("button", { name: "Host a game" }).click();
  const code = (await host.locator("#room-code").innerText()).trim();

  const player = await (await browser.newContext()).newPage();
  await player.goto(`/?join=${code}`);
  await player.locator("#join-name").fill("Ada");
  await player.getByRole("button", { name: "Join" }).click();

  const script = player.locator("#script-view");
  await script.locator("summary").click();
  await expect(script.locator("h3")).toHaveText(["Townsfolk", "Outsiders", "Minions", "Demons"]);
  const washerwoman = script.locator(".script-character", { hasText: "Washerwoman" });
  await expect(washerwoman).toContainText("You start knowing that 1 of 2 players is a particular Townsfolk.");
  await expect(washerwoman.locator("img")).toHaveAttribute("src", /washerwoman\.png$/);

  await host.locator("#script").selectOption("bad_moon_rising");
  // Still open, now on the new script.
  await expect(script.locator("summary")).toContainText("Bad Moon Rising");
  await expect(script.locator(".script-character", { hasText: "Zombuul" })).toBeVisible();
  await expect(script.locator(".script-character", { hasText: "Washerwoman" })).toHaveCount(0);
});
