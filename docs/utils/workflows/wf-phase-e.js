export const meta = {
  name: "rbms-phase-e",
  description:
    "Phase E full skin customisation per docs/plan/2026-09-09-phase-e-spec.md: scaffold, primitives | timers, properties | loader (json5 + Lua sandbox), screen port | skin UI, adversarial review, fix, docs",
  phases: [
    {
      title: "Remap",
      detail: "baseline + ownership table for the current tree",
    },
    { title: "Wave 0", detail: "rbms-skin scaffold (serial)" },
    {
      title: "Wave 1",
      detail:
        "E-prim (render primitives, PNG golden, atlas) | E-timer (timers, dst interpolator, op gating)",
    },
    {
      title: "Wave 2",
      detail:
        "E-prop (property registry generated from the reference constants) | E-load (serde model, json5, Lua sandbox, file resolution, customfile)",
    },
    {
      title: "Wave 3",
      detail:
        "E-screen (play -> select -> result -> decide -> keyconfig port) | E-ui (skin selection & customise UI)",
    },
    { title: "Gate", detail: "fmt/test/clippy -D warnings" },
    {
      title: "Review",
      detail:
        "reference semantics | render correctness & perf | security & convention",
    },
    { title: "Fix", detail: "apply confirmed findings" },
    { title: "Verify", detail: "final gate + sample skin render + docs" },
  ],
};

const SPEC = "docs/plan/2026-09-09-phase-e-spec.md";
const RULES = `
HARD RULES (violations are rejected):
- Repo /Users/gkn/R-BMS. The spec ${SPEC} is the SSOT for Phase E (read fully: §1 crate layout, §2 primitives with exact signatures and batching rule, §3 timers/dst/op gating, §4 property registry, §5 model/loader/json5/Lua/resolution/customfile/stretch, §6 screen port, §7 UI, §8 ownership, §9 order incl. the golden PNG decision, §11 unverified, §12 critique). Also docs/acknowledge/2026-09-09-enhancement-decisions.md (decision 1: JSON skin compatibility + json5 + Lua via mlua sandbox, skin-root sandbox, API whitelist).
- Phases C, D and F landed AFTER the spec: the app is split into apps/rbms-player/src/stage/*.rs, settings use a descriptor table, rbms-config/rbms-store/rbms-library exist, render has new modules (toast, option overlay, result graphs). Use the OWNERSHIP TABLE from the Remap step; re-anchor by symbol names.
- NEVER write the literal name of the reference Java player ("beat"+"oraja") or its path anywhere in the repo; say "the reference implementation" and cite bare file names (SkinObject.java:348). You may READ <reference-root> for semantics and constant tables. The generated property/timer constant tables must be produced by a checked-in generator (tools/) reading the reference file at generation time, with the generated Rust file committed and a test that re-derives a checksum — the generator must not embed the reference project name in output.
- Rust: NO "//" or "/* */" comments; only brief English "///" on public items. No TODO. No emojis. No magic numbers.
- Only modify files in your OWNERSHIP; other needs go to needs_from_others. Never read/write .env. No git state-changing commands.
- Security: Lua runs in a sandbox (no io/os/require/load, instruction/memory limits, skin-root-only file resolution, path traversal rejected); json5 parsing has size limits. Tests must prove each restriction.
- Every step: precise unit tests; golden PNG harness per spec §2.5 (decide per §9.1: commit small PNGs under crates/rbms-render/tests/golden/ if under 200 KB total, else keep the block-signature approach); cargo fmt --all; cargo test for touched crates; cargo clippy --workspace --all-targets -- -D warnings clean before finishing.
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
TASK REMAP (read-only): baseline (HEAD, cargo test --workspace totals, clippy -D warnings pass/fail, fmt check) and the OWNERSHIP TABLE mapping spec §8 waves (0 scaffold; 1 E-prim | E-timer; 2 E-prop | E-load; 3 E-screen | E-ui) onto the CURRENT tree: list crates/rbms-render/src/** and tests/**, apps/rbms-player/src/**/*.rs (esp. gpu.rs, stage/*.rs, where set_bga/clear_bga are called now, where the settings descriptor table and tabs live), root Cargo.toml members. Each file Phase E touches gets exactly one owner per wave; parallel branches within a wave are disjoint; wave 0 owns root Cargo.toml, crates/rbms-skin/src/lib.rs and every Cargo.toml edit. Also verify the json5 and mlua crate versions available (cargo search or ~/.cargo/registry) and record them.`,
  {
    label: "remap",
    phase: "Remap",
    model: "opus",
    effort: "high",
    schema: REMAP,
  },
);
const OWN = remap ? remap.ownership : "";

