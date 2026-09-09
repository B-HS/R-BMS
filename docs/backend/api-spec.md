# rbms 백엔드 — HTTP API 규격 (IR 슈퍼셋)

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
> 단, **IR 호환 GET(랭킹·베스트 등)** 은 클라이언트(`rbms-ir`)가 봉투 없는 raw 배열/객체를 기대하므로(현행 `HttpScoreServer`), 두 가지 모드를 둔다:
> - **rbms 모드(기본)**: `rbms-ir`가 파싱하는 raw JSON(아래 각 DTO 그대로).
> - **envelope 모드**: `?envelope=1` 또는 `Accept: application/vnd.rbms.v1+json` 시 위 봉투로 감쌈.
> (구현 단순화를 위해 v1은 **raw 모드 고정**, envelope는 capability로 후속. `rbms-ir`/`docs/reference/ir-api.md`와 일치.)

### 0.2 상태코드 / 에러코드
| HTTP | 상황 | code(예) |
|---|---|---|
| 200/201 | 성공 | — |
| 400 | DTO 검증 실패 | `VALIDATION_ERROR` |
| 401 | 토큰 없음/무효 | `UNAUTHORIZED` |
| 403 | 권한 없음 | `FORBIDDEN` |
| 404 | 리소스 없음 | `IR_CHART_NOT_FOUND`·`IR_PLAYER_NOT_FOUND`·`IR_SCORE_NOT_FOUND`·`IR_REPLAY_NOT_FOUND` |
| 409 | 충돌(중복 계정) | `IR_ACCOUNT_EXISTS` |
| 413 | 본문 초과(리플레이) | `IR_PAYLOAD_TOO_LARGE` |
| 429 | rate-limit | `RATE_LIMITED` |
| 503 | 미구성 서비스 | `SERVICE_NOT_CONFIGURED` |
> 에러는 `lib/error-code.ts`·`error-message.ts`·`error.ts` 3파일 중앙화(컨벤션 §6). 도메인 접두사 `IR_*`.

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
{ "name": "my-ir", "version": "0.1.0", "ir_compat": "1",
  "capabilities": { "ranking": true, "player_best": true, "rivals": true,
                    "courses": true, "replays": true, "tables": true,
                    "settings_sync": true, "accounts": true, "lr2ir_compat": false } }
```
> `capabilities`는 `rbms-ir`의 `ServerCapabilities` 슈퍼셋(여기 `settings_sync`·`accounts`·`lr2ir_compat` 추가). 클라가 모르는 키는 무시.

### `GET /api/version` → `{ "api_version": 1, "server": "0.1.0", "commit": "abc1234" }`

---

## 2. auth — 계정 (레퍼런스 구현 IRAccount 슈퍼셋)

### `POST /api/auth/register`
```jsonc
// req  (IRAccount = {id, password, name} 슈퍼셋)
{ "api_version":1, "id":"alice", "password":"…", "name":"Alice", "email": null, "extra":{} }
// 201 → AuthResult
{ "token":"<bearer>", "player": { "id":"alice", "name":"Alice", "rank":"", "total_plays":0, "rank_points":0.0, "extra":{} } }
// 409 IR_ACCOUNT_EXISTS
```
### `POST /api/auth/login`
```jsonc
{ "api_version":1, "id":"alice", "password":"…" }   // → 200 AuthResult (위와 동일)
// 401 UNAUTHORIZED
```
### `GET /api/auth/me`  (Bearer)
```jsonc
// 200 PlayerProfile (자기 자신, 비공개 필드 포함 가능)
{ "id":"alice", "name":"Alice", "rank":"", "total_plays":1234, "rank_points":56.7, "email":null, "extra":{} }
```
### `POST /api/auth/token`  (Bearer) — API 토큰 발급/회전 → `{ "token":"…", "created_at":1700000000000 }`
> better-auth 기반(컨벤션 §11). 레퍼런스 구현 `register`/`login`(IRConnection)에 1:1 대응. `guest` 제출은 토큰 없이 `player.id="guest"`(서버 설정 `allow_guest`).

---

## 3. players — 프로필 / 라이벌 (IRPlayerData)

### `GET /api/players/{id}` → `PlayerProfile`
```jsonc
{ "id":"alice", "name":"Alice", "rank":"", "total_plays":1234, "rank_points":56.7, "extra":{} }
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
### `GET /api/charts/{hash}/ranking?limit=50&page=1&rival_of={id}&lnmode=1`
```jsonc
// 200 ScoreRecord[]  (rbms 모드 raw 배열)
[ { "player":{"id":"alice"}, "player_name":"Alice", "clear":"Hard",
    "ex_score":1600, "max_ex_score":1624, "max_combo":812, "minbp":1, "rank":1,
    "judge":{ "epg":700,"lpg":12,"egr":50,"lgr":14,"egd":5,"lgd":16,"ebd":1,"lbd":7,"epr":2,"lpr":3,"ems":0,"lms":2,
              "pgreat":712,"great":64,"good":21,"bad":8,"poor":5,"miss":2,"fast":30,"slow":40,"combobreak":15,"empty_poor":9 },
    "gauge_value":86.0, "options": PlayOptions, "seed":12345, "replay_id":"rp_…",
    "played_at":1700000000000, "extra":{} } ]
```
- `rival_of={id}`: 그 플레이어의 라이벌만(+본인). `lnmode`: 0=LN,1=CN,2=HCN(레퍼런스 구현 `lntype`).
- LR2IR 호환 필드(`pg=epg+lpg`, `gr=egr+lgr`, `notes`, `combo`, `minbp`)는 `judge`+상위 필드로 유도 가능.

