# docs/acknowledge — 결정 기록 색인

> 최종 갱신 2026-10-15 · 대응 커밋 ea698a9

사용자 결정, 합의, 전제를 둔다. 2026-10-14 부터 새 결정은 결정 1건 = 파일 1개의 ADR(`YYYY-MM-DD-adr-<슬러그>.md`, 머리: `최종 갱신 · 대응 커밋` 과 `상태 · 결정일 · 결정자 · 커밋`, 절: 배경 / 결정 / 이유 / 기각된 대안 / 영향과 관련 파일 / 날짜와 사용자 원문)로 쓴다. 그 전의 파일은 여러 결정을 한 파일의 표로 묶는다. 결정이 바뀌면 옛 파일의 결정 문장은 고치지 않고, 바뀐 행 끝에 "대체됨: <새 파일>" 을 더하고 그 파일의 상태 줄과 이 색인의 상태를 함께 바꾼다.

상태: 유효 = 지금 코드와 운영의 근거. 일부 대체됨 = 파일 안의 표시된 행만 대체됨. 대체됨 = 이력으로만 둔다.

## 2026-10-12 ~ 10-15 세션의 ADR

결정일 순서. 같은 날은 결정이 나온 순서에 가깝게 둔다.

| 파일 | 한 줄 요약 | 결정일 | 상태 |
| --- | --- | --- | --- |
| [2026-10-14-adr-scratch-auto-rule.md](2026-10-14-adr-scratch-auto-rule.md) | SCRATCH AUTO: 턴테이블 노트는 초록·판정에서 빠진 없는 노트, 그 판도 최고 기록·이력·리플레이·전송, 램프 LIGHT ASSIST(2026-10-14 사용자 확인), 규칙 세대 3, 옛 세대의 SCRATCH AUTO 판(사용자 DB 37판)은 램프·이력만 | 2026-10-14 | 유효 |
| [2026-10-14-adr-save-and-send-every-played-run.md](2026-10-14-adr-save-and-send-every-played-run.md) | 사람이 친 판은 어시스트와 상관없이 전부 저장·리플레이·전송. 넓힌 판정 판은 최고 기록·순위만 제외(2026-10-14 사용자 확인) | 2026-10-14 | 유효 |
| [2026-10-14-adr-assisted-course-submission.md](2026-10-14-adr-assisted-course-submission.md) | 어시스트 곡이 낀 코스도 코스로 보낸다(`scratch_auto`·`assist` 선택 필드), 서버는 단일 곡과 같은 정책 | 2026-10-14 | 유효 |
| [2026-10-14-adr-scratch-auto-ranking-toggle.md](2026-10-14-adr-scratch-auto-ranking-toggle.md) | SCRATCH AUTO 판도 순위에 올림. 최고 기록을 SCRATCH AUTO 여부로 나눔(마이그레이션 0002), 목록은 기본 숨김 + `include_scratch_auto` 토글, `A-SCR` 표지 | 2026-10-14 | 유효 |
| [2026-10-14-adr-score-target-fallback.md](2026-10-14-adr-score-target-fallback.md) | 목표 점수가 없을 때의 TARGET FALLBACK(기본 RATE AAA)과 RATE CUSTOM, 화면이 실제 목표를 말함 | 2026-10-14 | 유효 |
| [2026-10-14-adr-settings-enter-next-value.md](2026-10-14-adr-settings-enter-next-value.md) | 설정 화면의 스위치·선택 행 Enter = 다음 값, 닫기는 Esc 만 | 2026-10-14 | 유효 |
| [2026-10-14-adr-same-layout-retry.md](2026-10-14-adr-same-layout-retry.md) | 결과 화면에서 파란 건반 + START = 같은 시드·옵션으로 다시 | 2026-10-14 | 유효 |
| [2026-10-14-adr-default-insane-table.md](2026-10-14-adr-default-insane-table.md) | 발광 난이도표: 스냅샷 커밋, 웹 사본 주소 제공, 실행 파일 내장, 한 번만 등록(`seeded_default_tables`) | 2026-10-14 | 일부 대체됨: 결정 2 의 출처·삭제 요청 문구와 출처 표기 → [표 페이지](2026-10-14-adr-table-page-no-source-notice.md) |
| [2026-10-14-adr-user-folders.md](2026-10-14-adr-user-folders.md) | 사용자 폴더: 웹 편집, 클라이언트 전용 읽기 API, 선곡의 MY FOLDERS, v1 비공개 | 2026-10-14 | 유효 |
| [2026-10-14-adr-production-db-operations.md](2026-10-14-adr-production-db-operations.md) | 운영 DB 작업은 `web-db.yml` + 저장소 시크릿 `WEB_DATABASE_URL`, 실행은 메인(파괴적 작업은 승인 뒤). `vercel env` 경로 기각 | 2026-10-14 | 유효 |
| [2026-10-14-adr-vercel-git-deploy.md](2026-10-14-adr-vercel-git-deploy.md) | Vercel Git 연결, 운영 브랜치 `dev`, 루트 `web`, Preview 없음, `prod` 기각 | 2026-10-14 | 유효 |
| [2026-10-14-adr-release-policy.md](2026-10-14-adr-release-policy.md) | 릴리스는 태그 + draft 까지 자동, 게시는 사용자. 자격 증명은 빌드 전에, 로컬 검증 스크립트, `signing-check` | 2026-10-12 ~ 10-14 | 유효(`v0.1.3` draft 를 지우지 않은 예외는 [v0.1.4 릴리스](2026-10-14-adr-v0-1-4-release.md)) |
| [2026-10-14-adr-macos-app-bundle-dmg.md](2026-10-14-adr-macos-app-bundle-dmg.md) | macOS 는 공증 티켓을 붙인 `R-BMS.app` + dmg. 맨 실행 파일은 티켓을 붙일 수 없어 기각 | 2026-10-14 | 유효 |
| [2026-10-14-adr-layout-seed-range.md](2026-10-14-adr-layout-seed-range.md) | 새 배치 시드는 2^53 미만, 서버는 범위 밖 시드를 0 으로 받아 판을 거절하지 않음. KNOWN ISSUE 1건 | 2026-10-14 | 유효 |
| [2026-10-14-adr-release-toolchain-pin.md](2026-10-14-adr-release-toolchain-pin.md) | 릴리스 빌드 툴체인을 `rust-toolchain.toml` 과 같은 1.95.0(`dtolnay/rust-toolchain@master`)으로 | 2026-10-12 | 유효 |
| [2026-10-14-adr-table-page-no-source-notice.md](2026-10-14-adr-table-page-no-source-notice.md) | 발광 표 페이지·응답·곡 데이터에서 다른 사이트의 흔적을 뺀다(출처·삭제 요청 문구, 정보 패널·카드의 출처, `original_url`·`mirror_of`, 곡 `comment`·artist 주소). 곡 다운로드 링크(`url`·`url_diff`·`name_diff`)는 남긴다 | 2026-10-14, 추가·정정 2026-10-15 | 유효(파일 안: 첫 결정의 "정보 패널 출처 유지"는 추가 결정이, 추가 결정 2 의 다운로드 링크 삭제는 정정이 대체. 현재 상태 절이 정본) |
| [2026-10-14-adr-judge-count-consumed-notes.md](2026-10-14-adr-judge-count-consumed-notes.md) | 점수 제출의 판정 수 검사는 노트를 소비한 판정만(空POOR 제외, 9K 는 BAD 제외). 空POOR 400 사고의 수정 | 2026-10-14 | 유효 |
| [2026-10-14-adr-ir-error-field-display.md](2026-10-14-adr-ir-error-field-display.md) | 서버는 운영에서도 오류 `details` 의 `field`·`reason` 만 내고, 결과 화면 `IR ERROR:` 줄과 기록 상세가 코드·필드·이유를 보인다 | 2026-10-14 | 유효 |
| [2026-10-14-adr-v0-1-4-release.md](2026-10-14-adr-v0-1-4-release.md) | `v0.1.0`·`v0.1.3` draft 는 두고, 다음 릴리스는 S19 고도화까지 넣은 `v0.1.4`(2026-10-15 게시됨) | 2026-10-14 | 유효 |
| [2026-10-14-adr-record-sync.md](2026-10-14-adr-record-sync.md) | 로컬 이력 전체를 서버로 동기화(자동+수동, 리플레이 포함), 판별 서버 상태 기록, 거절 판은 다시, 옛 SCRATCH AUTO 제외, 서버는 같은 초 멱등 | 2026-10-14 | 일부 대체됨: 결정 2 의 거절 재전송 시점 → [거절 재전송](2026-10-15-adr-record-sync-refusal-resend.md) |
| [2026-10-14-adr-abandoned-run-saved.md](2026-10-14-adr-abandoned-run-saved.md) | Esc 중도 포기 판도 FAILED 로 저장·전송(남은 노트는 놓침) | 2026-10-14 | 일부 대체됨: 결정 2(판정 없는 판 제외)·결정 3 의 코스 부분(코스 도중 Esc 무기록) → [Esc 판은 모두 저장](2026-10-15-adr-escape-keeps-every-run.md) |
| [2026-10-14-adr-play-hitch-criteria.md](2026-10-14-adr-play-hitch-criteria.md) | 플레이 중 렉의 기준: 프레임 스레드에 I/O·큰 할당·잠금·동기 네트워크 없음, 헤드리스 프레임 처리 p99 1 ms 미만, 오디오 언더런 0, 로딩·결과 진입 끊김 없음 | 2026-10-14 | 유효 |
| [2026-10-15-adr-avgjudge-reference-definition.md](2026-10-15-adr-avgjudge-reference-definition.md) | `avgjudge` 는 원본 정의(평균 절대 오차, 놓친 노트 1초), 치우침은 `extra.mean_offset_us`, 본문에 `extra.rule_version`. 규칙 세대 4, 옛 행 값은 최고 기록에서 빼고 재전송은 0 | 2026-10-15 | 유효 |
| [2026-10-15-adr-event-time-judging.md](2026-10-15-adr-event-time-judging.md) | 입력은 들어온 시각(키는 받은 순간, 컨트롤러는 gilrs 시각)으로, 스윕·지연 릴리스·지뢰·HCN 틱·GAS 는 그 시각 순서로. HCN 틱은 경과 시간, 게이지가 빈 순간 판정 멈춤, 리플레이 `clock` 세대(EVENT/FRAME) | 2026-10-15 | 유효(배경의 원본 서술은 정정 표기 있음. 결정 6 의 컨트롤러 버튼 디바운스 방식은 [키보드 시각](2026-10-15-adr-keyboard-input-timing.md) 남은 한계 4 가 대체, 남은 한계 PERF-2 는 같은 ADR 이 해소) |
| [2026-10-15-adr-total-override-assist.md](2026-10-15-adr-total-override-assist.md) | TOTAL 덮어쓰기(>0)는 넓힌 판정과 같은 어시스트(저장·전송, 최고 기록·순위 제외), 리플레이에 TOTAL 기록·재생은 기록값, 업로드는 안 함 | 2026-10-15 | 유효 |
| [2026-10-15-adr-failed-run-bp-unreached.md](2026-10-15-adr-failed-run-bp-unreached.md) | 게이지 사망 판의 BP 에 도달하지 못한 노트를 넣는다(`min_bp_with_unreached`), 옛 행은 판정 수에서 계산해 비교 | 2026-10-15 | 유효 |
| [2026-10-15-adr-course-stage-combo.md](2026-10-15-adr-course-stage-combo.md) | 코스 곡별 기록의 max_combo·풀콤은 스테이지 콤보로, 화면·다음 스테이지는 코스 콤보. 코스 램프는 원본 규칙(코스의 끊김 없는 연속, 2026-10-15 사용자 확인) | 2026-10-15 | 유효 |
| [2026-10-15-adr-full-combo-condition.md](2026-10-15-adr-full-combo-condition.md) | 풀콤 = 끝에 남은 콤보가 지나간 노트 수(원본 `BMSPlayer.java:868`). 5K·PMS 의 마지막 노트 뒤 空POOR 는 풀콤을 깬다 | 2026-10-15 | 유효 |
| [2026-10-15-adr-long-note-body-notes.md](2026-10-15-adr-long-note-body-notes.md) | 같은 레인 롱노트 몸통 안의 일반 노트는 원본처럼 레인에서 빼 배경음으로(지뢰는 버림), 머리 칸·삼킴 규칙도 원본대로. 곡 DB `PARSER_VERSION` 3(지금은 5) | 2026-10-15 | 유효 |
| [2026-10-15-adr-unpaired-long-note-heads.md](2026-10-15-adr-unpaired-long-note-heads.md) | 끝나지 않은 롱노트 머리는 디코더가 지우고 노트 수 정의를 `rbms-model` 한 곳에. 판정 엔진 `total_notes` 와 같은 값 | 2026-10-15 | 유효 |
| [2026-10-15-adr-escape-keeps-every-run.md](2026-10-15-adr-escape-keeps-every-run.md) | Esc 판은 판정 수와 상관없이 FAILED 저장·전송. 코스 도중 Esc 는 남은 노트가 없으면 정상 판정, 있으면 곡 기록 + FAILED 코스 기록까지. 앱 종료·코스 스테이지 사이에서 나감도 같은 규칙 | 2026-10-15 | 유효 |
| [2026-10-15-adr-record-sync-refusal-resend.md](2026-10-15-adr-record-sync-refusal-resend.md) | 저절로 시작한 동기화는 같은 클라이언트·서버 빌드에서 이미 거절된 판을 다시 보내지 않는다. 빌드가 바뀌거나 SYNC RECORDS NOW 면 다시 | 2026-10-15 | 유효(사용자 미확인) |
| [2026-10-15-adr-app-icon.md](2026-10-15-adr-app-icon.md) | 앱 아이콘: 스크립트로 만든 무채색 레인 그림, macOS `.icns`, Windows 는 `rc.exe` + `build.rs`(새 의존성 없음), 창 아이콘, 웹 파비콘 | 2026-10-15 | 유효 |
| [2026-10-15-adr-keyboard-input-timing.md](2026-10-15-adr-keyboard-input-timing.md) | 화면 대기(present)를 전용 스레드로 떼어 이벤트 루프가 키를 오는 즉시 받고 그 순간으로 판정(LR2·beatoraja·osu! 의 원리). KEY DEBOUNCE·PAD DEBOUNCE(기본 16 ms, LR2 `inputinterval` 형: 누름은 즉시, 누른 뒤 창 안의 뗌만 미룸), 아날로그 턴테이블은 원본 양방향 잠금 유지 | 2026-10-15 | 유효(기본값 16 ms·컨트롤러 LR2 형·턴테이블 잠금 유지는 사용자 확인 끝. 메뉴의 PAD DEBOUNCE 는 확인 대기) |
| [2026-10-15-adr-wgpu-metal-main-thread.md](2026-10-15-adr-wgpu-metal-main-thread.md) | THR2-1: present 스레드의 wgpu Metal 획득이 AppKit 메인 스레드 API 를 읽는 위반은 구조를 유지하고 알려진 위험으로, 상류 gfx-rs/wgpu#10627 을 기다린다 | 2026-10-15 | 유효 |
| [2026-10-15-adr-accidental-sync-records-kept.md](2026-10-15-adr-accidental-sync-records-kept.md) | 감사 에이전트의 실수로 운영 서버에 간 판 49개(리플레이 43 대기)는 지우지 않고 둔다. 재발 방지는 scratch HOME 규칙과 `--help`·`--version` 처리 | 2026-10-15 | 유효 |
| [2026-10-15-adr-bga-resident-texture-cap.md](2026-10-15-adr-bga-resident-texture-cap.md) | BGA 상주 텍스처 상한 `RESIDENT_BYTES` 256 MiB 유지(메모리보다 플레이 프레임 p99) | 2026-10-15 | 유효 |

