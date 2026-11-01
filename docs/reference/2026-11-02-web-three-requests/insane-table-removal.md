> 최종 갱신 2026-11-02 · 대응 커밋 `28085a7`·`8af1a39`·`f737c4d`(조사 시점 `837d652`, 구현은 이 작업) · 조사 wf_db20f1ba-f79

# 조사 보고서: 발광표(insane1) ★10 "PopCoRian -EX-" 를 표에서 영구히 빼려면

읽기 전용으로 조사했습니다. 파일 수정, git 변경, 네트워크, DB 접속은 하지 않았습니다. 작업 트리는 깨끗했습니다. 표기는 "확인"(파일에서 읽음)과 "추측"(코드에서 읽을 수 없어 추론)으로 구분합니다.

## 구현 결과 (2026-11-02, 커밋 `28085a7`)

조사 뒤 사용자가 8절 결정 1 에서 **B안(표 사본 + 운영 기록까지 삭제)** 을 골랐고(원문: "표 + 운영 기록까지 삭제"), 나머지는 추천안대로 구현됐다. 결정 기록: [PopCoRian -EX- ADR](../../acknowledge/2026-11-02-adr-popcorian-ex-removed.md).

- 3.2절과 6절의 "삭제 후" 값은 **구현 결과로 확정**됐다: 1,034 → 1,033곡, ★10 64 → 63곡, 레벨 26개 그대로, 새 `data.json` sha256 `afdb520ce15343394848add244c47a78d9e2e08e9252c90a21a82456b75a31e9`(`source.json` 의 `data_file_sha256`·`entries`·`entries_with_valid_md5`·`method.normalization` 을 맞춤, `http.data[0].entries` 는 그대로).
- 7절 목록 중 바뀐 파일: `web/data/tables/insane1/{data,source}.json`, `web/scripts/snapshot-table.ts`(`ENTRY_MD5_REMOVED` 가 SABOTHER `f8b3b8ca…` 와 이 곡 `4c2a85ef…` 두 개), `web/tests/{service,route}/table-snapshot.test.ts`, `apps/rbms-player/src/tablesrc/seed.rs`, `crates/rbms-table/{tests/insane_snapshot.rs,src/fetch/tests.rs}`.
- 8절 결정 1 의 B안에 따라 5절의 "chart-purge 는 SABOTHER 고정" 이 해소됐다: `web/scripts/chart-purge.{count,delete}.sql` 을 두 md5 대상으로 일반화했고(`md5 IN (…)`, AI 결과 검색에 md5·제목 추가, SABOTHER 는 이미 0행이라 다시 돌려도 무해) `.github/workflows/web-db.yml` 의 액션 설명문을 "charts" 로 고쳤다. 새 액션은 만들지 않았다.
- 운영 삭제는 **아직 실행하지 않았다**: `chart-purge-count` → 사용자 확인 → 사용자 `!` `chart-purge-delete`. 10절의 "운영 DB 에 이 md5 의 행이 있는지" 는 count 결과로 확인된다.
- 문서 쪽: `docs/backend/api-spec.md`·`docs/ci-release.md`·`docs/architecture.md` 의 곡 수를 1,033 으로, `docs/web/architecture.md` §1-3 `/records` 설명의 표 곡 수도 1,033 으로 고쳤다.
- 검증(실제 결과): 웹 `bun test` 2,004 통과, `cargo test -p rbms-table` 93 통과, `rbms-player` `tablesrc::seed` 7 통과.

## 0. 결론 요약

1. **확인**: 웹은 `web/data/tables/insane1/data.json` 파일 하나가 단일 출처입니다. 화면·API·AI·폴더·차트 상세가 모두 이 파일에서 나옵니다. DB 의 `table_snapshot` 에는 insane1 이 들어가지 않습니다.
2. **확인**: 원본에서 다시 받아 덮어쓰는 자동 경로는 insane1 에 없습니다. 크론은 Stella·Satellite 두 표만 받고, `snapshot-table.ts` 는 수동 스크립트입니다.
3. **확인**: 같은 종류의 선례가 있습니다. 커밋 `a6e7ffe` (SABOTHER, md5 `f8b3b8ca…`)가 "스냅샷 재생성 시 되살아나지 않게 하는 제외 목록 `ENTRY_MD5_REMOVED`" 를 이미 만들어 두었습니다. 이번에는 md5 한 개를 그 목록에 더하면 됩니다.
4. **확인**: 파일을 바꾸면 웹·Rust 시험 일부가 깨집니다. `1,034` 와 `64곡` 을 고정한 단언들이라 같은 커밋에서 같이 고쳐야 합니다.
5. **확인**: 운영 DB 반영은 필요 없습니다. 표 사본은 배포되는 파일이라 `web-db.yml` 액션이 필요 없습니다.
6. **확인**: 클라이언트(Rust)도 같은 파일을 `include_bytes!` 로 실행 파일에 내장합니다. 웹만 바꿔도 파일이 하나라 저장소 안에서는 어긋나지 않습니다. 다만 이미 배포된 v0.1.10 바이너리의 내장 사본에는 다음 릴리스 전까지 남습니다.

