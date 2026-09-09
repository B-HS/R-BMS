# Crate reference

Per-crate API + invariants reference, accurate to the current code. Generated from a full read of each
crate; see `architecture.md` for the big picture and `development.md` for build/test/conventions.

Dependency order: `apps/rbms-player → rbms-play → {rbms-render, rbms-audio, rbms-ir, rbms-judge, rbms-chart, rbms-table} → rbms-parser → rbms-model`.

---

### rbms-model
- **Role**: The foundational data-model crate for the rbms BMS rhythm-game project. It defines the in-memory chart representation (notes, timelines, song metadata, hashes) and the play-mode profiles, with all engine-agnostic timing constants. It contains pure data + lookup logic only; no parsing, audio, or rendering.
- **Public API**:
  - `Micros` — `i64` type alias; the global absolute-time unit in microseconds.
  - `US_PER_MEASURE_NUM` — `f64` const `240_000_000.0`; numerator of the `240_000_000 / bpm` measure-length invariant.
  - `measure_us(bpm: f64) -> f64` — microseconds per 4/4 measure at `bpm` (`US_PER_MEASURE_NUM / bpm`).
  - `LnKind` — enum `Ln | Cn | Hcn`; long-note flavor.
  - `NoteKind` — enum `Normal | LongStart { ln } | LongEnd { ln } | Mine { damage: f64 }`.
  - `Note` — a single note: `kind`, `wav: i32`, `start_us`, `duration_us`, `time_us`, `section: f64`, plus `layered: Vec<Note>` for stacked notes. `Note::normal(wav, time_us, section)` builds a `Normal` note with long-note fields zeroed.
  - `TimeLine` — one timing row: `time_us`, `section`, per-lane `notes`/`hidden` (`Vec<Option<Note>>`), `bgnotes: Vec<Note>`, `section_line: bool`, `bpm`, `stop_us`, `scroll`, `bga: i32`, `layer: i32`. `TimeLine::empty(lanes, time_us, section, bpm)` allocates `None`-filled lane vectors with default scroll `1.0`, `bga`/`layer` `-1`.
  - `ModelMeta` — song metadata (title, subtitle, artist, subartist, genre, play_level, difficulty, rank, total, stagefile); `Default` zeroes/empties all.
  - `Model` — top-level chart: `mode`, `meta`, `wavmap`/`bgamap: Vec<String>`, `init_bpm`, `timelines: Vec<TimeLine>`, `md5`, `sha256`.
  - `Mode` (re-exported from `mode`) — a play-mode profile struct: `name`, `key`, `player`, `scratch: &[usize]`, `channel_assign: &[i8; 18]`.
  - `Mode` constants `BEAT_7K`, `BEAT_5K`, `BEAT_10K`, `BEAT_14K`, `POPN_9K`, and `Mode::ALL` slice.
  - `Mode::is_scratch(lane) -> bool` — is this logical lane a scratch lane.
  - `Mode::lane_of_raw(raw) -> Option<usize>` — maps a raw 18-wide channel index to a logical lane.
- **Key invariants & algorithms**:
  - **Measure-length invariant**: `measure_us(bpm) = 240_000_000 / bpm`. This is the core timing constant; `240_000_000` is microseconds per measure at 1 BPM (4 beats × 60s × 1e6). It is NOT guarded — see gotchas.
  - **Channel mapping is data, not code**: `channel_assign` is a fixed 18-entry table indexed by a raw channel where P1 occupies raw `0..9` and P2 occupies raw `9..18`. Value `-1` means "ignored"; any other value is the logical lane. New key modes are intended to be added as new `Mode` constants/tables only, never by branching engine logic.
  - **`lane_of_raw` algorithm**: returns the table value only when `0 <= value < self.key`, else `None`. `key` therefore both bounds owned lanes and is what makes a 7K mode (`key 8`) ignore the P2 lanes that 14K (`key 16`) keeps — the same `BEAT7`/`BEAT5` tables are shared across 1P and 2P modes, with `key` selecting how many lanes are live. Out-of-range raw (>=18) yields `None`.
  - **Shared tables**: `BEAT7` is shared by `BEAT_7K` (key 8) and `BEAT_14K` (key 16); `BEAT5` by `BEAT_5K` (key 6) and `BEAT_10K` (key 12); `POPN` by `POPN_9K` (key 9). Scratch lanes: 7K→`[7]`, 5K→`[5]`, 10K→`[5,11]`, 14K→`[7,15]`, popn→`[]`.
  - **Note time fields**: `time_us` is the note's absolute play time; `section` is its measure-relative position; `start_us`/`duration_us` are populated only for long notes (zeroed by `Note::normal`). `layered` holds stacked/overlaid notes at the same position.
  - **TimeLine defaults**: `scroll` defaults to `1.0`, `stop_us` to `0`, `bga`/`layer` to `-1` (sentinel for "none"), `section_line` to `false`. `notes` and `hidden` are independently allocated `Vec<Option<Note>>` of length `lanes`.
- **Gotchas / edge cases**:
  - `measure_us` is intentionally unguarded: BPM `0.0` returns `+inf` (positive), and negative BPM returns a negative length (e.g. `-120.0 → -2_000_000.0`). Both are pinned by tests and flagged in-code as "suspect" — do not add guards without checking callers/tests.
  - `Note::wav`, `time_us`, and `section` accept and preserve negative values (no validation).
  - `lane_of_raw` is total over all `usize` inputs (out-of-range, `usize::MAX` → `None`) because it uses `slice::get`.
  - `Mode::ALL` has exactly 5 entries; tests assert distinct names, `player ∈ {1,2}`, 2-player scratch modes have exactly 2 scratch lanes, and scratch lanes are `< key` and reachable via `lane_of_raw`.
  - Every logical lane `0..key` is produced by exactly one raw index (a pinned bijection invariant) — breaking the table's uniqueness/coverage fails tests.
  - `NoteKind::Mine` carries `f64 damage`, so `NoteKind` is `PartialEq` but not `Eq`; `LnKind` and `Mode` are `Eq`.
- **Tests**: Inline `#[cfg(test)] mod tests` in both files. `src/lib.rs` covers the `measure_us` invariant (known values, monotonicity, inverse proportionality, zero/negative BPM), `TimeLine::empty` defaults/lane allocation/independence, `Note::normal` zeroing and negative-value preservation, enum equality, `ModelMeta::default`, and `Note` clone round-trip. `src/mode.rs` covers exhaustive `lane_of_raw` maps for all 5 modes (raw `0..18`), out-of-range raw handling, `lane_of_raw`-vs-`key` invariants, the full-key-range bijection, `is_scratch` behavior, `channel_assign` self-consistency (no duplicate lanes, `-1`-only sentinel, P1-group values `< key`), and `Mode` constant invariants (distinct names, key counts, value equality, player/scratch-count rules).

---

### rbms-parser

- **Role**: Pure lexical/structural parser for BMS-family charts (`.bms`/`.bme`/`.bml`/`.pms`). Turns raw file bytes into a `BmsSource` (headers, resource maps, per-measure channel objects) plus content hashes, resolving `#RANDOM`/`#IF` control flow but doing no timing/BPM integration.

- **Public API**:
  - `parse(bytes: &[u8]) -> BmsSource` — parse with default options (seed 0).
  - `parse_with(bytes, opts: ParseOptions) -> BmsSource` — parse with an explicit `random_seed`.
  - `ParseOptions { random_seed: u64 }` — only knob; `Default` is seed `0`.
  - `BmsSource` — top-level result: `headers`, `wav`/`bmp` (`BTreeMap<u32,String>`), `bpm_def`/`stop_def`/`scroll_def` (`BTreeMap<u32,f64>`), `measures: BTreeMap<u32,Measure>`, `base: u32`, `md5`, `sha256`.
  - `Headers` — all `#`-header scalar fields (title, artist, `init_bpm`, `rank`, `total: Option<f64>`, `lnobj: Option<u32>`, `lntype`, `lnmode`, `difficulty`, etc.) with a non-trivial `Default`.
  - `Measure { rate: f64, channels: Vec<ChannelData> }` and `ChannelData { channel: u32, objects: Vec<Obj> }`.
  - `Obj { num, den, c0, c1 }` — one placed token; `value(base)`/`value16()` decode the pair, `pos()` returns `num/den` as `f64`.
  - `digit(c: u8, base: u32) -> Option<u32>` and `parse_pair(c0, c1, base) -> u32` (re-exported from `base.rs`) — radix decoding for 2-char tokens.

