# 사용자 폴더와 기본 난이도표 — 서버·클라이언트 계약 설계 (웨이브 17A · 작업 A4)

> 작성 2026-10-14. 조사와 설계만 담는다. 구현은 다음 웨이브가 이 문서를 보고 한다.
> 같은 웨이브의 다른 문서: 웹 현황 점검(A1, `docs/web/2026-10-14-audit.md`), 웹 화면 설계(A2), 발광 난이도표 스냅샷과 사본 주소(A3, `docs/web/2026-10-14-insane-table.md`). A1 과 A3 는 이 문서를 다 쓴 뒤에 나타나 끝에서 대조했고, 맞춘 곳은 본문에 적었다. A2 의 문서는 끝까지 보이지 않아 대조하지 못했다.
> A3 가 추천한 값(사용자 결정 전): 사본 주소 `https://bms.hyuns.uk/tables/insane1`(끝 슬래시 없음), 스냅샷 `web/data/tables/insane1/{header.json,data.json,source.json}`, 표 id `insane1`.
> 줄 번호는 2026-10-14 `dev` 브랜치(`d12b22c`) 기준이다. 같은 시각에 다른 워크플로가 Rust 소스를 고치고 있으므로 구현 전에 심볼 이름으로 다시 찾는다.

---

## 1. 요구와 결정

### 1.1 사용자 요청(원문 발췌)

- "사용자 로그인을하면 사용자 폴더가 할당될거야. r-bms는 로그인한 사용자의 정보를 불러올때 곡 preset 폴더도 불러오는거지(기능은 table이랑같음)"
- "https://darksabun.club/table/archive/insane1/ 의 테이블 정보를 가져와서 저장해놔 … r-bms에서 기본적으로 등록되어있을거야"

### 1.2 이미 내려진 결정(2026-10-14)

| 항목 | 결정 |
|---|---|
| 폴더 수 | 사용자당 여러 개, 이름을 붙인다 |
| 편집 | 웹에서만 한다. R-BMS 는 읽기만 한다(표와 같은 동작) |
| 기본 표 | 저장소에 스냅샷으로 커밋하고 웹이 표준 BMS 표 형식으로도 제공한다 |
| 기본 등록 주소 | 원본이 아니라 `https://bms.hyuns.uk` 의 사본 주소 |

### 1.3 이 문서가 정하는 것

1. 서버 데이터 모델과 한도, 마이그레이션 방법
2. 웹이 쓰는 편집 API 와 클라이언트가 읽는 API
3. 클라이언트가 폴더를 받아 선곡 화면에 올리는 길(저장본, 다시 받기, 로그아웃)
4. 발광 난이도표를 클라이언트 설정에 기본으로 넣는 규칙
5. 구현 단위, 파일 소유, 순서, 검증 방법

---

## 2. 지금의 구조 — 클라이언트

### 2.1 로그인과 토큰

| 사실 | 근거 |
|---|---|
| 설정 파일은 `~/.config/rbms/settings.ron` 하나다 | `apps/rbms-player/src/lib.rs:1386` |
| 토큰은 그 파일의 `network.ir_token` 에 평문으로 저장된다. 비밀번호는 저장하지 않는다 | `crates/rbms-config/src/schema.rs:686-688`, `apps/rbms-player/src/app_input.rs:529-533` (`save_settings` 가 세션의 토큰을 설정에 접어 넣는다) |
| 실행 중에는 `AccountSession` 이 토큰과 로그인 id 를 쥔다 | `apps/rbms-player/src/ir_session.rs:55-59`, 복원은 `restored` (`:64`) |
| 서버 주소 기본값은 없다(`server_url: None`). 주소가 없으면 `NullScoreServer` 로 돈다 | `crates/rbms-config/src/schema.rs:727`, `apps/rbms-player/src/app_network.rs:37-51` |
| 주소는 `/api` 까지 포함한다(`https://bms.hyuns.uk/api`). 클라이언트는 그 뒤에 경로를 그대로 붙인다 | `docs/reference/ir-api.md:9`, `crates/rbms-ir/src/http.rs:89-91` |
| 모든 요청은 토큰이 있으면 `Authorization: Bearer` 를 싣는다 | `crates/rbms-ir/src/http.rs:93-98` |
| 요청 하나의 시간 한도는 5초다. 응답 본문 크기에는 한도가 없다(`resp.json()`) | `crates/rbms-ir/src/http.rs:13`, `:112-114` |
| 경로에 넣는 식별자는 퍼센트 인코딩 없이 그대로 들어간다 | `crates/rbms-ir/src/http.rs:23-26` |
| 여러 서버 프로필(`ir_profiles`)이 있어도 계정 화면이 다루는 것은 첫 서버(`server_url` + `ir_token`)다 | `apps/rbms-player/src/ir_ext.rs:10`, `:85-87` |

### 2.2 "로그인한 사용자의 정보를 불러올 때" 실제로 부르는 것

| 시점 | 부르는 것 | 위치 |
|---|---|---|
| 시작 직후 첫 프레임(저장된 토큰이 있고 서버 주소가 있을 때) | `GET /auth/me` (`whoami`) 와 `GET /players/{id}/rivals` | `app_network.rs:260-268` |
| 로그인·가입 성공 | 토큰이 바뀌면 `rebuild_server()`, 이어서 `refresh_rivals()` | `app_network.rs:274-298` (`:289`) |
| 설정 동기화 | 자동이 아니다. NETWORK 탭의 올리기·내려받기 행을 눌렀을 때만 `GET`/`PUT /players/{id}/settings/player` | `app_network.rs:173`, `:206`, `ir_sync.rs:15` |
| 로그아웃 | 세션을 비우고 `player_id` 를 `guest` 로 되돌리고 서버를 다시 만든다 | `app_network.rs:159-170` |

공통 동작:

- 스레드: 요청마다 `rbms_ir::spawn_query` 가 스레드 하나를 띄우고 채널로 돌려준다(`crates/rbms-ir/src/worker.rs:29-39`). 프레임 루프는 `poll_network` 에서 `try_recv` 로 걷는다. 화면은 기다리지 않는다.
- 실패 처리: 결과는 NETWORK 탭의 상태 줄(`net_status`)에만 적힌다. `whoami` 가 401·403 이면 토큰을 버린다(`ir_session.rs:117-132`). 네트워크 실패는 토큰을 건드리지 않는다.
- 오프라인: 라이벌 목록은 설정에 캐시된 값(`network.rivals`)을 계속 쓴다. 다시 시도하는 장치는 없다(다음 시작이나 다음 로그인 때 다시 부른다).
- 설정 동기화는 곡 폴더·난이도표 목록·토큰을 올리지도 덮어쓰지도 않는다(`ir_sync.rs:43-55` `keep_local`). 사용자 폴더는 이 blob 과 무관한 새 통로가 필요하다.

### 2.3 표가 선곡에 나오기까지

| 단계 | 내용 | 위치 |
|---|---|---|
| 설정 | `library.tables: Vec<TableSource>`, `TableSource { name, location }`. 기본값은 빈 목록 | `crates/rbms-config/src/schema.rs:376-379`, `:762`, `:775` |
| 저장본 | 주소로 등록한 표는 한 번 받아 `<설정 폴더>/tables/<주소 해시 16자리>.json` 에 둔다. 통째로 쓰고 이름을 바꿔 넣는다 | `apps/rbms-player/src/tablesrc.rs:55`, `:163-169`, `:235-310` |
| 저장본 형식 | `name, symbol, level_order, entries, course?, skipped, source, fetched_at?` | `crates/rbms-table/src/saved.rs:21-32`, `:53-59` |
| 읽기 | 라이브러리 스캔 뒤 작업 스레드에서 `read_and_match_all` 이 저장본과 파일만 읽는다. 네트워크는 묻지 않는다 | `tablesrc.rs:454-479`, `stage/loading.rs:452-459` |
| 첫 받기 | 저장본이 없는 표는 선곡이 뜬 뒤 `start_first_fetches` 가 한 번 받는다 | `tablesrc.rs:828-838`, `stage/loading.rs:411`, `stage/select/mod.rs:1035` |
| 등록 | Tables 화면에서 주소나 파일을 넣으면 `register_table` 이 설정에 넣고 받기를 시작한다 | `tablesrc.rs:848-857`, `stage/tables.rs:226-233` |
| 다시 받기 | 선곡의 F6(`Shortcut::ReloadTables`)은 전부, Tables 화면의 R 은 커서의 표, A 는 전부 | `keyconfig.rs:915`, `stage/select/mod.rs:948`, `stage/tables.rs:367-368`, `tablesrc.rs:805-819` |
| 받기 일꾼 | `TableFetches` 가 한 번에 최대 8개를 받고, 뜬 화면이 `apply_finished_fetches` 로 걷는다 | `tablesrc.rs:52`, `:655-789`, `:881-929` |
| 목록 자리 지키기 | 표가 도착해 목록을 다시 지을 때 `ListPlace` 가 보고 있던 레벨과 커서의 폴더를 이름으로 다시 찾는다 | `tablesrc.rs:1009-1061` |
| 받기 통로 | `rbms_table::DifficultyTable::fetch` — 웹 페이지의 `bmstable` 메타 → `header.json` → `data_url`. 인증 헤더를 싣는 길이 없다. 전체 30초, 응답 64MB 한도 | `crates/rbms-table/src/lib.rs:131-135`, `fetch.rs:27`, `:37`, `:78-95`, `:123-133` |

선곡 화면의 모양:

- 목록 종류는 `SelectView { Root, AllSongs, TableLevels(표), TableLevel(표, 레벨) }` 네 가지다(`apps/rbms-player/src/lib.rs:416-421`).
- 뿌리: `ALL SONGS (n)` 다음에 표마다 `{표 이름} ({가진 곡 수})` 한 줄. 레벨이 하나도 없는 표(아직 못 받았거나 실패)는 줄이 없다(`stage/select/list.rs:276-288`).
- 표 안: 레벨마다 `LV {레벨} ({가진 곡 수})` 한 줄. 가진 곡이 0 인 레벨도 빈 폴더로 나온다(`list.rs:290-303`, `crates/rbms-table/src/lib.rs:162-165`).
- 이름: 사용자가 붙인 이름 → 표 자신의 이름 → `DIFFICULTY TABLE` 순. 라틴 글자는 대문자로 바꾼다(`tablesrc.rs:390-392`).

곡을 맞추는 규칙:

- **md5 로만 맞춘다.** `match_levels_in` 은 항목의 `md5` 만 본다(`crates/rbms-table/src/lib.rs:172-189`, `:178`). `sha256` 만 있는 항목은 파싱은 되지만(`dto.rs:30-32`) 어느 곡과도 맞지 않는다.
- 라이브러리 항목(`SongEntry`)에는 `md5` 만 있다(`crates/rbms-library/src/lib.rs:37`). md5 색인은 `Library::indices_for_md5` 로 바로 물을 수 있다(`:143-145`).

가지고 있지 않은 곡:

- 선곡 목록의 줄은 `SelectItem::Song(라이브러리 색인)` 과 `SelectItem::Folder` 둘뿐이다(`lib.rs:425-428`). **가지고 있지 않은 곡은 선곡에 줄로 나오지 않는다.**
- 얼마나 가지고 있는지는 Tables 화면의 줄에만 나온다: `{owned} OF {entries} CHARTS OWNED  -  {levels} LEVELS`(`stage/tables.rs:127-131`).

### 2.4 음악 폴더 화면(Folders)과의 차이

| | Folders (`stage/folders.rs`) | Tables (`stage/tables.rs`) |
|---|---|---|
| 다루는 것 | 이 컴퓨터의 디렉터리 경로(`library.folders`) | 표의 주소나 파일(`library.tables`) |
| 바뀌면 | 화면을 나갈 때 라이브러리를 다시 스캔한다 | 스캔 없이 받은 표를 지금 라이브러리와 맞춘다 |
| 네트워크 | 없음 | 있음(받기 일꾼) |
| 선곡에서 | 곡 자체가 생긴다 | 이미 있는 곡을 묶어 보여 준다 |

사용자 폴더는 Tables 쪽 성격이다(곡을 만들지 않고 묶는다). 다만 주소를 사용자가 넣지 않고 계정에서 온다는 점이 다르다.

---

## 3. 지금의 구조 — 서버

### 3.1 DB

