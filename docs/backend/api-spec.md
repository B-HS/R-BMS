# rbms 백엔드 — HTTP API 규격 (IR 슈퍼셋)

> 최종 갱신 2026-10-22 · 대응 커밋 47a7f9c (2026-10-22 에 응답의 `assist_grade` 와 순위 키만 고쳤다. 나머지는 2026-10-15 기준)

> **현재 구현 기준:** 이 문서는 `web/src/app/api`와 `web/src/server`를 따라 읽습니다. 코드와 다르면 코드(`web/src/server/dto/*.ts` 의 zod 스키마)가 맞습니다. 초기 설계에 남은 `/api/settings/*`, `?envelope=1`, 외부 CORS, LR2IR 어댑터 설명은 아래의 구현 상태 표기를 우선합니다. 운영 배포(https://bms.hyuns.uk, Vercel `rbms-web`)는 2026-10-14 에 끝났고, 운영 DB 에는 마이그레이션 0000~0011 이 모두 적용돼 있습니다(0003·0004 는 2026-10-15, 0011 어시스트 등급 열은 2026-10-22). 2026-10-15 관찰 시 운영 웹은 `51f3a1e`(`/api/version`)이며 그 뒤 커밋은 `web/` 을 바꾸지 않았습니다. 현재 배포 상태는 [../web/architecture.md](../web/architecture.md) 6-2 절이 정본입니다.

> REST + JSON. 모든 시각은 **unix epoch ms**(레퍼런스 구현 IR은 초 단위 — [compatibility.md](./compatibility.md) 변환표). 차트 키는 **md5(32hex) 또는 sha256(64hex)** 둘 다 허용(`{hash}` 길이로 판별).
> 인증: `Authorization: Bearer <token>`(선택). 미인증 제출은 서버 설정상 `guest` 허용 가능.
> 버전: 모든 제출 본문에 `api_version`(현재 **1**). 모든 DTO는 `extra?: object`(free-form) 보존.
> URL 접두사: `/api`(코어). 레거시 LR2IR 어댑터는 `/lr2ir/*`(§11).

## 0. 공통

### 0.1 응답 봉투
백엔드 컨벤션(`backend.md §8`)의 헬퍼를 사용한다.
```jsonc
// 성공
{ "success": true, "data": { /* ... */ } }
// 페이지네이션
{ "success": true, "data": [ /* ... */ ], "pagination": { "page":1, "limit":50, "total":1234, "totalPages":25 } }
// 실패
{ "success": false, "error": { "code": "BLOG_POST_NOT_FOUND", "message": "...", "details": { } } }
```
> 네이티브 IR 경로는 `rbms-ir`가 파싱하는 **raw JSON**이고, FE 전용 경로(`/api/fe/*`)만 `{success,data,pagination?}` envelope를 반환합니다. `?envelope=1`과 Accept 협상은 구현하지 않았습니다.

`details` 는 비프로덕션(`NODE_ENV !== 'production'`)에서만 통째로 실린다. 프로덕션에서는 **문자열인 `details.field`·`details.reason` 만** 남기고 나머지(Zod `issues`, 경로, 플레이어 id 등 입력이 섞일 수 있는 값)는 숨긴다. 필드 이름과 고정 이유 토큰은 비밀이 아니고, 클라이언트가 결과 화면에 `VALIDATION_ERROR: judge (judge count exceeds total_notes)` 처럼 거절 이유를 보여 주려면 필요하다(구현 `publicErrorDetails`, `web/src/server/lib/api-response.ts`). 둘 다 없으면 `details` 키 자체가 없다.
```jsonc
// 프로덕션의 400 예
{ "success": false, "error": { "code": "VALIDATION_ERROR", "message": "요청 형식이 올바르지 않습니다.",
  "details": { "field": "judge", "reason": "judge count exceeds total_notes" } } }
```
본문·쿼리의 스키마(Zod) 검증 실패는 `details = { field, reason, issues }` 이고, `field` 는 첫 이슈의 경로(`judge.poor`, 경로가 비면 `body`), `reason` 은 그 이슈의 메시지다(`web/src/server/lib/parse-request.ts`). 라우트가 직접 던지는 검증 오류는 `field` 만(`played_at`, `chart.sha256`, `client_build_sha256`) 또는 `field`+`reason`(`judge`)을 싣는다.

### 0.2 상태코드 / 에러코드
| HTTP | 상황 | code(예) |
|---|---|---|
| 200/201 | 성공 | — |
| 400 | DTO 검증 실패 / 지원하지 않는 `api_version` / 판별 trace 가 연결한 점수 행과 다름(`POST /api/charts/[hash]/traces`, `details = { field: "judge.poor" 등, reason: "score 25, trace 0" }`, 규칙은 [../web/architecture.md](../web/architecture.md) 13-1) | `VALIDATION_ERROR` · `IR_API_VERSION_UNSUPPORTED` · `IR_TRACE_MISMATCH` |
| 401 | 토큰 없음/무효 | `UNAUTHORIZED` |
| 403 | 권한 없음 | `FORBIDDEN` |
| 404 | 리소스 없음 | `IR_CHART_NOT_FOUND`·`IR_PLAYER_NOT_FOUND`·`IR_SCORE_NOT_FOUND`·`IR_REPLAY_NOT_FOUND`·`IR_SETTING_NOT_FOUND`·`IR_COURSE_NOT_FOUND`·`IR_TABLE_NOT_FOUND`·`IR_FOLDER_NOT_FOUND`(그 밖에 일반 `NOT_FOUND`) |
| 409 | 충돌 | `IR_ACCOUNT_EXISTS`(중복 계정) · `IR_FOLDER_NAME_TAKEN` · `IR_FOLDER_LIMIT_EXCEEDED` · `IR_FOLDER_CONFLICT`(폴더 통째 저장의 `base_updated_at` 불일치). 설정 blob 의 낙관적 잠금 충돌은 에러 봉투가 아니라 `{ conflict, server }` 본문의 409 다(§9) |
| 413 | 본문 초과(점수·코스·설정·리플레이) 또는 저장 한도(리플레이 개수·총 바이트 §8, 설정 이름 수 §9) | `IR_PAYLOAD_TOO_LARGE` (비프로덕션 응답의 `details.scope` 가 `replay_count`·`replay_bytes`·`setting_names` 면 저장 한도다. 운영 응답은 `field`·`reason` 외의 `details` 를 싣지 않으므로 클라이언트는 413 한 가지로 다룬다) |
| 429 | rate-limit | `RATE_LIMITED` |
| 500 | 원인을 알 수 없는 서버 오류. 가입·로그인에서는 DB 끊김, 인증 설정 누락처럼 계정 문제가 아닌 실패가 여기로 온다 | `INTERNAL_ERROR` (원인은 서버 로그에만 남는다) |
| 503 | 미구성 서비스 | `SERVICE_NOT_CONFIGURED` |
> 에러는 `web/src/server/lib/error-code.ts`·`error-message.ts`·`error.ts` 3파일 중앙화(컨벤션 §6). 도메인 접두사 `IR_*`. 코드 목록의 정본은 `error-code.ts` 다.

### 0.2.1 웹 전용 코드 (게임 클라이언트는 받지 않는다)

위 표는 게임 클라이언트가 쓰는 IR 경로의 코드다. 웹 화면과 관리자 스크립트만 쓰는 경로(`/api/fe/me/ai/*`, `/api/admin/chart-patterns/*`)의 코드는 아래다. 같은 봉투(`{ success: false, error: { code, message, details? } }`)이고 `details` 는 프로덕션에서 `field`·`reason` 만 나간다. 정본은 `error-code.ts`·`error.ts`(`STATUS_MAP`).

| HTTP | 코드 | 상황 |
|---|---|---|
| 400 | `IR_CHART_PATTERN_INVALID` | 곡별 패턴 업로드(`PUT /api/admin/chart-patterns/[md5]`)의 경로 md5 와 본문 md5 가 다름 `details = { field: "md5", reason: "md5_mismatch" }`. 본문 형식·상한(24시간, 노트 20만 개) 위반은 `VALIDATION_ERROR` |
| 400 | `AI_CREDENTIAL_INVALID` | AI 키가 맞지 않거나 취소됨 |
| 402 | `AI_PROVIDER_QUOTA` | AI 공급자 계정의 잔액·지출 한도 |
| 403 | `AI_PROVIDER_FORBIDDEN` | 공급자가 이 키의 요청을 막음(키 권한·모델 접근) |
| 404 | `AI_CREDENTIAL_NOT_FOUND` | 등록된 AI 키가 없음 |
| 422 | `AI_NOT_ENOUGH_DATA` | 해석할 기록이 모자라 AI 를 부르지 않음. `details.need` 가 모자란 것(`table_records`·`candidates`·`recent_plays`·`week_plays`·`goal_reached`·`remaining_charts`·`chart_patterns`·`table`) |
| 422 | `AI_MODEL_UNAVAILABLE` · `AI_PROVIDER_REFUSED` | 고른 모델이 없어짐 · 모델이 답을 거절함 |
| 429 | `AI_PROVIDER_RATE_LIMITED` · `AI_USAGE_LIMIT` | 공급자가 잠시 막음(`details.retry_after_seconds`) · 서비스의 사용 한도 |
| 502 | `AI_PROVIDER_BAD_REQUEST` · `AI_PROVIDER_UNAVAILABLE` · `AI_OUTPUT_TRUNCATED` · `AI_OUTPUT_INVALID` | 공급자가 요청을 받지 않음 · 연결 실패 · 출력이 상한에서 잘림 · 출력 형식이 틀리거나 검증 뒤 남는 항목이 없음 |

관리자 경로의 인증·권한은 `UNAUTHORIZED`(토큰 없음·무효, 세션 쿠키만으로는 401)·`FORBIDDEN`(관리자가 아닌 사용자의 토큰), 한도는 `RATE_LIMITED` 이다.

### 0.3 enum 표기 (variant 이름 그대로 JSON 문자열)
- `ClearLamp`: `NoPlay|Failed|AssistEasy|LightAssistEasy|Easy|Normal|Hard|ExHard|FullCombo|Perfect|Max` (the reference implementation `ClearType` id 0-10).
- `GaugeType`: `AssistEasy|Easy|Normal|Hard|ExHard|Hazard|Class|ExClass|ExHardClass`.
- `RandomOption`: `Off|Mirror|Random|RRandom|SRandom|Spiral|HRandom|AllScratch|Converge`.
- `Mode`(차트): `BEAT_5K|BEAT_7K|BEAT_10K|BEAT_14K|POPN_9K|...`(rbms `Mode.name`).

---

## 1. system — health / version

### `GET /api/health`
연결 표시(초록/빨강) + capability 탐색. **무인증**.
```jsonc
// 200 ServerInfo
{ "name": "rbms-ir", "version": "0.1.0", "ir_compat": "superset-1",
  "capabilities": { "ranking": true, "player_best": true, "rivals": true,
                    "courses": true, "replays": true, "tables": true,
                    "settings_sync": true, "folders": true, "accounts": true, "lr2ir_compat": false } }
```
> `capabilities`는 `rbms-ir`의 `ServerCapabilities` 슈퍼셋(여기 `settings_sync`·`folders`·`accounts`·`lr2ir_compat` 추가). 클라가 모르는 키는 무시. 값의 정본은 `web/src/server/service/domain/system/capabilities.ts` 의 `SERVER_CAPABILITIES` 다.

### `GET /api/version` → `{ "api_version": 1, "server": "0.1.0", "commit": "abc1234" }`
`commit` 은 `SERVER_COMMIT` 또는 Vercel 의 `VERCEL_GIT_COMMIT_SHA` 앞 7자이고, 둘 다 없으면 `null` 이다(Git 연동 배포가 아니라는 신호). 운영 배포가 어느 커밋인지 보려면 `curl https://bms.hyuns.uk/api/version`.

---

## 2. auth — 계정 (레퍼런스 구현 IRAccount 슈퍼셋)

### `POST /api/auth/register`
```jsonc
// req  (accountSchema) id: ^[a-zA-Z0-9_-]+$ 1~64자, "guest"(대소문자 무시) 예약. password 8~128자. email 필수(유효한 주소, 255자 이하 = user.email 열). name 선택(1~64자, 웹 가입 화면과 같은 상한)
{ "api_version":1, "id":"alice", "password":"…", "email":"alice@example.com", "name":"Alice" }
// 201 → AuthResponse (authResponseSchema). token 은 새로 발급한 api_token 이고 세션 쿠키도 함께 세팅된다
{ "token":"<bearer>", "player": { "id":"alice" }, "name":"Alice" }
// 409 IR_ACCOUNT_EXISTS — 아이디나 이메일이 이미 있을 때만(better-auth 의 USER_ALREADY_EXISTS*, 또는 사용자 생성 실패 직후 같은 아이디가 생겨 있을 때)
// 400 VALIDATION_ERROR — 아이디 형식·예약어, 비밀번호 길이(8~128), 이메일 형식·길이(255), 이름 길이(64) (details.field). 이메일·이름 상한은 S19 4차 검증자 추가: 전에는 255자를 넘는 이메일이 DB 에서 막혀 500 이었다
// 500 INTERNAL_ERROR — 그 밖의 모든 실패(DB 끊김, BETTER_AUTH_SECRET 누락 등). 409 로 바뀌지 않고 서버 로그에 원인이 남는다
```
요청에 `Authorization: Bearer <이전 토큰>` 이 실려 있고 가입이 성공하면 그 이전 토큰은 같은 응답 안에서 폐기된다(아래 로그인과 같다).
### `POST /api/auth/login`
```jsonc
{ "api_version":1, "id":"alice", "password":"…" }   // password 1~128자 → 200 AuthResponse (위와 동일, 로그인마다 새 api_token 을 만든다)
// 401 UNAUTHORIZED — 모르는 아이디, 틀린 비밀번호(better-auth INVALID_EMAIL_OR_PASSWORD)만. 그 밖의 실패는 500 INTERNAL_ERROR 다
```
**이전 토큰 정리**(S19, 사용자 결정 2026-10-15): 요청의 `Authorization: Bearer <토큰>` 이 있고 로그인(또는 가입)이 성공하면 서버는 새 토큰을 발급한 뒤 그 토큰을 폐기한다. 같은 클라이언트가 로그아웃 없이 다시 로그인해도 쓰던 토큰이 쌓이지 않는다. 비밀번호가 틀린 요청은 아무 토큰도 건드리지 않고, 헤더가 없으면 아무것도 폐기하지 않는다. 같은 사용자의 다른 기기 토큰은 그대로다. 토큰 만료는 강제하지 않는다.
### `GET /api/auth/me`  (Bearer)
```jsonc
// 200 PlayerProfile (자기 자신). Bearer 또는 세션 둘 다 받는다
{ "id":"alice", "name":"Alice", "total_plays":1234, "rank_points":56.7, "extra":{ "rank":"" } }
```
### `DELETE /api/auth/token/current`  (Bearer) — 로그아웃: 보낸 토큰 하나를 폐기
```jsonc
// 204 (본문 없음). 그 토큰은 이후 모든 요청에서 401 이다. 같은 사용자의 다른 토큰은 그대로다
// 401 UNAUTHORIZED — 헤더가 없거나, 모르는 토큰이거나, 이미 폐기된 토큰
```
클라이언트는 로그아웃할 때 일꾼 스레드에서 이 요청을 보내고, 성공·실패와 무관하게 로컬 토큰을 지운다(오프라인이어도 로그아웃은 끝난다). 한도는 `/api/auth/*` 와 같은 5/min/IP 이지만 로그인·가입과 다른 버킷이다.
### `POST /api/auth/token`  (**세션**, Bearer 아님) — API 토큰 회전: 그 사용자의 기존 토큰을 전부 폐기하고 새로 발급한다 → `{ "token":"…", "created_at":1700000000000 }`. 본문 `{ "label"?: string }`. 웹 설정 화면은 폐기 없이 추가 발급하는 `POST /api/fe/tokens` 를 쓴다.
> better-auth 기반(컨벤션 §11). 레퍼런스 구현 `register`/`login`(IRConnection)에 1:1 대응. `guest` 제출은 토큰 없이 `player.id="guest"`(서버 설정 `ALLOW_GUEST`, 운영 기본값 `false`). 웹 화면의 로그인·가입은 토큰을 만들지 않는 `/api/fe/auth/*`(§12)를 쓴다.
>
> **오류 구분**(S19, 이전 KNOWN ISSUE 해소): 서비스는 better-auth 가 던진 `APIError` 의 `body.code` 만 보고 분류한다(`AUTH_PROVIDER_ERROR_CODE`; 코드 이름이 바뀌면 `tests/service/auth-login.test.ts` 의 "better-auth 오류 코드 이름" 테스트가 실제 better-auth 로 실패한다). 알 수 없는 원인은 그대로 던져 `withErrorHandling` 이 로그에 남기고 500 으로 답한다.
>
> **운영 필수 환경 변수**(S19): `NODE_ENV=production` 에서 `BETTER_AUTH_SECRET`·`BETTER_AUTH_URL`·`NEXT_PUBLIC_APP_URL` 이 비어 있으면 인증을 처음 쓰는 순간(`getAuth()`) `Missing required production environment variables: …` 로 실패한다(누락한 키를 모두 알려 준다). 환경 변수 없이 `next build` 가 성공해야 하는 조건 때문에 `parseEnv` 에서는 검사하지 않는다(`tests/env.test.ts`).

---

## 3. players — 프로필 / 라이벌 (IRPlayerData)

### `GET /api/players/{id}` → `PlayerProfile` (`playerProfileSchema`: `id`·`name`·`total_plays`·`rank_points`·`extra`. `id` 는 로그인 id 다. `total_plays` 는 그 사용자의 `score` 행 수를 센 값이고, `rank_points` 는 `user.rank_points` 열을 그대로 읽는데 이 열에 쓰는 코드가 없어 항상 0 이다. 웹 순위는 이 값을 쓰지 않는다)
```jsonc
{ "id":"alice", "name":"Alice", "total_plays":1234, "rank_points":56.7, "extra":{ "rank":"" } }
// 404 IR_PLAYER_NOT_FOUND
```
### `GET /api/players/{id}/rivals` → `PlayerProfile[]`  (the reference implementation `getRivals`)
### `PUT /api/players/{id}/rivals` (Bearer, 본인) — 라이벌 목록 설정
```jsonc
{ "rivals": ["bob","carol"] }   // → 200 PlayerProfile[]
```
> `getPlayerURL` 대응: `PlayerProfile.extra.url`에 웹 프로필 URL을 담아 반환 가능.

---

## 4. charts — 차트 메타 / 랭킹 / 베스트 (LR2IR getrankingxml 슈퍼셋)

`{hash}` = md5(32) 또는 sha256(64). 서버는 둘 다 인덱싱.

### `GET /api/charts/{hash}` → `ChartMeta` (IRChartData 슈퍼셋)
```jsonc
{ "md5":"32hex", "sha256":"64hex", "title":"", "subtitle":"", "genre":"", "artist":"", "subartist":"",
  "level":12, "total":280.0, "mode":"BEAT_7K", "lntype":1, "judge":3,
  "minbpm":150, "maxbpm":150, "notes":812,
  "has_ln":true, "has_cn":false, "has_hcn":false, "has_mine":false, "has_random":false, "has_stop":false,
  "url":null, "appendurl":null, "extra":{} }
// 404 IR_CHART_NOT_FOUND
```
### `POST /api/charts` (Bearer) — 차트 메타 업서트(클라가 보유 차트 등록; md5↔sha256 연결)
```jsonc
{ "api_version":1, "chart": ChartMeta }   // → 200 ChartMeta
```
> 본문 64KB, 문자열은 DB 열 길이(제목류 255, `mode` 16, `url` 512), `extra` 16KB 상한(초과는 400/413). 이미 있는 차트는 **빈 필드만 채우고 값이 있는 필드를 덮어쓰지 않는다**(토큰 하나로 메타를 망가뜨릴 수 없다). 현재 클라이언트는 이 경로를 부르지 않고 `POST /api/scores` 의 `chart.title/artist/level` 로 메타를 올린다. 일반 사용자 토큰으로 부를 수 있으므로 여기서 들어온 `notes`(그리고 점수 제출이 차트 행을 처음 만들 때 넣는 `total_notes`)는 표시용 메타일 뿐 어떤 제출 검증의 상한에도 쓰지 않는다(S19 2차 수리, 검증자 missed 1). `GET /api/charts?hash=` 는 쿼리 형태의 조회다(해시가 없으면 400).
### `GET /api/charts/{hash}/ranking?limit=50&page=1&rival_of={id}&lnmode=1&include_scratch_auto=false`
```jsonc
// 200 ScoreRecord[]  (rbms 모드 raw 배열, scoreRecordSchema)
[ { "player":{"id":"alice"}, "player_name":"Alice", "clear":"Hard", "assist_grade":0,
    "ex_score":1600, "max_combo":812, "minbp":1, "rank":1,
    "played_at":1700000000000, "lntype":1, "option":0, "total_notes":812,
    "judge":{ "epg":700,"lpg":12,"egr":50,"lgr":14,"egd":5,"lgd":16,"ebd":1,"lbd":7,"epr":2,"lpr":3,"ems":0,"lms":2,
              "pgreat":712,"great":64,"good":21,"bad":8,"poor":5,"miss":2,"fast":30,"slow":40,"combobreak":15,"empty_poor":9 },
    "extra":{ "replay_id":"rp_…" } } ]
```
- `extra.replay_id`(리플레이가 연결된 판만)와 `extra.scratch_auto: true`(SCRATCH AUTO 판 행만)가 `extra` 에 실린다. 플레이 옵션·시드는 `ScoreRecord` 에 없다(`GET /api/scores/{id}` 상세에서 본다).
- `assist_grade`(2026-10-22, 0~4 정수, 기본 0): `clear` 가 `LightAssistEasy` 일 때만 0 이 아닐 수 있고 SCRATCH AUTO 로 클리어한 게이지를 이름한다(1 ASSIST EASY ← EASY 게이지, 2 ASSIST CLEAR ← NORMAL·CLASS, 3 ASSIST HARD ← HARD·EXCLASS, 4 ASSIST EXHARD ← EX-HARD·HAZARD·EXHARDCLASS). 서버가 `clear` 와 `score.gauge` 로 파생한다(`web/src/server/service/domain/score/assist-grade.ts`). 코스 순위 행은 항상 0 이다. 같은 필드가 `GET /best`, `GET /api/players/{id}/scores`, 코스 순위·best, `GET /api/scores/{id}` 의 `ScoreRecord` 에 실린다(`scoreRecordSchema`, `dto/score.ts:77`). 제출 본문은 바뀌지 않았다. 구 클라이언트는 이 필드를 무시한다. 의미와 설계: [../reference/clear-lamps.md](../reference/clear-lamps.md) 1.8·3절.
- `judge` 의 합계(`pgreat`~`miss`)는 제출 검사와 EX·BP 가 읽은 값이다(S19 3차 수리, R3-WEB-4): 그 단계의 early/late 합(`epg+lpg` 등)이 0 이 아니면 그 합, 0 이면 저장된 합계다(`effectiveJudgeCounts`, `web/src/server/compose/judge-row.ts` 의 `toJudgeBreakdown`). 분리 필드는 저장한 그대로다. 차트 순위·best·상세(`GET /api/scores/{id}`)·플레이어 기록(`GET /api/players/{id}/scores`)·코스 순위·코스 best 가 모두 같다. 이전에는 차트 쪽만 저장된 합계를 그대로 돌려줘, 분리 필드와 다른 합계(검사를 거치지 않는 값)를 보낸 본문이 EX·BP 와 맞지 않는 판정 내역으로 보였다. 저장된 행은 바꾸지 않으므로 이미 들어온 그런 행도 응답에서 바로잡힌다.
- `rival_of={id}`: 그 플레이어의 라이벌만(+본인). `lnmode`: 0=LN,1=CN,2=HCN(레퍼런스 구현 `lntype`).
- `include_scratch_auto`(`true`|`false`, 기본 `false`, 웨이브 20): 기본 목록은 일반 판 최고 기록(`chart_best.scratch_auto=false`)만이다. `true` 면 플레이어마다 일반 판 최고와 SCRATCH AUTO 판 최고 가운데 더 높은 것(램프 → 등급 → EX → BP, 모두 같으면 일반 판) 한 행이다. SCRATCH AUTO 판 행은 `extra.scratch_auto: true` 를 단다. `rank` 는 그 목록 안의 순위다.
- **순위 번호와 동점**(S19): 정렬은 램프 → 어시스트 등급 → EX → BP(적은 쪽이 위)이고(2026-10-22 부터 등급이 램프 바로 다음 키다), 램프·등급·EX·BP 가 모두 같은 사람들은 **공동 순위**(`RANK()`: 1, 2, 2, 4)다. 같은 쪽 경계에서 동점자가 겹치거나 빠지지 않도록 마지막 정렬 키는 그 최고 기록을 먼저 세운 쪽이 앞(`chart_best.updated_at`), 그다음 사용자 id 다. 목록의 `rank`, 제출 응답의 `rank`, `GET /best` 의 `rank` 는 같은 정의다(이 판보다 램프·등급·EX·BP 가 엄격히 좋은 최고 기록의 수 + 1). 원본(beatoraja `RankingData`)은 EX 만으로 정렬하고 동점을 같은 순위로 세는데, R-BMS 의 램프 우선 정렬은 의도한 설계라 그대로 두고 BP 를 순위에 넣었다.
- **`lnmode` 필터의 뜻(현재 동작, 사용자 확인 대기)**: `chart_best` 의 키에는 LN 방식이 없어서 필터는 "최고 기록이 그 LN 방식으로 친 판인 플레이어"만 남기고, 순위는 그렇게 남은 사람들 안에서 센다. LN 방식별 최고 기록을 따로 두는 것(키 확장 + 마이그레이션)이나 파라미터 제거는 사용자 결정이 필요해 바꾸지 않았다. `GET /best?lnmode=` 도 같은 목록 기준으로 `rank` 를 센다.
- LR2IR 호환 필드(`pg=epg+lpg`, `gr=egr+lgr`, `notes`, `combo`, `minbp`)는 `judge`+상위 필드로 유도 가능.

### `GET /api/charts/{hash}/best?player={id}&lnmode=1&include_scratch_auto=false` → `ScoreRecord | null`  (레퍼런스 구현 getPlayData 본인분)
- `include_scratch_auto` 는 순위와 같다: 기본은 일반 판 최고, `true` 면 둘 중 더 높은 것. `rank` 도 같은 목록 기준이다.

---

## 5. scores — 제출 / 조회 (레퍼런스 구현 IRScoreData 슈퍼셋)

### `POST /api/scores` (Bearer 또는 guest)  — 레퍼런스 구현 `sendPlayData`
> **원칙**: 제출은 *플레이에 사용된 모든 정보*를 담는다(공평 평가). 채점·난이도에 영향을 주는 **모든 플레이 옵션 + 클라이언트 빌드 해시 + (선택)리플레이**를 함께 보내고 서버가 보존한다. 서버가 `ranked` 적격성과 무결성(빌드해시)을 직접 판정한다(§11).
```jsonc
// req ScoreSubmission (rbms-ir 슈퍼셋 — early/late + full play context + integrity)
{ "api_version":1,
  "chart": { "md5":"…", "sha256":"…", "title":"…", "artist":"…", "level":12 },   // title·artist·level 은 선택(아래)
  "player": { "id":"alice" },
  "mode":"BEAT_7K",
  "clear":"Hard",
  "ex_score":1488, "max_ex_score":1624,
  "judge": {
     "epg":700,"lpg":12, "egr":50,"lgr":14, "egd":5,"lgd":16,
     "ebd":1,"lbd":7,   "epr":2,"lpr":3,   "ems":0,"lms":2,
     "avgjudge":11800,                       // µs, 레퍼런스 avgjudge: 노트마다 입력과의 거리(놓친 노트 1초)의 평균. 0 이상, 작을수록 정확
                                             // extra.rule_version 이 없는 본문(옛 클라이언트)은 맞은 입력의 부호 있는 평균을 실었다
     "empty_poor":9,                         // rbms 空POOR(노트 미소실 빈 POOR)
     // 합계 미러(호환·검증용, 서버가 재계산 가능):
     "pgreat":712,"great":64,"good":21,"bad":8,"poor":5,"miss":2,
     "fast":30,"slow":40,"combobreak":15 },
  "max_combo":540, "total_notes":812, "passnotes":812, "minbp":15, "gauge_value":86.0,

  // ── 플레이에 사용된 전체 옵션(공평 평가의 단일 출처). rbms PlaySettings/PlayerConfig 전수. ──
  "options": {
     "gauge":"Hard", "random":"Random", "random_p2":null, "option":1,   // option=레퍼런스 구현 노트옵션 비트필드
     "lntype":1, "scratch_left":false, "scratch_auto":false,
     "hispeed":2.0, "constant":false, "green_number":300,               // 스크롤
     "lift":0.0, "lane_cover":0.0,                                      // 시야
     "judge_rate":100, "offset_ms":0, "auto_offset":false,             // 판정폭/오프셋 (난이도 영향!)
     "total_override":0.0,                                             // 0=차트값, >0=오버라이드 (게이지 영향!)
     "autoplay":false,                                                  // 오토플레이 여부 (랭킹 무효 트리거)
     "assist":[], "input_device":"keyboard" },
  "seed":12345, "judge_algorithm":"Combo", "rule":"BEAT", "skin":"NORMAL",   // 최상위 필드(options 안이 아니다)

  // ── 무결성(클라이언트 변조 방지) ──
  "client":"rbms/0.1.0",                    // 클라 표시 버전
  "client_build_sha256":"<64hex>",          // 실행 중인 클라 빌드 바이너리 SHA-256 (필수, 서버 로깅·검증 §11)
  "client_platform":"macos-universal",      // 빌드 타깃
  "replay_id":null,                         // 이미 업로드한 리플레이 연결(또는 제출 후 §8로 업로드)
  "played_at":1700000000000,                // unix ms
  "extra":{ "mean_offset_us":-1200,         // 맞은 입력의 부호 있는 평균(빠르면 양수), 결과 화면이 만든 본문에만
            "rule_version":4 } }            // 클라이언트 규칙 세대. 기록 동기화가 옛 행에서 다시 만든 본문은 그 행의 세대(4 미만이면 avgjudge 0)

// 201 SubmitResponse (같은 판의 재전송은 200)
{ "accepted":true, "ranked":true, "rank":3, "previous_best":1400, "is_new_best":true,
  "score_id":"sc_…", "flags":[], "message":"saved" }
// ranked=false 예: { "accepted":true, "ranked":false, "flags":["AUTOPLAY"], "message":"recorded (unranked: autoplay)" }
// SCRATCH AUTO 예: { "accepted":true, "ranked":true, "rank":2, "flags":["SCRATCH_AUTO"], "message":"saved (scratch auto)" }
```
규칙:
- **`chart.title`·`chart.artist`·`chart.level`**(모두 선택, 결정 M8): 서버가 만든 차트 행의 제목·아티스트·레벨이 비어 있을 때만 채운다. 값이 이상하면 제출을 막지 않고 버린다(문자열은 앞뒤 공백을 떼고 255자로 자름, `level` 은 0~1000 정수만). `GET /api/charts/{hash}` 등은 그 `md5` 가 번들 표에 있으면 읽을 때 표의 제목·아티스트로 빈 값을 채워 응답한다(DB 는 고치지 않음).
- **`seed`**(최상위, 선택): 클라이언트는 `u64` 로 보내고 예전 빌드는 시계의 나노초(2^53 초과)를 보냈다. 서버는 `Number.isSafeInteger` 가 아닌 값을 거절하지 않고 **0 으로 저장**한다(`seedSchema`, `web/src/server/dto/common.ts`. 리플레이 업로드의 `seed` 도 같다). 시드는 참고용이라 순위·무결성 판정에 쓰지 않는다. 이 규칙이 없던 d733a8f 이전 서버는 큰 시드의 제출을 400 으로 거절했다(v0.1.2 점수 제출 400 사고. 근거 [../acknowledge/2026-10-14-adr-layout-seed-range.md](../acknowledge/2026-10-14-adr-layout-seed-range.md)).
- **멱등**: 같은 사용자·같은 차트(sha256)에서 `played_at` 이 **같은 초**(`floor(played_at / 1000)` 이 같음)인 재전송은 기존 score_id 를 돌려주고 새로 만들지 않는다. 응답은 200, `message: "duplicate submission ignored"`(상수 `DUPLICATE_SUBMISSION_MESSAGE`. 클라이언트가 이 문자열로 "이미 있음"을 센다), `is_new_best: false`, `previous_best: null`. 초 단위로 넓힌 이유: 로컬 기록(`scorelog.date`)의 시각은 초 단위라, 리플레이 파일이 없는 옛 판을 다시 보내면 `played_at` 이 원래 밀리초와 다르다. 한 판은 1초보다 길어서 같은 사용자·차트의 서로 다른 두 판이 같은 초에 끝날 수 없다. 조회는 `score_idempotent_uniq(user_id, chart_sha256, played_at)` 의 범위 탐색이라 새 색인이 없고, 유일 색인은 그대로라 정확히 같은 밀리초의 동시 삽입은 DB 가 막는다. guest 제출은 멱등 조회를 하지 않는다. 근거: [../acknowledge/2026-10-14-adr-record-sync.md](../acknowledge/2026-10-14-adr-record-sync.md) 6항(사용자 결정 "서버 멱등"). 같은 판을 정확히 같은 밀리초로 **동시에** 두 번 보내면 둘 다 멱등 조회를 지나 한쪽이 유일 색인에 걸려 500 이다(KNOWN ISSUE, 테스트 부채 3.72).
- **재전송의 최고 기록 복구**(S19 4차 R4-WEB-2): 점수 행 저장(`insertScore`, autocommit)과 최고 기록 갱신은 따로 커밋된다. 그 사이가 실패하면(잠금 재시도 소진, 연결 끊김, 함수 종료) 첫 요청은 500 이고 점수만 남는다. 클라이언트는 5xx 를 일시 오류로 보고 다시 보내는데, 멱등 응답은 전에는 최고 기록을 건드리지 않아 그 판이 `chart_best` 에 영영 오르지 않았다. 지금은 멱등 응답 경로가 **저장된 판**의 램프·EX·BP·최대 콤보·`scratch_auto`·`ranked` 로 같은 조건부 갱신(`upsertChartBestIfBetter`)을 한 번 부른다(저장된 판이 ranked 일 때만). 갱신은 더 좋을 때만 바꾸므로 되풀이해도 결과가 같고, 이미 반영된 판이나 그 뒤의 더 좋은 판은 그대로다. 응답의 `rank` 는 그 갱신 뒤에, 저장된 판의 값과 칸(일반/SCRATCH AUTO)으로 센다(저장된 판이 unranked 면 `null`). `is_new_best: false`·`previous_best: null` 은 그대로다. 코스(`course_best`, `upsertCourseBestIfBetter`)도 같고, 코스의 멱등 응답도 이제 `rank` 를 센다(전에는 늘 `null`).
- **DB 열 범위**(S19 3차 수리, R3-WEB-1): 저장하는 값은 모두 DB 열 안이어야 한다. 넘으면 스키마 검증이 400 `VALIDATION_ERROR` 로 거절하고 `submission_audit` 에 `accepted=false` 로 남긴다. 거절은 차트 행을 만들기 전에 끝나므로, 거절된 본문은 어떤 행도 남기지 않는다(이전 서버는 범위 밖 값을 DB 가 막아 500 이었고, 처음 보는 차트면 점수 없는 차트 행이 먼저 남았다). 정상 클라이언트는 이 범위에 닿지 않는다.
  - INT 열(부호 있는 32비트, `INT_COLUMN_MAX = 2147483647`, `web/src/server/dto/common.ts`): 판정 수 전부(`pgreat`~`miss`, `epg`~`lms`, `fast`·`slow`·`combobreak`·`empty_poor`)와 `ex_score`·`max_ex_score`·`max_combo`·`total_notes`·`passnotes`·`minbp` 는 0 이상 그 이하, `options.lntype`·`options.judge_rate`·`options.offset_ms` 는 -2147483648 이상 그 이하. 코스 제출과 `POST /api/charts`(`level`·`lntype`·`judge`·`minbpm`·`maxbpm`·`notes`, 이제 아래쪽도)도 같다.
  - 서버가 셈해 저장하는 EX(PGREAT × 2 + GREAT)와 BP(BAD + POOR + 空POOR)도 INT 열 안이어야 한다(무결성 검사 6, `field: "judge"`). 필드마다 범위 안이어도 PGREAT 15억이면 EX 30억이 되고, pop'n 의 BAD 와 空POOR 는 노트 수에 묶이지 않는다.
  - FLOAT 열(`FLOAT_COLUMN_MAX = 3.402823466e38`): `gauge_value`, `options.lift`, `options.lane_cover`. `options.green_number` 는 반올림한 값이 INT 열 밖이면 거절하지 않고 `null` 로 저장한다(참고값).
  - `extra` 는 DB JSON 열의 깊이 100 안이어야 한다(스칼라나 빈 배열·객체가 한 단계. 모든 `extra` 스키마 공통). 키와 문자열에 짝 없는 UTF-16 서로게이트(`"\ud800"`)가 있으면 400 이다(MySQL JSON 이 받지 않아, 거절하지 않으면 500 이 나고 처음 보는 차트면 차트 행이 먼저 남았다. `fitsJsonColumn`). JSON 열에 들어가는 코스의 `assist` 태그, 코스 메타의 `constraint`·`trophy[].name` 도 같다(`jsonTextSchema`).
  - `extra` 의 객체 키는 UTF-8 로 65535 바이트(`JSON_KEY_MAX_BYTES`) 이하여야 한다(중첩 객체의 키 포함, 모든 `extra` 스키마 공통). 넘으면 400 이다(S19 4차 R4-WEB-1: MySQL 이 `ER_JSON_KEY_TOO_BIG`(3151)로 받지 않아 500 이었고, 처음 보는 차트면 차트 행이 먼저 남았다). 한도는 글자 수가 아니라 바이트라, 3바이트 글자 키는 21845자까지다.
  - 글자는 거절하지 않고 열 길이(코드 포인트)로 잘라 저장한다: `mode` 16(차트 행에도 같은 값), `options.input_device` 32, `judge_algorithm` 16, `rule` 16, `skin` 64, `client` 64, `client_platform` 32. `client_build_sha256` 은 64자 이하의 소문자 16진수(빈 값과 `null` 허용, 아니면 400), `replay_id` 는 36자 이하(아니면 400)다.
- **본문 무결성 검사**(순서대로 첫 실패만 400 `VALIDATION_ERROR`, `details: { field, reason }`, 거절은 `submission_audit` 에 `accepted=false` 로 남는다). **본문만 본다**(S19 2차 수리, R2-ACC-1·2): 서버의 `chart.notes` 는 그 차트를 처음 제출한 본문(또는 일반 사용자 토큰의 `POST /api/charts`)이 정한 값이라 신뢰할 수 없으므로 어떤 상한에도 쓰지 않는다. 한 사용자의 제출이 다른 사용자의 제출을 거절시킬 수 없다. 판정 수는 모든 검사에서 같은 값으로 읽는다: 각 단계(PG·GR·GD·BD·POOR·MISS)의 early/late 합(`epg+lpg` 등)이 0 이 아니면 그 합, 0 이면 합계 필드(`pgreat` 등)다(`effectiveJudgeCounts`, EX·BP 유도와 같은 읽기). 그래서 합계 필드를 0 으로 두고 분리 필드만 키운 본문도 노트 수에 묶인다.
  1. `ex_score` 가 0 이 아니면 판정에서 유도한 값(PGREAT × 2 + GREAT)과 같아야 한다(`field: "ex_score"`). 0 이면 서버가 유도한다.
  2. 노트를 소비한 판정 `pgreat + great + good + bad + poor` 가 `total_notes` 를 넘으면 안 된다(`field: "judge"`, `judgedNoteCount`·`countNotesTaken`. 근거 [../acknowledge/2026-10-14-adr-judge-count-consumed-notes.md](../acknowledge/2026-10-14-adr-judge-count-consumed-notes.md), 사용자 결정 2026-10-14 추천안 A). `total_notes` 가 0 이면 노트를 소비한 판정도 0 이어야 한다(예전에는 0 을 "모름"으로 보고 건너뛰었다. 모든 rbms 클라이언트는 판정과 함께 차트 노트 수를 보낸다). `miss`(rbms 의 空POOR, 노트를 소비하지 않음)와 `empty_poor` 는 세지 않는다. `mode` 가 `POPN_9K` 면 BAD 도 노트를 소비하지 않으므로 뺀다(레퍼런스 `JudgeProperty.PMS` 의 `judgeVanish = {true,true,true,false,true,false}` — BAD 와 空POOR 만 노트를 지우지 않는다. rbms 의 `rbms-judge` `PMS.judge_vanish` 도 같다). `mode` 는 대소문자와 `-`/`_` 차이를 무시한다(`BEAT-7K` = `BEAT_7K`). 옛 식 `pgreat+great+good+bad+max(poor-empty_poor,0)+miss` 는 `miss` 를 노트 소비로 세어, 空POOR 가 見逃し POOR 보다 많은 완주 판을 거절했다(이 수리 전의 서버. 사용자 판 #52: total 1835 = PG~BD 1815 + POOR 20, miss 43 → 옛 식 1858 로 400).
  3. `max_ex_score` 는 0 이거나 `total_notes × 2` 이하다(`field: "max_ex_score"`).
  4. `minbp` 는 0 이거나 BAD + POOR + 空POOR 이상이다(`field: "minbp"`). 게이지가 끝난 판은 도달하지 못한 노트가 더해져 더 크다(`BMSPlayer.java:909`). 0 이면 서버가 유도한다.
  5. 램프가 판정과 맞아야 한다(`field: "clear"`, 원본 `BMSPlayer.java:868` 의 풀콤 조건). `FullCombo`·`Perfect`·`Max` 는 POOR 가 0 이고, BAD 가 노트를 가져가는 모드(`POPN_9K` 밖)에서는 BAD 도 0 이다. `Perfect` 는 GOOD 이 0, `Max` 는 GREAT·GOOD 이 0 이다. 空POOR(`miss`) 는 노트를 가져가지 않고 첫 노트 앞에 올 수 있어(7K 는 콤보도 잇는다) 어느 램프도 막지 않고, pop'n BAD 도 같은 이유로 막지 않는다. 그 아래 램프(게이지 램프, FAILED, 어시스트 램프)는 보지 않는다. 클라이언트의 `rbms_judge::clear_lamp` 와 0.1.0 의 옛 규칙(BAD·POOR 가 있으면 풀콤 아님)이 만드는 램프는 모두 통과한다.
  6. 서버가 저장할 EX·BP(1·4 의 유도값)가 INT 열 안이어야 한다(`field: "judge"`, `reason: "judge counts add up past what a score can hold"`). 코스도 같다.
  이 검사는 정직한 클라이언트가 일관되게 계산한 값을 확인할 뿐이다. `total_notes`·`max_ex_score` 를 함께 부풀리고 판정 수와 맞는 EX·램프를 만든 위조는 막지 못한다(재시뮬 검증 §11.3 몫, 테스트 부채 3.36).
- **best 갱신**: 램프(우선) → EX → BP 순 비교, 더 좋을 때만 best 갱신. 갱신은 조건부 문장 하나(`INSERT IGNORE` 로 행을 보장한 뒤 `UPDATE … WHERE (램프·EX·BP 가 저장된 값보다 좋음)` 의 `affectedRows`)라서 같은 사용자·차트의 두 제출이 동시에 와도 늦게 쓰는 쪽이 낮은 판이면 덮지 못하고, `is_new_best` 는 그 문장이 실제로 바꿨는지로 정한다. 코스 best 도 같다. 같은 갭에 넣는 동시 INSERT 의 교착(1213)과 잠금 대기 초과(1205)는 쓰기 문장 단위로 다시 한다(`withLockRetry`, 연결마다 `innodb_lock_wait_timeout = 5`. 원인과 적용 범위는 [data-model.md](./data-model.md) "잠금 오류 재시도"). 모든 제출은 history로 누적(랭킹은 best 기준). **ranked=false는 best/랭킹 비반영**(기록·리플레이는 보존).
- **SCRATCH AUTO**(웨이브 20, 사용자 결정 2026-10-14): `options.scratch_auto=true` 이거나 `options.assist` 에 `AUTO_SCRATCH` 가 있는 판은 `SCRATCH_AUTO` 표지 flag 를 달고 **ranked** 다. best 는 `(chart, user, scratch_auto)` 로 나눠 두므로 일반 판 최고와 SCRATCH AUTO 판 최고가 따로 남는다. `previous_best`·`is_new_best` 는 그 판의 칸(일반/SCRATCH AUTO) 기준이고, `rank` 는 그 판이 속한 목록(일반 판은 기본 목록, SCRATCH AUTO 판은 `include_scratch_auto=true` 목록)에서 이 판보다 좋은 기록의 수 + 1 이다. 같은 사용자의 더 좋은 다른 칸 최고 기록(예: 일반 판 800 이 있는데 SCRATCH AUTO 700 을 낸 경우)도 센다. `message` 는 `saved (scratch auto)`. `AUTO_SCRATCH` 밖의 어시스트 태그(`CUSTOM_JUDGE` 등)는 여전히 `ASSIST` 로 unranked 이고, 그때 `message` 의 사유에는 `scratch_auto` 를 넣지 않는다(`recorded (unranked: assist,judge_width)`).
- **`ranked=false` 판의 저장**: 클라이언트는 손으로 친 모든 판(SCRATCH AUTO 판 포함, 사용자 결정 2026-10-14)을 보내고, 서버는 `score` 행으로 보존하되 `chart_best`·순위에는 올리지 않는다. 넓힌 판정 판은 클라이언트가 `ranked` 를 막지 않고 보내도 서버가 `ASSIST` 로 unranked 처리한다(최고 기록·순위 제외는 2026-10-14 사용자 확인, S18 결정 6. 근거 [../acknowledge/2026-10-14-adr-save-and-send-every-played-run.md](../acknowledge/2026-10-14-adr-save-and-send-every-played-run.md)).
- EX/BP가 누락되면 `judge`에서 유도: `ex = (epg+lpg)*2 + egr+lgr`, `bp = ebd+lbd+epr+lpr+ems+lms`(레퍼런스 구현 minbp 정의는 BAD+POOR+MISS).
- **`ranked` 적격성**(§11): autoplay·`AUTO_SCRATCH` 밖의 assist·judge_rate>100·total_override≠0·미인증 빌드해시 등 → `ranked=false` + `flags`. SCRATCH AUTO 는 표지만 단다. guest 제출도 unranked(설정) 또는 별도 게스트 보드.

### `GET /api/scores/{score_id}` → `ScoreRecord`(상세, judge·options·replay_id 포함)
### `GET /api/players/{id}/scores?since=&limit=&page=&mode=` → `ScoreRecord[]` (레퍼런스 구현 getPlayData(player, null) — 플레이어 전체. `rank` 는 항상 `null`)

---

## 6. courses — 단위인정 (IRCourseData)

### `GET /api/courses/{course_hash}` → `CourseMeta`
```jsonc
{ "course_hash":"…", "name":"発狂段位 ★01", "lntype":1,
  "charts":[ {"md5":"…","sha256":"…"}, … ],
  "constraint":["GRADE","GAUGE_LR2"],                 // CourseDataConstraint
  "trophy":[ {"name":"bronze","scorerate":0.7,"smissrate":0.05}, … ], // IRTrophyData
  "extra":{} }
```
> `course_hash` = 코스 차트들의 sha256(또는 md5) 결합 해시(레퍼런스 구현/LR2 관행). [compatibility.md] 참조.

### `POST /api/courses` (Bearer) — 코스 결과 제출(네이티브 클라이언트 계약)
### `POST /api/courses/meta` (admin) — 코스 메타 업서트
> **열 범위**(S19 4차 R4-WEB-3): `name` 255자(UTF-16 단위) 이하, `lntype` INT 범위, 넘으면 400(전에는 DB 가 막아 500). 관리자가 쓴 메타라 점수 본문의 참고 글자처럼 자르지 않고 `POST /api/charts` 처럼 거절한다. `POST /api/tables` 의 `courses[]` 도 같은 스키마다.
### `POST /api/courses/{course_hash}/scores` (Bearer) — 코스 결과 제출 별칭
```jsonc
// CourseSubmission (judge는 §5와 동일 구조)
{ "api_version":1, "course_hash":"…", "player":{"id":"alice"}, "lntype":1,
  "clear":"Normal", "ex_score":5000, "max_ex_score":5400, "judge":{…},
  "max_combo":3000, "gauge_value":42.0, "minbp":30,
  "charts":[{"md5":"…","sha256":"…"}, …],
  "trophy":"bronze", "played_at":1700000000000,
  "client_build_sha256":"<64hex>", "client_platform":"macos-universal",   // 선택. 점수 제출과 같은 빌드 신뢰 판정(아래)
  "scratch_auto":false,                     // 선택. 어느 스테이지든 SCRATCH AUTO 로 쳤으면 true (없으면 false)
  "assist":[],                              // 선택. 스테이지들의 어시스트 태그 합(AUTO_SCRATCH·CUSTOM_JUDGE, 각각 한 번, 최대 16개·각 32자). 없으면 []
  "extra":{} }
// 201 SubmitResponse
// SCRATCH AUTO 코스 예: { "accepted":true, "ranked":true, "rank":2, "is_new_best":true,
//                        "flags":["SCRATCH_AUTO"], "message":"saved (scratch auto)" }
// 넓힌 판정이 낀 코스 예: { "accepted":true, "ranked":false, "rank":null, "is_new_best":false,
//                          "flags":["SCRATCH_AUTO","ASSIST"], "message":"recorded (unranked: assist)" }
```
규칙:
- (근거 [../acknowledge/2026-10-14-adr-assisted-course-submission.md](../acknowledge/2026-10-14-adr-assisted-course-submission.md)) `scratch_auto`·`assist` 는 단일 곡 제출의 `options.scratch_auto`·`options.assist` 와 같은 어휘이고, 코스 전체로 접은 값이다(어느 스테이지가 어떤 어시스트였는지는 싣지 않는다: 서버는 코스 전체로 판정한다). 두 필드가 없는 옛 클라이언트 제출은 어시스트 없는 코스로 받는다. 클라이언트는 어시스트가 없으면 두 필드를 싣지 않는다.
- **ranked 판정은 단일 곡과 같다**(`assistFlags`, 웨이브 20): `scratch_auto` 이거나 `assist` 에 `AUTO_SCRATCH` 가 있으면 `SCRATCH_AUTO` 표지(ranked 를 막지 않는다), `AUTO_SCRATCH` 밖의 태그가 있으면 `ASSIST` flag 와 `ranked=false`. `course_score.scratch_auto` 에 SCRATCH AUTO 여부를 남기고, flag 가 하나라도 있으면 `extra` 에 `flags`·`scratch_auto`·`assist` 도 덧붙인다. SCRATCH AUTO 코스는 `course_best` 의 SCRATCH AUTO 칸을 갱신하고 `rank` 는 포함 목록 기준이다. unranked 코스는 `course_best` 를 갱신하지 않고 `rank` 는 `null` 이다.
- 멱등: 동일 `(player, course_hash, played_at)` 재전송은 기존 id 를 돌려주고, `ranked`·`flags` 는 이번 본문으로 다시 판정해 돌려준다.
- **단일 곡과 같은 검사**(S19): 점수 제출과 같은 사용자당 60/min 한도(버킷은 따로, 초과 시 429), 본문 256KB(초과 413), `charts` 최대 64개, `client_build_sha256`·`client_platform`(둘 다 선택)으로 빌드 신뢰를 판정한다. `REQUIRE_BUILD_HASH=true` 인데 해시가 없으면 400(`field: "client_build_sha256"`)이다. 알 수 없는 빌드는 `UNKNOWN_BUILD` flag 를 달고, 명시적 untrusted 빌드이거나 `REQUIRE_BUILD_HASH=true` 인 알 수 없는 빌드는 unranked 로 저장된다(§11.1). 본문 무결성은 단일 곡처럼 본문만 본다(S19 2차 수리): `ex_score` 가 판정에서 유도한 값과 같아야 하고(`field: "ex_score"`), 노트를 소비한 판정(PG·GR·GD·POOR. 코스 본문은 모드가 없어 pop'n BAD 를 위해 BAD 는 세지 않는다)의 2배가 `max_ex_score` 를 넘을 수 없으며(`field: "judge"`. 클라이언트는 스테이지 노트 수 × 2 의 합을 보낸다. 0 이면 판정도 0 이어야 한다), `minbp` 는 0 이거나 BAD+POOR+空POOR 이상이며(`field: "minbp"`), 서버가 저장할 BP 가 INT 열 안이어야 한다(`field: "judge"`. `minbp` 0 이면 BAD+POOR+空POOR 로 유도하므로). 열 범위는 단일 곡과 같다. `charts` 의 차트 행 노트 수는 쓰지 않는다(첫 제출자가 정한 값이라 다른 사람의 코스 제출을 막을 수 있었다). 램프는 보지 않는다: 0.1.0 은 코스 램프로 마지막 스테이지의 램프를 보내 코스 판정 합과 맞지 않을 수 있다(테스트 부채 3.36). 본문 검증 거절과 수락 모두 `submission_audit` 에 남는다.
- **판정 저장**(S19 2차 수리, R2-ACC-3, 마이그레이션 `0004_course_score_judge_totals`): 클라이언트는 코스 판정을 합계(`pgreat`~`miss`)와 `fast`·`slow`·`combobreak`·`empty_poor` 로 보내고 early/late 분리는 0 이다. `course_score` 는 합계·FAST/SLOW·콤보 끊김·空POOR·`max_ex_score` 를 그대로 저장하고, 순위·best 응답의 `judge` 는 분리 합이 0 이 아니면 그것, 아니면 합계를 돌려준다. 0004 이전에 저장된 코스 행은 합계를 잃어 판정이 0 이다(복구 불가).
- **코스 best 갱신과 순위**는 단일 곡과 같다: 조건부 문장 하나로 갱신하고, 목록·제출·`best` 의 `rank` 는 같은 공동 순위 정의다(§4).
### `GET /api/courses/{course_hash}/ranking?limit=&page=&rival_of=&include_scratch_auto=false` → `ScoreRecord[]`
### `GET /api/courses/{course_hash}/best?player={id}&include_scratch_auto=false` → `ScoreRecord | null` (레퍼런스 구현 getCoursePlayData 본인분)
- `include_scratch_auto` 는 차트 순위·best 와 같은 뜻이다(기본은 일반 판만, `true` 면 플레이어마다 더 높은 쪽, SCRATCH AUTO 행은 `extra.scratch_auto: true`).

---

## 7. tables — IR 표 (IRTableData)  레퍼런스 구현 `getTableDatas`

### `GET /api/tables` → `TableData[]`
```jsonc
[ { "name":"発狂BMS難易度表", "url":null,
    "folders":[ { "name":"★1", "charts":[ {"md5":"…","sha256":"…"}, … ] }, … ],
    "courses":[ CourseMeta, … ], "extra":{} } ]
```
### `GET /api/tables/{id}` → `TableData`
### `POST /api/tables` (admin) — 표 등록/갱신
> **열 범위**(S19 4차 검증자 추가): `name` 255자, `url` 512자, `folders[].name` 255자 이하, 넘으면 400(전에는 500). 폴더는 `<표 id>#<위치>` 로 저장되므로(`folderIdFor`, `table_folder.id` varchar(64)), 표 id 와 마지막 폴더 위치로 만든 id 가 64자를 넘으면 `field: "folders"` 400 이다(예: 표 id 64자면 폴더를 둘 수 없고, 62자면 폴더 10개까지).
> 클라이언트(rbms)는 기존 난이도표(`tables.ron`, header.json/data.json)도 직접 fetch하지만, IR 표는 **서버가 큐레이션한 표·코스**를 추가로 제공.

### 7.1 서버가 번들한 표와 표준 BMS 표 형식 (웨이브 17B-1)

DB 의 IR 표(위)와 별개로, 저장소의 스냅샷(`web/data/tables/<id>/`)을 정적으로 싣는 표가 있다. 지금은 `insane1`(発狂BMS難易度表, 1,035곡, 레벨 26개) 하나이고 모든 곡이 `md5` 만 가진다. 응답·화면에는 다른 사이트의 흔적(원본·보관본 주소, 출처 문구, 원본 표의 메모)을 싣지 않는다(사용자 지시 2026-10-14~15, 커밋 4a0872d·ea34002·dd5518e). 스냅샷을 받은 주소와 HTTP 기록은 저장소의 `web/data/tables/insane1/source.json` 에만 있고(재현용, 서빙하지 않음. `ETag` 의 해시만 이 파일에서 읽는다), 스냅샷은 `web/scripts/snapshot-table.ts` 가 정규화한다: header 의 `original_url` 을 버리고, 곡의 `comment` 를 비우고, 내려받기 주소(`url`·`url_diff`·`name_diff`) 밖의 글자 필드(`title`·`artist` 등)에서 웹 주소를 지운다. 곡 내려받기 주소는 남긴다(사용자 정정 "그건 지우면안되는데").

| 경로 | 응답 | 인증 |
|---|---|---|
| `GET /tables/{id}/header.json` | 표준 header. `{ name, symbol, data_url(요청 origin 의 절대 주소), level_order, last_update }`(`toStandardHeader`). 봉투 없음 | 없음 |
| `GET /tables/{id}/data.json` | 표준 body 배열(스냅샷 그대로: `md5 level title artist url url_diff name_diff comment lr2_bmsid`, `comment` 는 모두 빈 문자열, 약 340KB). `ETag`, `If-None-Match` 304 | 없음 |
| `GET /api/fe/tables` | `[{ id, name, symbol, entry_count, level_count, last_update, levels:[{level,count}], standard_paths }]`(봉투, `tableSummaryOutputSchema`) | 없음 |
| `GET /api/fe/tables/{id}` | `{ table, levels:[{ level, entries:[{ md5, level, title, artist, url, url_diff, name_diff }] }] }`(`tableEntryOutputSchema`, `comment` 없음, 약 307KB, 공개 캐시 `s-maxage=3600`) | 없음 |
| `GET /api/fe/tables/{id}/entries?q=&level=&page=&limit=` | 제목·아티스트·md5 앞 6자 이상으로 검색. `limit` 최대 100. 쪽 나누기 포함 | 없음 |
| `GET /api/fe/tables/{id}/records` | `{ table_id, records:{ "<md5>": { clear, assist_grade, ex_score, minbp, max_combo, total_notes, played_at, scratch_auto } } }` — 내 곡별 최고 기록(기록 있는 곡만, `chart.md5` 로 맞춤). 일반 판 최고와 SCRATCH AUTO 판 최고 가운데 더 높은 것이고 `scratch_auto` 가 어느 쪽인지 알린다. `no-store` | 세션 |

`/tables/{id}` 의 사람이 보는 페이지(`<meta name="bmstable" content="/tables/{id}/header.json">`)는 화면 웨이브가 만든다. 클라이언트(`rbms-table`)는 header 의 `data_url` 을 header 의 최종 주소 기준으로 풀고, 이 header 는 절대 주소를 내므로 끝 슬래시(운영은 `/x/` 를 308 로 `/x` 로 뗀다)에 영향받지 않는다. 정본 등록 주소는 `https://bms.hyuns.uk/tables/insane1`(슬래시 없음).

### 7.2 내 폴더 (웹에서 편집, R-BMS 는 읽기만)

결정 근거: [../acknowledge/2026-10-14-adr-user-folders.md](../acknowledge/2026-10-14-adr-user-folders.md). 설계 전문: [../plan/2026-10-14-web-user-folders.md](../plan/2026-10-14-web-user-folders.md).

사용자당 폴더 최대 50개, 폴더당 2,000곡, 사용자 전체 20,000곡, 이름 1~64자(NFC·앞뒤 공백 제거·제어 문자 불가), 설명 255자, 메모 100자. 곡의 식별자는 `md5`(소문자 16진 32자), `sha256` 은 알 때만. 폴더 id 는 `uf_` + 16진 30자.

편집(세션만, 봉투. 경로에 사용자 id 가 없고 남의 폴더 id 는 404 `IR_FOLDER_NOT_FOUND`):

| 메서드 | 경로 | 비고 |
|---|---|---|
| GET / POST | `/api/fe/me/folders` | 목록(`FolderSummary[]`) / 만들기(201) |
| PUT | `/api/fe/me/folders/order` | `{ folder_ids }` — 지금 폴더와 같은 집합이어야 한다(아니면 400) |
| GET / PATCH / DELETE | `/api/fe/me/folders/{folderId}` | 상세(`items` 포함) / `{ name?, description? }` / 204 |
| POST | `/api/fe/me/folders/{folderId}/items` | `{ items:[{ md5, sha256?, title?, artist?, note? }] }` 최대 500. 이미 든 곡은 `skipped` |
| PUT | `/api/fe/me/folders/{folderId}/items` | **통째로 바꾸기**(명시 저장): `{ items:[…순서대로], base_updated_at?, name?, description? }`. `name`·`description` 을 주면 곡과 같은 트랜잭션에서 함께 바뀐다(화면의 저장 버튼 하나 = 요청 하나). `base_updated_at` 이 서버 `updated_at` 과 다르면 409 `IR_FOLDER_CONFLICT`, 이름이 겹치면 409 `IR_FOLDER_NAME_TAKEN` 이고 어느 쪽이든 아무것도 쓰지 않는다. 비교는 폴더 행을 잠근 트랜잭션 안에서 한다 |
| POST | `/api/fe/me/folders/{folderId}/items/remove` | `{ md5s }` |
| PUT | `/api/fe/me/folders/{folderId}/items/order` | `{ md5s }` — 같은 집합이어야 한다 |
| PATCH | `/api/fe/me/folders/{folderId}/items/{md5}` | `{ note }` |
| POST | `/api/fe/me/folders/{folderId}/import` | `{ table_id, levels? }` — 번들 표에서 서버가 곡을 읽어 넣는다 |
| GET | `/api/fe/me/folders/containing/{md5}` | 그 곡이 든 내 폴더 id 배열 |

동시 쓰기: 곡 넣기·빼기·순서·메모·통째로 바꾸기는 모두 폴더 행을 먼저 잠그고(`SELECT … FOR UPDATE`) 항목을 만진다. 같은 곡을 동시에 넣어도 500 이 아니라 한쪽만 `added` 로 세고 나머지는 `skipped` 이며(`INSERT IGNORE`), 폴더당 2,000곡 한도는 잠근 상태에서 다시 확인해 합쳐서 넘으면 뒤에 처리된 요청이 409 `IR_FOLDER_LIMIT_EXCEEDED` 가 된다. 사용자 전체 20,000곡 한도는 요청 앞에서 한 번 확인하는 최선 노력이다(서로 다른 폴더에 동시에 넣으면 한 요청 분량까지 넘을 수 있다).

응답의 `FolderSummary` = `{ id, name, description, is_public, position, item_count, updated_at(unix ms) }`, `ItemChange` = `{ added, removed, skipped, item_count, updated_at }`. 곡의 제목·아티스트 사본은 서버가 정한다(차트 행 → 표 → 요청 순으로 빈 값을 건너뜀, 제어 문자 제거, 200자). 한도를 넘으면 아무것도 넣지 않고 409 `IR_FOLDER_LIMIT_EXCEEDED`(`details` 는 비프로덕션에서만), 같은 이름은 409 `IR_FOLDER_NAME_TAKEN`. `is_public` 은 항상 `false`(공개 동작은 아직 없다).

클라이언트 읽기(Bearer 또는 세션, 본인만. 다른 playerId 는 403. 봉투 없음, `Cache-Control: private, no-store`):

- `GET /api/players/{playerId}/folders` → `{ "folders": [ FolderSummary… ] }`(`position` 순서)
- `GET /api/players/{playerId}/folders/{folderId}` → `FolderSummary` + `"items": [ { md5, sha256, title, artist, note }… ]`(`position` 순서, `sha256` 은 모르면 `""`)

`GET /api/health` 의 `capabilities.folders` 가 `true` 다(없으면 false 로 읽힌다).

---

## 8. replays — 리플레이 서버 저장 (rbms 추가, **µs 정밀**)

> **µs 정밀 필수**: 리플레이의 모든 입력 이벤트는 **마이크로초(µs, i64) raw 타임스탬프**를 보존한다(rbms `ReplayEvent.t`가 이미 µs). 이래야 서버/분석기가 **핵(매크로·오토) 분석**(비인간적 정밀·로봇틱 등간격·동시입력 패턴·시드 대비 불가능한 판정)을 할 수 있다. ms 라운딩 금지.

### `POST /api/charts/{hash}/replays` (Bearer) — 업로드
```jsonc
// ReplayData (rbms-ir 슈퍼셋 — µs 이벤트)
{ "api_version":1, "format":"rbms-replay-ron-v1",   // rbms Replay(RON) 직렬화. (또는 "rbms-keylog-v1" 바이트)
  "chart": {"md5":"…","sha256":"…"},
  "score_id":"sc_…",                                // 연결 스코어(권장 — 검증의 기준)
  "mode":"BEAT_7K",
  // 재현에 필요한 결정적 입력(시드/옵션) — 이 값들로 동일 셔플·판정 재구성:
  "random":"Random", "random_p2":null, "seed":12345, "lntype":1,
  "offset_ms":0, "judge_rate":100, "scratch_auto":false, "constant":false, "gauge":"Hard",
  "client_build_sha256":"<64hex>",                  // 어느 빌드로 기록됐는지(무결성)
  // 입력 스트림: 각 이벤트 = {t_us(µs raw), lane, press(bool)}. RON/JSON 어느 표현이든 µs 보존.
  "events": [ { "t_us": 1234567, "lane": 0, "press": true },
              { "t_us": 1289000, "lane": 0, "press": false }, … ],
  "event_count": 1624, "duration_us": 142000000,
  "size":10240, "extra":{} }
// 201 → { "id":"rp_…", "url":"/api/replays/rp_…", "event_count":1624 }
// 200 → 같은 모양. 본문 score_id 에 이미 리플레이가 연결돼 있으면 새로 저장하지 않고 그 리플레이를 돌려준다(멱등)
// 413 IR_PAYLOAD_TOO_LARGE (상한: env REPLAY_MAX_BYTES, 예 4MB)
// 413 IR_PAYLOAD_TOO_LARGE (비프로덕션에서만 details.scope = replay_count / replay_bytes) — 사용자별 저장 한도(아래)
```
> **열 범위**(S19 리뷰): `lntype`·`offset_ms`·`judge_rate` 는 INT 범위, `event_count`·`size` 는 0~INT 최대, `duration_us`·`events[].t_us` 는 JavaScript 와 BIGINT 가 정확히 담는 범위(절댓값 2^53-1, `duration_us` 는 0 이상), `score_id` 는 36자 이하다. 넘으면 400 이다(전에는 DB 범위 오류로 500). `format` 32자, `mode` 16자는 거절하지 않고 열 길이로 자른다. 계약 본문(`replay-long.json`·`replay-random.json`)은 그대로 받는다.

> **사용자별 저장 한도**(S19, 메인이 정한 값 — 바꾸려면 `REPLAY_USER_QUOTA`): 사용자당 리플레이 2,000개, 저장 크기 합계 512MiB. 새 리플레이를 만들 때만 센다(이미 연결된 리플레이를 다시 올리는 멱등 200 은 세지 않는다). 확인과 저장 사이는 원자적이지 않아 동시에 올리면 한 건 분량까지 넘을 수 있다(업로드는 사용자당 10/min 이다). `replay.size` 는 클라이언트가 보낸 `size` 와 실제 저장 길이 가운데 큰 값이다(작게 속여 한도를 피할 수 없다). `replay.user_id` 색인(`replay_user_idx`, 마이그레이션 0003)으로 합계를 읽는다.
> **멱등**: `score_id` 가 있고 그 점수(같은 사용자)에 이미 `replay_id` 로 연결된 리플레이가 있으면 `replay` 행을 만들지 않고 200 으로 기존 `{ id, url, event_count }` 를 돌려준다(캐시 무효화도 하지 않는다). 기록 동기화가 같은 판의 리플레이를 다시 올려도 행이 쌓이지 않게 하기 위함이다. 먼저 `score_id` 의 점수가 있어야 연결이 생기므로 점수 제출(§5)이 앞선다. `score_id` 가 없는 업로드는 지금처럼 올릴 때마다 새 행이다. rate-limit(사용자당 10/min)은 200 응답에도 한 번 센다.
### `GET /api/replays/{id}` → `ReplayData`(events µs 포함) — 다운로드(고스트·관전·**핵분석**·검증)
### `GET /api/charts/{hash}/replays?player={id}&limit=` → `ReplayMeta[]`(events 제외 목록)
> 현재 서버는 events JSON을 MySQL `replay.data`에 저장하고 메타를 같은 행에 인덱싱합니다. `REPLAY_STORAGE=db`만 지원하며, `blob`/Vercel Blob은 미구현입니다. 재시뮬 검증(FR-12)은 후속입니다.

---

## 9. settings — 설정 동기화 (rbms 추가)

계정에 named 설정 blob 저장. rbms 클라이언트 파일(`settings.ron`·`keyconfig.ron`·`tables.ron`)을 키별로 보관.

### `GET /api/players/{id}/settings/{name}` (Bearer) → 단일 blob
```jsonc
{ "name":"settings", "format":"ron", "content":"<문자열>", "updated_at":1700000000000 }
// 404 IR_SETTING_NOT_FOUND
```
### `PUT /api/players/{id}/settings/{name}` (Bearer) — 업서트(낙관적 동기화)
```jsonc
// req
{ "api_version":1, "format":"ron", "content":"<문자열>", "base_updated_at":1700000000000, "extra":{} }
// 200 → { "updated_at":1700000050000 }
// 409 conflict: base_updated_at 가 서버 최신과 불일치 → { "conflict":true, "server": {GET 결과} }
// 400 VALIDATION_ERROR (details.field "name"): 이름이 [A-Za-z0-9_.-]{1,64} 가 아님
// 413 IR_PAYLOAD_TOO_LARGE (비프로덕션에서만 details.scope = "setting_names"): 사용자당 이름 16개를 넘는 새 이름(있던 이름은 고칠 수 있다)
// 429 RATE_LIMITED: 사용자당 30/min
```
> **낙관적 잠금은 DB 문장 하나가 정한다**(S19): `base_updated_at` 이 있으면 `UPDATE … WHERE updated_at = base`(base 0 이면 `INSERT IGNORE`)의 `affectedRows` 로 저장 여부를 정한다. 같은 base 로 동시에 온 두 PUT 은 정확히 하나만 200 이고 나머지는 409 와 서버 최신본을 받는다. 새 `updated_at` 은 `max(now, 저장된 값 + 1)` 이라 서버 시계가 뒤처져도 base 보다 크다. `base_updated_at` 이 없으면 마지막 쓰기가 이긴다(예전과 같다).
> 현재 구현은 목록/삭제 `/api/settings/*`를 제공하지 않습니다. 이름별 get/put 경로가 네이티브 `rbms-ir` 계약이며, PUT은 저장한 `updated_at`을 200 응답으로 반환합니다. content는 클라 포맷(RON 포함)을 불투명 문자열로 보관하고, 충돌은 `base_updated_at` 기반입니다.

---

## 10. 인증·레이트리밋·CORS·페이지네이션 규약
- **인증(이중)**: better-auth(컨벤션 §11) 기반 — (a) **네이티브 클라(rbms)** = `Authorization: Bearer <token>`(`/api/auth/token`), (b) **웹 FE** = better-auth **세션 쿠키**(`HttpOnly`·`SameSite`·CSRF). 보호 엔드포인트는 `withAuth`/`withAdmin` HOF. 제출은 토큰/세션의 player == 본문 `player.id` 강제(guest 제외).
- **CORS**: Next 단일 오리진 배포이며 외부 웹 오리진을 지원하지 않으므로 CORS 미들웨어와 `ALLOWED_ORIGINS`는 없습니다. 네이티브 클라이언트에는 브라우저 CORS가 적용되지 않습니다.
- **rate-limit**(정본 `web/src/server/lib/with-rate-limit.ts` 의 `RATE_LIMIT`, 초과 시 429 `RATE_LIMITED`): 로그인 5/min/IP(`/api/auth/login` 과 `/api/fe/auth/login` 이 한 버킷), 가입 5/min/IP(`/api/auth/register` 와 `/api/fe/auth/register` 가 한 버킷), 점수 제출 60/min/사용자(guest 는 IP), 코스 제출 60/min/사용자(점수와 다른 버킷), 리플레이 업로드 10/min/사용자, 설정 PUT 30/min/사용자, 토큰 폐기(`DELETE /api/auth/token/current`) 5/min/IP(로그인·가입과 다른 버킷), `/api/fe/*` 공개 읽기와 `get-session`·`sign-out` 120/min/IP, 폴더 편집 60/min/사용자, 폴더에 표 가져오기 10/min/사용자, 폴더 읽기 120/min/사용자, 표의 내 기록 30/min/사용자. `/api/auth/me` 와 `POST /api/auth/token`(세션 토큰 회전)에는 한도가 없다. 한도는 인스턴스 로컬 메모리(`Map`, 버킷 5,000개를 넘으면 만료된 것부터 정리)라 Vercel 다중 인스턴스에서는 best-effort 다(KNOWN ISSUE: 지속형 한도는 미착수, 웹 2차 후보).
- **본문 크기**: `POST /api/scores`·코스 제출 256KB, `POST /api/charts` 64KB, 설정 PUT 256KB, 리플레이 `REPLAY_MAX_BYTES`(기본 4MB). 초과는 413 `IR_PAYLOAD_TOO_LARGE`.
- **페이지네이션**: `?page`(1-base)·`limit`(기본/상한은 DTO별). 네이티브는 raw 배열, 웹 FE는 `/api/fe/*` envelope를 사용합니다.

---

## 11. 무결성 & anti-cheat (rbms 추가 — 공평 평가의 핵심)
"플레이에 사용된 모든 정보 + 클라 빌드 해시 + µs 리플레이"를 보존해 **공정 평가·변조 탐지**를 가능케 한다.

### 11.1 클라이언트 빌드 해시 (변조 방지)
- 모든 `POST /api/scores`·리플레이 업로드는 `client_build_sha256`(실행 중 클라 바이너리 SHA-256) + `client_platform` 동봉. 서버는 **모든 제출 로그에 빌드 해시 기록**(`submission_audit`).
- **빌드 allowlist**: 공식 릴리스 빌드 해시를 `client_build` 테이블에 등록(릴리스 CI가 산출물 SHA-256을 자동 등록 — [data-model.md]). 정책:
  - 등록된 신뢰 빌드 → 정상 `ranked`.
  - 미등록/미상 해시 → `flags:["UNKNOWN_BUILD"]`; `REQUIRE_BUILD_HASH=false`이면 그 flag만으로는 unranked가 되지 않는다.
  - 명시적 untrusted 빌드, 또는 `REQUIRE_BUILD_HASH=true`인 미상/누락 해시 → unranked. 입력 자체를 거부하지는 않는다.
- 관리: `GET/POST /api/admin/builds`(admin) — 빌드 해시 등록/조회/신뢰표시.

### 11.2 ranked 적격성 (자동 판정)
서버가 제출 옵션으로 `ranked` 산출. **unranked 트리거(flags)**: `AUTOPLAY`(autoplay) · `ASSIST`(`AUTO_SCRATCH` 밖의 assist 태그) · `JUDGE_WIDTH`(judge_rate>100) · `TOTAL_OVERRIDE`(total_override≠0) · `UNKNOWN_BUILD` · `GUEST` · `REPLAY_MISMATCH`(검증 불일치, 후속: 지금 코드에 없다. flag 목록의 정본은 `ranked-policy.ts` 의 `SCORE_FLAG`). unranked도 **기록·리플레이는 보존**(연습/공유), best·랭킹만 제외. **표지 flag**(ranked 를 막지 않음): `UNKNOWN_BUILD`(설정에 따라), `SCRATCH_AUTO`(웨이브 20부터. SCRATCH AUTO 판은 따로 센 best 로 `include_scratch_auto=true` 목록에 오른다).

### 11.3 µs 리플레이 검증 (FR-12, 후속 워커)
- 제출 score를 `seed`+옵션으로 **재시뮬**해 EX/clear/판정분포 일치 확인 → `verified` 플래그(capability `verified_scores`).
- µs 입력 통계(등간격·반응속도·동시성)로 매크로/오토 의심 점수화 → `flags:["SUSPECT_TIMING"]`. (휴리스틱, v1은 데이터 적재만.)

### 11.4 제출 감사 로그
- `submission_audit`(score_id·user·build_sha256·platform·ip·ua·accepted·ranked·flags·ts) — 분석·어뷰즈 추적.

---

## 12. FE(web) 전용 엔드포인트
> 네이티브 IR 위에 구현된 FE 조회 경로입니다. 모두 `/api/fe/*`이고 envelope를 고정으로 사용합니다.

- `GET /api/fe/charts/search`, `/api/fe/charts/[hash]/leaderboard`, `/api/fe/activity/recent`, `/api/fe/leaderboards/players`, `/api/fe/players/[playerId]/{recent,stats}`, `/api/fe/stats/summary`.
- 어시스트 등급(2026-10-22): `ScoreRecord`, 활동 항목(`/api/fe/activity/recent`, `/api/fe/players/[playerId]/recent`)에 `assist_grade` 가 있고, `/api/fe/players/[playerId]/stats` 의 `lamps[]` 는 `{ clear, assist_grade, count }`, `/api/fe/tables/{id}/records` 의 곡마다도 `assist_grade` 가 있다(`fe.service.ts`, `table-snapshot.service.ts`). 코스 순위는 항상 0.
- SCRATCH AUTO 다루기(웨이브 20): `/api/fe/charts/[hash]/leaderboard?include_scratch_auto=true` 는 §4 차트 순위와 같은 포함 목록을 쓴다(기본 제외). `/api/fe/leaderboards/players?include_scratch_auto=true` 는 `cleared_charts`·`played_charts` 를 곡마다 두 칸 가운데 하나라도 있으면 세고 `total_plays` 에 SCRATCH AUTO 판을 더한다(기본은 일반 판만). `/api/fe/activity/recent` 는 SCRATCH AUTO 판을 늘 뺀다. `/api/fe/players/[playerId]/recent` 는 모든 판을 `scratch_auto` 와 함께 돌려준다. `/api/fe/players/[playerId]/stats` 의 `lamps`·`cleared` 는 곡마다 더 높은 쪽으로 세고 그중 SCRATCH AUTO 판인 곡 수를 `scratch_auto_charts` 로 준다.
- `GET /api/fe/me/settings`, `GET|POST /api/fe/tokens`, `DELETE /api/fe/tokens/[tokenId]`은 세션 사용자 전용입니다.
- `POST /api/fe/auth/login` `{ id, password }` 는 웹 로그인입니다. `/api/auth/login` 과 같은 자격 확인을 하고 세션 쿠키만 세팅합니다(API 토큰을 만들지 않음). 응답 `{ success:true, data:{ user:{ id, login_id, name, role } } }`, 실패 401 `UNAUTHORIZED`, 5/min/IP(`/api/auth/login` 과 같은 버킷).
- `POST /api/fe/auth/register` `{ id, email, password, name? }` 는 웹 가입입니다. `/api/auth/register` 와 같은 검사(아이디 형식·예약어, 중복은 409 `IR_ACCOUNT_EXISTS`)를 하고 세션 쿠키만 세팅합니다. 응답은 로그인과 같은 꼴이고 201 입니다.
- 웹 인증은 better-auth catch-all 및 IR 호환 `/api/auth/{register,login,me,token}` 경로를 사용합니다. OAuth 제공자는 이 저장소에서 구성하지 않았습니다.

---

## 13. (선택) LR2IR 레거시 호환 어댑터 — 미구현
원조 LR2 클라이언트/스크레이퍼 호환을 위한 **조회 어댑터**(capability `lr2ir_compat`).
- `GET /lr2ir/2/getrankingxml.cgi?id={playerid}&songmd5={md5}` → LR2IR XML
  ```xml
  #
  <ranking>
    <score><name>nakt</name><id>1</id><clear>4</clear><notes>1797</notes>
      <combo>602</combo><pg>1003</pg><gr>680</gr><minbp>39</minbp></score>
    …
  </ranking>
  <lastupdate>…</lastupdate>
  ```
  - `clear` 1-5(FAILED/EASY/CLEAR/HARD/FULLCOMBO)로 다운매핑(슈퍼셋 램프 → LR2 5단계). `pg=epg+lpg`, `gr=egr+lgr`, `minbp`=BP, `notes`/`combo` 그대로.
- `GET /lr2ir/search.cgi?mode=ranking&bmsmd5={md5}` → 최소 HTML(title) — 스크레이퍼 호환(선택).
> 제출(`gateway.cgi`)은 난독 바이너리라 **재현 비범위**(PRD §3)이며, 위 조회 어댑터도 현재 Route Handler로 구현되지 않았습니다.

---

## 14. DTO 인덱스 (Zod 스키마로 구현 — `dto/`)
- 입력: `accountSchema`·`loginSchema`·`sessionLoginSchema`·`sessionRegisterSchema`·`scoreSubmissionSchema`(`submissionChartSchema`)·`courseSubmissionSchema`·`replayUploadSchema`·`settingPutSchema`·`rankingQuerySchema`·`bestQuerySchema`·`chartUpsertSchema`·`chartSearchQuerySchema`·`clientBuildSchema`·`folder*Schema`(`dto/folder.ts`)·`tableEntriesQuerySchema`.
- 응답: `serverInfoSchema`·`authResponseSchema`·`playerProfileSchema`·`chartMetaSchema`·`scoreRecordSchema`·`submitResponseSchema`·`courseMetaSchema`·`tableDataSchema`·`settingBlobSchema`·`settingBlobResponseSchema`·`folderSummarySchema`·`folderDetailSchema`·`table*OutputSchema`(`dto/table-snapshot.ts`). 리플레이 목록 항목에는 별도 zod 스키마가 없다(서비스가 만든 객체를 그대로 돌려준다).
- 핵심 공유 스키마:
  - `judgeBreakdownSchema`(early/late epg…lms + 합계 미러 + empty_poor + avgjudge). 판정 수는 `countColumnSchema`(0~INT 열 상한).
  - 열 범위 스키마(S19 3차): `intColumnSchema`·`countColumnSchema`·`floatColumnSchema`·`columnTextSchema(길이)`(자름)·`buildHashSchema`, 상수 `INT_COLUMN_MAX`·`INT_COLUMN_MIN`·`FLOAT_COLUMN_MAX`·`JSON_COLUMN_DEPTH_MAX`·`JSON_KEY_MAX_BYTES`(S19 4차).
  - `playOptionsSchema`(**플레이 전체 옵션** — gauge·random·random_p2·option·seed·hispeed·constant·lift·lane_cover·judge_rate·offset_ms·auto_offset·total_override·autoplay·scratch_left·scratch_auto·assist·input_device·judge_algorithm·rule·skin·lntype).
  - 무결성: 점수·코스 제출의 `client_build_sha256`·`client_platform` 은 선택(`nullish`)이고 없으면 `UNKNOWN_BUILD` 표지가 붙는다. `client_build_sha256` 은 64자 이하 소문자 16진수(`buildHashSchema`), `client_platform` 은 32자로 자른다. 리플레이 업로드의 `client_build_sha256` 은 있으면 64자다. 대부분의 스키마에 `extra: z.record(z.string(), z.unknown()).default({})`.
  - 공용 시드: `seedSchema`(안전 정수가 아니면 0).
- 타입은 `z.infer<typeof …>`로 유도.

> `rbms-ir`은 이 규격의 DTO·HTTP 확장을 구현했으며, GUI 로그인·랭킹·리플레이·settings 동기화·기록 동기화(`sync.rs`)·로그아웃 토큰 폐기도 완료됐습니다. 계약 교차 테스트(플레이어 테스트 `apps/rbms-player/src/app_result/record_tests/contract_requests.rs` 가 실제로 보낸 본문을 `contract/client-requests/*.json` 으로 남기고 `web/tests/dto/client-requests.test.ts` 가 서버 스키마로 통과시킴, e9aebae. `crates/rbms-ir/src/contract_tests/fixtures.rs`·`web/tests/dto/ir-contract.test.ts` 는 각 쪽의 손으로 만든 고정 본문 테스트)가 와이어 형식의 회귀를 막는다. 운영에는 2026-10-15 기준 제출 119건이 들어와 있다(`/api/fe/stats/summary`). v0.1.4 로 친 판의 결과 화면 `IR: RANK` 확인은 사용자 대기다.
