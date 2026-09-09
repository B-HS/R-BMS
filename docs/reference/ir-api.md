# rbms IR-superset API 계약 — `rbms-ir` 클라이언트 정본

> 이 문서는 **`crates/rbms-ir` 가 실제로 말하는 와이어 계약**이다. 서버 쪽 설계 정본은 [`docs/backend/`](../backend/README.md) 와 `web/src/server/{dto,route}`(zod 스키마 = 진실 소스), 라우트 매트릭스는 [`docs/web/architecture.md`](../web/architecture.md) §1-2 다.
> 계약 원칙: **BMS IR(LR2IR/Mocha/Cinnamon 등)의 슈퍼셋**. MD5+SHA-256 양쪽 차트 id, early/late 판정 분해, µs 리플레이, 코스, 설정 동기화, capability 탐색, 그리고 모든 DTO의 `extra`(free-form)로 전방호환.
> REST + JSON. 네이티브 경로는 **봉투 없이 raw JSON**을 돌려준다(실패 응답만 봉투). 인증은 `Authorization: Bearer <api_token>`.

- base URL: 배포 서버는 `/api` 프리픽스 포함 — 클라 설정은 `--server https://bms.hyuns.uk/api`. `HttpScoreServer` 는 base 의 끝 `/` 만 정규화하고 아래 경로를 그대로 이어 붙인다.
- 모든 요청 본문에 `api_version`(현재 **1**). 서버는 `api_version != 1` 을 400 `IR_API_VERSION_UNSUPPORTED` 로 거절한다.
- 경로·쿼리에 넣는 식별자(player id, blob 이름, replay id, course hash)는 **percent-encoding 하지 않는다.** 호출자가 URL-safe 값을 넘겨야 한다.

---

## 1. 엔드포인트

| 메서드 | 경로 | 인증 | 요청 | 성공 응답 | 트레이트 메서드 |
|---|---|---|---|---|---|
| GET | `/health` | — | — | 200 `ServerInfo` | `health` |
| GET | `/version` | — | — | 200 `VersionInfo` | `version` |
| POST | `/auth/register` | — | `AuthRequest`(email 필수) | 201 `AuthResponse` | `register` |
| POST | `/auth/login` | — | `AuthRequest` | 200 `AuthResponse` | `login` |
| GET | `/auth/me` | Bearer | — | 200 `PlayerProfile` | `whoami` |
| POST | `/scores` | Bearer 또는 **guest 전용 id** | `ScoreSubmission` | 201 `SubmitResponse`(중복 제출은 200) | `submit_score` |
| GET | `/charts/{md5}/ranking?limit=&page=&lnmode=&rival_of=` | — | — | 200 `[ScoreRecord]` | `chart_ranking`, `chart_ranking_page` |
| GET | `/charts/{md5}/best?player={id}` | — | — | 200 `ScoreRecord` 또는 `null` | `player_best` |
| POST | `/charts/{md5}/replays` | Bearer | `ReplayData` | 201 `ReplayUploadResponse` | `upload_replay` |
| GET | `/charts/{md5}/replays?limit=&player=` | — | — | 200 `[ReplayMeta]` | `chart_replays` |
| GET | `/replays/{id}` | — | — | 200 `ReplayData` | `download_replay` |
| GET | `/players/{id}` | — | — | 200 `PlayerProfile` | `player_profile` |
| GET | `/players/{id}/rivals` | — | — | 200 `[PlayerProfile]` | `rivals` |
| PUT | `/players/{id}/rivals` | Bearer, 본인 | `RivalPutRequest` | 200 `[PlayerProfile]` | `put_rivals` |
| GET | `/players/{id}/scores?limit=&page=&since=&mode=` | — | — | 200 `[ScoreRecord]`(`rank` 은 항상 `null`) | `player_scores` |
| GET | `/players/{id}/settings/{name}` | Bearer, 본인 | — | 200 `SettingsBlob` | `get_settings` |
| PUT | `/players/{id}/settings/{name}` | Bearer, 본인 | `SettingsPutRequest` | 200 `SettingsPutResponse`(`{ updated_at }`) | `put_settings` |
| POST | `/courses` | Bearer | `CourseSubmission` | 201 `SubmitResponse`(중복 200) | `submit_course` |
| GET | `/courses/{hash}/ranking?limit=&page=&lnmode=&rival_of=` | — | — | 200 `[ScoreRecord]` | `course_ranking`, `course_ranking_page` |

