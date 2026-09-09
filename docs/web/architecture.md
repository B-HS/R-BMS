# rbms web — 아키텍처 설계 (W0)

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
| UI | Tailwind v4 `@theme`(design.md §16) + shadcn new-york |

### 0-1. 버전 (2026-09-09 레지스트리 최신, 설치 시 재확인)

`next 16.3.4` · `react/react-dom 19.2.8` · `tailwindcss 4.3.3` · `drizzle-orm 0.45.2` / `drizzle-kit 0.31.10` · `mysql2 3.24.4` · `better-auth 1.7.3` · `zod 4.5.4` · `@tanstack/react-query 5.102.8` · `shadcn 4.21.0`(CLI) · `sonner 2.0.8` · `react-hook-form 7.87.0` + `@hookform/resolvers 5.9.1`.

런타임·PM 은 **Bun**(W8 가정). Next 빌드/dev 는 `bun run` 으로 구동하되 번들러는 Next 기본(Turbopack).

---

## 1. 라우트 맵 (App Router)

### 1-1. 페이지 — 라우트 그룹 2개 = 디자인 Surface 2개

design.md §1-2 는 두 Surface 가 **스타일시트를 공유하지 않는다**고 못박는다. 라우트 그룹으로 그 경계를 그대로 옮긴다.

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
| `(app)` | `/tables` · `/tables/[id]` | IR 난이도표 | A | 공개 읽기 |
| `(app)` | `/courses/[hash]` | 코스 랭킹 | A | 공개 읽기 |
| `(app)` | `/replays/[id]` | 리플레이 뷰어(µs 이벤트 타임라인) | A | 공개 읽기 |
| `(app)` | `/settings` | 계정 설정(탭: 프로필 / API 토큰 / 동기화 blob / 라이벌) | A | **세션 필수** |
| `(app)` | `/admin/builds` | 빌드 allowlist(FR-13) | A | **세션 + role=admin** |

- `(app)` 루트의 미인증 처리: 공개 읽기 라우트는 통과, `/settings`·`/admin/*` 만 서버에서 세션 확인 후 `/login?next=` 로 redirect. **가정**: 미들웨어(`proxy.ts`/`middleware.ts`) 대신 각 세그먼트 `layout.tsx` 의 서버 세션 확인으로 게이팅한다(Vercel 미들웨어에서 DB 접속을 피하기 위함).
- `(app)` 셸은 design.md §8-1 3열(사이드바 256px / 콘텐츠 fluid / 컨텍스트 패널 320px, gap 1px, 헤더 없음). 컨텍스트 패널은 이 제품에서 **필터 패널**(모드·레벨·랭프·기간·검색어)로 쓴다.

### 1-2. Route Handler 맵 — 네이티브(계약 동결분, raw JSON)

경로는 contract-freeze A(클라가 실제 호출하는 경로)에 `/api` 프리픽스를 붙인 것과 **정확히 일치**한다. `crates/rbms-ir/src/http.rs` 검증 완료.

| 메서드 | 경로 | 파일 | 인증 | 캐시 태그(읽기) |
|---|---|---|---|---|
| GET | `/api/health` | `app/api/health/route.ts` | 없음 | 없음(항상 동적) |
| GET | `/api/version` | `app/api/version/route.ts` | 없음 | 없음 |
| POST | `/api/auth/register` | `app/api/auth/register/route.ts` | 없음 | — |
| POST | `/api/auth/login` | `app/api/auth/login/route.ts` | 없음 | — |
| GET/POST | `/api/auth/[...all]` | `app/api/auth/[...all]/route.ts` | better-auth 핸들러 | — |
| GET | `/api/auth/me` | `app/api/auth/me/route.ts` | Bearer 또는 세션 | 없음 |
| POST | `/api/auth/token` | `app/api/auth/token/route.ts` | 세션 | — |
| POST | `/api/scores` | `app/api/scores/route.ts` | Bearer 또는 guest | — |
| GET | `/api/scores/[scoreId]` | `app/api/scores/[scoreId]/route.ts` | 없음 | `score:{id}` |
| GET | `/api/charts/[hash]` | `app/api/charts/[hash]/route.ts` | 없음 | `chart:{sha}` |
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
| GET | `/api/courses/[courseHash]` | `.../[courseHash]/route.ts` | 없음 | `course:{hash}` |
| POST | `/api/courses/[courseHash]/scores` | `.../scores/route.ts` | Bearer | — |
| GET | `/api/courses/[courseHash]/ranking` | `.../ranking/route.ts` | 없음 | `course-ranking:{hash}` |
| GET | `/api/courses/[courseHash]/best` | `.../best/route.ts` | 없음 | `course-best:{hash}` |
| GET/POST | `/api/tables` | `app/api/tables/route.ts` | GET 없음 / POST admin | `tables` |
| GET | `/api/tables/[tableId]` | `.../[tableId]/route.ts` | 없음 | `table:{id}` |
| GET/POST | `/api/admin/builds` | `app/api/admin/builds/route.ts` | admin(POST 는 CI 토큰도 허용) | `client-builds` |

