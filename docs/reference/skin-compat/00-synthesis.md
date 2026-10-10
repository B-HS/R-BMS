# 00. 조사 종합 — 격차 매트릭스, 목표 구조, 작업 분해, 사용자 결정 목록

> 최종 갱신 2026-10-10 · 대응 단계: 웨이브 5(선곡) 반영 · 본문은 작성 시점 서술이며, 아래 "웨이브 … 반영 사항"이 최신 것부터 우선한다 · 색인과 갱신 규칙은 [README.md](README.md)

## 웨이브 5 반영 사항 (2026-10-10)

songlist 재작성, 선곡 상태 모델, 선곡 입력 키 표와 장면 수명, 패널 1~3 과 옵션 이벤트, 호스트 군집 F·H(선곡)·E, 슬라이더 쓰기와 편집 텍스트, 선곡 사운드, 스킨 위 시스템 오버레이를 넣은 뒤의 상태다.

- (W5-1) 00-synthesis.md 매트릭스의 songlist 행(S 계열): 렌더 쪽 구현 완료(W5-1), 데이터 공급과 클릭 실행은 W5-2·W5-3 로 상태 갱신
- (W5-2) 00-synthesis.md 매트릭스: songlist 데이터 공급(막대 종류·램프·라벨·분포)과 참조 이미지(-100/-101/-102, 선곡) 행을 '구현(W5-2)'으로, 트로피·라이벌 램프는 '원천 없음'으로 표시
- (W5-4) 00-synthesis.md 매트릭스: 선곡 속성 연결(군집 F)·선곡 IR 상태(군집 H)·선곡 설정 표시(군집 E) 행을 구현으로, 군집 A 선곡 연결(ChartState)과 Stage 소유 패널·필터 연결은 미완으로 갱신.
- (W5-3b) 00-synthesis.md 매트릭스: 선곡 패널 1~3, 옵션 이벤트, 스킨 버튼 대응 행을 구현으로 갱신
- (W5-7) 00-synthesis.md E2·S 계열 매트릭스: 선곡의 빈 목록 안내, IR 랭킹, 기록 모달, 필터, 옵션 오버레이, 단축키 안내 행을 완료로 갱신
- (리뷰 수정) 00-synthesis.md 매트릭스 S·E 행: 선곡의 리플레이 슬롯 선택(RESET_REPLAY, NEXT_REPLAY)과 코스 정보(문자열 150~159, 옵션 1002~1017)를 구현 완료로 바꿉니다


## 웨이브 4 반영 사항 (2026-10-10)

스킨 입력 디스패치와 이벤트 실행기, gauge·gaugegraph·timingdistributiongraph·judgegraph type 1·2, 플레이 엔진 기록 확장, Result·CourseResult Stage 의 장면 수명, 호스트 군집 B·G·H·E 일부, 스킨 사운드 버스를 넣은 뒤의 상태다.

- (W4-3) 00-synthesis.md §7.3: '게이지 이력 샘플 간격 … 전 종류 동시 기록 방식은 미확인' 항목을 해소로 표시(근거 BMSPlayer.java:625-635, 723-729, 구현 rbms-play record.rs)
- (W4-1) 00-synthesis.md 매트릭스: 입력 관련 행(객체 클릭, 슬라이더 드래그, mouseRect 호버, Gdx 키 폴링)을 구현 완료로, 텍스트 입력과 곡 바 클릭은 미구현으로 표시합니다
- (W4-2) 00-synthesis.md 매트릭스 T4(와 gaugegraph·timingdistributiongraph·judgegraph type 1/2 행): 렌더 구현 완료, 데이터 공급은 앱 단위 대기로 갱신
- (W4-5) 00-synthesis.md §1.6 결과 매트릭스(T 행) 중 상태 공급 항목과 §1.7 코스 결과(G 행)의 상태 공급 항목을 'W4-5 구현, 연결은 W4-4/W4-6'으로 표시. README.md 갱신 이력에 W4-5 행을 추가.


## 웨이브 3B 반영 사항 (2026-10-10)

스킨 텍스처 관리자(참조 source 만 로드, 화면 이탈 해제, 예산)와 Decide Stage(장면 수명 헬퍼, 백그라운드 차트 로드)를 넣은 뒤의 상태다.

- (W3-7) 00-synthesis.md 흐름 표(3.1)와 매트릭스의 장면 수명·DECIDE 행: 결정 화면 구현 완료. 연습·리플레이도 Decide 경유, 코스는 첫 스테이지만
- (리뷰 수정) 00-synthesis.md 매트릭스의 텍스처 수명 행(화면 이탈 시 해제): 상태를 '주차 화면 포함 구현'으로 갱신


## 웨이브 3A 반영 사항 (2026-10-10)

prepare/draw 2단계 파이프라인과 `SkinHost` 직접 그리기, 그리기 조건 의미론, 참조 이미지·음수 크기·이미지 인덱스·숫자·슬라이더·그래프 정합, TTF 텍스트, judgegraph·bpmgraph, Lua 함수 값 프레임 평가, 앱 호스트 군집 A·I·M 을 넣은 뒤의 상태다.

- (W3-1a) 00-synthesis.md 매트릭스: 'songlist·note 의 dst 없는 destination 처리' 항목을 'W3-1a 해소(자체 키프레임 규칙)' 로, prepare/draw 2단계 항목을 'W3-1a 구현' 으로 표시
- (W3-1b) 00-synthesis.md 매트릭스의 조건 의미론 행(미지 op 제거, 정적 1회 평가, 음수 타이머)과 README.md 갱신 이력: W3-1b 완료로 표시
- (W3-5) 00-synthesis.md P10 행: judgegraph/bpmgraph 부분 -> 해소(W3-5, 2026-10-10). type 1/2 는 데이터 구조와 색 표까지 완료, 실제 판정 기록 공급은 W4-3/W4-5. T8, E4 행도 같은 내용으로 갱신
- (W3-5) 00-synthesis.md 위험 목록: 그래프 데이터 통로(FrameExtra 화면당 한 종류) 해소 항목에 NoteDistribution 의 kinds/judgements/early_late/playing 과 BpmTimeline::of_chart 추가
- (W3-1c) 00-synthesis.md §2.2: '타이머 함수 1회 호출 후 재사용' 을 구현 완료(W3-1c)로 표시
- (W3-3) 00-synthesis.md 매트릭스의 텍스트 행(TTF)과 누락 글리프 행: 1차 TTF 구현 완료로 갱신, .fnt(type 0/1/2)는 웨이브 7 그대로


## 웨이브 2B 반영 사항 (2026-10-10)

스킨 덤프 CLI, 앱의 스킨 팩 폴더 지정과 `.luaskin` 로드, 오버레이 총 크기 상한, 외부 스킨 첫 정지 프레임을 넣은 뒤의 상태다.

- (W2-9) 00-synthesis.md 매트릭스: 참조 이미지(-100~-111), 음수 w/h 뒤집기, songlist·note 의 dst 없는 destination 처리 세 항목에 'W2-9 캡처로 현재 미구현 확인, 영향 범위는 r2 §5 실측 참조' 표시


## 웨이브 2A 반영 사항 (2026-10-10)

Lua 5.2 런타임(`crates/rbms-skin/src/lua/`), `SkinHost`, Lua 값 변환기, 2패스 `.luaskin` 로더를 넣고 구 샌드박스(`skin.*`)를 삭제한 뒤의 상태다.

- (W2-0) 00-synthesis.md §2.2(Lua 런타임 결정 절): 모듈 배치와 `SkinHost` 시그니처가 사양 §4.4 표와 다른 두 곳(`offset` 은 Option, `rate()`/`exscore()`/`volume_*` 는 `score(ScoreSlot)`/`volume(VolumeBus)`)을 기록한다. 사양 문서 `docs/plan/2026-10-09-lua-skin-compat.md` §4.4 표도 같은 내용으로 맞춰야 한다.
- (W2-4) 00-synthesis.md 매트릭스: Lua 테이블 변환(필드명 일치, 절삭, 진리값, 문자열화, 배열 구멍, 함수/이름/스크립트 참조)에 해당하는 행을 '구현(W2-4)'으로 갱신해야 합니다. 정확한 행 번호는 확인하지 않았습니다.
- (W2-2b) 00-synthesis.md §1 매트릭스 C2 행: 현재 상태 열의 'io·os … 제거' 를 io 오버레이·os 5함수 구현 완료(W2-2b)로 갱신해야 합니다.
- (W2-3) 00-synthesis.md 매트릭스: main_state API 행과 timer_util/event_util 행을 '구현(W2-3)' 으로, 정적 불리언 분류 행을 '표 생성 완료, 로드 시 1회 평가는 W3-1b' 로 표시.
- (W2-5) 00-synthesis.md 매트릭스: 2패스 로드, 헤더 병합(def 스템·무작위·자동 오프셋), type 9·15 로드, 플레이 헤더 적용 조건, 스킨 해상도 규칙 행을 '구현(W2-5)'로 갱신
- (리뷰 수정) 00-synthesis.md §2.2 '예산' 행: '프레임: 호출 횟수 상한 없이 벽시계 상한만' 은 낡았습니다. 현재는 호출 횟수(16,384)·호출당 명령 수(100만)·호출 구간 누적 벽시계(50ms) 세 상한이 있고, 로드는 명령 수(5억)·벽시계(10초), 메모리 256MiB 입니다(모두 잠정값)
- (리뷰 수정) 00-synthesis.md §2.2 '표준 라이브러리' 행: 'string(패턴 함수 포함, dump 제외)' 에 패턴 함수가 Rust 자체 구현이라는 점과 `setmetatable` 의 `__gc` 무효화를 추가해야 합니다


## 웨이브 1B 반영 사항 (2026-10-10)

타이머 µs, stretch 11종, 장면 시계, 마우스 이벤트, GPU 논리 크기 런타임화와 색 공간, START/SELECT, 해상도 설정, 캡처 하니스를 넣은 뒤의 상태다.

- (W1-4) 00-synthesis.md 매트릭스 C14(타이머) 행: 현황 '부분. ms i64/Option' 을 'µs 저장·OFF = i64::MIN 완료(W1-4). 전환 시 미초기화와 앱 전체 시계는 남음' 으로
- (W1-4) 00-synthesis.md 매트릭스에서 stretch 11종, acc 선언 순서, 오프셋 r 부호에 해당하는 행(공통 그리기 의미론)과 §4 W1-4 행(364행)을 완료로 표시하고, 소유 파일에 실제로 고친 `apps/rbms-player/src/app_options.rs` 를 추가
- (W1-7) 00-synthesis.md C25 행: '부분. 논리 1280x720 상수' -> Gpu 논리 크기 런타임화 완료(뷰포트 물리 픽셀). 남은 것은 스킨 화면 호출부의 Canvas::native() 전환.
- (W1-7) 00-synthesis.md C27 행: '불일치 추정 ... 실화면 미확인' -> 실측으로 불일치 확인 후 바이트 통과로 수정. 내장 화면 색감이 어두워지는 부작용 기록.
- (W1-7) 00-synthesis.md C28 행: 물리 픽셀 판정의 전제(C25)는 충족, 스킨 호출부 전환 뒤 자동 적용. 내장 배경 슬롯은 이미 물리 픽셀 판정.
- (W1-7) 00-synthesis.md §2.5 마지막 항목: 'wgpu 기본 한도 값은 조사에서 확인되지 않았다(미확인)' -> 기본 8192 확인, 어댑터 한도 사용으로 변경.
- (W1-10) 00-synthesis.md §1 매트릭스 S5 행 (패널 1/2/3) 의 선행 조건과 §2 입력 행 (282행 '레인 키·START/SELECT'): 'START/SELECT 바인딩 유무는 미확인' 을 '바인딩과 상태 질의가 W1-10 으로 존재' 로 고치고, §7.3 미확인 목록(594행)의 'START/SELECT 바인딩 유무' 를 삭제합니다.
- (W1-9) 00-synthesis.md 203행(난수: '테스트·캡처는 고정 시드'): 현재 캡처 하니스는 시드를 직접 고정하지 못하고 RBMS_SKIN_SEED 환경 변수에 의존한다고 적어야 함. 호스트가 로드 옵션에 시드를 넣는 경로(skin_select::reload_for → SkinLoadOptions.rng_seed)가 아직 없음.
- (W1-9) 00-synthesis.md 388행(결정 화면 캡처 t = 0, 500, 1500, 3000ms): 수단은 Shot.scene_us 이며, 장면 시계가 실제 시계라 문서가 보는 시각은 t 이상 t + 프레임 소요 시간 이하라는 제약을 덧붙여야 함.
- (리뷰 수정) 00-synthesis.md 매트릭스의 해상도·stretch 관련 행: stretch 8/9/10 과 필터 1:1 판정이 물리 픽셀에서 이뤄지는 화면이 선곡·결정·결과·키 설정 4종이고 플레이는 미완이라는 상태로 갱신해야 합니다.


## 웨이브 1A 반영 사항 (2026-10-09)

혼합 합성과 구 번들을 삭제한 뒤의 상태다. 본문의 해당 절은 아래 내용으로 읽는다.

- (W1-1a) 00-synthesis.md 290행(`scope: 'bundle'` 과 `shared` 저장), 295행(display.skin 프리셋·theme.ron 결합), 297행(설정 마이그레이션): 앱·설정 쪽 완료로 표시. 297행에 '스키마 2 → 3, Config::retire_bundled_skin, 스키마 0~2 에서 실행' 기재.
- (W1-3) 00-synthesis.md §2.2 'Lua 버전' 행: 결정이 구현 완료로 바뀌었고 '미검증'이던 수 표현 차이가 skin_lua.rs 의 a_whole_quotient_joins_into_text_without_a_fraction 등 테스트로 검증되었음을 적어야 합니다. 또한 r1 §5.3 의 '이 조사에서 실행으로 재현하지는 않았다(미검증)' 문구는 5.2 쪽 동작에 한해 검증됨으로 갱신해야 합니다.


작성일 2026-10-09. 읽기 전용 종합. 세 대상 디렉터리는 수정하지 않았고 cargo 는 실행하지 않았다.

표기
- `R:` = `/Users/hyunseokbyun/development/R-BMS/`, `B:` = `/Users/hyunseokbyun/development/beatoraja/src/bms/player/beatoraja/`, `M:` = `/Users/hyunseokbyun/Downloads/ModernChic/`.
- `[r1 §6.2]` 처럼 대괄호로 적은 것은 같은 폴더의 조사 보고서 절이다. 원본 줄 번호는 보고서가 확인한 것을 옮겼고, 이 종합에서 직접 다시 연 것은 "직접 확인"으로 표시했다.
- 크기: S = 한두 파일 100줄 안팎, M = 여러 파일 수백 줄, L = 새 모듈 또는 구조 변경.
- 이 문서가 읽은 범위와 읽지 못한 범위는 끝의 8절에 있다. 14개 보고서 전문을 다 읽지 못했다.

---

## 0. 결론 요약

1. ModernChic 은 데이터가 아니라 실행되는 Lua 프로그램이다. 화면에 들어갈 때마다 새 Lua 상태에서 진입 파일을 두 번 실행(헤더 → 본체)하고, 본체가 만든 클로저 수백~수천 개를 매 프레임 부른다 [b1 §3, m1 §2.2]. R-BMS 의 현재 로더(JSON 전용), 샌드박스(한 줄 식 전용), 상태 공급(화면별 match 어댑터), 화면 흐름(로딩 = decide), 합성(내장 + 문서 혼합)은 전부 이 전제와 맞지 않는다 [r1 §0, r2 §0, r3 §0].
2. 재사용할 수 있는 것은 분명하다: `SkinDef` 미러(레퍼런스 43개 최상위 필드와 기본값 일치) [r1 §3], `dst.rs` 보간(레퍼런스와 단계별 일치) [r1 §6.1], 속성·타이머 id 표 968개(레퍼런스와 차이 0) [b5 §1-1], `Renderer` 프리미티브(블렌드·회전·클립이 레퍼런스와 일치) [r2 §1.4], `SkinViewport`(임의 해상도 변환).
3. 새로 만들어야 하는 큰 덩어리는 여섯 개다: (가) Lua 5.2 런타임과 2패스 로더, (나) Lua 값 → `SkinDef` 변환기와 함수 값 필드, (다) id → 값 공급 계층(읽기·이미지 인덱스·이벤트·writer·센티널), (라) 레퍼런스식 장면 수명(전환 시 타이머 리셋, input/scene/fadeout, play 상태기계), (마) 객체 의미론 정합(텍스트, songlist, note, gauge, judge, 그래프, 클릭), (바) 해상도·텍스처 수명.
4. 내장 화면과의 혼합 합성(`composition`/`layer`/`replace`/`hotspot`/사설 id 20001~)과 steel-neon 3세대는 맨 먼저 철거한다. 철거 뒤에도 내장 네이티브 화면이 단독으로 그려지므로 빌드와 조작은 유지된다(1절 근거, 4절 웨이브 1).
5. 검증이 가장 싼 순서는 결정 → 결과 → 선곡 → 플레이다. 결정 화면은 상태 요구가 option 150-155, number 96, string 10-15/1003, 참조 이미지 -100, 그래프 2종, timer 2 뿐이다 [m5 §1-17].
6. 기본 스킨 단순화판은 원본 476 MiB 에서 35~45 MiB 로 줄일 수 있다 [m6 §9.1]. 다만 원본 readme 가 스킨 자체의 2차 배포를 금지하므로(허가 시 예외) 배포 방식은 사용자 결정이 선행돼야 한다 [m6 §12.1].

