# ADR: 조작·단축키에 키 2개 조합을 묶을 수 있고, 키·패드 버튼 하나를 ESC 로 묶을 수 있다

> 최종 갱신 2026-11-19 · 대응 커밋 `c4e75c2`
> 상태: 유효(코드는 `dev` 에 있음. 사용자가 정한 것은 요구 두 줄뿐이고 M10~M12 와 "구현에서 정한 것"은 메인·구현 단위의 결정이며 사용자 확인 전. 컨트롤러 실기 확인 전) · 결정일 2026-11-19 · 결정자 사용자(요구)와 메인(M10~M12), 구현 단위 U3(구현에서 정한 것), 메인(병합 수정) · 커밋 `c4e75c2`
> 관계: 키 설정의 저장 형식과 충돌 규칙([play-options.md](../play-options.md) 5.1절·21절)을 넓힌다. START·SELECT 와 레인 키를 눌러 두는 기존 내장 조합([play-options.md](../play-options.md) 5.2절)은 그대로 두고 명시적으로 묶은 조합이 우선한다. Esc 의 판 처리 정책은 [Esc 는 결과 화면으로](2026-10-23-adr-escape-shows-the-result.md)와 [Esc 는 모든 판을 남긴다](2026-10-15-adr-escape-keeps-every-run.md)가 정한 대로이고 이 ADR 은 입력 길만 더한다. 계획 U3, 조사 6·7절, 보고 [U3.md](../history/2026-11-19-feature-upgrade-unit-reports/U3.md).

## 배경

- 사용자 요청 요지(2026-11-19): 6 컨트롤러에 Esc 지정, 7 key config 조합키. 컨트롤러로만 플레이할 때 Esc(포기·뒤로)를 누를 방법이 없었고, 단축키가 부족한 컨트롤러에서 두 버튼을 함께 누르는 조합을 쓰고 싶다는 뜻이다.
- 지금까지 키보드 바인딩은 키 이름 문자열 하나(`ControlBinds` 의 `String`, `KeyConfig.shortcuts: BTreeMap<String, String>`)이고 패드는 `PadBinding` 하나였다. Esc 는 키보드 고정 키였고 컨트롤러에 묶을 수 있는 것은 레인·`ControlAction`·`Shortcut` 뿐이었다. Esc 는 누름과 뗌이 다 필요하다(PLAY 의 ESCAPE 설정이 IMMEDIATE·HOLD 600 ms·DOUBLE TAP).

## 결정

### 1) 사용자가 정한 것

| 번호 | 결정 |
| --- | --- |
| D1 | 컨트롤러에 Esc 를 지정할 수 있게 한다 |
| D2 | 키 설정에서 키 2개 조합을 지정할 수 있게 한다 |

### 2) 메인 결정(사용자 미확인)

| 번호 | 결정 |
| --- | --- |
| M10 | 새 조작 `ESC`(키 편집기의 조작 목록 끝). 키보드 기본 키 없음(Esc 고정 키는 그대로), 패드 기본 없음. 묶인 키·버튼의 누름·뗌을 Esc 키 입력으로 화면에 넘긴다. HOLD·DOUBLE TAP 을 포함해 모든 화면에서 같다 |
| M11 | 조작·단축키에 키 2개 조합을 묶을 수 있다(키보드·패드). 둘 다 눌린 순간(순서 무관) 실행하고, 누르고 있는 동안 반복하는 조작은 둘 다 눌린 동안 반복한다. 레인과 스크래치 역방향 키는 단일 키만 |
| M12 | 조합이 완성된 누름에서는 그 키의 단일 조작·단축키를 실행하지 않는다. 레인 입력은 항상 판정에 간다. 조합의 첫 키에 단일 조작이 묶여 있으면 "그 조작이 먼저 실행된다"는 안내를 편집기가 낸다(거절하지 않음) |

계획서 U3 3항의 "조합이 완성된 누름은 그 키의 단일 조작·단축키를 누른다"는 오타이고 M12 대로 "실행하지 않는다"가 맞다. 구현은 M12 를 따른다.

### 3) 구현에서 정한 것(구현 단위·메인이 정함, 사용자 미확인)

