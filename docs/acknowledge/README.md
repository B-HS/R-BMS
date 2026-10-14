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

- SCRATCH AUTO 판의 램프를 LIGHT ASSIST 로 둔다([SCRATCH AUTO 규칙](2026-10-14-adr-scratch-auto-rule.md)).
- 넓힌 판정 판은 저장·전송하되 최고 기록·순위에서 뺀다([모든 판 저장·전송](2026-10-14-adr-save-and-send-every-played-run.md), [순위 토글](2026-10-14-adr-scratch-auto-ranking-toggle.md)).
- `v0.1.0` draft(게시 안 됨)를 지울지는 사용자 미결정([릴리스 정책](2026-10-14-adr-release-policy.md)).
