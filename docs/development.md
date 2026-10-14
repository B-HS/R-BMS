# Development guide

Last updated 2026-10-14 · matches 7095ef7

For picking the project up in a fresh session.

## What this is

A **Rust** BMS rhythm-game player — a port of the reference implementation's PLAY core — plus `web/`, the score server and website it talks to (Next.js, TypeScript, bun). The player is a Cargo workspace, edition 2024, rust ≥ 1.95 (the toolchain is pinned to 1.95.0), GPL-3.0; it has no web/TS code, so generic "frontend" assumptions do not apply to it. `web/` has its own rules in `web/AGENTS.md` (arrow functions, no `any`/`enum`/comments, FSD, TanStack Query, Zod at the boundary, no `useCallback`/`useMemo`) and its own gate, below.

## Build / run / test

```bash
cargo build --release -p rbms-player
cargo build -p rbms-player --no-default-features   # without the `video` feature (no OpenH264)
cargo test --workspace --no-fail-fast --locked     # what CI runs
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
git diff --check

cargo run --release -p rbms-player --          # source: remembered folder, RBMS_SONGS, or empty GUI
cargo run --release -p rbms-player -- <folder> # source: song select
cargo run --release -p rbms-player -- <chart.bms>
cargo run --release -p rbms-player -- <chart.bms> --interactive
./target/release/rbms-player <folder>          # macOS / Linux built binary
./target/release/rbms-player <chart.bms> --auto
.\target\release\rbms-player.exe <folder>     # Windows PowerShell built binary
.\target\release\rbms-player.exe <chart.bms> --auto
```

