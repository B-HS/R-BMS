# 빌린 배치로 친 판을 재시도하면 사용자 배치 옵션으로 기록되던 문제

최종 갱신 2026-11-19 · 대응 커밋 `864ef42`

## 대상 파일

- `apps/rbms-player/src/app_result.rs`(`retry_failed_run`, `record_retried_run`, `retry`)
- `apps/rbms-player/src/stage/play/mod.rs`(`PlayState::retry`)
- `apps/rbms-player/src/borrowed_layout.rs`(`hand_borrowed_layout_on`, `let_go_of_borrowed_layout`, `put_back_own_play_options`)

## 증상

빌린 배치(LAYOUT PICK 으로 고른 배치, 리플레이 배치, 저장한 배치)로 친 판이 끝나기 전에 START 로 재시도(새 배치)하면, 그 판의 DB 행·리플레이·서버 제출 본문·trace 가 빌린 배치의 옵션이 아니라 사용자 자신의 배치 옵션 이름 + 빌린 시드로 기록됐다. 그 리플레이는 친 배치와 다른 배치로 펼쳐진다. 재현(테스트 웨이브 O4): 사용자 MIRROR, 빌린 R-RANDOM, 끝까지 친 뒤 `state.retry(&mut shared, false)` → `db.all_plays()[0].random` 이 MIRROR.

이번 작업(2026-11-19 기능 고도화) 이전부터 있던 결함이다. 테스트 웨이브가 플레이 화면의 옵션 표시를 검증하다 찾았고, 사용자 지시("이것도 그럼 지금 수정인가 ? 남은것도 다 잘 처리하고")로 고쳤다.

## 원인

`PlayState::retry` 가 `app_result::retry` 를 먼저 불렀다. 그 안에서 `hand_borrowed_layout_on` 이 시드가 다르면 `let_go_of_borrowed_layout` → `put_back_own_play_options` 로 `config.play.random` 을 사용자 값으로 되돌렸고, 그다음에야 `record_retried_run` 이 그 설정을 읽어 기록했다. 기록이 "그 판이 쳐진 값"이 아니라 "되돌린 뒤의 설정"을 읽은 것이다.

## 해결

`app_result::retry_failed_run(state, shared, same_layout)` 을 두고 순서를 바꿨다: 판을 기록(`record_retried_run`, 이 시점의 설정에는 아직 그 판의 배치가 있다) → `retry_seed` 를 넣고 `retry`(빌린 배치를 넘기거나 놓음) → `save_settings`. 제출 본문·리플레이 형식·`SCORE_RULE_VERSION` 은 그대로다.

다른 기록 입구도 같은 문제가 있는지 확인했다(끝까지 친 판·Esc·남기고 나간 판의 `show_result`, `keep_run_cut_short_by_quitting`, 코스 스테이지, 기록 동기화의 재전송, 결과 화면의 capture). 기록 전에 설정을 되돌리는 곳이 없었다.

## 남은 것

- 수리 전에 이렇게 기록된 행은 고치지 않았다. 판 이력에 빌린 배치나 재시도를 표시하는 칸이 없어, 옵션을 바꾸고 재시도한 정상 판과 구별할 근거가 없다(테스트 부채 3.185, 미확인 51).
- 의심(재현 안 함): 설정 내려받기 응답이 판 도중에 도착하면 설정 전체가 바뀌어 기록 값이 판과 어긋날 수 있다.

## 시험

`cargo test -p rbms-player --locked -- app_result:: borrowed_layout:: stage::play:: stage::result::`(수리 웨이브 F3: 425 통과). 새 시험은 DB 행의 배치 옵션, 저장된 리플레이의 옵션 이름과 시드, 제출 본문을 본다.
