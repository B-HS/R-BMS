# R1 — `crates/rbms-skin` 현황과 풀 Lua 스킨까지의 격차

> 최종 갱신 2026-10-11 · 대응 단계: 웨이브 7B(동영상 source) 반영 · 본문은 작성 시점 서술이며, 아래 "웨이브 … 반영 사항"이 최신 것부터 우선한다 · 색인과 갱신 규칙은 [README.md](README.md)

## 웨이브 7B 반영 사항 (2026-10-11)

`crates/rbms-video`(OpenH264 + `re_mp4`)와 스킨 동영상 source 재생을 넣은 뒤의 상태다.

- (W7-2) r1-rbms-skin.md: 모델·로더 절에 SourceKind(Image/Movie), resolve::source_kind 와 MOVIE_NAME_ENDINGS, LoadedSkin.movie_sources 와 source_kind() 추가


## 웨이브 3A 반영 사항 (2026-10-10)

prepare/draw 2단계 파이프라인과 `SkinHost` 직접 그리기, 그리기 조건 의미론, 참조 이미지·음수 크기·이미지 인덱스·숫자·슬라이더·그래프 정합, TTF 텍스트, judgegraph·bpmgraph, Lua 함수 값 프레임 평가, 앱 호스트 군집 A·I·M 을 넣은 뒤의 상태다.

- (W3-1b) r1-rbms-skin.md §6.2 D2: 해소. 정수 op 는 원본 구현 표(property::reference_implements(NameSpace::Boolean, id))로 갈라 내장이면 DrawCondition::Option(부호 유지), 아니면 스킨 옵션(dstop)으로 분리해 로드 뒤 옵션 맵(loader.rs option_selections, 행 순서대로 put)과 1회 대조. 맵에 없으면 부호와 무관하게 객체 제거. known_option·every_option_known 은 삭제됨
- (W3-1b) r1-rbms-skin.md §6.2 D3: 해소. loader.rs assemble 끝의 prepare_objects 가 최상위 객체에 한해 SkinHost::is_static(id) 인 조건을 1회 평가(거짓 = 객체 제거, 참 = 조건 제거). 이름 조건(DrawCondition::Name)은 id_of_name 으로 풀어 같은 규칙. 호스트 답이 None 이면 정리하지 않고 프레임 조건으로 남김. 중첩 슬롯은 정리 대상이 아님
- (W3-1b) r1-rbms-skin.md §6.2 D4·D17: 음수 타이머 id 는 타이머 없음으로 해소(model.rs PropertyRef::timer 가 None, destination 과 이미지 셀 타이머 공통). timer 0 은 그대로 TimerId(0). 웨이브 1 진행 기록의 '음수 타이머 id … 아직 옮기지 않았다(W3-1b)' 도 완료
- (W3-1b) r1-rbms-skin.md §2.2 표: SkinLoadOptions 행에서 known_option 삭제. dst::draw_conditions_from_ops 의 반환형이 OpLists { conditions, options } 로 바뀜(신규 행 OpLists). LoadedSkin.destinations 는 '조립 후 Skin.prepare 정리를 거친 목록'. loader 내부 TrackContext 는 { path, relative, warnings } 로 줄고 track::build_track 은 BuiltTrack { track, options } 를 돌려줌. assemble 은 host 인자를 받음
- (W3-1b) r1-rbms-skin.md §2.1 흐름: 로드 순서 끝에 '정리 단계(Skin.prepare): 스킨 옵션 대조 → 정적 조건 1회 평가' 추가. load_skin(DefaultState)은 정적 정리를 하지 않음(is_static 이 항상 false)
- (W3-1b) r1-rbms-skin.md §8 테스트 표: src/loader/track.rs 인라인 6건 → 10건, tests/skin_loader.rs 75건, tests/skin_luaskin.rs 25건 → 30건(선택 테스트 an_external_skin_pack_sheds_objects_and_conditions_when_it_is_prepared 포함), tests/skin_integration.rs 17건(known_option 주입 테스트 4건을 원본 구현 표 기준 테스트로 교체)
- (W3-1b) r1-rbms-skin.md §0 요약 5번과 §9: '남은 것은 미선언·미지원 op 처리' 를 완료로 표시
- (W3-1c) r1-rbms-skin.md 상단에 웨이브 3 반영 사항 추가: BoundFrame::timer(프레임 내 재사용, 프레임 번호 스탬프 캐시), LuaDrawEval::call_timer 가 이 경로를 탐, SkinLua::frame_cost() -> FrameCost { calls, reused, spent }, Meter::frame_calls / frame_spent. §5.4 예산 절에 '거부·중단 시 직전 프레임 값'이 타이머 재사용에도 적용됨을 추가
- (W3-1c) r1-rbms-skin.md §8 테스트 표: tests/skin_main_state.rs 31건 → 38건
- (리뷰 수정) r1-rbms-skin.md 의 Lua os 절: os.rs 에 호스트용 local_time(LocalTime) 이 생겼고 lua 모듈이 재수출한다고 추가합니다


## 웨이브 2B 반영 사항 (2026-10-10)

스킨 덤프 CLI, 앱의 스킨 팩 폴더 지정과 `.luaskin` 로드, 오버레이 총 크기 상한, 외부 스킨 첫 정지 프레임을 넣은 뒤의 상태다.

- (W2-10) r1-rbms-skin.md §2.2: 타입 표에 lua::OverlayLimits { max_bytes 64 MiB, max_entries 4096, resync_interval 5초 }, SkinLuaConfig.overlay_limits 필드, 크레이트 내부 io::OverlayQuota(measure/admit_change/record_change/admit_entries/create_directories/copy_original/invalidate)를 추가
- (W2-10) r1-rbms-skin.md §5.4: S7(io) 에 '오버레이 폴더 현재 총 크기와 항목 수 상한, 쓰기 전 확인, 초과 시 nil+메시지(io) 또는 false(헬퍼), 크기를 늘리지 않는 쓰기는 항상 허용' 을 구현 완료로 기록
- (W2-10) r1-rbms-skin.md §8: 테스트 표에 tests/skin_lua_io.rs 27개(오버레이 상한 8개 추가)로 갱신
- (W2-6) r1-rbms-skin.md §5.4 예산 표와 00-synthesis.md:14 '로드는 명령 수(5억)': 로드 명령 수 기본값이 1억(DEFAULT_LOAD_INSTRUCTIONS)으로 바뀌었고 나머지 상수(로드 10초, 프레임 호출 16,384, 프레임 50ms, 호출당 100만, 메모리 256MiB)는 '잠정'이 아니라 실측 근거를 가진 값이다. 5억에서는 폭주 루프가 최대 3.5초 뒤 차단됐다.
- (리뷰 수정) r1-rbms-skin.md 상단 '웨이브 2B 반영 사항', §5(샌드박스): OverlayQuota 가 스레드 로컬 장부(오버레이 경로 → Ledger)를 공유하고, missing_directories 와 admit(entries, before, after) 로 디렉터리 생성 전에 전부 승인한다. io.open, main_state.file_write/file_append, luajava File:mkdir 에 적용


## 웨이브 2A 반영 사항 (2026-10-10)

Lua 5.2 런타임(`crates/rbms-skin/src/lua/`), `SkinHost`, Lua 값 변환기, 2패스 `.luaskin` 로더를 넣고 구 샌드박스(`skin.*`)를 삭제한 뒤의 상태다.

