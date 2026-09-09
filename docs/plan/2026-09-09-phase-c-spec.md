# Phase C — 구조 개편 상세 설계 (2026-09-09)

> 상위 계획: `docs/plan/2026-09-09-enhancement-plan.md` §2 Phase C(항목 1~6), 근거 갭 계획 §1.4 C1~C11 · 계획 §1.5 P9.
> 결정 근거: `docs/acknowledge/2026-09-09-enhancement-decisions.md`(결정 2 `rule_version`, 결정 3 옵션 오버레이, 결정 11 별도 런처 없음, 결정 12 커스텀 판정 정책).
> 이 문서는 **설계 명세**다. 구현 워크플로는 §8 의 파일 소유권 분할과 §7 의 순서를 그대로 따른다.
> **선행 조건: Phase I(`crates/rbms-ir`, `apps/rbms-player` NETWORK/랭킹/리플레이/설정동기화/라이벌) 완료 후 착수.** 아래에서 그 두 트리의 줄번호는 신뢰하지 말고 함수명으로 앵커하며, 해당 표에 "Phase I 이후 재확인" 을 명시했다.

---

## 1. 현행 코드 지도 (착수 시점 실측)

### 1.1 파일 · 줄 수 (2026-09-09 재실측, `dev` `72bffa5`)

> 초판은 `c6f0885` 기준이었다. 아래 줄 수·앵커·수치는 전부 `72bffa5` 에서 다시 측정했다. 착수 직전 R1 대로 한 번 더 재측정하고 이 해시를 갱신한다.

| 파일 | 줄 | 역할 | Phase C 처리 |
|---|---:|---|---|
| `apps/rbms-player/src/main.rs` | 1740 | 진입점 · `PlayerConfig` · `App`(100필드) · `Stage`/`Loading`/`SelectView`/`SelectItem`/`SortMode`/`Hot`/`KcRow` · 설정 인덱스 상수 · 라이브러리 스캔 · 키음 디코드 · `ApplicationHandler`(`resumed`/`window_event`) | **해체**(S1·S2·S3·S4) |
| `apps/rbms-player/src/app_select.rs` | 1014 | 곡선택 · 검색/정렬 · 프리뷰 · 기록 모달 · 클릭 · 표/폴더 화면 · `setting_line`/`adjust_setting` | **해체**(S3·S4) |
| `apps/rbms-player/src/app_play.rs` | 818 | `load`/`after_load`/`start_play`/`poll_keysound_load`/`song_us`/`enter_result`/`frame` | **해체**(S3) |
| `apps/rbms-player/src/keyconfig.rs` | 671 | winit `KeyCode` ↔ 토큰, `KeyConfig`/`ControlBinds` RON | 유지(앱, winit 의존) |
| `apps/rbms-player/src/format.rs` | 433 | 표시 포맷 + `clear_type_id`/`clear_type_from_id` | `clear_type_id` 계열만 `rbms-judge` 로 이관(S1) |
| `apps/rbms-player/src/scores.rs` | 423 | `ScoreRecord`/`ScoreBook`/`SCORE_RULE_VERSION` | → `rbms-store`(S1) |
| `apps/rbms-player/src/ir_map.rs` | 323 | rbms 타입 ↔ IR DTO 매핑 | → `rbms-ir`(feature `mapping`, S1). **Phase I 이후 재확인** |
| `apps/rbms-player/src/settings.rs` | 223 | `PlaySettings`(24필드 flat RON) | → `rbms-config`(S2, 삭제) |
| `apps/rbms-player/src/replay.rs` | 139 | `Replay`/`ReplayEvent` | → `rbms-store`(S1) |
| `apps/rbms-player/src/tables.rs` | 119 | `TableSource`/`TableList` RON | → `rbms-config`(S2) |
| `apps/rbms-player/src/tablesrc.rs` | 103 | `compute_table_levels`/`load_and_match` | 매칭부 → `rbms-table`(P2), 나머지 → `rbms-library`(S1) |
| `apps/rbms-player/src/folders.rs` | 98 | `FolderList` RON | → `rbms-config`(S2) |
| `apps/rbms-player/src/app_input.rs` | 404 | 목록 재구성 · 키/컨트롤 매핑 · `current_settings` · 리플레이 피드 · 키컨피그 입력 | 분할(S2·S3) |
| `apps/rbms-player/src/gpu.rs` | 354 | wgpu `Renderer` 구현 | 유지(Phase E 대상) |
| `crates/rbms-play/src/lib.rs` | 915 | `Player`(judge+matcher 래핑), `PlayEvent`, `simulate_autoplay` | **`PlaySession` 신설**(S3) |
| `crates/rbms-judge/src/{lib,windows,gauge,matcher}.rs` | 987/485/559/548 | 판정 엔진 · `JudgeProperty` const 4행 · 게이지 | **데이터화 + 알고리즘**(P1) |
| `crates/rbms-table/src/lib.rs` | 614 | 난이도표 fetch/parse | thiserror + `match_levels`(P2) |
| `crates/rbms-render/src/*` | 2,300 | 렌더 · `thread_local` theme/font | thiserror + 전역 정리(P3) |
| `apps/rbms-cli/src/main.rs` | 39 | 차트 파싱 CLI | 신규 크레이트 첫 소비자(S1) |

`main.rs` 주요 앵커: `PlayerConfig` 87–116 / `Default` 118–181 · `write_atomic` 160 부근 · `resolve_file` 389 · `keysound_jobs` 412 · `spawn_keysound_decode` 425 · `decode_bga_256` 459 · `SongEntry` 466–485 · `ChartDetail` 488–498 · `is_chart` 500 · `scan_folders` 506 · `scan_folder` 514 · `compute_chart_detail` 558 · `Stage` 587–599 · `Loading` 602–609 · `ScanOutcome` 611–616 · `KcRow` 618–623 · `SelectView` 635–642 · `SelectItem` 644–649 · `SortMode` 651–682 · `SelectKey` 684 직전 · `Hot` 684–698 · `GAUGE_CYCLE` 699 · `SETTING_KEYCONFIG/FONT/SERVER_URL/PLAYER_ID` 700–703 · `SETTING_TABS` 707–715 · `App` **717–856(100필드, 4-스페이스 최상위 `이름: 타입` 라인 실측 100)** · `App::new` 859 · `impl ApplicationHandler` 1040 · `resumed` 1041 · `window_event` **1061–1311(251줄)** · `config_dir` 1312 · `main` 1323.

### 1.2 프레임 · 이벤트 디스패치 (해체 대상)

- `App::frame()` = `app_play.rs:395–805` **411줄**. 한 함수 안에서 ① fps/RAM 계측 → ② `Stage::Loading` 폴링(scan_rx / ks_rx / finish_loading) → ③ `Stage::Select` 이면 `refresh_focused_detail` + `update_preview`, 아니면 `stop_preview` → ④ 곡 시각 계산(수동 분석 클럭 분기) → ⑤ `Stage::Play` 이면 리플레이 피드·`player.update`·결과 진입·BGA 커서 → ⑥ `Stage::Settings` 이면 `setting_line` Vec 매 프레임 재구축(계획 §1.5 P9) → ⑦ `Stage` 별 draw 8분기(`:487 Loading` `:528 Select` `:553 Play` `:616 Settings` `:644 KeyConfig` `:691 Tables` `:726 Folders` `:753 Result`) → ⑧ present.
- `App::window_event()` = `main.rs:1061–1311`. `CloseRequested`/`Resized`/`CursorMoved`/`MouseInput`/`KeyboardInput`/`RedrawRequested` 6분기이고, `KeyboardInput` 안에서 다시 `match self.stage` 10분기(`Select`+모달 가드, `Select`+검색 가드 포함).
- 상태 전이는 코드 전역에 흩어진 `self.stage = Stage::X` **22곳**(`72bffa5` 실측, `grep -rn '\.stage = Stage::' apps/rbms-player/src/`):
  `app_play.rs` 154/166/392/418(4) · `app_select.rs` 597/656/673/681/686/729/830/889/901/903/905/929(12) · `main.rs` **1135/1168/1183/1238(4 — `window_event` 내부, S3 가 다시 쓰는 바로 그 함수)** · `app_input.rs` 332/389(2).
  S3 는 이 **22곳 전부**를 `Transition` 으로 치환해야 하며, 치환 완료 조건은 위 grep 이 0건이 되는 것이다.

### 1.3 설정 이중화 · 정수 인덱스 UI

- `PlayerConfig`(`main.rs:87`) 30필드(런타임형: `GaugeKind`/`NoteOption`/`Option<Vec<(KeyCode,usize)>>`) ↔ `PlaySettings`(`settings.rs:10`) 24필드(영속형: 전부 `String`/스칼라). 변환은 `apply_settings`(`main.rs:247`, 영속→런타임)와 `App::current_settings`(`app_input.rs:107`, 런타임→영속) **수동 양방향 2곳**. 필드 1개 추가 시 수정 지점: `PlayerConfig` 선언·`Default`·`PlaySettings` 선언·`Default`·`apply_settings`·`current_settings`·`setting_line`·`adjust_setting`·`SETTING_TABS` = **9곳**.
- 설정 UI 는 전역 정수 인덱스 0~23. `setting_line(i)`(`app_select.rs:935`)와 `adjust_setting(global, d)`(`app_select.rs:966`)의 두 `match` 가 인덱스로만 묶여 있고, 탭 구성은 `SETTING_TABS`(`main.rs:707`) 의 인덱스 배열, 특수 행은 `SETTING_KEYCONFIG=11`/`SETTING_FONT=18`/`SETTING_SERVER_URL=22`/`SETTING_PLAYER_ID=23` 상수로 하드코딩. 행 하나를 중간에 끼우면 4상수 + 6탭 배열 + 2 match 가 전부 어긋난다.
- 영속 파일 5종 전부 `schema_version` 없음: `settings.ron`·`scores.ron`·`replay/*.ron`·`folders.ron`·`tables.ron`(§1.4 G-06). `scores.ron` 만 레코드 단위 `rule_version`(`scores.rs:7 SCORE_RULE_VERSION = 1`, 결정 2)을 갖는다.

### 1.4 저장 · 라이브러리 · IR 매핑 · 표 레벨

| 대상 | 현 위치 | 형태 |
|---|---|---|
| `ScoreBook`/`ScoreRecord` | `scores.rs:23,55` | RON, `push`/`for_md5`/`best_ex_for_md5`/`best_clear_for_md5`(md5 선형 스캔 → 계획 §1.5 P2) |
| `Replay`/`ReplayEvent` | `replay.rs:7,17` | RON, `load -> Result<Replay, String>` |
| `write_atomic` | `main.rs` 160 부근 | temp+rename, pid 접미사. 위 5종 저장 전부가 호출 |
| 라이브러리 스캔 | `main.rs:466,488,500,506,514,558` | `SongEntry`(17필드)·`ChartDetail`(9필드)·`is_chart`·`scan_folders`·`scan_folder`·`compute_chart_detail` |
| IR 매핑 | `ir_map.rs:12~100` | `gauge_from_name`/`ir_clear`/`ir_gauge`/`ir_random`/`ir_lntype`/`assist_flags`/`combo_breaks`/`gauge_token`. `rbms-judge`·`rbms-chart`·`rbms-ir` 3크레이트 타입을 잇는다 |
| `clear_type_id`/`from_id` | `format.rs:105,120` | `ClearType` ↔ u8(레퍼런스 값). `ScoreRecord.clear` 와 IR 이 공유 |
| 표 레벨 매칭 | `tablesrc.rs:9 compute_table_levels` | `&[SongEntry]` + `DifficultyTable` → `Vec<(String, Vec<usize>)>` |

