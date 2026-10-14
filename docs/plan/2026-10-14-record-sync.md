# 서버 판정 수 검사 수리 + 기록 동기화 + Esc 포기 판 저장 — 구현 계약

> 작성 2026-10-14 · 기준 커밋 e2c5a38(dev) · 작성: 설계자(코드 미수정)
> 근거 결정: 사용자 결정(2026-10-14) (1)(2)(5)(7)(8)(9)(10)(11), 같은 날 원문 "3. 뭐 지울필요없이 그냥 0.1.4 로 내면 돼 / 4. 놔둬 / 7. 그냥 서버에 전체 보내서 실패하면 실패한거만 잘 처리해놔 / 나머지는 다 추천안대로". 클라이언트 변경은 0.1.4 로 낸다.
> 이 문서는 서버 구현자와 클라이언트 구현자가 서로 기다리지 않고 병렬로 구현하기 위한 계약이다. 두 구현자가 공유하는 것은 2절(와이어 계약)뿐이다.

## 0. 확인한 사실(근거)

| 사실 | 근거 |
| --- | --- |
| 서버 판정 수 식이 `pgreat+great+good+bad+max(poor-empty_poor,0)+miss` 라 空POOR 가 많은 완주 판을 400 으로 거절한다 | `web/src/server/service/domain/score/score.service.ts` 182~185행, `web/src/server/route/score.ts` 71~74행 |
| 클라이언트는 `poor=c[4]`(見逃し POOR, 노트 소비), `miss=c[5]`(空POOR, 노트 소비 안 함), `empty_poor=summary.empty_poor` 를 보낸다. 로컬 기록도 MISS 칸을 空POOR 로 읽는다 | `apps/rbms-player/src/app_result.rs` 151~177행, `apps/rbms-player/src/scoredb_store.rs` 205행(`empty_poor: log.judge.tier(JUDGE_MISS)`) |
| 노트를 소비하는 판정: 5K·7K·10K·14K·24K 는 PG·GR·GD·BD·POOR, POPN_9K 는 PG·GR·GD·POOR(BAD 는 소비 안 함). 판정 5(空POOR)는 어느 모드도 소비 안 함 | 원본 `beatoraja/src/bms/player/beatoraja/play/JudgeProperty.java` 42행 `PMS ... {true, true, true, false, true, false}`, 20·31·53행 나머지 `{true,true,true,true,true,false}`. rbms `crates/rbms-judge/src/windows.rs` 294행 `judge_vanish: [true, true, true, false, true, false]`, `matcher.rs` 699행 `vanish` 가 거짓이면 노트를 `judged` 로 두지 않는다 |
| 운영 서버는 오류 `details` 를 버린다(`NODE_ENV=production`) | `web/src/server/lib/api-response.ts` 19~22행. 사용자가 본 줄 `server error 400: 요청 형식이 올바르지 않습니다. (VALIDATION_ERROR)` 에 필드가 없는 이유 |
| Zod 실패의 `details` 는 `{ issues }` 뿐이고 `field` 가 없다 | `web/src/server/lib/parse-request.ts` 16·23·61행 |
| 점수 제출은 `(userId, chartSha256, playedAt)` 정확 일치로 멱등이다. 중복이면 200, `message: 'duplicate submission ignored'`, 기존 `score_id` | `score.service.ts` 214~229행, `compose/index.ts` 221~228행, `route/score.ts` 110행, `db/schema.ts` 131행 `score_idempotent_uniq` |
| 리플레이 업로드는 멱등이 아니다. 올릴 때마다 `rp_` 새 행을 넣고 `score.replay_id` 를 덮어쓴다(409 없음) | `web/src/server/service/domain/replay/replay.service.ts` 111~144행, `compose/replay.ts` 82~87행. `replay` 테이블에 유일 색인 없음(`schema.ts` 187행) |
| 레이트 리밋: 점수 제출 60/분/사용자(`score-submit:user:<id>`), 리플레이 업로드 10/분/사용자. 인스턴스 로컬 메모리라 다중 인스턴스에서는 best-effort. 429 본문은 `{"success":false,"error":{"code":"RATE_LIMITED","message":"…"}}`, `Retry-After` 헤더 없음 | `web/src/server/lib/with-rate-limit.ts` 4~13행, `route/score.ts` 30~32행, `route/replay.ts` 37~38행, `docs/backend/api-spec.md` 362행 |
| `isPlayedAtPlausible` 은 미래 5분만 막고 과거는 제한 없다. 오래된 판의 재전송은 문제없다 | `score.service.ts` 180행, 테스트 `web/tests/service/score-submit.test.ts` 405행 |
| `auditRejection` 은 이유 문자열을 받고 버린다. `submission_audit` 에 이유 열이 없다 | `route/score.ts` 34~45행, `schema.ts` 352~368행 |
| 로컬 `scorelog.date` 는 **초** 단위다. 제출 `played_at` 은 **밀리초**다. 리플레이 파일 이름에는 밀리초가 그대로 있다(`<md5 앞 8자>-<played_ms>.ron`) | `scoredb_store.rs` 163행, `app_result.rs` 209~211행 |
| `scorelog` 에 없어 제출 본문을 다시 만들 수 없는 값: 밀리초 시각, fast/slow, passnotes, lntype(키 `CHART` 인 판), 옵션(hispeed·lift·cover·green·judge_rate 등), 빌드 해시, 판정 알고리즘, 스킨, 아티스트·레벨. RON 에서 옮겨 온 행(`state=1`)은 sha256 도 빈 문자열 | `crates/rbms-store/src/scoredb/schema.sql` 39~76행, `scoredb/migrate.rs` 123~151행 |
| `ir_submitted` 는 "주 서버로 넘겼음"일 뿐 수락 여부가 아니다 | `app_result.rs` 260행 |
| 점수 DB 마이그레이션은 매번 `schema.sql`(전부 `CREATE … IF NOT EXISTS`)을 실행하고 세대를 찍는다. 더 높은 세대의 DB 는 거절한다(`FutureSchema`) | `crates/rbms-store/src/scoredb/schema.rs` 39~52행 |
| Esc 포기: `leave_run` 은 결과 화면·기록·전송 없이 나간다. 노트가 다 처리된 뒤의 Esc 와 닫히는 장면의 Esc 는 이미 `enter_result` 로 간다 | `apps/rbms-player/src/stage/play/mod.rs` 597~609행, 1285~1300행, 1395~1406행 |
| 원본은 Esc 포기를 `STATE_FAILED` 로 바꿔 결과 화면으로 가고 저장한다. 남은 노트는 판정 수에 넣지 않고 `minbp` 에만 더한다 | `beatoraja/.../play/BMSPlayer.java` 938~970행(`stopPlay`), 909행 |
| 로그인 상태 확인과 로그인 성공 지점 | `apps/rbms-player/src/app_network.rs` 271~331행(`poll_network` 의 `startup_whoami`, `poll_auth`, `poll_whoami`) |
| 라이브 제출은 `submit_rx` 한 칸이다. 새 제출이 오면 이전 수신기를 버린다 | `apps/rbms-player/src/app_ranking.rs` 107~114행, 203~218행 |
| 로컬 기록 모달은 `ScoreBook`(`ScoreRecord`)에서 만든다. `ScoreRecord` 에 `scorelog.id` 가 없다 | `apps/rbms-player/src/stage/select/scene.rs` 234~268행, `crates/rbms-render/src/select.rs` 107~126행 |
| 세대 2 이하 SCRATCH AUTO 판 = `assist == SCRATCH_AUTO_ASSIST(1) && rule_version < SCRATCH_AUTO_LEAVES_THE_TURNTABLE_RULE_VERSION(3)` | `crates/rbms-store/src/score.rs` 38·42·54~60행 |

확인 못함:

- 운영 서버의 `REQUIRE_BUILD_HASH` 값(기본 false, `web/src/server/lib/env.ts` 18행). true 면 재구성한 옛 판(빌드 해시 없음)은 400 `client_build_sha256` 로 거절돼 "거절" 로 기록된다.
- 사용자 DB 의 판 수, sha256 이 빈 행 수, 리플레이 파일이 남은 판 수.
- 곡 라이브러리 항목(`rbms_library::SongEntry`)에 차트의 `#LNMODE` 가 있는지(필드 목록에 보이지 않음, `crates/rbms-library/src/lib.rs` 25~45행).

## 1. 서버 변경

마이그레이션 없음. 운영 DB 작업 없음. 배포는 `web/` 변경이 `dev` 에 들어가면 Vercel 이 한다.

### 1.1 판정 수 검사 — `web/src/server/service/domain/score/score.service.ts`, `web/src/server/route/score.ts`

새 식: 노트를 소비한 판정만 센다.

```ts
const JUDGE_BAD_KEEPS_NOTE_MODES: ReadonlySet<string> = new Set(['POPN_9K'])

const normalizeMode = (mode: string) => mode.trim().toUpperCase().replaceAll('-', '_')

export const judgedNoteCount = (judge: JudgeBreakdownInput, mode: string) =>
    judge.pgreat + judge.great + judge.good + (JUDGE_BAD_KEEPS_NOTE_MODES.has(normalizeMode(mode)) ? 0 : judge.bad) + judge.poor

export const isJudgeCountConsistent = (judge: JudgeBreakdownInput, totalNotes: number, mode: string) =>
    totalNotes === 0 || judgedNoteCount(judge, mode) <= totalNotes
```

