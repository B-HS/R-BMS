# Phase C — 구조 개편 (2026-09-09)

> SSOT: `docs/plan/2026-09-09-phase-c-spec.md`. 선행 조건: Phase I(`rbms-ir`, NETWORK/랭킹/리플레이/설정동기화/라이벌) 완료 후 착수. BASELINE: `76d5ed2`(`cargo test --workspace`: 1,474 통과 / 0 실패 / 3 ignored, `cargo clippy --workspace --all-targets`: 경고 있음·에러 0, `cargo fmt --all -- --check`: clean).

## 1. 목적·범위

계획 `docs/plan/2026-09-09-enhancement-plan.md` §2 Phase C(항목 1~6), 갭 계획 §1.4 C1~C11(단 C11은 명시적 범위 밖 — 아래 참조), 계획 §1.5 P9를 해소한다.

- `apps/rbms-player`(진입점 하나에 100필드 `App`, 411줄 `frame()`, 251줄 `window_event()`)를 데이터 계층(`rbms-store`/`rbms-library`/`rbms-config`)과 재생 계층(`rbms-play::PlaySession`)으로 분리하고, 화면 전환을 `self.stage = Stage::X` 산재 대입(22곳)에서 `Stage`/`StageHandler`/`Transition` 모델로 재설계한다.
- 설정 화면의 전역 정수 행 인덱스(0~23, 4상수 + 6탭 배열 + 2개의 병렬 `match`)를 이름 있는 `SettingId` + 단일 descriptor 테이블로 통합한다.
- 판정 윈도우·게이지 상수를 RON 데이터 파일로 옮기고(`JudgeAlgorithm` 4종 도입), 크레이트별 clippy 경고를 0으로 만들고 CI 게이트(`-D warnings`, `forbid(unsafe_code)`, 고정 툴체인)를 킨다.
- 계획 §1.4 C11(`rbms-render → rbms-chart` 의존 제거)은 **범위 밖**이다. `rbms-render/src/playfield.rs`가 `rbms_chart`의 스크롤 계산을 실사용하므로 의존 제거는 렌더 스크롤 모델 재설계(Phase E)의 몫이며, Phase C의 렌더 작업(P3)은 `SkinError`·전역 정리·clippy만 다룬다.
- 전 과정은 **동작 보존 리팩터**다 — 판정·리플레이 재현성·골든 렌더·저장 파일 포맷은 한 줄도 바뀌지 않아야 하며, 검증은 매 단계 `cargo test --workspace` 통과 수의 순증(감소 0)으로 고정한다.

## 2. 작업 구성 — Wave 0 (병렬 6갈래) → Wave 1 (직렬 스파인 S1~S4) → Wave 2 (H0 → H1) → 리뷰·수정

Wave 0의 6갈래는 서로 파일을 공유하지 않는 크레이트 단위 정리이고, Wave 1의 스파인은 `apps/rbms-player`를 실제로 해체하는 단일 순서(S1 데이터 추출 → S2 설정 통합 → S3 스테이지 분해 → S4 descriptor화)다.

## 3. Wave 0 — 크레이트별 결과

### P1 — judge 데이터화 (`crates/rbms-judge`)

- 신규 `src/data.rs`: `JudgeWindowsData`/`JudgePropertyData`/`JudgeTables`, `GaugeModifier`/`GaugeParams`/`GaugeSet`/`GaugeTables`를 스펙 §6.1 필드 그대로 정의(serde + `PartialEq`). `builtin_judge_tables()`/`builtin_gauge_tables()`는 `OnceLock` + `include_str!`, 로드 함수는 파일 읽기 + `version` 검증 + `JudgeDataError`(thiserror).
- `data/judge.ron`(215줄, 키는 모드 id `BEAT_5K`/`BEAT_7K`/`BEAT_10K`/`BEAT_14K`/`POPN_9K` + 미배선 `KEYBOARD_24K`), `data/gauge.ron`(67줄, `BEAT_7K` 1행 6게이지 — 코스 게이지 3종 제외).
- 패리티 가드: 모드별 대조, 레퍼런스 const 4행 전수 대조, 런타임 타입 왕복 변환, 게이지 6종 값 완전 동일성까지 단언.
- 신규 `src/algorithm.rs`: `JudgeAlgorithm { Combo, Duration(기본), Lowest, Score }` 4종 술어를 레퍼런스 그대로 전부 구현. `matcher.rs`의 `best_abs` 비교 루프를 `algorithm.prefer(...)` 호출로 치환(`Duration` 경로가 기존 로직과 동치 — 기존 판정 테스트 131개 무변경 통과로 증명). 알고리즘별 선택이 갈리는 회귀 테스트 추가.
- `clear_type_id`/`clear_type_from_id`를 `gauge.rs`에 신설(§4.6 수신분, 테스트 5개). `apps/rbms-player/src/format.rs`는 이 시점에서 미변경(S1 소관).
- 게이지 생성 경로 통합: private `Modifier`/`Spec`/`spec()` → 공개 `gauge::params(kind) -> GaugeParams`, `Gauge::from_params` 신설. 판정 윈도우는 이 시점에서는 여전히 const 표를 읽고, RON은 패리티로만 잠가둔 상태(데이터 경로로의 실전환은 리뷰 수정 단계에서 완료 — §7 참조).
- clippy 기존 4건(`manual_clamp` 1, `type_complexity` 3) 해소, 신규 유발분(`result_large_err` 4건, `SpannedError` Box화)도 해소. `cargo clippy -p rbms-judge --all-targets` 0건.
- 검증: `cargo test -p rbms-judge` 131 → 167 통과·0 실패.