---

## 1. 요구 기능 매트릭스

"현황" 열: 있음 / 부분 / 없음. "담당 파일"은 R-BMS 기준이며 신규 파일은 (신규)로 표시했다.

### 1.1 공통 (모든 화면)

| # | 기능 | 레퍼런스 의미론 요약 | R-BMS 현황 (근거) | 필요한 작업 | 크기 | 담당 파일 |
|---|---|---|---|---|---|---|
| C1 | `.luaskin` 2패스 로드 | 같은 Lua 상태에서 진입 파일을 `skin_config == nil` 로 한 번, 설정 병합 후 한 번 더 실행. `package.loaded` 유지. 로드마다 새 상태 [b1 §3.2, `B:skin/lua/LuaSkinLoader.java:68-95` 직접 확인] | 없음. 진입점이 JSON 파싱뿐, 확장자 `json`/`json5` 고정 [r1 §2.1, `R:apps/rbms-player/src/skin_select.rs:39`] | Lua 로드 경로 신설, 확장자 분기 | L | `crates/rbms-skin/src/loader.rs`, `loader/lua_skin.rs`(신규), `apps/rbms-player/src/skin_select.rs` |
| C2 | Lua 실행 환경 | LuaJ 3.0.2(Lua 5.2 계열). base, package, table, string(패턴 포함), math, bit32, coroutine, 스킨 폴더 제한 io(읽기·쓰기), os 일부(clock/date/difftime/setlocale/time), luajava facade, `debug.getmetatable` [b1 §2] | 없음에 가까움. lua54, MATH/STRING/TABLE 만, `require`·`dofile`·`io`·`os`·`print`·패턴 함수 제거 [r1 §5.1] | 런타임 전면 재구성(2절 a) | L | `crates/rbms-skin/src/lua/*`(신규 분할), `Cargo.toml` |
| C3 | `require`/`dofile` | `require("A.b")` 는 점을 구분자로 바꿔 `?.lua;<스킨폴더>/?.lua` 탐색, 캐시. `dofile(절대경로)` 는 캐시 없음, 같은 전역 [b1 §2.4] | 없음 | 스킨 루트 전용 검색기, 루트 가두기 재사용(`resolve::contained`) | M | `lua/package.rs`(신규), `resolve.rs` |
| C4 | `skin_config` | `option[이름]=op`, `enabled_options[]`, `file_path[이름]`, `get_path(rel)`, `offset[이름]={x,y,w,h,r,a}` [b1 §3.3]. ModernChic 사용: get_path 238, offset 35, option 7 | 없음 | 본체 패스 직전 전역 주입 | M | `loader/lua_skin.rs`(신규) |
| C5 | Lua 값 → 스킨 모델 변환 | 필드명 일치 키만 대입, 모르는 키 무시, int 는 0 방향 절삭, boolean 은 Lua 진리값(숫자 0 도 참), String 은 숫자 문자열화, 배열은 구멍 메움 [b1 §4, r1 §3.3] | 없음. serde 는 실수·숫자 id·함수 값을 받지 못함 [r1 §3.3] | 전용 변환기 | L | `loader/from_lua.rs`(신규) |
| C6 | 함수 값 속성 | `draw`/`value`/`timer`/`act`/text value 에 함수 → 프레임마다 호출. 오류 시 기본값(false/0/""/OFF), 다음 프레임 재호출. 값 없는 `draw` 는 false(프레임 훅 용도). timer 함수가 nil 이면 0(켜짐) [b1 §5.1]. ModernChic: draw 327, value 128, act 35, timer 17 + `timer_observe_boolean` 148 [m1 요약] | 없음. `PropertyRef` 가 `Id`/`Expr` 뿐, 타이머 식은 경고 후 무시 [r1 §3.3, §6.2 D4] | `PropertyRef` 에 함수·이름 변형, 타이머 참조형, 프레임당 1회 상태 바인딩 | L | `model.rs`, `dst.rs`, `lua/mod.rs` |
| C7 | `main_state` | 읽기 26종 + 부수효과 함수. 시각·타이머는 마이크로초, 꺼짐은 -2^63 [b1 §6]. ModernChic 사용 20개: option 230, number 207, audio_play 82, text 62, time 22, timer 16, timer_off_value 15, event_index 14, gauge_type 11, judge 6 등 | 없음. 고유 이름 `skin.*` 6함수, ms 단위 [r1 §5.1] | 모듈 신설, 단위 정합 | L | `lua/main_state.rs`(신규), `property/mod.rs` |
| C8 | `timer_util`/`event_util` | `timer_observe_boolean(f)` 는 f 가 참이 된 시각을 래치, 거짓이면 OFF. 반환 함수마다 독립 상태 [b1 §7.1] | 없음 | Lua 프렐류드로 구현(Rust 상태 불필요) | S | `lua/prelude.lua`(신규) |
| C9 | luajava facade | `bindClass("java.io.File")` + `new(File,path):mkdir()/listFiles()`, `Gdx.input:isKeyPressed(Input.Keys.X)`, `URL` 계열 [b1 §2.3]. `Root/customfunction.lua:7-8` 이 모든 화면의 본체 패스에서 pcall 밖으로 로드되어 없으면 전 화면 실패 [m2 요약, m3 요약] | 없음 | 최소 facade. 키 코드는 원작 의도(UP 19, DOWN 20, LEFT 21, RIGHT 22)로 제공(6절 결정 7) | M | `lua/luajava.rs`(신규) |
| C10 | 파일 쓰기 | 스킨 폴더 안에서 w/a 허용, 부모 폴더 자동 생성 [b1 §2.2]. ModernChic 은 기본 설정에서도 구성 시마다 `History/information.txt` 를 덮어쓰고(선곡은 pcall 밖) `io/**`·`History/**` 를 읽는다 [m1 §9, m4 §14-1] | 없음 | 쓰기 오버레이(2절 a) | M | `lua/io.rs`(신규) |
| C11 | pcall 실패 가시화 | 원본은 부품 로드를 `pcall(dofile(...).load())` 107회로 감싸 실패 시 조용히 생략 [m1 §2.5] | 없음. 경고 래치가 샌드박스당 1개 [r1 §5.4 S14] | pcall 실패 로그, 함수별 1회 경고 + 횟수 집계 | S | `lua/env.rs`(신규) |
| C12 | destination 모델 | 배열 순서 = z 순서, 항목 하나 = 객체 하나. 첫 키프레임 기본값, 이후 상속. acc/blend/filter/center/timer/loop 는 객체 단위 첫 비영 값 [b2 요약, §18] | 있음. 보간·루프·오프셋·클립 일치 [r1 §6.1]. 차이: acc 결정 순서(D8) | acc 를 선언 순서 첫 비영 값으로 | S | `dst.rs`, `loader/track.rs` |
| C13 | 그리기 조건 | 정수 op 는 AND. 내장 불리언이면 조회(음수 부정, 정적이면 로드 시 1회 평가 후 제거), 내장이 아니면 스킨 옵션 맵 대조, 어디에도 없으면 부호와 무관하게 제거 [b1 §5.3, r1 §6.2 D2·D3] | 부분. 미지 id 를 매 프레임 `state.boolean` 으로 읽어 음수면 표시 | 미지 id 제거, 정적 1회 평가, 구현 없는 내장 id 구분(`Option<bool>`) | M | `loader/track.rs`, `dst.rs`, `property/mod.rs` |
| C14 | 타이머 | 0..2999 내장(µs, OFF = `Long.MIN_VALUE`), 그 밖은 커스텀. 화면 전환 시 전부 OFF + 시계 0 [b4 §1.4] | 부분. ms `i64`/`Option`, 전환 시 미초기화, 앱 전체 시계 [r1 §6.2 D5, r3 §6.1] | µs 저장, 장면 시계, 전환 리셋 | M | `timer.rs`, `dst.rs`, `R:apps/rbms-player/src/lib.rs`, `skin_screen.rs` |
| C15 | 오프셋 | `x += off.x - off.w/2`, `y += off.y - off.h/2`, `w += off.w`, `h += off.h`, `angle += off.r`, 알파 가산. 런타임 갱신은 id 1,2(스크래치 각도),3,4,5 뿐 [b5 §1-14] | 부분. `r` 부호 반대(D1), 1·2 없음, `OFFSET_ALL`(10) 없음 [r1 §6.2 D1·D13] | 부호 수정, 스크래치 각도, 전역 변환 | S~M | `dst.rs`, `skin_render/mod.rs` |
| C16 | stretch 11종 | TRIMMED 계열은 소스 영역도 자름 [r1 §6.3] | 부분. 0·1·2·9 만. ModernChic 은 3 을 10곳, 5 를 12곳 사용 | 7종 추가, 반환을 (목적지, 소스) 쌍으로 | M | `loader/stretch.rs`, `skin_render/draw.rs` |
| C17 | image/imageset | `ref` 는 NUMBER 와 별개인 이미지 인덱스 공간. 음수 ref 는 미표시, 범위 초과는 0번. 프레임 = `(time*len/cycle)%len`, time 은 장면 경과 - image.timer [b2 요약, b5 §1-5] | 부분. `state.integer` 로 읽음, 음수면 0번을 그림 [r2 §5.1] | 이미지 인덱스 종류 추가, 음수 숨김 | M | `skin_render/object.rs`, `draw.rs`, `property/mod.rs` |
| C18 | 참조 이미지 | destination id 가 음수 정수: -100 스테이지파일, -101 BACKBMP, -102 배너, -110 검정 1x1, -111 흰색 1x1 [b2 요약] | 없음. 본체 분기 없음, 프레임에 실을 텍스처가 1장뿐 [r2 §5.3] | 본체와 참조 텍스처 세트 | M | `skin_render/refs.rs`(신규), `mod.rs` |
| C19 | value/floatvalue | 프레임 수 24배수 → 부호 12+12, 10배수 → 10장, 그 외 11장(패딩 2 강제). `MIN_VALUE` 는 미표시. SkinNumber 와 SkinFloat 의 align 0/1 의미가 반대 [b2 요약] | 있음(식 일치) [r2 §5.1]. 단 어댑터가 센티널을 돌려주지 않아 0 을 그림 [b5 §16-2] | 센티널 공급(C22) | S | `skin_render/draw.rs` |
| C20 | 텍스트(TTF) | 글자 크기 = dst h, x 는 정렬 기준점, y 상단 기준, overflow 0/1/2, wrapping, 그림자(rgb/2, +x/-y), 직전 객체 blend 상속 [b2 요약, r2 §6.4] | 부분. 논리 720p 래스터, 패밀리 전환마다 캐시 전체 삭제, overflow/그림자/wrapping 없음 [r2 §6] | 아틀라스 쿼드 + 물리 픽셀 래스터, 의미론 추가 | L | `crates/rbms-render/src/font.rs`, `glyph_atlas.rs`, `skin_render/text.rs`(신규) |
| C21 | 텍스트(.fnt) | BMFont 텍스트 형식, `scale = size*dw/원본size`, type 0 표준·1/2 distance field(외곽선·그림자 유니폼) [r2 §6.4] | 없음. `load_font` 가 비폰트 데이터에 직전 face 를 성공으로 돌려줄 수 있음 [r2 §6.3] | 후속 웨이브. ModernChic 기본 옵션은 TTF [m6 §8.6] | L | `crates/rbms-render/src/bitmap_font.rs`(신규) |
| C22 | 값 없음 센티널 | NUMBER `Integer.MIN_VALUE`, FLOAT `Float.MIN_VALUE`, 타이머 `Long.MIN_VALUE`. ModernChic Lua 가 -2147483648 을 직접 비교 [b5 §1-7] | 없음. 미구현 = 0/false/"" [b5 §16-2] | 호스트 계약에 반영 | M | `property/mod.rs`, 호스트 구현 |
| C23 | 이벤트·writer | `act`(image/imageset), slider `event`(FloatWriter), text `event`(StringWriter), `event_exec`. Lua 이벤트 함수는 인자 1개로 호출 [b1 §5.2] | 없음. 이름 붙은 hotspot 8종뿐 [b5 §1-6] | 이벤트 실행기와 writer | M | `property/mod.rs`, `skin_render/input.rs`(신규), 앱 |
| C24 | 마우스 | press: z 역순, clickevent 가 있고 draw 인 첫 객체만. click 0~3 규칙. drag 는 슬라이더만. mouseRect 는 호버 표시 조건 [b2 요약] | 부분. 좌클릭 press 만, 휠·드래그·release·IME 없음 [r3 §7.1] | 입력 이벤트 확장과 디스패치 | M | `R:apps/rbms-player/src/lib.rs`, `stage/mod.rs`, `skin_render/input.rs`(신규) |
| C25 | 해상도 | `dw = 출력폭/스킨폭`, `dh` 독립. 스킨 w/h 가 Resolution 열거값일 때만 인정 [b1 §11] | 부분. 논리 1280x720 상수, 1920 문서를 2/3 로 줄였다가 창으로 늘림 [r2 §4] | 논리 크기 런타임화 | M | `apps/rbms-player/src/gpu/mod.rs`, `batch.rs`, `lib.rs` |
| C26 | 텍스처 수명 | 객체가 참조한 source 만 지연 로드, 화면 이탈 시 해제 [r2 §3.4] | 없음에 가까움. 선언 source 전부 선행 로드, 화면 간 미해제, 한도 확인 없음. ModernChic PNG 전부 = 2,794.8 MiB [r2 §3.4] | 텍스처 관리자 | M~L | `skin_render/textures.rs`(신규), `apps/rbms-player/src/skin_screen.rs`, `assets.rs` |
| C27 | 색 공간 | 바이트 통과(sRGB 설정 없음) | 불일치 추정. 표면 sRGB + `Rgba8Unorm` → 밝게 나옴(코드 근거, 실화면 미확인) [r2 §2] | 결정 반영(6절 결정 8) | S | `gpu/mod.rs`, `batch.rs` |
| C28 | filter | `filter != 0` 이고 크기가 다를 때만 전용 bilinear(색만 알파 가중 쌍선형, 알파 최근접) [b2 요약] | 근사(하드웨어 Linear), 1:1 판정이 논리 좌표 [r2 §1.4] | C25 뒤 물리 픽셀 판정. 셰이더 완전 재현은 후속(ModernChic FILTER.ON 3곳) | S | `loader/stretch.rs`, `draw.rs` |
| C29 | 동영상 소스 | 확장자가 영상이면 `SkinSourceMovie`, 첫 prepare 부터 반복 재생, 프레임 없으면 미표시 [b2 요약] | 없음. 디코더 의존성 없음 [r2 §3.5] | 후속 웨이브(6절 결정 6). 그 전에는 "프레임 없음 = 미표시" | L | 신규 모듈, `Cargo.toml` |
| C30 | 스킨 발 사운드 | `main_state.audio_play(path, vol)`: 0..2 클램프 x 시스템 볼륨, 0.0001 은 프리로드 [m1 §9] | 없음. 시스템 사운드는 1회 재생만, 스킨용 버스 없음 [r3 §10.1] | 스킨 버스와 경로 캐시 | M | `apps/rbms-player/src/syssound.rs`, `crates/rbms-audio` |
| C31 | 헤더·설정 병합 | 옵션은 이름으로, 값 -1 은 무작위. 파일은 설정이 있을 때만 선택, `def` 는 설정 UI 에서만 적용. 플레이 타입에 오프셋 4개(10, 30, 32, 33) 자동 추가 [b1 §10] | 부분. `def` 를 문자열 그대로 치환해 확장자 없는 경로가 됨, 자동 오프셋 없음, type 9·15 거부 [r1 §4.2 L6·L11·L14] | `def` 스템 대조, 무작위 -1, 자동 오프셋, 타입 표 확장 | M | `resolve.rs`, `loader/branch.rs`, `loader.rs` |
| C32 | 프레임 순서 | 시각 확정 → 화면 로직 → 커스텀 타이머 → 커스텀 이벤트 → 전 객체 prepare(조건 → 타이머 → 값) → 그리기 → 클릭 [b1 §8.2] | 부분. prepare 와 draw 가 객체별로 붙어 있고 커스텀 타이머·이벤트 없음 | 2단계 분리. ModernChic 은 customTimers/customEvents 가 빈 테이블 [m1 §2.6, 직접 확인] | M | `skin_render/mod.rs`, `frame.rs`(신규) |

