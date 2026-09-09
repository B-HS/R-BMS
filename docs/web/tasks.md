# rbms web — 구현 작업 분할 (W1 / W2)

> 근거: `docs/web/architecture.md`(라우트·계층·캐시·env), `docs/web/components.md`(shadcn 목록·토큰), `docs/backend/endpoint-tasks.md`(엔드포인트 체크리스트), `docs/acknowledge/2026-09-09-enhancement-decisions.md`(W1~W8).
> 원칙: **네 갈래 작업이 같은 파일을 쓰지 않는다.** 아래 "파일 소유권"은 배타적이며, 다른 작업의 소유 파일은 읽기만 한다.

---

## 0. W0-부트스트랩 (선행, 단독 실행 — 병렬 금지)

> **상태: 대부분 완료됨.** `web/` 는 병렬 작업으로 이미 스캐폴드되어 있다(alias 7종 · FSD + `src/server/*` 계층 · `(app)`/`(public)` 그룹 · `api/{auth,health,version}` 골격 · 의존성 설치). 남은 것은 아래 체크리스트 중 미완 항목과 `architecture.md` §9 의 **불일치 3건**(cacheComponents 미설정 · `db:push` 스크립트 · 중복 `cn` 패키지)이다. W1a 담당이 착수 시 먼저 해소한다.

W1a/W1b 가 동시에 시작하려면 뼈대가 먼저 있어야 한다. 이 단계만 순차로 한 명이 한다.

- [ ] `web/` 생성: `bun create next-app`(App Router · TypeScript · Tailwind v4 · src 디렉터리 · alias 미사용 → 직접 설정).
- [ ] `package.json` 스크립트: `dev` `build` `start` `typecheck`(`tsc --noEmit`) `test`(`bun test`) `lint` `db:generate` `db:migrate` `auth:generate`.
- [ ] `tsconfig.json` paths = architecture §2-1 의 6개 alias.
- [ ] `next.config.ts`: `reactCompiler: true`(완료) + `cacheComponents: true` + `serverExternalPackages: ['mysql2']`(미완).
- [ ] `package.json` 에서 `db:push` 제거, `cn` 패키지 제거(자체 `@shared/lib/cn` 사용).
- [ ] `prettier.config.js`: printWidth 150 · tabWidth 4 · semi false · singleQuote true · jsxSingleQuote true · trailingComma all · arrowParens always · bracketSameLine true · endOfLine lf.
- [ ] `bunx shadcn@latest init`(new-york · neutral · cssVariables · lucide, `aliases.ui = @shared/ui`).
- [ ] `src/app/globals.css`: design.md §16-1 전체 이식 + §16-2 델타를 `[data-surface='public']` 로. `src/app/layout.tsx` 에 §16-3 pre-paint 스크립트(키 `rbms-theme`).
- [ ] `.env.example`: architecture §6-1 의 전 키.
- [ ] `src/server/lib/env.ts`(getEnv + zod), `src/server/db/index.ts`(mysql2 풀 싱글톤 · `import 'server-only'`).
- [ ] 빈 디렉터리 골격 생성(§2 트리) + `.gitkeep`.
- **완료 기준**: `bun run dev` 기동, `/` 가 빈 Surface A 셸로 렌더, `bunx tsc --noEmit` 통과, DB 미접속 상태에서 `bun run build` 성공.

---

## W1a — api-core (스키마 전체 + M1 엔드포인트)

**목표**: 네이티브 클라(`rbms-player --server http://localhost:3000/api`)가 submit → ranking 왕복에 성공한다.

### 파일 소유권 (배타)

```
src/server/db/schema.ts            src/server/db/auth-schema.ts
src/server/lib/**                  (env.ts 는 W0 산출물 인계)
src/server/dto/**
src/server/service/**
src/server/compose/**
src/server/route/**
src/app/api/health/**              src/app/api/version/**
src/app/api/auth/**                src/app/api/scores/**
src/app/api/charts/**              src/app/api/players/[playerId]/route.ts
drizzle/**                         drizzle.config.ts
tests/{dto,service,lib,route}/**
```

### 작업

