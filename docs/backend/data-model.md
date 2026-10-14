# rbms 백엔드 — 데이터 모델 (Drizzle / MySQL)

> 최종 갱신 2026-10-14 · 대응 커밋 7095ef7

> 구현 정본은 `web/src/server/db/{schema,auth-schema}.ts`입니다. 테이블/컬럼은 **snake_case**, TS 필드는 **camelCase**, 모델 타입은 `$inferSelect`/`$inferInsert`에서 유도합니다. 시각은 `timestamp(fsp:3)` 또는 `bigint`(epoch ms)입니다. 설계 초안의 FK 표기는 일부가 **논리 관계**이며, 실제 `.references()`는 스키마 파일에 적힌 것만 존재합니다.

## ER 개요
```
user 1─* score *─1 chart        score *─0..1 replay
user 1─* setting_blob           score —(option/seed/gauge/judge early·late 인라인)
user *─* rival (self-ref)        course 1─* course_chart *─1 chart
user 1─* course_score *─1 course difficulty_table 1─* table_folder 1─* table_chart *─1 chart
session/account (better-auth)    difficulty_table 1─* table_course *─1 course
user 1─* user_folder 1─* user_folder_item   (내 폴더, 마이그레이션 0001)
```

## 테이블

### `user` (better-auth 호환 + IR 프로필)
| 컬럼 | 타입 | 비고 |
|---|---|---|
| id | varchar(36) PK | better-auth 생성 사용자 id |
| login_id | varchar(64) uniq | IR 로그인 id, `/api/players/{id}`가 해석하는 공개 식별자 |
| name | varchar(255) | 표시명 |
| email | varchar(255) | better-auth 이메일 |
| password_hash | 없음 | better-auth `account` 테이블이 소유 |
| rank | varchar(32) default '' | 단위/등급(IRPlayerData.rank) |
| rank_points | double default 0 | 산출 점수. **쓰는 코드가 없어 항상 0**(웹 순위는 쓰지 않는다) |
| total_plays | bigint default 0 | **쓰는 코드가 없어 항상 0**. 응답의 `total_plays` 는 `score` 행 수를 세서 채운다 |
| role | varchar(16) default 'user' | user/admin |
| allow_guest_merge | boolean default false | — |
| extra | json null | free-form |
| created_at / updated_at | timestamp(3) | — |

### `session` / `account` / `verification` — better-auth 표준 테이블(컨벤션 §11). 토큰 해시 저장.

### `api_token` (IR Bearer)
| id varchar(36) PK · user_id FK · token_hash varchar(255) uniq · label · last_used_at · created_at · revoked bool |

### `chart` (IRChartData 슈퍼셋) — md5↔sha256 연결의 단일 출처
| 컬럼 | 타입 | 비고 |
|---|---|---|
| sha256 | char(64) PK | 정본 키 |
| md5 | char(32) **uniq idx** | LR2 호환 키 |
| title/subtitle/genre/artist/subartist | varchar | 메타 |
| level | int null · total | double null · mode varchar(16) · lntype int |
| judge int · minbpm int · maxbpm int · notes int |
| has_ln/has_cn/has_hcn/has_mine/has_random/has_stop | boolean |
| url / appendurl | varchar(512) null |
| extra | json · created_at/updated_at |
- 인덱스: `PK(sha256)`, `uniq(md5)`. 제출 시 둘 중 있는 키로 upsert, 다른 키 backfill.