- 스키마 파일 둘: `web/src/server/db/schema.ts`(도메인), `web/src/server/db/auth-schema.ts`(better-auth).
- 사용자: `user.id varchar(36)` 가 PK, `user.login_id varchar(64)` 가 유일 키다. 클라이언트가 쓰는 `playerId` 는 `login_id` 다(`auth-schema.ts:3-23`).
- 차트: `chart.sha256 char(64)` 가 PK, `chart.md5 char(32)` 는 유일 색인이지만 **null 을 허용한다**(`schema.ts:20-54`). 차트 행은 누군가 스코어를 내거나 `POST /api/charts` 로 올렸을 때만 생긴다.
- 서버의 표 모델: `difficulty_table` → `table_folder` → `table_chart(folder_id, chart_sha256)` (`schema.ts:301-327`). **표의 곡을 sha256 으로만 담는다.**
- 설정 blob: `setting_blob(user_id, name)` PK, `updated_at bigint`(unix ms, 낙관적 잠금 기준), 본문 한도 256KB(`schema.ts:215-226`, `dto/setting.ts:5`).
- FK 를 건 곳은 `score.user_id`(set null)와 `api_token.user_id`(cascade) 둘뿐이다. 나머지 자식 행은 compose 의 트랜잭션에서 손으로 지운다(`compose/table.ts:51-75`).

관찰한 값:

- 운영 `GET https://bms.hyuns.uk/api/tables` 는 `[]` 를 돌려준다(2026-10-14 02:40 UTC, 같은 웨이브의 다른 작업자가 받아 둔 응답을 읽음. 이 작업에서 운영 사이트에 직접 요청을 보내지는 않았다).
- 발광 난이도표 원본(`data.json`, 같은 웨이브의 표 작업자가 받아 둔 파일을 읽음): 항목 1,035개, **md5 는 1,035개 전부 있고 sha256 은 0개**, 레벨 26개, 레벨당 최대 64곡. 제목 최대 66자, 아티스트 최대 95자. 항목의 키는 `artist, comment, level, lr2_bmsid, md5, name_diff, title, url, url_diff`. md5 는 전부 소문자 16진 32자이고 중복이 없다.
- 그러므로 발광 난이도표는 지금의 서버 표 모델(`table_chart`, sha256 키)에 그대로 넣을 수 없다. 사용자 폴더의 곡도 sha256 을 요구하면 표에서 가져온 곡을 담을 수 없다.

### 3.2 인증

| 자격 | HOF | 위치 |
|---|---|---|
| 세션 쿠키(웹) | `withSession`, `withAdmin` | `web/src/server/lib/with-session.ts` |
| API 토큰(클라이언트) | `withApiToken` | `web/src/server/lib/with-api-token.ts:8-18` |
| 둘 중 하나 | `withBearerOrSession` — Bearer 헤더가 있으면 토큰, 없으면 세션 | `with-api-token.ts:20-30` |

- 토큰: `rbms_` + 32바이트 base64url. DB 에는 sha256 해시만 둔다(`lib/token.ts:3-8`, `schema.ts:189-203`).
- **토큰에 범위(scope)가 없다.** `api_token` 에는 `label`, `revoked`, `last_used_at` 만 있다. 토큰 하나가 Bearer 를 받는 모든 경로(스코어 제출, 설정 쓰기, 라이벌 쓰기)를 연다.
- 로그인·가입은 토큰과 세션 쿠키를 함께 준다(`route/auth.ts:25-44`). `POST /api/auth/token` 은 그 사용자의 토큰을 전부 폐기하고 새로 낸다(`auth.service.ts:87-90`).
- 본인 확인의 선례: 설정 경로는 `user.loginId !== playerId` 면 403 이다(`route/setting.ts:16-18`).

### 3.3 응답 봉투와 에러

- 클라이언트가 읽는 경로(네이티브)는 봉투 없는 JSON 이다: `irRaw(data)`(`lib/api-response.ts:24`).
- 웹 전용 경로(`/api/fe/*`)는 `{ success, data, pagination? }` 봉투다(`successResponse`, `paginatedResponse`).
- 실패는 양쪽 다 `{ success: false, error: { code, message } }` 이다. `details` 는 운영에서 빠진다(`api-response.ts:19-22`).
- 에러는 세 파일에 나눠 둔다: `lib/error-code.ts`(코드), `lib/error-message.ts`(한국어 문구), `lib/error.ts`(`STATUS_MAP`). 지금 409 는 `IR_ACCOUNT_EXISTS` 하나다. 설정 충돌은 에러 코드 없이 409 를 직접 낸다(`route/setting.ts:46`).

### 3.4 rate limit

- `RATE_LIMIT`: `AUTH` 5/분, `SCORE_SUBMIT` 60/분, `REPLAY_UPLOAD` 10/분, `PUBLIC_READ` 120/분(`lib/with-rate-limit.ts:4-9`).
- 저장소는 프로세스 안의 `Map` 이다(`:15`). Vercel 에서는 인스턴스마다 따로 세므로 한도가 정확히 지켜지지 않는다. 남용을 늦추는 정도로 봐야 한다.
- 키는 기본이 IP 다. 사용자별로 세려면 `consumeRateLimit` 을 핸들러 안에서 직접 부른다(선례: `route/auth.ts:20-23`).

### 3.5 계층 규칙

`web/AGENTS.md` 는 Next.js 16 이 학습 자료와 다르니 `node_modules/next/dist/docs/` 를 먼저 읽으라는 안내만 담고 있다. 계층 규칙의 정본은 `docs/web/architecture.md` §2-2 와 실제 코드다.

```
app/api/**/route.ts            팩토리 호출과 export 만
  → @server/route/<도메인>.ts   DTO 검증 · 인증 HOF · null 을 에러로 · 응답 헬퍼
    → @server/service/domain/<도메인>/<도메인>.service.ts   순수 로직. HTTP 도 Drizzle 도 모른다
      → *ServiceDb               @server/compose/<도메인>.ts 가 Drizzle 로 구현. 트랜잭션은 여기서만
```

- 합성 순서: 바깥 `withErrorHandling` → 인증 HOF → 핸들러(`route/setting.ts:28-37`).
- `compose()` 는 싱글턴이고 도메인별 `composeXxx(db)` 를 펼쳐 합친다(`compose/index.ts:362-375`).
- 개인 데이터는 캐시하지 않는다. 설정 경로는 `'use cache'` 를 쓰지 않는다(`docs/web/architecture.md` §1-2 표의 "개인·비캐시").

### 3.6 테스트가 짜인 방식

- 러너는 `bun test`, 전처리는 `tests/lib/preload.ts`(`server-only` 를 빈 모듈로 바꾼다).
- 서비스 테스트는 DB 없이 돈다: `*ServiceDb` 를 `Map` 으로 만든 인라인 객체로 넣는다(`tests/service/setting.test.ts:9-25`).
- 라우트 테스트는 DB 에 닿기 전에 끝나는 경로만 본다(본문 크기 413, Bearer 없는 401, 쿼리 검증 400). `'use cache'` 를 타는 경로는 `RBMS_ROUTE_E2E=1` 일 때만 돈다(`tests/route/api-ext.test.ts:17-20`).
- 클라이언트와 서버의 필드 이름은 `tests/dto/ir-contract.test.ts` 가 고정한다. 클라이언트 쪽은 `crates/rbms-ir/src/contract_tests.rs` 가 루프백 가짜 서버(`mock_http.rs`)로 요청 줄과 응답 해석을 고정한다.

### 3.7 마이그레이션 — 주의가 필요한 상태

- 저장소에는 `web/drizzle/0000_smooth_cerebro.sql` 하나만 있다(테이블 21개).
- **`web/.gitignore:49` 가 `/drizzle/meta` 를 무시한다.** 이 컴퓨터의 `web/drizzle/` 에도 `meta/` 가 없다.
- `drizzle-kit migrate` 는 `meta/_journal.json` 이 없으면 `Can't find meta/_journal.json file` 로 멈춘다(`web/node_modules/drizzle-orm/migrator.js:6-9`).
- `drizzle-kit generate` 는 `meta/` 의 스냅샷과 지금 스키마를 견줘 차이를 낸다. 스냅샷이 없으면 21개 테이블을 전부 다시 만드는 SQL 을 낸다. A1 은 임시 폴더에 생성한 결과가 저장소의 `0000_smooth_cerebro.sql` 과 한 글자도 다르지 않음을 확인했다(`docs/web/2026-10-14-audit.md` §6-4. 되살리는 절차도 거기에 있다).
- 적용 여부 판단은 이렇게 한다: DB 의 `__drizzle_migrations` 에서 `created_at` 이 가장 큰 행을 읽고, journal 항목의 `when` 이 그보다 큰 것만 실행한다(`web/node_modules/drizzle-orm/mysql-core/dialect.js:39-54`).
- 운영 DB 에는 `drizzle-kit migrate` 로 21개 테이블을 적용했다는 기록이 있다(`docs/history/2026-09-09-web-nextjs-ir-server.md:10`).

되살리는 절차는 §5.4 에 적는다.

---

## 4. 설계 요약

| 주제 | 추천 |
|---|---|
| 곡 식별자 | md5 필수, sha256 선택. 제목과 아티스트의 사본을 항목에 둔다 |
| 한도 | 사용자당 폴더 50개, 폴더당 2,000곡, 사용자 전체 20,000곡, 이름 64자 |
| 편집 API | `/api/fe/me/folders…` (세션, 봉투) |
| 읽기 API | `/api/players/{playerId}/folders` 와 `/api/players/{playerId}/folders/{folderId}` (Bearer 또는 세션, 봉투 없음) |
| 클라이언트 통로 | 전용 통로. 색인을 받고 바뀐 폴더만 받는다 |
| 선곡 화면 | 뿌리에 사용자 묶음 한 줄 → 폴더 → 곡 |
| 공개 폴더 | v1 은 전부 비공개. 컬럼만 예약 |
| 기본 표 | 설정에 "이미 넣어 준 기본 표" 목록을 두고 한 번만 넣는다. 스냅샷을 실행 파일에 넣어 저장본까지 심는다 |

---

## 5. 설계 (가) — 데이터 모델

### 5.1 테이블

`web/src/server/db/schema.ts` 에 두 테이블을 더한다.

```ts
export const userFolder = mysqlTable(
    'user_folder',
    {
        id: varchar('id', { length: 36 }).primaryKey(),
        userId: varchar('user_id', { length: 36 })
            .notNull()
            .references(() => user.id, { onDelete: 'cascade' }),
        name: varchar('name', { length: 64 }).notNull(),
        description: varchar('description', { length: 255 }).default('').notNull(),
        isPublic: boolean('is_public').default(false).notNull(),
        position: int('position').default(0).notNull(),
        itemCount: int('item_count').default(0).notNull(),
        createdAt: timestamp('created_at', { fsp: 3 }).defaultNow().notNull(),
        updatedAt: bigint('updated_at', { mode: 'number' }).default(0).notNull(),
    },
    (table) => [
        uniqueIndex('user_folder_owner_name_uniq').on(table.userId, table.name),
        index('user_folder_owner_position_idx').on(table.userId, table.position),
    ],
)

export const userFolderItem = mysqlTable(
    'user_folder_item',
    {
        folderId: varchar('folder_id', { length: 36 })
            .notNull()
            .references(() => userFolder.id, { onDelete: 'cascade' }),
        md5: char('md5', { length: 32 }).notNull(),
        sha256: char('sha256', { length: 64 }),
        title: varchar('title', { length: 200 }).default('').notNull(),
        artist: varchar('artist', { length: 200 }).default('').notNull(),
        note: varchar('note', { length: 100 }).default('').notNull(),
        position: int('position').default(0).notNull(),
        createdAt: timestamp('created_at', { fsp: 3 }).defaultNow().notNull(),
    },
    (table) => [
        primaryKey({ columns: [table.folderId, table.md5] }),
        index('user_folder_item_position_idx').on(table.folderId, table.position),
        index('user_folder_item_md5_idx').on(table.md5),
    ],
)
```

컬럼의 뜻:

| 컬럼 | 뜻 |
|---|---|
| `user_folder.id` | `newId('uf_')` — `uf_` 와 16진 30자(120비트). URL 에 그대로 쓸 수 있다(`lib/token.ts:19`) |
| `name` | 사용자가 붙인 이름. 한 사용자 안에서 유일하다 |
| `is_public` | v1 에서는 항상 false. 공개 폴더를 열 때 쓴다(§9.3) |
| `position` | 사용자 안에서의 순서. 0부터 빈틈없이 |
| `item_count` | 곡 수의 사본. 색인 응답과 한도 검사에서 `count(*)` 를 피한다 |
| `updated_at` | unix ms. **이름·설명·순서·곡 어느 것이 바뀌어도 올린다.** 클라이언트가 "이 폴더를 다시 받아야 하는가"를 이 값으로 판단한다(`setting_blob.updated_at` 과 같은 방식) |
| `user_folder_item.md5` | 곡의 식별자. 소문자 16진 32자 |
| `sha256` | 알 때만 채운다(서버에 차트 행이 있을 때) |
| `title`, `artist` | 넣을 때의 사본. 서버에 차트 행이 없는 곡도 이름을 보여 주기 위해서다 |
| `note` | 곡마다 붙이는 짧은 메모(선택 기능) |

### 5.2 판단과 근거

- **기존 `table_*` 테이블을 다시 쓰지 않고 따로 둔다.** A1 의 점검 문서가 남긴 물음(소유자 열을 더해 다시 쓸지, 따로 둘지)에 대한 답이다. `table_chart` 는 sha256 이 PK 의 일부라 md5 만 아는 곡을 담지 못하고, 표 읽기는 공개이며 캐시된다(`getTablesCached`, `read-cache.ts`). 개인 데이터를 같은 테이블에 두면 공개 읽기 쿼리마다 소유자 조건을 빠뜨리지 않아야 한다.
- **md5 를 식별자로 삼는다.** 근거는 셋이다. (1) 클라이언트가 md5 로만 곡을 맞춘다(§2.3). (2) 발광 난이도표의 항목은 전부 md5 만 가진다(§3.1 관찰). (3) `chart.md5` 는 null 일 수 있지만 그런 차트는 어차피 클라이언트에서 맞출 수 없다. md5 가 없는 차트는 폴더에 넣지 못하게 한다.
- **제목과 아티스트의 사본을 둔다.** 서버의 `chart` 행은 스코어가 제출된 곡에만 있다. 운영 서버의 표 목록은 비어 있다(§3.1). 표에서 가져온 곡은 대부분 차트 행이 없다.
- **사본을 누구 말로 채우는가.** 넣을 때 서버가 정한다: 차트 행이 있으면 그 값 → 없고 서버가 아는 표(스냅샷)에 그 md5 가 있으면 표의 값 → 둘 다 없으면 요청이 보낸 값. 마지막 경우의 글은 사용자가 쓴 글이므로 §9 의 검증을 거친다.
- **FK 와 cascade.** 계정을 지우면 폴더가, 폴더를 지우면 곡이 같이 지워진다. `api_token` 의 선례(`schema.ts:193-195`)를 따른다. 기존 표 도메인처럼 트랜잭션에서 손으로 지우는 방법도 있으나, 여기는 소유 관계가 분명해 FK 가 더 단순하다.
- **`user_folder_item_md5_idx`.** "이 곡이 내 어느 폴더에 들어 있는가"(차트 화면의 담기 버튼 상태)를 묻는 데 쓴다.
- **이름의 유일성.** `(user_id, name)` 유일 색인이 겹침을 막는다. 운영 MySQL 의 collation 은 확인하지 못했다. 기본값(`utf8mb4_0900_ai_ci`)이면 대소문자와 악센트를 구분하지 않고 유일성을 본다. 클라이언트가 이름의 라틴 글자를 대문자로 바꿔 보여 주므로(§7.4) 대소문자를 구분하지 않는 쪽이 오히려 맞다.

### 5.3 한도

| 한도 | 값 | 근거 |
|---|---|---|
| 사용자당 폴더 | 50 | 선곡 화면에서 한 목록으로 훑을 수 있는 수. 색인 응답이 10KB 안쪽이다 |
| 폴더당 곡 | 2,000 | 발광 난이도표 전체(1,035곡)를 한 폴더에 담을 수 있는 크기의 약 두 배. 관찰한 항목 크기(평균 138바이트, 최대 316바이트 — `{md5, sha256:"", title, artist, note:""}` 로 줄였을 때)로 2,000곡은 약 280KB 다 |
| 사용자 전체 곡 | 20,000 | 폴더 한도의 곱(100,000)이 아니라 이 값으로 막는다. `item_count` 의 합으로 검사한다 |
| 폴더 이름 | 1~64자 | `api_token.label`, `setting_blob.name` 과 같은 길이 |
| 설명 | 0~255자 | |
| 제목·아티스트 사본 | 각 200자 | 관찰한 최대가 66자·95자다. 넘는 값은 서버가 잘라 담는다 |
| 메모 | 0~100자 | |
| 한 번에 넣는 곡 | 500 | 요청 본문을 512KB 로 막는다(`parseLimitedJsonBody`). 표 전체를 넣는 것은 본문이 필요 없는 가져오기 경로로 한다 |

폴더 하나의 응답은 최악의 경우(제목·아티스트·메모가 전부 3바이트 문자로 꽉 찼을 때) 약 3.3MB 다. Vercel 함수의 응답 본문 한도는 이 작업에서 문서로 확인하지 못했다(§12).

### 5.4 마이그레이션

`meta/` 가 없으므로(§3.7) 먼저 되살린다. 순서:

1. `web/.gitignore` 에서 `/drizzle/meta` 줄을 뺀다(결정 필요 — §11 의 9번).
2. 스키마를 고치기 **전에** 임시 출력 폴더로 `drizzle-kit generate` 를 한 번 돌려 지금 스키마의 스냅샷(`meta/0000_snapshot.json`)과 journal 을 얻는다. 나온 SQL 은 쓰지 않는다(저장소의 `0000_smooth_cerebro.sql` 과 같은 내용이어야 한다. 다르면 스키마가 그동안 바뀐 것이므로 멈추고 차이를 본다). `generate` 는 DB 에 접속하지 않고, `drizzle.config.ts:8` 은 DB 접속 값이 없으면 빈 문자열을 쓴다(`src/server/lib/env.ts:41-46`). 출력 폴더를 바꾸는 명령 옵션과, DB 값 없이 나머지 환경 변수 검증이 통과하는지는 실행해 보지 않았다.
3. 얻은 `meta/` 를 `web/drizzle/meta/` 에 두고 journal 의 첫 항목을 고친다: `tag` 는 `0000_smooth_cerebro`, `when` 은 **충분히 작은 값**(예: 0). `when` 이 운영 DB 에 기록된 값보다 크면 `migrate` 가 0000 을 다시 실행해 "이미 있는 테이블" 오류로 멈춘다. 작으면 건너뛴다.
4. 스키마에 두 테이블을 더하고 `bun run db:generate` 를 돌린다. `0001_*.sql` 에 `CREATE TABLE user_folder`, `user_folder_item` 과 FK 둘, 색인만 있는지 눈으로 확인한다.
5. 운영 적용은 사용자의 몫이다(`bun run db:migrate`, 접속 정보가 필요하다). 적용 전에 운영 DB 의 `__drizzle_migrations` 에 행이 하나 있고 그 `created_at` 이 새 journal 의 0001 `when` 보다 작은지 확인하도록 안내한다.

`drizzle-kit push` 는 쓰지 않는다(`docs/web/architecture.md` §3-2).

---

## 6. 설계 (나) — 웹 API

### 6.1 편집 API (웹 전용, 세션, 봉투)

전부 `withSession` 이다. 대상은 항상 세션의 사용자다. 경로에 사용자 id 를 받지 않는다(다른 사람의 폴더를 가리킬 길이 없다).

| 메서드 | 경로 | 하는 일 | 성공 |
|---|---|---|---|
| GET | `/api/fe/me/folders` | 내 폴더 목록(곡 없이) | 200 `FolderSummary[]` |
| POST | `/api/fe/me/folders` | 폴더 만들기 | 201 `FolderSummary` |
| PUT | `/api/fe/me/folders/order` | 폴더 순서 바꾸기 | 200 `FolderSummary[]` |
| GET | `/api/fe/me/folders/[folderId]` | 폴더 하나와 곡 전부 | 200 `FolderDetail` |
| PATCH | `/api/fe/me/folders/[folderId]` | 이름·설명 고치기 | 200 `FolderSummary` |
| DELETE | `/api/fe/me/folders/[folderId]` | 폴더 지우기 | 204 |
| POST | `/api/fe/me/folders/[folderId]/items` | 곡 넣기(최대 500) | 200 `ItemChange` |
| POST | `/api/fe/me/folders/[folderId]/items/remove` | 곡 빼기(여러 개) | 200 `ItemChange` |
| PUT | `/api/fe/me/folders/[folderId]/items/order` | 곡 순서 바꾸기 | 200 `ItemChange` |
| PATCH | `/api/fe/me/folders/[folderId]/items/[md5]` | 메모 고치기(선택 기능) | 200 `FolderItem` |
| POST | `/api/fe/me/folders/[folderId]/import` | 표에서 가져오기 | 200 `ItemChange` |
| GET | `/api/fe/me/folders/containing/[md5]` | 이 곡이 든 내 폴더의 id 목록 | 200 `string[]` |

경로가 겹치지 않는 이유: Next 는 고정 세그먼트를 동적 세그먼트보다 먼저 맞춘다. 폴더 id 는 `uf_` 로 시작하므로 `order`·`containing` 과 겹치지 않고, md5 는 16진 32자이므로 `remove`·`order` 와 겹치지 않는다.

여러 개를 빼는 요청을 `DELETE` 가 아니라 `POST …/remove` 로 둔 이유: `DELETE` 요청의 본문은 중간 장비가 버리는 경우가 있다.

### 6.2 DTO (Zod)

새 파일 `web/src/server/dto/folder.ts`. 기존 `dto/common.ts` 의 `isMd5Hash`, `isSha256Hash` 를 쓴다.

```ts
export const FOLDER_LIMIT = {
    FOLDERS_PER_USER: 50,
    ITEMS_PER_FOLDER: 2000,
    ITEMS_PER_USER: 20000,
    ITEMS_PER_REQUEST: 500,
    NAME_MAX: 64,
    DESCRIPTION_MAX: 255,
    TEXT_COPY_MAX: 200,
    NOTE_MAX: 100,
    WRITE_MAX_BYTES: 512 * 1024,
} as const

const CONTROL_CHARACTERS = /[\u0000-\u001f\u007f]/

const singleLineText = (max: number) =>
    z
        .string()
        .transform((value) => value.normalize('NFC').trim())
        .refine((value) => value.length <= max && !CONTROL_CHARACTERS.test(value))

const folderIdPattern = /^uf_[0-9a-f]{30}$/

export const folderIdSchema = z.string().regex(folderIdPattern)

export const folderNameSchema = singleLineText(FOLDER_LIMIT.NAME_MAX).refine((value) => value.length >= 1)

export const folderCreateSchema = z.object({
    name: folderNameSchema,
    description: singleLineText(FOLDER_LIMIT.DESCRIPTION_MAX).default(''),
})

export const folderUpdateSchema = z
    .object({
        name: folderNameSchema.optional(),
        description: singleLineText(FOLDER_LIMIT.DESCRIPTION_MAX).optional(),
    })
    .refine((value) => value.name !== undefined || value.description !== undefined)

export const folderOrderSchema = z.object({
    folder_ids: z.array(folderIdSchema).max(FOLDER_LIMIT.FOLDERS_PER_USER),
})

const md5Schema = z
    .string()
    .transform((value) => value.trim().toLowerCase())
    .refine(isMd5Hash)

export const folderItemInputSchema = z.object({
    md5: md5Schema,
    sha256: z
        .string()
        .transform((value) => value.trim().toLowerCase())
        .refine((value) => value === '' || isSha256Hash(value))
        .default(''),
    title: z.string().default(''),
    artist: z.string().default(''),
    note: singleLineText(FOLDER_LIMIT.NOTE_MAX).default(''),
})

export const folderItemsAddSchema = z.object({
    items: z.array(folderItemInputSchema).min(1).max(FOLDER_LIMIT.ITEMS_PER_REQUEST),
})

export const folderItemsRemoveSchema = z.object({
    md5s: z.array(md5Schema).min(1).max(FOLDER_LIMIT.ITEMS_PER_FOLDER),
})

export const folderItemsOrderSchema = z.object({
    md5s: z.array(md5Schema).max(FOLDER_LIMIT.ITEMS_PER_FOLDER),
})

export const folderItemNoteSchema = z.object({
    note: singleLineText(FOLDER_LIMIT.NOTE_MAX),
})

export const folderImportSchema = z.object({
    table_id: z.string().min(1).max(64),
    levels: z.array(z.string().max(32)).max(100).optional(),
})

export type FolderCreateInput = z.infer<typeof folderCreateSchema>
export type FolderUpdateInput = z.infer<typeof folderUpdateSchema>
export type FolderItemInput = z.infer<typeof folderItemInputSchema>
export type FolderImportInput = z.infer<typeof folderImportSchema>
```

