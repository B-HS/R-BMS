# UI Theme (`theme.ron`)

Last updated 2026-10-22 · matches 47a7f9c (`theme.ron` and `Theme` did not change; the default skin pack's own `theme.lua` gained the tokens listed under "The default skin pack's own tokens")

rbms separates three kinds of look-and-feel:

| Concern | Owned by | File |
|---|---|---|
| **Screens a skin pack draws** (select, decide, play, result) | the skin pack | `skin/<pack>/…`, see [`skin.md`](skin.md) |
| **Built-in note field** (lanes, notes, beams, gauge, key-bomb, HUD layout — drawn when no skin pack draws play) | `SkinConfig` | `assets/skins/*.ron`, or `--skin <file>` |
| **App chrome** (SKIN/PLAY menus, settings, key config, Tables, Folders, loading, practice panel, the key guide, the browser's option/search/filter overlays, notifications, and the built-in select/result screens and HUD text) | `ThemeConfig` → `Theme` | `~/.config/rbms/theme.ron` |

This document covers the native-player **theme** (the chrome).

## How it works

- `rbms_render::theme::Theme` is the resolved palette — a flat struct of `Color`s grouped by role.
- `rbms_render::theme::ThemeConfig` is its RON form: **every colour field is an optional `(r, g, b)` triple** (and three layout keys, below). Missing fields inherit `Theme::default()`, so a theme file may set only the colours it cares about.
- At startup the app reads `~/.config/rbms/theme.ron`, calls `ThemeConfig::parse(..).resolve()`, and installs it with `rbms_render::set_theme(theme)` (`apps/rbms-player/src/assets/theme_file.rs`).
- Every renderer reads the active palette via `rbms_render::theme()` (a cheap `Copy`), so changing the file and relaunching restyles the whole UI. A missing or broken file falls back to the built-in defaults — a bad theme can never brick the UI.

## The default palette

The default is **achromatic**: a pure black ground, grey surfaces and lines, white and grey text. What is selected is told by brightness, not hue — a lighter fill, white text, a white rail or outline. The steps are the ones the default skin pack's Mono palette uses (`assets/skins/rbms-default/lua/theme.lua`, `theme.mono`: 0, 22, 38, 60 for surfaces and lines; 184, 242 for text), so the app's menus and the skin's screens read as one set. Two steps differ from the pack on purpose: hints are `144` rather than `128` (they are written small on raised surfaces, where `128` would read worse than the palette before it), and the chosen-row fill `80` and the engaged-box fill `50` are steps the pack has no use for.

A hue is left only where the hue is the information (see "Colours that are not themed" below).

## Fields

All colour values are `Some((r, g, b))`, 0–255 per channel. Omit a line to keep its default. Three more keys are not colours and are described under "Layout fields" below.

| Field | Default | Role |
|---|---|---|
| `bg` | `(0, 0, 0)` | window background |
| `topbar` | `(22, 22, 22)` | top header bar |
| `panel` | `(22, 22, 22)` | panel fill (menu lists, unselected rows and tabs, graph panels) |
| `panel_hi` | `(38, 38, 38)` | raised panel / button fill (cards over a list, link buttons, empty bar tracks) |
| `divider` | `(60, 60, 60)` | hairline dividers / outlines, slider rails, graph grid lines |
| `text` | `(242, 242, 242)` | primary text |
| `text_dim` | `(184, 184, 184)` | secondary text |
| `text_muted` | `(144, 144, 144)` | muted hint text |
| `accent` | `(255, 255, 255)` | headings, counts, sort label, links, a prompt the screen is waiting on, the info-level notification |
| `focus` | `(255, 255, 255)` | focus ring / selection rails, the caret, the grip of the focused slider, the key-config cell waiting for a key |
| `title_focus` | `(255, 255, 255)` | what is written on the thing under the cursor: the focused song-row title, the chosen tab's name, the value of a menu's focused row |
| `title_dim` | `(200, 200, 200)` | unfocused song-row title |
| `row_even` | `(22, 22, 22)` | even list row |
| `row_odd` | `(30, 30, 30)` | odd list row |
| `row_folder` | `(38, 38, 38)` | folder list row |
| `row_focus` | `(60, 60, 60)` | focused list row |
| `lamp_default` | `(68, 68, 68)` | clear-lamp with no record yet |
| `button` | `(80, 80, 80)` | chosen tab / focused menu row / primary button fill |
| `button_active` | `(50, 50, 50)` | active/toggled button, search box, secondary button |
| `good` | `(200, 200, 200)` | progress bars (loading, replay analysis) |
| `warn` | `(230, 180, 60)` | cautions: a stale table copy, a table that failed, a hi-speed at its limit, a key-config notice that warns or asks (a lane put back on its shipped key, the ask to restore everything) |

### Layout fields

`ThemeConfig` also accepts three keys that place things rather than colour them. Each is optional, a rectangle that does not lie inside the 1280×720 canvas (or a number that is out of range) is ignored and the default is kept, and the template the app writes does not show them (a test pins that they are the only keys it leaves out).

| Key | Form | What it does |
|---|---|---|
| `select_layout` | `Some((list_rect: Some((x, y, w, h)), detail_rect: Some(…), cover_rect: Some(…), row_height: Some(f32), row_gap: Some(f32)))` | where the built-in song select puts its list, its detail panel and the cover art, and how tall a row is (defaults: list `(32, 60, 584, 600)`, detail `(632, 60, 616, 600)`, cover `(648, 78, 160, 160)`, row 36 with a gap of 4). A skin that draws the song select ignores it |
| `select_panel_alpha` | `Some(0–255)` | opacity of the panels of the built-in song select (default 255) |
| `options_layout` | `Some((rect: Some((x, y, w, h)), panel_alpha: Some(u8), row_overrides: Some([(index: n, y: …, label_x: …, value_right_x: …, label_color: Some((r, g, b)), value_color: Some((r, g, b)))])))` | the option overlay (`F1`, `apps/rbms-player/src/app_options.rs`): its rectangle, its opacity (default 235) and, for each of its 11 rows by index, a position or a colour |

Field names and meanings are unchanged from earlier versions; a theme file written for them is read as before. Three fields are used in more places than they were:

- `title_focus` and `focus` now colour the selection marks that used to be a fixed yellow (the chosen tab's name, a focused row's value, a slider's grip, a caret), so a theme can restyle them.
- `warn` now also colours the cautions that used to be a fixed yellow (the key-config notices that warn or ask, the hi-speed limit mark). A key-config line that only says how things stand (a confirmation, the lane a shortcut shares its key with) is `text_dim`, not `warn`.
- `accent` no longer colours FAST on the built-in result screen. FAST/SLOW is a meaning, and is always cyan/orange.

### Contrast (default palette)

WCAG contrast ratios of text on the fills it is drawn on, against the palette before it:

| Text on fill | Before | Now |
|---|---|---|
| `text` on `bg` | 16.75 | 18.76 |
| `text_dim` on `bg` | 8.71 | 10.59 |
| `text_muted` on `bg` | 4.81 | 6.58 |
| `text_muted` on `panel` | 4.57 | 5.67 |
| `text_muted` on `panel_hi` | 4.25 | 4.74 |
| `text` on `panel_hi` | 14.80 | 13.52 |
| `text_dim` on `panel_hi` | 7.70 | 7.63 |
| `text` on `button` | 6.58 | 7.20 |
| `text_dim` on `button` | 3.42 | 4.06 |
| `title_focus` on `button` (was yellow) | 5.10 | 8.06 |
| `text_muted` on `button_active` | 2.86 | 4.02 |
| `text_muted` on `row_focus` | 2.78 | 3.46 |
| `title_dim` on `row_odd` | 8.48 | 9.96 |

Hints are at 4.5 or more on the ground, a panel and a raised panel. They are below it on the two engaged fills and the focused row (4.02 and 3.46), where a hint sits beside brighter text that says the same thing more fully; both are higher than they were. The two raised-panel ratios that fell did so because the raised panel was lifted to the skin's step so surfaces separate (panel to raised panel 1.07 → 1.20); both stay above 7.

## Colours that are not themed

These convey data, not chrome, so they keep their hue whatever the theme:

- judgement colours (PGREAT/GREAT/GOOD/BAD/POOR/MISS) and FAST (cyan) / SLOW (orange);
- clear-lamp colours and clear names, including COURSE CLEAR / COURSE FAILED, and the four assist grades of a SCRATCH AUTO clear (ASSIST EASY, ASSIST CLEAR, ASSIST HARD, ASSIST EXHARD: `3ad19f`, `c4a796`, `afb8ef`, `ca76a9`, the colours the web's ranking pages give them; `apps/rbms-player/src/format.rs`);
- per-difficulty colours, per-mode badge colours (`mode_color`), DJ-rank band colours;
- score deltas (gain green, loss red), the combo and BP figures of a record, the score-graph bars (NOW cyan, BEST green), the density histogram, the gauge and its figure;
- the green number;
- the levels of a notification (warning orange, error red), a key-config clash or refusal (red), a course that cannot be played (red), the score-server lines and the connection dot (green/yellow/red);
- everything a skin draws, and the notes, key beams, bombs and judge line of the built-in note field (`SkinConfig`).

## The file the app writes

When there is no `theme.ron`, the app writes a template in which **every colour is a comment**: each line shows a field, its default and what it colours, and sets nothing. A file left as the template therefore follows the defaults of whichever version reads it. To set a colour, remove the `//` at the start of its line and edit the value.

Earlier versions wrote a template that set every colour to that version's default, which kept those colours in force after the defaults changed. A `theme.ron` that is still that template to the character (line endings and blank lines at the end aside) is taken for what it is — nothing you chose — and is replaced with the current template on the next start. A file that differs in anything else is yours: it is read as it stands and never written. If the folder cannot be written, the earlier template stays as it is, the defaults are drawn, and the terminal does not claim it was replaced.

If you edited that earlier template and want the achromatic palette, either delete `theme.ron` (the app writes the new template on the next start) or delete from it every line you did not mean to set.

## Example: the palette before the achromatic one

To bring back the blue-tinted palette of earlier versions, put this in `theme.ron`:

```ron
(
    bg:            Some((8, 8, 14)),
    topbar:        Some((18, 18, 30)),
    panel:         Some((14, 16, 26)),
    panel_hi:      Some((22, 24, 36)),
    divider:       Some((40, 42, 58)),
    text:          Some((235, 235, 235)),
    text_dim:      Some((170, 170, 185)),
    text_muted:    Some((120, 124, 140)),
    accent:        Some((120, 205, 235)),
    focus:         Some((120, 200, 240)),
    title_focus:   Some((244, 236, 156)),
    title_dim:     Some((190, 186, 150)),
    row_even:      Some((22, 24, 34)),
    row_odd:       Some((27, 29, 42)),
    row_folder:    Some((40, 44, 30)),
    row_focus:     Some((50, 56, 82)),
    lamp_default:  Some((44, 44, 54)),
    button:        Some((70, 80, 120)),
    button_active: Some((40, 56, 80)),
    good:          Some((90, 200, 230)),
    warn:          Some((230, 180, 60)),
)
```

One line must differ from the template earlier versions wrote for the file to count as yours — the comment lines that template had are left out above, which is enough. The selection marks that were a fixed yellow follow `title_focus` and `focus` now, so they come out pale yellow and sky blue rather than the old yellow.

## Example: a coloured accent on the achromatic palette

```ron
(
    accent:      Some((76, 201, 240)),
    focus:       Some((76, 201, 240)),
    title_focus: Some((76, 201, 240)),
)
```

Everything not listed keeps its default — so this themes the highlights without redesigning every surface.

## The default skin pack's own tokens (`theme.lua`)

`theme.ron` and `Theme` never reach a screen a skin pack draws. The default pack keeps its colours and type in `assets/skins/rbms-default/lua/theme.lua` ([`skin.md`](skin.md) §3.4.3), and only that pack's Lua reads them. These were added on 2026-10-22 for the song-select option panels ([`skin.md`](skin.md) §3.4.5), the assist grades of the clear lamp and the bold face. The values are the ones in the file at 47a7f9c.

| Token | Value | Role |
|---|---|---|
| `theme.panel.column` `.orange` `.green` `.violet` `.sky` `.locked` | `theme.tone.orange` (240, 136, 90), `theme.tone.green` (89, 209, 133), `theme.tone.violet` (178, 107, 240), `theme.accent.sky` (76, 201, 240), `theme.mono.muted` (128, 128, 128) | the colour of one column of an option panel: the wire from key to box, the box heading and the column name under the boxes. `locked` is a read-only column (EX JUDGE) |
| `theme.panel.pick`, `.pick_ink`, `.glow` | (255, 152, 52), (26, 16, 6), 112 | the chosen value of a box: its fill, the dark ink written on it and the alpha of the glow drawn around it. The INFORMATION tag and a changed value use the same orange |
| `theme.panel.key_white`, `.key_black`, `.key_white_ink`, `.key_black_ink` | (226, 230, 236), (30, 33, 40), (28, 30, 36), (236, 238, 242) | the seven key drawings of the panels and the numbers written on them |
| `theme.panel.arrow`, `.green_number`, `.caution` | `theme.tone.red`, `theme.tone.green`, `theme.tone.orange` | the `<` `>` of a number box, the green number beside NOTE DISPLAY TIME, a caution line (KEEP GREEN NUMBER IS OFF, the judge algorithm set to SCORE elsewhere) |
| `theme.grade_lamp` `.easy` `.clear` `.hard` `.exhard` | (58, 209, 159), (196, 167, 150), (175, 184, 239), (202, 118, 169) | the colours of ASSIST EASY, ASSIST CLEAR, ASSIST HARD and ASSIST EXHARD, the same hex as the web (`3ad19f`, `c4a796`, `afb8ef`, `ca76a9`). On the web each is the matching gauge's lamp colour mixed 35% towards the LIGHT ASSIST colour in OKLab, at least 12 OKLab units from each of the eleven older lamp colours (checked by script, not by eye) |
| `theme.assist_grade[0..4]`, `theme.assist_grade_last`, `theme.graded_clear` | name and colour of each grade (0 is `LIGHT ASSIST` in `theme.lamp.light_assist`), 4, 3 | the grades of the one lamp that has them, id 3 (`graded_clear`) |
| `theme.clear[2]`, `theme.clear[3]` | `ASSIST`, `LIGHT ASSIST` | lamp names; they were `ASSIST CLEAR` and `LIGHT ASSIST CLEAR` |
| `theme.font.bold` | `{ id = "inter-bold", path = "font/Inter-Bold.ttf" }` | the bold face the panels write names and chosen values in. A document declares it in its font list only when a bold line is built (`Builder:font_id` in `lua/ui.lua`), so the font lists of the other screens did not change |

The grade colours are a table of their own and not four more entries of `theme.lamp` because `tools/sprites.py` reads `theme.lamp` in order to draw the lamp strip: four more entries made it 15 cells instead of 11 and `generate-images.py --check` reported the select sheet as different. A skin that reads the grade of a lamp does so through the extension options of [`skin.md`](skin.md) §1.4.

## Web front-end boundary

`theme.ron` is a **native-player-only** configuration file. The existing web app uses CSS tokens and `next-themes`; it does not read, write, upload, or parse native RON theme files. A browser editor that imports or exports `theme.ron` is future work and needs an explicit file-transfer contract before implementation. If added, it must preserve partial-theme semantics (`ThemeConfig` uses `#[serde(default)]`) and retain the rule that semantic play-result colours are not user-themable.

## Status / coverage

The theme system is wired through the select screen, the result screen, the in-play HUD overlay chrome, and the SKIN/PLAY/settings/keyconfig/tables/folders/loading/practice menus and overlays. A colour that is still a literal in that code is one of the meanings listed above. The built-in note field's lane background, outline and divider come from `SkinConfig`; its defaults are grey (`SURFACE` 22, `Color::GRAY` 96, `LINE` 60), and the bundled `assets/skins/{normal,wide,default}.ron` give the same three values, which a test (`the_note_field_layouts_that_ship_draw_the_lane_in_the_default_greys`) holds equal, so the built-in play screen has no hue in its lanes either.
