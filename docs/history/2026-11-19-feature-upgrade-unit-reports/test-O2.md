# O2 테스트 웨이브 보고 (wf_3df278ff-bb7)

> 테스트 웨이브의 에이전트가 끝낼 때 낸 보고 원문이다. 메인이 병합하면서 바꾼 것은 계획서 9절에 있다.

- 상태: done

## 바꾼 파일

- apps/rbms-player/src/app_input.rs
- apps/rbms-player/src/gamepad.rs
- apps/rbms-player/src/gamepad/chord_tests.rs
- apps/rbms-player/src/keyconfig.rs
- apps/rbms-player/src/keyconfig_tests.rs
- apps/rbms-player/src/stage/input_tests.rs
- apps/rbms-player/src/stage/mod.rs

## 요약

U3(패드 ESC·2키 조합)에서 반례를 찾아 결함 5건을 실패하는 시험으로 먼저 재현했고, 원인을 고친 뒤 시험을 남겼습니다. 기존 U3 시험 하나는 기계 부하에 따라 결과가 달라져서 안정화했습니다. 범위 밖 결함 5건은 고치지 않고 보고만 합니다.

고친 결함:
(1) 조합이 풀린 뒤 남은 키가 다시 '눌림'으로 읽힘. START 키를 쥔 채 다른 키로 조합을 만들었다 그 키만 떼면 START 가 새로 눌린 것처럼 읽혔습니다. 플레이 화면에서는 START 두 번 누름(500 ms 안)으로 레인 커버가 켜지거나 꺼지는 조작이 됩니다. 패드도 같았는데, 패드 ESC 를 START+SELECT 로 묶는 흔한 설정에서 바로 걸립니다. 선곡의 레인 키 번호(키 인덱스)도 같은 이유로 다시 눌림으로 읽혔습니다. U3 시험 2개는 이 동작을 '조합이 풀리면 다시 START' 로 굳혀 두었고, '뗐다 다시 눌러야 START' 로 바꿨습니다. 고친 방법: 조합에 한 번 든 키·버튼은 뗄 때까지 조합의 것으로 남깁니다(`HeldKeys.kept_for_chords`, `PadMapper.kept_for_pairs`).
(2) 키 편집기에서 패드의 조합을 포착하는 도중 패드를 뽑으면, 쥐고 있던 버튼의 뗌이 오지 않았습니다. 다시 꽂고 다른 버튼을 누르면 '떼어진 버튼 + 새 버튼' 조합으로 묶였습니다. 이제 `PadCapture` 가 분리될 때 쥐고 있던 버튼·축 방향을 모두 뗌으로 냅니다.
(3) ESC 에 Enter·방향키·H 를 줄 수 있었습니다. ESC 의 키는 모든 화면에서 Esc 로 바뀌므로, 선곡의 확정·이동 키가 사라집니다. 조합 LSHIFT+ENTER 로 주면 Esc 와 Enter 가 함께 실행됩니다. 이제 ESC 는 선곡 화면의 고정 키를 받지 않습니다. 선곡 고정 키가 결과·리플레이 화면의 고정 키를 모두 포함합니다.
(4) ESC 가 다른 모드의 레인 키를 가져갈 수 있었습니다. 편집기는 보고 있는 모드(예: 7키)의 레인과만 충돌을 봅니다. 그래서 7키에서 ESC 를 M 에 주면 거절되지 않고, 14키 플레이에서 M 을 치면 판이 끝납니다. ESC 도 START·SELECT 처럼 지금 모드의 레인에 양보합니다(`yields_to_lanes`).
(5) 검색어 입력 중 글자 키와 조합이 만들어졌습니다. 글자 Q 를 쥔 채 Shift 를 누르면(빠른 타이핑에서 흔함) ESC=LSHIFT+Q 가 실행돼 검색이 닫혔습니다. 반대 순서에서는 글자가 우선이라 동작이 일관되지 않았습니다. 이제 입력 중에는 글자 키가 든 조합을 만들지 않습니다(`Stage::handle_key`).

