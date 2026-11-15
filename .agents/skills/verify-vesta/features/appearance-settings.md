# Appearance settings

The Appearance section controls the dashboard title and system/light/dark theme. Settings saves these values to the active `config.toml`, and the dashboard reflects the new title and theme.

## Sub-features

- Edit the required dashboard title.
- Select the system, light, or dark theme.
- Save or discard an unsaved Settings draft.
- Observe the saved title and theme on the dashboard after navigation/reload.

## How to get to it (user POV)

Open `/settings`, create the administrator on a fresh instance or sign in, then select `Appearance` (the default Settings section). Edit `Dashboard title` and `Theme`, choose `Save changes`, and use `Back to dashboard` to view the result.

## Driving it with playwright-cli

Use the seed journey in [the verification skill](../SKILL.md): capture the first-run form; create a disposable account through its visible form; edit the actual labeled controls and save:

```bash
./.agents/skills/verify-vesta/scripts/browser.sh "$RUN_ID" fill 'label:has-text("Dashboard title") input' 'Verification Home'
./.agents/skills/verify-vesta/scripts/browser.sh "$RUN_ID" select 'label:has-text("Theme") select' 'light'
./.agents/skills/verify-vesta/scripts/browser.sh "$RUN_ID" click 'button:has-text("Save changes")'
./.agents/skills/verify-vesta/scripts/browser.sh "$RUN_ID" click 'a:has-text("Back to dashboard")'
```

Capture the draft with its unsaved status and the dashboard after navigation. Before cleanup, copy the run's generated `config.toml` to the evidence directory and parse it with Python `tomllib`; require `[app].dashboard_title` to equal `Verification Home` and `[app].theme` to equal `light`. Keep the screenshots, snapshots, build/server logs, and TOML copy after cleanup.

## Gotchas

- The configured title/theme must be changed through the real form; editing `config.toml` directly does not prove the Settings journey.
- The config file is created on the first successful save, not at startup. If it does not exist after the save notice, treat persistence as failed and preserve the logs.
- Theme appearance may follow system preference before a choice is saved. Verify the selected setting and resulting UI rather than relying on an assumed OS color scheme.
