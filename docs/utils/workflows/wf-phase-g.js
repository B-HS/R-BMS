export const meta = {
  name: "rbms-phase-g",
  description:
    "Phase G data scale and long tail per docs/plan/2026-09-09-phase-g-spec.md: song DB, score DB, courses, practice, gamepad, system sounds, multi-IR, bmson, wiring, adversarial review, fix, docs",
  phases: [
    {
      title: "Remap",
      detail:
        "baseline + ownership table for the current tree (crate placement after Phase C)",
    },
    { title: "G0", detail: "scaffold (serial)" },
    {
      title: "G1-G9",
      detail:
        "songdb | scoredb | course | practice | gamepad | syssound | multi-IR | bmson (parallel)",
    },
    {
      title: "G8",
      detail: "wiring into stages, config, IR gate, scan (serial)",
    },
    { title: "Gate", detail: "fmt/test/clippy -D warnings + real run" },
    {
      title: "Review",
      detail:
        "data integrity & migration | reference parity | input/UX/convention",
    },
    { title: "Fix", detail: "apply confirmed findings" },
    { title: "Verify", detail: "final gate + docs" },
  ],
};

const SPEC = "docs/plan/2026-09-09-phase-g-spec.md";
const RULES = `
HARD RULES (violations are rejected):
- Repo /Users/gkn/R-BMS. The spec ${SPEC} is the SSOT for Phase G (read fully: §3 song DB DDL/API/scan, §4 score DB, §5 courses, §6 practice, §7 gamepad, §8 system sounds, §9 ownership, §10 multi-IR, §11 order/tests, §13 unverified, §14 critique). Also docs/acknowledge/2026-09-09-enhancement-decisions.md: decision 5 (rusqlite bundled), decision 7 (gilrs first, MIDI later), decision 10 (bmson IS in scope for Phase G — activate the G9 branch; the spec's own "non-goal" note is superseded by the user's decision), decision 12 (practice/autoplay excluded from records and IR).
- Phases C, D, E, F landed AFTER the spec: the app is split into apps/rbms-player/src/stage/*.rs; crates rbms-config, rbms-store (scores/replays), rbms-library (scan), rbms-skin exist; settings use a descriptor table; judge parity incl. 9 gauges is complete; an option overlay and toast bus exist. Per spec §13.8, the song DB goes INTO crates/rbms-library and the score DB INTO crates/rbms-store (no new crates for those); courses may be a new crate rbms-course. Use the OWNERSHIP TABLE from the Remap step; re-anchor by symbol names.
- NEVER write the literal name of the reference Java player ("beat"+"oraja") or its path anywhere; say "the reference implementation" and cite bare file names. You may READ <reference-root> for schemas/constants; pin numbers with tests.
- Rust: NO "//" or "/* */" comments; only brief English "///" on public items. No TODO. No emojis. No magic numbers. SQL DDL lives in .sql files or const strings without comments.
- Only modify files in your OWNERSHIP; other needs go to docs/plan/phase-g-wiring/<branch>.md as exact patch instructions (file, function, insertion point, code block) for G8, and to needs_from_others. Never read/write .env. No git state-changing commands. Never delete or rewrite the user's real ~/.config/rbms files: migrations run against fixtures and temp dirs in tests; the real-run gate uses HOME isolation.
- Every feature: precise unit tests (in-memory SQLite, tempdir scans, fixtures); cargo fmt --all; cargo test for touched crates; cargo clippy --workspace --all-targets -- -D warnings clean before finishing. Latest crate versions (check ~/.cargo/registry or cargo search) pinned in Cargo.toml.
- Keep files under ~800 lines. Report in Korean; identifiers English.
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
    ownership: { type: "string" },
    notes: { type: "string" },
  },
  required: ["baseline", "ownership", "notes"],
};

phase("Remap");
const remap = await agent(
  `${RULES}