| 항목 | 내용 |
| --- | --- |
| ESC 의 자리 | `ControlAction::Escape`(화면 이름 `ESC`, 설정 키 `ESCAPE`, `ControlBinds.escape` 기본 빈 값). `ControlAction::ALL` 이 21개, `Shortcut::ALL` 이 34개다(결과 화면 `CaptureLayout` 포함, [LAYOUT PICK ADR](2026-11-19-adr-layout-pick-playback-and-capture.md)). ESC 는 모든 단축키·레인·조작과 같은 순간에 읽히는 것으로 본다(`Bindable::read_with`) |
| 키보드 ESC | 공통 래퍼 `Stage::handle_key`(`stage/mod.rs`)가 ESC 에 준 키를 `KeyCode::Escape` 로 바꿔 옵션 오버레이 → 화면 순으로 넘긴다(누름·뗌·반복). 화면마다 `control_for` 를 보는 방식이면 빠지는 화면이 생기므로 한 곳에서 한다. 물리 Esc 고정 키 규칙은 그대로다 |
| 패드 ESC | `PadTarget::event` 가 ESC 만 뗌도 낸다. 프레임 루프가 이벤트마다 `PadMark { pressed, made_a_chord }` 를 붙여 `Stage::handle_marked_pad` 에 넘기고, 이 함수가 ESC 를 `Escape` 키 입력(누름·뗌)으로 바꿔 `deliver_key`(옵션 오버레이 → 화면)로 보낸다. 그래서 IMMEDIATE·HOLD·DOUBLE TAP 이 패드 버튼으로도 키와 같은 결과를 낸다. 처음 구현은 표식을 `HeldKeys` 큐에 넣고 `handle_pad` 가 꺼냈으나 메인이 이벤트와 함께 넘기는 방식으로 고쳤다 |
| 저장 형식(키보드) | 단일 키 토큰은 그대로(`SHIFT`), 조합은 두 토큰을 `+` 로 잇는다(`A+B`). `KeyBind::{Single, Chord}`(`keyconfig/chord.rs`)가 읽고 쓴다. 화면 표기는 `A + B`. 조합은 순서와 무관하게 같은 키 쌍이다(`same_keys`). 기존 파일의 단일 토큰은 그대로 읽히고 `controls.escape` 가 없어도 읽힌다 |
| 저장 형식(패드) | `PadConfig` 의 기존 첫 요소 표는 그대로 두고 둘째 요소 표 `control_chords`·`shortcut_chords`(`BTreeMap<String, PadBinding>`, `#[serde(default)]`, `skip_serializing_if` 없음)를 더했다. 둘째 요소가 없으면 단일이다. 패드 단일 지정·해제는 둘째 요소도 지운다 |
| 조합이 안 되는 것 | 레인·스크래치 역방향(played, not fired), 같은 키 두 번(`A+A`), 이름 없는 키, 아날로그 턴테이블, 패드를 못 받는 단축키(`OptionsPin`·`Hold`·`RefDetail`) |
| 눌림 판정 | `control_held`·`control_repeating`·`shortcut_held`·`fine_pressed`·`keyboard_index_pressed`·`control_for`·`shortcut_for`·`is_shortcut` 와 패드 `held` 가 조합을 이해한다. 조합이 눌려 있는 동안 그 키들은 단일 대상(START·SELECT·단축키 held, 키 인덱스)으로 세지 않는다. 키보드는 `note_key` 가 `ChordEvent`(만든 조합·깬 조합)를 `HeldKeys` 에 두고, 래퍼가 조합 조작은 `handle_pad(PadEvent::Control)` 로 먼저 넘긴 뒤 원래 키를 넘긴다. 패드는 매퍼가 단일/쌍 계획(`PadPlan`)으로 쌍이 완성될 때 한 번 발사하고, 쌍에 묶인 요소의 단일 조작·단축키는 억제한다(레인은 발사하고 mark 를 남김) |
| 레인 | PLAY 화면(`StageId::Play`)에서만 조합 완성 누름도 판정으로 간다. 그 밖의 화면에서는 그 누름의 레인 의미(키 인덱스)를 조합의 것으로 막는다(`lane_input_for` 가 None, 패드 Lane 누름 버림). 선곡에서 START + 레인 키를 명시 조합으로 묶었을 때 레인 키가 "플레이 시작" 같은 키 인덱스로 새지 않게 하기 위해서다. LAYOUT PICK·결과 등 레인을 읽는 다른 화면도 같다 |
| 충돌 | `clash_among` 이 `KeyBind` 를 받는다. 같은 화면에서 같은 조합(순서 무관)은 거절하고(`ChordTaken`), 고정 키(Esc 등)가 든 조합도 거절한다. 조합의 한 키가 다른 대상의 단일 키이거나 단일 키가 다른 대상의 조합에 들어 있으면 거절하지 않고 양쪽 줄에 안내한다(`ChordKeyAlone`·`KeyInChord`, 경고 색). 두 조합이 키 하나를 공유하는 것은 안내하지 않는다(단독 동작이 없으므로) |
| 편집기 | 조작·단축키 칸에서 한 키를 눌렀다 떼면 단일, 첫 키를 누른 채 둘째 키를 누르면 조합이다(`ChordCapture<T>`, 키보드·패드 공용). 레인 칸은 첫 누름에 확정한다. 패드는 `PadCapture` 가 누름·뗌을 전달하고 보관 개수를 `CAPTURE_EVENTS_KEPT` = 16 으로 제한한다(프레임 루프가 편집기 밖에서도 모든 변화를 기록하므로). 축은 `CAPTURE_AXIS_THRESHOLD` 를 넘을 때 누름, 안으로 돌아오면 뗌이다. 편집기는 키보드를 항상 있는 그대로 읽는다(`StageHandler::reads_keys_as_they_are`: ESC 에 준 키도 편집기 안에서는 그 키). 패드 칸이 요소를 기다리는 동안은 패드 ESC 를 받지 않는다(`takes_pad_escape`) |
| 표시 | 키 가이드(H)가 조합을 `A + B` 로 보이고 플레이 중 ESC 줄에 ESC 에 준 키를 `ESC / BACKSPACE` 꼴로 보인다. 메인이 설정 화면(`stage/settings.rs`)·옵션 오버레이(`app_options.rs`)의 키 안내도 조합을 표기하도록 고쳤다. 편집기 문구 힌트 둘(`CaptureChordKeyHint`·`CaptureChordPadHint`)과 알림 셋(`ChordTaken`·`ChordKeyAlone`·`KeyInChord`)과 라벨 `ESC` 는 세 언어다 |
| 스킨 바닥 표기 | 기본 스킨 선곡 바닥의 키 표지(`select/controls.lua`)는 고정 키 정적 문구라 조합이 나타날 자리가 없다. 바꾼 것 없음 |