시험 안정화: 기존 시험 `losing_focus_breaks_a_chord_and_lets_esc_come_up` 은 1마디(2초) 차트를 써서, 기계가 바쁘면 첫 프레임 전에 차트가 실제 시간으로 끝나 결과 화면으로 갔습니다. 원래 코드에서도 같았습니다(시험 시간 4.8초면 통과, 5.1초 이상이면 실패). 시험 차트에 멀리 떨어진 노트를 하나 더해 해결했습니다.

결함이 아닌 것으로 확인한 가설(무엇을 어떻게 확인했는지):
- A+B 와 A+C 를 함께 묶고 A, B, C 를 차례로 누르는 경우: `held_chords_with` 는 방금 누른 키가 든 조합만 돌려줍니다. 그래서 C 의 누름은 A+C 만 만들고, A 를 떼면 둘 다 깨집니다.
- 키를 쥐고 있을 때 오는 OS 반복 입력: `note_key` 가 아무것도 바꾸지 않아 조합 사건이 없습니다. 플레이에서 누르고 있는 동안 반복하는 조작은 프레임마다 `control_repeating` 으로 읽습니다.
- 조합을 쥔 채 화면이 바뀌거나 포커스를 잃는 경우: 뗌이 다음 화면에 그대로 가고, 포커스 잃음은 실제 뗌과 같은 경로로 갑니다. 시험이 통과합니다.
- 리플레이에 기록되는 입력: 조합을 완성한 누름의 레인은 PLAY 에서 `lane_input` 으로 판정되고 기록되며, 뗌도 빠지지 않습니다. 시험 `a_chord_runs_its_control_once...` 이 확인합니다.
- 짝 없이 ESC 의 뗌만 가는 경우: 플레이의 IMMEDIATE·DOUBLE TAP 은 누름만 읽고, HOLD 는 뗌을 취소로만 씁니다. 키 편집기도 뗌을 무시합니다.
- 패드의 디바운스: 늦게 열리는 뗌(`take_opened`)이 조합의 깨짐과 ESC 뗌을 냅니다. 한 폴링에 누름과 뗌이 함께 오면 순서대로 화면에 가고, 화면이 바뀌면 그다음 사건은 새 화면이 받습니다(lib.rs 의 폴링 루프).
- 옛 파일과 손으로 고친 토큰: `A+`, `+B`, `A+A`, `A+B+C`, 빈 값은 키로 읽히지 않습니다(단축키는 기본 키, 조작은 키 없음). 공백·소문자는 읽힙니다. 패드 둘째 요소 표가 없는 파일도 읽힙니다(serde default).
- 키 편집기: 포착 중 Esc 는 포착을 취소합니다. 패드 칸이 버튼을 기다리는 동안 패드 ESC 의 누름은 막고, 뗌은 무시됩니다. 포커스를 잃으면 단일 지정으로 확정되는데, U3 이 이미 받아들인 동작입니다.
- PLAY 에서 조작 조합이 먼저 실행되고 그다음 레인이 판정되는 순서도 맞습니다.

## 기준별 판정

- [pass] 가설을 실패하는 시험으로 증명한 뒤 원인을 고치고 시험을 남긴다: 수정 전 실행: 새 시험·바뀐 시험 6개가 실패했습니다(a_chord_is_held..., a_pair_is_held..., a_capture_lets_go..., esc_cannot_take..., a_chord_with_a_key_of_the_word..., the_key_given_to_esc_plays_a_lane...). 수정 뒤 전부 통과합니다.
- [pass] 결함이 아닌 가설도 한 줄씩 보고한다: summary 의 '결함이 아닌 것으로 확인한 가설' 목록
- [pass] 범위 밖 결함은 고치지 않고 fixed=false 로 보고한다: stage/result.rs, stage/layout_pick.rs, app_network.rs, 다른 모드 레인의 조작 키, 턴테이블 축 조합 건을 findings 에 fixed=false 로 넣었습니다
- [pass] 좁은 검사(test·clippy·fmt)를 통과한다: cargo test -p rbms-player --locked --lib: 3276 passed, 0 failed. clippy -D warnings 경고 없음. cargo fmt --all --check 통과
- [pass] 떠는 시험을 만들지 않는다: 새 시험은 실제 시계에 기대지 않습니다. 기존 U3 시험이 부하에 따라 실패하던 원인(차트가 실제 시간으로 끝남)을 고친 뒤, 부하가 있는 상태(load avg 11)에서 3회 연속 통과했습니다

