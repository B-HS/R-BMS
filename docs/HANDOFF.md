# HANDOFF — 2026-11-19 세션 스냅샷

최종 갱신 2026-11-19 · 대응 커밋 `c29c407`(코드), `039d21a`(release: v0.1.12)와 이 문서와 함께 커밋되는 문서 커밋. 이 문서는 항상 덮어쓴다. 이전 스냅샷: `docs/history/2026-11-19-handoff-snapshot.md`(바로 앞 판, 2026-10-24 작성에 2026-11-04 TODO 가 덧붙은 것), 그 앞 `2026-10-24-`, `2026-10-23-`, `2026-10-22-`, `2026-10-14-`, `2026-10-11-`, `2026-10-09-handoff-snapshot.md`.

복기 신뢰도: 2026-11-19 세션은 한 번 요약(compact)을 거쳤고, 도중에 세션 임시 폴더가 사라졌으며, 테스트 웨이브 첫 실행이 5시간 사용량 한도로 끊겼다. 결정·사용자 원문·커밋·CI run 번호는 대화와 `git log`·`gh run` 으로 확인한 값이다. 구현·테스트·수리 단위의 보고 원문은 `docs/history/2026-11-19-feature-upgrade-unit-reports/` 에 있다. 사실이 의심되면 이 문서보다 코드와 `git log` 를 믿는다.

## 1. 프로젝트 한 줄 정의

