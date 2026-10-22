# rbms IR-superset API 계약 — `rbms-ir` 클라이언트 정본

> 최종 갱신 2026-10-22 · 대응 커밋 47a7f9c (2026-10-22 에 `ScoreRecord.assist_grade` 만 더했다. 나머지는 2026-10-15 기준)

> **상태:** 현재 IR 계약과 구현된 경계의 정본입니다. 서버는 https://bms.hyuns.uk 에 배포됐고(2026-10-14) 클라이언트 기본 서버 주소가 이 주소입니다. 아래 JSON의 해시·시간·커밋 값은 **형식 예시**이며 현재 배포 식별자가 아닙니다. 알려진 클라이언트 갭은 본문에서 보존하고, 실제 수동 다중 IR/서버 QA는 [roadmap](../roadmap.md)의 미실시 항목입니다. 필드 규칙이 코드와 다르면 서버 zod 스키마(`web/src/server/dto/`)와 `crates/rbms-ir/src/dto/` 가 맞습니다.

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
| DELETE | `/auth/token/current` | Bearer(폐기할 토큰) | — | 204 (모르거나 이미 폐기된 토큰은 401) | `revoke_current_token` (로그아웃) |
| POST | `/scores` | Bearer 또는 **guest 전용 id** | `ScoreSubmission` | 201 `SubmitResponse`(중복 제출은 200) | `submit_score` |
| GET | `/charts/{md5}/ranking?limit=&page=&lnmode=&rival_of=` | — | — | 200 `[ScoreRecord]` | `chart_ranking`, `chart_ranking_page` |
| GET | `/charts/{md5}/best?player={id}` | — | — | 200 `ScoreRecord` 또는 `null` | `player_best` |
| POST | `/charts/{md5}/replays` | Bearer | `ReplayData` | 201 `ReplayUploadResponse` | `upload_replay` |
| GET | `/charts/{md5}/replays?limit=&player=` | — | — | 200 `[ReplayMeta]` | `chart_replays` |
| POST | `/charts/{md5}/traces` | Bearer, 점수의 주인 | `PlayTrace`(`score_id` 필수) | 201 `TraceUploadResponse`(같은 점수의 재전송은 200, 같은 id) | `upload_trace` |
| GET | `/replays/{id}` | — | — | 200 `ReplayData` | `download_replay` |
| GET | `/players/{id}` | — | — | 200 `PlayerProfile` | `player_profile` |
| GET | `/players/{id}/rivals` | — | — | 200 `[PlayerProfile]` | `rivals` |
| PUT | `/players/{id}/rivals` | Bearer, 본인 | `RivalPutRequest` | 200 `[PlayerProfile]` | `put_rivals` |
| GET | `/players/{id}/scores?limit=&page=&since=&mode=` | — | — | 200 `[ScoreRecord]`(`rank` 은 항상 `null`) | `player_scores` |
| GET | `/players/{id}/settings/{name}` | Bearer, 본인 | — | 200 `SettingsBlob` | `get_settings` |
| PUT | `/players/{id}/settings/{name}` | Bearer, 본인 | `SettingsPutRequest` | 200 `SettingsPutResponse`(`{ updated_at }`) | `put_settings` |
| POST | `/courses` | Bearer | `CourseSubmission` | 201 `SubmitResponse`(중복 200) | `submit_course` |
| GET | `/players/{id}/folders` | Bearer 또는 세션, 본인 | — | 200 `{ "folders": [FolderSummary] }`(`Cache-Control: private, no-store`) | `user_folders` |
| GET | `/players/{id}/folders/{folderId}` | Bearer 또는 세션, 본인 | — | 200 `FolderSummary` + `items`(`md5`·`sha256`·`title`·`artist`·`note`) | `user_folder` |
| GET | `/courses/{hash}/ranking?limit=&page=&lnmode=&rival_of=` | — | — | 200 `[ScoreRecord]` | `course_ranking`, `course_ranking_page` |

쿼리 기본값·상한(서버 zod 스키마와 동일, 상수는 `dto::query`):

| 파라미터 | 기본 | 상한 |
|---|---|---|
| ranking `limit` / `page` | 50 / 1 | `MAX_RANKING_LIMIT` = 500 |
| player scores `limit` / `page` | 50 / 1 | `MAX_PLAYER_SCORES_LIMIT` = 100 |
| chart replays `limit` | 50 | `MAX_REPLAY_LIST_LIMIT` = 200 |
| `lnmode` | 없음 | 0~2 |
| `include_scratch_auto`(ranking·best, 웨이브 20) | `false` | `true`·`false` |

