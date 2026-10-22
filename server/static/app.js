// The phone client: join or host a room, then follow the server's view of
// it over one WebSocket. The server sends this player's whole view after
// every change; this file only draws it and sends taps back.
"use strict";

// Official character icons, hotlinked from the townsquare (clocktower.online)
// repository at a pinned commit, as the simulator does; never copied here.
const ICON_BASE = "https://raw.githubusercontent.com/bra1n/townsquare/d9c2b17dc9b2d7b091b55c988707784f12980f44/src/assets/icons/";
const iconUrl = (character) => `${ICON_BASE}${character.toLowerCase().replace(/[^a-z]/g, "")}.png`;

const SEAT_KEY = "botc-seat";
const REVEAL_KEY = "botc-reveal";
const RECONNECT_MS = 1500;

const app = document.getElementById("app");
const state = {
  view: null,
  socket: null,
  error: "",
  // The prompt being answered and the players picked so far.
  picking: { prompt: null, picked: [] },
  reveal: localStorage.getItem(REVEAL_KEY) === "1",
  countdown: null,
  // Scripts fetched for the lobby's script view, by id (null while being
  // fetched, false if it could not be), and whether it is open.
  scripts: {},
  scriptOpen: false,
  // The lobby seat the host tapped first, to swap with the next one.
  selectedSeat: null,
  // While a seat is being dragged (redraws wait), and just after its drop.
  dragging: false,
  dragged: false,
};

// ---------- helpers ----------

function el(tag, attrs = {}, ...children) {
  const node = document.createElement(tag);
  for (const [key, value] of Object.entries(attrs)) {
    if (value === undefined || value === null || value === false) continue;
    if (key.startsWith("on")) node.addEventListener(key.slice(2), value);
    else if (key === "class") node.className = value;
    else if (value === true) node.setAttribute(key, "");
    else node.setAttribute(key, value);
  }
  for (const child of children.flat()) {
    if (child === null || child === undefined || child === false) continue;
    node.append(typeof child === "string" || typeof child === "number" ? String(child) : child);
  }
  return node;
}

function icon(character, cls) {
  const img = el("img", { class: cls, alt: "", src: iconUrl(character), loading: "lazy" });
  img.addEventListener("error", () => img.remove());
  return img;
}

function savedSeat() {
  try {
    return JSON.parse(localStorage.getItem(SEAT_KEY));
  } catch {
    return null;
  }
}

function send(message) {
  if (state.socket && state.socket.readyState === WebSocket.OPEN) {
    state.socket.send(JSON.stringify(message));
  }
}

async function post(path, body) {
  const response = await fetch(path, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(body),
    credentials: "same-origin",
  });
  const data = await response.json().catch(() => ({}));
  if (!response.ok) throw new Error(data.error || `Something went wrong (${response.status}).`);
  return data;
}

/** Text with the players and characters it names highlighted. */
function highlighted(text, players, characters) {
  const names = [
    ...players.map((n) => ({ n, kind: "who" })),
    ...characters.map((n) => ({ n, kind: "what" })),
  ].filter((x) => x.n).sort((a, b) => b.n.length - a.n.length);
  const parts = [];
  let rest = text;
  while (rest.length) {
    let best = null;
    for (const name of names) {
      const i = rest.indexOf(name.n);
      if (i >= 0 && (best === null || i < best.i)) best = { i, ...name };
    }
    if (!best) {
      parts.push(rest);
      break;
    }
    if (best.i > 0) parts.push(rest.slice(0, best.i));
    const span = el("span", { class: best.kind });
    if (best.kind === "what") span.append(icon(best.n));
    span.append(best.n);
    parts.push(span);
    rest = rest.slice(best.i + best.n.length);
  }
  return parts;
}

// ---------- connection ----------

