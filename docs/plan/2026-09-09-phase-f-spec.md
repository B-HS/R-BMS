# Phase F 상세 설계 — UX·기능 고도화

> 상위 계획: `docs/plan/2026-09-09-enhancement-plan.md` §2 Phase F. 결정 근거: `docs/acknowledge/2026-09-09-enhancement-decisions.md`(결정 3·4·6·12). 발산 대장: `docs/acknowledge/reference-divergences.md`.
> 대상 커밋 `c6f0885`(dev) 기준. 레퍼런스 구현은 자바 원본을 파일명만으로 인용한다(경로·제품명 미기재).
> **선행 의존**: Phase C(설정 descriptor 테이블, `Stage` 데이터 enum, `rbms-config`) · Phase B(단일 `AudioEngine`, 볼륨 3분리) · Phase I(NETWORK 탭·IR 클라이언트). Phase I 가 동시 진행 중이므로 `crates/rbms-ir`·`apps/rbms-player` 의 IR 관련 라인 번호는 **Phase I 이후 재확인** 대상이며, 이 문서는 그쪽을 함수명으로만 참조한다.

---

## 0. 현재 상태 요약 (앵커)

| 축 | 현 구현 | 앵커 |
|---|---|---|
| 시작 전 옵션 | 전체화면 Settings 화면 1종, 24행을 정수 인덱스로 관리 | `main.rs:700-715`(`SETTING_KEYCONFIG=11`·`SETTING_TABS` 6탭 = PLAY/GAUGE/JUDGE/DISPLAY/INPUT/NETWORK), `app_select.rs:935-1013`(`setting_line`:935 / `adjust_setting`:966) |
| Select 키맵 | Esc·←(뒤로) `/`(검색) F3(정렬) Tab(설정) O(폴더) T(표) R(기록) ↑↓ Enter/→ | `main.rs:1126-1152` |
| Play 키맵 | Esc(즉시 이탈) + `ControlAction` 6종(사용자 바인딩) + 분석 모드 Space·`=`·`-`·PageUp·PageDown | `main.rs:1241-1246`, `keyconfig.rs:83-112`, `app_input.rs:296-317` |
| Result 키맵 | Esc·Enter 만(둘 다 Select 복귀) | `main.rs:1218-1221` |
| 하이스피드 | `0.25` 스텝, `clamp(0.5, 10.0)`, CONSTANT/FLOATING 2택(실제로는 CONSTANT/OFF) | `app_input.rs:92-93`, `app_select.rs:969`(`adjust_setting` global 1), `main.rs:196-206`(`green_number_for`) |
| 레인커버/LIFT | 각 `0.05` 스텝, `clamp(0.0, 0.9)`, HIDDEN+ 없음 | `app_select.rs:979`(lift, global 5)·`:980`(cover, global 6) |
| 정렬 | 5종(DEFAULT/TITLE/ARTIST/LEVEL/CLEAR), F3 순환, 필터·즐겨찾기 없음 | `main.rs:651-657`(enum)·`659-673`(impl, `ALL: [SortMode; 5]`), `app_select.rs:48-53`(`cycle_sort`) |
| 결과 화면 | 판정 6행 + EX/MAX콤보/FAST·SLOW + DJ 랭크바 + vs BEST/PREV 델타. 게이지 추이·판정 분포·타이밍 히스토그램·타깃 없음 | `crates/rbms-render/src/result.rs:8-26,95-208` |
| 라이브 그래프 | 없음(HUD 는 게이지 바·콤보·EX 만) | `crates/rbms-render/src/hud.rs` |
| 토스트/에러 알림 | **없음**(전부 `eprintln`, 앱 크레이트 11파일 43건 — §F4-1 표) | 계획 §1.6 U1 |
| 로딩 | `Loading::{Song,Scan}` 2종, 단계 표시 없음 | `main.rs:598-610`(enum 은 602), `app_play.rs:490-496`, Esc 취소 `main.rs:1223-1240` |
| 미리듣기 | 시간 디바운스 333ms 는 이미 적용됨(계획 §1.6 U5 의 "프레임 수 기준"은 해소됨), 페이드·볼륨 없음 | `main.rs:59-62`(`PREVIEW_DEBOUNCE`/`PREVIEW_GAIN`/`PREVIEW_LOOP_TAIL_US`) |
| 리사이즈 | 논리 1280×720 고정 + 스트레치(레터박스 없음) | 계획 §1.3 K6 |

---

## 1. 레퍼런스 구현 패리티 데이터 (구현 시 그대로 박을 값)

### 1.1 타깃 / PACEMAKER (`TargetProperty.java`)

- 해석 순서: `Static → Rival → InternetRanking → "RANK_NEXT" → 폴백 "MAX"` (`TargetProperty.java:59-72`).
- 고정 레이트 타깃(`TargetProperty.java:117-141`), 전부 `100 * n / 27`:

| id | 표시명 | rate |
|---|---|---|
| `RATE_A-` | RANK A- | 17/27 |
| `RATE_A` | RANK A | 18/27 |
| `RATE_A+` | RANK A+ | 19/27 |
| `RATE_AA-` | RANK AA- | 20/27 |
| `RATE_AA` | RANK AA | 21/27 |
| `RATE_AA+` | RANK AA+ | 22/27 |
| `RATE_AAA-` | RANK AAA- | 23/27 |
| `RATE_AAA` | RANK AAA | 24/27 |
| `RATE_AAA+` | RANK AAA+ | 25/27 |
| `RATE_MAX-` | RANK MAX- | 26/27 |
| `MAX` | MAX | 100% |

- 임의 레이트: `RATE_<float>`(0~100) → 이름 `SCORE RATE <f>%` (`TargetProperty.java:142-152`).
- 타깃 EX 산출(`StaticTargetProperty.getTarget`, `TargetProperty.java:104-112`):
  `rivalscore = ceil(totalNotes * 2 * rate / 100)`, `epg = rivalscore / 2`, `egr = rivalscore % 2`. 즉 **EX = rivalscore** 이고 판정 분해까지 채운다. rbms 는 EX 정수만 필요하므로 `ceil(total_notes as f64 * 2.0 * rate / 100.0) as u32` 로 동일 값을 얻는다.
- 라이벌 타깃(`RivalTargetProperty`, `TargetProperty.java:157-230`): `RIVAL_<n>` 3종 축 `INDEX`(지정 라이벌) / `RANK`(EX 내림차순 n위, `index>0` 이면 "RIVAL RANK n", 0 이면 "RIVAL TOP") / `NEXT`(자기 위 n번째). 정렬은 `s2.exscore - s1.exscore`.
- `RANK_NEXT` = `NextRankTargetProperty`(`TargetProperty.java:301-331`). **본문 통독 완료**, 알고리즘은 다음과 같다(추론 아님):
  ```
  now  = 로컬 베스트 EX (없으면 0)
  max  = totalNotes * 2
  for i in 15..=26:           // 27분위 중 15/27 부터
      t = ceil(max * i / 27)
      if now < t { target = t; break }
  else target = max           // 26/27 도 넘었으면 MAX
  ```
  표시명은 `"NEXT RANK"`. rbms 이식: `fn next_rank_ex(now_ex: u32, total_notes: u32) -> u32`, 반환은 `u32`(항상 존재).
- **27분위 = DJ 랭크 밴드와 동일 축**(레퍼런스가 `i/27` 로 다음 랭크를 계산하는 것이 근거). rbms 현 구현의 정확한 형태는 다음과 같다 — 27분위 작업(F3-2)은 이 셋을 함께 바꾼다:
  - `RANK_BANDS: [(&str, Color); 8]`(`result.rs:78`) — F/E/D/C/B/A/AA/AAA **8밴드**.
  - `RANK_BOUNDS: [f32; 9]`(`result.rs:91`) — `0, 2/9 … 8/9, 1.0`. 즉 "9분위"가 아니라 **9개 경계값·8밴드**(스펙 초판 표현 정정).
  - `dj_rank`(`result.rs:95-104`)는 `RANK_BOUNDS[1..8]` 를 `rposition` 으로 훑어 밴드 인덱스를 낸다. `draw_rank_bar`(`result.rs:106-118`)는 `for b in 0..8` 로 **밴드 수가 하드코딩**돼 있다.

### 1.2 하이스피드 / 레인커버 (`PlayConfig.java`, `LaneRenderer.java`)

| 항목 | 값 | 앵커 |
|---|---|---|
| `hispeed` 기본 / 범위 | 1.0 / `HISPEED_MIN=0.01f` ~ `HISPEED_MAX=20f` | `PlayConfig.java:16`, 상수 `:18-19`, 클램프 `:251` |
| `hispeedmargin`(스텝) / 범위 | 0.25 / `HISPEEDMARGIN_MIN=0f` ~ `HISPEEDMARGIN_MAX=10f` | `PlayConfig.java:54`, 상수 `:56-57`, 클램프 `:254` |
| `fixhispeed` | `OFF=0 / STARTBPM=1 / MAXBPM=2 / MAINBPM=3 / MINBPM=4`, 기본 `MAINBPM` | `PlayConfig.java:43-49` |
| `lanecover` 기본 | 0.2 | `PlayConfig.java:62` |
| `enablelanecover` | true | `PlayConfig.java:66` |
| `lift` 기본 / enable | 0.1 / false | `PlayConfig.java:70,74` |
| `hidden`(HID+) 기본 / enable | 0.1 / false | `PlayConfig.java:78,82` |
| 레인커버 미세 조정 low/high | 0.001 / 0.01 | `PlayConfig.java:87,91` |
| 레인커버 전환 애니메이션 | 500 ms | `PlayConfig.java:95` |
| `hispeedautoadjust` | false | `PlayConfig.java:99` |

