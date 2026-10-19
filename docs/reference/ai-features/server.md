# 웹 AI 기능 기획 조사 (AI1) - 웹 서버가 가진 데이터와 AI 기능을 넣을 자리

> 최종 갱신 2026-10-19 · 대응 커밋 1eadbb5
> 이 문서는 조사 보고서 전문이다(줄이지 않았다). 끝에 조사자가 함께 낸 "제약"과 "확인하지 못한 것" 목록을 원문 그대로 붙였다. 기획은 [../../plan/2026-10-19-web-ai-features.md](../../plan/2026-10-19-web-ai-features.md), 색인은 [README.md](README.md).

조사 시점 2026-10-19, 브랜치 dev, 코드는 수정하지 않았고 Git 상태도 바꾸지 않았다. `.env*`, `secrets/`, `settings.ron`, `scoredb.sqlite` 는 읽지 않았고 운영 사이트·DB 에 요청하지 않았으며 rbms-player 도 실행하지 않았다. 측정에 쓴 것은 `target/release/rbms-cli` 의 `scan` 과 chart 요약 모드뿐이다(읽기 전용, 임시 sqlite 는 시스템 temp 에만 생성). 경로는 `/Users/hyunseokbyun/development/R-BMS` 기준 상대 경로이고 `web/` 안 파일은 `web/` 접두사를 붙였다. 사용자 결정(BYOK, 3개 공급자, 판별(a)+곡별(b) 패턴, 결정론 집계·AI 는 해석만)은 `docs/PROCESS.md:21-22` 에 이미 기록돼 있다.

---

## 요약 (먼저 읽을 것)

