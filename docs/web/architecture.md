# rbms web — 아키텍처 설계 (W0)

> 최종 갱신 2026-10-24 · 대응 커밋 66b9e63 + 어시스트 단위 작업(미커밋) (2026-10-24 에 어시스트 클리어를 A-단위로 따로 기록하도록 §15-3 을 고치고 §3-2 에 마이그레이션 0013 을 더했다(운영 미적용). §1 의 단위 API 두 줄과 프로필·순위의 `dans` 모양도 바뀌었다. 같은 날 새 §16 다국어(next-intl, 쿠키 방식, 전 페이지 동적 렌더, 기반까지)와 §0·§5-1 의 한 줄, §5-2 의 순위 태그 설명을 더했다. 2026-10-23 에 §1 의 화면·경로 표, §3-2 의 마이그레이션 0012 와 `web-db` 동작, §5 의 캐시 태그, §6 의 `CRON_SECRET` 과 Cron 을 더하고 새 §15 를 썼다. 2026-10-22 에 §3-2 마이그레이션 목록에 0011 을 더했다. 나머지 절은 2026-10-20 작업 트리 기준: W3 곡별 패턴 §1-2·§1-4·§3·§4-4·§11, W2 점수 기반 AI 기능 §12, W4 판별 trace §13, W5 판별 기반 AI 기능 §14)

