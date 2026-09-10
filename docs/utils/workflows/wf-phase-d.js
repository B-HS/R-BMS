export const meta = {
  name: "rbms-phase-d",
  description:
    "Phase D judgement parity completion per docs/plan/2026-09-09-phase-d-spec.md: D0 decl -> D1 gauges | D2 windows/algorithms -> D3 matcher/LN/scratch -> D4 play integration -> D5 JUDGE tab + decision 12, adversarial review, fix, docs",
  phases: [
    { title: "Baseline", detail: "measure at HEAD" },
    {
      title: "D0",
      detail: "judge crate declarations, tests.rs move, LnKind::Undefined",
    },
    {
      title: "D1-D2",
      detail: "gauges (9 x 5 sets, GAS) | windows rule + algorithms (parallel)",
    },
    {
      title: "D3",
      detail: "matcher fold, judge_vanish, BSS/MSS, CN deferral, HCN ticks",
    },
    { title: "D4", detail: "play integration, 24K mode, lnmode" },
    {
      title: "D5",
      detail:
        "JUDGE settings tab, assist/score flags, lamp downgrade, replay fields, rule_version",
    },
    {
      title: "Review",
      detail: "reference parity | engine correctness | app/UX (adversarial)",
    },
    { title: "Fix", detail: "apply confirmed findings" },
    { title: "Verify", detail: "gate + docs" },
  ],
};

const SPEC = "docs/plan/2026-09-09-phase-d-spec.md";
const RULES = `
HARD RULES (violations are rejected):
- Repo /Users/gkn/R-BMS. The spec ${SPEC} is the SSOT for Phase D (read fully: §0 anchors, §1-§7 items with exact reference tables, §8 ownership, §9 order, the critique section). Also docs/acknowledge/2026-09-09-enhancement-decisions.md (decision 12) and docs/acknowledge/reference-divergences.md.
- Phase C (structure refactor) landed AFTER the spec was written: crates/rbms-judge may already contain algorithm.rs, data.rs, data/*.ron and a JudgeAlgorithm enum; the app is split into apps/rbms-player/src/stage/*.rs with crates/rbms-config (settings), crates/rbms-store (scores/replays), crates/rbms-library. FIRST read the current code, then implement only what is still missing; extend existing types instead of duplicating; re-anchor spec line numbers by symbol name.
- NEVER write the literal name of the reference Java player ("beat"+"oraja") or its path anywhere. Say "the reference implementation" and cite bare file names (JudgeProperty.java:266). You may READ <reference-root> for parity — every numeric table must be copied from the reference source and pinned by a test that compares the Rust constants to the literal numbers.
- Rust: NO "//" or "/* */" comments; only brief English "///" on public items. No TODO. No emojis. No magic numbers.
- Only modify files in your OWNERSHIP; report other needs under needs_from_others. Never read/write .env. No git state-changing commands.
- Every change: precise unit tests; cargo fmt --all; cargo test for touched crates; cargo clippy --all-targets for touched crates with 0 warnings (the CI gate is now -D warnings — if Phase C flipped it, the whole workspace must stay warning-free: run cargo clippy --workspace --all-targets -- -D warnings before finishing).
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

phase("Baseline");
const baseline = await agent(
  `${RULES}
