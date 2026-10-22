# rbms 백엔드 — 데이터 모델 (Drizzle / MySQL)

> 최종 갱신 2026-10-22 · 대응 커밋 47a7f9c (2026-10-22 에 `chart_best`·`course_best` 의 `assist_grade` 열·색인과 0011 만 더했다. 나머지는 2026-10-15 기준이라 0005~0010 의 표는 아직 이 문서에 없고, 정본은 `web/src/server/db/schema.ts` 와 [../web/architecture.md](../web/architecture.md) 다)

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
| name | text | 표시명(가입 DTO 가 64자로 묶는다) |
| email | varchar(255) uniq | better-auth 이메일 |
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
| id varchar(36) PK · user_id varchar(36) FK → `user.id` ON DELETE CASCADE(idx) · token_hash varchar(255) uniq(SHA-256 hex) · label varchar(64) · last_used_at bigint(epoch ms) null · created_at · revoked bool |
- 폐기는 `revoked=true` 로 남긴다: 웹 설정의 토큰 폐기, 세션 토큰 회전(`POST /api/auth/token`, 그 사용자 토큰 전부), 로그아웃(`DELETE /api/auth/token/current`, 보낸 토큰 하나), 이전 토큰을 Bearer 로 실은 로그인·가입의 성공(그 토큰 하나, S19). 만료 열은 없다.

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
- `notes` 는 그 차트를 처음 제출한 본문의 `total_notes`(또는 일반 사용자 토큰의 `POST /api/charts`)가 정한 값이고 뒤의 제출이 고치지 않는다. 신뢰할 수 있는 출처가 아니므로 **표시용 메타**로만 쓰고 제출 검증의 상한으로 쓰지 않는다(S19 2차 수리 R2-ACC-2: 상한으로 쓰던 동안 `total_notes:1` 인 첫 제출 하나가 그 차트의 이후 정상 판을 모두 400 으로 막았다). 잘못 심긴 값의 점검·보정 SQL 은 테스트 부채 3.36.

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
| hispeed | double · constant bool · green_number int null(반올림한 값이 INT 밖이면 null, S19 3차) |
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
- 인덱스: `score_idempotent_uniq(user_id, chart_sha256, played_at)`(같은 밀리초의 이중 삽입을 막는다. 멱등 판정은 서비스가 같은 초 안의 `played_at` 을 이 색인의 범위 탐색으로 먼저 찾아 하며, 규칙은 api-spec 5절의 멱등 항), `score_ranking_idx(chart_sha256, ranked, clear, ex_score)`, `score_player_idx(user_id, played_at)`, `score_build_idx(client_build_sha256)`, `score_recent_idx(played_at)`.
- EX/BP는 저장하되 `judge`에서 유도 가능(`ex=(epg+lpg)*2+egr+lgr`). 랭킹/best는 **ranked=true**만.

