export const meta = {
  name: "rbms-phase-h",
  description:
    "Phase H: collect every minor follow-up left by Phases I, B, C, D, E, F and G (PROCESS.md, history docs, review leftovers), fix them, verify, and close the docs",
  phases: [
    {
      title: "Collect",
      detail: "inventory of open minor items from docs and review leftovers",
    },
    { title: "Fix", detail: "parallel fixes by area with disjoint files" },
    { title: "Verify", detail: "gate, headless renders, docs close-out" },
  ],
};
const RULES = `
HARD RULES: repo /Users/gkn/R-BMS. Never write the reference Java player's name ("beat"+"oraja") or its path. Rust: no "//" or "/* */" comments (only "///" on public items), no TODO, no emojis, no magic numbers. Never read/write .env. No git state-changing commands. Every change pinned by a test; cargo fmt --all; cargo clippy --workspace --all-targets -- -D warnings must stay clean; cargo test --workspace green. Korean report, English identifiers. Only modify files inside the ownership you are given.
`;
const INV = {
  type: "object",
  properties: {
    items: {
      type: "array",
      items: {
        type: "object",
        properties: {
          id: { type: "string" },
          source: { type: "string" },
          title: { type: "string" },
          files: { type: "array", items: { type: "string" } },
          area: {
            type: "string",
            enum: [
              "render-ui",
              "player-app",
              "crates-core",
              "web",
              "docs-only",
              "user-owned",
            ],
          },
          detail: { type: "string" },
        },
        required: ["id", "source", "title", "files", "area", "detail"],
      },
    },
    notes: { type: "string" },
  },
  required: ["items", "notes"],
};
const TEXT = {
  type: "object",
  properties: {
    summary: { type: "string" },
    files_changed: { type: "array", items: { type: "string" } },
    verification: { type: "string" },
    ok: { type: "boolean" },
  },
  required: ["summary", "files_changed", "verification", "ok"],
};

phase("Collect");
const inv = await agent(
  `${RULES}
TASK COLLECT (read-only): build the inventory of every still-open minor follow-up. Sources: docs/PROCESS.md (the "H 경미 후속 일괄" line and every "후속"/"보류"/"미확인" mention in the Phase I/B/C/D/E/F/G blocks), docs/history/2026-09-*.md and 2026-09-10-*.md (sections named 알려진 제약/후속/미확인/남은), docs/acknowledge/reference-divergences.md rows marked as temporary or "해소 조건", the web history (docs/history/2026-09-09-web-nextjs-ir-server.md 알려진 제약: md5-only submission, Vercel Blob replay storage, light/dark screenshot) and docs/web/*.md. For each item give: id, source (file:line), title, the concrete files that would change, an area, and enough detail for a fixer who has not read the docs. Classify user-owned items (release with the user's key, live listening check) as area user-owned and do NOT include Phase-scale work (bmson if already done, new features). Group so that items in different areas touch disjoint files; note conflicts in notes.`,
  {
    label: "collect",
    phase: "Collect",
    model: "opus",
    effort: "high",
    schema: INV,
  },
);
const items = (inv ? inv.items : []).filter((i) => i.area !== "user-owned");
log(`inventory: ${items.length} actionable items`);

phase("Fix");
const AREAS = ["render-ui", "player-app", "crates-core", "web", "docs-only"];
const fixes = await parallel(
  AREAS.map((area) => () => {
    const mine = items.filter((i) => i.area === area);
    if (!mine.length) return Promise.resolve(null);
    return agent(
      `${RULES}
TASK FIX area ${area}. OWNERSHIP: exactly the files listed in your items (plus new test files next to them); if two of your items need the same file, do them sequentially. Items: ${JSON.stringify(mine)}. For each: fix the root cause, pin with a test (or a render test for UI: e.g. the toast must not overlap the song-select hint row — assert bounding boxes), or if an item is a documentation-only correction update the doc. For web items run cd web && bun run verify. Report per item: done / skipped with reason.`,
      {
        label: `fix-${area}`,
        phase: "Fix",
        model: "opus",
        effort: "high",
        schema: TEXT,
      },
    );
  }),
);

phase("Verify");
const verify = await agent(
  `${RULES}
TASK VERIFY: cargo fmt --all -- --check (run cargo fmt --all first if needed), cargo test --workspace totals, cargo clippy --workspace --all-targets -- -D warnings, cd web && bun run verify (if web changed), forbidden-name grep, new "//" lines. Render the song-select screen with a toast visible headlessly to /private/tmp/claude-501/-Users-gkn-R-BMS/847c1230-e223-4d0c-adb0-2d6113bbb669/scratchpad/phase-h-shots/select-toast.png and confirm no overlap with the hint row. Then update docs/PROCESS.md: tick the H line listing every item and its outcome (done/skipped/user-owned), and write docs/history/2026-09-10-phase-h-followups.md (Korean). Inputs: INVENTORY ${JSON.stringify(items.map((i) => i.id + " " + i.title))} || FIXES ${JSON.stringify(fixes.filter(Boolean).map((f) => f.summary.slice(0, 500)))}. OWNERSHIP for writing: those two docs only.`,
  {
    label: "verify",
    phase: "Verify",
    model: "sonnet",
    effort: "medium",
    schema: TEXT,
  },
);
return { inv, fixes: fixes.filter(Boolean), verify };