### 1.4.1 계획 §1.4 C11 은 Phase C 범위 밖 (명시적 제외)

계획 §1.4 의 C11(`rbms-render → rbms-chart` 의존)은 **Phase C 에서 다루지 않는다.** 근거: `crates/rbms-render/src/playfield.rs:1` 이 `rbms_chart` 의 scroll 계산을 실사용하므로 의존 제거는 렌더의 스크롤 모델 재설계를 요구한다. 이는 구조 개편(C)이 아니라 렌더 재작업(Phase E)의 범위이고, 계획도 C11 을 "저우선 · 단순 제거 불가" 로 적었다. Phase C 의 어느 브랜치도 이 의존을 건드리지 않으며, P3(render)는 `SkinError`·전역 제거·clippy 만 한다. (초판은 C1~C11 을 근거로 인용하면서 C11 을 본문에서 다루지도, 제외 사유를 적지도 않았다.)

### 1.5 현행 clippy 실측 (`cargo clippy --workspace --all-targets`, 2026-09-09)

전체 재컴파일 시 진단 **121건**(타깃 중복 포함) / 카고 요약 기준 **77건**(중복 제외). 클래스는 **16종, 전부 style·complexity(deny 대상 정확성 린트 0)**.

| 클래스 | 건수 | 주 위치 | 처리 |
|---|---:|---|---|
| `clippy::collapsible_if` | 36 | 전 크레이트 | 기계적(`--fix`) |
| `clippy::needless_range_loop` | 16 | `rbms-model/mode.rs:164,191` 외 | 기계적 |
| `clippy::unnecessary_get_then_check` | 13 | 테스트 다수 | 기계적 |
| `clippy::field_reassign_with_default` | 9 | 테스트 | 기계적 |
| `clippy::unnecessary_sort_by` | 8 | 정렬부 | `sort_by_key` |
| `clippy::type_complexity` | 8 | `SelectKey`·스캔 채널·`table_levels` | **`type` 별칭 신설**(S1/S3 에서 자연 해소) |
| `clippy::too_many_arguments` | 7 | 렌더/플레이 진입점(9/7) | 파라미터 구조체화(S3·P3) |
| `clippy::manual_is_multiple_of` | 6 | `% n == 0` | 기계적 |
| `clippy::collapsible_match` | 6 | 이벤트 분기 | 기계적 |
| `wrong_self_convention` / `unnecessary_min_or_max` / `should_implement_trait` / `manual_clamp` / `derivable_impls` | 각 2 | `from_str`(→`FromStr` 구현), `Default` 파생 | 개별 판단 |
| `manual_contains` / `items_after_test_module` | 각 1 | — | 기계적 |

→ **게이트 달성 가능**: 정확성 린트가 0이므로 `-D warnings` 로 올려도 남는 부채는 위 16종 기계적 수정뿐이다.

#### 1.5.1 타깃별 잔여 건수와 **소유 브랜치**(`72bffa5` 재실측, `cargo clippy --workspace --all-targets --message-format=short` 의 "generated N warnings" 집계)

중복(`lib test` 가 `lib` 경고를 다시 세는 분)을 뺀 **고유 77건**이며, 아래 합이 정확히 77이다. **모든 행에 소유 브랜치가 있어야 §9 step 13 의 `-D warnings` 가 통과한다.**

| 크레이트 | lib | 테스트·예제 타깃 추가분 | 고유 합 | 소유 브랜치 |
|---|---:|---:|---:|---|
| `rbms-parser` | 2 | 13 (lib test 15 중 2 중복) | 15 | **P4** |
| `rbms-chart` | 4 | 3 (lib test 7 중 4 중복) | 7 | **P4** |
| `rbms-model` | 0 | 5 (lib test) | 5 | **P4** |
| `rbms-render` | 7 | 8 (lib test 15 중 7 중복) + **예제 2**(`render_frame` 1 · `render_select` 1) | 17 | **P3** |
| `rbms-judge` | 4 | 0 (전부 중복) | 4 | **P1** |
| `rbms-table` | 1 | 0 (중복) | 1 | **P2** |
| `rbms-play` | 4 | 0 (전부 중복) | 4 | **P6 play-lint**(신설, §8 Wave 0) |
| `apps/rbms-player` | bin 22 | 2 (bin test 24 중 22 중복) | 24 | **H0 app-lint**(신설, §8 Wave 2 선행) |
| 합계 | | | **77** | |

초판은 `rbms-play` 4건과 `rbms-player` 24건에 소유자를 두지 않았고 `rbms-render` 예제 2건을 누락했다. 이 28+2건이 미배정이면 H1 의 게이트 플립이 실패한다 → **P6(Wave 0, `crates/rbms-play/**`)** 와 **H0(Wave 2 선행, `apps/rbms-player/**` 잔여)** 를 신설해 배정한다(§8).
`apps/rbms-player` 의 24건을 Wave 0 에 두지 않는 이유: 그 트리는 Phase I 가 동시 수정 중이고 S1~S3 가 대부분을 다시 쓰므로, 스파인 종료 후 **잔존 코드에 대해 한 번에** 정리하는 편이 충돌·헛수고가 없다. 그때 건수는 24보다 줄어들 것이므로 H0 는 착수 시 재측정한다.

---

## 2. Stage enum 재설계 (계획 C-1)

### 2.1 App 분해

```rust
pub struct App {
    shared: AppShared,
    stage: Stage,
}
```

`AppShared` 는 스테이지 전환을 넘어 살아남는 것만 담는다(현 `App` **100필드** 중 약 40):
`config: Config`(rbms-config), `keyconfig: KeyConfig`, `paths: ConfigPaths`, `library: Library`(rbms-library), `scores: ScoreBook`(rbms-store), `tables: TableSet`, `gpu: Option<Gpu>`, `skin: Skin`, `skin_cfg: SkinConfig`, `result_palette: ResultPalette`, `audio: AudioHub`(Phase B 단일 엔진), `server: Arc<dyn ScoreServer>`(Phase I 이후 토큰 포함), `build_sha256`, `frame_stats: FrameStats`(fps/frame_count/ram_mb/last_frame), `cursor`, `hot: Vec<(Rect, Hot)>`.
스테이지 로컬 상태는 전부 각 상태 구조체로 내려간다.

```rust
pub enum Stage {
    Select(SelectState),
    Settings(SettingsState),
    KeyConfig(KeyConfigState),
    Tables(TablesState),
    Folders(FoldersState),
    Loading(LoadingState),
    Play(Box<PlayStage>),
    Result(ResultState),
}
```

`Play` 만 `Box` — `PlayStage` 가 `PlaySession`(수백 바이트 + Vec 다수)을 품어 enum 크기를 지배하기 때문(다른 변형은 100 B 미만).

### 2.2 스테이지별 상태 구조체 (현 `App` 필드에서 이동)

| 변형 | 상태 구조체 필드 | 이동 출처(`main.rs` 앵커) |
|---|---|---|
| `SelectState` | `view: SelectView`, `items: Vec<SelectItem>`, `sel`, `search: String`, `searching: bool`, `sort: SortMode`, `select_gen: u64`, `cached: Option<SelectScene>`, `cached_key: Option<SelectKey>`, `focused_detail`, `focused_detail_si`, `focus_settle_si`, `focus_settle_at`, `cover_rgba`, `record_modal: Option<usize>`, `esc_quit_at`, `preview: PreviewState`(11필드) | 735–760, 795–830 |
| `SettingsState` | `tab: SettingTab`, `sel: usize`, `text_input: Option<TextEdit>`, `lines: Vec<SettingLine>`(계획 §1.5 P9 해소용 캐시 + `dirty: bool`) | 750, 757–758 |
| `KeyConfigState` | `edit_mode: Mode`, `sel`, `capturing: bool`, `warn: bool`, `rows: Vec<KcRow>` | 771–775 |
| `TablesState` | `sel: usize`, `text_input: Option<TextEdit>` | 731 |
| `FoldersState` | `sel: usize` | 737 |
| `LoadingState` | `task: LoadingTask`, `drawn: bool`, `scan: Option<ScanJob>`, `keysound: Option<KeysoundJob>`(rx/progress/cancel/total) | 748–756 |
| `PlayStage` | `session: PlaySession`(rbms-play), `bga: BgaAssets`(`images: HashMap<i32, Vec<u8>>`), `hud_cache` | 780–786 |
| `ResultState` | `view: ResultView`, `submit: Option<SubmitStatus>`(Phase I 결과 표시) | 776 |

`PreviewState`/`ScanJob`/`KeysoundJob` 로 묶으면 `clippy::type_complexity` 3건이 자연 해소된다.

### 2.3 트레이트

```rust
pub(crate) struct FrameCtx<'a> {
    pub shared: &'a mut AppShared,
    pub now: Instant,
    pub dt: f32,
}

pub(crate) enum Transition {
    Stay,
    To(Stage),
    Quit,
}

pub(crate) trait StageHandler {
    fn update(&mut self, ctx: &mut FrameCtx<'_>) -> Transition;
    fn draw(&mut self, ctx: &mut FrameCtx<'_>, canvas: &mut dyn Renderer);
    fn handle_key(&mut self, ctx: &mut FrameCtx<'_>, key: KeyCode, pressed: bool, text: Option<&str>) -> Transition;
    fn handle_click(&mut self, ctx: &mut FrameCtx<'_>, at: (f32, f32)) -> Transition { let _ = (ctx, at); Transition::Stay }
    fn on_enter(&mut self, ctx: &mut FrameCtx<'_>) {}
    fn on_exit(&mut self, ctx: &mut FrameCtx<'_>) {}
}
```

- 디스패치는 **트레이트 오브젝트가 아니라 `impl Stage` 안의 `match`** 로 한다(변형 누락을 컴파일러가 잡고, 상태 구조체는 구체 타입 그대로 유지). 트레이트는 각 상태 구조체가 구현하고, `Stage::update` 는 `match self { Stage::Select(s) => s.update(ctx), … }` 6줄.
- 차용 규칙: `App { shared, stage }` 로 필드가 분리되어 있으므로 `self.stage.update(&mut FrameCtx { shared: &mut self.shared, .. })` 가 borrow checker 를 통과한다. **이 분리가 Stage enum 도입의 전제**다(현 `App` 처럼 한 구조체에 섞여 있으면 `&mut self.stage` 와 `&mut self.field` 가 충돌).
- `window_event` 는 winit 이벤트 → 위 4 메서드로의 번역만 남긴다(목표 40줄 이하). `frame()` 은 fps 계측 → `stage.update` → `stage.draw` → present 만 남긴다(목표 60줄 이하).
- `Transition::To(next)` 처리는 `App` 한 곳: `let prev = std::mem::replace(&mut self.stage, next); prev.on_exit(); self.stage.on_enter();` — `self.stage = Stage::X` 산재 **22곳**이 사라진다.

---

## 3. `rbms-play` 로 내려가는 것 — `PlaySession` (계획 C-1)

`enter_result`(180줄)·`frame` 의 Play 분기·`feed_replay`/`seek_replay`/`analysis_key`(`app_input.rs:224,258,296`)가 대상이다. **rbms-audio·wgpu 의존을 새로 만들지 않기 위해** 사운드/클럭은 트레이트로 주입한다.

