# B1. beatoraja Lua 스킨 로딩 파이프라인과 Lua 실행 환경 계약

> 최종 갱신 2026-10-10 · 대응 단계: 웨이브 4(결과와 코스 결과) 반영 · 본문은 작성 시점 서술이며, 아래 "웨이브 … 반영 사항"이 최신 것부터 우선한다 · 색인과 갱신 규칙은 [README.md](README.md)

## 웨이브 4 반영 사항 (2026-10-10)

스킨 입력 디스패치와 이벤트 실행기, gauge·gaugegraph·timingdistributiongraph·judgegraph type 1·2, 플레이 엔진 기록 확장, Result·CourseResult Stage 의 장면 수명, 호스트 군집 B·G·H·E 일부, 스킨 사운드 버스를 넣은 뒤의 상태다.

- (W4-1) b1-lua-env.md §5.3 표: 'act (클릭)' 과 '슬라이더·텍스트 event' 행에 R-BMS 구현 위치를 추가합니다 — skin_screen.rs run_skin_actions → BoundFrame::call_event(인자 1개) / call_float_writer, 프레임 바인딩 안 prepare 전
- (W4-1) b1-lua-env.md §2.3: Gdx.input:isKeyPressed 와 main_state.key_pressed 가 이제 앱의 실제 키 상태를 답한다는 점을 추가합니다(libGDX 코드 → winit KeyCode 표, 키보드에 없는 코드는 false)
- (W4-7) b1-lua-env.md §6 audio_* 표: 구현 위치를 skin_host/audio.rs 로 적는다. audio_preload 는 볼륨 0 재생이라 소리 없이 디코드만 하고, audio_play/loop 의 클램프된 볼륨이 0.001 이하이거나 NaN 이면 프리로드로 처리한다. 이미 루프 중인 소리에 audio_loop 를 또 부르면 무시하고, audio_stop 은 모든 복사본을, audio_dispose 는 정지와 캐시 제거를 한다. 파일이 없으면 wav, flac, ogg, mp3 순으로 같은 이름의 다른 확장자를 찾는다.


## 웨이브 3A 반영 사항 (2026-10-10)

prepare/draw 2단계 파이프라인과 `SkinHost` 직접 그리기, 그리기 조건 의미론, 참조 이미지·음수 크기·이미지 인덱스·숫자·슬라이더·그래프 정합, TTF 텍스트, judgegraph·bpmgraph, Lua 함수 값 프레임 평가, 앱 호스트 군집 A·I·M 을 넣은 뒤의 상태다.

- (W3-1a) b1-lua-env.md §5.3: 표의 'value 함수 | 각 객체의 prepare | 객체별 구현, 이 조사 범위 밖' 행을 b2 §5.3 의 종류별 순서로 대체하고, R-BMS 구현 위치(crates/rbms-render/src/skin_render/object.rs SkinObject::prepare)와 '타이머 함수 프레임당 객체당 2회는 미이식(1회), W3-1c' 를 반영 사항에 추가
- (W3-1b) b1-lua-env.md §5.3: 'isStatic 인 조건은 한 번만 평가 … 맵에 없는 id 는 어느 부호든 제거' 문단에 R-BMS 구현 완료와 위치(loader.rs prepare_objects·options_hold·settle_static)를 적고, 'destination.draw 가 숫자인데 내장 속성이 없으면 조건 없음(옵션 맵 대조 안 함, LuaSkinLoader.serializeLuaScript 의 byId 가 null)' 과 '중첩 객체(SkinBar·SkinJudge·SkinNote 내부)는 Skin.prepare 를 거치지 않아 dstop 이 영영 검사되지 않고 정적 조건도 매 프레임 평가된다' 를 추가
- (W3-1c) b1-lua-env.md §5.3: 'Destination.timer 함수' 행에 R-BMS 동작 추가 — 프레임당 함수당 1회 호출 후 같은 프레임의 모든 읽기(다른 객체, 이미지 셀 애니메이션 포함)에 재사용(lua/mod.rs BoundFrame::timer). 조건·값 함수는 객체마다 원본 횟수대로 호출
- (W3-1c) b1-lua-env.md §5.1: R-BMS 차이 추가 — 타이머 함수 실패는 그 프레임 동안 OFF 로 재사용되고 프레임당 1회만 기록, 다음 프레임에 재호출. 예산이 거부·중단한 타이머는 직전 프레임 값
- (W3-1c) b1-lua-env.md §7.1: 'destination 의 timer 로 쓰면 프레임당 2회 호출되지만 결과가 같다' 뒤에 'R-BMS 는 1회만 호출한다. 원본 방식(4회)과 1회 재사용의 답이 프레임마다 같음을 테스트로 확인(skin_main_state.rs)' 추가
- (W3-1c) b1-lua-env.md §8.2: '함수를 destination 에 직접 넣으면 객체마다 따로 호출된다'에 'R-BMS 는 같은 함수 값이면 프레임당 1회' 추가
- (W3-1c) b1-lua-env.md §14: 요약에 실제 준비 프레임 비용 추가 — 릴리스 빌드 선곡 1,401호출 207~251µs, 결과 98호출 22~34µs, 플레이 7키 23호출 8~15µs, 결정 1호출
- (리뷰 수정) b1-lua-env.md §6 의 float_number 행: R-BMS 구현 상태로 '원본에 float·rate 속성이 없는 id 와 이름은 0, 있는 id 는 호스트 값(무값 센티널 포함)' 을 적습니다


## 웨이브 2B 반영 사항 (2026-10-10)

스킨 덤프 CLI, 앱의 스킨 팩 폴더 지정과 `.luaskin` 로드, 오버레이 총 크기 상한, 외부 스킨 첫 정지 프레임을 넣은 뒤의 상태다.

- (W2-8) b1-lua-env.md §14-2: 헤더 전용 읽기의 앱 쪽 구현 위치(apps/rbms-player/src/skin_select.rs HeaderCache, 키 = 경로 + 수정 시각 + 크기)를 추가
- (W2-8) b1-lua-env.md §10.3: '무작위 옵션·무작위 파일은 화면에 들어갈 때마다 다시 뽑힌다' 에 R-BMS 상태 추가 — Lua 스킨은 장면 시작마다 다시 읽혀 다시 뽑히고, JSON 문서는 아직 앱 수명 동안 한 번만 읽힘


## 웨이브 2A 반영 사항 (2026-10-10)

Lua 5.2 런타임(`crates/rbms-skin/src/lua/`), `SkinHost`, Lua 값 변환기, 2패스 `.luaskin` 로더를 넣고 구 샌드박스(`skin.*`)를 삭제한 뒤의 상태다.

- (W2-0) b1-lua-env.md §14-5·§14-6: 스칼라 변환 규칙의 구현 위치를 `crates/rbms-skin/src/lua/coerce.rs`, 함수 호출 규약의 구현 위치를 `lua/mod.rs` 의 `BoundFrame::call_*` 로 명시. §2.3 Controllers 행에 'R-BMS 는 스킨에 컨트롤러 없음으로 보고한다(계약)' 를 추가.
- (W2-1) b1-lua-env.md §5.3 표 'value 함수' 행: '객체별 구현, 이 조사 범위 밖' 에 'SkinNumber.prepare 는 값 함수를 그리기 조건보다 먼저 호출한다(SkinNumber.java:134-150)' 를 추가. 사양 §4.3 의 '조건 통과 객체에 한해 op/draw → timer → 값 함수' 문장과 어긋난다
- (W2-2a) b1-lua-env.md §2.1: R-BMS 구현 상태 추가. base·table·string·math·bit32·coroutine 은 벤더 Lua 5.2.4 에서 열고 io·os·package 는 Rust/Lua 로 자체 구현. 벤더 빌드에 LUA_COMPAT_ALL 이 없어 unpack·loadstring·math.log10 이 LuaJ 와 똑같이 없음을 테스트로 확인. print 는 진단 로그로, debug 는 { getmetatable } 로 env.rs 가 설치. _VERSION 은 'Lua 5.2'(LuaJ 는 'Luaj-jse 3.0.2')
- (W2-2a) b1-lua-env.md §2.4: R-BMS 규칙 추가. 상대 경로는 작업 디렉터리가 아니라 항상 스킨 루트 기준, 역슬래시도 구분자, 청크 이름은 `@<루트 상대 경로>`, 실패 메시지는 `cannot open <경로>: <사유>`. require 는 package.path 를 읽지 않으며(표시용으로만 `?.lua;<루트>/?.lua` 설정) 루트 밖 이름은 'module not found' 로 끝남
- (W2-2a) b1-lua-env.md §5.1: R-BMS 차이 추가. 삼켜진 오류는 pcall·xpcall·coroutine.resume 에서 기록하고, 예산 초과 오류는 이들과 load 가 다시 던짐. Rust 함수가 던진 오류는 pcall 에서 문자열로 바뀜. 예산이 거부·중단한 호출은 기본값이 아니라 직전 값
- (W2-2a) b1-lua-env.md §12: '난수 시드' 행과 'math.random(m) 에 실수 인자' 행을 'R-BMS 는 인터프리터별 자체 생성기(SplitMix64)로 교체, 인자는 절삭, 시드 없으면 RBMS_SKIN_SEED 또는 시계'로 갱신. 'x % 0' 행의 미확인을 'Lua 5.2 에서 NaN 임을 테스트로 확인'으로. 선택지 2 는 채택·구현 완료로 표시
- (W2-2a) b1-lua-env.md §13: 'dofile 109회, pcall 110회' 행 근처에 'Lua 파일은 .lua 126개 + .luaskin 10개 = 136개, 전부 CRLF, BOM 없음, Lua 5.2 텍스트 청크로 전부 컴파일됨(실측)' 추가
- (W2-2a) b1-lua-env.md §15: 'LuaJ 소스 인코딩 처리와 BOM 처리' 미확인 항목에 'R-BMS 는 UTF-8 BOM 을 제거하고 바이트 그대로 넘김' 추가. 'mlua 0.12 의 lua52 기능 제공 여부'는 확인됨으로
- (W2-4) b1-lua-env.md §4.1: 배열 행에 R-BMS 구현 규칙을 추가해야 합니다. '양의 정수 키 값을 오름차순으로 먼저, 그 뒤 나머지 키 값을 인터프리터 순회 순서로. 시퀀스(구멍 포함)는 LuaJ 와 같고, 섞인 키의 상대 순서는 LuaJ 해시 순서를 재현하지 않는다.' 객체 행에는 '문자열 키만 필드명과 비교한다(숫자·boolean 키의 문자열화가 필드명과 같을 수 없으므로 결과 동일)'를 추가합니다.
- (W2-4) b1-lua-env.md §4.2: 'R-BMS 구현' 주석이 필요합니다. coerce::to_jstring 이 실수를 Float.toString((float)d) 형식(지수 표기 포함)으로, nan/inf/-inf 와 -0.0 → "0" 으로 냅니다. 테이블·함수는 'table: ...' 가 아니라 타입 이름만 냅니다.
- (W2-4) b1-lua-env.md §4.3: 표 아래에 R-BMS 대응을 추가해야 합니다. TimerProperty 의 음수 id 는 변환기에서 필드 미설정, StringWriter 의 숫자는 컴파일 실패 경고 후 미설정, 이름 조회는 FromLua.known_name(LuaFnKind, 이름)으로 주입받고 불리언은 선행 '!' 를 모두 뗀 뒤 조회하되 Name 에는 스킨이 쓴 철자 그대로 보관, 컴파일 실패 시 op 원소는 id 0·property 없음.
- (W2-4) b1-lua-env.md §4.6: 마지막 문단 'R-BMS 는 키 없음을 별도 상태(Option)로 표현하는 편이 안전하다'를 '구현됨: 키가 있으면 어떤 값이든 Some(toint), 없으면 None. 값이 정확히 Integer.MIN_VALUE 인 경우만 원본(미지정)과 다르다'로 바꿔야 합니다.
- (W2-2b) b1-lua-env.md §2.2: 'LuaJ 의 io.open 은 IOException 을 nil, 메시지 반환으로 바꾼다(… 이 jar 의 바이트코드로는 미확인)' 을 '확인됨'으로 바꿔야 합니다. javap 결과: IoLib.errorresult 는 `nil, "io error: <메시지>"` 두 값(errno 없음). IoLibV.invoke 가 IOException 을 잡아 이 형태로 바꾸되 LINES_ITER(줄 반복자)만 error 로 raise 합니다. io.lines·io.input·io.output 의 파일 열기는 ioopenfile 이 `error("io error: …")` 로 raise 합니다.
- (W2-2b) b1-lua-env.md §2.2: 'io.popen 금지' 는 raise 가 아닙니다. openProgram 의 IOException 이 errorresult 를 타서 `nil, "io error: Lua io.popen is not allowed"` 가 반환됩니다. 모드가 r·w 가 아니면 argerror 입니다.
- (W2-2b) b1-lua-env.md §2.2: 이 jar(luaj-jse-3.0.2-custom)의 IoLib 는 공개 3.0.2 보다 새 동작을 갖습니다. 추가할 내용: (1) rawopenfile 이 모드를 검증(첫 글자 r/w/a, 둘째 +, 이후 b; 아니면 argerror), (2) read 형식은 길이 2 이상이고 '*' 로 시작하며 둘째 글자가 n/l/L/a, 숫자는 바이트 수, 인자 없으면 한 줄, 첫 nil 에서 중단, (3) '*L' 지원, (4) io.lines(name, 형식…)은 끝에서 파일을 닫고 file:lines() 는 닫지 않음, (5) write 는 파일 자신을 반환, close 는 true, 표준 스트림 close 는 `nil, "cannot close standard file"`, (6) freadnumber 는 peek 가 EOF 에서 EOFException 을 던져 숫자가 파일 끝에 닿으면 `nil, "io error: java.io.EOFException"`.
- (W2-2b) b1-lua-env.md §2.2: R-BMS 구현 차이를 추가해야 합니다. 상대 경로는 항상 스킨 루트 기준(작업 디렉터리 규칙 (a) 미적용), 쓰기·추가·r+ 는 오버레이, a·r+ 는 루트 파일을 오버레이로 먼저 복사, stdout·stderr 쓰기는 버리지 않고 진단 로그에 기록, flush 는 fsync 안 함, tmpfile 은 오버레이 최상위, 읽기 1회 64MiB 상한.
- (W2-2b) b1-lua-env.md §2.1 os 행: 'clock, date, difftime, setlocale, time 만 남는다' 뒤에 LuaJ 의미를 추가해야 합니다. clock = (currentTimeMillis - 클래스 로드 시각)/1000.0 인 벽시계 경과 초, time() = currentTimeMillis/1000.0 인 소수 초, setlocale 은 인자와 무관하게 "C". R-BMS 는 이 셋을 Rust 로 같게 구현했고 date·difftime·time(table) 은 C 라이브러리를 씁니다.
- (W2-2b) b1-lua-env.md §12 표의 'os.time()' 행: LuaJ 열 '초 단위 수' 를 'ms 를 소수로 가진 초(double)' 로 바꾸고, R-BMS 가 같은 값을 돌려준다고 적어야 합니다.
- (W2-2b) b1-lua-env.md §15 미확인 목록: 첫 항목 'LuaJ IoLib 가 IOException 을 nil, 메시지 로 바꾸는 정확한 반환 형태' 를 지워야 합니다(확인됨, 두 값).
- (W2-3) b1-lua-env.md §2.5: 'R-BMS 구현' 주석 추가 — Full 모드는 헤더·본체 두 패스 모두 실제 main_state 테이블이고, 호스트가 묶이지 않은 호출과 HeaderOnly 모드의 호출은 둘 다 'attempt to call ... a nil value' 오류다. main_state 는 전역이 아니라 package.loaded 에만 있다.
- (W2-3) b1-lua-env.md §6.1 number 행: '없으면 0' 을 '팩토리에 속성이 없는 id·이름은 0, 속성은 있고 값이 없으면 -2147483648' 로 구체화. R-BMS 는 property::reference_implements(Integer, id) 로 가른다.
- (W2-3) b1-lua-env.md §6.1 key_pressed 행: '이름은 libGDX 1.9.9 Input.Keys.toString 표시 이름(Left, Up, Space, L-Shift, Numpad 0, F1 … 총 112개, 코드 0~255)' 을 jar 확인으로 추가. 숫자를 적은 문자열은 코드로 읽힌다(isnumber 가 먼저).
- (W2-3) b1-lua-env.md §6.2 event_exec 행: 원본은 인자 0개 또는 4개 이상에서 VarArgFunction 의 invoke/onInvoke 가 서로를 불러 반환하지 않는다는 점과, R-BMS 는 그 경우 Lua 오류를 낸다는 점을 추가.
- (W2-3) b1-lua-env.md §6.2 file_list 행: R-BMS 는 패턴을 Java 정규식으로 바꾸지 않고 Lua 패턴(string.find)으로 거른다는 의도적 차이를 추가. file_* 쓰기는 오버레이로만 가고 file_append 는 루트 원본을 오버레이로 복사한 뒤 덧붙인다.
- (W2-3) b1-lua-env.md §6.2 set_timer 행: R-BMS 는 값을 인터프리터 안(lua/main_state.rs 의 커스텀 타이머 저장소)에 두고 호스트에 알리지 않으며 custom_timer_us 로 읽는다.
- (W2-3) b1-lua-env.md §7: 구현 위치 추가 — crates/rbms-skin/src/lua/prelude.lua(순수 Lua). tolong 은 Lua 로 옮겼고 event_min_interval 의 간격은 32비트로 좁히지 않는다.
- (W2-3) b1-lua-env.md §14: 요약에 '바인딩 = 영구 트램펄린 + 프레임당 scope 함수 38개, 릴리스 실측 바인딩 약 10µs/프레임·호출 약 80ns' 추가.
- (W2-2c) b1-lua-env.md §2.3: 'Input.Keys.<NAME> 은 valueOf 를 부른다 ... RIGHT 는 -1' 은 원본 HEAD 서술로 맞지만, R-BMS 는 E7 에 따라 상수 이름(UP 19, DOWN 20, LEFT 21, RIGHT 22)을 먼저 보고 표시 이름을 폴백으로 받으며 모르는 이름은 -1 입니다. '이 숫자는 jar 로 미확인' 문구는 '확인됨: gdx.jar 의 Input$Keys 상수 155개 중 DPAD_*, UP/DOWN/LEFT/RIGHT 가 19~22'로 바꿔야 합니다.
- (W2-2c) b1-lua-env.md §2.3 표(luajava.new): R-BMS 의 File:mkdir 은 병합 뷰 기준 Files.createDirectory 의미(이미 있음/부모 없음/오버레이 없음은 false)이고 오버레이에 부모 사본을 만든 뒤 한 단계만 만든다는 설명을 추가해야 합니다.
- (W2-2c) b1-lua-env.md §2.3 세부: URL 연결의 R-BMS 처리를 추가해야 합니다. setRequestMethod(GET 만), setConnectTimeout(숫자 검증)은 원본대로 동작하고 connect/getResponseCode/getInputStream 은 항상 'Legacy Lua skin HTTP connection failed: network access is not available to skins' 입니다.
- (W2-5) b1-lua-env.md §14 요약 10번: 'def 를 로드 시점에 적용할지 결정 필요' → '로드 시점에 스템 대조로 적용, 미해결 슬롯은 로드당 1회 추첨 고정'으로 확정
- (W2-5) b1-lua-env.md §3.3: R-BMS 구현 주석 추가 — skin_config.offset 값은 오프셋 id 로 저장된 사용자 값을 이름 키로 게시. get_path 는 스킨 루트 절대 경로를 돌려주고 Java File.getPath 처럼 중복 구분자·끝 구분자를 정리. 슬롯이 덮지 않는 와일드카드는 호출마다 추첨
- (W2-5) b1-lua-env.md §2 표(샌드박스 모드): R-BMS 헤더 전용 읽기는 luajava 를 설치한 채 실행함을 주석으로 추가(ModernChic 헤더 패스 10개 모두 통과)
- (리뷰 수정) b1-lua-env.md §2.1(표준 라이브러리 표, 91행 부근 base 행과 string 행): R-BMS 쪽 차이로 `setmetatable` 이 `__gc` 를 종료자로 등록하지 않는다는 점과 `string.find/match/gmatch/gsub` 가 자체 구현이라는 점을 적어야 합니다
- (리뷰 수정) b1-lua-env.md §2.2(io)와 main_state 파일 API 표(419행 부근 `file_list` 가 있는 표): `file_read_lines` 의 64MiB 상한, `file_count_lines` 의 스트리밍, `file_append` 가 원본 권한을 물려받지 않는 사본을 만든다는 점, io 의 동시 열기 64개 상한을 R-BMS 동작으로 추가해야 합니다


