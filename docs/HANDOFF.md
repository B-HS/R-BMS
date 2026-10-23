# HANDOFF — 2026-10-24 세션 스냅샷

최종 갱신 2026-10-24 · 대응 커밋 `ec48d8d`(이 문서와 함께 커밋되는 문서 정리 커밋이 바로 뒤에 온다). 이 문서는 항상 덮어쓴다. 이전 스냅샷: `docs/history/2026-10-23-handoff-snapshot.md`(바로 앞 판, 2026-10-22 고도화 3.0절과 2026-10-14~15 작업 3.1절 표가 그대로 있다), 그 앞 `2026-10-22-`, `2026-10-14-`, `2026-10-11-`, `2026-10-09-handoff-snapshot.md`.

복기 신뢰도: 2026-10-23~24 세션은 대화 압축 없이 이어졌다. 결정·사용자 원문·커밋·운영 run 번호는 대화와 `git log`·`gh run` 으로 확인한 값이라 신뢰도가 높다. 하위 에이전트 내부 수치(테스트 수 일부, 측정값)는 각 보고를 옮긴 것이다. 하위 에이전트 보고서 원본은 세션 scratch(저장소 밖)에 있어 새 세션에서는 읽을 수 없고, 필요한 내용은 설계서·ADR·조사 문서에 옮겨 두었다. 사실이 의심되면 이 문서보다 코드와 `git log` 를 믿는다.

## 1. 프로젝트 한 줄 정의

