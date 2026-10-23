# 2026-10-23 설계: 웹 코스 등록·단위 자동 판정·Stella 표 매일 갱신(F6)

> 최종 갱신 2026-10-23 · 대응 커밋 1169efb(dev) · 상태: 설계(구현 전). 묶음 F6(웹·서버, `web/` 아래와 `.github/workflows/web-db.yml`).
> 근거: [PROCESS.md](../PROCESS.md) "Stella·코스·단위와 클라이언트 보완 (2026-10-23)" 의 사용자 결정 1~21, [조사 보고](../reference/2026-10-23-stella-course-dan-survey.md), [HANDOFF.md](../HANDOFF.md) 5절, 클라이언트 코스 설계 [2026-10-23-course-gauge-data.md](2026-10-23-course-gauge-data.md)(이하 "F5 설계", 부록 A 가 이 문서의 입력 계약), 선곡 트리 설계 [2026-10-23-select-tree-download.md](2026-10-23-select-tree-download.md)(이하 "F3 설계", 표 id). 이 문서의 파일:줄은 모두 1169efb 의 실제 코드를 다시 열어 확인한 값이다. 조사 보고·F5 설계와 다른 곳은 2.10절에 정정으로 모았다.
> Vercel 공식 문서(2026-10-23 WebFetch): https://vercel.com/docs/cron-jobs (last_updated 2026-09-16), https://vercel.com/docs/cron-jobs/manage-cron-jobs (2026-08-11), https://vercel.com/docs/cron-jobs/usage-and-pricing (2026-07-15). 인용은 2.8절.

---

## 1. 범위와 결정 번호 대응

| 결정 | 원문 요지(PROCESS 요약) | 이 문서에서 다루는 것 | 다루지 않는 것(다른 묶음) |
| --- | --- | --- | --- |
| 6 | 코스 제출에 게이지 종류를 더해 `course_best.assist_grade` 를 채움 | 서버 DTO 의 `gauge`·`course_gauge`·`stages` 받기, `course_score` 에 저장, `deriveAssistGrade` 재사용, 구 클라이언트(필드 없음)는 0, 중복 재전송 경로의 등급 | 클라이언트 본문 생성·골든 재생성(F5 C4) |
| 18 | GENOSIDE 2018 SP 22 + DP 22 + Stella·Satellite 표 코스 | 서버 `course`·`course_chart` 에 등록하는 경로(운영 DB 쓰기 = `web-db.yml` 새 action + 스크립트, 세기 action 포함), 표 코스는 매일 갱신에서 함께 등록, md5 만 있는 차트 저장 | 데이터 파일 생성(F5 C5), 클라이언트 COURSE 탭(F5 C6) |
| 14 | Stella·단위는 출처 표기, 발광 insane1 은 흔적 제거 유지 | 출처를 담는 표 `course_source`, 코스 목록·상세·Stella 표 페이지의 출처 표기, insane1 은 바꾸지 않음 | 클라이언트 표 관리 화면(F3) |
| 19 | 서버가 코스 기록으로 단위를 자동 판정해 `user_dan` 에 넣고 마이페이지에서 표시할 단위를 고름 | `dan_course`·`user_dan`·`user_dan_display` 표, 제출 뒤 판정, 다시 판정 action, 마이페이지 "단위" 탭, 프로필·순위 표시, 공개 범위, `user.rank` 와의 관계 | — |
| 20 | 단위 인정 = 게이지가 남은 채 끝낸 판, 미러·LN 제약 준수, 어시스트·SCRATCH AUTO 제외 | 판정 함수 `judgeDanEligibility` 와 이유 코드, 게이지 모드 3종 모두 인정하되 기록, 옛 판 소급 정책 | 클라이언트의 제약 강제(F5 C3) |
| 13 | Vercel Cron(UTC 15:00)이 원본을 받아 운영 DB 에 저장, 클라이언트도 원본 직접·하루 지나면 다시 받음 | `vercel.json` crons, `CRON_SECRET` 검증 라우트, 검증(항목 수 급감·해시 형식·변경량 상한), 새 표 `table_snapshot`, 바뀌었을 때만 저장, 표 페이지·`header.json`·`data.json` 이 저장값을 읽음, 실패 시 이전 값 유지·로그, 강제 반영 action | 클라이언트 자동 다시 받기(F3) |
| 12 | Stella st 와 Satellite sl 둘 다 | 웹 표 페이지 `/tables/stella-st`·`/tables/stella-sl`(insane1 위젯 재사용), 표 목록, 레벨 기호 st0/sl0 | 클라이언트 기본 표(F3) |

결정을 바꾸는 설계는 없다. 결정이 다루지 않는 세부는 8절 "메인 결정 필요" 에 추천안·근거·대안으로 모았다.

---

## 2. 현재 코드 사실(1169efb, 다시 확인)

### 2.1 코스 DB 표(web/src/server/db/schema.ts)

1. `course`(238-246): `course_hash varchar(128) PK`, `name varchar(255)`, `lntype int`, `constraint json`, `trophy json`, `extra json`, `created_at`. 출처를 담는 열이 없다.
2. `course_chart`(248-256): `course_hash`, `position`, `chart_sha256 char(64) NOT NULL`, PK `(course_hash, position)`. **md5 열이 없다.** GENOSIDE 2018 코스는 md5 만 있다(F5 설계 3.5절 1, sha256 을 비운다).
3. `course_score`(258-303): 판정 합계·`gauge_value float`·`trophy`·`scratch_auto`·`ranked`·`played_at`·`extra json`. **게이지 종류·RANDOM·코스 게이지 모드 열이 없다.** 유일 색인 `(user_id, course_hash, played_at)`(300).
4. `course_best`(305-323): PK `(course_hash, user_id, scratch_auto)`, `assist_grade tinyint`(312, 0011 이 더함).
5. `difficulty_table`·`table_folder`·`table_chart`(325-351): `table_chart.chart_sha256 char(64) NOT NULL`(348). `table_course(table_id, course_hash)`(353-360)는 순서 열이 없다.
6. 압축 바이너리 열 선례: `const mediumBlob = customType<{ data: Buffer; driverData: Buffer }>({ dataType: () => 'mediumblob' })`(527), `chart_pattern` 이 gzip JSON 을 담는다(`web/src/server/service/shared/storage/gzip-json.ts:8,11` `gzipJsonText`·`gunzipJson`).
7. 사용자 표 `user`(`web/src/server/db/auth-schema.ts:3-24`): `rank varchar(32) default ''`(18), `rank_points double`(19). better-auth `additionalFields` 로 `input: false`(`web/src/server/lib/auth.ts:26-27`). 쓰는 코드 없음. 프로필은 `extra: { rank: row.rank }` 로 내보낸다(`web/src/server/service/domain/player/player.service.ts:19-25`).

### 2.2 코스 제출·최고 기록(DTO·서비스·조립)

1. `courseSubmissionSchema = z.object({...})`(`web/src/server/dto/course.ts:44-64`): `gauge`·`course_gauge`·`stages` 가 없다. `z.object` 라 모르는 키는 버린다(Zod `^4.5.4`, `web/package.json`).
2. 트로피 정의 스키마 `courseTrophySchema = { name, scorerate, smissrate }`(`dto/course.ts:24-28`). `smissrate` 는 beatoraja IR 와이어 이름이다(`beatoraja/src/bms/player/beatoraja/ir/IRCourseData.java:82,87`, 코스 파일·표 header 는 `missrate`, `CourseData.java:235`). 등록 때 `missrate → smissrate` 로 옮겨야 한다.
3. `course.service.ts`: `COURSE_ASSIST_GRADE = NO_ASSIST_GRADE`(143-147, 문서 주석 "A course body names no gauge"), `submit` 이 그것을 쓴다(255). `upsertMeta`(228-243)는 sha256 이 빈 차트를 **버린다**(237-240 `.filter((chart) => chart.chartSha256.length > 0)`). `toCourseMeta`(127-135)는 md5 를 `chart` 표 조인에서만 얻는다.
4. `compose/course.ts`: `findCourseCharts`(136-144)는 `chart` 를 sha256 으로 left join 해 md5 를 얻는다. 중복 제출 경로 `findCourseScoreByIdempotency`(168-183)는 `assistGrade: NO_ASSIST_GRADE` 를 붙여 돌려준다(182). `findCourse`(123-135)는 `db.select().from(course)` 로 **모든 열**을 읽는다(열을 더한 schema.ts 가 마이그레이션보다 먼저 배포되면 안 되는 이유).
5. 등급 표 `ASSIST_GRADE_OF_GAUGE`(`web/src/server/service/domain/score/assist-grade.ts:11-21`): Class 2, ExClass 3, ExHardClass 4. `deriveAssistGrade({ clear, gauge })`(32-33)는 램프가 LIGHT ASSIST(3) 일 때만 등급을 준다. `gaugeTypeToId`·`gaugeTypeFromId`(`web/src/server/dto/common.ts:36-38`), `GAUGE_TYPES`(17), `RANDOM_OPTIONS`(19).
6. 순위 정책(`web/src/server/service/domain/score/ranked-policy.ts`): `assistFlags`(33-36, SCRATCH_AUTO·ASSIST), `isScratchAutoPlay`(31), `rankBlockingFlags`(38, SCRATCH_AUTO·UNKNOWN_BUILD 는 순위를 막지 않음). 코스 서비스는 이 셋과 `buildTrustFlags` 로 `ranked` 를 정한다(`course.service.ts:256-258`). 플래그는 비어 있지 않을 때만 `course_score.extra.flags` 에 남는다(`course.service.ts:213-214`).
7. 코스 제출 라우트 `submitCourse`(`web/src/server/route/course.ts:28-95`)는 Bearer API 토큰(`withApiToken`)으로 받고, 제출 뒤 `revalidateOnCourseSubmit`(91). 정의 등록 `createCourseMetaUpsertRoute`(101-111)는 관리자 **세션**(`withAdmin`, `web/src/server/lib/with-session.ts:18-26`).
8. 관리자 **API 토큰** HOF 도 있다: `withAdminBearer`(`web/src/server/lib/with-admin-bearer.ts:14-24`), 쓰는 곳 `route/admin.ts:69-81`(곡별 패턴), `route/trace.ts:82`.

### 2.3 계약 교차 테스트

1. `web/tests/dto/client-requests.test.ts:252-255` "파싱 결과가 입력과 같다 — 서버가 조용히 바꾸거나 버리는 값이 없다": 계약 파일의 키를 서버 스키마가 버리면 `"<경로> (서버가 버림)"` 으로 **실패**한다. 코스의 허용 목록 `DROPPED_FIELDS.course` 는 비어 있다(60-66).
2. 같은 파일의 `createCourseStub`(약 158-175)이 `CourseServiceDb` 를 메모리 객체로 채운다. `CourseServiceDb` 에 메서드·필드가 늘면 이 스텁도 고쳐야 타입이 맞는다.

### 2.4 표 스냅샷(웹)

1. 표 원본은 코드가 정적 import 한 insane1 한 건이다(`web/src/server/service/domain/table-snapshot/table-snapshot.data.ts:1-3, 15-17`). `getSnapshotTables()` 는 **동기** `Map` 을 돌려준다(21-36).
2. 머리 스키마 `snapshotHeaderSchema`(`web/src/server/dto/table-snapshot.ts:9-15`)는 `level_order: z.array(z.string())`(**기본값 없음**, 필수)이고 `course` 를 버린다. 항목 스키마(17-27)는 md5 가 유효 해시여야 하고 `id`·`sha256` 을 버린다. Stella header 에는 `level_order` 가 없다(2.9절) → 지금 스키마로는 파싱 실패.
3. 서비스(`table-snapshot.service.ts`): `loadTables: () => Map<string, SnapshotTable>`(29-32, 동기). 표준 머리 `toStandardHeader`(93-99)는 다섯 필드만 낸다. `getDataEtag`(171-174)는 `source.snapshot.data_file_sha256` 앞 32자.
4. 라우트(`web/src/server/route/table-snapshot.ts`): 목록·상세·항목·기록·표준 머리·표준 데이터(53-105). 표준 형식은 `Access-Control-Allow-Origin: *`, `Cache-Control: public, max-age=3600 …`(22-31).
5. 동기 호출처(모두 `getSnapshotTables`·`getSnapshotTable`·`findSnapshotTextByMd5` 직접): `compose/index.ts:119`(차트 제목 대체), `:368`(폴더 가져오기 텍스트), `:370-376`(패턴 coverage), `:389-392`(fe 검색), `:398, 408, 416`(AI), `widgets/folder/read-folder-table-index.ts:7`, `widgets/chart/chart-detail-content.tsx:18`, `widgets/table/table-snapshot-content.tsx:16`, `widgets/table/table-catalog-content.tsx:11`. 폴더 가져오기는 `loadTableEntries: (tableId) => TableImportEntry[] | null` 동기(`web/src/server/service/domain/folder/folder.service.ts:108`, 사용 425).
6. 페이지: `/tables/insane1` 은 정적 페이지(`web/src/app/(app)/tables/insane1/page.tsx`, `metadata.other.bmstable`), `/tables/[tableId]` 는 DB 표(`difficulty_table`) 상세(`web/src/app/(app)/tables/[tableId]/page.tsx`, `readTable`). 표준 JSON 은 `web/src/app/tables/[tableId]/{header.json,data.json}/route.ts`.
7. 캐시: `next.config.ts` 에 `cacheComponents: true`. 읽기 캐시는 `'use cache'` + `cacheTag` + `revalidateTag(tag, profile)`(`web/src/server/service/read-cache.ts:2, 6-20, 65-70`).

### 2.5 환경·배포·운영 DB

1. `web/vercel.json`: `$schema`, `framework`, `regions: ["icn1"]`, `ignoreCommand`(production 이고 `web/` 이 이전 배포와 같으면 건너뜀). `crons` 없음.
2. `web/src/server/lib/env.ts:6-27` 의 `envSchema` 에 `CRON_SECRET` 없음. `getEnv()`(98-102).
3. `.github/workflows/web-db.yml`: `workflow_dispatch` 의 `action` choice 11개(12행), DB URL 은 시크릿 `WEB_DATABASE_URL`(30), SQL 은 `bun scripts/run-sql.ts <file>.sql`, 세기 → 바꾸기 → 다시 세기 꼴(65-100). 러너에서 `bun install --frozen-lockfile` 후 `web/` 에서 돈다.
4. 스크립트는 `server-only` 를 import 하면 안 된다: `web/src/server/db/index.ts:1` 과 `compose/index.ts:1` 이 `import 'server-only'` 다. `run-sql.ts`·`db-migration-status.ts` 는 `../src/server/lib/env` 만 가져와 `mysql2/promise` 로 직접 접속한다. 테스트는 `bunfig.toml` 의 `preload = ["./tests/lib/preload.ts"]` 로 돈다(러너 스크립트 실행에는 preload 가 없다).
5. 마이그레이션 운영(`docs/web/architecture.md` 3-2절): 산출물 `drizzle/*.sql`·`drizzle/meta/` 커밋, 추가만(additive) 규칙, **두 번 푸시**(마이그레이션 파일만 먼저 → `status` → `migrate` → `status` → 코드). 저널 `when` 은 `db:generate` 가 붙인다. 마지막은 `0011_assist_grade`(`web/drizzle/meta/_journal.json`).
6. 로그 경로: `console.error(label, summarizeError(error))`(`web/src/server/lib/with-summarized-errors.ts:21`, `compose/trace.ts:286`).

### 2.6 마이페이지·프로필·순위

1. 설정 탭 `SETTINGS_TABS = [profile, tokens, sync, rivals, ai]`(`web/src/widgets/settings/settings-tabs.tsx:18, 27-31`). 단위 탭 없음.
2. 내비게이션 `NAV_ITEMS`(`web/src/shared/constants/navigation.ts:9-17`)에 코스 항목 없음. 코스 화면은 상세 한 쪽(`web/src/app/(app)/courses/[courseHash]/page.tsx`)뿐이고 목록이 없다. 상세 패널의 차트 링크는 `/charts/${row.sha256}`(`web/src/widgets/course/course-detail-panel.tsx:36`) — sha256 이 빈 차트는 링크가 깨진다.
3. 순위 행 `LeaderboardRow = PlayerRow & {...}`(`web/src/server/service/domain/fe/fe.service.ts:31`), 목록 `listPlayerLeaderboard`(44).
4. 클라이언트는 `/players/{id}` 프로필을 읽는다(`crates/rbms-ir/src/http.rs:184`), `PlayerProfile.extra` 는 `HashMap<String, Value>`(`crates/rbms-ir/src/dto/account.rs:14`) → `extra` 에 키를 더해도 읽기가 깨지지 않는다.
5. 쿼리 키 `QUERY_KEY`(`web/src/shared/constants/query-key.ts`)의 `COURSE` 는 ALL·META·RANKING·BEST 넷. 엔티티 파일은 평평한 `web/src/entities/<entity>.query.ts`(예 `course.query.ts`).

### 2.7 코스 id 규칙(다시 계산해 확인)

`Course::hash()` = SHA-256( 이름 UTF-8 + `\n` + 차트마다 (sha256 이 있으면 sha256, 없으면 md5) + `\n` ) 의 소문자 hex(`crates/rbms-course/src/model.rs:304-313`, `hash_key` 229-231). 이 문서 작성 중 Python 으로 같은 규칙을 계산했다(테스트 벡터, 7절·6절에서 씀):

| 코스 | 차트(md5, 소문자) | course_hash |
| --- | --- | --- |
| `GENOSIDE 2018 段位認定 初段` | `ae11930223a8f24e91633f1a7864e1eb`, `cb661ee05c67990012bfd12703ba656c`, `328dbf235f3b6cb2791e71bfb62951f8`, `7c6ca04aa14d07b2c081f4dd35432844` | `ae8ddca1ddec7737a393eddbd3862d2be2e42bb7184365fbc4664efe280ed0d1`(F5 설계 부록 A 와 같다) |
| `Stella Skill Simulator 4th st0` | `349bc491ec40d5595412637d8a4c8d2e`, `baee0a1921fc5041b44d7d87c7b5548d`, `72b1ce4b2051bd2a396dfa11a2d785ee`, `3a1661a3eaafa13f976e1010d5b87ca0` | `afce60f187e4ba4ab0519b54d9a0fcf04184e6b137c4826c4513be2d4f99f158` |
| `Satellite Skill Analyzer 2nd sl0` | `9ee5524e4ed8a3e7033b4de1fedc35fd`, `9498378a847b94b949d11a566a86d3c0`, `4023045d9134dfc47e953bc430e5ef07`, `bb5d0716c18b37f9687bbcea17fc646c` | `b327c69e3462aa0a0e07d60d0fe0b9e6873a8bd0be3336c1408901c842a59967` |

