# Phase D — 판정 패리티 완성 (2026-09-10)

> 근거: `docs/plan/2026-09-09-phase-d-spec.md`, 결정 `docs/acknowledge/2026-09-09-enhancement-decisions.md`(결정 12), 발산 `docs/acknowledge/reference-divergences.md`.
> Phase C(구조 개편)가 스펙 작성 이후 착지해, `crates/rbms-judge`가 이미 `algorithm.rs`/`data.rs`/`data/*.ron`과 `JudgeAlgorithm` enum을 갖고 있었고 앱은 `apps/rbms-player/src/stage/*.rs` + `crates/rbms-config`(설정)/`crates/rbms-store`(스코어·리플레이)/`crates/rbms-library`로 갈라져 있었다. D0~D5는 착수마다 먼저 실제 코드를 읽어 기존 타입을 확장하는 방향으로 스펙 라인번호를 심볼명으로 재앵커링했다.

## 진행 순서와 산출

- **D0(judge-decl)** — `crates/rbms-judge/src/lib.rs`에 `pub mod gauge_tables;` / `pub mod ln;` 선언 추가(`algorithm`/`data`는 Phase C가 이미 선언). 인라인 `#[cfg(test)] mod tests { .. }`(구 lib.rs 33~926행)를 `crates/rbms-judge/src/tests.rs`로 이관.
- **D1(judge-gauge)** — 스펙 §2 전 항목(J20/J21/J22/J26 + `ClearType` 확장 + 램프 매핑) 구현. `gauge_tables.rs`에 5세트×9원소(45) `GAUGE_TABLE`을 레퍼런스 `GaugeProperty.java:77-125` 그대로 이식, `GaugeSetId{FiveKeys,SevenKeys,Pms,Keyboard,Lr2}` + `for_mode`, `MODIFY_DAMAGE`(EXHARD_5/HARD_LR2/EXHARD_LR2), `GrooveGauge`가 9종을 병렬 갱신.
- **D2(judge-window)** — 소유 파일 2개(`windows.rs`, `algorithm.rs`)만 수정. `JudgeAlgorithm` 4종(Combo/Duration/Lowest/Score)과 `prefer()`의 술어 4개를 레퍼런스 `JudgeAlgorithm.java:17-37`과 문자 단위로 동치 확인. `JudgeWindowRule{Normal,Pms}`을 신설해 `JudgeProperty.java:209-224`의 judgerank 스케일링·fixjudge·클램프를 이식, `judge.ron`은 내용 변경 없이 pin 테스트의 대조 대상(SSOT)으로 고정.
- **D3(judge-matcher)** — 소유 파일 3개(`matcher.rs`/`ln.rs`/`tests.rs`)만 수정. §1 2단계 후보 폴드(`JudgeManager.java:385-415`)를 `select_candidate`의 stage1/stage2로 이식하고, 비평이 지적한 "판정 완료 노트가 t1로 고정되는 결함"을 `best.state != UNJUDGED_STATE ||` 절로 해소(변이 테스트로 검증). CN deferral·plain LN 헤드 확정·HCN 200ms 틱·BSS/MSS를 배선.
- **D4(play 통합)** — 스펙 §6 + §2~§5의 play 배선 항목을 D1~D3 산출물(소비만, 재정의 없음)로 구현. 24K Mode를 레퍼런스 `Mode` 원본에서 확정해 신설(레퍼런스 §11 미확인 1번 해소).
- **D5(app-judge-tab)** — JUDGE 탭 13행(스펙 §7 표 전부)을 디스크립터 테이블로 노출. 스펙이 지정한 소유 파일(`app_select.rs`/`settings.rs`/`scores.rs`/`replay.rs`/`ir_map.rs`)은 Phase C가 `crates/rbms-config`·`crates/rbms-store`·`crates/rbms-ir/src/mapping.rs`·`apps/rbms-player/src/stage/*`로 옮겨 놓아 그 위치에서 작업.

## 적대 리뷰 23건 반영(FIX)

D0~D5 산출물에 대한 적대 리뷰(critical 1 · major 12 · minor 10)를 전부 수정했다. 항목과 소재 파일:

| 등급 | 지적 | 소재 |
|---|---|---|
| critical | 스크래치 JUDGE WIDTH가 네 판정 테이블 전부에 적용됨(레퍼런스는 key rate만 사용) | `crates/rbms-judge/src/windows.rs` |
| critical | LN MODE 설정이 무효화됨 — `set_ln_mode`가 `LnKind::Undefined`를 소비해 두 번째 호출이 no-op | `crates/rbms-judge/src/matcher.rs` |
| major | `JudgeAlgorithm` 기본값이 여전히 `Duration` — 스펙이 요구한 `Combo` 미전환 | `crates/rbms-judge/src/algorithm.rs`(`#[default] Combo`), `crates/rbms-config/src/schema.rs` |
| major | 역회전 스크래치가 판정 엔진에 도달하지 않음(BSS/MSS 앱에서 사문) | `apps/rbms-player/src/app_play.rs`, `apps/rbms-player/src/app_input.rs` |
| major | `GAUGE AUTO SHIFT = NONE`의 stage failed 전이가 앱에 배선되지 않음 | `apps/rbms-player/src/stage/play.rs` |
| major | LN MODE 강제 시 노트 수 분모가 어긋남(`to_model`이 아니라 엔진에서만 해소) | `apps/rbms-player/src/app_play.rs` |
| major | LN MODE가 리플레이·스코어 키에 기록되지 않아 재현·비교가 깨짐 | `crates/rbms-store/src/replay.rs`, `crates/rbms-store/src/score.rs` |
| major | `apply_mode`/`from_model_for_mode`가 Normal 규칙으로 고정 — PMS 윈도우가 틀림 | `crates/rbms-judge/src/matcher.rs`(`JudgeWindowRule::for_mode` 반영) |
| major | 계정 설정 blob schema 1→2 마이그레이션 누락 — JUDGE WIDTH가 다운로드 시 100%로 리셋 | `apps/rbms-player/src/ir_sync.rs` |
| major | 스크래치 역회전 키가 플레이 중 디스패치되지 않음 | `apps/rbms-player/src/app_input.rs` |
| major | `best_clear_for_md5`가 assisted 기록을 세기 시작했으나 구 기록의 램프가 강등되지 않아 소급 상승 | `crates/rbms-store/src/score.rs`(`rule_version`/`is_stale_rule_version` 게이팅) |
| major | JUDGE ALGORITHM 기본값이 `DURATION`(스펙 §7이 요구하는 `COMBO` 아님) | `crates/rbms-config/src/schema.rs` |
| major | `PlaySession::seek`이 엔진 클럭을 전진시키지 않아 시크 재생이 정면 재생과 달라짐 | `crates/rbms-play/src/session.rs` |
| major | `Player::release`가 `auto_lanes`를 무시 — 오토 스크래치 레인에서 키를 떼면 오토플레이 LN이 파괴됨 | `crates/rbms-play/src/lib.rs` |
| minor | `apply_mode`/`from_model_for_mode`가 여전히 Normal 규칙으로 스케일해 PMS 오판정(major와 별개 잔여분) | `crates/rbms-judge/src/matcher.rs` |
| minor | PMS/KEYBOARD 빈 스크래치 테이블을 키 테이블로 대체한 사실이 기록되지 않음 | `crates/rbms-judge/src/windows.rs` |
| minor | GAS로 게이지가 바뀐 런이 원래 선택 게이지로 보고됨(`is_type_changed` 미사용) | `apps/rbms-player/src/stage/play.rs` |
| minor | KEYBOARD 전용 default TOTAL 공식이 어느 경로에서도 쓰이지 않음 | `crates/rbms-chart/src/lib.rs` |
| minor | Phase D 발산 기록이 갱신되지 않음(J17/J20~J26/J9/J12/J6가 "보류") | `docs/acknowledge/reference-divergences.md` |
| minor | 리플레이가 `ln_mode`·`gauge_set`·`gauge_auto_shift`·`bottom_shiftable_gauge`를 기록하지 않음 | `crates/rbms-store/src/replay.rs` |
| minor | `JudgeProperty::defaults_for_mode`에 KEYBOARD_24K 분기 없어 폴백이 24K를 7K로 강등 | `crates/rbms-judge/src/windows.rs` |
| minor | `total_judged`가 노트를 소비하지 않은 판정(PMS BAD)까지 세어 `passnotes`와 어긋남 | `crates/rbms-judge/src/matcher.rs` |
| minor | assist 1(AUTO SCRATCH) 런의 리플레이 저장이 레퍼런스와 다름(미기록 발산) | `apps/rbms-player/src/lib.rs`(문서화, D-D6로 발산 유지) |
| minor | `assisted_lamp`의 catch-all이 더 강한 assist를 더 약한 강등으로 매핑 | `apps/rbms-player/src/app_play.rs` |
| minor | `app_play.rs`가 800줄 상한을 넘김(693 → 833) | `apps/rbms-player/src/app_play.rs` |

23건 전부 반영. 각 건마다 회귀·pin 테스트를 추가해 재발을 고정했다(`crates/rbms-judge/src/tests.rs`, `crates/rbms-play` 테스트, `crates/rbms-store/src/tests.rs` 등).

## 검증 (2026-09-10 실측)

- `git rev-parse --short HEAD`: `0c20fa9`(dev) — Phase D/FIX 산출물은 이 커밋 위 워킹트리 변경으로 존재(미커밋)
- `cargo fmt --all -- --check`: 통과(EXIT 0)
- `cargo test --workspace`: **1876 통과 · 0 실패 · 3 ignored**(전 크레이트 unit + 통합 + doc-test 포함, 실패 없음)
- `cargo clippy --workspace --all-targets -- -D warnings`: **0 경고**
- 금지어(레퍼런스 구현 실명) grep: **0건**
- `git diff`의 신규 `//` 라인(`///`/`//!` 제외): **0건**
- `git status --short`: 소유 파일 대부분 수정(`M`) + 신설 파일 5개(`apps/rbms-player/src/judge_setup.rs`, `crates/rbms-config/src/judge.rs`, `crates/rbms-judge/src/gauge_tables.rs`, `crates/rbms-judge/src/ln.rs`, `crates/rbms-judge/src/tests.rs`) — 전부 D0~D5·FIX 소유 범위 내

## 남은 것

- 스펙 §7 TARGET 행은 값 저장만 하고 페이서 계산은 Phase F.
- 24K의 렌더 레인 레이아웃·차트 스캔은 Phase G(현재는 `Mode::ALL`에 미포함이라 UI 도달 경로 없음).
- `gauge.ron`은 여전히 `BEAT_7K` 1행뿐 — 5K/PMS/KB/LR2 등 비-7K 세트 수치는 `gauge_tables.rs`의 컴파일 내장 표(레퍼런스 이식 + pin 테스트)로 대신하고 있다.
- 나머지 세부 발산은 `docs/acknowledge/reference-divergences.md`의 Phase D 절 참조.