- 그린넘버 고정(floating hi-speed) 공식 — `LaneRenderer.java:206-210`:
  `hispeed = (2400 / (targetbpm / 100) / duration) * (1 - (enablelanecover ? lanecover : 0))`.
  `targetbpm` 이 `fixhispeed` 모드에 따라 START/MAX/MAIN/MIN BPM 으로 바뀐다. rbms 의 `2000.0` 상수 계열(Phase A 통합분)과 스케일이 다르므로 **rbms 의 그린넘버 정의(`rbms_chart::scroll::green_number`)에 맞춰 역산 공식을 유도**하고, 레퍼런스 상수 2400 을 그대로 복사하지 않는다(→ §6 리스크 R3).
- 하이스피드 증감 — `LaneRenderer.java:236-245`: 스텝 = `fixhispeed != OFF ? basehispeed * hispeedmargin : hispeedmargin`, 적용 조건은 **`0 < new < 20` 배타 범위**. 즉 상한 20 은 도달 불가. 이 배타 검사는 위 `HISPEED_MIN/MAX` 상수(`PlayConfig.java:18-19`)와 정합한다.
- **발산(의도적)**: rbms 의 `hispeed_step` 범위는 `0.01 ~ 1.0`(§3 표)으로, 레퍼런스 `HISPEEDMARGIN 0~10` 보다 좁다. 근거 = 스텝 10 은 상한 20 대비 무의미하고 `0` 스텝은 증감 불능 상태를 만든다. `reference-divergences.md` 에 "hispeed step 범위 축소" 1행으로 등록한다.
- `setLanecover`(`LaneRenderer.java:212-215`)는 0~1 클램프 후 **`resetHispeed(basebpm)` 를 재호출**한다 → 레인커버를 바꾸면 그린넘버가 유지되도록 hispeed 가 자동 재계산된다(IIDX LANE COVER 연동과 동일 동작).
- `lift`(`LaneRenderer.java:193-200`)·`hidden`(`LaneRenderer.java:228-233`)은 0~1 클램프만.

### 1.3 정렬 (`BarSorter.java`)

12종, 동률은 전부 `TITLE` 로 폴백:
`TITLE`(:19) `ARTIST`(:47) `BPM`(:65) `LENGTH`(:83) `LEVEL`(:101) `CLEAR`(:126) `SCORE`(:144) `MISSCOUNT`(:167) `DURATION`(:185) `LASTUPDATE`(:206) `RIVALCOMPARE_CLEAR`(:221) `RIVALCOMPARE_SCORE`(:236).

- **노출 집합은 2단이다**: `defaultSorter`(`BarSorter.java:260`) = `TITLE, ARTIST, BPM, LENGTH, LEVEL, CLEAR, SCORE, MISSCOUNT` **8종**, `allSorter`(`:262`) = `values()` **12종**. 즉 기본 순환은 8종이고 나머지 4종(`DURATION`/`LASTUPDATE`/`RIVALCOMPARE_*`)은 확장 집합이다.
- **F1-1 채택**: rbms 는 `SortMode::DEFAULT_CYCLE`(레퍼런스 8종 + rbms 고유 `Default`= 폴더 원순서 → **9종**)을 F3 순환 대상으로 하고, `DURATION`/`LASTUPDATE` 는 설정(`sort` descriptor)에서만 선택 가능, `RIVALCOMPARE_*` 2종은 Phase I 라이벌 데이터 도착 전까지 목록에서 숨긴다.

### 1.4 FLIP / BATTLE / SYNC-RAN / LEGACY NOTE

- 스코어에 기록되는 옵션 문자 enum 에 `H_RANDOM('p')`·`BATTLE('B')`·`BATTLE_ASSIST('b')` 가 존재(`ScoreData.java:610-625`).
- `doubleoption` 0~3 의 의미는 소비처 `BMSPlayer.java:238-297` 통독으로 **확정**했다(미확인 해소):

| 값 | 의미 | 근거 | 어시스트 |
|---|---|---|---|
| 0 | OFF | `BMSPlayer.java:256`(비-SP 에서 강제 0) | — |
| 1 | FLIP (`PlayerFlipModifier`) | `:294-295`, `mode.player == 2`(DP 보면)일 때만 적용 | **없음**(assist/score 미변경) |
| 2 | BATTLE (`PlayerBattleModifier`, SP→DP 모드 승격 5K→10K·7K→14K·24K→24K_DOUBLE) | `:238-254` | `assist=max(assist,1)`, `score=false` |
| 3 | BATTLE + 스크래치 오토(`AutoplayModifier(mode.scratchKey)`) | `:247-250` | 2 와 동일(L-ASSIST) |

  즉 **BATTLE 계열(2·3)만 어시스트이고 FLIP(1)은 어시스트가 아니다**. `SYNC-RANDOM` 은 `doubleoption` 축이 아니라 `randomoption` 축(`PatternModifier`)이므로 rbms 에서도 `LaneOption` 이 아니라 `NoteOption` 쪽 확장으로 다룬다(F2-4 변경 참조).
- FLIP 의 **기록 해시(어느 보면으로 저장할지)** 는 레퍼런스 자신이 미정 상태다(`PlayDataAccessor.java:243` "FLIP の扱いは？" TODO). 다만 **어시스트 여부는 미정이 아니다** — 위 표대로 FLIP 은 assist 를 올리지 않는다. 따라서 rbms 의 "FLIP = 비어시스트" 정책은 레퍼런스 동작과 일치하며, 남는 미확정은 기록 해시 키뿐이다(→ §7).
- LEGACY NOTE 는 IIDX 고유 표시 옵션이며 레퍼런스 구현에 대응물을 찾지 못했다(→ §7 미확인 2).

---

## 2. 작업 항목 (독립 출하 단위)

크기: S<1일 · M 1~3일 · L≈1주.

### F0 — 선행 배관 (직렬, 병렬 착수 전 단독 커밋) · M

**현 상태.** `App` 는 93필드 god object(`main.rs:717-...`), `frame()` 이 Play/Settings/Loading/Result 를 모두 그린다(`app_play.rs`), 키 라우팅이 `window_event` 한 곳에 몰려 있다(`main.rs:1061-1250`). 이 상태로 4갈래를 병렬 착수하면 전 갈래가 `main.rs`·`app_play.rs` 를 동시에 고쳐 충돌한다.

**변경.** Phase F 전체가 필요로 하는 **배관만** 한 번에 넣는다(기능은 스텁). 이 목록은 §5 소유권 파티션을 성립시키기 위한 **필수 전제**다 — 여기 빠진 항목이 있으면 두 갈래가 같은 파일을 건드리게 된다.

1. **파일 신설** — `apps/rbms-player/src/{app_result,app_options,toast,notify,target,favorites,dialog,settings_ui}.rs`, `crates/rbms-render/src/toast.rs`. 전부 F0 이 만들고(빈 스텁 포함) `mod` 선언·재export 를 `main.rs` / `crates/rbms-render/src/lib.rs` 에 **최종 형태로** 등록한다.
2. **`enter_result()` 이동** — `app_play.rs` → `app_result.rs`(174줄, 계획 §1.4 C6). Result 렌더 분기도 함께. 동작 무변경.
3. **`rebuild_select_items` / `arrange_songs` 이동** — 현재 `app_input.rs:9-79`(F2 소유 예정)에 있으나 F1-2 필터가 이 함수 앞단을 고쳐야 한다 → `app_select.rs`(F1 소유)로 이동.
4. **표 관리 함수 이동** — `app_select.rs` 의 `open_tables`(:678) `tables_row_count`(:755) `add_table_source`(:759) `add_table_file`(:767) `remove_table_source`(:779) `tables_input`(:795) 및 `rescan_all_folders`(:715) 안의 `fetch_and_match` 호출(:723) → `tables.rs`(F4 소유). F4-2 의 백그라운드화가 F1 파일을 침범하지 않게 한다.
5. **rfd 다이얼로그 3종 이동** → `dialog.rs`(F4 소유): `pick_font`(`app_input.rs:142`), `add_folder_dialog`(`app_select.rs:695`), `add_table_file` 내 `FileDialog`(`app_select.rs:768`). 호출부는 `self.dialog_pick_*()` 로 치환.
6. **설정 UI 이동 + 신규 행 선등록** — `setting_line`(`app_select.rs:935`)·`adjust_setting`(`:966`)을 `settings_ui.rs`(F0 소유, 이후 어느 갈래도 수정하지 않음)로 옮기고, §3 표의 **모든 신규 행을 F0 에서 미리 등록**한다(값은 현재 필드를 그대로 표시, 미구현 기능은 비활성 표시). `SETTING_TABS`(`main.rs:707-715`)에 `SELECT` 탭을 추가한다. Phase C descriptor 가 오면 이 테이블만 교체된다.
7. **`main.rs` 의 갈래별 변경을 전부 선반영** — F0 이후 `main.rs` 를 여는 갈래는 없다:
   - `SortMode`(`main.rs:651-673`)를 12 variant + `DEFAULT_CYCLE`(9종) + `label()`/`next()`/`prev()` 로 확장. 신규 variant 의 비교자는 `app_select.rs` 의 `fn sort_cmp` 에 위임(F1 이 채운다).
   - `Loading`(`main.rs:602-610`)을 `Scan / Song(usize) / Parse / Keysound{done,total} / Bga{done,total} / TableFetch{name} / Save` 로 확장. 신규 variant 는 F0 시점엔 생성되지 않는다.
   - Select 키 분기(`main.rs:1126-1152`)에 `Shift+F3`·`F2`·`F1`·`KeyF` 및 `App::options_key(code, pressed) -> bool` 선처리(오버레이가 먼저 소비)를 추가하고, 각 핸들러는 갈래 파일의 빈 함수로 위임.
   - Result 키 분기(`main.rs:1218-1221`)에 `KeyR`/`KeyN` → `self.result_retry()` / `self.result_next()`(`app_result.rs` 빈 함수) 추가.
   - Play Esc 분기(`main.rs:1241-1250`)에 결정 6(길게 누르기 / 2회) 배선. **이것만이 F0 의 유일한 동작 변경**이며 §4 말미·통과 조건에 명시한다.
   - `PlayerConfig`(`main.rs:88-118`)·`Default`(`:120-152`)에 §3 신규 필드 일괄 추가(전 필드 `#[serde(default)]`) + `settings.rs` 왕복.