`title` 과 `artist` 는 검증에서 거부하지 않는다. 표에서 온 긴 값이 요청 전체를 실패시키지 않도록 서비스가 제어 문자를 지우고 200자로 잘라 담는다.

응답 모양(필드 이름은 클라이언트와 같은 snake_case):

```ts
type FolderSummary = {
    id: string
    name: string
    description: string
    is_public: boolean
    position: number
    item_count: number
    updated_at: number
}

type FolderItem = {
    md5: string
    sha256: string
    title: string
    artist: string
    note: string
}

type FolderDetail = FolderSummary & { items: FolderItem[] }

type ItemChange = {
    added: number
    removed: number
    skipped: number
    item_count: number
    updated_at: number
}
```

웹 화면에서는 `FolderDetail.items` 의 각 곡에 서버가 아는 정보(레벨, 모드, 차트 화면 링크 여부)를 붙이고 싶을 수 있다. 그 경우 웹 전용 응답에만 `known: boolean` 을 더한다(클라이언트가 읽는 응답에는 넣지 않는다).

### 6.3 요청과 응답 예

폴더 만들기:

```http
POST /api/fe/me/folders
content-type: application/json

{ "name": "연습 12단", "description": "하루 한 판" }
```

```json
{
    "success": true,
    "data": {
        "id": "uf_3f9a1c0b7e2d4a6f8b1c2d3e4f5a6b",
        "name": "연습 12단",
        "description": "하루 한 판",
        "is_public": false,
        "position": 2,
        "item_count": 0,
        "updated_at": 1760400000000
    }
}
```

곡 넣기:

```http
POST /api/fe/me/folders/uf_3f9a1c0b7e2d4a6f8b1c2d3e4f5a6b/items
content-type: application/json

{ "items": [{ "md5": "0123456789abcdef0123456789abcdef", "title": "Example", "artist": "Someone" }] }
```

```json
{ "success": true, "data": { "added": 1, "removed": 0, "skipped": 0, "item_count": 1, "updated_at": 1760400005000 } }
```

표에서 가져오기(표의 레벨 둘만):

```http
POST /api/fe/me/folders/uf_3f9a1c0b7e2d4a6f8b1c2d3e4f5a6b/import
content-type: application/json

{ "table_id": "insane1", "levels": ["10", "11"] }
```

이름이 겹칠 때:

```json
{ "success": false, "error": { "code": "IR_FOLDER_NAME_TAKEN", "message": "같은 이름의 폴더가 이미 있습니다." } }
```

### 6.4 에러 코드

세 파일(`error-code.ts`, `error-message.ts`, `error.ts`)에 함께 더한다.

| 코드 | 상태 | 언제 | 문구 |
|---|---|---|---|
| `IR_FOLDER_NOT_FOUND` | 404 | 폴더가 없거나 내 것이 아니다 | 폴더를 찾을 수 없습니다. |
| `IR_FOLDER_NAME_TAKEN` | 409 | 같은 이름의 내 폴더가 있다 | 같은 이름의 폴더가 이미 있습니다. |
| `IR_FOLDER_LIMIT_EXCEEDED` | 409 | 폴더 수, 폴더당 곡 수, 전체 곡 수 한도를 넘는다 | 폴더 한도를 넘었습니다. |

이미 있는 코드를 쓰는 경우: 검증 실패 400 `VALIDATION_ERROR`, 세션 없음 401 `UNAUTHORIZED`, 본문 초과 413 `IR_PAYLOAD_TOO_LARGE`, 한도 초과 요청 429 `RATE_LIMITED`, 가져올 표가 없음 404 `IR_TABLE_NOT_FOUND`.

**다른 사람의 폴더는 403 이 아니라 404 로 답한다.** 폴더 id 가 있는지조차 알려 주지 않는다.

`IR_FOLDER_LIMIT_EXCEEDED` 는 한 요청이 한도를 넘으면 **아무것도 넣지 않는다**(전부 아니면 전무). `details` 에 `{ limit, current, requested }` 를 담는다(운영에서는 빠지므로 화면은 한도 값을 상수로도 알고 있어야 한다).

### 6.5 동작 규칙

- 곡 넣기: 이미 든 md5 는 건너뛰고 `skipped` 로 센다. 새 곡은 끝에 붙는다. 요청 안의 같은 md5 는 첫 것만 쓴다.
- 곡 순서: 요청의 `md5s` 는 지금 든 곡과 **같은 집합**이어야 한다. 아니면 400. 다른 탭에서 곡이 바뀐 상태로 순서를 덮어쓰는 일을 막는다.
- 폴더 순서: `folder_ids` 도 지금 폴더와 같은 집합이어야 한다.
- 폴더 만들기: 끝에 붙는다(`position = 지금 수`).
- 폴더 지우기: 뒤 폴더의 `position` 을 당긴다.
- 어떤 바뀜이든 그 폴더의 `updated_at` 을 `now()` 로 올린다. 폴더 순서가 바뀌면 자리가 바뀐 폴더의 `updated_at` 도 올린다(클라이언트는 색인의 순서를 그대로 쓰므로 필수는 아니지만 "언제 바뀌었나"가 한 값으로 남는다).
- 표에서 가져오기: 서버가 표의 항목을 읽어 넣는다. 본문으로 1,000곡을 보내지 않는다. 한도 검사와 건너뛰기는 곡 넣기와 같다.

### 6.6 검색해서 넣기에 필요한 검색

| 대상 | 쓰는 API | 상태 |
|---|---|---|
| 서버가 아는 차트 | `GET /api/fe/charts/search?q=&mode=&level=&sort=&page=&limit=` — 제목·아티스트·장르를 `LIKE` 로 찾는다. 응답의 `ChartMeta` 에 `md5`, `sha256`, `title`, `artist` 가 있다 | 이미 있다(`route/fe.ts:41-46`, `compose/fe.ts:62-78`). md5 가 빈 결과는 화면에서 담기를 막는다 |
| 표의 항목(서버에 차트 행이 없는 곡) | `GET /api/fe/tables/[tableId]/entries?q=&level=&page=&limit=` — 표 스냅샷 안에서 제목·아티스트로 찾는다 | **새로 필요하다.** A3 는 스냅샷을 `web/data/tables/insane1/*.json` 에 두고 정적 import 해 모듈을 읽을 때 Zod 로 한 번 검증하는 방법을 추천한다(`docs/web/2026-10-14-insane-table.md` §7.2). 표 항목 검색과 가져오기는 그 모듈을 읽는다. 폴더 서비스는 `loadTableEntries(tableId)` 라는 주입 함수로만 표를 읽게 해 둔다(§6.8) |
| 해시를 직접 붙여 넣기 | 검색 없이 곡 넣기 API 에 md5 를 보낸다. 제목은 서버가 아는 만큼 채운다 | 곡 넣기로 충분하다 |

### 6.7 인가와 rate limit

- 인가: 모든 편집 경로는 세션의 `user.id` 와 폴더의 `user_id` 가 같은 행만 다룬다. ServiceDb 의 조회·수정 메서드는 **항상 `userId` 를 조건에 넣는다**(`findFolder({ userId, folderId })`). id 만으로 찾는 메서드는 공개 읽기용 하나뿐이고 v1 에서는 만들지 않는다.
- API 토큰으로는 편집할 수 없다(`withSession` 만). 클라이언트의 토큰이 새어도 폴더를 고칠 수 없다.
- rate limit 을 `RATE_LIMIT` 에 더한다. 키는 사용자 id 다(`consumeRateLimit(\`folder-write:${user.id}\`, …)` 를 핸들러 안에서 부른다).

| 이름 | 한도 | 대상 |
|---|---|---|
| `FOLDER_WRITE` | 60/분/사용자 | 만들기·고치기·지우기·순서·곡 넣고 빼기 |
| `FOLDER_IMPORT` | 10/분/사용자 | 표에서 가져오기 |
| `FOLDER_READ` | 120/분/사용자 | 클라이언트가 읽는 두 경로(§7.2) |

### 6.8 서비스와 ServiceDb 의 모양

`web/src/server/service/domain/folder/folder.service.ts`:

```ts
export type FolderRow = {
    id: string
    name: string
    description: string
    isPublic: boolean
    position: number
    itemCount: number
    updatedAt: number
}

export type FolderItemRow = {
    md5: string
    sha256: string | null
    title: string
    artist: string
    note: string
    position: number
}

export type FolderServiceDb = {
    listFolders: (userId: string) => Promise<FolderRow[]>
    findFolder: (params: { userId: string; folderId: string }) => Promise<FolderRow | null>
    listItems: (folderId: string) => Promise<FolderItemRow[]>
    sumItemCount: (userId: string) => Promise<number>
    insertFolder: (row: { id: string; userId: string; name: string; description: string; position: number; updatedAt: number }) => Promise<'created' | 'name_taken'>
    updateFolder: (params: { userId: string; folderId: string; name?: string; description?: string; updatedAt: number }) => Promise<'updated' | 'name_taken' | 'missing'>
    deleteFolder: (params: { userId: string; folderId: string; updatedAt: number }) => Promise<boolean>
    reorderFolders: (params: { userId: string; folderIds: string[]; updatedAt: number }) => Promise<void>
    appendItems: (params: { folderId: string; items: Omit<FolderItemRow, 'position'>[]; updatedAt: number }) => Promise<{ itemCount: number }>
    removeItems: (params: { folderId: string; md5s: string[]; updatedAt: number }) => Promise<{ removed: number; itemCount: number }>
    reorderItems: (params: { folderId: string; md5s: string[]; updatedAt: number }) => Promise<void>
    updateItemNote: (params: { folderId: string; md5: string; note: string; updatedAt: number }) => Promise<boolean>
    listFolderIdsContaining: (params: { userId: string; md5: string }) => Promise<string[]>
    findChartsByMd5: (md5s: string[]) => Promise<{ md5: string; sha256: string; title: string; artist: string }[]>
}

export type FolderServiceDeps = {
    db: FolderServiceDb
    newId: (prefix: string) => string
    now: () => number
    loadTableEntries: (tableId: string) => Promise<{ md5: string; sha256: string; title: string; artist: string; level: string }[] | null>
}

export const createFolderService = (deps: FolderServiceDeps) => ({ … })
export type FolderService = ReturnType<typeof createFolderService>
```

- `appendItems`, `removeItems`, `reorderItems`, `deleteFolder`, `reorderFolders` 는 각각 compose 안에서 **트랜잭션 하나**다(곡 행 + `item_count` + `updated_at` + `position` 을 함께 바꾼다). 서비스는 트랜잭션을 모른다.
- 이름 겹침은 유일 색인이 잡는다. compose 가 중복 키 오류를 `'name_taken'` 으로 바꿔 돌려주고, 라우트가 `IR_FOLDER_NAME_TAKEN` 을 던진다.
- 한도 검사(폴더 수, 폴더당 곡 수, 전체 곡 수)와 "무엇을 넣고 무엇을 건너뛸지"는 서비스의 순수 함수로 둔다. 테스트가 여기에 집중된다.

테스트로 고정할 순수 함수(제안 이름):

| 함수 | 하는 일 |
|---|---|
| `planItemInsert(existingMd5s, incoming, limits)` | 넣을 곡, 건너뛸 수, 한도 초과 여부를 낸다 |
| `isSameSet(current, requested)` | 순서 바꾸기 요청이 지금 집합과 같은지 본다 |
| `resolveItemText(input, chart, tableEntry)` | 제목·아티스트 사본을 누구 말로 채울지 정하고 다듬는다 |
| `toFolderSummary(row)`, `toFolderDetail(row, items)` | 행을 응답 모양으로 바꾼다 |