### 1.2 플레이 SP (play7_hw type 0, play5_hw type 1)

| # | 기능 | 레퍼런스 의미론 요약 | R-BMS 현황 | 필요한 작업 | 크기 | 담당 파일 |
|---|---|---|---|---|---|---|
| P1 | 상태기계 | PRELOAD → READY(타이머 40) → PLAY(41, 140) → FAILED(3, `close` 후 종료) / FINISHED(908 → `finishmargin` → 2 → `fadeout`) [b3 §10.2]. ModernChic: loadend 3500, playstart 1000, close 3000, fadeout 500 [m1 §3] | 없음. 진입 즉시 PLAY, 종료·실패 즉시 결과 [r3 §6.2] | 상태기계와 헤더 시간 필드 소비. 로딩은 PRELOAD 로 이동 | L | `stage/play/mod.rs`, `stage/loading.rs` |
| P2 | 헤더 필드 적용 조건 | close/loadend/playstart/judgetimer/finishmargin 은 note 객체가 destination 에 있을 때만 적용 [b3 요약] | 없음 | 로더 규칙 | S | `loader/lua_skin.rs`(신규) |
| P3 | note | 레인 수 = note 배열 길이. y 는 section 차 x scroll x `(hu-hl)*hispeed*speed` 누적, hu/hl 은 0번 레인 기준. 구식 명명: lnbody = 누르는 중, lnactive = 아님, hcnbody/active/damage/reactive. LN 은 본체·끝·시작 순, 클립 없음. 일찍 친 노트는 판정선까지 유지 [b3 요약] | 부분. HCN 필드 없음, processed·expansion 미구현, 위치가 내장 `Skin` 의 `judge_y`/`top_y` 의존 [r2 §5.2] | 문서 기반 레인 지오메트리, LN/HCN 매핑, 마디선 4종 | L | `skin_render/notes.rs` |
| P4 | 리프트·레인 커버·히든 | drawLane 이 오프셋 3/4/5 를 매 프레임 기록. hiddenCover 는 오프셋 3·5, liftCover 는 3 자동 부착, `disapearLine` 아래 scissor. 레인 커버는 slider type 4 또는 offset 4 이미지 [b3 요약] | 부분. 오프셋 3/4/5 공급됨, 계산이 내장 `Skin` 의존 [r2 §5.2, b5 §16-24] | 내장 필드 의존 제거, 값 id 14/314/315/316/312/313 공급 | M | `skin_render/covers.rs`, 호스트 play 군집 |
| P5 | gauge | 36칸 = 종류*6 + 상태. nodes 길이 4/8/12/36. 칸 선택식과 type 3 점멸 [b3 요약] | 부분. 36칸 전개 있음, 애니메이션을 결정적 함수로 대체, 경계가 내장 `Skin` 의존 [r2 §5.2] | 칸 선택식 정합, 의존 제거 | M | `skin_render/gauge.rs` |
| P6 | judge | images 최대 7, numbers 는 영역 콤보(PG/GR/GD 만), shift 는 숫자 폭 절반 이동. 타이머 46/47/247, 446~448, `judgeregion` = 최대 index + 1 [b3 요약]. ModernChic SP 는 index 0 을 3세트, `offsets = {LIFT, JUDGE_1P}` [b3 §13] | 부분. 콤보를 `NUMBER_COMBO` 로 강제, 플레이어 접기 [r2 §5.2] | 영역 콤보, MAX 이미지, BD 이하 숫자 없음 | M | `skin_render/judge.rs` |
| P7 | 레인별 타이머 | id = base + 스킨 오프셋 + player*10. 봄 50, 홀드 70, 키 on 100, 키 off 120. 봄은 `judge <= judgetimer` 마다 재시작하고 끄지 않음. READY 는 켠 채 유지 [b3 §11.4, b5 §16-17·21] | 부분. 봄을 연소 구간으로 켜고 끔, READY 를 끔, 42 를 켬(레퍼런스는 안 켬) | 드라이버 정합. STARTINPUT, RHYTHM(1박 = 1000), ENDOFNOTE(143), SCORE_A/AA/AAA/BEST/TARGET(348~352), MUSIC_END(908), FADEOUT(2) 추가 | L | 호스트 `play_timers.rs`(신규), `skin_render/screen.rs` 철거 |
| P8 | 플레이 값 id | ModernChic SP 참조: 옵션 약 50종, 숫자 약 75종, 타이머 약 45종, 문자열 6종, 이벤트 308/55/74 [m2 요약]. early/late(1242/1243)는 PG 제외, NOW 랭크(340~347)와 1102 는 통과 노트 기준, 1107 은 0..100, 407 은 소수 1자리 [b5 §1-11] | 부분. 정의 불일치 다수 [b5 §16-4~16] | 군집 B·C·D 구현과 정의 교정 | L | 호스트 `score.rs`, `play.rs`(신규) |
| P9 | bga | PLAY 전 검정, 본 BGA 위 레이어(순수 검정 투명), 미스 레이어, 256 캔버스 보정, `bgaExpand` [b3 요약] | 부분. 프레임 1장, 로드 조건이 내장 `Skin.bga` [r3 §10.2] | 레이어 분리, 조건을 문서 bga 객체로 | M | `skin_render/bga.rs`(신규), `assets.rs`, `app_play.rs` |
| P10 | 그래프 4종 | judgegraph(초당 버킷 4x4 칩, type 0/1/2, 750ms 갱신), bpmgraph(로그 계단선), timingvisualizer(빠름 = 오른쪽), hiterrorvisualizer(빠름 = 왼쪽). 뒤 두 개의 width 는 스케일 미적용 [b3 요약] | 부분. judgegraph 는 다른 모양, bpmgraph 는 결과에서만 데이터 [r2 §5.2] | 레퍼런스 텍스처 규칙으로 재작성 | L | `skin_render/graphs/*`(분할) |
| P11 | 로드 시점 분기 | 본체가 `main_state.option(170 NO_BGA)`, 32, 난이도, `text(TABLE_FULL)` 을 로드 시 읽어 구조를 바꿈 [m2 요약, m1 §10] | 없음 | 곡 확정 뒤 스킨 구성, 화면 진입마다 재구성 | 구조 | 앱 장면 수명 |
| P12 | 5키 전용 | 7키 키 베드 시트 재사용 + `5keysFrame` 으로 2키분 가림. 폭발 LN 행 오프셋이 7키와 다른 순서(원본 결함) [m2 요약] | 해당 없음 | 수정 없이 실행하면 자동 재현 | - | - |
| P13 | 내장 `Skin`(RON) 의존 | 해당 없음 | BGA 디코드 조건, 레인 → 타이머 키 번호, 판정 측, 봄 지속, 결과 팔레트가 내장 `Skin` 을 읽음 [r3 §11-4] | 모드 데이터에서 직접 계산 | M | `app_play.rs`, `app_input.rs`, `stage/play/mod.rs`, `crates/rbms-render/src/skin.rs` |

### 1.3 플레이 DP (play14_hw type 2, play10_hw type 3)

| # | 기능 | 레퍼런스 의미론 요약 | R-BMS 현황 | 필요한 작업 | 크기 | 담당 파일 |
|---|---|---|---|---|---|---|
| D1 | 레인 인덱스 | 14K = [1P 1-7, 1P SC, 2P 1-7, 2P SC], 10K 같은 패턴. 스킨 오프셋 표 `1..7,0,1..7,0`, player = lane/(레인 수/2) [b3 §12] | 부분. 내장 x 좌표 정렬로 번호 부여 [r3 §6.2] | 모드 표 기반 번호 | S | 호스트 `play_timers.rs`(신규) |
| D2 | judge 2영역 | index 0/1 두 객체, 타이머 46/47, 콤보 446/447. gauge 는 하나, group 은 2개 [b3 §12] | 부분 | P6 과 같이 | - | `skin_render/judge.rs` |
| D3 | 2P 레인 타이머 | 2P 는 +10(오프셋 10 미만) [b3 §11.4] | 있음(60~69 등 구동) [b5 §2.6] | 의미 정합만(P7) | - | - |
| D4 | DP 본체 | 부품 17개, destination 약 375(14키)·357(10키), judge 10, timer 함수 1곳, act 함수 1곳(리프트 토글) [m3 요약]. `inputkey.lua` 우측 스크래치 x 407 과 `notes.lua` 408 의 1px 차이는 원본 그대로 [m3 요약] | 해당 없음 | 수정 없이 실행 | - | - |
| D5 | 10키 전용 | `10keysFrame` 2개(옵션 954), 14키만 `pmchara = {}` [m3 §10] | pmchara 모델 있음, 미사용 | 빈 배열 허용 | - | - |

### 1.4 선택 (musicselect type 5)

| # | 기능 | 레퍼런스 의미론 요약 | R-BMS 현황 | 필요한 작업 | 크기 | 담당 파일 |
|---|---|---|---|---|---|---|
| S1 | songlist(SkinBar) | 본체는 imageset 필수, images 인덱스 = 막대 종류 0 곡, 1 폴더, 2 표/해시/랜덤, 3 코스, 4 곡 없음, 5 커맨드, 6 검색. 텍스트 11종, 레벨 7, 램프 11, 트로피 3, 라벨 5, 분포 그래프 1. 하위 객체는 막대 기준 상대 좌표 [b4 요약] | 부분. 바 셀 0 고정, 램프는 단색 사각형, trophy·rivallamp·그래프 없음, 내용이 색·문자열로 미리 풀린 `SelectRow` [r2 §5.2] | 전면 재작성 | L | `skin_render/songlist.rs` |
| S2 | 스크롤 보간 | 벽시계 기준 선형, x 는 [-1,1] 클램프, y 미클램프, int 절삭, 키 유지 300ms 후 50ms 반복 [b4 §0-6] | 없음(바 애니메이션 없음) | 식 그대로 구현 | M | `skin_render/songlist.rs`, `stage/select/mod.rs` |
| S3 | 막대 모델 | SongBar/FolderBar/TableBar/HashBar/GradeBar/CommandBar/SearchWordBar 등 [b4 §3.9] | 부분. `SelectItem::{Song, Folder}` 둘, 램프가 색 값 [r3 §6.3] | 막대 종류·램프 id·난이도·라벨·트로피를 가진 모델 | L | `stage/select/scene.rs`, `list.rs`, `lib.rs` |
| S4 | 타이머 | 실제로 켜는 것은 1, 11, 21~23, 31~33, 172~174. 10/12/13/14 는 켜지 않음 [b4 §0-7] | 불일치. 10/12/13 을 켜고 22/23/32/33/1 은 없음 [b5 §1-8] | 정합 | S | 호스트 select 군집 |
| S5 | 패널 1/2/3 | START = 1, SELECT = 2, NUM5 또는 START+SELECT = 3. 누르는 동안만 열림. op 21/22/23. ModernChic 은 PANEL2/3_ON 을 58회 참조 [b4 §3.4, b5 §15] | 부분. 패널 1 하나(LeftShift/F1), 내용은 고유 11행 [r3 §6.3] | 패널 상태와 옵션 이벤트 표(11, 12, 40, 42, 43, 54, 55, 72, 74, 75, 78, 308, 330~332, 340~342, 400 등) [b4 §3.6] | L | `stage/select/mod.rs`, `app_options.rs`, 호스트 options 군집 |
| S6 | 이미지 인덱스 ref | `ref = MAIN.BUTTON.*` 42회, `event_index` 14회. 380~389/390~399 는 값과 인덱스에서 뜻이 다름 [b5 §1-5, m4 요약] | 없음 | C17 과 군집 E | M | 호스트 `options.rs`(신규) |
| S7 | 슬라이더 | type 1(목록 위치, 드래그), 8(IR 순위 위치), 17/18/19(볼륨) [m4 §9] | 부분. 그리기만, 조작 없음 | writer 와 드래그 | M | `skin_render/input.rs`(신규), 호스트 |
| S8 | 검색 | 문자열 30 은 읽기 빈 문자열·쓰기 `search()`. writer 가 있고 event 가 없는 text 는 자동 편집 가능, 클릭 시 입력 [b4 §3.7]. ModernChic 은 표시만 | 부분. 내장 검색 상자 | 편집 텍스트 객체와 기존 `TextEdit` 연결 | M | `skin_render/text_input.rs`(신규), `apps/rbms-player/src/textedit.rs` |
| S9 | 곡 정보·통계 id | 숫자 177종 중 R-BMS 가 채우는 것 30종, 불리언 139종 중 23종, 문자열 60종 중 7종 [b5 §1-4] | 부분 | 군집 A·F·I 구현(b5 §15 표를 사용 횟수 순으로) | L | 호스트 `chart.rs`, `select.rs`, `system.rs`(신규) |
| S10 | IR·라이벌·리플레이 슬롯 | 타이머 172~174, 랭킹 값 380~399, op 196~1208, 603~608, 625 [b4 §3.10] | 없음(내장 패널만) | 1차는 오프라인 고정값, 실연결은 후속(6절 결정 10) | M | 호스트 `ir.rs`(신규) |
| S11 | 사운드 | SELECT BGM 루프, 막대 이동 SCRATCH, 패널 OPTION_OPEN/CLOSE/CHANGE [b4 §1.6] | 부분. 1회 재생만, 이동음 없음 [r3 §10.1] | 루프·정지 API 와 호출 지점 | M | `syssound.rs`, `crates/rbms-audio` |
| S12 | 본체 규모 | 18개 섹션, 기본 옵션 destination 1914(캐릭터 제외 912), draw 클로저 약 1380, 타이머 함수 약 420 [m4 요약] | 해당 없음 | 프레임 예산이 이를 감당해야 함 | - | `lua/budget.rs`(신규) |
| S13 | NaN | `score.lua:10-11` 의 0/0 그래프 값 [m4 요약] | 미확인 | 그래프 값 NaN 은 0 | S | `skin_render/draw.rs` |
| S14 | 장면 수명 | `input` 만 사용(STARTINPUT). scene/fadeout 미사용, 결정으로 즉시 전환 [b4 §0-4] | 없음 | 장면 수명 공통 헬퍼 | S | `stage/scene_life.rs`(신규) |

### 1.5 결정 (decide type 6)

| # | 기능 | 레퍼런스 의미론 요약 | R-BMS 현황 | 필요한 작업 | 크기 | 담당 파일 |
|---|---|---|---|---|---|---|
| E1 | 독립 장면 | `nowtime > input` → STARTINPUT. `nowtime > scene` → FADEOUT. FADEOUT 경과 > `fadeout` → PLAY(취소면 선택). 건너뛰기는 STARTINPUT 뒤 키 인덱스 0/2/4/6 눌린 상태 또는 ENTER, 취소는 ESC 또는 START+SELECT. 진입 시 DECIDE 사운드 [b4 §4] | 없음. 로딩 화면을 decide 문서로 그림, 스캔·테이블 받기 중에도 그림, DECIDE 를 로딩 완료 시 재생 [r3 §6.5] | Decide Stage 신설, 로딩은 병행 후 PRELOAD 로 | M | `stage/decide.rs`(신규), `stage/loading.rs` |
| E2 | 상태 요구 | option 150-155 중 정확히 하나 true(아니면 `diffRGB()` nil → 본체 로드 실패), number 96, string 10-15/1003, -100, judgegraph, bpmgraph, timer 2 [m5 §2.2, 부록 B] | 부분(제목류만) | 군집 A. 난이도 없으면 150 을 true | S | 호스트 `chart.rs`(신규) |
| E3 | 로드 시점 스냅샷 | 난이도 색, tips(무작위), lockon 시작색(무작위 3값) [m5 §2.4] | 없음 | 진입마다 재구성. 검증 시 시드 고정 | - | `lua/env.rs`(신규) |
| E4 | 그래프 | 노트 분포 + BPM 그래프가 선택·결정에서도 그려짐 | 부분. bpmgraph 가 결과 프레임에서만 데이터 [r2 §7.2-6] | 프레임 데이터를 능력별 선택 필드로 | M | `skin_render/frame.rs`(신규) |
| E5 | 본체 | 파트 파일 없이 내부 함수 5개, destination 약 69 [m5 요약, m1 §2.6] | 해당 없음 | - | - | - |

### 1.6 결과 (result type 7)

