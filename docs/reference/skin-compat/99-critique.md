# 99. 종합 보고서(00-synthesis.md) 비판 — 틀린 주장, 빠진 기능, 작업 분해 결함, 고유 기능 구멍

> 최종 갱신 2026-10-10 · 대응 단계: 웨이브 2(Lua 런타임·로더·스킨 팩) 반영 · 본문은 기준 커밋 `9ce92bb` 시점 서술이며, 아래 "웨이브 … 반영 사항"이 최신 것부터 우선한다 · 색인과 갱신 규칙은 [README.md](README.md)

## 웨이브 2B 반영 사항 (2026-10-10)

스킨 덤프 CLI, 앱의 스킨 팩 폴더 지정과 `.luaskin` 로드, 오버레이 총 크기 상한, 외부 스킨 첫 정지 프레임을 넣은 뒤의 상태다.

- (W2-8) 99-critique.md §4 P5: '웨이브 3~6 에서 앱이 .luaskin 을 고를 방법이 없다' 는 W2-8 로 해소됨으로 표시
- (W2-6) 99-critique.md §4 P6 '프레임 비용 실측' 행과 §2 W9: 해소됨. 실측(조건 무시 상한): 선곡 1801호출 평균 0.278ms·p99 0.339ms·최대 0.653ms, 플레이 5/7키 75호출 평균 0.03ms, 10/14키 123호출 평균 0.05ms, 결과 118호출 0.025ms, 코스 119호출 0.040ms. 로드 한 패스 필요 명령 수 8천~36만, 호출당 약 2천 이하, 메모리 최대 6.1MiB. 같은 절의 '패턴 단계 환산 비율' 항목은 분리 측정되지 않았고 --probe-budget 총량에 포함된다.
- (리뷰 수정) 99-critique.md 상단 '웨이브 2B 반영 사항': M8(오버레이 상한)과 W9·P6(덤프 도구)의 후속 — 인터프리터별 계정 한계 해소(스레드 내 공유), 덤프의 자동 게이트 옵션(--strict / --allow-failure) 추가, 팩 무변경 검사가 크기+수정 시각으로 강화됨


## 웨이브 2A 반영 사항 (2026-10-10)

Lua 5.2 런타임(`crates/rbms-skin/src/lua/`), `SkinHost`, Lua 값 변환기, 2패스 `.luaskin` 로더를 넣고 구 샌드박스(`skin.*`)를 삭제한 뒤의 상태다.

- (W2-2b) 99-critique.md §3 M8 (가): 'LuaJ 의 줄 읽기는 \r 을 전부 버리지만' 을 바이트코드(IoLib.freaduntil)로 확인됨, lua/io.rs 가 read('*l')·lines 에서 재현해 해소됨으로 표시해야 합니다. '*L' 은 \r 과 \n 을 유지합니다.
- (W2-2c) 99 또는 r1 의 W2-2c 계약 서술('모르는 이름 -1', 'Controllers size 0')은 구현과 일치합니다. 'luajava HeaderOnly 설치 여부'가 문서에 없으므로 b1 §2 표의 샌드박스 열(luajava nil)과 R-BMS 구현(항상 설치)의 차이를 한 줄 추가해야 합니다.
- (리뷰 수정) 99-critique.md P6(프레임 비용·예산 설계) 관련 절: 프레임 벽시계가 호출 구간 누적으로 바뀌었고 패턴 작업이 명령 한도에 포함된다는 점을 반영해, 덤프 도구 실측 항목에 '패턴 단계 환산 비율(1단계=1명령, 8바이트=1명령)'을 넣어야 합니다


## 웨이브 1B 반영 사항 (2026-10-10)

타이머 µs, stretch 11종, 장면 시계, 마우스 이벤트, GPU 논리 크기 런타임화와 색 공간, START/SELECT, 해상도 설정, 캡처 하니스를 넣은 뒤의 상태다.

- (W1-4) 99-critique.md §2 W1 과 §4 P1 의 W1-4 행: 해소로 표시. 실제 호출부는 목록에 없던 `apps/rbms-player/src/app_options.rs`(패널 타이머 `switch_panel_timers`)가 하나 더 있었다고 덧붙인다
- (W1-10) 99-critique.md §0 항목 3, §2 W3, §4 P6 첫 행('START/SELECT 바인딩과 키 인덱스 0~8 상태 질의'), §4 P2 의 'W5-3 선행: START/SELECT 바인딩': 해소됨으로 표시하고 근거를 `keyconfig.rs`(`ControlAction`, `key_index_of`, `HeldKeys`), `app_input.rs`(질의), `gamepad.rs`(`PadMapper::held`, `StandardButton`)로 바꿉니다. 기존 '`keyconfig.rs:199-208` 에 START/SELECT 없음' 줄 번호는 낡았습니다.
- (W1-9) 99-critique.md §4 P7 1번: '새 하니스를 만드는 단위는 W7-6 뿐'과 상단 반영 사항의 'W1-9 전까지 RBMS_SKIN_CAPTURE_DIR 를 읽는 코드가 없음'은 해소됨. apps/rbms-player/src/stage/capture.rs 가 RBMS_SKIN_CAPTURE_DIR 를 읽고 Shot::take 로 임의 해상도 HeadlessCanvas 프레임을 <이름>.png 로 저장한다고 바꿔야 함.
- (W1-9) 99-critique.md §4 P7 2번: '캡처는 1280x720 CpuCanvas 픽셀이고 GPU 창과의 동일성은 미검증, GPU 읽기는 코드에 없다'는 낡음. Shot::take_on_gpu 가 Gpu::offscreen + Gpu::capture 로 같은 프레임을 <이름>.gpu.png 로 저장하고, 내장 선곡 화면은 1280x720·1920x1080 에서 CPU 프레임과 채널 차 2 초과 픽셀 1% 이하로 단언된다고 바꿔야 함(실측 0.035%, 0.42%). 실제 창 표면의 동일성은 여전히 사용자 절차.
- (W1-9) 99-critique.md §4 P7 3번과 수정 제안 첫 항목: 웨이브 1 에 눈으로 볼 결과가 생겼음(select-1280x720, select-1920x1080 의 .png 와 .gpu.png). 외부 스킨 선택 테스트는 RBMS_SKIN_PACK 으로 구현됐고 현재 JSON/JSON5 문서만 대상(.luaskin 은 웨이브 2)이라고 적어야 함.
- (W1-9) 99-critique.md §0 판정 요약 6번: '검증 수단에 구멍'은 웨이브 1B 의 W1-9 로 메워졌다고 표시해야 함.
- (리뷰 수정) 99-critique.md P7(캡처 하니스): 헬퍼 구성이 바뀌었습니다. 어댑터 없음 처리는 `Gpu::offscreen_if_available` 과 `gpu::REQUIRE_GPU_ENV`, 프레임 그리기는 `render_tests::render_on`, 문서 컴파일 대기는 `capture::draw_until_compiled` 하나이며 `play/tests.rs` 도 이를 씁니다. 캡처 이름 `document-unresized-1920x1080` 이 추가됐습니다.


## 웨이브 1A 반영 사항 (2026-10-09)

혼합 합성과 구 번들을 삭제한 뒤의 상태다. 본문의 해당 절은 아래 내용으로 읽는다.

- (W1-1b) 99-critique.md §4 P1 의 W1-1 행: lib.rs 재노출, skin_render/draw.rs 의 Body::Density, skin_render/tests*.rs, stage/mod.rs 선언, skin_select/tests.rs, 5개 테스트 파일은 처리 완료. 남은 것은 rbms-config(STEEL_NEON_SKIN, shared_customise, default_skin_installed)와 stage/settings.rs·settings_ui.rs·settings/skin_tests.rs·main_tests.rs·skin_select/fixtures.rs 의 번들·프리셋 참조(W1-1a). §4 P7 1번: 캡처 테스트 5개는 삭제됐고 W1-9 전까지 RBMS_SKIN_CAPTURE_DIR 를 읽는 코드가 없음.
- (W1-1a) 99-critique.md §4 P1 의 W1-1 행 중 설정·자산·번들 참조 부분(rbms-config lib.rs/settings.rs/tests.rs, stage/settings.rs, skin_tests.rs, skin_select tests/fixtures, main_tests.rs)과 P6 의 '설정 마이그레이션 테스트' 행: 해소로 표시(검증 파일 crates/rbms-config/src/tests.rs:503, 568, 605).
- (W1-1a) 99-critique.md 에 새 항목 추가 권장: 계정 설정 블롭 경로(apps/rbms-player/src/ir_sync.rs parse_blob)가 rbms_config::migrate 를 거치지 않아 스키마 2 이하 블롭에 구 번들 마이그레이션이 적용되지 않음.


작성일 2026-10-09. 읽기 전용 검증. 세 대상 디렉터리는 수정하지 않았고 cargo 는 실행하지 않았다. 유일한 쓰기는 이 파일과 스크래치의 집계 스크립트 2개(`scratchpad/critique-tools/ids.py`, `casecheck.py`)다.

표기
- `R:` = `/Users/hyunseokbyun/development/R-BMS/`, `B:` = `/Users/hyunseokbyun/development/beatoraja/src/bms/player/beatoraja/`, `M:` = `/Users/hyunseokbyun/Downloads/ModernChic/`.
- "종합" = `00-synthesis.md`. `[종합 4절 W3-1]` 처럼 적은 것은 종합의 위치다.
- 심각도: 높음 = 그대로 두면 구현 단위가 게이트를 통과하지 못하거나 사용자 기능이 사라진다. 중간 = 사양이 틀리거나 비어 있어 재작업이 생긴다. 낮음 = 수치·표현 오류.