쿼리 기본값·상한(서버 zod 스키마와 동일, 상수는 `dto::query`):

| 파라미터 | 기본 | 상한 |
|---|---|---|
| ranking `limit` / `page` | 50 / 1 | `MAX_RANKING_LIMIT` = 500 |
| player scores `limit` / `page` | 50 / 1 | `MAX_PLAYER_SCORES_LIMIT` = 100 |
| chart replays `limit` | 50 | `MAX_REPLAY_LIST_LIMIT` = 200 |
| `lnmode` | 없음 | 0~2 |

미등록 차트의 ranking/replays 는 404 가 아니라 **빈 배열**, best 는 **`null`** 이다.

### 비인증 제출은 player id 가 반드시 `guest` 여야 한다

`POST /scores` 를 Bearer 없이 보낼 때 서버는 `player.id` 가 **문자열 `guest`(= `rbms_ir::GUEST_PLAYER_ID`)** 인 경우에만 받는다. 다른 id 는 **401 `UNAUTHORIZED`** 다 — 토큰이 만료된 것이 아니라 *id 가 틀린 것*이므로, 클라의 `is_auth_failure()` 가 true 여도 "다시 로그인" 안내로는 해결되지 않는다.

- 클라이언트 규칙: 세션 토큰이 없으면 **항상** `GUEST_PLAYER_ID` 로 제출한다(`ir_session::submission_player_id`). 로그아웃은 저장된 PLAYER ID 도 `guest` 로 되돌린다 — 안 그러면 로그아웃 이후 모든 기록이 401 로 유실된다.
- 서버가 받아들인 guest 제출은 `GUEST` flag 가 붙어 랭킹에서 제외된다(`message` 예: `recorded (unranked: unknown_build,guest)`).
- 서버 쪽 개선 여지: player id 오류는 401 보다 **400 `VALIDATION_ERROR`** 가 진실에 가깝다.

---

## 2. DTO (필드명·타입 = 아래 그대로)