```rust
// crates/rbms-play/src/session.rs
pub trait SoundSink {
    fn play(&mut self, wav: u32, gain: f32, pan: f32, pitch: f32, at_us: i64);
    fn stop_all(&mut self);
}
pub struct NullSink;                 // 테스트·분석 모드용

pub struct PlaySession {
    player: Player,
    replay: Option<ReplayTrack>,     // 재생용(events + cursor)
    recording: Vec<ReplayEvent>,     // 기록용
    calibration: Calibration,        // cal_sum_us / cal_count → mean_us()
    analysis: AnalysisState,         // manual/paused/rate/us
    msoff: RingBuffer<(usize, i64, u8)>,   // 무한 push 방지(현 Vec)
    bga: BgaTimeline,                // events: Vec<(i64, i32)>, cursor, current
    lntype: i32,
    started_at_us: i64,
}

impl PlaySession {
    pub fn new(model: Model, opts: SessionOptions) -> Self;
    pub fn tick(&mut self, song_us: i64, sink: &mut dyn SoundSink) -> Vec<SessionEvent>;
    pub fn press(&mut self, lane: usize, raw_us: i64, sink: &mut dyn SoundSink) -> Option<JudgeResult>;
    pub fn release(&mut self, lane: usize, raw_us: i64) -> Option<JudgeResult>;
    pub fn seek(&mut self, target_us: i64);
    pub fn is_finished(&self, song_us: i64) -> bool;
    pub fn summary(&self) -> PlaySummary;      // enter_result 의 계산부 전부
    pub fn judge(&self) -> &JudgeEngine;
    pub fn bga_frame(&self, song_us: i64) -> i32;
}

pub struct SessionOptions {
    pub autoplay: bool,
    pub random: NoteOption,
    pub seed: u64,
    pub gauge: GaugeKind,
    pub judge_offset_us: i64,        // 판정에만 적용(키음은 raw, Phase A A3)
    pub judge_window_rates: JudgeRates,
    pub auto_lanes: Vec<bool>,
    pub replay: Option<Replay>,
    pub analysis: bool,
}
```

- `PlaySummary` 는 `enter_result` 가 계산하던 것(EX/판정 6종/최대콤보/게이지 추이/램프/`assist`/`score: bool`(결정 12)/BP/타이밍 분포)을 **순수 값**으로 반환한다. IR 제출·`ScoreBook.push`·리플레이 저장은 앱(`ResultState::on_enter`)에 남는다 — 네트워크·파일 I/O 를 `rbms-play` 로 내리지 않는다.
- `rbms-play` 의 새 의존: `rbms-store`(`Replay`/`ReplayEvent` 타입 공유). 순환 없음(`rbms-store` 는 serde/ron 만 의존).
- 테스트: `PlaySession` 은 `NullSink` + 가상 시각으로 헤드리스 구동 가능 → 리플레이 재현 회귀 테스트를 GUI 없이 돌린다(현재는 `apps/rbms-player/tests/autoplay_preview.rs` 39줄이 유일).

---

## 4. 신규 크레이트

의존 방향(추가분만):

```
rbms-config  → rbms-model, rbms-chart(NoteOption), rbms-judge(GaugeKind), rbms-store(write_atomic 재사용), serde, ron
rbms-store   → serde, ron            (숫자 id 로만 판정/램프 표현 — rbms-judge 비의존)
rbms-library → rbms-model, rbms-parser, rbms-chart, serde, ron
rbms-play    → (기존) + rbms-store
rbms-table   → (기존)                 (rbms-library 비의존 — §4.4)
rbms-ir      → (기존) + [feature "mapping"] rbms-judge, rbms-chart
```

**`rbms-config` 는 winit 을 의존하지 않는다.** 키 바인딩은 문자열 토큰으로만 저장하고, `KeyCode` 변환은 지금처럼 `apps/rbms-player/src/keyconfig.rs` 가 담당한다(현 `KeyConfig` RON 도 토큰 문자열이라 스키마 변화 없음).

### 4.1 `crates/rbms-config`

단일 serde 스키마 + 스키마 버전 + 마이그레이션. `PlayerConfig`/`PlaySettings` 이중화를 **런타임 타입 하나로 통일**한다(문자열 저장은 유지하되 `serde` 어댑터로 흡수 → `apply_settings`/`current_settings` 소멸).

```rust
pub const CURRENT_SCHEMA_VERSION: u32 = 1;   // v0 = schema_version 필드가 없는 기존 파일

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub schema_version: u32,
    pub play: PlayOptions,       // hispeed, constant_speed, random, gauge, lift, cover,
                                 // scratch_left, scratch_auto, total_override, autoplay, auto_replay
    pub judge: JudgeOptions,     // offset_ms, auto_offset, judge_rate, (D 확장: algorithm,
                                 // key_rates[3], scratch_rates[3], ln_margin_rate, lnmode, gas, bottom_shiftable, target)
    pub display: DisplayOptions, // bga, skin(SkinRef), font_path, score_graph, replay_analysis, debug
    pub audio: AudioOptions,     // (B 확장: device, buffer_frames, sample_rate, volumes 3종, polyphony)
    pub network: NetworkOptions, // server_url, player_id, (I: token 은 별도 파일 — §4.1 주석)
    pub library: LibraryOptions, // folders: Vec<String>, songs_folder, preview, tables: Vec<TableSource>
}

#[derive(Debug)]
pub enum ConfigError { Read(std::io::Error), Parse(ron::error::SpannedError), Migrate { from: u32, reason: String } }

pub struct LoadOutcome { pub config: Config, pub migrated_from: Option<u32>, pub backup: Option<PathBuf> }

pub fn load(path: &Path) -> Result<LoadOutcome, ConfigError>;
pub fn save(config: &Config, path: &Path) -> Result<(), ConfigError>;
pub fn migrate(raw: &str) -> Result<(Config, Option<u32>), ConfigError>;
pub use rbms_store::write_atomic;   // main.rs 에서 rbms-store 로 이관, rbms-config 는 재노출만
```

마이그레이션 규칙:
- `schema_version` 이 없거나 `0` → `LegacyV0`(현 `PlaySettings` 24필드 + `folders.ron`/`tables.ron` 병합) 으로 파싱 후 `From<LegacyV0> for Config`.
- 알 수 없는 상위 버전 → `ConfigError::Migrate` 를 반환하고 **앱은 파일을 덮어쓰지 않고** 기본값으로 기동(다운그레이드 시 사용자 설정 파괴 방지). 현행 `.bak` 백업 동작은 유지.
- `folders.ron`/`tables.ron` 은 v0 로 남기고, 마이그레이션이 `Config.library` 로 흡수한 뒤 **원본 파일은 삭제하지 않고 남긴다**(롤백 가능). `scores.ron`·`replay/*.ron` 은 `rbms-store` 소관이라 이 마이그레이션 범위 밖(레코드 단위 `rule_version` 유지).
- **`settings.ron` 자신도 롤백 가능해야 한다**: 마이그레이션한 원본을 `settings.ron.v{from}.bak` 으로 먼저 복사한 뒤에만 현 스키마로 덮어쓰고, `LoadOutcome.backup` 이 그 사본을 가리킨다. 구 `PlaySettings` 는 `#[serde(default)]` 플랫 구조라 v1 문서를 읽으면 전 필드가 조용히 기본값이 되므로, 사본이 없으면 이전 릴리스로 되돌리는 순간 설정이 전멸한다.
- 읽기 자체가 실패한 경우(권한·디렉터리 등 `NotFound` 아닌 I/O 오류)는 "파일 없음" 과 구분해 `ConfigError::Read` 로 반환하고 **파일을 건드리지 않는다**. 기본값으로 덮어쓰면 읽지 못했을 뿐인 설정을 잃는다.
- 같은 상한 검사는 **계정 동기화 blob** 에도 적용한다(`apps/rbms-player/src/ir_sync.rs::parse_blob`): `schema_version` 이 현 스키마보다 높은 blob 을 현 스키마로 강등 파싱하면 신설·이동 필드가 전부 기본값으로 떨어지고, 그 강등본이 로컬에 영속화된 뒤 다음 업로드에서 서버 blob 을 덮어쓴다.

테스트(구파일 필수):
1. `tests/fixtures/settings-v0-default.ron` — 현행 `PlaySettings::default()` 를 그대로 직렬화한 파일. 로드 결과가 `Config::default()` 와 동치.
2. `settings-v0-full.ron` — 24필드 전부 비기본값. 전 필드가 새 스키마 대응 필드로 옮겨졌는지 1:1 검증.
3. `settings-v0-partial.ron` — 일부 필드만. 나머지가 기본값.
4. `settings-v0-unknown-key.ron` — 미지 키 포함 시 무시하고 로드 성공.
5. `folders-v0.ron` + `tables-v0.ron` 이 `Config.library` 로 병합.
6. `settings-v99.ron` — 상위 버전은 `Migrate` 에러 + 파일 무변경(저장 호출 안 함) 확인.
7. round-trip: `save`→`load` 가 `Config` 동치, 그리고 `schema_version == CURRENT_SCHEMA_VERSION`.

### 4.2 `crates/rbms-store`

```rust
pub const SCORE_RULE_VERSION: u32 = 1;            // scores.rs:7 에서 이관

#[derive(Clone, Serialize, Deserialize)] pub struct ScoreRecord { /* scores.rs:23 그대로 */ }
#[derive(Default, Serialize, Deserialize)] pub struct ScoreBook { records: Vec<ScoreRecord>, index: HashMap<String, Vec<usize>> /* #[serde(skip)] */ }

impl ScoreBook {
    pub fn load(path: &Path) -> ScoreBook;                  // 현행 관대 로드 유지
    pub fn save(&self, path: &Path) -> Result<(), StoreError>;
    pub fn push(&mut self, record: ScoreRecord);
    pub fn for_md5(&self, md5: &str) -> Vec<&ScoreRecord>;
    pub fn best_ex_for_md5(&self, md5: &str) -> Option<u32>;
    pub fn best_clear_for_md5(&self, md5: &str) -> Option<u8>;
    pub fn rebuild_index(&mut self);                        // load 후 1회
}

#[derive(Clone, Serialize, Deserialize)] pub struct ReplayEvent { /* replay.rs:7 */ }
#[derive(Clone, Serialize, Deserialize)] pub struct Replay { /* replay.rs:17 */ }
impl Replay { pub fn load(path: &Path) -> Result<Replay, StoreError>; pub fn save(&self, path: &Path) -> Result<(), StoreError>; }

#[derive(Debug, thiserror::Error)] pub enum StoreError { … }
```

- `index: HashMap<md5_lowercase, Vec<usize>>` 를 `#[serde(skip)]` 으로 두고 `push`/`rebuild_index` 가 갱신 → §1.5 **P2 의 O(N×M) 선형 스캔이 O(N)** 으로 떨어진다(Phase C 에서 얻는 성능 부수효과. 곡선택 캐시 재구축 자체의 개선은 Phase F/G).
- 파일 포맷·필드명은 **무변경**(기존 `scores.ron` 그대로 읽혀야 함). 회귀 테스트: 현행 `scores.rs` 의 인라인 테스트 30여 개를 그대로 이식 + "구 `scores.ron` 샘플 로드 → `for_md5` 결과가 이관 전과 동일" 픽스처 1개.

### 4.3 `crates/rbms-library`