TASK BASELINE (read-only): git rev-parse --short HEAD; cargo test --workspace totals; cargo clippy --workspace --all-targets -- -D warnings (pass/fail); cargo fmt --all -- --check; list crates/rbms-judge/src/*.rs and crates/rbms-judge/data/* with line counts and the public items of algorithm.rs/data.rs if they exist; list apps/rbms-player/src/stage/*.rs; list where settings rows are defined now (grep setting descriptors / JUDGE tab). Put everything in verification — later agents depend on this map.`,
  {
    label: "baseline",
    phase: "Baseline",
    model: "sonnet",
    effort: "medium",
    schema: TEXT,
  },
);
const MAP = baseline ? baseline.verification : "";

phase("D0");
const d0 = await agent(
  `${RULES}
CODE MAP: ${MAP}
TASK D0 judge-decl (spec §8-0): OWNERSHIP crates/rbms-judge/src/lib.rs, crates/rbms-model/src/lib.rs. Ensure module declarations exist for gauge_tables.rs, ln.rs and (if missing) algorithm.rs (create empty stubs with a one-line //! doc), move the inline #[cfg(test)] mod tests body of lib.rs into crates/rbms-judge/src/tests.rs if it is still inline, add LnKind::Undefined to rbms-model (§6) with serde/round-trip tests. Do not re-export new types. cargo test --workspace must stay green.`,
  {
    label: "D0-decl",
    phase: "D0",
    model: "opus",
    effort: "high",
    schema: REPORT,
  },
);

phase("D1-D2");
const d12 = await parallel([
  () =>
    agent(
      `${RULES}
CODE MAP: ${MAP}
TASK D1 judge-gauge (spec §2 entirely). OWNERSHIP crates/rbms-judge/src/gauge.rs, crates/rbms-judge/src/gauge_tables.rs, crates/rbms-judge/data/gauge.ron (if Phase C introduced RON tables, extend them instead of adding a second source of truth; the Rust consts and the RON must agree and a test proves it). Deliver: J20 parallel update of all 9 gauge types, J21 the 5 sets x 9 elements (45 entries) with the exact reference numbers, J22 MODIFY_DAMAGE applied once at construction, J26 GAS + bottom shiftable, ClearType::LightAssistEasy + AssistEasy downgrade rules, clear-lamp mapping table. Tests pin every number against the reference literal tables (§2 "정확 테이블").`,
      {
        label: "D1-gauge",
        phase: "D1-D2",
        model: "opus",
        effort: "max",
        schema: REPORT,
      },
    ),
  () =>
    agent(
      `${RULES}
CODE MAP: ${MAP}
TASK D2 judge-window (spec §1 and §3). OWNERSHIP crates/rbms-judge/src/windows.rs, crates/rbms-judge/src/algorithm.rs, crates/rbms-judge/data/judge.ron (extend, single source of truth with a consistency test). Deliver: J17 JudgeAlgorithm 4 kinds (Combo/Duration/Lowest/Score) with NoteType-based prefer() exactly per the reference JudgeAlgorithm.java (if Phase C already added the enum, complete/verify it and keep the default the spec/critique settled on, recording the divergence), J23 JudgeWindowRule (fixjudge + fixmin/fixmax with the exact reference arrays), judge_code/in_good_band/in_ms_band helpers, J12 stage 2 key/scratch width split, and judge_property_tables_pin tests comparing every array with the reference literals.`,
      {
        label: "D2-window",
        phase: "D1-D2",
        model: "opus",
        effort: "max",
        schema: REPORT,
      },
    ),
]);
const d12ok = d12.filter(Boolean);

phase("D3");
const d3 = await agent(
  `${RULES}
CODE MAP: ${MAP}
D1/D2 APIs: ${d12ok.map((r) => r.public_api).join("\n---\n")}
TASK D3 judge-matcher (spec §1 fold, §3-B, §4, §5). OWNERSHIP crates/rbms-judge/src/matcher.rs, crates/rbms-judge/src/ln.rs, crates/rbms-judge/src/tests.rs. Deliver: two-stage candidate fold per algorithm (fixing the critique's flaw about already-judged notes), judge_vanish/MissCondition wiring, J9/A10 scratch windows with forward/backward two-key input and BSS/MSS including the release condition the critique added, J24 CN deferral, plain LN confirmation and HCN continuous gauge ticks (reference LongNote handling), wiring of the 9 parallel gauges into the matcher. Tests: synthetic fixtures per rule, replay-determinism (same input -> same result), and byte-for-byte comparisons where the spec lists reference cases. Existing tests must keep passing (or be corrected with an explicit reason when the old expectation was a divergence — list each).`,
  {
    label: "D3-matcher",
    phase: "D3",
    model: "opus",
    effort: "max",
    schema: REPORT,
  },
);

phase("D4");
const d4 = await agent(
  `${RULES}
CODE MAP: ${MAP}
Judge APIs: ${[...d12ok, d3]
    .filter(Boolean)
    .map((r) => r.public_api)
    .join("\n---\n")}
TASK D4 play-integration (spec §6 and the play wiring rows of §2-§5). OWNERSHIP crates/rbms-play/**, crates/rbms-model/src/mode.rs, crates/rbms-chart/**. Deliver: 24K Mode (KEYBOARD) with for_mode branch, GAS frame hook, set_judge_window_rates via JudgeWindowRule, gauge set selection by mode, J25 lnmode enforcement in to_model, PlaySession exposes the new judge state (algorithm, window rule, gauge set, GAS) for the app. Tests incl. the autoplay/replay reproduction tests staying green.`,
  {
    label: "D4-play",
    phase: "D4",
    model: "opus",
    effort: "max",
    schema: REPORT,
  },
);

phase("D5");
const d5 = await agent(
  `${RULES}
CODE MAP: ${MAP}
Crate APIs: ${[...d12ok, d3, d4]
    .filter(Boolean)
    .map((r) => r.public_api)
    .join("\n---\n")}
TASK D5 app-judge-tab (spec §7, §6.5, decision 12). OWNERSHIP apps/rbms-player/** , crates/rbms-config/**, crates/rbms-store/**, crates/rbms-render/src/result.rs, crates/rbms-ir/src/mapping* (only if the assist flags mapping lives there). Deliver: JUDGE settings tab rows via the descriptor table (algorithm, per-judge widths PG/GR/GD for key and scratch, LN margin rate, lnmode, GAS, bottom shiftable, TARGET) with ranges/steps; decision 12 wiring: judge width rate > 100 or LN margin rate > 100 -> assist=2 and score=false (IR submit blocked, replay not saved, EX/BP/combo not updated, lamp and play count still updated), assist > 0 -> no FC/PERFECT/MAX and lamp downgrade LightAssistEasy/AssistEasy, AUTO SCRATCH = assist 1; scratch reverse-rotation key bindings incl. keyconfig rows; replay records algorithm and widths; rule_version bump with score migration (old scores keep loading, marked); DJ rank pin test (§6.5); result screen shows the new lamps. Headless CpuCanvas render test of the JUDGE tab rows. All existing Phase I/B behaviour intact.`,
  {
    label: "D5-app",
    phase: "D5",
    model: "opus",
    effort: "max",
    schema: REPORT,
  },
);

phase("Review");
const REVIEW_RULES = `${RULES}
Adversarial reviewer: only verified findings with file:line and evidence; do not modify files. critical = judgement/gauge/score number differs from the reference or from pre-Phase-D behaviour without a documented reason; major = spec item missing or untested; minor = convention.`;
const reviews = await parallel([
  () =>
    agent(
      `${REVIEW_RULES} REVIEW REFERENCE PARITY: open the reference JudgeProperty.java, JudgeAlgorithm.java, GaugeProperty.java, GrooveGauge.java, ClearType.java, the LongNote/scratch handling and compare every table and rule implemented in crates/rbms-judge (windows.rs, algorithm.rs, gauge.rs, gauge_tables.rs, ln.rs, matcher.rs, data/*.ron) number by number and branch by branch. Report every deviation not recorded in docs/acknowledge/reference-divergences.md.`,
      {
        label: "review-parity",
        phase: "Review",
        model: "opus",
        effort: "max",
        schema: FINDINGS,
      },
    ),
  () =>
    agent(
      `${REVIEW_RULES} REVIEW ENGINE CORRECTNESS: crates/rbms-judge + crates/rbms-play. Hunt for: candidate fold selecting judged notes, off-by-one at window edges, GAS oscillation, HCN tick timing, CN deferral double judgement, BSS release condition, 24K lane mapping, lnmode override precedence, replay determinism across algorithms, panics on empty charts. Write extra tests mentally and cite lines; run cargo test --workspace.`,
      {
        label: "review-engine",
        phase: "Review",
        model: "opus",
        effort: "high",
        schema: FINDINGS,
      },
    ),
  () =>
    agent(
      `${REVIEW_RULES} REVIEW APP/UX/POLICY: apps/rbms-player, crates/rbms-config, crates/rbms-store. Check decision 12 end to end (widths > 100 -> no IR submit, no replay save, no EX/BP update, lamp and play count still updated; assist lamps), JUDGE tab rows/ranges/persistence/backward compatibility of config and scores (rule_version migration), keyconfig reverse scratch, result screen lamps, no "//" comments, forbidden name, magic numbers, clippy -D warnings, file sizes > 800 lines.`,
      {
        label: "review-app",
        phase: "Review",
        model: "opus",
        effort: "high",
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
TASK FIX (ownership: every file Phase D touched — crates/rbms-judge/**, crates/rbms-play/**, crates/rbms-model/**, crates/rbms-chart/**, crates/rbms-config/**, crates/rbms-store/**, crates/rbms-render/src/result.rs, apps/rbms-player/**): apply these verified findings, pin each with a test, refute with evidence if wrong. Findings: ${JSON.stringify(findings)}. Finish with cargo fmt --all, cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings.`,
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
TASK VERIFY (fix nothing except cargo fmt --all): exact lines for cargo fmt --all -- --check, cargo test --workspace per crate (compare baseline ${MAP.slice(0, 200)}), cargo clippy --workspace --all-targets -- -D warnings, forbidden-name grep count, new "//" lines in git diff, git status --short. Then write docs/history/2026-09-09-phase-d-judge-parity.md (Korean) and append a Phase D section to docs/acknowledge/reference-divergences.md (each remaining divergence with reference file:line) and tick the Phase D line in docs/PROCESS.md with the numbers. Inputs: D0 ${d0 ? d0.summary.slice(0, 300) : ""} || D1/D2 ${JSON.stringify(d12ok.map((r) => r.summary.slice(0, 400)))} || D3 ${d3 ? d3.summary.slice(0, 500) : ""} || D4 ${d4 ? d4.summary.slice(0, 300) : ""} || D5 ${d5 ? d5.summary.slice(0, 500) : ""} || FINDINGS ${JSON.stringify(findings.map((f) => f.severity + " " + f.file + " " + f.title))} || FIX ${fix ? fix.summary.slice(0, 400) : "none"}. OWNERSHIP for writing: those three docs only.`,
  {
    label: "verify",
    phase: "Verify",
    model: "sonnet",
    effort: "medium",
    schema: TEXT,
  },
);

return { baseline, d0, d12: d12ok, d3, d4, d5, findings, fix, verify };
