# rbms web — 구현 작업 분할 (W1 / W2)

> **역사 작업 계획:** W1/W2의 체크 항목은 2026-09-09에 완료됐습니다. 현재 구현 상태와 미구현 경계는 [../backend/endpoint-tasks.md](../backend/endpoint-tasks.md), 실제 라우트는 `web/src/app/api/`가 정본입니다. Phase R의 실제 키·배포 검증은 이 계획의 완료와 별개로 보류합니다.

> 근거: `docs/web/architecture.md`(라우트·계층·캐시·env), `docs/web/components.md`(shadcn 목록·토큰), `docs/backend/endpoint-tasks.md`(엔드포인트 체크리스트), `docs/acknowledge/2026-09-09-enhancement-decisions.md`(W1~W8).
> 원칙: **네 갈래 작업이 같은 파일을 쓰지 않는다.** 아래 "파일 소유권"은 배타적이며, 다른 작업의 소유 파일은 읽기만 한다.

---

## 0. W0-부트스트랩 (완료 이력)

> **상태: 완료(2026-09-09, 실파일 대조).** 불일치 3건 해소: `cacheComponents: true` 설정, `db:push` 스크립트 제거, 미사용 `cn` 패키지 제거(2026-09-09). 항목별 대응: design.md §16-3 테마 적용은 `next-themes` `ThemeProvider`(`storageKey = rbms-theme`) 자체 주입으로 완료 · `withAdmin` 은 `lib/with-session.ts` 에 동거 · `/guide` 서버 주소는 `shared/constants/server-info.ts`. 잔여는 완료 기준 중 **라이트/다크 실렌더 스크린샷 미확보**(하드코딩 색 0건은 grep 확인) — `docs/history/2026-09-09-web-nextjs-ir-server.md` 알려진 제약.

당시 W1a/W1b 병렬 작업에 앞서 이 단계를 순차로 완료했습니다.

- [x] `web/` 생성: `bun create next-app`(App Router · TypeScript · Tailwind v4 · src 디렉터리 · alias 미사용 → 직접 설정).
- [x] `package.json` 스크립트: `dev` `build` `start` `typecheck`(`tsc --noEmit`) `test`(`bun test`) `lint` `db:generate` `db:migrate` `auth:generate`.
- [x] `tsconfig.json` paths = architecture §2-1 의 6개 alias.
- [x] `next.config.ts`: `reactCompiler: true` + `cacheComponents: true` + `serverExternalPackages: ['mysql2']`.
- [x] `package.json` 에서 `db:push` 제거, `cn` 패키지 제거(자체 `@shared/lib/cn` 사용).
- [x] `prettier.config.js`: printWidth 150 · tabWidth 4 · semi false · singleQuote true · jsxSingleQuote true · trailingComma all · arrowParens always · bracketSameLine true · endOfLine lf.
- [x] shadcn 구성: `radix-nova` · neutral · CSS variables · lucide, `aliases.ui = @shared/ui`.
- [x] `src/app/globals.css`: design.md §16-1 기본 토큰과 §16-2 `[data-surface='public']` 델타 적용. §16-3 테마 적용은 `Providers`의 next-themes `ThemeProvider` 자체 주입을 사용.
- [x] `.env.example`: architecture §6-1 의 전 키.
- [x] `src/server/lib/env.ts`(getEnv + zod), `src/server/db/index.ts`(mysql2 풀 싱글톤 · `import 'server-only'`).
- [x] 빈 디렉터리 골격 생성(§2 트리) + `.gitkeep`.
- **완료 기준**: `bun run dev` 기동, `/` 가 빈 Surface A 셸로 렌더, `bunx tsc --noEmit` 통과, DB 미접속 상태에서 `bun run build` 성공.

---

## W1a — api-core (완료 이력)

**당시 목표**: 네이티브 클라(`rbms-player --server http://localhost:3000/api`)의 submit → ranking 왕복.

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

### 완료 기록

