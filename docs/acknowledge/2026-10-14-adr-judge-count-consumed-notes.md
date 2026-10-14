# ADR: 점수 제출의 판정 수 검사는 노트를 소비한 판정만 센다

> 상태: 유효 · 결정일 2026-10-14 · 결정자 사용자(추천안 A) · 커밋 `865643f`

## 배경

v0.1.3 까지의 판 대부분이 결과 화면에 `IR ERROR: server error 400: … (VALIDATION_ERROR)` 를 보이며 서버에 저장되지 않았다. 서버의 판정 수 검사(`judgedNoteCount`)가 `pgreat+great+good+bad+max(poor-empty_poor,0)+miss` 였다. 클라이언트는 `poor` 에 見逃し POOR(노트 소비), `miss` 와 `empty_poor` 에 空POOR(노트 소비 안 함)를 보낸다. 그래서 空POOR 가 見逃し POOR 보다 많은 완주 판이 `judge count exceeds total_notes` 로 거절됐다(사용자 로컬 52판 중 37판, 운영 순위와 대조해 확인). 기록: [../bug/2026-10-14-score-submit-400-empty-poor.md](../bug/2026-10-14-score-submit-400-empty-poor.md).

## 결정

1. 판정 수 = PGREAT + GREAT + GOOD + BAD + POOR. `miss`·`empty_poor` 는 세지 않는다.
2. `POPN_9K` 는 BAD 도 세지 않는다(원본 `JudgeProperty.PMS` 의 `judgeVanish` 가 BAD 와 空POOR 만 거짓, rbms `crates/rbms-judge/src/windows.rs` 같음). `mode` 는 대소문자와 `-`/`_` 차이를 무시한다.
3. 판정 수가 `total_notes` 를 넘는 판은 지금처럼 거절한다(검사를 없애지 않는다).
4. 고치는 쪽은 서버뿐이다. 클라이언트 본문은 원본과 같은 의미라 옛 클라이언트(v0.1.0~v0.1.3)도 그대로 받아진다.

## 기각된 대안

| 대안 | 기각 이유 |
| --- | --- |
| 검사 자체를 없앤다(B) | 과다 판정 본문을 막는 최소한의 일관성 검사가 사라진다 |
| 클라이언트가 `miss` 를 0 으로 보낸다 | 원본 IR 의 ems/lms 의미와 어긋나고, 이미 나간 빌드는 계속 거절된다 |

## 영향과 관련 파일

`web/src/server/service/domain/score/score.service.ts`(`judgedNoteCount`, `isJudgeCountConsistent`), `web/src/server/route/score.ts`, 테스트 `web/tests/service/score-submit.test.ts`(사용자 판 #52 의 값), 계약 파일 `contract/client-requests/score-empty-poor-exceeds-poor.json`.

## 날짜와 사용자 원문

2026-10-14. 결정 묶음에 "나머지는 다 추천안대로 가면 돼"(1번 A). 보고 원문: "내가 간헐적으로 저장이안된다했지만 거의 다 저장이안돼 저장된것도 거의 우연인듯해".
