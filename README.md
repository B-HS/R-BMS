<div align="center">

# R-BMS

**A BMS rhythm-game player in Rust — reference-accurate judgment, sample-clocked audio, wgpu rendering.**

[Features](#features) · [Build](#build-and-run) · [Skins](#skins) · [Controls](#controls) · [Development](#development)

</div>

Last updated 2026-10-11 · matches 03e1dd8 plus the wave 10A working tree (not yet committed).

R-BMS reimplements the core PLAY loop of a GPL-3.0 reference implementation in Rust. Judgment windows, gauges and TOTAL scaling are cross-checked against the reference implementation's source, the play clock is the number of audio samples actually played, and every mode, skin, key map and setting is data, not code. It is a single native binary with no JVM and no runtime dependencies.

## Features

- **Play** — 5K / 7K / 9K (pop'n) / 10K / 14K auto-detected from the chart, normal and long notes with LN / CN / HCN, mines, BGA images, key beams and hit bombs
- **Judgment** — reference implementation `JudgeProperty` windows per mode, `#RANK` / `#DEFEXRANK`, judge offset with AUTO ADJUST (the offset follows your timing as each input lands, like the reference's judge timing auto adjust), judge width, FAST / SLOW, empty POOR handled like the reference implementation
- **Gauges** — ASSISTED EASY / EASY / NORMAL / HARD / EX-HARD / HAZARD with TOTAL scaling, clear lamps, DJ LEVEL and EX score
- **Options** — MIRROR / RANDOM / S-RANDOM / R-RANDOM / H-RANDOM / ALL-SCRATCH / ROTATE (seeded, DP sides independent), hi-speed with constant-speed mode and green number, KEEP GREEN NUMBER (the hi-speed is worked out for every chart from one green number, its tempo and the lane cover), lift, lane cover, HIDDEN+, scratch side and auto-scratch
- **Song select** — recursive library scan, folders and difficulty tables (`data.json`), search and sort, `#PREVIEW` or autoplay preview, cover art, note density graph, local records with replay playback
- **Result** — judge counts, FAST / SLOW, max combo, gauge, clear lamp, rank graph and delta against your best
- **Replays** — saved per play, replayable with the same seed and options, analysis mode with seek, speed and per-note timing
- **Skins and themes** — a built-in default skin for every screen (select, decide, play 5/7/9/10/14 keys, result, course result) from the first run, with Normal and Wide (1.5x) lanes on the play screens; beatoraja-format Lua skins (`.luaskin`) and JSON skins load unmodified from a skin pack folder, with per-screen selection and customisation on the SKIN menu, which previews the skin as you change it; mp4 (H.264) video sources; any TTF font with full Unicode fallback; built-in screens and a RON theme remain as the fallback
- **Input** — lanes, in-play controls, song-select shortcuts, the keys that open each screen, result and replay-analysis keys all rebindable in-app on keyboard and controller, with conflict detection per screen
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

Every start installs the default skin pack under `~/.config/rbms/skin/rbms-default/` (the equivalent settings folder on Windows) from a copy built into the binary; see [Skins](#skins).

Options: `--interactive` `--auto` `--hispeed F` `--gauge NAME` `--lift F` `--sc-left` `--sc-auto` `--keys Z,S,X,...` `--skin file.ron` (a built-in play-field layout; the play screens are then drawn by the built-in field instead of a skin) `--font file.ttf` `--table URL` `--keyconfig path.ron` `--replay file.ron` `--server URL` `--player ID`.

## Skins

The default skin `rbms-default` draws every screen from the first run, with no setup. Its files are copied to `~/.config/rbms/skin/rbms-default/` on each start: a file you changed is left alone, an untouched file is replaced when an update ships a new version, and a file you deleted comes back. If that folder cannot be written the player warns once and draws its built-in screens.

To use another skin pack (a folder of beatoraja-format `.luaskin` files, for example one written for beatoraja; JSON skins work too):

1. Press `F12` on the song select to open the SKIN menu (the SKIN button of a skin opens it too, and so does `F12` or the `SKIN MENU >` button in SETTINGS). The settings are listed on the left; the right half draws the skin of the screen being configured with sample data, live.
2. On `PACK FOLDER` press `Enter` and choose the pack's folder. Pressing the left arrow clears it and returns to the default skin. For one run only, set `RBMS_SKIN_PACK=/path/to/pack` instead.
3. `Tab` (or a click on the strip above the preview) changes the screen. A screen the pack has no document for is drawn by the player's built-in screen, not by the default skin, and the preview says so. The same menu lets you pick a different document per screen and change the options, files and offsets a document offers; the preview follows each change. `Esc` saves and goes back.

Skins can read their own folder but cannot write into it, open files outside it, or reach the network. If a skin fails to load, the player says which file failed and draws the built-in screen. [docs/skin.md](docs/skin.md) (Korean) has the details: pack rules, the default skin, the Lua environment, the `rbms-cli skin-dump` checker and the known limits. Third-party notices for the bundled video decoder and font are in [THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md).

## Controls

These are the shipped keys. Press `H` on the song select for the full list as it is set right now.

| Key | Song select | Play |
| --- | --- | --- |
| ↑ ↓ | Move | Hi-speed (the held green number while KEEP GREEN NUMBER is on) |
| → / Enter | Open / play | — |
| ← / Esc | Back / quit | Leave (result if no notes remain) |
| Tab | Settings | — |
| F5 | Key config | — |
| F12 | Skin menu (settings on the left, a live preview on the right) | — |
| H | Every key as it is set | — |
| → / ← (play) | — | Lane cover down (covers more) / up (covers less) |
| `\` | — | Lane cover on / off (START twice does the same) |
| [ ] | — | Lift |
| Shift that plays no lane | — | Held: fine step for the covers and the held green number |
| Mouse wheel, START + turntable | — | Lane cover |
| START + keys / SELECT + keys | — | Hi-speed / green number by 1 ms of travel |
| O / T / R / `/` / F3 | Folders / tables / records / search / sort | — |

Default lanes follow the reference keyboard layout: 7K `Z S X D C F V` + `LShift` (scratch), 9K `Z S X D C F V G B`, 14K adds `M K , L . ; /` + `RShift`. START is `A` and SELECT is `W` on the keyboard (the standard Start / Select buttons on a controller).

**Key config** opens with `F5` on the song select, with `F5` or the `KEY CONFIG >` button in Settings, and with `6` on a skin's browser. It lists lanes, in-play controls, song-select shortcuts, the keys that open each screen, the number keys, result and replay-analysis keys, each with a keyboard cell and a controller cell. A key already read on the same screen is refused and the row says by what; `Esc`, `Enter`, the arrows and `H` stay fixed. Controls that ship without a key and can be given one: lift and HIDDEN+ on / off, HIDDEN+ up / down, green number up / down, KEEP GREEN NUMBER on / off, judge offset ±1 ms, AUTO ADJUST on / off, and the fine-step modifier. Bindings are saved to `~/.config/rbms/keyconfig.ron`; an older file loads as it is.

**KEEP GREEN NUMBER** (Settings → PLAY, off by default) holds one green number across charts: the hi-speed is recalculated from the chart's tempo and the lane cover when a chart starts and whenever the cover moves. **AUTO ADJUST** (Settings → JUDGE) moves JUDGE OFFSET a little after each input that is more than 15 ms early or late; the play screen's TIMING figure shows it. Definitions, units and ranges are in [docs/play-options.md](docs/play-options.md) (Korean). Replays record every offset the run moved through (`judge.offset_steps`), so an adjusted run plays back as it was judged.

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
cargo test --workspace --no-fail-fast
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo run -p rbms-render --example render_select   # headless render to PPM
cargo run -p rbms-cli -- skin-dump assets/skins/rbms-default --strict --frames 120   # load and check a skin pack
```

Rebuilding the default skin's images or sounds needs [uv](https://docs.astral.sh/uv/): `uv run assets/skins/rbms-default/tools/generate-images.py` (add `--check` to compare with the committed files) and `uv run assets/skins/rbms-default/tools/generate-sounds.py`. Builds and CI do not need uv.

Workspace crates, bottom up: `rbms-model` (chart types) → `rbms-parser` (BMS lexer, `#RANDOM` / `#SWITCH`, MD5 + SHA-256) → `rbms-chart` (timing, lanes, shuffle, scroll) → `rbms-judge` (windows, algorithms, gauges, RON tables) → `rbms-audio` (cpal + symphonia, real-time mixer, interpolated clock) / `rbms-render` (`Renderer` trait, CPU reference canvas, skin renderer, fonts) / `rbms-skin` (skin model, Lua 5.2 runtime and loader) / `rbms-video` (H.264 movies for skins; OpenH264 built from source, `video` feature) / `rbms-ir` / `rbms-table` / `rbms-store` (scores, replays) / `rbms-library` (scan) / `rbms-config` (settings schema, descriptors) / `rbms-course` (course model and run state) → `rbms-play` (PlaySession) → `apps/rbms-player` (winit + wgpu, stage-based app); `apps/rbms-cli` is the headless inspector. CI runs `cargo test --workspace --no-fail-fast` on Linux, macOS and Windows with fmt and clippy `-D warnings` gates; after a push, check the three results with `gh run list --branch dev`.

Architecture, the reference parity ledger, decisions and the improvement plan live in [docs/](docs/README.md) (Korean). Start with [docs/PROCESS.md](docs/PROCESS.md). Phase R (creating `prod`, first release/tag and signing or deployment credentials) is deliberately deferred until the maintainer supplies the required real keys.

## License

[GPL-3.0-or-later](LICENSE). The play logic is a clean-room Rust port of a GPL-3.0 reference implementation. Third-party licenses and notices for bundled software (OpenH264, Inter and the Rust dependencies) are in [THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md).