---

## 0. 판정 요약

1. 종합의 격차 매트릭스 중 R-BMS 현황 열은 직접 연 26개 행에서 전부 사실이었다(6절). 레퍼런스 의미론 서술도 대조한 19개 항목에서 식·분기가 원본과 일치했다(7절). 이 두 축에서는 틀린 사양을 찾지 못했다.
2. 틀린 것은 계획 쪽에 몰려 있다. 종합 7.2절 위험 9 의 "호출부까지 같은 단위가 소유하도록 나눴다"는 사실이 아니다. W1-1, W1-4, W2-1, W2-4, W3-1, W6-4 의 소유 파일 목록에 실제 호출부·선언부가 빠져 있어, 목록만 지키면 `cargo test --workspace` 게이트를 통과할 수 없다(4절 P1~P4).
3. "미확인"으로 남긴 것 중 둘은 원본에서 바로 답이 나온다: 게이지 이력은 전 종류를 500ms 간격으로 동시에 기록하고(B:play/BMSPlayer.java:631-635), R-BMS 에는 START/SELECT 바인딩이 없다(R:apps/rbms-player/src/keyconfig.rs:199-208). 후자는 패널 1~3, 결정 취소, 차트 미리보기, 폐점 즉시 재시작의 전제라서 작업 단위가 하나 통째로 빠져 있는 셈이다(2절 W2·W3, 4절 P6).
4. 매트릭스에 없는 기능은 9건(3절). 가장 큰 것은 PRELOAD 중 차트 미리보기(타이머 141, ModernChic 8곳 참조), 원형 곡 목록과 빈 목록, TTF 누락 글리프 처리(사용자의 기존 "모든 언어" 요구와 충돌)다.
5. 스킨 단독 소유로 바꿀 때 사라지거나 길을 잃는 R-BMS 고유 기능은 15건(5절). 종합 6절 결정 5 와 결정 10 의 추천안이 서로 모순되어 IR 랭킹 패널이 갈 곳이 없고, 결정 12 와 W7-5/W8-5 의 순서가 모순되어 9키가 플레이 불가가 된다.
6. 검증 수단에 구멍이 있다. 현재의 헤드리스 스킨 캡처 통로(`RBMS_SKIN_CAPTURE_DIR`)는 W1-1 이 삭제하는 테스트 파일 5개에만 있고, 새 캡처 하니스를 만드는 단위가 웨이브 7 까지 없다. 웨이브 3~6 의 "캡처" 검증은 그 전에 성립하지 않는다(4절 P7).

---

## 1. 검증 범위

| 절차 | 한 일 | 분량 |
|---|---|---|
| 1. R-BMS 현황 대조 | 종합 매트릭스의 "있음"·"부분" 행을 R-BMS 원본으로 확인 | 26행(6절 표) |
| 2. 레퍼런스 의미론 대조 | 보간·블렌드·필터·숫자 시트·비트맵 폰트·songlist·게이지·상태기계·Lua 래퍼 등을 beatoraja 원본과 대조 | 19항목(7절 표) |
| 3. ModernChic 전수 grep | `main_state.*`, 표준 라이브러리, 전역 함수, 메서드 호출, `skin.*` 키, 테이블 필드, id 표(NUM/OP/STRING/BUTTON/SLIDER/GRAPH/OFFSET/IMAGE/TIMER) 사용 집계, 리터럴 id, 함수 값 필드 종류, 파일·네트워크 접근, 경로 대소문자 | Lua 126 + luaskin 10 전부(8절) |
| 4. 작업 분해 | 소유 파일 목록을 실제 사용처 grep 과 대조, 선행 관계·검증 수단 점검 | 웨이브 1~7 전 단위 |
| 5. 고유 기능 구멍 | R-BMS 의 Stage·키·오버레이·설정 스키마를 열어 스킨 계약에 대응물이 있는지 확인 | Select/Play/Result/Loading/Practice/옵션/토스트/디버그 |

---

## 2. 틀렸거나 부정확한 주장

| # | 심각도 | 종합의 주장 | 사실 | 근거 | 수정 제안 |
|---|---|---|---|---|---|
| W1 | 높음 | [종합 7.2 위험 9] "크레이트 경계를 넘는 시그니처 변경은 호출부까지 같은 단위가 소유하도록 나눴다(W1-4, W2-1)" | 사실이 아니다. W1-4(타이머 µs) 소유 목록은 `skin_render/{draw.rs, screen.rs, state.rs}` 뿐인데 타이머 API 사용처는 `skin_render/{covers, gauge, judge, mod, notes, object, songlist, tests, tests_list_graphs, tests_play_objects}.rs`, `R:crates/rbms-render/src/lib.rs`, `R:crates/rbms-render/tests/skin_render.rs`, 앱의 `skin_screen.rs`·`skin_screen/tests.rs`·`stage/play/mod.rs`·`stage/play/tests.rs`·`stage/result.rs`·`stage/select/mod.rs`·`lib.rs`, `R:crates/rbms-skin/tests/{skin_integration,skin_lua}.rs` 에 있다. W2-1(`PropertyRef` 확장)도 `loader/track.rs`(`PropertyRef::Id`/`Expr` 전수 match, R:crates/rbms-skin/src/loader/track.rs:209-222, 232-241)와 `skin_render/covers.rs` 가 목록에 없다 | `grep -rln "TimerState\|timers\.\(get\|set_on\|...\)\|now_ms"`, `grep -rln PropertyRef` 결과(이 조사에서 실행) | 4절 P1 의 보정 목록으로 소유 파일을 다시 쓴다. 또는 W1-4 를 "새 µs API 추가 + 구 ms API 를 얇은 래퍼로 유지"로 바꿔 호출부 수정을 뒤 단위로 미룬다 |
| W2 | 중간 | [종합 7.3] "게이지 이력 샘플 간격 … 전 종류 동시 기록 방식은 미확인" | 확인된다. PLAY 상태에서 매 프레임 `for i in gaugelog: if (gaugelog[i].size <= ptime / 500) gaugelog[i].add(gauge.getValue(i))` 로 전 게이지 종류를 500ms 간격(플레이 타이머 41 기준)으로 동시에 쌓는다. 폐점 시에는 `TIMER_FAILED - TIMER_PLAY` 부터 `playtime + 500` 까지 500ms 마다 0 을 채운다 | B:play/BMSPlayer.java:625, 631-635, 723-729 | T3·W4-3 의 사양에 그대로 적는다. "미확인" 항목에서 뺀다 |
| W3 | 높음 | [종합 2.8, 7.3] "START/SELECT 바인딩 유무는 미확인", 붙일 곳으로 "`SelectState::handle_pad`" | R-BMS 에 START/SELECT 가 없다. `ControlAction` 은 HiSpeedUp/Down, CoverUp/Down, LiftUp/Down, HiddenUp/Down 8개뿐이고, 패드 이벤트는 `Lane`/`Control` 두 종류다. `SelectState` 는 `handle_pad` 를 구현하지 않는다(트레이트 기본 no-op). 즉 컨트롤러로 선곡 화면을 조작하는 경로 자체가 지금 없다 | R:apps/rbms-player/src/keyconfig.rs:199-220, R:apps/rbms-player/src/gamepad.rs:302-305, R:apps/rbms-player/src/stage/mod.rs:144-148, R:apps/rbms-player/src/stage/select/mod.rs(함수 목록 87-779 에 `handle_pad` 없음) | 4절 P6 의 신규 단위("START/SELECT 와 키 인덱스 0~8 질의")를 웨이브 3 앞에 둔다. 결정 4 의 선택지에 "START/SELECT 를 키 설정에 추가한다"는 전제를 명시한다 |
| W4 | 높음 | [종합 6절 결정 5 추천] "IR 랭킹은 (a) 스킨 객체로 대체" 와 [결정 10 추천] "IR 은 이번에 오프라인 고정값(랭킹 값 = 값 없음, 관련 옵션 false)" | 두 추천이 양립하지 않는다. 둘 다 따르면 현재 동작하는 IR 랭킹 패널(R:apps/rbms-player/src/stage/select/mod.rs:365-496, 856-859), 랭킹 리플레이 내려받기(같은 파일 450-538), 결과 화면의 IR 제출 상태 줄(R:apps/rbms-player/src/stage/result.rs:133-137)이 전부 표시 수단을 잃는다 | 왼쪽 열 | 결정 10 이 (a)인 동안에는 IR 랭킹 패널·IR 상태 줄을 시스템 오버레이로 유지한다고 결정 5 를 고친다. 스킨 쪽 IR 타이머 172~174 와 옵션은 실제 `ir_status` 에서 공급한다(오프라인 고정이 아니라) |
| W5 | 높음 | [종합 6절 결정 12] "9키 문서가 나오기 전에는 내장 플레이 화면을 지우지 않는다" 와 [4절] W7-5(내장 play/select/result/loading 처리, 선행 W7-4) → W8-5(9키 문서, 선택적 후속) | 순서가 모순이다. W7-5 는 W8-5 에 선행 관계가 없어, 결정 3 이 (b)이면 9키·24키가 플레이 불가가 된다. 또 W6-8 이 `R:crates/rbms-render/src/skin.rs` 의 내장 `Skin` 의존을 걷어 내는데 내장 플레이 렌더러는 그 `Skin` 으로 레이아웃을 잡는다(R:apps/rbms-player/src/stage/play/mod.rs:243-289, 570) | 왼쪽 열 | W7-5 의 선행에 "W8-5 완료 또는 결정 12 = (c)"를 추가한다. W6-8 의 범위를 "스킨 경로가 내장 `Skin` 을 읽지 않게 한다"로 한정하고 내장 렌더러용 `Skin` 은 건드리지 않는다고 적는다 |
| W6 | 낮음 | [종합 4절 원칙] 게이트 = `cargo fmt --check`, `cargo clippy --workspace -- -D warnings`, `cargo test --workspace` | 저장소 게이트는 `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace`, `git diff --check`, 금지 명칭 grep 0건이다. `--all-targets` 가 빠지면 테스트 코드의 clippy 실패를 놓친다 | R:docs/HANDOFF.md:19 | 게이트 문구를 HANDOFF 원문으로 교체한다 |
| W7 | 낮음 | [종합 0절 2, 2.3] "레퍼런스 43개 최상위 필드" | `JsonSkin.Skin` 의 최상위 필드는 46개다(스칼라 13 + 배열·객체 32 + destination). R-BMS `SkinDef` 는 46개를 전부 갖고 있으므로 결론("빠진 객체 종류 없음")은 맞다 | B:skin/json/JsonSkin.java:9-56, R:crates/rbms-skin/src/model.rs:402-464 | 수치만 46 으로 고친다 |
| W8 | 중간 | [종합 7.2 위험 3] "시각 일치를 판정할 레퍼런스 실행 화면이 없다. 조사는 전부 소스 읽기다" | 수단이 둘 있다. (가) 이전 세션에서 사용자가 참고 화면 11장을 대화에 첨부했고 "재사용이 필요하면 사용자에게 다시 요청한다"고 기록돼 있다. (나) 레퍼런스 체크아웃에 `build.xml`·`lib/`·`natives/`·`skin/default` 가 있어 사용자 기기에서 빌드·실행해 같은 곡·같은 설정의 캡처를 뜰 수 있다 | R:docs/HANDOFF.md:71, `/Users/hyunseokbyun/development/beatoraja/` 목록(build.xml, lib, natives, skin) | 사용자 결정 목록에 "레퍼런스 캡처를 (가) 재첨부 (나) 로컬 실행으로 확보 (다) 없이 좌표 대조만" 을 추가한다. 웨이브 3 착수 전 선행 조건으로 둔다 |
| W9 | 중간 | [종합 4절 W2-6 검증] "ModernChic 10개 실행. 기대: keyconfig 본체만 실패, 그 외 pcall 실패 0" 을 "고정값 호스트"로 | 고정값의 내용에 따라 결과가 달라진다. decide 본체는 option 150~155 중 하나가 true 여야 `CUSTOM.NUM.diffRGB()` 가 테이블을 돌려주고, 아니면 `RGB[1]` 에서 본체가 실패한다(M:decide.lua:52, 61, 145). 플레이 본체도 `main_state.option(NO_BGA)` 등으로 구조가 갈린다. 또 ModernChic 은 저장소 밖 경로라 `cargo test` 게이트에 넣을 수 없다 | M:decide.lua:52-61, 145; 종합 1.5 E2 자신의 서술 | 덤프 호스트를 "시나리오 파일(JSON)에서 id → 값을 읽는 호스트"로 정의하고, 화면별 기준 시나리오(곡 난이도 1개 true, BGA 있음/없음 등)를 사양에 표로 적는다. 저장소 게이트는 자작 미니 픽스처(W2-7)로, ModernChic 실행은 환경 변수로 경로를 받을 때만 도는 선택 테스트로 분리한다 |