### `chart_best` (랭킹 가속 — (chart, user, scratch_auto) 최고 1행)
| chart_sha256 + user_id + scratch_auto **복합 PK** · score_id varchar(36)(논리 score 관계, FK 없음) · clear · **assist_grade tinyint default 0 not null**(0011) · ex_score · minbp · max_combo · scratch_auto bool default false · updated_at |
- best 갱신 규칙: 램프 > 어시스트 등급 > EX > (BP 낮을수록) 순(2026-10-22 부터. 등급은 램프 `LightAssistEasy`(3) 안에서만 0~4 이고 다른 램프는 0). 랭킹 조회는 이 테이블 정렬. 갱신은 `INSERT IGNORE` + `UPDATE … WHERE (저장된 값보다 좋음)` 문장 하나라 동시 제출이 낮은 판으로 덮지 못한다(S19, `compose/best-scope.ts` 의 `isBeatenBy`). 목록 정렬의 마지막 키는 `updated_at`, `user_id` 다(`listOrder`).
- 웨이브 20(마이그레이션 `0002_scratch_auto_bests`): SCRATCH AUTO 판도 ranked 이고, 한 사용자가 한 차트에 일반 판 최고(`scratch_auto=false`)와 SCRATCH AUTO 판 최고(`true`)를 따로 가진다. 기본 순위는 `scratch_auto=false` 행만, 포함 순위는 사용자마다 두 행 가운데 더 높은 것(모두 같으면 일반 판)만 읽는다(`web/src/server/compose/best-scope.ts`). 0002 이전 행은 기본값 false 로 일반 판이 된다. 인덱스 `chart_best_rank_idx` 는 `(chart_sha256, scratch_auto, clear, ex_score)`.
- 마이그레이션 `0011_assist_grade`(2026-10-22): `assist_grade` 는 최고 기록 행이 가리키는 `score` 의 `clear`·`gauge` 에서 파생한 값이다(`clear = 3` 이면 `gauge` 1 → 1, 2 → 2, 3 → 3, 4 → 4, 5 → 4, 6 → 2, 7 → 3, 8 → 4, 그 밖 0. `web/src/server/service/domain/score/assist-grade.ts` 의 `ASSIST_GRADE_OF_GAUGE`). 새 색인 `chart_best_rank_grade_idx(chart_sha256, scratch_auto, clear, assist_grade, ex_score)` 가 순위·비교 질의를 받치고, 옛 `chart_best_rank_idx` 는 additive 규칙에 따라 그대로 둬 앞부분이 겹친다(옛 코드와의 공존이 끝나면 후속 마이그레이션에서 지울 수 있다). 기존 행은 마이그레이션의 마지막 문장(`chart_best` ⨝ `score` 백필 UPDATE)이 채웠다. 푸시 사이 창에서 생길 수 있는 낡은 등급은 `web-db.yml` 액션 `assist-grade-backfill`(세기만 하는 `assist-grade-backfill-count` 도 있다)이 바로잡는다. 운영에서 낡은 행 0 을 확인했다. 의미: [../reference/clear-lamps.md](../reference/clear-lamps.md) 1.8·3.3절.

### `replay` (µs 정밀)
| id varchar(36) PK · user_id varchar(36) idx · chart_sha256 idx · score_id null idx · format varchar(32) · mode/random/random_p2/seed/lntype/offset_ms/judge_rate/scratch_auto/constant/gauge(재현용) · client_build_sha256 char(64) · **event_count int · duration_us bigint** · size int · storage_key varchar(255) null · `data` mediumtext null · created_at |
- 현재 `REPLAY_STORAGE=db`는 events JSON을 `data`에 저장하고 `storage_key`는 null입니다. 외부 object storage와 `REPLAY_STORAGE=blob`은 미구현입니다. 한 건의 상한은 `REPLAY_MAX_BYTES`, 사용자별 합계 상한은 2,000개·512MiB(`REPLAY_USER_QUOTA`, S19)이며 합계는 `replay_user_idx(user_id)`(마이그레이션 `0003_replay_user_quota`)로 읽는다. `size` 는 실제 저장 길이보다 작게 기록되지 않는다.
- **µs 보존**: 이벤트 타임스탬프 `t_us`(i64)를 라운딩 없이 저장 → 핵분석(등간격·반응속도·동시성)·재시뮬 가능.

### `client_build` (빌드 allowlist — 변조 탐지)
| sha256 char(64) PK · version varchar(32) · platform varchar(32) · channel varchar(16)(stable/dev) · released_at bigint · trusted boolean default true · note varchar(255) · created_at |
- `/api/admin/builds`는 등록 경로를 제공하지만, 실제 키를 쓰는 릴리스 CI 등록은 Phase R에서 검증합니다. 미등록 해시는 `UNKNOWN_BUILD` flag가 되며 `REQUIRE_BUILD_HASH` 설정에 따라 ranked 여부가 달라집니다.

### `submission_audit` (제출 감사)
| id PK · score_id null(논리 관계, FK 없음) · user_id null · client_build_sha256 char(64) idx · client_platform · ip varchar(64) · user_agent varchar(255) · accepted bool · ranked bool · flags json · created_at idx |
- 모든 제출(거부 포함) 기록 → 어뷰즈·변조 추적, 빌드 분포 분석.

### `rival` (self-ref M:N)
| user_id varchar(36) + rival_id varchar(36) **복합 PK**(둘 다 `user.id`) · created_at |

### `setting_blob` (설정 동기화)
| user_id + name(varchar(64)) **복합 PK** · format varchar(16) · content mediumtext · size int · updated_at bigint(epoch ms) |
- 이름별 `/api/players/{id}/settings/{name}` 경로가 구현돼 있습니다. 낙관적 잠금=`updated_at` vs req `base_updated_at` 이고, 비교와 쓰기는 조건부 문장 하나(`UPDATE … WHERE updated_at = base`, base 0 이면 `INSERT IGNORE`)의 `affectedRows` 로 정한다(S19, `compose/setting.ts`). 이름은 `[A-Za-z0-9_.-]{1,64}`, 사용자당 16개(`SETTING_NAMES_PER_USER_MAX`). 규칙 전문은 [api-spec.md](./api-spec.md) 9절.