---

## 7. 설계 (다) — 클라이언트가 읽는 형식

### 7.1 두 방법의 비교

어느 방법이든 **"내 폴더가 무엇무엇인가"를 묻는 인증된 호출은 필요하다.** 사용자가 주소를 직접 넣지 않기 때문이다. 그러므로 실제로 갈리는 것은 폴더의 내용을 어느 통로로 받느냐다.

| | ① 폴더마다 표준 BMS 표 주소 | ② 전용 API 와 전용 통로 |
|---|---|---|
| 내용 주소 | `…/folders/{id}/` 의 `bmstable` 메타 → `header.json` → `data.json` | `GET /players/{id}/folders/{folderId}` |
| 클라이언트에서 다시 쓰는 것 | 받기·저장본·일꾼·Tables 화면 전부(`rbms_table`, `tablesrc.rs`) | 맞추기(`Library::indices_for_md5`), 통째로 쓰기(`write_atomic`), 알림. 받기와 저장본은 새로 쓴다 |
| 인증 | `rbms_table` 의 받기에는 헤더를 싣는 길이 없다(`fetch.rs:123-133`). 싣게 만들면 표 주소가 다른 호스트일 때 토큰이 새지 않도록 호스트 비교가 필요하고, 표 통로 전체가 계정 상태에 묶인다 | 이미 있는 `HttpScoreServer` 가 Bearer 를 싣는다 |
| 인증 없이 열면 | 주소를 아는 사람은 누구나 읽는다. 주소(120비트 id)가 로그·화면 캡처·브라우저 기록으로 새면 비공개 폴더가 열린다. 주소를 바꾸려면 폴더 id 를 다시 내야 한다 | 해당 없음 |
| 계정이 바뀌면 | 설정의 표 목록(`library.tables`)에 폴더 주소가 섞인다. 로그아웃·계정 바꿈 때 어느 줄이 누구 것인지 가려 지워야 한다 | 계정별 저장본 파일 하나를 바꾸면 끝난다 |
| 폴더가 지워지면 | 주소가 404 가 되어 "표 받기 실패"로 남는다. 줄을 지우는 것은 사용자 몫이다 | 색인에 없는 폴더는 저장본에서 빠진다 |
| 폴더가 50개면 | 뿌리에 표 줄이 50개 생긴다. 한 폴더가 한 표이고 표 안에는 레벨이 있어야 하므로 곡까지 두 단계가 된다 | 묶음 한 줄 아래 폴더들이 나온다 |
| 다른 BMS 프로그램 | 쓸 수 있다(beatoraja 등에 주소를 등록) | 못 쓴다 |
| 요청 수 | 폴더마다 3번(페이지, 헤더, 본문) | 색인 1번 + 바뀐 폴더마다 1번 |
| 서버 일 | 폴더마다 HTML·헤더·본문 경로 셋 | 경로 둘 |

**추천: ②.** 비공개가 기본인 개인 데이터를 주소의 비밀에 기대지 않고, 계정 상태(로그인·로그아웃·계정 바꿈·폴더 삭제)를 한 곳에서 다룰 수 있다. ①의 유일한 이점(다른 프로그램에서 쓰기)은 공개 폴더를 열 때 **같은 데이터를 표준 형식으로도 내보내는 것**으로 얻을 수 있다(§9.3). 그때는 그 주소를 R-BMS 의 Tables 화면에 여느 표처럼 등록하면 되고 클라이언트를 더 고칠 것이 없다.

### 7.2 읽기 API (네이티브, 봉투 없음)

| 메서드 | 경로 | 인증 | 성공 |
|---|---|---|---|
| GET | `/api/players/[playerId]/folders` | Bearer 또는 세션, 본인 | 200 `UserFolderIndex` |
| GET | `/api/players/[playerId]/folders/[folderId]` | Bearer 또는 세션, 본인 | 200 `UserFolder` |

- HOF: `withBearerOrSession` + `user.loginId === playerId` 확인(설정 경로의 `withOwner`·`requireOwner` 와 같은 모양, `route/setting.ts:16-26`).
- 본인이 아니면 403 `FORBIDDEN`(설정 경로와 같다). 폴더가 없으면 404 `IR_FOLDER_NOT_FOUND`.
- 캐시하지 않는다.
- 클라이언트의 경로(주소가 `/api` 를 포함하므로): `/players/{id}/folders`, `/players/{id}/folders/{folderId}`.

색인:

```json
{
    "folders": [
        { "id": "uf_3f9a1c0b7e2d4a6f8b1c2d3e4f5a6b", "name": "연습 12단", "description": "하루 한 판", "is_public": false, "position": 0, "item_count": 2, "updated_at": 1760400005000 },
        { "id": "uf_77aa0c0b7e2d4a6f8b1c2d3e4f5a6c", "name": "LN", "description": "", "is_public": false, "position": 1, "item_count": 0, "updated_at": 1760300000000 }
    ]
}
```

폴더 하나(`items` 는 `position` 순서):

```json
{
    "id": "uf_3f9a1c0b7e2d4a6f8b1c2d3e4f5a6b",
    "name": "연습 12단",
    "description": "하루 한 판",
    "is_public": false,
    "position": 0,
    "item_count": 2,
    "updated_at": 1760400005000,
    "items": [
        { "md5": "0123456789abcdef0123456789abcdef", "sha256": "", "title": "Example", "artist": "Someone", "note": "" },
        { "md5": "fedcba9876543210fedcba9876543210", "sha256": "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08", "title": "Another", "artist": "Else", "note": "오른손" }
    ]
}
```

`sha256` 은 모를 때 빈 문자열이다(`chartIdSchema` 의 기본값, `rbms_table::TableEntry` 의 읽기와 같은 약속).

색인을 배열이 아니라 객체로 둔 이유: 나중에 한도나 서버 시각 같은 값을 깨지 않고 더할 수 있다.

서버 능력 표시: `ServerCapabilities` 에 `folders: bool` 을 더한다(서버 `service/domain/system/capabilities.ts:11-21`, 클라이언트 `crates/rbms-ir/src/dto/system.rs:7-20`. 클라이언트 쪽은 없으면 false 로 읽힌다). 클라이언트는 이 값으로 호출 여부를 가르지 않는다. 옛 서버가 404 를 주면 조용히 넘어간다(§7.6).

### 7.3 클라이언트 DTO 와 호출 (`crates/rbms-ir`)

새 파일 `crates/rbms-ir/src/dto/folder.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserFolderSummary {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub is_public: bool,
    #[serde(default)]
    pub position: u32,
    #[serde(default)]
    pub item_count: u32,
    #[serde(default)]
    pub updated_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserFolderItem {
    pub md5: String,
    #[serde(default)]
    pub sha256: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub artist: String,
    #[serde(default)]
    pub note: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserFolderIndex {
    #[serde(default)]
    pub folders: Vec<UserFolderSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserFolder {
    #[serde(flatten)]
    pub summary: UserFolderSummary,
    #[serde(default)]
    pub items: Vec<UserFolderItem>,
}
```

`ScoreServer` 트레이트(`crates/rbms-ir/src/lib.rs:45`)에 기본 구현이 `Err(IrError::Unsupported)` 인 메서드 둘을 더한다. 기본 구현이 있으므로 `NullScoreServer` 와 테스트용 가짜 서버들은 고치지 않아도 된다.

```rust
fn user_folders(&self, _player: &PlayerId) -> Result<UserFolderIndex, IrError> {
    Err(IrError::Unsupported)
}

fn user_folder(&self, _player: &PlayerId, _folder_id: &str) -> Result<UserFolder, IrError> {
    Err(IrError::Unsupported)
}
```

`HttpScoreServer`(`http.rs`):

```rust
fn user_folders(&self, player: &PlayerId) -> Result<UserFolderIndex, IrError> {
    self.get(&format!("/players/{}/folders", player.id))
}

fn user_folder(&self, player: &PlayerId, folder_id: &str) -> Result<UserFolder, IrError> {
    self.get(&format!("/players/{}/folders/{folder_id}", player.id))
}
```

주의 둘:

- `folder_id` 는 서버가 준 값인데 경로에 그대로 들어간다(`http.rs:23-26`). 클라이언트는 색인에서 받은 id 가 `[A-Za-z0-9_-]` 로만 되어 있고 64자 이하인지 확인하고, 아니면 그 폴더를 버린다.
- 응답 본문 크기에 한도가 없다(`http.rs:112-114`). 폴더 호출은 본문을 한도 안에서 읽도록 한다(`rbms-table` 의 `read_within`, `fetch.rs:141` 이 같은 일을 한다). 한도는 8MB 를 제안한다(§5.3 의 최악 3.3MB 의 두 배 남짓).

### 7.4 선곡 화면에 보이는 모양

사용자의 말("사용자 폴더가 할당될거야 … 곡 preset 폴더도 불러오는")을 그대로 옮긴다: **뿌리에 사용자 묶음 한 줄, 그 안에 폴더들, 폴더 안에 곡.**

| 목록 | 줄 | 예 |
|---|---|---|
| 뿌리 | 표 줄들 다음에 `{묶음 이름} ({가진 곡의 합})` 한 줄 | `MY FOLDERS (87)` |
| 묶음 안 | 폴더마다 `{이름} ({가진 곡}/{전체})`. 다 가지고 있으면 `{이름} ({수})` | `연습 12단 (12/30)`, `LN (8)` |
| 폴더 안 | 가진 곡만. 정렬과 필터는 다른 목록과 같다 | |

- 묶음 줄은 로그인돼 있고 폴더가 하나 이상 있을 때만 나온다. 곡이 0 인 폴더도 줄은 나온다(표의 빈 레벨과 같다).
- 묶음 이름: `MY FOLDERS` 를 제안한다. 로그인 id 를 넣는 방법(`{ID} FOLDERS`)도 있다.
- 폴더 이름은 표 이름처럼 라틴 글자만 대문자로 바꾼다(`str::to_ascii_uppercase`, `tablesrc.rs:391`).
- **가지고 있지 않은 곡은 표와 똑같이 줄로 나오지 않는다.** 대신 폴더 줄의 `(가진 곡/전체)` 로 빠진 수가 보인다. 빠진 곡을 줄로 보여 주는 일은 표에도 같이 적용돼야 하는 별도의 작업이다(§11 의 6번).
- 레벨이 없다. 표는 "표 → 레벨 → 곡" 이고 사용자 묶음은 "묶음 → 폴더 → 곡" 이다. 깊이가 같으므로 화면 구조를 그대로 쓴다.

구현 방법: `SelectView` 에 `UserFolders` 와 `UserFolder(usize)` 를 더하고, 표와 따로 `AppShared` 에 값을 둔다. 표의 자리(`table_names`, `table_levels`)는 설정의 표 목록과 번호가 맞아야 하고(`register_table` 이 `tables.len()` 자리에 넣는다 — `tablesrc.rs:849-851`, 지울 때 같은 번호를 지운다 — `stage/tables.rs:248-256`) 거기에 끼워 넣으면 번호가 어긋난다.

`SelectView` 를 `match` 하는 곳(컴파일 오류가 안내한다): `stage/select/list.rs:276`(목록 짓기), `stage/select/scene.rs:153`(머리글), `skin_host/select/bridge.rs:102`(스킨이 읽는 열린 폴더 이름), `stage/decide.rs:310`(결정 화면의 표 이름과 레벨 — 묶음 이름과 폴더 이름을 넣는다), `tablesrc.rs:958-1052`(`ListPlace` — 폴더를 **id 로** 다시 찾게 한다. 이름은 웹에서 바뀔 수 있다). 그 밖에 `SelectView` 가 나오는 파일: `stage/select/mod.rs`, `stage/select/skinned.rs`, `stage/select/search.rs`, `app_result.rs`, `app_options.rs`, `stage/loading.rs`, `stage/capture*.rs`(이 파일들은 이 작업에서 전부 읽지는 못했다).

### 7.5 저장본

- 위치: `<설정 폴더>/user-folders/<해시>.json`. 해시는 `서버 주소 + "\n" + 로그인 id` 에 `tablesrc.rs:163` 의 `url_hash` 를 쓴다. 계정마다, 서버마다 파일이 다르므로 섞이지 않는다.
- 형식:

```json
{
    "server": "https://bms.hyuns.uk/api",
    "player_id": "dj",
    "fetched_at": 1760400100,
    "folders": [
        {
            "id": "uf_3f9a1c0b7e2d4a6f8b1c2d3e4f5a6b",
            "name": "연습 12단",
            "description": "하루 한 판",
            "position": 0,
            "item_count": 2,
            "updated_at": 1760400005000,
            "items": [{ "md5": "0123456789abcdef0123456789abcdef", "sha256": "", "title": "Example", "artist": "Someone", "note": "" }]
        }
    ]
}
```

- 쓰기: 통째로 쓰고 이름을 바꿔 넣는다(`crate::write_atomic`, 표 저장본과 같다 — `tablesrc.rs:290-298`).
- 읽기: 파일의 `server` 와 `player_id` 가 지금 값과 같을 때만 쓴다(해시가 겹친 다른 계정의 파일을 막는다. 표 저장본의 `copy_of` 와 같은 확인 — `tablesrc.rs:264-266`). 크기 한도 안에서 읽는다. 깨진 파일은 없는 것으로 본다.
- 오프라인: 토큰이 있고 서버에 닿지 않으면 저장본으로 묶음을 보여 준다. 받기에 실패해도 저장본은 건드리지 않는다.
- 받은 값의 다듬기(서버를 믿지 않는다): 폴더 50개와 폴더당 2,000곡을 넘는 부분은 버린다. 이름·설명에서 제어 문자를 지우고 64자·255자로 자른다. md5 가 16진 32자가 아닌 곡은 버린다.

### 7.6 다시 받기

받는 순서(작업 스레드 하나):

1. `user_folders(player)` 로 색인을 받는다.
2. 저장본과 견준다. 저장본에 없거나 `updated_at` 이 다른 폴더만 `user_folder(player, id)` 로 받는다. 한 번에 최대 4개.
3. 색인에 없는 폴더는 버린다. 색인의 순서대로 새 저장본을 만들어 쓴다.
4. 프레임 루프가 결과를 걷어 지금 라이브러리와 맞추고(`Library::indices_for_md5`) 목록을 다시 짓는다. 이때 `ListPlace` 로 보고 있던 자리를 지킨다(`tablesrc.rs:1057-1061` 과 같은 일).

일부 폴더만 실패하면: 받은 폴더는 반영하고, 실패한 폴더는 저장본의 옛 내용을 둔다. 저장본에도 없던 새 폴더는 이번에는 빠진다. 상태에 이유를 남긴다.

언제 받는가:

| 계기 | 위치 | 실패했을 때 |
|---|---|---|
| 시작(저장된 토큰이 있을 때) | `app_network.rs:260-268` 의 `refresh_rivals()` 옆 | 알림 없음. Tables 화면의 상태에만 남긴다. 서버에 닿지 않는 것은 흔한 일이고 연결 표시가 따로 있다 |
| 로그인·가입 성공 | `app_network.rs:289` 의 `refresh_rivals()` 옆 | 경고 알림 한 번 |
| 선곡의 F6 | `stage/select/mod.rs:948` — 표를 다시 받으면서 같이 | 경고 알림 한 번 |
| Tables 화면 | 사용자 폴더 줄에서 R, 전체 다시 받기 A 에 포함 | 경고 알림 한 번, 줄에 이유 |
| 라이브러리를 다시 읽은 뒤 | `stage/loading.rs:407` `apply_scan` — **네트워크 없이** 저장본을 새 라이브러리와 다시 맞춘다 | 해당 없음 |

- 이미 받는 중이면 새로 시작하지 않는다.
- 색인 호출이 404(옛 서버)나 `Unsupported`(서버 없음)면 조용히 넘어간다. 401·403 이면 폴더를 숨긴다(세션 만료는 `whoami` 가 따로 알린다).
- 알림 문구(예): `user folders: 5 folders, 120 charts (87 owned)` — 내용이 바뀌었을 때만. 실패: `user folders not refreshed: {짧은 이유}`.
- 다중 서버 프로필(`ir_profiles`)의 폴더는 받지 않는다. 첫 서버(`server_url`)의 계정만 본다.

### 7.7 로그아웃, 계정 바꿈, 폴더가 지워졌을 때

| 일 | 동작 |
|---|---|
| 로그아웃(`finish_logout`, `app_network.rs:159`) | 묶음을 목록에서 내리고 **그 계정의 저장본 파일을 지운다.** 보고 있던 목록이 묶음 안이면 뿌리로 돌린다 |
| 토큰이 거부됨(`poll_whoami` 가 토큰을 버림, `app_network.rs:302-311`) | 묶음을 내린다. 저장본은 둔다. 같은 계정으로 다시 로그인하면 받기 전에도 바로 보인다 |
| 다른 계정으로 로그인 | 묶음을 내리고 새 계정의 저장본이 있으면 올린 뒤 받기를 시작한다 |
| 받는 도중 계정이 바뀜 | 받기를 시작할 때 `(서버 주소, 로그인 id)` 를 붙여 두고, 걷을 때 지금 값과 다르면 버린다. 저장본 파일은 계정별이라 잘못 쓰여도 다른 계정에 섞이지 않는다 |
| 서버에서 폴더가 지워짐 | 다음 받기에서 색인에 없으므로 저장본과 목록에서 빠진다. 그 폴더를 보고 있었으면 묶음 목록으로 돌린다(`ListPlace::view` 의 "없어진 레벨은 표의 레벨 목록으로" 와 같은 규칙, `tablesrc.rs:1032-1043`) |
| 폴더 이름이 바뀜 | id 가 같으므로 커서와 보고 있던 자리가 유지된다 |
| 서버 주소가 바뀜 | 저장본의 `server` 가 달라 쓰이지 않는다 |

### 7.8 Tables 화면에서의 표시

Tables 화면(`stage/tables.rs`)에 사용자 폴더 줄 하나를 더한다. 지울 수 없는 줄이다.

| 상태 | 줄에 보이는 말(예) |
|---|---|
| 로그인 안 됨 | `MY FOLDERS  -  LOG IN TO LOAD YOUR FOLDERS` |
| 받은 적 없음 | `MY FOLDERS  -  NOT FETCHED YET` |
| 받는 중 | `MY FOLDERS  -  FETCHING...` |
| 받음 | `MY FOLDERS  -  87 OF 120 CHARTS OWNED  -  5 FOLDERS`, 저장본의 나이(`copy_age`, `tablesrc.rs:188`) |
| 받기 실패, 저장본 있음 | 위와 같되 아래에 이유 |
| 받기 실패, 저장본 없음 | `MY FOLDERS  -  {이유}` |

- R 은 이 줄에서 다시 받기, D 는 아무 일도 하지 않고 "웹에서 편집한다"는 안내를 낸다.
- 줄의 자리: 지금 키 처리는 `self.sel < tables.len()` 으로 표 줄을 가린다(`stage/tables.rs:366-367`). 번호 계산을 덜 흔들려면 맨 끝(`+ ADD` 두 줄 다음)에 둔다.

---

## 8. 설계 (라) — 발광 난이도표의 기본 등록

### 8.1 지금

- `library.tables` 의 기본값은 빈 목록이다(`schema.rs:775`).
- 설정 스키마 버전은 5 다(`schema.rs:20`). 파일이 더 새 버전을 말하면 읽기를 거부하고 파일을 건드리지 않는다(`io.rs:96-98`).
- 시작할 때 기본 스킨을 설정 폴더 옆에 깔아 주는 선례가 있다(`install_default_pack`, `apps/rbms-player/src/lib.rs:1401`).

### 8.2 규칙

기본 표마다 **id** 를 붙이고, 설정에 "이미 한 번 넣어 준 기본 표의 id 목록"을 둔다.

```rust
pub struct DefaultTable {
    pub id: &'static str,
    pub location: &'static str,
    pub same_as: &'static [&'static str],
}

pub const DEFAULT_TABLES: &[DefaultTable] = &[DefaultTable {
    id: "insane1",
    location: "https://bms.hyuns.uk/tables/insane1",
    same_as: &["darksabun.club/table/archive/insane1"],
}];
```

`location` 은 A3 가 추천한 사본 주소다(끝 슬래시 없음. 운영 사이트가 끝 슬래시를 308 으로 떼는 것을 A3 가 확인했다). 사용자가 다른 꼴을 고르면 이 값만 바꾼다. 이 문자열은 저장본 파일 이름의 해시 입력이므로 한 번 내보낸 뒤에는 바꾸지 않는다.

`LibraryOptions` 에 필드 하나를 더한다.

```rust
pub seeded_default_tables: Vec<String>,
```

시작할 때(설정을 읽은 직후, `install_default_pack` 근처) 기본 표마다:

1. `seeded_default_tables` 에 id 가 있으면 아무것도 하지 않는다.
2. 없으면 id 를 `seeded_default_tables` 에 넣는다. 그리고
   - `library.tables` 에 같은 표가 이미 있으면(§8.3) 표는 넣지 않는다.
   - 없으면 `TableSource { name: "", location }` 을 목록 **맨 앞**에 넣는다. 이름을 비워 두면 표 자신의 이름(`発狂BMS難易度表`)이 쓰인다(`tablesrc.rs:390-392`).
3. 무엇이든 바뀌었으면 설정을 저장한다.

경우별 결과:

| 경우 | 결과 |
|---|---|
| 새 설치(설정 파일 없음) | `seeded_default_tables` 가 비어 있으므로 표가 들어간다 |
| 기존 사용자(파일에 `seeded_default_tables` 없음) | serde 기본값이 빈 목록이므로 표가 한 번 들어간다 |
| 사용자가 그 표를 지움 | id 가 남아 있으므로 다시 들어가지 않는다 |
| 같은 표의 원본 주소를 이미 등록함 | 표는 넣지 않고 id 만 적는다. 원본 줄은 그대로 둔다 |
| 나중에 기본 표가 늘어남 | 새 id 만 한 번 들어간다. 이미 처리한 표는 건드리지 않는다 |
| 계정에서 설정을 내려받음 | `keep_local` 이 `library.tables` 를 이 컴퓨터의 값으로 지킨다(`ir_sync.rs:51`). **`seeded_default_tables` 도 같이 지키도록 한 줄을 더해야 한다.** 빠뜨리면 다른 컴퓨터의 `seeded_default_tables` 가 덮여 표가 다시 들어가거나 영영 안 들어간다 |

`Config::default()` 의 `tables` 를 바꾸지 않는 이유: 기본값은 테스트, 설정 올리기의 가림(`sanitise_for_upload` 는 `Config::default()` 를 "이 컴퓨터의 값"으로 쓴다 — `ir_sync.rs:59-61`), serde 의 빠진 필드 채움에 두루 쓰인다. 거기에 주소를 넣으면 테스트의 플레이어가 표를 받으려 들고, 손으로 `tables` 를 지운 파일에 표가 되살아난다.

### 8.3 "같은 표"의 판단

주소를 다듬어 견준다: 스킴(`http://`, `https://`)을 떼고, 소문자로 바꾸고, 끝의 `/`, `/index.html`, `/header.json`, `/data.json` 을 뗀다. 그 결과가

- 기본 표의 `location` 을 같은 방법으로 다듬은 값과 같거나,
- `same_as` 의 어느 값과 같으면

같은 표다. 발광 난이도표는 여러 곳에 사본이 있다(원본 헤더의 `original_url` 은 `http://nekokan.dyndns.info/~lobsak/genocide/insane.html` 이다). 주소만으로 그 전부를 알아볼 수는 없다. 알아보지 못해 표가 둘이 되면 사용자가 Tables 화면에서 하나를 지우면 되고, 지운 뒤에는 다시 들어가지 않는다.

### 8.4 처음 실행에서의 동작 — 두 방법

