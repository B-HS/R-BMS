<div align="center">

# R-BMS

**A BMS rhythm-game player in Rust — reference-accurate judgment, sample-clocked audio, wgpu rendering.**

[Features](#features) · [Build](#build-and-run) · [Controls](#controls) · [Development](#development)

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
- **Skins and themes** — reference-format JSON skins (json5, sandboxed Lua expressions, per-screen selection and customisation from the SKIN tab), RON defaults for the play field and HUD, UI chrome colours from a RON theme, any TTF/OTF font with full Unicode fallback
- **Input** — every lane and control key rebindable in-app, conflict detection
- **IR** — sign in, submit scores, browse rankings, sync settings, manage rivals and upload or replay ghosts, all from the in-game SETTINGS NETWORK tab and the song-select ranking panel
- **Debug** — on-screen overlay with FPS, memory, audio clock and judge state

## Build and run

Requires Rust 1.95 or newer. The release workflow is present, but this repository has not completed its first `prod` release; do not assume a published binary is available.

```sh
cargo build --release -p rbms-player
```

`README.md` is the execution reference. The first positional argument is a song folder or one chart. With no positional argument, the player opens the remembered `songs_folder`, then `RBMS_SONGS` when set, otherwise an empty GUI.

macOS / Linux:

```sh
cargo run --release -p rbms-player --                         # source: remembered folder, RBMS_SONGS, or empty GUI
cargo run --release -p rbms-player -- /path/to/songs           # source: song select
cargo run --release -p rbms-player -- /path/to/chart.bms       # source: single chart, autoplay
cargo run --release -p rbms-player -- /path/to/chart.bms --interactive
./target/release/rbms-player /path/to/songs                    # built binary
./target/release/rbms-player /path/to/chart.bms --auto          # built binary, explicit autoplay
```

Windows PowerShell:

```powershell
cargo run --release -p rbms-player --                          # source: remembered folder, RBMS_SONGS, or empty GUI
cargo run --release -p rbms-player -- C:\path\to\songs       # source: song select
cargo run --release -p rbms-player -- C:\path\to\chart.bms --interactive
.\target\release\rbms-player.exe C:\path\to\songs          # built binary
.\target\release\rbms-player.exe C:\path\to\chart.bms --auto # built binary, explicit autoplay
```

To register a library through the GUI, press `O`, select `+ ADD FOLDER`, press `Enter` to choose the folder, then press `Esc` to save and rescan.

Settings, key config, difficulty tables, local scores and replays live in `~/.config/rbms/` (`%USERPROFILE%\.config\rbms\` on Windows) and are created on first run.

The first run also installs the editable default skin bundle under `~/.config/rbms/skin/steel-neon-v3/` (the equivalent settings folder on Windows). Every screen is an object-level JSON5 document: song wheel, detail panel, option panel, notes, gauge, judgement pop-ups, lane covers, HUD numbers, result tables and graphs are individual objects you can move, recolour or replace. Bundle-wide choices (play side, BGA size, graph position, judge timing, lane cover art, offsets) live in the SKIN tab and apply to every document of the bundle. `play.ron` / `play-dual.ron` are the fallback field layouts, `theme.ron` holds song-select layout and UI colours, `palette.json` plus `tools/generate-assets.py` regenerate the artwork, `tools/generate-sounds.py` the system sound set in `sound/`. Use Settings → SKIN → RELOAD to reread screen documents, or restart the player after editing the RON files. Existing edited files and explicit screen choices are preserved on later runs; see `docs/skin.md` for the document contract.

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

## Web (IR server + site)

`web/` is a Next.js app that serves both the IR API the player talks to and the public site (leaderboards, charts, players, replays, settings). The configured production endpoint is `https://bms.hyuns.uk`, but its current availability is an external deployment concern and must be checked before use.

When that endpoint is reachable, connect the player from inside the game, not from the command line: press Tab on the song select to open SETTINGS, go to the NETWORK tab, set `SERVER URL` to `https://bms.hyuns.uk/api` and `PLAYER ID` to your account id, then fill EMAIL / PASSWORD and run REGISTER or LOGIN. The bearer token is stored locally, the password never is. The same tab holds settings sync, rival management and the replay auto-upload toggle; press `I` on the song select for the IR ranking panel. The walkthrough endpoint is [`/guide`](https://bms.hyuns.uk/guide). `--server` and `--player` remain overrides for one run only.

```sh
cd web
bun install
cp .env.example .env.local        # fill DB_*, BETTER_AUTH_SECRET, URLs
bun run db:migrate                # Drizzle migrations (never push)
bun run dev                       # http://localhost:3000
bun run verify                    # typecheck · lint · test
```

Stack: Next.js 16 (App Router, Route Handlers, Cache Components), React 19, Tailwind v4 + shadcn, TanStack Query v5, Drizzle + MySQL, better-auth. Native endpoints under `/api/*` answer raw JSON exactly as `rbms-ir` expects; site-only endpoints live under `/api/fe/*`. External tools can call the API with a token from the site settings page and an `Authorization: Bearer` header. Design tokens, component list and the route/cache matrix are in [docs/web/](docs/web/architecture.md).

## Development

```sh
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo run -p rbms-render --example render_select   # headless render to PPM
```

Workspace crates, bottom up: `rbms-model` (chart types) → `rbms-parser` (BMS lexer, `#RANDOM` / `#SWITCH`, MD5 + SHA-256) → `rbms-chart` (timing, lanes, shuffle, scroll) → `rbms-judge` (windows, algorithms, gauges, RON tables) → `rbms-audio` (cpal + symphonia, real-time mixer, interpolated clock) / `rbms-render` (`Renderer` trait, CPU reference canvas, skin renderer, fonts) / `rbms-skin` (JSON skin loader) / `rbms-ir` / `rbms-table` / `rbms-store` (scores, replays) / `rbms-library` (scan) / `rbms-config` (settings schema, descriptors) / `rbms-course` (course model and run state) → `rbms-play` (PlaySession) → `apps/rbms-player` (winit + wgpu, stage-based app). CI runs tests on Linux, macOS and Windows with fmt and clippy `-D warnings` gates.

Architecture, the reference parity ledger, decisions and the improvement plan live in [docs/](docs/README.md) (Korean). Start with [docs/PROCESS.md](docs/PROCESS.md). Phase R (creating `prod`, first release/tag and signing or deployment credentials) is deliberately deferred until the maintainer supplies the required real keys.

## License

[GPL-3.0-or-later](LICENSE). The play logic is a clean-room Rust port of a GPL-3.0 reference implementation.