function connect(seat) {
  const scheme = location.protocol === "https:" ? "wss" : "ws";
  const socket = new WebSocket(`${scheme}://${location.host}/ws/play`);
  state.socket = socket;
  socket.addEventListener("open", () => {
    socket.send(JSON.stringify({ type: "hello", room: seat.room, token: seat.token }));
  });
  socket.addEventListener("message", (event) => {
    const message = JSON.parse(event.data);
    if (message.type === "view") {
      const before = state.view;
      state.view = message;
      state.error = "";
      if (message.prompt && (!before || !before.prompt || before.prompt.id !== message.prompt.id)) {
        if (navigator.vibrate) navigator.vibrate([120, 80, 120]);
      }
      render();
    } else if (message.type === "error") {
      state.error = message.message;
      render();
    } else if (message.type === "gone") {
      localStorage.removeItem(SEAT_KEY);
      state.view = null;
      state.socket = null;
      state.error = message.message;
      socket.close();
      render();
    }
  });
  socket.addEventListener("close", () => {
    if (state.socket !== socket) return;
    // Phones sleep and drop connections: come back with the same seat.
    setTimeout(() => {
      const again = savedSeat();
      if (state.socket === socket && again) connect(again);
    }, RECONNECT_MS);
  });
}

function seatAt(room, token) {
  localStorage.setItem(SEAT_KEY, JSON.stringify({ room, token }));
  connect({ room, token });
}

function leave() {
  if (!confirm("Leave this room on this phone? You can only rejoin before the game starts.")) return;
  localStorage.removeItem(SEAT_KEY);
  const socket = state.socket;
  state.socket = null;
  state.view = null;
  if (socket) socket.close();
  render();
}

// ---------- screens ----------

function render() {
  // A seat being dragged would be redrawn away; the drop redraws.
  if (state.dragging) return;
  clearInterval(state.countdown);
  const view = state.view;
  document.body.className = view ? view.stage : "";
  app.replaceChildren(...(view ? game(view) : home()));
}

function home() {
  const params = new URLSearchParams(location.search);
  const joinError = el("p", { class: "error" }, state.error);
  const code = el("input", {
    class: "code", id: "join-code", maxlength: 5, autocomplete: "off", autocapitalize: "characters",
    placeholder: "ROOM", value: params.get("join") || "",
  });
  const name = el("input", { id: "join-name", maxlength: 20, autocomplete: "nickname", placeholder: "Your name" });
  const join = el("form", {
    class: "card stack",
    onsubmit: async (event) => {
      event.preventDefault();
      try {
        const seated = await post(`/api/rooms/${encodeURIComponent(code.value.trim().toUpperCase())}/join`, { name: name.value });
        state.error = "";
        seatAt(seated.room, seated.token);
      } catch (e) {
        joinError.textContent = e.message;
      }
    },
  },
    el("h2", {}, "Join a game"),
    el("label", { for: "join-code" }, "Room code"), code,
    el("label", { for: "join-name" }, "Your name"), name,
    el("button", { class: "primary", type: "submit" }, "Join"),
    joinError,
  );

  const hostError = el("p", { class: "error" });
  const hostName = el("input", { id: "host-name", maxlength: 20, autocomplete: "nickname", placeholder: "Your name" });
  const hosting = el("form", {
    class: "card stack",
    onsubmit: async (event) => {
      event.preventDefault();
      try {
        const seated = await post("/api/rooms", { name: hostName.value });
        seatAt(seated.room, seated.token);
      } catch (e) {
        hostError.textContent = e.message;
      }
    },
  },
    el("h2", {}, "Host a game"),
    el("p", { class: "muted small" }, "The host plays too; the storyteller is the app. No account needed."),
    el("label", { for: "host-name" }, "Your name at the table"), hostName,
    el("button", { type: "submit" }, "Host a game"),
    hostError,
  );
  return [el("h1", {}, "Blood on the Clocktower"), join, hosting];
}

