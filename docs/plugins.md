# Vesta plugins

Vesta plugins are git repositories that add widgets to the dashboard. A plugin
contains static web bundles (plain HTML/JS/CSS compiled however you like) and
a `vesta-plugin.toml` manifest that declares its widgets and their settings
fields. Vesta clones the repository, serves the bundles, and embeds them as
sandboxed iframes.

> **Trust note.** Plugin code downloaded from a git repository runs in your
> browser with all of that page's authority. Only install plugins from
> repositories you trust. Vesta deliberately does not sandbox, audit, or
> version-pin third-party plugin code beyond the iframe sandbox described
> below; installing from a URL is an explicit trust decision.

## Installing

Settings → Plugins, or:

```
POST /api/plugins        {"url": "https://example.com/vesta-plugin.git"}
GET  /api/plugins
PUT  /api/plugins/<id>
DELETE /api/plugins/<id>
```

- The URL may be HTTPS, SSH, git, or a local `file:///repo/path` (useful while
  developing). Clones are shallow (depth 1); the fetched commit is recorded in
  `vesta-plugin-source.toml` next to the manifest.
- Installing or removing a plugin rebuilds the widget registry immediately; no
  restart is required.
- A plugin cannot be removed while dashboard tiles still use its widgets.
- `PUT /api/plugins/<id>` updates an installed plugin in place: Vesta re-clones
  from the URL recorded at install time, refuses the update if the fetched
  manifest declares a different plugin id (that is a different plugin — install
  it separately), and atomically swaps the directory with rollback if the move
  fails. Tile configs survive; the same-id manifest changes (new fields, new
  versions) apply on the registry rebuild.
- Plugins are stored under the `VESTA_PLUGINS` directory (default
  `./data/plugins`).

## Manifest

`vesta-plugin.toml` must exist at the repository root:

```toml
id = "hello-world"          # [a-z0-9][a-z0-9_-]{0,63}, unique
name = "Hello World"
version = "1.0.0"           # optional, informational
description = "Shows a counter widget."

[[widgets]]
id = "counter"              # [a-z0-9][a-z0-9_-]{0,63}, unique within plugin
title = "Counter"
description = "Configured via the label field; add or subtract on click."
entry = "index.html"        # relative path, no `..`, no leading `/`, required
width = 3                   # default tile width 2..12
height = 2                  # default tile height 2..12
min_width = 3               # minimum tile width (<= width)
min_height = 2              # minimum tile height (<= height)
refresh_interval_secs = 60  # optional; 0 disables periodic refetch

# Optional settings fields, rendered by the widget configuration form.
[[widgets.fields]]
key = "label"
label = "Label"
kind = "text"               # text | url | password | number | select
required = true
description = "Text shown above the counter."

[[widgets.fields]]
key = "theme"
label = "Theme"
kind = "select"
default = "auto"
description = "Color scheme."

[[widgets.fields.options]]
value = "auto"
label = "Follow host"
```

Rules enforced at install time and again at config-save time:

- Widget and plugin ids are slugs; a plugin id cannot collide with an
  already-installed one.
- `entry` must exist in the repository and cannot escape the plugin directory.
- Sizes are bounded (2..12); `min_width <= width`, `min_height <= height`.
- At most 32 widgets and 32 fields per widget.
- `select` fields need at least one option; other kinds must not declare any.
- Required fields must be non-empty for the tile to validate.

Widget names registered in the grid are `Plugin:<plugin_id>:<widget_id>`; they
appear in the Add tile menu and the Settings → Widgets catalog like any
built-in widget.

## Serving and embedding

Assets are served publicly (the dashboard itself is public) at:

```
GET /api/plugins/<plugin_id>/assets/<path>
```

- Path resolution rejects `..` traversal and symlink escapes out of the plugin
  directory; only a small allow-list of file extensions is served, each with an
  explicit content type and `X-Content-Type-Options: nosniff`.
- The response carries `Cache-Control: no-cache` and a restrictive CSP
  (`default-src 'none'`, inline script/style allowed, `frame-ancestors 'self'`).
- The bundle is embedded with
  `<iframe sandbox="allow-scripts allow-forms">`, which gives the plugin an
  opaque origin: it cannot read Vesta's DOM, cookies, or storage.

## Data and configuration contract (v1)

Plugin widgets do not run backend code. When the dashboard requests widget
data, Vesta answers with pointers only:

```json
{
  "plugin_id": "hello-world",
  "widget_id": "counter",
  "entry": "index.html",
  "config": { "label": "Visitors", "start_value": 10 }
}
```

The frontend renders `<iframe src="/api/plugins/hello-world/assets/index.html">`
and posts the configuration to the frame:

```js
{ type: "vesta:config", title: "Counter", config: { label: "Visitors" } }
```

The message is (re)sent whenever the frame loads and whenever the tile's saved
configuration changes. A plugin page that needs external data (e.g. a public
API) fetches it itself from inside the frame with its own credentials.

## Example

`tests/fixtures/plugins/hello-world` in the Vesta repository is a complete,
working plugin (counter widget with a label, start value, and theme). Copy it
to any git repository — local paths work too:

```
git -C /tmp init /tmp/hello-plugin
cp -r tests/fixtures/plugins/hello-world/* /tmp/hello-plugin/
git -C /tmp/hello-plugin add . && git -C /tmp/hello-plugin commit -m init
# then install via Settings → Plugins with file:///tmp/hello-plugin
```
