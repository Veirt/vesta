# Dashboard tiles

The homepage is one canvas of independent service and widget tiles. Settings owns app/server configuration and the widget catalog with sample previews; tile creation, configuration, ordering, sizing, and saving happen from the homepage's **Edit dashboard** mode.

## Sub-features

- Add, edit, move, resize, and remove service tiles and widget tiles independently.
- Group related services and widgets into named sections; sections can be reordered (drag the grip or use Move earlier/later in the section dialog), resized in width (the right-edge horizontal-arrow slider or the section dialog's Width select), and stay visible in the saved dashboard even with no tiles — an empty group renders a centered empty-state box, never a floating message or hidden section.
- The built-in **Ungrouped** section is a first-class section that is always visible: it can be dragged (HTML5 drag from its `Move Ungrouped section` grip) and its width resized like a named group. It persists in the saved TOML `[[groups]]` as an entry with an empty `id`; at most one such entry is valid, and tile validation skips it.
- Configure JellyfinRandomShow per instance (Jellyfin URL plus an API key); see [`features/jellyfin-random-show.md`](jellyfin-random-show.md) for the loopback-mock journey covering the populated, empty-library, and bad-credential paths.
- Drag any non-control part of a tile on desktop, resize tiles, choose size presets, and use move-earlier/move-later controls from the edit dialog. Tiles enforce minimum sizes: services 3×1; widget minimums come from the widget descriptor (for example Clock 3×2).
- Configure service titles, destination URLs, icons, and optional availability-check URLs.
- Verify availability-check URL syntax immediately and test a valid URL from Vesta after a 500 ms debounce.
- Configure QuickLinks, Clock, Weather, and SonarrCalendar options per widget instance. SystemStats is hidden from new tile choices but still renders in saved legacy dashboards.
- Preview the built-in widget catalog in Settings.
- Persist layout as independent `[[items]]` entries with optional `group` labels; legacy grouped service configs migrate to independent tiles while retaining their group names.

## How to get to it (user POV)

Sign in through `/settings`, return to `/`, and choose **Edit dashboard**. Use the **Add tile** dropdown to create a service or choose a widget. In the tile editor, use the group popover to assign an existing group or create one; each service and widget remains a separate tile. Choose **Done** to return to the canvas, and **Save dashboard** to persist all draft changes. On desktop, drag any non-control area of a card and use its resize handle; the pencil stays overlaid without changing the card preview. On narrow screens, tiles stack and the edit dialog's **Move earlier/later** and **Tile size** controls remain available. Invalid tile details are explained and disable saving. Open `/settings` → **Widgets** to see the available sample previews and field descriptions.

## Driving it with playwright-cli

Complete first-run setup or authenticate through the visible `/settings` form. The authenticated settings session is shared with the homepage. Then open the homepage and create a service and a Clock widget through the visible editor:

```bash
./.agents/skills/verify-vesta/scripts/browser.sh "$RUN_ID" open "$BASE_URL/" --browser=firefox
./.agents/skills/verify-vesta/scripts/browser.sh "$RUN_ID" resize 1440 900
./.agents/skills/verify-vesta/scripts/browser.sh "$RUN_ID" click 'button:has-text("Edit dashboard")'
./.agents/skills/verify-vesta/scripts/browser.sh "$RUN_ID" click 'button:has-text("Add tile")'
./.agents/skills/verify-vesta/scripts/browser.sh "$RUN_ID" click '[role="menuitem"]:has-text("Add service")'
./.agents/skills/verify-vesta/scripts/browser.sh "$RUN_ID" fill '[role="dialog"] label:has-text("Tile title") input' 'Docs'
./.agents/skills/verify-vesta/scripts/browser.sh "$RUN_ID" fill '[role="dialog"] label:has-text("Destination URL") input' 'https://example.invalid/'
./.agents/skills/verify-vesta/scripts/browser.sh "$RUN_ID" fill '[role="dialog"] label:has-text("Availability check URL") input' 'https://example.invalid/health'
./.agents/skills/verify-vesta/scripts/browser.sh "$RUN_ID" click '[role="dialog"] button[aria-label="Choose a dashboard group"]'
./.agents/skills/verify-vesta/scripts/browser.sh "$RUN_ID" fill '#new-dashboard-group' 'Reference'
./.agents/skills/verify-vesta/scripts/browser.sh "$RUN_ID" click 'button:has-text("Use group")'
./.agents/skills/verify-vesta/scripts/browser.sh "$RUN_ID" snapshot
./.agents/skills/verify-vesta/scripts/browser.sh "$RUN_ID" click 'button:has-text("Done")'
./.agents/skills/verify-vesta/scripts/browser.sh "$RUN_ID" click 'button:has-text("Add tile")'
./.agents/skills/verify-vesta/scripts/browser.sh "$RUN_ID" click '[role="menuitem"]:has-text("Clock")'
./.agents/skills/verify-vesta/scripts/browser.sh "$RUN_ID" fill '[role="dialog"] label:has-text("Tile title") input' 'Local time'
./.agents/skills/verify-vesta/scripts/browser.sh "$RUN_ID" click '[role="dialog"] button[aria-label="Choose a dashboard group"]'
./.agents/skills/verify-vesta/scripts/browser.sh "$RUN_ID" click '[role="group"][aria-label="Groups"] button:has-text("Reference")'
./.agents/skills/verify-vesta/scripts/browser.sh "$RUN_ID" select '[role="dialog"] label:has-text("Tile size") select' '6x2'
./.agents/skills/verify-vesta/scripts/browser.sh "$RUN_ID" click 'button:has-text("Done")'
./.agents/skills/verify-vesta/scripts/browser.sh "$RUN_ID" click 'button:has-text("Save dashboard")'
./.agents/skills/verify-vesta/scripts/browser.sh "$RUN_ID" snapshot
```

Wait for the status text `No successful response` or `The endpoint is responding` after the URL check; `.invalid` is intentionally not reachable, and must never be opened as a link. Capture a desktop screenshot of the edit canvas and saved dashboard. Resize to `390 844`, re-enter **Edit dashboard**, and confirm tiles stack while the accessible edit buttons and size/move controls remain usable. Open `/settings`, select **Widgets**, and verify the sample previews appear in the catalog.

Save `"$SCRATCH_DIR/config.toml"` into the evidence directory and parse it with `tomllib`. Verify the service and widget are separate `items` entries (distinct `id` and `kind`), both retain the `Reference` group, and the widget title/config and layouts persisted. When the Ungrouped section was dragged or resized, also confirm `groups` contains exactly one entry with an empty `id` whose `order`/`width` match what the canvas showed. The endpoint used by the dashboard editor is authenticated `PUT /api/settings/dashboard`; the live URL test uses authenticated `POST /api/settings/ping-test`. Use browser `requests` only to confirm these calls followed the visible journey, not as a substitute for the UI actions.

## Live drag and resize

Grid dragging and resizing is pointer behavior; dispatched synthetic events (e.g. `new MouseEvent('mousemove')` via eval) never drive it. Use raw pointer sequences with explicit viewport coordinates, and capture observations mid-drag — the correct claim is "the tile snapped with the pointer", provable only while the button is still down:

```bash
B="./.agents/skills/verify-vesta/scripts/browser.sh"
# Measure the handle's viewport center AFTER any scroll into view. Rects measured before a scroll are stale.
$B "$RUN_ID" eval '() => { const tile = Array.from(document.querySelectorAll(".react-grid-item")).find(el => el.textContent.includes("Clock")); const h = tile.querySelector(".dashboard-tile-resize-handle"); const r = h.getBoundingClientRect(); return {x: r.x + r.width/2, y: r.y + r.height/2}; }'
$B "$RUN_ID" mousemove X Y
$B "$RUN_ID" mousedown
$B "$RUN_ID" mousemove X2 Y2
$B "$RUN_ID" eval '() => { const t = Array.from(document.querySelectorAll(".react-grid-item")).find(el => el.textContent.includes("Clock")); return {w: t.style.width, h: t.style.height, resizing: t.closest("section")?.hasAttribute("data-tile-resize") ?? false}; }'
$B "$RUN_ID" screenshot --filename=NN-mid-resize.png
$B "$RUN_ID" mouseup
```

Mid-drag, assert through `eval` that the item itself is the preview (no trailing placeholder): its pixel width must already be a whole-column step (`columnWidth = (containerWidth - gap*(cols-1)) / cols`, then width = `columnWidth*n + (n-1)*gap`, with the gap/row constants in `dashboard-items.tsx`), sizes clamp upward at the tile's minimum (role=slider `aria-valuemin`), and the end state after `mouseup` equals the last mid-drag size. Screenshots before and after `mouseup` must show the same geometry — a mid-drag screenshot that disagrees with the released state is the preview-mismatch bug this recipe exists to catch.

Additional grid-behavior checks, all read back through `eval` on the item's `style.transform` and the canvas height:

- **Minimum sizes**: use the handles' `role=slider` Arrow-key support (`ArrowRight`/`ArrowDown` grow the tile's width/height, `ArrowLeft`/`ArrowUp` shrink it; group sliders expose width only) as the deterministic clamp check; services floor at 3x1, each widget at its descriptor minimum (authoritative values live in `src/widgets/*_widget.rs` `min_size()`, mirrored in `frontend/src/features/dashboard/tile-sizes.ts`). Confirm an oversized widget is never scrollable/clipped content at its minimum by reading `scrollHeight` vs `clientHeight` inside the widget frame.
- **Compaction**: drag a tile far below the last occupied row and release, several times in a row. After each drop the tile's translated Y must return to the compacted position and the canvas height must stay constant; a growing canvas with items pushed endlessly downward is the empty-row bug this check guards against.
- **Group width**: every section exposes a horizontal resize slider at its right edge — verify both the pointer drag and `ArrowLeft`/`ArrowRight` keyboard paths change the group width by whole columns between the 3-column minimum and 12.

Interactions that only prove runtime behavior (snap, clamp, compaction) may stay in draft: click **Discard** afterwards and confirm the saved `config.toml` copy still matches the state persisted before the experiment.

## Gotchas

- A fresh dashboard is intentionally empty. Create a service/widget through homepage edit mode; Settings contains servers, app preferences, and catalog previews, not a tile editor.
- `label:has-text(...)` locators can strict-collide when one label's text is a prefix of another ("Destination URL" matches the "Destination display" checkbox's label too); target inputs with `getByRole("textbox", { name: ... })` instead.
- When the **Add tile** dropdown is still open from a previous step it intercepts clicks on the button; press `Escape` (or click a menuitem) before re-clicking the trigger.
- Use reserved `.invalid` URLs to test the down result without contacting an external service; the URL is checked from Vesta, not the browser. Do not click the destination tile.
- Test the up state only with a loopback health endpoint that is already part of the isolated test setup. The check has a five-second request timeout.
- Widget handlers that require an external service or credentials may not produce live data in an isolated run. Clock and QuickLinks are suitable for local edit/render smoke checks.
- Snap, clamp, compaction, and group-width assertions go through the live pointer recipe above; the move controls and size preset remain the deterministic accessible path for order/size persistence checks.