## 찾은 것

- [medium, 고침] 조합이 풀린 뒤 남은 키·버튼이 START·SELECT·키 인덱스로 다시 눌림 (플레이에서 START 두 번 누름으로 커버가 바뀜): AppShared::keys_in_chords 와 PadMapper::held 가 지금 쥐고 있는 조합만 봤습니다. 그래서 START 키를 쥔 채 A+Z 조합을 만들었다 Z 만 떼면 start_pressed 가 false 에서 true 로 바뀌었습니다. 플레이 화면은 이 바뀜을 프레임마다 START 누름으로 읽고(stage/play/controls.rs read_held → start_tapped), 500 ms 안의 두 번째 누름이면 레인 커버를 켜거나 끕니다. 패드도 같았습니다(chord_tests 의 기존 단언 '...START's again once it is broken'). 수정: HeldKeys.kept_for_chords 와 PadMapper.kept_for_pairs 로, 조합에 든 키·버튼을 뗄 때까지 조합의 것으로 남깁니다. (시험: stage::input_tests::a_chord_is_held_while_both_keys_are_and_takes_its_keys_from_what_they_hold_alone, gamepad::chord_tests::a_pair_is_held_while_both_are_down_and_takes_its_elements_from_what_they_hold_alone)
- [medium, 고침] ESC 에 Enter·NumpadEnter·방향키·H 를 줄 수 있어 선곡의 확정·이동 키가 Esc 로 바뀜: Bindable::fixed_keys 가 조작에는 [Escape] 만 막았습니다. ESC 의 단일 키는 Stage::key_through_bindings 에서 모든 화면에 KeyCode::Escape 로 바뀌어 갑니다. 그래서 ESC=ENTER 로 두면 선곡에서 Enter 가 Esc 가 됩니다. 조합 RSHIFT+ENTER 로 두면 Esc 가 먼저 가고 Enter 도 그대로 가서 둘 다 실행됩니다. 수정: Control(Escape) 의 고정 키를 fixed_keys_of(Browser) 로 바꿨습니다(결과·리플레이의 고정 키를 포함합니다). (시험: keyconfig_tests::esc_cannot_take_a_key_any_screen_keeps_for_itself)
- [medium, 고침] ESC 에 다른 모드의 레인 키를 주면 그 모드의 플레이에서 레인을 친 순간 판이 끝남: 편집기의 충돌 검사(clash_of)는 보고 있는 모드의 레인만 봅니다. 시험에서 7키 편집기가 14키 전용 레인 키를 ESC 에 주는 것을 거절하지 않음을 확인했습니다. 그리고 escape_key 는 레인보다 먼저 키를 Esc 로 바꿨습니다(수정 전, 레인 키에 ESC 를 주고 PLAY 에서 누르면 판을 떠남이 재현됐습니다). 수정: ControlAction::yields_to_lanes 에 Escape 를 더해, START·SELECT 처럼 지금 모드의 레인에 양보합니다. (시험: stage::input_tests::the_key_given_to_esc_plays_a_lane_that_has_it, keyconfig_tests::start_and_select_are_the_controls_that_give_way_to_a_lane)
- [low, 고침] 검색어 입력 중 글자 키를 쥔 채 수식 키를 누르면 조합(ESC 등)이 실행됨: Stage::handle_key 는 입력 중 '지금 누른 키'가 글자 키일 때만 조합을 건너뛰었습니다. 그래서 ESC=LSHIFT+Q 일 때 Q 를 쥔 채 LSHIFT 를 누르면 Escape 가 가서 검색이 닫혔습니다(빠른 타이핑의 키 겹침). 반대 순서에서는 글자가 이겨서 동작이 일관되지 않았습니다. 수정: 입력 중에는 글자·편집 키가 든 조합을 조합 사건에서 뺍니다(chord_has_a_key_of_the_word). (시험: stage::input_tests::a_chord_with_a_key_of_the_word_is_no_chord_while_a_word_is_typed)
- [low, 고침] 키 편집기가 패드 조합을 포착하는 도중 패드를 뽑으면, 다시 꽂고 누른 버튼과 떼어진 버튼이 조합으로 묶임: PadCapture::note(Disconnected) 가 축 상태만 지우고 뗌을 내지 않았습니다. 그래서 ChordCapture 의 첫 요소가 남아 다음 누름과 Captured::Chord 가 됐습니다. 수정: 쥐고 있던 버튼(새 buttons 집합)과 축 방향을 분리될 때 모두 뗌으로 냅니다. 키보드에서 포커스를 잃을 때와 같은 결과(단일 지정)가 됩니다. (시험: gamepad::chord_tests::a_capture_lets_go_of_what_a_controller_held_when_it_went_away)
- [low, 고침] 기존 U3 시험 losing_focus_breaks_a_chord_and_lets_esc_come_up 이 기계 부하에 따라 실패함: 원래 소스 그대로에서도 시험 시간 약 4.8초면 통과, 5.1초 이상이면 결과 화면으로 가서 실패했습니다. 1마디(2초) 차트가 실제 시간으로 끝나 run_is_over 가 됐기 때문입니다. 수정: 시험 차트(CHART)에 99마디의 노트를 더했습니다. 부하(load avg 11)에서 3회 연속 통과했습니다. (시험: stage::input_tests (10 passed x3))
- [medium, 안 고침] 스킨이 그리는 결과 화면이 조합으로 묶은 RETRY·NEXT 를 읽지 못하고, 조합을 완성한 누름에서 단일 RETRY 가 실행됨: stage/result.rs:1281 이 listen_for(shortcut_key(Retry), shortcut_key(NextSong)) 로 단일 키만 넘깁니다. 조합이면 shortcut_key 가 None 이라 조합 RETRY·NEXT 가 동작하지 않습니다. 또 ResultScene::key(:424)가 키 코드만 비교하므로, RETRY=R 이고 NEXT=LSHIFT+R 이면 LSHIFT, R 순서로 눌렀을 때 RETRY 가 실행됩니다(M12 위반). 기본 결과 화면(shortcut_for)은 맞습니다. 코드 경로로만 확인했고 범위 밖 파일이라 시험으로 재현하지 않았습니다(의심). (시험: 없음(범위 밖 stage/result.rs))
- [low, 안 고침] LAYOUT PICK 이 조합 완성 누름의 레인과 START·SELECT 를 자체 키 목록으로 읽어 막지 않음: stage/layout_pick.rs:726-735 의 other_key 가 self.lane_keys 와 keyconfig.control_key(Start/Select) 를 직접 비교합니다. 그래서 lane_input_for·control_for 의 조합 억제를 거치지 않습니다. U3 보고의 'LAYOUT PICK 도 같은 규칙' 과 다릅니다. 코드 읽기로만 확인했습니다(의심). (시험: 없음(범위 밖 stage/layout_pick.rs))
- [low, 안 고침] 설정 동기화가 키 설정을 통째로 바꾸며 눌린 키 상태를 버리고 pad.sanitise 를 거치지 않음: app_network.rs:409 `self.keyconfig = payload.keyconfig;` 가 HeldKeys(serde skip)를 기본값으로 만듭니다. 그 순간 쥐고 있던 ESC 조합의 뗌이 다시는 가지 않습니다. KeyConfig::load 와 달리 pad.sanitise() 도 부르지 않습니다. 이 코드는 U3 이전부터 있던 것입니다. (시험: 없음(범위 밖 app_network.rs))
- [low, 안 고침] ESC 외의 조작 키도 다른 모드의 레인 키를 가져가 그 모드에서 레인을 죽임: clash_of 가 편집 중인 모드만 검사합니다. 플레이의 handle_key 는 control_for 를 레인보다 먼저 봅니다. 그래서 7키 편집기에서 HI-SPEED UP 을 M 에 주면 14키에서 M 레인이 죽습니다. U3 이전부터 있던 정책 문제입니다. ESC 만 이번에 레인에 양보하게 고쳤습니다. (시험: 없음(정책 결정 필요))
- [low, 안 고침] 턴테이블로 쓰는 축의 방향이 조작 조합의 한 요소로 묶일 수 있음: PadConfig::bind_chord 는 AnalogScratch 만 거절하고, 같은 축의 PadBinding::Axis 는 받습니다. 포착 중 턴테이블을 돌리면 CAPTURE_AXIS_THRESHOLD 를 넘어 Axis 요소가 포착될 수 있습니다. 그러면 그 요소는 플레이에서 턴테이블 알고리즘으로 읽힙니다. U3 의 '아날로그 턴테이블은 조합 불가' 취지와 어긋납니다. 실기로 확인하지 않았습니다(의심). (시험: 없음)

## 남은 위험

- 조합에 든 키를 뗄 때까지 단일로 읽지 않는 규칙이 생겼습니다. 이제 START 를 쥔 채 조합을 쓴 뒤 같은 START 로 'START+레인' 내장 조작을 하려면 START 를 다시 눌러야 합니다.
- ESC 가 레인에 양보하므로, 손으로 고친 파일이나 다른 모드에서 ESC 를 레인 키에 준 사용자는 그 모드에서 ESC 키가 없는 것으로 보입니다(키 가이드는 control_bind_in_force 를 따라 표시합니다).
- ESC 의 고정 키가 늘어서, 이미 ESC=ENTER 등을 저장한 파일은 편집기에서 빨간 충돌로 보입니다. 실행 중에는 그대로 동작합니다.
- 입력 중 조합 필터는 선곡 검색어(takes_composed_text)에만 적용됩니다. 스킨의 텍스트 입력은 skin_text_key 가 먼저 가져가므로 해당하지 않습니다.
- 패드 분리 때의 뗌은 모든 장치의 Disconnected 사건에 반응합니다. 장치 이름을 지정하지 않은 ANY 에서 다른 패드가 분리되면, 편집기 포착이 단일로 확정될 수 있습니다.

## 갱신이 필요한 문서

- docs/reference/2026-11-19-feature-upgrade-12/README.md 7절: 조합에 든 키·버튼은 뗄 때까지 조합의 것(HeldKeys.kept_for_chords, PadMapper.kept_for_pairs), 입력 중 글자 키 조합 무시, ESC 고정 키(선곡 고정 키)와 레인 양보, PadCapture 분리 때 뗌
- docs/play-options.md 조합 지정 규칙 절: 위 네 규칙
- docs/quality-assurance 테스트 부채: 스킨 결과 화면의 조합 RETRY/NEXT, LAYOUT PICK 의 조합 억제, 턴테이블 축 조합, 설정 동기화의 held 초기화
- docs/plan/2026-11-19-feature-upgrade-12.md 결과 절: U3 반례 검토 결과

## 돌린 검사

- `cargo test -p rbms-player --locked --lib -- stage::input_tests gamepad::chord_tests keyconfig_tests::esc_cannot (수정 전)` → 6 failed: 새 시험·바뀐 시험이 결함을 재현함
- `cargo test -p rbms-player --locked --lib -- stage::input_tests gamepad keyconfig (수정 뒤)` → 198 passed, 1 failed(부하에 따라 실패하던 focus 시험) → 차트 수정 뒤 stage::input_tests 10 passed x3
- `cargo test -p rbms-player --locked --lib` → 3276 passed; 0 failed; 2 ignored
- `cargo clippy -p rbms-player --all-targets --locked -- -D warnings` → 경고 없이 통과
- `cargo fmt --all --check` → cargo fmt -p rbms-player 적용 뒤 통과
- `rbms-player 실기·패드 실장치` → 실행하지 않음(실장치 없음)
