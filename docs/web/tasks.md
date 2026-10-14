# rbms web — 구현 작업 분할 (W1 / W2) 과 웹 남은 일

> 최종 갱신 2026-10-14 · 대응 커밋 7095ef7

> **역사 작업 계획 + 현재 남은 일:** W1/W2의 체크 항목은 2026-09-09에 완료됐고, 웨이브 17B~20 은 아래에 이어서 적었습니다. 새 세션이 이어서 할 웹 일은 맨 아래 "남은 일" 절이 정본입니다. 현재 구현 상태와 미구현 경계는 [../backend/endpoint-tasks.md](../backend/endpoint-tasks.md), 실제 라우트는 `web/src/app/api/`가 정본입니다. 운영 배포(Phase R)는 2026-10-14 에 끝났습니다.

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

---

## 웨이브 17B-1 (U2) — 서버: 마이그레이션, 사용자 폴더, 표 제공, 고장 수리 (2026-10-14)

근거: `docs/acknowledge/2026-10-14-web-decisions.md`(M1~M11), `docs/web/2026-10-14-audit.md`, `docs/plan/2026-10-14-web-user-folders.md`. 구현은 `docs/web/architecture.md`(1-3, 3-2, 4-5, 5-4, 6)와 `docs/backend/api-spec.md` 7.1~7.2 에 적었다.

- [x] drizzle `meta/` 복원(`.gitignore` 해제, 0000 의 `when` = 2026-09-01), `0001_user_folders` 생성, `scripts/db-migration-status.ts` 와 기준선 SQL
- [x] 사용자 폴더 API(편집 13개 + 클라이언트 읽기 2개), 한도·이름 검증·순서·표에서 가져오기·rate limit·에러 코드 4개(`IR_FOLDER_NOT_FOUND`/`NAME_TAKEN`/`LIMIT_EXCEEDED`/`CONFLICT`)
- [x] 표 제공: 표준 형식 `header.json`/`data.json`, FE 표 API 4개, 내 기록 일괄 조회, 차트 제목 읽기 보강(M8)
- [x] 점검 고장: 가입 id 서버 검증, `POST /api/charts` 덮어쓰기·상한, `POST /api/courses/meta` admin, `/api/auth/[...all]` 허용 경로만, `unstable_rethrow`, 리더보드 실제 값 정렬(M9), 순위의 `replay_id`, 보안 헤더, 점수 제출의 차트 메타 수용(M8), 환경 변수 없는 빌드의 오류 로그
- [x] 배포: `web/vercel.json` `ignoreCommand`, `.github/workflows/web.yml`, 문서
- [ ] 서버 컴포넌트의 자기 URL fetch 를 서비스 직접 호출로(프런트 소유 — 모든 `*-content.tsx`, `entities/*.query.ts`, `shared/lib/fetch.ts`)
- [x] `.github/workflows/ci.yml` 에 `paths-ignore: ["web/**"]`(web 만 바뀐 푸시에서 Rust 3 OS 빌드가 도는 낭비) — 웨이브 17B-1 통합에서 추가
- [x] Q2~Q6 의 답이 정해졌다(`docs/acknowledge/2026-10-14-web-decisions.md` 3절): 발광 표는 카드 격자, 폴더는 v1 비공개. (운영 DB 마이그레이션은 처음엔 사용자 로컬 실행으로 정했으나 사용자가 "migrate 도 너가 처리" 로 바꿔 `web-db.yml` 워크플로로 메인이 실행했다 — 아래 웨이브 20 절)
- [x] 웹 로그인·가입은 세션 전용 `POST /api/fe/auth/login`·`/api/fe/auth/register`(API 토큰을 만들지 않음). 게임용 `/api/auth/login`·`/register` 는 토큰을 발급한 채로 둔다
- [ ] 지속형 rate limit(Upstash 등) — 미착수, "남은 일" 로 이월
- [x] 운영 시험 데이터 정리(Q6 답 A) — 2026-10-14 `web-db.yml` 의 `cleanup-delete` 로 삭제(아래)

## 웨이브 17B-2 — 발광 표 화면, 폴더 화면, 사이트 리뷰 (2026-10-14)

근거: `docs/web/2026-10-14-foundation-contract.md`(13·14절이 이 웨이브의 화면·컴포넌트·검증), `docs/web/2026-10-14-design-direction.md` 3.3·4절, `docs/acknowledge/2026-10-14-web-decisions.md`(Q2=A 카드 격자, Q3=A 폴더 비공개, Q6=A 시험 데이터 정리).

