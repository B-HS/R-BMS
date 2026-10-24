# R-BMS — PROCESS (작업 체크리스트와 상태)

최종 갱신 2026-10-24 · 대응 커밋 `0621e2f`

이 문서는 체크리스트와 현재 상태만 둔다. 끝난 구간의 세부 결과, 웨이브 기록, 사용자 결정 원문은 `docs/history/` 로 옮긴다(ai-process 2.1). 옛 0~9절과 Phase 0~7 로드맵도 같은 곳에 있다(아래 "아카이브").

> 새 세션은 [HANDOFF.md](HANDOFF.md)(맥락, 결정, 작업 규칙, 우선순위 TODO. 항상 덮어쓴다)를 먼저 읽고 이 문서로 체크리스트를 확인한다. 이전 스냅샷은 `history/*-handoff-snapshot.md`.
> 베이스 룰: 프로젝트 작업 규칙의 단일 출처는 HANDOFF 5절이다. 코딩 컨벤션은 사용자 전역 컨벤션을 따르되 Rust 에는 TS/JS 전용 규칙을 적용하지 않고, 웹(`web/`)은 `web/AGENTS.md` 를 따른다.
> 위치: 저장소 루트 `/Users/hyunseokbyun/development/R-BMS`, 원격 `B-HS/R-BMS`, 작업 브랜치 `dev`. `prod` 브랜치는 만든 적이 없고(`release.yml` 은 `prod` 푸시도 받게 돼 있지만 쓴 적이 없다) 릴리스는 `dev` 에서 `release.yml` 을 수동 실행한다([ci-release.md](ci-release.md) 2절).
> 명령: 빌드, 테스트, 게이트는 [development.md](development.md)의 "Build / run / test", 실행 정본은 루트 `README.md`. 툴체인은 `rust-toolchain.toml` 로 `1.95.0` 고정이고 CI 는 `cargo fmt --all --check` 와 `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` 를 통과해야 한다. 테스트 수는 시점 값이라 여기에 적지 않는다(마지막 게이트 기록은 2026-10-23 Discord 병합 뒤 Rust 6,644 통과·0 실패, 웹 1,808 통과·0 실패).
> Git: Conventional Commits, 설명은 한국어([acknowledge/2026-09-17-commit-language.md](acknowledge/2026-09-17-commit-language.md)), author 사용자 단독, AI 트레일러 금지, 선별 스테이징, force push 금지. 게이트를 통과하면 논리 단위로 커밋·푸시하고 `gh run list --branch dev` 로 CI 를 확인한다. `Cargo.lock` 은 추적한다(2026-10-11 부터, 결정 D12). `target/` 과 곡 차트는 커밋하지 않는다.

## 상태 (2026-10-24)

- 2026-10-24 "다국어·리플레이 대전·랭킹 보기·NOTES RADAR·선곡 UI"(아래 첫 체크리스트) 구현·문서 완료(`72726c7`~`0434628`, 웨이브 W1~W6). 클라이언트 v0.1.9(release run 40055763447, 버전 커밋 `49fe493`, 앱·dmg 공증 Accepted, 한글 노트)를 사용자가 받아 확인하고 게시했다(2026-10-24 03:59 UTC, Latest). 같은 날 "어시스트 단위 A-단위"(위 첫 체크리스트) 완료: 운영 마이그레이션 0013(run 40074383695), 소급 A-단위 1개(run 40074842429), 웹 배포. 사용자 "배포다했고 ... 잘 다되네". 남은 것: 실기 점검 QA 3.23(Y1~Y17)·3.24(Z1~Z7)과 3.20~3.22, 일본어 원어민 감수, `CRON_SECRET` 확인, 곡 미러(보류).

- 2026-10-24 끝: 문서 마무리 `80cac02` 의 Windows ci 에서 가끔 실패하던 GPU 동영상 시험을 근본 수리(장면 시계가 실제 시계였음, 시험만 변경, `ab6f7a4`, ci run 40094404863 3 OS 통과). 기록 [bug/2026-10-24-movie-test-scene-clock-flaky.md](bug/2026-10-24-movie-test-scene-clock-flaky.md).

## 상태 (2026-10-23)

- 2026-10-23 "Stella·코스·단위와 클라이언트 보완"(아래 첫 체크리스트)과 Discord 상태 표시 완료: 구현 `9e39262`~`3d21944`, 운영 DB 0012(run 39909099575), GENOSIDE 44코스 등록(run 39946701885), Stella·Satellite 표 저장(run 39946847395), 클라이언트 v0.1.8(run 39970364536, `e92866a`, 로그로 서명·공증 확인, 한글 노트)을 사용자가 게시했다(Latest). 남은 것: R(실기 결과), Vercel `CRON_SECRET` 등록 여부 미확인, 곡 미러 질문 답.
- 2026-10-22 "클라이언트 기능 고도화"(아래 체크리스트) 구현과 웹 배포 완료: 운영 DB 마이그레이션 0011 적용(web-db run 39693988962), 웹 `64c6120`·`47a7f9c`, 운영 낡은 등급 행 0(run 39733088817). 클라이언트 v0.1.7(run 39812316178, `d438294`, 산출물 점검 통과, [ci-release.md](ci-release.md) 2.3)은 사용자가 게시했다(Latest). v0.1.6 도 게시됨, v0.1.5 는 draft. (2026-10-22 기준 서술)

