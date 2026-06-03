# rbms IR-superset API 계약 (백엔드 구현용)

> ⚠️ **이 문서는 `rbms-ir` 클라이언트가 현재 호출하는 최소 계약(요약)**이다. 백엔드 **전체 설계(PRD·전 엔드포인트·early/late 스코어·설정 동기화·µs 리플레이·빌드해시 무결성·FE·Drizzle 스키마·LR2IR/beatoraja 매핑)**는 **[`docs/backend/`](../backend/README.md)** 가 정본이다. 신규 구현은 그쪽을 따른다.
>
> 클라이언트(`rbms-ir`)는 완성돼 있다. 이 문서는 **나중에 만들 독자 서버**가 구현해야 할 HTTP 엔드포인트 계약이다.
> 설계 원칙: **BMS IR(LR2IR/Mocha/Cinnamon 등)의 슈퍼셋**. MD5+SHA-256 양쪽 차트 id, fast/slow 집계, 리플레이, 코스, capability 탐색, 그리고 모든 DTO의 `extra`(free-form) 필드로 전방호환.
> REST + JSON, 선택적 `Authorization: Bearer <token>`. 모든 요청에 `api_version`(현재 1).

## 엔드포인트

| 메서드 | 경로 | 요청 | 응답 | 용도 |
|---|---|---|---|---|
| GET | `/health` | — | `ServerInfo` | 연결 표시(초록/빨강)·capability 탐색 |
| POST | `/scores` | `ScoreSubmission` | `SubmitResponse` | 스코어 제출 |
| GET | `/charts/{md5}/ranking?limit=N` | — | `[ScoreRecord]` | 차트 랭킹 |
| GET | `/charts/{md5}/best?player={id}` | — | `ScoreRecord?`(null 허용) | 내 최고 기록 |
| GET | `/players/{id}` | — | `PlayerProfile` | 프로필 |
| GET | `/players/{id}/rivals` | — | `[PlayerProfile]` | 라이벌 |
| POST | `/courses` | `CourseSubmission` | `SubmitResponse` | 코스(단위) 결과 |
| POST | `/charts/{md5}/replays` | `ReplayData` | `{ "id": "..." }` | 리플레이 업로드 |

성공은 2xx + JSON. 실패는 4xx/5xx + 본문(텍스트) → 클라이언트가 `IrError::Server(code, body)`로 표면화. 네트워크 실패 → `IrError::Network`. 미설정(`--server` 없음) → `NotConfigured`(빨강).

## DTO (serde JSON; 필드명=아래 그대로)

```jsonc
// ServerInfo
{ "name": "my-ir", "version": "0.1", "ir_compat": "1",
  "capabilities": { "ranking": true, "player_best": true, "rivals": false,
                    "courses": false, "replays": false, "tables": false } }

// ChartId  — MD5는 IR 호환 키, SHA-256은 슈퍼셋
{ "md5": "32hex", "sha256": "64hex" }

// ScoreSubmission (POST /scores)
{ "api_version": 1,
  "chart": { "md5": "...", "sha256": "..." },
  "player": { "id": "guest" },
  "mode": "BEAT_7K",
  "clear": "Hard",                 // NoPlay|Failed|AssistEasy|LightAssistEasy|Easy|Normal|Hard|ExHard|FullCombo|Perfect|Max
  "ex_score": 1488, "max_ex_score": 1624,
  "judge": { "pgreat":712,"great":64,"good":21,"bad":8,"poor":5,"miss":2,
             "fast":30,"slow":40,"combobreak":15 },   // fast/slow/combobreak = 슈퍼셋
  "max_combo": 540, "total_notes": 812, "minbp": 15, "gauge_value": 86.0,
  "options": { "gauge":"Hard", "random":"Random", "random_p2": null,
               "scratch_auto": false, "lntype": 1, "input_device": "keyboard", "assist": [] },
  "played_at": 1700000000000,      // unix ms
  "client": "rbms/0.1.0", "replay_id": null,
  "extra": {} }                    // 자유 확장 (향후 IR 슈퍼셋 필드)

// SubmitResponse
{ "accepted": true, "rank": 3, "previous_best": 1400, "message": "saved" }

// ScoreRecord (랭킹/내기록)
{ "player": {"id":"p1"}, "player_name": "Alice", "clear": "ExHard",
  "ex_score": 1600, "max_combo": 812, "minbp": 1, "rank": 1,
  "played_at": 1700000000000, "extra": {} }

// PlayerProfile
{ "id":"p1", "name":"Alice", "total_plays": 1234, "rank_points": 56.7, "extra": {} }

// CourseSubmission (POST /courses)
{ "api_version":1, "course_hash":"...", "player":{"id":"guest"},
  "clear":"Normal", "ex_score":5000, "judge":{...}, "max_combo":3000,
  "gauge_value":42.0, "charts":[{"md5":"..","sha256":".."}, ...],
  "played_at":1700000000000, "extra":{} }

// ReplayData (POST /charts/{md5}/replays)
{ "format":"rbms-keylog-v1", "events":[/* bytes */], "seed": 12345 }
```

## enum 표기
- `ClearLamp`, `GaugeType`(AssistEasy|Easy|Normal|Hard|ExHard|Hazard|Class|ExClass|ExHardClass), `RandomOption`(Off|Mirror|Random|RRandom|SRandom|Spiral|HRandom|AllScratch|Converge) 는 **variant 이름 그대로** JSON 문자열.

## 슈퍼셋 확장 지점 (백엔드에서 추가 기능 만들 때)
- 모든 DTO의 `extra: { }` 에 임의 필드 추가 가능(클라이언트가 보존·무시).
- `capabilities` 로 서버가 지원하는 기능을 클라이언트에 광고 → 클라이언트가 UI 토글.
- `api_version` 으로 계약 버전 관리. 새 엔드포인트는 추가만(기존 불변).
- 클라이언트 구현체는 `rbms-ir`의 `ScoreServer` 트레이트. 새 메서드 추가 시 트레이트 확장.

## 현재 클라이언트 상태
- `HttpScoreServer`(reqwest blocking, rustls) — 위 엔드포인트 호출. 5초 타임아웃.
- 앱: `--server <url>`, `--player <id>`. 백그라운드 스레드가 5초마다 `/health` → **초록(연결)/빨강(미연결)** 표시. 결과 화면 진입 시 비차단 스레드로 `/scores` 제출.
- `NullScoreServer`(미설정 기본) — 항상 `NotConfigured`(빨강), 플레이 무영향.
