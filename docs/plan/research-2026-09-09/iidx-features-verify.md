# IIDX 기능 격차 감사 — 회의적 재검증

대상 보고서: `research/iidx-feature-gap.md` (findings 01~14)
검증 방식: 인용된 rbms / beatoraja 파일을 직접 열어 주장·라인번호·심각도 확인. 10분 상한.

## 요약

| id | 판정 | 요지 |
|----|------|------|
| 01 | confirmed | HIDDEN+ 부재 사실. beatoraja 근거는 `PlayConfig.java:78-82`가 더 직접적 |
| 02 | confirmed | PACEMAKER 타깃 선택 없음. hud/설정 근거 모두 일치 |
| 03 | confirmed | 정렬 5종만, 필터·즐겨찾기 0건 |
| 04 | partially | 격차는 실재하나 beatoraja 근거가 부정확, severity high는 과대 |
| 05 | confirmed | FLIP/BATTLE/SYNC 부재. 라인번호 일부 어긋남 |
| 06 | confirmed | IR에 course DTO만 있고 플레이 경로 없음 |
| 07 | confirmed | LEGACY NOTE 부재. LEGACY 단독은 effort S가 타당 |
| 08 | confirmed | 리절트 스칼라 전용 |
| 09 | confirmed | "beatoraja 미확인"은 해소됨 (`PlayConfig.java:24,43-49`) |
| 10 | confirmed | 화이트넘버 없음 |
| 11 | partially | 격차는 실재하나 beatoraja 실제 동작(gaugeAutoShift)과 권고가 다름 |
| 12 | confirmed | RANK_BANDS 8단, 9분위 |
| 13 | confirmed | 스킨 NORMAL/WIDE 토글뿐 |
| 14 | confirmed | 자기 신고대로 미확인 |

---

## 개별 검증

### 01 HIDDEN+ — confirmed

- `crates/rbms-render/src/playfield.rs:166` `render_lane_cover(r, skin, cover_frac)`가 `skin.top_y`부터 아래로 `h*f`만 칠한다. 대칭 함수 없음.
- `grep -rn hidden --include=*.rs`: `rbms-model/src/lib.rs:52` (hidden 채널), `shuffle.rs:167,214` (hidden 채널 remap)만. 렌더/설정 쪽 hidden 0건.
- LIFT는 하단 커버가 아니라 판정선 상승이다: `crates/rbms-render/src/skin.rs:221` `judge_y = cfg.judge_y - (cfg.judge_y - cfg.top_y) * cfg.lift`. 따라서 HIDDEN+와 별개 기능이라는 주장은 정확.
- 정정: 인용 라인 `playfield.rs:163-167` → 실제 `166-176`, `app_select.rs:893-894` → 실제 `896-897`.
- 근거 보강: beatoraja `PlayConfig.java:78-82` `private float hidden = 0.1f; private boolean enablehidden = false;` 가 `SkinHidden.java`보다 직접적. `SkinHidden.java:26-33`은 lift 연동 소실선을 보여줘 보조 근거로 유효.
- 권고에 대한 정정: "그린넘버를 cover+hidden 합산으로 보정" 이전에, **현재도 LIFT가 그린넘버에 반영되지 않는 버그**가 있다(missed-01 참조).

### 02 PACEMAKER 타깃 — confirmed

- `crates/rbms-render/src/hud.rs:29` `fn draw_score_graph(..., ex: u32, max_ex: u32, best: Option<u32>)` — 타깃 인자 없음.
- `hud.rs:34` 밴드가 `A/AA/AAA` 3개 하드코딩.
- `apps/rbms-player/src/app_select.rs:891-915` 설정 0~23 전수 확인, target/pacemaker 항목 없음.
- beatoraja `play/TargetProperty.java:118-140` RATE_A- ~ RATE_MAX- 27분위 정적 타깃, `:155-190` RivalTargetProperty 확인.

### 03 필터·즐겨찾기 — confirmed

- `apps/rbms-player/src/main.rs:543-566` `enum SortMode { Default, Title, Artist, Level, Clear }` — 정확히 5종.
- `grep -rni "favorit"` 0건. `filter` 히트는 전부 이터레이터/파일다이얼로그로 무관.
- beatoraja `select/DifficultyFilter.java`, `select/ModeFilter.java`, `select/BarSorter.java` 실존 확인.

### 04 시작 전 옵션 패널 — partially