폴더 두 경로의 필드·한도·오류는 [backend/api-spec.md](../backend/api-spec.md) 7.2 절이 정본이다(편집은 웹의 세션 경로만 한다. 클라이언트는 읽기만 하고, 응답은 `get_bounded` 로 크기·시간을 제한해 읽는다. `GET /health` 의 `capabilities.folders` 가 `true` 여야 서버가 지원한다).

`include_scratch_auto` 를 빼면(클라이언트는 싣지 않는다) 일반 판 최고만의 기본 목록이다. `true` 면 플레이어마다 일반 판 최고와 SCRATCH AUTO 판 최고 가운데 더 높은 것이고, SCRATCH AUTO 판 행은 `extra.scratch_auto: true` 를 단다. 곡 선택의 IR RANKING 패널은 기본 목록을 쓴다.

미등록 차트의 ranking/replays 는 404 가 아니라 **빈 배열**, best 는 **`null`** 이다.

### 비인증 제출은 player id 가 반드시 `guest` 여야 한다

`POST /scores` 를 Bearer 없이 보낼 때 서버는 `player.id` 가 **문자열 `guest`(= `rbms_ir::GUEST_PLAYER_ID`)** 인 경우에만 받는다. 다른 id 는 **401 `UNAUTHORIZED`** 다 — 토큰이 만료된 것이 아니라 *id 가 틀린 것*이므로, 클라의 `is_auth_failure()` 가 true 여도 "다시 로그인" 안내로는 해결되지 않는다.

- 클라이언트 규칙: 세션 토큰이 없으면 **항상** `GUEST_PLAYER_ID` 로 제출한다(`ir_session::submission_player_id`). 로그아웃은 저장된 PLAYER ID 도 `guest` 로 되돌린다 — 안 그러면 로그아웃 이후 모든 기록이 401 로 유실된다.
- 반대쪽도 같다: 토큰이 있는데 `player.id` 가 토큰 주인이 아니면 **403 `FORBIDDEN`** 이다(점수·코스 제출, 설정, 라이벌, 폴더). 그래서 로그인 중인 클라이언트가 계정으로 내는 요청은 모두 **서버가 토큰의 주인으로 알려 준 login id**(로그인 응답, `whoami`. `AccountSession::signed_in_as`)만 쓴다. 설정의 PLAYER ID 는 다음 LOGIN·REGISTER 에 쓸 입력일 뿐 요청의 주인이 아니다. 토큰은 있는데 주인을 아직 모르면(저장된 login id 가 없는 오래된 설정을 막 연 때) 계정 요청은 만들지 않고 `account not confirmed yet` 을 띄운다.
- 서버가 받아들인 guest 제출은 `GUEST` flag 가 붙어 랭킹에서 제외된다(`message` 예: `recorded (unranked: unknown_build,guest)`).
- 서버 쪽 개선 여지: player id 오류는 401 보다 **400 `VALIDATION_ERROR`** 가 진실에 가깝다.

---

## 2. DTO (필드명·타입 = 아래 그대로)

