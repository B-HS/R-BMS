export const meta = {
  name: "rbms-phase-c",
  description:
    "Phase C structural refactor per docs/plan/2026-09-09-phase-c-spec.md: Wave 0 (6 parallel crate branches), Wave 1 serial spine (store/library/config/stage/settings descriptors), Wave 2 lint gate flip, adversarial review, fix, docs",
  phases: [
    { title: "Baseline", detail: "measure test/clippy at HEAD" },
    {
      title: "Wave 0",
      detail:
        "judge-data | table | render | core-lint | ci-workspace | play-lint (parallel)",
    },
    {
      title: "Wave 1",
      detail:
        "S1 extract -> S2 config -> S3a/b/c stage -> S4 descriptors (serial, Opus max)",
    },
    {
      title: "Wave 2",
      detail:
        "H0 app lint -> H1 gate flip (-D warnings, forbid unsafe, toolchain pin)",
    },
    {
      title: "Review",
      detail:
        "behaviour preservation | config migration | structure & lint (adversarial)",
    },
    { title: "Fix", detail: "apply confirmed findings" },
    { title: "Verify", detail: "final gate + history doc" },
  ],
};

const SPEC = "docs/plan/2026-09-09-phase-c-spec.md";
const RULES = `
HARD RULES (violations are rejected):
- Repo /Users/gkn/R-BMS. The spec ${SPEC} is the SSOT for Phase C: read it fully first (§2 Stage, §3 PlaySession, §4 crates, §5 descriptors, §6 judge data, §7 hygiene, §8 ownership, §9 step order, §12 critique). Also docs/PROCESS.md header and docs/acknowledge/2026-09-09-enhancement-decisions.md.
- NEVER write the literal name of the reference Java player ("beat"+"oraja") or its path anywhere. Say "the reference implementation". You may READ <reference-root> for parity.
- Rust: NO "//" line comments and NO "/* */" block comments. Only brief English "///" doc comments on public items. No TODO/FIXME. No emojis. No magic numbers.
- Only modify files in your OWNERSHIP list; report anything else under needs_from_others. Never read/write .env. Never run git state-changing commands (no add/commit/stash/checkout/worktree).
- The spec's line numbers for apps/rbms-player and crates/rbms-audio/rbms-play are snapshots before Phase I and Phase B landed (new modules: app_network.rs, app_ranking.rs, ir_*.rs, settings_ui.rs, timing.rs; single AudioEngine with namespaces; Player::update_schedule/update_judge). Re-anchor by symbol names; preserve every Phase I/B behaviour and test.
- This is a behaviour-preserving refactor: test counts must not decrease; replay determinism, golden renders and corpus MD5 must stay identical. Every step ends with cargo build --workspace green and the tests of the touched crates green; cargo fmt --all; cargo clippy --all-targets on touched crates with 0 new warnings.
- Report in Korean; code identifiers English.
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
TASK BASELINE (read-only): at HEAD run cargo test --workspace (totals), cargo clippy --workspace --all-targets (total warnings and per crate), cargo fmt --all -- --check, git rev-parse --short HEAD, and list apps/rbms-player/src/*.rs with line counts. Put it all in verification.`,
  {
    label: "baseline",
    phase: "Baseline",
    model: "sonnet",
    effort: "low",
    schema: TEXT,
  },
);

