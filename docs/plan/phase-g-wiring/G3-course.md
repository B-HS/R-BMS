# G3 코스 — G8 배선 지시

대상 스펙: `docs/plan/2026-09-09-phase-g-spec.md` §5 (코스/단위), §9.2 G3 행, §11 테스트 7·8.
이 문서만 보고 배선할 수 있게 파일·앵커(심볼명)·코드·근거를 항목마다 적는다. 줄 번호는 쓰지 않는다.

## 0. G3 가 만든 것 (읽기 전용 참조)

| 심볼 | 위치 | 용도 |
|---|---|---|
| `rbms_course::Course` / `CourseChart` / `CourseConstraint` / `TrophyRule` | `crates/rbms-course/src/model.rs` | 코스 문서. `validate()` 는 정규화 후 채택 여부만 돌려준다 |
| `rbms_course::{parse, load_file, load_dir, save, file_name_for}` | `crates/rbms-course/src/load.rs` | `*.json` 배열/단일 양쪽 로드 |
| `rbms_course::{CourseRun, CourseStep, CourseTotals, StageResult, CLEAR_NO_PLAY, CLEAR_FAILED}` | `crates/rbms-course/src/run.rs` | 진행 상태 |
| `course_ui::{SelectTab, CourseList, CourseEntry, CourseRow, CourseOverrides, RandomLock}` | `apps/rbms-player/src/course_ui.rs` | 코스 탭 목록·제약 적용 |
| `course_ui::{courses_dir, library_index, constraint_badge, stage_label, result_rows, COURSE_DIR_NAME}` | 같은 파일 | 경로·해석·표시 |
| `course_ir::{build_course_submission, course_block_reason, submit, UNRELEASED_COURSE_REASON}` | `apps/rbms-player/src/course_ir.rs` | IR 코스 제출 |

두 플레이어 모듈은 지금 호출부가 없어 파일 상단에 `#![allow(dead_code)]` 가 있다.
**첫 배선을 넣을 때 그 줄을 지운다.**

## 1. 선행 조건 — 다른 갈래/후속 Phase 에 필요한 것 (착수 전 확인)

### 1.1 게이지·콤보 이월 API 부재 (차단 항목)

스펙 §5.3 은 스테이지 경계에서 게이지와 콤보를 **이월**하라고 한다. `CourseRun` 은 이월값
(`carry_gauge`, `totals.combo_carry`)을 정확히 들고 있지만, **다음 스테이지의 세션에 그 값을
심을 경로가 현재 없다.**

- `rbms_play::SessionOptions` 에 시작 게이지·시작 콤보 필드가 없다.
- `PlaySession::judge()` 는 `&JudgeEngine` 뿐이라 `judge.gauge`/`judge.combo`(둘 다 `pub` 필드)에
  가변 접근이 안 된다.
- 코스 게이지 슬롯(`GaugeIndex::Class`/`ExClass`/`ExHardClass`)도 `SessionOptions.gauge: GaugeKind`
  로는 고를 수 없다. `GaugeSet::select` 는 `pub` 이지만 역시 가변 경로가 없다.

`crates/rbms-play/**` 와 `crates/rbms-judge/**` 는 **Phase G 의 어느 갈래도 소유하지 않는다**
(스펙 §9.3 read-only 목록). 따라서 아래 패치는 G8 이 소유권을 확인한 뒤에만 넣는다.

```rust
// crates/rbms-play/src/session.rs — struct SessionOptions 에 추가
    /// Course play: the gauge slot the run is judged on and the value it carries in from the
    /// previous stage. `None` outside a course.
    pub course_gauge: Option<(rbms_judge::gauge::GaugeIndex, f32)>,
    /// Combo the run starts on, carried in from the previous stage of a course.
    pub initial_combo: u32,
```

```rust
// crates/rbms-play/src/session.rs — PlaySession::new, 판정 엔진을 만든 직후
        if let Some((index, value)) = options.course_gauge {
            player.judge_mut().gauge.select(index);
            player.judge_mut().gauge.set_value_at(index, value);
        }
        if options.initial_combo > 0 {
            player.judge_mut().combo = options.initial_combo;
            player.judge_mut().max_combo = options.initial_combo;
        }
```

```rust
// crates/rbms-judge/src/gauge.rs — impl GaugeSet 에 추가
    /// Put one gauge at `value` without touching the other eight, which is how a course stage
    /// starts from the gauge the stage before it ended on.
    pub fn set_value_at(&mut self, index: GaugeIndex, value: f32) {
        let gauge = &mut self.gauges[index.index()];
        gauge.add_value(value - gauge.value());
    }
```