| | A. 스냅샷을 실행 파일에 넣는다 | B. 주소만 등록한다 |
|---|---|---|
| 처음 실행 | 표를 넣을 때 저장본(`<설정>/tables/<해시>.json`)도 함께 쓴다. 네트워크 없이 바로 폴더가 보인다 | 선곡이 뜬 뒤 `start_first_fetches` 가 받는다 |
| 오프라인 첫 실행 | 문제없다 | 시작할 때마다 `table fetch failed (…)` 경고가 한 번씩 뜬다(`tablesrc.rs:503-509`, `:824-827`) |
| 사이트가 내려가 있을 때 | 문제없다 | 표가 없다 |
| 실행 파일 크기 | 필요한 필드만 남기면 약 150~200KB 가 는다(§5.3 의 항목 크기 × 1,035, 레벨 포함) | 변화 없음 |
| 다시 받기 | F6 과 R 이 사본 주소에서 받는다 | 같다 |
| 고칠 곳 | `TableStore` 에 "주어진 표를 이 주소의 저장본으로 쓴다"는 공개 함수 하나(테스트용 `keep_for_tests` 가 같은 일을 한다 — `tablesrc.rs:321-325`) | 없음 |

**추천: A.** 보관본(archive)이라 내용이 바뀌지 않고, 사용자 결정의 취지("원본 사이트가 내려가도 동작")가 클라이언트에서도 성립한다. 저장본의 `fetched_at` 은 비워 둔다(Tables 화면에 `SAVED` 로만 보인다).

A 에 쓰는 것은 A3 가 저장소에 둔 스냅샷이다: `web/data/tables/insane1/header.json`(467바이트)과 `data.json`(400,653바이트, 원본의 필드를 전부 가진다). 둘 다 표준 형식 그대로라 `rbms_table::parse_header` 와 `DifficultyTable::parse_body(본문, Some(헤더))` 로 읽고 `SavedTable::json_of` 로 저장본을 쓴다. 실행 파일에는 `include_bytes!` 로 넣는다(크레이트 밖 경로를 가리키게 된다).

A3 의 문서는 같은 방법을 C안으로 적고 추천하지는 않았다(§7.4: "내장본과 웹 사본이 어긋남(갱신 경로 둘)", 오프라인 첫 실행이 중요하면 추가로 고려). 두 문서의 추천이 다르므로 §11 의 8번에서 정한다. 넣는 파일이 저장소의 스냅샷 그 자체이면 웹 사본과 어긋날 길은 없다(둘 다 같은 파일에서 나온다).

### 8.5 스키마 버전

`seeded_default_tables` 는 더해지는 필드이고 기존 값의 뜻을 바꾸지 않는다. **버전(5)을 올리지 않는 것을 제안한다.** 올리면 옛 실행 파일이 새 설정을 거부한다(`io.rs:96-98`).

올리지 않을 때 남는 틈: 옛 실행 파일로 한 번 저장하면 `seeded_default_tables` 가 파일에서 사라지고(모르는 필드는 읽을 때 버려진다), 그 뒤 새 실행 파일이 표를 다시 넣는다. 사용자가 표를 지운 뒤 옛 판으로 내려갔다 올라온 경우에만 생긴다.

### 8.6 서버 주소의 기본값(관련 사실)

`network.server_url` 의 기본값이 없다(§2.1). 새로 설치한 사용자는 NETWORK 탭에서 주소를 넣고 로그인해야 사용자 폴더가 보인다. 기본 표는 서버 주소와 무관하게 들어간다. 서버 주소에도 기본값을 둘지는 이 문서의 범위 밖이다(§11 의 10번).

---

## 9. 설계 (마) — 보안과 사생활

### 9.1 입력 검증과 출력

| 지점 | 규칙 |
|---|---|
| 폴더 이름·설명·메모 | NFC 로 고르고 앞뒤 공백을 뗀다. 제어 문자(`U+0000–001F`, `U+007F`)가 있으면 거부한다. 길이 한도(§5.3) |
| 제목·아티스트 사본 | 서버가 아는 값(차트 행, 표)을 먼저 쓴다. 요청의 값은 제어 문자를 지우고 200자로 자른다 |
| md5, sha256 | 소문자로 바꾼 뒤 `isMd5Hash`, `isSha256Hash` |
| 폴더 id | `^uf_[0-9a-f]{30}$` |
| 본문 크기 | 편집 요청은 512KB(`parseLimitedJsonBody`) |
| 웹 출력 | React 의 자동 이스케이프에 맡긴다. `dangerouslySetInnerHTML` 을 쓰지 않는다. 이름을 URL·`href`·스타일에 넣지 않는다 |
| JSON 출력 | `NextResponse.json` |
| DB | Drizzle 의 바인딩만. `LIKE` 검색어의 `%`·`_` 는 지금 검색(`compose/fe.ts:63`)이 그대로 넣고 있다. 표 항목 검색을 새로 만들 때는 이스케이프한다 |
| 클라이언트 | 받은 이름을 글자로만 그린다. 길이·개수·md5 모양을 다시 확인한다(§7.5). 폴더 id 를 경로에 넣기 전에 문자 집합을 확인한다(§7.3) |

### 9.2 다른 사용자의 폴더

- 편집 경로는 경로에 사용자 id 가 없다. 폴더 id 로 들어온 요청은 `user_id = 세션 사용자` 조건으로만 찾고, 없으면 404 다.
- 읽기 경로는 `loginId === playerId` 가 아니면 403 이다. 다른 사람의 폴더 목록이 있는지조차 답하지 않는다.
- 테스트로 고정한다: 남의 폴더 id 로 읽기·고치기·지우기·곡 넣기를 하면 전부 "없음"이다.
- 로그에 폴더의 이름이나 내용을 남기지 않는다.

### 9.3 공개 폴더

v1 에서는 열지 않는 것을 제안한다. 이유:

- 사용자 결정에 공유는 없다("사용자 폴더가 할당", "R-BMS 는 읽기만").
- 공개하면 이름·설명·메모와 "서버가 모르는 곡의 제목"이 남에게 보이는 사용자 글이 된다. 신고와 숨김 같은 운영 수단이 지금 없다.

열 때의 모양(뒤 단계의 밑그림):

| 주소 | 내용 |
|---|---|
| `https://bms.hyuns.uk/folders/{folderId}` | 폴더 페이지. `<meta name="bmstable" content="…/header.json">` |
| `…/header.json` | `{ "name": 폴더 이름, "symbol": "", "data_url": "data.json", "level_order": [폴더 이름] }` |
| `…/data.json` | `[{ "md5", "sha256", "title", "artist", "level": 폴더 이름 }]` |

- 주소의 꼴은 A3 가 기본 표에 정한 꼴(`/tables/insane1`, `…/header.json`, `…/data.json`, 메타는 루트 상대 경로)을 따른다.
- 공개 폴더에는 서버가 아는 제목·아티스트만 내보내고, 사용자가 쓴 사본과 메모는 내보내지 않는다.
- `is_public` 컬럼은 지금 만들어 둔다. 나중에 컬럼을 더하는 마이그레이션을 한 번 줄인다.

### 9.4 토큰 범위

- API 토큰에 범위가 없다(§3.2). 클라이언트의 토큰으로 폴더를 **읽을** 수 있고 **고칠** 수는 없다(편집은 세션만).
- 토큰은 클라이언트의 설정 파일에 평문으로 있다(§2.1). 그 파일을 읽을 수 있는 사람은 폴더의 내용도 읽을 수 있다. 저장본 파일 자체도 같은 폴더에 평문으로 있으므로 새로 생기는 노출은 아니다.
- 범위를 나누는 일(읽기 전용 토큰 등)은 이 기능에 꼭 필요하지 않다.

### 9.5 남용 방지

- 한도(§5.3)와 rate limit(§6.7), 본문 크기 한도.
- rate limit 은 인스턴스마다 따로 센다(§3.4). 한도를 넘는 양을 막는 실제 장치는 DB 쪽의 개수 한도다.
- 표에서 가져오기는 서버가 큰 삽입을 하는 유일한 경로다. 분당 10회로 묶고, 한 번에 넣는 수는 폴더당 한도로 막힌다.
- CSRF: 편집 요청은 세션 쿠키(SameSite=Lax, `docs/web/architecture.md` §1-4)와 JSON 본문을 쓴다. 지금의 토큰 발급·폐기 경로(`route/fe.ts:99-120`)와 같은 조건이고 새로 더하는 방어는 없다.

---

## 10. 구현 단위

### 10.1 단위와 파일 소유

| 단위 | 하는 일 | 새 파일 | 고치는 파일 |
|---|---|---|---|
| S1 스키마 | 두 테이블, `meta/` 되살리기, 0001 마이그레이션 | `web/drizzle/0001_*.sql`, `web/drizzle/meta/*` | `web/src/server/db/schema.ts`, `web/.gitignore` |
| S2 서버 도메인 | DTO, 서비스, compose, 에러 코드, rate limit 값 | `web/src/server/dto/folder.ts`, `web/src/server/service/domain/folder/folder.service.ts`, `web/src/server/compose/folder.ts`, `web/tests/service/folder.test.ts`, `web/tests/dto/folder.test.ts` | `web/src/server/compose/index.ts`, `web/src/server/lib/error-code.ts`, `error-message.ts`, `error.ts`, `with-rate-limit.ts` |
| S3 서버 라우트 | 편집 12개, 읽기 2개, 능력 표시 | `web/src/server/route/folder.ts`, `web/src/app/api/fe/me/folders/**/route.ts`, `web/src/app/api/players/[playerId]/folders/route.ts`, `…/folders/[folderId]/route.ts`, `web/tests/route/folders.test.ts` | `web/src/server/service/domain/system/capabilities.ts`, `web/src/server/dto/system.ts`, `web/tests/dto/ir-contract.test.ts` |
| S4 웹 화면 | 폴더 목록·상세, 담기 동작 | `web/src/entities/folder.type.ts`, `folder.query.ts`, `web/src/widgets/folder/*`, `web/src/app/(app)/folders/page.tsx`, `…/folders/[folderId]/page.tsx` | `web/src/shared/constants/query-key.ts`, `navigation.ts`, 차트·검색·표 화면의 담기 버튼 자리 |
| S5 클라 계약 | DTO, 트레이트 메서드, HTTP 구현, 능력 필드 | `crates/rbms-ir/src/dto/folder.rs` | `crates/rbms-ir/src/dto.rs`, `lib.rs`, `http.rs`, `dto/system.rs`, `contract_tests.rs` |
| S6 클라 저장본과 받기 | 저장본, 받기 일꾼, 계기, 로그아웃 처리 | `apps/rbms-player/src/user_folders.rs` | `apps/rbms-player/src/lib.rs`(`AppShared` 필드, `mod`), `app_network.rs` |
| S7 클라 선곡 연결 | `SelectView` 둘, 목록·머리글·스킨 다리·결정 화면·자리 지키기, F6, 스캔 뒤 다시 맞추기 | — | `apps/rbms-player/src/lib.rs`, `stage/select/list.rs`, `scene.rs`, `mod.rs`, `skin_host/select/bridge.rs`, `stage/decide.rs`, `tablesrc.rs`(`ListPlace`), `stage/loading.rs` |
| S8 클라 Tables 화면 | 사용자 폴더 줄과 R | — | `apps/rbms-player/src/stage/tables.rs` |
| S9 기본 표 | `seeded_default_tables`, `DEFAULT_TABLES`, 시작할 때 넣기, 저장본 심기, 내려받기에서 지키기 | 없음(`web/data/tables/insane1/` 의 스냅샷을 `include_bytes!` 로 읽는다) | `crates/rbms-config/src/schema.rs`, `apps/rbms-player/src/lib.rs`(`run`), `tablesrc.rs`(`TableStore`), `ir_sync.rs`(`keep_local`) |
| S10 문서 | API 규격과 구조 문서 반영 | — | `docs/backend/api-spec.md`, `docs/web/architecture.md` §1-2·§1-3, `docs/reference/ir-api.md`, `docs/web/tasks.md` |

### 10.2 순서와 의존

```
S1 → S2 → S3 → S4
              → S5 → S6 → S7 → S8
A3 의 주소·스냅샷 → S9
전부 → S10
```

- S4 와 S5 는 S3 의 응답 모양이 정해지면 동시에 할 수 있다(이 문서의 §6.2, §7.2 가 그 모양이다. S3 을 기다리지 않고 이 문서로 시작해도 된다).
- S6·S7·S8·S9 는 `apps/rbms-player/src/lib.rs`, `tablesrc.rs`, `stage/tables.rs` 를 함께 고친다. **한 작업자가 차례로 하거나, 파일을 나눠 직렬로 한다.** S9 는 기능으로는 독립이지만 `lib.rs` 와 `tablesrc.rs` 가 겹친다.
- 같은 시각에 다른 워크플로가 Rust 소스를 고치고 있다. S6 을 시작하기 전에 `tablesrc.rs` 와 `stage/select/` 의 지금 모습을 다시 읽는다.