function game(view) {
  if (view.stage === "lobby") return lobby(view);
  const parts = [header(view)];
  if (view.failure) parts.push(el("div", { class: "card alert" }, `The game stopped: ${view.failure}`));
  if (view.stage === "over") parts.push(over(view));
  else parts.push(token(view));
  if (view.prompt) parts.push(prompt(view));
  else if (view.stage === "night" && !(view.me && view.me.awake)) parts.push(eyesClosed(view));
  if (view.thinking) parts.push(el("p", { class: "thinking" }, "The storyteller is thinking…"));
  if (view.vote) parts.push(vote(view));
  if (view.stage === "dusk") parts.push(el("p", { class: "banner" }, "The day is over. Waiting for the host to start the night."));
  if (view.day_actions.length) parts.push(dayActions(view));
  if (["day", "vote", "dusk", "over"].includes(view.stage)) parts.push(town(view));
  if (state.error) parts.push(el("p", { class: "error center" }, state.error));
  parts.push(log(view));
  if (view.host) parts.push(hostDock(view));
  return parts;
}

function header(view) {
  return el("div", { class: "row" },
    el("div", {},
      el("div", { class: "banner", id: "phase" }, view.phase || "Setting up"),
      el("div", { class: "muted small center" }, `Room ${view.room} · ${view.script.name}`),
    ),
  );
}

function lobby(view) {
  const parts = [
    el("div", { class: "card center" },
      el("div", { class: "muted small" }, "Room code"),
      el("div", { class: "code-big", id: "room-code" }, view.room),
      el("div", { class: "muted small" }, `Others join at ${location.host} with this code`),
    ),
  ];
  const count = view.seats.length;
  const max = view.script.max_players;
  parts.push(el("div", { class: "card" },
    el("h2", {}, `Seating, clockwise (${count} ${count === 1 ? "player" : "players"})`),
    lobbyCircle(view),
    view.host ? seatTools(view) : null,
  ));
  if (view.host) {
    const select = el("select", {
      id: "script",
      onchange: (event) => send({ type: "script", id: event.target.value }),
    }, view.scripts.map((s) => el("option", { value: s.id, selected: s.id === view.script.id }, `${s.name} (5–${s.max_players})`)));
    parts.push(el("div", { class: "card" }, el("label", { for: "script" }, "Script"), select));
    parts.push(bridgeCard(view));
    const ready = count >= 5 && count <= max;
    parts.push(el("div", { class: "card stack" },
      el("button", { class: "primary", id: "start", disabled: !ready, onclick: () => send({ type: "start" }) }, "Start the game"),
      ready ? null : el("p", { class: "muted small center" }, `${view.script.name} needs 5 to ${max} players.`),
    ));
  } else {
    parts.push(el("p", { class: "muted center" }, `Waiting for the host to start ${view.script.name}…`));
  }
  parts.push(scriptView(view.script));
  if (state.error) parts.push(el("p", { class: "error center" }, state.error));
  parts.push(el("button", { onclick: leave }, "Leave room"));
  return parts;
}

const KIND_ORDER = ["Townsfolk", "Outsider", "Minion", "Demon", "Traveller", "Fabled", "Loric"];

/** The chosen script's characters by kind, with icons and abilities; for everyone. */
function scriptView(choice) {
  const script = state.scripts[choice.id];
  if (script === undefined) {
    state.scripts[choice.id] = null;
    fetch(`/api/scripts/${encodeURIComponent(choice.id)}`)
      .then((r) => (r.ok ? r.json() : null))
      .then((fetched) => {
        if (!fetched) throw new Error("no such script");
        state.scripts[choice.id] = fetched;
        render();
      })
      .catch(() => {
        state.scripts[choice.id] = false;
        render();
      });
  }
  const sections = KIND_ORDER.map((kind) => {
    const members = script ? script.characters.filter((c) => c.kind === kind) : [];
    if (members.length === 0) return null;
    return el("section", { class: `kind ${kind.toLowerCase()}` },
      el("h3", {}, kind === "Townsfolk" ? kind : `${kind}s`),
      ...members.map((c) => el("div", { class: "script-character" },
        icon(c.name),
        el("div", {}, el("strong", {}, c.name), el("p", {}, c.ability)),
      )),
    );
  });
  const details = el("details", { class: "card", id: "script-view" },
    el("summary", {}, `Characters on ${choice.name}`),
    script ? null : el("p", { class: "muted small" }, script === false ? "Could not load the script." : "Loading…"),
    ...sections,
  );
  details.open = state.scriptOpen;
  details.addEventListener("toggle", () => (state.scriptOpen = details.open));
  return details;
}