**클라 호출 대조(contract-freeze A 전수)**: `/health` `/scores` `/charts/{md5}/ranking?limit=` `/charts/{md5}/best?player=` `/players/{id}` `/players/{id}/rivals` `/courses` `/charts/{md5}/replays` `/courses/{hash}/ranking?limit=` `/replays/{id}` `/players/{id}/settings/{name}`(GET/PUT) `/auth/register` `/auth/login` — 14개 전부 위 표에 존재. 누락 없음.

> **주의 1 — `POST /api/courses` 의 이중 의미.** 클라(`submit_course`)는 `POST /courses` 에 **CourseSubmission** 을 보내고 `SubmitResponse` 를 기대한다(`http.rs:131`). api-spec §6 은 같은 경로를 "코스 메타 업서트"로 쓴다. **계약 우선 원칙(contract-freeze 원칙)**에 따라 `POST /api/courses` = **코스 결과 제출**로 확정하고, 메타 업서트는 `POST /api/courses/meta` 로 분리한다. 제출 본문은 `course_hash` 를 담고 있으므로 라우팅 모호성 없음.
> **주의 2 — better-auth 경로 충돌 없음.** better-auth 기본 경로는 `/api/auth/sign-in/email` · `/sign-up/email` · `/get-session` · `/sign-out` 이라 `register`/`login`/`me`/`token` 과 겹치지 않는다. 겹치더라도 Next 라우팅은 **정적 세그먼트 > 동적 > catch-all** 순으로 우선하므로 `app/api/auth/register/route.ts` 가 `[...all]` 보다 먼저 매칭된다.
> **주의 3 — 세 라우트의 성공 응답 모양(클라가 실제로 읽는 필드).**
> - `POST /api/scores` · `POST /api/courses` · `POST /api/courses/[courseHash]/scores` → `SubmitResponse` 8필드 전부: `accepted`·`rank`·`previous_best`·`message` + `ranked`·`flags`·`is_new_best`·`score_id`(`dto/score.ts submitResponseSchema`). `score_id` 는 리플레이를 스코어에 연결하는 키라 생략할 수 없다.
> - `PUT /api/players/[playerId]/settings/[name]` → 200 `{ updated_at }`(서버가 저장한 stamp, unix ms). 클라가 보낸 `updated_at` 은 무시되므로 이 값이 다음 조건부 쓰기의 낙관적 잠금 base 다. 잠금에서 밀리면 409 `{ conflict, server }`.
> - `GET /api/replays/[replayId]` → `ReplayData` 전체. `gauge` 와 차트의 `md5`+`sha256` 을 포함해야 다운로드한 리플레이가 원래 런을 재현한다.
>
> **주의 4 — `/api/charts/search` 금지.** api-spec §12 의 `/api/charts/search` 는 `[hash]` 동적 세그먼트와 충돌 가능성이 있고 envelope 정책도 다르다. FE 검색은 `/api/fe/charts/search` 로 둔다.

### 1-3. Route Handler 맵 — FE 전용(`/api/fe/*`, envelope)

전부 `{ success, data, pagination? }` 봉투. 세션 쿠키 기준(공개 읽기는 무인증).