```rust
#[derive(Clone, Debug)] pub struct SongEntry { /* main.rs:466 17필드 그대로 */ }
#[derive(Clone, Debug)] pub struct ChartDetail { /* main.rs:488 9필드 그대로 */ }

pub fn is_chart(path: &Path) -> bool;
pub fn scan_folder(root: &Path, progress: &AtomicUsize, cancel: &AtomicBool) -> Vec<SongEntry>;
pub fn scan_folders(folders: &[String], progress: &AtomicUsize, cancel: &AtomicBool) -> Vec<SongEntry>;
pub fn compute_chart_detail(path: &Path, mode: Mode) -> Option<ChartDetail>;

pub struct Library { songs: Vec<SongEntry>, by_md5: HashMap<String, Vec<usize>> }
impl Library {
    pub fn from_songs(songs: Vec<SongEntry>) -> Self;
    pub fn songs(&self) -> &[SongEntry];
    pub fn indices_for_md5(&self, md5: &str) -> &[usize];
    pub fn md5s(&self) -> impl Iterator<Item = &str>;
}
```

`cancel: &AtomicBool` 파라미터는 신규(현행 스캔은 취소 불가). Phase C 에서는 배선만 하고 항상 `false` 를 넘겨도 되지만, Loading 화면 Esc 취소(계획 §1.5 P6 과 동종)를 위해 시그니처를 지금 확정한다.

### 4.4 `rbms-table` 매칭 API (`compute_table_levels` 이관)

`rbms-table` 이 `rbms-library` 를 의존하면 역방향 결합이 생기므로, **`SongEntry` 를 받지 않고 md5 이터레이터를 받는다**:

```rust
impl DifficultyTable {
    /// 로컬 라이브러리 md5 목록과 표를 대조해 레벨별 라이브러리 인덱스를 만든다.
    /// `library_md5s` 는 라이브러리 순서대로의 md5(대소문자 무관).
    pub fn match_levels<'a>(&self, library_md5s: impl Iterator<Item = &'a str>) -> Vec<(String, Vec<usize>)>;
}
```

호출부: `table.match_levels(library.md5s())`. 동작·정렬·dedup 은 현 `tablesrc.rs:9` 와 동일해야 하며, 이관 전후 결과 동치 테스트를 둔다.

### 4.5 `ir_map` → `rbms-ir` (feature `mapping`)

`rbms-ir` 은 현재 rbms 크레이트 무의존이다(순수 DTO+HTTP). 매핑을 옮기면 `rbms-judge`·`rbms-chart` 의존이 생기므로 **선택 feature** 로 격리한다.

```toml
# crates/rbms-ir/Cargo.toml
[features]
default = []
mapping = ["dep:rbms-judge", "dep:rbms-chart", "dep:rbms-model"]
```

`rbms-ir::mapping` 모듈에 `gauge_from_name`/`ir_clear`/`ir_gauge`/`ir_random`/`ir_lntype`/`assist_flags`/`combo_breaks`/`gauge_token` 을 옮기고 인라인 테스트 20여 개를 함께 이동. `apps/rbms-player` 는 `rbms-ir = { workspace = true, features = ["mapping"] }`.
**Phase I 가 `rbms-ir` 을 대폭 변경하므로 이 이관은 Phase I 머지 후 함수명 기준으로 재확인한다.**

### 4.6 `clear_type_id` → `rbms-judge`

`format.rs:105,120` 의 `clear_type_id(ClearType) -> u8` / `clear_type_from_id(u8) -> ClearType` 을 `rbms-judge::gauge` 로 이동(`ClearType` 이 정의된 파일 = 의존 방향상 하위). 인라인 테스트 5개(레퍼런스 값 대조·단조성·legacy LightAssist 매핑)도 함께. `format.rs` 에는 표시용 `clear_label_color` 만 남는다.

**작업 분담(중복 방지 — 두 에이전트가 각자 "이관" 을 수행하면 wave 1 에서 중복 정의·중복 테스트가 된다):**
- **P1(Wave 0)** = `crates/rbms-judge/src/gauge.rs` 에 두 함수와 테스트 5개를 **추가만** 한다. 이 시점에 `format.rs` 의 원본과 공존하지만 크레이트가 달라 컴파일 충돌은 없다.
- **S1-d(Wave 1)** = `apps/rbms-player/src/format.rs` 에서 원본 두 함수와 테스트 5개를 **삭제**하고 호출부를 `rbms_judge` 경로로 바꾼다. `crates/rbms-judge/**` 는 절대 건드리지 않는다.

### 4.7 `[lib]` 타깃과 `rbms-cli`

```toml
# apps/rbms-player/Cargo.toml
[lib]
name = "rbms_player"
path = "src/lib.rs"

[[bin]]
name = "rbms-player"
path = "src/main.rs"
```

`src/lib.rs` 가 모듈 루트 + `pub fn run(args: impl Iterator<Item = String>) -> ExitCode`, `src/main.rs` 는 `fn main() -> ExitCode { rbms_player::run(std::env::args()) }` 3줄. 이로써 `apps/rbms-player/tests/*.rs` 가 앱 내부 타입에 접근 가능해진다(현재 `autoplay_preview.rs` 39줄이 유일한 통합 테스트인 이유).

**`rbms-cli` 는 `rbms-player` 를 의존하지 않는다** — 의존하면 wgpu/winit/cpal 이 CLI 빌드에 끌려온다. 계획 §2 C-5 의 "rbms-cli 를 첫 소비자로" 는 **신규 크레이트(`rbms-config`/`rbms-library`/`rbms-store`)의 첫 소비자**로 해석해 다음 서브커맨드를 추가한다(헤드리스 검증 통로 확보):

| 서브커맨드 | 소비 크레이트 | 검증 대상 |
|---|---|---|
| `rbms-cli scan <dir>` | rbms-library | 스캔 결과 곡 수·md5·모드 |
| `rbms-cli config <path>` | rbms-config | 마이그레이션 결과 + `migrated_from` + 보존된 원본 사본, 이어서 탭별 전 설정 행(`tab_rows` → `descriptor` → `display_value`) |
| `rbms-cli scores <path> --md5 <md5>` | rbms-store | 기록 조회·베스트 |

---

## 5. 설정 descriptor 테이블 (계획 C-3)

`rbms-config::settings` 에 둔다(Phase D 판정 항목·Phase F 표시 항목이 여기에 행만 추가하면 되도록).

```rust
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum SettingId {
    Autoplay, HiSpeed, SpeedFix, Random, Gauge, Lift, LaneCover, ScratchSide, ScratchAuto,
    JudgeOffset, Bga, KeyConfig, JudgeWidth, Total, Skin, AutoCal, AutoReplay, DebugMode,
    Font, ScoreGraph, ReplayAnalysis, Preview, ServerUrl, PlayerId,
    // Phase D 확장 예정: JudgeAlgorithm, JudgeWidthKey/Scratch(PG/GR/GD), LnMarginRate, LnMode, Gas, BottomShiftable, Target
    // Phase B 확장 예정: AudioDevice, AudioBuffer, VolumeSystem/Key/Bg, Polyphony
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SettingTab { Play, Gauge, Judge, Display, Input, Audio, Network }

pub enum SettingKind {
    Toggle,
    IntRange { min: i32, max: i32, step: i32, unit: &'static str },
    FloatRange { min: f64, max: f64, step: f64, decimals: u8, unit: &'static str },
    Percent { min: f32, max: f32, step: f32 },
    Cycle { values: &'static [&'static str] },
    Text { max_len: usize, secret: bool },
    Action,     // "KEY CONFIG >"
    FilePick,   // FONT / SKIN
}

pub struct SettingDescriptor {
    pub id: SettingId,
    pub tab: SettingTab,
    pub label: &'static str,
    pub kind: SettingKind,
    pub help: &'static str,
    /// 조건부 노출(예: 커스텀 판정폭 항목은 JudgeAlgorithm 이 특정 값일 때만)
    pub visible: fn(&Config) -> bool,
}

pub const SETTINGS: &[SettingDescriptor] = &[ /* 24행(현행) → D·F 에서 확장 */ ];

pub fn descriptor(id: SettingId) -> &'static SettingDescriptor;
pub fn tab_rows(tab: SettingTab, cfg: &Config) -> Vec<SettingId>;   // SETTING_TABS 인덱스 배열 대체
pub fn display_value(cfg: &Config, id: SettingId) -> String;         // setting_line 의 값부
pub fn adjust(cfg: &mut Config, id: SettingId, delta: i32) -> AdjustOutcome;
pub enum AdjustOutcome { Changed, Unchanged, Action(SettingId) }     // KeyConfig/Font/Skin 처럼 앱이 처리할 것
```

- `SETTING_KEYCONFIG=11` 등 4개 정수 상수와 `SETTING_TABS` 인덱스 배열은 **삭제**된다. 앱의 설정 화면은 `tab_rows` → `display_value` 만 호출하고, `AdjustOutcome::Action(SettingId::Font)` 같은 신호로 파일 선택 다이얼로그를 띄운다.
- `SettingsState.lines` 캐시 + `dirty` 플래그로 §1.5 **P9(매 프레임 Vec 재구축)** 를 해소한다.
- 테스트: ① `SETTINGS` 에 모든 `SettingId` 가 정확히 1회 등장(누락/중복 방지) ② 모든 탭이 비지 않음 ③ 모든 `IntRange`/`FloatRange`/`Percent` 항목에 대해 `adjust` 를 ±1000 회 반복해도 값이 선언 범위를 벗어나지 않음 ④ `display_value` 가 모든 id 에 대해 빈 문자열이 아님 ⑤ 이관 전후 24행의 라벨·표시 문자열이 현행 `setting_line` 과 동일(문자열 스냅샷).

---

## 6. 판정 · 게이지 데이터화 + `JudgeAlgorithm` (계획 C-4)

### 6.1 RON 데이터 모델

```rust
// crates/rbms-judge/src/data.rs
#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct JudgeWindowsData { pub pg: (i64, i64), pub gr: (i64, i64), pub gd: (i64, i64), pub bd: (i64, i64), pub ms: Option<(i64, i64)> }

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct JudgePropertyData {
    pub note: JudgeWindowsData,
    pub scratch: JudgeWindowsData,
    pub ln_end: JudgeWindowsData,
    pub ln_scratch_end: JudgeWindowsData,
    pub longnote_margin_us: i64,
    pub longscratch_margin_us: i64,
    pub combo: [bool; 6],
    pub judge_vanish: [bool; 6],
    pub miss_condition: MissCondition,
}

#[derive(Serialize, Deserialize)]
pub struct JudgeTables { pub version: u32, pub properties: BTreeMap<String, JudgePropertyData> }   // key = "BEAT_7K" 등 모드 id

/// 레퍼런스 `GrooveGauge.GaugeModifier` 미러. 현행 `gauge.rs:27 enum Modifier` 와 1:1.
#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Debug)]
pub enum GaugeModifier { Total, LimitIncrement, None }

/// 레퍼런스 `GaugeProperty.GaugeElementProperty` 1행 미러.
/// 생성자 인자 순서 `(modifier, min, max, init, border, float[6] deltas, float[][] guts)` 를 그대로 필드화했고,
/// 현행 `crates/rbms-judge/src/gauge.rs:33 struct Spec`(private) 과 필드가 정확히 일치한다.
/// `guts` 는 `(임계값, 감쇠배율)` 쌍의 오름차순 목록(현행 `HARD_GUTS` = 5쌍).
#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct GaugeParams {
    pub modifier: GaugeModifier,
    pub min: f32,
    pub max: f32,
    pub init: f32,
    pub border: f32,
    pub deltas: [f32; 6],          // [PG, GR, GD, BD, PR, MS] — Judge 인덱스 순
    pub guts: Vec<(f32, f32)>,     // 현행은 &'static [(f32, f32)]
}

/// 한 모드의 게이지 6종. 레퍼런스 `GaugeProperty` 는 모드당 9종(+ CLASS/EXCLASS/EXHARDCLASS)이지만
/// rbms 에는 코스(단위인정) 게이지가 없으므로 **Phase C 는 6종만** 담고, 나머지 3종은 Phase D 에서 행을 추가한다.
#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct GaugeSet {
    pub assist_easy: GaugeParams,
    pub easy: GaugeParams,
    pub normal: GaugeParams,
    pub hard: GaugeParams,
    pub exhard: GaugeParams,
    pub hazard: GaugeParams,
}

#[derive(Serialize, Deserialize)]
pub struct GaugeTables { pub version: u32, pub gauges: BTreeMap<String, GaugeSet> }   // key = "BEAT_7K" 등 모드 id

pub fn builtin_judge_tables() -> &'static JudgeTables;   // OnceLock + include_str!("../data/judge.ron")
pub fn builtin_gauge_tables() -> &'static GaugeTables;   // include_str!("../data/gauge.ron")
pub fn load_judge_tables(path: &Path) -> Result<JudgeTables, JudgeDataError>;   // 사용자 오버라이드(Phase D/E)
```

