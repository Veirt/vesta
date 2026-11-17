# AGENTS.md — Vesta

Vesta is a self-hosted dashboard: a Rust (axum, SQLite) server serving a React + TypeScript single-page app. Rust lives in `src/`, the frontend in `frontend/src/` (built by `bun run build` into `static/`), tests live next to the code they own.

## Run and verify

- Format, lint, typecheck: `cargo fmt`, `cargo clippy --all-targets`, `bun run check` — all three before claiming anything. `bun run build` writes `static/` and must succeed for frontend changes.
- React-cod health: `bunx react-doctor` (364 lines is the largest allowed component; keep it under that by extraction, not suppression). The scanner config is `doctor.config.ts`; only add entries there with a written reason.
- Real user paths are proven in a browser via `.agents/skills/verify-<app>/`: launch an isolated instance, drive the visible controls, capture evidence in `artifacts/`. A check that cannot fail proves nothing; report VERIFIED / NOT VERIFIED / INCONCLUSIVE with what ran.

## Change discipline

- One purpose per commit, conventional-commit format, enforced by commitlint (see `commitlint.config.js`).
- Read a file before editing it; keep diffs minimal; delete dead code you touch; matching style beats personal preference.
- Widget configs are validated at load (backend) and while editing (frontend): adding a widget means touching the Rust handler, `config.rs` default size, and both schema files — follow the existing widget as the recipe.

## Architecture

- `src/` (Rust): axum routes in `main.rs`/`api.rs`/`settings_api.rs`; widget handlers in `src/widgets/` registered via `WidgetHandler`; the plugin system in `src/plugins/` clones git bundles and serves their static assets behind traversal-checked paths.
- `frontend/` (React 19 + Vite): `app.tsx` roots the page; `features/dashboard/` renders the grid (tiles, groups, edit dialogs); `features/settings/` renders the settings sections; widget data flows backend -> `/api/widgets/<name>` -> `useWidgetData` in `tile-renderers.tsx`.
- Errors: recover where the contract allows (bad input input -> 4xx with message), propagate with context otherwise; never swallow.