8. **`notify` 도입 + `eprintln` 전수 치환(동작 동일)** — `notify.rs` 에 프로세스 전역 싱크
   ```rust
   pub enum Level { Info, Warn, Error }
   pub fn notify(level: Level, msg: impl Into<String>);   // F0: eprintln 으로 그대로 출력 + 링버퍼(상한 64)에 적재
   pub fn drain(out: &mut Vec<(Level, String)>);          // F4-1 이 ToastQueue 로 옮길 때 사용
   ```
   앱 크레이트의 `eprintln!` **43건 전부**를 `notify(...)` 로 치환한다(파일별 건수는 §F4-1 표). 자유 함수(`scores.rs`·`folders.rs`·`keyconfig.rs`·`replay.rs`·`tables.rs`·`settings.rs`)도 전역 함수라 `self` 없이 호출된다. 출력이 동일하므로 **동작 무변경**이고, `eprintln` 잔존 0 grep 테스트는 이 시점에 이미 통과한다.
9. **골든 테스트 파일 분할** — `crates/rbms-render/tests/golden.rs`(258줄)를 공통 하네스 `golden.rs`(F0 소유, 이후 동결) + `golden_select.rs`(F1) · `golden_play.rs`(F2) · `golden_result.rs`(F3) · `golden_shell.rs`(F4)로 나눈다. 픽스처 디렉터리도 `tests/golden/{select,play,result,shell}/` 로 분리한다. 분할 시점의 픽셀은 전부 동일해야 한다.
10. **`crates/rbms-play/src/lib.rs` 계측 필드** — F3-3 이 쓸 `gauge_series`·`timing_hist`·`judge_dist` 필드와 갱신 훅을 F0 이 추가하고(값은 채워지지만 소비처 없음), 이후 F3 만 소비한다.

**테스트.** `cargo test --workspace` 무회귀(현 1,060 통과 기준). 분할된 골든 5파일 전부 픽셀 불변. `settings.ron` 왕복 테스트에 신규 필드 포함. `eprintln!` 잔존 0 grep 테스트 추가.

**통과 조건.** 골든 전량 불변 + 기존 키 동작 무변경. **유일한 예외는 항목 7 의 Play Esc(결정 6)** 이며, 이 1건은 별도 커밋으로 쪼개 리뷰한다.

---

### 갈래 F1 — 곡선택 UI

#### F1-1 정렬 12종 · S
- **현 상태**: 5종(`main.rs:651-657` enum / `659-673` impl), `SortMode::Level` 비교자 안에서 `parse` 수행(계획 §1.6 U7). enum 확장 자체는 **F0-7 이 완료**하므로 F1 은 `app_select.rs` 의 비교자만 채운다.
- **변경**: `app_select.rs` 에 `fn sort_cmp(&self, m: SortMode, a: usize, b: usize) -> Ordering` 를 두고 12종 비교자를 구현 — `Default/Title/Artist/Bpm/Length/Level/Clear/Score/MissCount/Duration/LastUpdate/RivalClear/RivalScore`(§1.3 대응). F3 순환 대상은 §1.3 의 `DEFAULT_CYCLE` 9종, `Duration`/`LastUpdate` 는 설정에서만, `RivalCompare*` 2종은 Phase I 도착 전 숨김. 동률 폴백은 전부 `Title`. 레벨은 `SongEntry` 에 사전 파싱한 `level_num: i32` 를 캐시해 비교자 내 parse 제거.
- **키**: F3 순환 유지 + `Shift+F3` 역순환(키 배선은 F0-7, 동작은 F1).
- **테스트**: 12종 각각에 대해 정렬 안정성(동률 → 타이틀 오름차순) 단위 테스트, `level_num` 파싱 실패 시 폴백.
- **크기**: S

#### F1-2 필터(난이도·모드·클리어) + 즐겨찾기 · M
- **현 상태**: 필터 개념 없음. 목록 조립은 `rebuild_select_items`/`arrange_songs`(현 `app_input.rs:9-79` → **F0-3 이 `app_select.rs` 로 이동**), 뷰 조립은 `build_select_view`(`app_select.rs:106-274`), 뷰 축은 `SelectView`(`main.rs:635-648`)·`App::select_view`(`main.rs:736`).
- **변경**: `SelectFilter { level: Option<RangeInclusive<i32>>, mode: Option<Mode>, clear: Option<ClearFilter>, favorite_only: bool }` 를 `rebuild_select_items` 앞단에 적용. 즐겨찾기는 md5 집합을 `favorites.ron`(신규, `write_atomic` 사용 — `main.rs:160` 헬퍼)에 영속.
- **키**: `F2` = 필터 패널 토글, `KeyF` = 포커스 곡 즐겨찾기 토글. Select 문자키 점유는 `O`(:1136)/`T`(:1137)/`R`(:1138) 뿐이라 충돌 없음. 배선은 F0-7.
- **테스트**: 필터 조합별 결과 집합, 즐겨찾기 영속 왕복, 필터 적용 후 `sel` 이 범위를 넘지 않음.
- **크기**: M

#### F1-3 검색 뷰 복원 + 결과 개수 표시 · S
- **현 상태**: `exit_search`(`app_select.rs:34-41`)가 `select_view` 를 복원하지 않아 `/` 진입 시 `start_search`(`app_select.rs:25-33`)가 강제한 `AllSongs` 뷰에 갇힌다.
- **변경**: `start_search` 가 이전 `select_view`·`sel` 을 저장, `exit_search` 가 복원. 검색 헤더에 `N hits` 표시(`crates/rbms-render/src/select.rs` 헤더 영역).
- **테스트**: 폴더 뷰 → 검색 → Esc → 원 폴더·원 커서 복귀.
- **크기**: S

#### F1-4 DJ LEVEL 표시 + BGA 썸네일 · S
- **현 상태**: 행에 클리어 램프만. `result.rs:95-104` 의 `dj_rank` 는 결과 화면 전용.
- **변경**: `dj_rank` 를 `crates/rbms-render/src/result.rs` 에서 공용 위치(`crates/rbms-render/src/lib.rs` 재export)로 노출하고 곡 행·상세 패널에 `AAA` 등 표기. 썸네일은 이미 있는 커버 디코드 경로 재사용(`refresh_focused_detail` 안의 `#STAGEFILE`→`#BANNER` 디코드, `app_select.rs:287-292`, `decode_bga_256`), 없으면 `#BACKBMP`.
- **테스트**: `dj_rank` 재export 후 result 골든 불변(`golden_result.rs`, F3 파일 — **F1 은 읽기만 하고 수정하지 않는다**; 재export 는 시그니처를 바꾸지 않으므로 갱신 불필요), 행 렌더 골든 신규 1장은 `golden_select.rs`(F1 소유)에 넣는다.
- **크기**: S

#### F1-5 미리듣기 페이드 · S (**Phase B 의존**)
- **현 상태**: 디바운스 333ms·`PREVIEW_GAIN` 고정(`main.rs:59-62`), 페이드 없음, 전용 엔진(계획 §1.2 A8). 프리뷰 구동부는 `update_preview`(`app_select.rs:300`)·`start_preview`(:396)·`start_autoplay_preview`(:467)·`stop_preview`(:556).
- **변경**: Phase B 의 단일 `AudioEngine` 위에서 진입 200ms 페이드인 / 이탈 150ms 페이드아웃, 볼륨은 Phase B 의 `bg` 볼륨 × `PREVIEW VOLUME` 설정.
- **테스트**: 믹서 램프 단위 테스트(첫/끝 프레임 진폭), 곡 이동 연타 시 이전 프리뷰가 즉시 잘리지 않고 페이드아웃되는지.
- **크기**: S. **Phase B 미완이면 F1 에서 제외**하고 B 뒤로 미룬다.

**F1 소유 파일**: `apps/rbms-player/src/app_select.rs` · `apps/rbms-player/src/favorites.rs` · `crates/rbms-render/src/select.rs` · `crates/rbms-render/tests/golden_select.rs`

---

### 갈래 F2 — 옵션 오버레이 / 플레이 옵션

#### F2-1 곡선택 옵션 오버레이(결정 3, 주경로) · M
- **현 상태**: 없음. F0 이 만든 `app_options.rs` 스텁.
- **변경**: 홀드형 오버레이. 표시 항목 = 곡마다 바뀌는 값만 — `RANDOM / GAUGE / HI-SPEED(+SPEED FIX) / LANE COVER / LIFT / HIDDEN+ / SCRATCH SIDE / SCRATCH AUTO / AUTOPLAY / TARGET`. 값 원천은 `PlayerConfig` 단일(설정 화면과 같은 필드) → **제출 옵션의 진실 출처가 갈리지 않는다**.
- **키**: 새 바인딩 `SelectAction::OptionsHold`(기본 `ShiftLeft`) — 누르는 동안 오버레이, ↑↓ 항목, ←→ 값, 릴리스 시 닫힘. 홀드가 어려운 환경을 위해 `F1` 토글 병행(현재 미사용). Select 의 기존 점유(`Esc ← / F3 Tab O T R ↑↓ Enter →`, `main.rs:1126-1152`)와 충돌 없음.
- **주의**: 오버레이가 열린 동안 Select 키 분기보다 **먼저** `App::options_key` 가 소비해야 한다(F0-2 진입점).
- **테스트**: 홀드 중 Select 키가 리스트를 움직이지 않음, 릴리스 후 값이 `PlayerConfig` 에 남고 설정 화면과 동일 표시, 오버레이 중 Enter 가 곡을 시작하지 않음.
- **크기**: M

