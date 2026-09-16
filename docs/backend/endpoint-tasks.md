# rbms 백엔드 — 구현 상태와 남은 갭

> 이 문서는 `web/`에 구현된 Next.js Route Handler, Service, compose/Drizzle 경계를 기준으로 한 현재 상태입니다. 초기 설계 태스크는 2026-09-09에 완료됐으며, 과거의 미체크 목록은 이 파일의 정본이 아닙니다. 실제 키를 넣은 배포·운영 검증은 Phase R에서 진행합니다.

## 구현 완료

| 영역 | 구현 경로 | 상태 |
|---|---|---|
| 공통 | `src/server/{lib,dto,compose,route}/` | Zod 경계 검증, raw IR/FE envelope 응답, 중앙 오류, Bearer·세션 인증, 인스턴스 로컬 rate limit 구현 |
| system | `/api/health`, `/api/version` | capability와 빌드 버전 응답 구현 |
| auth | `/api/auth/{register,login,me,token}` 및 `/api/auth/[...all]` | better-auth 세션과 네이티브 Bearer 토큰 발급 구현 |
| players | `/api/players/[playerId]/{,rivals,scores,settings/[name]}` | 프로필, 라이벌, 기록, named settings blob 및 낙관적 잠금 구현 |
| charts / scores | `/api/charts/**`, `/api/scores/**` | md5/sha256 해석, 메타, ranking/best, 점수 제출·멱등·best·ranked 정책 구현 |
| replay | `/api/charts/[hash]/replays`, `/api/replays/[replayId]` | µs 이벤트 무손실 검증·저장·조회, score 연결 구현 |
| courses / tables | `/api/courses/**`, `/api/tables/**` | 코스 결과·메타·ranking/best 및 난이도표 조회/관리 구현 |
| FE | `/api/fe/**`, `(app)` 페이지 | 검색, 리더보드, 활동, 통계, 토큰, settings, replay viewer 조회 경로 구현 |
| admin | `/api/admin/builds` | 클라이언트 빌드 allowlist 조회/등록 구현 |

## 현재 제약과 후속

- `REPLAY_STORAGE`는 환경 스키마에 `db | blob` 값을 허용하지만 compose는 **`db`만 구현**합니다. `blob`을 선택한 요청 경로에서 replay compose가 생성될 때 명시적으로 실패하며, Vercel Blob 저장소는 아직 구현되지 않았습니다.
- Next 단일 오리진 서버이므로 CORS 미들웨어와 `ALLOWED_ORIGINS`는 구현하지 않았습니다. 네이티브 클라이언트는 브라우저 CORS 대상이 아닙니다. 외부 브라우저 오리진을 지원할 때에만 별도 설계·검증합니다.
- LR2IR XML 조회 어댑터와 리플레이 재시뮬레이션 기반 anti-cheat는 미구현입니다. 현재 ranked 정책은 제출 옵션·guest·빌드 신뢰도에 기반합니다.
- 운영 MySQL, better-auth 시크릿, 실제 도메인과 배포 키는 이 저장소에서 읽거나 기록하지 않습니다. 이를 넣은 배포 smoke test와 릴리스 자산 등록은 Phase R 범위입니다.

## 검증 기준

- 코드 변경 시 `cd web && bun run verify` 및 `bun run build`를 실행합니다.
- 네이티브 계약 회귀는 `crates/rbms-ir` HTTP mock 테스트와 `web` Route Handler 테스트가 담당합니다.
- 실제 키가 필요한 register/login/submit/replay/settings 배포 왕복은 [Phase R 수동 검증](../quality-assurance/2026-09-16-phase-h-manual-checks.md)에서 수행합니다.
