# ADR: 점수 서버가 거절하면 결과 화면에 오류 코드와 필드·이유를 보인다

> 최종 갱신 2026-10-15 · 대응 커밋 ea698a9
> 상태: 유효 · 결정일 2026-10-14 · 결정자 사용자(S18 결정 2, 추천안 A) · 커밋 `865643f`(서버), `5f4a9a2`(클라이언트)

## 배경

점수 제출 400 사고([판정 수 ADR](2026-10-14-adr-judge-count-consumed-notes.md))에서 결과 화면은 `IR ERROR: server error 400: 요청 형식이 올바르지 않습니다. (VALIDATION_ERROR)` 만 보였다. 어느 필드가 왜 거절됐는지 알 수 없어 원인을 찾는 데 운영 DB 와 로컬 기록을 대조해야 했다. 서버는 운영에서 오류 응답의 `details` 를 내보내지 않았다.

## 결정

1. 서버는 운영에서도 오류 `details` 가운데 문자열인 `field` 와 `reason` 두 키만 내보낸다(`publicErrorDetails`, `PRODUCTION_DETAIL_KEYS`). 나머지(Zod issues, 경로, 플레이어 id)는 운영에서 뺀다. 비운영은 그대로 낸다.
2. 클라이언트는 실패 봉투에 필드가 있으면 `<코드>: <필드> (<이유>)`(이유가 없으면 `<코드>: <필드>`)로, 없으면 지금처럼 `<메시지> (<코드>)` 로 한 줄을 만든다(`envelope_text`, `ServerReason`). 결과 화면의 `IR ERROR:` 줄이 이 문장이다. 예: `IR ERROR: server error 400: VALIDATION_ERROR: judge (judge count e...`(결과 줄 길이로 잘림).
3. 기록 동기화는 같은 코드·필드·이유를 판마다 기록 DB(`ir_sync.error_code`·`error_field`·`error_reason`)에 적고, 기록 상세의 `IR:` 줄에 보인다([기록 동기화 ADR](2026-10-14-adr-record-sync.md)).

## 이유

- 사용자가 거절 원인을 바로 보고 알릴 수 있다. 필드 이름과 고정된 이유 문구는 비밀이 아니다. 입력을 되돌려 보일 수 있는 나머지는 운영에서 숨긴다(보안 규칙의 `details` 비노출).

## 기각된 대안

| 대안 | 기각 이유 |
| --- | --- |
| 지금처럼 메시지와 코드만 보인다 | 거절 원인을 알 수 없다. 사용자가 A 를 택함 |
| 운영에서도 `details` 전체를 낸다 | Zod issues·경로가 입력을 되돌려 보일 수 있다 |

## 영향과 관련 파일

- 서버: `web/src/server/lib/api-response.ts`(`PRODUCTION_DETAIL_KEYS`, `publicErrorDetails`, `errorResponse`).
- 클라이언트: `crates/rbms-ir/src/error.rs`(`ServerReason`, `ErrorBody::server_reason`, `IrError::server_reason`, `envelope_text`), `apps/rbms-player/src/ir_outcome.rs`(`format_submit_outcome`).
- 테스트: `error.rs` 의 `a_validation_envelope_names_its_code_field_and_reason`·`an_envelope_with_a_field_only_names_the_code_and_field`·`an_envelope_without_details_keeps_the_message_and_code`·`server_reason_reads_code_field_and_reason_from_the_envelope`, `ir_outcome.rs` 의 `a_validation_error_names_its_field_on_the_result_line`.

## 날짜와 사용자 원문

2026-10-14. S18 결정 묶음 2번(추천안 A: 결과 화면에 오류 필드 표시) — "나머지는 다 추천안대로 가면 돼".