R-BMS 는 Rust 로 만든 BMS 리듬게임 플레이어(beatoraja 형식 Lua 스킨 호환 + 자작 모노톤 기본 스킨)와, 같은 저장소의 점수 서버 겸 웹사이트(`web/`, Next.js, https://bms.hyuns.uk)다.

## 2. 현재 목표

- 최종 목표: 사용자가 실제로 쓰는 배포 가능한 BMS 플레이어 + 웹(순위, 난이도표, 내 폴더, 코스·단위). 사용자는 "더 이상 구현하거나 버그가 없도록" 남은 일을 정확히 끝내길 원하고, 가끔 실패하는 테스트도 없어야 한다고 했다.
- 현재 마일스톤: 2026-10-23 "Stella·코스·단위와 클라이언트 보완"(사용자 요구 8개 + 결정 21개)과 Discord 상태 표시를 끝내고 **클라이언트 v0.1.8 을 사용자가 게시했다**(Latest, 2026-10-23 14:52 UTC). 진행 중인 구현 작업은 없다. 다음은 사용자 실기(QA 3.20·3.21·3.22)와 남은 사용자 몫(8절 0).
- 직전 작업(2026-10-23~24 세션, 시간 순):
  1. 컨텍스트 복원과 문서-코드 대조(동작 불일치 0, 표현 6건), 2026-10-22 ADR 5건 사용자 확인("1-11 추천대로").
  2. 요구 1~8 조사(`docs/reference/2026-10-23-stella-course-dan-survey.md`), 결정 21개(원문은 `docs/PROCESS.md` "Stella·코스·단위" 체크리스트), 설계서 5개 + 교차 검토 + 메인 결정(`docs/plan/2026-10-23-*.md`, `design-review.md` 10절).
  3. 구현 웨이브 1~7(`9e39262` ~ `5a0eacf`), F8 통합 리뷰와 수리(`45d09fd`, `76e026a`), G 문서(`92a0caa`, ADR 13건).
  4. 사이에 사용자 요청: `架空ユリカゴ　 [SABOTHER]` 웹 전체 삭제(`a6e7ffe`, `0b4084e`), Discord Rich Presence(`d7892ad`, `3d21944`, `41f1a1b`, Developer Portal 은 메인이 Claude in Chrome 으로 설정), README 재작성(`e9d2ed7`), v0.1.8 릴리스(사용자 `!` 실행 run 39970364536, 버전 커밋 `e92866a`, 한글 draft 노트, 기록 `ec48d8d`).
  5. 이 인수인계(`/prepare-new`).

## 3. 완료 / 진행 중 / 미착수

### 3.0 완료(2026-10-23~24 세션)

결정의 "왜"는 [acknowledge/README.md](acknowledge/README.md) 의 "2026-10-23" 절 ADR 15건, 체크리스트는 [PROCESS.md](PROCESS.md) 첫 체크리스트(F0~F8·G·D).

| 묶음 | 내용 | 커밋 | 주요 파일 |
| --- | --- | --- | --- |
| 결정·설계 | 2026-10-22 ADR 5건 확인, 조사 보고, 설계서 5개·교차 검토·메인 결정 | `e2da295`, `1169efb`, `3c0cd22` | `docs/reference/2026-10-23-stella-course-dan-survey.md`, `docs/plan/2026-10-23-*.md` |
| Esc 결과 화면(결정 9) | 노트를 1개 이상 친 판의 Esc → 결과 화면, 코스는 FAILED 코스 결과까지, 판정 수는 저장값과 같게 | `9e39262` | `apps/rbms-player/src/app_result.rs`, `stage/play/mod.rs`, `crates/rbms-play/src/{record,session}.rs` |
| SCRATCH AUTO 색(결정 8) | PLAY 메뉴 CHART → SCRATCH AUTO COLOR, 외부 스킨 포함, 저장값 없으면 `#30E050` | `9e39262` | `crates/rbms-render/src/{skin_render/notes,playfield}.rs`, `stage/play_menu/*` |
| 패널 키 opt-in(결정 5) | 선곡 스킨 헤더 `rbms_extensions` 가 있을 때만 재배정 키 표, 없으면 원본 표 | `9e39262` | `stage/select/keys.rs`, `crates/rbms-skin/src/model.rs`, `assets/skins/rbms-default/lua/select/main.lua` |
| IIDX 게이지 표 | `GaugeSetId::Iidx`, `GutsEdge::AtOrBelow`, `CourseGaugeMode` | `9e39262` | `crates/rbms-judge/src/{gauge_tables,gauge,data}.rs`, `crates/rbms-judge/data/gauge.ron` |
| 다운로드 크레이트 | `crates/rbms-download`(zip 9.0.0 + typed-path 0.12.3, 안전 장치·상한), Windows 이름 충돌 수리 | `9e39262`, `a6dc73c` | `crates/rbms-download/**` |
| 마이그레이션 0012 | 코스 출처·묶음·단위·표 스냅샷 표 7개, 코스 열 5개(추가만) | `9e39262`(사고 커밋, 5절·feedback) | `web/drizzle/0012_course_dan_tables.sql` |
| 코스 게이지 설정(결정 21) | 설정 JUDGE → COURSE GAUGE(LR2 기본/BEATORAJA/IIDX) | `9f8dfbc` | `crates/rbms-config/src/{judge,schema,settings}.rs`, `stage/settings/catalog.rs` |
| GENOSIDE 2018 데이터(결정 18) | SP 22·DP 22 코스 데이터와 생성기 `rbms-cli course-snapshot`, 표 header 코스 변환 `courses_from_table_header`, DP 제목 88개 | `f174f2b`, `ea5829d` | `web/data/courses/genoside-2018/courses.json`, `apps/rbms-cli/src/course_snapshot*`, `crates/rbms-course/src/{bundled,table}.rs` |
| 폴더 스택(결정 10·11) | 임의 깊이 폴더, 나올 때 커서 복원, 표 묶음 폴더, 레벨 제목 `symbol+레벨`(★12) | `fc23232` | `apps/rbms-player/src/select_tree.rs`, `stage/select/{mod,list,tree_tests}.rs` |
| 웹 코스 제출 등급(결정 6) | 제출의 `gauge`·`course_gauge`·`stages`, `course_best.assist_grade` | `db534d1` | `web/src/server/{dto/course.ts,service/domain/course/course.service.ts}` |
| Stella·Satellite 기본 표(결정 12) | 원본 주소에서 직접 받음, `STELLA` 묶음, 내장 스냅샷 없음 | `2787264` | `crates/rbms-config/src/tables.rs`, `apps/rbms-player/src/tablesrc/seed.rs` |
| 표 다시 받기(결정 13 클라이언트) | 저장본이 하루 지나면 시작 때 백그라운드로 | `519d7d6` | `apps/rbms-player/src/tablesrc.rs` |
| 다운로드 폴더·없는 곡 설정(결정 16) | `library.download_folder`, `show_missing_charts`, 곡 폴더 목록에 포함 | `6998d95` | `crates/rbms-config/src/schema.rs`, `stage/folders.rs`, `stage/settings/*` |
| 코스 진행(J-4·J-7) | COURSE GAUGE 표로 판정, 시작 슬롯이 GAUGE 설정을 따름, 9칸 이월, ln·cn·hcn 제약 적용 | `c4fcdfa`, `47e8f70`(코스 시작 때 규칙 고정) | `apps/rbms-player/src/{app_play,course_ui}.rs`, `crates/rbms-course/src/run.rs` |
| 곡 미러 준비(결정 17) | `rbms-cli mirror-pack`(결정적 zip + manifest, 업로드 없음) | `fbae422` | `apps/rbms-cli/src/mirror_pack*` |
| 웹 코스 카탈로그 | GENOSIDE·표 코스 등록 서비스, `web/scripts/seed-courses.ts` | `c79f53f` | `web/src/server/service/domain/course-catalog/**` |
| COURSE 탭 번들 코스 | GENOSIDE 44코스를 사용자 코스 앞에 | `a52d9a3` | `apps/rbms-player/src/course_ui.rs` |
| 없는 곡 막대(결정 16) | 표 레벨 안 흐린 막대, 외부 스킨에는 Song exists false | `0ff2077` | `stage/select/{list,scene,missing_tests}.rs`, `crates/rbms-render/src/select.rs` |
| 코스 제출 필드(결정 6 클라이언트) | `gauge`·`course_gauge`·`stages`, 계약 골든 | `47e8f70` | `apps/rbms-player/src/course_ir.rs`, `crates/rbms-ir/src/dto/score.rs`, `contract/client-requests/course-*.json` |
| 단위 자동 판정(결정 19·20) | `user_dan`, 표시 선택 API, 판정 이유 코드 | `d57c6c2` | `web/src/server/service/domain/dan/**`, `web/src/app/api/fe/me/dans/**` |
| 표 COURSE 폴더 | 표 header 코스 + 발광 아래 GENOSIDE 44(번들과 같은 차트 구성의 header 코스는 한 번만), 코스 파일 매번 다시 읽기 | `0247561` | `stage/select/*`, `tablesrc.rs` |
| 웹 Cron·조립 | Vercel Cron `0 15 * * *`, `CRON_SECRET`, 표 스냅샷 저장소, 서비스 조립 | `942ed71` | `web/vercel.json`, `web/src/server/route/cron.ts`, `web/src/server/compose/index.ts` |
| 다운로드 모달(결정 15) | 차분·곡 URL 안내, 직접 zip 받기, 브라우저 열기 | `8d0a62a` | `stage/select/download_modal.rs`, `apps/rbms-player/src/open_url.rs` |
| 웹 화면 | `/tables/stella-st`·`/tables/stella-sl` 출처 표기, `/courses`, 단위 배지, 마이페이지 단위 탭 | `f2d8ce5`, `608eae7` | `web/src/app/(app)/{tables,courses}/**`, `web/src/widgets/settings/dan-panel.tsx` |
| web-db 액션 | `courses-seed*`, `tables-refresh-*`, `dan-recompute*`(그 앞 `chart-purge-*`) | `df52b3c`, `a6e7ffe` | `.github/workflows/web-db.yml` |
| 받기 작업 | 받기 워커, 부분 재스캔, 취소, 진행 표시 | `5a0eacf`, 수리 `45d09fd` | `apps/rbms-player/src/{download,library}.rs`, `crates/rbms-library/src/scan.rs` |
| 곡 삭제(사용자 요청) | 발광 사본 1,034곡, 운영 행 전부 삭제 | `a6e7ffe`, `0b4084e` | `web/data/tables/insane1/*`, `web/scripts/{snapshot-table.ts,chart-purge.*.sql}` |
| Discord | Rich Presence, 설정 DISCORD STATUS, `discord-1024.png` | `d7892ad`, `3d21944`, `41f1a1b` | `apps/rbms-player/src/discord_presence/**`, `assets/icon/png/discord-1024.png` |
| 문서 | ADR 15건, 사용자 문서·아키텍처·API·QA 3.21·3.22·테스트 부채 3.100~3.111 | `92a0caa`, `41f1a1b`, 이 커밋 | `docs/**` |
| README | 짧게 다시 쓰고 화면 캡처 2장 | `e9d2ed7` | `README.md`, `docs/assets/screenshot-{select,play}.jpg` |
| 릴리스 v0.1.8 | 사용자 실행·게시, 메인이 로그로 서명·공증 확인, 한글 노트 | `e92866a`, `ec48d8d` | `docs/ci-release.md` 2.3 |

운영 반영(사용자 승인 "3번은 승인 그냥 바로 진행해놔", 실행은 자동 모드 거부로 모두 사용자 `!`): 마이그레이션 0012(run 39909099575, 7/7), 곡 삭제 `chart-purge-delete`(run 39927201095, 차트 1·판 63·리플레이 57·trace 63 등, 삭제 뒤 모두 0), `courses-seed`(run 39946701885, SP 22·DP 22), `tables-refresh-force`(run 39946847395, st 2,350·sl 2,497).

마지막 게이트: Rust 6,644 통과·0 실패(`3d21944` 병합 뒤), 웹 1,808 통과·0 실패, ci 3 OS 통과(run 39967408202, Discord 의 Windows pipe 코드 포함), web CI 통과.

### 3.1 이전 세션(요약, 상세는 이전 스냅샷)

- 2026-10-22: 옵션 패널·램프 등급·색 행·UI 글자 고도화와 v0.1.7(`docs/history/2026-10-23-handoff-snapshot.md` 3.0절).
- 2026-10-14~20: 기록 동기화, 판정 의미 9건, 키 입력 시각, 웹 AI 기능 W1~W6, 앱 아이콘 등(같은 파일 3.1절, `docs/history/2026-10-15-process-archive.md`).

### 3.2 진행 중

진행 중인 구현 작업 없음(2026-10-24). 남은 것은 모두 사용자 몫이거나 실기 결과에 달렸다(8절).

### 3.3 미착수

8절이 정본이다.

## 4. 의사결정 요약

정본은 [acknowledge/README.md](acknowledge/README.md). 2026-10-14~22 결정 표는 이전 스냅샷 4절에 그대로 있다. 2026-10-23 결정:

| 결정 | 채택 | 기각된 대안과 이유 | ADR |
| --- | --- | --- | --- |
| 2026-10-22 결정 5건 | 사용자 확인("1-11 추천대로"). 바뀐 둘: 패널 키는 스킨 헤더 opt-in, `course_best.assist_grade` 는 게이지로 채움 | 패널 키를 모든 스킨에 적용(외부 스킨에서 표시 없이 설정이 바뀜), 열 삭제 | `panel-keys-opt-in`, `course-submission-gauge` |
| Esc | 친 판은 결과 화면, 코스는 FAILED 코스 결과 | 곧장 선곡(옛 결정, 원본과 다름) | `escape-shows-the-result` |
| SCRATCH AUTO 색 | PLAY 전역 RGBA 한 값, 외부 스킨 포함 | 기본 스킨 색 행만(외부 스킨 불가), 엔진 자동 색 행 | `scratch-auto-note-colour` |
| 선곡 폴더 | 폴더 스택(원본 BarManager) | 고정 깊이 enum 최소 수정(재귀 폴더에서 다시 갈아엎음) | `select-folder-stack` |
| Stella·Satellite | 원본 주소 직접, 내장 스냅샷 없음, 하루 지나면 다시 받음 | 웹 사본만, 내장 스냅샷(낡음) | `stella-default-tables` |
| 표 매일 갱신 | Vercel Cron + 운영 DB `table_snapshot` | GitHub Actions 가 스냅샷 커밋(이력 오염, Rust CI 같이 돎) | `table-daily-refresh` |
| 다운로드 | 직접 zip 만 앱이 받음(새 의존성 zip 승인), 나머지 브라우저 | rar·7z 지원(큰 의존성), 링크 열기만 | `missing-charts-and-download` |
| zip 버전 | 게시 14일 지난 9.0.0·typed-path 0.12.3 | 설계서의 9.0.5(게시 14일 미만) | 같은 ADR |
| 곡 미러 | 업로드 준비만(도구), 조사 기록 보고 | 허락 없이 전곡 업로드(권리) | `song-mirror-prep` |
| 코스 원천 | GENOSIDE 2018 SP·DP(운영자 공식 코스 파일 + beatoraja 표 제약) + 표 코스 | bms-ir 페이지 파싱(같은 해시, 안정성 낮음) | `genoside-2018-course-data` |
| 코스 게이지 | 설정 LR2(기본)/BEATORAJA/IIDX, 설정이 제약보다 우선, IIDX 실패선 0%, 코스 시작 때 COURSE GAUGE 규칙 고정(시작 슬롯은 스테이지마다 그때의 GAUGE 설정) | 제약 우선(원본), LR2·BEATORAJA 둘만 | `course-gauge-modes` |
| 코스 시작·LN(J-4·J-7, 사용자 미확인) | 시작 슬롯이 GAUGE 설정, ln·cn·hcn 강제, 9칸 이월(원본과 같음) | 지금 R-BMS(항상 CLASS, 제약 미적용) | `course-stage-start-and-ln` |
| 단위 | 서버 자동 판정 `user_dan`, 마이페이지에서 표시 선택, ranked·제약 준수·어시스트 제외, 모드 제한 없음(J-11, 미확인) | 자기 신고, `user.rank` 재사용 | `dan-auto-award` |
| 곡 삭제 | 웹의 모든 흔적(사용자) | 표에서만 숨김, DB 만 | `chart-removed-at-owner-request` |
| Discord | 로컬 IPC Rich Presence(새 크레이트 없음), 기본 켬 | Social SDK(네이티브 의존), 크레이트, 큰 그림 URL | `discord-rich-presence` |

## 5. 사용자 방향성 & 작업 규칙

- 응답: 한국어 존댓말만(짧은 상태 보고 포함), 간결, 첫 줄은 행동이나 답, 매 응답에 현재 상태 한 줄, 번호 목록, 이모지·글자 그림 금지. 확인하지 못한 것은 그렇다고 말한다.
- 새 실행 작업과 세션 경계(새 세션, resume, compact, clear, prepare-new)마다 workflow 사용 여부를 한 번 묻고 답을 기다린다. 선택은 승계하지 않는다.
- workflow 를 쓰면: Workflow 도구만, Fable·Haiku 금지, Opus 는 effort high 까지, Sonnet 은 xhigh 까지, 모든 agent 에 model·effort 명시, 에이전트는 커밋·푸시 금지, 에이전트가 `rbms-player` 를 실행하면 scratch HOME 과 `--server http://127.0.0.1:9`. 웨이브는 worktree 격리로 병렬, 메인이 병합·게이트·커밋. 세션 한도로 끊긴 단위는 새 worktree 가 아니라 남은 worktree 에서 이어받게 한다(2026-10-23 두 번 실제로 했다).
- **병합과 부분 커밋(2026-10-23 사고 뒤)**: worktree 패치는 `git apply`(`--3way` 금지, 인덱스에 올라감)로 합치고, 단위별 커밋은 `git apply --cached <단위 패치>` 로 올린 뒤 `git diff --cached --name-only` 를 **별도 명령으로** 읽고 커밋한다. 확인과 commit·push 를 한 줄에 잇지 않는다([feedback](feedback/2026-10-23-stage-check-before-commit.md)).
- 결정을 맡기는 말("추천대로", "나머지도 추천대로")이면 추천안으로 정하고 ADR 에 "사용자 미확인" 으로 남긴다. 답이 없는 결정은 추천안으로 정하고 알린다.
- 운영 반영: 자동 모드가 `web-db.yml` 의 쓰기 action·`release.yml`·운영 사이트 GET(`curl https://bms.hyuns.uk/...`)·`gh release download` 를 거부할 수 있다. 우회하지 말고 명령을 보여 `!` 실행을 요청한다. 읽기 action(`status`, `*-count`)은 통과했다. 새 웹 코드가 새 마이그레이션에 기대면 마이그레이션 파일만 먼저 커밋·적용하고 코드는 뒤에 푸시한다(0012 때 지키지 못해 사고). 릴리스 산출물은 사용자가 "받을 필요가있나 ?" 라고 해 v0.1.8 은 실행 로그로만 확인했다.
- 근본 원인 수정, 떠는 테스트 0, 실제 측정·캡처·산출물로 확인한 뒤 됐다고 말한다. 판정·점수 의미가 원본과 달라지는 수정은 고치기 전에 확인한다(원본과 같아지는 수정은 진행하되 "지금 R-BMS 대비 바뀜" 으로 보고).
- 사람이 친 판은 전부 저장·전송(노트를 하나도 치지 않은 판만 제외), 메인이 제외 조건을 추천하지 않는다.
- Git: Conventional Commits, 한국어 설명, author 사용자 단독, AI 트레일러 금지(세션 링크 트레일러도 넣지 않는다), 선별 스테이징, force push 금지, 논리 단위 커밋, 푸시 뒤 `gh run list` 로 3 OS ci 와 web CI 확인. 진행 중인 ci 가 있으면 새 푸시가 그것을 취소하므로 중요한 검증(예: Windows) 중에는 로컬 커밋만 해 두고 끝난 뒤 푸시한다.
- Rust: `//` 주석 금지(`///`·`//!` 영어만), 매직넘버·새 `#[allow]`·`unsafe`·`todo!()`·새 의존성 금지(2026-10-23 예외 승인: zip·typed-path), rustfmt 160, `--locked`. 웹은 `web/AGENTS.md`. 설정에는 `#[serde(default)]`, `skip_serializing_if` 를 새로 쓰지 않는다(R-1).
- 비밀 파일(.env*, `~/.config/rbms/settings.ron` 등) 접근 금지, 시크릿 값이 대화에 남는 방법 금지. ModernChic·beatoraja 폴더는 읽기만. 외부 게시(다른 저장소 이슈·댓글·PR)는 초안 승인 뒤에만.
- 사용자가 직접 하라고 한 외부 계정 작업(예: Discord Developer Portal)은 Claude in Chrome 으로 하고 결과를 화면으로 확인해 보고한다.
- 다른 사이트의 흔적을 싫어한다. 단 Stella·Satellite·단위 코스는 출처 표기(결정 14), 발광 insane1 은 흔적 제거 유지.
- 지우라는 요청은 무엇이 함께 사라지는지 개수를 보여 확인하고 지운다.
- README 는 TAIDE·raw-viewer README 처럼 짧게(가운데 정렬 아이콘·한 줄 소개·링크, 화면 캡처, 기능 목록, 설치, 조작, 개발, 라이선스). 자세한 것은 docs 링크로. 릴리스 노트는 한글.

## 6. 미해결 질문 / 사용자 확인 필요

1. **2026-10-23 메인 결정(사용자 미확인)**: J-4 코스 시작 슬롯이 GAUGE 설정을 따름, J-7 ln·cn·hcn 제약을 플레이에 적용, 9칸 이월(셋 다 원본과 같아짐, 지금 R-BMS 대비 체감이 바뀜), J-11 단위에 게이지 모드 제한 없음, J-13 ranked 판만, J-14 레벨 제목 `★12`, 각 설계서의 메인 결정, Discord 결정 2~4(v0.1.8 에 넣음·표시 내용·기본 켬). 목록: `docs/acknowledge/README.md` 미확인 13~18.
2. **곡 미러(결정 17) 질문 8개**: Q-R1 업로드 가능 근거(A 작자 허락 / B GENOCIDE "各自保管…自由" / C 무료 공개 배포 / D 기타), Q-R2 범위, Q-R3 로그인 사용자만인지, Q-R4 삭제 요청 창구, Q-R5 사전 연락, Q-C1~C3 계정·버킷·결과 폴더(`docs/plan/2026-10-23-song-mirror-prep.md` 3.3·11절). 조사 요지: 어느 곳도 제3자 재배포 허락 문구가 없다, R2 Standard 월 약 $0.45(発狂 ≈ 40GB 추정), 이그레스 무료. 사용자는 아직 이 보고를 받지 못했다(최종 보고에 넣기로 했으나 세션이 릴리스로 이어졌다).
3. **Vercel `CRON_SECRET`**: 사용자가 등록했는지 확인하지 못했다. 없으면 매일 KST 0시 Cron 이 503 으로 끝나고(사이트는 정상), 1~15자면 첫 `getEnv()` 호출이 던져 DB·인증을 쓰는 요청이 모두 실패한다.
4. 표 코스(Stella 26·Satellite 13)가 `tables-refresh-force` 에서 등록됐는지 숫자로 확인하지 못했다(스크립트가 코스 수를 출력하지 않음). `/courses` 화면 확인 필요.
5. 표 코스 막대 위의 PRACTICE 키는 아무 일도 하지 않는다(COURSE 탭에서는 코스 시작). 원본과 맞출지(U5 보고).
6. 이전 세션의 열린 질문(이전 스냅샷 6절 1~7, 9~12): 순위 `lnmode` 필터 뜻, 코스 램프 서버 검사, 남은 다른 사이트 흔적, 거절 재전송 좁힘, 메뉴 PAD DEBOUNCE, 연습 게이지 사망, TOTAL 행 경고, rate limit 저장소, `user.rank_points`, 기록 목표 설계 선택, 비례 표시.
7. v0.1.7 실기(QA 3.20) 결과도 아직 받지 않았다.

## 7. 환경 & 전제

- 저장소 `/Users/hyunseokbyun/development/R-BMS`, 브랜치 `dev`, 원격 `B-HS/R-BMS`(공개). 릴리스는 `dev` 에서 `gh workflow run release.yml --ref dev -f bump=patch`(사용자 `!`). Rust 1.95.0, 웹 bun 1.4.0(설치본 1.4.2), Cargo 버전 0.1.8. 게시: v0.1.4·v0.1.6·v0.1.7·v0.1.8(Latest), draft: v0.1.0·v0.1.3·v0.1.5(그대로 둠).
- 게이트: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`, `cargo test --workspace --no-fail-fast --locked`(마지막 6,644 통과), 웹 `cd web && bun run verify && bun run format:check && bun run build`(마지막 1,808 통과), DB 통합 `RBMS_DB_E2E=1 bun test tests/db`(Docker `mysql:8.4.11`, 끝나면 컨테이너 삭제), 스킨 `cargo run -q -p rbms-cli --locked -- skin-dump assets/skins/rbms-default --strict --frames 120`, 아이콘 `uv run assets/icon/tools/generate-icon.py --check`, 표 `cd web && bun scripts/snapshot-table.ts https://darksabun.club/table/archive/insane1/ data/tables/insane1 --check`(1,034곡, 제거 md5 를 뺀 원본과 같음), 계약 `RBMS_UPDATE_CONTRACT=1 cargo test --locked -p rbms-player --lib a_settings_upload`(설정 필드가 바뀌면) + `bun test tests/dto/client-requests.test.ts`.
- 이 Mac 의 `python3` 는 3.9 라 긴 한글 줄이 든 heredoc·스크립트에서 "Non-UTF-8 code" 오류가 난다. 스크립트는 파일로 쓰고 `uv run -q --python 3.13 python -I <파일>` 로 돌린다.
- README 화면 캡처는 `RBMS_SKIN_PACK=$PWD/assets/skins/rbms-default RBMS_SKIN_CAPTURE_DIR=<폴더> cargo test -p rbms-player --lib --locked the_play_scene_of_a_skin_pack_named_by_the_environment_is_captured_through_its_states the_browser_of_a_skin_pack_named_by_the_environment_is_captured_over_a_scanned_library` 로 만들고 `sips -s format jpeg` 로 줄였다.
- CI: `ci.yml`(Rust 3 OS, 진행 중이면 새 푸시가 취소), `web.yml`, `release.yml`, `signing-check.yml`, `web-db.yml`(`status|migrate|cleanup-*|chart-notes-*|admin-seed-count|assist-grade-backfill*|chart-purge-count|chart-purge-delete|courses-seed-count|courses-seed|tables-refresh-count|tables-refresh-force|dan-recompute-count|dan-recompute`).
- 운영 DB 마이그레이션 0000~0012. Vercel 프로젝트 `rbms-web`, Production `dev`, Cron `0 15 * * *`(UTC, KST 0시) → `GET /api/cron/tables`(`CRON_SECRET` Bearer).
- 클라이언트 값: 기록 규칙 세대 4, `PARSER_VERSION` 5, 설정 스키마 6(새 필드는 모두 `#[serde(default)]`), `SETTING_COUNT` 97, 확장 id 이벤트 21000~21099·불린 21100~21199, Discord Application ID `1563156237539999865`(Art Asset `rbms`). 정리: [memory/client-versions-and-run-rules.md](memory/client-versions-and-run-rules.md).
- 차트 라이브러리 `/Volumes/SSD/bms`(자주 빠짐). 2026-10-23 조사 산출물(외부 원문 사본·GENOSIDE 공식 코스 zip·DP 제목 입력)은 이전 세션 scratch 에 있었고 새 세션에서는 없다. GENOSIDE 데이터를 다시 만들려면 `docs/plan/2026-10-23-course-gauge-data.md` 의 입력 출처에서 다시 받아야 한다(`--check` 도 입력이 필요).
- 앱을 띄우면(테스트 포함) 기본 표 주소(`bms.hyuns.uk`, `stellabms.xyz`)를 GET 한다(쓰기 아님).
- 이름 주의: `apps/rbms-player/src/ir_sync.rs` 는 설정 동기화, DB 표 `ir_sync` 는 기록 동기화.

## 8. 다음 세션 TODO(우선순위)

0. 사용자 몫 확인: (a) Vercel `CRON_SECRET` 등록 여부와 첫 Cron 결과(Vercel 대시보드 로그), (b) 웹 화면 `/courses`·`/tables/stella-st`·`/tables/stella-sl`·마이페이지 단위 탭, (c) v0.1.8 실기 QA 3.21(U1~U22)·3.22(Discord D1~D3)와 3.20(v0.1.7) 결과, (d) 6절 1·2·5 답(곡 미러 조사 보고를 아직 전하지 않았으니 6절 2 의 요지를 먼저 보고). 결과를 받으면 PROCESS 의 R 항목으로 고친다.
1. 실기에서 나온 문제 수리(관련 파일은 QA 항목마다 ADR 에 있다). 특히 코스 게이지(J-4·J-7) 체감, IIDX 게이지, Discord 경과 시간 단위(테스트 부채 3.111), Windows 받기·브라우저 열기.
2. 곡 미러: Q-R1~R5·Q-C1~C3 답을 받으면 `docs/plan/2026-10-23-song-mirror-prep.md` 12절(버킷·토큰은 사용자, 값은 대화에 남기지 않음) → 업로드 스크립트 dry-run → 서버 등록·모달 버튼(M1~M4).
3. 테스트 부채 3.100~3.111(`docs/quality-assurance/2026-10-14-test-debt.md`): 표 코스 막대의 스킨 문자열, NO GOOD/NO GREAT 미적용, DP MIRROR FLIP, 코스 스테이지 리플레이의 코스 게이지, Stella ETag 없음, 이름 거절로 건너뛰는 키음 등.
4. 이전 세션 TODO(이전 스냅샷 8절 2~7): 실제 창 QA 나머지, 테스트 부채 3.72·3.74·3.55·3.56·3.12·3.73·3.37, 오토플레이 만점 검사 도구, 클라이언트 한계, 웹 2차, wgpu 상류 이슈.

## 9. 문서 지도

- [HANDOFF.md](HANDOFF.md) — 이 문서, 새 세션의 진입점(항상 덮어씀).
- [PROCESS.md](PROCESS.md) — 체크리스트와 상태. 첫 체크리스트가 2026-10-23 "Stella·코스·단위와 클라이언트 보완"(결정 1~21 원문, F0~F8·G·D, 진행 기록과 커밋).
- [README.md](README.md) — docs 색인(영어). 루트 [README](../README.md) 는 사용자용 짧은 소개.
- [architecture.md](architecture.md), [crates.md](crates.md), [development.md](development.md) — 구조, 크레이트, 개발 명령.
- [play-options.md](play-options.md) — 플레이 옵션과 조작(Esc 결과, COURSE GAUGE, SCRATCH AUTO 색, 패널 키, 없는 곡·다운로드 13절, Discord 14절).
- [skin.md](skin.md), [theme.md](theme.md), [font-cjk-support.md](font-cjk-support.md) — 스킨(`rbms_extensions` 헤더 키 포함), 앱 테마, 글꼴.
- [ci-release.md](ci-release.md) — CI, 릴리스 이력 2.3(v0.1.8 까지), 서명·공증, web-db 액션과 운영 실행 기록.
- [acknowledge/README.md](acknowledge/README.md) — 결정 색인(2026-10-23 ADR 15건)과 미확인 목록 1~18. `reference-divergences.md` 에 2026-10-23 절.
- [plan/](plan/) — 2026-10-23 설계서 6개(`client-small`, `select-tree-download`, `course-gauge-data`, `web-course-dan-tables`, `song-mirror-prep`, `discord-presence`)와 교차 검토 `design-review.md`(10절 메인 결정), 이전 설계서들.
- [reference/](reference/) — `2026-10-23-stella-course-dan-survey.md`(요구 원문·조사 보고·외부 출처 사실), `ir-api.md`, 2026-10-22 조사 4종, `ai-features/`, 그 밖의 조사.
- [backend/](backend/), [web/](web/) — API 명세·데이터 모델(0012, 코스 제출 세 필드, 단위 API), 웹 구조(Cron, 캐시 태그).
- [quality-assurance/](quality-assurance/) — 실기 점검표(3.20 v0.1.7, 3.21 2026-10-23 작업, 3.22 Discord), 테스트 부채(3.100~3.111 이 2026-10-23).
- [feedback/](feedback/) — 지적·사고(2026-10-23 커밋 사고 포함), [memory/](memory/) — 버전 상수·실행 규칙·시험 라이브러리, [bug/](bug/), [history/](history/) — 이전 스냅샷과 아카이브, [utils/](utils/), [assets/](assets/) — README 캡처.
