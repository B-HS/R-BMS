# rbms docs

Project documentation for [rbms](../README.md) — a Rust port of the reference implementation's BMS-player PLAY core.

| Doc | What |
|---|---|
| [architecture.md](architecture.md) | Crate map, end-to-end data flow, the realtime play loop, determinism, the UI state machine. **Start here.** |
| [crates.md](crates.md) | Per-crate API reference (roles, public types/fns, invariants, gotchas, tests). |
| [development.md](development.md) | Build / run / test, persistence files, the app module layout, how to extend, **conventions**. |
| [theme.md](theme.md) | The `theme.ron` UI-theme schema — the reference a web front-end authors themes against. |
| [roadmap.md](roadmap.md) | Deferred work + intentional edge-case behaviours pinned by tests. |

A fresh session should also read the repo-root `CLAUDE.md` (local) for the hard rules and quick context.