## 1. insane1 이 서빙되는 모든 경로 (확인)

### 1.1 파일 → 메모리

- `web/src/server/service/domain/table-snapshot/table-snapshot.data.ts:1-3` 가 `data/tables/insane1/{data,header,source}.json` 을 정적 import 합니다. 빌드에 번들되는 방식입니다.
- `table-snapshot.data.ts:30-32` 의 `RAW_SNAPSHOTS` 에 insane1 한 개만 있습니다.
- `table-snapshot.data.ts:44-61` 의 `getSnapshotTables()` 가 첫 호출에서 Zod 로 파싱해 `Map` 에 캐시합니다. `entries` 는 `snapshotEntriesSchema.parse(raw.entries)`, `standardData` 는 같은 배열을 `JSON.stringify` 한 것(`serialiseOnce`, 34-40행)입니다.
- `table-snapshot.data.ts:63` 의 `getSnapshotTable(tableId)` 와 `:67-77` 의 `findSnapshotTextByMd5(md5)` 가 있습니다. 후자는 모든 번들 표 항목의 md5 → `{title, artist}` 색인을 만듭니다.

### 1.2 번들 표 + 원격 표 합치기

- `web/src/server/compose/table-snapshot.ts:106-108` 의 `createTableLoader` 가 `new Map([...getSnapshotTables(), ...원격표])` 를 만듭니다. 원격 표는 `REMOTE_TABLES` 의 두 개(`stella-st`, `stella-sl`)뿐이라 insane1 을 덮어쓸 수 없습니다.
- `web/src/server/service/domain/table-refresh/table-sources.ts:25-50` 의 `REMOTE_TABLES` 는 `stella-st`, `stella-sl` 두 개뿐입니다.

### 1.3 HTTP 경로

- 표준 형식 `GET /tables/insane1/header.json` 과 `/tables/insane1/data.json` 은 `web/src/app/tables/[tableId]/{header.json,data.json}/route.ts` 가 `createTableSnapshotRoutes()` 를 호출합니다.
- `web/src/server/route/table-snapshot.ts:21-22` 의 캐시는 `STATIC_JSON_CACHE = 'public, max-age=300, s-maxage=3600, stale-while-revalidate=86400'`, `STANDARD_FORMAT_CACHE = 'public, max-age=3600, s-maxage=3600, stale-while-revalidate=86400'` 입니다.
- 같은 파일 `:101-103` 에서 ETag 를 쓰고 `If-None-Match` 가 같으면 304 를 냅니다. ETag 값은 `source.json` 의 `snapshot.data_file_sha256` 앞 32자입니다 (`table-snapshot.service.ts:174-177`).
- 이 ETag 때문에 **`source.json` 의 `data_file_sha256` 을 새 `data.json` 의 sha256 으로 바꾸지 않으면 CDN·브라우저 캐시가 304 로 옛 본문을 유지합니다.** 이 위험이 이 작업의 핵심입니다. 이전 선례도 `source.json` 해시를 같이 바꿨습니다.
- `GET /api/fe/tables`(목록)와 `/api/fe/tables/[tableId]`(상세·`entries`·`records`)는 `web/src/app/api/fe/tables/**` 가 같은 서비스를 씁니다.
- `web/src/app/api/tables/route.ts`, `web/src/app/api/tables/[tableId]/route.ts` 는 IR 호환 경로입니다. 이쪽은 DB `difficulty_table` 계열을 읽고 파일을 읽지 않습니다 (1.5 참고).

### 1.4 화면

