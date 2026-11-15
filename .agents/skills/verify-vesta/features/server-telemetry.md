# Server telemetry

Settings lets an administrator configure local system metrics or a remote Prometheus text-format endpoint such as node_exporter. Vesta then displays the source and live CPU, memory, disk, load, and uptime values that the source provides.

## Sub-features

- Add, edit, reorder, and remove server metric sources.
- Configure the local `local` collector or an HTTP(S) node_exporter endpoint.
- Choose the overview server and observe metric status and values.
- Optional hostname, device name, icon, and scrape interval.

## How to get to it (user POV)

From `/`, open `Settings`, select the `Servers` section, and use `Add server`. Set the required server name, endpoint, and poll interval, then save. Return to the dashboard's Servers section and Overview to inspect the resulting status and metrics.

## Driving it with playwright-cli

Authenticate through the visible Settings form first, then open Settings' `Servers` section and add a local source using its visible controls:

```bash
./.agents/skills/verify-vesta/scripts/browser.sh "$RUN_ID" click 'a[aria-label="Settings"]'
./.agents/skills/verify-vesta/scripts/browser.sh "$RUN_ID" resize 1440 900
./.agents/skills/verify-vesta/scripts/browser.sh "$RUN_ID" click 'button:has-text("Servers")'
./.agents/skills/verify-vesta/scripts/browser.sh "$RUN_ID" click 'button:has-text("Add server")'
./.agents/skills/verify-vesta/scripts/browser.sh "$RUN_ID" fill 'label:has-text("Server name") input' 'Local verification host'
./.agents/skills/verify-vesta/scripts/browser.sh "$RUN_ID" click 'button:has-text("Save changes")'
```

At mobile widths use `select '#settings-section-select' 'servers'` instead of the desktop-only section button. On the dashboard, the sidebar link is hidden until you click `button[aria-label="Open navigation"]`; alternatively, open `/settings` directly.

The new server defaults to the supported `local` endpoint and 15-second interval. Capture the draft and saved editor, then click `a:has-text("Back to dashboard")`, followed by `a[aria-label="Servers"]`. Wait for the first poll; verify the named source progresses from Collecting to Reporting and values appear. Copy/parse the generated TOML to confirm persistence.

For a remote-exporter journey, run a small loopback HTTP fixture serving representative Prometheus text at the normal `endpoint` URL; that substitutes only for the external node_exporter HTTP boundary the product already consumes. The browser still configures and saves the endpoint through the real Settings form.

## Gotchas

- New runs have no configured nodes; add one through the UI before expecting metrics. If the UI or settings data fails to load, report that precondition/runtime failure rather than inserting a node directly into TOML or calling internal state setters.
- Polling is asynchronous. A correct configuration may show `Collecting` before the first successful local sample; wait for the configured interval and check browser-visible state again.
- Never point a verification instance at a production exporter or include production hostnames, credentials, or icon URLs in evidence.