제약 토큰(`model.rs:91-110`): `grade, grade_mirror, grade_random, no_speed, no_good, no_great, gauge_lr2, gauge_5k, gauge_7k, gauge_9k, gauge_24k, ln, cn, hcn`, 기준 구현 이름(`CLASS`, `MIRROR`, …, `GAUGE_5KEYS`, `LN`)도 대소문자 무시로 읽는다(`from_any`, 152-156). 그룹(`group`, 116-128): 0 grade 3종, 1 no_speed, 2 no_good·no_great, 3 gauge 5종, 4 ln·cn·hcn. 같은 그룹은 처음 것만 남긴다(`collapse_constraints`, 330-340). LN 종류 번호 0 LN, 1 CN, 2 HCN(`apps/rbms-player/src/course_ir.rs:119-129`).

### 2.8 Vercel Cron(공식 문서 원문 인용, 2026-10-23)

- "To trigger a cron job, Vercel makes an HTTP GET request to your project's production deployment URL, using the `path` provided in your project's `vercel.json` file." / "Vercel Functions triggered by a cron job on Vercel will always contain `vercel-cron/1.0` as the user agent." / "The timezone is always UTC" (cron-jobs).
- "It is possible to secure your cron job invocations by adding an environment variable called `CRON_SECRET` to your Vercel project. We recommend using a random string of at least 16 characters … The value of the variable will be automatically sent as an `Authorization` header when Vercel invokes your cron job." / "The `authorization` header will have the `Bearer` prefix for the value." 예제: `if (!cronSecret || authHeader !== \`Bearer ${cronSecret}\`) return new Response('Unauthorized', { status: 401 })` (manage-cron-jobs).
- "Vercel will not retry an invocation if a cron job fails." / "Cron job delivery is best effort … Cron delivery can also occasionally invoke the same scheduled run more than once … Design your operations to be idempotent" / "Cron jobs do not follow redirects." / "when cron jobs respond with a redirect or a cached response, they will not be shown in the logs." / 기간 제한은 Functions 와 같다(`maxDuration`) (manage-cron-jobs).
- 요금제(usage-and-pricing): Hobby·Pro·Enterprise 모두 프로젝트당 100개. Hobby 는 "Once per day", 정밀도 "Per-hour (±59 min)" — "a cron job configured as `0 1 * * *` … will trigger anywhere between 1:00 am and 1:59 am". Pro 는 분 단위. "Cron jobs are included in all plans", 비용은 함수 사용량.
- 결론: `0 15 * * *`(KST 0시)는 Hobby 에서도 배포된다. Hobby 면 실제 실행은 KST 00:00~00:59 사이. 요금제는 저장소에서 확인되지 않는다(사용자가 대시보드에서 확인, 4.2절).

### 2.9 외부 원천(조사 사본, 다시 읽음)

1. Stella `header.json`(사본 SHA-256 `951f0558a27817f88ccac02e383db529158ff5613d4553cfeb683c06120bbb4c`, 13,430 B): 키 `name="Stella"`, `symbol="st"`, `data_url="score.json"`, `course`(배열의 배열, 1묶음 26개). `level_order`·`last_update` 없음. 코스 26개 모두 제약 `grade_mirror, gauge_lr2, ln`, 트로피 silvermedal·goldmedal, md5 4개(`stMuscle`·`stMarathon` 은 5개). 이름: `Stella Skill Simulator 4th st0`~`st12` 13개 + `STELLA技能訓練 …` 13개.
2. Satellite `sl_header.json`(SHA-256 `1fd0a3bab625f6528ffedc15f897031359c4117209deb9b2def19d6c4393161c`, 6,761 B): `name="Satellite"`, `symbol="sl"`, `data_url="score.json"`, 코스 13개 `Satellite Skill Analyzer 2nd sl0`~`sl12`.
3. `score.json`(st 840,194 B, 2,350항목, SHA-256 `1a0fe417…f7e9`; sl 869,678 B, 2,497항목, `94955…3da4`): 키 `id, md5, sha256, title, artist, url, url_diff, level`, level 은 문자열 숫자, md5 중복 0, sha256 은 모두 64자.
4. HTTP: `cache-control public, max-age=60`, ETag·Last-Modified 없음, CORS 헤더 없음, 응답 서버 Cloudflare(조사 A-1). → 서버 쪽 fetch 와 내용 해시 비교가 필요하다.

### 2.10 조사 보고·F5 설계 정정

| 위치 | 적힌 내용 | 실제(1169efb) |
| --- | --- | --- |
| 조사 6절 "스케줄러가 쓸 수 있는 서버 토큰 경로는 없습니다", "관리용 HTTP 경로는 … 두 개뿐" | 세션 경로 둘뿐 | 관리자 API 토큰 HOF `withAdminBearer`(`lib/with-admin-bearer.ts:14-24`)와 그 라우트(`route/admin.ts:69-81` 패턴, `route/trace.ts:82`)가 있다. 이 설계는 그 경로를 쓰지 않는다(Cron 은 `CRON_SECRET`, 등록은 `web-db.yml`) |
| 조사 4절 "스냅샷 머리 스키마 web/src/server/dto/table-snapshot.ts:8-13", "항목 스키마 dto:15-25" | 줄 번호 | 머리 9-15, 항목 17-27 |
| 조사 6절 "web/src/server/lib/env.ts:6-23" | 줄 번호 | 6-27(AI 키 두 줄이 더해졌다) |
| 조사 8절 "POST /api/courses/meta (route/course.ts:100-110)" | 줄 번호 | 101-111 |
| 조사 8절 "course.service.ts submit 255-345행 부근" | 줄 번호 | 245-337 |
| F5 설계 2.4절 7 "`courseSubmissionSchema` (`web/src/server/dto/course.ts:44-66`)" | 줄 번호 | 44-64 |
| F5 설계 6.3절 4 "구 DTO 는 새 키를 버려 통과해야 한다" | 웹 교차 검사가 통과 | **실패한다.** `client-requests.test.ts:252-255` 는 서버가 버린 키를 실패로 본다(2.3절). 골든 재생성(F5 C4)은 이 문서의 W2(DTO)와 같은 푸시이거나 그 뒤여야 한다(5.2절) |
| 조사 8절 "코스 정의 등록 … 관리자 세션만" | — | 맞다. 다만 `upsertMeta` 가 sha256 이 빈 차트를 버려(2.2절 3) md5 만 있는 GENOSIDE 코스는 지금 경로로 등록해도 차트가 0개가 된다 |

---

## 3. 설계

### 3.1 전체 흐름

1. **코스 제출(결정 6·19·20)**: 클라이언트 본문의 새 필드(`gauge`·`course_gauge`·`stages`)를 받아 `course_score` 의 새 열에 저장하고, `course_best.assist_grade` 를 `deriveAssistGrade` 로 채운다. 제출이 끝난 뒤 같은 요청 안에서 단위 판정(후처리)을 돌려 `user_dan` 에 넣는다. 판정 실패(예외)는 제출을 실패시키지 않는다.
2. **코스 등록(결정 18·14)**: GENOSIDE 2018 44개는 저장소 데이터 파일(F5 C5 산출)을 `web-db.yml` 의 새 action 이 읽어 등록한다(세기 → 승인 → 등록). Stella·Satellite 표 코스는 매일 갱신(3)이 header 가 바뀔 때 함께 등록한다. 둘 다 같은 서비스 `courseCatalogService.syncGroup` 를 쓴다.
3. **표 매일 갱신(결정 13·12)**: Vercel Cron 이 KST 0시에 `GET /api/cron/tables` 를 부르고, 서버가 stellabms.xyz 의 st·sl `header.json`·`score.json` 을 받아 검증한 뒤 바뀌었을 때만 `table_snapshot` 에 저장한다. 표 페이지와 표준 JSON 은 insane1 은 커밋 스냅샷, st·sl 은 저장값을 읽는다.
4. **단위 표시(결정 19)**: 마이페이지 "단위" 탭에서 체계별로 표시할 단위를 고르고(자동 = 가장 높은 단위 / 숨김 / 특정 단위), 프로필·순위에 고른 단위가 보인다.

새 npm 의존성은 없다(`node:crypto`, `node:zlib`, 전역 `fetch`, 기존 `zod`·`drizzle-orm`). Rust 코드는 이 묶음에서 바꾸지 않는다.

### 3.2 마이그레이션 0012(하나로 묶음, 모두 추가만)

파일: `web/drizzle/0012_course_dan_tables.sql`, `web/drizzle/meta/0012_snapshot.json`, `web/drizzle/meta/_journal.json`(항목 추가). 생성: `cd web && bunx drizzle-kit generate --name course_dan_tables`(schema.ts 를 먼저 고친 뒤). 기존 열의 타입·NULL 여부를 바꾸지 않고, 기존 행을 고치는 수동 SQL 도 없다. 하나로 묶는 이유: 모두 같은 코드 푸시가 기대하고, 옛 코드는 새 열·새 표를 읽지도 쓰지도 않아 적용 순서 위험이 같다(8절 결정 18).

**Drizzle 정의(schema.ts 에 추가, 기존 `mediumBlob` 재사용)**

```ts
export const courseScore = mysqlTable('course_score', {
    ...
    gauge: tinyint('gauge'),
    courseGauge: varchar('course_gauge', { length: 16 }),
    stages: json('stages').$type<StoredCourseStage[]>(),
})

export const courseChart = mysqlTable('course_chart', {
    ...
    md5: char('md5', { length: 32 }).default('').notNull(),
    title: varchar('title', { length: 255 }).default('').notNull(),
})

export const courseSource = mysqlTable('course_source', {
    id: varchar('id', { length: 64 }).primaryKey(),
    kind: varchar('kind', { length: 16 }).notNull(),
    name: varchar('name', { length: 255 }).default('').notNull(),
    maker: varchar('maker', { length: 255 }).default('').notNull(),
    pageUrl: varchar('page_url', { length: 512 }).default('').notNull(),
    fileUrl: varchar('file_url', { length: 512 }).default('').notNull(),
    retrievedAt: bigint('retrieved_at', { mode: 'number' }).default(0).notNull(),
    fileSha256: char('file_sha256', { length: 64 }).default('').notNull(),
    detail: json('detail'),
    updatedAt: timestamp('updated_at', { fsp: 3 }).defaultNow().$onUpdate(() => new Date()).notNull(),
})

export const courseGroup = mysqlTable('course_group', {
    id: varchar('id', { length: 64 }).primaryKey(),
    sourceId: varchar('source_id', { length: 64 }).notNull(),
    name: varchar('name', { length: 255 }).default('').notNull(),
    keys: int('keys').default(0).notNull(),
    position: int('position').default(0).notNull(),
})

export const courseGroupMember = mysqlTable(
    'course_group_member',
    {
        groupId: varchar('group_id', { length: 64 }).notNull(),
        courseHash: varchar('course_hash', { length: 128 }).notNull(),
        position: int('position').notNull(),
        active: boolean('active').default(true).notNull(),
    },
    (table) => [primaryKey({ columns: [table.groupId, table.courseHash] }), index('course_group_member_course_idx').on(table.courseHash)],
)

export const danCourse = mysqlTable(
    'dan_course',
    {
        courseHash: varchar('course_hash', { length: 128 }).primaryKey(),
        system: varchar('system', { length: 32 }).notNull(),
        label: varchar('label', { length: 64 }).notNull(),
        rank: int('rank').notNull(),
    },
    (table) => [index('dan_course_system_idx').on(table.system, table.rank)],
)

export const userDan = mysqlTable(
    'user_dan',
    {
        userId: varchar('user_id', { length: 36 }).notNull().references(() => user.id, { onDelete: 'cascade' }),
        system: varchar('system', { length: 32 }).notNull(),
        label: varchar('label', { length: 64 }).notNull(),
        rank: int('rank').notNull(),
        courseHash: varchar('course_hash', { length: 128 }).notNull(),
        courseScoreId: varchar('course_score_id', { length: 36 }).notNull(),
        courseGauge: varchar('course_gauge', { length: 16 }).notNull(),
        gauge: tinyint('gauge').notNull(),
        acquiredAt: bigint('acquired_at', { mode: 'number' }).notNull(),
        createdAt: timestamp('created_at', { fsp: 3 }).defaultNow().notNull(),
    },
    (table) => [primaryKey({ columns: [table.userId, table.system, table.label] })],
)

export const userDanDisplay = mysqlTable(
    'user_dan_display',
    {
        userId: varchar('user_id', { length: 36 }).notNull().references(() => user.id, { onDelete: 'cascade' }),
        system: varchar('system', { length: 32 }).notNull(),
        label: varchar('label', { length: 64 }),
        updatedAt: timestamp('updated_at', { fsp: 3 }).defaultNow().$onUpdate(() => new Date()).notNull(),
    },
    (table) => [primaryKey({ columns: [table.userId, table.system] })],
)

export const tableSnapshot = mysqlTable('table_snapshot', {
    tableId: varchar('table_id', { length: 64 }).primaryKey(),
    headerUrl: varchar('header_url', { length: 512 }).notNull(),
    dataUrl: varchar('data_url', { length: 512 }).notNull(),
    headerSha256: char('header_sha256', { length: 64 }).notNull(),
    dataSha256: char('data_sha256', { length: 64 }).notNull(),
    headerGz: mediumBlob('header_gz').notNull(),
    dataGz: mediumBlob('data_gz').notNull(),
    entryCount: int('entry_count').notNull(),
    levelCount: int('level_count').notNull(),
    fetchedAt: bigint('fetched_at', { mode: 'number' }).notNull(),
    checkedAt: bigint('checked_at', { mode: 'number' }).notNull(),
    lastOutcome: varchar('last_outcome', { length: 64 }).default('').notNull(),
})
```

뜻:

- `course_score.gauge`: `gaugeTypeToId(gauge)`(0~8), **NULL = 옛 클라이언트(게이지를 모름)**. `course_gauge`: `'LR2' | 'BEATORAJA' | 'IIDX'` 또는 NULL. `stages`: 서버가 본문의 `stages[i]` 와 `charts[i]` 를 짝지어 만든 `StoredCourseStage[]`(3.3절), 본문에 `stages` 가 없으면 NULL.
- `course_chart.md5`·`title`: 코스 **정의의** md5 와 표시용 제목. md5 만 있는 차트는 `chart_sha256 = ''` 로 둔다(열이 `NOT NULL` 이라 빈 문자열은 들어간다. 타입·NULL 여부를 바꾸지 않아 추가만 규칙을 지킨다, 8절 결정 16). **정의에 없는 sha256 을 채우지 않는다**(코스 id 가 sha256 우선이라 IR 코스 정의를 읽는 쪽이 다른 id 를 만든다, F5 설계 3.7절 4).
- `course_source`: 출처 한 건(결정 14). `kind` = `'bundled' | 'table'`. `retrieved_at` = 받은 시각(ms, UTC). `detail` = 출처별 나머지(GENOSIDE 의 `entry_name`·`entry_sha256`·`entry_hash_prefix`·`entry_type`·`constraint_sources`·`title_source`, 표의 `data_url`·`data_sha256`).
- `course_group`·`course_group_member`: 화면 묶음과 순서. 표에서 사라진 코스는 `active=false`(기록·순위는 남는다).
- `dan_course`: "이 코스를 인정 조건대로 끝내면 체계 `system` 의 단위 `label`(순서 `rank`)을 준다". 등록 때 3.5절 규칙으로 만든다.
- `user_dan`: 획득 단위. PK `(user_id, system, label)` — **처음 획득만 남긴다**(INSERT IGNORE). `acquired_at` = 그 판의 `played_at`(ms), `created_at` = 서버가 판정한 시각.
- `user_dan_display`: 체계별 표시 선택. **행 없음 = 자동(가장 높은 `rank`)**, `label NULL` = 숨김, 값 = 그 단위.
- `table_snapshot`: 원본에서 받은 바이트를 gzip 해 그대로 보관(표준 `data.json` 을 원본과 같은 바이트로 낸다). 해시는 받은 바이트의 SHA-256. `fetched_at` = 이 내용을 처음 받은 시각, `checked_at` = 마지막으로 확인한 시각, `last_outcome` = 마지막 확인의 결과 코드(3.6절 표).

`AssistGrade` 를 schema.ts 에서 import 하는 선례(20행)처럼 `StoredCourseStage` 는 `@server/dto/course` 에서 type import 한다.

### 3.3 코스 제출: 게이지·스테이지·등급(결정 6, W2)

**DTO(`web/src/server/dto/course.ts`)**

```ts
export const COURSE_GAUGE_RULES = ['LR2', 'BEATORAJA', 'IIDX'] as const
export type CourseGaugeRule = (typeof COURSE_GAUGE_RULES)[number]
export const courseGaugeRuleSchema = z.enum(COURSE_GAUGE_RULES)

export const courseStagePlaySchema = z.object({
    gauge: gaugeTypeSchema,
    random: randomOptionSchema,
    lntype: z.number().int(),
})

export const courseSubmissionSchema = z.object({
    ...기존 필드,
    gauge: gaugeTypeSchema.nullish().default(null),
    course_gauge: courseGaugeRuleSchema.nullish().default(null),
    stages: z.array(courseStagePlaySchema).max(COURSE_CHARTS_MAX).default([]),
})

export type CourseStagePlayInput = z.infer<typeof courseStagePlaySchema>

export type StoredCourseStage = CourseStagePlayInput & { md5: string; sha256: string }
```