- `web/src/app/(app)/tables/insane1/page.tsx:6-17` 는 `TABLE_ID='insane1'` 로 `TableSnapshotContent` 를 렌더하고, `bmstable` 메타는 `/tables/insane1/header.json` 입니다.
- `web/src/widgets/table/table-snapshot-content.tsx:19-21` 은 `getSnapshotTable(tableId)` 가 있으면 `groupTableByLevel(...)` 로 바로 그립니다 (번들 표는 DB 를 안 탑니다).
- 설명문의 곡 수와 레벨 범위는 `table.entry_count`, `levels[0]`, `levels[last]` 에서 계산됩니다 (`:27, :32`). 하드코딩이 없어서 파일이 바뀌면 자동 갱신됩니다.
- 레벨별 곡 수는 `summariseTable` (`table-snapshot.service.ts:73-90`) 이 `entries` 를 세어 만듭니다. 레벨 10 은 이 항목이 빠지면 자동으로 63곡이 됩니다.

### 1.5 DB 테이블 (`web/src/server/db/schema.ts`) 중 insane1 이 들어가는 곳

- `table_snapshot` (schema.ts:640-653)
  - 확인: 쓰는 곳은 `web/src/server/compose/table-refresh.ts` 의 저장소(`createTableSnapshotStore`)뿐이고, 그 대상은 `REMOTE_TABLES` 의 두 id 입니다. insane1 행은 만들어지지 않습니다.
- `difficulty_table` (schema.ts:415-421), `table_folder` (:422-431), `table_chart` (:433-440), `table_course` (:442-450)
  - 확인: 이 넷은 IR 호환의 관리자 업서트 경로(`web/src/server/route/table.ts:20-32`, `withAdmin`, `tableService.upsert`)로만 채워집니다.
  - 확인: 번들 표 ADR 이 "DB(`table_chart`)에 넣지 않고 파일을 직접 읽는다" 고 정했습니다 (`docs/acknowledge/2026-10-14-adr-default-insane-table.md` 결정 2).
  - 확인: SABOTHER 삭제 때 운영에서 `table_chart` 포함 "표·코스·내 폴더 항목·AI 결과는 0행" 이었습니다 (`docs/acknowledge/2026-10-23-adr-chart-removed-at-owner-request.md` 결정 2).
  - 추측: 운영 DB 를 직접 보지 못했으므로 이번 md5 의 `table_chart` 행이 0이라는 것은 같은 구조에서 나온 추정입니다. 확인하려면 `web-db.yml` 에 개수 액션이 필요합니다 (5절).
- 코스: md5 `4c2a85ef…` 와 `PopCoRian` 은 `git grep` 으로 `data.json` 밖에서 0건입니다 (`web/data/courses` 포함). 코스 정의와 무관합니다.

### 1.6 파일에서 파생되는 소비처 (모두 `getSnapshotTables`/`findSnapshotTextByMd5` 경유, 별도 수정 불필요)

- `web/src/server/compose/index.ts:122, 376, 402` 는 `findSnapshotTextByMd5` 로 차트 서비스, 폴더, FE 의 곡 제목 대체 조회에 씁니다.
- 같은 파일 `:379-382` 는 `listCoverageTables` 로 곡별 패턴 커버리지(`md5s`)를 만듭니다. 이 md5 가 목록에서 빠집니다.
- 같은 파일 `:403` 은 `searchTableMd5s` 로 `searchMd5sInTables` 를 씁니다. 검색 결과에서 빠집니다.
- 같은 파일 `:410, 420, 428` 은 AI 레코드 쿼리, 점수 특징, 플레이 특징의 `loadTables: getSnapshotTables` 입니다. AI 후보에서 자동으로 빠집니다.
- `web/src/widgets/folder/read-folder-table-index.ts:6-18` 은 폴더 편집기의 표 색인(`groupTableByLevel` 결과를 `{md5,title,artist}` 로 줄인 것)입니다. 폴더 가져오기 후보에서 자동으로 빠집니다.
- `web/src/widgets/chart/chart-detail-content.tsx:17-32` 의 `findTableMemberships(md5)` 는 차트 상세의 "속한 표·레벨" 배지입니다. 이 md5 의 차트 상세에서 배지가 사라집니다.
- `web/src/server/service/domain/folder/folder.service.ts:423-` 의 `importFromTable` 은 `loadEntriesForImport` (`table-snapshot.service.ts:184-188`)를 씁니다. 표에서 폴더로 가져오기에서 빠집니다.

## 2. 원격 원본에서 다시 받아 덮어쓰는 경로 (확인)