## 웨이브 1A 반영 사항 (2026-10-09)

혼합 합성과 구 번들을 삭제한 뒤의 상태다. 본문의 해당 절은 아래 내용으로 읽는다.

- (W1-3) b1-lua-env.md §12 첫 문단 'R-BMS 현재 설정: features = ["lua54", "vendored"]' 이 낡았습니다(현재 lua52). 같은 절 선택지 2의 '미확인' 문구(mlua 0.12 lua52 존재 여부, 기존 샌드박스가 5.4 전용 API 에 의존하는지)는 '확인됨: 존재하며 샌드박스는 5.4 전용 API 에 의존하지 않음'으로 바꿔야 합니다. 5.2 에서 10/2 연결이 "5", -2^63 double 뺄셈이 순환하지 않음을 테스트로 실행 검증했다는 사실을 반영할 수 있습니다.


상태: 조사 완료(읽기 전용). 배정 파일 전부 정독. 근거 없는 항목은 "미확인"으로 표시.

## 0. 기준과 표기

- 기준 소스: `/Users/hyunseokbyun/development/beatoraja` (origin `exch-bms2/beatoraja`, HEAD `8320241d`, 작업 트리 변경 없음).
- 경로 약어: `B/` = `/Users/hyunseokbyun/development/beatoraja/src/bms/player/beatoraja/`, `MC/` = `/Users/hyunseokbyun/Downloads/ModernChic/`, `R/` = `/Users/hyunseokbyun/development/R-BMS/`.
- LuaJ: `/Users/hyunseokbyun/development/beatoraja/lib/luaj-jse-3.0.2-custom.jar`. `_VERSION` 문자열은 `Luaj-jse 3.0.2` (BaseLib.class 상수 풀). 문법·표준 라이브러리는 Lua 5.2 계열이다(`bit32`, `table.unpack`, `package.searchers`, `rawlen` 존재 / `unpack`, `loadstring`, `setfenv` 없음).
- libGDX: `lib/gdx.jar` 의 `com/badlogic/gdx/Version.class` 상수 `1.9.9`.
- JRE 가 없어 `javap` 를 쓸 수 없었다. jar 는 스크래치패드(`scratchpad/b1-luaj`, `b1-gdx`)에 풀고, 직접 만든 클래스 파일 파서(`scratchpad/b1-tools/*.py`)로 메서드 목록·상수 풀·바이트코드를 읽어 확인했다. 이 방식으로 확인한 항목은 "(jar 확인)"으로 표시한다. 세 대상 디렉터리에는 아무것도 쓰지 않았다.

중요한 전제: 이 HEAD 는 예전 beatoraja(0.8.x)와 Lua 환경이 다르다. 진짜 `luajava` 는 제거되고 제한된 흉내(facade)만 남았으며, `io` 는 스킨 폴더 안으로 제한되고 `os` 는 일부 함수가 지워졌다. ModernChic 은 예전 환경(진짜 luajava)을 전제로 작성된 스킨이다. 13장 참고.

---

## 1. 로딩 파이프라인 전체 흐름

### 1.1 호출 지점

| 단계 | 위치 | 내용 |
| --- | --- | --- |
| 상태 전환 | `B/MainController.java:248-284` | `changeState` → `current.shutdown()`, `current.setSkin(null)` → `newState.create()` → `newState.getSkin().prepare(newState)` → `timer.setMainState(newState)` → `current.prepare()` |
| 스킨 로드 요청 | `B/MainState.java:137-139` | `loadSkin(SkinType)` = `setSkin(SkinLoader.load(this, skinType))`. 각 상태의 `create()` 에서 호출 |
| 설정 선택 | `B/skin/SkinLoader.java:42-54` | `PlayerConfig.getSkin()[skinType.getId()]` 의 `SkinConfig` 로 로드. 실패(null)면 `SkinConfig.Default` 경로로 새 `SkinConfig` 를 만들어 재시도 |
| 확장자 분기 | `B/skin/SkinLoader.java:56-86` | `.json` → `JSONSkinLoader(state, config)`, `.luaskin` → `LuaSkinLoader(state, config)`, 그 외 → LR2 CSV. 예외는 전부 삼키고 null |
| Lua 로드 | `B/skin/lua/LuaSkinLoader.java:68-95` | `load(p, type, property)` (아래 3장) |
| 헤더만 읽기(게임 내 스킨 선택) | `B/config/SkinConfiguration.java:489-531` | `skin/` 아래를 재귀 스캔(`.lr2skin`/`.luaskin`/`.json`), `.luaskin` 은 `LuaSkinLoader.sandboxed(path).loadHeader(path)` |
| 헤더만 읽기(런처) | `B/launcher/SkinConfigurationView.java:176-190` | `.luaskin` 은 `new LuaSkinLoader().loadHeader(path)` (샌드박스 아님) |
| 스킨 적용 | `B/MainState.java:116-135` | `setSkin` 이 이전 스킨 dispose 후, `skin.getOffset()` 의 사용자 오프셋 값을 `main.getOffset(id)` (전역 `SkinOffset[200]`)에 복사 |
| 프레임 | `B/MainController.java:401-413` | `timer.update()` → `current.render()` → `sprite.begin()` → `skin.updateCustomObjects(current)` → `skin.drawAllObjects(sprite, current)` → `sprite.end()` |
| 입력 | `B/MainController.java:496-508` | 렌더 뒤에 `current.input()`, 마우스 눌림/드래그를 `skin.mousePressed/mouseDragged` 로 전달 |

`SkinConfig.validate()` 는 경로가 쓸 수 있는 문자열인지만 보고 `properties` 가 null 이면 빈 `Property` 를 만든다(`B/SkinConfig.java:44-53`). 파일 존재 여부는 검사하지 않는다.

### 1.2 SkinType id (`B/skin/SkinType.java:12-30`)

| id | 이름 | id | 이름 |
| --- | --- | --- | --- |
| 0 | PLAY_7KEYS | 10 | SOUND_SET |
| 1 | PLAY_5KEYS | 11 | THEME |
| 2 | PLAY_14KEYS | 12 | PLAY_7KEYS_BATTLE |
| 3 | PLAY_10KEYS | 13 | PLAY_5KEYS_BATTLE |
| 4 | PLAY_9KEYS | 14 | PLAY_9KEYS_BATTLE |
| 5 | MUSIC_SELECT | 15 | COURSE_RESULT |
| 6 | DECIDE | 16 | PLAY_24KEYS |
| 7 | RESULT | 17 | PLAY_24KEYS_DOUBLE |
| 8 | KEY_CONFIG | 18 | PLAY_24KEYS_BATTLE |
| 9 | SKIN_SELECT | | |

헤더의 `type` 이 `-1`(기본값)이면 헤더가 만들어지지 않는다(`B/skin/json/JSONSkinLoader.java:118`). 본 로드에서 객체 로더를 고르는 기준은 스킨이 반환한 `type` 이 아니라 호출자가 요청한 `SkinType` 이다(`JSONSkinLoader.java:275-284`). 둘이 일치하는지는 검증하지 않는다.