- `lntype` 은 범위를 좁히지 않는다(0~2 밖의 값으로 판 전체를 400 으로 버리지 않는다. 기록 원칙). 판정에서만 쓴다.
- `gauge`·`random` 은 단일 곡 제출과 같은 스키마(`gaugeTypeSchema`, `randomOptionSchema`)를 쓴다. 각 스테이지의 단일 곡 제출이 이미 같은 스키마로 받아진 값이라 새 거절 사유가 생기지 않는다.
- 와이어 이름: F5 설계 3.4절과 부록 A 그대로(`"Class"`, `"LR2"`, `{ "gauge": "Class", "random": "Off", "lntype": 0 }`).

**서비스(`course.service.ts`)**

1. `COURSE_ASSIST_GRADE` 상수와 그 문서 주석(143-147)을 지우고 다음을 더한다.

```ts
/** The assist grade of a course run: none when the body names no gauge (a client from before 2026-10-23). */
export const courseAssistGrade = (run: { clear: number; gauge: number | null }): AssistGrade =>
    run.gauge === null ? NO_ASSIST_GRADE : deriveAssistGrade({ clear: run.clear, gauge: run.gauge })

/** The stages a body played, each with the chart the course lists at that place. */
export const toStoredStages = (input: Pick<CourseSubmissionInput, 'stages' | 'charts'>): StoredCourseStage[] | null =>
    input.stages.length === 0
        ? null
        : input.stages.map((stage, index) => ({ ...stage, md5: input.charts[index]?.md5 ?? '', sha256: input.charts[index]?.sha256 ?? '' }))
```

2. `CourseScoreInsertRow` 에 `gauge: number | null`, `courseGauge: CourseGaugeRule | null`, `stages: StoredCourseStage[] | null`. `buildCourseScoreRow` 가 `gauge: input.gauge === null ? null : gaugeTypeToId(input.gauge)`, `courseGauge: input.course_gauge`, `stages: toStoredStages(input)` 를 채운다.
3. `submit` 의 `assistGrade` = `courseAssistGrade({ clear, gauge: row.gauge })`.
4. `CourseServiceDb.findCourseScoreByIdempotency` 반환에 `gauge: number | null` 을 더하고(`StoredRunForBest & { gauge: number | null }`), 조립(`compose/course.ts:168-183`)이 `courseScore.gauge` 를 읽어 `assistGrade: courseAssistGrade(row)` 를 붙인다(182행의 `NO_ASSIST_GRADE` 고정을 대체). 같은 판의 재전송은 처음 판의 저장값으로 등급을 다시 만든다(점수 쪽 `compose/index.ts:185,196` 과 같은 방식).
5. `submit` 반환에 `scoreRow: { id, userId, courseHash, clear, ranked, scratchAuto, flags, gauge, courseGauge, gaugeValue, stages, playedAt }` 를 더한다(중복이면 저장된 행에서). 단위 판정(3.5절)이 이 값을 받는다. IR 응답(`response`)은 바꾸지 않는다.
6. 옛 행(gauge NULL)은 손대지 않는다. 0012 이전 `course_best.assist_grade` 는 모두 0 이고 옛 판에는 게이지 정보가 없어 백필할 값이 없다(백필 action 없음).

**순위 의미**: `course_best` 의 순서식(`isRankedAheadOf`, `best-scope.ts`)은 이미 `assist_grade` 를 쓴다. 지금까지 모든 코스 행이 0 이었으므로 SCRATCH AUTO 코스 클리어끼리의 순서가 이제 게이지 등급을 따른다(4절 E3).

### 3.4 코스 등록과 출처(결정 18·14, W3)

**순수 함수(`web/src/server/service/domain/course-catalog/`)**

```ts
export const courseHashOf = (course: { name: string; charts: { md5: string; sha256: string }[] }) => string
```
`course-hash.ts`. `createHash('sha256')` 에 이름, `\n`, 차트마다 (sha256 이 비어 있지 않으면 sha256, 아니면 md5) + `\n` 을 넣고 hex. 입력은 이미 소문자로 정규화된 값을 받는다(정규화는 아래 변환기가 한다).

```ts
export type CatalogChart = { md5: string; sha256: string; title: string }
export type CatalogDan = { system: DanSystemId; label: string; rank: number }
export type CatalogCourse = { courseHash: string; name: string; lntype: number; constraint: string[]; trophy: CourseTrophy[]; charts: CatalogChart[]; dan: CatalogDan | null }
export type CatalogSource = { id: string; kind: 'bundled' | 'table'; name: string; maker: string; pageUrl: string; fileUrl: string; retrievedAt: number; fileSha256: string; detail: Record<string, unknown> }
export type CatalogGroup = { id: string; sourceId: string; name: string; keys: number; position: number; courses: CatalogCourse[] }
export type CatalogDefinition = { source: CatalogSource; groups: CatalogGroup[] }
```
`course-catalog.type.ts`. `CourseTrophy = z.infer<typeof courseTrophySchema>`(손으로 다시 적지 않는다).

- `constraint-tokens.ts`: `canonicalConstraints(raw: string[]) => string[]` — 2.7절의 14 토큰과 기준 구현 이름을 대소문자 무시로 읽어 소문자 토큰으로, 모르는 것은 버리고, 그룹마다 처음 것만 남겨 그룹 순서로(Rust `collapse_constraints` 와 같은 결과). `lntypeOfConstraints(tokens) => number`(ln 0, cn 1, hcn 2, 없으면 0).
- `bundled-courses.ts`: `bundledCoursesFileSchema`(Zod, F5 설계 3.5절 모양: `schema: z.literal(1)`, `source{maker, page_url, file_url, retrieved_on, file_bytes, file_sha256, entry_name, entry_sha256, entry_hash_prefix, entry_type, constraint_sources[], title_source}`, `groups[{id, name, keys, courses[{course_hash, name, charts[{md5, sha256?: default '', title}], constraint, trophy[{name, missrate, scorerate}], release}]}]`). `bundledCatalog(file: BundledCoursesFile) => CatalogDefinition`:
  - `source.id = 'genoside-2018'`, `kind 'bundled'`, `name 'GENOSIDE 2018 段位認定'`, `maker = file.source.maker`, `pageUrl = page_url`, `fileUrl = file_url`, `retrievedAt = Date.parse(`${retrieved_on}T00:00:00Z`)`, `fileSha256 = file_sha256`, `detail` = 나머지 필드.
  - 묶음 id·이름·`keys` 는 파일 그대로, `position` 은 파일 순서(SP 0, DP 1).
  - 코스마다 `courseHashOf` 로 다시 계산해 `course_hash` 와 **다르면 던진다**(Rust 와 TS 규칙이 갈라진 것을 등록 전에 잡는다). md5·sha256 은 소문자로. `trophy` 는 `{ name, scorerate, smissrate: missrate }`. `constraint` 는 `canonicalConstraints`. `dan` 은 3.5절 `danOfCourse(groupId, name, index)`.
- `table-courses.ts`: `tableCatalog(params: { table: TableSourceDefinition; headerCourse: unknown; entries: SnapshotEntry[]; headerSha256: string; dataUrl: string; fetchedAt: number }) => { definition: CatalogDefinition; skipped: number }`. Rust `courses_from_table_header`(F5 설계 3.6절)와 같은 규칙: 입력이 배열의 배열·평평한 배열·객체 하나 모두 읽고 펴서 순서대로, `md5`(문자열 배열 또는 문자열 하나)·`sha256`(같은 꼴) 또는 `charts` 객체 배열을 차트로, 소문자로, 차트가 0개이거나 두 해시가 모두 빈 차트가 있으면 건너뛰고 센다. 제목은 `entries` 의 md5 → title(없으면 ''). `source.id = table.id`, `kind 'table'`, `name = table.sourceName`, `maker = table.maker`, `pageUrl = table.pageUrl`, `fileUrl = table.headerUrl`, `retrievedAt = fetchedAt`, `fileSha256 = headerSha256`, `detail = { data_url, data_sha256 }`. 묶음은 하나(`id = table.id`, `name = header.name`, `keys = table.keys`, `position = table.groupPosition`).

**서비스(`course-catalog.service.ts`)**

```ts
export type CourseCatalogServiceDb = {
    findGroupMembers: (groupId: string) => Promise<{ courseHash: string; position: number; active: boolean }[]>
    findCourses: (courseHashes: string[]) => Promise<{ courseHash: string; name: string; lntype: number; constraint: unknown; trophy: unknown; charts: CatalogChart[] }[]>
    applyGroup: (params: { source: CatalogSource; group: Omit<CatalogGroup, 'courses'>; courses: CatalogCourse[]; deactivate: string[] }) => Promise<void>
    listGroups: () => Promise<CatalogGroupRow[]>
    findEntry: (courseHash: string) => Promise<CatalogEntryRow | null>
}
export type CourseCatalogServiceDeps = { db: CourseCatalogServiceDb }
export const createCourseCatalogService = (deps: CourseCatalogServiceDeps) => ({
    planSync: async (definition: CatalogDefinition) => CatalogSyncPlan,
    applySync: async (definition: CatalogDefinition) => CatalogSyncPlan,
    listGroups: async () => CourseCatalogOutput,
    getEntry: async (courseHash: string) => CourseCatalogEntryOutput | null,
})
export type CourseCatalogService = ReturnType<typeof createCourseCatalogService>
```

- `CatalogSyncPlan = { groups: { id: string; inserted: number; changed: number; unchanged: number; deactivated: number; danCourses: number; newDanHashes: string[] }[] }`. "changed" = 이름·제약·트로피·차트(md5·sha256·제목)·lntype 중 하나라도 다름.
- `applyGroup`(compose) 은 **한 트랜잭션**: `course_source` upsert → `course_group` upsert → 코스마다 `course` upsert(`extra = { source_id }`), `course_chart` 교체(`position`, `chart_sha256` = 정의의 sha256 또는 '', `md5`, `title`), `course_group_member` upsert(`active=true`, `position`), `dan_course` upsert(dan 이 있을 때, 없으면 그 코스의 행 삭제) → `deactivate` 의 멤버를 `active=false`. 반복 실행해도 같은 결과(멱등).
- `planSync` 는 쓰지 않고 같은 계획만 만든다(세기 action).
- `newDanHashes` = 이번 동기화로 처음 `dan_course` 가 생긴 코스. 호출자가 단위 소급(3.5절 7)에 넘긴다.

**기존 코스 정의 경로 수리**: `upsertMeta`(관리자 세션)는 sha256 이 빈 차트를 버리지 않고 `md5` 와 함께 저장하게 고친다(`chartSha256: chart.sha256`, `md5: chart.md5`, 둘 다 빈 차트만 버림). `CourseServiceDb.replaceCourseCharts` 의 인자에 `md5`·`title` 을 더한다(`title` 은 이 경로에서 ''). `findCourseCharts` 는 `md5: courseChart.md5 || chart.md5`(`sql\`COALESCE(NULLIF(${courseChart.md5}, ''), ${chart.md5})\`` 또는 조립 쪽에서 합침)와 `title` 을 돌려준다. `toCourseMeta` 의 출력(IR 와이어)은 `{ md5, sha256 }` 그대로(제목은 IR 에 넣지 않는다).

**GENOSIDE 등록 스크립트(`web/scripts/seed-courses.ts`)**

```
bun scripts/seed-courses.ts count   # 읽기만: 계획 숫자 출력, 종료 0
bun scripts/seed-courses.ts apply   # 계획 → 적용 → 다시 계획(바뀔 것 0 이어야 종료 0)
```

1. `web/data/courses/genoside-2018/courses.json` 을 읽어 `bundledCoursesFileSchema` → `bundledCatalog`(해시 불일치면 종료 1).
2. DB 는 `web/scripts/lib/script-db.ts` 의 `createScriptDb()`(새 파일: `parseEnv(process.env)` → `resolveDatabaseUrl` → `mysql2` 풀 → `drizzle(pool, { schema: { ...authSchema, ...domainSchema }, mode: 'default' })`, `LOCK_WAIT_TIMEOUT_SECONDS` 는 값만 같게 상수로) 로 만든다. `@server/db`·`@server/compose`(server-only)를 값으로 import 하지 않고 `composeCourseCatalog(db)`(새 `web/src/server/compose/course-catalog.ts`, server-only 를 import 하지 않음)를 부른다.
3. 출력은 숫자만(묶음 id, inserted/changed/unchanged/deactivated/danCourses). 행 내용·DB 주소는 출력하지 않는다(Actions 로그는 공개, `run-sql.ts:38-39` 와 같은 원칙).
4. 종료 코드: `{ OK: 0, FAILED: 1, USAGE: 2 }`(`run-sql.ts:8` 과 같은 꼴).

**web-db.yml action 추가**: `courses-seed-count`(세기만), `courses-seed`(세기 → 적용 → 다시 세기). 4.2절.

### 3.5 단위 자동 판정(결정 19·20, W4)

> 구현과 다름: 체계와 규칙은 `dan-rules.ts` 가 아니라 `web/src/server/service/domain/course-catalog/dan-of-course.ts` 에 있다([ADR](../acknowledge/2026-10-23-adr-dan-auto-award.md)).

**체계와 규칙(`web/src/server/service/domain/dan/dan-rules.ts`)**

```ts
export const DAN_SYSTEM_IDS = ['genoside2018_sp', 'genoside2018_dp', 'stella', 'satellite'] as const
export type DanSystemId = (typeof DAN_SYSTEM_IDS)[number]

export const DAN_SYSTEMS = [
    { id: 'genoside2018_sp', name: 'GENOSIDE 2018 SP', groupId: 'genoside-2018-sp' },
    { id: 'genoside2018_dp', name: 'GENOSIDE 2018 DP', groupId: 'genoside-2018-dp' },
    { id: 'stella', name: 'Stella', groupId: 'stella-st' },
    { id: 'satellite', name: 'Satellite', groupId: 'stella-sl' },
] as const satisfies readonly { id: DanSystemId; name: string; groupId: string }[]

export const danOfCourse = (groupId: string, courseName: string, indexInGroup: number) => CatalogDan | null
```

| 체계 | 대상 코스 | label | rank |
| --- | --- | --- | --- |
| `genoside2018_sp` | 묶음 `genoside-2018-sp` 에서 이름이 `GENOSIDE 2018 段位認定 ` 로 시작 | 접두사 뒤 글자(`初段` … `十段`, `発狂初段` … `発狂皆伝`, `Overjoy`) | 묶음 안 순서 + 1(1~22, 初段 1, Overjoy 22) |
| `genoside2018_dp` | 묶음 `genoside-2018-dp`, 접두사 `GENOSIDE 2018 DP段位認定 ` | 같은 방식 | 같은 방식 |
| `stella` | 묶음 `stella-st`, 이름이 `/^Stella Skill Simulator \S+ st(\d+)$/` | `st{n}` | n(0~12) |
| `satellite` | 묶음 `stella-sl`, 이름이 `/^Satellite Skill Analyzer \S+ sl(\d+)$/` | `sl{n}` | n |

`STELLA技能訓練 …` 13개는 코스로 등록하고 순위도 있지만 단위를 주지 않는다(8절 결정 7). 판(版)이 바뀌어 `… 5th st5` 가 생기면 새 코스 id 도 `st5` 를 준다(`\S+`). 같은 label 은 PK 로 한 번만 남는다.

**인정 판정(`dan-eligibility.ts`, 순수 함수)**

```ts
export const DAN_INELIGIBLE_REASON = {
    NOT_A_DAN_COURSE: 'not_a_dan_course',
    GUEST: 'guest',
    UNRANKED: 'unranked',
    NOT_CLEARED: 'not_cleared',
    SCRATCH_AUTO: 'scratch_auto',
    ASSISTED: 'assisted',
    NO_STAGE_FACTS: 'no_stage_facts',
    NOT_A_COURSE_GAUGE: 'not_a_course_gauge',
    STAGE_COUNT: 'stage_count',
    CHART_MISMATCH: 'chart_mismatch',
    RANDOM_NOT_ALLOWED: 'random_not_allowed',
    LN_MODE_MISMATCH: 'ln_mode_mismatch',
} as const
export type DanIneligibleReason = (typeof DAN_INELIGIBLE_REASON)[keyof typeof DAN_INELIGIBLE_REASON]

export type DanRunFacts = {
    userId: string | null
    clear: number
    ranked: boolean
    scratchAuto: boolean
    flags: string[]
    gauge: number | null
    courseGauge: CourseGaugeRule | null
    gaugeValue: number
    stages: StoredCourseStage[] | null
}
export type DanCourseFacts = { dan: CatalogDan | null; constraint: string[]; charts: { md5: string; sha256: string }[] }

export const judgeDanEligibility = (run: DanRunFacts, course: DanCourseFacts) =>
    ({ eligible: true } as const) | ({ eligible: false; reason: DanIneligibleReason } as const)
```

위에서 아래로 처음 걸리는 이유를 돌려준다(테스트·로그용, 저장하지 않음):