- **Key invariants & algorithms**:
  - **Base 36 vs 62**: default `base` is `36`. It is upgraded to `62` only by a `#BASE 62` directive — detected in a *first pre-pass* over all lines (so it applies regardless of where the directive sits), and again in the header handler. In base 62 lowercase `a..z` map to `36..61`; otherwise lowercase aliases uppercase (`a..z` == `10..35`).
  - **Channel codes are always parsed in base 36** (`parse_data_line` calls `parse_pair(b[3],b[4],36)`), independent of `src.base`. Object *values* are stored raw (`c0`,`c1`) and decoded lazily via `Obj::value`.
  - **Channel 2 = measure length scale** (`#xxx02`): parsed as a single `f64` into `Measure.rate` (default `1.0`), not as objects.
  - **Object placement**: data string is split into 2-char tokens; `len = chars/2`. Each token at index `i` becomes `Obj { num: i, den: len }`, so position is `i/len`. `00` tokens and tokens with non-ASCII or non-base-36 digits are skipped. Empty channels are not pushed.
  - **Control flow (`control.rs`)**: deterministic LCG RNG seeded with `seed.wrapping_add(0x9E3779B97F4A7C15)`; `next_rng` is the classic `6364136223846793005 * x + 1442695040888963407`, returning `rng >> 33`. `#RANDOM n`/`#RONDAM` draw `1..=n` (n==0 -> 1); `#SETRANDOM` forces a value. `#IF`/`#ELSEIF`/`#ELSE`/`#ENDIF` use a `Frame` stack with `active`/`matched` flags; `current_random` reads the nearest enclosing `Random` frame; nesting honored via `active_excluding`. `handle()` returns `true` for any control line (consumed); non-control lines are skipped entirely when `ctrl.active()` is false.
  - **Header dispatch**: head/rest split on first whitespace, head uppercased. Indexed resources keyed by fixed-length head: `#WAVxx`/`#BMPxx`/`#BPMxx` (len 5, id from bytes 3..4), `#STOPxx` (len 6, bytes 4..5), `#SCROLLxx` (len 8, bytes 6..7). `#BPMxx`/`#STOPxx`/`#SCROLLxx` only insert when `rest` parses as `f64`.
  - **Hashing**: `md5` and `sha256` are computed over the **raw input bytes** (before any decode), lowercase hex.
  - **Text decode**: strips a UTF-8 BOM (`EF BB BF`) if present; else uses bytes as UTF-8 if valid; else falls back to `encoding_rs::SHIFT_JIS`.

- **Gotchas / edge cases**:
  - `parse_pair` is lenient: a bad digit silently contributes `0` (via `unwrap_or(0)`), so malformed pairs decode rather than error.
  - `is_data_line` requires `len >= 6`, three leading ASCII digits, and a `:` at byte index 5 — channel chars at 3..4 are *not* validated here, so a non-base-36 channel still routes to data parsing (and `parse_pair` will fold the bad digit to 0).
  - Measure index is computed only from the first three digit bytes (`xxx`), so it caps at `999`.
  - `#PLAYER`/`#RANK`/`#BPM`/`#LNTYPE`/etc. fall back to `Default`-ish literals on parse failure (`unwrap_or`), e.g. `init_bpm` -> `130.0`, `rank` -> `3`, `player` -> `1`. `#TOTAL` and `#LNOBJ` instead become `None` on failure.
  - `#LNOBJ` is decoded with the *current* `src.base`; only the first 2 bytes of `rest` are used.
  - Object `den` equals the token count of that single channel line; multiple lines on the same channel/measure are stored as separate `ChannelData` entries (not merged), each with its own `den`.

- **Tests**: `#[cfg(test)] mod tests;` declared at the bottom of `lib.rs` (lives in a sibling `tests.rs` / `tests/` module under `crates/rbms-parser/src/`). Expect coverage of radix decoding (base 36/62), data-line object placement and `00`-skipping, channel-2 rate, header parsing/defaults, hash stability, and `#RANDOM`/`#IF` branch selection determinism under a fixed seed.

---

### rbms-chart
- **Role**: Converts a parsed `BmsSource` into a playable, timed `Model` (lane assignment, note/LN/mine/BGA/BPM/STOP/SCROLL resolution, absolute-time stamping) and provides the derived gameplay math: note-density stats, scroll/offset geometry, and deterministic lane-shuffle ("note option") transforms.
- **Public API**:
  - `detect_mode(src, filename) -> Mode` — infer play mode from used channels + extension (`.pms`→POPN_9K, P2 channels→14K, keys 6/7→7K else 5K).
  - `to_model(src, mode) -> Model` — main builder: events→sorted timelines→`assign_times`, plus meta/wavmap/bgamap/hashes.
  - `count_playable_notes(model) -> usize` — playable count: excludes mines, and a plain LN/Normal counts **1** (its tail is excluded), but a CN/HCN counts **2** (head + the release end, judged separately) — kept in step with the judge engine's `total_notes`.
  - `struct NoteDensity { bins, peak, avg, end }` + `note_density(model, total_value) -> NoteDensity` — per-second density histogram and the three scalar readouts (reference port).
  - `scroll::LaneGeometry { hu, hl }` with `height()` — lane pixel geometry (`hl`=judgment line, `hu`=top edge).
  - `scroll::green_number(bpm, hispeed, scroll, lanecover) -> f64` — IIDX green number (lane traversal ms).
  - `scroll::closed_form_offset(...)` — closed-form pixel offset above the judgment line (fast path / test oracle).
  - `scroll::visible_offsets(timelines, microtime, hispeed, lane_height)` — FLOATING scroll: exact BPM/SCROLL/STOP segment walk, returns `(timeline_index, pixel_offset)`.
  - `scroll::constant_offsets(...)` — CONSTANT (green-number-fixed) scroll, BPM-independent.
  - `shuffle::NoteOption` (8 variants `Off/Mirror/Random/SRandom/RRandom/Rotate/HRandom/AllScratch`) with `ALL`, `label()`, `from_str()`.
  - `shuffle::lane_permutation(option, mode, seed) -> Vec<usize>` — whole-chart `perm[old]=new` (MIRROR/RANDOM/R-RANDOM/ROTATE).
  - `shuffle::apply(model, option, seed)` — apply an option in place (dispatches per-row options to internal walkers).
- **Key invariants & algorithms**:
  - `SECTION_EPS = 1e-7`: events whose fractional `section` differ by less than this collapse into one `TimeLine`; events are sorted by `section` before timelines are built, and `to_model` emits one bar-line `TimeLine` per measure.
  - Channel→role map in `role()` uses decimal-parsed channel numbers (e.g. 37–45 = base-36 `11`–`19` key lanes, 73–81 = P2, `1020`=SCROLL ch `SC`); P2 lanes add `+9` to the raw lane index. Unknown lanes → `Role::Ignore`.
  - Time math in `assign_times`: `dt_us = 240_000_000 * Δsection / prev.bpm`; STOP µs = `240_000_000 * s / (192 * bpm)`; non-positive/zero BPM falls back to prior bpm, init bpm defaults to **130** if `≤0`. STOP accumulates additively into `stop_us` and is added to the *previous* segment when advancing.
  - LN handling: `#LNOBJ` converts the *prior* normal note in that lane into a `LongStart`/`LongEnd` pair (tracked via `last_normal`); channel-based LNs (181–225) toggle `ln_open` per lane. `#LNMODE` 2→Cn, 3→Hcn, else Ln — the kind is recorded on the note and now drives judging: `rbms-judge` scores a CN/HCN as two judgments (head + release end) vs a plain LN's one (see that crate).
  - `note_density`: bins sized from the last *note-bearing* timeline (not the trailing bar-line) +2; 7-category per-second array (scr/key × LN-head/body/normal + mine). `peak`=busiest second; `avg`=mean over seconds ≥ `total_notes/bins/4`; `end`=max 5-second-window density after the gauge "border" (`total_notes*(1-100/total_value)`). `total_value ≤ 0` ⇒ standard-BMS default `(7.605*N/(0.01*N+6.5)).max(260)`.
  - Scroll: `visible_offsets` integrates segments exactly and freezes during STOP (full segment height held); `constant_offsets` uses `pps = hispeed*lane_height/2_000_000` and is calibrated so CONSTANT == FLOATING at 120 BPM. Both binary-search `microtime`, emit only future timelines (`i>cur`), and break once `y > lane_height`. Negative `scroll` flips offset sign.
  - Shuffle: deterministic xorshift64 `Rng` (seed 0 remapped to `0x9E3779B97F4A7C15`); scratch lanes never move under key shuffles; DP shuffles stay within a player side (no P1↔P2 crossing — that FLIP is unimplemented). Per-side seed is `seed + side * GOLDEN`. Anti-jack windows: `HRAN_THRESHOLD_US = 125_000` (ceil(15000/120)ms) for key lanes, `SCRATCH_THRESHOLD_US = 40_000` for the scratch lane under ALL-SCRATCH. Long notes are pinned head→tail to the same remapped lane across all options.
- **Gotchas / edge cases**:
  - ALL-SCRATCH / H-RANDOM concentration is a *preference, not a guarantee*: jacks closer than the threshold spill back to key lanes, so tests only assert `>= a few`, not exact placement.
  - `lane_permutation` returns the **identity** for `Off/SRandom/HRandom/AllScratch` — their real work is per-row inside `apply` (don't assume the perm reflects the final layout).
  - `apply_perm`/per-row walkers only act on timelines whose `notes.len()` equals `mode.key`; mismatched rows are skipped silently.
  - Note count is invariant under every option (heavily test-pinned), including dense full rows where notes exceed fresh lanes — the loop falls back to the source lane (`s`) when the pool is exhausted, so it never panics or drops notes.
  - Mine notes use `Note::normal(0,0,..)` with `kind` overwritten to `Mine` and lane wav `0`; mines are excluded from `count_playable_notes` and density note totals, and a plain LN's tail is excluded too — but a CN/HCN end *is* counted (the charge note's separately-judged release).
  - `BgaBase` and `BgaPoor` both map to `EvKind::Bga` (POOR currently shares the base BGA channel handling).
