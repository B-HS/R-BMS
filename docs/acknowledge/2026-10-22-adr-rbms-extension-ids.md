# ADR: R-BMS 확장 id 대역 (이벤트 21000~21099, 불린 21100~21199)

> 최종 갱신 2026-10-22 · 대응 커밋 47a7f9c
> 상태: 일부 대체됨("21002~21099 미사용", "`op` 에는 쓰지 못한다", 정수·Float·Rate·텍스트·참조 이미지에 확장 id 가 없다는 전제 → [확장 id 공간 ADR](2026-10-24-adr-rbms-extension-id-spaces.md)). 나머지는 유효(사용자 확인 2026-10-23: "1-11 추천대로") · 결정일 2026-10-22 · 결정자 메인(사용자 지시 "끝까지 다 구현해놔"에 따라 추천안 채택, 사용자 미확인) · 커밋 7746932(대역과 첫 id), 3f058f4(닿지 않는 키 id)
> 대체하는 결정: [2026-10-09-lua-skin-compat-decisions.md](2026-10-09-lua-skin-compat-decisions.md) 의 "사설 id 철거"(이전 결정과의 관계 절)와 그 뒤 "호스트는 원본이 선언한 id 만 답한다"는 운영 규칙을 일부 바꾼다. 폐기한 옛 사설 id 20001~20316 은 되살리지 않는다.

## 배경

- 2026-09-17 결정(D10·D15)은 옵션 패널과 결과 글자를 `select`·`result` 문서의 대체 단위로 두고 사설 id 20001~20316 을 썼다. 2026-10-09 결정이 혼합 합성과 함께 이를 삭제했다(철거 작업 W1-1). 그 뒤 호스트는 원본(beatoraja)이 선언한 id 만 답했다. 원본 id 표는 `tools/gen-skin-property.rs` 가 `SkinProperty.java` 에서 뽑아 커밋한 파일이고 체크섬 테스트(`crates/rbms-skin/tests/property_checksum.rs`)가 손 수정을 막는다.
- 2026-10-22 요구(IIDX 식 패널, SCRATCH AUTO 등 토글, 어시스트 등급 표시)에는 원본에 id 가 없는 값이 있었다: SCRATCH AUTO, KEEP GREEN NUMBER 의 켬끔 값과 그 토글 이벤트, 어시스트 등급, 5키·10키에서 닿지 않는 키. 기본 스킨이 이를 읽을 길이 없었다([option-panels.md](../reference/option-panels.md) 2.2절, 3.4절).
- 조사 문서 6절 결정 3 은 "2026-10-09 결정이 사설 id 를 폐기했으므로 사용자가 다시 확인해야 한다"고 적었다. 사용자는 질문 없이 끝까지 구현하라고 지시했고 메인이 추천안으로 정해 이 ADR 로 남긴다. `docs/PROCESS.md` E1 이 같은 내용을 "2026-10-09 사설 id 철거 결정의 일부 변경"으로 적었다.

## 결정

### 1) 두 대역을 R-BMS 가 답한다

- 이벤트 21000~21099, 불린(옵션) 21100~21199. 단일 목록은 `apps/rbms-player/src/skin_host/extension.rs` 다.
- 위치 규칙: 원본 선언 최대(옵션 3035, 이벤트 388)와 원본이 스킨 몫으로 남긴 대역(옵션 900~999, 이벤트 1000~1999, 타이머 10000~19999)보다 위이고, 폐기한 옛 사설 id(20001~20316)와도 겹치지 않는다. 옵션과 이벤트는 id 공간이 따로다.
- 대역은 숫자 범위일 뿐이다. 대역 안에 있고 아래 표에 없는 번호는 옵션으로 읽히지 않고(꺼짐) 이벤트로 실행되지 않는다.
- `reference_implements`·생성된 원본 id 표·체크섬 테스트는 건드리지 않았다. 확장은 호스트의 `ROUTES`·`EVENT_ROUTES`(`skin_host/mod.rs`)에 따로 등록한다. `reference_implements` 를 고치면 "원본이 구현하는 id 수" 테스트가 거짓이 된다.

### 2) 이벤트 id 표

어떤 인자로 실행해도 한 번 실행에 한 번 켬끔을 뒤집는다. `Cluster::Options` 로 가고 선곡의 `EVENTS` 표(`stage/select/events.rs:285-286`)에 등록돼 있다. 소리는 OptionChange 이고 패널을 닫거나 선곡을 떠날 때 저장된다.

| id | 상수 | 뜻 |
| --- | --- | --- |
| 21000 | `RBMS_EVENT_SCRATCH_AUTO` | SCRATCH AUTO 켬끔(`play.scratch_auto`) |
| 21001 | `RBMS_EVENT_KEEP_GREEN_NUMBER` | KEEP GREEN NUMBER 켬끔(`play.keep_green_number`) |
| 21002~21099 | - | 미사용(대체됨: [확장 id 공간 ADR](2026-10-24-adr-rbms-extension-id-spaces.md)) |