#### F2-2 하이스피드 정밀화 + floating hi-speed · M
- **현 상태**: 스텝 0.25 고정, `clamp(0.5, 10.0)`(`app_input.rs:92-93` = `apply_control`, `app_select.rs:969` = `adjust_setting` global 1 → **F0-6 이후 `settings_ui.rs`**), `SPEED FIX` 라벨이 CONSTANT/FLOATING 2택인데 실제 의미는 CONSTANT/OFF(계획 §1.7).
- **변경**:
  - 범위 `0.01 ~ 20.0`(상수 `HISPEED_MIN/MAX`, `PlayConfig.java:18-19`; 증감 경로는 `0 < x < 20` 배타 검사, `LaneRenderer.java:243`), 스텝을 설정값 `hispeed_step`(기본 0.25, 범위 축소는 §1.2 발산 항목)으로.
  - `SPEED FIX` 를 5택 `OFF / START / MAX / MAIN / MIN`(`PlayConfig.java:43-49`)으로 교체하고, OFF 가 아닐 때 §1.2 공식으로 **그린넘버를 고정**(BPM 변화 구간에서 hispeed 를 재계산). 기존 CONSTANT 는 `OFF` 로 마이그레이션하지 말고 별도 `constant_speed` 를 유지 — 두 축은 의미가 다르다(CONSTANT = BPM 무시 등속, FIX = 그린넘버 고정).
  - `fixhispeed != OFF` 일 때 증감 스텝을 `base_hispeed * step` 으로(`LaneRenderer.java:238-240`).
- **테스트**: 5모드 × 대표 BPM 3종(단일/변속/급변속)에 대한 그린넘버 불변 테스트, 경계값 0.01/19.99, 마이그레이션(구 `constant_speed` 설정 로드).
- **크기**: M

#### F2-3 LANE COVER 미세 조정 · HIDDEN+ · 화이트넘버 · S
- **현 상태**: cover/lift 각 0.05 스텝·`clamp(0.0, 0.9)`(`app_select.rs:979`=lift·`:980`=cover → F0-6 이후 `settings_ui.rs`), HIDDEN+ 없음. 인게임 조정은 `apply_control`(`app_input.rs:90-106`).
- **변경**: 커버/리프트/HID+ 를 0.0~1.0, 스텝을 `lanecover_step_coarse`(기본 0.01)·`lanecover_step_fine`(기본 0.001)으로 분리(`PlayConfig.java:87,91`). 미세 조정은 조정 키 + 수정자(`Shift`) 조합. 커버 변경 시 `fixhispeed != OFF` 면 §1.2 대로 hispeed 재계산(`LaneRenderer.java:212-215`).
  화이트넘버(= 커버 하단까지의 노트 도달 시간)는 그린넘버와 함께 HUD 에 표기.

  **SUD+ / HID+ 동시 적용**(계획 항목, 초판 누락분). 레퍼런스는 `enablelanecover`(`PlayConfig.java:66`)·`enablelift`(`:74`)·`enablehidden`(`:82`)을 **서로 독립된 bool** 로 두고 값(`lanecover`/`lift`/`hidden`)과 분리한다. rbms 도 동일하게 값 3개 + 토글 3개를 따로 두어 **SUD+(cover) 와 HID+ 를 동시에 켤 수 있게** 한다(양쪽이 켜지면 가시 구간 = `[lift, 1-cover]` 에서 다시 `hidden` 만큼 하단이 가려진다). 토글 off 시 값은 보존하고 렌더/그린넘버 계산에서만 0 으로 취급한다 — §1.2 그린넘버 공식의 `enablelanecover ? lanecover : 0` 과 같은 규약. descriptor 행 `enable_cover`/`enable_lift`/`enable_hidden` 3개를 §3 표에 추가했다.
  가시 구간 계산식(rbms 정의):
  ```
  top    = if enable_cover  { cover  } else { 0.0 }
  bottom = if enable_lift   { lift   } else { 0.0 }
  hid    = if enable_hidden { hidden } else { 0.0 }
  visible = (bottom + hid) .. (1.0 - top)      // 빈 구간이면 노트 0장 표시(패닉 금지)
  ```
- **키**: 기존 `ControlAction::{HiSpeedUp,HiSpeedDown,CoverUp,CoverDown,LiftUp,LiftDown}`(enum `keyconfig.rs:83-90`) 유지 + `HiddenUp/HiddenDown` 2종 추가. `ALL` 은 `keyconfig.rs:93-100` 의 `[ControlAction; 6]` 상수이므로 `[ControlAction; 8]` 로 갱신하고 `label()`(`:103-112`)에 2행 추가.
- **테스트**: 스텝 2종 경계, HID+ 가 노트 가시 구간을 실제로 줄이는지(플레이필드 골든), 커버 변경 → hispeed 재계산 후 그린넘버 불변.
- **크기**: S

#### F2-4 FLIP / BATTLE / SYNC-RAN / LEGACY NOTE · M
- **현 상태**: `NoteOption`(`rbms-chart::shuffle`) 계열만. DP 축·표시 옵션 없음.
- **변경**: `NoteOption` 과 직교하는 `LaneOption { Off, Flip, Battle, BattleAutoScratch }` 신설 — §1.4 표의 `doubleoption` 0~3 과 **값 순서까지 1:1 대응**시킨다(레퍼런스 리플레이 호환에 유리). `SyncRandom`/`SymmetryRandom` 은 `doubleoption` 축이 아니므로 `LaneOption` 이 아니라 **`NoteOption` 확장**(좌우 레인에 같은 시드를 적용하는 변형)으로 구현한다.
  적용 조건도 레퍼런스와 맞춘다: `Flip` 은 보면이 이미 DP(`mode.player == 2`)일 때만(`BMSPlayer.java:294-295`), `Battle*` 은 SP 보면을 DP 로 승격시킨 뒤 적용하고(5K→10K·7K→14K, `:238-245`) DP 보면에는 적용하지 않는다(`:255`, 비대상이면 값을 0 으로 강제). `LEGACY NOTE`(LN 을 일반 노트로 표시)는 렌더 전용 플래그로 `SkinConfig` 가 아니라 `PlayerConfig` 에 둔다.
- **기록 정책**: 결정 12 의 어시스트 규약을 따른다 — BATTLE 계열(`doubleoption` 2·3)은 `assist` 플래그를 세워 램프를 강등하고 IR 제출을 막는다. 이는 **레퍼런스 동작과 일치**한다(`BMSPlayer.java:252-253` 의 `assist = max(assist,1); score = false;`). FLIP 은 레퍼런스도 assist 를 올리지 않으므로 **rbms 도 어시스트로 보지 않고 옵션 문자열에만 기록**한다. 남은 미확정은 FLIP 의 기록 해시 키뿐이다(`PlayDataAccessor.java:243` TODO → §7).
- **테스트**: DP 차트에서 FLIP 이 레인을 대칭 치환, BATTLE 이 1P 보면을 양쪽에 복제, SYNC-RAN 이 좌우 동일 시드, 어시스트 게이트가 IR 제출을 막는지.
- **크기**: M

#### F2-5 CN/HCN 별도 표시 · 판정문자 위치 · 키/스크 FAST-SLOW 분리 · S
- **현 상태**: CN/HCN 판정은 Phase A 에서 구현됨(`reference-divergences.md` CN/HCN 절)이나 **표시가 LN 과 동일**. FAST/SLOW 는 단일 카운터(`ResultView`, `result.rs:8-26`).
- **변경**: `SkinConfig` 에 CN/HCN 전용 색/폭 필드 추가(기존 필드 뒤에 `#[serde(default)]` 로 붙여 RON 후방호환), 판정문자 위치를 `judge_text_y` 로 데이터화, FAST/SLOW 를 `[u32; 2]`(키/스크래치)로 분리해 HUD·Result 양쪽에 표기.
- **테스트**: RON 3종(NORMAL/WIDE/default) 로드 무회귀, 플레이필드 골든 갱신 1장, FAST/SLOW 분리 집계 단위 테스트.
- **크기**: S

#### F2-6 5KEYS 레인/모드 표시 · S
- **현 상태**: 5K 판정 윈도우는 Phase A 에서 반영됨(`reference-divergences.md` J4). 그러나 **5K 전용 레인 레이아웃/모드 표기가 없다** — 계획 §2 Phase F 의 `5KEYS` 항목이 이것이며 초판 스펙에서 누락됐다.
- **변경**: `SkinConfig` 의 레인 폭/개수 테이블을 `Mode` 별로 분기해 5K(스크래치 + 5레인)·10K 를 7K/14K 와 별도 배치로 그린다. 곡선택·HUD·결과의 모드 라벨도 5K/10K 를 표기. BATTLE 승격(F2-4)이 5K→10K 를 만들므로 **F2-4 와 같은 갈래에 두는 것이 필수**다.
- **테스트**: 5K·10K 플레이필드 골든 각 1장(`golden_play.rs`), 7K 골든 불변.
- **크기**: S

**F2 소유 파일**: `apps/rbms-player/src/app_options.rs` · `app_input.rs` · `keyconfig.rs` · `crates/rbms-render/src/{hud,playfield,skin}.rs` · `crates/rbms-render/tests/golden_play.rs`

---

### 갈래 F3 — 결과 화면 / 타깃·그래프