- **Tests**: `lib.rs` has `#[cfg(test)] mod tests;` (external `tests.rs` file, not shown). `scroll.rs` in-file tests cross-check the segment walk vs. closed form under constant BPM, hi-speed/BPM/scroll linearity, STOP freezing, green-number guards, and the future-only/break-at-height window behavior. `shuffle.rs` in-file tests pin: `from_str`/`label` round trips, per-mode bijection + scratch-fixed permutations, DP side isolation, note-count preservation across all options for 5K/7K/9K/10K/14K, LN head/tail alignment, ALL-SCRATCH concentration, determinism per seed, and no-panic on dense/empty charts.

---

### rbms-judge
- **Role**: The judging engine for a BMS rhythm-game player: it matches key presses/releases against chart note timings, classifies them into judges, and tracks combo, EX score, per-judge tallies, and the groove gauge. Pure, deterministic, stateful logic with no I/O or rendering.

- **Public API**:
  - `Judge` (enum) — one judgment outcome: `PerfectGreat`, `Great`, `Good`, `Bad`, `Poor`, `Miss` (discriminant order = severity, 0..5).
  - `JudgeWindows` (struct) — timing windows `pg/gr/gd/bd/ms`, each `(late_bound, early_bound)` in µs; consts `SEVENKEY_NOTE`, `SEVENKEY_LN_END`, `POPN_NOTE`, `POPN_LN_END`.
    - `JudgeWindows::note_for_mode(&Mode)` / `ln_end_for_mode(&Mode)` — pick the note/LN-release table by mode (`"POPN_9K"` vs everything else).
    - `JudgeWindows::scaled(judgerank_percent)` — scale PG/GR/GD/BD by a percent; MS stays fixed.
    - `JudgeWindows::judge(dmtime) -> Option<Judge>` — classify a signed delta, `None` if outside MS.
  - `rank_to_judgerank(rank) -> i32` — `#RANK` index 0..4 → judgerank percent `[25,50,75,100,125]`, clamped.
  - `JudgeEngine` (struct) — the stateful matcher. Key fns: `new(per_lane_times, windows)`, `from_pairs(per_lane (head,Option<end>), windows)`, `from_model(&Model, windows)`, `set_gauge(kind,total)`, `set_windows(w)`, `set_ln_end(w)`, `press(lane,us)`, `release(lane,us)`, `update(now_us)`, `total_notes()`, `total_judged()`, `avg_judge_us()`, `clear_lamp()`. Public fields: `combo`, `max_combo`, `counts[6]`, `ex_score`, `gauge`, `last_judge`, `last_fast`, `fast`, `slow`, `early[6]`, `late[6]`, `empty_poor`.
  - `JudgeResult` (struct) — `{ judge, lane, note_index, fast, delta_us }`.
  - `Gauge` (struct) + `GaugeKind` (AssistEasy/Easy/Normal/Hard/ExHard/Hazard), `ClearType` (NoPlay/Failed/AssistEasy/Easy/Normal/Hard/ExHard/FullCombo/Perfect/Max), `clear_lamp(gauge, counts, max_combo, total_notes)`.

- **Key invariants & algorithms**:
  - **Delta convention**: `dm = note_time - press_time`. `dm > 0` = pressed EARLY/FAST; `dm <= 0` = LATE/SLOW. `dm == 0` (exact) is counted as LATE, not early, and is neither fast nor slow.
  - **Window tables** are the reference implementation `JudgeProperty` values at judgerank 100, µs. NOTE PG `±20k`, GR `±60k`, GD `±150k`; BD is **asymmetric** `(-280k, +220k)`; MS `(-150k, +500k)`. Because `ms.0 == gd.0 == -150k`, there is **no late POOR** — every late delta is GD/BD until it falls off the BD edge into `None`. POOR exists only on the early side `+220_001..=+500_000`.
  - **Empty poor (空POOR)**: a press landing only in MS (beyond BD, i.e. far-early) is an empty poor: increments `empty_poor`, applies the MS gauge penalty (`gauge.update(Judge::Miss)`), sets `last_judge=Poor`, but does **not** consume the note, does **not** touch `counts`/`early`/`late`/`fast`/`slow`/timing, and does **not** break combo. The note stays hittable. Empty poors never block a full combo. This was a recent fix — previously such presses ate notes and broke combo on slightly-early/jack presses.
  - **press()** scans from the lane cursor for the nearest-by-`|dm|` unjudged, non-holding note within the gate `[gate_late=w.bd.0, gate_early=w.ms.1]`; the gate is derived from the *active scaled* windows (not constants) so it tracks judge width. Non-LN: marks judged, applies, records timing. LN head: sets `holding=true`, stores `head_judge`, scores nothing yet (combo/EX advance only on release).
  - **release()** finds the holding note, classifies `dm = end - release` against `ln_end`, and the final judge is `worse(head_judge, end_judge)` (worse = higher discriminant). Marks judged, applies, records timing.
  - **update(now_us)** sweeps: a normal note is MISS only when `head - now < miss_bound (= bd.0)` — strict `<`, so exactly at the bound it is not yet swept. A held LN is force-finalized only when `now > end + LN_MARGIN` (strict `>`; `LN_MARGIN = 200_000`); until then it blocks its lane cursor. **All sweep events are recorded as LATE.** update is idempotent after sweep (cursor advances past judged notes).
  - **LN window** (`ln_end`) is a separate field, wider than the note head (PG `±120k`). `from_model` sets it from `ln_end_for_mode(mode).scaled(rank_to_judgerank(rank))` so releases respect mode + `#RANK` + JUDGE WIDTH (recent fix — was previously a hardcoded 100% constant). Mode-less constructors default `ln_end` to `SEVENKEY_LN_END`.
  - **LN flavour per note** (`JNote.ln: Option<LnKind>`): `from_model` carries the chart's real `LnKind` (Ln/Cn/Hcn) through an internal `from_triples`; the public `from_pairs`/`new` (tests) treat any long note as a plain `LnKind::Ln`, so their signatures are unchanged. `is_charge(ln)` gates the charge-note behaviour to `Cn`/`Hcn` only.
  - **CN/HCN = two counted judgments, plain LN = one.** A charge note (CN/HCN) is judged twice — the **head at press** (committed immediately: `apply`+`record_timing` in `press`) and the **release end at key-up** (its own judge, not capped by the head: `final_judge = end_judge`). A plain LN remains a **single** judgment scored only at release as `worse(head_judge, end_judge)` — byte-identical to before (`is_charge` is false → unchanged path). The same split holds in the `update` sweep (charge head already counted; the end is the swept judgment). `total_notes` (and thus EX/gauge denominators) counts a CN/HCN with an end as **2**, every plain LN/Normal as **1**, matching `rbms_chart::count_playable_notes`. *HCN continuous gauge, CN early-release deferral, and scratch BSS are deferred to Phase 7.* Full spec: `docs/reference/cn-hcn-judgment.md`.
  - **from_model** builds per-lane notes from `model.timelines`: `Normal` → point note; `LongStart`/`LongEnd` pair into `(head, Some(end), Some(ln))` (kind preserved); `Mine` is skipped; an unterminated `LongStart` (no matching `LongEnd`) is dropped (`pending_start` never flushed). Then sets `ln_end` and `set_gauge(Normal, meta.total)`.
  - **EX score** = `2·PG + 1·GR` only. PG/GR/GD keep combo; BD/POOR/MISS break it (`combo = 0`). `max_combo` is updated in `apply()`.
  - **avg_judge_us** = mean signed `dm` over timed hits (press/release, excludes empty poors and sweeps); 0 when nothing timed. `early[i] + late[i] == counts[i]` always holds.
  - **Gauge**: deltas are pre-modified at construction. TOTAL modifier (AssistEasy/Easy/Normal) scales only positive deltas by `total/notes`; LIMIT_INCREMENT (Hard/ExHard) caps PG gain via `pg = clamp((2·total-320)/notes, 0, 0.15)`, then scales positives by `pg/0.15`; Hazard (`Modifier::None`) leaves deltas raw. `notes` is `max(1)`; `total <= 0` falls back to 200. HARD guts soften negative deltas at low values (first matching band of `[(10,0.4),(20,0.5),(30,0.6),(40,0.7),(50,0.8)]`). `is_cleared` requires `value > 0 && value >= border` (so a survival gauge sitting exactly at 0 border is *not* cleared). Total gauges floor at min 2.0 (never permanently dead); survival gauges hit 0 and stay dead (`update` returns early once `value <= 0`).
  - **clear_lamp**: `NoPlay` if 0 notes; `Failed` if gauge not cleared; else if no break (`counts[3]+[4]+[5]==0`) and `max_combo == total_notes`: `Max` (no GR/GD), `Perfect` (no GD), else `FullCombo`; otherwise falls to the gauge-kind lamp (Hazard maps to `ExHard`).