- `miss`·`empty_poor` 는 세지 않는다(空POOR 는 노트를 소비하지 않는다, 0절 근거).
- `normalizeMode` 는 옛 표기(`BEAT-7K`)도 받는다. rbms 클라이언트 `crates/rbms-store/src/scoredb/schema.rs` 150~154행의 `normalise_mode` 와 같은 규칙이다.
- 9K 판정: 원본 `JudgeProperty.PMS` 의 `judgeVanish[3] = false`(BAD 가 노트를 지우지 않는다). 9K 의 BAD 뒤에 같은 노트가 다시 판정되므로 BAD 를 세면 정상 판도 넘친다.
- `route/score.ts` 71행: `isJudgeCountConsistent(input.judge, input.total_notes, input.mode)`. 오류 details 는 지금 그대로 `{ field: 'judge', reason: 'judge count exceeds total_notes' }`.

### 1.2 멱등 판정을 같은 초 안으로 — `score.service.ts`, `web/src/server/compose/index.ts`

이유: 로컬 기록의 시각은 초 단위라, 리플레이 파일이 없는 옛 판을 다시 보내면 `played_at` 이 원래 밀리초와 다르다(`date*1000`). 정확 일치로는 이미 받은 판이 중복 저장된다.

- `ScoreServiceDb.findScoreIdByIdempotency` 의 인자를 `{ userId: string; chartSha256: string; playedAtFrom: number; playedAtTo: number }` 로 바꾼다.
- 서비스에 상수 `IDEMPOTENCY_WINDOW_MS = 1000` 을 두고 `playedAtFrom = Math.floor(input.played_at / IDEMPOTENCY_WINDOW_MS) * IDEMPOTENCY_WINDOW_MS`, `playedAtTo = playedAtFrom + IDEMPOTENCY_WINDOW_MS - 1`.
- compose: `where(and(eq(score.userId, userId), eq(score.chartSha256, chartSha256), between(score.playedAt, playedAtFrom, playedAtTo))).orderBy(asc(score.playedAt)).limit(1)`. 색인 `score_idempotent_uniq(userId, chartSha256, playedAt)` 의 앞부분 범위 탐색이라 새 색인이 필요 없다.
- 같은 사용자가 같은 차트를 같은 1초 안에 두 번 끝내는 일은 없다(한 판이 1초보다 길다). 유일 색인은 그대로 두므로 정확히 같은 밀리초의 동시 삽입은 지금처럼 DB 가 막는다.
- 중복 응답 문자열을 상수로 뺀다: `export const DUPLICATE_SUBMISSION_MESSAGE = 'duplicate submission ignored'`(값 바꾸지 않음. 클라이언트가 이 문자열로 "이미 있음"을 센다, 2절).

### 1.3 오류 본문에 필드와 이유 — `web/src/server/lib/api-response.ts`, `web/src/server/lib/parse-request.ts`

- `api-response.ts` 에 순수 함수 `publicErrorDetails(details: Record<string, unknown> | undefined, isProduction: boolean)` 를 export 한다.
  - 비프로덕션: `details` 그대로.
  - 프로덕션: `details.field`·`details.reason` 중 **문자열인 것만** 담은 객체. 둘 다 없으면 `undefined`(본문에 `details` 키 없음).
  - 이유: 필드 이름과 고정 이유 문자열은 비밀이 아니다. `issues`(입력값 일부가 들어갈 수 있는 Zod 원문)·`player`·`path` 같은 나머지 키는 계속 프로덕션에서 숨긴다(`docs/convention/security.md` §6). 폴더 경로의 `reason`(`limit_exceeded` 등 고정 토큰, `route/folder.ts` 57~59행)도 이 규칙으로 노출된다 — 고정 토큰이라 문제없다.
  - `errorResponse` 는 `publicErrorDetails(details, getEnv().NODE_ENV === 'production')` 결과가 있을 때만 `details` 를 싣는다.
- `parse-request.ts` 세 곳(`parseJsonBody`, `parseSearchParams`, `parseLimitedJsonBody`)의 Zod 실패를 한 함수로 모은다(2회 이상 사용): `details = { field, reason, issues }`.
  - `field` = 첫 이슈의 `path.join('.')`, 경로가 비면 상수 `ROOT_FIELD = 'body'`.
  - `reason` = 첫 이슈의 `message`.
- 결과(프로덕션): `{"success":false,"error":{"code":"VALIDATION_ERROR","message":"요청 형식이 올바르지 않습니다.","details":{"field":"judge","reason":"judge count exceeds total_notes"}}}`.

### 1.4 리플레이 업로드 멱등 — `replay.service.ts`, `compose/replay.ts`, `route/replay.ts`

- `ReplayServiceDb` 에 `findLinkedReplay: (params: { scoreId: string; userId: string }) => Promise<{ id: string; eventCount: number } | null>` 를 더한다.
  - compose: `db.select({ id: replay.id, eventCount: replay.eventCount }).from(score).innerJoin(replay, eq(replay.id, score.replayId)).where(and(eq(score.id, scoreId), eq(score.userId, userId))).limit(1)`.
- `upload` 첫머리: `input.score_id` 가 있고 `findLinkedReplay` 가 행을 돌려주면 저장·연결 없이 `{ id, url: replayUrl(id), event_count: eventCount, created: false }` 를 돌려준다. 새로 저장하면 `created: true`.
- 라우트: `const { created, ...body } = result` → `irRaw(body, created ? CREATED_STATUS : 200)`. `revalidateOnReplayUpload` 는 `created` 일 때만. 응답 본문 모양은 바뀌지 않는다.
- `score_id` 없는 업로드는 지금처럼 매번 새 행이다.

### 1.5 거절 이유 감사 기록 — 이번 범위 밖(권고: 후속)

`submission_audit` 에 이유 열이 없어 열을 더하는 마이그레이션(0003)과 운영 적용(`web-db` 워크플로, `docs/acknowledge/2026-10-14-adr-production-db-operations.md`)이 코드 배포보다 먼저 필요하다. 순서를 어기면 `insertAudit` 이 `.catch(() => undefined)` 로 실패를 삼켜 **수락 감사까지** 조용히 빠진다. `flags`(타입 `ScoreFlag[]`)에 이유를 섞는 것은 열의 뜻을 깨는 우회라 하지 않는다. 이번에는 하지 않고 `docs/quality-assurance` 의 테스트 부채가 아니라 웹 후속 후보로 메인이 기록한다.

### 1.6 서버 테스트(값 고정)

`web/tests/service/score-submit.test.ts` 의 `describe('judge 합계 일관성')` 를 아래로 바꾼다(기존 416~443행 고정값은 클라이언트 의미와 달라 폐기). 공통 고정값 `judge = judgeBreakdownSchema.parse({ pgreat: 700, great: 60, good: 20, bad: 8, poor: 12, miss: 4, fast: 0, slow: 0, combobreak: 0, empty_poor: 4 })`.

| 테스트 이름 | 입력 | 기대 |
| --- | --- | --- |
| `노트를 소비한 판정(PG·GR·GD·BD·POOR)만 센다` | 위 judge, `'BEAT_7K'` | `judgedNoteCount` = 800 |
| `판정 합이 total_notes 를 넘으면 거부한다` | 위 judge, `'BEAT_7K'` | 812 → true, 800 → true, 799 → false |
| `空POOR 가 見逃し POOR 보다 많은 완주 판을 받는다(사용자 판 #52)` | `{ pgreat: 1500, great: 250, good: 50, bad: 15, poor: 20, miss: 43, empty_poor: 43, fast: 0, slow: 0, combobreak: 0 }`, total 1835, `'BEAT_7K'` | `judgedNoteCount` = 1835, `isJudgeCountConsistent` = true. (옛 식은 1815 + 0 + 43 = 1858 로 거절) |
| `POPN_9K 는 노트를 소비하지 않는 BAD 를 세지 않는다` | `{ pgreat: 90, great: 5, good: 0, bad: 8, poor: 5, miss: 0, empty_poor: 0, … }`, total 100 | `'POPN_9K'` → true(100), `'popn-9k'` → true, `'BEAT_7K'` → false(108) |
| `total_notes 가 0 인 구버전 제출은 검사를 건너뛴다` | 위 judge, 0, `'BEAT_7K'` | true |
| `같은 초 안의 played_at 재전송은 기존 score_id 를 돌려준다` | `createStub({ existingScoreId: 'sc_old' })`, `played_at: PLAYED_AT + 456`(PLAYED_AT = 1700000000000) | 스텁이 받은 인자 `{ playedAtFrom: 1700000000000, playedAtTo: 1700000000999 }`, 응답 `score_id: 'sc_old'`, `message: DUPLICATE_SUBMISSION_MESSAGE`, `insertScore` 호출 0 |
| `중복 응답 문구는 클라이언트 계약 문자열 그대로다` | — | `DUPLICATE_SUBMISSION_MESSAGE === 'duplicate submission ignored'` |

`createStub` 의 `findScoreIdByIdempotency` 스텁은 받은 인자를 `recorder.idempotencyWindows` 에 쌓도록 바꾼다.

`web/tests/service/replay.test.ts`:

