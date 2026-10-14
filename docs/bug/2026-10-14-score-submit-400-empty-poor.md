# 空POOR 가 많은 판이 점수 서버에서 400 으로 거절됨

최종 갱신 2026-10-14 · 수정 커밋 `865643f`(서버), 동기화 `5f4a9a2`

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

- 운영 배포 뒤 실제 판의 `IR: RANK` 확인, v0.1.4 의 동기화로 옛 판이 올라가는지 확인.
