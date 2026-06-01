# UI Theme (`theme.ron`)

rbms separates two kinds of look-and-feel:

| Concern | Owned by | File |
|---|---|---|
| **In-play note field** (lanes, notes, beams, gauge, key-bomb, HUD layout) | `SkinConfig` | `assets/skins/*.ron`, or `--skin <file>` |
| **UI chrome** (song-select, result, settings/keyconfig/tables/folders/loading menus) | `ThemeConfig` → `Theme` | `~/.config/rbms/theme.ron` |

This document covers the **theme** (the chrome). It is the schema a future web front-end will author against.

## How it works

- `rbms_render::theme::Theme` is the resolved palette — a flat struct of `Color`s grouped by role.
- `rbms_render::theme::ThemeConfig` is its RON form: **every field is an optional `(r, g, b)` triple**. Missing fields inherit `Theme::default()`, so a theme file may set only the colours it cares about.
- At startup the app reads `~/.config/rbms/theme.ron` (writing an editable, fully-populated template there on first run), calls `ThemeConfig::parse(..).resolve()`, and installs it with `rbms_render::set_theme(theme)`.
- Every renderer reads the active palette via `rbms_render::theme()` (a cheap `Copy`), so changing the file and relaunching restyles the whole UI. A missing or broken file falls back to the built-in defaults — a bad theme can never brick the UI.

## Fields

All values are `Some((r, g, b))`, 0–255 per channel. Omit a line to keep its default.

| Field | Default | Role |
|---|---|---|
| `bg` | `(8, 8, 14)` | window background |
| `topbar` | `(18, 18, 30)` | top header bar |
| `panel` | `(14, 16, 26)` | panel fill (detail panel, list base) |
| `panel_hi` | `(22, 24, 36)` | raised panel / button fill |
| `divider` | `(40, 42, 58)` | hairline dividers / outlines |
| `text` | `(235, 235, 235)` | primary text |
| `text_dim` | `(170, 170, 185)` | secondary text |
| `text_muted` | `(120, 124, 140)` | muted hint text |
| `accent` | `(120, 205, 235)` | accents, counts, sort label |
| `focus` | `(120, 200, 240)` | focus ring / selection rails |
| `title_focus` | `(244, 236, 156)` | focused song-row title |
| `title_dim` | `(190, 186, 150)` | unfocused song-row title |
| `row_even` | `(22, 24, 34)` | even list row |
| `row_odd` | `(27, 29, 42)` | odd list row |
| `row_folder` | `(40, 44, 30)` | folder list row |
| `row_focus` | `(50, 56, 82)` | focused list row |
| `lamp_default` | `(44, 44, 54)` | clear-lamp with no record yet |
| `button` | `(70, 80, 120)` | selected tab / button fill |
| `button_active` | `(40, 56, 80)` | active/toggled button, search box |
| `good` | `(90, 200, 230)` | progress / score-graph bars |
| `warn` | `(230, 180, 60)` | warning / pacemaker-behind |

Colours that are **semantic** (not themed): clear-lamp colours, per-mode badge colours (`mode_color`), per-difficulty colours, DJ-rank band colours, and everything from the skin. These convey data, not chrome, so they stay fixed.

## Example: a high-contrast dark theme

```ron
(
    bg:       Some((0, 0, 0)),
    topbar:   Some((10, 10, 14)),
    accent:   Some((0, 255, 200)),
    focus:    Some((0, 255, 200)),
    row_focus:Some((0, 60, 50)),
    text:     Some((255, 255, 255)),
)
```

Everything not listed keeps its default — so this themes the highlights without redesigning every surface.

## For the web front-end

The FE theme editor should expose the 21 fields above as colour pickers (grouped: Surfaces / Text / List / Controls / Meters), serialize to the RON form, and write `theme.ron`. Round-trip is loss-free: `ThemeConfig` is `#[serde(default)]`, so older/newer files interoperate (new fields take defaults). Keep the "semantic colours are not themable" rule in the UI copy.

## Status / coverage

The theme system is wired through the select screen, the result screen, the in-play HUD overlay chrome, and the settings/keyconfig/tables/folders/loading menus. Any colour still hardcoded is either semantic (intentional) or a small follow-up — adding a field + routing one `Color::rgb(..)` is mechanical now that the system exists.