> **현재 구현 기준:** W0~W2 와 웨이브 17B(웹 골격·폴더·발광 표)·20(SCRATCH AUTO 순위), S18~S19(2026-10-14~15, 서버 수리·발광 표의 다른 사이트 흔적 제거·파비콘)까지 반영했습니다. 실제 라우트는 `web/src/app/{(app),(public),api,tables}/`, 서버 경계는 `web/src/server/`가 정본입니다. 본문에 남은 "착수 시", "향후", 가정은 설계 이력으로만 읽고, 이 상태 블록과 §9를 우선합니다. **운영 배포는 2026-10-14 에 끝났습니다**(https://bms.hyuns.uk, Vercel 프로젝트 `rbms-web`, 시험 계정 삭제). 운영 DB 에는 마이그레이션 0000~0012 가 모두 적용됐고(0003·0004 는 2026-10-15, 0011 은 2026-10-22, 0012 는 2026-10-23. 0013 `assisted_dan` 은 파일만 커밋됐고 운영에는 아직 적용하지 않았다: 아래 §3-2), 2026-10-15 관찰 시 운영 웹은 `51f3a1e` 입니다(`/api/version`). 배포 설정은 §6-2, 운영 DB 작업은 §3-2 가 정본입니다. 2026-10-14 점검에서 나온 고장 12건의 해소 현황은 [2026-10-14-audit.md](./2026-10-14-audit.md) 맨 위 표입니다.

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
| 언어(2026-10-24) | next-intl 4.14.9, 쿠키 방식(경로 접두사 없음), 기본 한국어, 메시지 `web/messages/{ko,en,ja}/*.json`(§16) |

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
| `(app)` | `/players/[id]` | 플레이어 프로필(통계·플레이 기록(전체 판, `?page=` 페이지)·라이벌) | A | 공개 읽기 |
| `(app)` | `/leaderboards` | 종합 플레이어 랭킹 | A | 공개 읽기 |
| `(app)` | `/tables` · `/tables/[id]` | 번들 표 카드 + IR 난이도표 | A | 공개 읽기 |
| `(app)` | `/tables/insane1` | 発狂BMS難易度表 사본(카드 격자, 내 램프, `bmstable` 메타 = `/tables/insane1/header.json`, noindex). 2026-10-14 웨이브 17B-2(711b518). 이 페이지의 주소 `https://bms.hyuns.uk/tables/insane1` 가 R-BMS 의 기본 등록 표 주소다 | A | 공개 읽기(로그인 시 내 기록) |
| `(app)` | `/tables/stella-st` · `/tables/stella-sl` | Stella·Satellite 표 사본(DB 에 저장된 사본을 읽는다. insane1 과 같은 카드 격자·내 램프·`bmstable` 메타 = `/tables/stella-st/header.json`, noindex). 위쪽에 **출처 표기**: 표 이름, 제작 주체(`stellabms.xyz`), 원본 표 페이지 링크(새 탭), 내용을 받은 날(KST), 마지막 확인, 데이터 SHA-256 앞 12자. 레벨 기호는 헤더의 `st`·`sl` 이라 `st0`~`st12`. 2026-10-23 (§15-4). 아직 한 번도 받지 못했으면 목록에 없다 | A | 공개 읽기(로그인 시 내 기록) |
| `(app)` | `/folders` · `/folders/[folderId]` | 내 폴더 목록과 명시 저장 편집. 2026-10-14 웨이브 17B-2 | A | **세션 필수**(proxy + `SessionGate`) |
| `(app)` | `/courses` | 코스 목록(2026-10-23): 묶음(GENOSIDE 2018 SP·DP, Stella, Satellite)별로 출처 블록과 코스 줄(이름 → 상세, 스테이지 수, 단위 배지, 표에서 빠졌으면 "표에서 빠짐"). 내비게이션 "코스" | A | 공개 읽기 |
| `(app)` | `/courses/[hash]` | 코스 랭킹(2026-10-23 부터 위쪽에 출처 블록과 "이 코스는 … 단위를 줍니다" 한 줄, 차트 제목 열, 차트 링크는 `sha256 \|\| md5`) | A | 공개 읽기 |
| `(app)` | `/replays/[id]` | 리플레이 뷰어(µs 이벤트 타임라인) | A | 공개 읽기 |
| `(app)` | `/settings` | 계정 설정(탭: 프로필 / API 토큰 / 동기화 blob / 라이벌 / AI / 단위(`?tab=dans`, 2026-10-23: 체계별 획득 단위와 남에게 보일 단위 선택. 2026-10-24: 어시스트로 얻은 A-단위도 목록과 선택에 함께)) | A | **세션 필수** |
| `(app)` | `/admin/builds` | 빌드 allowlist(FR-13) | A | **세션 + role=admin** |

- `(app)` 루트의 미인증 처리: 공개 읽기 라우트는 통과하고 `/settings`·`/admin/*`·`/folders/*` 만 보호한다. 두 겹이다. ① `web/src/proxy.ts`(Next 16 의 proxy, matcher `/settings/:path*`·`/admin/:path*`·`/folders/:path*`)가 better-auth 세션 쿠키가 없으면 `/login?next=` 로 진짜 리다이렉트한다(세션 확인은 DB 접속 없이 쿠키 존재만 본다. S19 부터 같은 proxy 가 `/charts/[hash]`·`/players/[id]`·`/courses/[hash]`·`/replays/[id]` 의 존재도 DB 로 확인해 없으면 404 로 재작성한다. §5-4). ② 각 세그먼트 `layout.tsx` 의 `SessionGate`(`widgets/auth/session-gate.tsx`)가 서버에서 세션을 다시 읽어 없으면 `redirect`, `/admin` 은 `role=admin` 이 아니면 `/` 로 보낸다(쿠키가 있어도 만료·위조면 여기서 막힌다). 2026-10-14 점검 당시의 "200 + meta refresh" 는 d38b6fb 에서 해소됐다.
- 셸은 design.md §8-1 의 2열이다(사이드바 `16rem`, 접으면 `3rem`, 모바일 시트 `18rem` / 본문, 페이지마다 48px 머리 하나). 우측 컨텍스트 패널은 폴더 편집 화면(`folder-editor-workspace.tsx` 의 `곡 추가` aside, `xl` 이상)에서만 쓴다. 옛 3열(256px / fluid / 320px 필터 패널)과 `FilterPanel`·`NavRail` 은 d38b6fb 에서 사라졌다.

### 1-2. Route Handler 맵 — 네이티브(계약 동결분, raw JSON)

아래 표는 `web/src/app/api`의 실제 Route Handler 전부를 담습니다. 그중 `crates/rbms-ir/src/http.rs`가 호출하는 경로는 contract-freeze A와 대조했습니다.

| 메서드 | 경로 | 파일 | 인증 | 캐시 태그(읽기) |
|---|---|---|---|---|
| GET | `/api/health` | `app/api/health/route.ts` | 없음 | 없음(항상 동적) |
| GET | `/api/version` | `app/api/version/route.ts` | 없음 | 없음 |
| GET | `/api/cron/tables` | `app/api/cron/tables/route.ts` (`maxDuration` 60) | `Authorization: Bearer <CRON_SECRET>`(Vercel Cron 이 보낸다. §15-4) | 없음(`private, no-store`) |
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
| POST | `/api/charts/[hash]/traces` | `.../traces/route.ts` | Bearer(점수의 주인만). 읽기 경로는 없다(§13) | — |
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
| PUT | `/api/admin/chart-patterns/[md5]` | `app/api/admin/chart-patterns/[md5]/route.ts` | 관리자 Bearer만(`withAdminBearer`). 세션은 받지 않는다 | 없음(`private, no-store`) |
| GET | `/api/admin/chart-patterns/coverage` | `app/api/admin/chart-patterns/coverage/route.ts` | 관리자 Bearer만(`withAdminBearer`) | 없음(`private, no-store`) |
| POST | `/api/admin/chart-patterns/recompute` | `app/api/admin/chart-patterns/recompute/route.ts` | 관리자 Bearer만(`withAdminBearer`) | 없음(`private, no-store`) |
| GET | `/api/admin/traces/usage` | `app/api/admin/traces/usage/route.ts` | 관리자 Bearer만(`withAdminBearer`). 판별 trace 보관량 수치만(§13) | 없음(`private, no-store`) |

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
| GET | `/api/fe/charts/[hash]/leaderboard` | 리더보드(랭킹 + 차트 메타 조인 + player_name·replay_id). `include_scratch_auto=true` 면 SCRATCH AUTO 판 포함 목록(API 기본은 `false`, 화면 기본은 켬: 아래 SCRATCH AUTO 절) | 없음 | `chart-ranking:{sha}` |
| GET | `/api/fe/activity/recent` | 전체 최근 제출 피드(ranked 이고 SCRATCH AUTO 가 아닌 판) | 없음 | `activity-recent` |
| GET | `/api/fe/leaderboards/players` | 종합 플레이어 랭킹. `include_scratch_auto=true` 면 SCRATCH AUTO 판까지 센다 | 없음 | `leaderboard-players` |
| GET | `/api/fe/players/[playerId]/recent` | 플레이어의 모든 판을 `played_at`·점수 id 내림차순으로(차트 조인). `page`(기본 1)·`limit`(기본 20, 최대 100), 응답은 `{ success, data, pagination: { page, limit, total, totalPages } }`. 화면 제목은 "플레이 기록" | 없음 | `player-recent:{id}`(모든 페이지가 같은 태그) |
| GET | `/api/fe/players/[playerId]/stats` | 플레이어 통계(램프 분포·레벨별 EX·플레이수) | 없음 | `player-stats:{id}` |
| GET | `/api/fe/stats/summary` | 서버 통계(차트/플레이어/제출 수) | 없음 | `stats-summary` |
| POST | `/api/fe/auth/login` | 웹 로그인(`{ id, password }` → 세션 쿠키만 세팅하고 `{ user: { id, login_id, name, role } }`. API 토큰은 만들지 않음. 5/min/IP, 게임용 `/api/auth/login` 과 같은 버킷) | 없음 | — |
| POST | `/api/fe/auth/register` | 웹 가입(`{ id, email, password, name? }` → 201, 세션 쿠키만 세팅하고 `{ user }`. API 토큰은 만들지 않음. 5/min/IP, `/api/auth/register` 와 같은 버킷) | 없음 | — |
| GET/POST | `/api/fe/tokens` | 내 API 토큰 목록 / 발급 | 세션 | 없음 |
| DELETE | `/api/fe/tokens/[tokenId]` | 토큰 폐기 | 세션·본인 | — |
| GET | `/api/fe/me/settings` | 내 설정 blob 키 목록 | 세션 | 없음 |
| GET | `/api/fe/me/traces/usage` | 내 판별 trace 보관량과 경고 단계(`ok`·`near`·`over`, §13) | 세션 | 없음(`private, no-store`) |
| GET | `/api/fe/courses` | 코스 카탈로그: 묶음마다 `{ id, name, keys, source, courses: [{ course_hash, name, stage_count, active, dan }] }`(§15-2). `withRateLimit('fe-read')` | 없음 | `course-catalog` |
| GET | `/api/fe/courses/[courseHash]/catalog` | 한 코스의 `{ group, source, dan, charts: [{ position, md5, sha256, title }] }`. 카탈로그에 없는 코스는 404 `IR_COURSE_NOT_FOUND` | 없음 | `course-catalog` |
| GET | `/api/fe/me/dans` | 내 단위: 체계 넷마다 `{ system, name, display: { mode, label, assisted }, shown: [{ system, name, label, assisted }], acquired: [{ label, assisted, rank, course_hash, course_name, course_gauge, acquired_at }] }`. `shown` 은 남에게 실제로 보이는 단위의 배열(0~2개, 빈 배열이 "없음"), A-단위의 `label` 은 서버가 `A-` 를 붙인 글자(§15-3) | 세션 | 없음(`private, no-store`) |
| PUT | `/api/fe/me/dans/display` | 한 체계에서 남에게 보일 단위: `{ system, mode: 'auto' }`(정식이 같거나 높으면 정식만, A-단위가 더 높으면 둘 다, 행 삭제) \| `{ system, mode: 'hidden' }` \| `{ system, mode: 'label', label, assisted? }`(내가 획득한 (label, assisted) 조합만, `assisted` 기본 false, 아니면 400). 1 KiB 상한 | 세션 | 없음 |
| GET | `/api/fe/tables` | 번들된 표 목록(요약) | 없음 | 정적(배포 단위) |
| GET | `/api/fe/tables/[tableId]` | 표 머리(요약)·레벨별 곡 전체 | 없음 | 정적(배포 단위) |
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

표 읽기(`/api/fe/tables*`, 표준 `header.json`·`data.json`)는 2026-10-23 부터 번들 insane1 에 DB 에 저장된 Stella·Satellite 사본(`stella-st`, `stella-sl`)을 더해서 답한다(서비스가 비동기). 표 요약에는 `source`(이름·제작 주체·원본 페이지·받은 시각·마지막 확인·해시)가 붙고 insane1 은 `null` 이다. 표준 머리는 헤더에 `course` 가 있으면 그대로 싣는다(insane1 은 바이트까지 그대로). 플레이어 프로필의 `extra.dans`(남에게 보이기로 한 `{ system, name, label }` 목록)와 순위 행의 `dans` 도 같은 날 더해졌다. 2026-10-24 부터 이 원소는 `{ system, name, label, assisted }` 이고 체계마다 0~2개다(A-단위의 `label` 은 `A-` 가 붙은 글자, §15-3).

네이티브 클라이언트가 읽는 폴더 경로(봉투 없음, Bearer 또는 세션, 본인만, 캐시하지 않음): `GET /api/players/[playerId]/folders`(`{ folders: [...] }`)와 `GET /api/players/[playerId]/folders/[folderId]`(폴더 하나와 `items`). 응답 필드는 `docs/plan/2026-10-14-web-user-folders.md` §7.2 와 같고 `GET /api/health` 의 `capabilities.folders` 가 `true` 다. API 토큰으로는 **읽기만** 되고 편집(`/api/fe/me/folders…`)은 세션만 받는다.

표준 BMS 표 형식(루트 `src/app/tables/[tableId]/`, 봉투 없음, 무인증, `Access-Control-Allow-Origin: *`): `GET /tables/insane1/header.json`(`{ name, symbol, data_url, level_order, last_update }`, `data_url` 은 **요청 origin 의 절대 주소**)과 `GET /tables/insane1/data.json`(스냅샷 배열 그대로. `ETag` 와 304 지원, `Cache-Control: public, max-age=3600, s-maxage=3600, stale-while-revalidate=86400`). 응답과 화면에는 다른 사이트의 주소·출처를 싣지 않는다(2026-10-14~15 사용자 지시: header 의 `original_url`·`mirror_of`, 요약 API 의 `original_url`·`source`, 곡의 `comment`, `artist` 등 글자 필드 속 웹 주소를 뺐고, 곡 내려받기 주소 `url`·`url_diff`·`name_diff` 는 남겼다. 정규화 규칙과 필드 목록의 정본은 [../backend/api-spec.md](../backend/api-spec.md) 7.1). 자료는 `web/data/tables/<id>/{header,data,source}.json` 을 **정적 import** 해서 쓰므로(`source.json` 은 스냅샷 재현 기록이고 `ETag` 해시만 읽는다) DB 도 파일 추적 설정(`outputFileTracingIncludes`)도 필요 없다(서버 번들에 실린다). 표를 더하려면 폴더를 만들고 `table-snapshot.data.ts` 의 `RAW_SNAPSHOTS` 에 한 줄을 더한다. 사람이 보는 페이지 `/tables/insane1`(`bmstable` 메타는 `/tables/insane1/header.json` 루트 상대 경로)은 웨이브 17B-2 에서 구현됐다.

내 기록 조회(`/records`)는 표의 md5 전부(1,035개)를 `IN` 한 번으로 묻고, 기록이 있는 곡만 `{ md5: { clear, ex_score, minbp, max_combo, total_notes, played_at, scratch_auto } }` 로 돌려준다(최대 약 150KB). 곡마다 일반 판 최고와 SCRATCH AUTO 판 최고 가운데 더 높은 것이고, 화면은 SCRATCH AUTO 쪽이면 A-SCR 표지를 붙인다(웨이브 20). 표 상세는 약 307KB(gzip 전, `comment` 를 뺀 뒤)이고 공개 캐시(`s-maxage=3600`)다. `GET /api/fe/leaderboards/players` 는 `rank_points` 를 쓰지 않는다: `sort` 는 `cleared`(클리어한 차트 수, 기본)·`played`(플레이한 차트 수)·`plays`(제출 수)이고 옛 값 `rank_points`·`total_plays` 는 각각 `cleared`·`plays` 로 받는다. 항목에 `cleared_charts`·`played_charts` 가 더해진다(클리어 = `chart_best.clear >= AssistEasy`, 곧 NO PLAY 와 FAILED 가 아닌 모든 램프. 서버는 `MIN_CLEAR_LAMP_ID`, 화면은 `MIN_CLEAR_LAMP` 하나가 정의하고 `tests/dto/clear-lamps.test.ts` 가 둘의 일치를 지킨다).

### 1-4. 인증 방식 요약 표

| 대상 | 자격 | 검증 위치 |
|---|---|---|
| 네이티브 클라(rbms-player) | `Authorization: Bearer <api_token>` | `withApiToken` HOF → `api_token.token_hash` 조회 |
| 웹 FE | better-auth 세션 쿠키(HttpOnly·SameSite=Lax) | `withSession` HOF → `auth.api.getSession({ headers })` |
| guest 제출 | 자격 없음 + `ALLOW_GUEST=true` | `POST /api/scores` 만 허용, 항상 `ranked=false` + `flags:["GUEST"]` |
| admin | 세션 + `user.role='admin'` | `withAdmin` |
| admin 스크립트(곡별 패턴 업로드) | `Authorization: Bearer <api_token>` + 토큰 사용자의 `user.role='admin'`. 역할은 요청마다 토큰 사용자 행에서 다시 읽으므로 역할을 빼면 같은 토큰이 바로 403 이다. 세션 쿠키만으로는 401 | `withAdminBearer`(`lib/with-admin-bearer.ts`), §11 |
| 릴리스 CI(빌드 등록) | 미구현. `withAdmin` 이 세션만 받아 API 토큰으로는 등록할 수 없고 `release.yml` 도 호출하지 않는다 | `POST /api/admin/builds` |
| Vercel Cron(표 매일 갱신) | `Authorization: Bearer <CRON_SECRET>`. Vercel 이 `CRON_SECRET` 환경 변수 값을 자동으로 헤더에 싣는다. 값이 없으면 503 `SERVICE_NOT_CONFIGURED`(401 과 로그에서 구별), 틀리면 401. 비교는 `timingSafeEqual`(길이가 다르면 바로 401)이고 값은 응답·로그에 넣지 않는다 | `withCronSecret`(`lib/with-cron-secret.ts`), §15-4 |

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
      error.tsx  global-error.tsx  not-found.tsx  robots.ts  sitemap.ts  favicon.ico(16·32·48)  icon.png(512)  apple-icon.png(180) — 아이콘 셋은 앱 아이콘과 같은 그림이고 `assets/icon/tools/generate-icon.py` 가 만든다(089c737, 옛 icon.svg 삭제)
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
      <domain>.query.ts  <domain>.type.ts   (auth build chart course folder player replay setting stats table table-snapshot token. rival 은 query 만, score 는 type 만)
    shared/
      ui/                             shadcn 산출물(components.json 의 aliases.ui)
      lib/  constants/  hooks/  providers/  types/  utils/
    server/                           ← 서버 전용(클라 번들 반입 금지)
      route/                          Route Handler 팩토리(도메인별 핸들러 조립)
      service/
        domain/{admin,auth,chart,course,fe,folder,player,replay,score,setting,system,table,table-snapshot}/
        shared/{storage,integrity}/        (rate limit 은 lib/with-rate-limit.ts)
      dto/                            zod 스키마(입력·응답)
      db/  index.ts  schema.ts  auth-schema.ts
      lib/  error-code.ts error-message.ts error.ts api-response.ts parse-request.ts
            with-error-handling.ts with-api-token.ts with-session.ts(withSession · withAdmin)
            with-rate-limit.ts with-lock-retry.ts mysql-error.ts env.ts auth.ts auth-context.ts token.ts
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
| 도메인 전부(`chart` `score` `chart_best` `replay` `api_token` `rival` `setting_blob` `course*` `difficulty_table` `table_*` `client_build` `submission_audit` `user_folder` `user_folder_item` `ai_*` `chart_pattern` `play_trace` `play_analysis`) | **Drizzle 수기** | `src/server/db/schema.ts` (= `docs/backend/data-model.md` 그대로) |

- contract-freeze D-2 결정 유지. 도메인 FK 는 `user.id` 참조.
- `user` 의 IR 확장 필드(`rank`·`rank_points`·`total_plays`·`allow_guest_merge`·`extra`)는 better-auth `user.additionalFields` 로 선언해 어댑터가 같은 테이블에 들고 있게 한다. **가정**: data-model.md 의 `user.id varchar(64)` = 로그인 id 를 그대로 better-auth PK 로 쓴다(better-auth 기본은 생성 id). 어긋나면 §7 오픈 질문 참조.
- `password_hash` 는 better-auth 가 `account` 테이블에 보관하므로 `user.password_hash` 컬럼은 **두지 않는다**(data-model.md 대비 유일한 의도적 이탈).

### 3-2. 마이그레이션 운영

- `drizzle.config.ts`: `schema: ['./src/server/db/schema.ts', './src/server/db/auth-schema.ts']`, `dialect: 'mysql'`, `out: './drizzle'`.
- 흐름: `bunx @better-auth/cli generate` → `bun run db:generate` → `bun run db:migrate`. 산출 파일(`drizzle/*.sql` 과 **`drizzle/meta/`**)은 **커밋**한다. `meta/` 가 없으면 `db:migrate` 는 `Can't find meta/_journal.json` 으로 멈추고 `db:generate` 는 21개 테이블 전체를 다시 만든다(2026-10-14 까지 `.gitignore` 가 `/drizzle/meta` 를 막고 있었다. 이제 풀었다).
- **`drizzle-kit push` 금지**(운영 DB 히스토리 유실). 로컬 실험도 generate/migrate 로 통일.
- 현재 파일(`web/drizzle/`, `meta/_journal.json` 과 스냅샷 5개도 커밋): `0000_smooth_cerebro`(21 테이블) → `0001_user_folders`(`user_folder`, `user_folder_item` 과 FK 둘, 인덱스 셋) → `0002_scratch_auto_bests`(`chart_best`·`course_best`·`course_score` 에 `scratch_auto` 열, `chart_best`·`course_best` 의 PK 를 `scratch_auto` 포함으로, `chart_best_rank_idx` 재생성). **셋 모두 운영 DB 에 적용됐다(2026-10-14, `web-db.yml` 의 `migrate`).** 그 뒤 `0003_replay_user_quota`(`replay.user_id` 색인, S19) → `0004_course_score_judge_totals`(`course_score` 에 판정 합계 `pgreat`·`great`·`good`·`bad`·`poor`·`miss`, `fast`·`slow`·`combobreak`·`empty_poor`·`max_ex_score` 열 추가, 모두 `int DEFAULT 0 NOT NULL`, S19 2차 R2-ACC-3)가 있고 **둘 다 2026-10-15 에 운영에 적용됐다**(아래 순서대로. 0003 은 `web-db.yml` run 38627474716, 0004 는 run 38673724720. `__drizzle_migrations` 5행). 저널 `when`: 0000 = 1788220800000, 0001 = 1791947162271, 0002 = 1791970074354, 0003 = 1791999623784, 0004 = 1792027422523 (단조 증가).
- 이후 파일: `0005_ai_foundation`, `0006_admin_seed`, `0007_chart_pattern`(아래), `0008_ai_goal`(§12-3), `0009_play_trace`(§13-2), `0010_play_share_setting`(§14-2), `0011_assist_grade`(아래), `0012_course_dan_tables`(아래, §15), `0013_assisted_dan`(아래, 운영 미적용). 위 "현재 파일" 줄은 0004 까지의 목록이다(저널 파일은 0000~0013 모두 `meta/_journal.json` 에 있다).
- **0003·0004 의 운영 적용 순서(푸시를 두 번에 나눈다. 2026-10-15 에 이 순서로 끝냈고, 다음 마이그레이션도 같은 순서를 쓴다)**: Production Branch 가 `dev` 라 `web/` 이 바뀐 푸시는 곧바로 배포되고, `web-db.yml` 의 `migrate` 는 `dev` 에 올라간 마이그레이션 파일을 읽는다. 그래서 마이그레이션 파일과 새 코드를 한 푸시에 담으면 migrate 가 끝날 때까지 새 코드가 없는 열을 읽어 코스 제출·코스 순위·코스 best 가 500 이 된다. 순서: ① 아직 `dev` 에 없는 마이그레이션 파일만(`web/drizzle/0004_course_score_judge_totals.sql`, `web/drizzle/meta/0004_snapshot.json`, `web/drizzle/meta/_journal.json`. 0003 은 이미 `dev` 에 있다) 담은 커밋을 먼저 `dev` 에 푸시한다. 이 배포는 옛 코드 그대로라(옛 코드는 새 열·색인을 읽지도 쓰지도 않는다) 안전하다 → ② `gh workflow run web-db.yml --ref dev -f action=status` 로 0003·0004 가 `would_run=yes` 인지 본다 → ③ `-f action=migrate`(0003 색인, 0004 열 추가. 둘 다 기존 행을 바꾸지 않는 추가뿐이고 0004 의 새 열은 기본값 0) → ④ 다시 `status` 가 최신인지 본다 → ⑤ 그 다음에 `web/src` 와 테스트를 담은 코드 커밋을 푸시한다. ①과 ⑤ 사이에 `web/src` 를 바꾼 커밋이 끼면 같은 문제가 생긴다.
- **0007_chart_pattern(W3, 2026-10-20 생성. 생성 당시 운영 미적용이었고 이후 0010 까지 적용됐다)**: `chart_pattern` 표 하나와 `chart_pattern_sha256_idx` 색인을 더할 뿐 기존 표를 바꾸지 않는다. 저널 `when` = 1792423286917(0006 보다 큼). 운영 적용은 위와 같은 두 번 푸시 순서(마이그레이션 파일만 먼저 → `status` → `migrate` → `status` → 코드)를 따른다. 로컬 `mysql:8.4.11`(기본 `max_allowed_packet` 64 MiB)에서 0000~0007 적용과 7만 노트(약 2 MB JSON) 패턴의 저장·읽기를 확인했다(`tests/db/chart-pattern.test.ts`). 운영 DB 의 `max_allowed_packet` 은 아직 보지 못했다.
- **0011_assist_grade(2026-10-22, 운영 적용)**: `chart_best` 와 `course_best` 에 `assist_grade tinyint DEFAULT 0 NOT NULL` 을 더하고 색인 `chart_best_rank_grade_idx(chart_sha256, scratch_auto, clear, assist_grade, ex_score)` 를 만든 뒤, 마지막 문장(수동 SQL)이 `chart_best` 를 `score` 와 조인해 `assist_grade` 를 채운다(`clear = 3` 이면 `score.gauge` 별 등급 1/2/3/4, 그 밖은 0. WHERE 가 없어 멱등이고 `updated_at` 은 바뀌지 않는다). 앞 세 문장은 `drizzle-kit generate` 산출물이고 이름만 `0011_assist_grade` 로 바꿨다(다시 generate 하면 'No schema changes'). 옛 `chart_best_rank_idx` 는 additive 규칙에 따라 그대로 둬 두 색인의 앞부분이 겹친다. 저널 `when` = 1792606768379. 운영 적용은 위 두 번 푸시 순서였다: 마이그레이션 파일만 담은 `8d85b32` → `web-db.yml` `migrate`(run 39693988962, 0011 recorded) → 웹 코드 `64c6120`. 푸시 사이에 옛 코드가 `chart_best` 를 바꿔 생길 수 있는 낡은 등급은 `web-db.yml` 액션 `assist-grade-backfill-count`(세기만: 낡은 행 수, 0 초과 등급 행 수, 점수 행이 사라진 행 수)와 `assist-grade-backfill`(세기 → `assist-grade.backfill.sql` → 세기, 바뀔 행에만 걸려 멱등)으로 바로잡는다(`web/scripts/assist-grade.{count,backfill}.sql`). 운영에서 낡은 행 0 을 확인했다(run 39733088817). `course_best.assist_grade` 는 0012 까지 항상 0 이었다(코스는 게이지를 저장하지 않았다). 0012 이후의 코스 제출은 마지막 스테이지의 게이지로 이 열을 채운다(§15-1). 로컬 `mysql:8.4.11` 에서 0010 상태 DB 에 옛 행을 넣고 실제 `db:migrate` 를 돌려 게이지 0..8 별 등급이 0,1,2,3,4,4,2,3,4 로 채워지고 게이지 99·clear 6·clear 2·가리키는 score 가 없는 행은 0, `updated_at` 불변, 새 색인 생성을 확인했다. SQL 의 CASE 와 TS 표 `ASSIST_GRADE_OF_GAUGE` 가 같은지는 단위 테스트가 파일을 읽어 비교한다. 의미: [../reference/clear-lamps.md](../reference/clear-lamps.md) 1.8·3.3절.
- **0012_course_dan_tables(2026-10-23, 운영 적용)**: 추가만. 새 표 일곱: `course_source`(출처 한 건: `kind` = `bundled` \| `table`, 제작 주체, 페이지·파일 주소, 받은 시각, 파일 SHA-256, 나머지는 `detail` json), `course_group`(화면 묶음: 이름, `keys`, `position`), `course_group_member`(PK `group_id`+`course_hash`, `position`, `active`, 색인 `course_group_member_course_idx`), `dan_course`(PK `course_hash`; "이 코스를 인정 조건대로 끝내면 체계 `system` 의 단위 `label`(순서 `rank`)을 준다", 색인 `dan_course_system_idx(system, rank)`), `user_dan`(PK `user_id`+`system`+`label`, 처음 획득만, `course_hash`·`course_score_id`·`course_gauge`·`gauge`·`acquired_at`, 사용자를 지우면 연쇄 삭제), `user_dan_display`(PK `user_id`+`system`, `label` NULL = 숨김, 행 없음 = 자동), `table_snapshot`(표마다 한 행: 받은 머리·본문의 gzip `mediumblob` 둘과 SHA-256 둘, `entry_count`, `level_count`, `fetched_at`, `checked_at`, `last_outcome`). 기존 표에 열 다섯: `course_score.gauge tinyint`(NULL = 게이지를 모르는 옛 클라이언트), `course_score.course_gauge varchar(16)`, `course_score.stages json`, `course_chart.md5 char(32) DEFAULT ''`, `course_chart.title varchar(255) DEFAULT ''`. 기존 열의 타입·NULL 여부를 바꾸지 않고 기존 행을 고치는 SQL 도 없다. md5 만 있는 코스 차트는 `chart_sha256 = ''` 로 저장한다(그 열이 NOT NULL 이라 빈 문자열이 들어가고, 코스 id 는 그때 md5 로 계산된다). 저널 `when` = 1792731542379. 운영 적용: 마이그레이션 커밋 `9e39262` → `status`(run 39904962733) → 사용자가 `!` 로 `migrate` 실행(run 39909099575, 앞 상태 12행 → 적용 뒤 최신) → 코드. 단 `9e39262` 에 웨이브 1 전체와 `db/schema.ts` 가 함께 들어가 `schema.ts` 가 migrate 보다 먼저 배포됐다(사고 기록: [../feedback/2026-10-23-stage-check-before-commit.md](../feedback/2026-10-23-stage-check-before-commit.md)). 그 구간에 옛 DB 로 새 열을 읽는 코스 경로가 500 이 날 수 있었으나 관찰한 피해는 기록되지 않았다.
- **0013_assisted_dan(2026-10-24, 커밋 `35f77ae`, 운영 미적용)**: `user_dan` 에 `assisted boolean DEFAULT false NOT NULL` 을 더하고 기본 키를 `(user_id, system, label)` 에서 `(user_id, system, label, assisted)` 로 바꾸며(`DROP PRIMARY KEY, ADD PRIMARY KEY(...)` 를 **한 문장**으로), `user_dan_display` 에 같은 열을 더한다(그 표의 기본 키 `(user_id, system)` 은 그대로). 기존 행은 모두 `assisted = 0`(정식)이 되고 데이터를 바꾸는 문장은 없다. 문장이 셋이다: ① `ADD assisted`, ② 기본 키 교체, ③ 표시 선택 표의 `ADD assisted`. `drizzle-kit generate` 는 기본 키 교체를 `DROP PRIMARY KEY;` 와 `ADD PRIMARY KEY(...)` 의 두 문장으로 내는데, 첫 문장이 `user_dan_user_id_user_id_fk` 가 기본 키의 첫 열 색인을 쓰는 탓에 MySQL 8.4.11 에서 `ERROR 1553` 으로 실패해 손으로 한 문장으로 합쳤다(파일 이름도 `0013_bouncy_randall` 에서 `0013_assisted_dan` 으로 바꿨다. 합친 뒤 `db:generate` 는 'No schema changes'). 저널 `when` = 1792824932355(0012 보다 큼). 되돌림 마이그레이션은 만들지 않는다(`assisted = 1` 행이 생기면 기본 키를 되돌릴 수 없다). 공존: 마이그레이션이 먼저 적용되고 새 코드가 아직 없는 동안 옛 코드의 `INSERT`/`INSERT IGNORE` 는 `assisted` 를 보내지 않아 기본값 false 인 정식 행으로 정상 동작한다(로컬 `mysql:8.4.11` 에서 확인: 0000~0012 적용 뒤 행을 넣고 0013 을 적용하자 기존 `user_dan`·`user_dan_display` 행 모두 `assisted = 0`, 같은 (user, system, label) 의 정식과 A-단위가 함께 들어가고, `INSERT IGNORE` 와 `ON DUPLICATE KEY UPDATE` 가 기대대로 동작, 새 빈 DB 에 0000~0013 을 한 번에 적용). 이 커밋에는 `schema.ts`·`src` 를 섞지 않았다(0012 사고의 반복 방지, 위 0003·0004 의 두 번 푸시 순서). 운영 적용 순서는 [../plan/2026-10-24-assist-dan.md](../plan/2026-10-24-assist-dan.md) 10절: 마이그레이션 파일만 푸시 → `status` 로 `would_run=yes` → 사용자 `!` 로 `migrate` → `status` 최신 → 서버·웹 코드. 운영 `user_dan` 행 수는 확인하지 못했다(기본 키 교체가 표를 다시 만든다. 0002 가 같은 형태로 적용됐고 표는 작다고 본다).
- **저널 `when` 의 규칙**: 설치된 drizzle-orm 0.45.2 의 마이그레이터(`mysql-core/dialect.js` 의 `migrate`)는 `__drizzle_migrations` 에서 `created_at` 이 가장 큰 행 하나만 읽고, 저널 항목의 `when` 이 그 값보다 **큰** 항목만 실행한다(해시는 비교하지 않는다. `drizzle-kit migrate` 는 `drizzle-orm/mysql2/migrator` 를 그대로 부른다). 그래서 복원한 `0000` 의 `when` 은 운영에 적용된 값(2026-09-09 근처)보다 작은 **2026-09-01 00:00 UTC(`1788220800000`)** 로 두었다. 운영에서 `0000` 은 건너뛰고 `0001`(`when` = 생성 시각)만 실행된다. 새 마이그레이션은 `db:generate` 가 현재 시각으로 `when` 을 붙이므로 항상 더 크다.
- **운영 DB 의 상태 확인**: `bun scripts/db-migration-status.ts`(환경 변수의 DB 로 접속, 읽기 전용 쿼리만. 해시 앞 8자·`created_at`·파일별 적용 여부·새 테이블의 존재만 출력하고 접속 정보와 자료는 출력하지 않는다). 종료 코드 0 = 최신, 1 = 할 일이 있음(마이그레이션 대기 또는 기준선 필요), 2 = 접속 실패.
- **운영이 `push` 로 만들어져 `__drizzle_migrations` 가 없다면(기준선 절차)**: `db:migrate` 를 먼저 돌리면 `0000` 의 `CREATE TABLE` 이 "이미 있음" 으로 실패한다. 상태 스크립트가 이 경우를 알려 주면 ① `bun scripts/db-migration-status.ts --baseline-sql` 의 출력(`__drizzle_migrations` 테이블 생성과 `0000` 의 해시·`when` 한 행 삽입)을 DB 에 한 번 적용하고 ② 상태를 다시 확인한 뒤 ③ `bun run db:migrate` 를 돌린다. 이 절차는 로컬 MySQL 에서 `0000` 을 직접 적용해 만든 push 상태의 DB 로 확인했다.
- **운영 DB 작업 워크플로(2026-10-14, 운영 DB 는 GitHub 러너에서 접속됨 — 확인됨. 결정 근거: [../acknowledge/2026-10-14-adr-production-db-operations.md](../acknowledge/2026-10-14-adr-production-db-operations.md))**: `.github/workflows/web-db.yml`(수동 실행). 운영 `DATABASE_URL` 은 Vercel 에 Secret 형식으로 있어 `vercel env pull`·`vercel env run` 으로 받을 수 없으므로(확인: "2 Secret values cannot be pulled"), 저장소 시크릿 `WEB_DATABASE_URL` 로 둔다. 동작: `status`(상태 확인), `migrate`(상태 → 적용 → 상태), `cleanup-count`, `cleanup-delete`(세기 → 지우기 → 다시 세기), `chart-notes-count`, `admin-seed-count`(`web/scripts/admin-seed.count.sql`, 읽기만 한다. 관리자 지정 마이그레이션 `0006_admin_seed` 를 적용하기 전에 `hs@gumyo.net` 행 수·바뀔 행 수·현재 관리자 수 세 숫자만 출력하고 행 내용은 출력하지 않는다. 바뀔 행이 1 이어야 migrate 한다), `chart-notes-repair`·`chart-notes-restore`·`chart-notes-drop-backup`(세기 → 그 SQL → 다시 세기. 복구·되돌리기·백업 삭제, S19 3차 R3-WEB-3 에 되돌리기·백업 삭제를 더함. 쓰임새는 테스트 부채 3.36). chart-notes SQL 의 뜻: `chart-notes.count.sql` 은 읽기만 하고 여섯 줄을 센다(LN 모드·SCRATCH AUTO 아닌 판의 `total_notes` 와 다른 `chart.notes`, 복구가 바꿀 행, 정상 판을 노트 수의 2배 넘게 담은 행, 점수 없는 차트 행, 누적 거절 제출 수, `chart_notes_backup` 표의 존재). `chart-notes.repair.sql` 은 바꿀 행을 `chart_notes_backup` 에 먼저 담고(표가 있으면 첫 문장에서 멈춤) 가장 많은 사용자가 보낸 `total_notes` 로 `chart.notes` 를 바꾼다. `chart-notes.restore.sql` 은 복구 뒤 다시 바뀌지 않은 행만 옛 값으로 되돌리고, `chart-notes.drop-backup.sql` 은 백업 표를 지운다. `chart.notes` 는 표시용이라(제출 검증의 상한이 아니다) 복구는 표시만 바꾼다. 2026-10-15 운영 `chart-notes-count`(run 38673771025): 어긋난 행 0, 복구 대상 0, 점수 없는 차트 행 3, 누적 거절 39. 복구는 돌리지 않았다. 값은 출력하지 않는다. 실행: `gh workflow run web-db.yml --ref dev -f action=<동작>`. SQL 파일은 `web/scripts/run-sql.ts`(`web/scripts/` 아래의 `.sql` 만, 여러 문장 허용)로 돈다. (`web-db.yml` 의 `migrate`·`cleanup-delete` 는 실제로 돌렸고, 시험 계정 5개는 삭제됐다. 남은 데이터는 다른 사용자 점수 3건.) 운영 DB 가 러너의 접속을 받지 않게 되면 같은 명령을 로컬에서 `DATABASE_URL` 을 환경에만 주고 실행한다. 이 저장소에서 `.env` 계열은 읽지 않는다. 2026-10-22~23 에 동작이 더해졌다: `assist-grade-backfill-count`·`assist-grade-backfill`(위 0011), `chart-purge-count`·`chart-purge-delete`(사이트 주인의 요청으로 지운 차트 하나의 행. [ADR](../acknowledge/2026-10-23-adr-chart-removed-at-owner-request.md)), `courses-seed-count`·`courses-seed`(GENOSIDE 2018 코스 등록), `tables-refresh-count`·`tables-refresh-force`(Stella·Satellite 사본 확인·강제 저장), `dan-recompute-count`·`dan-recompute`(저장된 판에서 단위 다시 판정). 전체 열아홉 개와 각 동작이 도는 것은 [../ci-release.md](../ci-release.md) 6.1절이다. 뒤의 세 쌍은 SQL 이 아니라 `web/scripts/{seed-courses,refresh-tables,dan-recompute}.ts` 를 돈다. 운영 DB 쓰기는 여전히 이 워크플로가 원칙이고, 예외는 Vercel Cron 이 매일 도는 표 사본 하나다(§15-4). 2026-10-23 운영 실행: `chart-purge-delete` run 39927201095, `courses-seed` run 39946701885(`genoside-2018-sp` 22개·`genoside-2018-dp` 22개 등록), `tables-refresh-force` run 39946847395(`stella-st` 2,350곡·`stella-sl` 2,497곡 첫 저장). 전체 기록은 ci-release.md 6.1절이다.
- **적용 순서**: 마이그레이션을 **먼저** 운영 DB 에 적용하고 **그 다음** 웹을 배포한다. 새 테이블을 쓰는 코드가 먼저 나가면 폴더 경로가 500 을 낸다. `dev` 푸시가 곧 배포이므로 "먼저 적용" 은 마이그레이션 파일만 담은 커밋을 먼저 푸시하고 migrate 한 뒤 코드 커밋을 푸시하는 두 번의 푸시로만 지킬 수 있다(위 0003·0004 순서). 마이그레이션은 빌드에 포함하지 않는다(`DATABASE_URL` 은 사용자 PC 의 환경에만 준다).
- 인덱스 우선 적용: `chart(uniq md5)`, `score uniq(user_id, chart_sha256, played_at)`(멱등), `score idx(chart_sha256, ranked, clear, ex_score)`, `chart_best PK(chart_sha256, user_id, scratch_auto)`(웨이브 20, 0002), `chart_best idx(chart_sha256, scratch_auto, clear, ex_score)`, `chart_best idx(chart_sha256, scratch_auto, clear, assist_grade, ex_score)`(2026-10-22, 0011), `score idx(client_build_sha256)`.
- 커넥션: `mysql2`(콜백 API) 풀 싱글톤을 Drizzle 이 감싼다(`src/server/db/index.ts`). Vercel Fluid Compute 특성상 인스턴스당 풀이 살아있으므로 `connectionLimit: 5`, `enableKeepAlive: true`. 풀이 새 연결을 만들 때마다 `SET SESSION innodb_lock_wait_timeout = 5`(`LOCK_WAIT_TIMEOUT_SECONDS`)를 보내고, 쓰기는 교착·잠금 대기 초과 때 `withLockRetry` 로 다시 한다(근거와 적용 범위는 [../backend/data-model.md](../backend/data-model.md) "잠금 오류 재시도").

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
| `POST /api/auth/login` · `/register` | 5/min/IP(로그인 한 버킷, 가입 한 버킷) |
| `POST /api/scores` · 코스 제출 | 60/min/사용자(guest 점수 제출은 IP. 점수와 코스는 다른 버킷) |
| `DELETE /api/auth/token/current` | 5/min/IP (로그인·가입과 다른 버킷) |
| `PUT /api/players/[id]/settings/[name]` | 30/min/사용자 |
| `POST /api/charts/[hash]/replays` | 10/min/사용자 |
| `POST /api/charts/[hash]/traces` | 30/min/사용자(`TRACE_UPLOAD_RATE_LIMIT`. 클라이언트는 2.5초 간격, 분당 24건) |
| `/api/fe/*` 공개 읽기, `/api/auth/get-session`·`sign-out` | 120/min/IP |
| `POST /api/fe/auth/login` · `/register` | 5/min/IP (각각 `/api/auth/login`·`/api/auth/register` 와 같은 버킷) |
| 폴더 편집(만들기·고치기·지우기·순서·곡 넣고 빼기) | 60/min/사용자 |
| 폴더에 표 가져오기 | 10/min/사용자 |
| 폴더 읽기(웹·클라이언트) | 120/min/사용자 |
| 표의 내 기록 조회 | 30/min/사용자 |
| 곡별 패턴 업로드(`PUT /api/admin/chart-patterns/[md5]`) | 1,200/min/관리자(`CHART_PATTERN_WRITE_RATE_LIMIT`). 업로드 스크립트가 요청 시작 간격을 `UPLOAD_REQUEST_INTERVAL_MS`(56 ms, 한도의 90%)로 벌려 7K 전체 7,903곡을 한 번에 올려도 넘지 않는다(§11-2) |
| 곡별 패턴 일괄 재계산(`POST /api/admin/chart-patterns/recompute`) | 30/min/관리자(`CHART_PATTERN_RECOMPUTE_RATE_LIMIT`) |

- 구현: `withRateLimit(scope, { limit, windowMs })`(IP 키)와 `enforceUserRateLimit(scope, userId, config)`(사용자 키), 값은 `RATE_LIMIT` 상수. **v1 은 인스턴스 로컬 고정 창 카운터(`Map`, 버킷 5,000개를 넘으면 만료된 것을 정리)** — Vercel 멀티 인스턴스에서는 best-effort. 정본 표는 [../backend/api-spec.md](../backend/api-spec.md) 10절. 로그인/가입을 DB 카운터로 보강하는 계획은 **구현하지 않았다**(KNOWN ISSUE: 가입·로그인 5/분도 인스턴스 로컬이라 Vercel 다중 인스턴스에서 한도가 인스턴스 수만큼 늘고 콜드스타트마다 초기화된다). 전역 정확성이 필요해지면 Upstash Redis 등 지속형 저장소나 Vercel Firewall 규칙을 도입한다(웹 2차 후보, 사용자 결정 필요).
- IP 키(2026-10-14 웨이브 17B-2 리뷰): `getClientIp` 는 `X-Forwarded-For` 의 첫 값이다. Vercel 은 이 헤더를 덮어쓰고 바깥 IP 를 넘기지 않으므로(Vercel 문서 "Request headers", 스푸핑 방지) 운영에서는 클라이언트가 바꿀 수 없다. 다른 호스팅이나 Vercel 앞의 다른 프록시에서는 헤더를 바꿔 한도를 피할 수 있으므로(로컬에서 재현), 호스팅을 바꾸거나 지속형 한도로 옮길 때 신뢰하는 헤더를 하나로 고정한다.
- 본문 크기: `POST /api/scores`·코스 제출·설정 PUT 256KB, `POST /api/charts` 64KB(`content-length` 헤더 + 실제 스트림 바이트 수 둘 다 검사, `parseLimitedJsonBody`), 리플레이 `REPLAY_MAX_BYTES`(기본 4MB), 곡별 패턴 `CHART_PATTERN_MAX_BYTES`(4 MiB, 측정한 가장 큰 7K 차트가 약 1.6 MB), 판별 trace `TRACE_MAX_BYTES`(2 MiB, 클라이언트 `TRACE_MAX_BYTES` 와 같다) 초과 시 `413 IR_PAYLOAD_TOO_LARGE`.

### 4-5. 보안 기본

- 비밀번호는 better-auth 위임. 토큰은 해시 저장. 시크릿은 `getEnv()` 경유만.
- CORS: 같은 오리진(Next 단독 서버)이라 FE 용 CORS 불필요. 네이티브 클라는 브라우저가 아니라 CORS 무관. 따라서 **CORS 미들웨어를 두지 않는다**(api-spec §10 대비 이탈 — 단독 서버 구조의 결과). 외부 오리진 요구가 생기면 `ALLOWED_ORIGINS` 로 추가.
- 제출 검증: 토큰 user.id == body `player.id`(guest 제외), `played_at` 이 서버 시각보다 5분 넘게 미래가 아님(`isPlayedAtPlausible`, 과거 쪽 한도는 없다), `chart.sha256` 형식(소문자 16진수 64자), DB 열 범위, 그리고 **본문만 보는** 일관성 검사 6개(EX·노트를 소비한 판정 수·`max_ex_score`·`minbp`·램프·저장할 EX/BP 의 INT 범위). 서버의 `chart.notes` 는 첫 제출자가 정한 표시용 값이라 상한으로 쓰지 않는다(S19 2차). 코스 제출도 같은 한도(60/min, 256KB, 빌드 해시 정책)와 본문 검사(램프 제외)를 받는다. 규칙 전문의 정본은 [../backend/api-spec.md](../backend/api-spec.md) 5·6절.
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

**제약**: `'use cache'` 안에서는 `cookies()`/`headers()`/`searchParams` 접근 불가(번역도 같다: `getTranslations`·`getLocale` 은 그 바깥에서, §16-2. 루트가 쿠키를 읽으므로 모든 페이지가 요청마다 렌더되고 캐시되는 것은 데이터뿐이다). 따라서 개인화 데이터(내 설정 blob, 내 토큰 목록, `auth/me`)는 캐시하지 않고, 필요한 식별자는 **인자로 넘겨** 캐시 키에 포함시킨다(`getPlayerScores(playerId, limit)`).

### 5-2. 태그 사전

| 태그 | 내용 | cacheLife |
|---|---|---|
| `chart:{sha256}` | 차트 메타 | `days` |
| `chart-ranking:{sha256}` | 차트 랭킹 상위 N(2026-10-24 부터 줄마다 `extra.dans` 포함: 단위를 얻거나 표시를 바꿔도 이 시간만큼 이전 값이 보인다) | `minutes` |
| `chart-best:{sha256}` | 차트 개인 베스트(플레이어 인자 포함) | `minutes` |
| `chart-replays:{sha256}` | 차트 리플레이 목록 | `minutes` |
| `chart-search` | 검색 결과 | `minutes` |
| `player:{id}` | 프로필 | `minutes` |
| `player-scores:{id}` · `player-recent:{id}` · `player-stats:{id}` | 플레이어 기록/통계 | `minutes` |
| `player-rivals:{id}` | 라이벌 목록 | `hours` |
| `course:{hash}` · `course-ranking:{hash}` · `course-best:{hash}` | 코스 | `minutes` / `days`(메타) |
| `course-catalog` | 코스 카탈로그 목록과 항목(`/api/fe/courses*`, `/courses`) | `minutes` |
| `player-profiles` | 모든 플레이어 프로필(`player:{id}` 와 같이 붙는다. 단위가 새로 주어졌을 때 한꺼번에 무효화) | `minutes` |
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
| `POST /api/courses` 가 단위를 줬을 때 | `player:{loginId}` · `leaderboard-players` |
| `PUT /api/fe/me/dans/display` | `player:{loginId}` · `leaderboard-players` |
| Cron 표 갱신(`revalidateOnTableRefresh`) | 표가 저장됐거나 코스가 등록·바뀌었으면 `course-catalog`; 새로 단위 코스가 된 코스의 `course:{hash}`; 단위가 주어졌으면 `player-profiles` · `leaderboard-players` |
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
- **없는 상세 페이지의 404 상태**(S19, 이전 KNOWN ISSUE "soft 404" 해소): Cache Components 에서는 `Suspense` 안의 `notFound()` 가 이미 200 을 보낸 뒤라 상태를 못 바꾼다(Next 문서 loading.md "Status Codes"). 그래서 `src/proxy.ts` 가 `/charts/[hash]`·`/players/[id]`·`/courses/[hash]`·`/replays/[id]` 에서 존재를 먼저 확인한다(`server/service/detail-existence.ts`: 차트는 해시 모양과 `findByHash`, 플레이어는 로그인 id, 코스는 `courseService.exists`, 리플레이는 `replayService.exists` — 리플레이 본문은 읽지 않는다). 없으면 `NextResponse.rewrite` 로 존재하지 않는 경로를 보여 앱의 `not-found` 화면을 진짜 404 로 낸다. 조회가 실패하면(DB 끊김 등) 판단하지 않고 통과시켜 멀쩡한 페이지를 404 로 만들지 않는다. 운영 확인(2026-10-15, 읽기 전용 GET): `/charts/zzzz`·`/players/no-such-player-xyz`·`/courses/nocourse`·`/replays/rp_nope` 가 404, `/players/bhs` 는 200. 같은 조회는 페이지 안의 `fetchQueryOrNotFound` 가 한 번 더 한다(경합 대비). 표 상세(`/tables/[id]`)는 번들 표라 이번 범위에 넣지 않았다.
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
| `AI_CREDENTIAL_KEKS` | AI 기능에 필수 | 저장한 AI 키의 데이터 키를 감싸는 KEK 들. 버전 이름 → base64(32바이트) 의 JSON 객체. 스키마는 `.optional()` 이고 키를 처음 봉인·개봉할 때 `readAiCredentialKeyRing()` 이 검사한다. 없거나 틀리면 AI 경로만 503 `SERVICE_NOT_CONFIGURED` |
| `CRON_SECRET` | 표 매일 갱신에 필수(Sensitive) | 16자 이상(`CRON_SECRET_MIN_LENGTH`), 없어도 스키마는 통과한다. Vercel 이 이 값을 `Authorization: Bearer` 로 Cron 요청에 싣고 `/api/cron/tables` 가 같은 값인지 확인한다. 없으면 그 경로만 503 `SERVICE_NOT_CONFIGURED`, 값은 사용자가 Vercel 에 직접 넣는다 |
| `AI_CREDENTIAL_KEK_CURRENT` | AI 기능에 필수 | 새로 감쌀 때 쓰는 KEK 버전 이름(`AI_CREDENTIAL_KEKS` 안에 있어야 한다). 옛 버전으로 감싼 행은 읽을 때 이 버전으로 다시 감싼다(최선 노력) |
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

> 확인한 것: `curl https://bms.hyuns.uk/api/version` 이 `{"api_version":1,"server":"0.1.0","commit":"7095ef7"}` 를 돌려줘(2026-10-14 관찰) Git 연동 배포이고 `dev` 의 최신 커밋이 운영에 올라 있다. 2026-10-15 관찰값은 `"commit":"51f3a1e"` 이다(그 뒤 커밋은 `web/` 을 바꾸지 않아 `ignoreCommand` 가 건너뛴다). 응답 헤더에 §4-5 의 보안 헤더와 CSP 가 모두 있다. 확인하지 못한 것: Vercel 대시보드 값(프로젝트 `rbms-web`(팀 b-hs-projects), Root `web`, Production `dev`, 환경 변수 6개 중 `DATABASE_URL`·`BETTER_AUTH_SECRET` 은 Secret)은 세션 인벤토리의 기록이며 이 문서 점검에서 Vercel 에 접근하지 않았다. 변경 없는 푸시가 `CANCELED` 로 기록되는지(`ignoreCommand`)도 대시보드에서만 본다.

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
| `AI_CREDENTIAL_KEKS` `AI_CREDENTIAL_KEK_CURRENT` | AI 기능에 필수(Sensitive) | 6-1. 값은 사용자가 만들어 넣는다. 없으면 AI 경로만 503 |
| `CRON_SECRET` | 표 매일 갱신에 필수(Sensitive) | 6-1. 16자 이상 무작위. 값은 대화·저장소·로그에 두지 않고 Vercel 대시보드 또는 `vercel env add` 의 프롬프트에 직접 넣는다. 등록 전에는 첫 Cron 이 503 이다 |

#### Cron (2026-10-23)

- `web/vercel.json` 의 `crons`: `[{ "path": "/api/cron/tables", "schedule": "0 15 * * *" }]`. Vercel 은 Production 배포 주소로 GET 을 보내고 시각은 항상 UTC 다(15:00 UTC = KST 0시). 같은 예약이 두 번 와도, 실패해도 다시 보내지 않는다("best effort", 설계서가 2026-10-23 에 읽은 공식 문서 `vercel.com/docs/cron-jobs`). 그래서 갱신은 멱등이고(내용이 같으면 확인 시각만 바꾼다) 하루를 놓치면 다음 날이 따라잡는다. 리다이렉트는 따르지 않고, 지속 시간 제한은 Functions 와 같다(경로에 `maxDuration = 60`). Hobby 요금제면 실행이 지정 시각부터 59분 안에 흩어진다. 요금제는 저장소에서 확인되지 않았다.
- `web/tests/lib/vercel-config.test.ts` 가 `crons` 항목과 그 경로의 라우트 파일, 그리고 `ignoreCommand` 가 그대로임을 고정한다.
- 두 표의 첫 사본은 2026-10-23 에 `web-db` 의 `tables-refresh-force`(run 39946847395)로 이미 저장됐다(`stella-st` 2,350곡·`stella-sl` 2,497곡). Cron 의 첫 실행은 `CRON_SECRET` 이 Vercel 에 등록돼 있어야 하고, 등록·첫 응답은 저장소에서 확인되지 않았다. 등록 전 `web-db` 의 `tables-refresh-count` 로 러너가 원본을 받아 검증되는지 먼저 본다. 원본은 Cloudflare 뒤에 있어 Vercel 서버의 요청이 봇 검사에 걸릴 수 있다(확인 못함): 걸리면 `failed:http_status` 가 매일 남고, 같은 일을 러너에서 하는 `tables-refresh-force` 가 대신한다.

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
| SCRATCH AUTO(웨이브 20, 0b04629) | 사용자 결정(2026-10-14, 근거 [../acknowledge/2026-10-14-adr-scratch-auto-rule.md](../acknowledge/2026-10-14-adr-scratch-auto-rule.md)·[../acknowledge/2026-10-14-adr-scratch-auto-ranking-toggle.md](../acknowledge/2026-10-14-adr-scratch-auto-ranking-toggle.md)): SCRATCH AUTO 판도 일반 판처럼 순위에 올린다(2026-10-19 부터 웹 화면은 기본으로 포함하고 토글로 끈다. [../acknowledge/2026-10-19-adr-web-clear-and-play-history.md](../acknowledge/2026-10-19-adr-web-clear-and-play-history.md)). `chart_best`·`course_best` 를 `scratch_auto` 로 나눠 일반 판 최고와 SCRATCH AUTO 판 최고를 따로 남기고, 목록 질의는 `best-scope.ts` 의 `chartBestScope`·`courseBestScope` 로 "일반 판만" 또는 "플레이어마다 더 높은 쪽"을 고른다. 차트 순위·코스 순위·리더보드 화면에는 "SCRATCH AUTO 포함" 스위치가 있다. 화면 기본은 켬이고 주소에 `include_scratch_auto` 가 없으면 켠 것으로 읽으며, 끄면 `include_scratch_auto=false` 가 주소에 남고 켜면 파라미터를 지운다(쿼리 키에 포함). 서버 API 의 기본값은 그대로 `false` 다: Rust 클라이언트가 파라미터를 싣지 않고 기본 목록(일반 판만)에 기대기 때문이다. 목록의 SCRATCH AUTO 행에는 A-SCR 배지. SCRATCH AUTO 판의 `rank` 는 그 판이 속한 목록(일반 판은 기본 목록, SCRATCH AUTO 판은 포함 목록) 기준이고 `previous_best`·`is_new_best` 는 그 판의 칸 기준이다(`score.service.ts` `submit`). 넓힌 판정(`options.assist` 의 `CUSTOM_JUDGE`)은 `ASSIST` flag 로 그대로 unranked |
| 클라 정책 정합 | 옛 결정 #12("어시스트 판은 최고 기록이 아니다")는 2026-10-14 사용자 결정으로 **대체**됐다: 손으로 친 모든 판은 클라이언트가 저장·전송하고, SCRATCH AUTO 는 서버에서도 ranked 다. 넓힌 판정(클라이언트 어시스트 태그 `CUSTOM_JUDGE`)은 서버에서 `ASSIST` flag + unranked 이고 `chart_best`·순위에 들어가지 않는다. SCRATCH AUTO 램프는 `LightAssistEasy` 로 유지한다. 둘 다 2026-10-14 사용자 확인(S18 결정 6). 서버는 클라이언트의 판단을 신뢰하지 않고 독립 재판정한다 |
| FR-18 감사 로그 | `score.service.ts` 의 `submit` 이 `deps.db.insertAudit`(compose 구현)을 불러 `submission_audit` 에 insert 한다. IP 는 `x-forwarded-for` 첫 값, UA 는 `user-agent`. 별도 `audit.ts` 파일은 없다 |
| best 갱신 | `chart_best` 조건부 갱신(키 `(chart, user, scratch_auto)`, `upsertChartBestIfBetter`: `INSERT IGNORE` 뒤 `UPDATE … WHERE` 저장된 값이 더 나쁨(`isBeatenBy`), `affectedRows` 로 `is_new_best`). 비교 순서 **램프 > EX > BP(낮을수록)**. `ranked=false` 는 갱신 대상 제외. 코스는 `upsertCourseBestIfBetter` |
| 멱등 | 서비스가 같은 사용자·같은 차트에서 `played_at` 이 **같은 초**(`floor(played_at / 1000)` 이 같음)인 점수를 먼저 찾아(`findScoreByIdempotency`) 있으면 기존 `score_id` 를 돌려준다(201 대신 200, `message: "duplicate submission ignored"`). 저장된 판이 ranked 면 그 판의 값으로 최고 기록 조건부 갱신을 한 번 부른다(점수 행만 남고 최고 기록 갱신 전에 실패한 첫 요청의 복구, S19 4차). 코스 멱등은 같은 밀리초다. `score uniq(user_id, chart_sha256, played_at)` 색인은 정확히 같은 밀리초의 이중 삽입만 막는다. 기록 동기화가 기록 DB 의 초 단위 시각으로 다시 보낸 판을 중복 저장하지 않으려는 규칙이다 |

---

## 8. 테스트 전략

| 층 | 러너 | 대상 |
|---|---|---|
| dto | `bun test` | zod 라운드트립. **`crates/rbms-ir/src/dto.rs` 가 실제로 보내는 JSON 픽스처**를 그대로 파싱 성공하는지(계약 회귀 방지) |
| lib | `bun test` | `createAppError`/STATUS_MAP, `api-response` 헬퍼, `getEnv` 검증 실패 케이스, 토큰 해시 |
| service | `bun test` | `*ServiceDb` 를 인라인 객체로 대체(mocking 라이브러리 없음). best 갱신 규칙·멱등·EX/BP 유도·낙관적 잠금 충돌·`rankedPolicy` 전 flags |
| Route Handler | `bun test` | 핸들러를 직접 import 해 **`new Request(url, {...})`** 로 호출하고 `Response` 를 검증. 인증 401/403, 404, 413, 429, envelope 유무, raw 응답 형태 |
| 컴포넌트 | `bun test` + Testing Library + happy-dom | features 순수 UI 렌더·상호작용(`tests/components/`). 서버 상태는 목을 쓰지 않고 순수 함수·계약 테스트로 확인한다. `msw` 는 devDependency 로 설치만 돼 있고 쓰는 테스트가 없다(KNOWN ISSUE: 쓰지 않으면 `bun remove msw`) |
| DB 질의 | `bun test`(임시 MySQL) | `tests/db/` 의 `scratch-auto-ranking`(순위 질의 `best-scope.ts`), `best-race`(동시 제출의 최고 기록), `best-repair`(재전송의 최고 기록 복구), `lock-retry`(교착 재시도·잠금 대기 5초), `submission-integrity`(본문 검사·거절 시 차트 행 안 남김), `write-races`(설정 낙관적 잠금·폴더 동시 쓰기와 한 요청 저장·토큰 폐기·리플레이 저장량), `admin-column-bounds`, `chart-notes-scripts`. `tests/support/folder-memory-db.ts`(폴더 서비스용 메모리 DB) |
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

## 11. 곡별 차트 패턴과 결정론 분석 (W3, 2026-10-20)

근거: [기획](../plan/2026-10-19-web-ai-features.md) 3.3·3.4·3.5·4절, [W0 ADR](../acknowledge/2026-10-19-adr-web-ai-decisions.md)(기획보다 우선: Q8 A, Q9 B 7K 전체, Q17 gzip).

### 11-1. 저장과 접근

| 항목 | 내용 |
|---|---|
| 형식 | `rbms-cli chart-pattern` 이 쓰는 `<md5>.json`(`apps/rbms-cli/src/chart_pattern/document.rs` 가 정본). Zod: `server/dto/chart-pattern.ts` `chartPatternDocumentSchema`. `format`·`version` 1·`mode` `BEAT_7K` 고정, 레인 0~6 = 건반 1~7, 7 = 스크래치, `kind` 0 일반·1 LN 머리·2 LN 끝·3 지뢰. `play_level` null(정수 레벨이 없는 헤더)·`total` 0 을 받는다. 네 열 길이가 같아야 하고 `(time_us, lane)` 오름차순(같은 쌍 반복은 허용), `tempo` 는 `t_us` 오름차순. 열 검사는 많은 노트가 틀려도 첫 위치 한 개만 보고한다(오류 본문이 커지지 않게). 상한: `time_us`·`t_us` 는 24시간(`CHART_PATTERN_MAX_TIME_US` = 86,400,000,000 µs)까지, 노트는 열마다 200,000개(`CHART_PATTERN_MAX_NOTES`, 측정한 가장 큰 곡이 약 64,000개)까지. 밀도 창 배열은 시각에 비례하므로 이 상한이 길이를 `MAX_DENSITY_WINDOWS`(86,401)로 묶는다 |
| 범위 | 7K 전체(ADR Q9 B). 발광 표 밖 md5 도 받는다(기획 3.3 의 "표 밖 md5 400"은 폐지) |
| 표 | `chart_pattern`(PK `md5`, `sha256` 색인, `mode`, `format_version`, `parser_version`, `playable_notes`, `note_count`, `content_hash`, `byte_size`, `data` mediumblob, `features` json, `features_version`, `uploaded_by`, `created_at`, `updated_at`). Drizzle 0.45 mysql-core 에 blob 열이 없어 `customType` 으로 `mediumblob` 을 선언했다(`db/schema.ts`) |
| 압축 | `data` = 정규화한 JSON(Zod 가 돌려준 객체의 `JSON.stringify`, 스키마 순서·모르는 필드 제거) 의 gzip(`node:zlib`, `service/shared/storage/gzip-json.ts`). `byte_size` 는 압축 전 UTF-8 바이트, `content_hash` 는 압축 전 텍스트의 SHA-256 |
| 멱등 | 같은 md5 + 같은 `parser_version` + 같은 `content_hash` → `unchanged`(200, 아무것도 바꾸지 않음. 단 `features_version` 이 낮으면 특징만 다시 계산). 그 밖의 차이(`parser_version` 이 다르거나, 같아도 내용이 다르면) → `replaced`(200, 행 전체와 특징을 다시 씀). 처음 → `created`(201). 동시에 처음 올린 같은 md5 는 `ER_DUP_ENTRY` 를 잡아 다시 비교하므로 한 건만 `created` 이다 |
| 검증 실패 | 형식 오류 400 `VALIDATION_ERROR`(details.field 예 `notes.lane.0`), 경로 md5 와 본문 md5 가 다르면 400 `IR_CHART_PATTERN_INVALID` `{ field: "md5", reason: "md5_mismatch" }`, 4 MiB 초과 413 |
| coverage | `GET /api/admin/chart-patterns/coverage` → `{ tables: [{ table_id, name, table_md5s, covered, missing: md5[] }], patterns_total, stale_features, analysis_version }`. 표는 `data/tables` 스냅샷(지금 `insane1` 1,035곡) |
| 일괄 재계산 | `POST /api/admin/chart-patterns/recompute` 본문 `{ limit?, after? }`(`limit` 1~`CHART_PATTERN_RECOMPUTE_BATCH.MAX` 200, 기본 50 / `after` = 앞 배치가 끝난 md5, 처음은 생략·null) → `{ analysis_version, examined, recomputed, skipped: md5[], next_after, remaining }`. 분석 버전이 낮은 곡을 md5 순서로 `limit` 곡만 하나씩 다시 계산한다(가장 큰 곡도 약 60 ms). 읽을 수 없는 배치는 `skipped` 에 적고 그대로 둔다. `next_after` 가 null 이 될 때까지 받은 값을 `after` 로 넣어 부르면 끝나고, 못 읽는 곡이 있어도 같은 곡을 되풀이하지 않는다. 본문은 `{}` 도 된다 |
| 읽기 경로 없음 | 노트 배치를 돌려주는 라우트가 없다. `chartPatternService.readPattern`·`readFeatures` 는 서버 분석 서비스 전용이고 라우트 의존 타입은 `Pick<ChartPatternService, 'upload' \| 'coverage' \| 'recomputeStale'>` 다. `tests/route/chart-pattern-admin.test.ts` 가 (1) 패턴 라우트 파일이 PUT·GET·POST 하나씩뿐임 (2) `chart_pattern` 표를 읽는 곳이 `compose/chart-pattern.ts` 하나뿐임 (3) 저장된 배치를 읽는 함수가 서비스·compose 밖에서 불리지 않음을 고정한다 |
| 로그 | 패턴 라우트의 예상 밖 오류는 클래스 이름과 드라이버 코드만 남긴다(`route/admin.ts` `withSummarizedErrors`). DB 오류 메시지에는 바인딩 값(노트 배치)이 들어 있기 때문이다. AI 라우트의 `withSafeAiErrors`(`route/ai.ts`)와 같은 일을 하므로 하나로 합칠 후보다 |

### 11-2. 업로드 스크립트

`bun scripts/upload-chart-patterns.ts <패턴 폴더> [--server <주소>] [--token-env <이름>] [--dry-run]`(`web/` 에서 실행).

- 폴더의 `<md5>.json` 만 이름 순서로 올린다. 읽지 못한 파일은 그 파일만 실패("파일을 읽지 못했습니다")로 남기고 나머지를 계속 올린다. `_` 로 시작하는 파일(`_coverage.json`)과 다른 이름은 건너뛴다. 보내기 전에 서버와 같은 Zod 스키마로 검사하고, 파일 이름의 md5 와 본문 md5 가 다르면 보내지 않고 실패로 남긴다. `--dry-run` 은 검사만 한다.
- 동시 요청은 `UPLOAD_CONCURRENCY`(4). 201 `created`, 200 `replaced`·`unchanged` 를 따로 세고, 실패 목록(md5, HTTP 상태, 오류 코드·필드·이유)과 요약을 출력한다. 실패가 있으면 종료 코드 1. 401·403 이 오면 나머지는 같은 이유로 실패하므로 멈춘다. 실패한 파일을 다시 보내는 재시도는 없다: 다시 돌리면 이미 올라간 것은 200 `unchanged` 라 빠진 것만 다시 올라간다.
- 서버 한도(분당 1,200)를 지킨다. 워커 전체가 요청 시작 사이를 `UPLOAD_REQUEST_INTERVAL_MS`(= ceil(60,000 / (1,200 × `UPLOAD_RATE_SHARE` 0.9)) = 56 ms, 분당 약 1,071건) 이상 벌린다. 7,903곡이면 약 7.4분이다. 429 를 받으면 실패로 세지 않고 레이트리밋을 따른다: `Retry-After`(초 또는 HTTP 날짜, 없으면 서버 창 길이 `RATE_LIMIT_FALLBACK_PAUSE_MS` 60초, 0 이나 지난 날짜면 최소 `RATE_LIMIT_MIN_PAUSE_MS` 1초, 한 번에 최대 `RATE_LIMIT_MAX_PAUSE_MS` 300초)만큼 모든 워커가 멈췄다가 같은 파일을 다시 보낸다. 다시 보내기 전에 429 응답 본문을 읽어 버린다. 요청이 날아가던 중에 같은 때 429 를 받은 워커들은 대기 한 번으로 센다. 요청이 한 번이라도 통과하면 연속 횟수를 0 으로 되돌리고, 통과 없이 `MAX_CONSECUTIVE_RATE_LIMIT_PAUSES`(5)번 기다린 뒤에도 429 이면 그 파일을 429 실패로 보고하고 나머지를 멈춘다(요약에 "속도 제한이 이어져 나머지를 멈춤").
- 서버 기본값 `https://bms.hyuns.uk`. `--server` 는 https 이거나 이 컴퓨터의 http(`localhost`·`127.0.0.1`·`[::1]`)여야 한다(관리자 토큰이 평문으로 나가지 않게).
- 토큰: 기본은 실행 중에 `security find-generic-password -a rbms-admin -s rbms-pattern-upload -w` 를 `Bun.spawn` 으로 불러 읽는다(stderr 는 버림). `--token-env NAME` 이면 그 환경변수. 값은 Authorization 헤더에만 쓰고 출력·로그·파일에 남기지 않는다.
- 검증은 실제 서버·DB 없이 실제 라우트 핸들러와 메모리 표로 한다(`tests/scripts/upload-chart-patterns.test.ts`). 한도 준수와 429 대기는 실제 시간을 기다리지 않는 가상 시계(`tests/support/virtual-clock.ts`)로 7,903곡 업로드를 서버의 고정 창 카운터(`consumeRateLimit`)에 대어 보고, 429 가 한 번도 나지 않고 어떤 60초 창에서도 1,200건을 넘지 않음을 확인한다(`tests/scripts/upload-rate-limit.test.ts`). 운영 업로드는 배포·마이그레이션 뒤 메인이 한다.

### 11-3. 결정론 분석 엔진(`service/domain/analysis/`, 순수 함수)

DB·HTTP·시계를 모르고 같은 입력에 같은 출력을 낸다. 모든 임계값은 `analysis-constants.ts` 한 파일의 이름 있는 상수이고, 바꾸면 `ANALYSIS_VERSION`(지금 1)을 올린다. 저장된 특징의 버전이 낮으면 `readFeatures` 가 읽을 때 다시 계산해 저장하고, 같은 내용을 다시 올려도 특징만 다시 계산한다. 여러 곡을 읽는 `readFeaturesMany` 는 요청 한 번에 `CHART_PATTERN_READ_RECOMPUTE_MAX`(10)곡까지만 다시 계산하고(§12-4), 나머지는 관리자 일괄 재계산(§11-1 `recompute`)이 따라잡는다.

| 파일 | 내용 |
|---|---|
| `pattern-tags.ts` | 기획 4.2 패턴 분류 v1 `classifyPattern`: 입력 노트마다 태그 목록(입력 순서). 태그 19개 `PATTERN_TAGS`. 밀도 창 배열(`densityWindowCounts`)은 `MAX_DENSITY_WINDOWS` 를 넘지 않고 24시간을 넘는 시각은 마지막 창에 센다(`densityWindowOf`). 간격은 뒤 노트 시각의 국소 16분(`15,000,000 / bpm` µs)의 배수로 재고 µs 반올림을 견디게 `GAP_TOLERANCE_RATIO`(1%)를 더한다. BPM 이 0 이하인 곳은 `init_bpm`, 그것도 0 이하면 `FALLBACK_BPM` |
| `chart-features.ts` | 곡별 특징 `computeChartFeatures`(→ `chart_pattern.features`): 판정 노트 수, 태그별 노트 수·비율, 1초 창 밀도(최고·평균·후반 1/3 평균), 소프란 횟수·STOP 수·BPM 최소/최대 비, LN·스크래치 비율, 지뢰 수, 길이. `layout: "original"` 과 `layout_dependent_tags`(레인을 읽는 태그) 를 함께 담아 원본 배치(옵션 OFF) 기준임을 표시한다(기획 4.5) |
| `play-metrics.ts` | 기획 4.3 판 지표 `computePlayMetrics`(W4 trace 가 넣을 입력 모양 `JudgedPlayNote`, `layout: "played"`): 전체·태그별·레인별·밀도 칸별 판정과 EX 비율, FAST/SLOW(등급별, `err_us >= 0` 이 빠름), 32 구간 BP(BD + POOR + 空POOR)·콤보 끊김, PG·GR 오차 평균·중앙값·표준편차·IQR·1ms 히스토그램(±150ms, 0 쪽으로 버림 = `TimingDistribution`), 앞·뒤 1/3 EX 비율 차(`fatigue`; 판정이 하나도 없는 구간 = 노트가 없거나 판이 거기까지 가지 못한 구간은 비율 0 이 아니라 null 이고 그러면 `delta` 도 null 이라 비교에서 빠진다). `findGaugeDrops`(4초 창 안 범위의 15% 이상 하락), `summarizeGaugeCurves`(게이지마다 끝값·클리어·바닥 닿음, 서버는 게이지 규칙을 계산하지 않는다. 클리어: 생존 게이지(최소 0·border 0 = HARD·EXHARD·HAZARD·CLASS 류, `gauge_tables.rs`)는 끝값이 0 보다 크면, 그 밖은 끝값이 border 이상이면. 곡선에 `failedAtMs`(trace 의 `failed_at_ms`)가 있으면 그것이 먼저라 실패), `describeGreenNumber`(점수 행의 레인 통과 ms 에서 GREEN = ms × 0.6 을 어림하고 ms 병기, `is_estimated: true`, ADR Q11)·`readTraceGreenNumber`(trace `speed.end` 의 `green`·`green_ms` 를 다시 계산하지 않고 그대로, `is_estimated: false`) |
| `sample.ts` | 기획 4.4·4.6: 판 내부 잔차의 노트 가중 평균, 95% 윌슨 구간, 등급 `strong`·`uncertain`·`insufficient`(`MIN_TAG_NOTES` 300, `MIN_TAG_PLAYS` 5)과 `weakness`·`strength`, SCRATCH AUTO 판은 스크래치 태그 표본에서 빼고 `auto_plays` 로 센다, 오프셋 제안 표본(`MIN_TIMING_PLAYS` 10, `MIN_TIMING_NOTES` 2,000) |

기획 4.2 문장을 코드로 옮기며 정한 해석(골든 테스트 `tests/service/analysis/pattern-tags.test.ts` 가 고정, 메인 확인 대상):

1. 행·동시치기·트릴·계단·연타·연속 난타는 "치는 노트"(일반 노트와 LN 머리)로 만든다. 판정되는 CN·HCN 끝은 `ln_end` 와 시간 창 태그(`soflan`·`dense`·`mine_near`)만 받는다. 판정되지 않는 LN 끝과 지뢰는 태그가 없다. 곡별 패턴에서 LN 끝은 `declared_ln` 이 `cn`·`hcn` 일 때만 판정 노트로 센다(선언 없음은 기본 LN 으로 본다).
2. `chord_k` 는 그 행의 건반 노트에만 붙는다. 같은 행의 스크래치는 `scratch_chord` 다.
3. 연타(`jack`)는 같은 레인의 두 노트 사이에 다른 건반 행이 없을 때만 잇는다. 기획 문장 그대로("같은 레인 연속 간격")면 16분 트릴의 각 레인이 8분 연타로도 잡혀 트릴 곡이 연타 곡으로 보이기 때문이다. 동시치기 행이 같은 레인을 이어 받는 연타(코드 연타)는 잡힌다.
4. 트릴·계단은 "건반 하나만 있는 행"이 이어질 때이고, 스크래치만 있는 행은 끼어도 끊지 않는다. 깨진 자리의 앞 행부터 새 트릴·계단이 시작될 수 있다(예: 1-2-3-4-3-2-1 은 계단 둘).
5. 고밀도(`dense`)는 창의 수가 90 분위 이상이면서 중앙값보다 클 때다(`DENSE_BASELINE_PERCENTILE`). 90 분위만 쓰면 밀도가 고른 차트에서 모든 노트가 고밀도가 된다.
6. `mine_near` 는 기획 문장대로 레인을 가리지 않는다(어느 레인의 지뢰든 ±16분).
7. `ln_release_then_hit` 의 손은 기본 배정(`SP7_LANE_HANDS`: S·1·2·3 왼손, 4 공유, 5·6·7 오른손)이고 공유 레인은 양쪽과 같은 손으로 본다. 화면에는 "기본 손 배정 기준"이라고 적는다(기획 4.4).
8. 소프란 구간은 BPM 이 직전 대비 1.25배 이상 바뀐 지점부터 2초, 100ms 이상 STOP 지점부터 STOP 길이 + 2초, `scroll ≠ 1` 인 동안이다. 0 이하 BPM 으로 바뀌면 값이 다르기만 해도 소프란이다.

성능: 분류는 노트 수에 거의 비례한다(LN 은 누르는 중인 LN 만 들고 다니는 쓸기, 소프란 구간은 합친 뒤 이진 탐색). 이 때문에 분석 모듈 안의 지역 배열은 `push` 로 쌓는다(공통 컨벤션의 비파괴 연산 예외, 대용량 처리 사유). 7만 노트 패턴의 업로드·특징 계산·저장이 로컬 DB 통합 테스트에서 1초 안에 끝났다.

## 12. 점수 기반 AI 기능 (W2, 2026-10-20)

기획 [2026-10-19-web-ai-features.md](../plan/2026-10-19-web-ai-features.md) 5.4(점수 행 부분)·5.5·5.6·5.7·5.10. 모두 점수 행·발광 표 기록·곡별 특징(§11)만 읽고 판별 trace(W4)는 쓰지 않는다.

### 12-1. 공통 흐름

| 단계 | 어디 | 내용 |
| --- | --- | --- |
| 결정론 digest | `service/domain/ai/{recommend,practice-folder,goal-plan,weekly-summary,judge-habit}.service.ts`, 공통 `table-progress.ts`·`ai-digest.ts`, 상수 `score-feature.constants.ts` | 숫자는 서버가 계산한다. 항목마다 `key` 가 있고 AI 출력의 `key`·`focus_key`·`check_key` 는 digest 안의 키여야 한다(`collectDigestKeys`) |
| 보기(GET) | `route/ai.ts` `createAiScoreFeatureRoutes` | digest 와 그 기능의 마지막 저장 결과(`readStoredInterpretation`)를 함께 준다. 공급자를 부르지 않는다. `is_current` 는 저장 결과의 `input_hash` 가 지금 digest·프롬프트·스키마와 그 결과의 공급자·모델로 다시 계산한 값과 같은지다(`hashAiRunInput`, `feature-runner.service.ts`) |
| AI 해석(POST) | 같은 라우트 | 사용자가 버튼을 누를 때만. W1 `aiFeatureRunner.run` 한 번(한도·캐시·사용량·자동 재시도 없음). 표본이 모자라면 부르지 않고 422 `AI_NOT_ENOUGH_DATA` + `details.need` |
| 출력 검증 | 기능별 `refine*Output` | 후보 밖 md5·모르는 키·한 줄에 안 맞는 항목은 버린다(자르지 않는다). 요약 문장만 자른다. 남는 항목이 없으면 `AI_OUTPUT_INVALID` |
| 주입 방어 | 프롬프트 공통 문장 `DIGEST_IS_DATA_RULE`·`DIGEST_KEY_RULE` | 곡 제목·아티스트는 digest JSON 의 문자열로만 들어간다. 화면은 React 기본 이스케이프로만 그린다 |

응답 헤더는 성공 응답 모두 `Cache-Control: private, no-store`(W1 `aiPrivateJson`). 오류 응답은 W1 과 같이 `withErrorHandling` 이 만든다.

### 12-2. 기능별 결정

| 기능 | 경로 | 결정론 | 캐시 주제(`subject_key`) |
| --- | --- | --- | --- |
| 5 다음 도전 곡 | `GET/POST /api/fe/me/ai/recommend` (`table`) | 경계 레벨 = 클리어 비율 0.5 이상인 가장 높은 레벨의 다음(없으면 첫 레벨, 전부면 마지막). 후보 = 경계 ±1 레벨의 실패·미플레이·HARD 아래 클리어, 점수 = 종류 가중(실패 3·미플레이 2·램프 갱신 1) − 1.5 × 레벨 거리 + 실패 판의 EX 비율. 상위 40곡, 곡별 패턴이 있으면 상위 태그 3개(원본 배치) | `table:<id>` |
| 6 AI 연습 폴더 | `GET /practice-folder`(`source`=recommend·tag·goal), `POST /practice-folder/preview`(AI), `POST /practice-folder`(만들기) | 후보 최대 60곡(추천 순위, 태그 비율 0.1 이상 경계 −2~+1 레벨, 목표의 남은 곡). 폴더 50개면 미리보기도 하지 않는다 | `src:<source>:<table 또는 goal>:<tag>` |
| 7 목표 계획 | `GET/POST /goals`, `DELETE /goals/[goalId]`(보관), `GET/POST /goals/[goalId]/plan` | 목표 종류는 `lamp_count_at_level`·`clear_all_level`·`ex_rate_on_chart` 셋(자유 문장 없음, `tag_residual` 은 trace 가 있는 W5 에서). 진행도·남은 곡(가까운 순)·최근 28일 갱신 곡 수. 활성 목표 10개 | `goal:<id>` |
| 10 주간 요약 | `GET/POST /weekly` (`week`=월요일 `YYYY-MM-DD`, UTC) | ISO 주 [월 00:00 UTC, 다음 월 00:00). 그 주 전 최고(같은 SCRATCH AUTO 여부)와 비교한 램프 갱신·첫 클리어·EX 갱신·처음 친 곡, FAST/SLOW, 규칙 세대 4 판의 치우침, 앞 주 값. 열 때만 만든다(자동 생성 없음) | `week:<월요일>` |
| 4 판정 습관(점수 행) | `GET/POST /judge-habit` | 최근 100판(오토플레이 제외). FAST 비율의 윌슨 95% 구간, 등급별 빠름/늦음, `extra.mean_offset_us`(세대 4, AUTO ADJUST 판 제외) 가중 평균, 오프셋 제안(10판·PG+GR 2,000노트·FAST 비율 strong·같은 쪽일 때만, 빠르면 내리고 늦으면 올림), avgjudge 앞·뒤 절반, 녹색 숫자 GREEN + ms 병기(점수 행만 있으면 레인 ms × 0.6 을 어림한 값이라 `is_estimated: true` 이고 화면에 "추정", 판에 trace 의 green 값(`traceGreen`)이 있으면 그 값을 그대로 쓰고 추정이 아니다. 가운데값은 하나라도 추정이면 추정), 50 단위 칸 | `recent` |

연습 폴더 만들기: 이름 `AI 연습 · <이름>`, 겹치면 끝에 오늘 날짜(UTC). 같은 미리보기(출처·이름·설명·곡과 메모의 해시)로 다시 누르면 `ai_practice_folder` 연결로 같은 폴더의 곡을 바꾼다. 곡은 그때 다시 계산한 후보 안에서만 남는다. 폴더가 지워지면 연결 행도 FK cascade 로 지워진다. 같은 미리보기를 동시에 누르는 경합: 폴더를 만든 직후 PK `(user_id, preview_hash)` 로 연결을 잡고(`claimLink`, `ER_DUP_ENTRY` 를 잡아 먼저 잡은 폴더 id 를 돌려준다), 진 요청은 자기가 만든 폴더를 지우고 이긴 폴더의 곡을 바꾼다(`updated`). 진 요청이 폴더 한도·이름 겹침으로 폴더를 못 만들었을 때도 연결을 다시 읽어 이긴 폴더를 갱신한다. 연결을 곡 채우기보다 먼저 잡으므로 곡 채우기가 실패해도 다시 누르면 같은 폴더를 채운다. 새 폴더는 잠깐(수 ms) 둘이 보일 수 있다.

### 12-3. 표(마이그레이션 0008_ai_goal, 추가만)

- `ai_goal`(PK `id` `aig_…`; `user_id` FK cascade, `kind`, `params` json(종류를 뺀 나머지), `created_at`, `archived_at`; 색인 `(user_id, archived_at)`)
- `ai_practice_folder`(PK `(user_id, preview_hash)`; `folder_id` FK `user_folder.id` cascade, `created_at`)

운영 적용은 §3-2 의 두 번 푸시 순서를 따른다.

### 12-4. 곡별 특징 읽기

`chartPatternService.readFeaturesMany(md5s)`(§11 서비스에 추가)가 배치 없이 저장된 특징만 읽는다. GET 한 번이 분석 버전이 낮은 특징을 한꺼번에 다시 계산하지 않도록, 요청한 순서로 앞에서부터 `CHART_PATTERN_READ_RECOMPUTE_MAX`(10)곡만 `readFeatures` 로 다시 계산해 저장하고 나머지는 옛 특징을 그대로 쓰며 `isStale: true` 로 돌려준다(옛 특징이 태그 비율로 읽히지 않으면 그 곡은 뺀다). 태그 불러오기(`createChartTagLoader`)는 `{ shares, staleMd5s }` 를 돌려주고, 다음 도전 곡 보기는 `pattern_coverage.stale_features` 로 옛 특징을 쓴 후보 수를 알리며 화면이 "옛 분석 버전 N곡"을 적는다. 남은 곡은 관리자 일괄 재계산(§11-1)으로 따라잡는다. `chart_pattern` 표를 읽는 compose 는 여전히 `compose/chart-pattern.ts` 하나다.

### 12-5. 화면

`/ai` 허브는 탭(`?tab=query|recommend|goals|weekly|habit`, `widgets/ai/ai-hub-tabs.tsx`)이다. 기록 질문만 키 게이트 안에 있고, 나머지 탭은 키가 없어도 결정론 숫자를 보이며 AI 버튼을 누르면 실패 안내(설정 링크)를 보인다. 공통 조각은 `features/ai/{ai-interpret-section,ai-interpretation-meta,ai-insight-list,ai-data-notice,ai-chart-candidate-list,ai-goal-form}.tsx`, 쿼리는 `entities/ai.query.ts`(`QUERY_KEY.AI.DIGEST(feature, params)`, `QUERY_KEY.AI.GOALS`; AI 호출 뒤 그 기능의 digest 와 사용량을 무효화).

## 13. 판별 trace 와 판별 분석 (W4 웹, 2026-10-20)

근거: [기획](../plan/2026-10-19-web-ai-features.md) 3.1·3.2·3.4·3.5·4.4·4.5·8절 W4 행, [W0 ADR](../acknowledge/2026-10-19-adr-web-ai-decisions.md)(Q3 본인만, Q6 게이지 9개, Q17 gzip·거절 없는 보관 상한, 구현 중 메인 결정의 9키 규칙). 형식의 정본은 Rust 구현(`crates/rbms-ir/src/dto/trace.rs` `PlayTrace`, `apps/rbms-player/src/play_trace.rs`)과 계약 파일 `contract/client-requests/trace-{normal,random,scratch-auto,abandoned}.json` 이다.

### 13-1. 업로드 `POST /api/charts/[hash]/traces`

| 항목 | 내용 |
|---|---|
| 능력 플래그 | `GET /api/health` 의 `capabilities.traces: true`. 클라이언트는 이 값이 참일 때만 보낸다(`record_sync.rs` `takes_traces`) |
| 순서 | 본문 크기(2 MiB, `content-length` 와 실제 스트림) → Bearer 유무 → 토큰 사용자 → 30/min/사용자 → 경로 해시 꼴(md5·sha256 아니면 404 `IR_CHART_NOT_FOUND`) → Zod(`server/dto/trace.ts` `traceUploadSchema`) → 서비스 |
| 형식 검사 | `format` `"rbms-trace"`·`version` 1, `score_id` 필수, `chart.sha256` 64자 16진(`md5` 는 32자 또는 빈 값), 노트 다섯 열·空POOR 두 열의 길이가 같음, `time_us` 0~24시간, `lane` 0~63, `kind` 0~3, `state` 0~5, 정수 칸은 안전한 정수, 속도 변경·tempo 각 4,096개 이하(`TRACE_MAX_SPEED_CHANGES`·`TRACE_MAX_TEMPO_POINTS`, 클라이언트 상한과 같다), `speed.changes[].field` 는 `TRACE_SPEED_FIELDS` 아홉 이름. 노트 순서는 강제하지 않고 분석이 정렬한다 |
| 소유 | `score_id` 의 점수 행이 토큰 사용자의 것이어야 한다. 남의 점수와 없는 점수는 똑같이 404 `IR_SCORE_NOT_FOUND`(`details.field: "score_id"`) |
| 멱등 | 점수 하나에 trace 하나(`play_trace.score_id` 고유). 이미 있으면 본문과 관계없이 그 id 를 200 으로 돌려준다. 동시에 들어온 같은 점수는 `ER_DUP_ENTRY` 를 잡아 이긴 쪽 id 를 200 으로 돌려준다 |
| 일관성 검사 | 서버는 재판정하지 않고 점수 행과 허용 오차 없이 맞춘다(`service/domain/trace/trace-check.ts` `findTraceMismatch`). ① `chart.sha256` = 점수의 `chart_sha256`, 경로 해시 = 점수의 sha256 또는 md5(점수에 md5 가 없으면 본문 md5) ② state 1~5 개수 = `pgreat`·`great`·`good`·`bad`·`poor` ③ state 1~4 의 `err_us >= 0` 개수와 `< 0` 개수 = `epg/lpg`…`ebd/lbd` ④ `empty_poors` 개수 = `empty_poor`. 판정 칸보다 먼저 판 조건이 같아야 한다: `mode`·`lntype`·`random`(점수의 P1 옵션)이 그대로 같고, `auto_lanes` 에 켜진 레인이 있는지 = 점수의 `scratch_auto`(어긋나면 `field` 가 `mode`·`lntype`·`random`·`auto_lanes`). 9키 예외는 점수 행의 모드를 따르고 trace 모드가 다르면 그 전에 `mode` 로 어긋난다. POOR 쪽: `failed_at_ms` 가 null 인 판(끝까지 친 판, Esc·종료로 남긴 판)은 도달하지 못한 판정 객체(state 0, 일반 LN 끝 kind 2 제외)를 POOR 로 더한다(`PlaySummary::left_unfinished` 와 같음, `trace-abandoned.json` 이 고정). 게이지가 빈 판(`failed_at_ms` 숫자)은 더하지 않는다(점수가 판정된 노트만 센다). 9키(`POPN_9K`, `badTakesNote` 가 false 인 모드)는 BAD·POOR·`ebd`·`lbd` 를 비교하지 않는다. 어긋나면 400 `IR_TRACE_MISMATCH` `{ field: "judge.poor" 등, reason: "score 25, trace 0" }` |
| 저장 | `play_trace`: 정규화한 JSON(Zod 결과의 `JSON.stringify`)의 gzip 을 `data` mediumblob 에, `byte_size`(압축 전 바이트)·`stored_size`(압축 뒤)·`note_count`·`format_version`·점수의 `played_at`·`chart_sha256`·`chart_md5`. 6만 노트 trace(약 1.4 MB)를 Docker MySQL 8.4.11 에 저장·재분석하는 DB 테스트가 있다 |
| 응답 | 201 `{ id: "tr_…", analysis: "stored" \| "pending", storage }`, 중복 200 `{ id, storage }`. 클라이언트는 `id` 만 읽는다(`TraceUploadResponse`) |
| 보관 상한 | ADR Q17: 사용자당 5,000판·512 MiB(`TRACE_USER_QUOTA`, 바이트는 압축 전 `byte_size` 합). **넘어도 거절하지 않는다.** `storage = { traces, bytes, limit_traces, limit_bytes, level }`, `level` 은 어느 한쪽이 90%(`NEAR_RATIO`) 이상이면 `near`, 넘으면 `over`. 같은 값을 `GET /api/fe/me/traces/usage`(세션)가 화면 경고용으로 돌려주고, `GET /api/admin/traces/usage`(관리자 Bearer)가 `{ limits, totals: { users, traces, bytes, stored_bytes }, accounts_near, accounts_over, top: [{ login_id, traces, bytes, stored_bytes, level }](바이트 상위 20), analysis: { analysis_version, missing, stale } }` 를 돌려준다 |
| 로그 | 세 라우트(업로드·본인 보관량·관리자 수치)와 W5 의 공유 설정·재계산 라우트의 예상 밖 오류는 클래스 이름과 드라이버 코드만 남긴다(`lib/with-summarized-errors.ts`, AI·패턴 라우트와 같은 HOF). DB 오류 메시지에는 바인딩 값(trace 본문)이 들어 있기 때문이다 |
| 읽기 경로 없음 | trace 를 돌려주는 라우트가 없다(Q3: 본인만, 화면은 분석만). `tests/route/trace.test.ts` 가 trace 라우트 파일이 업로드 POST·보관량 GET 둘·관리자 재계산 POST·공유 설정 GET/PUT 뿐임, `play_trace`·`play_analysis` 표를 쓰는 곳이 `compose/trace.ts` 뿐임, 저장된 trace 를 푸는 함수(`findStoredTrace`·`readAnalysis`)를 부르는 곳이 compose 와 서비스뿐임을 고정한다 |

### 13-2. 판별 분석 `play_analysis`

업로드가 저장한 직후 같은 요청에서 계산한다(`service/domain/trace/trace-analysis.ts` `analyzeTrace`, 순수 함수). 분석이 실패해도 trace 는 저장되고 응답은 `analysis: "pending"` 이며 오류는 클래스 이름과 드라이버 코드만 로그에 남긴다. `traceService.readAnalysis({ traceId, userId })` 는 본인 trace 만 읽고, 분석이 없거나 `analysis_version` 이 낮으면 저장된 trace 에서 다시 계산해 저장한다. 저장 버전은 `PLAY_ANALYSIS_VERSION`(지금 2, `analysis-constants.ts`)이고 곡별 특징의 `ANALYSIS_VERSION`(1)과 따로 오른다: 판 하나의 요약 모양만 바뀌면 7,903곡의 특징을 다시 계산하지 않는다. 요약에는 두 버전을 함께 저장하고(`analysis_version` 과 `pattern_analysis_version` = 만든 때의 `ANALYSIS_VERSION`) 둘 중 하나라도 이 빌드의 값과 다르면 읽을 때 다시 계산한다(W5 결정: 곡별 분류 임계값을 바꿔 `ANALYSIS_VERSION` 만 올려도 저장된 판 분석이 낡은 채 남지 않는다. 사람이 둘을 함께 올리는 것을 기억할 필요가 없다). 관리자 수치와 일괄 재계산의 SQL 은 `analysis_version` 열이 낮거나 `summary` 의 `pattern_analysis_version`(JSON 경로, 없으면 0)이 낮은 행을 뒤처진 것으로 센다. 버전 2(2026-10-20)는 구간별 EX·FAST/SLOW·태그, 레인별 오차, 게이지를 게이지 사망 시점까지만 읽는 규칙을 더했다. 낮은 버전 분석은 읽을 때 한 번에 `PLAY_ANALYSIS_READ_RECOMPUTE_MAX`(10)판까지 다시 계산하고, 나머지는 관리자 일괄 재계산 `POST /api/admin/traces/recompute`(관리자 Bearer, 본문 `{ limit?(1~200, 기본 50), after? }` → `{ analysis_version, examined, recomputed, skipped, next_after, remaining }`, 읽을 수 없는 trace 는 `skipped` 에 적고 넘어간다)가 따라잡는다.

다시 계산이 예외로 끝난 trace(풀 수 없는 gzip, 스키마를 통과하지 못함, 분석·패턴 읽기 오류)는 읽기 전체를 실패시키지 않고 그 trace 만 실패로 표시한다. `play_analysis` 행을 `pattern_join: "failed"`, `summary: { analysis_version, pattern_analysis_version, failed: true }`, `analysis_version` = 현재 값으로 쓰고(표시도 저장하지 못하면 로그만 남긴다), 오류는 클래스 이름과 드라이버 코드만 로그에 남긴다. 표시는 두 버전이 그대로인 동안 읽기에서도 일괄 재계산에서도 다시 계산하지 않고(`remaining` 에도 들지 않는다) 한쪽 버전이 오르면 한 번 다시 시도한다. 판을 모아 읽는 메서드는 `{ plays, stale, failed }` 를 돌려주고(`failed` = 실패해서 뺀 판 수), 한 판을 읽는 메서드와 `readAnalysis` 는 실패한 판을 없는 판처럼 null 로 답한다. 그래서 trace 하나가 깨져도 리포트·비교·게이지 조언의 GET 이 500 이 되지 않는다.

- 표: `play_analysis`(PK `trace_id` FK `play_trace.id` cascade, `analysis_version`, `summary` json, `pattern_join` `matched`·`mismatch`·`unsupported`·`absent`, `pattern_md5`, `computed_at`). `play_trace` 는 `user_id` FK cascade(사용자를 지우면 trace 와 분석이 같이 지워진다), 색인 `(user_id, played_at)`·`(chart_sha256, user_id)`. 마이그레이션 `0009_play_trace`(추가만).
- `summary`: `analysis_version`, `pattern_analysis_version`, `format_version`, `mode`, `random`, `lntype`, `scratch_auto`(자동 레인이 있었나), `run_end`(`finished`·`gauge_empty`·`left`), `metrics`(BEAT_7K 만 `computePlayMetrics`: 판정·태그·레인·밀도·구간 32개·FAST/SLOW·오차 분포·피로, 레이아웃은 플레이어가 본 배치 `played`. 도달하지 못한 노트는 넣지 않는다. 다른 모드는 null), `gauge`(`selected`, `interval_ms`, `failed_at_ms`, 친 게이지의 `drops`, 구간마다 끝의 친 게이지 값 `segment_values`, 9개 게이지 각각의 `end`·`cleared`·`reached_min`·`outcome`(`cleared`·`failed`·`unknown`). 규칙(`summarizeGaugeCurves`): 끝까지 친 판은 생존 게이지는 끝값 > 0, 그 밖은 끝값 ≥ border 면 통과. Esc·종료로 남긴 판(`left`)은 FAILED 로 기록되므로 9개 모두 `cleared: false`. 게이지 사망 판(`gauge_empty`)은 클라이언트가 실패 시점부터 9개 열을 0 으로 채우므로(`GaugeLogRecorder::closed`, rbms-play) 실패 시점 표본까지만 읽는다(`liveGaugeValues`): 친 게이지와 그때까지 이미 바닥에 닿은 생존 게이지는 `failed`, 나머지는 `unknown`(`cleared: null`, `end` 는 실패 시점 값). 실패 뒤 구간의 `segment_values` 는 null. Esc·종료로 나간 판(`left`)은 로그가 나간 시점에서 끊기므로 마지막 표본 시각(`(표본 수 − 1) × interval_ms`) 뒤에 끝나는 구간의 `segment_values` 도 null — 마지막 표본으로 채우지 않는다), `speed`(시작·끝 GREEN 과 레인 시간 — trace 값을 그대로 써서 `is_estimated: false`, 변경 횟수와 바뀐 칸 이름), `pattern`(`join`, `md5`).
- 패턴 정합(기획 3.4, 거절하지 않음): trace 의 `chart.md5` 로 곡별 패턴을 읽고(`chartPatternService.readPattern`, 서버 안에서만), 패턴의 sha256 이 점수의 sha256 과 같고 trace 의 모든 객체 시각 다중집합이 패턴의 지뢰를 뺀 노트 시각 다중집합(SCRATCH AUTO 레인 `auto_lanes` 의 원본 노트 제외)과 같으면 `matched` 이고 `pattern_md5` 를 남긴다. 다르면 `mismatch`(특징을 잇지 않음), 패턴이 없으면 `absent`. ALL-SCRATCH(건반 노트를 스크래치 레인으로 옮기는 유일한 옵션, `shuffle.rs`)와 SCRATCH AUTO 가 함께인 판은 자동으로 친 노트가 플레이 배치의 스크래치 레인 노트라 원본 레인으로 가릴 수 없다: trace 시각이 패턴 시각 안에 다 들면 `unsupported`(특징을 잇지 않음), 아니면 `mismatch`. 옵션은 시각을 바꾸지 않으므로 RANDOM 판도 맞는다. LN 끝은 양쪽에 다 있어 LN 종류와 무관하다.
- 업로드 뒤에 패턴이 올라온 판은 `absent` 로 남는다(분석 버전을 올리거나 다시 계산할 때 바뀐다).

## 14. 판별 기반 AI 기능 (W5, 2026-10-20)

근거: [기획](../plan/2026-10-19-web-ai-features.md) 4.4·4.6·5.1·5.2·5.3·5.4·5.8·7.4·7.6, [W0 ADR](../acknowledge/2026-10-19-adr-web-ai-decisions.md)(Q3 본인만, Q4 허용한 라이벌의 구간 집계 차이만, Q10 자동 재시도 없음, Q11 GREEN + ms). 공통 흐름은 §12-1 과 같다: GET 은 결정론 결과와 마지막 저장 해석만(공급자 호출 없음), POST 는 "AI 해석 만들기" 한 번(`aiFeatureRunner.run`, 한도·캐시·사용량, 자동 재시도 없음), 표본이 모자라면 부르지 않고 422 `AI_NOT_ENOUGH_DATA` + `details.need`, 출력의 `key`·`segment_id`·게이지·옵션은 digest 안의 것만 남긴다, 곡 제목은 digest JSON 의 문자열로만(프롬프트 공통 `DIGEST_IS_DATA_RULE`). 응답은 `private, no-store`.

### 14-1. 판별 분석 읽기 (trace 서비스만 표를 읽는다)

`play_trace`·`play_analysis` 표는 여전히 `compose/trace.ts` 만 읽는다(§13-1 의 고정 테스트). AI 서비스는 `TraceService` 의 메서드만 받는다: `recentAnalyzedPlays`(본인 최근 판, 오토플레이 제외, 최신순), `analyzedPlayOfScore`, `analyzedPlaysOfScores`, `bestAnalyzedPlayOnChart`(같은 차트·같은 SCRATCH AUTO 에서 EX 가 가장 높은 판, 한 점수 제외), `rivalSegmentsOnChart`. 모두 저장 분석이 현재 버전이 아니면 한 번에 10판까지 다시 계산하고 나머지는 빼서 `stale` 로 센다. 다시 계산에 실패한 판은 실패로 표시하고 `failed` 로 센 뒤 나머지로 답한다(§13-2).

`rivalSegmentsOnChart({ requesterId, rivalId, chartSha256, scratchAuto })` 는 요청자와 그 라이벌이 **서로** 상대를 라이벌 목록(`rival` 표, 양쪽 방향)에 두고 있고 그 라이벌이 `play_share_setting.rival_segments` 를 켰을 때만 `allowed: true` 이다(W5 결정: 라이벌 등록은 상대 동의 없이 되므로 한쪽 등록만으로 구간 데이터를 받게 하면 모르는 사람에게 열린다). 내보내는 것은 그 라이벌의 그 차트 최고 trace 판의 구간별 `id`·`chart_notes`·`notes`·`ex_score`·`bp` 와 판 결과(EX·램프·옵션)뿐이다. 노트·오차·레인·태그는 서비스 밖으로 나가지 않는다.

### 14-2. 라이벌 공유 설정 (마이그레이션 0010_play_share_setting, 추가만)

- 표 `play_share_setting`(PK `user_id` FK `user.id` cascade, `rival_segments` boolean 기본 false, `updated_at`). 행이 없으면 꺼짐. 다른 사람에게 보내는 것이라 옵트인이고, 받는 사람은 서로 라이벌인 플레이어뿐이다. 비교 화면의 라이벌 목록 `allows_segments` 도 "서로 라이벌이고 허용을 켬"일 때만 참이다(`compose/ai-play-features.ts` 의 `listRivals` 가 상대의 목록을 같은 SQL 에서 한 번 더 이어 본다).
- `GET/PUT /api/fe/me/traces/sharing`(세션, `{ rival_segments: boolean }`, PUT 은 분당 10). 화면: 설정 > 라이벌 탭의 "라이벌에게 구간 요약 허용" 스위치(`widgets/settings/rival-sharing-panel.tsx`, 문구는 "서로 라이벌인 플레이어에게만").
- 운영 적용은 §3-2 의 두 번 푸시 순서.

### 14-3. 기능별 결정

| 기능 | 경로(`/api/fe/me/ai/…`) | 결정론 | 캐시 주제 | AI 를 부르지 않는 경우 |
| --- | --- | --- | --- | --- |
| 1 플레이어 리포트 | `GET/POST /report` | 최근 100판(`REPORT_RECENT_PLAYS`)의 태그 잔차·등급(`aggregateTagEvidence`, 4.4·4.6), 레인별 EX·FAST 비율(친 배치), 밀도 칸별 EX, 피로 평균, PG·GR 오차 평균, 옵션 사용 비율, 선호 패턴(2판 이상 친 곡의 원본 배치 태그 비율 − 친 곡 전체 평균, 차이 0.01 이상 상위 5), 첫 발광 표 레벨별 램프, 점수 행 FAST/SLOW(판별 데이터가 없어도 보임). 표본은 7키 지표가 있는 판 수(`plays.measured`)로 센다. 다른 모드의 판은 `plays.analyzed` 에만 들어간다. AI 에는 강점·약점 각 5(등급 strong 만, 키 `tag.<태그>`), 불확실·표본 부족 태그는 이름만(가리킬 수 없게), 습관, 편차 큰 레인 3, 프런티어 근처 레벨. 프롬프트(버전 2)가 오차 `mean_ms` 의 부호(양수 = 빠름), 피로 `mean_delta`(뒤 1/3 − 앞 1/3), 레인 `deviation`(레인 EX − 전체 EX), `fast_share` 의 정의를 적는다 | `recent` | 7키 지표가 있는 판 5판 미만(`traced_plays`) |
| 2 리플레이 코치 | `GET/POST /plays/[scoreId]/coach` | 구간 32개(시각 범위, 판정 객체 수, EX 비율, BP, 콤보 끊김, FAST/SLOW, 많은 태그 3, 구간 끝 게이지), 꺾임, 9개 게이지 결과. 문제 구간 = BP 큰 순(같으면 앞) + BP 0 이지만 꺾임이 끝나는 구간, 최대 5 | `score:<id>` | trace 없음(`play_trace`), 7키 SP 아님(`play_metrics`) |
| 3 베스트·라이벌 비교 | `GET/POST /plays/[scoreId]/compare?against=best\|rival&rival=<login>` | 같은 차트를 같은 32 구간으로 자르므로 구간의 판정 객체 수가 모두 같을 때만 맞댄다(LN 종류·SCRATCH AUTO 가 다르면 `note_layout_differs`). 구간 EX 차·BP 차(이 판 − 비교 판)·누적, 차이 큰 구간 5. 한쪽 판이 도달하지 못한 구간(그 판에서 판정 객체가 모두 판정되지 않은 첫 구간부터 끝까지: Esc·게이지 사망 뒤)은 차이·누적이 null 이고 합계(`overall.ex_diff`, 도달한 구간 수 `overall.compared_segments`)·차이 큰 구간·AI digest 에서 빠진다. 맞댈 수 없으면 판정 합계 표(`mode: totals`; 내 다른 판은 그 판의 분석 합계, 라이벌은 공개 최고 기록 `chart_best`). 머리의 EX 와 표의 EX 는 항상 같은 판이다: 구간으로 맞대면 구간을 공유한 라이벌의 최고 trace 판, 합계로 내려가면 라이벌의 공개 최고 기록(두 판은 다를 수 있다). 라이벌이 목록에 없으면 `not_a_rival`, 서로 라이벌이 아니거나 허용 안 함 `rival_not_sharing`(어느 쪽인지는 알리지 않는다), 허용했지만 trace 없음 `rival_no_trace`, 공개 기록도 없어 맞댈 판이 없음 `rival_no_record`. 프롬프트 버전 2 | `cmp:<id>:<best\|rival>:<비교 점수 id>` | 구간 비교가 없거나 차이가 없음(`comparable_play`). AI digest 의 라이벌 쪽은 `{ key, who: "rival" }` 뿐 |
| 4 판정 습관(전체) | `GET/POST /judge-habit`(W2 경로) | §12-2 에 trace 부분 `trace_timing`: AUTO ADJUST 판을 뺀 PG·GR 오차의 노트 가중 평균, 판별 표준편차로 묶은 분산에서 95% 구간, 등급(10판·2,000노트 이상이고 구간이 0 을 벗어나면 strong), 전체보다 가장 치우친 레인 3. strong 이면 오프셋 제안이 trace 를 따른다(`offset.source: "trace"`), 아니면 점수 행 규칙(`"score"`). GREEN 은 trace 의 `speed.end` 를 그대로(추정 아님). 점수 행은 `compose/ai-score-features.ts` 가 읽고 trace 부분은 `traceService.analyzedPlaysOfScores` 로 붙인다. 프롬프트 버전 2 | `recent` | 최근 판 10판 미만 |
| 8 게이지·옵션 조언 | `GET/POST /gauge-advice` | 최근 100판의 친 게이지별 통과, 레벨·친 게이지별 실패 판과 그중 다른 게이지가 `cleared` 였던 수(`unknown` 은 세지 않음, 게이지 사망 판은 `unknown_runs`), 옵션별 판 수·EX 비율·레인 의존 태그 잔차 상위 3(그 옵션 3판 이상), 점수 행의 게이지·램프(판별 데이터가 없어도 보임). 게이지 이름은 `AI_GAUGE_NAMES`(`GAUGE_TYPES` 순서 = `GaugeIndex::ALL`), 옵션은 `AI_OPTION_NAMES` | `recent` | 분석된 판 5판 미만(`traced_plays`) |

검증(`refine*Output`): 리포트는 강점·약점 목록에 strong 목록 밖의 태그 키가 하나라도 있으면 출력을 버리고(기획 4.6), 불확실·표본 부족 태그가 있는데 `caveats` 가 비면 버린다. 코치·비교는 문제·차이 구간 밖의 `segment_id` 를 버리고 남은 것이 없으면 `AI_OUTPUT_INVALID`. 게이지 조언은 digest 에 나온 게이지·옵션만 남긴다.

### 14-4. 화면

- `/ai` 허브 탭에 `report`(플레이어 리포트, 주간 요약 옆)와 `gauge`(게이지·옵션) 추가. 판정 습관 탭은 노트 단위 오차·레인 치우침을 함께 보인다.
- `/ai/plays/[scoreId]`(세션 필요, `proxy.ts` 의 `/ai/:path*`): 코치(구간 BP 막대 + 게이지 선 `features/ai/ai-segment-chart.tsx`, 막대를 누르면 그 구간과 AI 진단)와 비교(비교 대상 고르기: 내 다른 최고 판 또는 라이벌, 라이벌 옆에 "구간 공유"·"합계만". 대상을 바꾸는 동안 같은 판의 이전 비교가 `placeholderData` 로 남아 라이벌 목록이 사라지지 않고 본문은 스켈레톤을 보인다. 도달하지 못해 뺀 구간 수를 한 줄로 알리고, 그런 구간이 있으면 머리의 차이도 "도달한 구간의 차이"로 도달한 구간의 EX 차이만 더한다).
- 플레이어 페이지의 플레이 기록 행은 세션 사용자의 페이지일 때만 "판 코치" 링크를 단다(`ScoreRow` 의 `action`). 남의 기록에는 없다.
- 쿼리: `QUERY_KEY.AI.DIGEST('play_report'|'replay_coach'|'play_compare'|'gauge_advice', params)`, `QUERY_KEY.AI.SHARING`. AI 호출 뒤 그 기능의 digest 와 사용량을 무효화, 공유 설정 저장 뒤 비교 digest 를 무효화.

## 15. 코스 카탈로그, 단위, 표 매일 갱신 (2026-10-23)

설계는 [../plan/2026-10-23-web-course-dan-tables.md](../plan/2026-10-23-web-course-dan-tables.md), 결정은 PROCESS "Stella·코스·단위와 클라이언트 보완" 의 12·13·14·18·19·20 번이다. 이 절은 코드(5a0eacf)가 정본이고, 설계서와 다른 곳은 설계서 각 절에 "구현과 다름" 으로 적었다.

### 15-1. 코스 제출의 게이지·스테이지·등급

- 본문의 새 선택 필드: `gauge`(코스가 끝난 게이지 `Class`·`ExClass`·`ExHardClass`), `course_gauge`(`LR2`·`BEATORAJA`·`IIDX`), `stages`(스테이지마다 `{ gauge, random, lntype }`, 최대 `COURSE_CHARTS_MAX` 64). 셋 다 없어도(`null`·빈 배열) 받는다. `lntype` 은 범위를 좁히지 않는다(0~2 밖의 값으로 판을 거절하지 않는다).
- 저장: `course_score.gauge`(게이지 번호 0~8, 없으면 NULL), `course_gauge`, `stages`(서버가 `stages[i]` 와 `charts[i]` 를 짝지어 `md5`·`sha256` 을 붙인 json). `course_best.assist_grade` 는 `deriveAssistGrade({ clear, gauge })` 로 채운다(LIGHT ASSIST 램프일 때만 값이 있다. 게이지가 없는 옛 본문은 0). 같은 판을 다시 보내는 경로도 처음 판의 저장된 게이지로 등급을 다시 만든다.
- 순위 의미가 바뀐다(결정 6의 직접 결과): SCRATCH AUTO 코스 클리어끼리의 순서가 게이지 등급을 따른다(그 전에는 모두 0).
- 새 클라이언트가 옛 서버에 보내면 세 키는 버려지고 400 은 없다(`z.object` 가 모르는 키를 버린다). 거꾸로, `contract/client-requests/course-*.json` 을 먼저 푸시하면 웹 `client-requests.test.ts`("파싱 결과가 입력과 같다")가 서버가 버린 키를 실패로 본다: 웹 DTO 가 같은 푸시이거나 먼저 나가야 한다(커밋 순서는 웹 DTO `db534d1` 다음 클라이언트 골든 `47e8f70`).

### 15-2. 코스 카탈로그

- 출처 모델: `course_source`(한 건) → `course_group`(화면 묶음) → `course_group_member`(순서·`active`). 같은 코스가 한 번만 `course`·`course_chart` 에 있고, 표에서 사라진 코스는 `active = false` 가 되며 기록·순위·단위는 남는다.
- 등록 순수 함수는 `service/domain/course-catalog/`: `courseHashOf`(이름 원문 + 차트마다 sha256 이 있으면 sha256 아니면 md5 + `\n`, SHA-256. 이름은 trim·정규화 없이 그대로), `canonicalConstraints`(14 토큰과 기준 구현 이름을 대소문자 무시로 읽고 모르는 것은 버리며 그룹마다 처음 것만 남김, Rust `Course::validate` 와 같은 결과), `bundledCatalog`(데이터 파일 `web/data/courses/genoside-2018/courses.json` → 카탈로그, 파일의 `course_hash` 가 계산값과 다르면 던진다), `tableCatalog`(표 헤더 `course` → 카탈로그: 배열의 배열·평평한 배열·객체 하나를 읽고, `md5`/`sha256` 문자열 배열 또는 `charts` 배열, 차트 제목은 표 항목에서). 코스 정의에 없는 sha256 은 채우지 않는다.
- 서비스 `courseCatalogService`: `planSync`(쓰지 않고 `inserted`·`changed`·`unchanged`·`deactivated`·`danCourses`·`newDanHashes` 를 센다)와 `applySync`(묶음 하나를 한 트랜잭션으로 등록, 잠금 대기 초과·교착은 `withLockRetry`), `listGroups`·`getEntry`(읽기, `course-catalog` 태그).
- 등록 수단은 둘: GENOSIDE 2018 은 `web-db` 의 `courses-seed-count`/`courses-seed`(`scripts/seed-courses.ts`), 표 코스는 매일 갱신(15-4)이 표 헤더가 바뀌었을 때 포함해서.
- 출처 표기(결정 14): 코스 목록·상세와 Stella·Satellite 표 페이지가 출처를 보인다. GENOSIDE: 제작 주체, 코스 파일 받은 날과 SHA-256 앞 12자, 공식 페이지 링크, 제약·트로피를 beatoraja 기본 난이도표에서 가져왔다는 둘째 줄, DP 코스의 제약·트로피는 같은 단 SP 값이라는 안내. 발광 insane1 은 다른 사이트의 흔적을 뺀 그대로다([ADR](../acknowledge/2026-10-14-adr-table-page-no-source-notice.md)).

### 15-3. 단위 판정

- 체계 넷(`DAN_SYSTEM_IDS`): `genoside2018_sp`, `genoside2018_dp`, `stella`, `satellite`. 대상은 묶음별로 정해진다: GENOSIDE 는 묶음 안 순서(첫 코스 rank 1)와 이름 접두사 뒤의 글자(`初段` … `Overjoy`)가 label, Stella·Satellite 는 이름 `Stella Skill Simulator <판> st<n>`·`Satellite Skill Analyzer <판> sl<n>` 의 `st<n>`·`sl<n>` 이 label 이고 rank 는 n. `STELLA技能訓練 …` 는 단위를 주지 않는다. 코드는 `service/domain/course-catalog/dan-of-course.ts`(`danOfCourse`)다.
- 판정 `judgeDanEligibility`(순수 함수, 2026-10-24 부터 어시스트 판을 거절하지 않고 A-단위로 가른다)는 성공이면 `{ eligible: true, kind: 'formal' | 'assisted', assistedBy, grant }`, 실패면 위에서 아래로 처음 걸리는 `reason` 을 낸다(저장하지 않음, 테스트·로그용). 순서: 단위 코스가 아님(`not_a_dan_course`), 게스트(`guest`), 순위 조건(`unranked`: 판이 `ranked` 이거나, `ASSIST` 플래그가 있고 **어시스트만 빼면 ranked 였을 판**이면 통과), 램프가 NO PLAY·FAILED 이거나 게이지 0 이하(`not_cleared`. 하한이 ASSIST 램프(id 2)로 내려가 ASSIST·LIGHT ASSIST 램프는 통과), `stages`·`course_gauge`·`gauge` 가 없음(옛 클라이언트, `no_stage_facts`), 코스 게이지가 아닌 스테이지, 스테이지 수 불일치, 코스 차트와 다른 곡, 그레이드 제약이 허용하지 않는 RANDOM(`grade` = OFF 만, `grade_mirror` = OFF·MIRROR), LN 제약과 다른 `lntype`(`ln`·`cn`·`hcn` = 0·1·2). 거절 사유는 열 개이고 전의 `scratch_auto`·`assisted` 는 없어졌다. 통과한 판의 종류: `assistedBy` 에 `scratch_auto`(SCRATCH AUTO 판), `assist_lamp`(램프가 EASY 보다 낮음), `assist_flag`(`ASSIST` 플래그: 넓힌 판정·자기 TOTAL)가 하나라도 있으면 `assisted`(A-단위), 없으면 `formal`(정식). 한 판은 한 종류만 준다. `no_speed`·`no_good`·`no_great`·`gauge_*` 는 확인하지 않는다(본문에 사실이 없거나 COURSE GAUGE 설정이 우선이다). 게이지 규칙(LR2·BEATORAJA·IIDX)은 제한하지 않고 `user_dan.course_gauge` 에 기록만 한다(PROCESS 에는 이 조건이 없는 메인 판단이라 ADR 에 사용자 미확인으로 적는다). 결정: [A-단위 ADR](../acknowledge/2026-10-24-adr-assisted-dan.md).
- 어시스트만 빼면 ranked 였던가(`rankedWithoutAssist`): 어시스트 판은 `ranked = false` 로 저장되는데 같은 `false` 가 "신뢰하지 않는 빌드가 막음"일 수도 있어, 제출 때 서버가 `ASSIST` 를 뺀 플래그에 순위를 막는 것이 없고 빌드가 막지 않으면 참으로 계산한다. `ASSIST` 플래그가 있으면 `course_score.extra.ranked_without_assist` 에 **항상 서버 값**을 쓴다(클라이언트가 보낸 같은 키는 덮인다). 저장된 판은 `storedRankedWithoutAssistOf(extra)` 로 읽고 불리언이 아니면 `null` 이다. `null`(마이그레이션 0013 배포 전에 저장된 0012 이후 판)은 `ASSIST` 를 뺀 플래그에 순위를 막는 것이 없으면 참으로 추정한다(코스 플래그는 `ASSIST`·`SCRATCH_AUTO`·`UNKNOWN_BUILD` 뿐이라 사실상 "빌드가 막지 않았다고 본다"). 운영의 `REQUIRE_BUILD_HASH` 가 켜져 있거나 `trusted = false` 빌드가 있으면 빌드가 막았던 판이 이 추정으로 A-단위를 받을 수 있다(둘 다 확인하지 못함). 이 경로의 판 수가 소급 출력의 `assistedLegacy` 다. 클라이언트가 서버 플래그가 없는 판(`flags` 가 빈 판)의 `extra.flags` 에 `ASSIST` 를 넣으면 소급 때 같은 label 의 A-단위 행이 생길 수 있다(자기 판의 종류를 스스로 낮출 뿐이며 테스트 부채 3.128).
- 시점: 코스 제출 라우트가 `courseService.submit` 뒤에 `danService.awardFromRun(scoreRow)` 를 같은 요청 안에서 부른다. 같은 트랜잭션이 아니며 판정이 던져도 제출은 성공하고 `console.error('dan award failed', …)` 만 남는다. 획득하면(정식이든 A-단위든) `revalidateOnDanAward`. 재판정은 `awardForCourses`(소급): `stages` 가 있고 사용자가 있는 저장된 판만 `playedAt` 오름차순, 같으면 id 오름차순으로 읽고 합계 `{ scanned, eligible, awarded, eligibleAssisted, awardedAssisted, assistedLegacy }` 를 낸다(`eligible`·`awarded` 는 정식만, `assistedLegacy` 는 dry run 에서도 센다). 표 갱신이 새 단위 코스를 등록했을 때와 `dan-recompute` 가 부르며, 표 갱신 보고의 `awarded` 는 정식과 A-단위를 합한 값이다(`read-cache` 의 무효화 기준 `awarded > 0` 을 그대로 쓰기 위해). `stages` 가 NULL 인 판(0012 이전, 옛 클라이언트)은 판정하지 않는다.
- `user_dan` 은 (사용자, 체계, label, 종류) 마다 처음 획득한 판만 남긴다(`INSERT IGNORE`, 기본 키 `(user_id, system, label, assisted)`). 같은 단위의 정식과 A-단위가 따로 한 줄씩 있을 수 있고, 정식을 나중에 얻어도 A-단위 행은 지우지 않는다. 표시 선택 `user_dan_display`: 행 없음 = 자동, `label` NULL = 숨김(`assisted` 는 false), 값 + `assisted` = 그 단위(획득한 (label, 종류) 조합만). 보일 단위는 서버의 단일 함수 `shownDansOf` 가 체계마다 정한다: `hidden` 은 없음, `label` 은 고른 한 줄, `auto` 는 정식 최고 rank 가 A-단위 최고 rank 이상이면 정식만, A-단위가 더 높으면 정식(있으면)과 A-단위 둘, A-단위만 있으면 A-단위 하나. 프로필 `extra.dans`·순위 행 `dans`·내 단위 `shown` 이 모두 이 함수 하나를 쓰고, 원소는 `{ system, name, label, assisted }` 이며 A-단위의 `label` 은 `danDisplayLabel`(`dto/dan.ts`)이 `A-` 를 붙인 글자(`A-十段`, `A-st3`)다. 출력 순서는 체계 순, 체계 안에서 정식 다음 A-단위이고, 내 획득 목록은 rank 순에 같은 rank 는 정식이 먼저다. 남에게는 표시 단위만 보이고, 획득 목록 전체·판 id·시각·게이지 규칙은 본인 마이페이지에서만 보인다. `user.rank` 는 쓰지 않는다.
- 웹 화면(A-단위, [설계서](../plan/2026-10-24-assist-dan.md) 7절 기준): 배지(`features/dan/dan-badge.tsx`)는 정식이 `secondary`, A-단위가 점선 테두리의 윤곽 배지와 설명 `title` 이다. 한 체계에 배지가 둘일 수 있어 목록의 `key` 는 `system:assisted` 다. 마이페이지 단위 카드의 선택 목록과 획득 표에는 정식과 A-단위가 함께 나오고 A-단위는 `danDisplayLabel` 의 글자로 보인다. 인정 조건 목록(`dan-panel.tsx`)에 A-단위 항목이 늘었다. 클라이언트(R-BMS 플레이어)는 바뀌지 않는다: 순위 오버레이는 `label` 에 든 `A-十段` 을 그대로 그리고 모르는 `assisted` 는 무시한다(줄마다 배지 2개 제한은 그대로).

### 15-4. 표 매일 갱신과 읽기

- 정의 `REMOTE_TABLES`(`service/domain/table-refresh/table-sources.ts`): `stella-st`(이름 `Stella`, 기호 `st`)와 `stella-sl`(`Satellite`, `sl`), 허용 호스트 `stellabms.xyz`. 표 id 는 클라이언트 기본 표 id 와 같다.
- `tableRefreshService.refreshAll({ allowLargeChange, dryRun })`: 표마다 차례로(원본에 한 번에 하나만 요청) 머리 → 검증 → 본문 → 검증 → 저장된 사본과 비교 → 저장. 받기는 `fetchLimited`(리다이렉트를 따르지 않는 `fetch`, 머리 1 MiB·본문 8 MiB·20초, 사용자 에이전트 `R-BMS table sync (+https://bms.hyuns.uk)`). 검증: UTF-8 JSON(BOM 제거), 머리의 이름·기호가 정의와 같고 `data_url` 이 `https` 와 허용 호스트인지, 본문 항목이 md5 32자 hex 를 갖는지, md5 중복 없음. 변경 판단 `judgeChange`: 첫 사본은 500곡 이상, 이전 사본이 있으면 새 곡 수가 이전의 90% 미만(급감)이거나 추가·삭제·레벨 이동이 25% 초과면 `rejected`(`allowLargeChange` 가 이 둘만 푼다). 결과 outcome 은 `saved`·`unchanged`·`rejected`·`failed`, 이유는 고정 코드(`fetch_failed`, `http_status`, `too_large`, `not_json`, `header_invalid`, `name_mismatch`, `symbol_mismatch`, `data_url_off_site`, `entries_invalid`, `duplicate_md5`, `too_few_entries`, `shrank`, `changed_too_much`, `course_sync_failed`, `unexpected_error`).
- 저장: `table_snapshot` 에 받은 바이트를 gzip 한 그대로(표준 `data.json` 을 원본과 같은 바이트로 낸다)와 SHA-256. 이전 사본이 있으면 `UPDATE … WHERE data_sha256 = <이전>` 이라 다른 실행이 먼저 쓴 사본을 덮지 않는다. 내용이 같으면 `checked_at` 과 `last_outcome` 만 바뀐다. 거절·실패는 저장된 사본을 그대로 두고 `checked_at` 과 `last_outcome`(`rejected:<이유>`)만 남기며 로그에는 고정 코드와 숫자만 쓴다.
- 코스 동기화: 저장됐거나 내용이 같은 표마다 헤더의 코스로 `planSync` 를 하고 바꿀 것이 있을 때만 `applySync` 한다(내용이 같아도 매번 계획을 세워 실패한 등록을 다음 날 따라잡는다). 새로 단위 코스가 된 코스가 있으면 같은 실행에서 소급 판정(`awardForCourses`)을 한다. 코스 동기화가 던지면 표 사본은 남고 이유가 `course_sync_failed` 가 된다. 이 경로의 응답(`{ tables: [...] }`)은 표마다 outcome·이유·수(`entries`·`added`·`removed`·`releveled`)와 코스 계획·`awarded` 만 담고 `Cache-Control: private, no-store` 이다.
- 읽기: `tableSnapshotService` 의 `loadTables` 가 비동기로 번들 insane1 과 저장된 사본을 합친다. 원격 사본은 인스턴스 메모리 캐시(`createRemoteSnapshotCache`)가 `listVersions()` 의 작은 행만 매번 읽고, 저장된 두 해시가 기억한 것과 다를 때만 그 표를 풀어 `remoteHeaderSchema`·`remoteEntriesSchema` 로 다시 검증해 새로 만든다(서버리스 인스턴스마다 한 번씩만 푼다). 레벨 순서는 헤더 `level_order` 가 없으면 항목의 레벨을 수 오름차순(모두 정수일 때) 또는 처음 나온 순서로 만든다. 번들 insane1 만 보는 호출처(차트 제목 대체, 검색, 패턴 coverage, AI)는 비동기로 바꾸지 않았다.
- 같은 일을 사람이 숫자를 보고 하는 길: `tables-refresh-count`(받고 검증·비교만, 쓰지 않음)와 `tables-refresh-force`(`apply-large`: 급감·변경량 검사만 풀고 저장).


## 16. 다국어(한국어·영어·일본어, 2026-10-24)

결정과 근거는 [웹 다국어 ADR](../acknowledge/2026-10-24-adr-web-i18n.md), 용어는 [용어집](../reference/2026-10-24-i18n-glossary.md)이다. 이 절은 **기반(WI0, `eb5ddbb`)** 의 구조이고 문구를 메시지로 옮기는 단위(WI1 위젯 `ai`·`settings`, WI2 탐색 화면 `browse`, WI3 페이지·공용 `pages`, WI4 서버 오류 문구와 AI 결과 언어 `server`)는 같은 웨이브에서 진행 중이라 그 결과는 각 커밋과 `web/messages/` 를 본다.

### 16-1. 구조

| 항목 | 값 |
| --- | --- |
| 라이브러리 | next-intl 4.14.9(`web/package.json` 에 정확 고정, 게시 2026-10-02, 사용자 승인). 간접 의존성도 게시 14일 이상으로 맞췄다(`bun add --minimum-release-age=1209600`) |
| 방식 | 쿠키, 경로 접두사 없음. `createNextIntlPlugin('./src/shared/i18n/request.ts')`(`next.config.ts`), `getRequestConfig` 가 `cookies()` 와 `headers()` 를 읽는다. URL·라우트 맵(§1)과 캐시 태그(§5)는 그대로다 |
| 언어 결정 | 올바른 `NEXT_LOCALE` 쿠키 → `Accept-Language`(q 값 순서, 주 부분만: `en-US` 는 `en`, `q=0` 과 잘못된 q 값은 제외) → 기본 `ko`. 첫 방문에는 쿠키를 심지 않고 요청마다 헤더로 정하므로 `proxy.ts` 를 고치지 않았다. 쿠키·헤더가 없는 봇과 지원 밖 언어는 `ko` (`web/src/shared/i18n/locale.ts`) |
| 메시지 | `web/messages/{ko,en,ja}/{common,ai,settings,browse,pages,server}.json`. 네임스페이스는 이 여섯으로 고정(새로 만들려면 `MESSAGE_NAMESPACES` 와 `messages.ts` 의 import 를 함께 고친다). 최상위 키가 네임스페이스이고 `useTranslations('browse.xxx')`(클라이언트) / `await getTranslations('pages.xxx')`(서버)로 부른다 |
| 타입·검사 | 한국어가 원본이고 en·ja 는 `Record<Locale, Messages>` 라 키가 빠지면 `tsc` 가 막는다(`next-intl.d.ts`). `tests/lib/i18n-messages.test.ts`: 세 언어의 키 집합·ICU 자리표 `{name}` 일치, 빈 문자열 금지. `i18n-locale.test.ts`: 언어 결정 |
| 루트 레이아웃 | `<html lang={await getLocale()}>`, `NextIntlClientProvider` 로 감싼다. `export const instant = false` |
| 언어 선택 위젯 | `widgets/locale-switcher/locale-switcher.tsx`: 사이드바 바닥의 아이콘 버튼 + 라디오 메뉴. 고르면 쿠키(`Path=/`, 1년, `SameSite=Lax`)를 쓰고 `router.refresh()`. 언어 이름은 그 언어로(`한국어`, `English`, `日本語`). 화면에 붙이는 일(`app-shell.tsx` 바닥 `SidebarMenu` 에 `ShellThemeControl` 옆으로, 격자 2열에서 3열)은 WI2 몫이라 기반 단위는 붙이지 않았다 |

### 16-2. 캐시와 렌더(메인 결정)

- 루트 레이아웃이 쿠키를 읽으면 Next 16 에서 앱 전체가 정적 prerender 에서 빠지고 Cache Components 에서는 그 아래 모든 segment 가 blocking 이 된다. 실제로 `bun run build` 가 `/guide` 에서 `blocking-prerender-dynamic` 오류로 멈춰 오류가 제안한 대로 `instant = false` 를 넣었다. 그 결과 기존 부분 정적(◐) 페이지 전부와 정적(○)이던 `/guide`·`/login`·`/signup` 이 모두 요청마다 렌더(ƒ)이고, 정적으로 남은 것은 `robots`·`sitemap`·아이콘뿐이다(빌드 78 페이지).
- **데이터 캐시(§5)는 그대로**다. `read-cache.ts` 를 바꾸지 않았고 `'use cache'` 함수들은 locale 인자도 쿠키 읽기도 없다. 규칙: `'use cache'` 함수 안에서 `getTranslations`·`getLocale`·`cookies()` 를 부르지 않는다. 캐시된 데이터는 언어 없이 받고 번역은 그 바깥 컴포넌트에서 한다(§5-1 의 개인화 데이터 제약과 같은 이유).
- 정적 셸을 되찾으려면 next-intl `localePrefix: 'never'` 와 proxy 내부 rewrite, `app/[locale]` 로 라우트 이동, `next/root-params` 가 필요하다(`src/app/**` 전체를 옮기는 별도 단위, 미확인 목록 26번).
- `timeZone` 은 정하지 않았다(서버 런타임 시간대). 날짜를 `useFormatter` 로 쓰기 시작하면 정해야 한다.

### 16-3. 아직 언어를 따르지 않는 곳

루트 `metadata` 의 `SITE_DESCRIPTION` 과 `openGraph.locale: 'ko_KR'`(`shared/constants/site.ts`), `global-error.tsx` 의 `lang='ko'` 와 한국어 문구(루트 레이아웃 밖이라 provider 가 없다), 그리고 메시지로 옮기지 않은 문구 전부는 WI1~WI4 의 몫이다. API 오류 응답의 형식은 바꾸지 않고 웹 화면이 오류 코드로 번역한다. AI 결과 문구의 언어는 서버에서 `getLocale()` 로 받아 프롬프트 언어 인자로 넘겨 UI 언어를 따른다(WI4).