/**
 * Seats drawn clockwise from the top around a circle, as the players sit.
 * Seats are as large as fits between neighbours (a fifth of the circle at
 * most); sizes are percentages of the circle, so it fits any phone.
 */
function circle(id, seats, seatNode, centre) {
  const n = seats.length;
  const gap = Math.sin(Math.PI / Math.max(n, 2));
  // A seat's width d and the ring's radius r = 50 - d/2 (percent): seats
  // span 85% of the distance between neighbouring centres, 2r·sin(π/n).
  const d = Math.min(21, (85 * gap) / (1 + 0.85 * gap));
  const r = 50 - d / 2;
  const ring = el("div", { class: "circle", id, style: `--d:${d.toFixed(2)}` }, el("div", { class: "centre" }, centre));
  seats.forEach((seat, i) => {
    const angle = (2 * Math.PI * i) / n;
    const node = seatNode(seat, i);
    node.style.left = `${(50 + r * Math.sin(angle) - d / 2).toFixed(2)}%`;
    node.style.top = `${(50 - r * Math.cos(angle) - d / 2).toFixed(2)}%`;
    ring.append(node);
  });
  return ring;
}

/** A seat's face: connection light, its icon if known (else its number), name. */
function seatFace(seat, i) {
  return [
    el("span", { class: `dot${seat.connected ? " on" : ""}` }),
    seat.character ? icon(seat.character, "seat-icon") : el("span", { class: "num" }, i + 1),
    el("span", { class: "name" }, seat.name),
  ];
}

function lobbyCircle(view) {
  const selected = view.host && view.seats.some((s) => s.name === state.selectedSeat) ? state.selectedSeat : null;
  const hint = !view.host
    ? "Waiting for the host"
    : selected ? `Tap where ${selected} sits` : "Tap two seats to swap them, or drag one onto another";
  const ring = circle("seats", view.seats, (seat, i) => el(view.host ? "button" : "div", {
    class: `seat${seat.host ? " host" : ""}${seat.name === view.you ? " you" : ""}${seat.name === selected ? " selected" : ""}`,
    "data-name": seat.name,
    "aria-label": view.host ? `Seat ${i + 1}: ${seat.name}` : null,
  }, seatFace(seat, i)), el("span", { class: "small muted" }, hint));
  if (view.host) swappable(view, ring);
  return ring;
}

/** The selected seat's actions, for the host. */
function seatTools(view) {
  const name = state.selectedSeat;
  const seat = view.seats.find((s) => s.name === name);
  if (!seat) return null;
  return el("div", { class: "row seat-tools" },
    seat.host ? null : el("button", {
      onclick: () => {
        if (!confirm(`Remove ${name} from the room?`)) return;
        state.selectedSeat = null;
        send({ type: "remove", name });
      },
    }, `Remove ${name}`),
    el("button", { onclick: () => { state.selectedSeat = null; render(); } }, "Cancel"),
  );
}

function swap(view, a, b) {
  const order = view.seats.map((s) => s.name);
  const i = order.indexOf(a);
  const j = order.indexOf(b);
  state.selectedSeat = null;
  if (i < 0 || j < 0 || i === j) return;
  [order[i], order[j]] = [order[j], order[i]];
  send({ type: "reseat", order });
}

/**
 * The host swaps two seats by tapping one and then the other, or by
 * dragging one onto another. Redraws wait while a seat is being dragged.
 */