- `같은 score_id 에 이미 연결된 리플레이가 있으면 새로 저장하지 않고 그 리플레이를 돌려준다` — 스텁 `findLinkedReplay` 가 `{ id: 'rp_old', eventCount: 7 }` → 결과 `{ id: 'rp_old', url: '/api/replays/rp_old', event_count: 7, created: false }`, `stored.length === 0`, `links.length === 0`.
- `score_id 가 없으면 이미 있는 리플레이를 찾지 않고 새로 저장한다` — `findLinkedReplay` 호출 0, `created: true`.
- 기존 테스트의 스텁 db 에 `findLinkedReplay: async () => null` 을 더한다.

`web/tests/lib/error-and-response.test.ts`:

- `publicErrorDetails 는 프로덕션에서 문자열 field·reason 만 남긴다` — `({ field: 'judge', reason: 'x', issues: [1], player: 'p' }, true)` → `{ field: 'judge', reason: 'x' }`.
- `프로덕션에서 field·reason 이 없으면 details 를 싣지 않는다` — `({ player: 'p' }, true)` → `undefined`, `(undefined, true)` → `undefined`.
- `비프로덕션에서는 details 를 그대로 둔다` — `({ issues: [1] }, false)` → 같은 객체.

`web/tests/lib/parse-request.test.ts`:

- `스키마 검증 실패는 첫 이슈의 경로와 메시지를 field·reason 으로 싣는다` — 스키마 `z.object({ judge: z.object({ poor: z.number().int().min(0) }) })`, 본문 `{"judge":{"poor":-1}}` → `details.field === 'judge.poor'`, `typeof details.reason === 'string'`, `Array.isArray(details.issues)`.
- `본문 전체가 틀리면 field 는 body 다` — 본문 `"null"` → `details.field === 'body'`.

## 2. 와이어 계약(클라이언트가 의존하는 서버 응답)

새 필드는 만들지 않는다. 바뀌는 것은 (a) 프로덕션에서 `details.field`·`details.reason` 이 보인다, (b) 멱등이 같은 초 안으로 넓어진다, (c) 리플레이 재업로드가 200 으로 기존 것을 돌려준다, 셋이다.

### 2.1 `POST /api/scores`

| 상태 | 본문 | 클라이언트 분류 |
| --- | --- | --- |
| 201 | `SubmitResponse{ accepted: true, score_id: "sc_…", message: "saved" \| "saved (scratch auto)" \| "recorded (unranked: …)", ranked, flags, rank, previous_best, is_new_best }` | 수락(새로 저장) |
| 200 | 같은 모양, `message: "duplicate submission ignored"`, `score_id` = 기존 id, `is_new_best: false`, `previous_best: null` | 수락(이미 있음). `HttpScoreServer::submit_score` 는 상태 코드를 버리므로 `message` 로 구분한다 |
| 400 | `{ "success": false, "error": { "code": "VALIDATION_ERROR" \| "IR_API_VERSION_UNSUPPORTED", "message": "…", "details"?: { "field"?: string, "reason"?: string } } }`. 비프로덕션은 `details.issues`(Zod 이슈 배열)도 있다 | 거절 |
| 401 | `code: "UNAUTHORIZED"` | 동기화 중단(토큰 문제) |
| 403 | `code: "FORBIDDEN"`(본문 `player.id` ≠ 토큰 사용자). 프로덕션 details 없음 | 거절 |
| 404 | `code: "IR_CHART_NOT_FOUND"` | 거절 |
| 413 | `code: "IR_PAYLOAD_TOO_LARGE"` | 거절 |
| 429 | `code: "RATE_LIMITED"`, details 없음, `Retry-After` 없음 | 일시 실패(기다렸다 같은 판 재시도) |
| 5xx | `code: "INTERNAL_ERROR"` 또는 프록시 본문 | 일시 실패 |

`accepted: false` 인 2xx 는 지금 서버가 만들지 않지만 오면 거절로 센다.

### 2.2 `POST /api/charts/{hash}/replays`

| 상태 | 본문 | 클라이언트 분류 |
| --- | --- | --- |
| 201 | `{ "id": "rp_…", "url": "/api/replays/rp_…", "event_count": n }` | 올림 |
| 200 | 같은 모양. 본문의 `score_id` 에 이미 연결된 리플레이 | 올림(이미 있음) |
| 4xx | 위 2.1 과 같은 오류 봉투 | 리플레이 거절 |
| 429·5xx·네트워크 | — | 리플레이 일시 실패 |

### 2.3 배포 순서

서버를 먼저 배포하고(`dev` 푸시 → Vercel) 운영에서 `GET` 만으로 배포 커밋을 확인한 뒤 클라이언트 0.1.4 를 낸다. 옛 서버에 새 클라이언트가 동기화하면 (a) #52 같은 판은 거절로 기록되고 다음 동기화에서 다시 간다(무해), (b) 리플레이 없는 옛 판이 초 단위 `played_at` 으로 **중복 저장될 수 있다**(유해). 그래서 순서가 필요하다.

## 3. 클라이언트 변경

### 3.1 점수 DB — `crates/rbms-store/src/scoredb/schema.sql`, `scoredb.rs`, `scoredb/rows.rs`, 새 `scoredb/sync.rs`

세대(`SCORE_DB_SCHEMA_VERSION`)는 **1 그대로 둔다.** 새 테이블 둘만 더하는 추가 변경이고, `migrate` 가 매번 `schema.sql` 을 실행하므로 기존 DB 에 그대로 생긴다. 세대를 올리면 0.1.3 으로 되돌린 사용자의 DB 가 `FutureSchema` 로 거절돼 기록이 아예 안 남는다(`schema.rs` 41~45행). 옛 빌드는 새 테이블을 모르고 무시하며, 옛 빌드가 적은 판은 본문이 없는 판으로 동기화된다(3.3 재구성).

`schema.sql` 끝에 더한다(SQL 파일에도 주석을 쓰지 않는다):

```sql
CREATE TABLE IF NOT EXISTS play_submission (
    log_id    INTEGER PRIMARY KEY,
    body      TEXT NOT NULL,
    played_as TEXT
);

CREATE TABLE IF NOT EXISTS ir_sync (
    log_id          INTEGER NOT NULL,
    server          TEXT NOT NULL,
    login_id        TEXT NOT NULL DEFAULT '',
    state           TEXT NOT NULL,
    score_id        TEXT,
    http_status     INTEGER,
    error_code      TEXT,
    error_field     TEXT,
    error_reason    TEXT,
    attempts        INTEGER NOT NULL DEFAULT 0,
    last_attempt_at INTEGER NOT NULL DEFAULT 0,
    replay_state    TEXT NOT NULL DEFAULT 'none',
    replay_id       TEXT,
    replay_error    TEXT,
    PRIMARY KEY (log_id, server)
);

CREATE INDEX IF NOT EXISTS idx_ir_sync_server_state ON ir_sync(server, state);
```

열의 뜻(문서와 Rust `///` 에만 적는다):

| 열 | 뜻 |
| --- | --- |
| `play_submission.log_id` | `scorelog.id` |
| `play_submission.body` | 그 판을 끝낼 때 만든 `ScoreSubmission` 의 JSON(`serde_json`). 보낼 때 `player.id` 는 그때 로그인한 id 로 덮는다 |
| `play_submission.played_as` | 판을 칠 때 로그인한 login id. 게스트면 NULL |
| `ir_sync.server` | 정규화한 SERVER URL(앞뒤 공백 제거, 끝 `/` 제거). 서버를 바꾸면 상태가 따로 쌓인다 |
| `ir_sync.login_id` | 마지막으로 보낸 계정 |
| `ir_sync.state` | `accepted` \| `rejected` \| `retry` \| `skipped` |
| `ir_sync.score_id` | 수락이면 서버 `score_id` |
| `http_status`, `error_code`, `error_field`, `error_reason` | 거절·일시 실패·건너뜀의 이유. `skipped` 의 `error_code` 는 로컬 토큰(`NO_CHART_HASH`, `OLD_SCRATCH_AUTO_RULE`) |
| `attempts` | 이 서버로 보낸 횟수(업서트마다 +1) |
| `last_attempt_at` | 마지막 시도 시각, epoch **밀리초** |
| `replay_state` | `none`(로컬 리플레이 없음) \| `pending`(점수 수락, 아직 안 올림. AUTO UPLOAD REPLAY 꺼짐 포함) \| `uploaded` \| `failed`(일시 실패) \| `rejected`(4xx) \| `skipped`(올릴 대상 아님: unranked 응답 또는 `wire_carries_judge` 거짓) \| `missing`(파일이 지워짐) |
| `replay_id`, `replay_error` | 올린 리플레이 id, 마지막 리플레이 오류 한 줄 |

"미확인" 은 행이 없는 것이다. 그래서 옛 `ir_submitted` 를 옮기는 마이그레이션이 필요 없고, `ir_submitted` 는 어떤 결정에도 읽지 않는다(값은 지금처럼 계속 적는다, 문서에 "참고용, 수락 증거 아님"으로 적는다).

Rust 쪽(이름은 계약, 내부 구현은 자유):

