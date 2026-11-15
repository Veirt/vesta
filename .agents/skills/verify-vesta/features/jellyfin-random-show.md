# Jellyfin Random Show widget

`JellyfinRandomShow` fetches one random series from a Jellyfin server and renders a poster/backdrop card. It needs an external Jellyfin URL and API key, so isolated verification runs against a loopback mock serving canned PascalCase `Items` payloads — the same shape a real Jellyfin returns.

## Sub-features

- Add the widget as a tile from the homepage **Add tile** menu; the dialog shows its two required fields (Jellyfin URL, API key) with live "required" messages until both are filled.
- Library picker: a `remote_select` field ("Library", defaults to "All libraries") fetches the server's media libraries through `POST /api/widgets/{widget}/options` (`{field, config}` → jsend `{options: [{value, label}]}`), debounced ~600 ms after URL/key last change. Before URL+key are filled it shows a hint line ("Enter the URL and API key above to list them."); on failure it shows the backend's specific error plus a **Retry** button and drops to just "All libraries"; a saved selection missing from the offered options renders a regression notice.
- Populated path: the tile shows title, year, star rating, genres, and a clamped overview, with a link to the series details page; poster and backdrop images load from the configured server.
- Empty-library path: a library with zero series renders a centered "No series in library" placeholder.
- Error path: rejected credentials surface as a styled in-tile error, never a blank card.
- The Settings widget catalog shows a static sample preview with a photo placeholder.

## The mock

Save this as `mock-jellyfin.mjs` and start two instances with the durable background-process runner, not a bare bash call (see the harness gotcha in [the skill](../SKILL.md)):

```bash
bun mock-jellyfin.mjs 9991 full   # one series with images
bun mock-jellyfin.mjs 9992 empty  # zero items
```

```js
// Minimal Jellyfin mock: bun server.mjs <port> <mode>
const PNG_1PX = Uint8Array.from(
  atob("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg=="),
  (c) => c.charCodeAt(0),
);
const [port, mode] = process.argv.slice(2);
const key = "mock-key";

Bun.serve({
  port: Number(port),
  async fetch(req) {
    const url = new URL(req.url);
    if (req.headers.get("x-api-key") !== key) {
      return new Response(JSON.stringify({ error: "Unauthorized" }), { status: 401 });
    }
    if (url.pathname === "/Items") {
      const items = mode === "empty" ? [] : [{
        Id: "abc123", Name: "Example Show",
        Overview: "A random pick from the mock library for verification.",
        CommunityRating: 8.7, ProductionYear: 2020,
        Genres: ["Drama", "Comedy"],
        ImageTags: { Primary: "primary-tag" }, BackdropImageTags: ["backdrop-tag"],
      }];
      return new Response(JSON.stringify({ Items: items, TotalRecordCount: items.length }), {
        headers: { "content-type": "application/json" },
      });
    }
    if (url.pathname.startsWith("/Items/abc123/Images/")) {
      return new Response(PNG_1PX, { headers: { "content-type": "image/png" } });
    }
    return new Response("not found", { status: 404 });
  },
});
```

Real Jellyfin responses are PascalCase; sending camelCase keys proves nothing about the real integration. The `X-Api-Key` check doubles as the error path. Stop both mock processes during cleanup.

### Real-server check (when the user supplies a server + key)

If the user names a real Jellyfin host, prefer verifying against it in addition to the mock. Critical: **Jellyfin ≥12 rejects `X-Api-Key` outright** — the backend must send the classic auth header `Authorization: MediaBrowser Token="<key>", Client=Vesta, Device=Vesta, DeviceId=Vesta, Version=1.0`. If library listing or random-series fetches 401 against a real server while the mock passes, the auth header is the first thing to check (`grep authorized` in `src/widgets/jellyfin_random_show_widget.rs`). `GET /Library/MediaFolders` is the library-list endpoint; the chosen library id is sent as `ParentId` on the random `/Items` call and persisted as `config.library` in `[[items]]`. Never write the user's key into this skill or any committed file. Verify the specific error paths through the UI: wrong key → "Jellyfin rejected the API key (HTTP 401…)…", unreachable host → "Could not reach the Jellyfin server at <url> for <context>…" in the tile.

## Driving it

Complete first-run setup, then from the homepage in **Edit dashboard** mode add the widget twice — the populated mock URL and the empty one — filling `getByRole("textbox", { name: "Jellyfin URL" })` / `API key` with `http://127.0.0.1:999N` and the mock key, and **Save dashboard**. Then add a third instance pointed at the populated mock with a wrong key as a live error-path check.

## Success criteria

- Both live tiles render their own path (populated card / empty placeholder); the saved `config.toml` keeps both `[[items]]` widget entries with their `url`, `key`, and `library` configs intact.
- The wrong-key tile shows the error message (mock returns 401) in the tile's error style, naming the cause — never the generic "Widget request failed (N)".
- The remote library select assigns properly in the tile dialog: `playwright select '[role=dialog] select >> nth=1' '<label>'` (the first select is the tile-size preset, the second is the library); confirm the persisted `config.library` matches the option's value.
- The widget catalog preview in `/settings` → Widgets renders the sample.
- Read the rendered content back through `eval` over `.widget-frame` elements rather than trusting a screenshot alone.

## Gotchas

- The 1-px PNG stretches, so the poster column looks empty in screenshots; assert the `<img>` exists and `naturalWidth > 0` via `eval` instead of eyeballing the poster.
- The populated card is the only element matching `.jellyfin-widget`; the empty placeholder renders different markup — count both through `.widget-frame` when asserting series counts.
