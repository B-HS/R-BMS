# G4 연습 모드 — G8 배선 지시

대상 모듈: `apps/rbms-player/src/practice.rs` (+ `practice/tests.rs`). 스펙 §6.
아래 항목은 **한 항목 = 한 패치**다. 줄 번호가 아니라 심볼명으로 앵커한다.

이 갈래가 소유한 파일 안에서 끝난 것: `PracticeProperty`(값·클램프 전량), `PracticePanel`(커서·행 편집·값 표시),
`PracticeSession`(플레이에 넘길 범위·게이지·판정폭·TOTAL·옵션), `PracticePhase`(패널↔슬라이스 상태 기계),
`PracticeBook`(`practice.ron` 차트별 영속), `practice_block_reason`(결정 12 제외 플래그).
아래 8건은 전부 **호출부**이며 G8 소유 파일에서만 가능하다.

---

## 1. `ir_submission_block_reason` 에 practice 사유 추가

- **파일**: `apps/rbms-player/src/lib.rs`
- **위치**: 함수 `ir_submission_block_reason` 시그니처와 본문 전체를 교체. 바로 아래 `updates_score` 도 함께.
- **코드**:

```rust
fn ir_submission_block_reason(autoplay: bool, replay: bool, custom_judge: bool, scratch_auto: bool, practice: bool) -> Option<&'static str> {
    if autoplay {
        return Some("autoplay");
    }
    if replay {
        return Some("replay playback");
    }
    if custom_judge {
        return Some("judge window widened");
    }
    if scratch_auto {
        return Some("scratch assist");
    }
    crate::practice::practice_block_reason(practice)
}

fn updates_score(autoplay: bool, replay: bool, custom_judge: bool, scratch_auto: bool, practice: bool) -> bool {
    ir_submission_block_reason(autoplay, replay, custom_judge, scratch_auto, practice).is_none()
}
```

- **호출부**: 두 함수의 기존 호출 전부에 마지막 인자를 추가한다. 연습 중이 아니면 `false`,
  연습 슬라이스의 결과 경로에서는 `true`. 현재 호출부는 `apps/rbms-player/src/app_result.rs` 와
  `apps/rbms-player/src/main_tests.rs` 다(`rg 'ir_submission_block_reason|updates_score\('` 로 전수 확인).
- **근거**: 스펙 §6.2 "기록·IR", 결정 12. 이 인자가 없으면 연습 플레이가 EX/램프/BP 를 갱신하고 IR 에 올라간다.
  `practice_block_reason` 이 `Option` 을 그대로 돌려주므로 기존 체인의 마지막에 그대로 이어 붙는다.

## 2. `Stage` / `StageId` 에 Practice 추가

- **파일**: `apps/rbms-player/src/stage/mod.rs`
- **위치**: `enum Stage` 의 `Result(ResultState)` 다음 줄, `enum StageId` 의 `Result` 다음 줄, `StageId::label` 의 `StageId::Result` 팔 다음 줄.
- **코드**:

```rust
    Practice(Box<PracticeState>),
```

```rust
    Practice,
```

```rust
            StageId::Practice => "Practice",
```

- **덧붙임**: `PracticeState` 는 G8 가 만드는 `apps/rbms-player/src/stage/practice.rs` 의 화면 상태다.
  `crate::practice::PracticePanel` 을 필드로 들고 `StageHandler` 를 구현한다 — 이 갈래는 화면을 그리지 않는다.
  `PracticePanel::move_cursor(down)` / `adjust(inc, turbo, analog)` / `focused()` / `value_text(element)` /
  `PracticeElement::ALL` 과 `PracticeElement::label()` 만으로 패널 한 장이 그려진다.
  `turbo` 는 Shift 등 가속 키, `analog` 는 G5 의 아날로그 컨트롤에서 온 입력일 때만 `true` 다(키보드는 항상 `false`).
- **근거**: 스펙 §6.2 "진입". `Stage` 에 변형이 없으면 패널로 갈 곳이 없다.

## 3. 곡선택에서 F2 로 진입

- **파일**: `apps/rbms-player/src/stage/select/mod.rs`
- **위치**: `impl StageHandler for SelectState` 의 `handle_key` 안, 루트 `match key.code` 의
  `KeyCode::F2 => self.toggle_filter_panel(),` 바로 다음 줄.
- **코드**:

```rust
            KeyCode::F4 => return self.start_practice(ctx.shared),
```

- **키 선택 근거**: 현재 이 트리가 쓰는 기능키는 F1·F2·F3 뿐이다(`rg 'KeyCode::F[0-9]+' apps/rbms-player/src`).
  F4 가 비어 있어 기존 바인딩과 충돌하지 않는다. 스펙 §6.2 는 "별도 키(예: F2)"라고만 정했으므로 F2 는 이미
  필터 패널이 쓰는 이 트리에서는 쓸 수 없다.