- (W2-0) r1-rbms-skin.md §1 모듈 구성 표: `lua` 행을 `lua/{mod, budget, coerce, env, io, os, luajava, main_state, package, legacy}.rs` + `prelude.lua` 로, `loader` 행에 `loader/{from_lua, lua_skin}.rs`, `property` 행에 `property/host.rs` 를 추가. 한 줄 식 샌드박스 위치는 `src/lua.rs` 가 아니라 `src/lua/legacy.rs`.
- (W2-0) r1-rbms-skin.md §2.2 표: `OffsetSource / DrawStateSource / SkinStateSource` 행을 `DrawStateSource::boolean(id) -> Option<bool>`(None = 미구현), `SkinHost`(property/host.rs, 구 SkinStateSource)로 교체. `string(id) -> &str` 은 `text(id) -> Cow<str>`. 새 메서드 `is_static, image_index, rate, exec_event, write_rate, write_text, audio, key_pressed, screen_size, gauge, gauge_type, judge, score, volume, set_volume` 을 적는다.
- (W2-0) r1-rbms-skin.md §2.2 표: `SkinError` 행 10종 → 11종(`LuaLoad { path, message }` 추가). `SkinLoadOptions` 행에 `write_overlay: Option<&Path>`, `LoadedSkin` 행에 `runtime: Option<SkinLua>`(접근자 `runtime()`), `SkinHeader`/`LoadedSkin` 의 `ParserKind` 에 `Lua` 변형, `LuaSandbox / LuaFrame` 행 위치를 `src/lua/legacy.rs` 로.
- (W2-0) r1-rbms-skin.md §5.1: `skin.boolean(id)` 가 호스트의 None 을 false 로 읽는다는 점과 파일 위치(`src/lua/legacy.rs`)를 반영.
- (W2-0) r1-rbms-skin.md §5.4: S8 은 `LuaBudget { load: LoadBudget, frame: FrameBudget, max_memory_bytes }` + `Meter` 로 구조 확정(수치 잠정, 훅 미구현). S10 은 `SkinLua::frame` + `main_state::bind` 로 확정. S12 는 `LuaExprId` 재사용이 아니라 `LuaFnId`(dst.rs) + `LuaFnKind` 레지스트리(`SkinLua::register`, 함수+종류 동일성으로 중복 제거). S14·S16 은 `LuaLog`(함수별 첫 메시지+횟수, pcall 삼킴 목록, print) 로 구조 확정.
- (W2-0) r1-rbms-skin.md §5.5: '안전한 구성 한 가지' 를 확정 설계로 바꾼다. 숨은 host 테이블은 레지스트리 키 `rbms.skin.host`, 바인딩 진입점은 `lua/main_state.rs` 의 `bind(lua, shared, scope, host)`, 로드 패스도 `SkinLua::run_entry` 가 같은 방식으로 호스트를 묶는다.
- (W2-0) r1-rbms-skin.md §9 '소비자에 전파되는 시그니처 변경': `SkinStateSource` → `SkinHost` 개명, `boolean` 의 Option 화, `string` → `text` 를 완료로 표시.
- (W2-1) r1-rbms-skin.md §2.1(107행 부근): 식 평가 설명에 '함수 값과 이름은 LuaDrawEval 의 call_boolean/integer/float/text/timer(LuaFnId), named_boolean/integer/float/text(&str) 로 평가하며 기본 구현이 beatoraja 기본값(false/0/0.0/""/TIMER_OFF)을 돌려준다. 널 평가기는 dst::NullLuaEval' 을 추가
- (W2-1) r1-rbms-skin.md §2.2 표: PropertyRef 행을 'Id(i32) / Func(LuaFnId) / Name(String) / Expr(String), 접근자 id·function·name·expr·timer' 로. DrawCondition 행을 'Option(i32) / Lua(LuaExprId) / Function(LuaFnId) / Name(String), Copy 아님' 으로. LuaDrawEval 행에 함수·이름 메서드 9개와 NullLuaEval 추가. TimerRef 행을 'Id(TimerId) / Lua(LuaFnId), value_us(timers, lua)' 로. dst::resolve 시그니처에 끝 인자 lua: Option<&dyn LuaDrawEval> 추가. 신규 행 EventRef, FloatWriterRef, StringWriterRef, DestinationOption::UNCONDITIONAL
- (W2-1) r1-rbms-skin.md §3.3 표의 'rbms 현재' 열: BooleanProperty·IntegerProperty·FloatProperty·StringProperty 행을 'PropertyRef::Func/Name 으로 모델이 받는다. 함수는 평가기 경유, 이름표는 W2-3 대기' 로. TimerProperty 행을 'Func 는 TimerRef::Lua 로 destination 과 이미지 셀 타이머 모두 지원. 문자열은 여전히 경고 후 프레임 시계' 로. FloatWriter 행을 'SliderDef.event: Option<FloatWriterRef>', StringWriter 행을 'TextDef.event: Option<StringWriterRef>(숫자는 None)', Event 행을 'ImageDef.act·ImageSet.act·CustomEvent.action: Option<EventRef>' 로. DestinationOption 행에 '함수 수용, Lua boolean 은 UNCONDITIONAL' 추가. 말미 결론의 (b) 를 완료로 표시
- (W2-1) r1-rbms-skin.md §6.2 D4: '함수 타이머 해소(TimerRef::Lua, 프레임마다 LuaDrawEval::call_timer). 이미지 셀 애니메이션 타이머도 Sprite.timer: Option<TimerRef>. 식 문자열 타이머는 미해소(경고 후 프레임 시계)' 로. 줄 번호 track.rs:231-242 와 skin_render/object.rs:524-526 은 낡았다(timer_of 와 cell_timer 가 PropertyRef::timer() 를 쓴다)
- (W2-1) r1-rbms-skin.md §6.2 D14: '모델에 PropertyRef::Name 과 평가 통로 named_* 가 생겼다. 이름→id 역표와 JSON 문자열의 이름 우선 조회는 미구현(W2-3, W2-5)' 로
- (W2-1) r1-rbms-skin.md §8 테스트 표: src/dst/tests.rs 41건 → 49건, tests/skin_model.rs 21건 → 32건, src/loader/track.rs 에 인라인 테스트 6건 신설
- (W2-1) r1-rbms-skin.md §9 U3 행: 완료로 표시. 실제 형태는 PropertyRef::Func(LuaFnId)(LuaExprId 아님), TimerRef::Lua(LuaFnId), 타이머 값은 콜백이 아니라 resolve 의 lua 인자로 받는다. '소비자에 전파되는 시그니처 변경' 의 'PropertyRef 변형 추가(U3) → skin_render/object.rs:267-272, 524-526' 항목을 완료로
- (W2-2a) r1-rbms-skin.md §5.1: 현재 구조를 lua/{mod, env, package, budget}.rs 기준으로 다시 서술. 구 샌드박스는 lua/legacy.rs 로만 남음
- (W2-2a) r1-rbms-skin.md §5.4: S1(io·os 제외)·S2·S3·S4·S5·S6·S8·S9·S12·S14·S16 을 구현 완료로, S15 를 '고정 시드에서 설정 시드로 변경'으로 표시. S6 의 'C 내부 폭주는 벽시계로만 막는다'는 '훅이 C 내부에 걸리지 않아 벽시계로도 못 막는다'로 정정
- (W2-2a) r1-rbms-skin.md §5.5: 로드 뒤 변환·타이머 스크립트 시험 호출용 SkinLua::with_host 가 추가됐음을 반영
- (W2-2a) r1-rbms-skin.md §5.6: 재사용 표에 '훅 기반 한도는 set_hook 이 아니라 set_global_hook(코루틴 포함)', 'ChunkMode::Text 강제는 Lua 쪽 load 모드 t 강제와 병행', 'WarnOnce 대신 함수별·메시지별 집계'를 반영
- (W2-2a) r1-rbms-skin.md §2.2: 타입 표에 SkinPaths(logical/script/readable/writable/entries/display/chunk_name), LuaDiagnostics(PrintedLine, SwallowedError 의 file·line), Meter, error_message, error_location 추가
- (W2-2a) r1-rbms-skin.md §8: 테스트 표에 tests/skin_lua_env.rs 45건과 fixtures/luaenv 추가
- (W2-4) r1-rbms-skin.md §3.3: 표의 'rbms 현재' 열과 결론 문단이 낡았습니다. Lua 경로는 crates/rbms-skin/src/loader/from_lua.rs 의 skin_def_from_lua 가 구현합니다: 일반 필드는 mlua 값용 serde Deserializer(Reader)가 모델의 Deserialize 정의를 재사용해 toint/tofloat/toboolean/tojstring 규칙으로 읽고, 참조 필드(BooleanProperty~Event, DestinationOption)와 FontFallback 은 두 번째 순회(References)가 함수 → id → 이름 → 컴파일 순서로 읽습니다. 'JSON 경로의 이름 조회도 rbms 에 없다'는 문단은 JSON 경로에 한해 그대로입니다.
- (W2-4) r1-rbms-skin.md §3.1: `source[]`, `font[]` 행의 'FontFallback 의 문자열 단축형은 미지원'을 'Lua 경로는 지원(from_lua.rs font_fallback), JSON 경로는 미지원'으로 바꿔야 합니다.
- (W2-4) r1-rbms-skin.md §3.2: `Animation` 행 비고에 'Lua 경로에서 time 을 포함한 모든 정수 필드는 toint(32비트)로 읽는다'를 추가해야 합니다.
- (W2-4) r1-rbms-skin.md §8 테스트 표: tests/skin_from_lua.rs 17건(선택 테스트 1건 포함)과 src/lua/coerce.rs 단위 테스트의 Float.toString 1건 추가를 반영해야 합니다.
- (W2-4) r1-rbms-skin.md §9: 'Lua 값 → SkinDef 전용 변환기' 단위를 완료로 표시하고, 남은 것은 lua_skin.rs 의 2패스 로더(W2-5)임을 적어야 합니다.
- (W2-2b) r1-rbms-skin.md §5.4: 풀 Lua 실행을 위해 바꿔야 할 것 중 io·os 항목을 구현 완료로 갱신해야 합니다(lua/io.rs: Rust UserData 파일 핸들, lua/os.rs: C os 를 연 뒤 허용 목록만 남기고 원본 테이블 비움). 테스트는 tests/skin_lua_io.rs 18개, 픽스처 tests/fixtures/luaio 입니다.
- (W2-2b) r1-rbms-skin.md §8: 테스트 자산에 tests/skin_lua_io.rs 와 tests/fixtures/luaio/{files.lua, data/*.txt} 를 추가해야 합니다.
- (W2-3) r1-rbms-skin.md §2.2: 타입 표에 property::names(NameSpace, NumberedNames, StaticScope, StaticScreen, id_of_name, name_of_id, reference_implements, reference_writes, static_scope)와 lua::main_state(named_id, is_property_name, custom_timer_us), BoundFrame::host, 'BoundFrame 이 dst::LuaDrawEval 구현' 을 추가.
- (W2-3) r1-rbms-skin.md §5.5: '상태 바인딩 설계 제약' 을 구현 결과로 갱신 — 트램펄린은 Lua 함수, 숨은 host 테이블은 레지스트리 키 rbms.skin.host, bind 가 scope 함수로 채우고 scope 소멸자가 비운다. unsafe 없음. 바인딩은 중첩되지 않는다. 호스트가 필요 없는 함수(file_*, set_timer, http_*)는 영구 Rust 함수.
- (W2-3) r1-rbms-skin.md §8: 테스트 표에 tests/skin_main_state.rs 28건 추가, tests/property_checksum.rs 는 29건(이름표 체크섬·개수·정적 목록·writer 목록 5건 추가), src/property/names.rs 단위 10건, host.rs 4건, lua/main_state.rs 3건.
- (W2-3) r1-rbms-skin.md §9: 생성 도구 항목에 'tools/gen-skin-property.rs 가 팩토리 enum 이름표·정적 목록·writer 목록도 생성(generated/names.rs, NAME_TABLE_CHECKSUM)' 과 바뀐 실행법(rustc --edition 2024, 생성 파일에만 rustfmt) 추가.
- (W2-5) r1-rbms-skin.md §2.1·§2.2: 진입점을 `load_skin`(DefaultState) / `load_skin_with_host` / `lua_skin::{load_lua_skin, load_lua_header}` 로 갱신. `LoadedSkin` 에 resolution·play(PlayTimings)·offsets·selected_options 추가, `lua()` 삭제, `runtime()` 이 JSON 에서도 Some. `SkinHeader` 에 categories 추가와 자동 오프셋 포함. `SkinLoadOptions.lua_budget`·`Budget` 삭제
- (W2-5) r1-rbms-skin.md §4.2 L1·L2·L4: 완료(확장자 분기, 헤더 패스 결과를 from_lua 로 변환, Lua 경로는 branch 변환 없음)
- (W2-5) r1-rbms-skin.md §4.2 L3: 'Lua 진입 파일에 max_document_bytes, require·dofile 파일에 env.rs 의 파일당 64MiB, include 한도는 JSON 전용'으로 확정
- (W2-5) r1-rbms-skin.md §4.2 L6: 완료 — `resolve::default_candidate` 가 def 를 전체 이름 또는 스템과 대소문자 무시로 대조. ModernChic 9개 화면의 source 전부가 디스크의 실제 파일로 해석됨을 실행으로 확인
- (W2-5) r1-rbms-skin.md §4.2 L7·L8·L9: 의도적 차이로 유지(코드 문서에 명시). 추가: `scan_candidates` 가 파일뿐 아니라 폴더도 후보로 포함(원본 listFiles 와 같음, `*|1P|` 슬롯용)
- (W2-5) r1-rbms-skin.md §4.2 L10: 무작위(-1) 지원 완료(`branch::merged_options`, 시드 난수). 항목에 없는 저장값은 def 폴백 유지로 결정
- (W2-5) r1-rbms-skin.md §4.2 L11·L14·L16: 완료 — 자동 오프셋 4종(타입 0,1,2,3,4,16,17), `is_known_skin_type`(0~18)로 로드 허용(`is_supported_skin_type` 은 앱의 '그릴 수 있나' 질문으로 유지), `skin_resolution`(Lua 경로는 def.w/h 도 교체, JSON 은 resolution 필드만)
- (W2-5) r1-rbms-skin.md §5 전체(§5.1 현재 구조, §5.6 재사용·버릴 것): `lua/legacy.rs`(구 lua.rs) 삭제 완료. skin.* 6함수와 한 줄 식 전용 Budget 없음. JSON 식은 `loader/script.rs` 가 로드 시 SkinLua 에 컴파일(이름표 조회 → return 접두 컴파일, 타이머는 시험 호출, 이벤트·writer 는 return 없이). 상단 '웨이브 1B 반영 사항'의 §5.1 skin.timer/skin.time 항목도 폐기
- (W2-5) r1-rbms-skin.md §6.2: '타이머 식(함수) 미지원' 차이는 JSON 경로에서도 해소(문자열 타이머가 TimerRef::Lua 로 컴파일됨)
- (W2-5) r1-rbms-skin.md §8 테스트 표: skin_lua.rs 는 legacy 33건 삭제 후 JSON 식 경로 11건으로 재작성, skin_luaskin.rs 25건 신규(미니 픽스처 + RBMS_SKIN_PACK 선택 1건), skin_loader.rs 68건(식 관련 4건 교체), skin_integration.rs with_lua 5건 재작성, dst/tests.rs 에서 LuaExprId 테스트 3건 삭제. 픽스처 `tests/fixtures/luaskin/mini` 추가
- (W2-5) r1-rbms-skin.md §9: 2패스 로더·헤더 병합·legacy 제거 단위를 완료로 표시. '소비자에 전파되는 시그니처 변경'에 SkinExprEval → LuaDrawEval, SkinAssets::expression 삭제, LoadedSkin::lua 삭제 추가
- (리뷰 수정) r1-rbms-skin.md §5.4: '풀 Lua 실행을 위해 바꿔야 할 것' 의 예산 항목에 훅(1,000명령 간격)이 보지 못하는 구간과 그 대책을 적어야 합니다. 종료자는 훅이 꺼진 채 돌므로 등록 자체를 막았고, 패턴 함수는 Rust 로 옮겨 단계를 청구하며, 호스트 측 메모리(이름 캐시·줄 읽기·gsub 결과)는 별도 상한을 둡니다
- (리뷰 수정) r1-rbms-skin.md §5.5: 프레임당 1회 `scope` 바인딩 설명에 '프레임 벽시계는 바인딩 전체가 아니라 호출 구간만 잰다. 앱이 그리기 전체를 한 번의 frame 으로 감싸도 래스터 시간이 스킨 예산을 깎지 않는다'를 추가해야 합니다


## 웨이브 1B 반영 사항 (2026-10-10)

타이머 µs, stretch 11종, 장면 시계, 마우스 이벤트, GPU 논리 크기 런타임화와 색 공간, START/SELECT, 해상도 설정, 캡처 하니스를 넣은 뒤의 상태다.

- (W1-4) r1-rbms-skin.md §0 요약 5번: '확인된 차이는 4건' 중 오프셋 r 부호와 타이머 해상도(ms vs µs)는 W1-4 에서 해소. 남은 것은 미선언·미지원 op 처리와 타이머 식(함수) 미지원
- (W1-4) r1-rbms-skin.md §2.1 흐름(90행 부근): `SkinFrame { now_ms, ... }` 를 `SkinFrame { now_us, ... }`(µs)로
- (W1-4) r1-rbms-skin.md §2.2 표: `SkinStateSource` 행의 `timer(id)`/`now_ms()` 를 `timer_us(id) -> i64`(OFF = TIMER_OFF)/`now_us()` 로. `TimerState` 행을 '켜진 시각(µs) 저장소, OFF = i64::MIN, API set_on/off/switch/is_on/value_us/clear, 상수 TIMER_OFF·MICROS_PER_MILLI' 로. `StretchKind`/`stretch_rect` 행에 반환형 `(SkinRect 목적지, SkinRect 소스)` 와 `is_supported` 삭제를 반영. `DestinationTrack` 설명에 `timer: Option<TimerRef>`, `acc: Acc` 필드 추가와 `Keyframe.acc`·`effective_acc()` 삭제를 반영
- (W1-4) r1-rbms-skin.md §5.1 표 288행 '노출 API': `skin.timer(id)`(꺼져 있으면 nil, ms), `skin.time()`(ms) 를 `skin.timer(id)`(µs, 꺼져 있으면 -2^63), `skin.time()`(µs) 로
- (W1-4) r1-rbms-skin.md §6.1 표: 'acc' 행의 rbms 위치(dst.rs:134-140, 396)는 유지하되 트랙 단일 값(`DestinationTrack.acc`)임을 적는다. 타이머 `switch` 행의 줄 번호(timer.rs:79-85)가 바뀌었다. 새 일치 행 추가: '경과 시간 = now_us/1000 - timer_us/1000(시계와 타이머를 각각 절삭)' rbms `dst.rs resolve`, `skin_render/object.rs animation_index` / 레퍼런스 `SkinObject.java:352-358`, `TimerProperty.java:8-10`, `TimerManager.java:33-35`
- (W1-4) r1-rbms-skin.md §6.2: D1(오프셋 r 부호) → 해소(dst.rs 가 `angle_deg - offset.r` 로 문서 공간 덧셈과 동치). D5(타이머 해상도) → 해소(µs 저장, `timer_us`/`now_us`). D6(OFF 표현) → 해소(`TIMER_OFF = i64::MIN`, Lua 에는 -2^63 double). D8(acc 결정) → 해소(로더 track.rs 가 선언 순서 첫 비영 값을 `DestinationTrack.acc` 로, 4 이상·음수는 자리를 차지하고 선형). D10(stretch) → 해소(11종). D4 의 줄 번호 `track.rs:231-242` 는 약 241-252 로 이동했고 반환형이 `Option<TimerRef>` 다
- (W1-4) r1-rbms-skin.md §6.3: 제목 '미구현 7종' 과 표의 rbms 열을 전부 '구현' 으로. 마지막 문단의 '시그니처로는 표현할 수 없다 … 바꿔야 한다' 를 '`stretch_rect(kind, rect, source: SkinRect) -> (SkinRect, SkinRect)` 로 변경 완료, `(int)` 는 Java 캐스트 의미(0 방향 절삭·포화·NaN 은 0)로 옮김, 소스 면적 0 이하는 무변경 반환' 으로
- (W1-4) r1-rbms-skin.md §8 테스트 표: `src/timer/tests.rs` 14건 → 15건(상태 테스트 µs·센티널로 재작성), `src/dst/tests.rs` 는 acc 테스트 교체와 오프셋 r·ms 절삭 테스트 추가로 39건 → 43건 수준, `tests/skin_loader.rs` 54건 → 67건(stretch 11종 수치, acc 선언 순서, 오프셋 r), `tests/skin_lua.rs` 32건 → 33건, `tests/skin_integration.rs` 는 stretch 미지원 분류 테스트 1건 삭제
- (W1-4) r1-rbms-skin.md §9: U2(타이머 µs 화)와 U10(stretch_rect 반환형)을 완료로 표시. '소비자에 전파되는 시그니처 변경' 목록의 `SkinStateSource::timer/now_ms` 단위와 `stretch_rect` 반환형 항목을 완료로
- (W1-9) r1-rbms-skin.md 로더 절(문서 파싱 관용도): 레퍼런스 기본 스킨 실측 추가 필요. decide.json·result.json·play7.json 은 문자열 자리에 정수 id 를 써서 load_skin 이 거부하고, select.json 은 212행 쉼표 누락으로 load_header 가 Parse 오류를 낸다.


## 웨이브 1A 반영 사항 (2026-10-09)

혼합 합성과 구 번들을 삭제한 뒤의 상태다. 본문의 해당 절은 아래 내용으로 읽는다.

- (리뷰 수정) §(510행, 소비자 쪽 테스트 규모): 'render_tests_skin*.rs 5파일 1,639줄'을 '삭제, 대체 render_tests_document.rs 1파일 약 240줄'로 갱신.
- (W1-1b) r1-rbms-skin.md §7(확장 표)의 '소비자 위치' 열: composition, destination.layer, replace, hotspot, densitygraph, 사설 id 20001~20315, judge.images 의 text 행은 소비자 없음으로 갱신. assets.rs:1002 의 replace_names 사용도 삭제됨. scope 'bundle' 행(skin_select.rs)과 nested 행은 변동 없음.
- (W1-1a) r1-rbms-skin.md §7 473행 부근 `scope: 'bundle'` 행의 앱 사용처 열: apps/rbms-player/src/skin_select.rs 사용처는 이제 없음(SCOPE_BUNDLE import 와 shared_scope 삭제). 남은 사용처는 rbms-skin 내부와 그 테스트뿐.
- (W1-2) r1-rbms-skin.md §0 한 장 요약과 §3.1 표: 마지막 행 '(없음) | composition, densitygraph, result, replace, hotspot | rbms 전용' 이 사라졌습니다. 이제 SkinDef 최상위 필드는 레퍼런스 43개와 일치합니다. 대응 줄(§3.1 line 122, 129)의 '+ rbms 확장 scope' 표기도 삭제하세요.
- (W1-2) r1 §3.2 표: Property, Filepath, Offset 행의 '추가 scope' 비고와 Destination 행의 '추가 layer' 비고를 삭제하세요.
- (W1-2) r1 §7 표: composition, destination.layer, replace, hotspot, scope bundle, densitygraph 행을 '이 크레이트에서 제거 완료(W1-2)' 로 바꾸세요. NamedTrack 에서 layer 필드, loader.rs 에서 HOTSPOT_ACTIONS·keep_known_hotspots·replace_names, resolve.rs 에서 CustomFile.scope 가 사라졌고, 줄 번호 참조(model.rs:359-366 등)가 모두 낡았습니다.
- (W1-2) r1 §8 테스트 표: skin_model.rs 는 23건에서 21건, skin_nested.rs 는 14건에서 8건입니다. 확장 의존 5~6건이 삭제됐고 fixtures/nested 는 확장 키가 정리됐으니 '확장 키 정리' 비고를 완료로 바꾸세요. skin_loader.rs 54건·skin_resolve.rs 30건 그대로입니다(scope 초기화만 삭제).
- (W1-2) r1 §9 U11 행: 이 크레이트 몫(model.rs·loader.rs·resolve.rs·skin_nested.rs·skin_model.rs·fixtures/nested)이 완료됐고 scope·densitygraph 도 같은 단위에서 함께 삭제됐습니다. §9 말미 '소비자에 전파되는 시그니처 변경' 의 NamedTrack.layer, replace_names, def.hotspot, def.composition 항목은 이 크레이트에서 제거 완료로 표시하세요.
- (W1-3) r1-rbms-skin.md §5.3 말미 '권고는 lua52 이다' 와 §5.4 S15 등: 'lua52 전환 완료(W1-3), src/lua.rs 는 무수정으로 5.2 에서 동작, skin_lua 테스트 32개 통과'로 현황을 바꿔야 합니다. §5.1~5.2 의 현재 설정이 lua54 라고 적힌 곳이 있으면 lua52 로 고쳐야 합니다.


조사일 2026-10-09. 읽기 전용 조사이며 세 디렉터리의 파일은 수정하지 않았다. cargo 는 실행하지 않았다(테스트 통과 여부는 미확인, 코드 정독 기준).

표기 규칙
- `R:` 접두사 없이 적은 경로는 `/Users/hyunseokbyun/development/R-BMS` 기준이다.
- `B:` 는 `/Users/hyunseokbyun/development/beatoraja/src/bms/player/beatoraja` 기준이다(이하 "레퍼런스").
- `M:` 은 `/Users/hyunseokbyun/Downloads/ModernChic` 기준이다. ModernChic 은 배정 범위 밖이므로 grep 집계만 했고, 그 수치는 "가벼운 집계"로 표시한다.
- 레퍼런스 저장소는 `origin https://github.com/exch-bms2/beatoraja`, HEAD `8320241d` 이다. 이 판은 `DestinationOption`·`clip_*`·`Practice`·이름 기반 속성 조회·`SkinLuaAccessor` 샌드박스 생성자를 포함한 최신 계열이다.

---

## 0. 한 장 요약

1. `rbms-skin` 은 "JSON/JSON5 문서 → serde 미러(`SkinDef`) → `LoadedSkin`(트랙 조립 완료)" 까지만 한다. Lua 는 "문자열 한 줄 식"만 컴파일하며, 테이블을 돌려주는 스킨 본체 Lua 를 실행하는 경로가 전혀 없다(`src/loader.rs:598-693`, `src/lua.rs:175-190`).
2. 모델 미러(`src/model*.rs`)는 레퍼런스 `JsonSkin.java` 의 필드·기본값을 전부 갖고 있다. 빠진 객체 종류는 없다. 격차는 "값의 형태"에 있다: 함수 값(Lua function)·이름 문자열 속성·실수 좌표·숫자 id 를 받을 자리가 없다.
3. 샌드박스(`src/lua.rs`)는 풀 Lua 스킨과 정반대 방향으로 설계돼 있다. `require`·`dofile`·`load`·`io`·`os`·`print`·`pcall` 주변 전부를 제거했고(55-74행), 식 1회당 20만 명령·20ms, 프레임당 4096회·4ms, 메모리 8MiB 한도를 건다(`src/loader.rs:39-55`). ModernChic 은 `require` 145회·`dofile` 109회·`io.open` 45회·`pcall` 110회·`print` 119회를 쓴다(가벼운 집계).
4. 로더의 조건 분기·include(`src/loader/branch.rs`)는 JSON 전용이다. 레퍼런스의 Lua 경로(`B:skin/lua/LuaSkinLoader.java:169-208`)에는 `if`/`include` 처리가 아예 없다(Lua 스킨은 Lua 로 분기한다). 따라서 branch 계층은 Lua 경로에서 건너뛴다.
5. `dst.rs` 보간은 레퍼런스 `SkinObject.prepareRegion/getRate/prepareColor/prepareAngle` 와 단계별로 일치한다. 확인된 차이는 4건이다: 오프셋 `r` 의 부호, 미선언·미지원 `op` id 의 처리, 타이머 식(함수) 미지원, 타이머 해상도(ms vs µs).
6. `stretch` 11종 중 4종만 구현돼 있고(`src/loader/stretch.rs:61-63`), ModernChic 은 미구현인 3(`FIT_OUTER_TRIMMED`)·5(`FIT_WIDTH_TRIMMED`)를 22곳에서 쓴다(가벼운 집계).
7. rbms 전용 확장(`composition`·`layer`·`replace`·`hotspot`·`scope`·`densitygraph`)은 이 크레이트에서는 모델 필드 6종과 로더 30행 남짓이다. 실제 무게는 소비자(`apps/rbms-player/src/skin_screen.rs`·`skin_select.rs`, `crates/rbms-render/src/skin_render`)에 있다.

---

## 1. 크레이트 개요

| 항목 | 값 | 근거 |
| --- | --- | --- |
| 의존성 | `rbms-model`, `serde`, `serde_json`, `thiserror`, 선택 `json5 1.3.1`, 선택 `mlua 0.12.1 (lua54, vendored)` | `crates/rbms-skin/Cargo.toml:13-19` |
| 실제 잠금 버전 | `mlua 0.12.2`, `mlua-sys 0.13.0`, `lua-src 551.0.2` | `Cargo.lock:1967-1968, 1982-1983, 1864-1865` |
| 피처 | `default = ["json5", "lua"]` | `Cargo.toml:8-11` |
| 안전 정책 | `#![forbid(unsafe_code)]` | `src/lib.rs:12` |
| 소비 크레이트 | `rbms-config`, `rbms-render`, `apps/rbms-player` | `crates/rbms-config/Cargo.toml:12`, `crates/rbms-render/Cargo.toml:11`, `apps/rbms-player/Cargo.toml:22` |
| 규모 | src 약 6,600줄(생성 표 2,054줄 포함), tests 2,702줄 | `wc -l` |

mlua 0.12.2 의 로컬 레지스트리 소스에는 `lua51`·`lua52`·`lua53`·`lua54`·`lua55`·`luajit`·`luau` 피처와 `serde`·`send` 피처가 있고, `lua-src 551.0.2` 는 `lua-5.1.5`·`lua-5.2.4`·`lua-5.3.6`·`lua-5.4.9`·`lua-5.5.1` 소스를 동봉한다(`$CARGO_HOME/registry/src/.../mlua-0.12.2/Cargo.toml` `[features]`, `lua-src-551.0.2/` 디렉터리 목록). Lua 버전 교체는 새 의존성 없이 피처 문자열만 바꾸면 된다.

모듈 구성(`src/lib.rs:14-24`)

| 모듈 | 파일 | 줄 수 | 책임 |
| --- | --- | --- | --- |
| `model` | `model.rs`, `model/objects.rs`, `model/graphs.rs` | 522 + 625 + 306 | `JsonSkin.java` 의 serde 미러 |
| `loader` | `loader.rs`, `loader/branch.rs`, `loader/track.rs`, `loader/stretch.rs` | 693 + 394 + 268 + 128 | 읽기·파싱·분기/include·경로 해석·트랙 조립 |
| `resolve` | `resolve.rs` | 340 | 와일드카드·filemap·루트 가두기·시드 난수 |
| `dst` | `dst.rs` (+ `dst/tests.rs` 517) | 495 | 키프레임 보간·그리기 게이트 |
| `timer` | `timer.rs`, `timer/generated.rs` (+ `timer/tests.rs` 186) | 113 + 327 | 타이머 id 표·`TimerState` |
| `property` | `property/mod.rs`, `property/generated*.rs` | 460 + 1,812 | 속성 id 표·`SkinStateSource`·라우팅 표 |
| `lua` | `lua.rs` | 397 | 한 줄 식 샌드박스 |

---

## 2. 공개 API 와 데이터 흐름

### 2.1 흐름(현재)

1. `loader::load_skin(path, SkinLoadOptions)` (`src/loader.rs:598`)
2. `resolve::contained(root, path)` 로 문서 경로가 루트 안인지 확인 (`:599`, `src/resolve.rs:220-226`)
3. `read_document` — 8MiB 초과면 `TooLarge`, BOM 제거 (`:492-499`)
4. `parse_value` — `serde_json` 엄격 파싱 후 실패 시 `json5` (`:506-528`)
5. `type` 확인 — 없으면 `TypeMissing`, 지원 목록 밖이면 `TypeUnsupported` (`:604-610`)
6. 헤더 선독 — `property`·`filepath` 를 분기 해소 전에 읽어 `enabled_options`·`declared_options`·`custom_files`·`filemap` 구성 (`:612-621`)
7. `branch::transform` — `if`/`value`/`values`/`include` 를 `serde_json::Value` 트리에서 제자리 해소 (`:626-631`, `src/loader/branch.rs:273-362`)
8. `serde_json::from_value::<SkinDef>` (`:633`, `:543-550`)
9. `LuaSandbox::new(root, budget)` — 문서당 인터프리터 1개 (`:653`)
10. `source`·`font` 경로 해석 → `LoadedSkin.sources/fonts: BTreeMap<String, PathBuf>` (`:657-675`)
11. `keep_known_hotspots` (`:677`, `:482-489`)
12. 최상위 `destination` → `track::build_track` → `LoadedSkin.destinations: Vec<NamedTrack>` (`:679-688`)
13. `build_nested` — `note.group/bpm/stop/time`, `judge[].images/numbers`, `songlist` 9배열+`graph` 를 같은 빌더로 조립 (`:442-479`, `:690`)

렌더 쪽 소비(범위 밖이지만 형태 확인용으로 일부만 읽음)

- `rbms_render::skin_render::SkinScreen::build(r, text, &LoadedSkin, &mut dyn SkinAssets)` 가 `skin.sources` 전부를 즉시 디코드·텍스처 등록하고, `skin.fonts` 를 등록한 뒤 `object::build_objects(skin, ...)` 로 `skin.destinations` 를 순회해 객체를 만든다(`crates/rbms-render/src/skin_render/mod.rs:266-296`, `object.rs:539-547`).
- 프레임마다 `SkinFrame { now_ms, timers: &TimerState, state: &dyn SkinStateSource, lua: Option<&dyn SkinExprEval>, mouse, background, extra: FrameExtra }` 를 받아 `rbms_skin::dst::prepare` 로 각 트랙을 해소한다(`mod.rs:214-228`, `mod.rs:393`).
- 식 필드는 `SkinAssets::expression(source) -> Option<LuaExprId>` 로 컴파일하고(`mod.rs:118-122`), 프레임에서 `SkinExprEval::{eval_integer, eval_float, eval_text}` + `LuaDrawEval::eval_draw` 로 평가한다(`mod.rs:128-135`). 앱은 이것을 `LuaSandbox::frame(state)` 가 만든 `LuaFrame` 에 위임한다(`apps/rbms-player/src/skin_screen.rs:92-127`).

### 2.2 타입별 책임과 소비자

| 타입 | 위치 | 책임 | 소비자 |
| --- | --- | --- | --- |
| `SkinError` | `src/lib.rs:31-65` | 스킨 전체를 포기하는 오류 10종(`Read`, `Parse`, `TooLarge`, `NumberRange`, `PathEscape`, `TypeMissing`, `TypeUnsupported`, `Lua`, `LuaBudget`, `LuaUnavailable`) | 앱(폴백 판단) |
| `SkinLoadOptions<'a>` | `src/loader.rs:168-199` | `root`, `user`, `rng_seed`, `lua_budget`, `mode`, `max_document_bytes`, `known_option: fn(i32)->bool` | `skin_select.rs:695, 749`, `assets.rs:994`, 테스트 |
| `SkinUserConfig` | `src/loader.rs:140-151` | 플레이어 선택: `path`, `properties: BTreeMap<String /*행 이름*/, i32>`, `filepaths: BTreeMap<String, String>`, `offsets: BTreeMap<i32 /*offset id*/, SkinOffset>`. `OffsetSource` 구현 | `rbms-config/src/schema.rs:7-8, 237-239` |
| `Budget` | `src/loader.rs:76-98` | Lua 한도 5종 | `lua.rs`, 앱 |
| `SkinHeader` | `src/loader.rs:248-260` | 설정 화면용 요약: `skin_type`, `name`, `author`, `width`, `height`, `parser`, `properties`, `offsets`, `custom_files` | `skin_select.rs` |
| `SkinDef` 와 하위 레코드 | `src/model.rs:402-464` | 문서 미러. 해석은 하지 않는다 | 렌더(`object.rs`, `graphs.rs`, `notes.rs` 등이 `skin.def.*` 를 id 로 조회) |
| `PropertyRef` | `src/model.rs:49-54` | `Id(i32)` 또는 `Expr(String)` | 로더·렌더 `ValueSource::new` (`object.rs:267-272`) |
| `LoadedSkin` | `src/loader.rs:313-343` | `def`, `path`, `root`, `parser`, `mode`, `custom_files`, `enabled_options`, `declared_options`, `sources`, `fonts`, `destinations`, `nested`, `warnings` + 비공개 `replace`, `resolver`, `known_option`, `lua` | 렌더 `SkinScreen::build`, 앱 `skin_screen.rs` |
| `NamedTrack` | `src/loader.rs:263-268` | `id: String`, `layer: SkinLayer`, `track: DestinationTrack` | 렌더 |
| `NestedTracks` / `JudgeTracks` / `SongListTracks` | `src/loader.rs:271-310` | 반복 객체 안쪽 destination 을 슬롯 순서대로 보관 | 렌더 `notes.rs`, `judge.rs`, `songlist.rs` |
| `DestinationTrack` | `src/dst.rs:192-214` | `timer: Option<TimerId>`, `loop_ms`, `blend`, `filter`, `center`, `offsets`, `relative`, `frames`, `draw_conditions`, `mouse_rect`, `stretch` | 렌더 `draw.rs` |
| `Keyframe` / `Resolved` | `src/dst.rs:150-159, 249-255` | 채워진 키프레임 / 한 프레임의 사각형·클립·색·각도 | 렌더 |
| `DrawCondition` | `src/dst.rs:181-187` | `Option(i32)` 또는 `Lua(LuaExprId)` | `dst::prepare` |
| `OffsetSource` / `DrawStateSource` / `SkinStateSource` | `src/dst.rs:76-90`, `src/property/mod.rs:134-154` | 게임 상태 읽기 계약: `offset(id)`, `boolean(id)`, `integer(id)`, `float(id)`, `string(id) -> &str`, `timer(id) -> Option<i64>`, `now_ms()` | 렌더 `state.rs` 의 화면별 구현체 |
| `LuaDrawEval` / `LuaExprId` | `src/dst.rs:97-104` | 보간기가 Lua 없이도 컴파일되도록 하는 얇은 계약 | 렌더·앱 |
| `TimerId` / `TimerState` / `timer_id::*` | `src/timer.rs:22, 56-112`, `src/timer/generated.rs` | 타이머 id 151종과 "켜진 시각(ms)" 저장소 | 렌더 `screen.rs` 타이머 드라이버, 앱 |
| `property::generated::*` | `src/property/generated/*.rs` | `OPTION_*` 287, `NUMBER_*` 273, `RATE_*` 30, `SLIDER_*` 9, `BARGRAPH_*` 22, `FLOAT_*` 40, `STRING_*` 43, `BUTTON_*` 74, `OFFSET_*` 16, `VALUE_*` 16, `IMAGE_*` 5, `EVENT_*` 2 = 817 | 렌더 `state.rs`(glob import), 앱 |
| `PropertyKind` / `MAPPINGS` / `StateSource` / `UnmappedLog` | `src/property/mod.rs:30-41, 279-395, 202-223, 416-460` | id 대역 → rbms 하위 시스템 라우팅 표(문서화·통계용) | 테스트, 디버그 뷰 |
| `LuaSandbox` / `LuaFrame` | `src/lua.rs:86-96, 291-296` | 식 컴파일·평가·프레임 예산 | 앱 `skin_screen.rs:30, 49, 93-99` |
| `StretchKind` / `stretch_rect` / `Filtering` / `filtering_for` | `src/loader/stretch.rs` | `stretch`·`filter` 산술 | 렌더 `draw.rs:14` |
| `FileResolver` / `CustomFile` / `Draw` | `src/resolve.rs:233-283, 97-112, 55-90` | 패턴 → 파일, 후보 스캔, SplitMix64 | 로더, 설정 화면 |

화면 타입 표(`src/loader.rs:202-242`): 지원은 `0`(7K)·`1`(5K)·`2`(14K)·`3`(10K)·`4`(9K)·`5`(MUSIC_SELECT)·`6`(DECIDE)·`7`(RESULT)·`8`(KEY_CONFIG) 이다. `16`(24K)은 표에 있으나 `rbms_model::Mode::ALL` 에 24K 가 없어 거부된다(`crates/rbms-model/src/mode.rs:37`). 레퍼런스의 `SKIN_SELECT(9)`·`COURSE_RESULT(15)`·`PLAY_24KEYS_DOUBLE(17)`·BATTLE 계열(12·13·14·18)은 전부 `TypeUnsupported` 다(`B:skin/SkinType.java:12-30`). ModernChic 의 `skinselect.luaskin`(type 9, `M:SkinSelect/lua/require/header.lua:8`)은 현재 로더가 거부한다. `course.luaskin` 은 결과 헤더가 `type` 을 인자로 받으므로(`M:Result/lua/require/header.lua:11`) 실제 값은 미확인이나, 레퍼런스의 코스 결과 타입은 15 이고 이 역시 거부 대상이다.

문서 탐색 확장자는 앱이 `["json", "json5"]` 로 고정한다(`apps/rbms-player/src/skin_select.rs:39`). `.luaskin` 은 목록에 나타나지도 않는다.

---

## 3. 모델 미러와 `JsonSkin.java` 전수 대조

### 3.1 최상위 `Skin` (`B:skin/json/JsonSkin.java:7-57` 대 `src/model.rs:402-464`)

| 레퍼런스 필드 | 타입·기본값 | rbms 필드 | 상태 |
| --- | --- | --- | --- |
| `type` | `int = -1` | `skin_type: i32 = -1` | 일치 |
| `name`, `author` | `String`(null) | `String`("") | 일치(헤더가 null→"" 로 바꿈, `B:skin/json/JSONSkinLoader.java:121-122`) |
| `w`, `h` | `1280`, `720` | `w`, `h` 동일 | 일치 |
| `fadeout`, `input`, `scene`, `close`, `loadend`, `playstart` | `int = 0` | 동일 | 일치(값 보관만, 이 크레이트에서 해석 없음) |
| `judgetimer` | `1` | `1` | 일치 |
| `finishmargin` | `0` | `0` | 일치 |
| `category[]`, `property[]`, `filepath[]`, `offset[]` | | `Vec<Category/PropertyDef/Filepath/OffsetDef>` | 일치 + rbms 확장 `scope` |
| `source[]`, `font[]` | | `Vec<Source/FontDef>` | 일치(단 `FontFallback` 의 문자열 단축형은 미지원, 3.3) |
| `image[]`, `imageset[]`, `value[]`, `floatvalue[]`, `text[]`, `slider[]`, `graph[]` | | 동일 이름 `Vec` | 일치 |
| `gaugegraph[]`, `judgegraph[]`, `bpmgraph[]`, `hiterrorvisualizer[]`, `timingvisualizer[]`, `timingdistributiongraph[]` | | 동일 | 일치 |
| `note`, `gauge`, `bga`, `skinpreview`, `practice`, `songlist`, `skinSelect` | 단일 객체(null) | `Option<...>` | 일치 |
| `hiddenCover[]`, `liftCover[]`, `judge[]`, `pmchara[]`, `customEvents[]`, `customTimers[]` | | `Vec` | 일치 |
| `destination[]` | null 기본 | `Vec<Destination>` | 일치 |
| (없음) | | `composition`, `densitygraph`, `result`, `replace`, `hotspot` | rbms 전용(7장) |

결론: 레퍼런스가 선언한 최상위 필드 43개가 전부 있다. 빠진 객체 종류는 없다.

### 3.2 레코드별 대조

| 레퍼런스 클래스 (`JsonSkin.java` 행) | rbms 레코드 (행) | 필드 누락 | 기본값 차이·비고 |
| --- | --- | --- | --- |
| `Property` 59-64, `PropertyItem` 66-69 | `PropertyDef` `model.rs:123-134`, `PropertyItem` 137-142 | 없음 | rbms 추가 `scope` |
| `Filepath` 71-76 | `Filepath` 145-154 | 없음 | rbms 추가 `scope` |
| `Offset` 78-88 | `OffsetDef` 157-171 | 없음 | rbms 추가 `scope` |
| `Category` 90-93 | `Category` 115-120 | 없음 | |
| `Source` 95-98 | `Source` 174-179 | 없음 | `id` 가 `String` 고정(3.3) |
| `Font` 100-105, `FontFallback` 107-110 | `FontDef` 182-190, `FontFallback` 193-199 | 없음 | 문자열 단축형 미지원(3.3) |
| `Image` 112-127 | `ImageDef` `objects.rs:12-51` | 없음 | `divx/divy=1`, `click=0` 일치 |
| `ImageSet` 129-136 | `ImageSet` 54-64 | 없음 | |
| `Value` 138-157 | `ValueDef` 67-114 | 없음 | `offset: Value[]`(null) → `Vec`(빈) |
| `FloatValue` 159-181 | `FloatValueDef` 117-171 | 없음 | `gain=1.0` 일치 |
| `Text` 183-201 | `TextDef` 174-226 | 없음 | `overflow=0`, 색 `"ffffff00"` 일치(`B:skin/SkinText.java:32`) |
| `Slider` 203-223 | `SliderDef` 229-279 | 없음 | `changeable=true` 일치 |
| `Graph` 225-242 | `GraphDef` 282-326 | 없음 | `angle=1` 일치(`objects.rs:318`) |
| `GaugeGraph` 244-261 | `GaugeGraph` `graphs.rs:11-67` | 없음 | 색 14종 일치. `color` null → 빈 `Vec` |
| `JudgeGraph` 263-271 | `JudgeGraph` 73-94 | 없음 | `delay=500` 일치 |
| `BPMGraph` 273-283 | `BpmGraph` 100-135 | 없음 | `lineWidth=2` 일치 |
| `HitErrorVisualizer` 285-305 | 144-207 | 없음 | 19필드 일치 |
| `TimingVisualizer` 307-321 | 210-256 | 없음 | 일치 |
| `TimingDistributionGraph` 323-337 | 259-306 | 없음 | 일치 |
| `NoteSet` 339-367 | `NoteSet` `objects.rs:332-400` | 없음 | `dst2: int = MIN_VALUE` → `Option<i32>`, `expansionrate={100,100}` 일치 |
| `Gauge` 369-378 | `GaugeDef` 403-421 | 없음 | `parts=50`, `range=3`, `cycle=33`, `endtime=500` 일치 |
| `HiddenCover` 380-393, `LiftCover` 395-408 | 424-499 | 없음 | `disapearLine=-1`, `isDisapearLineLinkLift` true/false 일치 |
| `BGA` 410-412, `SkinPreview` 414-416, `Practice` 424-427 | 502-531 | 없음 | `visibleItems=10` 일치 |
| `Judge` 429-435 | `JudgeDef` 534-542 | 없음 | |
| `SongList` 437-451 | `SongList` 545-561 | 없음 | `graph` 단일 |
| `Destination` 453-481 | `Destination` `model.rs:294-334` | 없음 | `stretch=-1` 일치. rbms 추가 `layer` |
| `DestinationOption` 484-498 | 231-291 | 없음 | 숫자·문자열·객체 3형 수용 |
| `Rect` 500-505 | `RectDef` 221-228 | 없음 | |
| `Animation` 507-529 | `Animation` 338-356 | 없음 | `MIN_VALUE` 센티넬 → `Option` |
| `PMchara` 531-537 | `PmChara` `objects.rs:564-579` | 없음 | `type = MIN_VALUE` → `Option<i32>` |
| `SkinConfigurationProperty` 539-544 | 582-599 | 없음 | `customPropertyCount=-1` 일치 |
| `CustomEvent` 546-551, `CustomTimer` 553-556 | `model.rs:202-218` | 없음 | 보관만 하고 로더·렌더 어디서도 쓰지 않음(사양 §12 비목표, `docs/plan/2026-09-17-skin-system-completion.md:293`) |

### 3.3 미러가 받지 못하는 "값의 형태" (풀 Lua 스킨의 실제 격차)

레퍼런스의 Lua 경로는 JSON 직렬화기를 거치지 않고 `fromLuaValue` 로 Lua 값을 직접 필드에 넣는다(`B:skin/lua/LuaSkinLoader.java:169-208`). 규칙은 다음과 같다.

| 대상 타입 | 변환 규칙 | 근거 | rbms 현재 |
| --- | --- | --- | --- |
| `int`/`Integer` | `LuaValue::toint` — 실수는 0 방향 절삭, 숫자 문자열은 변환, 그 외는 0 | `LuaSkinLoader.java:101-102` | `i32` serde. `serde_json::Value` 의 실수는 `i32` 로 역직렬화 실패한다. `PropertyRef` 만 `visit_f64` 절삭을 구현(`model.rs:95-101`) |
| `float`/`Float` | `LuaValue::tofloat` | `:103-104` | `f32` — 정수도 수용 |
| `boolean` | `LuaValue::toboolean` — nil/false 만 거짓. 숫자 0 은 참 | `:99-100` | `bool` — 숫자를 받으면 실패 |
| `String` | `LuaValue::tojstring` — 숫자 `1` 은 `"1"` 이 됨 | `:105` | `String` — 숫자를 받으면 실패. `id`, `src`, `font`, `note[]`, `images[]`, `nodes[]` 가 전부 해당 |
| 배열 `T[]` | 테이블이면 `table.keys()` 순서대로 원소 변환(순차 키가 아니어도 됨), 테이블이 아니면 빈 배열 | `:172-184` | `Vec<T>` — 객체(맵)면 실패 |
| 객체 클래스 | 테이블의 키 중 필드명과 같은 것만 대입, 모르는 키 무시. 테이블이 아니면 기본 인스턴스 | `:186-207` | `#[serde(default)]` 구조체 — 모르는 키 무시는 일치 |
| `BooleanProperty` | 함수 → 프레임마다 호출(`toboolean`, 예외 시 false). 숫자 → `BooleanPropertyFactory.getBooleanProperty(int)`. 문자열 → 이름 조회 후 없으면 `return <문자열>` 로 컴파일 | `:118-120, 153-166`, `B:skin/lua/SkinLuaAccessor.java:601-628` | `PropertyRef::Id/Expr` — 함수 값 자리 없음, 이름 조회 없음 |
| `IntegerProperty` | 함수(`toint`, 예외 시 0) / id / 이름 / 식 | `:121-123`, `SkinLuaAccessor.java:630-652` | 동일 격차 |
| `FloatProperty` | 함수(`tofloat`, 예외 시 0) / id 는 `getRateProperty`(RATE 공간만) / 이름 / 식 | `:124-126`, `SkinLuaAccessor.java:654-676` | 동일 격차. rbms `float(id)` 는 RATE·FLOAT 를 한 접근자로 합침(`property/mod.rs:52-60`) |
| `StringProperty` | 함수(`tojstring`, 예외 시 "") / id / 이름 / 식 | `:127-129`, `SkinLuaAccessor.java:678-700` | 동일 격차 |
| `TimerProperty` | 함수 → 항상 "타이머 함수"(µs 시작 시각 또는 `Long.MIN_VALUE` 반환, 프레임마다 호출). 숫자 → `getTimerProperty(id)`(id<0 이면 null). 문자열 → `return <식>` 을 한 번 시험 호출해 결과가 함수면 그 함수를, 아니면 식 자체를 타이머 함수로 | `:130-132`, `SkinLuaAccessor.java:712-747`, `B:skin/property/TimerPropertyFactory.java:6-8` | 식이면 경고 후 프레임 시계로 폴백(`src/loader/track.rs:231-242`). 함수 자리 없음 |
| `FloatWriter` | 함수(값 1개 받음) / id `getRateWriter` / 이름 | `:133-135`, `SkinLuaAccessor.java:790-811` | `SliderDef.event: Option<PropertyRef>` 보관만 |
| `StringWriter` | 함수 / 이름 | `:136-138` | `TextDef.event` 보관만 |
| `Event` | 함수(인자 0·1·2개, `narg()` 로 구분) / id / 이름 | `:139-141`, `SkinLuaAccessor.java:759-788` | `ImageDef.act`, `ImageSet.act`, `CustomEvent.action` 보관만 |
| `DestinationOption` | 숫자 → id. 그 외(함수·문자열) → `BooleanProperty` | `:142-149` | 숫자·문자열·`{id, property}` 수용, 함수 불가 |
| `FontFallback` | 문자열이면 `path`, 테이블이면 `path`/`type` | `:106-117` | 구조체만(문자열이면 실패) |

JSON 경로의 이름 조회도 rbms 에 없다. 레퍼런스 `LuaScriptSerializer.read` 는 문자열을 먼저 `namePropertyLoader`(예: `BooleanPropertyFactory.getBooleanProperty(String)`, `!` 접두사 부정 포함)로 찾고, 없을 때만 Lua 식으로 컴파일한다(`B:skin/json/JsonSkinSerializer.java:356-369`, `B:skin/property/BooleanPropertyFactory.java:70-84`). rbms 는 문자열을 무조건 Lua 식으로 컴파일한다(`src/loader/track.rs:221`, `crates/rbms-render/src/skin_render/object.rs:270`).

가벼운 집계로 본 ModernChic 의 함수 값 필드: `draw = function` 329곳, `value = function` 128곳, `act = function` 35곳, `timer = function` 17곳, `offset = function` 7곳(이것은 `Value.offset` 이 아니라 헤더 테이블 안의 다른 용도일 수 있음, 미확인), `timer_util.timer_observe_boolean` 148곳.

결론: `SkinDef` 를 serde 로 채우는 방식은 Lua 테이블을 그대로 받을 수 없다. (a) Lua 값 → `SkinDef` 전용 변환기를 새로 쓰고, (b) `PropertyRef` 에 "등록된 Lua 함수 핸들" 변형을 추가해야 한다.

---

## 4. 로더가 풀 Lua 스킨과 충돌하는 지점

### 4.1 레퍼런스의 Lua 로드 절차(재현 대상)

`B:skin/SkinLoader.java:67-71` 이 확장자 `.luaskin` 을 `LuaSkinLoader(state, config)` 로 보낸다. 이 생성자는 `new SkinLuaAccessor(false)` 를 쓴다(`LuaSkinLoader.java:45-47`). 즉 실행 환경은 "샌드박스 생성자"가 아니라 "표준 전역 + 제한 io" 쪽이다.

| 단계 | 내용 | 근거 |
| --- | --- | --- |
| 0 | `JSONSkinLoader(state, c, lua)` 생성자가 `lua.exportMainStateAccessor(state)`, `lua.exportUtilities(state)` 를 먼저 호출 → `package.loaded` 에 `main_state`, `timer_util`, `event_util` 등록 | `B:skin/json/JSONSkinLoader.java:78-85`, `SkinLuaAccessor.java:880-914` |
| 1 | 헤더 패스: `lua.setDirectory(p.getParent())` → `package.path` 에 `<스킨폴더>/?.lua` 추가, 제한 io 의 루트를 스킨 폴더로 지정 | `LuaSkinLoader.java:53`, `SkinLuaAccessor.java:855-871` |
| 2 | 헤더 패스: `lua.execFile(p)` — 이때 전역 `skin_config` 는 nil. 반환 테이블을 `JsonSkin.Skin` 으로 변환해 헤더 구성 | `LuaSkinLoader.java:54-56`, `SkinLuaAccessor.java:840-853` |
| 3 | `header.setSkinConfigProperty(property)` — 옵션·파일·오프셋 선택 반영 | `LuaSkinLoader.java:75`, `B:skin/SkinHeader.java:152-221` |
| 4 | filemap 구성: 선택된 파일명이 있는 슬롯만 `키 = <스킨폴더>/<path>` 로 등록 | `LuaSkinLoader.java:78-83` |
| 5 | `lua.exportSkinProperty(header, property, get_path)` — 전역 `skin_config` 설정 | `LuaSkinLoader.java:85-87`, `SkinLuaAccessor.java:925-980` |
| 6 | 본체 패스: 같은 `Globals` 로 `lua.execFile(p)` 재실행 → 변환 → `loadJsonSkin` | `LuaSkinLoader.java:88-90` |

단계 2 와 6 이 같은 인터프리터를 쓰므로 `package.loaded` 가 유지된다. `.luaskin` 의 `local t = require("play7_hw")` 는 헤더 패스에서 모듈을 한 번만 실행하고, 본체 패스에서는 캐시된 `t` 의 `t.main()` 만 부른다(`M:play7_hw.luaskin` 전문 6줄). 따라서 모듈 최상위 코드는 `skin_config == nil` 상태에서 한 번만 돈다. 헤더용과 본체용 인터프리터를 따로 만들면 모듈 최상위가 두 번 실행돼 의미가 달라진다.

`skin_config` 테이블(`SkinLuaAccessor.java:931-980`)

| 키 | 값 |
| --- | --- |
| `file_path` | `{ [슬롯 이름] = 저장된 선택 문자열 }` (`property.getFile()` 그대로, 선택이 없는 슬롯은 키가 없음) |
| `get_path(path)` | `getPath(<스킨폴더> + "/" + path, filemap).getPath()` — filemap 치환 또는 와일드카드 무작위 선택이 적용된 경로 문자열 |
| `option` | `{ [행 이름] = 선택된 op 값 }` |
| `enabled_options` | 선택된 op 값의 배열(1부터) |
| `offset` | `{ [오프셋 이름] = { x, y, w, h, r, a } }` (저장값이 없으면 0) |

플레이 화면 헤더에는 오프셋 4개가 자동으로 붙는다: `"All offset(%)"`(`OFFSET_ALL`), `"Notes offset"`(`OFFSET_NOTES_1P`), `"Judge offset"`(`OFFSET_JUDGE_1P`), `"Judge Detail offset"`(`OFFSET_JUDGEDETAIL_1P`) (`JSONSkinLoader.java:167-190`). rbms 헤더에는 이 자동 추가가 없다(`src/loader.rs:571-591`).

### 4.2 충돌 지점 표

| 번호 | 현재 동작 | 근거 | 풀 Lua 스킨과의 충돌 | 필요한 변경 |
| --- | --- | --- | --- | --- |
| L1 | 진입점이 JSON 텍스트 파싱뿐 | `src/loader.rs:600-601` | `.luaskin` 은 Lua 청크다 | `load_skin`/`load_header` 를 확장자로 분기하고 Lua 경로를 추가 |
| L2 | 헤더를 "분기 해소 전 원시 트리"에서 읽음 | `:556-558, 612-617` | Lua 는 실행 결과 테이블에서 읽어야 함 | 헤더 패스 실행 결과를 `SkinDef` 변환기로 읽는 경로 |
| L3 | 문서 크기 한도 8MiB, include 깊이 8, 총 1,024회 | `:36`, `branch.rs:43, 51` | Lua 는 `require`/`dofile` 로 수백 파일을 읽음. include 한도는 무관해지고 파일당 크기 한도만 의미가 있음 | Lua 파일 읽기에는 파일당 한도만 적용 |
| L4 | `branch::transform` 이 `if`/`include` 를 해소 | `:631` | Lua 경로에는 해당 문법이 없음(`LuaSkinLoader.java:169-208` 에 분기 처리 부재) | Lua 경로에서 건너뜀. 단 Lua 테이블에 `if`·`include` 키가 있어도 해석하지 않아야 레퍼런스와 같음 |
| L5 | 모든 경로를 `contained(root, ...)` 로 루트 안에 가둠 | `resolve.rs:220-226` | 그대로 유효. `require`·`dofile`·`io.open`·`get_path` 결과에도 같은 검사를 적용하면 됨 | 재사용 |
| L6 | 파일 슬롯 기본값: 선택이 없으면 `def` 문자열을 그대로 filemap 값으로 사용 | `resolve.rs:330-334` | ModernChic 의 `def` 는 `"#default"`·`"sample"` 처럼 확장자가 없다(`M:Result/lua/require/property.lua:267-275`, `M:Decide/lua/require/property.lua:124-125`). `apply_filemap` 은 `패턴[..별] + 값 + 패턴[키길이..]` 이라 결과가 `.../#default`(확장자 없음)가 되어 파일이 없다(`resolve.rs:146-150`) | `def` 를 후보 파일의 "전체 이름 또는 확장자 뗀 이름"과 대소문자 무시로 대조해 실제 파일명으로 바꾼다. 레퍼런스 설정 화면이 이렇게 한다(`B:config/SkinConfiguration.java:327-335`, `B:launcher/SkinConfigurationView.java:404-412`) |
| L7 | 선택도 `def` 도 없으면 후보 중 하나를 시드 난수로 골라 filemap 에 고정 | `resolve.rs:331-332` | 레퍼런스는 이 경우 filemap 에 넣지 않고 `getPath` 호출마다 `Math.random()` 으로 고른다(`B:skin/SkinLoader.java:107-128`). 호출마다 다른 파일이 나올 수 있다 | 로드당 1회 고정이 재현성 면에서 낫다. 의도적 차이로 기록 |
| L8 | 와일드카드 후보를 정렬 | `resolve.rs:162-172` | 레퍼런스는 파일시스템 순서 | 의도적 차이(시드 재현) |
| L9 | filemap 키 충돌 시 최장 일치 | `resolve.rs:146-150` | 레퍼런스는 해시 순서 첫 일치 | 의도적 차이 |
| L10 | 옵션 선택: 플레이어 값이 행에 있으면 채택, 아니면 `def`, 아니면 첫 항목 | `branch.rs:134-144` | 레퍼런스는 저장값이 `OPTION_RANDOM_VALUE(-1)` 이면 무작위 항목을 고르고, 그 외에는 저장값을 검증 없이 `op` 로 쓰되 항목에 없으면 `selectedIndex=-1` → `getSelectedOption()` 이 `-1` 반환(`SkinHeader.java:153-171, 302-304`) | 무작위(-1) 선택 지원 추가. 항목에 없는 저장값은 rbms 쪽(기본값 폴백)이 더 안전하므로 유지 여부 결정 필요 |
| L11 | 헤더에 자동 오프셋 4종 없음 | `:571-591` | 4.1 표 참조 | 플레이 타입 헤더에 4종 추가 |
| L12 | `sources`/`fonts` 를 로드 시 전부 경로 해석 | `:657-675` | 렌더가 `sources` 전부를 즉시 디코드한다(`skin_render/mod.rs:272-282`). 레퍼런스는 객체가 참조할 때만 읽는다(`JSONSkinLoader.java:481-520`). ModernChic 은 mp4 를 `source` 로 선언한다(패턴 `Play/parts/common/BGA/movie/*.mp4`, `Select/bg/movie/*.mp4`, 가벼운 집계) | 참조된 source 만 디코드, 동영상 source 는 별도 종류로 분류 |
| L13 | `LoadedSkin.sources` 키가 `String` | `:329` | Lua 에서 `id` 가 숫자면 `tojstring` 결과와 같아야 한다. Lua 5.4 에서 실수 `1.0` 은 `"1.0"` 이 되어 레퍼런스(`"1"`)와 달라진다(5.1 절) | 변환기에서 정수값 실수는 정수 문자열로 |
| L14 | 화면 타입 제한 | `:240-242` | type 9·15 거부 | 타입 표 확장(앱 화면 유무와 별개로 로드는 가능하게 할지 결정 필요) |
| L15 | `mode` 를 옵션으로 받음 | `:179` | Lua 는 문서 `type` 이 모드를 결정 | 영향 없음 |
| L16 | 문서 좌표는 `SkinDef.w/h` 그대로 | `skin_render/mod.rs:294` | 레퍼런스는 `w,h` 가 알려진 `Resolution` 과 일치할 때만 그 해상도를 쓰고 아니면 HD(1280x720)로 본다(`JSONSkinLoader.java:263-269`). 1920x1080 은 일치 | 영향 없음(임의 해상도 스킨에서만 차이) |
| L17 | `destination.id` 가 음수 정수 문자열이면? 처리 없음 | `:679-688` (id 문자열 그대로 보관) | 레퍼런스는 `Integer.parseInt(dst.id) < 0` 이면 `SkinImage(-id)`(참조 이미지, 예: `IMAGE_STAGEFILE`)를 만든다(`JSONSkinLoader.java:313-322`) | 렌더 쪽 과제. 로더는 id 를 그대로 넘기면 됨 |
| L18 | `LoadedSkin` 이 `mlua::Lua` 를 소유 → `!Send` | `:341-342`, `Cargo.toml:19`(`send` 피처 없음) | 풀 Lua 실행은 수백 ms 가 걸릴 수 있어 워커 스레드 로드가 필요할 수 있다. 현재 워커는 이미지 디코드만 한다(`apps/rbms-player/src/skin_screen.rs:432`) | `send` 피처 채택 여부 결정, 또는 메인 스레드 동기 로드 유지 |

---

## 5. `lua.rs` 샌드박스 — 구조와 풀 Lua 실행을 위한 변경

### 5.1 현재 구조

| 항목 | 현재 값 | 근거 |
| --- | --- | --- |
| Lua 버전 | 5.4 (vendored) | `Cargo.toml:19` |
| 적재 라이브러리 | `StdLib::MATH \| STRING \| TABLE` 만(기본 함수는 mlua 가 항상 적재) | `src/lua.rs:110` |
| 제거 전역 18종 | `dofile`, `loadfile`, `load`, `loadstring`, `require`, `collectgarbage`, `rawset`, `rawget`, `rawequal`, `rawlen`, `setmetatable`, `getmetatable`, `newproxy`, `print`, `io`, `os`, `package`, `debug` | `:55-74, 113-116` |
| 제거 `string` 함수 5종 | `dump`, `find`, `gmatch`, `gsub`, `match` | `:83, 117-120` |
| 난수 | `math.randomseed(0x5eed5eed)` 고정 | `:37, 121` |
| 메모리 한도 | `lua.set_memory_limit(8MiB)` | `:111`, `src/loader.rs:42` |
| 훅 | 1,000 명령마다 발화. 남은 명령 수 < 1,000 이거나 경과 > `max_call_micros` 면 `RuntimeError` | `:33, 130-139` |
| 식 1회 한도 | 200,000 명령, 20,000µs | `src/loader.rs:39, 52` |
| 프레임 한도 | 4,096회, 4,000µs. 넘으면 나머지는 평가하지 않고 기본값 | `src/loader.rs:45, 55`, `src/lua.rs:310-325` |
| 컴파일 | `return <src>` 로 시도, 실패하면 원문을 청크로. `ChunkMode::Text` 강제. 동일 소스는 핸들 재사용 | `:175-190` |
| 호출 | 매 호출마다 `lua.scope` 안에서 새 테이블 + 스코프 함수 6개를 만들어 전역 `skin` 에 넣고, 끝나면 `skin = nil` | `:193-223` |
| 노출 API | `skin.boolean(id)`, `skin.number(id)`, `skin.float(id)`, `skin.text(id)`, `skin.timer(id)`(꺼져 있으면 nil, ms), `skin.time()`(ms) | `:204-209` |
| 결과 변환 | bool: nil/false 만 거짓. int: `trunc`. 문자열→숫자 강제 변환 허용. 불리언→0/1 | `:231-257, 379-387` |
| 실패 처리 | 프로세스당 1회만 경고(`WarnOnce`), 그 객체는 기본값(그리기 조건이면 숨김) | `:331-348`, `src/dst.rs:259-272` |

`skin.*` 6함수는 rbms 고유 이름이다. 레퍼런스에는 `skin` 전역이 없고, JSON 스킨의 식은 `exportMainStateAccessor` 가 `isGlobal == true` 일 때 전역에 직접 넣은 `option`, `number`, `float_number`, `text`, `timer`, `time` 등을 부른다(`SkinLuaAccessor.java:880-889`, `JSONSkinLoader.java:74-76` 의 `new SkinLuaAccessor(true)`). 즉 현재 샌드박스는 레퍼런스 JSON 스킨의 식과도 이름이 호환되지 않는다.

### 5.2 레퍼런스 실행 환경(`SkinLuaAccessor(false)`)

| 항목 | 내용 | 근거 |
| --- | --- | --- |
| Lua 구현 | LuaJ(Lua 5.2 계열). 숫자는 정수/실수 내부 구분이 있으나 `tojstring` 은 정수값 실수를 정수로 출력 | `SkinLuaAccessor.java:13-16` (동작 세부는 LuaJ 구현, 본 조사에서 소스 미확인) |
| 라이브러리 | `JseBaseLib`, `PackageLib`, `Bit32Lib`, `TableLib`, `JseStringLib`, `CoroutineLib`, `JseMathLib`, 제한 `IoLib`, 안전 `OsLib` | `:68-82` |
| 제거 | `luajava`, `debug` 를 nil 로 → 직후 `LegacySkinLuaApi.install` 이 제한된 `luajava`·`debug` 를 다시 넣음. `package.loadlib` nil, `cpath` "", `searchers[3]`·`[4]` 제거 | `:84-93, 131-136`, `B:skin/lua/LegacySkinLuaApi.java:51-64` |
| `os` | `execute`, `exit`, `getenv`, `remove`, `rename`, `tmpname` 제거. `time`, `date`, `clock`, `difftime` 유지 | `:138-150` |
| `io` | 스킨 폴더 안에서 읽기·쓰기·추가 모두 허용. 폴더 밖은 `"Lua skin file access denied"`. `popen` 금지. 쓰기 시 부모 디렉터리 자동 생성. stdin/stdout/stderr 는 메모리 파일 | `:317-399` |
| `require` | `package.path` = 기존 값 + `;<스킨폴더>/?.lua` | `:869` |
| `dofile`/`loadfile`/`load`/`pcall`/`print`/`setmetatable` 등 | 기본 라이브러리 그대로 | `:70` |
| `luajava` 파사드 | `bindClass` 는 `com.badlogic.gdx.Gdx`, `.Input`, `.controllers.Controllers`, `.controllers.Controller`, `java.io.File` 만 허용. `new(File, path)` 는 `mkdir`/`listFiles` 등 제한 메서드. `newInstance` 는 `java.net.URL`, `InputStreamReader`, `BufferedReader` | `LegacySkinLuaApi.java:66-116, 118-140` (140행 이후 미독) |
| `main_state` 모듈 | `option`, `number`, `float_number`, `text`, `offset`, `timer`, `timer_off_value`, `timer_is_on`, `timer_is_off`, `timer_elapsed`, `timer_elapsed_ms`, `timer_elapsed_seconds`, `time`, `set_timer`, `event_exec`, `event_index`, `key_pressed`, `numbers`, `screen_width`, `screen_height`, `rate`, `exscore`, `rate_best`, `exscore_best`, `rate_rival`, `exscore_rival`, `volume_sys`, `volume_key`, `volume_bg`, `gauge`, `gauge_type`, `set_volume_sys`, `set_volume_key`, `set_volume_bg`, `judge` + 파일 8종(`file_exists`, `file_mkdir`, `file_list`, `file_read_lines`, `file_write`, `file_append`, `file_clear`, `file_count_lines`) + `http_get`, `http_get_lines` + 오디오 5종(`audio_play`, `audio_loop`, `audio_preload`, `audio_stop`, `audio_dispose`) | `B:skin/lua/MainStatePropertyLuaApiExporter.java:33-93`, `SkinFileLuaApiExporter.java:23-70`, `SkinHttpLuaApiExporter.java:32-33`, `SkinAudioLuaApiExporter.java:20-48` |
| `timer_util` 모듈 | `now_timer`, `is_timer_on`, `is_timer_off`, `timer_function`, `timer_observe_boolean`, `new_passive_timer` | `B:skin/lua/TimerUtility.java:19-24` |
| `event_util` 모듈 | `event_observe_turn_true`, `event_observe_timer`, `event_observe_timer_on`, `event_observe_timer_off`, `event_min_interval` | `B:skin/lua/EventUtility.java:18-22` |

`main_state` 핵심 함수의 정확한 의미(`MainStatePropertyLuaApiExporter.java`)

| 함수 | 의미 | 행 |
| --- | --- | --- |
| `option(id \| name)` | `BooleanProperty` 조회 후 `get(state)`. 없으면 false | 159-165 |
| `number(id \| name)` | `IntegerProperty`. 없으면 0 | 154-172 |
| `float_number(id \| name)` | `getFloatProperty`(FLOAT 우선, 없으면 RATE). 없으면 0 | 144-147, 174-180 |
| `text(id \| name)` | `StringProperty`. null 이면 "" | 182-189 |
| `offset(id)` | `{x, y, w, h, r, a}` 새 테이블 | 191-204 |
| `timer(id)` | 타이머 시작 시각(µs). 꺼져 있으면 `Long.MIN_VALUE` | 206-211 |
| `timer_off_value` | `Long.MIN_VALUE` 상수 | 39 |
| `time()` | 현재 시각(µs, 화면 진입 기준) | 250-255 |
| `set_timer(id, µs)` | 커스텀 타이머 id 에만 허용, 그 외는 예외 | 257-267, `B:skin/SkinPropertyMapper.java:163-166` |
| `event_exec(id [, a1 [, a2]])` | 이벤트 실행 | 269-295 |
| `event_index(id)` | `getImageIndexProperty(id).get(state)` | 297-303 |
| `gauge()` / `gauge_type()` | 플레이 중 게이지 값 / 종류, 그 외 0 | 105-117 |
| `judge(n)` | 판정 n 의 early+late 합 | 87-93 |
| `rate()`, `exscore()` 등 | `ScoreDataProperty` 값 | 54-59 |

`timer_util.timer_observe_boolean(f)` 는 상태를 가진 타이머 함수를 돌려준다: 호출될 때마다 `f()` 를 평가해, 참이 된 순간의 `now µs` 를 기억하고 거짓이 되면 `MIN_VALUE` 로 되돌린다(`TimerUtility.java:92-112`). 같은 함수 객체를 여러 객체가 공유하면 호출마다 `f()` 가 다시 돈다.

가벼운 집계로 본 ModernChic 의 사용: `main_state.option` 230, `.number` 207, `.audio_play` 82, `.text` 62, `.time` 22, `.timer` 16, `.timer_off_value` 15, `.event_index` 14, `.gauge_type` 11, `.judge` 6, `.volume_*`/`.set_volume_*` 각 2, `.rate` 2, `.float_number` 2, `.gauge` 1, `.exscore` 1. `timer_util.timer_observe_boolean` 148. `event_util` 0. `io.open` 45(쓰기 `"w"`·추가 `"a"` 포함, `M:Result/lua/history.lua:30, 43, 71`), `os.date` 5, `os.time` 3. `luajava` 는 4파일(`M:Result/lua/irmenu.lua:6-8`, `M:Result/lua/mainmenu.lua:5-7`, `M:Root/customfunction.lua:7-8, 103, 114`, `M:Select/lua/require/http.lua:8, 22, 32`). `string.gsub`/`match`/`gmatch` 3곳(`M:Root/customfunction.lua:124-125`, `M:Play/lua/sp/detailinfo/nentyakuinfo.lua:26`). `setmetatable`·`coroutine`·`bit32`·`goto`·`math.pow`·전역 `unpack` 0. 전역 변수 대입을 쓴다(`main_state = require("main_state")`, `PROPERTY = ...`, `MAIN`, `CUSTOM`, `CONFIG`, `BASE`; `M:play7_hw.lua:8-20`). 부품 로드는 `pcall(function() return dofile(skin_config.get_path("...lua")).load(n) end)` 형태다(`M:play7_hw.lua:45-55`).

### 5.3 Lua 버전 문제

현재 5.4 와 LuaJ(5.2 계열) 사이에서 스킨 결과가 달라질 수 있는 지점은 다음과 같다. 전부 Lua 언어 사양 차이이며 이 조사에서 실행으로 재현하지는 않았다(미검증).

| 지점 | 5.4 | 5.2 | 영향 |
| --- | --- | --- | --- |
| `/` 결과의 문자열화 | `10/2 .. ""` → `"5.0"` | `"5"` | `id = "x" .. (n/2)` 꼴의 id 문자열이 달라져 객체 참조가 끊어짐 |
| `string.format("%d", 1.5)` | 오류 | 절삭 | `pcall` 안이면 조용히 부품이 사라짐 |
| 정수 나눗셈 `//`, 비트 연산자 | 있음 | 없음(구문 오류) | ModernChic 에서 URL·주석 줄을 제외하면 `//` 사용 0건(grep). 5.2 로 내려도 구문 오류 위험 없음 |
| `math.floor` 반환 | 정수 | 실수(정수값) | 문자열화에서만 차이 |
| `table.unpack` / `unpack` | `table.unpack` | `table.unpack` | 동일 |
| `bit32` | 없음 | 있음 | ModernChic 미사용 |
| `load` | 있음 | 있음 | 동일 |

`mlua` 피처를 `lua52` 로 바꾸면 위 차이가 모두 사라진다. 5.4 를 유지하려면 변환기에서 "정수값 실수 → 정수 문자열" 보정(L13)과 `%d` 호환 패치가 필요하고, 그래도 스킨 내부의 문자열 연결은 고칠 수 없다. 권고는 `lua52` 이다.

### 5.4 풀 Lua 실행을 위해 바꿔야 할 것

| 번호 | 현재 | 필요한 동작 | 비고 |
| --- | --- | --- | --- |
| S1 | 라이브러리 3종만 | `coroutine`, `io`(자체 구현), `os`(안전 부분집합), `package`(자체 검색기), `bit32`(5.2 면 기본 포함) | `io`·`os`·`package` 는 mlua 표준 라이브러리를 그대로 열지 말고 Rust 로 구현한 테이블을 넣는다(루트 가두기) |
| S2 | `require` 제거 | `package.loaded` 캐시 + `<스킨폴더>/?.lua` 검색(점 → 경로 구분자). 선등록 모듈 `main_state`, `timer_util`, `event_util`, `luajava` | 헤더 패스에서도 `require("main_state")` 가 실패하면 안 됨. 레퍼런스는 빈 테이블을 미리 넣는다(`SkinLuaAccessor.java:95-102`) |
| S3 | `dofile`/`loadfile`/`load` 제거 | 스킨 루트 안 경로만 허용하는 `dofile`·`loadfile`, 텍스트 청크만 받는 `load` | `ChunkMode::Text` 강제는 유지 |
| S4 | `print` 제거 | 로그로 보내는 `print` | 119회 호출이 nil 호출 오류가 되면 `pcall` 밖에서는 스킨 전체가 실패 |
| S5 | `setmetatable` 등 제거 | 복원 | ModernChic 은 미사용이나 다른 스킨 호환 |
| S6 | `string.find/gmatch/gsub/match` 제거 | 복원 | 백트래킹 폭주는 벽시계 한도로만 막을 수 있다(C 내부라 훅이 안 걸림, `src/lua.rs:76-82` 주석의 우려는 유효) |
| S7 | `io`/`os` 없음 | `io.open`(읽기·쓰기·추가, 루트 안), `io.lines`, 파일 핸들 `read`/`write`/`lines`/`close`/`seek`; `os.time`/`date`/`clock`/`difftime` | 스킨 폴더에 쓰기를 허용할지는 결정 필요(레퍼런스는 허용, ModernChic 은 결과 이력을 스킨 폴더에 기록) |
| S8 | 명령 20만·20ms / 프레임 4ms·4,096회 | 로드 단계와 프레임 단계의 예산 분리. 로드는 초 단위·수천만 명령, 프레임은 객체 수백~수천 호출을 감당 | 레퍼런스는 프레임마다 모든 객체의 `prepare` 를 돌린다(`B:skin/Skin.java:316-323`, 기본 `prepareduration = 1µs`) |
| S9 | 메모리 8MiB | 로드 중 수십 MiB | 25,500줄 Lua + 수천 개 테이블. 실측 필요(미확인) |
| S10 | 호출마다 `scope` + 테이블 + 클로저 6개 생성 | 상태 바인딩을 프레임당 1회로 | 아래 5.5 |
| S11 | `skin.*` 6함수 | `main_state` 전 함수 + JSON 식용 전역 별칭 | 이름 조회(`option("name")`)도 지원해야 함 |
| S12 | 식 핸들만 | Lua 함수 값을 레지스트리에 보관하고 `LuaExprId` 로 참조 | `functions: RefCell<Vec<mlua::Function>>` 구조 재사용 가능(`src/lua.rs:93`) |
| S13 | 반환값 4형 | 타이머 함수(µs 또는 OFF), 이벤트(인자 0~2), `FloatWriter`/`StringWriter`(인자 1) 호출 경로 | |
| S14 | 실패 경고가 프로세스당 1회 | 함수(객체)별 1회 | 현재 `WarnOnce` 는 샌드박스당 하나라 첫 오류 뒤로는 모든 오류가 묻힌다(`src/lua.rs:95, 313, 319, 342`) |
| S15 | 난수 시드 고정 | 유지 가능(선택) | ModernChic `math.random` 13회 |
| S16 | `pcall` 은 남아 있음(기본 함수) | 실패한 `pcall` 을 디버그 로그에 남기는 선택 기능 | ModernChic 은 부품 로드를 전부 `pcall` 로 감싸므로 API 누락이 "부품이 조용히 없음"으로 나타난다 |

### 5.5 상태 바인딩 설계 제약

- `#![forbid(unsafe_code)]`(`src/lib.rs:12`) 때문에 원시 포인터로 프레임 상태를 넘길 수 없다.
- `SkinStateSource` 구현체는 프레임마다 만들어지는 빌린 뷰다(`crates/rbms-render/src/skin_render/state.rs:200-209` 의 `PlayViewState<'a>`). `'static` 클로저가 직접 잡을 수 없다.
- 스킨은 로드 시점에 `local main_state = require("main_state")` 로 테이블을 잡고, 함수를 지역 변수로 캐시할 수도 있다. 따라서 `main_state` 테이블과 그 함수들은 로드부터 해제까지 같은 객체여야 한다.

안전한 구성 한 가지: `main_state.number` 등은 영구 Lua 트램펄린(`function(id) return host.number(id) end`)으로 만들고, 숨긴 `host` 테이블의 필드만 "프레임당 1회" `lua.scope` 안에서 스코프 함수로 갈아 끼운다. 프레임의 모든 Lua 호출(그리기 조건·값·타이머·커스텀 타이머 갱신)을 그 한 스코프 안에서 수행한다. 로드 단계(헤더·본체 패스)도 같은 방식으로 상태 원천을 받아야 한다(레퍼런스는 로드 중에도 `main_state` 가 실제 상태를 읽는다, `JSONSkinLoader.java:83-84`). 이 구성은 `LuaFrame`(`src/lua.rs:291-364`)의 "프레임 단위 객체" 개념을 그대로 살린다.

`timer_util.timer_observe_boolean` 같은 상태 보유 함수는 Lua 로 구현해 번들하면 Rust 쪽 상태가 필요 없다(레퍼런스 동치 코드가 주석에 있음, `TimerUtility.java:63-70`).

### 5.6 재사용 가능한 것과 버릴 것

| 구분 | 항목 | 근거 |
| --- | --- | --- |
| 재사용 | `LuaExprId` 핸들 + `functions` 벡터 + 소스별 중복 제거 | `src/lua.rs:93-94, 175-190` |
| 재사용 | `LuaFrame`/`spend` 의 프레임 예산·통계 골격(수치만 조정) | `:291-348` |
| 재사용 | 훅 기반 명령·벽시계 한도, `set_memory_limit` | `:111, 130-139` |
| 재사용 | `ChunkMode::Text` 강제(바이트코드 차단), `string.dump` 제거 | `:179, 83` |
| 재사용 | `as_number` 강제 변환, 진리값 규칙 | `:231-257, 379-387` |
| 재사용 | `LuaDrawEval` 분리(`dst` 가 Lua 없이 컴파일) | `src/dst.rs:100-104` |
| 재사용 | `Budget` 구조체(필드 추가) | `src/loader.rs:76-98` |
| 폐기 | `FORBIDDEN_GLOBALS` 대부분(`debug`·`newproxy` 정도만 유지) | `:55-74` |
| 폐기 | `FORBIDDEN_STRING_FUNCTIONS` 중 `dump` 외 4종 | `:83` |
| 폐기 | `skin.*` 6함수와 호출마다 만드는 스코프 테이블 | `:202-213` |
| 폐기 | 샌드박스당 단일 `WarnOnce` | `:95` |
| 폐기 | 기본 예산 수치 | `src/loader.rs:39-55` |

---

## 6. `dst.rs`·타이머·속성 레지스트리의 의미론 차이

### 6.1 일치가 확인된 부분

| 항목 | rbms | 레퍼런스 |
| --- | --- | --- |
| 센티넬 채움: 첫 키프레임 기본값(0, 색 255), 이후는 직전 값 상속, 클립은 첫 프레임에서 기본값 없음 | `src/loader/track.rs:44-86` | `JSONSkinLoader.java:422-453` |
| 클립은 4변이 모두 있을 때만 성립 | `track.rs:89-94` | `JSONSkinLoader.java:461-463` |
| 오프셋 목록 = `offsets` + `offset`(0 이어도 덧붙임) | `track.rs:252-253` | `JSONSkinLoader.java:470-475` |
| `stretch >= 0` 일 때만 적용 | `model.rs:327`, `stretch.rs:40-54` | `JSONSkinLoader.java:476-478` |
| 키프레임 시간 오름차순 삽입(같은 시간은 선언 순) | `track.rs:181`(안정 정렬) | `SkinObject.java:240-254` |
| 루프: `loop == -1` 이면 끝 뒤로 미표시, 그 외 `end > 0 && time > loop` 이면 `end == loop ? loop : (time-loop) % (end-loop) + loop` | `src/dst.rs:379-389` | `SkinObject.java:360-375` |
| `getRate`: 마지막 키프레임 정각이면 (last, 0), 뒤에서부터 구간 탐색, 못 찾으면 (0, 0) | `dst.rs:300-316` | `SkinObject.java:545-575` |
| `acc`: 1 = `r*r`, 2 = `1-(r-1)^2`, 3 = 보간 없음 | `dst.rs:134-140, 396` | `SkinObject.java:559-566, 388-394` |
| 영역 오프셋: `relative` 가 아니면 `x += off.x - off.w/2`, `y += off.y - off.h/2`; 항상 `w += off.w`, `h += off.h` | `dst.rs:343-350` | `SkinObject.java:404-413` |
| 클립 보간·오프셋·`w>0 && h>0` 판정 | `dst.rs:403-413` | `SkinObject.java:435-473` |
| 색: 전 키프레임 색이 같으면 오프셋 알파 적용, 보간 중이면 오프셋 알파 미적용, `rate == 0` 이면 적용 | `dst.rs:415-425` | `SkinObject.java:480-520` |
| 각도 보간 후 절삭 | `dst.rs:427` | `SkinObject.java:537` |
| 그리기 순서: 조건 → 영역 → 화면 오프셋 → 클립 → 마우스 사각형 | `dst.rs:460-492` | `SkinObject.java:591-611` |
| 마우스 사각형: 영역 기준 상대좌표, 경계 포함 | `dst.rs:174-176, 484-489` | `SkinObject.java:603-604` |
| 타이머 표 151종, 속성 표 817종, 합계 968 | `src/timer/generated.rs:167`, `src/property/generated.rs:61, 65` | `B:skin/SkinProperty.java` 의 `public static final int` 968건, 그중 `TIMER_` 151건(grep 집계) |
| 불리언 음수 id 는 `abs` 조회 후 부정 | `src/property/mod.rs:125-127` | `B:skin/property/BooleanPropertyFactory.java:28-67` |
| 타이머 대역: 내장 `0..=2999`, 커스텀 `10000..=19999` | `src/timer.rs:32-39`, `timer/generated.rs:324-326` | `B:TimerManager.java:20, 59-65` |
| `switch`: 켤 때 이미 켜져 있으면 유지 | `src/timer.rs:79-85` | `TimerManager.java:87-95` |

### 6.2 차이가 확인된 부분

| 번호 | 항목 | rbms | 레퍼런스 | 영향 |
| --- | --- | --- | --- | --- |
| D1 | 오프셋 `r` 의 부호 | 문서 각도를 로드 시 부호 반전해 시계방향으로 저장(`track.rs:107`)한 뒤, 프레임에서 `angle_deg + offset.r` (`dst.rs:428-430`). 오프셋 공급자에 부호 반전 없음(`crates/rbms-render/src/skin_render/state.rs:195-197, 285-287`, `crates/rbms-config/src/schema.rs:237-239`) | `angle += off.r` — 문서(반시계) 공간에서 더함(`SkinObject.java:526-542`) | 같은 `r` 값이 반대 방향으로 돈다. `r` 축을 여는 오프셋에서만 보임 |
| D2 | 미선언·미지원 정수 `op` | 문서가 선언한 id 는 로드 시 확정(`track.rs:197-204`). 그 외는 `known_option(abs)` 가 참이면 매 프레임 `state.boolean(id)`(`dst.rs:283-296`). 앱은 `known_option` 을 지정하지 않아 기본 `every_option_known` 이 쓰인다(`loader.rs:163-165, 196`; 앱에 `known_option` 참조 없음) | `BooleanPropertyFactory` 가 아는 id 는 `BooleanProperty` 로(정적이면 로드 시 1회 평가 후 제거). 모르는 id 는 스킨 옵션 맵에서 조회: 양수는 값이 1 이 아니면 객체 제거, 음수는 값이 0 이 아니면 제거. 맵에 없으면(-1) 부호와 무관하게 제거(`SkinObject.java:282-299`, `Skin.java:199-232`) | "아무도 모르는 음수 id" 가 rbms 에서는 보이고(`!false`), 레퍼런스에서는 제거된다 |
| D3 | 정적 조건의 평가 시점 | 모든 조건을 매 프레임 평가 | `isStatic(state)` 인 조건은 `Skin.prepare` 에서 1회 평가, 거짓이면 객체 영구 제거(`Skin.java:204-214`, `BooleanPropertyFactory.java:215-222`) | 로드 후 상태가 바뀌는 "정적" 속성에서만 차이. 성능 차이는 큼(객체 수 감소) |
| D4 | 타이머를 식·함수로 지정 | 경고 후 타이머 없음(프레임 시계)으로 폴백(`track.rs:231-242`). 이미지 셀 애니메이션 타이머도 id 만(`skin_render/object.rs:524-526`) | 타이머 함수를 프레임마다 호출해 µs 시작 시각을 얻음(`SkinLuaAccessor.java:735-747`) | ModernChic 의 `timer_observe_boolean` 148곳이 전부 무효 |
| D5 | 타이머 해상도 | `TimerState` 는 ms `i64`(`src/timer.rs:56-58, 98-100`), `SkinStateSource::timer` 도 ms, `now_ms` | 내부 µs, `main_state.timer`/`time` 은 µs, `get()` 은 `µs/1000`(`B:skin/property/TimerProperty.java:6-10`, `TimerManager.java:44-61`) | Lua API 를 µs 로 내려면 저장도 µs 여야 왕복 오차가 없다 |
| D6 | 타이머 OFF 표현 | `Option::None` | `Long.MIN_VALUE` | Lua 경계에서 변환 필요. Lua 5.2 숫자(double)는 `-2^63` 을 정확히 표현 |
| D7 | 커스텀 타이머·이벤트 | 모델에 보관만 | `customTimers` 는 프레임당 1회 id 오름차순으로 갱신 후 `customEvents` 갱신(`Skin.java:782-789`, `B:skin/CustomTimer.java`). 능동 타이머는 함수 결과를, 수동 타이머는 `set_timer` 값을 보관. `getMicroTimer(id)` 는 내장 범위 밖 id 를 스킨의 커스텀 타이머로 넘김(`TimerManager.java:59-65`) | ModernChic 은 `customTimers`/`customEvents` 를 선언(`M:musicselect.lua:55`, `M:result.lua:38-39`, `M:course.lua:38-39`) |
| D8 | `acc` 결정 | 시간 정렬 뒤 첫 비선형 키프레임(`dst.rs:242-244`), 1~3 밖은 선형으로 접음(`dst.rs:123-130`) | 선언 순서로 첫 비영 값이 고정(`SkinObject.java:218-220`). 4 이상도 "비영"이라 이후 값을 막음 | 키프레임이 시간 역순이거나 `acc >= 4` 일 때만 차이 |
| D9 | 색 정밀도 | 8비트 저장·보간 후 반올림(`dst.rs:324-327`) | float 보간 | 최대 1/255 오차 |
| D10 | `stretch` | 0·1·2·9 만 구현, 나머지는 0 으로 폴백(`stretch.rs:61-63, 104-106`) | 11종 전부(`B:skin/StretchType.java`) | 6.3 |
| D11 | 필터 | `filter == 0` 또는 1:1 이면 Nearest, 아니면 Linear(`stretch.rs:124-128`) | `dstfilter != 0` 이고 크기가 다르면 전용 bilinear 셰이더(`SkinObject.java:634-636`, `Skin.java:554`) | 기록된 근사 |
| D12 | 좌표 스케일 | 문서 좌표 그대로 보관, 렌더가 뷰포트로 변환 | `setDestination` 에서 `dw/dh` 를 곱해 저장(`Skin.java:150-156`), 오프셋은 스케일 후 픽셀에 더함 | 오프셋 값의 단위가 다르다: 레퍼런스는 출력 해상도 픽셀, rbms 는 문서 픽셀. 1920 스킨을 다른 해상도로 그릴 때 차이 |
| D13 | `OFFSET_ALL` | 처리 없음(이 크레이트 범위 밖일 수 있음, 미확인) | 플레이 화면에서 전체 변환 행렬에 적용: 이동 `width*x/100`, `height*y/100`, 배율 `(w+100)/100`, `(h+100)/100`(`Skin.java:382-388, 720-729`) | 플레이 스킨 전역 오프셋 |
| D14 | 이름 기반 속성 | 없음 | 3.3 | JSON·Lua 모두 |
| D15 | `float` 조회 공간 | 한 접근자가 RATE·FLOAT·SLIDER·BARGRAPH 를 모두 답함(`property/mod.rs:52-60`) | 객체 `value` 숫자는 `getRateProperty`(RATE 만), `main_state.float_number` 는 FLOAT 우선 후 RATE(`B:skin/property/FloatPropertyFactory.java:57-60, 104-111`) | 두 공간이 겹치지 않으므로(테스트 `the_two_float_id_spaces_do_not_overlap`) 관측 차이는 "RATE 필드에 FLOAT id 를 쓴 스킨"뿐 |
| D16 | `string(id) -> &str` | 빌린 문자열 반환 | 프레임마다 새 문자열 | Lua 로 넘길 때 복사 필요(`src/lua.rs:207` 이 이미 `to_owned`) |
| D17 | `timer: 0` | `TimerId(0)` 을 조회, 꺼져 있으면 미표시 | `getTimerProperty(0)` 도 비null 이라 타이머 0(항상 OFF)에 묶여 미표시(`TimerPropertyFactory.java:6-8`) | 일치. ModernChic 에 `timer = 0` 은 없음(grep 0건) |

### 6.3 `stretch` 구현 사양(미구현 7종)

`rect` 는 해소된 목적지 사각형, `img` 는 소스 영역(`x, y, w, h` 픽셀)이다. 근거는 `B:skin/StretchType.java` 전문이다.

- 보조 함수
  - `fitWidth(rect, width)`: 중심 x 유지, `rect.w = width`.
  - `fitHeight(rect, height)`: 중심 y 유지, `rect.h = height`.
  - `fitWidthTrimmed(rect, scale, img)`: `width = scale * img.w`. `rect.w < width` 이면 소스를 자른다: `w = rect.w / scale`, `img.x = (int)(img.x + img.w/2 - w/2)`, `img.w = (int)w`. 아니면 `fitWidth(rect, width)`.
  - `fitHeightTrimmed(rect, scale, img)`: 위와 대칭.

| id | 이름 | 계산 | rbms |
| --- | --- | --- | --- |
| 0 | STRETCH | 그대로 | 구현 |
| 1 | FIT_INNER | `sx = rect.w/img.w`, `sy = rect.h/img.h`. `sx <= sy` 이면 `fitHeight(img.h*sx)` 아니면 `fitWidth(img.w*sy)` | 구현 |
| 2 | FIT_OUTER | `sx >= sy` 이면 `fitHeight(img.h*sx)` 아니면 `fitWidth(img.w*sy)` | 구현 |
| 3 | FIT_OUTER_TRIMMED | `sx >= sy` 이면 `fitHeightTrimmed(sx)` 아니면 `fitWidthTrimmed(sy)` | 미구현 |
| 4 | FIT_WIDTH | `fitHeight(img.h * rect.w / img.w)` | 미구현 |
| 5 | FIT_WIDTH_TRIMMED | `fitHeightTrimmed(rect.w / img.w)` | 미구현 |
| 6 | FIT_HEIGHT | `fitWidth(img.w * rect.h / img.h)` | 미구현 |
| 7 | FIT_HEIGHT_TRIMMED | `fitWidthTrimmed(rect.h / img.h)` | 미구현 |
| 8 | NO_EXPANDING | `s = min(1, sx, sy)`; `fitWidth(img.w*s)`; `fitHeight(img.h*s)` | 미구현 |
| 9 | NO_RESIZE | `fitWidth(img.w)`; `fitHeight(img.h)` | 구현 |
| 10 | NO_RESIZE_TRIMMED | `fitWidthTrimmed(1.0)`; `fitHeightTrimmed(1.0)` | 미구현 |

트리밍 계열은 목적지 사각형과 소스 영역을 함께 바꾸므로 `stretch_rect(kind, rect, source) -> SkinRect`(`stretch.rs:82`) 시그니처로는 표현할 수 없다. 반환을 `(SkinRect /*목적지*/, SkinRect /*소스 영역*/)` 으로 바꿔야 한다. ModernChic 사용: `FIT_INNER` 3, `FIT_OUTER_TRIMMED` 10, `FIT_WIDTH_TRIMMED` 12(가벼운 집계).

### 6.4 그 밖의 목적지 필드(렌더 쪽 과제지만 이 크레이트가 값을 운반)

- `blend`: 레퍼런스가 구분하는 값은 2(가산), 3(감산 의도, 주석상 미완), 4(곱), 9(반전)이다(`Skin.java:655-665`). ModernChic 은 `blend = 9` 를 1곳에서 쓴다.
- `center`: 0~9 가 회전 중심 표 `CENTERX = {0.5, 0, 0.5, 1, 0, 0.5, 1, 0, 0.5, 1}`, `CENTERY = {0.5, 0, 0, 0, 0.5, 0.5, 0.5, 1, 1, 1}` 을 가리킨다(`SkinObject.java:80-81, 229-233`). y 는 아래가 0 인 문서 공간 기준이다.
- `loop`: ModernChic 분포는 `-1` 271, `1000` 33, `0` 19, `3000` 14 등이다.

---

## 7. rbms 전용 확장의 위치와 제거·유지 영향

| 확장 | 이 크레이트의 위치 | 소비자 위치 | 제거 시 영향 | 유지 시 영향 |
| --- | --- | --- | --- | --- |
| `composition`(`replace`/`overlay`/`layered`) | `model.rs:359-366, 407, 470` | `apps/rbms-player/src/skin_screen.rs:31, 599, 607, 612` | 필드·enum 삭제. 앱의 3갈래 합성 분기 삭제. 테스트 `skin_model.rs:36` | 레퍼런스 스킨에는 이 키가 없어 기본 `Replace` 로 읽힌다. 무해하지만 "내장 화면 혼합" 경로가 남는다 |
| `destination.layer`(`background`/`foreground`) | `model.rs:298, 372-378`, `loader.rs:266, 425, 682` (`NamedTrack.layer`) | `skin_render/object.rs:15, 78`, `skin_render/mod.rs:400-402`, `skin_render/screen.rs:13, 60`, `stage/play/mod.rs:23, 593, 878, 901`, `stage/select/mod.rs:20, 814, 864`, `stage/result.rs:7, 128, 139`, `skin_screen.rs:760, 803, 864` | `NamedTrack` 에서 필드 삭제, `draw_layer` 계열 삭제. 테스트 `skin_model.rs:46` | 단독으로는 무해 |
| `replace`(최상위) + `result.replace` | `model.rs:383-387, 440, 461`, `loader.rs:338, 369-371, 634, 649` | `skin_screen.rs:518, 523`, `assets.rs:1002` | 필드 3개와 `replace_names()` 삭제(로더 5행). 앱의 대체 단위 게이트(`screen_content_of`) 전체가 무의미해짐. 테스트 `skin_nested.rs:152, 159` | 레퍼런스 스킨에는 없으므로 빈 집합 |
| `hotspot` | `model.rs:394-399, 462`, `loader.rs:66, 482-489, 677` | `skin_render/mod.rs:251, 293, 359-397`, `skin_render/state.rs:809` | 필드·`HOTSPOT_ACTIONS`·`keep_known_hotspots` 삭제. 클릭은 레퍼런스 방식(`image.act`/`click`, `songlist.clickable`, `mouseRect`)으로 대체해야 함. 테스트 `skin_nested.rs:135, 144` | 레퍼런스 클릭 디스패치를 구현하면 중복 |
| `scope: 'bundle'` | `model.rs:133, 153, 170, 381`, `resolve.rs:108-111, 308` | `apps/rbms-player/src/skin_select.rs:27, 154, 335-374, 767`, `crates/rbms-config/src/schema.rs`(shared 저장, 세부 미독) | 필드 4곳 삭제. 설정의 번들 공유 저장소와 SKIN 탭 `BUNDLE` 그룹 삭제 또는 무력화. 테스트 `skin_nested.rs:168, 180` | 레퍼런스 스킨은 `scope` 가 없어 문서 전용으로 읽힌다. ModernChic 은 화면별 `property.lua` 가 따로라 공유가 필요 없다 |
| `densitygraph` | `model/objects.rs:601-625`, `model.rs:15, 439` | `skin_render/graphs.rs:309` | 필드·레코드 삭제. 레퍼런스의 `SkinNoteDistributionGraph`(JSON 에서는 `graph` 의 `type` 음수 등으로 선언, 본 조사 미확인) 구현으로 대체 | 무해 |
| 사설 속성 id 20001~20315 | 이 크레이트에는 없음 | `crates/rbms-render/src/skin_render/state.rs:101-148` | 이 크레이트 무관 | 커스텀 타이머 대역(10000~19999) 바로 위지만 문자열·옵션 id 공간이라 충돌 없음. 레퍼런스 `BooleanPropertyFactory` 는 `abs(id) < 65536` 까지 받는다(`BooleanPropertyFactory.java:24, 30`) |
| `judge.images[k]` 가 `text` id 를 가리킴 | 이 크레이트에는 없음(모델은 id 문자열만) | `skin_render/judge.rs` | 무관 | 무해 |
| `LoadedSkin.nested`(반복 객체 트랙 선조립) | `loader.rs:271-310, 442-479` | `skin_render/notes.rs`, `judge.rs`, `songlist.rs` | 확장이 아니라 구조 결정(D2)이다. 레퍼런스도 같은 시점에 조립하므로 유지 권고 | |
| `SkinStateSource`·`MAPPINGS`·`UnmappedLog` | `property/mod.rs` | `skin_render/state.rs` | 확장이 아니라 rbms 측 계약. 유지 | |
| 경로 가두기·시드 난수·크기 한도 | `resolve.rs`, `loader.rs:36` | | 레퍼런스에 없는 안전장치. 유지 | |

판단 근거: 사용자는 "내장 화면 + 문서 혼합 합성(layered/replace)"을 부적합으로 판정했다(과제 설명). 이 크레이트에서 그 계약을 이루는 것은 `composition`·`layer`·`replace`·`hotspot` 4종이며, 제거 비용은 모델 약 60행·로더 약 30행·테스트 6건이다. `scope` 와 `densitygraph` 는 혼합 합성과 독립이다.

문서 정책 주의: `docs/acknowledge/2026-09-17-skin-system-decisions.md:168-169` 는 "외부 스킨 묶음의 자산을 저장소에 복사하지 않는다", "저장소 파일명·문서·UI 문자열에 외부 스킨과 레퍼런스 엔진의 고유 이름을 쓰지 않는다"를 기록한다. ModernChic 단순화판을 기본 스킨으로 저장소에 넣는 이번 목표와 정면으로 부딪히므로 사용자 결정으로 갱신해야 한다. `docs/skin.md`·`docs/plan/2026-09-17-skin-system-completion.md` 는 JSON5 + 혼합 합성 전제로 쓰여 있어 전면 개정 대상이다.

---

## 8. 테스트 자산 평가

테스트 수는 `#[test]` 집계 기준이다. 실행 결과는 확인하지 않았다.