## 그 전의 결정 파일

| 파일 | 한 줄 요약 | 날짜 | 상태 |
| --- | --- | --- | --- |
| [2026-10-14-web-decisions.md](2026-10-14-web-decisions.md) | 웹 배포 준비: 사용자 결정 U1~U5, 메인 추천안 M1~M11, 질문 Q1~Q6 | 2026-10-14 | 일부 대체됨: Q5, Q6 의 실행 주체 → [운영 DB 작업](2026-10-14-adr-production-db-operations.md), M1·Q1 의 출처·삭제 요청 문구 → [표 페이지](2026-10-14-adr-table-page-no-source-notice.md) |
| [2026-10-09-lua-skin-compat-decisions.md](2026-10-09-lua-skin-compat-decisions.md) | Lua 스킨 호환 D1~D18(기본 스킨 자작, F=즐겨찾기만, Cargo.lock 추적, IIDX 식 판정, 정적 판정 글자, "GREAT 67", 앱 UI 무채색 등), 동영상 디코더 | 2026-10-09 ~ 10-12 | 유효 |
| [2026-09-17-skin-system-decisions.md](2026-09-17-skin-system-decisions.md) | 스킨 시스템 완성 D1~D23 | 2026-09-17 | 대체됨(2026-10-09, Lua 스킨 호환 결정) |
| [2026-09-17-commit-language.md](2026-09-17-commit-language.md) | 커밋 설명 언어는 한국어 | 2026-09-17 | 유효 |
| [2026-09-09-enhancement-decisions.md](2026-09-09-enhancement-decisions.md) | 고도화 결정 1~12, 웹 W1~W8·OQ1~OQ6, Phase E E1~E13, 마무리 순서 | 2026-09-09 ~ 09-10 | 일부 대체됨: 결정 8(작업 규칙), 결정 12 → [SCRATCH AUTO 규칙](2026-10-14-adr-scratch-auto-rule.md)·[모든 판 저장·전송](2026-10-14-adr-save-and-send-every-played-run.md), 마무리 순서의 릴리스 → [릴리스 정책](2026-10-14-adr-release-policy.md) |
| [release-branch-strategy.md](release-branch-strategy.md) | prod 브랜치·보호·첫 버전의 옵션(결정 대기였음) | 2026-09-16 | 대체됨 → [릴리스 정책](2026-10-14-adr-release-policy.md) |
| [design.md](design.md) | 웹 디자인 정본(토큰, Surface, 화면 규칙) | 2026-09-09(최종 수정 2026-10-14) | 유효(웹 2026-10-14 디자인 기준과의 차이는 `docs/web/2026-10-14-design-direction.md` 3.1.1) |
| [empty-poor-local-scores.md](empty-poor-local-scores.md) | 空POOR 처리와 로컬 기록 스키마 | 2026-06-03(상태 갱신 2026-09-16, 대조 2026-10-15) | 일부 대체됨: 풀콤 조건 → [풀콤 조건](2026-10-15-adr-full-combo-condition.md), LightAssistEasy 흡수 → [SCRATCH AUTO 규칙](2026-10-14-adr-scratch-auto-rule.md). 파일 머리의 대조 결과 참고 |
| [reference-divergences.md](reference-divergences.md) | 레퍼런스 구현과의 의도적·보류 차이. 2026-10-14~15 절: Esc 포기 판, 플레이 수리, 롱노트 정리, 입력 시각 판정, 기록의 원본 정의, 연습과 #TOTAL | 2026-09-09 ~ 2026-10-15 | 유효(Phase D 의 D-D6, Phase E 절은 이력. 파일 안 표시) |
| [decisions.md](decisions.md) | 포팅 착수 결정(그래픽 스택, 첫 범위, 모드 확장성) | 2026-05-31 | 대체됨(범위 제한은 Phase E~G 에서 해소) |