function swappable(view, ring) {
  ring.addEventListener("click", (event) => {
    const seat = event.target.closest(".seat");
    if (!seat || state.dragged) return;
    const name = seat.dataset.name;
    if (state.selectedSeat && state.selectedSeat !== name) swap(view, state.selectedSeat, name);
    else state.selectedSeat = state.selectedSeat === name ? null : name;
    render();
  });
  ring.addEventListener("pointerdown", (event) => {
    const seat = event.target.closest(".seat");
    if (!seat) return;
    const start = { x: event.clientX, y: event.clientY };
    const under = (e) => document.elementsFromPoint(e.clientX, e.clientY)
      .find((n) => n !== seat && n.classList.contains("seat")) || null;
    const onMove = (e) => {
      if (!state.dragging && Math.hypot(e.clientX - start.x, e.clientY - start.y) < 12) return;
      // Held from here, so the drag survives leaving the seat (a tap is not
      // captured: its click must still reach the seat).
      if (!state.dragging) ring.setPointerCapture(e.pointerId);
      state.dragging = true;
      seat.classList.add("dragging");
      seat.style.translate = `${e.clientX - start.x}px ${e.clientY - start.y}px`;
      const onto = under(e);
      for (const s of ring.querySelectorAll(".seat")) s.classList.toggle("drop", s === onto);
    };
    const onUp = (e) => {
      ring.removeEventListener("pointermove", onMove);
      ring.removeEventListener("pointerup", onUp);
      ring.removeEventListener("pointercancel", onUp);
      if (!state.dragging) return;
      state.dragging = false;
      // The click that follows a drop is not a tap.
      state.dragged = true;
      setTimeout(() => (state.dragged = false), 0);
      const onto = e.type === "pointerup" ? under(e) : null;
      if (onto) swap(view, seat.dataset.name, onto.dataset.name);
      render();
    };
    ring.addEventListener("pointermove", onMove);
    ring.addEventListener("pointerup", onUp);
    ring.addEventListener("pointercancel", onUp);
  });
}

// The production site, which the bridge scripts default to (bridge.mjs,
// bridge.sh and bridge.ps1 each name it too).
const SITE = "https://botc.kiblitz.xyz";

/** The one-line commands that run the bridge for this room's pairing code. */
function bridgeCommands(pairing) {
  const server = location.origin === SITE ? "" : ` --server ${location.origin}`;
  return {
    sh: `curl -fsSL ${location.origin}/bridge.sh | sh -s -- ${pairing}${server}`,
    powershell: `& ([scriptblock]::Create((irm ${location.origin}/bridge.ps1))) ${pairing}${server}`,
  };
}

function bridgeCard(view) {
  const bridge = view.bridge;
  const commands = bridgeCommands(bridge.pairing);
  const command = (id, label, text) => [
    el("p", { class: "muted small" }, label),
    el("pre", { class: "command", id }, text),
    el("button", { onclick: () => navigator.clipboard && navigator.clipboard.writeText(text) }, "Copy"),
  ];
  return el("div", { class: "card stack", id: "bridge" },
    el("h2", {}, "Storyteller"),
    el("p", {},
      el("span", { class: `dot${bridge.connected ? " on" : ""}` }), " ",
      bridge.connected ? "AI connected: it makes the storyteller's choices." : "No AI connected: the storyteller chooses at random.",
    ),
    el("p", { class: "muted small" }, "To let Codex or Claude Code on a computer be the storyteller, paste one line into a terminal there (needs Node 22+):"),
    ...command("bridge-command", "macOS, Linux, Git Bash", commands.sh),
    ...command("bridge-command-powershell", "Windows PowerShell", commands.powershell),
    el("button", { onclick: () => send({ type: "new_pairing" }) }, "New code"),
  );
}

function token(view) {
  const me = view.me;
  if (!me) return el("div");
  const toggle = () => {
    state.reveal = !state.reveal;
    localStorage.setItem(REVEAL_KEY, state.reveal ? "1" : "0");
    render();
  };
  if (!state.reveal) {
    return el("div", { class: "card token", id: "token", onclick: toggle },
      el("div", { class: "hidden-face" }, "hidden"),
      el("div", {}, el("div", { class: "character" }, view.you), el("div", { class: "muted small" }, "Tap to see your character")),
    );
  }
  return el("div", { class: "card token", id: "token", onclick: toggle },
    icon(me.character),
    el("div", {},
      el("div", { class: "character", id: "my-character" }, me.character),
      el("div", { class: me.alignment }, `${me.alignment} ${me.kind}`),
      me.alive ? null : el("div", { class: "muted" }, "You are dead"),
      el("div", { class: "muted small" }, "Tap to hide"),
    ),
  );
}