### `score` (IRScoreData 슈퍼셋) — 모든 제출 history
| 컬럼 | 타입 | 비고 |
|---|---|---|
| id | varchar(36) PK | `sc_…` |
| user_id | varchar(36), `user.id` 참조 | 제출자(guest는 null + guest_name) |
| chart_sha256 | char(64), 논리 chart 관계 | 정본 차트 키 |
| chart_md5 | char(32) null idx | 호환 |
| mode varchar(16) · lntype int(0=LN,1=CN,2=HCN) |
| clear | tinyint | ClearType id 0-10 |
| **epg lpg egr lgr egd lgd ebd lbd epr lpr ems lms** | int | **판정 early/late 분리(레퍼런스 구현)** |
| empty_poor | int default 0 | rbms 空POOR |
| avgjudge | bigint | µs 평균 판정오차 |
| ex_score int · max_ex_score int · max_combo int · notes int · passnotes int · minbp int |
| gauge_value | float | 종료 게이지 |
| **── 플레이 전체 옵션(공평 평가 단일 출처) ──** | | |
| gauge | tinyint | GaugeType(플레이 게이지 종류) |
| option | int | 노트옵션 비트필드(레퍼런스 구현) |
| random | varchar(16) · random_p2 varchar(16) null · scratch_left bool · scratch_auto bool |
| seed | bigint | 셔플 시드(참고용). 안전 정수(±2^53 미만)가 아닌 값은 서버가 0 으로 저장한다(`seedSchema`) |
| hispeed | double · constant bool · green_number int null |
| lift | float · lane_cover float |
| assist int · judge_rate int default 100 · offset_ms int default 0 · auto_offset bool |
| total_override | double default 0 | 0=차트값, >0=오버라이드(게이지 난이도 영향) |
| autoplay | boolean default false | 오토플레이 여부 |
| input_device varchar(32) · judge_algorithm varchar(16) · rule varchar(16) · skin varchar(64) |
| **── 무결성 / 평가 ──** | | |
| client | varchar(64) | 클라 표시 버전(`rbms/x.y.z`) |
| client_build_sha256 | char(64) idx | 실행 클라 빌드 해시(변조 탐지) |
| client_platform | varchar(32) | 빌드 타깃 |
| ranked | boolean default true idx | 랭킹 적격(서버 산출) |
| flags | json null | unranked/의심 사유(`["AUTOPLAY",…]`) |
| verified | boolean default false | µs 리플레이 재시뮬 검증(후속) |
| replay_id | varchar(36) null, 논리 replay 관계 |
| played_at | bigint(epoch ms) idx · created_at |
| extra | json |
- 인덱스: `uniq(user_id, chart_sha256, played_at)`(같은 밀리초의 이중 삽입을 막는다. 멱등 판정은 서비스가 같은 초 안의 `played_at` 을 먼저 찾아 하며, 규칙은 api-spec 의 멱등 항), `idx(chart_sha256, ranked, clear, ex_score)`(랭킹은 ranked=true만), `idx(user_id, played_at)`, `idx(client_build_sha256)`.
- EX/BP는 저장하되 `judge`에서 유도 가능(`ex=(epg+lpg)*2+egr+lgr`). 랭킹/best는 **ranked=true**만.

### `chart_best` (랭킹 가속 — (chart, user, scratch_auto) 최고 1행)
| chart_sha256 + user_id + scratch_auto **복합 PK** · score_id FK · clear · ex_score · minbp · max_combo · scratch_auto bool default false · updated_at |
- best 갱신 규칙: 램프 > EX > (BP 낮을수록) 순. 랭킹 조회는 이 테이블 정렬.
- 웨이브 20(마이그레이션 `0002_scratch_auto_bests`): SCRATCH AUTO 판도 ranked 이고, 한 사용자가 한 차트에 일반 판 최고(`scratch_auto=false`)와 SCRATCH AUTO 판 최고(`true`)를 따로 가진다. 기본 순위는 `scratch_auto=false` 행만, 포함 순위는 사용자마다 두 행 가운데 더 높은 것(모두 같으면 일반 판)만 읽는다(`web/src/server/compose/best-scope.ts`). 0002 이전 행은 기본값 false 로 일반 판이 된다. 인덱스 `chart_best_rank_idx` 는 `(chart_sha256, scratch_auto, clear, ex_score)`.