phase("Wave 0");
const W0 = [
  {
    key: "P1-judge-data",
    model: "opus",
    effort: "max",
    files:
      "crates/rbms-judge/** (lib.rs, windows.rs, gauge.rs, matcher.rs, new data.rs, algorithm.rs, data/judge.ron, data/gauge.ron, Cargo.toml)",
    task: "spec §6 entirely: RON data model for judge windows and gauges with program defaults equal to the reference tables (parity guard tests comparing RON-loaded tables to the existing consts byte for byte), JudgeAlgorithm as the enum the spec chose (Combo/Duration/Lowest/Score all implemented, default Duration = current behaviour, divergence noted), NoteType-based prefer(), receive clear_type_id/clear_type_from_id from the app (§4.6: ADD them here with tests; do not touch the app), crate clippy to 0.",
  },
  {
    key: "P2-table",
    model: "opus",
    effort: "medium",
    files: "crates/rbms-table/**",
    task: "spec §4.4 match_levels (equivalence test replicating the current compute_table_levels from apps/rbms-player/src/tablesrc.rs in the test), TableError via thiserror, crate clippy to 0.",
  },
  {
    key: "P3-render",
    model: "opus",
    effort: "high",
    files: "crates/rbms-render/** (src, examples, tests, Cargo.toml)",
    task: "spec §7 render items: SkinError via thiserror, thread_local theme/font globals -> argument injection (keep the public API used by the app compiling: if a signature must change, keep a compatibility wrapper and list it in public_api), too_many_arguments -> parameter structs, crate clippy to 0. Golden signatures (tests/golden.rs GOLDEN_*) must remain unchanged.",
  },
  {
    key: "P4-core-lint",
    model: "opus",
    effort: "medium",
    files: "crates/rbms-parser/**, crates/rbms-chart/**, crates/rbms-model/**",
    task: "spec §1.5/§7: clippy to 0 in the three crates (items_after_test_module etc.), Result<_, String> -> thiserror where the spec lists it, corpus MD5 invariance tests must pass unchanged.",
  },
  {
    key: "P5-ci-workspace",
    model: "opus",
    effort: "low",
    files:
      "root Cargo.toml ([workspace.dependencies] and [workspace.lints] stanzas ONLY; do not touch members), .github/workflows/ci.yml",
    task: "spec §7/§9 step 1: add [workspace.dependencies] for shared deps (keep crate Cargo.toml files untouched for now), [workspace.lints] definition, ci.yml: clippy with --all-targets, keep continue-on-error for now (the gate flips in H1). cargo metadata must succeed.",
  },
  {
    key: "P6-play-lint",
    model: "opus",
    effort: "medium",
    files: "crates/rbms-play/**",
    task: "spec §1.5.1/§8: crate clippy to 0, Player::judge pub field -> judge() accessor (keep app compiling: if the app reads the field directly, keep the field pub AND add the accessor, and list the call sites in needs_from_others). Do NOT create PlaySession (that is S3).",
  },
];
const w0 = await parallel(
  W0.map(
    (b) => () =>
      agent(
        `${RULES}
TASK Wave 0 branch ${b.key}. OWNERSHIP (exclusive): ${b.files}. TASK: ${b.task} Self-check before finishing: git diff --name-only must only list your files (plus Cargo.lock).`,
        {
          label: b.key,
          phase: "Wave 0",
          model: b.model,
          effort: b.effort,
          schema: REPORT,
        },
      ),
  ),
);
const w0ok = w0.filter(Boolean);
const integ0 = await agent(
  `${RULES}
TASK Wave 0 integration: cargo build --workspace, cargo test --workspace, cargo clippy --workspace --all-targets. If red, fix the smallest side (you may edit any crates/** file and apps/rbms-player/src/** only for call-site adaptation such as judge() accessor or SkinError). Branch reports: ${JSON.stringify(w0ok.map((r) => ({ key: r.summary.slice(0, 80), api: r.public_api, needs: r.needs_from_others })))}. ok=true only when tests are 0 failed and clippy warnings for crates/** are 0.`,
  {
    label: "wave0-integrate",
    phase: "Wave 0",
    model: "opus",
    effort: "high",
    schema: TEXT,
  },
);

