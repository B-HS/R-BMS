# O3 테스트 웨이브 보고 (wf_3df278ff-bb7)

> 테스트 웨이브의 에이전트가 끝낼 때 낸 보고 원문이다. 메인이 병합하면서 바꾼 것은 계획서 9절에 있다.

- 상태: done

## 바꾼 파일

- apps/rbms-player/src/captured_layouts.rs
- apps/rbms-player/src/captured_layouts/tests.rs
- apps/rbms-player/src/stage/layout_pick.rs
- apps/rbms-player/src/stage/layout_pick/tests.rs
- apps/rbms-player/src/stage/result/tests.rs

## 요약

U7 범위에서 반례를 찾아 결함 3건을 재현하고 원인을 고쳤으며, 각 결함에 시험을 남겼습니다. 가장 중요하다고 한 충실도는 결함이 없었습니다. 실제 세션으로 돌려 보는 시험을 새로 만들었고 모두 통과했습니다.

1) 키보드에서 KEY 4 레인 키를 두 번 눌러도 저장분이 지워지지 않았습니다.
- 원인: `handle_key` 가 Delete 가 아닌 모든 키에서 삭제 대기를 먼저 풀었습니다. 그래서 두 번째 누름이 항상 다시 첫 누름이 됐습니다. 패드는 이 경로를 거치지 않아 정상이었고, 기존 시험도 패드만 다뤘습니다.
- 수정: `removes_a_capture`(Delete, 또는 목록에 있을 때 각 사이드 4번째 레인 키)와 `place_of` 를 두었습니다. 이 두 키만 삭제 대기를 유지합니다.

2) 같은 배치가 목록에 여러 번 저장됐습니다.
- 원인: 중복 판정이 (md5, 옵션, 시드)만 봤습니다. MIRROR·ROTATE 는 시드와 무관하게 같은 배치이고, R-RANDOM(7키)은 시프트가 6가지뿐입니다. 그런데 판마다 시드가 새로 정해지므로, MIRROR 판을 두 번 저장하면 똑같은 'MIRROR 7654321' 이 두 줄 생겼습니다. 중복 알림 문구도 '이미 저장한 배치'입니다.
- 수정: `same_layout` 이 시드가 같거나, 적힌 레인 순서가 같으면 같은 배치로 봅니다(md5·옵션은 계속 비교). 레인 순서가 비어 있는 손편집 항목은 예전처럼 시드로만 구별합니다. 계획 M15 의 '같은 (md5, 옵션, 시드)는 한 번만'보다 조금 넓은 규칙이니 메인의 확인이 필요합니다.

3) UTF-8 이 아닌 파일이나 권한 오류로 읽지 못한 파일을 알림 없이 빈 목록으로 열었습니다. 이어서 다음 저장이 그 파일을 덮어썼습니다. 문서 주석('one that cannot be read is reported')과 어긋납니다.
- 수정: 파일이 없을 때(NotFound)만 조용히 비우고, 다른 읽기 오류는 `NotLoaded` 로 알립니다.

충실도 시험: 5·7·9·10·14키 × RANDOM·R-RANDOM·MIRROR·ROTATE 20가지 조합을 같은 경로로 돌렸습니다.
- 실제 적재 경로(`layout_to_load` + `lay_out_notes`)로 배치해 끝까지 치고, `enter_result` 로 결과 화면에 들어갑니다.
- 옵션 패널을 흉내 내 설정 배치를 OFF 로 바꾼 뒤 P 를 누릅니다.
- 저장된 레인 순서를 노트를 추적해 얻은 실제 배치와 비교했습니다.
- LAYOUT PICK 미리보기의 배치가 그 판과 같은지 봤습니다.
- START 다음 적재가 같은 시드·같은 배치인지, 그 판을 다시 저장하면 같은 항목인지 확인했습니다.
- 모두 통과해 결함이 없습니다. 빌린 배치로 친 판과 같은 배치 재시도도 이 시험이 지나갑니다.

결함이 아닌 것으로 확인한 가설은 criteria 에 하나씩 적었습니다.

## 기준별 판정

