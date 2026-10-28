# 배치 지정·선곡 정보창·밀도·음량·배속 설계 (2026-10-29)

최종 갱신 2026-10-29 · 대응 커밋 `c1dc988`. 요청 원문과 체크리스트는 [PROCESS.md](../PROCESS.md) "배치 지정·선곡 정보창·밀도·음량·배속". 조사 보고서는 [docs/reference/2026-10-29-feature-upgrade/](../reference/2026-10-29-feature-upgrade/) 의 `layout-pick.md`, `select-info-density.md`, `select-volume.md`, `play-rate.md`.

## 1. 결정

사용자 답(2026-10-29):

| 번호 | 결정 |
| --- | --- |
| D1 | 직접 정한 배치의 판은 RANDOM 판으로 저장한다. 그 배치를 내는 RANDOM 시드를 역탐색해 옵션 RANDOM + 시드로 기록한다. 서버·웹·리플레이 형식은 바꾸지 않는다 |
| D2 | 배속이 100% 가 아닌 판은 기록을 저장하지 않는다(로컬 DB·리플레이·trace·서버 전송·동기화 전부). "사람이 친 판은 전부 저장" 정책의 사용자 명시 예외다 |
| D3 | 배속은 재생 속도 자체를 바꿔 피치도 함께 변한다. 50~200%, 1% 단위, 기본 100% |
| D4 | 배치 미리보기는 그 배치를 적용한 실제 노트를 레인에 그려 스크롤하고 레인 번호 순서를 보인다 |
| D5 | NPS 는 beatoraja 밀도 값(정수 id 364/365 평균, 360/361 최고)을 그대로 쓴다. LN 몸통이 걸친 초마다 1개씩 센다 |
| D6 | 지정한 배치 문자열은 앱이 켜져 있는 동안 모드별로 기억한다(설정 파일에 저장하지 않음) |

D5·D6 은 사용자 답 "2번으로"를 두 질문 모두 2번 선택지로 읽은 것이다.

메인이 추천안으로 정한 것(사용자 미확인, HANDOFF 5절 "답이 없는 결정은 추천안"):

| 번호 | 결정 |
| --- | --- |
| M1 | 배치 지정은 선곡 위에 여는 새 내장 화면(연습 패널과 같은 층)이다. 새 단축키로 연다. 커서가 차트 막대일 때만 열린다 |
| M2 | 배치 문자열은 "왼쪽 i 번째 레인에 오는 원래 레인 번호"(beatoraja 표시와 같음). OFF·MIRROR 와 같은 모양(1234567, 7654321)도 D1 대로 RANDOM + 시드로 기록한다 |
| M3 | DP 는 양쪽을 모두 지정한다(Tab 으로 쪽 전환, 기본은 1234567). 10키에서 RANDOM 으로 만들 수 없는 쌍과 24키는 거절하고 이유를 보인다 |
| M4 | AUTOPLAY 가 켜져 있으면 미리보기는 되고 시작은 막고 안내한다. 코스·연습에서는 열지 않는다 |
| M5 | 배속 값은 설정(PlayOptions)에 저장한다. PLAY 메뉴 CHART 묶음과 F1 오버레이에서 바꾼다. 플레이 중 키는 없다 |
| M6 | 코스와 리플레이 감상은 100% 로 강제하고 코스 시작 때 알린다. 연습은 자기 FREQUENCY 를 쓴다 |
| M7 | 배속 판의 판정 창은 실시간 길이, 녹색 숫자·KEEP GREEN NUMBER·BPM 표시는 실시간 기준(BPM x 배율), AUTO ADJUST 는 그대로 동작 |
| M8 | 연습 경로의 기존 결함 후보 A(판정 그래프 커서 이중 배율)와 B(리듬 타이머가 배율을 모름)를 같은 경로이므로 함께 고친다 |
| M9 | 배속 판의 결과 화면은 NEW RECORD 를 보이지 않고 "기록 저장 안 됨(배속 n%)" 줄과 IR SKIPPED 사유를 보인다 |
| M10 | 선곡 정보창 넘침은 엔진의 새 overflow 값 3(하한 비율 0.75 까지 가로 축소 뒤 넘치면 잘라 `…`)으로 고치고, 정보창과 목록 막대 제목에 쓴다. 영어 상수(EX SCORE 등)는 용어집대로 원어 유지 |
| M11 | 선곡 음량: 기본 스킨 MASTER 슬라이더(rate 17)와 NUMBER 57 을 진짜 마스터 `audio.master` 로 다시 묶는다(레퍼런스 id 뜻과 다름을 문서화). 자동 재생 미리듣기는 키음을 Key 버스, BGM 을 Bg 버스로 나눈다. 선형 이득 곡선은 유지 |

## 2. 단위와 파일 소유권

| 단위 | 내용 | 주 파일(소유) | 모델 |
| --- | --- | --- | --- |
| U1 배치 지정 | 시드 역탐색, 새 내장 화면, 미리보기, 시작 | `crates/rbms-chart/src/shuffle.rs`, `apps/rbms-player/src/stage/layout_pick*`(새), `stage/mod.rs`, `stage/select/mod.rs`, `keyconfig*`, `stage/select/guide.rs`, `text/menus/layout_pick.rs`(새), `text/menus.rs` | Opus high |
| U2 배속 | 설정·시계·판정·오디오 배율, 기록 차단, 결과 표시, PLAY 메뉴·F1 | `practice.rs`, `play_sink.rs`, `stage/play/*`, `app_play.rs`, `app_result*`, `stage/result*`, `ir_outcome.rs`, `text/settings/*`, `text/menus/ir_outcome.rs`, `app_options.rs`, `stage/play_menu/*`, `skin_host/play_timers.rs`, `notes_dist.rs`, `rbms-config`, `contract/client-requests/settings-blob.json`, 스킨 `i18n/{play,result}.lua` | Opus high |
| U3 정보창·밀도 | 정보창 재배치, overflow 3, NPS 표시, 캡처 | `assets/skins/rbms-default/lua/select/*`, `theme.lua`, `props.lua`, `parts.lua`, `i18n/{select_info,common}.lua`, `crates/rbms-render/src/font/*`, `skin_render/text*`, `apps/rbms-cli/scenarios/select.json` | Sonnet xhigh |
| U4 음량 | rate 17·NUMBER 57 재결합, 미리듣기 버스 분리 | `skin_host/{writers,system}*`, `skin_screen.rs`, `skin_screen/preview.rs`, `stage/select/preview.rs`, `crates/rbms-skin/src/property/host.rs`, `i18n/select_other.lua` | Sonnet xhigh |

공유 지점: `settings_ui.rs`(U2·U4), `text/menus.rs`(U1·U2), 선곡 기본 스킨의 배속 표지(U2 의 확장 id 를 U3 영역에 그림). 네 단위는 worktree 로 병렬 실행하고 메인이 `git apply` 로 합친다. 선곡 화면의 배속 표지는 병합 뒤 메인이 붙인다. 문서(`play-options.md`, `skin.md`, ADR, QA)는 단위 보고를 받아 메인이 쓴다.

## 3. 검증

- 단위: 자기 크레이트의 좁은 `cargo test -p <crate> --locked <필터>`, `cargo clippy -p <crate> --all-targets --locked -- -D warnings`, `cargo fmt --all --check`.
- 통합(메인): `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`, `cargo test --workspace --locked`, 기본 스킨 캡처 1장, 푸시 뒤 3 OS ci.
- 실창 확인(배치 화면 키 입력, 배속 소리, 선곡 음량 체감)은 QA 실기 점검표로 사용자에게 넘긴다.
