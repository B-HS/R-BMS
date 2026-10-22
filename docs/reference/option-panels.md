# 선곡 옵션 패널 조사 (START, SELECT, 상세 패널과 플레이 옵션 전체)

> 최종 갱신 2026-10-22 · 대응 커밋 47a7f9c

조사로 시작해 구현을 따라 고친 살아 있는 문서다. 1절(현재 구현)과 6절(결정)은 구현 뒤의 코드 기준이고, 2~5절의 조사(원본, IIDX 이미지, Lua 가능성)는 조사 당시 그대로다. 구현으로 달라진 곳은 그 절 머리나 해당 줄에 적었다. 문서와 코드가 다르면 코드를 믿었다. 근거는 저장소 루트 기준 `파일:줄` 이다. "직접 확인"은 이 조사에서 실행하거나 화면으로 본 것, "읽기만"은 코드를 읽어 추론한 것이다.

구현(2026-10-22): `7746932`(키 표, 확장 id, F1 홀드 키, IR 등급), `0c450db`(기본 스킨 IIDX 식 패널), `3f058f4`(패널 수리 3건). 결정의 근거는 ADR [옵션 패널 배치](../acknowledge/2026-10-22-adr-option-panel-layout.md)와 [R-BMS 확장 id](../acknowledge/2026-10-22-adr-rbms-extension-ids.md). 결정은 모두 사용자 지시 "끝까지 다 구현해놔" 에 따라 메인이 추천안으로 정했고 사용자는 아직 확인하지 않았다(6절 각 결정의 "결정됨" 줄).

목차: 0 요약 / 1 현재 구현 / 2 플레이 옵션 전체와 바꾸는 곳 / 3 beatoraja 원본 / 4 IIDX 참고 이미지 / 5 기본 스킨 Lua 로 구현할 때 / 6 결정 지점 / 7 검증 기록

---

## 0. 요약

1. 선곡 옵션 패널은 열고 닫는 화면이 아니라 키를 누르고 있는 동안만 올라오는 구조다(원본과 같다). START 단독이 패널 1, SELECT 단독이 패널 2, 둘을 함께 누르거나 상세 키(숫자 5)를 누르고 있으면 패널 3. 기본 키는 `A`, `W`, `5`.
2. 칸은 화면 요소가 아니라 건반 인덱스(0~8)에 묶인 역할이다. 키 표는 원본 `MusicSelectKeyProperty` 에서 `NEXT_REPLAY` 를 빼고, 일을 하지 않던 칸과 키가 없던 역할에 R-BMS 토글을 배정한 것이다. 원본과 다른 칸은 10곳이고 테스트가 목록으로 고정한다(`stage/select/keys.rs:98-125`, `CHANGED_SLOTS` `keys.rs:391`).
3. 값을 바꾸는 것: 패널 1은 RANDOM(키 1·2·7), GAUGE 다음(3)·이전(4), HI-SPEED FIX 다음(5)·이전(6), TARGET(턴테이블·휠·방향키). 패널 2는 CONSTANT(2), SCRATCH AUTO(3), LEGACY NOTE(4), KEEP GREEN NUMBER(5), LANE COVER(6), LIFT(7), HIDDEN+(턴테이블 어느 방향이나). 패널 3은 BGA(1), GAUGE AUTO SHIFT(2), AUTO ADJUST(3), NOTE DISPLAY TIME(4·6), JUDGE TIMING(5·7), JUDGE ALGORITHM(턴테이블: 정방향 이전, 역방향 다음).
4. 죽은 칸은 패널 2의 EX JUDGE(키 1, 읽기 전용) 하나다. DP OPTION 은 패널 키에서 뺐다(값 `play.lane_option`, 설정 행, 스킨 이벤트 54, 인덱스 54 응답은 그대로이고 플레이는 아직 읽지 않는다).
5. 원본에 id 가 없는 SCRATCH AUTO 와 KEEP GREEN NUMBER 는 R-BMS 확장 id 로 스킨이 읽고 바꾼다(이벤트 21000·21001, 불린 21100·21101, 3.4절). 확장 id 는 2026-10-09 "사설 id 철거" 결정의 부분 변경이다. 스킨 id 도 패널 키도 없는 것: SCRATCH SIDE, AUTOPLAY, TARGET FALLBACK·RATE, AUTO REPLAY, ESCAPE, TOTAL, JUDGE WIDTH, LN MARGIN. 스킨 id 와 이벤트는 있고 패널 키만 없는 것: LN MODE, BOTTOM SHIFTABLE, GUIDE SE.
6. Rust 가 Lua 에 값을 주는 길은 `SettingsView::image_index`(`apps/rbms-player/src/skin_host/options.rs:291`)와 Lua 의 `main_state.event_index(id)`, 확장 불린은 `SettingsView::boolean`(`options.rs:330`)과 `main_state.option(id)` 다. 원본 id 는 원본이 선언한 것만 쓰고, 확장은 21000 대에 둔다(3.4절).
7. 기본 스킨이 패널마다 IIDX 옵션 화면처럼 키 그림, 연결선, 값 전체를 세로로 나열한 상자, 고른 값의 주황 강조, INFORMATION 줄, 왼쪽 도움말을 그린다(1.8절). 못 하는 것은 차트에 의존하는 레인 미리보기뿐이다(5.2절).
8. 원본 키 표의 배열(흰 키 1·3·5·7, 검은 키 2·4·6)은 참고 이미지 3·4(IIDX)의 키 그림과 한 칸도 어긋남 없이 같다(4.5절).
9. F1 오버레이는 START, SELECT, 상세 키 중 하나가 눌려 있으면 홀드 키(`LSHIFT`)로 열리지 않는다(`apps/rbms-player/src/app_options.rs:204-226`). 패널 1의 턴테이블 TARGET 과 패널 2·3의 턴테이블 역할이 오버레이와 부딪히지 않는다. 핀 키 F1 은 그대로다.
10. 해상도: 기본 스킨 팩은 1920x1080 으로 작성되고(`assets/skins/rbms-default/lua/theme.lua:5-6`) 기본 창은 1280x720(`crates/rbms-config/src/window.rs`)이라 2/3 배로 줄어 그려진다. 앱 내장 화면(F1 오버레이, PLAY 메뉴 등)만 1280x720 고정 좌표다(`apps/rbms-player/src/lib.rs:149-150`).
11. 테스트: 패널을 그린 픽셀을 비교하는 골든은 여전히 없다. 키 표(`keys.rs` 테스트), 패널 키(`panel_tests.rs`), 확장 id(`extension.rs`), 엄격 덤프 게이트와 금지 API 검사가 코드와 기본 스킨 Lua 를 지킨다(5.6절). 패널 캡처는 옵트인 하네스 `stage/capture/panels.rs` 가 만든다.
12. 알려진 위험(수리하지 않음): 키 표 변경은 모든 스킨에 적용되므로, 확장 id 를 모르는 외부 스킨 위에서 패널 2의 죽었던 칸 키가 표시 없이 저장 설정을 바꾼다(1.10절 14).

## 1. 현재 구현

### 1.1 여는 입력 경로

| 패널 | 누르고 있는 것 | 기본 키 | 컨트롤러 | 근거 |
| --- | --- | --- | --- | --- |
| 1 PLAY OPTIONS | START 단독 | `A` | 패드의 START | `stage/select/keys.rs:184-192`, `keyconfig.rs:582`, `app_input.rs:285` |
| 2 ASSIST OPTIONS | SELECT 단독 | `W` | 패드의 SELECT | `keyconfig.rs:585`, `app_input.rs:290` |
| 3 DETAIL OPTIONS | START 와 SELECT 함께, 또는 상세 키(START·SELECT 가 모두 안 눌렸을 때) | `A`+`W`, 또는 `5`(`Shortcut::RefDetail`, 키 설정에서 변경 가능) | START+SELECT. 상세 키는 키보드 전용(`takes_a_pad` false) | `keyconfig.rs:928`, `keyconfig.rs:946-948`, `app_input.rs:340` |

- 판정 순서는 START 단독, SELECT 단독, 둘 또는 상세 키다. `Panel::held(true, false, true)` 는 Play 다(START 가 상세 키보다 먼저). `keys.rs:184-192`, 테스트 `keys.rs:345-352`.
- 패널은 래치가 아니다. 키를 떼면 닫힌다. 원본도 같다(`MusicSelectInputProcessor.java:62-330`).
- 열리지 않는 때: 검색어를 치는 중(`shared.searching`), 기록 모달·필터 패널·키 가이드(H)·IR 랭킹 패널이 떠 있을 때(`stage/select/skinned.rs:138-145` 의 `list_is_covered`, `skinned.rs:259-262` 의 `Panel::held` 호출), 앱 오버레이 F1 이 열려 있을 때도 같다(`ctx.shared.options.is_open()`).
- 패널 키가 눌린 뒤에는 그 키의 "눌림 변화"를 패널이 먹는다. 목록 쪽 역할(PLAY 등)로 새어 가지 않는다(`keys.rs:279-284`). 단 패널 3의 NOTE DISPLAY TIME 두 키(4·6)는 눌림이 아니라 "누르고 있음"으로 읽혀 목록에 눌림이 남는다(`keys.rs:121-131` 주석).
- 앱 전체 키 처리 순서: `Stage::handle_key`(`stage/mod.rs:317-326`)가 먼저 `note_key` 로 눌린 키 집합을 갱신한 뒤 F1 오버레이(`options_key`)에 키를 보이고, 그 다음에 선곡이 받는다. START·SELECT·턴테이블 판독은 눌린 키 집합을 읽으므로 오버레이가 키를 먹어도 영향을 받지 않는다(읽기만). 다만 선곡의 `skinned_key` 가 받는 "한 프레임보다 짧은 눌림" 기록은 오버레이가 먹은 키에는 남지 않는다.

건반 번호(키 인덱스)와 7키 기본 키보드 배치(`keyconfig.rs:285-326`, `keyconfig.rs:1055-1084`):

| 인덱스 | 이름 | 7키 기본 키 | 비고 |
| --- | --- | --- | --- |
| 0~6 | 건반 1~7 | `Z` `S` `X` `D` `C` `F` `V` | 5키 모드는 1~5 만(6·7 없음), 10키·14키는 2P 쪽이 같은 인덱스로 접힘 |
| 7 | 턴테이블 정방향 | `LSHIFT` | 목록에서 `Up`, 패널 1에서 `TargetUp` |
| 8 | 턴테이블 역방향 | `LCTRL` | 목록에서 `Down`, 패널 1에서 `TargetDown` |

9키(`POPN_9K`)는 턴테이블이 없어 8·9번 버튼이 인덱스 7·8 이고 별도 표를 쓴다(`keys.rs:89-99`). 패널에서 같은 인덱스는 같은 패널 역할을 한다.

### 1.2 패널 상태, 타이머, 옵션 id

- 상태: `PanelState { open, key_pressed, key_released, target_repeat_at_ms, duration_repeat_at_ms, duration_repeats }`(`stage/select/panel.rs:174-`). 열린 패널은 `Panel::{Closed, Play, Assist, Detail}`(`keys.rs:194`).
- 스킨에 알리는 값: 옵션 21·22·23(`OPTION_PANEL1..3`)이 열린 패널을 답한다(`PanelState::number`, `panel.rs:204-`, `skin_host/select.rs:320`, 전달은 `skin_host/select/bridge.rs:39`).
- 타이머: 열리는 패널의 도착 타이머를 켜고 닫는 패널의 출발 타이머를 켠다. 도착 `PANEL1_ON` 21, `PANEL2_ON` 22, `PANEL3_ON` 23, 출발 `PANEL1_OFF` 31, `PANEL2_OFF` 32, `PANEL3_OFF` 33(`panel.rs:56-61`, `lua/props.lua:29-34`). 패널에서 패널로 곧장 넘어가면 앞 패널의 출발과 뒤 패널의 도착이 같은 프레임에 켜진다(`show_panel`, `panel.rs:300-`).
- 소리: 패널 키가 하나라도 눌린 순간 `OptionOpen` 한 번, 모두 뗀 순간 `OptionClose` 한 번, 값이 바뀔 때마다 `OptionChange`(TARGET 은 제외). 패널 사이 전환은 무음(`panel.rs:216-`, `run_setting_event` `events.rs:365-`, `run_panel_act` `panel.rs:353-`).
- 패널이 올라와 있는 동안 목록은 멈춘다(`skinned.rs:252-285`의 `skin_input_frame`: 패널이 열리면 `panel_keys` 만 돌고 목록 입력은 안 읽는다).
- 값 저장: 값이 바뀌면 `options_dirty` 를 켜고, 패널이 모두 닫힌 프레임에 한 번 `rebuild_skin` + `save_settings` 한다(`settle_option_changes` `events.rs:414-`, 호출 `mod.rs:1081`). 선곡을 떠날 때도 쓴다(`mod.rs:1106`). 키를 누르고 있는 동안은 파일을 쓰지 않는다.

### 1.3 키 표: 인덱스와 역할

원본 표에서 `NEXT_REPLAY` 를 빼고 R-BMS 토글을 배정한 표다(`keys.rs:98-111`, 표 위 문서 `keys.rs:76-97`). 원본에서 바뀐 칸은 `CHANGED_SLOTS`(`keys.rs:391-402`)로 고정되고 그 밖의 칸이 원본과 같다는 것도 단언한다. 아래는 7키/14키(`BEAT_TABLE`)이고, 원본에서 바뀐 칸에 `*` 를 붙였다.

| 인덱스 (7키 키) | 목록 역할 | 패널 1 (START) | 패널 2 (SELECT) | 패널 3 (START+SELECT 또는 5) |
| --- | --- | --- | --- | --- |
| 0 (`Z`) | Play, FolderOpen | `Option1Down`: RANDOM 다음 | `JudgeWindowUp`: EX JUDGE(무동작) | `BgaDown`: BGA 뒤집기 |
| 1 (`S`) | FolderClose | `Option1Up`: RANDOM 이전 | `Constant`: CONSTANT 뒤집기 | `GaugeAutoShiftDown`: GAS 다음 |
| 2 (`X`) | Practice, FolderOpen | `GaugeDown`: GAUGE 다음 | `ScratchAuto`*: SCRATCH AUTO 켬끔(원본 `JUDGEAREA` 자리) | `NotesDisplayTimingAutoAdjust`: AUTO ADJUST 뒤집기 |
| 3 (`D`) | FolderClose | `GaugeUp`*: GAUGE 이전(원본 `OPTIONDP_DOWN` 자리) | `LegacyNote`: LEGACY NOTE 뒤집기 | `DurationDown`: NOTE DISPLAY TIME 줄임(누르고 있음으로 읽음, 반복) |
| 4 (`C`) | FolderOpen, Auto | `HsFixDown`: HI-SPEED FIX 다음 | `KeepGreenNumber`*: KEEP GREEN NUMBER 켬끔(원본 `MARKNOTE` 자리) | `NotesDisplayTimingDown`: JUDGE TIMING -1 ms |
| 5 (`F`) | 없음 | `HsFixUp`*: HI-SPEED FIX 이전(원본 `OPTION2_UP` 자리) | `LaneCover`*: LANE COVER 켬끔(원본 `BPMGUIDE` 자리) | `DurationUp`: NOTE DISPLAY TIME 늘림(반복) |
| 6 (`V`) | FolderOpen, Replay | `Option2Down`: RANDOM 다음(원본 그대로) | `Lift`*: LIFT 켬끔(원본 `NOMINE` 자리) | `NotesDisplayTimingUp`: JUDGE TIMING +1 ms |
| 7 (`LSHIFT`) | Up | `TargetUp`: 목표를 위로(이전 목표) | `Hidden`*: HIDDEN+ 켬끔 | `JudgeAlgorithmUp`*: JUDGE ALGORITHM 이전 |
| 8 (`LCTRL`) | Down | `TargetDown`: 목표를 아래로(다음 목표) | `Hidden`*: HIDDEN+ 켬끔 | `JudgeAlgorithmDown`*: JUDGE ALGORITHM 다음 |

- 바뀐 칸 10곳(`CHANGED_SLOTS`): 패널 2 키 3·5·6·7(`JUDGEAREA`, `MARKNOTE`, `BPMGUIDE`, `NOMINE`)이 SCRATCH AUTO, KEEP GREEN NUMBER, LANE COVER, LIFT 로, 패널 1 키 4(`OPTIONDP_DOWN`)가 GAUGE 이전으로, 패널 1 키 6(`OPTION2_UP`)이 HI-SPEED FIX 이전으로, 패널 2 턴테이블 두 방향이 HIDDEN+ 로, 패널 3 턴테이블 두 방향이 JUDGE ALGORITHM 으로. 지워진 `SelectKey` 변종은 `OptionDpUp/Down`, `Option2Up`, `JudgeArea`, `MarkNote`, `BpmGuide`, `NoMine` 이다.
- 패널 1 키 6 이 RANDOM 이전이 아니고 키 7 이 RANDOM 다음으로 남은 이유: 2P 쪽 RANDOM 은 독립 값이 없다. `option1p`/`option2p` 이벤트가 같은 `play.random` 하나를 만진다(`events.rs:83-85`, 키 1·2 가 이미 양방향). 그래서 키 6 은 키 2 와 완전 중복이라 HI-SPEED FIX 이전으로 돌렸고, 키 7 은 "고유 기능을 가진 기존 역할" 이 남지 않아 원본 역할(`OPTION2_DOWN`)을 그대로 두었다(실제로 동작하고 원본·ModernChic 의 키 7 라벨 RANDOM 2P 와 어긋나지 않는다).
- 패널 1의 GAUGE 와 HI-SPEED FIX 는 이제 다음·이전 양쪽 키가 있다. GAS 와 BGA 는 여전히 다음으로만 간다(`GaugeAutoShiftDown`, `BgaDown`).
- 패널 2의 턴테이블과 패널 3의 턴테이블은 "눌림" 으로 읽는다(패널 1의 TARGET 은 "누르고 있음"). 패널이 올라오기 전부터 눌려 있던 방향은 눌림이 아니다(`forget_turntable_changes`, `keys.rs:298`, 호출 `panel.rs:307`).
- DP OPTION 역할(`OptionDpUp/Down`)은 `SelectKey` 에서 지워져 패널 키로 바꿀 수 없다(1.10절 1).
- 패널 1에서 목표를 움직이는 것은 턴테이블 외에 방향키 `ArrowDown`=TargetUp, `ArrowUp`=TargetDown(`panel.rs:53-54`), 마우스 휠(`take_wheel`, `skinned.rs:232`). 처음 누르면 한 칸, 300 ms 뒤부터 50 ms 마다 한 칸 반복(`SCROLL_DURATION_LOW_MS` 300, `SCROLL_DURATION_HIGH_MS` 50. `crates/rbms-render/src/skin_render/songlist/bars.rs:104-109`).
- 9키 표(`POPN_TABLE`, `keys.rs:113-125`)는 같은 인덱스에 같은 패널 역할을 둔다(8·9번 버튼이 인덱스 7·8). 목록 역할만 다르다(인덱스 0 `Auto`, 4 `Play`, 8 `Replay`).
- 5키·10키 키보드에는 6·7번 건반(인덱스 5·6)이 없다(`keyconfig.rs:1071-1084`의 `key_index_of`). 그 모드에서 닿지 않는 역할: 패널 1 HI-SPEED FIX 이전(6)·RANDOM 다음(7), 패널 2 LANE COVER(6)·LIFT(7), 패널 3 NOTE DISPLAY TIME 늘림(6)·JUDGE TIMING +1(7). 동작은 바꾸지 않았고 기본 스킨이 그 키를 어둡게 그린다(1.8절).