- The wave 20 gate (2026-10-14) passed 5,671 Rust tests with 0 failures (a few are ignored: they need an audio or graphics device, wait five seconds, or read a copy of a player's own database) and 415 web tests; later commits added a few of each. The numbers are not a contract. CI gates formatting and the clippy command above, and builds/tests on Linux, macOS, and Windows with `cargo test --workspace --no-fail-fast`, so one run lists every failure of a platform.
- **After every push, read the CI result.** `gh run list --branch dev --limit 3` shows the latest runs; `gh run view <id>` shows the three OS jobs and `gh run view <id> --log-failed` the failing output. A local pass is not evidence for the other two operating systems. Fix a failing run before the next unit of work (`docs/feedback/2026-10-11-ci-not-checked-after-push.md`).
- The default Cargo features of `rbms-player` include `video`, which links OpenH264 (built from C source by `cc`; no extra install step on any OS). `--no-default-features` builds and tests the player without a codec. AUDIO, Practice, Course and Multi-IR have no completed hardware/server manual result; their required procedure is [quality-assurance/2026-09-16-phase-h-manual-checks.md](quality-assurance/2026-09-16-phase-h-manual-checks.md).
- The first positional argument is a song folder or chart. With no positional argument, the player uses the remembered `songs_folder`, then `RBMS_SONGS`, otherwise opens an empty GUI. A single chart defaults to autoplay; use `--interactive` or `--auto` to choose explicitly. The execution source of truth is the repository `README.md`.
- To register a folder in the GUI: `O` → `+ ADD FOLDER` → `Enter` → `Esc` (save and rescan).
- The app reads/writes config under `~/.config/rbms/`. `config_dir()` resolves `HOME` then
  `USERPROFILE` (then `.`), so on Windows (where `HOME` is unset) config lands in
  `%USERPROFILE%\.config\rbms`.

## The website (`web/`)

```bash
cd web
bun install --frozen-lockfile
cp .env.example .env.local   # then fill the values (DATABASE_URL, BETTER_AUTH_SECRET, BETTER_AUTH_URL, NEXT_PUBLIC_APP_URL)
bun run db:migrate           # apply web/drizzle/*.sql; never `drizzle-kit push`
bun run dev                  # http://localhost:3000
bun run verify               # typecheck (next typegen + tsc) · eslint · bun test
bun run format:check         # prettier
bun run build                # next build; works with no environment variables
bun run db:generate          # after a change to web/src/server/db/schema.ts: writes the next migration
bun run db:status            # which migrations a database has
```

- The gate for a change under `web/` is `.github/workflows/web.yml` (install frozen, `verify`, `format:check`, a build). It does not run for a Rust-only change, and `ci.yml` does not run for a web-only one, except that `ci.yml` also runs when `web/data/tables/**` changes because the player embeds those files.
- Production is Vercel (project `rbms-web`, root `web`, production branch `dev`); a push to `dev` that changes `web/` deploys. The production database is touched only through the manual `web-db.yml` workflow (status, migrate, count or delete the test accounts), using the repository secret `WEB_DATABASE_URL`; never read or write `.env*` files from an agent.
- `web/data/tables/insane1/{header,data,source}.json` is the snapshot of the 発狂BMS難易度表 the site serves and the player carries (`include_bytes!`); `bun scripts/snapshot-table.ts <table-url> <output-dir>` (run in `web/`) writes `header.json`, `data.json` and `source.json` into the folder you name (here `data/tables/insane1`); with `--check` it writes nothing and only compares (exit 0 same, 1 different, 2 failed).
- The wire contract with the player is tested on each side separately (`crates/rbms-ir` and `web/tests/dto/ir-contract.test.ts`). When the client changes what it sends, change both and run both gates.

## Environment variables

| Variable | Read by | Effect |
|---|---|---|
| `RBMS_SONGS` | the player | song folder when no argument and no remembered `songs_folder` |
| `RBMS_SKIN_PACK` | the player; the opt-in skin tests | skin pack folder for this run only, above `skin.pack` and the installed default pack; blank is ignored. Use an absolute path in tests |
| `RBMS_SKIN_SEED` | `rbms-skin` | pins wildcard file selection and a skin's `math.random` (a whole number) |
| `RBMS_SKIN_CAPTURE_DIR` | the capture harness (tests) | writes each captured frame as `<name>.png` (GPU frames as `<name>.gpu.png`) |
| `RBMS_REQUIRE_GPU` | the GPU pixel tests | makes a test that needs a graphics adapter fail instead of skipping when none comes up |
| `RBMS_GOLDEN_UPDATE` | `rbms-render` golden tests | regenerates the built-in screen golden images |
| `RBMS_TIMING_CSV`, `RBMS_SOAK_LOG` | the player | timing sample dump and periodic stability log (same as `--timing-csv`) |
| `RBMS_SCOREDB_COPY` | one ignored player test | path of a copy of a player's own `scoredb.sqlite`; the test reads it with this build and reports how its rows are read (`cargo test -p rbms-player --lib -- --ignored a_players_own_database`). Never point it at the live file |
| `RBMS_SKIN_EXPORT_DIR` | one skin-menu test | with `RBMS_SKIN_PACK`, a folder outside the repository the pack is exported into |
| `RBMS_IR_E2E_URL` | `crates/rbms-ir/tests/e2e_local.rs` | base URL of a local score server for the end-to-end test; the test is skipped when it is unset |
| `HOME`, `USERPROFILE` | the player | the config dir is `$HOME/.config/rbms`, else `%USERPROFILE%\.config\rbms`, else `./.config/rbms` |

## Persistence (`~/.config/rbms/`)

| File | What |
|---|---|
| `settings.ron` | versioned `Config` (schema 6): play, judge, library folders and difficulty tables (and `seeded_default_tables`, the default tables already given), display, audio, network (server address, the signed-in login id and bearer token, extra IR profiles), skin |
| `keyconfig.ron` | lane, control and shortcut bindings for keyboard and pad. `controls` holds the 20 in-play controls (`start` / `select` default to A / W, `cover_toggle` to BACKSLASH; the ones added in wave 10A, `fine` among them, default to empty = unbound). `shortcuts` maps a shortcut's name to a key word (no entry = the shipped key, an empty string = unbound). An older file without these loads with the defaults and is not rewritten on load. The editor opens with F5 from the song select or Settings; what each control does is in [play-options.md](./play-options.md) |
| `songdb.sqlite` | scanned song-library index and chart-detail cache |
| `scoredb.sqlite` | one merged best per chart and full local play history (`scorelog` has a row for every run played by hand, with the judging-rule generation and the assist level; the best row is built only from runs that may set one) |
| `scores.ron.migrated` | a legacy `scores.ron` after its first successful import into `scoredb.sqlite` |
| `tables/` | the copy of each fetched difficulty table, `<hash of the address>.json`; the default table's copy is written the first time it is given |
| `user-folders/` | the copy of the signed-in account's folders (`MY FOLDERS`), `<hash of the server and the login id>.ron`; deleted on sign-out |
| `favorites.ron` | starred chart MD5 values |
| `practice.ron` | per-chart practice properties |
| `theme.ron` | UI-theme file; a template that sets no colour is written on first run, and an untouched template of an earlier version is replaced (see `theme.md`) |
| `replays/` | saved replays (`<md5[:8]>-<ms>.ron`) |
| `courses/` | user course documents |
| `skin/rbms-default/` | the default skin pack, written by every start from the copy embedded in the binary (`install-manifest.txt` records what was installed; a file the player edited is left alone). Other packs may sit beside it |
| `skin-data/<pack id>/` | what a skin's Lua wrote (its write overlay), one folder per pack, created on the first write |

`settings.ron` includes the current schema version. A parse failure moves it to `*.ron.bak`; a schema migration first copies it to `*.ron.v<version>.bak`. The legacy `folders.ron` and `tables.ron` files are read only when migrating an old settings document, then folded into `settings.ron`. A legacy `scores.ron` is imported into SQLite only when `scoredb.sqlite` is first created; if the database cannot open, the player falls back to reading that legacy score book.

## App module layout (`apps/rbms-player/src/`)

The crate is a library plus a three-line binary: `main.rs` is `fn main() -> ExitCode { rbms_player::run(std::env::args()) }`,
and `lib.rs` is the module root — the crate's consts and free helpers, `App` / `AppShared`, `App::new`,
the frame loop, `ApplicationHandler` and `run`. Everything else hangs off it:

| File | Holds |
|---|---|
| `lib.rs` | `App` / `AppShared`, consts, free helpers, the frame loop, `ApplicationHandler`, `run() -> ExitCode` |
| `assets.rs`, `assets/default_pack.rs`, `assets/theme_file.rs`, `assets/scripted_movie.rs` | built-in note-field layouts, skin pack identifiers and overlay folders, the `theme.ron` template and how an earlier one is told from a player's own file (`theme_file.rs`), chart-relative file resolution, the keysound decode pool, BGA decode, the library scan; the embedded default pack and its install rules (`build.rs` makes the file table); movie stand-ins for tests |
| `gpu/` | the wgpu instanced-quad renderer (`Gpu`, with `batch.rs` draw-call batching and `background.rs`), fallible so a machine with no adapter gets a message |
| `stage/mod.rs` | `Stage`, `StageId`, `Transition`, `FrameCtx`, `KeyInput`, the `StageHandler` trait and its dispatch |
| `stage/{select/,decide,settings,skin_menu,play_menu,keyconfig,tables,folders,loading,play/,result,practice,course_result}.rs` | one screen each, owning that screen's own state. `skin_menu.rs` is the skin settings with a live preview (`F12`); `play_menu.rs` (+ `play_menu/rows.rs`) is the play options with a live play screen (`F8`); `preview_menu.rs` is what the two draw and move the same way. `select/` is split into `mod.rs` (list, record modal, ranking panel and course tab), `keys.rs` (key tables), `panel.rs` and `events.rs` (option panels 1 to 3 and skin events), `skinned.rs` (the skin-drawn browser), `overlay.rs` and `guide.rs` (what the app draws over it), `preview.rs` (hover preview), `list.rs`, `filter.rs` and `scene.rs` (what the renderer is handed). `decide.rs` is the scene between picking a chart and its run; `scene_life.rs` its timing rules, shared with result. `play/` holds the state machine (PRELOAD, READY, PLAY, FAILED, FINISHED). `practice.rs` owns practice setup; `course_result.rs` renders the aggregate course result. `capture.rs` (+ `capture/gate.rs`) is the test-only capture harness. |
| `skin_select.rs` / `skin_screen.rs` / `skin_host/` / `scene_clock.rs` | the skin plumbing: `SkinLibrary` (pack, documents, loads, the skin menu's rows), compiled screens and texture decode (`skin_screen/preview.rs` and `skin_preview.rs` draw one screen's skin with sample state inside the skin menu), `ScreenHost` (the `SkinHost` implementation, one file per cluster of property ids), the scene clock |
| `window_mode.rs` / `pointer.rs` / `textedit.rs` / `toast.rs` / `notify.rs` | window resolution and mode, mouse events as `PointerInput`, the single-line text editor, the notification bus and its on-screen strips |
| `app_input.rs` / `app_network.rs` / `app_options.rs` / `app_play.rs` / `app_ranking.rs` / `app_result.rs` | `AppShared` methods by concern: keyboard and gamepad input, NETWORK tab and sync, option overlay, chart load / shared stream / song clock, IR ranking and result persistence or submission. |
| `library.rs` / `scoredb_store.rs` / `practice.rs` / `course_ui.rs` / `course_ir.rs` | Phase G persistence and modes: SQLite song index, SQLite score import and history, per-chart practice properties and clock, course selection/constraints/results, and course IR submission. |
| `target.rs` / `user_folders.rs` / `favorites.rs` / `tablesrc/seed.rs` / `judge_setup.rs` | what a run is paced against (TARGET, TARGET FALLBACK), the account's own folders (`MY FOLDERS`), starred charts, the default difficulty table, and how a run's judge settings are read and whether they count as an assist. |
| `gamepad/` / `syssound.rs` / `ir_ext.rs` | Phase G device and network extensions: gilrs pad state and analog mapping, optional system-sound set, and primary plus additional IR profile fan-out. |
| `format.rs` / `tablesrc.rs` / `settings_ui.rs` / `settings_view.rs` / `ir_*.rs` / `play_sink.rs` / `timing.rs` | display formatting, table loading, the settings rows the program owns, the settings renderer, IR panels/session/ranking/replay/sync, the `SoundSink` adapter, and the timing probe. |
| `keyconfig.rs` | the winit `KeyCode` ↔ token map and the `keyconfig.ron` shape (the one persisted type still in the app, because it is the one that needs winit) |

The screen and `app_*` modules use `use crate::*;` so they see the crate-root items, and they are
*descendants* of the crate root — which is what lets them read `AppShared`'s private fields without
`App` / `AppShared` having to expose them. `rbms-config` owns the versioned settings schema;
`rbms-library` and `rbms-store` provide the SQLite song and score stores that the player wires to
the configuration directory.

## How to extend

- **New play mode**: add a `Mode` constant + its `[i8;18]` channel map in `rbms-model` — engine code
  does not branch on mode (it is data). Judge windows: add a row to `crates/rbms-judge/data/judge.ron`
  keyed by the mode name if it needs different timing; `JudgeProperty::for_mode` reads that file and
  only falls back to the compiled-in table for a mode the file does not name.
- **Built-in note-field layout**: `assets/skins/normal.ron` and `wide.ron` (`SkinConfig`), or `--skin file.ron`. They lay out the
  built-in play screen, which is the fallback for 24-key play and for a play document that is missing or fails; a run started with
  `--skin` draws its play screens with that layout whatever documents are chosen.
- **Skins** (the screens people normally see): `docs/skin.md` is the guide and `plan/2026-10-09-lua-skin-compat.md` the
  specification. To change the default skin, edit `assets/skins/rbms-default/` (colours in `lua/theme.lua`, layout in
  `lua/<screen>/`, sheets in `tools/sections/<section>.py`); the next build embeds the new files (`apps/rbms-player/build.rs`
  walks the folder, so a file added or removed needs no list edit), and `cargo run -p rbms-cli -- skin-dump assets/skins/rbms-default --strict --frames 120`
  checks it. A skin can be tried without installing anything: `RBMS_SKIN_PACK=/path/to/pack cargo run --release -p rbms-player`.
  To add a property id answer or an event to the host, add it to the cluster in `apps/rbms-player/src/skin_host/` that owns where
  the value comes from and to the `ROUTES` / `EVENT_ROUTES` table in `skin_host/mod.rs`.
- **Default skin images and sounds** (regenerate, never hand-edit the PNGs or `lua/atlas/*.lua`):
  `uv run assets/skins/rbms-default/tools/generate-images.py [section ...] [--check]` and
  `uv run assets/skins/rbms-default/tools/generate-sounds.py`. Commit the outputs; CI does not run uv. Details in `skin.md` §3.5.
- **UI theme**: `~/.config/rbms/theme.ron` (`ThemeConfig`); add a field to `rbms_render::theme::Theme`
  + route one `Color::rgb(..)` through `theme()`. See `theme.md`.
- **Settings screen**: add a `SettingId` variant and a `SETTINGS` row in `rbms_config::settings`
  (tab, label, `SettingKind`, help, visibility) — the screen calls `tab_rows` → `display_value` and
  reacts to `AdjustOutcome::Action(id)`, so there is no index space to keep in step. A row whose
  value only the running program knows (the live account, the password held in memory, a skin forced
  on the command line) is marked `host_value` and rendered by `settings_ui` / `app_network`.
  Free-text rows edit in place through `SettingsState.text_input`, and the row the editor was
  *opened on* decides which field the commit writes.

## Conventions

- **Commits: NEVER add a `Co-Authored-By:` line** (or any AI attribution). Description only.
- Branches: `dev` = work, and it is the branch the website deploys from. Client releases are cut by hand from `dev` with `gh workflow run release.yml --ref dev -f bump=patch` (`release.yml` raises the version, tags it, builds and signs, and leaves a draft GitHub release); no `prod` branch is needed for a release (`origin` lists only `dev` and an old `feat/skin-followups` as of this commit). Do not publish a draft release: that is the maintainer's step. Details: `ci-release.md`.
- Tests are edge-case heavy and pin some intentional/suspect behaviours — see `roadmap.md` before
  "fixing" a surprising assertion.
- The deterministic `CpuCanvas` backend makes rendering testable without a GPU; prefer it in tests.
- Rust style (`rustfmt.toml`: `max_width = 160`): no `//` comments (only `//!` and `///` documentation, in English), no magic numbers (name the constant), no `#[allow]` or `unsafe`.
- `Cargo.lock` is tracked and CI and release builds pass `--locked`. When a dependency changes, commit `Cargo.lock` with it and regenerate the tables of `THIRD-PARTY-NOTICES.md` (see `ci-release.md` section 2.1).
- A wave or unit is committed only after the gate (fmt, clippy `-D warnings`, `cargo test --workspace --no-fail-fast`, `git diff --check`) passes; push, then check CI as described above.

## See also

- `architecture.md` — crate map, data flow, play loop, determinism.
- `crates.md` — per-crate API reference.
- `theme.md` — the `theme.ron` schema (web-FE reference).
- `skin.md` — skins: pack selection, the default pack, the Lua environment, `skin-dump`, known limits.
- `ci-release.md` — the CI matrix, release workflow and the third-party notices that ship with a release.
- `roadmap.md` — deferred work + pinned edge-case behaviours.