## 이유

- ESC 를 입력 래퍼에서 키 입력으로 바꾸면 화면·오버레이·ESCAPE 설정이 키와 같게 동작해 화면마다 고칠 곳이 없다.
- 두 키가 모두 눌린 순간 실행하되 그 키의 단일 동작을 억제하면, 조합의 첫 키가 다른 일을 해 버리는 사고를 막는다. 단일 실행을 뗄 때까지 미루면 모든 단일 키에 입력 지연이 생기므로 하지 않는다.
- 레인은 판정을 먹지 않아야 한다(노트는 바인딩에 먹히지 않는다). 다른 화면에서는 레인 키가 키 인덱스(선곡의 건반 조작)로 새는 것이 더 나쁘다.
- 조합 토큰이 단일 토큰과 같은 문자열 칸에 들어가 키 설정 파일 형식이 바뀌지 않는다.

## 기각된 대안

| 대안 | 기각 이유 |
| --- | --- |
| 화면마다 패드 ESC 처리 추가 | 빠지는 화면이 생긴다 |
| 3키 이상 조합 | 편집기·충돌 규칙이 커지고 요청은 "2개" 다 |
| 조합의 단일 실행을 뗄 때까지 미루기 | 입력 지연 |
| 조합 충돌 시 단일 쪽 거절 | 사용자가 의도한 구성을 막는다. 안내만 낸다 |
| 이벤트 열거형 `PadEvent` 를 바꿔 표식을 담기 | 패드 이벤트가 화면 밖(`lib.rs`)에서 분배되어 영향이 크다. `PadMark` 를 이벤트 옆으로 넘긴다 |

