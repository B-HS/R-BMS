# 스킨 후속 기능 검증 체크리스트 (2026-09-18)

사양: `docs/plan/2026-09-18-skin-followups.md` §4. 체크는 실제 실행 뒤에만 표시한다. 캡처는 `captures/`(1280×720 CpuCanvas 헤드리스).

## 1. 자동 검사 (L2 단위별, 각 워크트리에서 실측)

- [x] A 번들 결함: `cargo test -p rbms-render skin_render`(71) · `cargo test -p rbms-player --lib skin`(82) · `--lib assets`(14) · `--lib select`(115) 통과. 캡처 픽셀 측정: 10키 프레임 세로선 x 334/335·584/585·708/709·958/959(필드 판 336..584, 710..958 바깥 2px), NOTES 최우측 점등 x 989(테두리 1000).
- [x] B BGM 루프: `cargo test -p rbms-audio`(194, 루프 6건) · `cargo test -p rbms-player --lib`(914, 개별 target) 통과.
- [x] C 문서 이벤트: `cargo test -p rbms-skin`(신규 4) · `cargo test -p rbms-render skin_render`(83, 이벤트 13) · `cargo test -p rbms-player --lib skin`(82) · `--lib select`(115) 통과.
- [x] D 미리보기·연습·마우스: `cargo test -p rbms-render skin_render`(78) · `cargo test -p rbms-player --lib`(921) 통과, 캡처 `practice-7k.png`·`settings-skin.png`.
- [x] E 24키: `cargo test -p rbms-model`(66) · `-p rbms-judge`(295) · `-p rbms-render skin`(112) · `-p rbms-player --lib keyconfig`(44) · `--lib skin`(85) 통과, 캡처 `play-24k.png`. `format.rs` 24키 라벨/색은 머지 뒤 메인이 보정(`75fa520`).

## 2. 통합 게이트 (머지된 브랜치에서 메인 실측)

- [ ] `cargo fmt --all --check`
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- [ ] `cargo test --workspace` 0 실패
- [ ] 금지 명칭 grep 0건, `git diff --check`
- [ ] 헤드리스 캡처 재생성(`the_current_default_bundle_keeps_information_and_chart_art_visible`, `render_tests_skin_v3`) 육안 판정

## 3. 헤드리스 캡처 육안 판정 (메인)

- [x] `play-10k.png` — 프레임이 10키 필드와 일치.
- [x] `play-7k.png` — NOTES 수치가 패널 안.
- [x] `select-records.png` — RECORDS 패널에 판정 6행.
- [x] `practice-7k.png` — 7K 문서가 연습 12행을 그림(플레이 크롬 0 이 뒤에 보임 — 문서 제약, §2.9).
- [x] `settings-skin.png` — SKIN 탭 우측에 미리보기 축소판.
- [x] `play-24k.png` — 26레인 건반 패턴, 양끝 스크래치, 게이지·카운터·그래프 열.

## 4. 실제 창 확인 (GPU, 사용자 권한 필요)

- [ ] 곡 선택 BGM 이 끊김 없이 반복되고 곡 결정 시 멈춘 뒤 `decide` 가 1회 난다.
- [ ] 옵션 패널 행을 마우스로 클릭해 값이 좌/우로 바뀐다.
- [ ] 24키 bmson 차트가 목록에 보이고 플레이된다.

## 5. 남은 위험 / 테스트 부채

- 캡처 비결정성(기존): `play-9k`·`play-14k` 게이지 숫자 영역과 `result.png` 는 런마다 다르다(캡처 바이트 비교로 회귀를 판단하지 않는다).
- 문서 `skinpreview` 재귀 가드는 렌더러 단위 테스트만 있다(번들 문서가 이 객체를 쓰지 않아 앱 수준 픽스처 없음).
