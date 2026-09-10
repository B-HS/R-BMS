export const meta = {
  name: "rbms-phase-b",
  description:
    "Phase B audio clock redesign per docs/plan/2026-09-09-phase-b-spec.md: Wave 0 stub, Wave 1 (4 branches), Wave 1.5 move, Wave 2 (2 branches), gate, adversarial review, fix, soak, docs",
  phases: [
    { title: "Baseline", detail: "measure test/clippy baseline at HEAD" },
    { title: "Wave 0", detail: "mixer.rs declarations stub (serial)" },
    {
      title: "Wave 1",
      detail:
        "audio-clock | audio-mixer | volwav | play-api (parallel, Opus max)",
    },
    { title: "Wave 1.5", detail: "settings_ui.rs pure move (serial)" },
    {
      title: "Wave 2",
      detail: "app-settings | app-clock (parallel, Opus max)",
    },
    { title: "Gate", detail: "fmt/test/clippy, fix if red" },
    {
      title: "Review",
      detail: "RT safety | clock accuracy | app integration (adversarial)",
    },
    { title: "Fix", detail: "apply confirmed findings" },
    { title: "Soak", detail: "12 minute autoplay soak with RBMS_SOAK_LOG" },
    { title: "Docs", detail: "history + divergences" },
  ],
};

const SPEC = "docs/plan/2026-09-09-phase-b-spec.md";
const RULES = `
HARD RULES (violations are rejected):
- Repo /Users/gkn/R-BMS. The spec ${SPEC} is the SSOT for Phase B: read it fully first (sections 3.0 frozen contract, 3.6 axis split, 6 ownership, 8 unverified, 9 critique). Also read docs/PROCESS.md header and docs/acknowledge/2026-09-09-enhancement-decisions.md.
- NEVER write the literal name of the reference Java player ("beat"+"oraja") or its path anywhere (code, tests, docs). Say "the reference implementation". You may READ <reference-root> for parity.
- Rust: NO "//" line comments and NO "/* */" block comments in code. Only brief English "///" doc comments on public items. No TODO/FIXME. No emojis. No magic numbers (named constants with units in the name, e.g. LOOKAHEAD_EXTRA_FRAMES, ATTACK_MS).
- Only modify files in your OWNERSHIP list. Anything else you need goes into needs_from_others in your report. Never read/write .env. Never run git state-changing commands.
- Line numbers in apps/rbms-player inside the spec are snapshots taken before Phase I landed; re-anchor by function name (build_server now lives in app_network.rs; new modules app_network.rs, app_ranking.rs, ir_*.rs exist).
- Every behaviour change needs precise unit tests. Before finishing: cargo fmt --all; cargo test for the crates you touched (and -p rbms-player if you touched the app); cargo clippy --all-targets for those crates with 0 new warnings.
- Report in Korean; code identifiers English.
`;

