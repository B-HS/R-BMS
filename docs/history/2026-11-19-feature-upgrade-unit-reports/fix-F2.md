# F2 수리 웨이브 보고 (wf_08935ad2-8ca)

> 수리 웨이브의 에이전트가 끝낼 때 낸 보고 원문이다.

- 상태: done

## 바꾼 파일

- apps/rbms-player/src/app_input.rs
- apps/rbms-player/src/app_network.rs
- apps/rbms-player/src/app_network/tests.rs
- apps/rbms-player/src/captured_layouts.rs
- apps/rbms-player/src/captured_layouts/tests.rs
- apps/rbms-player/src/gamepad.rs
- apps/rbms-player/src/gamepad/chord_tests.rs
- apps/rbms-player/src/keyconfig.rs
- apps/rbms-player/src/keyconfig_tests.rs
- apps/rbms-player/src/stage/keyconfig.rs
- apps/rbms-player/src/stage/keyconfig/chord_tests.rs
- apps/rbms-player/src/stage/keyconfig/tests.rs
- apps/rbms-player/src/stage/layout_pick.rs
- apps/rbms-player/src/stage/layout_pick/captured.rs
- apps/rbms-player/src/stage/layout_pick/tests.rs
- apps/rbms-player/src/stage/result.rs
- apps/rbms-player/src/stage/result/tests.rs
- apps/rbms-player/src/text/menus/key_editor.rs
- apps/rbms-player/src/text/menus/layout_pick.rs

## 요약

8개 항목을 모두 처리했습니다. 7개는 수정 전에 실패하는 시험으로 먼저 재현한 뒤 고쳤습니다. 4번은 결함이 코드 경로에서 드러나 있어 시험만 새로 붙였습니다. 8번은 실제로 레인이 죽는다는 것을 시험으로 확인했습니다. 그래서 테스트 웨이브가 ESC 에 넣은 '레인이 이긴다' 규칙을 모든 조작 키에 똑같이 적용했습니다. 이에 따라 옛 파일에 남은 같은 충돌도 레인이 이깁니다. 기존 규칙인 '옛 조작 키가 레인을 이긴다'와 그 시험 an_older_control_still_wins_over_a_lane_on_its_key 는 정책이 바뀐 것이어서 시험을 반대 내용으로 바꿨습니다. 스킨 결과 화면은 이제 RETRY·NEXT 를 AppShared::is_shortcut 으로 읽습니다. 그래서 조합을 이해하고, 조합을 완성한 누름은 단일 키 동작을 하지 않습니다(M12). 입력을 받기 시작하는 시점, 페이드, 패드 단축키(ask)는 바뀌지 않았습니다. 코스 결과 화면에는 같은 문제가 없었습니다. 스킨 파일은 고치지 않았습니다. 검사: rbms-player lib 3307 통과, clippy -D warnings 통과, fmt 통과.

## 항목별 결과