1. `course.dan === null` → `NOT_A_DAN_COURSE`.
2. `run.userId === null` → `GUEST`.
3. `!run.ranked` → `UNRANKED`(넓힌 판정 등 `rankBlockingFlags`, 신뢰하지 않는 빌드. 8절 결정 8).
4. 램프: `clear` 가 `NoPlay`·`Failed`·`AssistEasy`(ASSIST)·`LightAssistEasy`(LIGHT ASSIST) 중 하나이거나 `gaugeValue <= 0` → `NOT_CLEARED`. "게이지가 남은 채 끝남"(결정 20). 코스 게이지 클리어는 NORMAL(Class)·HARD(ExClass)·EX-HARD(ExHardClass) 이상 램프다(`gauge.rs:146-155`). ASSIST·LIGHT ASSIST 램프는 어시스트로 강등된 클리어라 여기서 걸린다.
5. `run.scratchAuto` 또는 `flags` 에 `SCRATCH_AUTO` → `SCRATCH_AUTO`(결정 20).
6. `flags` 에 `ASSIST` → `ASSISTED`(결정 20).
7. `stages === null` 또는 `courseGauge === null` 또는 `gauge === null` → `NO_STAGE_FACTS`(옛 클라이언트: 제약 준수를 알 수 없다).
8. `gauge` 와 모든 `stages[i].gauge` 가 `Class`·`ExClass`·`ExHardClass` 가 아니면 → `NOT_A_COURSE_GAUGE`. 게이지 **모드**(LR2/BEATORAJA/IIDX)는 제한하지 않는다(결정 20 "모두 인정하되 기록").
9. `stages.length !== course.charts.length` → `STAGE_COUNT`(클리어한 코스는 모든 스테이지를 친다).
10. 스테이지마다 코스 차트와 같은 차트인지: 코스 차트에 md5 가 있으면 md5(소문자 비교), 없으면 sha256 → 아니면 `CHART_MISMATCH`(코스 id 는 클라이언트가 붙이므로, 다른 곡으로 친 판이 단위 코스 id 를 달고 오는 것을 막는다).
11. grade 제약(그룹 0): `grade` → 모든 스테이지 `random === 'Off'`; `grade_mirror` → `Off` 또는 `Mirror`; `grade_random` 또는 없음 → 제한 없음. 어기면 `RANDOM_NOT_ALLOWED`(원본 `MusicSelector.java:483-491` 의 강제와 같은 집합).
12. LN 제약(그룹 4): `ln`/`cn`/`hcn` → 모든 스테이지 `lntype` 이 0/1/2. 어기면 `LN_MODE_MISMATCH`. 제약이 없으면 제한 없음.

`no_speed`·`no_good`·`no_great`·`gauge_*` 는 서버가 확인하지 않는다(본문에 사실이 없거나, 결정 21 로 게이지 표 제약이 설정에 밀린다).

**서비스(`dan.service.ts`)**

```ts
export type DanServiceDb = {
    findDanCourse: (courseHash: string) => Promise<(DanCourseFacts & { courseHash: string }) | null>
    findDanCourses: (courseHashes: string[]) => Promise<(DanCourseFacts & { courseHash: string })[]>
    findCourseScore: (courseScoreId: string) => Promise<(DanRunFacts & { id: string; courseHash: string; playedAt: number }) | null>
    listCourseScoresWithStages: (params: { courseHashes: string[]; afterId: string | null; limit: number }) => Promise<(DanRunFacts & { id: string; courseHash: string; playedAt: number })[]>
    insertUserDanIgnore: (row: UserDanRow) => Promise<boolean>
    listUserDans: (userId: string) => Promise<UserDanRow[]>
    listDisplays: (userIds: string[]) => Promise<{ userId: string; system: string; label: string | null }[]>
    listUserDansFor: (userIds: string[]) => Promise<UserDanRow[]>
    upsertDisplay: (row: { userId: string; system: DanSystemId; label: string | null }) => Promise<void>
    deleteDisplay: (params: { userId: string; system: DanSystemId }) => Promise<void>
}
export const createDanService = (deps: { db: DanServiceDb; now: () => number }) => ({
    awardFromRun: async (run: DanRunFacts & { id: string; courseHash: string; playedAt: number }) => DanAwardResult,
    awardFromCourseScore: async (courseScoreId: string) => DanAwardResult,
    awardForCourses: async (params: { courseHashes: string[]; dryRun: boolean }) => { scanned: number; awarded: number },
    listMine: async (userId: string) => MyDansOutput,
    setDisplay: async (userId: string, input: DanDisplayInput) => MyDansOutput,
    listDisplayed: async (userIds: string[]) => Map<string, DisplayedDan[]>,
})
export type DanService = ReturnType<typeof createDanService>
```

- `DanAwardResult = { awarded: CatalogDan & { system: DanSystemId } | null; reason: DanIneligibleReason | null }`.
- `UserDanRow = typeof userDan.$inferSelect` 에서 `createdAt` 을 뺀 `Omit<..., 'createdAt'>`(손으로 다시 적지 않는다).
- `awardFromRun`: `findDanCourse` → `judgeDanEligibility` → 인정이면 `insertUserDanIgnore({ userId, system, label, rank, courseHash, courseScoreId: run.id, courseGauge, gauge, acquiredAt: run.playedAt })`. 이미 있으면 바꾸지 않는다(처음 획득 유지).
- **제출 경로(W4 가 `route/course.ts` 를 고침)**: `courseService.submit` 이 끝나면(중복이든 아니든) `danService.awardFromRun(result.scoreRow)` 을 `try/catch` 로 부르고, 실패는 `console.error('dan award failed', summarizeError(error))` 만 남긴다(제출 응답은 그대로). 획득했으면 `revalidateOnDanAward({ loginId })`(새 `read-cache.ts` 함수: 플레이어 프로필·순위 태그). 같은 요청 안의 **후처리**이고 같은 트랜잭션이 아니다(코스 서비스는 트랜잭션 없이 쓰고, 판정 실패가 판 저장을 막으면 안 된다. 8절 결정 24).
- `awardForCourses`(소급): `course_score` 중 `course_hash IN (...)` 이고 `stages IS NOT NULL` 이고 `user_id IS NOT NULL` 인 행을 id 순으로 페이지(`DAN_BACKFILL_PAGE = 500`)마다 읽어 `awardFromRun`. `dryRun` 이면 인정 수만 센다. 표 갱신(3.6절)이 `newDanHashes` 로 부르고, `web-db.yml` 의 `dan-recompute-count`/`dan-recompute`(스크립트 `web/scripts/dan-recompute.ts count|apply`, 대상 = 모든 `dan_course`)이 부른다.
- **옛 판 소급(8절 결정 11)**: `stages` 가 NULL 인 판(0012 이전 판, 옛 클라이언트 판)은 판정하지 않는다. 미러·LN 제약 준수를 알 수 없고, 어느 곡으로 쳤는지도 저장돼 있지 않다.
- `listMine`: 4체계 모두, 체계마다 `acquired`(rank 오름차순, 코스 이름 포함) 와 `display`(`{ mode: 'auto' | 'hidden' | 'label'; label: string | null }`) 와 `shown`(실제 보이는 label 또는 null).
- `setDisplay(userId, { system, mode, label })`: `auto` → 행 삭제, `hidden` → `label NULL`, `label` → 그 label 을 그 사용자가 획득했을 때만 저장(아니면 `createAppError(ERROR_CODE.VALIDATION_ERROR, { field: 'label' })`).
- `listDisplayed(userIds)`: 사용자별 `[{ system, name, label }]`(체계 순서 `DAN_SYSTEMS`), 자동이면 가장 높은 rank, 숨김이면 빠짐.

**공개 범위(8절 결정 13)**: 다른 사람에게는 고른(또는 자동) 단위만 보인다 — 플레이어 프로필(`extra.dans`), 순위 목록 행(`dans`). 획득 목록 전체·획득 판 id·획득 시각·게이지 모드는 본인 마이페이지에서만.

**`user.rank` 와의 관계(8절 결정 12)**: 쓰지 않는다. 단일 varchar(32) 라 4체계를 담지 못하고, 자동 판정 값과 사용자의 표시 선택을 따로 두는 것이 이 설계의 핵심이다. 프로필의 `extra.rank`(빈 문자열)는 와이어 호환을 위해 그대로 둔다. `rank_points` 문제(HANDOFF 6절 10)는 범위 밖.

**프로필·순위 출력**

- `PlayerProfileOutput.extra` 에 `dans: DisplayedDan[]` 를 더한다(`player.service.ts` 의 `toPlayerProfile` 에 인자 추가, `createPlayerService` 의 deps 에 `listDisplayedDans: (userId: string) => Promise<DisplayedDan[]>`). 클라이언트는 `extra` 를 `HashMap` 으로 읽어 깨지지 않는다(2.6절 4).
- `DisplayedDan = { system: DanSystemId; name: string; label: string }`(`dto/dan.ts` 의 Zod 스키마에서 `z.infer`).
- fe 순위 행에 `dans: DisplayedDan[]`: `createFeService` deps 에 `listDisplayedDans: (userIds: string[]) => Promise<Map<string, DisplayedDan[]>>` 를 더하고 `playerLeaderboard` 가 한 번 묶어 부른다(N+1 없음).

**DTO(`web/src/server/dto/dan.ts`)**

```ts
export const danSystemIdSchema = z.enum(DAN_SYSTEM_IDS)
export const danDisplayInputSchema = z.discriminatedUnion('mode', [
    z.object({ system: danSystemIdSchema, mode: z.literal('auto') }),
    z.object({ system: danSystemIdSchema, mode: z.literal('hidden') }),
    z.object({ system: danSystemIdSchema, mode: z.literal('label'), label: z.string().min(1).max(DAN_LABEL_MAX_LENGTH) }),
])
export const displayedDanSchema = z.object({ system: danSystemIdSchema, name: z.string(), label: z.string() })
export const myDansOutputSchema = z.object({ systems: z.array(z.object({ system: danSystemIdSchema, name: z.string(), display: ..., shown: z.string().nullable(), acquired: z.array(...) })) })
export type DanDisplayInput = z.infer<typeof danDisplayInputSchema>
```
`DAN_LABEL_MAX_LENGTH = 64`(열 길이와 같다). `DAN_SYSTEM_IDS` 는 `dan-rules.ts` 가 아니라 이 DTO 파일에 두고 규칙 파일이 import 한다(DTO → service 방향).

**라우트·앱 경로**

| 경로 | 인증 | 내용 |
| --- | --- | --- |
| `GET /api/fe/me/dans` | 세션(`withSession`) | `successResponse(listMine)` |
| `PUT /api/fe/me/dans/display` | 세션 | `parseJsonBody(danDisplayInputSchema)` → `setDisplay` → `revalidateOnDanAward({ loginId })` |
| `GET /api/fe/courses` | 공개(`withRateLimit('fe-read', RATE_LIMIT.PUBLIC_READ)`) | 코스 묶음 목록(출처 포함) |
| `GET /api/fe/courses/[courseHash]/catalog` | 공개 | 한 코스의 묶음·출처·단위·차트 제목 |

`web/src/server/route/dan.ts`(`createMyDansRoute`, `createMyDanDisplayRoute`), `web/src/server/route/course-catalog.ts`(`createCourseCatalogListRoute`, `createCourseCatalogEntryRoute`). 앱 파일 `web/src/app/api/fe/me/dans/route.ts`, `web/src/app/api/fe/me/dans/display/route.ts`, `web/src/app/api/fe/courses/route.ts`, `web/src/app/api/fe/courses/[courseHash]/catalog/route.ts`.

### 3.6 표 매일 갱신(결정 13, W5)

> 구현과 다름: 검증 상수는 비율이 아니라 정수 퍼센트다(`TABLE_MIN_KEPT_PERCENT` 90, `TABLE_MAX_CHANGE_PERCENT` 25. 값은 같다). 사유 코드에 `course_sync_failed`·`unexpected_error` 가 더해졌다([ADR](../acknowledge/2026-10-23-adr-table-daily-refresh.md)).

**표 정의(`web/src/server/service/domain/table-refresh/table-sources.ts`)**

```ts
export type TableSourceDefinition = {
    id: 'stella-st' | 'stella-sl'
    expectedName: string
    symbol: string
    sourceName: string
    maker: string
    pageUrl: string
    headerUrl: string
    allowedHost: string
    keys: number
    groupPosition: number
}
export const REMOTE_TABLES = [
    { id: 'stella-st', expectedName: 'Stella', symbol: 'st', sourceName: 'Stella', maker: 'stellabms.xyz', pageUrl: 'https://stellabms.xyz/st/table.html', headerUrl: 'https://stellabms.xyz/st/header.json', allowedHost: 'stellabms.xyz', keys: 7, groupPosition: 2 },
    { id: 'stella-sl', expectedName: 'Satellite', symbol: 'sl', sourceName: 'Satellite', maker: 'stellabms.xyz', pageUrl: 'https://stellabms.xyz/sl/table.html', headerUrl: 'https://stellabms.xyz/sl/header.json', allowedHost: 'stellabms.xyz', keys: 7, groupPosition: 3 },
] as const satisfies readonly TableSourceDefinition[]
```
표 id 는 클라이언트 기본 표 id(F3 설계 `STELLA_TABLE_ID = "stella-st"`, `SATELLITE_TABLE_ID = "stella-sl"`)와 같게 한다(8절 결정 1).

**받기(`web/src/server/service/shared/http/fetch-limited.ts`, 새 파일)**

```ts
export type FetchedBytes = { ok: true; status: number; bytes: Uint8Array; contentType: string } | { ok: false; reason: 'network' | 'timeout' | 'too_large' | 'redirect' }
export const fetchLimited = async (url: string, limits: { maxBytes: number; timeoutMs: number; userAgent: string }) => FetchedBytes
```
`fetch(url, { redirect: 'manual', signal: AbortSignal.timeout(timeoutMs), headers: { 'user-agent': userAgent, accept: 'application/json' }, cache: 'no-store' })`, 3xx → `redirect`, 본문은 `response.body` 를 읽으며 `maxBytes` 를 넘으면 멈추고 `too_large`. 상수: `TABLE_HEADER_MAX_BYTES = 1 * 1024 * 1024`, `TABLE_DATA_MAX_BYTES = 8 * 1024 * 1024`, `TABLE_FETCH_TIMEOUT_MS = 20_000`, `TABLE_FETCH_USER_AGENT = 'R-BMS table sync (+https://bms.hyuns.uk)'`.

**검증(`table-refresh.validate.ts`, 순수 함수)**

```ts
export const TABLE_REFRESH_OUTCOME = { SAVED: 'saved', UNCHANGED: 'unchanged', REJECTED: 'rejected', FAILED: 'failed' } as const
export const TABLE_REFRESH_REASON = {
    FETCH_FAILED: 'fetch_failed', HTTP_STATUS: 'http_status', TOO_LARGE: 'too_large', NOT_JSON: 'not_json',
    HEADER_INVALID: 'header_invalid', NAME_MISMATCH: 'name_mismatch', SYMBOL_MISMATCH: 'symbol_mismatch', DATA_URL_OFF_SITE: 'data_url_off_site',
    ENTRIES_INVALID: 'entries_invalid', DUPLICATE_MD5: 'duplicate_md5', TOO_FEW_ENTRIES: 'too_few_entries', SHRANK: 'shrank', CHANGED_TOO_MUCH: 'changed_too_much',
} as const
export const validateRemoteHeader = (definition: TableSourceDefinition, raw: unknown) => { ok: true; header: RemoteHeader; dataUrl: string } | { ok: false; reason }
export const validateRemoteEntries = (raw: unknown) => { ok: true; entries: RemoteEntry[] } | { ok: false; reason }
export const diffEntries = (previous: RemoteEntry[], next: RemoteEntry[]) => { added: number; removed: number; releveled: number }
export const judgeChange = (params: { previousCount: number | null; next: RemoteEntry[]; diff: Diff | null; allowLargeChange: boolean }) => { ok: true } | { ok: false; reason }
export const deriveLevelOrder = (entries: { level: string }[]) => string[]
```

| 검사 | 규칙 | 상수 |
| --- | --- | --- |
| 받기 | 200 이 아니면 `HTTP_STATUS`, 실패 `FETCH_FAILED`/`TOO_LARGE`, 3xx 는 `FETCH_FAILED`(원본에 다른 호스트로 가는 리다이렉트를 따르지 않는다) | 위 크기·시간 |
| JSON | UTF-8 BOM 제거 뒤 `JSON.parse` 실패면 `NOT_JSON` | — |
| 머리 | `remoteHeaderSchema = z.object({ name: z.string().min(1), symbol: z.string(), data_url: z.string().min(1), level_order: z.array(z.string()).optional(), last_update: z.string().optional(), course: z.unknown().optional() })` 실패 `HEADER_INVALID`; `name !== expectedName` `NAME_MISMATCH`; `symbol !== symbol` `SYMBOL_MISMATCH` | — |
| data_url | `new URL(data_url, headerUrl)` 가 `https:` 이고 호스트가 `allowedHost` 가 아니면 `DATA_URL_OFF_SITE` | — |
| 항목 | `remoteEntrySchema = z.object({ md5: z.string().regex(/^[0-9a-fA-F]{32}$/), sha256: z.string().regex(/^([0-9a-fA-F]{64})?$/).default(''), level: z.string().min(1).max(MAX_LEVEL_TEXT), title: z.string().default(''), artist: z.string().default(''), url: z.string().default(''), url_diff: z.string().default(''), name_diff: z.string().default('') })` 의 배열이 아니면 `ENTRIES_INVALID`(한 항목이라도). md5 는 소문자로. `level` 은 숫자 문자열(`0`)이든 숫자(`0`)든 글자로 강제(`z.union([z.string(), z.number()]).transform(String)`) | `MAX_LEVEL_TEXT = 32` |
| 중복 | 같은 md5 가 둘 이상이면 `DUPLICATE_MD5` | — |
| 첫 저장 | 저장된 행이 없을 때 항목 수 `< TABLE_FIRST_SNAPSHOT_MIN_ENTRIES` 면 `TOO_FEW_ENTRIES` | 500 |
| 급감 | 이전 수 `p`, 새 수 `n`: `n < p * TABLE_MIN_KEPT_RATIO` 면 `SHRANK` | 0.9 |
| 변경량 | `added + removed + releveled > p * TABLE_MAX_CHANGE_RATIO` 면 `CHANGED_TOO_MUCH` | 0.25 |
| 강제 | `allowLargeChange` 면 급감·변경량 검사만 건너뛴다(형식 검사는 그대로) | — |

근거: Stella 정식 표는 2주마다 일요일에 반영된다(조사 A-2). 두 표의 크기(2,350/2,497)에 비해 한 번 반영의 변경은 수십 곡 규모로 보이며(관찰 1회분이라 확정 아님, 9절 R5), 10% 급감·25% 변경은 정상 반영을 막지 않고 빈 표·잘린 응답·다른 표로 바뀐 응답을 막는다.

`deriveLevelOrder`: `level_order` 가 있으면 그것, 없으면 항목의 레벨을 중복 없이 모아 "모두 정수 글자면 수 오름차순, 아니면 처음 나온 순서" 로(클라이언트 `crates/rbms-table/src/lib.rs:194-207` 의 숫자 순 정렬과 같은 결과).