- `web/vercel.json` 의 크론은 `{ "path": "/api/cron/tables", "schedule": "0 15 * * *" }` 하나입니다. 이 크론은 `REMOTE_TABLES` 두 표(stellabms.xyz)만 받습니다 (`docs/acknowledge/2026-10-23-adr-table-daily-refresh.md` 결정 2).
- `web/scripts/refresh-tables.ts` 는 같은 갱신을 러너에서 하는 도구이며 대상은 같습니다. insane1 언급이 0건입니다 (`grep` 확인).
- `web/scripts/snapshot-table.ts` 는 **수동 실행** 스크립트입니다 (`bun scripts/snapshot-table.ts <표 주소> <출력 폴더> [--check]`).
  - `package.json` 스크립트, `.github/workflows`, 시험(`web/tests`) 어디에도 이 스크립트를 부르는 곳이 없습니다 (`git grep snapshot-table` 확인).
  - 그래서 자동으로 되살아나지 않습니다.
  - 누군가 수동으로 다시 돌리면 원본에 이 곡이 있으므로 **제외 목록에 넣지 않으면 되살아납니다.**
- 기존 장치 (확인):
  - `snapshot-table.ts:23` `const ENTRY_MD5_REMOVED: readonly string[] = ['f8b3b8ca5b5c5f7be1352b7aae723bee']`.
  - `:110` `isKeptEntry` 가 필터하고, `:150` `collect` 에서 `.filter(isKeptEntry)` 로 적용합니다.
  - `:206` 에서 `source.json` 의 `method.normalization` 마지막 줄에 `data.json: entries whose md5 is ${ENTRY_MD5_REMOVED.join(', ')} are left out (removed at the site owner's request)` 가 자동 기록됩니다. 배열을 여러 값으로 늘려도 같은 줄에 쉼표로 이어집니다.
  - `--check` (`:226-253`)도 같은 `collect` 를 쓰므로 제외 목록에 넣으면 "원본에서 그 곡만 빠진 것과 같다"로 일치로 판정합니다.
- 주의 (확인): 위 문구는 "removed at the site owner's request" 로 고정되어 있습니다. 이번 삭제도 사이트 주인(사용자)의 지시라 문구는 맞습니다.

## 3. 무결성 장치와 시험이 고정하는 값 (확인)

### 3.1 현재 값

- `data.json` 은 1034 항목이고 sha256 은 `fd87c00a607b89b2564f60bd93364d9def5875c5614e10a33c2ff5c0b6b85572` 입니다 (`shasum -a 256`). `source.json:46-52` 의 `snapshot.data_file_sha256` 와 일치합니다.
- `header.json` sha256 은 `c5beb338…ad5` 이며 `source.json` 의 `header_file_sha256` 와 일치합니다. header 는 건드릴 필요가 없습니다. `level_order` 가 26개(`1`~`25`, `???`)이고 레벨 10 이 비지 않아서 그대로입니다.
- 대상 항목은 `data.json` 의 309행(배열 인덱스 307, 파일 첫 줄 `[` 때문에 행 번호 = 인덱스+2)입니다. 정확히 1건입니다.
  - 내용: `level "10"`, `title "PopCoRian -EX-"`, `md5 4c2a85efe95c8ee2121effaa4627a0cf`, `lr2_bmsid "1120"`, `artist "なおきち / obj:arufis"`.
  - 필드: `url` 은 web.archive.org 주소로 비어 있지 않고, `url_diff` 와 `name_diff` 는 `http://bms.nothing.sh/player/arufis` 입니다.

### 3.2 삭제 뒤 값 (파이썬으로 계산함)

| 항목 | 현재 | 삭제 후 |
| --- | --- | --- |
| 총 곡 수 | 1034 | 1033 |
| 레벨 10 곡 수 | 64 | 63 |
| 레벨 수 | 26 | 26 (그대로) |
| `url` 이 비지 않은 곡 | 1032 | 1031 |
| 첫 항목 md5 | `0077fe17…` | 그대로 |

- `source.json` 의 `snapshot.entries` / `entries_with_valid_md5` 는 1034 → 1033 입니다.
- `source.json` 의 `snapshot.levels` 는 26 그대로입니다.
- `source.json` 의 `http.data[0].entries: 1035` 는 원본 응답의 수라 **그대로 둡니다.** 선례 커밋도 그대로 두었습니다.
- 두 번째 계산 후 `data_file_sha256` 은 새 `data.json` 의 sha256 으로 갱신해야 합니다.

### 3.3 웹 시험 중 값이 고정된 곳

