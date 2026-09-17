# 기본 스킨 실화면 보정 결과

기준 사양: `docs/plan/2026-09-17-skin-visual-correction.md`. 사용자 실행 캡처에서 확인된 잘못된 좌우 배치, 반투명 옵션, 재생 테두리의 노트·그래프 침범을 재현한 뒤 수정했다.

## 구현

- 문서·테마·플레이 RON을 새 버전 번들로 함께 설치한다. 최초 설치는 새 버전만 배포하고, 기존 설치의 원본 문서와 일치하는 선택만 새 경로로 옮긴다. 편집 문서와 외부 경로는 설정과 파일을 유지한다. 다른 프리셋에서 좌표가 맞지 않는 번들 문서는 렌더하지 않는다.
- 화면 문서의 `layered` 합성은 배경·내장 UI·안전한 전경 순서다. 재생 BGA와 선택 커버는 문서 배경을 그린 뒤 해당 파일 배치 좌표에 표시한다. 기존 `replace`·`overlay` 문서와 기본 프리셋은 종전 순서를 유지한다.
- 새 테마는 선택 상세/목록 위치와 패널 투명도, 옵션 창의 중앙 좌표·불투명도를 파일로 선언한다. 단일·이중 플레이 RON은 필드/BGA를 분리하고, BGA·비교 그래프 테두리는 실제 배치에서 계산한다.
- 옵션 11행의 이름·값의 x, 행 y, 이름·값 색은 테마 RON의 `options_layout.row_overrides`에서 행별로 바꿀 수 있다. 설정 값과 키 조작은 기존 descriptor를 쓴다.
- `assets/skins/steel-neon-v2/`에 각 화면 문서·RON·팔레트·생성 스크립트·PNG를 두었다. 그림은 배경과 화면 외곽만 담당하고 노트·판정·수치 위치를 그림에 고정하지 않는다.
- 결과의 점수·클리어·6종 판정·목표 텍스트는 JSON5의 독립 객체로 옮겼다. 필수 객체가 컴파일되지 않으면 한 번 경고하고 해당 네이티브 묶음을 숨기지 않는다.

## 실화면 판정

`RBMS_SKIN_CAPTURE_DIR=/private/tmp/rbms-skin-v2-captures cargo test -p rbms-player the_current_default_bundle_keeps_information_and_chart_art_visible --lib`로 1280×720 픽셀 캡처를 만들고 직접 확인한 뒤 `docs/quality-assurance/2026-09-17-default-skin/`에 복사했다. 저장된 자료는 `select.png`, `options.png`, `play-5k.png`, `play-7k.png`, `play-9k.png`, `play-10k.png`, `play-14k.png`, `result.png`다.

| 장면 | 판정 |
| --- | --- |
| 곡 선택 | 상세 왼쪽·5개 곡 목록 오른쪽, 별빛 배경은 정보 뒤에 있음. 우측 행 클릭 영역 확인 |
| 옵션 | 중앙의 불투명 패널 11행과 값이 배경 글자에 섞이지 않음 |
| 5·7·9키 | 필드·BGA·비교 그래프가 별도 영역이며 합성 배경이 차트 BGA 픽셀을 덮지 않음 |
| 10·14키 | 양쪽 필드, BGA, 비교 그래프가 분리됨. 첫 14키 캡처에서 BGA와 판정 수치가 겹쳐 RON 좌표를 수정한 뒤 재검사 통과 |
| 결과 | 점수·클리어·판정·목표의 개별 객체가 중복 네이티브 글자 없이 표시되고 하단 그래프가 읽힘 |

PNG의 진한 분홍색 직사각형은 테스트용 4×4 차트 BGA를 늘려 표시한 것이다. 실제 차트 영상의 내용이나 품질을 뜻하지 않는다. 이번 검증은 헤드리스 CPU 렌더의 픽셀과 배치에 대한 것으로, 실제 창의 GPU·입력 지연까지 검증한 증거는 아니다.

## 검사

- `cargo test -p rbms-player assets::tests::default_skin_installation_preserves_user_files_and_selections --lib` — 통과.
- `cargo test -p rbms-player assets::tests::unchanged_legacy_document_moves_to_a_complete_current_bundle --lib` — 통과.
- `cargo test -p rbms-player the_current_default_bundle_keeps_information_and_chart_art_visible --lib` — 통과, 선택/옵션/7키/14키/결과 출력 확인.
- `cargo test -p rbms-player stage::result::tests --no-fail-fast` — 5개 통과.
- `cargo check -p rbms-player` — 통과.
- `cargo test -p rbms-render theme::tests::an_options_row_can_be_moved_and_recolored_without_moving_its_neighbors --lib`와 `cargo test -p rbms-player a_theme_file_override_moves_one_option_row --lib` — 통과.
- `cargo test -p rbms-player -p rbms-render -p rbms-skin` — 통과.
- `cargo fmt --all --check`와 `cargo clippy -p rbms-player --all-targets -- -D warnings` — 통과.
- `git diff --check`와 `docs/`·코드·자산의 금지 명칭 검색 — 오류/검색 결과 0건.

## 남은 범위

이번 수정은 사용자 캡처에서 확인된 배치·가림 결함과 기본 배포를 고쳤고, 결과 텍스트와 옵션 행 위치/색의 파일 편집을 시작했다. 곡 행·상세 정보·재생 노트/판정/게이지/HUD, 결과 등급·그래프·힌트, 옵션 패널의 나머지 요소를 JSON5 객체로 각각 이동·교체하는 기능은 아직 구현되지 않았다. 원 사양의 “전부 파일로 커스터마이징” 완료를 뜻하지 않는다. 상세 기능 계약과 완료 조건은 `docs/plan/2026-09-17-skin-object-spec.md`에 기록했다. 24키 문서는 준비 파일이며 기본 선택 대상이 아니다.