- [ ] `db/schema.ts` — `data-model.md` **전 테이블**(chart · score · chart_best · replay · api_token · rival · setting_blob · course · course_chart · course_score · course_best · difficulty_table · table_folder · table_chart · table_course · client_build · submission_audit). W2a 가 쓸 테이블까지 **여기서 한 번에** 정의(W2a 는 스키마를 건드리지 않는다).
- [ ] better-auth 설정(`server/lib/auth.ts`) + `auth:generate` → `db/auth-schema.ts`. `user.additionalFields` 로 IR 확장 필드.
- [ ] `db:generate` → `db:migrate`. `push` 사용 금지.
- [ ] `lib/error-code.ts` · `error-message.ts` · `error.ts`(STATUS_MAP · createAppError · isAppError). 코드: `IR_CHART_NOT_FOUND` `IR_PLAYER_NOT_FOUND` `IR_SCORE_NOT_FOUND` `IR_REPLAY_NOT_FOUND` `IR_SETTING_NOT_FOUND` `IR_COURSE_NOT_FOUND` `IR_TABLE_NOT_FOUND` `IR_ACCOUNT_EXISTS` `IR_PAYLOAD_TOO_LARGE` `RATE_LIMITED` `UNAUTHORIZED` `FORBIDDEN` `VALIDATION_ERROR` `SERVICE_NOT_CONFIGURED`.
- [ ] `lib/api-response.ts`: `successResponse` · `paginatedResponse` · `errorResponse` · `irRaw`.
- [ ] HOF: `withErrorHandling` · `withApiToken` · `withSession` · `withAdmin` · `withRateLimit`.
- [ ] dto: `judgeBreakdownSchema` · `playOptionsSchema` · `scoreSubmissionSchema` · `chartUpsertSchema` · `rankingQuerySchema` · `accountSchema` · `loginSchema` · `chartMetaSchema` · `scoreRecordSchema` · `submitResponseSchema` · `playerProfileSchema` · `serverInfoSchema`.
  - **필드명은 `crates/rbms-ir/src/dto.rs` 그대로**. 누락 필드는 `.default()`/`.optional()`.
- [ ] 엔드포인트: `GET /api/health` · `GET /api/version` · `POST /api/auth/register` · `POST /api/auth/login` · `GET /api/auth/me` · `POST /api/auth/token` · `GET/POST /api/auth/[...all]` · `POST /api/scores` · `GET /api/scores/[scoreId]` · `GET/POST /api/charts` · `GET /api/charts/[hash]` · `GET /api/charts/[hash]/ranking` · `GET /api/charts/[hash]/best` · `GET /api/players/[playerId]`.
- [ ] 무결성: `rankedPolicy` · `build-allowlist` · `submission_audit` insert · `chart_best` 갱신(램프>EX>BP) · 멱등 처리.
- [ ] 캐시: 읽기 서비스에 `'use cache'` + `cacheTag`(architecture §5-2), 제출 후 `revalidateTag`(§5-3).

### 완료 기준 / 검증

```
bunx tsc --noEmit
bun test tests/
bun run dev &  →  cargo run -p rbms-player -- --server http://localhost:3000/api --player alice
```
- `/api/health` 200 + capabilities. `/api/scores` 제출 → 같은 본문 재전송 시 **중복 생성 없음**(같은 score_id).
- `/api/charts/{md5}/ranking?limit=50` 이 **raw 배열**(봉투 없음)로 응답, `rbms-ir` 가 `Vec<ScoreRecord>` 로 디코드 성공.
- autoplay=true 제출 → `ranked:false`, `flags:["AUTOPLAY"]`, 랭킹 미노출.
- `bun run build` 가 DB 접속 없이 성공.

---

## W1b — web-ui (셸 · 페이지 · entities · MSW)

**목표**: 실제 API 없이 MSW 목만으로 전 화면이 동작한다. W1a 완료 후 목만 걷어내면 붙는다.

### 파일 소유권 (배타)

```
src/app/layout.tsx                 src/app/(app)/**   src/app/(public)/**
   (단 src/app/api/** 는 전부 W1a·W2a 소유 — 건드리지 않음)
src/widgets/**   src/features/**   src/entities/**   src/shared/**
mocks/**         tests/components/**
```

### 작업