- **덧붙임**: `start_practice` 는 Enter 의 차트 로드 경로를 그대로 타되 "로드 끝나면 PLAY 대신 PRACTICE 패널"
  플래그를 세운다(§4 참고). 패널을 열려면 파싱된 `Model` 이 필요하므로 **엔트리만으로 열지 않는다**.
- **근거**: 스펙 §6.2 "진입". `PracticePanel::new` 가 `last_timeline_ms` 와 차트 TOTAL 을 요구한다.

## 4. 차트 로드 후 패널 열기

- **파일**: `apps/rbms-player/src/app_play.rs`
- **위치**: `load_chart` 안, 셔플 적용과 `SessionOptions` 조립 사이. 로드 요청에 `practice: bool` 을 실어 보낸 경우에만 탄다.
- **코드**:

```rust
        if self.practice_requested {
            let last_ms = crate::practice::last_timeline_ms(&model);
            let saved = self.practice_book.get(&model.md5);
            let panel = crate::practice::PracticePanel::new(model.md5.clone(), model.mode, last_ms, saved, model.meta.total);
            self.practice_model = Some(model);
            return Some(LoadedChart::practice(panel));
        }
```

- **덧붙임**: `practice_model` 은 트림 전 원본 모델이다. 슬라이스를 시작할 때마다 §5 의 트림을 원본에서 다시 하므로
  패널로 돌아와 범위를 바꿔도 앞선 트림이 누적되지 않는다.
- **근거**: 스펙 §6.2. `PracticePanel::new` 는 저장값을 읽고 `sanitise` 로 차트 경계 안으로 끌어온 뒤,
  TOTAL 이 `0`(미설정)이면 차트 자신의 TOTAL 을 넣는다 — 레퍼런스 `PracticeConfiguration.java:55-71` 과 같은 순서다.

## 5. 슬라이스 시작 — 범위·게이지·판정폭·TOTAL·옵션 적용

- **파일**: `apps/rbms-player/src/app_play.rs`
- **위치**: 연습 패널에서 시작 키를 받았을 때 도는 새 헬퍼 `start_practice_slice`. `PlaySession::new` 호출 직전.
- **코드**:

```rust
    pub(crate) fn start_practice_slice(&mut self, panel: &mut crate::practice::PracticePanel) -> Option<LoadedChart> {
        let practice = panel.start();
        let mut model = self.practice_model.clone()?;
        model.timelines.retain(|tl| tl.time_us >= practice.start_us && tl.time_us <= practice.end_us);
        if let Some(total) = practice.total {
            model.meta.total = total;
        }
        rbms_chart::shuffle::apply(&mut model, practice.option, seed);
        let options = SessionOptions {
            gauge: practice_gauge_kind(practice.gauge),
            judge_rate_percent: practice.judge_rate_percent,
            ..SessionOptions::default()
        };
        let mut session = PlaySession::new(model, options);
        let mut judge_setup = self.judge_setup();
        judge_setup.gauge_set = Some(practice.gauge_set);
        judge_setup.judge_rate_key = [practice.judge_rate_percent; JUDGE_WIDTH_TIER_COUNT];
        judge_setup.judge_rate_scratch = [practice.judge_rate_percent; JUDGE_WIDTH_TIER_COUNT];
        session.set_judge_setup(judge_setup);
        self.practice_session = Some(practice);
        Some(LoadedChart::play(session))
    }
```

- **덧붙임 1 (게이지 인덱스)**: `SessionOptions.gauge` 는 `GaugeKind` 6종뿐이라 코스 게이지 3종(GRADE/EX GRADE/
  EXHARD GRADE)을 직접 받지 못한다. `PracticeSession.gauge` 는 `GaugeIndex` 9종이므로 `ir_replay.rs` 의
  `gauge_kind_from_ir` 과 **같은 규약**(class→NORMAL, EX 변형→EX-HARD)으로 접는다. `ir_gauge` 는 `GaugeKind` 를
  받으므로 그 경로로는 접을 수 없다 — `app_play.rs` 에 아래 헬퍼를 둔다.

```rust
    fn practice_gauge_kind(index: rbms_judge::gauge::GaugeIndex) -> GaugeKind {
        match index.kind() {
            Some(kind) => kind,
            None => match index {
                rbms_judge::gauge::GaugeIndex::Class => GaugeKind::Normal,
                _ => GaugeKind::ExHard,
            },
        }
    }
```

  `JUDGE_WIDTH_TIER_COUNT` 는 `rbms_config` 가 이미 재노출하는 상수다(`judge_setup.rs` 가 쓰는 것과 같은 것).
  `spread_uniform_judge_rate` 는 `JudgeSetup` 이 아니라 `rbms_config` 의 판정 설정에 있는 메서드이므로 여기서는
  두 배열에 직접 넣는다.
- **덧붙임 2 (범위 재생)**: 타임라인 트림이 `crates/rbms-play` 를 건드리지 않고 범위 재생을 얻는 유일한 길이다
  (`PlaySession::seek` 은 리플레이 런에서만 동작하고, 시작 시각으로 클럭만 밀면 `update_judge` 가 앞 구간 노트를
  전부 미스로 쓸어버린다). **분기(divergence)**: 게이지 TOTAL 이 차트 전체가 아니라 슬라이스의 노트 수에 걸린다.
  레퍼런스는 전체 모델을 유지한 채 탐색한다. `docs/acknowledge/reference-divergences.md` 에 적어 둘 것.
