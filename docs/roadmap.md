# Roadmap / deferred work

Tracked items the team chose to defer. Ordered roughly by priority.

## Deferred features (do later)

### 1. NETWORK settings tab  *(done — 2026-06-03)*
~~The IR/online values are CLI-only~~. **Implemented**: `SETTING_TABS` tab `"NETWORK"` with rows SERVER URL / PLAYER ID (indices 22/23), editing via the shared `text_input` field (Enter commits, Esc cancels, in-place pre-fill, buffer shown live). On commit: set `config.server_url`/`config.player_id`, persist to `settings.ron` (new `PlaySettings` fields, round-trip tested), rebuild the `ScoreServer` via the extracted `build_server` helper. Empty URL = offline (Null), empty ID = `guest`.

### 3. Settings screen mouse/button UX  *(deferred — document only)*
The settings rows/tabs are already click-hittable (`Hot::SettingTab` / `Hot::SettingRow`), but value changes are keyboard-only (←/→). Plan: render explicit ◀ ▶ stepper buttons and an inline toggle switch per row, and a "DONE/BACK" button — mirroring the new bottom-bar buttons on the select screen.

### Theme coverage completion
The theme system (`docs/theme.md`) covers the main screens. Any remaining hardcoded **chrome** colour is a one-line follow-up (add a `Theme` field + route one `Color::rgb(..)` through `theme()`). Semantic colours (lamps, mode/difficulty badges, dj-rank) are intentionally fixed.

## Known edge-case behaviours (pinned by tests, NOT bugs to fix)

These were surfaced while hardening the test suite. They are intentional or harmless edge cases; tests assert the *actual* behaviour so any future change is caught.

- **`measure_us(bpm)` is unguarded** for `bpm <= 0` (→ `+inf` / negative). Safe in practice: `rbms_chart::assign_times` clamps `bpm <= 0` to a fallback before any timing math, so charts never reach `measure_us` with a bad BPM.
- **Parser control-flow on malformed charts**: a stray `#IF 0` with no enclosing `#RANDOM` is *active* (the implicit current-random is 0); `#RANDOM 0` yields value 1 (so `#IF 1` always fires); an unbalanced `#IF` without `#ENDIF` suppresses the rest of the file; `#ENDRANDOM` silently closes dangling `#IF`s. These match a lenient BMS parser; well-formed charts are unaffected.
- **`CpuCanvas::fill_rect` with `a == 0`** leaves RGB unchanged but still forces the stored alpha to 255 (the canvas is always opaque). No visible effect.
- **A note exactly at `microtime`** is not in `visible_offsets`/`constant_offsets` (they iterate from the *next* timeline) — the note disappears the instant it reaches the judgment line, which is also when it is hit. By design.
- **IR `gauge_value: f32` NaN** serializes to JSON `null` and won't decode back. The gauge value is always finite in practice (clamped), so this can't occur from real play.
- **`clear_type_id`/`clear_type_from_id` are asymmetric at id 3** (beatoraja `LightAssistEasy` folds to `AssistEasy`) — intentional; rbms has no separate light-assist lamp.

## Fixed while hardening tests (done)

- Mixer dropped the final source frame (1-frame samples were silent) → now clamps interpolation to the last sample so every frame plays.
- `mode_color` had no case for BEAT_10K (key 12) → rendered grey while `mode_short` said "10K". Now has a dedicated colour.
- `tables.ron` / `folders.ron` did not back up a corrupt file before defaulting (unlike keyconfig/settings/scores) → now write `.ron.bak` first, so the next save can't clobber a recoverable file.
