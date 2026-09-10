# Architecture

rbms is a Rust port of the reference implementation's PLAY core: a BMS rhythm-game player. Cargo workspace, 12 library
crates + 2 apps. The single source of truth for *behaviour* is the code + its tests; this document is
the map.

## Crate dependency order (top → bottom)

```
apps/rbms-player ─► { rbms-config, rbms-library, rbms-play, rbms-render, rbms-audio, rbms-ir, rbms-judge, rbms-table }
     rbms-config ─► rbms-store, rbms-judge, rbms-chart
     rbms-play   ─► rbms-store, rbms-judge
     rbms-library ─► rbms-chart ─► rbms-parser ─► rbms-model
apps/rbms-cli (headless inspector) ─► rbms-parser, rbms-chart, rbms-config, rbms-library, rbms-store
```

`rbms-store` sits at the bottom with `rbms-model`: it depends on serde/RON alone, which is what lets both the
configuration crate and the play crate share it without a cycle. `rbms-table` deliberately does **not** depend on
`rbms-library` — it takes an iterator of md5s, so the join runs in the app rather than inverting the layering.

| Crate | Role |
|---|---|
| **rbms-model** | Dependency-free core types: `Micros = i64` (the global time unit), `Note`/`TimeLine`/`Model`, `Mode` (channel→lane map as data). |
| **rbms-parser** | BMS lexical: UTF-8/BOM/Shift-JIS decode, `#RANDOM`/`#IF` resolved deterministically from a seed, MD5+SHA-256 of raw bytes, base36/62, `#mmmCC` data lines → `BmsSource`. |
| **rbms-chart** | `BmsSource` → fully-timed `Model`: `detect_mode`, measure→µs integration (BPM/STOP/SCROLL), LN/LNOBJ/mines, `note_density`, `scroll` (render offsets), `shuffle` (note options). |
| **rbms-judge** | Stateful judge: reference windows (mode-aware), PG/GR/GD/BD/POOR/MISS, combo/EX/early-late, gauges (6 kinds), 空POOR, clear lamp. |
| **rbms-audio** | RT-safe output: symphonia decode + cpal lock-free mixer. **Master clock = mixed sample count** (the timing-source fix vs the reference implementation). |
| **rbms-render** | Backend-agnostic 2D: `Renderer` trait (`fill_rect`) → wgpu/CpuCanvas. Skin (RON), playfield, HUD, result, select, key-bomb, multilingual font (cosmic-text), **UI theme** (`docs/theme.md`). |
| **rbms-ir** | "IR-superset" score-server contract: `ScoreServer` trait + serde DTOs + HTTP/Null clients. |
| **rbms-table** | BMS difficulty tables (header.json/data.json): fetch, md5-match, level grouping, disk cache. |
| **rbms-store** | Local persistence: `scores.ron` + `replays/*.ron`, the atomic write every store file goes through, and the judging-rule version stamped on each record. serde/RON only — lamps travel as numeric ids so the judge engine stays out of the file layer. |
| **rbms-library** | The song library: cancellable folder scan → `SongEntry`, the lazy per-chart `ChartDetail`, and the md5-indexed `Library`. Headless, so `rbms-cli scan` exercises it. |
| **rbms-config** | One versioned configuration document (`Config` + `schema_version`), the migration that absorbs the pre-version `settings.ron`/`folders.ron`/`tables.ron`, and the settings **descriptor table** the settings screen is generated from. |
| **rbms-play** | Real-time play driver. `Player` advances over the song clock (scheduling and audible axes separately), emits keysound events, feeds judging, tracks beams/bombs; `PlaySession` is one whole run — judging + replay + calibration + analysis clock — emitting sound through a `SoundSink` so it can run with no audio device. |
| **apps/rbms-player** | Native desktop app: winit event loop, wgpu renderer (`gpu.rs`), the stage machine (Select/Settings/KeyConfig/Tables/Folders/Loading/Play/Result) under `stage/`, IR account and ranking UI under `ir_*.rs`. Built as a library with a three-line binary, so integration tests reach its types. |
| **apps/rbms-cli** | Headless companion: a bare chart path prints metadata/hashes/counts, `scan <dir>` drives `rbms-library`, `config <settings.ron>` drives `rbms-config` (migration result, the copy it kept, then every settings row), `scores <scores.ron> [--md5 …]` queries `rbms-store`. It never depends on the player, so no window/audio stack is pulled into a CLI build. |

