# docs/acknowledge — 결정 기록 색인

> 최종 갱신 2026-10-14 · 대응 커밋 7095ef7

사용자 결정, 합의, 전제를 둔다. 2026-10-14 부터 새 결정은 결정 1건 = 파일 1개의 ADR(`YYYY-MM-DD-adr-<슬러그>.md`, 절: 배경 / 결정 / 이유 / 기각된 대안 / 영향과 관련 파일 / 날짜와 사용자 원문)로 쓴다. 그 전의 파일은 여러 결정을 한 파일의 표로 묶는다. 결정이 바뀌면 옛 파일은 고치지 않고, 바뀐 행 끝에 "대체됨: <새 파일>" 을 더하고 이 색인의 상태를 바꾼다.

상태: 유효 = 지금 코드와 운영의 근거. 일부 대체됨 = 파일 안의 표시된 행만 대체됨. 대체됨 = 이력으로만 둔다.

## 2026-10-12 ~ 10-14 세션의 ADR

| 파일 | 한 줄 요약 | 결정일 | 상태 |
| --- | --- | --- | --- |
| [2026-10-14-adr-scratch-auto-rule.md](2026-10-14-adr-scratch-auto-rule.md) | SCRATCH AUTO: 턴테이블 노트는 초록·판정에서 빠진 없는 노트, 그 판도 최고 기록·이력·리플레이·전송, 램프 LIGHT ASSIST, 규칙 세대 3, 옛 세대의 SCRATCH AUTO 판(사용자 DB 37판)은 램프·이력만 | 2026-10-14 | 유효 |
| [2026-10-14-adr-save-and-send-every-played-run.md](2026-10-14-adr-save-and-send-every-played-run.md) | 사람이 친 판은 어시스트와 상관없이 전부 저장·리플레이·전송. 넓힌 판정 판은 최고 기록·순위만 제외. KNOWN ISSUE 1건 | 2026-10-14 | 유효 |
| [2026-10-14-adr-assisted-course-submission.md](2026-10-14-adr-assisted-course-submission.md) | 어시스트 곡이 낀 코스도 코스로 보낸다(`scratch_auto`·`assist` 선택 필드), 서버는 단일 곡과 같은 정책 | 2026-10-14 | 유효 |
| [2026-10-14-adr-scratch-auto-ranking-toggle.md](2026-10-14-adr-scratch-auto-ranking-toggle.md) | SCRATCH AUTO 판도 순위에 올림. 최고 기록을 SCRATCH AUTO 여부로 나눔(마이그레이션 0002), 목록은 기본 숨김 + `include_scratch_auto` 토글, `A-SCR` 표지 | 2026-10-14 | 유효 |
| [2026-10-14-adr-score-target-fallback.md](2026-10-14-adr-score-target-fallback.md) | 목표 점수가 없을 때의 TARGET FALLBACK(기본 RATE AAA)과 RATE CUSTOM, 화면이 실제 목표를 말함 | 2026-10-14 | 유효 |
| [2026-10-14-adr-settings-enter-next-value.md](2026-10-14-adr-settings-enter-next-value.md) | 설정 화면의 스위치·선택 행 Enter = 다음 값, 닫기는 Esc 만 | 2026-10-14 | 유효 |
| [2026-10-14-adr-same-layout-retry.md](2026-10-14-adr-same-layout-retry.md) | 결과 화면에서 파란 건반 + START = 같은 시드·옵션으로 다시 | 2026-10-14 | 유효 |
| [2026-10-14-adr-default-insane-table.md](2026-10-14-adr-default-insane-table.md) | 발광 난이도표: 스냅샷 커밋, 웹 사본 주소 제공, 실행 파일 내장, 한 번만 등록(`seeded_default_tables`) | 2026-10-14 | 유효 |
| [2026-10-14-adr-user-folders.md](2026-10-14-adr-user-folders.md) | 사용자 폴더: 웹 편집, 클라이언트 전용 읽기 API, 선곡의 MY FOLDERS, v1 비공개 | 2026-10-14 | 유효 |
| [2026-10-14-adr-production-db-operations.md](2026-10-14-adr-production-db-operations.md) | 운영 DB 작업은 `web-db.yml` + 저장소 시크릿 `WEB_DATABASE_URL`, 실행은 메인(파괴적 작업은 승인 뒤). `vercel env` 경로 기각 | 2026-10-14 | 유효 |
| [2026-10-14-adr-vercel-git-deploy.md](2026-10-14-adr-vercel-git-deploy.md) | Vercel Git 연결, 운영 브랜치 `dev`, 루트 `web`, Preview 없음, `prod` 기각 | 2026-10-14 | 유효 |
| [2026-10-14-adr-release-policy.md](2026-10-14-adr-release-policy.md) | 릴리스는 태그 + draft 까지 자동, 게시는 사용자. 자격 증명은 빌드 전에, 로컬 검증 스크립트, `signing-check` | 2026-10-12 ~ 10-14 | 유효 |
| [2026-10-14-adr-macos-app-bundle-dmg.md](2026-10-14-adr-macos-app-bundle-dmg.md) | macOS 는 공증 티켓을 붙인 `R-BMS.app` + dmg. 맨 실행 파일은 티켓을 붙일 수 없어 기각 | 2026-10-14 | 유효 |
| [2026-10-14-adr-layout-seed-range.md](2026-10-14-adr-layout-seed-range.md) | 새 배치 시드는 2^53 미만, 서버는 범위 밖 시드를 0 으로 받아 판을 거절하지 않음. KNOWN ISSUE 1건 | 2026-10-14 | 유효 |
| [2026-10-14-adr-release-toolchain-pin.md](2026-10-14-adr-release-toolchain-pin.md) | 릴리스 빌드 툴체인을 `rust-toolchain.toml` 과 같은 1.95.0(`dtolnay/rust-toolchain@master`)으로 | 2026-10-12 | 유효 |
| [2026-10-14-adr-table-page-no-source-notice.md](2026-10-14-adr-table-page-no-source-notice.md) | 발광 표 페이지의 출처·삭제 요청 문구(요약, 맨 아래 문단)를 뺀다. 정보 패널의 출처 항목과 `source.json` 은 유지 | 2026-10-14 | 유효 |
| [2026-10-14-adr-judge-count-consumed-notes.md](2026-10-14-adr-judge-count-consumed-notes.md) | 점수 제출의 판정 수 검사는 노트를 소비한 판정만(空POOR 제외, 9K 는 BAD 제외). 空POOR 400 사고의 수정 | 2026-10-14 | 유효 |
| [2026-10-14-adr-record-sync.md](2026-10-14-adr-record-sync.md) | 로컬 이력 전체를 서버로 동기화(자동+수동, 리플레이 포함), 판별 서버 상태 기록, 거절 판은 다음에 다시, 옛 SCRATCH AUTO 제외, 서버는 같은 초 멱등 | 2026-10-14 | 유효 |
| [2026-10-14-adr-abandoned-run-saved.md](2026-10-14-adr-abandoned-run-saved.md) | Esc 중도 포기 판도 FAILED 로 저장·전송(남은 노트는 놓침), 판정 없는 판·코스는 제외 | 2026-10-14 | 일부 대체됨: 결정 2·3 → [Esc 판은 모두 저장](2026-10-15-adr-escape-keeps-every-run.md) |
| [2026-10-15-adr-long-note-body-notes.md](2026-10-15-adr-long-note-body-notes.md) | 같은 레인 롱노트 몸통 안의 일반 노트는 원본처럼 레인에서 빼 배경음으로(지뢰는 버림), 머리 칸·삼킴 규칙도 원본대로. 곡 DB `PARSER_VERSION` 3 | 2026-10-15 | 유효 |
| [2026-10-15-adr-unpaired-long-note-heads.md](2026-10-15-adr-unpaired-long-note-heads.md) | 끝나지 않은 롱노트 머리는 디코더가 지우고 노트 수 정의를 `rbms-model` 한 곳에. 판정 엔진 `total_notes` 와 같은 값 | 2026-10-15 | 유효 |
| [2026-10-15-adr-avgjudge-reference-definition.md](2026-10-15-adr-avgjudge-reference-definition.md) | `avgjudge` 는 원본 정의(평균 절대 오차, 놓친 노트 1초), 치우침은 `extra.mean_offset_us`, 본문에 `extra.rule_version`. 규칙 세대 4, 옛 행 값은 최고 기록에서 빼고 재전송은 0 | 2026-10-15 | 유효 |
| [2026-10-15-adr-total-override-assist.md](2026-10-15-adr-total-override-assist.md) | TOTAL 덮어쓰기(>0)는 넓힌 판정과 같은 어시스트(저장·전송, 최고 기록·순위 제외), 리플레이에 TOTAL 기록·재생은 기록값, 업로드는 안 함 | 2026-10-15 | 유효 |
| [2026-10-15-adr-failed-run-bp-unreached.md](2026-10-15-adr-failed-run-bp-unreached.md) | 게이지 사망 판의 BP 에 도달하지 못한 노트를 넣는다(`min_bp_with_unreached`), 옛 행은 판정 수에서 계산해 비교 | 2026-10-15 | 유효 |
| [2026-10-15-adr-course-stage-combo.md](2026-10-15-adr-course-stage-combo.md) | 코스 곡별 기록의 max_combo·풀콤은 스테이지 콤보로, 화면·다음 스테이지는 코스 콤보. 코스 램프는 코스의 끊김 없는 연속으로 | 2026-10-15 | 유효 |
| [2026-10-15-adr-full-combo-condition.md](2026-10-15-adr-full-combo-condition.md) | 풀콤 = 끝에 남은 콤보가 지나간 노트 수(원본 `BMSPlayer.java:868`). 5K·PMS 의 마지막 노트 뒤 空POOR 는 풀콤을 깬다 | 2026-10-15 | 유효 |
| [2026-10-15-adr-escape-keeps-every-run.md](2026-10-15-adr-escape-keeps-every-run.md) | Esc 판은 판정 수와 상관없이 FAILED 저장·전송. 코스 도중 Esc 는 남은 노트가 없으면 정상 판정, 있으면 곡 기록 + FAILED 코스 기록까지 저장·전송 | 2026-10-15 | 유효 |
| [2026-10-15-adr-event-time-judging.md](2026-10-15-adr-event-time-judging.md) | 입력은 들어온 시각(키는 받은 순간, 컨트롤러는 gilrs 시각)으로, 스윕·지연 릴리스·지뢰·HCN 틱·GAS 는 그 시각 순서로. HCN 틱은 경과 시간, 게이지가 빈 순간 판정 멈춤, 리플레이 `clock` 세대(EVENT/FRAME) | 2026-10-15 | 유효 |