R-BMS 는 Rust 로 만든 BMS 리듬게임 플레이어(beatoraja 형식 Lua 스킨 호환 + 자작 기본 스킨)와, 같은 저장소의 점수 서버 겸 웹사이트(`web/`, Next.js, https://bms.hyuns.uk)다.

## 2. 현재 목표

- 최종 목표: 사용자가 실제로 쓰는 배포 가능한 BMS 플레이어 + 웹. 사용자는 남은 일을 정확히 끝내길 원하고, 가끔 실패하는 테스트도 없어야 한다고 했다.
- 현재 마일스톤: 2026-11-19 "기능 고도화 12건"과 추가 요청 "램프 반 쪼개기"의 구현·테스트·수리·문서가 끝났다. 클라이언트만 바뀌었고 웹·DB·릴리스는 하지 않았다. 이 변경은 **v0.1.12 draft** 로 올라가 있고 게시는 사용자 몫이다(6절 3). 진행 중인 구현 작업은 없다.
- 직전 작업(2026-11-19 세션, 시간 순): 조사·계획·검토(`cac3795`) → 웨이브 1(`df80201`~`4cbb323`) → 웨이브 2(`51f4884`~`b4444b1`) → 테스트 웨이브와 램프 반 쪼개기(`4874930`~`5f61681`) → 수리 웨이브(`864ef42`~`c29c407`) → 문서.

## 3. 완료 / 진행 중 / 미착수

### 3.0 완료(2026-11-19 세션)

계획·결정·웨이브 결과의 정본은 [plan/2026-11-19-feature-upgrade-12.md](plan/2026-11-19-feature-upgrade-12.md)(M1~M21, U1~U10, 6~10절), ADR 은 [acknowledge/README.md](acknowledge/README.md) 의 2026-11-19 절 9건.

| 요구 | 내용 | 커밋 | 주요 파일 |
| --- | --- | --- | --- |
| 1 블루투스 | 장치가 보고하는 출력 지연을 오디오 시계에 보정(macOS, AUDIO 탭 OUTPUT LATENCY AUTO/OFF), 기본 출력 장치를 판마다 다시 물음, 판 기준점을 예약 축으로, 번들에 Game Mode 키 | `b47398a`, `d0bff5c`, `4d1ce8e` | `crates/rbms-audio/src/{latency,engine,host}.rs`, `apps/rbms-player/src/app_play.rs`, `scripts/macos-app.sh` |
| 2·3·11 플레이 화면 | 출발 폴더, 그 판의 게이지·배치·어시스트, NPS(스킨 옵션 `NPS Display`) | `3c546d3`, `ee59b33`, `c29c407` | `apps/rbms-player/src/stage/play/run_setup.rs`, `skin_host/play/**`, `lua/play/**` |
| 4·13 램프 | 목록 막대·MY RECORDS·랭킹 줄의 어시스트 등급 색, 어시스트 없는 램프와 더 높은 어시스트 등급을 위아래로 나눠 표시 | `df80201`, `4874930` | `crates/rbms-render/src/skin_render/songlist*`, `crates/rbms-store/src/scoredb.rs`, `lua/select/{list,records,info}.lua` |
| 5 창 모드 | WINDOW MODE 에 FULLSCREEN(독점) | `e62240d`, `fe9d387` | `crates/rbms-config/src/window.rs`, `apps/rbms-player/src/window_mode.rs` |
| 6·7 입력 | 조작 `ESC`(패드·보조 키), 조작·단축키의 2키 조합 | `c4e75c2`, `1141d41`, `733ba62` | `apps/rbms-player/src/{keyconfig.rs,keyconfig/chord.rs,gamepad.rs,app_input.rs}`, `stage/{mod.rs,keyconfig.rs}` |
| 8·9 LAYOUT PICK | 실시간 미리보기, 결과 화면 `P` 로 배치 저장, CAPTURED 목록에서 같은 배치·시드로 플레이 | `37e8ddd`, `a23074f`, `733ba62` | `apps/rbms-player/src/{captured_layouts.rs,stage/layout_pick.rs,stage/layout_pick/**,stage/result.rs}` |
| 10 판정 위치 | 사용자 오프셋을 스킨 문서 단위로 읽음(원인: 화면 물리 화소 단위였음) | `80ca8d5`, `1eb4768` | `apps/rbms-player/src/skin_screen.rs`, `stage/capture/play_parts/judge_sizes.rs` |
| 12 선곡 화면 | 머리 한 띠와 경로, 제목 축소, 바닥 한 줄, 정보창 구획 통일 | `51f4884`, `88bfbd4`, `c29c407` | `assets/skins/rbms-default/lua/select/**`, `lua/theme.lua` |
| 이전부터의 결함 | 빌린 배치 판의 재시도가 사용자 배치 옵션으로 기록되던 것, 연습 화면에서 스트림이 다시 열리던 것, 설정 동기화가 눌린 키 상태를 버리던 것 | `864ef42`, `4d1ce8e`, `733ba62` | [bug/2026-11-19-borrowed-layout-retry-recorded-with-own-option.md](bug/2026-11-19-borrowed-layout-retry-recorded-with-own-option.md) |

마지막 게이트(수리 웨이브 병합 뒤): `cargo fmt`·`clippy` 통과, Rust 7,347 통과·0 실패·6 무시, 스킨 덤프·시트 검사 통과, 설정 계약 재생성 뒤 웹 DTO 시험 234 통과(웨이브 1). CI 는 테스트 웨이브 푸시까지 3 OS 통과(run 44476099005). 마지막 푸시의 CI 는 [PROCESS.md](PROCESS.md) 맨 위 체크리스트의 진행 기록을 본다.

### 3.1 이전 세션(요약, 상세는 이전 스냅샷)

- 2026-11-04: AI 훈련곡·분석 v3·로그인 대시보드, 훈련곡 난이도 강화, v0.1.11 draft(게시는 사용자 몫이었다. 게시 여부는 확인하지 않았다).
- 2026-10-29 이전: `docs/history/2026-11-19-handoff-snapshot.md` 와 그 앞.

### 3.2 진행 중

없음.

### 3.3 미착수

8절이 정본이다.

## 4. 의사결정 요약

정본은 [acknowledge/README.md](acknowledge/README.md). 2026-11-19 에 사용자가 정한 것은 요구 12개와 추가 요청 1개, 진행 방식, 그리고 열린 세 건의 처리("2. 그럼 수리지 3. 이것도 그럼 지금 수정인가 ? 남은것도 다 잘 처리하고 다 잘 처리해놔")다. 그 밖의 M1~M21 은 메인이 정했고 **사용자 확인 전**이다(미확인 목록 39~51).

| 결정 | 채택 | ADR |
| --- | --- | --- |
| 블루투스 | 지연 보정 + Game Mode 자격. 키음 지연은 소프트웨어로 못 없앤다. macOS 만 자동, 그 밖은 JUDGE OFFSET | `output-latency-compensation` |
| 창 모드 | FULLSCREEN 은 일반 창으로 열고 첫 동기화에 독점으로, 못 얻으면 BORDERLESS | `exclusive-fullscreen` |
| 조합키·ESC | 2키까지, 조작·단축키만, 조합 완성 누름은 단일 동작을 억제, 레인이 조작 키를 이김 | `chord-bindings-and-escape` |
| 램프 | 기존 등급 색 4개를 목록에, 어시스트 쪽이 더 높을 때만 위아래로 쪼갬, 범위는 클라이언트 | `assist-grade-lamps-in-lists`, `split-clear-lamp` |
| 오프셋 | 모든 사용자 오프셋을 스킨 문서 단위로(저장값의 뜻이 바뀜) | `user-offsets-in-document-units` |
| 플레이 화면 | 값은 설정이 아니라 그 판의 것, NPS 는 직전 1초의 노트 수 | `play-screen-run-facts` |
| LAYOUT PICK | 미리보기는 내장 플레이필드 실시간(소리 없음), capture 는 LAYOUT PICK 에 통합, 기본 키 `P` | `layout-pick-playback-and-capture` |
| 선곡 | 스킨만 개편 | `select-screen-refresh` |

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
- Rust: `//` 주석 금지(`///`·`//!` 영어만), 매직넘버·새 `#[allow]`·`unsafe`·`todo!()`·새 의존성 금지(예외 승인: zip·typed-path, 웹 next-intl. 2026-11-19 메인이 정한 예외, 사용자 확인 전: `rbms-audio` 의 macOS 한정 `coreaudio-rs` 직접 의존, cpal 의 전이 의존성이라 새 크레이트는 없음), 표준 오류에 직접 쓰지 않음(`eprintln!` 을 막는 시험이 있다), rustfmt 160, `--locked`. 웹은 `web/AGENTS.md`. 설정에는 `#[serde(default)]`, `skip_serializing_if` 를 새로 쓰지 않는다.
- 새 사용자 표시 문구는 처음부터 세 언어 카탈로그로(앱 `translations!`, 스킨 `lua/i18n/*.lua`, 웹 `web/messages/{ko,en,ja}`), 용어집을 따른다. 영어 문구를 바꾸면 기존 골든이 깨진다.
- 비밀 파일(.env*, `~/.config/rbms/settings.ron` 등) 접근 금지, 시크릿 값이 대화에 남는 방법 금지. ModernChic·beatoraja 폴더는 읽기만. 외부 게시(다른 저장소 이슈·댓글·PR)는 초안 승인 뒤에만.
- 사용자가 직접 하라고 한 외부 계정 작업은 Claude in Chrome 으로 하고 결과를 화면으로 확인해 보고한다.
- 다른 사이트의 흔적을 싫어한다. 단 Stella·Satellite·단위 코스는 출처 표기, 발광 insane1 은 흔적 제거 유지. 선곡 UI 는 IIDX 식이지만 IIDX 문구·로고·마스코트는 쓰지 않는다(NOTES RADAR 만 사용자 표현).
- 지우라는 요청은 무엇이 함께 사라지는지 개수를 보여 확인하고 지운다.
- README 는 TAIDE·raw-viewer README 처럼 짧게. 릴리스 노트는 한글.


- (2026-11-19 추가) 이 세션의 진행 방식(사용자 원문): "모든 계획 및 검토는 Fable 로 하고 계획 -> 검토 -> 구현(opus + workflow) -> 테스트(sonnet + opus, 여기서는 Fable이 workflow에 필요한 모델선정해서 사용)", "workflow는 사용해도 subagent는 사용하지 말도록". 다음 작업에도 그대로인지는 묻는다.
- (2026-11-19 추가) Workflow 운용에서 배운 것: worktree 는 `origin/dev` 에서 만들어지므로 웨이브 사이에 푸시한다. 세션 임시 폴더는 사라질 수 있으니 단위 보고는 `docs/history/` 에 저장한다. 사용량 한도로 끊긴 실행은 변경이 없으면 같은 스크립트로 다시 띄운다. 기본 스킨 파일을 고친 뒤에는 다시 빌드해야 "포함된 표 = 폴더" 시험이 맞는다. 결과 화면은 레인 키를 키 표로 읽으므로 기본 레인 키를 새 단축키의 기본 키로 쓰지 않는다.

## 6. 미해결 질문 / 사용자 확인 필요

1. **메인 결정 M1~M21 의 확인**: [acknowledge/README.md](acknowledge/README.md) 미확인 39~51. 특히 "지금 R-BMS 대비 바뀐" 둘: 판 시작 직후 정지가 약 38 ms 길어짐(49), 레인 키와 같은 키의 조작은 그 모드에서 듣지 않음(50). 그리고 오프셋 저장값의 뜻 변화(1920x1080 이 아닌 화면에서 맞춘 JUDGE POSITION 등은 다시 맞춰야 함).
2. **실기 결과**: QA 3.29(G1~G17). AirPods AUTO/OFF 체감, 실행 중 장치 바꾸기, Game Mode(BORDERLESS 전체 화면) 전후, FULLSCREEN(Windows 포함), 패드 ESC·조합, 램프 색과 반 쪼개기(GPU), 플레이 화면 표시, LAYOUT PICK 재생·capture, 선곡 화면 세 언어.
3. **v0.1.12 는 draft 다**: release run 44529321802(사용자 `!` 실행, 첫 시도는 Apple 약관 동의 문제로 macOS 잡이 실패했고 동의 뒤 실패한 잡만 다시 실행), 버전 커밋 `039d21a`, 메인이 산출물 점검과 한글 노트 교체까지 끝냈다. 게시는 사용자가 한다. [ci-release.md](ci-release.md) 2.3절.
4. **웹**: 램프 반 쪼개기와 게이지 표기 `EX-HARD` 는 클라이언트만 바꿨다. 웹까지 할지.
5. 이전 세션의 열린 것: Vercel `CRON_SECRET`, 곡 미러(보류), QA 3.20~3.28 실기, 일본어 감수, v0.1.11 게시 여부.

## 7. 환경 & 전제

- 저장소 `/Users/hyunseokbyun/development/R-BMS`, 브랜치 `dev`, 원격 `B-HS/R-BMS`(공개). 릴리스는 `dev` 에서 `gh workflow run release.yml --ref dev -f bump=patch`(자동 모드가 거부하면 사용자 `!`). Rust 1.95.0, 웹 bun 1.4.x, Cargo 버전 0.1.12.
- 게이트: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`, `cargo test --workspace --no-fail-fast --locked`(마지막 7,347 통과), 스킨 `cargo run -q -p rbms-cli --locked -- skin-dump assets/skins/rbms-default --strict --frames 120`, 시트 `uv run assets/skins/rbms-default/tools/generate-images.py --check`, 계약 `RBMS_UPDATE_CONTRACT=1 cargo test --locked -p rbms-player --lib a_settings_upload` + `cd web && bun test tests/dto/client-requests.test.ts`. 웹 게이트는 이전과 같다(`cd web && bun run verify && bun run format:check && bun run build`).
- 캡처: 선곡 `RBMS_SKIN_PACK=$PWD/assets/skins/rbms-default RBMS_SKIN_CAPTURE_DIR=<폴더> cargo test -p rbms-player --lib --locked the_browser_of_a_skin_pack_named_by_the_environment_is_captured_over_a_scanned_library`, 플레이 `RBMS_SKIN_CAPTURE_DIR=<폴더> cargo test -p rbms-player --lib --locked the_play_scene_of_the_default_pack_is_captured_with_its_rows_switched`(약 4분), 쪼개진 램프 `... the_browser_splits_the_lamp_of_a_chart_cleared_without_assists_and_with_scratch_auto_on_a_stronger_gauge`, 판정 위치 측정 `cargo test -p rbms-player --lib --locked judge_sizes -- --nocapture`. 캡처 하니스는 CJK 를 그리지 못한다. README 용은 `sips -s format jpeg`.
- 이 Mac 의 `python3` 는 3.9 라 긴 한글 줄 스크립트는 `uv run -q --python 3.13 python -I` 로 돌린다. 임시 파일은 저장소의 `target/tmp-claude/`(git 이 무시)에 둔다.
- CI: `ci.yml`(Rust 3 OS, 문서만 바꾼 푸시에도 돈다, 14~18분), `web.yml`, `release.yml`, `signing-check.yml`, `web-db.yml`. 진행 중인 ci 가 있으면 새 푸시가 그것을 취소하므로 끝난 뒤 푸시한다.
- 운영 DB 마이그레이션 0000~0014. 이번 작업은 웹·DB 를 바꾸지 않았다.
- 클라이언트 값: 기록 규칙 세대 4, 설정 행 `SETTING_COUNT` 101(새 행 OUTPUT LATENCY), `ControlAction` 21(새 `Escape`), `Shortcut` 34(새 `CaptureLayout`, 기본 `P`), WINDOW MODE 3값, 확장 id(숫자 21085 NPS, 21087~21092 MY RECORDS 등급, 21093~21102 랭킹 등급, 텍스트 21010~21014 출발 폴더·게이지·배치 1P·2P·어시스트, 불린 21160~21163 쪼개진 등급, 플레이 스킨 옵션 916·917 `NPS Display`), `songlist` 확장 키 `lamp_graded`, 점수 DB 열 `score.best_assist_grade`(열 추가 방식, 스키마 버전 그대로), 새 파일 `captured_layouts.ron`(설정 폴더), 키 설정의 조합 토큰 `A+B` 와 `pad.control_chords`·`shortcut_chords`. 정리: [memory/client-versions-and-run-rules.md](memory/client-versions-and-run-rules.md).
- `rbms-audio` 는 macOS 에서 `coreaudio-rs` 를 직접 의존한다(장치 지연 조회, `unsafe` 없음).
- 차트 라이브러리 `/Volumes/SSD/bms`(자주 빠짐). 남은 worktree: `.claude/worktrees/wf_*`(이번 세션의 것 포함, 패치는 모두 반영됨. 지워도 된다).

## 8. 다음 세션 TODO(우선순위)

0. 사용자 몫 확인: 6절 1~4. 결과를 받으면 PROCESS 에 새 R 항목으로 고친다.
1. 실기에서 나온 문제 수리(QA 3.29 항목마다 ADR 과 테스트 부채 3.163~3.187 에 관련 파일이 있다).
2. v0.1.12 게시 여부 확인(사용자). 게시 뒤 문제가 보고되면 다음 패치 릴리스로.
3. 테스트 부채 3.182~3.187: GPU 에서의 램프 반쪽, 기본 장치 따라가기 실기, 판 시작 체감, 옛 잘못 기록된 재시도 행, 검색 입력의 누르는 영역(엔진에 입력 영역 필드 필요), 조작 키와 다른 모드 레인의 편집기 안내.
4. 후속 후보: LAYOUT PICK 미리보기의 소리(M13 에서 뺌), 웹의 램프 반 쪼개기와 `EX-HARD` 표기, Windows·Linux 의 출력 지연 자동 조회(안전한 API 가 생기면).
5. 이전 세션 TODO: `docs/history/2026-11-19-handoff-snapshot.md` 8절.

## 9. 문서 지도

- [HANDOFF.md](HANDOFF.md) — 이 문서, 새 세션의 진입점(항상 덮어씀).
- [PROCESS.md](PROCESS.md) — 체크리스트와 상태. 첫 체크리스트가 2026-11-19 "기능 고도화 12건".
- [plan/2026-11-19-feature-upgrade-12.md](plan/2026-11-19-feature-upgrade-12.md) — 결정 M1~M21, 단위 U1~U10, 검토 기록(6절), 웨이브 결과(7~10절).
- [reference/2026-11-19-feature-upgrade-12/README.md](reference/2026-11-19-feature-upgrade-12/README.md) — 조사(블루투스 지연 실측 포함)와 구현 결과.
- [history/2026-11-19-feature-upgrade-unit-reports/](history/2026-11-19-feature-upgrade-unit-reports/README.md) — 구현(U1~U9)·테스트(`test-*`)·수리(`fix-*`) 단위의 보고 원문.
- [acknowledge/README.md](acknowledge/README.md) — 결정 색인(2026-11-19 ADR 9건)과 미확인 목록(39~51 이 이번 것).
- [bug/2026-11-19-borrowed-layout-retry-recorded-with-own-option.md](bug/2026-11-19-borrowed-layout-retry-recorded-with-own-option.md) — 이번에 고친 이전부터의 기록 결함.
- [play-options.md](play-options.md), [skin.md](skin.md), [crates.md](crates.md), [architecture.md](architecture.md) — 사용자·스킨·크레이트·구조 문서(2026-11-19 반영).
- [quality-assurance/](quality-assurance/) — 실기 점검표(3.29 가 이번 것), 테스트 부채(3.163~3.187 이 이번 것).
- [ci-release.md](ci-release.md), [backend/](backend/), [web/](web/), [feedback/](feedback/), [memory/](memory/), [history/](history/), [utils/](utils/), [assets/](assets/) — 이전과 같다.
