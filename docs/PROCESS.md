# R-BMS — PROCESS (작업 체크리스트와 상태)

최종 갱신 2026-10-19 · 대응 커밋 `e2c829f`

이 문서는 체크리스트와 현재 상태만 둔다. 끝난 구간의 세부 결과, 웨이브 기록, 사용자 결정 원문은 `docs/history/` 로 옮긴다(ai-process 2.1). 옛 0~9절과 Phase 0~7 로드맵도 같은 곳에 있다(아래 "아카이브").

> 새 세션은 [HANDOFF.md](HANDOFF.md)(맥락, 결정, 작업 규칙, 우선순위 TODO. 항상 덮어쓴다)를 먼저 읽고 이 문서로 체크리스트를 확인한다. 이전 스냅샷은 `history/*-handoff-snapshot.md`.
> 베이스 룰: 프로젝트 작업 규칙의 단일 출처는 HANDOFF 5절이다. 코딩 컨벤션은 사용자 전역 컨벤션을 따르되 Rust 에는 TS/JS 전용 규칙을 적용하지 않고, 웹(`web/`)은 `web/AGENTS.md` 를 따른다.
> 위치: 저장소 루트 `/Users/hyunseokbyun/development/R-BMS`, 원격 `B-HS/R-BMS`, 작업 브랜치 `dev`. `prod` 브랜치는 만든 적이 없고(`release.yml` 은 `prod` 푸시도 받게 돼 있지만 쓴 적이 없다) 릴리스는 `dev` 에서 `release.yml` 을 수동 실행한다([ci-release.md](ci-release.md) 2절).
> 명령: 빌드, 테스트, 게이트는 [development.md](development.md)의 "Build / run / test", 실행 정본은 루트 `README.md`. 툴체인은 `rust-toolchain.toml` 로 `1.95.0` 고정이고 CI 는 `cargo fmt --all --check` 와 `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` 를 통과해야 한다. 테스트 수는 시점 값이라 여기에 적지 않는다(마지막 게이트 기록은 S19 마무리의 Rust 6,024 통과).
> Git: Conventional Commits, 설명은 한국어([acknowledge/2026-09-17-commit-language.md](acknowledge/2026-09-17-commit-language.md)), author 사용자 단독, AI 트레일러 금지, 선별 스테이징, force push 금지. 게이트를 통과하면 논리 단위로 커밋·푸시하고 `gh run list --branch dev` 로 CI 를 확인한다. `Cargo.lock` 은 추적한다(2026-10-11 부터, 결정 D12). `target/` 과 곡 차트는 커밋하지 않는다.

## 상태 (2026-10-15)

- 클라이언트 `v0.1.4` 는 2026-10-15 07:27 UTC 에 게시됐다(release run 38696475211, 버전 커밋 `87eacd0`). `v0.1.3`·`v0.1.0` draft 는 게시하지 않고 남겨 뒀다. 릴리스 이력은 [ci-release.md](ci-release.md) 2.3절.
- 웹 운영 배포는 `51f3a1e`(`/api/version` 으로 확인, 이후 커밋은 `web/` 를 바꾸지 않음). 운영 DB 마이그레이션은 0000~0004 적용(`web/drizzle/`, 실행은 `web-db.yml`).
- 2026-10-19 "웹 기록 표시와 구동기 Esc·목표 보완" 완료, 클라이언트 `v0.1.5` draft 게시 대기(사용자). 열린 확인 사항은 [HANDOFF.md](HANDOFF.md) 6절, 남은 일은 8절.

## 체크리스트 — 웹 AI 기능 기획 (2026-10-19)