```jsonc
// ServerInfo — GET /health
{ "name": "rbms-ir", "version": "0.1.0", "ir_compat": "superset-1",
  "capabilities": { "ranking": true, "player_best": true, "rivals": true, "courses": true,
                    "replays": true, "tables": true, "settings_sync": true, "folders": true,
                    "accounts": true, "lr2ir_compat": false, "traces": true } }   // 블록·개별 플래그 모두 생략 시 false

// VersionInfo — GET /version
{ "api_version": 1, "server": "0.1.0", "commit": "c6f0885" }   // commit 은 null 가능(Git 연동 배포가 아니면)

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
  "chart": { "md5": "…", "sha256": "…", "title": "…", "artist": "…", "level": 12 },   // title·artist·level 은 선택(`SubmittedChart`)
  "player": { "id": "guest" },
  "mode": "BEAT_7K",
  "clear": "Hard",                 // NoPlay|Failed|AssistEasy|LightAssistEasy|Easy|Normal|Hard|ExHard|FullCombo|Perfect|Max
  "ex_score": 1488, "max_ex_score": 1624,
  "judge": { "pgreat":712,"great":64,"good":21,"bad":8,"poor":5,"miss":2,
             "fast":30,"slow":40,"combobreak":15,
             "epg":400,"lpg":312,"egr":0,"lgr":0,"egd":0,"lgd":0,"ebd":0,"lbd":0,
             "epr":0,"lpr":0,"ems":0,"lms":0,"avgjudge":11500,"empty_poor":3 },   // avgjudge: 레퍼런스 정의(평균 거리 µs, 규칙 세대 4 부터)
  "max_combo": 540, "total_notes": 812, "passnotes": 812, "minbp": 15, "gauge_value": 86.0,
  "options": { "gauge":"Hard", "random":"Random", "random_p2": null, "scratch_auto": false,
               "lntype": 1, "input_device": "keyboard", "assist": [],
               "option": 0, "judge_rate": 100, "offset_ms": 0, "constant": false,
               "hispeed": 3.0, "lift": 0.0, "lane_cover": 0.0, "total_override": 0.0,
               "autoplay": false, "auto_offset": false, "scratch_left": false, "green_number": 310.0 },
  "played_at": 1700000000000,      // unix ms. 음수면 400, 현재보다 5분 이상 미래면 400
  "client": "rbms/0.1.0", "replay_id": null, "seed": 42,   // seed: 클라 u64. 서버는 2^53 를 넘는 값(옛 빌드의 나노초)을 거절하지 않고 0 으로 저장한다
  "judge_algorithm": "Duration", "rule": "", "skin": "NORMAL",
  "client_build_sha256": null, "client_platform": "macos-aarch64",
  "extra": {} }

// 서버 zod 스키마가 Rust 타입보다 좁은 두 필드 — 클라가 보내기 직전에 교정한다
//   played_at:    z.number().int().min(0)  → Rust 는 i64 라 음수도 담기지만 서버는 400.
//   gauge_value:  z.number()               → f32 NaN/Inf 는 JSON `null` 로 직렬화되고,
//                                            zod default 는 undefined 에만 걸리므로 400.
// `ScoreSubmission::clamp_to_server_bounds()` 가 played_at 을 MIN_PLAYED_AT_MS(0) 로 올리고
// 비유한 gauge_value 를 FALLBACK_GAUGE_VALUE(0.0) 로 바꾼다. `spawn_submit` 과 기록 동기화(점수·코스)가
// 제출 전에 호출하므로 정상 경로에서는 이 두 사유의 400 이 나오지 않는다. CourseSubmission 도 같은 두 제약을 받는다.
//
// extra 의 키(클라이언트가 쓰는 것, `dto/score.rs`):
//   rule_version    (RULE_VERSION_EXTRA_KEY)  기록 규칙 세대(지금 4). 세대 4 부터 judge.avgjudge 가 레퍼런스 정의다.
//                   이 키가 없는 본문의 avgjudge 는 옛 정의(부호 있는 평균 오프셋)다.
//   mean_offset_us  (MEAN_OFFSET_EXTRA_KEY)   치우침: 친 노트의 부호 있는 평균 오프셋 µs(양수 = 빠름). 엔진의 avg_judge_us.
//   reconstructed   (RECONSTRUCTED_EXTRA_KEY) true 면 끝날 때 남긴 본문이 없어 로컬 기록 행으로 다시 만든 본문이다
//                   (fast/slow·옵션·빌드가 없다. play-options.md 10.9절 "재구성").

// SubmitResponse — POST /scores, POST /courses
{ "accepted": true, "rank": 3, "previous_best": 1400, "message": "saved",
  "ranked": true, "flags": [], "is_new_best": true, "score_id": "sc_123", "extra": {} }
// 서버(`submitResponseSchema`)는 `extra` 를 뺀 8개를 항상 보낸다. 클라는 뒤 5개를 serde default 로
// 받으므로 이 필드들을 모르는 구버전 서버 응답도 그대로 디코드된다.
// 멱등(기록 동기화의 전제): 로그인한 사용자·차트(sha256)·같은 1초 칸(played_at 을 1000ms 로 내림)의 판이 이미
// 있으면 서버는 새로 저장하지 않고 200 과 message "duplicate submission ignored"(`DUPLICATE_SUBMISSION_MESSAGE`),
// 저장된 판의 score_id 로 답한다. 저장된 판이 ranked 면 그 값으로 최고 기록을 한 번 더 조건부 갱신한다
// (점수는 커밋됐는데 최고 기록 갱신이 실패한 판을 다시 보내면 고쳐진다). 클라는 `SubmitResponse::is_duplicate` 로
// "이미 서버에 있음"을 센다. 같은 판을 거의 같은 순간에 두 번 보내면 늦은 쪽이 500 일 수 있다(테스트 부채 3.72).
// `score_id` 는 리플레이 업로드를 스코어에 연결하는 키이고(`ReplayData.score_id`),
// `flags`/`ranked` 는 `unranked_summary()` 와 `is_replay_upload_warranted` 가 읽는다.

// ScoreRecord — ranking / best / player scores
{ "player": {"id":"gkn"}, "player_name": "gkn", "clear": "ExHard", "assist_grade": 0,
  "ex_score": 1600, "max_combo": 812, "minbp": 1, "rank": 1, "played_at": 1700000000000,
  "lntype": 1, "option": 0, "total_notes": 812, "judge": null, "extra": {} }
// assist_grade (2026-10-22): 응답에만 있는 최상위 정수 0..4, 없으면 0(Rust `#[serde(default)] u8`, `crates/rbms-ir/src/dto/score.rs:203`).
//   `clear` 가 LightAssistEasy 일 때만 0 이 아닐 수 있다. SCRATCH AUTO 로 클리어한 게이지를 이름한다:
//   1 ASSIST EASY(EASY 게이지), 2 ASSIST CLEAR(NORMAL·CLASS), 3 ASSIST HARD(HARD·EXCLASS), 4 ASSIST EXHARD(EX-HARD·HAZARD·EXHARDCLASS).
//   0 은 등급 없음(LIGHT ASSIST 그대로이거나 다른 램프). 서버가 `clear` 와 `score.gauge` 에서 파생한다(`web/src/server/service/domain/score/assist-grade.ts`).
//   ScoreRecord 를 돌려주는 모든 응답(차트 순위, best, 플레이어 점수 목록, 코스 순위)에 실린다. 코스 순위 행은 항상 0 이다(코스 기록에 게이지가 없다).
//   순위 정렬은 clear → assist_grade → EX → BP 다. 제출 본문(ScoreSubmission, CourseSubmission)과 SubmitResponse 는 바뀌지 않았다.
//   구 클라이언트는 모르는 필드를 무시하고(`deny_unknown_fields` 없음) 등급 없이 LIGHT ASSIST 로 본다. 서버가 0..4 밖의 값을 보내면 그 응답 전체의 역직렬화가 실패한다.