- [x] `db/schema.ts` — `data-model.md` **전 테이블**(chart · score · chart_best · replay · api_token · rival · setting_blob · course · course_chart · course_score · course_best · difficulty_table · table_folder · table_chart · table_course · client_build · submission_audit). W2a 가 쓸 테이블까지 **여기서 한 번에** 정의(W2a 는 스키마를 건드리지 않는다).
- [x] better-auth 설정(`server/lib/auth.ts`) + `auth:generate` → `db/auth-schema.ts`. `user.additionalFields` 로 IR 확장 필드.
- [x] `db:generate` → `db:migrate`. `push` 사용 금지.
- [x] `lib/error-code.ts` · `error-message.ts` · `error.ts`(STATUS_MAP · createAppError · isAppError). 코드: `IR_CHART_NOT_FOUND` `IR_PLAYER_NOT_FOUND` `IR_SCORE_NOT_FOUND` `IR_REPLAY_NOT_FOUND` `IR_SETTING_NOT_FOUND` `IR_COURSE_NOT_FOUND` `IR_TABLE_NOT_FOUND` `IR_ACCOUNT_EXISTS` `IR_PAYLOAD_TOO_LARGE` `RATE_LIMITED` `UNAUTHORIZED` `FORBIDDEN` `VALIDATION_ERROR` `SERVICE_NOT_CONFIGURED`.
- [x] `lib/api-response.ts`: `successResponse` · `paginatedResponse` · `errorResponse` · `irRaw`.
- [x] HOF: `withErrorHandling` · `withApiToken` · `withSession` · `withAdmin` · `withRateLimit`.
- [x] dto: `judgeBreakdownSchema` · `playOptionsSchema` · `scoreSubmissionSchema` · `chartUpsertSchema` · `rankingQuerySchema` · `accountSchema` · `loginSchema` · `chartMetaSchema` · `scoreRecordSchema` · `submitResponseSchema` · `playerProfileSchema` · `serverInfoSchema`.
  - **필드명은 `crates/rbms-ir/src/dto.rs` 그대로**. 누락 필드는 `.default()`/`.optional()`.
- [x] 엔드포인트: `GET /api/health` · `GET /api/version` · `POST /api/auth/register` · `POST /api/auth/login` · `GET /api/auth/me` · `POST /api/auth/token` · `GET/POST /api/auth/[...all]` · `POST /api/scores` · `GET /api/scores/[scoreId]` · `GET/POST /api/charts` · `GET /api/charts/[hash]` · `GET /api/charts/[hash]/ranking` · `GET /api/charts/[hash]/best` · `GET /api/players/[playerId]`.
- [x] 무결성: `rankedPolicy` · `build-allowlist` · `submission_audit` insert · `chart_best` 갱신(램프>EX>BP) · 멱등 처리.
- [x] 캐시: 읽기 서비스에 `'use cache'` + `cacheTag`(architecture §5-2), 제출 후 `revalidateTag`(§5-3).

### 당시 완료 기준 / 검증

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

## W1b — web-ui (완료 이력)

**당시 목표**: MSW 목으로 전 화면을 구성한 뒤 실제 API로 전환.

### 파일 소유권 (배타)

```
src/app/layout.tsx                 src/app/(app)/**   src/app/(public)/**
   (단 src/app/api/** 는 전부 W1a·W2a 소유 — 건드리지 않음)
src/widgets/**   src/features/**   src/entities/**   src/shared/**
mocks/**         tests/components/**
```

### 완료 기록