**저장소(조립: `web/src/server/compose/table-refresh.ts`, `TableSnapshotStoreDb`)**

```ts
export type RemoteSnapshotVersion = { tableId: string; headerSha256: string; dataSha256: string; fetchedAt: number; checkedAt: number; lastOutcome: string }
export type RemoteSnapshotRow = RemoteSnapshotVersion & { headerUrl: string; dataUrl: string; headerBytes: Uint8Array; dataBytes: Uint8Array; entryCount: number; levelCount: number }
export type TableSnapshotStoreDb = {
    listVersions: () => Promise<RemoteSnapshotVersion[]>
    loadSnapshot: (tableId: string) => Promise<RemoteSnapshotRow | null>
    saveSnapshot: (row: RemoteSnapshotRow, previousDataSha256: string | null) => Promise<boolean>
    markChecked: (params: { tableId: string; checkedAt: number; outcome: string }) => Promise<void>
}
```
- `saveSnapshot` 은 gzip(`gzipJsonText` 는 문자열 → 같은 `node:zlib` 로 바이트 그대로 압축하는 `gzipBytes`/`gunzipBytes` 를 `gzip-json.ts` 에 더한다)해 쓴다. `previousDataSha256 === null` 이면 `INSERT … ON DUPLICATE KEY UPDATE` 없이 `insert().ignore()`, 아니면 `UPDATE … WHERE table_id = ? AND data_sha256 = previous`. 영향 행 0 = 다른 실행이 먼저 썼다 → `false`(그 실행의 결과가 남는다). 같은 예약이 두 번 와도 결과가 같다(2.8절 "idempotent").
- 바뀌지 않았으면(`header_sha256`·`data_sha256` 둘 다 같음) `markChecked` 만.
- 실패·거절이면 행이 있을 때만 `markChecked({ outcome: 'rejected:<reason>' | 'failed:<reason>' })`, 저장된 내용은 그대로(결정 13 "실패 시 이전 값 유지"), `console.error('table refresh rejected', { tableId, outcome, reason, ... 숫자만 })`. 성공·변경 없음은 `console.info('table refresh', { tableId, outcome, entries, added, removed, releveled })`.

**서비스(`table-refresh.service.ts`)**

```ts
export type TableRefreshReport = {
    tableId: string
    outcome: (typeof TABLE_REFRESH_OUTCOME)[keyof typeof TABLE_REFRESH_OUTCOME]
    reason: (typeof TABLE_REFRESH_REASON)[keyof typeof TABLE_REFRESH_REASON] | null
    entries: number | null
    added: number | null
    removed: number | null
    releveled: number | null
    courses: CatalogSyncPlan['groups'][number] | null
    awarded: number | null
}
export const createTableRefreshService = (deps: {
    tables: readonly TableSourceDefinition[]
    fetchBytes: typeof fetchLimited
    store: TableSnapshotStoreDb
    courseCatalog: Pick<CourseCatalogService, 'planSync' | 'applySync'>
    dan: Pick<DanService, 'awardForCourses'>
    now: () => number
}) => ({
    refreshAll: async (options: { allowLargeChange: boolean; dryRun: boolean }) => TableRefreshReport[],
})
```
표마다 차례로(동시에 하지 않는다 — 원본 서버 부담, 2.9절 Cloudflare): 머리 받기 → 검증 → data 받기 → 검증 → 이전 저장값과 비교 → 저장(또는 확인만) → **머리가 바뀌었거나 처음 저장이면** `tableCatalog(...)` 로 코스 정의를 만들어 `applySync` → `newDanHashes` 가 있으면 `awardForCourses({ courseHashes: newDanHashes, dryRun: false })`. `dryRun` 이면 쓰지 않고 `planSync` 만. 한 표의 실패는 다른 표를 막지 않는다. 표 코스는 같은 갱신에서 등록한다(8절 결정 5).

**Cron 라우트**

- `web/vercel.json` 에 `"crons": [{ "path": "/api/cron/tables", "schedule": "0 15 * * *" }]` 를 더한다(나머지 키 그대로).
- `web/src/server/lib/env.ts` 의 `envSchema` 에 `CRON_SECRET: z.string().min(CRON_SECRET_MIN_LENGTH).optional()`, `CRON_SECRET_MIN_LENGTH = 16`(Vercel 권장 "at least 16 characters").
- `web/src/server/lib/with-cron-secret.ts`(새 HOF):

```ts
export const withCronSecret =
    (deps: { readSecret: () => string | undefined }) =>
    <TContext>(handler: (request: Request, context: TContext) => Promise<Response>) =>
    async (request: Request, context: TContext) => {
        const secret = deps.readSecret()
        if (!secret) throw createAppError(ERROR_CODE.SERVICE_NOT_CONFIGURED, { setting: 'CRON_SECRET' })
        const expected = Buffer.from(`Bearer ${secret}`)
        const given = Buffer.from(request.headers.get('authorization') ?? '')
        if (given.length !== expected.length || !timingSafeEqual(given, expected)) throw createAppError(ERROR_CODE.UNAUTHORIZED)
        return handler(request, context)
    }
```
  공식 예제의 문자열 비교 대신 `node:crypto` `timingSafeEqual`(길이를 먼저 맞춘다). 비밀 값은 응답·로그에 넣지 않는다.
- `web/src/server/route/cron.ts`: `createTableRefreshCronRoute = (resolveDeps = compose) => withErrorHandling(withCronSecret({ readSecret: () => getEnv().CRON_SECRET })(async () => { const reports = await resolveDeps().tableRefreshService.refreshAll({ allowLargeChange: false, dryRun: false }); revalidateOnTableRefresh(reports); return NextResponse.json(successResponse({ tables: reports }), { headers: { 'Cache-Control': 'private, no-store' } }) }))`.
- `web/src/app/api/cron/tables/route.ts`: `export const GET = createTableRefreshCronRoute()`, `export const maxDuration = TABLE_REFRESH_MAX_DURATION_SECONDS`(60. 두 표 x 약 0.9 MB 두 번 받기와 코스 동기화에 충분, 2.8절 "duration limits … identical to … Functions").
- `revalidateOnTableRefresh`(read-cache.ts): 저장된 표가 있으면 `CACHE_TAG_EXT.courseCatalog()` 와 단위를 준 사용자의 플레이어 태그를 무효화한다.

**운영 스크립트(`web/scripts/refresh-tables.ts`)**: `count`(받고 검증·비교만, 쓰지 않음 = `dryRun`), `apply-large`(급감·변경량 검사를 건너뛰고 저장 = `allowLargeChange: true`). 원본이 정말로 크게 바뀌어 Cron 이 계속 거절할 때 사람이 `count` 숫자를 보고 승인 뒤 `apply-large` 를 돌린다. 러너(GitHub Actions)가 stellabms.xyz 로 나간다. `web-db.yml` action `tables-refresh-count`, `tables-refresh-force`.

**표 읽기(결정 12·13): 표 스냅샷 서비스를 비동기로**

1. `SnapshotTable` 타입(`table-snapshot.data.ts`)에 `attribution: TableAttribution | null`, `course: unknown | null`, `standardData: () => string`(표준 data.json 본문) 을 더한다. insane1 은 `attribution: null`, `course: null`(지금과 같은 출력), `standardData` = 지금의 `JSON.stringify(rawEntries)`.

```ts
export type TableAttribution = { sourceName: string; maker: string; pageUrl: string; fetchedAt: number; checkedAt: number; dataSha256: string }
```
2. `snapshotHeaderSchema.level_order` 에 `.default([])`(insane1 은 값이 있어 그대로).
3. 조립(`compose/table-snapshot.ts`)에 원격 표 메모리 캐시: `createRemoteSnapshotCache(store: TableSnapshotStoreDb)` → `load(): Promise<SnapshotTable[]>`. 매 호출 `listVersions()`(작은 행)만 읽고, `data_sha256`·`header_sha256` 이 기억한 것과 같으면 기억한 파싱 결과를 쓰고, 다르면 그 표만 `loadSnapshot` → 압축 풀기 → `remoteEntrySchema` 로 다시 파싱(저장 때 검증했지만 경계에서 다시 좁힌다) → `SnapshotTable`(`header: { name, symbol, data_url: '', level_order: deriveLevelOrder, last_update: new Date(fetchedAt).toISOString().slice(0, 10) }`, `entries`, `standardData: () => 저장된 원본 바이트의 UTF-8`, `source` 는 `snapshotSourceSchema` 모양으로 행에서, `attribution`). 서버리스 인스턴스마다 한 번씩만 압축을 푼다. 표가 0행이면 빈 배열.
4. `TableSnapshotServiceDeps.loadTables: () => Promise<Map<string, SnapshotTable>>`(번들 insane1 + 원격). 서비스 메서드 `list`·`getDetail`·`getStandardHeader`·`getStandardDataText`·`getDataEtag`·`searchEntries`·`loadEntriesForImport` 를 `async` 로. `getRecords` 는 이미 async.
5. `toStandardHeader` 는 `course !== null` 일 때 `course` 를 그대로 더한다(Stella 코스를 우리 사본을 등록한 클라이언트도 받게). insane1 출력은 바이트까지 같다.
6. `getDataEtag` 는 원격 표면 `data_sha256` 앞 32자.
7. `TableSummaryOutput`(`tableSummaryOutputSchema`)에 `source: z.object({ name, maker, page_url, fetched_at, checked_at, data_sha256 }).nullable()`. insane1 은 `null`(흔적 없음 유지, 2026-10-14 ADR "현재 상태").
8. 라우트(`route/table-snapshot.ts`)는 `await` 를 더한다(모양은 그대로).
9. 폴더 가져오기: `FolderServiceDeps.loadTableEntries` 를 `(tableId) => Promise<TableImportEntry[] | null>` 로, `folder.service.ts:425` 에서 `await`. Stella 표도 MY FOLDERS 로 가져올 수 있게 된다.
10. **바꾸지 않는 동기 호출처(번들 insane1 만 계속 본다, 9절 T3)**: 차트 제목 대체(`compose/index.ts:119, 368, 391`), fe 검색(392), 패턴 coverage(370-376), AI(398, 408, 416), `read-folder-table-index.ts`, `chart-detail-content.tsx`.

### 3.7 화면(W7·W8)

**표(W7, 결정 12·13·14)**

1. 새 페이지 `web/src/app/(app)/tables/stella-st/page.tsx`, `web/src/app/(app)/tables/stella-sl/page.tsx`: insane1 페이지와 같은 꼴(`buildPageMetadata({ title: 'Stella', … path: '/tables/stella-st', index: false })`, `other: { bmstable: '/tables/stella-st/header.json' }`), 본문은 `<Suspense fallback={<ShellPageLoading title='Stella' />}><TableSnapshotContent tableId='stella-st' /></Suspense>`. 정적 세그먼트가 `[tableId]` 보다 먼저 잡힌다.
2. `TableSnapshotContent`(`web/src/widgets/table/table-snapshot-content.tsx`)를 `async` 서버 컴포넌트로 바꾸고 `await compose().tableSnapshotService.getDetail(tableId)` 로 읽는다(insane1 페이지는 그대로 이 컴포넌트를 쓴다. insane1 경로는 DB 를 읽지 않으므로 지금처럼 정적으로 남는지 빌드 출력으로 확인, 6.4절). 레벨 범위 글은 지금 코드(22행 `${table.symbol}${level}`)가 그대로 `st0 ~ st12` 를 만든다.
3. 출처 표기: 새 `web/src/features/table/table-source-attribution.tsx`(`FC<{ source: NonNullable<TableSummaryOutput['source']> }>`, 데이터는 props 로만): "출처 Stella(stellabms.xyz) · 원본 표 [링크](`page_url`, 새 탭, 기존 `features/table/table-external-link.tsx` 재사용) · 내용 받은 날 YYYY-MM-DD HH:mm(KST, `Intl.DateTimeFormat('ko-KR', { timeZone: 'Asia/Seoul' })`) · 마지막 확인 … · SHA-256 앞 12자". `TableSnapshotContent` 가 `detail.table.source` 가 있을 때만 그린다(insane1 은 null 이라 그리지 않는다).
4. 표 목록(`table-catalog-content.tsx`): `async` 로 바꿔 `await compose().tableSnapshotService.list()` 로 기본 표(insane1 + 저장된 st·sl)를 그린다. 원격 표가 아직 없으면 그 줄이 없다. Suspense 는 페이지(`web/src/app/(app)/tables/page.tsx`)에서 감싼다. 표 카드(`table-snapshot-list-item.tsx`)는 `source` 가 있으면 "출처 Stella" 한 줄을 더한다.

**코스·단위(W8, 결정 14·19)**

1. 새 코스 목록 `web/src/app/(app)/courses/page.tsx` + `web/src/widgets/course/course-catalog-content.tsx`(서버: `getQueryClient` + `prefetchQuery({ ...courseCatalogQueryOptions(), queryFn: () => readCourseCatalog() })` + `HydrationBoundary`) + `web/src/widgets/course/course-catalog-panel.tsx`(클라이언트, `useCourseCatalog()`): 묶음마다 제목, 출처 블록, 코스 줄(이름 → `/courses/{hash}` 링크, 스테이지 수, 단위 label 배지, 비활성이면 "표에서 빠짐").
2. 출처 블록 `web/src/features/course/course-source-notice.tsx`(props 만): GENOSIDE 는 "코스 정의: GENOCIDE 공식 코스 파일(2018_course.zip), YYYY-MM-DD 받음, SHA-256 앞 12자 · 공식 페이지 링크 · 제약·트로피: beatoraja 기본 난이도표 · DP 코스의 제약·트로피는 같은 단 SP 값(원천 없음)" — 문구는 `detail.constraint_sources` 가 있을 때만 둘째 줄. Stella 는 표 출처와 같은 형식.
3. 코스 상세(`course-detail-panel.tsx`): 차트 링크를 `/charts/${row.sha256 || row.md5}` 로(2.6절 2 수리), 제목 열(카탈로그 항목의 `charts[position].title`), 위쪽에 출처 블록과 "이 코스는 {체계} {label} 을 줍니다" 한 줄. 데이터는 `courseCatalogEntryQueryOptions(hash)`(`course-detail-content.tsx` 에서 같이 프리페치).
4. 내비게이션에 `{ href: '/courses', label: '코스', icon: ListOrdered }`(`lucide-react` 기존 의존, 난이도표 다음).
5. 마이페이지 "단위" 탭: `settings-tabs.tsx` 의 `SETTINGS_TABS` 에 `DAN_SETTINGS_TAB = 'dans'`(`shared/constants/navigation.ts` 에 상수), 트리거 "단위", 내용 `web/src/widgets/settings/dan-panel.tsx`: 체계마다 획득 단위 목록(라벨, 코스 이름, 받은 날, 게이지 모드 `LR2`/`BEATORAJA`/`IIDX`)과 표시 선택(shadcn `Select`: "자동(가장 높은 단위)" / "숨김" / 획득한 label 들). 바꾸면 `useSetDanDisplay()` mutation → `toast.success('표시할 단위를 바꿨습니다')`. 획득이 없으면 "아직 획득한 단위가 없습니다. 코스를 클리어하면 자동으로 들어옵니다(제약: …)" 와 인정 조건 요약. 단일 선택이라 `useState` + mutation(폼 라이브러리 불필요, frontend.md 8.4).
6. 배지 `web/src/features/dan/dan-badge.tsx`(`FC<{ dan: DisplayedDan }>`, 예 "GENOSIDE 2018 SP 発狂五段"). 프로필(`widgets/player/player-detail-panel.tsx`, 이름 아래)과 순위 목록(`widgets/leaderboard/player-leaderboard-panel.tsx` 또는 행을 그리는 그 파일, 이름 옆)에 그린다.
7. 엔티티: `web/src/entities/course.query.ts` 에 `courseCatalogQueryOptions()`(`QUERY_KEY.COURSE.CATALOG`)·`courseCatalogEntryQueryOptions(hash)`(`QUERY_KEY.COURSE.CATALOG_ENTRY(hash)`)·`useCourseCatalog`·`useCourseCatalogEntry`, 타입은 `course.type.ts`. 새 `web/src/entities/dan.query.ts`(`'use client'`, `myDansQueryOptions()` = `QUERY_KEY.DAN.MINE`, `useMyDans`, `useSetDanDisplay`: `onSuccess` 에서 `invalidateQueries({ queryKey: QUERY_KEY.DAN.ALL })` 와 `QUERY_KEY.PLAYER.PROFILE(loginId)`, `onError` toast), `web/src/entities/dan.type.ts`. `QUERY_KEY` 에 `COURSE.CATALOG: ['course', 'catalog']`, `COURSE.CATALOG_ENTRY: (hash) => ['course', 'catalog', hash]`, `DAN: { ALL: ['dan'], MINE: ['dan', 'mine'] }`.
8. 읽기 함수: `public-reads.ts` 에 `readCourseCatalog`, `readCourseCatalogEntry`, `read-cache.ts` 에 `getCourseCatalogCached`(`'use cache'`, `cacheLife('minutes')`, `cacheTag(CACHE_TAG_EXT.courseCatalog())`)·`getCourseCatalogEntryCached`. 등록 스크립트는 캐시를 무효화하지 못하므로 `minutes`(몇 분 안에 반영).

### 3.8 호환성·마이그레이션 요약

| 대상 | 변경 | 호환 | 마이그레이션 |
| --- | --- | --- | --- |
| 코스 제출 와이어(받기) | `gauge`·`course_gauge`·`stages` 선택 필드 | 옛 클라이언트(필드 없음)는 그대로 받고 등급 0·단위 판정 없음. 새 클라이언트 + 옛 서버는 키를 버림(400 없음) | 0012 의 `course_score` 세 열 |
| IR 응답(제출·순위·코스 정의) | 없음(코스 정의 `charts` 는 md5 만 있는 차트에 `sha256: ''`) | — | — |
| IR 프로필 | `extra.dans` 추가 | `extra` 는 맵(2.6절 4) | `user_dan`·`user_dan_display` |
| 표준 표 JSON | insane1 그대로. st·sl 새로(`course` 포함) | — | `table_snapshot` |
| fe API | 표 요약에 `source`(insane1 은 null), 순위 행에 `dans`, 새 경로 4개 | 웹 화면만 소비 | — |
| `course_chart` | `md5`·`title` 열, md5 만 있는 차트는 `chart_sha256 = ''` | 옛 코드는 새 열을 모르고 빈 sha256 행이 없다(등록은 코드 푸시 뒤) | 0012 |
| 운영 DB 쓰기 경로 | Vercel Cron 이 `table_snapshot`·코스 묶음·`user_dan` 을 쓴다 | 2026-10-14 운영 DB ADR("수동 `web-db.yml` 만")의 부분 변경(결정 13) | ADR 갱신(7절) |