부기: 종합이 "직접 확인"이라 적은 항목 중 다시 확인한 것은 모두 맞았다 — mlua 0.12.2 의 `lua52` 피처와 lua-src 551.0.2 의 `lua-5.2.4`(`$CARGO_HOME/registry/src/index.crates.io-1949cf8c6b5b557f/mlua-0.12.2/Cargo.toml:69`, `lua-src-551.0.2/` 목록), customTimers/customEvents 빈 테이블(M:musicselect.lua:55, M:result.lua:38-39, M:course.lua:38-39), 버전 확인 블록 주석(M:musicselect.lua:215-227), keyconfig 본체 실패 지점(M:keyconfig.lua:19 → M:Decide/lua/require/textproperty.lua:37), `Root/customfunction.lua` 최상위의 luajava 의존(M:Root/customfunction.lua:7-8), 방향키 폴링이 "아무 키"가 되는 facade(B:skin/lua/LegacySkinLuaApi.java:313-338 — `Input.Keys.valueOf("RIGHT")` 는 표시 이름 "Right" 와 달라 -1 = ANY_KEY).

---

## 3. 매트릭스에 빠진 기능

| # | 심각도 | 빠진 것 | 레퍼런스·ModernChic 근거 | R-BMS 현황 | 넣을 곳 |
|---|---|---|---|---|---|
| M1 | 중간 | PRELOAD 중 차트 미리보기. `config.isChartPreview()` 이면 START/SELECT 를 누른 프레임에 타이머 141 을 `now - 첫 노트 시각 + 1s` 로 켜고 레인 렌더러가 그 시계로 노트를 흘린다. 떼면 끄고 초기화. READY 전이 조건에 `now - startpressedtime > 1s` 가 있다 | B:play/BMSPlayer.java:481-503. ModernChic 이 타이머 141 을 8곳에서 쓴다: M:Play/lua/sp/lane.lua:23, 32, 36, M:Play/lua/dp/lane.lua:29, 31, 43, 48, M:Root/customoption.lua:183 ("Pattern Preview..." 문구 포함) | 타이머 141 구동 없음(R:crates/rbms-render/src/skin_render/screen.rs 에 없음). START 키 없음(W3) | 1.2절 P1 과 P7 에 행 추가. W6-3·W6-4 범위에 포함. 종합은 141 을 모순 판정 표(7.1-3)에만 적고 작업으로 잡지 않았다 |
| M2 | 중간 | 곡 목록이 원형이다. 슬롯 i 의 막대 = `(selectedindex + len*100 + i - center) % len`. 막대 수가 슬롯 수(ModernChic 17칸)보다 적으면 같은 막대가 여러 번 나오고, len == 0 이면 0 으로 나누기다 | B:select/BarRenderer.java:86-88, M:Select/lua/songlist.lua:8 (`center = 8`) | 목록이 비원형이다(위쪽은 `saturating_sub`, 아래쪽은 끝에서 멈춤, R:apps/rbms-player/src/stage/select/mod.rs:721-730). 빈 목록이 실제로 생긴다(첫 실행, 검색 결과 없음, 코스 없음 — R:apps/rbms-player/src/stage/select/scene.rs:187-193, 240) | 1.4절 S1~S3 에 "원형 인덱스"와 "빈 목록 처리" 행 추가. 빈 목록에서 songlist 를 그리지 않고 시스템 안내를 띄우는 규칙을 사양에 적는다(5절 H1) |
| M3 | 높음 | TTF 누락 글리프 처리. 이 레퍼런스 사본은 BMP 밖 문자와 누락 글리프를 fallback 폰트 → 대체 글리프 → 직접 그린 사각형 순으로 채운다 [b2 §12.3, b2:803]. ModernChic 의 TTF 는 Mgen+ 2종뿐이라 한글 제목은 레퍼런스에서 사각형으로 나온다 | M:Decide/lua/require/textproperty.lua:39-42 (TTF 2종), b2-object-model.md:803 | 사용자가 2026-05-31 에 "CJK 만이 아니라 모든 언어(한글 포함)" 지원을 요구했고 현 엔진은 그렇게 동작한다(R:docs/font-cjk-support.md "갱신된 요구사항" 절). `FontDef.fallback` 은 무시된다 [r2:303] | 종합 2.6절과 6절 결정 목록에 추가: "스킨 폰트에 없는 글자는 (a) 레퍼런스처럼 사각형 (b) 앱의 다국어 폰트로 대체". "똑같이"와 기존 요구가 충돌하므로 사용자 결정이 필요하다. 추천 (b), 결정 7 의 예외로 기록 |
| M4 | 중간 | 일반 `graph` 객체(SkinGraph) 행이 없다. ModernChic 은 `graph` 정의를 82곳에서 만들고 선곡 destination 중 111개가 graph 다 [m4 요약]. 레퍼런스는 값을 클램프하지 않는다(음수·1 초과가 그대로 폭·소스 영역에 곱해짐) | B:skin/SkinGraph.java:98-107, `grep "skin\.graph\|parts\.graph"` = 35 + 47 | 값을 0..1 로 클램프하고 0 이하면 그리지 않는다(R:crates/rbms-render/src/skin_render/draw.rs:340-345, 364-368). r2 도 이 차이를 적지 않았다. NaN 이 들어오면 `value <= 0.0` 이 거짓이라 NaN 크기 쿼드가 나간다(종합 S13 과 같은 뿌리) | 1.1절에 "graph/slider 기본 객체" 행 추가. W3-2 범위에 "클램프 제거 여부와 NaN 규칙"을 명시 |
| M5 | 낮음 | 결과 화면 gauge 의 차오름 애니메이션(`starttime`~`endtime` 동안 `min(value, max(max*(t-start)/(end-start), min))`)과, 원래 모드와 다른 모드로 플레이했을 때 `parts` 재계산 | B:play/SkinGauge.java:137-175 | 없음(결과 프레임에 gauge 자체가 없음, 종합 T4) | T4 에 식 추가. b3:385-386, 441 에 이미 있다 |
| M6 | 낮음 | songlist 의 그리기 순서가 막대별이 아니라 패스별이다: 전 막대 이미지 → 전 막대 분포 그래프 → 전 막대 제목 → 트로피 → 램프 → … 겹치는 막대에서 순서가 달라진다 | B:select/BarRenderer.java:255-330 | 해당 없음(재작성 대상) | S1 에 한 줄 추가. W5-1 지시의 금지 사항으로 넣는다 |
| M7 | 낮음 | 폐점(FAILED) 상태의 즉시 재시작: START xor SELECT 를 누르면 `close` 를 기다리지 않고 같은 곡을 다시 시작(START = 옵션 유지, SELECT = 같은 배치) | B:play/BMSPlayer.java:696-710 | 결과 화면의 R/N 키가 비슷한 역할(R:apps/rbms-player/src/stage/result.rs:106-108) | P1 에 추가하거나 "미구현으로 둔다"를 결정으로 기록 |
| M8 | 낮음 | LuaJ 와 PUC Lua 5.2 의 차이 목록에 두 가지가 더 있다. (가) LuaJ 의 줄 읽기는 `\r` 을 전부 버리지만 PUC 는 유지한다. (나) 큰 수의 문자열화가 다르다(배포본 `M:Select/lua/settings/checkversion` 의 `1.7200937E9` 가 흔적) | m1-root-config.md:458, `file` 결과(Lua 126개 전부 CRLF, `io/**` 데이터는 LF) | 해당 없음 | 종합 7.3 의 "LuaJ 고유 동작" 목록에 추가. 배포본 데이터는 LF 라 당장 영향은 없고, 사용자가 Windows 에서 쓰던 사본을 가져올 때만 드러난다. io 래퍼가 `lines()`/`read("*l")` 에서 `\r` 을 버리게 하면 해소된다 |
| M9 | 낮음 | prepare 주기 제한(`PrepareFramePerSecond`)과 `drawAllObjectsSafely`(스킨 미리보기 전용) | B:skin/Skin.java:265, 276-331, 333-359 | 해당 없음 | 구현하지 않는다고 명시만 한다(기본값에서는 매 프레임 prepare). 누락이 아니라 범위 밖 선언이 필요하다 |