**이 패치가 없어도 코스는 돌아간다.** 스테이지마다 게이지가 표 기본값에서 다시 시작할 뿐이고,
`CourseRun` 의 누적·트로피·제출은 그대로 정확하다. 그 상태로 낼 거면
`docs/acknowledge/reference-divergences.md` 에 "코스 게이지·콤보 이월 미적용" 을 등재한다.

### 1.2 NO GOOD / NO GREAT 미적용 (스펙 §12 R7 그대로)

판정 데이터가 런 단위로 교체되지 않으므로 두 제약은 **저장·표시만** 한다.
`CourseOverrides::unapplied` 가 배지 문자열을 돌려주므로 코스 행/시작 화면에 그대로 띄운다.

### 1.3 sha256 해석 (G1 곡DB 이후)

`course_ui::library_index` 는 md5 로만 찾는다. 스캔 라이브러리(`rbms_library::Library`)에 sha256
인덱스가 없기 때문이다. G1 의 `SongDb` 가 들어오면 이 함수 한 개만 sha256 우선으로 바꾼다.
바꿀 곳은 `library_index` 본문뿐이고 호출부(`CourseList::resolve_in_library`)는 그대로다.

## 2. `apps/rbms-player/src/lib.rs`

### 2.1 use 추가

**위치**: `use course_ir::…` 는 기존 `use app_network::build_server;` 부터 시작되는 `use` 블록,
알파벳 순서상 `use assets::…` 앞.

```rust
use course_ir::{UNRELEASED_COURSE_REASON, build_course_submission, course_block_reason};
use course_ui::{CourseEntry, CourseList, CourseOverrides, SelectTab, courses_dir, library_index, result_rows, stage_label};
```

### 2.2 `struct AppShared` 필드 추가

**위치**: `struct AppShared` 의 `replay: Option<Replay>,` 바로 아래(런 전체에 걸리는 상태끼리 모음).

```rust
    /// The course being played, when one is. Every stage loads, plays and finishes through the
    /// screens a single chart uses; this is what tells them they are inside a course.
    course_run: Option<rbms_course::CourseRun>,
    /// One IR block reason per finished stage, folded into the course verdict at the end.
    course_stage_reasons: Vec<Option<String>>,
    /// The play settings as they stood before the course rewrote them, put back when it ends.
    course_settings_backup: Option<rbms_config::PlayOptions>,
```

`AppShared` 를 만드는 곳(`App::new` 계열 리터럴)에 `course_run: None, course_stage_reasons: Vec::new(),
course_settings_backup: None,` 을 넣는다.

### 2.3 코스 시작·중단 헬퍼

**위치**: `fn ir_submission_block_reason` 바로 위(같은 성격의 자유 함수 구역).

```rust
/// Gauge a course stage starts on before anything has been played, read off the class row of the
/// gauge table so the number the browser shows is the one the run will use.
fn course_start_gauge(set: rbms_judge::gauge_tables::GaugeSetId) -> f32 {
    rbms_judge::builtin_gauge_tables()
        .get(set.data_key())
        .map_or(0.0, |table| table.at(rbms_judge::gauge::GaugeIndex::Class).init)
}

/// Begin a course: rewrite the play settings the constraints narrow, remember the originals, and
/// hand the first stage to LOADING.
fn start_course(shared: &mut AppShared, entry: &CourseEntry) -> Transition {
    let overrides = CourseOverrides::of(&entry.course);
    shared.course_settings_backup = Some(shared.config.play.clone());
    overrides.apply_to(&mut shared.config.play);
    shared.course_stage_reasons.clear();
    let set = overrides.gauge_set_for(shared.config.judge.gauge_set).unwrap_or_default();
    shared.course_run = Some(rbms_course::CourseRun::new(entry.course.clone(), course_start_gauge(set)));
    load_course_stage(shared)
}

/// Hand the stage the run is standing on to LOADING, or give up when the library cannot supply it.
fn load_course_stage(shared: &mut AppShared) -> Transition {
    let Some(chart) = shared.course_run.as_ref().and_then(|run| run.current_chart()).cloned() else {
        return end_course(shared);
    };
    let Some(index) = library_index(&shared.library, &chart) else {
        notify(Level::Warn, format!("course stage not in the library: {}", chart.title));
        return end_course(shared);
    };
    shared.release_play_audio();
    Transition::To(Stage::Loading(LoadingState::song(index)))
}

/// Put the player's own settings back and forget the run. Called on the course result screen's way
/// out and on an abandoned course alike.
fn end_course(shared: &mut AppShared) -> Transition {
    if let Some(play) = shared.course_settings_backup.take() {
        shared.config.play = play;
    }
    shared.course_run = None;
    shared.course_stage_reasons.clear();
    Transition::Back
}
```