- 사실 부분: `apps/rbms-player/src/main.rs:479-488` `enum Stage { Select, Settings, KeyConfig, Tables, Folders, Loading, Play, Result }` — 곡별 옵션 오버레이 스테이지 없음. 맞다.
- 반박 1(근거): 인용한 "beatoraja decide 화면 + PlayerConfig 반영"은 옵션 패널 근거가 아니다. beatoraja의 decide는 전환 연출이고, 옵션은 select 화면 키바인딩으로 바꾼다. 즉 beatoraja도 별도 "시작 전 옵션 패널 스테이지"를 갖지 않는다 — 대응물은 select 화면 내 즉시 조작이다.
- 반박 2(심각도): rbms에도 select에서 바로 진입하는 `Stage::Settings`가 있고, 플레이 중 실시간 조절도 가능하다(`app_input.rs:92-101` HiSpeedUp/Down, CoverUp/Down, LiftUp/Down). 옵션 접근 불가가 아니라 동선 길이 문제이므로 **high → medium** 이 타당.

### 05 DP 옵션 FLIP/BATTLE/SYNC — confirmed

- `crates/rbms-chart/src/shuffle.rs:6-14, 17-27` NoteOption 8종 확인, FLIP/BATTLE 없음.
- 인용 주석 위치 정정: "crossing is a separate FLIP option in beatoraja, not implemented here"는 `shuffle.rs:88` (보고서는 95-97).
- side별 독립 시드: `shuffle.rs:113` `Rng::new(seed.wrapping_add(side as u64).wrapping_mul(0x9E3779B97F4A7C15))` — SYNC 불가 주장 성립. (보고서 인용 106-112는 근사)
- beatoraja `pattern/LaneShuffleModifier.java` 내 PlayerFlipModifier/PlayerBattleModifier 존재는 보고서 인용대로. (본 검증에서 파일 존재만 확인, 라인 미확인)

### 06 코스/단위인정 — confirmed

- `crates/rbms-ir/src/dto.rs:238-240` `CourseSubmission { course_hash, charts }`, `crates/rbms-ir/src/lib.rs` submit_course/course_ranking 존재.
- 플레이어 쪽 course 히트는 `apps/rbms-player/src/ir_map.rs:207` 주석("rbms never plays" 단위인정 게이지)뿐 — 오히려 미구현을 코드 주석이 자인한다. 보고서보다 강한 근거.
- 게이지 이월 권고도 타당: `crates/rbms-judge/src/gauge.rs:73` `Gauge::new(kind, total, notes)`에 초기값 인자 없고 `value: s.init` 고정.

### 07 LEGACY NOTE / 5KEYS — confirmed (effort 세분 필요)

- rbms 셔플러는 레인 재배치 전용, 노트 종류 변환 없음(`shuffle.rs` 전수 시그니처 확인).
- beatoraja `pattern/LongNoteModifier.java:27-38` REMOVE 모드가 `ln.isEnd() ? null : new NormalNote(...)` 로 정확히 보고서 설명대로 동작. 추가로 `setAssistLevel(AssistLevel.ASSIST)`(:37)를 호출한다 — 어시스트 플래그 기록 권고는 beatoraja 실동작과 일치.
- 정정: LEGACY NOTE 단독은 S(반나절)이 타당하고, M은 5KEYS(ModeModifier 포팅)에 해당. 묶어서 M으로 둔 것은 과대.

### 08 리절트 그래프 — confirmed

- `crates/rbms-render/src/result.rs:86-135` 텍스트 + `draw_rank_bar`만. 시계열/분포 없음.
- beatoraja `result/SkinGaugeGraphObject.java` 실존 확인.

### 09 FLOATING HI-SPEED — confirmed (근거 보강)

- rbms `app_select.rs:894` `2 => ("SPEED FIX", if constant_speed { "CONSTANT" } else { "FLOATING" })` (보고서 892는 오차). 실제 동작은 BPM 무시 스크롤 토글이 맞다.
- **보고서가 "미확인"으로 남긴 beatoraja 근거를 확정**: `PlayConfig.java:24` `private int duration = 500;`(그린넘버 ms), `PlayConfig.java:43-49` `fixhispeed` + `FIX_HISPEED_OFF/STARTBPM/MAXBPM/MAINBPM/MINBPM`. 즉 beatoraja는 duration(그린넘버)을 고정하고 곡 BPM 기준으로 hispeed를 역산한다 — 보고서 권고와 정확히 일치.

### 10 화이트넘버 — confirmed

- `crates/rbms-render/src/hud.rs:84-91` 좌측 컬럼 EX/BEST/GREEN 3줄. `HudView`(hud.rs:8-22)에 hispeed 필드 자체가 없다.

### 11 게이지 폴백 — partially