- `PlayLog` 에 `pub id: i64` 를 더한다(0 = 아직 저장 안 됨). `log_from_row` 가 `id` 를 읽는다. `insert_log` 는 무시한다. `migrate.rs` 의 `log_from_record`, `scoredb_store::play_log`, 테스트의 `play()` 를 맞춘다.
- `ScoreRecord`(`crates/rbms-store/src/score.rs`)에 `#[serde(default, skip_serializing_if = "Option::is_none")] pub log_id: Option<i64>` 를 더한다. RON 책과의 호환은 serde 기본값으로 지킨다.
- 새 모듈 `scoredb/sync.rs`:
  - `pub enum SyncState { Accepted, Rejected, Retry, Skipped }`, `pub enum ReplayState { None, Pending, Uploaded, Failed, Rejected, Skipped, Missing }` 와 각각 `as_token()`/`from_token()`(위 표의 소문자 토큰). 알 수 없는 토큰은 `Retry`/`Pending` 으로 읽는다.
  - `pub struct IrSyncRow { log_id, server, login_id, state, score_id, http_status: Option<u16>, error_code, error_field, error_reason, attempts: u32, last_attempt_at_ms: i64, replay_state, replay_id, replay_error }`.
  - `pub struct SyncCandidate { pub log: PlayLog, pub body: Option<String>, pub played_as: Option<String>, pub state: Option<SyncState>, pub score_id: Option<String>, pub replay_state: Option<ReplayState> }`.
  - `ScoreDb::put_submission(&mut self, log_id: i64, body: &str, played_as: Option<&str>)` — `INSERT OR REPLACE`.
  - `ScoreDb::sync_candidates(&self, server: &str, login_id: &str, only: Option<&[i64]>) -> Result<Vec<SyncCandidate>, ScoreDbError>` — 아래 SQL. `only` 가 있으면 `l.id IN (…)` 을 더한다.
  - `ScoreDb::replay_candidates(&self, server: &str, login_id: &str) -> Result<Vec<SyncCandidate>, ScoreDbError>` — `y.state = 'accepted' AND y.login_id = ?3 AND y.replay_state IN ('pending','failed','rejected') AND l.replay_file IS NOT NULL`. 리플레이는 그 점수를 보낸 계정으로만 올리므로 계정도 받는다.
  - `ScoreDb::put_sync_state(&mut self, row: &IrSyncRow)` — `INSERT … ON CONFLICT(log_id, server) DO UPDATE SET … , attempts = ir_sync.attempts + 1`. 점수 단계의 결과용.
  - `ScoreDb::put_replay_state(&mut self, log_id: i64, server: &str, state: ReplayState, replay_id: Option<&str>, error: Option<&str>)` — 리플레이 열만 바꾼다.
  - `ScoreDb::sync_states(&self, server: &str) -> Result<HashMap<i64, IrSyncRow>, ScoreDbError>` — 기록 모달용.
  - `ScoreDb::mark_old_scratch_auto_skipped(&mut self, server: &str, now_ms: i64) -> Result<usize, ScoreDbError>` — `INSERT OR IGNORE INTO ir_sync(log_id, server, state, error_code, last_attempt_at) SELECT id, ?1, 'skipped', 'OLD_SCRATCH_AUTO_RULE', ?2 FROM scorelog WHERE (assist = ?3 OR assist = ?5) AND rule_version < ?4`(`?3 = SCRATCH_AUTO_ASSIST`, `?4 = SCRATCH_AUTO_LEAVES_THE_TURNTABLE_RULE_VERSION`, `?5 = ASSIST_UNRECORDED`). 모달 표시용이다. `scores.ron` 에서 옮겨 온 판은 어시스트가 있었다는 것만 알 뿐 SCRATCH AUTO 인지 모르므로(`ASSIST_UNRECORDED`), 세대 2 이하의 그런 판은 SCRATCH AUTO 일 수 있어 같이 보내지 않는다(리뷰 반영).
  - `ScoreDb::backfill_sha256(&mut self, log_id: i64, sha256: &str)` — `UPDATE scorelog SET sha256 = ?2 WHERE id = ?1 AND sha256 = ''`.

대상 선정 SQL(`sync_candidates`, 상수는 바인딩한다):

```sql
SELECT l.id, l.chart_key, …(LOG_COLUMNS 전부 l. 접두), s.body, s.played_as,
       y.state AS sync_state, y.score_id, y.replay_state
FROM scorelog l
LEFT JOIN play_submission s ON s.log_id = l.id
LEFT JOIN ir_sync y ON y.log_id = l.id AND y.server = ?1
WHERE (y.state IS NULL OR y.state <> 'accepted')
  AND (s.played_as IS NULL OR s.played_as = ?2)
  AND NOT ((l.assist = ?3 OR l.assist = ?6) AND l.rule_version < ?4)
ORDER BY l.date ASC, l.id ASC
```

`scorelog.state` 와 `ir_sync.state` 의 이름이 겹치므로 별칭을 반드시 쓴다. `LOG_COLUMNS` 는 접두 없는 문자열이라 이 쿼리용 접두 목록을 `rows.rs` 에 따로 둔다(한 목록에서 만들어 드리프트를 막는다).

선정 규칙 요약: 이 서버에서 수락되지 않은 판 전부(거절·일시 실패·건너뜀·미확인 모두 다시 보낸다, 사용자 결정 (7)), 다른 계정으로 친 판 제외, 세대 2 이하 SCRATCH AUTO 판 제외(결정 (10). `?6 = ASSIST_UNRECORDED` 로 옮겨 온 어시스트 판도 같은 세대 조건이면 뺀다. `?5` 는 `'accepted'`). 게스트로 친 판과 이 변경 이전의 판(`played_as` NULL)은 처음 동기화하는 계정으로 간다(결정 (7) "로컬 이력 전체").

### 3.2 IR 크레이트 — `crates/rbms-ir/src/error.rs`, `crates/rbms-ir/src/dto/score.rs`, 새 `crates/rbms-ir/src/sync.rs`

`error.rs`:

- `pub struct ServerReason { pub code: String, pub field: Option<String>, pub reason: Option<String> }` 와 `impl IrError { pub fn server_reason(&self) -> Option<ServerReason> }` — `detail()` 이 오류 봉투면 `error.code`, `error.details.field`, `error.details.reason`(문자열일 때만).
- `readable_detail` 의 봉투 분기: `field` 가 있으면 `"{code}: {field}"`, `reason` 도 있으면 `"{code}: {field} ({reason})"`. `field` 가 없으면 지금처럼 `"{message} ({code})"`. 200자 자르기는 그대로.
- 결과 줄 길이: 결과 화면은 `format!("IR ERROR: {}", short_error(e))` 이고 `short_error` 가 56자로 자른다(`ir_outcome.rs` 17·112~114행). #52 오류는 `IR ERROR: server error 400: VALIDATION_ERROR: judge (judge count e...` 로 보인다. 코드와 필드가 앞 41자 안에 들어오므로 자르는 규칙은 바꾸지 않는다.

`dto/score.rs`:

- `pub const DUPLICATE_SUBMISSION_MESSAGE: &str = "duplicate submission ignored";`, `impl SubmitResponse { pub fn is_duplicate(&self) -> bool }`(앞뒤 공백 무시, 정확 일치).

`sync.rs`(순수 분류, 스레드 없음):

```rust
pub struct Rejection { pub status: Option<u16>, pub code: Option<String>, pub field: Option<String>, pub reason: Option<String>, pub detail: String }
pub enum SubmitVerdict {
    Accepted { score_id: Option<String>, already: bool, replay_warranted: bool },
    Rejected(Rejection),
    RateLimited,
    Transient { detail: String },
    StopSync { detail: String },
}
pub fn submit_verdict(result: &Result<SubmitResponse, IrError>) -> SubmitVerdict
pub enum UploadVerdict { Uploaded { replay_id: String }, Rejected(Rejection), RateLimited, Transient { detail: String }, StopSync { detail: String } }
pub fn upload_verdict(result: &Result<String, IrError>) -> UploadVerdict
```

분류 규칙: `Ok(accepted)` → Accepted(`already = is_duplicate()`, `replay_warranted = is_replay_upload_warranted(resp)`), `Ok(!accepted)` → Rejected, `RateLimited` → RateLimited, `Network`·`Decode`·`Server(≥500)` → Transient, `Unauthorized`·`NotConfigured`·`Unsupported` → StopSync, 그 밖(`Server(4xx)`·`Forbidden`·`NotFound`·`Conflict`·`PayloadTooLarge`·`SettingsConflict`) → Rejected(`server_reason()` 으로 code·field·reason 을 채운다, `detail` = `short_error` 와 같은 한 줄). `lib.rs` 에서 `pub use`.

### 3.3 동기화 엔진 — 새 `apps/rbms-player/src/record_sync.rs`

#### 3.3.1 메인 스레드와 일꾼 스레드의 나눔

- 메인 스레드: 대상 선정(DB 조회 두 번), 곡 라이브러리에서 md5 → (경로, 아티스트, 레벨) 표 만들기, 결과를 DB 에 쓰기. DB 연결은 `AppShared::scoredb` 하나만 쓴다(일꾼은 DB 를 만지지 않는다).
- 일꾼 스레드(`std::thread::spawn`, 기존 `rbms_ir::spawn_submit` 방식): 본문 복원·재구성, 빈 sha256 채우기(차트 파일 읽기), 리플레이 파일 읽기와 변환, 전송, 간격 맞추기. 결과는 `mpsc::Sender<SyncEvent>` 로 보낸다.
- 화면 프레임: 메인은 `AppShared::poll_ir_jobs`(`app_ranking.rs` 157행) 안에서 `poll_record_sync()` 로 그 프레임에 와 있는 사건을 모두 `try_recv` 로 꺼내 처리한다. 사건 하나당 DB 업서트 한 번.

