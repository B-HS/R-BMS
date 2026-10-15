# 空POOR 가 많은 판이 점수 서버에서 400 으로 거절됨

최종 갱신 2026-10-15 · 대응 커밋 ea698a9 (수정 커밋 `865643f`(서버), 동기화 `5f4a9a2`)

## 대상 파일

- `web/src/server/service/domain/score/score.service.ts` (`judgedNoteCount`, `isJudgeCountConsistent`), `web/src/server/route/score.ts`
- 진단: `crates/rbms-ir/src/error.rs`(오류 세부 표시), `web/src/server/lib/api-response.ts`(`publicErrorDetails`)

## 증상

시드 수정(`7095ef7`) 뒤에도 결과 화면에 `IR ERROR: server error 400: 요청 형식이 올바르지 않습니다. (VALIDATION_ERROR)`. 사용자 표현으로 "거의 다 저장이 안 된다". 사용자의 판이 모두 SCRATCH AUTO 라 SCRATCH AUTO 문제로 보였다.

## 원인

서버 판정 수 = `pg+gr+gd+bd+max(poor-empty_poor,0)+miss`. 클라이언트는 `poor`=見逃し POOR, `miss`=`empty_poor`=空POOR 를 보내므로 서버 값은 `pg+gr+gd+bd+max(見逃し, 空POOR)` 가 되어, 끝까지 친 판에서 空POOR 가 見逃し POOR 보다 많으면 `total_notes` 를 넘었다. SCRATCH AUTO 와는 무관하다. 사용자 로컬 52판 중 37판이 해당했고, 운영 순위에 남은 판·빠진 판이 예측과 맞았다(예: #52 total 1835, pg+gr+gd+bd 1815, poor 20, 空POOR 43 → 옛 식 1858). 운영 서버가 오류 `details` 를 버리고 클라이언트도 코드만 보여서 원인 필드가 화면에 나오지 않았다.

## 해결

- 판정 수는 노트를 소비한 판정만 센다(9K 는 BAD 제외): [ADR](../acknowledge/2026-10-14-adr-judge-count-consumed-notes.md).
- 프로덕션 오류 본문에 문자열 `field`·`reason` 을 싣고, 결과 화면이 `VALIDATION_ERROR: judge (…)` 처럼 보인다.
- 이미 거절된 판은 기록 동기화로 다시 보낸다: [ADR](../acknowledge/2026-10-14-adr-record-sync.md).
- 같은 종류의 사고를 막는 계약 교차 테스트(`contract/client-requests/`, `web/tests/dto/client-requests.test.ts`).

## 남은 일

- 해소(2026-10-15): 옛 판이 운영에 올라가는지. 서버 수정은 배포됐고(마이그레이션 `0004_course_score_judge_totals` 는 2026-10-15 02:57 UTC 에 적용), 같은 날 12:15 사고로 실행된 개발 빌드의 기록 동기화가 이 사용자의 기록 DB 에서 판 49개를 운영에 보냈고 모두 accepted 였다(리플레이 6개 업로드, 43개 대기). 이 사용자의 로컬 52판 중 37판이 옛 식으로는 거절될 판이었으므로(위 원인), 수리가 운영에서 동작한다는 간접 증거다. 49개 가운데 空POOR 판이 몇 개인지는 세지 않았다. 그 사고는 [피드백](../feedback/2026-10-15-run-the-player-with-a-scratch-home.md)에 있다.
- 해소(2026-10-15): 운영의 `chart.notes` 가 잘못 심겼는지. `web-db` 의 `chart-notes-count`(run 38673771025)가 받아들인 판과 다른 행 0, 복구 대상 0 이었다([테스트 부채](../quality-assurance/2026-10-14-test-debt.md) 3.36). 누적 거절 제출은 39건이다(거절 이유는 감사 표가 남기지 않아 분류하지 못했다).
- 남음: 게시된 v0.1.4(2026-10-15T07:27:28Z)로 새 판을 쳐서 결과 화면에 `IR: RANK` 가 나오는지 사람이 본다(QA 항목 W3 → X3, [실제 창 QA](../quality-assurance/2026-10-11-default-skin-manual-check.md) 0.1절). 43개 대기 리플레이가 다음 동기화에서 올라가는지도 같이 본다(3.19절 R1).
