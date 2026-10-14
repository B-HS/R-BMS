# ADR: TOTAL 덮어쓰기는 어시스트(넓힌 판정과 같은 처리)

> 상태: 유효 · 결정일 2026-10-15 · 결정자 사용자(감사 결정 3, 추천안) · 커밋 미정(메인이 채운다)

## 배경

감사 PLAY-4. 설정 JUDGE 탭의 TOTAL(`config.play.total_override`)은 원본에 없는 rbms 확장이다. 값이 0 보다 크면 차트의 `#TOTAL` 을 덮어써 판정마다 게이지가 움직이는 폭을 바꾼다. 그런데 다음 문제가 있었다.

- 어시스트로 보지 않았다. `is_custom_judge` 와 `updates_score` 가 TOTAL 을 보지 않아 램프가 강등되지 않고 최고 기록이 갱신됐다. 재현: 같은 입력이 `#TOTAL 100` 에서 FAILED 게이지 22, TOTAL 5000 에서 NORMAL 게이지 88.
- 리플레이에 TOTAL 이 없어, 리플레이를 볼 때 지금 설정의 TOTAL 이 들어갔다. TOTAL 로 클리어한 판이 TOTAL 0 으로 보면 FAILED 로 재생됐다.

## 결정

1. TOTAL 덮어쓰기(> 0)로 친 판은 넓힌 판정 판과 같다(`judged_wider`): 어시스트 단계 2, 램프는 ASSIST EASY 이하, 저장·리플레이·전송은 하고 최고 기록·순위에서 뺀다. 제출 본문은 `options.total_override` 를 그대로 싣고 어시스트 태그 `CUSTOM_JUDGE` 를 단다. 서버는 `total_override > 0` 을 `TOTAL_OVERRIDE`, 태그를 `ASSIST` 로 표지해 unranked 로 둔다.
2. 그런 스테이지가 낀 코스도 넓힌 판정 스테이지와 같이 다룬다(코스 본문의 `CUSTOM_JUDGE` 태그로 unranked, 코스 최고 기록 없음).
3. 리플레이는 `ReplayJudge.total_override` 에 TOTAL 을 기록하고, 볼 때는 설정이 아니라 기록값을 쓴다(`run_total_override`). 이 필드가 없는 옛 리플레이는 차트 TOTAL 로 재생한다.
4. 와이어 리플레이(`ReplayData`)에는 TOTAL 자리가 없으므로 TOTAL 을 덮어쓴 판의 리플레이는 업로드하지 않는다(`wire_carries_judge`, 넓힌 LN MARGIN 과 같은 처리). 로컬 파일로는 그대로 재생된다.
5. 결과 화면은 "OWN TOTAL: SENT AND KEPT, BUT NO BEST SCORE" 로 알린다.

## 이유

- TOTAL 은 게이지를 원하는 만큼 움직여 램프를 바꾸므로, 판정 폭을 넓힌 것과 같은 성격의 완화다. 사용자 결정 "넓힌 판정과 같은 처리".
- 리플레이가 판을 재현하려면 그 판을 정한 설정을 담아야 한다.

## 기각된 대안

| 대안 | 기각 이유 |
| --- | --- |
| 어시스트로 보지 않는다(지금) | TOTAL 로 클리어한 판이 일반 판과 같은 최고 기록·램프가 된다 |
| 저장·전송에서 빼기 | 모든 판 저장·전송 원칙([ADR](2026-10-14-adr-save-and-send-every-played-run.md))과 어긋난다 |
| 별도 어시스트 태그 `TOTAL_OVERRIDE` 를 새로 둔다 | 서버가 이미 `total_override` 로 표지하고, 코스는 태그로만 알릴 수 있어 기존 `CUSTOM_JUDGE` 로 충분하다 |

## 영향과 관련 파일

- `apps/rbms-player/src/lib.rs`(`run_total_override`, `overrides_total`, `judged_wider`), `app_result.rs`(`record_run`), `app_play.rs`(모델 TOTAL 에 기록값 사용), `ir_replay.rs`(`wire_carries_judge`), `stage/result.rs`(`OWN_TOTAL_NOTE`, 결과 화면의 최고 기록 여부), `crates/rbms-store/src/replay.rs`(`ReplayJudge.total_override`, `REPLAY_CHART_TOTAL`).
- 옛 기록: 세대 3 이하 행 중 TOTAL 로 친 판은 다른 판과 구분할 수 없어 그대로 둔다(규칙 세대 4 의 문서에 적음). 기록 동기화가 다시 만든 본문은 TOTAL 을 모르므로 `total_override` 0 을 보낸다. 세대 4 행은 본문을 함께 저장하므로 다시 만들 일이 없다.
- 테스트: `app_result/record_tests.rs` `a_run_with_a_total_override_is_assisted_and_never_a_best`, `ir_replay.rs` `only_a_judge_the_wire_can_state_is_uploaded`, `crates/rbms-store/src/tests.rs`(리플레이 왕복과 옛 파일 기본값), `stage/result/tests.rs`(알림 문구).

## 날짜와 사용자 원문

2026-10-15. 결정 묶음에 "전부 추천안대로"(3번: "TOTAL 덮어쓰기(>0)는 어시스트: 저장·전송은 하고 최고 기록·순위에서 제외(넓힌 판정과 같은 처리), 리플레이에 TOTAL 기록·재생 시 기록값").