#### 3.3.2 타입(이름은 계약)

```rust
pub(crate) struct SyncJob {
    pub(crate) login_id: String,
    pub(crate) scores: Vec<SyncCandidate>,
    pub(crate) replays: Vec<SyncCandidate>,
    pub(crate) upload_replays: bool,
    pub(crate) replay_dir: PathBuf,
    pub(crate) charts: HashMap<String, ChartFacts>,
    pub(crate) pacing: SyncPacing,
}
pub(crate) struct ChartFacts { pub(crate) path: PathBuf, pub(crate) artist: String, pub(crate) level: String }
pub(crate) struct SyncPacing { pub(crate) score_interval: Duration, pub(crate) replay_interval: Duration, pub(crate) rate_limit_wait: Duration, pub(crate) cancel_check: Duration }
pub(crate) enum SyncEvent {
    Started { scores: usize, replays: usize },
    Backfilled { log_id: i64, sha256: String },
    Score { log_id: i64, outcome: ScoreOutcome },
    Replays { total: usize },
    Replay { log_id: i64, state: ReplayState, replay_id: Option<String>, error: Option<String> },
    Finished { stop: Option<SyncStop> },
}
pub(crate) enum ScoreOutcome { Accepted { score_id: Option<String>, already: bool, replay: ReplayState }, Rejected(Rejection), Retry { detail: String }, Skipped { code: &'static str, reason: &'static str } }
pub(crate) enum SyncStop { Unauthorized(String), Unreachable(String), RateLimited, Cancelled }
pub(crate) fn spawn_record_sync(server: Arc<dyn ScoreServer>, job: SyncJob, cancel: Arc<AtomicBool>) -> Receiver<SyncEvent>
```

구현에서 계약과 달라진 점: 서버 키는 일꾼에 넘기지 않는다(DB 에 쓰는 것은 메인이라 `RecordSyncRun` 이 갖는다). 일꾼의 입력은 `SyncCandidate` 를 그대로 쓰므로 `SyncRow` 는 없다. 점수 단계가 끝나 리플레이 단계가 시작될 때 총수를 알리는 `Replays { total }` 가 더해졌다(진행 줄의 `REPLAYS 3/80`).

#### 3.3.3 상수(`record_sync.rs` 상단, 의미 있는 값은 전부 상수)

| 상수 | 값 | 근거 |
| --- | --- | --- |
| `SCORE_SEND_INTERVAL` | 1 500 ms | 서버 60/분. 40/분으로 보내 라이브 판 제출 몫 20/분을 남긴다 |
| `REPLAY_SEND_INTERVAL` | 7 500 ms | 서버 10/분. 8/분으로 보내 라이브 리플레이 몫 2/분을 남긴다 |
| `RATE_LIMIT_WAIT` | 60 s | 서버 창 `windowMs: 60_000` |
| `RATE_LIMIT_WAITS_PER_REQUEST` | 2 | 같은 요청에 두 번 기다려도 429 면 동기화를 멈춘다 |
| `TRANSIENT_FAILURES_BEFORE_STOP` | 3 | 연속 일시 실패 세 번이면 서버가 닿지 않는 것으로 보고 멈춘다 |
| `CANCEL_CHECK_INTERVAL` | 100 ms | 기다리는 동안 취소 깃발을 보는 간격 |

간격은 "앞 요청 시작부터 다음 요청 시작까지"로 잰다(`interval - elapsed` 만 잔다). 테스트는 `SyncPacing` 을 0 으로 넣는다.

#### 3.3.4 순서

1. `Started` 를 보낸다.
2. **점수 단계**: `scores` 를 받은 순서(오래된 판부터)로 하나씩.
   1. 본문 만들기(3.3.5). 만들 수 없으면 `Skipped` 를 보내고 다음 판(요청 없음, 간격 없음).
   2. `submission.player.id = login_id`, `clamp_to_server_bounds()`, `server.submit_score`.
   3. `submit_verdict`:
      - Accepted → `Score{Accepted{score_id, already, replay}}`. `replay` 는 로컬 리플레이 파일이 없으면 `None`, `replay_warranted` 거짓이면 `Skipped`, 아니면 `Pending` 이고 이 판을 리플레이 단계 목록 끝에 더한다.
      - Rejected → `Score{Rejected}`. 연속 일시 실패 수를 0 으로.
      - RateLimited → `RATE_LIMIT_WAIT` 기다렸다 같은 판을 다시. `RATE_LIMIT_WAITS_PER_REQUEST` 를 넘으면 이 판을 `Retry` 로 보내고 `Finished{RateLimited}`.
      - Transient → `Score{Retry}`, 연속 일시 실패 +1, `TRANSIENT_FAILURES_BEFORE_STOP` 이면 `Finished{Unreachable}`.
      - StopSync → 이 판을 `Retry` 로 보내고 `Finished{Unauthorized}`.
   4. 각 요청 사이 `SCORE_SEND_INTERVAL`.
3. **리플레이 단계**(`upload_replays` 가 참일 때만, 거짓이면 건너뛰고 `pending` 은 그대로 남는다): `replays`(DB 에서 미리 고른 것) + 점수 단계에서 더한 것.
   1. 파일 `replay_dir/<replay_file>` 를 `rbms_store::Replay::load`. 없거나 못 읽으면 `Replay{Missing}`.
   2. `wire_carries_judge(&replay.judge)` 거짓이면 `Replay{Skipped}`.
   3. `to_ir_replay(&replay, &chart_id, constant, lntype, build_sha)` — `constant`·`lntype`·`build_sha` 는 그 판의 본문 값(저장 본문이 없으면 `false`, 재구성 lntype, `None`). `data.score_id = Some(score_id)`, `data.chart` 가 비면 `chart_id`.
   4. `server.upload_replay(&chart_id, &data)` → `upload_verdict` 를 점수 단계와 같은 규칙으로 처리(Rejected → `Replay{Rejected}`, Transient → `Replay{Failed}`).
   5. 각 요청 사이 `REPLAY_SEND_INTERVAL`.
4. `Finished{stop: None}`.

취소: `cancel` 이 참이 되면 다음 확인 지점(판 사이, 기다리는 중 `CANCEL_CHECK_INTERVAL` 마다)에서 `Finished{Cancelled}` 를 보내고 끝난다. 진행 중인 HTTP 요청은 끊지 않는다(기존 일꾼과 같다, `REQUEST_TIMEOUT` 5초).

#### 3.3.5 본문 만들기

1. `body` 가 있고 `serde_json::from_str::<ScoreSubmission>` 이 되면 그것을 쓴다.
2. 아니면 `PlayLog` 와 `ChartFacts` 로 다시 만든다(`fn rebuilt_submission(log: &PlayLog, facts: Option<&ChartFacts>) -> ScoreSubmission`, 순수 함수):

| 필드 | 값 |
| --- | --- |
| `chart` | md5 = `log.md5`, sha256 = `log.sha256`(비면 아래 3), title = `log.title`, artist·level = `ChartFacts`(없으면 빈 값·None. 서버는 비어도 받는다) |
| `mode` | `mode_name(log.mode)` |
| `clear` | `ir_clear(clear_type_from_id(log.clear))` |
| `ex_score` / `max_ex_score` | `log.judge.ex_score()` / `log.notes * 2` |
| `judge` | 티어별 합(PG·GR·GD·BD·POOR·MISS), e*/l* 은 `log.judge` 그대로, `empty_poor = tier(MISS)`, `combobreak = combo_breaks(mode, counts)`(모드를 `Mode::ALL` 에서 이름으로 못 찾으면 `log.minbp`), `avgjudge` = `log.avgjudge`(`UNSET_MINIMUM` 이면 0), `fast`·`slow` = 0(기록 안 됨) |
| `max_combo` / `total_notes` / `minbp` / `gauge_value` | `log.combo` / `log.notes` / `log.minbp` / `log.gauge_value` |
| `passnotes` | PG~POOR 합, `log.notes` 로 상한 |
| `options.gauge` | `ir_gauge(gauge_from_name(gauge_token(log.gauge)))` |
| `options.random` | `ir_random(NoteOption::from_str(random_token(log.random)))` |
| `options.scratch_auto` / `options.assist` | assist 1 → true / `["AUTO_SCRATCH"]`, assist 2 → false / `["CUSTOM_JUDGE"]`, assist -1(`ASSIST_UNRECORDED`) → false / 새 상수 `UNRECORDED_ASSIST_FLAG = "UNRECORDED_ASSIST"`(서버는 `AUTO_SCRATCH` 밖의 태그를 `ASSIST` 로 unranked 처리한다, `ranked-policy.ts` 33~36행), 0 → false / `[]` |
| `options.lntype` | `ln_mode` 가 `LN`/`CN`/`HCN` 이면 `ln_mode_from_token(..).id()`, `CHART` 면 0(확인 못함: 라이브러리에 `#LNMODE` 가 없다. CN·HCN 을 선언한 차트의 옛 판은 서버에 LN 으로 적힌다 — 알려진 한계로 문서에 적는다) |
| `options.judge_rate` | 100(넓힌 판정 판은 `CUSTOM_JUDGE` 태그로 이미 unranked) |
| 나머지 옵션 | 0·false·빈 문자열(`input_device` 빈 문자열) |
| `played_at` | `replay_file` 이 `<8자>-<숫자>.ron` 이고 `숫자 div 1000 == log.date` 면 그 숫자(밀리초 정확), 아니면 `log.date * 1000`(서버 1.2 의 같은 초 멱등으로 중복을 막는다) |
| `seed` | `log.seed` 가 음수(`SEED_UNRECORDED`)면 0 |
| `client` | `concat!("rbms/", env!("CARGO_PKG_VERSION"))` |
| `client_build_sha256` / `client_platform` | None / None(그 판을 친 빌드를 모른다) |
| `judge_algorithm`·`rule`·`skin` | 빈 문자열 |
| `extra` | `{"reconstructed": true}`(기존 `extra` 자유 칸. 서버 스키마 변경 없음) |