```jsonc
// ServerInfo — GET /health
{ "name": "rbms-web", "version": "0.1.0", "ir_compat": "superset-1",
  "capabilities": { "ranking": true, "player_best": true, "rivals": true, "courses": true,
                    "replays": true, "tables": true, "settings_sync": true,
                    "accounts": true, "lr2ir_compat": false } }   // 블록·개별 플래그 모두 생략 시 false

// VersionInfo — GET /version
{ "api_version": 1, "server": "rbms-web", "commit": "c6f0885" }   // commit 은 null 가능

// AuthRequest — POST /auth/{register,login}
{ "api_version": 1, "id": "gkn", "password": "…", "email": "gkn@example.com", "name": "gkn" }
// register: email 필수(비밀번호 8자 이상), name 은 null 허용. login: id + password 만 읽는다.

// AuthResponse
{ "token": "…", "player": { "id": "gkn" }, "name": "gkn" }

// PlayerProfile — GET /auth/me, /players/{id}, /players/{id}/rivals
{ "id": "gkn", "name": "gkn", "total_plays": 42, "rank_points": 13.5, "extra": { "rank": "AAA" } }

// RivalPutRequest — PUT /players/{id}/rivals (전체 교체, 빈 배열이면 전부 해제)
{ "rivals": ["rival-a", "rival-b"] }

// ChartId — MD5 는 IR 호환 키, SHA-256 은 슈퍼셋(제출 시 필수)
{ "md5": "32hex", "sha256": "64hex" }

// ScoreSubmission — POST /scores
{ "api_version": 1,
  "chart": { "md5": "…", "sha256": "…" },
  "player": { "id": "guest" },
  "mode": "BEAT_7K",
  "clear": "Hard",                 // NoPlay|Failed|AssistEasy|LightAssistEasy|Easy|Normal|Hard|ExHard|FullCombo|Perfect|Max
  "ex_score": 1488, "max_ex_score": 1624,
  "judge": { "pgreat":712,"great":64,"good":21,"bad":8,"poor":5,"miss":2,
             "fast":30,"slow":40,"combobreak":15,
             "epg":400,"lpg":312,"egr":0,"lgr":0,"egd":0,"lgd":0,"ebd":0,"lbd":0,
             "epr":0,"lpr":0,"ems":0,"lms":0,"avgjudge":-1500,"empty_poor":3 },
  "max_combo": 540, "total_notes": 812, "passnotes": 812, "minbp": 15, "gauge_value": 86.0,
  "options": { "gauge":"Hard", "random":"Random", "random_p2": null, "scratch_auto": false,
               "lntype": 1, "input_device": "keyboard", "assist": [],
               "option": 0, "judge_rate": 100, "offset_ms": 0, "constant": false,
               "hispeed": 3.0, "lift": 0.0, "lane_cover": 0.0, "total_override": 0.0,
               "autoplay": false, "auto_offset": false, "scratch_left": false, "green_number": 310.0 },
  "played_at": 1700000000000,      // unix ms. 음수면 400, 현재보다 5분 이상 미래면 400
  "client": "rbms/0.1.0", "replay_id": null, "seed": 42,
  "judge_algorithm": "Combo", "rule": "", "skin": "NORMAL",
  "client_build_sha256": null, "client_platform": "macos-aarch64",
  "extra": {} }

// 서버 zod 스키마가 Rust 타입보다 좁은 두 필드 — 클라가 보내기 직전에 교정한다
//   played_at:    z.number().int().min(0)  → Rust 는 i64 라 음수도 담기지만 서버는 400.
//   gauge_value:  z.number()               → f32 NaN/Inf 는 JSON `null` 로 직렬화되고,
//                                            zod default 는 undefined 에만 걸리므로 400.
// `ScoreSubmission::clamp_to_server_bounds()` 가 played_at 을 MIN_PLAYED_AT_MS(0) 로 올리고
// 비유한 gauge_value 를 FALLBACK_GAUGE_VALUE(0.0) 로 바꾼다. `spawn_submit` 이 제출 전에 호출하므로
// 정상 경로에서는 이 두 사유의 400 이 나오지 않는다. CourseSubmission 도 같은 두 제약을 받는다.

// SubmitResponse — POST /scores, POST /courses
{ "accepted": true, "rank": 3, "previous_best": 1400, "message": "saved",
  "ranked": true, "flags": [], "is_new_best": true, "score_id": "sc_123", "extra": {} }
// 서버(`submitResponseSchema`)는 `extra` 를 뺀 8개를 항상 보낸다. 클라는 뒤 5개를 serde default 로
// 받으므로 이 필드들을 모르는 구버전 서버 응답도 그대로 디코드된다.
// `score_id` 는 리플레이 업로드를 스코어에 연결하는 키이고(`ReplayData.score_id`),
// `flags`/`ranked` 는 `unranked_summary()` 와 `is_replay_upload_warranted` 가 읽는다.

// ScoreRecord — ranking / best / player scores
{ "player": {"id":"gkn"}, "player_name": "gkn", "clear": "ExHard",
  "ex_score": 1600, "max_combo": 812, "minbp": 1, "rank": 1, "played_at": 1700000000000,
  "lntype": 1, "option": 0, "total_notes": 812, "judge": null, "extra": {} }

// CourseSubmission — POST /courses
{ "api_version": 1, "course_hash": "…", "player": {"id":"gkn"},
  "clear": "Normal", "ex_score": 4800, "max_ex_score": 6000, "judge": { … },
  "max_combo": 900, "minbp": 12, "gauge_value": 42.0, "lntype": 1,
  "charts": [{"md5":"…","sha256":"…"}], "trophy": "bronzemedal",
  "played_at": 1700000000000, "extra": {} }

// ReplayData — POST /charts/{md5}/replays 본문 이자 GET /replays/{id} 응답
{ "api_version": 1, "id": null, "format": "rbms-us-v1",
  "chart": {"md5":"…","sha256":"…"},   // 경로 해시가 서버에 없을 때의 보조 해석 키.
                                        // 다운로드는 저장된 차트 행을 조인해 양쪽 해시를 채운다
                                        // (등록된 md5 가 없으면 "")
  "score_id": "sc_123",                 // SubmitResponse.score_id — 스코어↔리플레이 연결
  "mode": "BEAT_7K", "random": "Random", "random_p2": null, "seed": 42,
  "lntype": 1, "offset_ms": 0, "judge_rate": 100, "scratch_auto": false, "constant": false,
  "gauge": "Hard", "client_build_sha256": null,
  "events": [{ "t_us": 0, "lane": 1, "press": true }],   // µs 절대 시각
  "event_count": null, "duration_us": null, "size": null, // null 이면 서버가 events 에서 유도
  "extra": {} }

// ReplayUploadResponse — 201
{ "id": "rp_77", "url": "/api/replays/rp_77", "event_count": 2 }

// ReplayMeta — GET /charts/{md5}/replays
// url 은 **배포 오리진 기준**이라 이미 배포의 `/api` 프리픽스를 포함한다. 설정 base
// (`https://bms.hyuns.uk/api`)와 이어 붙이면 `/api/api/...` 가 되므로 join 하지 말고
// `ReplayMeta::download_id()` + `download_replay(id)` 를 쓴다.
{ "id": "rp_77", "url": "/api/replays/rp_77", "player": {"id":"gkn"}, "player_name": "gkn",
  "chart_sha256": "…", "score_id": "sc_123", "format": "rbms-us-v1", "mode": "BEAT_7K",
  "seed": 7, "lntype": 1, "event_count": 2, "duration_us": 120000, "size": 128,
  "client_build_sha256": null, "created_at": 1700000000000 }