### `course` / `course_chart` / `course_score`
- `course`: `course_hash` PK · name · lntype · constraint json · trophy json · extra.
- `course_chart`: course_hash + position **복합 PK** · chart_sha256(논리 chart 관계).
- `course_score`: id PK · user_id · course_hash idx · clear · judge(early/late 동일 컬럼군) · 판정 합계 `pgreat`·`great`·`good`·`bad`·`poor`·`miss` · `fast`·`slow`·`combobreak`·`empty_poor` · `max_ex_score`(이 열들은 0004, S19 2차 R2-ACC-3: 클라이언트는 코스 판정을 합계로만 보내고 분리는 0 이라, 그 전에는 판정이 모두 0 으로 저장됐다. 0004 이전 행은 복구할 수 없어 0 이다. 읽기는 분리 합이 0 이 아니면 그것, 아니면 합계) · ex_score · max_combo · gauge_value · minbp · trophy · scratch_auto bool default false · ranked · played_at · extra · `uniq(user_id, course_hash, played_at)`. `scratch_auto` 는 SCRATCH AUTO 스테이지가 낀 코스다(웨이브 20, 0002). 넓힌 판정 스테이지가 낀 코스(`assist` 에 `AUTO_SCRATCH` 밖의 태그)는 `ranked=false` 이다. flag 가 하나라도 있으면 `extra` 에 `flags`·`scratch_auto`·`assist` 를 담는다(api-spec §6).
- `course_best`: course_hash + user_id + scratch_auto 복합 PK(랭킹 가속). 0011 로 `assist_grade tinyint default 0 not null` 열이 있지만 **항상 0** 이다(코스 제출과 `course_score` 에 게이지가 없어 파생할 수 없다. 공유 비교 코드를 균일하게 하려고 열만 둔 죽은 열이고, 필요 없다고 보면 후속 마이그레이션으로 지운다). `chart_best` 와 같이 일반 판 최고와 SCRATCH AUTO 판 최고를 따로 둔다(0002).
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
- `web/drizzle.config.ts`(`schema: ['./src/server/db/schema.ts', './src/server/db/auth-schema.ts']`, `dialect:'mysql'`, `out:'./drizzle'`) → `web/drizzle/`. 현재 파일 12개(0000~0011)이고 `meta/` 도 커밋돼 있다: `0000_smooth_cerebro`(21 테이블, 저널 `when` 은 일부러 작은 2026-09-01 값) → `0001_user_folders`(폴더 두 테이블) → `0002_scratch_auto_bests`(`chart_best`·`course_best`·`course_score` 에 `scratch_auto` 열, `chart_best`·`course_best` 의 PK 에 `scratch_auto` 추가, `chart_best_rank_idx` 재생성) → `0003_replay_user_quota`(`replay.user_id` 색인 `replay_user_idx`) → `0004_course_score_judge_totals`(`course_score` 판정 합계·FAST/SLOW·콤보 끊김·空POOR·`max_ex_score` 열) → `0005_ai_foundation`, `0006_admin_seed`, `0007_chart_pattern`, `0008_ai_goal`, `0009_play_trace`, `0010_play_share_setting`(웹 AI 기능, 표는 [../web/architecture.md](../web/architecture.md) §3-2·§12~§14 가 정본) → `0011_assist_grade`(`chart_best`·`course_best` 에 `assist_grade` 열, 색인 `chart_best_rank_grade_idx`, `chart_best` 백필 UPDATE. 저널 `when` 1792606768379). 운영 DB 에는 모두 적용됐다: 0000~0002 는 2026-10-14, 0003 은 2026-10-15(`web-db.yml` run 38627474716), 0004 는 2026-10-15(run 38673724720), 0011 은 2026-10-22(run 39693988962, 마이그레이션 파일만 먼저 푸시한 `8d85b32`, 웹 코드 `64c6120` 은 그 뒤). 이 문단의 앞부분(0000~0004)은 2026-10-15 에 쓴 것이다. 적용 절차, 저널 `when` 규칙, 기준선, 운영 DB 워크플로(`web-db.yml`)는 [../web/architecture.md](../web/architecture.md) 3-2 절이 정본이다. `drizzle-kit push` 는 쓰지 않는다.