---

## 2. Lua 실행 환경

`SkinLuaAccessor` 생성자가 둘이고 환경이 다르다.

| 구분 | 표준 모드 | 샌드박스 모드 |
| --- | --- | --- |
| 생성 | `new SkinLuaAccessor(false)` (`B/skin/lua/SkinLuaAccessor.java:46-54`) | `new SkinLuaAccessor(false, sandboxRoot)` (`:56-66`) |
| 쓰이는 곳 | 실제 스킨 로드(`LuaSkinLoader(state, config)`), 런처 헤더 읽기(`new LuaSkinLoader()`) | 게임 내 스킨 선택 화면의 헤더 목록(`LuaSkinLoader.sandboxed`) |
| base | `JseBaseLib` | `BaseLib` |
| package | `PackageLib` | `PackageLib` + `globals.finder = SkinResourceFinder(root)` |
| bit32/table/coroutine | 있음 | 있음 |
| string | `JseStringLib` | `StringLib` |
| math | `JseMathLib` | `MathLib` (acos/asin/atan/atan2/cosh/sinh/tanh/log 없음, jar 확인) |
| io | `RestrictedIoLib` (스킨 폴더 안에서 실제 읽기·쓰기) | `SandboxIoLib` (루트 안 읽기만, 쓰기는 메모리에 버림) |
| os | `SafeOsLib` (일부 함수만) | nil |
| luajava | 제한 facade 테이블 (`LegacySkinLuaApi.install`) | nil |
| debug | `{ getmetatable = getmetatable }` 만 | nil |
| package.path | 기존값 뒤에 `;<스킨폴더>/?.lua` 추가 | `<폴더>/?.lua;<폴더>/?/init.lua` 로 교체 |

근거: `createStandardGlobals` `:68-82`, `restrictStandardGlobals` `:84-93`, `createSandboxGlobals` `:104-117`, `restrictPackageLoaders` `:119-129`, `setDirectory` `:855-871`.

ModernChic 을 실제로 그리는 경로는 표준 모드다. R-BMS 는 표준 모드를 재현 대상으로 삼으면 된다. 샌드박스 모드는 "헤더만 읽는 가벼운 실행"의 참고로만 본다.

### 2.1 전역과 표준 라이브러리 (표준 모드, jar 확인)

| 라이브러리 | 제공 이름 |
| --- | --- |
| base | `_G`, `_VERSION`, `assert`, `collectgarbage`, `dofile`, `error`, `getmetatable`, `load`, `loadfile`, `pcall`, `print`, `rawequal`, `rawget`, `rawlen`, `rawset`, `select`, `setmetatable`, `tonumber`, `tostring`, `type`, `xpcall`, `next`, `pairs`, `ipairs` |
| base 에 없는 것 | `unpack`, `loadstring`, `setfenv`, `getfenv`, `module` |
| package | `require`, `package.loaded`, `package.preload`, `package.path`, `package.searchers`, `package.searchpath`, `package.config`. `package.loadlib = nil`, `package.cpath = ""`, `searchers[3]`·`[4]` 제거(자바 클래스 검색 차단) |
| table | `concat`, `insert`, `pack`, `remove`, `sort`, `unpack` |
| string | `byte`, `char`, `dump`, `find`, `format`, `gmatch`, `gsub`, `len`, `lower`, `match`, `rep`, `reverse`, `sub`, `upper` |
| math | `abs`, `ceil`, `cos`, `deg`, `exp`, `floor`, `fmod`, `frexp`, `huge`, `ldexp`, `max`, `min`, `modf`, `pi`, `pow`, `rad`, `random`, `randomseed`, `sin`, `sqrt`, `tan` + Jse 추가분 `acos`, `asin`, `atan`, `atan2`, `cosh`, `log`, `sinh`, `tanh`. `log10` 은 없다 |
| bit32 | `arshift`, `band`, `bnot`, `bor`, `btest`, `bxor`, `extract`, `lrotate`, `lshift`, `replace`, `rrotate`, `rshift` |
| coroutine | `create`, `resume`, `running`, `status`, `wrap`, `yield` |
| os | `clock`, `date`, `difftime`, `setlocale`, `time` 만 남는다. `execute`, `exit`, `getenv`, `remove`, `rename`, `tmpname` 은 nil (`SkinLuaAccessor.java:138-150`) |
| io | `close`, `flush`, `input`, `lines`, `open`, `output`, `popen`, `read`, `tmpfile`, `type`, `write`. 파일 메서드 `close`, `flush`, `lines`, `read`, `seek`, `setvbuf`, `write` |

`print` 는 표준 출력으로 나간다(JseBaseLib). 스킨이 디버그 출력에 쓰므로 R-BMS 는 로그로 돌리면 된다.

### 2.2 io 제한 (`RestrictedIoLib`, `SkinLuaAccessor.java:317-599`)

- 루트는 `setDirectory` 가 넘긴 스킨 폴더의 절대 정규화 경로(`:868`).
- 경로 해석(`resolve`, `:376-392`): 절대 경로면 정규화만. 상대 경로면 (a) 프로세스 작업 디렉터리 기준 절대 경로가 루트 아래이면 그것을 쓰고, (b) 아니면 루트 기준으로 해석한다. 결과가 루트 밖이면 `IOException("Lua skin file access denied: ...")`.
- 읽기 모드에서 파일이 없으면 `IOException("Lua file not found: ...")`. LuaJ 의 `io.open` 은 IOException 을 `nil, 메시지` 반환으로 바꾼다(표준 LuaJ 동작, 이 jar 의 바이트코드로는 미확인). ModernChic 은 `io.open(path, "r") == nil` 로 존재 여부를 판정한다(`MC/Root/customfunction.lua:18-28`).
- 쓰기·추가 모드는 부모 폴더를 자동 생성한다(`:349-354`). `"w"` 는 길이 0 으로 자르고 `"a"` 는 끝으로 이동(`:356-361`).
- `io.popen` 금지(`:371-374`), `io.tmpfile` 은 루트 안에 `lua-*.tmp` 생성(`:365-369`).
- `stdin` 은 빈 메모리 파일, `stdout`/`stderr` 는 버려지는 메모리 파일(`:324-337`). 즉 `io.write(...)` 는 아무 데도 나가지 않는다.

### 2.3 luajava facade (`B/skin/lua/LegacySkinLuaApi.java`)

`luajava` 전역과 `package.loaded.luajava` 양쪽에 같은 테이블이 들어간다(`:51-59`). 따라서 `require("luajava")` 가 동작한다.

| 호출 | 허용 인자 | 반환 |
| --- | --- | --- |
| `luajava.bindClass(name)` (`:66-77`) | `"com.badlogic.gdx.Gdx"` | `{ graphics = { getWidth(), getHeight() }, input = { isKeyPressed(...) } }` (`:295-321`) |
| | `"com.badlogic.gdx.Input"` | `{ Keys = <__index 로 이름 조회하는 테이블> }` (`:323-340`) |
| | `"com.badlogic.gdx.controllers.Controllers"` | `{ getControllers() → { size, first() } }`, `first()` 는 `{ getButton(...), getName() }` 또는 nil (`:342-377`) |
| | `"com.badlogic.gdx.controllers.Controller"`, `"java.io.File"` | `{ __legacy_class = name }` 표식 테이블 |
| | 그 외 | Lua 오류 `Legacy Lua skin class access denied` |
| `luajava.new(classFacade, path)` (`:85-104`) | `java.io.File` 표식만 | `{ mkdir(), listFiles() }` (`:118-148`). 경로는 스킨 폴더 안으로 제한 |
| `luajava.newInstance(name, arg)` (`:106-116`) | `"java.net.URL"` | `{ openConnection() }` → `{ setRequestMethod, setConnectTimeout, connect, getResponseCode, getInputStream }` (`:150-198`) |
| | `"java.io.InputStreamReader"` | 두 번째 인자를 그대로 돌려준다 |
| | `"java.io.BufferedReader"` | 두 번째 인자(테이블)를 그대로 돌려준다. `readLine()` 을 가진 테이블이어야 함 |