| 메서드 | 경로 | 용도 | 인증 | 캐시 태그 |
|---|---|---|---|---|
| GET | `/api/fe/charts/search` | 차트 검색(q·mode·level·sort·page·limit) | 없음 | `chart-search`(+쿼리 인자) |
| GET | `/api/fe/charts/[hash]/leaderboard` | 리더보드(랭킹 + 차트 메타 조인 + player_name·replay_id) | 없음 | `chart-ranking:{sha}` |
| GET | `/api/fe/activity/recent` | 전체 최근 제출 피드 | 없음 | `activity-recent` |
| GET | `/api/fe/leaderboards/players` | 종합 플레이어 랭킹 | 없음 | `leaderboard-players` |
| GET | `/api/fe/players/[playerId]/recent` | 플레이어 최근 플레이(차트 조인) | 없음 | `player-recent:{id}` |
| GET | `/api/fe/players/[playerId]/stats` | 플레이어 통계(램프 분포·레벨별 EX·플레이수) | 없음 | `player-stats:{id}` |
| GET | `/api/fe/stats/summary` | 서버 통계(차트/플레이어/제출 수) | 없음 | `stats-summary` |
| GET/POST | `/api/fe/tokens` | 내 API 토큰 목록 / 발급 | 세션 | 없음 |
| DELETE | `/api/fe/tokens/[tokenId]` | 토큰 폐기 | 세션·본인 | — |
| GET | `/api/fe/me/settings` | 내 설정 blob 키 목록 | 세션 | 없음 |

### 1-4. 인증 방식 요약 표

| 대상 | 자격 | 검증 위치 |
|---|---|---|
| 네이티브 클라(rbms-player) | `Authorization: Bearer <api_token>` | `withApiToken` HOF → `api_token.token_hash` 조회 |
| 웹 FE | better-auth 세션 쿠키(HttpOnly·SameSite=Lax) | `withSession` HOF → `auth.api.getSession({ headers })` |
| guest 제출 | 자격 없음 + `ALLOW_GUEST=true` | `POST /api/scores` 만 허용, 항상 `ranked=false` + `flags:["GUEST"]` |
| admin | 세션 + `user.role='admin'` | `withAdmin` |
| 릴리스 CI(빌드 등록) | 전용 admin `api_token` | `POST /api/admin/builds` |

`GET /api/auth/me` 는 **Bearer 와 세션 둘 다** 수용한다(네이티브·FE 공용). 그 외 네이티브 경로는 Bearer, FE 경로는 세션.

---

## 2. 디렉터리 구조 (FSD + 서버 계층)

```
web/
  package.json  next.config.ts  tsconfig.json  drizzle.config.ts  components.json
  bunfig.toml  .env.example
  drizzle/                            drizzle-kit 산출 마이그레이션(커밋)
  src/
    app/                              ← FSD app 레이어 = Next App Router
      layout.tsx                      html/body, 프로바이더, pre-paint 스크립트
      (public)/layout.tsx             Surface B 셸
        login/page.tsx  signup/page.tsx  guide/page.tsx
      (app)/layout.tsx                Surface A 3열 셸
        page.tsx  search/page.tsx
        charts/[hash]/page.tsx
        players/[playerId]/page.tsx
        leaderboards/page.tsx
        tables/page.tsx  tables/[tableId]/page.tsx
        courses/[courseHash]/page.tsx
        replays/[replayId]/page.tsx
        settings/page.tsx
        admin/builds/page.tsx
      api/…                           ← §1-2·§1-3 Route Handler (얇은 어댑터만)
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
        domain/{auth,score,chart,player,replay,setting,course,table,fe}/
        shared/{storage,ratelimit,integrity}/
      dto/                            zod 스키마(입력·응답)
      db/  index.ts  schema.ts
      lib/  error-code.ts error-message.ts error.ts api-response.ts
            with-error-handling.ts with-api-token.ts with-session.ts
            with-admin.ts with-rate-limit.ts env.ts auth.ts
      compose/                        의존성 조립(도메인별 ServiceDb 구현)
  tests/                              bun test (dto·service·lib·route)
  mocks/                              MSW 핸들러(UI 선행 개발용)
```

### 2-1. Path alias (= 레이어, `fsd.md` §5)

```jsonc
{
  "@app/*":      ["./src/app/*"],
  "@widgets/*":  ["./src/widgets/*"],
  "@features/*": ["./src/features/*"],
  "@entities/*": ["./src/entities/*"],
  "@shared/*":   ["./src/shared/*"],
  "@server/*":   ["./src/server/*"]
}
```