- [고침] 1. 스킨 결과 화면의 조합 RETRY·NEXT 와 조합 완성 누름의 단일 RETRY (M12): 수정 전 시험에서 RETRY=R, NEXT=LSHIFT+R 일 때 LSHIFT, R 순으로 누르면 sel 이 0 그대로였습니다(left 0, right 1). 다시 플레이가 실행된 것입니다. 원인은 stage/result.rs:1281 의 listen_for(shortcut_key(..)) 가 단일 키만 넘기고, Scene::key 가 키 코드만 비교한 것입니다. 수정으로 Scene 의 run_again_key·follow_on_key·listen_for 를 지웠습니다. 대신 Scene::shortcut_key(Shortcut, &KeyInput) 를 두었고, ResultState::handle_key 가 RETRY·NEXT 마다 ctx.shared.is_shortcut(shortcut, key.code) 로 물어 넘깁니다. 조합을 완성한 누름은 조합의 단축키로만 읽고, 조합을 깨는 뗌은 그 단축키의 뗌으로 읽습니다. Enter·Escape·레인 키 표·START+파란 키·패드 ask·입력 시작 시점·페이드는 그대로입니다. 코스 결과 화면은 SceneInput 의 can_run_again·can_follow_on 이 늘 false 라서 R·N 이 원래 아무것도 하지 않습니다. 같은 결함이 없어 고치지 않았습니다. 수정 뒤 이 화면은 R·N 에 대기 표시도 세우지 않는데, 동작은 같습니다. (시험: stage::result::tests::a_skins_scene_reads_the_run_again_keys_bound_to_two_keys_held_together (새 시험, 수정 전 실패). stage::result::tests::the_run_again_keys_are_read_from_where_the_key_configuration_has_them (장면 단위 부분을 스킨 화면 경로로 다시 씀). stage::result 55 통과)
- [고침] 2. LAYOUT PICK 이 조합 완성 누름을 레인·START·SELECT 로 읽음: 수정 전 시험에서 F7+(레인 5 키) 조합을 완성한 누름이 레인을 입력했습니다(left ["5123467"], right ["1234567"]). 수정으로 LayoutPickState::lane_of 와 menu_button_of 를 두었습니다. 둘 다 HeldKeys::press_made_a_chord 를 거치고, other_key 와 removes_a_capture 가 이 두 함수를 씁니다. 다른 화면의 lane_input_for·control_for 와 같은 조합 억제 조회입니다. 차트 모드의 레인 목록과 control_key 로 START·SELECT 를 직접 읽던 방식은 그대로 두었습니다. control_for 는 지금 모드(shared.mode)의 레인에 양보하므로 차트 모드가 다르면 결과가 달라지기 때문입니다. 레인 키·START·SELECT 를 단독으로 누르면 예전처럼 동작합니다. 기존 layout_pick 시험(행 입력, 목록, 시작, KEY 4 삭제)도 모두 통과합니다. (시험: stage::layout_pick::tests::the_press_that_makes_two_keys_held_together_writes_no_lane_and_is_no_start_or_select (새 시험, Stage::handle_key 경로, 수정 전 실패). layout_pick 26 통과)
- [고침] 3. capture 파일 쓰기 실패 뒤에도 '배치를 저장했습니다' 알림: 수정 전 시험의 알림이 두 줄이었습니다: 'captured layouts save failed (...): File exists (os error 17)' 와 'layout captured: RANDOM 3517246'. 수정으로 CapturedLayouts::save 가 bool 을 돌려주고, keep 은 쓰기에 실패하면 새 변형 Keeping::NotWritten 을 돌려줍니다. keep_captured_layout 은 NotWritten 이면 알림을 더 내지 않아서 실패 알림만 남습니다. 항목은 이번 실행 동안 메모리에 남습니다. 변형이 세 개가 되자 clippy enum_variant_names 가 AlreadyKept 에 걸렸습니다. 그래서 열거형 이름을 Kept 에서 Keeping 으로 바꿨습니다(쓰는 곳은 captured_layouts.rs 와 그 시험뿐). (시험: captured_layouts::tests::a_capture_whose_file_cannot_be_written_says_only_that_it_was_not_written (새 시험, 수정 전 실패). captured_layouts 10 통과)
- [고침] 4. CAPTURED 빈 목록 안내의 빈 키 이름 ('PRESS  ON A RESULT'): 수정 전 draw_list 는 shortcut_bind(CaptureLayout).map(label).unwrap_or_default() 로 빈 문자열을 CapturedNone 의 {0} 에 넣었습니다(코드 경로). 이 표현식은 테스트할 수 있는 함수 밖에 있어서 수정 전 실패를 시험으로 따로 보이지는 못했습니다. 수정으로 ListView.capture_key 를 Option<&str> 로 바꾸고 ListView::nothing_captured 를 두었습니다. 키가 없으면 새 문구 LayoutPickText::CapturedNoneWithoutKey 를 냅니다(EN 'NOTHING CAPTURED: GIVE CAPTURE LAYOUT A KEY IN KEY CONFIG' / KO '저장한 배치 없음: 키 설정에서 배치 저장 키를 지정하세요' / JA '保存した配置なし: キー設定で配置を保存のキーを指定してください'). 목록 폭이 328px 라서 stage::preview_menu::wrapped 로 최대 두 줄에 그립니다. 조합으로 묶으면 KeyBind::label 로 'LSHIFT + P' 가 됩니다. (시험: stage::layout_pick::tests::an_empty_list_names_the_key_that_captures_a_layout_or_where_to_give_it_one (새 시험): P, 'LSHIFT + P', 미지정일 때 세 언어 문구와 두 줄 안에 드는지 확인. text::menus::layout_pick::tests::every_entry_is_written_in_every_language 통과)
- [고침] 5. 턴테이블 축 방향이 조작 조합의 요소로 묶임: 수정 전 시험에서 PadConfig::bind_chord(ESC, Button 21, Axis 3 +) 가 묶였고, 같은 쌍을 담은 파일을 sanitise 로 읽어도 남았습니다. 수정은 세 가지입니다. 첫째, PadConfig::is_on_a_turntable 을 두었습니다. 어느 모드든 레인이 AnalogScratch 로 읽는 축이면 그 축과 그 방향이 해당합니다. 둘째, bind_chord 가 이것을 거절합니다. 셋째, sanitise(KeyConfig::load 와 설정 받기가 부름)가 forget_pairs_on_turntables 로 그런 조합의 두 요소를 모두 버립니다. 그 조작·단축키는 출하 상태로 돌아가서 START 는 STANDARD START 가 됩니다. 편집기 capture_elements 는 그 쌍을 묶지 않고 Notice::TurntableInChord 를 빨간 글자로 냅니다. 문구 KeyEditorNoticeText::TurntableInChord 는 EN 'A TURNTABLE CANNOT BE PART OF A COMBINATION - PICK BUTTONS' / KO '턴테이블은 조합에 넣을 수 없습니다 - 버튼을 고르세요' / JA 'ターンテーブルは組み合わせに使えません - ボタンを選んでください' 입니다. (시험: gamepad::chord_tests::a_direction_of_a_turntable_is_not_bound_into_a_pair, gamepad::chord_tests::a_pair_on_a_turntable_is_let_go_of_when_the_file_is_read (둘 다 수정 전 실패), stage::keyconfig::chord_tests::the_controllers_cell_refuses_a_pair_with_a_turntable_in_it)
- [고침] 6. 설정 동기화가 held 를 버리고 pad.sanitise 를 건너뜀: 수정 전 시험에서 받은 debounce_ms 60000 이 그대로 남았습니다(left 60000, right 100). 수정으로 KeyConfig::take_bindings_of(read) 를 두었습니다. 지금의 held 를 그대로 둔 채 바인딩만 바꾸고 pad.sanitise() 를 부릅니다(파일에서 읽을 때와 같은 정리). app_network.rs 의 apply_downloaded_settings 는 이제 이 함수를 씁니다. 시험에서는 ESC=RSHIFT+Q 를 쥔 채 설정을 받은 뒤 Q 를 떼면, chord_event 가 ESC 조합이 깨진 뗌(Escape 올라감)으로 나옵니다. (시험: app_network::tests::a_key_configuration_downloaded_is_read_as_its_file_is_and_keeps_the_keys_held (새 시험, 수정 전 실패))
- [고침] 7. 결과 화면 단축키를 레인 키와 같은 키에 묶으면 그 레인 동작도 실행되는데 편집기가 안내하지 않음: 기존 KeyConfig::lane_sharing 은 선곡(Browser) 단축키에만 안내를 냈습니다. 수정 전 시험에서 CAPTURE LAYOUT=Z(레인 1)인데도 상태 줄이 비어 있어 실패했습니다. 수정으로 lane_sharing 이 Result 단축키도 봅니다. 다만 그 모드의 스킨 결과 화면 키 표가 역할을 주는 레인만 셉니다(새 stage::result::reads_lane, key_index_of 와 key_assignment 사용). 그래서 7키 턴테이블처럼 결과 화면에서 아무것도 하지 않는 레인은 안내하지 않습니다. 거절하지 않는 기존 규칙(read_with)은 그대로입니다. 선곡 안내와 같은 방식(text_dim 색, 상태 줄)으로 새 문구 KeyEditorNoticeText::AlsoLaneOnResult 를 냅니다: EN 'ALSO {0} - ON A SKIN'S RESULT SCREEN BOTH ACT' / KO '{0}도 같은 키를 씁니다 - 스킨 결과 화면에서는 둘 다 동작합니다' / JA '{0}にも割り当て済みです - スキンのリザルト画面では両方が動作します'. 내장 결과 화면은 레인을 읽지 않으므로 '스킨' 이라고 밝혔습니다. (시험: stage::keyconfig::tests::the_status_line_names_the_lane_a_shortcut_of_the_result_shares_its_key_with (새 시험, 수정 전 실패). keyconfig_tests::a_browser_shortcut_on_a_lane_key_is_named_rather_than_refused 의 메시지만 고침)
- [고침] 8. ESC 외 조작 키가 다른 모드의 레인 키를 가져가 그 모드에서 레인을 죽임: 재현됐습니다. 7키 편집기의 clash 는 M 을 HI-SPEED UP 에 주는 것을 거절하지 않습니다. 그 상태로 14키에서 control_for(M) 이 Some(HiSpeedUp) 을 돌려줬습니다. 플레이의 handle_key 는 control_for 를 레인보다 먼저 보므로 M 레인이 죽습니다. 수정으로 ControlAction::yields_to_lanes 를 지우고, AppShared::control_bind_in_force 가 모든 조작의 단일 키를 지금 모드의 레인에 양보하게 했습니다. ESC 에 넣은 규칙과 같은 '레인이 이김'입니다. control_for, control_held, control_repeating, START·SELECT 조회, 키 가이드가 모두 이 함수를 따릅니다. 옛 키 설정 파일이 레인과 조작 키를 겹쳐 둔 경우도 레인이 이깁니다. 출하 조작 키(UP DOWN LEFT RIGHT [ ] A W \ 9 0)는 어느 모드의 출하 레인에도 없어서 기본 동작은 바뀌지 않습니다. (시험: app_input::tests::a_control_on_a_key_another_mode_plays_a_lane_on_gives_way_to_the_lane (새 시험, 수정 전 실패). app_input::tests::a_lane_keeps_the_key_it_shares_with_an_older_control (an_older_control_still_wins_over_a_lane_on_its_key 를 정책 변경에 맞게 반대 내용으로 바꿈). keyconfig_tests::start_and_select_are_the_controls_that_give_way_to_a_lane 은 메서드를 지워서 삭제함. rbms-player lib 전체 3307 통과)