- [ ] `shared/lib/cn.ts` · `shared/lib/fetch.ts`(`clientFetch` — 봉투 해석: FE 경로 envelope, 네이티브 경로 raw) · `shared/utils/get-query-client.ts`(staleTime 60s · gcTime 10m) · `shared/constants/query-key.ts`.
- [ ] `QUERY_KEY` 계층: `CHART.{META(hash), RANKING(hash,params), BEST(hash,player), SEARCH(params)}` · `PLAYER.{PROFILE, SCORES, RECENT, STATS, RIVALS}` · `SCORE.DETAIL` · `REPLAY.DETAIL` · `COURSE.*` · `TABLE.*` · `STATS.SUMMARY` · `ACTIVITY.RECENT` · `LEADERBOARD.PLAYERS` · `TOKEN.LIST` · `AUTH.SESSION`.
- [ ] `entities/*.query.ts` — 각 도메인의 `*QueryOptions` 팩토리 + `use*` 훅. `'use client'` 명시. **인라인 queryKey 금지**.
- [ ] `entities/*.type.ts` — 응답 타입. 가능한 한 zod 없이 `dto.rs` 형태를 그대로 반영(W1a 의 zod 스키마와 중복 정의하지 않고, FE 는 필요한 필드만 타입 선언).
- [ ] 셸: `(app)/layout.tsx` 3열(NavRail 256 / main fluid / FilterPanel 320, gap 1px, 헤더 없음) · `(public)/layout.tsx` 중앙 카드.
- [ ] 컴포넌트: components.md §3 의 Panel Card · Stat Tile · Data Table · Status Badge(램프 매핑) · State Triad · Pager · Column Toggle · Panel Footnote · Bar-Count List.
- [ ] shadcn 설치: components.md §4-1 목록.
- [ ] 페이지(전부 MSW 데이터로): `/` · `/search` · `/charts/[hash]` · `/players/[playerId]` · `/leaderboards` · `/login` · `/signup` · `/guide`.
- [ ] 서버 prefetch + `HydrationBoundary`(architecture §5-4). DB 를 읽는 영역은 `<Suspense>` 로 감싼다.
- [ ] MSW 핸들러: architecture §1-2·§1-3 응답 형태를 그대로 흉내. **W1a 의 실제 응답이 나오면 픽스처를 실응답으로 교체**.

### 완료 기준 / 검증

- `bunx tsc --noEmit` · `bun test tests/components` 통과.
- 라이트/다크 **양쪽** 실렌더 확인(스크린샷). components.md §6-1 grep 2종 0건.
- 필터/검색/정렬/페이지가 전부 URL 에 직렬화되고, 새로고침으로 화면이 재현된다.
- 목록 로딩 시 EmptyState 깜빡임 없음(`isLoading` 가드 → 스켈레톤 → empty/list 순서).

---

## W2a — api-ext (replays · settings · courses · tables · fe)

**전제**: W1a 완료(스키마·lib·HOF·dto 공용부가 이미 존재). **`db/schema.ts` 를 수정하지 않는다.**

### 파일 소유권 (배타)

```
src/app/api/replays/**
src/app/api/players/[playerId]/{rivals,scores,settings}/**
src/app/api/courses/**   src/app/api/tables/**   src/app/api/admin/**
src/app/api/fe/**
src/server/route/{replay,setting,course,table,fe,admin}.ts
src/server/service/domain/{replay,setting,course,table,fe}/**
src/server/service/shared/storage/**
src/server/dto/{replay,setting,course,table,fe}.ts
tests/{service,route}/{replay,setting,course,table,fe}.test.ts
```

> `src/server/dto/index` 성격의 배럴은 만들지 않으므로 W1a 파일과 충돌하지 않는다. 공용 스키마(`judgeBreakdownSchema` 등)는 W1a 파일에서 **import 만** 한다.

### 작업

- [ ] replays: `POST /api/charts/[hash]/replays`(µs 무손실 · `REPLAY_MAX_BYTES` 초과 413 · score 연결) · `GET /api/replays/[replayId]` · `GET /api/charts/[hash]/replays`. `storageService`(db blob | Vercel Blob 추상화).
- [ ] settings: `GET/PUT /api/players/[playerId]/settings/[name]`(본인 검증 · PUT 은 **204 No Content** — `http.rs:92` 가 그렇게 기대) · 낙관적 잠금(`base_updated_at`) · `GET /api/fe/me/settings`.
- [ ] rivals: `GET/PUT /api/players/[playerId]/rivals`. scores: `GET /api/players/[playerId]/scores`.
- [ ] courses: `POST /api/courses`(= **결과 제출**, architecture §1-2 주의 1) · `POST /api/courses/meta` · `GET /api/courses/[courseHash]` · `POST /api/courses/[courseHash]/scores` · `GET .../ranking` · `GET .../best`.
- [ ] tables: `GET /api/tables` · `GET /api/tables/[tableId]` · `POST /api/tables`(admin).
- [ ] admin: `GET/POST /api/admin/builds`.
- [ ] FE 전용(envelope): `/api/fe/charts/search` · `/api/fe/charts/[hash]/leaderboard` · `/api/fe/activity/recent` · `/api/fe/leaderboards/players` · `/api/fe/players/[playerId]/{recent,stats}` · `/api/fe/stats/summary` · `/api/fe/tokens`(GET/POST) · `/api/fe/tokens/[tokenId]`(DELETE).
- [ ] 캐시 태그·무효화를 architecture §5-2·§5-3 에 맞춰 추가.