세부:
- `File:mkdir()` 은 `Files.createDirectory` 성공 시 true, 실패(이미 있음 포함) 시 false. `File:listFiles()` 는 1 부터 시작하는 문자열 배열(전체 경로, `\` 는 `/` 로 치환)이며 실패 시 nil.
- 메서드는 `obj:method(...)` 로 호출되므로 첫 인자는 self 이고 무시된다. `isKeyPressed`, `getButton` 은 마지막 인자를 값으로 쓴다(`lastArgument`, `:379-381`).
- HTTP 는 GET 만 허용, 스킴은 http/https 만, 타임아웃은 `1..5000` ms 로 제한(기본 1000), 응답은 최대 1024 줄·65536 문자(`:46-49`, `:219-293`). `getInputStream()` 은 `{ readLine() }` 테이블을 준다.
- `Input.Keys.<NAME>` 은 `Input.Keys.valueOf(name)` 을 부른다(`:327-336`). libGDX 1.9.9 의 `valueOf` 는 필드명이 아니라 표시 이름(`"Right"`, `"Left"`, `"Up"`, `"Down"` 등)을 키로 쓰는 맵에서 찾고 없으면 `-1` 을 준다(jar 확인: `keyNames.get(keyname, -1)`). 그래서 `input.Keys.RIGHT` 는 이 HEAD 에서 `22` 가 아니라 `-1` 이 된다. `Gdx.input.isKeyPressed(-1)` 은 libGDX 에서 "아무 키나"를 뜻한다(백엔드 구현은 미확인). 즉 HEAD beatoraja 는 ModernChic 의 `Gdx.input:isKeyPressed(input.Keys.RIGHT)` (`MC/Result/lua/mainmenu.lua:986-989`, `MC/Result/lua/irmenu.lua:654-657`)를 원래 의도대로 처리하지 못한다. 원래 의도(진짜 luajava)는 정적 필드 `RIGHT=22`, `LEFT=21`, `UP=19`, `DOWN=20` 이다(이 숫자는 libGDX 상수에 대한 일반 지식이며 jar 로는 미확인).
- `debug` 는 `{ getmetatable = <base getmetatable> }` 뿐이다(`:60-63`).

### 2.4 require / dofile / loadfile 경로 규칙

- `setDirectory(p.getParent())` 가 표준 모드에서 `package.path` 뒤에 `;<스킨폴더 절대경로>/?.lua` 를 붙인다(`SkinLuaAccessor.java:869`). LuaJ 기본 `package.path` 는 `?.lua` 다(PackageLib.class 상수, 시스템 프로퍼티 `luaj.package.path` 로 바꿀 수 있음). 최종값은 `?.lua;<스킨폴더>/?.lua`.
- `require("A.b.c")`: 모듈 이름의 `.` 을 파일 구분자로 바꿔(`PackageLib$searchpath`, jar 확인) 템플릿의 `?` 에 넣는다. 탐색 순서는 (1) `package.preload`, (2) 작업 디렉터리 기준 `A/b/c.lua`, (3) `<스킨폴더>/A/b/c.lua`. 작업 디렉터리(beatoraja 실행 폴더)가 먼저다.
- 결과는 `package.loaded[name]` 에 캐시된다. 모듈 본문은 Lua 상태당 한 번만 실행된다.
- `require("main_state")`, `require("timer_util")`, `require("event_util")`, `require("luajava")`, `require("debug")` 는 파일을 찾지 않고 `package.loaded` 에 미리 넣어 둔 테이블을 돌려준다(`SkinLuaAccessor.java:95-102`, `:887`, `:909-912`, `LegacySkinLuaApi.java:58,63`).
- `dofile(path)` / `loadfile(path)`: `globals.finder.findResource(path)` 로 연다. 표준 모드의 finder 는 JseBaseLib 이고 `new File(path).exists()` 면 그 파일, 아니면 클래스패스 리소스를 찾는다(jar 확인). 즉 상대 경로는 작업 디렉터리 기준이다. 스킨 폴더 기준이 아니다. 실패 메시지는 `cannot open <path>: No such file or directory`. 청크 이름은 `@<path>`.
- 그래서 스킨은 `dofile(skin_config.get_path("Play/lua/background.lua"))` 처럼 `get_path` 로 스킨 폴더가 붙은 경로를 만들어 넘긴다(`MC/play7_hw.lua:46-49`). `dofile` 은 캐시하지 않으므로 부를 때마다 다시 실행된다. `dofile` 로 읽은 청크도 같은 전역을 공유한다.
- 진입 파일 실행은 `globals.loadfile(path.toString()).call()` 이다(`SkinLuaAccessor.java:852`).
- 소스 인코딩: LuaJ 문자열은 바이트열이고 Java 문자열로 바꿀 때 UTF-8 로 해석한다(표준 LuaJ 동작, 미확인). ModernChic 소스는 일본어 주석을 포함한 UTF-8 로 보인다.

### 2.5 스킨에 주입되는 모듈

| 모듈 | 헤더 실행 시 | 본 로드 시 |
| --- | --- | --- |
| `main_state` | 빈 테이블(자리 표시) | `MainStateAccessor.export` 가 채운 테이블 |
| `timer_util` | 빈 테이블 | `TimerUtility.export` |
| `event_util` | 빈 테이블 | `EventUtility.export` |
| `skin_config` 전역 | nil | 테이블(3.3) |

빈 테이블을 넣는 이유는 헤더를 읽을 때 `require("main_state")` 만으로 오류가 나지 않게 하려는 것이다(`SkinLuaAccessor.java:97` 주석). 헤더 실행 중에 `main_state.option(...)` 같은 호출을 하면 nil 호출 오류가 난다.

본 로드용 생성자 `JSONSkinLoader(state, c, lua)` 가 로드 전에 `exportMainStateAccessor(state)` 와 `exportUtilities(state)` 를 부른다(`B/skin/json/JSONSkinLoader.java:78-85`). JSON 스킨은 `SkinLuaAccessor(true)` 를 쓰며 이때는 같은 함수들이 모듈이 아니라 전역으로 직접 들어간다(`JSONSkinLoader.java:74-76`, `SkinLuaAccessor.java:880-889`).

---

## 3. .luaskin 을 두 번 실행하는 절차

### 3.1 헤더만 읽기 (`LuaSkinLoader.loadHeader`, `B/skin/lua/LuaSkinLoader.java:49-61`)

1. `lua.setDirectory(p.getParent())`.
2. `lua.execFile(p)` 로 진입 파일을 실행한다. 이때 `skin_config` 는 nil 이다.
3. 반환값을 `fromLuaValue(JsonSkin.Skin.class, value)` 로 변환한다.
4. `loadJsonSkinHeader(sk, p)` 로 `SkinHeader` 를 만든다.
5. 모든 `Throwable` 을 잡아 스택만 출력하고 null 을 돌려준다.

### 3.2 본 로드 (`LuaSkinLoader.load`, `:68-95`)

1. `loadHeader(p)` 를 먼저 부른다. 같은 Lua 상태에서 진입 파일이 한 번 실행된다(`skin_config == nil`, 단 `main_state` 등은 이미 실제 테이블).
2. 헤더가 null 이면 null 반환.
3. `header.setSkinConfigProperty(property)` 로 사용자 설정을 병합한다(10장).
4. `filemap` 구성: 각 `CustomFile` 중 `getSelectedFilename() != null` 인 것만 `customFile.path → 선택 파일명`.
5. `lua.exportSkinProperty(header, property, pathGetter)` 로 전역 `skin_config` 를 만든다.
6. `lua.execFile(p)` 로 진입 파일을 다시 실행한다. 같은 Lua 상태다.
7. 반환값을 `fromLuaValue(JsonSkin.Skin.class, value)` 로 변환하고 `loadJsonSkin(header, sk, type, property, p)` 를 부른다.
8. 예외는 전부 삼키고 null.

구현에 직접 영향을 주는 결과:

- 두 번째 실행에서 `require("play7_hw")` 는 캐시된 모듈을 돌려준다. 모듈 최상위 코드(예: `main_state = require("main_state")`, `PROPERTY = require(...).load(false)`, `local header = ...load(0)`, `MC/play7_hw.lua:8-13`)는 `skin_config == nil` 인 첫 실행에서 한 번만 돈다. 두 번째 실행은 `.luaskin` 의 6 줄만 다시 돌아 `t.main()` 을 부른다.
- 헤더 정보(옵션·파일·오프셋·카테고리·이름·type)는 첫 실행의 반환값에서 나오고, 해상도 `w`/`h`·`fadeout`/`input`/`scene`·소스·객체·`destination` 은 두 번째 실행의 반환값에서 나온다. `main()` 이 `w`/`h` 를 넣지 않으면 1280x720 으로 취급된다.
- 로더 인스턴스는 로드마다 새로 만들어지므로(`SkinLoader.java:68`) Lua 상태·`package.loaded`·전역은 스킨 로드 사이에 공유되지 않는다. 화면에 들어갈 때마다 스킨 Lua 전체가 다시 실행된다.
- 스킨이 만든 클로저는 로드가 끝난 뒤에도 그 Lua 상태를 붙잡고 매 프레임 호출된다.

공식 예시의 진입 파일도 같은 패턴이다(`/Users/hyunseokbyun/development/beatoraja/skin/default/play24.luaskin:1-6`, ModernChic 의 10 개 `.luaskin` 모두 동일).

### 3.3 skin_config 구조 (`SkinLuaAccessor.exportSkinProperty`, `:925-980`)

| 키 | 타입 | 내용 |
| --- | --- | --- |
| `skin_config.option` | 맵 `옵션이름 → 숫자` | 헤더의 모든 `CustomOption` 에 대해 `getSelectedOption()` (선택된 항목의 `op`). 항목이 없거나 선택 인덱스가 무효면 `-1` |
| `skin_config.enabled_options` | 배열(1 부터) | 위 선택값을 헤더 옵션 순서대로 나열 |
| `skin_config.file_path` | 맵 `파일항목이름 → 문자열` | 사용자 설정 `property.getFile()` 의 원본 값. 사용자가 고른 파일명이며 `"Random"` 이면 그 문자열 그대로. 사용자 설정에 없는 항목은 키 자체가 없다 |
| `skin_config.get_path(path)` | 함수 | `SkinLoader.getPath(<스킨폴더> + "/" + path, filemap).getPath()` 의 결과 문자열. 와일드카드가 없으면 `<스킨폴더>/<path>` 를 그대로 준다(존재 여부 무관). 규칙은 9.1 |
| `skin_config.offset` | 맵 `오프셋이름 → {x,y,w,h,r,a}` | 헤더의 모든 `CustomOffset`(플레이 스킨에 자동 추가되는 4 개 포함)에 대해 사용자 설정값, 없으면 0. 정수 |

- `get_path` 가 붙이는 스킨 폴더는 `p.getParent().toString()` 이다(`LuaSkinLoader.java:85-87`). 설정에 저장된 스킨 경로가 상대 경로(예: `skin/ModernChic/play7_hw.luaskin`)면 결과도 상대 경로다. 그래서 `dofile`·`io.open`·`audio_play` 가 작업 디렉터리 기준으로 맞아떨어진다.
- `skin_config.offset` 은 로드 시점의 스냅샷이다. 실행 중 바뀐 값은 `main_state.offset(id)` 로 읽는다.
- ModernChic 사용 빈도: `get_path` 238 회, `offset` 35 회, `option` 7 회. `file_path`·`enabled_options` 는 쓰지 않는다.

---

## 4. Lua 테이블 → JsonSkin 변환 규칙

구현: `LuaSkinLoader.fromLuaValue` (`B/skin/lua/LuaSkinLoader.java:168-208`), 스칼라·속성 변환표 `serializerMap` (`:97-151`).

### 4.1 일반 규칙

| 대상 타입 | 규칙 |
| --- | --- |
| 객체(클래스) | 기본 생성자로 인스턴스를 만든다(필드 기본값은 `JsonSkin.java` 의 초기값). 값이 테이블이면 `table.keys()` 를 돌며 키를 문자열로 바꿔(`tojstring`) 공개 필드 이름과 정확히(대소문자 구분) 일치하는 것만 대입한다. 일치하지 않는 키는 조용히 무시한다. 값이 테이블이 아니면 기본값 인스턴스를 그대로 돌려준다(null 이 아니다) |
| 배열 | 값이 테이블이면 `table.keys()` 순서대로 모든 키의 값을 원소로 변환한다. 정수 키만 고르지 않는다. nil 값인 칸은 keys 에 없으므로 구멍은 메워지고, 문자열 키가 섞여 있으면 그 값도 원소가 된다. 테이블이 아니면 길이 0 배열 |
| `int`/`Integer` | `LuaValue.toint()` |
| `float`/`Float` | `LuaValue.tofloat()` |
| `boolean`/`Boolean` | `LuaValue.toboolean()` |
| `String` | `LuaValue.tojstring()` |
| 속성·이벤트 타입 | 4.3 |

- Lua 테이블에 키가 없으면 그 필드는 기본값으로 남는다. `keys()` 는 nil 값을 내지 않으므로 필드에 nil 이 대입되는 일은 없다.
- JSON 로더와 달리 `include`, `{if, value}`, `{if, values}` 분기는 Lua 경로에 없다. Lua 스킨은 분기를 Lua 코드로 직접 한다.
- 최상위 반환값이 테이블이 아니면 기본값 `Skin`(`type = -1`)이 되어 헤더가 null → 로드 실패.

### 4.2 LuaJ 스칼라 강제 변환의 실제 동작 (jar 확인)

| Lua 값 | `toint` | `tofloat` | `toboolean` | `tojstring` |
| --- | --- | --- | --- | --- |
| nil | 0 | 0 | false | `"nil"` |
| true / false | 0 | 0 | true / false | `"true"` / `"false"` |
| 정수 n | n | n | true | 10 진 문자열 |
| 실수 d | `(int)(long)d` (0 쪽으로 버림, NaN 은 0) | `(float)d` | true (0 도 true) | 정수값이면 `Long.toString`(예 `5`), 아니면 `Float.toString((float)d)`, `nan`/`inf`/`-inf` |
| 숫자 모양 문자열 | 파싱한 수 | 파싱한 수 | true | 그대로 |
| 그 외 문자열 | 0 | 0 | true | 그대로 |
| 테이블·함수 | 0 | 0 | true | `table: ...` 꼴 |

- `LuaValue` 기본 구현은 `toint`=0, `tolong`=0, `tofloat`=0, `toboolean`=true 이고 `LuaBoolean` 은 `toboolean` 만 재정의한다.
- `LuaDouble.valueOf(d)` 는 d 가 int 범위의 정수값이면 `LuaInteger` 로 정규화한다. 그래서 `10 / 2` 는 정수 5 가 되고 문자열로는 `"5"` 다.
- `isnumber()` 는 숫자와 "숫자로 읽히는 문자열" 모두 true, `isstring()` 은 문자열과 숫자 모두 true 다. 4.3 의 분기 순서에 영향을 준다.
- 불리언 필드(`Offset.x/y/w/h/r/a`, `Text.editable`, `Slider.changeable` 등)에 숫자 `0` 을 넣으면 Lua 경로에서는 true 가 된다.
- 정수 필드에 실수를 넣으면 버림된다. ModernChic 은 `x = RESULT_BASE.CENTER_POS_X + 520 / 2` 처럼 나눗셈 결과를 좌표에 넣는다(`MC/Result/lua/centerinfo.lua:73`).
- 문자열 필드(`id`, `src`, `font`, `images[]`, `note[]` 등)에 숫자를 넣는 것이 흔하다. ModernChic 은 `{id = 1, path = ...}`, `src = 1` 을 쓴다(`MC/decide.lua:15,235`). 이때 `"1"` 로 변환된다.

### 4.3 숫자·문자열·함수가 섞일 수 있는 필드

공통 변환 함수 `serializeLuaScript(lv, asFunction, asScript, byId, byName)` (`LuaSkinLoader.java:153-166`)의 분기 순서:

1. `lv.isfunction()` → Lua 함수를 그대로 감싼 속성.
2. `lv.isnumber() && byId != null` → `byId(lv.toint())` (내장 id 조회). 숫자 모양 문자열도 여기로 온다.
3. `lv.isstring()` → 이름으로 `byName` 조회, 없으면 `asScript(문자열)` (Lua 코드로 컴파일).
4. 그 외(nil, boolean, 테이블) → null.

| 필드 타입 | 등장 위치(JsonSkin) | 숫자 | 이름 문자열 | 스크립트 문자열 | 함수 |
| --- | --- | --- | --- | --- | --- |
| `TimerProperty` | `Image/Value/FloatValue/Slider/Graph/HiddenCover/LiftCover.timer`, `Destination.timer`, `CustomTimer.timer` | `TimerPropertyFactory.getTimerProperty(id)`, 음수면 null | 없음(byName null) | `return <s>` 를 컴파일해 한 번 시험 호출. 결과가 함수면 그 함수가 타이머 함수, 아니면 청크 자체가 타이머 함수 | 타이머 함수 |
| `BooleanProperty` | `Destination.draw`, `CustomEvent.condition` | `BooleanPropertyFactory.getBooleanProperty(id)` (음수는 부정) | 같은 팩토리의 이름 조회, 앞에 `!` 를 붙이면 부정 | `return <s>` | 호출 결과의 `toboolean` |
| `IntegerProperty` | `ImageSet.value`, `Value.value` | `IntegerPropertyFactory.getIntegerProperty(id)` | 이름 조회 | `return <s>` | `toint` |
| `FloatProperty` | `FloatValue.value`, `Slider.value`, `Graph.value` | `FloatPropertyFactory.getRateProperty(id)` | 이름 조회 | `return <s>` | `tofloat` |
| `StringProperty` | `Text.value` | `StringPropertyFactory.getStringProperty(id)` | 이름 조회 | `return <s>` | `tojstring` |
| `Event` | `Image.act`, `ImageSet.act`, `CustomEvent.action` | `EventFactory.getEvent(id)` | 이름 조회 | `<s>` 를 그대로 컴파일(return 을 붙이지 않음) | 5.2 |
| `FloatWriter` | `Slider.event` | `FloatPropertyFactory.getRateWriter(id)` | 이름 조회 | `<s>` 그대로 | 값 1 개를 인자로 호출 |
| `StringWriter` | `Text.event` | byId 가 null 이라 숫자도 3 번으로 가서 이름 `"123"` 조회 → 없으면 스크립트 `123` 컴파일 실패 → null | `StringPropertyFactory.getStringWriter(name)` | `<s>` 그대로 | 문자열 1 개를 인자로 호출 |
| `DestinationOption` | `Destination.op[]` | `isnumber()` 면 숫자 옵션 id(`:142-145`). 음수는 부정 | 이름 조회 후 실패 시 스크립트 | `return <s>` | 불리언 속성 |
| `FontFallback` | `Font.fallback[]` | — | 문자열이면 `path`, `type=0` | — | 테이블이면 `path`(문자열일 때만), `type`(숫자일 때만, 아니면 0) (`:106-117`) |

- 정수 필드 `ref` 와 속성 필드 `value` 가 함께 있는 객체(`Value`, `FloatValue`, `ImageSet`, `Text`, `Slider`, `Graph`)는 `value != null` 이면 `value` 가 우선이고 아니면 `ref` 를 쓴다(`B/skin/json/JsonSkinObjectLoader.java:91,119-124,199-202,399-411,441-449,634,664`).
- `Destination.op` 원소 중 숫자는 `getOptionIds()` 로(0 은 버림), 속성은 `getDrawConditions()` 로 나뉘고(`JsonSkin.java:468-480`), `draw` 가 있으면 조건 목록 끝에 추가된다(`JSONSkinLoader.java:454-458`). 모든 조건이 참이어야 그려진다.
- ModernChic 에서 `draw/timer/value/act/event = "..."` 꼴의 문자열 리터럴 대입은 grep 으로 0 건이다. 변수에 담은 문자열을 넘기는 경우까지는 확인하지 않았다.

### 4.4 JsonSkin.Skin 최상위 필드와 기본값 (`B/skin/json/JsonSkin.java:7-57`)

| 필드 | 타입 | 기본값 |
| --- | --- | --- |
| `type` | int | -1 |
| `name`, `author` | String | null (헤더에서는 `""` 로 치환) |
| `w`, `h` | int | 1280, 720 |
| `fadeout`, `input`, `scene`, `close`, `loadend`, `playstart` | int | 0 |
| `judgetimer` | int | 1 |
| `finishmargin` | int | 0 |
| `category`, `property`, `filepath`, `offset`, `source`, `font`, `image`, `imageset`, `value`, `floatvalue`, `text`, `slider`, `graph`, `gaugegraph`, `judgegraph`, `bpmgraph`, `hiterrorvisualizer`, `timingvisualizer`, `timingdistributiongraph`, `hiddenCover`, `liftCover`, `judge`, `pmchara`, `customEvents`, `customTimers` | 배열 | 길이 0 |
| `note`, `gauge`, `bga`, `skinpreview`, `practice`, `songlist`, `skinSelect` | 객체 | null |
| `destination` | `Destination[]` | null |

- `destination` 이 없으면 `loadJsonSkin` 의 반복문(`JSONSkinLoader.java:313`)에서 NullPointerException 이 나고 스킨 로드가 null 이 된다.
- `scene` 을 지정하지 않으면 `skin.setScene(0)` 이 된다(`JSONSkinLoader.java:307`). `Skin` 내부 기본값 `3600000 * 24` (`B/skin/Skin.java:85`)는 덮어써진다.
- `skinSelect != null` 이고 `customPropertyCount <= 0` 이면 모든 `image[].act.getEventId()` 를 조사한다(`JSONSkinLoader.java:338-363`). `act` 가 없는 이미지가 하나라도 있으면 NullPointerException 으로 로드가 실패한다. 스킨 선택 스킨에서만 해당한다.

### 4.5 헤더 관련 객체

| 클래스 | 필드(기본값) | 근거 |
| --- | --- | --- |
| `Property` | `category`(String), `name`, `item`(`PropertyItem[]`, 빈 배열), `def`(String) | `JsonSkin.java:59-64` |
| `PropertyItem` | `name`, `op`(int) | `:66-69` |
| `Filepath` | `category`, `name`, `path`, `def` | `:71-76` |
| `Offset` | `category`, `name`, `id`(int), `x`,`y`,`w`,`h`,`r`,`a`(boolean, 사용자가 바꿀 수 있는 성분) | `:78-88` |
| `Category` | `name`, `item`(`String[]`) | `:90-93` |

### 4.6 Destination / Animation

| 클래스 | 필드(기본값) |
| --- | --- |
| `Destination` (`JsonSkin.java:453-481`) | `id`(String), `blend`(0), `filter`(0), `timer`(null), `loop`(0), `center`(0), `offset`(0), `offsets`(`int[]` 빈 배열), `stretch`(-1), `op`(빈 배열), `draw`(null), `dst`(`Animation[]` 빈 배열), `mouseRect`(`Rect{x,y,w,h}`, null) |
| `Animation` (`:507-529`) | `time`, `x`, `y`, `w`, `h`, `clip_x`, `clip_y`, `clip_w`, `clip_h`, `acc`, `a`, `r`, `g`, `b`, `angle` 전부 `Integer.MIN_VALUE`(미지정 표식) |

`setDestination` 의 채움 규칙(`JSONSkinLoader.java:422-468`):

- 첫 프레임에서 미지정이면 `time=0, x=0, y=0, w=0, h=0, acc=0, angle=0, a=255, r=255, g=255, b=255`. `clip_*` 는 미지정으로 남는다.
- 이후 프레임에서 미지정이면 직전 프레임 값을 물려받는다(`clip_*` 포함).
- `clip_x/y/w/h` 넷이 모두 지정된 프레임에서만 클립을 건다.
- `dst.id` 가 정수로 읽히고 음수면 `new SkinImage(-id)` (내장 이미지 참조)로 처리하고, 아니면 객체 로더로 id 를 찾는다(`:313-331`). 못 찾으면 그 destination 은 건너뛴다.
- 오프셋은 `offsets[]` 뒤에 `offset` 을 덧붙인 배열이다(`:470-475`). `stretch >= 0` 일 때만 적용.
- 그리기 조건·`blend`·`filter`·`center`·`timer`·`loop` 는 객체의 첫 프레임에서만 사실상 확정된다(`B/skin/SkinObject.java:178-181,218-239`).

`Animation` 의 정수 필드에 실수가 오면 버림이다. `Integer.MIN_VALUE` 와 구별해야 하므로 R-BMS 는 "키 없음"을 별도 상태(Option)로 표현하는 편이 안전하다.

---

## 5. Lua 로 정의된 속성의 호출 규약과 오류 처리

### 5.1 감싸는 방식 (`B/skin/lua/SkinLuaAccessor.java:601-834`)

| 속성 | 호출 | 반환 변환 | 예외 시 |
| --- | --- | --- | --- |
| Boolean (`:611-628`) | `function.call()` 인자 없음 | `toboolean` | 경고 로그 후 false. `isStatic` 은 항상 false |
| Integer (`:640-652`) | 인자 없음 | `toint` | 0 |
| Float (`:664-676`) | 인자 없음 | `tofloat` | 0 |
| String (`:688-700`) | 인자 없음 | `tojstring` | `""` |
| Timer (`:735-747`) | 인자 없음 | `tolong` | `Long.MIN_VALUE`(꺼짐) |
| Event (`:759-788`) | 5.2 | 무시 | 무시 |
| FloatWriter (`:800-811`) | `function.call(LuaDouble.valueOf(value))` | 무시 | 무시 |
| StringWriter (`:823-834`) | `function.call(LuaString.valueOf(value))` | 무시 | 무시 |

- Lua 의 `pcall` 을 쓰지 않는다. Java 쪽에서 `RuntimeException` 을 잡는다(LuaJ 의 `LuaError` 는 RuntimeException). 잡으면 `Logger.getGlobal().warning("Lua実行時の例外 : " + 메시지)` 를 남기고 기본값을 돌려준다.
- 실패한 속성은 비활성화되지 않는다. 다음 프레임에도 그대로 다시 호출되고 실패할 때마다 로그가 남는다. 객체가 영구히 숨겨지지도 않는다. 그 프레임에만 조건이 false(혹은 타이머 꺼짐)로 처리되어 안 그려질 뿐이다.
- 값을 돌려주지 않는 함수는 nil 로 취급된다. `draw` 함수가 nil 을 돌려주면 false 다. ModernChic 은 이것을 "매 프레임 실행되는 훅"으로 쓴다. 크기 0 인 destination 에 값을 돌려주지 않는 `draw` 함수를 달아 부수효과만 실행한다(`MC/Result/lua/mainmenu.lua:984-994`).
- 타이머 함수가 nil 이나 boolean 을 돌려주면 `tolong` 이 0 이라 "시각 0 에 켜진 타이머"가 된다. 꺼짐을 뜻하려면 `main_state.timer_off_value` 를 돌려줘야 한다.
- 문자열 스크립트는 로드 때 한 번 컴파일된다. 컴파일 실패 시 `Lua解析時の例外` 경고 후 속성이 null 이 된다(필드가 없는 것과 같다).
- 미리보기 화면만 `drawAllObjectsSafely` 를 써서 prepare/draw 중 예외가 난 객체의 `draw` 를 그 프레임에 false 로 둔다(`B/skin/Skin.java:333-359`, 호출 `B/config/SkinPreview.java:89-90`).

### 5.2 Event 함수의 인자 개수

`loadEvent(LuaFunction)` 은 `function.narg()` 로 0/1/2 인자를 가른다(`SkinLuaAccessor.java:759-788`). 그런데 `narg()` 는 `LuaValue` 에 정의된 "가변 인자 묶음의 개수"이고 값 하나에 대해 항상 1 을 돌려준다(jar 확인: `LuaValue.narg` 의 바이트코드가 `iconst_1; ireturn`, `LuaFunction`·`LuaClosure` 는 재정의하지 않음). 따라서 실제로는 언제나 `case 1` 이다.

결론: Lua 로 쓴 이벤트 함수는 항상 인자 1 개(`arg1`, 정수)로 호출된다. 조건으로 자동 실행될 때는 0 이 넘어온다. 두 번째 인자는 전달되지 않는다. `event_util` 이 만든 자바 함수도 같은 경로를 탄다.

### 5.3 호출 시점

| 대상 | 시점 | 근거 |
| --- | --- | --- |
| 능동 커스텀 타이머 함수 | 프레임당 1 회, `updateCustomObjects` 의 앞 절반 | `B/skin/Skin.java:782-785`, `B/skin/CustomTimer.java:43-47` |
| 커스텀 이벤트의 `condition` 과 `action` | 프레임당 1 회, 타이머 다음 | `Skin.java:786-788`, `B/skin/CustomEvent.java:30-38` |
| `Destination.op`/`draw` 의 함수 조건 | 객체 `prepare` 때 선언 순서대로, 첫 false 에서 중단 | `B/skin/SkinObject.java:591-598` |
| `Destination.timer` 함수 | 조건 통과 뒤 `prepareRegion` 에서 `isOff` 로 1 회, `get` 으로 1 회(프레임당 객체당 2 회) | `SkinObject.java:350-358`, `B/skin/property/TimerProperty.java:8-23` |
| `value`(숫자·문자열·실수) 함수 | 각 객체의 `prepare` | 객체별 구현, 이 조사 범위 밖 |
| `act` (클릭) | 렌더 뒤 입력 처리에서 `skin.mousePressed` | `B/MainController.java:496-503`, `Skin.java:394-411` |
| 슬라이더·텍스트 `event` | 마우스 조작·텍스트 확정 시 | 객체별 구현, 범위 밖 |

- `prepare` 는 `nextpreparetime <= 현재시각` 일 때만 돈다. 간격은 `prepareFramePerSecond > 0` 이면 `1000000 / 값` 마이크로초, 0(기본)이면 1 마이크로초라 사실상 매 프레임이다(`Skin.java:265-266,316-323`, `B/Config.java:75`).
- `prepare` 순서는 `destination` 선언 순서(= 그리기 순서)다. 클릭 판정은 역순(위에 그려진 것부터)이다(`Skin.java:398`).
- 스킨 적용 직후 한 번 도는 `Skin.prepare` (`Skin.java:199-267`)에서 `isStatic` 인 조건은 한 번만 평가해 거짓이면 객체를 제거한다. Lua 함수 조건은 항상 동적이라 매 프레임 평가된다. 숫자 `op` 중 내장 조건으로 풀리지 않는 것(스킨 커스텀 옵션 id)은 스킨의 옵션 맵과 비교한다. 양수 id 는 값이 1 이어야, 음수 id 는 값이 0 이어야 남는다. 맵에 없는 id 는 어느 부호든 제거된다(`Skin.java:216-229`).
- 한 프레임 안의 "현재 시각"은 `timer.update()` 에서 한 번 정해진다(`B/TimerManager.java:109-111`). 프레임 중 `main_state.time()` 은 같은 값을 준다.

---

## 6. main_state 모듈 전체 API

조립: `MainStateAccessor` 가 네 개의 내보내기를 순서대로 같은 테이블에 넣는다(`B/skin/lua/MainStateAccessor.java:21-35`). 인자의 숫자 변환은 모두 `toint`/`tolong`/`tofloat` 이다(4.2 표).

### 6.1 상태 읽기 (`B/skin/lua/MainStatePropertyLuaApiExporter.java`)

| 함수 | 인자 | 반환 | 의미 | 근거 |
| --- | --- | --- | --- | --- |
| `option(id\|name)` | 숫자 id 또는 이름 문자열 | boolean | 내장 불리언 속성. 없으면 false | `:159-165` |
| `number(id\|name)` | 같음 | 정수 | 내장 정수 속성. 없으면 0 | `:167-172,154-157` |
| `numbers(a, b, ...)` | 가변 | 정수 여러 개 | 인자마다 `number` 를 적용한 다중 반환 | `:305-314` |
| `float_number(id\|name)` | 같음 | 실수 | `getFloatProperty`, 없으면 Rate 속성으로 대체, 그래도 없으면 0 | `:174-180` |
| `text(id\|name)` | 같음 | 문자열 | 내장 문자열 속성. null 이면 `""` | `:182-189` |
| `offset(id)` | 숫자 | `{x,y,w,h,r,a}` 새 테이블(실수) | 현재 오프셋 값. id 는 0..199 범위여야 한다(범위 밖은 예외) | `:191-204`, `B/MainController.java:124-125,208-210` |
| `timer(id)` | 숫자 | 숫자(마이크로초) | 타이머가 켜진 시각. 꺼져 있으면 `timer_off_value` | `:206-211` |
| `timer_off_value` | 상수 | 숫자 | `Long.MIN_VALUE` 를 double 로 바꾼 값(-2^63) | `:20,39` |
| `timer_is_on(id)` / `timer_is_off(id)` | 숫자 | boolean | | `:213-225` |
| `timer_elapsed(id)` | 숫자 | 숫자 | 경과 마이크로초, 꺼져 있으면 -1 | `:42,227-239` |
| `timer_elapsed_ms(id)` | 숫자 | 숫자 | 경과 밀리초(정수 나눗셈), 꺼져 있으면 -1 | `:43` |
| `timer_elapsed_seconds(id)` | 숫자 | 실수 | 경과 초, 꺼져 있으면 -1 | `:241-248` |
| `time()` | 없음 | 숫자 | 현재 시각(상태 진입 후 마이크로초) | `:250-255` |
| `event_index(id)` | 숫자 | 숫자 | `IntegerPropertyFactory.getImageIndexProperty(id)` 의 현재 값(버튼·옵션의 현재 선택 인덱스). 속성이 없으면 예외 | `:297-303` |
| `key_pressed(code\|name)` | 숫자 또는 libGDX 키 표시 이름 | boolean | 키가 눌려 있는지. 음수·모르는 이름은 false | `:316-330` |
| `screen_width()` / `screen_height()` | 없음 | 정수 | 창 픽셀 크기 | `:51-52` |
| `rate()` | 없음 | 실수 | 현재 스코어 레이트 | `:54` |
| `exscore()` | 없음 | 숫자 | 현재 EX 스코어 | `:55` |
| `rate_best()` / `exscore_best()` | 없음 | 숫자 | 자기 베스트의 레이트 / 스코어 | `:56-57` |
| `rate_rival()` / `exscore_rival()` | 없음 | 숫자 | 라이벌 레이트 / 스코어 | `:58-59` |
| `volume_sys()` / `volume_key()` / `volume_bg()` | 없음 | 실수 | 시스템·키음·BGM 볼륨 | `:60-62` |
| `gauge()` | 없음 | 숫자 | 플레이 중 게이지 값, 플레이 화면이 아니면 0 | `:63,105-110` |
| `gauge_type()` | 없음 | 숫자 | 플레이 중 게이지 종류, 아니면 0 | `:64,112-117` |
| `judge(i)` | 판정 번호 | 정수 | 해당 판정의 빠름+느림 합계. 0 PG, 1 GR, 2 GD, 3 BD, 4 PR ... | `:87-93`, `B/ScoreData.java:255-261` |

이름 문자열 조회 결과는 이름별로 캐시된다(`:119-132`). 숫자 id 조회는 팩토리 내부 캐시를 쓴다.

타이머 id 범위: `0..2999` 는 내장 타이머 배열, 그 밖의 모든 id 는 현재 스킨의 커스텀 타이머에서 찾는다(`B/TimerManager.java:59-65`). 없는 커스텀 id 는 꺼짐으로 읽힌다(`B/skin/Skin.java:752-758`).

### 6.2 부수효과가 있는 함수

| 함수 | 인자 | 반환 | 부수효과 | 근거 |
| --- | --- | --- | --- | --- |
| `set_timer(id, value)` | 숫자, 마이크로초 값 | true | 타이머 값 기록. id 가 `10000..19999` (커스텀 타이머)가 아니면 예외. 능동(함수형) 커스텀 타이머에 쓰면 무시. 없는 id 면 수동 타이머를 새로 만든다 | `MainStatePropertyLuaApiExporter.java:257-267`, `B/skin/SkinPropertyMapper.java:159-166`, `B/skin/Skin.java:766-774`, `B/skin/CustomTimer.java:37-41` |
| `event_exec(id [, arg1 [, arg2]])` | 숫자들 | true | `state.executeEvent(id, arg1, arg2)`. 현재는 모든 id 허용. `1000..1999` 는 스킨 커스텀 이벤트, 그 밖은 각 화면이 처리하는 내장 이벤트 | `:269-295`, `SkinPropertyMapper.java:148-157`, `B/MainState.java:90-94` |
| `set_volume_sys(v)` / `set_volume_key(v)` / `set_volume_bg(v)` | 실수 | true | 오디오 설정값을 직접 바꾼다. 범위 검사 없음 | `:66-86` |
| `audio_play(path [, volume])` | 경로, 배율 | true | 효과음 1 회 재생. 배율은 nil 이면 1, `0..2` 로 제한, 시스템 볼륨을 곱한다 | `B/skin/lua/SkinAudioLuaApiExporter.java:20-26,57-60` |
| `audio_loop(path [, volume])` | 같음 | true | 반복 재생 | `:27-33` |
| `audio_preload(path)` | 경로 | true | 볼륨 0 으로 재생해 미리 읽기 | `:34-40` |
| `audio_stop(path)` | 경로 | true | 정지 | `:41-47` |
| `audio_dispose(path)` | 경로 | true | 해제 | `:48-54` |
| `file_exists(path)` | 경로 | boolean | 읽기만 | `B/skin/lua/SkinFileLuaApiExporter.java:23-28` |
| `file_mkdir(path)` | 경로 | boolean | 폴더 생성(중간 폴더 포함) | `:29-39` |
| `file_list(dir [, pattern])` | 경로, Lua 패턴 | 문자열, 개수 | 폴더 항목을 줄바꿈으로 이은 문자열과 개수. 패턴이 있으면 `%` 를 `\` 로 바꾼 Java 정규식으로 `find` 하고 일치한 부분만 넣는다. 실패 시 `"", 0` | `:40-45,78-104,144-162` |
| `file_read_lines(path)` | 경로 | 배열 | UTF-8 줄 배열. 실패 시 빈 테이블 | `:46-51,106-116` |
| `file_write(path, text)` | 경로, 문자열 | boolean | 덮어쓰기(부모 폴더 자동 생성) | `:52-57,118-134` |
| `file_append(path, text)` | 같음 | boolean | 덧붙이기 | `:58-63` |
| `file_clear(path)` | 경로 | boolean | 내용 비우기 | `:64-69` |
| `file_count_lines(path)` | 경로 | 정수 | 줄 수, 실패 시 0 | `:70-75,136-142` |
| `http_get_lines(url [, timeout_ms])` | URL, 타임아웃 | 성공 `배열, true` / 실패 `nil, 메시지` | HTTP GET. http/https 만, 타임아웃 `1..5000`(기본 1000), 최대 1024 줄·65536 문자 | `B/skin/lua/SkinHttpLuaApiExporter.java:13-32,53-84` |
| `http_get(url [, timeout_ms])` | 같음 | 성공 `문자열, true` / 실패 `nil, 메시지` | 줄을 `\n` 으로 이은 본문 | `:33-50` |

- 파일·오디오 경로는 `SkinLuaPathResolver.resolve` 를 거친다(`B/skin/lua/SkinLuaPathResolver.java:16-35`). 규칙은 2.2 의 io 와 같고 스킨 폴더 밖이면 Lua 오류 `skin file access denied`.
- ModernChic 사용 빈도(전체 Lua 대상 grep): `option` 230, `number` 207, `audio_play` 82, `text` 62, `time` 22, `timer` 16, `timer_off_value` 15, `event_index` 14, `gauge_type` 11, `judge` 6, `volume_sys/key/bg` 각 2, `set_volume_sys/key/bg` 각 2, `rate` 2, `float_number` 2, `gauge` 1, `exscore` 1. `file_*`, `http_*`, `set_timer`, `event_exec`, `offset`, `numbers`, `key_pressed`, `timer_elapsed*`, `screen_*` 는 쓰지 않는다. 파일과 HTTP 는 `io`·`luajava` 로 직접 한다(13장).

---

## 7. timer_util / event_util

### 7.1 timer_util (`B/skin/lua/TimerUtility.java:18-25`)

| 함수 | 인자 | 반환 | 의미 |
| --- | --- | --- | --- |
| `now_timer(value)` | 타이머 값(id 아님) | 숫자 | 켜져 있으면 `현재시각 - value`(마이크로초), 꺼져 있으면 0 (`:32-38`) |
| `is_timer_on(value)` / `is_timer_off(value)` | 타이머 값 | boolean | `value != / == Long.MIN_VALUE` (`:44-60`) |
| `timer_function(id)` | 타이머 id | 함수 | 호출하면 그 id 의 타이머 값을 돌려주는 함수 (`:71-82`) |
| `timer_observe_boolean(func)` | `() → boolean` | 타이머 함수 | func 가 참이 된 순간의 시각을 기억해 돌려주고, 거짓이 되면 꺼짐 값으로 되돌린다. 상태는 반환된 함수마다 따로 가진다 (`:89-111`) |
| `new_passive_timer()` | 없음 | `{ timer, turn_on, turn_on_reset, turn_off }` | `timer()` 는 현재 값, `turn_on()` 은 꺼져 있을 때만 현재 시각으로 켬, `turn_on_reset()` 은 항상 현재 시각으로 다시 켬, `turn_off()` 는 끔 (`:123-163`) |

`timer_observe_boolean` 의 정확한 동작:

```lua
local value = OFF
return function()
  local on = func()           -- toboolean
  if on and value == OFF then value = main_state.time()
  elseif (not on) and value ~= OFF then value = OFF end
  return value
end
```

- func 가 Lua 오류를 내면 이 함수 전체가 오류가 되고 5.1 규칙으로 꺼짐 처리된다.
- destination 의 `timer` 로 쓰면 프레임당 2 회 호출되지만 같은 프레임에서는 시각이 같아 결과가 같다.
- ModernChic 은 `timer_observe_boolean` 만 148 회 쓴다. `timer_util` 의 나머지와 `event_util` 은 쓰지 않는다.

### 7.2 event_util (`B/skin/lua/EventUtility.java:17-23`)

| 함수 | 인자 | 반환 | 의미 |
| --- | --- | --- | --- |
| `event_observe_turn_true(func, action)` | `() → boolean`, 함수 | 이벤트 함수 | 호출될 때마다 func 를 평가하고, 거짓에서 참으로 바뀐 순간에만 `action()` 실행 (`:42-66`) |
| `event_observe_timer(timerFunc, action)` | 타이머 함수, 함수 | 이벤트 함수 | 타이머 값이 직전과 다르고 꺼짐이 아니면 `action()` (켜지거나 다시 켜질 때마다) (`:74-96`) |
| `event_observe_timer_on(timerFunc, action)` | 같음 | 이벤트 함수 | 꺼짐에서 켜짐으로 바뀐 순간 `action()` (`:110-134`) |
| `event_observe_timer_off(timerFunc, action)` | 같음 | 이벤트 함수 | 켜짐에서 꺼짐으로 바뀐 순간 `action()`. 내부 초기 상태가 `isOff = false` 라서 처음부터 꺼져 있는 타이머는 첫 호출에서 한 번 발동한다 (`:148-172`) |
| `event_min_interval(ms, action)` | 밀리초, 함수 | 이벤트 함수 | 마지막 실행 뒤 ms 이상 지났을 때만 `action()` (`:184-206`) |

반환된 함수는 `customEvents[].action` 에 넣는 것을 전제로 한다. 매 프레임 관찰하려면 `condition` 을 항상 참으로 두어야 한다.

---

## 8. customTimers / customEvents

### 8.1 선언 (`JsonSkin.java:546-556`, 등록 `JSONSkinLoader.java:365-375`)

| 객체 | 필드 | 의미 |
| --- | --- | --- |
| `customTimers[]` | `id`(int), `timer`(TimerProperty, 생략 가능) | `timer` 가 있으면 능동 타이머, 없으면 수동 타이머 |
| `customEvents[]` | `id`(int), `action`(Event), `condition`(BooleanProperty, 생략 가능), `minInterval`(int, ms, 기본 0) | `condition` 이 있으면 자동 실행 대상 |

- id 범위 관례: 타이머 `10000..19999`, 이벤트 `1000..1999` (`B/skin/SkinProperty.java:175-176,1051-1052`). 로더는 범위를 검사하지 않는다. 다만 타이머 id 가 `0..2999` 면 내장 배열에 가려 읽히지 않고, 이벤트 id 가 `1000..1999` 밖이면 `executeEvent` 가 커스텀 이벤트로 보내지 않는다(`B/MainState.java:90-94`).
- 같은 id 를 두 번 선언하면 나중 것이 덮어쓴다(`Skin.java:731-743`, IntMap put).

### 8.2 프레임당 평가 순서

`MainController.render` (`B/MainController.java:401-413`):

1. `timer.update()` 로 현재 시각 확정.
2. `current.render()` (화면 로직. 여기서 내장 타이머가 켜지고 꺼진다).
3. `skin.updateCustomObjects(current)` (`Skin.java:782-789`)
   1. 모든 커스텀 타이머 `update`: 능동이면 `time = timerFunc.getMicro(state)`. 수동이면 아무것도 안 함.
   2. 모든 커스텀 이벤트 `update`: `condition == null` 이면 건너뜀. `condition.get(state)` 가 참이고 (한 번도 실행 안 했거나 `(현재 - 마지막 실행) / 1000 >= minInterval`)이면 `action.exec(state)` (인자 0, 0) 후 마지막 실행 시각 갱신.
4. `skin.drawAllObjects`: 객체별 `prepare`(조건·타이머·값 함수 호출) 후 그리기.
5. 렌더가 끝난 뒤 입력 처리에서 클릭 이벤트.

- 주석은 "ID 가 작은 순"이라 하지만(`Skin.java:777-780`) 실제 순회는 libGDX `IntMap` 의 내부 순서다. 연속된 id 에서는 대체로 오름차순이지만 보장되지 않는다(IntMap 내부 구현은 미확인). R-BMS 는 문서화된 의도대로 id 오름차순으로 돌리면 된다.
- id 로 참조하는 커스텀 타이머는 프레임당 한 번 계산된 값을 여러 객체가 공유한다. 함수를 destination 에 직접 넣으면 객체마다 따로 호출된다.
- `event_exec(id, ...)` 나 클릭으로 커스텀 이벤트를 직접 실행하면 `condition`·`minInterval` 과 무관하게 `action` 이 돌고 마지막 실행 시각이 갱신된다(`CustomEvent.java:25-28`).
- `action` 이 없는(null) 이벤트가 실행되면 NullPointerException 이 난다.

ModernChic 은 `skin.customTimers = {}` / `skin.customEvents = {}` 를 선언만 하고(`MC/musicselect.lua:55`, `MC/result.lua:38-39`, `MC/course.lua:38-39`) 내용이 채워지는지는 이 조사에서 확인하지 않았다(미확인, ModernChic 담당 조사 범위).

---

## 9. JSON 스킨과 Lua 스킨이 공유하는 경로

| 기능 | JSON | Lua | 근거 |
| --- | --- | --- | --- |
| 데이터 모델 `JsonSkin.*` | 사용 | 사용 | `LuaSkinLoader extends JSONSkinLoader` (`LuaSkinLoader.java:30`) |
| 헤더 생성 `loadJsonSkinHeader` | 사용 | 사용 | `JSONSkinLoader.java:115-210` |
| 본 로드 `loadJsonSkin`, 객체 로더, `setDestination` | 사용 | 사용 | `JSONSkinLoader.java:260-391` |
| 소스 경로 해석 `getPath` + `filemap` | 사용 | 사용 | `JSONSkinLoader.java:494` |
| `include` | 있음 | 없음 | `JsonSkinSerializer.java:233-244,300-311,326-338` |
| 조건 분기 `[{if, value}]`, 배열 안 `{if, value}`/`{if, values}` | 있음 | 없음 | `JsonSkinSerializer.java:161-212,220-232,285-299` |
| 문자열 스크립트 필드 | 있음 | 있음(함수도 가능) | `JsonSkinSerializer.java:341-370`, `LuaSkinLoader.java:153-166` |
| `skin_config` 전역 | 설정됨 | 설정됨 | `JSONSkinLoader.java:237-239` |
| `main_state` 함수 위치 | 전역 함수 | `require("main_state")` | `JSONSkinLoader.java:74-76` |

JSON 전용 조건식 규칙(참고): `901` 은 "901 선택됨", `-901` 은 "901 선택 안 됨", `[901, 911]` 은 AND, `[[901, 902], 911]` 은 (901 OR 902) AND 911. 판정 대상은 선택된 옵션 값 집합이다(`JsonSkinSerializer.java:171-211`, `JSONSkinLoader.java:252-258`). 객체 위치의 배열은 조건을 만족하는 첫 가지의 `value` 를 택한다. 헤더 읽기에서는 옵션 집합이 비어 있다.

### 9.1 와일드카드 경로와 선택값 치환 (`SkinLoader.getPath`, `B/skin/SkinLoader.java:95-130`)

입력: `imagepath`(스킨 폴더 + `/` + 스킨이 쓴 경로), `filemap`(키: 스킨 폴더 + `/` + `filepath[].path`, 값: 선택된 파일명).

1. `filemap` 의 키를 돌며 `imagepath.startsWith(key)` 인 첫 키를 찾는다.
   - `foot = imagepath.substring(key.length())`
   - 결과 = `imagepath.substring(0, imagepath.lastIndexOf('*')) + filemap[key] + foot`
   - 이후 와일드카드 처리는 하지 않는다.
2. 일치하는 키가 없고 `imagepath` 에 `*` 가 있으면 무작위 선택:
   - `ext` = 마지막 `*` 뒤의 문자열. 경로에 `|` 가 있으면 `*` 뒤부터 첫 `|` 앞까지 + 마지막 `|` 뒤(마지막 `|` 가 끝이면 앞부분만). 즉 첫 `|` 와 마지막 `|` 사이는 버린다.
   - 폴더 = `imagepath` 의 마지막 `/` 앞.
   - 폴더 안에서 소문자로 바꾼 전체 경로가 `ext` 로 끝나는 항목을 모아 `Math.random()` 으로 하나 고른다. `ext` 는 소문자로 바꾸지 않으므로 대문자가 든 패턴은 일치하지 않는다.
   - 하나도 없으면 `*` 가 든 원래 경로를 그대로 돌려준다(없는 파일).
3. `*` 가 없으면 입력을 그대로 돌려준다.

예: `filepath = {name="배경", path="Select/bg/image/*.png"}` 에서 `a.png` 를 골랐고 소스가 `Select/bg/image/*.png` 면 결과는 `<스킨>/Select/bg/image/a.png`. 폴더형 `path="parts/notes/*"`, 소스 `parts/notes/*/note.png`, 선택 `blue` 면 `<스킨>/parts/notes/blue/note.png`.

주의:
- 1 번에서 `lastIndexOf('*')` 는 `imagepath` 전체의 마지막 `*` 다. `foot` 에 `*` 가 또 있으면 의도와 다른 위치에서 잘린다.
- 2 번의 무작위는 호출할 때마다 다시 뽑는다. 소스는 id 별로 한 번만 읽고 캐시하지만(`JSONSkinLoader.java:481-520`) `skin_config.get_path` 는 호출마다 결과가 달라질 수 있다.
- `filemap` 키가 서로 접두 관계일 때 어느 것이 먼저 걸리는지는 맵 순회 순서에 달려 있다.
- ModernChic 은 소스 경로에 와일드카드를 쓴다(`MC/decide.lua:235-236`, `MC/Result/lua/base.lua:27-34`, `MC/Select/lua/background.lua:12`). `get_path` 에는 와일드카드를 넘기지 않는다(grep 0 건).

### 9.2 소스 로딩 (`JSONSkinLoader.getSource`, `:481-520`)

- `source[].id → path` 를 맵에 넣고, 처음 참조될 때 `getPath` 로 해석한다.
- 파일이 있고 확장자가 동영상 형식이면 `SkinSourceMovie`, 아니면 텍스처. 파일이 없으면 데이터는 null 이고 다시 시도하지 않는다.

---

## 10. SkinHeader 와 사용자 설정 병합

### 10.1 헤더 생성 (`JSONSkinLoader.loadJsonSkinHeader`, `:115-210`)

| 헤더 항목 | 출처 | 비고 |
| --- | --- | --- |
| `skinType` | `SkinType.getSkinTypeById(sk.type)` | |
| `name`, `author` | `sk.name`, `sk.author` | null 이면 `""` |
| `path` | 진입 파일 경로 | |
| `CustomOption(name, int[] op, String[] contents, def)` | `property[i]` 의 `name`, `item[].op`, `item[].name`, `def` | `:131-151` |
| `CustomFile(name, path, def)` | `filepath[i].name`, `<스킨폴더>/` + `filepath[i].path`, `def` | `:153-165` |
| `CustomOffset(name, id, x, y, w, h, r, a)` | `offset[i]` | `:171-182` |
| 플레이 스킨 자동 오프셋 4 개 | 스킨 선언 뒤에 추가 | `:183-190` |
| `CustomCategory(name, items[])` | `category[i]` | `:193-204` |

플레이 스킨(5/7/9/10/14/24/24DOUBLE 키)에 자동으로 붙는 오프셋(`B/skin/SkinProperty.java:950-954`):

| 이름 | id | 조정 가능 성분 |
| --- | --- | --- |
| `All offset(%)` | 10 (`OFFSET_ALL`) | x, y, w, h |
| `Notes offset` | 30 (`OFFSET_NOTES_1P`) | h |
| `Judge offset` | 32 (`OFFSET_JUDGE_1P`) | x, y, w, h, a |
| `Judge Detail offset` | 33 (`OFFSET_JUDGEDETAIL_1P`) | x, y, w, h, a |

카테고리: `category[].item` 은 문자열 배열이고, `property[]`/`filepath[]`/`offset[]` 각 항목의 `category` 문자열과 같은 값이면 그 자리에 배치된다. 이름(`name`)이 아니라 `category` 필드로 맞춘다. 어느 카테고리에도 안 걸린 항목은 카테고리 목록에는 없지만 옵션 목록에는 남는다.

### 10.2 병합 (`SkinHeader.setSkinConfigProperty`, `B/skin/SkinHeader.java:152-221`)

사용자 설정 구조(`B/SkinConfig.java:70-161`): `option[] = {name, value}`, `file[] = {name, path}`, `offset[] = {name, x, y, w, h, r, a}`. 전부 이름으로 맞춘다.

옵션:
1. 기본값 = `def` 와 이름이 같은 항목의 `op`, 없으면 첫 항목의 `op`, 항목이 없으면 `-1` (`:294-300`).
2. 사용자 설정에 같은 이름이 있으면: 값이 `-1`(`OPTION_RANDOM_VALUE`, `B/skin/SkinProperty.java:621`)이 아니면 그 값, `-1` 이면 항목 중 하나를 `Math.random()` 으로 고른다.
3. 고른 값이 항목 목록에 있으면 그 인덱스를 선택으로 기록한다. 없으면 선택 없음이 되어 `getSelectedOption()` 이 `-1` 을 준다.

파일:
- 사용자 설정에 같은 이름이 있을 때만 처리한다.
- 값이 `"Random"` 이 아니면 그 문자열을 선택 파일명으로 쓴다.
- `"Random"` 이면 패턴에서 9.1 과 같은 방식으로 `ext` 를 구해 폴더 안에서 무작위로 하나 골라 그 파일명을 쓴다.
- 사용자 설정에 없으면 선택 파일명이 null 이고 `filemap` 에 들어가지 않는다. 이때 해당 와일드카드는 9.1 의 2 번(호출마다 무작위)으로 처리된다. 헤더의 `def` 는 이 단계에서 쓰이지 않는다.

오프셋: 같은 이름의 사용자 값을 쓰고 없으면 전부 0 인 새 값.

`def` 가 실제로 반영되는 곳은 설정 화면이다. 스킨 선택 화면이 사용자 설정에 없는 항목을 채워 넣는다(`B/config/SkinConfiguration.java:249-296,298-352`).
- 옵션: `def` 와 이름이 같은 항목, 없으면 첫 항목.
- 파일: 폴더 목록에서 `def` 와 대소문자 무시로 같거나 확장자를 뗀 이름이 같은 파일, 없으면 목록 첫 항목. 목록 끝에는 `"Random"` 이 붙는다.
- 옵션 목록 끝에도 `"Random"`(값 -1)이 붙는다.

### 10.3 스킨에 넘겨지는 값

- `skin.setOption`: 헤더의 모든 옵션 항목 `op → (선택됐으면 1, 아니면 0)` (`JSONSkinLoader.java:291-297`).
- `skin.setOffset`: `오프셋 id → 사용자 오프셋 값` (`:299-303`). `MainState.setSkin` 이 이를 전역 오프셋 배열에 복사한다.
- 무작위 옵션·무작위 파일은 로드할 때마다(화면에 들어갈 때마다) 다시 뽑힌다.

---

## 11. 해상도, 스케일, 좌표축

- 스킨 해상도는 `sk.w`, `sk.h`. 이 값이 `Resolution` 열거형 중 하나와 정확히 같을 때만 그 해상도로 인정하고, 아니면 HD(1280x720)로 본다(`JSONSkinLoader.java:263-269`).
- 인정되는 해상도(`B/Resolution.java:9-23`): 640x480, 800x600, 1024x768, 1280x720, 1280x960, 1366x768, 1400x1050, 1600x900, 1600x1200, 1680x1050, 1920x1080, 1920x1200, 2048x1536, 2560x1440, 3840x2160.
- 출력 해상도는 `config.getResolution()` 이고(`JSONSkinLoader.java:80,287`), 창 크기도 이 값으로 만든다(`B/MainLoader.java:166-167`).
- 배율: `dw = dst.width / src.width`, `dh = dst.height / src.height` (`B/skin/Skin.java:110-115`). 가로·세로가 따로다. 비율이 다르면 화면을 꽉 채우도록 늘어난다(레터박스 없음).
- 적용 대상: 모든 destination 의 `x, y, w, h` (`Skin.java:150-156`), `mouseRect` (`:183-185`), 클립 사각형 (`:187-189`). 객체에는 `setSkinScale(dw, dh)` 도 전달된다(글자 크기 등 객체별 사용).
- 좌표축: 원점은 왼쪽 아래, y 는 위로 증가한다. 근거는 `scissorCamera.setToOrtho(false, width, height)` (`Skin.java:390`, yDown = false)와 회전 중심 주석 "下端:0.0 - 上端:1.0" (`SkinObject.java:83-90`). 이미지 소스의 `x, y` 가 왼쪽 위 기준인지는 객체 로더 범위라 이 조사에서 확인하지 않았다(미확인).
- `OFFSET_ALL`(id 10): 플레이 화면에서만 전체 변환에 적용된다. 이동 `(width * x / 100, height * y / 100)`, 배율 `((w + 100) / 100, (h + 100) / 100)` (`Skin.java:382-388,720-729`). 렌더러를 처음 만들 때 한 번만 읽는다.
- 텍스처 영역 그리기에 `x + 0.01, y + 0.01` 보정이 들어간다(`Skin.java:612-626`).
- 블렌드 값별 설정(`Skin.java:655-665`): 2 는 `(SRC_ALPHA, ONE)`, 3 은 블렌드 방정식을 `FUNC_SUBTRACT` 로 바꾸고 `(SRC_ALPHA, ONE)` 을 건 뒤 곧바로 `FUNC_ADD` 로 되돌린다(주석 "減算描画は難しいか？"), 4 는 `(ZERO, SRC_COLOR)`, 9 는 `(ONE_MINUS_DST_COLOR, ZERO)`. 그린 뒤 `blend >= 2` 면 `(SRC_ALPHA, ONE_MINUS_SRC_ALPHA)` 로 복원(`:680-682`). 3 의 실제 화면 결과는 배치 플러시 시점에 달려 있어 이 조사에서 판정하지 않았다(미확인).

ModernChic 은 1920x1080 이라 FULLHD 로 인정된다(`MC/Play/lua/require/header.lua:12-13`).

---

## 12. LuaJ(Lua 5.2 계열)와 Lua 5.4(mlua)의 차이

R-BMS 현재 설정: `mlua = { version = "0.12.1", features = ["lua54", "vendored"] }` (`R/crates/rbms-skin/Cargo.toml:19`), 표준 라이브러리는 `MATH | STRING | TABLE` 만 연다(`R/crates/rbms-skin/src/lua.rs:110`).

| 항목 | LuaJ 3.0.2 | Lua 5.4 | ModernChic 영향 |
| --- | --- | --- | --- |
| 수 체계 | 정수값이면 내부적으로 정수로 정규화, 나머지는 double. `type` 은 항상 `number` | integer 와 float 하위 타입이 분리 | 아래 항목들의 원인 |
| 정수값 실수의 문자열화 | `tostring(10/2)` = `"5"` | `"5.0"` | 나눗셈·거듭제곱 결과를 id 나 파일명에 이어 붙이면 문자열이 달라진다. ModernChic 에서 id 에 직접 이어 붙이는 사례는 grep 으로는 0 건이나 전수 확인은 아님 |
| 비정수 실수의 문자열화 | `Float.toString((float)d)`: `1/3` → `0.33333334`, 작은 값은 `1.0E-4` 꼴 | `%.14g`: `0.33333333333333` | 화면 표시용 문자열을 Lua 에서 만들면 달라진다 |
| `/` 결과 | 정수로 떨어지면 정수 | 항상 float | Rust 로 받을 때 정수 필드에 float 가 온다. 버림으로 받아야 한다 |
| `^` 결과 | 정수로 떨어지면 정수 | 항상 float | 같음 |
| 정수 넘침 | double 로 승격 | 64 비트에서 순환 | 꺼진 타이머 값(-2^63)과의 뺄셈이 순환해 부호가 뒤집힌다. 타이머·시각은 float 로 넘겨야 한다 |
| `timer_off_value` | double -2^63 | integer 로 넘기면 `math.mininteger` | 비교는 같지만 산술이 다르다 |
| `math.pow`, `atan2`, `cosh`, `sinh`, `tanh`, `ldexp`, `frexp` | 있음 | 없음(호환 빌드가 아니면) | ModernChic 미사용. 다른 스킨 대비 필요 |
| `bit32` | 있음 | 없음 | ModernChic 미사용 |
| `math.log10` | 없음 | 없음 | 무관 |
| `unpack`, `loadstring`, `setfenv` | 없음 | 없음 | 무관 |
| `math.random(m)` 에 실수 인자 | 허용(버림, 미확인) | 정수 표현이 없으면 오류 | `MC` 에 `math.random` 13 회. 리터럴 정수 6 회, `#목록` 4 회, `(1, #msg)` 1 회, `(min, max)` 변수 2 회(`MC/Root/customnumber.lua:73`, `MC/Play/lua/sp/attack.lua:45`, 값의 출처 미확인) |
| 난수 시드 | 시작 시 임의 시드(Java `Random`, 미확인) | 5.4 는 시작 시 임의 시드, 5.2·5.3 은 고정 시드 | ModernChic 은 `math.randomseed` 를 부르지 않는다(grep 0 건). `lua52` 를 고르면 호스트가 시드를 넣어야 매번 같은 결과가 나오지 않는다 |
| `string.format("%d", 실수)` | 허용(미확인) | 정수 표현이 없으면 오류 | `MC/Play/lua/sp/detailinfo/nentyakuinfo.lua:47,52` 는 정수 인자라 안전 |
| `x % 0` | NaN | 정수끼리면 오류 | 미확인 |
| `os.time()` | 초 단위 수 | integer | `os.time() + 604800 .. "\n"` (`MC/Select/lua/require/http.lua:87`)은 둘 다 정수 문자열 |
| `table.insert(t, pos, v)` 범위 밖 | 관대(미확인) | 오류 | 미확인 |
| `pairs` 순서 | 구현 고유 | 구현 고유 | 순서에 의존하는 코드가 있으면 달라질 수 있다 |
| 함수 인자 개수 조회 | `narg()` 는 항상 1 | — | 이벤트 함수는 인자 1 개로 호출(5.2) |
| 문자열 → 숫자 판정 | `isnumber()` 가 숫자 모양 문자열에도 true | mlua 는 타입이 분리 | `op = {"901"}` 같은 값은 숫자로 처리해야 같다 |
| 불리언 강제 | 숫자 0 도 true | 같음 | `Offset.x = 0` 이 true 가 되는 점까지 같게 할지 결정 필요 |
| 코루틴 | Java 스레드 기반 | 네이티브 | ModernChic 미사용 |

선택지:

1. `lua54` 를 유지하고 경계에서 보정한다. 타이머·시각은 float 로 내보내고, 정수 필드는 float 를 버림으로 받고, 문자열 필드로 들어오는 float 는 정수값이면 `.0` 없이 문자열화하고, `bit32`·`math.pow` 등은 Lua 로 덧대어 준다. 스킨 내부의 `tostring`·`..` 결과는 고칠 수 없다.
2. mlua 의 `lua52` 기능으로 바꾼다. 수 체계가 double 하나라 `tostring(10/2)` 이 `"5"` 가 되고 `bit32`·`math.pow` 가 기본으로 있다. LuaJ 와 가장 가깝다. mlua 0.12 에 `lua52` 기능이 남아 있는지는 로컬 레지스트리에 소스가 없어 확인하지 못했다(미확인). 기존 한 줄 식 샌드박스(`R/crates/rbms-skin/src/lua.rs`)가 5.4 전용 API 에 의존하는지도 확인이 필요하다.

어느 쪽이든 비정수 실수의 문자열화(`Float.toString`)는 LuaJ 고유라 완전히 같아지지 않는다.

---

## 13. ModernChic 이 실제로 쓰는 환경 요소 (교차 확인)

전체 Lua 파일 대상 grep 결과다. ModernChic 내부 구조 분석은 다른 조사 범위이고, 여기서는 환경 계약과 닿는 부분만 적는다.

| 요소 | 사용 | 근거 |
| --- | --- | --- |
| 진입 패턴 | 10 개 `.luaskin` 모두 `require` 후 `skin_config` 유무로 `t.main()` / `t.header` | `MC/*.luaskin` |
| 전역 대입 | `main_state`, `timer_util`, `PROPERTY`, `COMMONFUNC`, `MAIN`, `CUSTOM`, `CONFIG`, `BASE` 등을 전역으로 둔다. `dofile` 로 읽은 파일이 이 전역에 의존 | `MC/play7_hw.lua:8-20`, `MC/result.lua:8-10` |
| `require` 점 표기 | `Root.define`, `Play.lua.require.header` 등 폴더를 `.` 로 구분. 대소문자가 실제 폴더명과 같아야 한다 | `MC/play7_hw.lua:10-21` |
| `dofile(skin_config.get_path(...))` + `pcall` | 부품 파일마다 사용. 실패해도 조용히 그 부품만 빠진다 | `MC/play7_hw.lua:46-55`. 전체 Lua 126 개에서 `dofile(` 109 회, `pcall(` 110 회 |
| `io.open` | 45 회. 모드 `"a"` 19, `"w"` 14, `"r"` 8, 생략 4. `:lines()` 9, `:write()` 41, `:close()` 42 | `MC/Result/lua/history.lua`, `MC/Root/customfunction.lua:12-40` |
| `os.date` / `os.time` | 5 회 / 3 회. `os.date('*t').wday`, `os.date("%Y-%m-%d", os.time() - 3600 * 6)` 포함 | `MC/Root/customtime.lua:6-10`, `MC/Play/lua/sp/detailinfo/bgaareainfo.lua:148-150` |
| `luajava.bindClass("java.io.File")` + `luajava.new(File, path)` 의 `mkdir()`, `listFiles()` | 폴더 생성과 파일 목록 | `MC/Root/customfunction.lua:7-8,102-130` |
| `luajava.bindClass("com.badlogic.gdx.Gdx")`, `("com.badlogic.gdx.Input")` | `Gdx.input:isKeyPressed(input.Keys.RIGHT/LEFT/UP/DOWN)` | `MC/Result/lua/mainmenu.lua:5-7,986-989`, `MC/Result/lua/irmenu.lua:6-8,654-657` |
| `luajava.newInstance("java.net.URL", url)` 등 | 선곡 화면의 버전 확인용 HTTP GET. `connect()` 를 `pcall` 로 감싸 실패를 허용 | `MC/Select/lua/require/http.lua:20-50` |
| 값을 돌려주지 않는 `draw` 함수 | 프레임 훅으로 사용 | `MC/Result/lua/mainmenu.lua:984-994` |
| 헤더 실행 경로 | 플레이 스킨의 최상위 실행은 순수 Lua 뿐이고 `skin_config`·`main_state` 호출은 클로저 안에 미뤄져 있다 | `MC/Play/lua/require/header.lua:6-30`, `MC/Play/lua/require/sp_property.lua:50,69-73` |

스킨 폴더 안에 `io` 라는 이름의 폴더가 있다(`MC/io/Play`, `MC/io/Result`, `MC/io/Select`). 스킨이 읽고 쓰는 데이터 폴더로 보이며 `require("io")` 와는 무관하다(표준 `io` 는 `package.loaded` 에 이미 있다).

---

## 14. R-BMS 구현 사양으로 옮길 때의 요약

1. 스킨 로드마다 새 Lua 상태를 만든다. 헤더 실행과 본 실행은 같은 상태에서 연달아 한다(`skin_config` nil → 설정 → 재실행). `package.loaded` 는 두 실행 사이에 유지한다.
2. 헤더만 읽는 용도(스킨 목록·설정 UI)는 별도 상태에서 `main_state`/`timer_util`/`event_util` 을 빈 테이블로 두고 한 번만 실행한다.
3. `require` 는 `.` 을 `/` 로 바꿔 스킨 루트에서 찾는다. `dofile`/`loadfile` 은 넘겨받은 경로를 그대로 연다. `get_path` 가 돌려주는 경로와 `dofile`·`io.open`·`audio_play` 가 받는 경로가 같은 기준이어야 한다. 절대 경로로 통일하고 스킨 루트 밖은 거부하는 방식이 단순하다.
4. 표준 라이브러리는 base, package(`require`, `loaded`, `path`), table, string, math, coroutine, 제한 io, 제한 os(`clock`, `date`, `difftime`, `time`, `setlocale`), `bit32` 를 연다. `luajava` facade 는 2.3 표의 범위로 만든다.
5. 변환기는 4 장 규칙을 따른다. 알 수 없는 키 무시, 누락 기본값, 정수 필드의 버림, 문자열 필드의 숫자 허용, 배열 구멍 메움, 속성 필드의 함수·숫자·문자열 분기 순서.
6. Lua 함수 호출은 오류를 잡아 기본값(false, 0, `""`, 꺼짐)으로 대체하고 계속 호출한다. beatoraja 는 매 프레임 로그를 남기지만 R-BMS 는 로그를 제한해도 화면 결과는 같다.
7. 프레임 순서: 시각 확정 → 화면 로직 → 커스텀 타이머 → 커스텀 이벤트 → 객체 prepare(조건 → 타이머 → 값) → 그리기 → 입력(클릭).
8. 이벤트 함수는 인자 1 개로 호출한다.
9. 타이머 값과 시각은 마이크로초 실수, 꺼짐은 -2^63.
10. 와일드카드 해석은 9.1, 설정 병합은 10.2 를 따른다. 사용자 설정이 없을 때 `def` 를 로드 시점에 적용할지(beatoraja 는 설정 화면에서만 적용)는 결정이 필요하다.
11. 스케일은 가로·세로 독립, 원점 왼쪽 아래, y 위쪽.

---

## 15. 미확인 사항과 읽지 못한 것

배정 파일은 전부 읽었다: `skin/SkinLoader.java`, `skin/lua/*.java` 12 개, `skin/json/JSONSkinLoader.java`, `JsonSkin.java`, `JsonSkinSerializer.java`, `skin/SkinHeader.java`, `skin/SkinType.java`, `skin/CustomTimer.java`, `skin/CustomEvent.java`, `SkinConfig.java`, `Resolution.java`, `MainState.java`, `config/SkinConfiguration.java`, `play24.luaskin`.

부분만 읽은 것:
- `MainController.java`: 스킨 관련 구간(195-300, 380-420, 490-512)만.
- `skin/Skin.java`: 전체. `skin/SkinObject.java`: 1-420, 520-600 행.
- `skin/default/play24main.lua`: 1-140 행(전체 775 행). `play_parts.lua` 는 읽지 않음.
- `skin/property/*Factory.java`: id·이름 조회 진입부만. 개별 id 의 의미는 범위 밖.
- `skin/json/JsonSkinObjectLoader.java` 와 화면별 객체 로더: `ref`/`value` 우선순위 확인용 grep 만.
- `TimerManager.java`, `SkinPropertyMapper.java`(100-167 행), `launcher/SkinConfigurationView.java`(160-215 행).

미확인:
- LuaJ `IoLib` 가 IOException 을 `nil, 메시지` 로 바꾸는 정확한 반환 형태.
- LuaJ `LuaTable.keys()` 의 순회 순서 세부. 배열 부분 다음에 해시 부분을 도는 것까지는 확인했고(`next` 반복), 문자열 키와 정수 키가 섞인 테이블에서 정수 키가 해시 부분에 머무를 때의 순서는 확인하지 못했다.
- libGDX `IntMap` 순회 순서, `LwjglInput.isKeyPressed(-1)` 의 동작, `Input.Keys` 상수의 실제 숫자.
- LuaJ 의 `string.format("%d", 실수)`, `math.random(실수)`, `table.insert` 범위 밖, `x % 0` 동작.
- LuaJ 소스 인코딩 처리(UTF-8 가정)와 BOM 처리.
- mlua 0.12 의 `lua52` 기능 제공 여부.
- 이미지 소스 좌표(`Image.x/y`)의 기준점.
- ModernChic 의 `customTimers`/`customEvents` 실제 내용, `math.random` 인자 형태, 숫자를 문자열에 이어 붙이는 모든 지점.
