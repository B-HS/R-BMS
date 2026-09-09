# Phase W — web/ Next.js 단독 서버 + 웹 FE (2026-09-09)

> 결정: `docs/acknowledge/2026-09-09-enhancement-decisions.md` §웹. 설계: `docs/web/{architecture,components,tasks}.md`. 에이전트 산출물 원본: 세션 스크래치패드 `web/{scaffold,w1a,w1b,w2a,w2b1,w2b2}.md` + `review-*`·`fix-*`.

## 진행

| 단계 | 내용 | 검증 |
|---|---|---|
| W0 | 설계 문서 3종 ∥ 스캐폴드(Next 16.3.4·React 19.2.8·Tailwind 4.3.3·shadcn 4.21·Drizzle 0.45.2·mysql2·better-auth 1.7.3·TanStack Query 5.102.8·zod 4.5.4, bun 1.4). design.md §16-1 토큰을 `globals.css`에, §16-2는 `[data-surface='public']` 스코프, §16-3 pre-paint 스크립트 | typecheck/lint/test/build 통과, DB 없이 빌드 |
| W1 | api-core(21테이블 스키마+마이그레이션, error/api-response/HOF, zod DTO = `rbms-ir` 필드명, health/version/auth register·login·me·token/scores/charts/ranking/best/players) ∥ web-ui(FSD 뼈대, QUERY_KEY, queryOptions, 3열 셸, Surface A/B, 컴포넌트 아키타입, 페이지 8종, MSW) → 리뷰 2 → 수정 | 테스트 155, 빌드 19 라우트. 사용자 DB에 `drizzle-kit migrate` 적용(21테이블) |
| W2 | api-ext(replays µs·settings 204/409·rivals·player scores·courses 제출/meta/랭킹/베스트·tables·admin builds·`/api/fe/*` 10종·capabilities 갱신·계약 보정) ∥ ui-part1(인증 흐름·세션 게이트·설정 4탭·표·코스·리플레이 워터폴·admin) → ui-part2(MSW 제거, entities 타입을 서버 DTO 정본으로 재정렬, SSR prefetch origin 버그·204 처리 버그 수정, 실 DB 전 화면 확인) → 리뷰 3 → 수정 | 테스트 196 통과·2 skip, 빌드 35 페이지 |

## 실 DB 스모크 (Fable 직접, 2026-09-09)

- `/api/health` capabilities: ranking·player_best·rivals·courses·replays·tables·settings_sync·accounts = true, lr2ir_compat = false.
- 가입 201 + API 토큰 → guest 제출 201(unranked: guest) → 토큰 제출 rank 1 → 더 나쁜 점수 재제출 시 베스트 유지 → autoplay 제출 unranked → 랭킹 raw 배열·베스트 raw 객체·차트 메타.
- 미등록 차트: ranking `[]`, best `null`(계약 보정 1). 설정 PUT 204(본문 없음)·GET·`base_updated_at` 불일치 409 `{conflict, server}`. 리플레이 업로드 → 다운로드 `t_us` 바이트 동일, 목록 조회. FE envelope: 검색·리더보드·통계·플레이어 리더보드. 페이지 10종 200, 비로그인 `/settings` → `/login?next=` 리다이렉트.
- Rust 클라이언트 `ir_probe`: health OK(예제의 sha256 자리표시자 때문에 제출은 400 — 예제 데이터 문제).

## 알려진 제약 / 후속

- md5만 보내는 클라이언트의 제출은 400(`chart.sha256`): `chart` 테이블 PK가 sha256이라 스키마 변경이 필요. LR2IR 어댑터(Phase G)에서 처리.
- `judge.fast/slow/combobreak`·`options.scratch_auto`가 없는 축약 페이로드는 400. Rust 클라이언트는 항상 전체 필드를 보내므로 실영향 없음. 슈퍼셋 서버 원칙상 `.default()` 부여 검토.
- `REPLAY_STORAGE=blob`(Vercel Blob)은 인터페이스만. MySQL blob + `REPLAY_MAX_BYTES` 상한으로 운영.
- 라이트/다크 실제 브라우저 스크린샷 미확보(하드코딩 색 0건은 grep으로 확인). 리플레이 워터폴·Dialog 오버레이 대비는 사람이 확인.
- 코스 제출 `ranked`는 인증 여부만으로 판정(클라 `CourseSubmission`에 options 없음). `POST /api/tables`·`/api/admin/builds`는 admin 세션만(CI 토큰 후속).
- `user.total_plays` 컬럼은 실시간 count로 대체(배치 갱신 후속). 레이트리밋 인메모리(OQ3).
- 배포: Vercel 프로젝트 링크 후 환경변수(DB_*·BETTER_AUTH_SECRET·BETTER_AUTH_URL·NEXT_PUBLIC_APP_URL·ALLOW_GUEST·REPLAY_MAX_BYTES)를 Vercel에 등록해야 프로덕션이 동작. 도메인 `bms.hyuns.uk`.
