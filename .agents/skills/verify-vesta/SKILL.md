---
name: verify-vesta
description: Verify Vesta's web dashboard, first-run access, server telemetry, and Settings in an isolated local browser session; use when proving user-visible Vesta behavior.
---

# Verify Vesta

Use this skill to prove Vesta behavior through its real web UI in a disposable local instance. The primary user surface is the browser dashboard at `/` and Settings at `/settings`; the app also exposes a JSON/SSE API. The flow below uses `playwright-cli` and a clean config/database per run.

## Launch

From the repository root, choose a unique run ID and start the isolated instance:

```bash
RUN_ID="verify-$(date -u +%Y%m%dT%H%M%SZ)-$$"
./.agents/skills/verify-vesta/scripts/instance.sh start "$RUN_ID"
BASE_URL="$(./.agents/skills/verify-vesta/scripts/instance.sh url "$RUN_ID")"
SCRATCH_ROOT="${VESTA_VERIFY_STATE_ROOT:-${TMPDIR:-/tmp}/vesta-verify-${UID:-$(id -u)}}"
SCRATCH_DIR="$SCRATCH_ROOT/$RUN_ID"
EVIDENCE_ROOT="${VESTA_VERIFY_EVIDENCE_ROOT:-$PWD/artifacts/verification/vesta}"
EVIDENCE_DIR="$EVIDENCE_ROOT/$RUN_ID"
```

`start` installs the locked Bun dependencies only when `node_modules` is absent, builds the frontend and Rust binary, then launches the binary on a free loopback port. It sets `VESTA_CONFIG` and `VESTA_DATABASE` to new paths under a private temporary run directory, points `VESTA_STATIC_DIR` at the built assets, and lets SQLite apply the embedded migrations during startup. No config file is seeded: Vesta's supported first-run default is an empty dashboard, and the UI creates the admin account. Startup waits for the server's listening log and successful health, page, and asset responses before returning the URL. `VESTA_BIND`, `VESTA_COOKIE_SECURE`, and `RUST_LOG` are set for local verification; no deployment or production data is touched. A failed launch stops only its verified run process and retains scratch state if process identity cannot be confirmed.

Install Playwright's Firefox browser if it is not already cached:

```bash
bunx --package @playwright/cli playwright-cli install-browser firefox
```

## Doctor

Whenever startup or browser behavior looks wrong, run this read-only check before changing anything:

> **Harness gotcha — a Vesta process (or mock server) spawned from an ordinary shell call is reaped when that call ends.** The symptom is a silent death right after the `Ready:` line: later connections are refused, `doctor` reports "recorded PID is not this Vesta binary/run", and `server.log` ends at the listening line with no error. Start the instance and any mock servers the journeys need through the durable background-process runner available in this environment; in a plain shell call, prefix the start with `setsid` — `setsid ./.agents/skills/verify-vesta/scripts/instance.sh start "$RUN_ID"` — so the `nohup`ed server daemonizes out of the call's process group and survives it. Either way, wait for the run's `Ready:` output and confirm the listening port answers before driving the browser — then retry `doctor` only after starting a fresh run ID, since a reaped process invalidates the scratch dir.

```bash
./.agents/skills/verify-vesta/scripts/instance.sh doctor "$RUN_ID"
```

It verifies that the recorded PID is still this run's Vesta binary, `ss` attributes the listening port to that PID, `/api/health?detailed=true` reports the Cargo package version, `/settings` and both built frontend assets are served, SQLite was initialized, `/api/auth/status` is coherent for either first-run setup or an existing administrator, and unauthenticated `/api/settings` requests are rejected. A doctor failure means the instance is not ready to drive; read the preserved `server.log` and `build.log` in `$EVIDENCE_DIR`, then clean up the run before retrying.

The Playwright session itself can also restart between driver calls (a new browser pid in the output); its HTTP-only session cookie is then lost and the next `/settings` snapshot shows the sign-in form. Treat that as a session restart, not a fresh-instance precondition failure: sign in again with the disposable password and continue the journey. Two reliable ways to avoid losing the session mid-journey: keep one browser session, and navigate with `goto <url>` — it preserves the HTTP-only session cookie, while a second `open` can start a fresh context that drops it (`cookie-list` printing `No cookies found` is the tell). Drive navigation with `goto` and visible controls, not raw `eval`: this playwright-cli build returns no value from `eval`, prints an error stack for it, and even `location.href` assignments through it proved unreliable; use `eval` only for read-only assertions, and fall back to snapshot + click when it fails. After signing back in, revisit auth-gated pages with `goto` rather than trusting the stale view.