| # | 기능 | 레퍼런스 의미론 요약 | R-BMS 현황 | 필요한 작업 | 크기 | 담당 파일 |
|---|---|---|---|---|---|---|
| T1 | 장면 수명 | 150/151 을 첫 프레임에 동시, 152 는 ranktime == 0 이라 첫 프레임(JSON/Lua 는 ranktime 설정 불가). input 2500 동안 입력 잠금, FADEOUT 뒤 전환 [b4 §0-8] | 불일치. 151 을 1초 뒤, 152 를 신기록 때만, 키 즉시 전환 [b5 §16-20] | 정합 | M | `stage/result.rs`, `app_result.rs` |
| T2 | 입력 | 키 0~3 OK, 4 REPLAY_DIFFERENT, 5 CHANGE_GRAPH, 6 REPLAY_SAME, NUM1~4 리플레이 저장(이벤트 19/316/317/318), 휠은 랭킹 오프셋. CHANGE_GRAPH 는 0~5 에서 (g+1)%6 [b4 요약] | 없음 | 입력 표와 게이지 종류 전환 | M | `stage/result.rs` |
| T3 | gaugegraph | delay 1500ms·선폭 2px 고정, 드러남은 화면 경과 시간 기준. 색 24개 또는 이름 14색. 게이지 종류별 이력 필요 [b4 §0-9] | 부분. groove 쌍만, 게이지 로그 1종 [r2 §5.2, r3 §6.4] | 재작성과 엔진의 전 종류 이력 기록 | L | `skin_render/graphs/gauge_graph.rs`(신규), 플레이 엔진 |
| T4 | gauge 객체 | 결과에서도 gauge(36 노드) 표시 | 없음. 결과 프레임에 플레이 상태가 없어 미표시 [r2 §7.2-6] | E4 와 같은 프레임 구조 변경 | S | `skin_render/gauge.rs` |
| T5 | 메뉴 전환 | Info 버튼 `act` 와, 0x0 BLACK 객체의 `draw` 함수 안에서 `Gdx.input:isKeyPressed` 폴링 → `timer_observe_boolean` 으로 메뉴 1/2 전환 [m5 §1-7·8] | 없음 | C6, C8, C9, C23 이 갖춰지면 수정 없이 동작 | - | - |
| T6 | 상태 요구 | option 90/91, 300-307, 320-327, 330-332, 180-184, 196-204+1196-1204, 150-155, 160-164+1160/1161, 191, 280-282+289+290, 51, 606. number 약 55개. string 2, 10, 12-15, 120-129, 150-159, 1003, 1020, 1030, 1031. 이미지 인덱스 370, 42, 43, 90, 301-307. timer 2, 172-174 [m5 부록 A] | 부분(약 12개 정수, 랭크·클리어) | 군집 B·G·H | L | 호스트 `score.rs`, `result.rs`, `ir.rs`(신규) |
| T7 | 로드 순서 | 스코어 DB 갱신 뒤 스킨 로드. 제목·날짜·플레이어명·코스 여부가 로드 시 문자열로 굳음 [m5 §2.4] | 없음 | 결과 확정 뒤 구성 | - | `stage/result.rs` |
| T8 | 그래프 | judgegraph 3종, bpmgraph, timingdistributiongraph(평균·편차선) [m5 요약] | 부분 | P10 과 공유 | - | `skin_render/graphs/*` |
| T9 | 사운드 | CLEAR/FAIL(루프 설정), FADEOUT 시작에 CLOSE, 이탈 시 정지 [b4 §1.6] | 부분(1회 재생) | S11 과 공유 | - | `syssound.rs` |
| T10 | 미정의 이벤트 | `act = 370/371` 은 오류 없이 무시 [m5 §17-10] | 없음 | 실행기가 미정의 id 를 무시 | S | 호스트 이벤트 |

### 1.7 코스 결과 (course type 15)

| # | 기능 | 레퍼런스 의미론 요약 | R-BMS 현황 | 필요한 작업 | 크기 | 담당 파일 |
|---|---|---|---|---|---|---|
| G1 | 타입 로드 | type 15 | 없음. 로더가 거부, Stage 가 내장 텍스트만 [r3 §1.4] | 타입 허용과 스킨 호출 | M | `loader.rs`, `stage/course_result.rs` |
| G2 | 장면 | 150/151/152 를 조건 없이 첫 프레임, FADEOUT 뒤 항상 선택. CHANGE_GRAPH 는 (g-5)%3+6. 사운드는 COURSE_* 없으면 RESULT_* [b4 요약] | 없음 | T1 헬퍼 재사용 | S | `stage/course_result.rs` |
| G3 | 상태 | 코스 곡 제목 150~159, 게이지 이력을 곡별로 이어 붙이고 경계에 흰 세로선 [b4 요약] | 없음 | 코스 합산 공급 | M | 호스트 `result.rs`(신규) |
| G4 | 본체 | result 와 같되 impression·diflist 없음, destination 약 175, property 17, filepath 25 [m5 요약] | 해당 없음 | - | - | - |

### 1.8 키 설정 (keyconfig type 8)

| # | 기능 | 레퍼런스 의미론 요약 | R-BMS 현황 | 필요한 작업 | 크기 | 담당 파일 |
|---|---|---|---|---|---|---|
| K1 | 화면 | 레퍼런스도 스킨 기반이 아님. 내장 UI 위에 스킨 덧그리기. ModernChic keyconfig 는 `Decide.lua.require.textproperty` 가 `PROPERTY.isOutlineFont` 를 불러 본체 로드 실패, 성공해도 destination 비어 있음 [b4 §0-10, m1 §2.7, `M:keyconfig.lua:19` 직접 확인] | 내장 UI 있음 | 헤더만 인식, 내장 UI 유지. 본체 실패는 경고만 | S | `stage/keyconfig.rs` |
| K2 | 문자열 id | 40~49, 240~283 은 `getKeyAssign(id-240+10)` [b5 §1-12] | 결함. 10칸 어긋나고 50~239 에도 키 라벨 반환 | 교정 또는 어댑터 철거 | S | 호스트 `keyconfig.rs`(신규) |

### 1.9 스킨 선택 (skinselect type 9)

| # | 기능 | 레퍼런스 의미론 요약 | R-BMS 현황 | 필요한 작업 | 크기 | 담당 파일 |
|---|---|---|---|---|---|---|
| N1 | 화면 | 완전 스킨 기반. 이벤트 190(스킨 변경), 220~228(n 번째 항목 값 순환, 229 는 `< 229` 조건으로 무시), 170~185·386~388(종류 선택), 문자열 50/51/100~119, 이미지 인덱스 170~, 슬라이더 type 7, 휠 [b4 §0-11] | 없음. Stage 없음, type 9 거부. SETTINGS 의 SKIN 탭이 역할 대행 [r3 §1.4] | Stage 신설 또는 SKIN 탭 유지(6절 결정 5) | M | `stage/skin_config.rs`(신규) |
| N2 | 본체 | source 2, font 2, text 22, image 58, imageset 18, slider 1, destination 108, Lua 함수 값 0건. skinSelect·skinpreview 미사용 [m6 요약] | 해당 없음 | 군집 J | M | 호스트 `skin_config.rs`(신규) |
| N3 | 저장 모델 | 화면 타입별 `{path, option[], file[], offset[]}` 이름 기준 [r3 §4] | 있음(`custom[문서경로]` 가 1:1) | 팩 단위 일괄 지정 추가 | M | `crates/rbms-config/src/schema.rs`, `skin_select.rs` |

---

## 2. 목표 구조 제안

### 2.1 크레이트별 책임

| 크레이트 | 맡는 것 | 버리는 것 |
|---|---|---|
| `rbms-skin` | 스킨 문서 모델(`SkinDef`), Lua 런타임, 2패스 로더, Lua 값 변환, 경로·설정 병합, destination 보간, 타이머 저장소, 호스트 계약(`SkinHost` 트레이트) | 혼합 합성 확장 필드, `skin.*` 6함수, 한 줄 식 전용 예산 |
| `rbms-render` | `Renderer` 프리미티브, 텍스트·비트맵 폰트, 스킨 객체 빌드와 2단계 prepare/draw, 클릭·드래그 판정(순수 기하), 텍스처 핸들 관리 | 화면별 상태 어댑터(`skin_render/state.rs`), 타이머 드라이버(`screen.rs`), `content.rs`, 내장 화면의 `*_with_content*` 진입점 |
| `rbms-player`(앱) | 장면 상태기계와 장면 시계, `SkinHost` 구현(군집별 파일), 타이머 드라이버, 이벤트 실행, 입력 분배, 스킨 팩 선택·설치, 자산 디코드 워커, 사운드 | 대체 요건 표, 프리셋 결합, `include_bytes!` 번들 표 |
| `rbms-config` | 스킨 팩 선택, 문서별 커스터마이즈, 해상도·창 모드 | `display.skin` 프리셋, 번들 공유 스코프 |
| `rbms-audio` | 루프·정지, 스킨 버스 | - |
| `rbms-cli` | 스킨 덤프 도구(객체 수·경고·pcall 실패) | - |

상태 어댑터를 렌더 크레이트 밖으로 빼는 이유: 수백 개 id 의 값은 점수 DB·설정·IR·코스처럼 앱만 아는 원천에서 온다 [r2 §7.2-2]. 별도 상태 크레이트를 새로 두는 안은 `rbms-store`/`rbms-ir`/`rbms-course`/`rbms-table`/`rbms-config`/`rbms-play` 를 모두 끌어와야 해서 앱 모듈(`apps/rbms-player/src/skin_host/`)로 두는 편이 의존이 단순하다.

### 2.2 판단 (a) Lua 런타임 전략

| 항목 | 결정 | 근거 |
|---|---|---|
| Lua 버전 | mlua 피처를 `lua54` → `lua52` 로 | LuaJ 는 5.2 계열이고 수가 double 하나다. 5.4 는 `10/2 .. ""` 가 "5.0", 정수 넘침 순환으로 꺼진 타이머(-2^63) 산술이 뒤집힘 [b1 §12]. ModernChic 은 정수 연결을 실제로 쓴다(`"STAGE" .. count + 1`, 날짜 문자열, [m5 §2.4]). 잠긴 mlua 0.12.2 에 `lua52 = ["ffi/lua52"]` 가 있고 lua-src 551.0.2 가 `lua-5.2.4` 를 동봉한다(직접 확인: `$CARGO_HOME/registry/src/index.crates.io-1949cf8c6b5b557f/mlua-0.12.2/Cargo.toml:69`, 같은 위치 `lua-src-551.0.2/lua-5.2.4`; `CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo`). 새 의존성 없음 |
| 상태 수명 | 스킨 로드 1회 = Lua 상태 1개. 헤더 패스와 본체 패스는 같은 상태. 화면 진입마다 새로 만든다 | 속성 모듈의 op 번호 카운터가 모듈 지역 상태라 비멱등 [m1 §3.1], 모듈 최상위가 `skin_config == nil` 에서 한 번만 돌아야 함 [b1 §3.2] |
| 헤더 전용 읽기 | 스킨 목록·설정 UI 용은 별도 상태에서 `main_state`/`timer_util`/`event_util` 을 빈 테이블로 두고 1회 실행. 결과를 파일 mtime 키로 캐시 | [b1 §14-2], 스캔마다 Lua 실행 비용 [r3 §5-2] |
| 표준 라이브러리 | base 전부(`print` 는 로그로), package(자체 검색기), table, string(패턴 함수 포함, `dump` 제외), math(+`pow`·`atan2` 등 5.2 기본), bit32, coroutine. `load` 는 텍스트 청크만 | [b1 §2.1], 바이트코드 차단 유지 [r1 §5.6] |
| `require` 탐색 | 스킨 루트만. 레퍼런스의 "작업 디렉터리 먼저"는 따르지 않는다 | 작업 디렉터리 탐색은 스킨 밖 파일 실행 통로가 된다. ModernChic 모듈 이름은 전부 스킨 루트 기준 [m1 §4-10] |
| `dofile`/`loadfile` | 절대 경로와 스킨 루트 상대 경로 허용, 정규화 뒤 루트 밖이면 오류 | `get_path` 가 절대 경로를 돌려준다 [m1 §2.5] |
| io 읽기 | 스킨 루트 안만. 없는 파일은 `nil, 메시지` | `existFile` 이 이 형태에 의존 [m1 §7.2] |
| io 쓰기 | 허용하되 스킨 폴더가 아니라 쓰기 오버레이로 보낸다. 오버레이는 `<설정 폴더>/skin-data/<스킨 식별자>/` 이고, 읽기·`listFiles` 는 오버레이를 먼저 본 뒤 스킨 루트를 본다. 경로 문자열은 스킨 루트 기준으로 유지 | 스킨이 읽기 전용 위치(동봉 폴더·앱 리소스)에 있어도 동작해야 한다 [m1 §9, r3 §3.3]. 원본은 기본 설정에서도 구성 시 `History/information.txt` 를 쓴다 [m1 §9] |
| os | `clock`, `date`(현지 시간), `difftime`, `time`, `setlocale` 만 | [b1 §2.1]. History 폴더명이 현지 날짜 [m1 §12-8] |
| luajava | `bindClass` 는 `java.io.File`, `com.badlogic.gdx.Gdx`, `com.badlogic.gdx.Input`, Controllers 계열만. `File:mkdir/listFiles` 는 오버레이 규칙 적용. `Gdx.input:isKeyPressed(code)` 는 호스트 키 상태 질의. `newInstance("java.net.URL")` 은 `connect()` 가 항상 실패 | ModernChic 의 URL 경로는 블록 주석으로 도달 불가 [m1 §9]. 호출부가 `connect` 를 pcall 로 감쌈 [b1 §13] |
| `main_state.http_*` | 항상 `nil, 메시지` | ModernChic 미사용 [b1 §6.2] |
| 난수 | 운영은 호스트 엔트로피로 시드, 테스트·캡처는 고정 시드 | 5.2 는 기본 고정 시드라 호스트가 넣어야 함 [b1 §12]. 원본은 비시드 [m1 §4-7] |
| 예산 | 로드 단계와 프레임 단계를 분리. 로드: 벽시계 수 초·메모리 수백 MiB 상한. 프레임: 호출 횟수 상한 없이 벽시계 상한만, 초과 시 그 프레임 나머지는 직전 값 유지. 수치는 덤프 도구 실측 뒤 확정 | 현 한도(식 20ms·프레임 4,096회·8MiB)로는 정상 스킨이 중단됨 [r1 §5.4 S8·S9]. 실측값 없음(미확인) |
| 오류 처리 | 레퍼런스처럼 기본값으로 대체하고 매 프레임 재호출. 로그는 함수별 첫 1회 + 누적 횟수 | 화면 결과는 레퍼런스와 같고 로그 폭주만 막는다 [b1 §14-6] |
| 상태 바인딩 | `main_state.*` 는 영구 Lua 트램펄린, 숨긴 host 테이블 필드만 프레임당 1회 `scope` 안에서 교체. 로드 단계도 같은 방식으로 호스트를 받는다 | `#![forbid(unsafe_code)]` 와 빌린 프레임 상태 [r1 §5.5] |
| 스레드 | 1차는 장면 전환 시 동기 로드(레퍼런스와 동일). 이미지 디코드는 기존 워커 풀. 덤프 실측에서 로드가 체감될 만큼 길면 mlua `send` 피처로 워커 이동을 검토 | `mlua::Lua` 는 `send` 없이 `!Send` [r1 §4.2 L18] |

### 2.3 판단 (b) 로드 결과 표현

기존 `SkinDef` 를 재사용하고 값 참조 타입만 넓힌다. 새 모델을 만들지 않는다.

- 근거: 미러가 `JsonSkin.java` 의 최상위 43필드와 하위 레코드·기본값을 전부 갖고 있고 빠진 객체 종류가 없다 [r1 §3.1~3.2]. 레퍼런스도 Lua 테이블을 `JsonSkin` 으로 옮긴 뒤 JSON 과 같은 경로를 탄다 [b1 요약]. `LoadedSkin`(sources, fonts, destinations, nested)과 트랙 빌더, dst 테스트 38건이 그대로 쓰인다.
- 변경점
  1. `PropertyRef` 에 `Func(LuaFnId)` 와 `Name(String)` 추가. 해석 순서는 함수 → 숫자 id → 이름 조회 → 실패 시 `return <문자열>` 컴파일 [b1 §4, b2 §17].
  2. `DestinationTrack.timer` 를 `Option<TimerRef>`(`Id` / `Lua`)로. 타이머 함수는 객체당 프레임 2회 호출되지만 결과가 같으므로 1회 호출 후 재사용해도 관측 차이가 없다 [b1 §7.1].
  3. `act`/`event`/`click` 을 실제로 소비하는 `EventRef`, `FloatWriterRef`, `StringWriterRef`.
  4. 속성 필드에 Lua boolean 이 오면 null(조건 없음) [m2 요약: `info.lua:186`].
  5. 함수 값은 Lua 레지스트리에 보관하고 `LoadedSkin` 이 Lua 상태를 소유한다(현 구조 유지, [r1 §5.6]).
