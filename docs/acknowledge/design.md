# DESIGN.md — flunti-otel

> Framework-agnostic design system specification for the **flunti-otel** observability platform.
> A recreator with only this document and their target framework's docs can rebuild both UI surfaces at ~100% visual fidelity.

---

## 1. Meta & Scope

### 1-1. Provenance

| Field | Value |
|---|---|
| Source repository | `/Users/gkn/flunti-otel` |
| Commit | `9aa5ff6` (branch `dev`) |
| Extraction method | **Declarative** — every value below is read from source files, not inferred from computed styles |
| Corroborating measurements | `docs/quality-assurance/spa-ui-consistency.md` — the project independently verified layout constants by `getBoundingClientRect()` across 24–26 routes in light and dark. Where this document states a pixel constant, that measurement is the second witness. |
| Design decisions of record | `docs/acknowledge/0018-dashboard-spa-monorepo.md` (design rules), `0021-three-column-shell.md` (shell), `0028-admin-form-validation-feedback.md` (form error surface) |

### 1-2. Two surfaces, two token sets

flunti-otel ships **two distinct UI surfaces** that do not share a stylesheet. This is the single most important fact for a recreator.

| | **Surface A — Dashboard SPA** | **Surface B — Public pages** |
|---|---|---|
| Path | `apps/web/` | `apps/server/src/view/` |
| Theme file | `apps/web/src/index.css` | `apps/server/src/view/style/base.css` |
| Renders | **30 authenticated routes** + catch-all redirect | login / sign-up / pending-approval / guide |
| Radius | **`0rem`** — hard square | **`0.375rem`** — 6px rounded |
| Surface shadow | **`none`** | `0px 0px 7px` glow stack |
| Background tiers | **3** (sidebar / body / card) | **1** (`oklch(1 0 0)`) |
| `--warning` token | **present** | absent |
| Component source | shadcn/ui new-york + Radix | hand-written `hono/jsx` + cva |

Surface A is the product. Surface B is the doorway. They are deliberately different: B inherited the pre-migration token set and was left alone when A adopted the square/flat identity. **A recreator must not unify them without an explicit decision.**

### 1-3. Framework matrix (source stack)

| Concern | Surface A | Surface B |
|---|---|---|
| Runtime | Browser (Vite 7 SPA) | Bun + Hono 4.12 server render |
| View | React 19.2 + React Compiler (`babel-plugin-react-compiler`) | `hono/jsx` |
| Routing | `react-router` 7 (`BrowserRouter`, `basename = import.meta.env.BASE_URL`) | HTTP routes |
| CSS | Tailwind v4.3.2 (`@tailwindcss/vite`) | Tailwind v4.3.2 (`@tailwindcss/cli`, built to `public/style.css`) |
| Components | shadcn/ui style `new-york`, baseColor `neutral`, cssVariables `true` | none |
| Primitives | `radix-ui` 1.6.7 | native HTML |
| Icons | `lucide-react` | inline SVG |
| Charts | `recharts` 3.8.0 | none |
| Motion | `motion` (framer) 12 | none |
| Toasts | `sonner` 2 | none |
| Command palette | `cmdk` 1.1.1 | none |
| Forms | `react-hook-form` 7.83 + `@hookform/resolvers` + `zod` 4 | native `<form>` POST |
| Server state | `@tanstack/react-query` 5.90 | none |
| Theme switch | hand-rolled `useTheme` hook (`localStorage` + `.dark` class). `next-themes` 0.4.6 **is installed but has no provider** — see §4-6 | inline blocking `ThemeScript` |
| Class merge | `clsx` + `tailwind-merge` 3.6 via `cn()` | same |
| Variants | `class-variance-authority` 0.7.1 | same |

### 1-4. In scope

- Both surfaces' complete color, typography, spacing, radius, border, shadow, motion and z-index systems.
- Every component archetype with anatomy, tokens, and states.
- Every page archetype and its responsive collapse.
- Light and dark palettes (both already exist in source — **derived, not invented**).

### 1-5. Out of scope