- `web/tests/service/table-snapshot.test.ts`
  - `:14` `ENTRY_COUNT = 1034` → `1033`
  - `:16` `DOWNLOAD_LINK_COUNT = 1032` → `1031` (이 항목의 `url` 이 비어 있지 않아서)
  - `:39` 시험 제목 `1034곡` 과 `:40-43` 본문은 상수를 쓰므로 제목만 갱신합니다.
  - `:69` 제목 `... 합계가 1034 이다` 는 제목만 갱신하고, `:72` `expect(counts.get('10')).toBe(64)` → `63` (★25 가 2곡, `???` 가 5곡은 불변)
  - `:219` 제목 `1034곡 전부 기록이 있어도 ...` 와 `:237` 제목 `1034곡 전체 상세가 ...` 은 제목만 갱신합니다.
  - `:132` ETag 시험은 `source.json` 의 해시를 읽어 비교하므로 `source.json` 이 새 해시면 통과합니다.
  - 해시를 못 맞추면 이 시험이 실패하는 것이 아니라(자기 참조), 캐시 위험이 조용히 남는 점이 중요합니다.
- `web/tests/route/table-snapshot.test.ts`
  - `:69` 제목 `1034곡 배열...` 과 `:76` `toHaveLength(1034)` → `1033`
  - `:114` `entry_count: 1034` → `1033` (`level_count: 26` 은 그대로)
- 그 밖의 `insane1` 사용 시험은 모두 가짜 데이터나 `entries[0]` 만 씁니다 (`folder.test.ts`, `folders.test.ts`, `chart-pattern.test.ts`, `record-query*.test.ts`, `score-table-features.test.ts`, `ai-score-fixture.ts`, `assist-grade.test.ts:217-218` 의 `entries[0]` 은 `0077fe17…` 라 불변). 수정 불필요입니다.
- `PopCoRian`·`4c2a85ef` 를 직접 쓰는 시험이나 문서는 없습니다 (`git grep` 0건).
- 스크립트 `snapshot-table.ts` 를 직접 시험하는 파일은 없습니다 (`web/tests/scripts/` 에 없음).

### 3.4 Rust 시험·내장

- `apps/rbms-player/src/tablesrc/seed.rs:35-38` 이 `header.json` 과 `data.json` 을 `include_bytes!(…/../../web/data/tables/insane1/…)` 로 내장합니다. 같은 파일을 읽으므로 **저장소 안에서는 웹과 클라이언트가 어긋날 수 없습니다.**
- 단언 위치는 다음과 같습니다.
  - `seed.rs:126` `(1_034, 26, 0)` → `(1_033, 26, 0)`
  - `seed.rs:180` `entries: 1_034` → `1_033`
  - `crates/rbms-table/tests/insane_snapshot.rs:11` `const CHARTS: usize = 1_034` → `1_033`
  - `crates/rbms-table/src/fetch/tests.rs:261` 과 `:268` 의 `entries.len(), 1_034` 두 곳 → `1_033`
- `crates/rbms-config/src/tables.rs`, `apps/rbms-player/src/stage/{tables,select/tree_tests}.rs`, `tablesrc.rs` 의 `insane1` 은 id·주소 문자열이고 곡 수를 고정하지 않습니다. 수정 불필요입니다.
- `.github/workflows/ci.yml:8-17` 은 `web/data/tables/**` 가 바뀌면 Rust CI 를 돌립니다. 즉 이 변경은 **웹 CI(`web.yml`)와 Rust CI(`ci.yml`) 둘 다** 돕니다.
- 이미 배포된 클라이언트는 실행 파일 안의 옛 사본을 갖고 있습니다. 다만 아키텍처 문서에 따르면 등록 주소가 `https://bms.hyuns.uk/tables/insane1` 이고 하루 이상 지난 저장본이나 내장 사본(날짜 없음)은 실행마다 한 번 백그라운드로 다시 받습니다 (`docs/architecture.md` "The default tables" 절, `COPY_STALE_AFTER_SECS`). 그래서 웹 배포 후 하루 안팎에 기존 사용자 PC 의 저장본도 갱신될 것으로 읽힙니다.
  - 추측: 정확한 갱신 시점은 코드 `tablesrc.rs` 를 추적하지 않았습니다. 확인된 것은 문서의 서술입니다.

## 4. 같은 md5 가 다른 곳에서 레벨 표시에 쓰이는가 (확인)