- 프레임당 호출: 조건 통과 객체에 한해 선언 순서로 `op`/`draw` → `timer` → 값 함수. 첫 false 에서 중단 [b1 §5.3]. 부수효과가 있는 훅(fcSound, 키 폴링)이 이 순서에 의존한다.
- JSON/JSON5 경로는 유지한다. 레퍼런스 JSON 스킨도 같은 모델을 쓰고 `branch.rs` 테스트가 살아 있다. 다만 식의 전역 이름을 레퍼런스(`option`, `number` 등 전역 함수)로 맞추는 일은 후순위다 [r1 §5.1].

### 2.4 판단 (c) 속성·타이머·이벤트 공급

`rbms-skin` 에 호스트 계약을 두고 앱이 구현한다.

| 계약 메서드 | 반환·의미 | 레퍼런스 대응 |
|---|---|---|
| `boolean(id) -> Option<bool>` | `None` = 구현 없음(로더는 스킨 옵션 맵 대조로, Lua `option()` 은 false 로) | BooleanPropertyFactory null [b5 §2.3] |
| `is_static(id) -> bool` | 정적 불리언 분류. 생성기가 표로 내보냄 | `isStatic` [b5 §2.4] |
| `integer(id) -> i32` | 값 없음 = `i32::MIN` | [b5 §1-7] |
| `image_index(id) -> i32` | NUMBER 와 별개 공간 | `getImageIndexProperty` [b5 §9] |
| `rate(id) -> Option<f32>` / `float(id) -> f32` | 슬라이더·그래프는 RATE 만, `float_number` 는 FLOAT 우선 후 RATE | [r1 §6.2 D15] |
| `text(id) -> Cow<str>` | 없으면 "" | - |
| `offset(id) -> SkinOffset` | 0..199 | - |
| `timer_us(id) -> i64` / `now_us()` | OFF = `i64::MIN`. Lua 에는 double 로 | [b1 §6.1] |
| `exec_event(id, a1, a2)` | 미정의 id 무시, 1000..1999 는 커스텀 이벤트 | [b1 §6.2] |
| `write_rate(id, v)` / `write_text(id, s)` | 슬라이더·검색 | [b2 §17] |
| `audio(cmd)`, `key_pressed(code)`, `gauge()`, `gauge_type()`, `judge(n)`, `rate()`, `exscore()`, `volume_*`, `set_volume_*` | `main_state` 전용 | [b1 §6] |

- 구현 배치: `apps/rbms-player/src/skin_host/` 아래 값의 출처별 파일. 군집은 b5 §17 의 A~M 을 그대로 쓴다(A 곡 메타 `chart.rs`, B 라이브 점수 `score.rs`, C·D 판정·입력 `play.rs`, E 플레이어 설정 `options.rs`, F 선곡 바·패널 `select.rs`, G 결과 `result.rs`, H IR `ir.rs`, I 시간·통계 `system.rs`, J 스킨 설정 `skin_config.rs`, K 키 설정 `keyconfig.rs`, M 로딩 `loading.rs`). 파일이 군집별로 갈려 병렬 작업이 겹치지 않는다.
- 프레임 데이터: `FrameExtra`(화면당 한 종류)를 능력별 선택 필드로 바꾼다 — 노트 필드, 곡 바 목록, 시계열(게이지 이력·타이밍·BPM·노트 분포), 참조 이미지 세트. 결과 화면의 gauge 와 선택 화면의 bpmgraph 가 그려지지 않는 현 결함이 이 구조에서 해소된다 [r2 §7.2-6].
- 타이머: `TimerState` 를 µs 로 바꾸고 장면이 소유한다. 전환 시 전부 OFF. 드라이버는 게임 로직에 묶이므로 앱으로 옮긴다. 구동 차이 목록은 b5 §1-8 과 §16-17~21 을 기준으로 한다.
- 죽은 선언 정리: `MAPPINGS`/`source_of`/`UnmappedLog` 는 호출처가 없다 [b5 §16-1]. 호스트 구현이 진실이 되므로 삭제하거나 "호스트가 답하는 id 집합" 통계로만 남긴다.

### 2.5 판단 (d) 해상도 일반화

- 스킨 화면의 논리 크기 = 물리 뷰포트 크기(레터박스 적용 뒤). `SkinViewport` 가 레퍼런스의 dw/dh 역할을 하고, 필터의 1:1 판정과 텍스트 래스터가 물리 픽셀에서 이뤄진다 [r2 §4].
- `Gpu` 의 논리 크기를 런타임 값으로: 유니폼 갱신, `size()`, scissor, 레터박스 비율, 마우스 역변환 [r2 §10 A2].
- 스킨이 없는 내장 화면과 시스템 오버레이는 1280x720 좌표를 유지하고 배율 래퍼 렌더러를 거친다 [r2 §10 A3].
- 스케일되지 않는 값은 레퍼런스대로 둔다: 오프셋 값(출력 픽셀), TTF size, shadowOffset, 자릿수 offset, 비주얼라이저 width [b2 요약, b3 §15]. 현재 R-BMS 는 오프셋을 문서 좌표에 더하므로 [r1 §6.2 D12] 출력 해상도가 1920x1080 이 아닐 때만 차이가 난다. 사용자 오프셋 단위를 "출력 픽셀"로 맞춘다.
- 스킨 w/h 가 Resolution 열거값이 아니면 1280x720 으로 본다 [b1 §11].
- 텍스처 한도: 어댑터 한도를 조회하고 초과 시 경고 후 그 소스를 쓰는 객체만 생략한다. ModernChic 최대는 6400x1200, 5190x2571 [r2 §3.4]. wgpu 기본 한도 값은 조사에서 확인되지 않았다(미확인).

### 2.6 판단 (e) 텍스트

| 단계 | 내용 |
|---|---|
| 1차(TTF) | cosmic-text 아틀라스 쿼드 경로로 전환, 물리 픽셀 래스터, 패밀리별 캐시 분리. 크기 = dst h, x 정렬 기준점, 상단 기준 y, overflow 0/1/2, wrapping, 그림자(rgb/2, +x/-y), `ref` 가 유효하면 `constantText` 무시, 직전 객체 blend 상속 |
| 2차(.fnt) | BMFont 텍스트 파서, 페이지 텍스처 지연 로드, `scale = size*dw/원본size`, type 0 |
| 3차(distance field) | type 1/2 전용 파이프라인과 외곽선·그림자 유니폼. GPU 전용이라 CpuCanvas 골든으로 검증되지 않으므로 검증 전략을 따로 둔다 [r2 위험] |

- ModernChic 기본 옵션은 전 화면 TTF 이고 고유 TTF 는 medium/black 2종이다 [m6 §8.6]. 2·3차는 "画像フォント" 옵션을 켰을 때만 필요하다.
- 세로 기준(libGDX 는 대문자 윗선, cosmic-text 는 줄 상자 상단)의 보정값은 조사에서 계산되지 않았다 [r2 §6.4]. libGDX 1.9.9 소스가 저장소에 없어 글리프 배치·정수 반올림도 지식 기반이다 [b2 §20]. 실화면 비교로 맞춰야 한다.

### 2.7 판단 (f) 동영상 소스

- 로더는 source 를 종류(이미지/동영상)로 분류하고, 렌더는 "같은 key 재등록 = 제자리 업로드" 통로를 쓴다 [r2 §3.5].
- 디코더는 트레이트 뒤에 두고 의존성 승인 전에는 구현을 꽂지 않는다. 그동안 동영상 source 는 "프레임 없음 = 미표시"로 처리하고 경고를 남긴다(레퍼런스도 프레임이 없으면 그리지 않는다).
- 영향 범위: ModernChic 기본 설정에서 동영상이 실제로 쓰이는 곳은 "BGA 없는 곡의 범용 BGA" 한 곳이다(기본값이 動画) [m6 §8.3]. 선택·결정 배경은 기본이 정지화다.

### 2.8 판단 (g) 입력

| 기능 | 붙일 곳 | 내용 |
|---|---|---|
| 마우스 버튼·release·휠 | `R:apps/rbms-player/src/lib.rs` 이벤트 루프, `StageHandler` | `handle_mouse(at, button)`, `handle_mouse_drag(at)`, `handle_scroll(delta)`. 기본 구현 no-op [r3 §7.5] |
| 클릭 판정 | `skin_render/input.rs`(신규) | 그려진 객체를 역순으로, clickevent 가 있고 draw 인 첫 객체. click 0~3, 버튼 → 인자 `{1,-1,1,1,-1}` [b4 §1.3] |
| 드래그 | 같은 곳 | 슬라이더만. 이동 축 range, 양 끝 1px 스냅 [b2 요약] |
| 호버 | 이미 `SkinDraw.mouse` 로 전달 중 | 좌표계만 새 논리 크기를 따름 |
| 텍스트 입력 | `skin_render/text_input.rs`(신규) + 기존 `textedit.rs` | 편집 가능 text 포커스. 한글·일본어 검색이 필요하면 `set_ime_allowed(true)` 와 `WindowEvent::Ime` |
| 키 상태 질의 | 호스트 `key_pressed` | 결과 메뉴의 Gdx 폴링용 |
| 레인 키·START/SELECT | `SelectState::handle_pad`, `shared.lane_for(code)` | 패널 1~3 과 결정·결과의 키 인덱스 입력. START/SELECT 바인딩 유무는 미확인 [r3 §14.2] |

### 2.9 판단 (h) 내장 화면과 R-BMS 고유 화면

- 스킨이 있는 화면 타입(0~3, 5, 6, 7, 15)은 스킨이 화면 전체를 단독으로 그린다. 내장 레이아웃과 섞지 않는다.
- 스킨 계약에 없는 것은 시스템 계층으로 스킨 위에 그린다: 토스트, 디버그 패널, 대화상자, 리플레이 분석 오버레이, 스킨 로드 오류 안내.
- R-BMS 전용 화면(Settings, Tables, Folders, Practice, 스캔·테이블 받기 대기)은 내장 화면으로 유지한다. 진입은 스킨 이벤트 13(키 설정), 14(스킨 설정)와 기존 단축키로 한다.
- 키 설정은 내장 UI 를 유지한다(K1).
- 스킨이 없거나 로드에 실패했을 때의 화면은 6절 결정 3 에 따른다. 전환 기간에는 현 내장 play/select/result 화면이 폴백 역할을 한다.
- 9키·24키는 ModernChic 에 문서가 없다. 6절 결정 12 에 따른다.

### 2.10 판단 (i) steel-neon 번들과 rbms 전용 확장

| 대상 | 처리 | 근거 |
|---|---|---|
| `composition`, `destination.layer`, 최상위 `replace`, `result.replace`, `hotspot` | 삭제 | 사용자가 혼합 합성을 부적합으로 판정. 크레이트 내 비용은 모델 약 60행·로더 약 30행·테스트 6건 [r1 §7] |
| `scope: 'bundle'` 과 `shared` 저장 | 삭제 | `.luaskin` 에는 선언 문법이 없고 레퍼런스는 화면 타입별 독립 저장 [r3 §4] |
| `densitygraph` | 삭제 | 레퍼런스의 노트 분포는 judgegraph·`graph type < 0` 으로 표현 [b4 요약] |
| 사설 id 20001~20316 | 삭제 | 레퍼런스 id 공간 밖 [b5 §2.5] |
| `judge.images` 에 text 허용 | 삭제 | 구 번들 전용 [r2 §5.2] |
| steel-neon, -v2, -v3 와 `include_bytes!` 표, 세대 이동 로직 | 삭제 | 혼합 합성 전제 [r3 §3.1] |
| `display.skin` 프리셋(NORMAL/WIDE/STEEL NEON), `theme.ron` 결합 | 프리셋 결합 제거. 내장 화면을 폴백으로 두는 동안 NORMAL/WIDE 필드 RON 은 유지 | [r3 §2.3] |
| 경로 가두기, 시드 난수, 문서 크기 한도, `nested` 선조립 | 유지 | 레퍼런스에 없는 안전장치이거나 같은 구조 [r1 §7] |
| 설정 마이그레이션 | 스키마 버전을 올려 구 번들 선택과 `shared` 를 버림 | [r3 §12 B3] |

---

## 3. 화면 대응과 UI·레이아웃 변화

### 3.1 화면 흐름 대응

| 레퍼런스 상태 (type) | ModernChic 진입 | 현재 R-BMS | 목표 |
|---|---|---|---|
| MUSICSELECT (5) | musicselect | `Select` + 내장 UI | `Select` 가 스킨 단독. 곡 결정 시 즉시 Decide |
| DECIDE (6) | decide | `Loading` 이 decide 문서를 그림 | `Decide` 신설. input 500 / scene 3000 / fadeout 1000. 차트·키음 로드는 이 동안 백그라운드로 시작 |
| PLAY (0,1,2,3) | play7/5/14/10 | `Play` 즉시 시작 | `Play` 가 PRELOAD(로딩 연출) → READY → PLAY → FAILED/FINISHED → 페이드 |
| RESULT (7) | result | `Result` 즉시 종료 | 입력 잠금 2500ms, 메뉴 전환, 페이드 1000ms |
| COURSERESULT (15) | course | 내장 텍스트 | 스킨 화면 |
| CONFIG (8) | keyconfig(비어 있음) | 내장 | 내장 유지 |
| SKINCONFIG (9) | skinselect | SETTINGS 의 SKIN 탭 | 결정 5 에 따라 Stage 신설 또는 SKIN 탭 유지 |
| 없음 | 없음 | Settings, Tables, Folders, Practice, 스캔 대기 | 내장 유지 |

전환 공통 규칙: 이전 장면 종료 → 이전 스킨 해제 → 새 장면 상태 확정 → 스킨 동기 로드(헤더 → 본체) → 정적 조건 제거 → 타이머 전체 OFF + 장면 시계 0 → 장면 준비 [b4 §0-1]. 스킨 구성 시점에 곡·결과 상태가 확정돼 있어야 한다 [m1 §10].

코스: 레퍼런스는 스테이지 사이에 RESULT → PLAY 로 직행한다. R-BMS 는 스테이지마다 Loading 을 거친다 [r3 §11-9]. PRELOAD 가 로딩을 맡으면 자연히 직행이 된다.

### 3.2 사용자가 체감할 변화

| 영역 | 지금 | 바뀐 뒤 |
|---|---|---|
| 창·해상도 | 1280x720 논리 화면을 창으로 늘림 | 창 픽셀 해상도로 직접 그림. 글자가 선명해짐. 해상도·전체 화면 설정 추가(결정 9) |
| 흐름 | 선택 → 로딩 막대 → 플레이 → 결과 | 선택 → 결정 연출(건너뛰기 가능) → 플레이 로딩·READY 연출 → 종료 페이드 → 결과 |
| 선택 화면 | 왼쪽 목록 + 오른쪽 상세 + 아래 버튼 줄 | 오른쪽 곡 바 휠(17칸, 중앙 8번), 상하단 프레임, 하단 3창(정보·점수·분석), 우상단 버튼 열, 스크롤바, 사이드메뉴 [m4 요약]. 아래 버튼 줄은 없어지고 단축키·스킨 버튼으로 대체 |
| 옵션 | Shift 홀드/F1 의 11행 패널 | 스킨의 패널 3종(플레이 옵션, 어시스트, 상세). 누르는 동안만 열림 |
| 검색·필터·기록·랭킹 | 내장 상자·패널·모달 | 스킨에 대응물이 있는 것은 스킨, 없는 것은 시스템 오버레이(결정 5) |
| 플레이 | 내장 필드 + 문서 장식, 키 봄은 내장 | 레인·노트·판정·게이지·봄·커버 전부 스킨. 좌/우 배치와 스크래치 방향은 스킨 옵션. 폐점·풀콤보 연출 |
| 결과 | 키 즉시 종료, IR 한 줄 | 2.5초 뒤 입력, Info 메뉴 전환(클릭·방향키), 게이지 종류 전환, 페이드 |
| 코스 결과 | 내장 텍스트 목록 | 스킨 화면 |
| 스킨 설정 | 화면 타입마다 따로 고르는 행 목록 | 스킨 팩 폴더 하나로 전 화면 지정. 옵션 수가 많다(SP 36개, 파일 20개, 오프셋 10개 [m1 §3]) |
| 소리 | 전환 순간 효과음 | 선택 BGM 루프, 막대 이동음, 결과 루프·닫힘음, 스킨 자체 클릭음 |
| 설정·폴더·표·연습 | 내장 | 내장 유지. 스킨 화면과 시각적 이질감이 남는 부분 |
| 조작 지연 | 없음 | 결정 최대 4초(건너뛰기 가능), 플레이 진입 시 loadend 3500 + playstart 1000, 실패 시 close 3000 |

---

## 4. 작업 분해