| 파일 | 수 | 판정 | 이유 |
| --- | --- | --- | --- |
| `src/dst/tests.rs` | 38 | 유지 | 보간·루프·오프셋·클립·색·조건·마우스. 레퍼런스 의미론의 회귀 방어선. D1(오프셋 `r` 부호)을 고치면 `each_angle_offset_truncates_on_its_own`(312행)의 기대값 검토, D2 를 고치면 `op_lists_drop_zero_duplicates_and_unknown_ids`(379행) 수정 |
| `src/timer/tests.rs` | 14 | 유지, 일부 수정 | 표 체크섬·id 대역은 그대로. `TimerState` 를 µs 로 바꾸면 상태 테스트 5건(126~180행) 단위 수정 |
| `tests/property_checksum.rs` | 23 | 유지 | 생성 표와 `MAPPINGS` 검증. 구조 변경과 무관 |
| `tests/skin_resolve.rs` | 30 | 유지 | 와일드카드·filemap·루트 가두기·시드. L6(`def` 스템 대조)을 고치면 `a_documents_suggestion_names_the_file_when_the_player_has_chosen_nothing`(240행) 수정 |
| `tests/skin_model.rs` | 23 | 대부분 유지 | 기본값·키 철자 검증. `composition`·`layer` 2건(36, 46행)은 확장 제거 시 삭제 |
| `tests/skin_loader.rs` | 56 | 유지(JSON 경로를 남길 때) | 파서·헤더·센티넬 채움·조건·include·stretch. 센티넬 채움 5건(351~404행)과 stretch 5건(636~675행)은 Lua 경로와 공유되는 로직이라 가치가 높다. `a_timer_named_by_an_expression_runs_on_the_frame_clock_and_says_so`(544행)는 D4 수정 시 반대 기대로 바뀜. `a_mode_this_build_does_not_reproduce_stretches_instead_of_dropping_the_object`(658행)는 stretch 완성 시 삭제 |
| `tests/skin_branches.rs` | 5 | 유지(JSON 경로 한정) | Lua 경로와 무관 |
| `tests/skin_nested.rs` | 14 | 9건 유지, 5건 확장 의존 | 중첩 트랙 조립은 유지. `hotspot` 2건(135, 144행), `replace` 2건(152, 159행), `scope` 2건(168, 180행) 중 제거하는 확장에 해당하는 것 삭제 |
| `tests/skin_integration.rs` | 19 | 대부분 유지 | 상태 계약 통합. Lua 하위 모듈 5건(303~351행)은 `skin.*` 이름에 묶여 재작성. `an_expression_still_cannot_reach_the_filesystem_through_the_registry`(351행)는 정책 반전 |
| `tests/skin_lua.rs` | 28 | 약 절반 무효 | 금지 전역(94행), 파일 불가(113행), 런타임 로드 불가(121행), 패턴 함수 제거(325행), 프레임 호출 한도 수치(270, 283, 351, 370행)는 정책이 뒤집힌다. 유지 가치: 무한 루프 차단(128행), 메모리 차단(146행), 예산 복원(136행), 바이트코드 차단(305행), 진리값(206행), 동일 소스 1회 컴파일(216행), 벽시계 한도(383행) |
| `tests/fixtures/minimal` | | 유지 | `skin.json` 55줄 + `parts/panel.json` + 더미 png/ttf. 로더·resolve 테스트 공용 |
| `tests/fixtures/nested` | | 유지, 확장 키 정리 | `play.json5`, `select.json5`, `result.json5`, `shared/note.json5`. `hotspot`·`replace`·`scope` 키 포함 |
| `tests/fixtures/escape`, `lenient`, `secret.txt` | | 유지 | 경로 탈출·json5 관용 파싱 |
| `tools/gen-skin-property.rs`, `tools/gen-skin-timer.rs` | | 유지 | 현재 레퍼런스 소스와 선언 수가 일치(968/151). 이름 기반 조회를 넣으려면 생성기가 이름 → id 역표도 내도록 확장(`BooleanType` 등 enum 이름은 `SkinProperty.java` 상수명과 다를 수 있음, 미확인) |

