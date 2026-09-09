# Development guide

For picking the project up in a fresh session.

## What this is

A **Rust** BMS rhythm-game player — a port of the reference implementation's PLAY core. **Not** a web/TS project; any
generic "frontend" assumptions do not apply. Cargo workspace, edition 2024, rust ≥ 1.95, GPL-3.0.

## Build / run / test

```bash
cargo build --release -p rbms-player          # build the player
cargo test  --workspace                        # ~887 unit tests, all green
cargo build --workspace --examples             # build crate examples too
cargo clippy --workspace                        # informational only (CI is NOT gated on clippy/fmt)

./start.sh ["<song folder | chart | --replay file>"]   # local launcher (gitignored): debug build + run
./start.sh -r ...                              # release build instead
```

- `./start.sh` with no args opens the GUI on `./assets/songs` (created empty if missing).
- The app reads/writes config under `~/.config/rbms/`. `config_dir()` resolves `HOME` then
  `USERPROFILE` (then `.`), so on Windows (where `HOME` is unset) config lands in
  `%USERPROFILE%\.config\rbms`.

## Persistence (`~/.config/rbms/`, all RON, all `#[serde(default)]`)

| File | What |
|---|---|
| `settings.ron` | play options (`PlaySettings`), incl. `server_url` / `player_id` (IR/NETWORK) |
| `keyconfig.ron` | lane/control key bindings |
| `scores.ron` | local score history (`ScoreBook`) |
| `tables.ron` | difficulty-table sources |
| `folders.ron` | **multi-folder** song library (union scanned) |
| `theme.ron` | **UI theme** colours — written as an editable template on first run (see `theme.md`) |
| `replays/` | saved replays (`<md5[:8]>-<ms>.ron`) |

Resilient load: a corrupt file is renamed to `.ron.bak` and defaults are used (so the next save can't
clobber it). This applies to keyconfig / settings / scores / **tables / folders**.

## App module layout (`apps/rbms-player/src/`)

The crate is a library plus a three-line binary: `main.rs` is `fn main() -> ExitCode { rbms_player::run(std::env::args()) }`,
and `lib.rs` is the module root — the crate's consts and free helpers, `App` / `AppShared`, `App::new`,
the frame loop, `ApplicationHandler` and `run`. Everything else hangs off it:

| File | Holds |
|---|---|
| `lib.rs` | `App` / `AppShared`, consts, free helpers, the frame loop, `ApplicationHandler`, `run() -> ExitCode` |
| `assets.rs` | bundled skins, the `theme.ron` template, chart-relative file resolution, the keysound decode pool, BGA decode, the library scan |
| `gpu.rs` | the wgpu instanced-quad renderer (`Gpu`), fallible so a machine with no adapter gets a message |
| `stage/mod.rs` | `Stage`, `StageId`, `Transition`, `FrameCtx`, `KeyInput`, the `StageHandler` trait and its dispatch |
| `stage/{select/,settings,keyconfig,tables,folders,loading,play,result}.rs` | one screen each, owning that screen's own state. `select/` is split again into `mod.rs` (list, record modal, ranking panel), `preview.rs` (hover preview) and `scene.rs` (what the renderer is handed) |
| `app_input.rs` / `app_library.rs` / `app_network.rs` / `app_play.rs` / `app_ranking.rs` | `AppShared` methods by theme: input mapping, library and tables, the NETWORK tab and sync, chart load / shared stream / song clock, IR ranking |
| `format.rs` / `tablesrc.rs` / `settings_ui.rs` / `settings_view.rs` / `ir_*.rs` / `play_sink.rs` / `timing.rs` | display formatting, table loading, the settings rows the program owns, the settings renderer, the IR panels and sync, the `SoundSink` adapter, the timing probe |
| `keyconfig.rs` | the winit `KeyCode` ↔ token map and the `keyconfig.ron` shape (the one persisted type still in the app, because it is the one that needs winit) |

The screen and `app_*` modules use `use crate::*;` so they see the crate-root items, and they are
*descendants* of the crate root — which is what lets them read `AppShared`'s private fields without
`App` / `AppShared` having to expose them. Everything persisted other than the key config lives in
`rbms-config` (settings, folders, tables) or `rbms-store` (scores, replays).

## How to extend

- **New play mode**: add a `Mode` constant + its `[i8;18]` channel map in `rbms-model` — engine code
  does not branch on mode (it is data). Judge windows: add a row to `crates/rbms-judge/data/judge.ron`
  keyed by the mode name if it needs different timing; `JudgeProperty::for_mode` reads that file and
  only falls back to the compiled-in table for a mode the file does not name.
- **Note-field skin**: `assets/skins/*.ron` (`SkinConfig`), or `--skin file.ron`.
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
- Branches: `dev` = work, `prod` = deploy/release (CI auto-bumps + builds universal macOS / Windows).
- Tests are edge-case heavy and pin some intentional/suspect behaviours — see `roadmap.md` before
  "fixing" a surprising assertion.
- The deterministic `CpuCanvas` backend makes rendering testable without a GPU; prefer it in tests.

## See also

- `architecture.md` — crate map, data flow, play loop, determinism.
- `crates.md` — per-crate API reference.
- `theme.md` — the `theme.ron` schema (web-FE reference).
- `roadmap.md` — deferred work + pinned edge-case behaviours.