### `GET /api/charts/{hash}/best?player={id}&lnmode=1` → `ScoreRecord | null`  (레퍼런스 구현 getPlayData 본인분)

---

## 5. scores — 제출 / 조회 (레퍼런스 구현 IRScoreData 슈퍼셋)

### `POST /api/scores` (Bearer 또는 guest)  — 레퍼런스 구현 `sendPlayData`
> **원칙**: 제출은 *플레이에 사용된 모든 정보*를 담는다(공평 평가). 채점·난이도에 영향을 주는 **모든 플레이 옵션 + 클라이언트 빌드 해시 + (선택)리플레이**를 함께 보내고 서버가 보존한다. 서버가 `ranked` 적격성과 무결성(빌드해시)을 직접 판정한다(§11).
```jsonc
// req ScoreSubmission (rbms-ir 슈퍼셋 — early/late + full play context + integrity)
{ "api_version":1,
  "chart": { "md5":"…", "sha256":"…" },
  "player": { "id":"alice" },
  "mode":"BEAT_7K", "lntype":1,
  "clear":"Hard",
  "ex_score":1488, "max_ex_score":1624,
  "judge": {
     "epg":700,"lpg":12, "egr":50,"lgr":14, "egd":5,"lgd":16,
     "ebd":1,"lbd":7,   "epr":2,"lpr":3,   "ems":0,"lms":2,
     "avgjudge":-1200,                       // µs, 평균 판정오차(레퍼런스 구현 avgjudge)
     "empty_poor":9,                         // rbms 空POOR(노트 미소실 빈 POOR)
     // 합계 미러(호환·검증용, 서버가 재계산 가능):
     "pgreat":712,"great":64,"good":21,"bad":8,"poor":5,"miss":2,
     "fast":30,"slow":40,"combobreak":15 },
  "max_combo":540, "total_notes":812, "passnotes":812, "minbp":15, "gauge_value":86.0,

  // ── 플레이에 사용된 전체 옵션(공평 평가의 단일 출처). rbms PlaySettings/PlayerConfig 전수. ──
  "options": {
     "gauge":"Hard", "random":"Random", "random_p2":null, "option":1,   // option=레퍼런스 구현 노트옵션 비트필드
     "lntype":1, "seed":12345, "scratch_left":false, "scratch_auto":false,
     "hispeed":2.0, "constant":false, "green_number":300,               // 스크롤
     "lift":0.0, "lane_cover":0.0,                                      // 시야
     "judge_rate":100, "offset_ms":0, "auto_offset":false,             // 판정폭/오프셋 (난이도 영향!)
     "total_override":0.0,                                             // 0=차트값, >0=오버라이드 (게이지 영향!)
     "autoplay":false,                                                  // 오토플레이 여부 (랭킹 무효 트리거)
     "assist":[], "input_device":"keyboard",
     "judge_algorithm":"Combo", "rule":"BEAT", "skin":"NORMAL" },

  // ── 무결성(클라이언트 변조 방지) ──
  "client":"rbms/0.1.0",                    // 클라 표시 버전
  "client_build_sha256":"<64hex>",          // 실행 중인 클라 빌드 바이너리 SHA-256 (필수, 서버 로깅·검증 §11)
  "client_platform":"macos-universal",      // 빌드 타깃
  "replay_id":null,                         // 이미 업로드한 리플레이 연결(또는 제출 후 §8로 업로드)
  "played_at":1700000000000,                // unix ms
  "extra":{} }

// 201 SubmitResponse
{ "accepted":true, "ranked":true, "rank":3, "previous_best":1400, "is_new_best":true,
  "score_id":"sc_…", "flags":[], "message":"saved" }
// ranked=false 예: { "accepted":true, "ranked":false, "flags":["AUTOPLAY"], "message":"recorded (unranked: autoplay)" }
```
규칙:
- **멱등**: 동일 `(player, sha256||md5, played_at)` 재전송은 기존 score_id 반환(중복 생성 X).
- **best 갱신**: 램프(우선) → EX → BP 순 비교, 더 좋을 때만 best 갱신. 모든 제출은 history로 누적(랭킹은 best 기준). **ranked=false는 best/랭킹 비반영**(기록·리플레이는 보존).
- EX/BP가 누락되면 `judge`에서 유도: `ex = (epg+lpg)*2 + egr+lgr`, `bp = ebd+lbd+epr+lpr+ems+lms`(레퍼런스 구현 minbp 정의는 BAD+POOR+MISS).
- **`ranked` 적격성**(§11): autoplay·scratch_auto·assist≠∅·judge_rate≠100·total_override≠0·미인증 빌드해시 등 → `ranked=false` + `flags`. guest 제출도 unranked(설정) 또는 별도 게스트 보드.

