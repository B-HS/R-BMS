# ADR: 선곡 옵션 패널의 키 배치, 토글 추가, IIDX 식 기본 스킨 패널

> 최종 갱신 2026-10-22 · 대응 커밋 47a7f9c
> 상태: 유효(사용자 확인 2026-10-23: "1-11 추천대로") · 결정일 2026-10-22 · 결정자 메인(사용자 지시 "끝까지 다 구현해놔"에 따라 추천안 채택, 사용자 미확인) · 커밋 7746932(키 표(7절의 외부 스킨 키 재배정은 2026-10-23 결정 5 로 바뀐다: 스킨 헤더의 확장 지원 표시가 있을 때만. 새 ADR 은 구현 뒤에 쓴다) · 커밋 7746932(키 표·이벤트·F1 홀드 키), 0c450db(기본 스킨 패널), 3f058f4(패널 수리)
> 관계: 원본 id 만 쓰던 2026-10-09 결정을 일부 바꾸는 확장 id 는 [확장 id ADR](2026-10-22-adr-rbms-extension-ids.md). 패널이 보여 주는 램프 등급은 [어시스트 등급 ADR](2026-10-22-adr-assist-lamp-grade.md).

## 배경

- 사용자 지시(2026-10-22, 요지. 원문이 아니라 `docs/PROCESS.md` 의 기록): (1) SCRATCH AUTO 등 플레이 옵션 토글을 A(START)·A+W 패널에서 바로 바꾸게 한다. (2) A·W·A+W 패널을 IIDX 옵션 화면(키 그림, 연결선, 값 전체를 세로로 나열, 고른 값은 주황 강조)처럼 만든다. 참고 이미지는 IIDX 옵션 화면 3장이다.
- 패널은 키를 누르는 동안만 올라온다(원본과 같다). START 단독이 패널 1(기본 키 `A`), SELECT 단독이 패널 2(`W`), 둘을 함께 누르거나 상세 키 `5` 를 누르면 패널 3 이다. 칸은 화면 요소가 아니라 건반 인덱스(0~8)에 묶인 역할이고, 키 표는 원본 `MusicSelectKeyProperty` 에서 `NEXT_REPLAY` 만 뺀 것이었다([option-panels.md](../reference/option-panels.md) 1.1~1.3절).
- 죽은 칸이 있었다. 패널 2 는 7칸 가운데 CONSTANT 와 LEGACY NOTE 만 동작했고 JUDGE AREA·MARK NOTE·BPM GUIDE·NO MINE 은 아무것도 하지 않았다(EX JUDGE 는 읽기 전용). 패널 1 의 DP OPTION(건반 4)은 값이 저장되지만 플레이가 읽지 않았다. SCRATCH AUTO 와 KEEP GREEN NUMBER 는 패널 키도, 스킨이 읽을 id 도 없었다.
- F1 오버레이의 홀드 키(출하 LSHIFT)가 7키의 턴테이블과 같은 키라서, 선곡에서 턴테이블을 만지면 오버레이가 열렸다.

## 결정

### 1) 키 배치: 동작하는 칸은 두고 죽은 칸과 비어 있던 칸에만 새 토글을 배정한다

조사 문서 6절 결정 1 의 A 안이다. 건반 번호는 인덱스 + 1 이고 턴테이블은 인덱스 7(정방향)·8(역방향)이다. 9키 표(`POPN_TABLE`)도 같은 인덱스에 같은 패널 역할을 둔다. 아래 열 개 칸 말고는 원본 표와 같다는 것을 `apps/rbms-player/src/stage/select/keys.rs` 의 테스트(`CHANGED_SLOTS`)가 단언한다.

| 패널 | 칸 | 원본 역할 | 지금 |
| --- | --- | --- | --- |
| 1 START | 건반 4 | OPTIONDP_DOWN(DP OPTION 다음) | GAUGE 이전(`GaugeUp`) |
| 1 START | 건반 6 | OPTION2_UP(2P RANDOM 이전) | HI-SPEED FIX 이전(`HsFixUp`) |
| 2 SELECT | 건반 3 | JUDGEAREA | SCRATCH AUTO 켬끔 |
| 2 SELECT | 건반 5 | MARKNOTE | KEEP GREEN NUMBER 켬끔 |
| 2 SELECT | 건반 6 | BPMGUIDE | LANE COVER 켬끔 |
| 2 SELECT | 건반 7 | NOMINE | LIFT 켬끔 |
| 2 SELECT | 턴테이블 정방향 | 없음 | HIDDEN+ 켬끔 |
| 2 SELECT | 턴테이블 역방향 | 없음 | HIDDEN+ 켬끔 |
| 3 START+SELECT | 턴테이블 정방향 | 없음 | JUDGE ALGORITHM 이전 |
| 3 START+SELECT | 턴테이블 역방향 | 없음 | JUDGE ALGORITHM 다음 |

그대로인 칸: 패널 1 건반 1 RANDOM 다음, 2 RANDOM 이전, 3 GAUGE 다음, 5 HI-SPEED FIX 다음, 7 RANDOM 다음, 턴테이블·휠·방향키 TARGET. 패널 2 건반 1 EX JUDGE(읽기 전용), 2 CONSTANT, 4 LEGACY NOTE. 패널 3 건반 1 BGA, 2 GAUGE AUTO SHIFT, 3 AUTO ADJUST, 4·6 NOTE DISPLAY TIME 줄임·늘림, 5·7 JUDGE TIMING -·+.

- 제거된 `SelectKey` 변종: `OptionDpUp`·`OptionDpDown`·`Option2Up`·`JudgeArea`·`MarkNote`·`BpmGuide`·`NoMine`. 새 토글은 기존 설정 이벤트 경로(`PanelAct::Setting` → `run_setting_event`)로 실행되어 OptionChange 소리, 저장 표시, 패널을 닫을 때 저장이 기존과 같다.
- LANE COVER·LIFT·HIDDEN+·JUDGE ALGORITHM 은 못 치는 코스 위에서는 기존 규칙대로 움직이지 않는다. SCRATCH AUTO 와 KEEP GREEN NUMBER 는 전역 설정이라 막지 않는다.
- 패널 1 건반 6 과 7: `OPTION2_UP`·`OPTION2_DOWN` 은 1P 와 같은 값 `play.random` 하나를 움직인다(`stage/select/events.rs:83-85` `SettingEvent::Random` 문서: 두 면을 한 옵션으로 나눈다). 2P 만의 값이 없으므로 건반 6 은 건반 2 와 완전히 중복이다. 그래서 건반 6 을 HI-SPEED FIX 이전으로 바꿨다. 건반 7 은 HI-SPEED FIX 이전이 6 에 생겨 남은 고유 기능이 없지만, 실제로 동작하고(플레이에 반영) 원본·ModernChic 의 건반 7 라벨(RANDOM 2P)과 어긋나지 않아 원본 역할(RANDOM 다음)을 그대로 뒀다.

### 2) 숨기거나 단순하게 둔 것

- DP OPTION: 패널 키에서 뺐다(건반 4 는 GAUGE 이전). 값 `play.lane_option`, 설정 행, 스킨 이벤트 54·이미지 인덱스 54 의 응답은 그대로 남는다. 기본 스킨 패널은 DP OPTION 상자를 그리지 않는다. 지금 값이 바뀌어도 플레이에 영향이 없어 사용자를 속이기 때문이다.
- BGA: 이미 `display.bga` 불린이라 ON/OFF 두 값만 돈다. 이미지 인덱스 72 는 0(ON) 또는 2(OFF)만 답하고 AUTOPLAY ONLY(1)는 나오지 않는다. 기본 스킨의 BGA 이름도 ON/OFF 다(`lua/select/side.lua:24`).
- NOTE DISPLAY TIME: KEEP GREEN NUMBER 가 켜져 있을 때만 움직이고 꺼져 있으면 OFF 로 읽는다(키 한 번이 다른 설정을 바꾸지 않게). 커서가 차트가 아닌 막대(폴더·표·실행 파일, 못 치는 코스)에 있으면 키 4·6 도 값을 바꾸지 않고(`events.rs:396` `run_duration_event` 가 칠 수 있는 막대일 때만 움직인다), 패널은 `-` 와 "커서를 차트로 옮기라"는 안내를 보인다. KEEP GREEN NUMBER 꺼짐 경고와 섞이지 않는다.
- 이번에 넣지 않은 토글: LN MODE, BOTTOM SHIFTABLE, GUIDE SE(자리 부족), AUTOPLAY, SCRATCH SIDE, TARGET FALLBACK·RATE(AUTOPLAY 는 출하 ON 이고 켠 채 저장되면 판이 기록에서 빠진다). JUDGE WIDTH 와 TOTAL 은 어시스트 판을 만들어 패널에 맞지 않는다.

### 3) 턴테이블 역할은 패널을 연 뒤의 눌림만 센다

원본은 패널 위의 턴테이블을 "누르고 있음"으로만 읽는다(TARGET). 새 역할 HIDDEN+·JUDGE ALGORITHM 은 "눌림"으로 읽는다. 패널이 열릴 때 이미 돌고 있던 방향이 새 눌림으로 세어져 열자마자 HIDDEN+ 나 판정 알고리즘이 바뀌던 결함이 있어, 패널이 열리거나 다른 패널로 바뀔 때(`panel.rs` `show_panel`) `SelectKeys::forget_turntable_changes` 가 턴테이블 두 인덱스의 변화 표시를 지운다. 패널이 뜬 뒤에 돌린 것은 센다. TARGET 은 누르고 있음 읽기 그대로다. 테스트 `a_turntable_direction_held_before_the_panel_came_up_is_not_a_press_on_it`.

### 4) F1 오버레이 홀드 키

홀드 키(`Shortcut::OptionsHold`, 출하 `LSHIFT`)가 눌렸을 때 START·SELECT·상세 키(`Shortcut::RefDetail`) 중 하나라도 눌려 있으면 오버레이를 열지 않고 키를 선곡으로 보낸다(`apps/rbms-player/src/app_options.rs` `open_key`, `an_option_panel_is_called_up`). 그래서 패널 1 의 턴테이블 TARGET, 패널 2 의 HIDDEN+, 패널 3 의 JUDGE ALGORITHM 이 동작한다. 핀 키 F1 은 그대로이고, 패널이 없으면 오버레이는 기존처럼 열린다. 부작용: 스킨이 없는 내장 선곡에서도 START 를 누른 채 LSHIFT 를 누르면 오버레이가 열리지 않는다(영향 작음). 오버레이 자체는 스킨 없는 선곡의 유일한 바로 바꾸기라 제거하지 않았다.

### 5) 5키·10키

`keyconfig.rs` 의 `key_index_of` 는 한쪽 6레인(건반 5 + 스크래치)을 위치 0~4 와 7·8 로만 접는다. 인덱스 5·6(건반 6·7)에 닿는 레인이 키보드와 컨트롤러 모두에 없다. 그 모드에서 닿지 않는 역할은 패널 1 의 HI-SPEED FIX 이전(건반 6)·RANDOM 다음(7), 패널 2 의 LANE COVER(6)·LIFT(7), 패널 3 의 NOTE DISPLAY TIME 늘림(6)·JUDGE TIMING +(7)이다. 키 표 동작은 바꾸지 않았다(`keys.rs` 모듈 문서에 한 줄).

- 호스트가 불린 옵션 21140~21148 로 "이 모드에서 키 인덱스 i 에 닿는 레인이 없다"를 답하고(`skin_host/select.rs`, `select/shown.rs` `unreachable_keys_of(mode)`), 기본 스킨이 건반 6·7 과 그 열 상자(SELECT 패널의 LANE COVER·LIFT)를 알파 196 으로 흐리게 그린다. 7키는 변화가 없다. 외부 스킨은 이 id 를 읽지 않아 영향이 없다.
- JUDGE TIMING 은 -200~200 에서 감기지 않고 멈추므로 5키에서는 패널로 줄이기만 되고 올리기는 설정 화면에서 해야 한다.
- 키 가이드(H)의 패널 줄에 세 줄을 더했다: `{START} + TURNTABLE = TARGET`, `{SELECT} + TURNTABLE = HIDDEN+ ON/OFF`, `{START}+{SELECT} + TURNTABLE = JUDGE ALGORITHM`(`stage/select/guide.rs:233-235`).

### 6) 기본 스킨 패널 (IIDX 식)

- 문서 좌표 1920x1080. 왼쪽 열 x64~472, 본 영역 x504~1856. 위쪽에 INFORMATION, 그 아래 건반 7개(흰 1·3·5·7 은 아래 줄, 검은 2·4·6 은 위 줄), 건반 왼쪽에 턴테이블 상자, 상자는 y492 부터다. 연결선은 열 색의 4px 꺾인 선이고 서로 엇갈리지 않는다. 상자는 머리(굵은 이름) + 값 전체를 세로로 나열(지금 값은 밝은 주황 바탕에 어두운 굵은 글자와 빛 테두리) + 아래 굵은 열 이름이다.
- 패널 1: RANDOM(건반 1·2·7), GAUGE(3·4), HI-SPEED FIX(5·6), 왼쪽 TARGET 21줄, 왼쪽 아래 GROOVE GAUGE 도움. 패널 2: 7개 ON/OFF 상자(EX JUDGE 는 회색 LOCKED·READ ONLY), SCRATCH AUTO 아래 `ON: THE LAMP IS AN ASSIST GRADE`, 왼쪽 HIDDEN+ 상자, 왼쪽 아래 ASSIST LAMP 규칙(게이지가 어느 ASSIST 등급이 되는지). 패널 3: BGA, GAS(5값), AUTO ADJUST, JUDGE TIMING 숫자 칸, NOTE DISPLAY TIME 숫자 칸, 왼쪽 JUDGE ALGORITHM(SCORE 면 강조 없이 `SCORE: SET IN SETTINGS > JUDGE`), 왼쪽 아래 GAS 설명.
- INFORMATION 줄은 패널이 열린 동안만 값을 읽는다. 열림 타이머(21~23) 값이 바뀌면 기준값을 새로 잡고, 이후 값이 바뀌면 왼쪽에 옵션 이름, 오른쪽에 새 값, 아래에 한 줄 설명을 보인다. 움직임은 기존 관례(rise 24px·150ms, 도착 타이머 21~23, 출발 31~33)를 그대로 쓴다.
- 값은 원본 id 로 읽는다(`event_index` 40·42·55·72·75·78·301·302·304·340, `number` 12·312·313, `text` 3·200~219). 원본 id 가 없는 SCRATCH AUTO·KEEP GREEN NUMBER·LANE COVER·LIFT·HIDDEN+ 는 확장 불린 21100~21104 를 `draw` 함수 안에서 읽는다.
- 글자는 굵은 글꼴(Inter Bold)을 옵션 이름·강조 값·INFORMATION 제목과 값·태그·건반 표지에 쓰고, 값 글자는 24 이상, 보조 글자는 18 이상(문서 단위)이다. 1280x720 창에서는 2/3 배로 줄어든다. 굵은 글꼴 결정은 [글자 ADR](2026-10-22-adr-ui-text-device-pixels.md).
- 구현: `sheet.lua` 는 조립부, `panel.lua` 는 그리기 도구, `words.lua` 는 문구, `inform.lua` 는 INFORMATION 감시다. 화살표는 그림이 아니라 굵은 `<`·`>` 글자이고 턴테이블은 사각 상자다.

### 7) 외부 스킨의 키 재배정 위험은 남긴다 (통합 리뷰 panels-1)

키 표 재배정은 모든 스킨에 적용된다. ModernChic 같은 beatoraja 스킨이 죽은 칸 라벨(JUDGE AREA·MARK NOTE·BPM GUIDE·NO MINE)로 그린 키가 이제 저장되는 설정을 바꾼다. 화면에는 아무 변화가 없고 OptionChange 소리만 난다.

- SELECT + 건반 3 은 SCRATCH AUTO, + 건반 5 는 KEEP GREEN NUMBER, + 건반 6 은 LANE COVER, + 건반 7 은 LIFT, + 턴테이블은 HIDDEN+ 를 바꾼다. START + 건반 4 는 GAUGE 를 거꾸로, + 건반 6 은 HI-SPEED FIX 를 바꾼다(DP OPTION 이미지 54 는 더 이상 바뀌지 않는다). START + SELECT + 턴테이블은 JUDGE ALGORITHM 을 바꾼다. 패널을 닫을 때 저장되므로 다음 판이 SCRATCH AUTO 로 시작해 어시스트 램프가 될 수 있다.
- 고치지 않은 이유: 스킨이 확장 id 를 안다는 것을 알려 주는 통로가 없다. 로더는 `op` 목록에서 확장 id 를 버리고, Lua 는 실행 중에 `main_state.option`·`event_exec` 로 읽고, 호스트는 스킨이 무엇을 물었는지 기억하지 않으며(`RequestQueue` 는 이벤트와 쓰기만 담는다), `ScreenHost` 는 프레임마다 다시 만들어진다.
- 검토한 방법 셋: (a) 헤더나 맞춤 행에 확장 지원 표시를 두고 로더가 읽어 `KeyLayout` 이 따른다(`crates/rbms-skin` 로더 변경이 필요). (b) 번들 기본 팩 경로일 때만 확장 키를 켠다(사용자가 고친 사본, `RBMS_SKIN_PACK`, 테스트가 쓰는 저장소 경로 팩에서 꺼진다). (c) 실행 중에 확장 id 읽기를 기록한다(프레임마다 다시 만드는 `ScreenHost` 와 Options·Select 클러스터에 셀을 통과시켜야 하고 첫 프레임은 알 수 없다). 바뀐 옵션을 알리는 토스트는 요구된 "아무 일도 안 함"이 아니라 추가하지 않았다.
- 사용자 확인이 필요하다(미확인): 지금대로 둘지, (a)~(c) 중 하나로 외부 스킨에서는 원본 표를 지킬지.

## 이유

- 원본 키 표의 배열(흰 키 1·3·5·7, 검은 키 2·4·6)이 IIDX 참고 이미지의 키 그림과 한 칸도 어긋남 없이 같다(조사 4.5절). 그림을 IIDX 처럼 바꾸는 데 표를 바꿀 필요가 없으므로 죽은 칸만 재배정해 ModernChic 같은 외부 스킨의 고정 라벨과 동작하는 칸이 어긋나지 않게 했다.
- 패널 2(SELECT)는 IIDX 의 ASSIST 와 뜻이 같아 SCRATCH AUTO 를 두기에 맞다. 패널 3 의 턴테이블은 패널 1 의 TARGET 과 같은 방향 관례(정방향 = 이전, 역방향 = 다음)를 따랐고, IIDX 참고 이미지에서 ALGORITHM 상자가 키와 이어지지 않은 점에 맞췄다.
- 원본 id 가 이미 있는 것(LANE COVER·LIFT·HIDDEN+·JUDGE ALGORITHM, 이벤트 330~332·340)은 Rust 가 키 역할만 더하면 된다. id 가 없는 둘만 확장 id 가 필요했다.
- Lua 만으로 값 목록 전체, 강조, INFORMATION 줄, 숫자 칸, 연결선을 그릴 수 있어(조사 5.1절) 패널 그림은 Rust 변경이 없다.

## 기각된 대안

조사 문서 6절의 결정 지점이다.

| 대안 | 기각 이유 |
| --- | --- |
| IIDX 이미지 2 처럼 열별 키 두 개를 쓰는 R-BMS 전용 표(결정 1 B) | 표와 테스트 전면 변경, 외부 스킨 호환이 깨진다 |
| 원본 표를 그대로 두고 토글을 네 번째 패널(새 조합)에 모음(결정 1 C) | 여는 입력과 `Panel` 변종이 늘고, 키 그림이 원본 표와 같다는 이점이 사라진다 |
| 설정 항목(OPTION PANEL KEYS: REFERENCE / EXTENDED)으로 표를 바꿈(결정 4 B) | 설정 분기가 늘면 테스트와 문서가 둘이 된다 |
| NOTE DISPLAY TIME 이 꺼져 있을 때 키 4·6 이 HI-SPEED 를 움직임(결정 5 B)이나 KEEP GREEN NUMBER 를 자동으로 켬(C) | 키 한 번이 다른 설정을 바꿔 사용자 의도를 벗어난다 |
| DP OPTION 을 "적용 안 됨"으로 표시(결정 6 B), 플레이에 구현(C) | 구현(FLIP, BATTLE, `random_2p`)은 차트 변형이 필요한 별도 큰 작업이다 |
| BGA AUTOPLAY ONLY 구현(결정 9 B) | 구현 이득이 작고 이름표만 거짓이었다 |
| 패널 1 건반 7 을 비움 | 실제로 동작하는 역할이고 원본 라벨과 어긋나지 않아 변경을 최소로 했다(메인 판단) |
| F1 오버레이를 새 패널로 대체해 제거(결정 8 B) | 오버레이는 스킨 없는 선곡의 유일한 바로 바꾸기이고 범위 밖이다 |
| 5키·10키에서 닿지 않는 역할에 다른 키를 줌 | 하지 않았다. 대신 닿지 않는 칸을 흐리게 그려 알린다 |

## 영향과 관련 파일

- 선곡 입력·패널: `apps/rbms-player/src/stage/select/{keys,panel,events,guide}.rs`, `app_options.rs`, 호스트 `skin_host/{extension,select,options}.rs`, `skin_host/select/{shown,bridge}.rs`.
- 기본 스킨: `assets/skins/rbms-default/lua/select/{sheet,panel,inform,words,info,side}.lua`, `lua/{props,theme,ui,parts}.lua`. 캡처 보조 `apps/rbms-player/src/stage/capture/panels.rs`(`RBMS_SKIN_CAPTURE_DIR` 가 있을 때만).
- 테스트: `keys.rs` 의 키 표 단언, `panel_tests.rs` 의 `a_turntable_direction_held_before_the_panel_came_up_is_not_a_press_on_it`·`the_third_panel_leaves_the_travel_time_alone_while_the_cursor_is_off_a_chart`, `skin_host/select/tests.rs`, `skin-dump --strict` 게이트(경고 0).
- 비용: 기본 스킨 select 문서가 destination 503 → 1,611, 함수 61 → 171 로 늘었다(패널 구현 시점 측정). release `rbms-cli skin-dump` 200프레임에서 로드 7.0 → 24.8 ms, 프레임 평균 0.024 → 0.047 ms, p99 0.043 → 0.060 ms, 예산 초과 0 이다. 5키·10키 흐림까지 넣은 뒤의 debug 덤프는 destination 1,677, 함수 203, 경고 0 이다. 패널이 닫혀 있어도 출발 destination 의 `draw` 함수는 매 프레임 불린다.
- 남은 위험(미확인 포함):
  - 외부 스킨의 키 재배정(결정 7).
  - INFORMATION 문구와 게이지·GAS·알고리즘 설명은 영어 대문자로 새로 썼다. 사용자의 문구 검토가 필요하다.
  - 폴더 위에서 HI-SPEED FIX 는 값이 `-` 로 읽히는데 `run_setting_event` 는 `has_play_settings()` 만 보고 값을 바꾸는 것으로 보인다(수리 작업자의 추정, 확인하지 않았다). NOTE DISPLAY TIME 과 달리 고치지 않았다.
  - 한 프레임에 두 옵션이 동시에 바뀌면 INFORMATION 은 감시 순서상 마지막 것(GAUGE)만 보인다.
  - 실제 창에서 패널·F1 충돌을 보지 않았다. 단위 테스트(`options_key` 직접 호출)와 헤드리스 1920x1080 캡처로만 확인했다.
  - 5키·10키에서 건반 6·7 에서 열로 가는 연결선과 JUDGE TIMING·NOTE DISPLAY TIME 의 `>` 화살표는 그 열이 다른 키로 닿으므로 흐리게 하지 않았다.

## 날짜와 사용자 원문

2026-10-22.

- 지시 요지(`docs/PROCESS.md` 의 기록, 원문 아님): "(1) SCRATCH AUTO 등 플레이 옵션 토글을 A(START)·A+W 패널에서 바로 바꾸게, (2) A·W·A+W 패널을 IIDX 옵션 화면(키 그림 + 연결선 + 값 전체를 세로로 나열, 고른 값 주황 강조) 처럼".
- 설계 확정 지시: "끝까지 다 구현해놔"
