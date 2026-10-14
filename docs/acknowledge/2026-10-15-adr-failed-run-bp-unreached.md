# ADR: 게이지 사망 판의 BP 에 도달하지 못한 노트를 넣는다

> 상태: 유효 · 결정일 2026-10-15 · 결정자 사용자(감사 결정 4, 추천안) · 커밋 미정(메인이 채운다)

## 배경

감사 PLAY-5. 기록 DB 의 `minbp` 와 제출 본문의 `minbp` 는 `PlaySummary::min_bp`(BAD+POOR+空POOR)였다. 원본은 여기에 `notes - pastNotes`(도달하지 못한 노트)를 더한다(`BMSPlayer.java:909`). 게이지가 비면 판이 바로 끝나므로, HARD 로 시작하자마자 죽은 판이 BP 14(원본 정의 31)로 저장됐다. 최고 MISS COUNT 는 최솟값이라 일찍 죽을수록 좋은 기록이 됐다. Esc 포기 판만 `left_unfinished` 로 보정되고 있었다.

## 결정

1. 기록하고 보내는 BP 는 `PlaySummary::min_bp_with_unreached`(BAD+POOR+空POOR+도달하지 못한 노트)다. 정의는 이 함수 한 곳이고 `PlayRecord::min_bp_with_unreached` 도 이를 부른다. Esc 포기 판의 요약(`left_unfinished`)은 남은 노트를 이미 POOR 로 셌으므로 더 더하지 않는다.
2. 진행 중 화면이 읽는 `PlaySummary::min_bp` 는 바꾸지 않는다(미래 노트가 섞이지 않게).
3. 규칙 세대 4 부터 행에 이 값이 저장된다. 세대 3 이하 행은 판정 수에서 도달하지 못한 노트를 계산해 더한 값으로 비교한다(`charged_minbp`: 노트 수 − PGREAT·GREAT·GOOD·POOR·BAD 합, pop'n 은 BAD 를 빼고 센다). 최고 기록 접기(`merge_best`)와 DB 를 열 때의 다시 접기(`refold_reference_bests`), 다시 만든 본문의 `minbp` 가 같은 값을 쓴다. 끝까지 친 판과 Esc 포기 판은 0 을 더한다.

## 이유

- 원본 정의. 일찍 죽은 판이 최고 MISS COUNT 가 되는 역전을 없앤다.
- 옛 행은 판정 수와 노트 수가 남아 있어 같은 값을 계산할 수 있다.

## 기각된 대안

| 대안 | 기각 이유 |
| --- | --- |
| `PlaySummary::min_bp` 자체를 바꾼다 | 진행 중 HUD·스킨이 읽는 값에 미래 노트가 섞인다 |
| 옛 행은 그대로 둔다 | 옛 실패 판이 새 판보다 낮은 BP 로 최고 기록을 계속 쥔다 |

## 영향과 관련 파일

- `crates/rbms-play/src/session.rs`(`PlaySummary::min_bp_with_unreached`), `record.rs`, `apps/rbms-player/src/app_result.rs`(`record_run` 의 `minbp`), `scoredb_store.rs`(`play_log`), `record_sync.rs`(`rebuilt_submission`), `crates/rbms-store/src/scoredb.rs`(`charged_minbp`, `merge_best`, `refold_reference_bests`).
- 계약 `contract/client-requests/score-failed.json` 의 `minbp` 7 → 27.
- 코스: 코스 합계의 BP 는 스테이지 판정 수의 합이라, 게이지 사망 스테이지의 도달하지 못한 노트와 아직 안 친 스테이지의 노트(원본 `MusicResult.java:430-436`)가 들어가지 않는다. 이 결정의 범위 밖이라 테스트 부채로 남긴다.
- 테스트: `app_result/record_tests.rs` `a_run_that_fails_is_stored_with_the_score_it_reached`, `session.rs` `a_failed_run_counts_unreached_notes_in_min_bp`, `scoredb/tests.rs` `an_old_failed_play_is_charged_for_the_notes_it_never_reached`·`an_old_popn_play_is_not_charged_for_its_bads`, `scoredb_store/tests.rs` `a_run_that_did_not_reach_every_note_is_charged_for_the_rest`.

## 날짜와 사용자 원문

2026-10-15. 결정 묶음에 "전부 추천안대로"(4번: "게이지 사망 판의 BP 에 미도달 노트 포함(원본)").