### 3) 불린(옵션) id 표

| id | 상수 | 답하는 화면(클러스터) | 뜻 |
| --- | --- | --- | --- |
| 21100 | `RBMS_OPTION_SCRATCH_AUTO_ON` | 선곡(Options) | SCRATCH AUTO 켜짐 |
| 21101 | `RBMS_OPTION_KEEP_GREEN_NUMBER_ON` | 선곡(Options) | KEEP GREEN NUMBER 켜짐 |
| 21102 | `RBMS_OPTION_LANE_COVER_ON` | 선곡(Options) | LANE COVER 켜짐(= 이미지 인덱스 330 이 1) |
| 21103 | `RBMS_OPTION_LIFT_ON` | 선곡(Options) | LIFT 켜짐(= 331 이 1) |
| 21104 | `RBMS_OPTION_HIDDEN_ON` | 선곡(Options) | HIDDEN+ 켜짐(= 332 가 1) |
| 21105~21109 | - | - | 미사용 |
| 21110 + k (k = 0~4) | `RBMS_OPTION_SELECTED_BEST_ASSIST_GRADE_FIRST` | 선곡(Select) | 커서 아래 곡의 최고 램프의 어시스트 등급이 k. 최고 기록이 없는 곡·폴더·코스는 다섯 모두 꺼짐 |
| 21115~21119 | - | - | 미사용 |
| 21120 + k | `RBMS_OPTION_RESULT_ASSIST_GRADE_FIRST` | 결과(Result) | 이번 판의 등급이 k |
| 21125~21129 | - | - | 미사용 |
| 21130 + k | `RBMS_OPTION_RESULT_PREVIOUS_ASSIST_GRADE_FIRST` | 결과(Result) | 이번 판 이전의 최고 기록(이미지 인덱스 371 과 같은 LN 모드 기준)의 등급이 k. 처음 친 곡은 다섯 모두 꺼짐 |
| 21135~21139 | - | - | 미사용 |
| 21140 + i (i = 0~8) | `RBMS_OPTION_UNREACHABLE_KEY_FIRST` | 선곡(Select) | 키가 묶인 모드에서 키 인덱스 i 에 닿는 레인이 없음. 5키·10키의 인덱스 5·6(건반 6·7)이 켜진다 |
| 21149~21199 | - | - | 미사용 |

- 등급 k 는 `rbms_judge::AssistGrade::id` 다: 0 등급 없음, 1 ASSIST EASY, 2 ASSIST CLEAR, 3 ASSIST HARD, 4 ASSIST EXHARD([어시스트 등급 ADR](2026-10-22-adr-assist-lamp-grade.md)). 등급 0 은 LIGHT ASSIST 램프의 미등급일 때와 다른 모든 램프일 때 켜지므로 램프 id 와 함께 읽는다(선곡 이미지 인덱스 370, 결과 370·371).
- 키 인덱스는 건반 1~7 이 0~6, 턴테이블 정방향·역방향이 7·8 이다([옵션 패널 ADR](2026-10-22-adr-option-panel-layout.md)).
- 켬끔 값 21100~21104 는 원본 id 가 있는 330~332 와 같은 값을 불린으로도 답한다. 패널의 모든 켬끔 값을 한 방식으로 읽게 하려는 것이다. 읽기와 실행은 원본 id(이벤트 330~332, 이미지 인덱스 330~332)로 해도 같다.
- 원본 id 가 없는 값은 SCRATCH AUTO 와 KEEP GREEN NUMBER 둘뿐이다. 나머지 패널 값(RANDOM, GAUGE, HI-SPEED FIX, TARGET, CONSTANT, LEGACY NOTE, BGA, GAS, AUTO ADJUST, JUDGE ALGORITHM, NOTE DISPLAY TIME, JUDGE TIMING)은 원본 id 로 읽는다.

### 4) 스킨이 읽고 실행하는 방법

- 불린은 `main_state.option(id)` 로 함수 필드(`draw`, `value`, `text`)에서만 읽는다. 로더는 `op = {id}` 목록을 원본이 선언한 id 로 제한해 확장 id 를 버리므로(`crates/rbms-skin/src/loader/track.rs:156-158` `is_builtin_option`) `op` 에는 쓰지 못한다. `main_state.option` 은 원본 선언 확인 없이 호스트에 묻는다(`crates/rbms-skin/src/lua/main_state.rs:851-853`). 음수 id 는 부정으로 읽고, 이 불린에 답하지 않는 화면에서는 꺼짐이다. (대체됨: `op` 목록에 확장 불린을 직접 쓰는 길은 2026-10-24 로더 수리로 열렸다. [확장 id 공간 ADR](2026-10-24-adr-rbms-extension-id-spaces.md))
- 이벤트는 `main_state.event_exec(id)` 나 버튼의 `act` 로 실행한다. 이벤트 공간은 원본 선언에 대한 게이트가 없어(`reference_implements(NameSpace::Event, _)` 가 항상 참, `crates/rbms-skin/src/property/names.rs:363-366`) 라우트 표에 줄을 더하면 된다.
- 기본 스킨은 `assets/skins/rbms-default/lua/props.lua` 의 `props.extension` 표에 불린 id 의 이름을 둔다. 기본 스킨은 이벤트 21000·21001 을 쓰지 않는다. 패널 키는 Rust 키 표의 경로로 같은 설정을 바꾼다.
- 새 id 를 더할 때는 `extension.rs` 에 상수와 대역 확인 테스트 목록(`every_id_is_in_its_band_and_no_two_are_the_same`)을 고치고 `skin_host/mod.rs` 의 라우트를 등록한다.