const REPORT = {
  type: "object",
  properties: {
    summary: { type: "string" },
    files_changed: { type: "array", items: { type: "string" } },
    public_api: {
      type: "string",
      description:
        "exact new/changed public signatures other branches depend on",
    },
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
TASK BASELINE (read-only, modify nothing): at the current HEAD run cargo test --workspace (report totals: passed/failed/ignored), cargo clippy --workspace --all-targets (count warnings, also per crate for rbms-audio, rbms-play, rbms-parser, rbms-model, rbms-chart, rbms-player), cargo fmt --all -- --check (pass/fail), git rev-parse --short HEAD. Put everything in verification.`,
  {
    label: "baseline",
    phase: "Baseline",
    model: "sonnet",
    effort: "low",
    schema: TEXT,
  },
);
log(`baseline: ${baseline ? baseline.verification.slice(0, 200) : "none"}`);

phase("Wave 0");
const w0 = await agent(
  `${RULES}
TASK Wave 0 (spec §6 "Wave 0"): OWNERSHIP crates/rbms-audio/src/mixer.rs only. Add DECLARATIONS from the frozen contract §3.0 — Bus enum (System/Key/Bg), the Command::Play extension carrying bus and the new play_on entry point the engine will call, MixStats struct and Mixer::stats(), any shared types §3.0 assigns to mixer.rs — as a minimal stub that keeps current behaviour byte-identical (bus ignored, stats() zeros). cargo build --workspace and cargo test -p rbms-audio must pass. Report public_api with exact signatures (Wave 1 branches code against it).`,
  {
    label: "wave0-stub",
    phase: "Wave 0",
    model: "opus",
    effort: "high",
    schema: REPORT,
  },
);
if (!w0) throw new Error("wave 0 failed");

phase("Wave 1");
const W1 = [
  {
    key: "B-audio-clock",
    files: "crates/rbms-audio/src/engine.rs, crates/rbms-audio/src/lib.rs",
    steps:
      "B1 interpolated clock (seqlock 5 fields, cpal OutputCallbackInfo::timestamp playback-ahead, audible_us vs scheduled_us, extrapolation cap, monotonic clamp per §3.1/§3.2), B2 MixStats publication + underrun estimate, B5 AudioOptions + open fallback ladder as a pure device-independent function + clear_namespace + IdNamespace constants (§3.3, §3.5). Use the mixer stub API exactly as declared: ",
  },
  {
    key: "B-audio-mixer",
    files: "crates/rbms-audio/src/mixer.rs",
    steps:
      "B3: fill the Wave 0 stub with the real implementation — Bus gains (System/Key/Bg defaults per §3.4), chart_gain applied after bus sum, master 1.0 with the existing soft limiter, attack/release ramps (ATTACK_MS/RELEASE_MS), voice stealing priority 3 stages, MixStats counters, StopRange; keep the amplitude-equivalence regression test (§3.4 A5 table) so final loudness equals the pre-Phase-B output for the same input. Stub API you must keep compatible: ",
  },
  {
    key: "B-volwav",
    files:
      "crates/rbms-parser/src/lib.rs, crates/rbms-model/src/lib.rs, crates/rbms-chart/src/lib.rs",
    steps:
      "B4: parse #VOLWAV into the model (boundary rules exactly per §1.3 reference table, 0 < v < 200 semantics), propagate through to_model() into ModelMeta (spec §8.1 anchor), tests for parse edge cases and propagation. API context: ",
  },
  {
    key: "B-play-api",
    files: "crates/rbms-play/src/lib.rs",
    steps:
      "B0 (§3.6): split the Player::update axis — add update_schedule(sched_us, ...) and update_judge(audible_us) with a judge_cursor, keep update(now_us) as the backward-compatible composition so existing tests pass, add PlaySource { Bgm, Key } to PlayEvent and set it at every emission point (§3.6 table). Tests: schedule ahead of judge does not move misses/auto beams; bg events tagged Bgm, autoplay key sounds tagged Key. API context: ",
  },
];
const w1 = await parallel(
  W1.map(
    (b) => () =>
      agent(
        `${RULES}
TASK Wave 1 branch ${b.key}. OWNERSHIP (exclusive, complete list): ${b.files}. Do not open crates/rbms-audio/src/decode.rs.
STEPS: ${b.steps}${w0.public_api}
Read the spec sections for these steps and implement them exactly; where the spec says "미확인", read the code/reference and decide, recording the decision in your summary. Report public_api exactly (Wave 2 depends on it).`,
        {
          label: b.key,
          phase: "Wave 1",
          model: "opus",
          effort: "max",
          schema: REPORT,
        },
      ),
  ),
);
const w1ok = w1.filter(Boolean);
log(`Wave 1: ${w1ok.length}/4 branches reported`);

const integ1 = await agent(
  `${RULES}
TASK Wave 1 integration check (modify nothing unless the build is red): run cargo build --workspace, cargo test -p rbms-audio -p rbms-play -p rbms-parser -p rbms-model -p rbms-chart, cargo clippy for those crates --all-targets. If anything is red because two branches disagree on a signature, fix the SMALLEST side to match the spec §3.0 frozen contract (you may edit crates/rbms-audio/src/{engine.rs,lib.rs,mixer.rs}, crates/rbms-play/src/lib.rs, crates/rbms-chart/src/lib.rs, crates/rbms-parser/src/lib.rs, crates/rbms-model/src/lib.rs for that purpose only) and re-run. Branch reports: ${JSON.stringify(w1ok.map((r) => ({ api: r.public_api, needs: r.needs_from_others })))}`,
  {
    label: "wave1-integrate",
    phase: "Wave 1",
    model: "opus",
    effort: "high",
    schema: TEXT,
  },
);
if (!integ1 || !integ1.ok)
  log(
    "Wave 1 integration reported problems: " +
      (integ1 ? integ1.verification.slice(0, 300) : "no report"),
  );

phase("Wave 1.5");
const w15 = await agent(
  `${RULES}
TASK Wave 1.5 (spec §6 "Wave 1.5", pure move, zero behaviour change): OWNERSHIP apps/rbms-player/src/settings_ui.rs (new), apps/rbms-player/src/app_select.rs, apps/rbms-player/src/main.rs. Move setting_line / adjust_setting (find them by name in app_select.rs) and SETTING_TABS + SETTING_* constants (find by name in main.rs) into settings_ui.rs, re-export pub(crate), add mod settings_ui. Also move any Phase I settings-row additions (NETWORK tab rows) that live in those functions so that after the move main.rs/app_select.rs contain no settings-UI code. cargo test --workspace must stay green; cargo clippy -p rbms-player --all-targets no new warnings. Report the final function/const locations.`,
  {
    label: "wave1.5-move",
    phase: "Wave 1.5",
    model: "opus",
    effort: "high",
    schema: REPORT,
  },
);

phase("Wave 2");
const apiCtx = `Wave 1 public APIs: ${w1ok.map((r) => r.public_api).join("\n---\n")}\nWave 1.5 locations: ${w15 ? w15.public_api : ""}`;
const w2 = await parallel([
  () =>
    agent(
      `${RULES}
TASK Wave 2 branch B-app-settings. OWNERSHIP (exclusive): apps/rbms-player/src/settings.rs, apps/rbms-player/src/settings_ui.rs, apps/rbms-player/src/app_input.rs. STEPS B6 (§3.5): AUDIO settings tab rows (MASTER/KEY/BGM/SYSTEM VOL, DEVICE, BUFFER, SAMPLE RATE, POLYPHONY) with ranges/steps, PlaySettings audio fields (#[serde(default)], RON round-trip + old-file compatibility tests), and the frozen interface B-app-clock calls: impl App { pub(crate) fn audio_options(&self) -> rbms_audio::AudioOptions; pub(crate) fn audio_reopen_pending(&self) -> bool; pub(crate) fn clear_audio_reopen_pending(&mut self); } (any state these need must be stored in PlaySettings or a field B-app-clock declares — coordinate via the spec §6 boundary rules; if you need an App field, name it in needs_from_others exactly). app_input.rs: replay playback audio.play -> play_on(Bus::Key, ...) with gain 1.0. ${apiCtx}`,
      {
        label: "B-app-settings",
        phase: "Wave 2",
        model: "opus",
        effort: "max",
        schema: REPORT,
      },
    ),
  () =>
    agent(
      `${RULES}
TASK Wave 2 branch B-app-clock. OWNERSHIP (exclusive): apps/rbms-player/src/main.rs, apps/rbms-player/src/app_play.rs, apps/rbms-player/src/app_select.rs, apps/rbms-player/src/timing.rs (new). STEPS B7 + B8 (§3.1-§3.3, §3.6, §4): single AudioEngine for the app lifetime (remove preview_audio, IdNamespace PLAY/PREVIEW, clear_namespace at the two engine-drop points listed in §8.1 instead of self.audio = None), audible_us vs scheduled_us split at every consumer (judgement/render/replay on audible, BGM/autoplay/preview scheduling on scheduled with LOOKAHEAD), Player::update_schedule/update_judge wiring, press path: immediate key sound via play_on(Bus::Key, .., at_us 0) + TimingSample push, Loading-cancel path keeps the engine, wall-clock fallback helper shared by play and preview (R10), App field declarations (audio_report, timing, song_us_last, plus fields B-app-settings requests: declare fields named audio_reopen_pending-style if the settings branch needs them — assume it stores state in PlaySettings unless the spec says otherwise), debug overlay lines (LOOKAHEAD / UNDERRUN~ / VOICES,STEAL,LATE,TS-FB / JUDGE stats), timing.rs TimingProbe/TimingStats + RBMS_SOAK_LOG CSV writer per §4.3 (15 columns), reopen debounce using audio_reopen_pending()/clear_audio_reopen_pending()/audio_options() from settings_ui.rs (call only; if not yet present, add a thin fallback only in your files). Tests: clock interpolation math, lookahead scheduling, namespace clearing, timing stats, CSV format. ${apiCtx}`,
      {
        label: "B-app-clock",
        phase: "Wave 2",
        model: "opus",
        effort: "max",
        schema: REPORT,
      },
    ),
]);
const w2ok = w2.filter(Boolean);

