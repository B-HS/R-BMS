# F3 수리 웨이브 보고 (wf_08935ad2-8ca)

> 수리 웨이브의 에이전트가 끝낼 때 낸 보고 원문이다.

- 상태: done

## 바꾼 파일

- /Users/hyunseokbyun/development/R-BMS/.claude/worktrees/wf_08935ad2-8ca-3/apps/rbms-player/src/app_result.rs
- /Users/hyunseokbyun/development/R-BMS/.claude/worktrees/wf_08935ad2-8ca-3/apps/rbms-player/src/stage/play/mod.rs

## 요약

빌린 배치로 친 판을 START(새 배치)로 재시도하면 그 판이 사용자 자신의 배치로 기록되던 결함을 고쳤습니다.

원인: `PlayState::retry` 가 `app_result::retry` 를 먼저 불렀습니다. 그 안에서 `hand_borrowed_layout_on` → `let_go_of_borrowed_layout` → `put_back_own_play_options` 가 `config.play.random` 을 사용자 값으로 되돌렸고, 그다음에야 `record_retried_run` 이 그 설정을 읽어 DB 행·리플레이·제출 본문·트레이스를 썼습니다.

수정: `app_result.rs` 에 `retry_failed_run(state, shared, same_layout) -> Option<Transition>` 을 새로 두었습니다. 순서는 다음과 같습니다.
1. 라이브러리에 없는 곡이면 `None` 을 돌려줍니다. 이때는 아무것도 기록하지 않고, 판은 결과 화면에서 기록됩니다.
2. `record_retried_run` 으로 판을 기록합니다. 이 시점에는 설정에 아직 그 판의 배치가 들어 있습니다.
3. `retry_seed` 를 넣고 `retry` 를 불러 빌린 배치를 넘기거나 놓습니다.
4. `save_settings` 를 부릅니다.

`PlayState::retry` 는 이 함수를 부르는 한 줄로 줄였습니다. 이로써 기록이 배치 되돌리기보다 먼저 일어납니다. 제출 본문·리플레이 형식과 `SCORE_RULE_VERSION` 은 바꾸지 않았고, 모든 판 저장·전송 정책과 빌린 배치 정책(같은 배치 재시도에서만 유지)도 그대로입니다.

판의 배치 옵션을 PlayState 에 들고 다니게 하는 방식은 택하지 않았습니다. 판이 만들어지는 곳이 범위 밖인 `app_play.rs` 의 `PendingChart`/`into_play` 이고, 그 방식을 쓰면 47곳의 `PlayState::new` 호출도 바꿔야 하기 때문입니다. 대신 기록 시점과 표시 시점이 같은 출처(로드 때 `layout_to_load` 가 넣은 설정)를 읽는다는 불변식이 모든 기록 입구에서 지켜지는지 확인했습니다. 표시용 `ensure_setup` 도 이 값을 읽고, 이번 수정으로 재시도 기록도 같은 값을 읽습니다.

기록 입구별로 각 값을 어디서 읽는지 정리한 표입니다. 기록은 모두 `record_run` 을 거칩니다.
- 배치 옵션(행·본문·리플레이·트레이스): `config.play.random`
- 시드: `session.seed()`
- 게이지(행·본문): `record.finished_gauge`. 리플레이의 gauge 만 `config.play.gauge`
- 어시스트: 세션의 `has_auto_played_notes` 와 `judge_setup`. 리플레이의 scratch_auto 만 설정값
- LN MODE: `state.ln_mode_key`/`lntype`(로드 시점)
- 배속·커버·리프트·오프셋: 판이 끝날 때의 설정. 플레이 중 키로 바뀐 마지막 값을 남기는 것이 의도된 동작입니다.

입구별 확인 결과:
- `show_result`(끝까지 친 판, Esc, 남기고 나간 판): 기록 전에 설정을 되돌리는 곳이 없습니다. `leave_run` 의 `save_settings` 는 설정을 바꾸지 않습니다.
- `keep_run_cut_short_by_quitting`: `save_settings` 뒤에 바로 기록하므로 문제없습니다.
- 코스 스테이지: 코스 제한값이 `end_course` 전까지 설정에 남아 있고, 로드도 그 값으로 펼치므로 일치합니다. 재시도는 코스에서 막혀 있습니다.
- 기록 동기화의 재전송: 저장된 본문이나 DB 행에서 다시 만들며 플레이 설정을 읽지 않습니다.
- 결과 화면의 capture(`ResultRun::of`): `show_result` 안에서 기록 직후에 만들어지고, 그때 설정에는 아직 그 판의 배치가 있습니다. 시험으로 확인해서 `stage/result.rs` 는 고치지 않았습니다.