---

## 4. 원본 대비 차이 표

"의미" 열: **판정·점수·기록 의미가 원본(beatoraja → LR2/LR2IR → IIDX)이나 지금 R-BMS 와 달라지는 것**은 굵게. 원본에는 서버 쪽 단위 판정이 없다(beatoraja 는 클라이언트가 코스 클리어를 기록할 뿐이고, LR2IR 의 판정 방식은 소스가 없어 확인 못함).

| id | 항목 | 이 설계 | 원본·참고 | 의미 | 근거 |
| --- | --- | --- | --- | --- | --- |
| E1 | 단위 인정의 게이지 모드 | LR2·BEATORAJA·IIDX 모두 인정, `user_dan.course_gauge` 에 기록 | 발광·Stella 코스는 `gauge_lr2` 태그라 beatoraja 에서도 LR2 단위 게이지로만 돈다(조사 B-2) | **단위 의미 다름**: BEATORAJA·IIDX 모드 클리어도 같은 단위 | 결정 20·21 |
| E2 | 어시스트·SCRATCH AUTO | 단위 없음(코스 기록·순위는 지금처럼) | beatoraja 는 어시스트 옵션 판을 따로 기록, 서버 단위 없음 | R-BMS 규칙 | 결정 20 |
| E3 | 코스 최고 기록의 등급 | `course_best.assist_grade` = 마지막 스테이지 게이지의 등급(Class 2, ExClass 3, ExHardClass 4), LIGHT ASSIST 램프일 때만 | 등급은 R-BMS 확장(원본에 없음) | **순위 의미 바뀜(지금 R-BMS 대비)**: SCRATCH AUTO 코스 클리어끼리 게이지 등급이 순서에 들어간다 | 결정 6 |
| E4 | 단위 인정의 순위 판 조건 | `ranked=true` 판만(넓힌 판정·신뢰하지 않는 빌드 제외) | — | R-BMS 규칙(8절 결정 8) | 추천 |
| E5 | 램프 조건 | NO PLAY·FAILED·ASSIST·LIGHT ASSIST 램프와 게이지 0 이하 제외 | beatoraja 코스 클리어 = 모든 스테이지 `isQualified` | 같은 뜻(어시스트 강등 램프만 R-BMS 확장) | 결정 20 |
| E6 | 미러·LN 확인 | 스테이지마다 RANDOM 은 grade 제약의 허용 집합, `lntype` 은 LN 제약 값 | beatoraja 는 선곡에서 강제(`MusicSelector.java:470-505`), 서버 확인 없음 | 확인 위치만 다름. 차트 자체 `#LNMODE` 가 강제된 곡은 `lntype` 이 달라 **단위가 안 나올 수 있음**(9절 R3) | 결정 20, F5 R5 |
| E7 | DP 단위 | DP 코스 제약·트로피는 같은 단 SP 값, DP 미러에 FLIP 없음 | 원천 없음 / beatoraja 는 MIRROR 코스에서 FLIP | **DP 배치 의미 다름**(F5 D9·D13) | F5 설계 |
| E8 | 트로피 | 단위 판정에 쓰지 않는다(정의만 저장) | beatoraja 트로피 은·금 | 범위 밖(9절 T5) | 조사 B-2 추천 |
| E9 | 같은 단위 여러 번 | 처음 획득만 남김, 표시 자동은 체계의 가장 높은 rank | LR2IR 단위 표시 방식은 확인 못함 | R-BMS 규칙 | 8절 결정 13 |
| E10 | 옛 판 소급 | `stages` 없는 판은 판정 안 함 | — | **0012 이전 클리어는 단위가 생기지 않음** | 8절 결정 11 |
| E11 | 표 판 바뀜(Stella 4th → 5th) | 옛 판 코스도 계속 단위를 준다(`active=false` 라도 `dan_course` 유지) | Stella 쪽 운영 방식 확인 못함 | R-BMS 규칙 | 8절 결정 6 |
| E12 | 표 사본 지연 | 원본은 2주 주기 반영, 사본은 하루 한 번(Hobby 면 KST 0:00~0:59) | 원본 `max-age=60` | 사본 표가 원본보다 최대 약 하루 늦다 | 결정 13 |
| E13 | 코스 id | 이름 포함 R-BMS 규칙 | LR2IR 접두사+md5, beatoraja 캐시 차트+제약 | 기존(F5 D11) | F5 설계 |

---

## 5. 구현 단위

### 5.1 단위 표

모든 단위 공통: `web/AGENTS.md` 와 컨벤션(arrow function, `any`·`enum`·주석 금지 — JSDoc 만 영어, FSD, TanStack Query + `queryOptions` + `QUERY_KEY`, Zod 경계 검증, `useCallback`·`useMemo` 금지, Route → Service → compose `*ServiceDb`), 매직넘버는 이름 있는 상수, 새 npm 의존성 금지, `server-only` 를 스크립트 import 경로에 넣지 않음. 에이전트는 커밋·푸시·운영 DB·운영 사이트 요청을 하지 않는다. `.env*` 를 읽지 않는다.

| 단위 | 내용 | 소유(수정 가능) 파일 | 읽기 전용 | 선행 | 다른 묶음과 겹치는 파일 | 병렬 |
| --- | --- | --- | --- | --- | --- | --- |
| W1 | 마이그레이션 0012 와 Drizzle 정의 | `web/src/server/db/schema.ts`, 새 `web/drizzle/0012_course_dan_tables.sql`, 새 `web/drizzle/meta/0012_snapshot.json`, `web/drizzle/meta/_journal.json`, 새 `web/tests/db/migration-0012.test.ts` | `web/drizzle/0011_assist_grade.sql`, `docs/web/architecture.md` 3-2절 | 없음 | F7 M2(`schema.ts`, 다음 번호 0013) → F6 뒤 직렬 | 첫 단계 |
| W2 | 코스 제출 게이지·스테이지·등급(결정 6) | `web/src/server/dto/course.ts`, `web/src/server/service/domain/course/course.service.ts`(제출 부분), `web/src/server/compose/course.ts`(`findCourseScoreByIdempotency`·`insertCourseScore`), `web/tests/service/course.test.ts`, `web/tests/dto/client-requests.test.ts`, `web/tests/db/assist-grade.test.ts`(코스 테스트 1건), 새 `web/tests/db/course-submission-gauge.test.ts` | `contract/client-requests/*.json`, `web/src/server/service/domain/score/assist-grade.ts` | W1 | F5 C4 가 `contract/client-requests/course-*.json` 을 다시 만든다 → **C4 골든 커밋은 W2 와 같은 푸시이거나 그 뒤**(5.2절) | W5 와 병렬 |
| W3 | 코스 카탈로그·출처·등록 스크립트(결정 18·14) | 새 `web/src/server/service/domain/course-catalog/{course-hash.ts,constraint-tokens.ts,bundled-courses.ts,table-courses.ts,course-catalog.type.ts,course-catalog.service.ts}`, 새 `web/src/server/compose/course-catalog.ts`, 새 `web/src/server/dto/course-catalog.ts`, `web/src/server/service/domain/course/course.service.ts`(`upsertMeta`·`toCourseMeta`·`CourseChartRow`), `web/src/server/compose/course.ts`(`findCourseCharts`·`replaceCourseCharts`), 새 `web/scripts/seed-courses.ts`, 새 `web/scripts/lib/script-db.ts`, 새 테스트 `web/tests/service/course-hash.test.ts`·`course-catalog.test.ts`·`table-courses.test.ts`, 새 `web/tests/scripts/seed-courses.test.ts`, 새 `web/tests/db/course-catalog.test.ts` | `web/data/courses/genoside-2018/courses.json`(F5 C5 산출), `crates/rbms-course/src/model.rs` | W2(같은 두 파일 → 직렬) | F5 C5 의 데이터 파일(읽기만). 파일이 아직 없으면 테스트는 6.1절의 합성 픽스처로 하고, 44개 해시 대조 테스트는 파일이 생긴 뒤 켠다(`test.skipIf(!existsSync(...))` 가 아니라 **C5 가 먼저 들어온 뒤 W3 를 시작**하는 것을 기본 순서로) | 아니오(W2 뒤) |
| W4 | 단위 판정·표시 API(결정 19·20) | 새 `web/src/server/dto/dan.ts`, 새 `web/src/server/service/domain/dan/{dan-rules.ts,dan-eligibility.ts,dan.service.ts}`, 새 `web/src/server/compose/dan.ts`, `web/src/server/route/course.ts`(제출 뒤 판정 호출), 새 `web/src/server/route/dan.ts`, 새 `web/src/app/api/fe/me/dans/route.ts`, 새 `web/src/app/api/fe/me/dans/display/route.ts`, `web/src/server/service/read-cache.ts`(`revalidateOnDanAward`), 새 `web/scripts/dan-recompute.ts`, 새 테스트 `web/tests/service/dan-rules.test.ts`·`dan-eligibility.test.ts`·`dan.test.ts`, `web/tests/route/dan.test.ts`, 새 `web/tests/db/dan-award.test.ts` | W3 의 `CatalogDan` 타입 | W3 | 없음 | 아니오(W3 뒤) |
| W5 | 표 스냅샷 저장소·검증·Cron(결정 13) | 새 `web/src/server/service/domain/table-refresh/{table-sources.ts,table-refresh.validate.ts,table-refresh.service.ts}`, 새 `web/src/server/service/shared/http/fetch-limited.ts`, `web/src/server/service/shared/storage/gzip-json.ts`(바이트 함수 2개), 새 `web/src/server/compose/table-refresh.ts`, `web/src/server/compose/table-snapshot.ts`, `web/src/server/service/domain/table-snapshot/{table-snapshot.data.ts,table-snapshot.service.ts}`, `web/src/server/dto/table-snapshot.ts`, 새 `web/src/server/dto/table-refresh.ts`, `web/src/server/route/table-snapshot.ts`, 새 `web/src/server/route/cron.ts`, 새 `web/src/server/lib/with-cron-secret.ts`, `web/src/server/lib/env.ts`, 새 `web/src/app/api/cron/tables/route.ts`, `web/vercel.json`, `web/src/server/compose/folder.ts`, `web/src/server/service/domain/folder/folder.service.ts`(`loadTableEntries` async), 새 `web/scripts/refresh-tables.ts`, 테스트 `web/tests/route/table-snapshot.test.ts`·`web/tests/service/table-snapshot.test.ts`·`web/tests/service/folder.test.ts`·`web/tests/env.test.ts`·`web/tests/db/assist-grade.test.ts`(216·225행의 표 읽기를 await 로만), 새 `web/tests/service/table-refresh.test.ts`·`table-refresh-validate.test.ts`, 새 `web/tests/route/cron.test.ts`, 새 `web/tests/lib/vercel-config.test.ts`, 새 `web/tests/db/table-snapshot-store.test.ts` | Stella 사본(조사 산출물, 저장소에 복사하지 않음) | W1. 코스 동기화 호출은 `courseCatalog`·`dan` 을 deps 로 받아 W3·W4 없이도 스텁으로 테스트 | F7 M2 가 `env.ts` 를 만진다 → F6 뒤 직렬. `web/tests/db/assist-grade.test.ts` 는 W2 와 겹침 → W2 다음 커밋 | W2 와 병렬 |
| W6 | 조립 통합 | `web/src/server/compose/index.ts`(courseCatalog·dan·tableRefresh 조립, `tableSnapshotService` 의 비동기 `loadTables`, `playerService`·`feService` deps 의 `listDisplayedDans`), `web/src/server/service/domain/player/player.service.ts`, `web/src/server/service/domain/fe/fe.service.ts`, `web/src/server/compose/fe.ts`, `web/src/server/dto/auth.ts`(프로필 extra 타입, 필요하면), `web/src/server/dto/fe.ts`(순위 행 `dans`), 테스트 `web/tests/service/fe-player-history.test.ts` 등 프로필·순위 스텁 | — | W3, W4, W5 | F7 M2(`compose/index.ts`) → F6 뒤 | 아니오 |
| W7 | 표 화면(결정 12·14) | 새 `web/src/app/(app)/tables/stella-st/page.tsx`, 새 `web/src/app/(app)/tables/stella-sl/page.tsx`, `web/src/app/(app)/tables/page.tsx`, `web/src/widgets/table/table-snapshot-content.tsx`, `web/src/widgets/table/table-catalog-content.tsx`, `web/src/widgets/table/table-snapshot-list-item.tsx`, 새 `web/src/features/table/table-source-attribution.tsx`, 새 `web/tests/components/table-source-attribution.test.tsx` | `web/src/features/table/table-external-link.tsx`, `web/src/widgets/table/table-chart-details.tsx` | W6 | F7 M4 가 `table-chart-details.tsx` 를 만진다(W7 은 고치지 않음) | W8 과 병렬 |
| W8 | 코스·단위 화면(결정 14·19) | 새 `web/src/app/(app)/courses/page.tsx`, 새 `web/src/widgets/course/{course-catalog-content.tsx,course-catalog-panel.tsx}`, `web/src/widgets/course/{course-detail-content.tsx,course-detail-panel.tsx}`, 새 `web/src/features/course/course-source-notice.tsx`, 새 `web/src/features/dan/dan-badge.tsx`, 새 `web/src/widgets/settings/dan-panel.tsx`, `web/src/widgets/settings/settings-tabs.tsx`, `web/src/widgets/player/player-detail-panel.tsx`, 순위 행을 그리는 `web/src/widgets/leaderboard/*.tsx`(구현자가 행 그리는 파일 하나를 확인), `web/src/entities/{course.query.ts,course.type.ts}`, 새 `web/src/entities/{dan.query.ts,dan.type.ts}`, `web/src/shared/constants/{query-key.ts,navigation.ts}`, `web/src/server/service/public-reads.ts`, `web/src/server/service/read-cache.ts`(카탈로그 캐시), 새 `web/src/server/route/course-catalog.ts`, 새 `web/src/app/api/fe/courses/route.ts`, 새 `web/src/app/api/fe/courses/[courseHash]/catalog/route.ts`, 새 테스트 `web/tests/components/dan-panel.test.tsx`·`course-source-notice.test.tsx`, `web/tests/route/course-catalog.test.ts` | — | W6 | `read-cache.ts` 는 W4 와 겹침 → W4 뒤 | W7 과 병렬 |
| W9 | 운영 워크플로 | `.github/workflows/web-db.yml`(action 6개) | `web/scripts/{seed-courses,refresh-tables,dan-recompute}.ts` | W3, W4, W5 | 없음 | W7·W8 과 병렬 |
| W10 | 문서(7절) | 7절 목록 | — | W1~W9 | F5·F3·F7 문서와 `HANDOFF.md`·`PROCESS.md`·`docs/backend/*` 공유 → 메인이 합친다 | 마지막 |

### 5.2 커밋·배포 순서(사용자 승인 포함)

1. **커밋 A(W1 의 마이그레이션 파일만)**: `web/drizzle/0012_course_dan_tables.sql`, `web/drizzle/meta/0012_snapshot.json`, `web/drizzle/meta/_journal.json`. `schema.ts` 는 넣지 않는다(2.2절 4: `db.select().from(course…)` 가 새 열을 읽어 migrate 전에 500 이 난다). 푸시 → Vercel 배포는 옛 코드 그대로.
2. `gh workflow run web-db.yml --ref dev -f action=status` → 0012 가 `would_run=yes` 인지(읽기만).
3. **사용자 승인으로 운영 적용**: 자동 모드가 거부하므로(HANDOFF 5절) 메인이 무엇을 왜 하는지(새 열 3·2개, 새 표 7개, 기존 행 변경 없음)와 명령 `gh workflow run web-db.yml --ref dev -f action=migrate` 를 보여 주고 사용자가 승인하거나 `! gh workflow run web-db.yml --ref dev -f action=migrate` 로 직접 실행 → 다시 `status` 로 최신 확인.
4. **사용자가 직접**: Vercel 에 `CRON_SECRET` 등록(4.2절 1). 이 단계가 5 보다 먼저여야 첫 배포부터 Cron 이 인증된다.
5. **커밋 B 이후(W2~W9 코드)**: `schema.ts` 포함 웹 코드, `vercel.json`, `web-db.yml`. W2(DTO)와 F5 C4(골든 재생성)는 **같은 푸시**로 낸다(2.3절: C4 골든만 먼저 가면 웹 CI 가 "서버가 버림" 으로 실패, W2 만 먼저 가면 문제없음). 푸시 → Vercel 배포 → `gh run list --branch dev` 로 web CI·Rust CI 확인.
6. `gh workflow run web-db.yml --ref dev -f action=courses-seed-count`(읽기만, 숫자 확인: SP 22·DP 22 inserted, danCourses 44) → 메인이 숫자를 보여 주고 **사용자 승인** → `-f action=courses-seed`(세기 → 적용 → 다시 세기, 바뀔 것 0).
7. 첫 표 갱신: 사용자가 Vercel 대시보드 Settings → Cron Jobs 에서 실행(2.8절 "Run cron jobs from deployment summary" changelog 와 관리 화면)하거나 다음 KST 0시를 기다린다. 그 전에 `-f action=tables-refresh-count`(러너가 원본을 받아 검증·비교만, 쓰지 않음)로 숫자를 볼 수 있다.
8. 클라이언트 릴리스(F5, `stages` 를 보내는 빌드)는 웹 배포 뒤에(2026-10-14 어시스트 코스 ADR 결정 5 와 같은 순서). 웹보다 먼저 나간 빌드의 판은 서버가 `stages` 를 버려 단위가 영영 생기지 않는다(소급 불가, 9절 R1).

