# Phase H — 경미 후속 정리 (2026-09-16)

## 대상

- 토스트 배치: `crates/rbms-render/src/{lib.rs,toast.rs}`, `apps/rbms-player/src/{lib.rs,toast.rs,stage/render_tests_shell.rs}`
- 경계 회귀 검사: `apps/rbms-player/src/app_input.rs`
- 실행과 수동 점검 정본: `README.md`, `docs/development.md`, `docs/quality-assurance/2026-09-16-phase-h-manual-checks.md`

## 완료

| 항목 | 결과 |
|---|---|
| H1 | SELECT 화면의 토스트에만 하단 navigation·hint 영역을 피하는 inset을 적용했다. 다른 화면의 기존 위치는 바뀌지 않는다. |
| H1 적대 검토 반영 | inset이 토스트 스택 footprint에 섞이지 않도록 반환값을 바로잡고, 지나치게 큰 inset에서도 최신 토스트가 화면 밖으로 밀리지 않도록 anchor를 제한했다. 두 경우의 렌더 회귀 검사를 추가했다. |
| H2 | `app_input::tests::the_settings_row_and_the_in_play_key_stop_at_the_same_ceiling`의 `App::new` 호출을 6회에서 1회로 줄이고 상한 정확값·상한 직전·하한 거부·하한 직후를 독립적으로 비교했다. 실측 시간은 16.93초에서 3.01초로 줄었다. |
| H3 | README에 현재 실행 명령과 GUI 폴더 등록 절차를 정본화하고, 실제 오디오·연습·코스·복수 IR 수동 확인 절차와 합격 기준을 분리해 기록했다. |

## 최종 검증

| 명령 또는 점검 | 실제 결과 |
|---|---|
| `cargo fmt --all --check` | exit 0 |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | exit 0 |
| `cargo test --workspace` | exit 0; 테스트 목록 3,040개 등록; 실제 오디오 장치 테스트 2건 ignored |
| `cargo build --release -p rbms-player` | exit 0; `target/release/rbms-player` 생성 |
| `git diff --check` | exit 0 |
| 독립 적대 리뷰 | blocker 0 |

## 수동 확인과 후속

`docs/quality-assurance/2026-09-16-phase-h-manual-checks.md`는 실제 장치와 서버가 필요한 AUDIO, Practice, Course, Multi-IR 절차를 정의한다. 이 문서화는 실제 가청·BGA·IR 서버 확인을 통과했다는 뜻이 아니며, 해당 확인은 사용자 환경에서 수행한다.

Phase R의 prod 배포, 태그와 서명은 실제 키가 필요한 작업이다. 사용자가 실제 키를 준비한 뒤 함께 진행하며, 이 단계에서는 착수하지 않았다.