- **패리티 가드 테스트(필수)**: `builtin_judge_tables()` 를 파싱한 결과가 현행 `windows.rs` 의 const `JudgeProperty` 4행과 필드 단위로 완전히 동일함을 단언한다. Phase A 에서 레퍼런스 값으로 맞춰 놓은 표가 데이터화 과정에서 흔들리지 않게 하는 유일한 방어선이다(`JudgeProperty.java` 대조는 Phase D 에서 5K/PMS/24K 행을 추가할 때 다시 수행).
- **`gauge.ron` 의 Phase C 내용**: 키 `"BEAT_7K"` 한 행뿐이며, 값은 현행 `gauge.rs:45-51 fn spec()` 의 6개 `Spec` 리터럴을 그대로 옮긴 것이다(레퍼런스 `GaugeProperty.SEVENKEYS` 와 이미 일치 — 계획 §0). `builtin_gauge_tables()` 파싱 결과가 `spec(kind)` 6종과 필드 단위로 동일한지 단언하는 **게이지 패리티 가드 테스트**를 judge 쪽과 동일하게 둔다. 5K/PMS/KEYBOARD/LR2 행 추가는 Phase D.
- const 4행은 즉시 삭제하지 않고, 데이터 로드 실패 시 fallback 겸 위 테스트의 기준값으로 **한 릴리스 동안 유지**한 뒤 Phase D 에서 제거한다.
- **소비 전환은 Phase C 안에서 완료한다**(R4 의 "패리티 가드 먼저, 그 다음 소비 전환"). `JudgeProperty::for_mode` 와 `gauge::params` 가 데이터 표를 조회하고, 행이 없을 때만 내장 const 표(`JudgeProperty::defaults_for_mode` / `gauge::default_params`)로 폴백한다. 패리티 가드는 데이터 vs 내장 const 를 비교하므로 전환 후에도 실질 검증이며, 별도로 "조회 결과 == 데이터 파일" 을 단언하는 테스트가 전환 자체를 고정한다. 전환하지 않으면 `builtin_*_tables`/`load_*_tables` 가 도달 불가능한 공개 API 로 남아 C-4 의 데이터화가 무효가 된다.

### 6.2 `JudgeAlgorithm`

레퍼런스 `JudgeAlgorithm.java:12-40` 는 **4개 enum 상수(`Combo` / `Duration` / `Lowest` / `Score`)** 이고, 각 상수는 `compare(t1, t2, ptime, window, type) -> boolean`("후보 t2 가 현재 최선 t1 을 대체해야 하는가")을 갖는 pairwise 술어다. 이 형태를 그대로 미러한다.

#### 현행 rbms 의 후보 선택 = `Duration` (착수 전 반드시 읽을 것)

**앵커: `crates/rbms-judge/src/matcher.rs:318 fn press` 내부의 후보 루프 `326–351`.** 실제 코드는

```rust
let mut best: Option<usize> = None;      // matcher.rs:326
let mut best_abs = i64::MAX;             // matcher.rs:327
…
if !n.judged && !n.holding {
    if a < best_abs {                    // matcher.rs:344  (a = (n.head_us - press_us).abs())
        best_abs = a;                    // matcher.rs:345
        best = Some(i);
    }
}
```

즉 **`|Δt|` 최소 후보를 고른다 = 레퍼런스 `JudgeAlgorithm.Duration`** (`Math.abs(t1-ptime) > Math.abs(t2-ptime) && t2.getState()==0`). 계획 §1.1 J17 의 "rbms = Duration 고정" 과 일치한다.
레퍼런스의 기본값은 `Combo` 이고 `defaultAlgorithm = {Combo, Duration, Lowest}` 이므로, **rbms 는 현재 이 지점에서 레퍼런스와 다르다**(기존 divergence이며 Phase C 가 만든 것이 아니다).

따라서:

- **Phase C 의 `JudgeAlgorithm` 기본값은 `Duration` 이다.** 기본값을 `Combo` 로 두면 후보 선택이 바뀌어 현행 판정 결과가 달라진다 — 초판이 "기본값 `Combo` 로 현행 동작이 바이트 단위 동일" 이라 적은 것은 **오류**이며 여기서 정정한다.
- Phase C 의 패리티 기준은 "`JudgeAlgorithm::Duration` 경로가 `matcher.rs:326–351` 의 현행 `best_abs` 루프와 완전히 동일한 후보를 고른다" 이고, **기존 판정 테스트 전량 통과가 그 증거**다.
- 기본값을 레퍼런스와 같은 `Combo` 로 바꿀지는 **Phase D 의 결정 사항**이다(기대값이 바뀌므로 판정 테스트 재작성 동반). Phase C 는 `Combo` 를 *구현만* 하고 기본값으로 쓰지 않는다.

#### API

```rust
/// 레퍼런스 `JudgeProperty.NoteType`(JudgeProperty.java:205-206) 미러.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum NoteType { Note, LongNoteEnd, Scratch, LongScratchEnd }

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize, Default)]
pub enum JudgeAlgorithm { Combo, #[default] Duration, Lowest, Score }

impl JudgeAlgorithm {
    pub fn name(self) -> &'static str;
    /// 레퍼런스 `JudgeAlgorithm.compare(t1, t2, ptime, window, type)` 미러.
    /// `best`(=t1) 를 `cand`(=t2) 로 교체해야 하면 true.
    pub fn prefer(self, best: &NoteRef, cand: &NoteRef, ptime_us: i64, window: &JudgeWindows, note_type: NoteType) -> bool;
}

pub struct NoteRef { pub time_us: i64, pub state: u8, pub is_long: bool }
```

`is_scratch: bool` 이 아니라 **`NoteType` 4값**을 받는다. 레퍼런스의 `window.getTime(type, judge, early)`(`JudgeProperty.java:175-197`)가 `NOTE`/`LONGNOTE_END`/`SCRATCH`/`LONGSCRATCH_END` 별로 **다른 표**(`note`/`longnote`/`scratch`/`longscratch`)를 고르기 때문에, bool 로 축약하면 LN 종단 표를 지목할 수 없다.
(사실 관계: 레퍼런스의 후보 선택 호출부 `JudgeManager.java:396` 은 현재 `sc >= 0 ? SCRATCH : NOTE` 만 넘기므로 **오늘 당장의 동작 차이는 없다.** LN 종단 표는 `JudgeManager.java:362`·`:494` 의 별도 경로가 쓴다. 그럼에도 시그니처를 `NoteType` 으로 두는 이유는 ① 레퍼런스 술어의 충실한 미러이고 ② Phase D 의 LN 판정 확장이 시그니처 변경 없이 들어오기 때문이다.)

각 상수의 술어(레퍼런스 `JudgeAlgorithm.java:18-37` 그대로, judge 인덱스는 0=PG 1=GR 2=GD):

| 상수 | 술어 | Phase C |
|---|---|---|
| `Combo` | `cand.state == 0 && best.time < ptime + w.get(type, 2, false) && cand.time <= ptime + w.get(type, 2, true)` | **구현** |
| `Duration` | `(best.time - ptime).abs() > (cand.time - ptime).abs() && cand.state == 0` | **구현 · 기본값** |
| `Lowest` | 항상 `false`(첫 후보 유지) | **구현**(§11 미확인 4 해소분 참조) |
| `Score` | `cand.state == 0 && best.time < ptime + w.get(type, 1, false) && cand.time <= ptime + w.get(type, 1, true)` | **구현** |

- 4종 모두 술어 자체는 위 표 그대로 **Phase C 에서 구현한다**(각 3줄 내외). 초판의 "`Combo` 만 구현, 나머지는 `Combo` 로 위임" 은 폐기 — 위임하면 설정에서 고른 값과 실제 동작이 달라 사용자에게 거짓말이 된다.
- `JudgeEngine`/`Matcher` 에 `set_algorithm(JudgeAlgorithm)` 을 추가하고, `press()` 의 `best_abs` 루프를 `algorithm.prefer(best, cand, …)` 호출로 치환한다. **치환 후 `JudgeAlgorithm::Duration` 에서 기존 판정 테스트가 전량 통과**해야 하며, 통과하지 않으면 치환이 잘못된 것이다.
- 다만 `Combo`/`Score` 는 후보 열거 순서에 의존하므로(레퍼런스는 시간 오름차순), rbms 의 루프도 **시간 오름차순 열거임을 유지**해야 한다(`matcher.rs:333-352` 의 `while i < l.notes.len()` 이 이미 그렇다). 열거 순서를 바꾸는 최적화를 이 단계에서 넣지 않는다.
- `rehit`(空POOR) 경로(`matcher.rs:347-350`)는 알고리즘 대상이 **아니다** — 레퍼런스도 `judgenote.getState() != 0` 분기를 후보 선택 밖에서 처리한다. 현행 로직 그대로 둔다.
- 트레이트가 아니라 enum 인 이유: 설정 파일에 직렬화되어야 하고(`rbms-config::JudgeOptions.algorithm`), 레퍼런스도 enum 이며, 동적 확장 요구가 없다. 사용자 정의 알고리즘이 필요해지면 `pub trait JudgeSelector { fn prefer(&self, …) -> bool }` 를 추가하고 enum 이 이를 구현하는 형태로 확장한다(Phase C 범위 아님).

---

## 7. 위생 항목 (계획 C-6)