## 사용자에게 아직 확인받지 않은 메인의 결정

결정마다 지금 동작(메인이 정한 것)과 선택지를 적는다. 사용자가 답하면 해당 ADR 을 새로 쓰거나 고치고 이 목록에서 뺀다.

1. **순위 `lnmode` 필터의 뜻.** 지금은 "최고 기록이 그 LN 방식으로 친 판인 플레이어"만 남기고 그 안에서 순위를 센다(`chart_best` 키에 LN 방식이 없음). 선택지: (a) LN 방식별 최고 기록(키 확장 + 마이그레이션), (b) 파라미터 제거, (c) 지금대로. 문서 `docs/backend/api-spec.md` 4절, 테스트 부채 3.35.
2. **코스 램프의 서버 검사.** 0.1.0 클라이언트가 마지막 스테이지의 램프를 코스 램프로 보내므로 서버는 코스 램프를 판정 수와 대조하지 않는다(`course.service.ts` 의 `findCourseIntegrityFailure` 문서). 0.1.0 이 쓰이지 않게 되면 검사를 켤지. 테스트 부채 3.36.
3. **저장소에 남은 다른 사이트의 흔적(서빙하지 않음).** `web/data/tables/insane1/source.json`(원본 주소·HTTP 기록, 스냅샷 재현용), `web/scripts/snapshot-table.ts` 의 사용 예, `crates/rbms-config/src/tables.rs` 의 `same_as: ["darksabun.club/table/archive/insane1"]`(원본 주소를 이미 등록한 사용자에게 사본을 중복 등록하지 않기 위한 값)와 그 테스트, `docs` 의 조사 문서. 메인 판단: 기능에 쓰이는 `tables.rs` 는 유지, `source.json` 은 스냅샷 재현용으로 유지를 권한다. [표 페이지 ADR](2026-10-14-adr-table-page-no-source-notice.md) 의 현재 상태 절.
4. **기록 동기화의 거절 재전송 좁힘.** 저절로 시작한 동기화는 같은 빌드끼리 이미 거절된 판을 다시 보내지 않는다(수동과 빌드 변경 때는 보낸다). [거절 재전송 ADR](2026-10-15-adr-record-sync-refusal-resend.md).
5. **메뉴 입력의 PAD DEBOUNCE.** 컨트롤러 디바운스가 LR2 형이 되면서 메뉴에서도 뗌 뒤 2~16 ms 튀는 버튼이 두 번 눌린 것이 될 수 있다. 메뉴에만 옛 양방향 잠금을 둘지. [키보드 시각 ADR](2026-10-15-adr-keyboard-input-timing.md) 남은 한계 4, 테스트 부채 3.71.
6. **연습 판의 게이지 사망.** 지금은 게이지 잠금으로 구간 끝까지 판정한다(NONE 을 CONTINUE 로). 원본은 연습 패널로 돌아간다. A 원본대로 / B 지금대로 두고 ADR 로 남긴다. `reference-divergences.md` "연습과 #TOTAL" 절, `docs/play-options.md` 9절.

