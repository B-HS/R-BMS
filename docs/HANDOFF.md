# HANDOFF — 2026-10-20 세션 갱신

최종 갱신 2026-10-19 · 대응 커밋 `46d0a01` + 문서 작업 트리(2026-10-19 갱신은 2절·3.2절·6절 11~12·8절 0 만, 나머지는 2026-10-15 스냅샷 그대로). 이 문서는 항상 덮어쓴다. 이전 스냅샷: `docs/history/2026-10-14-handoff-snapshot.md`(그 앞 `2026-10-11`, `2026-10-09`).

복기 신뢰도: 이 세션(2026-10-14 저녁 ~ 10-15)은 대화 압축 없이 원문 기준으로 복기했다(높음). 하위 에이전트가 보고한 세부 수치(계측, 테스트 수)는 그 보고를 옮긴 것이고, 메인이 직접 다시 확인한 것은 운영 DB 마이그레이션 상태, chart-notes 개수, v0.1.4 산출물 점검, CI 결과, 운영 `/api/version`·표 응답이다. 사실이 의심되면 이 문서보다 코드와 `git log` 를 믿는다.

## 1. 프로젝트 한 줄 정의

R-BMS 는 Rust 로 만든 BMS 리듬게임 플레이어(beatoraja 형식 Lua 스킨 호환 + 자작 모노톤 기본 스킨)와, 같은 저장소의 점수 서버 겸 웹사이트(`web/`, Next.js, https://bms.hyuns.uk)다.

## 2. 현재 목표

- 최종 목표: 사용자가 실제로 쓰는 배포 가능한 BMS 플레이어 + 웹(순위, 발광 난이도표, 내 폴더). 사용자는 "더 이상 구현하거나 버그가 없도록" 남은 일을 정확히 끝내길 원하고, "가끔씩" 실패하는 테스트도 없어야 한다고 했다.
- 현재 마일스톤: 클라이언트 `v0.1.4` 게시 완료(2026-10-15 07:27 UTC, 사용자가 게시. `v0.1.0`·`v0.1.3` 은 draft 로 남김). 웹 운영 배포 `51f3a1e`, 운영 DB 마이그레이션 0000~0004 적용. 진행 중인 작업 없음. 2026-10-19: 웹 3건(`b302db3`), 노트를 하나도 치지 않은 판의 Esc·종료 미저장(`84e1dc7`), 기록 목표의 리플레이 따라가기(`46d0a01`), 클라이언트 `v0.1.5` 태그와 release draft(`9ee3481`, run 39245774265, 산출물 점검 통과, 게시 대기).
- 직전 작업(이 세션): (1) 점수 제출 400(空POOR) 근본 수리, (2) 로컬 기록 서버 동기화, (3) 계약 교차 테스트·작은 수리(S18), (4) 잠재 버그 전수 감사 4회와 수리·입력 시각 판정·렉 최적화(S19), (5) 앱 아이콘·파비콘, (6) 발광 표의 다른 사이트 흔적 제거, (7) v0.1.4 릴리스와 산출물 점검, (8) 이 인수인계.

## 3. 완료 / 진행 중 / 미착수

### 3.1 완료(이번 세션)

세부 결과와 결정 원문은 [PROCESS.md](PROCESS.md)(체크리스트 S18·S19)와 [history/2026-10-15-process-archive.md](history/2026-10-15-process-archive.md), 결정의 "왜"는 [acknowledge/README.md](acknowledge/README.md).

| 묶음 | 내용 | 커밋 | 주요 파일 |
| --- | --- | --- | --- |
| 400 수리 | 서버 판정 수 검사가 空POOR(`miss`)를 노트 소비로 셈 → PG·GR·GD·BD·POOR 만(9K 는 BAD 제외). 같은 초 멱등, 프로덕션 오류 `details` 의 field·reason, 리플레이 멱등 | `865643f` | `web/src/server/service/domain/score/score.service.ts`(`judgedNoteCount`), `web/src/server/lib/api-response.ts` |
| 기록 동기화 | 판별 서버 상태(`play_submission`·`ir_sync`), 시작·로그인 자동 + `SYNC RECORDS NOW`, 리플레이 업로드, 거절은 같은 빌드끼리는 자동으로 다시 보내지 않음, Esc 포기 판 저장, 결과 화면 오류 필드 | `5f4a9a2` 외 | `apps/rbms-player/src/record_sync.rs`, `crates/rbms-ir/src/sync.rs`, `crates/rbms-store/src/scoredb/sync.rs` |
| 계약 교차 테스트 | 실제 판 경로가 만든 본문 15개 → 웹 스키마·검사·서비스 통과, `web.yml` MySQL `db` 잡 | `e9aebae` | `contract/client-requests/*.json`, `apps/rbms-player/src/app_result/record_tests/contract_requests.rs`, `web/tests/dto/client-requests.test.ts` |
| 작은 수리 | web-db status 종료 코드, 공증 점검 숨김 입력, `find-identity -v`, release concurrency, run-sql 행 수만 | `258596c` | `.github/workflows/{web-db,release}.yml`, `scripts/verify-*.sh`, `web/scripts/run-sql.ts` |
| 설정 동기화 LF | 설정 blob 줄바꿈을 모든 OS 에서 LF(Windows CI 가 잡음) | `8b6f8bb` | `apps/rbms-player/src/ir_sync.rs`(`BLOB_NEW_LINE`) |
| S19 웹 | 최고 기록 조건부 갱신, EX·판정·램프 검증(`effectiveJudgeCounts`), 노트 수 상한 제거, 범위 검증(400·잘라 저장), 교착 재시도(`withLockRetry`, 세션 잠금 대기 5초), 최고 기록 복구 경로, 인증 오류 구분, 토큰 폐기(`DELETE /api/auth/token/current`), 운영 env 검증, public-reads, proxy 404, 공동 순위, 설정·폴더 경합, 리플레이 한도, 마이그레이션 0003·0004, chart-notes 점검·복구 SQL | `b197446`, `0e7be91`, `e7890fc`, `9e2cc0e` | `web/src/server/**`, `web/drizzle/0003_*`·`0004_*`, `web/scripts/chart-notes.*.sql` |
| S19 클라이언트 | 떠는 테스트(캡처 하네스 시계·스킨 Lua 예산 시계·숫자 행 시각·테스트 타임아웃·임시 파일), 저장 전용 스레드, pending-runs, 코스 기록 저장·재전송, 리플레이 GC 보호, 계정(토큰 미업로드·로그인 계정·0600·로그아웃 폐기), 오토플레이 지뢰·CN/HCN, 판정 그래프·RunTrace·BGA·키음 최적화, 가림 처리, BGA 미리 칠하기·상주 256 MiB·텍스처 슬롯 재사용, 연습 FREQUENCY·게이지 잠금, `#TOTAL`·CR 파싱, 종료·코스 포기 저장, `--help`/`--version` | `f81d499`, `4cc72d0`, `1676c57`, `c81eda1` | `apps/rbms-player/src/{save_worker,record_sync,command_line}.rs`, `gpu/{present,registry}.rs`, `crates/rbms-render/src/skin_render/bga/**` |
| 판정 의미 9건 | avgjudge 원본 정의, 입력 이벤트 시각 판정, TOTAL 덮어쓰기 어시스트, 게이지 사망 BP, 코스 곡별 콤보·램프, 풀콤, LN 몸통 안 노트, 짝 없는 LN 머리, Esc 판 전부 저장. 기록 규칙 세대 4, `PARSER_VERSION` 5 | `f81d499` 외 | `crates/rbms-judge/src/matcher.rs`, `crates/rbms-play/src/session.rs`, `crates/rbms-chart/src/long_note.rs` |
| 키 입력 시각 | 화면 대기·present 를 전용 스레드 `rbms-present` 로(이벤트 루프는 `ControlFlow::Wait`), LR2 방식 디바운스(키보드 KEY DEBOUNCE, 컨트롤러 PAD DEBOUNCE, 기본 16 ms, 턴테이블은 원본 잠금) | `794361b`, `1676c57` | `apps/rbms-player/src/gpu/present.rs`, `apps/rbms-player/src/input_debounce.rs`, `stage/play/key_gate.rs`, `gamepad.rs` |
| 앱 아이콘·파비콘 | 무채색 레인 아이콘, macOS `.icns`, Windows `rc.exe` 리소스(새 의존성 없음), 창 아이콘, 웹 favicon·icon·apple-icon | `cc500ef`, `089c737` | `assets/icon/**`, `apps/rbms-player/build.rs`, `scripts/windows-icon-resource.ps1`, `web/src/app/{favicon.ico,icon.png,apple-icon.png}` |
| 다른 사이트 흔적 | 출처·삭제 요청 문구, 정보 패널 출처, 카드 출처, header `original_url`·`mirror_of`, 요약 API `source`, 곡 데이터 `comment`·artist 주소 제거. 다운로드 링크(`url`·`url_diff`·`name_diff`)는 유지 | `4a0872d`, `ea34002`, `4fda17c`→`dd5518e` | `web/src/widgets/table/**`, `web/data/tables/insane1/**`, `web/scripts/snapshot-table.ts` |
| 릴리스 | v0.1.4(run 38696475211). 메인 점검: SHA-256, dmg·앱 티켓, Gatekeeper, codesign, syspolicy_check, 0.1.4, 아이콘, `x86_64 arm64`, Windows exe `RT_ICON` 7 | `87eacd0`, `ea698a9` | `docs/ci-release.md` 2.3 |
| 운영 | 마이그레이션 0003(run 38627474716)·0004(run 38673724720), chart-notes 점검(어긋남 0, 복구 미실행), 상류 이슈 gfx-rs/wgpu#10627 | — | `.github/workflows/web-db.yml` |

### 3.2 진행 중

웹 AI 기능 W1~W6 구현·배포 완료(PROCESS 의 "웹 AI 기능 기획" 체크리스트, 결정은 [W0 결정 ADR](acknowledge/2026-10-19-adr-web-ai-decisions.md) 과 그 끝의 메인 결정 표). 운영 DB 0010 까지, 곡별 패턴 7,903곡(発狂 1,035/1,035), 클라이언트 v0.1.5 게시 대기·v0.1.6 draft. 다음 한 줄: 사용자가 AI 키 등록·진단, v0.1.6 게시, 실기로 리포트 확인.

### 3.3 미착수

8절의 우선순위 목록이 정본이다.

## 4. 의사결정 요약

정본은 [acknowledge/README.md](acknowledge/README.md). 아래는 이 세션의 요지와 기각 이유(파일 이름의 날짜 접두는 `2026-10-14-adr-`·`2026-10-15-adr-`).

| 결정 | 채택 | 기각된 대안과 이유 | ADR |
| --- | --- | --- | --- |
| 판정 수 검사 | 노트를 소비한 판정만(9K BAD 제외) | 검사 삭제 — 과다 판정을 막는 검사가 사라짐 | `judge-count-consumed-notes` |
| 결과 화면 오류 | field·reason 표시 | 미표시 — 원인이 "요청 형식"으로만 보여 이번 사고를 늦게 찾음 | `ir-error-field-display` |
| 기록 동기화 | 로컬 이력 전체를 보내고 실패한 판만 다룸(사용자 원문 "그냥 서버에 전체 보내서 실패하면 실패한거만 잘 처리해놔"), 서버 멱등(같은 초) | 수락 미확인만 선별(메인 추천) — 사용자가 전체 전송 택. 클라이언트만으로 중복 거르기 — 다른 기기 판을 모름 | `record-sync`, `record-sync-refusal-resend` |
| Esc·종료·코스 포기 | 노트를 하나라도 쳤으면 FAILED 로 저장·전송, 남은 노트 없으면 결과로. 노트를 하나도 치지 않은 판(PG·GR·GD·BD 0, 쥔 롱노트 없음. 빈 POOR·지나간 노트의 POOR 만 있어도)은 저장·전송하지 않음(2026-10-19 사용자 지시, `escape-keeps-every-run` 결정 7) | 판정 없는 판 미저장·코스 기록 미전송(메인 제안, 2026-10-15) — 사용자 "실패기록도 다 서버로 보내야지". 판정 없는 판의 제외는 10-19 에 사용자가 직접 지시한 "노트를 하나도 치지 않은 판"으로만 들어왔다 | `abandoned-run-saved`, `escape-keeps-every-run` |
| 판정 의미 9건 | 원본(beatoraja)에 맞춤 | 지금대로(B) — 원본과 다름 | 2026-10-15 ADR 9건(avgjudge, event-time, total-override, failed-run-bp, course-stage-combo, full-combo, long-note-body, unpaired-heads, escape) |
| 입력 시각 | 이벤트 시각 판정 + 시각 단위 엔진 진행, 키보드는 present 대기 분리 스레드(IIDX·LR2·beatoraja·osu! 의 공통 원리) | 프레임 기준 유지 — 리플레이 비결정. Mailbox — Metal 드로어블 풀 때문에 효과 없음. 처리 시각 유지. OS 이벤트 시각 — unsafe·새 의존성. 게임 로직 스레드 분리 — `SkinLibrary`(Lua `Rc`) 가 Send 아님 | `event-time-judging`, `keyboard-input-timing` |
| 디바운스 | LR2 방식(누름 즉시, 창 안 뗌만 미룸), 기본 16 ms, 키보드·컨트롤러 모두 설정에서 조절, 턴테이블은 원본 잠금 | 양방향 잠금 — 정상 연타 늦춤. 턴테이블 LR2 — 감도가 원본과 달라짐 | `keyboard-input-timing` |
| wgpu Metal 위반(THR2-1) | 구조 유지, 알려진 위험, 상류 이슈 | macOS 만 메인 획득으로 되돌림 — 키 지연 복귀. wgpu 패치 사본 — 새 의존성 규칙 예외 | `wgpu-metal-main-thread` |
| BGA 메모리 | 상주 상한 256 MiB 유지(YATA 1,144 → 1,504 MB) | 상한 낮춤 — 플레이 중 업로드 프레임 | `bga-resident-texture-cap` |
| 렉 기준 | 프레임 스레드 I/O·큰 할당·잠금·동기 네트워크 없음, p99 1 ms, 언더런 0 | — | `play-hitch-criteria` |
| 앱 아이콘 | 무채색 레인 그림, 생성 스크립트, Windows `rc.exe`+`build.rs` | `winresource` 의존성 — 새 의존성 금지 | `app-icon` |
| 표 페이지 | 다른 사이트 흔적 제거, 다운로드 링크 유지 | 출처 유지. 다운로드 링크까지 삭제(`4fda17c`) — 사용자 정정 "그건 지우면안되는데". D안(표 사이트 주소만) — 결과적으로 comment·artist 주소·original_url 만 제거와 같아짐 | `table-page-no-source-notice` |
| 릴리스 | v0.1.3 은 게시하지 않고 v0.1.4 로, v0.1.0·v0.1.3 draft 는 둔다, 고도화까지 넣어 한 번에 | draft 삭제 — 사용자 "놔둬" | `v0-1-4-release` |
| 사고 49판 | 운영에 그대로 둔다("나중에 테스트할떄 참조가능할수도") | 삭제 | `accidental-sync-records-kept` |

## 5. 사용자 방향성 & 작업 규칙

- 응답: 한국어 존댓말만(짧은 상태 보고 포함 — 영어로 바뀌어 두 번 지적받음), 간결, 첫 줄은 행동이나 답, 매 응답에 현재 상태 한 줄, 번호 목록, 이모지·글자 그림 금지. 확인하지 못한 것은 그렇다고 말한다. 진행 예측을 물으면 근거를 들어 정직하게 답한다.
- 새 실행 작업과 세션 경계(새 세션, resume, compact, clear, prepare-new)마다 다중 에이전트 workflow 사용 여부를 한 번 묻고 답을 기다린다. 선택은 승계하지 않는다.
- workflow 를 쓰면: Workflow 도구만, Fable·Haiku 금지(이 프로젝트), Opus 는 effort high 까지, Sonnet 은 xhigh 까지, 모든 agent 에 model·effort 명시. 에이전트는 커밋·푸시하지 않는다. 에이전트가 `rbms-player` 를 실행하면 반드시 scratch HOME 과 `--server http://127.0.0.1:9`(2026-10-15 사고, [feedback](feedback/2026-10-15-run-the-player-with-a-scratch-home.md)).
- 근본 원인을 고친다(증상 가리기·허용 오차·재시도로 덮기 금지). 실제 측정·캡처·내려받은 산출물로 확인한 뒤 됐다고 한다. 떠는 테스트는 0 이어야 한다.
- 판정·입력은 원본(beatoraja)·IIDX·LR2 와 같게. 판정·점수 의미가 원본과 달라지는 수정은 고치기 전에 사용자 확인.
- 기록: 사람이 친 판은 실패·포기·종료 포함 전부 저장·전송. 제외: 노트를 하나도 치지 않은 판(사용자 지시 2026-10-19). 메인이 제외 조건을 추천하지 않는다(필요해 보이면 질문으로)([feedback](feedback/2026-10-15-do-not-exclude-runs.md)).
- 외부 게시(다른 저장소 이슈·댓글·PR)는 초안 전문을 보여 주고 명시적 승인 뒤에만. 추천안 선택은 게시 승인이 아니다([feedback](feedback/2026-10-15-confirm-before-external-posts.md)).
- 지우라는 요청은 무엇이 함께 사라지는지(예: 다운로드 링크) 분명히 확인하고 지운다([feedback](feedback/2026-10-15-delete-requests-keep-what-works.md)). 다른 사이트의 흔적을 싫어한다.
- Git: Conventional Commits, 한국어 설명, author 사용자 단독, AI 트레일러 금지, 선별 스테이징(`git add -A` 금지), force push 금지. 게이트 통과 뒤 논리 단위로 커밋·푸시하고 `gh run list --branch dev` 로 3 OS CI 와 web CI 를 확인한다.
- Rust: `//` 주석 금지(`//!`·`///` 영어만), 매직넘버 금지, 새 `#[allow]`·`unsafe`·`todo!()` 금지, rustfmt max_width 160, 새 의존성 금지, `--locked`. 웹: `web/AGENTS.md`(arrow function, any·enum·주석 금지, FSD, TanStack Query + queryOptions + QUERY_KEY, Zod 경계 검증, useCallback·useMemo 금지).
- 비밀: `.env`·`.env.*`·`secrets/`·키 파일·토큰이 든 설정 파일(`~/.config/rbms/settings.ron` 등)을 읽거나 쓰거나 출력하지 않는다. 시크릿 값이 대화에 남는 방법 금지(별도 터미널 `gh secret set`). `vercel link` 금지. 점수 DB(`scoredb.sqlite`)는 scratch 로 복사한 사본만 읽는다.
- 운영: 운영 사이트에는 읽기 요청만. 운영 DB 는 `web-db.yml` 로, 바꾸는 작업은 개수를 보여 주고 승인 뒤(마이그레이션처럼 추가만 하는 것은 진행해 왔다). 새 웹 코드가 새 마이그레이션에 기대면 마이그레이션 파일을 먼저 커밋·적용하고 코드를 나중에 푸시한다(0004 선례).
- 릴리스: 태그 + draft 까지 자동, 메인이 산출물을 내려받아 점검, draft 본문은 사실 확인 뒤 정확히(다른 사이트 이름 넣지 않음), 게시는 사용자. 결함이 든 draft 는 다음 패치 버전으로.
- 폴더 금지: `/Users/hyunseokbyun/Downloads/ModernChic`, `/Users/hyunseokbyun/development/beatoraja` 는 읽기만. ModernChic 자산을 저장소에 복사하지 않는다.

## 6. 미해결 질문 / 사용자 확인 필요

1. **v0.1.4 실기 확인**(결정 아님): 결과 화면 `IR: RANK #n`, 설정 NETWORK 의 `SYNC RECORDS NOW`, 키 입력 체감과 JUDGE OFFSET 재조정, PAD DEBOUNCE, 첫 실행의 곡 DB 전체 재스캔. 사고 때 개발 빌드가 다시 쓴 `settings.ron` 이 의도대로인지. 대기 중인 리플레이 43개가 올라가는지.
2. **순위 `lnmode` 필터의 뜻**: 지금은 "최고 기록이 그 LN 방식인 플레이어만". (a) LN 방식별 최고 기록(마이그레이션) / (b) 파라미터 제거 / (c) 지금대로. `docs/backend/api-spec.md` 4절, test-debt 3.35.
3. **코스 램프 서버 검사**: 0.1.0 이 마지막 스테이지 램프를 보내 꺼 둠. 0.1.0 이 쓰이지 않게 되면 켤지. test-debt 3.36.
4. **저장소에 남은 다른 사이트 흔적(서빙 안 함)**: `web/data/tables/insane1/source.json`, `web/scripts/snapshot-table.ts` 사용 예, `crates/rbms-config/src/tables.rs` 의 `same_as`(원본 주소 등록자에게 중복 등록 방지), 조사 문서. 메인 권고: `tables.rs`·`source.json` 은 유지.
5. **거절 재전송 좁힘 확인**: 자동 동기화는 같은 빌드끼리 거절된 판을 다시 보내지 않음(수동·빌드 변경 때 보냄).
6. **메뉴의 PAD DEBOUNCE**: 메뉴에서 뗄 때 튀는 버튼이 두 번 눌릴 수 있음. 메뉴만 옛 양방향 잠금을 둘지. test-debt 3.71.
7. **연습 판의 게이지 사망**: 지금은 게이지 잠금으로 구간 끝까지 판정, 원본은 연습 패널로 돌아감. 원본대로 / 지금대로.
8. **램프 이름 두 가지**: 앱 화면(`format.rs`)은 "LIGHT ASSIST"·"ASSIST EASY", 기본 스킨(`assets/skins/rbms-default/lua/theme.lua`)은 "LIGHT ASSIST CLEAR"·"ASSIST CLEAR". 통일할지.
9. **TOTAL 행 경고**: 설정의 TOTAL 행이 "직접 정하면 어시스트(최고 기록 없음)"라고 말하지 않는다(`apps/rbms-player/src/stage/settings/info.rs` 의 `CUSTOM_JUDGE_CAUTION` 은 JUDGE WIDTH·LN MARGIN 만). KNOWN ISSUE, 고칠지.
11. **기록 목표 설계 선택(2026-10-19, 메인이 원본 기준으로 구현, 사용자 미확인)**: 기준 인덱스는 시각이 아니라 판정된 노트 수(원본 `passnotes`), 스킨 BEST 칸도 같은 기준, RIVAL·다른 기기 IR BEST 는 비례, 같은 EX 여럿이면 처음 낸 판, 고스트를 로딩이 기다림. 상세와 근거는 [ADR](acknowledge/2026-10-19-adr-record-target-follows-its-run.md), [reference/record-target-pace.md](reference/record-target-pace.md), acknowledge/README 미확인 7.
12. **비례로 물러날 때 화면 표시**: 리플레이가 없거나 다시 판정이 기록과 다르면 범례가 따로 말하지 않는다(test-debt 3.78). 2026-10-14 ADR 원칙대로 `LOCAL BEST (NO REPLAY)` 처럼 표시할지. 또 앱 종료도 Esc 와 같은 "친 노트 0 이면 남기지 않음"을 따르게 했다(Esc ADR 결정 7, 사용자 원문은 Esc 만 말함).
10. 오래된 것: 웹 rate limit 지속형 저장소(Vercel Firewall / Upstash / 지금대로), `user.rank_points`(항상 0) 처리.

## 7. 환경 & 전제

- 저장소 `/Users/hyunseokbyun/development/R-BMS`, 브랜치 `dev`, 원격 `B-HS/R-BMS`(공개). `prod` 브랜치는 없고 릴리스는 `dev` 에서 `gh workflow run release.yml --ref dev -f bump=patch`. Rust 1.95.0(`rust-toolchain.toml`), 웹 bun 1.4.0, Cargo 버전 0.1.5.
- 게이트: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`, `cargo test --workspace --no-fail-fast --locked`(마지막 6,024 통과·6 ignored), 웹 `cd web && bun run verify && bun run format:check && bun run build`(653 통과), DB 통합 `RBMS_DB_E2E=1 bun test tests/db`(Docker `mysql:8.4.11`, 끝나면 컨테이너 삭제), 스킨 `cargo run -q -p rbms-cli --locked -- skin-dump assets/skins/rbms-default --strict --frames 120`, 이미지 `uv run assets/skins/rbms-default/tools/generate-images.py --check`, 아이콘 `uv run assets/icon/tools/generate-icon.py --check`, 표 스냅샷 `cd web && bun scripts/snapshot-table.ts https://darksabun.club/table/archive/insane1/ data/tables/insane1 --check`, 계약 `cargo test -p rbms-player --locked contract_requests` + `bun test tests/dto/client-requests.test.ts`(갱신은 `RBMS_UPDATE_CONTRACT=1`).
- CI: `ci.yml`(Rust 3 OS + fmt/clippy, Windows 는 아이콘 리소스도 컴파일·링크), `web.yml`(typecheck·lint·test·format·build + MySQL `db` 잡, `contract/**` 변경에도 돎), `release.yml`(수동, concurrency), `signing-check.yml`, `web-db.yml`(`status|migrate|cleanup-count|cleanup-delete|chart-notes-count|chart-notes-repair|chart-notes-restore|chart-notes-drop-backup`).
- 저장소 시크릿: `MACOS_CERTIFICATE_P12`, `MACOS_CERTIFICATE_PASSWORD`, `APPLE_ID`, `APPLE_APP_SPECIFIC_PASSWORD`, `APPLE_TEAM_ID`, `WEB_DATABASE_URL`.
- Vercel: 프로젝트 `rbms-web`, Git 연결, Root `web`, Production `dev`, `web/vercel.json` ignoreCommand(`web/` 변경 때만 배포). 운영 `/api/version` 의 commit 으로 배포 확인.
- 운영 DB: 마이그레이션 0000~0004. chart-notes 점검(run 38673771025): 점수 없는 차트 행 3, 누적 거절 39. 사용자 계정의 판은 대부분 SCRATCH AUTO 라 기본 순위 목록에서 숨겨진다(설계대로).
- 차트 라이브러리: `/Volumes/SSD/bms`(11,336 차트), `/Volumes/SSD/2026-07-31-backup/Documents/personally/1`(727 차트). SSD 는 자주 빠진다(옛 경로 `~/Documents/personally/1` 은 없음, `docs/memory/test-library.md` 참고). 오토플레이 만점 검사의 예외는 `airTKkill.bme`·`airTkill.bme`(#TOTAL 1.1·1, 원본도 FAILED) 둘뿐이다. 이 검사 도구(corpus)는 세션 scratch 에만 있었고 저장소에 없다.
- 클라이언트 값: 기록 규칙 세대 4(`SCORE_RULE_VERSION`), `PARSER_VERSION` 5(새 빌드 첫 실행에 전체 재스캔), 설정 스키마 6, `play.key_debounce_ms`(기본 16), keyconfig `pad.debounce_ms`(PAD DEBOUNCE), BGA 상주 `RESIDENT_BYTES`·`PREPARED_BGA_BYTES` 256 MiB, 동기화 간격 점수 1.5초·리플레이 7.5초. 정리: [memory/client-versions-and-run-rules.md](memory/client-versions-and-run-rules.md).
- 이름 주의: `apps/rbms-player/src/ir_sync.rs` 는 **설정** 동기화이고, DB 표 `ir_sync` 는 **기록** 동기화 상태다.
- 앱을 띄우면(테스트 실행 포함) 기본 설정의 표 주소 `https://bms.hyuns.uk/tables/insane1` 을 GET 한다(쓰기 아님).
- macOS 는 파일 시스템이 대소문자를 구분하지 않으므로 `docs/ARCHITECTURE.md` 를 따로 만들지 않는다. 아키텍처 정본은 [architecture.md](architecture.md).

## 8. 다음 세션 TODO(우선순위)

0. 사용자 확인: v0.1.5·v0.1.6 draft 게시, 웹 설정 AI 키 탭에서 세 공급자 키 등록·다시 검증(Ollama 는 구조화 출력이 보장되지 않아 형식 오류가 날 수 있음), v0.1.6 으로 몇 판 친 뒤 `/ai` 리포트·코치 확인. 메인이 외출 중 정한 설계 선택(W0 결정 ADR 끝의 표, 특히 라이벌 공유를 서로 라이벌로 좁힌 것)을 보고하고 바꿀 것이 있으면 반영. 6절 11·12 답도.
1. 사용자 확인(6절 1)을 받고 실기에서 나온 문제를 고친다. 6절 2~9 는 사용자가 답하면 해당 ADR 을 쓰고 구현한다(질문은 한 번에 묶어서).
2. 실제 창 QA: `docs/quality-assurance/2026-10-11-default-skin-manual-check.md` 맨 위 "먼저 확인할 것", 3.18 의 K1~K8(키 입력·present 스레드·가림·Main Thread Checker), 나머지 표(미체크 126줄). Windows·Linux 실기(test-debt 3.3·3.4). 사람이 해야 하므로 결과를 받아 고친다.
3. 테스트 부채 정리([quality-assurance/2026-10-14-test-debt.md](quality-assurance/2026-10-14-test-debt.md) 3절): 우선 데이터·정확성에 닿는 것 — 3.72(같은 판 동시 중복 제출 `ER_DUP_ENTRY` 500, 재시도 밖 쓰기), 3.74(관리자 표 저장 중복 키 500), 3.55(키보드·컨트롤러 섞어 칠 때 순서), 3.56(옛 FRAME 리플레이 재생), 3.12(서버 응답 → Rust 반대 방향 계약), 3.73(`rbms-cli config` 쓰기 부작용), 3.37(번들 표 없는 id 200). 그다음 실기 의존 항목.
4. 오토플레이 만점 검사를 저장소 도구로 남길지 결정(지금은 매번 scratch 에서 다시 만듦). 남긴다면 `apps/rbms-cli` 의 하위 명령이 자연스럽다.
5. 클라이언트 한계: 설정 화면 컨트롤러 조작(`apps/rbms-player/src/stage/settings.rs`), 게임 안 IR 순위 패널의 SCRATCH AUTO 토글, 턴테이블 노트만 있는 차트의 자동 종료, 초록 노트를 노트 그림 모양으로(`apps/rbms-player/src/gpu`), 곡 로딩의 남은 프레임 스레드 작업(test-debt 3.39).
6. 웹 2차([web/tasks.md](web/tasks.md)): 한 곡 한 행 보기, 폴더 끌어 옮기기·2,000곡 가상화, 표 RSC 자료 축소, OG 이미지, 홈 대시보드, 릴리스 CI 의 `client_build` 등록 경로.
7. wgpu 를 올릴 때 gfx-rs/wgpu#10627 이 고쳐졌는지 확인(QA K8). Windows 코드 서명은 사용자가 원할 때.

## 9. 문서 지도

- [HANDOFF.md](HANDOFF.md) — 이 문서. 새 세션의 진입점(항상 덮어씀).
- [PROCESS.md](PROCESS.md) — 체크리스트와 현재 상태만(S1~S19). 세부는 [history/2026-10-15-process-archive.md](history/2026-10-15-process-archive.md), 그 전은 [history/2026-10-14-process-archive.md](history/2026-10-14-process-archive.md).
- [architecture.md](architecture.md) — 아키텍처 정본: 저장소 배치, 크레이트 의존, Stage, 플레이 → 기록 DB → 저장 스레드 → 동기화 → 점수 서버 → 웹, 스레드, 배포, 앱 아이콘.
- [crates.md](crates.md) — 크레이트별 공개 API·모듈·테스트.
- [development.md](development.md) — 개발 명령, 환경 변수(`RBMS_UPDATE_CONTRACT`, `RBMS_WINDOWS_RESOURCE` 등), 영속 파일, 모듈 표.
- [play-options.md](play-options.md) — 플레이 옵션, 조작, 입력 시각·디바운스(3.4), 기록이 남는 조건(10절, 동기화 10.9, 규칙 세대 10.11).
- [skin.md](skin.md), [theme.md](theme.md), [font-cjk-support.md](font-cjk-support.md) — 스킨 엔진, 앱 UI 테마, 글꼴(당시 기록).
- [ci-release.md](ci-release.md) — CI, 릴리스 흐름·이력(v0.1.0~v0.1.4), 서명·공증, web-db, KNOWN ISSUE.
- [acknowledge/README.md](acknowledge/README.md) — 결정 색인과 미확인 결정 목록. `reference-divergences.md` 는 원본과의 의도적 차이.
- [plan/](plan/) — 설계서(`2026-10-14-record-sync.md` 기록 동기화 계약, 설정 화면, 사용자 폴더, 기본 스킨 등).
- [web/](web/) — 웹: `architecture.md`, `components.md`, `tasks.md`(남은 일), `2026-10-14-audit.md`(점검과 해소), `2026-10-14-foundation-contract.md`, `2026-10-14-insane-table.md`, `2026-10-14-design-direction.md`.
- [backend/](backend/) — 점수 서버 API 명세(`api-spec.md`), 데이터 모델(`data-model.md`), 호환성, 엔드포인트 작업.
- [reference/](reference/) — `ir-api.md`(클라이언트 IR 와이어·동기화), `frame-cost.md`(프레임 비용 계측), `cn-hcn-judgment.md`, `wgpu29-winit030-api.md`, `skin-compat/`(원본 조사).
- [quality-assurance/](quality-assurance/) — `2026-10-14-test-debt.md`(테스트 부채 3.1~3.74), `2026-10-11-default-skin-manual-check.md`(실제 창 QA, K1~K8).
- [bug/](bug/) — `2026-10-14-score-submit-400-large-seed.md`, `2026-10-14-score-submit-400-empty-poor.md` 등.
- [feedback/](feedback/) — 지적받은 것(한국어 응답, 외부 게시, 판 제외 금지, 지우기 요청, scratch HOME 등).
- [memory/](memory/) — `client-versions-and-run-rules.md`, `test-library.md`.
- [utils/](utils/) — 보조 도구(`class-disassembler.py`: jbms-parser 바이트코드 읽기).
- [history/](history/) — 이전 스냅샷과 이력.
- 저장소 밖: `assets/icon/README.md`(아이콘 생성), `web/AGENTS.md`(웹 규칙).
