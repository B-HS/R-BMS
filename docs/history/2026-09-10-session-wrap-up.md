# 2026-09-09 ~ 09-10 고도화 세션 마무리 (토큰 한도로 중단, 재개 지점 기록)

> 사용자 지시: "멈추라 할 때까지 계획대로 전부 구현" → 토큰 한도 도달로 "여기까지만 정리하고 docs 에 적어 두고 commit push". 이 문서가 재개의 진입점이다. 다음 세션은 이 문서와 `docs/PROCESS.md` 를 먼저 읽는다.

## 1. 현재 상태 한눈에

| 항목 | 상태 |
|---|---|
| `dev` HEAD | `9d603b0` (`fix(audio): answer device enumeration from one long-lived host thread`) — **CI 3-OS 전부 통과**(run 34445698764) |
| 작업 트리(dev) | clean |
| 미완 작업 브랜치 | `wip/phase-g` (`729ba39` + `docs(wip)` 1건) — Phase G 갈래 G0~G7·G9 완료분 + G8 배선 지시서. **컴파일 안 됨**(G8 배선 중 중단) |
| 웹 | `https://bms.hyuns.uk` 프로덕션 배포 최신(Phase I 계약 수정 반영), DB 실연동 정상 |
| 완료 Phase | I(클라 GUI IR 연동) · A · B · C · D · F · E |
| 남은 Phase | G(G8 배선·게이트·리뷰·수정·문서) → H(경미 후속 일괄) |
| 사용자 몫 | 릴리스(prod 브랜치·보호·v0.1.0·서명, 자택 키) · Phase B 실기 가청 확인 |

## 2. Phase 별 결과 (커밋 범위 · 수치)

| Phase | 커밋 | 게이트 실측 | history |
|---|---|---|---|
| I 클라 GUI IR 연동 + 계약 수정 | `a1987ba`..`a91f73f`, `7fa8106`..`f98899b` | 테스트 1,277 → 1,284, 실서버 GUI e2e(계정 `gui-e2e-871795`) | `2026-09-09-phase-i-client-ir-integration.md` |
| B 오디오 클럭 | `ec4d9d2`..`b683e88` | 1,475 통과, 소크 13분 통과, 보간 잔차 sd 12.5µs | `2026-09-09-phase-b-audio-clock.md`, `…-review-fixes.md` |
| C 구조 개편 | `3813075`..`a243ce5` | 1,656 통과, clippy `-D warnings` 0, CI 게이트 플립 | `2026-09-09-phase-c-structure.md` |
| D 판정 패리티 | `bd50521`..`0e13e57` | 1,876 통과, 리뷰 23건 반영 | `2026-09-09-phase-d-judge-parity.md` |
| F UX 고도화 | `f15a482`..`73e7d96` | 2,232 통과, 리뷰 21건 반영, 헤드리스 렌더 3종 확인 | `2026-09-09-phase-f-ux.md`, `2026-09-10-phase-f-integration.md` |
| E 스킨 | `7f28025`..`557a674` (+ `0f21ce7` 테스트 경합 수정) | 2,633 통과, 리뷰 28건 반영, 골든 PNG 4장 | `2026-09-09-phase-e-skin.md` |
| Windows 크래시 | `9d603b0` | cpal WASAPI COM 열거자 수명 문제, 호스트 스레드로 격리 | `docs/bug/2026-09-10-windows-skin-tab-access-violation.md` |

## 3. Phase G 재개 절차

1. `git checkout wip/phase-g` — 갈래 산출물: `crates/rbms-library`(곡DB SQLite), `crates/rbms-store/src/scoredb*`(스코어DB), `crates/rbms-course`, `apps/rbms-player/src/{practice,gamepad,syssound,ir_ext,library,scoredb_store,course_ui,course_ir}.rs`, `crates/rbms-parser/src/bmson*`, 배선 지시서 `docs/plan/phase-g-wiring/*.md`.
2. 현재 컴파일 오류: `apps/rbms-player/src/settings_ui.rs` `SHIPPED_ROWS` 가 `SettingId`(82) 보다 3행 적음(79) — G8 이 `rbms-config` 에 행을 추가한 뒤 스냅샷을 못 채운 상태. 먼저 이것부터 맞춘다.
3. `docs/utils/workflows/wf-phase-g.js` 의 **G8 단계부터** 실행(스크립트 안 `<reference-root>` 는 레퍼런스 구현 소스 루트로 치환). Remap/G0/갈래 결과는 `wip/phase-g` 에 있으므로 G8 프롬프트에 "갈래 산출물은 이미 트리에 있다" 를 명시하면 된다.
4. 게이트(`cargo fmt --all --check` · `cargo test --workspace` · `cargo clippy --workspace --all-targets -- -D warnings`) → 리뷰 3갈래 → 수정 → 실행 검증(HOME 격리, 자동 플레이 1곡 후 SQLite 조회) → 크레이트별 Conventional Commit → `dev` 로 병합(fast-forward 또는 `git merge --no-ff`, force push 금지) → CI 3-OS 확인.
5. 이어서 `docs/utils/workflows/wf-phase-h.js`(경미 후속 일괄).

## 4. Phase H 에 누적된 경미 항목

- 토스트가 곡선택 우하단 힌트 줄과 겹침(토스트를 힌트 위로 올리거나 표시 중 힌트 숨김)
- `app_input::tests::the_settings_row_and_the_in_play_key_stop_at_the_same_ceiling` 가 Linux CI 에서 60초 초과 → 경계값 검사로 축소
- 웹: md5 단독 제출(LR2IR 어댑터), Vercel Blob 리플레이 저장(현재 `REPLAY_STORAGE=db` 만), 슈퍼셋 서버 누락 필드 `.default()`
- 각 Phase history 의 "알려진 제약/후속" 절 항목(H 의 Collect 단계가 전수 수집)

## 5. 운용 메모

- 위임은 Workflow 만, 구현 Opus max · 리뷰 Opus high · 기계 작업 Sonnet. 세션 사용량 한도로 워크플로가 끊기면 `Workflow({scriptPath, resumeFromRunId})` 로 재개 가능(완료 에이전트는 캐시).
- 에이전트가 규칙과 달리 `//` 주석을 넣는 일이 반복됐다(Phase C 에서 693줄). 게이트에서 `grep -rnE '^\s*//[^/!]' --include='*.rs' crates apps tools` 로 매번 확인한다.
- 커밋 훅은 명령 문자열을 정적으로 검사하므로 커밋 메시지는 변수가 아니라 리터럴로 쓴다.
- 레퍼런스 구현 명칭은 코드·문서·커밋 어디에도 쓰지 않는다(`<reference-root>` 플레이스홀더).