## 그 밖에 찾은 것

- [medium, 고침] 정책 변경: 옛 조작 키가 레인을 이기던 규칙을 '모든 조작 키가 레인에 양보'로 바꿈: app_input.rs 의 기존 시험 an_older_control_still_wins_over_a_lane_on_its_key 가 '옛 조작 키가 레인을 이긴다'를 고정하고 있었습니다. 과제 8번의 지시(ESC 와 같은 규칙을 일관되게, 옛 파일은 레인이 이김)를 따라 이 규칙을 뒤집었고 시험도 반대 내용으로 바꿨습니다. 같은 모드에서 레인과 겹친 옛 파일은 이제 그 모드에서 조작 키가 듣지 않습니다. 편집기는 그 줄을 빨간 충돌(SharedWith)로 보여 줍니다.
- [low, 안 고침] 다른 모드의 레인 키를 조작 키에 줄 때 편집기가 안내하지 않음: 8번 수정으로 레인은 죽지 않습니다. 대신 7키 편집기에서 HI-SPEED UP 을 M 에 주면 14키에서는 HI-SPEED UP 이 조용히 듣지 않습니다. ESC 도 테스트 웨이브 수정 이후 같은 상태입니다. 모든 모드의 레인을 거절하거나 안내하는 기능은 범위를 넓히는 일이라 넣지 않았습니다.
- [low, 고침] keyconfig_tests.rs 를 고침 (범위 목록에 문자 그대로 없는 형제 파일): keyconfig_tests.rs 는 lib.rs 의 mod keyconfig_tests 로 들어가는 keyconfig 의 시험 파일입니다. yields_to_lanes 를 지우면 그 메서드를 고정하던 시험 start_and_select_are_the_controls_that_give_way_to_a_lane 을 지워야 해서 고쳤습니다. 한 시험의 메시지('R plays no lane')도 바꿨습니다. 메인에서 범위를 확인해 주시기 바랍니다.
- [low, 안 고침] 설정 동기화로 ESC 바인딩 자체가 바뀌면 옛 조합의 뗌은 Escape 로 가지 않음: take_bindings_of 는 held 를 유지합니다. 하지만 뗌이 깨는 조합은 새 설정의 chords() 로 계산합니다. 그래서 받은 설정이 ESC 를 다른 키로 묶으면, 쥐고 있던 옛 ESC 조합을 떼도 Escape 가 올라가지 않습니다. 같은 바인딩을 동기화하는 흔한 경우는 시험으로 확인했습니다.