grep 으로 확인했지만 종합이 이미 옳게 다룬 것(추가 조치 없음): `blend = 9` 1곳(M:Play/lua/sp/detailinfo/bgaareainfo.lua:650) — R-BMS 가 9 를 지원(R:crates/rbms-render/src/lib.rs:142-143). destination `center` 는 ModernChic 미사용(`songlist.center` 만). `event =` 필드 미사용. `act = MAIN.NUM.CLEAR`(370) 3곳은 종합 T10 이 다룸. `src = -105`(M:skinselect.lua:99)는 호출부가 주석(M:skinselect.lua:160). `MAIN.BUTTON.TARGET` 은 표에 없는 이름이라 런타임 nil(2곳). 리터럴 id `379 + i`, `389 + indexNum`, `119 + indexNum` 은 랭킹 값·문자열이며 b5 가 다룸. 경로 대소문자 불일치 0건(리터럴 경로·require 335개 검사).

---

## 4. 작업 분해의 문제

### P1. 소유 파일 목록이 실제 사용처를 덮지 못한다 (높음)

| 단위 | 목록에 없는데 반드시 고쳐야 하는 파일 | 근거 |
|---|---|---|
| W1-1 | `R:crates/rbms-render/src/lib.rs`(`pub mod content`, `FrameExtra`·`render_hud_with_content` 재노출, `SkinLayer` 사용), `skin_render/draw.rs`(`Body::Density` 분기 180행), `skin_render/{tests, tests_list_graphs, tests_play_objects}.rs`, `R:crates/rbms-config/src/{lib.rs:53, settings.rs:124·1447, tests.rs:63·239-374·532-546}`(`STEEL_NEON_SKIN`, `shared_customise`, `default_skin_installed`), `apps/rbms-player/src/stage/mod.rs:31-40`(삭제할 테스트 모듈 선언), `stage/settings.rs`·`settings_ui.rs`·`stage/settings/skin_tests.rs`·`skin_select/{tests,fixtures}.rs`·`skin_screen/tests.rs`·`main_tests.rs`(번들·프리셋 참조) | `grep -rln "composition\|SkinLayer\|hotspot"`, `grep -rn "steel\|STEEL\|default_skin_installed\|shared"` 결과 |
| W1-4 | 2절 W1 의 목록 전부 | 2절 W1 |
| W2-1 | `crates/rbms-skin/src/loader/track.rs`, `skin_render/covers.rs`, `crates/rbms-skin/tests/skin_model.rs` | `grep -rln PropertyRef` |
| W2-4 | `crates/rbms-skin/src/loader.rs`(`mod from_lua;` 선언이 여기 들어가는데 W2-5 소유) | Rust 모듈 규칙 |
| W3-1 | `state.rs` 삭제의 호출부: `apps/rbms-player/src/{skin_screen.rs, stage/loading.rs, stage/play/mod.rs:18}`, `R:crates/rbms-render/src/lib.rs`, `skin_render/{screen.rs, songlist.rs, tests*.rs}`, `R:crates/rbms-render/tests/skin_render.rs`. `FrameExtra` 재구성의 호출부: `stage/{play/mod.rs, result.rs:116-140, select/mod.rs:808-866}`. 이 중 `skin_screen.rs` 는 같은 웨이브의 W3-4 소유, `screen.rs` 는 W6-4 가 삭제 | `grep -rln "skin_render::state\|PlayViewState\|FrameExtra"` |
| W3-6 | `apps/rbms-player/src/lib.rs`(`mod skin_host;`) | Rust 모듈 규칙 |
| W4-5, W4-7, W5-4, W5-5, W6-4, W6-5 | `skin_host/mod.rs`(모듈 선언과 id → 군집 분배). W4-5 와 W4-7, W6-4 와 W6-5 는 같은 웨이브 병렬이라 같은 파일을 동시에 고친다 | 종합 2.4절의 군집 배치 |
| W5-5, W6-6, W6-7 | `skin_render/mod.rs`(신규 `text_input.rs`·`bga.rs` 선언), `skin_render/object.rs`(`Body` 열거형과 빌더 분기), `skin_render/draw.rs`(`draw_object` 의 match, 161-181행), `graphs/mod.rs` | R:crates/rbms-render/src/skin_render/draw.rs:161-181 |
| W6-4 | `screen.rs` 삭제 시 `R:crates/rbms-render/src/lib.rs` 재노출, `SelectTimers`(358-370)·`ResultTimers`(381-415)·`render_*_screen`(69-123) 의 이전처 | R:crates/rbms-render/src/skin_render/screen.rs |

수정 제안
- 뼈대 단위가 선언을 미리 깐다: W3-1 이 `Body` 변형과 `draw_object` 분기를 전부(bga, text_input 포함) 빈 구현으로 만들고, W3-6 이 `skin_host/mod.rs` 에 A~M 전 군집 모듈을 빈 파일로 선언하고 id 분배 표를 확정한다. 이후 단위는 자기 파일만 채운다.
- 시그니처를 바꾸는 단위(W1-4, W2-1, W3-1)는 "호출부 맞춤 전용 소유"를 별도 열로 적고, 같은 웨이브에서 그 파일을 소유하는 단위보다 먼저 직렬로 둔다.
- W3-1 은 `state.rs` 삭제를 하지 말고 "새 `SkinHost` 경로 추가"만 하고, 삭제는 앱 쪽 마지막 사용처가 사라지는 단위(W6-8)로 옮긴다.

### P2. 빠진 선행 관계 (높음)

| 단위 | 적힌 선행 | 실제로 필요한 선행 | 이유 |
|---|---|---|---|
| W3-7 (결정 화면 캡처 4장) | W3-6 | W3-2(참조 이미지 -100, 기본 객체), W3-3(TTF 텍스트), W3-4(텍스처 관리자 — `skin_screen.rs` 소유), W3-5(노트 분포·BPM 그래프), 그리고 P7 의 캡처 하니스 | 결정 화면 요구가 정확히 이 넷이다(종합 1.5 E2). 캡처가 검증 수단인데 그릴 것이 없다 |
| W4-5 (호스트 B·G) | 웨이브 3 | W4-3(엔진 기록 확장)의 API | early/late, 통과 노트 기준 값, 전 종류 게이지 이력을 읽는다 |
| W7-5 | W7-4 | W8-5 또는 결정 12 | 2절 W5 |
| 웨이브 3~6 전체 | - | 앱이 `.luaskin` 팩을 찾고 고르는 경로 | P5 |
| W5-3 (패널 1~3) | W5-2 | START/SELECT 바인딩 | 2절 W3 |
| W6-3 (플레이 상태기계) | 웨이브 5 | 연습·리플레이 분석과의 관계 결정(5절 H6·H7) | 같은 `PlayState` 를 공유 |

### P3. 한 단위가 너무 크다 (중간)