## End-to-end data flow (disk → pixels + audio)

```
.bms bytes ─► rbms_parser::parse_with ─► BmsSource
           ─► rbms_chart::detect_mode + to_model ─► Model   (raw objects become absolute Micros here)
           ─► rbms_chart::shuffle::apply (optional)
           ─► rbms_play::PlaySession::new ─► Player ─► JudgeEngine::from_model
           ─► rbms_audio::decode_bytes + AudioEngine::load   (keysounds, decoded on background threads)
   per frame: PlaySession state + rbms_chart::scroll offsets ─► rbms_render::render_playfield/hud/... ─► Gpu (wgpu)
```

Everything drawn — notes, beams, text (cosmic-text), BGA frame — lowers to `Renderer::fill_rect` (plus one textured BGA pipeline in the wgpu backend).

## The realtime play loop

- **Master clock is audio frames, not vsync.** `Mixer` increments an `AtomicU64` of mixed frames each callback; `AudioEngine::clock_us() = frames·1e6/rate`. `App::frame` computes `song_us = audio.clock_us() - anchor_us` (anchor captured at *play start*, after keysounds finish loading, so load time doesn't count against position).
- **Per frame** `PlaySession::tick(clock, sink)`: feed any due replay input, book keysounds ahead on the scheduling axis (`Player::update_schedule`), then judge on the audible axis (`Player::update_judge`) — flush BGM, autoplay/auto-lane actions, expire tap beams, and let `JudgeEngine::update` sweep unhittable notes to MISS and finalise over-held LNs.
- **Sound leaves through a trait.** `PlaySession` emits `SoundRequest`s into a `SoundSink`; the app's `PlayAudioSink` is the only code that maps them onto `AudioEngine` (source → bus, `#WAVxx` → play id namespace). With `NullSink` a whole run — including a replay — reproduces headlessly, which is how play is tested without a device.
- **Input** `PlaySession::press(lane, raw_us, sink)` records the raw time for the replay and judges at `raw + judge_offset`; underneath, `Player::press(lane, now_us)`: lights beam, plays nearest keysound, `JudgeEngine::press` gates candidates by the active scaled windows, classifies by `dmtime = note_time - press_time`. 空POOR (far-early) does not consume the note or break combo.
- **Invariants**: single `Micros = i64` everywhere (`measure_us(bpm) = 240_000_000/bpm`); `dmtime` sign convention (`>0` early/FAST); `early[i] + late[i] == counts[i]`; monotonic cursors (rewind only via replay re-simulation); `BPM <= 0` guards prevent NaN.

## Determinism & correctness

- **Chart identity** = MD5 + SHA-256 of **raw bytes** → flows to `Model.md5/sha256`, `ChartId`, score records, replays, table md5-joins.
- **`#RANDOM`** resolved by an LCG seeded from `random_seed`; same seed → byte-identical `BmsSource`.
- **Shuffle** uses a hand-rolled xorshift64 (no `rand` dep) → replay-reproducible. DP uses per-side seeds.
- **Replay** stores `seed + random + offset_ms + gauge` to reconstruct the exact lane layout and re-judge.
- **Judge windows** are mode-aware (`JudgeWindows::note_for_mode` / `ln_end_for_mode`); LN-release windows scale with `#RANK` and JUDGE WIDTH just like note heads.

## Data-driven surfaces

- **Modes** are data (`Mode` + an `[i8;18]` channel map) — a new key mode is a constant, never an engine branch.
- **Skins** (`SkinConfig`, RON): note-field geometry, palettes, gauge thresholds, key-bomb, DP layout.
- **UI theme** (`ThemeConfig`, RON): menu/chrome colours — see `docs/theme.md`.
- **Judge windows and gauges** are data: `rbms-judge/data/judge.ron` (six modes) and `data/gauge.ron` (`BEAT_7K`) are embedded with `include_str!` and can be overridden from disk. The in-code const tables stay for one release as the fallback *and* as the reference of a **parity guard test** that asserts the parsed data equals them field by field — the values Phase A matched to the reference cannot drift as the data path takes over.
- **Candidate selection** is a policy value (`JudgeAlgorithm`: Combo / Duration / Lowest / Score) rather than a hard-coded loop; the default stays `Duration`, which is what this engine has always done (see `acknowledge/reference-divergences.md` C-D1).
- **The settings screen is a table**: `rbms_config::SETTINGS` describes every row (tab, label, kind, range, help, visibility predicate) and the screen renders `tab_rows` → `display_value`, feeding keys back through `adjust`. Adding an option is one row, not nine edits across a runtime struct, a persisted struct, two converters, two matches and a tab array.
- **Configuration is one versioned document** (`settings.ron`, `schema_version`): play/judge/display/audio/network/library options in one `Config`. A pre-version file migrates on load and absorbs `folders.ron`/`tables.ron` **without deleting them**; a file from a newer schema is left untouched and the app starts on defaults, so a downgrade cannot destroy settings.
- **Keymaps** (`KeyConfig`, RON), **skins**, **themes**, **difficulty tables** and **replays** stay their own files under the config dir (every file `#[serde(default)]`, parse-fail → `.ron.bak` + defaults). The config dir resolves `HOME` → `USERPROFILE` → `.` for cross-platform parity (Windows lands in `%USERPROFILE%\.config\rbms`); `config_dir_from` is unit-tested. Every write goes through `rbms_store::write_atomic` (temp + rename).

### JSON skins (`rbms-skin` + `rbms-render::skin_render`)

Since Phase E a screen can be driven by a reference-format JSON skin document instead of its built-in layout: `rbms-skin` loads the document (json5, sandboxed Lua expressions, skin-root file resolution), binds its timers and property ids to live `PlaySession`/app state, and `rbms-render::skin_render` draws it through the textured-quad, clip and rotation primitives of the `Renderer` trait (batched per draw order in the GPU backend). When no document is selected the built-in screens render exactly as before, which the golden signatures pin. Selection and customisation are persisted in `rbms-config` (`Config.skin`) and edited on the SKIN settings tab.

## App UI state machine (`Stage`)

The screens: `Select` (song list: search `/`, sort `F3`, clickable bottom nav, IR ranking panel) · `Settings` (tab strip over the descriptor table, incl. a **NETWORK** tab that signs in and edits SERVER URL / PLAYER ID in place) · `KeyConfig` · `Tables` (difficulty-table manager) · `Folders` (multi-folder library manager) · `Loading` (determinate keysound-decode progress bar) · `Play` · `Result`.

How they are wired:

- **`App = { shared: AppShared, stage: Stage, suspended: Vec<Stage> }`.** `AppShared` holds what outlives a screen change (config, library, scores, audio engine, GPU, skin, IR session, clocks, frame stats); each `Stage` variant holds only its own screen's state. That split is what lets `self.stage.update(&mut FrameCtx { shared: &mut self.shared, .. })` borrow-check — one struct holding both would not.
- **Each screen implements `StageHandler`** (`update` / `draw` / `handle_key` / `handle_mouse` / `on_enter` / `on_exit` / `debug_lines`). `Stage::handler()` / `Stage::view()` are the only two places that match on the variants — they hand back a `&mut dyn StageHandler`, and every dispatch method is a one-line call through it. The match is still exhaustive, so the compiler still catches a screen missing from any path; what it avoids is repeating an eight-arm match once per method. `Play` and `Select` are boxed so their size does not set every variant's.
  (The Phase C spec §2.3 called for a per-method match and no trait object; the single-match-plus-`dyn` form keeps that spec's stated guarantee — the compiler catches a missing variant — with one match instead of seven.)
- **One transition rule, in one place.** A screen returns `Transition::{Stay, Open, To, Back, Quit}` and `App::apply` runs `on_exit` → swap → `on_enter`. `Open` suspends the current screen and `Back` resumes it exactly as it was, which is how the settings screens return to the browser. The scattered `self.stage = Stage::X` assignments are gone.
- **`on_exit`/`on_enter` carry the ordering contracts** a screen change has to honour — the select preview is torn down on the way out of the browser before anything else touches the audio engine.
- **The browser's *list* state stays in `AppShared`**, not in `SelectState`: which folder is open, the filtered rows, the search text, the sort and the cursor. Every other screen returns to the browser and several act on the focused row while it is not the live screen (`Tables` and `Folders` rebuild the list, `Loading` resets it to the root, the debug overlay reports the cursor from anywhere), so the list is shared state that outlives the screen. `kc_edit_mode` is in `AppShared` for the same reason: the key-config screen is rebuilt on every visit and the mode it was editing has to survive that. The Phase C spec §2.2 assigned both to their screens; this is a deliberate departure, and it is the reason `AppShared` has ~69 fields rather than the ~40 the spec estimated.
- **`App` and `AppShared` live in the crate root** (`lib.rs`) rather than a module of their own. Every screen and `app_*` module is a *descendant* of the crate root, which is what lets them touch `AppShared`'s private fields; moving the two structs into a sibling module would force every one of those fields to `pub(crate)` and buy nothing but a shorter file. What did move out are the parts with no such coupling: `assets.rs` (bundled skins, theme template, file resolution, keysound/BGA decode, library scan) and `stage/select/{preview,scene}.rs`.
- **The frame loop and the event handler are dispatches**: `frame()` is fps/RAM, the polls, `stage.update`, `stage.draw` and two app-wide overlays; `window_event` translates six winit events into `handle_key`/`handle_mouse`/`frame`.

## Testing and the lint gate

Over 1,650 tests across the workspace, edge-case heavy (boundaries, malformed input, invariants,
round-trips, determinism). Run `cargo test --workspace`. The deterministic render backend `CpuCanvas`
makes pixel-level rendering testable without a GPU — the player's screens are rendered onto a
headless canvas in tests, and a whole run (replay included) reproduces through `NullSink` with no
audio device.

Lints are a gate, not a suggestion:

- The workspace defines the lint set once (`[workspace.lints]`: `unsafe_code = "deny"`, clippy `all` at warn) and **every crate opts in** with `[lints] workspace = true`.
- **Every crate root carries `#![forbid(unsafe_code)]`** — including the player, whose `bytemuck` derives turned out not to trip it. The single exception is `rbms-audio`'s `tests/rt_safety.rs`, which installs a counting global allocator to prove the mixer callback never allocates; that test crate root opts out with `#![allow(unsafe_code)]`.
- CI runs `cargo fmt --all --check` and `cargo clippy --workspace --all-targets --all-features -- -D warnings` as **failing** steps (they were informational before), so a new warning cannot land.
- `rust-toolchain.toml` pins an exact toolchain version rather than tracking `stable`, so a new release cannot break the gate with a new lint until the pin is moved deliberately. The CI workflow installs the same version.

## See also

- [`crates.md`](crates.md) — per-crate API reference.
- [`development.md`](development.md) — build/run/test, app module layout, conventions.
- [`theme.md`](theme.md) — the `theme.ron` UI-theme schema.
- [`roadmap.md`](roadmap.md) — deferred work + pinned edge-case behaviours.
