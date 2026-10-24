# HANDOFF — 2026-10-24 세션 스냅샷

최종 갱신 2026-10-24 · 대응 커밋 `0621e2f`(이 문서와 함께 커밋되는 문서 정리 커밋이 바로 뒤에 온다). 이 문서는 항상 덮어쓴다. 이전 스냅샷: `docs/history/2026-10-24-handoff-snapshot.md`(바로 앞 판, 2026-10-23 작업과 그 결정 표가 그대로 있다), 그 앞 `2026-10-23-`, `2026-10-22-`, `2026-10-14-`, `2026-10-11-`, `2026-10-09-handoff-snapshot.md`.

복기 신뢰도: 2026-10-24 세션은 clear 뒤 재개로 시작했고, 중간에 세션 한도로 한 번 끊겼다(W6 첫 실행). 결정·사용자 원문·커밋·운영 run 번호는 대화와 `git log`·`gh run` 으로 확인한 값이다. 하위 에이전트 보고서 원본은 세션 scratch(저장소 밖)에 있어 새 세션에서는 읽을 수 없고, 필요한 내용은 계획서·ADR·조사 문서·QA·테스트 부채에 옮겨 두었다. 사실이 의심되면 이 문서보다 코드와 `git log` 를 믿는다.

## 1. 프로젝트 한 줄 정의