- 사실 부분: `gauge.rs:63-72` `Gauge { kind, value, ... }` 단일 kind, `gauge.rs:130-155` `clear_lamp`가 `gauge.kind`로만 램프 결정. 맞다.
- 정정(근거·권고): beatoraja의 대응 기능은 "보조 게이지 병행 판정"이 아니라 **GAUGE AUTO SHIFT** 다 — `PlayerConfig.java:157-167` `gaugeAutoShift` + `GAUGEAUTOSHIFT_NONE/CONTINUE/SURVIVAL_TO_GROOVE/BESTCLEAR/SELECT_TO_UNDER`. 실패 시 **다음 플레이의 게이지 종류를 낮추는** 동작에 가깝다.
- 따라서 "항상 3트랙 동시 시뮬레이션 후 최고 등급 램프" 권고는 beatoraja 시맨틱이 아니며, 그대로 하면 HARD 실패 플레이에 EASY CLEAR 램프를 주어 램프 인플레가 난다. 권고를 autoshift 포팅으로 교체해야 한다. severity low는 유지.

### 12 MAX- 표기 — confirmed

- `result.rs:30-39` RANK_BANDS 8종(F..AAA), `result.rs:43` RANK_BOUNDS 9분위, `result.rs:47-53` dj_rank.
- beatoraja 27분위는 `TargetProperty.java:118-140`에서 확인.
- 보강: 같은 9분위 문제가 **라이브 HUD 그래프에도** 있다(`hud.rs:34`). 보고서는 리절트만 지적했다.

### 13 스킨 축 — confirmed

- `app_select.rs:905` SKIN 표시, `:962-965` adjust가 NORMAL<->WIDE 토글 + skin_path None 초기화뿐.
- `hud.rs:99-100` judge_colors/judge_labels가 skin 구조체 고정값 사용. 맞다.

### 14 IIDX 31~34 미확인 — confirmed

- 보고서가 스스로 미확인이라 표기했고, 본 검증에서도 1차 출처를 확보하지 않았다. 판정 유지.

---

## 보고서가 놓친 항목 (missed)

| id | 내용 | 근거 |
|----|------|------|
| M1 | 그린넘버가 LIFT를 반영하지 않는다(표시·IR 제출 모두 오차) | `app_play.rs:294-297`, `app_play.rs:551-555` 는 cover만 곱함 / `skin.rs:221` 은 lift로 judge_y를 올려 실제 이동거리를 줄임 |
| M2 | 어시스트 플래그가 기록되지 않는다 — SCRATCH AUTO/AUTOPLAY 여부와 무관하게 항상 빈 배열 | `app_play.rs:282` `assist: vec![]`, 반면 `app_play.rs:132` 에서 scratch_auto 실동작. beatoraja `pattern/LongNoteModifier.java:31-37` 은 `setAssistLevel` 로 램프 자격을 낮춤 |
| M3 | GAUGE AUTO SHIFT 미구현(finding 11과 별개 기능) | beatoraja `PlayerConfig.java:157-167` 4모드 / rbms `settings.rs` 에 대응 필드 없음 |
| M4 | 레인커버 조절이 5% 계단 + 즉시 반영 — 미세조정·전환 연출 없음 | `app_input.rs:94-95` `+/- 0.05` / beatoraja `PlayConfig.java:87-95` lanecovermarginlow 0.001, marginhigh 0.01, lanecoverswitchduration 500 |
| M5 | 하이스피드 범위·해상도가 좁다 | rbms `app_input.rs:92-93` step 0.25, clamp 0.5..10.0 / beatoraja `PlayConfig.java:18-19` HISPEED_MIN 0.01, HISPEED_MAX 20 |
| M6 | 라이브 HUD 스코어그래프도 A/AA/AAA 3밴드뿐 — finding 12의 27분위 지적이 리절트에만 적용됨 | `hud.rs:34` `[(6.0/9.0,"A"),(7.0/9.0,"AA"),(8.0/9.0,"AAA")]` |
| M7 | CONSTANT 모드 그린넘버가 매직상수 2000.0(=BPM 120)으로 두 곳에 중복 구현 | `app_play.rs:295`, `app_play.rs:552` — `scroll::green_number` 를 우회 |
| M8 | 대전/ARENA 계열 멀티플레이 축이 전혀 없음(IIDX 32 복귀 기능) — 보고서 §0에 언급만 되고 finding 미기재 | rbms IR은 제출/랭킹 조회 전용(`crates/rbms-ir/src/lib.rs`), 실시간 대전 경로 없음 |

## 미조사 범위

- beatoraja `pattern/LaneShuffleModifier.java` 의 정확한 라인번호(파일 존재·클래스명만 확인).
- `crates/rbms-model/src/mode.rs:20-21` 의 10K/14K 정의(직접 미열람, finding 05 인용 신뢰).
- IIDX 31~34 1차 출처(finding 14와 동일 사유).
- judge algorithm(beatoraja `play/JudgeAlgorithm.java`) 대응 여부 미확인.