| 단위 | 범위 | 제안 분할 |
|---|---|---|
| W1-1 | 앱 10여 파일 + 렌더 10여 파일 + 설정 스키마 + 자산 삭제 + 테스트 파일 5개 삭제 + 앱 테스트 약 50건 무효화 | (가) 자산·번들 표·설치 로직 (나) 혼합 합성 소비자(layer/replace/hotspot/content) (다) 설정 스키마와 프리셋 (라) 테스트 정리. (나)가 끝날 때마다 내장 화면 골든 불변 확인 |
| W2-2 | 환경, 검색기, io 오버레이, os, luajava, print, 예산, 오류 로그 = 신규 파일 7개 | (가) 환경·require/dofile·예산·오류 로그 (나) io 오버레이·os (다) luajava facade. (가)만으로 헤더 패스가 돈다 |
| W3-1 | 2단계 prepare/draw + Lua 함수 평가 + 조건 의미론 + 커스텀 타이머·이벤트 + 능력별 프레임 데이터 + 어댑터 철거 + 모듈 골격 | (가) 프레임 데이터·호스트 연결 골격 (나) 조건 의미론(로더 `track.rs`) (다) Lua 함수 평가와 프레임 바인딩. ModernChic 은 customTimers/customEvents 가 빈 테이블이므로(종합 자신의 판정) 커스텀 타이머·이벤트는 웨이브 8 로 뺀다 |
| W5-3 | 선곡 입력 + 패널 3종 + 이벤트 표 + 장면 수명 | 패널·이벤트 표를 별도 단위로 |
| W6-4 | 타이머 드라이버 전체 + 오프셋 1~5 + `screen.rs` 삭제 | 삭제를 W6-8 로 |

W1-3(Lua 5.2 전환, `src/lua.rs` 손질)은 W2-2 가 `src/lua.rs` 를 삭제하고 새로 쓰므로 버려지는 작업이다. W2-2 에 합치거나, W1-3 을 `Cargo.toml` 피처 교체와 기존 테스트 통과 확인만으로 줄인다.

### P4. 전환 기간의 "내장 화면 폴백"이 Stage 재작성과 충돌한다 (높음)

종합은 웨이브 1~6 동안 내장 play/select/result 가 폴백이라고 하면서, 그 화면이 올라타 있는 Stage 로직을 W4-4(결과 수명·입력), W5-2/W5-3(선곡 상태 모델·입력), W6-3(플레이 상태기계)에서 다시 쓴다. 내장 렌더러는 지금의 `SelectView`(R:apps/rbms-player/src/stage/select/scene.rs), `HudView`, `ResultView` 를 먹는다. 소유 목록에는 내장 렌더러(`R:crates/rbms-render/src/{select.rs, hud.rs, playfield.rs, result.rs}`)가 없다.
- 결과: 폴백이 조용히 깨지거나, Stage 가 두 벌의 흐름(스킨용 PRELOAD/READY/close/fadeout 과 내장용 즉시 전환)을 갖게 된다. 어느 쪽인지 정해져 있지 않다.
- 수정 제안: 사양에 "스킨이 없을 때 Stage 는 헤더 시간값을 0 으로 본 같은 상태기계를 탄다(= 즉시 전환)"와 "뷰 모델은 새 호스트 모델에서 내장 뷰로 내려 변환한다"를 명시하고, 변환기 파일을 해당 단위 소유에 넣는다.

### P5. 웨이브 3~6 에서 앱이 `.luaskin` 을 고를 방법이 없다 (높음)

스킨 문서 확장자는 `["json", "json5"]` 고정이고(R:apps/rbms-player/src/skin_select.rs:38-39, 759-761), 이를 넓히는 단위는 W7-1 뿐이다. W1-1 은 같은 파일을 철거 목적으로만 소유한다. 따라서 웨이브 3(결정)·4(결과)·5(선곡)·6(플레이)의 "화면이 나온다"는 테스트 하니스 안에서만 참이고, 사용자의 실화면 확인(R:docs/feedback/2026-09-17-skin-visual-verification.md 가 요구)은 웨이브 7 전까지 불가능하다.
- 수정 제안: 웨이브 2 끝에 "스킨 팩 폴더 지정(설정 한 줄 + 환경 변수), 화면 타입 → `.luaskin` 매핑, 헤더 전용 읽기와 mtime 캐시"를 하는 작은 단위를 둔다. SKIN 탭 UI(W7-1)는 그대로 뒤에 둔다.

### P6. 빠진 작업 단위 (높음~중간)

| 빠진 단위 | 왜 필요한가 | 근거 |
|---|---|---|
| START/SELECT 바인딩과 "키 인덱스 0~8 상태" 질의(키보드·패드), 키 설정 UI 행 | 패널 1~3(START/SELECT/둘 다), 결정 건너뛰기(키 0/2/4/6)·취소(START+SELECT), 결과 키 0~6 배정, 차트 미리보기, 폐점 재시작 | B:decide/MusicDecide.java:54-64, B:play/BMSPlayer.java:481-503, 696-710; 2절 W3 |
| 스킨 화면 캡처 하니스 | P7 | P7 |
| 조사 문서 영속화와 단위별 갱신 | 사용자 지시 L1-b: 조사 보고서 전문과 색인을 `docs/reference/skin-compat/` 에 저장하고, L3~L5 의 각 구현 단위가 끝날 때 그 단위가 바꾼 코드를 다루는 절을 같은 단위 안에서 갱신한다. 종합의 문서 작업은 W7-7 하나뿐이다 | R:docs/PROCESS.md 현재 작업 절 L1-b |
| 금지 명칭 정리 | 조사 보고서와 종합은 레퍼런스 엔진명·외부 스킨명을 그대로 쓴다. 저장소 게이트에 금지 명칭 grep 이 있고 이전 세션은 같은 이유로 보고서를 저장소에 넣지 않았다. L1-b 와 정면으로 부딪힌다 | R:docs/HANDOFF.md:19, 70-71 |
| 프레임 비용 실측 | 선곡 기본 옵션이 프레임당 클로저 약 1380 + 타이머 함수 약 420(객체당 2회 호출, B:skin/property/TimerProperty.java:8-25)이다. 예산 설계(종합 2.2)가 틀렸는지는 웨이브 5 에 가서야 드러난다 | 종합 7.2 위험 5 |
| 설정 마이그레이션 테스트 | 스키마 버전 상승과 구 선택값 폐기(결정 13)의 검증 파일 `R:crates/rbms-config/src/tests.rs` 가 어느 단위에도 없다 | P1 |

수정 제안: 프레임 비용 실측은 W2-6 의 수용 기준에 "선곡 본체의 draw/timer/value 함수 전부를 고정 호스트로 1000 프레임 호출한 평균·최대 시간"을 넣는다. 금지 명칭은 결정 2 의 선택지에 "조사 문서를 저장소에 넣을 때 중립 이름으로 치환한다"를 명시한다.

### P7. 검증 수단이 성립하지 않는 구간 (높음)

1. 현재 헤드리스 스킨 캡처는 `RBMS_SKIN_CAPTURE_DIR` 를 읽는 테스트 5개(`R:apps/rbms-player/src/stage/render_tests_skin.rs`, `render_tests_skin_v3_{decide_result, play_dp, play_sp, select}.rs`)에만 있다. W1-1 이 이 파일들을 삭제한다. 새 하니스를 만드는 단위는 W7-6("테스트·골든 재편")뿐이다. W3-7·W4-4·W4-6·웨이브 5·웨이브 6 의 "캡처" 검증은 그 전에 수단이 없다.
2. 캡처는 1280x720 `CpuCanvas` 픽셀이고 GPU 창과의 동일성은 미검증이다(R:docs/HANDOFF.md:68). 색 공간(표면 sRGB + `Rgba8Unorm`, R:apps/rbms-player/src/gpu/mod.rs:179, 488), 물리 픽셀 래스터, bilinear·distance field 는 CpuCanvas 로 볼 수 없다. GPU 읽기(`copy_texture_to_buffer` 등)는 코드에 없다.
3. 눈으로 볼 결과가 없는 웨이브: 웨이브 1(끝 상태 = "내장 화면이 전과 같다", 회귀 확인뿐이고 해상도 변화는 사용자 GPU 창 확인에만 의존), 웨이브 2(끝 상태 = 덤프 텍스트). 이전 피드백은 "파싱·설치 검사만으로 시각 완료를 판단하지 말 것"이었다(R:docs/feedback/2026-09-17-skin-visual-verification.md).

수정 제안
- 웨이브 1 에 "캡처 하니스" 단위를 추가한다: 임의 해상도 HeadlessCanvas(W1-7 과 같은 파일이므로 그 뒤), 외부 스킨 경로를 환경 변수로 받는 선택 테스트, PNG 출력. 가능하면 wgpu 오프스크린 렌더 타깃 읽기를 같은 단위에 넣어 GPU 경로도 파일로 떨어뜨린다.
- 웨이브 2 의 끝 상태에 "이미지 객체만으로 결정 화면 정지 프레임 1장"을 더한다(텍스트·그래프 없이도 배경·띠·lockon 이 나온다). 이러면 웨이브 2 부터 사용자가 볼 것이 생긴다.

### P8. 기타 (낮음~중간)

