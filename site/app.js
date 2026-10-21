import * as botcWeb from "./pkg/botc_web.js";

const MIN_PLAYERS = 5;
const MAX_PLAYERS = 15;
// Official character icons, loaded from the townsquare (clocktower.online)
// assets (pinned to a commit) rather than copied into this repository.
const ICON_BASE = "https://raw.githubusercontent.com/bra1n/townsquare/d9c2b17dc9b2d7b091b55c988707784f12980f44/src/assets/icons/";
const iconUrl = (character) => `${ICON_BASE}${character.toLowerCase().replace(/[^a-z]/g, "")}.png`;

const isPhase = (step) => step.kind === "phase_began";
const phaseLabel = ({ kind, number }) =>
  kind === "setup" ? "Setup" : `${kind === "night" ? "Night" : "Day"} ${number}`;

const $ = (id) => document.getElementById(id);
const el = (tag, cls, text) => {
  const e = document.createElement(tag);
  if (cls) e.className = cls;
  if (text !== undefined) e.textContent = text;
  return e;
};

/** Everything about the loaded game that does not change while stepping. */
let game = null;
/** UI state: the point of view (a player name, or "" for the storyteller). */
let pov = "";
let selected = null;
/** Index into `visibleSteps()` of the current step. */
let cursor = 0;
/** The scripts the engine offers: `[{ id, name, characters: [{ id, name, kind }] }]`. */
let scripts = [];
/** Ability text by character id without underscores, loaded on first use. */
let abilities = null;

/** The custom seat names typed in the Names box, in seat order. */
function customNames() {
  return $("names").value.split(",").map((n) => n.trim()).filter((n) => n !== "");
}

function load(seed, players, script) {
  const names = customNames();
  const trace = JSON.parse(botcWeb.simulate(seed, players, script, names));
  // Who is awake, and whose deaths the table has heard about, after each step.
  const awake = [];
  const announced = [];
  let now = new Set();
  let dead = new Set();
  for (const step of trace.steps) {
    if (step.kind === "wake") now = new Set(now).add(step.player);
    if (step.kind === "sleep") { now = new Set(now); now.delete(step.player); }
    if (isPhase(step)) now = new Set();
    if (step.kind === "death") dead = new Set(dead).add(step.player);
    awake.push(now);
    announced.push(dead);
  }
  game = { seed, players, script, trace, awake, announced };
  pov = "";
  selected = null;
  cursor = 0;
  $("seed").value = seed;
  $("players").value = players;
  $("script").value = script;
  const hash = new URLSearchParams({ script, seed, players });
  if (names.length > 0) hash.set("names", names.join(","));
  history.replaceState(null, "", `#${hash}`);
  const povSelect = $("pov");
  povSelect.replaceChildren(new Option("Storyteller (everything)", ""));
  for (const seat of trace.players) povSelect.add(new Option(`${seat.name} (${seat.character})`, seat.name));
  buildBoard();
  render();
}

function visibleTo(step, who) {
  return who === "" || step.audience === "public" || (step.audience === "player" && step.player === who);
}

function visibleSteps() {
  return game.trace.steps
    .map((step, index) => ({ step, index }))
    .filter(({ step }) => visibleTo(step, pov));
}

function buildBoard() {
  const board = $("board");
  board.querySelectorAll(".seat").forEach((s) => s.remove());
  const n = game.trace.players.length;
  game.trace.players.forEach((seat, i) => {
    const angle = -Math.PI / 2 + (2 * Math.PI * i) / n;
    const div = el("div", "seat");
    div.dataset.name = seat.name;
    div.dataset.angle = String(angle);
    div.style.left = `${50 + 39 * Math.cos(angle)}%`;
    div.style.top = `${50 + 39 * Math.sin(angle)}%`;
    div.append(el("div", "token"), el("div", "name", seat.name), el("div", "reminders"));
    div.addEventListener("click", () => {
      selected = selected === seat.name ? null : seat.name;
      render();
    });
    board.append(div);
  });
  sizeSeats();
}

/** Shrinks tokens so seats never overlap around the circle. */
function sizeSeats() {
  const board = $("board");
  if (!game) return;
  const circumference = Math.PI * 2 * 0.39 * board.clientWidth;
  // Each seat is 1.35 tokens wide; keep a little gap between neighbours.
  const fit = circumference / game.trace.players.length / 1.35 / 1.1;
  board.style.setProperty("--token", `${Math.max(24, Math.min(84, fit))}px`);
}

