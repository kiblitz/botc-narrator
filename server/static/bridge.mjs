// botc storyteller bridge: lets this computer make a room's storyteller
// choices with an LLM CLI. Fetched and run by bridge.sh / bridge.ps1, so
// nobody downloads anything by hand; needs only Node 22 or later.
//
//   node bridge.mjs <PAIRING-CODE> [--server URL] [--codex-home DIR]
//                   [--timeout SECONDS] [-- COMMAND...]
//
// The command gets each decision's prompt on stdin and prints the chosen
// option numbers. Without `-- COMMAND`, Codex is used if it is installed,
// else Claude Code. The server checks every answer, and picks at random when
// the bridge is gone or slow, so the game never waits.
//
// The same protocol as the Rust bridge (bridge/src/wire.rs).
import { spawn, spawnSync } from "node:child_process";

const DEFAULT_SERVER = "https://botc.kiblitz.xyz";
// Under the server's 90 seconds per decision.
const DEFAULT_TIMEOUT_SECONDS = 80;
const RECONNECT_MS = 3000;
const COMMANDS = [
  ["codex", "exec", "--skip-git-repo-check", "--ephemeral", "--sandbox=read-only", "-"],
  ["claude", "-p"],
];
const USAGE = "usage: node bridge.mjs <PAIRING-CODE> [--server URL] [--codex-home DIR] [--timeout SECONDS] [-- COMMAND...]";

const log = (line) => console.error(`[bridge] ${line}`);
const fail = (message) => {
  console.error(`botc-bridge: ${message}\n${USAGE}`);
  process.exit(2);
};

function parse(argv) {
  const options = { code: null, server: DEFAULT_SERVER, timeout: DEFAULT_TIMEOUT_SECONDS, command: null };
  for (let i = 0; i < argv.length; i++) {
    const arg = argv[i];
    const value = () => argv[++i] ?? fail(`${arg} needs a value`);
    if (arg === "--server") options.server = value();
    else if (arg === "--codex-home") process.env.CODEX_HOME = value();
    else if (arg === "--timeout") options.timeout = Number(value()) || fail("--timeout needs a number of seconds");
    else if (arg === "--") {
      options.command = argv.slice(i + 1);
      if (!options.command.length) fail("-- needs a command");
      break;
    } else if (arg === "-h" || arg === "--help") {
      console.log(USAGE);
      process.exit(0);
    } else if (!options.code && !arg.startsWith("-")) options.code = arg;
    else fail(`unexpected argument ${JSON.stringify(arg)}`);
  }
  if (!options.code) fail("the pairing code is missing");
  return options;
}

// On Windows, codex and claude are npm .cmd shims, which only a shell starts;
// the shell gets one command line.
const shell = process.platform === "win32";
const quote = (arg) => (/[\s"]/.test(arg) ? `"${arg.replace(/"/g, '\\"')}"` : arg);
const start = (command, options) =>
  shell ? spawn(command.map(quote).join(" "), { ...options, shell }) : spawn(command[0], command.slice(1), options);

/** The first of COMMANDS installed here, or null. */
function detect() {
  for (const command of COMMANDS) {
    const probe = shell
      ? spawnSync(`${command[0]} --version`, { shell, stdio: "ignore" })
      : spawnSync(command[0], ["--version"], { stdio: "ignore" });
    if (probe.status === 0) return command;
  }
  return null;
}

/** Runs `command` with `prompt` on stdin; resolves to its stdout. */
function answer(command, prompt, timeoutSeconds) {
  return new Promise((resolve, reject) => {
    const child = start(command, { stdio: ["pipe", "pipe", "pipe"] });
    let stdout = "";
    let stderr = "";
    const timer = setTimeout(() => {
      child.kill();
      reject(new Error(`${command[0]} took longer than ${timeoutSeconds}s`));
    }, timeoutSeconds * 1000);
    child.stdout.on("data", (chunk) => (stdout += chunk));
    child.stderr.on("data", (chunk) => (stderr += chunk));
    child.on("error", (e) => {
      clearTimeout(timer);
      reject(new Error(`could not start ${command[0]}: ${e.message}`));
    });
    child.on("close", (status) => {
      clearTimeout(timer);
      if (status === 0) resolve(stdout);
      else {
        const last = stderr.trim().split("\n").pop() || "";
        reject(new Error(`${command[0]} exited with ${status}: ${last.trim()}`));
      }
    });
    child.stdin.on("error", () => {});
    child.stdin.end(prompt);
  });
}

/** The bridge socket's address on `server` (`https://host` -> `wss://host/ws/bridge`). */
function endpoint(server) {
  return `${server.replace(/\/+$/, "").replace(/^http/, "ws")}/ws/bridge`;
}

/** Pairs and answers decisions until the connection ends: "refused" or "dropped". */
function serve(options, command) {
  return new Promise((resolve) => {
    const socket = new WebSocket(endpoint(options.server));
    // One decision at a time, in order.
    let queue = Promise.resolve();
    socket.addEventListener("open", () => socket.send(JSON.stringify({ type: "pair", code: options.code })));
    socket.addEventListener("message", (event) => {
      const message = JSON.parse(event.data);
      if (message.type === "paired") log(`paired with room ${message.room}`);
      else if (message.type === "refused") {
        log(`refused: ${message.reason}`);
        resolve("refused");
      } else if (message.type === "decide") {
        queue = queue.then(async () => {
          log(`decision ${message.id}: asking ${command[0]}`);
          const started = Date.now();
          let reply;
          try {
            const text = await answer(command, message.prompt, options.timeout);
            log(`decision ${message.id}: answered in ${((Date.now() - started) / 1000).toFixed(1)}s: ${text.trim().slice(0, 40)}`);
            reply = { type: "reply", id: message.id, text };
          } catch (e) {
            log(`decision ${message.id}: ${e.message}`);
            reply = { type: "reply", id: message.id, error: e.message };
          }
          if (socket.readyState === WebSocket.OPEN) socket.send(JSON.stringify(reply));
        });
      }
    });
    socket.addEventListener("close", () => resolve("dropped"));
    socket.addEventListener("error", () => {});
  });
}

async function main() {
  if (typeof WebSocket === "undefined") fail(`Node ${process.versions.node} has no WebSocket; install Node 22 or later`);
  const options = parse(process.argv.slice(2));
  const command = options.command ?? detect();
  if (!command) {
    log("neither codex nor claude is installed here; the storyteller will choose at random");
    process.exit(1);
  }
  log(`answering with: ${command.join(" ")}`);
  for (;;) {
    if ((await serve(options, command)) === "refused") process.exit(1);
    log("disconnected; reconnecting");
    await new Promise((r) => setTimeout(r, RECONNECT_MS));
  }
}

main();