TASK REMAP (read-only): baseline (HEAD, cargo test --workspace totals, clippy -D warnings, fmt) and the OWNERSHIP TABLE mapping spec §9 branches (G0 scaffold; G1 songdb; G2 scoredb; G3 course; G4 practice; G5 gamepad; G6 syssound; G7 multi-IR; G9 bmson; G8 wiring) onto the CURRENT tree: crates/rbms-library/** (scan, ChartDetail), crates/rbms-store/** (ScoreBook/Replay), crates/rbms-config/**, crates/rbms-parser/**, crates/rbms-ir/** (read-only for all G branches), apps/rbms-player/src/** incl. stage/*.rs and the input/key map. Decide the crate placement per §13.8 (songdb module inside rbms-library, scoredb module inside rbms-store, new crate rbms-course), list every file each branch may write (new files included), prove G1..G9 pairwise disjoint, assign all shared files (root Cargo.toml, crate Cargo.toml files, lib.rs module declarations, stage dispatch, config schema, key map) to G0 (before) or G8 (after). Record the latest versions of rusqlite (bundled), gilrs, and any bmson JSON needs (serde_json already present?).`,
  {
    label: "remap",
    phase: "Remap",
    model: "opus",
    effort: "high",
    schema: REMAP,
  },
);
const OWN = remap ? remap.ownership : "";

phase("G0");
const g0 = await agent(
  `${RULES}
OWNERSHIP TABLE: ${OWN}
TASK G0 scaffold (serial): add dependencies (rusqlite bundled, gilrs) and the module/crate skeletons the table assigns to G0 — empty modules and lib.rs declarations only, no logic; cargo build --workspace green; cargo test --workspace unchanged. Run a quick cross-platform sanity: cargo build must succeed; note any bundled-sqlite build time in the report.`,
  {
    label: "G0-scaffold",
    phase: "G0",
    model: "opus",
    effort: "high",
    schema: REPORT,
  },
);

phase("G1-G9");
const BR = [
  {
    key: "G1-songdb",
    effort: "max",
    task: "spec §3: SQLite song DB (DDL per §3.2 mirroring the reference songdata.db columns), API §3.3, incremental scan by (mtime,size) with parallel parsing and cancellation §3.4, RON/legacy migration §3.5, batch commits so partial results are queryable during a scan. Tests per §11 rows 2-3 (PRAGMA table_info column check, upsert/all_songs round trip, tempdir N charts: parsed N -> skipped N -> touch 1 -> parsed 1 -> delete 1 -> removed 1, cancel).",
  },
  {
    key: "G2-scoredb",
    effort: "max",
    task: "spec §4: SQLite score DB (DDL §4.2), API §4.3 (record_play honouring decision 12 updates_best/assist, player_stat daily accumulation, scorelog), scores.ron migration §4.4 with rollback on failure, replay GC plan (keep_recent_per_chart + best). Tests per §11 rows 4-6.",
  },
  {
    key: "G3-course",
    effort: "max",
    task: 'spec §5: course data model (reference CourseData/CourseResult mirror, 14 constraint tokens with round trip, validation rules incl. empty name -> "No Course Title", duplicate group constraints collapse, empty charts invalid, order-sensitive hash), course progress state (gauge carry-over, fail stops later stages, trophy selection at missrate/scorerate boundaries), UI flow module, IR course submission helper consuming rbms_ir public types only (ScoreServer::course_ranking Unsupported = hide panel). Tests per §11 rows 7-8.',
  },
  {
    key: "G4-practice",
    effort: "high",
    task: "spec §6: practice mode parameters (start/end clamps with the reference rounding rules, turbo/analog steps 2500/1000/100, gauge lock, PMS gaugetype clamp), state machine, decision 12 exclusion flag. Tests per §11 row 9.",
  },
  {
    key: "G5-gamepad",
    effort: "max",
    task: "spec §7: gilrs mapping model (buttons/axes to lanes, scratch two-key and analog turntable with compute_analog_diff wrap-around exactly like the reference, debounce constants, V1/V2 state transitions table in §7.2, analog_threshold default 100 clamp 1..=1000), PadConfig with #[serde(default)] in the key config schema (old files load), mouse scratch §7.3. Tests per §11 row 10 (all state transitions).",
  },
  {
    key: "G6-syssound",
    effort: "medium",
    task: "spec §8: system sound set (22 stems, folder resolution, silent when missing, played through the Phase B System bus). Tests per §11 row 11.",
  },
  {
    key: "G7-multi-ir",
    effort: "high",
    task: "spec §10: IrProfile/MultiIr with parallel submit_all isolating failures, primary_server for the ranking panel, config extension via wiring doc (ir_profiles), docs/reference/ir-multi.md. Tests per §11 row 12.",
  },
  {
    key: "G9-bmson",
    effort: "max",
    task: "decision 10: bmson parser in crates/rbms-parser (bmson.rs, serde model of the bmson spec: info, lines, bpm/stop events, sound channels with notes incl. long notes, bga, mine/key channels), mapping to the rbms chart model (judge_rank percent -> judgerank, total percent -> TOTAL, resolution, LN semantics), md5/sha256 of the file, scan extension filter instruction for G8 in the wiring doc. Tests: fixture bmson files authored by you (minimal + LN + BPM change + stop), golden note counts/timings, malformed input rejection.",
  },
];
const br = await parallel(
  BR.map(
    (b) => () =>
      agent(
        `${RULES}
OWNERSHIP TABLE: ${OWN}
G0 API: ${g0 ? g0.public_api : ""}
TASK branch ${b.key}: write ONLY the files the table assigns to ${b.key}, plus docs/plan/phase-g-wiring/${b.key}.md with exact wiring instructions for G8. ${b.task} Self-check: git diff --name-only shows only your files.`,
        {
          label: b.key,
          phase: "G1-G9",
          model: "opus",
          effort: b.effort,
          schema: REPORT,
        },
      ),
  ),
);
const brok = br.filter(Boolean);

phase("G8");
const g8 = await agent(
  `${RULES}
OWNERSHIP TABLE: ${OWN}
Branch APIs: ${brok.map((r) => r.public_api).join("\n---\n")}
TASK G8 wiring (serial; ownership = the G8 files in the table): apply every docs/plan/phase-g-wiring/*.md instruction — scan -> song DB, ScoreBook -> score DB (with first-run migration and rollback), Stage additions (Course/CourseResult/Practice) in the stage dispatch, PadState::poll in input polling, system sound triggers, ir_submission_block_reason gains practice, config fields (ir_profiles, sound_folder, guide_se, pad), bmson in the scan extension filter and chart loader, descriptor rows for new settings, key bindings without conflicts. Then the real-run gate per §11 row 13 under HOME isolation (HOME=<scratchpad>/home): build release, start with the user's songs folder read-only (copy the folder path from ~/.config/rbms/settings.ron; never write there), autoplay one chart with --auto, confirm the song DB and score DB files were created under the isolated HOME and contain the scan and the play (sqlite3 queries), kill the app. Report the queries and results.`,
  {
    label: "G8-wiring",
    phase: "G8",
    model: "opus",
    effort: "max",
    schema: REPORT,
  },
);

phase("Gate");
const gate = await agent(
  `${RULES}
TASK GATE (ownership: every file in the table; fix only for green): cargo fmt --all; cargo build --workspace; cargo test --workspace; cargo clippy --workspace --all-targets -- -D warnings; unresolved needs: ${JSON.stringify(brok.map((r) => r.needs_from_others))}. ok=true only when green.`,
  { label: "gate", phase: "Gate", model: "opus", effort: "high", schema: TEXT },
);

phase("Review");
const REVIEW_RULES = `${RULES}
Adversarial reviewer: verified findings only with file:line and evidence; do not modify files. critical = data loss/corruption (migration, GC, scan removal), wrong scores, sandbox of user files broken, render-thread stalls; major = spec item missing or untested; minor = convention.`;
const reviews = await parallel([
  () =>
    agent(
      `${REVIEW_RULES} REVIEW DATA INTEGRITY: song DB and score DB DDL vs spec, migration idempotence and rollback, WAL/backup (VACUUM INTO), replay GC never deleting a best, scan removal only for truly missing files (case-insensitive FS, symlinks), concurrent scan + UI reads, transaction boundaries, path normalisation, bmson hash identity.`,
      {
        label: "review-data",
        phase: "Review",
        model: "opus",
        effort: "high",
        schema: FINDINGS,
      },
    ),
  () =>
    agent(
      `${REVIEW_RULES} REVIEW REFERENCE PARITY: course model/constraints/trophy rules, practice clamps and steps, gamepad analog diff and V1/V2 transitions, system sound stems, song DB columns — compare with the reference sources the spec cites; report deviations not in docs/acknowledge/reference-divergences.md.`,
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
      `${REVIEW_RULES} REVIEW INPUT/UX/CONVENTION: gamepad polling on the render thread cost, debounce correctness, key conflicts with Phase F bindings, course/practice UI flows reachable from song select, decision 12 exclusions honoured, "//" comments, forbidden name, magic numbers, files > 800, clippy -D warnings, test quality.`,
      {
        label: "review-ux",
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
TASK VERIFY (fix nothing except cargo fmt --all): exact lines for fmt check, cargo test --workspace per crate vs baseline (${remap ? remap.baseline.slice(0, 200) : ""}), clippy -D warnings, forbidden-name grep, new "//" lines, git status --short. Then write docs/history/2026-09-09-phase-g-data-scale.md (Korean) incl. migration notes for users, tick Phase G in docs/PROCESS.md, append divergences. Inputs: G0 ${g0 ? g0.summary.slice(0, 300) : ""} || BRANCHES ${JSON.stringify(brok.map((r) => r.summary.slice(0, 350)))} || G8 ${g8 ? g8.summary.slice(0, 600) : ""} || GATE ${gate ? gate.verification.slice(0, 300) : ""} || FINDINGS ${JSON.stringify(findings.map((f) => f.severity + " " + f.file + " " + f.title))} || FIX ${fix ? fix.summary.slice(0, 400) : "none"}.`,
  {
    label: "verify",
    phase: "Verify",
    model: "sonnet",
    effort: "medium",
    schema: TEXT,
  },
);

return { remap, g0, branches: brok, g8, gate, findings, fix, verify };