원칙
- 같은 파일을 같은 웨이브의 두 단위가 동시에 고치지 않는다. 겹치는 경우 선행 관계로 직렬화했다.
- 각 웨이브 끝의 게이트는 `cargo fmt --check`, `cargo clippy --workspace -- -D warnings`, `cargo test --workspace` 다(저장소 규칙, `R:docs/HANDOFF.md` 5절 직접 확인). 눈으로 볼 결과는 헤드리스 캡처 또는 덤프 출력이다. GPU 창 확인은 사용자 절차다.
- 모델은 사용자 지시대로 opus 는 high 까지, sonnet 은 xhigh 까지다. 구조 판단이 큰 단위는 opus high, 사양이 표로 고정된 단위는 sonnet.
- 단위에 적은 "사양 근거"는 구현자가 읽어야 할 조사 보고서 절이다.

### 웨이브 1 — 철거와 기반

끝났을 때: 구 번들과 혼합 합성이 사라지고 앱은 내장 화면으로 동작한다. 창 해상도로 직접 그린다.

| 단위 | 내용 | 소유 파일 | 선행 | 병렬 | 검증 | 모델 | 사양 근거 |
|---|---|---|---|---|---|---|---|
| W1-1 | 소비자 쪽 혼합 합성·구 번들·사설 id 철거 | `apps/rbms-player/src/{skin_screen.rs, assets.rs, app_options.rs, app_input.rs, skin_select.rs}`, `stage/{play/mod.rs, select/mod.rs, result.rs, loading.rs, keyconfig.rs}`, `stage/render_tests_skin*.rs`(삭제), `stage/{select,play}/tests.rs`, `crates/rbms-render/src/{content.rs(삭제), hud.rs, select.rs, result.rs}`, `skin_render/{mod.rs, screen.rs, state.rs, songlist.rs, object.rs, graphs.rs, judge.rs}`, `assets/skins/steel-neon*`(삭제), `crates/rbms-config/src/schema.rs` | 없음 | 단독 | 게이트 + 내장 select/play/result 헤드리스 캡처가 철거 전과 같음 | opus high | r3 §8.2·§8.6, r2 §8 |
| W1-2 | 모델·로더 확장 필드 삭제 | `crates/rbms-skin/src/{model.rs, model/objects.rs, loader.rs, resolve.rs}`, `tests/{skin_nested.rs, skin_model.rs, fixtures/nested/*}` | W1-1 | 단독 | `cargo test -p rbms-skin` | sonnet high | r1 §7 |
| W1-3 | Lua 5.2 전환 | `crates/rbms-skin/Cargo.toml`, `Cargo.lock`, 필요 시 `src/lua.rs` | W1-2 | W1-6 과 병렬 | `tests/skin_lua.rs` 28건, `set_memory_limit`·훅 동작 확인 | sonnet high | r1 §5.3 |
| W1-4 | 타이머 µs, 타이머 참조형 자리, 오프셋 `r` 부호, acc 선언 순서, stretch 11종 | `crates/rbms-skin/src/{timer.rs, timer/tests.rs, dst.rs, dst/tests.rs, loader/track.rs, loader/stretch.rs, property/mod.rs, lua.rs}`, `tests/skin_loader.rs`, `crates/rbms-render/src/skin_render/{draw.rs, screen.rs, state.rs}` | W1-3 | W1-6 과 병렬 | dst 38건, stretch 테스트 추가, 게이트 | opus high | r1 §6.2·§6.3 |
| W1-5 | 장면 시계와 전환 시 타이머 리셋, 마우스 버튼·release·drag·휠 이벤트 배선 | `apps/rbms-player/src/{lib.rs, stage/mod.rs, skin_screen.rs, app_play.rs}` | W1-4 | W1-6 과 병렬 | 전환·타이머 단위 테스트 | sonnet xhigh | r3 §12 A1·A3, §7.5 |
| W1-6 | 텍스트 캐시 패밀리 분리, `load_font` 성공 판정 | `crates/rbms-render/src/font.rs` | W1-1 | 병렬 가능 | `crates/rbms-render/tests/primitives.rs`, 캐시 통계 테스트 | sonnet high | r2 §6.2·§6.3 |
| W1-7 | GPU 논리 크기 런타임화, 배율 래퍼, 텍스처 한도 확인, 배치 키 `rotated` 제거, 색 공간(결정 8) | `apps/rbms-player/src/gpu/{mod.rs, batch.rs, background.rs}`, `stage/canvas.rs`, `crates/rbms-render/src/lib.rs`, `apps/rbms-player/src/lib.rs` | W1-5 | 단독(`lib.rs`) | gpu 계산 테스트, 내장 화면 캡처 불변, 사용자 GPU 창 확인 | opus high | r2 §10 A1~A5 |
| W1-8 | 해상도·창 모드 설정 행 | `crates/rbms-config/src/schema.rs`, `apps/rbms-player/src/{settings_ui.rs, stage/settings.rs}` | W1-7 | 단독 | 설정 저장·복원 테스트 | sonnet high | r3 §9.4 |

### 웨이브 2 — Lua 런타임과 로더

끝났을 때: 덤프 도구가 ModernChic 의 `.luaskin` 10개를 읽어 헤더와 본체 객체 수를 출력한다.

| 단위 | 내용 | 소유 파일 | 선행 | 병렬 | 검증 | 모델 | 사양 근거 |
|---|---|---|---|---|---|---|---|
| W2-1 | 값 참조 타입 확장(함수·이름·이벤트·writer) | `crates/rbms-skin/src/{model.rs, model/objects.rs, dst.rs}`, `crates/rbms-render/src/skin_render/object.rs`(컴파일 맞춤만) | 웨이브 1 | W2-2 와 병렬 | `tests/skin_model.rs` | opus high | r1 §3.3, b2 §17 |
| W2-2 | 샌드박스 재구성: 환경, 검색기, io 오버레이, os, luajava, print, 예산, 오류 로그 | `crates/rbms-skin/src/lua/{mod.rs, env.rs, package.rs, io.rs, os.rs, luajava.rs, budget.rs}`(신규), `src/lua.rs`(삭제) | 웨이브 1 | W2-1 과 병렬 | 새 `tests/skin_lua.rs`(루트 가두기, 쓰기 오버레이, 패턴 함수, 무한 루프·메모리 차단, 바이트코드 차단) | opus high | b1 §2, r1 §5.4·§5.5, m1 §4 |
| W2-3 | 호스트 계약, `main_state`, `timer_util`/`event_util` 프렐류드, 프레임 바인딩 | `crates/rbms-skin/src/{property/mod.rs, property/host.rs(신규), lua/main_state.rs(신규), lua/prelude.lua(신규)}`, `tools/gen-skin-property.rs`(정적 분류·이름 역표) | W2-2 | W2-4 와 병렬 | 고정값 호스트로 API 26종 테스트 | opus high | b1 §6·§7, b5 §2 |
| W2-4 | Lua 값 → `SkinDef` 변환기 | `crates/rbms-skin/src/loader/from_lua.rs`(신규) | W2-1, W2-2 | W2-3 과 병렬 | 절삭, 진리값, 숫자 문자열화, 배열 구멍, boolean 속성 null, 함수 등록 | opus high | b1 §4, r1 §3.3 |
| W2-5 | 2패스 로드, 헤더 병합(`def` 스템, 무작위 -1, 자동 오프셋 4종), type 9·15, 플레이 헤더 필드 적용 조건 | `crates/rbms-skin/src/{loader.rs, loader/lua_skin.rs(신규), loader/branch.rs, resolve.rs}` | W2-3, W2-4 | 단독 | `.luaskin` 픽스처 | opus high | b1 §3·§9·§10, r1 §4 |
| W2-6 | 스킨 덤프 CLI와 고정값 호스트 | `apps/rbms-cli/**` | W2-5 | W2-7 과 병렬 | ModernChic 10개 실행. 기대: 헤더 표가 m1 §3 과 일치, keyconfig 본체만 `M:Decide/lua/require/textproperty.lua:37` 에서 실패, 그 외 pcall 실패 0 | sonnet high | m1 §3, m2~m6 의 객체 수 |
| W2-7 | 테스트와 픽스처 | `crates/rbms-skin/tests/{skin_luaskin.rs(신규), skin_integration.rs}`, `tests/fixtures/luaskin/*`(신규) | W2-5 | W2-6 과 병렬 | 게이트 | sonnet xhigh | r1 §8 |

덤프 대조용 수기 집계(전부 정적 추정이므로 불일치 시 덤프를 믿고 조사 수치를 고친다): play7 destination 약 288·image 약 223·value 약 63 [m2 요약], play14 destination 약 375·image 263 [m3 요약], select destination 1914·image 1438·imageset 49·value 154·graph 111·text 65 [m4 요약], result 194·course 175·decide 약 69 [m5 요약], skinselect 108 [m6 요약].

### 웨이브 3 — 공통 렌더 의미론과 결정 화면

끝났을 때: ModernChic 결정 화면이 헤드리스 캡처로 나온다(t = 0, 500, 1500, 3000ms).

| 단위 | 내용 | 소유 파일 | 선행 | 병렬 | 검증 | 모델 | 사양 근거 |
|---|---|---|---|---|---|---|---|
| W3-1 | 프레임 파이프라인: 2단계 prepare/draw, Lua 함수 평가, 조건 의미론(미지 op 제거·정적 1회), 커스텀 타이머·이벤트, 능력별 프레임 데이터, 어댑터 철거, 모듈 골격 분할 | `crates/rbms-render/src/skin_render/{mod.rs, frame.rs(신규), object.rs, state.rs(삭제), text.rs·refs.rs·textures.rs·input.rs(빈 골격)}`, `crates/rbms-skin/src/loader/track.rs` | 웨이브 2 | 단독 | `crates/rbms-render/tests/skin_render.rs` 재작성 | opus high | b1 §5.3·§8.2, r2 §7 |
| W3-2 | 기본 객체 정합: 이미지 인덱스, 음수 ref 숨김, 센티널, 음수 w/h, 참조 이미지, NaN, 필터 판정 | `skin_render/{draw.rs, refs.rs}` | W3-1 | W3-3~6 과 병렬 | 객체별 CpuCanvas 테스트 | opus high | b2 §18, r2 §5 |
| W3-3 | 텍스트 1차(TTF) | `crates/rbms-render/src/{font.rs, glyph_atlas.rs}`, `skin_render/text.rs` | W3-1 | 병렬 | 정렬·overflow·그림자 골든 | opus high | r2 §6, b2 요약 |
| W3-4 | 텍스처 관리자: 참조 소스만 지연 로드, 이탈 시 해제, 메모리 집계 | `skin_render/textures.rs`, `apps/rbms-player/src/{skin_screen.rs, assets.rs}` | W3-1 | 병렬 | 로드된 소스 수 테스트 | opus high | r2 §3 |
| W3-5 | judgegraph(노트 분포), bpmgraph | `skin_render/graphs.rs` → `graphs/{mod.rs, notes_dist.rs, bpm.rs}` | W3-1 | 병렬 | 텍스처 생성 규칙 테스트 | sonnet xhigh | b3 §8.1·§8.2 |
| W3-6 | 호스트 골격과 군집 A·I·M | `apps/rbms-player/src/skin_host/{mod.rs, chart.rs, system.rs, loading.rs}`(신규) | W3-1 | 병렬 | id 단위 테스트 | sonnet xhigh | b5 §3~§6 중 해당 군집 |
| W3-7 | Decide Stage, 장면 수명 헬퍼, 로딩 분리 | `apps/rbms-player/src/stage/{decide.rs(신규), scene_life.rs(신규), loading.rs, mod.rs}`, `stage/select/mod.rs`(곡 결정 전이 지점만), `app_play.rs` | W3-6 | 단독 | 전이 테스트, 결정 화면 캡처 4장 | opus high | b4 §4, m5 부록 B |

### 웨이브 4 — 결과와 코스 결과

끝났을 때: 결과·코스 결과 화면 캡처(클리어/실패, 메뉴 1/2), Info 클릭 전환.

| 단위 | 내용 | 소유 파일 | 선행 | 병렬 | 검증 | 모델 | 사양 근거 |
|---|---|---|---|---|---|---|---|
| W4-1 | 클릭·호버·슬라이더 드래그·휠 디스패치와 앱 연결 | `skin_render/input.rs`, `apps/rbms-player/src/skin_screen.rs`, `stage/mod.rs` | 웨이브 3 | W4-2, W4-3, W4-5, W4-7 과 병렬 | z 역순·click 0~3·mouseRect 테스트 | opus high | b2 요약(마우스), b4 §1.3 |
| W4-2 | gauge 정합, gaugegraph, timingdistributiongraph, judgegraph type 1/2 | `skin_render/gauge.rs`, `graphs/{gauge_graph.rs, timing_dist.rs, notes_dist.rs}` | 웨이브 3 | 병렬 | 색 표·드러남 비율 테스트 | opus high | b3 §5, b4 §5.6 |
| W4-3 | 플레이 엔진 기록 확장: 게이지 전 종류 이력, 판정별 early/late, 통과 노트 기준 값 | `crates/rbms-play/**`, `apps/rbms-player/src/app_result.rs` (정확한 파일은 착수 시 확인, 미확인) | 웨이브 3 | 병렬 | 엔진 단위 테스트 | opus high | b5 §16-4~15, b4 위험 |
| W4-4 | Result Stage 수명·입력·게이지 종류 전환·리플레이 슬롯 | `apps/rbms-player/src/stage/result.rs`, `app_result.rs` | W4-3 | 단독 | 전이·타이머 테스트, 캡처 | opus high | b4 요약(결과), m5 §1 |
| W4-5 | 호스트 군집 B·G·H(오프라인 고정)·E 일부(42, 43, 90, 301~307, 370) | `skin_host/{score.rs, result.rs, ir.rs, options.rs}`(신규) | 웨이브 3 | 병렬 | m5 부록 A 의 id 전수 테스트 | sonnet xhigh | m5 부록 A, b5 §17 |
| W4-6 | CourseResult Stage | `stage/course_result.rs`, `course_ui.rs` | W4-4 | 단독 | 캡처 | sonnet high | b4 요약(코스) |
| W4-7 | 오디오: 루프·정지, 스킨 버스 `audio_play`, 호스트 키 상태 | `crates/rbms-audio/**`, `apps/rbms-player/src/syssound.rs`, `skin_host/audio.rs`(신규) | 웨이브 3 | 병렬 | 볼륨 클램프·경로 가두기 테스트 | sonnet xhigh | m1 §9, b4 §1.6 |

### 웨이브 5 — 선곡

끝났을 때: 선곡 화면 캡처 세트(곡/폴더/코스 바 x 점수 유무 x 패널 0~3) [m4 §13].

| 단위 | 내용 | 소유 파일 | 선행 | 병렬 | 검증 | 모델 | 사양 근거 |
|---|---|---|---|---|---|---|---|
| W5-1 | songlist 재작성과 스크롤 보간 | `skin_render/songlist.rs` | 웨이브 4 | W5-2, W5-5, W5-6 과 병렬 | 인덱스 규칙·보간식 테스트 | opus high | b4 §2, m4 §5 |
| W5-2 | 선곡 상태 모델(막대 종류, 램프 id, 라벨, 트로피, 리플레이 슬롯, 배너·스테이지파일 분리) | `stage/select/{scene.rs, list.rs}`, `apps/rbms-player/src/lib.rs` | 웨이브 4 | 병렬 | 목록 구성 테스트 | opus high | b4 §3.9·§3.10, r3 §6.3 |
| W5-3 | 선곡 입력, 패널 1~3, 이벤트 표, 장면 수명 | `stage/select/{mod.rs, filter.rs}`, `app_options.rs` | W5-2 | 단독 | 키 표·패널 타이머 쌍 테스트 | opus high | b4 §3.3~§3.6 |
| W5-4 | 호스트 군집 F·E·H(선곡) | `skin_host/{select.rs, options.rs, ir.rs}` | W5-2 | W5-3 과 병렬 | b5 §15 표 상위 id 테스트 | sonnet xhigh | b5 §15·§17, m4 §6 |
| W5-5 | 슬라이더 writer(type 1, 8, 17, 18, 19), 편집 텍스트 | `skin_render/text_input.rs`(신규), `skin_host/writers.rs`(신규), `textedit.rs` | 웨이브 4 | 병렬 | 드래그 값·검색 확정 테스트 | sonnet xhigh | b4 §3.7, m4 §9 |
| W5-6 | 선택 BGM, 이동·패널 사운드 지점 | `syssound.rs`, `stage/select/preview.rs` | 웨이브 4 | 병렬 | 재생 지점 테스트 | sonnet high | b4 §1.6 |

### 웨이브 6 — 플레이 SP·DP

끝났을 때: 7키·5키·14키·10키 캡처(로딩, READY, 플레이 중, 폐점, 풀콤보)와 오토플레이 실행.