phase("Wave 1");
const spine = [];
const STEPS = [
  {
    key: "S1-extract",
    files:
      "new crates/rbms-store/**, crates/rbms-library/**; apps/rbms-player/src/{main.rs, scores.rs (delete), replay.rs (delete), tablesrc.rs, format.rs, ir_map.rs (move), lib.rs (new)}, apps/rbms-player/Cargo.toml, apps/rbms-cli/{Cargo.toml, src/main.rs}, root Cargo.toml members, crates/rbms-ir/** (mapping feature only)",
    task: "spec §4.2 §4.3 §4.5 §4.6 §4.7 and §9 steps 3-6: rbms-store (ScoreBook/Replay persistence incl. rule_version and atomic writes), rbms-library (scan/ChartDetail, tablesrc uses rbms_table::match_levels), [lib] target + run() + rbms-cli subcommands scan/config/scores, delete clear_type_id from format.rs and use rbms_judge, ir_map -> rbms-ir mapping feature. Old scores.ron fixtures must load.",
  },
  {
    key: "S2-config",
    files:
      "new crates/rbms-config/**; apps/rbms-player/src/{main.rs, settings.rs (delete), folders.rs (delete), tables.rs (delete), app_input.rs, app_select.rs, settings_ui.rs, app_network.rs, ir_sync.rs} and any file that references PlayerConfig/PlaySettings",
    task: "spec §4.1 and §9 steps 7-8: rbms-config single serde schema with schema version + migration from the current settings.ron/folders.ron/tables.ron (fixture tests for old files incl. Phase I fields ir_token/ir_login_id/rivals and Phase B audio fields), then switch the app to Config in one step, deleting PlayerConfig/PlaySettings/apply_settings/current_settings. The settings sync blob sanitisation (token blanked) must keep working and its tests must pass.",
  },
  {
    key: "S3a-app-split",
    files: "apps/rbms-player/src/** (main.rs, lib.rs, new stage/mod.rs)",
    task: "spec §2.1 and §9 step 9: App -> { shared: AppShared, stage: Stage } field move only, logic identical, all tests green.",
  },
  {
    key: "S3b-play-session",
    files:
      "crates/rbms-play/**, apps/rbms-player/src/** (Play path only: app_play.rs and the audio adapter)",
    task: "spec §3 and §9 step 10: PlaySession in rbms-play with a SoundSink trait (respecting Phase B audible/scheduled axes, PlaySource bus routing, namespaces); the app implements SoundSink over AudioEngine; add a NullSink headless replay-reproduction test (same replay -> identical judgement) and keep the autoplay integration test.",
  },
  {
    key: "S3c-stages",
    files:
      "apps/rbms-player/src/** (new stage/{select,settings,keyconfig,tables,folders,loading,play,result}.rs; main.rs, lib.rs; app_play.rs/app_select.rs/app_input.rs dismantled)",
    task: "spec §2.2 §2.3 and §9 step 11: per-stage state structs + StageHandler (update/draw/handle_key/handle_mouse), frame()/window_event() reduced to dispatch, every self.stage = assignment replaced by Transition (grep must be 0), one stage at a time with the tree compiling after each. Keep the IR ranking panel, NETWORK tab, preview coordinator, debug overlay and soak logging behaviour intact. Headless render snapshot tests per stage (CpuCanvas).",
  },
  {
    key: "S4-descriptors",
    files:
      "crates/rbms-config/src/settings.rs, apps/rbms-player/src/stage/settings.rs, apps/rbms-player/src/settings_ui.rs (delete or reduce)",
    task: "spec §5 and §9 step 12: settings descriptor table (enum + label/range/step/tab meta) replacing integer indices and SETTING_TABS; setting_line/adjust_setting become descriptor-driven; label snapshot equivalence test proves every existing row (incl. NETWORK 13 rows and AUDIO rows) renders identically; §5 tests.",
  },
];
for (const s of STEPS) {
  const r = await agent(
    `${RULES}
TASK Wave 1 step ${s.key} (serial spine; previous steps: ${JSON.stringify(spine.map((x) => x.public_api).slice(-2))}). OWNERSHIP: ${s.files}. TASK: ${s.task} End condition: cargo build --workspace, cargo test --workspace (count must be >= baseline ${baseline ? baseline.verification.slice(0, 120) : ""}), cargo clippy for touched crates 0 new warnings, cargo fmt --all. Report public_api (new crate APIs / moved symbols) precisely for the next step.`,
    {
      label: s.key,
      phase: "Wave 1",
      model: "opus",
      effort: "max",
      schema: REPORT,
    },
  );
  if (!r) throw new Error(`${s.key} returned nothing`);
  spine.push(r);
  log(`${s.key} done`);
}

phase("Wave 2");
const h0 = await agent(
  `${RULES}
TASK H0 app-lint. OWNERSHIP: apps/rbms-player/src/** (remaining files) incl. keyconfig.rs -> split inline tests to keyconfig_tests.rs, gpu.rs. Goal: cargo clippy -p rbms-player --all-targets 0 warnings, test count unchanged, list files > 800 lines.`,
  {
    label: "H0-app-lint",
    phase: "Wave 2",
    model: "opus",
    effort: "high",
    schema: REPORT,
  },
);
const h1 = await agent(
  `${RULES}
TASK H1 gate flip. OWNERSHIP: every crate Cargo.toml ([lints] workspace = true), every lib.rs/main.rs top (#![forbid(unsafe_code)] where feasible, deny where the crate needs unsafe — list any), .github/workflows/ci.yml (remove continue-on-error, add -D warnings to clippy and cargo fmt --check as a gate), root rust-toolchain.toml (pin the version the workspace already requires), docs/crates.md, docs/architecture.md, docs/acknowledge/reference-divergences.md (spec §9 step 14 items 1-3), docs/PROCESS.md (tick Phase C items). Verify: cargo clippy --workspace --all-targets -- -D warnings succeeds, cargo fmt --all --check succeeds, cargo test --workspace green.`,
  {
    label: "H1-gate",
    phase: "Wave 2",
    model: "opus",
    effort: "high",
    schema: TEXT,
  },
);