// CourseSubmission — POST /courses
{ "api_version": 1, "course_hash": "…", "player": {"id":"gkn"},
  "clear": "Normal", "ex_score": 4800, "max_ex_score": 6000, "judge": { … },
  "max_combo": 900, "minbp": 12, "gauge_value": 42.0, "lntype": 1,
  "charts": [{"md5":"…","sha256":"…"}], "trophy": "bronzemedal",
  "scratch_auto": false,    // 선택. 어느 스테이지든 SCRATCH AUTO 였으면 true
  "assist": [],             // 선택. 스테이지들의 어시스트 태그 합(AUTO_SCRATCH, CUSTOM_JUDGE)
  "played_at": 1700000000000, "extra": {} }
// scratch_auto/assist 는 단일 곡 options 와 같은 어휘이고 코스 전체로 접은 값이다. 어시스트가 없으면
// 클라는 두 필드를 싣지 않아 본문이 전과 같고, 이 필드를 모르는 구버전 서버도 거절하지 않는다.
// 서버(웨이브 20)는 SCRATCH AUTO 만 낀 코스를 ranked=true + flags ["SCRATCH_AUTO"] 로 저장하고
// SCRATCH AUTO 칸의 코스 베스트에 넣는다(rank 는 SCRATCH AUTO 포함 목록 기준). CUSTOM_JUDGE 가 낀 코스는
// ranked=false + ASSIST 로 저장하고 코스 순위·베스트에 넣지 않는다.

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