- **Gotchas / edge cases**:
  - BD asymmetry means a `+250ms` press is an (early) empty POOR while `-250ms` is still a real BAD — symmetric reasoning is wrong.
  - The miss/force-finalize bounds are strict inequalities; tests pin the exact 1µs boundary (`1_280_000` not swept, `1_280_001` swept; `end+200_000` still held, `end+200_001` finalized).
  - LN head reported judge ≠ scored judge: `press` on an LN head returns the head judge but scores nothing (counts/combo/EX unchanged) until `release`. A far-early press on an LN head is an empty poor and must **not** arm a hold.
  - `release` finds the holding note by `position(|n| n.holding)`, not by lane cursor; releasing with nothing held, or on an invalid/empty/out-of-range lane, returns `None`. A consumed note can't be hit twice.
  - `scaled` uses truncating integer division and clamps percent to `>= 1` (never zero-width). MS is intentionally never scaled.
  - `POPN_NOTE`/`POPN_LN_END` currently mirror the SEVENKEY tables (placeholder pending verified PMS values) but are kept as distinct consts so tuning pop'n never touches the beat table.
  - `worse(a,b)` compares enum discriminants — relies on the `Judge` variant ordering being severity order.

- **Tests**: Three inline `#[cfg(test)]` modules. `lib.rs::tests` is the broadest: window classification, press/release/update matching and gating, empty-poor invariants, LN head/release/force-finalize, early/late split + `early+late==counts`, EX/combo/`max_combo`, determinism, `from_model` wiring (notes/LNs/mines, unterminated LongStart, POPN ln_end), and 5 synthetic CN/HCN fixtures (`cn_*`/`hcn_*`/`ln_remains_*`) locking the charge-note two-count behaviour against the unchanged plain-LN single count. `windows.rs::windows_tests` pins exact `judge()` boundaries (PG/GR/GD/BD/MS edges, asymmetry, no-late-POOR, monotonic tier widening), `scaled()` (identity at 100, fixed MS, truncation, clamp-to-1), mode selection, and `rank_to_judgerank` table/clamp/monotonicity. `gauge.rs::gauge_tests` covers init values, TOTAL/LIMIT_INCREMENT modifiers, HARD guts bands, clamping, dead-gauge permanence, and every `clear_lamp` tier/break-detection/kind-mapping path.

---

### rbms-audio
- **Role**: Real-time, allocation-free software audio engine for BMS keysounds: decodes audio files, owns the cpal output stream, and mixes scheduled keysounds with sample-accurate timing. Crucially, it exposes a master clock derived from `samples_played / rate` (not the frame/vsync clock) as the project's authoritative play-position timing source.

- **Public API** (re-exported from `lib.rs`):
  - `AudioError` — enum: `NoDevice`, `Decode(String)`, `Stream(String)`, `Unsupported(String)`; impls `Display` + `std::error::Error`.
  - `DecodedAudio` — struct `{ samples: Vec<f32>, channels: u16, rate: u32 }` holding interleaved f32 PCM at native rate.
  - `decode_bytes(bytes: Vec<u8>, ext: Option<&str>) -> Result<DecodedAudio, AudioError>` — symphonia-based decode of WAV/OGG/FLAC/MP3 from memory.
  - `AudioEngine` — top-level host handle: owns stream, RT command queue, clock, and the keysound `bank`.
    - `new()` (512 max voices) / `with_max_voices(n)` — open default device, start stream.
    - `out_rate()`, `clock_frames()` (frames played), `clock_us()` (clock in µs).
    - `load(id, bytes, ext)` — decode + insert; `insert_decoded(id, DecodedAudio)` — insert pre-decoded sample (skips CPU-heavy decode so a host can decode off-thread in parallel).
    - `loaded()`, `has_sample(id)`, `sample_duration_us(id) -> Option<i64>`.
    - `play(id, gain, pan, pitch, at_us)`, `stop(key)`, `set_master_gain(g)` — push RT commands.
  - `Mixer` — RT-safe mixer: `new(out_rate, out_channels, max_voices)`, `clock_frames()`, `apply(Command)`, `mix(&mut [f32])`.
  - `Command` — enum `Play{ sample, gain, pan, pitch, key, at_frame }`, `Stop{ key }`, `MasterGain(f32)`.
  - `SampleData` — struct `{ pcm: Arc<[f32]>, channels: u16, rate: u32 }` with `frames()`.

- **Key invariants & algorithms**:
  - **Master clock = sample count, not frames/vsync.** `AudioEngine.clock` is an `Arc<AtomicU64>`; the cpal callback stores `mixer.clock_frames()` with `Release` after each block, read with `Acquire`. `clock_us = frames * 1_000_000 / out_rate.max(1)`. This is the explicit "timing-source fix vs the reference implementation."
  - **Time→frame conversion**: `play`'s `at_us` becomes `at_frame = at_us.max(0) * out_rate / 1_000_000`; in the mixer `delay = at_frame.saturating_sub(clock)` — scheduling in the past clamps to play-immediately, never negative.
  - **Per-voice resampling = pitch.** `stride = (sample.rate / out_rate.max(1)) * pitch.max(0.0001)`. One fractional read stride handles both device-rate resampling and pitch shift. `pitch <= 0` is floored to `0.0001` (never zero stride). `out_rate == 0` is treated as 1.
  - **Equal-power pan**: `angle = (pan.clamp(-1,1)+1)*0.5*FRAC_PI_2`; `lgain=cos`, `rgain=sin`, so `lgain²+rgain²==1`. Center gains ≈ `FRAC_1_SQRT_2`.
  - **Linear interpolation with tail clamp**: partner sample is `next = (i+1).min(nframes-1)` — at the tail it clamps to the last sample so the final (and single-frame) source frame still plays. This was a deliberate fix; it previously dropped the last frame and silenced 1-frame samples.
  - **Voice allocation**: fixed pool of `max_voices` (engine default 512); `alloc_slot` round-robins from `alloc_cursor` to find a free slot, otherwise **steals** the slot at the cursor. `play(key)` first `stop(key)`s, so re-triggering the same key cuts the previous voice (only one voice per key).
  - **Output routing**: mono out (`oc==1`) writes `(l+r)*0.5`; stereo writes `l`/`r`. `out_channels==0` is treated as 1. Mono source duplicates to L/R; stereo source routes channels independently.
  - **Master gain** applied last, output hard-clamped to `[-1.0, 1.0]` (negative master inverts phase). Default master gain `1.0`.
  - **RT safety**: `mix` does no allocation/locking; `scratch` buffer is resized only when `data.len()` changes. Commands flow through an `rtrb` SPSC ring of capacity 8192; pushes/pops are best-effort (`let _ =`), so a full queue silently drops commands.
  - `clock` advances by **frames** (`out.len()/oc`), not raw samples, every `mix` call.

- **Gotchas / edge cases**:
  - `decode_bytes` returns `AudioError::Decode("empty audio")` if zero channels or no samples; `ResetRequired` and `UnexpectedEof` end decoding gracefully, `DecodeError` packets are skipped (lossy).
  - `play(id, ...)` on an unknown id is a silent no-op (bank lookup misses); `stop`/`MasterGain` of unknown keys are no-ops.
  - `_stream` is held only to keep the cpal stream alive; dropping `AudioEngine` stops audio.
  - `SampleData::frames()` returns 0 when `channels == 0` (guards divide-by-zero).
  - cpal sample formats: only F32/I16/U16 supported; anything else → `AudioError::Unsupported`. Output samples are converted via `T::from_sample`.
  - Pitch floor literal `0.0001f32` widens to ≈`9.9999997e-5` as f64 — tests compare with tolerance, not equality.

- **Tests**: Inline `#[cfg(test)]` modules in `decode.rs` and `mixer.rs` (no `tests/` dir). `decode.rs` builds in-memory 16-bit PCM WAVs (mono + stereo helpers) and covers: successful decode, channel/rate/sample-count preservation, normalization to `[-1,1]`, silence round-trip, determinism, no-extension-hint probing, and error paths (empty/garbage/truncated/zero-length-data). `mixer.rs` (~40 tests) pins: clock accounting (frames vs samples), sample-accurate delay across mix boundaries, stride resampling/pitch combos and floors, equal-power pan constant-power invariant, mono/stereo channel routing, voice gain + master-gain clamping/phase, voice lifecycle (retrigger-cuts, coexistence, stealing, round-robin cursor, stop), and end-of-sample tail behavior (all frames play, single-frame audible, position persists across calls).

---

