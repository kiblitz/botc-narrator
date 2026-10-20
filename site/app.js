import * as botcWeb from "./pkg/botc_web.js";

const MIN_PLAYERS = 5;
const MAX_PLAYERS = 15;
const PLAY_INTERVAL_MS = 700;

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
let timer = null;

function load(seed, players) {
  stop();
  const trace = JSON.parse(botcWeb.simulate(seed, players));
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
  game = { seed, players, trace, awake, announced };
  pov = "";
  selected = null;
  cursor = 0;
  $("seed").value = seed;
  $("players").value = players;
  history.replaceState(null, "", `#seed=${seed}&players=${players}`);
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
  board.style.setProperty("--token", `${Math.max(36, Math.min(84, fit))}px`);
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
  if (step.player && hasSpeaker(step)) caption.append(el("span", "who", `${step.player}: `));
  caption.append(step.text);

  for (const div of board.querySelectorAll(".seat")) {
    const seat = snapshot.find((s) => s.name === div.dataset.name);
    const known = pov === "" || pov === seat.name;
    div.className = "seat";
    div.classList.toggle("evil", known && seat.alignment === "Evil");
    div.classList.toggle("hidden", !known);
    // Players only see deaths once they are announced.
    const dead = pov === "" ? !seat.alive : game.announced[index].has(seat.name);
    div.classList.toggle("dead", dead);
    div.classList.toggle("awake", awake.has(seat.name) && (pov === "" || pov === seat.name));
    div.classList.toggle("active", step.player === seat.name);
    div.classList.toggle("selected", selected === seat.name);
    div.querySelector(".token").textContent = known ? seat.character : "?";
    const reminders = div.querySelector(".reminders");
    reminders.replaceChildren(
      ...(pov === "" ? seat.reminders : []).map((r) => el("span", `reminder ${r}`, r)),
    );
    div.querySelector(".bubble")?.remove();
    if (step.player === seat.name && (step.kind === "tell" || step.kind === "ask")) {
      div.append(el("div", "bubble", step.text));
    }
  }

  renderLog(steps);
  renderKnowledge(index);
  const scrub = $("scrub");
  scrub.max = steps.length - 1;
  scrub.value = cursor;
  $("counter").textContent = `${cursor + 1} / ${steps.length}`;
  $("play").textContent = timer ? "⏸" : "▶";
}

/** Steps said to or by a player, shown with their name. */
function hasSpeaker(step) {
  return ["wake", "sleep", "tell", "ask"].includes(step.kind);
}

function logItem(step, current) {
  const li = el("li", step.audience);
  if (isPhase(step)) li.classList.add("phase");
  if (current) li.classList.add("current");
  if (step.player && hasSpeaker(step)) li.append(el("span", "who", `${step.player}: `));
  li.append(step.text);
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
  log.querySelector(".current")?.scrollIntoView({ block: "nearest" });
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
  if (next === cursor) stop();
  go(next);
}

function jumpPhase(direction) {
  const steps = visibleSteps();
  let i = cursor + direction;
  while (i > 0 && i < steps.length - 1 && !isPhase(steps[i].step)) i += direction;
  go(Math.max(0, Math.min(i, steps.length - 1)));
}

function stop() {
  clearInterval(timer);
  timer = null;
}

function togglePlay() {
  if (timer) stop();
  else timer = setInterval(() => step(1), PLAY_INTERVAL_MS);
  render();
}

function randomSeed() {
  return Math.floor(Math.random() * 2 ** 32);
}

function fromHash() {
  const params = new URLSearchParams(location.hash.slice(1));
  const seed = Number.parseInt(params.get("seed"), 10);
  const players = Number.parseInt(params.get("players"), 10);
  return {
    seed: Number.isFinite(seed) && seed >= 0 ? seed : randomSeed(),
    players: players >= MIN_PLAYERS && players <= MAX_PLAYERS ? players : 8,
  };
}

async function main() {
  await botcWeb.default();
  for (let n = MIN_PLAYERS; n <= MAX_PLAYERS; n++) $("players").add(new Option(String(n), String(n)));
  $("controls").addEventListener("submit", (e) => {
    e.preventDefault();
    load(Number($("seed").value) >>> 0, Number($("players").value));
  });
  $("random").addEventListener("click", () => load(randomSeed(), Number($("players").value)));
  $("pov").addEventListener("change", (e) => {
    // Keep the position in the game: find the last visible step at or before the current one.
    const current = visibleSteps()[cursor].index;
    pov = e.target.value;
    selected = null;
    const steps = visibleSteps();
    cursor = Math.max(0, steps.findLastIndex(({ index }) => index <= current));
    render();
  });
  $("first").addEventListener("click", () => go(0));
  $("last").addEventListener("click", () => go(visibleSteps().length - 1));
  $("prev").addEventListener("click", () => step(-1));
  $("next").addEventListener("click", () => step(1));
  $("prev-phase").addEventListener("click", () => jumpPhase(-1));
  $("next-phase").addEventListener("click", () => jumpPhase(1));
  $("play").addEventListener("click", togglePlay);
  $("scrub").addEventListener("input", (e) => go(Number(e.target.value)));
  document.addEventListener("keydown", (e) => {
    if (e.target.tagName === "INPUT" || e.target.tagName === "SELECT") return;
    if (e.key === "ArrowRight") e.shiftKey ? jumpPhase(1) : step(1);
    else if (e.key === "ArrowLeft") e.shiftKey ? jumpPhase(-1) : step(-1);
    else if (e.key === " ") { e.preventDefault(); togglePlay(); }
    else if (e.key === "Home") go(0);
    else if (e.key === "End") go(visibleSteps().length - 1);
    else return;
  });
  new ResizeObserver(sizeSeats).observe($("board"));
  const { seed, players } = fromHash();
  load(seed, players);
  document.body.dataset.ready = "true";
}

main();