- 클라이언트 `v0.1.4` 는 2026-10-15 07:27 UTC 에 게시됐다(release run 38696475211, 버전 커밋 `87eacd0`). `v0.1.3`·`v0.1.0` draft 는 게시하지 않고 남겨 뒀다. 릴리스 이력은 [ci-release.md](ci-release.md) 2.3절.
- 웹 운영 배포는 `51f3a1e`(`/api/version` 으로 확인, 이후 커밋은 `web/` 를 바꾸지 않음). 운영 DB 마이그레이션은 0000~0013 적용(0012 는 2026-10-23 run 39909099575, 0013 은 2026-10-24 run 40074383695. `web/drizzle/`, 실행은 `web-db.yml`. 이 줄의 웹 배포 `51f3a1e` 는 2026-10-15 관찰값).
- 2026-10-19 "웹 기록 표시와 구동기 Esc·목표 보완" 완료, 클라이언트 `v0.1.5` 는 draft 로 그대로 둔다(2026-10-23 결정 7). 열린 확인 사항은 [HANDOFF.md](HANDOFF.md) 6절, 남은 일은 8절.

## 체크리스트 — 어시스트 단위 A-단위 (2026-10-24)

사용자 요청 원문(2026-10-24): "추가 기능으로, 코스모드에서 어시스트 클리어를하면 그것도 웹에서 볼수있게해줘 A-단위 이런식으로". 결정(사용자 "전부 추천대로 그리고 workflow도 사용"): 1 workflow 사용, 2 어시스트 범위 = 지금 단위 인정에서 빠지는 판 전부(SCRATCH AUTO, ASSIST·LIGHT ASSIST 램프, 어시스트 옵션. 게이지가 남은 채 끝냈고 미러·LN 제약은 지킨 판), 3 정식 단위와 따로 기록, 정식이 더 높으면 정식만·A-단위가 더 높으면 둘 다 보임, 4 마이페이지에서 A-단위도 표시 단위로 고를 수 있음, 5 스테이지 정보가 있는 판(v0.1.8 이후)은 소급.

- [x] A1. 조사·설계 — 단위 판정·저장·표시 경로, 마이그레이션 필요 여부, 단위 분해(`docs/plan/2026-10-24-assist-dan.md`)
- [x] A2. 구현 — 서버 판정·저장·API, 웹 표시(배지·마이페이지·순위), 시험
- [x] A3. 운영 반영 — 마이그레이션이 있으면 파일 먼저 커밋·적용(사용자 `!`) 뒤 코드 푸시, 소급(`dan-recompute`)
- [x] A4. 문서·ADR·QA, 게이트, 커밋·푸시·CI

  진행 기록: 설계 wf_dafcd56d-d23 `53804d5`. 웨이브 1(wf_baf39040-fb2) 마이그레이션 0013 `35f77ae`(drizzle 이 낸 두 문장 기본 키 교체가 외래 키 때문에 MySQL 8.4.11 에서 1553 으로 실패해 한 문장으로), 서버. 웨이브 2(wf_0c4bb768-7f6) 웹·문서. 게이트: 웹 1,981 통과·0 실패, DB 161 통과, 빌드, `drizzle-kit generate` 변경 없음. 운영: status run 40073142588(0013 대기) → 사용자 `!` migrate run 40074383695 → status run 40074421795 최신 → 코드 푸시 `bfb4078`·`34e21d1`·`0a73e87`(web CI 40074454401 통과) → dan-recompute-count run 40074592117(scanned 2, eligibleAssisted 1, assistedLegacy 0) → 사용자 `!` dan-recompute run 40074842429(awardedAssisted 1, 정식 0). 클라이언트는 바꾸지 않았다(서버가 `A-` 를 붙인 label 을 보냄).

## 체크리스트 — 다국어·리플레이 대전·랭킹 보기·NOTES RADAR·선곡 UI (2026-10-24)