R-BMS 는 Rust 로 만든 BMS 리듬게임 플레이어(beatoraja 형식 Lua 스킨 호환 + 자작 기본 스킨)와, 같은 저장소의 점수 서버 겸 웹사이트(`web/`, Next.js, https://bms.hyuns.uk)다.

## 2. 현재 목표

- 최종 목표: 사용자가 실제로 쓰는 배포 가능한 BMS 플레이어 + 웹(순위, 난이도표, 내 폴더, 코스·단위). 사용자는 남은 일을 정확히 끝내길 원하고, 가끔 실패하는 테스트도 없어야 한다고 했다.
- 현재 마일스톤: 2026-10-24 "다국어·리플레이 대전·랭킹 보기·NOTES RADAR·선곡 UI"(요구 5개)와 "어시스트 단위 A-단위" 를 끝내고 **클라이언트 v0.1.9 를 사용자가 게시했다**(Latest, 2026-10-24 03:59 UTC). 웹은 `dev` 푸시로 운영에 배포됐다. 사용자 원문 "배포다했고 뭐 draft -> release로 해놨고 잘 다되네". 진행 중인 구현 작업은 없다.
- 직전 작업(2026-10-24 세션, 시간 순):
  1. 컨텍스트 복원과 문서-코드 대조(동작 불일치 0, 표현 불일치 10건 수리 `bae5d86`), 이전 세션의 인수인계 커밋 `9fed3c0`.
  2. 요구 5개 조사(`docs/reference/2026-10-24-*.md`, `d2a636c`)와 통합 계획(`docs/plan/2026-10-24-five-features-plan.md`, `bca5306`), 결정 21개 사용자 답(`d6da51e`).
  3. 구현 웨이브 W1~W6(`72726c7`~`0434628`), CI 에서만 드러난 시험 수리 `69d86a7`.
  4. v0.1.9 릴리스(메인 실행 run 40055763447, 버전 커밋 `49fe493`, 한글 노트, 사용자가 받아 확인 뒤 게시).
  5. 추가 요청 A-단위: 설계 `53804d5`, 마이그레이션 0013 `35f77ae`, 서버 `bfb4078`, 웹 `34e21d1`, 문서 `0a73e87`, 운영 반영 기록 `0621e2f`.
  6. 이 인수인계.

## 3. 완료 / 진행 중 / 미착수

### 3.0 완료(2026-10-24 세션)

결정의 "왜"는 [acknowledge/README.md](acknowledge/README.md) 의 2026-10-24 절 ADR 8건, 체크리스트는 [PROCESS.md](PROCESS.md) 첫 두 체크리스트.

| 묶음 | 내용 | 커밋 | 주요 파일 |
| --- | --- | --- | --- |
| 확장 id | 정수·Float·Rate·텍스트·참조 이미지 공간 개방, 다섯 기능의 id 배정 | `72726c7` | `crates/rbms-skin/src/property/extension.rs`, `apps/rbms-player/src/skin_host/extension.rs`, `lua/props.lua` |
| 다국어 기반 | LANGUAGE(AUTO·영어·한국어·일본어, DISPLAY 맨 위), `translations!` 카탈로그, 용어집 | `b7efac1` | `crates/rbms-model/src/{language,translations}.rs`, `apps/rbms-player/src/{app_language.rs,text/**}`, `docs/reference/2026-10-24-i18n-glossary.md` |
| NOTES RADAR | 6축 계산(`rbms-chart`), 곡 DB `song_radar`, 참조 이미지 `-21020` 육각형 | `0e74d08`, `54ff02c` | `crates/rbms-chart/src/radar.rs`, `crates/rbms-library/src/songdb/**`, `crates/rbms-render/src/skin_render/graphs/radar.rs` |
| 랭킹 보기 | 쪽 받기·만료·무효화(RK-D), 웹 순위 `extra.dans`(RK-W), 파란 건반 300 ms 길게 = 랭킹 모드, RANKING KEYS, 오버레이, 외부 스킨 순위 슬롯 | `d727923`, `d74794b`, `423484b` | `apps/rbms-player/src/{ir_ranking,app_ranking,ir_ranking_view}.rs`, `stage/select/ranking_mode.rs` |
| 리플레이 배치 플레이 | 보는 중 J 키, 기록 모달·랭킹 줄의 PLAY THIS LAYOUT, 로컬 기록 고스트, 배치 골든 | `30264b8`, `eba8a73` | `apps/rbms-player/src/borrowed_layout.rs`, `stage/select/layout_play.rs`, `ghost.rs` |
| 선곡 값 | 커서 차트 분석 작업자(레이더·레인별 노트·그래프), 막대 등급 배지·썸네일, 형제 난이도·폴더 램프 분포·내 기록 6줄·순위 스크롤 | `d75b819`, `b5f15df` | `stage/select/{analysis,thumbs,browse_facts,ranking_scroll}.rs` |
| 기본 스킨 선곡 | IIDX 식 배치·토큰, 목록·머리·바닥, 정보창(CHART DATA, NOTES RADAR, MY SCORE, SCORE TARGETS, CLEAR RATE, MY RECORDS/IR RANKING) | `038481b`, `c0bebfa`, `90d57d1` | `assets/skins/rbms-default/lua/select/**`, `lua/theme.lua` |
| 번역 | 설정·렌더·메뉴·알림(`7c9bb0d`), 선곡·코스 막대 PRACTICE 키(`5014bb6`), 기본 스킨·확장 불린 `op` 로더 수리(`eadde84`), 남은 문구(`f82e6a9`) | 여러 | `apps/rbms-player/src/text/**`, `lua/i18n/**`, `crates/rbms-skin/src/loader/track.rs` |
| 웹 다국어 | next-intl 4.14.9 쿠키 방식(`eb5ddbb`), 화면 문구(`8d01908`, `c9404f7`, `b0cf509`), AI 결과 언어·오류 코드 번역(`0405496`) | 여러 | `web/src/shared/i18n/**`, `web/messages/{ko,en,ja}/*.json` |
| 문서 | ADR 7건, 사용자·스킨·웹 문서, QA 3.23, 테스트 부채 3.112~3.126, README 선곡 캡처 | `0434628` | `docs/**` |
| 릴리스 v0.1.9 | 메인 실행, 로그로 서명·공증 확인, 한글 노트, 사용자 게시 | `49fe493` | `docs/ci-release.md` 2.3 |
| A-단위 | 0013(`user_dan.assisted`, 기본 키 확장), 판정·표시·소급, 점선 배지, 마이페이지 선택, ADR·QA 3.24·부채 3.127~3.129 | `35f77ae`, `bfb4078`, `34e21d1`, `0a73e87` | `web/drizzle/0013_assisted_dan.sql`, `web/src/server/service/domain/dan/**`, `web/src/features/dan/**` |

운영 반영(2026-10-24): 마이그레이션 0013(사용자 `!` run 40074383695, status run 40074421795 최신), 소급 `dan-recompute`(사용자 `!` run 40074842429, A-단위 1개, 정식 0). 다섯 기능은 마이그레이션이 없었다.

마지막 게이트: Rust 7,129 통과·0 실패·6 무시(W5 병합 뒤, W6·A-단위는 Rust 무변경), 웹 1,981 통과·0 실패, DB 통합 161 통과(0013 포함), 스킨 덤프 세 언어 통과, 시트 검사 통과, ci 3 OS·web CI 통과(마지막 `0621e2f` 까지).

### 3.1 이전 세션(요약, 상세는 이전 스냅샷)

- 2026-10-23: Stella·코스·단위와 클라이언트 보완, Discord, v0.1.8(`docs/history/2026-10-24-handoff-snapshot.md` 3.0절).
- 2026-10-22 이전: `docs/history/2026-10-23-handoff-snapshot.md` 와 그 앞.

### 3.2 진행 중

없음(2026-10-24). 남은 것은 사용자 실기 결과와 아래 8절.

### 3.3 미착수

8절이 정본이다.

## 4. 의사결정 요약

정본은 [acknowledge/README.md](acknowledge/README.md). 2026-10-23 이전 결정 표는 이전 스냅샷 4절에 있다. 2026-10-24 결정(사용자 원문 "18번까지 전부 추천안대로 19번은 그럼 보류 20,21도 추천대로", A-단위는 "전부 추천대로 그리고 workflow도 사용"):

| 결정 | 채택 | 기각된 대안과 이유 | ADR |
| --- | --- | --- | --- |
| 다국어 | 앱·기본 스킨·웹 모두, 기본 AUTO, 게임 용어 원어, 한국어 chart = 차트, 일본어는 초안 + 감수 요청 | 웹 제외, 용어까지 번역 | `ui-language`, `web-i18n` |
| 웹 다국어 방식 | next-intl 쿠키 방식(경로 접두사 없음), 기본 한국어. 대가: 전 페이지 요청마다 렌더(메인 결정) | `app/[locale]` 라우트 이동(전체 이동이 커서 보류, 되돌리는 길로 ADR 에 기록) | `web-i18n` |
| 배치 플레이 | 옵션+시드만 빌림, 같은 배치 재시도에서만 유지, 일반 판으로 저장·전송, 로컬 기록은 고스트 | 게이지·SCRATCH AUTO·LN MODE 까지 복사, "배치 빌림" 표지 | `layout-play` |
| 랭킹 모드 | 2·4·6 길게(300 ms), 짧게는 지금 동작, 9키 버튼 2·8, 키보드 S·D, 오버레이 | 짧게 누르면 랭킹(컨트롤러로 폴더를 못 나옴), 별도 화면 | `ranking-mode` |
| NOTES RADAR | 공개 역산 자료(NOTES·PEAK) + 추정 3축(임시 스케일), 5·7키 SP·10·14키 DP, 9·24키 없음, 스캔 때 계산·저장 | 확실한 3축만, 9키 계산 | `notes-radar` |
| 선곡 UI | IIDX 식 구조, 바탕 모노톤 유지·강조색은 등급·램프·메달·레이더만, RIVAL 자리 = 내 기록 6줄(랭킹 모드에서는 IR 순위) | 전면 채색, 두 화면 유지 | `select-iidx-layout` |
| 곡 미러 | 보류(업로드·서버·버튼 만들지 않음) | 진행 | `song-mirror-prep` |
| 2026-10-23 미확인 13~18 | 지금대로 확정 | 바꿈 | 각 2026-10-23 ADR |
| 표 코스 막대 PRACTICE 키 | 원본처럼 코스 시작 | 그대로 둠 | `reference-divergences.md` |
| A-단위 | 어시스트 판(SCRATCH AUTO, ASSIST·LIGHT ASSIST 램프, 어시스트 옵션)을 따로 기록, 정식이 같거나 높으면 정식만, 더 높으면 둘 다, 마이페이지 선택 가능, 스테이지 정보 있는 판 소급, 서버가 `A-` 를 붙여 클라이언트 무변경 | SCRATCH AUTO 만, 항상 둘 다 | `assisted-dan` |

## 5. 사용자 방향성 & 작업 규칙

- 응답: 한국어 존댓말만(짧은 상태 보고 포함), 간결, 첫 줄은 행동이나 답, 매 응답에 현재 상태 한 줄, 번호 목록, 이모지·글자 그림 금지. 확인하지 못한 것은 그렇다고 말한다.
- 새 실행 작업과 세션 경계(새 세션, resume, compact, clear, prepare-new)마다 workflow 사용 여부를 한 번 묻고 답을 기다린다. 선택은 승계하지 않는다.
- workflow 를 쓰면: Workflow 도구만, Fable·Haiku 금지, Opus 는 effort high 까지, Sonnet 은 xhigh 까지, 모든 agent 에 model·effort 명시, 에이전트는 커밋·푸시 금지, 에이전트가 `rbms-player` 를 실행하면 scratch HOME 과 `--server http://127.0.0.1:9`. 웨이브는 worktree 격리로 병렬, 메인이 병합·게이트·커밋. 세션 한도로 끊긴 단위는 남은 worktree 에서 이어받게 한다(2026-10-24 W6 처럼 worktree 가 남지 않았으면 같은 계약으로 다시 실행).
- 사용자가 "끝까지 진행", "세션 한도 뒤 알아서 이어서" 를 맡기면 세션 안 Cron(매시 점검, 7일 만료)을 걸어 둔다. 창을 닫으면 사라진다.
- **병합과 부분 커밋(2026-10-23 사고 뒤)**: worktree 패치는 `git apply`(`--3way` 금지)로 합치고, 충돌은 `git apply --reject` 뒤 메인이 손으로. 단위별 커밋은 `git apply --cached <단위 패치>` 로 올린 뒤 `git diff --cached --name-only` 를 **별도 명령으로** 읽고 커밋한다. 확인과 commit·push 를 한 줄에 잇지 않는다([feedback](feedback/2026-10-23-stage-check-before-commit.md)).
- 결정을 맡기는 말("추천대로")이면 추천안으로 정하고 ADR 에 남긴다. 답이 없는 결정은 추천안으로 정하고 알린다.
- 운영 반영: 자동 모드가 `web-db.yml` 의 쓰기 action·운영 사이트 GET·`gh release download` 를 거부할 수 있다. 우회하지 말고 명령을 보여 `!` 실행을 요청한다. 읽기 action(`status`, `*-count`)과 `release.yml` 실행(2026-10-24 사용자 지시 "드래프트까지 다만들어놔")은 메인이 실행했다. 새 웹 코드가 새 마이그레이션에 기대면 마이그레이션 파일만 먼저 커밋·적용하고 코드는 뒤에 푸시한다(2026-10-24 0013 은 이 순서로 했다).
- 근본 원인 수정, 떠는 테스트 0, 실제 측정·캡처·산출물로 확인한 뒤 됐다고 말한다. 판정·점수 의미가 원본과 달라지는 수정은 고치기 전에 확인한다(원본과 같아지는 수정은 진행하되 "지금 R-BMS 대비 바뀜" 으로 보고).
- 사람이 친 판은 전부 저장·전송(노트를 하나도 치지 않은 판만 제외), 메인이 제외 조건을 추천하지 않는다.
- Git: Conventional Commits, 한국어 설명, author 사용자 단독, AI 트레일러 금지(세션 링크 트레일러도 넣지 않는다), 선별 스테이징, force push 금지, 논리 단위 커밋, 푸시 뒤 `gh run list` 로 3 OS ci 와 web CI 확인. 진행 중인 ci 가 있으면 새 푸시가 그것을 취소하므로 끝난 뒤 푸시한다.
- Rust: `//` 주석 금지(`///`·`//!` 영어만), 매직넘버·새 `#[allow]`·`unsafe`·`todo!()`·새 의존성 금지(예외 승인: zip·typed-path, 웹 next-intl), rustfmt 160, `--locked`. 웹은 `web/AGENTS.md`. 설정에는 `#[serde(default)]`, `skip_serializing_if` 를 새로 쓰지 않는다.
- 새 사용자 표시 문구는 처음부터 세 언어 카탈로그로(앱 `translations!`, 스킨 `lua/i18n/*.lua`, 웹 `web/messages/{ko,en,ja}`), 용어집을 따른다. 영어 문구를 바꾸면 기존 골든이 깨진다.
- 비밀 파일(.env*, `~/.config/rbms/settings.ron` 등) 접근 금지, 시크릿 값이 대화에 남는 방법 금지. ModernChic·beatoraja 폴더는 읽기만. 외부 게시(다른 저장소 이슈·댓글·PR)는 초안 승인 뒤에만.
- 사용자가 직접 하라고 한 외부 계정 작업은 Claude in Chrome 으로 하고 결과를 화면으로 확인해 보고한다.
- 다른 사이트의 흔적을 싫어한다. 단 Stella·Satellite·단위 코스는 출처 표기, 발광 insane1 은 흔적 제거 유지. 선곡 UI 는 IIDX 식이지만 IIDX 문구·로고·마스코트는 쓰지 않는다(NOTES RADAR 만 사용자 표현).
- 지우라는 요청은 무엇이 함께 사라지는지 개수를 보여 확인하고 지운다.
- README 는 TAIDE·raw-viewer README 처럼 짧게. 릴리스 노트는 한글.

## 6. 미해결 질문 / 사용자 확인 필요

1. **Vercel `CRON_SECRET`**: 2026-10-24 아침에 다시 확인을 부탁했고 답을 받지 못했다. 없으면 매일 KST 0시 Cron 이 503 으로 끝난다(사이트는 정상). 1~15자면 첫 `getEnv()` 호출이 던져 DB·인증 요청이 실패한다.
2. **곡 미러(결정 17)**: 2026-10-24 보류. 질문 Q-R1~R5·Q-C1~C3 는 `docs/plan/2026-10-23-song-mirror-prep.md` 3.3·11절.
3. **실기 결과**: QA 3.23(Y1~Y17, 다섯 기능), 3.24(Z1~Z7, A-단위), 3.20~3.22(v0.1.7·v0.1.8). 사용자는 "아마 다 문제없을거야", "잘 다되네" 라고 했지만 항목별 결과는 받지 않았다.
4. **일본어 감수**: 앱·스킨·웹 일본어는 모델 초안이다. 원어민 감수 요청은 하지 않았다(외부 게시는 초안 승인 뒤).
5. 이전 세션의 열린 질문(이전 스냅샷 6절 4·6): 표 코스 등록 수 숫자 확인, 순위 `lnmode` 필터 뜻 등 미확인 목록 1~12(`docs/acknowledge/README.md`).

## 7. 환경 & 전제

- 저장소 `/Users/hyunseokbyun/development/R-BMS`, 브랜치 `dev`, 원격 `B-HS/R-BMS`(공개). 릴리스는 `dev` 에서 `gh workflow run release.yml --ref dev -f bump=patch`. Rust 1.95.0, 웹 bun 1.4.0(설치본 1.4.2), Cargo 버전 0.1.9. 게시: v0.1.4·v0.1.6·v0.1.7·v0.1.8·v0.1.9(Latest), draft: v0.1.0·v0.1.3·v0.1.5(그대로 둠).
- 게이트: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`, `cargo test --workspace --no-fail-fast --locked`(마지막 7,129 통과), 웹 `cd web && bun run verify && bun run format:check && bun run build`(마지막 1,981 통과), DB 통합 `RBMS_DB_E2E=1 bun test tests/db`(Docker `mysql:8.4.11`, 3306 은 다른 컨테이너가 쓰므로 3317 같은 포트로, 끝나면 컨테이너 삭제, 마지막 161 통과), 스킨 `cargo run -q -p rbms-cli --locked -- skin-dump assets/skins/rbms-default --strict --frames 120`(언어는 시나리오 `base.json` 의 불린 21150~21152), 시트 `uv run assets/skins/rbms-default/tools/generate-images.py --check`, 아이콘 `uv run assets/icon/tools/generate-icon.py --check`, 계약 `RBMS_UPDATE_CONTRACT=1 cargo test --locked -p rbms-player --lib a_settings_upload` + `bun test tests/dto/client-requests.test.ts`.
- 선곡 캡처: `RBMS_SKIN_PACK=$PWD/assets/skins/rbms-default RBMS_SKIN_CAPTURE_DIR=<폴더> cargo test -p rbms-player --lib --locked the_browser_of_a_skin_pack_named_by_the_environment_is_captured_over_a_scanned_library`(캡처 하니스는 CJK 를 그리지 못한다), README 용은 `sips -s format jpeg`.
- 이 Mac 의 `python3` 는 3.9 라 긴 한글 줄 스크립트는 파일로 쓰고 `uv run -q --python 3.13 python -I <파일>` 로 돌린다. 자동 모드가 `rm -rf` 가 든 긴 명령을 거부한 적이 있어 임시 폴더는 새 이름으로 만든다.
- CI: `ci.yml`(Rust 3 OS), `web.yml`, `release.yml`, `signing-check.yml`, `web-db.yml`(`status|migrate|cleanup-*|chart-notes-*|admin-seed-count|assist-grade-backfill*|chart-purge-*|courses-seed*|tables-refresh-*|dan-recompute*`). web 시험은 bun 이 한 프로세스에서 파일을 돌려 sonner 토스트 같은 전역 상태가 파일 사이로 샌다(시험 시작 때도 비울 것, `69d86a7`).
- 운영 DB 마이그레이션 0000~0013. Vercel 프로젝트 `rbms-web`, Production `dev`, 웹은 언어 쿠키 때문에 전 페이지 요청마다 렌더(`instant = false`), Cron `0 15 * * *` → `GET /api/cron/tables`(`CRON_SECRET` Bearer).
- 클라이언트 값: 기록 규칙 세대 4, `PARSER_VERSION` 5, 설정 스키마 6, `SETTING_COUNT` 99, 단축키 32, `RADAR_VERSION` 1, 확장 id 공간(이벤트 21000~21099, 불린 21100~21199, 값 공간 21000~21199), Discord Application ID `1563156237539999865`. 정리: [memory/client-versions-and-run-rules.md](memory/client-versions-and-run-rules.md).
- 차트 라이브러리 `/Volumes/SSD/bms`(자주 빠짐). 앱을 띄우면 기본 표 주소(`bms.hyuns.uk`, `stellabms.xyz`)를 GET 한다.
- 이름 주의: `apps/rbms-player/src/ir_sync.rs` 는 설정 동기화, DB 표 `ir_sync` 는 기록 동기화.

## 8. 다음 세션 TODO(우선순위)

0. 사용자 몫 확인: (a) `CRON_SECRET` 등록과 첫 Cron 결과, (b) 실기 QA 3.23·3.24(특히 한글·일본어 글꼴 모양, 2·4·6 길게 누르기 손맛, 고스트, 레이더 값, 썸네일·휠 속도, 웹 세 언어, A-단위 배지), (c) 일본어 감수를 누구에게 맡길지. 결과를 받으면 PROCESS 에 새 R 항목으로 고친다.
1. 실기에서 나온 문제 수리(관련 파일은 QA 항목마다 ADR 에 있다).
2. 테스트 부채 3.112~3.129(`docs/quality-assurance/2026-10-14-test-debt.md`): CJK 하니스 골든 없음, 레이더 3축 스케일·IIDX 대조, 썸네일·분석·형제 탐색 프레임 비용, 랭킹 모드에서 앱 오버레이가 정보창 IR RANKING 을 덮음, 순위 응답 크기, 서버 리플레이 고스트(보류), 0013 이전 판 추정, 플레이어 순위 배지 2개 제한.
3. 웹 정적 셸을 되찾을지(`app/[locale]` 이동, `acknowledge/2026-10-24-adr-web-i18n.md`).
4. 곡 미러 보류 해제 여부와 이전 세션 TODO(이전 스냅샷 8절 3·4).

## 9. 문서 지도

- [HANDOFF.md](HANDOFF.md) — 이 문서, 새 세션의 진입점(항상 덮어씀).
- [PROCESS.md](PROCESS.md) — 체크리스트와 상태. 첫 체크리스트가 2026-10-24 "어시스트 단위 A-단위", 둘째가 "다국어·리플레이 대전·랭킹 보기·NOTES RADAR·선곡 UI"(결정과 웨이브 W1~W6 진행 기록).
- [README.md](README.md) — docs 색인(영어). 루트 [README](../README.md) 는 사용자용 짧은 소개.
- [architecture.md](architecture.md), [crates.md](crates.md), [development.md](development.md) — 구조, 크레이트, 개발 명령(`RBMS_LANGUAGE` 포함).
- [play-options.md](play-options.md) — 플레이 옵션과 조작(언어, 배치 플레이, 랭킹 모드, 선곡 화면 포함).
- [skin.md](skin.md), [theme.md](theme.md), [font-cjk-support.md](font-cjk-support.md) — 스킨(확장 id 표, `songlist` 의 `rank`·`thumb`, `t(key)`), 앱 테마, 글꼴.
- [ci-release.md](ci-release.md) — CI, 릴리스 이력 2.3(v0.1.9 까지), 서명·공증, web-db 액션과 운영 실행 기록.
- [acknowledge/README.md](acknowledge/README.md) — 결정 색인(2026-10-24 ADR 8건 포함)과 미확인 목록.
- [plan/](plan/) — `2026-10-24-five-features-plan.md`(확장 id 배정표·소유권·웨이브), `2026-10-24-assist-dan.md`, 이전 설계서들.
- [reference/](reference/) — `2026-10-24-*-survey.md` 조사 5건, `2026-10-24-i18n-glossary.md`(세 언어 용어집), `ir-api.md`, 이전 조사.
- [backend/](backend/), [web/](web/) — API 명세·데이터 모델(0013, 단위 `assisted`), 웹 구조(§16 다국어).
- [quality-assurance/](quality-assurance/) — 실기 점검표(3.23 다섯 기능, 3.24 A-단위, 3.20~3.22), 테스트 부채(3.112~3.129 가 2026-10-24).
- [feedback/](feedback/), [memory/](memory/), [bug/](bug/), [history/](history/), [utils/](utils/), [assets/](assets/).
