<div align="center">

# R-BMS

**A BMS rhythm-game player in Rust — reference-accurate judgment, sample-clocked audio, wgpu rendering.**

[Features](#features) · [Build](#build-and-run) · [Skins](#skins) · [Controls](#controls) · [Development](#development)

</div>

Last updated 2026-10-11 · matches 03e1dd8 plus the wave 10A working tree (not yet committed).

R-BMS reimplements the core PLAY loop of a GPL-3.0 reference implementation in Rust. Judgment windows, gauges and TOTAL scaling are cross-checked against the reference implementation's source, the play clock is the number of audio samples actually played, and every mode, skin, key map and setting is data, not code. It is a single native binary with no JVM and no runtime dependencies.

## Features

- **Play** — 5K / 7K / 9K (pop'n) / 10K / 14K auto-detected from the chart, normal and long notes with LN / CN / HCN, mines, BGA images, key beams and hit bombs
- **Judgment** — reference implementation `JudgeProperty` windows per mode, `#RANK` / `#DEFEXRANK`, judge offset with AUTO ADJUST (the reference's judge timing: the notes are drawn that many ms ahead of the song while the sound and the judgement stay on the beat, and with AUTO ADJUST the offset follows your timing as each input lands), judge width, FAST / SLOW, empty POOR handled like the reference implementation
- **Gauges** — ASSISTED EASY / EASY / NORMAL / HARD / EX-HARD / HAZARD with TOTAL scaling, clear lamps, DJ LEVEL and EX score
- **Options** — MIRROR / RANDOM / S-RANDOM / R-RANDOM / H-RANDOM / ALL-SCRATCH / ROTATE (seeded, DP sides independent), hi-speed with constant-speed mode and green number, KEEP GREEN NUMBER (the hi-speed is worked out for every chart from one green number, its tempo and the lane cover), lift, lane cover, HIDDEN+, scratch side and auto-scratch
- **Song select** — recursive library scan, folders and difficulty tables (`data.json`), search and sort, `#PREVIEW` or autoplay preview, cover art, note density graph, local records with replay playback
- **Result** — judge counts, FAST / SLOW, max combo, gauge, clear lamp, rank graph and delta against your best
- **Replays** — saved per play, replayable with the same seed and options, analysis mode with seek, speed and per-note timing
- **Skins and themes** — a built-in default skin for every screen (select, decide, play 5/7/9/10/14 keys, result, course result) from the first run, drawn in black, greys and white with colour kept for what means something by it (notes, bombs, judgements, lamps, gauges) and options for the theme, the accent, the note colours and the bomb colour, with Normal and Wide (1.5x) lanes, a judge line that is the bottom edge of the note field, a judgement that stays still with the score difference and FAST/SLOW over it, and a per-part opacity on the play screens; beatoraja-format Lua skins (`.luaskin`) and JSON skins load unmodified from a skin pack folder, with per-screen selection and customisation on the SKIN menu, which previews the skin as you change it and exports the adjusted pack as a folder; mp4 (H.264) video sources; any TTF font with full Unicode fallback; built-in screens remain as the fallback, and the player's own menus and overlays are drawn in the same black, greys and white, with a RON theme file (`theme.ron`) to recolour them
- **Input** — lanes, in-play controls, song-select shortcuts, the keys that open each screen, result and replay-analysis keys all rebindable in-app on keyboard and controller, with conflict detection per screen
- **IR** — sign in, submit scores, browse rankings, sync settings, manage rivals and upload or replay ghosts, all from the NETWORK group of the in-game Settings and the song-select ranking panel
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

Settings, key config, difficulty tables (the list in `settings.ron`, and the saved copy of each fetched table in `tables/`), local scores and replays live in `~/.config/rbms/` (`%USERPROFILE%\.config\rbms\` on Windows) and are created on first run.

Every start installs the default skin pack under `~/.config/rbms/skin/rbms-default/` (the equivalent settings folder on Windows) from a copy built into the binary; see [Skins](#skins).

Options: `--interactive` `--auto` `--hispeed F` `--gauge NAME` `--lift F` `--sc-left` `--sc-auto` `--keys Z,S,X,...` `--skin file.ron` (a built-in play-field layout; the play screens are then drawn by the built-in field instead of a skin) `--font file.ttf` `--table URL` `--keyconfig path.ron` `--replay file.ron` `--server URL` `--player ID`.

## Skins

The default skin `rbms-default` draws every screen from the first run, with no setup. Its files are copied to `~/.config/rbms/skin/rbms-default/` on each start: a file you changed is left alone, an untouched file is replaced when an update ships a new version, and a file you deleted comes back. If that folder cannot be written the player warns once and draws its built-in screens.

To use another skin pack (a folder of beatoraja-format `.luaskin` files, for example one written for beatoraja; JSON skins work too):

1. Press `F12` on the song select to open the SKIN menu (the SKIN button of a skin opens it too, and so does `F12` or the `SKIN MENU` button in Settings). The settings are listed on the left; the right half draws the skin of the screen being configured with sample data, live.
2. On `PACK FOLDER` press `Enter` and choose the pack's folder. Pressing the left arrow clears it and returns to the default skin. For one run only, set `RBMS_SKIN_PACK=/path/to/pack` instead.
3. `Tab` (or a click on the strip above the preview) changes the screen. A screen the pack has no document for is drawn by the player's built-in screen, not by the default skin, and the preview says so. The same menu lets you pick a different document per screen and change the options, files and offsets a document offers; the preview follows each change. An offset a skin lets you move along its alpha alone is listed as an opacity in percent (a slider from 0% to 100%, 1% a step, 10% with `Shift`, `0` for 100%; see [Number rows](#number-rows)): on the default play skin, taking Lane Background, Lane Divider and the readouts you do not want to 0% leaves the notes, the key beams, the bombs and the judge line on a black field. `Esc` saves and goes back.
4. A row other screens of the pack declare under the same name says so under the preview (`ALSO ON 8 OTHER SCREENS - A APPLIES THIS VALUE TO THEM ALL`); `A` puts the value it stands on onto all of them. The play menu has the same key for the rows of the play skin.
5. `EXPORT`, the last row of the list, writes the pack as you adjusted it into a new folder `<pack>-export` inside the folder you choose: every file of the pack and one text file, `rbms-adjustments.ron`, that holds your options, files, offsets and opacities by name. Choosing that folder as `PACK FOLDER` on another machine, or in a fresh setup, draws every screen the way you left it; what you then change in the menu is saved in your own settings and wins over the file. The pack you exported from is only read. A third-party skin is copied as it is, so passing its export on is redistributing that skin, on its author's terms.

The default skin is monochrome: a black background, grey panels and lines, white and grey text and a white accent. Four options change that, each listed on the SKIN menu and, for the play skins, in the LANE group of the play menu:

| Option | Items (default first) | On |
| --- | --- | --- |
| Theme | Mono / Slate (the dark blue-grey of earlier versions) | every screen |
| Accent | White / Sky / Amber / Green / Pink | every screen |
| Note Colors | White-Blue / White-Deep Blue / White-Yellow / White-Green / Mono | play 5, 7, 10 and 14 keys |
| Bomb Color | Note / White | every play screen |

An option is stored for each screen; press `A` on its row to set it for all of them at once. Theme Slate with Accent Sky is the look of earlier versions. The menus, the key guide and the notices the player draws over a skin keep the player's own theme and do not follow these options: that theme is black, greys and white as well, and is recoloured in `theme.ron` ([docs/theme.md](docs/theme.md)).

Skins can read their own folder but cannot write into it, open files outside it, or reach the network. If a skin fails to load, the player says which file failed and draws the built-in screen. [docs/skin.md](docs/skin.md) (Korean) has the details: pack rules, the default skin, the Lua environment, the `rbms-cli skin-dump` checker and the known limits. Third-party notices for the bundled video decoder and font are in [THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md).

## Controls

These are the shipped keys. Press `H` on the song select for the full list as it is set right now.

| Key | Song select | Play |
| --- | --- | --- |
| ↑ ↓ | Move | Hi-speed (the held green number while KEEP GREEN NUMBER is on) |
| → / Enter | Open / play | — |
| ← / Esc | Back / quit | Leave (result if no notes remain) |
| Tab | Settings (see [Settings](#settings)) | — |
| F5 | Key config | — |
| F6 | Fetch every difficulty table again (see [Difficulty tables](#difficulty-tables)) | — |
| F8 | Play menu (play options on the left, a live play screen on the right) | — |
| F12 | Skin menu (settings on the left, a live preview on the right) | — |
| H | Every key as it is set | — |
| → / ← (play) | — | Lane cover down (covers more) / up (covers less) |
| `\` | — | Lane cover on / off (START twice does the same) |
| [ ] | — | Lift |
| Shift that plays no lane | — | Held: fine step for the covers and the held green number |
| Mouse wheel | — | Lane cover, 0.5% a notch |
| START + turntable | — | Lane cover by one white number (0.1%) a press, repeating while held; the green number stays and the hi-speed follows |
| START + SELECT + turntable | — | Lift by one a press, repeating while held |
| START + keys / SELECT + keys | — | Hi-speed / green number by 1 ms of travel |
| 9 / 0 | — | Judge offset -1 / +1 ms, repeating while held (the TIMING figure and the notes move at once) |
| O / T / R / `/` / F3 | Folders / tables / records / search / sort | — |

Default lanes follow the reference keyboard layout: 7K `Z S X D C F V` + `LShift` (scratch), 9K `Z S X D C F V G B`, 14K adds `M K , L . ; /` + `RShift`. The turntable has a second key for its other direction, `LCtrl` (`RCtrl` for the right-hand turntable of 10K and 14K), as in the reference. START is `A` and SELECT is `W` on the keyboard (the standard Start / Select buttons on a controller). With START held, `LShift` brings the lane cover down by one and `LCtrl` takes it up by one; with SELECT held as well the two move the lift.

**Key config** opens with `F5` on the song select, with `F5` or the `KEY CONFIG` button in Settings, and with `6` on a skin's browser. It lists lanes, in-play controls, song-select shortcuts, the keys that open each screen, the number keys, result and replay-analysis keys, each with a keyboard cell and a controller cell. A key already read on the same screen is refused and the row says by what; `Esc`, `Enter`, the arrows and `H` stay fixed. Controls that ship without a key and can be given one: lift and HIDDEN+ on / off, HIDDEN+ up / down, green number up / down, KEEP GREEN NUMBER on / off, AUTO ADJUST on / off, and the fine-step modifier. Bindings are saved to `~/.config/rbms/keyconfig.ron`; an older file loads as it is, and reads the keys that shipped later (`9` / `0` for the judge offset, `LCtrl` / `RCtrl` for the turntable's other direction) wherever it had not already given those keys to something else.

**Play menu** opens with `F8` on the song select (the key can be moved in the key config) and with the same key or the `PLAY MENU` button in Settings. The options a player changes from one sitting to the next are listed on the left in groups (SPEED, LANE, JUDGE, CHART, with the options and offsets the play skin declares at the end of LANE, such as Lane Width, Judge Display and Judge Position, and a SKIN group whose OPACITY and POSITION AND SIZE rows unfold the skin's per-part opacities and the offsets every play skin has); the right half draws a play screen of the chosen mode with its skin on a sample chart that never stops, and every change shows from the next frame. `Tab` changes the mode (7K 5K 14K 10K 9K) and the PREVIEW BPM row the tempo of the sample chart; neither is stored. Under the preview, five figures say what the options come to at that tempo: BPM, HI-SPEED, TRAVEL (ms), GREEN and WHITE. A row that holds a number has a slider and can be typed into (see [Number rows](#number-rows)); `0` puts the row back to its default, a right click steps back, and `Esc` saves and goes back. On the default skin the judgement is drawn still: the word and the combo stay in place for half a second with the difference to the target score (`+0003`) and FAST / SLOW on the line above. JUDGE POSITION Y moves all of it up (+) or down (-) in pixels, JUDGE DISPLAY hides it, and SCORE DIFFERENCE and JUDGE DETAIL choose what the line above says. Settings keeps the rules (judge algorithm and windows, gauge set, TOTAL) and everything that is not a play option; the option overlay (`F1`) and the option panels of the song select are unchanged.

**KEEP GREEN NUMBER** (play menu → SPEED, off by default) holds one green number across charts: the hi-speed is recalculated from the chart's tempo and the lane cover when a chart starts and whenever the cover moves. With it on, stepping PREVIEW BPM in the play menu leaves the note travel time and the green number where they are and moves the hi-speed figure; with it off the hi-speed stays and the note speed changes. **JUDGE OFFSET** (play menu → JUDGE) draws the notes that many ms earlier (+) or later (-) than the song and moves nothing else: an input on the beat is on time at every value and its keysound stays with the song. Raise it if you hit late, lower it if you hit early. **AUTO ADJUST** moves it a little after each input that is more than 15 ms early or late (down when you are early, up when you are late); the play screen's TIMING figure shows it. A value saved by an earlier version is read with its sign turned once, which keeps the notes and the judgement as far apart as they were. Definitions, units and ranges are in [docs/play-options.md](docs/play-options.md) (Korean). A replay is judged where its inputs landed, whatever offset its watcher holds; replays saved by earlier versions play back as they were judged.

**TARGET** (play menu → CHART) is what the score graph, the figure over the judgement and the result screen pace a run against. LOCAL BEST, IR BEST and RIVAL name a score that may not be there -- a chart never played by hand, no score server, a rival who has not played it -- and then the run is paced against **TARGET FALLBACK** (RATE AAA unless changed; IR BEST tries your local best first). RATE CUSTOM asks for the share of the chart's best possible score set in **TARGET RATE**. When a run fell back, the default skin's score graph says so (`RATE AAA (NO RECORD)`). Every run played by hand is stored locally whether or not you are signed in. A run with SCRATCH AUTO on (on a chart with turntable notes) or a judge widened past 100% is an assist: it keeps its history row, lamp and play count but sets no best score and is not sent, so it is no LOCAL BEST either.

### Settings

**Settings** opens with `Tab` on the song select. The groups (JUDGE, DISPLAY, INPUT, NETWORK, AUDIO, SELECT) are on a rail at the left with the number of rows in each, the rows of the open group are in the middle under the headings of its sections, and a panel on the right says what the row under the cursor is for, what it holds and ships as, its range and step, and the keys it hears. The same keys are written along the foot of the screen, followed by the keys of the screen itself.

A row is drawn by what it holds: a switch for on and off, every value side by side for a choice of up to four, the value between two arrows for a longer choice, a slider and a number (see [Number rows](#number-rows)), a field for text, the end of a path for the font and the system sound folder, and `>` for something that is run or opened. A row that is not at its default has a dot at its left edge, and so has its group on the rail.

| Input | What it does |
| --- | --- |
| `↑` `↓`, `PgUp` `PgDn`, `Home` `End` | Move the cursor: a row, a section, the first or the last row |
| `←` `→` | Turn a switch over, step a choice, move a number one step (`Shift` ten) |
| `Enter` | Type a number; edit a line of text; run an action or open a list in NETWORK; open the key config; choose a font. On any other row it saves and closes, like `Esc` |
| `0` | The row under the cursor back to its default. Text (the server address, the player id, the e-mail, the password) is never reset |
| `Shift` + `0`, or the `RESET GROUP` button | Ask, then put every row of the open group back to its default |
| `Tab` / `Shift` + `Tab` | Next / previous group |
| Click | On a row's label: move the cursor only. On a switch, a value, an arrow, a field or `>`: set, step, edit or run it. On a group of the rail: open it |
| Right click | One step back on a row that holds a value |
| Wheel | Scroll the rows, when a group is longer than the screen |
| `F8` / `F12` / `F5`, or the buttons at the foot of the rail | Play menu / skin menu / key config |
| `Esc` | Save and go back |

A JUDGE WIDTH or LN MARGIN over 100% makes a run a custom judge: it sets no best score and is not sent to the score server, while its lamp (ASSIST EASY at best), play count and history are kept. The panel says so while such a row is under the cursor.

### Number rows

Every row that holds a number in the play menu, the skin menu, Settings and the practice panel is set the same three ways.

| Input | What it does |
| --- | --- |
| `←` `→` | One step of the smallest change the row holds: 0.01 for HI-SPEED, 0.1% for the lane cover, lift and HIDDEN+ (one white number), 1 ms for JUDGE OFFSET and PREVIEW FADE, 1% for volumes, judge widths and opacities, 1 for an offset, TOTAL, GREEN NUMBER and POLYPHONY. `Shift` makes it ten steps. |
| Drag the slider | A press on the track puts the row there, and it follows the pointer until the button comes back up, even off the row. A press on the knob takes hold of it without moving the row. The preview follows from the next frame and the settings file is written once, on release. The wheel never moves a slider; it scrolls the list. |
| `Enter`, or a click on the number | Type the number in place. The current value is selected, so the first digit replaces it. `Enter` sets it (held to the row's range and rounded to its step), `Esc` cancels the typing only, and `↑` `↓` `Tab` set it and move on. A unit typed after the number (`%`, `ms`, `x`, `px`) is ignored; anything that is not a number is refused and the row says `NOT A NUMBER`. |
| `0` | Back to the default. A right click steps once back. |

A position or size offset runs over ±2000, more than its slider has places for, so use the arrows or type the number to set one exactly. An opacity's slider and arrows stop at 100%; typing goes up to 200%, which adds alpha and only matters to a skin that lights a part it draws fully transparent. A row that is worked out from the others (HI-SPEED while KEEP GREEN NUMBER is on) is drawn faint and cannot be set. The option panels of the song select and the in-play keys keep their own key steps.

The **practice panel** (`F4` on the song select) sets its six numbers the same way: START TIME and END TIME in steps of 0.1 s, GAUGE VALUE, JUDGERANK, TOTAL and FREQUENCY in steps of 1. A time is shown as `1:05.5` and typed either as seconds (`65.5`) or as minutes and seconds (`1:05.5`). The first row, START, starts the slice with `Enter` or a click, and the cursor is on it when the panel opens; `Enter` starts the slice from the three choice rows as well, and on a number row it opens the row for typing instead. `Tab` moves the cursor down and `Shift` + `Tab` up. END TIME never goes below START TIME + 1 s. Raising START TIME past END TIME pushes it along; while START TIME is being dragged the push lasts only as long as it is past END TIME, so a drag that overshoots and comes back leaves END TIME where it was. TOTAL's slider and arrows run from 10 to 1000, and typing goes up to 5000.

## Difficulty tables

Press `T` on the song select to open the table manager, choose `+ ADD TABLE (URL)` and enter the address of a table's page (or `+ ADD TABLE (FILE)` for a `data.json` on disk). Each table that loads becomes a folder on the song select with its levels inside.

Adding a table does not leave the table manager: the new row appears at once, reads `FETCHING...` (`READING...` for a file) while the table is fetched in the background, and you can go on using the manager or go back to the song select. When the table arrives its folder appears in the song list and a notification says `table added: NAME (N local charts)`. If it cannot be fetched the row stays, says `LOAD FAILED` with the reason, and can be fetched again with `R` or removed with `D`.

A table at an address is downloaded once. The player keeps a copy in `~/.config/rbms/tables/` and every later start, and every library rescan, reads that copy without touching the network. Starting the player never waits for a table's server: MATCHING TABLES reads only the saved copies and the table files on disk. A table that has no copy yet (one that has never been fetched successfully) is fetched once the song select is up, in the background; its folder appears where it belongs when it arrives, with the cursor left on the folder it was on, and a notification says `difficulty tables fetched for the first time: N added, M failed`. An address that does not answer costs no waiting: about 15 seconds after the song select appears a notification says `table fetch failed (ADDRESS): REASON`, the row in the table manager says `LOAD FAILED`, and the address is not asked again until the next start, `R`, `A` or `F6`. The copy is not refreshed by itself:

| Key | Where | What it does |
| --- | --- | --- |
| `R` | Table manager | Fetch the table under the cursor again |
| `A` | Table manager | Fetch every table again |
| `F6` | Song select | Fetch every table again (the key can be moved in the key config) |

A reload runs in the background: the screen stays usable, the row reads `RELOADING...`, and when the table arrives its folder changes in place and a notification says `difficulty tables reloaded: N updated, M failed`. If a reload fails, or the address now answers with an empty table, the saved copy and the folder stay as they were and the row says `RELOAD FAILED` with the reason. Each row shows how long ago its copy was fetched (`SAVED 3 DAYS AGO`); after 30 days the age is drawn in the warning colour. Removing a table (`D`) deletes its copy, also while it is being fetched. A table registered as a file has no copy and is read from the file each time. A table that has never been fetched successfully has no copy, so the player tries it once on every start, after the song select is up, until it loads or is removed. A table given with `--table` is treated the same way: read from its copy at startup, or fetched after the song select is up when it has none.

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