phase("Wave 0");
const w0 = await agent(
  `${RULES}
OWNERSHIP TABLE: ${OWN}
TASK Wave 0 E-scaffold (spec §8 wave 0): create crates/rbms-skin with the declared module tree as empty stubs, SkinError, optional features json5/lua with the versions Remap found, register the crate in the workspace and as dependencies of rbms-render and rbms-player, add the png dev-dependency, pre-record the divergences listed in the spec into docs/acknowledge/reference-divergences.md and the one-line decision into docs/acknowledge/2026-09-09-enhancement-decisions.md. cargo build --workspace and cargo test --workspace green.`,
  {
    label: "E-scaffold",
    phase: "Wave 0",
    model: "opus",
    effort: "high",
    schema: REPORT,
  },
);

phase("Wave 1");
const w1 = await parallel([
  () =>
    agent(
      `${RULES}
OWNERSHIP TABLE: ${OWN}
TASK Wave 1 E-prim (spec §2 entirely): Renderer trait extension with the exact signatures (draw_textured_quad with src uv/tint/blend, push_clip/pop_clip with logical-to-physical scaling, rotation), texture registry in gpu.rs with batching that preserves draw order per §2.2 and handles COPY_BYTES_PER_ROW_ALIGNMENT, set_bga/clear_bga kept as a thin shim over the registry (do not touch the play stage callers), CpuCanvas reference implementation incl. sampling/blend modes per the spec's corrected blend table, PNG golden harness (§2.5, decision per §9.1), glyph atlas (§2.6). Tests: primitives, blend table, clip transforms, batching order, golden renders unchanged for existing screens.`,
      {
        label: "E-prim",
        phase: "Wave 1",
        model: "opus",
        effort: "max",
        schema: REPORT,
      },
    ),
  () =>
    agent(
      `${RULES}
OWNERSHIP TABLE: ${OWN}
TASK Wave 1 E-timer (spec §3 entirely): TimerId registry with the full TIMER_* table (generated from the reference SkinProperty.java via a small generator in tools/, output committed, name-free), dst keyframe interpolator with prepareRegion semantics (acc types, loop incl. loop == -1 path, offset, timer-relative time), op/draw gating per §3.3 with the option/timer combination rules. Tests pin the reference examples in the spec (frame-by-frame values).`,
      {
        label: "E-timer",
        phase: "Wave 1",
        model: "opus",
        effort: "max",
        schema: REPORT,
      },
    ),
]);
const w1ok = w1.filter(Boolean);