## Drive

Use the accessible snapshot after each page transition; refs expire when the page changes. Click buttons/links by the role and accessible name shown there, and fill labeled fields by their accessible name or stable selector. Simulate user actions through visible controls, with app-state reads via `eval` for assertions only; raw `mousemove`/`mousedown` sequences with explicit viewport coordinates are the exception, for live drag/resize checks where the pointer path is itself the behavior under test (see [`features/services-widgets.md`](features/services-widgets.md)).

`click` and `fill` targets must be a snapshot ref (`e29`), a CSS selector, or a Playwright locator such as `getByRole('button', { name: 'Sign in' })` — the shorthand `click 'button "Sign in"'` (role + quoted name) is not a valid target and errors. `browser.sh` commands resolve relative `--filename` paths from the evidence directory (`browser.sh` chdirs there), not from the repo root. Refs also go stale after in-page re-renders (fills saving drafts, dialogs opening), not only after page transitions: if a `click` on a freshly typed form does nothing, re-snapshot and click the fresh ref. Filling by ref is `fill <ref> <text>`; `type <text>` is a different command (it types into the focused element and does not take a ref). Invoke `browser.sh`/`instance.sh` from the repository root — their relative script path breaks when the cwd moves (e.g. after chdir-ing into an evidence dir).

Start a named Playwright session and inspect the actual first-run page:

```bash
./.agents/skills/verify-vesta/scripts/browser.sh "$RUN_ID" open "$BASE_URL/settings" --browser=firefox
./.agents/skills/verify-vesta/scripts/browser.sh "$RUN_ID" snapshot
```

For the Settings feature journey, create a disposable administrator through the visible form, then edit and save Appearance:

```bash
VERIFY_PASSWORD='Vesta-verification-only-2026!'
./.agents/skills/verify-vesta/scripts/browser.sh "$RUN_ID" screenshot --filename=01-first-run-setup.png
./.agents/skills/verify-vesta/scripts/browser.sh "$RUN_ID" fill '#admin-password' "$VERIFY_PASSWORD"
./.agents/skills/verify-vesta/scripts/browser.sh "$RUN_ID" fill '#admin-password-confirm' "$VERIFY_PASSWORD"
./.agents/skills/verify-vesta/scripts/browser.sh "$RUN_ID" click 'button:has-text("Create administrator account")'
./.agents/skills/verify-vesta/scripts/browser.sh "$RUN_ID" snapshot
```

After the authenticated Settings editor and its data finish loading, edit the title and theme, then save through the visible button:

```bash
./.agents/skills/verify-vesta/scripts/browser.sh "$RUN_ID" fill 'label:has-text("Dashboard title") input' 'Verification Home'
./.agents/skills/verify-vesta/scripts/browser.sh "$RUN_ID" select 'label:has-text("Theme") select' 'light'
./.agents/skills/verify-vesta/scripts/browser.sh "$RUN_ID" screenshot --filename=02-settings-draft.png
./.agents/skills/verify-vesta/scripts/browser.sh "$RUN_ID" snapshot > "$EVIDENCE_DIR/02-settings-draft.snapshot.txt"
./.agents/skills/verify-vesta/scripts/browser.sh "$RUN_ID" click 'button:has-text("Save changes")'
./.agents/skills/verify-vesta/scripts/browser.sh "$RUN_ID" snapshot > "$EVIDENCE_DIR/03-settings-saved.snapshot.txt"
./.agents/skills/verify-vesta/scripts/browser.sh "$RUN_ID" screenshot --filename=03-settings-saved.png
./.agents/skills/verify-vesta/scripts/browser.sh "$RUN_ID" click 'a:has-text("Back to dashboard")'
./.agents/skills/verify-vesta/scripts/browser.sh "$RUN_ID" snapshot > "$EVIDENCE_DIR/04-dashboard-updated.snapshot.txt"
./.agents/skills/verify-vesta/scripts/browser.sh "$RUN_ID" screenshot --filename=04-dashboard-updated.png
cp "$SCRATCH_DIR/config.toml" "$EVIDENCE_DIR/config.toml"
python3 - "$EVIDENCE_DIR/config.toml" > "$EVIDENCE_DIR/config-validation.txt" <<'PY'
import sys
import tomllib

with open(sys.argv[1], "rb") as config_file:
    config = tomllib.load(config_file)
assert config["app"]["dashboard_title"] == "Verification Home"
assert config["app"]["theme"] == "light"
print("Saved TOML matches the dashboard title and light theme selected in Settings.")
PY
```