---

## 6. 테스트 계획

### 6.1 새 테스트(파일, 이름, 단언). 설명은 한국어(`bun:test`)

**W1 — `web/tests/db/migration-0012.test.ts`(RBMS_DB_E2E)**
- `0012 는 기존 표의 열 타입을 바꾸지 않고 열과 표를 더하기만 한다`: SQL 파일 텍스트에 `ALTER TABLE ... MODIFY`·`DROP`·`UPDATE` 가 없고 `ADD`·`CREATE TABLE` 만.
- `0012 뒤 course_chart 에 sha256 이 빈 md5 차트를 넣고 읽을 수 있다`: `chart_sha256 = ''`, `md5` 32자 행 왕복.
- `user_dan 은 사용자를 지우면 같이 지워진다`: FK cascade.

**W2 — `web/tests/service/course.test.ts`(추가)**
- `게이지가 있는 SCRATCH AUTO 코스 클리어는 그 게이지 등급으로 최고 기록에 들어간다`: `clear: LightAssistEasy`, `gauge: 'ExClass'` → upsert 행 `assistGrade === 3`.
- `게이지가 없는 옛 코스 본문은 등급 0 이다`: `gauge` 없음 → 0, 삽입 행 `gauge === null`, `stages === null`.
- `스테이지는 같은 자리 차트와 짝지어 저장된다`: 3차트 코스, 2스테이지 → `stages` 길이 2, `stages[1].md5 === charts[1].md5`.
- `같은 판을 다시 보내면 처음 판의 게이지로 등급을 다시 만든다`: 중복 경로 `findCourseScoreByIdempotency` 가 `gauge: 8` 을 돌려주면 upsert `assistGrade === 4`(clear LightAssist).
- `코스 게이지 모드는 그대로 저장된다`: `course_gauge: 'IIDX'` → 행 `courseGauge === 'IIDX'`.
- `범위 밖 lntype 도 판을 거절하지 않는다`: `stages[0].lntype = 7` → 파싱 성공.

**W2 — `web/tests/dto/client-requests.test.ts`(추가·수정)**
- `createCourseStub` 의 `findCourseScoreByIdempotency`·`insertCourseScore` 타입을 새 행에 맞춘다(단언 변화 없음).
- `세 필드가 없는 옛 코스 본문도 받고 gauge 는 null, stages 는 비어 있다`: `course-normal.json` 에서 `gauge`·`course_gauge`·`stages` 를 지운 객체 → `success`, `gauge === null`, `stages` 길이 0.
- 기존 `파싱 결과가 입력과 같다` 는 C4 재생성 골든(세 필드 포함)에서 통과해야 한다(새 키를 버리지 않음).

**W2 — `web/tests/db/course-submission-gauge.test.ts`(RBMS_DB_E2E)**
- `코스 제출이 gauge·course_gauge·stages 를 저장하고 course_best.assist_grade 를 채운다`: LightAssist + `ExHardClass` → `course_best.assist_grade === 4`, `course_score.gauge === 8`, `stages` JSON 왕복.
- `0012 이전 모양의 판(게이지 NULL)을 다시 보내도 등급은 0 이다`.

**W2 — `web/tests/db/assist-grade.test.ts`**: 229행 테스트 이름을 `게이지가 없는 옛 코스 본문은 코스 최고 기록의 등급이 0 이다` 로 바꾸고 단언은 그대로(본문에 `gauge` 가 없다). 같은 블록에 `게이지를 실은 코스 본문은 그 게이지 등급으로 순위에 오른다`(`gauge: 'Class'` → `assistGrade 2`, 순위 `assist_grade 2`) 추가.

**W3 — `web/tests/service/course-hash.test.ts`**
- `코스 id 는 이름과 차트 md5 로 만든다 — GENOSIDE 初段`: 2.7절 첫 줄 벡터.
- `Stella Skill Simulator 4th st0 의 id`, `Satellite Skill Analyzer 2nd sl0 의 id`: 2.7절 벡터.
- `sha256 이 있는 차트는 sha256 을 쓴다`: 같은 이름, 한 차트에 sha256 → 다른 값이고 손 계산 SHA-256 과 같음.
- `저장소의 GENOSIDE 2018 파일 44개 코스 id 가 TS 계산과 모두 같다`: `web/data/courses/genoside-2018/courses.json` 을 읽어 44/44(Rust 가 만든 파일과 TS 규칙의 교차 확인).

**W3 — `web/tests/service/table-courses.test.ts`(합성 픽스처)**
- `배열의 배열을 펴서 순서대로 읽는다`, `평평한 배열과 객체 하나도 읽는다`, `md5 는 소문자로 바뀐다`, `md5 문자열 하나도 차트 하나가 된다`, `차트가 없는 코스는 건너뛰고 센다`, `모르는 제약은 버리고 같은 그룹은 처음 것만 남는다`(`['GRADE_MIRROR','grade','gauge_lr2','LN','cn']` → `['grade_mirror','gauge_lr2','ln']`), `트로피의 missrate 는 smissrate 로 옮겨진다`, `제목은 표 항목의 md5 로 채우고 없으면 빈 글자`, `정의에 없는 sha256 을 채우지 않는다`(항목에 sha256 이 있어도 코스 차트 `sha256 === ''`), `ln 제약 코스의 lntype 은 0, cn 은 1, hcn 은 2`.

**W3 — `web/tests/service/course-catalog.test.ts`(메모리 db)**
- `처음 동기화는 모든 코스를 넣고 단위 코스를 센다`: GENOSIDE 합성 2묶음 → inserted, `danCourses`.
- `같은 정의를 다시 동기화하면 바뀌는 것이 없다`(멱등).
- `표에서 사라진 코스는 비활성이 되고 단위 코스는 남는다`.
- `계획(planSync)은 쓰지 않는다`: db 쓰기 호출 0.
- `파일의 course_hash 가 계산과 다르면 등록을 멈춘다`: `bundledCatalog` 가 던진다.
- `md5 만 있는 차트도 관리자 정의 등록에서 버리지 않는다`: `upsertMeta` 에 `sha256: ''` 차트 → `replaceCourseCharts` 인자에 남음.

**W3 — `web/tests/scripts/seed-courses.test.ts`**: `count 는 숫자만 출력하고 행 내용을 출력하지 않는다`(출력에 md5·이름이 없음), `알 수 없는 인자는 종료 코드 2`. `web/tests/db/course-catalog.test.ts`(RBMS_DB_E2E): `apply 를 두 번 돌려도 같은 행 수다`, `GENOSIDE 코스 상세의 차트는 md5 와 제목을 돌려준다`(`toCourseMeta` 의 `md5` 가 `course_chart.md5`).

**W4 — `web/tests/service/dan-rules.test.ts`**
- `GENOSIDE SP 初段 은 rank 1, Overjoy 는 rank 22`, `DP 접두사는 DP 체계`, `Stella Skill Simulator 4th st12 는 st12 rank 12`, `5th 판 이름도 같은 label`, `STELLA技能訓練 코스는 단위가 없다`, `다른 묶음의 같은 이름은 단위가 없다`.

**W4 — `web/tests/service/dan-eligibility.test.ts`**(기준 판 하나에서 한 가지씩 바꿈, 이유 코드 단언)
- `조건을 모두 지킨 LR2 판은 인정된다`, `BEATORAJA·IIDX 모드 판도 인정된다`, `단위 코스가 아니면 not_a_dan_course`, `게스트는 guest`, `순위에 오르지 않는 판은 unranked`, `FAILED 는 not_cleared`, `LIGHT ASSIST 램프는 not_cleared`, `게이지 0 은 not_cleared`, `SCRATCH AUTO 는 scratch_auto`, `ASSIST 플래그는 assisted`, `stages 가 없으면 no_stage_facts`, `코스 게이지가 아닌 스테이지는 not_a_course_gauge`, `스테이지 수가 다르면 stage_count`, `다른 곡이면 chart_mismatch`(대소문자만 다른 md5 는 같다), `grade_mirror 코스의 RANDOM 은 random_not_allowed`, `grade_mirror 코스의 MIRROR 는 인정`, `grade 코스의 MIRROR 는 random_not_allowed`, `제약 없는 코스의 RANDOM 은 인정`, `ln 코스의 CN 스테이지는 ln_mode_mismatch`.

**W4 — `web/tests/service/dan.test.ts`(메모리 db)**
- `인정된 판은 단위를 넣고 다시 넣어도 처음 획득이 남는다`, `표시 기본은 가장 높은 단위`, `숨김이면 공개 목록에서 빠진다`, `획득하지 않은 label 은 고를 수 없다`(VALIDATION_ERROR), `자동으로 되돌리면 표시 행이 지워진다`, `소급은 stages 가 있는 판만 본다`, `소급 dryRun 은 넣지 않고 센다`.

**W4 — `web/tests/route/dan.test.ts`**: `세션이 없으면 401`, `GET 은 네 체계를 모두 돌려준다`, `PUT 은 잘못된 mode 를 400 으로`, `PUT 뒤 GET 의 shown 이 바뀐다`. 그리고 코스 제출 라우트: `단위 판정이 던져도 코스 제출은 201 이다`(판정 스텁이 던짐 → 응답 201, console.error 1회).

**W4 — `web/tests/db/dan-award.test.ts`(RBMS_DB_E2E)**: 등록(W3 apply) → 조건을 지킨 코스 제출 → `user_dan` 1행(`course_gauge`, `gauge`, `acquired_at === played_at`), FAILED·SCRATCH AUTO·RANDOM 판은 0행, 같은 판 재전송은 1행 유지, 프로필 `extra.dans` 에 보임, `dan-recompute apply` 가 등록 전에 낸 판(stages 있음)에 단위를 준다.

**W5 — `web/tests/service/table-refresh-validate.test.ts`**
- `level_order 가 없으면 레벨을 수 순서로 만든다`(`['0','12','3']` → `['0','3','12']`), `글자 레벨은 처음 나온 순서`, `BOM 이 붙은 머리도 읽는다`, `이름이 다르면 name_mismatch`, `기호가 다르면 symbol_mismatch`, `data_url 이 다른 호스트면 data_url_off_site`, `상대 data_url score.json 은 같은 폴더로 합쳐진다`, `md5 가 32자 hex 가 아니면 entries_invalid`, `숫자 레벨도 글자로 읽는다`, `같은 md5 가 둘이면 duplicate_md5`, `첫 저장이 500곡 미만이면 too_few_entries`, `10% 넘게 줄면 shrank`, `25% 넘게 바뀌면 changed_too_much`(레벨 이동도 센다), `강제 반영은 급감·변경량만 건너뛴다`(형식 오류는 여전히 거절).

**W5 — `web/tests/service/table-refresh.test.ts`(스텁 fetch·메모리 store)**
- `처음 받은 표를 저장하고 코스를 등록한다`(courseCatalog.applySync 1회), `내용이 같으면 확인 시각만 바꾼다`, `머리만 같고 data 가 바뀌면 저장하고 코스는 다시 등록하지 않는다`, `받기 실패는 이전 값을 남기고 failed 로 표시한다`, `한 표가 실패해도 다른 표는 갱신된다`, `다른 실행이 먼저 쓰면 덮지 않는다`(saveSnapshot false), `dryRun 은 쓰지 않는다`, `새 단위 코스가 생기면 소급 판정을 부른다`, `리다이렉트 응답은 따르지 않고 실패로 본다`, `크기 상한을 넘는 응답은 too_large`.

**W5 — `web/tests/route/cron.test.ts`**: `CRON_SECRET 이 없으면 503`, `Authorization 이 없으면 401`, `Bearer 값이 다르면 401`(길이가 같은 다른 값 포함), `맞으면 200 이고 표마다 결과를 돌려준다`, `응답은 캐시하지 않는다`(`Cache-Control: private, no-store`), `비밀 값이 응답 본문에 없다`.

**W5 — `web/tests/lib/vercel-config.test.ts`**: `vercel.json 에 매일 UTC 15시 표 갱신 Cron 이 있고 그 경로의 라우트 파일이 있다`(`crons` 에 `{ path: '/api/cron/tables', schedule: '0 15 * * *' }`, `src/app/api/cron/tables/route.ts` 존재), `ignoreCommand 는 그대로다`.

**W5 — `web/tests/route/table-snapshot.test.ts`·`web/tests/service/table-snapshot.test.ts`(수정·추가)**: 기존 단언을 `await` 로. 추가: `insane1 의 표준 머리는 지금과 바이트까지 같다`(키 다섯 개, `course` 없음), `insane1 요약의 source 는 null 이다`, `저장된 Stella 표는 목록·상세·표준 머리(course 포함)·표준 데이터(받은 바이트 그대로)로 나간다`, `Stella 의 ETag 는 data_sha256 앞 32자`, `저장 내용이 바뀌면 다음 읽기에서 새 표가 보인다`(메모리 캐시가 버전으로 갈아탐), `원격 표가 없으면 insane1 만 보인다`. `web/tests/service/folder.test.ts`: `loadTableEntries` 스텁을 async 로, `Stella 표도 폴더로 가져올 수 있다`.

**W5 — `web/tests/db/table-snapshot-store.test.ts`(RBMS_DB_E2E)**: `약 1 MB data 를 압축해 저장하고 같은 바이트로 읽는다`, `previous 가 다르면 update 영향 0`, `markChecked 는 내용 열을 바꾸지 않는다`.

**W5 — `web/tests/env.test.ts`**: `CRON_SECRET 은 16자 미만이면 환경 파싱이 실패한다`, `없어도 된다`.

**W6**: 프로필·순위 스텁에 `listDisplayedDans` 를 더하고, `플레이어 프로필 extra 에 표시 단위가 들어간다`, `순위 행마다 표시 단위가 붙고 조회는 한 번이다`.

**W7 — `web/tests/components/table-source-attribution.test.tsx`**: `출처 이름·원본 링크(새 탭)·KST 날짜·해시 앞 12자를 보인다`(Testing Library 역할 쿼리 `link`).

**W8 — `web/tests/components/dan-panel.test.tsx`(msw)**: `획득이 없으면 안내를 보인다`, `표시를 숨김으로 바꾸면 PUT 을 보내고 성공 토스트를 띄운다`, `자동 항목에 가장 높은 단위 이름이 보인다`. `course-source-notice.test.tsx`: `GENOSIDE 출처는 DP 값의 원천 없음 안내를 보인다`. `web/tests/route/course-catalog.test.ts`: `코스 묶음 목록은 출처와 단위를 싣는다`, `없는 코스의 catalog 는 404`.

### 6.2 뒤집히는 기존 테스트

| 테스트 | 위치 | 왜 | 처리 |
| --- | --- | --- | --- |
| `코스 본문에는 게이지가 없어 코스 최고 기록의 등급은 0 이다` | `web/tests/db/assist-grade.test.ts:229` | 이름이 더는 사실이 아님(단언은 그대로 맞음) | 이름 변경 + 새 테스트 |
| 표 스냅샷 서비스 동기 호출 단언 | `web/tests/service/table-snapshot.test.ts:94, 126` 등, `web/tests/route/table-snapshot.test.ts:15-16` | 서비스가 async, `loadTables` 가 Promise | `await`·`async () => getSnapshotTables()` 로 |
| `loadTableEntries` 스텁 | `web/tests/service/folder.test.ts:36` | async | `async (tableId) => …` |
| `getRecords` 직접 호출 | `web/tests/db/assist-grade.test.ts:216, 225` | `getSnapshotTable` 은 그대로 동기라 변화 없음, `composeTableSnapshot(getDb())` 의 인자가 바뀌면 맞춘다 | 조립 시그니처만 |
| `createCourseStub` | `web/tests/dto/client-requests.test.ts:158-175` 부근 | `CourseServiceDb` 타입 | 필드 추가 |
| 프로필·순위 스텁 | `createPlayerService`·`createFeService` 를 만드는 테스트(구현자가 `grep -rn "createPlayerService\|createFeService" web/tests` 로 확인) | deps 추가 | `listDisplayedDans: async () => …` |
| `web/tests/db/ai-score-features.test.ts:162` | `createTableSnapshotService({ …, loadTables: () => new Map() })` | Promise | `async () => new Map()` |

### 6.3 골든·계약 영향

1. `contract/client-requests/course-normal.json`·`course-scratch-auto.json` 은 F5 C4 가 다시 만든다(이 묶음은 고치지 않는다). 웹 `client-requests.test.ts` 는 W2 의 DTO 로 새 골든을 "버림 없이" 통과해야 한다. 푸시 순서는 5.2절 5.
2. `contract/client-requests/settings-blob.json`(F5 C2·F2 가 바꿈)은 서버가 `content` 를 문자열로 저장하므로 영향 없음(통과 여부만 확인).
3. IR 응답 계약(`crates/rbms-ir/src/contract_tests/fixtures.rs`, `web/tests/dto/ir-contract.test.ts`)은 바뀌지 않는다. 프로필 `extra.dans` 는 응답 고정 본문에 넣지 않는다(빈 맵으로도 읽힘을 Rust 테스트 `player_profile_decodes_without_extra` 가 이미 지킨다). 구현자가 `ir-contract.test.ts` 가 프로필 `extra` 를 정확 비교하는지 확인하고, 하면 `dans: []` 를 기대값에 더한다.
4. 렌더 골든·Rust 테스트: 영향 없음.

### 6.4 실행 명령(위험에 비례, 단위별 최소)