사용자 요청 원문(2026-10-24): "1. 다국어 지원 (한글, 영어, 일본어) 2. 리플레이를 볼때 해당 리플레이의 배치를 활용하여 나도 플레이하는 모드(battle모드였나? 그거랑비슷한거야) 3. 랭킹 보는 모드 (2, 4, 6 셋중 하나 누르면 해당 곡의 랭킹들을 볼수있어짐) 4. 곡선택에서, NOTES RADAR 구현도 가능하게해줘 CHORD, NOTES, PEAK, SCRATCH, SOF-LAN 이렇게해서 6각형 표시야 5. UI/UX 를 조금 더 꾸며봐봐. 또한 곡선택부분에 보면 곡 + 왼쪽에 스코어도 들어가있는데 이것도 구현하도록하고, 기타 화면에 있는것처럼 조금 더 iidx와 비슷하게끔 구현 진행해봐 현재 r-bms 에있는 내용도 표시 가능하게하고. rival 기능은 당장은 필요없는데 해당 UI 부분에 다른 수치를 넣는다던가 하면되겠지". 참고 화면: IIDX 선곡(NOTES RADAR 6각형 NOTES·PEAK·SCRATCH·SOF-LAN·CHARGE·CHORD, MUSIC DATA·SCORE DATA·RIVAL SCORE DATA 패널, 막대 왼쪽 등급). "restore 끝나면 지시 사항을 끝까지 진행하고, 5시간 세션때문에 멈추면 reset되는 타이밍에 알아서 이어서" — workflow 사용(Workflow 도구만, Fable·Haiku 금지, Opus high·Sonnet xhigh 상한). 세션 안 자동 재개: 매시 17분 Cron 점검(세션 한정, 7일 만료).

- [x] N0. 복원 결과 정리 — 문서-코드 동작 불일치 0, 표현 불일치 10건 문서 수리, 이전 세션 인수인계 커밋 `9fed3c0`
- [x] N1. 조사 — 5개 기능의 현재 코드·원본(beatoraja)·IIDX 기준 조사, 결과 `docs/reference/2026-10-24-*.md` (wf_77c97e35-052, `d2a636c`)
- [x] N2. 설계와 결정 — 통합 계획 `docs/plan/2026-10-24-five-features-plan.md`(wf_25f77f5a-21d, `bca5306`, 웨이브 W1~W6). 결정 묶음 1~21 사용자 답(2026-10-24 원문 "18번까지 전부 추천안대로 19번은 그럼 보류 20,21도 추천대로"): 1~18 추천안 A(웹 다국어·next-intl 포함), 19 곡 미러 보류, 20 미확인 13~18 지금대로, 21 표 코스 막대 PRACTICE 키 = 코스 시작. 확인 부탁 1~3(CRON_SECRET, 웹 화면, 실기)은 사용자 "아마 다 문제없을거야"(실제 확인 결과는 아님), CRON_SECRET 은 구현이 끝난 뒤 2026-10-24 오전 10시 43분에 다시 알림(세션 Cron 6e7a0ed0)
- [x] N3. 다국어(한·영·일)
- [x] N4. 리플레이 배치로 플레이하는 모드
- [x] N5. 랭킹 보기(2·4·6 키)
- [x] N6. NOTES RADAR(6축)
- [x] N7. 선곡 UI 꾸미기(막대 왼쪽 등급, MUSIC DATA·SCORE DATA·다른 수치 패널)
- [x] N9. 표 코스 막대 위 PRACTICE 키로 코스 시작(결정 21, 원본과 같게) (`5014bb6`)
- [x] N8. 통합 리뷰, 게이트, 문서·ADR·QA, 커밋·푸시·CI

  진행 기록: W1(wf_240136df-843) K0 `72726c7`, I0 `b7efac1`(LANGUAGE 행 포함, SETTING_COUNT 98, DISPLAY 탭도 스크롤), R1 `0e74d08`, RK-D `d727923`, RK-W `d74794b`. L1 은 호출부가 없어 clippy dead_code 8건이라 커밋하지 않고 W2 RP-P 가 먼저 적용(패치 보관). W1 게이트: Rust 6,765 통과·0 실패·6 무시, clippy·fmt 통과, 스킨 덤프 통과, 웹 1,818 통과·DB 153 통과(임시 MySQL 8.4.11)·빌드 통과. W2(wf_b6e33e9b-58c): 레이더 저장·칠하기(R2·R4) `54ff02c`, 랭킹 모드·오버레이·스킨 순위 슬롯(RK-M·RK-V·RK-S) `423484b`(SETTING_COUNT 99 RANKING KEYS), 배치 플레이 단축키 J·고스트·L1(RP-P) `30264b8`. 병합 충돌 2곳(`bridge.rs`, `text/mod.rs`)은 메인이 손으로. W2 게이트: Rust 6,866 통과·0 실패·6 무시, clippy·fmt·스킨 덤프·설정 계약 234 통과. W3 로 넘김: 외부 스킨 순위 목록 스크롤(rate 8 쓰기, RK-S 미완) → S3, 기록 모달의 로컬 기록 고스트 진입·내장 선곡 랭킹 머리 곡 제목 → L2. W1 푸시 CI 통과(ci 40001562553, web 40001562460). W3(wf_94f5727d-90f): 색·배치 토큰(D0) `038481b`, 기록 모달·랭킹 줄 배치 치기(L2) `eba8a73`, 커서 차트 분석·스킨 언어(A3·I1) `d75b819`, 막대 등급·썸네일·선곡 보조 값·순위 스크롤(BAR·S3) `b5f15df`. 손으로 합친 곳: `skin_host/tests.rs`(NOT_YET_FILLED 가 비어 상수 삭제), `stage/select/mod.rs` 3줄. W3 게이트: Rust 6,968 통과·0 실패·6 무시, clippy·fmt·스킨 덤프·시트 검사 통과. W2 푸시 CI 통과(ci 40006501338, web 40006501405). N9(표 코스 막대 PRACTICE 키)는 W5 I4 에 넣음. W4(wf_459eabfb-920): 선곡 목록 IIDX 식(D1) `c0bebfa`, 정보창·NOTES RADAR·기록(D2) `90d57d1`, 설정·렌더·메뉴·알림 번역(I2·I3·I5·I6) `7c9bb0d`. 손으로 합친 곳 `text/mod.rs`, `tablesrc.rs`. W4 게이트: Rust 7,062 통과·0 실패·6 무시, clippy·fmt·스킨 덤프·시트·선곡 캡처 통과(캡처를 메인이 직접 봄). W5 로 넘김: 확장 불린을 `op` 에 쓰면 스킨 옵션으로 오인되는 로더(`loader/track.rs` `is_builtin_option`), 경로 이중 표시(`info.lua` 머리와 목록 위), 남은 영어(설정 미리보기·스킨·플레이 메뉴 값, ir_session·ir_sync·record_sync IR 줄, 키 편집기 줄 이름, 스테이지 알림의 notify_text 이전), 용어집 새 용어 → DOC. W5(wf_61981d30-e03): 선곡 번역·코스 막대 PRACTICE 키(I4, N9) `5014bb6`, 기본 스킨 세 언어·확장 불린 op 로더 수리·경로 이중 표시(I7) `eadde84`, 남은 플레이어 문구(I8, 계획 밖 정리 단위) `f82e6a9`, 웹 다국어 기반 next-intl 4.14.9(WI0) `eb5ddbb`. 충돌 0. W5 게이트: Rust 7,129 통과·0 실패·6 무시, clippy·fmt·스킨 덤프(세 언어)·시트 통과, 웹 1,834 통과·빌드 통과. 메인 결정: 루트 레이아웃이 언어 쿠키를 읽어 웹 전 페이지가 요청마다 렌더(정적 셸 없음, `instant = false`, 데이터 캐시는 유지)되는 것을 받아들임(M11 쿠키 방식의 대가, 대안 `app/[locale]` 이동은 ADR 에 기록). W6(wf_2498787d-a5b 는 세 시간 한도로 5단위가 모두 시작 직후 끊겼고 남은 worktree 가 없어 wf_4b3aa1c6-c35 로 다시 실행): AI·설정 화면(WI1) `8d01908`, 셸·탐색 화면과 사이드바 언어 선택(WI2) `c9404f7`, AI 결과 언어·오류 코드 번역 도우미(WI4) `0405496`, 페이지·메타데이터·공용 기능(WI3)과 메인 수리 `b0cf509`, 문서(DOC) `0434628`. 메인 수리: 내비게이션 라벨을 `NAV_ITEMS.labelKey` + `common.nav` 하나로(WI2·WI3 겹침), 셸 바닥 단추 수를 공용 상수 3 으로, 리치 텍스트 태그를 못 읽는 중복 ICU 시험 삭제(WI2 의 `i18n-icu-format.test.ts` 가 같은 검사를 태그까지 함), next-intl 요청 설정과 시험 공급자에 `timeZone`(표시 시간대 Asia/Seoul)을 넣어 `ENVIRONMENT_FALLBACK` 제거. W6 게이트: 웹 1,930 통과·0 실패, DB 153 통과(임시 MySQL 8.4.11), 포맷·빌드 통과. Rust 는 W6 에서 바뀌지 않아 W5 게이트(7,129 통과)를 그대로 씀. 마지막 푸시 CI: `ea205f6` 의 ci 3 OS 통과(40045739584), web 은 시험 1개 실패(40045739556: `mutation-error-toast.test.ts` 가 sonner 전역 저장소에 다른 시험 파일이 남긴 토스트를 읽음, CI 파일 순서에서만 드러남). 시작 때도 비우게 고쳐 로컬에서 그 순서로 재현·해소 확인 뒤 `69d86a7`, web CI 통과(40046745698).