### 1.4 패널별 슬롯, 값 목록, 순환 방향, 실제 동작

값 순환은 모두 끝에서 처음으로 돌아가고(`stepped`, `settings.rs:1165`), 값이 안 바뀌면(범위 끝 등) 소리가 나지 않는다. 키를 눌러 바꾼 값은 모두 `options_dirty` 를 켜고 패널이 닫힐 때 저장된다.

패널 1 PLAY OPTIONS

| 슬롯 | 역할 → 이벤트 | 값 목록(순서) | 방향 | 실제 동작 |
| --- | --- | --- | --- | --- |
| 1·7 | `Option1Down`·`Option2Down` → `option1p`/`option2p`(42/43) | OFF, MIRROR, RANDOM, R-RANDOM, S-RANDOM, ROTATE, H-RANDOM, ALL-SCRATCH (8개, 원본 번호 순 `OPTION_GENERAL`, `skin_host/options.rs:73-85`) | 다음 | 1P 와 2P 이벤트가 같은 값 `play.random` 하나를 만진다(`events.rs:83-85`, `196`). 차트 양쪽에 같은 옵션이 적용된다 |
| 2 | `Option1Up` | 위와 같음 | 이전 | 위와 같음 |
| 3 | `GaugeDown` → `gauge1p`(40) | ASSIST EASY, EASY, NORMAL, HARD, EX HARD, HAZARD (6개 `GAUGE_CYCLE`, `settings.rs:144`) | 다음 | `play.gauge` |
| 4 | `GaugeUp` → `gauge1p`(40) | 위와 같음 | 이전 | `play.gauge` |
| 5 | `HsFixDown` → `hsfix`(55) | OFF, START, MAX, MAIN, MIN (5개) | 다음 | `play.fix_hispeed`. 커서가 폴더이거나 못 치는 코스일 때의 동작은 1.10절 |
| 6 | `HsFixUp` → `hsfix`(55) | 위와 같음 | 이전 | 위와 같음 |
| 턴테이블(인덱스 7·8) | `TargetUp`·`TargetDown`, 방향키, 휠 → `target`(77) | MAX, RATE A-, A, A+, AA-, AA, AA+, AAA-, AAA, AAA+, MAX-, RANK NEXT, RATE CUSTOM, LOCAL BEST, IR BEST, RIVAL (16개 `ScoreTarget::ALL`, `crates/rbms-config/src/judge.rs:84-101`, 라벨 `judge.rs:199-216`) | 정방향은 이전, 역방향은 다음. 휠 노치 수만큼 | `judge.target`. 한 칸마다 `Scratch` 소리, `OptionChange` 소리는 없다 |

DP OPTION(`optiondp`, 54)은 패널 키에서 뺐다. 값 4개(OFF, FLIP, BATTLE, BATTLE AUTO-SC. `crates/rbms-config/src/options.rs:62-120`)와 이벤트·인덱스는 남아 있고 플레이에 적용되지 않는다.

패널 2 ASSIST OPTIONS

| 슬롯 | 역할 | 값 | 실제 동작 |
| --- | --- | --- | --- |
| 1 | `JudgeWindowUp` → `AssistSwitch::CustomJudge` | OFF/ON 표시만 | 뒤집을 설정이 없다(`panel.rs:87`). 표시되는 값은 JUDGE WIDTH 6개 중 하나나 LN MARGIN 이 100% 를 넘으면 켜짐인 읽기 전용 값이다(`skin_host/options.rs:190`, `apps/rbms-player/src/judge_setup.rs:19-22`). 누르면 아무 일도 없고 소리도 없다 |
| 2 | `Constant` → `AssistSwitch::Constant` | OFF/ON | `play.constant_speed` 뒤집기(`SettingId::SpeedFix`). FLOATING 과 CONSTANT 를 맞바꿈 |
| 3 | `ScratchAuto` → 확장 이벤트 21000 | OFF/ON | `play.scratch_auto` 뒤집기(`SettingId::ScratchAuto`). 전역 설정이라 코스 위에서도 막지 않는다 |
| 4 | `LegacyNote` | OFF/ON | `play.legacy_note` 뒤집기. 롱노트를 일반 노트로 그릴 뿐이다(화면 전용, 판정은 그대로: `stage/play/field.rs:52-82`) |
| 5 | `KeepGreenNumber` → 확장 이벤트 21001 | OFF/ON | `play.keep_green_number` 뒤집기. 전역 설정이라 코스 위에서도 막지 않는다 |
| 6 | `LaneCover` → 이벤트 330 | OFF/ON | `play.enable_cover` 뒤집기. 못 치는 코스 위에서 무동작 |
| 7 | `Lift` → 이벤트 331 | OFF/ON | `play.enable_lift` 뒤집기. 같음 |
| 턴테이블(인덱스 7·8, 어느 방향이나) | `Hidden` → 이벤트 332 | OFF/ON | `play.enable_hidden` 뒤집기. 같음 |

패널 3 DETAIL OPTIONS

| 슬롯 | 역할 → 이벤트 | 값 | 실제 동작 |
| --- | --- | --- | --- |
| 1 | `BgaDown` → `bga`(72) | ON(0) / OFF(2). AUTOPLAY ONLY(1)는 엔진이 만들 수 없다 | `display.bga` 뒤집기(`skin_host/options.rs:115-116`, `197`) |
| 2 | `GaugeAutoShiftDown` → `gaugeautoshift`(78) | NONE, CONTINUE, SURVIVAL TO GROOVE, BEST CLEAR, SELECT TO UNDER (5개 `GaugeAutoShift::ALL`) | 다음만. `judge.gauge_auto_shift` |
| 3 | `NotesDisplayTimingAutoAdjust` → 이벤트 75 | OFF/ON | `judge.auto_offset` 뒤집기 |
| 4·6 | `DurationDown`·`DurationUp` → `duration1p`(59) | 이동 시간 N MS, 녹색 숫자 | KEEP GREEN NUMBER 가 켜져 있고 커서가 곡·완성 코스 위일 때만 `play.duration_ms` 를 1 ms 씩(키 누르고 있으면 300 ms 뒤부터 50 ms 마다, 반복이 50번을 넘으면 10 ms 씩) 움직인다(`panel.rs:266-`, `events.rs:169`, `396-405`). 꺼져 있거나 커서가 폴더·표·실행 파일 위면 아무것도 안 한다. 범위 1~10,000 ms(`schema.rs:99-102`) |
| 5 | `NotesDisplayTimingDown` → `notesdisplaytiming`(74) | JUDGE TIMING | `judge.offset_ms` -1, -200 에서 멈춤 |
| 7 | `NotesDisplayTimingUp` | JUDGE TIMING | `judge.offset_ms` +1, +200 에서 멈춤. 키 한 번이 1 ms, 누르고 있어도 반복하지 않는다(눌림으로 읽음) |
| 턴테이블(인덱스 7·8) | `JudgeAlgorithmUp`(정방향, 이전)·`JudgeAlgorithmDown`(역방향, 다음) → `judgealgorithm`(340) | COMBO, DURATION, LOWEST 3개만 | `judge.judge_algorithm`. 엔진이 SCORE 인 상태면 순환하지 못한다. 못 치는 코스 위에서 무동작 |

### 1.5 값을 바꾸는 코드 경로

1. 프레임마다 `skin_input_frame`(`skinned.rs:252`)이 `Panel::held` 로 열린 패널을 정하고 `show_panel` 이 타이머를 켠다.
2. `panel_keys`(`panel.rs:323-`)가 `keys.panel_presses` 로 이번 프레임에 눌린 패널 역할을 모으고 `press_act`(`panel.rs:107-`)로 `PanelAct` 로 바꾼다. 패널 1은 목표 움직임(`target_acts`), 패널 3은 NOTE DISPLAY TIME 반복(`duration_moves`)을 더한다.
3. `run_panel_act`(`panel.rs:353-`)가 실행한다: `Setting` → `run_setting_event`(`events.rs:365-`), `Assist` → `adjust(config, row, NEXT)` 후 `OptionChange` 소리, `Duration` → `run_duration_event`(`events.rs:396-`).
4. `step_setting`(`events.rs:190-`)은 설정 화면의 행과 똑같이 `rbms_config::adjust` 로 한 칸 움직인다. 예외 넷은 RANDOM(원본 번호 순), JUDGE TIMING(1 ms 씩, 범위에서 멈춤), JUDGE ALGORITHM(원본이 번호를 매긴 셋만), NOTE DISPLAY TIME.
5. 스킨 단추나 스킨 Lua 의 `main_state.event_exec` 도 같은 `run_event_stepping`(`events.rs:336-`)을 탄다. 한 경로다.
6. `of_the_play` 로 분류된 이벤트(HI-SPEED FIX, HI-SPEED, 이동 시간, 레인 커버·리프트·히든 켬끔, JUDGE ALGORITHM)는 못 치는 코스(`SelectTab::Courses` 에서 차트가 모자란 코스) 위에서는 무동작이다(`events.rs:365-384`). SCRATCH AUTO 와 KEEP GREEN NUMBER 는 전역 설정이라 분류에 없다. 이동 시간은 이에 더해 커서가 곡·완성 코스(`FocusedBar::Playable`)가 아니어도 무동작이다(`events.rs:396-405`).

### 1.6 스킨 이벤트 id 표 (`stage/select/events.rs:250-`)

패널 키와 스킨 단추가 같은 이벤트를 쓴다. 숫자는 `lua/props.lua:282-311` 과 `crates/rbms-skin/src/property/generated/misc.rs` 의 값이다.

| id | 원본 이름 | R-BMS 동작 |
| --- | --- | --- |
| 11, 12, 312 | mode, sort, songbar_sort | 모드 필터 순환, 정렬 순환 |
| 13, 14 | keyconfig, skinconfig | 키 설정 화면, 스킨 메뉴 |
| 15, 16, 315 | play, autoplay, practice | 시작, 1회 오토플레이(설정은 그대로, `events.rs:429-445`), 연습 |
| 19, 316, 317, 318 | replay1~4 | 가장 새 리플레이 4개 중 슬롯 |
| 90 | favorite_chart | 별 켬끔 |
| 308 | lnmode | LN MODE 순환(목록 다시 만듦) |
| 40 | gauge1p | GAUGE 6개 순환 |
| 42, 43 | option1p, option2p | RANDOM(원본 번호 순 8개) |
| 54 | optiondp | DP OPTION(적용 없음, 패널 키 없음) |
| 55 | hsfix | HI-SPEED FIX |
| 57 | hispeed1p | HI-SPEED 를 HI-SPEED STEP(기본 0.25) 씩 |
| 59 | duration1p | 이동 시간(KEEP GREEN NUMBER 켜짐일 때만). 두 번째 인자가 걸음(0이면 1 ms) |
| 72 | bga | BGA 뒤집기 |
| 74 | notesdisplaytiming | JUDGE TIMING 1 ms 씩 |
| 75 | notesdisplaytimingautoadjust | AUTO ADJUST 뒤집기 |
| 77 | target | TARGET 순환(소리 없음) |
| 78 | gaugeautoshift | GAS 5개 순환 |
| 341 | bottomshiftablegauge | BOTTOM SHIFTABLE 3개 순환(패널 키 없음) |
| 330, 331, 332 | lanecover, lift, hidden | 켬·끔(`EnableCover`, `EnableLift`, `EnableHidden`). 패널 2 키 6·7·턴테이블 |
| 340 | judgealgorithm | COMBO, DURATION, LOWEST 3개만 순환. 엔진이 SCORE 인 상태면 순환 못 함. 패널 3 턴테이블 |
| 343 | guidese | GUIDE SE 뒤집기 |
| 352 | scrollmode | SPEED FIX(CONSTANT) 뒤집기 |
| 353 | longnotemode | LEGACY NOTE 뒤집기 |
| 21000 | R-BMS `RBMS_EVENT_SCRATCH_AUTO` | SCRATCH AUTO 켬끔(`events.rs:285`). 첫 인자가 어느 쪽이든 한 번 실행에 한 번 뒤집는다. 패널 2 키 3 |
| 21001 | R-BMS `RBMS_EVENT_KEEP_GREEN_NUMBER` | KEEP GREEN NUMBER 켬끔(`events.rs:286`). 같음. 패널 2 키 5 |

원본 이벤트인데 아무 일도 하지 않는 것(`events.rs:20-29`): 10, 17, 73, 79, 89, 210~213, 321~324, 342, 344, 350, 351, 360, 361, 400. 301~307(assist)은 원본에서도 이벤트가 아니다. id 가 아예 표에 없는 이벤트도 조용히 버려진다(`skin_host/mod.rs`의 `EVENT_ROUTES`, 21000~21001 은 `mod.rs:572`).

### 1.7 Rust 에서 Lua 로 값이 가는 길

- 설정 화면과 패널과 스킨이 같은 `Config` 필드를 읽고 쓴다. `SelectShown::of` 가 프레임마다 `SettingsView::of_config(config)` 를 만든다(`skin_host/select/shown.rs:205-`, `options.rs` 의 `of_config`). 커서가 폴더이거나 못 치는 코스에 있으면 `settings.without_chart()` 로 차트별 값(HI-SPEED FIX, HI-SPEED, 이동 시간)이 "없음"이 된다(`shown.rs:207`).
- 이미지 인덱스(Lua 의 `main_state.event_index(id)`, imageset 의 `ref`)로 읽는 것(`options.rs:291-`):

| id | 값 |
| --- | --- |
| 40 | GAUGE 0~5(`GaugeIndex`) |
| 42, 43 | RANDOM 0~7(원본 번호 순). 1P 와 2P 가 같은 값 |
| 54 | DP OPTION 0~3 |
| 55 | HI-SPEED FIX 0~4. 폴더 위에서는 -1(없음) |
| 61~63 | 없음(-1) |
| 72 | BGA 0(켬) 또는 2(끔) |
| 75 | AUTO ADJUST 0/1 |
| 78 | GAS 0~4 |
| 341 | BOTTOM SHIFTABLE 0~2 |
| 301 | 판정 폭을 넓힘 0/1 |
| 302, 352 | CONSTANT 0/1 |
| 304, 353 | LEGACY NOTE 0/1 |
| 308 | LN MODE 0~2 |
| 330, 331, 332 | 레인 커버, 리프트, 히든 켬 0/1 |
| 340 | JUDGE ALGORITHM 0~2. SCORE 이면 -1 |
| 343 | GUIDE SE 0/1 |
| 303, 305, 306, 307, 321~324, 342, 350, 351, 360, 361, 400 | 없음(-1): 대응 설정이 없다 |

- 숫자: 12(JUDGE TIMING), 10(HI-SPEED 의 LR2 방식 정수), 310·311(HI-SPEED 정수부·소수부), 실수 310(HI-SPEED), 312(이동 시간), 313(녹색 숫자). 312·313 은 KEEP GREEN NUMBER 가 꺼져 있으면 "없음"(`options.rs` 의 `integer`).
- 글자: 3(TARGET 이름, 데이터가 없으면 `LOCAL BEST (NO RECORD: AAA)` 꼴), 200~209(앞의 목표 10개), 210~219(뒤의 목표 10개)(`options.rs:372-` 의 `text`).
- 값 라우팅: `skin_host/mod.rs:265-` 의 `ROUTES` 에서 위 ids 는 `Cluster::Options`, 옵션 21~23 은 `Cluster::Select`. 확장 불린은 21100~21104 가 `Options`(`mod.rs:381`), 21110~21114·21140~21148 이 `Select`(`mod.rs:394-398`), 21120~21134 가 `Result`(`mod.rs:420-424`). 표 밖의 id 는 모든 클러스터에 물어본다(`mod.rs:253-257`).
- 반드시 알아야 할 게이트: `reference_implements`(`crates/rbms-skin/src/property/names.rs:363-375`)가 "원본이 이 id 를 가진다"를 판정하고, 이 함수를 8곳이 쓴다: Lua `number`·`float_number`·`event_index`(`lua/main_state.rs:859,868,927`), imageset·image 의 `ref`(`rbms-render/src/skin_render/draw.rs:300,312`), 글자 `ref`(`text.rs:191`), songlist(`songlist/build.rs:168`), 로더의 `op` 목록(`loader/track.rs:157`). 원본에 없는 id 는 `event_index` 가 Lua 오류를 던지고 imageset 은 첫 이미지를 항상 보인다.
- 게이트 밖의 두 길(종합 검토에서 코드로 확인): (1) 이벤트 공간은 `reference_implements(NameSpace::Event, _)` 가 항상 참이다(`names.rs:363-366`). 새 이벤트 id 는 게이트를 고칠 필요가 없고 `EVENT_ROUTES` 와 선곡 `EVENTS` 표에 줄만 더하면 된다. 원본이 사용자용으로 남긴 대역은 1000~1999(3.4절 표). (2) Lua 의 `main_state.option(id)`(불린)은 게이트를 거치지 않고 곧장 호스트에 묻는다(`lua/main_state.rs:851-853`, 호스트 `skin_host/mod.rs:923-926`). 그래서 켬·끔 두 값뿐인 새 옵션(SCRATCH AUTO, KEEP GREEN NUMBER)은 게이트 8곳을 고치지 않고도 "호스트 불린 응답 + Lua `draw` 함수"로 값을 그릴 수 있다. 구현은 이 길을 택했다(6절 결정 3, 확장 id 표는 3.4절). 원본에 없는 불린 id 를 쓰는 것이라 2026-10-09 사설 id 폐기 결정의 부분 변경이다. `op = {id}` 로 쓰면 로더 게이트(`track.rs:157`)에 걸려 버려지므로 `draw`/`value`/`text` 함수 안에서만 읽는다.

