# Plugin system

Vesta installs external plugins from git URLs. A plugin is a git repo with a `vesta-plugin.toml` manifest plus a static web bundle; the backend clones it (gix, shallow), serves its assets, and registers its widgets, which the dashboard renders inside sandboxed iframes fed a `vesta:config` postMessage. Settings gains a **Plugins** section for install/remove; unauthenticated asset routes stay public while admin routes guard install/delete. Only install plugins from trusted sources — installed widget code runs unrestricted inside the dashboard page (sandboxed iframe, but same-origin asset hosting).

## Sub-features

- Install from a git URL (HTTPS, SSH, or local path) via the visible install form; validation errors from a missing/invalid manifest surface as a clear status message.
- Installed plugins are listed with name, version, and widget titles; the widget catalog and Add-tile menu include plugin widgets next to built-ins.
- Update re-fetches the installed plugin from its recorded source URL and swaps the tree in place behind a visible **Update** button (`PUT /api/plugins/<id>`): the version and commit shown in the installed list change, the asset route serves the swapped-in tree with no restart, and hidden temp/backup trees are cleaned up. An update whose fetched manifest declares a different plugin id is refused with an explicit error and the installed tree is kept.
- Plugin widget tiles accept the manifest-declared config fields (text/number/select), validate them with the same dialog validation as built-ins, persist a `Plugin:<plugin_id>:<widget_id>` tile in `config.toml`, and render in a sandboxed iframe that receives the tile title and config on load and on each change.
- Uninstall is refused with an explicit error while any dashboard tile still uses the plugin; after removing the tiles, uninstall succeeds and the plugin disappears from the widget catalog.
- Asset serving rejects path traversal (both in the plugin id and the asset path) and serves only the manifest's declared assets; responses carry `X-Content-Type-Options: nosniff`, `Cache-Control: no-cache`, and `Content-Security-Policy: frame-ancestors 'self'`.

## How to get to it (user POV)

1. Prepare a local plugin git repo (the repo ships `tests/fixtures/plugins/hello-world`; clone or copy it to a scratch path and `git init && git add -A && git commit` if it is not already a repo).
2. Sign in via `/settings`, open the **Plugins** section, paste the repo path or URL, and install.
3. Back on `/`, use **Edit dashboard** → **Add tile** → choose a plugin widget (for example **Counter**), fill its required fields, **Done**, **Save dashboard**.
4. Confirm the live tile renders the iframe content (label from the posted title, working widgets inside the iframe), then edit the tile title and verify both preview and saved dashboard pick it up.
5. Remove the tile (edit dialog → **Remove tile** → **Save dashboard**), then uninstall in Settings; the Plugins list empties.

## Driving it with playwright-cli

```bash
# Fixture repo as an installable git remote
rm -rf /tmp/vesta-plugin-repo && mkdir -p /tmp/vesta-plugin-repo
cp -r tests/fixtures/plugins/hello-world /tmp/vesta-plugin-repo/hello-plugin
cd /tmp/vesta-plugin-repo/hello-plugin && git init -q && git add -A && git -c user.email=v@v -c user.name=v commit -qm init

# Install through the UI (the Plugins controls are inside the outer settings <form>,
# so drive them via JS fill + click, not form submit)
./.agents/skills/verify-vesta/scripts/browser.sh "$RUN_ID" fill 'input[placeholder*="git"]' /tmp/vesta-plugin-repo/hello-plugin
./.agents/skills/verify-vesta/scripts/browser.sh "$RUN_ID" click 'button:has-text("Install")'
./.agents/skills/verify-vesta/scripts/browser.sh "$RUN_ID" snapshot   # expect "<Name> <version> installed."
./.agents/skills/verify-vesta/scripts/browser.sh "$RUN_ID" screenshot --filename=01-plugin-installed.png

# Update in place: commit a new manifest version to the same remote, then press Update
cd /tmp/vesta-plugin-repo/hello-plugin && git -c user.email=v@v -c user.name=v \
  commit -qam "version bump" --allow-empty  # plus a real vesta-plugin.toml version edit
cd - >/dev/null
./.agents/skills/verify-vesta/scripts/browser.sh "$RUN_ID" click 'getByRole("button", { name: "Update" })'
./.agents/skills/verify-vesta/scripts/browser.sh "$RUN_ID" snapshot   # expect "<Name> updated to v<new>."
curl -s -o /dev/null -w '%{http_code}\n' -b admin-cookies.txt "$BASE_URL/api/plugins/hello-world/assets/index.html"  # swapped-in tree serves

# Saved-tile proof: tile persists as Plugin:<plugin_id>:<widget_id> with its config
grep -n 'Plugin:hello-world' "$SCRATCH_DIR/config.toml"

# Uninstall guard (tile still on dashboard): expect the explicit in-use error
./.agents/skills/verify-vesta/scripts/browser.sh "$RUN_ID" click 'button:has-text("Remove")'
./.agents/skills/verify-vesta/scripts/browser.sh "$RUN_ID" snapshot   # "...still used by dashboard tiles..."

# Asset boundary (public route): valid asset 200, traversal 404, manifest never served
curl -s -o /dev/null -w '%{http_code}\n' "$BASE_URL/api/plugins/hello-world/assets/index.html"
curl -s -o /dev/null -w '%{http_code}\n' "$BASE_URL/api/plugins/hello-world/assets/../vesta-plugin.toml"
```

## Assertions in user-visible terms

- Status text confirms the exact plugin name and version after install, and the new version after Update (`"<Name> updated to v<x.y.z>."`); the installed list's commit line changes too.
- Update on an unknown id returns `404 plugin_not_found`; unauthenticated `PUT /api/plugins/<id>` is `401`; a traversal plugin id (`..%2f…`) on the asset route is `404`.

- Status text confirms the exact plugin name and version after install.
- The Add-tile menu lists the plugin widget by its manifest title, and its edit dialog shows the manifest's config fields with working validation.
- The saved tile renders on the live dashboard inside an iframe whose content shows the manifest-driven UI; the tile title flows into the iframe via postMessage.
- Uninstalling a used plugin fails with a message naming the tiles; removing the tiles first makes uninstall succeed and empty the list.

## Gotchas

- **Nested forms are stripped by the browser.** The Plugins section renders inside the settings page's outer `<form>`; any `<form>` nested inside it disappears from the DOM and its submit button does nothing. Plugin controls must be plain buttons with click handlers, and journeys must click them directly rather than pressing Enter in a field expecting submit.
- **Iframe internals are opaque to the driver.** `contentDocument` access throws (opaque origin); assert via accessible snapshot `f…` refs inside the `iframe` node, which Playwright exposes.
- **Never `open` a second URL in the middle of an authenticated journey.** Each `open` can restart the Playwright browser (new pid) and the session cookie is lost; later steps see signed-out state and the dashboard hides editor controls, which looks like an app bug. Navigate in-page with `eval '() => { location.href = "…"; }'` or `location.reload()` after signing in, and keep one browser session per journey.
- `VESTA_PLUGINS` defaults to `./data/plugins`; `instance.sh` sets it into the run's scratch dir, so a plugin installed during a run lands under `$SCRATCH_DIR/data/plugins/<plugin_id>` (its `.git` directory is stripped on install).
- Install URLs are validated by actually cloning: offline runs fail HTTPS installs with a clone error. Prefer the local path fixture; set the remote-clone expectation aside when the sandbox has no network.