3. sha256 이 비면(RON 에서 옮겨 온 행) `ChartFacts.path` 의 차트를 `rbms_parser` 로 다시 읽어 `model.md5 == log.md5` 일 때 `model.sha256` 을 쓰고 `Backfilled` 를 보낸다(메인이 `backfill_sha256`). 경로가 없거나 md5 가 다르거나 읽기 실패면 `Skipped{ code: "NO_CHART_HASH", reason: "chart not in the library" }`(다음 동기화에서 다시 본다).

#### 3.3.6 메인 쪽 상태 — `apps/rbms-player/src/lib.rs`(`AppShared` 필드), `record_sync.rs`

- 필드: `record_sync: Option<RecordSyncRun>`(`rx`, `cancel`, `server_key`, `login_id`, 진행 수, 집계), `record_sync_pending: Vec<i64>`, `record_sync_states: HashMap<i64, IrSyncRow>`(현재 서버의 상태, 모달용).
- `AppShared::start_record_sync(&mut self, scope: SyncScope)`, `enum SyncScope { All, Logs(Vec<i64>) }`:
  - 서버 URL 이 없거나 로그인 안 했거나 서버가 토큰의 주인(`session.login_id()`)을 아직 말해 주지 않았으면 하지 않는다(게스트 금지). 설정 화면에서 입력한 PLAYER ID 는 계정으로 쓰지 않는다: 토큰의 주인과 다르면 모든 판이 403 으로 거절된다(리뷰 반영). 수동 버튼(`sync_records_now`)은 `set SERVER URL first`·`log in first`·`account not confirmed yet` 을 띄운다.
  - 주 서버가 legacy(`SERVER URL`) 서버일 때만 한다. `ir_profiles` 로 동기화하지 않는다(범위 밖).
  - 이미 돌고 있으면: `All` 은 `net_status = "records sync already running"`, `Logs` 는 `record_sync_pending` 에 더한다.
  - `All` 이면 `mark_old_scratch_auto_skipped(server_key, now)` → `sync_candidates(server_key, login_id, None)` → `replay_candidates(server_key, login_id)` → `sync_states` 로 화면이 읽는 상태를 다시 읽는다. `Logs` 이면 `sync_candidates(server_key, login_id, Some(ids))` 만 한다(Esc 로 나가는 길의 비용을 줄이려고. 화면이 읽는 상태는 답이 적힐 때마다 갱신되므로 다시 읽지 않는다). 결과 화면의 전송이 아직 들고 있는 판(`submit_log`)은 양쪽 모두 대상에서 뺀다: 그 전송이 자기 답을 적고, 같은 판을 둘이 겹쳐 보내면 답이 서로 덮는다(리뷰 반영). → 라이브러리 표 → `spawn_record_sync(self.server.clone(), job, cancel)`.
  - 이 선정은 프레임 스레드에서 한다. 잰 값(릴리스, 판마다 본문 2.5KB, 따뜻한 캐시): 5천 판 `sync_candidates` 25ms, 2만 판 100ms. `Logs` 는 판 하나를 고르는 데 0.07ms 다.
  - 대상이 0 이면 일꾼을 띄우지 않는다(자동 동기화는 조용히, 수동은 `records: nothing to send`).
- `poll_record_sync`: 사건별로
  - `Score` → `put_sync_state`(state·score_id·오류 열·`last_attempt_at = now_ms`·`login_id`·`replay_state`), `record_sync_states` 갱신, 집계.
  - `Replay` → `put_replay_state`, 갱신, 집계.
  - `Backfilled` → `backfill_sha256`.
  - `Finished` → 요약을 `net_status` 와 `notify(Level::Info, …)`(거절·실패가 있으면 `Level::Warn`)로. `record_sync_pending` 이 있으면 `Logs` 로 다시 시작.
  - 수신기가 끊기면 `Finished{Unreachable("records sync worker stopped")}` 와 같게.
- 취소: `finish_logout`(`app_network.rs` 162행)과 `rebuild_server`(76행)에서 `cancel` 을 세우고 `record_sync = None`. 이미 받은 사건은 그 계정의 정당한 결과라 버리지 않는다(취소 직전 프레임까지 처리한 것만 남는다).
- 시작할 때 DB 를 연 직후와 로그인·서버 변경 때 `record_sync_states = db.sync_states(server_key)`.
- 필드는 `record_sync`, `record_sync_pending`, `record_sync_states`, `record_sync_pacing` 이다(`record_sync_last` 는 만들지 않았다. 마지막 결과는 `net_status` 와 메시지로 충분하다).

#### 3.3.7 자동 동기화 시점

- 시작: `poll_whoami` 의 `Ok` 분기(세션이 확인된 뒤, `app_network.rs` 320~326행)에서 `start_record_sync(SyncScope::All)`. 오프라인(`Err` 이 인증 실패가 아님)이면 하지 않는다.
- 로그인: `poll_auth` 의 성공 분기(293~304행, `AuthAction::Login`·`Register` 모두)에서 `rebuild_server` 뒤에 `start_record_sync(SyncScope::All)`.
- 수동: 설정 NETWORK 의 새 행(3.5).
- Esc 포기 판: `SyncScope::Logs(vec![log_id])`(3.6).

### 3.4 새 판 제출 경로가 같은 상태 열을 쓰게 — `app_result.rs`, `app_ranking.rs`, `scoredb_store.rs`

- `scoredb_store::record_finished_play` 가 `Option<i64>`(저장한 `scorelog.id`)를 돌려준다. 실패면 지금처럼 알리고 `None`.
- 저장 직후 `put_submission(log_id, serde_json::to_string(&sub), played_as)`(`played_as` = 로그인했으면 `session.login_id()`, 아니면 None). 실패는 `notify(Level::Warn, …)` 만 하고 판은 그대로 둔다(본문 없는 판은 재구성으로 간다).
- `shared.scores.push(record_of(&log))` 의 `log.id` 를 돌려받은 id 로 채워 `ScoreRecord.log_id` 가 들어가게 한다.
- `spawn_score_submit` 에 `log_id: Option<i64>` 와 `replay_plan: ReplayState` 를 넘기고 `AppShared` 에 `submit_log: Option<LiveSubmit>`(`log_id`, `server`, `login_id`, `replay_plan`)로 둔다. `replay_plan`: 리플레이 파일 없음 → `None`, `wire_carries_judge` 거짓 → `Skipped`, AUTO UPLOAD REPLAY 꺼짐 → `Pending`, 그 밖 → `Pending`.
- `poll_submit`(`app_ranking.rs` 203행)이 결과를 받으면 `submit_verdict` 로 `IrSyncRow` 를 만들어 `put_sync_state`. 리플레이: `outcome.replay` 가 `Some(Ok(id))` → `Uploaded`, `Some(Err)` → `upload_verdict`(Rejected/Failed), `None` 이고 점수 수락 → `replay_plan` 이 `Pending` 이면서 업로드가 시도되지 않은 이유가 unranked 이면 `Skipped`, 설정 꺼짐이면 `Pending` 그대로. `Unauthorized` 등 StopSync 분류는 라이브 판에서는 `Retry` 로 적는다.
- 라이브 제출이 끝나기 전에 다음 판이 제출돼 `submit_rx` 가 바뀌면 앞 판의 결과는 버려진다(지금과 같다). 그 판은 `ir_sync` 행이 없어 다음 동기화가 보낸다(서버 멱등이라 무해). 문서에 적는다.
- `ir_profiles` 로의 팬아웃(`spawn_profile_submits`)은 `ir_sync` 에 적지 않는다(`server` 가 다르다. 범위 밖).

### 3.5 설정 화면 수동 버튼 — `apps/rbms-player/src/settings_ui.rs`, `stage/settings.rs`, `stage/settings/catalog.rs`, `stage/settings/info.rs`, `app_network.rs`