### `replay` (µs 정밀)
| id varchar(36) PK · user_id varchar(36) · chart_sha256 idx · score_id null · format varchar(32) · mode/random/random_p2/seed/lntype/offset_ms/judge_rate/scratch_auto/constant/gauge(재현용) · client_build_sha256 char(64) · **event_count int · duration_us bigint** · size int · storage_key varchar(255) null · `data` mediumtext null · created_at |
- 현재 `REPLAY_STORAGE=db`는 events JSON을 `data`에 저장하고 `storage_key`는 null입니다. 외부 object storage와 `REPLAY_STORAGE=blob`은 미구현입니다. 상한은 `REPLAY_MAX_BYTES`입니다.
- **µs 보존**: 이벤트 타임스탬프 `t_us`(i64)를 라운딩 없이 저장 → 핵분석(등간격·반응속도·동시성)·재시뮬 가능.

### `client_build` (빌드 allowlist — 변조 탐지)
| sha256 char(64) PK · version varchar(32) · platform varchar(32) · channel varchar(16)(stable/dev) · released_at bigint · trusted boolean default true · note varchar(255) · created_at |
- `/api/admin/builds`는 등록 경로를 제공하지만, 실제 키를 쓰는 릴리스 CI 등록은 Phase R에서 검증합니다. 미등록 해시는 `UNKNOWN_BUILD` flag가 되며 `REQUIRE_BUILD_HASH` 설정에 따라 ranked 여부가 달라집니다.

### `submission_audit` (제출 감사)
| id PK · score_id FK null · user_id null · client_build_sha256 char(64) idx · client_platform · ip varchar(64) · user_agent varchar(255) · accepted bool · ranked bool · flags json · created_at idx |
- 모든 제출(거부 포함) 기록 → 어뷰즈·변조 추적, 빌드 분포 분석.

### `rival` (self-ref M:N)
| user_id varchar(64) + rival_id varchar(64) **복합 PK** · created_at |

### `setting_blob` (설정 동기화)
| user_id + name(varchar(64)) **복합 PK** · format varchar(16) · content mediumtext · size int · updated_at |
- 이름별 `/api/players/{id}/settings/{name}` 경로가 구현돼 있습니다. 낙관적 잠금=`updated_at` vs req `base_updated_at`.

### `course` / `course_chart` / `course_score`
- `course`: `course_hash` PK · name · lntype · constraint json · trophy json · extra.
- `course_chart`: course_hash + position **복합 PK** · chart_sha256(논리 chart 관계).
- `course_score`: id PK · user_id · course_hash idx · clear · judge(early/late 동일 컬럼군) · ex_score · max_combo · gauge_value · minbp · trophy · scratch_auto bool default false · ranked · played_at · extra · `uniq(user_id, course_hash, played_at)`. `scratch_auto` 는 SCRATCH AUTO 스테이지가 낀 코스다(웨이브 20, 0002). 넓힌 판정 스테이지가 낀 코스(`assist` 에 `AUTO_SCRATCH` 밖의 태그)는 `ranked=false` 이다. flag 가 하나라도 있으면 `extra` 에 `flags`·`scratch_auto`·`assist` 를 담는다(api-spec §6).
- `course_best`: course_hash + user_id + scratch_auto 복합 PK(랭킹 가속). `chart_best` 와 같이 일반 판 최고와 SCRATCH AUTO 판 최고를 따로 둔다(0002).
- `score.scratch_auto` 는 0002 전부터 있던 열이고, 웨이브 20부터는 `options.scratch_auto` 이거나 `options.assist` 에 `AUTO_SCRATCH` 가 있는 판이면 true 로 적는다. 그래서 점수 행에는 새 열을 더하지 않았다.

### `difficulty_table` / `table_folder` / `table_chart` / `table_course` (IRTableData)
- 서버가 큐레이션하는 IR 표용이다. 발광 난이도표(`insane1`)는 이 테이블이 아니라 파일 스냅샷(`web/data/tables/insane1/`)으로 제공한다: `table_chart.chart_sha256` 이 NOT NULL 이라 md5 만 있는 표 항목을 담지 못하기 때문이다(결정 U1, [../web/2026-10-14-insane-table.md](../web/2026-10-14-insane-table.md)). 운영 `GET /api/tables` 는 지금 `[]` 이다.
- `difficulty_table`: id PK · name · url · extra.
- `table_folder`: id PK · table_id(논리 difficulty_table 관계) · name · position.
- `table_chart`: folder_id + chart_sha256 복합 PK.
- `table_course`: table_id + course_hash 복합 PK.