### P2 — table (`crates/rbms-table`)

- `src/error.rs`에 thiserror 기반 `TableError`(7변형) 신설, 기존 `Result<_, String>` 메시지와 완전히 동일하게 유지(캐시 실패 2종은 원 에러를 `Box`로 중첩). `from_body_bytes`/`fetch_or_cache`는 `.map_err(|e| e.to_string())` 1줄 호환 shim으로 유지(소유권 밖 `tablesrc.rs` 호출부 보호).
- 신규 `parse_body`/`fetch`/`fetch_cached` 타입 API, `match_levels(impl Iterator<Item=&str>)`(§4.4, `rbms-library` 역의존 없음, 기존 `compute_table_levels`와 6케이스 동치 대조).
- clippy `unnecessary_sort_by` 1건 해소, `#![forbid(unsafe_code)]` + `[lints] workspace = true` + 워크스페이스 의존 통일.
- 검증: `cargo test -p rbms-table` 57 통과, `cargo test --workspace` 1,482 통과(순증), clippy 0건.

### P3 — render (`crates/rbms-render`)

- `SkinConfig::load`가 `Result<_, String>` → `Result<SkinConfig, SkinError>`(thiserror, `Read`/`Parse` 변형). 인자를 `impl AsRef<Path>`로 넓혀 호출부는 무변경 컴파일.
- `thread_local` theme/font 전역 → 인자 주입: `font::TextEngine`을 공개 `TextContext`로 승격, 신규 `RenderCtx { theme, text: &mut TextContext }` + `with_render_ctx`. 화면 조립 함수(`render_select_ctx` 등)와 내부 헬퍼 전부가 ctx를 받도록 전환. composer 내부에 전역 직접 호출 0건(grep 확인), 전역은 호환 래퍼 3개에만 잔존.
- `too_many_arguments` 구조체화: `PlayfieldView`, private `ScoreGraph`, private `Badge`, `examples/render_select`의 `RowBadges`.
- clippy 크레이트 고유 17건(lib 7 + lib test 8 + 예제 2) 전부 해소, `cargo clippy -p rbms-render --all-targets` 0건.
- 의도적 예외 1건(보고 대상): `render_playfield` 호환 래퍼에만 `#[allow(clippy::too_many_arguments)]`. 앱이 8인자 함수를 그대로 호출하는 동안은 래퍼를 없앨 수 없어 남겨두었으나, **이 예외는 리뷰 수정 단계에서 해소됐다**(§7 참조 — 호출부 24곳이 `PlayfieldView`로 이관되며 shim과 allow 모두 삭제).

### P4 — core-lint (`rbms-parser`/`rbms-chart`/`rbms-model`)

- clippy 경고 27건(spec 실측치 parser 15 · chart 7 · model 5과 일치) 전부 0으로 정리. 동작 변경 없는 리팩터.
- `derivable_impls`(`ParseOptions`의 수동 `Default` → derive), `collapsible_if`(let-chain), `unnecessary_get_then_check`(13곳을 `contains_key`로), `too_many_arguments`(`apply_event` 9인자 → `EventTargets<'a>` 구조체로 4인자화, 구조분해로 기존 식별자 유지), `needless_range_loop`(`enumerate`/`iter_mut().skip().take()`로 치환), `should_implement_trait`(`NoteOption`에 `FromStr` 구현).
- corpus MD5는 코퍼스·하네스가 레포에 없어 직접 재실행 불가 — 해시 경로·파싱 로직 무변경 및 인메모리 테스트 전량 통과로 대체 확인.

### P5 — ci-workspace (§7/§9 step 1)

- 루트 `Cargo.toml`에 공유 외부 의존 5종(serde, serde_json, ron, reqwest, thiserror)을 `[workspace.dependencies]`로, `[workspace.lints.rust]`/`[workspace.lints.clippy]` 스탠자를 신설. `ci.yml` clippy를 `--all-targets`로 확장.
- 크레이트별 `[lints] workspace = true`는 이 시점에서는 아직 켜지 않아(게이트 플립은 H1) 빌드/lint 동작은 무변경. `cargo build --workspace` 성공, `Cargo.lock` 무변경.

### P6 — play-lint (`crates/rbms-play`)

- `lib.rs` 단일 파일만 수정. clippy `collapsible_if` 4건(Press/Release 분기 2곳, bomb 기록 2곳)을 let-chain으로 병합해 0건.
- `Player`에 read-only 접근자 `pub fn judge(&self) -> &JudgeEngine` 신설(`pub judge` 필드는 앱이 아직 직접 읽으므로 유지, 크레이트 내부 테스트만 접근자 전환). `PlaySession`은 이 시점에서 미신설(S3 범위).
- 검증: `cargo test -p rbms-play` 58 통과(착수 전 동수), clippy 0건.

## 4. Wave 1 — 스파인 S1~S4 요약

### S1-extract — 데이터 계층 추출(§4.2·§4.3·§4.5·§4.6·§4.7, §9 step 3~6)

신규 크레이트 3종을 도입하고 `apps/rbms-player`에서 순수 데이터 코드를 이관했다.

**`rbms-store`**(deps: serde, ron, thiserror) — 점수·리플레이·원자적 저장.