새로 필요한 테스트 자산: `.luaskin` 최소 픽스처(헤더 패스/본체 패스 분기, `require` 캐시 유지, `dofile`, `skin_config` 5키, 함수 값 필드 5종, 실수 좌표 절삭, 숫자 id 문자열화, `io` 루트 가두기, `pcall` 실패 로그).

소비자 쪽 테스트(범위 밖, 규모만 확인): `crates/rbms-render/tests/skin_render.rs` 687줄, `skin_render/tests*.rs` 3파일 1,866줄, `apps/rbms-player/src/stage/render_tests_skin*.rs` 5파일 1,639줄, `skin_select/tests.rs` 444줄, `stage/play/tests.rs` 792줄, `stage/select/tests.rs` 657줄. 이들은 `steel-neon-v3` 번들과 혼합 합성에 묶여 있어 구조 변경 시 대량 무효가 예상된다(상세는 미독).

---

## 9. 권고 — 이 크레이트의 변경 단위

크기: S = 100줄 이하, M = 100~400줄, L = 400줄 이상(테스트 제외 추정).

| 순서 | 단위 | 파일 | 크기 | 선행 | 내용 |
| --- | --- | --- | --- | --- | --- |
| U1 | Lua 버전·피처 | `Cargo.toml` | S | 없음 | `mlua` 피처 `lua54` → `lua52`(5.3 절). `send` 채택 여부 결정(L18) |
| U2 | 타이머 µs 화 | `src/timer.rs`, `src/dst.rs`(`resolve` 의 `time -= timers.get`), `src/property/mod.rs`(`timer`, `now_ms` 계약) | M | 없음 | 저장을 µs 로, 보간은 `µs/1000`. 커스텀 타이머(능동/수동) 저장소와 "내장 범위 밖 id → 커스텀" 라우팅 추가(D5·D7). 소비자 타이머 드라이버 호출부 동반 수정 필요 |
| U3 | 값 참조 타입 확장 | `src/model.rs`(`PropertyRef`), `src/dst.rs`(`DrawCondition`, `DestinationTrack.timer`) | M | 없음 | `PropertyRef` 에 `Func(LuaExprId)` 와 `Name(String)` 추가. `DestinationTrack.timer` 를 `Option<TimerRef>`(`Id(TimerId)` / `Lua(LuaExprId)`)로. `dst::resolve` 가 타이머 값을 콜백으로 받도록 변경(D4) |
| U4 | 샌드박스 재구성 | `src/lua.rs` → `src/lua/{mod,env,io,package,main_state,util}.rs` 로 분할 | L | U1, U2 | 5.4 절 S1~S16. 로드 예산과 프레임 예산 분리, 프레임당 1회 상태 바인딩(5.5), 함수 등록, `main_state`·`timer_util`·`event_util` 모듈, 제한 `io`/`os`/`package`, `print`, `luajava` 파사드 최소형(범위는 다른 조사 결과에 따름) |
| U5 | Lua 값 → `SkinDef` 변환기 | 새 `src/loader/from_lua.rs` | L | U3, U4 | 3.3 표의 규칙 그대로. 정수 절삭, 진리값, 숫자 → 문자열, `keys()` 순서 배열, 모르는 키 무시, 함수 값 등록, `FontFallback` 문자열형, `DestinationOption` 3형 |
| U6 | Lua 로드 경로 | `src/loader.rs`(분기), 새 `src/loader/lua_skin.rs` | L | U4, U5 | 4.1 절 6단계: 같은 인터프리터로 헤더 패스 → 선택 반영 → filemap → `skin_config` → 본체 패스. `load_header` 도 Lua 분기. `LoadedSkin` 은 그대로 채움(`sources`, `fonts`, `destinations`, `nested`). `branch::transform` 은 건너뜀 |
| U7 | 헤더·선택 의미론 보정 | `src/resolve.rs`(`build_filemap`), `src/loader/branch.rs`(`selected_option`), `src/loader.rs`(`SkinHeader`) | M | 없음 | L6(`def` 스템 대조), L10(무작위 -1), L11(플레이 자동 오프셋 4종), L14(type 9·15) |
| U8 | 그리기 조건 의미론 | `src/loader/track.rs`, `src/dst.rs` | M | U3 | D2(미선언·미지원 id 는 부호와 무관하게 객체 제거), D3(정적 조건 선평가는 선택), 이름 조회(D14) |
| U9 | 오프셋 `r` 부호 | `src/dst.rs:427-430` | S | 없음 | D1. 테스트 기대값 동반 수정 |
| U10 | `stretch` 완성 | `src/loader/stretch.rs` | M | 없음 | 6.3 절 7종. 반환을 (목적지, 소스 영역) 쌍으로. 렌더 `draw.rs:14` 호출부 동반 수정 |
| U11 | 혼합 합성 확장 제거 | `src/model.rs`, `src/loader.rs`, `src/resolve.rs`, `tests/skin_nested.rs`, `tests/skin_model.rs`, `tests/fixtures/nested/*` | M | 소비자 쪽 제거와 동시 | 7장. `composition`·`layer`·`replace`·`hotspot` 삭제. `scope`·`densitygraph` 는 별도 결정 |
| U12 | source 지연 해석 | `src/loader.rs:657-665` | S | U6 | L12. `LoadedSkin.sources` 를 "참조된 것만" 또는 종류(이미지/동영상) 포함 형태로 |
| U13 | 테스트 | `tests/skin_lua.rs` 재작성, 새 `tests/skin_luaskin.rs`, 새 `tests/fixtures/luaskin/*` | L | U4~U8 | 8장 |
| U14 | 문서 | `docs/skin.md`, `docs/plan/*`, `docs/acknowledge/*` | M | 전체 | 7장 말미의 정책 충돌 갱신 포함 |

