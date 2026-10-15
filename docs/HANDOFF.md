# HANDOFF — 2026-10-14 세션 스냅샷

최종 갱신 2026-10-15 · 대응 커밋 `51f3a1e` + v0.1.4 draft. 이 문서는 항상 덮어쓴다. 이번 세션의 상세는 `docs/PROCESS.md` S18·S19, 결정은 `docs/acknowledge/README.md`.

복기 신뢰도: 2026-10-09 ~ 10-12(웨이브 1~13)는 대화가 두 번 압축되어 압축 요약·커밋·문서로 복기했다(중간). 2026-10-12 ~ 10-14(웨이브 14~20, 릴리스, 웹 배포, 인수인계)는 원문 기준(높음). 사실이 의심되면 이 문서보다 코드와 `git log` 를 믿는다.

## 1. 프로젝트 한 줄 정의

R-BMS 는 Rust 로 만든 BMS 리듬게임 플레이어(beatoraja 형식 Lua 스킨 호환 + 자작 모노톤 기본 스킨)와, 같은 저장소의 점수 서버 겸 웹사이트(`web/`, Next.js, https://bms.hyuns.uk)다.

## 2. 현재 목표

- 최종 목표: 사용자가 실제로 쓰는 배포 가능한 BMS 플레이어 + 웹(순위, 발광 난이도표, 내 폴더). 사용자는 "새 세션에서 더 이상 구현하거나 버그가 없도록" 남은 일을 정확히 끝내길 원한다.
- 현재 마일스톤(2026-10-15): 클라이언트 `v0.1.4` draft 게시 직전(점검·노트 끝, 게시는 사용자), 웹 운영 배포 `51f3a1e` 이후 최신. S18(空POOR 400·기록 동기화·계약 교차 테스트)과 S19(잠재 버그 전수 감사 4회와 수리, 입력 시각 판정, 렉 최적화, 앱 아이콘, 발광 표의 다른 사이트 흔적 제거)를 끝냈다. 남은 것은 (1) 사용자의 v0.1.4 실기 확인과 게시, (2) 실제 창 QA(K1~K8 포함), (3) 웹 2차.
- 직전 작업: 점수 제출이 모든 판에서 `400 VALIDATION_ERROR` 로 거절되던 버그(시드 1.79e18 > 2^53)를 클라이언트·서버 양쪽에서 고치고(`7095ef7`), 웹 재배포, `v0.1.3` 릴리스(run 38563811699, 서명·공증·티켓 확인 완료, draft 본문 작성), 그리고 이 인수인계 문서 작업(문서 4묶음 점검, ADR 15건, test-debt, bug 기록).

## 3. 완료 / 진행 중 / 미착수

### 3.1 완료(이번 세션, 커밋)

웨이브 단위 상세는 `docs/PROCESS.md`(현재 작업 절)와 `docs/history/2026-10-14-process-archive.md`, 결정은 `docs/acknowledge/README.md` 색인.

| 묶음 | 내용 | 주요 커밋 | 주요 파일 |
| --- | --- | --- | --- |
| 웨이브 13 | 판정선 필드 맨 아래, 판정 오프셋은 노트 표시만 옮김(설정 스키마 5) | 499993a, 6a4568f | `assets/skins/rbms-default/lua/play/*`, `apps/rbms-player/src/stage/play/mod.rs` |
| 웨이브 14 | 판정 오프셋 키 9/0, START(+SELECT)+턴테이블 1 단위, 모노톤 기본 스킨·색 옵션, 스킨 내보내기, 스크래치 역방향 키 출하 | 8051f60, bc40ec7, 2613afa | `stage/play/controls*`, `keyconfig.rs`, `skin_select/{export,adjustments}.rs` |
| 웨이브 15 | 앱 UI 무채색, 옛 `theme.ron` 템플릿 교체 | 4db1c53 | `crates/rbms-render/src/theme.rs`, `apps/rbms-player/src/assets/theme_file.rs` |
| 첫 릴리스 | 툴체인 고정, 서명 시크릿 점검, 빌드 전 자격 증명 확인, 로컬 검증 스크립트 | 9dbb8ff, 39afc3a, 287b80d, 2ad01aa | `.github/workflows/{release,signing-check}.yml`, `scripts/verify-*.sh` |
| 웨이브 16 | 기록 저장 점검(원인 SCRATCH AUTO), 리플레이가 설정을 덮던 결함, 목표 fallback, 설정 화면 재구성 | 4bed711 | `apps/rbms-player/src/target.rs`, `stage/settings*`, `crates/rbms-config/src/judge.rs` |
| 웹 17A·17B | 점검·디자인·표 스냅샷·폴더 설계, 골격·폴더 API·표 제공·수리, 발광 표 페이지·내 폴더 화면 | 1e56fb4, d38b6fb, 711b518 | `web/src/**`, `web/drizzle/0001_user_folders.sql`, `web/data/tables/insane1/*` |
| 웨이브 18 | SCRATCH AUTO 새 규칙(초록, 없는 노트, 저장·전송), 모든 판 저장·전송, 파란 건반+START, 설정 Enter, 기본 발광 표·MY FOLDERS·서버 주소 기본값·제출 차트 메타 | 8b11a17, 38153fc | `crates/rbms-play/src/session.rs`, `crates/rbms-store/src/score.rs`, `apps/rbms-player/src/{user_folders.rs,tablesrc/seed.rs}` |
| 웨이브 19·20 | 어시스트 코스 전송, SCRATCH AUTO 순위 반영 + 목록 토글(마이그레이션 0002) | ffdb7a3, 12a756d, cf40490, 0b04629 | `apps/rbms-player/src/course_ir.rs`, `web/drizzle/0002_scratch_auto_bests.sql`, `web/src/server/compose/best-scope.ts` |
| 웹 배포 | web-db 워크플로, 운영 DB 0000~0002 적용, 시험 계정 5개 삭제, Vercel Git 연결(Root web, Production dev), 첫 Git 배포 | baff560, d81eafc | `.github/workflows/web-db.yml`, `web/scripts/run-sql.ts` |
| macOS 배포 형식 | `R-BMS.app` + dmg(공증 티켓 부착), Finder `-psn_` 인자 무시 | bfa0cca | `scripts/macos-app.sh`, `scripts/macos-notarize.sh`, `apps/rbms-player/src/lib.rs`(`is_system_launch_arg`) |
| 400 버그 | 새 시드 2^53 미만(`fresh_layout_seed`), 서버 `seedSchema` 관대 | 7095ef7 | `apps/rbms-player/src/app_play.rs`, `web/src/server/dto/common.ts` |
| 릴리스 | v0.1.1·v0.1.2 draft 는 결함으로 삭제, v0.1.3 draft 준비 | cdd27a6, c3937f4, fb29d24 | `docs/ci-release.md` 2.3 |

### 3.2 진행 중

- 클라이언트 `v0.1.4` 게시 대기(release run 38696475211, 점검 결과는 `docs/ci-release.md` 2.3). 다음 한 줄: 사용자가 v0.1.4 로 한 판 쳐서 결과 화면의 `IR:` 줄과 기록 동기화(`SYNC RECORDS NOW`)를 확인 → GitHub Releases 에서 Publish.

### 3.3 미착수(우선순위는 8절)

- 문서 점검에서 나온 웹 KNOWN ISSUE(아래 8절 4~5). 계약 교차 테스트와 작은 코드 수리는 S18 에서 끝났다.
- 실제 창 QA 116개 ID(`docs/quality-assurance/2026-10-11-default-skin-manual-check.md` 맨 위 표, 먼저 볼 8개).
- 웹 2차(`docs/web/tasks.md`): 한 곡 한 행 보기, 카드 높이, 폴더 끌어 옮기기·2,000곡 가상화, 표 RSC 자료 축소, OG 이미지, 홈 대시보드.
- Windows 코드 서명(사용자: 나중에), 앱 아이콘(.icns), 설정 화면 컨트롤러 조작, SCRATCH AUTO 초록을 노트 그림 모양으로(GPU 셰이더), 턴테이블 노트만 있는 차트의 자동 종료, 게임 안 IR 순위 패널의 SCRATCH AUTO 토글.
- 오래된 백로그(웨이브 9): IR 순위 스킨 연결, customTimers, 24K 문서, BGA 비디오, songdb 노트 종류 수(D9).

## 4. 의사결정 요약

정본은 `docs/acknowledge/README.md`(색인)와 각 ADR. 아래는 요지와 기각 이유.

| 결정 | 채택 | 기각된 대안과 이유 | ADR |
| --- | --- | --- | --- |
| SCRATCH AUTO | 턴테이블 노트는 초록·판정에서 빠진 없는 노트, 최고 기록·이력·리플레이·전송, 램프 LIGHT ASSIST, 옛 37판은 램프만 | "SC 점수 빼고 별도 최고 기록 행" — 사용자가 재정리해 대체. 결정 12(어시스트는 램프만) — 사용자 "당연히 갱신" | `2026-10-14-adr-scratch-auto-rule.md` |
| 저장·전송 | 사람이 친 판은 전부. 오토플레이·리플레이 보기·연습 제외. 넓힌 판정은 최고 기록·순위만 제외 | — | `…-save-and-send-every-played-run.md` |
| 순위 | SCRATCH AUTO 도 순위, 목록 기본 숨김 + 토글 | 순위에서 빼기(A) — 사용자 B | `…-scratch-auto-ranking-toggle.md` |
| 어시스트 코스 | 코스로도 전송, 서버는 단일 곡 정책 | 코스 전송 안 함 — 사용자 B | `…-assisted-course-submission.md` |
| 목표 | TARGET FALLBACK(기본 RATE AAA), RATE CUSTOM | 조용히 MAX 로 물러나기 — 무엇을 쓰는지 알 수 없었음 | `…-score-target-fallback.md` |
| 기본 표 | 스냅샷 커밋·사본 주소·실행 파일 내장·한 번 등록 | DB(table_chart) 저장 — sha256 필수라 md5 만인 표 불가. 원본 주소 등록 — 사용자가 사본 선택 | `…-default-insane-table.md` |
| 폴더 | 전용 읽기 API, MY FOLDERS, v1 비공개 | 표준 표 주소로 읽기 — 비공개 자료를 주소 비밀에 기댐 | `…-user-folders.md` |
| 운영 DB | web-db 워크플로 + `WEB_DATABASE_URL` | `vercel env pull/run` — Secret 은 못 받음. `!` 로 시크릿 입력 — 대화에 값이 남음 | `…-production-db-operations.md` |
| Vercel | Git 연결, Production `dev`, Root `web`, Preview 없음 | `prod` 를 웹 운영 브랜치로 — 앱 릴리스 버전 올림 브랜치와 겹침 | `…-vercel-git-deploy.md` |
| 릴리스 | 태그 + draft 자동, 게시는 사용자, 자격 증명은 빌드 전에 | 빌드 뒤 서명 점검 — 사용자 "오래걸리잖아" | `…-release-policy.md` |
| macOS 배포 | `R-BMS.app` + dmg, 앱·dmg 모두 티켓 부착 | 맨 실행 파일 tar.gz — 티켓을 붙일 수 없어 Gatekeeper 차단(v0.1.1) | `…-macos-app-bundle-dmg.md` |
| 시드 | 새 시드 2^53 미만, 서버는 범위 밖을 0 으로 | 서버만 고치기 — 정확한 시드를 잃음. 클라만 고치기 — 옛 빌드의 판 거절 | `…-layout-seed-range.md` |
| 툴체인 | 릴리스도 1.95.0 고정 | `@stable` — 타깃이 다른 툴체인에 설치되어 실패 | `…-release-toolchain-pin.md` |
| 그 이전 | D1~D18(스킨 호환), 웹 U1~U5·M1~M11·Q1~Q6 | 각 파일 참조 | `2026-10-09-lua-skin-compat-decisions.md`, `2026-10-14-web-decisions.md` |

## 5. 사용자 방향성 & 작업 규칙

- 응답: 한국어 존댓말, 간결, 첫 줄은 행동이나 답, 매 응답에 현재 상태 한 줄, 번호 목록, 이모지·글자 그림 금지. 확인하지 못한 것은 그렇다고 말한다.
- 세션 경계(새 세션, resume, compact, clear, prepare-new)와 새 실행 작업마다 다중 에이전트 workflow 사용 여부를 한 번 묻고 답을 기다린다(지적받은 적 있음). 선택은 승계하지 않는다.
- workflow 를 쓰면: Workflow 도구만(Agent 단독 금지), Fable·Haiku 금지(이 프로젝트), Opus 는 effort high 까지, Sonnet 은 xhigh 까지, 모든 agent 에 model·effort 명시. 에이전트는 커밋·푸시하지 않는다.
- 근본 원인을 고친다(증상 가리기 금지). 실제 측정·캡처·내려받은 산출물로 확인한 뒤 됐다고 한다.
- Git: Conventional Commits, 한국어 설명, author 사용자 단독, AI 트레일러 금지, 선별 스테이징(`git add -A` 금지), force push 금지. 게이트 통과 뒤 논리 단위로 커밋·푸시하고 `gh run list --branch dev` 로 3 OS CI 와 web CI 를 확인한다.
- Rust: `//` 주석 금지(`//!`·`///` 영어만), 매직넘버 금지, 새 `#[allow]` 금지, `unsafe`·`todo!()` 금지, rustfmt max_width 160, 새 의존성 금지, `--locked`.
- 웹: `web/AGENTS.md`(arrow function, any·enum·주석 금지, FSD, TanStack Query + queryOptions + QUERY_KEY, Zod 경계 검증, useCallback·useMemo 금지).
- 비밀: `.env`·`.env.*`·`secrets/`·키 파일을 읽거나 쓰거나 출력하지 않는다(훅이 막음). 시크릿 값이 대화에 남는 방법을 안내하지 않는다(별도 터미널의 `gh secret set` 이나 GitHub 웹). `vercel link` 는 `web/.env.local` 을 자동으로 만든다(사고 이력) — 쓰지 않는다.
- 운영: 운영 사이트에는 읽기 요청만. 운영 DB 쓰기는 web-db 워크플로로, 파괴적 작업(삭제)은 개수를 보여 주고 승인 뒤.
- 릴리스: 항상 태그 + draft 까지 자동, draft 본문은 사실을 확인해 정확히, 게시는 사용자. 결함이 든 draft 는 지우고 다음 패치 버전으로(v0.1.1, v0.1.2 선례).
- 폴더 금지: `/Users/hyunseokbyun/Downloads/ModernChic` 와 `/Users/hyunseokbyun/development/beatoraja` 를 고치지 않는다. ModernChic 자산을 저장소에 복사하지 않는다(D1).
- 사용자가 싫어한 것: 튀는 판정 애니메이션, 5 단위 걸음, 딱딱한 설정 화면, 빌드 뒤에 하는 시크릿 점검, 조사 결과 재탐색(조사 결과는 `docs/` 에 남긴다), 문서 미갱신.

## 6. 미해결 질문 / 사용자 확인 필요

1. `v0.1.4` 로 한 판 쳐서 `IR: RANK #n`, 기록 동기화, 키 입력 체감(JUDGE OFFSET 재조정), PAD DEBOUNCE 를 확인한 뒤 게시.
2. 웹 rate limit 의 지속형 저장소(Vercel Firewall 규칙 / Upstash / 지금대로) — 사용자 결정.
3. `user.rank_points`(항상 0) — 산식을 정할지, 응답에서 뺄지.
4. 알려진 위험: wgpu Metal 의 메인 스레드 위반(gfx-rs/wgpu#10627, `bug` 라벨, 사용자 결정으로 유지). wgpu 를 올릴 때 확인.

## 7. 환경 & 전제

- 저장소 `/Users/hyunseokbyun/development/R-BMS`, 기본 브랜치 `dev`, 원격 `B-HS/R-BMS`(공개). Rust 1.95.0(`rust-toolchain.toml`), 웹은 bun 1.4.0(`web/package.json` packageManager).
- 게이트: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`, `cargo test --workspace --no-fail-fast --locked`(웨이브 20 기준 5,671 통과), 웹 `cd web && bun run verify && bun run format:check && bun run build`(415 통과), 기본 스킨 `cargo run -q -p rbms-cli --locked -- skin-dump assets/skins/rbms-default --strict --frames 120`, 생성 이미지 `uv run assets/skins/rbms-default/tools/generate-images.py --check`.
- CI: `ci.yml`(Rust 3 OS + fmt/clippy, `web/**` 만 바뀐 푸시는 건너뜀, 표 스냅샷 변경은 돎), `web.yml`, `release.yml`(수동 `gh workflow run release.yml --ref dev -f bump=patch` 또는 `prod` 푸시·태그), `signing-check.yml`(시크릿만 확인), `web-db.yml`(`-f action=status|migrate|cleanup-count|cleanup-delete`).
- 저장소 시크릿: `MACOS_CERTIFICATE_P12`, `MACOS_CERTIFICATE_PASSWORD`, `APPLE_ID`, `APPLE_APP_SPECIFIC_PASSWORD`, `APPLE_TEAM_ID`, `WEB_DATABASE_URL`. Apple 개발자 계약 동의가 끝나야 공증이 됨(2026-10-13 사례).
- Vercel: 프로젝트 `rbms-web`(b-hs-projects), Git 연결, Root `web`, Production `dev`, `web/vercel.json` 의 ignoreCommand(프리뷰는 건너뜀, production 은 `web/` 이 바뀔 때만). 환경 변수 6개(`DATABASE_URL`·`BETTER_AUTH_SECRET` 은 Secret, `BETTER_AUTH_URL`·`ALLOW_GUEST`·`REPLAY_MAX_BYTES`·`NEXT_PUBLIC_APP_URL`). 로컬 Vercel CLI 는 `b-hs` 로그인, `web/.vercel` 링크(무시됨). 지금 배포는 `7095ef7`.
- 운영 DB: 마이그레이션 0000~0002 적용(2026-10-14), 시험 계정 삭제, 다른 사용자 점수 3건. GitHub 러너에서 접속됨.
- 사용자 실제 기록 DB: 옛 규칙(세대 2) SCRATCH AUTO 37판 → 램프·이력만.
- 로컬에 `x86_64-apple-darwin` 타깃이 설치돼 있음(필요 없으면 `rustup target remove x86_64-apple-darwin`).
- macOS 는 파일 시스템이 대소문자를 구분하지 않으므로 `docs/ARCHITECTURE.md` 를 따로 만들지 않는다. 아키텍처 정본은 `docs/architecture.md`.

## 8. 다음 세션 TODO(우선순위)

1. 사용자 확인(6절 1). 실기에서 나온 문제를 고친다.
2. 실제 창 QA: `docs/quality-assurance/2026-10-11-default-skin-manual-check.md` 맨 위 "먼저 확인할 것" 8개, 3.18 의 K1~K8(키 입력·화면 대기 스레드·Main Thread Checker), 116개 표.
3. 테스트 부채 `docs/quality-assurance/2026-10-14-test-debt.md` 3.x 의 남은 항목(반대 방향 계약 3.12, 장치 섞기 3.55, 같은 판 동시 중복 제출 3.72, CLI 읽기 명령 부작용 3.73, 표 업로드 중복 키 3.74 등).
4. 클라이언트 한계: 설정 화면 컨트롤러 조작, 게임 안 IR 순위 패널의 SCRATCH AUTO 토글, 턴테이블 노트만 있는 차트의 자동 종료, 초록 노트를 그림 모양으로.
5. 웹 2차(`docs/web/tasks.md`), Windows 서명(사용자가 원할 때), `docs/PROCESS.md` 의 "보존 이력" 블록 정리.

## 9. 문서 지도

- `docs/HANDOFF.md` — 이 문서. 새 세션의 진입점.
- `docs/PROCESS.md` — 진행 중 작업의 체크리스트와 웨이브별 결과(S1~S14). 완료 구간은 `docs/history/2026-10-14-process-archive.md`.
- `docs/architecture.md` — 아키텍처 정본: 저장소 배치, 크레이트 의존, Stage, 플레이 → 기록 DB → 점수 서버 → 웹 흐름, 배포.
- `docs/crates.md` — 크레이트별 공개 API·모듈·테스트.
- `docs/development.md` — 개발 명령, 환경 변수, 영속 파일, web 명령.
- `docs/play-options.md` — 플레이 옵션, 조작, 기록이 남는 조건(10절), 목표(11절).
- `docs/skin.md` — 스킨 엔진, 기본 스킨, SKIN 메뉴, 내보내기.
- `docs/theme.md` — 앱 UI 테마(`theme.ron`).
- `docs/ci-release.md` — CI, 릴리스 흐름과 이력(v0.1.0~v0.1.3), 서명·공증, web-db, KNOWN ISSUE.
- `docs/roadmap.md` — 오래된 로드맵(현재 정본 아님).
- `docs/acknowledge/README.md` — 결정 색인(ADR 15건 + 이전 결정 파일의 상태).
- `docs/plan/` — 설계서: `2026-10-14-settings-screen.md`, `2026-10-14-web-user-folders.md`, `2026-10-11-{play-menu,skin-menu,default-skin-design}.md`, `2026-10-09-lua-skin-compat.md` 등.
- `docs/web/` — 웹: `architecture.md`, `components.md`, `tasks.md`(남은 일), `2026-10-14-audit.md`(점검과 해소 현황), `2026-10-14-design-direction.md`, `2026-10-14-insane-table.md`, `2026-10-14-foundation-contract.md`(화면·API 계약).
- `docs/backend/` — 점수 서버 API 명세(`api-spec.md`), 데이터 모델, 호환성.
- `docs/reference/` — 원본(beatoraja) 조사: `skin-compat/README.md` 색인과 보고서, `ir-api.md`(클라이언트 IR 와이어).
- `docs/quality-assurance/` — `2026-10-11-default-skin-manual-check.md`(실제 창 QA 116개), `2026-10-14-test-debt.md`(테스트 부채와 교차 테스트 설계).
- `docs/bug/2026-10-14-score-submit-400-large-seed.md` — 400 사고 기록.
- `docs/feedback/` — 지적받은 것(CI 미확인 등). `docs/memory/` — 장기 기억(시험 곡 라이브러리).
- `docs/history/` — 이전 스냅샷과 이력.