function render() {
  const steps = visibleSteps();
  cursor = Math.max(0, Math.min(cursor, steps.length - 1));
  const { step, index } = steps[cursor];
  const snapshot = game.trace.snapshots[step.snapshot];
  const awake = game.awake[index];
  const board = $("board");
  board.classList.toggle("night", step.phase.kind === "night");
  board.classList.toggle("day", step.phase.kind === "day");
  $("phase").textContent = phaseLabel(step.phase);

  const caption = $("caption");
  caption.className = step.audience === "storyteller" ? "storyteller" : "";
  caption.replaceChildren();
  caption.append(...speaker(step));
  caption.append(annotate(step, describe(step)));

  for (const div of board.querySelectorAll(".seat")) {
    const seat = snapshot.find((s) => s.name === div.dataset.name);
    const known = pov === "" || pov === seat.name;
    div.className = "seat";
    div.classList.toggle("evil", known && seat.alignment === "Evil");
    div.classList.toggle("outsider", known && seat.kind === "Outsider");
    div.classList.toggle("hidden", !known);
    // Players only see deaths once they are announced.
    const dead = pov === "" ? !seat.alive : game.announced[index].has(seat.name);
    div.classList.toggle("dead", dead);
    div.classList.toggle("awake", awake.has(seat.name) && (pov === "" || pov === seat.name));
    div.classList.toggle("active", step.player === seat.name);
    // On a vote, the nominee is marked and everyone who raised a hand glows.
    const isVote = step.kind === "vote";
    div.classList.toggle("nominee", isVote && step.player === seat.name);
    div.classList.toggle("voted", isVote && (step.voters ?? []).includes(seat.name));
    // Players named in the information (or chosen) are marked on the board;
    // information naming everyone (the Spy's grimoire) would only add noise.
    const mentioned = step.players ?? [];
    div.classList.toggle(
      "mentioned",
      mentioned.length < snapshot.length && mentioned.includes(seat.name),
    );
    div.classList.toggle("selected", selected === seat.name);
    renderToken(div.querySelector(".token"), known ? seat.character : null);
    const reminders = div.querySelector(".reminders");
    reminders.replaceChildren(
      ...(pov === "" ? seat.reminders : []).map((r) => el("span", `reminder ${r}`, r)),
    );
    renderBluffs(div, index);
  }

  renderArrow(step);
  renderLog(steps);
  renderKnowledge(index);
  $("counter").textContent = `${cursor + 1} / ${steps.length}`;
}

/**
 * The demon's bluffs as small tokens in a crown hugging the top of the demon's
 * token (clear of the name below it), once the demon has been told them.
 */
function renderBluffs(div, index) {
  div.querySelector(".bluffs")?.remove();
  const bluffs = game.trace.bluffs;
  const visible = bluffs && div.dataset.name === bluffs.demon && index >= bluffs.step
    && (pov === "" || pov === bluffs.demon);
  if (!visible) return;
  // Bluffs stay with the demon who was told them, even after a starpass.
  const ring = el("div", "bluffs");
  const top = -Math.PI / 2;
  const spread = 0.85; // radians between bluffs
  const radius = 0.62; // token widths from the token's centre: just outside its rim
  bluffs.characters.forEach((character, i) => {
    const angle = top + (i - (bluffs.characters.length - 1) / 2) * spread;
    const token = el("div", "bluff");
    token.title = `Bluff: ${character}`;
    token.style.left = `calc(50% + ${Math.cos(angle) * radius} * var(--token, 84px))`;
    token.style.top = `calc(var(--token, 84px) / 2 + ${Math.sin(angle) * radius} * var(--token, 84px))`;
    const img = el("img");
    img.alt = character;
    img.src = iconUrl(character);
    img.addEventListener("error", () => img.replaceWith(character.slice(0, 2)));
    token.append(img);
    ring.append(token);
  });
  div.append(ring);
}

/** The character's icon with its name underneath, or "?" when unknown. */
function renderToken(token, character) {
  if (token.dataset.character === (character ?? "")) return;
  token.dataset.character = character ?? "";
  token.title = character ?? "Unknown character";
  if (!character) {
    token.replaceChildren("?");
    return;
  }
  const img = el("img", "icon");
  img.alt = "";
  img.src = iconUrl(character);
  img.addEventListener("error", () => img.remove());
  token.replaceChildren(img, el("span", "label", character));
}

/** A step's text; votes also list who raised a hand. */
function describe(step) {
  if (step.kind !== "vote") return step.text;
  const voters = step.voters ?? [];
  return `${step.text}: ${voters.length ? voters.join(", ") : "no hands"}`;
}

/**
 * `text` with the step's named players and characters marked up: players as
 * highlighted names, characters with their icon.
 */