## 체크리스트 — Stella·코스·단위와 클라이언트 보완 (2026-10-23)

사용자 요구 원문 8개와 조사 보고는 [reference/2026-10-23-stella-course-dan-survey.md](reference/2026-10-23-stella-course-dan-survey.md). 사용자 결정(2026-10-23 원문): "1-11 추천대로 / 12. st, sl 둘 다 구현 / 13 - 16 추천대로 / 17. 아마 전곡 다 업로드가능할건데 일단 업로드할 준비만해두고 조사한 기록을 나한테 맨마지막에 알려줘 / 18. DP도 포함 / 19. 맞어 / 20. 추천대로 / 21. 좋아 너가말하나 3개 다 iidx까지 구현 하고 나머지도 추천대로 (0%)". workflow 사용(Workflow 도구만, Opus high·Sonnet xhigh 상한, Fable·Haiku 금지). v0.1.7 실기(QA 3.20) 결과는 아직 받지 않았다(요구 1~3 을 실기 보고로 다룬다).

결정(질문 번호 그대로): 1 패널 키 배치·확장 id 지금대로. 2 9키 행 이름 공유 지금대로. 3 등급 이름·색·ASSIST EASY 게이지 = 등급 0·웹 순위 키 지금대로. 4 Inter Bold 유지. 5 panels-1: 스킨 헤더의 확장 지원 표시가 있을 때만 재배정 키. 6 코스 제출에 게이지 종류를 더해 `course_best.assist_grade` 를 채움. 7 v0.1.5 draft 그대로 둠. 8 SCRATCH AUTO 노트 색 = PLAY 전역 RGBA 한 값(외부 스킨 포함, 알파 = 불투명도, 저장값 없으면 `#30E050`). 9 노트를 1개 이상 친 판의 Esc 는 로그인과 상관없이 결과 화면, 코스 도중 Esc 는 FAILED 코스 결과까지, 결과 판정 수는 저장값과 같게. 10 폴더 스택(원본 BarManager)으로 커서 복원. 11 임의 깊이 폴더 = 표 묶음 > 표 > 레벨, 표 코스 폴더. 12 Stella st 와 Satellite sl 둘 다. 13 Vercel Cron(UTC 15:00)이 원본을 받아 운영 DB 에 저장, 클라이언트도 원본 직접·하루 지나면 다시 받음. 14 Stella·단위는 출처 표기, 발광 insane1 은 흔적 제거 유지. 15 직접 zip 링크만 앱이 받아 풀고(새 의존성 `zip` 승인) rar·7z·HTML 은 브라우저로, 모달에 "차분만, 음원은 곡 URL" 안내. 16 표 레벨 안 흐린 없는 곡 막대 + 설정의 전용 다운로드 폴더(곡 폴더 목록에 자동 포함). 17 곡 미러는 업로드 준비(도구·설계)만 하고 조사 기록을 마지막에 보고. 18 GENOSIDE 2018 SP 22 + DP 22 + Stella·Satellite 표 코스. 19 서버가 코스 기록으로 단위를 자동 판정해 `user_dan` 에 넣고 마이페이지에서 표시할 단위를 고름. 20 단위 인정 = 게이지가 남은 채 끝낸 판, 미러·LN 제약 준수, 어시스트·SCRATCH AUTO 제외. 21 코스 게이지 설정 LR2(기본)/BEATORAJA/IIDX, 설정이 코스 제약 `gauge_lr2` 보다 우선, IIDX 실패선 0%.