phase("Wave 2");
const w2 = await parallel([
  () =>
    agent(
      `${RULES}
OWNERSHIP TABLE: ${OWN}
Wave 1 APIs: ${w1ok.map((r) => r.public_api).join("\n---\n")}
TASK Wave 2 E-prop (spec §4): property registry (Boolean/Integer/Float/String/Timer ids) generated from the reference constants by tools/gen-skin-property (committed output, checksum test), SkinStateSource trait mapping each id to the rbms state source table (§4.3) with the play ids first (100-150) then select/result/decide/keyconfig; unmapped ids return a documented default and are counted. Tests: id ranges, every mapped id has a source, generator determinism.`,
      {
        label: "E-prop",
        phase: "Wave 2",
        model: "opus",
        effort: "max",
        schema: REPORT,
      },
    ),
  () =>
    agent(
      `${RULES}
OWNERSHIP TABLE: ${OWN}
Wave 1 APIs: ${w1ok.map((r) => r.public_api).join("\n---\n")}
TASK Wave 2 E-load (spec §5): serde model mirroring the reference JsonSkin structures (§5.1), json5 tolerant parsing with size limits, Lua expression evaluation through an mlua sandbox (whitelist, no io/os/require, instruction and memory limits, deterministic), SkinLoader rules (wildcard/random file selection with seed injection, filemap, customfile, property/filepath/offset user settings persisted per §5.5), stretch/dstfilter rules (§5.6), relative flag (§5.7), path traversal rejection. Tests: load a fixture skin written under crates/rbms-skin/tests/fixtures (author it yourself in the reference JSON shape, name-free), every rule above incl. security negatives. If E-prop's SkinStateSource trait is not yet present when you need it, define your usage against the spec §4.3 signature and list it under needs_from_others.`,
      {
        label: "E-load",
        phase: "Wave 2",
        model: "opus",
        effort: "max",
        schema: REPORT,
      },
    ),
]);
const w2ok = w2.filter(Boolean);

const integ2 = await agent(
  `${RULES}
TASK Wave 2 integration (ownership: crates/rbms-skin/**): cargo build/test the skin crate and the workspace; reconcile E-prop/E-load API mismatches (needs: ${JSON.stringify(w2ok.map((r) => r.needs_from_others))}); clippy -D warnings clean. ok=true when green.`,
  {
    label: "wave2-integrate",
    phase: "Wave 2",
    model: "opus",
    effort: "high",
    schema: TEXT,
  },
);

phase("Wave 3");
const w3 = await parallel([
  () =>
    agent(
      `${RULES}
OWNERSHIP TABLE: ${OWN}
Crate APIs: ${[...w1ok, ...w2ok].map((r) => r.public_api).join("\n---\n")}
TASK Wave 3 E-screen (spec §6): skin object renderer (skin_render.rs) drawing loaded skins through the primitives with timers/properties bound to the live PlaySession and app state; port screens in order play -> select -> result -> decide -> keyconfig, keeping the existing RON SkinConfig as default parameters (M8 compatibility): when no JSON skin is selected, output must stay golden-identical; replace the play stage set_bga/clear_bga callers with the texture API and remove the shim. Tests: golden renders unchanged in default mode; fixture JSON skin renders deterministic PNG/signature; timer-driven animation frames.`,
      {
        label: "E-screen",
        phase: "Wave 3",
        model: "opus",
        effort: "max",
        schema: REPORT,
      },
    ),
  () =>
    agent(
      `${RULES}
OWNERSHIP TABLE: ${OWN}
Crate APIs: ${[...w1ok, ...w2ok].map((r) => r.public_api).join("\n---\n")}
TASK Wave 3 E-ui (spec §7): SKIN settings tab / skin selection screen listing discovered skins per screen type, custom option/file/offset editing rows from the skin's declared customisation, persisted via rbms-config (§5.5), applied live; headless CpuCanvas render test of the rows; descriptor-table integration.`,
      {
        label: "E-ui",
        phase: "Wave 3",
        model: "opus",
        effort: "max",
        schema: REPORT,
      },
    ),
]);
const w3ok = w3.filter(Boolean);

phase("Gate");
const gate = await agent(
  `${RULES}
TASK GATE (ownership: every file in the table; fix only for green): cargo fmt --all; cargo build --workspace; cargo test --workspace; cargo clippy --workspace --all-targets -- -D warnings. Resolve cross-branch needs: ${JSON.stringify(w3ok.map((r) => r.needs_from_others))}. Report exact lines; ok=true only when green and the default-mode golden signatures are unchanged.`,
  { label: "gate", phase: "Gate", model: "opus", effort: "high", schema: TEXT },
);