```rust
pub fn write_atomic(path: &Path, contents: &str) -> std::io::Result<()>;
pub enum StoreError { Read(io::Error), Write(io::Error), Parse(SpannedError), Serialize(ron::Error) }
pub const SCORE_RULE_VERSION: u32 = 1;
pub struct ScoreRecord { /* 기존 필드·default 동일 */ }
pub struct ScoreBook { pub records: Vec<ScoreRecord>, index: HashMap<String, Vec<usize>> /* skip */ }
impl ScoreBook {
    pub fn from_records(records: Vec<ScoreRecord>) -> ScoreBook;
    pub fn load(path: &Path) -> ScoreBook;                          // 관대 로드 + .ron.bak
    pub fn try_save(&self, path: &Path) -> Result<(), StoreError>;  // 신규
    pub fn save(&self, path: &Path);                                // 기존 시그니처(stderr) 유지
    pub fn push(&mut self, record: ScoreRecord);
    pub fn for_md5 / best_ex_for_md5 / best_clear_for_md5(&self, md5: &str) -> ...;
}
pub struct Replay { /* 기존 필드 동일 */ }
impl Replay { pub fn load(path: &Path) -> Result<Replay, StoreError>; }  // Result<_, String> 대체, Display 문자열 동일
```

`ScoreBook`은 `#[serde(from = "ScoreRecords")]`로 역직렬화 시 인덱스를 자동 재구축해 직렬화 결과(`(records: [...])`)가 그대로 유지되고, md5 조회가 선형 스캔에서 해시 조회로 바뀐다(계획 §1.5 P2 해소). 구 `scores.ron`(rule_version·assisted·empty_poor·replay_file 없는 파일) 로드는 fixture + 통합 테스트로 검증.

- 비소유 파일 무수정을 위한 호환 alias 3줄(`ir_map`/`replay`/`write_atomic` 재수출)로 `ir_replay.rs`/`folders.rs`/`tables.rs`/`settings.rs`/`keyconfig.rs`/`main_tests.rs`가 무수정으로 동작.
- `rbms_library::scan_folders(folders, count, cancel)`(§4.3) 확정 시그니처를 지키되, 비소유 호출부(`app_library.rs:54`)를 위한 2인자 wrapper를 `lib.rs`에 유지.
- `run()`의 `unwrap`(`EventLoop::new()`/`run_app()`) 제거 → 사람이 읽는 메시지 + `ExitCode::FAILURE`(§7 C7, 이후 H1 단계에서 기동 경로 나머지 패닉까지 완전 제거 — §7 참조).

검증: `cargo test --workspace` 1550 통과(+24, 골든 렌더 8건·corpus MD5·리플레이/오토플레이 무변경). 신규/수정 크레이트(`rbms-store`/`rbms-library`/`rbms-cli`/`rbms-ir`) clippy 0건, 잔여 경고는 전부 `apps/rbms-player` 기존 패턴 16건(기준선 24건 → 16건, 신규 0).

### S2-config — 단일 설정 크레이트(§4.1, §9 step 7~8)

**`rbms-config`**(deps: `rbms-chart`, `rbms-judge`, `rbms-store`, serde/ron/thiserror — winit/audio/네트워크 무의존):

```rust
pub struct Config { schema_version, play: PlayOptions, judge: JudgeOptions, display: DisplayOptions,
                     audio: AudioOptions, network: NetworkOptions, library: LibraryOptions }
impl Config { pub fn sanitise(&mut self); }
```

- 그룹 구성은 설정 화면 탭 그대로. 게이지·노트옵션은 `#[serde(with)]` 어댑터로 기존 파일 어휘(`"normal"`/`"S-RANDOM"`) 유지, 런타임은 엔진 enum.
- `sanitise()`가 구 `apply_settings` + `AudioSettings::from_settings`의 클램프 전부(hispeed/lift/cover/offset/judge_rate/total/skin 대문자화/공백 폴백/오디오 8필드)를 흡수. load·migrate·다운로드 병합 3경로에서 호출.
- 마이그레이션: `schema_version` 없음 → `LegacyV0`(구 38필드 `PlaySettings`) 파싱 후 `From<LegacyV0> for Config`, 형제 `folders.ron`/`tables.ron`을 `library`로 흡수(원본은 삭제하지 않음). 상위 버전은 `ConfigError::Migrate`로 거부하고 파일 무수정. **이 시점의 마이그레이션은 원본 백업 없이 덮어썼으며, 리뷰에서 결함으로 확인돼 이후 단계에서 수정됐다**(§7).
- 앱 전환: `PlayerConfig`/`PlaySettings`/`apply_settings`/`current_settings` 전부 삭제, `settings.rs`/`folders.rs`/`tables.rs` 삭제. `App { config: Config, launch: LaunchOptions }`로 일원화, 저장은 `save_settings()` 한 곳.
- 설정 동기화 blob: `SyncPayload.settings: Config`, `keep_local`에 `library.folders`/`library.tables` 추가(예전엔 별도 파일이라 동기화 대상이 아니었음 — 그대로면 남의 폴더 경로가 올라가고 내 목록이 덮어써졌을 것). `parse_blob`을 버전 인식으로 확장해 구(스키마 없음) blob도 `LegacyV0` 경로로 마이그레이션. **이 시점의 `parse_blob`은 상위 스키마를 거부하지 않고 v1로 강등 파싱했으며, 리뷰에서 결함으로 확인돼 이후 단계에서 상한 검사가 추가됐다**(§7).
- 실사용 파일 검증(스펙 R2): 사용자의 실제 `settings.ron`(30키)+`folders.ron`으로 마이그레이션·클램프·폴더 흡수·재마이그레이션 없음을 확인(복사본, 즉시 삭제).