| 단위 | 내용 | 소유 파일 | 선행 | 병렬 | 검증 | 모델 | 사양 근거 |
|---|---|---|---|---|---|---|---|
| W6-1 | note(레인 지오메트리, y 누적식, LN/HCN 매핑, 마디선류) | `skin_render/notes.rs` | 웨이브 5 | W6-2~W6-7 과 병렬 | STOP·SCROLL 차트 픽스처 | opus high | b3 §3 |
| W6-2 | 커버·judge | `skin_render/{covers.rs, judge.rs}` | 웨이브 5 | 병렬 | shift·영역 콤보 테스트 | opus high | b3 §4·§6 |
| W6-3 | 플레이 상태기계 | `stage/play/mod.rs` | 웨이브 5 | 병렬 | 상태 전이 테스트 | opus high | b3 §10 |
| W6-4 | 타이머 드라이버와 오프셋 1~5 | `skin_host/play_timers.rs`(신규), `skin_render/screen.rs`(삭제) | 웨이브 5 | 병렬 | b3 §11 표 전수 테스트 | opus high | b3 §11, b5 §16-17~21 |
| W6-5 | 호스트 군집 C·D | `skin_host/play.rs`(신규) | 웨이브 5 | 병렬 | m2·m3 의 참조 id 테스트 | sonnet xhigh | m2 §5, m3 §5, b5 §17 |
| W6-6 | BGA 레이어 | `skin_render/bga.rs`(신규), `stage/canvas.rs`, `assets.rs` | 웨이브 5 | 병렬 | 레이어 순서 테스트 | sonnet xhigh | b3 §7 |
| W6-7 | 비주얼라이저 2종, 슬라이더 type 4/5/6, `OFFSET_ALL` | `graphs/{timing_vis.rs, hit_error.rs}`(신규) | 웨이브 5 | 병렬 | 방향·스케일 테스트 | sonnet high | b3 §8.3·§8.4 |
| W6-8 | 내장 `Skin` 의존 제거 | `crates/rbms-render/src/skin.rs`, `apps/rbms-player/src/{app_input.rs, app_play.rs}` | W6-1~W6-5 | 단독 | 게이트, 플레이 캡처 | opus high | r3 §11-4, r2 §8.1 |

`skin_render/mod.rs` 의 `OFFSET_ALL` 적용은 W6-7 이 아니라 W6-8 이 맡는다(같은 파일을 W6-8 이 정리하므로).

### 웨이브 7 — 스킨 팩, 기본 스킨, 배포, 정리

끝났을 때: 새 기본 스킨으로 첫 실행부터 전 화면이 그려진다.

| 단위 | 내용 | 소유 파일 | 선행 | 병렬 | 검증 | 모델 | 사양 근거 |
|---|---|---|---|---|---|---|---|
| W7-1 | 스킨 팩 선택 모델과 SKIN 탭 | `crates/rbms-config/src/schema.rs`, `apps/rbms-player/src/{skin_select.rs, settings_ui.rs}`, `stage/settings.rs` | 웨이브 6 | W7-3 과 병렬 | `skin_select/tests.rs`, `stage/settings/skin_tests.rs` | opus high | r3 §4·§5 |
| W7-2 | 스킨 설정 Stage(결정 5 가 "신설"일 때), keyconfig 헤더 전용 | `stage/{skin_config.rs(신규), keyconfig.rs}`, `skin_host/{skin_config.rs, keyconfig.rs}`(신규) | W7-1 | 단독 | 이벤트 190, 220~228 테스트 | sonnet xhigh | b4 요약(스킨 설정), m6 §7 |
| W7-3 | 기본 스킨 단순화판 제작(5절) | `assets/skins/<기본 스킨 폴더>/**` | 웨이브 6, 결정 1·2 | 화면별 하위 단위로 병렬(Root, play sp, play dp, select, decide, result/course) | 덤프 도구 pcall 실패 0, 화면별 캡처 | opus high(Lua 편집), sonnet high(자산 정리) | m2 §9, m3 §9, m4 §12, m5 §16, m6 §9 |
| W7-4 | 설치·배포 교체 | `apps/rbms-player/src/{assets.rs, lib.rs}`, `.github/workflows/release.yml` | W7-3, 결정 1 | 단독 | 설치 테스트 재작성 | sonnet high | r3 §3 |
| W7-5 | 내장 play/select/result/loading 화면 처리(결정 3) | `crates/rbms-render/src/{hud.rs, playfield.rs, select.rs, result.rs, theme.rs}`, 각 Stage | W7-4 | 단독 | 게이트 | opus high | r2 §8.2 |
| W7-6 | 테스트·골든 재편 | `apps/rbms-player/src/stage/render_tests*.rs`, `crates/rbms-render/tests/**` | W7-5 | W7-7 과 병렬 | 게이트 | sonnet xhigh | r3 §8.6 |
| W7-7 | 문서 | `docs/{skin.md, architecture.md, development.md, HANDOFF.md, PROCESS.md, acknowledge/*}` | W7-5 | 병렬 | 문서 diff | sonnet high | r1 §7 말미 |

### 웨이브 8 — 후속(선택)

| 단위 | 내용 | 선행 | 모델 |
|---|---|---|---|
| W8-1 | `.fnt` 로더와 type 0 그리기 | 웨이브 3 | opus high |
| W8-2 | distance field 파이프라인 | W8-1 | opus high |
| W8-3 | 동영상 소스(결정 6, 의존성 승인 뒤) | 웨이브 3 | opus high |
| W8-4 | IR 랭킹·라이벌 실연결(군집 H) | 웨이브 5 | sonnet xhigh |
| W8-5 | 9키 문서(결정 12) | 웨이브 7 | opus high |
| W8-6 | `filter != 0` 전용 bilinear 셰이더, skinpreview | 웨이브 3 | sonnet high |

---

## 5. 기본 스킨 단순화판 제안

### 5.1 방침

- 단순화판도 Lua 로 유지한다. 정적 JSON 으로 변환하지 않는다. 본체가 로드 시 `main_state.option(NO_BGA)` 같은 곡 상태로 구조를 분기하므로 정적 문서로는 표현할 수 없고 [m3 요약], 런타임은 "수정 없이 읽기" 목표 때문에 어차피 필요하다.
- 단순화는 추가가 아니라 삭제로 한다: 파트 파일 제거, 옵션 고정, 자산 변종 제거. 좌표와 시트는 원본 그대로 쓴다.
- `luajava`, 파일 쓰기, `os.date` 의존을 기본판에서 없앤다(`Root/customfunction.lua` 의 파일 계열, `customnumber.lua:289-303` 의 로드 시 읽기, `infoOutput`). 기본판은 읽기 전용 위치에서 오버레이 없이도 돌아야 한다.

### 5.2 유지와 제거

| 화면 | 유지 | 제거 |
|---|---|---|
| 공통(Root) | `define`, `main*.lua` id 표, `customoption`/`customnumber`/`customgraph`/`customslider` 의 순수 함수, TTF 2종(medium, black) 한 벌 | `customfunction` 의 파일·디렉터리·로테이션·`infoOutput`, `customsound` 의 음성·구역 효과, `customtime`, `customtext` 팁, `config.lua` 의 voice/impression/bpmLinkChar, 캐릭터 이미지 7장, 음성 48개 |
| 플레이 SP | 레인, 키 베드·키빔·키플래시·스크래치, 노트(LN/HCN/지뢰/히든 포함), 판정 + 콤보, 게이지(36 노드), 폭발 기본 1종, 레인 커버·리프트·히든, 진행바, 정보 영역, BGA 프레임, 스코어 그래프, 판정 카운터, 로딩 UI, 폐점 연출, 종료 페이드, 좌/우 배치와 스크래치 방향 옵션, 5키 처리 [m2 §9.2] | 상세 모드(`detailinfo/*`), 공격 모션, 구역 점수, 음성 훅, 캐릭터 연동, 점수 플랩, 레인 커버 로테이션, 범용 BGA 동영상, 폭발 규격 OADX, 판정 타이밍 폭발, 비트맵 폰트, 자산 변종(배경 5색, 노트 9종, 판정 3종, 폭발 6종), 풀콤보 시트는 축소 또는 제거 [m2 §9.1] |
| 플레이 DP | SP 와 같은 핵심 + 좌우 판정, 중앙 게이지, 10키 프레임 [m3 §9.2] | BGA 복제 레이아웃(1:1 x4, 16:9), `hcnBomb`, 판정 변형 8종(judge.lua 의 약 80%), 로테이션 [m3 §9.1] |
| 선곡 | songlist, 상하단 프레임, 곡 정보 텍스트, 스테이지파일·배너, 버튼 열, 스크롤바, 점수·정보·분석 창, 폴더 현황, 옵션 패널 3종, 검색 표시, 시작 연출 1종 [m4 §12] | BPM 연동 캐릭터(객체 1002개, draw 클로저 1000개), 배경 동영상과 로테이션, 비트맵 폰트, 이력, `infoOutput`, 사이드메뉴 상태 보존, 언어 2벌, 버전 확인, 어시스트 패널의 무동작 클릭 영역. 사이드메뉴는 크게 축소(결정 11) |
| 결정 | 난이도 색, 제목·장르·아티스트·레벨 배치와 페이드, 검정 띠 + 스테이지파일, lockon 1세트, 노트 분포·BPM 그래프, 페이드아웃 [m5 §16] | 동영상 배경, 비트맵 폰트, STAGE n, tips, lockon 4세트 |
| 결과·코스 결과 | 배경 + 링 + 빔, 게이지·판정·타이밍 그래프, 랭크·클리어 표시, 점수·판정 행, FAST/SLOW 막대, Info 메뉴 전환, 중앙 열, 시작 애니메이션, 페이드아웃, 코스 패널 [m5 §16] | IR 메뉴 전체(결정 10 에 따라), 인프레션, 이력, diflist, 로테이션, ClearType·Rank 배경 슬롯(Clear or Failed 와 ALL 만), 캐릭터, 리플레이 3중 중복 |
| 키 설정·스킨 선택 | 헤더만(type 8), 스킨 선택은 결정 5 에 따름 | `SkinSelect/` 의 제3자 일러스트 배경 |

메뉴 전환의 Gdx 폴링 객체는 기본판에서도 그대로 둘 수 있다(luajava facade 가 제공되므로). 의존을 줄이려면 `main_state.key_pressed` 로 바꾼다.

### 5.3 예상 용량

| 묶음 | 원본 MiB | 기본판 |
|---|---|---|
| mp4 8개 | 192.8 | 0 |
| 비트맵 폰트(fnt + 페이지) | 139.4 | 0 |
| TTF 11개(고유 3종) | 54.7 | 10.4(2종 한 벌) |
| Result 배경 27장 | 38.9 | 2~4 |
| fullcombo 시트 | 10.5 | 0 또는 축소 |
| bomb 7종, bg 7종 | 15.0 | 2~3 |
| 언어 3벌 중 2벌 | 4.9 | 0 |
| SkinSelect 폴더 | 10.6 | 0 |
| 그 외 PNG·ogg·Lua | 약 10 | 약 20 |
| 합계 | 476.4 | 35~45 |

출처는 m6 §9.1 의 추정이다. 어떤 슬롯을 남기느냐에 따라 달라진다.

### 5.4 배포 방식 선택지

| 방식 | 장점 | 단점·제약 | 라이선스 조건 |
|---|---|---|---|
| A. 저장소 폴더 + 릴리스에 폴더 동봉, 실행 시 직접 참조 | 바이너리 크기 불변, 복사 없음, 수정이 바로 반영 | 배포물이 "바이너리 + 폴더"가 됨. 읽기 전용 위치일 수 있어 기본판이 쓰기를 하지 않아야 함. 저장소가 35~45 MiB 증가 | 공개 저장소 `github.com/B-HS/R-BMS`(직접 확인) 에 올리면 2차 배포에 해당할 수 있음. 원작자 허가와 KASAKO 표기 필요 [m6 §12.1] |
| B. A + 최초 실행 시 사용자 폴더로 복사 | 사용자가 고칠 사본 확보 | 디스크 2배, 갱신 정책 재설계 | A 와 같음 |
| C. `include_bytes!` 내장 | 단일 파일 배포 | 바이너리 +35~45 MiB(macOS 유니버설은 2배), `compute_build_hash` 가 전체를 읽음, 파일 표 수작업 [r3 §3.3] | A 와 같음 |
| D. 별도 다운로드 | 저장소·바이너리 모두 가벼움 | 호스팅 필요, 오프라인 첫 실행에 스킨 없음 → 폴백 필수 | 호스팅도 2차 배포 |
| E. 저장소에는 넣지 않고 사용자가 가진 원본 폴더를 가리켜 로컬에서 단순화판을 생성 | 재배포 없음 | 다른 사용자는 원본을 직접 구해야 함. 생성 도구 필요 | 개인 사용 범위. 재배포 문제 없음 |
| F. 원본 Lua 구조·좌표만 참고하고 이미지·폰트·사운드를 자작으로 교체 | 재배포 가능 | 제작량이 큼. 시각이 원본과 달라짐 | 코드 개조 공개 시 표기 조건은 여전히 확인 필요 |

제3자 자산은 어느 방식에서도 뺀다: VOICEVOX 음성, 캐릭터 일러스트, `Result/parts/rank` 의 `#default`·`Damage`·`Formal`·`Underworld`(readme 가 자유 사용에서 제외, 그런데 기본 선택값이다), 출처 미기재 동영상·배경·bomb 시트 [m6 §12.2, m5 §1-15]. Mgen+ 폰트는 SIL OFL 1.1 이라 라이선스 전문 동봉으로 재배포 가능하다. DOVA-SYNDROME 사운드의 약관은 조사에서 확인되지 않았다(미확인).

---

## 6. 사용자 결정이 필요한 항목