이미 잘못 기록된 행: 고치지 않았습니다. `PlayLog` 에는 빌린 배치였는지, 재시도였는지를 남기는 칸이 없습니다. "같은 곡·같은 시드·다른 배치 옵션"인 행은 정상인 경우와 구별되지 않습니다. 예를 들어 결과 화면에서 옵션을 바꾼 뒤 SELECT 로 재시도하면 정당하게 그런 행이 생깁니다. 서버 리플레이에서 빌린 배치는 출처 행이 로컬에 없기도 합니다. 확실한 근거가 되려면 리플레이 입력을 배치 옵션 후보 9가지로 다시 판정해 저장된 판정 수와 정확히 맞는 것을 찾아야 하는데, 이런 도구는 아직 없습니다. 리플레이를 저장하지 않은 행은 그 방법으로도 알 수 없습니다. 서버에 이미 보낸 제출도 함께 틀려 있습니다.

## 항목별 결과

- [고침] 빌린 배치 판을 START 로 재시도하면 DB 행·리플레이·제출 본문이 사용자 배치 옵션 + 빌린 시드로 기록됨: 수정 전 새 시험이 실패했습니다: `same layout false: the row's arrangement left: 1 right: 4`(MIRROR 로 기록, 기대값 R-RANDOM). 기록을 `retry` 앞으로 옮긴 뒤 통과합니다. 같은 시험이 START 와 SELECT 양쪽에서 DB 행의 random·seed, 보낼 본문의 options.random·seed, 저장된 리플레이의 random 이름·seed 를 확인하고, 그 리플레이를 다시 펼친 레인 순서가 친 판과 같은지까지 확인합니다. (시험: app_result::tests::a_run_on_a_borrowed_layout_started_again_is_recorded_with_the_layout_it_was_played_on)
- [결함 아님] 결과 화면 capture(ResultRun::of 의 config.play.random)가 빌린 배치 판, 같은 배치 재시도, 새 배치 재시도 뒤에 옳은지: capture 는 show_result 안에서 기록 직후, 배치가 되돌려지기 전에 만들어집니다. 시험 결과: 빌린 판은 R-RANDOM 과 REPLAY_SEED, 결과 화면에서 같은 배치로 재시도한 판도 R-RANDOM 과 REPLAY_SEED, 새 배치로 재시도한 판은 MIRROR 와 그 판 자신의 시드로 잡혔습니다. 시험은 처음부터 통과했습니다. (시험: app_result::tests::the_result_screen_captures_the_layout_its_run_was_played_on)
- [재현 안 됨] 다른 기록 입구(record_run/show_result, keep_run_cut_short_by_quitting, 코스 스테이지, record_sync 재전송)가 판이 끝난 뒤 바뀌는 설정을 읽는지: 코드로 경로를 확인했습니다. 이 입구들은 기록 전에 put_back_own_play_options, let_go_of_borrowed_layout, end_replay, end_course 를 부르지 않습니다. leave_play 경로는 기록하지 않는 판에서만 쓰입니다. record_sync 는 저장된 본문이나 DB 행에서 만듭니다. 기존 시험도 통과했습니다: stage::play::screen_tests::a_failed_run_started_again_is_kept_first, app_result::record_tests::left_behind::quitting_in_the_middle_of_a_run_keeps_it_as_a_failed_run_and_sends_nothing 등. (시험: cargo test -p rbms-player --locked -- app_result:: borrowed_layout:: stage::play:: stage::result:: (425 passed))
- [미룸] 이미 잘못 기록된 로컬 DB 행 고치기: PlayLog 에 빌린 배치나 재시도를 표시하는 칸이 없습니다. 같은 곡·같은 시드에 배치 옵션이 다른 행은 결과 화면에서 옵션을 바꾼 뒤 SELECT 로 재시도한 정상 행과 구별되지 않습니다. 확실히 가려내려면 리플레이 입력을 배치 옵션 후보마다 다시 판정해 저장된 판정 수와 비교해야 하는데 그런 도구가 없습니다. 추측으로는 행을 고치지 않았습니다. (시험: 없음(판단만 함))

## 그 밖에 찾은 것