- 코스: 쓰이지 않습니다 (`web/data/courses` 및 전체 `git grep` 0건).
- AI 후보·곡별 패턴·폴더 색인·차트 상세 배지: 모두 `getSnapshotTables()` 에서 파생되므로 파일에서 빠지면 일관되게 빠집니다 (1.6).
- 사용자 폴더(`user_folder_item`)는 자체 `md5`·`title` 을 들고 있어 폴더 목록은 계속 보입니다. 표 색인에서 빠지는 것은 "가져오기 후보" 뿐입니다. DB 에 이 md5 를 담은 사용자 폴더가 있을 수 있으나 코드로는 알 수 없습니다 (추측).
- 차트 서비스(`lookupTableText`): 이미 DB 에 있는 `chart` 행의 제목이 비어 있을 때만 표 제목으로 대체하는 용도입니다. 삭제 후에는 해당 md5 의 제목 대체가 null 이 됩니다. 영향은 제목이 비어 있는 차트 행이 있을 때만 나타납니다 (코드를 끝까지 추적하지는 않았습니다. 추측).

## 5. 운영 DB 반영이 필요한가 (확인 + 추측)

- 필요 없음 (확인): 표 사본은 배포 번들입니다. Vercel 배포가 끝나면 반영됩니다. `ignoreCommand` 가 `web/` 이 바뀐 `dev` 푸시에서 프로덕션 빌드를 만듭니다 (`web/vercel.json`).
- `table_snapshot` 에 insane1 행이 없으므로 `tables-refresh-*` 액션은 무관합니다.
- `.github/workflows/web-db.yml` 액션 목록: `status, migrate, cleanup-count, cleanup-delete, chart-notes-count, chart-notes-repair, chart-notes-restore, chart-notes-drop-backup, admin-seed-count, assist-grade-backfill-count, assist-grade-backfill, chart-purge-count, chart-purge-delete, courses-seed-count, courses-seed, tables-refresh-count, tables-refresh-force, dan-recompute-count, dan-recompute`
- `chart-purge-*`(`web/scripts/chart-purge.{count,delete}.sql`)는 **SABOTHER md5 가 SQL 에 고정**되어 있습니다. 그 SQL 은 `chart`, `score`, `chart_best`, `replay`, `play_trace`, `play_analysis`, `submission_audit`, `chart_pattern`, `table_chart`, `user_folder_item`, `ai_result` 를 지웁니다.
  - 이번 지시는 "표에서 난이도를 지워" 라서 점수·리플레이 같은 기록까지 지우는 일이 아닙니다. 쓰면 사용자 점수 기록을 파괴하므로 사용자가 범위를 정하지 않은 채 쓰면 안 됩니다. 파괴 작업은 `!` 직접 실행이 필요합니다 (메모리: `production-actions-need-user-run`).
- 캐시: 번들 파일 기반이라 새 배포가 새 빌드를 만듭니다. `s-maxage=3600` 은 배포 후 최대 1시간(SWR 하루) 이전 응답이 남을 수 있지만, ETag 가 바뀌어 재검증하면 새 본문이 갑니다. 그래서 `source.json` 해시 갱신이 필수입니다.

## 6. 지우면 함께 사라지는 것의 개수

| 대상 | 변화 |
| --- | --- |
| `data.json` 항목 | 1034 → 1033 (1건) |
| ★10 곡 수 | 64 → 63 (다운로드 링크가 있는 곡 1032 → 1031) |
| 레벨 수 | 26 (변화 없음, ★10 이 비지 않음) |
| `header.json` | 변화 없음 |
| `source.json` | `data_file_sha256`, `entries`, `entries_with_valid_md5`, `normalization` 한 줄 |
| `table_snapshot` / `table_course` 등 DB 행 | 0 (insane1 이 DB 에 없음. `table_chart` 는 추측으로 0) |
| 클라이언트 내장 사본 | 다음 클라이언트 빌드부터 1033 (v0.1.10 바이너리에는 남음) |

## 7. 구현 시 수정할 파일 목록 (파일별 변경 요지)

필수 (선례 `a6e7ffe` 와 같은 모양):