function eyesClosed(view) {
  return el("div", { class: "card eyes-closed" },
    el("div", { class: "moon" }, "☾"),
    el("p", {}, `${view.phase || "Night"}: close your eyes.`),
    el("p", { class: "small" }, "Your phone buzzes if the storyteller wakes you."),
  );
}

function prompt(view) {
  const p = view.prompt;
  if (state.picking.prompt !== p.id) state.picking = { prompt: p.id, picked: [], shown: Date.now() };
  const picked = state.picking.picked;
  const shown = state.picking.shown;
  const answer = (fields) => {
    send({ type: "answer", prompt: p.id, ...fields });
    state.picking = { ...state.picking, sent: true };
    render();
  };
  const left = el("span", { class: "countdown" });
  const tick = () => {
    const s = Math.max(0, p.seconds - Math.floor((Date.now() - shown) / 1000));
    left.textContent = `${s}s`;
  };
  tick();
  state.countdown = setInterval(tick, 1000);
  const card = el("div", { class: "card alert stack", id: "prompt" },
    el("div", {}, left, el("strong", {}, "The storyteller asks you")),
    el("p", {}, p.text),
  );
  if (state.picking.sent) {
    card.append(el("p", { class: "muted" }, "Answer sent."));
    return card;
  }
  if (p.kind === "players") {
    const choices = el("div", { class: "choices" });
    for (const name of p.candidates) {
      const chosen = picked.includes(name);
      choices.append(el("button", {
        class: chosen ? "chosen" : "",
        onclick: () => {
          if (chosen) picked.splice(picked.indexOf(name), 1);
          else if (p.max === 1) picked.splice(0, picked.length, name);
          else if (picked.length < p.max) picked.push(name);
          render();
        },
      }, name));
    }
    const ok = picked.length >= p.min && picked.length <= p.max;
    const label = picked.length === 0 && p.min === 0 ? "Choose no-one" : `Confirm (${picked.length}/${p.max})`;
    card.append(choices, el("button", { class: "primary", id: "confirm", disabled: !ok, onclick: () => answer({ players: [...picked] }) }, label));
  } else if (p.kind === "character") {
    const choices = el("div", { class: "choices" });
    p.options.forEach((name, i) => choices.append(el("button", { onclick: () => answer({ option: i }) }, icon(name), name)));
    card.append(choices);
  } else {
    card.append(el("div", { class: "row" },
      el("button", { class: "primary", onclick: () => answer({ yes: true }) }, "Yes"),
      el("button", { onclick: () => answer({ yes: false }) }, "No"),
    ));
  }
  return card;
}

function vote(view) {
  const v = view.vote;
  const raised = v.hands.filter(([, up]) => up).map(([name]) => name);
  const mine = v.hands.find(([name]) => name === view.you);
  const card = el("div", { class: "card alert stack", id: "vote" },
    el("div", { class: "banner" }, v.nominator ? `${v.nominator} nominated ${v.nominee}` : `${v.nominee} is nominated`),
    el("p", { class: "center" }, `${raised.length} hand${raised.length === 1 ? "" : "s"} raised · ${v.needed} needed`),
    raised.length ? el("p", { class: "muted small center" }, raised.join(", ")) : null,
  );
  if (v.eligible) {
    card.append(el("div", { class: "row" },
      el("button", { class: mine && mine[1] ? "primary chosen" : "primary", id: "vote-yes", onclick: () => send({ type: "vote", yes: true }) }, "✋ Raise hand"),
      el("button", { class: mine && !mine[1] ? "chosen" : "", id: "vote-no", onclick: () => send({ type: "vote", yes: false }) }, "No"),
    ));
  } else {
    card.append(el("p", { class: "muted small center" }, "You have no vote."));
  }
  return card;
}