확인받은 것(2026-10-14): SCRATCH AUTO 램프 LIGHT ASSIST 유지, 넓힌 판정 판은 저장·전송하되 최고 기록·순위 제외(추천안 A), `v0.1.0` draft 는 둔다, `v0.1.3` draft 도 두고 다음은 `v0.1.4`. 확인받은 것(2026-10-15): Esc 포기 판의 두 예외 거부([Esc 판은 모두 저장](2026-10-15-adr-escape-keeps-every-run.md)), 키 디바운스 기본 16 ms·컨트롤러 LR2 형·PAD DEBOUNCE 설정·턴테이블 원본 잠금([키보드 시각](2026-10-15-adr-keyboard-input-timing.md)), 코스 램프 원본 규칙([코스 곡별 콤보](2026-10-15-adr-course-stage-combo.md)), BGA 상주 256 MiB([BGA 상주 상한](2026-10-15-adr-bga-resident-texture-cap.md)), THR2-1 구조 유지([wgpu Metal 메인 스레드](2026-10-15-adr-wgpu-metal-main-thread.md)), 사고 49개 유지([사고 기록 유지](2026-10-15-adr-accidental-sync-records-kept.md)).

사용자 실기 확인으로 남은 것(결정이 아님): v0.1.4 의 IR: RANK·SYNC RECORDS NOW·키 입력 체감·JUDGE OFFSET 재조정·PAD DEBOUNCE 확인(게시는 2026-10-15 07:27 UTC 에 끝남), 사고 때 개발 빌드가 다시 쓴 설정이 의도대로인지. 오래된 질문(웹 rate limit 지속형 저장소, `user.rank_points` 처리)은 `docs/HANDOFF.md` 와 웹 문서에 있다.
