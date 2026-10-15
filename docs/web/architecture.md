# rbms web — 아키텍처 설계 (W0)

> 최종 갱신 2026-10-14 · 대응 커밋 7095ef7

> **현재 구현 기준:** W0~W2 와 웨이브 17B(웹 골격·폴더·발광 표)·20(SCRATCH AUTO 순위)까지 반영했습니다. 실제 라우트는 `web/src/app/{(app),(public),api,tables}/`, 서버 경계는 `web/src/server/`가 정본입니다. 본문에 남은 "착수 시", "향후", 가정은 설계 이력으로만 읽고, 이 상태 블록과 §9를 우선합니다. **운영 배포는 2026-10-14 에 끝났습니다**(https://bms.hyuns.uk, Vercel 프로젝트 `rbms-web`, 운영 DB 마이그레이션 0000~0002 적용, 시험 계정 삭제). 배포 설정은 §6-2, 운영 DB 작업은 §3-2 가 정본입니다. 2026-10-14 점검에서 나온 고장 12건의 해소 현황은 [2026-10-14-audit.md](./2026-10-14-audit.md) 맨 위 표입니다.

> 근거: `docs/acknowledge/2026-09-09-enhancement-decisions.md` §웹·백엔드 단독 서버(W1~W8), `docs/backend/{PRD,api-spec,data-model,endpoint-tasks,compatibility,contract-freeze}.md`, `docs/reference/ir-api.md`, 실제 와이어 `crates/rbms-ir/src/{http.rs,dto.rs}`, `docs/acknowledge/design.md`.
> 컨벤션: `~/.claude/convention/{common,frontend,fsd,query,security,backend,comments}.md`.
> 이 문서는 **설계 정본**이다. 코드는 포함하지 않는다(형식 예시 제외). 확정되지 않은 항목은 **가정**으로 표기했다.

---

## 0. 결정 요약

| 항목 | 값 |
|---|---|
| 형태 | Next.js 단독 서버(App Router). API 는 Route Handler. Hono 미사용 |
| 위치 | 이 레포 `web/` (Rust 크레이트와 병존, 별 패키지) |
| 배포 | Vercel, 도메인 `bms.hyuns.uk`, Node.js 런타임 |
| 클라 base | `--server https://bms.hyuns.uk/api` (contract-freeze B-1 권장안) |
| DB | MySQL + Drizzle(mysql2). 스키마 = `docs/backend/data-model.md` |
| 인증 | better-auth(이메일+비밀번호, 세션 쿠키) + 자체 `api_token`(Bearer, 해시 저장) |
| 응답 | 네이티브 경로 = raw JSON, FE 전용 `/api/fe/*` = envelope |
| 상태 | 서버 상태 = TanStack Query v5(prefetch + HydrationBoundary) |
| UI | Tailwind v4 `@theme`(design.md §16) + shadcn radix-nova |

### 0-1. 버전 (2026-09-09 레지스트리 최신, 설치 시 재확인)

`next 16.3.4` · `react/react-dom 19.2.8` · `tailwindcss 4.3.3` · `drizzle-orm 0.45.2` / `drizzle-kit 0.31.10` · `mysql2 3.24.4` · `better-auth 1.7.3` · `zod 4.5.4` · `@tanstack/react-query 5.102.8` · `shadcn 4.21.0`(CLI) · `sonner 2.0.8` · `react-hook-form 7.87.0` + `@hookform/resolvers 5.9.1`.

런타임·PM 은 **Bun**(W8 가정). Next 빌드/dev 는 `bun run` 으로 구동하되 번들러는 Next 기본(Turbopack).

---

## 1. 라우트 맵 (App Router)

### 1-1. 페이지 — 라우트 그룹 2개 (Surface 는 하나)

설계 당시에는 design.md §1-2 가 두 Surface(A 앱, B 공개)로 시각 토큰을 나눴으나, 2026-10-14 에 **Surface 하나로 합쳤다**(design-direction T1, [../acknowledge/design.md](../acknowledge/design.md) 1-2). 모든 화면이 루트 `app/layout.tsx` 의 `AppShell`(사이드바 + 본문 2열) 안에 있고 `data-surface` 스코프는 없다. `(app)`·`(public)` 은 폴더 정리용이며 두 그룹의 `layout.tsx` 는 삭제됐다. 아래 표의 Surface 열은 설계 이력이다.

| 그룹 | 라우트 | 화면 | Surface | 인증 |
|---|---|---|---|---|
| `(public)` | `/login` | 로그인 | **B**(radius 6px, 그림자 있음, 배경 1단) | 무인증 |
| `(public)` | `/signup` | 가입 | B | 무인증 |
| `(public)` | `/guide` | 클라 연동 가이드(`--server` 설정법·토큰 발급) | B | 무인증 |
| `(app)` | `/` | 홈 대시보드(서버 통계 + 최근 제출 피드) | **A**(radius 0, 3열 셸) | 공개 읽기 |
| `(app)` | `/search` | 차트 검색 | A | 공개 읽기 |
| `(app)` | `/charts/[hash]` | 차트 리더보드(랭킹·메타·내 기록) | A | 공개 읽기 |
| `(app)` | `/players/[id]` | 플레이어 프로필(통계·최근 기록·라이벌) | A | 공개 읽기 |
| `(app)` | `/leaderboards` | 종합 플레이어 랭킹 | A | 공개 읽기 |
| `(app)` | `/tables` · `/tables/[id]` | 번들 표 카드 + IR 난이도표 | A | 공개 읽기 |
| `(app)` | `/tables/insane1` | 発狂BMS難易度表 사본(카드 격자, 내 램프, `bmstable` 메타 = `/tables/insane1/header.json`, noindex). 2026-10-14 웨이브 17B-2(711b518). 이 페이지의 주소 `https://bms.hyuns.uk/tables/insane1` 가 R-BMS 의 기본 등록 표 주소다 | A | 공개 읽기(로그인 시 내 기록) |
| `(app)` | `/folders` · `/folders/[folderId]` | 내 폴더 목록과 명시 저장 편집. 2026-10-14 웨이브 17B-2 | A | **세션 필수**(proxy + `SessionGate`) |
| `(app)` | `/courses/[hash]` | 코스 랭킹 | A | 공개 읽기 |
| `(app)` | `/replays/[id]` | 리플레이 뷰어(µs 이벤트 타임라인) | A | 공개 읽기 |
| `(app)` | `/settings` | 계정 설정(탭: 프로필 / API 토큰 / 동기화 blob / 라이벌) | A | **세션 필수** |
| `(app)` | `/admin/builds` | 빌드 allowlist(FR-13) | A | **세션 + role=admin** |

- `(app)` 루트의 미인증 처리: 공개 읽기 라우트는 통과하고 `/settings`·`/admin/*`·`/folders/*` 만 보호한다. 두 겹이다. ① `web/src/proxy.ts`(Next 16 의 proxy, matcher `/settings/:path*`·`/admin/:path*`·`/folders/:path*`)가 better-auth 세션 쿠키가 없으면 `/login?next=` 로 진짜 리다이렉트한다(세션 확인은 DB 접속 없이 쿠키 존재만 본다. S19 부터 같은 proxy 가 `/charts/[hash]`·`/players/[id]`·`/courses/[hash]`·`/replays/[id]` 의 존재도 DB 로 확인해 없으면 404 로 재작성한다. §5-4). ② 각 세그먼트 `layout.tsx` 의 `SessionGate`(`widgets/auth/session-gate.tsx`)가 서버에서 세션을 다시 읽어 없으면 `redirect`, `/admin` 은 `role=admin` 이 아니면 `/` 로 보낸다(쿠키가 있어도 만료·위조면 여기서 막힌다). 2026-10-14 점검 당시의 "200 + meta refresh" 는 d38b6fb 에서 해소됐다.
- 셸은 design.md §8-1 의 2열이다(사이드바 `16rem`, 접으면 `3rem`, 모바일 시트 `18rem` / 본문, 페이지마다 48px 머리 하나). 우측 컨텍스트 패널은 폴더 편집 화면(`folder-editor-workspace.tsx` 의 `곡 추가` aside, `xl` 이상)에서만 쓴다. 옛 3열(256px / fluid / 320px 필터 패널)과 `FilterPanel`·`NavRail` 은 d38b6fb 에서 사라졌다.

### 1-2. Route Handler 맵 — 네이티브(계약 동결분, raw JSON)

아래 표는 `web/src/app/api`의 실제 Route Handler 전부를 담습니다. 그중 `crates/rbms-ir/src/http.rs`가 호출하는 경로는 contract-freeze A와 대조했습니다.

| 메서드 | 경로 | 파일 | 인증 | 캐시 태그(읽기) |
|---|---|---|---|---|
| GET | `/api/health` | `app/api/health/route.ts` | 없음 | 없음(항상 동적) |
| GET | `/api/version` | `app/api/version/route.ts` | 없음 | 없음 |
| POST | `/api/auth/register` | `app/api/auth/register/route.ts` | 없음 | — |
| POST | `/api/auth/login` | `app/api/auth/login/route.ts` | 없음 | — |
| GET/POST | `/api/auth/[...all]` | `app/api/auth/[...all]/route.ts` | better-auth 핸들러 | — |
| GET | `/api/auth/me` | `app/api/auth/me/route.ts` | Bearer 또는 세션 | 없음 |
| POST | `/api/auth/token` | `app/api/auth/token/route.ts` | 세션 | — |
| DELETE | `/api/auth/token/current` | `app/api/auth/token/current/route.ts` | Bearer(폐기할 토큰 자신) | — |
| POST | `/api/scores` | `app/api/scores/route.ts` | Bearer 또는 guest | — |
| GET | `/api/scores/[scoreId]` | `app/api/scores/[scoreId]/route.ts` | 없음 | `score:{id}` |
| GET | `/api/charts/[hash]` | `app/api/charts/[hash]/route.ts` | 없음 | `chart:{sha}` |
| GET | `/api/charts` | `app/api/charts/route.ts` | 없음 | `chart-search` |
| POST | `/api/charts` | `app/api/charts/route.ts` | Bearer | — |
| GET | `/api/charts/[hash]/ranking` | `.../ranking/route.ts` | 없음 | `chart-ranking:{sha}` |
| GET | `/api/charts/[hash]/best` | `.../best/route.ts` | 없음 | `chart-best:{sha}` |
| POST | `/api/charts/[hash]/replays` | `.../replays/route.ts` | Bearer | — |
| GET | `/api/charts/[hash]/replays` | `.../replays/route.ts` | 없음 | `chart-replays:{sha}` |
| GET | `/api/replays/[replayId]` | `app/api/replays/[replayId]/route.ts` | 없음 | `replay:{id}` |
| GET | `/api/players/[playerId]` | `app/api/players/[playerId]/route.ts` | 없음 | `player:{id}` |
| GET/PUT | `/api/players/[playerId]/rivals` | `.../rivals/route.ts` | GET 없음 / PUT 세션·본인 | `player-rivals:{id}` |
| GET | `/api/players/[playerId]/scores` | `.../scores/route.ts` | 없음 | `player-scores:{id}` |
| GET/PUT | `/api/players/[playerId]/settings/[name]` | `.../settings/[name]/route.ts` | Bearer·본인 | 없음(개인·비캐시) |
| POST | `/api/courses` | `app/api/courses/route.ts` | Bearer | — |
| POST | `/api/courses/meta` | `app/api/courses/meta/route.ts` | admin 세션(`withAdmin`) | — |
| GET | `/api/courses/[courseHash]` | `.../[courseHash]/route.ts` | 없음 | `course:{hash}` |
| POST | `/api/courses/[courseHash]/scores` | `.../scores/route.ts` | Bearer | — |
| GET | `/api/courses/[courseHash]/ranking` | `.../ranking/route.ts` | 없음 | `course-ranking:{hash}` |
| GET | `/api/courses/[courseHash]/best` | `.../best/route.ts` | 없음 | `course-best:{hash}` |
| GET/POST | `/api/tables` | `app/api/tables/route.ts` | GET 없음 / POST admin | `tables` |
| GET | `/api/tables/[tableId]` | `.../[tableId]/route.ts` | 없음 | `table:{id}` |
| GET/POST | `/api/admin/builds` | `app/api/admin/builds/route.ts` | admin 세션만(`withAdmin`). 릴리스 CI 는 아직 이 경로로 등록하지 않는다(KNOWN ISSUE) | `client-builds` |

**클라 호출 대조(contract-freeze A 전수)**: `/health` `/scores` `/charts/{md5}/ranking?limit=` `/charts/{md5}/best?player=` `/players/{id}` `/players/{id}/rivals` `/courses` `/charts/{md5}/replays` `/courses/{hash}/ranking?limit=` `/replays/{id}` `/players/{id}/settings/{name}`(GET/PUT) `/auth/register` `/auth/login` — 14개 전부 위 표에 존재. 누락 없음.

> **주의 1 — `POST /api/courses` 의 이중 의미.** 클라(`submit_course`)는 `POST /courses` 에 **CourseSubmission** 을 보내고 `SubmitResponse` 를 기대한다(`http.rs:131`). api-spec §6 은 같은 경로를 "코스 메타 업서트"로 쓴다. **계약 우선 원칙(contract-freeze 원칙)**에 따라 `POST /api/courses` = **코스 결과 제출**로 확정하고, 메타 업서트는 `POST /api/courses/meta` 로 분리한다. 제출 본문은 `course_hash` 를 담고 있으므로 라우팅 모호성 없음.
> **주의 2 — better-auth 경로 충돌 없음.** better-auth 기본 경로는 `/api/auth/sign-in/email` · `/sign-up/email` · `/get-session` · `/sign-out` 이라 `register`/`login`/`me`/`token` 과 겹치지 않는다. 겹치더라도 Next 라우팅은 **정적 세그먼트 > 동적 > catch-all** 순으로 우선하므로 `app/api/auth/register/route.ts` 가 `[...all]` 보다 먼저 매칭된다.
> **주의 3 — 세 라우트의 성공 응답 모양(클라가 실제로 읽는 필드).**
> - `POST /api/scores` · `POST /api/courses` · `POST /api/courses/[courseHash]/scores` → `SubmitResponse` 8필드 전부: `accepted`·`rank`·`previous_best`·`message` + `ranked`·`flags`·`is_new_best`·`score_id`(`dto/score.ts submitResponseSchema`). `score_id` 는 리플레이를 스코어에 연결하는 키라 생략할 수 없다.
> - `POST /api/courses` · `POST /api/courses/[courseHash]/scores` 본문의 선택 필드 `scratch_auto`(boolean)·`assist`(string[]) 는 코스 전체로 접은 어시스트다(단일 곡의 `options.scratch_auto`·`options.assist` 와 같은 어휘, 없으면 false·[]). 넓힌 판정이 낀 코스(`assist` 에 `AUTO_SCRATCH` 밖의 태그)는 단일 곡과 같이 `ranked=false` + `flags`(`ASSIST`, SCRATCH AUTO 도 끼었으면 `SCRATCH_AUTO` 표지까지), `rank=null`, `is_new_best=false` 이고 `course_best` 에 들어가지 않는다. SCRATCH AUTO 만 낀 코스는 웨이브 20부터 `ranked=true` + `flags:["SCRATCH_AUTO"]` 이고 `course_best` 의 SCRATCH AUTO 칸에 들어가며, `/courses/[hash]` 순위 화면의 "SCRATCH AUTO 포함" 토글을 켜야 보인다. `rank` 는 그 포함 목록 기준이다(api-spec §6).
> - `PUT /api/players/[playerId]/settings/[name]` → 200 `{ updated_at }`(서버가 저장한 stamp, unix ms). 클라가 보낸 `updated_at` 은 무시되므로 이 값이 다음 조건부 쓰기의 낙관적 잠금 base 다. 잠금에서 밀리면 409 `{ conflict, server }`.
> - `GET /api/replays/[replayId]` → `ReplayData` 전체. `gauge` 와 차트의 `md5`+`sha256` 을 포함해야 다운로드한 리플레이가 원래 런을 재현한다.
>
> **주의 4 — `/api/charts/search` 금지.** api-spec §12 의 `/api/charts/search` 는 `[hash]` 동적 세그먼트와 충돌 가능성이 있고 envelope 정책도 다르다. FE 검색은 `/api/fe/charts/search` 로 둔다.

### 1-3. Route Handler 맵 — FE 전용(`/api/fe/*`, envelope)

전부 `{ success, data, pagination? }` 봉투. 세션 쿠키 기준(공개 읽기는 무인증).

| 메서드 | 경로 | 용도 | 인증 | 캐시 태그 |
|---|---|---|---|---|
| GET | `/api/fe/charts/search` | 차트 검색(q·mode·level·sort·page·limit) | 없음 | `chart-search`(+쿼리 인자) |
| GET | `/api/fe/charts/[hash]/leaderboard` | 리더보드(랭킹 + 차트 메타 조인 + player_name·replay_id). `include_scratch_auto=true` 면 SCRATCH AUTO 판 포함 목록 | 없음 | `chart-ranking:{sha}` |
| GET | `/api/fe/activity/recent` | 전체 최근 제출 피드(ranked 이고 SCRATCH AUTO 가 아닌 판) | 없음 | `activity-recent` |
| GET | `/api/fe/leaderboards/players` | 종합 플레이어 랭킹. `include_scratch_auto=true` 면 SCRATCH AUTO 판까지 센다 | 없음 | `leaderboard-players` |
| GET | `/api/fe/players/[playerId]/recent` | 플레이어 최근 플레이(차트 조인) | 없음 | `player-recent:{id}` |
| GET | `/api/fe/players/[playerId]/stats` | 플레이어 통계(램프 분포·레벨별 EX·플레이수) | 없음 | `player-stats:{id}` |
| GET | `/api/fe/stats/summary` | 서버 통계(차트/플레이어/제출 수) | 없음 | `stats-summary` |
| POST | `/api/fe/auth/login` | 웹 로그인(`{ id, password }` → 세션 쿠키만 세팅하고 `{ user: { id, login_id, name, role } }`. API 토큰은 만들지 않음. 5/min/IP, 게임용 `/api/auth/login` 과 같은 버킷) | 없음 | — |
| POST | `/api/fe/auth/register` | 웹 가입(`{ id, email, password, name? }` → 201, 세션 쿠키만 세팅하고 `{ user }`. API 토큰은 만들지 않음. 5/min/IP, `/api/auth/register` 와 같은 버킷) | 없음 | — |
| GET/POST | `/api/fe/tokens` | 내 API 토큰 목록 / 발급 | 세션 | 없음 |
| DELETE | `/api/fe/tokens/[tokenId]` | 토큰 폐기 | 세션·본인 | — |
| GET | `/api/fe/me/settings` | 내 설정 blob 키 목록 | 세션 | 없음 |
| GET | `/api/fe/tables` | 번들된 표 목록(요약) | 없음 | 정적(배포 단위) |
| GET | `/api/fe/tables/[tableId]` | 표 머리·출처·레벨별 곡 전체 | 없음 | 정적(배포 단위) |
| GET | `/api/fe/tables/[tableId]/entries` | 표 안의 곡 검색(q·level·page·limit) | 없음 | 정적(배포 단위) |
| GET | `/api/fe/tables/[tableId]/records` | 내 곡별 최고 기록(`chart.md5` 로 맞춤) | 세션 | 없음 |
| GET/POST | `/api/fe/me/folders` | 내 폴더 목록 / 만들기 | 세션 | 없음 |
| PUT | `/api/fe/me/folders/order` | 폴더 순서 바꾸기 | 세션 | 없음 |
| GET | `/api/fe/me/folders/containing/[md5]` | 그 곡이 든 내 폴더 id 목록 | 세션 | 없음 |
| GET/PATCH/DELETE | `/api/fe/me/folders/[folderId]` | 폴더 상세(곡 포함) / 이름·설명 / 삭제 | 세션·본인 | 없음 |
| POST/PUT | `/api/fe/me/folders/[folderId]/items` | 곡 넣기(최대 500) / 곡 목록 통째로 바꾸기(저장 한 번, `base_updated_at` 충돌 검사) | 세션·본인 | 없음 |
| POST | `/api/fe/me/folders/[folderId]/items/remove` | 곡 빼기 | 세션·본인 | 없음 |
| PUT | `/api/fe/me/folders/[folderId]/items/order` | 곡 순서 바꾸기(지금 집합과 같아야 함) | 세션·본인 | 없음 |
| PATCH | `/api/fe/me/folders/[folderId]/items/[md5]` | 곡 메모 고치기 | 세션·본인 | 없음 |
| POST | `/api/fe/me/folders/[folderId]/import` | 표에서 가져오기(`table_id`, `levels?`) | 세션·본인 | 없음 |

네이티브 클라이언트가 읽는 폴더 경로(봉투 없음, Bearer 또는 세션, 본인만, 캐시하지 않음): `GET /api/players/[playerId]/folders`(`{ folders: [...] }`)와 `GET /api/players/[playerId]/folders/[folderId]`(폴더 하나와 `items`). 응답 필드는 `docs/plan/2026-10-14-web-user-folders.md` §7.2 와 같고 `GET /api/health` 의 `capabilities.folders` 가 `true` 다. API 토큰으로는 **읽기만** 되고 편집(`/api/fe/me/folders…`)은 세션만 받는다.

표준 BMS 표 형식(루트 `src/app/tables/[tableId]/`, 봉투 없음, 무인증, `Access-Control-Allow-Origin: *`): `GET /tables/insane1/header.json`(`data_url` 은 **요청 origin 의 절대 주소**, `mirror_of` 로 원본 보관본을 밝힘)과 `GET /tables/insane1/data.json`(스냅샷 배열 그대로. `ETag` 와 304 지원, `Cache-Control: public, max-age=3600, s-maxage=3600, stale-while-revalidate=86400`). 자료는 `web/data/tables/<id>/{header,data,source}.json` 을 **정적 import** 해서 쓰므로 DB 도 파일 추적 설정(`outputFileTracingIncludes`)도 필요 없다(서버 번들에 실린다). 표를 더하려면 폴더를 만들고 `table-snapshot.data.ts` 의 `RAW_SNAPSHOTS` 에 한 줄을 더한다. 사람이 보는 페이지 `/tables/insane1`(`bmstable` 메타는 `/tables/insane1/header.json` 루트 상대 경로)은 웨이브 17B-2 에서 구현됐다.

내 기록 조회(`/records`)는 표의 md5 전부(1,035개)를 `IN` 한 번으로 묻고, 기록이 있는 곡만 `{ md5: { clear, ex_score, minbp, max_combo, total_notes, played_at, scratch_auto } }` 로 돌려준다(최대 약 150KB). 곡마다 일반 판 최고와 SCRATCH AUTO 판 최고 가운데 더 높은 것이고, 화면은 SCRATCH AUTO 쪽이면 A-SCR 표지를 붙인다(웨이브 20). 표 상세는 약 380KB(gzip 전)이고 공개 캐시(`s-maxage=3600`)다. `GET /api/fe/leaderboards/players` 는 `rank_points` 를 쓰지 않는다: `sort` 는 `cleared`(클리어한 차트 수, 기본)·`played`(플레이한 차트 수)·`plays`(제출 수)이고 옛 값 `rank_points`·`total_plays` 는 각각 `cleared`·`plays` 로 받는다. 항목에 `cleared_charts`·`played_charts` 가 더해진다(클리어 = `chart_best.clear >= Easy`).

### 1-4. 인증 방식 요약 표

| 대상 | 자격 | 검증 위치 |
|---|---|---|
| 네이티브 클라(rbms-player) | `Authorization: Bearer <api_token>` | `withApiToken` HOF → `api_token.token_hash` 조회 |
| 웹 FE | better-auth 세션 쿠키(HttpOnly·SameSite=Lax) | `withSession` HOF → `auth.api.getSession({ headers })` |
| guest 제출 | 자격 없음 + `ALLOW_GUEST=true` | `POST /api/scores` 만 허용, 항상 `ranked=false` + `flags:["GUEST"]` |
| admin | 세션 + `user.role='admin'` | `withAdmin` |
| 릴리스 CI(빌드 등록) | 미구현. `withAdmin` 이 세션만 받아 API 토큰으로는 등록할 수 없고 `release.yml` 도 호출하지 않는다 | `POST /api/admin/builds` |

`GET /api/auth/me` 는 **Bearer 와 세션 둘 다** 수용한다(네이티브·FE 공용). 그 외 네이티브 경로는 Bearer, FE 경로는 세션.

---

## 2. 디렉터리 구조 (FSD + 서버 계층)

```
web/
  package.json  next.config.ts  tsconfig.json  drizzle.config.ts  components.json
  vercel.json  bunfig.toml  .env.example  AGENTS.md
  data/tables/<id>/             표 스냅샷(header.json · data.json · source.json)
  scripts/                      db-migration-status.ts · run-sql.ts · snapshot-table.ts · cleanup-test-accounts.{count,delete}.sql · chart-notes.{count,repair,restore,drop-backup}.sql
  drizzle/                            drizzle-kit 산출 마이그레이션(커밋)
  src/
    app/                              ← FSD app 레이어 = Next App Router
      layout.tsx                      html/body, 프로바이더(next-themes 기본 다크), AppShell
      error.tsx  global-error.tsx  not-found.tsx  robots.ts  sitemap.ts  favicon.ico  icon.png  apple-icon.png(assets/icon/tools/generate-icon.py 가 만든다)
      (public)/                       login/page.tsx  signup/page.tsx  guide/page.tsx
      (app)/                          (layout.tsx 없음)
        page.tsx  search/page.tsx
        folders/page.tsx  folders/[folderId]/page.tsx  tables/insane1/page.tsx
        charts/[hash]/page.tsx
        players/[playerId]/page.tsx
        leaderboards/page.tsx
        tables/page.tsx  tables/[tableId]/page.tsx
        courses/[courseHash]/page.tsx
        replays/[replayId]/page.tsx
        settings/page.tsx
        admin/builds/page.tsx
      api/…                           ← §1-2·§1-3 Route Handler (얇은 어댑터만)
      tables/[tableId]/{header,data}.json   표준 BMS 표 형식 Route Handler (§1-3)
    proxy.ts                          세션 쿠키 없는 /settings·/admin·/folders 요청을 /login 으로 (§1-1), 없는 차트·플레이어·코스·리플레이 상세를 404 로 (§5-4)
    widgets/                          비즈니스 로직(쿼리 훅·mutation·권한) 조립
    features/                         순수 UI(props+콜백)
    entities/                         데이터 계층
      chart.api.ts  chart.query.ts  chart.type.ts
      score.* player.* replay.* course.* table.* auth.* token.* settings.* stats.*
    shared/
      ui/                             shadcn 산출물(components.json 의 aliases.ui)
      lib/  constants/  store/
    server/                           ← 서버 전용(클라 번들 반입 금지)
      route/                          Route Handler 팩토리(도메인별 핸들러 조립)
      service/
        domain/{admin,auth,chart,course,fe,folder,player,replay,score,setting,system,table,table-snapshot}/
        shared/{storage,integrity}/        (rate limit 은 lib/with-rate-limit.ts)
      dto/                            zod 스키마(입력·응답)
      db/  index.ts  schema.ts
      lib/  error-code.ts error-message.ts error.ts api-response.ts
            with-error-handling.ts with-api-token.ts with-session.ts
            with-session.ts(withSession · withAdmin) with-rate-limit.ts env.ts auth.ts
      compose/                        의존성 조립(도메인별 ServiceDb 구현)
  tests/                              bun test (components · db · dto · lib · route · service · support)
```

### 2-1. Path alias (= 레이어, `fsd.md` §5)

```jsonc
{
  "@/*":         ["./src/*"],
  "@app/*":      ["./src/app/*"],
  "@widgets/*":  ["./src/widgets/*"],
  "@features/*": ["./src/features/*"],
  "@entities/*": ["./src/entities/*"],
  "@shared/*":   ["./src/shared/*"],
  "@server/*":   ["./src/server/*"]
}
```

- 의존 방향: `app → widgets → features → entities → shared`. 역참조 금지, barrel(`index.ts`) 금지. `pages` 레이어는 없다(Next App Router 가 `app` 에 통합). `entities/` 에는 `*.api.ts` 가 없고 `*.query.ts` 가 `shared/lib/fetch.ts` 의 `fetchEnvelope`·`fetchEnvelopePage`·`fetchRaw`·`fetchNoContent` 를 직접 쓴다(`clientFetch` 라는 이름의 함수는 없다).
- `@server/*` 는 **`app/api/**` 와 서버 컴포넌트에서만** import 한다. `widgets`/`features`/`entities` 는 절대 import 하지 않는다(번들 유출 방지). 파일 상단 `import 'server-only'` 로 강제.
- 브라우저/서버 양쪽에서 도는 fetch 래퍼는 `shared/lib/fetch.ts`(`fetchEnvelope` 등)에 있고 `entities/*.query.ts` 가 부른다(설계 당시의 `entities/*.api.ts` 는 만들지 않았다). Drizzle 은 `@server/db` 안에서만.

### 2-2. 계층 규칙 (backend.md 를 Route Handler 에 적용)

```
Route Handler (app/api/**/route.ts)
   → @server/route/<domain>.ts        HTTP 경계: DTO 검증 · 인증 HOF · 에러 throw · 응답 헬퍼
      → @server/service/domain/<d>    도메인 로직. HTTP·Drizzle 모름. 없음 = null 반환
         → *ServiceDb (compose 구현)  Drizzle 쿼리 격리. 트랜잭션은 여기서만
```

- `route.ts` 파일 자체는 **팩토리 호출 + export** 만 담는 3~5줄 어댑터. 로직은 `@server/route/<domain>.ts` 에 둔다(Next 파일 규약과 계층 규칙을 동시에 만족).
- 모든 핸들러는 `withErrorHandling` 로 감싼다. 합성 순서는 바깥 `withErrorHandling` → 안쪽 `withApiToken`/`withSession`/`withAdmin` → `withRateLimit`.
- 에러는 `createAppError(code)` 만 throw. 코드는 `IR_*` 접두사 + 공통(`UNAUTHORIZED`·`FORBIDDEN`·`VALIDATION_ERROR`·`RATE_LIMITED`·`SERVICE_NOT_CONFIGURED`). 3파일 중앙화.
- 응답: 네이티브 경로는 `irRaw(data)`(봉투 없음, `NextResponse.json`), FE 경로는 `successResponse`/`paginatedResponse`. 실패는 **양쪽 다** `errorResponse`(클라는 body 를 텍스트로 표면화하므로 무해 — `http.rs` 는 실패 body 를 파싱하지 않는다).

---

## 3. DB / Drizzle

### 3-1. 스키마 소유권

| 영역 | 소유 | 파일 |
|---|---|---|
| `user` · `session` · `account` · `verification` | **better-auth**(Drizzle adapter, `provider: 'mysql'`) | `npx @better-auth/cli generate` 산출 → `src/server/db/auth-schema.ts` |
| 도메인 전부(`chart` `score` `chart_best` `replay` `api_token` `rival` `setting_blob` `course*` `difficulty_table` `table_*` `client_build` `submission_audit` `user_folder` `user_folder_item`) | **Drizzle 수기** | `src/server/db/schema.ts` (= `docs/backend/data-model.md` 그대로) |

- contract-freeze D-2 결정 유지. 도메인 FK 는 `user.id` 참조.
- `user` 의 IR 확장 필드(`rank`·`rank_points`·`total_plays`·`allow_guest_merge`·`extra`)는 better-auth `user.additionalFields` 로 선언해 어댑터가 같은 테이블에 들고 있게 한다. **가정**: data-model.md 의 `user.id varchar(64)` = 로그인 id 를 그대로 better-auth PK 로 쓴다(better-auth 기본은 생성 id). 어긋나면 §7 오픈 질문 참조.
- `password_hash` 는 better-auth 가 `account` 테이블에 보관하므로 `user.password_hash` 컬럼은 **두지 않는다**(data-model.md 대비 유일한 의도적 이탈).

### 3-2. 마이그레이션 운영

- `drizzle.config.ts`: `schema: ['./src/server/db/schema.ts', './src/server/db/auth-schema.ts']`, `dialect: 'mysql'`, `out: './drizzle'`.
- 흐름: `bunx @better-auth/cli generate` → `bun run db:generate` → `bun run db:migrate`. 산출 파일(`drizzle/*.sql` 과 **`drizzle/meta/`**)은 **커밋**한다. `meta/` 가 없으면 `db:migrate` 는 `Can't find meta/_journal.json` 으로 멈추고 `db:generate` 는 21개 테이블 전체를 다시 만든다(2026-10-14 까지 `.gitignore` 가 `/drizzle/meta` 를 막고 있었다. 이제 풀었다).
- **`drizzle-kit push` 금지**(운영 DB 히스토리 유실). 로컬 실험도 generate/migrate 로 통일.
- 현재 파일(`web/drizzle/`, `meta/_journal.json` 과 스냅샷 3개도 커밋): `0000_smooth_cerebro`(21 테이블) → `0001_user_folders`(`user_folder`, `user_folder_item` 과 FK 둘, 인덱스 셋) → `0002_scratch_auto_bests`(`chart_best`·`course_best`·`course_score` 에 `scratch_auto` 열, `chart_best`·`course_best` 의 PK 를 `scratch_auto` 포함으로, `chart_best_rank_idx` 재생성). **셋 모두 운영 DB 에 적용됐다(2026-10-14, `web-db.yml` 의 `migrate`).** 그 뒤 `0003_replay_user_quota`(`replay.user_id` 색인, S19) → `0004_course_score_judge_totals`(`course_score` 에 판정 합계 `pgreat`·`great`·`good`·`bad`·`poor`·`miss`, `fast`·`slow`·`combobreak`·`empty_poor`·`max_ex_score` 열 추가, 모두 `int DEFAULT 0 NOT NULL`, S19 2차 R2-ACC-3)가 있고 둘 다 운영에 아직 적용하지 않았다. 저널 `when`: 0000 = 1788220800000, 0001 = 1791947162271, 0002 = 1791970074354, 0003 = 1791999623784, 0004 = 1792027422523 (단조 증가).
- **0003·0004 의 운영 적용 순서(푸시를 두 번에 나눈다)**: Production Branch 가 `dev` 라 `web/` 이 바뀐 푸시는 곧바로 배포되고, `web-db.yml` 의 `migrate` 는 `dev` 에 올라간 마이그레이션 파일을 읽는다. 그래서 마이그레이션 파일과 새 코드를 한 푸시에 담으면 migrate 가 끝날 때까지 새 코드가 없는 열을 읽어 코스 제출·코스 순위·코스 best 가 500 이 된다. 순서: ① 아직 `dev` 에 없는 마이그레이션 파일만(`web/drizzle/0004_course_score_judge_totals.sql`, `web/drizzle/meta/0004_snapshot.json`, `web/drizzle/meta/_journal.json`. 0003 은 이미 `dev` 에 있다) 담은 커밋을 먼저 `dev` 에 푸시한다. 이 배포는 옛 코드 그대로라(옛 코드는 새 열·색인을 읽지도 쓰지도 않는다) 안전하다 → ② `gh workflow run web-db.yml --ref dev -f action=status` 로 0003·0004 가 `would_run=yes` 인지 본다 → ③ `-f action=migrate`(0003 색인, 0004 열 추가. 둘 다 기존 행을 바꾸지 않는 추가뿐이고 0004 의 새 열은 기본값 0) → ④ 다시 `status` 가 최신인지 본다 → ⑤ 그 다음에 `web/src` 와 테스트를 담은 코드 커밋을 푸시한다. ①과 ⑤ 사이에 `web/src` 를 바꾼 커밋이 끼면 같은 문제가 생긴다.
- **저널 `when` 의 규칙**: 설치된 drizzle-orm 0.45.2 의 마이그레이터(`mysql-core/dialect.js` 의 `migrate`)는 `__drizzle_migrations` 에서 `created_at` 이 가장 큰 행 하나만 읽고, 저널 항목의 `when` 이 그 값보다 **큰** 항목만 실행한다(해시는 비교하지 않는다. `drizzle-kit migrate` 는 `drizzle-orm/mysql2/migrator` 를 그대로 부른다). 그래서 복원한 `0000` 의 `when` 은 운영에 적용된 값(2026-09-09 근처)보다 작은 **2026-09-01 00:00 UTC(`1788220800000`)** 로 두었다. 운영에서 `0000` 은 건너뛰고 `0001`(`when` = 생성 시각)만 실행된다. 새 마이그레이션은 `db:generate` 가 현재 시각으로 `when` 을 붙이므로 항상 더 크다.
- **운영 DB 의 상태 확인**: `bun scripts/db-migration-status.ts`(환경 변수의 DB 로 접속, 읽기 전용 쿼리만. 해시 앞 8자·`created_at`·파일별 적용 여부·새 테이블의 존재만 출력하고 접속 정보와 자료는 출력하지 않는다). 종료 코드 0 = 최신, 1 = 할 일이 있음(마이그레이션 대기 또는 기준선 필요), 2 = 접속 실패.
- **운영이 `push` 로 만들어져 `__drizzle_migrations` 가 없다면(기준선 절차)**: `db:migrate` 를 먼저 돌리면 `0000` 의 `CREATE TABLE` 이 "이미 있음" 으로 실패한다. 상태 스크립트가 이 경우를 알려 주면 ① `bun scripts/db-migration-status.ts --baseline-sql` 의 출력(`__drizzle_migrations` 테이블 생성과 `0000` 의 해시·`when` 한 행 삽입)을 DB 에 한 번 적용하고 ② 상태를 다시 확인한 뒤 ③ `bun run db:migrate` 를 돌린다. 이 절차는 로컬 MySQL 에서 `0000` 을 직접 적용해 만든 push 상태의 DB 로 확인했다.
- **운영 DB 작업 워크플로(2026-10-14, 운영 DB 는 GitHub 러너에서 접속됨 — 확인됨. 결정 근거: [../acknowledge/2026-10-14-adr-production-db-operations.md](../acknowledge/2026-10-14-adr-production-db-operations.md))**: `.github/workflows/web-db.yml`(수동 실행). 운영 `DATABASE_URL` 은 Vercel 에 Secret 형식으로 있어 `vercel env pull`·`vercel env run` 으로 받을 수 없으므로(확인: "2 Secret values cannot be pulled"), 저장소 시크릿 `WEB_DATABASE_URL` 로 둔다. 동작: `status`(상태 확인), `migrate`(상태 → 적용 → 상태), `cleanup-count`, `cleanup-delete`(세기 → 지우기 → 다시 세기), `chart-notes-count`, `chart-notes-repair`·`chart-notes-restore`·`chart-notes-drop-backup`(세기 → 그 SQL → 다시 세기. 복구·되돌리기·백업 삭제, S19 3차 R3-WEB-3 에 되돌리기·백업 삭제를 더함. 쓰임새는 테스트 부채 3.36). 값은 출력하지 않는다. 실행: `gh workflow run web-db.yml --ref dev -f action=<동작>`. SQL 파일은 `web/scripts/run-sql.ts`(`web/scripts/` 아래의 `.sql` 만, 여러 문장 허용)로 돈다. (`web-db.yml` 의 `migrate`·`cleanup-delete` 는 실제로 돌렸고, 시험 계정 5개는 삭제됐다. 남은 데이터는 다른 사용자 점수 3건.) 운영 DB 가 러너의 접속을 받지 않게 되면 같은 명령을 로컬에서 `DATABASE_URL` 을 환경에만 주고 실행한다. 이 저장소에서 `.env` 계열은 읽지 않는다.
- **적용 순서**: 마이그레이션을 **먼저** 운영 DB 에 적용하고 **그 다음** 웹을 배포한다. 새 테이블을 쓰는 코드가 먼저 나가면 폴더 경로가 500 을 낸다. `dev` 푸시가 곧 배포이므로 "먼저 적용" 은 마이그레이션 파일만 담은 커밋을 먼저 푸시하고 migrate 한 뒤 코드 커밋을 푸시하는 두 번의 푸시로만 지킬 수 있다(위 0003·0004 순서). 마이그레이션은 빌드에 포함하지 않는다(`DATABASE_URL` 은 사용자 PC 의 환경에만 준다).
- 인덱스 우선 적용: `chart(uniq md5)`, `score uniq(user_id, chart_sha256, played_at)`(멱등), `score idx(chart_sha256, ranked, clear, ex_score)`, `chart_best PK(chart_sha256, user_id, scratch_auto)`(웨이브 20, 0002), `chart_best idx(chart_sha256, scratch_auto, clear, ex_score)`, `score idx(client_build_sha256)`.
- 커넥션: `mysql2/promise` 풀 싱글톤(`src/server/db/index.ts`). Vercel Fluid Compute 특성상 인스턴스당 풀이 살아있으므로 `connectionLimit` 은 작게(**가정: 5**) 잡고 `enableKeepAlive: true`.

---

## 4. 인증 상세

### 4-1. better-auth

- `emailAndPassword: { enabled: true }`, 세션 쿠키(HttpOnly·Secure·SameSite=Lax). OAuth 는 후속(W2).
- `database: drizzleAdapter(db, { provider: 'mysql' })`. 핸들러는 `app/api/auth/[...all]/route.ts` 에서 `toNextJsHandler(auth)` 로 GET/POST 노출.
- `baseURL = BETTER_AUTH_URL`, 시크릿 = `BETTER_AUTH_SECRET`.
- **IR 호환 래퍼**: `POST /api/auth/register` · `/api/auth/login` 은 better-auth 를 내부 호출(`auth.api.signUpEmail` / `signInEmail`)한 뒤, 응답을 클라가 기대하는 `AuthResponse { token, player: {id}, name }`(`dto.rs:311`)로 **변환**해 raw 로 돌려준다. 동시에 세션 쿠키도 함께 세팅해 웹에서 같은 엔드포인트를 재사용할 수 있게 한다.
  - `AuthRequest { id, password, email, name? }` 에서 `id` 는 로그인 id. **가입(`accountSchema`)은 `email` 이 필수**(유효한 주소)이고 비밀번호는 8자 이상이다. 설계 당시의 `{id}@local.invalid` 합성 주소 가정은 폐기됐다(클라이언트도 이메일을 입력받는다). 로그인(`loginSchema`)은 `id`·`password` 만 쓴다.
  - 반환 `token` 은 세션 토큰이 아니라 **새로 발급한 `api_token`** 이다(네이티브 클라가 Bearer 로 재사용해야 하므로).

### 4-2. api_token (네이티브 Bearer)

- 발급: `POST /api/auth/token`(세션) 또는 `POST /api/fe/tokens`(FE UI). 응답에 **평문은 그 1회만** 반환.
- 저장: `api_token.token_hash` = SHA-256(평문). 평문·역산 가능한 형태로 저장하지 않는다. 조회는 해시 룩업(uniq 인덱스).
- 형식: `rbms_<32바이트 base64url>`. prefix 로 스캐너 탐지 용이.
- 검증(`withApiToken`): `Authorization: Bearer` 파싱 → 해시 → `api_token where token_hash and revoked=false` → `user` 조인 → `last_used_at` 갱신(쓰기 부하 완화를 위해 **60초 이내 재갱신 생략**).
- 폐기: `DELETE /api/fe/tokens/[tokenId]`(세션) → `revoked=true`. 네이티브 클라이언트의 로그아웃은 `DELETE /api/auth/token/current`(Bearer 로 보낸 토큰 하나, 204)다. 로그인·가입 요청에 이전 토큰이 Bearer 로 실려 있으면 성공한 뒤 그 토큰도 폐기한다(S19). 같은 사용자의 다른 기기 토큰은 건드리지 않으며 만료는 강제하지 않는다(사용자 결정).
- **better-auth `apiKey` 플러그인은 쓰지 않는다.** `data-model.md` 가 이미 `api_token` 을 정의했고, 클라가 기대하는 발급 응답 형태(`AuthResponse.token`)를 직접 통제해야 하기 때문. (플러그인 채택 여부는 §7 오픈 질문 아님 — 설계 확정.)

### 4-3. guest

- `ALLOW_GUEST=true` 일 때만 `POST /api/scores` 를 무자격 수용. `player.id` 는 `guest` 로 강제 고정하고 `score.user_id = null`, `ranked=false`, `flags:["GUEST"]`.
- guest 제출은 `chart_best`·랭킹에 반영하지 않는다. 감사 로그(`submission_audit`)에는 남긴다.

### 4-4. 레이트리밋

| 대상 | 한도 |
|---|---|
| `POST /api/auth/login` · `/register` | 5/min/IP |
| `POST /api/scores` · 코스 제출 | 60/min/token |
| `DELETE /api/auth/token/current` | 5/min/IP (로그인·가입과 다른 버킷) |
| `PUT /api/players/[id]/settings/[name]` | 30/min/사용자 |
| `POST /api/charts/[hash]/replays` | 10/min/token |
| `/api/fe/*` 공개 읽기, `/api/auth/get-session`·`sign-out` | 120/min/IP |
| `POST /api/fe/auth/login` · `/register` | 5/min/IP (`/api/auth/*` 와 같은 버킷) |
| 폴더 편집(만들기·고치기·지우기·순서·곡 넣고 빼기) | 60/min/사용자 |
| 폴더에 표 가져오기 | 10/min/사용자 |
| 폴더 읽기(웹·클라이언트) | 120/min/사용자 |
| 표의 내 기록 조회 | 30/min/사용자 |

- 구현: `withRateLimit(key, limit, windowMs)`. **v1 은 인스턴스 로컬 카운터(LRU)** — Vercel 멀티 인스턴스에서는 best-effort. 로그인/가입을 DB 카운터로 보강하는 계획은 **구현하지 않았다**(KNOWN ISSUE: 가입·로그인 5/분도 인스턴스 로컬이라 Vercel 다중 인스턴스에서 한도가 인스턴스 수만큼 늘고 콜드스타트마다 초기화된다). 전역 정확성이 필요해지면 Upstash Redis 등 지속형 저장소나 Vercel Firewall 규칙을 도입한다(웹 2차 후보, 사용자 결정 필요).
- IP 키(2026-10-14 웨이브 17B-2 리뷰): `getClientIp` 는 `X-Forwarded-For` 의 첫 값이다. Vercel 은 이 헤더를 덮어쓰고 바깥 IP 를 넘기지 않으므로(Vercel 문서 "Request headers", 스푸핑 방지) 운영에서는 클라이언트가 바꿀 수 없다. 다른 호스팅이나 Vercel 앞의 다른 프록시에서는 헤더를 바꿔 한도를 피할 수 있으므로(로컬에서 재현), 호스팅을 바꾸거나 지속형 한도로 옮길 때 신뢰하는 헤더를 하나로 고정한다.
- 본문 크기: `POST /api/scores` 256KB(`content-length` 헤더 + 실제 스트림 바이트 수 둘 다 검사), 리플레이 `REPLAY_MAX_BYTES`(기본 4MB) 초과 시 `413 IR_PAYLOAD_TOO_LARGE`.

### 4-5. 보안 기본

- 비밀번호는 better-auth 위임. 토큰은 해시 저장. 시크릿은 `getEnv()` 경유만.
- CORS: 같은 오리진(Next 단독 서버)이라 FE 용 CORS 불필요. 네이티브 클라는 브라우저가 아니라 CORS 무관. 따라서 **CORS 미들웨어를 두지 않는다**(api-spec §10 대비 이탈 — 단독 서버 구조의 결과). 외부 오리진 요구가 생기면 `ALLOWED_ORIGINS` 로 추가.
- 제출 검증: 토큰 user.id == body `player.id`(guest 제외), `played_at` 합리 범위(서버시각 ±7일), `chart.sha256` 형식(소문자 16진수 64자), `ex_score` == 판정에서 유도한 값, `total_notes` ≤ 서버가 아는 차트 노트 수의 2배, judge 합계 ≤ `total_notes` 일관성(`empty_poor` 제외, `total_notes=0` 인 구버전 제출은 서버가 아는 노트 수를 상한으로). 코스 제출도 같은 검사와 한도(60/min, 256KB, 빌드 해시 정책)를 받는다(api-spec §5·§6).
- **보안 헤더**(`next.config.ts` 의 `headers`, 전 경로): `X-Content-Type-Options: nosniff`, `X-Frame-Options: DENY`, `Referrer-Policy: strict-origin-when-cross-origin`, `Permissions-Policy`(카메라·마이크·위치·결제·USB 끔). **CSP 는 프로덕션 빌드에만** 붙인다(`default-src 'self'`, `script-src`·`style-src` 에 `'unsafe-inline'`(Next 의 인라인 부트스트랩과 next-themes 스크립트가 nonce 없이 쓰임), `img-src 'self' data: blob:`, `font-src 'self' data:`, `connect-src 'self'`, `object-src 'none'`, `base-uri`·`form-action 'self'`, `frame-ancestors 'none'`). 지금 쓰는 자원은 전부 같은 출처라 깨지는 것이 없음을 프로덕션 빌드로 띄워 홈·검색·로그인에서 콘솔 위반 0건으로 확인했다. 외부 이미지·분석 스크립트를 더하면 CSP 도 함께 고친다. HSTS 는 Vercel 이 붙이는 값을 그대로 쓴다(`includeSubDomains`·`preload` 는 켜지 않았다).
- **`/api/auth/[...all]`**: better-auth 의 기본 엔드포인트가 앱의 가입 검증과 레이트리밋을 우회하지 못하도록 **`GET /get-session` 과 `POST /sign-out` 만** 넘기고 나머지는 404 로 막는다(`route/auth.ts` 의 `ALLOWED_AUTH_HANDLER_ROUTES`). 가입·로그인은 앱의 `/api/auth/register`·`/login`(서버 측 `auth.api` 호출)만 쓴다. 필요한 경로가 늘면 그 배열에 더한다.
- **가입 id**: 서버가 `^[a-zA-Z0-9_-]+$`(1~64자)와 예약어 `guest`(대소문자 무시) 거부를 검사한다. 로그인 스키마는 형식을 묻지 않는다(이미 있는 계정이 들어올 수 있도록).
- **`POST /api/charts`**: 본문 64KB, 문자열은 DB 열 길이(제목류 255, `mode` 16, `url` 512), `extra` 16KB 상한. 이미 있는 차트는 **빈 필드만 채우고 값이 있는 필드를 덮어쓰지 않는다**(토큰 하나로 메타를 망가뜨릴 수 없다). `POST /api/courses/meta` 는 admin 세션만 받는다.
- **`withErrorHandling`** 은 Next 의 내부 신호(`notFound`·`redirect`·프리렌더 중단)를 `unstable_rethrow` 로 다시 던져 삼키지 않는다.
- `dangerouslySetInnerHTML` 사용처 없음(모든 텍스트는 React 자동 이스케이프). 차트 제목 등 외부 유래 문자열도 텍스트로만 렌더.

---

## 5. 캐싱 매트릭스

### 5-1. 층위

| 층 | 도구 | 적용 |
|---|---|---|
| 서버 데이터 캐시 | Next 16 **Cache Components**(`cacheComponents: true`) + `'use cache'` · `cacheLife` · `cacheTag` | `@server/service` 의 **순수 읽기 함수**에만 |
| 무효화 | `revalidateTag`(백그라운드) / `updateTag`(같은 요청 내 즉시) | 제출·업서트 Route Handler |
| 클라 서버상태 | TanStack Query v5, `staleTime` 기본 **60s** | 모든 FE 조회 |
| SSR 하이드레이션 | 서버 컴포넌트 `prefetchQuery(queryOptions)` + `HydrationBoundary` | 첫 화면에 반드시 보이는 데이터만 |

`unstable_cache` 는 쓰지 않는다(Next 16 에서 `'use cache'` 로 대체됨).

**제약**: `'use cache'` 안에서는 `cookies()`/`headers()`/`searchParams` 접근 불가. 따라서 개인화 데이터(내 설정 blob, 내 토큰 목록, `auth/me`)는 캐시하지 않고, 필요한 식별자는 **인자로 넘겨** 캐시 키에 포함시킨다(`getPlayerScores(playerId, limit)`).

### 5-2. 태그 사전

| 태그 | 내용 | cacheLife |
|---|---|---|
| `chart:{sha256}` | 차트 메타 | `days` |
| `chart-ranking:{sha256}` | 차트 랭킹 상위 N | `minutes` |
| `chart-best:{sha256}` | 차트 개인 베스트(플레이어 인자 포함) | `minutes` |
| `chart-replays:{sha256}` | 차트 리플레이 목록 | `minutes` |
| `chart-search` | 검색 결과 | `minutes` |
| `player:{id}` | 프로필 | `minutes` |
| `player-scores:{id}` · `player-recent:{id}` · `player-stats:{id}` | 플레이어 기록/통계 | `minutes` |
| `player-rivals:{id}` | 라이벌 목록 | `hours` |
| `course:{hash}` · `course-ranking:{hash}` · `course-best:{hash}` | 코스 | `minutes` / `days`(메타) |
| `tables` · `table:{id}` | IR 표 | `days` |
| `replay:{id}` | 리플레이 본문(이벤트 포함) | `max`(불변) |
| `score:{id}` | 스코어 상세 | `max`(불변) |
| `leaderboard-players` · `activity-recent` · `stats-summary` | 홈/종합 | `minutes` |
| `client-builds` | 빌드 allowlist | `hours` |

### 5-3. 무효화 매트릭스

| 변경 | 무효화 대상 |
|---|---|
| `POST /api/scores` (accepted) | `chart-ranking:{sha}` · `chart-best:{sha}` · `chart:{sha}` · `player:{userId}` · `player-scores:{userId}` · `player-recent:{userId}` · `player-stats:{userId}` · `activity-recent` · `stats-summary` · `leaderboard-players` |
| `POST /api/charts` (메타 업서트) | `chart:{sha}` · `chart-search` |
| `POST /api/charts/[hash]/replays` | `chart-replays:{sha}` · (연결 시) `score:{scoreId}` |
| `PUT /api/players/[id]/rivals` | `player-rivals:{id}` · `player:{id}` |
| `POST /api/courses` (결과 제출) | `course-ranking:{hash}` · `course-best:{hash}` · `player-*:{userId}` · `activity-recent` |
| `POST /api/courses/meta` | `course:{hash}` |
| `POST /api/tables` | `tables` · `table:{id}` |
| `POST /api/admin/builds` | `client-builds` |
| `PUT /api/players/[id]/settings/[name]` | 없음(비캐시) |

- 제출 응답이 곧바로 새 순위를 보여줘야 하는 FE 경로(리더보드 재진입)에서는 `revalidateTag` 대신 **`updateTag`** 를 쓴다. 네이티브 제출(rbms-player)은 응답 직후 재조회하지 않으므로 `revalidateTag` 로 충분하다.
- TanStack 측 무효화는 mutation `onSuccess` 에서 `QUERY_KEY` 로 `invalidateQueries`. 서버 태그와 **이중으로** 건다(서버 캐시 ≠ 클라 캐시).

### 5-4. 페이지별 prefetch

| 페이지 | 서버 prefetch(queryOptions) | 동적(Suspense) |
|---|---|---|
| `/` | `statsSummaryQueryOptions()` · `activityRecentQueryOptions({limit:20})` | — |
| `/search` | `chartSearchQueryOptions(params)` | — |
| `/charts/[hash]` | `chartQueryOptions(hash)` · `chartRankingQueryOptions(hash, {limit:50})` | 내 기록(`chartBest`, 세션 의존) |
| `/players/[id]` | `playerQueryOptions(id)` · `playerStatsQueryOptions(id)` · `playerRecentQueryOptions(id)` | — |
| `/leaderboards` | `playerLeaderboardQueryOptions(params)` | — |
| `/tables`·`/tables/[id]` | `tablesQueryOptions()` / `tableQueryOptions(id)` | — |
| `/courses/[hash]` | `courseQueryOptions(hash)` · `courseRankingQueryOptions(hash)` | — |
| `/replays/[id]` | `replayQueryOptions(id)` | — |
| `/settings` | 없음(전부 세션 의존) | 전 영역 |

- **서버 prefetch 는 서비스 계층을 직접 읽는다**(S19, 이전 KNOWN ISSUE "자기 공개 URL 로 HTTP fetch" 해소). 위젯의 서버 컴포넌트와 `generateMetadata` 는 `entities` 의 `queryOptions` 에서 키만 가져오고 `queryFn` 은 `@server/service/public-reads.ts` 의 `read*` 로 바꿔 끼운다(`{ ...xQueryOptions(…), queryFn: () => readX(…) }`). `read*` 는 `read-cache.ts` 의 `*Cached` 를 부르고 FE·IR 라우트(`route/fe.ts`, `route/player.ts`, `route/course.ts`, `route/replay.ts`, `route/table.ts`)도 같은 함수를 쓰므로 SSR 과 HTTP 응답의 모양이 갈라지지 않는다. 그래서 SSR 이 `/api/fe/*` 의 IP 당 120/분 한도를 같이 쓰지 않고 공개 도메인 왕복도 없다. 쿼리 문자열이 거치던 검증은 `parseQueryParams(schema, params)` 가 같은 Zod 스키마로 다시 한다. `shared/lib/fetch.ts` 의 `buildApiUrl` 은 이제 origin 을 붙이지 않는다(브라우저 전용). 서버에서 실수로 `fetch` 하면 URL 오류로 드러나며, `tests/lib/ssr-no-self-fetch.test.ts` 가 서버 프리페치 호출마다 `queryFn` 이 바뀌는지 훑는다. 세션이 필요한 설정·관리자 화면도 같은 방식이다(쿠키 전달 헬퍼 `forwardedCookieInit` 은 없앴다). 요청 시점 데이터(`/`, `/tables`)는 `await connection()` 으로 빌드 중 실행을 막는다(아래 "빌드 시 DB 미접속 보장").
- **없는 상세 페이지의 404 상태**(S19, 이전 KNOWN ISSUE "soft 404" 해소): Cache Components 에서는 `Suspense` 안의 `notFound()` 가 이미 200 을 보낸 뒤라 상태를 못 바꾼다(Next 문서 loading.md "Status Codes"). 그래서 `src/proxy.ts` 가 `/charts/[hash]`·`/players/[id]`·`/courses/[hash]`·`/replays/[id]` 에서 존재를 먼저 확인한다(`server/service/detail-existence.ts`: 차트는 해시 모양과 `findByHash`, 플레이어는 로그인 id, 코스는 `courseService.exists`, 리플레이는 `replayService.exists` — 리플레이 본문은 읽지 않는다). 없으면 `NextResponse.rewrite` 로 존재하지 않는 경로를 보여 앱의 `not-found` 화면을 진짜 404 로 낸다. 조회가 실패하면(DB 끊김 등) 판단하지 않고 통과시켜 멀쩡한 페이지를 404 로 만들지 않는다. 같은 조회는 페이지 안의 `fetchQueryOrNotFound` 가 한 번 더 한다(경합 대비). 표 상세(`/tables/[id]`)는 번들 표라 이번 범위에 넣지 않았다.
- **차트 제목 보강(결정 M8)**: 제출이 만든 차트 행의 제목·아티스트가 비어 있어도, 그 `md5` 가 번들된 표에 있으면 **읽을 때** 표의 제목·아티스트로 채워 응답한다(DB 는 고치지 않는다 — 운영 DB 작업이 필요 없고 배포만으로 적용되며 표가 갱신되면 함께 바뀐다). 적용 범위: `GET /api/charts/[hash]`, FE 차트 리더보드의 `chart`, 차트 검색 결과, 홈·플레이어의 활동 항목. 차트 검색은 표 제목에 맞는 `md5`(최대 200개)를 `OR chart.md5 IN (…)` 으로 함께 묻는다. 레벨은 표의 레벨과 BMS `#PLAYLEVEL` 의 뜻이 달라 채우지 않는다. 앞으로는 클라이언트가 `POST /api/scores` 의 `chart.title`·`chart.artist`·`chart.level`(모두 선택, 긴 값은 자르고 이상한 값은 비움 — 제출을 막지 않는다)로 메타를 함께 올리면 공란인 행만 채운다. `read-cache` 의 `use cache` 항목은 `days` 단위라 배포 직후 옛 값이 남을 수 있다(태그 `chart:{hash}` 무효화나 만료로 갱신).
- 서버에서는 **요청마다 `new QueryClient()`**. 클라이언트는 싱글톤(`get-query-client.ts`), `gcTime >= staleTime`(기본 `staleTime 60s`, `gcTime 10m`).
- **빌드 시 DB 미접속 보장**: `cacheComponents` 가 켜지면 `'use cache'` 함수는 프리렌더 중 실행될 수 있다. 따라서 **DB 를 읽는 모든 페이지 영역은 `<Suspense>` 로 감싼다**(정적 셸만 프리렌더). `generateStaticParams` 를 쓰지 않고, `next build` 는 셸만 생성한다. 이 규칙은 §8 검증 게이트에서 확인한다.

---

## 6. 환경변수 · 배포

### 6-1. `getEnv()` (zod, `@server/lib/env.ts`)

| 키 | 필수 | 설명 |
|---|---|---|
| `DATABASE_URL` | 조건부 | `mysql://user:pass@host:port/db`. 없으면 아래 5개 필수 |
| `DB_HOST` `DB_PORT` `DB_USER` `DB_PASSWORD` `DB_NAME` | 조건부 | `DATABASE_URL` 미제공 시 조합해 URL 생성. **코드(`src/server/lib/env.ts`)가 정본** — 2026-09-09 결정 "env 키" 항목 |
| `BETTER_AUTH_SECRET` | 운영 필수 | 세션 서명. 스키마는 `.optional()` 이지만 `NODE_ENV=production` 에서 인증을 처음 쓸 때(`getAuth()`) `assertProductionAuthEnv()` 가 비었으면 던진다(S19) |
| `BETTER_AUTH_URL` | 운영 필수(스키마 기본 `http://localhost:3000`) | `https://bms.hyuns.uk`. production 에서는 생략하면 위와 같이 던진다 |
| `NEXT_PUBLIC_APP_URL` | 운영 필수(스키마 기본 `http://localhost:3000`) | `https://bms.hyuns.uk` (클라 노출 가능한 값만). production 에서는 생략하면 위와 같이 던진다. 서버 코드는 더 이상 이 값으로 자기 주소를 만들지 않는다 |
| `ALLOW_GUEST` | X(기본 `false`) | guest 제출 허용 |
| `REPLAY_MAX_BYTES` | X(기본 `4194304`) | 리플레이 상한 |
| `REQUIRE_BUILD_HASH` | X(기본 `false`) | `client_build_sha256` 누락 시 거부 여부 |
| `REPLAY_STORAGE` | X(기본 `db`) | 환경값은 `db` \| `blob`을 파싱하지만 구현은 **`db`만** 지원 |
| `BLOB_READ_WRITE_TOKEN` | 미사용 | Blob 백엔드가 구현되기 전까지 사용하지 않음 |
| `SERVER_VERSION` `SERVER_COMMIT` | X | `/api/version` 표기(Vercel `VERCEL_GIT_COMMIT_SHA` fallback) |
| `NODE_ENV` | 자동 | — |

- zod `safeParse`. `DATABASE_URL` 또는 `DB_HOST`/`DB_USER`/`DB_NAME` 조합은 `resolveDatabaseUrl()` 이 판정하고, 실제 접속이 필요한 시점(`getRequiredDatabaseUrl()`)에만 throw 한다 — 빌드 시 DB 미접속 보장(§5-4)을 위해 모듈 로드 시점에 던지지 않는다.
- **빈 문자열(`KEY=`)은 미설정으로 취급**한다(`.env` 스캐폴드가 빈 값을 담고 있어도 기본값이 적용된다).
- `process.env` 직접 접근은 `env.ts` 안으로 한정. `NEXT_PUBLIC_*` 에는 시크릿을 넣지 않는다.
- **환경 변수가 전혀 없어도 `bun run build` 가 성공한다**(CI 의 웹 잡이 그렇게 돈다). 요청 시점 API 를 쓰는 라우트가 빌드 중에도 실행되므로 `connection()`(또는 `request` 접근)이 `compose()` 보다 앞서야 한다(`/api/auth/me`, `/api/fe/me/settings` 에 적용). `withErrorHandling` 이 프리렌더 중단 신호를 다시 던지므로 빌드 로그에 오류 블록이 남지 않는다. 새 세션 라우트를 만들 때도 같은 순서를 지킨다.
- `NEXT_PUBLIC_APP_URL` 은 **빌드 시점에 번들(서버와 클라이언트 모두)에 박힌다**. S19 이전에는 서버 컴포넌트의 prefetch 가 이 값으로 자기 공개 주소에 HTTP 로 다시 요청했지만(Preview 에서 운영 데이터를 읽고 `/api/fe/*` 의 IP 당 120/분 제한을 SSR 이 공유), 지금은 서비스 계층을 직접 읽는다(5-4). 이 값은 브라우저의 `authClient` baseURL 에만 쓰인다.
- 운영 필수 변수 검사는 `env.ts` 의 `findMissingProductionAuthEnv`/`assertProductionAuthEnv` 다. `parseEnv` 에서 던지면 환경 변수 없는 `next build` 가 깨지므로(`tests/env.test.ts` 가 `parseEnv({ NODE_ENV: 'production' })` 의 성공을 고정한다) 인증을 처음 쓰는 시점에 검사한다. 누락한 키를 한 번에 모두 알린다.
- `REPLAY_STORAGE=blob`은 compose가 즉시 실패시켜 잘못된 운영 구성을 드러냅니다. Vercel Blob 구현과 토큰 검증은 후속입니다.

### 6-2. Vercel

- 런타임 **Node.js**(mysql2 필요, Edge 불가). Cache Components 도 Edge 미지원.
- 리전 **`icn1`**(서울) — DB 위치가 결정되면 그 리전에 맞춘다(**가정**).
- 도메인 `bms.hyuns.uk`. 클라 안내 문구는 `--server https://bms.hyuns.uk/api`.
- 빌드 명령 `bun run build`, 설치 `bun install`. `drizzle-kit migrate` 는 **빌드에 포함하지 않는다**(빌드 시 DB 미접속 원칙). 마이그레이션은 별도 수동/CI 스텝(3-2).
- `next.config.ts`는 `reactCompiler: true`, `cacheComponents: true`, `serverExternalPackages: ['mysql2']`, 보안 헤더를 설정했습니다.
- **응답 크기 한도**: Vercel Functions 의 요청·응답 본문은 최대 4.5MB(`FUNCTION_PAYLOAD_TOO_LARGE`, 공식 문서 `functions/limitations` 2026-08-24 판). 폴더 하나(2,000곡)의 최악 응답은 제목·아티스트·메모가 전부 4바이트 문자로 꽉 찬 경우 약 3.9MB 로 한도 안이지만 여유가 작다. 실제 폴더는 곡당 약 140바이트(약 280KB)다. 표 상세는 약 380KB, 표준 표 `data.json` 은 약 400KB.

#### 배포 설정(결정 U5·M10, 근거 [../acknowledge/2026-10-14-adr-vercel-git-deploy.md](../acknowledge/2026-10-14-adr-vercel-git-deploy.md)) — 2026-10-14 적용 완료

> 확인한 것: `curl https://bms.hyuns.uk/api/version` 이 `{"api_version":1,"server":"0.1.0","commit":"7095ef7"}` 를 돌려줘(2026-10-14 관찰) Git 연동 배포이고 `dev` 의 최신 커밋이 운영에 올라 있다. 응답 헤더에 §4-5 의 보안 헤더와 CSP 가 모두 있다. 확인하지 못한 것: Vercel 대시보드 값(프로젝트 `rbms-web`(팀 b-hs-projects), Root `web`, Production `dev`, 환경 변수 6개 중 `DATABASE_URL`·`BETTER_AUTH_SECRET` 은 Secret)은 세션 인벤토리의 기록이며 이 문서 점검에서 Vercel 에 접근하지 않았다. 변경 없는 푸시가 `CANCELED` 로 기록되는지(`ignoreCommand`)도 대시보드에서만 본다.

| 항목 | 값 |
|---|---|
| Git 연결 | `B-HS/R-BMS` |
| Root Directory | `web` |
| Production Branch | `dev` |
| Preview | 만들지 않는다. `web/vercel.json` 의 `ignoreCommand` 가 프로덕션이 아닌 빌드를 건너뛴다 |
| 변경 없는 푸시 | 같은 `ignoreCommand` 가 `web/` 이 바뀌지 않은 푸시(Rust·docs)를 건너뛴다 |
| 시스템 환경 변수 | "Enable access to System Environment Variables" 를 켠다(`VERCEL_GIT_COMMIT_SHA` 가 `/api/version` 의 `commit` 을 채운다) |

`ignoreCommand`(공식 문서: 종료 코드 0 = 빌드 건너뜀, 1 이상 = 빌드, `VERCEL_ENV` 는 빌드·런타임에 있고 `VERCEL_GIT_PREVIOUS_SHA` 는 Ignored Build Step 이 지정된 경우에만 빌드 시점에 노출되며 브랜치의 첫 배포에서는 빈 값, 명령은 Root Directory 에서 실행):

```
if [ "$VERCEL_ENV" != "production" ]; then exit 0; fi; git diff --quiet "$VERCEL_GIT_PREVIOUS_SHA" HEAD -- . || exit 1
```

로컬에서 `sh -c` 로 확인한 값: `VERCEL_ENV=preview` 0(건너뜀), 프로덕션이고 마지막 성공 SHA 가 `HEAD` 면 0, `web/` 이 바뀐 구간 1, 빈 SHA·없는 SHA(첫 배포, 얕은 클론 밖)는 `git diff` 가 128 을 내 `|| exit 1` 로 1(빌드). Vercel 위에서의 실제 동작(환경 변수 노출 시점)은 푸시로만 확인되므로 **확인하지 못함**. 건너뛴 빌드는 `CANCELED` 배포로 기록되고 빌드 슬롯은 쓴다. 내장 "Skip deployment" 토글은 JS workspace 가 있는 모노레포용이라 이 저장소(Cargo)에서는 쓸 수 없다.

#### 환경 변수(값은 Vercel 에서만 입력. 저장소에 두지 않는다)

| 키 | Production | 비고 |
|---|---|---|
| `DATABASE_URL`(또는 `DB_HOST` `DB_PORT` `DB_USER` `DB_PASSWORD` `DB_NAME`) | 필수 | 둘 중 하나 |
| `BETTER_AUTH_SECRET` | 필수 | 32바이트 이상 무작위. 비우면 안 된다(코드상 선택이라 조용히 잘못 동작한다) |
| `BETTER_AUTH_URL` | `https://bms.hyuns.uk` | 생략하면 `http://localhost:3000` |
| `NEXT_PUBLIC_APP_URL` | `https://bms.hyuns.uk` | **빌드 시점에 박힌다**(6-1). 바꾸면 재배포 |
| `ALLOW_GUEST` | `false`(결정 M11, 기본값) | 비로그인 판은 클라이언트에 로컬 저장 |
| `REPLAY_MAX_BYTES` `REQUIRE_BUILD_HASH` `REPLAY_STORAGE` | 선택 | `REPLAY_STORAGE=blob` 은 미구현 |
| `SERVER_VERSION` `SERVER_COMMIT` | 선택 | 비우면 `VERCEL_GIT_COMMIT_SHA` 앞 7자 |

#### 최초 배포 절차(2026-10-14 에 이 순서로 끝냈다. 새 환경을 만들 때의 기록)

1. Vercel 프로젝트에 위 연결·Root Directory·Production Branch·시스템 환경 변수를 확인하고 환경 변수를 입력한다. 이미 CLI 업로드로 배포한 프로젝트라면 Git 연결부터 한다(`/api/version` 의 `commit` 이 `null` 이면 Git 연동 배포가 아니다). `vercel link` 는 `web/.env.local` 을 자동 생성할 수 있으니(2026-10-14 에 한 번 생김, 사용자가 지움) 링크 뒤 확인하고 커밋하지 않는다(`.gitignore` 로 무시됨).
2. 운영 DB 의 상태를 본다: `gh workflow run web-db.yml --ref dev -f action=status`(3-2).
3. 상태가 "기준선 필요" 면 `--baseline-sql` 의 SQL 을 한 번 적용한다.
4. `-f action=migrate` 로 `0001`·`0002` 를 적용하고, 상태가 "최신" 인지 본다.
5. 시험 데이터 정리(결정 Q6=A, 완료): `-f action=cleanup-count` → 숫자 확인 → `-f action=cleanup-delete`(세기 → 한 트랜잭션으로 삭제 → 다시 세기, 0 확인). 대상은 점검 문서 3-2 의 시험 계정 다섯 개와 그 제출·토큰·세션·폴더. 0001 이 만든 `user_folder` 를 세므로 마이그레이션 뒤에 실행한다.
6. 그 다음 `dev` 에 푸시해 배포한다. `curl https://bms.hyuns.uk/api/version` 의 `commit` 과 `curl -I https://bms.hyuns.uk/tables/insane1/header.json` 으로 확인한다.

CI(`.github/workflows/web.yml`)는 `web/**` 이 바뀐 푸시·PR 에서 설치(`--frozen-lockfile`)·typecheck·lint·test·format:check·환경 변수 없는 build 를 돈다(`docs/ci-release.md` 6절).

---

## 7. 무결성 (FR-13 / FR-14 / FR-18) 처리 위치

| 요구 | 위치 |
|---|---|
| FR-13 빌드 sha256 allowlist | `@server/service/shared/integrity/build-allowlist.ts` — `client_build` 조회(`'use cache'` + `cacheTag('client-builds')`). CI 등록은 `POST /api/admin/builds` |
| FR-14 ranked 판정 | `@server/service/domain/score/ranked-policy.ts` — 순수 함수 `rankedPolicy({ options, buildTrust, isGuest, requireTrustedBuild }) => { ranked, flags }`. 입력만으로 결정되므로 DB 무관, 단위 테스트 대상 |
| 빌드 신뢰도와 ranked | `UNKNOWN_BUILD` flag 는 항상 남기되, ranked 를 막는 것은 ① `untrusted`(allowlist 에 `trusted=false` 로 등록) ② `unknown` + `REQUIRE_BUILD_HASH=true` 두 경우다. 기본 배포(`REQUIRE_BUILD_HASH=false`)에서 미등록 빌드는 flag 만 남고 ranked 로 집계된다 — allowlist 등록 경로(`POST /api/admin/builds`)가 W2a 이므로, 그 전까지 랭킹이 통째로 비는 것을 막기 위함 |
| flags 종류 | `AUTOPLAY` `SCRATCH_AUTO` `ASSIST`(`AUTO_SCRATCH` 밖의 assist 태그) `JUDGE_WIDTH`(judge_rate>100) `TOTAL_OVERRIDE` `UNKNOWN_BUILD` `GUEST`(`ranked-policy.ts` 의 `SCORE_FLAG`) `REPLAY_MISMATCH`(후속, 코드에 없음). `SCRATCH_AUTO` 와 `UNKNOWN_BUILD`(기본 설정) 는 표지일 뿐 ranked 를 막지 않는다(`rankBlockingFlags`) |
| SCRATCH AUTO(웨이브 20, 0b04629) | 사용자 결정(2026-10-14, 근거 [../acknowledge/2026-10-14-adr-scratch-auto-rule.md](../acknowledge/2026-10-14-adr-scratch-auto-rule.md)·[../acknowledge/2026-10-14-adr-scratch-auto-ranking-toggle.md](../acknowledge/2026-10-14-adr-scratch-auto-ranking-toggle.md)): SCRATCH AUTO 판도 일반 판처럼 순위에 올리되 목록에서는 기본으로 숨기고 토글로 보인다. `chart_best`·`course_best` 를 `scratch_auto` 로 나눠 일반 판 최고와 SCRATCH AUTO 판 최고를 따로 남기고, 목록 질의는 `best-scope.ts` 의 `chartBestScope`·`courseBestScope` 로 "일반 판만" 또는 "플레이어마다 더 높은 쪽"을 고른다. 차트 순위·코스 순위·리더보드 화면에는 "SCRATCH AUTO 포함" 스위치가 있고 상태는 주소의 `include_scratch_auto=true` 로 남는다(쿼리 키에 포함). 목록의 SCRATCH AUTO 행에는 A-SCR 배지. SCRATCH AUTO 판의 `rank` 는 그 판이 속한 목록(일반 판은 기본 목록, SCRATCH AUTO 판은 포함 목록) 기준이고 `previous_best`·`is_new_best` 는 그 판의 칸 기준이다(`score.service.ts` `submit`). 넓힌 판정(`options.assist` 의 `CUSTOM_JUDGE`)은 `ASSIST` flag 로 그대로 unranked |
| 클라 정책 정합 | 옛 결정 #12("어시스트 판은 최고 기록이 아니다")는 2026-10-14 사용자 결정으로 **대체**됐다: 손으로 친 모든 판은 클라이언트가 저장·전송하고, SCRATCH AUTO 는 서버에서도 ranked 다. 넓힌 판정(클라이언트 어시스트 태그 `CUSTOM_JUDGE`)은 서버에서 `ASSIST` flag + unranked 이고 `chart_best`·순위에 들어가지 않는다(사용자 미확인 정책). SCRATCH AUTO 램프는 `LightAssistEasy` 로 유지(메인이 정함, 사용자 미확인). 서버는 클라이언트의 판단을 신뢰하지 않고 독립 재판정한다 |
| FR-18 감사 로그 | `score.service.ts` 의 `submit` 이 `deps.db.insertAudit`(compose 구현)을 불러 `submission_audit` 에 insert 한다. IP 는 `x-forwarded-for` 첫 값, UA 는 `user-agent`. 별도 `audit.ts` 파일은 없다 |
| best 갱신 | `chart_best` upsert(키 `(chart, user, scratch_auto)`). 비교 순서 **램프 > EX > BP(낮을수록)**. `ranked=false` 는 갱신 대상 제외 |
| 멱등 | 서비스가 같은 사용자·같은 차트에서 `played_at` 이 **같은 초**(`floor(played_at / 1000)` 이 같음)인 점수를 먼저 찾아(`findScoreIdByIdempotency`) 있으면 기존 `score_id` 를 돌려준다(201 대신 200, `message: "duplicate submission ignored"`). `score uniq(user_id, chart_sha256, played_at)` 색인은 정확히 같은 밀리초의 이중 삽입만 막는다. 기록 동기화가 기록 DB 의 초 단위 시각으로 다시 보낸 판을 중복 저장하지 않으려는 규칙이다 |

---

## 8. 테스트 전략

| 층 | 러너 | 대상 |
|---|---|---|
| dto | `bun test` | zod 라운드트립. **`crates/rbms-ir/src/dto.rs` 가 실제로 보내는 JSON 픽스처**를 그대로 파싱 성공하는지(계약 회귀 방지) |
| lib | `bun test` | `createAppError`/STATUS_MAP, `api-response` 헬퍼, `getEnv` 검증 실패 케이스, 토큰 해시 |
| service | `bun test` | `*ServiceDb` 를 인라인 객체로 대체(mocking 라이브러리 없음). best 갱신 규칙·멱등·EX/BP 유도·낙관적 잠금 충돌·`rankedPolicy` 전 flags |
| Route Handler | `bun test` | 핸들러를 직접 import 해 **`new Request(url, {...})`** 로 호출하고 `Response` 를 검증. 인증 401/403, 404, 413, 429, envelope 유무, raw 응답 형태 |
| 컴포넌트 | `bun test` + Testing Library + happy-dom | features 순수 UI 렌더·상호작용(`tests/components/`). 서버 상태는 목을 쓰지 않고 순수 함수·계약 테스트로 확인한다. `msw` 는 devDependency 로 설치만 돼 있고 쓰는 테스트가 없다(KNOWN ISSUE: 쓰지 않으면 `bun remove msw`) |
| DB 질의 | `bun test` | `tests/db/scratch-auto-ranking.test.ts`(순위 질의 `best-scope.ts`), `tests/support/folder-memory-db.ts`(폴더 서비스용 메모리 DB) |
| E2E | 후속(합의 필요) | Playwright. W0 범위 아님 |

- 테스트 설명은 한국어. `describe`/`test`.
- **계약 왕복 검증(수동, PRD §8)**: `rbms-player --server http://localhost:3000/api --player <id>` 로 submit → ranking 왕복. W1 완료 게이트.
- 최소 기계 검증: `cd web && bun run verify`(= `next typegen && tsc --noEmit` → `eslint` → `bun test`) 와 `bun run format:check`, 필요하면 환경 변수 없는 `bun run build`. 테스트 수는 시점마다 달라 `bun test` 출력이 정본이다(2026-10-14 점검 208 통과·2 건너뜀, 17B-2 종료 395 통과·2 건너뜀).

---

## 9. 현재 스캐폴드(`web/`)와의 정합 — W1a 에서 해소 완료 (2026-09-09)

`web/` 는 병렬 작업으로 부트스트랩되어 있었고, 이 설계와 **일치**하는 것: alias 7종(`@/*` 추가), FSD + `src/server/{route,service,dto,db,lib,compose}` 계층, `(app)`/`(public)` 라우트 그룹, `api/{auth,health,version}` 골격, 버전 세트(next 16.3.4 · react 19.2.8 · better-auth 1.7.3 · drizzle 0.45.2 · zod 4.5.4 · TanStack Query 5.102.8).

**불일치 3건 — W1a 착수 시 해소했다.**

| # | 이전 상태 | 조치(완료) |
|---|---|---|
| 1 | `next.config.ts` 에 `cacheComponents` 없음 | `cacheComponents: true` + `serverExternalPackages: ['mysql2']` 추가. 기존 페이지는 전부 정적 셸이라 프리렌더 통과 |
| 2 | `package.json` 에 `db:push` 스크립트 존재 | 제거. 대신 `auth:generate` 추가(`bunx @better-auth/cli generate`) |
| 3 | `cn@0.2.6` 패키지 설치됨 | `bun remove cn`. 코드베이스에 `from 'cn'` import 는 0건이었다(자체 `@shared/lib/cn` 은 W1b 소유) |

추가로 W1a 에서 생긴 정합 사항:

- `server-only` 패키지를 설치해 `@server/db`·`@server/compose`·`@server/service/read-cache` 상단에 `import 'server-only'` 를 붙였다. `bun test` 는 client 조건으로 해석하므로 `bunfig.toml` 의 `[test] preload` 로 `tests/lib/preload.ts`(`mock.module('server-only', …)`)를 로드한다.
- `revalidateTag` 는 Next 16.3 에서 **2인자**(`revalidateTag(tag, profile)`)다. `@server/service/read-cache.ts` 의 `CACHE_PROFILE` 이 태그별 프로필을 고정한다.
- `GET /api/health` · `GET /api/version` 은 2026-09-09 에는 정적 프리렌더(○)였으나 지금은 `await connection()` 으로 **동적**이다(`server/route/system.ts`). `/api/version` 의 `commit` 이 배포 시점 환경 변수(`VERCEL_GIT_COMMIT_SHA`)에 따라 달라지기 때문이다. `connection()` 은 Next 요청 컨텍스트 밖(`bun test` 에서 `new Request()` 로 직접 호출)에서는 실패하므로 두 라우트의 테스트는 서비스 함수(`getServerInfo`·`getVersionInfo`, `tests/service/system-info.test.ts`)를 대상으로 한다. 빌드 로그의 프리렌더 중단 오류 블록은 `withErrorHandling` 의 `unstable_rethrow` 로 사라졌다.
- `user` PK 는 better-auth 생성 id, 로그인 id 는 `user.login_id`(uniq) 이며 `password_hash` 컬럼은 두지 않는다(OQ1 결정 반영). `/api/players/{id}` 는 `login_id` 로 해석한다.

## 10. 가정 목록 (사용자 회신 시 변경)

1. `web/` 라이선스 = 레포 GPL-3.0(W8). contract-freeze 의 "별 MIT 레포" 전제는 W1 결정으로 폐기됨.
2. better-auth `user.id`는 생성 id이고 로그인 id는 `user.login_id`다.
3. (폐기) email 미제공 계정의 `{id}@local.invalid` 합성 주소는 쓰지 않는다. 가입은 email 필수.
4. Vercel 리전 `icn1`, mysql2 풀 `connectionLimit: 5`.
5. 리플레이 저장은 `REPLAY_STORAGE=db`와 `replay.data`(mediumtext)만 구현됐다. Vercel Blob 전환은 후속이다.
6. 레이트리밋 v1 = 인스턴스 로컬(best-effort).
7. `POST /api/courses` = 코스 **결과 제출**(클라 계약 우선), 메타 업서트는 `/api/courses/meta`.
8. (정정) `(app)` 인증 게이팅은 `proxy.ts`(쿠키 존재 확인)와 세그먼트 layout 의 `SessionGate`(서버 세션 확인) 두 겹으로 수행한다(§1-1).