### 완료 기준 / 검증

- `bun test` 전체 통과. Route Handler 통합 테스트는 `new Request(...)` 직접 호출.
- 리플레이 왕복: 업로드 → `GET /api/replays/{id}` 의 `events[].t_us` 가 **입력과 바이트 동일**(µs 라운딩 0).
- settings PUT 이 **204** 이고 본문이 비어 있다(클라 `put_no_content` 계약).
- `GET /api/courses/{hash}/ranking?limit=` 이 raw 배열, `/api/fe/*` 는 전부 `{success,data,pagination?}`.
- 낙관적 잠금: 어긋난 `base_updated_at` → 409 + `{conflict:true, server:{...}}`.

---

## W2b — ui-integration (MSW 제거 · 확장 화면 · 인증 흐름)

**전제**: W1b 완료 + (해당 화면에 대해) W1a/W2a 엔드포인트 완료.

### 파일 소유권 (배타)

```
mocks/**  (제거)
src/app/(app)/{tables,courses,replays,settings,admin}/**
src/widgets/{replay,course,table,settings,admin}/**
src/features/{inspect,dialog}/**
tests/components/{replay,settings,admin}*.test.tsx
```

> W1b 소유의 기존 페이지(`/`·`/search`·`/charts`·`/players`·`/leaderboards`)는 **MSW → 실 API 전환 시에만** 수정하며, 그 전환은 W1b 담당이 마무리한다(파일 소유권 유지).

### 작업

- [ ] MSW 제거, `entities/*.query.ts` 를 실 엔드포인트에 연결. 응답 타입 대조.
- [ ] 인증 흐름: better-auth 클라이언트로 로그인/가입/로그아웃, 세션 기반 `(app)/settings`·`(app)/admin` 게이팅(세그먼트 layout 서버 확인 → `/login?next=` redirect).
- [ ] `/settings` 탭 4종: 프로필 / API 토큰(발급 Form Dialog + 폐기 Confirm Action, 평문은 1회만 표시) / 동기화 blob(목록·다운로드·읽기전용 뷰) / 라이벌.
- [ ] `/tables`·`/tables/[tableId]`·`/courses/[courseHash]`.
- [ ] `/replays/[replayId]` 리플레이 뷰어(§10-7 워터폴, 인라인 SVG, `var(--color-*)` 직접 참조, 자체 오버플로).
- [ ] `/admin/builds`(role=admin).
- [ ] mutation: 토큰 발급/폐기·라이벌 저장·표 등록·빌드 등록. 에러 토스트는 **전역 `mutationCache.onError`** 로 위임, 성공 토스트만 컴포넌트.
- [ ] `/guide` 본문 확정: `--server https://bms.hyuns.uk/api` · 토큰 발급 절차 · guest 정책 · unranked 조건.

### 완료 기준 / 검증

- `bunx tsc --noEmit` · `bun test` 전체 통과.
- 라이트/다크 실렌더로 전 화면 확인. components.md §6 제약 10항 위반 0.
- 로그인 → 토큰 발급 → 그 토큰으로 `rbms-player` 제출 → 웹 리더보드에 즉시 반영(`updateTag`) 되는 것을 실제로 확인.
- `mocks/` 디렉터리가 남아 있지 않다.

---

## 1. 의존 순서

```
W0 부트스트랩
   ├── W1a api-core ──────┬── W2a api-ext ──┐
   └── W1b web-ui (MSW) ──┴─────────────────┴── W2b ui-integration
```

- W1a 와 W1b 는 **완전 병렬**(파일 교집합 0, W1b 는 MSW 로 자립).
- W2a 는 W1a 완료 후. W2b 는 W1b + (W1a·W2a) 완료 후.

## 2. 공통 검증 명령

```
bunx tsc --noEmit
bunx prettier --check "src/**/*.{ts,tsx,css}"
bun test
bun run build            # DB 미접속 상태에서 성공해야 한다
```

계약 회귀 검증(W1a·W2a 필수):
```
cargo run -p rbms-player -- --server http://localhost:3000/api --player <id>
```

## 3. 금지 사항 (전 작업 공통)

- 다른 작업의 소유 파일 수정 금지. 필요하면 소유자에게 요청한다.
- `drizzle-kit push` 금지. `@ts-ignore`·`eslint-disable` 금지.
- 코드 주석 금지(영어 JSDoc 만). 이모지 금지. `any`/`enum` 금지. `useCallback`/`useMemo` 금지.
- barrel(`index.ts`) 금지. FSD 역방향 import 금지. `@server/*` 를 클라이언트 레이어에서 import 금지.
- 커밋은 사용자 지시가 있을 때만.
