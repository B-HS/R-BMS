# 2026-09-09 — Phase C: strip `//` comments from source

## What happened

Mechanical sweep across `crates/**/*.rs` and `apps/**/*.rs` (no `tools/` directory exists
in this repo): every plain `//` line comment and trailing `// ...` comment was removed,
leaving only `///` / `//!` doc comments in place. `/* */` block comments were checked for
separately; only one occurrence existed and it was a `//!` doc line, not a block comment,
so no block-comment removal was needed.

A Python scanner (string/char/raw-string literal aware, not a naive regex) walked each
file's token stream so that `//` sequences inside string literals (URLs like
`"https://..."`, and a RON theme template embedded as a Rust string that itself contains
`//`-style RON comments) were correctly left untouched.

## Non-obvious rationale preserved from removed comments

The vast majority of the ~790 removed lines were self-evident from the surrounding code
(section-divider dashes, or restating an assertion already visible in the test). The
following carried information not otherwise recoverable from the code and are recorded
here instead of staying inline:

- `crates/rbms-model/src/lib.rs:195` — division by zero in the measure-length calculation
  yields `+inf` rather than an error or guard; flagged as suspect behavior, not fixed here.
- `crates/rbms-model/src/lib.rs:202` — a negative BPM produces a negative measure length;
  this path is not guarded against.
- `crates/rbms-render/src/cpu.rs:207` — an alpha value of `a=0` in the blit path is NOT a
  true no-op: the alpha channel is still forced to `255` while the RGB channels are
  preserved from the source.
- `crates/rbms-render/src/playfield.rs:413-414` — at `microtime == note_t`, the note's own
  timeline position is `cur`, but `visible_offsets` iterates starting from `cur + 1`; the
  practical effect is that a note is not drawn on the exact frame it reaches the judgment
  line (it has already been judged and removed by then).

## Verification

- `cargo fmt --all`
- `cargo build --workspace --all-targets`
- `cargo test --workspace`

Grep check `grep -rnE '^\s*//[^/!]' crates apps --include='*.rs'` drops from 693 to 2; the
2 remaining hits are RON-comment syntax (`//`) inside the `THEME_TEMPLATE` string literal
in `apps/rbms-player/src/assets.rs`, which is data, not a Rust comment, and is intentionally
left as-is.