- Data fetching, query keys, RPC contracts, auth flows.
- Server architecture (`route/` · `service/` · `compose/` · `db/`).
- OTLP ingest, alerting, retention — see `docs/ARCHITECTURE.md`.
- The compiled artifact `apps/server/public/style.css` (build output of Surface B's `base.css`).

### 1-6. Delivery layer

Section 16 ships a paste-ready `:root { --… }` + `@theme` block. Every color is OKLCH. Components reference **semantic** variables only; the palette tier is internal. The same tokens work in React, Vue, Svelte, Astro or Solid — nothing in the anatomy of Section 10 requires a specific framework.

### 1-7. Reading guide

| If you are… | Read |
|---|---|
| Rebuilding the whole product | 1 → 18, in order |
| Only theming an existing build | 3, 4, 5, 6, 16 |
| Recreating one component | 6 (tokens) → 10 (that archetype) → 17 (its prompt) |
| Porting away from React/shadcn | 14, then 10 |
| Verifying a recreation | 18 |

### 1-8. Notation

- `oklch(L C H)` — L in 0…1, C absolute, H in degrees. Three decimals for L and C, integer for H.
- Every palette entry carries a `/* … */` comment naming its **source variable and role**. The source is already OKLCH, so there is no hex ancestor to preserve; the comment carries provenance instead. The seven values that *were* HSL in source keep their original HSL string in the comment.
- Pixel constants are absolute. `12px` means twelve device-independent pixels, not "the framework default".

---

## 2. Visual Theme & Atmosphere

**flunti-otel looks like an instrument, not an application.** Every softening affordance that a normal dashboard reaches for has been deliberately removed: corners are square (`--radius: 0rem`), surfaces cast no shadow, and cards have no border. What separates one block from the next is a **one-pixel gap through which the page background shows** — the seam is the absence of surface, not a drawn line. The result reads like a panel of gauges bolted flush to a chassis. There is no floating, no elevation, no rounded chrome to suggest that any element could be picked up and moved.

**Depth is carried entirely by lightness.** Three background tiers stack in a fixed order — the navigation rails sit darkest-in-light (`oklch(0.915)`), the page body sits between (`oklch(0.955)`), and content cards sit brightest (`oklch(1)`). In dark mode the order inverts its absolute values but preserves its logic exactly: rails `oklch(0.098)`, body `oklch(0.162)`, cards `oklch(0.212)`. A user who learns "content is the brighter plane" in light mode does not have to relearn it in dark. This is the whole depth system; there is no second mechanism.

**Color is rationed, and it means something when it appears.** The interface is monochrome by default — every chart series that carries no semantic weight is drawn in `--muted-foreground`, the same gray as secondary text. Hue is spent only on meaning: `--destructive` for 5xx and errors, `--warning` for 4xx and thresholds, `--chart-2` / `--chart-4` for the good/needs-improvement/poor ladder of web vitals. The project encoded a corollary that is easy to miss and hard to unlearn: **a zero value loses its accent color and falls back to `text-muted-foreground`** — painting a zero red would advertise a problem that does not exist. Motion follows the same austerity: route transitions fade opacity only over `0.18s`, bars grow over `0.24s`, and nothing slides, bounces, or scales.

### Key characteristics

1. **Square** — `--radius: 0rem`, and all four derived radii pinned to it. No rounded surface anywhere on Surface A.
2. **Flat** — `--shadow-2xs` through `--shadow` are `none`. Shadows exist only for overlays (`md` and above).
3. **Borderless** — regions are separated by background lightness and a `1px` gap, not by strokes. Even the sidebar's right border is explicitly zeroed.
4. **Three-tier depth** — sidebar < body < card, in both themes, as the sole spatial cue.
5. **Twelve** — content inset is `12px` on both axes; there is one padding value and it does not vary by axis.
6. **Monochrome-first** — default series and non-semantic UI are `--muted-foreground`; hue is reserved for status.
7. **Full-bleed navigation** — sidebar menu items have no radius, no gap, and no inset; the active item fills the rail edge to edge rather than floating as a pill.
8. **Dense** — table cells are `text-xs` at `p-2`; the design assumes an operator reading many rows, not a visitor skimming a page.
9. **No emoji, ever** — iconography is `lucide-react` only. This is an explicit project rule, not an accident.

---

## 3. Color System (Light)

### 3-1. Conversion policy

The source is **already OKLCH**. There is no hex ancestor to preserve, so this document does not manufacture one.

- **Source OKLCH values are reproduced verbatim, including fractional hue** (`27.325`, `184.704`, `41.116`). Re-rounding hue to an integer would inject drift into a source that is already in the target color space — a fidelity loss with no benefit. The "integer hue" convention applies only to values this document *converts*.
- **Seven source values were HSL** (all in the sidebar group). Those were converted to OKLCH via sRGB → linear-sRGB → OKLab → OKLCH at 3-decimal L, 3-decimal C, integer H. Each carries its original HSL string in the comment. The conversion was validated against known shadcn anchors: `hsl(0 0% 98%)` → `oklch(0.985 0 0)`, matching shadcn's declared `oklch(0.985 0 0)` exactly.
- **Shadow colors were `hsl(0 0% 0% / α)`** — pure black with alpha. Expressed as `oklch(0 0 0 / α)`.

### 3-2. Palette layer — Surface A (raw values, never referenced by components)

The source declares no palette tier; it binds semantic names straight to literals. This document lifts the literals into a palette so that dark mode is a re-binding rather than a rewrite.

**Neutral ramp.** Every neutral in this system has **chroma exactly 0**. The identity is true monochrome, not a tinted gray. Names encode L × 1000.

```css
--palette-neutral-1000: oklch(1 0 0);         /* card, popover, destructive-foreground (light) */
--palette-neutral-985:  oklch(0.985 0 0);     /* primary-foreground (light) / foreground (dark) */
--palette-neutral-970:  oklch(0.97 0 0);      /* secondary, muted, accent (light) */
--palette-neutral-955:  oklch(0.955 0 0);     /* background — body tier (light) */
--palette-neutral-922:  oklch(0.922 0 0);     /* border, input (light) / primary (dark) */
--palette-neutral-915:  oklch(0.915 0 0);     /* sidebar — rail tier (light) */
--palette-neutral-900:  oklch(0.9 0 0);       /* sidebar-border (light) */
--palette-neutral-855:  oklch(0.855 0 0);     /* sidebar-accent — active nav item (light) */
--palette-neutral-708:  oklch(0.708 0 0);     /* ring (light) / muted-foreground (dark) */
--palette-neutral-556:  oklch(0.556 0 0);     /* muted-foreground (light) / ring (dark) */
--palette-neutral-420:  oklch(0.42 0 0);      /* sidebar-foreground (light) */
--palette-neutral-269:  oklch(0.269 0 0);     /* secondary, muted, accent (dark) */
--palette-neutral-232:  oklch(0.232 0 0);     /* sidebar-accent (dark) */
--palette-neutral-212:  oklch(0.212 0 0);     /* card — content tier (dark) */
--palette-neutral-205:  oklch(0.205 0 0);     /* primary (light) / popover, primary-foreground (dark) */
--palette-neutral-162:  oklch(0.162 0 0);     /* background — body tier (dark) */
--palette-neutral-145:  oklch(0.145 0 0);     /* foreground (light) */
--palette-neutral-098:  oklch(0.098 0 0);     /* sidebar — rail tier (dark) */
```

**Alpha neutrals.** Dark mode's border and input are translucent white, not opaque gray — they take on whatever surface tier sits beneath them.

```css
--palette-white-a10: oklch(1 0 0 / 10%);      /* border (dark) */
--palette-white-a15: oklch(1 0 0 / 15%);      /* input (dark) */
```

**Chromatic accents.** Thirteen values. Every one of them means something.

```css
--palette-red-577:    oklch(0.577 0.245 27.325);   /* destructive (light) — 5xx, error, delete */
--palette-red-704:    oklch(0.704 0.191 22.216);   /* destructive (dark) */
--palette-amber-705:  oklch(0.705 0.153 70);       /* warning (light) — 4xx, threshold */
--palette-amber-790:  oklch(0.79 0.145 75);        /* warning (dark) */
--palette-orange-646: oklch(0.646 0.222 41.116);   /* chart-1 (light) — selected series */
--palette-teal-600:   oklch(0.6 0.118 184.704);    /* chart-2 (light) — "good" */
--palette-blue-398:   oklch(0.398 0.07 227.392);   /* chart-3 (light) */
--palette-yellow-828: oklch(0.828 0.189 84.429);   /* chart-4 (light) — "needs improvement" */
--palette-amber-769:  oklch(0.769 0.188 70.08);    /* chart-5 (light) AND chart-3 (dark) */
--palette-indigo-488: oklch(0.488 0.243 264.376);  /* chart-1 (dark) — selected series */
--palette-green-696:  oklch(0.696 0.17 162.48);    /* chart-2 (dark) — "good" */
--palette-purple-627: oklch(0.627 0.265 303.9);    /* chart-4 (dark) — "needs improvement" */
--palette-rose-645:   oklch(0.645 0.246 16.439);   /* chart-5 (dark) */
```

**Sidebar chromatics** — the only values in the system that are *not* pure neutral or semantic accent. They arrived with shadcn's sidebar block, were never re-authored, and carry a faint blue tint (`C ≈ 0.001–0.006`) that the rest of the palette does not.

```css
--palette-slate-210:  oklch(0.210 0.006 286);  /* hsl(240 5.9% 10%)     — sidebar-primary, sidebar-accent-foreground (light) */
--palette-slate-968:  oklch(0.968 0.001 286);  /* hsl(240 4.8% 95.9%)   — sidebar-foreground, sidebar-accent-foreground (dark) */
--palette-slate-274:  oklch(0.274 0.005 286);  /* hsl(240 3.7% 15.9%)   — sidebar-border (dark) */
--palette-blue-623:   oklch(0.623 0.188 260);  /* hsl(217.2 91.2% 59.8%) — sidebar-ring (both themes) */
--palette-blue-488:   oklch(0.488 0.217 264);  /* hsl(224.3 76.3% 48%)  — sidebar-primary (dark) */
```

**Shadow ink.**

```css
--palette-shadow-ink: oklch(0 0 0);            /* hsl(0 0% 0%) — overlay shadows only */
```

**Palette total: 18 neutral + 1 preserved drift variant + 2 alpha + 13 chromatic + 5 sidebar + 1 ink = 40 values.**

> **Known source drift.** Dark `--destructive-foreground` is written `oklch(0.9851 0 0)` — four decimals, `0.0001` above the `oklch(0.985 0 0)` used by every other near-white in the system. The difference is below both perceptual and 8-bit rendering thresholds. This document preserves the value as written (it is what the source ships) as `--palette-neutral-985-alt`, and recommends — but does not silently apply — unification to `--palette-neutral-985`.

### 3-3. Semantic layer — Surface A, light

Components reference **only** these. Thirty-three tokens.

| Semantic token | Palette source | Role |
|---|---|---|
| `--color-background` | `neutral-955` | **Tier 2** — page body plane; the 1px seams between blocks show this |
| `--color-foreground` | `neutral-145` | Body text |
| `--color-card` | `neutral-1000` | **Tier 3** — content surface, the brightest plane |
| `--color-card-foreground` | `neutral-145` | Text on cards |
| `--color-popover` | `neutral-1000` | Floating surfaces (menu, select, tooltip body) |
| `--color-popover-foreground` | `neutral-145` | Text on floating surfaces |
| `--color-primary` | `neutral-205` | Solid action fill — near-black, not a brand hue |
| `--color-primary-foreground` | `neutral-985` | Text on primary |
| `--color-secondary` | `neutral-970` | Quiet fill |
| `--color-secondary-foreground` | `neutral-205` | Text on secondary |
| `--color-muted` | `neutral-970` | Inert fill (skeleton, disabled track, table head) |
| `--color-muted-foreground` | `neutral-556` | **Secondary text AND the default chart series color** |
| `--color-accent` | `neutral-970` | Hover fill |
| `--color-accent-foreground` | `neutral-205` | Text on hover fill |
| `--color-destructive` | `red-577` | 5xx, error, exception, delete |
| `--color-destructive-foreground` | `neutral-1000` | Text on destructive |
| `--color-warning` | `amber-705` | 4xx, threshold approach, degraded — **Surface A only** |
| `--color-border` | `neutral-922` | Hairlines where a border is unavoidable (tables, inputs) |
| `--color-input` | `neutral-922` | Form control border |
| `--color-ring` | `neutral-708` | Focus ring |
| `--color-chart-1` | `orange-646` | Series slot 1 — **the selected/highlighted series** |
| `--color-chart-2` | `teal-600` | Series slot 2 — "good" in vitals |
| `--color-chart-3` | `blue-398` | Series slot 3 |
| `--color-chart-4` | `yellow-828` | Series slot 4 — "needs improvement" in vitals |
| `--color-chart-5` | `amber-769` | Series slot 5 |
| `--color-sidebar` | `neutral-915` | **Tier 1** — left nav rail AND right context panel |
| `--color-sidebar-foreground` | `neutral-420` | Nav label text |
| `--color-sidebar-primary` | `slate-210` | Nav emphasis fill |
| `--color-sidebar-primary-foreground` | `neutral-985` | Text on nav emphasis |
| `--color-sidebar-accent` | `neutral-855` | **Active / hovered nav item fill** |
| `--color-sidebar-accent-foreground` | `slate-210` | Text on active nav item |
| `--color-sidebar-border` | `neutral-900` | Nav internal separator |
| `--color-sidebar-ring` | `blue-623` | Focus ring inside nav |

#### The three-tier rule

```
sidebar  oklch(0.915)   ← left rail + right context panel (symmetric)
background oklch(0.955) ← page body; visible as the 1px seam between blocks
card     oklch(1.000)   ← content surfaces
```

This ladder is the entire depth system. It is **not** decorative: `docs/acknowledge/0018` records that when `--background` and `--card` were both `oklch(1 0 0)`, region separation was impossible without borders, and the borderless identity could not exist. The tiers were spread deliberately, and spread *again* when the first dark values (`0.145 / 0.196 / 0.112`) proved too close to read as separate planes.

### 3-4. Chart series assignment rules

1. **A series with no semantic meaning is `--muted-foreground`.** Single-series charts are always monochrome. This is the default, not a fallback.
2. **`--chart-1` marks selection.** In the field-comparison view, the selected cohort is `--chart-1` and the remainder is `--muted-foreground`. Slot 1 is reserved for "the thing you picked".
3. **HTTP status classes are semantic, not slots.** Total is `--muted-foreground`, 4xx is `--warning`, 5xx is `--destructive`. `--chart-4` is explicitly rejected for 4xx because it renders purple in dark mode, which contradicts "warning".
4. **Web-vitals rating uses a fixed three-color ladder**: good `--chart-2`, needs-improvement `--chart-4`, poor `--destructive`.
5. **A zero value drops its accent** and renders `--muted-foreground`. Painting a zero red advertises a problem that does not exist.
6. Slots 3 and 5 exist for multi-series charts and carry no fixed meaning.

### 3-5. Gradient policy

**There are no gradients.** No `linear-gradient`, `radial-gradient`, or `conic-gradient` appears in either surface. Fills are flat; area charts use flat fills with opacity, not gradient stops. A recreator introducing a gradient is introducing a foreign element.

### 3-6. Palette layer — Surface B

Surface B declares its own literals. It shares the neutral ramp's upper half and the five light chart values, but **not** the three-tier backgrounds and **not** `--warning`.

```css
--palette-b-background-light: oklch(1 0 0);       /* --background (light) — a single white plane */
--palette-b-card-light:       oklch(1 0 0);       /* --card (light) — identical to background */
--palette-b-background-dark:  oklch(0.145 0 0);   /* --background (dark) */
--palette-b-card-dark:        oklch(0.205 0 0);   /* --card (dark) */
```

Every other Surface B color is a value already present in the Surface A palette above. **Surface B semantic total: 24 tokens** (Surface A's 33, minus `--warning`, minus the 8 `--sidebar-*`).

---

## 4. Dark Mode System

### 4-1. Philosophy

**Dark mode is not derived here — the source ships it.** Both surfaces declare a complete `.dark` block. This section documents what exists.

The derivation logic the original author applied, and which a recreator must preserve when adding any new token:

1. **Monochrome stays monochrome.** Every neutral is `C = 0` in both themes. No cool navy, no warm sepia. A tinted dark would be a different product.
2. **The three-tier ladder inverts its values but not its order.** Light rises `0.915 → 0.955 → 1.000`; dark rises `0.098 → 0.162 → 0.212`. In both themes **content is the brighter plane**. The user's spatial model survives the switch.
3. **The dark ladder is spread wider in relative terms** than a naive inversion. `docs/acknowledge/0018` records that the first attempt (`0.145 / 0.196 / 0.112`) put the tiers too close and out of order — the shell read as one undifferentiated mass. The shipped values fixed both.
4. **Accents gain lightness and lose chroma.** `--destructive` goes `0.577 0.245` → `0.704 0.191`; `--warning` goes `0.705 0.153` → `0.79 0.145`. Brighter to clear the dark surface, less saturated to avoid glare.
5. **Borders become translucent white** rather than a lighter gray, so a hairline reads correctly over any of the three tiers.
6. **Chart slots are re-hued entirely**, not merely re-lightened — see 4-4.

### 4-2. Dark palette

No new palette values are needed beyond §3-2 — dark re-binds the same ramp. The values used exclusively in dark:

| Palette entry | Value | Dark role |
|---|---|---|
| `--palette-neutral-985` | `oklch(0.985 0 0)` | foreground, card/popover/secondary/accent foreground |
| `--palette-neutral-922` | `oklch(0.922 0 0)` | primary |
| `--palette-neutral-708` | `oklch(0.708 0 0)` | muted-foreground |
| `--palette-neutral-556` | `oklch(0.556 0 0)` | ring |
| `--palette-neutral-269` | `oklch(0.269 0 0)` | secondary, muted, accent |
| `--palette-neutral-232` | `oklch(0.232 0 0)` | sidebar-accent |
| `--palette-neutral-212` | `oklch(0.212 0 0)` | **card — tier 3** |
| `--palette-neutral-205` | `oklch(0.205 0 0)` | popover, primary-foreground |
| `--palette-neutral-162` | `oklch(0.162 0 0)` | **background — tier 2** |
| `--palette-neutral-098` | `oklch(0.098 0 0)` | **sidebar — tier 1** |
| `--palette-white-a10` | `oklch(1 0 0 / 10%)` | border |
| `--palette-white-a15` | `oklch(1 0 0 / 15%)` | input |
| `--palette-red-704` | `oklch(0.704 0.191 22.216)` | destructive |
| `--palette-amber-790` | `oklch(0.79 0.145 75)` | warning |
| `--palette-indigo-488` | `oklch(0.488 0.243 264.376)` | chart-1 |
| `--palette-green-696` | `oklch(0.696 0.17 162.48)` | chart-2 |
| `--palette-amber-769` | `oklch(0.769 0.188 70.08)` | chart-3 |
| `--palette-purple-627` | `oklch(0.627 0.265 303.9)` | chart-4 |
| `--palette-rose-645` | `oklch(0.645 0.246 16.439)` | chart-5 |
| `--palette-slate-968` | `oklch(0.968 0.001 286)` | sidebar-foreground, sidebar-accent-foreground |
| `--palette-slate-274` | `oklch(0.274 0.005 286)` | sidebar-border |
| `--palette-blue-488` | `oklch(0.488 0.217 264)` | sidebar-primary |
| `--palette-blue-623` | `oklch(0.623 0.188 260)` | sidebar-ring (unchanged from light) |

### 4-3. Semantic re-bindings — Surface A, dark

All 33 semantic tokens, re-bound. Anatomy, spacing, typography, radius and shadow **do not change between modes**; only these bindings do.

| Semantic token | Light | Dark | Δ |
|---|---|---|---|
| `--color-background` | `neutral-955` | `neutral-162` | tier 2 inverts |
| `--color-foreground` | `neutral-145` | `neutral-985` | text inverts |
| `--color-card` | `neutral-1000` | `neutral-212` | tier 3 inverts |
| `--color-card-foreground` | `neutral-145` | `neutral-985` | |
| `--color-popover` | `neutral-1000` | `neutral-205` | **darker than card** — floats below content tier |
| `--color-popover-foreground` | `neutral-145` | `neutral-985` | |
| `--color-primary` | `neutral-205` | `neutral-922` | near-black ⇄ near-white |
| `--color-primary-foreground` | `neutral-985` | `neutral-205` | |
| `--color-secondary` | `neutral-970` | `neutral-269` | |
| `--color-secondary-foreground` | `neutral-205` | `neutral-985` | |
| `--color-muted` | `neutral-970` | `neutral-269` | |
| `--color-muted-foreground` | `neutral-556` | `neutral-708` | **lightens** — dark needs more L for the same read |
| `--color-accent` | `neutral-970` | `neutral-269` | |
| `--color-accent-foreground` | `neutral-205` | `neutral-985` | |
| `--color-destructive` | `red-577` | `red-704` | +0.127 L, −0.054 C |
| `--color-destructive-foreground` | `neutral-1000` | `neutral-985` *(source writes `oklch(0.9851 0 0)`)* | see §3-2 drift note |
| `--color-warning` | `amber-705` | `amber-790` | +0.085 L, −0.008 C |
| `--color-border` | `neutral-922` | `white-a10` | opaque → translucent |
| `--color-input` | `neutral-922` | `white-a15` | opaque → translucent |
| `--color-ring` | `neutral-708` | `neutral-556` | **darkens** — the ring sits on a bright-ish control |
| `--color-chart-1` | `orange-646` | `indigo-488` | re-hued |
| `--color-chart-2` | `teal-600` | `green-696` | re-hued |
| `--color-chart-3` | `blue-398` | `amber-769` | re-hued |
| `--color-chart-4` | `yellow-828` | `purple-627` | re-hued |
| `--color-chart-5` | `amber-769` | `rose-645` | re-hued |
| `--color-sidebar` | `neutral-915` | `neutral-098` | tier 1 inverts |
| `--color-sidebar-foreground` | `neutral-420` | `slate-968` | |
| `--color-sidebar-primary` | `slate-210` | `blue-488` | |
| `--color-sidebar-primary-foreground` | `neutral-985` | `neutral-1000` | |
| `--color-sidebar-accent` | `neutral-855` | `neutral-232` | active nav fill |
| `--color-sidebar-accent-foreground` | `slate-210` | `slate-968` | |
| `--color-sidebar-border` | `neutral-900` | `slate-274` | |
| `--color-sidebar-ring` | `blue-623` | `blue-623` | **unchanged** |

### 4-4. Chart series in dark — the hue rotation

This is the one place the dark theme does not merely re-light the palette. Light and dark chart slots are **different hues**:

| Slot | Light | Dark | Relationship |
|---|---|---|---|
| 1 | orange `41°` | indigo `264°` | opposite side of the wheel |
| 2 | teal `185°` | green `162°` | adjacent — "good" stays cool-green |
| 3 | deep blue `227°`, L `0.398` | amber `70°`, L `0.769` | dark's darkest slot would vanish, so it becomes the brightest |
| 4 | yellow `84°` | purple `304°` | opposite |
| 5 | amber `70°` | rose `16°` | |

**Only slot 2 preserves its meaning across themes** (good = green/teal). This is why the vitals ladder is safe (`chart-2` / `chart-4` / `destructive`) but generic multi-series charts change character between modes. It is also the reason `--warning` exists as its own token: `--chart-4` is yellow in light and **purple** in dark, so it cannot carry "4xx = warning".

**Consequence a recreator must honor**: the light chart-3 (`L 0.398`) is the darkest chart value in the system and is unreadable on a dark surface; the dark chart-3 (`L 0.769`) is the brightest and washes out on white. The slots are not interchangeable — do not collapse them into one theme-agnostic set.

### 4-5. Shadow and border in dark

Surface A ships **`none`** for `2xs` / `xs` / `sm` / base in both themes, so there is no light-shadow-vanishes problem on surfaces — there was never a surface shadow to lose. The overlay shadows (`md` and above) are `oklch(0 0 0 / 0.10…0.18)` and are **not re-declared in dark**; black-on-dark shadow is nearly invisible, which is acceptable because overlays are separated by the `--color-popover` tier and (for modals) by the modal scrim, not by shadow.

Hairlines in dark rely on `--color-border: oklch(1 0 0 / 10%)`. Because it is translucent white it lightens whatever tier it sits on, which keeps a table rule legible on card (`0.212`), body (`0.162`) and rail (`0.098`) alike — an opaque gray could not do that.

### 4-6. Dark caveats

1. **`--color-popover` (`0.205`) is darker than `--color-card` (`0.212`).** A floating menu opened over a card reads as *receding*, not elevating. This is the source's value; it is intentional to the extent that overlays rely on the scrim, but a recreator should not "fix" it silently — it would change how every dropdown reads.
2. **Toast theming is decoupled from app theming.** `Toaster` calls `useTheme()` from `next-themes`, but **no `next-themes` provider is mounted**. The hook falls back to `'system'`, so sonner follows the operating system while the rest of the application follows `localStorage`. A user on OS-dark who chose app-light gets dark toasts. Documented as-is; see 4-7 for the mechanism.
3. **The application never reads `prefers-color-scheme`.** There is no such media query anywhere in either surface. An unset preference resolves to **light**, regardless of OS.
4. **Light `--chart-3` at `L 0.398`** is below the contrast floor for text; it is used for fills and strokes only. Never bind it to text.

### 4-7. Toggle implementation

Both surfaces use the **same storage key and the same class**, which is what lets the public login page and the dashboard agree on theme across a full page load.

| Constant | Value | Declared in |
|---|---|---|
| storage key | `flunti-otel-theme` | `apps/server/src/lib/theme.ts` |
| dark class | `dark` | same |
| light class | `light` (value only — never written to the DOM) | same |

**Surface A** — a hand-rolled hook (there is no `ThemeProvider`):

```
read  localStorage['flunti-otel-theme'] === 'dark' ? 'dark' : 'light'   (initial state)
apply documentElement.classList.toggle('dark', theme === 'dark')
write localStorage['flunti-otel-theme'] = theme
```

**Surface B** — a blocking inline script in `<head>`, so the first paint is already correct (no flash):

```html
<script>
  if (localStorage.getItem('flunti-otel-theme') === 'dark')
    document.documentElement.classList.add('dark')
</script>
```

**Custom variant** (both surfaces declare it identically):

```css
@custom-variant dark (&:is(.dark *));
```

#### Delivery for recreators

Section 16 ships the dark bindings under **three** selectors so the tokens work in any host:

```css
.dark                { /* source mechanism — authoritative */ }
[data-theme='dark']  { /* interop alias for hosts that stamp data-theme */ }
```

A `@media (prefers-color-scheme: dark)` trigger is **deliberately not enabled by default**, because it would contradict the source's explicit behavior (unset preference ⇒ light). Section 16 includes it as a commented opt-in block, guarded as `:root:not(.light):not([data-theme='light'])` so that a manual light choice still wins.

---

## 5. Typography

### 5-1. Root policy

**`font-size: 100%` — the browser default, 16px.** Neither surface overrides the root size. There is no `62.5%` trick. Every `rem` in this document is therefore `× 16px`, and Tailwind's spacing base is `--spacing: 0.25rem` = **4px**.

`html, body` receive exactly: `bg-background text-foreground font-sans antialiased`, plus `scroll-behavior: smooth` and hidden scrollbars.

### 5-2. Font families

Three stacks, both surfaces identical, all system fonts. **No web font is loaded** — there is no `@font-face`, no `<link rel=preconnect>`, no font CDN. First paint uses whatever the OS provides.

```css
--font-sans:
    ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto,
    'Helvetica Neue', Arial, 'Noto Sans', sans-serif,
    'Apple Color Emoji', 'Segoe UI Emoji', 'Segoe UI Symbol', 'Noto Color Emoji';

--font-serif:
    ui-serif, Georgia, Cambria, 'Times New Roman', Times, serif;

--font-mono:
    ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono',
    'Courier New', monospace;
```

**`--font-serif` is declared in both themes and referenced by zero components.** It is dead weight carried from the shadcn preset. Preserved here for completeness; a recreator may drop it.

**Language binding.** `<html lang="ko">` on both surfaces; all UI copy is Korean. The stack is **not** language-switched — there is no `html[lang="xx"]` selector anywhere. A recreator adding locales would introduce the binding as:

```css
html[lang='ko'] { font-family: var(--font-sans); }   /* current, implicit */
```

### 5-3. Mono is a first-class role, not an accent

`font-mono` appears **133 times** — more often than any single text-size utility except `text-xs`. It is the type treatment for machine-generated identity, and it is paired with `tabular-nums` (77) and `text-right` (126) for numeric columns.

| Mono is used for | Example |
|---|---|
| Trace / span IDs | `/trace/:traceId` cells |
| Durations, counts, percentiles | every numeric table column |
| Log bodies and raw payloads | `<pre>` blocks |
| Search expressions | the `q` input in the context panel |
| Query-syntax error messages | `QueryErrorCard` description |
| Attribute keys and values | the attribute tree |

**Rule**: any value the machine produced renders mono. Any value a human wrote renders sans.

### 5-4. Weight policy — exactly four, and `bold` is not one of them

This is a hard constraint. The entire product uses **four** weights, and `font-bold` (700) appears **zero** times.

| Weight | Utility | Uses | Where |
|---|---|---|---|
| 400 | `font-normal` | 4 | shadcn primitives only — resetting an inherited weight |
| 500 | `font-medium` | **44** | the workhorse: labels, table headers, nav items, buttons, badges |
| 600 | `font-semibold` | 16 | card titles, section headings, the app wordmark |
| 800 | `font-extrabold` | 4 | **Surface B only** — the guide `h1` and the auth/guide wordmark |

The ladder is `normal → medium → semibold`, with Surface B alone reaching for `extrabold` on two marketing-adjacent elements. **Introducing `font-bold` would break the system**: there is no 700 anywhere, and the jump 600 → 800 on the public pages is deliberate, not an approximation of bold.

### 5-5. Type scale

Two custom sizes are added to Tailwind's default ramp. **The two surfaces disagree on `--text-2xs`** — this is a real divergence, not a rounding artifact.

| Token | Surface A | Surface B | Uses | Role |
|---|---|---|---|---|
| `--text-3xs` | *(not declared)* | `0.5rem` / 8px | **0** | dead — declared and never used |
| `--text-2xs` | **`0.6875rem` / 11px** | **`0.625rem` / 10px** | 33 | micro-labels, timestamps, section eyebrows |
| `text-xs` | `0.75rem` / 12px | same | **155** | **the body size of the product** |
| `text-sm` | `0.875rem` / 14px | same | 73 | primitives, prose, form fields |
| `text-base` | `1rem` / 16px | same | 3 | shadcn primitives only |
| `text-lg` | `1.125rem` / 18px | same | 3 | shadcn primitives only |
| `text-xl` | `1.25rem` / 20px | same | 13 | **Surface B only** — page and section headings |
| `text-2xl` | `1.5rem` / 24px | same | 1 | one dashboard heading |
| `text-3xl` | `1.875rem` / 30px | same | 1 | **Surface B only** — the guide `h1` |

> **Surface A's effective scale is three sizes: 11 / 12 / 14px.** Everything above `text-sm` in the dashboard is either a shadcn default nobody overrode or a single one-off. This is what "dense operator tool" means numerically.

### 5-6. Semantic typography

| Element | Size | Weight | Family | Extras |
|---|---|---|---|---|
| App wordmark (rail) | `text-sm` | `font-semibold` | sans | `tracking-tight`, `truncate` |
| Panel title | `text-sm` | `font-medium` | sans | — |
| Section eyebrow (context panel) | `text-2xs` | `font-medium` | sans | `uppercase tracking-wide text-muted-foreground` |
| Table header cell | `text-xs` | `font-medium` | sans | `text-muted-foreground`; numeric columns add `text-right` |
| Table body cell | `text-xs` | inherit | sans, or **mono for machine values** | numeric adds `tabular-nums text-right`; the flexible column adds `max-w-0 truncate` |
| Stat tile value | `text-xs`…`text-2xl` per tile | `font-medium` | mono, `tabular-nums` | accent color only when semantic |
| Stat tile label | `text-2xs` | — | sans | `text-muted-foreground` |
| Body copy / hints | `text-xs` | — | sans | `text-muted-foreground` |
| Panel footnote | `text-xs` | — | sans | `text-muted-foreground`, sits below its table or chart |
| Nav item | `text-sm` | `font-medium` | sans | shadcn sidebar default |
| Button label | `text-sm` | `font-medium` | sans | `whitespace-nowrap` |
| Badge label | `text-xs` | `font-medium` | sans | |
| Code block (Surface B) | `text-2xs` → `lg:text-sm` | — | mono | `whitespace-pre-wrap break-all` — **the only `lg:` in the product** |
| Guide `h1` (Surface B) | `text-3xl` | `font-extrabold` | sans | `tracking-tight` |
| Guide `h2` (Surface B) | `text-xl` | `font-semibold` | sans | `tracking-tight` |
| Guide prose (Surface B) | `text-sm` | — | sans | `text-muted-foreground leading-7` |
| Auth `h1` (Surface B) | `text-xl` | `font-extrabold` | sans | |

### 5-7. Tracking and leading

Sparse and intentional. `tracking-tight` (15) on headings and the wordmark; `tracking-wide` (6) on uppercase micro-labels; `tracking-widest` (2). Leading is left to the size default except `leading-7` (6, Surface B prose), `leading-none` (4), `leading-snug` (4), `leading-normal` (2), `leading-relaxed` (1).

### 5-8. Principles

1. **12px is the body size.** Anything larger is a heading or a shadcn default nobody touched.
2. **Machine values are mono; human values are sans.** No exceptions in the source.
3. **Numeric columns are `tabular-nums text-right`.** Always both.
4. **Emphasis is weight, not size.** The scale barely moves; `font-medium` carries the load.
5. **Never `font-bold`.** The system has no 700.
6. **Truncation is `max-w-0 truncate` inside a flex/grid cell.** `truncate` alone is inert there — this cost the project a real bug.
7. **No heading levels in Surface A.** Panel titles render as `div`, not `h1`–`h6`. This is a known, deliberately deferred accessibility gap (§7-4).

---

## 6. Spacing, Radius, Border, Shadow

### 6-1. Spacing scale

`--spacing: 0.25rem` (4px) in both surfaces — Tailwind v4's default base. Values are `n × 4px`.

**The scale in actual use, by frequency:**

| Token | px | Uses | What it is |
|---|---|---|---|
| `gap-2` | 8 | **122** | the default gap between anything inline |
| `gap-1` | 4 | 58 | tight icon-to-label, action clusters |
| **`gap-px`** | **1** | **50** | **the block separator — the signature of the design** |
| `gap-3` | 12 | 29 | panel internal stack, nav item icon-to-label |
| `p-3` / `px-3` / `py-3` | 12 | 11 / 16 / 5 | **the panel inset, both axes** |
| `gap-4` | 16 | 15 | Surface B card internals |
| `px-2` | 8 | 20 | badge and table cell horizontal |
| `p-2` | 8 | 10 | **table cell padding** |
| `gap-1.5` | 6 | 15 | shadcn primitives |
| `gap-6` | 24 | 5 | Surface B section stack |
| `p-6` / `px-6` | 24 | 5 / 5 | Surface B auth card |
| `pb-20` | 80 | 1 | Surface B guide bottom runway |

`space-x-*`, `space-y-*` and `divide-*` are **never used** — separation is always `gap` plus background, never a divider utility.

**Negative margins** exist only inside stock shadcn primitives (`-mx-1`, `-my-2`, `-mt-1`, `-mb-2`). Application code uses none.

### 6-2. Layout pixel constants

Everything below is absolute. None of it is "a framework default" — a recreator without shadcn must set these explicitly.

| Constant | Value | Origin |
|---|---|---|
| Sidebar expanded | **256px** (`16rem`) | shadcn `SIDEBAR_WIDTH`, not overridden |
| Sidebar collapsed (icon rail) | **48px** (`3rem`) | shadcn `SIDEBAR_WIDTH_ICON` |
| Sidebar mobile sheet | **288px** (`18rem`) | shadcn `SIDEBAR_WIDTH_MOBILE` |
| Sidebar cookie | `sidebar_state`, max-age **604800s** (7 days) | shadcn |
| Sidebar keyboard toggle | **`Cmd/Ctrl + B`** | shadcn |
| Context panel | **320px** (`w-80`) | project |
| Rail header / footer | **48px** (`h-12`) | project |
| Nav item | **36px** (`h-9`) | project override of shadcn's 32px |
| Rail trigger button | **32px** (`size-8`) | project override of shadcn's 28px |
| Content left baseline | **268px** = 256 + 12 | emergent |
| Chart body | **224px** (`h-56`) | project |
| Page skeleton | **384px** (`h-96`) | project |
| Table skeleton | **256px** (`h-64`) | project |
| Panel skeleton | **96px** (`h-24`) | project |
| Facet row skeleton | **24px** (`h-6`) | project |
| Dialog max width | **672px** (`sm:max-w-2xl`) | project override of shadcn's `sm:max-w-lg` |
| Dialog max height | **85svh**, `overflow-y-auto` | project |
| Auth card max width | **384px** (`max-w-sm`) | Surface B |
| Guide content max width | **768px** (`max-w-3xl`) | Surface B |
| Mobile JS breakpoint | **768px** | `use-mobile.ts` |

**Table column width scale** — fixed utilities applied to `TableHead`, the flexible column taking `max-w-0 truncate`:

| Utility | px | Uses |
|---|---|---|
| `w-16` | 64 | 6 |
| `w-20` | 80 | 48 |
| `w-24` | 96 | **67** |
| `w-28` | 112 | 21 |
| `w-32` | 128 | 31 |
| `w-40` | 160 | 21 |
| `w-56` | 224 | 6 |

**Waterfall geometry** (`trace-waterfall.tsx`): name column 256px (`w-64`), service 96px (`w-24`), duration 80px (`w-20`), self 64px (`w-16`), timeline lane `min-w-0 flex-1`, indent **12px per depth level** applied as `paddingLeft` inside the name cell (so the timeline origin never shifts), minimum bar width **0.5%**.

**Service-map geometry** (`service-map-graph.tsx`, all JS constants driving an SVG):

```
LAYER_GAP 220   ROW_GAP 72     NODE_WIDTH 176   NODE_HEIGHT 40   PADDING 24
NODE_LABEL_FONT_SIZE 11        EDGE_LABEL_FONT_SIZE 10           NODE_LABEL_MAX_CHARS 24
EDGE_STROKE_WIDTH 1.5          NODE_STROKE_WIDTH 1.5
EDGE_LABEL_WIDTH 108           EDGE_LABEL_HEIGHT 32
ARROW_MARKER_SIZE 7            EXTERNAL_DASH '4 3'
```

### 6-3. Radius policy

| | Surface A | Surface B |
|---|---|---|
| `--radius` | **`0rem`** | `0.375rem` (6px) |
| `--radius-sm` | `var(--radius)` = 0 | `calc(var(--radius) - 4px)` = 2px |
| `--radius-md` | `var(--radius)` = 0 | `calc(var(--radius) - 2px)` = 4px |
| `--radius-lg` | `var(--radius)` = 0 | `var(--radius)` = 6px |
| `--radius-xl` | `var(--radius)` = 0 | `calc(var(--radius) + 4px)` = 10px |

**Surface A pins all four derived radii to `var(--radius)` rather than keeping shadcn's `calc()` chain.** This is mandatory: with `--radius: 0rem`, `calc(var(--radius) - 4px)` evaluates to `-4px`, which is invalid. A recreator who keeps the calc chain and sets radius to zero produces broken CSS, not square corners.

Application code contains exactly **one** radius utility — `rounded-none` in the nav button class, kept only because it is entangled with the collapsed-rail `!important` overrides through `tailwind-merge`. Everywhere else, squareness comes from the token.

Surface B uses bare `rounded` (Tailwind's default `0.25rem`) on cards and code blocks, `rounded-md` on controls, `rounded-full` on badges — i.e. it does **not** consume its own `--radius` token consistently.

### 6-4. Border policy

**Borders are suppressed.** The design separates regions by background lightness and a 1px gap. `divide-*` is never used.

Where a border does survive, it is one of four cases:

1. **Table rules** — `border-b` on rows (stock shadcn table).
2. **Form controls** — `border-input` on inputs and selects; controls must read as recessed.
3. **Focus and validation** — `border-ring` on focus-visible, `border-destructive` on `aria-invalid`.
4. **Tree indentation** — `border-l pl-3` on nested attribute-tree levels.

Explicitly zeroed:

| Element | Class | Why |
|---|---|---|
| `PanelCard` | `border-0` | the 1px gap is the separator |
| `StatTile` | `border-0` | shadcn `Item` ships `border border-transparent`; leaving it adds 1px and breaks the 268px baseline |
| Sidebar right edge | `border-r-0 group-data-[side=left]:border-r-0` | **both forms are required** — see below |

> **The variant-specificity trap.** shadcn applies the sidebar border as `group-data-[side=left]:border-r`. A plain `border-r-0` does **not** override it, because `tailwind-merge` does not merge across variant boundaries — the variant-qualified class simply wins. The border must be cancelled with the *same* variant: `group-data-[side=left]:border-r-0`. This regressed once in the project's history and was invisible to color and coordinate checks alike.

**Border colors in use**: `border-border` (2), `border-border/50` (1), `border-destructive` (7), `border-input` (8), `border-primary` (2), `border-ring` (12), `border-sidebar-border` (2), `border-transparent` (10), `border-neutral-600` (1, Surface B code block). Widths: default 1px, plus one `border-[1.5px]`; `border-dashed` twice (empty states, external service-map nodes).

### 6-5. Shadow policy

| Token | Surface A | Surface B |
|---|---|---|
| `--shadow-2xs` | **`none`** | `0px 0px 7px 0px oklch(0 0 0 / 0.03)` |
| `--shadow-xs` | **`none`** | `0px 0px 7px 0px oklch(0 0 0 / 0.03)` |
| `--shadow-sm` | **`none`** | `0px 0px 7px 0px oklch(0 0 0 / 0.06), 0px 1px 2px -1px oklch(0 0 0 / 0.06)` |
| `--shadow` | **`none`** | same as `sm` |
| `--shadow-md` | `0px 2px 8px 0px oklch(0 0 0 / 0.10)` | `0px 0px 7px 0px oklch(0 0 0 / 0.06), 0px 2px 4px -1px oklch(0 0 0 / 0.06)` |
| `--shadow-lg` | `0px 4px 16px 0px oklch(0 0 0 / 0.12)` | `0px 0px 7px 0px oklch(0 0 0 / 0.06), 0px 4px 6px -1px oklch(0 0 0 / 0.06)` |
| `--shadow-xl` | `0px 8px 24px 0px oklch(0 0 0 / 0.14)` | `0px 0px 7px 0px oklch(0 0 0 / 0.06), 0px 8px 10px -1px oklch(0 0 0 / 0.06)` |
| `--shadow-2xl` | `0px 16px 40px 0px oklch(0 0 0 / 0.18)` | `0px 0px 7px 0px oklch(0 0 0 / 0.15)` |

> Source writes these ink values as `hsl(0 0% 0% / α)`; they are reproduced above in OKLCH per §3-1. The values are identical — pure black at the same alpha.

**Surface A's rule: surfaces are flat, overlays are lifted.** The four small steps are `none`; only `md` and above carry a real shadow, and only overlays use them.

The reason is recorded in `docs/acknowledge/0018`: the inherited token set used a **zero-offset 7px blur** — a glow, not a shadow — which drew a halo around every card and read as a foreign gradient in a system with no other gradients. Surface A replaced the small steps with `none` and re-authored `md`–`2xl` as **offset** shadows. Surface B still carries the original glow stack.

**Application code contains zero shadow utilities.** Every shadow in Surface A is emitted by a stock shadcn overlay primitive.

**Two shadows are used as borders**, both inside the stock sidebar: `shadow-[0_0_0_1px_var(--sidebar-accent)]` and `shadow-[0_0_0_1px_var(--sidebar-border)]` — spread-only rings that outline without affecting layout.

### 6-6. Focus rings

One idiom, applied identically everywhere:

```
focus-visible:border-ring
focus-visible:ring-ring/50
focus-visible:ring-[3px]
```

and the invalid state:

```
aria-invalid:border-destructive
aria-invalid:ring-destructive/20
dark:aria-invalid:ring-destructive/40
```

`ring-[3px]` appears 13 times, `ring-ring/50` 13 times, `ring-destructive/20` and `/40` 9 times each. Inside the sidebar the ring token is `ring-sidebar-ring` (5). **`outline: none` never appears without a ring replacement.** The global base layer also sets `outline-ring/50` on `*`.

---

## 7. Motion & Accessibility

### 7-1. Motion tokens

The entire authored motion budget is **three constants** in one file.

```css
--motion-ease-standard:  cubic-bezier(0.4, 0, 0.2, 1);   /* EASE_STANDARD */
--motion-fade-duration:  0.18s;                          /* DURATION_FADE */
--motion-bar-duration:   0.24s;                          /* DURATION_BAR  */
```

There are exactly **two** motion patterns in the product, and nothing else:

| Pattern | Property | Duration | Where |
|---|---|---|---|
| **Fade** | `opacity` 0 → 1 | `0.18s` | route transition (the content column), stat tile mount, context-panel block |
| **Bar grow** | `width` 0 → *n*% | `0.24s` | facet bars, comparison bars, vitals distribution, waterfall span bars |

No slide, no scale, no bounce, no spring, no stagger, no page-exit animation. Route transitions changed from `opacity + translateY(4px)` to **opacity only** by explicit decision.

> **One drift**: the stat tile hardcodes `duration: 0.18` instead of importing the token. Same value, unbound. A recreator should bind it.

### 7-2. The second, unbound motion track

Stock shadcn primitives carry their own CSS transitions that do **not** use the tokens above:

| Utility | Uses | Where |
|---|---|---|
| `duration-200` | 6 | sidebar width, sheet |
| `duration-300` | 1 | |
| `duration-500` | 1 | |
| `duration-100` | 1 | `Item` color transition |
| `ease-linear` | 4 | sidebar width transitions |
| `ease-in-out` | 1 | |
| `transition-[color,box-shadow]` | 7 | the standard control transition |
| `transition-colors` | 5 | |
| `transition-transform` | 5 | chevron rotation |
| `transition-opacity` | 5 | hover-reveal affordances |

**Two systems coexist**: authored motion is `0.18s / 0.24s` with a Material curve; inherited motion is `200/300/500ms` with `linear`/`in-out`. They are not reconciled in source. A recreator porting to another framework should preserve both as documented rather than unify them, since unifying changes the sidebar's collapse feel.

### 7-3. Charts do not animate — and the enter animations are inert

**`isAnimationActive={false}` on every chart line.** No `animationDuration` is set anywhere. This is a hard constraint: an operator reading a latency chart should not watch it draw itself.

**A finding a recreator must know**: ten stock overlay primitives use `animate-in` / `animate-out` / `fade-in-0` / `zoom-in-95` / `slide-in-from-*` and the accordion uses `animate-accordion-down` / `animate-accordion-up`. **Neither `tailwindcss-animate` nor `tw-animate-css` is installed**, and neither theme file declares `--animate-accordion-*` keyframes. These utilities currently resolve to nothing — **dialogs, popovers, tooltips, selects, dropdowns, sheets and accordions open and close instantly.**

This document treats **instant overlay transitions as the specified behavior**, because that is what the product does and it is consistent with the flat, instrument-like identity. A recreator who installs the animation plugin will produce a product that moves more than the original. If you want the animations, that is a design change — make it deliberately.

The only animations that *do* resolve are Tailwind v4 core: `animate-spin` (2, the sonner loading icon) and `animate-pulse` (2, the skeleton and the live-stream indicator dot).

### 7-4. Accessibility

**`prefers-reduced-motion` is not handled anywhere.** No media query, no `motion-safe:` / `motion-reduce:` variant, no `useReducedMotion`, no `MotionConfig`. Every user gets the fades and bar tweens.

This is the single clearest accessibility gap in the system. A recreator **should** add the guard; it costs nothing and changes nothing for users who have not asked for it:

```css
@media (prefers-reduced-motion: reduce) {
  *, *::before, *::after {
    animation-duration: 0.01ms !important;
    animation-iteration-count: 1 !important;
    transition-duration: 0.01ms !important;
    scroll-behavior: auto !important;
  }
}
```

Note the last line: the source sets `scroll-behavior: smooth` globally, which reduced-motion users should not receive.

**What the source does well:**

| Concern | Status |
|---|---|
| Focus ring | Present on every interactive element (§6-6). `outline: none` never appears bare. |
| Keyboard | `Cmd/Ctrl + B` toggles the sidebar. All Radix primitives ship correct roving focus and dialog focus trapping. |
| Screen-reader labels | `sr-only` used 7×, including the mobile sheet's `SheetHeader`. Icon-only exclude buttons carry `aria-label`. |
| Validation | `aria-invalid` drives a visible border and ring, and every admin form field renders a `FieldError` bound to its input with focus moved to the offending field on submit. |
| Semantic controls | Actions are `<button>`, navigation is `<a>` (`NavLink`). Pagers are disabled buttons, not faked anchors. |
| Alt text | The service map SVG is `role='img'` with `aria-label='서비스맵'`. |

**What is missing:**

1. **No `<h1>`–`<h6>` anywhere in Surface A.** Panel titles render as `div` (shadcn `CardTitle`). A screen-reader user gets no document outline for any of the 30 dashboard routes. The project logged this as knowingly deferred because fixing it means forking `Card`. Surface B does use real headings. **A recreator building fresh should render panel titles as `h2`** — the visual result is identical.
2. **No landmark roles** beyond the implicit ones (`<aside>` for the context panel). No `<main>`, no `<nav>` wrapper on the rail beyond shadcn's internals.
3. **Colour is sometimes the only signal**: 4xx vs 5xx counts in the traffic table differ only by `text-warning` vs `text-destructive`. The number itself carries the meaning, so the information is not lost — but the severity distinction is.
4. **Scrollbars are globally hidden** (`scrollbar-width: none` + `::-webkit-scrollbar { display: none }`), removing a visible affordance that content continues below the fold.

### 7-5. Z-index scale

**Three values. That is the whole stack.**

| Token | Value | Uses | What sits there |
|---|---|---|---|
| `--z-rail` | `10` | 5 | the fixed sidebar container, sticky table headers |
| `--z-raised` | `20` | 1 | the sidebar's drag rail |
| `--z-overlay` | `50` | 13 | **every** portal layer — dialog, alert-dialog, sheet, popover, dropdown, select, tooltip, command, toast, plus the Surface B sticky guide header |

There is no layering *within* the overlay tier — Radix portals stack in DOM order at `z-50`. A recreator should not invent intermediate levels.

### 7-6. Visually-hidden utility

The source relies on Tailwind's `sr-only`. For a framework-agnostic recreation:

```css
.sr-only {
  position: absolute;
  width: 1px; height: 1px;
  padding: 0; margin: -1px;
  overflow: hidden;
  clip-path: inset(50%);
  white-space: nowrap;
  border-width: 0;
}
```

---

## 8. Layout System

### 8-1. The three-column shell (Surface A)

There is **no header bar.** This is the defining structural decision (`docs/acknowledge/0021`): the shell is purely horizontal, the content column owns the full `100dvh`, and the two rails are symmetric. Everything a top bar would have held — service picker, time range, search expression, trace jump — lives in the right panel; everything else — title, nav, identity, theme, logout — lives in the left rail.

```
┌─ 0 ────────────────── viewport width ───────────────────────────────┐
│                    │                                    │           │
│  SIDEBAR           │  CONTENT (SidebarInset)            │  CONTEXT  │
│  256px             │  fluid, min-w-0                    │  320px    │
│  bg-sidebar        │  bg-background                     │ bg-sidebar│
│                    │                                    │           │
│ ┌ SidebarHeader ─┐ │ ┌ ScrollArea ────────────────────┐ │ ┌ fixed ┐ │
│ │ h-12 (48px)    │ │ │ motion.div  flex-col gap-px    │ │ │ p-3   │ │
│ │ [trigger] name │ │ │ ┌────────────────────────────┐ │ │ │ svc   │ │
│ └────────────────┘ │ │ │ PanelCard                  │ │ │ │ range │ │
│ ┌ SidebarContent ┐ │ │ └────────────────────────────┘ │ │ │ from  │ │
│ │ 18 nav items   │ │ │  ↕ 1px  ← --color-background   │ │ │ to    │ │
│ │ each h-9(36px) │ │ │ ┌────────────────────────────┐ │ │ │ q     │ │
│ │ full-bleed     │ │ │ │ PanelCard                  │ │ │ │ trace │ │
│ │ gap-0, p-0     │ │ │ └────────────────────────────┘ │ │ └───────┘ │
│ └────────────────┘ │ └────────────────────────────────┘ │ ┌ scroll┐ │
│ ┌ SidebarFooter ─┐ │                                    │ │ facets│ │
│ │ h-12 (48px)    │ │                                    │ │Collaps│ │
│ │ name ☼ ⏻       │ │                                    │ └───────┘ │
│ └────────────────┘ │                                    │           │
└────────────────────┴─ 1px gap ──────────────────────────┴───────────┘
  ↑ top = 0            ↑ top = 0                            ↑ top = 0
  no right border      content left baseline = 256 + 12 = 268px
```

### 8-2. Structural constants

Every value below was independently confirmed by `getBoundingClientRect()` measurement across 24 routes in both themes.

| Constant | Value | Declared as |
|---|---|---|
| Shell height | `100dvh` | `h-dvh min-h-0` on the provider root |
| `--sidebar-width` (expanded) | **`16rem` / 256px** | shadcn sidebar default, not overridden |
| `--sidebar-width-icon` (collapsed) | **`3rem` / 48px** | shadcn default |
| `--sidebar-width` (mobile sheet) | **`18rem` / 288px** | inline style on the sheet |
| Context panel width | **`20rem` / 320px** | `w-80 shrink-0` |
| Rail header / footer height | **`3rem` / 48px** | `h-12` |
| Nav item height | **`2.25rem` / 36px** | `h-9` (overrides shadcn's `h-8`) |
| Sidebar trigger | **`2rem` / 32px** | `size-8` (overrides shadcn's `size-7`) |
| Panel gutter (horizontal and vertical) | **`1px`** | `gap-px` |
| Content inset | **`12px` on both axes** | `py-3` on the card + `px-3` on header/content |
| Content left baseline | **`268px`** = 256 + 12 | emergent; measured on 25 routes |
| Top alignment | `sidebarTop = insetTop = asideTop = 0` | no header exists |
| Sidebar right border | **`0px`** | `border-r-0 group-data-[side=left]:border-r-0` |
| Chart body height | **`14rem` / 224px** | `h-56 w-full` on the chart container |

### 8-3. The 1px gutter is the border system

```css
/* the entire separation mechanism */
.shell-columns { display: flex; gap: 1px; }        /* content ↔ context panel */
.page-root     { display: flex; flex-direction: column; gap: 1px; }  /* block ↔ block */
```

Blocks are `--color-card`; the gap reveals `--color-background`. Because the tiers differ in lightness, a 1px reveal reads as a rule. **This replaces every border in the content area.**

Two rules follow, and both were learned the hard way in this codebase:

1. **The gap is produced in exactly one place.** Adding `pt-px` to a content wrapper *in addition to* the parent's `gap-px` shifts content down by 1px relative to the fixed rail. Separation is the parent's job; children contribute padding only.
2. **A `fixed` element does not participate in flex gap.** The sidebar is `fixed`; the content is in flow. Any offset applied through gap must be mirrored onto the fixed element by an explicit `top`/`height` calculation, or the two will disagree by exactly the gap.

### 8-4. Grid definitions

The app uses exactly **three** grid shapes. There is no dashboard grid engine, no drag-resize, no span system.

```css
/* 1. Stat tile row — the only responsive grid */
.tile-grid { display: grid; grid-template-columns: repeat(2, minmax(0,1fr)); gap: 1px; }
@media (min-width: 768px) {
  .tile-grid-3 { grid-template-columns: repeat(3, minmax(0,1fr)); }
  .tile-grid-4 { grid-template-columns: repeat(4, minmax(0,1fr)); }
  .tile-grid-5 { grid-template-columns: repeat(5, minmax(0,1fr)); }
}

/* 2. Comparison grid — the only three-tier layout in the product */
.compare-grid { display: grid; grid-template-columns: 1fr; gap: 1.5rem; }
@media (min-width: 768px)  { .compare-grid { grid-template-columns: repeat(2, minmax(0,1fr)); } }
@media (min-width: 1280px) { .compare-grid { grid-template-columns: repeat(3, minmax(0,1fr)); } }

/* 3. Waterfall row — a fixed four-column ledger + one fluid lane */
.trace-row { display: grid; grid-template-columns: <name> <service> <duration> <self> 1fr; }
```

**Known deviation**: the metrics page uses `grid-cols-3` with **no** `md:` prefix, so it renders three columns at every viewport width including phones. Every other tile grid starts at two.

### 8-5. Overflow ownership

Scrolling is delegated, never global. The document body does not scroll.

| Region | Rule |
|---|---|
| Shell root | `h-dvh min-h-0` — never scrolls |
| `SidebarInset` | `min-h-0 min-w-0 overflow-hidden` |
| Content column | a `ScrollArea` with `min-w-0 flex-1` — **the only vertical scroller for page content** |
| Context panel facets | its own `ScrollArea` with `min-h-0 flex-1`; the filter block above it is `shrink-0` and never scrolls away |
| Tables | wrapped in `overflow-x-auto` |
| Full-bleed visualizations | own their overflow (`overflow-auto`) |
| Scrollbars | **hidden globally** — `scrollbar-width: none` + `::-webkit-scrollbar { display: none }`, with `scroll-behavior: smooth` |

`min-w-0` appears on every flex child that contains a table or a chart. Without it, a wide table forces the whole shell to grow and the fixed rails misalign. Treat it as mandatory, not defensive.

### 8-6. Public page layouts (Surface B)

Two shapes, both centered — the opposite of the SPA's edge-to-edge posture.

```
AUTH (login / register / pending)          GUIDE
┌──────────────────────────────┐           ┌──────────────────────────────┐
│  min-h-dvh, grid-center, p-4 │           │ sticky header h-12 z-50      │
│      ┌────────────────┐      │           │ backdrop-blur-xs bg/60 bd-b  │
│      │ max-w-sm 384px │      │           ├──────────────────────────────┤
│      │ p-6, gap-4     │      │           │   main max-w-3xl 768px       │
│      │ rounded border │      │           │   mx-auto px-4 pt-6 pb-20    │
│      │ shadow-sm      │      │           │   flex-col gap-6             │
│      └────────────────┘      │           │   9 numbered sections        │
└──────────────────────────────┘           └──────────────────────────────┘
```

The guide's sticky header is the **only** sticky element in the entire product, and its `h-12` deliberately matches the SPA rail header height.

---

## 9. Breakpoints

### 9-1. Declared scale

Neither theme file overrides `--breakpoint-*`, so **Tailwind v4 defaults apply unchanged**:

| Name | Min width | rem |
|---|---|---|
| `sm` | 640px | 40rem |
| `md` | **768px** | 48rem |
| `lg` | 1024px | 64rem |
| `xl` | 1280px | 80rem |
| `2xl` | 1536px | 96rem |

### 9-2. Actual usage — the product has one breakpoint

Exhaustive count across both surfaces:

| Prefix | Total | In application code | In stock shadcn | In Surface B |
|---|---|---|---|---|
| `sm:` | 20 | **1** (`sm:max-w-2xl` on the form dialog) | 19 | 0 |
| `md:` | 28 | **15** (14 grid-column switches + 1 `md:p-3` that *cancels* shadcn's `md:p-12`) | 13 | 0 |
| `lg:` | **1** | 0 | 0 | 1 (`lg:text-sm` on the code block) |
| `xl:` | **1** | 1 (`xl:grid-cols-3`, one page) | 0 | 0 |
| `2xl:` | **0** | 0 | 0 | 0 |

**`md` (768px) is the only structural breakpoint in the product**, and it does exactly one thing: switch stat-tile grids from 2 columns to 3, 4 or 5. Everything else that reflows does so through `flex-wrap` (40 uses), not media queries.

The JS breakpoint matches: `use-mobile.ts` uses `MOBILE_BREAKPOINT = 768` with `(max-width: 767px)`. Its initial state is `undefined → false`, so **first paint is always desktop-biased** and corrects in an effect.

### 9-3. Container queries

Used once, inside a stock primitive: `@md/field-group` (6 occurrences in `field.tsx`). The application does not author container queries.

### 9-4. Recreator guidance

Build this as a **desktop-first tool with a single 768px break**. Do not manufacture a tablet tier or a wide tier — the source has neither, and adding one would change the density story. If you need the full ladder for your own framework's tooling, declare all five names but leave `sm`, `lg` and `2xl` unused, exactly as the source does.

---

## 10. Component Catalog

### 10-0. How this catalog is scoped

An independent structural diff against the shadcn `new-york-v4` registry established that **all 39 files in the design-system layer are byte-equivalent to stock**, differing only by import-path rewrites, `'use client'` removal, and Prettier formatting. Their class-token multisets are identical to upstream.

**Therefore this catalog documents the 30 project-authored components, not the 39 vendored ones.** The vendored layer is a dependency (mapped in §14), not an original design artifact. Recreating it means re-deriving those primitives in your framework from the token set in §16 — the archetypes below tell you how they are *composed*, which is where this product's design actually lives.

**Thirteen primary archetypes** (§10-1 – §10-12 and §10-14), plus eight supporting patterns collected in §10-13.

---

### 10-1. Panel Card — the surface primitive

**Purpose.** The single content surface of the product. Every block on every page is one of these. It exists to enforce three things at once: no border, no radius, and exactly 12px of inset on both axes.

**Anatomy.**

```
┌─ <section> ────────────────────────────── bg: --color-card ─┐
│                                                              │
│  12px                                                        │
│   ┌─ header (optional) ──────────────────────────────────┐   │
│   │ <h2> title · text-sm · font-medium                   │   │
│   └──────────────────────────────────────────────────────┘   │
│   ↕ 12px                                                     │
│   ┌─ content ────────────────────────────────────────────┐   │
│   │ table · chart · list · form                          │   │
│   └──────────────────────────────────────────────────────┘   │
│  12px                                                        │
└──────────────────────────────────────────────────────────────┘
     ↕ 1px gap  ← --color-background shows through
┌─ next panel ─────────────────────────────────────────────────┐
```

```html
<section class="panel">
  <header class="panel-header"><h2 class="panel-title">활성 인시던트</h2></header>
  <div class="panel-content"><!-- … --></div>
</section>
```

```css
.panel         { background: var(--color-card); border: 0; border-radius: 0;
                 box-shadow: none; display: flex; flex-direction: column;
                 gap: 12px; padding-block: 12px; }
.panel-header,
.panel-content { padding-inline: 12px; }
.panel-title   { font-size: var(--text-sm); font-weight: 500; }

/* the page root that stacks them — this is where separation happens */
.page-root     { display: flex; flex-direction: column; gap: 1px; }
```

**Tokens.**

| Property | Token |
|---|---|
| background | `--color-card` |
| title color | `--color-card-foreground` |
| padding | `12px` both axes |
| internal stack gap | `12px` |
| border / radius / shadow | `0` / `0` / `none` |
| separator | parent's `1px` gap over `--color-background` |

**States.** None. The panel is inert — it has no hover, focus, or active state. Interactivity lives in its contents.

**Do**
- Put every page block inside one.
- Let the parent's `gap: 1px` do all separation.
- Give the content wrapper a modifier class when it needs a different internal layout (`flex flex-col gap-2`, `flex flex-wrap items-center gap-2`).

**Don't**
- Add a border, radius, or shadow — that is the entire point of the component.
- Add `padding-top: 1px` "to line things up". The gap already did it, and you will shift content 1px relative to the fixed rail.
- Nest panels. A panel inside a panel double-insets to 24px and breaks the 268px baseline.

---

### 10-2. Stat Tile — the KPI cell

**Purpose.** A single labelled number in a tile grid. Used for percentiles, counts, rates, health readings.

**Anatomy.**

```
┌─ <article> ──── bg: --color-card · border 0 · p-12px · h-100% ─┐
│  label            text-xs   --color-muted-foreground           │
│  1,284            text-2xl  font-semibold  tabular-nums         │
│                             tracking-tight  [accent]            │
│  hint (optional)  text-xs   --color-muted-foreground           │
└─────────────────────────────────────────────────────────────────┘
```

```html
<article class="tile">
  <p class="tile-label">p95 응답시간</p>
  <p class="tile-value tile-value--warning">482ms</p>
  <p class="tile-hint">히스토그램 http.server.request.duration</p>
</article>
```

```css
.tile        { display: flex; flex-direction: column; align-items: flex-start;
               height: 100%; background: var(--color-card);
               border: 0; padding: 12px; gap: 4px; }
.tile-label,
.tile-hint   { font-size: var(--text-xs); color: var(--color-muted-foreground); }
.tile-value  { font-size: var(--text-2xl); font-weight: 600;
               font-variant-numeric: tabular-nums; letter-spacing: -0.025em; }
.tile-value--warning { color: var(--color-warning); }
.tile-value--danger  { color: var(--color-destructive); }
```

**Accent contract.** Exactly two accents exist, and they are a closed set:

```
accent = 'warning' → --color-warning      (approaching a threshold)
accent = 'danger'  → --color-destructive  (breached)
undefined          → inherit --color-foreground
```

The caller decides by threshold, e.g. `rejected > 0 ? 'warning' : undefined`. **There is no trend indicator, no delta arrow, no percentage-change UI anywhere in this product.** Do not add one.

**States.** Mount fade `opacity 0 → 1` over `0.18s` with the standard curve. No hover, no click — tiles are not interactive.

**Do**
- Give the tile and its wrapper `height: 100%`. A tile with a `hint` is taller than one without; without `h-full` the background shows through the grid gap unevenly and the 1px rhythm visibly breaks.
- Use `tabular-nums` so digits do not jitter between polls.
- Drop the accent when the value is zero.

**Don't**
- Paint a zero red. `0 errors` in `--color-destructive` advertises a problem that does not exist — this is an explicit project rule.
- Pass a raw class name as the accent. The accent is a two-value union mapped internally.

---

### 10-3. Data Table — the ledger

**Purpose.** The dominant content type. Nine of the thirty routes are primarily a table.

**Anatomy.**

```
┌─ scroll container ── overflow-x: auto ───────────────────────────┐
│ ┌─ <table> ── text-xs · width 100% ────────────────────────────┐ │
│ │ TIME      SERVICE   STATUS  ROUTE ················  DURATION │ │  ← th
│ │ w-40      w-24      w-20    flex, max-width:0        w-24    │ │
│ │──────────────────────────────────────────────────── border-b │ │
│ │ 14:32:07  gumba     [500]   /api/orders/checkout…      482ms │ │
│ │ 14:32:05  gumba     [200]   /api/cart                   31ms │ │
│ └──────────────────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────────────┘
```

```html
<div class="table-scroll">
  <table class="data-table">
    <thead>
      <tr>
        <th scope="col" style="width:160px">시각</th>
        <th scope="col" style="width:96px">서비스</th>
        <th scope="col" style="width:80px">상태</th>
        <th scope="col">경로</th>
        <th scope="col" class="num" style="width:96px">소요</th>
      </tr>
    </thead>
    <tbody>
      <tr>
        <td class="mono">14:32:07</td>
        <td>gumba</td>
        <td><span class="badge badge--destructive">500</span></td>
        <td class="flex-cell mono">/api/orders/checkout</td>
        <td class="num mono">482ms</td>
      </tr>
    </tbody>
  </table>
</div>
```

```css
.data-table            { width: 100%; font-size: var(--text-xs); }
.data-table th,
.data-table td         { padding: 8px; vertical-align: middle; }
.data-table th         { font-weight: 500; color: var(--color-muted-foreground); text-align: left; }
.data-table tbody tr   { border-bottom: 1px solid var(--color-border); }
.data-table .num       { text-align: right; font-variant-numeric: tabular-nums; }
.data-table .mono      { font-family: var(--font-mono); }
.data-table .flex-cell { max-width: 0; overflow: hidden;
                         text-overflow: ellipsis; white-space: nowrap; }
.table-scroll          { overflow-x: auto; }
```

**The column width contract.** Every column except one is a fixed pixel width from a seven-value scale — 64, 80, **96**, 112, 128, 160, 224px. The one flexible column takes `max-width: 0` plus ellipsis. This is not a style choice: inside a table cell, `text-overflow: ellipsis` does nothing until the cell is given a max-width it can shrink below. `truncate` alone is inert, and the project shipped that bug once.

**Tokens.** Text `--text-xs`; header `--color-muted-foreground`, weight 500; row rule `--color-border`; cell padding `8px`; the container is a Panel Card (§10-1), so the table sits on `--color-card`.

**States.**

| State | Treatment |
|---|---|
| Row hover | none by default; the waterfall's rows opt in with `--color-accent` |
| Row is the focused record | `background: var(--color-accent)` (log context view) |
| Numeric cell, threshold breached | `--color-destructive` or `--color-warning` on the text |
| Numeric cell, value is zero | `--color-muted-foreground` |
| Cell is a link | `text-decoration: underline` on hover, mono if it is an ID |

**Do**
- Set `font-size` once on the table, never per cell.
- Pair `text-align: right` with `tabular-nums` on every numeric column, always both.
- Wrap in `overflow-x: auto` — the table never causes the shell to grow.

**Don't**
- Let more than one column be flexible.
- Use `divide-y`. Row rules are `border-b` here; the `1px` gap idiom is for *blocks*, not rows.
- Add row striping. There is none in this system.

---

### 10-4. Series Chart — the only chart

**Purpose.** Time-bucketed line series. Every chart in the product is this component; there is no bar chart, area chart, pie, or gauge.

**Anatomy.**

```
┌─ chart container ── height 224px · width 100% ────────────────┐
│ 2.4k ┤                                              ╭──       │  ← y axis, width 44
│      ┤                  ╭───╮                   ╭───╯         │    no line, no tick line
│ 1.2k ┤     ╭────────────╯   ╰──────╮        ╭───╯             │    tickMargin 8
│      ┤ ╭───╯                       ╰────────╯                 │
│    0 ┼──────────────────────────────────────────────────────  │  ← horizontal grid only
│      14:00      14:15      14:30      14:45      15:00        │    strokeOpacity 0.15
│                                                                │  ← x axis, minTickGap 48
│                    ● 전체   ● 4xx   ● 5xx                      │  ← legend
└────────────────────────────────────────────────────────────────┘
   margin: top 4 · right 8 · left 4
```

**Configuration — every value is a hard constraint.**

| Property | Value |
|---|---|
| Chart type | line only; `monotone` interpolation |
| Stroke width | `2` |
| Dots | **off** |
| Null handling | **`connectNulls: false`** — gaps in the data are drawn as gaps |
| **Animation** | **disabled** — `isAnimationActive: false` on every series, no duration set |
| Height | `224px`, width `100%` |
| Margin | `{ top: 4, right: 8, left: 4 }` |
| Grid | horizontal only (`vertical: false`), `stroke-opacity: 0.15` |
| X axis | no axis line, no tick line, `tickMargin: 8`, `minTickGap: 48`, formatted `HH:MM` |
| Y axis | no axis line, no tick line, `tickMargin: 8`, `width: 44`, `≥1000 → "N.Nk"` |
| Tick color | `--color-muted-foreground` |
| Grid line color | `--color-border` at 50% |
| Tooltip surface | `--color-background`, `1px` border `--color-border/50`, `--text-xs`, padding `10px 6px`, the one place `--shadow-xl` is used |
| Tooltip cursor | stroke `--color-border`, fill `--color-muted` |
| Cursor when clickable | `pointer` |

**Series color assignment.**

```
series i → var(--color-chart-((i mod 5) + 1))      // slot cycling, 5 slots
explicit per-series color overrides the slot
```

Slot semantics are in §3-4. In practice: a single-series chart passes `--color-muted-foreground` explicitly and never touches a slot; the HTTP-status chart passes `--color-muted-foreground` / `--color-warning` / `--color-destructive` explicitly.

**States.** Hover shows the tooltip and cursor. Click emits the bucket timestamp, which the page turns into a `from`/`to` range — this is the drill-down mechanism. No loading state of its own; the page renders a skeleton in its place.

**Do**
- Keep `connectNulls: false`. A missing bucket is information — a percentile series with no observations must break, not interpolate a false continuity.
- Keep animation off.
- Pass an explicit color whenever the series has meaning.

**Don't**
- Turn dots on. With `dot: false`, an isolated single bucket surrounded by gaps renders a zero-length path and is **invisible** — a real limitation the project accepted. Turning dots on to fix it changes the chart's character; the accepted answer is denser buckets.
- Add gradient fills. There are no gradients in this system.
- Use a slot color for a semantic series — slot hues rotate between themes (§4-4).

---

### 10-5. Status Badge — the semantic status system

**Purpose.** The product's entire discrete-status vocabulary. Every enum — severity, HTTP class, log level, issue state, delivery result, enablement — renders as one of four badge variants.

**Anatomy.**

```html
<span class="badge badge--destructive">500</span>
```

```css
.badge { display: inline-flex; align-items: center; justify-content: center;
         gap: 4px; width: fit-content; flex-shrink: 0; overflow: hidden;
         border-radius: 9999px; border: 1px solid transparent;
         padding: 2px 8px; font-size: var(--text-xs); font-weight: 500;
         white-space: nowrap; transition: color .15s, box-shadow .15s; }

.badge--default     { background: var(--color-primary);     color: var(--color-primary-foreground); }
.badge--secondary   { background: var(--color-secondary);   color: var(--color-secondary-foreground); }
.badge--destructive { background: var(--color-destructive); color: oklch(1 0 0); }
                                        /* ↑ the source uses a literal white here, NOT
                                           --color-destructive-foreground. In dark mode the token
                                           is oklch(0.9851 0 0); the badge stays pure white. */
.badge--outline     { border-color: var(--color-border);    color: var(--color-foreground); }
```

> Badges are **`border-radius: 9999px`** — the one intentionally round thing in a product whose radius token is `0`. It is stock shadcn and was left alone.

**The semantic map — this is the contract.**

| Domain | `destructive` | `outline` | `secondary` |
|---|---|---|---|
| Alert / rule / incident severity | `critical` | — | everything else |
| Incident lifecycle | — | open | resolved |
| **HTTP status class** | **≥ 500** | **400–499** | 2xx/3xx, or unknown |
| **Log severity** | severityNumber **≥ 17** (ERROR) | — | below 17 |
| Issue status | `unresolved` | — | resolved |
| Live event | `error` | — | ok |
| Service freshness | stale | — | live |
| Cardinality budget | over budget | — | ok |
| Instrumentation finding | severity above `low` | — | `low` |
| Delivery result | `failed` | pending | `sent` |
| Signal arrival | count = 0 (nothing arrived) | — | count > 0 |
| Approval | rejected | pending | approved |
| **Enablement (≈10 sites)** | — | disabled | **enabled** |

**Three rules fall out of this table, and they define the product's color language:**

1. **There is no success color in the dashboard.** Healthy is `secondary` — a neutral gray fill — or plain `--color-muted-foreground`. Green never means "good" here; `--color-chart-2` is green but is reserved for the vitals ladder.
2. **`outline` is the "intermediate / pending / off" state.** Not a visual variant — a semantic tier between neutral and alarming.
3. **`destructive` is the only alarm.** `--color-warning` never appears on a badge; it is a *text* color for numeric thresholds (§10-3).

> **Server-side definition of "error span"**, which the waterfall consumes: status code `error`, OR HTTP ≥ 500, OR HTTP 400–499 **only when the span is a client span**. A 4xx on an inbound server span is not an error. Encode this rule, not a naive `>= 400`.

> **Surface B forks here.** Its badge set adds `warning` (`--color-chart-4` at 20% over `--color-foreground`) and `success` (`--color-chart-2` at 20%), and drops `ghost`/`link`. The dashboard has neither tinted variant. This fork is unresolved in source; a recreator should pick one and say so.

**States.** Focus-visible ring per §6-6. Link badges get a hover background. Otherwise inert.

**Do**
- Map every enum through the table above.
- Keep the badge label as the raw machine value (`500`, `ERROR`, `unresolved`) rather than a prose translation, so it matches log output.

**Don't**
- Introduce green for success.
- Use `--color-warning` on a badge — it is a text-threshold color.

---

### 10-6. Bar-Count List — the facet primitive

**Purpose.** A list of values with counts and a proportional bar. This is how the product does distribution: in the context panel's facets, the field-comparison view, and the vitals breakdown.

**Anatomy.**

```
SERVICE                                      ← group label, text-2xs, uppercase, tracking-wide
  gumba                          1,284   ⊖   ← value (mono, flex, truncate) · count · exclude
  ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓░░░░░░░░░░░           ← 1px track, fill = --color-foreground/40
  pawa-up                          312   ⊖
  ▓▓▓▓▓░░░░░░░░░░░░░░░░░░░░░░░░░░░░░
```

```html
<section class="facet-group">
  <h3 class="facet-label">service</h3>
  <div class="facet-row">
    <div class="facet-line">
      <button class="facet-value">gumba</button>
      <span class="facet-count">1,284</span>
      <button class="facet-exclude" aria-label="gumba 제외">–</button>
    </div>
    <div class="facet-track"><div class="facet-fill" style="width:72%"></div></div>
  </div>
</section>
```

```css
.facet-group   { display: flex; flex-direction: column; gap: 6px; }
.facet-label   { font-size: var(--text-2xs); font-weight: 500;
                 text-transform: uppercase; letter-spacing: 0.025em;
                 color: var(--color-muted-foreground); }
.facet-line    { display: flex; align-items: center; gap: 8px; font-size: var(--text-xs); }
.facet-value   { min-width: 0; flex: 1; text-align: left; font-family: var(--font-mono);
                 overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.facet-value:hover { text-decoration: underline; }
.facet-count   { flex-shrink: 0; font-variant-numeric: tabular-nums;
                 color: var(--color-muted-foreground); }
.facet-exclude { flex-shrink: 0; opacity: 0; transition: opacity .15s;
                 color: var(--color-muted-foreground); }
.facet-row:hover .facet-exclude { opacity: 1; }
.facet-exclude:hover            { color: var(--color-destructive); }
.facet-track   { height: 1px; width: 100%; background: var(--color-border); }
.facet-fill    { height: 100%; background: color-mix(in oklch, var(--color-foreground) 40%, transparent);
                 transition: width .24s cubic-bezier(.4,0,.2,1); }
```

**Three instances, one pattern.**

| Instance | Track height | Fill | Notes |
|---|---|---|---|
| **Facet list** (leader) | **1px** | `--color-foreground` @ 40% | single series |
| **Comparison field** | **2px**, two stacked bars in a `1px` gap | selected `--color-chart-1`; baseline `--color-muted-foreground` | adds a header row with a score |
| **Vitals distribution** | **6px**, one track, three segments | good `--color-chart-2`, needs-improvement `--color-chart-4`, poor `--color-destructive` | the only three-step ramp in the product |

> **A 1px bar cannot show two colors.** The comparison field originally used the facet list's `1px` track and the two series were visually indistinguishable, even though measurement confirmed the widths and colors were correct — a bug that only a screenshot could catch. **Single series: 1px is enough. Two or more series: 2px minimum.**

**States.** The exclude control is `opacity: 0` until the row is hovered, then reveals and turns `--color-destructive` on its own hover. Bar fill animates width `0 → n%` over `0.24s` on mount.

**Do**
- Give every icon-only control an `aria-label` — the label is the only affordance a screen reader gets.
- Animate width from 0 on mount; it is the product's second and last motion pattern.

**Don't**
- Make the exclude control visible at rest. The list is dense; permanent affordances turn it into noise.
- Use a 1px track for more than one series.

---

### 10-7. Trace Waterfall — the deepest custom component

**Purpose.** Renders a span tree as an indented ledger plus a proportional timeline. The most information-dense object in the product.

**Anatomy.**

```
│←── 256px ──→│← 96 →│←80→│←64→│←────── timeline lane, flex ──────→│
  NAME          SVC   DUR  SELF
▾ GET /checkout  gumba 482ms 12ms  ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
  ▾ db.query     gumba 310ms 310ms      ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
    ─ log line                              ┃                      ← 2px tick
  ▾ http POST /pay [!] pay  140ms  140ms                 ▓▓▓▓▓▓▓
      ↑ issue badge, destructive
  indent = depth × 12px, applied INSIDE the name cell
```

```html
<div class="wf-row">
  <div class="wf-name" style="padding-left:24px">
    <button class="wf-toggle" aria-expanded="true" aria-label="접기">▾</button>
    <a class="wf-link wf-link--error" href="/errors/42">db.query</a>
    <span class="badge badge--destructive wf-issue">#12</span>
  </div>
  <div class="wf-service">gumba</div>
  <div class="wf-duration">310ms</div>
  <div class="wf-self">310ms</div>
  <div class="wf-lane">
    <div class="wf-bar wf-bar--error" style="left:18%;width:64%"></div>
    <div class="wf-self-seg wf-self-seg--error" style="left:18%;width:64%"></div>
  </div>
</div>
```

```css
.wf-row      { display: flex; align-items: center; gap: 12px; padding: 4px; }
.wf-row:hover{ background: var(--color-accent); }
.wf-name     { width: 256px; flex-shrink: 0; display: flex; min-width: 0;
               align-items: center; gap: 4px; }
.wf-service  { width: 96px;  flex-shrink: 0; }
.wf-duration { width: 80px;  flex-shrink: 0; text-align: right;
               font-size: var(--text-xs);  font-variant-numeric: tabular-nums; }
.wf-self     { width: 64px;  flex-shrink: 0; text-align: right;
               font-size: var(--text-2xs); font-variant-numeric: tabular-nums;
               color: var(--color-muted-foreground); }
.wf-lane     { position: relative; height: 8px; min-width: 0; flex: 1;
               background: var(--color-muted); }
.wf-bar      { position: absolute; height: 100%;
               background: color-mix(in oklch, var(--color-muted-foreground) 40%, transparent); }
.wf-bar--error      { background: color-mix(in oklch, var(--color-destructive) 40%, transparent); }
.wf-self-seg        { position: absolute; height: 100%; background: var(--color-muted-foreground); }
.wf-self-seg--error { background: var(--color-destructive); }
.wf-link     { font-family: var(--font-mono); font-size: var(--text-xs);
               overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.wf-link--error { color: var(--color-destructive); }
.wf-toggle   { width: 16px; height: 16px; flex-shrink: 0; }
```

**The two-layer bar.** Each span draws **two** overlaid bars: total duration at **40% opacity**, and the critical-path self-time segments at **full opacity**. The result is a monotone two-step depth cue plus one semantic hue — exactly three visual values in the whole timeline:

| Layer | Normal | Error span |
|---|---|---|
| Total duration | `--color-muted-foreground` @ 40% | `--color-destructive` @ 40% |
| Self time (critical path) | `--color-muted-foreground` | `--color-destructive` |
| Lane track | `--color-muted` | same |
| Log tick mark | `--color-muted-foreground`, 2px wide | same |

**Indentation is inside the name cell.** Depth is `depth × 12px` of `padding-left` on the name cell only. The four ledger columns are fixed-width, so **the timeline lane starts at the same x for every row regardless of depth** — which is what lets the top ruler (`0ms · mid · total`) line up with the bars. Indenting the whole row would break the axis.

**Minimum bar width is `0.5%`** so a sub-millisecond span is still visible.

**States.**

| State | Treatment |
|---|---|
| Row hover | `background: var(--color-accent)` |
| Collapsed / expanded | disclosure glyph swaps; collapse state is a **URL parameter**, not local state |
| Leaf (no children) | a `16px` spacer replaces the toggle so names stay aligned |
| Error span | name text, both bars turn `--color-destructive` |
| Has issue | a `destructive` badge, `padding-inline: 4px`, `--text-2xs` |
| Span event / exception | a nested disclosure with a mono `--text-2xs` trigger and a stack trace inside |

**Do**
- Put the indent in the name cell.
- Keep collapse state in the URL so a waterfall view is shareable.
- Enforce the minimum bar width.

**Don't**
- Indent the row. The axis will drift with depth.
- Rely on the bar alone to signal error — the name text carries the color too.

---

### 10-8. Attribute Tree — recursive key/value inspector

**Purpose.** Renders nested telemetry attributes. Every leaf is actionable: click the value to filter by it, or use the two micro-controls to exclude it or promote it to a table column.

**Anatomy.**

```
▸ http (4)                          ← branch: mono, text-xs, muted, count at 60% opacity
  │ request.method   GET      ⊟ ⊖   ← leaf: label muted · value link · add-column · exclude
  │ response.status  500      ⊟ ⊖
  │ ▸ url (2)
  │   │ path         /api/pay ⊟ ⊖
  ↑ 1px left border + 12px padding per nesting level
```

```css
.attr-level     { display: flex; flex-direction: column; }
.attr-level--nested { border-left: 1px solid var(--color-border); padding-left: 12px; }
.attr-leaf      { display: flex; align-items: baseline; gap: 8px;
                  padding-block: 2px; font-family: var(--font-mono); font-size: var(--text-xs); }
.attr-key       { flex-shrink: 0; color: var(--color-muted-foreground); }
.attr-value     { text-align: left; word-break: break-all; }
.attr-value:hover { text-decoration: underline; }
.attr-action    { flex-shrink: 0; color: var(--color-muted-foreground); }
.attr-exclude:hover { color: var(--color-destructive); }
.attr-addcol:hover  { color: var(--color-foreground); }
.attr-branch    { display: flex; align-items: center; gap: 4px;
                  font-family: var(--font-mono); font-size: var(--text-xs);
                  color: var(--color-muted-foreground); }
.attr-chevron   { width: 12px; height: 12px; transition: transform .15s; }
.attr-branch[aria-expanded="true"] .attr-chevron { transform: rotate(90deg); }
.attr-count     { opacity: .6; }
```

**Tokens.** Icons are `12px`. Indentation is a `1px` left rule plus `12px` padding — the one place in the product where a border *is* the separator, because a background tier would be wrong at this scale.

**States.** Branch open/closed rotates the chevron 90°. Value and both actions underline or shift color on hover. The exclude action goes `--color-destructive`.

**Do**
- Give both micro-controls `aria-label`s. (One of them shipped without and was caught in audit.)
- Use a real disclosure element so keyboard and screen-reader semantics come free.

**Don't**
- Show the actions only on hover here. Unlike the facet list, these are always visible — the tree is an inspection surface, not a scanning surface.

---

### 10-9. Form Dialog — the CRUD shell

**Purpose.** One modal shell used by **all thirteen** admin create/edit forms. Documenting it once documents the whole admin surface.

**Anatomy.**

```
        ┌─ overlay: black @ 50% ────────────────────────────┐
        │   ┌─ dialog ── max-w 672px · max-h 85svh ──────┐  │
        │   │  Title                                     │  │
        │   │  Description (optional, muted)             │  │
        │   │                                            │  │
        │   │  ┌ field ─────────────────────────────┐    │  │
        │   │  │ label                              │    │  │
        │   │  │ [ input · height 36px ]            │    │  │
        │   │  │ description (muted, text-xs)       │    │  │
        │   │  │ error (destructive, text-xs)       │    │  │
        │   │  └────────────────────────────────────┘    │  │
        │   │   ↕ 12px between fields                    │  │
        │   │  submit error (destructive, text-xs)       │  │
        │   │                          [취소] [저장]      │  │
        │   └────────────────────────────────────────────┘  │
        └───────────────────────────────────────────────────┘
```

```html
<div class="overlay" data-state="open"></div>
<div role="dialog" aria-modal="true" aria-labelledby="dlg-title" class="dialog">
  <h2 id="dlg-title" class="dialog-title">스크러빙 규칙 추가</h2>
  <p class="dialog-desc">본문에서 일치하는 값을 마스킹합니다.</p>
  <form class="dialog-form" novalidate>
    <div class="field">
      <label class="field-label" for="scrub-name">규칙 이름</label>
      <input id="scrub-name" class="input" aria-invalid="true" aria-describedby="scrub-name-err">
      <p class="field-desc">목록에서 이 이름으로 표시됩니다.</p>
      <p id="scrub-name-err" class="field-error">필수 입력입니다</p>
    </div>
    <p class="form-error">같은 이름의 규칙이 이미 있습니다</p>
    <footer class="dialog-footer">
      <button type="button" class="btn btn--ghost btn--sm">취소</button>
      <button type="submit" class="btn btn--sm" disabled>저장</button>
    </footer>
  </form>
</div>
```

```css
.overlay       { position: fixed; inset: 0; z-index: 50; background: oklch(0 0 0 / 50%); }
.dialog        { position: fixed; top: 50%; left: 50%; transform: translate(-50%,-50%);
                 z-index: 50; width: calc(100% - 2rem); max-width: 672px;
                 max-height: 85svh; overflow-y: auto;
                 background: var(--color-background); border-radius: 0;
                 box-shadow: var(--shadow-lg); padding: 16px;
                 display: flex; flex-direction: column; gap: 16px; }
.dialog-form   { display: flex; flex-direction: column; gap: 12px; }
.field         { display: flex; flex-direction: column; gap: 6px; }
.field-desc    { font-size: var(--text-xs); color: var(--color-muted-foreground); }
.field-error,
.form-error    { font-size: var(--text-xs); color: var(--color-destructive); }
.dialog-footer { display: flex; justify-content: flex-end; gap: 8px; }
```

**Two error channels, and both are required.**

| Channel | Source | Placement |
|---|---|---|
| **Field error** | client-side schema validation | directly below its own input, bound by `aria-describedby`, with focus moved to the first offending field on submit |
| **Form error** | server response code mapped to a human sentence | above the footer |

> **This is a fix, not a nicety.** The thirteen dialogs originally rendered only the form-level channel, so a client-side validation failure produced **no request and no message** — the submit button simply did nothing. Both channels must exist. And when the field channel was added, untranslated library defaults immediately leaked into the UI, so **every message must be explicitly authored**, not inherited from the validation library.

> **Native constraints silently block submit.** An `<input type="number" max="1">` refuses to submit and never runs your validator, so no message appears anywhere. Do validation in one layer — either native or schema — never split across both.

**States.** Trigger is a small button with a `+` icon, `outline` variant when editing an existing record and `default` when creating. While submitting, **only the submit button is disabled** — there is no spinner in this product. Cancel is always enabled. Escape and overlay click close.

**Do**
- Move focus to the first invalid field.
- Author every validation message in the product's language.
- Use a two-column field grid only when the form exceeds ~10 fields (one of the thirteen does).

**Don't**
- Add a loading spinner. `disabled` is the entire loading vocabulary here.
- Rely on the form-level channel for field errors.

---

### 10-10. State Triad — empty, load error, query error

**Purpose.** Three visually identical cards distinguished only by icon and copy. The distinction is semantic and the product treats it as load-bearing.

**Anatomy.**

```
┌─ panel ── bg: --color-card · padding 12px ───────────┐
│                                                       │
│                      ⊙                                │  ← icon, 24px
│                                                       │
│              검색식을 해석할 수 없습니다                 │  ← title
│         목록의 괄호가 닫히지 않았습니다 (mono)           │  ← description
│                                                       │
└───────────────────────────────────────────────────────┘
```

```css
.state-card { background: var(--color-card); padding: 12px;
              display: flex; flex-direction: column; align-items: center;
              justify-content: center; gap: 24px; text-align: center;
              text-wrap: balance; border: 1px dashed transparent; }
```

| Variant | Icon | Icon color | Title | Description | Means |
|---|---|---|---|---|---|
| **Empty** | inbox | inherit | *"…가 없습니다"* | — | the query succeeded and returned nothing |
| **Load error** | triangle-alert | **`--color-destructive`** | *"…를 불러오지 못했습니다"* | *"요청이 실패했습니다. 잠시 후 다시 시도하세요."* | the request failed |
| **Query error** | circle-alert | inherit | *"검색식을 해석할 수 없습니다"* (fixed) | the parser's message, **mono** | the request succeeded and reported that **the user's query is malformed** |

> **Only the load-error icon is colored.** Empty is not a problem, and a bad query is the user's typo — neither earns an alarm color. This restraint is the whole point of splitting three states that could have been one.

> **Query error is not an HTTP error.** It arrives inside a `200` payload as a `queryError` field. A recreator collapsing it into the load-error path will show "the request failed" for what is actually a syntax hint.

**Loading is a skeleton, never a spinner.** Four sizes, by context: page `384px`, table `256px`, panel `96px`, facet row `24px`. `spinner` exists in the vendored layer and is imported by **zero** files.

**Do**
- Keep the three states distinct.
- Match the skeleton to the shape it replaces where you can — one page renders five tile-sized skeletons inside the real grid so the layout does not jump. Every other page shows one slab and jumps. Prefer the former.

**Don't**
- Use a spinner.
- Show an empty state while loading. Guard on the pending flag first — otherwise a filter change flashes "no results" between requests, because the cache key changed and data is briefly undefined.

---

### 10-11. Navigation Rail — the left column

**Purpose.** Flat, full-bleed navigation plus identity and account controls. It replaces the header the product deliberately does not have.

**Anatomy.**

```
┌── 256px ──────────────┐    collapsed: 48px
│ [⇤]  flunti-otel      │ 48px header — toggle centered on the 48px icon rail
├───────────────────────┤
│ ⚡ 트래픽              │ 36px, full-bleed, no radius, no gap
│ ⚠ 에러                │
│ ⟿ 트레이스             │
│ 🐞 이슈                │
│ …  (18 items, flat)   │
│ 🔔 알림          [3]  │ ← live badge; destructive when critical
├───────────────────────┤
│ hyunseok    ☀  ⏻     │ 48px footer — collapses to a vertical icon stack
└───────────────────────┘
  ↑ no right border
```

```css
.rail            { width: 256px; background: var(--color-sidebar);
                   color: var(--color-sidebar-foreground);
                   border-right: 0; position: fixed; inset-block: 0; left: 0; z-index: 10;
                   transition: width .2s linear; }
.rail[data-collapsed="true"] { width: 48px; }
.rail-header,
.rail-footer     { height: 48px; display: flex; align-items: center; padding: 0; }
.rail-title      { font-size: var(--text-sm); font-weight: 600; letter-spacing: -0.025em;
                   overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.rail-menu       { display: flex; flex-direction: column; gap: 0; padding: 0; }
.rail-item       { height: 36px; display: flex; align-items: center; gap: 12px;
                   padding-inline: 12px; border-radius: 0;
                   font-size: var(--text-sm); font-weight: 500; }
.rail-item:hover,
.rail-item[aria-current="page"] { background: var(--color-sidebar-accent);
                                  color: var(--color-sidebar-accent-foreground); }
.rail-badge      { margin-left: auto; font-variant-numeric: tabular-nums; }
.rail-badge--critical { color: var(--color-destructive); }
```

**Full-bleed is the rule.** Zero group padding, zero menu gap, zero item radius. Left at their defaults, active items float as inset pills — which reads as a different, softer product.

**Collapse behavior.** At `48px`, labels are hidden **explicitly** (`[&>span]:hidden`), not by overflow clipping — clipping lets the first glyph bleed. The item keeps `height: 36px` and `width: 100%` so its icon centers on the same 24px axis as the header toggle.

**States.**

| State | Treatment |
|---|---|
| Hover / active | `--color-sidebar-accent` fill |
| Active match | **exact pathname equality** — `/errors/42` does *not* light up `/errors` |
| Collapsed | label hidden, badge hidden, count moves into the tooltip string |
| Alert badge, critical | `--color-destructive` text |
| Mobile (< 768px) | the rail is `display: none`; navigation moves into an overlay drawer, `288px`, over a 50% black scrim |
| Toggle | `Cmd/Ctrl + B`; state persisted in a cookie for 7 days |

**Navigation preserves the query string.** Every link carries the current `search` forward, so service and time-range selections survive navigation. This is a product behavior, not a styling one, but it is why the rail has no filter controls of its own.

**Do**
- Zero the right border with the *same variant selector* the framework used to add it (§6-4).
- Keep the item list flat — 18 items, one group, no labels, no nesting.

**Don't**
- Change item height without also restoring the collapsed-width constraint. Overriding `height` alone can drop the width rule through class-merge conflict, and labels then spill out of the 48px rail.
- Add a header. There isn't one, by decision.

---

### 10-12. Context Filter Panel — the right column

**Purpose.** The product's global query surface: what service, what time window, what search expression, and what facets. Always visible, never collapsible.

**Anatomy.**

```
┌── 320px ─── bg: --color-sidebar ────┐
│ ┌ fixed block · padding 12px ─────┐ │  ← never scrolls
│ │ [ service        ▾ ] 32px       │ │
│ │ [ range          ▾ ] 32px       │ │
│ │ [ from datetime  ] 32px         │ │
│ │ [ to   datetime  ] 32px         │ │
│ │ [ 기간 적용 ][ 기간 해제 ] 28px  │ │
│ │ [ status:500 -route:/health ]   │ │  ← mono, 32px
│ │ [ traceId 점프              ]   │ │
│ └─────────────────────────────────┘ │
│ ┌ scrolling block ────────────────┐ │
│ │ ▾ FACETS                  [812] │ │
│ │   service · route · status …    │ │  ← Bar-Count List §10-6
│ └─────────────────────────────────┘ │
└──────────────────────────────────────┘
```

**The split is the design.** The filter block is `flex-shrink: 0`; only the facet block scrolls, and only the facet block collapses. Putting the global filters behind a collapse would make service, time range and search expression unreachable — that is a loss of function, not a space saving. The panel as a whole has **no toggle**: it is 320px, always.

**Tokens.** Panel background `--color-sidebar` — the same tier as the left rail, making the two columns symmetric. Controls are `32px` tall; the apply/clear pair is `28px`. The search input is mono at `--text-xs`. The facet section eyebrow is `--text-2xs`, uppercase, `tracking-wide`, muted.

**States.** The panel is route-aware: on log routes it shows log facets, elsewhere error facets. Its own loading/empty/error vocabulary is **inline text**, not the state-triad cards — six `24px` skeleton rows while pending, a `--text-xs` `--color-destructive` sentence on failure, a muted sentence when empty. This is a deliberate down-weighting: the panel is a sidecar and must not present like page content.

**Do**
- Keep the filter block pinned and the facet block scrollable.
- Use inline micro-states here, not the full state cards.

**Don't**
- Add a collapse toggle for the whole panel.
- Hide it below 768px without also reclaiming its width — see the known gap in §13.

---

### 10-13. Supporting patterns

Eight further authored components. Each is a thin composition; anatomy is inherited from an archetype above.

| Component | Composition | Distinguishing detail |
|---|---|---|
| **Admin Table** | Panel Card + Data Table + State Triad | Encapsulates the whole state ladder behind a column descriptor `{ key, label, width, align, cell }`. Right-aligned columns automatically gain `tabular-nums`. This is why eleven admin tabs contain no state logic. |
| **Admin Section** | Toolbar Panel Card + Admin Table | Toolbar is `flex-wrap items-center gap-2` holding the create trigger and a muted `--text-xs` hint sentence. |
| **Pager** | button pair, `28px`, ghost | Two dialects: **keyset** (prev/next only) and **offset** (adds `page / total` in `--text-2xs tabular-nums` muted). Disabled is `opacity: .5` + `pointer-events: none` — never a faked anchor. |
| **Column Toggle** | multi-select chip group, `flex-wrap` | Selected state is `--color-accent` fill. Feeds a `cols=` URL parameter, including dynamic `attr.*` columns discovered from data. |
| **Saved View Bar** | outline badge row, `flex-wrap` | Each badge packs up to four controls: name (navigates), lock glyph (private), share toggle (owner only), promote-to-alert (admin only), delete (hover → `--color-destructive`). All glyphs `12px`. |
| **Confirm Action** | modal with a destructive confirm | The only place the button variant generator is invoked directly, to style the confirm slot as `destructive`. Cancel is always the ghost on the left. |
| **Stack Trace** | mono block on `--color-muted`, `12px` padding | Parsed frames list in-app frames plainly with a `secondary` badge; consecutive vendor frames collapse behind a muted *"N개 프레임 숨김"* disclosure. If the parser yields zero frames it falls back to a raw `<pre>` — **the fallback is the parse-failure signal.** |
| **Issue Event Item** | disclosure row, mono `--text-xs` | Signals enrichment by *un-muting*: a symbolicated event's timestamp is `--color-foreground` instead of `--color-muted-foreground`. Body holds an action row and a raw-JSON `<pre>` at `--text-2xs`. |

---

### 10-14. Service Map Graph — hand-drawn SVG

**Purpose.** A layered service dependency graph. Not a library — a topological layout computed in code and emitted as inline SVG.

**Geometry.** All values are absolute pixels; the SVG's intrinsic `width`/`height` are computed from them and the wrapper scrolls in both axes.

```
LAYER_GAP 220   ROW_GAP 72    NODE_WIDTH 176   NODE_HEIGHT 40   PADDING 24
NODE_LABEL_FONT_SIZE 11       EDGE_LABEL_FONT_SIZE 10           NODE_LABEL_MAX_CHARS 24
EDGE_STROKE_WIDTH 1.5         NODE_STROKE_WIDTH 1.5
EDGE_LABEL_WIDTH 108          EDGE_LABEL_HEIGHT 32
ARROW_MARKER_SIZE 7           EXTERNAL_DASH '4 3'
```

Layout: Kahn topological layering → alphabetical sort within a row → rows centered vertically.

**Color contract.** This component addresses tokens **directly as CSS custom properties** — it is the one place in the product where that is correct, because SVG presentation attributes cannot take utility classes.

| Element | Value |
|---|---|
| Node fill | `var(--color-card)` |
| Node stroke — has error | `var(--color-destructive)` |
| Node stroke — external | `var(--color-chart-3)` + `stroke-dasharray: 4 3` |
| Node stroke — internal service | `var(--color-chart-1)` |
| Node label | `var(--color-foreground)` |
| Edge + arrowhead — error | `var(--color-destructive)` |
| Edge + arrowhead — normal | `var(--color-muted-foreground)` |
| Edge label backing | `var(--color-card)` |
| Edge label line 1 (rate, p99) | `var(--color-foreground)` |
| Edge label line 2 (error %) | error → `var(--color-destructive)`, else `var(--color-muted-foreground)` |

A node is in the error state if **any** incident edge, inbound or outbound, carries a nonzero error count.

**Accessibility.** `role="img"` with a text `aria-label`. The graph is not keyboard-navigable — a known limitation.

**Do**
- Address tokens as `var(--color-*)` in SVG attributes.
- Dash external nodes; the dash is the only thing distinguishing "not our service" besides hue, and hue alone is insufficient.

**Don't**
- Introduce a graph library. The layout is deterministic and the output is 200 lines of SVG.
- Use utility classes for SVG fills and strokes.

---

## 11. Page Patterns

Thirty routes reduce to **seven page archetypes**. Every page root is the same element:

```html
<div class="page-root"><!-- flex column, gap: 1px --></div>
```

with two exceptions where the root *is* the tab container (still `flex column, gap: 1px`).

### 11-A. Signal List — leader: the errors route

Nine routes: errors, logs, traces, issues, perf-issues, services, live, log-patterns, log-context.

```
┌─ toolbar panel ───────────────────────────────────────┐
│ [column toggle chips]              [CSV] [source ▾]   │
└───────────────────────────────────────────────────────┘ ↕1px
┌─ saved view bar panel ────────────────────────────────┐
│ 저장된 뷰  (badge)(badge)  [name input][공유][저장]     │
└───────────────────────────────────────────────────────┘ ↕1px
┌─ result panel ────────────────────────────────────────┐
│ ┌ table ──────────────────────────────────────────┐   │
│ └─────────────────────────────────────────────────┘   │
│              [◀ 이전]  [다음 ▶]                        │
└───────────────────────────────────────────────────────┘
```

**The four-state block is inline, not an early return** — the toolbar must stay usable while the result region is pending, empty, or erroring. Order of the guards matters: pending → transport error → **query-syntax error** → empty → results.

Two pager dialects: keyset for the two high-volume signals, offset elsewhere. All list state (filters, columns, cursor, saved view) is URL-serialized.

### 11-B. Stacked Detail — leader: the issue detail route

Six routes: issue, error, log, incident, alert-rule, perf-issue details.

```
┌─ identity panel ──────────────────────────────────────┐
│ [◀ 목록]                                               │
│ TypeError: cannot read property 'id' of undefined     │
│ (badge)(badge)(badge)                                 │
│ 최초 14:02 · 최근 14:47 · 릴리스 2.0.0   ← muted meta  │
└───────────────────────────────────────────────────────┘ ↕1px
┌─ tile grid: 2 cols → 4 cols at 768px ─────────────────┐
│ [tile] [tile] [tile] [tile]                           │
└───────────────────────────────────────────────────────┘ ↕1px
┌─ chart panel ─────────────────────────────────────────┐ ↕1px
┌─ related-records panel (repeats) ─────────────────────┐
```

**There is no split view anywhere in this product.** No two-column detail, no sticky detail sidebar, no master-detail pane. Detail is always a single vertical stack, because the right column is already occupied by the global filter panel.

The identity panel is consistent: back link, then title, then a badge row, then a `flex-wrap` muted metadata row at `--text-xs`.

### 11-C. Metric Overview — leader: the traffic route

Six routes: traffic, metrics, rum, cache, system, diagnostics.

```
┌─ tile grid: 2 → 4 or 5 at 768px ──────────────────────┐
│ [tile][tile][tile][tile][tile]                        │
└───────────────────────────────────────────────────────┘ ↕1px
┌─ chart panel — 224px ─────────────────────────────────┐ ↕1px
┌─ breakdown table panel (repeats) ─────────────────────┐
│  …table…                                              │
│  히스토그램이 없는 서비스는 span 정렬로 계산합니다     │ ← panel footnote
└───────────────────────────────────────────────────────┘
```

**The panel footnote** is a recurring element worth naming: a `--text-xs` `--color-muted-foreground` sentence below a table or chart explaining a caveat about the data — how a value was derived, what is excluded, why a row might be missing. It is a distinguishing habit of this product.

Multi-query dashboards use **per-panel** states: the primary query gates the page, then each secondary panel resolves independently, with a `96px` skeleton and a bare `--text-xs` `--color-destructive` sentence instead of a full error card.

### 11-D. Tabbed Console — leader: the admin route

Two routes: admin (11 tabs), alerts (3 tabs).

```
┌─ tab bar panel ── list wraps at narrow widths ────────┐
│ [무음][보존][라우팅][채널][스크러빙][서비스][토큰]…    │
└───────────────────────────────────────────────────────┘ ↕1px
┌─ tab body — re-enters archetype A or E ───────────────┐
```

The page root **is** the tab container, so the tab bar and the body participate in the same 1px rhythm as any other block. The tab list lives inside a Panel Card so its triggers align to the 268px baseline like everything else.

### 11-E. Admin CRUD — leader: the alert-rules route

Three routes plus every admin tab body.

```
┌─ toolbar panel ───────────────────────────────────────┐
│ [+ 규칙 추가]   규칙은 60초마다 평가됩니다  ← muted hint │
└───────────────────────────────────────────────────────┘ ↕1px
┌─ result panel — "규칙 4개" ───────────────────────────┐
│ NAME        TARGET   SEVERITY  ENABLED       ACTIONS  │
│ 5xx 급증    issue    [critical] [사용]  [중지][✎][🗑] │
└───────────────────────────────────────────────────────┘
```

The action column is a `gap: 4px` cluster of small ghost buttons: toggle, edit (opens the Form Dialog pre-filled), delete (opens the Confirm Action modal). The panel title carries the count.

### 11-F. Query Console — leader: the explore route

Two routes: explore, field comparison.

```
┌─ control panel ── flex column, gap 12px ──────────────┐
│ [ signal tabs ]                                       │
│ [ metric tabs ]                                       │
│ [ dimension chips — flex-wrap, justify-start ]        │
└───────────────────────────────────────────────────────┘ ↕1px
┌─ result: chart, table, or card grid ──────────────────┐
```

The field-comparison route holds the product's **only three-tier grid**: 1 column → 2 at 768px → 3 at 1280px, with a `24px` gap rather than the usual 1px, because comparison cards are read as separate objects rather than as a continuous ledger.

Every control writes to the URL. Reloading the page reproduces the view exactly.

### 11-G. Full-bleed Visualization — leader: the trace route

Two routes plus the waterfall half of the perf-issue detail.

```
┌─ panel ───────────────────────────────────────────────┐
│  0ms                    240ms                   482ms │ ← ruler, aligned to the lane
│  ▾ GET /checkout   gumba  482ms  12ms  ▓▓▓▓▓▓▓▓▓▓▓▓▓  │
│    ▾ db.query      gumba  310ms 310ms      ▓▓▓▓▓▓▓    │
└───────────────────────────────────────────────────────┘
```

The visualization owns its overflow and ignores breakpoints entirely. It scrolls; it does not reflow.

### 11-H. Public pages (Surface B)

| Pattern | Shape |
|---|---|
| **Auth** | `min-height: 100dvh`, centered both axes, `16px` page padding, a `384px` card with `24px` padding, `16px` internal gap, real border, `6px` radius, `--shadow-sm`. Heading `--text-xl` `font-extrabold`. Error message `--text-sm` `--color-destructive`. |
| **Pending approval** | Identical shell, center-aligned text, a single full-width sign-out button. |
| **Guide** | `48px` sticky translucent header (`backdrop-filter: blur`, background at 60%, bottom border, `z-index: 50`) over a `768px` centered column with `24px` page padding, `24px` section gaps, and an `80px` bottom runway. `h1` is `--text-3xl` `font-extrabold`; sections are `h2` at `--text-xl` `font-semibold`; prose is `--text-sm` `--color-muted-foreground` with `line-height: 1.75`. |

The guide's code blocks are **theme-invariant on purpose** — a fixed dark surface with light text in both light and dark mode, so a copied command always looks like a terminal.

---

## 12. Do's & Don'ts

**1. Separate blocks with a 1px gap, not a border.**
- **Do** stack panels in a `gap: 1px` flex column over `--color-background`.
- **Don't** add a border, a divider utility, or a rule element between panels. If you can see a line, it should be the background showing through.

**2. Produce spacing in exactly one place.**
- **Do** let the parent's `gap` create all separation between siblings.
- **Don't** add `padding-top: 1px` to "align" a child. The parent already spaced it, and the child will now sit 1px lower than the fixed rail beside it.

**3. Keep the radius token at zero and pin the derived steps.**
- **Do** set all four derived radii to `var(--radius)` when radius is `0`.
- **Don't** keep a `calc(var(--radius) - 4px)` chain. It evaluates to `-4px` and produces invalid CSS, not square corners.

**4. Reserve hue for meaning.**
- **Do** draw non-semantic series and secondary text in `--color-muted-foreground`.
- **Don't** color a zero. `0` in red announces a problem that does not exist — drop the accent and render it muted.

**5. Use exactly four font weights, and never 700.**
- **Do** carry emphasis with `font-medium` (500) and reach for `font-semibold` (600) on titles.
- **Don't** introduce `font-bold`. The system has no 700; the public pages' `800` is a deliberate jump, not an approximation.

**6. Machine values are mono; human values are sans.**
- **Do** render IDs, durations, counts, query expressions, log bodies and attribute keys in `--font-mono`, with `tabular-nums` and right alignment when numeric.
- **Don't** mono a label, a title, or prose.

**7. Give tables one flexible column and fix the rest.**
- **Do** set explicit pixel widths from the seven-value scale and give the flexible cell `max-width: 0` plus ellipsis.
- **Don't** apply `text-overflow: ellipsis` without a max-width inside a table or flex cell — it is silently inert.

**8. Cancel a framework's variant-qualified style with the same variant.**
- **Do** write `[data-side=left]:border-right-0` to kill `[data-side=left]:border-right`.
- **Don't** expect a plain utility to win. Class-merge tools do not merge across variant boundaries, and the failure is silent — this regressed once and neither color nor coordinate checks caught it.

**9. Surfaces are flat; only overlays lift.**
- **Do** keep the four small shadow steps at `none` and reserve real offset shadows for `md` and above.
- **Don't** ship a zero-offset blur. A `0 0 7px` "shadow" is a glow, and it halos every card in a system that has no other gradients.

**10. Motion is two patterns and nothing else.**
- **Do** fade opacity over `0.18s` and grow bars over `0.24s`, both on `cubic-bezier(0.4, 0, 0.2, 1)`.
- **Don't** animate charts, slide routes, scale on hover, or add a spinner. `disabled` is this product's entire loading vocabulary.

**11. Distinguish "no data" from "request failed" from "your query is malformed".**
- **Do** ship three separate states, and color only the transport-failure icon.
- **Don't** collapse them. A syntax hint delivered as "the request failed" sends the user to the wrong problem entirely.

**12. Guard the pending state before the empty state.**
- **Do** check pending first, then error, then query error, then empty, then results.
- **Don't** branch on data alone. When a filter changes, the cache key changes and data is briefly undefined — the user sees "no results" flash between every keystroke.

**13. Show validation errors in two channels.**
- **Do** render field-level messages beneath their inputs and server-level messages above the footer, and move focus to the first invalid field.
- **Don't** leave the field channel out. Without it a failed client-side check produces no request and no message — the submit button silently does nothing.

**14. Put every recurring measurement in a token, not in a component.**
- **Do** name the panel inset, the gutter, the rail widths, the four skeleton heights.
- **Don't** hardcode a duration inline when the token exists. One tile does, and it is a latent divergence.

---

## 13. Responsive Behavior

The product is a **desktop-first operator console with a single 768px break**. There is no tablet tier and no wide tier. Below 768px, exactly one thing changes structurally: the left rail becomes an overlay drawer. Everything else adapts by wrapping.

### 13-1. Shell

| Width | Left rail | Content | Context panel |
|---|---|---|---|
| **≥ 768px** | 256px fixed (48px when collapsed) | fluid, `min-width: 0` | **320px, always** |
| **< 768px** | `display: none`; navigation moves to a **288px overlay drawer** over a 50% black scrim, opened by the header trigger | fluid | **still 320px** |

> **Known gap.** The context panel has no responsive rule and no toggle. At 375px, the 320px panel leaves roughly 55px of content column. This is the shell's one unhandled case. A recreator should hide the panel below 768px and surface it as a second drawer — but should know that doing so is a **change**, not a faithful reproduction.

### 13-2. Per-archetype collapse

| Archetype | < 768px | ≥ 768px | ≥ 1280px |
|---|---|---|---|
| **A · Signal List** | Tables do not stack or card-ify. Fixed column widths plus the `max-width: 0` truncating cell compress; the wrapper scrolls horizontally. Toolbars reflow via `flex-wrap`. | same | same |
| **B · Stacked Detail** | Tile grid 2 columns. Badge and metadata rows wrap. | Tile grid 4 columns. | same as 768 |
| **C · Metric Overview** | Tile grid 2 columns; 5-tile pages leave a ragged last row. Chart stays `224px` at every width. | 4 or 5 columns. | same as 768 |
| **D · Tabbed Console** | The 11-trigger tab list wraps onto multiple rows. | usually one row | same |
| **E · Admin CRUD** | Toolbar wraps. Action-button clusters **do not** collapse into a menu — they stay inline and compress. | same | same |
| **F · Query Console** | Chip groups wrap. Comparison grid 1 column. | Comparison grid 2 columns. | **Comparison grid 3 columns — the only `xl` rule in the product.** |
| **G · Full-bleed Viz** | No breakpoint response. The visualization overflows and scrolls. | same | same |
| **Public auth** | `384px` card with `16px` page padding — already fluid below that. | same | same |
| **Public guide** | `768px` column with `16px` padding — fluid below that. Code blocks step from `--text-2xs` to `--text-sm` at **1024px**, the product's only `lg` rule. | same | code text grows at 1024 |

### 13-3. The known deviation

One route sets a **3-column tile grid with no breakpoint prefix**, so it renders three columns at every width including a phone. Every other tile grid starts at two. Reproduce it if you are matching the source; fix it if you are shipping a product — but do not do it silently.

### 13-4. Wrapping is the real responsive mechanism

`flex-wrap` appears 40 times; media queries appear 17 times in application code, 14 of which are the tile-grid column switch. Toolbars, chip groups, badge rows, tab lists and metadata rows all reflow by wrapping. **When adding a horizontal cluster to this product, make it wrap rather than adding a breakpoint.**

---

## 14. Framework-Agnostic Translation

Everything in §§ 2–13 is expressed in HTML, ARIA and CSS. This section is the only place vendor names appear, and it exists so a recreator can map what they read back to what the source ships.

### 14-1. Primitive mapping

| Source primitive | Semantic HTML + ARIA | CSS / utility equivalent |
|---|---|---|
| `SidebarProvider` | `<div>` shell with `display:flex; height:100dvh; min-height:0` | sets `--sidebar-width`, `--sidebar-width-icon` as custom properties |
| `Sidebar collapsible="icon"` | `<aside>` (fixed, `inset-block:0; left:0; z-index:10`) + an in-flow spacer `<div>` of equal width | width `256px` ⇄ `48px`, `transition: width .2s linear` |
| `SidebarTrigger` | `<button aria-label="사이드바 토글" aria-expanded>` | `width:32px; height:32px` |
| `SidebarMenuButton` | `<a aria-current="page">` inside `<li>` | `height:36px; padding-inline:12px; border-radius:0; gap:12px` |
| `SidebarMenuBadge` | `<span>` with `margin-left:auto` | `tabular-nums`; `--color-destructive` when critical |
| `SidebarInset` | `<div>` with `min-height:0; min-width:0; overflow:hidden` | — |
| `Sheet` (mobile rail) | `<dialog>` or `role="dialog" aria-modal="true"` + scrim `<div>` | `position:fixed; inset-block:0; left:0; width:288px; z-index:50`; scrim `oklch(0 0 0 / 50%)` |
| `ScrollArea` | `<div style="overflow-y:auto">` | scrollbars hidden globally |
| `Card` + `PanelCard` | `<section>` > `<header><h2></header>` + `<div>` | §10-1 |
| `CardTitle` | **`<h2>`** (source renders `<div>` — see §7-4) | `font-size:.875rem; font-weight:500` |
| `Item` (stat tile) | `<article>` | §10-2 |
| `Table` / `TableHead` / `TableCell` | `<table>` / `<th scope="col">` / `<td>` | §10-3 |
| `Badge` | `<span>` (or `<a>` when it navigates) | §10-5 |
| `Button` | `<button type="button">`; `asChild` → render the child element directly | §14-5 |
| `Dialog` | `role="dialog" aria-modal="true" aria-labelledby` + scrim | §10-9 |
| `AlertDialog` | `role="alertdialog" aria-modal="true"` | confirm slot styled destructive |
| `Popover` / `DropdownMenu` / `Select` | `<button aria-haspopup="menu|listbox" aria-expanded>` + `role="menu"`/`role="listbox"` panel | `z-index:50`, background `--color-popover` |
| `Tooltip` | `role="tooltip"` linked by `aria-describedby`; delay **0ms** | `z-index:50` |
| `Tabs` / `TabsList` / `TabsTrigger` | `role="tablist"` / `role="tab" aria-selected` / `role="tabpanel"` | active trigger `--color-background` on a `--color-muted` list |
| `ToggleGroup type="multiple"` | `<div role="group">` of `<button aria-pressed>` | pressed → `--color-accent` |
| `Switch` | `<button role="switch" aria-checked>` | — |
| `Accordion` / `Collapsible` | `<details><summary>` or `<button aria-expanded>` + region | chevron rotates 90° |
| `Pagination` | `<nav aria-label="페이지 이동">` with `<button disabled>` | never a faked anchor |
| `Skeleton` | `<div aria-hidden="true">` | `background: --color-muted`; `animation: pulse 2s infinite` |
| `Empty` / `EmptyMedia` / `EmptyTitle` | `<div role="status">` + icon + text | §10-10 |
| `Field` / `FieldLabel` / `FieldError` | `<label for>` + `<input aria-invalid aria-describedby>` + `<p role="alert">` | §10-9 |
| `Command` (palette) | `role="combobox"` + `role="listbox"` | present in the vendored layer; **used by zero pages** |
| `Resizable*` | — | present in the vendored layer; **used by zero pages** |
| `Spinner` | — | present in the vendored layer; **used by zero files** |
| `Toaster` | `role="status" aria-live="polite"`, bottom-right | mounted, but **never invoked** — see §14-7 |
| `ChartContainer` | `<figure>` wrapping a `<svg>` | `height:224px; width:100%` |
| `motion.div` | element with a CSS transition or Web Animation | `opacity .18s` / `width .24s`, `cubic-bezier(.4,0,.2,1)` |

### 14-2. Icon strategy

The source uses **lucide-react**: 64 distinct icons, 47 in application code, 17 inside vendored primitives. Every one becomes an inline `<svg>` in a framework-agnostic recreation — lucide ships as plain SVG paths and the license permits copying the markup.

**Sizing is by a single dimension utility, never a width/height pair.** There is not one `h-4 w-4` on an icon in application code.

| Context | Size | How |
|---|---|---|
| Inside a button | **16px** | inherited from the button's descendant rule; the icon carries **no class at all** |
| Inside an extra-small button | 12px | same mechanism |
| Inline micro-affordance (exclude, lock, add-column, delete) | **12px**, explicit | `width:12px; height:12px` |
| Waterfall disclosure and its spacer | 16px, explicit, `flex-shrink:0` | |
| Empty-state media | 24px | from the empty-state media rule |
| Chrome control **buttons** (theme, logout, trigger) | 32px on the *button*, icon still 16px | |

```css
/* the descendant rule that makes "no class" the default */
.btn svg:not([class*="size-"]) { width: 16px; height: 16px; }
.btn svg { pointer-events: none; flex-shrink: 0; }
```

**No emoji, anywhere.** This is an explicit project rule covering UI, code, commits and documentation.

### 14-3. Locale binding

`<html lang="ko">` on both surfaces. All UI copy is Korean; there is no i18n framework, no message catalog, and no `html[lang]` style selector. The font stack is not language-switched.

A recreator adding locales should introduce the binding at the root element and keep the mono stack unswitched — machine values are language-neutral:

```css
html[lang='ko'] { font-family: var(--font-sans); }
html[lang='en'] { font-family: var(--font-sans); }
code, pre, .mono { font-family: var(--font-mono); }
```

Note the source loads **zero web fonts**. Korean rendering depends entirely on the OS stack resolving through `system-ui` / `Noto Sans`.

### 14-4. Framework defaults as absolute pixels

These values are never written in the source — they arrive from the component library. A recreator without that library **will miss every one of them**.

| Hidden default | Absolute value |
|---|---|
| Sidebar expanded width | **256px** |
| Sidebar collapsed (icon rail) width | **48px** |
| Sidebar mobile drawer width | **288px** |
| Sidebar state cookie name / lifetime | `sidebar_state` / **604800s (7 days)** |
| Sidebar keyboard shortcut | **`Cmd/Ctrl + B`** |
| Dialog default max width (**overridden** to 672px) | 512px → **672px** |
| Dialog scrim | **`oklch(0 0 0 / 50%)`** |
| Sheet default width (**overridden**) | `75%` → **288px** |
| Empty-state padding at ≥768px (**cancelled**) | 48px → **12px** |
| Table cell padding | **8px** |
| Button height — default / sm / lg / icon | **36 / 32 / 40 / 36px** |
| Button horizontal padding — default / sm / lg | **16 / 12 / 24px** |
| Icon inside button | **16px** (12px in the xs sizes) |
| Focus ring width | **3px**, at 50% of `--color-ring` |
| Badge padding / radius | **2px 8px** / **9999px** |
| `Item` default border (**must be zeroed**) | `1px solid transparent` → **0** |
| Nav item height (**overridden**) | 32px → **36px** |
| Rail trigger (**overridden**) | 28px → **32px** |
| Tooltip open delay (**overridden**) | 700ms → **0ms** |
| Tailwind spacing base | **4px** |
| Tailwind `md` breakpoint | **768px** |

### 14-5. Variant generators

Two `cva` sets exist and they are **not identical** — this is a real fork.

| | Dashboard | Public pages |
|---|---|---|
| Button variants | default, destructive, outline, secondary, ghost, link | same six |
| Button sizes | default, **xs**, sm, lg, icon, **icon-xs**, **icon-sm**, **icon-lg** | default, sm, lg, icon |
| Button extras | `aria-invalid` rings, `has-[>svg]` padding compensation, `data-slot`/`data-variant`/`data-size`, polymorphic rendering | none |
| Badge variants | default, secondary, destructive, outline, **ghost**, **link** | default, secondary, destructive, outline, **warning**, **success** |
| Badge borders | `border: 1px solid transparent` in base | real border, per-variant `transparent` |

**The badge fork is the substantive one**: the public surface encodes tinted status badges (`--color-chart-4` and `--color-chart-2` at 20% over `--color-foreground`) that the dashboard does not have, while the dashboard encodes link/ghost badges the public surface lacks. Pick one set for a unified recreation and record the choice.

### 14-6. Custom variants

```css
/* dark mode — the source's mechanism */
@custom-variant dark (&:is(.dark *));

/* interop alias for hosts that stamp data-theme */
@custom-variant dark-attr (&:is([data-theme='dark'] *));
```

No other custom variants are declared. No `@plugin`, no `@utility`, no `@keyframes` in either theme file.

### 14-7. Animation library replacement

| Source | Replacement | Notes |
|---|---|---|
| `motion` opacity fade | `transition: opacity .18s cubic-bezier(.4,0,.2,1)` or `element.animate()` | keyed on route change; re-trigger by remounting or by resetting opacity |
| `motion` bar width tween | `transition: width .24s cubic-bezier(.4,0,.2,1)`, set the target width after first paint | |
| `recharts` line series | any SVG line renderer, or hand-rolled paths | **animation must stay off** |
| `animate-in` / `zoom-in-95` / `slide-in-from-*` / `animate-accordion-*` | **nothing — these are inert in source** | the plugin that defines them is not installed; overlays open instantly (§7-3) |
| `animate-spin` / `animate-pulse` | Tailwind core; trivial keyframes | the only animations that actually run |

### 14-8. Dead vendored surface

Four things ship in the source and are reachable by zero code paths. A recreator should not port them without deciding they are wanted:

1. **Resizable panels** — the primitive exists; no page imports it. There is no resizable split in the product.
2. **Spinner** — exists; zero importers. Loading is skeletons.
3. **Toast host** — mounted at the app root, but `toast()` is never called. Mutation errors surface as inline text instead. Its theme also resolves independently of the app's (§4-6).
4. **`--font-serif`** and **`--text-3xs`** — declared, never referenced.

---

## 15. File Tree Coverage Checklist

Every style-relevant file in both surfaces. `[x]` = inspected. Each entry is labelled **leader** (its anatomy is documented in §10 or §11), **instance** (composes a documented pattern), **vendored** (byte-equivalent to the upstream registry — §10-0), or **dead** (zero importers).

### Surface A — `apps/web/` (106 source files + 3 config)

**Config and entry (5)**
```
[x] index.html                         lang="ko"; no theme script (FOUC — §4-6); no font links
[x] vite.config.ts                     React Compiler on; Tailwind v4 plugin; base '/'
[x] components.json                    style new-york · baseColor neutral · cssVariables · lucide
[x] src/main.tsx                       root mount; Korean validation locale
[x] src/index.css                      ★ THEME SOURCE — §3, §4, §5, §6
```

**App shell (3)**
```
[x] src/app/app.tsx                    leader — provider nesting + 31-entry route table (§8-1)
[x] src/app/dashboard-shell.tsx        leader — Navigation Rail (§10-11); theme hook (§4-7)
[x] src/app/context-panel.tsx          leader — Context Filter Panel (§10-12)
```

**Widgets (3)**
```
[x] src/widgets/series-chart.tsx       leader — Series Chart (§10-4)
[x] src/widgets/stat-tile.tsx          leader — Stat Tile (§10-2)
[x] src/widgets/service-map-graph.tsx  leader — Service Map Graph (§10-14)
```

**Features — root (16)**
```
[x] src/features/panel-card.tsx        leader — Panel Card (§10-1)
[x] src/features/query-error.tsx       leader — State Triad (§10-10)
[x] src/features/form-dialog.tsx       leader — Form Dialog (§10-9)
[x] src/features/facet-list.tsx        leader — Bar-Count List (§10-6)
[x] src/features/trace-waterfall.tsx   leader — Trace Waterfall (§10-7)
[x] src/features/attr-tree.tsx         leader — Attribute Tree (§10-8)
[x] src/features/bubble-field.tsx      instance — Bar-Count List; override: 2px track, two stacked bars
[x] src/features/vital-distribution.tsx instance — Bar-Count List; override: 6px track, three segments
[x] src/features/admin-table.tsx       supporting (§10-13) — Panel Card + Data Table + State Triad
[x] src/features/saved-view-bar.tsx    supporting (§10-13) — outline badge row
[x] src/features/stack-trace.tsx       supporting (§10-13) — mono block + frame collapse
[x] src/features/issue-event-item.tsx  supporting (§10-13) — disclosure row
[x] src/features/keyset-pager.tsx      supporting (§10-13) — Pager, keyset dialect
[x] src/features/offset-pager.tsx      instance — Pager; override: adds page/total readout
[x] src/features/column-toggle.tsx     supporting (§10-13) — multi-select chip group
[x] src/features/trace-jump.tsx        instance — mono input, 32px, inside the context panel
```

**Features — admin forms (14)**
```
[x] src/features/admin-forms/confirm-action.tsx           supporting (§10-13) — destructive confirm modal
[x] src/features/admin-forms/ingest-token-form-dialog.tsx instance — Form Dialog; override: post-submit
                                                          reveal-once panel (readonly mono input + copy)
[x] src/features/admin-forms/alert-rule-form-dialog.tsx   instance — Form Dialog; override: 2-column
                                                          field grid (15 fields) + notes list
[x] .../channel-form-dialog.tsx              instance — Form Dialog; horizontal switch fields
[x] .../inbound-filter-form-dialog.tsx       instance — Form Dialog, no overrides
[x] .../log-metric-rule-form-dialog.tsx      instance — Form Dialog; mono inputs
[x] .../notification-route-form-dialog.tsx   instance — Form Dialog; textarea; trigger disabled when
                                                        no channels exist
[x] .../retention-filter-form-dialog.tsx     instance — Form Dialog, no overrides
[x] .../retention-form-dialog.tsx            instance — Form Dialog, no overrides
[x] .../route-rule-form-dialog.tsx           instance — Form Dialog, no overrides
[x] .../scrub-rule-form-dialog.tsx           instance — Form Dialog; mono inputs
[x] .../service-registry-form-dialog.tsx     instance — Form Dialog, no overrides
[x] .../silence-form-dialog.tsx              instance — Form Dialog; textarea; bell-off trigger icon
[x] .../slo-form-dialog.tsx                  instance — Form Dialog, no overrides
```

**Pages (30)**
```
Archetype A · Signal List — leader: errors-page
[x] src/pages/errors-page.tsx          leader (§11-A)
[x] src/pages/logs-page.tsx            instance; override: dynamic attr.* columns + source select
[x] src/pages/traces-page.tsx          instance; offset pager
[x] src/pages/issues-page.tsx          instance; status tabs inside a Panel Card
[x] src/pages/perf-issues-page.tsx     instance; offset pager
[x] src/pages/services-page.tsx        instance; single panel, no toolbar
[x] src/pages/live-page.tsx            instance; streaming; play/pause cluster + pulse indicator
[x] src/pages/log-patterns-page.tsx    instance; back bar
[x] src/pages/log-context-page.tsx     instance; focused row highlighted with --color-accent

Archetype B · Stacked Detail — leader: issue-detail-page
[x] src/pages/issue-detail-page.tsx    leader (§11-B)
[x] src/pages/error-detail-page.tsx    instance; Attribute Tree + Stack Trace
[x] src/pages/log-detail-page.tsx      instance; two Attribute Trees + mono body block
[x] src/pages/incident-detail-page.tsx instance; ack cluster + textarea with counter
[x] src/pages/alert-rule-detail-page.tsx instance; simulation tiles
[x] src/pages/perf-issue-detail-page.tsx instance; hybrid B+G — embeds a Trace Waterfall

Archetype C · Metric Overview — leader: traffic-page
[x] src/pages/traffic-page.tsx         leader (§11-C); the only shape-matched skeleton in the product
[x] src/pages/metrics-page.tsx         instance; ⚠ deviation — grid-cols-3 with no breakpoint (§13-3)
[x] src/pages/rum-page.tsx             instance; metric chip group + Vitals Distribution
[x] src/pages/cache-page.tsx           instance
[x] src/pages/system-page.tsx          instance; 16 tiles; per-panel inline states
[x] src/pages/diagnostics-page.tsx     instance; 6 independent queries; raw↔normalized diff panel

Archetype D · Tabbed Console — leader: admin-page
[x] src/pages/admin-page.tsx           leader (§11-D); 11 tabs; 8 form dialogs
[x] src/pages/alerts-page.tsx          instance; 3 tabs

Archetype E · Admin CRUD — leader: alert-rules-page
[x] src/pages/alert-rules-page.tsx     leader (§11-E)
[x] src/pages/slo-page.tsx             instance
[x] src/pages/channels-page.tsx        instance; three tables (channels, routes, deliveries)

Archetype F · Query Console — leader: explore-page
[x] src/pages/explore-page.tsx         leader (§11-F)
[x] src/pages/bubbleup-page.tsx        instance; ⚠ the only 3-tier grid (1→2→3 at 768/1280)

Archetype G · Full-bleed Viz — leader: trace-page
[x] src/pages/trace-page.tsx           leader (§11-G)
[x] src/pages/service-map-page.tsx     instance; two distinct empty reasons
```

**Shared non-visual (12)** — inspected for style content; none carries design decisions except where noted.
```
[x] src/shared/lib/cn.ts               class merge helper
[x] src/shared/lib/motion.ts           ★ MOTION TOKENS — §7-1
[x] src/shared/lib/format.ts           severity threshold constant (drives §10-5)
[x] src/shared/lib/table-columns.ts    column descriptors (drives §10-3)
[x] src/shared/lib/query-term.ts       no style content
[x] src/shared/lib/csv.ts              no style content
[x] src/shared/hooks/use-mobile.ts     ★ JS BREAKPOINT 768px — §9-2
[x] src/shared/hooks/use-dashboard-query.ts  URL state — drives §11-F
[x] src/shared/hooks/use-live-events.ts      no style content
[x] src/shared/api/rpc.ts              no style content
[x] src/shared/api/query-key.ts        no style content
[x] src/shared/api/types.ts            no style content
```

**Vendored design-system layer (39)** — all byte-equivalent to the upstream `new-york-v4` registry (§10-0). Documented as a dependency in §14-1, not as anatomy.
```
[x] accordion  [x] alert         [x] alert-dialog  [x] badge      [x] breadcrumb
[x] button     [x] button-group  [x] card          [x] chart      [x] checkbox
[x] collapsible[x] command †     [x] dialog        [x] dropdown-menu [x] empty
[x] field      [x] form          [x] input         [x] item       [x] kbd
[x] label      [x] pagination    [x] popover       [x] resizable †[x] scroll-area
[x] select     [x] separator     [x] sheet         [x] sidebar    [x] skeleton
[x] sonner †   [x] spinner †     [x] switch        [x] table      [x] tabs
[x] textarea   [x] toggle        [x] toggle-group  [x] tooltip
```
† `command`, `resizable`, `spinner` have **zero importers**; `sonner` is mounted but never invoked (§14-8).

### Surface B — `apps/server/src/view/` (4 files) + 1 shared constant

```
[x] style/base.css        ★ THEME SOURCE — the second token set (§1-2, §3-6, §6-3, §6-5)
[x] ui/variants.ts        leader — the public-surface variant generators (§14-5)
[x] components.tsx        leader — pre-paint theme script (§4-7); flat Card; theme-invariant code block
[x] auth.tsx              leader — centered 384px card (§11-H); raw inputs, not the shared control set
[x] guide.tsx             leader — 768px document column + the product's only sticky header (§11-H)
[x] ../lib/theme.ts       ★ SHARED — storage key and class name used by BOTH surfaces (§4-7)
```

### Explicitly excluded

```
[ ] apps/server/public/style.css       build artifact — gitignored, produced from base.css
[ ] apps/web/dist/**                   build output
```

### Coverage summary

| Group | Files | Leaders | Instances | Vendored | Dead |
|---|---|---|---|---|---|
| Surface A config + entry | 5 | 1 (theme) | 4 | — | — |
| Surface A shell | 3 | 3 | — | — | — |
| Surface A widgets | 3 | 3 | — | — | — |
| Surface A features | 30 | 6 | 23 | — | — |
| Surface A pages | 30 | 7 | 23 | — | — |
| Surface A shared | 12 | 2 | 10 | — | — |
| Surface A vendored | 39 | — | — | 39 | 3 (+1 inert) |
| Surface B | 6 | 5 | 1 | — | — |
| **Total inspected** | **128** | **27** | **61** | **39** | **4** |

---

## 16. Delivery Block — Tailwind v4 `@theme` Reference

Complete and paste-ready. No ellipses. Surface A first (the product), Surface B second (the public pages).

### 16-1. Surface A — the dashboard

```css
@import 'tailwindcss';

/* ─────────────────────────────────────────────────────────────
   Dark-mode variants.
   `.dark` is the source mechanism (a script toggles it on <html>).
   `[data-theme='dark']` is an interop alias for hosts that stamp
   a data attribute instead.
   ───────────────────────────────────────────────────────────── */
@custom-variant dark (&:is(.dark *, [data-theme='dark'] *));

/* ═════════════════════════════════════════════════════════════
   TIER 1 — PALETTE.  Raw values. Never referenced by a component.
   ═════════════════════════════════════════════════════════════ */
:root {
    /* Neutral ramp — chroma is exactly 0 throughout. Name = L × 1000. */
    --palette-neutral-1000: oklch(1 0 0);       /* card, popover, destructive-fg (light) */
    --palette-neutral-985:  oklch(0.985 0 0);   /* primary-fg (light) / foreground (dark) */
    --palette-neutral-970:  oklch(0.97 0 0);    /* secondary, muted, accent (light) */
    --palette-neutral-955:  oklch(0.955 0 0);   /* background — tier 2 (light) */
    --palette-neutral-922:  oklch(0.922 0 0);   /* border, input (light) / primary (dark) */
    --palette-neutral-915:  oklch(0.915 0 0);   /* sidebar — tier 1 (light) */
    --palette-neutral-900:  oklch(0.9 0 0);     /* sidebar-border (light) */
    --palette-neutral-855:  oklch(0.855 0 0);   /* sidebar-accent (light) */
    --palette-neutral-708:  oklch(0.708 0 0);   /* ring (light) / muted-fg (dark) */
    --palette-neutral-556:  oklch(0.556 0 0);   /* muted-fg (light) / ring (dark) */
    --palette-neutral-420:  oklch(0.42 0 0);    /* sidebar-foreground (light) */
    --palette-neutral-269:  oklch(0.269 0 0);   /* secondary, muted, accent (dark) */
    --palette-neutral-232:  oklch(0.232 0 0);   /* sidebar-accent (dark) */
    --palette-neutral-212:  oklch(0.212 0 0);   /* card — tier 3 (dark) */
    --palette-neutral-205:  oklch(0.205 0 0);   /* primary (light) / popover, primary-fg (dark) */
    --palette-neutral-162:  oklch(0.162 0 0);   /* background — tier 2 (dark) */
    --palette-neutral-145:  oklch(0.145 0 0);   /* foreground (light) */
    --palette-neutral-098:  oklch(0.098 0 0);   /* sidebar — tier 1 (dark) */

    /* Source drift, preserved verbatim: dark destructive-foreground is written
       with four decimals. 0.0001 above --palette-neutral-985. See §3-2. */
    --palette-neutral-985-alt: oklch(0.9851 0 0);

    /* Translucent white — dark hairlines that adapt to any surface tier. */
    --palette-white-a10: oklch(1 0 0 / 10%);    /* border (dark) */
    --palette-white-a15: oklch(1 0 0 / 15%);    /* input (dark) */

    /* Chromatic accents. Fractional hue is reproduced from source verbatim. */
    --palette-red-577:    oklch(0.577 0.245 27.325);   /* destructive (light) */
    --palette-red-704:    oklch(0.704 0.191 22.216);   /* destructive (dark) */
    --palette-amber-705:  oklch(0.705 0.153 70);       /* warning (light) */
    --palette-amber-790:  oklch(0.79 0.145 75);        /* warning (dark) */
    --palette-orange-646: oklch(0.646 0.222 41.116);   /* chart-1 (light) — selection */
    --palette-teal-600:   oklch(0.6 0.118 184.704);    /* chart-2 (light) — "good" */
    --palette-blue-398:   oklch(0.398 0.07 227.392);   /* chart-3 (light) — fills only, too dark for text */
    --palette-yellow-828: oklch(0.828 0.189 84.429);   /* chart-4 (light) — "needs improvement" */
    --palette-amber-769:  oklch(0.769 0.188 70.08);    /* chart-5 (light) AND chart-3 (dark) */
    --palette-indigo-488: oklch(0.488 0.243 264.376);  /* chart-1 (dark) — selection */
    --palette-green-696:  oklch(0.696 0.17 162.48);    /* chart-2 (dark) — "good" */
    --palette-purple-627: oklch(0.627 0.265 303.9);    /* chart-4 (dark) — "needs improvement" */
    --palette-rose-645:   oklch(0.645 0.246 16.439);   /* chart-5 (dark) */

    /* Sidebar chromatics — the only faintly-tinted values in the system.
       Converted from HSL; original strings kept for traceability. */
    --palette-slate-210: oklch(0.210 0.006 286);  /* hsl(240 5.9% 10%) */
    --palette-slate-968: oklch(0.968 0.001 286);  /* hsl(240 4.8% 95.9%) */
    --palette-slate-274: oklch(0.274 0.005 286);  /* hsl(240 3.7% 15.9%) */
    --palette-blue-623:  oklch(0.623 0.188 260);  /* hsl(217.2 91.2% 59.8%) */
    --palette-blue-488:  oklch(0.488 0.217 264);  /* hsl(224.3 76.3% 48%) */

    /* Shadow ink — hsl(0 0% 0%) */
    --palette-shadow-ink: oklch(0 0 0);
}

/* ═════════════════════════════════════════════════════════════
   TIER 2 — SEMANTIC (LIGHT).  The only tier components reference.
   ═════════════════════════════════════════════════════════════ */
:root {
    /* Surface tiers — the entire depth system */
    --sidebar:    var(--palette-neutral-915);   /* tier 1 — both rails */
    --background: var(--palette-neutral-955);   /* tier 2 — page body, shows in the 1px seams */
    --card:       var(--palette-neutral-1000);  /* tier 3 — content */

    --foreground:            var(--palette-neutral-145);
    --card-foreground:       var(--palette-neutral-145);
    --popover:               var(--palette-neutral-1000);
    --popover-foreground:    var(--palette-neutral-145);
    --primary:               var(--palette-neutral-205);
    --primary-foreground:    var(--palette-neutral-985);
    --secondary:             var(--palette-neutral-970);
    --secondary-foreground:  var(--palette-neutral-205);
    --muted:                 var(--palette-neutral-970);
    --muted-foreground:      var(--palette-neutral-556);  /* also: the default chart series */
    --accent:                var(--palette-neutral-970);
    --accent-foreground:     var(--palette-neutral-205);
    --destructive:           var(--palette-red-577);
    --destructive-foreground:var(--palette-neutral-1000);
    --warning:               var(--palette-amber-705);    /* project-added, not upstream */
    --border:                var(--palette-neutral-922);
    --input:                 var(--palette-neutral-922);
    --ring:                  var(--palette-neutral-708);

    --chart-1: var(--palette-orange-646);
    --chart-2: var(--palette-teal-600);
    --chart-3: var(--palette-blue-398);
    --chart-4: var(--palette-yellow-828);
    --chart-5: var(--palette-amber-769);

    --sidebar-foreground:         var(--palette-neutral-420);
    --sidebar-primary:            var(--palette-slate-210);
    --sidebar-primary-foreground: var(--palette-neutral-985);
    --sidebar-accent:             var(--palette-neutral-855);
    --sidebar-accent-foreground:  var(--palette-slate-210);
    --sidebar-border:             var(--palette-neutral-900);
    --sidebar-ring:               var(--palette-blue-623);

    /* Type */
    --font-sans:
        ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto,
        'Helvetica Neue', Arial, 'Noto Sans', sans-serif,
        'Apple Color Emoji', 'Segoe UI Emoji', 'Segoe UI Symbol', 'Noto Color Emoji';
    --font-serif: ui-serif, Georgia, Cambria, 'Times New Roman', Times, serif;
    --font-mono:
        ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono',
        'Courier New', monospace;

    /* Geometry */
    --radius:  0rem;      /* square. all four derived steps pin to this — never calc() */
    --spacing: 0.25rem;   /* 4px base */

    /* Shadows — surfaces flat, overlays lifted */
    --shadow-2xs: none;
    --shadow-xs:  none;
    --shadow-sm:  none;
    --shadow:     none;
    --shadow-md:  0px 2px 8px 0px oklch(0 0 0 / 0.10);
    --shadow-lg:  0px 4px 16px 0px oklch(0 0 0 / 0.12);
    --shadow-xl:  0px 8px 24px 0px oklch(0 0 0 / 0.14);
    --shadow-2xl: 0px 16px 40px 0px oklch(0 0 0 / 0.18);

    /* Motion — the entire authored budget */
    --motion-ease-standard: cubic-bezier(0.4, 0, 0.2, 1);
    --motion-fade-duration: 0.18s;
    --motion-bar-duration:  0.24s;

    /* Layout constants — absolute, not framework defaults */
    --sidebar-width:        16rem;   /* 256px */
    --sidebar-width-icon:    3rem;   /*  48px */
    --sidebar-width-mobile: 18rem;   /* 288px */
    --context-panel-width:  20rem;   /* 320px */
    --rail-chrome-height:    3rem;   /*  48px — rail header and footer */
    --nav-item-height:     2.25rem;  /*  36px */
    --panel-inset:          12px;
    --panel-gutter:          1px;    /* the block separator */
    --chart-height:         14rem;   /* 224px */
    --skeleton-page:         24rem;  /* 384px */
    --skeleton-table:        16rem;  /* 256px */
    --skeleton-panel:         6rem;  /*  96px */
    --skeleton-row:           1.5rem;/*  24px */

    /* Z-index — three values, that is the whole stack */
    --z-rail:    10;
    --z-raised:  20;
    --z-overlay: 50;
}

/* ═════════════════════════════════════════════════════════════
   TIER 2 — SEMANTIC (DARK).  Only bindings change. Anatomy,
   spacing, type, radius and shadow are identical to light.
   ═════════════════════════════════════════════════════════════ */
.dark,
[data-theme='dark'] {
    --sidebar:    var(--palette-neutral-098);   /* tier 1 */
    --background: var(--palette-neutral-162);   /* tier 2 */
    --card:       var(--palette-neutral-212);   /* tier 3 — brighter than body, as in light */

    --foreground:            var(--palette-neutral-985);
    --card-foreground:       var(--palette-neutral-985);
    --popover:               var(--palette-neutral-205);  /* NOTE: darker than card — §4-6 */
    --popover-foreground:    var(--palette-neutral-985);
    --primary:               var(--palette-neutral-922);
    --primary-foreground:    var(--palette-neutral-205);
    --secondary:             var(--palette-neutral-269);
    --secondary-foreground:  var(--palette-neutral-985);
    --muted:                 var(--palette-neutral-269);
    --muted-foreground:      var(--palette-neutral-708);
    --accent:                var(--palette-neutral-269);
    --accent-foreground:     var(--palette-neutral-985);
    --destructive:           var(--palette-red-704);
    --destructive-foreground:var(--palette-neutral-985-alt);
    --warning:               var(--palette-amber-790);
    --border:                var(--palette-white-a10);
    --input:                 var(--palette-white-a15);
    --ring:                  var(--palette-neutral-556);

    --chart-1: var(--palette-indigo-488);
    --chart-2: var(--palette-green-696);
    --chart-3: var(--palette-amber-769);
    --chart-4: var(--palette-purple-627);
    --chart-5: var(--palette-rose-645);

    --sidebar-foreground:         var(--palette-slate-968);
    --sidebar-primary:            var(--palette-blue-488);
    --sidebar-primary-foreground: var(--palette-neutral-1000);
    --sidebar-accent:             var(--palette-neutral-232);
    --sidebar-accent-foreground:  var(--palette-slate-968);
    --sidebar-border:             var(--palette-slate-274);
    --sidebar-ring:               var(--palette-blue-623);  /* unchanged from light */
}

/* Explicit light — wins over any OS preference.
   Empty on purpose: :root above already holds the light bindings. */
[data-theme='light'] { color-scheme: light; }

/* ─────────────────────────────────────────────────────────────
   OPT-IN: follow the operating system.
   DISABLED BY DEFAULT — the source resolves an unset preference
   to LIGHT and never reads prefers-color-scheme (§4-6). Enable
   this only if you intend to change that behavior. The guard
   keeps a manual light choice winning.
   ─────────────────────────────────────────────────────────────
@media (prefers-color-scheme: dark) {
    :root:not(.light):not([data-theme='light']) {
        --sidebar:    var(--palette-neutral-098);
        --background: var(--palette-neutral-162);
        --card:       var(--palette-neutral-212);
        --foreground:            var(--palette-neutral-985);
        --card-foreground:       var(--palette-neutral-985);
        --popover:               var(--palette-neutral-205);
        --popover-foreground:    var(--palette-neutral-985);
        --primary:               var(--palette-neutral-922);
        --primary-foreground:    var(--palette-neutral-205);
        --secondary:             var(--palette-neutral-269);
        --secondary-foreground:  var(--palette-neutral-985);
        --muted:                 var(--palette-neutral-269);
        --muted-foreground:      var(--palette-neutral-708);
        --accent:                var(--palette-neutral-269);
        --accent-foreground:     var(--palette-neutral-985);
        --destructive:           var(--palette-red-704);
        --destructive-foreground:var(--palette-neutral-985-alt);
        --warning:               var(--palette-amber-790);
        --border:                var(--palette-white-a10);
        --input:                 var(--palette-white-a15);
        --ring:                  var(--palette-neutral-556);
        --chart-1: var(--palette-indigo-488);
        --chart-2: var(--palette-green-696);
        --chart-3: var(--palette-amber-769);
        --chart-4: var(--palette-purple-627);
        --chart-5: var(--palette-rose-645);
        --sidebar-foreground:         var(--palette-slate-968);
        --sidebar-primary:            var(--palette-blue-488);
        --sidebar-primary-foreground: var(--palette-neutral-1000);
        --sidebar-accent:             var(--palette-neutral-232);
        --sidebar-accent-foreground:  var(--palette-slate-968);
        --sidebar-border:             var(--palette-slate-274);
        --sidebar-ring:               var(--palette-blue-623);
    }
}
   ───────────────────────────────────────────────────────────── */

/* ═════════════════════════════════════════════════════════════
   TAILWIND THEME MAPPING
   ═════════════════════════════════════════════════════════════ */
@theme inline {
    --color-background: var(--background);
    --color-foreground: var(--foreground);
    --color-card: var(--card);
    --color-card-foreground: var(--card-foreground);
    --color-popover: var(--popover);
    --color-popover-foreground: var(--popover-foreground);
    --color-primary: var(--primary);
    --color-primary-foreground: var(--primary-foreground);
    --color-secondary: var(--secondary);
    --color-secondary-foreground: var(--secondary-foreground);
    --color-muted: var(--muted);
    --color-muted-foreground: var(--muted-foreground);
    --color-accent: var(--accent);
    --color-accent-foreground: var(--accent-foreground);
    --color-destructive: var(--destructive);
    --color-destructive-foreground: var(--destructive-foreground);
    --color-warning: var(--warning);
    --color-border: var(--border);
    --color-input: var(--input);
    --color-ring: var(--ring);

    --color-chart-1: var(--chart-1);
    --color-chart-2: var(--chart-2);
    --color-chart-3: var(--chart-3);
    --color-chart-4: var(--chart-4);
    --color-chart-5: var(--chart-5);

    --color-sidebar: var(--sidebar);
    --color-sidebar-foreground: var(--sidebar-foreground);
    --color-sidebar-primary: var(--sidebar-primary);
    --color-sidebar-primary-foreground: var(--sidebar-primary-foreground);
    --color-sidebar-accent: var(--sidebar-accent);
    --color-sidebar-accent-foreground: var(--sidebar-accent-foreground);
    --color-sidebar-border: var(--sidebar-border);
    --color-sidebar-ring: var(--sidebar-ring);

    --font-sans: var(--font-sans);
    --font-serif: var(--font-serif);
    --font-mono: var(--font-mono);

    /* Radius: all four pin to --radius. With --radius: 0rem, shadcn's
       calc(var(--radius) - 4px) would evaluate to -4px. */
    --radius-sm: var(--radius);
    --radius-md: var(--radius);
    --radius-lg: var(--radius);
    --radius-xl: var(--radius);

    /* Custom type step below Tailwind's xs */
    --text-2xs: 0.6875rem;   /* 11px */

    --shadow-2xs: none;
    --shadow-xs:  none;
    --shadow-sm:  none;
    --shadow:     none;
    --shadow-md:  var(--shadow-md);
    --shadow-lg:  var(--shadow-lg);
    --shadow-xl:  var(--shadow-xl);
    --shadow-2xl: var(--shadow-2xl);
}

/* ═════════════════════════════════════════════════════════════
   BASE
   ═════════════════════════════════════════════════════════════ */
@layer base {
    * {
        @apply border-border outline-ring/50;
    }
}

html,
body {
    @apply bg-background text-foreground font-sans antialiased;
    scrollbar-width: none;
    scroll-behavior: smooth;
}

::-webkit-scrollbar {
    display: none;
}

/* RECOMMENDED ADDITION — the source ships no reduced-motion guard (§7-4).
   Note the scroll-behavior reset: `smooth` is set globally above. */
@media (prefers-reduced-motion: reduce) {
    *,
    *::before,
    *::after {
        animation-duration: 0.01ms !important;
        animation-iteration-count: 1 !important;
        transition-duration: 0.01ms !important;
        scroll-behavior: auto !important;
    }
}
```

### 16-2. Surface B — the public pages

Same palette tier. Only the deltas are shown; everything not listed is identical to 16-1.

```css
@import 'tailwindcss';
@custom-variant dark (&:is(.dark *, [data-theme='dark'] *));

:root {
    /* ONE background plane, not three. No sidebar tier. */
    --background: oklch(1 0 0);
    --card:       oklch(1 0 0);

    /* Rounded, not square */
    --radius: 0.375rem;   /* 6px */

    /* The inherited zero-offset glow stack, retained on this surface */
    --shadow-2xs: 0px 0px 7px 0px oklch(0 0 0 / 0.03);
    --shadow-xs:  0px 0px 7px 0px oklch(0 0 0 / 0.03);
    --shadow-sm:  0px 0px 7px 0px oklch(0 0 0 / 0.06), 0px 1px 2px -1px oklch(0 0 0 / 0.06);
    --shadow:     0px 0px 7px 0px oklch(0 0 0 / 0.06), 0px 1px 2px -1px oklch(0 0 0 / 0.06);
    --shadow-md:  0px 0px 7px 0px oklch(0 0 0 / 0.06), 0px 2px 4px -1px oklch(0 0 0 / 0.06);
    --shadow-lg:  0px 0px 7px 0px oklch(0 0 0 / 0.06), 0px 4px 6px -1px oklch(0 0 0 / 0.06);
    --shadow-xl:  0px 0px 7px 0px oklch(0 0 0 / 0.06), 0px 8px 10px -1px oklch(0 0 0 / 0.06);
    --shadow-2xl: 0px 0px 7px 0px oklch(0 0 0 / 0.15);

    /* NO --warning token on this surface.
       NO --sidebar-* tokens on this surface. */
}

.dark,
[data-theme='dark'] {
    --background: oklch(0.145 0 0);
    --card:       oklch(0.205 0 0);
}

@theme inline {
    /* shadcn's calc chain is valid here because --radius is non-zero */
    --radius-sm: calc(var(--radius) - 4px);   /* 2px */
    --radius-md: calc(var(--radius) - 2px);   /* 4px */
    --radius-lg: var(--radius);               /* 6px */
    --radius-xl: calc(var(--radius) + 4px);   /* 10px */

    --text-2xs: 0.625rem;   /* 10px — NOT 0.6875rem. The two surfaces diverge. */
    --text-3xs: 0.5rem;     /*  8px — declared, referenced nowhere. Safe to drop. */
}
```

### 16-3. Pre-paint theme script

Required on any page that can be loaded directly, or the first frame renders light and then flips. Surface B ships this; Surface A does not, and has a visible flash.

```html
<script>
  if (localStorage.getItem('flunti-otel-theme') === 'dark')
    document.documentElement.classList.add('dark')
</script>
```

Place it in `<head>`, before any stylesheet link, un-deferred.

---

## 17. Agent Prompt Guide

Copy-paste prompts for recreating specific pieces. Each is self-contained and token-driven. Replace `<FRAMEWORK>` with React, Vue, Svelte, Astro or Solid.

### 17-1. Master prompt

```
Build the flunti-otel observability dashboard UI in <FRAMEWORK> + Tailwind v4.

Paste DESIGN.md §16-1 as your stylesheet entry before writing any component.

Non-negotiable constraints:
- border-radius is 0 everywhere. All four derived radius steps pin to var(--radius).
  Never write calc(var(--radius) - Npx) — it evaluates negative.
- Surfaces have NO border and NO shadow. Blocks are separated by a 1px gap over
  var(--color-background). Only overlays use shadows (--shadow-md and above).
- Exactly four font weights: 400, 500, 600, 800. font-bold (700) must not appear.
- Colors come only from var(--color-*). No hex, rgb, hsl, or Tailwind palette
  utilities (neutral-800, black/50, etc.) in any component you write.
- Content inset is 12px on both axes. Every page block is a Panel Card (§10-1).
- Motion: opacity fades at 0.18s, bar widths at 0.24s, both on
  cubic-bezier(0.4,0,0.2,1). Nothing else animates. Charts never animate.
- Loading is a skeleton, never a spinner. "Loading" on a button means disabled.
- Support .dark and [data-theme="dark"] on the root element.
- No emoji anywhere — icons only.

Build order: theme block → Panel Card → Data Table → Stat Tile → State Triad →
Navigation Rail → Context Filter Panel → pages.

After each component, grep your output for /#[0-9a-fA-F]{3,8}|rgb\(|hsl\(/ and for
/rounded-(?!none)|shadow-(?!none)|font-bold/. Both must return zero matches
outside the palette comments in the theme block.
```

### 17-2. Panel Card + page root

```
Implement two things from DESIGN.md §10-1 and §8-3 in <FRAMEWORK>.

1. PanelCard: a section with background var(--color-card), border 0, radius 0,
   box-shadow none, 12px block padding, 12px internal gap. An optional header
   holds an h2 at 0.875rem / weight 500. Header and content both get 12px inline
   padding. Accept a class override for the content wrapper only.

2. PageRoot: a flex column with gap: 1px.

Verify: two stacked PanelCards must show exactly 1px of var(--color-background)
between them, and a card title's left edge must be exactly 12px inside the card.
Do not add padding-top anywhere to "fix" alignment — the gap already did it.
```

### 17-3. Data Table

```
Implement the Data Table from DESIGN.md §10-3 in <FRAMEWORK>.

Driven by a column descriptor: { key, label, width?, align?, cell }.
- font-size 0.75rem set ONCE on the table, never per cell.
- th and td padding 8px. th is weight 500, var(--color-muted-foreground), left aligned.
- rows separated by border-bottom 1px var(--color-border). No striping.
- align: 'right' adds BOTH text-align: right AND font-variant-numeric: tabular-nums.
- width comes from {64,80,96,112,128,160,224}px. Exactly one column omits width;
  that column's cell gets max-width: 0 plus overflow hidden / ellipsis / nowrap.
- wrap the table in a div with overflow-x: auto.

CRITICAL: ellipsis without max-width: 0 is inert inside a table cell. Test with a
200-character value in the flexible column — it must ellipsize, not widen the table.
```

### 17-4. Status Badge + the semantic map

```
Implement the Status Badge from DESIGN.md §10-5 in <FRAMEWORK>.

Four variants: default, secondary, destructive, outline. border-radius 9999px,
padding 2px 8px, font-size 0.75rem, weight 500, 1px transparent border in base.

Then implement mapper functions for these domains using ONLY those four variants:
severity, HTTP status class, log severity number, issue status, delivery result,
enablement, approval, service freshness.

Rules you must encode:
- There is NO success/green variant. Healthy is `secondary`.
- `outline` means intermediate/pending/off — it is a semantic tier, not decoration.
- `destructive` is the only alarm.
- HTTP: >= 500 destructive; 400-499 outline; else secondary.
- Log severity: number >= 17 is destructive.
- "Is this span an error": status is error, OR http >= 500, OR (http 400-499 AND
  the span is a client span). A 4xx on an inbound server span is NOT an error.
```

### 17-5. Series Chart

```
Implement the Series Chart from DESIGN.md §10-4 in <FRAMEWORK>.

Line chart only. Height 224px, width 100%. Margin { top:4, right:8, left:4 }.
- monotone interpolation, strokeWidth 2, dots OFF, connectNulls FALSE.
- ANIMATION DISABLED. Do not set a duration; disable it outright.
- Horizontal grid only, stroke-opacity 0.15, color var(--color-border).
- X axis: no axis line, no tick line, tickMargin 8, minTickGap 48, labels "HH:MM".
- Y axis: no axis line, no tick line, tickMargin 8, width 44, >=1000 renders "N.Nk".
- Ticks in var(--color-muted-foreground).
- Tooltip: var(--color-background), 1px border var(--color-border) at 50%,
  font-size 0.75rem, --shadow-xl. Cursor stroke var(--color-border).
- Series i with no explicit color -> var(--color-chart-((i mod 5)+1)).
- Clicking a point emits that bucket's timestamp; cursor becomes pointer when a
  click handler is present.

Semantic series pass explicit colors and must NOT use a slot:
total = var(--color-muted-foreground), 4xx = var(--color-warning),
5xx = var(--color-destructive). Slot hues rotate between light and dark, so a slot
color cannot carry meaning.
```

### 17-6. Three-column shell

```
Implement the shell from DESIGN.md §8-1 and §10-11 in <FRAMEWORK>.

Root: height 100dvh, min-height 0, background var(--color-background).
THERE IS NO HEADER BAR. Three columns, all starting at top: 0.

Left rail: 256px (48px collapsed), fixed, background var(--color-sidebar),
z-index 10, transition width 0.2s linear, border-right 0.
  - 48px header: a 32px toggle centered on the 48px icon-rail axis, then the wordmark
    at 0.875rem / weight 600 / tracking-tight, hidden when collapsed.
  - flat list of nav items: 36px tall, 12px inline padding, 12px gap, radius 0,
    NO group padding and NO menu gap (full bleed). Hover and current page fill with
    var(--color-sidebar-accent). Current page is EXACT pathname equality.
  - 48px footer: user name, theme toggle, sign-out. Collapses to a vertical icon stack.
  - Cmd/Ctrl+B toggles; persist to a cookie for 7 days.
  - Below 768px the rail is display:none and navigation becomes a 288px overlay
    drawer over a 50% black scrim.

Center: min-height 0, min-width 0, overflow hidden; inside it a vertical scroller
holding the routed page. Route change fades opacity 0->1 over 0.18s. Nothing else moves.

Right panel: 320px, flex-shrink 0, background var(--color-sidebar). No toggle,
always present.

The gap between center and right panel is 1px.

If your framework's sidebar component draws a right border via a variant-qualified
selector, cancel it with the SAME variant. A plain border-width:0 utility will lose.
```

### 17-7. Form Dialog + one admin form

```
Implement the Form Dialog from DESIGN.md §10-9 in <FRAMEWORK>, then build one
admin CRUD form on top of it.

Dialog: role="dialog" aria-modal="true" aria-labelledby, centered, max-width 672px,
max-height 85svh with overflow-y auto, radius 0, --shadow-lg, 16px padding,
16px gap. Scrim oklch(0 0 0 / 50%) at z-index 50. Escape and scrim click close.

Form: flex column, 12px gap. Each field is a flex column with 6px gap holding a
label, a control, an optional 0.75rem muted description, and an error slot.
Footer is right-aligned with 8px gap: a ghost Cancel and a submit button.

TWO ERROR CHANNELS ARE MANDATORY:
1. Field errors below their input, 0.75rem var(--color-destructive), linked by
   aria-describedby, with focus moved to the first invalid field on submit.
2. Server errors above the footer, same styling.
Omitting channel 1 produces a dead submit button that fires no request and shows
no message. Author every validation message explicitly — do not let your validation
library's untranslated defaults reach the UI.

Do NOT put native constraints (min/max/required) on inputs that your schema also
validates. Native validation blocks submit before your handler runs, so no message
appears anywhere. Choose one layer.

Loading state = the submit button is disabled. There is no spinner in this product.
```

### 17-8. State Triad + skeletons

```
Implement the four render states from DESIGN.md §10-10 in <FRAMEWORK>.

Three cards, visually identical, differing only in icon and copy:
  Empty      — inbox icon, inherited color,    title only
  LoadError  — triangle-alert, var(--color-destructive), title + a retry sentence
  QueryError — circle-alert, inherited color,  fixed title + the parser's message in mono
Card: background var(--color-card), 12px padding, centered, 24px gap, text-wrap balance.
ONLY the LoadError icon is colored.

Skeleton: background var(--color-muted), pulse animation. Four sizes —
page 384px, table 256px, panel 96px, list row 24px. NEVER a spinner.

Render order in every list view, as an inline block so the toolbar stays usable:
  pending -> transport error -> query-syntax error -> empty -> results

QueryError is NOT an HTTP failure: it arrives inside a 200 response as a field on
the payload. Do not route it through the transport-error path.

Guard on pending BEFORE empty. If you branch on data alone, changing a filter
flashes "no results" while the new request is in flight.
```

---

## 18. Verification Checklist

`[x]` = verified while authoring this document. `[ ]` = a runtime check the recreator must perform against their own output.

### Gate 1 — Tree coverage

- [x] Every style-relevant file in both surfaces appears in §15 with a label. 128 files enumerated, not sampled.
- [x] Every file labelled **leader** has a matching entry in §10 or §11.
- [x] Every file labelled **instance** names the pattern it composes and states its override, or states that it has none.
- [x] The 39 vendored files were established as byte-equivalent to the upstream registry by structural diff (quote style, whitespace and import aliases normalized), not by assumption. Class-token multisets match upstream exactly.
- [x] Build artifacts are listed as explicitly excluded, with the reason.
- [ ] Your recreation's file tree maps 1:1 onto §15's leader and instance entries.

### Gate 2 — Token usage

- [x] Every `--color-*` in §16-1 is referenced in §10 or §11 at least once. The four with **no** consumer in the source are named as such: `--font-serif`, `--text-3xs` (Surface B), and the two dead vendored components.
- [x] Every layout constant in §16-1 has a stated absolute pixel value in §6-2 or §8-2.
- [x] All 33 semantic tokens are re-bound in the dark block. Count verified against the light block, key for key.
- [x] All 7 HSL source values were converted and carry their original HSL string as a comment. Conversion validated against a known upstream anchor.
- [x] Fractional source hues are reproduced verbatim rather than re-rounded, with the reason stated in §3-1.
- [ ] Grep your component output for `#[0-9a-fA-F]{3,8}`, `rgb(`, `hsl(` — zero matches outside §16's palette comments.
- [ ] Grep for Tailwind palette utilities (`neutral-`, `slate-`, `zinc-`, `black/`, `white`) — zero matches in components you authored. The source has exactly 8, all inside vendored primitives or the deliberately theme-invariant code block.

### Gate 3 — Prompt validation

- [x] Each §17 prompt is self-contained: it names its section, states its constraints, and requires no other context.
- [x] Each prompt structurally forbids the four things that break this design: hex colors, non-zero radius, surface shadows, and `font-bold`.
- [x] The master prompt requires `.dark` **and** `[data-theme="dark"]` support.
- [x] Prompts carry the four failure modes the source actually hit: inert ellipsis without `max-width: 0`; variant-qualified border that a plain utility cannot cancel; a dead submit button from a missing field-error channel; an empty state flashing because pending was not guarded first.
- [ ] Run each prompt. Its output passes Gate 2's greps.

### Gate 4 — Framework independence

- [x] **Zero vendor component tags appear in any anatomy.** Verified by grep over §§10–13: no `<Dialog>`, `<Card>`, `<Button>`, `<v-*>`, `<chakra-*>`, `@radix-*` or equivalent. Every anatomy block is HTML, ARIA and CSS only.
- [x] Vendor *names* appear only as provenance annotations — where a measurement came from (§5-4, §5-5, §6-2, §6-5), which mechanism is wired (§4-6), and the mapping, sourcing and file-tree sections (§10-0, §14, §15, §1). None of them is required to build anything in §10 or §11.
- [x] Every measurement the component library supplies implicitly is stated as an absolute pixel value in §14-4 — 22 of them, including the five the project overrode and the three it cancelled.
- [x] The animation library, the chart library and the icon library each have a stated replacement path in §14-2 and §14-7.
- [x] The two variant-generator forks between surfaces are documented rather than silently unified (§14-5).
- [ ] Your recreation compiles with zero dependencies on the source's component library.

### Gate 5 — Statistical consistency

| Statistic | Value | Cross-checked against |
|---|---|---|
| Palette values (Surface A) | **40** | §3-2 enumeration ↔ §16-1 block |
| Semantic tokens, light | **33** | §3-3 table ↔ §16-1 |
| Semantic tokens, dark | **33** | §4-3 table ↔ §16-1 |
| Semantic tokens, Surface B | **24** | §3-6 (33 − `--warning` − 8 `--sidebar-*`) |
| HSL → OKLCH conversions | **7** | §3-1 ↔ §3-2 sidebar block |
| Font weights | **4** (400, 500, 600, 800) | §5-4; `font-bold` count = 0 |
| Type scale steps in use | **9** total; Surface A effectively **3** (11/12/14px) | §5-5 |
| Radius values, Surface A | **1** (`0rem`), 4 derived pinned to it | §6-3 |
| Radius values, Surface B | **1** base (`0.375rem`), 4 derived by `calc` | §6-3 |
| Shadow steps | **8**; 4 are `none` on Surface A | §6-5 |
| Shadow utilities in application code | **0** | §6-5 |
| Z-index values | **3** (10, 20, 50) | §7-5 |
| Breakpoints declared | **5**; structurally used **1** (`md` 768px), plus one `xl` and one `lg` one-off; `2xl` unused | §9-2 |
| Motion durations authored | **2** (`0.18s`, `0.24s`), **1** curve | §7-1 |
| Chart animation | **disabled** | §10-4 |
| Primary component archetypes | **13** | §10-1 – §10-12, §10-14 |
| Supporting patterns | **8** | §10-13 |
| Page archetypes | **7** | §11-A – §11-G (+ Surface B in §11-H) |
| Routes | **30** + catch-all | §8-1, §15 |
| Navigation items | **18**, one flat group | §10-11 |
| Framework mapping rows | **32** | §14-1 |
| Files inspected | **128** | §15 summary |
| Raw color literals outside the theme files | **1** occurrence set, non-applied (attribute-selector matchers in a vendored chart primitive) | §14-8, Gate 2 |

### Gate 6 — Known source defects carried forward, not silently fixed

This document reproduces the source faithfully. Six things in it are defects or gaps. Each is documented where it lives, and each is a decision the recreator must make explicitly.

| # | Issue | Where | Recommendation |
|---|---|---|---|
| 1 | No `prefers-reduced-motion` handling anywhere | §7-4 | **Add it.** Costs nothing, changes nothing for users who did not ask. A ready block is in §16-1. |
| 2 | Overlay enter/exit utilities are inert — the plugin that defines them is not installed | §7-3 | **Leave inert** unless you intend the product to move more. Installing the plugin is a design change. |
| 3 | Dashboard has no `<h1>`–`<h6>`; panel titles render as `div` | §7-4 | **Fix.** Render titles as `h2`; the visual result is identical. |
| 4 | The 320px context panel has no responsive rule — at 375px it leaves ~55px of content | §13-1 | **Fix**, but know it is a change: hide below 768px and surface as a second drawer. |
| 5 | Toast host resolves its theme independently of the app, and is never invoked | §4-6, §14-8 | **Wire it or remove it.** Half-mounted is the worst state. |
| 6 | One route sets a 3-column grid with no breakpoint, so it renders 3 columns on a phone | §13-3 | **Fix** if shipping; reproduce if matching. Do not do either silently. |

Two further divergences are **forks, not defects** — the source is internally consistent on each side and a recreator must simply choose:

- `--text-2xs` is `0.6875rem` on the dashboard and `0.625rem` on the public pages (§5-5).
- The badge variant sets differ: the public surface has tinted `warning`/`success`; the dashboard has `ghost`/`link` (§14-5).