## 영향과 관련 파일

- 코드: `apps/rbms-player/src/keyconfig.rs`·`keyconfig/chord.rs`·`keyconfig/chord/tests.rs`, `gamepad.rs`·`gamepad/chord_tests.rs`, `app_input.rs`, `stage/mod.rs`(`handle_key`, `key_through_bindings`, `handle_marked_pad`, `StageHandler::{reads_keys_as_they_are, takes_pad_escape}`), `stage/keyconfig.rs`·`keyconfig/table.rs`·`keyconfig/chord_tests.rs`, `stage/input_tests.rs`, `stage/select/guide.rs`, `text/menus/key_editor.rs`, 계약 `contract/client-requests/settings-blob.json`(keyconfig 에 `controls.escape`, `pad.control_chords`, `pad.shortcut_chords` 가 늘어 계약을 다시 만듦).
- 시험: 조합 상태 기계(`a_chord_runs_its_control_once_in_either_order_and_still_plays_the_lane_of_its_key`, `a_chord_is_held_while_both_keys_are_and_takes_its_keys_from_what_they_hold_alone`, `the_press_that_makes_a_chord_is_the_chords_shortcut_and_not_its_keys_own`, `the_lane_of_the_press_that_makes_a_chord_is_played_only_in_a_run`, `losing_focus_breaks_a_chord_and_lets_esc_come_up`), 옛 키 설정 파일(`a_key_config_from_before_chords_still_loads_and_a_chord_survives_the_file`), 패드 쌍(`two_elements_held_together_fire_once_in_either_order_and_the_making_press_plays_only_its_lane` 등 6개), 패드 ESC(`esc_on_a_controller_does_what_the_escape_key_does_under_every_setting`: IMMEDIATE·HOLD·DOUBLE TAP), 편집기(`a_row_that_takes_a_chord_waits_to_tell_one_key_from_two` 등 5개).
- 알려진 위험(테스트 부채 3.168~3.170, 실기 QA 3.29 G4~G6):
  - 컨트롤러로 확인하지 못했다(이 환경에 기기 없음). 패드는 주입한 이벤트로만 시험했다.
  - 옛 빌드가 새 파일을 읽으면 조합 토큰(`A+B`)은 단축키는 기본 키로, 조작은 키 없음으로 읽힌다(앞으로의 호환은 보장하지 않음). 패드 `control_chords` 는 옛 빌드가 무시해 첫 요소 단일로 동작한다.
  - 패드 `PadMark` 는 `lib.rs` 가 `poll_pad` 결과를 빠짐없이 순서대로 `handle_marked_pad` 에 넘긴다는 전제에 기댄다(지금 코드는 그렇다). 이 루프를 바꾸면 ESC 누름·뗌 판단이 어긋날 수 있다.
  - ESC 에 준 키가 짝 없이 뗌만 화면에 갈 수 있다(조합 완성 누름으로 억제된 뒤 뗌). PLAY 의 HOLD 는 뗌을 취소로 보아 해가 없고 다른 화면은 뗌을 무시한다.
  - 포커스를 잃을 때 편집기가 키보드 조합 캡처 중이면 첫 키의 뗌이 단일 지정으로 확정된다(드문 경우).
  - PLAY 밖 화면에서 조합 완성 누름의 레인 의미를 막으므로, 레인 키를 조합에 쓴 사용자는 선곡에서 그 누름이 키 인덱스로 쓰이지 않는다(의도).
- 문서: [play-options.md](../play-options.md) 5.1절·21절, [crates.md](../crates.md) `Keys` 항목.

## 날짜와 사용자 원문

2026-11-19.

- 사용자 요청 요지(PROCESS 체크리스트): "6 컨트롤러 Esc 지정, 7 key config 조합키". 원문 전문은 이 저장소에 없다.