1. `web/data/tables/insane1/data.json`: 309행(`md5 4c2a85ef…`) 한 줄 삭제. 앞 줄 끝의 `,` 는 유지합니다 (중간 행이라 쉼표 규칙 불변). 가능하면 손수정 대신 아래 스크립트 수정 후 재생성 대신, 한 줄 삭제 후 `shasum` 으로 확인하는 것이 안전합니다 (재생성은 원본 네트워크가 필요하고 `fetched_at` 등이 바뀝니다).
2. `web/data/tables/insane1/source.json`: `snapshot.data_file_sha256` 을 새 sha256 으로, `entries` 와 `entries_with_valid_md5` 를 1033 으로, `method.normalization` 마지막 줄을 `entries whose md5 is f8b3b8ca5b5c5f7be1352b7aae723bee, 4c2a85efe95c8ee2121effaa4627a0cf are left out (...)` 형태로 맞춥니다.
3. `web/scripts/snapshot-table.ts:23`: `ENTRY_MD5_REMOVED` 에 `'4c2a85efe95c8ee2121effaa4627a0cf'` 추가. 이것이 재생성 시 부활 방지 장치입니다.
4. `web/tests/service/table-snapshot.test.ts`: 3.3 의 상수·단언(`ENTRY_COUNT` 1033, `DOWNLOAD_LINK_COUNT` 1031, `counts.get('10')` 63)과 제목 문구.
5. `web/tests/route/table-snapshot.test.ts`: `:69-76` 과 `:114` 의 1034 → 1033.
6. `apps/rbms-player/src/tablesrc/seed.rs`: `:126` 과 `:180` 의 `1_034` → `1_033`.
7. `crates/rbms-table/tests/insane_snapshot.rs:11` 와 `crates/rbms-table/src/fetch/tests.rs:261, 268`: `1_034` → `1_033`.

문서 (코드 규칙상 어긋난 기록은 갱신):

8. `docs/backend/api-spec.md:345` (`1,034곡 — 사이트 주인의 요청으로 한 곡을 뺐다` → 두 곡, 1,033곡)와 `docs/ci-release.md:231` (`사이트 주인의 요청으로 뺀 차트의 md5 한 개 ... 1,034곡` → 두 개, 1,033곡).
9. 새 ADR (`docs/acknowledge/2026-11-02-adr-...`): 사용자 원문, 결정, "남는 것"(v0.1.10 내장 사본). `docs/acknowledge/README.md` 색인 한 줄, `docs/PROCESS.md` 체크리스트·기록. `docs/acknowledge/2026-10-23-adr-chart-removed-at-owner-request.md` 에는 이 작업과의 관계를 한 줄 더하는 것을 권합니다.

선택 (결정 필요): DB 기록 정리용 SQL 신규. 아래 결정 1 참고.

## 8. 사용자 결정이 필요한 지점

1. **삭제 범위** (이 지시를 가장 크게 가르는 지점)
   - **A안(추천): 표 사본에서만 뺀다.**
     - 근거: 지시 원문이 "표에서 지워" 입니다.
     - SABOTHER 때는 사용자가 범위 질문에 "웹의 모든 흔적" 을 직접 골라서 기록·리플레이 삭제까지 간 것이라 그 선례를 자동으로 확장하면 안 됩니다.
     - 점수 기록은 되돌릴 수 없어서 더 보수적인 쪽이 안전합니다.
   - B안: 표 사본 + 운영 DB 의 그 차트 행 전부 삭제. `chart-purge.*.sql` 을 md5 매개변수 형태로 일반화하거나 신규 SQL 이 필요하고, `web-db` 액션 추가와 사용자 `!` 실행이 필요합니다.
2. **v0.1.10 내장 사본**
   - **A안(추천): 그대로 두고 다음 릴리스에 자연히 반영.** 선례도 그렇게 처리했고, 실행 시 웹에서 다시 받아 갱신됩니다 (3.4).
   - B안: 별도 클라이언트 핫픽스 릴리스.
3. **제외 사유 문구**
   - **A안(추천): 기존 고정 문구 "removed at the site owner's request" 를 그대로 재사용.** 사용자가 곧 사이트 주인이고 지시도 본인 요청입니다.
   - B안: md5 별 사유를 구분하도록 구조 변경. 요청 범위를 넘는 리팩터링이라 비추천.
4. **커밋 단위** (확인 후 결정, 컨벤션상 메인이 정함)
   - 웹 시험과 Rust 시험이 같은 데이터 변경에 묶이므로 **한 커밋(`feat(web)`/`fix(web)` 스타일, 한국어 설명)** 이 되돌리기 쉬운 단위입니다. 선례 `a6e7ffe` 도 한 커밋이었습니다.
   - 나머지는 문서 커밋을 분리해도 됩니다.

