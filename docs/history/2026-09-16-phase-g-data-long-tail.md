# Phase G — 데이터 스케일과 롱테일 배선 (2026-09-16)

## 대상

- CLI 및 라이브러리: `apps/rbms-cli/src/main.rs`, `crates/rbms-library/src/scan.rs`
- 코스 및 연습: `crates/rbms-judge/src/gauge.rs`, `crates/rbms-play/src/{lib.rs,session.rs}`, `apps/rbms-player/src/{app_play.rs,practice.rs,play_sink.rs,stage/practice.rs,stage/play/mod.rs}`
- IR 및 선택 화면: `apps/rbms-player/src/{app_network.rs,app_ranking.rs,ir_ext.rs,ir_ranking.rs,ir_ranking_view.rs,stage/select/mod.rs}`
- 회귀 테스트: `apps/rbms-player/src/{practice/tests.rs,stage/play/tests.rs,stage/select/tests.rs,ir_ext/tests.rs}`

## 완료한 배선

| 항목 | 결과 |
|---|---|
| G1·G9 | CLI 스캔을 SQLite 증분 경로로 전환하고 bmson 차트 요약을 포함했다. |
| G3 | 코스 스테이지 사이에 gauge와 standing combo를 보존한다. |
| G4 | 연습 시작 gauge, 50~200% 시간축, 판정·키음 스케줄·오디오 피치를 같은 축으로 연결했다. 구간 경계의 LN은 경계에 맞춰 클리핑하고, 시작 시점 BGA를 보이며 이미지 소유권은 플레이 종료 후 복원한다. |
| G5 | 테스트 전용 접근자를 `cfg(test)`로 한정하고, 더는 쓰이지 않는 선행 추상화와 dead-code 허용을 제거했다. |
| G7 | extra-only IR fan-out도 완료 상태로 수렴한다. 랭킹과 리플레이는 primary를 사용하며, primary 변경 또는 서버 재구성 시 랭킹 캐시와 진행 중 요청을 세대 증가로 무효화한다. |

## 검증

| 명령 또는 점검 | 실제 결과 |
|---|---|
| `cargo fmt --all --check` | exit 0 |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | exit 0 |
| `cargo test --workspace` | exit 0, 실패 0; 테스트 목록 3,037개 등록; 실제 오디오 장치 테스트 2건 ignored |
| `cargo test -p rbms-player` | 단위 839건과 통합 2건, 실패 0 |
| `cargo test -p rbms-cli` | 12건, 실패 0 |
| `git diff --check` | exit 0 |
| CLI 임시 폴더 스모크 | `charts=1`, `read=1` |
| 독립 적대 리뷰 | 최종 blocker 0 |

## 사용자 수동 확인

- 실제 GUI에서 큰 BGA를 반복해 연습할 때 메모리 사용량이 안정적인지 확인합니다.
- 연습 구간의 시작·종료 경계에서 LN과 BGA가 의도대로 표시되고 키음이 자연스럽게 들리는지 확인합니다.
- 실제 복수 IR 서버에서 primary 전환, 랭킹 갱신, 리플레이 다운로드를 확인합니다.
- 코스 진행 중 gauge와 combo 이월 체감, 실제 오디오 장치 두 건의 가청 확인은 자동화 범위 밖입니다.