## 남은 위험

- 8번 정책 변경으로 레인과 같은 키를 쓰는 조작 키는 그 모드에서 듣지 않습니다. 손으로 고친 옛 키 설정이나 다른 모드 편집기에서 준 키가 여기에 해당합니다. 편집기는 다른 모드 레인과 겹치는 것을 따로 알리지 않습니다.
- 턴테이블 판정은 모든 모드의 AnalogScratch 레인 축을 셉니다. 한 모드에서만 턴테이블로 쓰는 축도 조합 요소로 쓸 수 없고, 읽을 때 그런 조합은 두 요소 모두 버려져 출하 상태로 돌아갑니다.
- 새 CAPTURED 빈 목록 문구는 두 줄에 들어가는지를 시험 글꼴로 쟀습니다. 시험 글꼴에 한·일 글자가 없어 실제 폭은 실기로 확인해야 합니다.
- Kept 를 Keeping 으로 이름을 바꿨습니다. 다른 에이전트가 같은 열거형을 쓰는 코드를 동시에 넣으면 병합 때 충돌합니다(현재 쓰는 곳은 captured_layouts.rs 와 그 시험뿐).
- Scene::listen_for 를 지웠습니다. 다른 범위의 코드가 이것을 새로 부르면 컴파일이 깨집니다(현재 쓰는 곳은 없음).
- 워크스페이스 전체 시험과 3 OS CI 는 실행하지 않았습니다(메인 몫).