검증: `cargo test --workspace` 1569 통과(+19). `rbms-config` clippy 0건, `rbms-player` 15건은 전부 기존 구문(신규 0).

### S3a/b/c — 스테이지 분해와 `PlaySession`

**S3a**: `App`을 `{ shared: AppShared, stage: Stage }`로 필드 이동만 수행(로직 무변경). `Stage` enum(8변형)을 `stage/mod.rs`로 이관, `AppShared`가 구 `App`의 132필드를 이름·타입·순서·doc 그대로 보유. `self.<field>` → `self.shared.<field>` 1,158곳 기계 치환. `cargo build --workspace` 통과 = 곧 검증(치환 오류는 컴파일 에러로 드러남).

**S3b**: `crates/rbms-play/src/session.rs`에 `PlaySession` + `SoundSink`/`NullSink` 신설.

```rust
pub struct SessionClock { pub audible_us: i64, pub scheduled_us: i64 }
pub enum SoundTime { Immediate, Scheduled(i64) }
pub struct SoundRequest { pub wav: u32, pub source: PlaySource, pub gain: f32, pub pan: f32, pub pitch: f32, pub at: SoundTime }
pub trait SoundSink { fn play(&mut self, sound: SoundRequest); fn stop_all(&mut self); }
pub struct SessionOptions { autoplay, gauge, judge_offset_us, judge_rate_percent, auto_lanes, seed, analysis, auto_calibration, replay }
pub struct PlaySummary { counts, ex_score, max_ex_score, max_combo, total_notes, total_judged, fast, slow, early, late, avg_judge_us, empty_poor, gauge_value, clear_lamp, min_bp }
impl PlaySession {
    pub fn new(model: Model, options: SessionOptions) -> Self;
    pub fn tick(&mut self, clock: SessionClock, sink: &mut dyn SoundSink);
    pub fn press(&mut self, lane: usize, raw_us: i64, sink: &mut dyn SoundSink) -> Option<JudgeResult>;
    pub fn release(&mut self, lane: usize, raw_us: i64) -> Option<JudgeResult>;
    pub fn seek(&mut self, target_us: i64);
    pub fn summary(&self) -> PlaySummary;
}
```

한 판의 상태(`Player`, 재생 중 replay, 기록 입력, 오토캘리브레이션 누산기, 분석 가상 클럭, 판정오차 링, BGA 타임라인)를 전부 소유. Phase B의 2축 유지 규칙(`tick`이 `feed_replay(audible) → update_schedule(scheduled) → update_judge(audible) → bga.advance(audible)` 순서로 실행해 룩어헤드가 판정을 못 건드림)이 그대로 보존된다. 앱 어댑터 `play_sink.rs`(신규)의 `PlayAudioSink`가 `PlaySource→Bus`/`wav→sample id`/`SoundTime→song_us` 변환을 담당.

`AppShared`에서 15필드(`player`/`replay_cursor`/`recording`/`cal_sum_us`/`cal_count`/`analysis*` 5종/`msoff`/`bga_events`/`bga_cursor`/`cur_bga`/`seed`) 제거 → `play: Option<PlaySession>` 하나로 통합. `enter_result`는 `PlaySession::summary()`(순수 값) 소비로 전환.

의도적 미세 개선 1건: 오디오 장치가 없을 때 기존 코드는 press를 *기록만 하고 판정하지 않아* 리플레이가 원본과 어긋났다 — **이 시점에서는 그대로 유지**했고, 리뷰에서 지적돼 이후 단계에서 press도 판정하도록 수정됐다(§7).

**S3c**: `Stage`를 상태를 품는 enum으로 재설계, 8개 화면을 `stage/<name>.rs`로 분리.

```rust
pub(crate) enum Stage { Select(Box<SelectState>), Settings(SettingsState), KeyConfig(KeyConfigState),
                         Tables(TablesState), Folders(FoldersState), Loading(LoadingState),
                         Play(Box<PlayState>), Result(ResultState) }
pub(crate) enum StageId { Select, Settings, KeyConfig, Tables, Folders, Loading, Play, Result }
pub(crate) enum Transition { Stay, Open(Stage), To(Stage), Back, Quit }
pub(crate) struct FrameCtx<'a> { shared: &'a mut AppShared, now: Instant, dt: f32 }
pub(crate) trait StageHandler {
    fn update(&mut self, ctx: &mut FrameCtx<'_>) -> Transition;
    fn draw(&mut self, ctx: &mut FrameCtx<'_>, canvas: &mut Canvas<'_>);
    fn handle_key(&mut self, ctx: &mut FrameCtx<'_>, key: KeyInput<'_>) -> Transition;
    fn handle_mouse(&mut self, ctx: &mut FrameCtx<'_>, at: (f32, f32)) -> Transition { Transition::Stay }
    fn on_enter(&mut self, ctx: &mut FrameCtx<'_>) {}
    fn on_exit(&mut self, ctx: &mut FrameCtx<'_>) {}
}
```

`App { shared: AppShared, stage: Stage, suspended: Vec<Stage>, launch_chart: bool }`. 화면 교체 규칙(on_exit → swap → on_enter, `Open` 시 이전 화면 suspend)은 `App::switch` 한 곳에만 있다. `frame()` 411줄 → 48줄 디스패치, `window_event()` 251줄 → 45줄. `self.stage = Stage::` 산재 21곳 → 0곳(grep 검증).

`AppShared`는 132 → 65필드로 축소. 렌더 타깃은 `Canvas<'_>` enum(`Window`/`#[cfg(test)] Headless`)으로 도입해 헤드리스 스냅샷 테스트가 가능해졌다.