### rbms-render
- **Role**: Backend-agnostic 2D rendering for every BMS screen — playfield, HUD, song-select, result — plus text shaping, a data-driven skin/theme system, and a CPU reference canvas. All composers draw through the `Renderer` trait, so the wgpu and CPU backends share identical layout code.
- **Public API**:
  - `Renderer` (trait): `size`/`clear`/`fill_rect` — the only primitive every composer uses. `Color` (rgba u8, with named consts), `Rect`.
  - `CpuCanvas`: deterministic software RGBA8 backend; `new`, `pixel_at`, `pixels`.
  - `Skin` / `SkinConfig`: resolved per-lane geometry+colours vs. its RON config. `Skin::build(cfg, mode, screen_w, screen_h)`, `default_for`, `lane_count`/`lane_height`/`lane_center`/`note_color`.
  - `Theme` / `ThemeConfig` + `set_theme(t)` / `theme()`: UI chrome palette (non-skin screens); RON-loaded, thread-local active theme.
  - `render_playfield`, `render_lane_cover`, `render_key_bomb`: in-play field draw, sudden+ cover, hit explosions.
  - `render_hud` + `HudView`: live gauge/combo/judge/score-graph overlay.
  - `render_select` + `SelectView`/`SelectRow`/`SelectDetail`/`DetailView`/`DensityView`/`RecordsView`/`RecordRowView`/`SelectModal`/`StatCell`/`SelectHot`/`CoverState`; `cover_rect()`. Returns clickable `(Rect, SelectHot)` hot-regions.
  - `render_result` + `ResultView`; `dj_rank`, `draw_rank_bar`, `ex_delta_label`, `RANK_BANDS`.
  - Text: `draw_text`/`draw_text_centered`/`draw_text_right`, `text_width`, `fit_text`, `load_font`, `set_ui_family`/`reset_ui_family`.
- **Key invariants & algorithms**:
  - **Reference space is 1280×720 (16:9)**; most pixel constants (select panel rects, result/modal layout) are hard-coded for it. `SkinConfig` mixes units: `field_x`/`field_width`/`dual_gap` are *fractions of screen width*; `top_y`/`judge_y`/`note_height`/`bga` are *pixels in 720p*.
  - **Theme is a thread-local** `RefCell<Theme>` (`theme.rs`); `set_theme` once at startup, every composer reads `theme()`. Rendering is single-threaded (also true of the font `ENGINE` thread-local). Both `ThemeConfig::parse` and a malformed RON theme fall back to all-defaults so a broken theme can't brick the UI; `resolve` merges set fields over `Theme::default`.
  - **Font engine** (`font.rs`): bundled Inter (OFL); cosmic-text falls back to system fonts for CJK/Thai/Arabic. `scale` is a legacy unit → `px_for(scale)= round(scale*8.5).max(8)`. Two caches: shaped layout (`px→text→Laid`) and run-length-merged rasterized glyph pixels keyed by `(CacheKey, packedRGB)`; `merge_runs` coalesces contiguous same-colour/coverage pixels into one `fill_rect` per run (pixel-identical, fewer quads). Both caches cleared on family change.
  - **Skin lane layout**: `dual` DP (two fields, P1 left/P2 right, turntables on the *outer* edge, ignoring SP `scratch_left`) only when `dual_field && players>=2 && key%players==0`; the right field is sized to clear the BGA column (`right_limit = bga.x − 24`). SP uses `scratch_left` to order scratch first/last. `lift` clamps to `0.0..=0.9` and raises `judge_y` toward `top_y`; `beam_height_frac` clamps to `0..=1`. `lane_bg`/`outline`/`divider` keep their configured alpha (not forced opaque). `bomb_us = bomb_duration_ms * 1000`.
  - **Playfield** (`playfield.rs`): uses `constant_offsets`/`visible_offsets` from `rbms-chart`. A note exactly at its time is *excluded* (its timeline is `cur`; iteration starts at `cur+1`) — it's judged and gone. LN bodies: head on-screen uses its offset; head past line clamps body bottom to the line; end above window extends to `top_y`; whole-LN-above-window or LN-past-line is **skipped** (else it tints the whole lane). `LongEnd` notes are never drawn as a cap (the body bar is the hold). Beams: `BEAM_RELEASE_US=120_000`µs linear fade, `BEAM_SLICES=6` gradient brightest at the line; `beam_on`/`beam_off` are per-lane µs timestamps, `i64::MIN`=inactive, pass `&[]` to omit. `render_lane_cover` clamps to `0.0..=0.9`. Judgment line / decor drawn *per field* so they don't bridge the DP gap.
  - **Result/HUD**: DJ-LEVEL bands sit at **ninths of max EX** (`RANK_BOUNDS`, `F`<2/9 … `AAA`≥8/9); `dj_rank` guards `max_ex==0`→F and `ex>max`→AAA. HUD spans the *full* note area (both DP fields), score-graph drawn in the gap between field and BGA, judge counts pinned right of the field/over the BGA. Gauge colour switches at skin `gauge_clear_threshold`/`gauge_warn_threshold`.
- **Gotchas / edge cases**:
  - `CpuCanvas::fill_rect` **always forces stored alpha to 255**, even for `a==0` (a transparent fill is *not* a true no-op — rgb is preserved but alpha is stamped). Tests therefore compare lanes against a reference empty lane rather than against `skin.lane_bg`.
  - `fill_rect` floors origin / ceils extent, clips to canvas, draws nothing for zero/negative size; a sub-pixel rect still paints ≥1 cell. `pixel_at` does **no** bounds check (panics out of range — pinned by a `#[should_panic]` test).
  - `render_select` suppresses row/record hot-rects while the modal is open (the modal owns clicks). Focused list row is highlighted colour-only (side rails, no box) and does not grow. Detail-panel pixel offsets are tightly tuned (stat grid must clear the DENSITY section); density histogram uses sub-pixel bar `step` so the last bin lands inside the box.
  - `wrap_two` / `fit_text` are character-wrap (CJK has no spaces); a single over-wide glyph collapses to a lone `…`; non-positive width is a no-op.
  - `SkinConfig`/`ThemeConfig` use `#[serde(default)]` so partial RON inherits defaults; `SkinConfig` also carries the **HUD presentation** (judge colours/labels, gauge, bomb, combo/judge/fastslow y-offsets) so a skin restyles the play HUD without code.
- **Tests**: Extensive `#[cfg(test)]` modules in each file. `skin.rs` pins lane layout (SP/DP dual fields, outer scratches, BGA clearance, lift/beam clamps, uniform width, bomb µs). `playfield.rs` pins note/LN positioning and all LN edge cases, beam fade math, lane cover, key bomb, determinism. `font.rs` pins `merge_runs` coalescing, `fit_text`, multilingual shaping. `result.rs` pins `dj_rank` ninth boundaries/monotonicity and result smoke. `theme.rs` pins default/partial/malformed resolve + thread-local round-trip. `cpu.rs` pins the source-over blend math and clipping. No tests shown for `hud.rs`/`select.rs` beyond what their helpers exercise elsewhere.

---

### rbms-ir
- **Role**: Defines the rbms "IR-superset" score-server contract: the `ScoreServer` trait, all wire DTOs (serde JSON), plus two concrete clients (HTTP reference, offline null). It is a pure data/contract crate — no backend, no audio/judge/UI logic lives here.
- **Public API**:
  - `ScoreServer` (trait, `Send + Sync`): the score-server contract; 8 required methods (`health`, `submit_score`, `chart_ranking`, `player_best`, `player_profile`, `rivals`, `submit_course`, `upload_replay`) + 6 superset-extension methods with default `Err(Unsupported)` bodies (`course_ranking`, `download_replay`, `get_settings`, `put_settings`, `register`, `login`).
  - `HttpScoreServer` — REST+JSON reference client over `reqwest::blocking`, optional bearer token.
  - `NullScoreServer` — offline stub; every required method returns `NotConfigured`.
  - `IrError` (enum): `NotConfigured | Network(String) | Server(u16,String) | Decode(String) | Unsupported`; impls `Display` + `std::error::Error`.
  - `API_VERSION: u32 = 1` — contract version stamped into every submission.
  - DTOs (re-exported via `pub use dto::*`): `ChartId{md5,sha256}`, `PlayerId{id}`, `ScoreSubmission`, `ScoreRecord`, `SubmitResponse`, `CourseSubmission`, `PlayerProfile`, `ServerInfo`/`ServerCapabilities`, `JudgeBreakdown`, `PlayOptions`, `ReplayData`/`ReplayEvent`, `SettingsBlob`, `AuthRequest`/`AuthResponse`.
  - Enums: `ClearLamp` (11 variants `NoPlay..Max`), `GaugeType` (9 variants), `RandomOption` (9 variants) — all externally-tagged (serialize to the bare variant-name string).