## 갱신이 필요한 문서

- docs/reference/2026-11-19-feature-upgrade-12/README.md 7절: 스킨 결과 장면이 RETRY·NEXT 를 AppShared::is_shortcut 으로 읽음(조합 이해, Scene::listen_for 삭제). LAYOUT PICK 의 조합 억제(lane_of, menu_button_of). 모든 조작 키가 지금 모드의 레인에 양보함(ControlAction::yields_to_lanes 삭제, control_bind_in_force). 턴테이블 축의 패드 조합 거절과 읽을 때 버림(PadConfig::is_on_a_turntable, sanitise). 설정 받기의 KeyConfig::take_bindings_of(held 유지, pad.sanitise)
- docs/play-options.md 조합 지정 규칙 절과 키 충돌 규칙: '옛 조작 키는 레인보다 먼저' 문구가 있으면 '모든 조작 키는 레인에 양보'로 고치기. 패드 조합에 턴테이블 축 불가. 결과 화면 단축키와 레인 키를 같은 키에 묶을 때의 편집기 안내
- docs/quality-assurance 테스트 부채: 고친 항목 지우기(스킨 결과 조합 RETRY/NEXT, LAYOUT PICK 조합 억제, 턴테이블 축 조합, 설정 동기화 held 초기화, CAPTURED 빈 목록의 빈 키 이름). 남은 항목 더하기: 다른 모드 레인 키를 준 조작이 그 모드에서 조용히 듣지 않음, CAPTURED 빈 목록 두 줄 문구의 한·일 실제 폭 실기 확인, 동기화로 ESC 바인딩이 바뀔 때 옛 조합의 뗌
- docs/reference/2026-10-24-i18n-glossary.md: 스킨 결과 화면 / スキンのリザルト画面(편집기 AlsoLaneOnResult), 턴테이블 / ターンテーブル(편집기 TurntableInChord, 기존 PadAnalogLabel 과 같은 말), 키 설정 / キー設定 + 배치 저장 / 配置を保存(LAYOUT PICK CapturedNoneWithoutKey)
- docs/plan/2026-11-19-feature-upgrade-12.md 결과 절: U3·U7 의 남은 발견 8건 처리 결과와 8번 정책 변경(레인이 이김) 기록, Kept 를 Keeping 으로 바꾼 사실