- 새 `SettingId::SyncRecords`. 표(`settings_ui.rs` 448행 근처)에 `(SettingId::SyncRecords, "SYNC RECORDS NOW", ">")`.
- 위치: NETWORK 의 `SCORES` 묶음 첫 줄(`catalog.rs` 63행): `[SettingId::SyncRecords, SettingId::AutoUploadReplay, SettingId::Rivals, SettingId::IrProfiles]`.
- 설명(`catalog.rs` 설명 표): `"Sends every run kept on this machine that the score server has not taken yet, with its replay. It also runs at start and on sign-in while you are signed in."`
- Enter(`stage/settings.rs` 641행): `SettingId::SyncRecords => shared.sync_records_now()`. 이유를 말해야 하는 경우(서버 없음·로그인 안 함·계정 미확인·이미 도는 중·보낼 것 없음)를 한곳에 모았다.
- NETWORK 묶음이 스크롤 없이 한 화면에 다 보여야 해서(`layout_tests.rs` 의 `every_group_lists_its_rows_under_the_headings_of_its_sections`) 이 행을 더하며 SERVER 와 ACCOUNT 두 머리를 `SERVER AND ACCOUNT` 하나로 합쳤다.
- 값(`network_setting_line`): 도는 중이면 `"SENDING {done}/{total}"`(점수 단계) 또는 `"REPLAYS {done}/{total}"`(리플레이 단계), 아니면 `ACTION_VALUE`(`">"`).
- 상태 줄(`net_status`)은 기존 문체(소문자, `logged out`, `log in first` 와 같다)를 따른다:
  - 진행: `syncing records 12/340`
  - 끝: `records: 120 sent, 30 already on server, 5 rejected, 2 failed, 1 skipped; replays: 80 uploaded, 3 failed`(0 인 항목은 뺀다)
  - 멈춤: `records sync stopped: not signed in (401)` / `records sync stopped: server unreachable` / `records sync stopped: rate limited` / `records sync cancelled`
- 키 안내(`info.rs`)는 다른 실행 행(UPLOAD SETTINGS NOW)과 같게 둔다(확인 후 맞춤).

### 3.6 Esc 포기 판 저장 — `crates/rbms-play/src/session.rs`, `apps/rbms-player/src/stage/play/mod.rs`, `apps/rbms-player/src/app_result.rs`

권고: **결과 화면을 거치지 않고** 기록·전송한 뒤 바로 곡 선택으로 간다.
근거: (a) Esc 는 지금도 앞으로도 "빨리 나가는 키"이고, ESCAPE 설정(즉시·길게·두 번)이 이미 실수를 막는다(`mod.rs` 549~588행). (b) 결과 화면을 강제하면 나가는 데 한 번 더 눌러야 해 체감이 바뀐다. (c) 판의 결과는 기록 모달(R)과 IR 상태 줄로 볼 수 있다. (d) 원본은 결과 화면을 거친다(`BMSPlayer.stopPlay`) — 원본과 다른 점으로 `docs/acknowledge/reference-divergences.md` 에 적는다.

`crates/rbms-play/src/session.rs`:

```rust
impl PlaySummary {
    /// The summary of a run left before its end: every note it never reached counts as a late POOR the player let pass, and the run is FAILED.
    pub fn left_unfinished(&self) -> PlaySummary
}
```

- `unreached = total_notes.saturating_sub(total_judged)`. `counts[POOR] += unreached`, `late[POOR] += unreached`, `min_bp += unreached`, `total_judged = total_notes`, `clear_lamp = ClearType::Failed`, `failed = true`. 나머지(EX·콤보·게이지 값·fast/slow·avg)는 그대로. 인덱스는 `rbms_judge::Judge::Poor as usize` 로(매직넘버 금지).
- 원본은 남은 노트를 판정 수에 넣지 않고 minbp 에만 더한다(`BMSPlayer.java` 909행). 사용자 결정 (5) "남은 노트는 놓친 것(見逃し POOR)" 이 우선이다 — 원본과 다른 점으로 적는다. 이렇게 하면 PG~POOR 합이 정확히 `total_notes` 라 서버 검사(1.1)를 통과한다.

`stage/play/mod.rs` 의 `leave_run`(597행):

```text
esc 상태 초기화, PlayStop 소리, save_settings (지금 그대로)
practice → Back (그대로)
course_run → end_course (그대로. 코스 도중 Esc 는 이번 범위 밖, 기록하지 않는다)
session.all_notes_resolved() 이거나 장면이 이미 닫히는 중(실패·끝) → enter_result (방어. 장면이 닫히는 중에 Esc 를 누르면 결과 화면으로 가므로, Esc 길게 누르기가 그 사이에 끝나도 같은 길로 보내 같은 판이 같은 기록으로 남게 한다: 리뷰 반영)
기록 대상(사람이 친 판: !shared.run_plays_itself() && shared.replay.is_none()) 이고 summary.total_judged > 0
    → crate::app_result::record_left_run(self, shared)
shared.leave_play() (그대로)
```

- "판정된 노트가 하나도 없는 판" = `summary().total_judged == 0`(노트를 소비한 판정이 없다. 空POOR·9K 의 BAD 만 있는 판 포함). 저장하지 않는다.

`app_result.rs`:

- `enter_result` 의 "기록·리플레이 저장·제출 본문·전송" 부분을 `fn record_run(state: &PlayState, shared: &mut AppShared, summary: &PlaySummary) -> RecordedRun`(lamp, log_id, sub, replay_file, recorded_replay, block_reason, scores_count, custom_judge, scratch_auto 를 담는다)로 뽑는다. `enter_result` 는 `record_run(state, shared, &play.summary())` 를 부르고 나머지(결과 화면·코스 진행)는 지금 그대로. 기존 결과 화면 테스트의 출력이 바뀌지 않아야 한다.
- `pub(crate) fn record_left_run(state: &mut PlayState, shared: &mut AppShared)`:
  1. `let summary = state.session.summary().left_unfinished();`
  2. `record_run(state, shared, &summary)` — 램프는 `assisted_lamp(Failed, assist)`(FAILED 유지인지 구현자가 확인), 리플레이는 AUTO REPLAY 규칙 그대로 저장, 기록은 `updates_score` 그대로.
  3. 라이브 제출(`spawn_score_submit`)은 하지 않는다(결과 화면이 없어 상태를 보여줄 곳이 없고, 다음 판의 `submit_rx` 와 겹치지 않게). 대신 로그인 상태면 `start_record_sync(SyncScope::Logs(vec![log_id]))`. 게스트면 다음 로그인 동기화가 보낸다. `spawn_profile_submits(&sub)` 는 부른다(IR 프로필 팬아웃은 지금 모든 판에 하는 동작).
  4. `notify(Level::Info, "run left mid-song kept as FAILED")`.
  5. `keep_run_record`·`dump_timing_csv`·결과 화면은 하지 않는다.
- 잡은 롱노트를 놓고 나간 경우: plain LN 은 끝에서 한 번 세고 CN·HCN 은 머리와 끝을 따로 센다(`matcher.rs` 703~710행). 어느 쪽이든 아직 세지 않은 몫만 `unreached` 에 들어가야 한다 — 테스트로 `counts[..=POOR] 합 == total_notes` 를 확인한다.

리플레이: 포기한 판의 리플레이는 나간 시점까지의 입력이다. 재생하면 남은 노트가 그대로 지나가 見逃し POOR 가 되므로 판정 수는 기록과 같다. 다만 재생은 곡 끝까지 가고 게이지가 기록과 다르게 끝날 수 있다(쉬운 게이지면 클리어로 끝날 수도 있다) — 알려진 한계로 `docs/play-options.md` 에 적는다.

### 3.7 판별 거절 이유 표시 — `crates/rbms-render/src/select.rs`(`SelectModal`, `render_select_modal`), `apps/rbms-player/src/stage/select/scene.rs`

- `SelectModal` 에 `pub ir_line: Option<(String, Color)>` 를 더하고 모달 아래쪽(리플레이 안내 위)에 한 줄 그린다. 스킨 선곡도 같은 `render_select_modal` 을 쓴다(`stage/select/overlay.rs` 222~224행).
- `scene.rs` 234행 모달 조립에서 `r.log_id` 로 `shared.record_sync_states` 를 찾는다. 문구(영어 대문자, `ir_outcome::truncate` 로 56자):

| 상태 | 문구 | 색 |
| --- | --- | --- |
| `accepted` | `IR: ON SERVER`. 지금 로그인한 계정과 다른 계정으로 받은 판이면 `IR: ON SERVER (AS <계정>)`(`Info`) | `IrLineKind::Good` |
| `rejected` | `IR: REJECTED {code}: {field} ({reason})`(필드·이유 없으면 `IR: REJECTED {code}`, 코드도 없으면 `IR: REJECTED (HTTP {status})`) | `Bad` |
| `retry` | `IR: NOT SENT YET (WILL RETRY)` | `Warn` |
| `skipped` + `OLD_SCRATCH_AUTO_RULE` | `IR: NOT SENT (OLD SCRATCH AUTO RULE)` | `Info` |
| `skipped` + `NO_CHART_HASH` | `IR: NOT SENT (CHART NOT IN LIBRARY)` | `Warn` |
| 행 없음, 로그인 | `IR: NOT SENT YET` | `Info` |
| 행 없음, 게스트 | `IR: NOT SENT (NOT SIGNED IN)` | `Info` |
| 서버 URL 없음 | 줄 없음(`None`) | — |

리플레이 줄은 더하지 않는다(모달 높이). 리플레이 거절은 동기화 요약 줄에만 나온다.

### 3.8 문서 정정