의존 순서 요약
1. U1, U2, U3, U7, U9, U10 은 서로 독립이라 병렬 가능하다(단 U2·U3 은 모두 `src/dst.rs` 를 만지므로 같은 작업자 또는 직렬).
2. U4 는 U1·U2 뒤.
3. U5 는 U3·U4 뒤, U6 은 U4·U5 뒤, U8 은 U3 뒤.
4. U11 은 소비자(`rbms-render`, `rbms-player`) 변경과 한 묶음으로 가야 컴파일이 유지된다.
5. U12·U13·U14 는 마지막.

소비자에 전파되는 시그니처 변경(이 크레이트 밖 작업량의 근거)
- `SkinStateSource::timer`/`now_ms` 단위(U2) → `skin_render/state.rs` 의 화면별 구현체 5종, `screen.rs` 타이머 드라이버.
- `PropertyRef` 변형 추가(U3) → `skin_render/object.rs:267-272, 524-526`.
- `stretch_rect` 반환형(U10) → `skin_render/draw.rs`.
- `LuaSandbox`/`LuaFrame` API(U4) → `apps/rbms-player/src/skin_screen.rs:30-127`, `SkinAssets::expression`(`skin_render/mod.rs:118-122`).
- `NamedTrack.layer`·`replace_names()`·`def.hotspot`·`def.composition` 제거(U11) → 7장 표의 소비자 위치 전부.

