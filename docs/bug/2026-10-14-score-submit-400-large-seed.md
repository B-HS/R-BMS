# v0.1.2 의 모든 판이 점수 서버에서 400 으로 거절됨 (큰 시드)

최종 갱신 2026-10-14 · 대응 커밋 7095ef7 (서버 수정 `d733a8f`, 클라이언트 수정 `7095ef7`)

## 대상 파일

- `apps/rbms-player/src/app_play.rs` (`fresh_layout_seed`, `LAYOUT_SEED_MASK`)
- `web/src/server/dto/common.ts` (`seedSchema`), `web/src/server/dto/score.ts`, `web/src/server/dto/replay.ts`
- 테스트: `web/tests/dto/ir-contract.test.ts`, `web/tests/service/replay.test.ts`, `app_play.rs` 의 `a_fresh_layout_seed_fits_what_the_score_server_reads_exactly`

## 증상

릴리스 v0.1.2 로 친 모든 판의 점수 제출이 서버에서 400 으로 거절됐다(v0.1.1 도 같은 코드지만 Gatekeeper 차단으로 실행되지 않아 이때 처음 드러났다). 로컬 기록(이력, 최고 기록, 리플레이)은 정상이다. 결과 화면의 `IR:` 줄이 `RANK` 가 되지 않는다. v0.1.2 draft 를 지우고 v0.1.3 으로 다시 내기로 했다. 태그 `v0.1.2` 는 남아 있다.

## 원인

클라이언트는 새 판의 배치 시드를 시계의 나노초(`as_nanos() as u64`, 2026-10-14 기준 약 1.79e18)로 만들었다. 서버의 `seed` 검증은 `z.number().int()` 였고 Zod 4.5.4 의 `int()` 는 안전한 정수(절댓값 `2^53 - 1 = 9_007_199_254_740_991` 이하)만 받는다. 점수 제출(`scoreSubmissionSchema.seed`)과 리플레이 업로드(`replayUploadSchema.seed`)가 모두 이 규칙이라 시드를 담은 본문이 전부 400 이 됐다. JSON 에서 읽은 2^53 이상의 수는 정밀도도 잃는다.

## 해결

- 서버: `seedSchema = z.number().transform((value) => (Number.isSafeInteger(value) ? value : 0))`. 시드는 참고 값이므로 범위 밖이면 0 으로 저장하고 기록은 받는다. 점수와 리플레이 두 스키마가 같이 쓴다.
- 클라이언트: 새 판의 시드를 시계의 아래 53비트(`LAYOUT_SEED_MASK = (1 << 53) - 1`)로 만들고 0 이면 1 로 한다. 리플레이와 다시 하기(파란 건반 + START)가 넘겨받는 시드는 기록된 그대로 쓴다.

## 결정 기록

무엇을 어느 쪽이 고치는지와 기각된 대안: [배치 시드 ADR](../acknowledge/2026-10-14-adr-layout-seed-range.md).

## 남은 일

- 운영 서버에서 실제 판의 제출이 `IR: RANK` 로 돌아오는지 사용자가 한 판 쳐서 확인한다(QA 항목 W3, X3).
- 같은 종류의 사고를 막는 교차 테스트(실제 판이 만든 요청 본문을 서버 스키마로 통과시킴)는 아직 없다: `docs/quality-assurance/2026-10-14-test-debt.md` 2절.
- 2^53 이상의 시드를 담은 옛 클라이언트의 리플레이는 서버에서 시드 0 이 된다. 해당 빌드는 게시된 적이 없어 영향이 거의 없다(같은 문서 3.8).
