# rbms 백엔드 — 구현 상태와 남은 갭

> 최종 갱신 2026-10-15 · 대응 커밋 ea698a9

> 이 문서는 `web/`에 구현된 Next.js Route Handler, Service, compose/Drizzle 경계를 기준으로 한 현재 상태입니다. 초기 설계 태스크는 2026-09-09에 완료됐으며, 과거의 미체크 목록은 이 파일의 정본이 아닙니다. 운영 배포는 2026-10-14 에 끝났습니다(https://bms.hyuns.uk, Vercel `rbms-web`, 시험 계정 삭제). 운영 DB 에는 마이그레이션 0000~0004 가 모두 적용돼 있고(0003·0004 는 2026-10-15), 2026-10-15 관찰 시 운영 웹은 `51f3a1e` 입니다. 배포 설정의 정본은 [../web/architecture.md](../web/architecture.md) 6-2 절입니다.

## 구현 완료

| 영역 | 구현 경로 | 상태 |
|---|---|---|
| 공통 | `src/server/{lib,dto,compose,route}/` | Zod 경계 검증(DB 열 범위 포함, S19), raw IR/FE envelope 응답, 중앙 오류(프로덕션 `details` 는 `field`·`reason` 만), Bearer·세션 인증, 인스턴스 로컬 rate limit, 쓰기의 잠금 오류 재시도(`withLockRetry`, [data-model.md](./data-model.md) 설계 노트) 구현 |
| system | `/api/health`, `/api/version` | capability와 빌드 버전 응답 구현 |
| auth | `/api/auth/{register,login,me,token}`, `DELETE /api/auth/token/current`, `/api/auth/[...all]`(`GET /get-session`·`POST /sign-out` 만 통과), `/api/fe/auth/{login,register}` | better-auth 세션과 네이티브 Bearer 토큰 발급, 로그아웃 토큰 폐기, 재로그인 때 이전 토큰 폐기, 오류 구분(중복 409·자격 401·그 밖 500), 운영 필수 환경 변수 검증(S19). 웹 화면은 토큰을 만들지 않는 `/api/fe/auth/*` 를 쓴다 |
| players | `/api/players/[playerId]/{,rivals,scores,settings/[name]}` | 프로필, 라이벌, 기록, named settings blob 과 조건부 문장 하나의 낙관적 잠금(이름 16개, 쓰기 30/min, S19) 구현 |
| charts / scores | `/api/charts/**`, `/api/scores/**` | md5/sha256 해석, 메타, ranking/best, 점수 제출·멱등·best·ranked 정책 구현. SCRATCH AUTO 판은 ranked 이고 `chart_best` 를 `scratch_auto` 칸으로 나눠 저장, 순위·best 는 `include_scratch_auto` 로 포함 여부 선택(웨이브 20). 점수 제출은 `chart.title/artist/level` 로 차트 메타를 채우고, 2^53 를 넘는 `seed` 는 0 으로 저장(d733a8f). S18~S19: 노트를 소비한 판정만 세는 판정 수 검사(9K BAD 제외), 같은 초 멱등, EX·`max_ex_score`·`minbp`·램프의 본문 일관성 검사(`effectiveJudgeCounts`), `chart.notes` 를 상한에서 뺌, 최고 기록 조건부 갱신과 재전송의 최고 기록 복구, 공동 순위. 규칙 전문은 [api-spec.md](./api-spec.md) 4·5절 |
| replay | `/api/charts/[hash]/replays`, `/api/replays/[replayId]` | µs 이벤트 무손실 검증·저장·조회, score 연결, 같은 점수에 연결된 리플레이 재업로드의 멱등, 사용자별 2,000개·512MiB 한도(마이그레이션 0003) 구현 |
| courses / tables | `/api/courses/**`, `/api/tables/**` | 코스 결과·메타(admin)·ranking/best 및 난이도표 조회/관리 구현. 어시스트가 낀 코스도 코스로 받는다(`scratch_auto`·`assist` 필드, 웨이브 19~20). 코스 판정 합계 저장(마이그레이션 0004, S19). `POST /api/courses/meta` 는 admin 세션만 받는다 |
| FE | `/api/fe/**`, `(app)` 페이지 | 검색, 리더보드(`sort` = `cleared`·`played`·`plays`), 활동, 통계, 토큰, settings, replay viewer 조회 경로 구현 |
| admin | `/api/admin/builds` | 클라이언트 빌드 allowlist 조회/등록 구현 |
| folders / tables(번들) | `/api/fe/me/folders/**`, `/api/players/[playerId]/folders/**`, `/api/fe/tables/**`, `/tables/[tableId]/{header,data}.json` | 사용자 폴더(웹 편집·클라이언트 읽기, 폴더 행 잠금, 이름·설명·곡을 한 요청으로 저장), 번들 표의 표준 형식·검색·내 기록 구현 (웨이브 17B-1, S19). 표 응답에는 다른 사이트의 주소·출처를 싣지 않는다([api-spec.md](./api-spec.md) 7.1) |

## 현재 제약과 후속

- KNOWN ISSUE(알려진 미해결): 인스턴스 로컬 rate limit(`with-rate-limit.ts` 의 `Map`)이라 Vercel 다중 인스턴스에서 best-effort 다(지속형 저장소는 사용자 결정 대기). `user.rank_points` 는 쓰는 코드가 없어 항상 0 이다(처리 방법 사용자 결정 대기). 같은 판을 같은 밀리초로 동시에 두 번 보내면 한쪽이 `ER_DUP_ENTRY` 로 500 이고 재시도 밖 쓰기가 남았다(테스트 부채 3.72). 관리자 난이도표 저장의 중복 항목이 500 이다(3.74). 순위 `lnmode` 필터의 뜻은 사용자 결정 대기(3.35, [api-spec.md](./api-spec.md) 4절). 코스 램프 검사는 0.1.0 클라이언트 때문에 꺼 두었다(3.36). 웹 2차 후보와 함께 [../web/tasks.md](../web/tasks.md) 의 "남은 일"에 모았다.
- (해소, S19) 서버 컴포넌트의 자기 URL fetch, 가입·로그인 실패 원인 구분, 운영 필수 환경 변수 검증, 없는 상세 페이지의 200(지금은 404, 운영 GET 으로 확인 2026-10-15).
- (해소, e9aebae) 계약 교차 테스트: 플레이어 테스트 `apps/rbms-player/src/app_result/record_tests/contract_requests.rs` 가 실제 클라이언트 본문을 `contract/client-requests/*.json` 으로 뽑고 `web/tests/dto/client-requests.test.ts` 가 서버 zod 스키마로 통과시킨다(`crates/rbms-ir/src/contract_tests/fixtures.rs`·`web/tests/dto/ir-contract.test.ts` 는 각 쪽의 손으로 만든 고정 본문 테스트). [../reference/ir-api.md](../reference/ir-api.md) 5절.

- `REPLAY_STORAGE`는 환경 스키마에 `db | blob` 값을 허용하지만 compose는 **`db`만 구현**합니다. `blob`을 선택한 요청 경로에서 replay compose가 생성될 때 명시적으로 실패하며, Vercel Blob 저장소는 아직 구현되지 않았습니다.
- Next 단일 오리진 서버이므로 CORS 미들웨어와 `ALLOWED_ORIGINS`는 구현하지 않았습니다. 네이티브 클라이언트는 브라우저 CORS 대상이 아닙니다. 외부 브라우저 오리진을 지원할 때에만 별도 설계·검증합니다.
- LR2IR XML 조회 어댑터와 리플레이 재시뮬레이션 기반 anti-cheat는 미구현입니다. 현재 ranked 정책은 제출 옵션·guest·빌드 신뢰도에 기반합니다.
- 운영 MySQL, better-auth 시크릿, 배포 키는 이 저장소에서 읽거나 기록하지 않습니다(Vercel 환경 변수와 GitHub 저장소 시크릿 `WEB_DATABASE_URL` 에만 있다). 운영 DB 작업은 `web-db.yml` 수동 워크플로로 한다. 릴리스 자산(`client_build`) 자동 등록은 아직 없다: `release.yml` 은 sha256 을 계산만 하고 `POST /api/admin/builds` 를 부르지 않는다(admin 세션만 받는다).
- 실제 클라이언트의 점수 제출 왕복은 v0.1.2 에서 400(시드), v0.1.3 까지 400(空POOR 판정 수)으로 막혔고 서버를 d733a8f·865643f 로 고쳤다. 2026-10-15 운영에는 제출 119건·차트 96개가 있고(그중 49건은 2026-10-15 12:15~12:17 의 사고로 개발 빌드의 기록 동기화가 보낸 판이며 사용자 결정으로 남겨 둔다), 사용자 `bhs` 의 판은 모두 SCRATCH AUTO 판이라 기본 순위·최근 활동에는 보이지 않고 `include_scratch_auto=true` 에서 보인다(관찰: 기본 `played_charts` 0, 포함 91). v0.1.4 로 친 판의 결과 화면 `IR: RANK` 확인은 사용자 대기다.

## 검증 기준

- 코드 변경 시 `cd web && bun run verify` 및 `bun run build`를 실행합니다.
- 네이티브 계약 회귀는 `crates/rbms-ir` HTTP mock 테스트와 `web` Route Handler 테스트가 담당합니다.
- 실제 키가 필요한 register/login/submit/replay/settings 배포 왕복은 [Phase R 수동 검증](../quality-assurance/2026-09-16-phase-h-manual-checks.md)에서 수행합니다.