### 10.3 단위별 검증

| 단위 | 검증 | 통과 기준 |
|---|---|---|
| S1 | `bun run typecheck`. `bun run db:generate` 를 한 번 더 돌린다 | 두 번째 생성이 "바뀐 것 없음"이다. 0001 에 두 테이블 외의 문장이 없다 |
| S2 | `bun test tests/service/folder.test.ts tests/dto/folder.test.ts` | DB 없이 돈다. `FolderServiceDb` 를 `Map` 으로 만든 인라인 객체로 넣는다(`tests/service/setting.test.ts:9-25` 방식) |
| S3 | `bun test tests/route/folders.test.ts tests/dto/ir-contract.test.ts`, `bun run typecheck` | DB 에 닿기 전에 끝나는 경로(본문 413, 잘못된 폴더 id 400)만 본다 |
| S4 | A2 의 설계가 정한 검사 + `bun test tests/components` | 기존 `entity-contract.test.ts`, `entity-query-wiring.test.ts` 에 폴더를 더한다 |
| S5 | `cargo test -p rbms-ir` | 요청 줄(`GET /players/p1/folders HTTP/1.1`), Bearer 헤더, 응답 해석, 빠진 선택 필드의 기본값, 404 가 `IrError::NotFound` 인지 |
| S6 | `cargo test -p rbms-player user_folders` | `ScoreServer` 가짜로: 바뀐 폴더만 받는다, 색인에 없는 폴더가 빠진다, 일부 실패에서 옛 내용이 남는다, 계정이 바뀐 뒤 도착한 결과를 버린다, 로그아웃이 파일을 지운다, 다른 계정의 파일을 읽지 않는다 |
| S7 | `cargo test -p rbms-player`(선곡·표 관련 기존 테스트 포함) | 묶음 줄이 표 줄 다음에 온다, 폴더 줄의 `(가진 곡/전체)`, 받기가 도착해도 커서와 보고 있던 폴더가 유지된다, 폴더가 지워지면 묶음 목록으로 돌아간다 |
| S8 | `stage/tables.rs` 의 테스트 | 상태별 줄의 말, R 이 받기를 시작한다, D 가 줄을 지우지 않는다 |
| S9 | `cargo test -p rbms-config`, `cargo test -p rbms-player` | §8.2 의 표의 여섯 경우가 각각 테스트다. 저장본이 심긴 뒤 네트워크 없이 표의 폴더가 보인다 |
| 통합 | 로컬 dev 서버와 클라이언트(`crates/rbms-ir/tests/e2e_local.rs` 가 같은 방식의 선례다) | 웹에서 폴더를 만들고 곡을 넣은 뒤 클라이언트가 로그인하면 묶음이 보인다. DB 와 로컬 서버가 필요하므로 사용자의 환경에서 한다 |

S2 가 꼭 덮어야 할 경우:

- 폴더 수 한도, 폴더당 곡 수 한도, 전체 곡 수 한도 — 넘으면 아무것도 넣지 않는다.
- 이미 든 곡과 요청 안의 중복을 건너뛴다.
- 순서 바꾸기는 집합이 다르면 거부한다.
- 남의 폴더 id 는 "없음"이다.
- 이름 겹침이 `'name_taken'` 으로 올라온다.
- 제목·아티스트 사본의 출처 순서(차트 → 표 → 요청)와 다듬기.
- 어떤 바뀜에서든 `updated_at` 이 `now()` 로 오른다.

### 10.4 예상되는 어려움

| 어려움 | 내용 |
|---|---|
| `meta/` 없음 | §3.7, §5.4. journal 의 `when` 을 잘못 두면 운영 적용이 0000 을 다시 돌리다 멈춘다. 운영 DB 의 기록을 볼 수 있는 사람은 사용자뿐이다 |
| 세션 경로의 DB 없는 테스트 | 세션 확인(`getAuth().api.getSession`)이 쿠키 없는 요청에서 DB 를 건드리는지 확인하지 못했다. 건드리면 "세션 없음 401" 을 DB 없이 볼 수 없다. 그 경우 인증보다 앞서는 검증(본문 크기, id 모양)만 라우트 테스트에 두고 나머지는 서비스 테스트와 통합 확인에 맡긴다 |
| 순서 바꾸기의 동시성 | 두 탭이 동시에 곡을 넣고 순서를 바꾸면 `position` 이 겹칠 수 있다. 순서 바꾸기를 "지금 집합과 같아야 한다"로 묶고 한 트랜잭션에서 전부 다시 매긴다 |
| `SelectView` 의 퍼짐 | 선곡·스킨·결정·캡처 테스트까지 여러 파일이 `match` 한다(§7.4). 스킨이 읽는 "열린 폴더 이름"(`bridge.rs:99-114`)에 묶음과 폴더를 어떻게 줄지도 정해야 한다(표는 `[표 이름, "LEVEL n"]` 꼴이다) |
| 자리 지키기 | `ListPlace` 는 표의 레벨을 이름으로 다시 찾는다. 사용자 폴더는 id 로 찾아야 하고, 그 값을 `AppShared` 가 들고 있어야 한다 |
| 큰 응답과 5초 | 폴더 하나가 최악 3.3MB 다. 느린 회선에서는 5초(`http.rs:13`)에 걸릴 수 있다. 폴더 호출만 더 긴 한도의 클라이언트로 부르는 방법(`try_with_timeout`, `http.rs:40`)이 있다 |
| 표에서 가져오기의 출처 | 서버의 지금 표 모델은 sha256 키라 발광 난이도표를 담지 못한다(§3.1). A3 는 DB 에 넣지 않고 스냅샷 파일을 바로 읽는 방법을 추천한다. 그 결정에 따라 `loadTableEntries` 의 구현이 정해진다 |
| Next 16 | `web/AGENTS.md` 의 안내대로 라우트를 쓰기 전에 `node_modules/next/dist/docs/` 의 해당 장을 읽는다. 동적 경로의 `params` 는 Promise 다(기존 라우트가 모두 `await context.params` 를 쓴다) |

---

## 11. 결정이 필요한 곳

| # | 결정 지점 | A안(추천) | 다른 안 |
|---|---|---|---|
| 1 | 클라이언트가 폴더를 읽는 통로 | 전용 API(색인 + 폴더별)와 전용 통로. 비공개 데이터를 주소의 비밀에 기대지 않는다 | 폴더마다 표준 BMS 표 주소로 내보내 표 통로를 탄다 |
| 2 | 선곡 화면의 모양 | 뿌리에 사용자 묶음 한 줄(`MY FOLDERS`) → 폴더 → 곡. 폴더가 많아도 뿌리가 길어지지 않는다 | 폴더마다 뿌리에 한 줄(표와 나란히) |
| 3 | 공개 폴더 | v1 은 전부 비공개. `is_public` 컬럼만 만들어 둔다 | v1 부터 공개와 표준 표 형식의 공유 주소를 연다 |
| 4 | 곡의 식별자 | md5 필수, sha256 선택. 클라이언트가 md5 로만 맞추고 발광 표가 md5 만 가진다 | md5 와 sha256 중 하나만 있어도 받는다(sha256 만 있는 곡은 클라이언트에서 안 보인다) |
| 5 | 한도 | 폴더 50, 폴더당 2,000곡, 전체 20,000곡, 이름 64자 | 다른 값 |
| 6 | 가지고 있지 않은 곡 | 폴더 줄에 `(가진 곡/전체)` 만 보인다. 표와 같은 동작 | 선곡 목록에 빠진 곡을 흐린 줄로 보여 준다(표에도 같이 적용하는 별도 작업) |
| 7 | 로그아웃할 때 저장본 | 지운다. 로그아웃한 계정의 개인 데이터를 컴퓨터에 남기지 않는다 | 남겨 두고 다음 로그인 때 다시 쓴다 |
| 8 | 기본 표를 넣는 방법 | `seeded_default_tables` 로 한 번만 넣고, 저장소의 스냅샷을 실행 파일에 넣어 저장본까지 심는다. 오프라인 첫 실행과 사이트가 내려간 때에도 표가 보인다 | 주소만 등록하고 처음 실행에서 받는다(A3 의 추천) / 스키마를 6으로 올리는 일회성 마이그레이션 |
| 9 | drizzle `meta/` | `web/.gitignore` 의 `/drizzle/meta` 를 빼고 journal 과 스냅샷을 되살려 커밋한다. `docs/web/architecture.md` §3-2 의 "산출 파일은 커밋한다"와 맞다 | 0001 을 손으로 쓰고 운영에는 SQL 을 직접 적용한다 |
| 10 | 서버 주소의 기본값 | 지금대로 둔다(이 기능의 범위 밖). 가이드에서 주소 넣는 법을 안내한다 | `server_url` 기본값을 `https://bms.hyuns.uk/api` 로 둔다. 새 설치에서 로그인만 하면 폴더가 보인다 |
| 11 | 폴더 안의 묶음 | v1 에는 없다. 폴더는 곡의 평평한 목록이다 | 곡에 묶음 이름을 두어 폴더 안에 한 단계를 더 만든다(선곡 화면이 세 단계가 된다) |
| 12 | 묶음의 이름 | `MY FOLDERS` | `{로그인 id} FOLDERS` 또는 계정의 표시 이름 |

---

## 12. 위험과 확인하지 못한 것

### 12.1 위험

- 운영 DB 마이그레이션: `meta/` 를 되살리는 과정에서 journal 의 `when` 을 잘못 두면 `migrate` 가 0000 을 다시 실행하려다 멈춘다. 트랜잭션 안이라 반쯤 적용되지는 않지만 배포 순서가 막힌다.
- 배포 순서: 웹이 새 테이블을 쓰는 코드를 먼저 배포하고 마이그레이션이 늦으면 폴더 경로가 500 을 낸다. 마이그레이션을 먼저 적용한다. Vercel 은 `web/` 이 바뀐 푸시에서 바로 배포하므로 스키마를 쓰는 커밋을 올리기 전에 적용을 끝낸다.
- 옛 클라이언트: 이미 나간 v0.1.0 은 폴더를 모른다. 서버가 더해도 깨지는 것은 없다(새 경로만 생긴다).
- 옛 서버: 새 클라이언트가 폴더 색인을 물으면 404 를 받는다. 조용히 넘어가게 한다(§7.6).
- 사용자 글의 노출: 공개 폴더를 열면 이름·설명이 남에게 보인다(§9.3).
- rate limit 이 인스턴스마다 따로라 실제 한도가 설정값보다 느슨하다(§3.4).
- 동시 작업: 다른 워크플로가 Rust 소스를 고치는 중이다. 이 문서의 줄 번호가 밀릴 수 있다.

### 12.2 확인하지 못한 것

| 무엇 | 이유 |
|---|---|
| 운영 DB 의 `__drizzle_migrations` 내용과 collation | DB 접속 정보를 읽지 않는다 |
| Vercel 함수의 응답 본문 크기 한도 | 이 작업에서 공식 문서를 조회하지 않았다. 폴더당 최악 3.3MB 가 한도 안인지 구현 전에 확인한다 |
| 세션 확인이 쿠키 없는 요청에서 DB 를 건드리는지 | 코드를 실행하지 않았다 |
| A2(웹 화면)의 문서 | 끝까지 저장소에 보이지 않았다. 폴더 화면의 경로(`/folders`, `/folders/[folderId]`)와 메뉴 이름은 A2 의 설계에 맞춰 바꿀 수 있다 |
| `SelectView` 를 쓰는 모든 파일의 내용 | `list.rs`, `scene.rs`, `bridge.rs`, `decide.rs`, `tablesrc.rs` 만 읽었다. `mod.rs` 의 뒤로 가기, `skinned.rs`, `app_result.rs` 는 읽지 않았다 |
| bmson 차트의 md5 가 무엇으로 계산되는지 | `crates/rbms-library/src/scan.rs` 의 해당 부분을 읽지 않았다 |
| 운영 사이트의 지금 응답 | 직접 요청하지 않았다. `/api/tables` 가 `[]` 라는 것은 다른 작업자가 받아 둔 응답이다 |
| 웹과 클라이언트의 테스트가 지금 통과하는지 | 조사만 했고 테스트를 돌리지 않았다 |
