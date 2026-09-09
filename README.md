<div align="center">

# R-BMS

**A BMS rhythm-game player in Rust — reference-accurate judgment, sample-clocked audio, wgpu rendering.**

[Features](#features) · [Build](#build) · [Controls](#controls) · [Development](#development)

</div>

R-BMS reimplements the core PLAY loop of a GPL-3.0 reference implementation in Rust. Judgment windows, gauges and TOTAL scaling are cross-checked against the reference implementation's source, the play clock is the number of audio samples actually played, and every mode, skin, key map and setting is data, not code. It is a single native binary with no JVM and no runtime dependencies.

## Features

- **Play** — 5K / 7K / 9K (pop'n) / 10K / 14K auto-detected from the chart, normal and long notes with LN / CN / HCN, mines, BGA images, key beams and hit bombs
- **Judgment** — reference implementation `JudgeProperty` windows per mode, `#RANK` / `#DEFEXRANK`, judge offset with one-play auto-calibration, judge width, FAST / SLOW, empty POOR handled like the reference implementation
- **Gauges** — ASSISTED EASY / EASY / NORMAL / HARD / EX-HARD / HAZARD with TOTAL scaling, clear lamps, DJ LEVEL and EX score
- **Options** — MIRROR / RANDOM / S-RANDOM / R-RANDOM / H-RANDOM / ALL-SCRATCH / ROTATE (seeded, DP sides independent), hi-speed with constant-speed mode and green number, lift, lane cover, scratch side and auto-scratch
- **Song select** — recursive library scan, folders and difficulty tables (`data.json`), search and sort, `#PREVIEW` or autoplay preview, cover art, note density graph, local records with replay playback
- **Result** — judge counts, FAST / SLOW, max combo, gauge, clear lamp, rank graph and delta against your best
- **Replays** — saved per play, replayable with the same seed and options, analysis mode with seek, speed and per-note timing
- **Skins and themes** — play field and HUD from a RON skin, UI chrome colours from a RON theme, any TTF/OTF font with full Unicode fallback
- **Input** — every lane and control key rebindable in-app, conflict detection
- **IR** — optional score submission to a custom server (`--server`, `--player`)
- **Debug** — on-screen overlay with FPS, memory, audio clock and judge state

## Build

Requires Rust 1.95 or newer. Binaries are not published yet.

```sh
cargo build --release -p rbms-player
./target/release/rbms-player <songs folder>      # song select
./target/release/rbms-player <chart.bme>         # play one chart (autoplay)
./target/release/rbms-player <chart.bme> --interactive
```

Settings, key config, difficulty tables, local scores and replays live in `~/.config/rbms/` (`%USERPROFILE%\.config\rbms\` on Windows) and are created on first run. Run without arguments to open the last library.

Options: `--interactive` `--auto` `--hispeed F` `--gauge NAME` `--lift F` `--sc-left` `--sc-auto` `--keys Z,S,X,...` `--skin file.ron` `--font file.ttf` `--table URL` `--keyconfig path.ron` `--replay file.ron` `--server URL` `--player ID`.

## Controls

| Key | Song select | Play |
| --- | --- | --- |
| ↑ ↓ | Move | Hi-speed |
| → / Enter | Open / play | — |
| ← / Esc | Back / quit | Leave (result if no notes remain) |
| Tab | Settings | — |
| ← → (play) | — | Lane cover |
| [ ] | — | Lift |
| O / T / R / `/` / F3 | Folders / tables / records / search / sort | — |

Default lanes follow the reference keyboard layout: 7K `Z S X D C F V` + `LShift` (scratch), 9K `Z S X D C F V G B`, 14K adds `M K , L . ; /` + `RShift`. Everything is remappable in Settings → KEY CONFIG or `~/.config/rbms/keyconfig.ron`.

## Development

```sh
cargo test --workspace
cargo clippy --workspace --all-targets
cargo run -p rbms-render --example render_select   # headless render to PPM
```

Workspace crates, bottom up: `rbms-model` (chart types) → `rbms-parser` (BMS lexer, `#RANDOM` / `#SWITCH`, MD5 + SHA-256) → `rbms-chart` (timing, lanes, shuffle, scroll) → `rbms-judge` (windows, matcher, gauges) → `rbms-audio` (cpal + symphonia, real-time mixer) / `rbms-render` (`Renderer` trait, CPU reference canvas, skins, fonts) / `rbms-ir` / `rbms-table` → `rbms-play` (session driver) → `apps/rbms-player` (winit + wgpu). CI builds a macOS universal binary and Windows on every push to `dev`.

Architecture, the reference parity ledger, decisions and the improvement plan live in [docs/](docs/README.md) (Korean). Start with [docs/PROCESS.md](docs/PROCESS.md).

## License

[GPL-3.0-or-later](LICENSE). The play logic is a clean-room Rust port of a GPL-3.0 reference implementation.