phase("Review");
const REVIEW_RULES = `${RULES}
Adversarial reviewer: verified findings only with file:line and evidence; do not modify files. critical = wrong skin semantics vs the reference (timers/dst/op/blend/loader rules), sandbox escape, render regression in default mode, render-thread stalls; major = spec item missing or untested; minor = convention.`;
const reviews = await parallel([
  () =>
    agent(
      `${REVIEW_RULES} REVIEW REFERENCE SEMANTICS: compare crates/rbms-skin (timer.rs, dst.rs, property/*, model.rs, loader.rs, resolve.rs, lua.rs) against the reference SkinObject.java (prepareRegion/getRate/prepare), SkinLoader/JSONSkinLoader, SkinProperty.java tables and the JsonSkin data classes, rule by rule and id by id; report deviations not recorded in docs/acknowledge/reference-divergences.md.`,
      {
        label: "review-semantics",
        phase: "Review",
        model: "opus",
        effort: "max",
        schema: FINDINGS,
      },
    ),
  () =>
    agent(
      `${REVIEW_RULES} REVIEW RENDER CORRECTNESS/PERF: crates/rbms-render + apps/rbms-player gpu.rs and skin_render: batching order preserved, clip scaling, texture upload alignment, atlas eviction, per-frame allocations in the render path, default-mode golden identity, fixture skin renders, BGA path after shim removal, frame time on a large skin (write a benchmark-ish test with many objects).`,
      {
        label: "review-render",
        phase: "Review",
        model: "opus",
        effort: "high",
        schema: FINDINGS,
      },
    ),
  () =>
    agent(
      `${REVIEW_RULES} REVIEW SECURITY/CONVENTION: Lua sandbox escape attempts (io/os/require/load/string.dump/metatables/coroutines/infinite loops/memory bombs), path traversal, json5 size bombs, symlink handling; generator output must not contain the forbidden name; "//" comments; magic numbers; file sizes > 800; clippy -D warnings; test quality.`,
      {
        label: "review-security",
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
TASK VERIFY (fix nothing except cargo fmt --all): exact lines for fmt check, cargo test --workspace per crate vs baseline (${remap ? remap.baseline.slice(0, 200) : ""}), clippy -D warnings, forbidden-name grep over crates apps tools docs (count — generated tables included), new "//" lines, git status --short. Render the fixture skin and the default skin headlessly to PNGs under /private/tmp/claude-501/-Users-gkn-R-BMS/847c1230-e223-4d0c-adb0-2d6113bbb669/scratchpad/phase-e-shots/ and list paths. Then write docs/history/2026-09-09-phase-e-skin.md (Korean) incl. a user guide section (where skins go, how to select/customise) and tick Phase E in docs/PROCESS.md; append divergences. Inputs: W0 ${w0 ? w0.summary.slice(0, 300) : ""} || W1 ${JSON.stringify(w1ok.map((r) => r.summary.slice(0, 400)))} || W2 ${JSON.stringify(w2ok.map((r) => r.summary.slice(0, 400)))} || W3 ${JSON.stringify(w3ok.map((r) => r.summary.slice(0, 400)))} || GATE ${gate ? gate.verification.slice(0, 300) : ""} || FINDINGS ${JSON.stringify(findings.map((f) => f.severity + " " + f.file + " " + f.title))} || FIX ${fix ? fix.summary.slice(0, 400) : "none"}.`,
  {
    label: "verify",
    phase: "Verify",
    model: "sonnet",
    effort: "medium",
    schema: TEXT,
  },
);

return {
  remap,
  w0,
  w1: w1ok,
  w2: w2ok,
  integ2,
  w3: w3ok,
  gate,
  findings,
  fix,
  verify,
};