- [x] `shared/lib/cn.ts` · `shared/lib/fetch.ts`(`clientFetch` — 봉투 해석: FE 경로 envelope, 네이티브 경로 raw) · `shared/utils/get-query-client.ts`(staleTime 60s · gcTime 10m) · `shared/constants/query-key.ts`.
- [x] `QUERY_KEY` 계층: `CHART.{META(hash), RANKING(hash,params), BEST(hash,player), SEARCH(params)}` · `PLAYER.{PROFILE, SCORES, RECENT, STATS, RIVALS}` · `SCORE.DETAIL` · `REPLAY.DETAIL` · `COURSE.*` · `TABLE.*` · `STATS.SUMMARY` · `ACTIVITY.RECENT` · `LEADERBOARD.PLAYERS` · `TOKEN.LIST` · `AUTH.SESSION`.
- [x] `entities/*.query.ts` — 각 도메인의 `*QueryOptions` 팩토리 + `use*` 훅. `'use client'` 명시. **인라인 queryKey 금지**.
- [x] `entities/*.type.ts` — 응답 타입. 가능한 한 zod 없이 `dto.rs` 형태를 그대로 반영(W1a 의 zod 스키마와 중복 정의하지 않고, FE 는 필요한 필드만 타입 선언).
- [x] 셸: `(app)/layout.tsx` 3열(NavRail 256 / main fluid / FilterPanel 320, gap 1px, 헤더 없음) · `(public)/layout.tsx` 중앙 카드.
- [x] 컴포넌트: components.md §3 의 Panel Card · Stat Tile · Data Table · Status Badge(램프 매핑) · State Triad · Pager · Column Toggle · Panel Footnote · Bar-Count List.
- [x] shadcn 설치: components.md §4-1 목록.
- [x] 페이지(전부 MSW 데이터로): `/` · `/search` · `/charts/[hash]` · `/players/[playerId]` · `/leaderboards` · `/login` · `/signup` · `/guide`.
- [x] 서버 prefetch + `HydrationBoundary`(architecture §5-4). DB 를 읽는 영역은 `<Suspense>` 로 감싼다.
- [x] MSW 핸들러: architecture §1-2·§1-3 응답 형태를 그대로 흉내낸 뒤, 실제 응답 전환에서 픽스처를 교체했습니다.

### 당시 완료 기준 / 검증

- `bunx tsc --noEmit` · `bun test tests/components` 통과.
- 라이트/다크 **양쪽** 실렌더 확인(스크린샷). components.md §6-1 grep 2종 0건.
- 필터/검색/정렬/페이지가 전부 URL 에 직렬화되고, 새로고침으로 화면이 재현된다.
- 목록 로딩 시 EmptyState 깜빡임 없음(`isLoading` 가드 → 스켈레톤 → empty/list 순서).

---

## W2a — api-ext (완료 이력)

**당시 전제**: W1a 완료(스키마·lib·HOF·dto 공용부가 이미 존재). **`db/schema.ts`를 수정하지 않음.**

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

### 완료 기록

- [x] replays: `POST /api/charts/[hash]/replays`(µs 무손실 · `REPLAY_MAX_BYTES` 초과 413 · score 연결) · `GET /api/replays/[replayId]` · `GET /api/charts/[hash]/replays`. 저장은 DB만 구현됐으며 `REPLAY_STORAGE=blob`은 compose에서 명시적으로 실패합니다.
- [x] settings: `GET/PUT /api/players/[playerId]/settings/[name]`(본인 검증 · PUT 은 **200 `{ updated_at }`**(저장된 잠금 스탬프, epoch ms — 2026-09-09 계약 수정, 이전 204 계약은 클라가 하위호환 처리)) · 낙관적 잠금(`base_updated_at`) · `GET /api/fe/me/settings`.
- [x] rivals: `GET/PUT /api/players/[playerId]/rivals`. scores: `GET /api/players/[playerId]/scores`.
- [x] courses: `POST /api/courses`(= **결과 제출**, architecture §1-2 주의 1) · `POST /api/courses/meta` · `GET /api/courses/[courseHash]` · `POST /api/courses/[courseHash]/scores` · `GET .../ranking` · `GET .../best`.
- [x] tables: `GET /api/tables` · `GET /api/tables/[tableId]` · `POST /api/tables`(admin).
- [x] admin: `GET/POST /api/admin/builds`.
- [x] FE 전용(envelope): `/api/fe/charts/search` · `/api/fe/charts/[hash]/leaderboard` · `/api/fe/activity/recent` · `/api/fe/leaderboards/players` · `/api/fe/players/[playerId]/{recent,stats}` · `/api/fe/stats/summary` · `/api/fe/tokens`(GET/POST) · `/api/fe/tokens/[tokenId]`(DELETE).
- [x] 캐시 태그·무효화를 architecture §5-2·§5-3 에 맞춰 추가.

### 당시 완료 기준 / 검증

