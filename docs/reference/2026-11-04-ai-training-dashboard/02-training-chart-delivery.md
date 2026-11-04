# 훈련곡 전달 경로 조사 - 웹 생성, 내 폴더, 구동기에서 바로 받아 플레이

> 조사일 2026-11-04 · 브랜치 dev · HEAD `4bac699` · 코드·설정은 수정하지 않았고 커밋·푸시도 하지 않았다. `.env*` 와 비밀 파일은 읽지 않았다.
> 근거 표기는 `경로:줄` 이고 `web/` 안 파일은 `web/` 를 붙였다. 확인한 사실과 추측을 갈라 적었고, 확인하지 못한 것은 8절에 모았다.
> 이 조사는 `docs/reference/ai-features/` 와 `docs/plan/2026-10-23-select-tree-download.md` 등 이미 저장된 조사를 먼저 읽고, 훈련곡 전달에 필요한 부분만 원본 코드로 다시 검증했다. 어긋난 곳은 해당 절에 적었다.

사용자 결정(2026-11-04): 생성은 LLM 이 패턴 구성안을 만들고 서버가 BMS 로 변환한다(보완으로 결정론 분석 결과를 입력). 산출물은 앱에서 바로 플레이 가능한 곡이고, 플레이 결과는 다시 분석에 반영한다. 사용자 폴더에 "훈련곡" 이 들어간다. 곡 미러(저작권 곡 재배포)는 보류 상태이지만 훈련곡은 서버가 만든 합성 차트라 그 보류와는 별개로 다룰 수 있다(이 문장은 추정이며, 권리 판단이 아니라 곡의 출처가 다르다는 뜻이다).

---

## 0. 먼저 읽을 결론

1. **서버는 이미 "폴더에 md5 를 넣고 클라이언트가 그 폴더를 받는" 길이 있다.** 폴더 항목의 식별자는 md5(소문자 hex 32자)이고 sha256 은 비워도 된다(`web/src/server/dto/folder.ts:32-38,60-66`). 클라이언트는 `GET /api/players/[playerId]/folders[/folderId]` 로 받아 저장한다(`web/src/server/route/folder.ts:94-107,191-198`).
2. **그런데 클라이언트는 폴더에 있어도 라이브러리에 없는 곡은 줄로 보여 주지 않는다.** `(가진 곡/전체)` 숫자만 보인다(`apps/rbms-player/src/user_folders.rs:152-190`, `docs/plan/2026-10-14-web-user-folders.md:790`, `docs/plan/2026-10-23-select-tree-download.md:22`). 그래서 "내 폴더에서 바로 받기" 는 지금 클라이언트로는 되지 않는다.
3. **"받아서 풀어 라이브러리에 넣는" 클라이언트 기계는 이미 구현돼 있다**(보류 아님). 직접 zip 링크를 받아 풀고, 그 폴더만 부분 스캔하고, 표와 내 폴더를 다시 맞춘다(`apps/rbms-player/src/download.rs:312-332,447-455,483-508`, `crates/rbms-download/src/*`). 다만 입구가 난이도표의 `TableEntry` 하나뿐이다(`apps/rbms-player/src/stage/select/download_modal.rs:141-143`, `apps/rbms-player/src/tablesrc.rs:1087-1089`).
4. **서버는 파일을 서빙한 적이 없다.** `Content-Disposition`·`application/zip` 은 `web/src` 에 0건이다. 저장 매체는 MySQL 블롭이 선례(`chart_pattern.data` mediumblob, `play_trace.data` mediumblob)이고 Vercel Blob 은 설치돼 있지 않다. 1분 곡은 BMS 5.7 KB, zip 5.3 KB 수준이라(직접 측정, 8절 부록) 새 인프라도 새 의존성도 필요 없다.
5. **zip 은 TypeScript 로 손으로 써도 된다.** 직접 만든 zip(deflate 한 BMS + 저장 방식 wav, UTF-8 이름 플래그, 데이터 기술자 없음)이 `rbms_download::download_with` 와 `extract_zip` 을 통과해 `written 2, charts 1` 로 풀렸고 풀린 md5 가 원본과 같았다(직접 실행, 부록 B).
6. **URL 은 경로가 `.zip` 으로 끝나야 하고 인증 헤더를 못 보낸다.** `classify_link` 는 경로(쿼리 제외)만 본다(`crates/rbms-download/src/link.rs:32-39`)이고 `download_to` 는 헤더 없이 GET 한다(`crates/rbms-download/src/fetch.rs:85-91`). 그래서 예컨대 `/api/training-charts/<md5>/chart.zip` 이 공개 GET 이거나 서명 쿼리를 붙인 주소여야 한다(서명 쿼리를 붙여도 `DirectZip` 으로 판정됨을 직접 확인).
7. **BMS 최소 요건은 사실상 없다.** 파서는 실패하지 않는다(`crates/rbms-parser/src/lib.rs:135-192`). 7키 판정은 채널 18 또는 19 를 한 번이라도 써야 한다(`crates/rbms-chart/src/lib.rs:38-60`). 키음(`#WAV`)이 없어도 플레이 경로는 키음 0개로 진행한다(코드 읽기, 구동은 하지 않음).
8. **TypeScript 로 만든 BMS 를 같은 파서로 검증하는 길이 있다.** `rbms-cli <chart>` 요약 모드와 `rbms-cli chart-pattern <폴더> --out <폴더>` 가 그 길이고, 후자의 출력이 웹 `chartPatternDocumentSchema` 와 같은 모양이다(`apps/rbms-cli/src/chart_pattern/document.rs:17-20`, `web/src/server/dto/chart-pattern.ts:80-120`). BPM 160·16분 격자와 BPM 175·12분 격자 두 경우 모두 TS 로 계산한 노트 시각 568개가 Rust 출력과 전부 일치했다(부록 A).
9. **지금 서버에 훈련곡을 구별하는 장치가 없다.** 모르는 md5 의 점수도 `chart` 행을 자동으로 만들고 `ranked` 로 받아 `chart_best`·랭킹·통계에 들어간다(`web/src/server/route/score.ts:81-106`, `web/src/server/service/domain/chart/chart.service.ts:148-196`, `web/src/server/service/domain/score/score.service.ts:334-422`). 훈련곡 표(마이그레이션 0014)와 `TRAINING` 판정이 필요하고, `score` 를 읽는 공개·AI 쿼리 여러 곳에 제외 조건이 필요하다(6.3절).
10. **클라이언트(Rust) 변경이 꼭 필요한 것은 "내 폴더에서 바로 받기" 한 가지뿐이다.** 서버 쪽 산출(zip 엔드포인트, 훈련곡 표, 폴더 항목의 `download_url`)은 어느 경로에서도 같고, 클라이언트 없이 되는 대체 경로가 둘 있다(7.3절).

---

## 1. 사용자 폴더 (서버)

### 1.1 저장 구조와 식별자

| 항목 | 값 | 근거 |
|---|---|---|
| 폴더 표 `user_folder` | `id varchar(36)`(`uf_` + 소문자 hex 30자), `user_id` FK cascade, `name varchar(64)`, `description varchar(255)`, `is_public` 항상 false, `position`, `item_count`, `created_at`, `updated_at bigint`(ms). unique `(user_id, name)`, 색인 `(user_id, position)` | `web/src/server/db/schema.ts:480-499`, `web/src/server/dto/folder.ts:21-24` |
| 항목 표 `user_folder_item` | PK `(folder_id, md5)`, `md5 char(32)`, `sha256 char(64)` NULL 가능, `title varchar(200)`, `artist varchar(200)`, `note varchar(100)`, `position`. 색인 `(folder_id, position)`, `(md5)` | `web/src/server/db/schema.ts:501-519` |
| 곡 식별자 | **md5**. 입력은 공백 제거·소문자화 뒤 `isMd5Hash`. `sha256` 은 `''` 또는 64자 hex, 기본 `''` | `web/src/server/dto/folder.ts:32-38,60-66`, `web/src/server/dto/common.ts:93-95` |
| 한 폴더 안 중복 | md5 로 중복 제거(`uniqueByMd5`) | `web/src/server/service/domain/folder/folder.service.ts:159-166` |
| 항목 글자 채우기 | sha256 은 `chart` 행 > 입력 > null, title·artist 는 `chart` 행 > 난이도표 글자 > 입력 순으로 비지 않은 첫 값, 제어문자 제거 뒤 200자로 자름 | `folder.service.ts:131-132,202-212`, `web/src/server/compose/folder.ts:256-263` |