// PlayTrace — POST /charts/{md5}/traces 본문(형식 정본: crates/rbms-ir/src/dto/trace.rs, 계약 파일 contract/client-requests/trace-*.json)
{ "format": "rbms-trace", "version": 1, "score_id": "sc_1", "chart": { "md5": "…", "sha256": "…" },
  "mode": "BEAT_7K", "lntype": 0, "random": "Off", "auto_lanes": [],
  "notes": { "time_us": [1600000], "lane": [0], "kind": [0], "state": [2], "err_us": [-40000] },   // 길이가 같은 열
  "empty_poors": { "time_us": [], "lane": [] },
  "gauge": { "interval_ms": 500, "selected": 2, "series": [[20.0, 22.5]] }, "gauge_bounds": [{ "min": 2.0, "max": 100.0, "border": 80.0 }],
  "failed_at_ms": null, "play_time_ms": 12800,
  "speed": { "start": { … }, "end": { … }, "changes": [] }, "tempo": [{ "t_us": 0, "bpm": 150.0, "stop_us": 0, "scroll": 1.0 }] }
// 서버는 재판정하지 않고 점수 행과 판정 합계·FAST/SLOW·空POOR 를 허용 오차 없이 맞춘다(규칙: web/architecture.md 13-1).
// 2 MiB(TRACE_MAX_BYTES) 초과 413, 남의 점수·없는 점수 404 IR_SCORE_NOT_FOUND, 어긋나면 400 IR_TRACE_MISMATCH { field, reason }, 30/min/사용자.
// 읽기 경로는 없다(본인 분석은 웹 화면만).

// TraceUploadResponse — 201(처음) / 200(같은 점수에 이미 있음)
{ "id": "tr_…", "analysis": "stored", "storage": { "traces": 1, "bytes": 9267, "limit_traces": 5000, "limit_bytes": 536870912, "level": "ok" } }
// 클라이언트는 id 만 읽는다. storage.level 은 ok·near·over 이고 over 여도 저장한다(ADR Q17).

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
| `SCRATCH_AUTO` | `ScratchAuto` | X (웨이브 20부터 표지만. SCRATCH AUTO 판은 ranked 이고 `rank` 는 SCRATCH AUTO 포함 목록 기준이며, 결과 화면은 `IR: RANK #n (SCRATCH AUTO)` 로 쓴다) |
| `ASSIST` | `Assist` | O |
| `JUDGE_WIDTH` | `JudgeWidth` | O |
| `TOTAL_OVERRIDE` | `TotalOverride` | O |
| `UNKNOWN_BUILD` | `UnknownBuild` | X (서버가 신뢰 빌드를 강제할 때만 막는다) |
| `GUEST` | `Guest` | O |

모르는 문자열은 버리지 않고 `flags` 에 원문으로 남는다(`SubmitResponse::unknown_flags`). 랭킹을 막는 것만 추리려면 `unranked_reasons()`, 한 줄 사유는 `unranked_summary()`.

---

## 3. 실패 응답과 에러 매핑

실패는 항상 봉투다: `{"success": false, "error": {"code": "…", "message": "…", "details": { … }}}` (`details` 는 비프로덕션에서는 그대로 나가고, 프로덕션에서는 문자열인 `field`·`reason` 만 나간다).

| 상태 | `IrError` | 대표 `error.code` |
|---|---|---|
| 401 | `Unauthorized(body)` | `UNAUTHORIZED` |
| 403 | `Forbidden(body)` | `FORBIDDEN` |
| 404 | `NotFound(body)` | `IR_CHART_NOT_FOUND` · `IR_PLAYER_NOT_FOUND` · `IR_SCORE_NOT_FOUND`(trace 업로드의 `score_id`) · `IR_REPLAY_NOT_FOUND` · `IR_SETTING_NOT_FOUND` · `IR_COURSE_NOT_FOUND` · `IR_TABLE_NOT_FOUND` · `IR_FOLDER_NOT_FOUND` |
| 409 | `Conflict(body)` | `IR_ACCOUNT_EXISTS` · `IR_FOLDER_NAME_TAKEN` · `IR_FOLDER_LIMIT_EXCEEDED` · `IR_FOLDER_CONFLICT`(폴더 편집은 웹 전용이라 클라이언트는 받지 않는다) |
| 409 (settings PUT, 잠금 본문) | `SettingsConflict(server copy)` | — |
| 413 | `PayloadTooLarge(body)` | `IR_PAYLOAD_TOO_LARGE` |
| 429 | `RateLimited(body)` | `RATE_LIMITED` |
| 그 외 4xx/5xx | `Server(status, body)` | `VALIDATION_ERROR` · `IR_API_VERSION_UNSUPPORTED` · `IR_TRACE_MISMATCH`(400, trace 와 점수 행이 다름) · `INTERNAL_ERROR` |
| 전송 실패·타임아웃 | `Network(reason)` | — |
| 2xx 인데 본문이 계약과 다름 | `Decode(reason)` | — |
| `--server` 미설정(`NullScoreServer`) | `NotConfigured` | — |
| 서버가 지원하지 않는 트레이트 메서드 | `Unsupported` | — |