#### F3-1 타깃 · PACEMAKER · L
- **현 상태**: 로컬 베스트 1개(`prev_best_ex`)뿐(`result.rs:20-23`).
- **변경**: `rbms-play`(또는 신규 `apps/rbms-player/src/target.rs`)에
  ```rust
  pub enum TargetKind { Max, Rate(f32), RankNext, LocalBest, Rival(usize), IrRank(usize) }
  pub struct Target { pub kind: TargetKind, pub name: String, pub ex: u32 }
  pub fn target_ex(kind: TargetKind, total_notes: u32, ctx: &TargetContext) -> Option<u32>;
  ```
  고정 레이트는 §1.1 표를 그대로 상수 배열로: `const RATE_TARGETS: [(&str, &str, f32); 11]`. EX 산출은 `(total_notes as f64 * 2.0 * rate as f64 / 100.0).ceil() as u32`.
  `Rival`/`IrRank` 는 Phase I 의 `chart_ranking`/`rivals` 비동기 캐시를 소비한다(**함수명으로만 결합, Phase I 이후 재확인**). 서버 없이도 `Max/Rate/RankNext/LocalBest` 는 동작해야 한다.
  HUD 에 실시간 페이스(현재 EX − 타깃 동시점 EX) 표시.
- **키**: 오버레이(F2-1)의 `TARGET` 행에서 선택.
- **설정 행**: `TARGET`(값 = 타깃 id, 기본 `RATE_AAA`).
- **테스트**: 11개 레이트의 EX 가 `ceil` 경계에서 정확(총노트 홀수/짝수 각각), `RANK_NEXT` 가 현재 랭크 바로 위 밴드를 고르는지, 서버 없음일 때 IR 타깃이 조용히 `LocalBest` 로 폴백.
- **크기**: L

#### F3-2 그래프 27분위 + MAX- 표기 · S
- **현 상태**(정확한 형태): `RANK_BANDS: [(&str, Color); 8]`(`result.rs:78`, F~AAA), `RANK_BOUNDS: [f32; 9]`(`:91`, `0, 2/9 … 8/9, 1`), `dj_rank`(`:95-104`), `draw_rank_bar`(`:106-118`, `for b in 0..8` 하드코딩), 소비처 `RANK_BANDS[dj_rank(..)]`(`:149`).
- **채택 설계(밴드 8 유지 + 세부 눈금 27)**. 27밴드로 늘리지 않는다 — 램프 색·`RANK_BANDS` 색 테이블·기존 소비처가 전부 8색 전제이고, DJ LEVEL 표기 자체는 IIDX 도 8단계이기 때문이다. 대신:
  1. `RANK_BANDS`(8) · `RANK_BOUNDS`(9) · `dj_rank` 는 **그대로 둔다**(기존 테스트 20여 개 전부 유지, 회귀 0). `RANK_BOUNDS` 가 `k/9 == 3k/27` 이므로 27분위의 부분집합이라 축이 어긋나지 않는다.
  2. 새로 `const RATE_27: [f32; 28] = [i/27 for i in 0..=27]` 과 `fn rate_27(ex, max_ex) -> u8`(0..=27, `max_ex == 0` 이면 0)를 추가한다.
  3. 새로 `fn dj_rank_label(ex, max_ex) -> &'static str` — 8밴드 이름에 27분위 3등분을 `-`/(무표기)/`+` 로 붙여 **`A-`/`A`/`A+` … `MAX-`(=26/27 이상 27/27 미만)/`MAX`(=100%)** 를 낸다. 라벨 집합은 §1.1 타깃 표와 정확히 같은 축이므로 타깃 이름과 1:1 대응한다.
  4. `draw_rank_bar` 는 굵은 경계 8개(기존 `RANK_BOUNDS`, 색) 위에 **27분위 얇은 눈금**을 덧그린다. `for b in 0..8` 은 유지하고 눈금 루프(`for i in 1..27`)를 추가한다 — 배열 길이 변경 없음.
- **테스트**: `rate_27` 27개 경계 정확 일치(`ceil` 이 아니라 `rate >= i/27` 하한 비교)·단조성·`max_ex == 0` 방어, `dj_rank_label` 이 24개 라벨(8밴드×3) + `MAX` 를 빠짐없이 내는지, 기존 `dj_rank_*` 테스트 전량 불변(회귀 검출용).
- **크기**: S

#### F3-3 게이지 추이 · 판정 분포 · 타이밍 히스토그램 · M
- **현 상태**: 결과에 판정 6행 카운트만.
- **변경**: 플레이 중 `PlaySession` 이 (a) 게이지 값을 1초 간격 링버퍼에 적재, (b) 판정별 ms 오차를 히스토그램 버킷(±200ms, 5ms 폭 = 81 bin)에 누적한다 — **계측 필드·훅 자체는 F0-10 이 `crates/rbms-play/src/lib.rs` 에 넣어 두므로 F3 은 소비만 한다**. `ResultView`(`result.rs:8-26`)에 `gauge_series: Vec<f32>` · `timing_hist: [u32; 81]` · `judge_dist: [u32; 6]` 필드를 추가하고 렌더.
- **주의**: 필드 추가는 `ResultView` 를 만드는 `app_result.rs`(F0 이 분리)와 `result.rs` 두 파일만 건드린다. 골든 갱신은 `golden_result.rs`(F3 소유)에서만 한다.
- **테스트**: 링버퍼 상한(장곡에서 무제한 증가 금지), 히스토그램 경계(±200ms 밖 클램프), 렌더 골든 1장, 빈 데이터에서 패닉 없음.
- **크기**: M

#### F3-4 리트라이 · 다음 곡 동선 · S
- **현 상태**: Result 는 Esc/Enter 로 Select 복귀만(`main.rs:1218-1222`). `R`/`N` 키 배선과 빈 핸들러는 **F0-7 이 선반영**하므로 F3 은 `app_result.rs` 의 본문만 채운다.
- **변경**: `R` = 같은 차트 재시작(옵션 유지, 로딩 경로 재사용), `N` = 정렬 순서상 다음 곡 시작, `Enter/Esc` = Select 복귀(현행 유지). 리플레이/오토플레이 결과에서는 `R`/`N` 을 비활성.
- **키 충돌**: Result 스테이지에는 현재 `Esc/Enter` 만 바인딩 → `R`/`N` 충돌 없음.
- **테스트**: 리트라이가 `PlayerConfig` 를 그대로 들고 재진입, 다음 곡이 현재 필터·정렬을 따르는지, 목록 끝에서 `N` 이 아무 일도 하지 않음.
- **크기**: S

**F3 소유 파일**: `apps/rbms-player/src/app_result.rs` · `apps/rbms-player/src/target.rs` · `crates/rbms-render/src/result.rs` · `crates/rbms-render/tests/golden_result.rs`

---

### 갈래 F4 — 로딩 / 백그라운드 / 셸

#### F4-1 토스트 · 상태줄(U1) · M
- **현 상태**: 실패 알림 수단 0. 앱 크레이트의 `eprintln!` 은 **11파일 43건**으로 흩어져 있다(실측):

| 파일 | 건수 | F0-8 이후 소유 |
|---|---|---|
| `app_select.rs` | 12 | F1 |
| `keyconfig.rs` | 5 | F2 |
| `app_play.rs` | 5 (`:17` 차트 없음 · `:32` md5 불일치 · `:60` 스킨 로드 실패 · `:208` 오디오 클럭 폴백 · `:332` 스코어 제출) | F4 |
| `main.rs` | 4 | F0 |
| `folders.rs` · `scores.rs` · `settings.rs` · `tables.rs` | 각 3 | F0(자유 함수, F0-8 에서 치환 후 무수정) |
| `app_input.rs` · `replay.rs` | 각 2 | F2 / F0 |
| `tablesrc.rs` | 1 | F4 |

  `crates/rbms-audio/src/engine.rs` 의 1건은 **Phase F 범위 밖**(크레이트 경계가 다르고 앱 알림 채널이 없다)이라 그대로 둔다. `crates/rbms-chart/src/tests.rs` 의 1건은 테스트 코드라 제외.
- **소유권 해법**: 43건의 `eprintln → notify` 치환은 **F0-8 이 직렬 단계에서 일괄 수행**한다(출력 문자열 동일 = 동작 무변경). F4-1 은 자기 파일만 쓰면서 `notify::drain` 을 `ToastQueue` 에 연결하고 렌더를 붙인다. 이로써 "`eprintln` 잔존 0" 조건이 F4 소유 밖 파일에 의존하지 않는다.
- **변경**: F0-3 의 `ToastQueue` 를 채운다.
  ```rust
  pub enum ToastLevel { Info, Warn, Error }
  pub struct Toast { pub level: ToastLevel, pub text: String, pub born: Instant }
  impl ToastQueue { pub fn push(&mut self, level: ToastLevel, text: impl Into<String>); pub fn active(&mut self, now: Instant) -> &[Toast]; }
  ```
  상한 5개·수명 4초(Error 는 8초), 렌더는 `crates/rbms-render/src/toast.rs`(신규)로 전 스테이지 공통 오버레이. 매 프레임 `notify::drain` 을 호출해 F0-8 이 심어 둔 전 호출 지점(차트 없음·표 로드 실패·폰트·미리듣기·스코어 제출·파일 저장·리플레이 로드)을 토스트로 승격시킨다.
- **테스트**: 상한 초과 시 오래된 것부터 밀림, 수명 만료, 렌더 골든 1장(`golden_shell.rs`), `notify::drain` 이 비워지는지. (`eprintln` 잔존 0 grep 테스트는 **F0-8 에서 이미 추가·통과**한 상태이므로 F4 는 깨지지 않게만 유지한다.)
- **크기**: M

#### F4-2 백그라운드화 + 로딩 단계 표시(U2·U8) · M
- **현 상태**: 아래가 전부 메인 스레드다. **괄호 안은 F0 이동 후의 소유 파일**이다.
  - 표 fetch: `fetch_and_match`(`tablesrc.rs:94`)를 `rescan_all_folders`(`app_select.rs:723`)와 `tables_input` 경로가 호출 — 스캔 스레드 안이라 UI 는 살아 있으나 **단계 표시가 없다**. 표 소스 추가/삭제(`app_select.rs:759-853`)는 메인 스레드 → **F0-4 가 `tables.rs`(F4)로 이동**.
  - BGA 전량 디코드: `app_play.rs:110-120`(진입 시 `bgamap` 전량 `decode_bga_256`) + 재생 중 갱신 경로.
  - rfd 3종: `pick_font`(`app_input.rs:142`) · `add_folder_dialog`(`app_select.rs:695`) · `add_table_file`(`app_select.rs:768`) → **F0-5 가 `dialog.rs`(F4)로 이동**.
  - `Loading` 은 `{Song, Scan}` 2종(`main.rs:602-610`)이고 단계 표시가 없어 진행바 100% 후 정지처럼 보인다 → **enum 확장은 F0-7**, 단계 값 생성·표시는 F4.