| 항목 | 현행 | 목표 | 비고 |
|---|---|---|---|
| CI lint 게이트 | `.github/workflows/ci.yml:61-67` 이 `cargo fmt --all --check` / `cargo clippy --workspace` 를 **둘 다 `continue-on-error: true`**, `--all-targets` 없음 | `continue-on-error` 제거 + `cargo clippy --workspace --all-targets --all-features -- -D warnings` | §1.5 대로 잔여 16종·77건을 먼저 0 으로 만든 뒤 **마지막 단계에서** 플래그를 뒤집는다 |
| `[workspace.lints]` | 없음 | 루트에 `[workspace.lints.rust]`(`unsafe_code = "deny"`, `missing_debug_implementations = "allow"`) + `[workspace.lints.clippy]`(`all = { level = "warn", priority = -1 }`, `type_complexity = "warn"`) / 각 크레이트 `[lints] workspace = true` | 각 크레이트 `Cargo.toml` 1스탠자 |
| `forbid(unsafe_code)` | 없음(workspace unsafe 0, 계획 §1.4) | **9개 라이브러리 크레이트 + `rbms-cli` = `#![forbid(unsafe_code)]`**, `apps/rbms-player` 는 `#![deny(unsafe_code)]` | `rbms-player` 는 `bytemuck` derive(`#[derive(Pod, Zeroable)]`, `gpu.rs:56`)가 `unsafe impl` 을 생성할 수 있어 `forbid`(로컬 allow 불가)를 쓰면 막힐 위험이 있다. `deny` 로 두면 필요 시 해당 구조체에만 `#[allow(unsafe_code)]`. **먼저 `forbid` 로 시도해 컴파일되면 `forbid` 유지**(§10 미확인 1) |
| 워크스페이스 dep 통합 | `serde`/`serde_json`/`ron`/`reqwest` 가 크레이트마다 개별 버전 표기, `reqwest` 클라이언트 빌더가 `rbms-ir`·`rbms-table` 중복 | `[workspace.dependencies]` 에 `serde`/`serde_json`/`ron`/`reqwest`/`thiserror` 등록 후 전 크레이트 `{ workspace = true }` | `rbms-ir` 은 Phase I 소유 → **Phase I 이후 재확인** |
| `Result<_, String>` → `thiserror` | 7곳: `rbms-table/src/lib.rs:41,50,65,140` · `rbms-ir/src/http.rs:26,53` · `rbms-render/src/skin.rs:115` · `apps/rbms-player/src/replay.rs:32` · `tablesrc.rs:48` | `TableError`/`SkinError`/`StoreError`/`LibraryError` (+ 앱 최상위 `AppError`) | `rbms-ir/http.rs` 의 2곳은 **Phase I 소유이므로 Phase C 범위에서 제외**하고 Phase I 결과에 맞춰 후속 처리 |
| 인라인 대형 테스트 분리 | `keyconfig.rs` 671줄 중 테스트 약 380 · `format.rs` 433 중 약 265 · `scores.rs` 423 중 약 300 · `ir_map.rs` 323 중 약 200 · `settings.rs` 223 중 약 120 · `folders.rs`/`tables.rs`/`replay.rs` 각 약 60~70 | 이관되는 크레이트에서는 `tests.rs`(선례: `rbms-parser/src/tests.rs`, `rbms-chart/src/tests.rs`) 로 분리, 앱에 남는 `format.rs` 는 `format_tests.rs`(**S1 소유**), `keyconfig.rs` 는 `keyconfig_tests.rs`(**H0 소유** — S1~S3 어느 단계도 `keyconfig.rs` 를 건드리지 않으므로 앱 잔여 정리 브랜치에 배정) | 테스트는 **한 줄도 삭제하지 않고 이동**. 이동 전후 `cargo test --workspace` 총 개수가 같아야 한다(**현행 실측 1,165 통과 · 0 실패 · 2 ignored**, `72bffa5`) |
| `theme`/`font` 전역 | `rbms-render/src/theme.rs:164`, `font.rs:253` `thread_local!` | 렌더 진입점이 `&RenderCtx { theme, font }` 를 받도록 인자 주입, 전역은 호환용 얇은 래퍼로 남김 | 골든 테스트(`crates/rbms-render/tests/golden.rs`)가 `embedded_only` 경로를 쓰므로 회귀 감지 가능 |
| `Player::judge` pub 필드 · `pub use dto::*` | `rbms-play/src/lib.rs` 의 `Player`, `rbms-ir/src/lib.rs` 의 **`pub use dto::*;` 줄**(현재 11행 — Phase I 가 동시 수정 중이므로 줄번호 대신 이 심볼로 찾을 것, **Phase I 이후 재확인**) | `judge()` 접근자(§3 에 포함), 글롭 re-export → 명시 목록 | `dto` 글롭은 **Phase I 이후 재확인** |
| 런타임 패닉(C7 잔여) | 기동 실패 `unwrap`(사용자 안내 없음) | `run()` 이 `Result` 를 반환하고 최상위에서 사람이 읽는 메시지 + `ExitCode::FAILURE` | `chart/lib.rs:156` NaN 은 Phase A 에서 파서 단계 거부로 처리 완료 |

---

## 8. 브랜치 분할 (파일 소유권)

**규칙: 같은 wave 안에서 *동시에* 도는 두 브랜치는 어떤 파일도 공유하지 않는다.** wave 사이, 그리고 같은 wave 안이라도 **직렬로 고정된 단계 사이**(Wave 1 의 S1→S2→S3→S4, Wave 2 의 H0→H1)에는 공유가 허용되며 순서대로 머지한다. 동시 실행 구간은 **Wave 0(P1~P6)** 뿐이므로, 무공유 검증이 필요한 곳도 거기다(§8.1).
`main.rs`/`app_*.rs` 는 본질적으로 순차라서 하나의 **직렬 스파인(S1→S2→S3→S4)** 으로 묶고, 그 주변만 병렬화한다.

### Wave 0 — 병렬 6갈래 (스파인 착수 전, 서로 무공유 — §8.1 소유권 검증)

| 브랜치 | 소유 파일 | 작업 | 검증 |
|---|---|---|---|
| **P1 judge-data** | `crates/rbms-judge/**`(`lib.rs`·`windows.rs`·`gauge.rs`·`matcher.rs`·신규 `data.rs`·`algorithm.rs`·`data/judge.ron`·`data/gauge.ron`·`Cargo.toml`) | §6 전부 + `clear_type_id`/`from_id` 수신(§4.6) + 크레이트 clippy 4건 | `cargo test -p rbms-judge`; 패리티 가드 테스트; 기존 판정 테스트 전량 통과 |
| **P2 table** | `crates/rbms-table/**` | `match_levels`(§4.4) 신설, `TableError`(thiserror), clippy 1건 | `cargo test -p rbms-table`; `compute_table_levels` 동치 테스트(현행 구현을 테스트에 복제해 대조) |
| **P3 render** | `crates/rbms-render/**` | `SkinError`, `thread_local` → 인자 주입, clippy 7+8건, `too_many_arguments` 파라미터 구조체화 | `cargo test -p rbms-render`; 골든 PNG 무변경 |
| **P4 core-lint** | `crates/rbms-parser/**`, `crates/rbms-chart/**`, `crates/rbms-model/**` | clippy(parser 2+13 · chart 4+3 · model 5), `items_after_test_module` | `cargo test -p rbms-parser -p rbms-chart -p rbms-model`; corpus MD5 불변 |
| **P5 ci-workspace** | 루트 `Cargo.toml`(`[workspace.dependencies]`·`[workspace.lints]` 추가만), `.github/workflows/ci.yml`(`--all-targets` 추가, **`-D warnings` 는 아직 아님**) | 워크스페이스 dep 통합 준비, lint 정의 | CI dev 실행이 초록(informational 유지) |
| **P6 play-lint** | `crates/rbms-play/**` | 크레이트 clippy 4건 → 0(§1.5.1). **`PlaySession` 신설은 하지 않는다**(그건 S3) — lint 정리와 `Player::judge` pub 필드 → `judge()` 접근자까지만 | `cargo test -p rbms-play`; 오토플레이 시뮬 테스트 무변경 |

> P5 는 루트 `Cargo.toml` 의 `[workspace.dependencies]`/`[workspace.lints]` 스탠자만 건드리고 `members` 는 손대지 않는다. `members` 추가는 S1 소유(다른 wave).

### Wave 1 — 직렬 스파인 (단일 에이전트, 순서 고정)

| 단계 | 소유 파일 | 작업 |
|---|---|---|
| **S1 extract** | 신규 `crates/rbms-store/**`, `crates/rbms-library/**`; 수정 `apps/rbms-player/src/{main.rs,scores.rs(삭제),replay.rs(삭제),tablesrc.rs,format.rs,ir_map.rs(이동),lib.rs(신규)}`, `apps/rbms-player/Cargo.toml`, `apps/rbms-cli/{Cargo.toml,src/main.rs}`, 루트 `Cargo.toml`(members) | §4.2·§4.3·§4.5·§4.6·§4.7 |
| **S2 config** | 신규 `crates/rbms-config/**`; 수정 `apps/rbms-player/src/{main.rs,settings.rs(삭제),folders.rs(삭제),tables.rs(삭제),app_input.rs,app_select.rs}` | §4.1. `PlayerConfig`/`PlaySettings`/`apply_settings`/`current_settings` 제거 |
| **S3 stage** | 수정 `crates/rbms-play/**`; 신규 `apps/rbms-player/src/stage/{mod,select,settings,keyconfig,tables,folders,loading,play,result}.rs`; 수정 `apps/rbms-player/src/{main.rs,lib.rs,app_play.rs(삭제),app_select.rs(해체),app_input.rs(해체)}` | §2·§3 |
| **S4 settings-ui** | 수정 `crates/rbms-config/src/settings.rs`, `apps/rbms-player/src/stage/settings.rs` | §5 |

### Wave 2 — 마무리 2단계(직렬 H0 → H1)

| 브랜치 | 소유 파일 | 작업 |
|---|---|---|
| **H0 app-lint** | `apps/rbms-player/src/keyconfig.rs` + 신규 `apps/rbms-player/src/keyconfig_tests.rs`, `apps/rbms-player/src/gpu.rs`, 그리고 S1~S4 가 손대지 않고 남긴 `apps/rbms-player/src/**` 잔여 파일 | 앱 잔여 clippy 0(초판 실측 24건 — 스파인 후 재측정), `keyconfig.rs` 인라인 테스트 약 380줄 → `keyconfig_tests.rs` 분리 | `cargo test -p rbms-player` 총수 무변경; `cargo clippy -p rbms-player --all-targets` 0 |
| **H1 gate** | 전 크레이트 `Cargo.toml`(`[lints] workspace = true`), 전 크레이트 `lib.rs`/`main.rs` 상단(`#![forbid(unsafe_code)]`), `.github/workflows/ci.yml`(`continue-on-error` 제거 + `-D warnings`), 루트 `rust-toolchain.toml`(버전 고정 — §7 R5), `docs/PROCESS.md`·`docs/crates.md`·`docs/architecture.md`·`docs/acknowledge/reference-divergences.md` | §7 게이트 플립 + §9 step 14 문서 갱신(**step 14 의 소유 브랜치는 H1 이다**) |

### 병렬성 요약

```
Wave 0 :  P1 ∥ P2 ∥ P3 ∥ P4 ∥ P5 ∥ P6       (무공유, 동시)
Wave 1 :  S1 → S2 → S3 → S4                 (main.rs/app_*.rs 공유라 직렬 불가피)
Wave 2 :  H0 → H1                            (앱 잔여 정리 → 게이트 플립)
```

`apps/rbms-player` 와 `crates/rbms-ir` 은 **Phase I 가 동시 수정 중**이므로 Wave 0 의 어느 갈래도 그 두 트리를 건드리지 않게 배치했다. S1 이 `rbms-ir` 에 `mapping` feature 를 추가하는 것이 유일한 접점이며, Phase I 머지 완료 후 함수명 기준으로 재확인한다.

### 8.1 파일 소유권 명시 목록과 소유권 검증