## 9. 최소 검증 명령 (실제 `package.json` 과 CI 설정 확인함)

웹 (`web/` 에서, `package.json` 스크립트 `test` = `bun test`):

1. 가장 직접적인 검증: `cd web && bun test tests/service/table-snapshot.test.ts tests/route/table-snapshot.test.ts`
   - 곡 수, ★10 수, 링크 수, ETag 가 새 `source.json` 해시와 일치함을 덮습니다.
2. 해시 일치를 별도로 눈으로 확인: `shasum -a 256 web/data/tables/insane1/data.json` 의 출력이 `source.json` 의 `data_file_sha256` 과 같아야 합니다 (시험이 자기 참조라 잡지 못합니다).
3. 삭제 확인: `grep -c 4c2a85efe95c8ee2121effaa4627a0cf web/data/tables/insane1/data.json` 이 0 이어야 합니다.
4. 문법 확인: 변경한 시험 파일만 `bun run format:check` (prettier 로 `web/` 전체를 보므로 필요 시 변경 파일에 `bunx prettier --check <파일>`). 전체 `bun run verify`(typecheck, lint, test)는 변경이 파일 삭제와 숫자 상수라 필수는 아니지만 CI 의 `web.yml` 이 `typecheck`, `lint`, `test`, `format:check`, `build` 를 돌립니다.

Rust (저장소 루트, `docs/development.md` 와 `ci.yml` 의 게이트 명령은 `cargo test --workspace --no-fail-fast --locked`):

5. 좁은 검증: `cargo test -p rbms-table --locked` (`insane_snapshot.rs`, `fetch/tests.rs` 포함)와 `cargo test -p rbms-player --locked --lib every_carried_snapshot_reads a_first_run_has_the_table_and_its_folders_without_the_network`
   - 시험 이름은 `apps/rbms-player/src/tablesrc/seed.rs:116, 162` 에서 확인했습니다.
   - 실제 `cargo` 필터 문법은 위 이름을 부분 문자열로 받으므로 두 번 나눠 실행해도 됩니다. 이 두 이름을 한 번에 넘기는 방식은 `cargo test` 가 하나의 필터만 받는지 확인하지 못했습니다. 안전하게 한 번에 하나씩 실행하는 것을 권합니다.
6. 푸시 뒤: `gh` 로 3개 OS 의 `ci` 와 `web` 워크플로 결과를 확인합니다 (메모리: `check-ci-after-push`). 로컬 게이트만으로 완료 보고하지 않습니다.

운영 확인 (사용자 지시가 있을 때만, 읽기 요청): 배포 후 `https://bms.hyuns.uk/tables/insane1/data.json` 의 배열 길이가 1033 이고 `4c2a85ef…` 가 없는지입니다. 이번 조사에서는 네트워크를 쓰지 않아 실행하지 않았습니다.

## 10. 남은 위험과 확인하지 못한 것

- 운영 DB 에 이 md5 의 `table_chart`, `user_folder_item`, `chart_pattern`, `ai_result` 행이 있는지는 코드로 알 수 없습니다 (추측: `table_chart` 는 0). 필요하면 개수 액션이 먼저 필요합니다.
- 클라이언트의 정확한 재다운로드 시점은 `tablesrc.rs` 를 끝까지 추적하지 않았습니다. 문서의 서술을 근거로 썼습니다.
- `chart.lookupTableText` 의 실제 영향 범위는 코드를 끝까지 따라가지 않았습니다 (4절).
- `.claude/worktrees/wf_ab1b5d4e-9fd-*` 네 곳에 같은 `data.json` 사본이 있습니다. 이는 작업 트리 사본이라 이번 변경의 대상이 아니며, 메인 저장소의 `web/data/tables/insane1/` 만 고치면 됩니다.

주요 파일 경로 (절대):
- `/Users/hyunseokbyun/development/R-BMS/web/data/tables/insane1/data.json`
- `/Users/hyunseokbyun/development/R-BMS/web/data/tables/insane1/source.json`
- `/Users/hyunseokbyun/development/R-BMS/web/scripts/snapshot-table.ts`
- `/Users/hyunseokbyun/development/R-BMS/web/src/server/service/domain/table-snapshot/table-snapshot.data.ts`
- `/Users/hyunseokbyun/development/R-BMS/apps/rbms-player/src/tablesrc/seed.rs`
- `/Users/hyunseokbyun/development/R-BMS/docs/acknowledge/2026-10-23-adr-chart-removed-at-owner-request.md`