- **변경**: F0-7 이 깔아 둔 `Loading::{Scan, Song, Parse, Keysound{done,total}, Bga{done,total}, TableFetch{name}, Save}` 를 실제로 생성·전이시킨다. 각 단계를 워커 스레드로 옮기고 진행률·현재 단계명을 LOADING 화면에 표기(`app_play.rs:490-496`). rfd 는 `dialog.rs` 에서 별도 스레드로 열고 결과만 채널로 회수.
- **주의**: Esc 취소 시 `ks_cancel`(필드 `main.rs:758`, 초기화 `:963`, 세팅 `:1227` — Loading Esc 분기 `main.rs:1223-1240`) 과 동일하게 **모든 신규 워커에 cancel 플래그**를 배선한다(Phase A P6 과 같은 실수 반복 금지). 분기 자체는 F0-7 이 확장해 두고, F4 는 자기 워커의 cancel 만 채운다.
- **테스트**: 각 단계 취소 시 워커가 즉시 종료(플래그 관찰), 표 fetch 실패가 토스트로 노출되고 빈 레벨로 흡수되지 않음, 진행률 단조 증가.
- **크기**: M

#### F4-3 리사이즈 레터박스(K6) · S
- **현 상태**: 논리 1280×720 고정, surface 만 재구성 → 비 16:9 에서 왜곡(계획 §1.3 K6).
- **변경**: 논리 캔버스를 물리 surface 에 **비율 유지로 중앙 배치**하고 남는 영역은 배경색. 마우스 좌표 보정은 `gpu.rs`(F4)에서 논리 좌표로 역변환한 뒤 기존 히트 테스트(`app_select.rs:608` `handle_click`, `crates/rbms-render/src/select.rs` 의 `SelectHot`)에 넘긴다 — **F1 소유 파일은 수정하지 않는다**.
- **테스트**: 21:9·4:3·세로 창에서 골든 렌더(레터박스 폭 정확, `golden_shell.rs`), 마우스 히트 좌표 왕복 테스트(`gpu.rs` 단위 테스트).
- **크기**: S. **E1(클립/프리미티브)과 충돌 가능** — E1 이 `gpu.rs` 를 크게 고치므로 E1 착수 전에 끝내거나 E1 뒤로 미룬다(§6 R2).

#### F4-4 텍스트 입력 커서 · 붙여넣기 · S
- **현 상태**: 끝에 추가/백스페이스만. 텍스트 입력은 두 곳 — `settings_text_input`(`app_input.rs:180-209`, F2 소유)과 표 URL 입력(`tables_input` 내, F0-4 이후 `tables.rs`, F4 소유).
- **변경**: 편집 로직을 `apps/rbms-player/src/textedit.rs`(**F0-1 이 함께 신설**, F4 소유)의 `struct TextEdit { buf: String, cursor: usize }` 로 뽑고, 두 호출부는 F0 이 이 타입으로 치환한다(동작 동일). F4-4 는 `textedit.rs` 안에서만 커서 이동(←→/Home/End)·`Ctrl+V` 붙여넣기(`arboard` 신규 의존 — 근거를 커밋 메시지에 남긴다)·긴 문자열 스크롤을 구현한다.
- **키 충돌**: 텍스트 입력 모드는 키를 전부 소비하므로 전역 바인딩과 충돌 없음(`main.rs:1082-1125` 의 modal/search 분기와 동일 패턴). 신규 키는 `main.rs` 를 건드리지 않는다(이미 전량 위임된 경로).
- **테스트**: 커서 편집 시퀀스 단위 테스트(삽입/삭제/경계), 붙여넣기 실패 시 토스트.
- **크기**: S

**F4 소유 파일**: `apps/rbms-player/src/{app_play,toast,tablesrc,tables,gpu,dialog,textedit}.rs` · `crates/rbms-render/src/toast.rs` · `crates/rbms-render/tests/golden_shell.rs`

---

## 3. 설정 descriptor 신규 행

Phase C 의 descriptor 테이블(`{ id, 탭, 라벨, 값 타입, 범위, 스텝 }`)에 아래를 추가한다. C 미완 시 `SETTING_TABS`(`main.rs:707-714`) 폴백.

| 탭 | 라벨 | 필드 | 타입 | 범위 | 스텝 | 항목 |
|---|---|---|---|---|---|---|
| PLAY | HI-SPEED | `hispeed` | f64 | 0.01 ~ 20.0(배타) | `hispeed_step` | F2-2 |
| PLAY | HI-SPEED STEP | `hispeed_step` | f64 | 0.01 ~ 1.0 | 0.01 | F2-2 |
| PLAY | SPEED FIX | `fix_hispeed` | enum | OFF/START/MAX/MAIN/MIN | — | F2-2 |
| PLAY | LANE OPTION | `lane_option` | enum | OFF/FLIP/BATTLE/SYNC-RAN/SYMM-RAN | — | F2-4 |
| PLAY | LEGACY NOTE | `legacy_note` | bool | — | — | F2-4 |
| DISPLAY | LANE COVER | `cover` | f32 | 0.0 ~ 1.0 | 0.01 / 0.001(fine) | F2-3 |
| DISPLAY | LANE COVER ON | `enable_cover` | bool | — | — | F2-3 |
| DISPLAY | LIFT | `lift` | f32 | 0.0 ~ 1.0 | 0.01 / 0.001(fine) | F2-3 |
| DISPLAY | LIFT ON | `enable_lift` | bool | — | — | F2-3 |
| DISPLAY | HIDDEN+ | `hidden` | f32 | 0.0 ~ 1.0 | 0.01 / 0.001(fine) | F2-3 |
| DISPLAY | HIDDEN+ ON | `enable_hidden` | bool | — | — | F2-3 |
| DISPLAY | COVER FINE STEP | `lanecover_step_fine` | f32 | 0.0001 ~ 0.01 | 0.0001 | F2-3 |
| DISPLAY | WHITE NUMBER | `show_white_number` | bool | — | — | F2-3 |
| DISPLAY | JUDGE TEXT Y | `judge_text_y` | f32 | 0.0 ~ 1.0 | 0.01 | F2-5 |
| DISPLAY | LETTERBOX | `letterbox` | bool | — | — | F4-3 |
| DISPLAY | TARGET | `target_id` | string(enum id) | §1.1 목록 | — | F3-1 |
| DISPLAY | RESULT GRAPHS | `result_graphs` | bool | — | — | F3-3 |
| PLAY | 5KEYS LAYOUT | `five_key_layout` | bool | — | — | F2-6 |
| SELECT | SORT | `sort` | enum | 순환 9종 + 확장 4종(§1.3) | — | F1-1 |
| SELECT | FAVORITE ONLY | `favorite_only` | bool | — | — | F1-2 |
| SELECT | PREVIEW VOLUME | `preview_volume` | f32 | 0.0 ~ 1.0 | 0.05 | F1-5(B 의존) |
| SELECT | PREVIEW FADE MS | `preview_fade_ms` | u32 | 0 ~ 1000 | 50 | F1-5(B 의존) |

`SELECT` 탭은 신규다(현재 6탭 = PLAY/GAUGE/JUDGE/DISPLAY/INPUT/NETWORK, `main.rs:707-715`). NETWORK 탭은 **Phase I 소유**이므로 Phase F 가 건드리지 않는다.

**전체화면 설정 축소(결정 3 의 나머지 절반).** 결정 3 은 "오버레이 주경로 + 전체화면 설정은 환경설정으로 축소"인데 위 표는 행을 늘린다 — 모순이 아니라 **역할 분리**로 해소한다. 곡마다 바뀌는 값(HI-SPEED / SPEED FIX / LANE COVER / LIFT / HIDDEN+ / LANE OPTION / TARGET / RANDOM / GAUGE)은 오버레이가 주경로이고, 전체화면 설정에서는 같은 행을 **읽기 가능·편집 가능하되 목록 하단의 `PLAY OPTIONS` 하위 그룹으로 접어** 첫 화면 밀도를 늘리지 않는다. 축소의 실체는 "행 삭제"가 아니라 **기본 노출 행 수 유지 + 나머지는 접힘**이며, 이 그룹 접힘 상태는 `settings_ui.rs`(F0 소유)가 구현한다. 행 삭제가 필요하다는 판단이 서면 별도 결정으로 올린다.

---

## 4. 키 바인딩 총괄 (충돌 검사 결과)

| 스테이지 | 기존 점유 | Phase F 신규 | 충돌 |
|---|---|---|---|
| Select | `Esc ←` `/` `F3` `Tab` `O` `T` `R` `↑↓` `Enter →` (`main.rs:1126-1152`) | `Shift+F3`(역정렬) `F2`(필터) `F1`(옵션 토글) `F`(즐겨찾기) `ShiftLeft`(옵션 홀드) | 없음 |
| Select(검색 중) | 전 문자 소비 + `Esc` `F3` `↑↓` `Enter`(`main.rs:1093-1120`) | 없음(신규 키는 검색 중 비활성) | 없음 |
| Select(기록 모달) | `Esc ↑↓ Enter`(`main.rs:1082-1091`) | 없음 | 없음 |
| Play | `Esc`(`main.rs:1241-1250`) + `ControlAction` 6종(사용자 바인딩) + 분석 `Space = - PageUp PageDown` | `ControlAction::{HiddenUp,HiddenDown}` 2종 추가(기본 미할당) + `Esc` 길게/2회(결정 6) | 없음(기본 미할당) |
| Result | `Esc Enter`(`main.rs:1218-1222`) | `R`(리트라이) `N`(다음 곡) | 없음 |
| Loading | `Esc`(취소, `main.rs:1223-1240`) | 없음 | 없음 |