---

## 10. 읽은 범위와 읽지 못한 범위

전부 읽은 파일(R-BMS)
- `crates/rbms-skin/Cargo.toml`, `src/lib.rs`, `src/loader.rs`, `src/loader/branch.rs`, `src/loader/track.rs`, `src/loader/stretch.rs`, `src/model.rs`, `src/model/objects.rs`, `src/model/graphs.rs`, `src/dst.rs`, `src/resolve.rs`, `src/lua.rs`, `src/timer.rs`, `src/property/mod.rs`, `src/property/generated.rs`
- `docs/skin.md`, `docs/plan/2026-09-17-skin-system-completion.md`, `docs/acknowledge/2026-09-17-skin-system-decisions.md`

구조 파악 수준으로 읽은 파일(R-BMS)
- `src/timer/generated.rs`(1~60행, 290~327행), `src/property/generated/{boolean,float,integer,misc,text}.rs`(각 머리 30행과 꼬리 15행)
- `tests/*.rs` 8개, `src/dst/tests.rs`, `src/timer/tests.rs`: 테스트 함수 이름 전수와 일부 본문(`dst/tests.rs:296-345`, `skin_loader.rs:380-395`, `skin_lua.rs:60-110`)
- `tests/fixtures`: 디렉터리 구조와 `minimal/skin.json` 전문
- `tools/gen-skin-property.rs`(1~60행), `tools/gen-skin-timer.rs`(1~40행)
- 소비자: `crates/rbms-render/src/skin_render/mod.rs`(1~425행), `apps/rbms-player/src/skin_screen.rs`(40~60, 86~130행), 나머지는 grep 으로 사용 위치만 확인