- [pass] 충실도: 모드별(5·7·9·10·14), 옵션별(RANDOM·R-RANDOM·MIRROR·ROTATE)로 저장한 레인 순서가 그 판의 실제 배치와 같다: 새 시험 a_capture_is_the_layout_the_run_was_played_on_in_every_mode_and_arrangement: bmson 차트의 레인마다 노트 하나를 두고 실제 적재 함수로 배치해 play_through, enter_result, P 순으로 진행. 원래 시각별 레인과 배치 뒤 레인으로 순열을 만들어 layout_of_permutation 결과와 capture.layout 이 같음을 20가지 조합 모두에서 확인했습니다.
- [pass] 결과 화면에서 옵션 패널로 배치를 바꾼 뒤 P 를 눌러도 저장은 판의 것: 같은 시험에서 enter_result 뒤 config.play.random 을 OFF 로 바꾸고 P 를 눌렀는데 저장 항목의 옵션은 판의 옵션이었습니다. capture 는 ResultRun::of 에서 미리 만들어지고, ResultRun::of 를 부르는 곳은 app_result.rs:210 하나뿐입니다.
- [pass] 빌린 배치로 친 판, 같은 배치 재시도의 시드: 같은 시험에서 LAYOUT PICK START 다음 적재의 시드가 저장 시드와 같고 배치도 같으며, 다시 저장하면 기존 항목 그대로였습니다. 기록과 전송의 옵션 이름은 저장과 같은 곳(config.play.random, app_result.rs:395)에서 오고, lay_out_as_borrowed 가 그 값을 빌린 옵션으로 둡니다.
- [pass] LAYOUT PICK 미리보기 배치가 실제 판과 같다: 같은 시험에서 previewed()(새 #[cfg(test)] 접근자) 노트의 위치가 판과 같음을 20가지 조합에서 확인했습니다.
- [pass] 미리보기 속도 식이 PlaySample, 실제 플레이와 같다(HI-SPEED 고정, GREEN 고정, CONSTANT, 커버, 리프트, 히든): 코드 대조 결과입니다. layout_pick.rs shown_hispeed 는 skin_preview.rs:511 과 같은 식(BpmStats::held + held_speed(HeldTravel::of))이고, 실제 플레이 hold_travel_at 도 같은 held_speed 를 씁니다. 내장 필드는 플레이에서도 config.play.hispeed 와 constant 플래그로 그리므로 SPEED FIX 만 켜진 경우에도 같습니다. 리프트는 rebuild_skin 처럼 effective_lift, 커버·히든은 effective_cover·effective_hidden 을 씁니다. 예외는 PLAYBACK RATE 로, findings 에 적었습니다.
- [pass] 미리보기 시계 경계(dt 0·음수·NaN·아주 큼, End, Page, 끝에서 처음): clock.rs 를 읽고 확인했습니다. dt.max(0.0) 은 NaN 을 0 으로 만들고, 아주 큰 값은 as i64 포화 뒤 saturating_add 를 거쳐 0 으로 돌아갑니다. End 는 i64::MAX 를 saturating 으로 마지막 마디에 묶습니다. 기존 시험 the_preview_plays_on_the_screens_clock_and_begins_again_at_the_end 와 the_preview_clock_counts_frames_and_measures 가 통과했습니다.
- [pass] 저장소: 파일 없음, 손상, 읽을 수 없는 파일: 수정 전 새 시험 a_file_that_is_no_text_is_said_not_to_be_read 가 실패했습니다(알림 0건). 수정 뒤 통과하며, NotFound 는 여전히 알림이 없습니다.
- [pass] 중복 판정(대소문자, 시드와 무관한 같은 배치), 두 번 연속 P: 수정 전 새 시험 a_layout_kept_already_is_kept_once_whatever_seed_laid_it_out 가 실패했습니다(MIRROR 시드 2 가 Added). 수정 뒤 통과합니다. 대소문자와 두 번 연속 P 는 기존 시험(a_capture_is_kept_once…, the_capture_key_keeps_the_runs_layout_once)이 통과했습니다.
- [pass] 목록: Delete 확인 뒤 다른 키로 취소, 레인 키로 삭제: 수정 전 새 시험 the_fourth_lane_key_pressed_twice_on_the_keyboard_removes_a_capture 가 실패했습니다(left 2, right 1). 수정 뒤 통과하며, 기존 Delete 와 패드 시험도 통과합니다.
- [pass] 거절: S/H-RANDOM, ALL-SCRATCH, OFF, 오토플레이, 리플레이, 코스·연습, 24키: 기존 시험 a_run_that_is_no_lane_order_of_the_players_is_not_captured 와 the_capture_key_says_why_a_run_is_not_captured 가 통과했습니다. show_result 를 읽어 보니 연습은 결과 화면 전에 Back 으로, 코스는 course 경로로 빠지므로 Course 거절은 실제로 닿지 않는 방어 코드입니다(무해).
- [pass] 중도 포기한 판과 실패한 판도 저장 가능(모든 판 저장 결정과 일치): 코드로 확인했습니다. enter_result_of_a_left_run 도 같은 show_result 를 거쳐 ResultRun::of 로 가고, 실패나 포기를 거절하는 조건은 없습니다. 실행 시험은 만들지 않았습니다.
- [pass] 알림 세 언어: text/menus/layout_pick.rs 의 every_entry_is_written_in_every_language(catalog_problems 가 LayoutPickText 와 CaptureText 를 검사)가 통과했습니다. 새 문구는 추가하지 않았습니다.
- [pass] 목록 0개, 삭제 뒤 커서 범위, 범위 전환, 다른 차트로 이동, 같은 md5 가 여러 폴더에 있음: captured.rs 를 읽고 확인했습니다. selected·cursor 는 len-1 로 묶고, press_remove 뒤 step_list(0) 으로 다시 맞춥니다. 0개이면 selected 가 None 이라 Enter 는 행 시작으로 갑니다. go_to_chart 는 라이브러리의 첫 일치 항목으로 갑니다. 기존 시험 a_capture_of_another_chart_goes_to_that_chart_or_says_it_is_missing 가 통과했습니다.
- [pass] 시작: AUTOPLAY, 탐색 중 목록 이동(탐색 취소), 고쳐 쓰면 탐색 복귀: show_capture 다음 relay_out 이 cancel 을 걸고 SeedLook::Captured 가 됩니다. not_playable 은 AutoplayOn 을 먼저 봅니다. 기존 시험 a_captured_layout_written_over_is_played_as_the_rows_are 가 통과했습니다.
- [pass] 결과 화면 P 가 스킨 씬을 건드리지 않고, 패드로도 동작: 기존 시험 the_capture_key_acts_at_once_on_a_skins_scene_and_from_a_controller 가 통과했습니다. 조합(chord)으로 묶은 경우는 shortcut_for 가 chord_event 를 먼저 보는 공용 경로라 따로 시험하지 않았습니다(untested).
- [untested] P 를 레인 키로 다시 지정한 경우: 재현 시험은 만들지 않았습니다. 결과 화면 스킨 씬은 held_key_indices 로 레인 키를 폴링하므로(result.rs:262) 그 키의 확정·재시도도 함께 동작할 가능성이 큽니다. 충돌 규칙은 keyconfig 의 Bindable 충돌 로직(범위 밖)에 있어 고치지 않았습니다.

## 찾은 것

- [medium, 고침] 키보드 KEY 4 레인 키 두 번 눌러 삭제가 동작하지 않음: layout_pick.rs handle_key 가 key.code != Delete 이면 먼저 list.disarm() 을 해서, 두 번째 KEY 4 누름이 항상 첫 누름(arm)이 됐습니다. 안내 문구(HintCaptured)와 보고서는 'KEY 4 두 번 삭제'라고 말합니다. 새 시험이 수정 전에 실패했습니다(left 2, right 1). (시험: stage::layout_pick::tests::the_fourth_lane_key_pressed_twice_on_the_keyboard_removes_a_capture)
- [medium, 고침] 같은 배치가 시드만 달라 중복 저장됨(MIRROR, ROTATE, R-RANDOM): same_layout 이 seed 까지 같아야 같은 항목으로 봤습니다. MIRROR·ROTATE 는 lane_permutation 에서 시드를 쓰지 않고, R-RANDOM 7키는 시프트가 6가지라 시드 1과 3이 같은 배치입니다. 판마다 시드가 새로 정해지므로 같은 'MIRROR 7654321' 이 여러 줄 쌓였습니다. 수정: 시드가 같거나 적힌 레인 순서(layout)가 같으면 같은 항목입니다. 계획 M15 의 키보다 넓은 규칙이라 메인 확인이 필요합니다. (시험: captured_layouts::tests::a_layout_kept_already_is_kept_once_whatever_seed_laid_it_out)
- [low, 고침] UTF-8 이 아니거나 권한 오류로 읽지 못한 저장 파일을 알림 없이 비우고 다음 저장에서 덮어씀: CapturedLayouts::load 가 read_to_string 의 모든 Err 를 '파일 없음'처럼 다뤘습니다. 새 시험(0xFF 0xFE 바이트 파일)이 수정 전에 알림 0건으로 실패했습니다. 이제 NotFound 만 조용히 넘어가고 나머지는 NotLoaded 로 알립니다. favorites.rs 도 같은 패턴이지만 범위 밖이라 고치지 않았습니다. (시험: captured_layouts::tests::a_file_that_is_no_text_is_said_not_to_be_read)
- [low, 안 고침] 미리보기가 PLAYBACK RATE 를 반영하지 않음(의심, 계획 M13 목록 밖): 실제 플레이는 GREEN 고정 속도를 practice_clock().heard_bpm(bpm) 으로 계산하고(stage/play/mod.rs:574), 판을 config.play.playback_percent 속도로 진행합니다. 미리보기 PreviewClock 은 dt 를 1배로 더하고 held_speed 를 차트 BPM 으로 계산합니다. PlaySample 도 같습니다. PLAYBACK RATE 가 100% 가 아니면 한 화면에 보이는 차트 길이나 실제 시간당 스크롤 속도가 플레이와 다릅니다. 시험은 만들지 않았습니다. 계획에 없는 요구라 고치지 않았습니다. (시험: 없음(코드 대조))
- [low, 안 고침] 캡처 키를 레인 키로 다시 지정하면 결과 화면 스킨 씬이 그 레인 동작도 실행(보고된 위험, 재현 안 함): result.rs handle_key 는 CaptureLayout 을 처리하고 Stay 로 돌아가지만, 스킨 씬은 SceneInput::read 의 held_key_indices(shared.key_index_pressed)로 레인을 폴링합니다. 그래서 키 이벤트를 막아도 확정·재시도가 일어날 수 있습니다. 충돌 거절이나 안내는 keyconfig 의 Bindable 충돌 규칙(범위 밖) 몫입니다. (시험: 없음(의심))
- [low, 안 고침] 저장 파일 쓰기 실패 시에도 '배치를 저장했습니다' 알림이 뒤따름: keep() 안의 save() 가 SaveFailedAt(Error)을 알린 뒤 keep_captured_layout 이 Captured(Info)를 알립니다. 항목은 이번 실행 동안 메모리에만 있습니다. 동작은 맞지만 문구 순서 때문에 오해할 수 있습니다. 고치지 않았습니다. (시험: 없음(코드 읽기))
- [low, 안 고침] 목록에서 다른 차트로 옮겨 간 뒤 Esc 로 돌아가면 선곡 커서는 원래 차트에 있음(관찰): go_to_chart 는 LayoutPickState 만 새로 열고 shared.sel 을 바꾸지 않습니다. 시작과 재시도는 chart_index 와 chart_path 를 쓰므로 판에는 문제가 없습니다. 계획이 이 동작을 정하지 않아 결함으로 보지 않았습니다. 패드 레인은 ctx.shared.mode == self.mode 일 때만 읽으므로, 모드가 다른 차트로 옮겨 가면 패드로 목록을 조작할 수 없습니다(기존 조건). (시험: 없음(관찰))

## 남은 위험

- 중복 판정 규칙을 (md5, 옵션, 시드)에서 (md5, 옵션, 시드 또는 레인 순서)로 넓혔습니다. 계획 M15 의 문구와 다르므로 메인이 확인하고 계획서에 반영해야 합니다. 수정 전에 이미 쌓인 중복 항목은 하나를 지우면 같은 배치가 모두 함께 지워집니다.
- 워크스페이스 전체 시험과 3 OS CI 는 실행하지 않았습니다(메인 몫).
- 스킨 파일은 고치지 않아 skin-dump 는 돌리지 않았습니다.
- 실제 창으로 rbms-player 를 띄워 보지는 않았습니다. 확인은 헤드리스 시험으로만 했습니다.

## 갱신이 필요한 문서

- docs/plan/2026-11-19-feature-upgrade-12.md: M15 중복 규칙을 '같은 (md5, 옵션) 에서 시드 또는 레인 순서가 같으면 한 번만'으로 정정, 테스트 웨이브 결과에 U7 결함 3건
- docs/reference/2026-11-19-feature-upgrade-12/README.md: captured_layouts 의 중복 규칙과 읽기 오류 알림, LAYOUT PICK 의 KEY 4 삭제 키 처리(removes_a_capture)
- docs/quality-assurance: 테스트 부채로 캡처 키를 레인 키로 다시 지정한 경우, PLAYBACK RATE 가 100% 가 아닐 때의 미리보기 속도, 조합(chord)으로 묶은 캡처 키

## 돌린 검사

- `cargo test -p rbms-player --locked --lib the_fourth_lane_key_pressed_twice (수정 전)` → FAILED: left 2, right 1 (결함 재현)
- `cargo test -p rbms-player --locked --lib captured_layouts (수정 전)` → 2 failed: a_file_that_is_no_text_is_said_not_to_be_read(알림 0건), a_layout_kept_already_is_kept_once_whatever_seed_laid_it_out(MIRROR 시드 2 가 Added) (결함 재현)
- `cargo test -p rbms-player --locked --lib -- a_capture_is_the_layout_the_run_was_played_on captured_layouts the_fourth_lane_key (수정 후)` → 10 passed, 0 failed
- `cargo test -p rbms-player --locked --lib -- layout_pick captured_layouts stage::result:: borrowed_layout keyconfig` → 220 passed, 0 failed
- `cargo clippy -p rbms-player --all-targets --locked -- -D warnings` → 통과(경고 0, rbms-player 재검사 확인)
- `cargo fmt --all --check` → 통과