phase("Review");
const REVIEW_RULES = `${RULES}
Adversarial reviewer: only verified findings with file:line and evidence; do not modify files. critical = behaviour change (judgement/score/replay/render/config data loss); major = spec deviation or missing test; minor = convention.`;
const reviews = await parallel([
  () =>
    agent(
      `${REVIEW_RULES} REVIEW BEHAVIOUR PRESERVATION: prove the refactor did not change behaviour. Run cargo test --workspace; run the golden render tests; run the corpus MD5 tests; run the replay reproduction test; compare judge/gauge RON-loaded tables to the reference tables; check that Phase I (NETWORK tab rows, ranking panel, submit outcome, sync sanitisation) and Phase B (axis split, namespaces, AUDIO tab) behaviours survived the stage split by reading the new stage modules. Try to find a lost code path (grep old function names vs new).`,
      {
        label: "review-behaviour",
        phase: "Review",
        model: "opus",
        effort: "high",
        schema: FINDINGS,
      },
    ),
  () =>
    agent(
      `${REVIEW_RULES} REVIEW CONFIG MIGRATION + STORE: crates/rbms-config, rbms-store, rbms-library. Old settings.ron/folders.ron/tables.ron/scores.ron/replay files (write fixtures from the pre-refactor formats found in git history: git show HEAD~N:apps/rbms-player/src/settings.rs for the old struct) must load with every value preserved; atomic writes; rule_version; sync blob sanitisation; migration idempotence; path handling on Windows (USERPROFILE).`,
      {
        label: "review-config",
        phase: "Review",
        model: "opus",
        effort: "high",
        schema: FINDINGS,
      },
    ),
  () =>
    agent(
      `${REVIEW_RULES} REVIEW STRUCTURE + LINT + CONVENTION: crate boundaries (no app-only logic left in crates, no crate reaching into the app), Stage/StageHandler design vs spec §2, descriptor table vs §5, file sizes (list > 800 lines), clippy -D warnings really passes, forbid(unsafe_code) coverage, "//" comments, forbidden name, magic numbers, CI workflow correctness (yaml valid, fmt/clippy gates).`,
      {
        label: "review-structure",
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
TASK FIX (ownership: all files Phase C touched — crates/** and apps/** and .github/workflows/ci.yml): apply these verified findings, pin each with a test, refute with evidence if wrong. Findings: ${JSON.stringify(findings)}. Finish with cargo fmt --all, cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings.`,
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
TASK VERIFY (fix nothing except cargo fmt --all): report exact lines for cargo fmt --all -- --check, cargo test --workspace per-crate totals (compare with baseline ${baseline ? baseline.verification.slice(0, 200) : ""}), cargo clippy --workspace --all-targets -- -D warnings, forbidden-name grep count over crates apps docs README.md, new "//" comment lines in git diff (count), file list > 800 lines, git status --short. Then write docs/history/2026-09-09-phase-c-structure.md (Korean): 목적, Wave 0 크레이트별 결과, 스파인 S1~S4 요약(신규 크레이트 API·Stage/PlaySession·descriptor), 게이트 플립(CI -D warnings·forbid unsafe·toolchain), 리뷰 findings 와 수정, 검증 수치(baseline vs after), 미확인/후속. Inputs: W0 ${JSON.stringify(w0ok.map((r) => r.summary.slice(0, 400)))} || SPINE ${JSON.stringify(spine.map((r) => r.summary.slice(0, 500)))} || H0 ${h0 ? h0.summary.slice(0, 300) : ""} || H1 ${h1 ? h1.verification.slice(0, 300) : ""} || FINDINGS ${JSON.stringify(findings.map((f) => f.severity + " " + f.file + " " + f.title))} || FIX ${fix ? fix.summary.slice(0, 400) : "none"}.`,
  {
    label: "verify",
    phase: "Verify",
    model: "sonnet",
    effort: "medium",
    schema: TEXT,
  },
);

return { baseline, w0: w0ok, integ0, spine, h0, h1, findings, fix, verify };