- **Key invariants & algorithms**:
  - **Superset, not subset**: charts carry both `md5` and `sha256` (BMS IRs key only on MD5); the HTTP client routes ranking/best/replay endpoints by `chart.md5` only (`/charts/{md5}/...`), while `sha256` rides along in the body.
  - **Early/late judge split is additive**: `JudgeBreakdown` mirrors the reference implementation `IRScoreData`'s 12 `epg..lms` fields; the documented invariant is `epg+lpg == pgreat`, `egr+lgr == great`, … `ems+lms == miss`. Plus rbms-only `avgjudge` (mean signed timing in **µs**, `i64`) and `empty_poor`.
  - **Forward-compat via serde defaults**: the 9 basic judge fields (`pgreat..combobreak`) and 7 basic `PlayOptions` fields (`gauge,random,random_p2,scratch_auto,lntype,input_device,assist`) are **required**; everything else (`#[serde(default)]`) — including `seed`, `judge_algorithm`, `rule`, `skin`, `client_build_sha256`, `client_platform`, `extra`, and all extended `PlayOptions` — defaults so pre-superset payloads still decode. `extra: HashMap<String,Value>` is a plain field (NOT `#[serde(flatten)]`), so unknown sibling keys are **ignored, not captured**.
  - **Replay format is lossless µs events**, not a byte stream: `ReplayEvent{t_us:i64, lane:u32, press:bool}`, format string `"rbms-us-v1"`; `t_us` is `i64` so pre-song-start (negative) times survive; `seed: Option<u64>` lets the server reproduce the exact shuffle/lane layout (paired with `ScoreSubmission.seed`).
  - **Capability discovery**: `health()` returns `ServerInfo{name,version,ir_compat,capabilities}`; `ServerCapabilities` (all-`bool`, defaults all-false) is how a client learns which superset endpoints exist before calling them.
  - **HTTP client specifics** (`http.rs`): fixed **5-second** request timeout (`reqwest` builder; falls back to `Client::default()` if the builder fails); base URL has trailing `/` stripped at construction; non-2xx → typed `IrError` variants (401 Unauthorized · 403 Forbidden · 404 NotFound · 409 Conflict/SettingsConflict · 413 PayloadTooLarge · 429 RateLimited · else `Server(code, body)`); network failure → `Network`; JSON decode failure → `Decode`. `put_settings` expects 200 `{ updated_at }` (the stored lock stamp, epoch ms) and treats a body-less 204 from older servers as success with the sent stamp (`SettingsPutResult { from_server: false }`). `upload_replay` POSTs and unwraps a local `{ "id": String }` response into a bare `String`.
  - Endpoint map: `/health`, `POST /scores`, `/charts/{md5}/ranking?limit=`, `/charts/{md5}/best?player=`, `/players/{id}`, `/players/{id}/rivals`, `POST /courses`, `POST /charts/{md5}/replays`, `/courses/{hash}/ranking?limit=`, `/replays/{id}`, `GET|PUT /players/{id}/settings/{name}`, `POST /auth/register`, `POST /auth/login`.
- **Gotchas / edge cases**:
  - **`NotConfigured` vs `Unsupported` are deliberately distinct**: `NullScoreServer` returns `NotConfigured` for the 8 required methods but inherits the trait-default `Unsupported` for the 6 superset methods (it does NOT override them). Tests pin this difference — don't "fix" it by making them uniform.
  - **`gauge_value` is a bare `f32` (and `f64` math fields like `hispeed` likewise non-optional)**: a NaN `gauge_value` serializes to JSON `null` and then **fails to decode** — silently lossy. A test (`..._nan_serializes_to_null_and_decodes_back_nan`) pins this current behavior; it is flagged "suspect", not endorsed. Avoid feeding NaN.
  - `played_at` and `t_us` are `i64` (signed) on purpose — negative values round-trip.
  - Enum decode is **case-sensitive** and rejects unknown variants (`"normal"` and `"Bogus"` both error) — adding a lamp/gauge/random value is a wire-breaking change for older decoders.
  - `option: i64` in `PlayOptions` is the reference implementation's raw option bitmask, kept verbatim for round-tripping (don't reinterpret).
  - `HttpScoreServer` is **blocking** (`reqwest::blocking`) — callers must not invoke it on an async/UI thread without offloading.
  - URL path segments (player id, chart md5, settings name, course hash) are interpolated **without escaping** — assumed safe/hash-like; not URL-encoded.
- **Tests**: Inline `#[cfg(test)]` modules in each file. `lib.rs` covers `ScoreSubmission` round-trips (full superset + minimal/legacy decode), `ReplayData`, the early+late additive split, `IrError::Display`, `API_VERSION`, and the NaN-`gauge_value` edge case. `dto.rs` exhaustively round-trips every enum variant (asserting exact JSON strings), serde defaults/required-field failures for each DTO, µs-resolution replay deltas, and `extra`-map forward-compat (unknown siblings ignored). `null.rs` asserts all 8 required methods → `NotConfigured`, all 6 superset defaults → `Unsupported`, that the two error variants differ, and that `NullScoreServer` is `Send + Sync` and usable as `Box<dyn ScoreServer>`. No `http.rs` tests (no live-server/mock harness).

---

### rbms-table
- **Role**: Fetches, parses, caches, and groups BMS difficulty tables (a `header.json` + body `data.json` pair, or a bare body) into level-ordered chart lists. Pure data layer — no UI, no audio.
- **Public API**:
  - `DifficultyTable` (struct) — resolved table: `name`, `symbol`, `level_order: Vec<String>`, `entries: Vec<TableEntry>`.
  - `DifficultyTable::from_parts(header: Option<TableHeader>, entries) -> DifficultyTable` — merge header metadata + body; derives `level_order` when missing/empty.
  - `DifficultyTable::from_body_bytes(bytes, header) -> Result<_, String>` — parse a JSON-array body with optional header.
  - `DifficultyTable::fetch(url) -> Result<_, String>` — HTTP fetch; auto-detects whether `url` is a header object or a body array; for a bare body, best-effort fetches sibling `header.json`.
  - `DifficultyTable::fetch_or_cache(url, cache_path) -> Result<_, String>` — fetch + persist resolved body to cache; on fetch failure, reload from cache for offline use.
  - `DifficultyTable::by_level() -> Vec<(String, Vec<usize>)>` — entries grouped by level (values are indices into `entries`), in `level_order`, empties dropped, unknown levels appended first-seen.
  - `TableEntry` (re-export from `dto`) — one chart; only `md5` is required, all other string fields `#[serde(default)]`.
  - `TableHeader` (re-export from `dto`) — header metadata: all four fields `Option`, all `#[serde(default)]`.
  - Internal (not exported): `CachedTable` (on-disk envelope), `derive_level_order`, `level_key`, `join_url`, `build_client`, `get_bytes`, `sibling_header`.
- **Key invariants & algorithms**:
  - **Level ordering** — `level_key(s)` returns `(u8, i64, String)`: numeric levels parse via `i64::parse` → tag `0`, sort ascending by value; everything else → tag `1`, sort by full string (Unicode scalar order, so uppercase before lowercase). So numerics come first ("9" < "10", negatives before positives) and non-numerics ("???", "beginner") sort last by string. Overflow beyond i64, leading space, underscores, `0x` hex, `5_000`, and empty string all fall into the non-numeric bucket; `+5`, `05`, `-0` parse as numeric.
  - **Derivation** — `derive_level_order` sorts then dedups *by `level_key`*, so numerically-equal textual forms ("5"/"05") collapse to **one** entry, keeping the **first-seen** textual form (stable sort). `level_order.len() <= entries.len()`.
  - **Defaults** — missing `symbol` → `"*"`; an *explicit empty string* symbol is preserved (not replaced). Missing `name` → `""`. `level_order` that is `None` **or** an explicit empty `Vec` triggers derivation; a non-empty explicit `level_order` is used **verbatim, not re-sorted**.
  - **Grouping** (`by_level`) — builds a `pos` HashMap from `level_order`; if `level_order` contains duplicate keys, `pos` collapses to the **last** index, but listed-yet-empty groups are dropped so only one survives. Total entry count is preserved (each entry appears exactly once); indices are valid and unique.
  - **URL joining** — `join_url` uses reqwest's re-exported `url::Url` (RFC 3986): handles relative, root-relative `/abs`, protocol-relative `//host`, absolute, `..` (clamped at host), query-only `?v=2`, empty rel (returns base sans fragment). Unparseable base → returns raw `rel`.
  - **Caching** — `fetch_or_cache` only writes the cache when `entries` is non-empty. Cache envelope stores resolved name/symbol/level_order so offline reload preserves them; on reload, an empty cached `level_order` is re-derived.
  - **HTTP client** — `build_client`: 15-second timeout, `user_agent("rbms-table")`, blocking reqwest. Non-2xx → `Err("HTTP {status} for {url}")`.
- **Gotchas / edge cases**:
  - `TableEntry` derives `Serialize` + `Deserialize`; `TableHeader` derives **only `Deserialize`** (no `Serialize`). `DifficultyTable` has **no `Debug` impl** — tests can't `unwrap_err()`, they `match` on the Result instead (`body_err` helper).
  - `fetch` shape-detection tries `Vec<TableEntry>` first; a header object that *also* happens to deserialize as an array would be misread (not a real case but worth knowing).
  - Body must be a JSON **array**; an object, malformed JSON, empty input, or an entry missing `md5` all → `Err` prefixed `"body parse:"`. Unknown fields (`sha256`, `extra`) are ignored.
  - An entry with only `md5` gets empty defaults for every other field; an empty `level` still groups under the `""` level.
  - Error strings are load-bearing and tested: `"body parse:"`, `"header parse:"`, `"header.json has no data_url"`, and the combined cache-fallback message.
- **Tests**: All in `crates/rbms-table/src/lib.rs` under `#[cfg(test)] mod tests` (no test in `dto.rs`). They cover `level_key` (numeric/non-numeric/overflow/negatives/leading-zero/whitespace/case ordering), `derive_level_order` (dedup, first-seen textual form, numeric-before-alpha, length bound), `by_level` (count preservation, unique/in-range indices, index-to-level match, level_order ordering + first-seen extras, dropping empty listed levels, duplicate-key behavior), `from_parts`/`from_body_bytes` (defaults, empty-symbol preservation, verbatim level_order, serde defaults/unknown-field tolerance, malformed/empty/missing-md5 errors, serde roundtrip + determinism), header serde, and `join_url` (relative/root-relative/absolute/dotdot/protocol-relative/query-only/empty-rel/unparseable-base). Network paths (`fetch`, `fetch_or_cache`, `build_client`, `get_bytes`, `sibling_header`) are **not** covered by tests.