| # | 질문 | 선택지 | 추천안과 이유 |
|---|---|---|---|
| 1 | ModernChic 파생 기본 스킨을 저장소와 릴리스에 넣을 권리가 있습니까? | (a) 원작자 허가를 이미 받았거나 받을 예정 → 5.4 의 A (b) 허가 없음 → E(로컬 생성)로 시작하고 저장소에는 넣지 않음 (c) 자산 자작 교체 F | (b)로 시작하고 허가가 확인되면 A 로 전환. readme 가 스킨 자체의 2차 배포를 금지하고 저장소가 공개 원격을 가진다. 구현 웨이브 1~6 은 이 결정과 무관하게 진행할 수 있다 |
| 2 | 저장소에 레퍼런스 엔진·외부 스킨 이름을 쓰지 않는 기존 규칙(`R:docs/HANDOFF.md` 5절, `R:docs/acknowledge/2026-09-17-skin-system-decisions.md` 참고 자료 취급, 직접 확인)을 어떻게 합니까? | (a) 규칙 유지: 기본 스킨 폴더는 중립 이름, 원작자 표기는 스킨 폴더 안 readme 한 곳에만 (b) 규칙 폐지 (c) 엔진 이름만 계속 금지 | (a). 원작 라이선스가 요구하는 것은 원작자명 표기뿐이라 스킨 폴더 안 readme 로 충족된다. "외부 자산 복사 금지" 항목은 결정 1 의 결과에 맞춰 갱신해야 한다 |
| 3 | 스킨이 없거나 로드에 실패했을 때 무엇을 보여 줍니까? | (a) 현 내장 play/select/result 화면을 폴백으로 영구 유지 (b) 내장 화면 삭제, 오류 안내와 스킨 재선택만 있는 최소 화면 (c) 아주 작은 내장 스킨 | (b). 내장 화면을 유지하면 두 벌의 화면을 계속 관리해야 하고 사용자가 혼합을 부적합으로 판정했다. 단 전환 기간(웨이브 1~6)에는 내장 화면을 그대로 둔다 |
| 4 | 선곡 조작 체계를 어떻게 합니까? | (a) 레퍼런스식만(START/SELECT + 레인 키, NUM 키) (b) 현 키보드 단축키 유지 + 레퍼런스식 추가, 옵션은 스킨 패널로 (c) 현 단축키만 유지하고 패널을 그 키에 대응 | (b). 기존 사용자 조작을 깨지 않으면서 컨트롤러로 패널을 쓸 수 있다. LeftShift/F1 은 패널 1 열기로 재배정 |
| 5 | R-BMS 고유 UI 를 각각 어떻게 둡니까? (옵션 오버레이, 기록 모달, IR 랭킹 패널, 필터 패널, 검색 상자, 코스 탭, 스킨 설정) | 항목별로 (a) 스킨 객체로 대체 (b) 스킨 위 시스템 오버레이 | 옵션 오버레이·검색 표시·IR 랭킹·코스 탭은 (a), 기록 모달·필터 패널은 (b). 스킨 설정은 SKIN 탭을 유지하고 type 9 Stage 는 후순위(ModernChic skinselect 는 skinpreview 가 없어 SKIN 탭보다 나은 점이 적다 [m6 요약]) |
| 6 | 동영상(mp4) 재생을 범위에 넣습니까? 넣는다면 디코더 의존성 추가를 승인합니까? | (a) 넣지 않음(동영상 source 는 미표시) (b) 후속 웨이브에서 지원 (c) 처음부터 지원 | (b). 원본 기본 설정에서 동영상이 쓰이는 곳은 BGA 없는 곡의 범용 BGA 한 곳이고, 기본판은 정지 이미지로 바꾼다. 의존성은 저장소 규칙상 사용자 확인 대상이다 |
| 7 | "똑같이"의 기준을 어디에 둡니까? | (a) 이 저장소의 레퍼런스 HEAD 그대로(결함 포함: 방향키 폴링이 "아무 키"로 동작, 229 버튼 무동작, 비커서 막대 클릭 시 커서 곡 시작) (b) 시각·타이밍은 HEAD 그대로, 명백한 입력 결함은 원작 의도로 교정 | (b). HEAD 의 luajava facade 는 `Input.Keys.RIGHT` 를 -1 로 돌려줘 원작 의도와 다르게 동작한다 [b1 §2.3]. 그리기 의미론의 특이 동작 18개 [b2 §18] 는 그대로 재현한다 |
| 8 | GPU 표면을 비 sRGB(바이트 통과)로 바꿔 스킨 색을 레퍼런스와 맞출 때, 내장 화면 색감이 어두워지는 것을 감수합니까? | (a) 바꾸고 내장 테마 색은 따로 보정 (b) 그대로 두고 스킨 텍스처·색을 sRGB 로 일관 처리 | (a). 블렌딩까지 레퍼런스와 같아지려면 바이트 공간 블렌딩이 필요하다. 다만 색 불일치 자체가 코드 근거이고 실화면은 미확인이므로 웨이브 1 에서 먼저 비교 캡처를 본다 |
| 9 | 창·해상도 정책은? | (a) 기본 창 1280x720 유지 + 해상도·전체 화면 설정 추가 (b) 기본 창을 1920x1080 으로 | (a). 16:9 가 아니면 레터박스를 기본으로(레퍼런스는 가로·세로 독립으로 늘림) |
| 10 | IR 랭킹·라이벌을 이번 범위에 넣습니까? | (a) 이번에는 오프라인 고정값(랭킹 값 = 값 없음, 관련 옵션 false), 실연결은 후속 (b) 이번에 포함 | (a). 해당 id 가 약 100개이고 선곡 사이드메뉴·결과 IR 메뉴에 한정된다 [b5 요약]. `rbms-ir` 크레이트가 있으므로 후속에서 연결 가능 |
| 11 | 기본판의 언어와 사이드메뉴 범위는? | 언어: (a) 일본어 시트 (b) 영어 시트 (c) 한국어 시트 자작. 사이드메뉴: (a) 제거 (b) 설정 + 볼륨 2패널 (c) 전부 유지 | 언어 (b), 사이드메뉴 (b). 문구가 시트 이미지에 구워져 있어 한국어는 자작이 필요하다 [m4 §10]. 사이드메뉴는 객체 약 800개·클로저 417개로 선곡 비용의 대부분이다 |
| 12 | ModernChic 에 문서가 없는 9키(와 24키)는 어떻게 플레이합니까? | (a) 내장 플레이 화면을 9키 폴백으로 유지 (b) 기본판에 9키 문서를 새로 제작 (c) 스킨 없음 안내 | 전환 기간 (a), 최종 (b). 결정 3 의 (b)와 묶이므로 9키 문서가 나오기 전에는 내장 플레이 화면을 지우지 않는다 |
| 13 | 구 번들(steel-neon 3세대)과 사용자 설정의 구 선택값은? | (a) 삭제하고 설정 스키마를 올려 선택 초기화 (b) 호환용으로 남김 | (a). 혼합 합성 전제의 문서라 새 의미론(미지 op 제거 등)에서 그대로 그려지지 않는다 |

조사로 답이 나와 질문에서 뺀 것: Lua 버전(5.2), 파일 쓰기 처리(오버레이), HTTP(항상 실패), `require` 탐색(스킨 루트만), 파일 슬롯 기본값(`def` 를 로드 시 적용), 키 설정 화면(내장 유지), JSON 변환 여부(Lua 유지), 오류 함수 처리(매 프레임 재호출 + 로그 제한), 로드 스레드(1차 동기).

---

## 7. 위험과 미확인 사항

### 7.1 보고서 간 모순과 판정

| # | 모순 | 판정 | 근거 |
|---|---|---|---|
| 1 | mlua 의 `lua52` 피처: b1 "미확인", r1 "있음" | 있음 | 직접 확인. `mlua-0.12.2/Cargo.toml:69` `lua52 = ["ffi/lua52"]`, `lua-src-551.0.2/lua-5.2.4` 존재. b1 은 `~/.cargo` 를 봤고 실제 `CARGO_HOME` 은 `/Users/hyunseokbyun/development/rust/cargo` |
| 2 | ModernChic 의 customTimers/customEvents: b1 "미확인", r1 "선언함", m1 "빈 테이블" | 빈 테이블 | 직접 확인. `M:musicselect.lua:55`, `M:result.lua:38-39`, `M:course.lua:38-39` 가 `{}` 대입뿐이고 `GET_CUSTOMTIMER_ID` 는 정의(`M:Root/customtimer.lua:9`)만 있고 호출이 없다 |
| 3 | 타이머 141: b5 "구현 없음(사용자 옵션 id 가능성)", b3 "차트 미리보기 타이머" | 엔진 타이머 | 직접 확인. `B:play/BMSPlayer.java:482-502` 가 리터럴 141 을 켜고 끈다. `M:Root/maintimer.lua:139` `m.PREVIEW = 141` |
| 4 | Lua 이벤트 함수 인자 수: r1 "0·1·2개를 `narg()` 로 구분", m4 "0개", b1 "항상 1개" | 항상 1개로 구현 | `B:skin/lua/SkinLuaAccessor.java:760` 에 `switch (function.narg())` 가 있음은 직접 확인. `narg()` 가 항상 1 이라는 것은 b1 의 jar 바이트코드 확인이며 이 종합에서 재검증하지 못했다. ModernChic 의 act 함수 35개는 인자를 받지 않아 어느 쪽이든 결과가 같다 |
| 5 | `Sound/` 파일 수: r3 "13개", m1·m6 "14개" | ogg 13 + txt 1 | 직접 확인(`ls`) |
| 6 | 키 설정: r3 "스킨화(D6)", b4·m1·m6 "내장 UI + 헤더만" | 내장 유지 | `B:config/KeyConfiguration.java` 가 내장으로 그림 [b4 §0-10]. `M:keyconfig.lua:19` 가 Decide 의 textproperty 를 require 하고 KeyConfig 의 property 에 `isOutlineFont` 가 없음(직접 확인) |
| 7 | 헤더·본체 Lua 상태: m4 체크리스트 "각각 새 상태", 나머지 "같은 상태" | 같은 상태 | 직접 확인. `B:skin/lua/LuaSkinLoader.java:68-95` 가 `loadHeader(p)` 뒤 같은 `lua` 로 `execFile(p)` |
| 8 | `MAIN.BLEND.*` 값: b2 "미확인" | ALPHA = 1, ADDITION = 2 | 직접 확인. `M:Root/define.lua:21, 23`. ALPHA(1)는 기본 알파 블렌드로 떨어진다 |
| 9 | 스킨 화면 논리 해상도: r2 "물리 뷰포트", r3 "저작 크기 또는 설정 해상도" | 물리 뷰포트 | 레퍼런스가 출력 해상도로 미리 곱한 좌표에서 필터를 판정하고 글자를 그린다 [r2 §4] |
| 10 | 파일 슬롯 기본값: b1 "결정 필요", m1·m5·r1 "`def` 적용" | `def` 를 로드 시 적용 | 레퍼런스에서 `def` 는 설정 화면이 채운다. 설정을 한 번이라도 연 사용자의 실효 동작이 `def` 다 [b1 §10.2] |
| 11 | 전체 용량: r3 "478MB", m6 "476.4 MiB" | 둘 다 맞음 | `du` 블록 합과 `stat` 바이트 합의 차이 |

### 7.2 위험

| # | 위험 | 대응 |
|---|---|---|
| 1 | pcall 침묵 실패. API 하나가 빠지면 부품(예: 결과의 점수 패널 전체, 선곡의 사이드메뉴)이 오류 없이 사라진다 | pcall 실패 로그와 덤프 도구를 웨이브 2 에 넣는다. 덤프의 실패 0 을 게이트로 삼는다 |
| 2 | 조사의 객체 수·좌표는 Lua 실행 없이 손으로 센 값이다 | 덤프 결과를 기준으로 삼는다 |
| 3 | 시각 일치를 판정할 레퍼런스 실행 화면이 없다. 조사는 전부 소스 읽기다 | 가능하면 사용자에게 레퍼런스 캡처를 요청한다. 없으면 좌표 표 대조로 대신한다 |
| 4 | 텍스트 위치. libGDX 1.9.9 소스가 없어 글리프 배치·반올림·세로 기준이 지식 기반이다 [b2 §20] | 웨이브 3 에서 실화면 비교로 보정값을 정한다 |
| 5 | 프레임 비용. 선곡 기본 옵션에서 프레임당 클로저 약 1380 + 타이머 함수 약 420 [m4 요약] | 정적 조건 제거, 타이머 함수 1회 호출 재사용, 실측 뒤 예산 확정. 기본판은 캐릭터·사이드메뉴 축소로 약 90% 감소 |
| 6 | GPU 메모리와 텍스처 한도. 전부 올리면 2.79 GiB, 4096 초과 8장 [r2 §3.4] | 참조 소스만 로드, 화면 이탈 해제, 한도 초과는 생략 + 경고 |
| 7 | 로드 정지. 장면 전환마다 Lua 수천 줄 실행과 텍스처 디코드 | 1차 동기 로드 후 실측. 결정·PRELOAD 연출이 로딩 시간을 가린다 |
| 8 | 웨이브 1 의 철거가 약 50개 앱 테스트와 설치 테스트를 한 번에 무효화한다 [r3 §8.6] | W1-1 을 단독 단위로 두고 철거 전후 내장 화면 캡처 불변을 확인한다 |
| 9 | 크레이트 경계를 넘는 시그니처 변경(타이머 단위, `PropertyRef`, `stretch_rect`)이 병렬 단위의 컴파일을 깨뜨린다 [r1 위험] | 호출부까지 같은 단위가 소유하도록 나눴다(W1-4, W2-1) |
| 10 | 라이선스. 결정 1 이 늦어지면 웨이브 7 이 막힌다 | 웨이브 1~6 은 사용자 로컬의 원본 폴더로 검증한다 |
| 11 | 레퍼런스 특이 동작을 "정상화"해 버릴 위험(구식 LN 명명, 일찍 친 노트 유지, 텍스트 blend 상속, 색 보간 중 알파 오프셋 누락) | b2 §18, b3 §15 목록을 구현 단위 지시에 금지 사항으로 넣는다 |
| 12 | 동영상 미지원 기간에 원본 ModernChic 의 BGA 없는 곡 화면이 비어 보인다 | 경고 표시. 기본판은 정지 이미지 |
| 13 | 스킨 설치 경로에 "Play"/"Select"/"Result" 문자열이 있으면 원본의 로테이션 경로가 틀어진다(`string.match(abs, "Play.+%.png")`) [m1 §12-1] | 원본 결함. 로테이션은 기본 OFF 이고 기본판에서 제거 |
| 14 | 쓰기 오버레이가 `listFiles` 와 `io.open` 에서 같은 가상 루트를 보이지 않으면 로테이션·이력이 어긋난다 | W2-2 테스트에 포함 |

### 7.3 미확인

- 실행 검증 전무: 로드 시간, Lua 메모리, 프레임 비용, 색 공간 차이, 텍스처 한도.
- Lua 5.2 전환 시 mlua 의 `set_memory_limit`·훅 동작과 기존 테스트 호환 [r1 §10].
- LuaJ 고유 동작: `io.open` 실패 반환 형태, `table.keys` 혼합 키 순서, `string.format('%d', 실수)`, 비정수 실수의 문자열화(`Float.toString` 계열이라 5.2 의 `%.14g` 와도 다름) [b1 §15].
- `bms.model` 패키지 소스 부재: Mode, LN 타입 상수, 24키 마지막 두 레인 [b3 §16.3].
- `JudgeManager.java` 1-119, 497-590 행, `GaugeProperty` 의 종류별 max/border/min [b3 §16.3].
- 게이지 이력 샘플 간격. b4 는 미확인, b3 은 "PLAY 중 500ms 마다 게이지 로그"(`B:play/BMSPlayer.java:620-691`)로 적었다. 전 종류 동시 기록 방식은 미확인.
- DECIDE 사운드 정지 시점 [b4 §1.6].
- 하위 객체(lamp/trophy/label)의 draw 가 false 일 때 SkinBar 가 무엇을 그리는지 [b4 §12].
- R-BMS 의 `HudView`/`ResultView` 필드 정의, 키 설정의 START/SELECT 바인딩 유무, W4-3 이 고칠 정확한 엔진 파일.
- DOVA-SYNDROME 사운드, 동영상·배경·bomb 시트의 재배포 조건.
- 이 저장소의 레퍼런스(HEAD 8320241d, 0.8.9)와 ModernChic 4.6 이 겨냥한 버전의 차이. STRING 3 과 200-219(라이벌 목록)는 이 사본에 구현이 없다 [m1 요약].
- blend 3 의 실제 화면 결과(ModernChic 미사용).

---

## 8. 읽은 범위

배정은 보고서 14개 전문 정독이었다. 합계가 12,444줄(약 1.47MB)이라 전부 읽지 못했다. 읽지 못한 구간은 과제 설명에 실린 조사자 요약(요약·핵심·위험·미해결)에 의존했다.

| 보고서 | 읽은 구간(행) | 읽지 못한 구간(행) |
|---|---|---|
| r1-rbms-skin | 전부(1-577) | 없음 |
| r2-rbms-render | 전부(1-499) | 없음 |
| r3-rbms-app | 전부(1-689) | 없음 |
| b1-lua-env | 55-197, 312-713 | 1-54(호출 지점, SkinType), 198-311(변환 규칙 세부, 최상위 필드 표) |
| b2-object-model | 1033-1118 | 1-1032(좌표·보간·stretch·blend·filter·소스·image·number·float·text·slider·graph·마우스의 식 전부) |
| b3-play-objects | 718-953 | 1-717(note·커버·gauge·judge·bga·그래프·practice 의 식 전부) |
| b4-screens | 1-190, 425-654, 963-1026 | 191-424(songlist 스키마·보간·클릭), 655-962(결과·gaugegraph·코스·키 설정·스킨 설정) |
| b5-properties | 1-130, 2033-2201 | 131-2032(id 전수 표 3~14장, 15장의 OP/NUM/STRING/GRAPH/SLIDER 격차 표) |
| m1-root-config | 14-243, 274-303, 337-504 | 1-13, 244-273(main_state 집계 표), 304-336(id 이름표), 505-542(부록) |
| m2-play-sp | 626-725 | 1-625(헤더·테이블 구성·함수 자리·id·레이아웃), 726-946(객체 필드 원문, 엔진 동작) |
| m3-play-dp | 621-692 | 1-620, 693-776 |
| m4-select | 731-885 | 1-730(헤더·객체 수·songlist 필드·id·레이아웃) |
| m5-decide-result | 26-103, 918-998 | 1-25, 104-917(헤더·객체·id·레이아웃·결과 전용 객체 값) |
| m6-misc-assets | 290-523 | 1-289(헤더, skinselect 테이블·레이아웃, Lua 의존) |

구현 단위 지시를 쓸 때는 4절 표의 "사양 근거" 열에 적은 절을 원문으로 다시 읽어야 한다. 특히 b2 전체, b3 1-717, b4 191-424·655-962, b5 3~15장은 이 종합에 식이 옮겨져 있지 않다.

직접 확인한 원본: `B:skin/lua/LuaSkinLoader.java:68-95`, `B:skin/lua/SkinLuaAccessor.java:760`, `B:play/BMSPlayer.java:482-502`, `M:Root/define.lua:21-23`, `M:Root/maintimer.lua:139`, `M:Root/customtimer.lua:9`, `M:musicselect.lua:55`, `M:result.lua:38-39`, `M:course.lua:38-39`, `M:keyconfig.lua:17-21`, `M:Sound/` 목록, `R:crates/rbms-skin/Cargo.toml:19`, `R:Cargo.lock:1864-1866, 1967-1970`, `R:Cargo.toml:25`, `R:docs/HANDOFF.md` 4~6절, `R:docs/acknowledge/2026-09-17-skin-system-decisions.md` 참고 자료 취급 절, mlua 0.12.2 `Cargo.toml` 피처 목록, lua-src 551.0.2 디렉터리 목록, `R:` 의 `crates/`·`apps/`·`assets/skins/` 목록, git 원격.
