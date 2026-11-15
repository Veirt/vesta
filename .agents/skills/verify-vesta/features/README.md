# Vesta verification feature map

Use these user-facing journeys as the maintained source of verification coverage. Each journey starts from a fresh disposable instance unless its Gotchas says otherwise.

- [First-run access](authentication.md) — create the local administrator, sign in, and sign out.
- [Dashboard overview](dashboard.md) — open the dashboard, navigate its sections, and observe telemetry/empty states.
- [Server telemetry](server-telemetry.md) — configure a local or remote metrics source and verify the dashboard status.
- [Dashboard tiles](services-widgets.md) — create independent service/widget tiles, arrange and resize them, test availability checks, and verify saved layout/previews.
- [Jellyfin Random Show](jellyfin-random-show.md) — verify the JellyfinRandomShow widget against a loopback mock through the populated, empty-library, and bad-credential paths.
- [Plugin system](plugins.md) — install a plugin from a git URL, add and render its widget tile, exercise the uninstall guard, and verify asset-serving boundaries.
- [Appearance settings](appearance-settings.md) — change dashboard title/theme and verify saved TOML plus the resulting dashboard.

The repeatable browser driver and disposable-instance lifecycle are in [the skill](../SKILL.md). The seed proof covers Appearance settings; a complete verification pass should also cover each remaining feature entry below.
