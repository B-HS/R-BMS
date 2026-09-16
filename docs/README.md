# rbms docs

Project documentation for [rbms](../README.md), a Rust port of the reference implementation's BMS-player PLAY core plus the `web/` IR server. Last audited 2026-09-16 after Phase G·H integration and CI repair.

A fresh session starts with [PROCESS.md](PROCESS.md), then follows its linked execution, architecture, release or phase document. [HANDOFF.md](HANDOFF.md) is a preserved 2026-09-10 snapshot, not a current task list.

| Doc | What |
|---|---|
| [PROCESS.md](PROCESS.md) | Current single-source checklist, phase status, verification evidence and next work. **Start here.** |
| [HANDOFF.md](HANDOFF.md) | Preserved 2026-09-10 session snapshot. Its branch and TODO instructions are historical. |
| [architecture.md](architecture.md) | Crate map, end-to-end data flow, realtime play loop, determinism, the `Stage` UI state machine, data-driven surfaces. |
| [crates.md](crates.md) | Per-crate API reference (roles, public types/fns, invariants, gotchas, tests). |
| [development.md](development.md) | Build / run / test, persistence files, app module layout, conventions. |
| [theme.md](theme.md) | The `theme.ron` UI-theme schema. |
| [roadmap.md](roadmap.md) | Deferred work and intentional edge-case behaviours pinned by tests. |
| [ci-release.md](ci-release.md) | CI matrix and the Phase R release boundary. `prod`, first release/tag and signing or deployment credentials remain unstarted. |
| [plan/](plan/) | Phase A~G planning and specifications, research reports, and completed Phase G wiring instructions. Phase H is recorded in history; Phase R remains a future key-dependent release task. |
| [acknowledge/](acknowledge/) | Decisions (`2026-09-09-enhancement-decisions.md`), reference divergence ledger (`reference-divergences.md`), web design tokens (`design.md`). |
| [history/](history/) | Immutable phase evidence and review outcomes. The latest implementation records are [Phase G](history/2026-09-16-phase-g-data-long-tail.md) and [Phase H](history/2026-09-16-phase-h-minor-followups.md); `2026-09-10-session-wrap-up.md` is historical only. |
| [quality-assurance/](quality-assurance/) | Hardware/server-dependent checks. [Phase H manual checks](quality-assurance/2026-09-16-phase-h-manual-checks.md) define the still-unrun AUDIO, Practice, Course and Multi-IR checks. |
| [bug/](bug/) | Bug reports with cause and fix (e.g. the Windows audio enumeration crash). |
| [reference/](reference/) | IR API contract (`ir-api.md`), Windows compatibility, UI specs, reference mechanics notes. |
| [web/](web/) | Web architecture, shadcn component list, task split. |
| [backend/](backend/) | Early backend design; superseded by `web/` but kept for the data model rationale. |
| [utils/workflows/](utils/workflows/) | Historical orchestration scripts used for phases B to H; do not treat interrupted workflow instructions as current work without checking PROCESS. |