## 돌린 검사

- `cargo test -p rbms-player --locked --lib a_skins_scene_reads_the_run_again_keys (수정 전 원본 result.rs)` → FAILED: 'the press that made NEXT SONG ran this chart again' left 0 right 1 (결함 재현)
- `cargo test -p rbms-player --locked --lib the_press_that_makes_two_keys_held_together_writes_no_lane (수정 전)` → FAILED: left ["5123467"] right ["1234567"] (결함 재현)
- `cargo test -p rbms-player --locked --lib a_capture_whose_file_cannot_be_written (수정 전)` → FAILED: 알림 2건(save failed + layout captured) (결함 재현)
- `cargo test -p rbms-player --locked --lib a_control_on_a_key_another_mode_plays_a_lane_on (수정 전)` → FAILED: control_for(M) left Some(HiSpeedUp) right None (14키 레인이 죽는 것을 확인)
- `cargo test -p rbms-player --locked --lib the_status_line_names_the_lane_a_shortcut_of_the_result (수정 전)` → FAILED: 상태 줄에 레인 안내 없음 (결함 재현)
- `cargo test -p rbms-player --locked --lib -- gamepad::chord_tests::a_direction_of_a_turntable gamepad::chord_tests::a_pair_on_a_turntable (수정 전, is_on_a_turntable 단언 한 줄을 잠시 뺀 상태)` → 2 FAILED: 턴테이블 축 조합이 묶이고, 파일을 읽은 뒤에도 남음 (결함 재현)
- `cargo test -p rbms-player --locked --lib a_key_configuration_downloaded (수정 전)` → FAILED: debounce_ms left 60000 right 100 (결함 재현)
- `cargo test -p rbms-player --locked --lib` → 3307 passed; 0 failed; 2 ignored (수정 뒤)
- `cargo clippy -p rbms-player --all-targets --locked -- -D warnings` → 통과(처음에는 enum_variant_names 로 실패해 Kept 를 Keeping 으로 바꾼 뒤 통과)
- `cargo fmt --all --check` → 통과
- `skin-dump` → 실행하지 않음: 스킨 파일을 고치지 않음
- `rbms-player 실기·패드 실장치` → 실행하지 않음: 헤드리스 시험으로만 확인
