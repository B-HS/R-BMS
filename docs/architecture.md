# Architecture

rbms is a Rust port of beatoraja's PLAY core: a BMS rhythm-game player. Cargo workspace, 9 library
crates + 2 apps. The single source of truth for *behaviour* is the code + its tests; this document is
the map.

## Crate dependency order (top → bottom)

```
apps/rbms-player ─► rbms-play ─► { rbms-render, rbms-audio, rbms-ir, rbms-judge, rbms-chart, rbms-table }
                                            └─► rbms-parser ─► rbms-model
apps/rbms-cli (debug inspector) ─► rbms-parser, rbms-chart
```

| Crate | Role |
|---|---|
| **rbms-model** | Dependency-free core types: `Micros = i64` (the global time unit), `Note`/`TimeLine`/`Model`, `Mode` (channel→lane map as data). |
| **rbms-parser** | BMS lexical: UTF-8/BOM/Shift-JIS decode, `#RANDOM`/`#IF` resolved deterministically from a seed, MD5+SHA-256 of raw bytes, base36/62, `#mmmCC` data lines → `BmsSource`. |
| **rbms-chart** | `BmsSource` → fully-timed `Model`: `detect_mode`, measure→µs integration (BPM/STOP/SCROLL), LN/LNOBJ/mines, `note_density`, `scroll` (render offsets), `shuffle` (note options). |
| **rbms-judge** | Stateful judge: beatoraja windows (mode-aware), PG/GR/GD/BD/POOR/MISS, combo/EX/early-late, gauges (6 kinds), 空POOR, clear lamp. |
| **rbms-audio** | RT-safe output: symphonia decode + cpal lock-free mixer. **Master clock = mixed sample count** (the timing-source fix vs beatoraja). |
| **rbms-render** | Backend-agnostic 2D: `Renderer` trait (`fill_rect`) → wgpu/CpuCanvas. Skin (RON), playfield, HUD, result, select, key-bomb, multilingual font (cosmic-text), **UI theme** (`docs/theme.md`). |
| **rbms-ir** | "IR-superset" score-server contract: `ScoreServer` trait + serde DTOs + HTTP/Null clients. |
| **rbms-table** | BMS difficulty tables (header.json/data.json): fetch, md5-match, level grouping, disk cache. |
| **rbms-play** | Real-time play driver: `Player` advances over the song clock, emits keysound events, feeds judging, tracks beams/bombs. |
| **apps/rbms-player** | Native desktop app: winit event loop, wgpu renderer (`gpu.rs`), the UI state machine (Select/Settings/KeyConfig/Tables/Folders/Loading/Play/Result), persistence. Split across `app_input.rs` / `app_select.rs` / `app_play.rs` + support modules. |
| **apps/rbms-cli** | Thin chart inspector (parse → print metadata/hashes/counts). |

## End-to-end data flow (disk → pixels + audio)

```
.bms bytes ─► rbms_parser::parse_with ─► BmsSource
           ─► rbms_chart::detect_mode + to_model ─► Model   (raw objects become absolute Micros here)
           ─► rbms_chart::shuffle::apply (optional)
           ─► rbms_play::Player::new ─► JudgeEngine::from_model
           ─► rbms_audio::decode_bytes + AudioEngine::load   (keysounds, decoded on background threads)
   per frame: Player state + rbms_chart::scroll offsets ─► rbms_render::render_playfield/hud/... ─► Gpu (wgpu)
```

Everything drawn — notes, beams, text (cosmic-text), BGA frame — lowers to `Renderer::fill_rect` (plus one textured BGA pipeline in the wgpu backend).

## The realtime play loop

- **Master clock is audio frames, not vsync.** `Mixer` increments an `AtomicU64` of mixed frames each callback; `AudioEngine::clock_us() = frames·1e6/rate`. `App::frame` computes `song_us = audio.clock_us() - anchor_us` (anchor captured at *play start*, after keysounds finish loading, so load time doesn't count against position).
- **Per frame** `Player::update(now_us)`: flush BGM, autoplay/auto-lane actions, expire tap beams, then `JudgeEngine::update` sweeps unhittable notes to MISS and finalizes over-held LNs.
- **Input** `Player::press(lane, now_us)`: lights beam, plays nearest keysound, `JudgeEngine::press` gates candidates by the active scaled windows, classifies by `dmtime = note_time - press_time`. 空POOR (far-early) does not consume the note or break combo.
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
- **Keymaps** (`KeyConfig`, RON), **note options**, **difficulty tables**, **song folders** (multi-folder), all persisted under `~/.config/rbms/` (every file `#[serde(default)]`, parse-fail → `.ron.bak` + defaults).

## App UI state machine (`Stage`)

`Select` (song list: search `/`, sort `F3`, clickable bottom nav buttons) · `Settings` · `KeyConfig` · `Tables` (difficulty-table manager) · `Folders` (multi-folder library manager) · `Loading` (determinate keysound-decode progress bar) · `Play` · `Result`.

## Testing

~880 unit tests across the workspace, edge-case heavy (boundaries, malformed input, invariants,
round-trips, determinism). Run `cargo test --workspace`. The deterministic render backend `CpuCanvas`
makes pixel-level rendering testable without a GPU.

## See also

- [`crates.md`](crates.md) — per-crate API reference.
- [`development.md`](development.md) — build/run/test, app module layout, conventions.
- [`theme.md`](theme.md) — the `theme.ron` UI-theme schema.
- [`roadmap.md`](roadmap.md) — deferred work + pinned edge-case behaviours.