스킨이 패널 값을 읽는 방법(선곡 화면). 원본 id 로 읽는 것은 `main_state.event_index(id)`, 확장은 함수 필드 안의 `main_state.option(id)` 다.

| 옵션 | 읽는 방법 | 값 |
| --- | --- | --- |
| RANDOM | `event_index(42)` (43 도 같은 값) | 0 OFF, 1 MIRROR, 2 RANDOM, 3 R-RANDOM, 4 S-RANDOM, 5 ROTATE, 6 H-RANDOM, 7 ALL-SCRATCH |
| GAUGE | `event_index(40)` | 0 ASSIST EASY, 1 EASY, 2 NORMAL, 3 HARD, 4 EX HARD, 5 HAZARD |
| HI-SPEED FIX | `event_index(55)` | 0 OFF, 1 START, 2 MAX, 3 MAIN, 4 MIN. 폴더·못 치는 코스 위 -1 |
| TARGET | `main_state.text(3)` 현재 이름(기록이 없으면 `LOCAL BEST (NO RECORD: AAA)` 꼴), `text(200..209)` 앞 목표(200 = 10칸 앞 ... 209 = 1칸 앞), `text(210..219)` 뒤 목표(210 = 1칸 뒤 ... 219 = 10칸 뒤) | 이미지 인덱스 77 은 원본에 없다. `event_index(77)` 을 쓰면 Lua 오류 |
| CONSTANT | `event_index(302)` 또는 `(352)` | 0 끔, 1 켬 |
| LEGACY NOTE | `event_index(304)` 또는 `(353)` | 0/1 |
| EX JUDGE(읽기 전용) | `event_index(301)` | 0/1(판정 폭이나 LN MARGIN 이 넓혀짐) |
| BGA | `event_index(72)` | 0 ON, 2 OFF (1 은 나오지 않음) |
| GAS | `event_index(78)` | 0 NONE, 1 CONTINUE, 2 SURVIVAL TO GROOVE, 3 BEST CLEAR, 4 SELECT TO UNDER |
| AUTO ADJUST | `event_index(75)` | 0/1 |
| JUDGE ALGORITHM | `event_index(340)` | 0 COMBO, 1 DURATION, 2 LOWEST, -1 SCORE |
| NOTE DISPLAY TIME | `number(312)` ms, `number(313)` 녹색 숫자 | KEEP GREEN NUMBER 가 꺼져 있거나 커서가 차트가 아니면 `INTEGER_ABSENT`(`i32::MIN`) |
| JUDGE TIMING | `number(12)` | -200..+200 ms |
| LANE COVER·LIFT·HIDDEN+ 켬끔 | `event_index(330/331/332)` 또는 `option(21102/21103/21104)` | 0/1, true/false |
| SCRATCH AUTO | `option(21100)` | true/false (원본 id 없음) |
| KEEP GREEN NUMBER | `option(21101)` | true/false (원본 id 없음) |
| DP OPTION(패널 키 없음) | `event_index(54)` | 0..3 (값은 남음) |
| 열린 패널 | `option(21/22/23)`, 타이머 21~23 도착·31~33 출발 | 기존 |
| 닿지 않는 키 | `option(21140 + 인덱스)` (인덱스 0..8) | 5키·10키에서 인덱스 5·6 이 true |

### 1.8 기본 스킨이 그리는 구조 (`assets/skins/rbms-default/lua/select/`)

IIDX 옵션 화면 참고 이미지(4절)의 구성을 따른다. 파일: `sheet.lua`(패널 3개를 조립한다. `PAGES` `sheet.lua:673`, `sheet.build` `sheet.lua:685`), `panel.lua`(그리기 도구 `Sheet`: 값 목록 `list`, 숫자 칸 `field`, 키 `key`, 연결선 `wire`, 태그 `tag`, 본문 `body`, 베일 `veil`, 어둡게 `dim`), `inform.lua`(INFORMATION 줄의 변화 감시), `words.lua`(문구). 모든 요소는 올라오는 한 벌과 내려가는 한 벌, 두 destination 으로 놓인다(`Sheet:both`, `panel.lua:40`).

1920x1080 좌표(`sheet.lua:8-29`, `panel.lua:7-19`, `theme.lua:258-`):

| 요소 | 좌표와 크기 | 색(Mono 테마) |
| --- | --- | --- |
| 화면 전체 베일 | 0,0,1920x1080. 알파 248/255(뒤 선곡 화면이 6/255 로 비침), 150 ms | `bg` |
| INFORMATION | 태그 x 504, top 120, 232x32. 본문 x 504, top 156, 1352x80. 오른쪽에 `HOLD ...` 안내(캡션) | 태그·값 `theme.panel.pick` (255,152,52), 제목 `text`, 설명 `sub` |
| 건반 7개 | 흰 건반 1·3·5·7 은 아래 줄 top 312, 검은 건반 2·4·6 은 위 줄 top 264, 첫 x 836, pitch 112, 92x92 | 흰 `key_white` (226,230,236), 검 `key_black` (30,33,40). 라벨은 DOWN, UP |
| 턴테이블 상자 | x 626, top 284, 120x120. "TURNTABLE" 과 동작 이름(TARGET, HIDDEN+, JUDGE ALGORITHM) | 열 색 |
| 연결선 | 폭 4(`WIRE`). 꺾인 직선을 사각형 이어 붙여 그린다. 차선 높이 424·444·464(바깥 건반일수록 높게 해 엇갈림을 피한다), 패널 1 건반 7 과 패널 3 건반 4·6 은 건반 위 차선 y 248 로 돌아간다 | 그 열의 색 |
| 옵션 상자 | y 492 부터. 머리(굵은 이름 1~2줄, 높이 72) + 값 전체를 세로로 나열(pitch 44, 높이 38) | 어두운 칸 |
| 고른 값 | 밝은 주황 바탕 + 어두운 굵은 글자 + 빛 테두리 4. 지금/아님 두 `draw` 함수로 전환 | `pick` / `pick_ink` |
| 열 이름 | 상자 아래 굵은 글자: STYLE, GAUGE, SPEED, ASSIST, RANGE, JUDGE, TIMING, DISPLAY | 열 색 `theme.panel.column`: orange, green, violet, sky, locked(회색) |
| 왼쪽 열 | x 64, 폭 408: TARGET 21줄, GROOVE GAUGE 도움, HIDDEN+ 상자, ASSIST LAMP 규칙, JUDGE ALGORITHM, GAS 설명 | |

- 올라오기·내려가기는 이전과 같은 관례다: 타이머 `PANELn_ON`/`PANELn_OFF`, 옵션 `OPTION_PANELn`, 24 px 처짐과 알파 0 에서 150 ms 로 제 자리(`panel.lua:7`, `rise 24`, `move 150`).
- 패널 1(`play_page` `sheet.lua:249`): RANDOM(키 1·2·7), GAUGE(3·4), HI-SPEED FIX(5·6) 상자를 가운데 정렬 폭 280 으로 나열. 왼쪽에 TARGET 21줄(글자 `200..209`, `3`, `210..219`, 지금 줄은 주황 강조), 그 아래 GROOVE GAUGE 도움(지금 게이지 이름과 두 줄 설명). 건반 7(RANDOM 다음)은 건반 위 차선으로 돌아가 연결선이 교차하지 않는다.
- 패널 2(`assist_page` `sheet.lua:352`): ON/OFF 상자 7개. EX JUDGE 는 회색 `locked` 열에 `LOCKED / READ ONLY`. SCRATCH AUTO 아래 "ON: THE LAMP IS AN ASSIST GRADE". 왼쪽에 HIDDEN+ 상자와 ASSIST LAMP 규칙(게이지 → ASSIST EXHARD/HARD/CLEAR/EASY/LIGHT ASSIST 를 등급 색으로, `words.assist_rule` `words.lua:99-`).
- 패널 3(`detail_page` `sheet.lua:566`): BGA(ON/OFF), GAS(5값), AUTO ADJUST(OFF/ON), JUDGE TIMING 숫자 칸(양쪽 화살표 글자 `<` `>`, `±N MS`, FAST/SLOW 안내 두 줄), NOTE DISPLAY TIME 숫자 칸(ms 와 녹색 숫자. KEEP GREEN NUMBER 가 꺼져 있으면 `OFF` 와 "KEEP GREEN NUMBER IS OFF (SELECT + KEY 5)", 켜져 있는데 커서가 차트가 아니면 `-` 와 "MOVE THE CURSOR TO A CHART TO SET IT", 값이 있으면 LEFT/RIGHT 안내. `display_time` `sheet.lua:481`, `words.lua:71-80`). 왼쪽에 JUDGE ALGORITHM 목록(SCORE 상태면 강조 없이 "SCORE: SET IN SETTINGS > JUDGE")과 아래 GAS 설명 목록.
- INFORMATION(`inform.lua:13-36`): 패널이 열려 있는 동안만 값을 읽는 클로저다. 열림 타이머 값이 바뀌면 기준값을 새로 잡고 패널 안내(`words.guide` `words.lua:90-97`)를 보인다. 그 뒤 값이 바뀌면 왼쪽에 옵션 이름(굵게), 오른쪽에 새 값(굵은 주황), 아래에 한 줄 설명을 보인다. 한 프레임에 옵션 둘이 바뀌면 감시 순서상 마지막 것만 보인다.
- 5키·10키에서 닿지 않는 키(인덱스 5·6): 호스트가 불린 21140~21148 로 알리면(`extension.rs`, `skin_host/select/shown.rs` 의 `unreachable_keys_of(mode)`, `bridge.rs`) 스킨이 그 키와 외곽선, 패널 2의 해당 열 상자를 알파 196 으로 어둡게 그린다(`sheet.lua:132` `out_of_reach`, `Sheet:dim` `panel.lua:93`, `OUT_OF_REACH_ALPHA` `panel.lua:15`). 그 키에서 다른 열로 가는 연결선과 화살표는 그대로 밝다.
- 글꼴: 굵은 글씨는 `Inter-Bold.ttf`(Inter 4.1 `extras/ttf`, 무수정, 420,428 바이트)를 팩 `font/` 에 더한 별도 글꼴 id `inter-bold` 로 쓴다(`theme.lua:255`). `ui.lua` 의 `Builder:font_id` 가 `weight == "bold"` 일 때만 글꼴 목록에 늦게 더하므로 다른 화면은 글꼴 목록이 그대로다. 크기는 값 `label`(24) 이상, 보조 `caption`(18) 이상이다. 라이선스는 THIRD-PARTY-NOTICES 3절.
- 램프 등급: 패널 2의 ASSIST LAMP 규칙과 선곡 정보창·결과 화면의 등급 이름·색은 `clear-lamps.md` 4.2절.
- 측정(`rbms-cli skin-dump`, 7.1절): `select.luaskin` 은 release 200프레임에서 로드 7.0 → 24.8 ms, destination 503 → 1,611, 함수 값 61 → 171, 프레임당 호출 59 → 169, 평균 0.024 → 0.047 ms, p99 0.043 → 0.060 ms, 예산 초과 0(3단계 작업자 보고). 출발 destination 의 `draw` 함수는 `op` 가 없어 패널이 닫혀 있어도 매 프레임 불린다. 패널 수리(`3f058f4`) 뒤 debug 엄격 덤프 120프레임은 destination 1,677, 함수 203, 평균 0.230 ms, 최대 0.292 ms, 경고 0(debug 값이라 위 release 값과 비교하지 않는다).

### 1.9 선곡의 다른 "바로 바꾸기" 길

- 사이드 메뉴 QUICK SETTINGS(`lua/select/side.lua:20-40`, 마우스 클릭): KEY CONFIG, SKIN, LN MODE(308), BGA(72), GAUGE AUTO SHIFT(78), JUDGE ALGORITHM(340). 왼쪽 클릭 다음, 오른쪽 클릭 이전. BGA 이름표는 ON/OFF 둘이다(패널 3과 같다, 1.10절 3).
- 머리의 MODE, SORT, LN MODE 단추(`lua/select/controls.lua:11-15`)와 숫자 키 1·2·3(`Shortcut::RefMode/RefSort/RefLnMode`).
- F1 오버레이와 PLAY 메뉴(F8)는 2.3절.

### 1.10 현재 구현의 결함과 문서-코드 차이

코드로 확인된 것(읽기만, 실행 확인은 표시). 1~13은 조사 당시 목록이고 구현 뒤 상태를 덧붙였다. 14 이후는 구현에서 새로 생긴 것이다.

1. DP OPTION: 패널 1 키에서 뺐고 기본 스킨도 DP 상자를 그리지 않는다. 값 `play.lane_option`, 설정 행(`unbuilt_row` 로 숨김, `settings.rs:625`), 스킨 이벤트 54, 인덱스 54 응답은 그대로이고 플레이를 시작하는 코드는 여전히 이 값을 읽지 않는다.
2. 패널 2 키 1(EX JUDGE)은 눌러도 아무 일도 없다(`panel.rs:87`). 기본 스킨은 회색 `LOCKED / READ ONLY` 로 그려 죽은 칸임을 숨기지 않는다.
3. BGA 값 AUTOPLAY ONLY(1)는 엔진이 만들 수 없다(`options.rs:115-116`). 기본 스킨은 이름표를 ON/OFF 둘로 줄였다(`words.lua:4`, `side.lua` 의 BGA 이름).
4. 이전 값 키: GAUGE 와 HI-SPEED FIX 는 생겼다(1.3절). GAS 와 BGA 는 다음으로만 간다.
5. HI-SPEED FIX 는 커서가 폴더일 때 스킨 값이 "없음"(-1)이라 칸에 `-` 가 뜨는데 키를 누르면 실제로는 값이 움직인다(`has_play_settings` 는 코스 탭의 못 치는 코스만 막는다, `events.rs:407`). 읽기만. 이동 시간만 이 불일치를 고쳤다(6번).
6. NOTE DISPLAY TIME 키(패널 3의 4·6)는 KEEP GREEN NUMBER 가 꺼져 있으면 무동작이고(원본은 항상 움직임), 이제 커서가 곡·완성 코스가 아닌 막대(폴더, 표, 실행 파일)일 때도 무동작이다(`events.rs:396-405`). 칸은 꺼짐이면 `OFF` 와 경고, 켜짐인데 차트가 아니면 `-` 와 이동 안내를 따로 보인다. 테스트 `panel_tests.rs:444`(`the_third_panel_leaves_the_travel_time_alone_while_the_cursor_is_off_a_chart`).
7. RANDOM 값의 순환 순서가 화면마다 다르다. 패널과 스킨 이벤트는 원본 번호 순(`OPTION_GENERAL`)이고, 설정 행·PLAY 메뉴·F1 오버레이·연습은 `NoteOption::ALL` 순(S-RANDOM 이 R-RANDOM 앞, `crates/rbms-chart/src/shuffle.rs:18-27`)이다. 그대로다.
8. GAUGE 이름이 화면마다 다르다. 스킨 `ASSIST EASY ... EX HARD`, 설정 `EX-HARD`(`settings.rs:147`), IIDX 참고 이미지 `ASSISTED EASY`. 그대로다.
9. 키 6·7 이 없는 모드(5키, 10키)에서는 해당 역할에 닿지 못한다(1.3절). 동작은 그대로이고 기본 스킨이 그 키를 어둡게 그린다(1.8절).
10. F1 오버레이의 홀드 키 `LSHIFT` 와 턴테이블의 충돌: 해소했다. START, SELECT, 상세 키 중 하나가 눌려 있으면 홀드 키로 오버레이를 열지 않고 키가 선곡으로 간다(`app_options.rs:204-226`). 핀 키 F1 과 패널이 없을 때는 이전과 같다. 부작용: 스킨 없는 내장 선곡에서도 START 를 누른 채 LShift 를 누르면 열리지 않는다. 실기 미확인(단위 테스트만).
11. 출하 설정에서 `play.autoplay` 는 true 다(`schema.rs:588`, 테스트 `crates/rbms-config/src/tests.rs:44`). `enable_cover` 도 true 다(`schema.rs:602`). 패널에 AUTOPLAY 같은 토글을 넣을 때 주의할 출하값이다. 패널에는 넣지 않았다.
12. 기본 스킨 파일은 설치 때 복사되고 사용자가 고친 파일은 덮어쓰지 않는다(`apps/rbms-player/src/assets/default_pack.rs:1-30`). 새 모듈(`panel.lua`, `inform.lua`, `words.lua`)과 `props.lua`·`theme.lua` 변경이 들어가므로, 옛 `sheet.lua` 를 직접 고쳐 둔 사용자는 새 모듈과 옛 `sheet.lua` 가 섞인 상태가 될 수 있다(읽기만).
13. 5키·10키에서 6·7번 건반이 없다는 사실은 실제 창에서 확인하지 못했다(캡처와 호스트 테스트로만).
14. 외부 스킨 위험(수리하지 않음, 리뷰 panels-1): 키 표 변경은 모든 스킨에 적용된다. 확장 id 를 모르는 외부 스킨(예 ModernChic)에서 패널 2 키 3·5·6·7 과 턴테이블, 패널 1 키 4·6, 패널 3 턴테이블이 표시 없이 저장 설정(SCRATCH AUTO, KEEP GREEN NUMBER, 가림막, GAUGE 이전, HI-SPEED FIX 이전, 판정 알고리즘)을 바꾼다. 패널 키 소리 `OptionChange` 외에 완화 수단이 없다. 고치려면 스킨이 확장 id 를 안다는 신호(헤더 플래그, 팩 경로 판정, 실행 중 읽기 기록)가 필요한데 로더 변경이나 사용자 파일 영향이 있어 이번에 하지 않았다. 사용자 판단 대기.
15. 패널 2 턴테이블(HIDDEN+)과 패널 3 턴테이블(JUDGE ALGORITHM)은 눌림으로 읽고, 패널이 올라오기 전부터 눌려 있던 방향은 눌림이 아니다(`forget_turntable_changes`, `keys.rs:298`; 테스트 `panel_tests.rs:322`). 같은 호출이 패널이 바뀔 때 직전 프레임의 한 번 돌림도 버린다(프레임 단위 테스트는 없다).
16. 5키·10키에서도 패널은 7건반을 그대로 그린다(닿지 않는 키만 어둡게). 패널 3 의 JUDGE TIMING `>` 와 NOTE DISPLAY TIME `>` 화살표, 6·7 키에서 나오는 연결선은 다른 키로 닿는 열이라 밝다.
17. 값이 바뀐 옵션 이름은 영어 대문자 문구다(`words.lua`). 문구와 게이지·GAS·알고리즘 설명을 메인이 새로 썼다. 사용자 문구 검토 대기.
18. `docs/play-options.md` 와 `docs/skin.md` 6절, 수동 점검표 C1(`NOT AVAILABLE` 문구)은 이 구현 전 상태를 적고 있다(이 문서 소유 밖).

