# IR 계약 동결 + 백엔드 부트스트랩 결정 (Phase 4 게이트 — 사용자 확인 필요)

> 로드맵 Phase 4의 **선결 게이트**(open decisions D·E). 백엔드(별 MIT 레포, Bun+Hono+Drizzle+Zod+better-auth)는 그린필드라 코딩 전에 클라(`rbms-ir`)와의 **와이어 계약 3건 + envelope 정책 + 부트스트랩 2건**을 동결해야 재작업이 없다. 본 문서가 그 단일 출처.
> **라이선스 경계:** 백엔드/FE는 **별 저장소·MIT**(HTTP로만 통신하는 분리 작품). **GPL rbms 레포에 백엔드 코드를 두지 않는다.** 따라서 레포 생성·MySQL·인증 소유권은 사용자 결정/리소스가 필요해 본 문서로 준비만 한다.

## A. 클라가 실제 호출하는 경로 (검증: `crates/rbms-ir/src/http.rs`)
`base = --server 값(trailing / 제거)`, 각 호출은 `{base}{path}`:
```
GET  /health
POST /scores
GET  /charts/{md5}/ranking?limit=
GET  /charts/{md5}/best?player=
GET  /players/{id}
GET  /players/{id}/rivals
POST /courses
POST /charts/{md5}/replays            → ReplayId
GET  /courses/{hash}/ranking?limit=
GET  /replays/{id}
GET  /players/{id}/settings/{name}    → SettingsBlob
PUT  /players/{id}/settings/{name}    (204)
POST /auth/register
POST /auth/login
```
- 차트 키 = **md5**(클라). 스펙은 md5/sha256 양쪽 지원이므로 호환(서버가 길이로 판별).

## B. reconciliation 3건 (decision D)
1. **`/api` 프리픽스** — 클라는 `/health` 등 **미프리픽스** 호출, 스펙 서버는 `/api` 마운트.
   - **권장(코드변경 0):** `--server`에 마운트 경로 포함(`--server https://host/api`). `base="…/api"` + `get("/health")` = `…/api/health` ✓. NETWORK 탭 SERVER URL도 동일 규약. → `ir-api.md`·NETWORK 도움말에 "API base(예: …/api)까지 포함" 명시.
   - 대안: 서버를 루트 마운트(`/health`…). 권장 안 함(스펙·FE와 분리도 저하).
2. **settings 경로** — 클라 `/players/{id}/settings/{name}` vs 스펙 `/api/settings/{key}`(토큰 스코프).
   - **권장: 클라 경로를 계약으로 채택** — 백엔드가 `GET/PUT /api/players/{id}/settings/{name}`을 구현(본인=토큰 user.id 일치 검증, `name`=blob 키). 이미 출하 예정인 클라 변경 0. (스펙의 `/settings/{key}`는 FE 전용 별칭으로 추가 가능.)
3. **DTO 필드명** — 클라 `SettingsBlob.name` vs 스펙 `key`; thin 클라 `ReplayData` vs 스펙 `replayUploadSchema`.
   - **권장: 백엔드 Zod가 클라 필드명을 수용**(`name`·thin replay shape). 누락 필드는 Zod `.default()`/optional. 클라가 보내는 JSON이 곧 계약. 서버 내부 컬럼명(snake_case)과는 매핑.

> 원칙: **출하될 GPL 클라의 와이어 포맷을 계약의 기준**으로 삼아 클라 churn을 0으로(그린필드 백엔드가 맞춘다). 단 위 채택은 **사용자 확인 후 동결**.

## C. envelope 정책 (decision)
- **네이티브 클라 GET/POST는 RAW JSON**(현 `rbms-ir`가 envelope 미가정, `ScoreRecord`/`ChartMeta` 직접 역직렬화). 절대 envelope로 감싸지 말 것.
- **FE 전용 엔드포인트(검색·리더보드·활동·통계)는 `{success,data,pagination}` envelope**(FE 컨벤션 필수). `?envelope=1` 또는 `Accept`로 분기, 혹은 경로 분리(`/api/fe/*`).
- **권장:** 동일 리소스라도 네이티브=raw, FE=envelope. api-spec §0.1의 "v1 raw-only"를 **FE 한정 envelope 허용**으로 갱신.

## D. 부트스트랩 (decision E)
1. **MySQL** — 로컬 Docker(`mysql:8`, 개발) vs 호스팅(PlanetScale/RDS 등, 배포). **권장:** 개발=로컬 Docker compose, 배포=호스팅. `DATABASE_URL` env로 추상화(컨벤션 `getEnv`).
2. **user/session/account 소유권** — better-auth가 자체 테이블(user/session/account/verification)을 마이그레이션으로 소유 vs Drizzle 스키마가 소유.
   - **권장: better-auth가 인증 코어 테이블 소유**(better-auth Drizzle adapter), 도메인 테이블(chart/score/replay/…)은 `db/schema.ts`(data-model.md). FK는 user.id 참조.

## E. M0 부트스트랩 단계 (greenlight 시 — 별 MIT 레포)
1. 레포 생성(`rbms-server`, MIT LICENSE). `bun init`, deps: hono·@hono/zod-validator·drizzle-orm·mysql2·zod·better-auth.
2. 구조(컨벤션 `backend.md`): `index.ts`(compose→router→middleware→mount `/api`)·`route/`·`service/`·`dto/`·`db/`(index 싱글톤+schema)·`middleware/`·`lib/`(error 3파일·api-response·with-*·env)·`compose/`.
3. `lib/env.ts`(getEnv+Zod): `DATABASE_URL`·`BASE_URL`·`JWT_SECRET`·`ALLOW_GUEST`·`REPLAY_MAX_BYTES`·`NODE_ENV`.
4. M1 슬라이스: `/api/health`·auth(register/login/me/token)·chart upsert·`POST /api/scores`(chart auto-upsert·멱등·best)·ranking/best. **네이티브 RAW.**
5. 검증: `bun:test`(설명 한국어) + 실제 `rbms-player --server http://localhost:3000/api --player <id>`로 submit→ranking 왕복(PRD §8).

## 상태
**대기** — B(1·2·3)·C·D 동결 확인 후 별 MIT 레포에서 M0→M1 구현. 클라측 후속(SubmitResponse 확장·로그인 UI·랭킹 패널 = M2a)은 백엔드 M1 후 이 레포(GPL)에서.
