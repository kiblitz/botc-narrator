# Deploying botc

botc runs on **slot1**, kiblitz's second shared VPS (`5.161.56.133`): one
static server binary behind the box's Caddy, which also serves the web client,
so the page, the API and the WebSockets share one origin.

The scaffolding is langbook's (`slot1/langbook/deploy`), which is bomb-squad's
(`slot0/games/bomb-squad/deploy`). The rules every app on the box follows are
under [Sharing the box](#sharing-the-box); what differs from langbook is under
[What is botc's own](#what-is-botcs-own).

## What is where

| In this directory | On the box | |
|---|---|---|
| `botc.caddy` | `/etc/caddy/sites/botc.caddy` | botc's site block, and nothing else. Synced by every deploy |
| `botc.service` | `/etc/systemd/system/botc.service` | The unit. Installed by `setup-vps.sh` only |
| `apply.sh` | `/usr/local/sbin/botc-apply` | Applies a staged deploy as root, all or nothing |
| `caddy-site.sh` | `/usr/local/sbin/botc-caddy-site` | Changes the site config under the box's Caddy lock |
| `setup-vps.sh` | — | Provisioning. Run by hand, idempotent |
| `tests/` | — | Tests for `apply.sh` and `caddy-site.sh`, and `files_agree.sh`, which holds every file here (and `server/src/route.rs`) to the same names, paths, port and routes |

```
                    botc.kiblitz.xyz
                          |
   +----------------------v-----------------------+
   | Caddy :443 (the box's; TLS for every app)    |
   |   /ws/*               -> 127.0.0.1:8081      |
   |                          (streamed sockets)  |
   |   /health /api/*      -> 127.0.0.1:8081      |
   |                          (bodies up to 64KB) |
   |   anything else       -> /var/www/botc       |
   +----------------------+-----------------------+
                          |
                 botc.service (User=botc)
                 /usr/local/bin/botc-server
                 (rooms in memory; nothing on disk)
```

The server binds loopback only, so port 8081 never needs a firewall rule.
`/ws/play` is the phones' socket and `/ws/bridge` the storyteller bridge's.
Anything that is not a file in the web client is answered with `index.html`.

## Sharing the box

slot1 hosts more than one app. These are slot0's rules, unchanged
(`slot0/CLAUDE.md`, "How apps share the box"), summarised in slot1's
`CLAUDE.md`. Change this copy together with those.

1. **One public entry point.** Only Caddy listens publicly. Every app binds
   loopback only, on its own port from slot1's `apps.md`.
2. **Caddy's config belongs to the box.** `/etc/caddy/Caddyfile` does nothing
   but `import sites/*.caddy`, installed by slot1's `box/bootstrap.sh`, never by
   an app: `setup-vps.sh` and `botc-caddy-site` refuse a box without it.
3. **Take the Caddy lock.** Whatever changes a site config and reloads Caddy
   holds `flock /run/lock/caddy-sites.lock` throughout. Mode 600; a lock
   directory other accounts can write to is refused; a site config goes in
   under a draft name that does not match `*.caddy`, then is renamed.
4. **Reload Caddy only when routing changed.** A reload closes proxied
   WebSockets for every app -- botc's own games among them.
5. **One subdomain per app**, `<name>.kiblitz.xyz`, with its own A record.
6. **Accounts per app:** a no-shell system account that runs the service, and a
   deploy account `<app>-deploy` whose sudoers entry allows exactly one
   root-owned apply script, with `Defaults!<script> env_reset, secure_path=…`.
7. **Deploys are all-or-nothing**, from private staging under
   `/var/lib/<app>-deploy/staging`: staged links and non-plain files refused,
   the shared Caddyfile validated **as the caddy user** before the binary or
   web files change, the previous site config put back on failure, a binary
   (and its web files) that does not come up rolled back. Web assets are
   unpacked fresh with `tar --no-same-owner --no-same-permissions`, anything but
   files and directories is refused, and modes are normalised.
8. **Provisioning is manual and idempotent; deploying is automatic.**
9. **Resources are tight** (2 GB RAM): every unit has `MemoryMax=`/`TasksMax=`.
10. **No sign-in.** Box apps normally sign in through the shared account
    service; botc is anonymous by the user's decision (2026-10-21), so it has
    no account client and no callback.

### botc's row in slot1's `apps.md`

| App | Repo | Hostname | Loopback port | systemd unit | Accounts | Site config | Paths |
| --- | --- | --- | --- | --- | --- | --- | --- |
| botc | `kiblitz/botc-narrator` | `botc.kiblitz.xyz` | `8081` | `botc` | `botc` (service), `botc-deploy` (deploys) | `/etc/caddy/sites/botc.caddy` | `/usr/local/bin/botc-server`, `/usr/local/sbin/botc-apply`, `/usr/local/sbin/botc-caddy-site`, `/var/www/botc`, `/var/lib/botc-deploy/staging`, `/var/lib/botc-apply`, `/var/lib/botc-caddy-site`, `/run/lock/botc-apply.lock` |

DNS: `botc A 5.161.56.133`. `apps.md` is the registry; if this copy and it
disagree, it is this copy that is out of date.

## The first time

### 1. Claim the resources

Add botc's row (above) to slot1's `apps.md`, then check the box agrees:

```sh
ssh -i ~/.ssh/slot1-admin claude@5.161.56.133 \
  'sudo ss -ltnp; ls /etc/caddy/sites; getent passwd | grep botc;
   grep -xF "import sites/*.caddy" /etc/caddy/Caddyfile'
```

Nothing should listen on 8081, no `botc*` account should exist, and the import
line should be there.

### 2. Point the name at the box

`botc A 5.161.56.133` on `kiblitz.xyz`, through the Porkbun API
(credentials `slot0/secrets/porkbun`; never print them). First, because Caddy
cannot get a certificate until the name resolves. Leave the `*` wildcard alone.

### 3. Keys

`scripts/deploy.sh` uses `~/.ssh/botc-deploy` on this machine; the workflow
gets the same key as a secret.

```sh
ssh-keygen -t ed25519 -N "" -f ~/.ssh/botc-deploy
gh secret set DEPLOY_SSH_KEY < ~/.ssh/botc-deploy
gh secret set DEPLOY_KNOWN_HOSTS --body "5.161.56.133 ssh-ed25519 AAAA…"
```

`DEPLOY_KNOWN_HOSTS` is the box's pinned host key from slot1's `CLAUDE.md`
("The box").

### 4. Provision

As `claude` on the box, from a private directory, from a checkout whose
`deploy/` is LF (`.gitattributes` pins it; a CRLF `setup-vps.sh` fails with
`env: 'bash\r'`):

```sh
stage=$(ssh -i ~/.ssh/slot1-admin claude@5.161.56.133 mktemp -d)
scp -i ~/.ssh/slot1-admin -r deploy "claude@5.161.56.133:$stage/"
ssh -i ~/.ssh/slot1-admin claude@5.161.56.133 \
  "cd $stage/deploy && sudo ./setup-vps.sh '$(cat ~/.ssh/botc-deploy.pub)'; rm -rf $stage"
```

`setup-vps.sh` refuses a box that is not on the shared Caddy layout, then
creates the `botc` service account (no shell, no home), the `botc-deploy`
account (sudo for exactly `botc-apply` and `systemctl status botc`, environment
reset, `PATH` pinned), `/var/lib/botc-deploy/staging` (mode 700), the web root
with a placeholder page, the site config (through `botc-caddy-site`),
`botc-apply`, and the unit, enabled. Re-run it after changing the unit, the
site config or either script; it restarts the service only when the unit's
bytes changed.

A unit the running binary cannot honour (a new required variable, say) must be
deployed straight after provisioning; if the service crash-loops meanwhile it
exhausts systemd's start-rate limit, and the deploy rolls back. Clear it with
`sudo systemctl reset-failed botc` and deploy again.

### 5. Deploy, and check

Merge to `main`, then `bash scripts/deploy.sh`, and:

```sh
ssh -i ~/.ssh/slot1-admin claude@5.161.56.133 \
  'curl -s http://127.0.0.1:8081/health; echo;
   sudo runuser -u caddy -- caddy validate --config /etc/caddy/Caddyfile --adapter caddyfile'
curl -s https://botc.kiblitz.xyz/health
curl -s https://botc.kiblitz.xyz/api/scripts   # JSON, not the web page
```

and check that every other app on the box still answers. Then bring slot1's
`apps.md` and `CLAUDE.md` up to date.

## Every time after

Run `bash scripts/check.sh` before merging (there is no CI on GitHub; the only
workflow is `deploy.yml`, on push to `main`). While GitHub Actions cannot run,
`scripts/deploy.sh` does the same from a clean checkout of `origin/main`
(`--dry-run` stops before staging). It refuses a worktree that is dirty, that is
not `origin/main`, or that `check.sh`'s stamp (`target/local-check/passed`,
written by `scripts/stamp.sh`) does not cover -- so after merging, run the
checks once more on `main` before deploying. The deploy builds the static musl
binary, packages `server/static/` as the web client, stages both with the site
config, and runs `sudo botc-apply` once, which:

1. refuses a staging directory that is not the deploy account's alone, copies
   what was staged where only root can reach it, and refuses anything but
   plain, non-empty files;
2. unpacks the web client into a fresh root-only directory, refuses anything in
   it but files and directories, and owns and modes it afresh;
3. has Caddy take the site config first -- validated as `caddy`, under the
   box's lock, reloaded only if the routing changed -- and stops if Caddy
   refuses it;
4. swaps the web client in, and installs the binary with one rename;
5. restarts the service if the binary changed or the service is inactive, and
   puts the previous binary and web client back if it does not come up.

The deploy then checks `/health` on loopback and in public, that `/` is the web
client, and that `/api/scripts` reaches the server rather than the static handler.

## What is botc's own

| | |
|---|---|
| Box | slot1, `5.161.56.133` |
| Hostname | `botc.kiblitz.xyz` |
| Loopback port | `8081` |
| Unit | `botc.service`, `MemoryMax=256M`, `TasksMax=128`, `LimitNOFILE=4096` |
| Unit environment | `BIND_ADDR=127.0.0.1:8081`, `LOG=info`; no `EnvironmentFile`, no `StateDirectory`, no secret |
| Accounts | `botc` (service), `botc-deploy` (deploys) |
| Request bodies | `/health` and `/api/*` capped at 64 KB by Caddy |
| Paths | `/usr/local/bin/botc-server` (+ `.previous`), `/usr/local/sbin/botc-apply`, `/usr/local/sbin/botc-caddy-site`, `/etc/sudoers.d/botc-deploy`, `/etc/systemd/system/botc.service`, `/etc/caddy/sites/botc.caddy`, `/var/www/botc` (+ `.previous`), `/var/lib/botc-apply`, `/var/lib/botc-caddy-site`, `/var/lib/botc-deploy/staging`, `/var/log/caddy/botc.kiblitz.xyz.log`, `/run/lock/botc-apply.lock` |

Differences from langbook's scaffolding:

- **WebSockets.** `/ws/*` (`/ws/play`, `/ws/bridge`) has its own matcher with
  bomb-squad's streaming settings: `flush_interval -1` and
  `stream_close_delay 5m`, so another app's Caddy reload does not cut a game
  off. A reload still closes sockets eventually, which is why
  `botc-caddy-site` skips the reload for a byte-identical site config.
  `files_agree.sh` checks that every constant in `server/src/route.rs` is
  covered by `@api` or `@ws`, and that `PLAY` and `BRIDGE` are under `@ws`.
- **Rooms live in memory, so a restart ends every game in progress.** There is
  no open-room guard in v1: `scripts/deploy.sh` does not check for running
  games. Deploy when no table is playing. A deploy that ships the same binary
  (docs, the web client) never restarts the service, and a web-client-only
  deploy leaves games running.
- **No database and no state directory**; nothing on the box outlives the
  process.
- **No build step for the web client**: `server/static/` (`index.html`,
  `app.js`, `style.css`) is packaged as it is committed.
- **`MemoryMax=256M`, `TasksMax=128`**: the engine runs one OS thread per
  active room, and the server caps rooms well under the task limit.

## Testing the scripts

```sh
bash deploy/tests/files_agree.sh                          # needs only a checkout
for t in deploy/tests/*_test.sh; do sudo bash "$t"; done  # need root: they chown
```

`scripts/check-deploy.sh` (run by `scripts/check.sh`) runs `files_agree.sh`,
`shellcheck`, `shfmt` and `caddy adapt` where installed, and copies `deploy/`
to the slot1 box to run the root tests in a throwaway `ubuntu:24.04` podman
container. If the box cannot be reached it says so loudly and carries on; the
stamp records `box_tests skipped`, and a change to `apply.sh` or
`caddy-site.sh` must wait for a run that does reach the box.

## When something is wrong

```sh
systemctl status botc
journalctl -u botc -e
journalctl -t botc-apply -e          # what each deploy did
journalctl -t botc-caddy-site -e     # and what it did to Caddy
curl -s http://127.0.0.1:8081/health
sudo runuser -u caddy -- caddy validate --config /etc/caddy/Caddyfile --adapter caddyfile
```

A socket that drops every few seconds behind Caddy but not on loopback means
the `/ws` route lost its streaming settings. After any change to the box, check
that every other app on it still answers.