1. **서버에는 노트별 판정도, 노트 배치도, 게이지 곡선도 없다.** 리플레이는 입력 이벤트(`t_us`, `lane`, `press`)뿐이고(`web/src/server/dto/replay.ts:25-29`), 서버 점수 행은 판정 합계·FAST/SLOW·옵션뿐이다. 서버 리플레이는 와이어에 판정 알고리즘·LN 종류·시계가 없어 재판정이 불가능하다고 클라이언트 문서가 못박았다(`docs/reference/record-target-pace.md:99`). 그래서 사용자가 정한 (a) "클라이언트가 판 끝에 실제 배치와 노트별 판정을 올린다"는 새 와이어 계약이 필수다.
2. **클라이언트는 그 데이터를 이미 가지고 있다.** `JudgeEngine::note_marks()` 가 노트별 `lane`, `time_us`, `state`(판정+1), `play_time_us`(부호 있는 타이밍)를 주고(`crates/rbms-judge/src/matcher.rs:129-145`), `PlayRecord.gauge_log` 가 9개 게이지를 500 ms 간격으로 기록한다(`crates/rbms-play/src/record.rs:21,421-444`). 로컬 리플레이 파일도 화면·소리 없이 다시 판정할 수 있다(`docs/reference/record-target-pace.md:42-46`, 단 FRAME 시계로 저장된 옛 리플레이는 근거로 쓰지 않음, 같은 문서 4.2절 3항).
3. **발광 표 1,035곡은 md5 만 있고 sha256 이 없다**(`web/data/tables/insane1/data.json`). 사용자 라이브러리 `/Volumes/SSD/bms` 에 1,035곡 전부가 있음을 측정으로 확인했다(아래 3절). 곡별(b) 패턴은 md5 키로 저장하고 서버 `chart.md5`(unique)로 잇는 것이 자연스럽다.
4. **Ollama Cloud 는 공식 문서상 구조화 출력(`format` JSON schema)을 지원하지 않는다**("Ollama's cloud currently does not support structured outputs", https://docs.ollama.com/capabilities/structured-outputs). Anthropic 과 OpenAI 는 지원한다. "공급자 3곳이 같은 구조화 출력 스키마"를 쓰려면 Ollama 어댑터는 프롬프트에 스키마를 넣고 파싱·Zod 검증·재시도하는 폴백이 필요하다.
5. **서버에는 바깥으로 나가는 fetch 선례도, AI SDK 도, 암호화 전용 시크릿도 없다.** 암호화에는 `better-auth/crypto` 의 `symmetricEncrypt`(버전 있는 봉투, 이미 설치됨)나 `node:crypto` AES-256-GCM 을 쓸 수 있고, 새 환경변수(마스터 키)가 필요하다.
6. **요청 한도**: Vercel Functions 본문 4.5 MB, 기본 최대 실행 300초, 이 프로젝트 플랜(Hobby/Pro)은 확인하지 못했다. 이 저장소에는 `maxDuration` 설정이 하나도 없다.

---

## 1. DB 스키마 (`web/src/server/db/schema.ts`, `web/src/server/db/auth-schema.ts`)

DB 는 MySQL + Drizzle(mysql2), 마이그레이션 0000~0004(`web/drizzle/`)가 운영에 적용돼 있다(`docs/web/architecture.md:265`). 컬럼은 DB 에서 snake_case, TS 에서 camelCase 다.

### 1.1 표 목록과 키·인덱스

| 표 | 위치 | 키 / 인덱스 | 비고 |
| --- | --- | --- | --- |
| `chart` | schema.ts:20-54 | PK `sha256`; uniq `chart_md5_uniq(md5)`; idx `chart_title_idx(title)` | `md5` 는 nullable. 제목·아티스트·레벨·모드·노트 수·BPM·`has_*` 플래그 |
| `score` | schema.ts:56-137 | PK `id`; uniq `score_idempotent_uniq(user_id, chart_sha256, played_at)`; idx `score_ranking_idx(chart_sha256, ranked, clear, ex_score)`, `score_player_idx(user_id, played_at)`, `score_build_idx(client_build_sha256)`, `score_recent_idx(played_at)` | 한 판 = 한 행. 사용자 시계열은 `score_player_idx` 로 읽힌다 |
| `chart_best` | schema.ts:139-159 | PK `(chart_sha256, user_id, scratch_auto)`; idx `chart_best_rank_idx(chart_sha256, scratch_auto, clear, ex_score)` | **`user_id` 가 앞에 오는 색인이 없다**(0002 마이그레이션 `web/drizzle/0002_scratch_auto_bests.sql:4,6-7`로 확인). `WHERE user_id = ?` 로 한 사용자의 전체 최고 기록을 읽으면(`countLampsByUser`, `listBestByMd5`) 색인을 못 쓴다. AI 기능이 이 읽기를 자주 하면 마이그레이션이 필요하다 |
| `replay` | schema.ts:161-192 | PK `id`; idx `replay_chart_idx`, `replay_score_idx`, `replay_user_idx`(0003) | 본문은 `data` mediumtext(DB 인라인), `storage_key` 는 쓰이지 않음 |
| `api_token` | schema.ts:194-208 | PK `id`; uniq `api_token_hash_uniq(token_hash)`; idx `api_token_user_idx` | SHA-256 해시만 저장 |
| `rival` | schema.ts:210-218 | PK `(user_id, rival_id)` | 라이벌 목록은 공개 조회(`GET /api/players/[id]/rivals` 무인증) |
| `setting_blob` | schema.ts:220-231 | PK `(user_id, name)` | 클라이언트 설정 동기화 blob(mediumtext) |
| `course`, `course_chart`, `course_score`, `course_best` | schema.ts:233-317 | `course_score_idempotent_uniq`, `course_score_rank_idx`, PK 들 | 코스. `course_score` 는 0004 에서 판정 합계가 추가됨 |
| `difficulty_table`, `table_folder`, `table_chart`, `table_course` | schema.ts:319-354 | idx `table_folder_table_idx` | **IR 용 DB 표**. 발광 표는 여기가 아니라 파일 스냅샷이다(3절) |
| `client_build` | schema.ts:356-365 | PK `sha256` | 빌드 allowlist(`trusted`, `channel`) |
| `submission_audit` | schema.ts:367-383 | idx `submission_audit_build_idx`, `submission_audit_created_idx` | 제출 감사 로그(IP, UA, accepted, flags) |
| `user_folder` | schema.ts:385-404 | PK `id`(`uf_` + 30 hex); uniq `user_folder_owner_name_uniq(user_id, name)`; idx `user_folder_owner_position_idx` | FK `user.id` cascade. `is_public` 은 v1 에서 항상 false |
| `user_folder_item` | schema.ts:406-425 | PK `(folder_id, md5)`; idx `(folder_id, position)`, `(md5)` | 곡 식별자는 md5, `sha256` 은 빈 문자열 가능 |
| `user`, `session`, `account`, `verification` | auth-schema.ts:3-64 | `user.email` uniq, `user.login_id` uniq | better-auth 소유. `user.role`('user'\|'admin'), `rank_points` 는 항상 0 |

### 1.2 분석에 쓸 수 있는 열 (`score` 중심, schema.ts:56-137)

| 분석 축 | 열 | 메모 |
| --- | --- | --- |
| 식별·시각 | `id`, `user_id`, `chart_sha256`, `chart_md5`(nullable), `mode`, `played_at`(unix ms), `created_at` | `played_at` 은 클라이언트 시각, 5분 넘는 미래는 거절 |
| 판정 수 | `pgreat`, `great`, `good`, `bad`, `poor`, `miss`, `empty_poor`, `combobreak` | 서버 검사는 노트를 소비한 판정만 센다(`judgedNoteCount`, HANDOFF 3.1) |
| 판정 early/late | `epg`, `lpg`, `egr`, `lgr`, `egd`, `lgd`, `ebd`, `lbd`, `epr`, `lpr`, `ems`, `lms` | 티어별 FAST/SLOW 분해 |
| 타이밍 | `fast`, `slow`, `avgjudge`(µs) | `avgjudge` 는 **규칙 세대 4 부터 원본 정의(노트별 거리 평균)**, 그 전 행은 부호 있는 평균 오프셋이다(`docs/reference/ir-api.md:108-124` 부근 extra 설명). 세대는 `score.extra.rule_version`(JSON)에 있다 |
| 점수 | `ex_score`, `max_ex_score`, `max_combo`, `notes`, `passnotes`, `minbp`, `clear`(램프 id), `gauge_value` | `notes` 는 제출자가 보낸 `total_notes` |
| 옵션 | `gauge`(id), `random`, `random_p2`, `option`(비트필드), `scratch_left`, `scratch_auto`, `assist`(개수), `constant`, `lntype`, `judge_rate`, `total_override`, `autoplay`, `auto_offset`, `input_device`, `judge_algorithm`, `rule`, `skin` | `assist` 는 태그 개수만 저장(태그 문자열은 안 남음) |
| 화면 설정 | `hispeed`(double), `green_number`(int, 반올림 저장, nullable), `lift`, `lane_cover`, `offset_ms` | `green_number` 는 제출 float 를 `Math.round` 해서 저장(score.service.ts:132,181). **`offset_ms` 의 뜻이 빌드마다 다르다**: 예전에는 판정을 옮겼고, 지금은 노트를 그리는 위치만 옮기며 판정은 옮기지 않는다(`crates/rbms-store/src/replay.rs` 의 `Replay::offset_ms` 문서). 판정 습관 진단(기능 4)은 `extra.mean_offset_us`(치우침)를 같이 봐야 한다 |
| 시드 | `seed` | **2^53 를 넘는 값은 0 으로 저장**된다(`web/src/server/dto/common.ts:124`, ir-api.md:117). 시드로 배치를 서버에서 재구성하는 길은 신뢰할 수 없다 |
| 클라이언트 | `client`, `client_build_sha256`, `client_platform`, `ranked`, `flags`(JSON), `verified`(항상 false), `replay_id` | `ranked=false` 판(어시스트·오토플레이 등)은 `chart_best` 에 안 들어간다 |
| 자유 JSON | `extra` | `rule_version`, `mean_offset_us`, `reconstructed`(재구성 본문) 등. 제출 본문 한도 256 KB 안에서 무검사로 저장(`dto/common.ts:114-117`, `score.service.ts:202`) |

`chart`(schema.ts:20-54)의 중요한 사실: 점수 제출이 만든 행은 `sha256`, `md5`, 제목, 아티스트, 레벨(**BMS `#PLAYLEVEL`**, 발광 레벨이 아님), 모드, `notes`, `lntype` 만 채우고 `total`, `minbpm`, `maxbpm`, `has_*` 는 0/false/null 로 둔다(`web/src/server/service/domain/chart/chart.service.ts:170-195`). `POST /api/charts`(메타 업서트)는 Rust 클라이언트가 호출하지 않는다(ir-api.md 5절). 즉 서버의 차트 메타는 빈약하고 BPM·밀도·구간 정보는 서버가 모른다. `chart.notes` 는 "첫 제출자가 정한 표시용 값"이라 상한으로 쓰지 않는다(architecture.md:326).

### 1.3 폴더 한도 (AI 연습 폴더용)

`FOLDER_LIMIT`(`web/src/server/dto/folder.ts:6-19`): 폴더 50, 폴더당 2,000곡, 사용자 전체 20,000곡, 요청당 500곡, 이름 64자, 설명 255자, 메모 100자. 이름은 사용자 안에서 unique(`user_folder_owner_name_uniq`)라 AI 폴더 이름이 겹치면 `name_taken` 이 난다.

---

## 2. 리플레이: 형식, 저장 위치, 한도, 클라이언트 wire 와의 대조

### 2.1 서버가 받는 본문

- 스키마 `replayUploadSchema`(`web/src/server/dto/replay.ts:31-52`): `format`, `chart{md5,sha256}`, `score_id`, `mode`, `random`, `random_p2`, `seed`, `lntype`, `offset_ms`, `judge_rate`, `scratch_auto`, `constant`, `gauge`, `client_build_sha256`, `events[]`, `event_count`, `duration_us`, `size`, `extra`. 이벤트는 `{ t_us, lane, press }` 셋뿐이다(replay.ts:25-29). 노트별 판정·배치·게이지 값·판정 알고리즘·TOTAL·LN MARGIN 은 없다.
- `extra` 는 받지만 **저장하지 않는다**(다운로드는 항상 `{}`, `replay.service.ts:104`, ir-api.md:282).
- 라우트 `POST /api/charts/[hash]/replays`(`web/src/server/route/replay.ts:31-49`): Bearer 필수, 10/분/사용자, 본문 크기 가드(`REPLAY_MAX_BYTES`, 기본 4 MB, `web/src/server/lib/env.ts:4,17`), 같은 `score_id` 에 이미 연결된 리플레이가 있으면 새로 만들지 않고 기존 id 를 돌려줌(replay.service.ts:125-126).

### 2.2 저장 위치와 한도

| 항목 | 값 | 근거 |
| --- | --- | --- |
| 저장 위치 | **MySQL `replay.data`(mediumtext)에 `JSON.stringify(events)` 인라인**. 외부 저장소 없음 | `replay.service.ts:128,154-155`, `web/src/server/service/shared/storage/replay-storage.ts:12-16`, `compose/replay.ts:44-48` |
| `REPLAY_STORAGE=blob` | 구현 없음. 설정하면 compose 가 즉시 던짐 | compose/replay.ts:44-48, architecture.md:434 |
| 1건 상한 | `REPLAY_MAX_BYTES` 기본 4 MB (`content-length` + 스트림 바이트 둘 다 검사) | env.ts:17, parse-request.ts `guardBodySize`·`readLimitedText` |
| 사용자 총량 | **2,000건 / 512 MiB** | replay.ts:13 `REPLAY_USER_QUOTA`, `findReplayQuotaExceeded`(replay.service.ts:113-120), `sumUserReplays`(compose/replay.ts:96-102) |
| 이벤트 크기 | 압축 없는 JSON 으로 이벤트 1개 약 40 바이트(fixture `contract/client-requests/replay-long.json`: 1,596이벤트, 64,489바이트) | 직접 계산 |
| 읽기 | 공개, 무인증. `getReplayCached` 는 `cacheLife('max')` + 태그 `replay:{id}` | `web/src/server/service/read-cache.ts:132-136` |

즉 리플레이 본문은 이미 **공개**다(`GET /api/replays/[replayId]`, `GET /api/charts/[hash]/replays`). 새로 올릴 노트별 판정·배치를 같은 수준으로 공개할지는 별도 결정이 필요하다(unknowns).

### 2.3 클라이언트 wire 와의 대조

- 클라이언트가 올리는 것: `to_ir_replay`(`apps/rbms-player/src/ir_replay.rs`)가 로컬 `Replay`(`crates/rbms-store/src/replay.rs`)에서 `events`(입력 시각은 판정 offset 단계별로 보정), `seed`, `random`, `gauge`, `judge_rate`(3단계 중 첫 값 하나), `offset_ms`, `scratch_auto` 를 만든다. **LN MARGIN, TOTAL, 판정 알고리즘, 시계, 스크래치 방향은 와이어에 없다.**
- `wire_carries_judge`(`ir_replay.rs:98-103`): 6개 판정 폭이 한 숫자이고 LN MARGIN 이 기본이고 TOTAL 을 덮어쓰지 않은 판만 와이어로 온전히 표현된다. 아니면 업로드를 건너뛴다(`record_sync.rs:680-690`, `app_ranking.rs:165-168`, `app_result.rs:395`). 그런 판은 로컬 파일로만 남는다.
- 업로드 조건: 서버가 점수를 받았고 랭킹을 막는 flag 가 없을 때만(`is_replay_upload_warranted`, `crates/rbms-ir/src/worker.rs:71-76`). 따라서 **서버에 리플레이가 있는 판은 ranked 이고 판정 폭이 기본인 판뿐**이다. 어시스트·오토플레이·넓힌 판정 판은 리플레이가 없다.
- 로컬 `Replay` 구조(`crates/rbms-store/src/replay.rs`)에는 `ReplayJudge`(알고리즘, 6개 폭, LN MARGIN, `ln_mode`, 게이지 표, auto-shift, 시계, `total_override`, `offset_steps`)가 있어 클라이언트가 같은 판정을 재현할 수 있다. 서버 와이어로 오면 이 정보가 사라진다.
- 다운로드한 서버 리플레이를 재생하면 스크래치 역방향이 정방향으로 재현된다(`from_ir_replay` 문서, ir_replay.rs).
- 계약 fixture: `contract/client-requests/*.json` 15종(점수 12, 코스 2, 리플레이 2, 설정 blob 1). 새 업로드 본문은 같은 방식(`RBMS_UPDATE_CONTRACT=1`)으로 교차 테스트에 넣는 것이 기존 규칙이다(ir-api.md:286).

### 2.4 클라이언트가 판 끝에 가진 것 (새 업로드 후보)

| 데이터 | 출처 | 근거 |
| --- | --- | --- |
| 노트별 판정(레인, 차트 시각, 상태 1~5=PG..POOR, 부호 있는 타이밍 µs, LN 끝·CN 여부) | `JudgeEngine::note_marks()` | `crates/rbms-judge/src/matcher.rs:129-145` |
| 판정 발생 순서(레인, 판정, delta, 콤보) | `JudgedNote` | matcher.rs:163-175 |
| 9개 게이지의 500 ms 간격 곡선, 게이지 경계 | `PlayRecord.gauge_log`, `gauge_bounds` | `crates/rbms-play/src/record.rs:21,126-136,421-444` |
| 초당 판정 분포, 타이밍 분포 | `PlayRecord.seconds`, `timing` | record.rs:421-444 |
| 옵션 적용 뒤 배치 | `rbms_chart::shuffle::apply`(MIRROR/RANDOM/R-RANDOM/ROTATE 는 레인 순열, S-RANDOM/H-RANDOM/ALL-SCRATCH 는 행마다 다른 RNG) | `crates/rbms-chart/src/shuffle.rs:115-148`, 배치 공유 함수 `lay_out_notes` (record-target-pace.md:47) |
| 옛 판의 재생성 | 로컬 리플레이 파일을 `PlaySession::play_out` 으로 재판정(EVENT 시계 판만 신뢰) | record-target-pace.md:44-46,63-64 |

로컬 점수 DB(`score`·`scorelog`)에는 노트별 판정 열이 없다(record-target-pace.md:43). 서버 업로드는 판이 끝나는 시점이나 리플레이 재판정으로 만들어야 하고, 기록 동기화(`apps/rbms-player/src/record_sync.rs`, 점수 1.5초·리플레이 7.5초 간격, HANDOFF 7절)가 재전송 경로이므로 새 업로드도 그 경로에 얹혀야 한다.

크기 추정(측정 아님, 위 3.3의 노트 수 기준): 노트당 `[레인, 상태, 타이밍]` JSON 약 12바이트라면 2,100노트 판 약 25 KB, 5,000노트 판 약 60 KB.

---

## 3. 차트 메타, 발광 표 스냅샷, 표 서비스

### 3.1 서버가 차트에 대해 아는 것

- `chart` 행(1.2절): md5, sha256, 제목류, 레벨(PLAYLEVEL), 모드, `notes`(표시용), `lntype`. BPM·TOTAL·`has_*` 는 제출 경로로는 채워지지 않는다.
- 해시 해석: `classifyChartHash`가 md5/sha256 을 구분하고 `findByHash`가 둘 중 하나로 찾는다(chart.service.ts). md5 는 nullable 이라 md5 가 비는 행이 있을 수 있다(운영 비율은 확인 못 함).
- 제목·아티스트가 비어 있으면 읽을 때 발광 표 스냅샷의 제목으로 채운다(`findSnapshotTextByMd5`, `fillBlankText`, architecture.md:403).
- `web/scripts/chart-notes.*.sql`(count/repair/restore/drop-backup)은 `chart.notes` 가 LN 모드·SCRATCH AUTO 판 때문에 어긋난 행을 점검·복구하는 SQL 이다. 2026-10-15 운영 점검: 어긋남 0, 복구 대상 0, 점수 없는 차트 행 3, 누적 거절 39(architecture.md:270). 복구 기준은 "일반 판 LN 모드 `total_notes` 를 가장 많은 사용자가 보낸 값"이다. 이 스크립트들은 `web-db.yml` 로 돌고 `web/scripts/run-sql.ts` 는 `web/scripts/` 아래 `.sql` 만 실행한다.

### 3.2 발광 표 스냅샷 (`web/data/tables/insane1/`)

- 파일: `header.json`(390 B), `data.json`(340,700 B), `source.json`(2,710 B). 저장소에 커밋된 **공개** 파일이다(원격 `B-HS/R-BMS` 공개, HANDOFF 7절).
- `header.json`: `name`, `symbol`, `data_url`, `level_order`("1".."25","???" 26개), `last_update`(2017.02.05).
- `data.json`: 길이 1,035 배열. 키는 `artist, comment, level, lr2_bmsid, md5, name_diff, title, url, url_diff`. **`md5` 1,035개 전부 있음, `sha256` 0개**(직접 집계). 레벨은 문자열이고 분포는 10→64곡, 7→60, 5→59, 12→58, ..., 24→11, "???"→5, 25→2 등.
- 로딩: 정적 import 후 Zod 파싱(`web/src/server/service/domain/table-snapshot/table-snapshot.data.ts:1-36`, 스키마 `dto/table-snapshot.ts:17-27`). md5 → 제목 인덱스(`findSnapshotTextByMd5`, data.ts:40-52).
- 표 서비스(`table-snapshot.service.ts`): `list`, `getDetail`(레벨별 묶음), `searchEntries`, `loadEntriesForImport`(폴더 가져오기용, `sha256: ''`), `getRecords`(185-190: 표의 md5 전부를 `IN` 한 번으로 묻고 `{clear, ex_score, minbp, max_combo, total_notes, played_at, scratch_auto}` 를 md5 키로 반환). 구현 쿼리는 `compose/table-snapshot.ts:12-30`(chart_best ⨝ chart ⨝ score, `chartBestScope(db, true)` = 일반/SCRATCH AUTO 중 더 높은 쪽).
- **발광 표 램프 분포(기능 5)에 필요한 입력은 이미 한 호출로 나온다**: `getRecords` + 표의 레벨 라벨. 최대 응답 약 150 KB(architecture.md:148). 단 한 md5 에 sha256 이 여럿이면 한 건만 담긴다(foundation-contract 6.2절).

### 3.3 곡별 패턴(b) 현실 측정 (직접 측정)

- `target/release/rbms-cli scan /Volumes/SSD/bms` → 11,336차트 전부 읽음, 실시간 4.96초. 모드별: BEAT_7K 7,903, BEAT_14K 2,077, POPN_9K 893, BEAT_5K 463.
- 이 중 **발광 표 1,035 md5 가 1,035개 모두 라이브러리에 있다**(scan 출력의 md5 와 `data.json` 의 md5 교집합 1,035).
- 1,035개 파일을 md5 로 찾아 `rbms-cli <파일>` 요약으로 노트 수를 쟀다: 전부 BEAT_7K, 노트 수 최소 946 / 중앙값 2,114 / 90분위 2,878 / 99분위 3,776 / 최대 5,000 / 합계 2,271,796, 길이 중앙값 130.7초 / 최대 2,072.3초. (바이너리는 `target/release` 의 기존 빌드이며 현재 소스와 같은지는 확인하지 않았다.)
- 저장 크기 추정(측정 아님): 노트당 시각·레인·종류를 8~12바이트로 잡으면 총 약 18~27 MB. 곡당 약 17~25 KB, 최대 곡 약 60 KB.
- 14K·9K·5K 차트는 발광 표에 없으므로 (b)로 못 덮는다. 표 밖 곡은 (a) 판별 업로드로만 분석된다.
- `rbms-cli` 의 현재 하위 명령은 `<chart>` 요약, `scan`, `config`, `scores`, `skin-dump` 뿐이다(`apps/rbms-cli/src/main.rs:17-24,45-65`). 요약 모드는 이미 `rbms_chart::to_model` 로 파싱해 md5·sha256·노트 수를 낸다(main.rs:260-308). 모델은 `Model.timelines[].notes: Vec<Option<Note>>`(레인별 슬롯), `Note.kind`(Normal/LongStart/LongEnd/Mine), `time_us`, `section` 을 갖고(`crates/rbms-model/src/lib.rs:77-113`), 패턴 추출 하위 명령은 `serde_json`(이미 의존) 만으로 만들 수 있다. HTTP 는 워크스페이스에 `reqwest`(blocking, rustls)가 `rbms-ir` 를 통해 있다(`Cargo.toml:51`, `crates/rbms-ir/Cargo.toml`). 압축 크레이트는 워크스페이스 의존성 목록에서 확인하지 못했다.
- 저장소는 공개라 **패턴 데이터를 `web/data/` 나 git 에 넣으면 안 된다**(비공개 조건 위반). 올리는 길은 서버 API(관리자)여야 한다. 그런데 기존 `withAdmin` 은 **세션만** 받고 API 토큰을 받지 않는다(`web/src/server/lib/with-session.ts:18-26`, architecture.md:158, 릴리스 CI 가 빌드 등록을 못 하는 같은 이유). 로컬 CLI 업로드를 허용하려면 관리자 Bearer 경로가 새로 필요하다. 본문 4.5 MB 한도 때문에 곡 단위 요청(약 1,035건)이 된다.
- 배치 정합: (a)에서 올리는 최종 배치와 (b)의 원본 배치는 같은 노트 목록의 레인 순열이어야 한다. MIRROR/RANDOM/R-RANDOM/ROTATE 는 한 번의 레인 순열(`lane_permutation`, shuffle.rs:115)이라 서버에서 순열 검증이 가능하다. S-RANDOM/H-RANDOM/ALL-SCRATCH 는 행마다 달라 순열 검증이 안 된다. 시드로 재구성하려면 클라이언트 RNG(`Rng::new`, 사용자 지정 구현)를 TS 로 이식해야 하고 서버의 `seed` 는 0 으로 저장될 수 있어 신뢰할 수 없다.

---

## 4. 기능 연결점: 폴더, 기록 조회, 라이벌, 순위

### 4.1 내 폴더 (기능 6: AI 연습 폴더 -> 게임 MY FOLDERS)

계약 정본은 `docs/plan/2026-10-14-web-user-folders.md`(7.2절 읽기 API, 13절 클라이언트 결과, 14절 서버 결과), 구현은 `web/src/server/{dto/folder.ts, route/folder.ts, service/domain/folder/folder.service.ts, compose/folder.ts}`.

- 웹 편집 API(세션 전용, 봉투 응답): `GET/POST /api/fe/me/folders`, `PUT .../order`, `GET/PATCH/DELETE .../[folderId]`, `POST/PUT .../[folderId]/items`(PUT 은 통째 교체 + `base_updated_at` 충돌 검사), `.../items/remove`, `.../items/order`, `PATCH .../items/[md5]`, `POST .../import`(표에서 가져오기), `GET .../containing/[md5]`(architecture.md:134-142).
- 클라이언트 읽기(봉투 없음, Bearer 또는 세션, 본인만, 캐시 안 함): `GET /api/players/[playerId]/folders`, `.../folders/[folderId]`. 시작·로그인·F6·Tables 화면에서 클라이언트가 받아 저장본(`user-folders/<해시>.ron`)과 비교해 반영한다(plan 7.6, 13.2). **AI 가 만든 폴더는 서버 폴더 표에 정상 저장되면 클라이언트가 다음 새로 받기에서 자동으로 MY FOLDERS 에 보인다.** 클라이언트는 가진 곡만 줄로 보이고 `(가진 곡/전체)` 로 표시한다(plan 7.4). 곡 식별자는 md5(16진 32자 소문자) 라서 발광 표 md5 와 그대로 호환된다.
- 서비스 재사용: `createFolderService` 의 `create`, `update`, 곡 교체·추가 메서드(folder.service.ts:214 이하)는 HTTP 를 모른다. `compose()` 가 `folderService` 를 노출한다(`compose/index.ts:366`). AI 라우트는 이 서비스를 직접 부르면 된다. 실패는 `FolderResult`(`not_found`, `name_taken`, `limit_exceeded`, `set_mismatch`, `conflict`, `table_not_found`, folder.service.ts:112-123).
- 한도(1.3절): 같은 이름 중복, 50폴더, 2,000곡, 요청당 500곡. 레이트리밋: 편집 60/분, 읽기 120/분, 가져오기 10/분(`web/src/server/lib/with-rate-limit.ts:9-11`).
- 공개 폴더는 v1 에서 쓰지 않는다(`is_public` 항상 false).
- 클라이언트 능력 플래그: `SERVER_CAPABILITIES`(`web/src/server/service/domain/system/capabilities.ts:11-21`)에 `folders: true`. AI 연동용 새 능력 필드를 더하면 클라이언트 `crates/rbms-ir/src/dto/system.rs` 쪽은 없으면 false 로 읽힌다(plan 7.2).

### 4.2 플레이어 기록 조회 경로

| 용도 | 경로 / 함수 | 근거 |
| --- | --- | --- |
| 플레이어의 모든 판(페이지) | `GET /api/fe/players/[playerId]/recent`(`page`,`limit<=100`, 공개, 태그 `player-recent:{id}`) | `compose/fe.ts:143-156`, `dto/fe.ts` `playerRecentQuerySchema` |
| 플레이어 점수 + 판정 분해 | `GET /api/players/[playerId]/scores`(`since`,`mode`,`limit<=100`,`page`, 공개) | `compose/fe.ts:221-287` (`listPlayerScoreRows`) |
| 램프 분포·클리어 수 | `GET /api/fe/players/[playerId]/stats` (`playerStats`: `lamps`, `cleared`, `scratch_auto_charts`) | `fe.service.ts:98-107`, `compose/fe.ts:183-190` |
| 곡별 최고 기록 | `chart_best` + `score` 조인(`compose/index.ts:245-261`, `table-snapshot` 쿼리) | |
| 차트 순위 | `GET /api/charts/[hash]/ranking`, `.../best?player=` | `compose/index.ts:226-261` |
| 점수 단건 | `GET /api/scores/[scoreId]` | |

모든 읽기가 공개라 **다른 사용자의 기록은 이미 누구나 읽을 수 있다**. 기능 3(베스트·라이벌 비교)의 라이벌 쪽 노트별 데이터를 새로 올린다면 그 공개 범위가 새 결정이다. 서버 입력 검증은 본문 일관성 6개(EX, 판정 합계, max_ex_score, minbp, 램프, INT 범위)뿐이고 리플레이 재판정(`REPLAY_MISMATCH`)은 코드에 없다(architecture.md:326,498). 새 노트별 업로드는 점수 행의 합계(판정 6종, early/late, fast/slow)와 일치하는지 서버가 검사할 수 있다.

### 4.3 라이벌·순위

- 라이벌: `rival(user_id, rival_id)`, `rival.service.ts`(`listByLoginId`, `replaceByUserId`), `GET/PUT /api/players/[id]/rivals`(PUT 은 세션·본인). 순위 목록에 `rival_of` 파라미터가 있다(`dto/score.ts:99-105`).
- 순위: `chartBestScope`(`compose/best-scope.ts:66-73`)가 "일반 판만"/"일반·SCRATCH AUTO 중 더 높은 쪽"을 고른다. 종합 랭킹 `GET /api/fe/leaderboards/players` 는 `cleared`/`played`/`plays` 정렬(클리어 = 램프 id >= ASSIST EASY, `MIN_CLEAR_LAMP_ID`, `dto/common.ts:33-34`). 비교 순서는 램프 > EX > BP 작을수록(best-scope.ts `outranks`).

---

## 5. 보안·운영

### 5.1 인증·역할

| 항목 | 내용 | 근거 |
| --- | --- | --- |
| 웹 세션 | better-auth(이메일+비밀번호, 세션 30일, 갱신 1일), 쿠키 HttpOnly·SameSite=Lax | `web/src/server/lib/auth.ts:9-34`, architecture.md:281 |
| API 토큰 | `rbms_` + 32바이트 base64url, SHA-256 해시 저장 | `web/src/server/lib/token.ts:3-8`, `api_token` 표 |
| HOF | `withSession`, `withAdmin`(세션만)(`with-session.ts`), `withApiToken`/`withBearerOrSession` | |
| 역할 | `user.role` = 'user' \| 'admin', 관리자 라우트는 `/api/admin/*`, 페이지 `/admin/builds` | auth-schema.ts:21, `route/admin.ts` |
| 보호 경로 | `proxy.ts` matcher: `/settings`, `/admin`, `/folders` + 상세 존재 확인(`charts`, `players`, `courses`, `replays`). 세그먼트 `SessionGate` 가 서버에서 세션을 다시 확인 | `web/src/proxy.ts:37-47` |
| `/api/auth/[...all]` | `GET /get-session` 과 `POST /sign-out` 만 통과 | architecture.md:328 |
| CSRF/Origin 검사 | **명시 구현 없음**(grep 으로 route/lib 에 Origin 검사 없음 확인). SameSite=Lax 에 의존 | |

### 5.2 Rate limit

- 구현 `web/src/server/lib/with-rate-limit.ts`: 인스턴스 로컬 `Map` 고정 창 카운터(버킷 5,000개 초과 시 만료분 정리, L16-26). Vercel 다중 인스턴스에서는 best-effort 이고 콜드스타트마다 초기화(architecture.md:318, "지속형 저장소 필요" KNOWN ISSUE, HANDOFF 6절 10).
- 상수 `RATE_LIMIT`(with-rate-limit.ts:4-14): AUTH 5/분, SCORE_SUBMIT 60, REPLAY_UPLOAD 10, PUBLIC_READ 120, FOLDER_WRITE 60, FOLDER_IMPORT 10, FOLDER_READ 120, TABLE_RECORDS 30, SETTING_WRITE 30. IP 키는 `X-Forwarded-For` 첫 값(L42-46; Vercel 이 이 헤더를 덮어쓴다는 근거 foundation-contract 9절).
- AI 호출은 "비싼 외부 호출"이므로 인스턴스 로컬 카운터로는 사용자별 일 한도를 지킬 수 없다. DB 기반 카운터가 필요하다.

### 5.3 환경변수와 getEnv

- `web/src/server/lib/env.ts:6-25` Zod 스키마, `getEnv()`(L67-71)는 캐시된 `parseEnv(process.env)`. 빈 문자열은 미설정 취급(L29-30). 현재 키: `DATABASE_URL`/`DB_*`, `BETTER_AUTH_SECRET`(optional), `BETTER_AUTH_URL`, `NEXT_PUBLIC_APP_URL`, `ALLOW_GUEST`, `REPLAY_MAX_BYTES`, `REQUIRE_BUILD_HASH`, `REPLAY_STORAGE`, `BLOB_READ_WRITE_TOKEN`, `SERVER_VERSION`, `SERVER_COMMIT`, `VERCEL_GIT_COMMIT_SHA`, `NODE_ENV`.
- 운영 필수 검사는 인증을 처음 쓸 때(`assertProductionAuthEnv`, L41-56)로 늦춰져 있다: 환경변수 없는 `next build` 가 성공해야 하기 때문(architecture.md:431-433, `tests/env.test.ts`). **새 마스터 키 변수도 같은 패턴(스키마는 optional, 사용 시점에 검사)을 따라야 환경 변수 없는 빌드가 깨지지 않는다.**
- `.env.example` 은 존재하지만(web 루트) 내용은 `.env.*` 규칙에 따라 읽지 않았다.
- 시크릿 규칙: `process.env` 직접 접근은 env.ts 안으로 한정, `NEXT_PUBLIC_*` 에 시크릿 금지(architecture.md:430).

### 5.4 암호화에 쓸 수 있는 것

| 선택지 | 확인한 사실 | 한계 |
| --- | --- | --- |
| `node:crypto` | 이미 `token.ts:1` 이 `createHash`, `randomBytes`, `randomUUID` 를 import. AES-256-GCM(`createCipheriv`)·HKDF 는 Node 표준 | 새 의존성 없음. AAD 로 `userId:provider:keyVersion` 을 묶을 수 있다 |
| `better-auth/crypto` | 설치됨(better-auth 1.7.3). `symmetricEncrypt({key, data})`/`symmetricDecrypt` 가 XChaCha20-Poly1305(`@noble/ciphers` 2.4.0)로 hex 암호문을 만들고, `SecretConfig {keys: Map<버전,비밀>, currentVersion, legacySecret?}` 를 넘기면 `$ba$<버전>$...` 봉투로 **키 로테이션**을 지원 (`node_modules/better-auth/dist/crypto/index.mjs`, `node_modules/@better-auth/core/dist/types/secret.d.mts`) | 키 파생은 비밀 문자열의 SHA-256 하나뿐(솔트·HKDF 없음, 고엔트로피 비밀이어야 안전). **AAD 인자가 없어** 같은 DB 쓰기 권한자가 암호문을 다른 사용자 행으로 옮기는 것을 막지 못한다. `./crypto` 서브패스는 package.json exports 에 있으나 공식 문서상 공개 API 인지는 확인하지 못했다 |
| `BETTER_AUTH_SECRET` 재사용 | 기존 유일한 서버 시크릿 | 로그인 세션 서명 키와 같은 값을 쓰면 한쪽 로테이션이 다른 쪽을 깨뜨림. 별도 키 권장 |

### 5.5 마이그레이션 운영 규칙 (HANDOFF 5절 운영, architecture.md 3-2)

- `drizzle-kit push` 금지. `bun run db:generate` 로 만든 `drizzle/*.sql` 과 **`drizzle/meta/` 를 커밋**한다. 저널 `when` 은 단조 증가해야 한다(L267).
- 운영 적용은 `gh workflow run web-db.yml --ref dev -f action=status|migrate|...` (`.github/workflows/web-db.yml`, 저장소 시크릿 `WEB_DATABASE_URL`). 사용자가 "migrate 도 너가 처리"로 바꿨고 추가만 하는 마이그레이션은 진행해 왔다(HANDOFF 5절).
- **dev 푸시가 곧 Vercel 프로덕션 배포**라서(Production Branch = dev) 새 코드가 새 표를 읽으면 migrate 전에 500 이 난다. 순서: (1) 마이그레이션 파일만 담은 커밋을 먼저 푸시 -> (2) `status` -> (3) `migrate` -> (4) `status` -> (5) 코드 커밋 푸시. 0003·0004 가 이 순서로 적용됐다(architecture.md:266,271).
- 새 표를 추가하려면 `web/src/server/db/schema.ts` 에 정의하고 `db:generate` 를 쓴다. 대용량 INSERT(패턴)는 SQL 파일이 아니라 API 로 한다(3.3절).
- 로컬 DB 통합 테스트: `RBMS_DB_E2E=1 bun test tests/db`(Docker `mysql:8.4.11`), 웹 게이트 `bun run verify && bun run format:check && bun run build`(HANDOFF 7절).

### 5.6 Vercel 함수 제한과 설정

- `web/vercel.json` 전체: `regions: ["icn1"]`, `ignoreCommand`(L4-5). **`functions`/`maxDuration` 설정 없음.** `grep -rn "maxDuration\|export const runtime\|preferredRegion" web/src` 결과 0건.
- Next 16.3.4 번들 문서: `maxDuration` 은 route segment config 로 Route Handler 에서 `export const maxDuration = N` 으로 지정하며 값은 플랫폼이 쓴다(`web/node_modules/next/dist/docs/01-app/03-api-reference/03-file-conventions/02-route-segment-config/maxDuration.md`). Cache Components(`cacheComponents: true`, next.config.ts:26)에서는 `dynamic`/`dynamicParams`/`revalidate`/`fetchCache` 가 제거됐고 `maxDuration`/`runtime` 은 남아 있다(같은 폴더 `index.md` Version History v16.0.0). `after()` 는 응답 뒤 작업을 돌리고 라우트의 최대 실행 시간 안에서 돈다(`04-functions/after.md`).
- Vercel 공식(https://vercel.com/docs/functions/limitations, 2026-08-24 판, Fluid compute 기준): 최대 실행 시간 기본 **300초**, Hobby 최대 300초, Pro·Enterprise 최대 800초(베타 확장 1,800초는 함수별 설정); 요청·응답 본문 최대 **4.5 MB**(초과 시 413 `FUNCTION_PAYLOAD_TOO_LARGE`); 메모리 Hobby 2 GB, Pro 4 GB; 스트리밍 응답 시간도 최대 실행 시간에 포함; 파일 디스크립터 1,024 공유. 활성 CPU 시간 과금에서 "AI 모델 호출 같은 I/O 대기는 활성 CPU 에 포함되지 않는다". 설정법: https://vercel.com/docs/functions/configuring-functions/duration (`export const maxDuration`). 무제한 실행이 필요하면 Vercel Workflows(https://vercel.com/docs/workflows)를 권한다.
- 응답 4.5 MB 한도는 이미 폴더 응답 설계에 반영돼 있다(architecture.md:443). 리포트 응답은 이 한도보다 훨씬 작아야 한다.
- **CSP**: 프로덕션에서만 붙고 `connect-src 'self'`(next.config.ts:3-14,21). 브라우저가 공급자 API 를 직접 부르면 CSP 에 막힌다. 서버 경유가 맞다. 서버 쪽 외부 fetch 선례는 없다(`grep fetch( web/src/server` 0건).
- DB 풀: `connectionLimit: 5`, 락 대기 5초(`web/src/server/db/index.ts:8,14,19`). 긴 AI 호출 동안 DB 연결을 잡고 있으면 안 된다(DB 읽기 -> 연결 반환 -> 공급자 호출 -> 결과 저장 순서).

### 5.7 캐시

- Cache Components 의 `'use cache'` + `cacheLife` + `cacheTag`(`web/src/server/service/read-cache.ts`)는 **쿠키/헤더를 읽지 않는 순수 읽기**에만 쓴다(architecture.md:349). 개인화 데이터(세션·키)는 인자로 넘기거나 아예 캐시하지 않는다. 암호화된 키·AI 결과는 `'use cache'` 에 넣지 않는다. 결과 캐시는 DB 표에 저장하고, 새 점수 제출 때 무효화하는 hook 은 `revalidateOnScoreSubmit`(read-cache.ts:72-85)이 있는 자리다.
- 요청 시점 라우트는 `compose()` 보다 `connection()`/`headers()` 를 먼저 읽어야 환경변수 없는 빌드 로그가 깨끗하다(architecture.md:431, foundation-contract 9절).

### 5.8 FSD 구조와 AI 기능을 넣을 자리

현재 구조(architecture.md 2절, 실제 디렉터리 확인):

- `web/src/app/{(app),(public),api,tables}` 가 app 레이어, `widgets/{admin,auth,chart,course,folder,guide,home,leaderboard,player,replay,search,settings,shell,table}`, `features/{badge,dashboard,dialog,filter,folder,lamp,score,shell,state,stats,table}`, `entities/*.query.ts|*.type.ts`, `shared/{constants,hooks,lib,providers,ui,utils}`, 서버는 `web/src/server/{route,service/domain,dto,db,lib,compose}`.
- 서버 계층: `app/api/**/route.ts`(3~5줄 어댑터) -> `server/route/<domain>.ts`(DTO 검증, HOF, 에러) -> `service/domain/<d>`(HTTP·Drizzle 모름) -> `*ServiceDb`(compose 구현, 트랜잭션은 여기서만). 모든 핸들러는 `withErrorHandling` 바깥 + `withSession` 안쪽(`route/folder.ts:67-96` 패턴).
- 에러 코드는 세 파일(`lib/error-code.ts`, `error-message.ts`, `error.ts`)에 같이 추가(`IR_*` 접두사 + 공통 코드).
- 제안 배치(이 조사가 정한 것이 아니라 기존 패턴의 적용): `dto/ai.ts`, `route/ai.ts`, `service/domain/ai/{credential,provider-adapter,report,...}.service.ts`, `compose/ai.ts`, 표 `ai_credential`·`chart_pattern`·`play_detail` 등을 `db/schema.ts` 에. 클라이언트는 `entities/ai.query.ts`·`ai.type.ts`, `widgets/ai/*`(훅·mutation), `features/ai/*`(순수 UI), `QUERY_KEY.AI`(`shared/constants/query-key.ts`). 키 관리 UI 는 `/settings` 탭(`widgets/settings/settings-tabs.tsx`)에 얹을 수 있다. 메뉴는 `shared/constants/navigation.ts` 의 `NAV_ITEMS`.
- 규칙: 화살표 함수만, `any`/`enum`/코드 주석 금지, FSD 역참조 금지, barrel 금지, Zod 경계 검증, TanStack Query + `queryOptions` + `QUERY_KEY`, `useCallback`/`useMemo` 금지(HANDOFF 5절, web/AGENTS.md는 Next 16 문서 확인 지시만).

---

## 6. 의존성

### 6.1 현재 `web/package.json` (L20-60)

런타임: `next 16.3.4`, `react 19.2.8`, `better-auth ^1.7.3`, `drizzle-orm ^0.45.2`, `mysql2 ^3.24.4`, `zod ^4.5.4`, `@tanstack/react-query ^5.102.8`, `react-hook-form`, `@hookform/resolvers`, `radix-ui`, `shadcn`, `sonner`, `server-only`, `lucide-react`, `next-themes`, `clsx`, `class-variance-authority`, `tailwind-merge`, `tw-animate-css`. 개발: `bun`(packageManager `bun@1.4.0`), `msw ^2.15.0`(설치만 되고 쓰는 테스트 없음, architecture.md:515), `@types/node ^20`, `typescript ^5` 등.

**AI SDK(`ai`, `@ai-sdk/*`), Anthropic/OpenAI/Ollama SDK 는 없다.** `web/node_modules` 에서 `ai`, `@ai-sdk`, `anthropic`, `openai`, `ollama` 모두 부재를 확인했다. 쓸 수 있는 것은 Node/Bun 전역 `fetch`(스트리밍 `ReadableStream` 포함), `node:crypto`, `node:zlib`, 전이 의존성으로 있는 `@noble/ciphers 2.4.0`·`@noble/hashes`·`undici`·`kysely`(직접 의존이 아니므로 import 하지 않는 것이 안전하다).

### 6.2 새로 넣는다면 후보만 (설치 금지, 레지스트리 최신 확인 2026-10-19)

| 후보 | 최신 | 역할 | 비고 |
| --- | --- | --- | --- |
| `ai` (Vercel AI SDK) | 7.0.143 (Apache-2.0, peer zod `^3.25.76 \|\| ^4.1.8`) | `generateText`/`streamText` + `output: Output.object({schema})` 로 구조화 출력, `AI_NoObjectGeneratedError` | 공식 문서 https://ai-sdk.dev/docs/ai-sdk-core/generating-structured-data . 어떤 공급자가 네이티브 구조화 출력이고 어떤 공급자가 폴백인지는 그 페이지에 나오지 않는다 |
| `@ai-sdk/anthropic` | 4.0.82 | Anthropic 어댑터 | |
| `@ai-sdk/openai` | 4.0.96 | OpenAI 어댑터 | |
| `ollama-ai-provider-v2` | 4.0.1 (peer `ai ^7`, zod `^4.0.16`) | Ollama 커뮤니티 어댑터 | 커뮤니티 문서 https://ai-sdk.dev/providers/community-providers/ollama 의 예시는 `baseURL: 'https://api.ollama.com'` 이지만 Ollama 공식 문서는 `https://ollama.com/api/chat` 이다. 서로 어긋나 확인이 필요하다. 구조화 출력 지원은 그 페이지에 없다 |
| `@anthropic-ai/sdk` | 0.135.0 (MIT) | 공식 SDK(대안) | |
| `openai` | 7.39.0 (Apache-2.0) | 공식 SDK(대안) | |
| `ollama` | 0.6.4 (MIT) | 공식 JS 클라이언트(대안) | |
| SDK 없이 `fetch` | - | 3개 어댑터를 직접 작성 | 의존성 0, 공급자 3곳 REST 모양이 단순(아래 6.3). 이 경우 `msw`(이미 devDependency)로 어댑터 테스트 가능 |
| `workflow` / `@vercel/queue` | - | 300초(플랜별 800초)를 넘는 장기 작업 | https://vercel.com/docs/workflows 에서 언급. 기획 단계에서는 필요 여부 미정 |

(`bun info` 는 package.json 이 없는 디렉터리에서 동작하지 않아 `registry.npmjs.org/<pkg>/latest` 응답으로 확인했다.)

### 6.3 공급자 3곳의 공식 API 사실 (어댑터 계층 근거)

| 항목 | Anthropic | OpenAI | Ollama Cloud |
| --- | --- | --- | --- |
| 호출 | `POST https://api.anthropic.com/v1/messages`, 헤더 `x-api-key`, `anthropic-version: 2023-06-01` | Responses API(`text.format`) 또는 Chat Completions(`response_format`), `Authorization: Bearer` | `POST https://ollama.com/api/chat`, `Authorization: Bearer <키>` (키는 ollama.com/settings/keys) |
| 구조화 출력 | **지원, GA**: `output_config.format = { type: "json_schema", schema }`, 베타 헤더 불필요, 옛 `output_format` 은 deprecated | **지원**: `text: { format: { type: "json_schema", strict: true, schema } }` | **공식 문서: "Ollama's cloud currently does not support structured outputs"** (로컬은 `format` 필드 지원) |
| 스키마 제약 | `additionalProperties: false` 필수, 재귀 불가, `minimum`/`maximum`/`minLength`/`maxLength` 불가, `minItems` 는 0 또는 1 만, 외부 `$ref` 불가. 지원하지 않는 기능은 400 | 모든 필드 `required`(선택은 null 유니언), `additionalProperties: false`, **루트는 object(루트 `anyOf` 불가)**, `allOf`/`not`/`if`/`then` 불가, 최대 5,000 속성·10단계 중첩·열거값 합계 1,000·문자열 총합 120,000자 | - |
| 거절·잘림 | `stop_reason: "refusal"`(200, 스키마와 안 맞을 수 있음), `max_tokens` 로 잘리면 JSON 이 깨질 수 있음 | `refusal` 콘텐츠 항목, `status: incomplete` + `max_output_tokens` | - |
| 첫 호출 | 새 스키마는 문법 컴파일 지연, 24시간 캐시 | 새 스키마는 첫 요청 지연 | - |
| 모델 나열(키 검증용) | `GET /v1/models` (`x-api-key`, `anthropic-version`) | `GET https://api.openai.com/v1/models` | `GET https://ollama.com/api/tags` |
| 기본 모델명 | 모델 id 는 `/v1/models` 응답으로 확인(`claude-sonnet-5-5`, `claude-opus-5-5`, `claude-haiku-5-5` 등이 구조화 출력 지원 목록에 있음) | 문서 권장 `gpt-6-astra`, `response_format json_schema` 는 `gpt-4o-2024-08-06` 이후 | `/api/tags` 가 주는 이름(예 `gemma4:31b`), 앱·CLI 에서는 `:cloud` 접미사 |
| 한도 | 문서 외 | 문서 외 | 플랜별 수치 없음, 사용량은 ollama.com/settings/usage |

출처: https://platform.claude.com/docs/en/docs/build-with-claude/structured-outputs , https://platform.claude.com/docs/en/api/models/list , https://developers.openai.com/api/docs/guides/structured-outputs , https://developers.openai.com/api/reference/resources/models/methods/list , https://docs.ollama.com/cloud , https://docs.ollama.com/capabilities/structured-outputs .

사용자 표현 "codex, api key"는 OpenAI Platform API 키(`Authorization: Bearer`)로 해석해야 호출이 성립한다. ChatGPT 구독 로그인 방식의 Codex 인증은 API 키가 아니며 이 조사에서 다루지 않았다(확인 질문 대상).

---

## 부록 A. 새 기능 10개와 현재 데이터의 격차

| # | 기능 | 지금 서버에서 가능한 입력 | 부족한 것 |
| --- | --- | --- | --- |
| 1 | 플레이어 리포트 | `score` 시계열(`score_player_idx`), 판정·FAST/SLOW·옵션, 램프 분포, 표 기록 | 약점·선호 "패턴"은 노트 배치·노트별 판정이 필요 -> (a)+(b) |
| 2 | 리플레이 코치 | 리플레이 입력 이벤트 | 구간별 BP·게이지 꺾임은 노트별 판정·게이지 곡선 필요 -> (a) |
| 3 | 베스트·라이벌 비교 | 각자의 `chart_best` 판정 합계 | 노트 단위 비교는 양쪽 노트별 판정 필요 -> (a), 공개 범위 결정 필요 |
| 4 | 판정 습관 진단 | `fast`/`slow`, `avgjudge`(세대 4+), `extra.mean_offset_us`, `offset_ms`, `green_number`, `hispeed` | `offset_ms` 의 빌드별 의미 차이 처리 |
| 5 | 다음 도전 곡 추천 | `getRecords`(표 1,035곡 램프), 표 레벨 | 곡 난이도 특성은 (b) 패턴으로 보강 |
| 6 | AI 연습 폴더 | 폴더 서비스(생성·교체), md5 호환 | 폴더 이름 중복·50개 한도 처리 |
| 7 | 목표 계획 | 위 1·5 의 결정론 집계 | 사용자 목표 저장 표 필요 |
| 8 | 게이지·옵션 조언 | `gauge`, `random`, `option`, `gauge_value`, 램프 | 게이지 곡선은 (a) |
| 9 | 자연어 기록 질문 | 기존 조회 DTO(`playerScoresQuerySchema` 등)를 확장한 필터 스키마 | AI 는 필터 JSON 만 만들고 서버가 허용 목록으로 쿼리(SQL 금지) |
| 10 | 주간 성장 요약 | `played_at` 범위 집계 | 저장 주기·무효화 정책 |

## 부록 B. 이 조사로 확인된 기존 서버의 이상 징후 (설계가 부딪힐 지점)

- `chart_best` 에 `user_id` 앞 색인이 없다(1.1절).
- 레이트리밋이 인스턴스 로컬이다(5.2절).
- `withAdmin` 이 세션만 받는다(3.3절).
- 서버가 재판정하지 않는다(`verified` 항상 false, 4.2절).
- 시드 2^53 초과가 0 으로 저장된다(1.2절).
- 리플레이 `extra` 가 저장되지 않는다(2.1절).

---

## 조사자가 낸 제약 목록 (원문)

1. 코드 규칙(HANDOFF 5절, web/AGENTS.md 및 전역 컨벤션): 화살표 함수만, any·enum·코드 주석 금지, JSDoc 은 영어만, 자명한 타입 명시 금지(ReturnType·z.infer 등으로 유도), FSD(app>widgets>features>entities>shared, 역참조·barrel 금지), 1파일 1컴포넌트, FC<Props>, useCallback·useMemo 금지, Zod 로 경계 검증, TanStack Query + queryOptions + QUERY_KEY, 서버 계층은 Route(DTO·HOF·에러 throw) -> Service(HTTP·Drizzle 모름) -> compose 의 ServiceDb 구현(Drizzle·트랜잭션은 여기만). 새 에러 코드는 error-code.ts·error-message.ts·error.ts 세 파일에 함께 추가.
2. 마이그레이션: drizzle-kit push 금지, db:generate 로 만들고 drizzle/*.sql 과 drizzle/meta/ 를 커밋, 저널 when 단조 증가. dev 푸시가 곧 운영 배포이므로 순서는 마이그레이션 파일만 먼저 푸시 -> web-db.yml status -> migrate -> status -> 코드 푸시. 운영 DB 변경은 web-db.yml 로만, 직접 접속·mysql CLI 금지. 새 코드가 아직 없는 표를 읽으면 500 이 난다.
3. BYOK 키는 서버에 암호화 저장하며 평문은 요청 처리 중 메모리에만 둔다. 저장·로그·에러 details·응답·클라이언트 번들·git 어디에도 평문 키를 남기지 않고 UI 에는 마지막 몇 글자만 보인다. 마스터 키는 새 환경변수로 분리하고(BETTER_AUTH_SECRET 재사용 지양) env.ts 스키마는 optional 로 두되 사용 시점에 검사해 환경변수 없는 next build 가 성공해야 한다. 키 로테이션(버전 필드)과 사용자·공급자에 암호문을 묶는 방법(AAD 또는 별도 검증)을 설계에 넣는다.
4. 키를 쓰는 엔드포인트는 세션 전용으로 한다(API 토큰 Bearer 로는 복호화·호출 금지). 모든 AI 응답은 Cache-Control: private, no-store, 'use cache' 에 개인 데이터·키를 넣지 않는다. CSP connect-src 'self' 이므로 공급자 호출은 서버에서만 한다.
5. 공급자는 Anthropic, OpenAI, Ollama Cloud 셋만이고 Ollama 는 https://ollama.com 고정이다(사용자 지정 baseURL 허용 금지: SSRF 방지). 세 어댑터가 같은 Zod 출력 스키마를 공유하되 스키마는 공통 부분집합으로 제한한다: 루트 object, 모든 필드 required(선택은 null 유니언), additionalProperties false, min/max/minLength/pattern·allOf·재귀·루트 anyOf 금지, 최대 10단계 중첩. Ollama Cloud 는 공식 문서상 네이티브 구조화 출력이 없으므로 프롬프트에 스키마를 싣고 JSON 파싱 + Zod 검증 + 제한된 재시도 폴백이 필요하며, 모든 어댑터는 모델 출력을 신뢰하지 않고 Zod 로 재검증한다.
6. 집계·패턴 분류·필터 컴파일은 결정론 코드가 하고 AI 는 해석·설명·추천만 한다. 자연어 질문은 AI 가 정해진 필터 스키마(허용 목록 필드·연산자·상한)만 만들고 서버가 Drizzle 파라미터 바인딩으로 실행한다(SQL 금지, security.md). AI 가 고른 md5·id 는 서버가 실제로 존재하는 값인지 검증한 뒤 폴더에 넣는다.
7. 차트 노트 배치 전체는 비공개로 저장하고 공개 API 로 내보내지 않는다. 저장소가 공개(B-HS/R-BMS)이므로 패턴 데이터를 web/data·docs·git 에 넣지 않는다. 입력 경로는 관리자 전용 API 여야 하는데 현재 withAdmin 은 세션만 받으므로 CLI 업로드를 허용하려면 관리자 Bearer 경로를 새로 설계한다. Vercel 본문 4.5 MB 한도 때문에 곡 단위 요청으로 나눈다. 원본 BMS 파일·소리·BGA 는 업로드하지 않는다.
8. (b) 곡별 패턴의 키는 발광 표가 가진 md5 다(표에 sha256 없음). 서버 chart.md5 는 unique 이자 nullable 이다. 발광 표 1,035곡 전부 BEAT_7K 이고 사용자 라이브러리에 전부 있다(측정). 14K·9K·5K 와 표 밖 곡은 (b) 로 덮이지 않는다.
9. (a) 판별 업로드는 새 와이어 계약이다: 옵션 적용 뒤 실제 배치, 노트별 판정(상태·부호 있는 타이밍), 게이지 곡선, 필요한 옵션. 기존 리플레이 와이어는 알고리즘·LN 종류·시계·TOTAL·LN MARGIN 이 없어 서버 재판정이 불가능하다. 클라이언트는 NoteMark·PlayRecord·GaugeLog 로 만들 수 있고 FRAME 시계로 저장된 옛 리플레이 재판정은 근거로 쓰지 않는다. 새 필드는 contract/client-requests 교차 테스트(RBMS_UPDATE_CONTRACT=1)와 web/tests/dto/client-requests.test.ts 에 같이 넣고, 서버는 업로드를 점수 행의 판정 합계·early/late·fast/slow 와 일관되는지 검사한다(서버는 재판정하지 않는다). 클라이언트 변경은 Rust 규칙(//주석 금지, 새 의존성 금지, rustfmt 160)을 따른다.
10. 서버에 이미 있는 값의 신뢰도를 분석 쿼리에 반영한다: seed 는 2^53 초과 시 0 으로 저장, avgjudge 는 rule_version(score.extra) 4 이상에서만 원본 정의, offset_ms 의 의미가 빌드마다 다르며 assist 는 태그 개수만 남는다, chart.level 은 PLAYLEVEL 이고 발광 레벨은 표에서만 온다(md5 매칭), chart.notes 와 BPM·TOTAL 은 신뢰할 수 없거나 비어 있다, 한 md5 에 sha256 이 여럿이면 한 건만 담긴다.
11. 공급자 호출은 DB 연결을 잡은 채 하지 않는다(풀 5개, 락 대기 5초). 장시간 호출은 maxDuration 을 명시해야 하며 기본 300초(Hobby 최대 300초, Pro 최대 800초)와 응답 4.5 MB 한도 안에서 스트리밍 또는 비동기 작업으로 설계한다. 사용자별 일·분 한도와 중복 요청 방지는 인스턴스 로컬 카운터가 아니라 DB 기반으로 둔다.
12. BYOK 이므로 비용·한도 오류, 키 만료, 공급자 장애, 거절(refusal)·잘림(max_tokens) 을 사용자 문구로 구분해 반환하고 details 는 프로덕션에서 field·reason 문자열만 나간다(기존 정책). 실패 시 결과를 캐시하지 않는다.
13. 기존 규칙 유지: 새 의존성은 최신 버전을 쓰되 추가 전에 사용자에게 근거를 묻는다(기본안은 SDK 없이 fetch 또는 Vercel AI SDK 후보 중 선택), Rust 는 새 의존성 금지, 계약이 바뀌면 client-requests fixture 로 교차 검증, 에이전트는 커밋·푸시하지 않고 rbms-player 를 실행하지 않으며 분석에 쓰는 로컬 DB 는 scratch 사본만 읽는다.

## 확인하지 못한 것 (원문)

1. Vercel 프로젝트 플랜(Hobby/Pro)과 Fluid compute 사용 여부를 확인하지 못했다. 최대 실행 시간(300초 vs 800초)과 과금이 달라진다. Vercel 대시보드와 MCP 인증에 접근하지 않았다.
2. 운영 DB 호스트 종류와 max_allowed_packet, 저장소 용량 한도를 확인하지 못했다. 패턴 약 18~27 MB(추정)와 노트별 판정 업로드가 mediumtext(최대 16 MB)·패킷 한도에 걸리는지 모른다.
3. 운영 DB 의 실제 규모를 확인하지 못했다: chart 행 수, md5 가 비어 있는 chart 행의 비율, 사용자 수, 리플레이 보유율(대기 리플레이 43개는 HANDOFF 기록만), 플레이어별 score 행 수. 운영 DB·사이트에 요청하지 않았다.
4. 발광 표 1,035곡의 sha256 은 표에 없다. CLI 가 md5 와 sha256 을 함께 내므로 서버 chart.sha256 과 잇는 방법은 가능하지만 한 md5 에 sha256 이 여럿인 경우(서버 쪽 중복)의 빈도를 모른다.
5. Ollama Cloud 가 지금도 구조화 출력을 지원하지 않는지는 문서 한 줄에만 근거한다(실제 호출로 확인하지 않음, 키 없음). Ollama 커뮤니티 문서의 baseURL(api.ollama.com)과 공식 문서(ollama.com/api/chat)가 달라 어느 쪽이 맞는지, OpenAI 호환 /v1 경로가 클라우드에서 response_format 을 받는지 확인하지 못했다.
6. 사용자의 'codex, api key' 가 OpenAI Platform API 키를 뜻하는지, ChatGPT 구독 기반 Codex 인증을 뜻하는지 확인하지 못했다.
7. 공급자별 모델 id 기본값, 요금, 레이트리밋, 컨텍스트 한도를 확인하지 않았다(Anthropic·OpenAI 모델 목록은 문서 시점의 이름이고 BYOK 라 사용자가 고를 수 있어야 한다). 한국어 출력 품질과 구조화 출력 스키마 컴파일 지연의 실제 값도 모른다.
8. better-auth/crypto 의 symmetricEncrypt 를 공식 문서가 공개 API 로 보장하는지, AES-256-GCM(node:crypto, AAD 가능)과 어느 쪽을 쓸지 확인하지 못했다. 키 로테이션·유출 시 대응 절차(사용자 키 일괄 무효화)는 설계 전이다.
9. 노트별 판정 업로드의 공개 범위(리플레이처럼 공개 / 본인·라이벌만 / 완전 비공개)와 라이벌의 노트별 데이터를 요청자의 공급자 키로 외부 AI 에 보내도 되는지는 새 결정이 필요하다. 기존 리플레이는 이미 공개다.
10. (a) 업로드를 판이 끝날 때 한 번에 올릴지, 로컬 리플레이 재판정으로 과거 판까지 채울지(과거 리플레이를 EVENT 시계 판만 쓸 수 있음), 어떤 판(실패·Esc·어시스트·SCRATCH AUTO·넓힌 판정)을 포함할지는 정하지 않았다. 사람이 친 판을 임의로 제외하지 말라는 HANDOFF 규칙과 '노트를 하나도 치지 않은 판' 제외가 적용되는 범위를 맞춰야 한다.
11. rbms-cli 패턴 추출 하위 명령의 출력 형식(곡당 JSON, 시각 단위, LN 표현), 업로드 인증(관리자 Bearer 신설 vs 웹 관리자 화면 업로드), 재실행 멱등과 버전(PARSER_VERSION 5)과의 관계는 설계 전이다. 14K·9K 표 밖 곡을 (b) 에 포함할지도 미정이다.
12. 쓰기 시점 무결성: 노트별 판정 합계와 점수 행의 판정 수가 서로 어긋났을 때(예: 롱노트 CN/HCN 이중 판정, 9K BAD 처리) 서버가 어떻게 처리할지, 서버가 불일치를 허용하는 오차가 있는지(허용 오차 없음 원칙)는 정하지 않았다.
13. rbms-cli 바이너리(target/release)가 현재 소스와 같은지 확인하지 않았다. 노트 수 측정은 그 바이너리의 count_playable_notes 결과이고 LN 끝·지뢰를 노트로 세는 기준이 서버 chart.notes 와 같은지 대조하지 않았다.
14. AI 결과(리포트·코치·주간 요약)를 DB 에 저장할지, 저장 시 보존 기간·무효화(새 점수 제출 때 stale 표시)·언어(한국어) 정책이 정해지지 않았다. 사용자 일일 호출 한도와 한 번에 보낼 수 있는 컨텍스트(토큰) 상한도 미정이다.
15. 기존 서버 이슈의 영향 범위: chart_best 의 user_id 색인 부재가 실제 데이터 규모에서 문제인지, 레이트리밋 지속 저장소(Upstash 등, 사용자 결정 대기 중 HANDOFF 6절 10)를 AI 한도에 쓸지 미확인이다.