**근거**: 스펙 §5.4(시작 → Loading → Play → 자동 진행 → CourseResult), §5.4 마지막 줄(Esc 중단은
기록 없음). 설정을 복원하지 않으면 코스가 강제한 `hispeed=1.0`·`random=Off` 가 코스 후에도 남는다.

## 3. `apps/rbms-player/src/stage/mod.rs`

### 3.1 `enum Stage` / `enum StageId`

**위치**: `Result(ResultState)` 변형 바로 아래.

```rust
    CourseResult(CourseResultState),
```

`StageId` 에도 같은 자리에 `CourseResult,` 를 넣고, `Stage::id()` 대응 표(있다면)와
`StageHandler` 디스패치 `match` 에 한 줄씩 추가한다.

**`Stage::Course` 변형은 넣지 않는다.** 스펙 §5.4 는 코스 목록을 "정렬 축이 아니라 곡선택의 별도
탭"으로 규정한다(`SelectTab { Songs, Courses }`). 코스 목록은 `SelectState` 안에 살고 별도 화면이
아니다. 코스 상세/확인 화면을 굳이 만들 거면 `course_ui::CourseList`·`CourseRow` 를 그대로 쓰면
되고, 그때만 변형을 추가한다.

### 3.2 `pub(crate) mod course_result;` (신규 화면)

`stage/result.rs` 옆에 `stage/course_result.rs` 를 만들고 `CourseResultState` 를 둔다. 표시 내용은
`course_ui::result_rows(&run)` 이 이미 라벨/값 쌍으로 준다.

```rust
pub(crate) struct CourseResultState {
    rows: Vec<(String, String)>,
    cleared: bool,
    course_name: String,
}

impl CourseResultState {
    pub(crate) fn of(run: &rbms_course::CourseRun) -> CourseResultState {
        CourseResultState { rows: crate::course_ui::result_rows(run), cleared: run.failed_at.is_none(), course_name: run.course.name.clone() }
    }
}
```

`handle_key` 의 나가기 키(`ResultState` 와 같은 집합)는 `crate::end_course(ctx.shared)` 를 부른다.

## 4. `apps/rbms-player/src/stage/select/**`

### 4.1 `SelectState` 필드

**위치**: `struct SelectState` 의 `filter: FilterPanel,` 아래. `Default` 리터럴에도 같은 순서로 추가.

```rust
    /// Which of the two lists the browser is showing.
    tab: SelectTab,
    /// The course list, resolved against the library it was built for.
    courses: CourseList,
```

### 4.2 코스 목록 적재

**위치**: `SelectState::on_enter`(없으면 `update` 의 목록 재빌드 분기, `applied` 를 갱신하는 곳).
라이브러리 세대가 바뀌었을 때만 다시 읽는다.

```rust
        if self.courses.is_empty() {
            self.courses = CourseList::load(&courses_dir(&config_dir()), &ctx.shared.library);
        } else {
            self.courses.resolve_in_library(&ctx.shared.library);
        }
```

`config_dir()` 는 `lib.rs` 의 자유 함수라 `crate::config_dir()` 로 부른다. 스캔이 끝나
`Stage::Select` 로 돌아올 때마다 `resolve_in_library` 를 다시 부르면 새로 스캔된 곡이 코스의
누락 표시를 해소한다.

### 4.3 키 처리

**위치**: `SelectState::handle_key`, 검색창·모달이 키를 먹지 않는 분기 안.

```rust
            KeyCode::Tab if key.pressed => {
                self.tab = self.tab.next();
                return Transition::Stay;
            }
```

`self.tab == SelectTab::Courses` 인 동안 위/아래는 `self.courses.move_cursor(-1 / 1)`,
Enter 는 아래로 간다.