- **근거**: 스펙 §6.2 "범위 재생".

## 6. 게이지 락 · 슬라이스 종료

- **파일**: `apps/rbms-player/src/stage/play/mod.rs`
- **위치**: 함수 `run_is_over`.
- **코드**:

```rust
    fn run_is_over(&self, song_us: i64) -> bool {
        if let Some(practice) = &self.practice {
            return practice.is_past_end(song_us);
        }
        self.session.is_finished(song_us) || (!self.session.analysis_enabled() && self.session.is_failed())
    }
```

- **덧붙임**: `self.practice: Option<PracticeSession>` 를 `PlayState` 에 추가한다. 이 한 분기가 **게이지 락 그 자체**다
  — `is_failed()` 를 보지 않으므로 게이지가 비어도 슬라이스가 끊기지 않고, `is_finished` 대신 `is_past_end` 를 보므로
  마지막 노트가 아니라 END TIME 에서 끝난다. `rbms-play` 쪽 `failed` 는 래치만 할 뿐 판정을 멈추지 않으므로
  (`crates/rbms-play/src/lib.rs` `run_gauge_auto_shift`) 이 분기만으로 충분하다.
  `PracticeSession.gauge_locked` 는 항상 `true` 이며, 이 분기가 그 계약을 지킨다는 표식이다.
- **근거**: 스펙 §6.2 "게이지 락", §11 행 9.

## 7. 슬라이스 종료 후 패널 복귀 + 저장

- **파일**: `apps/rbms-player/src/stage/play/mod.rs` (종료 전이) 및 `apps/rbms-player/src/stage/practice.rs`
- **위치**: 연습 런이 끝났을 때 `Transition::To(Stage::Result(..))` 로 가던 자리.
- **코드**:

```rust
        if self.practice.is_some() {
            return Transition::Back;
        }
```

```rust
    fn on_exit(&mut self, ctx: &mut FrameCtx<'_>) {
        self.panel.finish();
        ctx.shared.practice_book.put(self.panel.chart_key(), self.panel.property.clone());
        ctx.shared.practice_book.save(&crate::practice::practice_path(&ctx.shared.settings_path));
    }
```

- **덧붙임**: `AppShared` 에 `practice_book: crate::practice::PracticeBook` 를 두고 기동 시
  `PracticeBook::load(&practice_path(&settings_path))` 로 채운다(즐겨찾기 `Favorites` 와 같은 자리·같은 수명).
  `PracticePanel::finish()` 는 값을 건드리지 않고 상태만 패널로 돌린다 — 스펙의 "값이 유지된다" 가 이것이다.
- **근거**: 스펙 §6.2 "진입" 마지막 문장.

## 8. 결과 화면·리플레이 제외 표시

- **파일**: `apps/rbms-player/src/app_result.rs`
- **위치**: `saves_replay` / 점수 기록 분기.
- **코드**: §1 의 새 인자를 `true` 로 넘기고, 그 결과가 `Some("practice")` 이면 기록·제출·리플레이 저장을 모두 건너뛴다.
- **근거**: 결정 12. 연습은 플레이어가 고른 게이지로 한 조각만 되풀이하는 것이라 점수도 리플레이도 남기지 않는다.

---

## Phase G 소유권 밖 — 남는 한 가지

**GAUGE VALUE(시작 게이지량)는 이번 Phase 안에서 적용할 수 없다.** `PlaySession` 은 게이지를 가변으로 노출하지 않고
(`judge()` 는 `&JudgeEngine`), `SessionOptions`·`JudgeSetup` 어디에도 시작값 필드가 없다. `crates/rbms-play/**` 는
스펙 §9.3 상 **어느 G 갈래도 소유하지 않으므로** G8 도 고칠 수 없다.

- 패널은 값을 편집·저장하고 `PracticeSession.start_gauge` 로 넘기지만, 배선 시점에는 **읽히지 않는다.**
- 필요한 추가(Phase G 밖): `PlaySession::set_gauge_value(f32)` 또는 `SessionOptions.start_gauge: Option<f32>`.
  `rbms_judge::gauge::GrooveGauge::add_value` 가 이미 공개돼 있으므로 얇은 위임 한 겹이면 된다.
- 그때까지 패널의 GAUGE VALUE 행은 **미적용 배지**를 달아 표시한다(스펙 §12 R7 의 코스 제약과 같은 처리).

**FREQUENCY 도 같은 성격이나 이쪽은 스펙이 이미 결정해 뒀다**: `PracticeSession::freq_percent()` 가 항상 `100` 을
돌려준다(믹서가 리샘플링을 하지 않는다 — `crates/rbms-audio/src/mixer.rs`). 스펙 §6.2·§13-9 대로 Phase B 이후 재확인.