---

## 2. 플레이 옵션 전체와 바꾸는 곳

### 2.1 값, 기본값, 바꾸는 곳 (저장 필드 기준)

범례: P1·P2·P3 = 선곡 패널 1·2·3(건반 번호), F1 = F1 오버레이 11행(`apps/rbms-player/src/app_options.rs:34-46`), PLAY = PLAY 메뉴(F8, `SettingTab::PLAY_MENU` 네 묶음 SPEED·LANE·JUDGE·CHART), SET = SETTINGS 화면의 탭(JUDGE, DISPLAY, INPUT, NETWORK, AUDIO, SELECT), 연습 = 연습 패널, 키 = 플레이 중 키. 설정 행 정의는 `crates/rbms-config/src/settings.rs:529-`, 저장 필드는 `schema.rs:528-700`.

| 옵션(필드) | 값 목록과 범위 | 기본 | 패널 | F1 | PLAY | SET | 연습 | 플레이 중 키 | 스킨 이벤트 / 값 id |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| AUTOPLAY (`play.autoplay`) | OFF, ON | ON | 없음 | Autoplay | CHART | 없음 | 없음 | 없음 | 이벤트 16 은 1회 오토플레이이고 설정은 안 바꾼다. 명령줄 `--auto`/`--interactive` |
| HI-SPEED (`play.hispeed`) | 0.01~20.00, 한 번에 HI-SPEED STEP | 2.00 | 없음 | HiSpeed | SPEED | 없음 | 없음 | UP/DOWN, START+건반 | 이벤트 57. 숫자 10·310·311, 실수 310 |
| HI-SPEED STEP (`play.hispeed_step`) | 0.01~1.00, 0.01 | 0.25 | 없음 | 없음 | SPEED | 없음 | 없음 | 없음 | 없음 |
| HI-SPEED FIX (`play.fix_hispeed`) | OFF, START, MAX, MAIN, MIN | MAIN | P1 키 5(다음)·6(이전) | FixHiSpeed | SPEED | 없음 | 없음 | 없음 | 이벤트 55, 인덱스 55 |
| SPEED FIX (`play.constant_speed`) | FLOATING, CONSTANT | FLOATING | P2 키 2 | 없음 | SPEED | 없음 | 없음 | 없음 | 이벤트 352, 인덱스 302·352 |
| KEEP GREEN NUMBER (`play.keep_green_number`) | OFF, ON | OFF | P2 키 5 | 없음 | SPEED | 없음 | 없음 | 지정 키(미지정) | 이벤트 21001, 불린 21101(R-BMS 확장) |
| GREEN NUMBER (`play.duration_ms`) | 이동 시간 1~10,000 ms(녹색 1~6,000) | 500 ms(300) | P3 키 4·6(KEEP 켜짐이고 커서가 곡·코스일 때) | 없음 | SPEED | 없음 | 없음 | START+건반, SELECT+건반, 지정 키 | 이벤트 59, 숫자 312·313 |
| RANDOM (`play.random`) | OFF, MIRROR, RANDOM, S-RANDOM, R-RANDOM, ROTATE, H-RANDOM, ALL-SCRATCH(설정 순), 패널은 R/S 순서 바뀜(1.10절 7) | OFF | P1 키 1·2·7 | Random | CHART | 없음 | OPTION-1P | 없음 | 이벤트 42·43, 인덱스 42·43 |
| LANE OPTION (`play.lane_option`) | OFF, FLIP, BATTLE, BATTLE AUTO-SC | OFF | 없음(패널 키에서 뺌, 적용 안 됨) | 없음 | 숨김(`unbuilt_row`) | 숨김 | 없음 | 없음 | 이벤트 54, 인덱스 54 |
| GAUGE (`play.gauge`) | ASSIST EASY, EASY, NORMAL, HARD, EX-HARD, HAZARD | NORMAL | P1 키 3(다음)·4(이전) | Gauge | CHART | 없음 | GAUGE TYPE(9종, 별개 값) | 없음 | 이벤트 40, 인덱스 40 |
| LEGACY NOTE (`play.legacy_note`) | OFF, ON | OFF | P2 키 4 | 없음 | CHART | 없음 | 없음 | 없음 | 이벤트 353, 인덱스 304·353 |
| LANE COVER ON (`play.enable_cover`) | ON, OFF | ON | P2 키 6 | 없음 | LANE | 없음 | 없음 | `\`(COVER TOGGLE), START 두 번 | 이벤트 330, 인덱스 330 |
| LANE COVER (`play.cover`) | 0~100%(0.1% 격자) | 0% | 없음 | LaneCover(1%) | LANE | 없음 | 없음 | RIGHT/LEFT, START+턴테이블, 휠 | 숫자 14, 비율 4 |
| LIFT ON (`play.enable_lift`) | OFF, ON | OFF | P2 키 7 | 없음 | LANE | 없음 | 없음 | 미지정 | 이벤트 331, 인덱스 331 |
| LIFT (`play.lift`) | 0~100% | 0% | 없음 | Lift | LANE | 없음 | 없음 | `[` `]`, START+SELECT+턴테이블 | 숫자 314 |
| HIDDEN+ ON (`play.enable_hidden`) | OFF, ON | OFF | P2 턴테이블 | 없음 | LANE | 없음 | 없음 | 미지정 | 이벤트 332, 인덱스 332 |
| HIDDEN+ (`play.hidden`) | 0~100% | 0% | 없음 | Hidden | LANE | 없음 | 없음 | 위·아래 키 미지정 | 숫자 315 |
| COVER FINE STEP (`play.lanecover_step_fine`) | 0.0001~0.0100 | 0.0010 | 없음 | 없음 | LANE | 없음 | 없음 | FINE STEP 키 | 없음 |
| SCRATCH SIDE (`play.scratch_left`) | RIGHT, LEFT | RIGHT | 없음 | ScratchSide | 없음 | INPUT | 없음 | 없음 | 스킨 옵션 "Scratch Side". 명령줄 `--sc-left` |
| SCRATCH AUTO (`play.scratch_auto`) | OFF, ON | OFF | P2 키 3 | ScratchAuto | CHART | 없음 | 없음 | 없음 | 이벤트 21000, 불린 21100(R-BMS 확장). 명령줄 `--sc-auto` |
| AUTO REPLAY (`play.auto_replay`) | ON, OFF | ON | 없음 | 없음 | CHART | 없음 | 없음 | 없음 | 없음 |
| ESCAPE (`play.play_escape`) | IMMEDIATE, HOLD, DOUBLE TAP | IMMEDIATE | 없음 | 없음 | CHART | 없음 | 없음 | 없음 | 없음 |
| KEY DEBOUNCE (`play.key_debounce_ms`) | 0~100 ms | 16 | 없음 | 없음 | 없음 | INPUT | 없음 | 없음 | 없음 |
| PAD DEBOUNCE (키 설정 파일) | 0~100 ms | 16 | 없음 | 없음 | 없음 | INPUT(호스트 행) | 없음 | 없음 | 없음 |
| TOTAL (`play.total_override`) | AUTO(0) 또는 10 단위 | AUTO | 없음 | 없음 | 없음 | JUDGE | TOTAL(10~5000) | 없음 | 없음 |
| JUDGE OFFSET (`judge.offset_ms`) | -200~+200 ms | 0 | P3 키 5·7 | 없음 | JUDGE | 없음 | 없음 | `9`/`0` | 이벤트 74, 숫자 12 |
| AUTO ADJUST (`judge.auto_offset`) | OFF, ON | OFF | P3 키 3 | 없음 | JUDGE | 없음 | 없음 | 미지정 | 이벤트 75, 인덱스 75 |
| JUDGE ALGORITHM (`judge.judge_algorithm`) | COMBO, DURATION, LOWEST, SCORE | COMBO | P3 턴테이블 | 없음 | 없음 | JUDGE | 없음 | 없음 | 이벤트 340(앞의 셋만), 인덱스 340 |
| JUDGE WIDTH 6개 (`judge.judge_rate_key/scratch`) | 50~200%, 5 단위 | 100% | 없음 | 없음 | 없음 | JUDGE | JUDGERANK(1~400, 별개) | 없음 | 없음. 100 초과면 ASSIST 판(어시스트 2) |
| LN MARGIN (`judge.longnote_margin_rate`) | 50~200%, 5 | 100% | 없음 | 없음 | 없음 | JUDGE | 없음 | 없음 | 없음 |
| LN MODE (`judge.ln_mode`) | LN, CN, HCN | LN | 없음 | 없음 | 없음 | JUDGE | 없음 | 없음 | 이벤트 308, 키 `3`, 머리 단추, QUICK SETTINGS |
| GAUGE SET (`judge.gauge_set`) | AUTO, LR2 | AUTO | 없음 | 없음 | 없음 | JUDGE | GAUGE CATEGORY(5종) | 없음 | 없음 |
| GAUGE AUTO SHIFT (`judge.gauge_auto_shift`) | NONE, CONTINUE, SURVIVAL TO GROOVE, BEST CLEAR, SELECT TO UNDER | NONE | P3 키 2 | 없음 | 없음 | JUDGE | 없음 | 없음 | 이벤트 78, 인덱스 78 |
| BOTTOM SHIFTABLE (`judge.bottom_shiftable_gauge`) | ASSIST EASY, EASY, NORMAL | ASSIST EASY | 없음 | 없음 | 없음 | JUDGE | 없음 | 없음 | 이벤트 341, 인덱스 341(패널 키 없음) |
| TARGET (`judge.target`) | 16개(1.4절) | RATE AAA | P1 턴테이블·휠·방향키 | Target | CHART | 없음 | 없음 | 없음 | 이벤트 77, 글자 3·200~219 |
| TARGET FALLBACK (`judge.target_fallback`) | 13개 | RATE AAA | 없음 | 없음 | CHART | 없음 | 없음 | 없음 | 없음 |
| TARGET RATE (`judge.target_rate_percent`) | 1~100% | 85% | 없음 | 없음 | CHART | 없음 | 없음 | 없음 | 없음 |
| BGA (`display.bga`) | ON, OFF | ON | P3 키 1 | 없음 | 없음 | DISPLAY | 없음 | 없음 | 이벤트 72, 인덱스 72 |
| GUIDE SE (`audio.guide_se`) | OFF, ON | OFF | 없음 | 없음 | 없음 | AUDIO | 없음 | 없음 | 이벤트 343, 인덱스 343 |
| 연습만(구간 START·END TIME, GAUGE VALUE, FREQUENCY 50~200%) | 연습 패널 9절 | | | | | | 연습 | | |

- 화면 전용: WHITE NUMBER, JUDGE TEXT Y, 5KEYS LAYOUT 은 내장 화면만 읽는다(`settings.rs:732-744`). SCORE GRAPH, RESULT GRAPHS, REPLAY ANALYSIS, PREVIEW, LETTERBOX, RESOLUTION, WINDOW MODE, DEBUG MODE, FONT, SKIN 은 플레이 결과에 영향이 없는 표시 설정이다.
- 불투명도, 레인 폭, 노트 색 등 스킨 옵션은 SKIN 메뉴와 PLAY 메뉴의 LANE·SKIN 묶음(`play_menu/rows.rs`)이 바꾼다. 이 문서의 범위 밖이다.

### 2.2 패널에서 바꿀 수 없는 것

패널 안에 키가 없는 항목이다. "스킨 id" 는 스킨이 현재 값을 읽을 수 있는지, "이벤트" 는 스킨 단추가 바꿀 수 있는지다.

이 절의 표는 조사 당시 모습이다. 2026-10-22 구현으로 SCRATCH AUTO, KEEP GREEN NUMBER, LANE COVER·LIFT·HIDDEN+ 켬끔, JUDGE ALGORITHM 은 패널 키와 스킨 id 를 갖게 되었고(1.3, 3.4절), 해당 행에 "구현됨" 을 적었다. 키가 없는 채 남은 것은 LN MODE, BOTTOM SHIFTABLE, GUIDE SE(스킨 id·이벤트는 있음)와 아래 표의 나머지다.

요청문이 패널에서 바꿀 수 없다고 든 것 가운데 LEGACY NOTE(패널 2 키 4), BGA(패널 3 키 1), GAUGE AUTO SHIFT(패널 3 키 2)는 조사 당시에도 패널에서 바뀌었다. 요청한 사람의 체감과 구현이 달랐던 것은 값 하나만 보이고 목록이 없어 "무엇이 바뀌는지" 가 안 드러난 때문으로 보인다(추정이고 확인한 사실은 아니다). 지금은 값 목록이 전부 보인다.

| 항목 | 플레이에 주는 영향 | 지금 바꾸는 곳 | 스킨 id | 이벤트 | 패널에 넣을 때 필요한 것 |
| --- | --- | --- | --- | --- | --- |
| SCRATCH AUTO | 턴테이블 노트를 사람 대신 쳐 주고 노트에서 뺀다. 램프가 LIGHT ASSIST 로 내려간다(`docs/play-options.md` 10.2, 10.5절) | F1, PLAY > CHART, 명령줄 | 없음 | 없음 | 구현됨(2026-10-22): 패널 2 키 3, 확장 이벤트 21000, 불린 21100 |
| KEEP GREEN NUMBER | 녹색 숫자 유지 여부. 패널 3의 NOTE DISPLAY TIME 이 이 값이 켜져야 움직인다 | PLAY > SPEED | 없음 | 없음 | 구현됨(2026-10-22): 패널 2 키 5, 확장 이벤트 21001, 불린 21101 |
| AUTOPLAY | 판 전체를 컴퓨터가 침. 출하 기본 ON | F1, PLAY > CHART | 옵션 32·33 은 플레이 화면 전용 | 이벤트 16 은 1회만 | 패널에 넣으면 실수로 켜진 채 저장될 수 있다. 기록 저장·IR 전송에서 빠지는 판이 된다 |
| LANE COVER·LIFT·HIDDEN+ 켬/끔 | 판정선 위 가림막 | PLAY > LANE, `\` | 인덱스 330~332 있음 | 있음 | 구현됨(2026-10-22): 패널 2 키 6·7·턴테이블 |
| LANE COVER·LIFT·HIDDEN+ 양 | 가림막 높이 | F1, PLAY > LANE, 플레이 중 키 | 숫자 14·314·315 있음 | 없음 | 패널은 켬끔만 다루는 것이 원본 관례 |
| HI-SPEED, HI-SPEED STEP | 스크롤 속도 | F1, PLAY > SPEED, 플레이 중 | 숫자 10·310·311 있음 | 57 있음 | 패널 키 역할만 필요(원본도 패널엔 없음) |
| JUDGE ALGORITHM | 눌림이 어느 노트를 먹는가 | SETTINGS > JUDGE, QUICK SETTINGS | 인덱스 340 있음(SCORE 는 -1) | 340 있음 | 구현됨(2026-10-22): 패널 3 턴테이블(COMBO, DURATION, LOWEST 만 순환, SCORE 는 보이기만) |
| LN MODE | 롱노트 종류 | SETTINGS > JUDGE, 머리 단추, 키 3 | 인덱스 308 있음 | 308 있음 | 패널 키 역할 |
| BOTTOM SHIFTABLE | GAS 가 내릴 수 있는 최저 게이지 | SETTINGS > JUDGE | 인덱스 341 있음 | 341 있음 | 패널 키 역할(원본 ModernChic 은 패널 3의 단추로 둔다) |
| GUIDE SE | 판정마다 소리 | SETTINGS > AUDIO, QUICK SETTINGS 없음 | 인덱스 343 있음 | 343 있음 | 패널 키 역할 |
| SCRATCH SIDE | 스크래치 레인이 어느 쪽 | F1, SETTINGS > INPUT, 스킨 옵션 | 없음 | 없음 | 스킨 옵션이 따로 있어 우선순위가 겹친다(`settings.rs:775-781`의 `SKIN_OPTION_NOTE`) |
| TARGET FALLBACK, TARGET RATE | 목표가 없을 때의 대체 목표, 사용자 비율 | PLAY > CHART | 없음 | 없음 | 인덱스 id 둘 |
| AUTO REPLAY | 모든 판의 리플레이 저장 | PLAY > CHART | 없음 | 없음 | 플레이에 직접 영향은 없다 |
| ESCAPE | Esc 한 번으로 포기할지 | PLAY > CHART | 없음 | 없음 | 플레이에 직접 영향은 없다 |
| TOTAL 직접 지정 | 게이지 증가량. 직접 지정하면 어시스트 판(최고 기록 없음) | SETTINGS > JUDGE | 없음 | 없음 | 어시스트 판을 만들므로 패널에는 부적합하다 |
| JUDGE WIDTH 6개, LN MARGIN | 판정 폭. 넓히면 어시스트 판 | SETTINGS > JUDGE | 인덱스 301 은 읽기 전용 | 없음 | 숫자 칸이 필요하고 어시스트 판을 만든다 |
| DP 옵션(FLIP/BATTLE) | 적용 안 됨 | 패널 키에서 뺌(2026-10-22). 설정 행은 숨김 | 있음 | 있음 | 먼저 플레이에 구현해야 의미가 있다 |
| 2P 쪽 RANDOM | 설정이 하나뿐 | 없음 | 인덱스 43 은 1P 와 같은 값 | 같은 값을 만짐 | 별도 `random_2p` 필드가 필요하다 |
| MARK NOTE, BPM GUIDE, JUDGE AREA, NO MINE | 대응 기능이 없다 | 없음 | 없음(-1) | 없음 | 기능 자체를 구현해야 한다 |

### 2.3 다른 "옵션 바꾸는 곳" 요약

| 곳 | 여는 법 | 내용 |
| --- | --- | --- |
| F1 오버레이(`app_options.rs`) | 선곡에서 `LSHIFT` 누르고 있기 또는 `F1`(고정)(`app_options.rs:99-103`, 키 설정에서 변경) | 1280x720 고정 좌표의 오른쪽 패널 420 폭. 행 11개: RANDOM, GAUGE, HI-SPEED, HI-SPEED FIX, LANE COVER, LIFT, HIDDEN+, SCRATCH SIDE, SCRATCH AUTO, AUTOPLAY, TARGET. 위·아래 방향키로 행 이동, 왼쪽·오른쪽 방향키로 값, Enter 로 다음 값, Esc 로 닫기 |
| PLAY 메뉴 | 선곡에서 `F8` | SPEED(6행), LANE(7행), JUDGE(2행), CHART(RANDOM, GAUGE, LEGACY NOTE, TARGET, TARGET FALLBACK, TARGET RATE, AUTOPLAY, SCRATCH AUTO, AUTO REPLAY, ESCAPE 10행) + 미리 보는 플레이 스킨이 선언한 옵션과 오프셋. 왼쪽 목록과 오른쪽 미리보기 |
| SETTINGS | 선곡에서 `Tab` | 탭 JUDGE, DISPLAY, INPUT, NETWORK, AUDIO, SELECT |
| 연습 패널 | 선곡에서 `F4` | START TIME, END TIME, GAUGE TYPE, GAUGE CATEGORY, GAUGE VALUE, JUDGERANK, TOTAL, FREQUENCY, OPTION-1P (`docs/play-options.md` 9.1절). 패드는 받지 않는다 |
| 플레이 중 키 | 키 설정(F5) | 조작 20종(HI-SPEED UP·DOWN, 레인 커버 UP·DOWN·ON/OFF, 리프트 UP·DOWN·ON/OFF, 히든 UP·DOWN·ON/OFF, GREEN UP·DOWN, KEEP GREEN ON/OFF, OFFSET ±1, AUTO ADJUST ON/OFF, FINE STEP, START, SELECT)(`keyconfig.rs:384-405`). 출하 키 UP/DOWN, RIGHT/LEFT, `\`, `[`/`]`, `9`/`0`, `A`, `W`. 나머지는 미지정. START·SELECT 조합은 `docs/play-options.md` 5.2절 |
| 사이드 메뉴 | 선곡에서 마우스 | 1.9절 QUICK SETTINGS 6행 |
| 명령줄 | `rbms-player --help` | `--auto`, `--interactive`, `--sc-left`, `--sc-auto`, `--lift`, `--hispeed`, `--gauge`(`command_line.rs:125-148`) |

---

## 3. beatoraja 원본

원본 소스: `/Users/hyunseokbyun/development/beatoraja`(읽기만, 저장소에 복사하지 않음).

### 3.1 키 표와 패널

`select/MusicSelectKeyProperty.java:9-67` 의 `BEAT_7K`(14K 는 두 벌, `POPN_9K` 는 따로)가 R-BMS `BEAT_TABLE` 과 같다. 다른 점은 R-BMS 가 `NEXT_REPLAY` 를 인덱스 5(7키)와 7(9키)에서 뺀 것뿐이다. 패널 진입 규칙은 `MusicSelectInputProcessor.java:62-330`:

- START 만: `setPanelState(1)`. 키마다 `option1p`(42)·`gauge1p`(40)·`optiondp`(54)·`option2p`(43)·`hsfix`(55)를 `executeEvent(type, ±1)` 로 실행. 턴테이블과 방향키 위아래와 휠은 `target`(77)을 `±1` 로 실행하고 `SCRATCH` 소리.
- SELECT 만: `setPanelState(2)`. 이벤트 없이 `PlayerConfig` 를 직접 뒤집는다: `JUDGEWINDOW_UP`=`customJudge`, `CONSTANT`=`scrollMode` 0/1, `JUDGEAREA`=`showjudgearea`, `LEGACYNOTE`=`longnoteMode` 0/1, `MARKNOTE`=`markprocessednote`, `BPMGUIDE`=`bpmguide`, `NOMINE`=`mineMode` 0/1. 매번 `OPTION_CHANGE` 소리.
- START+SELECT 또는 `NUM5`: `setPanelState(3)`. `bga`(72), `gaugeautoshift`(78), `notesdisplaytiming`(74, 키 5는 -1, 키 7은 인자 없음=+1), `notesdisplaytimingautoadjust`(75), `duration1p`(59, 키 4는 -1, 키 6은 +1, 처음 한 번에 즉시 이동·300 ms 뒤 50 ms 마다 반복·50번 넘으면 10씩).
- 어느 패널도 아닐 때: 목록 키와 숫자 키(`NUM0` 검색, `NUM1` mode, `NUM2` sort, `NUM3` lnmode, `NUM4` 다음 리플레이, `NUM7` rival, `NUM8` 같은 폴더 곡, `NUM9` open_document).

### 3.2 원본 이벤트·옵션 id (`skin/SkinProperty.java`, `skin/property/EventFactory.java`)

| 항목 | id | 원본 값 개수와 순환 |
| --- | --- | --- |
| gauge1p | 이벤트 40, 인덱스 `gaugetype_1p` 40 | `(gauge + 1 또는 5) % 6` — 6개 |
| option1p, option2p | 42, 43 | `% 10` — 10개(IDENTITY, MIRROR, RANDOM, ROTATE, S_RANDOM, SPIRAL, H_RANDOM, ALL_SCR, RANDOM_EX, S_RANDOM_EX, `Random.java:32-33`) |
| optiondp | 54 | `% 4` |
| hsfix | 55 | `% 5` |
| hispeed1p | 57 | 하이스피드 margin 씩, 범위로 자름 |
| duration1p | 59 | `arg2 > 0 ? arg2 : 1` |
| bga | 72 | `% 3`(ON, AUTOPLAY ONLY, OFF) |
| notesdisplaytiming | 74 | ±1, 범위 끝에서 멈춤 |
| notesdisplaytimingautoadjust | 75 | 뒤집기 |
| target | 77 | 목표 목록 순환 |
| gaugeautoshift | 78 | `% 5` |
| bottomshiftablegauge | 341 | `% 3` |
| lnmode | 308 | `% 3` |
| lanecover, lift, hidden | 330, 331, 332 | `PlayConfig` 의 `enable*` 뒤집기 |
| judgealgorithm | 340 | `JudgeAlgorithm.defaultAlgorithm`(3개) |
| hispeedautoadjust | 342 | 뒤집기(R-BMS 는 없음) |
| guidese | 343 | 뒤집기 |
| extranotedepth, minemode, scrollmode, longnotemode | 350~353 | `% 4`, `% 5`, `% 3`, `% 6` |
| seventonine_pattern, seventonine_type | 360, 361 | `% 7`, `% 3` |
| 이미지 인덱스 assist | `customjudge` 301, `assist_constant` 302, `showjudgearea` 303, `assist_legacy` 304, `markprocessednote` 305, `bpmguide` 306, `assist_nomine` 307 | 이 일곱은 인덱스이지 이벤트가 아니다(`events.rs:28-29`) |
| 인덱스 constant | 400(`PlayConfig.enableConstant`) | `assist_constant`(302, `scrollMode == 1`)와 다른 값이다. R-BMS 는 400 을 답하지 않는다 |
| 패널 | 옵션 21·22·23, 타이머 21~23(켜짐)·31~33(꺼짐) | |
| 숫자 | 12(JUDGETIMING), 312(DURATION), 313(DURATION_GREEN), 310·311(하이스피드) | |

R-BMS 가 원본과 다르게 구현한 것: RANDOM 의 값 개수(8, RANDOM_EX·S_RANDOM_EX 없음), BGA 값(둘), 판정 알고리즘(넷째 SCORE 는 원본 번호 밖), 패널 3의 NOTE DISPLAY TIME(KEEP GREEN NUMBER 의존), 일곱 assist 중 둘만 구현.

### 3.3 원본이 패널을 그리는 방식

- 원본 기본 스킨(`skin/default/select.json`, 1280x720): 패널마다 배경 이미지 한 장(`option-panel1` 290x309, `option-panel2` 300x159, `option-panel3` 300x188, `panel.png`)과 값별 선택 표시 이미지. 선택 표시는 `imageset` 의 `ref` 가 인덱스 id 이고 `images` 가 값 개수만큼이다(`select.json:153-167`): `option-random` ref 42(10장), `option-gauge` ref 40(6장), `option-hsfix` ref 55(5장), `option-dp` ref 54(4장), `option-bga` ref 72(3장), assist 셋은 ref 301·302·304(2장). 숫자 `duration` ref 312, `judgetiming` ref 12(`select.json:195-196`). 좌표(`select.json:794-865`): 패널 1 상자 (50,100,580x618), 선택기 random1 (56,100,134x300), gauge (200,220,134x180), hsfix (342,250,134x150), random2 (486,100,134x300), dp (270,594,134x120). 패널 2 칸 134x30 일곱(키 홀수는 y 368, 짝수는 y 596). 패널 3 bga (52,282,134x120).
- ModernChic(`/Users/hyunseokbyun/Downloads/ModernChic/Select/lua/{option,assistoption,suboption}.lua`, 읽기만): 패널 1~3 이 1920x1080 에서 위·아래 프레임이 100 ms 에 들어오고(`docs/reference/skin-compat/m4-select.md:756`), 내용은 같은 `imageset ref` 방식이다. 패널 3 의 `suboption.lua:239-245` 는 CONSTANT(400)·BOTTOMSIFTABLEFGAUGE(341)·LANECOVER(330)·LIFT(331)·HIDDEN(332)·HISPEEDAUTOADJUST(342)·JUDGEALGORITHM(340)을 `act` 와 `ref` 가 같은 id 인 클릭 단추로 둔다. R-BMS 가 CONSTANT(400)와 HAA(342)를 답하지 않아 그 두 단추는 늘 첫 이미지이고 눌러도 무반응이다.
- 원본 방식의 핵심: 값 하나당 이미지 한 장을 미리 굽는다. 새 값이 생기면 이미지를 새로 그려야 한다. R-BMS 기본 스킨 Lua 는 이미지를 굽지 않고 `draw` 함수나 imageset 으로 같은 효과를 낼 수 있다(5.1절).

### 3.4 R-BMS 확장 id 의 과거와 현재, 범위 규칙

2026-10-22 구현으로 R-BMS 전용 id 가 다시 생겼다. 단일 목록은 `apps/rbms-player/src/skin_host/extension.rs`의 머리 문서와 상수이고, 결정은 ADR [R-BMS 확장 id](../acknowledge/2026-10-22-adr-rbms-extension-ids.md)다. 2026-10-09 "사설 id 철거" 결정(아래)의 부분 변경이며 사용자는 아직 확인하지 않았다. 숫자·글자·비율 공간에는 확장이 없다.

두 대역은 원본이 선언한 최대(옵션 3035, 이벤트 388)와 스킨에 남긴 대역(옵션 900~999, 이벤트 1000~1999, 타이머 10000~19999) 위에 있고 서로 겹치지 않는다(`extension.rs` 테스트가 고정). 원본 id 표(`reference_implements`), 생성된 원본 id 표, `property_checksum` 은 건드리지 않았다.

- 이벤트 21000~21099(`EXTENSION_EVENT_FIRST/LAST`): 이벤트 공간에는 게이트가 없어 스킨이 `main_state.event_exec(id)` 나 단추 `act` 로 그대로 부른다. `EVENT_ROUTES` 가 `Cluster::Options` 로 보내고(`skin_host/mod.rs:572`) 선곡 `EVENTS` 표에 등록돼 있다(`events.rs:285-286`). 첫 인자가 어느 쪽이든 한 번 실행에 한 번 뒤집는다. 소리는 `OptionChange`, 저장은 패널이 닫힐 때와 선곡을 떠날 때.
- 불린 21100~21199(`EXTENSION_OPTION_FIRST/LAST`): Lua 는 `draw`/`value`/`text` 같은 함수 필드 안에서 `main_state.option(id)` 로만 읽는다. `op = {id}` 는 로더 게이트(`loader/track.rs:157`)에 걸려 버려진다. 음수 id 는 부정, 답하지 않는 화면에서는 false.

| 종류 | id | 상수 | 답하는 화면 | 뜻 |
| --- | --- | --- | --- | --- |
| 이벤트 | 21000 | `RBMS_EVENT_SCRATCH_AUTO` | 선곡 | SCRATCH AUTO 켬끔(`play.scratch_auto`) |
| 이벤트 | 21001 | `RBMS_EVENT_KEEP_GREEN_NUMBER` | 선곡 | KEEP GREEN NUMBER 켬끔(`play.keep_green_number`) |
| 불린 | 21100 | `RBMS_OPTION_SCRATCH_AUTO_ON` | 선곡(Options) | SCRATCH AUTO 켜짐 |
| 불린 | 21101 | `RBMS_OPTION_KEEP_GREEN_NUMBER_ON` | 선곡(Options) | KEEP GREEN NUMBER 켜짐 |
| 불린 | 21102~21104 | `RBMS_OPTION_LANE_COVER_ON`, `_LIFT_ON`, `_HIDDEN_ON` | 선곡(Options) | 가림막 켜짐(= `event_index(330/331/332) == 1`). 원본 id 로도 읽을 수 있다 |
| 불린 | 21110 + k (k = 0..4) | `RBMS_OPTION_SELECTED_BEST_ASSIST_GRADE_FIRST` | 선곡(Select) | 커서 아래 곡의 최고 램프 어시스트 등급 == k. 최고 기록이 없는 곡·폴더·코스는 다섯 모두 꺼짐 |
| 불린 | 21120 + k | `RBMS_OPTION_RESULT_ASSIST_GRADE_FIRST` | 결과(Result) | 이번 판 등급 == k(코스 결과는 0) |
| 불린 | 21130 + k | `RBMS_OPTION_RESULT_PREVIOUS_ASSIST_GRADE_FIRST` | 결과(Result) | 이번 판 이전의 최고 기록(이미지 인덱스 371 과 같은 LN 모드 기준) 등급 == k. 처음 친 곡은 다섯 모두 꺼짐 |
| 불린 | 21140 + i (i = 0..8) | `RBMS_OPTION_UNREACHABLE_KEY_FIRST` | 선곡(Select) | 지금 모드에서 키 인덱스 i 에 닿는 레인이 없음(5키·10키의 5·6) |

등급 k: 0 은 등급 없음(램프가 LIGHT ASSIST 가 아니거나 ASSIST EASY 게이지로 클리어. 램프 id 와 함께 읽는다: 선곡 `event_index(370)`, 결과 370/371), 1 ASSIST EASY, 2 ASSIST CLEAR, 3 ASSIST HARD, 4 ASSIST EXHARD(`rbms_judge::AssistGrade::id`). 등급 의미는 `clear-lamps.md`.

과거와 근거:

- 2026-09-17: 옵션 패널을 `select` 문서의 대체 단위로 두고 사설 id 를 썼다. 대역은 20001~20011(결과 글자), 20101~20161(선곡 옵션 패널 행: 라벨 20101~, 값 20121~, 포커스 20141~), 20201~20205(플레이 목표 이름과 차이, 선곡·결과 힌트, IR 상태), 20301~20316(선곡 상세 통계) 이었다. 선곡 어댑터 `state.rs:101-148` 에 섞여 있었다(`docs/reference/skin-compat/b5-properties.md:173`, `docs/acknowledge/2026-09-17-skin-system-decisions.md` D10·D15).
- 충돌 방지 방식: 원본 id 공간 밖(원본이 선언한 최대 id 보다 위)에 둔다. `b5-properties.md:173` 는 "beatoraja id 공간 밖이므로 ModernChic와 충돌하거나 쓰이지 않는다"고 적었다.
- 2026-10-09 결정으로 폐기: 혼합 합성(`composition`, `layer`, `replace`, `hotspot`)과 함께 "사설 id 20001~20316 삭제, 레퍼런스 id 공간 밖"(`docs/reference/skin-compat/00-synthesis.md:406`, `docs/acknowledge/2026-10-09-lua-skin-compat-decisions.md:75`). 철거 작업 W1-1(`00-synthesis.md:467`). 이때 20000 대 사설 id 는 모두 사라졌다.
- 2026-10-22: 위 두 대역으로 다시 도입(21000 대). 20000 대를 되살리지 않았고 2026-09-17 의 id 와 겹치지 않는다.
- 원본 id 표는 `tools/gen-skin-property.rs` 가 `SkinProperty.java` 에서 뽑아 커밋한 파일이고 FNV-1a 체크섬 테스트가 손 수정을 막는다(`crates/rbms-skin/src/property/generated.rs:1-12`, 체크섬 `PROPERTY_TABLE_CHECKSUM`, `crates/rbms-skin/tests/property_checksum.rs`). 종류(옵션, 숫자, 비율, 실수, 글자, 타이머, 이미지 인덱스, 오프셋)마다 id 공간이 따로다(`property/mod.rs:40-45`). 원본의 상한은 `ID_LENGTH` 65,536 이다(`names.rs:27-30`).
- 원본이 사용자 영역으로 남긴 대역(새 id 가 피해야 하는 곳):

| 종류 | 원본이 선언한 최대 id | 사용자 정의 대역 |
| --- | --- | --- |
| 옵션 | 3035 | 스킨 설정 항목이 쓰는 옵션 id 는 관례로 900~999(기본 팩 `lua/props.lua:163-164` 의 `CUSTOM_FIRST/LAST`, 테마 920~926) |
| 숫자 | 1327 | 없음 |
| 글자 | 1095 | 없음 |
| 실수 | 1115 | 없음 |
| 버튼(이벤트) | 388 | `EVENT_CUSTOM_BEGIN..END` 1000~1999(`SkinProperty.java:1051-1052`) |
| 타이머 | 2999 | `TIMER_CUSTOM_BEGIN..END` 10000~19999, `set_timer` 만 허용(`SkinProperty.java:175-176`, `lua/main_state.rs:46`) |
| 오프셋 | 199 | `OFFSET_MAX` |
| 이미지 인덱스 | 이름 표 62개(최대 469) | 없음 |

- 조사 당시 후보 규칙과 채택된 규칙의 차이: (1) 20000 이상이 아니라 21000 대로 정했다. (2) 종류별 공간이 따로라 이벤트와 불린에 같은 21000 대 번호가 있어도 충돌하지 않는다. (3) 별도 확장 표를 만들지 않았다: 이벤트는 게이트가 없고 불린은 게이트 밖 경로(`main_state.option`)로 읽혀 `reference_implements` 를 건드릴 필요가 없었다. (4) 게이트 8곳(1.7절)은 고치지 않았다.

---

## 4. IIDX 참고 이미지 묘사

이미지는 `/private/tmp/claude-501/-Users-hyunseokbyun-development-R-BMS/35559457-6343-4207-8f85-ff5841239c87/images/` 의 2.png, 3.png, 4.png 를 직접 보고 확대해서 적었다. 1.png 는 2.png 와 바이트가 같다.

### 4.1 공통 구성 요소

- 키 그림: 흰 건반은 흰 면에 붉은 글씨(분홍 테두리)로 라벨, 검은 건반은 어두운 회색 면에 붉은 라벨. 라벨은 그 키가 하는 일이다(DOWN, UP, ON/OFF, LEFT, RIGHT). 금속 느낌의 어두운 틀로 둘러싼 둥근 사각형이다. 흰 건반은 아래 줄, 검은 건반은 위 줄이고 가로로 엇갈려 놓인다. 기능이 있는 키만 그린다. 건반 번호(흰 1, 3, 5, 7 / 검은 2, 4, 6)는 그림에 적혀 있지 않고 라벨과 원본 키 표의 일치에서 추론한 것이다(4.5절).
- 열마다 색이 있다: 주황, 연두, 보라, 청록(그리고 이미지 3의 빨강). 같은 열의 키 테두리, 키에서 상자로 가는 연결선, 상자 테두리, 상자 배경의 어두운 틴트, 상자 바깥 아래의 작은 이름(STYLE 등)이 같은 색이다.
- 연결선: 직각으로 꺾인 굵은 선(약 3~4 px). 위 키와 아래 키에서 내려와 합쳐진 뒤 상자 윗변 가운데로 들어간다. 한 열이 키 두 개를 가지면 두 선이 한 줄로 합쳐진다. 상자는 키 위치에 맞춰 가로로 엇갈려 놓인다.
- 옵션 상자: 맨 위에 흰 굵은 글씨의 제목 한두 줄(영어 또는 일본어와 영어 병기), 아래에 값 상자를 세로로 전부 쌓는다. 값 상자는 검은 둥근 사각형에 회색 베벨, 흰 굵은 글씨. 고른 값은 주황색 그라데이션 면에 흰 글씨, 가장자리가 밝게 빛난다. 값 목록은 줄이 달라도 모두 한 상자에 전부 보인다(스크롤 없음).
- 상단 INFORMATION 줄(이미지 3·4): 화면 윗부분에 좌측이 기울어진 노란 라벨 "INFORMATION", 아래에 노란 테두리의 검은 띠가 있고 그 안에 방금 바꾼 옵션의 설명이 한 줄로 나온다. 이미지 3은 "RANDOMがONになりました。ランダムに譜面のレーンが入れ替わります"(RANDOM 이 켜졌다, 레인이 무작위로 바뀐다). 이미지 4는 "白鍵盤と黒鍵盤で詳細オプションを設定します"(흰 건반과 검은 건반으로 상세 옵션을 설정한다). 오른쪽 위에 흰 작은 네모 안 숫자와 함께 "1 キーフィルター", "2 ソート", "3 LNモード変更"(키 필터, 정렬, LN 모드 변경)이 있다.
- 좌측 도움말 패널(이미지 3·4): 화면 왼쪽에 세로로 쌓인 상자. 기울어진 색 라벨(제목) + 검은 본문. 설명 글을 보여 준다.
- 배경: 곡 선택 화면이 어둡게 눌려 흐릿하게 비친다.

### 4.2 이미지 2 (4열: STYLE / GAUGE / ASSIST / RANGE)

- 키 그림: 검은 건반 3개(UP)가 위 줄, 흰 건반 4개가 아래 줄. 위 줄은 왼쪽부터 열 1(주황), 열 2(연두), 열 3(보라)에 하나씩 달려 있다. 아래 줄의 흰 건반은 열 1·2·3 이 "DOWN" 라벨, 열 4(청록)는 라벨이 없는 빈 흰 건반이다. 열 4 에는 검은 건반이 없다.
- 상자 1 주황 "Note Chart Position"(`STYLE`): OFF(선택), RANDOM, R-RANDOM, S-RANDOM, MIRROR(5개).
- 상자 2 연두 "Groove Gauge Reduction"(`GAUGE`): OFF(선택), ASSISTED EASY, EASY, HARD, EX HARD(5개. 이 판에서 OFF 가 NORMAL 이다).
- 상자 3 보라 "Assisted Play Feature"(`ASSIST`): OFF(선택), AUTO SCRATCH, LEGACY NOTE, A-SCR & LEGACY(4개).
- 상자 4 청록 "Note Chart Display Area"(`RANGE`): OFF(선택), SUDDEN+, HIDDEN+, SUD+ & HID+, LIFT, LIFT & SUD+(6개).
- 상자 이름 라벨(STYLE, GAUGE, ASSIST, RANGE)은 상자 바깥 아래에 상자 색의 작은 글씨로 있다.

### 4.3 이미지 3 (SP/DP, 게이지, 스크롤 자동설정)

- 키 그림: 검은 건반 3개가 위 줄에 있다. 왼쪽 UP(주황), 가운데 DOWN(빨강 테두리), 오른쪽 UP(주황). 흰 건반 4개가 아래 줄에 있고 모두 DOWN 라벨이다: 주황, 연두, 보라, 주황 순. 이 배열은 원본 키 표와 같다(4.5절).
- 상자(왼쪽부터): 
  1. 주황 "譜面の配置(SP左右レーン)"(차트 배치, SP 좌우 레인): NORMAL, MIRROR, RANDOM(선택), R-RANDOM, S-RANDOM, SPIRAL, H-RANDOM, ALL-SCR, RANDOM+, S-RAN+ (10개).
  2. 연두 "グルーブゲージ減り幅"(게이지 감소 폭): ASSISTED EASY, EASY, NORMAL, HARD(선택), EX HARD, HAZARD (6개).
  3. 보라 "スクロールスピード自動設定"(스크롤 속도 자동 설정): OFF, START BPM, MAX BPM, MAIN BPM(선택), MIN BPM (5개).
  4. 주황 "譜面の配置(DP右レーン)": 1번 상자와 같은 10개, NORMAL 선택.
  5. 빨강 "ダブルプレイ"(더블 플레이, 화면 가운데 아래, 상자 열의 맨 아래 쪽에 다른 상자보다 낮게 놓임): OFF(선택), FLIP, BATTLE, BATTLE AS (4개).
- 가운데 위 검은 DOWN 키는 이 빨강 상자와 같은 빨강 테두리로 짝지어져 있다. 둘 사이의 연결선은 사진에서 보이지 않는다.
- 키 그림 양옆에 흰 테두리의 "PREVIEW" 작은 레인 그림이 둘 있다(왼쪽 SP, 오른쪽 DP). 7건반과 턴테이블이 아래에 있고 흰 노트와 파란 노트, 붉은 스크래치 선이 위에 있어서 선택한 배치 결과를 미리 보여 준다.
- 왼쪽 위 "TARGET SCORE GRAPH" 패널: 빨강 테두리 라벨과 "ターンテーブルを回して表示したいスコアグラフを選択"(턴테이블을 돌려 표시할 스코어 그래프를 고른다). 아래에 검은 둥근 사각형 11줄 목록이 있고 위아래 흰 삼각형 표시가 끝에 있다: PACEMAKER AA+, AA-, AA, A+, A-, A(선택: 주황 테두리가 바깥으로 튀어나오고 오른쪽 끝에 주황 네모), NEXT, MAX, AAA+, AAA-, AAA. 글자는 주황빛 노랑 굵은 글씨.
- 왼쪽 아래 "GROOVE GAUGE" 패널: 연두·노랑 라벨. 본문은 노란 글씨 "HARDがONになりました / ゲージの減少が激しくなります"(HARD 가 켜졌다 / 게이지 감소가 심해진다). 그 아래 청록 테두리 상자 "ゲージが無くなるとゲームオーバー"(게이지가 없어지면 게임 오버)와 붉은 게이지 막대 그림, 좌우 작은 건반 그림.

### 4.4 이미지 4 (상세 옵션)

- 윗줄 안내 띠: 오른쪽 INFORMATION 아래에 "自動 / START+SELECT 詳細オプション変更"(자동 / 상세 옵션 변경) 표지.
- 키 그림은 전부 청록이다. 검은 건반 3개(위 줄): DOWN, LEFT, RIGHT. 흰 건반 4개(아래 줄): ON/OFF, ON/OFF, LEFT, RIGHT. 원본 키 표와 같다(4.5절).
- 상자: 
  1. 왼쪽 위 "ゲージオートシフト(GAS)": NONE, CONTINUE, HARD TO GROOVE(선택), BEST CLEAR, SELECT TO UNDER (5개). 검은 DOWN 키와 연결.
  2. 가운데 위 넓은 "ノーツ表示時間(NOTES DISPLAY TIME)": 값 상자가 한 줄이고 양 끝에 붉은 삼각형 화살표가 하나씩 있다(왼쪽 끝은 왼쪽을, 오른쪽 끝은 오른쪽을 가리킨다). 가운데 "489ms"(흰) 와 "293"(연두, 녹색 숫자). 검은 LEFT 키와 검은 RIGHT 키가 이 상자 아래 두 곳에 연결된다.
  3. 오른쪽 가운데 "判定アルゴリズム(ALGORITHM)": COMBO(선택), SCORE, NOTES (3개). 연결선이 없다.
  4. 왼쪽 아래 "BGAの表示(BGA SHOW)": ON(선택), AUTO, OFF (3개). 흰 첫 번째 ON/OFF 키와 연결.
  5. 가운데 아래 "TIMING AUTO AJUST(判定タイミング自動調整)"(원문 철자 그대로): OFF(선택), ON. 흰 두 번째 ON/OFF 키와 연결.
  6. 오른쪽 아래 넓은 "判定タイミング調整(JUDGE TIMING)": 숫자 한 줄 "-29ms", 양 끝에 붉은 삼각형 화살표(2번 상자와 같은 구조), 그 아래 붉은 글씨 세 줄 "FASTが多く出る場合は-方向へ / SLOWが多く出る場合は+方向へ / 調整してください"(FAST 가 많이 나오면 - 방향, SLOW 가 많이 나오면 + 방향으로 조정). 흰 LEFT·RIGHT 키 둘이 이 상자에 연결된다.
- 좌측 도움말: 위 "GAUGE AUTO SHIFT"(보라 라벨, 설명 "ゲージオートシフトの種類 途中落ちがなくなります"). 그 아래 4개 설명 상자(BEST CLEAR, SELECT TO UNDER, HARD TO GROOVE, CONTINUE 각각 세리프 노랑 제목과 두 줄 설명). 맨 아래 "GROOVE GAUGE"(보라 라벨, "各プレイヤーのハードゲージの減少量")와 LR2, beatoraja, IIDX 의 HARD 게이지 감소량을 비교한 표(행 60~, 50, 40, 30, 20, 10, 열 BD·PR·EP).

### 4.5 원본 키 표와의 대응 (핵심 발견)

이미지 3·4 의 키 그림은 R-BMS 가 이미 쓰는 원본 키 표(1.3절)와 건반 단위로 같다. 흰 건반 = 1, 3, 5, 7번, 검은 건반 = 2, 4, 6번이다.

| 건반 | 이미지 3(패널 1) | R-BMS 패널 1 | 이미지 4(패널 3) | R-BMS 패널 3 |
| --- | --- | --- | --- | --- |
| 1 흰 | 주황 DOWN: SP 배치 | RANDOM 다음 | ON/OFF: BGA | BGA |
| 2 검 | 주황 UP: SP 배치 | RANDOM 이전 | DOWN: GAS | GAS 다음 |
| 3 흰 | 연두 DOWN: 게이지 | GAUGE 다음 | ON/OFF: TIMING AUTO ADJUST | AUTO ADJUST |
| 4 검 | 빨강 DOWN: 더블 플레이 | DP OPTION 다음 | LEFT: 표시 시간 | NOTE DISPLAY TIME 줄임 |
| 5 흰 | 보라 DOWN: 스크롤 자동 | HI-SPEED FIX 다음 | LEFT: 판정 타이밍 | JUDGE TIMING -1 |
| 6 검 | 주황 UP: DP 배치 | RANDOM 이전(2P) | RIGHT: 표시 시간 | NOTE DISPLAY TIME 늘림 |
| 7 흰 | 주황 DOWN: DP 배치 | RANDOM 다음(2P) | RIGHT: 판정 타이밍 | JUDGE TIMING +1 |

이미지 3의 두 주황 줄(1P 배치, DP 오른쪽 배치)은 R-BMS 에서 한 값(`play.random`)을 가리킨다. 이미지 4 의 ALGORITHM 상자는 키가 없고 표시만 한다(R-BMS 에는 키 자리가 따로 없다). 이미지 2 는 이 표와 다른 판(4열 STYLE, GAUGE, ASSIST, RANGE 가 키 두 개씩)이라 R-BMS 에서 그대로 옮기려면 키 표를 바꿔야 한다(6절 결정 1).

### 4.6 R-BMS 값과 대응

| IIDX 상자 | R-BMS 대응 | 차이 |
| --- | --- | --- |
| STYLE / SP 배치 | RANDOM | IIDX 는 RANDOM+, S-RAN+, SPIRAL 이 있다. R-BMS 는 8개이고 ROTATE 가 있다 |
| GAUGE | GAUGE | 여섯이 같다. IIDX 는 NORMAL 대신 OFF 로 쓰는 판이 있다 |
| ASSIST (A-SCR, LEGACY) | SCRATCH AUTO + LEGACY NOTE | 두 불린의 조합 4가지. R-BMS 의 LEGACY NOTE 는 화면 전용이고 SCRATCH AUTO 는 스킨이 읽을 id 가 없다 |
| RANGE (SUDDEN+, HIDDEN+, LIFT 조합) | LANE COVER ON, HIDDEN+ ON, LIFT ON | 세 불린의 조합 6가지. 이벤트 330~332 와 인덱스 330~332 가 이미 있다 |
| 스크롤 속도 자동 설정 | HI-SPEED FIX | 다섯이 같다 |
| 더블 플레이 | DP OPTION | 값 넷이 같다. 플레이에 적용되지 않는다 |
| TARGET SCORE GRAPH | TARGET | 11 대 16. 이름 목록용 글자 id 3·200~219 가 이미 있다 |
| GAS | GAUGE AUTO SHIFT | 다섯이 같다. `HARD TO GROOVE` 는 R-BMS 에서 `SURVIVAL TO GROOVE` |
| NOTES DISPLAY TIME | NOTE DISPLAY TIME | KEEP GREEN NUMBER 가 꺼져 있으면 움직이지 않는다 |
| ALGORITHM(COMBO, SCORE, NOTES) | JUDGE ALGORITHM(COMBO, DURATION, LOWEST, SCORE) | 이름 대응이 다르다 |
| BGA SHOW(ON, AUTO, OFF) | BGA(ON, OFF) | AUTO 에 닿지 못한다 |
| TIMING AUTO AJUST | AUTO ADJUST | 같다 |
| JUDGE TIMING | JUDGE TIMING | 같다. 부호 설명도 같은 방향(SLOW 가 많으면 +) |

---

## 5. 기본 스킨 Lua 로 구현할 때의 가능성과 한계

5절은 구현 전 조사다. 구현은 5.1절의 방법을 거의 그대로 썼다(값 목록 전체를 세로로 나열, `draw` 함수로 강조, 사각형 키 그림과 연결선, 글자 `value` 함수의 INFORMATION 줄, `timer(21)` 로 기준값을 새로 잡는 감시). 결과는 1.8절이고, 5.2~5.6절에서 달라진 곳은 해당 줄에 덧붙였다.

### 5.1 가능한 것(Rust 변경 없이)

| 하려는 것 | 방법 | 근거 |
| --- | --- | --- |
| 값 목록 전체를 세로로 나열 | 값마다 `s:rect` 상자와 `s:text`(상수 글자). 값 개수와 이름은 Lua 표(`theme.random_name` 등) | `lua/ui.lua:219-285` |
| 고른 값만 주황 강조 | 값마다 주황 사각형을 놓고 `draw = function() return main_state.event_index(ID) == i end`. 목록의 함수 값은 프레임마다 불리지만 비용은 호출 하나당 약 0.0004 ms(측정 평균 0.022 ms / 59회) | `ui.lua:36`(`draw` 필드), `lua/select/info.lua:244`(실제로 `draw` 함수를 쓰는 곳) |
| 같은 효과를 imageset 으로 | 원본 방식. 값 하나당 이미지 한 장이 필요해 상자 하나에 이미지 N 장. 이미지를 새로 구워야 해서(`tools/generate-images.py`) 비추천 | `draw.rs:279-323`(`slot`: 인덱스가 음수면 안 그림, 범위를 넘으면 첫 이미지) |
| 여러 불린의 조합 강조(RANGE 6가지, ASSIST 4가지) | 값별 `draw` 함수가 인덱스 330·331·332 나 304 를 조합해 참 거짓을 낸다 | 위와 같다 |
| 열 색(주황, 연두, 보라, 청록) | `theme.tone.orange (240,136,90)`, `theme.tone.green (89,209,133)`, `theme.tone.violet (178,107,240)`, `theme.accent.sky (76,201,240)` | `theme.lua:32-38,93-101` |
| 키 그림과 연결선 | 건반은 사각형 + 라벨 글자, 연결선은 가는 사각형(꺾이는 선은 사각형 둘). 삼각형은 `angle` 로 돌린 정사각형 둘이나 글자 `<` `>` | `ui.lua:88-117`(`angle`) |
| 숫자 칸(NOTE DISPLAY TIME, JUDGE TIMING) | 글자 `value` 함수가 `main_state.number(312/313/12)` 를 읽어 `"489 MS"` 꼴로. 이미 `sheet.lua:26-62` 가 같은 일을 한다 | `sheet.lua:26-62` |
| TARGET 목록(IIDX 의 11줄) | 글자 id 200~209, 3, 210~219 를 `ref` 로 읽어 21줄까지 | `skin_host/options.rs:348-359`, `lua/props.lua:259-280` |
| INFORMATION 줄 | 글자 `value` 함수가 이전 프레임의 값을 클로저에 기억해 바뀐 옵션을 알아내고 정해진 문장을 돌려준다. 열릴 때(`timer(21)`이 켜질 때) 기준값을 새로 잡으면 다른 길로 바뀐 값이 "방금 바꾼 것"으로 보이지 않는다 | `lua/select/side.lua:144-147`(`timer_observe_boolean`) |
| DP 칸 숨기기 | 선택된 차트가 14키·10키일 때만 그리는 `op = {162}`/`{163}`. 옵션 160~164(차트 모드)는 선곡에서 선택 차트로 답한다 | `skin_host/chart.rs:222-227`, `props.lua:98-102` |
| 틀의 위치 | 지금처럼 `PANELn_ON`/`OFF` 타이머와 옵션 21~23 으로 올림과 내림 | `sheet.lua:125-133` |

### 5.2 한계

1. 원본에 없는 옵션의 값은 읽을 수 없다. `event_index` 가 오류를 던지고(`lua/main_state.rs:927`) imageset 은 첫 이미지를 항상 보인다(`draw.rs:291-300`). 해당: SCRATCH AUTO, KEEP GREEN NUMBER, SCRATCH SIDE, AUTOPLAY, TARGET FALLBACK, TARGET RATE, AUTO REPLAY, ESCAPE, TOTAL, JUDGE WIDTH, LN MARGIN(1.7절 표 밖). 키로 바꾸는 것도 Lua 가 못 한다. 패널 키는 Rust 가 읽고, Lua 는 `event_exec` 를 단추 클릭이나 자기 함수에서만 부른다.
2. 색은 destination 마다 고정이다. 값에 따라 글자 색을 바꾸는 방법이 없고, 강조는 사각형을 하나 더 얹는 식이다(`ui.lua:88-117`). Mono/Slate, Accent 옵션과 열 색을 어떻게 맞출지는 설계 선택이다.
3. 글꼴은 지금 `Inter-Regular.ttf` 하나다(`theme.lua:235-238`). 엔진을 바꾸지 않고 굵은 글씨를 내는 길은 있다: 스킨 팩 `font/` 에 정적 굵은 TTF(예: Inter Bold, OFL)를 더하고 `theme.font` 에 두 번째 글꼴 id 로 선언하면, `TextContext::load_font` 가 파일의 굵기(`info.weight`)로 별도 face 를 등록한다(`crates/rbms-render/src/font.rs:410-427`, 같은 가족의 다른 굵기는 이름에 굵기가 붙는다). 같은 가변 파일로 굵기만 바꾸는 길은 스킨 글꼴 정의(`FontDef`, `crates/rbms-skin/src/model.rs:345-351`)에 굵기 칸이 없어 엔진 확장이 필요하다. 구현은 이 방법(정적 굵은 TTF 추가)을 택했다: 팩 `font/Inter-Bold.ttf` 와 글꼴 id `inter-bold`(1.8절, `docs/reference/font-rendering.md` 14절 결정 4). 글꼴 품질 문제 전반은 그 문서에 있다.
4. 차트에 의존하는 PREVIEW 레인 그림(선택한 RANDOM 결과를 보여 줌)은 Lua 로 만들 수 없다. 차트 패턴과 시드가 스킨에 노출되지 않는다.
5. 한 패널의 destination 은 올림·내림 각각 하나씩 두 벌이라 두 배다(`both`).
6. 사용자가 고친 `sheet.lua` 가 있으면 새 모듈과 섞인다(1.10절 13).
7. 패널은 키를 누르고 있는 동안만 보인다. 마우스로 값을 누르는 경로는 없다(패널 요소에는 `act` 가 없다). 값 목록을 눈으로 읽는 시간이 짧은 것이 IIDX 와 다르다.

### 5.3 새 옵션을 패널에 넣으려면 Rust 쪽에 필요한 것

| 옵션 | 스킨이 값을 읽는 길 | 값을 바꾸는 길 | 패널 키 |
| --- | --- | --- | --- |
| LANE COVER·LIFT·HIDDEN+ 켬/끔 | 있음(인덱스 330~332) | 있음(이벤트 330~332, `events.rs:265-267`) | 없음. `SelectKey` 변종과 키 표 슬롯, `press_act` 한 줄 |
| JUDGE ALGORITHM, LN MODE, BOTTOM SHIFTABLE, GUIDE SE | 있음 | 있음 | 위와 같다 |
| SCRATCH AUTO, KEEP GREEN NUMBER, SCRATCH SIDE, AUTOPLAY, TARGET FALLBACK·RATE | 없음. 이미지 인덱스로 읽게 하려면 새 인덱스 id, `SettingsView` 필드 + `image_index` 분기(`options.rs:133-168,280-315`) + `ROUTES` 한 줄(`skin_host/mod.rs:347-365`) + 게이트 통과(`reference_implements` 8곳). 켬·끔 둘뿐인 것은 불린 id 를 호스트가 답하고 Lua `draw` 함수가 `main_state.option` 으로 읽는 길이면 게이트를 고치지 않는다(1.7절) | 없음. `SettingEvent` 변종 + `row()` 매핑(`events.rs:75-145`) + `EVENTS` 한 줄(`events.rs:237-272`) + `EVENT_ROUTES` 한 줄(`mod.rs:524-557`). 이벤트 공간은 게이트가 없다 | 위와 같다 |
| 2P RANDOM 따로 | 인덱스 43 은 있으나 1P 와 같은 값이다 | `play.random_2p` 필드와 플레이 쪽 적용이 필요하다 | 있음(키 6·7) |
| DP OPTION 실제 적용 | 있음 | 있음 | 있음 |

키 표를 바꾸면 영향이 가는 곳: `stage/select/keys.rs` 의 표와 `*_PANEL_PRESSES`, `panel.rs` 의 `press_act`, 키 가이드 `stage/select/guide.rs:50-58,224-246`(`KEY_ROLES` 와 패널·키 행), 테스트 `keys.rs:311-333`(원본과 같은 표라고 단언)와 `panel.rs:375-452`. 키 설정 화면의 패널 행은 따로 없다(패널 키는 건반이 곧 역할이다).

패널 키를 Lua 가 받는 길은 이론상 있다. `main_state.key_pressed(code)`(`lua/main_state.rs:50`, 호스트 `skin_host/mod.rs:983`)로 눌린 키를 읽고, 함수 값 안에서 `main_state.event_exec(id)` 를 부르면 이벤트가 큐에 쌓여 다음 프레임에 실행된다(`skin_host/mod.rs:13-19`). 그러나 키 코드가 libGDX 이름이고 사용자가 바꾼 키 설정을 모르며, 같은 키를 Rust 가 이미 패널 역할로 쓰므로 이중 동작이 된다. 권하지 않는다.

구현 결과(2026-10-22): 위 표의 SCRATCH AUTO, KEEP GREEN NUMBER, 켬끔 3종, JUDGE ALGORITHM 이 들어갔다(1.3, 3.4절). 키 표 변경의 영향은 `keys.rs` 표와 `*_PANEL_PRESSES`, `panel.rs` 의 `press_act`, 키 가이드 `stage/select/guide.rs` 의 `panel_rows` 에 세 줄 추가("{START} + TURNTABLE = TARGET", "{SELECT} + TURNTABLE = HIDDEN+ ON/OFF", "{START}+{SELECT} + TURNTABLE = JUDGE ALGORITHM", 건반 1~7 의 패널 역할은 스킨이 그린다)이고, 가이드 화면 크기 테스트가 통과했다.

### 5.4 좌표 예산

- 기본 스킨 팩 캔버스는 1920x1080(`theme.lua:5-6`). 기본 창은 1280x720(`window.rs`의 `Hd720`)이고 `letterbox` 기본이 켜져(`schema.rs`의 `DEFAULT_LETTERBOX`) 2/3 배로 줄어 그려진다. 글자 크기 표(`theme.lua:206-213`)가 1280x720 창에서 `display` 96 은 64 px, `title` 64 는 43, `heading` 44 는 29, `body` 32 는 21, `label` 24 는 16, `caption` 18 은 12 px 이 된다.
- 앱 내장 화면(F1 오버레이, PLAY 메뉴, 키 가이드 등)은 1280x720 고정 좌표다(`apps/rbms-player/src/lib.rs:149-150`의 `CW`·`CH`). 이 폭을 지켜야 하는 것은 F1 오버레이를 손볼 때뿐이다. 패널을 스킨에서 그리는 한 해당 없다.
- 현재 패널 영역은 x 64~1856, y 312~768. 머리 선은 y 104, 바닥 선은 y 1000, 양쪽 여백 64(`theme.lua:217-226`). 머리 선과 바닥 선 사이는 높이 896 이다.
- IIDX 식 배치의 크기 감(설계 입력 초안, 확정 아님): 이미지 3의 상자 열은 폭 약 15%, 가장 긴 열은 값 10개다. 1920 기준으로 값 상자 pitch 44(높이 40 + 간격 4), 열 폭 200, 열 간격 24 이면 다섯 열이 1096 폭, 가장 긴 열(RANDOM 8~10개, 제목 80)이 약 480~520 높이다. 위에 INFORMATION 줄 56, 키 그림 150 을 얹으면 y 120~880 으로 선곡 화면의 머리 선·바닥 선 사이(104~1000)에 들어간다. 왼쪽 TARGET 목록(21줄 x 34 = 714 + 제목)과 오른쪽 도움말을 더해도 가로 1792 에 들어간다. 1280x720 창에서 값 상자 한 줄은 약 29 px, 글자 `label` 은 16 px 이다.
- 객체 수: 값 상자마다 상자, 글자, 강조 = 3 개를 올림·내림 두 벌 = 6 destination. 값이 패널 1에 29개 안팎이면 약 175 개 + 틀·제목·키 그림·연결선·도움말 약 150 개로 패널 1이 약 330 개(계산). 세 패널 합계 약 800 개, 선곡 전체 약 1,100 개로 추정된다(계산). 비교: 현재 503 개, 외부 풀 스킨 선곡 1,914 개가 예산 안에서 돈다(`docs/skin.md` 4.3절). 프레임 호출 한도는 16,384회이고 현재 59회다.

### 5.5 Lua 예산 수치(`crates/rbms-skin/src/lua/budget.rs:20-70`)

로드 한 패스 1억 명령·10초, 프레임당 함수 호출 16,384회·Lua 안에서 50 ms, 호출 하나 100만 명령, 메모리 256 MiB. 현재 선곡: 로드 6.5 ms, Lua 메모리 2,082 KiB.

### 5.6 깨지는 테스트와 골든의 범위

- 패널 모양을 픽셀로 비교하는 골든은 없다. 패널 캡처(`stage/capture.rs:1428-1450`의 `select-panel0~3` 등)는 환경 변수 `RBMS_SKIN_PACK` 과 `RBMS_SKIN_CAPTURE_DIR` 를 줘야만 파일을 쓰고, 파일을 비교하지 않는다. `crates/rbms-render/tests/golden/*.png` 는 렌더 프리미티브용이다.
- `sheet.lua` 의 글자(KEY n, NOT AVAILABLE 등)를 단언하는 Rust 테스트는 없다(`grep` 확인).
- 기본 스킨 Lua 를 지키는 것:
  1. 엄격 덤프 게이트 `apps/rbms-cli/src/skin_dump/tests.rs:491-540`(`the_default_skin_pack_in_the_repository_dumps_clean_under_the_strict_gate`): 문서 9개 로드와 종류, 경고 0, pcall 이 삼킨 오류 0, 함수 값 실패 0, 프레임 예산 초과 0. 새 id 를 Lua 가 `event_index` 로 읽는데 호스트가 허용하지 않으면 여기서 실패한다.
  2. 같은 명령: `cargo run -q -p rbms-cli --locked -- skin-dump assets/skins/rbms-default --strict --frames 120`(`docs/HANDOFF.md` 7절).
  3. `crates/rbms-skin/tests/default_pack.rs`: Lua 가 `io`, `os`, `luajava`, `debug`, `package`, `math.random`, 네트워크를 쓰지 않는지 읽고, 그림 시트가 표대로인지 본다.
  4. `apps/rbms-player/src/stage/capture/gate.rs`: 선곡을 640x360 으로 그려 SongList, Text, Image 종류와 경고 0 을 단언한다.
  5. `apps/rbms-player/src/assets/default_pack/tests.rs`: 설치 규칙. 파일 목록은 빌드 스크립트가 만든다(`apps/rbms-player/build.rs`).
  6. 그림 시트나 아틀라스를 바꾸면 `uv run assets/skins/rbms-default/tools/generate-images.py --check`.
- 키 표나 이벤트를 바꾸면 깨지는 것: `stage/select/keys.rs:311-333`, `panel.rs:375-452`, `panel_tests.rs`(고정 스킨으로 건반을 눌러 설정 값을 본다), `events_tests.rs`, `skin_host/options/tests.rs`. 원본 id 표(`reference_implements`)를 건드리면 `crates/rbms-skin/tests/property_checksum.rs`, `property/names.rs:551-` 의 개수 테스트가 깨진다.
- 구현 뒤 추가된 보호(2026-10-22): `keys.rs` 의 바뀐 칸 목록 단언(`CHANGED_SLOTS`), `panel_tests.rs`(신규 `a_turntable_direction_held_before_the_panel_came_up_is_not_a_press_on_it`, `the_third_panel_leaves_the_travel_time_alone_while_the_cursor_is_off_a_chart`), `skin_host/extension.rs` 의 대역 테스트, `skin_host/select/tests.rs` 의 닿지 않는 키 옵션, `app_options.rs` 의 홀드 키 테스트. 패널 캡처 하네스는 `stage/capture/panels.rs`(`the_default_packs_option_panels_and_graded_lamps_are_captured`, `RBMS_SKIN_CAPTURE_DIR` 가 있을 때만 파일을 쓰고 비교하지 않는다).
- 수동 점검표 `docs/quality-assurance/2026-10-11-default-skin-manual-check.md:223`(C1)은 패널 문구(`NOT AVAILABLE`)를 쓴다. 그림이 바뀌면 문구를 고친다.
- 고칠 문서: `docs/play-options.md`(2, 6절의 패널 설명), `docs/skin.md`(6절 알려진 한계), `docs/plan/2026-10-11-default-skin-design.md` 6.2절, `docs/crates.md`.

---

## 6. 결정 지점

각 항목은 선택지, 메인 추천, 이유, 영향 범위다.

아래 배치 초안은 조사 당시의 예(결정 1 은 A, 결정 2 는 B, 결정 6 은 A)이고, 구현된 배치는 1.3절 표다. 초안과 다른 곳은 패널 1 키 6(RANDOM 이전 대신 HI-SPEED FIX 이전)과 패널 2 턴테이블(HIDDEN+ 를 두었다)이다. "기존"은 지금 그대로, "표 변경"은 키 표 슬롯만 바꾸는 것(역할 이름은 이미 있다), "새 id"는 3.4절의 확장이 필요한 것이다.

| 패널 | 건반 | 옵션 | 분류 |
| --- | --- | --- | --- |
| 1 START | 1·2 | RANDOM 다음·이전 | 기존 |
| 1 START | 3 | GAUGE 다음 | 기존 |
| 1 START | 4 | GAUGE 이전(DP OPTION 자리를 비운 것) | 표 변경(`SelectKey::GaugeUp` 은 `keys.rs:38` 에 이미 있고 `press_act` 도 매핑돼 있다) |
| 1 START | 5 | HI-SPEED FIX 다음 | 기존 |
| 1 START | 6·7 | RANDOM 이전·다음(2P 자리) 또는 HI-SPEED FIX 이전(`HsFixUp`) | 기존 또는 표 변경 |
| 1 START | 턴테이블, 휠, 방향키 | TARGET | 기존 |
| 2 SELECT | 1 | EX JUDGE(읽기 전용 표시) | 기존 |
| 2 SELECT | 2 | CONSTANT | 기존 |
| 2 SELECT | 3 | SCRATCH AUTO(JUDGE AREA 자리) | 새 id + 새 이벤트 |
| 2 SELECT | 4 | LEGACY NOTE | 기존 |
| 2 SELECT | 5 | KEEP GREEN NUMBER(MARK NOTE 자리) | 새 id + 새 이벤트 |
| 2 SELECT | 6·7 | LANE COVER, LIFT 켬끔(BPM GUIDE, NO MINE 자리) | 표 변경(이벤트 330·331 과 인덱스는 있음) |
| 3 START+SELECT | 1~7 | BGA, GAS, AUTO ADJUST, NOTE DISPLAY TIME 줄임·늘림, JUDGE TIMING -·+ | 기존 |
| 3 START+SELECT | 턴테이블 | JUDGE ALGORITHM 이전·다음(IIDX 이미지 4 의 ALGORITHM 상자가 키와 이어지지 않은 점에 맞춤) | 표 변경(이벤트 340 과 인덱스는 있음) |

HIDDEN+ 켬끔, LN MODE, BOTTOM SHIFTABLE, GUIDE SE 는 자리가 모자라 이 초안에 없었다. 구현은 HIDDEN+ 켬끔을 패널 2 턴테이블에 두었고, LN MODE, BOTTOM SHIFTABLE, GUIDE SE 는 넣지 않았다. 패널 2·3 의 턴테이블이나 쪽 넘기기(턴테이블로 두 번째 쪽)를 쓰면 더 넣을 수 있다.

1. 패널 키 배치(어떤 옵션을 어느 키에)
   - A. 원본 키 표의 역할 이름과 이미 동작하는 칸은 그대로 두고, 죽은 칸(패널 2의 5칸, 패널 1 의 DP OPTION)과 역할만 있고 키가 없는 칸(`GaugeUp`, `HsFixUp`, 패널 2·3 의 턴테이블)에만 새 토글을 배정한다. 표의 슬롯은 바뀌므로 `keys.rs:311-333` 의 "원본과 같은 표" 단언은 고쳐야 한다.
   - B. IIDX 이미지 2 처럼 열별 키 두 개(STYLE 1·2, GAUGE 3·4, ASSIST 5·6, RANGE 7)로 R-BMS 전용 표를 쓴다.
   - C. 원본 표를 그대로 두고 R-BMS 전용 토글을 네 번째 패널(새 조합)에 모은다.
   - 추천: A(+ 필요하면 C). 이유: 이미지 3·4 의 키 그림이 원본 표와 같아서(4.5절) IIDX 모양을 얻는 데 표 변경이 필요 없고, ModernChic 같은 외부 스킨의 고정 라벨과 어긋나지 않는다. 패널 2(SELECT)는 IIDX 의 ASSIST 와 뜻이 같아 SCRATCH AUTO 를 두기에 맞다.
   - 영향: A 는 `keys.rs`, `panel.rs`, `guide.rs`, 해당 테스트 소수. B 는 표와 테스트 전면, 외부 스킨 호환 깨짐. C 는 여는 입력(새 조합)과 `Panel` 변종 추가.
   - 결정됨(2026-10-22, 메인이 정함, 사용자 미확인): A. 원본 표의 동작하는 칸은 그대로 두고 죽은 칸과 키 없는 역할에만 토글을 배정했다(1.3절). 초안과 다른 점은 패널 1 키 6 이 HI-SPEED FIX 이전, 패널 2 턴테이블이 HIDDEN+ 다. `keys.rs` 의 "원본과 같은 표" 단언은 바뀐 칸 목록 `CHANGED_SLOTS` 로 고쳤다. ADR [옵션 패널 배치](../acknowledge/2026-10-22-adr-option-panel-layout.md).

2. 새로 넣을 토글 목록
   - A. SCRATCH AUTO, KEEP GREEN NUMBER, LANE COVER·LIFT·HIDDEN+ 켬끔, JUDGE ALGORITHM, LN MODE, BOTTOM SHIFTABLE, GUIDE SE.
   - B. A 에서 id 가 필요한 것(SCRATCH AUTO, KEEP GREEN NUMBER)만 먼저, 나머지는 원본 id 로 바로 가능한 것부터 단계별.
   - C. A 에 AUTOPLAY, SCRATCH SIDE, TARGET FALLBACK·RATE 까지.
   - 추천: B. 이유: 원본 id 가 이미 있는 것(330~332, 340, 308, 341, 343)은 Rust 가 키 역할만 더하면 된다. AUTOPLAY 는 출하 ON 이고(`schema.rs:588`) 켜진 채 저장되면 판이 기록에서 빠지므로 패널에 넣지 않는다. JUDGE WIDTH, TOTAL 은 어시스트 판을 만들어 패널에 부적합하다.
   - 영향: 항목 수만큼 `SettingEvent`·`SelectKey`·표 슬롯·가이드 행·테스트.
   - 결정됨(2026-10-22, 메인이 정함, 사용자 미확인): A 의 목록 가운데 SCRATCH AUTO, KEEP GREEN NUMBER, LANE COVER·LIFT·HIDDEN+ 켬끔, JUDGE ALGORITHM 을 구현했다. 키 자리가 없어 넣지 않은 것: LN MODE, BOTTOM SHIFTABLE, GUIDE SE(스킨 id 와 이벤트는 있다). C 의 AUTOPLAY, SCRATCH SIDE, TARGET FALLBACK·RATE 는 하지 않았다. ADR [옵션 패널 배치](../acknowledge/2026-10-22-adr-option-panel-layout.md).

3. 스킨이 읽는 id(원본에 없는 옵션의 값)
   - A. R-BMS 확장 id 대역(예 20000 이상, 3.4절)을 되살리고 별도 확장 표로 게이트를 통과시킨다.
   - B. 원본이 선언했지만 R-BMS 에 대응 기능이 없는 id(303, 305, 306, 307, 350 등)를 재사용한다.
   - C. 값 읽기를 포기하고 라벨만 고정하는 읽기 전용 토글로 둔다(표시 없음).
   - A'(종합 검토 추가). 확장 대역을 불린 공간에만 두고 Lua `draw` 함수가 `main_state.option(id)` 로 읽는다. 게이트 8곳은 그대로이고 호스트 불린 응답 몇 줄만 더한다(1.7절). 두 값 옵션에만 쓸 수 있다.
   - 추천: A, 두 값 옵션(SCRATCH AUTO, KEEP GREEN NUMBER)뿐이면 A' 로 시작. 이유: B 는 외부 스킨이 그 id 의 라벨을 굽고 있어 잘못된 라벨을 보이게 된다. C 는 IIDX 식 값 표시와 모순된다. 단 2026-10-09 결정이 사설 id 를 폐기했으므로 사용자가 다시 확인해야 한다.
   - 영향: `crates/rbms-skin`(`names.rs`, `lua/main_state.rs`, `loader/track.rs`)과 `rbms-render`(`draw.rs`, `text.rs`, `songlist/build.rs`)의 게이트 8곳, `skin_host/mod.rs` 의 `ROUTES`·`EVENT_ROUTES`, `options.rs`, 테스트 체크섬 주변, `docs/reference/skin-compat/`.
   - 결정됨(2026-10-22, 메인이 정함, 사용자 미확인): A'. 이벤트는 21000~21099(스킨이 `event_exec` 로 부름), 불린은 21100~21199(Lua `main_state.option(id)` 로 읽음)이고 게이트 8곳과 원본 id 표는 건드리지 않았다. 20000 대가 아니라 21000 대다. 2026-10-09 사설 id 철거 결정의 부분 변경이라 사용자 확인이 필요하다. 3.4절, ADR [R-BMS 확장 id](../acknowledge/2026-10-22-adr-rbms-extension-ids.md).

4. 외부 Lua 스킨 호환 유지 방식
   - A. 키 표와 id 의미는 원본 그대로, 확장은 별도 id 와 별도 키로만 더한다. 외부 스킨은 확장을 못 그려도 원본 기능은 그대로.
   - B. 설정 항목(SELECT 탭의 OPTION PANEL KEYS: REFERENCE / EXTENDED)으로 키 표를 바꾼다.
   - 추천: A. 이유: 설정 분기가 늘면 테스트와 문서가 둘이 된다. 확장 키가 외부 스킨 위에서 아무 표시 없이 설정을 바꾸는 점은 남는 위험이다(죽은 칸에 새 토글을 두는 경우). 외부 스킨 호환 이득 중 눈에 띄는 것은 CONSTANT(400)와 HAA(342) 응답 추가다(3.3절).
   - 영향: 규칙은 문서, 외부 스킨 캡처 확인.
   - 결정됨(2026-10-22, 메인이 정함, 사용자 미확인): A. 키 표의 의미는 원본 그대로 두고 확장은 별도 id 로만 더했다. 남는 위험으로 적은 "죽은 칸에 토글을 두면 외부 스킨 위에서 표시 없이 설정이 바뀐다" 가 그대로 남았고 수리하지 않았다(1.10절 14). 고치려면 스킨이 확장 id 를 안다는 신호가 필요하다. ADR [옵션 패널 배치](../acknowledge/2026-10-22-adr-option-panel-layout.md).

5. 패널 3 숫자형 칸(NOTE DISPLAY TIME, JUDGE TIMING)
   - A. 지금처럼 KEEP GREEN NUMBER 가 켜져 있을 때만 NOTE DISPLAY TIME 이 움직이고 꺼져 있으면 OFF 로 읽는다.
   - B. 꺼져 있으면 키 4·6 이 HI-SPEED 를 HI-SPEED STEP 만큼 움직이고 칸은 하이스피드와 그 이동 시간을 보인다.
   - C. 4·6 을 누르면 KEEP GREEN NUMBER 를 자동으로 켠다.
   - 추천: A. 이유: B·C 는 키 한 번이 다른 설정(속도 방식)을 바꿔 사용자 의도를 벗어난다. 화면에는 IIDX 처럼 값 상자와 양쪽 화살표, 녹색 숫자를 두면 된다(숫자 312·313·12 가 이미 있다).
   - 영향: 순수 Lua. B·C 는 `events.rs`·`panel.rs`.
   - 결정됨(2026-10-22, 메인이 정함, 사용자 미확인): A. 숫자 칸(값 상자 + 양쪽 화살표 + 녹색 숫자)이고 KEEP GREEN NUMBER 가 꺼져 있으면 `OFF` 로 읽힌다. 추가로 켜져 있는데 커서가 곡·완성 코스가 아니면 칸이 `-` 이고 키 4·6 도 움직이지 않는다(1.10절 6). ADR [옵션 패널 배치](../acknowledge/2026-10-22-adr-option-panel-layout.md).

6. DP OPTION 과 2P RANDOM
   - A. 숨긴다(패널 1 키 4와 이미지 3 의 DP 상자). 키 4는 다른 일에 쓴다.
   - B. 보이게 두되 "적용 안 됨"으로 표시한다.
   - C. 플레이에 실제로 구현한다(FLIP, BATTLE, `random_2p`).
   - 추천: A. 이유: 지금 값이 바뀌지만 플레이에 영향이 없어 사용자를 속인다(1.10절 1). C 는 별도 큰 작업(차트 변형)이다.
   - 영향: A 는 Lua 와 키 역할 하나. C 는 `rbms-chart`/`rbms-play`.
   - 결정됨(2026-10-22, 메인이 정함, 사용자 미확인): A. DP OPTION 을 패널 키에서 빼고 기본 스킨은 DP 상자를 그리지 않는다. 키 4 는 GAUGE 이전이다. 2P RANDOM 은 키 7 이 같은 `play.random` 을 만진다. C(플레이에 적용)는 하지 않았다. ADR [옵션 패널 배치](../acknowledge/2026-10-22-adr-option-panel-layout.md).

7. INFORMATION 줄과 도움말 패널의 범위
   - A. INFORMATION 줄만(순수 Lua, 바뀐 옵션 한 줄 설명).
   - B. A + 왼쪽 도움말(GAS 설명 4개, HARD 게이지 표, TARGET 목록).
   - C. B + 레인 PREVIEW(차트 의존, Rust 필요).
   - 추천: B. 이유: C 만 Rust 변경이 필요하고 가치가 낮다. TARGET 목록은 글자 id 200~219 가 이미 있다.
   - 영향: Lua 문구 수십 줄, 영어 문장 작성.
   - 결정됨(2026-10-22, 메인이 정함, 사용자 미확인): B. INFORMATION 줄과 왼쪽 도움말(TARGET 21줄, GROOVE GAUGE 도움, ASSIST LAMP 규칙, GAS 설명, JUDGE ALGORITHM 목록)을 그린다. C(레인 PREVIEW)는 하지 않았다. 문구는 메인이 영어 대문자로 새로 썼다(사용자 검토 대기). ADR [옵션 패널 배치](../acknowledge/2026-10-22-adr-option-panel-layout.md).

8. F1 오버레이의 운명
   - A. 유지하고 패널이 바꾸는 값과 같은 `Config` 를 쓴다는 것만 문서화.
   - B. 선곡에서 F1 오버레이를 새 패널로 대체해 제거.
   - 추천: A. 이유: 오버레이는 스킨 없는 선곡의 유일한 바로 바꾸기이고(`app_options.rs`), 제거는 범위 밖이다. 홀드 키 `LSHIFT` 와 턴테이블 충돌(1.10절 10)은 별도로 사용자에게 알린다.
   - 영향: 없음(A).
   - 결정됨(2026-10-22, 메인이 정함, 사용자 미확인): A(F1 오버레이 유지). 추천은 홀드 키 충돌을 사용자에게 알리는 것이었으나, START·SELECT·상세 키가 눌려 있으면 오버레이를 열지 않게 코드로 풀었다(`app_options.rs:204-226`, 1.10절 10). ADR [옵션 패널 배치](../acknowledge/2026-10-22-adr-option-panel-layout.md).

9. BGA 값
   - A. ON/OFF 둘로 줄여 표시한다(이름표 AUTOPLAY ONLY 삭제).
   - B. AUTOPLAY ONLY 를 구현한다(오토플레이 판에만 BGA).
   - 추천: A. 이유: 구현 이득이 작고 이름표만 거짓이다. 영향: Lua 한 줄(`sheet.lua:23`, `side.lua:24`).
   - 결정됨(2026-10-22, 메인이 정함, 사용자 미확인): A. 기본 스킨의 BGA 이름표를 ON/OFF 둘로 줄였다(`words.lua:4`, `side.lua`). AUTOPLAY ONLY 는 만들지 않았다. ADR [옵션 패널 배치](../acknowledge/2026-10-22-adr-option-panel-layout.md).

10. 이름과 순서 통일
    - 패널의 RANDOM 순서(원본 번호 순)와 설정 행의 순서(S-RANDOM 먼저), GAUGE 표기(EX HARD / EX-HARD / ASSISTED EASY) 통일 여부. 추천: 패널은 원본 번호 순을 유지하고, 표기는 스킨 쪽(`theme.lua`)이 이미 쓰는 철자를 기준으로 하되 IIDX 이미지의 `ASSISTED EASY` 는 쓰지 않는다. 영향: `settings.rs` 라벨은 건드리지 않는다.
    - 결정됨(2026-10-22, 메인이 정함, 사용자 미확인): 추천안 유지. 패널의 RANDOM 순서는 원본 번호 순, GAUGE 표기는 스킨 `theme.lua` 의 철자 그대로이고 `ASSISTED EASY` 는 쓰지 않는다. 코드 변경 없음(1.10절 7·8).

### 6.1 사용자 확인 대기 목록(미확인)

위 결정은 모두 사용자 지시 "끝까지 다 구현해놔" 에 따라 질문 없이 메인이 추천안으로 정했다. 사용자가 확인할 것:

1. 패널 키 배치(1.3절 표): 패널 1 키 4·6, 패널 2 키 3·5·6·7·턴테이블, 패널 3 턴테이블.
2. R-BMS 확장 id 재도입(3.4절): 2026-10-09 사설 id 철거 결정의 부분 변경.
3. 외부 스킨 위험(1.10절 14): 수리하지 않았다. 고칠지, 어떤 신호(헤더 플래그, 번들 팩 판정, 실행 중 읽기 기록)로 막을지.
4. F1 오버레이 홀드 키 변경(1.10절 10): 패널 키가 눌려 있으면 열리지 않는다.
5. 패널 문구(영어 대문자 INFORMATION, 도움말, 게이지·GAS·알고리즘 설명).
6. 굵은 글꼴 에셋 `Inter-Bold.ttf` 추가(`font-rendering.md` 14절).

---

## 7. 검증 기록

| 한 것 | 결과 |
| --- | --- |
| `target/release/rbms-cli skin-dump assets/skins/rbms-default/select.luaskin --frames 200`(scratch HOME) | 로드 6.5 ms, destination 503, text 246, value 34, image 40, imageset 2, slider 4, graph 9, 함수 61개, 프레임 59회 호출 평균 0.022 ms·p99 0.034 ms·최대 0.092 ms, 경고 0, 함수 실패 0, 예산 초과 0, 팩 파일 73개 불변. 바이너리는 2026-10-20 빌드이고 Lua 는 현재 파일을 읽는다 |
| `cargo test -p rbms-player --locked --lib the_browser_of_a_skin_pack_named_by_the_environment_is_captured_over_a_scanned_library -- --nocapture`(환경 변수 `RBMS_SKIN_PACK`, `RBMS_SKIN_CAPTURE_DIR`, `TMPDIR`, `HOME` 모두 scratch) | 통과(38.72 s). `select-panel0~3`, `select-panel3-keeping`, `select-panel1-stepped` 를 1920x1080 으로 저장해 보았고 1.8절 설명과 일치함을 확인했다. 저장소 파일은 바뀌지 않았다(`git status` 로 확인) |
| 이미지 3장 | 2.png·3.png·4.png 를 열고 4.png 와 3.png 는 `sips` 로 잘라 확대해 확인 |
| 코드 읽기 | `stage/select/{keys,panel,events,skinned,guide}.rs`, `skin_host/{mod,options,select/*}.rs`, `crates/rbms-skin/src/{lua/main_state.rs,property/*}`, `crates/rbms-render/src/skin_render/draw.rs`, `crates/rbms-config/src/{settings,schema,options,judge}.rs`, `apps/rbms-player/src/{app_options,keyconfig,app_input}.rs`, 기본 스킨 Lua 전체(선곡) |

### 7.1 구현 뒤 검증 기록 (2026-10-22)

작업자 보고(세션 scratch, 저장소 밖)에서 옮긴 실행 결과다. 마지막 줄만 통합 게이트의 결과다.

| 한 것 | 결과 |
| --- | --- |
| `7746932`: `cargo test -p rbms-player --locked --lib`, `-p rbms-skin --locked`, `-p rbms-ir --locked` | 2,642 통과 0 실패 2 ignored. rbms-skin 545 통과(원본 id 표와 `property_checksum` 불변). rbms-ir 275 통과 1 ignored |
| 같은 작업: clippy 전체, fmt, `rbms-cli skin-dump assets/skins/rbms-default --strict --frames 120` | 경고·오류 0, 통과, 종료 코드 0(문서 9개 load ok, 경고 0) |
| 같은 작업: IR 패널 폭 임시 측정(측정 뒤 삭제) | `ASSIST EXHARD` 86.8 px + 꼬리 76.2 px < 가용 228 px |
| `0c450db`: skin-dump strict, `default_pack` 테스트, 캡처 | 문서 9개 경고 0·함수 실패 0·예산 초과 0, `default_pack` 4 통과, `the_default_packs_option_panels_and_graded_lamps_are_captured` 가 15장 저장(1920x1080 PNG 를 눈으로 확인). lib 2,656 통과 |
| `3f058f4`: lib | 2,666 통과 0 실패 2 ignored. 빨간 확인: `forget_turntable_changes()` 호출과 `run_duration_event` 의 차트 게이트를 빼면 새 테스트 2개가 실패하고 복원 뒤 통과 |
| `3f058f4`: 캡처 | 7키, 5키, 폴더 위 커서(KEEP GREEN 켬) 세 경우를 눈으로 확인. 5키는 임시 환경 스위치로만 찍고 되돌렸다(영구 5키 케이스는 없다) |
| 마지막 통합 게이트(메인) | Rust 6,269 통과·0 실패, 웹 1,491 통과, DB 통합 122 통과 |

실기(실제 GPU 창)에서 패널을 본 적은 없다. 외부 스킨(ModernChic)에서 패널 키를 눌러 본 적도 없다.

읽기만 하고 실행으로 확인하지 못한 것: F1 오버레이와 턴테이블 `LSHIFT` 의 충돌 동작(1.10절 10), 폴더 위에서 HI-SPEED FIX 가 `-` 를 보이면서 움직이는 것(1.10절 5), 5키·10키에서 6·7번 건반이 없는 것(1.3절). 이 셋은 실제 창 확인이 필요하다.