전부 읽은 파일(레퍼런스)
- `skin/json/JsonSkin.java`, `skin/json/JSONSkinLoader.java`, `skin/json/JsonSkinSerializer.java`, `skin/lua/LuaSkinLoader.java`, `skin/lua/SkinLuaAccessor.java`, `skin/lua/MainStateAccessor.java`, `skin/lua/SkinLuaPathResolver.java`, `skin/StretchType.java`, `TimerManager.java`, `skin/property/TimerProperty.java`, `skin/property/TimerPropertyFactory.java`, `skin/CustomTimer.java`

부분만 읽은 파일(레퍼런스)
- `skin/SkinObject.java`(40~740행), `skin/Skin.java`(47~396, 524~790행), `skin/SkinLoader.java`(20~179행), `skin/SkinHeader.java`(152~404행), `skin/lua/MainStatePropertyLuaApiExporter.java`(27~336행), `skin/lua/TimerUtility.java`(26~127행), `skin/lua/LegacySkinLuaApi.java`(1~140행), `skin/property/BooleanPropertyFactory.java`(20~100, 215~240행), `skin/property/FloatPropertyFactory.java`(40~135행)

읽지 못한 것(미확인으로 남김)
- `skin/json/JsonSkinObjectLoader.java`(761줄)과 화면별 `Json*SkinObjectLoader.java`: 객체 종류별 생성 규칙(`ref`·`value`·`act`·`click` 해석, 음수 id 참조 이미지 등). 이 보고서의 3장은 필드 존재 여부만 대조했고 객체 의미는 대조하지 않았다.
- `skin/lua/EventUtility.java`, `SkinFileLuaApiExporter.java`, `SkinHttpLuaApiExporter.java`, `SkinAudioLuaApiExporter.java`: 함수 이름만 grep 으로 확인.
- `skin/lua/LegacySkinLuaApi.java` 140행 이후(`Gdx`·`Input`·`Controllers` 파사드 세부).
- `skin/property/*Factory.java` 의 이름 → 속성 표 전체와 `isStatic` 분류 전체.
- LuaJ 자체의 숫자·문자열 변환 세부(5.3 절은 Lua 사양 지식에 근거, 실행 미검증).
- ModernChic 본문: grep 집계와 `play7_hw.lua` 1~75행, `play7_hw.luaskin` 전문만 확인.
- `crates/rbms-config/src/schema.rs` 의 번들 공유 저장 세부, 소비자 테스트 본문.
- cargo 미실행: 현재 테스트 통과 여부, Lua 5.2 전환 시 mlua API 차이(`set_memory_limit`·훅 동작) 미검증.
