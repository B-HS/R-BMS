# rbms docs

Project documentation for [rbms](../README.md), a Rust port of the reference implementation's BMS-player PLAY core plus the `web/` IR server. Last updated 2026-09-10 (`dev` after `3c61a53`).

A fresh session starts with [HANDOFF.md](HANDOFF.md), then [PROCESS.md](PROCESS.md).

| Doc | What |
|---|---|
| [HANDOFF.md](HANDOFF.md) | Session snapshot: goals, done / in progress / next, decisions, rules, environment. **Start here.** |
| [PROCESS.md](PROCESS.md) | Single-source checklist of every phase with commits and gate numbers. |
| [architecture.md](architecture.md) | Crate map, end-to-end data flow, realtime play loop, determinism, the `Stage` UI state machine, data-driven surfaces. |
| [crates.md](crates.md) | Per-crate API reference (roles, public types/fns, invariants, gotchas, tests). |
| [development.md](development.md) | Build / run / test, persistence files, app module layout, conventions. |
| [theme.md](theme.md) | The `theme.ron` UI-theme schema. |
| [roadmap.md](roadmap.md) | Deferred work and intentional edge-case behaviours pinned by tests. |
| [ci-release.md](ci-release.md) | CI matrix and the release procedure (release itself is done by the maintainer). |
| [plan/](plan/) | `2026-09-09-enhancement-plan.md` (phases A to G, decisions, limits), per-phase specs `2026-09-09-phase-{b,c,d,e,f,g}-spec.md`, research reports, Phase G wiring instructions. |
| [acknowledge/](acknowledge/) | Decisions (`2026-09-09-enhancement-decisions.md`), reference divergence ledger (`reference-divergences.md`), web design tokens (`design.md`). |
| [history/](history/) | What each phase did, reviewed and fixed, with numbers; `2026-09-10-session-wrap-up.md` is the resume procedure. |
| [bug/](bug/) | Bug reports with cause and fix (e.g. the Windows audio enumeration crash). |
| [reference/](reference/) | IR API contract (`ir-api.md`), Windows compatibility, UI specs, reference mechanics notes. |
| [web/](web/) | Web architecture, shadcn component list, task split. |
| [backend/](backend/) | Early backend design; superseded by `web/` but kept for the data model rationale. |
| [utils/workflows/](utils/workflows/) | The orchestration scripts used for phases B to H, with a README on how to run them. |
