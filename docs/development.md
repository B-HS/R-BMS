# Development guide

For picking the project up in a fresh session.

## What this is

A **Rust** BMS rhythm-game player — a port of beatoraja's PLAY core. **Not** a web/TS project; any
generic "frontend" assumptions do not apply. Cargo workspace, edition 2024, rust ≥ 1.95, GPL-3.0.

## Build / run / test

```bash
cargo build --release -p rbms-player          # build the player
cargo test  --workspace                        # ~880 unit tests, all green
cargo build --workspace --examples             # build crate examples too
cargo clippy --workspace                        # informational only (CI is NOT gated on clippy/fmt)

./start.sh ["<song folder | chart | --replay file>"]   # local launcher (gitignored): debug build + run
./start.sh -r ...                              # release build instead
```

- `./start.sh` with no args opens the GUI on `./assets/songs` (created empty if missing).
- The app reads/writes config under `~/.config/rbms/`.

## Persistence (`~/.config/rbms/`, all RON, all `#[serde(default)]`)

| File | What |
|---|---|
| `settings.ron` | play options (`PlaySettings`) |
| `keyconfig.ron` | lane/control key bindings |
| `scores.ron` | local score history (`ScoreBook`) |
| `tables.ron` | difficulty-table sources |
| `folders.ron` | **multi-folder** song library (union scanned) |
| `theme.ron` | **UI theme** colours — written as an editable template on first run (see `theme.md`) |
| `replays/` | saved replays (`<md5[:8]>-<ms>.ron`) |

Resilient load: a corrupt file is renamed to `.ron.bak` and defaults are used (so the next save can't
clobber it). This applies to keyconfig / settings / scores / **tables / folders**.

## App module layout (`apps/rbms-player/src/`)

`main.rs` was split (it was a 3300-line monolith). The `App` struct + its enums/consts/free-fns +
`impl ApplicationHandler` + `fn main` live in `main.rs`; the `impl App` methods are split by theme:

| File | Holds |
|---|---|
| `main.rs` | `App` struct, consts, free helpers, `App::new`, `ApplicationHandler`, `fn main`, theme loading |
| `gpu.rs` | the wgpu instanced-quad renderer (`Gpu`) |
| `app_input.rs` | input mapping, settings round-trip, replay/analysis, key-config editor methods |
| `app_select.rs` | song-select, library, **folders**, **search/sort**, difficulty tables, loading transitions, click hit-test |
| `app_play.rs` | chart `load`, the per-frame play loop, song clock, result + score submission, **all menu-screen rendering** |
| `format.rs` / `ir_map.rs` / `tablesrc.rs` | pure formatting / enum-mapping / table-loading helpers |
| `keyconfig.rs` / `scores.rs` / `settings.rs` / `replay.rs` / `tables.rs` / `folders.rs` | RON-persisted support types |

The split modules use `use crate::*;` so they see all the crate-root items; moved methods are
`pub(crate)`.

## How to extend

- **New play mode**: add a `Mode` constant + its `[i8;18]` channel map in `rbms-model` — engine code
  does not branch on mode (it is data). Judge windows: add a row in `JudgeWindows::note_for_mode` /
  `ln_end_for_mode` if the mode needs different timing.
- **Note-field skin**: `assets/skins/*.ron` (`SkinConfig`), or `--skin file.ron`.
- **UI theme**: `~/.config/rbms/theme.ron` (`ThemeConfig`); add a field to `rbms_render::theme::Theme`
  + route one `Color::rgb(..)` through `theme()`. See `theme.md`.

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