```rust
            KeyCode::Enter if key.pressed && self.tab == SelectTab::Courses => {
                let Some(entry) = self.courses.focused() else {
                    return Transition::Stay;
                };
                if !entry.is_playable() {
                    notify(Level::Warn, format!("{} stage(s) missing from the library", entry.missing.len()));
                    return Transition::Stay;
                }
                let entry = entry.clone();
                return crate::start_course(ctx.shared, &entry);
            }
```

**근거**: 스펙 §5.4 "각 차트의 라이브러리 존재 여부(전부 있어야 시작 가능)".

### 4.4 그리기

`self.courses.rows()` 가 `CourseRow { title, detail, badges, playable }` 를 목록 순서로 준다.
곡 리스트와 같은 행 렌더러에 태워 `playable == false` 면 흐리게 그린다.
헤더의 탭 이름은 `SelectTab::label()`.

## 5. `apps/rbms-player/src/app_result.rs` — 스테이지 종료 후 진행

### 5.1 `enter_result` 에서 코스 분기

**위치**: `pub(crate) fn enter_result`, 마지막 줄
`Transition::To(Stage::Result(ResultState::new(view)…))` **직전**.
그 위에서 이미 계산된 `summary`·`block_reason`·`lamp` 를 그대로 쓴다.

```rust
    if shared.course_run.is_some() {
        return advance_course(shared, &summary, block_reason, lamp);
    }
```

### 5.2 `advance_course` 신규 함수

**위치**: 같은 파일, `fn start_song` 바로 위.

```rust
/// Fold the stage that just ended into the course and go wherever it says: the next stage, or the
/// course result screen.
fn advance_course(
    shared: &mut AppShared,
    summary: &rbms_play::PlaySummary,
    block_reason: Option<&'static str>,
    lamp: ClearType,
) -> Transition {
    shared.course_stage_reasons.push(block_reason.map(str::to_string));
    let stage = rbms_course::StageResult {
        ex_score: summary.ex_score,
        max_ex_score: summary.max_ex_score,
        notes: summary.total_notes,
        counts: summary.counts,
        empty_poor: summary.empty_poor,
        fast: summary.fast,
        slow: summary.slow,
        combo_breaks: combo_breaks(&shared.mode, summary.counts),
        max_combo: summary.max_combo,
        combo_at_end: 0,
        gauge_value: summary.gauge_value,
        clear: clear_type_id(lamp),
        survived: !summary.failed,
    };
    let Some(run) = shared.course_run.as_mut() else {
        return Transition::Back;
    };
    match run.advance(&stage) {
        rbms_course::CourseStep::Next => load_course_stage(shared),
        rbms_course::CourseStep::Cleared | rbms_course::CourseStep::Failed => finish_course(shared),
    }
}
```

`combo_at_end` 는 **런이 끝난 시점의 콤보**여야 하는데 `PlaySummary` 가 지금 그 값을 싣지 않는다
(최대 콤보만 있다). 그래서 위 코드는 `0` 으로 두었다. 둘 중 하나로 마무리한다.

1. `rbms_play::PlaySummary` 에 `combo: u32`(= `judge.combo`)를 추가한다. `PlaySession::summary`
   한 줄이고 §1.1 패치와 같은 파일이라 그때 같이 넣는 것이 자연스럽다. 그러면
   `combo_at_end: summary.combo` 로 바꾼다.
2. 넣지 않으면 `0` 그대로 두고, §1.1 이 미적용인 것과 같은 이유로
   `reference-divergences.md` 에 함께 등재한다. `totals.max_combo` 는 이 값과 무관하게 정확하고,
   콤보 이월(§1.1)이 없으면 어차피 쓰이지 않는다.

**근거**: 스펙 §5.3(스테이지 결과를 받아 `CourseStep` 을 돌려준다), §11 테스트 8.

### 5.3 `finish_course` 신규 함수

**위치**: `advance_course` 바로 아래.

```rust
/// The course is over: submit it if every stage was submittable, then show the course result.
fn finish_course(shared: &mut AppShared) -> Transition {
    let Some(run) = shared.course_run.as_ref() else {
        return Transition::Back;
    };
    let mut reasons = shared.course_stage_reasons.clone();
    if !run.course.release {
        reasons.push(Some(UNRELEASED_COURSE_REASON.to_string()));
    }
    match course_block_reason(&reasons) {
        Some(reason) => println!("course not submitted: {reason}"),
        None if shared.config.network.server_url.is_none() => {}
        None => {
            let sub = build_course_submission(run, &submission_player_id(&shared.session, &shared.config.network.player_id));
            let server = shared.server.clone();
            std::thread::spawn(move || {
                let _ = crate::course_ir::submit(server.as_ref(), &sub);
            });
        }
    }
    let state = CourseResultState::of(run);
    Transition::To(Stage::CourseResult(state))
}
```