동시 실행 구간(Wave 0)의 각 브랜치가 쓰기 권한을 갖는 **경로 집합을 전개해 나열**한다. 여기에 없는 경로를 그 브랜치가 수정하면 규칙 위반이다.

| 브랜치 | 쓰기 허용 경로(전개) |
|---|---|
| **P1** | `crates/rbms-judge/src/lib.rs`, `crates/rbms-judge/src/windows.rs`, `crates/rbms-judge/src/gauge.rs`, `crates/rbms-judge/src/matcher.rs`, `crates/rbms-judge/src/data.rs`(신규), `crates/rbms-judge/src/algorithm.rs`(신규), `crates/rbms-judge/data/judge.ron`(신규), `crates/rbms-judge/data/gauge.ron`(신규), `crates/rbms-judge/Cargo.toml` |
| **P2** | `crates/rbms-table/src/lib.rs`, `crates/rbms-table/Cargo.toml` (+ 필요 시 `crates/rbms-table/src/**` 신규) |
| **P3** | `crates/rbms-render/src/**`(`skin.rs`·`theme.rs`·`font.rs`·`playfield.rs` 등), `crates/rbms-render/examples/**`, `crates/rbms-render/tests/**`, `crates/rbms-render/Cargo.toml` |
| **P4** | `crates/rbms-parser/**`, `crates/rbms-chart/**`, `crates/rbms-model/**` |
| **P5** | 루트 `Cargo.toml` 의 `[workspace.dependencies]`·`[workspace.lints]` **스탠자만**(`[workspace] members` 는 손대지 않음 — S1 소유), `.github/workflows/ci.yml` |
| **P6** | `crates/rbms-play/**` |

**소유권 검증**: 위 6개 경로 집합을 쌍마다 대조하면 **교집합은 공집합이다.** 서로 다른 최상위 크레이트 디렉터리(`rbms-judge` / `rbms-table` / `rbms-render` / `{rbms-parser, rbms-chart, rbms-model}` / `rbms-play`)로 분리되어 있고, 유일하게 크레이트 밖을 건드리는 P5 는 루트 `Cargo.toml`(그중 P1~P4·P6 가 손대지 않는 워크스페이스 전용 스탠자)과 `.github/workflows/ci.yml` 만 갖는다. **Phase C 가 손대는 모든 파일은 정확히 한 브랜치가 소유한다** — Wave 0 의 크레이트 6종, Wave 1 의 `apps/rbms-player/src/**` 와 신규 3크레이트(S1→S2→S3→S4 직렬이므로 파일 공유는 시간축으로 분리), Wave 2 의 앱 잔여(H0)와 게이트·문서(H1). 소유자가 없던 두 파일(`apps/rbms-player/src/keyconfig.rs`, `docs/PROCESS.md`)은 각각 H0·H1 로 배정했다(§7·§8).

착수 전 자가 점검(에이전트가 직접 실행):

```
# 자기 브랜치가 소유하지 않은 파일을 건드렸는지
git diff --name-only dev... | grep -v -E '^(<자기 소유 경로 정규식>)$'   # 출력이 비어야 한다
```

---

## 9. 단계별 순서와 테스트 (각 단계 종료 시 트리가 컴파일된다)

각 단계는 `cargo build --workspace` + `cargo test --workspace` 통과를 종료 조건으로 한다. 기준선: **테스트 1,165 통과 · 0 실패 · 2 ignored**(`72bffa5` 에서 `cargo test --workspace` 직접 실측, 2026-09-09). 초판의 1,060 과 `docs/PROCESS.md` Phase A 의 수치는 모두 stale 이므로 이 값을 쓴다. 이관 단계에서 테스트 총수는 **줄어들면 안 된다**.

| # | 단계 | 컴파일 유지 방법 | 테스트 |
|---:|---|---|---|
| 1 | P5: 루트 `[workspace.dependencies]`/`[workspace.lints]` 정의 + CI `--all-targets` | 정의만 추가, 각 크레이트는 아직 미사용 | `cargo metadata` 성공, CI 초록 |
| 2 | P1~P4·P6: 크레이트별 clippy 0 + thiserror + `match_levels` + judge 데이터화 | 각 크레이트 독립 | 크레이트별 `cargo test -p …`, judge 패리티 가드, 골든 PNG, corpus MD5 |
| 3 | S1-a: `rbms-store` 신설 + `scores.rs`/`replay.rs` 이관, 앱은 `pub use rbms_store::*` 로 경로만 교체 | 타입·필드 무변경이라 호출부 수정 최소 | 이관 테스트 전량 + 구 `scores.ron` 픽스처 로드 |
| 4 | S1-b: `rbms-library` 신설 + 스캔/`ChartDetail` 이관, `tablesrc` 가 `match_levels` 사용 | `SongEntry` 필드 pub 화만 필요 | 스캔 결과 동치(샘플 디렉터리), 표 레벨 동치 |
| 5 | S1-c: `[lib]` 타깃 + `run()` 분리 + `rbms-cli` 서브커맨드 3종 | `main.rs` 는 3줄로 축소, 나머지는 `lib.rs` 모듈 선언 | `rbms-cli scan/config/scores` 스모크, 기존 통합 테스트 |
| 6 | S1-d: **`format.rs` 에서 `clear_type_id`/`clear_type_from_id` 와 그 인라인 테스트 5개를 *삭제*하고 호출부를 `rbms_judge::gauge::…` 로 교체**(추가는 이미 P1 이 완료 — §4.6 작업 분담), `ir_map` → `rbms-ir/mapping` | re-export 로 호출부 무변경 | 이관 테스트 전량 |
| 7 | S2-a: `rbms-config` 신설(스키마 + 마이그레이션 + 픽스처 테스트), 앱은 **아직 미사용** | 신규 크레이트 추가만 | §4.1 테스트 7종 |
| 8 | S2-b: 앱을 `Config` 단일 타입으로 전환, `PlayerConfig`/`PlaySettings`/`apply_settings`/`current_settings` 삭제, `folders.ron`/`tables.ron` 흡수 | 한 커밋 안에서 전환(중간 상태 없음) | 기존 설정 파일로 기동 → 값 보존 확인(수동 1회 + 픽스처 테스트) |
| 9 | S3-a: `App` → `{ shared: AppShared, stage: Stage }` 분리(변형은 아직 데이터 없음) | 필드 이동만, 로직 동일 | 전량 |
| 10 | S3-b: `PlaySession` 을 `rbms-play` 에 신설(`SoundSink` 트레이트), 앱의 Play 경로가 이를 사용 | 앱이 `AudioEngine` 어댑터로 `SoundSink` 구현 | `NullSink` 헤드리스 리플레이 재현 테스트 신설, 오토플레이 통합 테스트 |
| 11 | S3-c: 스테이지별 상태 구조체 + `StageHandler` 구현, `frame()`/`window_event()` 를 디스패치로 축약, `self.stage = …` **22곳**(§1.2) → `Transition` | 스테이지 한 개씩 옮기고 나머지는 기존 분기 유지(각 커밋마다 컴파일) | 스테이지별 헤드리스 렌더 스냅샷 + 전량 |
| 12 | S4: descriptor 테이블 도입, 정수 인덱스 4상수 + `SETTING_TABS` 삭제 | `setting_line`/`adjust_setting` 을 descriptor 호출로 치환 | §5 테스트 5종 + 라벨 스냅샷 동치 |
| 13 | H0 → H1: 앱 잔여 clippy 0 + `keyconfig_tests.rs` 분리, 이어서 `[lints] workspace = true` 일괄, `forbid/deny(unsafe_code)`, CI `-D warnings` + `continue-on-error` 제거 | 앞 단계에서 경고 0 달성 후에만 | `cargo clippy --workspace --all-targets -- -D warnings` 성공, `cargo fmt --check` 성공 |
| 14 | **(H1 소유)** 문서: `docs/PROCESS.md` Phase C 체크, `docs/crates.md`(신규 3크레이트), `docs/architecture.md`(Stage/PlaySession), `docs/acknowledge/reference-divergences.md` — ① **후보 선택 기본값이 `Duration` 이라 레퍼런스 기본값 `Combo` 와 다름**(기존 divergence, §6.2) ② 계획 C-4 는 `JudgeAlgorithm` **trait** 을 적었으나 스펙은 **enum** 을 택함(사유 §6.2 말미) ③ 게이지는 모드당 9종 중 **6종만**(코스 게이지 3종 없음, §6.1) | — | 문서 경로 grep 검증 |

---

## 10. 리스크

| # | 리스크 | 영향 | 완화 |
|---|---|---|---|
| R1 | Phase I 가 `apps/rbms-player`(NETWORK 탭·랭킹 패널·설정 동기화)와 `crates/rbms-ir` 을 대폭 바꾸는 중 → S1~S4 가 그 위에 대규모 이동을 얹는다 | 머지 충돌 다발, Phase I 신규 UI 유실 | **Phase C 는 Phase I 머지 완료 후 착수**(PROCESS 실행 순서 I→B→C). 착수 직전 `apps/rbms-player/src/*.rs` 줄 수·함수 목록을 재측정해 이 문서 §1.1 을 갱신. Phase I 가 추가한 NETWORK 설정 항목은 §5 descriptor 행으로 편입 |
| R2 | S2-b(설정 단일화)가 사용자 기존 `settings.ron`/`folders.ron`/`tables.ron` 을 잘못 흡수하면 설정 유실 | 사용자 데이터 손상 | 마이그레이션은 **원본 파일을 삭제하지 않는다**. 상위 버전은 저장 자체를 안 함. 픽스처 7종 + 실제 사용자 파일 1회 수동 확인 |
| R3 | S3-c 가 `frame()` 411줄을 옮기며 미묘한 순서 의존(프리뷰 정지 → 오디오 엔진 생성 순서, `stop_preview` 가 Play 진입 전에 불려야 함 — `app_play.rs:9-13` 주석)을 깨뜨림 | cpal 스트림 2개 동시 오픈 → 무음/크래시 | `on_exit`/`on_enter` 훅에 그 계약을 명시적으로 표현하고, 전이 순서(`on_exit` → 교체 → `on_enter`)를 `App` 한 곳에 고정. Phase B 의 단일 `AudioEngine` 이 선행되면 리스크가 크게 줄어듦(실행 순서 B→C 가 이미 그렇게 잡혀 있음) |
| R4 | judge 데이터화가 Phase A 에서 맞춘 레퍼런스 수치를 미세하게 바꿈 | 판정 패리티 회귀(재현 어려움) | const 표를 남긴 채 **파싱 결과 == const** 패리티 가드 테스트를 먼저 넣고, 그 후에만 사용처를 데이터 경로로 전환 |
| R5 | `-D warnings` 게이트가 신규 러스트 릴리스의 새 린트로 CI 를 깨뜨림 | dev 브랜치 정지 | **실측(2026-09-09)**: 루트 `rust-toolchain.toml` 은 `channel = "stable"`(버전 미고정)이고 CI 는 `dtolnay/rust-toolchain@stable`(ci.yml:38, 57) → 둘이 어긋나지는 않지만 **어느 쪽도 버전을 고정하지 않는다.** 따라서 `-D warnings` 를 켜는 H1 에서 `rust-toolchain.toml` 의 `channel` 을 그때의 구체 버전(예: `"1.9x.y"`)으로 바꾸고 CI 는 `dtolnay/rust-toolchain@master` + `toolchain: 파일 준수` 로 맞춘다. 이것이 R5 의 실제 완화책이다 |
| R6 | `forbid(unsafe_code)` 가 `bytemuck` derive 와 충돌 | `rbms-player` 빌드 실패 | `rbms-player` 만 `deny` 로 시작, 컴파일되면 `forbid` 로 승격(§7) |
| R7 | 테스트 이동 중 누락 | 커버리지 조용한 감소 | 각 단계 종료 시 `cargo test --workspace` 의 **총 통과 수를 기록**하고 이전 단계 대비 감소하면 중단 |
| R8 | `Stage` enum 크기 팽창으로 매 전이마다 대형 memcpy | 프레임 스파이크 | `Play` 변형은 `Box`, 다른 변형도 128 B 초과 시 Box. `std::mem::size_of::<Stage>()` 상한 단언 테스트 |