스펙 §2.2/§2.3 대비 의도적 이탈 5건(전부 동작 보존이 이유, 리뷰에서 재검토됨 — §7): 브라우저 목록 상태·`kc_edit_mode`/`replay`/IR 백그라운드 요청류는 `AppShared` 잔류, `StageId`(판별 전용 enum) 신설, `handle_key` 시그니처를 `KeyInput{code,pressed,released,text}`로 확장(auto-repeat/key-up 구분 필요), `Stage::Select`도 Box 처리(768바이트라 `large_enum_variant` 회피).

### S4-descriptors — 설정 descriptor 테이블(§5)

신규 `crates/rbms-config/src/settings.rs`:

```rust
pub const SETTING_COUNT: usize = 43;
pub enum SettingId { Autoplay, HiSpeed, SpeedFix, Random, Gauge, Lift, LaneCover, ScratchSide, ScratchAuto,
                      JudgeOffset, Bga, KeyConfig, JudgeWidth, Total, Skin, AutoCal, AutoReplay, DebugMode,
                      Font, ScoreGraph, ReplayAnalysis, Preview, ServerUrl, PlayerId, Account, Email, Password,
                      Login, Register, Logout, SyncSettings, UploadSettings, DownloadSettings, AutoUploadReplay,
                      Rivals, MasterVolume, KeyVolume, BgmVolume, SystemVolume, AudioDevice, AudioBuffer,
                      AudioSampleRate, AudioPolyphony }   // = 선언 순 = 구 전역 행 인덱스 0..=42
pub enum SettingTab { Play, Gauge, Judge, Display, Input, Network, Audio }
pub enum SettingKind { Toggle, IntRange{..}, FloatRange{..}, Percent{..}, Cycle{..}, Text{..}, Action, FilePick }
pub struct SettingDescriptor { id, tab, label, kind, help, host_value: bool, visible: fn(&Config) -> bool }
pub enum AdjustOutcome { Changed, Unchanged, Action(SettingId) }
pub const SETTINGS: &[SettingDescriptor];   // 43행, 탭별 연속 배치
pub fn descriptor / tab_rows / cycle_values / display_value / adjust / step_skin(...);
```

스펙 §5의 24행 목록은 Phase I·B 이전 스냅샷이라, 실제 화면(NETWORK 13행 + AUDIO 8행 포함)에 맞춰 43행으로 확장했다. `settings_ui.rs`는 526줄 → 약 300줄(절반이 테스트)로 축소되며 `SETTING_TABS`/`SETTING_KEYCONFIG` 등 정수 상수·`AppShared::adjust_setting`이 전부 삭제됐다. `stage/settings.rs`는 `SettingId` + `AdjustOutcome` 기반 라우팅으로 전면 재작성, `lines`/`dirty` 캐시로 매 프레임 Vec 재구축(계획 §1.5 P9)을 해소.

라벨 스냅샷 동치 테스트가 43행 전부의 (라벨, 값)을 이관 전 문자열과 대조. 스펙 §5 대비 의도적 이탈 5건(문서가 모르는 값을 위한 `host_value` 플래그, `Action` 반환 15행으로 확장, 43행/실제 탭 순서, `TOTAL_MAX = f64::INFINITY`, `Text.max_len` 비강제) — 전부 현행 동작 보존이 근거이며 **이 중 NETWORK 탭의 정수 인덱스 이중화는 리뷰에서 결함으로 확인돼 이후 단계에서 완전히 제거됐다**(§7).

검증: `cargo test --workspace` 1,634 통과(+25). `rbms-config` clippy 0건, `rbms-player` 8건 전부 기존(신규 0).

## 5. Wave 2 — 게이트 플립(H0 → H1)

### H0 — app-lint

`apps/rbms-player` 잔여 clippy 8건(collapsible_if 4 · collapsible_match 3 · unnecessary_sort_by 1)을 let-chain/match guard/`sort_by_key`로 해소, `cargo clippy -p rbms-player --all-targets` 0건. `keyconfig.rs`(829줄, 테스트 31개/427줄 포함)를 402줄 + 신규 `keyconfig_tests.rs`(426줄)로 분리, 테스트 본문은 4스페이스 디덴트만 하고 한 줄도 삭제하지 않음.

800줄 초과 파일(이 시점 실측): `stage/select.rs` 1109줄, `lib.rs` 1058줄(둘 다 리뷰 수정 단계에서 분할됨 — §7).

검증: `cargo clippy -p rbms-player --all-targets` 0건, `cargo clippy --workspace --all-targets` 0건(H1의 `-D warnings` 게이트 플립 가능 확인), `cargo test --workspace` 1,634 통과·0 실패.

### H1 — CI 게이트 플립

- 전 크레이트(14개) `Cargo.toml`에 `[lints] workspace = true` 적용(기존 5개는 확인만).
- `crates/*/src/lib.rs` 12개 + `apps/rbms-cli/src/main.rs` + `apps/rbms-player/src/{lib.rs,main.rs}` 전부 `#![forbid(unsafe_code)]`. `apps/rbms-player`는 `deny`가 아니라 `forbid`로도 컴파일 성공을 확인했다(`gpu.rs`의 `#[derive(bytemuck::Pod, bytemuck::Zeroable)]`가 만드는 `unsafe impl`은 `forbid`에 걸리지 않음) — `deny`로 내린 크레이트는 0개.
  - 예외 1건(소유권 밖, 처리함): `crates/rbms-audio/tests/rt_safety.rs`는 계수 global allocator(`unsafe impl GlobalAlloc`)로 믹서 콜백의 무할당을 증명하는 테스트라 `unsafe`가 필수 — 그 테스트 크레이트 루트에만 `#![allow(unsafe_code)]`.
