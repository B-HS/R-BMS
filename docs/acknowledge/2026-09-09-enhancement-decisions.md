# 고도화 계획 결정 (2026-09-09)

> 근거 문서: `docs/plan/2026-09-09-enhancement-plan.md` §3. 사용자 회신: "결정 필요한 건 전부 다 추천안으로 진행".

| # | 결정 | 채택안 |
|---|---|---|
| 1 | 스킨 포맷 전략 | 레퍼런스 구현 JSON 스킨 호환(json5 관대 파서) + Lua 식 평가(`mlua`, 스킨 루트 샌드박스, 노출 API 화이트리스트). LR2 CSV는 후순위 선택 |
| 2 | 판정 규칙 변경 시 기존 기록 | `scores.ron`에 `rule_version` 필드 추가, 구버전 기록 유지 + 램프 비교에서 별도 표시 |
| 3 | 시작 전 옵션 UX | 곡선택 오버레이 옵션 패널(홀드형) 주경로 + 전체화면 설정은 환경설정으로 축소. decide 화면 미도입 |
| 4 | IIDX 31 폐지 옵션 | H-RANDOM·JUDGE WIDTH 유지(레퍼런스 구현 패리티 우선) |
| 5 | 곡DB | `rusqlite`(bundled), 레퍼런스 구현 `songdata.db` 유사 스키마 |
| 6 | Play 중 Esc 즉시 이탈 | 유지 + "길게 누르기/2회 누르기" 옵션 추가 |
| 7 | 입력 장치 범위 | 게임패드(gilrs) + 스크래치 2키/아날로그 먼저, MIDI 후순위 |
| 8 | git 운용 | 수동(요청 시에만 커밋·푸시). `llm-rules.auto-commit` 미설정 유지 |
| 9 | 라이브러리 규모 | 사용자 값 미회신 → 계획 §2 Phase G 순서를 기본값(수천 곡 가정)으로 진행. 값 회신 시 재조정 |
| 10 | bmson·`#SWITCH` | `#SWITCH/#CASE/#SKIP/#DEF`는 Phase A-core에 포함, bmson은 Phase G 별도 항목 |
| 11 | 별도 런처 | 신설하지 않음. 인게임 환경설정으로 통합 |
| 12 | 커스텀 판정·어시스트 기록 정책 | 레퍼런스 구현 정책 그대로 이식: 판정폭 rate>100 또는 LN 마진 rate>100 → assist=2·score=false(IR 제출·리플레이 저장·EX/BP/콤보 갱신 차단, 램프·플레이카운트는 갱신); assist>0 → FC/PERFECT/MAX 생략 + LightAssistEasy(id 3)/AssistEasy(id 2) 강등; AUTO SCRATCH=assist 1; autoplay/replay/practice는 기록·제출 제외 |

## Phase A 중 확정된 세부 (2026-09-09)

- `#SWITCH` 계열 `#DEF` 의미: 단일 패스 해석기라 파일 순서로 판정 → 일치 `#CASE` 앞에 `#DEF` 가 있으면 `#DEF` 가 이긴다(C `default:` 와의 알려진 이탈, `control.rs` `select_case` 문서 주석에 명시). BMS 관례상 `#DEF` 는 마지막이라 실영향 희박. C 의미로 통일하려면 2패스 전환 필요 — 필요 시 사용자 판단.
- 그린넘버와 LIFT: 레퍼런스 구현(`LaneRenderer.java:321-330`)와 rbms 모두 lift 는 duration 에 개입하지 않음 → 계획 §1.7 의 "LIFT 미반영" 항목 철회.
- JUDGE WIDTH 클램프 대상은 MS 가 아니라 BAD(`JudgeProperty.java:266-268` 인덱스 6,7 = BD). 원본대로 구현.

## 운용 규칙(사용자 지시, 2026-09-09)