| # | 내용 | 제안 |
|---|---|---|
| 1 | W4-3 의 소유가 "`crates/rbms-play/**`, 정확한 파일은 착수 시 확인(미확인)" 이다. 병렬 단위와 충돌 여부를 판단할 수 없다 | 착수 전 조사 단위를 따로 두거나 W4-3 을 단독 실행으로 표시 |
| 2 | 웨이브 순서가 결정 → 결과 → 선곡 → 플레이라 플레이가 마지막이다. 사용자 체감의 중심이 가장 늦다. 또 결과(웨이브 4)는 스킨 플레이가 없는 상태에서 내장 플레이의 기록으로만 검증된다 | 순서는 유지하되 웨이브 4 끝에 "내장 플레이 → 스킨 결과" 실화면 확인 항목을 명시 |
| 3 | 스택 전환과 스킨 수명. `Transition::Open` 은 선곡을 `suspended` 에 쌓고 `Back` 이 되살린다(R:apps/rbms-player/src/lib.rs:867-884). 종합의 전환 규칙("이전 스킨 해제 → 새로 로드 → 타이머 전체 OFF")을 그대로 적용하면 설정·폴더·표·키 설정에서 돌아올 때마다 선곡 스킨(destination 1914)을 다시 구성하고 시작 연출이 다시 돈다 | "Open/Back 은 스킨·Lua 상태·장면 시계를 보존한다(텍스처만 한도에 따라 해제)"를 사양에 추가. 레퍼런스는 매번 재구성하지만 R-BMS 는 선곡 위에 여는 화면이 4개라 체감이 다르다. 사용자 결정 후보 |
| 4 | `skin_render/mod.rs` 의 `OFFSET_ALL` 을 W6-8 이 맡는다고 표 아래 한 줄로만 적었다. 레퍼런스는 렌더러 생성 시 1회만 변환 행렬을 잡고(이동은 스킨 크기의 %, 배율은 (w+100)/100) 플레이 타입에서만 적용한다 | B:skin/Skin.java:377-392, 720-728. W6-8 지시에 식을 적는다 |
| 5 | 종합 2.2 "타이머 함수 1회 호출 후 재사용"은 `timer_observe_boolean` 에 한해 안전하다(같은 프레임 시각을 쓴다, B:skin/lua/TimerUtility.java 의 `timer_observe_boolean`). ModernChic 의 타이머 함수 17개(직접 함수)에는 부수효과가 없는지 단위 지시에서 확인하도록 적는다 | W3-1 지시의 확인 항목 |

---

## 5. 스킨 단독 소유 전환에서 생기는 R-BMS 고유 기능의 구멍

| # | 심각도 | 기능 | 지금 있는 곳 | 구멍 | 제안 |
|---|---|---|---|---|---|
| H1 | 높음 | 빈 목록 안내: 첫 실행 "WELCOME / 폴더 추가는 O", 검색 결과 없음, 차트 없음, 코스 없음 | R:apps/rbms-player/src/stage/select/scene.rs:187-193, 240, R:docs/history/2026-06-07-first-launch-onboarding.md | 스킨 계약에 대응물이 없다. songlist 는 막대 0개를 가정하지 않는다(M2). 종합에는 "첫 실행부터 전 화면이 그려진다"만 있다 | 시스템 오버레이로 안내를 유지하고, 막대 0개일 때 songlist 는 그리지 않는다고 사양에 적는다 |
| H2 | 높음 | IR 랭킹 패널(I 키), 랭킹 리플레이 내려받기·재생, 주 IR 전환, 결과의 IR 상태 줄 | R:apps/rbms-player/src/stage/select/mod.rs:365-538, 856-859, R:apps/rbms-player/src/stage/result.rs:133-137 | 2절 W4. 현재도 스킨이 화면을 대체하면 `draw_select_skin` 뒤 `return` 이라 패널이 안 그려진다(같은 파일 808-810) | 시스템 오버레이로 유지. 스킨 IR 타이머·옵션은 실제 상태에서 공급 |
| H3 | 중간 | 즐겨찾기(F 키, `favorite_only` 필터) | R:apps/rbms-player/src/stage/select/mod.rs:713, R:crates/rbms-config/src/schema.rs:737 | 종합에 언급이 없다. 스킨에는 `MAIN.BUTTON.FAVORITTE_CHART` 버튼이 있다(M:Select/lua/musicdisplay.lua:69) | 이벤트 90(차트 즐겨찾기)과 이미지 인덱스를 R-BMS 즐겨찾기에 연결. F 키 유지 |
| H4 | 중간 | 스킨 버튼 → R-BMS 화면 대응. 스킨의 PRACTICE·AUTOPLAY·REPLAY1~4·KEYCONFIG·SKINSELECT·OPEN_IR_WEBSITE·RIVAL·텍스트 열기(17) 버튼 | M:Select/lua/btnarea.lua:19-40, M:Select/lua/sidemenu.lua:136-152, M:Select/lua/rivalview.lua:137 | 종합 2.9 는 13·14 만 적었다. R-BMS 의 리플레이는 기록별 목록(기록 모달, R 키)이지 4슬롯이 아니고, 연습은 별도 Stage(F4)다. 대응 규칙이 없다 | 이벤트 표에 "PRACTICE → Practice Stage 열기, REPLAY1~4 → 최근 리플레이 4개 또는 무동작, AUTOPLAY → 오토플레이 시작" 을 정한다. 사용자 결정 5 의 항목에 추가 |
| H5 | 중간 | 폴더(O)·표(T)·기록(R)·설정(Tab)·검색(/)·정렬(F3)·필터(F2) 의 마우스 진입과 키 안내 | 하단 버튼 줄과 핫스팟(R:apps/rbms-player/src/stage/select/mod.rs:759-770) | 버튼 줄이 사라지면 폴더·표·설정은 키로만 열린다. 화면 어디에도 키 안내가 없다. 스킨의 도움말 그림(M:Select/lua/help.lua, `help.png`)은 레퍼런스 키 배치를 보여 줘 R-BMS 와 다르다 | 시스템 계층에 "단축키 안내 오버레이(F1 등)"를 둔다. 기본판 도움말 시트는 R-BMS 키로 다시 그린다 |
| H6 | 높음 | 연습 모드: 구간 반복, 시작 게이지, 50~200% 속도, BGA 반복 재시작 | R:apps/rbms-player/src/stage/play/mod.rs:174-176, 319-324, 431-434, 491, 701-702, R:apps/rbms-player/src/practice.rs | 종합 P1 의 상태기계는 연습을 다루지 않는다. 구간이 돌 때마다 PRELOAD(3500ms)·READY(1000ms)를 다시 타는지, 타이머 41·140 을 어디로 되감는지, FADEOUT 을 타는지 정해지지 않았다. 레퍼런스의 연습은 별도 상태(STATE_PRACTICE)와 스킨 내 설정 UI 다(B:play/BMSPlayer.java:524-595) | 사양에 "연습 구간 반복은 READY 부터 다시(PRELOAD 생략), 타이머 41/140 은 구간 시작 기준으로 재설정"처럼 규칙을 적고 W6-3 테스트에 넣는다 |
| H7 | 높음 | 리플레이 분석: 일시정지(Space), 배속(-/=), 탐색(PgUp/PgDn), 하단 분석 띠 | R:apps/rbms-player/src/stage/play/mod.rs:500-513, 659-685, 706-721 | 스킨 타이머는 벽시계 µs 다. 가상 시계가 멈추거나 뒤로 가면 봄·판정·홀드 타이머(46, 50~, 70~)와 41 이 어긋난다. 분석 띠는 1280x720 좌표(`CH - 66`)에 그린다 | 사양에 "분석 모드에서는 장면 시계 자체를 가상 시계에 묶고, 탐색 시 레인별 타이머를 전부 끈다"를 추가. 분석 띠는 시스템 오버레이 목록(종합 2.9)에 이미 있으나 좌표 규칙을 적는다 |
| H8 | 중간 | 결과 화면의 다시 하기(R)·다음 곡(N) | R:apps/rbms-player/src/stage/result.rs:106-108 | 종합 T2 의 입력 표(키 0~3 OK, 4 REPLAY_DIFFERENT, 6 REPLAY_SAME …)에 대응이 없다. 입력 잠금 2500ms 와의 관계도 없다 | R = REPLAY_SAME, N 은 R-BMS 고유로 유지한다고 적는다. 입력 잠금은 둘 다에 적용 |
| H9 | 높음 | 9키·24키 플레이 | 내장 플레이 | 2절 W5 | 2절 W5 |
| H10 | 중간 | 옵션 오버레이 11행 중 스킨 패널에 대응이 없는 것: 스크래치 방향, 오토 스크래치, 타깃, 하이스피드 수치·고정 | R:apps/rbms-player/src/app_options.rs:22-34, 89-92 | 결정 4 추천은 LeftShift/F1 을 패널 1 로 재배정한다. 그러면 위 항목의 빠른 접근이 없어진다(설정 화면으로만). 스크래치 방향은 원천이 둘이 된다: R-BMS `play.scratch_left`(R:crates/rbms-config/src/schema.rs:506)와 ModernChic 스킨 옵션 `playSide`(M:Play/lua/require/sp_property.lua:258) | 옵션 오버레이를 시스템 오버레이로 유지하고(F1), 패널 1~3 은 START/SELECT 에 둔다. 스크래치 방향은 "스킨 옵션이 있으면 스킨이 우선, 앱 설정 행은 숨김"으로 단일화 |
| H11 | 중간 | 스킨이 화면을 소유하면 의미가 없어지는 설정 행: `display.five_key_layout`, `judge_text_y`, `show_white_number`, `score_graph`, `result_graphs`, `display.skin` 프리셋 | R:crates/rbms-config/src/schema.rs:616-634 | 종합은 `display.skin` 프리셋 결합만 다룬다. 나머지는 스킨 화면에서 죽은 설정이 된다 | "스킨 적용 화면에서는 숨김 / 내장 폴백 전용 표시"로 정리. W7-1 범위에 추가 |
| H12 | 중간 | 명령줄 직접 실행(차트·리플레이 경로) | R:apps/rbms-player/src/lib.rs:1007-1019 | Select·Decide 를 거치지 않고 Play 로 들어간다. 종합의 흐름 표(3.1)에 이 진입이 없다. PRELOAD 가 단독으로 로딩을 맡아야 하고, 끝난 뒤 돌아갈 선곡이 없을 수 있다 | 3.1절에 행 추가: "직접 실행 → Play(PRELOAD) → Result → 종료 또는 선곡" |
| H13 | 낮음 | 플레이 탈출 방식(ESC 홀드/두 번), 플레이 중 조작 키 8종 | R:apps/rbms-player/src/stage/play/mod.rs:30-34, 448-486, R:apps/rbms-player/src/keyconfig.rs:199-208 | 레퍼런스식(ESC 즉시, START+건반)과 다르다. 폐점·페이드 상태에서 ESC 의 의미가 정해지지 않았다 | "R-BMS 방식 유지, FAILED/FINISHED 중 ESC 는 연출 건너뛰기"로 명시 |
| H14 | 낮음 | 선곡 미리듣기(파일 또는 오토플레이 키음) | R:apps/rbms-player/src/stage/select/preview.rs, R:docs/history/2026-06-07-select-autoplay-preview.md | 종합 S11 의 선택 BGM 루프와 겹친다. R-BMS 는 모든 곡에 미리듣기가 있어 BGM 이 거의 들리지 않는다 | "미리듣기 재생 중 BGM 정지, 포커스 이동 후 디바운스 동안만 BGM" 규칙을 W5-6 에 적는다 |
| H15 | 낮음 | 토스트·서버 연결 점·디버그 패널 | R:apps/rbms-player/src/lib.rs:948-976 | 1280x720 고정 좌표(`CW - 22` 등)다. 종합 2.5 의 "배율 래퍼"로 해결되지만, 토스트 위치가 스킨 요소(ModernChic 상단 프레임, 좌상단 도움말 버튼 x=10,y=1050)와 겹치는지는 보지 않았다 | 웨이브 3 캡처 항목에 "토스트·디버그 패널 겹침" 추가 |