- [x] U3 발광 표: `/tables/insane1`(카드 격자·레벨 묶음·상세 대화상자·필터와 주소 반영·출처 세 곳·`bmstable` 메타·내 기록 서버 미리 채우기·늦게 그리기), `/tables` 의 번들 표 카드, 차트 페이지 배지
- [x] U4 폴더: `/folders`, `/folders/[folderId]`(명시 저장·충돌 409·미저장 경고·표에서 가져오기·검색 패널), `AddToFolderMenu`, 로그인·가입·로그아웃의 사용자 묶음 캐시 정리, `FormDialog`·`ConfirmAction` 포커스 복귀
- [x] 리뷰 major 3건 수정: `useSessionUser` 의 하이드레이션 게이트(#418), 로그인 뒤 전체 이동(`next=/folders` 머묾), `ShellPage` 본문 자식 `shrink-0`(플레이어 패널 눌림)
- [x] 리뷰 minor 수정: 발광 표 "내 진행" 기본 접힘과 중복 제거(사이드바 진행·복사 버튼), 진행 패널의 레벨 링크 제거, 레벨 칩 한 줄, 카드의 레벨 반복 제거, 라이트 `--muted-foreground` 대비, 보호 경로 로그아웃의 콘솔 404, 쓰지 않는 export·쿼리 키 제거, `.rbms-level-header` 모바일 층
- [x] 운영 시험 데이터 정리 SQL: `web/scripts/cleanup-test-accounts.count.sql`(세기만), `.delete.sql`(한 트랜잭션). 실행은 `web-db.yml`(아래)
- [ ] 2차: 계약 문서 10절의 표(한 곡 한 행 보기, 폴더 끌어 옮기기·내 램프, 큰 폴더 가상화, 저장 원자성, RSC 자료 줄이기 등)

## 웨이브 18~20 과 운영 배포 (2026-10-14)

근거: `docs/acknowledge/2026-10-14-web-decisions.md`, 사용자의 SCRATCH AUTO 규칙(같은 날), [architecture.md](./architecture.md) 3-2·6-2·7, [../backend/api-spec.md](../backend/api-spec.md) 5·6·12.

- [x] 웨이브 18~19: SCRATCH AUTO 판 저장·전송, 어시스트 코스 전송(`ffdb7a3`)과 서버 저장(`12a756d`: 코스 제출의 `scratch_auto`·`assist`)
- [x] 웨이브 20(0b04629, 클라이언트 cf40490): SCRATCH AUTO 판을 ranked 로 순위에 올리고 목록은 기본 숨김 + `include_scratch_auto` 토글. 마이그레이션 `0002_scratch_auto_bests`(`chart_best`·`course_best`·`course_score` 의 `scratch_auto`). 순위 질의는 `server/compose/best-scope.ts`
- [x] 운영 DB: `.github/workflows/web-db.yml`·`web/scripts/run-sql.ts`(baff560). 저장소 시크릿 `WEB_DATABASE_URL`. `migrate` 로 0000~0002 적용(2026-10-14), `cleanup-delete` 로 시험 계정 5개 삭제
- [x] Vercel: 프로젝트 `rbms-web`, Git `B-HS/R-BMS` 연결, Root `web`, Production Branch `dev`, `vercel.json` 의 `ignoreCommand`. 첫 Git 배포 완료. `https://bms.hyuns.uk/api/version` 의 `commit` 이 7095ef7(2026-10-14 관찰)
- [x] d733a8f: 2^53 를 넘는 `seed` 를 거절하지 않고 0 으로 저장(`seedSchema`). v0.1.2 점수 제출 400 의 서버 쪽 수리. 클라이언트 쪽은 7095ef7(새 판 배치 시드를 2^53 미만으로)

## S19 서버 수리 (2026-10-15)

근거: 감사 결과 `scratchpad/s19/round1.json`(영역 account 의 서버 쪽), [../backend/api-spec.md](../backend/api-spec.md) 2·4·5·6·7.2·8·9, [architecture.md](./architecture.md) 4·5-4·6-1.

- [x] IR-1: `chart_best`·`course_best` 갱신을 조건부 문장 하나로(동시 제출이 낮은 판으로 덮지 못함). DB 테스트 `tests/db/best-race.test.ts`
- [x] IR-4: EX·`total_notes`·판정 수 일관성 검사, 코스 제출에 단일 곡과 같은 rate limit·본문 크기·빌드 해시 정책·감사
- [x] IR-5: 가입·로그인 오류 구분, 비밀번호 길이 상한 128, 운영 필수 환경 변수를 인증 첫 사용 시 검증
- [x] IR-8 서버: `DELETE /api/auth/token/current`, 로그인·가입 때 요청에 실린 이전 토큰 폐기(만료는 강제하지 않음)
- [x] IR-9: 서버 컴포넌트의 자기 URL fetch 제거 → `public-reads.ts` 직접 읽기
- [x] IR-10: `proxy.ts` 로 없는 차트·플레이어·코스·리플레이 상세를 404 로
- [x] IR-14: 순위 번호 정의 통일(공동 순위), 동점 정렬의 마지막 유일 키, `lnmode` 현재 동작 문서화(결정 대기: 테스트 부채 3.35)
- [x] IR-15: 설정 낙관적 잠금 조건부 쓰기, 폴더 동시 추가 직렬화, 폴더 저장 한 요청, 설정 이름 16개·쓰기 30/min, 리플레이 사용자별 2,000개·512MiB. 마이그레이션 `0003_replay_user_quota`(운영 미적용: 적용 순서는 [architecture.md](./architecture.md) 3-2)
- [x] 클라이언트 쪽 짝(Rust, 계정): 로그아웃 때 `DELETE /api/auth/token/current` 호출(`ScoreServer::revoke_current_token`, `app_network::revoke_token_in_background`), 로그인·가입 요청에 쓰던 토큰을 Bearer 로 싣기(`HttpScoreServer` 가 토큰이 있으면 모든 호출에 싣는다. 요청 헤더는 `http::tests` 가 확인)
- [ ] 클라이언트 쪽 짝(Rust, 기록): 413(`details.scope`)·409 구분

## 남은 일 (웹·서버, 새 세션이 이어서)

우선순위 순. 각 항목의 근거는 괄호 안 문서.

1. 사용자 확인 대기: 운영 서버에 한 판 쳐서 결과 화면에 `IR: RANK` 가 나오는지, 순위(`/leaderboards`)에 `bhs` 가 올라오는지. 안 오르면 `POST /api/scores` 의 400/401 원인을 본다(Vercel 로그). 계약 교차 테스트(다음 항목)가 재발 방지책이다
2. 계약 교차 테스트: Rust 쪽이 실제 클라이언트가 만든 요청 본문(일반, SCRATCH AUTO, 코스, 리플레이 업로드)을 JSON 파일로 뽑고 웹 테스트가 서버 zod 스키마로 통과시킨다. 관련: `crates/rbms-ir/src/contract_tests/fixtures.rs`, `web/tests/dto/ir-contract.test.ts` ([../reference/ir-api.md](../reference/ir-api.md) 5절)
3. (해소, S19) 없는 상세 주소가 HTTP 200: `proxy.ts` 가 존재를 확인해 404 로 재작성한다. 표 상세(`/tables/[id]`)는 아직 200 이다(번들 표라 보류). 배포 뒤 운영에서 읽기 전용 GET 으로 확인 ([architecture.md](./architecture.md) 5-4)
4. 웹 2차(사용자가 "나중에" 로 둔 것): 발광 표 한 곡 한 행 보기, 카드 높이 선택, 폴더 끌어 옮기기, 2,000곡 가상화, 표 RSC 자료 축소(`level`·`url_diff` 중복), 지속형 rate limit(서버 컴포넌트 자기 URL fetch 는 S19 에서 제거), OG 이미지, 홈 대시보드 재구성
5. 소소한 서버 부채: (해소, S19: 가입·로그인 실패 원인 구분 `auth.service.ts`, `BETTER_AUTH_SECRET`·`BETTER_AUTH_URL`·`NEXT_PUBLIC_APP_URL` 운영 필수 검증 `env.ts`) `release.yml` 이 `client_build` 를 등록하지 않음(admin 세션만 받는 `POST /api/admin/builds`), `user.rank_points` 열 미사용
6. 클라이언트 쪽(웹 문서 밖이지만 서버와 짝): 게임 안 IR 순위 패널의 SCRATCH AUTO 포함 토글(서버는 `include_scratch_auto` 를 이미 받는다)
7. 정책 확인 필요(메인이 정했고 사용자 미확인): 넓힌 판정 판은 저장·전송하지만 최고 기록·순위에서 제외, SCRATCH AUTO 램프는 `LightAssistEasy` 유지