## 설계 노트 (왜)
- **best 분리 테이블**: 랭킹은 차트당 수천 history를 매번 정렬하지 않고 `chart_best`만 정렬 → P95<150ms(NFR).
- **early/late 인라인**: 레퍼런스 구현 IRScoreData를 무손실 보존(EX·BP·avgjudge·FAST/SLOW 전부 유도). 별도 judge 테이블은 조인비용↑이라 인라인.
- **md5 null 허용**: sha256만 있는 레퍼런스 구현-only 차트 수용. md5만 있는 LR2 차트는 sha256 backfill 전까지 md5 키로 동작(보조 인덱스).
- **replay 저장 경계**: 현재 DB `mediumtext` 저장으로 기능을 제공하고, 규모 확장용 외부 Blob 저장소는 후속입니다.
- **잠금 오류 재시도 (S19 4차)**: `tests/db/best-race.test.ts` 의 동시 제출이 가끔 `ER_LOCK_DEADLOCK`(1213) 으로 500 이었다. 원인은 앱의 문장 순서가 아니라 INSERT 문장 하나 안에 있다. InnoDB 는 유일 보조 색인(`score_idempotent_uniq`)에 넣기 전 중복 검사로 다음 레코드에 S 잠금(넥스트키·갭)을 잡고, 그 갭에 넣는 삽입 의도 잠금(X)을 받아야 넣는다. 같은 갭에 넣는 두 autocommit INSERT 가 서로의 S 갭 잠금을 기다려 교착한다. 그 다음 레코드가 지워졌지만 아직 purge 되지 않은 레코드(삭제 표시)일 때 잘 생긴다(`SHOW ENGINE INNODB STATUS` 의 교착 기록: 두 트랜잭션 모두 `score_idempotent_uniq` 의 같은 삭제 표시 레코드에 `lock mode S` 를 갖고 `insert intention waiting`). 문장 하나 안의 잠금이라 잠금 순서를 고정할 수 없으므로 MySQL 공식 권장(교착된 트랜잭션은 처음부터 다시 한다)을 따른다.
  - 장치: `web/src/server/lib/with-lock-retry.ts` 의 `withLockRetry`. 1213 과 잠금 대기 초과 1205 일 때만 모두 `LOCK_RETRY_ATTEMPTS`(4)번까지, `LOCK_RETRY_BASE_DELAY_MS`(10ms)에서 두 배씩 늘린 시간에 같은 크기 이내의 지터를 더해 기다린다. 오류 코드는 Drizzle 이 감싼 오류의 `cause` 를 따라 찾는다(`lib/mysql-error.ts`, 폴더의 중복 이름 검사도 같은 함수를 쓴다).
  - 적용: compose 가 ServiceDb 의 쓰기 메서드를 감싼다. 점수 경로(`chartService` 의 `upsert`, `insertScore`, `upsertChartBestIfBetter`), 코스(`upsertCourse`, `replaceCourseCharts`, `insertCourseScore`, `upsertCourseBestIfBetter`), 리플레이(`insertReplay`, `linkScoreReplay`), 폴더의 쓰기 9개. 나머지 쓰기는 테스트 부채 3.72 에 있다.
  - 멱등: 감싸는 단위는 문장 하나이거나 `db.transaction` 하나라, 진 쪽은 MySQL 이 통째로 되돌린 상태에서 다시 한다(1205 는 문장만 되돌리지만 트랜잭션은 Drizzle 이 ROLLBACK 한다). `upsertChartBestIfBetter` 는 autocommit 문장 두 개(INSERT IGNORE, 더 좋을 때만 UPDATE)지만 되풀이해도 결과가 같다. 같은 판이 두 번 저장되지 않는 것은 `score_idempotent_uniq` 가 보장한다. 제출 전체(`scoreService.submit`)는 감싸지 않는다: 문장 단위로 이미 다시 하므로 겹쳐 감쌀 이유가 없다. 다시 하면 앞 시도가 남긴 판을 멱등 키 조회가 찾는데, 그 경로가 최고 기록 갱신을 건너뛰던 문제는 4차 R4-WEB-2 에서 고쳤다(아래 "점수 행과 최고 기록 사이의 실패"). 바깥 트랜잭션 안의 문장도 감싸지 않는다.
  - 점수 INSERT 와 최고 기록 갱신을 한 트랜잭션으로 묶지 않은 이유: 묶으면 INSERT IGNORE 가 중복 행에 잡은 S 잠금이 커밋까지 남아, 같은 최고 기록 행을 UPDATE 하려는 두 트랜잭션이 서로의 S 잠금을 기다리는 교착이 새로 생긴다.
  - 테스트: `web/tests/lib/with-lock-retry.test.ts`(재시도 횟수·대상 오류), `web/tests/db/lock-retry.test.ts`(실제 MySQL 에서 엇갈려 잠그는 두 트랜잭션: 감싸지 않으면 한쪽이 1213, 감싸면 둘 다 끝남).
  - 잠금 대기 상한 (S19 4차 R4-WEB-2): 풀이 새 연결을 만들 때마다 `SET SESSION innodb_lock_wait_timeout = 5`(`LOCK_WAIT_TIMEOUT_SECONDS`, `web/src/server/db/index.ts`)를 먼저 보낸다. 서버 기본값 50초 그대로면 1205 재시도 4번이 문장 하나에 200초를 쓸 수 있고, 제출 하나가 감싼 쓰기를 여럿 거치면 Vercel 함수 상한(Fluid compute 기본값이자 Hobby 최대 300초, `vercel.json` 에 `maxDuration` 없음)을 넘길 수 있었다. 5초면 최악이 점수 제출(차트 upsert 1문장, `insertScore` 1문장, 최고 기록 2문장) 4문장 × 4회 × 5초 = 80초에 백오프 0.2초 남짓이고, 코스 제출(`insertCourseScore` 1문장, 최고 기록 2문장)은 60초다. 근거: MySQL 8.4 문서의 `innodb_lock_wait_timeout` 은 범위 Both(SESSION 가능)·Dynamic, Vercel 문서 "Configuring Maximum Duration" 의 Duration limits 표. 정상 문장은 잠금을 밀리초만 쥐므로 5초는 1205 를 새로 만들지 않는다. 설정이 실패하면(그 변수를 막는 호스팅) 서버 로그에 남기고 기본값으로 계속한다. 풀은 콜백 API(`mysql2`)로 만들고 Drizzle 이 `.promise()` 로 감싼다: `mysql2/promise` 풀도 `connection` 이벤트에 콜백 연결을 넘기지만 타입은 Promise 연결이라 맞지 않는다. 테스트: `web/tests/db/lock-retry.test.ts` 가 문장 하나와 트랜잭션의 연결 모두 5 인지 본다.