Appearance is the default editor section. The label-based selectors above match the actual labels in `appearance-section.tsx`. Creating the administrator account does not sign you in: after the create succeeds, the panel switches to the `Sign in` form and the journey continues by filling the password again and clicking `Sign in` (see [`features/authentication.md`](features/authentication.md)). Avoid snapshots while either password field contains the temporary password; submit first, then capture the authenticated page. Inspect `02-settings-draft.snapshot.txt` for the unsaved status, `Verification Home`, and selected Light theme; after Save, confirm `Changes saved to config.toml.` in `03-settings-saved.snapshot.txt`. After navigation, the dashboard page title and heading must show `Verification Home`.

Feature journeys and their user-visible success criteria live in [`features/README.md`](features/README.md). Start with [`features/appearance-settings.md`](features/appearance-settings.md) for the seed journey.

## Evidence

Store each run under `artifacts/verification/vesta/$RUN_ID/`. `browser.sh` runs from that evidence directory, so Playwright CLI's own snapshots also remain there. Capture the interaction as well as the result: a screenshot of first-run setup, a draft screenshot showing the changed title and unsaved state, a saved-state screenshot/accessible snapshot, and a dashboard screenshot after following `Back to dashboard`. Save Playwright snapshots and relevant CLI output beside those images. Do not put the disposable password in captured evidence.

For saved settings, copy `"$SCRATCH_DIR/config.toml"` to `"$EVIDENCE_DIR/config.toml"` before cleanup and parse the copy with Python's `tomllib`; assert the expected `app.dashboard_title` and `app.theme`. A settings message alone is not proof of persistence. Preserve the browser screenshots, accessible snapshots, and saved TOML copy, and retain `server.log` and `build.log` for diagnosis. Exercise the visible user path and verify effects at the public UI/config-file boundary; use mocks only for external systems behind Vesta's existing HTTP integration boundary.

## Cleanup

Close the exact named Playwright session, then remove only this run's process and scratch config/database:

```bash
./.agents/skills/verify-vesta/scripts/browser.sh "$RUN_ID" close || true
./.agents/skills/verify-vesta/scripts/instance.sh cleanup "$RUN_ID"
test -s "$EVIDENCE_DIR/04-dashboard-updated.png"
test -s "$EVIDENCE_DIR/config.toml"
```

Run cleanup after every failed attempt too. It signals only the PID whose command line and run marker match this run, removes the private run directory, and leaves `artifacts/verification/vesta/$RUN_ID/` untouched. Evidence must still exist after cleanup; never delete the evidence directory as part of teardown.

## Console triage

The snapshot footer reports console errors, but it does not attribute them. Read the run's console logs to match an error to the action that exposed it: favicon.ico 404s are noise, while a 404 or failed load appearing right after a page transition or lazy-chunk mount (for example an asset the built HTML references at a path the server does not host) is a real finding — treat the UI as silently degraded until the cause is fixed or ruled out.

## Seeding server state over the API

The UI journey is the default, but when verifying rendering against specific server-side state, seed it directly at the public API boundary: authenticate once by submitting the visible sign-in form (the browser context then holds a `vesta_session` cookie), confirm with `cookie-list`, and drive writes with `curl -H "Cookie: vesta_session=<token>"` against the same origin (for example `PUT /api/settings` with the intended dashboard shape). Verify the effect in the browser afterwards; do not reach past the public API or touch internal state.

## Helpers

[`scripts/instance.sh`](scripts/instance.sh) is the executable instance manager. Its complete interface is:

```bash
./.agents/skills/verify-vesta/scripts/instance.sh start RUN_ID
./.agents/skills/verify-vesta/scripts/instance.sh doctor RUN_ID
./.agents/skills/verify-vesta/scripts/instance.sh url RUN_ID
./.agents/skills/verify-vesta/scripts/instance.sh cleanup RUN_ID
```

[`scripts/browser.sh`](scripts/browser.sh) is the executable Playwright wrapper; it runs from the run's evidence folder so CLI-generated snapshots stay with the proof:

```bash
./.agents/skills/verify-vesta/scripts/browser.sh RUN_ID open http://127.0.0.1:PORT/settings --browser=firefox
./.agents/skills/verify-vesta/scripts/browser.sh RUN_ID snapshot
./.agents/skills/verify-vesta/scripts/browser.sh RUN_ID close
```

All helper commands are scoped to the supplied run ID; choose a new ID for every attempt so parallel runs cannot share a port, profile, config, or database.
