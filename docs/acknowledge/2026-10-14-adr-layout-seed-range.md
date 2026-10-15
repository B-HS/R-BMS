# ADR: 배치 시드는 클라이언트가 2^53 미만으로 만들고 서버는 관대하게 받는다

> 최종 갱신 2026-10-15 · 대응 커밋 ea698a9
> 상태: 유효 · 결정일 2026-10-14 · 결정자 메인(버그 수정, 사용자에게 보고)

## 배경

`v0.1.2` 를 받은 사용자의 모든 판이 점수 서버에서 400 으로 거절됐다. 클라이언트는 새 판의 배치 시드로 시계의 나노초(약 1.79e18)를 그대로 `u64` 로 보냈고, 서버의 Zod 검증은 시드에 안전한 정수(2^53 - 1 이하)만 받았다. JSON 의 수는 JS 에서 2^53 을 넘으면 정확한 정수가 아니다. 계약 테스트는 손으로 만든 고정 본문만 써서 이 어긋남을 잡지 못했다.

## 결정

1. 클라이언트: 새 판의 시드는 시계 나노초의 아래 53비트만 쓴다(`LAYOUT_SEED_MASK = (1 << 53) - 1`), 0 이면 1(`apps/rbms-player/src/app_play.rs` 의 `fresh_layout_seed`). 리플레이와 다시 하기가 넘겨받는 시드는 기록된 그대로 쓴다(배치를 재현해야 하므로).
2. 서버: 시드는 참고 값이므로 범위 밖이어도 판을 거절하지 않는다. `seedSchema = z.number().transform((value) => (Number.isSafeInteger(value) ? value : 0))`(`web/src/server/dto/common.ts`). 점수 제출(`score.ts`, 기본 0)과 리플레이 업로드(`replay.ts`, 기본 null)가 같이 쓴다.
3. 배포 순서: 서버(`d733a8f`)를 먼저 운영에 올리고, 클라이언트(`7095ef7`)는 `v0.1.3` 으로 낸다. 서버 수정만으로 v0.1.2 클라이언트의 제출도 받아진다.

## 이유

- 클라이언트 쪽: 시드는 JSON 으로 가므로 처음부터 JS 의 정확한 정수 범위 안에 있어야 한다. 53비트면 배치의 다양성에 부족함이 없다.
- 서버 쪽: 시드 하나 때문에 기록 전체를 잃는 것은 "서버로 저장되지 않는 기록따윈 없어"(2026-10-14) 원칙과 반대다. 이미 배포된 클라이언트(v0.1.2)를 고치지 않고도 받아야 한다.

## 기각된 대안

| 대안 | 기각 이유 |
| --- | --- |
| 서버만 고치기 | 큰 시드는 서버에서 정확한 값으로 읽히지 않아 0 으로 저장된다. 새 판은 처음부터 범위 안이어야 한다 |
| 클라이언트만 고치기 | 이미 배포된 v0.1.2 의 제출이 계속 400 이다 |
| 시드를 문자열로 보내도록 계약 변경 | 옛 서버·옛 클라이언트와 호환이 깨진다. 이번 수정의 범위를 넘는다 |

## 영향과 관련 파일

- `apps/rbms-player/src/app_play.rs`(`LAYOUT_SEED_MASK`, `fresh_layout_seed`, 테스트 `a_fresh_layout_seed_fits_what_the_score_server_reads_exactly`), `web/src/server/dto/common.ts`(`seedSchema`), `web/src/server/dto/score.ts`, `web/src/server/dto/replay.ts`, 테스트 `web/tests/dto/ir-contract.test.ts`, `web/tests/service/replay.test.ts`.
- 커밋: `d733a8f`(서버), `7095ef7`(클라이언트).
- 재발 방지(2026-10-14 `e9aebae` 로 해소): 실제 클라이언트가 만든 요청 본문(일반, 실패, SCRATCH AUTO, 空POOR>POOR, 넓힌 판정, 14K, 코스, 리플레이, 동기화 재제출 등)을 플레이어 테스트가 `contract/client-requests/*.json` 으로 뽑고(`apps/rbms-player/src/app_result/record_tests/contract_requests.rs`, 기본은 비교만, `RBMS_UPDATE_CONTRACT=1` 일 때 갱신), 웹 테스트가 서버 스키마·라우트 검사·서비스로 통과시킨다(`web/tests/dto/client-requests.test.ts`). 손으로 만든 고정 본문의 계약 테스트는 `crates/rbms-ir/src/contract_tests.rs`, `web/tests/dto/ir-contract.test.ts` 에 그대로 있다. 시드 테스트는 `app_play.rs` 의 `a_fresh_layout_seed_fits_what_the_score_server_reads_exactly`.
- KNOWN ISSUE: 2^53 이상인 옛 시드를 넘겨받은 판(v0.1.2 이전에 저장된 로컬 리플레이로 다시 하기 등)의 리플레이를 올리면 서버가 시드를 0 으로 저장한다. 그 리플레이를 다른 클라이언트가 받으면(`apps/rbms-player/src/ir_replay.rs` 의 `from_ir_replay`, `seed: data.seed.unwrap_or_default()`) RANDOM 판의 배치가 기록과 달라 입력이 맞지 않게 재생된다. 로컬 리플레이 파일은 정확한 시드를 갖고 있어 영향이 없다. 고치는 방법(새 세션에서 결정): 시드를 10진 문자열로도 받는 선택 필드를 계약에 더하거나(서버는 문자열을 그대로 저장, 클라이언트는 2^53 이상일 때만 문자열 필드를 채움), 서버가 시드를 잃은 리플레이(`seed=0` 이고 RANDOM 이 OFF 가 아님)를 내려줄 때 재생 불가로 표시한다.

## 날짜와 사용자 원문

2026-10-14. 사용자 원문 없음(메인이 400 원인을 찾아 고치고 보고). 관련 원칙의 원문: "서버로 저장되지 않는 기록따윈 없어 무조건 저장될거야".