---

## 6. 사실로 확인한 종합 매트릭스 행 (R-BMS 원본 대조)

| 종합 행 | 종합의 현황 서술 | 확인 결과 | 근거 |
|---|---|---|---|
| C2 | lua54, 표준 라이브러리 일부 | 맞음 | R:crates/rbms-skin/Cargo.toml:19 |
| C6 | 타이머 식은 경고 후 무시 | 맞음 | R:crates/rbms-skin/src/loader/track.rs:231-241 |
| C12 | 보간·루프·오프셋·클립 일치, acc 결정 순서만 차이 | 맞음. `effective_acc` 가 시간 정렬 뒤 첫 비선형을 고른다(레퍼런스는 선언 순서 첫 비영) | R:crates/rbms-skin/src/dst.rs:242-244, 372-433, R:crates/rbms-skin/src/loader/track.rs:180-181 대 B:skin/SkinObject.java:218-220, 348-433, 545-575 |
| C13 | 미지 id 를 매 프레임 읽어 음수면 표시 | 맞음. 기본 `known_option` 이 전부 true 라 조건이 남는다 | R:crates/rbms-skin/src/loader.rs:163-165, 196, R:crates/rbms-skin/src/loader/track.rs:193-225 대 B:skin/Skin.java:216-229 |
| C14 | ms `i64`/`Option`, 전환 시 미초기화 | 맞음 | R:crates/rbms-skin/src/timer.rs(`TimerState { on: HashMap<TimerId, i64> }`) |
| C15 | 오프셋 `r` 부호 반대 | 맞음. 각도는 로더에서 부호를 뒤집는데 `offset.r` 은 그대로 더한다 | R:crates/rbms-skin/src/loader/track.rs:107, R:crates/rbms-skin/src/dst.rs:427-430 대 B:skin/SkinObject.java:526-543 |
| C16 | stretch 0·1·2·9 만 | 맞음 | R:crates/rbms-skin/src/loader/stretch.rs:61-63, 89-106 |
| C17 | `ref` 를 정수로 읽고 음수면 0번 | 맞음 | R:crates/rbms-render/src/skin_render/draw.rs:190-197 |
| C19 | 숫자 시트 식 일치 | 맞음. 24배수만 `zeropadding`, 10/11 장은 `padding`, 11 장은 2 강제 | R:crates/rbms-render/src/skin_render/object.rs:672-681, R:crates/rbms-render/src/skin_render/draw.rs:53-56, 262-279 대 B:skin/json/JsonSkinObjectLoader.java:99-172 |
| C20 | 텍스트 overflow/그림자/wrapping 없음 | 맞음. `constantText` 가 `ref` 보다 우선한다는 차이도 있다 | R:crates/rbms-render/src/skin_render/draw.rs:316-336 |
| C24 | 좌클릭 press 만 | 맞음 | R:apps/rbms-player/src/lib.rs:1034-1044 |
| C25 | 논리 1280x720 상수 | 맞음 | R:apps/rbms-player/src/lib.rs:134-135, R:apps/rbms-player/src/gpu/mod.rs:198, 617-626 |
| C27 | 표면 sRGB + `Rgba8Unorm` | 맞음(코드 기준) | R:apps/rbms-player/src/gpu/mod.rs:179, 488 |
| C28 | 하드웨어 Linear 근사, 1:1 판정이 논리 좌표 | 맞음. `filter == 0` 은 Nearest 로 레퍼런스(텍스처 기본 Nearest)와 같다 | R:crates/rbms-skin/src/loader/stretch.rs:124-128, R:crates/rbms-render/src/skin_render/draw.rs:99-107 대 B:skin/SkinObject.java:634-636, B:skin/SkinLoader.java:136-172 |
| C31 | type 9·15 거부, `def` 를 그대로 치환 | 맞음 | R:crates/rbms-skin/src/loader.rs:240-241, 608-609, R:crates/rbms-skin/src/resolve.rs:324-330 |
| C1 | 확장자 `json`/`json5` 고정 | 맞음 | R:apps/rbms-player/src/skin_select.rs:38-39 |
| 0절 2 | 블렌드가 레퍼런스와 일치 | 맞음. 2·3 → 가산, 4 → 곱, 9 → 대상 반전, 그 외 알파 | R:crates/rbms-render/src/lib.rs:134-156 대 B:skin/Skin.java:655-665, 675-683 |
| P3 | 위치가 내장 `Skin` 의 `judge_y`/`top_y` 의존, processed·expansion 미구현 | 맞음 | R:crates/rbms-render/src/skin_render/notes.rs:12-14, 159-171, 221, 249-250 |
| P5 | 애니메이션을 결정적 함수로 대체 | 맞음 | R:crates/rbms-render/src/skin_render/gauge.rs:10-11, 205-213 대 B:play/SkinGauge.java:113-135 |
| P6 | 콤보를 `NUMBER_COMBO` 로 강제, 플레이어 접기 | 맞음 | R:crates/rbms-render/src/skin_render/judge.rs:21, 103, 143-146 |
| P7 | READY 를 끔, 42 를 켬, 봄을 켜고 끔 | 맞음. 레퍼런스는 42 를 어디서도 켜지 않는다 | R:crates/rbms-render/src/skin_render/screen.rs:262-264, 290-294, 311-338 대 `grep TIMER_GAUGE_INCLEASE` (B:play 에 사용처 없음) |
| S3 | `SelectItem::{Song, Folder}` 둘 | 맞음 | R:apps/rbms-player/src/lib.rs:406-409 |
| S4 | 10/12/13 을 켜고 22/23/32/33/1 없음 | 맞음 | R:crates/rbms-render/src/skin_render/screen.rs:366-370 |
| S5 | 패널 1 하나(LeftShift/F1), 11행 | 맞음 | R:apps/rbms-player/src/app_options.rs:22-34, 89-92, 140-142 |
| T1 | 151 을 1초 뒤, 152 를 신기록 때만, 키 즉시 전환 | 맞음 | R:crates/rbms-render/src/skin_render/screen.rs:381, 405-415, R:apps/rbms-player/src/stage/result.rs:101-110 대 B:result/MusicResult.java:160-170 |
| N3 | `custom[문서경로]` 저장 | 맞음 | R:crates/rbms-config/src/schema.rs:249-263 |

---

## 7. 레퍼런스 의미론 대조 결과