- **점수 행과 최고 기록 사이의 실패 (S19 4차 R4-WEB-2)**: 위 이유로 점수 INSERT 와 최고 기록 갱신은 따로 커밋된다. 그 사이에서 요청이 실패하면 점수만 남고, 클라이언트가 다시 보낸 같은 본문은 멱등 조회가 찾아 중복 응답이 된다. 전에는 이 경로가 최고 기록을 건드리지 않아 그 판이 `chart_best`·`course_best` 에 영영 오르지 않았다(검증자 재현: `innodb_lock_wait_timeout=1` 에서 첫 제출 1205×4 → 500, 재전송 3번 모두 `chart_best` 0행). 지금은 멱등 조회가 저장된 판의 `clear`·`ex_score`·`minbp`·`max_combo`·`ranked`·`scratch_auto` 를 함께 읽고(`findScoreByIdempotency`·`findCourseScoreByIdempotency`), 저장된 판이 ranked 면 그 값으로 조건부 갱신을 한 번 부른 뒤 순위를 센다. 갱신은 더 좋을 때만 바꾸므로 되풀이해도 같고, 이미 반영된 판에는 아무것도 하지 않는다. 현재 본문이 아니라 저장된 판의 값을 쓰는 이유: 같은 초의 재전송이 다른 값을 실어도 최고 기록은 실제로 저장된 판이어야 한다. 테스트: `web/tests/service/score-submit.test.ts`·`course.test.ts`(갱신 실패 뒤 재전송), `web/tests/db/best-repair.test.ts`(최고 기록 행을 지운 상태에서 재전송하면 그 판으로 복구되고, 다시 보내도 그대로).