- settings PUT 의 409 본문이 잠금 봉투로 파싱되지 않으면 일반 `Conflict(body)` 로 떨어진다.
- 웹 전용 경로(AI 해석 `/api/fe/me/ai/*`, 관리자 곡별 패턴 `/api/admin/chart-patterns/*`)의 코드 `IR_CHART_PATTERN_INVALID`(400) · `AI_CREDENTIAL_INVALID`(400) · `AI_PROVIDER_QUOTA`(402) · `AI_PROVIDER_FORBIDDEN`(403) · `AI_CREDENTIAL_NOT_FOUND`(404) · `AI_NOT_ENOUGH_DATA` · `AI_MODEL_UNAVAILABLE` · `AI_PROVIDER_REFUSED`(422) · `AI_PROVIDER_RATE_LIMITED` · `AI_USAGE_LIMIT`(429) · `AI_PROVIDER_BAD_REQUEST` · `AI_PROVIDER_UNAVAILABLE` · `AI_OUTPUT_TRUNCATED` · `AI_OUTPUT_INVALID`(502) 는 게임 클라이언트가 부르지 않는 경로의 것이라 클라이언트 코드에는 없다. 만나면 위 표의 `Server(status, body)` 로 떨어지고 429 는 `RateLimited` 다. 뜻과 `details` 는 [api-spec.md](../backend/api-spec.md) 0.2.1 이 정본이다.
- `IrError::status()` · `detail()` · `error_code()` 로 상태·원문·서버 코드를 꺼낸다. `is_auth_failure()` 는 401/403, `is_retryable()` 은 네트워크·429·5xx.
- `Display` 는 봉투를 풀어 `not signed in (401): 인증이 필요합니다 (UNAUTHORIZED)` 처럼 렌더하고, 본문은 `ERROR_DETAIL_MAX_CHARS`(200자)에서 자른다. 봉투의 `details` 에 `field` 가 있으면 메시지 대신 코드·필드·이유를 쓴다: `server error 400: VALIDATION_ERROR: judge (judge count exceeds total_notes)`(필드만 있으면 `코드: 필드`). `IrError::server_reason()` 이 그 셋(`ServerReason { code, field, reason }`)을 꺼낸다.
- 기록 동기화가 답을 어떻게 다루는지는 `rbms_ir::sync`(`submit_verdict`·`upload_verdict`)가 한 곳에서 정한다: 수락(중복 포함), 4xx 와 수락되지 않은 2xx 는 거절(`Rejection`: 상태·코드·필드·이유), 429 는 대기, 네트워크·디코드·5xx 는 일시 실패, 401·서버 없음·`Unsupported` 는 동기화 중단. 규칙과 클라이언트 쪽 처리는 [crates.md](../crates.md) rbms-ir 의 `sync` 와 [play-options.md](../play-options.md) 10.9절이 정본이다.

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
- **로그아웃**은 로컬 토큰을 지우고, 서버가 설정돼 있으면 일꾼 스레드에서 `revoke_current_token`(`DELETE /auth/token/current`)을 보내 그 토큰을 서버에서도 폐기한다. 401(이미 모르는 토큰)은 목적을 이룬 것으로 보고, 네트워크 실패면 `logged out, but the server did not revoke the token (delete it on the web site)` 를 알림에 남긴다. 어느 쪽이든 로그아웃은 이 기기에서 끝난다. **로그인·가입**은 쓰던 토큰을 Bearer 로 싣고 가므로(`HttpScoreServer` 는 토큰이 있으면 모든 호출에 싣는다) 서버가 성공한 뒤 그 토큰을 폐기한다.
- **계정이 바뀌면**(로그아웃, 다른 계정으로 로그인, `whoami` 가 저장된 것과 다른 주인을 알려 줌) 이 기기에 남은 이전 계정의 것을 한곳(`AppShared::forget_account_data`)에서 정리한다: 라이벌 목록, 설정 동기화의 잠금(`SyncLock`), 그 계정으로 보낸 채 돌아오지 않은 요청(`whoami`·라이벌·설정 업로드·읽기·다운로드, 그리고 도는 중인 LOGIN·REGISTER). 서버나 토큰을 다시 만들 때(`rebuild_server`)도 그 요청들은 버린다: 답이 옛 서버·옛 토큰의 것이기 때문이다. 버린 LOGIN·REGISTER 는 세션을 요청 전 상태(토큰이 있으면 로그인, 없으면 게스트)로 되돌리고(`AccountSession::abandon`, 안 그러면 LOGIN 이 재시작 전까지 막힌다), 버린 요청의 답은 그 요청만 기다리는 스레드로 넘겨, 답에 토큰이 있으면 그 서버에 폐기를 보낸다(`app_network::AuthInFlight` 의 `Drop`, 실패하면 `a login that was let go of left its token valid on the server (delete it on the web site)`). 답이 이미 채널에 와 있었든(답이 온 직후 다음 프레임 전에 LOGOUT·SERVER URL 확정이 겹친 경우, S19 3차 R3-ACC-1) 버린 뒤에 오든 같은 길로 폐기한다. 앱이 받아 간 답은 앱의 것이라 폐기하지 않는다. 전에는 LOGIN 직후 LOGOUT 을 누르면 늦게 온 답이 몰래 다시 로그인시키고 토큰을 저장했다(S19 2차 THR2-4). REGISTER 를 버리면 서버에는 계정이 만들어진 채 이 기기는 로그인하지 않는다. 같은 계정으로 다시 로그인하면 라이벌과 잠금은 그대로다. `whoami` 는 401 일 때만 토큰을 버린다(방화벽·프록시의 403 으로는 로그아웃되지 않는다).
- 설정 동기화 절차: `get_settings` → 편집 → `base_updated_at = 서버 updated_at` 으로 `put_settings`. 409 `SettingsConflict` 는 서버 사본을 함께 실어 오므로 재조회 없이 병합/덮어쓰기를 결정할 수 있다.
- **성공한 PUT 은 저장한 `updated_at` 을 200 본문으로 돌려준다.** 서버는 클라가 보낸 `updated_at` 을 무시하고 자기 시각을 찍으므로, 새 잠금 값은 이 응답으로만 알 수 있다. `put_settings` 는 이를 `SettingsPutResult { updated_at, from_server: true }` 로 돌려주고 클라가 곧바로 잠금을 갱신한다 — 조건부 PUT 앞의 추가 GET 은 필요 없다.
- **204 로 답하는 구버전 서버**는 `from_server: false` + 보낸 stamp 로 떨어진다. 이 stamp 는 추측이라 잠금에 넣지 않고(`ir_sync::upload_outcome` 이 `None`), 클라(`App::refresh_sync_base`)가 `get_settings` 를 한 번 더 돌려 진짜 값을 읽는다. 이 재조회가 없으면 그런 서버에서는 한 세션의 **두 번째 저장부터 항상 409** 다.
- **409 는 잠금을 전진시키지 않는다.** 409 본문의 `server.updated_at` 을 그대로 다음 base 로 삼으면 바로 다음 업로드가 성공해 "먼저 다운로드하라"던 그 새 서버 사본을 덮어쓴다. 잠금 전이는 `ir_sync::SyncLock`/`SyncOutcome` 한 곳에 모여 있다: `Read(updated_at)` 와 `Uploaded(updated_at)`(= 서버가 알려준 stamp) 만 잠금을 설정하고, `Conflict` 는 그대로 두며, `SignedOut` 은 지운다.
- **설정 blob 에 실리지 않는 것**(`ir_sync::keep_local`): 토큰·login id·이메일·PLAYER ID·서버 주소, 이 기기의 경로(곡 폴더·표·글꼴·시스템 효과음 폴더·스킨 메뉴. 이름에 folder·path·dir·location·file 이 든 필드가 새로 생기면 `ir_sync.rs` 의 가드 테스트 `a_field_of_the_configuration_that_names_a_place_on_disk_is_one_the_upload_blanks` 가 실패한다), 그리고 **추가 IR 서버 목록 `ir_profiles` 전체**(서버마다 그 서버가 발급한 토큰이 들어 있다). 업로드는 이것들을 기본값으로 바꿔 보내고, 다운로드는 이 기기의 것을 그대로 둔다. S19 이전에 올라간 blob 에 프로필 토큰이 있었다면 그 계정의 다음 업로드가 blob 을 덮어쓸 때 사라진다(서버 쪽 일괄 정리는 없다).
- **설정 blob 의 줄바꿈은 모든 OS 에서 LF 다**(`ir_sync.rs` 의 `BLOB_NEW_LINE`, `8b6f8bb`). RON 의 기본 줄바꿈이 Windows 에서 CRLF 라 같은 설정이 OS 마다 다른 글로 올라가던 것을 계약 교차 테스트(`contract/client-requests/` 의 설정 blob)가 Windows CI 에서 잡았다. 테스트 `an_uploaded_blob_breaks_its_lines_the_same_way_on_every_platform`.
- 미설정 기본값 `NullScoreServer` 는 8개 필수 메서드에 `NotConfigured`, 슈퍼셋 확장 메서드에 `Unsupported` 를 돌려준다(플레이 무영향, 연결 표시 빨강).