운영 반영 승인(2026-10-23 사용자 원문 "3번은 승인 그냥 바로 진행해놔"): 이 체크리스트의 운영 반영(마이그레이션 0012 운영 적용, Vercel 환경 변수 등록)은 다시 묻지 않고 진행한다. 자동 모드가 거부하면 우회하지 않고 명령을 보여 `!` 실행을 요청한다. 시크릿 값은 대화·로그에 남기지 않는다. 클라이언트 release.yml 실행은 이 승인에 들지 않는다.

- [x] F0. 결정 기록 — 2026-10-22 ADR 5건 상태를 사용자 확인으로, 미확인 목록 정리, 요구 원문·조사 보고 저장 (`e2da295`, `1169efb`)
- [x] F1. 설계 — 묶음별 상세 설계(`docs/plan/2026-10-23-*.md`), 파일 소유권, 마이그레이션 계획 (workflow wf_98aa6bb9-ae7, `3c0cd22`: 설계서 5개 `docs/plan/2026-10-23-{client-small,select-tree-download,course-gauge-data,web-course-dan-tables,song-mirror-prep}.md`, 교차 검토와 메인 결정 `design-review.md` 10절, 실행은 그 3.2절 웨이브 1~7)
- [x] F2. 클라이언트 작은 묶음 — 8 SCRATCH AUTO 색, 9 Esc 결과 화면, 5 패널 키 스킨 opt-in (웨이브 1 `9e39262`, 통합 게이트 Rust 6,382 통과·0 실패)
- [x] F3. 선곡 폴더 스택과 표 — 10·11 폴더 스택·커서 복원, 12 Stella·Satellite 기본 표, 13 클라이언트 자동 다시 받기, 표 코스 폴더
- [x] F4. 없는 곡과 다운로드 — 16 흐린 없는 곡 막대·다운로드 폴더, 15 모달·zip 받기·브라우저 열기
- [x] F5. 코스 게이지와 코스 원천 — 21 코스 게이지 LR2/BEATORAJA/IIDX, 6 제출 게이지 종류, 18 GENOSIDE 2018 SP·DP 44 + 표 코스 가져오기
- [x] F6. 웹 — 6 `course_best.assist_grade`, 18 코스 등록·출처(14), 19·20 단위 자동 판정·마이페이지, 13 표 매일 갱신(Vercel Cron + DB), 12 Stella·Satellite 표 페이지. 마이그레이션 파일 먼저 커밋 → 사용자 승인으로 운영 적용 → 코드 푸시
- [x] F7. 곡 미러 준비 — 17 R2 업로드 도구·설계와 조사 기록(실행하지 않음, 마지막 보고)
- [x] F8. 통합 리뷰와 수리 (wf_10322c71-7d7: 확인 5건 수리 `45d09fd`·`76e026a`, 반박된 작업장 스캔도 근본 수리)

  진행 기록: 웨이브 1(wf_53d3eb78-f46) F2 U-A·U-B·U-C, F5 C1, F3 D1, F6 W1 → `9e39262`(커밋 사고: 마이그레이션 전용 커밋에 웨이브 전체가 섞임, [feedback](feedback/2026-10-23-stage-check-before-commit.md)), `db0aa7c`. 운영 0012 적용(사용자 `!` 실행, run 39909099575, 7/7). 웨이브 2(wf_8fbc1c6e-18e) F5 C2 `9f8dfbc`, C5 `f174f2b`, F3 U1 `fc23232`, F6 W2 `db534d1`(게이트 Rust 6,465·웹 1,512 통과, 0 실패). F6 W5 는 W6 과 함께 B2 로 보류(패치 보관). 웨이브 3(wf_088ab9d0-3ca, 세션 한도로 5단위가 끊겨 wf_a67f8781-768 이 같은 worktree 에서 이어받음) F3 U2 `2787264`, DP 제목 88개 `ea5829d`, U6 `6998d95`, U3 `519d7d6`, F5 C3 `c4fcdfa`, F7 U1 `fbae422`, F6 W3 `c79f53f`(게이트 Rust 6,550·웹 1,590 통과, 0 실패). Windows 해제 수리 `a6dc73c`(ci 3 OS 통과). 사이에 사용자 요청으로 `架空ユリカゴ　 [SABOTHER]` 웹 삭제(`a6e7ffe`, 운영 run 39927201095, [ADR](acknowledge/2026-10-23-adr-chart-removed-at-owner-request.md)). 웨이브 4(wf_9f61ed7d-64e) C6 `a52d9a3`, U4 `0ff2077`, C4 `47e8f70`(코스 시작 때의 COURSE GAUGE 로 스테이지 판정, 메인 수정 포함), W4 `d57c6c2`. 웨이브 5(wf_ceae3e00-e38) U5 `0247561`, W6(보류한 W5 포함) `942ed71`. 웨이브 6(wf_6dc77b43-fac) D2 `8d0a62a`, W7 `f2d8ce5`, W8 `608eae7`, W9 `df52b3c`. 웨이브 7(wf_1b82225e-f89) D3 `5a0eacf`(게이트 Rust 6,611 통과·0 실패, 스킨 덤프 통과, 웹 마지막 1,808 통과). 운영: courses-seed-count(run 39938376095: SP 22·DP 22 inserted), tables-refresh-count(run 39938388900: st 2,350·sl 2,497 저장 가능) 끝, courses-seed·tables-refresh-force 는 그 뒤 사용자 `!` 로 실행했다(run 39946701885, 39946847395). CRON_SECRET 등록은 사용자. F8 통합 리뷰는 wf_10322c71-7d7.