- `course_block_reason` 은 스테이지 사유를 코스 단위로 접는다. **하나라도 차단이면 코스 전체가
  제출 불가**(스펙 §5.5).
- `UNRELEASED_COURSE_REASON` 은 코스 파일이 release 가 아닐 때의 사유다(레퍼런스
  `CourseResult.java:96` 의 `isUpdateCourseScore() && isRelease()`).
- `course_ir::submit` 은 `IrError::Unsupported` 를 오류가 아니라 "이 서버엔 코스 엔드포인트가 없다"
  로 접어 `accepted=false` 인 응답을 돌려준다(스펙 §10).
- 스레드 대신 `rbms_ir::spawn_submit` 같은 워커에 태우고 싶으면 그쪽이 낫다. 요지는 프레임을 막지
  않는 것.
- G7 의 멀티 IR 이 배선돼 있으면 `shared.server` 대신 **primary 프로필 서버**를 쓴다(스펙 §10:
  코스는 primary 프로필에만 제출·조회).

### 5.4 `lntype` 덮어쓰기 (선택)

`build_course_submission` 은 코스가 LN 제약을 갖고 있을 때만 `lntype` 을 채우고, 아니면 0 을 쓴다.
런이 실제로 쓴 값이 있으면 제출 직전에 덮어쓴다.

```rust
            sub.lntype = state.lntype;
```

## 6. `apps/rbms-player/src/stage/loading.rs` — 코스 스테이지 로드

`LoadingState::song(index)` 를 그대로 쓰므로 로딩 자체는 손댈 것이 없다. 두 가지만 얹는다.

### 6.1 진행 표시

**위치**: `LoadingState::heading`.

```rust
        if let Some(run) = shared.course_run.as_ref() {
            return ("COURSE", format!("{} - {}", run.course.name, stage_label(run)));
        }
```

### 6.2 게이지·콤보 이월 (§1.1 패치가 들어온 뒤에만)

**위치**: `LoadingState::finish_song` 이 `SessionOptions` 를 만드는 곳
(실제로는 `into_play`/세션 생성부. `SessionOptions` 리터럴을 찾는다).

```rust
        let course = shared.course_run.as_ref();
        options.course_gauge = course.map(|run| (rbms_judge::gauge::GaugeIndex::Class, run.carry_gauge));
        options.initial_combo = course.map_or(0, |run| run.totals.combo_carry);
```

## 7. `apps/rbms-player/src/app_input.rs` / 플레이 중 키

### 7.1 하이스피드 잠금

**위치**: `ControlAction::HispeedUp`/`HispeedDown`(및 레인 커버/리프트 조작)을 처리하는 분기 진입부.

```rust
    if shared.course_run.as_ref().is_some_and(|run| !CourseOverrides::of(&run.course).accepts_speed_input()) {
        return;
    }
```

매 입력마다 `CourseOverrides::of` 를 다시 만들기 싫으면 `start_course` 에서 만든 것을
`AppShared` 에 함께 들고 있어도 된다(`course_overrides: Option<CourseOverrides>`).

**근거**: 레퍼런스는 `NO_SPEED` 코스에서 조작 자체를 끈다(`BMSPlayer.java:419-423`
`control.setEnableControl(false)`). 값 자체는 `CourseOverrides::apply_to` 가 이미 1.0/0 으로
고정했다.

### 7.2 코스 중 Esc

**위치**: 플레이 화면의 Esc 처리(`PlayState` 의 이탈 분기), 기존 `esc_confirms_quit` 규칙 뒤.

```rust
        if shared.course_run.is_some() {
            return crate::end_course(shared);
        }
```

**근거**: 스펙 §5.4 "코스 전체를 중단하고 Select 로 돌아간다(기록 없음)". 기존 2회 누르기 규칙은
그대로 두고, 확정된 뒤의 목적지만 바뀐다.

## 8. 설정·문서

- 코스 디렉터리는 `courses_dir(config_dir())` = `~/.config/rbms/courses`. 설정 행은 필요 없다.
  없으면 `load_dir` 이 빈 목록을 돌려준다.