- `ci.yml`: `continue-on-error: true` 2곳 전부 제거, clippy를 `cargo clippy --workspace --all-targets --all-features -- -D warnings`로, `dtolnay/rust-toolchain@stable` → `@master` + `toolchain: "1.95.0"`(액션이 `rust-toolchain.toml`을 읽지 않아 버전 명시가 필요 — 두 파일을 함께 올리라는 주석 추가).
- `rust-toolchain.toml`: `channel = "stable"` → `"1.95.0"`(워크스페이스 `rust-version`, 설치 rustc와 일치).
- 문서 갱신: `docs/crates.md`(신규 3크레이트 절, `rbms-judge`/`rbms-table`/`rbms-play` 신규 API 반영, `apps/rbms-player` 절 전면 재작성 — 초안에서 코드 대조로 틀린 5건 정정), `docs/architecture.md`(크레이트 12개, 의존 그래프, `PlaySession::tick`/두 클럭 축, 데이터화 표면, `Stage` 절, "Testing and the lint gate" 절 신설), `docs/acknowledge/reference-divergences.md`(Phase C 절: 후보 선택 기본값 `Duration` vs 레퍼런스 `Combo`, trait 대신 enum, 게이지 6종/코스 게이지 3종 제외), `docs/PROCESS.md`(체크리스트 갱신, 헤드 수치 실측치 교체).

검증: 소스 변경은 크레이트 루트 attribute 1줄씩 + 테스트 크레이트 allow 1줄뿐 — 로직·렌더·판정 경로 무변경, `cargo test --workspace` 1,634 통과(감소 0).

## 6. 리뷰 findings 30건과 수정 결과

Wave 1·2 완료 후 스파인 전체를 대상으로 리뷰를 수행해 **30건**(major 다수 · minor 다수)을 확인했다. 이 중 25건을 전건 처리했고(반박 0건), 2건은 스펙이 허용한 "문서화" 경로로 처리했다. 나머지 3건은 참고사항으로 대상 외 확인.

| # | 심각도 | 요지 | 위치 |
|---|---|---|---|
| 0/7/13 | major | `rbms-cli config <path>` 서브커맨드 미구현 — `rbms-config` 헤드리스 소비·스모크 통로 없음 | `apps/rbms-cli/src/main.rs` |
| 1 | major | RESULT 화면 오디오 재오픈 가드에서 `chart_loaded`가 상수 `false`로 바뀌어 차단이 풀림 | `apps/rbms-player/src/app_play.rs:309` |
| 5 | major | 설정 동기화 blob이 상위 스키마를 거부하지 않고 v1로 강등 파싱해 계정 설정을 파괴 | `apps/rbms-player/src/ir_sync.rs:102` |
| 6 | major | 마이그레이션이 구 `settings.ron`을 백업 없이 덮어써 롤백 불가 | `apps/rbms-player/src/lib.rs:965` |
| 8/16 | major | `rbms_library::Library`가 앱에서 한 번도 구성되지 않는 죽은 공개 API | `crates/rbms-library/src/lib.rs:160` |
| 9/15 | major | NETWORK 탭이 삭제 대상이었던 정수 행 인덱스 상수·배열을 유지해 descriptor와 이중화 | `apps/rbms-player/src/ir_panel.rs:7` |
| 14 | major | `judge.ron`/`gauge.ron`이 런타임에서 소비되지 않아 데이터화가 무효 | `crates/rbms-judge/src/data.rs:202` |
| 17 | major | `SelectState`/`KeyConfigState`가 §2.2 배정 상태를 소유하지 않음(`AppShared` 69필드) | `apps/rbms-player/src/lib.rs:476` |
| 18 | major | 기동 경로 패닉 잔존(C7 미완) | `apps/rbms-player/src/lib.rs:864` |
| 2 | minor | 오디오 장치 없을 때 press가 리플레이에 기록되지 않음(구 동작과 다름) | `apps/rbms-player/src/stage/play.rs:292` |
| 3 | minor | `AUDIO_PENDING_STATUS` 도달 불가능 코드화 | `apps/rbms-player/src/stage/settings.rs:95` |
| 4 | minor | IR 제출 `judge_algorithm`이 "Combo" 하드코딩 — 신규 기본값 `Duration`과 모순 | `apps/rbms-player/src/stage/play.rs:183` |
| 10/24 | minor | `rbms-config → rbms-store` 의존이 §4 의존 그래프 문서에 없음 | `crates/rbms-config/Cargo.toml` |
| 11 | minor | `ConfigError::Read`가 어디서도 생성되지 않는 죽은 변형 | `crates/rbms-config/src/error.rs:10` |
| 12 | minor | `reopen_pending`이 직렬화 제외인데 `PartialEq`에는 남음 | `crates/rbms-config/src/audio.rs:63` |
| 19 | minor | §2.3이 배제한 트레이트 오브젝트 디스패치 사용 | `apps/rbms-player/src/stage/mod.rs:167` |
| 20 | minor | 신규 파일 `//` 라인 주석 다수(주석 금지 규칙 위반) | `apps/rbms-player/src/stage/select.rs:185` |
| 21 | minor | `too_many_arguments`를 구조체화 대신 `#[allow]`로 억제 | `crates/rbms-render/src/playfield.rs:126` |
| 22 | minor | `tablesrc.rs` thiserror 이관 미완(String 에러 잔존) | `apps/rbms-player/src/tablesrc.rs:30` |
| 23 | minor | `Player::judge` pub 필드, `rbms-ir` 글롭 re-export 미정리 | `crates/rbms-play/src/lib.rs:105` |
| 25 | minor | `rbms-store`가 UI 표시 문자열 반환(데이터 크레이트에 표현 로직) | `crates/rbms-store/src/score.rs:14` |
| 26 | minor | 형제 호출부는 명명 상수를 쓰는데 두 곳만 매직 넘버 | `apps/rbms-player/src/stage/play.rs:101` |
| 27 | minor | 낡은 모듈 문서 + 크레이트 간 중복 상수 | `apps/rbms-player/src/ir_panel.rs:3` |
| 28 | minor | `rbms-ir`만 워크스페이스 dependency 통합에서 누락 | `crates/rbms-ir/Cargo.toml:9` |
| 29 | minor | 800줄 초과 파일 목록 — 신규 최대 파일이 대체 대상보다 커짐 | `apps/rbms-player/src/stage/select.rs` |