사용자 지시(2026-10-19): 웹에 AI 기능을 만든다. 먼저 기획. 기능 10개 모두 구현 방향 — 플레이어 리포트(약점·선호 패턴 등 리플레이로 얻을 수 있는 모든 정보 종합), 리플레이 코치, 베스트·라이벌 비교, 판정 습관 진단, 다음 도전 곡 추천, AI 연습 폴더(MY FOLDERS), 목표 계획, 게이지·옵션 조언, 자연어 기록 질문(필터 스키마), 주간 성장 요약. AI 키는 Anthropic(API key), OpenAI(Codex, API key), Ollama Cloud(API key) 셋만.
사용자 결정: 키는 사용자 BYOK + 암호화. 패턴 데이터는 판별(클라이언트가 실제 배치·노트별 판정을 올림) + 곡별(발광 전곡을 rbms-cli 로 뽑아 md5 로 표와 1:1). 서버의 차트 패턴은 노트 배치 전체를 비공개 저장("최대한 정확한 리포트"), 공개 API 로 내보내지 않음. 집계·패턴 분류는 일반 코드, AI 는 해석·설명·추천만. 공급자는 같은 구조화 출력 스키마의 어댑터 계층. workflow 사용. 곡별 패턴 추출의 원본 폴더는 `/Volumes/SSD/bms`(사용자 2026-10-19 "여기가 맞을거야", 추출 때 SSD 연결 필요, 원본 파일은 업로드하지 않음).

- [x] AI1. 조사 — 웹 서버 데이터·보안 경계, 클라이언트가 판 끝에 아는 것·리플레이·rbms-cli, 공급자 3곳 공식 API(docs/reference/ai-features/ 에 저장, workflow wf_2bde79bf-eeb)
- [x] AI2. 기획 문서 — docs/plan/2026-10-19-web-ai-features.md(기능 10개, 데이터 계약, 분석 엔진, 공급자 어댑터, 키 암호화, 단계 계획, 사용자 확인 질문)
- [ ] AI3. 메인 검토와 사용자 확인 질문 정리, 결정 ADR

## 체크리스트 — 웹 기록 표시와 구동기 Esc·목표 보완 (2026-10-19)

사용자 지시(2026-10-19) 원문 요지. 웹: (1) L-ASSIST 가 클리어로 세어지지 않는다, FAILED 가 아닌 램프는 모두 클리어, (2) SCRATCH AUTO 포함 토글의 기본값을 켬으로, (3) 플레이어의 최근 기록이 20개까지만 보인다, 페이지로 전체 플레이 기록을 보이고 제목을 "플레이 기록"으로. 구동기: (4) 노트를 하나도 치지 않고 Esc 를 누른 판은 저장하지 않는다, (5) LOCAL BEST·IR BEST(MY BEST) 같은 기록 목표는 퍼센트 비례가 아니라 그 기록의 리플레이를 같은 시점·노트 단위로 따라간다. 사용자 결정: workflow 사용, 리플레이가 없는 기록 목표는 지금처럼 퍼센트로 대체. 추가 지시(같은 날): "빈 poor도 사실상 안친거니까 그것도 저장안하는걸로", 놓친 노트의 POOR 도 "말그대로 아무것도 안쳤을때 ... 그것도 너가 판단해봐" → 메인 판단: 입력으로 판정된 노트가 0 이면(빈 POOR·지나간 노트의 POOR 만 있어도) 저장하지 않는다. 다음 클라이언트 버전은 v0.1.5, 태그 + release draft 까지.

