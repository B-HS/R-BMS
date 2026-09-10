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

## Phase E 착수 결정 (2026-09-10, 웨이브 0)

> 근거 문서: `docs/plan/2026-09-09-phase-e-spec.md` §2.2 / §9.1 / §11.2-5. 웨이브 1 이 막혀 있던 결정을 스캐폴드 단계에서 해소한다.

| # | 결정 | 채택안 |
|---|---|---|
| E1 | 드로우 콜 배치 병합 범위 | 계획 §2 E1 의 "per-texture draw call 배치 병합" 문구를 **사양 §2.2 의 "제출 순서 불변 + 연속 구간(run) 병합만 허용"** 으로 대체한다. 텍스처별 전역 수집은 z-order 를 깨므로 채택하지 않는다 (→ `reference-divergences.md` E-D1) |
| E2 | 골든 PNG 커밋 여부 (§9.1) | **A안 채택** — `crates/rbms-render/tests/golden/` 에 PNG 를 커밋한다. 판단 기준은 **바이트 예산(합계 200 KB 미만)** 이며, 코드 생성 텍스처(16x16~64x64) 장면이라 그 안에 들어갈 때만 유효하다. 초과하면 그 장면은 기존 `block_signature`/`signature_hash` 방식(B안)으로 남긴다. `block_signature` 는 어느 쪽이든 유지한다. **현황(2026-09-10): 4장, 합계 32 KB** — 착수 시점에 "3장 이하" 로 적었던 장수 제한은 예산과 무관한 임의 수치였으므로 예산 기준으로 정정한다 |
| E3 | `rbms-skin` optional 의존성 버전·라이선스 (§11.2-5) | `json5 = "1.3.1"`(MIT), `mlua = "0.12.1"` + `features = ["lua54", "vendored"]`(MIT, vendored Lua 5.4 도 MIT). 둘 다 레포의 GPL-3.0-or-later 와 호환. LuaJIT 이 아니라 **Lua 5.4** 를 쓴다(레퍼런스 스킨 식이 5.x 문법이고 JIT 이 필요한 부하가 아니다). 두 feature 는 **기본 on**(`default = ["json5", "lua"]`)이며, `lua` 를 끈 빌드는 Lua 식을 담은 스킨을 `SkinError::LuaUnavailable` 로 거부한다 |
| E4 | `SkinUserConfig` 영속화 위치 | 사양 §5.5 "기존 설정 파일 체계를 따른다" 에 따라 `rbms-config` 문서에 얹는다 → 웨이브 0 이 `crates/rbms-config/Cargo.toml` 에 `rbms-skin` 의존을 미리 넣었다. 웨이브 3 E-ui 가 루트 `Cargo.toml`·`rbms-skin/src/lib.rs` 재머지를 요청하지 않고 진행할 수 있게 하기 위함이다 |
| E5 | `rbms-skin` 의 `serde_json`·`rbms-model` 의존 | 사양 §5.2(strict JSON 먼저 → json5 폴백)와 §5.5(`SkinLoadOptions.mode: rbms_model::Mode`)가 요구하므로 스캐폴드에서 함께 선언한다. 같은 이유 — 웨이브 0 이후 이 두 파일은 아무도 수정하지 않는다 |

## Phase E 구현 결정 (2026-09-10, 웨이브 3 통합)

> 웨이브 3(E-screen ∥ E-ui)이 합류하면서 내려진 결정. 착수 결정(E1~E5)이 답하지 않은 것들이다.

| # | 결정 | 채택안 |
|---|---|---|
| E6 | SKIN 탭의 행 구성 | 항상 있는 5개 행(화면 타입 / 문서 / LOADED / RELOAD / RESET)은 `rbms_config::SETTINGS` 의 descriptor 표에 두고, 문서가 스스로 선언한 행(property·filepath·offset)은 `SettingRow::Skin` 으로 **RELOAD 위에 동적 삽입**한다. SKIN 탭만 길이가 고정되지 않는 탭이다 |
| E7 | 편집 중인 화면 타입의 영속화 | `Config.skin.screen`(= `SkinType` id)에 저장해 다음 실행에서도 같은 화면을 편집한다 |
| E8 | 사용자 설정의 저장형 | `rbms_skin::loader::SkinUserConfig` 에 `PartialEq` 가 없어 `Config` 의 왕복 비교가 성립하지 않는다 → `Config` 는 자체 `SkinCustomisation`(properties/filepaths/offsets)을 저장형으로 두고 로드 시 `SkinUserConfig` 로 변환한다 |
| E9 | 문서를 화면 타입별로 들고 있는다 | `SkinLibrary` 의 `loaded` 를 단일 슬롯이 아니라 **화면 타입 키의 맵**으로 둔다. SKIN 탭이 RESULT 를 편집하는 동안에도 브라우저는 자기 문서로 그려져야 하기 때문이다. 읽기마다 단조 증가하는 `build` 번호를 발급해, 컴파일된 화면(`SkinScreens`)이 그 번호가 움직였을 때만 다시 만든다 |
| E10 | 컴파일된 화면의 소유자 | `AppShared.skin_screens`(화면 타입별 `SkinScreen`). 화면(stage)보다 오래 살아야 한다 — 곡을 고르고 플레이하고 돌아오는 동안 브라우저의 텍스처를 두 번 디코드하지 않기 위해서다. 다시 만들 때는 반드시 이전 화면을 `release` 한 뒤에 만든다(빌드마다 새 텍스처 네임스페이스) |
| E11 | 플레이 화면의 문서를 고르는 축 | 런의 `Mode` → `rbms_skin::loader::mode_skin_type` 으로 역인출한다. `skin_type_mode` 와 같은 표를 반대로 읽으므로 둘이 어긋날 수 없고, 테스트가 왕복을 단언한다 |
| E12 | 스킨 빌드 경고의 보고 경로 | 첫 줄 + "(and N more)" 를 `notify(Level::Warn, ..)` 로 한 번만 올린다. 경고 수 전체는 SKIN 탭 LOADED 행이 이미 보고하므로 토스트를 오브젝트 수만큼 쌓지 않는다 |
| E13 | BGA 256 정사각 제약 해소 | 사양 14단계대로 `Canvas::set_bga` 와 `gpu::BGA_DIM` 을 삭제했다. 디코더는 파일 자체 해상도의 `DecodedImage { rgba, width, height }` 를 넘기고, 두 호출부(플레이·브라우저 커버)는 `Canvas::set_background(rgba, w, h, rect)` 를 쓴다 |

## 마무리 순서 (사용자 지시, 2026-09-10)

- 릴리스(prod 브랜치·보호·v0.1.0 태그·서명)는 **사용자가 자택 보관 키로 직접** 수행한다. 에이전트는 릴리스 단계를 착수하지 않는다.
- 각 Phase 에서 나온 **경미한 후속 항목은 Phase G 다음의 "H 경미 후속 일괄" 단계**에 모아 한 번에 처리한다(현재: 토스트·힌트 겹침, Phase B 실기 가청 확인).