- [x] G. ADR·문서, 전체 게이트, 커밋·푸시, CI, 운영 반영(사용자 승인), 실기 점검표(QA) 추가 (`92a0caa`, 운영: courses-seed run 39946701885, tables-refresh-force run 39946847395. CRON_SECRET 은 사용자)
- [ ] R. 실기 결과(QA 3.20 은 v0.1.7, 3.21·3.22 는 v0.1.8) 수리 — 결과를 받으면
- [x] D. Discord 상태 표시(2026-10-23 사용자 요청 "discord에 게임으로 인식되게하는 기능", 답 "좋아 1번대로 하도록하고 Application 만들어서 아이디는 1563156237539999865 / 아이콘이나 기타 꼼꼼하게"). 결정: workflow 사용, v0.1.8 draft 전에 넣음, Rich Presence(로컬 IPC, 새 의존성 없음)로 곡 제목·난이도·모드·코스·경과 시간, 설정 DISCORD STATUS 기본 켬(2~4 는 답이 없어 메인이 추천안으로 정함, 사용자 미확인). Application ID 1563156237539999865(비밀 아님, `apps/rbms-player/src/discord_presence.rs` 의 `DISCORD_APPLICATION_ID`). Developer Portal(앱 아이콘·설명·태그·Art Asset `rbms`)은 사용자 요청 "너가 직접 미리 다 해놔" 로 메인이 Claude in Chrome 으로 설정했다. 설계 `d7892ad`, 구현 `3d21944`, 문서 `41f1a1b`, [ADR](acknowledge/2026-10-23-adr-discord-rich-presence.md)

## 체크리스트 — 클라이언트 기능 고도화: 옵션 패널·램프·색상·글꼴 (2026-10-22)