| 단위 | 명령(`cd /Users/hyunseokbyun/development/R-BMS/web`) |
| --- | --- |
| W1 | `bunx drizzle-kit generate --name course_dan_tables` 뒤 다시 실행해 `No schema changes`, `RBMS_DB_E2E=1 DATABASE_URL=<로컬 mysql:8.4.11> bun run db:migrate && bun test tests/db/migration-0012.test.ts` |
| W2 | `bun test tests/service/course.test.ts tests/dto/client-requests.test.ts`, DB: `RBMS_DB_E2E=1 … bun test tests/db/course-submission-gauge.test.ts tests/db/assist-grade.test.ts` |
| W3 | `bun test tests/service/course-hash.test.ts tests/service/table-courses.test.ts tests/service/course-catalog.test.ts tests/scripts/seed-courses.test.ts`, DB: `… bun test tests/db/course-catalog.test.ts`, 로컬 DB 에 `bun scripts/seed-courses.ts count` → `apply` → `count`(바뀔 것 0) |
| W4 | `bun test tests/service/dan-*.test.ts tests/service/dan.test.ts tests/route/dan.test.ts`, DB: `… bun test tests/db/dan-award.test.ts` |
| W5 | `bun test tests/service/table-refresh*.test.ts tests/route/cron.test.ts tests/route/table-snapshot.test.ts tests/service/table-snapshot.test.ts tests/service/folder.test.ts tests/lib/vercel-config.test.ts tests/env.test.ts`, DB: `… bun test tests/db/table-snapshot-store.test.ts`. 실제 원본 호출은 하지 않는다(스텁). 조사 사본 `…/scratchpad/web/{header,score,sl_header,sl_score}.json` 을 스텁 fetch 에 넣는 한 번의 로컬 확인은 허용(저장소에 복사하지 않음, 테스트 파일에 경로를 넣지 않음) |
| W6 | `bun run typecheck`, 영향 테스트 |
| W7·W8 | `bun test tests/components tests/route/course-catalog.test.ts`, `bun run build`(insane1 페이지가 정적(○/◐)으로 남는지, 새 페이지가 빌드되는지 출력 확인) |
| W9 | `actionlint` 가 있으면 `.github/workflows/web-db.yml`, 없으면 YAML 파싱(`bun -e "…"` 로 `Bun.YAML` 또는 기존 CI 의 검사) — 실행은 하지 않는다 |
| 통합(메인) | `bun run verify`(typecheck + lint + test), `bun run format:check`, `RBMS_DB_E2E=1 … bun test tests/db`, `git diff --check` |

---

## 7. 문서 갱신 목록

| 문서 | 갱신 |
| --- | --- |
| 새 `docs/acknowledge/2026-10-23-adr-course-dan-judgement.md` | 결정 19·20: 체계 4개와 규칙, 인정 조건 12단계와 이유 코드, ranked 조건, 처음 획득 유지, 표시(자동/숨김/선택)와 공개 범위, `user.rank` 미사용, 옛 판 소급 안 함, 후처리(제출 실패시키지 않음). 8절의 메인 판단은 "사용자 미확인" 으로 |
| 새 `docs/acknowledge/2026-10-23-adr-course-catalog-sources.md` | 결정 18·14: `course_source`·`course_group` 모델, GENOSIDE 등록 경로(`web-db.yml`), 표 코스는 매일 갱신에서, md5 차트 저장(`chart_sha256 = ''`), 출처 표기 위치, insane1 흔적 제거 유지 |
| 새 `docs/acknowledge/2026-10-23-adr-table-daily-refresh.md` | 결정 13·12: Vercel Cron(공식 문서 인용·URL), `CRON_SECRET`, 검증 규칙·상수, `table_snapshot`, 강제 반영 action, 실패 처리, 표 id |
| `docs/acknowledge/2026-10-14-adr-production-db-operations.md` | 부분 변경: Vercel Cron 이 `table_snapshot`·코스 묶음·`user_dan` 을 자동으로 쓴다(결정 13), 나머지는 `web-db.yml` 만. 새 action 6개 |
| `docs/acknowledge/2026-10-14-adr-default-insane-table.md`·`2026-10-14-adr-table-page-no-source-notice.md` | "현재 상태" 에 한 줄: insane1 은 그대로, Stella·Satellite 는 출처 표기(결정 14, 다른 ADR 링크) |
| `docs/acknowledge/README.md` | 새 ADR 3건 색인, 부분 변경 표시 |
| `docs/backend/api-spec.md` | 코스 제출 본문 세 필드, `/api/cron/tables`, `/api/fe/me/dans`(GET·PUT `display`), `/api/fe/courses`, `/api/fe/courses/{hash}/catalog`, 표 요약 `source`, 프로필 `extra.dans`, 순위 행 `dans`, 표준 머리의 `course`(원격 표) |
| `docs/backend/data-model.md` | 0012 의 열·표와 뜻(3.2절) |
| `docs/backend/compatibility.md` | 구 클라이언트(세 필드 없음 → 등급 0·단위 없음), 새 클라이언트 + 구 서버, 푸시 순서(5.2절) |
| `docs/web/architecture.md` | 3-2절 마이그레이션 목록에 0012, 운영 DB 작업 action 6개, 6-2절 배포에 Cron 과 `CRON_SECRET`, 새 절 "표 매일 갱신"(검증·저장·읽기·메모리 캐시), 상태 블록 |
| `docs/web/2026-10-14-insane-table.md` | 원격 표와의 관계 한 줄(insane1 은 커밋 스냅샷 그대로) |
| `docs/quality-assurance/2026-10-14-test-debt.md` | 9절 T1~T7, 3.93(코스 등급) 갱신 |
| `docs/quality-assurance/2026-10-11-default-skin-manual-check.md`(또는 웹 수동 점검 절) | 운영 확인: `/tables/stella-st` 출처·레벨 st0~st12, 표준 머리의 `course`, `/courses` 출처, 마이페이지 단위 탭, Cron 실행 로그 1회 |
| `docs/HANDOFF.md` | 6절 14(코스 등급은 이제 채워짐), 4절 ADR 요약, 7절 마이그레이션 0012·Cron |
| `docs/PROCESS.md` | F6 체크 상태 |
| `docs/reference/2026-10-23-stella-course-dan-survey.md` | 2.10절 정정 반영(또는 머리에 정정 절) |
| `docs/plan/2026-10-23-course-gauge-data.md` | 6.3절 4 정정(웹 교차 검사는 버린 키를 실패로 본다, 푸시 순서) — 메인이 합친다 |

---

## 8. 메인 결정 필요

| # | 결정 지점 | 추천안 | 근거 | 대안 |
| --- | --- | --- | --- | --- |
| 1 | 웹 표 id·경로 | **`stella-st`·`stella-sl`**(`/tables/stella-st`) | 클라이언트 기본 표 id(F3 설계)와 같아 문서·로그가 한 이름. `tableIdParamSchema` 에 맞음 | `st`·`sl` / `stella`·`satellite` |
| 2 | 표 저장 방식 | **표마다 한 행, 받은 바이트를 gzip mediumblob 로, 해시는 받은 바이트의 SHA-256** | 표준 `data.json` 을 원본과 같은 바이트로 낼 수 있고 변경 비교가 한 번의 해시. 0007 의 gzip blob 선례 | JSON 열(다시 직렬화돼 바이트가 달라짐) / 항목 정규화 표(2,350행 x 2, 갱신이 무거움) |
| 3 | 검증 상수 | **첫 저장 500곡 이상, 급감 10% 초과 거절, 변경 25% 초과 거절, 머리 1 MiB·data 8 MiB·20초** | 2.9절 크기, 2주 반영 규모(9절 R5) | 더 엄격(5%) / 급감만 보기 |
| 4 | 큰 변경을 받아들이는 길 | **`web-db.yml` `tables-refresh-count` → 승인 → `tables-refresh-force`** | 운영 DB 쓰기는 사람이 숫자를 보고(운영 ADR). 새 HTTP 관리 라우트를 늘리지 않는다 | 관리자 API 토큰 라우트(`withAdminBearer`) |
| 5 | 표 코스를 같은 갱신에서 | **예, 머리가 바뀌었거나 처음 저장일 때** | 코스 정의가 같은 header 에 있다. 따로 등록하면 판이 바뀐 코스가 늦게 들어온다 | 표 코스도 수동 action |
| 6 | 표에서 사라진 코스 | **`active=false`, 기록·순위·단위는 유지, 단위 계속 부여** | 지난 판(4th)으로 친 기록이 사라지지 않게 | 단위 부여를 멈춤 / 정의 삭제 |
| 7 | 단위 체계와 대상 | **GENOSIDE SP·DP 44개, Stella `Skill Simulator` 13개, Satellite `Skill Analyzer` 13개. `STELLA技能訓練` 은 단위 없음** | 결정 19 의 체계 예. 技能訓練 은 훈련 코스라 단계가 아니다(이름으로 순서가 없다) | 技能訓練 도 각자 label 로 / Stella 를 체계 하나로 st·sl 합침 |
| 8 | ranked 조건 | **`ranked=true` 판만 인정** | 넓힌 판정(ASSIST)·신뢰하지 않는 빌드를 같은 기준으로 막는다. SCRATCH AUTO 는 결정 20 이 이미 제외 | ranked 무시(어시스트 플래그만 봄) |
| 9 | 램프 조건 | **NO PLAY·FAILED·ASSIST·LIGHT ASSIST 제외 + 게이지 > 0** | 결정 20 "게이지가 남은 채 끝남". 어시스트 강등 램프는 어시스트 판 | FAILED·NO PLAY 만 제외(어시스트는 플래그로만) |
| 10 | 스테이지 수·곡 일치 | **필수(수 = 코스 차트 수, md5 일치)** | 코스 id 는 클라이언트가 붙인다. 다른 곡 판이 단위 id 를 달고 오는 것을 막는다 | 확인 안 함 |
| 11 | 옛 판 소급 | **`stages` 없는 판은 판정 안 함. `stages` 있는 판은 `dan-recompute` 와 새 단위 코스 등록 때 소급** | 제약 준수·곡을 알 수 없다(결정 20 위반 가능) | 램프만으로 소급(제약 미확인 표시) / 소급 전혀 없음 |
| 12 | `user.rank` | **쓰지 않음(빈 값 유지), 단위는 `user_dan`·`user_dan_display`** | varchar(32) 한 칸은 4체계·표시 선택을 담지 못한다 | 대표 단위 한 개를 `rank` 에 복사 |
| 13 | 표시·공개 | **기본 자동(가장 높은 단위), 숨김 가능. 공개는 표시 단위만(프로필·순위), 획득 목록·판 id·시각·모드는 본인만** | 결정 19 "표시할 단위를 고름". 사용자가 고른 것만 남에게 보인다 | 획득 목록 전체 공개 / 기본 숨김 |
| 14 | 순위 목록의 단위 | **순위 행에 표시 단위 배지** | 결정 19 "프로필·순위에 보이기" 의 순위. 한 번의 묶음 조회 | 프로필만 |
| 15 | 게이지 모드별 순위 | **나누지 않음, `course_score.course_gauge`·`user_dan.course_gauge` 에 기록만** | 결정 20 "모두 인정하되 기록". IR 순위 와이어를 바꾸지 않는다 | `course_best` PK 에 모드 추가(마이그레이션·와이어 변경) |
| 16 | md5 만 있는 코스 차트 | **`course_chart.md5` 열 추가 + `chart_sha256 = ''`** | 추가만 규칙(열 타입·NULL 여부 변경 없음) | `chart_sha256` 을 NULL 허용으로 MODIFY / 별도 표 |
| 17 | GENOSIDE 등록 수단 | **TS 스크립트 + `web-db.yml` `courses-seed-count`/`courses-seed`** | 데이터 파일을 그대로 읽고 해시를 다시 확인, 세기 먼저(운영 ADR) | 0006 처럼 데이터 마이그레이션 SQL / 관리자 세션 `POST /api/courses/meta` 44번 |
| 18 | 마이그레이션 개수 | **0012 하나** | 같은 코드 푸시가 모두를 기대, 모두 추가만 | 제출·카탈로그·단위·표 넷으로 나눔 |
| 19 | 코스 목록 화면 | **새 `/courses` 페이지 + 내비게이션 "코스"** | 출처 표기(결정 14)를 보일 목록 화면이 없다 | 표 페이지 안에 코스 절 |
| 20 | 출처 표기 내용 | **출처 이름·제작 주체·원본 링크·받은 날(KST)·해시 앞 12자, GENOSIDE 는 제약·트로피 원천과 "DP 값은 SP 값" 안내** | 조사 B-3 추천(제작 주체·URL·날짜·해시) | 이름과 링크만 |
| 21 | 코스 차트 제목 | **`course_chart.title`(GENOSIDE 는 데이터 파일, 표 코스는 그 표 항목)** | 곡이 없는 사람도 무엇을 쳐야 하는지 보인다 | 제목 없음(md5 만) |
| 22 | IR 프로필 | **`extra.dans` 추가** | 웹 프로필과 같은 출력, 클라이언트 맵 읽기라 안전 | fe 전용 출력만 |
| 23 | `CRON_SECRET` 이 없을 때 | **503 `SERVICE_NOT_CONFIGURED`** | 설정 누락을 로그에서 401 과 구별 | 공식 예제처럼 401 |
| 24 | 판정 실패와 제출 | **제출은 성공, 판정 실패는 로그만(같은 요청의 후처리, 같은 트랜잭션 아님)** | 기록 원칙(모든 판 저장). 재전송·`dan-recompute` 로 다시 판정 가능 | 같은 트랜잭션(판정 실패가 제출 실패) |
| 25 | 원본 요청 방식 | **표를 차례로, User-Agent `R-BMS table sync (+https://bms.hyuns.uk)`, 리다이렉트 안 따름** | 원본 부담 최소(하루 4요청), 다른 호스트로 새지 않음 | 병렬 요청 |
| 26 | 원격 표 읽기 캐시 | **인스턴스 메모리 캐시(버전 행으로 갈아탐)** | `'use cache'` 에 약 1 MB 항목을 넣는 한계를 확인하지 못했다(9절 R6). 버전 조회는 작은 행 하나 | `'use cache'` + `revalidateTag` |

---

## 9. 남은 위험과 테스트 부채 후보

**위험**

- R1. 웹 배포보다 먼저 나간 클라이언트(또는 옛 클라이언트)의 코스 판은 `stages` 가 저장되지 않아 단위가 생기지 않고 소급도 안 된다(5.2절 8). 클라이언트 릴리스는 웹 배포 뒤로.
- R2. Stella 가 Cloudflare 뒤라 Vercel(icn1) 서버의 요청이 봇 검사에 걸릴 수 있다(확인 못함 — 운영 사이트·원본에 요청을 보내지 않았다). 걸리면 `failed:http_status` 가 매일 남는다. 첫 실행 로그로 확인하고, 막히면 GitHub Actions 일정 실행(`refresh-tables.ts`)으로 옮기는 것을 다시 묻는다.
- R3. 차트 자체의 `#LNMODE` 로 LN 종류가 고정된 곡이 LN 코스에 있으면 `lntype` 이 0 이 아니어서 단위가 나오지 않는다(F5 R5). GENOSIDE·Stella 곡의 `#LNMODE` 사용 여부는 확인 못함. 실기에서 `ln_mode_mismatch` 가 보이면 "제약이 정한 값 또는 차트가 정한 값" 을 받도록 본문에 차트 고정 여부를 싣는 후속이 필요하다.
- R4. Vercel 요금제를 모른다. Hobby 면 실행이 KST 0:00~0:59 사이로 흩어지고(2.8절), 함수 시간 제한은 요금제·Fluid 설정에 따른다(`maxDuration = 60` 이 허용되는지 확인 못함).
- R5. 검증 상수(10%·25%)는 관찰 1회분(2026-10-23 사본)과 원본 안내(2주 반영)에 기댄 값이다. 큰 개편(레벨 재배치)이 오면 Cron 이 거절하고 사람이 `tables-refresh-force` 를 돌려야 한다.
- R6. 원격 표의 메모리 캐시는 인스턴스마다 첫 읽기에 약 0.9 MB 를 풀고 파싱한다. 인스턴스가 많으면 DB 에서 blob 을 여러 번 읽는다(하루 내용이 바뀌지 않으면 인스턴스당 1회).
- R7. GENOSIDE 코스 파일·Stella 표 JSON 의 재배포 허락 문구를 찾지 못했다(조사 A-3·B-3). 서버는 정의(md5·이름·제약)와 표 메타만 보관하고 곡 파일은 없다. 요청이 오면 지울 수 있게 출처 행 단위로 모였다.
- R8. 판정 의미가 지금과 바뀌는 E3(SCRATCH AUTO 코스 순위에 등급)·E1(게이지 모드 무관 단위)은 결정 6·20 의 직접 결과지만 실제 순위 변화는 사용자 실기로만 확인된다.
- R9. 같은 웨이브의 F7(곡 미러) M2 가 `schema.ts`·`env.ts`·`compose/index.ts` 를 만진다 → F6 다음 번호(0013)로 직렬.

**테스트 부채 후보(`docs/quality-assurance/2026-10-14-test-debt.md`)**

- T1. 옛 판(stages 없음)의 단위 미부여. 재현: 0012 이전에 GENOSIDE 初段 을 클리어한 사용자. 필요해지는 때: 사용자가 옛 기록의 단위를 원할 때. 방안: 램프만으로 "제약 미확인" 단위를 따로 표시.
- T2. 트로피(은·금 메달)를 계산·표시하지 않는다. 필요해지는 때: 트로피 순위·표시를 원할 때.
- T3. 차트 제목 대체·검색·AI·패턴 coverage·차트 상세의 소속 표는 insane1 만 본다(3.6절 10). 필요해지는 때: Stella 곡을 검색·AI 에서 표 기준으로 볼 때.
- T4. 순위에 게이지 모드 표시 없음. 필요해지는 때: 모드별 비교를 원할 때(IR 와이어 변경 필요).
- T5. Cron 전달 누락(공식 문서 "best effort")에 대한 알림이 없다. 다음 날 실행이 따라잡는다. 필요해지는 때: 이틀 이상 `checked_at` 이 멈출 때 — 표 페이지에 "마지막 확인" 을 보이므로 눈으로 발견 가능.
- T6. DP 단위의 FLIP·DP 제약 원천(E7). 필요해지는 때: DP 실기 보고.
- T7. `user_dan` 은 처음 획득만 남겨, 뒤에 더 높은 게이지(EX-HARD)로 다시 깬 기록을 단위에 보이지 않는다. 필요해지는 때: 단위의 "최고 램프" 표시를 원할 때.
