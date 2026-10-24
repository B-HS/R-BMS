# Development guide

Last updated 2026-10-24 · matches 66b9e63 (adds `RBMS_LANGUAGE`, the language catalogs and the website's messages)

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

Commands added with the 2026-10-23 work (all offline; none of them uploads anything):

```bash
cargo test -p rbms-download   # link classes, download limits, archive limits and folder names; a fake server, no network
cargo run -p rbms-cli -- course-snapshot --zip <2018_course.zip> --normal-header <header_normal.json> --insane-header <header_insane.json> \
    --titles <md5-title.json> --retrieved-on <YYYY-MM-DD> --out web/data/courses/genoside-2018/courses.json [--check]
cargo run -p rbms-cli -- mirror-pack <library folder> --out <folder outside the repository> (--table <data.json>)... [--all] [--dry-run]
```

- `rbms-cli course-snapshot` rebuilds `web/data/courses/genoside-2018/courses.json`, the data of the 44 GENOSIDE 2018 courses the player carries (`rbms_course::genoside_2018`) and the website registers, from the maker's course archive and two difficulty-table headers the maintainer downloaded. It fetches nothing. With `--check` it writes nothing and exits 0 when the file is what the inputs give, 1 when it is not, 2 when an input could not be read.
- `rbms-cli mirror-pack` packs the song folders a table lists (`--table`, repeatable) or every song folder (`--all`; one of the two is required) into deterministic zip archives `packs/<sha256>.zip` plus `manifest.json` in `--out`, which must not be inside the library. It uploads nothing and reads no network; `--dry-run` writes nothing. The limits are the ones the player's unpacker applies (`rbms_download::ExtractLimits`), so a pack it writes is one the player can unpack. Keep `--out` outside this repository: the songs are not ours to commit.

- The last integrated local gate (2026-10-23, wave 7, over the commits up to 5a0eacf; not a CI run; the figures are the ones `PROCESS.md` records) passed 6,611 Rust tests with 0 failures, and the web side's last full count was 1,808. CI for 5a0eacf (run 39941541295) passed, and so did `ci` (run 39944917898) and `web` (run 39944917886) for 76e026a. The gate before it (2026-10-22, over the commits up to 47a7f9c) passed 6,269 Rust tests with 0 failures, and on the web side 1,491 tests plus 122 database tests (`RBMS_DB_E2E=1` against a MySQL 8.4 container). The last count taken from CI is the run of ea698a9 (run 38698550145): 6,024 Rust tests passed with 0 failures and 6 ignored (they need an audio or graphics device, wait five seconds, or read a copy of a player's own database; `ci-release.md` §1); the S19 closing gate (51f3a1e) passed 653 web tests besides the database integration tests. The wave 20 gate (2026-10-14) had 5,671 and 415. The numbers are not a contract. CI gates formatting and the clippy command above, and builds/tests on Linux, macOS, and Windows with `cargo test --workspace --no-fail-fast`, so one run lists every failure of a platform.
- **Frame cost.** The play screen's budget ([`acknowledge/2026-10-14-adr-play-hitch-criteria.md`](acknowledge/2026-10-14-adr-play-hitch-criteria.md)) is an update and a draw under 1 ms at the 99th percentile and no allocation of 256 KB or more in a frame of play, on the default skin at 1920x1080 (S19, `docs/quality-assurance/2026-10-14-test-debt.md` 3.38). It is measured headlessly with a release build: a test module kept outside the repository enters a chart through `AppShared::load` and `enter_loaded_chart`, steps the scene and the song 16 ms a frame with the audio engine dropped, times `Stage::update` and the draw onto an offscreen `Gpu` (skin evaluation, quads and texture uploads, no rasterising), and counts each frame's allocations with a counting global allocator armed on the frame thread. Charts of 2, 11 and 4 minutes (2,011, 17,107 with 32 changing background pictures, and 51,379 notes) are the reference cases. Call `device.poll` after an empty submit each frame, as the window's own `render` submits: an offscreen target that never submits piles its writes up and inflates the numbers. The results before and after S19 are in `reference/frame-cost.md`.
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