---

### rbms-play
- **Role**: Pure logic crate that drives BMS playback over a song clock: it advances notes against the chart, emits keysound `PlayEvent`s, and feeds judgments into `rbms-judge` (including an autoplay mode used for chart verification). It owns no audio/render/IO — callers supply the `now_us` clock and a sink closure, and read back per-lane render state (beams, bombs).

- **Public API**:
  - `struct PlayEvent { wav: i32, at_us: i64 }` — a keysound (BGM or a hit note's wav) that should fire at `at_us`.
  - `struct Player` — the real-time play driver; field `pub judge: JudgeEngine` is exposed directly.
  - `Player::new(model: Model, autoplay: bool)` — build a player; precomputes time-sorted BGM, note-head, and autoplay-action lists.
  - `Player::update(now_us, play: FnMut(PlayEvent))` — advance to `now_us`: flush due BGM/autoplay keysounds, feed autoplay/auto-lane judgments, age out tap beams, then call `judge.update(now_us)` (the MISS/LN-finalisation sweep).
  - `Player::press(lane, now_us, play) -> Option<JudgeResult>` / `release(lane, now_us) -> Option<JudgeResult>` — interactive input; updates beam/bomb and emits the nearest head's keysound on press.
  - `Player::set_judge_rate(rate_percent)` — user JUDGE WIDTH multiplier; rescales both note and LN-end windows.
  - `Player::set_auto_lanes(Vec<bool>)` — mark lanes (e.g. auto-scratch) as chart-driven even in interactive mode.
  - `Player::set_gauge(GaugeKind)`, `into_judge() -> JudgeEngine`, `model() -> &Model`, `last_time_us() -> i64`.
  - `Player::beam_on() -> &[i64]`, `beam_off() -> &[i64]`, `bomb() -> &[(i64, u8)]` — per-lane render state for the skin.
  - `fn simulate_autoplay(model: &Model) -> JudgeEngine` — full autoplay pass; verification helper (should yield all-PGREAT, LNs once).

- **Key invariants & algorithms**:
  - `AUTO_BEAM_US = 80_000` (80 ms): in autoplay/auto-lane, a tapped (non-LN) note lights its lane beam for 80 ms then auto-releases, mirroring the reference implementation's `auto_minduration`. LN-held beams stay lit (gated by `ln_active[lane]`) until the LN release.
  - Window selection (`windows_for`): note windows are mode-aware (`JudgeWindows::note_for_mode(&model.mode)`) and then scaled by the chart's `#RANK` via `rank_to_judgerank(meta.rank)`. `set_judge_rate` recomputes the effective judgerank as `base * rate.max(1) / 100` (min 1) and applies it to BOTH note windows and the separate LN-end windows (`ln_end_for_mode`). Recently changed: the LN-end window now follows `#RANK` instead of a fixed 100% constant.
  - Autoplay actions (`collect_actions`) are built per-lane: a `Press` for every Normal and LN head, a `Release` for every LN end; an LN head with no matching `LongEnd` is dropped (no press, no sound, no judge). All event lists are sorted by time so keysounds emit in non-decreasing time order.
  - Bombs fire only for judge index ≤ 3 (PG/GR/GD/BD) — never empty POOR (index 4), MISS, or `update()`'s sweep, which never touches the bomb array. `bomb[lane] = (hit_us, judge_index)`.
  - `press` emits `nearest_head_wav`: the lane-scoped note head (Normal or LN start) minimising `|t - now_us|`; lanes with no head emit nothing. Beam-on writes are bounds-guarded so out-of-range lanes (e.g. `press(99, …)`) just no-op/return None.
  - `simulate_autoplay`/`Player` clone the `Model`; `update` is idempotent past full judgment (the sweep won't re-judge).

- **Gotchas / edge cases**:
  - `set_auto_lanes` is a no-op unless the vector length exactly matches `model.mode.key`; a wrong-length vector is silently rejected and those lanes stay interactive (pinned by tests).
  - Input on an auto lane is fully ignored: `press`/`release` return `None` and do not light that lane's beam.
  - `release()` only stamps `beam_off` if `beam_on` was lit; an unpaired release leaves `beam_off` at `i64::MIN`. An empty press (far from any note) still lights the beam but no bomb.
  - The MS candidate-gate is fixed and does NOT widen with JUDGE WIDTH: the far-early empty-POOR edge (`ms.1 = +500ms`) stays put, so a press 600 ms early returns `None` even at 200%. Conversely widening reaches BAD/PGREAT edges that are otherwise unreachable.
  - A held LN that is never released is finalised only by `update()` past `end + LN_MARGIN` (~200 ms, owned by `rbms-judge`); a head PGREAT can be dragged to a worse final judge.
  - This crate has NO `main.rs`/binary, audio clock, theme thread-local, or library/search/sort — those live elsewhere; here the clock and audio are entirely the caller's responsibility.

- **Tests**: A large `#[cfg(test)] mod tests` at the bottom of `/Users/gkn/R-BMS/crates/rbms-play/src/lib.rs` (helpers `model()` via `to_model(parse(bms), Mode::BEAT_7K)`, `autoplay_collect`). Coverage: autoplay perfect-score/EX=2n/full-combo/no-miss invariants over dense charts, mines excluded, LN counted once, dangling-LongStart safety; keysound count/time-ordering; `auto_lanes` length guard and input-drop behaviour; interactive beam/bomb transitions; `nearest_head_wav` lane-scoping and LN-head lookup; `set_judge_rate` widening/clamping (rate 0 → min, fixed-MS-window non-widening); `update()` sweep MISS/LN finalisation, idempotency, monotonic autoplay combo; and rank-scaled LN-end windows. Runnable example: `examples/autoplay_score.rs` (`simulate_autoplay` on a real chart file).

---

### apps/rbms-player
- **Role**: The native winit + wgpu front-end binary for the rbms BMS player: a stage-machine app (Select / Settings / KeyConfig / Tables / Folders / Loading / Play / Result) wiring the parser/chart/judge/play/audio/render crates together. `main.rs` owns the `App` struct and event loop; the heavy method bodies live in three `app_*` sibling modules.
- **Public API** (crate-internal; the binary exports nothing for downstream crates):
  - `App` — the single `ApplicationHandler`; holds all stage/play/select/config state. Methods are split across `app_play.rs`, `app_select.rs`, `app_input.rs`.
  - `Gpu` (`gpu.rs`) — instanced-quad wgpu backend implementing `rbms_render::Renderer`; `fill_rect` = one GPU instance, the whole frame is one instanced draw, plus a separate single-texture BGA/cover pipeline (`set_bga`/`clear_bga`).
  - `FolderList` (`folders.rs`) — RON-persisted song-library folder list (`folders.ron`); the library is the union of all folders.
  - `PlaySettings` (`settings.rs`) — RON-persisted play options (`settings.ron`), enums stored as strings, `#[serde(default)]` for forward-compat.
  - `KeyConfig` / `ControlBinds` / `ControlAction` + `key_from_name`/`key_name`/`default_keys_for_mode`/`mode_config_key` (`keyconfig.rs`) — per-mode lane bindings + in-play control keys (`keyconfig.ron`).
  - `format.rs` helpers — `clear_type_id`/`clear_type_from_id`, `clear_label_color`, `difficulty_name`/`difficulty_color`, `rank_label`, `mode_short`/`mode_color`, `gauge_name`, `fmt_datetime`, `fmt_duration`.
  - Free fns in `main.rs`: `default_total`, `calibrated_offset`, `compute_build_hash`, `client_platform`, `bundled_skin`, `load_theme`, `scan_folder`/`scan_folders`, `compute_chart_detail`, `resolve_keysound`/`resolve_file`, `decode_bga_256`, `build_server` (builds the `ScoreServer` from `config` — `HttpScoreServer` for a non-empty URL, else `NullScoreServer`), `config_dir`/`config_dir_from`.
  - `ir_map.rs` mapping fns (`gauge_from_name`/`gauge_token`, `ir_clear`, `ir_gauge`, `ir_random`, `ir_lntype`) — translate engine enums to IR DTOs. `ir_lntype(lnmode)` maps the chart's `#LNMODE` to the backend `PlayOptions.lntype` encoding (0=LN, 1=CN, 2=HCN).
  - Enums/types: `Stage`, `Loading` (Song/Scan), `SelectView` (Root/AllSongs/TableLevels/TableLevel), `SelectItem`, `SortMode`, `Hot`, `KcRow`, `SongEntry`, `ChartDetail`, `ScanOutcome`, `SelectKey`.
- **Key invariants & algorithms**:
  - **main.rs module split**: `main.rs` defines `App` + the winit `window_event` dispatcher; `app_play.rs` = chart load / per-frame loop / song clock / result+submission; `app_select.rs` = library, focused detail, preview, record modal, tables, folders, loading transitions; `app_input.rs` = lane/control mapping, settings/skin/font round-trip, replay playback+analysis, key-config editor. Each `app_*.rs` does `use crate::*;` with `#![allow(clippy::wildcard_imports)]`.
  - **Fixed logical space**: UI is laid out in 1280×720 (`CW`/`CH`); the surface stretches it; cursor coords are mapped back into that space. Default `MODE = BEAT_7K`.
  - **Audio sample clock**: song time = `audio.clock_us() - anchor_us` (sample-accurate), falling back to wall-clock `Instant` only when audio is unavailable. `anchor_us` is captured in `start_play()` *after* keysounds finish decoding, so load time doesn't count against song position. Keysound events are scheduled at `e.at_us + anchor`.
  - **Keysound loading is multithreaded**: decode jobs fan out over `min(cores,8)` threads, stream back via mpsc + an `AtomicUsize` progress counter; `frame()` drains into the bank and draws a determinate bar; `start_play()` fires only once `ks_progress == ks_total` (with a final drain after the channel disconnects). Qualia-scale (650+) charts motivated this.
  - **Mode-aware judging**: judge windows/rate come from the `rbms_judge`/`Player`; `judge_rate` clamped 50–200%, `offset_ms` clamped ±200. Press judging happens at `raw + offset_us`; raw (un-offset) times are what gets recorded into replays.
  - **Auto-calibration** (`calibrated_offset`): offset held constant within a run (consistent judging + reproducible replays); accumulates mean timing error of accurate hits (judge ≤ GD, |delta| ≤ 150 ms) and recenters for the *next* run, requiring `cal_count >= 20`. `mean_us > 0` = early/FAST ⇒ offset increases. Converges in ~one run.
  - **default_total**: the reference implementation-style gauge TOTAL fallback when `#TOTAL` absent: `(7.605·n / (0.01·n + 6.5)).max(260.0)`, floor 260, `notes.max(1)`, non-decreasing in note count.
  - **Theme is a render-crate thread-local/global**: `load_theme()` writes a commented `theme.ron` template on first run, parses → `rbms_render::set_theme(...)`. The template values equal the built-in defaults (pinned by a test). Note `theme.ron` is the UI chrome; the in-play note field is themed by the *skin*, not the theme.
  - **Multi-folder library**: `scan_folders` unions every folder in `FolderList`; a directory launch arg auto-joins the persisted list. Rescans run off-thread (`rescan_all_folders` → `scan_rx`) with an indeterminate ping-pong loading bar (no total to count); a chart-load `Loading::Song` blocks but presents a LOADING frame first.
  - **Search/sort**: `/` opens search (forces `SelectView::AllSongs`, flat list); filter is case-insensitive substring over title/artist/subtitle; F3 cycles `SortMode` (Default/Title/Artist/Level/Clear, Level parses as i64 with `i64::MAX` fallback, Clear sorts best-clear-first). Both are applied only in `arrange_songs`.
  - **Select scene caching**: `SelectKey = (select_gen, sel, record_modal, scores.len(), score_graph)`; the `SelectScene` is rebuilt only on key change (continuous redraw loop would otherwise re-walk the whole list). `select_gen` bumps on every `rebuild_select_items` to catch same-length swaps.
  - **Focused detail / cover / preview are lazy + debounced**: heavy `compute_chart_detail` (full `to_model`) + cover decode run only when focus moves to a different song; `#PREVIEW` playback waits `PREVIEW_DEBOUNCE_FRAMES` (20) of settling, uses reserved sample id `PREVIEW_ID = 0`, gain `PREVIEW_GAIN = 0.85`, and loops by re-triggering at clip boundaries on a *separate* `AudioEngine` (torn down before Play so two cpal streams never coexist).
  - **Tables-as-folders**: difficulty tables browse like the reference implementation custom folders (Root → table → level → charts), indices into `table_levels`.
  - **Replay analysis (F4 feature)**: starts following the real audio clock at 1× with sound; once the user pauses/seeks/changes rate (`analysis_manual`) it switches to the virtual `analysis_us` clock and mutes keysounds. Seek re-simulates inputs from the start (`seek_replay`) so judging stays exact; rate clamped 0.25–4.0, seek is ±2 s.
  - **Score id mapping**: `clear_type_id` uses the reference implementation `ClearType.java` values and deliberately **skips id 3** (LightAssistEasy) — `clear_type_from_id(3)` folds to `AssistEasy`, so the round-trip is asymmetric. IDs are strictly monotonic so "best clear" = max-over-ids.
  - **Build integrity**: `compute_build_hash()` SHA-256s the running exe once at startup, submitted as `client_build_sha256` (None if unreadable); `client_platform()` = `OS-ARCH`.
  - **IR lntype is derived, not hardcoded**: `ir_lntype(src.headers.lnmode)` is computed at chart `load()` and stored in `App.chart_lntype`, then submitted as `PlayOptions.lntype`. Previously hardcoded to `1`; now correctly carries 0=LN / 1=CN / 2=HCN matching the backend data-model.
  - **NETWORK settings tab**: `SETTING_TABS` has a `NETWORK` tab (rows 22 `SERVER URL`, 23 `PLAYER ID`) with in-place text editing via the shared `text_input` field (Enter commits, Esc cancels, live buffer shown). `PlaySettings` persists `server_url: Option<String>` + `player_id: String` to `settings.ron`. On commit it saves and rebuilds the `ScoreServer` via `rebuild_server()` (which calls `build_server(&config)`): empty URL = offline (`NullScoreServer`), empty id = `"guest"`. These IR values were previously CLI-only (`--server`/`--player`); now reachable from the GUI.
  - **Windows config path**: `config_dir()` resolves `HOME` → `USERPROFILE` → `"."` (`config_dir_from`, unit-tested). `HOME` is unset on Windows, so the old `var_os("HOME")`-only logic dropped config into the cwd; now it lands in `%USERPROFILE%\.config\rbms`. `settings_path` + keyconfig use it; scores/tables/folders/theme/replays derive from `settings_path.parent()`. Status: `docs/reference/windows-compat.md`.
- **Gotchas / edge cases**:
  - Config files (settings/keyconfig/folders) back up a corrupt file to `*.ron.bak` (renamed aside) before falling back to defaults, so a later save can't silently clobber a recoverable file. `settings`/`keyconfig` *write defaults out* on a missing file; `folders` does **not** create a file on load.
  - `lane_keys()` guarantees every lane `0..mode.key` is bound: empty/missing/bad tokens backfill from the default (bad ones warn). Out-of-range trailing row entries are dropped. Control keys are resolved **before** lanes in play, so a shared key silently shadows the lane — the key-config editor flags collisions (`collisions`/`binding_collides`) and refuses a colliding rebind.
  - `gauge_token`/`gauge_from_name` round-trip gauges by *token* for persistence; `GAUGE_CYCLE` (6 kinds) is the settings cycle order.
  - Pressing Esc in Play goes to Result if every note is already resolved, otherwise quits; a non-analysis run also auto-advances to Result 2 s after the last note. `SongEntry.preview` is read by `start_preview` (the stale `#[allow(dead_code)]` is gone); focus preview is wired (not a follow-up), with `config.debug`-gated `eprintln` instrumentation on all six early-return branches of `start_preview`. `samples/preview-demo/` is a manual fixture; a one-time audible device check is still pending (`docs/bug/2026-06-03-preview-playback.md`).
  - Settings indices are positional: `SETTING_TABS` maps tab → global indices into `setting_line`/`adjust_setting`; `SETTING_KEYCONFIG = 11`, `SETTING_FONT = 18` are special-cased. Editing those constants without updating the tab table breaks the screen.
  - Replay launch forces `autoplay = false` (the recorded stream drives judging; autoplaying too would double-hit). MD5 mismatch between replay and chart only warns. Autoplay/replay runs do **not** persist a local `ScoreRecord` or auto-save a replay.
  - `handle_click` hit-tests `hot` regions topmost-first (last-drawn wins); a click that misses everything dismisses an open modal. `hot` is rebuilt every frame (immediate-mode).
  - Result deltas (`prev_ex`, `prev_best_ex`) are computed from history *before* this run's record is pushed.
- **Tests**: Unit tests are inline `#[cfg(test)]` modules per file. `main.rs` tests: theme template == defaults + valid RON, `fmt_datetime`, clear-lamp id round-trip, build-hash shape, `client_platform`, `calibrated_offset` (recentre/clamp/round/one-run convergence), `default_total` (floor/monotonic), `SortMode` cycle/labels, bundled skins parse, `config_dir_from` (HOME → USERPROFILE → cwd precedence). `ir_map.rs`: `ir_lntype` lnmode→backend encoding (0=LN/1=CN/2=HCN, unknown→LN), gauge token round-trip. `format.rs`: extensive `fmt_datetime` (epoch/negative/leap/boundary/16-char), clear-type id round-trip + the reference implementation values + monotonicity + legacy-id-3 fold, difficulty/rank/duration/mode helpers. `keyconfig.rs`: key token round-trip + aliases, per-mode lane completeness/backfill/short-row/override/ascending-order, collisions, `set_lane` growth, full RON round-trip, load missing/malformed. `settings.rs` & `folders.rs`: default sanity, full + partial RON round-trip (incl. `server_url`/`player_id`), load missing (writes defaults for settings; no-write for folders) / malformed (backs up to `.bak`). `gpu.rs` has no tests (GPU-bound).