phase("Gate");
let gate = await agent(
  `${RULES}
TASK GATE (fix only what is needed to make the build green; ownership = every Phase B file listed in spec §6 plus apps/rbms-player/src/*.rs for compile fixes): cargo fmt --all; cargo build --workspace; cargo test --workspace; cargo clippy --workspace --all-targets. Compare with baseline: ${baseline ? baseline.verification : ""}. If red, fix root causes (usually a signature mismatch between B-app-settings and B-app-clock: needs lists = ${JSON.stringify(w2ok.map((r) => r.needs_from_others))}), re-run, and report exact result lines. ok=true only when fmt check passes, 0 test failures and clippy warnings <= baseline.`,
  { label: "gate", phase: "Gate", model: "opus", effort: "high", schema: TEXT },
);

phase("Review");
const REVIEW_RULES = `${RULES}
You are an adversarial reviewer: report only findings verified by reading code or running tests, with file:line and evidence. Do not modify files. critical = audio glitch/RT violation/judgement time shift/data loss; major = spec deviation or missing test; minor = convention.`;
const reviews = await parallel([
  () =>
    agent(
      `${REVIEW_RULES} REVIEW RT-SAFETY: crates/rbms-audio (engine.rs, mixer.rs, lib.rs). Check the cpal callback for allocation, locking, blocking, panics, unbounded work; seqlock correctness (torn reads, ordering); voice stealing and ramps for clicks (write a test rendering a stolen voice and asserting no discontinuity above a threshold); limiter; namespace clearing races; stream-death fallback still works. notes: list what you ran.`,
      {
        label: "review-rt",
        phase: "Review",
        model: "opus",
        effort: "high",
        schema: FINDINGS,
      },
    ),
  () =>
    agent(
      `${REVIEW_RULES} REVIEW CLOCK ACCURACY: verify the interpolated clock and the audible/scheduled split end to end (crates/rbms-audio engine.rs, crates/rbms-play lib.rs, apps/rbms-player app_play.rs/main.rs/timing.rs). Prove with tests or reasoning: judgement uses audible_us only; BGM/autoplay scheduled exactly LOOKAHEAD ahead and never late; press key sound immediate; replay timestamps on the audible axis (replay determinism: same input -> same judgement); extrapolation cap; monotonicity; behaviour when the callback stops (stream death) and on device reopen. Compare with the spec §3.1/§3.2/§3.6 tables and the reference implementation's timing model where cited.`,
      {
        label: "review-clock",
        phase: "Review",
        model: "opus",
        effort: "high",
        schema: FINDINGS,
      },
    ),
  () =>
    agent(
      `${REVIEW_RULES} REVIEW APP INTEGRATION/UX/CONVENTION: apps/rbms-player (settings.rs, settings_ui.rs, app_input.rs, main.rs, app_play.rs, app_select.rs, timing.rs) and crates/rbms-parser/model/chart #VOLWAV. Check: preview and play share one engine without leaks (namespaces cleared on every Stage transition), AUDIO tab rows/ranges/persistence/backward-compat, reopen debounce, fallback ladder, overlay lines, RON compatibility, no "//" comments, no forbidden name, no magic numbers, file sizes (list files > 800 lines), clippy warnings inside changed files, #VOLWAV boundary semantics vs the reference table in the spec.`,
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
log(
  `review: ${findings.length} findings (${findings.filter((f) => f.severity === "critical").length} critical)`,
);

phase("Fix");
const fix = findings.length
  ? await agent(
      `${RULES}
TASK FIX: apply these verified findings (ownership = every Phase B file in spec §6 + apps/rbms-player/src/*.rs). For each: fix the root cause and pin it with a test; if you prove a finding wrong, say so with evidence and skip it. Findings: ${JSON.stringify(findings)}. Finish with cargo fmt --all, cargo test --workspace, cargo clippy --workspace --all-targets (<= baseline ${baseline ? baseline.verification.slice(0, 200) : ""}).`,
      {
        label: "fix",
        phase: "Fix",
        model: "opus",
        effort: "max",
        schema: TEXT,
      },
    )
  : null;

phase("Soak");
const soak = await agent(
  `${RULES}
TASK SOAK (spec §4.3, read-only on source; you may create files only under /private/tmp/claude-501/-Users-gkn-R-BMS/847c1230-e223-4d0c-adb0-2d6113bbb669/scratchpad/soak/): build cargo build --release -p rbms-player. Find a real chart with key sounds: read the songs folder from ~/.config/rbms/settings.ron (songs_folder) or use any .bms/.bme under it; if none exists, use the repo fixture under samples/ and say so. Run the player in autoplay on that chart with RBMS_SOAK_LOG=<scratch>/soak.csv for 12 minutes (the app writes the 15-column CSV per §4.3; if the chart ends earlier, the run may loop the chart or you may restart it every time it exits, appending to the same CSV — document which). Use run_in_background and poll; sample RSS with ps every 60s into rss.csv as an independent check. At the end compute: RSS slope MB/min, underrun count, hard_steals, late count, timing residual sd. Compare to the pass criteria table in §4.3 and state pass/fail per criterion in verification. ok=true only if every criterion passes. Kill the app afterwards.`,
  { label: "soak", phase: "Soak", model: "opus", effort: "high", schema: TEXT },
);

phase("Docs");
const docs = await agent(
  `${RULES}
TASK DOCS (ownership: docs/history/2026-09-09-phase-b-audio-clock.md (new), docs/acknowledge/reference-divergences.md (append a Phase B section only)): write the Phase B history in Korean — 목적, Wave 별 구현 요약, 동결 계약 최종형(public APIs), 축 분리 결정, 볼륨 모델 재배치(master 0.5 -> bus 0.5/chart_gain/master 1.0 등가), 폴백 사다리, 검증 수치(baseline vs after), 리뷰 findings 와 수정, 소크 결과 표, 실기 가청 확인은 사용자 몫으로 명시, 미확인 사항. Inputs: BASELINE ${baseline ? baseline.verification : ""} || W0 ${w0.summary} || W1 ${JSON.stringify(w1ok.map((r) => r.summary))} || W15 ${w15 ? w15.summary : ""} || W2 ${JSON.stringify(w2ok.map((r) => r.summary))} || GATE ${gate ? gate.verification : ""} || FINDINGS ${JSON.stringify(findings.map((f) => f.severity + " " + f.file + " " + f.title))} || FIX ${fix ? fix.summary : "none"} || SOAK ${soak ? soak.verification : ""}. In reference-divergences.md add rows for: ramps, master gain relocation, lookahead, axis split, voice stealing (rbms-only design, not parity).`,
  {
    label: "docs",
    phase: "Docs",
    model: "sonnet",
    effort: "medium",
    schema: TEXT,
  },
);

return {
  baseline,
  w0,
  w1: w1ok,
  w15,
  w2: w2ok,
  gate,
  findings,
  fix,
  soak,
  docs,
};