function dayActions(view) {
  return el("div", { class: "card stack" },
    el("h2", {}, "Your day ability"),
    view.day_actions.map((a) => el("button", { onclick: () => confirm(`Use the ${a.name} ability now, in front of everyone?`) && send({ type: "day_action", ability: a.ability }) }, icon(a.name), ` Use the ${a.name} ability`)),
  );
}

function town(view) {
  const hands = new Map(view.vote ? view.vote.hands : []);
  const nominee = view.vote ? view.vote.nominee : null;
  const ring = circle("town", view.seats, (seat, i) => {
    const nominate = view.may_nominate && view.nominees.includes(seat.name) && seat.name !== view.you;
    const classes = ["seat", seat.alive ? "" : "dead", seat.name === view.you ? "you" : "",
      seat.name === nominee ? "nominee" : "", nominate ? "nominable" : "", seat.alignment || ""];
    return el(nominate ? "button" : "div", {
      class: classes.filter(Boolean).join(" "),
      "data-name": seat.name,
      "aria-label": nominate ? `Nominate ${seat.name}` : null,
      title: seat.character ? `${seat.name}: ${seat.character}` : seat.name,
      onclick: nominate ? () => confirm(`Nominate ${seat.name}?`) && send({ type: "nominate", nominee: seat.name }) : null,
    },
      seatFace(seat, i),
      seat.alive ? null : el("span", { class: "shroud" }, seat.ghost_vote ? "☠ vote" : "☠"),
      hands.get(seat.name) ? el("span", { class: "hand", title: "hand raised" }, "✋") : null,
    );
  }, nominee
    ? el("span", { class: "small" }, `${nominee} is on trial`)
    : el("span", { class: "small muted" }, view.may_nominate ? "Tap a player to nominate them" : "Town square"));
  return el("div", { class: "card" },
    el("h2", {}, view.may_nominate ? "Town square: tap to nominate" : "Town square"),
    ring,
  );
}

function over(view) {
  return el("div", { class: "card alert center", id: "winner" },
    el("div", { class: `banner ${view.winner || ""}` }, view.winner ? `${view.winner} wins!` : "The game ended"),
    el("p", { class: "muted small" }, "Everyone's characters and the storyteller's notes are shown below."),
  );
}

function log(view) {
  const list = el("ol", { class: "log", id: "log" });
  for (const entry of [...view.log].reverse()) {
    if (entry.text.startsWith("===")) {
      list.append(el("li", { class: "phase" }, entry.text.replace(/=/g, "").trim()));
      continue;
    }
    list.append(el("li", { class: entry.audience.kind }, highlighted(entry.text, entry.players, entry.characters)));
  }
  return el("div", { class: "card" },
    el("h2", {}, "What you know (newest first)"),
    el("p", { class: "muted small" }, "Gold: told to you alone. Red: the storyteller's notes, shown after the game."),
    list,
    el("button", { onclick: leave, style: "margin-top:0.75rem" }, "Leave room"),
  );
}

function hostDock(view) {
  const dock = el("div", { class: "dock", id: "dock" });
  if (view.stage === "day") dock.append(el("button", { class: "primary", id: "end-day", onclick: () => confirm("End the day? The nominee on the block is executed.") && send({ type: "end_day" }) }, "End the day"));
  if (view.stage === "vote") dock.append(el("button", { class: "primary", id: "close-vote", onclick: () => send({ type: "close_vote" }) }, "Close the vote"));
  if (view.stage === "dusk") dock.append(el("button", { class: "primary", id: "start-night", onclick: () => send({ type: "start_night" }) }, "Everyone close your eyes: start the night"));
  const bridge = view.bridge;
  dock.append(el("span", { class: "small muted", style: "flex:none;align-self:center" },
    el("span", { class: `dot${bridge.connected ? " on" : ""}` }), bridge.connected ? " AI" : " random"));
  return dock;
}

// ---------- start ----------

function start() {
  const seat = savedSeat();
  if (seat) connect(seat);
  render();
}

start();