### 1.2 한도 `FOLDER_LIMIT`

`web/src/server/dto/folder.ts:6-19`: 사용자당 폴더 50, 폴더당 2,000곡, 사용자 전체 20,000곡, 요청당 500곡, 이름 64자, 설명 255자, 메모(`note`) 100자, 복사 글자 200자, 쓰기 본문 512 KiB(`WRITE_MAX_BYTES`, 같은 파일 4행). 이름 중복은 `name_taken`, 한도 초과는 `limit_exceeded`(`folder.service.ts:112-123`).

### 1.3 클라이언트로 내려가는 길

- 읽기 라우트: `GET /api/players/[playerId]/folders`(색인)와 `.../folders/[folderId]`(항목 포함). 봉투 없음, Bearer 또는 세션, 본인만(`user.loginId === playerId`, 아니면 403), `Cache-Control: private, no-store`, 읽기 120/분(`web/src/server/route/folder.ts:41,94-107,191-198`, `web/src/server/lib/with-rate-limit.ts:9-11`). 라우트 어댑터는 `web/src/app/api/players/[playerId]/folders/route.ts:3`.
- 응답 모양: 색인 `{ folders: [{ id, name, description, is_public, position, item_count, updated_at }] }`, 폴더 `...summary + items: [{ md5, sha256, title, artist, note }]`(`web/src/server/dto/folder.ts:105-123`, 계약 문서 `docs/plan/2026-10-14-web-user-folders.md:652-697`).
- 클라이언트가 받는 때: 저장된 토큰으로 시작할 때, 로그인할 때, 선곡 F6, Tables 화면의 R/A(`apps/rbms-player/src/app_network.rs:455,483,519`, `apps/rbms-player/src/stage/tables.rs:386,390`, `apps/rbms-player/src/stage/select/mod.rs:1139`). 색인을 받아 저장본과 `updated_at` 이 다른 폴더만 한 번에 최대 4개씩 받는다(`apps/rbms-player/src/user_folders.rs:233-260`).
- 저장본: `<설정 폴더>/user-folders/<해시>.ron`, 최대 32 MiB(`user_folders.rs:44,68`). 클라이언트도 폴더 50개·폴더당 2,000곡을 넘는 부분은 버리고, md5 모양이 아닌 항목은 버린다(`user_folders.rs:54-55,201-206`).
- 항목 DTO 는 알 수 없는 필드를 무시한다(`crates/rbms-ir/src/dto/folder.rs:26-39` 는 `deny_unknown_fields` 가 없고 나머지는 `#[serde(default)]`). 그래서 서버가 항목에 선택 필드(예: `download_url`)를 더해도 옛 클라이언트는 깨지지 않는다(추정이 아니라 DTO 정의에서 읽은 사실이다. 실제 옛 빌드로 시험하지는 않았다).
- 클라이언트 화면: 뿌리 `MY FOLDERS (n)` 아래 폴더, 폴더 아래 **가진 곡만** 줄이다. 폴더 줄은 `{이름} ({가진 곡}/{전체})`(`user_folders.rs:168-170`, `apps/rbms-player/src/stage/select/list.rs:376-390`). 가진 곡은 `Library::indices_for_md5` 로 맞춘다(`user_folders.rs:174-190`). 없는 곡 줄은 표 레벨 안에만 있다(`list.rs:405-441`).

### 1.4 AI 연습 폴더가 이미 하는 일 (`practice-folder.service.ts`)

- 상수: `AI_PRACTICE_LIMIT` = 항목 최대 30, 이름 접두사 `'AI 연습 · '`, 이름 접미사 40자, 메모 100자, 설명 255자, 쓰기 본문 32 KiB(`web/src/server/dto/ai.ts:351-358`). 기능 이름 `practice_folder`, 프롬프트 버전 2, 출력 토큰 8,000, effort `low`(`web/src/server/service/domain/ai/practice-folder.service.ts:42-47`).
- 흐름: `preview` 는 후보를 서버가 정하고 LLM 이 그 안에서 고른다(`runAiFeature`, `practice-folder.service.ts:252-267`). 폴더는 사용자가 만들기를 눌러야 생긴다. `create` 는 확인된 항목을 **후보 안의 md5 만** 남겨 `{ md5, sha256: '', title, artist, note }` 로 만들고(`:278-294`), 이름은 `접두사 + 접미사` 를 NFC 로 정규화한다(`:296`). 같은 이름이면 ` YYYY-MM-DD`(UTC)를 붙여 다시 시도한다(`:216-221`). 같은 미리보기를 다시 누르거나 동시에 눌러도 폴더를 하나만 만들도록 `ai_practice_folder`(PK `(user_id, preview_hash)`, `web/src/server/db/schema.ts:670-683`)로 잇고 `folders.replaceItems` 로 채운다(`:298-335`).
- 즉 지금 AI 폴더의 곡은 모두 **이미 서버가 아는 공개 난이도표 곡**이다(`refinePracticeOutput` 이 후보에 없는 md5 를 버린다, `:118-136`). 훈련곡은 서버가 만든 새 md5 이므로 이 검증을 훈련곡 표 조회로 넓혀야 한다.

---

## 2. 클라이언트가 곡을 받아서 플레이하는 기존 경로

### 2.1 상태 확인

| 기능 | 상태 | 근거 |
|---|---|---|
| 선곡 폴더 트리, 없는 곡 흐린 막대, 모달, 직접 zip 받기, 부분 스캔 | **구현됨** | `docs/acknowledge/2026-10-23-adr-missing-charts-and-download.md`, `apps/rbms-player/src/download.rs`, `crates/rbms-download` |
| 곡 미러(저작권 곡을 우리 쪽에 올려 받기) | **보류**. 업로드·서버·버튼을 만들지 않는다 | `docs/HANDOFF.md` 6절 2번, `docs/acknowledge/README.md:155`, `docs/acknowledge/2026-10-23-adr-song-mirror-prep.md` |
| MY FOLDERS 안의 없는 곡 줄 | **없음**(설계가 표 레벨 안만 다룸) | `docs/plan/2026-10-23-select-tree-download.md:22` |

`select-tree-download` 설계의 구현 단위는 코드에 모두 들어 있다(`apps/rbms-player/src/{download.rs,open_url.rs,select_tree.rs}`, `.../stage/select/download_modal.rs`, `crates/rbms-download/`).

### 2.2 받기·풀기·넣기의 실제 동작