- `apps/rbms-player/src/scoredb_store.rs` 19~21행의 모듈 문서("The module is complete but not yet called …")를 지운다. 실제로 `app_result.rs` 263행과 `lib.rs` 888행에서 부른다.
- `docs/play-options.md`: 10.2 첫 문단의 "나중에 로그인해도 지난 판을 다시 보내지 않는다"와 표의 `도중 포기(Esc)` 행(444행)을 새 동작으로, 10.4 의 4항 "점수 서버로 소급 전송하지 않는다"를 "세대 2 SCRATCH AUTO 판은 동기화에서도 보내지 않는다"로 고치고, 새 절 "기록 동기화"(대상·시점·상태·화면 문구·재구성 한계: fast/slow·옵션·빌드 해시·`CHART` lntype·밀리초)와 결과 화면 오류 줄 형식(133행 근처)을 더한다.
- `README.md` 194행 "Until you sign in, runs are kept locally and not sent, and they are not sent later." 를 "… and they are sent when you sign in (SYNC RECORDS NOW sends them on demand)" 취지로 고친다. 26행은 그대로 맞다.
- `docs/architecture.md` 103행(Esc KNOWN ISSUE, `ir_submitted`)과 `docs/development.md` 83행(`scoredb.sqlite` 표)에 새 테이블과 Esc 동작을 반영한다.
- `docs/acknowledge/reference-divergences.md` 에 "Esc 포기: 결과 화면 없이 저장, 남은 노트는 見逃し POOR" 행을 더한다.

## 4. 파일 소유권

겹치는 파일 없음. 상대 소유 파일은 읽기만 한다. 커밋·푸시는 메인.

| 구현자 | 쓰는 파일 |
| --- | --- |
| 서버 | `web/src/server/service/domain/score/score.service.ts`, `web/src/server/route/score.ts`, `web/src/server/compose/index.ts`, `web/src/server/service/domain/replay/replay.service.ts`, `web/src/server/compose/replay.ts`, `web/src/server/route/replay.ts`, `web/src/server/lib/api-response.ts`, `web/src/server/lib/parse-request.ts`, `web/tests/service/score-submit.test.ts`, `web/tests/service/replay.test.ts`, `web/tests/lib/error-and-response.test.ts`, `web/tests/lib/parse-request.test.ts`, `docs/backend/api-spec.md`(§0.1 오류 봉투의 프로덕션 details 규칙, §5 멱등·판정 수 규칙, §8 리플레이 200) |
| 클라이언트 | `crates/rbms-ir/**`, `crates/rbms-store/**`, `crates/rbms-play/src/session.rs`(필요하면 그 테스트 파일), `crates/rbms-render/src/select.rs`(와 `render_select_modal` 이 있는 파일), `apps/rbms-player/**`, `docs/play-options.md`, `README.md`(IR 문단), `docs/architecture.md`(103행 문단), `docs/development.md`(83행 표), `docs/acknowledge/reference-divergences.md` |
| 메인 | `docs/PROCESS.md`, ADR(`docs/acknowledge/`: Esc 포기 판·기록 동기화), 이 문서의 갱신, 커밋·푸시·CI 확인, 배포 순서(2.3) |

## 5. 테스트 계획

### 5.1 서버

1.6 의 테스트. 실행(`web/`):

```bash
bun test tests/service/score-submit.test.ts tests/service/replay.test.ts tests/lib/error-and-response.test.ts tests/lib/parse-request.test.ts
bun run verify
```

### 5.2 클라이언트

| 크레이트 | 위치 | 테스트 이름 |
| --- | --- | --- |
| rbms-ir | `crates/rbms-ir/src/error.rs` tests | `a_validation_envelope_names_its_code_field_and_reason`(본문 `{"success":false,"error":{"code":"VALIDATION_ERROR","message":"요청 형식이 올바르지 않습니다.","details":{"field":"judge","reason":"judge count exceeds total_notes"}}}` 의 `Server(400, …)` 표시가 `server error 400: VALIDATION_ERROR: judge (judge count exceeds total_notes)`), `an_envelope_with_a_field_only_names_the_code_and_field`(`… VALIDATION_ERROR: played_at`), `an_envelope_without_details_keeps_the_message_and_code`(`server error 400: 요청 형식이 올바르지 않습니다. (VALIDATION_ERROR)`), `server_reason_reads_code_field_and_reason_from_the_envelope` |
| rbms-ir | `crates/rbms-ir/src/sync.rs` tests | `a_duplicate_submission_is_accepted_as_already_on_server`, `a_validation_error_is_a_rejection_with_its_field`, `a_rate_limited_submission_is_neither_accepted_nor_rejected`, `network_and_server_errors_are_transient`, `an_unauthorized_submission_stops_the_sync`, `an_unranked_acceptance_warrants_no_replay` |
| rbms-ir | `dto/score/tests.rs` | `is_duplicate_matches_the_server_message_exactly` |
| rbms-store | `crates/rbms-store/src/scoredb/tests.rs`(또는 새 `scoredb/sync_tests.rs`) | `a_generation_one_database_gains_the_sync_tables_and_keeps_its_rows`, `an_old_ir_submitted_flag_is_not_read_as_accepted`, `sync_candidates_leave_out_accepted_rows_and_old_scratch_auto_runs`, `sync_candidates_leave_out_runs_played_as_another_account`, `rejected_retry_and_skipped_rows_are_candidates_again`, `sync_state_upsert_replaces_the_row_and_counts_attempts`, `a_server_change_keeps_its_own_states`, `replay_candidates_are_accepted_rows_with_a_replay_still_to_send`, `the_submission_body_round_trips`, `history_reads_back_the_log_id`, `backfilling_a_sha256_fills_only_an_empty_one`, `old_scratch_auto_rows_are_marked_skipped_once` |
| rbms-play | `crates/rbms-play/src/session.rs` tests | `a_run_left_unfinished_counts_every_unreached_note_as_a_late_poor_and_fails`, `a_run_left_unfinished_keeps_its_score_combo_and_gauge` |
| rbms-player | `apps/rbms-player/src/record_sync.rs` tests(가짜 `ScoreServer`, `SyncPacing` 0) | `a_stored_body_is_sent_under_the_signed_in_account`, `a_row_without_a_body_is_rebuilt_from_the_log`(#52 값: total 1835, PG~BD 합 1815, POOR 20, MISS 43 → 본문 `poor 20, miss 43, empty_poor 43`), `a_rebuilt_row_takes_its_millisecond_stamp_from_the_replay_file_name`, `a_rebuilt_row_without_a_replay_uses_the_second_stamp`, `an_unknown_assist_is_sent_as_an_unrecorded_assist`, `a_row_with_no_sha256_is_filled_from_the_library_or_skipped`, `a_replay_goes_up_after_its_score_with_the_score_id`, `a_missing_replay_file_is_marked_missing`, `a_rate_limit_is_waited_out_and_the_same_run_sent_again`, `a_second_rate_limit_on_one_run_stops_the_sync`, `three_transient_failures_in_a_row_stop_the_sync`, `unauthorized_stops_the_sync_and_keeps_the_rest_for_next_time`, `cancelling_ends_the_sync_between_runs` |
| rbms-player | `apps/rbms-player/src/app_result/record_tests.rs` | `leaving_mid_song_keeps_a_failed_run_with_the_rest_as_poor`, `leaving_before_any_note_is_judged_keeps_nothing`, `leaving_while_holding_a_long_note_counts_it_once`, `leaving_a_watched_replay_or_autoplay_keeps_nothing`, `leaving_mid_course_still_ends_the_course_unrecorded`, `a_finished_run_stores_its_submission_body`, `a_live_submission_writes_its_sync_state` |
| rbms-player | `apps/rbms-player/src/ir_outcome.rs` tests | `a_validation_error_names_its_field_on_the_result_line`(줄 = `IR ERROR: server error 400: VALIDATION_ERROR: judge (judge count e...`) |
| rbms-player | `stage/settings` 테스트 파일들 | `the_scores_section_starts_with_sync_records_now`, `sync_records_now_while_signed_out_says_log_in_first` |
| rbms-player | `stage/select` 테스트 | `the_record_modal_says_why_the_server_refused_the_run`, `the_record_modal_of_a_run_on_the_server_says_so` |
| rbms-player | `app_network` 테스트(있는 곳) | `a_confirmed_session_at_start_starts_the_records_sync`, `a_successful_login_starts_the_records_sync`, `a_guest_never_starts_the_records_sync`, `logging_out_cancels_a_running_sync` |

실행:

```bash
cargo test -p rbms-ir --locked
cargo test -p rbms-store --locked
cargo test -p rbms-play --locked
cargo test -p rbms-player --locked record_sync
cargo test -p rbms-player --locked app_result
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
```

메인의 통합 게이트는 `cargo test --workspace --no-fail-fast --locked` 와 `git diff --check`(`docs/development.md` 16~18·163행), 푸시 뒤 3 OS CI.

수동 확인(메인, 2.3 배포 뒤): 사용자 DB 의 읽기 전용 사본에서 `sync_candidates` 수와 sha256 빈 행 수를 세어 이 문서의 "확인 못함"을 채운다(`RBMS_SCOREDB_COPY` 패턴, `docs/development.md` 71행). 운영 서버에는 GET 만.

## 6. 열린 질문

1. 실패 화면에서 START/SELECT 로 바로 다시 친 판(`stage/play/mod.rs` 1252·1268행 → `app_result::retry`)은 지금 기록되지 않는다(결과 화면을 거치지 않는다). "사람이 친 판은 모두 저장" 원칙과 다르다. — **A안(추천: Esc 와 같은 경로로 FAILED 로 저장·동기화. 게이지가 비어 끝난 판이라 남은 노트는 지금 실패 판처럼 세지 않고 결과 화면 경로와 같은 summary 를 쓴다)** / B안: 이번 범위 밖으로 두고 KNOWN ISSUE 로 적는다.