### `GET /api/scores/{score_id}` → `ScoreRecord`(상세, judge·options·replay_id 포함)
### `GET /api/players/{id}/scores?since=&limit=&mode=` → `ScoreRecord[]` (레퍼런스 구현 getPlayData(player, null) — 플레이어 전체)

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

### `POST /api/courses` (Bearer) — 코스 메타 업서트
### `POST /api/courses/{course_hash}/scores` (Bearer) — the reference implementation `sendCoursePlayData`
```jsonc
// CourseSubmission (judge는 §5와 동일 구조)
{ "api_version":1, "course_hash":"…", "player":{"id":"alice"}, "lntype":1,
  "clear":"Normal", "ex_score":5000, "max_ex_score":5400, "judge":{…},
  "max_combo":3000, "gauge_value":42.0, "minbp":30,
  "charts":[{"md5":"…","sha256":"…"}, …],
  "trophy":"bronze", "played_at":1700000000000, "extra":{} }
// 201 SubmitResponse
```
### `GET /api/courses/{course_hash}/ranking?limit=&page=&rival_of=` → `ScoreRecord[]`
### `GET /api/courses/{course_hash}/best?player={id}` → `ScoreRecord | null` (레퍼런스 구현 getCoursePlayData 본인분)

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
> 클라이언트(rbms)는 기존 난이도표(`tables.ron`, header.json/data.json)도 직접 fetch하지만, IR 표는 **서버가 큐레이션한 표·코스**를 추가로 제공.

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
// 413 IR_PAYLOAD_TOO_LARGE (상한: env REPLAY_MAX_BYTES, 예 4MB)
```
### `GET /api/replays/{id}` → `ReplayData`(events µs 포함) — 다운로드(고스트·관전·**핵분석**·검증)
### `GET /api/charts/{hash}/replays?player={id}&limit=` → `ReplayMeta[]`(events 제외 목록)
> 서버는 events를 **불투명 blob(µs 무손실)** 로 스토리지에 저장 + 메타(event_count·duration_us·build·score 연결) 인덱싱. 재시뮬 검증(FR-12)·핵분석은 µs 스트림 + seed/options로 오프라인 재구성. rbms `Replay`(RON) 1:1 대응.

---

## 9. settings — 설정 동기화 (rbms 추가)

계정에 named 설정 blob 저장. rbms 클라이언트 파일(`settings.ron`·`keyconfig.ron`·`tables.ron`)을 키별로 보관.

### `GET /api/settings` (Bearer) → 보유 키 목록
```jsonc
{ "keys":[ {"key":"settings","format":"ron","updated_at":1700000000000,"size":512},
           {"key":"keyconfig","format":"ron","updated_at":…,"size":…},
           {"key":"tables","format":"ron","updated_at":…,"size":…} ] }