1. **입구**: `start_download(shared, &TableEntry)` 는 (a) 다른 일이 없고 (b) `classify_link(&entry.url_diff) == DirectZip` 이고 (c) `config.library.download_folder` 가 있어야 시작한다(`apps/rbms-player/src/download.rs:312-332`). 다운로드 폴더가 없으면 모달의 [GET CHART] 가 폴더 고르기 창을 먼저 연다(`docs/plan/2026-10-23-select-tree-download.md:359-365`).
2. **링크 판정** `classify_link`: http·https 와 호스트가 있어야 하고, `stellabms.xyz/upload/…` 이거나 **경로(쿼리 제외)가 대소문자 무시 `.zip` 으로 끝나면** `DirectZip`, 그 밖은 `Browser`, 웹 주소가 아니면 `Unusable`(`crates/rbms-download/src/link.rs:32-39`). 직접 확인한 값: `…/<md5>.zip` → `DirectZip`, `…/<md5>/download` → `Browser`, `…?file=a.zip` → `Browser`, `…/chart.zip?exp=1&sig=abc` → `DirectZip`.
3. **받기** `download_to`: 헤더 없는 GET, User-Agent `rbms-download`, 리다이렉트 10회·http/https 만, 최대 512 MiB(`DOWNLOAD_MAX_BYTES`), 무응답 30초, 전체 15분, 응답의 첫 4바이트가 `PK\x03\x04`(빈 보관은 `PK\x05\x06`)가 아니면 `NotAZip`(`crates/rbms-download/src/fetch.rs:13-33,85-104,189-193`). `Content-Type`·`Content-Disposition` 은 보지 않는다.
4. **풀기** `extract_zip`: 항목 10,000개, 파일당 1 GiB, 전체 2 GiB 상한(`crates/rbms-download/src/extract.rs:15-21`). 남기는 확장자는 차트 `bms bme bml pms bmson`, 소리 `wav ogg mp3 flac`, 그림, 영상, `txt` 뿐이다(`extract.rs:34-36`). 차트가 하나도 없으면 `NoChart` 로 실패한다(`extract.rs:264-275`). 이름은 `/`·`\` 로 나눈 조각마다 검사하고 Shift_JIS 이름도 처리한다(`extract.rs:77-108`, `crates/rbms-download/src/name.rs`).
5. **놓기** `settle`: `{다운로드 폴더}/{safe_folder_name(제목, md5)}` 로 옮긴다. 폴더 이름은 `제목(금지문자 `_` 치환, 64자) [md5 앞 8자]`, 같은 이름이 있으면 ` (2)`… 를 붙이고 덮어쓰지 않는다(`name.rs:68-100`). 작업 폴더 `{다운로드 폴더}/.rbms-partial` 은 스캔이 들어가지 않는다(`crates/rbms-library/src/scan.rs:58,205-209`). 직접 확인: `safe_folder_name("Training Demo", "5bf1c9f8…")` = `Training Demo [5bf1c9f8]`.
6. **스캔·맞추기**: 풀린 폴더만 `ScanRequest::partial` 로 스캔하고 표를 다시 맞춘 뒤 선곡이 라이브러리를 교체한다. 이때 `user_folders::rematch` 를 불러 내 폴더도 다시 맞춘다(`download.rs:447-455,472-479`). 즉 **받은 곡은 곧바로 해당 내 폴더의 가진 곡이 된다.**
7. **라이브러리 범위**: 스캔 대상은 곡 폴더들 + 다운로드 폴더다(`LibraryOptions::scan_roots`, `crates/rbms-config/src/schema.rs:956-968`). 차트 파일 판정은 확장자 `bms bme bml pms bmson`(`crates/rbms-library/src/lib.rs:67-69`).
8. **md5 매칭**: 라이브러리 행의 md5 는 차트 파일 **원 바이트**의 md5 이고(`crates/rbms-parser/src/lib.rs:136`, `crates/rbms-library/src/scan.rs:340-343`), 폴더 항목 md5 와 `Library::indices_for_md5` 로 맞는다.

### 2.3 폴더에 있는데 로컬에 없는 곡의 지금 처리

- 줄로 나오지 않고 `(가진 곡/전체)` 로만 보인다(1.3절).
- 표 레벨 안의 없는 곡은 `MissingRef { table, level, entry }` 로 가리키고 `TableExtras.missing` 에서 `TableEntry` 를 꺼낸다. 이 구조가 유일한 `SelectItem::Missing` 생산자이고(`apps/rbms-player/src/stage/select/list.rs:405-441`, `apps/rbms-player/src/lib.rs:478-503`) 모달도 거기서만 열린다(`download_modal.rs:141-143,303-322`).
- 모달 문구는 난이도표 전제다("THE TABLE'S DL IS THE CHART FILE ONLY…", `docs/acknowledge/2026-10-23-adr-missing-charts-and-download.md` 2)항). 훈련곡에는 그대로 맞지 않아 문구 분기가 필요하다.

---

## 3. 서버가 BMS(+키음)를 호스팅·서빙할 수 있는가

### 3.1 지금 있는 것과 없는 것

| 항목 | 사실 | 근거 |
|---|---|---|
| 파일을 내려주는 라우트 | **없음**. `Content-Disposition`·`application/zip` 0건 | `grep` 결과, `web/src` |
| 저장 선례(DB 블롭) | `chart_pattern.data` mediumblob(gzip JSON), `play_trace.data` mediumblob | `web/src/server/db/schema.ts:618-638,685-`, `web/drizzle/0007_chart_pattern.sql`, `web/src/server/service/shared/storage/gzip-json.ts:1-17` |
| 리플레이 | MySQL `replay.data` mediumtext 인라인. `REPLAY_STORAGE=blob` 은 자리만 있고 compose 가 던짐 | `web/src/server/service/shared/storage/replay-storage.ts:12-16`, `web/src/server/lib/env.ts:20-21`, `docs/reference/ai-features/server.md:79-80` |
| 외부 저장소 | 없음. `@vercel/blob` 미설치, `BLOB_READ_WRITE_TOKEN` 은 선언만 | `web/package.json:20-45`, `env.ts:21` |
| zip 라이브러리 | 웹에 **없음**. `node:zlib` 은 이미 쓴다 | `web/package.json`, `gzip-json.ts:1-2` |
| 함수 한도 | 요청·응답 본문 4.5 MB, 기본 최대 실행 300초. 플랜(Hobby/Pro)은 확인 못함. AI 라우트는 `maxDuration = 300` | `docs/reference/ai-features/server.md:223-226`, `web/src/app/api/fe/me/ai/practice-folder/preview/route.ts:3` |
| 라우트 보호 | `/api/*` 는 `proxy.ts` matcher 밖이다(세션 리다이렉트 없음) | `web/src/proxy.ts:36-47` |
| 레이트리밋 | 인스턴스 로컬 `Map` 이라 다중 인스턴스에서 best-effort | `web/src/server/lib/with-rate-limit.ts:19-26` |
| 절대 주소 | `NEXT_PUBLIC_APP_URL`(기본 `http://localhost:3000`) | `web/src/server/lib/env.ts:15-16` |

### 3.2 저장 매체와 크기

- 훈련곡 1곡은 BMS 텍스트(직접 측정: 40마디, 노트 이벤트 568개, **5,702 바이트**)와 선택적 메트로놈 wav(44.1 kHz 모노 50 ms 클릭, 4,454 바이트)뿐이다. 이를 deflate/저장 혼합 zip 으로 묶어도 **5,285 바이트**였다(부록 B). Vercel 본문 4.5 MB 한도에 비해 3자리 이상 작다.
- 권고(추정): 저장은 DB 에 BMS 텍스트를 gzip 해 `mediumblob` 으로 넣고 요청 때 zip 을 조립한다. 내용이 md5 로 정해지므로 응답은 `Cache-Control: public, max-age=31536000, immutable` 로 둘 수 있다. wav 는 고정 상수라 코드에서 생성할 수 있다(PCM 헤더 44바이트 + 샘플).
- **새 인프라는 필요 없다.** 새로 필요한 것: DB 표 하나(0014), zip 조립 코드(약 70줄 규모, 부록 B 에서 같은 구성을 직접 만들어 검증), 폴더 항목의 선택 필드. R2·Blob·미러는 쓰지 않는다.
- 새 의존성은 필요 없다. CRC-32 는 직접 구현한다(`node:zlib.crc32` 는 로컬 Node v24 에는 있으나 Vercel 런타임 버전과 `@types/node ^20` 타입에 있는지 확인하지 못해 쓰지 않는 쪽을 권한다).

### 3.3 URL 설계 제약

- 클라이언트는 **경로가 `.zip` 으로 끝나는 주소**를 `DirectZip` 으로 본다(2.2절 2). 쿼리 문자열은 판정에 영향이 없고, 서명 쿼리(`?exp=…&sig=…`)를 붙여도 `DirectZip` 이다.
- Next.js 의 동적 세그먼트는 세그먼트 전체여야 하므로 `[md5].zip` 형태는 쓰지 않는 쪽이 안전하다(이 문장은 App Router 일반 규칙에 대한 추정이고 이 저장소의 Next 16.3.4 문서로 다시 확인하지 않았다). 예: `/api/training-charts/[md5]/chart.zip`.
- 인증을 못 붙이므로 선택지는 둘이다. (가) md5 만 아는 사람이 받는 공개 GET(md5 는 128비트 해시라 추측이 어렵고, 내용은 서버가 만든 합성 차트라 저작권·비밀이 아니다 - 추정), (나) 폴더 항목의 `download_url` 에 서버가 만료·서명 쿼리를 붙여 내려 준다(새 시크릿 환경변수가 필요하고 `getEnv()` 패턴을 따라야 한다, `docs/reference/ai-features/server.md:201`).

---

## 4. BMS 형식 최소 요건 (파서 소스에서 확인한 사실만)

### 4.1 파서가 요구하는 것

| 항목 | 사실 | 근거 |
|---|---|---|
| 필수 헤더 | **없다.** `parse` 는 어떤 바이트든 `BmsSource` 를 돌려준다. 기본값: `#PLAYER 1`, `#BPM 130`, `#RANK 3`, `#LNTYPE 1`, `#LNMODE 0`, 나머지 빈 문자열 | `crates/rbms-parser/src/lib.rs:84-109,135-192` |
| 제목 | `#TITLE` 이 비면 라이브러리는 파일 이름을 제목으로 쓴다 | `crates/rbms-library/src/scan.rs:338` |
| 데이터 줄 | `#` 뒤 `MMMCC:` 형식. 마디 번호 **십진 3자리(0~999)**, 채널 두 글자(36진), 여섯 번째 글자가 `:` | `crates/rbms-parser/src/lib.rs:220-227` |
| 한 줄의 토큰 | 글자 2개 = 토큰 1개, 토큰 개수 N 이면 위치는 i/N. 토큰 `00` 은 건너뜀. 개수 제한 없음 | `lib.rs:243-263` |
| 마디 길이 | 채널 `02` 에 양의 실수 | `lib.rs:233-241` |
| `#BPM` | `#BPM <수>`, 파싱 실패 시 130 | `lib.rs:294` |
| `#WAVxx` | 선택. 파일 이름 문자열만 저장, 파서는 파일을 열지 않는다 | `lib.rs:309-312` |
| 인코딩 | 올바른 UTF-8 이면 UTF-8 로 읽는다(BOM 있어도 없어도). 그렇지 않으면 Shift_JIS 나 EUC-KR 추정 | `crates/rbms-parser/src/encoding.rs:2-14,77-89` |
| 해시 | md5·sha256 은 **파일 원 바이트**에 대한 해시, 정규화 없음 | `crates/rbms-parser/src/lib.rs:135-137` |
| 줄 끝 | `\n`, `\r`, `\r\n` 모두 줄 끝 | `lib.rs:200-202` |

주의: 같은 레인에 같은 위치의 노트를 두 번 쓰면 나중 것이 덮는다(`crates/rbms-chart/src/lib.rs:322-323`). 생성기는 레인·시각 충돌을 직접 막아야 한다.

### 4.2 7키 + 스크래치 채널 (1P)

모드는 파일 확장자와 사용 채널로 정해진다(`crates/rbms-chart/src/lib.rs:38-60`): `.pms` 는 9키, P2 채널(`21`~`29`, 36진 값 73~81)이 하나라도 있으면 14키, 아니면 채널 `18`·`19`(값 44·45) 중 하나라도 있으면 **7키**, 둘 다 없으면 **5키**. 직접 확인: 채널 11·15·16 만 쓴 차트는 `BEAT_5K`, 18 을 한 번 쓴 차트는 `BEAT_7K`.

| 레인(웹 패턴 문서 기준) | 건반 | 일반 노트 | LN | 지뢰 | 숨김 |
|---|---|---|---|---|---|
| 0 | 1 | `11` | `51` | `D1` | `31` |
| 1 | 2 | `12` | `52` | `D2` | `32` |
| 2 | 3 | `13` | `53` | `D3` | `33` |
| 3 | 4 | `14` | `54` | `D4` | `34` |
| 4 | 5 | `15` | `55` | `D5` | `35` |
| 5 | 6 | `18` | `58` | `D8` | `38` |
| 6 | 7 | `19` | `59` | `D9` | `39` |
| 7 | 스크래치 | `16` | `56` | `D6` | `36` |
| (무시) | 자유 구역 | `17` | `57` | | |

근거: 채널→레인 표 `BEAT7 = [0,1,2,3,4,7,-1,5,6,…]`(`crates/rbms-model/src/mode.rs:13,23`)와 채널 역할 분기(`crates/rbms-chart/src/lib.rs:78-99`). 레인 번호 0~6 = 건반 1~7, 7 = 스크래치는 웹 패턴 문서와 같다(`web/src/server/dto/chart-pattern.ts:8-10`). 그 밖: BGM `01`, 인라인 BPM `03`(16진 두 자리), 참조 BPM `08`(+`#BPMxx`), 정지 `09`(+`#STOPxx`), 스크롤 `SC`(+`#SCROLLxx`), 마디 길이 `02`.

### 4.3 롱노트 표기

- **채널 5x 쌍**: 같은 레인에서 첫 토큰이 머리, 다음 토큰이 끝, 이후 반복(`crates/rbms-chart/src/lib.rs:329-349`). 예: `#00151:01000001`(`apps/rbms-cli/src/chart_pattern/tests.rs:20`).
- **`#LNOBJ xx`**: 일반 채널의 토큰 값이 `xx` 이고 같은 레인에 직전 일반 노트가 있으면 그 노트가 LN 머리, 이 토큰이 끝이 된다(헤더 읽기 `crates/rbms-parser/src/lib.rs:297-302`, 변환 `crates/rbms-chart/src/lib.rs:306-321`). 직접 확인: `#LNOBJ 02` + `#00111:01000200` 이 LN 으로 읽힘.
- **`#LNTYPE` 은 파싱만 하고 어디서도 읽지 않는다**(`crates/rbms-parser/src/lib.rs:295` 한 줄뿐, `rbms-chart`·`rbms-library`·`rbms-judge`·`rbms-play` 에서 `grep` 0건). 무시해도 되는 헤더다.
- **LN 종류**는 `#LNMODE`(1 LN, 2 CN, 3 HCN)로 정해지고 없으면 `Undefined` 라서 플레이어의 LN MODE 설정이 정한다(`crates/rbms-chart/src/lib.rs:120-130,146-152`). 직접 확인: `#LNMODE 1` 이 있으면 패턴 문서에 `declared_ln: "ln"`, `has_undefined_ln: false`, 없으면 `declared_ln: null`, `has_undefined_ln: true`. 훈련곡은 통제를 위해 `#LNMODE 1` 을 고정하는 쪽을 권한다(결정 사항, 8절).
- 노트 수 계산: LN 끝은 CN·HCN 일 때만 센다, 지뢰는 세지 않는다(`crates/rbms-model/src/lib.rs:172-194`). 직접 확인: 노트 이벤트 568개(LN 끝 16개 포함)가 `notes : 552` 로 나온다.

### 4.4 키음 없이 플레이할 수 있는가

- 키음 경로는 `#WAV` 이름을 곡 폴더 기준으로 찾고, 확장자가 달라도 `ogg wav flac mp3` 순으로 찾는다. **못 찾은 항목은 조용히 버린다**(`apps/rbms-player/src/assets.rs:107-137`). `#WAV` 가 하나도 없으면 디코드 일이 0개이고 키음 적재를 띄우지 않는다(`apps/rbms-player/src/app_play.rs:564-571`). 그래서 `#WAV` 없이도 플레이 경로가 막히지 않는다는 것이 코드 읽기로 확인한 사실이다. **플레이어를 실제로 구동해 보지는 않았다**(조사 규칙).
- 라이브러리는 이런 곡에 `CONTENT_NO_KEYSOUND` 비트를 켠다. 표시용 정보일 뿐이다(`crates/rbms-library/src/scan.rs:481-497`, 상수 `crates/rbms-library/src/songdb/rows.rs:36-43`).
- 박자 단서가 필요하면 메트로놈을 BGM 채널 `01` 에 `#WAV01 click.wav` 로 놓고 wav 를 zip 에 넣는다. wav 확장자는 풀기 허용 목록에 있다(`extract.rs:35-36`). 직접 만든 zip 에서 `click.wav` 가 그대로 풀렸다.

### 4.5 한도와 1분 곡 구성

- 파서·모델·판정에는 노트 수·마디 수 상한이 없다. 마디 번호가 3자리라 최대 마디는 999, 시간 표 기반 계산은 86,400초까지다(`crates/rbms-chart/src/lib.rs:26`). `grep MAX_` 로 `rbms-model/parser/chart/judge` 를 확인했다.
- 서버 쪽 상한: 패턴 문서 노트 200,000개·24시간(`web/src/server/dto/chart-pattern.ts:23-27`), 점수 제출 본문 256 KB(`web/src/server/route/score.ts:16`), 판별 trace 본문 2 MiB(`web/src/server/dto/trace.ts:12`). 1분 곡(수백 노트)은 모두 한참 아래다.
- 시간 계산: 마디 하나(길이 1.0)는 `240,000,000 / BPM` 마이크로초이고, 타임라인마다 `time_us = 앞 time_us + 앞 stop_us + trunc(240_000_000 * (section - 앞 section) / 앞 bpm)` 로 **누적 절사**한다(`crates/rbms-chart/src/lib.rs:353-386`). 그래서 1분 = **BPM / 4 마디**다.
  - BPM 120 → 30마디(마디당 2.0초), BPM 150 → 37.5마디(1.6초), **BPM 160 → 40마디(1.5초)**, BPM 180 → 45마디(1.333초).
  - 직접 측정한 예(BPM 160, 40마디, 16분 격자): 파일 5,702 B, 노트 이벤트 568 / 플레이 노트 552, 마지막 노트 59,625,000 µs, `length : 59.6s`(부록 A).
- 라이브러리의 "길이" 는 마지막 타임라인의 시각이다(`crates/rbms-library/src/scan.rs:334-335`). 마디 시작마다 구분선 타임라인이 있으므로 마지막 마디의 시작 이후 노트가 길이를 정한다.

---

## 5. 기존 샘플·도구, 생성한 BMS 를 같은 파서로 검증하는 방법

### 5.1 저장소에 있는 것

| 위치 | 내용 | 훈련곡에 쓸 수 있는가 |
|---|---|---|
| `samples/preview-demo/` | `preview-demo.bms`, `autoplay-demo.bms`(헤더 8줄 + 데이터 1줄), `preview.wav`(6,444 B) | 형식 예시일 뿐, 생성기가 아님 |
| `tools/` | `gen-skin-property.rs`, `gen-skin-timer.rs` 뿐 | 아니오 |
| `crates/*` | BMS **쓰는** 코드는 없다(파서·모델·변환만) | 아니오 |
| `apps/rbms-cli` | 하위 명령: `<chart>` 요약, `scan`, `config`, `scores`, `skin-dump`, `chart-pattern`, `course-snapshot`, `mirror-pack` | 검증용으로 예(5.2) |
| `web/` | BMS 를 쓰는 TS 코드 없음. `#TITLE` 은 `web/src`·`web/scripts`·`web/tests` 에 0건 | 새로 써야 함 |

`rbms-cli` 사용법 상수는 `apps/rbms-cli/src/main.rs:25-36`, 요약 모드 `chart_command` 는 `:303-352`. `mirror-pack` 은 곡 폴더를 결정적 zip 으로 묶는다(`zip::ZipWriter`, `apps/rbms-cli/src/mirror_pack/pack.rs:14,89-106`). 훈련곡 zip 을 서버가 직접 조립하므로 이 도구는 필요 없다.

### 5.2 검증 명령 (직접 실행해 확인)

1. `rbms-cli <file.bms>`: md5, sha256, `#WAV` 정의 수, 마디 수, 타임라인 수, 모드, 노트 수, 길이, NOTES RADAR 를 출력한다. 읽기 실패와 bmson 오류가 아니면 항상 성공 코드라서 **BMS 문법 오류를 실패로 알려 주지는 않는다**(파서가 실패하지 않기 때문).
2. `rbms-cli chart-pattern <폴더> --out <폴더>`: `<md5>.json` 을 쓴다(기본 `--mode beat-7k`, 다른 모드 차트는 건너뜀). 문서 형식 `rbms-chart-pattern` v1: `notes.{time_us,lane,kind,section}`, `tempo[]`, `declared_ln`, `has_undefined_ln`, `playable_notes` 등(`apps/rbms-cli/src/chart_pattern/document.rs:17-20,88-160`). 웹이 받는 스키마와 같다(`web/src/server/dto/chart-pattern.ts:80-120`).
3. 비교 결과(부록 A): 두 구성에서 TS 가 계산한 `time_us`·`lane`·`kind` 배열이 Rust 출력과 **전부 같았다**(568개, 마지막 시각 59,625,000 / 54,399,754). Node `createHash('md5')` 값이 Rust 가 낸 md5 와 같았다.
4. 웹 CI 는 Rust 를 돌리지 않는다(`.github/workflows/web.yml:43-60` 은 `bun` 단계뿐). 따라서 TS 변환기의 정확성을 지키려면 `contract/client-requests/` 와 같은 방식의 골든 픽스처(BMS 텍스트, 기대 md5, 기대 패턴 문서)를 저장소에 두고 TS 시험과 Rust 시험이 같은 파일을 읽게 하는 편이 맞다(제안, `docs/HANDOFF.md` 7절의 `contract` 게이트 선례).

### 5.3 변환기가 지켜야 할 것 (확인한 사실에서 나온 제안)

- 생성한 바이트를 **그대로** 해시하고 그 바이트를 그대로 zip 에 넣는다(재인코딩 금지). 같은 텍스트를 UTF-8 로 한 번만 만든다.
- 시각 계산을 TS 로 복제할 때는 Rust 와 같은 순서로 `double` 연산을 한다: 타임라인 구간마다 `Math.trunc(240_000_000 * (section - previous) / bpm)` 를 누적. BPM 175 + 12분 격자처럼 마이크로초가 정수가 아닌 경우도 일치했다(부록 A).
- 웹 패턴 문서를 훈련곡에서도 만들어 `chart_pattern` 에 넣으면(`chartPatternService.upload`, `web/src/server/service/domain/chart-pattern/chart-pattern.service.ts:151`) 기존 패턴 분류(`PATTERN_TAGS`, `web/src/server/service/domain/analysis/pattern-tags.ts:33-52`)와 trace 조인(`joinPattern`, `web/src/server/service/domain/trace/trace-analysis.ts:90`)이 그대로 돈다. 조인은 시각 다중집합이 정확히 같아야 `matched` 이다(같은 파일, 90-100행).

---

## 6. 플레이 결과 회수와 오염 방지

### 6.1 지금 서버가 모르는 md5 의 점수를 받는 방식

1. 클라이언트는 점수 본문에 `chart.md5`, `chart.sha256`, 제목·아티스트·레벨을 싣는다. 설명 주석: "the server knows a chart only once somebody has played it"(`crates/rbms-ir/src/dto/score.rs:7-52`).
2. 서버 `POST /api/scores`: 본문 256 KB, 해시 검증, 정합 검사 뒤 `chartService.ensureFromSubmission` 이 **`chart` 행을 자동으로 만든다**(없고 sha256 이 있을 때). sha256 이 비었을 때만 `IR_CHART_NOT_FOUND`(`web/src/server/route/score.ts:67-95`, `web/src/server/service/domain/chart/chart.service.ts:148-196`). `score.chart_sha256` 은 `chart` 에 대한 FK 가 없다(`web/src/server/db/schema.ts:59-`).
3. `scoreService.submit`: `rankedPolicy` 가 옵션·빌드·게스트로 `ranked` 와 `flags` 를 정한다(`web/src/server/service/domain/score/ranked-policy.ts:61-74`, `score.service.ts:334-339`). `score` 행은 항상 삽입하고(`:392-406`), **`ranked` 이면 `chart_best` 를 갱신**한다(`:408-422`). 훈련곡이라는 개념이 없으므로 훈련곡 판도 일반 판과 똑같이 랭킹·`chart_best` 에 들어간다.
4. 클라이언트의 반응: 서버가 보낸 flag 중 이 빌드가 아는 것만 `known_flags` 로 읽고(`crates/rbms-ir/src/dto/score.rs:361-363`), 순위를 막는 flag 는 `Autoplay Assist JudgeWidth TotalOverride Guest` 등이다(`:300-302`). **새 flag(`TRAINING`)는 모르는 flag 라서 `unranked_reasons()` 가 비어 있고 리플레이 업로드가 계속 시도된다**(`crates/rbms-ir/src/worker.rs:74-76`, `crates/rbms-ir/src/sync.rs:59`). 리플레이 업로드는 `chart` 행이 있어야 하므로(`web/src/server/route/replay.ts:22-28`) `chart` 행을 만들지 않으면 서버가 `IR_CHART_NOT_FOUND` 로 거절하고 클라이언트는 `Rejected` 로 기록한다(`apps/rbms-player/src/record_sync.rs:667-670`). 어느 쪽이 맞는지는 결정 사항이다(8절).
5. 판별 trace(노트별 판정): 클라이언트는 서버가 점수를 받으면 trace 를 올린다(`record_sync.rs:567-575`). 서버는 해시 모양과 점수 소유만 본다(`web/src/server/route/trace.ts:89-100`, `web/src/server/service/domain/trace/trace.service.ts:301-308`). 본문에 `chart.{md5,sha256}`, 노트별 `time_us/lane/kind/state/err_us` 가 모두 들어 있어 **차트 패턴 없이도 단독으로 분석된다**(`web/src/server/dto/trace.ts:152-172`).

### 6.2 훈련곡을 식별해 "훈련 결과" 로 분석에 잇기

- **식별**: 훈련곡 md5(와 sha256)를 서버가 만든 시점에 `training_chart` 표에 기록해 두고, 점수 제출 때 `chart.md5`/`sha256` 로 조회하면 클라이언트 변경 없이 식별된다. `score.chart_md5` 가 저장되므로(`score.service.ts:132-140`) 나중 조회도 `training_chart` 조인으로 된다.
- **분석 연결**: trace 의 노트 순서(시각, 레인 순)가 생성기가 기억하는 구간(plan 의 segment)과 대응한다. 구간별 정확도는 `trace.notes.state/err_us` 를 구간 경계 시각으로 가르면 얻는다. 패턴 조인이 필요하면 5.3절의 패턴 문서를 `chart_pattern` 에 넣는다.
- **재사용할 기존 AI 인프라**: 기능 실행기 `runAiFeature`, 스펙 `AiFeatureSpec`(`web/src/server/service/domain/ai/ai-digest.ts:16-19,86-93`), 출력 정제 `refine` 훅. LLM 출력은 Zod 스키마로 받고 서버가 한도·밀도·레인 규칙으로 정제한다(practice-folder 가 하는 방식과 같다, 1.4절). 패턴 어휘는 `PATTERN_TAGS` 와 맞추면 훈련 결과가 기존 약점 분류와 같은 말로 이어진다(`pattern-tags.ts:33-52`).

### 6.3 랭킹·통계 오염 방지가 필요한 지점

훈련곡 판을 `ranked=false` 로 받으면 `chart_best` 는 막힌다(`score.service.ts:408-422`). 그러나 `score` 행은 남으므로 **`score` 를 직접 읽는 곳**에 별도 제외가 필요하다.

| 지점 | 무엇이 새는가 | 파일:줄 |
|---|---|---|
| 차트 검색 | 자동 생성된 `chart` 행이 공개 검색에 나온다 | `web/src/server/compose/fe.ts:123-140` |
| 종합 랭킹 `plays` | 제출 수에 훈련곡 판이 센다 | `compose/fe.ts:73-80` |
| 플레이어 "플레이 기록" | `ranked` 무관 모든 판 | `compose/fe.ts:154-166` |
| 일별 플레이 시간 | `score` 모든 판(ranked 무관) | `compose/fe.ts:203-218`, `docs/web/architecture.md:133` |
| 서버 통계 합계 | `score` 전체 수 | `compose/fe.ts:220-225` |
| 플레이어 점수 목록(공개) | `GET /api/players/[id]/scores` | `compose/fe.ts:249-296` |
| 점수 수 | 사용자 점수 개수, 라이벌 표시 | `compose/index.ts:176-179`, `compose/rival.ts:20-22`, `compose/fe.ts:319-322` |
| AI 최근 판 | 최근 점수, 타이밍 습관, 주간 요약 등이 `score` 에서 `autoplay=false` 만 거른다 | `compose/ai-play-features.ts:20-26`, `compose/ai-score-features.ts:134-157,202`, `compose/ai-record-query.ts:104-113` |
| 홈 최근 활동 | `ranked=true` 만 읽으므로 새지 않는다 | `compose/fe.ts:143-152` |
| 램프 분포·종합 랭킹 `cleared/played` | `chart_best` 만 읽으므로 `ranked=false` 면 새지 않는다 | `compose/fe.ts:73-97,195-201` |
| 발광 표 기록·단위 | 표 md5 `IN` 으로 거르고(`compose/table-snapshot.ts:114-133`), 단위는 `course_score` 만 읽는다. 훈련곡은 어느 쪽에도 없다 | 위 줄, `web/src/server/compose/dan.ts:53-83` |
| 캐시 무효화 | 점수 제출 때 `player-recent`, `player-stats`, `player-play-days`, `activity-recent`, `leaderboard-players` 태그 | `web/src/server/service/read-cache.ts:76-85`, `docs/web/architecture.md:403` |

그리고 웹 화면의 flag 배지 목록 `SUBMISSION_FLAGS` 에 새 flag 를 더해야 `unranked` 행에 배지가 나온다(`web/src/shared/constants/clear-lamp.ts:100-109`, `docs/web/components.md:154-162`).

제안(추정): 훈련곡 판은 (가) 서버가 `rankedPolicy` 에 `TRAINING` 을 더해 `ranked=false` 로 받고, (나) `score` 쿼리의 제외는 `score` 에 `training boolean` 열을 두어 `eq(score.training, false)` 한 줄로 하는 편이 `NOT EXISTS (training_chart)` 를 곳곳에 붙이는 것보다 단순하다. 단 그 열은 `score` 가 큰 표라 0014 에 `ADD COLUMN ... DEFAULT 0` 을 넣게 된다(MySQL 8.4 에서 즉시 열 추가 여부는 확인하지 못함).

### 6.4 DB 변경과 마이그레이션 번호

- 현재 마지막은 `0013_assisted_dan`(저널 idx 13, `web/drizzle/meta/_journal.json`), 다음은 **0014**.
- 규칙: `drizzle-kit push` 금지, `db:generate` 산출물과 `meta/` 커밋, 저널 `when` 단조 증가, `dev` 푸시가 곧 운영 배포이므로 **마이그레이션 파일만 먼저 푸시 → `web-db.yml` `status` → `migrate` → `status` → 코드 푸시** 순서(`docs/web/architecture.md:286,295`, `docs/reference/ai-features/server.md:213-219`). 운영 `migrate` 는 자동 모드가 거부하므로 사용자 `!` 실행이 필요하다(`docs/HANDOFF.md:5절`).
- 제안 스키마(추정, 컨벤션: 컬럼 snake_case): `training_chart` = `md5 char(32) PK`, `sha256 char(64) NOT NULL`(unique 색인), `user_id varchar(36) FK cascade`, `title varchar(200)`, `level int`, `note_count int`, `length_ms int`, `plan json`(LLM 구성안 + 구간 경계), `file_gz mediumblob`(BMS 바이트), `created_at bigint`; 색인 `(user_id, created_at)`. 선택: `score.training boolean DEFAULT 0` + 색인.

---

## 7. 구현 단위 제안

### 7.1 변경 파일 목록 (확인된 위치 기준)

**DB** (`web/`)
- `web/src/server/db/schema.ts`: `trainingChart` 표 추가(기존 표 정의 근처 480-520 또는 670-690 옆), 선택적 `score.training`.
- `web/drizzle/0014_*.sql`, `web/drizzle/meta/0014_snapshot.json`, `web/drizzle/meta/_journal.json`: 마이그레이션 파일만 담은 별도 커밋 먼저.

**서버** (`web/src/server/`)
- `dto/training-chart.ts`(신규): LLM 출력 스키마(구간 목록: 마디 수, 패턴 어휘, 레인 집합, 격자, 밀도), 한도 상수, 응답 스키마. `dto/ai.ts` 의 `AI_PRACTICE_LIMIT` 모양을 따른다.
- `service/domain/ai/training-plan.service.ts`(신규): `AiFeatureSpec` + `runAiFeature` + `refine`(구간·밀도·레인 규칙으로 정제). 입력 digest 는 결정론 분석 결과(`readFeaturesMany`, 약점 태그 등).
- `service/domain/training-chart/`(신규, 순수 함수 중심): 구성안→노트 목록, 노트→BMS 텍스트, 시각 계산(5.3절), zip 조립(CRC-32 포함), 서비스(저장·조회·패턴 문서 등록).
- `compose/training-chart.ts`(신규)와 `compose/index.ts` 연결(`composeFolder` 호출 `compose/index.ts:376`, `folderService` 노출 `:418`). 폴더 항목에 훈련곡 `download_url` 을 붙이는 조인은 `compose/folder.ts` 의 `listItems`(:69-81) 에.
- `route/ai.ts` 또는 새 라우트 + `app/api/fe/me/ai/training-chart/{preview,create}/route.ts`(practice-folder 와 같은 모양, `maxDuration = 300`), `app/api/training-charts/[md5]/chart.zip/route.ts`(공개 GET).
- `dto/folder.ts:115-121`: `folderItemSchema` 에 선택 필드 `download_url`. `folder.service.ts` 의 `toFolderItem`(:146-152) 과 `FolderItemRow`(:24-31).
- `service/domain/score/ranked-policy.ts`(:3-11 flag 목록, :61-74 정책), `score.service.ts:334-339`, `route/score.ts:81-106`: 훈련곡 판정.
- `compose/fe.ts`, `compose/index.ts`, `compose/rival.ts`, `compose/ai-play-features.ts`, `compose/ai-score-features.ts`, `compose/ai-record-query.ts`: 6.3절 표의 제외 조건.
- `lib/error-code.ts`, `error-message.ts`, `error.ts`: 훈련곡 없음·한도 초과 등(세 파일 모두, `docs/reference/ai-features/server.md:241`).
- 시험: `web/tests/` 아래 골든 픽스처 시험, 변환기 단위 시험, 라우트 시험, DB 통합(`RBMS_DB_E2E=1 bun test tests/db`).

**웹 UI** (`web/src/`, FSD)
- `entities/ai.query.ts`, `entities/ai.type.ts`, `shared/constants/query-key.ts`(`QUERY_KEY.AI` 확장).
- `widgets/ai/ai-training-chart-panel.tsx`(신규, `ai-practice-folder-panel.tsx` 와 같은 모양), `widgets/ai/ai-hub-tabs.tsx` 에 탭, `app/(app)/ai/` 페이지 연결.
- `shared/constants/clear-lamp.ts:100-109`: `TRAINING` flag 배지.
- 문구는 `web/messages/{ko,en,ja}/*.json` 세 언어 모두(`docs/HANDOFF.md` 5절 규칙).

**클라이언트 (Rust)** - 7.3절의 경로 선택에 따라
- `crates/rbms-ir/src/dto/folder.rs:29-39`: `UserFolderItem` 에 `#[serde(default)] download_url: String`.
- `apps/rbms-player/src/user_folders.rs`: `ShownFolder`(:152-163) 에 없는 곡 목록, `matched`(:174-190), `tidied`(:201-215), 저장본 호환(`#[serde(default)]`).
- `apps/rbms-player/src/lib.rs:478-503`: `MissingRef` 에 출처 구분(표/내 폴더), `SelectItem::Missing` 을 match 하는 곳 약 15곳(`app_result.rs:763`, `select_tree.rs:221-235`, `app_play.rs:1221`, `tablesrc.rs:1087`, `tablesrc/seed.rs:200`, `stage/loading.rs:926`, `stage/tables.rs:843`, `stage/capture/search.rs:90`, `stage/select/scene.rs:237`, `events.rs:361`, `list.rs:425-441,736,748`, `skinned.rs:337,371,426`, `mod.rs:511,1586`, `download_modal.rs:142,316`).
- `apps/rbms-player/src/stage/select/list.rs:376-390`: 내 폴더 안 줄에 없는 곡 추가.
- `apps/rbms-player/src/stage/select/download_modal.rs`: 내 폴더 출처의 `TableEntry` 합성(`md5`, `title`, `artist`, `url_diff = download_url`)과 문구 분기. `download.rs:312` 의 `start_download` 는 `TableEntry` 를 받으므로 그대로 쓴다.
- 새 문구는 세 언어 카탈로그(`apps/rbms-player/src/text/**`), 골든·캡처 시험 갱신.

### 7.2 경로 비교: 폴더 항목에 인라인 vs 별도 다운로드 엔드포인트

| 기준 | 인라인(폴더 JSON 에 BMS 내용 포함) | 별도 엔드포인트(zip URL 을 항목에 실음) |
|---|---|---|
| 서버 변경 | 폴더 응답에 base64 필드, 응답 크기 증가(30곡 × 약 6~8 KB, 추정) | zip 라우트 + `download_url` 필드 |
| 클라이언트 변경 | 파일 쓰기·폴더 배치·부분 스캔·충돌 처리 **전부 새 코드**(`rbms-download` 는 URL 을 받으므로 못 씀) | 내 폴더 줄 + 모달만(받기·풀기·스캔은 기존 재사용) |
| 저장본 | `user-folders/<해시>.ron`(최대 32 MiB, `user_folders.rs:68`)에 BMS 내용이 쌓임 | 항목당 URL 한 줄 |
| 요청 제한 | 폴더 요청 30초, 그 밖 5초(`docs/reference/ai-features/client.md:89`) | 받기는 `rbms-download` 자체 한도(30초 무응답, 15분) |
| 판정 | 불리 | **유리** |

### 7.3 클라이언트(Rust) 변경이 꼭 필요한가: 가장 작은 경로들

| 경로 | 클라이언트 변경 | 사용자 동작 | 근거·한계 |
|---|---|---|---|
| **P0 웹에서 직접 받기** | 없음 | 웹에서 훈련곡 zip 을 브라우저로 받아 곡 폴더(또는 다운로드 폴더)에 풀고 선곡에서 다시 스캔 | 폴더가 md5 를 들고 있으므로 스캔 뒤 곧바로 내 폴더의 가진 곡이 된다(2.2절 6, 8). "바로 받기" 요구는 충족하지 못한다 |
| **P1 사용자별 훈련곡 표** | 없음(기존 표·모달·받기 재사용) | 웹이 사용자별 비공개 주소의 `header.json`/`data.json`(`url_diff` = zip URL)을 서빙하고, 사용자가 클라이언트 Tables 에서 한 번 등록 | 표는 인증 헤더 없는 GET 이라 주소에 비밀이 든다(`crates/rbms-table/src/fetch.rs:117-135`, User-Agent 만 붙인 `client.get(url).send()`). 표는 하루 지난 저장본을 다시 받는다(`docs/acknowledge/2026-10-23-adr-table-daily-refresh.md`). 입구가 내 폴더가 아니라 표 레벨이다 |
| **P2 내 폴더에서 바로 받기** | `UserFolderItem.download_url` + 내 폴더 안 없는 곡 줄 + 모달 출처 분기(약 8~12개 파일, 추정) | 내 폴더에서 훈련곡 줄 선택 → 모달 → GET CHART → 스캔 뒤 같은 줄이 플레이 가능 | 사용자 결정 3의 형태와 일치. 서버 산출물은 P0·P1·P2 가 같다 |

권고(추정): 서버 쪽을 먼저 끝내 P0 로 end-to-end(생성 → 폴더 → zip → 플레이 → 점수 제출 → 분석)를 실제 클라이언트로 확인하고, 그다음 P2 를 별도 단위로 구현한다. P1 은 비밀 주소와 수동 등록 때문에 권하지 않는다.

### 7.4 순서와 의존

1. 마이그레이션 0014 파일만 커밋·푸시 → `status` → 사용자 `!` 로 `migrate` → `status`.
2. 서버: 변환기(순수 함수)와 골든 픽스처 → 서비스·저장 → zip 라우트 → 점수 `TRAINING` 판정과 제외 조건 → LLM 계획 서비스·라우트.
3. 웹 UI: 생성 화면, 결과 화면(구간별 정확도).
4. 클라이언트 P2: DTO → 내 폴더 줄 → 모달 → 시험·캡처.

병렬 가능: 변환기+픽스처(서버 순수 로직)와 UI 뼈대. 순서 고정: 마이그레이션 → 그 표를 읽는 코드 푸시(`dev` 푸시가 곧 운영 배포).

---

## 8. 확인하지 못한 것, 추측으로 남긴 것, 열린 질문

확인하지 못한 것:
- 플레이어를 실제로 구동해 키음 0개 차트를 끝까지 플레이해 보지 않았다(코드 읽기만). 훈련곡 BMS 를 라이브러리 스캔(`rbms-cli scan`)까지만 확인했다.
- 운영 Vercel 플랜(Hobby/Pro), 운영 DB 의 `max_allowed_packet`, Vercel Node 버전, `score` 표 크기와 `ADD COLUMN` 비용.
- Next 16.3.4 에서 `[md5]/chart.zip` 같은 중첩 동적 세그먼트 경로와 `Cache-Control: immutable` 이 `cacheComponents: true`(`web/next.config.ts`)에서 기대대로 동작하는지(일반 규칙으로만 판단).
- 옛 클라이언트 빌드가 `download_url` 이 든 폴더 응답을 실제로 읽는지(DTO 정의로만 확인).
- 리플레이 업로드를 훈련곡에서 허용할지에 따른 클라이언트 동작은 코드 읽기로만 정리했다.
- 서버 `unranked` flag 이름을 `TRAINING` 으로 둘 때 웹 화면·번역에서 필요한 모든 곳(`SUBMISSION_FLAGS` 외)을 전수 조사하지는 않았다.

열린 질문(결정 필요):
1. 훈련곡 zip 을 **공개 GET(md5 만 알면 받음)** 으로 둘지, 서명 URL 로 할지.
2. 훈련곡 `chart` 행을 서버가 만들어 검색에 노출될 수 있게 둘지(6.3절 첫 줄), 만들지 않고 리플레이 업로드를 막을지.
3. 훈련곡 판을 "플레이 기록"·"일별 플레이 시간"·AI 주간 요약에 넣을지 뺄지(6.3절 표).
4. 키음: 무음으로 둘지, 박자용 클릭 wav 를 동봉할지. `#LNMODE 1` 고정 여부.
5. 클라이언트 P2(Rust) 변경을 이번 범위에 넣을지, P0 로 먼저 닫을지.
6. 훈련곡 보존 정책(사용자당 최대 개수, 삭제 시 폴더 항목·`training_chart` 처리).
7. 훈련곡 폴더 이름 접두사(현재 AI 연습은 `AI 연습 · `)와 한도(폴더 50, 폴더당 2,000, 이름 64자).

---

## 부록 A. 직접 실행한 검증 기록

장소: 저장소 밖 임시 폴더(세션 scratchpad). 저장소 파일은 건드리지 않았고 `git status` 는 깨끗했다. `rbms-cli` 는 현재 소스를 별도 target 디렉터리에 새로 빌드해 썼다(`cargo build -p rbms-cli --locked --release`, 41초). 저장소 `target/release/rbms-cli` 는 2026-10-20 빌드라 파서·차트 크레이트의 마지막 변경(2026-10-29, `c38a7ea`)보다 오래돼 쓰지 않았다.

### A.1 생성기 핵심 (TypeScript, bun 1.4.2)

```typescript
import { createHash } from 'node:crypto'

const LANE_CHANNEL = ['11', '12', '13', '14', '15', '18', '19', '16'] as const
const LN_CHANNEL = ['51', '52', '53', '54', '55', '58', '59', '56'] as const
const MICROS_PER_MEASURE_NUM = 240_000_000

const timeTable = (sections: number[], bpm: number) => {
    const ordered = [...new Set(sections)].sort((a, b) => a - b)
    const timeAt = new Map<number, number>()
    let previous = 0
    let time = 0
    ordered.forEach((section, index) => {
        if (index > 0) time += Math.trunc((MICROS_PER_MEASURE_NUM * (section - previous)) / bpm)
        timeAt.set(section, time)
        previous = section
    })
    return timeAt
}

const md5Of = (text: string) => createHash('md5').update(Buffer.from(text, 'utf8')).digest('hex')
```

`sections` 는 모든 마디 시작(0..마지막 마디)과 각 노트의 `마디 + 칸/격자` 의 합집합이다(Rust 가 마디마다 구분선 타임라인을 만들기 때문, `crates/rbms-chart/src/lib.rs:164-166`). 시험 곡 구성: 마디 0~3 4분 단타, 4~11 16분 트릴(키3·키5), 12~19 16분 계단(키1~7), 20~27 16분 연타(키4)+4분 스크래치, 28~35 LN 두 줄(키1·키7, 반 마디 길이)+8분 키4, 36~39 4분 3동시(키2·4·6). 헤더: `#PLAYER 1`, `#GENRE TRAINING`, `#TITLE Training Demo`, `#ARTIST R-BMS`, `#BPM 160`, `#PLAYLEVEL 8`, `#RANK 3`, `#LNMODE 1`.

### A.2 결과

| 구성 | 파일 | 요약 모드 | 패턴 비교 |
|---|---|---|---|
| BPM 160, 40마디, 16분 격자 | 5,702 B, md5 `5bf1c9f88a8d2a2b94abea6bb7530980` | `mode BEAT_7K`, `measures 40`, `timelines 480`, `notes 552`, `length 59.6s`, `wav defs 0` | 노트 이벤트 568, `time_us`·`lane`·`kind` 전부 일치, 마지막 시각 59,625,000, `declared_ln "ln"`, `has_undefined_ln false`, `tempo []` |
| BPM 175, 40마디, 12분 격자 | 4,454 B, md5 `1f157df8ab6c144fee11072455a6af0d` | `BEAT_7K`, `notes 416`, `length 54.4s` | 노트 이벤트 432, 전부 일치, 마지막 시각 54,399,754 |

- `rbms-cli scan <풀린 폴더>`: `charts 1`, `BEAT_7K`, `L8`, 제목 `Training Demo`.
- 작은 시험: 채널 11·15·16 만 → `BEAT_5K`. `#LNOBJ 02` 로 LN 표기 인식. `#LNMODE` 없는 5x LN → `has_undefined_ln true`.

## 부록 B. TypeScript 로 만든 zip 의 호환 확인

- 구성: 로컬 헤더(시그니처 `0x04034b50`, 버전 20, 플래그 `0x0800`(UTF-8 이름), 방식 8(deflate) 또는 0(저장), CRC-32, 압축·원본 크기, 이름) + 데이터, 중앙 디렉터리(`0x02014b50`), 끝 레코드(`0x06054b50`). 데이터 기술자·확장 필드·ZIP64 없음. deflate 는 `node:zlib` 의 `deflateRawSync`, CRC-32 는 256칸 표 직접 구현.
- 항목: `training-demo.bms`(deflate, 5,702 → 압축), `click.wav`(저장, 4,454 B). zip 전체 5,285 B.
- 확인: 임시 Rust 크레이트(저장소 밖, `rbms-download` 를 경로 의존으로 사용, `Cargo.lock` 은 저장소 것을 복사)에서 `download_with`(가짜 `get` 이 zip 바이트를 돌려줌, 첫 4바이트 `PK\x03\x04` 통과) → `extract_zip`: `Ok(ExtractReport { written: 2, skipped: 0, charts: 1 })`. 풀린 `training-demo.bms` 의 md5 가 원본과 같다(`5bf1c9f8…`).
- `classify_link`: `…/training/<md5>.zip` → `DirectZip`, `…/<md5>/download` → `Browser`, `…?md5=abc&file=a.zip` → `Browser`, `…/<md5>/chart.zip?exp=1&sig=abc` → `DirectZip`.
