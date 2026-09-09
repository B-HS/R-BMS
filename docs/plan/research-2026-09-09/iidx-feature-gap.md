# IIDX(최신작 기준) 플레이/UI 기능 카탈로그 vs rbms 현 상태

조사일 2026-09-09. 코드 기준: /Users/gkn/R-BMS (dev, c6f0885), <reference>/.
읽기 전용 조사. 모든 상태 판정은 코드 근거(파일:라인)로만 기재했고, 코드로 확인 못 한 것은 "미확인"으로 표기했다.

## 0. 웹 조사 결과와 한계

IIDX 31~34의 신규 기능은 웹에서 1차 출처 수준으로 확보하지 못했다. 검색으로 확인된 것은
IIDX 32 Pinky Crush 가 1~4인 로컬/온라인 멀티, ARENA MODE(2024-12-19 복귀) 를 포함한다는 정도다
(출처: https://remywiki.com/AC_Pinky_Crush , https://beatmania.fandom.com/wiki/Beatmania_IIDX_32_Pinky_Crush ,
https://en.wikipedia.org/wiki/Beatmania_IIDX). 판정 표시 세분화·스킨 커스터마이즈·그래프 옵션 변화 등
"31~34 신규 항목"은 **미확인**으로 남긴다. 아래 카탈로그의 항목별 IIDX 사양은 대체로 IIDX 20~30대에
정착된 공통 사양 기준이며, 그 부분은 코드 대조가 목적이므로 유효하다.

## 1. 플레이 옵션 — 노트 배치(RANDOM 계열)

| IIDX 항목 | rbms 상태 | rbms 근거 | 레퍼런스 구현 참조 |
|---|---|---|---|
| OFF | 구현 | crates/rbms-chart/src/shuffle.rs:7,31 | pattern/PatternModifier.java |
| MIRROR | 구현 | shuffle.rs:8,115 | LaneShuffleModifier.java:112 LaneMirrorShuffleModifier |
| RANDOM | 구현 | shuffle.rs:9,121 | LaneShuffleModifier.java:148 LaneRandomShuffleModifier |
| R-RANDOM | 구현 | shuffle.rs:11,117 | LaneShuffleModifier.java:128 LaneRotateShuffleModifier |
| S-RANDOM | 구현(행 단위) | shuffle.rs:10,141 apply_srandom | pattern/NoteShuffleModifier.java |
| H-RANDOM | 구현(125ms anti-jack) | shuffle.rs:13,58-60,142 | NoteShuffleModifier.java |
| ALL-SCRATCH | 구현(40ms 스크래치 window) | shuffle.rs:14,62-65,143 | NoteShuffleModifier.java |
| ROTATE(=레퍼런스 구현 고유) | 구현 | shuffle.rs:12,116 | LaneRotateShuffleModifier |
| **FLIP (DP 좌우 교체)** | **미구현** — 코드 주석이 명시적으로 미구현이라 적음 | shuffle.rs:95-97 "that crossing is a separate FLIP option in 레퍼런스 구현, not implemented here" | LaneShuffleModifier.java:168 PlayerFlipModifier |
| **BATTLE (SP 보면을 DP 양쪽에)** | **미구현** | shuffle.rs 전체에 Battle 없음(grep 0건) | LaneShuffleModifier.java:186 PlayerBattleModifier |
| **SYNC/SYMMETRY-RANDOM (DP 양측 연동 랜덤)** | **미구현** — 셔플은 항상 side 단위 독립 시드 | shuffle.rs:106-112 (side 별 `Rng::new(seed+side)`) | LaneShuffleModifier.java:206 LaneCrossShuffleModifier, :225 LanePlayableRandomShuffleModifier |
| SP 옵션을 DP 각 side 에 적용 | 구현(side 별로 독립 적용) | shuffle.rs:89-94 side_key_lanes | — |

DP 자체(BEAT_10K/14K 모드)는 모델 레벨에 존재한다(crates/rbms-model/src/mode.rs:20-21). 다만 DP 전용
플레이 옵션(FLIP/BATTLE/SYNC)이 없어 DP 실사용 옵션 세트는 SP 옵션의 side 별 적용에 머문다.

## 2. ASSIST 옵션

| IIDX 항목 | rbms 상태 | rbms 근거 | 레퍼런스 구현 참조 |
|---|---|---|---|
| AUTO SCRATCH | 구현 | apps/rbms-player/src/app_select.rs:896 (`SCRATCH AUTO`), settings.rs:19 `scratch_auto` | pattern/AutoplayModifier.java:19 |
| **LEGACY NOTE (CN/HCN → 일반 노트)** | **미구현** | LongNote 변환 modifier 없음(shuffle.rs 는 lane 재배치만) | pattern/LongNoteModifier.java:25,73 enum Mode |
| **5KEYS (7K→5K 변환)** | **미구현** | 모드 변환 코드 없음 | pattern/ModeModifier.java:32,98 SEVEN_TO_NINE 등 |
| **A-SCRATCH(스크래치 판정 완화)** | **미구현/미확인** | 해당 설정 없음 | 미확인 |
| AUTOPLAY(전자동) | 구현 | app_select.rs:891 `AUTOPLAY`, settings.rs:20 | pattern/AutoplayModifier.java |
| 스크래치 좌/우 side | 구현 | app_select.rs:895 `SCRATCH SIDE` | play/LaneProperty.java |

## 3. GAUGE

| IIDX 항목 | rbms 상태 | rbms 근거 |
|---|---|---|
| ASSISTED EASY / EASY / NORMAL / HARD / EX-HARD / HAZARD | 6종 전부 구현 | crates/rbms-judge/src/gauge.rs:4-12 |
| 클리어램프 FAILED/ASSIST/EASY/NORMAL/HARD/EX-HARD/FULL COMBO(+PERFECT/MAX) | 구현 | gauge.rs:14-25 ClearType, apps/rbms-player/src/format.rs:139-148 |
| TOTAL 오버라이드 | 구현(IIDX 에는 없는 BMS 고유) | app_select.rs:903 `TOTAL` |
| **게이지 자동 폴백(HARD 실패 시 EASY 로 이어서 표시 등)** | **미구현** | gauge.rs 는 단일 kind 만 유지(gauge.rs:63-74), 보조 게이지 트랙 없음 |
| **ARENA / BPL BATTLE 용 게이지** | 미구현 (온라인 대전 모드 자체 없음) | — |

## 4. 레인 표시 / 하이스피드

| IIDX 항목 | rbms 상태 | rbms 근거 |
|---|---|---|
| SUDDEN+ (레인 커버) | 구현, 0~90% | crates/rbms-render/src/playfield.rs:166-167, app_select.rs:894 |
| LIFT | 구현, 0~90% | app_select.rs:893, apps/rbms-player/src/app_input.rs:97-101 |
| **HIDDEN+** | **미구현** — 하단 커버 렌더 자체가 없음 | playfield.rs 에 hidden 렌더 없음(`hidden` 은 rbms-model 의 hidden 노트로 무관, crates/rbms-model/src/lib.rs:52) |
| **SUD+ & HID+ 동시** | **미구현** (HIDDEN+ 부재의 종속) | — |
| 플레이 중 SUD+/LIFT/하이스피드 실시간 조절 | 구현 | app_input.rs:92-101 |
| 그린넘버 표시 | 구현(BPM/커버 반영, 매 프레임 재계산) | crates/rbms-render/src/hud.rs:17-19,89-91, apps/rbms-player/src/app_play.rs:294-297,551-555 |
| **화이트넘버(하이스피드 값) HUD 표시** | **미구현** — HUD 좌측은 EX/BEST/GREEN 만 | hud.rs:84-91 |
| HI-SPEED 소수 단위 | 부분 — 0.25 스텝 고정(0.5~10.0) | app_select.rs:920, app_input.rs:92-93 |
| **FLOATING HI-SPEED (그린넘버 고정 → 곡별 하이스피드 자동)** | **미구현** — `SPEED FIX` 는 CONSTANT(BPM 변화 무시) 토글이지 그린넘버 고정이 아님 | app_select.rs:892, app_play.rs:294-297 |

## 5. 타이밍 / 판정

| IIDX 항목 | rbms 상태 | rbms 근거 |
|---|---|---|
| 판정 오프셋 | 구현(±200ms, 5ms 스텝) | app_select.rs:899,933 |
| 오프셋 자동 보정 | 구현(IIDX 에 없는 rbms 확장) | app_select.rs:905 `AUTO CAL`, apps/rbms-player/src/main.rs:214-216 |
| FAST/SLOW 표시(직전 노트) | 구현 | hud.rs:11,101-102 |
| FAST/SLOW 누적 카운터 | 구현 | hud.rs:12-13,126-128 |
| 판정 폭 조절(EXPANDED JUDGE 계열) | 부분 — `JUDGE WIDTH` 50~200% 배율 (IIDX 의 이산 EXPANDED JUDGE 와는 다른 모델) | app_select.rs:902,936, crates/rbms-judge/src/windows.rs:77 `scaled` |
| 판정 4단(PGREAT/GREAT/GOOD/BAD/POOR) | 구현 | windows.rs:84-96 |
| **판정 타이밍 분포 그래프(±ms 히스토그램)** | **미구현** — 평균 오차(avgjudge)만 집계 | crates/rbms-judge/src/matcher.rs:362-390 (합/카운트만, 버킷 없음) |

## 6. 표시 옵션 / 스킨

| IIDX 항목 | rbms 상태 | rbms 근거 |
|---|---|---|
| 스킨 전환 | 부분 — NORMAL/WIDE 토글 + 커스텀 경로 | app_select.rs:900, :960-963 |
| **노트 스킨 / 판정 문자 스킨 개별 선택** | **미구현** — 색/라벨은 skin 구조체 고정 | crates/rbms-render/src/hud.rs:99-100 (`skin.judge_colors`, `skin.judge_labels`) |
| 키빔 | 미확인 (playfield 에서 확인 못 함) | — |
| 노트 폭발(bomb) | 구현 | playfield.rs:619-642 |
| 레인 커버 디자인 선택 | 미구현 — 커버는 배경색 단색 | playfield.rs:166-167 |
| BGA on/off | 구현 | app_select.rs:900(`BGA` idx10) |
| **스코어 그래프 target 선택(MY BEST/RIVAL 1~5/AAA/AA/A/RANK NEXT)** | **미구현** — 그래프는 A/AA/AAA 밴드 + 로컬 BEST 고정 | hud.rs:26-29,106-111; 설정 목록에 target 항목 없음(app_select.rs:888-916) |
| PACEMAKER 그래프 유형 선택 | 미구현 | 동상 |
| 스코어 그래프 on/off | 구현 | app_select.rs:907 `SCORE GRAPH` |

레퍼런스 구현 는 target 을 이미 완비: play/TargetProperty.java:119-137 (RANK A- ~ MAX-), :167-184 (RIVAL 1~n),
:69 (RANK_NEXT). rbms 는 이 개념 자체가 설정에 없다.

## 7. 모드 (EXPERT / DAN / STEP UP / ARENA / PREMIUM FREE)

| IIDX 항목 | rbms 상태 | rbms 근거 |
|---|---|---|
| **코스(EXPERT) / 단위인정(DAN)** | **미구현(플레이/선곡)** — IR DTO 에만 코스 개념 존재 | crates/rbms-ir/src/lib.rs:54,60, crates/rbms-ir/src/dto.rs:238-240 (`CourseSubmission`) 반면 apps/rbms-player 에 코스 스테이지 없음(main.rs:483 Stage 목록) |
| STEP UP | 미구현 | — |
| ARENA / BPL BATTLE (온라인 대전) | 미구현 | — |
| PREMIUM FREE (세션 제한 없음) | 해당 없음(가정용 클라이언트라 항시 프리) | — |
| 연습 모드(레퍼런스 구현 PRACTICE) | 미구현 | 레퍼런스 구현 play/PracticeConfiguration.java:23,52,242 |

## 8. 곡 선택 화면

| IIDX 항목 | rbms 상태 | rbms 근거 |
|---|---|---|
| 정렬 | 부분 — DEFAULT/TITLE/ARTIST/LEVEL/CLEAR 5종 (F3 순환) | apps/rbms-player/src/main.rs:543-566, app_select.rs:47-51 |
| 검색 | 구현(`/`, 전 라이브러리 flat) | app_select.rs:24-45 |
| 폴더 탐색 | 구현(루트/난이도표/레벨) | main.rs:523-531 SelectView |
| 난이도표(커스텀 폴더) | 구현 | main.rs:527-531, apps/rbms-player/src/tables.rs |
| **즐겨찾기 / 마킹** | **미구현** | 설정·선곡 코드에 favorite 개념 없음(grep 0건) |
| **난이도 필터(레벨 범위/클리어 상태 필터)** | **미구현** — 정렬만 있고 필터 없음 | main.rs:543-566 (SortMode 뿐) vs 레퍼런스 구현 select/DifficultyFilter.java, select/ModeFilter.java |
| 노트수·BPM 범위·CN/BSS 표기 | 부분 — stats 셀에 값 노출 | crates/rbms-render/src/select.rs:91 `stats: Vec<StatCell>` (구체 항목은 미확인) |
| 클리어램프 표시 | 구현(10단계) | select.rs:31 `lamp`, format.rs:139-148 |
| 밀도 그래프 | 구현(IIDX 에는 없는 레퍼런스 구현 계열 기능) | select.rs:37-41 DensityView |
| 미리듣기(#PREVIEW) | 구현 + 토글 | app_select.rs:909 `PREVIEW`, settings.rs:31 |
| **시작 전 옵션 패널(START 홀드)** | **미구현** — 옵션은 SETTINGS 탭에서만 | main.rs:599-606 SETTING_TABS, 선곡 화면에 옵션 오버레이 없음 |

## 9. 리절트

| IIDX 항목 | rbms 상태 | rbms 근거 |
|---|---|---|
| DJ LEVEL AAA~F | 구현(9분위 경계) | crates/rbms-render/src/result.rs:29-53 |
| EX SCORE | 구현 | result.rs:8,127 |
| 클리어 램프 | 구현 | result.rs:95-101, format.rs:139-148 |
| 랭크 바 + 자기베스트 델타 | 구현 | result.rs:58-70,103-120 |
| PGREAT/GREAT/GOOD/BAD/POOR 분류 | 구현 | result.rs:84-86 주석, ResultView counts |
| FAST/SLOW | 구현 | result.rs:84-86 |
| MISS COUNT(BP) | 구현(선곡 상세엔 명시) | select.rs:56 `bp` |
| **MAX- 표기(만점 대비 델타)** | **미구현** — 밴드는 AAA 가 상한 | result.rs:38 RANK_BANDS 최상단이 "AAA" |
| **게이지 추이 그래프** | **미구현** | result.rs 에 시계열 그래프 없음 | 레퍼런스 구현 result/SkinGaugeGraphObject.java |
| **판정 분포 그래프** | **미구현** | §5 참조 | 레퍼런스 구현 select/SkinDistributionGraph.java |
| **라이벌/타깃 비교** | **미구현** | target 개념 부재(§6) | 레퍼런스 구현 play/TargetProperty.java |

## 10. 문서 vs 코드

이번 조사 범위에서 docs/PROCESS.md 와 코드가 어긋나는 항목은 확인하지 못했다(PROCESS.md 에
COURSE/DAN/BATTLE/FLIP/PRACTICE 키워드 자체가 없다 — grep 0건). 즉 미구현 항목이 문서에 과장 기재되어
있지는 않다.

## 11. 미조사 범위

- IIDX 31~34 신규 기능의 1차 출처 확인(웹 5분 상한으로 미달성).
- 키빔 렌더 유무, select stats 셀의 구체 항목(노트수/BPM/CN·BSS 표기) 세부.
- rbms-render/src/skin.rs, theme.rs 의 커스터마이즈 범위.
- apps/rbms-player/src/replay.rs, scores.rs, ir_map.rs 전체.
- 레퍼런스 구현 select/bar/*, skin/* 상세.