## 그 전의 결정 파일

| 파일 | 한 줄 요약 | 날짜 | 상태 |
| --- | --- | --- | --- |
| [2026-10-14-web-decisions.md](2026-10-14-web-decisions.md) | 웹 배포 준비: 사용자 결정 U1~U5, 메인 추천안 M1~M11, 질문 Q1~Q6 | 2026-10-14 | 일부 대체됨: Q5, Q6 의 실행 주체 → [운영 DB 작업](2026-10-14-adr-production-db-operations.md) |
| [2026-10-09-lua-skin-compat-decisions.md](2026-10-09-lua-skin-compat-decisions.md) | Lua 스킨 호환 D1~D18(기본 스킨 자작, F=즐겨찾기만, Cargo.lock 추적, IIDX 식 판정, 정적 판정 글자, "GREAT 67", 앱 UI 무채색 등), 동영상 디코더 | 2026-10-09 ~ 10-12 | 유효 |
| [2026-09-17-skin-system-decisions.md](2026-09-17-skin-system-decisions.md) | 스킨 시스템 완성 D1~D23 | 2026-09-17 | 대체됨(2026-10-09, Lua 스킨 호환 결정) |
| [2026-09-17-commit-language.md](2026-09-17-commit-language.md) | 커밋 설명 언어는 한국어 | 2026-09-17 | 유효 |
| [2026-09-09-enhancement-decisions.md](2026-09-09-enhancement-decisions.md) | 고도화 결정 1~12, 웹 W1~W8·OQ1~OQ6, Phase E E1~E13, 마무리 순서 | 2026-09-09 ~ 09-10 | 일부 대체됨: 결정 8(작업 규칙), 결정 12 → [SCRATCH AUTO 규칙](2026-10-14-adr-scratch-auto-rule.md)·[모든 판 저장·전송](2026-10-14-adr-save-and-send-every-played-run.md), 마무리 순서의 릴리스 → [릴리스 정책](2026-10-14-adr-release-policy.md) |
| [release-branch-strategy.md](release-branch-strategy.md) | prod 브랜치·보호·첫 버전의 옵션(결정 대기였음) | 2026-09-16 | 대체됨 → [릴리스 정책](2026-10-14-adr-release-policy.md) |
| [design.md](design.md) | 웹 디자인 정본(토큰, Surface, 화면 규칙) | 2026-09-09(최종 수정 2026-10-14) | 유효(웹 2026-10-14 디자인 기준과의 차이는 `docs/web/2026-10-14-design-direction.md` 3.1.1) |
| [empty-poor-local-scores.md](empty-poor-local-scores.md) | 空POOR 처리와 로컬 기록 스키마 | 2026-06-03(상태 갱신 2026-09-16) | 유효 |
| [reference-divergences.md](reference-divergences.md) | 레퍼런스 구현과의 의도적·보류 차이 | 2026-09-09 ~ 09-10 | 유효 |
| [decisions.md](decisions.md) | 포팅 착수 결정(그래픽 스택, 첫 범위, 모드 확장성) | 2026-05-31 | 대체됨(범위 제한은 Phase E~G 에서 해소) |

## 사용자에게 아직 확인받지 않은 메인의 결정

- 없음. Esc 포기 판의 두 예외(판정 없는 판, 코스 도중)는 2026-10-15 에 사용자가 거부했다([Esc 판은 모두 저장](2026-10-15-adr-escape-keeps-every-run.md)).

2026-10-14 에 확인받은 것: SCRATCH AUTO 램프 LIGHT ASSIST 유지, 넓힌 판정 판은 저장·전송하되 최고 기록·순위 제외(추천안 A). `v0.1.0` draft 는 둔다, `v0.1.3` draft 도 두고 다음은 `v0.1.4`.
