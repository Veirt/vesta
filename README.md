# Vesta

Vesta is a self-hosted home-server dashboard for service links and live system telemetry.

## Deployment

Docker Compose is the recommended deployment. The named volume keeps `config.toml`, the SQLite authentication database, and uploaded icons across container upgrades:

```yaml
services:
  vesta:
    image: ghcr.io/veirt/vesta:latest
    ports:
      - "8450:3000"
    volumes:
      - vesta_data:/data
    restart: unless-stopped

volumes:
  vesta_data:
```

`config.toml` is created on first save. See [`docs/vesta.toml`](docs/vesta.toml) for an example. You can also edit the active config directly while Vesta is stopped; keep the TOML structure valid and preserve a private file permission because widget settings may contain API keys.

On first launch, visit `/settings` and create the administrator password. The initial dashboard is empty; configure servers in Settings, then add and arrange service and widget tiles on the homepage. Dashboard settings are written to `/data/config.toml`.

### Environment variables

| Variable | Default | Purpose |
| --- | --- | --- |
| `VESTA_CONFIG` | `./config.toml` | TOML file containing dashboard and app settings |
| `VESTA_DATABASE` | `./data/vesta.sqlite` | SQLite database file, including uploaded icon blobs |
| `VESTA_BIND` | `0.0.0.0:3000` | Listen address |
| `VESTA_STATIC_DIR` | `static` | Static assets directory |
| `VESTA_COOKIE_SECURE` | `false` | Set to `true` when served through HTTPS, so the login cookie requires a secure connection |
| `RUST_LOG` | `vesta=info` | Log filter (`tracing` syntax) |

Keep the data volume private and back it up. `config.toml` is the source of truth for servers, independent dashboard tiles, and app preferences; Vesta writes it atomically as `[[items]]` entries. Tiles can share an optional group label while services and widgets remain independent. Older `[groups.<id>]` layouts migrate to independent tiles while retaining their group names. SQLite is limited to the admin password, session records, and uploaded icon bytes. SQLx migrations are embedded and applied at startup.

## Dashboard settings

Settings manages app/server configuration and provides a preview catalog. The homepage's **Edit dashboard** mode manages the independent tile canvas:

- Servers: stable ID, display name, optional hostname, device name and icon, metrics endpoint, and scrape interval.
- Service tiles: title, destination, optional live availability check, image URL or uploaded icon, and tile size/position.
- Widget tiles: choose a built-in handler, set a display title, configure its structured options, and arrange/resize it independently of service links.
- Put related service and widget tiles in the same named section; sections can be reordered, widened or narrowed, and always stay visible even when empty.
- Drag any non-control area of a tile to arrange it on desktop, resize it, or use the edit dialog's accessible controls; invalid tiles disable dashboard saving. Types enforce minimum sizes (services 3×1; per-widget minimums) so tiles never collapse below usable space.
- App preferences: dashboard title and system/light/dark theme.

The dashboard starts empty. The administrator password is hashed with Argon2id; sessions use opaque HTTP-only, same-site cookies, with only a hash of each session token stored in SQLite. Set `VESTA_COOKIE_SECURE=true` when HTTPS is terminated by a reverse proxy.

## Nodes and telemetry

Add nodes in **Settings → Servers**:

- `local` reads the Vesta host using `sysinfo`.
- An `http://` or `https://` Prometheus text-format endpoint (commonly `node_exporter` on port 9100) reads a remote host.

Vesta reports CPU, memory, disk, load average, and uptime where the data source provides them. Vesta does not currently collect network counters, WAN latency, or speed-test results.

`GET /api/dashboard` returns current node values plus independent tile data; `GET /api/dashboard/services` returns that tile data alone. Live node updates are sent over `GET /api/nodes/events`, and individual snapshots are available at `GET /api/nodes/{id}/stats`.

## Widgets and extensions

Built-in handlers are listed in Settings with structured controls based on each handler's configuration. The current catalog includes:

- `QuickLinks`
- `Clock`
- `Weather`
- `SonarrCalendar`

`SystemStats` is no longer offered for new tiles, but remains available to render saved configurations made with earlier versions. The React dashboard renders service cards and widgets from JSON APIs. Widget data is available at `GET /api/widgets/{widget}?id=…`; service availability is available at `GET /api/ping?id=…`. Authenticated Settings APIs save dashboard layout and test availability-check URLs. Live server telemetry is delivered over Server-Sent Events. GitHub URL installation and third-party widget execution are **not implemented**. A future installer should use a versioned, permission-limited sandbox boundary rather than execute repository code inside the Vesta process.

Planned but not included yet: WAN latency/speed testing, Jellyfin recommended/random anime posters, notes/todo scratchpad, AdGuard widgets, and GitHub-installed plugins.

## Development

```bash
cargo run
bun install --frozen-lockfile
bun run build
```

For local development, Vesta reads and writes `./config.toml` and creates `./data/vesta.sqlite`; set `VESTA_CONFIG` and `VESTA_DATABASE` to separate paths for disposable runs.

## Credits

- [walkxcode/dashboard-icons](https://github.com/walkxcode/dashboard-icons): icon inspiration.

## Legal

This project is licensed under the GNU General Public License v3.0; see [LICENSE](LICENSE).

All product names, logos, and brands shown or mentioned are property of their respective owners.