- [medium, 고침] 빌린 배치 판의 START 재시도가 사용자 배치 옵션으로 기록·전송됨(PlayState::retry 호출 순서): stage/play/mod.rs 의 PlayState::retry 가 app_result::retry 를 부른 뒤에 record_retried_run 을 불렀습니다. retry → hand_borrowed_layout_on → let_go_of_borrowed_layout → put_back_own_play_options 가 config.play.random 을 되돌렸습니다. 새 시험이 수정 전 1(MIRROR)≠4(R-RANDOM)로 실패했습니다.
- [low, 안 고침] 의심: 설정 내려받기 응답이 판 도중에 도착하면 config 전체가 바뀌어 기록 값이 판과 어긋날 수 있음: app_network.rs:398-416 의 apply_downloaded_settings 가 `self.config = merged` 로 설정 전체를 바꿉니다. 응답은 매 프레임 폴링(app_network.rs:563)됩니다. 설정 화면에서 내려받기를 시작한 뒤 응답이 오기 전에 판을 시작하면 판 도중에 play.random 이 바뀔 수 있습니다. 이 경우 빌린 배치의 own_play_options 와도 꼬일 수 있습니다. 재현하지 않았고, 범위 밖(app_network.rs)이라 고치지 않았습니다. 근본 해법은 판의 배치 옵션을 PendingChart→PlayState 로 들고 다니는 것이고, 이를 위해 app_play.rs 를 고쳐야 합니다.
- [low, 안 고침] 참고: 코스 결과의 스킨 입력이 config.play.random 을 읽음: stage/course_result/skin_run.rs:45 가 `random_option_index(shared.config.play.random)` 를 읽습니다. finish_course 는 end_course 전에 불리므로 지금은 코스 제한값과 일치하는 것으로 보입니다. 결함으로 확인하지 않았고, 범위 밖이라 손대지 않았습니다.

## 남은 위험

- 판의 배치 옵션을 판과 함께 들고 다니지 않고 기록 순서만 고쳤습니다. 로드 뒤 기록 전에 config.play.random 을 바꾸는 새 경로가 생기면 같은 종류의 결함이 다시 생깁니다. 지금 알려진 후보는 판 도중에 도착하는 설정 내려받기 응답 하나입니다(의심, 재현 안 함).
- 이미 잘못 기록되어 서버에 보낸 판(사용자 배치 옵션 + 빌린 시드)은 그대로 남아 있습니다. 로컬 행과 서버 기록 모두 고치지 않았습니다.
- 새 시험 the_result_screen_captures_the_layout_its_run_was_played_on 은 결과 화면의 capture 키 처리(stage/result.rs 의 handle_key)를 지납니다. 그 부분을 다른 에이전트가 고치고 있으므로 병합 뒤 다시 확인해야 합니다.
- 워크스페이스 전체 시험은 실행하지 않았습니다(메인 담당).

## 갱신이 필요한 문서

- docs/bug: 빌린 배치 판의 START 재시도가 사용자 배치로 기록되던 결함. 증상, 원인(PlayState::retry 에서 기록이 retry 보다 늦게 일어나던 순서), 해결(app_result::retry_failed_run 이 먼저 기록), 이미 기록된 행은 식별 근거가 없어 고치지 않았다는 판단
- docs/history/2026-11-19-feature-upgrade-unit-reports/test-O4.md 의 '범위 밖, 안 고침' 항목을 고침으로 갱신하거나 후속 보고에서 언급
- docs/quality-assurance: 테스트 부채. 판 도중 설정 내려받기 응답이 도착하는 경우(app_network apply_downloaded_settings)의 기록 값 어긋남은 시험하지 않았음
- docs/reference 의 기록 값 출처 절이 있으면 추가: 기록 입구별로 배치 옵션·시드·게이지·어시스트·LN MODE·배속을 어디서 읽는지 정리한 표

## 돌린 검사

- `cargo test -p rbms-player --locked app_result::tests:: (수정 전, 재현)` → 18개 중 1개 실패: a_run_on_a_borrowed_layout_started_again_is_recorded_with_the_layout_it_was_played_on, the row's arrangement left 1 right 4
- `cargo test -p rbms-player --locked -- app_result:: borrowed_layout:: stage::play:: stage::result::` → 425 passed, 0 failed, 1 ignored
- `cargo test -p rbms-player --locked -- app_result:: borrowed_layout:: stage::play::screen_tests (fmt 후 다시 실행)` → 178 passed, 0 failed, 1 ignored
- `cargo clippy -p rbms-player --all-targets --locked -- -D warnings` → 통과(처음에 쓰지 않는 import Bindable 1건을 지운 뒤 다시 실행)
- `cargo fmt --all --check` → 통과(rustfmt 로 새 시험 두 줄을 감싼 뒤 다시 실행)