- 의존 방향: `app → widgets → features → entities → shared`. 역참조 금지, barrel(`index.ts`) 금지.
- `@server/*` 는 **`app/api/**` 와 서버 컴포넌트에서만** import 한다. `widgets`/`features`/`entities` 는 절대 import 하지 않는다(번들 유출 방지). 파일 상단 `import 'server-only'` 로 강제.
- `entities/*.api.ts` 는 브라우저/서버 양쪽에서 도는 fetch 래퍼(`clientFetch`)만 둔다. Drizzle 은 `@server/db` 안에서만.

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
| 도메인 전부(`chart` `score` `chart_best` `replay` `api_token` `rival` `setting_blob` `course*` `difficulty_table` `table_*` `client_build` `submission_audit`) | **Drizzle 수기** | `src/server/db/schema.ts` (= `docs/backend/data-model.md` 그대로) |

- contract-freeze D-2 결정 유지. 도메인 FK 는 `user.id` 참조.
- `user` 의 IR 확장 필드(`rank`·`rank_points`·`total_plays`·`allow_guest_merge`·`extra`)는 better-auth `user.additionalFields` 로 선언해 어댑터가 같은 테이블에 들고 있게 한다. **가정**: data-model.md 의 `user.id varchar(64)` = 로그인 id 를 그대로 better-auth PK 로 쓴다(better-auth 기본은 생성 id). 어긋나면 §7 오픈 질문 참조.
- `password_hash` 는 better-auth 가 `account` 테이블에 보관하므로 `user.password_hash` 컬럼은 **두지 않는다**(data-model.md 대비 유일한 의도적 이탈).

### 3-2. 마이그레이션 운영

- `drizzle.config.ts`: `schema: ['./src/server/db/schema.ts', './src/server/db/auth-schema.ts']`, `dialect: 'mysql'`, `out: './drizzle'`.
- 흐름: `bunx @better-auth/cli generate` → `bunx drizzle-kit generate` → `bunx drizzle-kit migrate`. 산출 파일은 **커밋**한다.
- **`drizzle-kit push` 금지**(운영 DB 히스토리 유실). 로컬 실험도 generate/migrate 로 통일.
- 인덱스 우선 적용: `chart(uniq md5)`, `score uniq(user_id, chart_sha256, played_at)`(멱등), `score idx(chart_sha256, ranked, clear, ex_score)`, `chart_best PK(chart_sha256, user_id)`, `score idx(client_build_sha256)`.
- 커넥션: `mysql2/promise` 풀 싱글톤(`src/server/db/index.ts`). Vercel Fluid Compute 특성상 인스턴스당 풀이 살아있으므로 `connectionLimit` 은 작게(**가정: 5**) 잡고 `enableKeepAlive: true`.

---

## 4. 인증 상세

### 4-1. better-auth

- `emailAndPassword: { enabled: true }`, 세션 쿠키(HttpOnly·Secure·SameSite=Lax). OAuth 는 후속(W2).
- `database: drizzleAdapter(db, { provider: 'mysql' })`. 핸들러는 `app/api/auth/[...all]/route.ts` 에서 `toNextJsHandler(auth)` 로 GET/POST 노출.
- `baseURL = BETTER_AUTH_URL`, 시크릿 = `BETTER_AUTH_SECRET`.
- **IR 호환 래퍼**: `POST /api/auth/register` · `/api/auth/login` 은 better-auth 를 내부 호출(`auth.api.signUpEmail` / `signInEmail`)한 뒤, 응답을 클라가 기대하는 `AuthResponse { token, player: {id}, name }`(`dto.rs:311`)로 **변환**해 raw 로 돌려준다. 동시에 세션 쿠키도 함께 세팅해 웹에서 같은 엔드포인트를 재사용할 수 있게 한다.
  - `AuthRequest { id, password, email?, name? }` 에서 `id` 는 로그인 id, `email` 은 optional. better-auth 는 email 을 필수로 요구하므로 **email 미제공 시 `{id}@local.invalid` 형태의 내부 합성 주소를 쓴다**(가정 — §7 오픈 질문).
  - 반환 `token` 은 세션 토큰이 아니라 **새로 발급한 `api_token`** 이다(네이티브 클라가 Bearer 로 재사용해야 하므로).

### 4-2. api_token (네이티브 Bearer)