- `ControlAction::ALL` 배열 길이가 `[ControlAction; 6]` 상수(`keyconfig.rs:93-100`)라 8 로 갱신해야 한다(`label()` 은 `:103-112`). 기존 키설정 파일 로드 시 미지정 액션은 미할당으로 두고 마이그레이션한다.
- **모든 신규 키 배선은 F0-7 이 `main.rs` 에 선반영**하고, 각 갈래는 자기 파일의 핸들러 본문만 채운다. 갈래가 `main.rs` 를 여는 일은 없다.
- 결정 6(Play 중 Esc 즉시 이탈 유지 + 길게/2회 옵션)은 `main.rs:1241-1250` 에 있어 **F0 에서 처리**한다(F0-7 마지막 항목). 이는 F0 의 "동작 무변경" 원칙에 대한 **유일한 명시적 예외**이며, F0 통과 조건도 그렇게 정정했다(§F0 통과 조건). 별도 커밋으로 분리해 리뷰한다.

---

## 5. 갈래별 파일 소유권 (교집합 0)

아래 표는 **파일 단위 배타 소유**다. F0 이 §F0 의 이동·선반영 10항목을 끝낸 뒤에만 성립하므로, F0 미완 상태에서 병렬 착수하면 안 된다.

### 5.1 F0 (선행·직렬)

`apps/rbms-player/src/main.rs` · `settings.rs` · `settings_ui.rs`(신규) · `notify.rs`(신규) · `crates/rbms-render/src/lib.rs` · `crates/rbms-render/tests/golden.rs`(공통 하네스) · `crates/rbms-play/src/lib.rs` · `Cargo.toml` 전부.
F0 은 위 6·7·8 항목으로 **다른 갈래가 필요로 하는 `main.rs`/`settings_ui.rs`/`eprintln` 변경을 전부 선반영**한다. F0 커밋 이후 이 파일들은 Phase F 동안 **동결**이다.

### 5.2 갈래별 파일 목록 (명시)

| 갈래 | 소유 파일 (전량 열거) |
|---|---|
| **F1** 곡선택 | `apps/rbms-player/src/app_select.rs`<br>`apps/rbms-player/src/favorites.rs`(F0 신설)<br>`crates/rbms-render/src/select.rs`<br>`crates/rbms-render/tests/golden_select.rs`(F0 분할) |
| **F2** 옵션·플레이 | `apps/rbms-player/src/app_options.rs`(F0 신설)<br>`apps/rbms-player/src/app_input.rs`<br>`apps/rbms-player/src/keyconfig.rs`<br>`crates/rbms-render/src/hud.rs`<br>`crates/rbms-render/src/playfield.rs`<br>`crates/rbms-render/src/skin.rs`<br>`crates/rbms-render/tests/golden_play.rs`(F0 분할) |
| **F3** 결과 | `apps/rbms-player/src/app_result.rs`(F0 신설)<br>`apps/rbms-player/src/target.rs`(F0 신설)<br>`crates/rbms-render/src/result.rs`<br>`crates/rbms-render/tests/golden_result.rs`(F0 분할) |
| **F4** 로딩·셸 | `apps/rbms-player/src/app_play.rs`<br>`apps/rbms-player/src/toast.rs`(F0 신설)<br>`apps/rbms-player/src/tablesrc.rs`<br>`apps/rbms-player/src/tables.rs`<br>`apps/rbms-player/src/gpu.rs`<br>`apps/rbms-player/src/dialog.rs`(F0 신설)<br>`apps/rbms-player/src/textedit.rs`(F0 신설)<br>`crates/rbms-render/src/toast.rs`(F0 신설)<br>`crates/rbms-render/tests/golden_shell.rs`(F0 분할) |

**소유권 검증.** 위 4갈래 목록을 집합 F1·F2·F3·F4 로 볼 때 **모든 쌍의 교집합은 공집합**이다(F1∩F2 = F1∩F3 = F1∩F4 = F2∩F3 = F2∩F4 = F3∩F4 = ∅). F0 집합(§5.1)과의 교집합도 전부 공집합이며, F0 은 병렬 갈래가 열리기 전에 종료되므로 시간축에서도 겹치지 않는다. 갈래에 속하지 않는 앱 파일(`scores.rs` · `folders.rs` · `replay.rs` · `songdb.rs` 등)은 **Phase F 동안 어느 갈래도 수정하지 않는다** — 이들에 필요한 변경(`eprintln` 치환)은 F0-8 에서 이미 끝난다.

**초판에서 정정된 충돌 4건.**
1. `main.rs` — F1(SortMode·키) · F2(오버레이 진입) · F3(Result R/N) · F4(Loading enum·Esc)가 각자 열어야 했다 → **F0-7 이 전부 선반영**.
2. `eprintln` 전수 치환 — F4-1 의 완료 조건이 F1·F0·무소유 파일에 걸쳐 있었다 → **F0-8 이 일괄 치환**, F4-1 은 drain 만.
3. `crates/rbms-render/tests/golden.rs` — 4갈래 공유였고 표에 없었다 → **F0-9 가 갈래별 4파일로 분할**.
4. 초판이 놓친 3건 — `rebuild_select_items`(F1 이 고쳐야 하는데 `app_input.rs`=F2), 표 관리·rfd 함수(F4 가 고쳐야 하는데 `app_select.rs`=F1), `setting_line`/`adjust_setting`(전 갈래가 행 추가, `app_select.rs`=F1) → **F0-3·4·5·6 이 이동**.

**공유 파일 규칙.** `crates/rbms-render/src/lib.rs`(모듈 선언·재export)와 `Cargo.toml`(`arboard` 포함 신규 의존)은 **F0 에서 최종 형태까지 미리 등록**한다(빈 모듈 파일 포함). 어느 갈래도 `crates/rbms-judge` · `crates/rbms-chart` · `crates/rbms-audio` · `crates/rbms-ir` · `web/` 를 수정하지 않는다.

**Phase 간 충돌.** Phase I 가 `apps/rbms-player`(NETWORK 탭·IR 패널)와 `crates/rbms-ir` 를 동시 수정 중이다. Phase F 는 **NETWORK 탭 행·IR 호출을 정의하지 않고**, F3-1 이 Phase I 의 랭킹/라이벌 조회 함수를 **함수명으로만** 호출한다(`chart_ranking` · `rivals`). Phase I 머지 후 F3-1 착수 직전에 그 함수 시그니처를 재확인한다 — 이 문서의 Phase I 관련 앵커는 **라인 번호를 쓰지 않는다**.

## 6. 리스크

| id | 리스크 | 완화 |
|---|---|---|
| R1 | F0 이 10항목으로 커져 병렬 이득이 사라진다 | 10항목은 전부 **기계적 이동·선반영**이며 로직 변경이 없다. 항목별 커밋으로 쪼개고, 골든 5파일 전량 불변 + `cargo test --workspace` 무회귀를 통과 조건으로 삼는다. 유일한 동작 변경(결정 6 Esc)은 마지막 별도 커밋 |
| R2 | F4-3(레터박스)이 `gpu.rs` 를 고쳐 Phase E1(프리미티브·클립)과 정면 충돌 | E1 착수 전에 F4-3 을 끝내거나, E1 완료 후로 이월. 둘을 동시에 열지 않는다 |
| R3 | 그린넘버 공식 스케일 차이(레퍼런스 2400 vs rbms `scroll::green_number`) | 상수 복사 금지. rbms 정의로 역산 유도 후 **5모드 × BPM 3종 불변 테스트**로 검증 |
| R4 | 27분위 전환이 기존 `dj_rank` 테스트 20여 개를 깬다 | **채택 설계가 `RANK_BANDS`/`RANK_BOUNDS`/`dj_rank` 를 건드리지 않으므로(F3-2) 기존 테스트는 전량 유지**되고, 오히려 회귀 검출기로 쓴다. 신규는 `rate_27`/`dj_rank_label` 테스트로 분리. 표시만 바뀌므로 결정 2(`rule_version`)와 무관 |
| R5 | F2-4 BATTLE 어시스트 게이트 누락 시 부정 기록 유입 | 결정 12 게이트 경로(`assist` 플래그 → IR 제출·리플레이 저장 차단)를 재사용하고, 어시스트 유발 옵션마다 단위 테스트 1개 |
| R6 | F4-2 워커 확대로 cancel 미배선 재발(Phase A P6 과 동일 실수) | 신규 워커마다 `Arc<AtomicBool>` cancel 을 **생성자 인자로 강제**하고, 취소 테스트를 워커별로 1개씩 |
| R7 | `ResultView`·`PlayerConfig` 필드 추가가 저장 스키마를 깬다 | 전 신규 필드에 `#[serde(default)]`, Phase C 의 `schema_version` 마이그레이션과 함께 왕복 테스트 |
| R8 | `arboard`(F4-4) 신규 의존 | 붙여넣기 없이도 동작하도록 feature 로 분리하거나, 실패 시 토스트로 degrade. `Cargo.toml` 추가는 **F0 이 미리 등록**한다(§5 공유 파일 규칙) |
| R9 | F0 미완 상태로 갈래를 열면 §5 파티션이 무효 | F0 커밋이 dev 에 들어가기 전에는 F1~F4 브랜치를 만들지 않는다. 각 갈래는 F0 커밋을 base 로 분기한다 |

---

## 7. 미확인 사항

> 초판 9건 중 **3건은 이번 검증에서 해소**되어 §1 본문으로 옮겼다(아래 "해소됨" 참조). 남은 6건만 미확인이다.

### 남은 미확인