- `bun test` 전체 통과. Route Handler 통합 테스트는 `new Request(...)` 직접 호출.
- 리플레이 왕복: 업로드 → `GET /api/replays/{id}` 의 `events[].t_us` 가 **입력과 바이트 동일**(µs 라운딩 0).
- settings PUT 이 **200** 이고 본문이 `{ updated_at }` 이다(클라 `put_settings` → `SettingsPutResult`; 204 도 하위호환 허용).
- `GET /api/courses/{hash}/ranking?limit=` 이 raw 배열, `/api/fe/*` 는 전부 `{success,data,pagination?}`.
- 낙관적 잠금: 어긋난 `base_updated_at` → 409 + `{conflict:true, server:{...}}`.

---

## W2b — ui-integration (완료 이력)

**당시 전제**: W1b 완료 + 해당 화면의 W1a/W2a 엔드포인트 완료.

### 파일 소유권 (배타)

```
mocks/**  (제거)
src/app/(app)/{tables,courses,replays,settings,admin}/**
src/widgets/{replay,course,table,settings,admin}/**
src/features/{inspect,dialog}/**
tests/components/{replay,settings,admin}*.test.tsx
```

> W1b 소유의 기존 페이지(`/`·`/search`·`/charts`·`/players`·`/leaderboards`)는 MSW → 실 API 전환에서만 수정했고, 해당 전환으로 파일 소유권을 마무리했습니다.

### 완료 기록

- [x] MSW 제거, `entities/*.query.ts` 를 실 엔드포인트에 연결. 응답 타입 대조.
- [x] 인증 흐름: better-auth 클라이언트로 로그인/가입/로그아웃, 세션 기반 `(app)/settings`·`(app)/admin` 게이팅(세그먼트 layout 서버 확인 → `/login?next=` redirect).
- [x] `/settings` 탭 4종: 프로필 / API 토큰(발급 Form Dialog + 폐기 Confirm Action, 평문은 1회만 표시) / 동기화 blob(목록·다운로드·읽기전용 뷰) / 라이벌.
- [x] `/tables`·`/tables/[tableId]`·`/courses/[courseHash]`.
- [x] `/replays/[replayId]` 리플레이 뷰어(§10-7 워터폴, 인라인 SVG, `var(--color-*)` 직접 참조, 자체 오버플로).
- [x] `/admin/builds`(role=admin).
- [x] mutation: 토큰 발급/폐기·라이벌 저장·표 등록·빌드 등록. 에러 토스트는 **전역 `mutationCache.onError`** 로 위임, 성공 토스트만 컴포넌트.
- [x] `/guide` 본문 확정: `--server https://bms.hyuns.uk/api` · 토큰 발급 절차 · guest 정책 · unranked 조건.

### 당시 완료 기준 / 검증

- `bunx tsc --noEmit` · `bun test` 전체 통과.
- 라이트/다크 실렌더로 전 화면 확인. components.md §6 제약 10항 위반 0.
- Phase R 보류: 실제 키로 로그인 → 토큰 발급 → `rbms-player` 제출 → 웹 리더보드 반영을 배포 환경에서 수동 확인.
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

## 2. 당시 공통 검증 명령

```
bunx tsc --noEmit
bunx prettier --check "src/**/*.{ts,tsx,css}"
bun test
bun run build            # DB 미접속 상태에서 성공해야 한다
```

당시 계약 회귀 검증(W1a·W2a):
```
cargo run -p rbms-player -- --server http://localhost:3000/api --player <id>
```

## 3. 금지 사항 (전 작업 공통)

- 다른 작업의 소유 파일 수정 금지. 필요하면 소유자에게 요청한다.
- `drizzle-kit push` 금지. `@ts-ignore`·`eslint-disable` 금지.
- 코드 주석 금지(영어 JSDoc 만). 이모지 금지. `any`/`enum` 금지. `useCallback`/`useMemo` 금지.
- barrel(`index.ts`) 금지. FSD 역방향 import 금지. `@server/*` 를 클라이언트 레이어에서 import 금지.
- 커밋은 사용자 지시가 있을 때만.