- 발급: `POST /api/auth/token`(세션) 또는 `POST /api/fe/tokens`(FE UI). 응답에 **평문은 그 1회만** 반환.
- 저장: `api_token.token_hash` = SHA-256(평문). 평문·역산 가능한 형태로 저장하지 않는다. 조회는 해시 룩업(uniq 인덱스).
- 형식: `rbms_<32바이트 base64url>`. prefix 로 스캐너 탐지 용이.
- 검증(`withApiToken`): `Authorization: Bearer` 파싱 → 해시 → `api_token where token_hash and revoked=false` → `user` 조인 → `last_used_at` 갱신(쓰기 부하 완화를 위해 **60초 이내 재갱신 생략**).
- 폐기: `DELETE /api/fe/tokens/[tokenId]` → `revoked=true`.
- **better-auth `apiKey` 플러그인은 쓰지 않는다.** `data-model.md` 가 이미 `api_token` 을 정의했고, 클라가 기대하는 발급 응답 형태(`AuthResponse.token`)를 직접 통제해야 하기 때문. (플러그인 채택 여부는 §7 오픈 질문 아님 — 설계 확정.)

### 4-3. guest

- `ALLOW_GUEST=true` 일 때만 `POST /api/scores` 를 무자격 수용. `player.id` 는 `guest` 로 강제 고정하고 `score.user_id = null`, `ranked=false`, `flags:["GUEST"]`.
- guest 제출은 `chart_best`·랭킹에 반영하지 않는다. 감사 로그(`submission_audit`)에는 남긴다.

### 4-4. 레이트리밋

| 대상 | 한도 |
|---|---|
| `POST /api/auth/login` · `/register` | 5/min/IP |
| `POST /api/scores` | 60/min/token |
| `POST /api/charts/[hash]/replays` | 10/min/token |
| `/api/fe/*` 공개 읽기 | 120/min/IP |

- 구현: `withRateLimit(key, limit, windowMs)`. **v1 은 인스턴스 로컬 카운터(LRU)** — Vercel 멀티 인스턴스에서는 best-effort. 로그인/가입만 DB 카운터(`submission_audit` 와 별개의 경량 테이블 또는 better-auth 내장 rateLimit)로 보강한다. 전역 정확성이 필요해지면 Upstash Redis 도입(§7 오픈 질문).
- 본문 크기: `POST /api/scores` 256KB(`content-length` 헤더 + 실제 스트림 바이트 수 둘 다 검사), 리플레이 `REPLAY_MAX_BYTES`(기본 4MB) 초과 시 `413 IR_PAYLOAD_TOO_LARGE`.

### 4-5. 보안 기본

- 비밀번호는 better-auth 위임. 토큰은 해시 저장. 시크릿은 `getEnv()` 경유만.
- CORS: 같은 오리진(Next 단독 서버)이라 FE 용 CORS 불필요. 네이티브 클라는 브라우저가 아니라 CORS 무관. 따라서 **CORS 미들웨어를 두지 않는다**(api-spec §10 대비 이탈 — 단독 서버 구조의 결과). 외부 오리진 요구가 생기면 `ALLOWED_ORIGINS` 로 추가.
- 제출 검증: 토큰 user.id == body `player.id`(guest 제외), `played_at` 합리 범위(서버시각 ±7일), `chart.sha256` 형식(소문자 16진수 64자), judge 합계 ≤ `total_notes` 일관성(`empty_poor` 제외, `total_notes=0` 인 구버전 제출은 예외).
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

- 서버에서는 **요청마다 `new QueryClient()`**. 클라이언트는 싱글톤(`get-query-client.ts`), `gcTime >= staleTime`(기본 `staleTime 60s`, `gcTime 10m`).
- **빌드 시 DB 미접속 보장**: `cacheComponents` 가 켜지면 `'use cache'` 함수는 프리렌더 중 실행될 수 있다. 따라서 **DB 를 읽는 모든 페이지 영역은 `<Suspense>` 로 감싼다**(정적 셸만 프리렌더). `generateStaticParams` 를 쓰지 않고, `next build` 는 셸만 생성한다. 이 규칙은 §8 검증 게이트에서 확인한다.

---

## 6. 환경변수 · 배포

### 6-1. `getEnv()` (zod, `@server/lib/env.ts`)