- [x] W1. 웹 클리어 기준 — `MIN_CLEAR_LAMP_ID` 를 FAILED 다음(ASSIST)부터로, 화면의 클리어 집계 전부
- [x] W2. 웹 SCRATCH AUTO 포함 기본 켬 — 화면 토글·URL 파라미터·서버 읽기 기본값
- [x] W3. 웹 플레이 기록 페이지네이션 — 플레이어 전체 판, 제목 "플레이 기록" (W1~W3 `b302db3`, [ADR](acknowledge/2026-10-19-adr-web-clear-and-play-history.md))
- [x] P1. 구동기 판정 0 판의 Esc — 저장·전송하지 않음, Esc ADR 갱신 (`84e1dc7`, 기준: PG·GR·GD·BD 0 이고 쥔 롱노트 없음, [Esc ADR 결정 7](acknowledge/2026-10-15-adr-escape-keeps-every-run.md))
- [x] P2. 구동기 기록 목표의 리플레이 따라가기 — 조사·설계 뒤 구현, 목표 ADR 갱신 (`46d0a01`, [ADR](acknowledge/2026-10-19-adr-record-target-follows-its-run.md), [조사](reference/record-target-pace.md), 미확인 선택은 HANDOFF 6절 11·12)
- [x] G. 게이트(fmt·clippy·cargo test·웹 verify·format:check), 커밋·푸시, CI 확인, 문서 정리 — workflow 게이트 Rust 6,044 통과·0 실패, 웹 676 통과·0 실패, DB 통합 62 통과(작업자). 리뷰 뒤 수정은 rbms-player lib 2,559 통과·clippy 0. 커밋 `b302db3`·`84e1dc7`·`46d0a01`·`109061c`, CI ci run 39244031688·web run 39244031816 통과, 운영 웹 `/api/version` = `109061c`
- [x] R. 클라이언트 v0.1.5 — 버전 커밋, 태그, release draft(산출물 내려받아 점검, 본문 사실 확인). 게시는 사용자 — run 39245774265, `9ee3481`, 점검 결과는 [ci-release.md](ci-release.md) 2.3절. 게시 대기

## 체크리스트 — 웹 배포 준비와 클라이언트 보완 (2026-10-14 ~ 10-15)

사용자 지시 (가)~(사)와 항목별 결정 원문, 웨이브 결과(16~20, 17A, 17B-1, 17B-2), 2026-10-15 사고 기록은 모두 [history/2026-10-15-process-archive.md](history/2026-10-15-process-archive.md)에 있다. 결정의 "왜"는 [acknowledge/README.md](acknowledge/README.md) 색인의 ADR.