---

## 5. 알려진 갭 (서버 쪽 후속)

- **리플레이의 `extra` 는 저장되지 않는다.** `replay` 테이블에 해당 컬럼이 없어 업로드 본문의 `extra` 가 버려지고 다운로드는 항상 `{}` 다. 나머지 필드는 전부 왕복한다(`tests/service/replay.test.ts` 가 업로드 본문 전체와 다운로드를 deep-equal 로 비교). 채우려면 컬럼 추가 + 마이그레이션이 필요하다.
- **`gauge` 를 생략한 업로드는 `AssistEasy` 로 되돌아온다.** 컬럼이 `NOT NULL DEFAULT 0` 이라 "미지정"과 0번 게이지를 구분하지 못한다. 클라(`ir_replay::to_ir_replay`)는 항상 게이지를 보내므로 정상 경로에는 영향이 없다.
- 경로·쿼리 percent-encoding 은 `url` 크레이트 신규 의존이 필요해 보류 중이다(현재는 URL-safe 식별자 전제).
- 차트 메타 업서트(`POST /charts`), 난이도표(`/tables`), 코스 메타 업서트(`POST /courses/meta`), 스코어 단건 조회(`GET /scores/{id}`) 는 서버에 있으나 `rbms-ir` 클라이언트에는 아직 없다. 차트 제목·아티스트·레벨은 `POST /charts` 대신 점수 제출의 `chart.title/artist/level` 로 올린다.
- **계약 교차 테스트(S18, `e9aebae`)**: 클라이언트 → 서버 방향은 닫혔다. 플레이어 테스트(`apps/rbms-player/src/app_result/record_tests/contract_requests.rs`)가 루프백 서버로 실제로 보낸 요청 본문 19종(일반·SCRATCH AUTO·넓힌 판정·오토플레이·FAILED·Esc 포기·空POOR 가 POOR 수를 넘는 판·14키 2P·동기화 재전송·코스 둘·리플레이 둘·설정 blob·판별 trace 넷(일반·RANDOM·SCRATCH AUTO·Esc 포기))을 `contract/client-requests/*.json` 과 비교하고(`RBMS_UPDATE_CONTRACT=1` 이면 다시 쓴다), 웹 테스트 `web/tests/dto/client-requests.test.ts` 가 그 파일들을 서버 zod 스키마로 통과시킨다. trace 넷은 짝이 되는 점수 파일을 점수 서비스가 저장한 행과 일관성 검사·저장·분석까지 통과해야 한다. v0.1.2 의 점수 제출 400(2^53 를 넘는 시드)이 이 공백에서 나왔다. 남은 것은 반대 방향(서버 응답 → Rust 디코드)이다(`quality-assurance/2026-10-14-test-debt.md` 3.12). 양쪽 고유 테스트(`crates/rbms-ir/src/contract_tests.rs`·`fixtures.rs`, `web/tests/dto/ir-contract.test.ts`)는 그대로 있다.