| 키 | 필수 | 설명 |
|---|---|---|
| `DATABASE_URL` | 조건부 | `mysql://user:pass@host:port/db`. 없으면 아래 5개 필수 |
| `DB_HOST` `DB_PORT` `DB_USER` `DB_PASSWORD` `DB_NAME` | 조건부 | `DATABASE_URL` 미제공 시 조합해 URL 생성. **코드(`src/server/lib/env.ts`)가 정본** — 2026-09-09 결정 "env 키" 항목 |
| `BETTER_AUTH_SECRET` | O | 세션 서명 |
| `BETTER_AUTH_URL` | O | `https://bms.hyuns.uk` |
| `NEXT_PUBLIC_APP_URL` | O | `https://bms.hyuns.uk` (클라 노출 가능한 값만) |
| `ALLOW_GUEST` | X(기본 `false`) | guest 제출 허용 |
| `REPLAY_MAX_BYTES` | X(기본 `4194304`) | 리플레이 상한 |
| `REQUIRE_BUILD_HASH` | X(기본 `false`) | `client_build_sha256` 누락 시 거부 여부 |
| `REPLAY_STORAGE` | X(기본 `db`) | `db` \| `blob`. blob = Vercel Blob |
| `BLOB_READ_WRITE_TOKEN` | 조건부 | `REPLAY_STORAGE=blob` 일 때 |
| `SERVER_VERSION` `SERVER_COMMIT` | X | `/api/version` 표기(Vercel `VERCEL_GIT_COMMIT_SHA` fallback) |
| `NODE_ENV` | 자동 | — |

- zod `safeParse`. `DATABASE_URL` 또는 `DB_HOST`/`DB_USER`/`DB_NAME` 조합은 `resolveDatabaseUrl()` 이 판정하고, 실제 접속이 필요한 시점(`getRequiredDatabaseUrl()`)에만 throw 한다 — 빌드 시 DB 미접속 보장(§5-4)을 위해 모듈 로드 시점에 던지지 않는다.
- **빈 문자열(`KEY=`)은 미설정으로 취급**한다(`.env` 스캐폴드가 빈 값을 담고 있어도 기본값이 적용된다).
- `process.env` 직접 접근은 `env.ts` 안으로 한정. `NEXT_PUBLIC_*` 에는 시크릿을 넣지 않는다.
- W1a 에서 추가된 키: `REQUIRE_BUILD_HASH`(기본 false) · `REPLAY_STORAGE`(기본 `db`) · `BLOB_READ_WRITE_TOKEN` · `SERVER_VERSION` · `SERVER_COMMIT` · `VERCEL_GIT_COMMIT_SHA`.

### 6-2. Vercel

- 런타임 **Node.js**(mysql2 필요, Edge 불가). Cache Components 도 Edge 미지원.
- 리전 **`icn1`**(서울) — DB 위치가 결정되면 그 리전에 맞춘다(**가정**).
- 도메인 `bms.hyuns.uk`. 클라 안내 문구는 `--server https://bms.hyuns.uk/api`.
- 빌드 명령 `bun run build`, 설치 `bun install`. `drizzle-kit migrate` 는 **빌드에 포함하지 않는다**(빌드 시 DB 미접속 원칙). 마이그레이션은 별도 수동/CI 스텝.
- `next.config.ts`: `reactCompiler: true`(이미 설정됨) + **`cacheComponents: true`** + `serverExternalPackages: ['mysql2']`. 뒤 두 개는 현재 스캐폴드에 없으므로 W1a 착수 시 추가한다.

---

## 7. 무결성 (FR-13 / FR-14 / FR-18) 처리 위치