### `user_folder` / `user_folder_item` (내 폴더, 마이그레이션 `0001_user_folders`)
- `user_folder`: `id` varchar(36) PK(`uf_` + 16진 30자) · `user_id` varchar(36) NOT NULL **FK → `user.id` ON DELETE CASCADE** · `name` varchar(64) · `description` varchar(255) default '' · `is_public` boolean default false(v1 은 항상 false, 결정 Q3) · `position` int · `item_count` int(저장된 곡 수 캐시) · `created_at` timestamp(3) · `updated_at` **bigint(epoch ms, 낙관적 잠금 기준)**. 인덱스 `uniq(user_id, name)`, `idx(user_id, position)`.
- `user_folder_item`: `folder_id` varchar(36) **FK → `user_folder.id` ON DELETE CASCADE** + `md5` char(32) **복합 PK** · `sha256` char(64) null · `title` varchar(200) · `artist` varchar(200) · `note` varchar(100) · `position` int · `created_at`. 인덱스 `idx(folder_id, position)`, `idx(md5)`. 곡의 식별자는 md5 이고 제목·아티스트는 담을 때의 사본이다.
- 한도(사용자당 폴더 50, 폴더당 2,000곡, 사용자 전체 20,000곡)와 API 는 [api-spec.md](./api-spec.md) 7.2 절, 설계 이유는 [../plan/2026-10-14-web-user-folders.md](../plan/2026-10-14-web-user-folders.md).

## 타입 유도 (예)
```ts
type Score = typeof score.$inferSelect
type NewScore = typeof score.$inferInsert
type ScoreRankingRow = Pick<Score, 'userId'|'clear'|'exScore'|'maxCombo'|'minbp'|'playedAt'> // 랭킹 응답 파생
type ChartKey = Pick<typeof chart.$inferSelect, 'md5'|'sha256'>
```

## 마이그레이션
- `web/drizzle.config.ts`(`schema: ['./src/server/db/schema.ts', './src/server/db/auth-schema.ts']`, `dialect:'mysql'`, `out:'./drizzle'`) → `web/drizzle/`. 현재 파일 3개이고 `meta/` 도 커밋돼 있다: `0000_smooth_cerebro`(21 테이블, 저널 `when` 은 일부러 작은 2026-09-01 값) → `0001_user_folders`(폴더 두 테이블) → `0002_scratch_auto_bests`(`chart_best`·`course_best`·`course_score` 에 `scratch_auto` 열, `chart_best`·`course_best` 의 PK 에 `scratch_auto` 추가, `chart_best_rank_idx` 재생성). 운영 DB 에는 세 개가 모두 적용됐다(2026-10-14). 적용 절차, 저널 `when` 규칙, 기준선, 운영 DB 워크플로(`web-db.yml`)는 [../web/architecture.md](../web/architecture.md) 3-2 절이 정본이다. `drizzle-kit push` 는 쓰지 않는다.

## 설계 노트 (왜)
- **best 분리 테이블**: 랭킹은 차트당 수천 history를 매번 정렬하지 않고 `chart_best`만 정렬 → P95<150ms(NFR).
- **early/late 인라인**: 레퍼런스 구현 IRScoreData를 무손실 보존(EX·BP·avgjudge·FAST/SLOW 전부 유도). 별도 judge 테이블은 조인비용↑이라 인라인.
- **md5 null 허용**: sha256만 있는 레퍼런스 구현-only 차트 수용. md5만 있는 LR2 차트는 sha256 backfill 전까지 md5 키로 동작(보조 인덱스).
- **replay 저장 경계**: 현재 DB `mediumtext` 저장으로 기능을 제공하고, 규모 확장용 외부 Blob 저장소는 후속입니다.