| # | 항목 | 종합의 서술 | 원본 | 판정 |
|---|---|---|---|---|
| 1 | 보간·루프 | 객체 단위 acc, loop 처리 | B:skin/SkinObject.java:360-375, 545-575 | 일치 |
| 2 | 오프셋·알파·각도 | `x += off.x - off.w/2` 등, 색 보간 중 알파 오프셋 누락 | B:skin/SkinObject.java:404-413, 480-543 | 일치 |
| 3 | 그리기 조건 | 내장이면 조회, 아니면 옵션 맵, 미지 id 는 부호와 무관하게 제거, 정적은 1회 평가 | B:skin/SkinObject.java:282-299, B:skin/Skin.java:199-237 | 일치 |
| 4 | 블렌드 | 2 가산, 3 은 식 복원 후 가산, 4 곱, 9 반전 | B:skin/Skin.java:655-665 | 일치 |
| 5 | filter | `filter != 0` 이고 크기가 다를 때만 bilinear | B:skin/SkinObject.java:634-636 | 일치 |
| 6 | 숫자 시트 | 24/10/11 규칙 | B:skin/json/JsonSkinObjectLoader.java:99-172 | 일치(필드가 `zeropadding`/`padding` 으로 갈린다는 점은 b2 §10.2 에만 있고 종합 C19 에는 없다) |
| 7 | 2패스 로드와 변환 | 같은 상태에서 두 번, 함수 → 숫자 → 이름 → 스크립트 | B:skin/lua/LuaSkinLoader.java:50-95, 152-207 | 일치 |
| 8 | Lua 값 함수 오류 | 기본값으로 대체, 타이머 오류는 OFF, nil 은 0 | B:skin/lua/SkinLuaAccessor.java:611-628, 640-652, 735-747 | 일치 |
| 9 | 이벤트 인자 | `narg()` 분기가 있으나 항상 1개 | B:skin/lua/SkinLuaAccessor.java:759-788 (LuaJ 에서 `LuaValue.narg()` 는 1) | 일치 |
| 10 | songlist 스크롤 | 벽시계 선형, x 만 [-1,1] 클램프, 300ms/50ms | B:select/BarRenderer.java:113-138, 439-462 | 일치 |
| 11 | 막대 종류 인덱스 | 0~6, 텍스트 11종 | B:select/BarRenderer.java:160-196 | 일치 |
| 12 | 결정 화면 | input → STARTINPUT, scene → FADEOUT, 키 0/2/4/6·ENTER, ESC·START+SELECT | B:decide/MusicDecide.java:37-65 | 일치 |
| 13 | 결과 타이머 | 150/151 첫 프레임, 152 는 ranktime == 0 | B:result/MusicResult.java:160-170 | 일치 |
| 14 | 플레이 상태기계 | PRELOAD → READY → PLAY → FAILED/FINISHED | B:play/BMSPlayer.java:479-517, 605-617, 620-691, 693-777 | 일치하나 M1(차트 미리보기), M7(즉시 재시작), W2(게이지 이력)가 빠짐 |
| 15 | gauge | `종류*6 + 상태`, type 3 점멸 | B:play/SkinGauge.java:182-215 | 일치하나 M5 가 빠짐 |
| 16 | 비트맵 폰트 | `scale = size / 원본size`, type 1/2 distance field, 그림자 rgb/2 | B:skin/SkinTextBitmap.java:93-104, 118-127 | 일치 |
| 17 | luajava facade | `Input.Keys.RIGHT` 가 -1 | B:skin/lua/LegacySkinLuaApi.java:313-338 | 일치 |
| 18 | audio_play | 0..2 클램프 x 시스템 볼륨 | B:skin/lua/SkinAudioLuaApiExporter.java:20-62 | 일치 |
| 19 | 프레임 순서·마우스 | 로직 → 커스텀 → prepare → draw → 입력, press 는 z 역순 첫 객체, drag 는 슬라이더만 | B:MainController.java:403-413, 494-508, B:skin/Skin.java:394-420 | 일치 |

---

## 8. ModernChic grep 집계 (재현 가능한 수치)

| 대상 | 결과 | 종합과 비교 |
|---|---|---|
| `main_state.*` | option 230, number 207, audio_play 82, text 62, time 22, timer 16, timer_off_value 15, event_index 14, gauge_type 11, judge 6, volume_sys/key/bg 각 2, set_volume_sys/key/bg 각 2, rate 2, float_number 2, gauge 1, exscore 1 (20종) | 일치 |
| `timer_util.*` | timer_observe_boolean 148. `event_util` 0 | 일치 |
| 표준 라이브러리 | table.insert 2200, io.open 45, math.random 13, math.floor 12, os.date 5, luajava.bindClass 5, math.modf 4, os.time 3, luajava.newInstance 3, table.concat 2, string.sub 2, string.format 2, luajava.new 2, string.match/gsub/gmatch 각 1, math.max/exp/ceil 각 1 | 종합 2.2 의 허용 목록 안 |
| 전역 함수 | require 145, print 118, pcall 110, dofile 109, tostring 11, tonumber 8. `setmetatable`·`coroutine`·`loadstring`·`unpack`(전역) 0 | 일치 |
| `skin.*` 최상위 키 | destination, image, source, value, graph, judgegraph, slider, bpmgraph, imageset, gauge, text, font, timingvisualizer, note, liftCover, judge, hiterrorvisualizer, hiddenCover, timingdistributiongraph, gaugegraph, bga, customTimers, songlist, customEvents, skinSelect, pmchara (26종). `floatvalue`·`skinpreview`·`practice` 0 | graph 행 없음(M4) |
| destination 필드 값 | blend: ADDITION 71, ALPHA 21, 9 가 1. stretch: 3 이 10, 5 가 12, 1 이 3. filter: ON 3. acc: 2 가 80, 1 이 7. loop: -1 이 271. `center`·`event` 0 | 일치 |
| 함수 값 필드 | draw 함수 약 330, value 함수 약 130, act 함수 35, timer 직접 함수 17 | 일치 |
| 타이머 id | 88종. 종합 P7 의 "추가" 목록에 없는 것: 141(8회). 48(9회)·44(5회)·3(13회)은 R-BMS 가 이미 구동 | M1 |
| NUM/OP/STRING/BUTTON | NUM 177종, OP 139종, STRING 60종, BUTTON 70종 | 종합 S9 의 177/139/60 과 일치 |
| 레퍼런스 상수에 없는 id | STRING 3, 200~219(종합 7.3 이 언급), 그 외는 범위 상수·팩토리에 존재(NUM 525/526 = B:skin/property/IntegerPropertyFactory.java:469-470) | 일치 |
| 파일·네트워크 | io.open 45, `File:mkdir/listFiles`, URL 은 블록 주석 뒤(M:musicselect.lua:215-227) | 일치 |
| 경로 대소문자 | 리터럴 경로·require 335개 중 불일치 0 | 문제 없음 |

---

## 9. 읽은 범위와 읽지 못한 범위

| 대상 | 읽은 것 | 읽지 못한 것 |
|---|---|---|
| 종합 `00-synthesis.md` | 전문(1-614) | 없음 |
| 조사 보고서 14개 | 종합이 인용한 절을 필요할 때 grep·부분 열람(b2 §10·§14, b3 일부, b4 일부, b5 일부, m1 일부, r1 §3, r2 일부) | 전문 통독은 하지 않았다. 보고서 내부의 오류는 이 비판의 대상에서 빠져 있다 |
| R-BMS | `crates/rbms-skin/src/{dst.rs, loader/track.rs, loader/stretch.rs}` 전문, `timer.rs`, `skin_render/draw.rs` 전문, `stage/mod.rs` 전문, `lib.rs` 850-1064, `stage/select/mod.rs` 636-869, 그 외는 grep 과 부분 열람 | `skin_render/{object.rs, mod.rs, songlist.rs, graphs.rs, notes.rs 본문}`, `skin_screen.rs`, `assets.rs`, `app_play.rs`, `app_result.rs`, `crates/rbms-play/**` 는 전문을 읽지 않았다 |
| beatoraja | `skin/SkinObject.java` 150-709, `skin/Skin.java` 195-434·560-790, `skin/lua/LuaSkinLoader.java` 전문, `skin/lua/SkinLuaAccessor.java` 46-96·560-790, `play/BMSPlayer.java` 476-777, `decide/MusicDecide.java`, `result/MusicResult.java` 160-222, `select/BarRenderer.java` 86-330 일부·410-490 grep, `play/SkinGauge.java` 95-215, `skin/SkinGraph.java`, `skin/SkinHeader.java` 152-221, `skin/SkinLoader.java` 85-179, `skin/json/JsonSkinObjectLoader.java` 39-175·561-650 | `play/LaneRenderer.java`, `play/JudgeManager.java`, `skin/SkinTextFont.java`, `skin/SkinNoteDistributionGraph.java`, `select/SkinBar.java`, `skin/property/*Factory.java` 전문. note·judge·텍스트(TTF)·노트 분포 그래프의 식은 이 비판에서 재검증하지 않았다 |
| ModernChic | 전 Lua 에 grep. 열어 본 파일: `musicselect.lua` 195-300, `decide.lua` 일부, `keyconfig.lua`, `Root/{define.lua, customfunction.lua 1-40, mainimage.lua}`, `Select/lua/{help.lua, versioncheck.lua, require/http.lua}`, `Decide/lua/require/{textproperty.lua, property.lua 일부}`, `Play/lua/sp/detailinfo/bgaareainfo.lua` 일부 | 본체 파트 파일 대부분은 grep 만 했다. Lua 를 실행하지 않았으므로 객체 수는 재검증하지 않았다(실행하면 스킨 폴더에 `History/information.txt` 를 쓰므로 금지 규칙에 걸린다) |
