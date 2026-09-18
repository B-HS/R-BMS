# 스킨 시스템 완성 검증 체크리스트 (2026-09-17)

사양: `docs/plan/2026-09-17-skin-system-completion.md` §11. 각 항목은 실행 명령·기대 결과·확인 상태를 적는다. 체크는 실제 실행 뒤에만 표시한다.

## 1. 자동 검사

- [x] 로더: `cargo test -p rbms-skin` — 중첩 트랙(note group/judge/songlist) 조립, hotspot 검증, scope 헤더 노출 테스트 통과.
- [x] 렌더 객체: `cargo test -p rbms-render skin_render` — note/gauge/judge/covers/songlist/graphs/color 픽셀 테스트 통과.
- [x] 설정·설치: `cargo test -p rbms-config` · `cargo test -p rbms-player --lib assets` — 공유 스코프 병합, 세대 목록 설치, v2→v3 이동 시 custom 키 이동, 사운드 폴더 결정.
- [x] 앱 배선: `cargo test -p rbms-player --lib skin` — FrameExtra 조립, 대체 단위 게이트(불완전 선언은 네이티브 유지), hotspot·songlist 히트 사각형, 타이머 드라이버.
- [x] 게이트(K5 게이트 실측 3,181 통과·0 실패): `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace`, `git diff --check`, 금지 명칭 grep 0건.

## 2. 헤드리스 장면 캡처

명령: `RBMS_SKIN_CAPTURE_DIR=<폴더> cargo test -p rbms-player the_current_default_bundle_keeps_information_and_chart_art_visible --lib` 와 `cargo test -p rbms-player render_tests_skin_v3 --lib`(35건). 저장본: `captures/` (select·options·decide·play-5k/7k/9k/10k/14k·result·v3-play-7k-2p·v3-play-7k-near·v3-select-records).

- [x] `select.png` — 목록 오른쪽 15슬롯, 선택 행 center 슬롯, 상세 왼쪽, 장식이 글자 위를 지나지 않음.
- [x] `options.png` — 옵션 패널이 왼쪽 상세 영역 안에 있고 목록·제목이 보임, 11행 라벨·값·포커스.
- [x] `decide.png` — 배경·제목·아티스트·레벨·진행 바.
- [x] `play-5k.png` / `play-7k.png` / `play-9k.png` — 노트 필드가 문서 레인 사각형과 일치, 게이지가 필드 밖, BGA 프로브 픽셀 보존, 판정 카운터·그래프 열 분리.
- [x] `play-10k.png` / `play-14k.png` — 양측 필드, 중앙 게이지, 좌 BGA 2슬롯, 우 그래프 열.
- [x] `result.png` — 등급·점수표·판정·3그래프·그래프 열, 네이티브 중복 글자 없음.

헤드리스 캡처 육안 판정(메인): 장식선이 글자·노트를 가로지르는 곳 없음, 옵션 패널은 상세 열 안에서 열리고 목록·제목이 보임, 5/7/9키 필드·게이지·키 위젯·카운터·그래프 열이 분리됨, 2P 미러(v3-play-7k-2p)에서 그래프 열이 왼쪽·필드가 오른쪽, 결과 표·그래프 틀·그래프 열이 겹치지 않음. 남은 미세 결함: 플레이 상단 NOTES 수치가 패널 오른쪽 경계에 1~2px 물림(shared/objects-play.json5 의 x 조정 대상).

## 3. 실제 창 확인 (GPU)

> 메인이 시도했으나 이 터미널에 macOS 화면 녹화 권한이 없어 `screencapture`가 `could not create image from display`로 실패했다(앱 기동·라이브러리 스캔은 정상). 사용자 절차: 시스템 설정 → 개인정보 보호 및 보안 → 화면 기록에서 사용 중인 터미널 앱을 허용한 뒤 아래를 실행한다.
>
> ```
> # 사용자 설정을 건드리지 않는 격리 HOME 으로 실행
> mkdir -p /tmp/rbms-live && HOME=/tmp/rbms-live ./target/release/rbms-player "<곡 폴더>"
> # 선택(F1 옵션) → Enter 결정/플레이 → Esc, 결과는 차트 경로 + --auto 로 끝까지 재생
> ```


- [ ] 앱 실행 → 곡 선택 → F1 옵션 → 곡 결정 → 7키 플레이 → 결과까지 화면 캡처 5장을 이 폴더의 `captures/live-{select,options,decide,play-7k,result}.png` 로 저장(`2026-09-17-default-skin/` 은 v2 세대의 옛 캡처).
- [ ] 곡 목록 마우스 클릭이 songlist 슬롯과 일치(클릭한 행이 선택됨).
- [ ] SKIN 탭에서 번들 옵션(PLAY SIDE 등) 변경 → 5개 플레이 문서가 함께 바뀜, 재시작 후 유지.
- [ ] 시스템 사운드가 번들 `sound/`에서 재생됨(옵션 열기/닫기·결정·클리어).

## 4. 남은 위험 / 테스트 부채

- 실제 GPU 창의 입력 지연·오디오 동기는 기존 Phase B 소크 결과에 의존하며 이번 변경으로 재측정하지 않는다.
- 24키 문서는 준비 파일이며 캡처 대상이 아니다.
