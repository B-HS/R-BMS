# Phase F — 갈래 합류와 교차 배선 (2026-09-10)

> 근거: `docs/plan/2026-09-09-phase-f-spec.md`, 결정 `docs/acknowledge/2026-09-09-enhancement-decisions.md`(결정 3·6), 발산 `docs/acknowledge/reference-divergences.md` §Phase F.
> F0~F4 다섯 갈래가 같은 워크트리에서 병렬로 작업한 뒤의 통합 단계다. 각 갈래가 "내 소유 파일만으로는 끝낼 수 없다"고 넘긴 항목을 여기서 배선하고, 나머지는 발산 문서에 미구현으로 등록했다.

## 게이트

작업 시작 시점에 `fmt` / `build` / `test` / `clippy --all-targets -D warnings` 는 이미 전부 통과 상태였다. 아래 배선을 얹은 뒤 다시 전부 통과한다.

| 명령 | 결과 |
|---|---|
| `cargo fmt --all` | 통과 |
| `cargo build --workspace --all-targets` | 통과 |
| `cargo test --workspace` | 38개 테스트 바이너리 전부 ok, 실패 0 (`rbms-player` 534→544, `rbms-render` 180→182) |
| `cargo clippy --workspace --all-targets -- -D warnings` | 통과 |

## 배선한 교차 항목

| 요청 | 배선 | 소재 |
|---|---|---|
| F2→F3: `ResultView.fast/slow` 가 `[summary.fast, 0]` 로 스크래치 열을 버림 | `enter_result` 가 `PlayState.fast/slow`(키·턴테이블 분리 집계)를 그대로 넘긴다 | `apps/rbms-player/src/app_result.rs`, 테스트 `stage/play/tests.rs` |
| F2→F1/F3: 스펙 F2-6 의 5K/10K 모드 라벨이 HUD 에만 있음 | 곡선택은 이미 `mode_short` 로 표시 중임을 확인. 결과 화면에 `ResultView.mode_label` 을 신설하고 HUD 와 **같은 좌표·같은 스케일**(`hud::MODE_LABEL_*` 를 `pub(crate)` 로 공유)로 그린다 | `crates/rbms-render/src/{hud.rs,result.rs}`, `apps/rbms-player/src/app_result.rs` |
| F4→F0: LETTERBOX 가 다음 로딩 전까지 반영되지 않음 | `App::draw_overlays` 가 매 프레임 `gpu::apply_letterbox` 를 부른다. `LoadingState::draw` 의 중복 호출은 제거 | `apps/rbms-player/src/lib.rs`, `apps/rbms-player/src/stage/loading.rs` |
| F4→F0: 설정 화면 텍스트 편집기가 append/backspace 뿐 | 표 URL 입력과 **같은 편집기**로 통일. 두 화면이 쓰던 키 처리를 `textedit::edit_key` 로 한 번만 쓰고, 표시는 `TextEdit::window` 로 캐럿을 따라가는 창을 그린다 | `apps/rbms-player/src/textedit.rs`, `.../ir_panel.rs`, `.../stage/settings.rs`, `.../stage/tables.rs` |
| F3→F2: 실시간 PACEMAKER 미구현 | `HudPace{name, delta}` 를 HUD 에 신설. 런 시작 시 타깃을 한 번 확정(`ensure_pace_target`)하고, 매 프레임 `현재 EX − 타깃의 동시점 EX`(진행률 = 판정된 노트 / 전체 노트)를 그린다 | `crates/rbms-render/src/hud.rs`, `apps/rbms-player/src/stage/play/mod.rs` |
| F3/F4→F0: re-export 누락 | `LaneShade`, `HudPace`, `ResultExtras`, `TargetView`, `RATE_STEPS`, `rate_27`, `dj_rank_label`, `draw_rank_bar_stepped` 를 크레이트 루트에 노출 | `crates/rbms-render/src/lib.rs` |
| F1→F0: `PREVIEW_GAIN` 이 사문 | 상수와 그것만 살려 두던 `const _: () = assert!(..)` 를 함께 제거. "마이그레이션이 음량 중립"이라는 사실은 `DEFAULT_PREVIEW_VOLUME` 의 문서로 옮겼다 | `apps/rbms-player/src/lib.rs`, `.../stage/select/preview.rs`, `crates/rbms-config/src/schema.rs` |
| F1→오케스트레이터: 정렬 비교자 방향 확인 요청 | 레퍼런스 `BarSorter.java:139,159-162,219` 를 직접 열어 **오름차순·무기록 마지막**이 맞음을 확인. F1 의 뒤집기를 승인하고, 아직 미구현인 `RivalClear`/`RivalScore` 의 "best first" 주석을 같은 방향으로 정정 | `crates/rbms-config/src/sort.rs` |
| F3→오케스트레이터: 발산 2건 등록 | `dj_rank_label` 의 밴드 3등분 축(F-D1)과 `draw_rank_bar_stepped` 의 분리(F-D2)를 등록. 정렬 방향 패리티(F-D3)도 함께 | `docs/acknowledge/reference-divergences.md` |

## 부수 정리

- `apps/rbms-player/src/stage/play.rs` 가 979행까지 자라 800행 상한을 넘겼다. 인라인 `mod tests` 를 자식 모듈로 분리해 `stage/play/mod.rs`(545) + `stage/play/tests.rs`(440)로 나눴다. `stage/select/` 가 이미 쓰던 배치와 같다.
- `app_result::enter_result` 안에 인라인이던 타깃 확정을 `run_target()` 으로 뽑았다. HUD 와 결과 화면이 같은 함수를 부르므로, 런 중에 페이스를 맞추는 선과 끝나고 채점되는 선이 같은 하나다.
- 스냅샷 하네스의 `1.0 / 60.0` 반복을 `render_tests::FRAME_DT` 로 모았고, 오버레이 패스까지 포함해 한 프레임을 그리는 `render_over` 헬퍼를 더했다(LETTERBOX 배선이 이 경로로 검증된다).
- 결과 화면 골든 3종(`GOLDEN_RESULT`, `GOLDEN_RESULT_SKIN`, `GOLDEN_RESULT_PACED`)은 모드 라벨 추가로 의도적으로 재생성했다. 다른 화면의 골든은 움직이지 않았다.

## 남긴 것

미구현·미배선 항목은 전부 `docs/acknowledge/reference-divergences.md` §Phase F 의 "미구현으로 남긴 것" 목록에 사유와 필요 조건을 적었다. 가장 큰 것은 **LANE OPTION 의 FLIP / BATTLE / BATTLE AUTO-SC** 로, 설정 쪽은 완비됐지만 런에 반영하려면 모델 레인 재배치와 SP→DP 모드 승격이 필요해 통합 단계의 배선 범위를 넘는다.

## 프로세스 지적

- 스펙 §6 R9("F0 커밋이 dev 에 들어가기 전에는 브랜치를 만들지 않는다")가 지켜지지 않아, 세션 초반 워크스페이스가 여러 번 컴파일 불가였다. 다음 페이즈에서 강제할 것.
- `crates/rbms-render/examples/{render_frame,render_result,render_select}.rs` 는 소유권 표 어디에도 없는데 뷰 타입이 바뀔 때마다 `--all-targets` 게이트를 막는다. 다음 페이즈부터 F0 소유로 명시할 것.