## 이유

- 원본이 선언했지만 R-BMS 에 대응 기능이 없는 id(303, 305, 306, 307, 350 등)를 재사용하면, 외부 스킨이 그 id 의 라벨을 이미 굽고 있어 잘못된 라벨을 보이게 된다.
- 값 읽기를 포기하고 라벨만 고정하면 IIDX 식으로 값을 보이는 요구와 모순된다.
- 조사 문서의 안 A'(확장 대역을 불린 공간에만 두고 Lua `draw` 함수가 `main_state.option` 으로 읽음)를 택했다. 원본 선언을 막는 게이트 8곳을 고치지 않고 호스트의 불린 응답 몇 줄이면 된다. 이벤트는 게이트가 없는 공간이라 같은 대역 방식으로 줄만 더했다.
- A' 는 원래 두 값 옵션에만 쓸 수 있다고 적혀 있었다. 구현은 등급 다섯 값과 닿지 않는 키 아홉 칸도 불린 하나씩으로 나눠 답하는 쪽으로 넓혔다. 값마다 숫자 id 를 따로 만들지 않고 같은 규칙을 쓰기 위해서다.
- 위치를 원본 선언 최대 위, 스킨 몫 대역 밖으로 둔 것은 조사 3.4절의 충돌 방지 규칙(제안)을 그대로 따랐다.

## 기각된 대안

| 대안 | 기각 이유 |
| --- | --- |
| 원본 선언을 확인하는 게이트 8곳(`crates/rbms-skin` 의 `names.rs`·`lua/main_state.rs`·`loader/track.rs`, `rbms-render` 의 `draw.rs`·`text.rs`·`songlist/build.rs` 등)이 확장 표를 통과시키게 함(결정 3 A) | 고칠 곳이 많고 불린 함수 읽기만으로 같은 효과를 낼 수 있다. 확장 id 를 `op` 목록에 쓰려는 스킨이 생기면 다시 검토한다 |
| 원본 id 재사용(결정 3 B) | 외부 스킨의 고정 라벨과 어긋난다 |
| 값 읽기 포기(결정 3 C) | IIDX 식 값 표시와 모순된다 |
| 옛 사설 id 20001~20316 대역을 되살림 | 2026-10-09 결정이 폐기한 번호다. 새 대역은 번호를 겹치지 않게 21000 부터 둔다 |

## 영향과 관련 파일

- 호스트: `apps/rbms-player/src/skin_host/{extension,mod,options,select,result}.rs`, `skin_host/select/{shown,bridge}.rs`, `stage/select/events.rs`.
- 기본 스킨: `assets/skins/rbms-default/lua/props.lua`(`props.extension`), 이 id 를 읽는 `lua/select/{sheet,panel,info,words}.lua`, `lua/result/score.lua`.
- 테스트: `skin_host/extension.rs` 의 대역 단언, `skin_host/tests.rs`(이벤트 대역 단언), `stage/select/events_tests.rs`, `skin_host/{select,result}/tests.rs`, `skin_host/options/tests.rs`. 원본 id 표 체크섬 테스트는 그대로 통과한다.
- 외부 스킨: 이 대역을 읽지 않아 동작이 바뀌지 않는다. 다만 어떤 스킨이 같은 번호를 자기 용도로 쓰면 R-BMS 에서는 이 표의 뜻으로 읽힌다.
- 문서: 조사 문서 [option-panels.md](../reference/option-panels.md) 3.4절의 "현재 코드에 R-BMS 전용 id 가 하나도 없다"는 구현 전 서술이다. `docs/reference/skin-compat/` 의 속성 문서에는 이 대역이 없다.
- 사용자 확인이 필요하다(미확인): 2026-10-09 결정(원본 id 만 답함)을 이 두 대역만큼 바꾸는 것. 사용자가 되돌리라고 하면 SCRATCH AUTO·KEEP GREEN NUMBER 값 표시, 등급 표시, 닿지 않는 키 흐림은 읽을 길이 없어진다.

## 날짜와 사용자 원문

2026-10-22.

- 설계 확정 지시: "끝까지 다 구현해놔"
- 2026-10-09 결정의 근거 문장(`docs/acknowledge/2026-10-09-lua-skin-compat-decisions.md`, 이전 결정과의 관계): "사설 id(D10)... 는 이 결정으로 폐기된다."
