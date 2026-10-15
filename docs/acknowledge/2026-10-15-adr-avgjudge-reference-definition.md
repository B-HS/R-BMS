# ADR: avgjudge 는 원본 정의(평균 절대 오차, 놓친 노트 1초)로, 치우침은 따로

> 최종 갱신 2026-10-15 · 대응 커밋 ea698a9
> 상태: 유효 · 결정일 2026-10-15 · 결정자 사용자(감사 결정 1, 추천안) · 커밋 `f81d499`

## 배경

감사 REC-1·IR-3·PLAY-7·PERF-12. `avgjudge` 의 뜻이 세 곳에서 달랐다.

- 결과 화면 제출은 엔진의 `avg_judge_us`(맞은 입력만의 부호 있는 평균)를 보냈다(예 -14655).
- 기록 DB 는 그 절댓값을 저장했다(14655). 기록 동기화가 다시 만든 본문도 그 값을 보냈다. 같은 판이 경로에 따라 -14655 와 14655 로 서버에 들어갔다.
- 원본은 노트마다 `state 1..4` 면 `|시간차|`, 아니면 1,000,000 µs 를 더해 노트 수로 나눈다(`BMSPlayer.java:911-930`). `IRScoreData.java:174` 가 그 값을 보낸다.
- 부호 있는 평균은 빠름·늦음이 상쇄된다. 사용자 DB 사본 71판 중 7판이 1 ms 미만이었고(1500노트에 29 µs), `merge_best` 의 최솟값 경쟁이 그 판을 최고 기록으로 남겼다.

원본 정의의 값은 `PlayRecord::avg_duration_us` 로 이미 계산되고 있었지만 결과 스킨에서만 쓰였다.

## 결정

1. 기록 DB(`scorelog.avgjudge`)와 제출 본문(`judge.avgjudge`)은 원본 정의의 값(`PlayRecord::avg_duration_us`)이다. 결과 화면 제출, 기록 DB, 다시 만든 본문이 같은 판에 같은 값을 낸다.
2. 부호 있는 평균(타이밍 치우침, 빠르면 양수)은 제출 본문의 `extra.mean_offset_us` 로 따로 싣는다. 서버는 `extra` 를 그대로 저장하므로 서버 스키마는 바꾸지 않는다.
3. 제출 본문의 `extra.rule_version` 에 규칙 세대를 싣는다(이번에 4). 서버는 이것으로 `avgjudge` 의 뜻을 가른다. 세대가 없는 본문(옛 클라이언트)은 부호 있는 평균, 4 이상은 원본 정의다.
4. 규칙 세대 `SCORE_RULE_VERSION` 을 4 로 올린다(`REFERENCE_RECORD_RULE_VERSION`). 같은 릴리스의 엔진 변경(입력 시각 판정과 게이지가 빈 순간의 판정 정지, 롱노트 몸통 정리와 짝 없는 머리 제거로 바뀐 노트 수)도 세대 4 의 정의에 든다([입력 시각 ADR](2026-10-15-adr-event-time-judging.md), [몸통 노트 ADR](2026-10-15-adr-long-note-body-notes.md)). 옛 행은 이렇게 다룬다.
   - 세대 3 이하 행의 `avgjudge` 는 이력으로만 남고 최고 기록(`score.avgjudge`) 경쟁에 들어가지 않는다.
   - DB 를 열 때마다(`ScoreDb::migrate`) `score.avgjudge` 를 세대 4 이상의 최고 기록 자격 판에서 다시 접는다(`refold_reference_bests`). 옛 빌드가 그 사이에 옛 값을 접어 넣었어도 다음 실행에서 바로잡힌다. 자격 판이 없는 차트는 `UNSET_MINIMUM` 이다.
   - 옛 행을 다시 보낼 때(기록 동기화의 다시 만든 본문)는 `avgjudge` 를 0(미상)으로 보내고 `extra.rule_version` 에 그 행의 세대를 싣는다. 원래 값은 되살릴 수 없다.
5. 판정할 노트가 없는 차트(`AVG_DURATION_ABSENT_US`)는 본문에 0, DB 에 `UNSET_MINIMUM` 으로 둔다.

## 이유

- 원본·원본 IR 과 같은 값이 되고, 작을수록 정확하다는 최고 기록 비교가 뜻을 가진다.
- 치우침은 오프셋 조정에 쓸모가 있어 버리지 않고 `extra` 로 남긴다.
- 옛 값은 되살릴 수 없으므로 섞이지 않게 세대로 가르는 것이 유일하게 안전하다.

## 기각된 대안

| 대안 | 기각 이유 |
| --- | --- |
| 와이어는 부호 있는 평균, DB 는 절댓값 그대로 | 같은 판이 경로에 따라 두 값이 되고, 상쇄된 판이 최고 기록이 된다 |
| DB 에 부호 있는 평균을 저장 | 최솟값 비교에서 가장 빠른 판이 최고가 된다 |
| 서버에 세대 열을 새로 만든다 | `extra` 를 이미 저장하므로 마이그레이션 없이 같은 정보를 남길 수 있다. 서버가 이 값을 비교·표시하는 곳도 없다 |
| 옛 행을 리플레이로 재계산 | 리플레이가 없는 행이 많고, 판정 규칙도 바뀌어 같은 판이 재현되지 않는다 |

## 영향과 관련 파일

- **로컬.** `crates/rbms-store/src/score.rs`(`SCORE_RULE_VERSION` 4, `REFERENCE_RECORD_RULE_VERSION`), `crates/rbms-store/src/scoredb.rs`(`merge_best`, `refold_reference_bests`), `scoredb/schema.rs`(열 때마다 다시 접기), `apps/rbms-player/src/app_result.rs`(`record_run`, `wire_avgjudge`), `scoredb_store.rs`(`FinishedPlay::avg_duration_us`), `record_sync.rs`(`rebuilt_avgjudge`).
- **와이어.** `crates/rbms-ir/src/dto/score.rs`(`MEAN_OFFSET_EXTRA_KEY`, `RULE_VERSION_EXTRA_KEY`), `dto/chart.rs`(`JudgeBreakdown` 문서). 계약 파일 `contract/client-requests/score-*.json` 갱신. 서버 코드는 바꾸지 않는다(`extra` 저장).
- **서버의 옛 행.** 이미 서버에 있는 행의 `avgjudge` 는 부호 있는 평균(재전송된 것은 절댓값)이다. `extra.rule_version` 이 없는 행은 그 뜻으로 읽는다.
- **엔진.** `JudgeEngine::avg_judge_us` 의 문서 주석(원본 avgjudge 라고 잘못 적었던 것)을 고쳤다. 값과 쓰임(치우침)은 그대로다.
- **테스트.** `app_result/record_tests.rs` `a_stored_run_keeps_the_reference_average_duration`, `scoredb/tests.rs` `an_old_generation_avgjudge_does_not_beat_a_reference_one`·`an_old_avgjudge_folded_in_by_an_older_build_is_taken_out_on_the_next_open`, `dto/score/tests.rs` `the_mean_offset_and_the_rule_version_ride_in_extra`.

## 날짜와 사용자 원문

2026-10-15. 결정 묶음에 "전부 추천안대로"(1번: "avgjudge = 원본 정의(절대 오차 평균, 놓친 노트 1초 벌점), 부호 있는 평균은 별도 필드(와이어는 extra 등 서버 스키마가 이미 받는 곳)").
