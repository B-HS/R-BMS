export const meta = {
  name: "rbms-phase-f",
  description:
    "Phase F UX and feature upgrades per docs/plan/2026-09-09-phase-f-spec.md: remap ownership to the post-Phase-C tree, F0 plumbing, F1-F4 parallel branches, adversarial review, fix, headless render checks, docs",
  phases: [
    {
      title: "Remap",
      detail: "baseline + ownership table for the current tree",
    },
    {
      title: "F0",
      detail:
        "serial plumbing (descriptor rows, notify/toast bus, key enums, golden split)",
    },
    {
      title: "F1-F4",
      detail:
        "select UI | options overlay | result & target | loading & shell (parallel)",
    },
    { title: "Gate", detail: "fmt/test/clippy -D warnings" },
    {
      title: "Review",
      detail: "parity | UX & threads | convention (adversarial)",
    },
    { title: "Fix", detail: "apply confirmed findings" },
    { title: "Verify", detail: "final gate + screenshots + docs" },
  ],
};

const SPEC = "docs/plan/2026-09-09-phase-f-spec.md";
const RULES = `
HARD RULES (violations are rejected):
- Repo /Users/gkn/R-BMS. The spec ${SPEC} is the SSOT for Phase F (read fully: §1 parity data, §2 work items F0-F4, §3 descriptor rows, §4 key bindings, §5 ownership, §8 critique). Also docs/acknowledge/2026-09-09-enhancement-decisions.md (decision 3: option overlay panel is the main path; decision 6: Esc hold/double option) and docs/acknowledge/reference-divergences.md.
- Phases C and D landed AFTER the spec: the app is split into apps/rbms-player/src/stage/*.rs with a settings descriptor table, crates/rbms-config, rbms-store, rbms-library; judge parity is complete. The spec's file names (app_select.rs, app_play.rs, settings.rs ...) may no longer exist — use the OWNERSHIP TABLE produced by the Remap step, which maps each branch to real current files. Re-anchor everything by symbol name.
- NEVER write the literal name of the reference Java player ("beat"+"oraja") or its path anywhere; say "the reference implementation" and cite bare file names. You may READ <reference-root> for parity; numeric tables come from it and are pinned by tests.
- Rust: NO "//" or "/* */" comments; only brief English "///" on public items. No TODO. No emojis. No magic numbers.
- Only modify files in your OWNERSHIP; other needs go to needs_from_others. Never read/write .env. No git state-changing commands.
- Every feature: precise unit tests + headless CpuCanvas render tests for new screens/panels; cargo fmt --all; cargo test for touched crates; cargo clippy --workspace --all-targets -- -D warnings must pass before you finish (CI gate).
- Keep files under ~800 lines: new features go into new modules.
- Report in Korean; identifiers English.
`;
const REPORT = {
  type: "object",
  properties: {
    summary: { type: "string" },
    files_changed: { type: "array", items: { type: "string" } },
    public_api: { type: "string" },
    tests: { type: "string" },
    needs_from_others: { type: "array", items: { type: "string" } },
  },
  required: [
    "summary",
    "files_changed",
    "public_api",
    "tests",
    "needs_from_others",
  ],
};
const FINDINGS = {
  type: "object",
  properties: {
    findings: {
      type: "array",
      items: {
        type: "object",
        properties: {
          severity: { type: "string", enum: ["critical", "major", "minor"] },
          file: { type: "string" },
          line: { type: "integer" },
          title: { type: "string" },
          evidence: { type: "string" },
          fix: { type: "string" },
        },
        required: ["severity", "file", "title", "evidence", "fix"],
      },
    },
    notes: { type: "string" },
  },
  required: ["findings", "notes"],
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
const REMAP = {
  type: "object",
  properties: {
    baseline: { type: "string" },
    ownership: {
      type: "string",
      description:
        "For F0, F1, F2, F3, F4: the complete list of CURRENT file paths (and new files to create) each branch may write, with the proof that F1..F4 sets are pairwise disjoint",
    },
    notes: { type: "string" },
  },
  required: ["baseline", "ownership", "notes"],
};

phase("Remap");
const remap = await agent(
  `${RULES}
TASK REMAP (read-only, modify nothing): measure the baseline (git rev-parse --short HEAD, cargo test --workspace totals, cargo clippy --workspace --all-targets -- -D warnings pass/fail, cargo fmt check) and produce the OWNERSHIP TABLE: map spec §5 (F0 plumbing; F1 select UI; F2 options overlay/play options; F3 result/target/graphs; F4 loading/background/shell/toast/dialog/textedit) onto the CURRENT tree (list apps/rbms-player/src/**/*.rs, crates/rbms-render/src/**, crates/rbms-config/**, crates/rbms-play/** with line counts; find where select/play/result/loading stages, settings descriptors, key maps, golden tests and eprintln calls live now). Every file Phase F will touch gets exactly one owner; F1..F4 pairwise disjoint; shared files (stage/mod.rs dispatch, key enums, descriptor registry, render lib.rs, golden harness, Cargo.toml) go to F0 which runs alone first and pre-registers everything the branches need (new modules as empty stubs, new descriptor rows, new key bindings, toast/notify bus, golden test split per branch).`,
  {
    label: "remap",
    phase: "Remap",
    model: "opus",
    effort: "high",
    schema: REMAP,
  },
);
const OWN = remap ? remap.ownership : "";

phase("F0");
const f0 = await agent(
  `${RULES}
OWNERSHIP TABLE: ${OWN}
TASK F0 plumbing (spec §F0 items 1-10, serial): implement everything the table assigns to F0 — new empty modules, descriptor rows for all §3 settings, key bindings from §4 (incl. option overlay hold key, result R/N, sort key, favourites), a notify/toast bus that replaces every eprintln in the app (drain point in the frame loop, render stub), golden harness split into per-branch test files, Cargo dependencies (arboard for clipboard paste if the spec keeps it). Zero behaviour change beyond the plumbing; all tests green; clippy -D warnings clean. Report public_api exactly (bus API, descriptor ids, key enum variants, module names).`,
  {
    label: "F0-plumbing",
    phase: "F0",
    model: "opus",
    effort: "max",
    schema: REPORT,
  },
);

phase("F1-F4");
const BR = [
  {
    key: "F1-select",
    task: "spec §F1: 12 sort orders (reference BarSorter parity), difficulty/mode filters, favourites, DJ LEVEL display, BGA thumbnail, search view restore, preview debounce by time + fade + volume (use Phase B bus API), text input cursor/paste.",
  },
  {
    key: "F2-options",
    task: "spec §F2: option overlay panel (decision 3) with hold key binding, HID+/SUD+ & HID+&SUD+ with white number, hi-speed 0.01-20, lane cover fine adjust, floating hi-speed (MAIN/MAX/MIN/START BPM), BPM change preview, LANE COVER / SPEED ADJUST, FLIP/BATTLE/SYNC-RAN/LEGACY NOTE/5KEYS, CN/HCN separate skin params, judge text position, key/scratch FAST-SLOW split, Esc hold/double option (decision 6). Reference formulas from §1.2/§1.4 pinned by tests.",
  },
  {
    key: "F3-result",
    task: "spec §F3: result screen retry/next-song, TARGET/PACEMAKER (MAX, RATE_x, RANK_NEXT, LOCAL_BEST; IR targets via chart_ranking/player_best/rivals async cache from Phase I by function name), 27-percentile graph and MAX- display, gauge trend, judge distribution and timing histogram on the result screen. Reference §1.1 target table pinned by tests.",
  },
  {
    key: "F4-shell",
    task: "spec §F4: table fetch / BGA decode / rfd dialogs moved to background threads with loading stage progress display, error toast/status line rendering (U1) driven by the F0 bus, resize letterbox (K6), dialog/textedit modules.",
  },
];
const br = await parallel(
  BR.map(
    (b) => () =>
      agent(
        `${RULES}
OWNERSHIP TABLE: ${OWN}
F0 public API: ${f0 ? f0.public_api : ""}
TASK branch ${b.key}: write ONLY the files the table assigns to ${b.key}. ${b.task} Self-check: git diff --name-only shows only your files (plus Cargo.lock).`,
        {
          label: b.key,
          phase: "F1-F4",
          model: "opus",
          effort: "max",
          schema: REPORT,
        },
      ),
  ),
);
const brok = br.filter(Boolean);

phase("Gate");
const gate = await agent(
  `${RULES}
TASK GATE (ownership: every file in the table; fix only what is needed for green): cargo fmt --all; cargo build --workspace; cargo test --workspace; cargo clippy --workspace --all-targets -- -D warnings. Branch needs: ${JSON.stringify(brok.map((r) => r.needs_from_others))}. Resolve cross-branch needs (wiring one branch's module into another's dispatch) here. Report exact result lines; ok=true only when all green.`,
  { label: "gate", phase: "Gate", model: "opus", effort: "high", schema: TEXT },
);

phase("Review");
const REVIEW_RULES = `${RULES}
Adversarial reviewer: verified findings only, file:line + evidence, do not modify files. critical = wrong gameplay numbers / render thread blocked / data loss; major = spec item missing or untested; minor = convention.`;
const reviews = await parallel([
  () =>
    agent(
      `${REVIEW_RULES} REVIEW PARITY: compare every number and formula in the new option/target/sort/hi-speed code against the reference sources cited in spec §1 (TargetProperty.java, PlayConfig.java, LaneRenderer.java, BarSorter.java) and the LIFT/green-number decisions in docs/acknowledge; report deviations.`,
      {
        label: "review-parity",
        phase: "Review",
        model: "opus",
        effort: "high",
        schema: FINDINGS,
      },
    ),
  () =>
    agent(
      `${REVIEW_RULES} REVIEW UX/THREADS: nothing blocks the render thread (table fetch, BGA decode, rfd, IR targets), stale results ignored, overlay panel input routing does not leak keys to play, Esc hold/double behaves per decision 6, resize letterbox math, toast lifetime, loading progress, favourites persistence, key conflicts (§4), Phase I/B/D behaviours intact.`,
      {
        label: "review-ux",
        phase: "Review",
        model: "opus",
        effort: "high",
        schema: FINDINGS,
      },
    ),
  () =>
    agent(
      `${REVIEW_RULES} REVIEW CONVENTION/QUALITY: "//" comments, forbidden name, magic numbers, files > 800 lines, clippy -D warnings, test quality (assert real values, not smoke), descriptor rows/ranges vs spec §3, CpuCanvas render tests present for overlay/result graphs/toast.`,
      {
        label: "review-quality",
        phase: "Review",
        model: "sonnet",
        effort: "medium",
        schema: FINDINGS,
      },
    ),
]);
const findings = reviews.filter(Boolean).flatMap((r) => r.findings);
log(`review: ${findings.length} findings`);

phase("Fix");
const fix = findings.length
  ? await agent(
      `${RULES}
TASK FIX (ownership: every file in the table): apply these verified findings, pin each with a test, refute with evidence if wrong. Findings: ${JSON.stringify(findings)}. Finish with cargo fmt --all, cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings.`,
      {
        label: "fix",
        phase: "Fix",
        model: "opus",
        effort: "max",
        schema: TEXT,
      },
    )
  : null;

phase("Verify");
const verify = await agent(
  `${RULES}
TASK VERIFY (fix nothing except cargo fmt --all): exact lines for fmt check, cargo test --workspace per crate vs baseline (${remap ? remap.baseline.slice(0, 200) : ""}), clippy -D warnings, forbidden-name grep, new "//" lines, git status --short. Render headless PNGs of the option overlay, result screen with graphs, select screen with filters and a toast via the CpuCanvas examples/tests (write them to the scratchpad /private/tmp/claude-501/-Users-gkn-R-BMS/847c1230-e223-4d0c-adb0-2d6113bbb669/scratchpad/phase-f-shots/ and list the paths). Then write docs/history/2026-09-09-phase-f-ux.md (Korean) and tick Phase F in docs/PROCESS.md with the numbers; append divergences to docs/acknowledge/reference-divergences.md if any. Inputs: F0 ${f0 ? f0.summary.slice(0, 400) : ""} || BRANCHES ${JSON.stringify(brok.map((r) => r.summary.slice(0, 400)))} || GATE ${gate ? gate.verification.slice(0, 300) : ""} || FINDINGS ${JSON.stringify(findings.map((f) => f.severity + " " + f.file + " " + f.title))} || FIX ${fix ? fix.summary.slice(0, 400) : "none"}.`,
  {
    label: "verify",
    phase: "Verify",
    model: "sonnet",
    effort: "medium",
    schema: TEXT,
  },
);

return { remap, f0, branches: brok, gate, findings, fix, verify };