사용자 지시(2026-10-22) 요지: (1) SCRATCH AUTO 등 플레이 옵션 토글을 A(START)·A+W 패널에서 바로 바꾸게, (2) A·W·A+W 패널을 IIDX 옵션 화면(키 그림 + 연결선 + 값 전체를 세로로 나열, 고른 값 주황 강조) 처럼, (3) 클리어 램프 세분화(ASSIST HARD·ASSIST EXHARD 등), (4) 레인·노트 색상과 불투명도를 각각 조절, 스킨 세부 조절 확대, (5) 글꼴 가독성(안티에일리어싱, 저화질 느낌 해소). 참고 이미지는 IIDX 옵션 화면 3장. 사용자 결정: workflow 사용(이 프로젝트 규칙: Workflow 도구만, Opus high·Sonnet xhigh 상한, Fable·Haiku 금지).

- [x] E0. 조사 — [option-panels](reference/option-panels.md), [clear-lamps](reference/clear-lamps.md), [skin-color-customization](reference/skin-color-customization.md), [font-rendering](reference/font-rendering.md)(workflow wf_90e33476-4e9, 종합 검토가 코드 대조로 4건 정정)
- [x] E1. 설계 확정 — 사용자 지시 "끝까지 다 구현해놔"(2026-10-22)에 따라 질문 없이 종합 검토 추천안 L1~L4·P1~P5·C1·C2·F1 을 모두 A(P2 는 A')로 정함. 2026-10-09 사설 id 철거 결정의 일부 변경(불린·이벤트 확장 대역 21000~)은 ADR 로 남기고 보고
- [x] E2-1. 1단계(병렬, workflow wf_7444d7a7-d9f): W1a 글꼴 엔진 `f03beb3`, W1c 색 데이터 층 `49dfbd3`, W1b 램프 등급 클라이언트 `4d57c2a`(통합 게이트 Rust 6,206 통과·0 실패). W2c 웹 램프 등급: 사용자 승인(2026-10-22 "마이그레이션도 니가하고") 뒤 마이그레이션 `8d85b32` 운영 적용(web-db run 39693988962, 0011 recorded) → 웹 코드 `64c6120`(웹 1,485 통과·0 실패)
- [x] E2-2. 2단계(병렬, workflow wf_83280bf0-bdb): W2a 패널 Rust·확장 id(이벤트 21000~, 불린 21100~)·F1 홀드 키·IR 등급 `7746932`, W2b 노트 색 곱셈·색 행 메뉴 `7459bf7`(통합 게이트 Rust 6,238 통과·0 실패). 푸시는 3단계(기본 스킨 라벨)와 함께
- [x] E2-3. 3단계(병렬, workflow wf_59a0a66f-73f): W3a 기본 스킨 IIDX 식 패널·램프 등급 표시·굵은 글꼴 `0c450db`, W3b 플레이 색 행·순백 노트 시트 `5192bbb`(저장값 없을 때 캡처 94장 화소 동일, 통합 게이트 Rust 6,244 통과·0 실패, 스킨 덤프 경고 0). 2~3단계 푸시
- [x] E2-4. 통합 리뷰(발견→검증, workflow wf_efddd39c-0ee: 확인 7·거절 2·검증 미완 7)와 수리(workflow wf_ed445327-c90). 수리 13건: lamps-1~3 `242397f`(코스 스테이지를 실제로 끝난 CLASS 계열 게이지로 저장·전송, 재전송 게이지 수리 포함, refold 스캔 좁힘, 내장 막대 LED 등급 색), panels-2~4 `3f058f4`, colors-1·2와 LN 전용 색 행 `c1c992e`, font-1~3 `f52e170`, web-1·3와 등급 백필 액션 `47a7f9c`. `panels-1`(외부 스킨 위 표시 없는 설정 변경)은 고치지 않고 남김(HANDOFF 6절 13). 운영 낡은 등급 행 0(web-db run 39733088817).
  - 수리 단계는 2026-10-22 사용자 지시("된곳까지만 일단 스톱 나중에 다시재개")로 한 번 정지했고, 재개 후 완료했다.
- [x] E2-5. ADR 5건(`acknowledge/2026-10-22-adr-{assist-lamp-grade,option-panel-layout,rbms-extension-ids,skin-colour-rows,ui-text-device-pixels}.md`: 램프 등급, 옵션 패널 배치, 확장 id 대역 21000~ 로 2026-10-09 결정 부분 변경, 색 행, UI 글자 장치 화소·굵은 글꼴 에셋)와 문서 갱신(reference 4종, `ir-api`, `ai-features/server`, `backend/{data-model,api-spec}`, `web/architecture`, HANDOFF, PROCESS 와 play-options·skin·theme·crates·architecture·QA 수동 점검표). 문서 커밋은 메인이 한다.
- [x] G. ADR·문서 갱신, 전체 게이트(마지막 Rust 6,269 통과·0 실패, 웹 1,491·DB 122 통과), 단계별 커밋·푸시(`f03beb3` ~ `47a7f9c` 와 문서 커밋), 운영 DB 마이그레이션 0011(run 39693988962), CI(ci run 39733082740·web run 39733082784 통과). 설정 설명 문구의 옛 램프 이름 `ASSIST EASY at best` 도 문서 커밋에서 고침. 남은 것은 사용자 실기와 확인(HANDOFF 6절 13·14, 8절)

## 체크리스트 — 웹 AI 기능 기획 (2026-10-19)

사용자 지시(2026-10-19): 웹에 AI 기능을 만든다. 먼저 기획. 기능 10개 모두 구현 방향 — 플레이어 리포트(약점·선호 패턴 등 리플레이로 얻을 수 있는 모든 정보 종합), 리플레이 코치, 베스트·라이벌 비교, 판정 습관 진단, 다음 도전 곡 추천, AI 연습 폴더(MY FOLDERS), 목표 계획, 게이지·옵션 조언, 자연어 기록 질문(필터 스키마), 주간 성장 요약. AI 키는 Anthropic(API key), OpenAI(Codex, API key), Ollama Cloud(API key) 셋만.
사용자 결정: 키는 사용자 BYOK + 암호화. 패턴 데이터는 판별(클라이언트가 실제 배치·노트별 판정을 올림) + 곡별(발광 전곡을 rbms-cli 로 뽑아 md5 로 표와 1:1). 서버의 차트 패턴은 노트 배치 전체를 비공개 저장("최대한 정확한 리포트"), 공개 API 로 내보내지 않음. 집계·패턴 분류는 일반 코드, AI 는 해석·설명·추천만. 공급자는 같은 구조화 출력 스키마의 어댑터 계층. workflow 사용. 곡별 패턴 추출의 원본 폴더는 `/Volumes/SSD/bms`(사용자 2026-10-19 "여기가 맞을거야", 추출 때 SSD 연결 필요, 원본 파일은 업로드하지 않음).

- [x] AI1. 조사 — 웹 서버 데이터·보안 경계, 클라이언트가 판 끝에 아는 것·리플레이·rbms-cli, 공급자 3곳 공식 API(docs/reference/ai-features/ 에 저장, workflow wf_2bde79bf-eeb)
- [x] AI2. 기획 문서 — docs/plan/2026-10-19-web-ai-features.md(기능 10개, 데이터 계약, 분석 엔진, 공급자 어댑터, 키 암호화, 단계 계획, 사용자 확인 질문)
- [x] AI3. 메인 검토와 사용자 확인 질문 정리, 결정 ADR([W0 결정](acknowledge/2026-10-19-adr-web-ai-decisions.md))
- [x] AI-W1. 기반 + 기능 9(마이그레이션 0005, secret-box, 어댑터 3종, 키 API·설정 탭, 사용량·운영자 한도, 관리자 seed) — 마이그레이션 `c42b00d`(운영 적용 run 39275472038, 관리자 1행 확인 39275562175), 코드는 다음 커밋. 게이트 웹 1,070 통과·DB 81 통과·무작위 순서 5회 0 실패. 사용자 키 실기(Q18)는 마지막에
- [x] AI-W2. 점수 기반 기능 5·6·7·10·4 일부(0006) — 마이그레이션 0008(`54f86a8`), 코드는 W3·W4 웹과 한 커밋
- [x] AI-W3. 곡별 패턴(rbms-cli chart-pattern, 0007, 7K 전체 추출 — SSD·관리자 토큰은 사용자) — CLI `d3ecf75`, 추출 7,903곡·표 1,035/1,035 매칭·333 MB(`/Users/hyunseokbyun/rbms-chart-patterns`, 저장소 밖). 웹 업로드 경로·분석 엔진 완료(마이그레이션 0007 `54f86a8`, 코드 `57fdc08`), 운영 업로드 2026-10-20 7,903곡 실패 0, coverage 発狂 1,035/1,035·stale 0, 토큰은 키체인 `rbms-admin`/`rbms-pattern-upload`
- [x] AI-W4. 판별 trace(0009 `4d30bb4`, 웹 `57fdc08`·클라이언트 `98a2cf0`), 클라이언트 릴리스는 R2
- [x] AI-W5. 판별 기반 기능 1·2·3·8·4 전체 — 0010 운영 적용(run 39316772885), 코드는 2026-10-20 커밋. 게이트 웹 1,451·DB 108·Rust 6,146 통과
- [x] AI-W6. 과거 판 소급(EVENT 시계 리플레이 재판정)
- [ ] AI-R2. 클라이언트 v0.1.6 태그·draft(trace 전송·소급 포함), 사용자 실기(키 진단·리포트 확인) — draft 완료(run 39318721794, `2bb868b`, 산출물 점검 통과, [ci-release.md](ci-release.md) 2.3), 사용자 게시됨(2026-10-23 확인). 남은 것: 키 진단·실기

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
| [history/](history/) | Phase 별 이력, 인수인계 스냅샷(`*-handoff-snapshot.md`, 가장 최근 `2026-10-24-handoff-snapshot.md` 는 다섯 기능 작업 전의 판) |

## 다음 세션

[HANDOFF.md](HANDOFF.md) 8절의 순서를 따른다(사용자 실기 결과 QA 3.23·3.24 → 수리 → 테스트 부채 → `CRON_SECRET` → 곡 미러 보류 해제 여부).
