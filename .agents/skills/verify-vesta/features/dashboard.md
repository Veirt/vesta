# Dashboard overview

The root route presents the dashboard title, server metric cards and list, plus a canvas of independent service and widget tiles. Optional group labels divide the canvas into sections with independent layouts; every defined section stays visible even when empty, and each section carries a width span. Authenticated administrators can enter **Edit dashboard** mode to add, drag, resize, configure, group, and save tiles. The page updates node state through the app's server-sent events stream.

## Sub-features

- Overview and metric cards for the selected server.
- Server selector and navigation to Overview, Servers, and Apps and widgets.
- Search across configured servers and dashboard tiles.
- Clear empty states when no servers or dashboard tiles are configured.
- Authenticated dashboard edit mode; guests can browse the canvas but cannot edit it.
- Live updates through `/api/nodes/events`.

## How to get to it (user POV)

Visit `/` or follow `Back to dashboard` from Settings. Use the `Dashboard sections` navigation to reach `Overview`, `Servers`, or `Apps and widgets`; select a server in the metric-card control when more than one is configured. After signing in, choose `Edit dashboard` to manage the tile canvas.

## Driving it with playwright-cli

Open `/` and take an accessible snapshot. Verify the title and empty-state messaging on a fresh run. The desktop sidebar links are visible from a `1440×900` viewport. On mobile, first open the sidebar using `button[aria-label="Open navigation"]`, then click its accessible `Servers`/`Overview` link; navigation closes itself. Search is a stable `#dashboard-search` field:

```bash
./.agents/skills/verify-vesta/scripts/browser.sh "$RUN_ID" open "$BASE_URL/" --browser=firefox
./.agents/skills/verify-vesta/scripts/browser.sh "$RUN_ID" resize 1440 900
./.agents/skills/verify-vesta/scripts/browser.sh "$RUN_ID" click 'a[aria-label="Servers"]'
./.agents/skills/verify-vesta/scripts/browser.sh "$RUN_ID" fill '#dashboard-search' 'Local verification host'
./.agents/skills/verify-vesta/scripts/browser.sh "$RUN_ID" snapshot
./.agents/skills/verify-vesta/scripts/browser.sh "$RUN_ID" requests
```

For the mobile path, set the viewport to `390 844`, click `button[aria-label="Open navigation"]`, then click `a[aria-label="Servers"]`. Clear the search with `fill '#dashboard-search' ''`, then click `a[aria-label="Overview"]` (open the mobile sidebar first if needed). After adding a local server through Settings, wait for telemetry collection and verify its Reporting status and metric values in the accessible snapshot. Use `requests` to confirm the normal dashboard page opened `/api/dashboard` and `/api/nodes/events`; do not issue those requests as a substitute for the visible journey.

## Gotchas

- A pristine config has no server or tile data. Empty states are expected; populated journeys must first add servers in Settings and tiles in dashboard edit mode.
- Local CPU values vary by host and time. Assert that a value and a current/reporting status appear, not a particular percentage.
- The dashboard's live section is provided by SSE. If it remains in a loading/error state, run the skill's doctor first and inspect browser requests and `server.log` before retrying.