- [x] S1. 기록 저장 점검 — 원인은 로그인이 아니라 SCRATCH AUTO 판의 최고 기록 규칙이었고, 리플레이가 설정을 덮던 결함도 고침(웨이브 16, `4bed711`)
- [x] S2. 목표 점수 — AAA 보고는 재현되지 않았고, LOCAL BEST 가 없을 때 조용히 MAX 로 물러나던 것을 TARGET FALLBACK(기본 RATE AAA)·RATE CUSTOM 으로 고침([ADR](acknowledge/2026-10-14-adr-score-target-fallback.md))
- [x] S3. 설정 화면(Tab) 재구성 — 묶음 레일 + 행 목록 + 정보 패널([설계](plan/2026-10-14-settings-screen.md), [ADR](acknowledge/2026-10-14-adr-settings-enter-next-value.md))
- [x] S4. 웹 현황 점검 — 고장·누락 목록([web/2026-10-14-audit.md](web/2026-10-14-audit.md), 웨이브 17A)
- [x] S5. 디자인 기준 — `iidx-rank` 의 디자인 언어를 화면 설계로 정리([web/2026-10-14-design-direction.md](web/2026-10-14-design-direction.md))
- [x] S6. 발광 난이도표 스냅샷 1,035곡 — `web/data/tables/insane1/`, `web/scripts/snapshot-table.ts`([web/2026-10-14-insane-table.md](web/2026-10-14-insane-table.md))
- [x] S7. 사용자 폴더와 기본 표의 서버·클라이언트 계약 설계([plan/2026-10-14-web-user-folders.md](plan/2026-10-14-web-user-folders.md), [폴더 ADR](acknowledge/2026-10-14-adr-user-folders.md), [기본 표 ADR](acknowledge/2026-10-14-adr-default-insane-table.md))
- [x] S8. 웹 구현 — 골격, 폴더 API, 표 제공(`d38b6fb`), 발광 표 페이지와 `/folders` 화면(`711b518`), 점검에서 나온 고장 수리([계약](web/2026-10-14-foundation-contract.md))
- [x] S9. 클라이언트 연동 — 기본 표 등록(스냅샷 내장), MY FOLDERS, 서버 주소 기본값, 제출의 차트 메타(웨이브 18, `8b11a17`)
- [x] S10. 배포 — Vercel Git 연결(Production `dev`, Root `web`), 운영 DB 0000~0002, 시험 계정 삭제, 첫 Git 배포 `d81eafc`([Vercel ADR](acknowledge/2026-10-14-adr-vercel-git-deploy.md), [운영 DB ADR](acknowledge/2026-10-14-adr-production-db-operations.md))
- [x] S11. SCRATCH AUTO 새 규칙(초록, 없는 노트), 사람이 친 모든 판의 저장·전송·리플레이, 파란 건반 + START 로 같은 배치 다시(웨이브 18, [규칙](acknowledge/2026-10-14-adr-scratch-auto-rule.md), [저장·전송](acknowledge/2026-10-14-adr-save-and-send-every-played-run.md), [다시 하기](acknowledge/2026-10-14-adr-same-layout-retry.md))
- [x] S12. SCRATCH AUTO 판의 순위 반영과 목록 토글(마이그레이션 0002, 웨이브 20 `cf40490`·`0b04629`). 같은 시기 웨이브 19 는 어시스트 곡이 낀 코스의 전송([순위 ADR](acknowledge/2026-10-14-adr-scratch-auto-ranking-toggle.md), [코스 ADR](acknowledge/2026-10-14-adr-assisted-course-submission.md))
- [x] S13. 클라이언트 `v0.1.1` 릴리스 draft(공증 통과했으나 맨 실행 파일이 Gatekeeper 에 막혀 S14 로 대체, draft 삭제)
- [x] S14. macOS 배포 형식을 `R-BMS.app` + dmg 로(`bfa0cca`, run 38559305840, `v0.1.2`)([ADR](acknowledge/2026-10-14-adr-macos-app-bundle-dmg.md))
- [x] S15. 점수 제출 400 — 새 판 시드(약 1.79e18)가 서버 Zod 4 `int()` 상한 2^53 을 넘던 것을 클라이언트·서버 양쪽에서 수리(`7095ef7`, v0.1.2 draft 삭제)([bug](bug/2026-10-14-score-submit-400-large-seed.md), [ADR](acknowledge/2026-10-14-adr-layout-seed-range.md))
- [x] S16. ~~v0.1.3 게시~~ 취소 — draft 준비(run 38563811699, `fb29d24`)까지 했으나 S18 사용자 결정으로 게시하지 않고 v0.1.4 로 대체
- [x] S17. 인수인계(2026-10-14) — 문서 4묶음 코드 대조, ADR 15건과 색인, 테스트 부채, bug 기록, HANDOFF(`e2c5a38`)
- [x] S18. 점수 제출 400(空POOR) 수리와 기록 동기화 — 서버 판정 수 검사(`865643f`), 로컬 기록 서버 동기화·Esc 포기 판 저장·IR 오류 필드(`5f4a9a2`), 계약 교차 테스트와 `web.yml` MySQL(`e9aebae`), CI·스크립트 작은 수리(`258596c`), 발광 표 페이지의 출처 문구 제거(`4a0872d`). 하위 S18-1~S18-7 모두 완료([bug](bug/2026-10-14-score-submit-400-empty-poor.md), [판정 수 ADR](acknowledge/2026-10-14-adr-judge-count-consumed-notes.md), [동기화 ADR](acknowledge/2026-10-14-adr-record-sync.md), [Esc 포기 판 ADR](acknowledge/2026-10-14-adr-abandoned-run-saved.md))
- [x] S19. 잠재 버그 전수 수리와 고도화·최적화 — 감사 4회와 수리, 하위 S19-1~S19-7 모두 완료. 상세는 아래 두 줄과 아카이브.
  - 웹 `b197446`·`e7890fc`·`9e2cc0e`(마이그레이션 0003 run 38627474716, 0004 run 38673724720), 클라이언트 `f81d499`·`794361b`(화면 대기 전용 스레드 `rbms-present`, 레인 키 디바운스)·`4cc72d0`·`1676c57`(PAD DEBOUNCE)·`c81eda1`, 앱 아이콘 `cc500ef`([assets/icon/README.md](../assets/icon/README.md)), 웹 파비콘 `089c737`, 발광 표의 다른 사이트 흔적 제거 `ea34002`(`4fda17c` 는 다운로드 링크까지 지워 사용자 정정으로 `dd5518e` 에서 복구).
  - 판정 의미 9건과 입력 시각: [avgjudge](acknowledge/2026-10-15-adr-avgjudge-reference-definition.md), [입력 시각 판정](acknowledge/2026-10-15-adr-event-time-judging.md), [TOTAL 덮어쓰기](acknowledge/2026-10-15-adr-total-override-assist.md), [게이지 사망 BP](acknowledge/2026-10-15-adr-failed-run-bp-unreached.md), [코스 곡별 콤보](acknowledge/2026-10-15-adr-course-stage-combo.md), [풀콤](acknowledge/2026-10-15-adr-full-combo-condition.md), [롱노트 몸통](acknowledge/2026-10-15-adr-long-note-body-notes.md), [짝 없는 롱노트 머리](acknowledge/2026-10-15-adr-unpaired-long-note-heads.md), [Esc 판](acknowledge/2026-10-15-adr-escape-keeps-every-run.md), [키보드 입력 시각](acknowledge/2026-10-15-adr-keyboard-input-timing.md). 프레임 비용 계측은 [reference/frame-cost.md](reference/frame-cost.md), 남은 위험은 [테스트 부채](quality-assurance/2026-10-14-test-debt.md).
  - 마무리 게이트(`51f3a1e`): Rust 6,024 통과·0 실패, 떠는 테스트 반복 0 실패, 웹 653 + DB 통합, CI 3 OS·웹 통과. 알려진 위험은 wgpu Metal 의 메인 스레드 위반([gfx-rs/wgpu#10627](https://github.com/gfx-rs/wgpu/issues/10627), 구조 유지가 사용자 결정). 사고(2026-10-15 12:15~12:17, 개발 빌드가 실제 HOME 으로 실행되어 판 49개가 운영에 전송, 기록 유지가 사용자 결정)의 경위와 재발 방지는 아카이브의 S19 절.

## 아카이브

| 파일 | 담긴 것 |
| --- | --- |
| [history/2026-10-15-process-archive.md](history/2026-10-15-process-archive.md) | 2026-10-14 ~ 10-15 작업 구간 S1~S19의 세부 결과·웨이브 기록·사용자 결정 원문, 보존 이력(전체 로드맵 Phase 0~7, 2026-06-03~), 옛 0~9절(목표, 확정 결정, 모드 프로파일, 크레이트 맵, 완료 기능, 실행, 검증 메모, 알려진 한계, docs 맵, 작업 규칙) |
| [history/2026-10-14-process-archive.md](history/2026-10-14-process-archive.md) | 2026-09-09 ~ 10-14 완료 구간: 전면 고도화 Phase A~I, 스킨 시스템과 기본 스킨, Lua 스킨 호환(웨이브 1~8, L1~L7), 첫 실행 피드백과 SKIN 메뉴(M1~M7), 플레이 기능 추가(N·O·Q·R, 웨이브 9~15), 첫 릴리스 v0.1.0 |
| [history/](history/) | Phase 별 이력, 인수인계 스냅샷(`*-handoff-snapshot.md`) |

## 다음 세션

[HANDOFF.md](HANDOFF.md) 8절의 순서를 따른다(사용자의 v0.1.4 실기 확인 → 실제 창 QA → 테스트 부채 → 클라이언트 한계 → 웹 2차).
