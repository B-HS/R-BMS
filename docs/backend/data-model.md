# rbms 백엔드 — 데이터 모델 (Drizzle / MySQL)

> 컨벤션 `backend.md §10`: 테이블/컬럼 **snake_case**, TS 필드 **camelCase**, 모델 타입은 `$inferSelect`/`$inferInsert`에서 유도. 시각은 `timestamp(fsp:3)` 또는 `bigint`(epoch ms). 차트 키는 md5/sha256 **둘 다 인덱싱**.

## ER 개요
```
user 1─* score *─1 chart        score *─0..1 replay
user 1─* setting_blob           score —(option/seed/gauge/judge early·late 인라인)
user *─* rival (self-ref)        course 1─* course_chart *─1 chart
user 1─* course_score *─1 course difficulty_table 1─* table_folder 1─* table_chart *─1 chart
session/account (better-auth)    difficulty_table 1─* table_course *─1 course
```

## 테이블

### `user` (better-auth 호환 + IR 프로필)
| 컬럼 | 타입 | 비고 |
|---|---|---|
| id | varchar(64) PK | 로그인 id(IRAccount.id) |
| name | varchar(64) | 표시명(IRPlayerData.name) |
| email | varchar(255) null | 선택 |
| password_hash | varchar(255) | better-auth 관리 |
| rank | varchar(32) default '' | 단위/등급(IRPlayerData.rank) |
| rank_points | double default 0 | 산출 점수 |
| total_plays | bigint default 0 | 누적(트리거/집계) |
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
| md5 | char(32) **uniq idx, null 허용** | LR2 호환 키 |
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
| user_id | varchar(64) FK→user | 제출자(guest는 null + guest_name) |
| chart_sha256 | char(64) FK→chart | 정본 차트 키 |
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
| seed | bigint | 셔플 시드(리플레이 재현) |
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
| replay_id | varchar(36) null FK→replay |
| played_at | bigint(epoch ms) idx · created_at |
| extra | json |
- 인덱스: `uniq(user_id, chart_sha256, played_at)`(멱등), `idx(chart_sha256, ranked, clear, ex_score)`(랭킹은 ranked=true만), `idx(user_id, played_at)`, `idx(client_build_sha256)`.
- EX/BP는 저장하되 `judge`에서 유도 가능(`ex=(epg+lpg)*2+egr+lgr`). 랭킹/best는 **ranked=true**만.

### `chart_best` (랭킹 가속 — (chart, user) 최고 1행)
| chart_sha256 + user_id **복합 PK** · score_id FK · clear · ex_score · minbp · max_combo · updated_at |
- best 갱신 규칙: 램프 > EX > (BP 낮을수록) 순. 랭킹 조회는 이 테이블 정렬.

### `replay` (µs 정밀)
| id varchar(36) PK(`rp_…`) · user_id FK · chart_sha256 idx · score_id null FK→score · format varchar(32) · mode/random/random_p2/seed/lntype/offset_ms/judge_rate/scratch_auto/constant/gauge(재현용) · client_build_sha256 char(64) · **event_count int · duration_us bigint** · size int · storage_key varchar(255) · created_at |
- `events`(blob)는 **오브젝트 스토리지/파일**(`storage_key`)에 저장(µs 무손실), DB엔 메타만. 상한 env `REPLAY_MAX_BYTES`(예 4MB).
- **µs 보존**: 이벤트 타임스탬프 `t_us`(i64)를 라운딩 없이 저장 → 핵분석(등간격·반응속도·동시성)·재시뮬 가능.

### `client_build` (빌드 allowlist — 변조 탐지)
| sha256 char(64) PK · version varchar(32) · platform varchar(32) · channel varchar(16)(stable/dev) · released_at bigint · trusted boolean default true · note varchar(255) · created_at |
- 릴리스 CI가 산출물 SHA-256을 자동 등록(POST /api/admin/builds). 미등록 해시 제출 → `flags:["UNKNOWN_BUILD"]`.

### `submission_audit` (제출 감사)
| id PK · score_id FK null · user_id null · client_build_sha256 char(64) idx · client_platform · ip varchar(64) · user_agent varchar(255) · accepted bool · ranked bool · flags json · created_at idx |
- 모든 제출(거부 포함) 기록 → 어뷰즈·변조 추적, 빌드 분포 분석.

### `rival` (self-ref M:N)
| user_id varchar(64) + rival_id varchar(64) **복합 PK** · created_at |

### `setting_blob` (설정 동기화)
| user_id + key(varchar(64)) **복합 PK** · format varchar(16) · content mediumtext · size int · updated_at |
- 표준 key: `settings`·`keyconfig`·`tables`. 낙관적 잠금=`updated_at` vs req `base_updated_at`.

### `course` / `course_chart` / `course_score`
- `course`: `course_hash` PK · name · lntype · constraint json · trophy json · extra.
- `course_chart`: course_hash + position **복합 PK** · chart_sha256 FK.
- `course_score`: id PK · user_id · course_hash idx · clear · judge(early/late 동일 컬럼군) · ex_score · max_combo · gauge_value · minbp · trophy · played_at · `uniq(user_id, course_hash, played_at)`.
- `course_best`: course_hash + user_id 복합 PK(랭킹 가속).

### `difficulty_table` / `table_folder` / `table_chart` / `table_course` (IRTableData)
- `difficulty_table`: id PK · name · url · extra.
- `table_folder`: id PK · table_id FK · name · position.
- `table_chart`: folder_id + chart_sha256 복합 PK.
- `table_course`: table_id + course_hash 복합 PK.

## 타입 유도 (예)
```ts
type Score = typeof score.$inferSelect
type NewScore = typeof score.$inferInsert
type ScoreRankingRow = Pick<Score, 'userId'|'clear'|'exScore'|'maxCombo'|'minbp'|'playedAt'> // 랭킹 응답 파생
type ChartKey = Pick<typeof chart.$inferSelect, 'md5'|'sha256'>
```

## 마이그레이션
- `drizzle.config.ts`(`schema:'./db/schema.ts'`, `dialect:'mysql'`) → `drizzle/`. 차트 키 교차 인덱스·best 복합 PK·멱등 유니크 키를 우선 적용.

## 설계 노트 (왜)
- **best 분리 테이블**: 랭킹은 차트당 수천 history를 매번 정렬하지 않고 `chart_best`만 정렬 → P95<150ms(NFR).
- **early/late 인라인**: 레퍼런스 구현 IRScoreData를 무손실 보존(EX·BP·avgjudge·FAST/SLOW 전부 유도). 별도 judge 테이블은 조인비용↑이라 인라인.
- **md5 null 허용**: sha256만 있는 레퍼런스 구현-only 차트 수용. md5만 있는 LR2 차트는 sha256 backfill 전까지 md5 키로 동작(보조 인덱스).
- **replay blob 외부화**: DB 비대화 방지, CDN/스토리지로 다운로드 스케일.