function annotate(step, text) {
  const players = step.players ?? [];
  const characters = step.characters ?? [];
  const names = [...new Set([...players, ...characters])].sort((a, b) => b.length - a.length);
  if (names.length === 0) return text;
  const escaped = names.map((n) => n.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"));
  const pattern = new RegExp(`\\b(${escaped.join("|")})\\b`, "g");
  const out = document.createDocumentFragment();
  let last = 0;
  for (const match of text.matchAll(pattern)) {
    out.append(text.slice(last, match.index));
    const name = match[0];
    if (characters.includes(name)) {
      const ref = el("span", "char-ref");
      const img = el("img");
      img.alt = "";
      img.src = iconUrl(name);
      img.addEventListener("error", () => img.remove());
      ref.append(img, name);
      out.append(ref);
    } else {
      out.append(el("span", "player-ref", name));
    }
    last = match.index + name.length;
  }
  out.append(text.slice(last));
  return out;
}

/** Steps said to or by a player, shown with their name. */
function hasSpeaker(step) {
  return ["wake", "sleep", "tell", "ask"].includes(step.kind);
}

const endpointName = (endpoint) => (endpoint.kind === "storyteller" ? "Storyteller" : endpoint.name);

/** "From → To: " for communication, "Name: " for other player steps. */
function speaker(step) {
  if (step.flow) {
    const span = el("span", `who flow ${step.kind}`);
    span.append(endpointName(step.flow.from), el("span", "arrow", " → "), endpointName(step.flow.to), ": ");
    return [span];
  }
  if (step.player && hasSpeaker(step)) return [el("span", "who", `${step.player}: `)];
  return [];
}

/** Centre of an endpoint in board coordinates: a seat's token, or the storyteller badge. */
function endpointCentre(endpoint) {
  const board = $("board").getBoundingClientRect();
  const target = endpoint.kind === "storyteller"
    ? $("storyteller")
    : $("board").querySelector(`.seat[data-name="${CSS.escape(endpoint.name)}"] .token`);
  if (!target) return null;
  const box = target.getBoundingClientRect();
  return { x: box.left + box.width / 2 - board.left, y: box.top + box.height / 2 - board.top, r: box.height / 2 };
}

/** An arrow across the board showing which way the current step's communication goes. */
function renderArrow(step) {
  const svg = $("arrows");
  svg.querySelectorAll(".flow-arrow, .flow-halo").forEach((e) => e.remove());
  if (!step.flow) return;
  const a = endpointCentre(step.flow.from);
  const b = endpointCentre(step.flow.to);
  if (!a || !b) return;
  const length = Math.hypot(b.x - a.x, b.y - a.y);
  if (length === 0) return;
  // Start and end just outside each endpoint.
  const [ux, uy] = [(b.x - a.x) / length, (b.y - a.y) / length];
  const pad = 6;
  const line = document.createElementNS("http://www.w3.org/2000/svg", "line");
  line.setAttribute("class", `flow-arrow ${step.kind}`);
  line.setAttribute("x1", a.x + ux * (a.r + pad));
  line.setAttribute("y1", a.y + uy * (a.r + pad));
  line.setAttribute("x2", b.x - ux * (b.r + pad));
  line.setAttribute("y2", b.y - uy * (b.r + pad));
  line.setAttribute("marker-end", "url(#arrowhead)");
  const halo = line.cloneNode();
  halo.setAttribute("class", "flow-halo");
  halo.removeAttribute("marker-end");
  svg.append(halo, line);
}

function logItem(step, current) {
  const li = el("li", step.audience);
  if (isPhase(step)) li.classList.add("phase");
  if (current) li.classList.add("current");
  li.append(...speaker(step));
  li.append(annotate(step, describe(step)));
  return li;
}

function renderLog(steps) {
  const log = $("log");
  log.replaceChildren();
  steps.slice(0, cursor + 1).forEach(({ step }, i) => {
    const li = logItem(step, i === cursor);
    li.addEventListener("click", () => { cursor = i; render(); });
    log.append(li);
  });
  if (cursor === steps.length - 1) {
    const end = game.trace.winner ? `Game over: ${game.trace.winner} wins.` : "Game over: no winner within the day limit.";
    log.append(el("li", "winner", end));
  }
  // Scroll only the log panel, never the page.
  const current = log.querySelector(".current");
  const area = $("log-area");
  // `#log-area` is the rows' offsetParent, so offsetTop is panel-relative.
  if (current) area.scrollTop = current.offsetTop - area.clientHeight / 2 + current.offsetHeight / 2;
}

/** What the selected player has been told and asked so far. */
function renderKnowledge(globalIndex) {
  // In a player's view, only that player's own knowledge is visible.
  const who = pov || selected;
  const list = $("knowledge-list");
  list.replaceChildren();
  if (!who) {
    $("knowledge-title").textContent = "Click a player to see what they know";
    return;
  }
  const seat = game.trace.players.find((s) => s.name === who);
  const shown = pov === "" || pov === who ? ` — ${seat.character}` : "";
  $("knowledge-title").textContent = `What ${who} knows${shown}`;
  const private_ = game.trace.steps
    .slice(0, globalIndex + 1)
    .filter((s) => s.audience === "player" && s.player === who && (s.kind === "tell" || s.kind === "ask"));
  if (private_.length === 0) list.append(el("li", "", "Nothing private yet."));
  for (const s of private_) {
    const li = el("li", "player");
    li.append(el("span", "who", `${phaseLabel(s.phase)}: `), s.kind === "ask" ? `chose — ${s.text}` : s.text);
    list.append(li);
  }
}

function go(to) {
  cursor = to;
  render();
}

function step(delta) {
  const steps = visibleSteps();
  const next = Math.max(0, Math.min(cursor + delta, steps.length - 1));
  go(next);
}

function jumpPhase(direction) {
  const steps = visibleSteps();
  let i = cursor + direction;
  while (i > 0 && i < steps.length - 1 && !isPhase(steps[i].step)) i += direction;
  go(Math.max(0, Math.min(i, steps.length - 1)));
}

const KIND_ORDER = ["Townsfolk", "Outsider", "Minion", "Demon", "Traveller", "Fabled", "Loric"];

/** The current script's characters by kind, with icons and ability text. */
async function showScript() {
  const script = scripts.find((s) => s.id === game.script);
  abilities ??= await fetch("abilities.json").then((r) => (r.ok ? r.json() : {})).catch(() => ({}));
  $("script-title").textContent = script.name;
  const body = $("script-characters");
  body.replaceChildren();
  for (const kind of KIND_ORDER) {
    const members = script.characters.filter((c) => c.kind === kind);
    if (members.length === 0) continue;
    const section = el("section", `kind ${kind.toLowerCase()}`);
    section.append(el("h3", "", kind === "Townsfolk" ? kind : `${kind}s`));
    for (const c of members) {
      const row = el("div", "script-character");
      const img = el("img");
      img.alt = "";
      img.src = iconUrl(c.name);
      img.addEventListener("error", () => img.remove());
      const text = el("div", "");
      text.append(el("strong", "", c.name), el("p", "", abilities[c.id.replaceAll("_", "")] ?? ""));
      row.append(img, text);
      section.append(row);
    }
    body.append(section);
  }
  $("script-view").showModal();
}

function randomSeed() {
  return Math.floor(Math.random() * 2 ** 32);
}

function fromHash() {
  const params = new URLSearchParams(location.hash.slice(1));
  const seed = Number.parseInt(params.get("seed"), 10);
  const players = Number.parseInt(params.get("players"), 10);
  const script = params.get("script");
  return {
    names: params.get("names") ?? "",
    script: scripts.some((s) => s.id === script) ? script : scripts[0].id,
    seed: Number.isFinite(seed) && seed >= 0 ? seed : randomSeed(),
    players: players >= MIN_PLAYERS && players <= MAX_PLAYERS ? players : 8,
  };
}

async function main() {
  await botcWeb.default();
  scripts = JSON.parse(botcWeb.scripts());
  for (const s of scripts) $("script").add(new Option(s.name, s.id));
  for (let n = MIN_PLAYERS; n <= MAX_PLAYERS; n++) $("players").add(new Option(String(n), String(n)));
  $("controls").addEventListener("submit", (e) => {
    e.preventDefault();
    load(Number($("seed").value) >>> 0, Number($("players").value), $("script").value);
  });
  $("random").addEventListener("click", () =>
    load(randomSeed(), Number($("players").value), $("script").value));
  $("pov").addEventListener("change", (e) => {
    // Keep the position in the game: find the last visible step at or before the current one.
    const current = visibleSteps()[cursor].index;
    pov = e.target.value;
    selected = null;
    const steps = visibleSteps();
    cursor = Math.max(0, steps.findLastIndex(({ index }) => index <= current));
    render();
  });
  $("prev").addEventListener("click", () => step(-1));
  $("next").addEventListener("click", () => step(1));
  document.addEventListener("keydown", (e) => {
    if (e.target.tagName === "INPUT" || e.target.tagName === "SELECT") return;
    if (e.key === "ArrowRight") e.shiftKey ? jumpPhase(1) : step(1);
    else if (e.key === "ArrowLeft") e.shiftKey ? jumpPhase(-1) : step(-1);
    else if (e.key === "Home") go(0);
    else if (e.key === "End") go(visibleSteps().length - 1);
    else return;
    // These keys drive the game; don't let them also scroll the page.
    e.preventDefault();
  });
  // Seat sizes and arrow endpoints depend on the board size.
  new ResizeObserver(() => {
    sizeSeats();
    if (game) render();
  }).observe($("board"));
  const { script, seed, players, names } = fromHash();
  $("names").value = names;
  $("show-script").addEventListener("click", showScript);
  $("close-script").addEventListener("click", () => $("script-view").close());
  load(seed, players, script);
  document.body.dataset.ready = "true";
}

main();