1. **LEGACY NOTE 의 레퍼런스 대응물** — 레퍼런스 소스에서 대응 옵션을 찾지 못했다(IIDX 고유 표시 옵션으로 추정). 사양은 "LN 을 일반 노트로 그린다"는 통념에 근거하며 1차 출처 미확인. F2-4 착수 시 재조사하고, 못 찾으면 rbms 고유 기능으로 `reference-divergences.md` 에 등록한다.
2. **FLIP 의 기록 해시 키** — 어시스트 여부는 해소됐다(§1.4: FLIP 은 어시스트 아님, 레퍼런스와 일치). 남은 것은 "FLIP 플레이의 스코어를 원보면 해시로 저장할지 좌우 반전 보면 해시로 저장할지"이며, 레퍼런스 자신이 미정이다(`PlayDataAccessor.java:243` TODO). **rbms 제안 = 원보면 해시로 저장 + 옵션 문자열에만 FLIP 기록**(DP 좌우 반전은 난도 동등으로 보므로). 사용자 확인 필요.
3. **`RivalTargetProperty`/`InternetRankingTargetProperty` 의 데이터 소스** — Phase I 의 IR 클라이언트가 라이벌·랭킹을 어떤 타입으로 노출할지 확정되지 않았다. F3-1 의 `TargetContext` 필드는 Phase I 머지 후 재확인.
4. **BPM 변화 예고 / SPEED ADJUST(IIDX 34)** — 계획 §2 Phase F 목록에 있으나 레퍼런스에 대응물이 없고 IIDX 34 는 로케테 사양(계획 §5)이다. 본 설계에서는 F2-2 의 `fix_hispeed`(그린넘버 고정)로 실질 효과를 대체했고, "변속 구간 예고 표시"는 **범위에서 제외**했다. 필요하면 별도 항목으로 추가 결정 필요.
5. **Phase C descriptor 테이블의 실제 시그니처** — 아직 구현되지 않았다. §3 의 열 구성(`id/탭/라벨/타입/범위/스텝`)은 이 문서의 제안이며 C 확정 시 맞춘다. F0-6 의 `settings_ui.rs` 는 C 가 오면 통째로 교체되는 것을 전제로 만든다.
6. **`crates/rbms-play` 의 계측 훅 위치** — F3-3 의 게이지 시계열·타이밍 히스토그램을 `PlaySession`(Phase C 에서 `rbms-play` 로 이관 예정) 중 어디에 붙일지는 C 완료 후 확정. 현 `crates/rbms-play/src/lib.rs`(915줄) 기준 앵커는 C 이후 무효가 될 수 있다(F0-10 은 함수명 기준으로 붙인다).

### 해소됨 (이번 검증에서 1차 출처 확인)

- ~~`doubleoption` 0~3 의 각 값 의미~~ → **해소**. 소비처 `BMSPlayer.java:238-297` 통독으로 `0=OFF / 1=FLIP / 2=BATTLE / 3=BATTLE+오토스크래치` 확정, 어시스트 규칙까지 확정(§1.4 표). SYNC-RANDOM 이 `doubleoption` 축이 아니라는 것도 함께 확인.
- ~~`NextRankTargetProperty` 본문 미통독~~ → **해소**. `TargetProperty.java:301-331` 통독. `now < ceil(max*i/27)` 을 `i = 15..26` 으로 훑어 첫 초과 값을 타깃으로 삼고, 없으면 `max`. 표시명 `"NEXT RANK"`(§1.1).
- ~~미리듣기 U5 stale~~ → **해소**(코드 확인). 현 코드는 이미 시간 기준 333ms(`main.rs:59-62`)다. 계획 §1.6 U5 의 "프레임 수(20) 기준" 서술이 stale — **계획 문서 수정은 이 스펙의 소유 밖이므로 보고만 한다**(§0 표에도 반영해 둠).

---

## 8. 비평 반영 (2026-09-09)

완성도 비평(앵커 정확성·소유권 파티션·계획 항목 누락·근거 부족) 전 항목을 코드/레퍼런스 원문 대조로 검증하고 반영했다.

### 8.1 수정한 앵커 (전부 실측으로 확인)

| 위치 | 초판 | 정정 |
|---|---|---|
| 레인커버/LIFT 스텝 | `app_select.rs:983-984` | **`:979`(lift) · `:980`(cover)** |
| hispeed clamp | `app_select.rs:967` | **`:969`**(`:967` 은 `match global {`) |
| `green_number_for` | `main.rs:184-193` | **`main.rs:196-206`**(184-193 은 `judge_time_us`/`keysound_time_us`) |
| `write_atomic` | `main.rs:157` | **`main.rs:160`** |
| `ks_cancel` | `main.rs:756` | **필드 `:758` · 초기화 `:963` · 세팅 `:1227`** |
| BGA 디코드 | `app_play.rs:105-118` | **`:110-120`** |
| `app_play.rs` eprintln | `17, 58, 314` | **`17, 32, 60, 208, 332`(5건)** |
| 커버 디코드 재사용 | `app_select.rs:266-278` | **`:287-292`**(`refresh_focused_detail` 내부) |
| 목록/뷰 조립 | `app_select.rs:78-128`(`select_view`/`select_items`) | **`rebuild_select_items` = `app_input.rs:9-79`, `build_select_view` = `app_select.rs:106-274`, `SelectView` = `main.rs:635-648`** |
| 표 fetch | `app_select.rs:740-748` | **`fetch_and_match` = `tablesrc.rs:94`, 호출 `app_select.rs:723`, 표 관리 `:759-853`**(`:738-750` 은 `folders_input`) |
| `SortMode` | `main.rs:651-674` | **enum `:651-657` · impl `:659-673`** |
| `Loading` | `main.rs:598-607` | **`:598-610`(enum `:602`) · Esc 분기 `:1223-1240`** |
| `SETTING_TABS` | `main.rs:707-714` | **`:707-715`** |
| `exit_search`/`start_search` | `:38-43` / `:27-36` | **`:34-41` / `:25-33`** |
| `settings_text_input` | `app_input.rs:180-203` | **`:180-209`** |
| `draw_rank_bar` | `result.rs:106-118` | 초판 정확(비평의 `:107` 이 오류) |
| BarSorter 12행 · 타깃 레이트표 · 그린넘버 공식 | — | 초판 정확(재검증 통과) |

### 8.2 소유권 파티션 재작업 (§5)

초판 §5 는 성립하지 않았다. 충돌 4종을 확인하고 **F0 의 이동·선반영 10항목**으로 해소한 뒤, 갈래별 파일을 전량 열거하고 "소유권 검증" 문장(모든 쌍의 교집합 = ∅)을 명시했다. 핵심 변경: `main.rs` 의 갈래별 편집을 F0-7 이 선반영, `eprintln` 43건 치환을 F0-8 이 일괄 수행, 골든 테스트를 F0-9 가 갈래별 4파일로 분할, `rebuild_select_items`/표 관리/rfd/설정 UI 를 F0-3~6 이 소유 정합 위치로 이동.

### 8.3 보강한 설계·근거

- **27분위(F3-2)**: `RANK_BANDS`(8) / `RANK_BOUNDS`(9) / `for b in 0..8` 하드코딩을 명시하고, **밴드 8 유지 + `rate_27`·`dj_rank_label`(±표기) 추가 + 눈금 오버레이** 로 설계를 확정. 배열 길이 불변이라 기존 테스트 20여 개가 전부 살아남는다.
- **정렬(§1.3·F1-1)**: `defaultSorter` 8종(`BarSorter.java:260`) vs `allSorter` 12종(`:262`) 구분을 추가하고, F3 순환 대상을 9종으로 확정.
- **하이스피드(§1.2)**: `HISPEED_MIN/MAX`(`PlayConfig.java:18-19`)·`HISPEEDMARGIN_MIN/MAX`(`:56-57`)를 직접 인용하고, `hispeed_step` 범위 축소를 **의도적 발산**으로 표기.
- **SUD+ / HID+ 동시 적용(F2-3)**: 초판 누락. `enable_cover`/`enable_lift`/`enable_hidden` 3토글과 가시 구간 계산식을 추가하고 §3 에 4행 증설.
- **5KEYS(F2-6)**: 계획 §2 항목이 통째로 빠져 있었다 → 5K/10K 레인 레이아웃·모드 표기 항목으로 신설(BATTLE 승격과 묶여 F2 소유).
- **결정 3 모순**: "전체화면 설정 축소"를 행 삭제가 아니라 **`PLAY OPTIONS` 하위 그룹 접힘 + 오버레이 주경로**로 해석해 §3 에 명시.
- **결정 6 모순**: Play Esc 변경이 F0 의 "동작 무변경"과 충돌했다 → F0-7 의 마지막 항목으로 편입하고, **F0 통과 조건에 유일한 예외**임을 적고 별도 커밋 분리를 규정.
- **미확인 3건 해소**: `doubleoption` 매핑 · `NextRankTargetProperty` 알고리즘 · 미리듣기 U5 stale.

### 8.4 반려한 지적

- **`draw_rank_bar` 앵커** — 비평은 `result.rs:107` 이라 했으나 실제 정의는 **`:106`**(`:107` 은 본문 첫 줄). 초판이 옳아 수정하지 않았다.
- **`dj_rank` 앵커** — 비평은 "`RANK_BOUNDS`(result.rs:91)가 곧 `dj_rank`"인 것처럼 적었으나, `:91` 은 `RANK_BOUNDS`, `dj_rank` 는 `:95` 다. 초판의 `result.rs:95-104` 가 옳다(단 "9분위 밴드"라는 표현은 8밴드/9경계로 정정했다).
- **`SortMode`·`SETTING_TABS`·`main.rs:1136-1138`(O/T/R)·`keyconfig.rs:93` 앵커** — 비평이 오차를 주장하거나 재지정했으나 실측 결과 초판 범위가 코드를 정확히 덮는다(±1행 이내). 표기만 정밀화했다.