---

## 11. 미확인 사항

> 2026-09-09 비평 대응으로 **2·3·4·5 는 해소**되어 §11.1 로 옮겼다. 아래에 남은 것만이 실제 미확인이다.

1. **`bytemuck` derive 와 `forbid(unsafe_code)` 의 실제 충돌 여부** — `gpu.rs:56 struct Instance` 의 `#[derive(Pod, Zeroable)]` 가 `unsafe impl` 을 생성하는데, 최신 `bytemuck_derive` 가 생성 코드에 `#[allow(unsafe_code)]` 를 붙이는지 확인하지 않았다. H1 단계에서 실제 컴파일로 판정한다.
2. **Phase I 가 추가할 앱 파일 목록** — NETWORK 탭·랭킹 패널·리플레이 다운로드가 새 모듈 파일을 만드는지(예: `app_network.rs`) 알 수 없다. S3 의 스테이지 분해 대상 목록(§2.2)이 그만큼 늘어날 수 있어, 착수 직전 `apps/rbms-player/src/` 파일 목록을 다시 뜬다.
3. **`AudioHub`(Phase B 단일 엔진)의 최종 API** — §2.1 `AppShared.audio` 를 그렇게 적었으나 Phase B 설계가 확정하는 타입이다. Phase B 완료 후 이름·시그니처를 맞춘다.
4. **실기 확인 미실시** — 이 문서는 정적 코드 읽기와 `cargo clippy` 실측만으로 작성했다. GUI 를 띄운 동작 확인은 하지 않았다.

### 11.1 해소된 미확인 사항 (2026-09-09 실측)

| 초판 번호 | 항목 | 결론 |
|---|---|---|
| 2 | clippy 총수 기준 불일치 | **해소.** `72bffa5` 에서 `cargo clippy --workspace --all-targets` 재실측 = **고유 77건**, 크레이트별 분포는 §1.5.1 표. `docs/PROCESS.md` 의 94/101 은 stale 이며 §1.5.1 이 정본이다. |
| 3 | CI 툴체인 고정 여부 | **해소.** 루트 `rust-toolchain.toml` 은 `channel = "stable"` + `components = ["rustfmt","clippy"]` 이고 CI 는 `ci.yml:38,57` 에서 `dtolnay/rust-toolchain@stable`. **둘 다 stable 채널이라 불일치는 없으나 어느 쪽도 버전을 고정하지 않는다** → R5 완화책은 "액션이 파일을 존중하는지" 가 아니라 **`rust-toolchain.toml` 의 채널을 구체 버전으로 바꾸는 것**이다(§7 R5 갱신 완료). |
| 4 | `JudgeAlgorithm::Lowest` 의 시맨틱 | **해소.** 후보 열거는 `JudgeManager.java:385-391` 의 `for(judgenote = lanemodel.getNote(); …)` 루프이고, `dmtime < mjudgestart` 를 `continue`, `dmtime >= mjudgeend` 에서 `break` 하므로 **시간 오름차순**이다. `compare` 가 항상 `false` 인 `Lowest` 는 곧 **판정창 안에서 가장 이른(= 화면상 가장 아래) 미판정 노트를 유지**한다는 뜻. Phase D 로 미룰 이유가 없어 §6.2 에서 구현 대상으로 올렸다. |
| 5 | `GaugeParams` 필드 구성 | **해소.** `GaugeProperty.java` 의 `GaugeElementProperty` 생성자는 `(modifier, min, max, init, border, float[6] deltas, float[][] guts)` 이고, 이는 현행 `crates/rbms-judge/src/gauge.rs:33 struct Spec` 과 **필드 단위로 1:1**이다. §6.1 에 `GaugeModifier`/`GaugeParams`/`GaugeSet` 로 명시했다. 레퍼런스는 모드당 9종(코스 게이지 CLASS/EXCLASS/EXHARDCLASS 포함)이나 rbms 에는 코스가 없어 **Phase C 는 6종**. |

---

## 12. 비평 반영 (2026-09-09)

완전성 비평(외부 리뷰)에 대해 **인용된 코드와 레퍼런스를 전부 직접 열어 확인**한 뒤 반영한 결과다. 실측 기준 커밋은 `dev` `72bffa5`.

### 반영한 것

| # | 지적 | 확인한 사실 | 스펙 변경 |
|---|---|---|---|
| 1 | 판정 알고리즘 기본값 오주장 | `matcher.rs:326-345` 의 `best_abs = min |Δt|` → 현행은 **`Duration`**. 기본값을 `Combo` 로 두면 동작이 바뀐다 | §6.2 전면 개정: 기본값 `Duration`, 현행 후보 루프 앵커(`matcher.rs:318 fn press` / `326–351`) 명시, "바이트 단위 동일" 주장 철회, 레퍼런스 기본값(`Combo`)과의 차이를 divergence 로 §9 step 14 에 기록 지시 |
| 2 | `prefer` 가 `NoteType` 4값을 bool 로 축약 | `JudgeProperty.java:205` `enum NoteType {NOTE, LONGNOTE_END, SCRATCH, LONGSCRATCH_END}` + `:175-197 getTime(type, judge, early)` 이 타입별로 다른 표를 고름 | §6.2 시그니처를 `note_type: NoteType` 로 변경하고 `NoteType` 정의 추가. 단, 현재 호출부(`JudgeManager.java:396`)는 NOTE/SCRATCH 만 넘긴다는 사실도 함께 기록 |
| 3 | `GaugeParams` 스키마 부재 | `gauge.rs` 에는 `GaugeProperty` 대응 공개 구조체가 없고 private `Spec`(33행)뿐 | §6.1 에 `GaugeModifier`/`GaugeParams`/`GaugeSet` 정의 + `gauge.ron` 의 Phase C 내용(7K 6종)·게이지 패리티 가드 테스트 추가 |
| 4 | 계획 C11 미언급 | 계획 123행에 C11 존재, 스펙 본문·브랜치 어디에도 없음 | §1.4.1 신설 — 제외 사유(스크롤 모델 재설계 = Phase E)와 "어느 브랜치도 건드리지 않음" 명시 |
| 5 | clippy 28건 미배정으로 게이트 파손 | 재실측: `rbms-play` 4 · `rbms-player` 24 · `rbms-render` 예제 2 가 미배정 | §1.5.1 타깃별 표 신설(합 77 검산) + **P6(Wave 0, `crates/rbms-play/**`)** 와 **H0(Wave 2 선행, 앱 잔여)** 브랜치 신설 |
| 6 | `clear_type_id` 이관 중복 배정 | P1 과 S1-d 양쪽에 "이관" 으로 적혀 있었음 | §4.6 에 작업 분담 문단 추가(P1 = 추가만 / S1-d = 삭제·호출부 교체), §9 step 6 문구 고정 |
| 7 | 소유자 없는 파일 | `keyconfig.rs`(테스트 분리 지시만 있고 소유자 없음), `docs/PROCESS.md`(step 14 에 브랜치 미배정) | `keyconfig.rs`/`keyconfig_tests.rs` → **H0**, step 14 문서 일체 → **H1** 로 배정 |
| 8 | `self.stage = Stage::X` 개수 오류 | 실측 **22곳**(`main.rs` 1135/1168/1183/1238 누락) | §1.2 를 22곳으로 정정하고 `main.rs` 4곳 추가, §2.3 도 22곳으로. 완료 조건을 "grep 0건" 으로 명시 |
| 9 | `App` 필드 수 | `main.rs:717-856` 최상위 필드 실측 **100** | §1.1·§2.1 을 100필드로 정정 |
| 10 | 테스트 기준선 | `cargo test --workspace` 실측 **1,165 통과 / 0 실패 / 2 ignored** | §7·§9 의 1,060 을 1,165 로 정정(R7 게이트가 이 숫자에 걸린다) |
| 11 | `pub use dto::*` 앵커 | 실제 `rbms-ir/src/lib.rs:11` | 줄번호 대신 심볼 앵커로 교체 + "Phase I 이후 재확인" |
| 12 | 커밋 해시 stale | HEAD 는 `72bffa5` | §1.1 헤더 갱신, R1 의 재측정 항목에 해시 갱신 포함 |
| 13 | trait→enum 결정이 divergence 문서에 없음 | 계획 C-4 는 trait | §9 step 14 에 기록 항목으로 추가 |
| 14 | `§1.5` 교차참조 충돌 | 스펙 자신의 §1.5 는 clippy 표 | 본문의 `§1.5 P2/P6/P9` 를 전부 `계획 §1.5 …` 로 |
| 15 | 소유권 표 불명확 | — | §8.1 신설: Wave 0 6브랜치의 경로 집합 전개 + **소유권 검증(교집합 공집합)** 문장 + 브랜치별 자가 점검 명령 |

### 반영하지 않은 것 (지적이 틀렸음을 확인)

| 지적 | 확인 결과 |
|---|---|
| "§1.2 draw 8분기 줄번호가 한 칸씩 밀렸다 — 실측은 528 Loading / 553 Select / …, 487 은 `let anchor = self.anchor_us;`" | **틀림.** `grep -n 'Stage::' apps/rbms-player/src/app_play.rs` 결과 `487: Stage::Loading => {`, `528: Stage::Select => {`, `553: Stage::Play`, `616: Stage::Settings`, `644: Stage::KeyConfig`, `691: Stage::Tables`, `726: Stage::Folders`, `753: Stage::Result`. 스펙의 목록이 정확하므로 변경하지 않았다. |
| "`main.rs:1312 config_dir` 은 실제 1311행" | **틀림.** `grep -n 'fn config_dir' apps/rbms-player/src/main.rs` → `1312:fn config_dir() -> PathBuf {`. 스펙이 맞다. |
| (부분) "`is_scratch: bool` 은 LN 종단 표를 지목 못 해 **기능이 깨진다**" | **결함의 크기는 과장.** 레퍼런스의 후보 선택 호출부(`JudgeManager.java:396`)가 넘기는 값은 `sc >= 0 ? SCRATCH : NOTE` 뿐이라 오늘의 동작 차이는 0이다. 그럼에도 미러 충실성·Phase D 확장성을 이유로 `NoteType` 으로 바꾸었으므로 위 반영표 #2 에 넣었다. |

### 남은 위험

- §11 의 4항목(Phase I 앱 파일 목록 · Phase B `AudioHub` API · `bytemuck`↔`forbid` 충돌 · 실기 확인)은 여전히 미확인이며, 각각 R1·R6·§9 step 13 에서 판정한다.
- §1.5.1 의 `apps/rbms-player` 24건은 S1~S3 가 앱을 다시 쓴 뒤에는 달라진다. **H0 는 착수 시 재측정**한다.