// SettingsBlob — GET /players/{id}/settings/{name}
{ "name": "keyconfig", "format": "ron", "content": "(keys:[1,2,3])", "updated_at": 1700000000000 }

// SettingsPutRequest — PUT /players/{id}/settings/{name}
{ "api_version": 1, "name": "keyconfig", "format": "ron", "content": "…",
  "updated_at": 1700000000000,
  "base_updated_at": 1699000000000,   // 낙관적 잠금. null 이면 무조건 덮어쓴다
  "extra": {} }

// SettingsPutResponse — 성공한 PUT 의 200 본문. 서버가 실제로 저장한 stamp(unix ms)다.
// 클라가 보낸 updated_at 은 무시되므로, 다음 조건부 쓰기의 base 는 이 값이어야 한다.
{ "updated_at": 1700000000042 }
// 이 필드를 모르는 구버전 서버는 여전히 204 No Content 로 답한다 →
// `SettingsPutResult { updated_at: 보낸 값, from_server: false }` 로 떨어지고 클라가 GET 으로 재조회한다.

// SettingsConflict — PUT 이 잠금에서 밀렸을 때의 409 본문
{ "conflict": true, "server": { "name": "keyconfig", "format": "ron",
                                "content": "…", "updated_at": 1700000000000 } }
```

**enum 표기**: `ClearLamp`, `GaugeType`(AssistEasy|Easy|Normal|Hard|ExHard|Hazard|Class|ExClass|ExHardClass), `RandomOption`(Off|Mirror|Random|RRandom|SRandom|Spiral|HRandom|AllScratch|Converge) 는 **variant 이름 그대로** JSON 문자열이며 대소문자를 구분한다.

### `SubmitResponse.flags` — 서버 `SCORE_FLAG` 문자열

| 문자열 | `ScoreFlag` | 랭킹 차단 |
|---|---|---|
| `AUTOPLAY` | `Autoplay` | O |
| `SCRATCH_AUTO` | `ScratchAuto` | O |
| `ASSIST` | `Assist` | O |
| `JUDGE_WIDTH` | `JudgeWidth` | O |
| `TOTAL_OVERRIDE` | `TotalOverride` | O |
| `UNKNOWN_BUILD` | `UnknownBuild` | X (서버가 신뢰 빌드를 강제할 때만 막는다) |
| `GUEST` | `Guest` | O |

모르는 문자열은 버리지 않고 `flags` 에 원문으로 남는다(`SubmitResponse::unknown_flags`). 랭킹을 막는 것만 추리려면 `unranked_reasons()`, 한 줄 사유는 `unranked_summary()`.

---

## 3. 실패 응답과 에러 매핑

실패는 항상 봉투다: `{"success": false, "error": {"code": "…", "message": "…", "details": { … }}}` (`details` 는 비프로덕션에서만).

| 상태 | `IrError` | 대표 `error.code` |
|---|---|---|
| 401 | `Unauthorized(body)` | `UNAUTHORIZED` |
| 403 | `Forbidden(body)` | `FORBIDDEN` |
| 404 | `NotFound(body)` | `IR_CHART_NOT_FOUND` · `IR_PLAYER_NOT_FOUND` · `IR_REPLAY_NOT_FOUND` · `IR_SETTING_NOT_FOUND` · `IR_COURSE_NOT_FOUND` |
| 409 | `Conflict(body)` | `IR_ACCOUNT_EXISTS` |
| 409 (settings PUT, 잠금 본문) | `SettingsConflict(server copy)` | — |
| 413 | `PayloadTooLarge(body)` | `IR_PAYLOAD_TOO_LARGE` |
| 429 | `RateLimited(body)` | `RATE_LIMITED` |
| 그 외 4xx/5xx | `Server(status, body)` | `VALIDATION_ERROR` · `IR_API_VERSION_UNSUPPORTED` · `INTERNAL_ERROR` |
| 전송 실패·타임아웃 | `Network(reason)` | — |
| 2xx 인데 본문이 계약과 다름 | `Decode(reason)` | — |
| `--server` 미설정(`NullScoreServer`) | `NotConfigured` | — |
| 서버가 지원하지 않는 트레이트 메서드 | `Unsupported` | — |

- settings PUT 의 409 본문이 잠금 봉투로 파싱되지 않으면 일반 `Conflict(body)` 로 떨어진다.
- `IrError::status()` · `detail()` · `error_code()` 로 상태·원문·서버 코드를 꺼낸다. `is_auth_failure()` 는 401/403, `is_retryable()` 은 네트워크·429·5xx.
- `Display` 는 봉투를 풀어 `not signed in (401): 인증이 필요합니다 (UNAUTHORIZED)` 처럼 렌더하고, 본문은 `ERROR_DETAIL_MAX_CHARS`(200자)에서 자른다.

---

## 4. 클라이언트 동작

- `HttpScoreServer::try_new(base, token)` — reqwest blocking + rustls, 요청당 `REQUEST_TIMEOUT`(5초). 빌더 실패는 삼키지 않고 *degraded* 상태로 남아(`degraded()`) 모든 호출이 `Network` 로 즉시 실패한다.
- 토큰 교체는 `with_token(Some(token))` / 로그아웃은 `with_token(None)`. 토큰이 있으면 **모든** 호출에 `Authorization: Bearer` 가 붙고, 없으면 붙지 않아 공개 읽기가 그대로 동작한다.
- 블로킹 호출은 프레임 루프에서 직접 부르지 않는다. `spawn_query` 로 단발 조회를, `spawn_submit` 으로 제출을 백그라운드 스레드에 넘기고 `Receiver` 를 프레임마다 `try_recv` 한다.
- `spawn_submit(server, SubmitJob { submission, replay })`:
  0. `ScoreSubmission::clamp_to_server_bounds()` 로 `played_at`·`gauge_value` 를 서버 허용 범위로 교정(위 §2 주석).
  1. `/scores` 제출.
  2. 성공이고 랭킹을 막는 flag 가 없으면(`is_replay_upload_warranted`) `SubmitResponse.score_id` 를 `ReplayData.score_id` 에 넣고, 비어 있던 `ReplayData.chart` 를 제출 차트로 채운 뒤 `/charts/{md5}/replays` 로 업로드.
  3. `SubmitOutcome { submit, replay }` 를 채널로 전달 — 제출 결과와 업로드 결과를 각각 그대로 볼 수 있다(`replay_id()`, `is_accepted()`).
- 설정 동기화 절차: `get_settings` → 편집 → `base_updated_at = 서버 updated_at` 으로 `put_settings`. 409 `SettingsConflict` 는 서버 사본을 함께 실어 오므로 재조회 없이 병합/덮어쓰기를 결정할 수 있다.
- **성공한 PUT 은 저장한 `updated_at` 을 200 본문으로 돌려준다.** 서버는 클라가 보낸 `updated_at` 을 무시하고 자기 시각을 찍으므로, 새 잠금 값은 이 응답으로만 알 수 있다. `put_settings` 는 이를 `SettingsPutResult { updated_at, from_server: true }` 로 돌려주고 클라가 곧바로 잠금을 갱신한다 — 조건부 PUT 앞의 추가 GET 은 필요 없다.
- **204 로 답하는 구버전 서버**는 `from_server: false` + 보낸 stamp 로 떨어진다. 이 stamp 는 추측이라 잠금에 넣지 않고(`ir_sync::upload_outcome` 이 `None`), 클라(`App::refresh_sync_base`)가 `get_settings` 를 한 번 더 돌려 진짜 값을 읽는다. 이 재조회가 없으면 그런 서버에서는 한 세션의 **두 번째 저장부터 항상 409** 다.
- **409 는 잠금을 전진시키지 않는다.** 409 본문의 `server.updated_at` 을 그대로 다음 base 로 삼으면 바로 다음 업로드가 성공해 "먼저 다운로드하라"던 그 새 서버 사본을 덮어쓴다. 잠금 전이는 `ir_sync::SyncLock`/`SyncOutcome` 한 곳에 모여 있다: `Read(updated_at)` 와 `Uploaded(updated_at)`(= 서버가 알려준 stamp) 만 잠금을 설정하고, `Conflict` 는 그대로 두며, `SignedOut` 은 지운다.
- 미설정 기본값 `NullScoreServer` 는 8개 필수 메서드에 `NotConfigured`, 슈퍼셋 확장 메서드에 `Unsupported` 를 돌려준다(플레이 무영향, 연결 표시 빨강).

---

## 5. 알려진 갭 (서버 쪽 후속)

- **리플레이의 `extra` 는 저장되지 않는다.** `replay` 테이블에 해당 컬럼이 없어 업로드 본문의 `extra` 가 버려지고 다운로드는 항상 `{}` 다. 나머지 필드는 전부 왕복한다(`tests/service/replay.test.ts` 가 업로드 본문 전체와 다운로드를 deep-equal 로 비교). 채우려면 컬럼 추가 + 마이그레이션이 필요하다.
- **`gauge` 를 생략한 업로드는 `AssistEasy` 로 되돌아온다.** 컬럼이 `NOT NULL DEFAULT 0` 이라 "미지정"과 0번 게이지를 구분하지 못한다. 클라(`ir_replay::to_ir_replay`)는 항상 게이지를 보내므로 정상 경로에는 영향이 없다.
- 경로·쿼리 percent-encoding 은 `url` 크레이트 신규 의존이 필요해 보류 중이다(현재는 URL-safe 식별자 전제).
- 차트 메타 업서트(`POST /charts`), 난이도표(`/tables`), 코스 메타 업서트(`POST /courses/meta`), 스코어 단건 조회(`GET /scores/{id}`) 는 서버에 있으나 `rbms-ir` 클라이언트에는 아직 없다.