### 수정 결과

**결함 수정(동작이 실제로 틀렸던 것):**

- RESULT 재오픈 가드: `stage_owns_chart_audio(StageId) -> bool`(Play/Loading/**Result**)를 신설해 인자 `chart_loaded`를 제거 — "어느 화면이 차트 키음 뱅크를 소유하는가"를 `StageId`의 성질로 명확화.
- 동기화 blob 강등 파싱: `parse_blob`에 `schema_version > CURRENT_SCHEMA_VERSION` 상한 검사 추가 — 상위 스키마 blob은 거부.
- 마이그레이션 백업 부재: `load`가 마이그레이션 시 원본을 `settings.ron.v{from}.bak`으로 복사, `LoadOutcome.backup`에 노출.
- 오디오 장치 없을 때 press 미기록: `NullSink`로 `session.press`를 무조건 호출 — press/release 대칭 확보(리뷰서의 "판정 동작은 그대로"라는 전제는 정확하지 않았고, 이 변경으로 무장치 모드에서도 press가 판정된다는 점을 명시적으로 인정하고 반영).
- IR `judge_algorithm` "Combo" 하드코딩: `session.judge().algorithm().name()`으로 교체(기본값 `"Duration"`). 서버가 검증 없는 `varchar(16)`로 저장해 계약 영향 없음을 확인 후 반영.
- 기동 경로 패닉: `Gpu::new -> Result<Gpu, GpuError>`, `create_window` 실패 처리, `App.startup_error` → `run()`이 메시지 + `ExitCode::FAILURE`. `unwrap`/`expect` 0건.

**배선 누락 해소:**

- `rbms-cli config <path>` 신설 — `migrated_from`/보존 사본을 출력하고 `tab_rows → descriptor → display_value`로 전 설정 행 출력, `ConfigError::Migrate`는 비영 종료.
- `judge.ron`/`gauge.ron` 런타임 소비 전환 — `JudgeProperty::for_mode`/`gauge::params`가 데이터 표를 조회하고, 행이 없을 때만 내장 const로 폴백. 패리티 가드는 내장 const를 기준값으로 유지해 실질 검증 유지.
- `AppShared.songs: Vec<SongEntry>` → `library: Library`로 전환, md5 인덱스가 실제로 배선(`load_and_match`/`fetch_and_match`가 `&Library` 소비).
- NETWORK 탭 정수 인덱스 완전 제거 — `SETTING_SERVER_URL`~`SETTING_RIVALS` 13개 상수 + `NETWORK_SETTING_ROWS` 삭제, `is_text_row`/`is_secret_row`/`network_row_label`/`is_network_row`/`network_setting_line` 전부 `descriptor(id)` 기반으로 전환. `row_index()`/`from_row_index()` 의존 0건.

**위생:** `ConfigError::Read` 실사용화, `AudioOptions` 수동 `PartialEq`(`reopen_pending` 제외), 도달 불가 `AUDIO_PENDING_STATUS` 제거, `Player::judge` 비공개화, `rbms-ir` 글롭 re-export → 40개 명시 목록, `render_playfield` shim + `#[allow(too_many_arguments)]` 삭제(호출부 24곳 `PlayfieldView` 전환), `tablesrc`에 `TableSourceError` 도입, `rbms-store` UI 문자열을 앱 `format.rs`로 이동(`ScoreBook.records` 비공개 + `records()` 접근자), 매직 넘버 상수화, `ir_panel` 문서 정정.

**주석·파일 크기:** 신규 파일의 `//` 라인 주석 전건 제거(60건, 설명은 `///` doc으로 승격). `stage/select.rs`(1,109줄) → `select/{mod,preview,scene}.rs`(593/352/200), `lib.rs`(1,101줄) → `lib.rs` + `assets.rs`(961/154).

**문서화로 처리한 2건**(스펙이 허용한 경로 — `docs/acknowledge/reference-divergences.md` C-S1~C-S4 표 신설):

- C-S1(#17): 곡목록 상태·`kc_edit_mode`가 `AppShared`에 남는 이유 — 화면보다 오래 사는 값들이라 명시.
- C-S2(#19): `dyn StageHandler` 단일 match 유지 사유 — 변형 누락을 컴파일러가 잡는 성질은 그대로 보존.
- C-S3(#10/24): `rbms-config → rbms-store` 의존을 스펙 §4/§4.1에 반영.
- C-S4: `App`/`AppShared`가 크레이트 루트에 남는 이유(비공개 필드 접근 구조).

**대상 외로 확인한 것**: 리뷰서 자신이 Phase C 소유 밖이라 적은 `mixer.rs`/`engine.rs`/parser `tests.rs`/`shuffle.rs`, HEAD 이전부터 있던 테스트 내 `//` 주석은 최소 변경 원칙에 따라 미변경.

## 7. 검증 수치

| 단계 | `cargo test --workspace` | clippy 경고 |
|---|---:|---|
| baseline(`76d5ed2`) | 1,474 통과 / 0 실패 / 3 ignored | 경고 다수(정확성 린트 0) |
| S1-extract | 1,550 통과 | 신규 크레이트 0건 |
| S2-config | 1,569 통과 | `rbms-config` 0건 |
| S4-descriptors | 1,634 통과 | `rbms-config`/`rbms-player` 0건 |
| H1(게이트 플립) | 1,634 통과 | `cargo clippy --workspace --all-targets --all-features -- -D warnings` 0건 |
| 리뷰 수정 후(최종) | **1,656 통과 / 0 실패 / 3 ignored** | **0건**(exit 0) |

최종 게이트(전부 실측): `cargo fmt --all --check` 통과 · `cargo build --workspace --all-targets` 에러 0·경고 0 · `cargo test --workspace` 1,656/0/3(리뷰 착수 전 1,634 대비 +22, 감소 0) · `cargo clippy --workspace --all-targets --all-features -- -D warnings` exit 0.

회귀 보존: 골든 렌더 8건(`crates/rbms-render/tests/golden.rs`) 전부 통과 — `render_playfield` shim 제거는 호출부만 `PlayfieldView`로 옮긴 것이라 픽셀 무변화. 리플레이 결정성·오토플레이 결정성·corpus MD5·SHA256·렌더 결정성·셔플 결정성 전부 통과. `rbms-judge` 170건 통과(데이터 소비 전환 후에도 패리티 가드 유지). `rbms-ir` 253+1건, 앱 267건 통과.

리뷰 지적 각각을 고정한 신규 테스트 22건이 추가됐다(RESULT 재오픈 가드, blob 상한, v0 백업, `ConfigError::Read`, press 기록, judge_algorithm, `rbms-cli config` 3종, judge/gauge 소비 3종, Library 배선 2종, `SettingId` 라우팅 다수, `reopen_pending` PartialEq, 기동 실패 2종, tablesrc 타입 에러 3종).

헤드리스 스모크(실행 확인): `cargo run -p rbms-cli -- config <v0 settings.ron>` — v0 파일은 `migrated : schema version 0 -> 1` / `kept : settings.ron.v0.bak` 출력, 사본이 원본과 바이트 동일, `folders.ron` 원본 유지, 7탭 43행 전부 값 출력. `schema_version: 99` 파일은 `config not loaded: schema version 99 cannot be migrated`로 종료코드 1, 파일 무변경.

검증하지 않은 것: 실기 GUI 렌더 확인(창·GPU 필요)은 하지 않았다. 대신 8개 화면 전부의 헤드리스 렌더 스냅샷(`stage/render_tests.rs`)과 골든 PNG 테스트로 대체했고, 이번 변경 중 렌더 경로를 건드린 것은 `render_playfield` shim 제거(인자 묶기, 픽셀 동일)뿐이다.

## 8. 알려진 후속 (미해결 needs)

- `crates/rbms-audio/tests/rt_safety.rs`의 `#![allow(unsafe_code)]` 1줄은 소유권 밖 파일에 대한 처리라 rbms-audio 소유자의 확인이 필요하다(계수 global allocator로 믹서 콜백 무할당을 증명하는 테스트라 `unsafe`가 불가피 — 대안인 "그 크레이트만 워크스페이스 lint 상속 포기"는 정책이 갈라져 더 나쁘다고 판단했다).
- 워크스페이스에 `//` 라인 주석이 754건 남아 있다(대부분 테스트 구획 구분선). 프로젝트 Rust 주석 규칙 위반이지만 전부 Phase C 소유 밖 파일이라 손대지 않았다.
- 툴체인 버전이 `rust-toolchain.toml`과 `ci.yml` 두 곳에 있다(`dtolnay/rust-toolchain` 액션이 `rust-toolchain.toml`을 읽지 않아 불가피) — `ci.yml`에 "두 곳을 함께 올리라"는 주석을 남겼다.
- `docs/PROCESS.md`의 Phase D 항목은 여전히 "판정 알고리즘 4종"을 언급한다. Phase C가 이미 4종을 구현했으므로 Phase D의 실제 잔여는 기본값 선택(`Duration` vs `Combo`)과 JUDGE 탭 UI 노출뿐이다 — Phase D 소유라 문구 자체는 정정하지 않았다.
- C-S1~C-S4로 문서화 처리한 스펙 §2.2/§2.3/§4/§4.1 이탈은 "결함"이 아니라 "동작 보존이 근거인 설계 이탈"로 `docs/acknowledge/reference-divergences.md`에 영구 기록됐다 — 이후 세션에서 재지적 시 그 문서를 먼저 확인한다.
- `apps/rbms-cli`가 `rbms-config`는 소비하게 됐지만, 스펙 §4.7이 원래 그렸던 `[lib]` 타깃 통합의 나머지 범위(다른 서브커맨드의 크레이트 재사용 확대 등)는 이번 범위에서 다루지 않았다.