- 위임은 Workflow만 사용(Agent 단독 호출 금지). 병렬 가능한 작업은 항상 병렬로.
- 모델 배분: 구현·적대 검증·정밀 대조 = Opus, 기계적 수정(clippy/문서 경로/테스트 분리) = Sonnet, 종합 판단·머지·문서 정본 = Fable 직접.
- personal-llm 갱신 질문은 생략.

## 웹·백엔드 단독 서버 (사용자 지시, 2026-09-09)

| # | 결정 | 내용 |
|---|---|---|
| W1 | 형태 | **Next.js 단독 서버**(App Router + Route Handlers 로 API 구현). 별도 Hono 서버·별도 레포 계획은 폐기. 위치 이 레포 `web/`, 배포 Vercel, 도메인 `bms.hyuns.uk` |
| W2 | 인증 | better-auth(이메일+비밀번호, 세션 쿠키) + 네이티브 클라(rbms-ir)용 Bearer API 토큰(api-spec §2). OAuth 는 후속 |
| W3 | DB | MySQL. 접속 정보(호스트·포트·계정·비번·DB명)는 추후 `.env` 로 제공. ORM Drizzle(mysql2), 스키마 `docs/backend/data-model.md`, better-auth 가 인증 코어 테이블 소유(contract-freeze D-2) |
| W4 | 버전 | 전부 최신(Next.js·React·Tailwind v4·shadcn·TanStack Query v5·better-auth·Drizzle·zod). 설치 시 레지스트리 최신 확인 |
| W5 | 캐싱 | TanStack Query prefetch + HydrationBoundary, Next fetch cache(태그) + `revalidateTag`/Cache Components, 변경(제출 등) 시 태그 무효화 매트릭스 문서화 |
| W6 | UI | `docs/acknowledge/design.md` 토큰(OKLCH, Surface A = 앱 / Surface B = 공개 인증 페이지) 을 Tailwind v4 `@theme` 로 이식. shadcn(new-york) 컴포넌트를 **목록화**(`docs/web/components.md`)해 필요 시 CLI 로 추가. 사용자 친화 UI |
| W7 | 계약 | contract-freeze 권장안 동결: `/api` 프리픽스(클라는 `--server https://bms.hyuns.uk/api`), settings 경로 = 클라 경로, DTO 필드명 = 클라 필드명, 네이티브 = raw JSON, FE 전용 = envelope(`/api/fe/*`) |
| W8 | 가정(회신 없으면 유지) | 런타임·PM Bun / `web/` 라이선스 = 레포 GPL-3.0 / 이메일+비밀번호 우선 / Surface A·B 분리 |

### W0 설계 미결 6건 결정 (2026-09-09, Fable 판단·추천안)

| # | 미결 | 결정 |
|---|---|---|
| OQ1 | `user.id` 정체성 | better-auth 생성 id 를 PK 로 두고, 사람이 읽는 `login_id`(유니크, = 클라이언트 PLAYER ID) 컬럼 추가. `/api/players/{id}` 는 `login_id` 로 해석(유니크 인덱스 1회 조회) |
| OQ2 | 이메일 없는 계정 | 가입 시 이메일 필수(웹·네이티브 register 모두). 합성 이메일·username 플러그인 사용 안 함 |
| OQ3 | 레이트리밋 | v1 인메모리 best-effort. 남용 관측 시 Upstash 검토 |
| OQ4 | 리플레이 저장 | MySQL `mediumblob` + `REPLAY_MAX_BYTES` 상한. 스토리지 추상화는 유지(후속 Vercel Blob 전환 가능) |
| OQ5 | 리전 | Vercel `icn1` 가정. `.env` 의 DB 호스트 리전 확인 후 재조정 |
| OQ6 | `web/` 라이선스 | 레포 GPL-3.0-or-later 그대로(별도 LICENSE 없음) |
| env 키 | 문서 `MYSQL_*` 표기 | 코드(`env.ts`)가 정본: `DATABASE_URL` 또는 `DB_HOST/DB_PORT/DB_USER/DB_PASSWORD/DB_NAME`. architecture.md §6 을 이에 맞춰 정정 |