- `docs/reference/architecture.md` 의 크레이트 표에 `rbms-course` 행을 추가한다.
- §1.1·§5.2 중 미적용으로 남는 것이 있으면 `docs/acknowledge/reference-divergences.md` 에 등재한다.

## 9. 스펙 대비 이탈·보완 기록 (G3 가 내린 판단)

| # | 항목 | 스펙 | G3 구현 | 근거 |
|---|---|---|---|---|
| 1 | `CourseRun.clear` 필드 추가 | §5.3 구조체에 없음 | `pub clear: u8` 추가 | §5.5 의 `CourseSubmission.clear` 는 필수 필드인데 스펙의 5개 필드로는 램프를 만들 수 없다. 코스는 게이지 하나로 이어지므로 마지막 스테이지의 램프이고, 실패하면 `CLEAR_FAILED` |
| 2 | `CourseTotals` 에 `fast`·`slow`·`combo_breaks` 추가 | §5.3 필드 목록에 없음 | 3개 추가 | `JudgeBreakdown` 의 비-default 필드(`fast`/`slow`/`combobreak`)를 채우기 위해서다. `combobreak` 는 모드별 combo 표가 필요해 `counts` 로 유도할 수 없으므로 호출부가 스테이지마다 넣는다. early/late 분할·`avgjudge` 는 serde default 가 있는 분석 값이라 합산하지 않는다 |
| 3 | 제약 정규화 후 순서 | §5.2 "선언 순서상 첫 원소만 남기고 제거" | 그룹당 첫 선언을 남기되 **그룹 번호 순으로 출력** | 레퍼런스가 5칸 버킷(`cdc[type]`)에 담고 압축하므로 결과가 그룹 순이다(`CourseData.java:118-124`). 어느 원소가 남는지는 스펙과 동일 |
| 4 | 트로피 유효성 필터 | §5.1 표에 없음 | `validate()` 가 `name` 빈 값·`missrate<=0`·`scorerate>=100` 트로피를 제거 | 레퍼런스가 `removeInvalidElements(trophy)` 를 돌린다(`CourseData.java:125`, `TrophyData.validate()`). 스펙 §5.1 표가 이 한 줄을 누락했다 |
| 5 | 트로피 `name` 빈 문자열 | — | 무효로 본다 | 레퍼런스는 `name != null` 만 본다. Rust 에는 null 이 없어 "필드 없음"이 `""` 로 오고, 이름 없는 트로피는 표시도 제출도 못 한다 |
| 6 | 실패한 코스의 트로피 | — | 게이트 없음(순수 rate 계산) | 레퍼런스는 실패한 코스의 스코어도 저장하고 `GradeBar.getTrophy()` 가 그 스코어로 판정한다. 실패 런은 분모가 코스 전체 노트라 scorerate 가 거의 항상 미달한다 |
| 7 | `Course::hash()` 산식 | §5.2 `sha256(name + '\n' + 차트 해시 순서 결합)` | 스펙 그대로 + 항목마다 `\n` 구분자 | 레퍼런스 `RankingDataCache.java:84-96` 은 **이름을 빼고 제약 토큰을 넣는다**. 두 산식이 다르므로 레퍼런스 코스 랭킹 캐시와는 호환되지 않는다. rbms IR 전용 id 로만 쓴다 |
| 8 | 저장 JSON 키 | §5.2 `charts`/`constraints`/`trophies` + 별칭 | 스펙 그대로 | 읽기는 `hash`/`song`/`constraint`/`trophy` 별칭으로 레퍼런스 파일을 받는다. **쓰기는 rbms 키**라 레퍼런스가 우리가 저장한 파일을 못 읽는다. 상호 저장이 필요해지면 `#[serde(rename = "hash", alias = "charts")]` 로 뒤집으면 되고 읽기 호환은 그대로 유지된다 |
| 9 | 제약 문자열 파싱 | §5.2 `from_token` | `from_token`(14 토큰) + `from_any`(레퍼런스 enum 이름도 허용) | 난이도표가 배포하는 코스는 토큰(`grade`)을, 레퍼런스가 직접 쓴 코스 파일은 enum 이름(`CLASS`)을 쓴다. 모르는 문자열은 코스를 떨어뜨리지 않고 그 항목만 버린다(`TableDataAccessor.java:198-199`) |
| 10 | `Stage::Course` 변형 | §9.2 소유권 표 | 만들지 않기를 권고 | §5.4 가 코스 목록을 곡선택의 탭으로 규정한다. §3.1 참조 |