| 요구 | 위치 |
|---|---|
| FR-13 빌드 sha256 allowlist | `@server/service/shared/integrity/build-allowlist.ts` — `client_build` 조회(`'use cache'` + `cacheTag('client-builds')`). CI 등록은 `POST /api/admin/builds` |
| FR-14 ranked 판정 | `@server/service/domain/score/ranked-policy.ts` — 순수 함수 `rankedPolicy({ options, buildTrust, isGuest, requireTrustedBuild }) => { ranked, flags }`. 입력만으로 결정되므로 DB 무관, 단위 테스트 대상 |
| 빌드 신뢰도와 ranked | `UNKNOWN_BUILD` flag 는 항상 남기되, ranked 를 막는 것은 ① `untrusted`(allowlist 에 `trusted=false` 로 등록) ② `unknown` + `REQUIRE_BUILD_HASH=true` 두 경우다. 기본 배포(`REQUIRE_BUILD_HASH=false`)에서 미등록 빌드는 flag 만 남고 ranked 로 집계된다 — allowlist 등록 경로(`POST /api/admin/builds`)가 W2a 이므로, 그 전까지 랭킹이 통째로 비는 것을 막기 위함 |
| flags 종류 | `AUTOPLAY` `SCRATCH_AUTO` `ASSIST` `JUDGE_WIDTH`(judge_rate>100) `TOTAL_OVERRIDE` `UNKNOWN_BUILD` `GUEST` `REPLAY_MISMATCH`(후속) |
| 클라 정책 정합 | 결정 #12(assist 정책)와 일치해야 한다. 클라가 이미 `score=false` 로 제출을 막는 케이스도 서버가 독립 재판정한다(신뢰하지 않음) |
| FR-18 감사 로그 | `@server/service/domain/score/audit.ts` — 모든 제출(거부 포함) `submission_audit` insert. IP 는 `x-forwarded-for` 첫 값, UA 는 `user-agent` |
| best 갱신 | `chart_best` upsert. 비교 순서 **램프 > EX > BP(낮을수록)**. `ranked=false` 는 갱신 대상 제외 |
| 멱등 | `score uniq(user_id, chart_sha256, played_at)` — 중복 시 기존 `score_id` 반환, 201 대신 200 |

---

## 8. 테스트 전략

| 층 | 러너 | 대상 |
|---|---|---|
| dto | `bun test` | zod 라운드트립. **`crates/rbms-ir/src/dto.rs` 가 실제로 보내는 JSON 픽스처**를 그대로 파싱 성공하는지(계약 회귀 방지) |
| lib | `bun test` | `createAppError`/STATUS_MAP, `api-response` 헬퍼, `getEnv` 검증 실패 케이스, 토큰 해시 |
| service | `bun test` | `*ServiceDb` 를 인라인 객체로 대체(mocking 라이브러리 없음). best 갱신 규칙·멱등·EX/BP 유도·낙관적 잠금 충돌·`rankedPolicy` 전 flags |
| Route Handler | `bun test` | 핸들러를 직접 import 해 **`new Request(url, {...})`** 로 호출하고 `Response` 를 검증. 인증 401/403, 404, 413, 429, envelope 유무, raw 응답 형태 |
| 컴포넌트 | `bun test` + Testing Library | features 순수 UI 렌더·상호작용. 서버 상태는 MSW 로 목 |
| E2E | 후속(합의 필요) | Playwright. W0 범위 아님 |

- 테스트 설명은 한국어. `describe`/`test`.
- **계약 왕복 검증(수동, PRD §8)**: `rbms-player --server http://localhost:3000/api --player <id>` 로 submit → ranking 왕복. W1 완료 게이트.
- 최소 기계 검증: `bunx tsc --noEmit` → `bunx prettier --check` → `bun test`.

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
- `GET /api/health` · `GET /api/version` 은 응답 본문이 전부 빌드 타임 상수라 **정적 프리렌더(○)** 로 남긴다(§1-2 의 "항상 동적" 대비 의도적 이탈). 동적화하려면 `connection()` 이 필요한데, 그러면 Route Handler 를 `new Request()` 로 직접 호출하는 테스트(§8)가 Next 요청 컨텍스트 밖이라 실패한다.
- `user` PK 는 better-auth 생성 id, 로그인 id 는 `user.login_id`(uniq) 이며 `password_hash` 컬럼은 두지 않는다(OQ1 결정 반영). `/api/players/{id}` 는 `login_id` 로 해석한다.

## 10. 가정 목록 (사용자 회신 시 변경)

1. `web/` 라이선스 = 레포 GPL-3.0(W8). contract-freeze 의 "별 MIT 레포" 전제는 W1 결정으로 폐기됨.
2. better-auth `user.id` 를 로그인 id(문자열)로 그대로 사용.
3. email 미제공 계정은 `{id}@local.invalid` 합성 주소.
4. Vercel 리전 `icn1`, mysql2 풀 `connectionLimit: 5`.
5. 리플레이 저장 기본 `REPLAY_STORAGE=db`(mediumblob), 규모 커지면 Vercel Blob 전환.
6. 레이트리밋 v1 = 인스턴스 로컬(best-effort).
7. `POST /api/courses` = 코스 **결과 제출**(클라 계약 우선), 메타 업서트는 `/api/courses/meta`.
8. `(app)` 인증 게이팅은 미들웨어가 아닌 세그먼트 layout 에서 수행.