- The gate for a change under `web/` is `.github/workflows/web.yml` (install frozen, `verify`, `format:check`, a build, and a second job that runs `web/tests/db` against a throwaway MySQL 8.4 service with `RBMS_DB_E2E=1`). It runs when `web/` or `contract/` changes, not for a Rust-only change, and `ci.yml` does not run for a web-only one, except that `ci.yml` also runs when `web/data/tables/**` changes because the player embeds those files.
- Production is Vercel (project `rbms-web`, root `web`, production branch `dev`); a push to `dev` that changes `web/` deploys. The production database is touched only through the manual `web-db.yml` workflow (nineteen actions, what each runs is in `ci-release.md` §6.1: `status`, `migrate`, the test accounts (`cleanup-count`/`cleanup-delete`), the chart note-count check `chart-notes-count` with its `repair`, `restore` and `drop-backup` steps, `admin-seed-count`, `assist-grade-backfill-count`/`assist-grade-backfill`, the rows of a chart removed at its owner's request (`chart-purge-count`/`chart-purge-delete`), the GENOSIDE 2018 course registration (`courses-seed-count`/`courses-seed`), the copy of the Stella and Satellite tables (`tables-refresh-count`/`tables-refresh-force`) and the dan recount (`dan-recompute-count`/`dan-recompute`); every action that writes is preceded by the count it acts on; the decision is `acknowledge/2026-10-14-adr-production-db-operations.md`), using the repository secret `WEB_DATABASE_URL`, with one exception: the Vercel Cron job `GET /api/cron/tables` (`web/vercel.json`, every day at 15:00 UTC, admitted only with `Authorization: Bearer <CRON_SECRET>`) stores the daily copy of the two remote tables, registers their courses and gives the dans their new courses allow. Never read or write `.env*` files from an agent.
- `web/scripts/seed-courses.ts <count|apply>`, `refresh-tables.ts <count|apply-large>` and `dan-recompute.ts <count|apply>` are what the `web-db` actions run. They need a database URL in the environment and print numbers only (the Actions log is public); point them at a local MySQL, never at the production database from a laptop. `count` writes nothing.
- `web/data/tables/insane1/{header,data,source}.json` is the snapshot of the 発狂BMS難易度表 the site serves and the player carries (`include_bytes!`); `bun scripts/snapshot-table.ts <table-url> <output-dir>` (run in `web/`) writes `header.json`, `data.json` and `source.json` into the folder you name (here `data/tables/insane1`); with `--check` it writes nothing and only compares (exit 0 same, 1 different, 2 failed).
- The website's words (2026-10-24): next-intl 4.14.9 with a cookie and no path prefix (`web/src/shared/i18n/`). Messages are `web/messages/{ko,en,ja}/{common,ai,settings,browse,pages,server}.json`: Korean is the source and a missing key in `en` or `ja` fails `tsc`; `tests/lib/i18n-messages.test.ts` checks that the three languages have the same keys and `{name}` placeholders and no empty string. Read them with `useTranslations('<namespace>.<group>')` (a client component) or `await getTranslations(...)` (a server component); never translate inside a `'use cache'` function. The root layout reads the cookie, so every page renders per request (`instant = false`, [the ADR](acknowledge/2026-10-24-adr-web-i18n.md)).
- The wire contract with the player is tested across the two sides: the player's tests write the bodies it really sends into `contract/client-requests/*.json` and fail when a body differs (`apps/rbms-player/src/app_result/record_tests/contract_requests.rs`), and `web/tests/dto/client-requests.test.ts` passes every file through the server's schemas. When the client changes what it sends, run `RBMS_UPDATE_CONTRACT=1 cargo test -p rbms-player --lib contract_requests`, commit the rewritten files, and run the web gate on them; each side also keeps its own tests (`crates/rbms-ir`, `web/tests/dto/ir-contract.test.ts`). The direction from the server's answers to the client's parser is not cross-tested yet (`quality-assurance/2026-10-14-test-debt.md` 3.12).

## Environment variables

| Variable | Read by | Effect |
|---|---|---|
| `RBMS_SONGS` | the player | song folder when no argument and no remembered `songs_folder` |
| `RBMS_LANGUAGE` | the player | the interface language for this run, `en`, `ko` or `ja` (a region or encoding after the language is ignored). It comes after the LANGUAGE setting and before the system locale, so it matters only while the setting is AUTO, and it is read once, the first time AUTO is settled. A value that names none of the three languages is passed over and the system locale decides. Unit tests run in English whatever the host says (`apps/rbms-player/src/app_language.rs`) |
| `RBMS_SKIN_PACK` | the player; the opt-in skin tests | skin pack folder for this run only, above `skin.pack` and the installed default pack; blank is ignored. Use an absolute path in tests |
| `RBMS_SKIN_SEED` | `rbms-skin` | pins wildcard file selection and a skin's `math.random` (a whole number) |
| `RBMS_SKIN_CAPTURE_DIR` | the capture harness (tests) | writes each captured frame as `<name>.png` (GPU frames as `<name>.gpu.png`) |
| `RBMS_REQUIRE_GPU` | the GPU pixel tests | makes a test that needs a graphics adapter fail instead of skipping when none comes up |
| `RBMS_GOLDEN_UPDATE` | `rbms-render` golden tests | regenerates the built-in screen golden images |
| `RBMS_TIMING_CSV`, `RBMS_SOAK_LOG` | the player | timing sample dump and periodic stability log (same as `--timing-csv`) |
| `RBMS_SCOREDB_COPY` | one ignored player test | path of a copy of a player's own `scoredb.sqlite`; the test reads it with this build and reports how its rows are read (`cargo test -p rbms-player --lib -- --ignored a_players_own_database`). Never point it at the live file |
| `RBMS_SKIN_EXPORT_DIR` | one skin-menu test | with `RBMS_SKIN_PACK`, a folder outside the repository the pack is exported into |
| `RBMS_UPDATE_CONTRACT` | the contract tests of the player (`contract_requests.rs`) | `1` writes the request bodies into `contract/client-requests/` instead of comparing them |
| `RBMS_WINDOWS_RESOURCE` | `apps/rbms-player/build.rs` | a compiled Windows resource (`.res`, the app icon) to link into `rbms-player.exe`; set by `scripts/windows-icon-resource.ps1` in CI and the release, unset everywhere else |
| `RBMS_IR_E2E_URL` | `crates/rbms-ir/tests/e2e_local.rs` | base URL of a local score server for the end-to-end test; the test is skipped when it is unset |
| `CRON_SECRET` | `web/` (`GET /api/cron/tables`), set in Vercel only | at least 16 characters; the cron route admits only a request whose `Authorization` header is `Bearer <CRON_SECRET>`. Unset, the route answers `SERVICE_NOT_CONFIGURED` (503); a wrong value is 401. Never print or commit it |
| `HOME`, `USERPROFILE` | the player | the config dir is `$HOME/.config/rbms`, else `%USERPROFILE%\.config\rbms`, else `./.config/rbms` |

## Persistence (`~/.config/rbms/`)

| File | What |
|---|---|
| `settings.ron` | versioned `Config` (schema 6): play (with `scratch_auto_color`), judge (with `course_gauge`, the table a course is played on), library folders and difficulty tables (each table row has a `group`, the root row several tables are shown under; `seeded_default_tables`, the default tables already given; `download_folder` and `show_missing_charts`), display, audio, network (server address, the signed-in login id and bearer token, extra IR profiles), skin. The fields added in 2026-10 are all `#[serde(default)]`, so the schema version stays 6 and an older build ignores them |
| `keyconfig.ron` | lane, control and shortcut bindings for keyboard and pad. `controls` holds the 20 in-play controls (`start` / `select` default to A / W, `cover_toggle` to BACKSLASH; the ones added in wave 10A, `fine` among them, default to empty = unbound). `shortcuts` maps a shortcut's name to a key word (no entry = the shipped key, an empty string = unbound). An older file without these loads with the defaults and is not rewritten on load. The editor opens with F5 from the song select or Settings; what each control does is in [play-options.md](./play-options.md) |
| `songdb.sqlite` | scanned song-library index and chart-detail cache |
| `scoredb.sqlite` | one merged best per chart and full local play history (`scorelog` has a row for every run played by hand, a run left with Esc included, with the judging-rule generation and the assist level; the best row is built only from runs that may set one). `play_submission` keeps the submission body each run was finished with and the account it was played under; `ir_sync` keeps, per run and score server, what the server answered (accepted with its `score_id`, rejected with the error code, field and reason and the builds it was refused under, `refused_under`, retry, skipped) and where the replay stands; `course_submission` and `course_sync` do the same for a finished course. The schema generation stays 1: the tables are added by `CREATE TABLE IF NOT EXISTS` and the later columns by `ADD COLUMN` (`ADDED_COLUMNS`), so a database opened by 0.1.4 still opens in 0.1.3 |
| `pending-runs/` | a run finished while `scoredb.sqlite` could not be opened or written, one `<played ms>-<chart>.ron` each (`PendingRun`: the history row, the submission body and the account); read into the database the next time it opens (`ScoreDb::import_pending_runs`) and removed once read in |
| `scores.ron.migrated` | a legacy `scores.ron` after its first successful import into `scoredb.sqlite` |
| `tables/` | the copy of each fetched difficulty table, `<hash of the address>.json`; the insane table's copy is written the first time it is given (the player carries it), the two Stella tables have none until their first fetch; a copy a day old, or one that says no date, is fetched again once per run |
| `<download folder>/` | not under the config folder: the folder the player chose in SETTINGS > SELECT for charts it downloads, scanned with the song folders. A download works in `.rbms-partial/` inside it, removes that on the way out, and no scan of the library walks into it; each unpacked chart ends in a folder of its own named from its title and the first eight characters of its md5 |
| `user-folders/` | the copy of the signed-in account's folders (`MY FOLDERS`), `<hash of the server and the login id>.ron`; deleted on sign-out |
| `favorites.ron` | starred chart MD5 values |
| `practice.ron` | per-chart practice properties |
| `theme.ron` | UI-theme file; a template that sets no colour is written on first run, and an untouched template of an earlier version is replaced (see `theme.md`) |
| `replays/` | saved replays (`<md5[:8]>-<ms>.ron`) |
| `courses/` | user course documents |
| `skin/rbms-default/` | the default skin pack, written by every start from the copy embedded in the binary (`install-manifest.txt` records what was installed; a file the player edited is left alone). Other packs may sit beside it |
| `skin-data/<pack id>/` | what a skin's Lua wrote (its write overlay), one folder per pack, created on the first write |

`settings.ron` includes the current schema version. A parse failure moves it to `*.ron.bak`; a schema migration first copies it to `*.ron.v<version>.bak`. The file holds the account's bearer token (and the tokens of extra IR profiles), so on Unix it and every copy of it are written `0600` (`rbms_store::write_atomic_private`), and a load closes the file and the copies an older build left `0644` (`rbms_store::restrict_to_owner`; only regular files, never a folder or a link). On Windows nothing is changed: a file created under `%USERPROFILE%` inherits the profile folder's access list, which admits only the user, the system and the administrators. The legacy `folders.ron` and `tables.ron` files are read only when migrating an old settings document, then folded into `settings.ron`. A legacy `scores.ron` is imported into SQLite only when `scoredb.sqlite` is first created; if the database cannot open, the player falls back to reading that legacy score book.

## App module layout (`apps/rbms-player/src/`)

The crate is a library plus a three-line binary: `main.rs` is `fn main() -> ExitCode { rbms_player::run(std::env::args()) }`,
and `lib.rs` is the module root — the crate's consts and free helpers, `App` / `AppShared`, `App::new`,
the frame loop, `ApplicationHandler` and `run`. Everything else hangs off it:

| File | Holds |
|---|---|
| `lib.rs` | `App` / `AppShared`, consts, free helpers, the frame loop, `ApplicationHandler`, `run() -> ExitCode` |
| `assets.rs`, `assets/default_pack.rs`, `assets/theme_file.rs`, `assets/scripted_movie.rs` | built-in note-field layouts, skin pack identifiers and overlay folders, the `theme.ron` template and how an earlier one is told from a player's own file (`theme_file.rs`), chart-relative file resolution, the keysound decode pool, BGA decode, the library scan; the embedded default pack and its install rules (`build.rs` makes the file table); movie stand-ins for tests |
| `gpu/` | the wgpu instanced-quad renderer (`Gpu`, with `batch.rs` draw-call batching and `background.rs`), fallible so a machine with no adapter gets a message; `present.rs` is the `rbms-present` thread that waits for the display's image and presents it, so the event loop never waits on vsync (`architecture.md`, "Frames are paced by the display"); `registry.rs` hands a released texture's slot to the next texture under a handle of its own |
| `stage/mod.rs` | `Stage`, `StageId`, `Transition`, `FrameCtx`, `KeyInput`, the `StageHandler` trait and its dispatch |
| `stage/{select/,decide,settings,skin_menu,play_menu,keyconfig,tables,folders,loading,play/,result,practice,course_result}.rs` | one screen each, owning that screen's own state. `skin_menu.rs` is the skin settings with a live preview (`F12`); `play_menu.rs` (+ `play_menu/rows.rs`) is the play options with a live play screen (`F8`); `preview_menu.rs` is what the two draw and move the same way. `select/` is split into `mod.rs` (list, record modal, ranking panel and course tab), `keys.rs` (key tables), `panel.rs` and `events.rs` (option panels 1 to 3 and skin events), `skinned.rs` (the skin-drawn browser), `overlay.rs` and `guide.rs` (what the app draws over it), `preview.rs` (hover preview), `list.rs`, `filter.rs` and `scene.rs` (what the renderer is handed), `download_modal.rs` (the modal over a chart a table lists and the library does not hold). `decide.rs` is the scene between picking a chart and its run; `scene_life.rs` its timing rules, shared with result. `play/` holds the state machine (PRELOAD, READY, PLAY, FAILED, FINISHED); `play/key_gate.rs` is KEY DEBOUNCE on the keyboard's lane keys and `play/trace.rs` the play screen's copy of every note's state. `practice.rs` owns practice setup; `course_result.rs` renders the aggregate course result. `capture.rs` (+ `capture/gate.rs`) is the test-only capture harness. |
| `skin_select.rs` / `skin_screen.rs` / `skin_host/` / `scene_clock.rs` | the skin plumbing: `SkinLibrary` (pack, documents, loads, the skin menu's rows, the colour rows and their channels), compiled screens and texture decode (`skin_screen/preview.rs` and `skin_preview.rs` draw one screen's skin with sample state inside the skin menu), `ScreenHost` (the `SkinHost` implementation, one file per cluster of property ids; `skin_host/extension.rs` lists every event and option id this player answers beyond the reference's), the scene clock |
| `command_line.rs` | the launch arguments: the options of one launch, and `--help` / `-h` / `--version` and a refused command line, answered before anything on disk or on the network is touched (exit 0, or 2 for a refusal) |
| `save_worker.rs` | the one thread the frame thread hands its file writes to (settings, replays, a run kept on the side): in order, a settings save a later one replaces skipped, failures reported back, drained at exit (`AppShared::finish_saves`) |
| `record_sync.rs` | the records sync: every kept run and course the server does not hold, sent oldest first on a worker with its own read-only database connection, then the replays; the answers written to `ir_sync` / `course_sync` (`play-options.md` §10.9) |
| `input_debounce.rs` | LR2's `inputinterval` state machine one key or button goes through, shared by `stage/play/key_gate.rs` (KEY DEBOUNCE) and `gamepad.rs` (PAD DEBOUNCE) |
| `window_mode.rs` / `pointer.rs` / `textedit.rs` / `toast.rs` / `notify.rs` | window resolution and mode and the window icon (the embedded `assets/icon/png/icon-256.png`), mouse events as `PointerInput`, the single-line text editor, the notification bus and its on-screen strips |
| `app_input.rs` / `app_network.rs` / `app_options.rs` / `app_play.rs` / `app_ranking.rs` / `app_result.rs` | `AppShared` methods by concern: keyboard and gamepad input, NETWORK tab and sync, option overlay, chart load / shared stream / song clock, IR ranking and result persistence or submission. |
| `library.rs` / `scoredb_store.rs` / `practice.rs` / `course_ui.rs` / `course_ir.rs` | Phase G persistence and modes: SQLite song index, SQLite score import and history, per-chart practice properties and clock, course selection/constraints/results, and course IR submission. |
| `select_tree.rs` | where the song browser is: the folders it has opened (`SelectFolder`, outermost first, each with the bar the cursor was on when it was opened), the identity a bar is found again by (`BarIdentity`), and `ListPlace`, the place kept across a table landing, a scan or a search |
| `download.rs` / `open_url.rs` | a chart download from a table: one job on a worker thread (fetch, unpack, move into the download folder), then a scan of that folder alone and a new match of the tables, taken in by the song list only; and handing a vetted web address to the system browser |
| `target.rs` / `user_folders.rs` / `favorites.rs` / `tablesrc/seed.rs` / `judge_setup.rs` | what a run is paced against (TARGET, TARGET FALLBACK), the account's own folders (`MY FOLDERS`), starred charts, the default difficulty table, and how a run's judge settings are read and whether they count as an assist. |
| `gamepad.rs` / `syssound.rs` / `ir_ext.rs` | Phase G device and network extensions: gilrs pad state (each change resolved at the instant gilrs stamped it, PAD DEBOUNCE) and analog mapping, optional system-sound set, and primary plus additional IR profile fan-out. |
| `format.rs` / `tablesrc.rs` / `settings_ui.rs` / `settings_view.rs` / `ir_*.rs` / `play_sink.rs` / `timing.rs` | display formatting, table loading, the settings rows the program owns, the settings renderer, IR panels/session/ranking/replay, the result screen's IR line (`ir_outcome.rs`) and settings sync (`ir_sync.rs`; the records sync is `record_sync.rs`), the `SoundSink` adapter, and the timing probe. |
| `app_language.rs` / `text/` / `notify.rs` | the language the settings, `RBMS_LANGUAGE` and the system locale resolve to (applied in the frame it changes, before the stage updates), and the `translations!` catalogs of every fixed word the player writes (`text/{settings,select,menus,notices,ranking,layout_play}` and their submodules; the built-in screens' words are in `crates/rbms-render/src/text/`). `notify_text` / `notify_with` queue a catalog entry that the frame thread puts into words, while stderr gets the English at once |
| `borrowed_layout.rs` | borrowing a replay's lane layout for a new run: `LayoutRequest`, `BorrowedLayout`, `LayoutRefusal`, and the `AppShared` methods that start it from a watched replay, a record or a ranking row |
| `keyconfig.rs` | the winit `KeyCode` ↔ token map and the `keyconfig.ron` shape (the one persisted type still in the app, because it is the one that needs winit) |

The screen and `app_*` modules use `use crate::*;` so they see the crate-root items, and they are
*descendants* of the crate root — which is what lets them read `AppShared`'s private fields without
`App` / `AppShared` having to expose them. `rbms-config` owns the versioned settings schema;
`rbms-library` and `rbms-store` provide the SQLite song and score stores that the player wires to
the configuration directory.

## How to extend

- **New default table** (a table every player is given once): add a `DefaultTable` to `rbms_config::DEFAULT_TABLES` (`crates/rbms-config/src/tables.rs`). Its `id` and `location` are never changed after a release; `group` is the root row several tables share. It goes right after the nearest earlier default table the player still has. The player carries a copy only if `SNAPSHOTS` in `tablesrc/seed.rs` has one (the insane table does; the Stella tables are always fetched). A table the website copies daily also needs an entry in `REMOTE_TABLES` (`web/src/server/service/domain/table-refresh/table-sources.ts`) with the same id.
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
- **A word the player shows** (2026-10-24): never write it as a literal. Add an entry to a `translations!` catalog (`apps/rbms-player/src/text/**`, or `crates/rbms-render/src/text/**` for the built-in select, result and HUD screens) with its three strings, in the vocabulary of `docs/reference/2026-10-24-i18n-glossary.md` (a new term goes into the glossary first), call `Catalog::get(language())` or `fill` where it is drawn, and let the catalog's test (`catalog_problems`) check it. Game terms stay in English. Logic must not compare a translated string: compare the English canonical value and draw a separate `shown` (the settings screen does). A screen that caches text keys its cache on the language. A notice that tells the player the result of what they did is `notify_text`; one that carries an operating-system or server error ends with that error as it came; internal diagnostics stay English. In the default skin a fixed word is an entry of `assets/skins/rbms-default/lua/i18n/<area>.lua` read with `i18n.t(key)` (`skin.md` §3.4.7). The website's words are `web/messages/{ko,en,ja}/<namespace>.json` (below).
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
- A test must pass however long the machine takes: hand a frame and a key the instant they happen at and assert against that instant, never against `Instant::now()` read later; give a client that has to get its answer a long budget and keep short timeouts for the tests of a timeout; name a file in the shared temporary folder with the process id and a counter and remove it with a drop guard. How the capture harness stops the clocks: `architecture.md`, "Testing and the lint gate".
- File writes of a running app go through the save thread (`apps/rbms-player/src/save_worker.rs`). In a unit test an app writes where the save is asked for (`SaveWorker::for_this_build`), so a test may read a file straight after saving it; a test of the thread itself, or of "the frame thread writes nothing", sets `shared.saves = SaveWorker::spawn_gated(..)` and calls `flush()` before it reads.
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
