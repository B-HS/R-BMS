# ADR: 코스 곡별 기록은 스테이지 콤보로

> 상태: 유효 · 결정일 2026-10-15 · 결정자 사용자(감사 결정 5, 추천안) · 커밋 미정(메인이 채운다)

## 배경

감사 PLAY-6. 코스 2스테이지부터 엔진의 `combo`·`max_combo` 가 이어받은 콤보로 시작해(`Player::set_initial_state`), 그 스테이지의 곡별 기록(`PlaySummary::max_combo`)에 앞 스테이지의 연속이 섞였다. `max_combo` 가 노트 수를 넘고, FC 조건(`max_combo == total_notes`)이 성립하지 않아 풀콤해도 FC·PERFECT·MAX 가 남지 않았다(재현: 이어받은 100, 32노트 전부 PG → combo 132, 램프 NORMAL). 원본은 스테이지 `combo` 와 `coursecombo`·`coursemaxcombo` 를 따로 둔다(`JudgeManager.java:200-206, 668-676`).

## 결정

1. 엔진에 스테이지 콤보(`stage_combo`, `stage_max_combo`)를 따로 둔다. 플레이어가 보는 콤보(`combo`, `max_combo`)는 지금처럼 코스를 이어간다(원본 `coursecombo`).
2. 곡별 요약의 `max_combo` 와 곡의 풀콤 판정은 스테이지 콤보로 한다. 다음 스테이지로 넘기는 콤보(`standing_combo`)와 코스 합계의 최대 콤보(`standing_max_combo`)는 코스 콤보다.
3. 코스 자체의 램프는 스테이지 램프가 아니라 코스로 정한다(`PlaySession::course_lamp`, `MusicResult.java:402-428`). 코스의 최장 연속이 지금까지의 모든 노트면 풀콤 계열(MAX·PERFECT 는 코스 판정 수로), 아니면 게이지 램프, 게이지가 비면 FAILED 다. 코스 어시스트로 강등한다. 스테이지 램프를 그대로 코스에 쓰면 앞 스테이지가 끊긴 코스가 마지막 스테이지 MAX 로 MAX 가 되기 때문이다.

## 이유

- 원본 정의. 곡별 기록이 그 곡의 플레이를 말하게 한다.
- 코스 램프는 이 변경으로 바로 잘못될 수 있어(스테이지가 FC 를 낼 수 있게 되었으므로) 같은 결정 안에서 원본 규칙으로 맞췄다.

## 기각된 대안

| 대안 | 기각 이유 |
| --- | --- |
| `combo` 를 스테이지 콤보로 바꾸고 코스 콤보를 새 필드로 | HUD·스킨·리플레이 추적이 모두 `combo` 를 화면 콤보로 읽어 바꿀 곳이 많다. 원본 화면도 코스 콤보를 보인다 |
| 코스 램프를 마지막 스테이지 램프로 둔다(지금) | 앞 스테이지가 끊겨도 코스가 MAX 가 된다 |

## 영향과 관련 파일

- `crates/rbms-judge/src/matcher.rs`(`stage_combo`, `stage_max_combo`), `crates/rbms-play/src/lib.rs`(`set_initial_state` 문서), `session.rs`(`summary`, `standing_max_combo`, `course_lamp`), `apps/rbms-player/src/app_result.rs`(`fold_course_stage`, `course_lamp`, `CarriedCombo`).
- 남은 차이: 원본은 코스 스테이지의 곡별 기록 램프를 FC 계열이 아니면 기록하지 않는다(`BMSPlayer.java:863-880` 의 `clear` 기본값 Failed, `MusicResult.java:368-371` 의 NoPlay). rbms 는 게이지 램프를 남긴다. 이 결정은 콤보만 다루므로 `reference-divergences.md` 에 차이로 적는다.
- 계약 `contract/client-requests/course-normal.json` 의 `clear` 가 Normal → Max(두 스테이지 모두 PG 로 끊김 없음, 원본과 같음).
- 옛 기록: 세대 3 이하 코스 스테이지 행의 `combo` 는 이어받은 값이 섞여 있을 수 있고 고칠 정보가 없어 그대로 둔다.
- 테스트: `session.rs` `a_course_stage_reports_its_own_combo`·`a_course_is_a_full_combo_only_while_its_streak_is_unbroken`·`a_course_session_keeps_its_gauge_and_combo_after_judge_setup`(뜻이 바뀌어 고침), `app_result/record_tests.rs` `a_course_stage_records_its_own_combo_and_full_combo`.

## 날짜와 사용자 원문

2026-10-15. 결정 묶음에 "전부 추천안대로"(5번: "코스 곡별 기록은 곡별(스테이지) 콤보로 max_combo·FC 판정").