```
### `GET /api/settings/{key}` (Bearer) → 단일 blob
```jsonc
{ "key":"settings", "format":"ron", "content":"<문자열>", "updated_at":1700000000000, "extra":{} }
// 404 IR_SETTING_NOT_FOUND
```
### `PUT /api/settings/{key}` (Bearer) — 업서트(낙관적 동기화)
```jsonc
// req
{ "api_version":1, "format":"ron", "content":"<문자열>", "base_updated_at":1700000000000, "extra":{} }
// 200 → { "key":"settings", "updated_at":1700000050000, "conflict":false }
// 409 conflict: base_updated_at 가 서버 최신과 불일치 → { "conflict":true, "server": {GET 결과} }
```
### `DELETE /api/settings/{key}` (Bearer) → 204
> 표준 키: `settings`·`keyconfig`·`tables`. 임의 named blob(예: `skin:custom`)도 허용. content는 클라 포맷(RON) 그대로 — 서버는 불투명. 충돌은 `base_updated_at` 기반 낙관적 잠금.

---

## 10. 인증·레이트리밋·CORS·페이지네이션 규약
- **인증(이중)**: better-auth(컨벤션 §11) 기반 — (a) **네이티브 클라(rbms)** = `Authorization: Bearer <token>`(`/api/auth/token`), (b) **웹 FE** = better-auth **세션 쿠키**(`HttpOnly`·`SameSite`·CSRF). 보호 엔드포인트는 `withAuth`/`withAdmin` HOF. 제출은 토큰/세션의 player == 본문 `player.id` 강제(guest 제외).
- **CORS**: FE 오리진 화이트리스트(env `ALLOWED_ORIGINS`), 쿠키 인증 시 `credentials: true`. 미들웨어(`middleware/cors`).
- **rate-limit**: 로그인 5/min/IP, 제출 60/min/token, 리플레이 업로드 10/min/token → 429 `RATE_LIMITED`.
- **페이지네이션**: `?page`(1-base)·`limit`(기본 50, 최대 100). 네이티브(rbms-ir)는 raw 배열, **웹 FE는 envelope 모드 권장**(`?envelope=1` → `{success,data,pagination}`).

---

## 11. 무결성 & anti-cheat (rbms 추가 — 공평 평가의 핵심)
"플레이에 사용된 모든 정보 + 클라 빌드 해시 + µs 리플레이"를 보존해 **공정 평가·변조 탐지**를 가능케 한다.

### 11.1 클라이언트 빌드 해시 (변조 방지)
- 모든 `POST /api/scores`·리플레이 업로드는 `client_build_sha256`(실행 중 클라 바이너리 SHA-256) + `client_platform` 동봉. 서버는 **모든 제출 로그에 빌드 해시 기록**(`submission_audit`).
- **빌드 allowlist**: 공식 릴리스 빌드 해시를 `client_build` 테이블에 등록(릴리스 CI가 산출물 SHA-256을 자동 등록 — [data-model.md]). 정책:
  - 등록된 신뢰 빌드 → 정상 `ranked`.
  - 미등록/미상 해시 → `ranked=false` + `flags:["UNKNOWN_BUILD"]`(거부 대신 기록, 정책에 따라 강화 가능).
  - 빌드 해시 누락(구버전 클라) → unranked 또는 거부(env `REQUIRE_BUILD_HASH`).
- 관리: `GET/POST /api/admin/builds`(admin) — 빌드 해시 등록/조회/신뢰표시.

### 11.2 ranked 적격성 (자동 판정)
서버가 제출 옵션으로 `ranked` 산출. **unranked 트리거(flags)**: `AUTOPLAY`(autoplay) · `SCRATCH_AUTO` · `ASSIST`(assist≠∅) · `JUDGE_WIDTH`(judge_rate≠100) · `TOTAL_OVERRIDE`(total_override≠0) · `UNKNOWN_BUILD` · `GUEST` · `REPLAY_MISMATCH`(검증 불일치). unranked도 **기록·리플레이는 보존**(연습/공유), best·랭킹만 제외.

### 11.3 µs 리플레이 검증 (FR-12, 후속 워커)
- 제출 score를 `seed`+옵션으로 **재시뮬**해 EX/clear/판정분포 일치 확인 → `verified` 플래그(capability `verified_scores`).
- µs 입력 통계(등간격·반응속도·동시성)로 매크로/오토 의심 점수화 → `flags:["SUSPECT_TIMING"]`. (휴리스틱, v1은 데이터 적재만.)

### 11.4 제출 감사 로그
- `submission_audit`(score_id·user·build_sha256·platform·ip·ua·accepted·ranked·flags·ts) — 분석·어뷰즈 추적.

---

## 12. FE(web) 전용 엔드포인트 (향후 FE 프로젝트 고려)
> 네이티브 IR(§1-9) 위에 **웹 FE가 바로 쓸 조회/검색/탐색**을 추가(envelope 모드·세션 쿠키·공개 읽기). 모두 페이지네이션·정렬·필터.

- `GET /api/charts/search?q=&mode=&level=&sort=&page=&limit=` → `ChartMeta[]`(title/artist/md5/sha256 검색). 공개.
- `GET /api/charts/{hash}/ranking`(§4) — FE 리더보드(envelope+player_name·rank·replay_id 링크).
- `GET /api/players/{id}` · `GET /api/players/{id}/scores`(§3) — FE 플레이어 페이지.
- `GET /api/players/{id}/recent?limit=` → 최근 플레이 피드(차트 메타 조인).
- `GET /api/activity/recent?limit=` → 전체 최근 제출 피드(홈/대시보드). 공개.
- `GET /api/leaderboards/players?sort=rank_points&page=` → 종합 랭킹(플레이어). 공개.
- `GET /api/replays/{id}`(§8) — FE **리플레이 뷰어**(µs 이벤트 → 재생/그래프).
- `GET /api/stats/summary` → 서버 통계(차트수·플레이어수·제출수) 대시보드. 공개.
- 웹 인증: `POST /api/auth/login`(쿠키 세션), `GET /api/auth/me`, better-auth OAuth(GitHub/Google) 라우트(`/api/auth/*`)도 노출 가능.
> 원칙: 공개 읽기 엔드포인트는 무인증, 쓰기/개인데이터는 세션. FE는 envelope 모드 고정. 신규 엔드포인트는 추가만(기존 IR 계약 불변).

---

## 13. (선택) LR2IR 레거시 호환 어댑터 — `/lr2ir/*`
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
> 제출(`gateway.cgi`)은 난독 바이너리라 **재현 비범위**(PRD §3). 조회만 어댑팅.

---

## 14. DTO 인덱스 (Zod 스키마로 구현 — `dto/`)
- 입력: `accountSchema`·`loginSchema`·`scoreSubmissionSchema`·`courseSubmissionSchema`·`replayUploadSchema`·`settingPutSchema`·`rankingQuerySchema`·`chartUpsertSchema`·`chartSearchQuerySchema`·`clientBuildSchema`.
- 응답: `serverInfoSchema`·`authResultSchema`·`playerProfileSchema`·`chartMetaSchema`·`scoreRecordSchema`·`submitResponseSchema`·`courseMetaSchema`·`tableDataSchema`·`replayMetaSchema`·`settingBlobSchema`.
- 핵심 공유 스키마:
  - `judgeBreakdownSchema`(early/late epg…lms + 합계 미러 + empty_poor + avgjudge).
  - `playOptionsSchema`(**플레이 전체 옵션** — gauge·random·random_p2·option·seed·hispeed·constant·lift·lane_cover·judge_rate·offset_ms·auto_offset·total_override·autoplay·scratch_left·scratch_auto·assist·input_device·judge_algorithm·rule·skin·lntype).
  - 무결성: `client_build_sha256`(`z.string().length(64)`)·`client_platform`. 모든 스키마에 `.extra: z.record(z.any()).optional()`.
- 타입은 `z.infer<typeof …>`로 유도.

> 클라이언트(`rbms-ir`) 현행과의 차이(early/late·seed·avgjudge·empty_poor·전체옵션·client_build_sha256·µs리플레이·settings·courses)는 [compatibility.md] §9 클라이언트-갭에 정리. `rbms-ir` DTO를 이 규격으로 확장하는 것이 클라 측 후속 태스크.
