# R2 — R-BMS 렌더 계층(crates/rbms-render, GPU 백엔드) 현황과 격차

> 최종 갱신 2026-10-10 · 대응 단계: 웨이브 3A(공통 그리기 의미론) 반영 · 본문은 작성 시점 서술이며, 아래 "웨이브 … 반영 사항"이 최신 것부터 우선한다 · 색인과 갱신 규칙은 [README.md](README.md)

## 웨이브 3A 반영 사항 (2026-10-10)

prepare/draw 2단계 파이프라인과 `SkinHost` 직접 그리기, 그리기 조건 의미론, 참조 이미지·음수 크기·이미지 인덱스·숫자·슬라이더·그래프 정합, TTF 텍스트, judgegraph·bpmgraph, Lua 함수 값 프레임 평가, 앱 호스트 군집 A·I·M 을 넣은 뒤의 상태다.

- (W3-0) r2-rbms-render.md §5 서두: 그리기 본체 열거형이 20개 → 22개(Bga 로 이름 변경, TextInput·Reference 추가), SkinObjectKind 에 Reference 추가. 디스패치는 draw.rs 의 draw_object 그대로지만 text → text.rs, bga → bga.rs, 참조 이미지 → refs.rs, 편집 텍스트 → text_input.rs 로 분기
- (W3-0) r2-rbms-render.md §5.1 표 text 행·bga 행: 구현 위치를 draw.rs → skin_render/text.rs(draw_text, TextBody, text_body), skin_render/bga.rs(draw_bga, 프레임의 data.bga.base)로. text 행에 'editable 이면 Body::TextInput 으로 조립되고 지금은 같은 그림' 추가
- (W3-0) r2-rbms-render.md §5.2 표의 그래프 6행: 파일 경로 graphs.rs → graphs/{gauge_graph, notes_dist, bpm, timing_dist, timing_vis, hit_error}.rs. gauge 행의 '결과 화면에서는 그리지 않음(gauge.rs:235-248)'을 '프레임이 data.gauge(GaugeFrame)를 줄 때만 그림. 결과 화면 호출부는 아직 주지 않음'으로. bpmgraph 행의 '결과 프레임에서만 데이터가 온다'를 'data.series.bpm 을 채우는 화면이면 어디서나, 현재 채우는 곳은 결과뿐'으로
- (W3-0) r2-rbms-render.md §5.3 '음수 id 참조 이미지' 행과 상단 W2-9 반영 사항: 'build_body 에 분기가 없어 경고 후 누락'을 'refs.rs 의 build_reference 가 5개 id 를 Body::Reference 로 조립(문서 선언 객체보다 먼저). 그리기는 프레임의 data.images 가 텍스처를 줄 때만이고 현재 공급 없음. 경고는 더 나오지 않음'으로
- (W3-0) r2-rbms-render.md §3.1 4번·§3.2: 소스 등록과 해제가 skin_render/mod.rs 에서 skin_render/textures.rs 의 SkinTextures::register/release 로 이동(동작 동일: 선언된 source 전부 등록)
- (W3-0) r2-rbms-render.md §7.1: SkinFrame 필드 표에서 background·extra 행을 'data: FrameData' 한 행으로. FrameExtra 표와 PlayObjectState·SelectListState·ResultSeriesState 표를 FrameData { field: NoteField, gauge: GaugeFrame, bars: SongBars, series: FrameSeries{gauge_history, timing, bpm, notes, recent_hits}, images: ReferenceImages, bga: BgaFrame } 로 교체. 각 타입의 정의 위치(notes.rs, gauge.rs, songlist.rs, graphs/*.rs, refs.rs, bga.rs)를 적음
- (W3-0) r2-rbms-render.md §7.2 6번·7번: 'FrameExtra 가 화면당 한 종류', '참조 이미지가 background 한 장뿐'은 구조상 해소(W3-0). 남은 것은 각 화면이 데이터를 채우는 일
- (W3-0) r2-rbms-render.md §8.1 '배경 슬롯 이중 경로' 행: 문서 bga 경로의 입력이 frame.background → frame.data.bga.base
- (W3-0) r2-rbms-render.md §10: E1 의 'FrameExtra 분해' 부분 완료, C2 는 본체·프레임 필드 골격까지 완료(그리기 의미론은 W3-2), D3 는 진입점 SkinScreen::pointer 자리만 생김
- (W3-1a) r2-rbms-render.md §5: §5.2 의 songlist 행과 note 행에서 'ModernChic 에서는 현재 한 번도 그려지지 않는다(dst 없는 destination)' 삭제. §5.3 의 'dst 없는 최상위 destination' 행을 '해소(W3-1a): self-placed 3종은 자체 키프레임으로 조립, 그 외 종류는 빌드에서 제외하고 경고' 로 변경. 그리기 진입이 draw_object 에서 SkinObject::prepare + draw::draw_resolved 로 바뀐 것 반영
- (W3-1a) r2-rbms-render.md §7.1: 프레임 입력 설명에 2단계 추가 — SkinFrame.lua 는 prepare 단계에서만 쓰이고 draw 단계는 PreparedFrame 의 기록을 읽음. 사용법이 'SkinLua::frame 안에서 screen.draw' 에서 'SkinLua::frame 안에서 screen.prepare, 바인딩 밖에서 screen.draw_prepared' 로 바뀜. 앱은 화면별 어댑터 대신 skin_host::ScreenHost(fallback = 기존 어댑터)로 그림
- (W3-1a) r2-rbms-render.md §8.1: 결합 지점 표에서 앱 skin_screen.rs 가 render_play/select/result/decide/keyconfig_screen 을 부르던 행을 'with_skin_frame 이 ScreenHost 로 prepare 하고 PreparedDocument::draw/draw_native 로 그림. render_*_screen 과 SkinDraw 는 렌더 테스트만 사용' 으로 변경
- (W3-2) r2-rbms-render.md 상단: '웨이브 3A 반영 사항'에 W3-2 추가 — 기본 객체는 draw.rs 의 Placement::texture 한 경로(뷰포트 → 부호 유지 stretch → 물리 픽셀 필터 판정 → quad 에서 정규화·UV 뒤집기·기준점 보정)로 그려지고, 판정 함수 ImageSelect::slot / number_value / float_value / share 를 prepare(object.rs)와 draw 가 공유한다
- (W3-2) r2-rbms-render.md §5.1 image 행: (1) 음수 선택값 `.max(0)` → 해소(조건 평가 전 미표시), (2) '일반 정수 속성으로 읽는다' → 해소(host.image_index, 레퍼런스에 없는 id 는 세트 0). imageset 행: 빠진 이미지가 자리를 유지하고 value 는 숫자 공간, ref 는 이미지 인덱스 공간이라고 고칠 것
- (W3-2) r2-rbms-render.md §5.1 value 행: '자릿수를 1..16 으로 제한' → 0..16(digit 0 은 자리 없음). '자리별 offset 을 뷰포트로 스케일…미확인' → 해소(원본은 스케일하지 않으며 화면 픽셀로 더함, SkinNumber.java:193-194). ref 없는 숫자는 미표시 추가
- (W3-2) r2-rbms-render.md §5.1 floatvalue 행: '대조하지 않았다(미확인)' → SkinFloat.java:153-207·FloatFormatter 와 대조 완료. 미표시 조건(값 없음, NaN·무한, MIN/MAX gain 전후, 자리 0), zeropadding 0..2 정규화, align 은 숫자와 반대 의미
- (W3-2) r2-rbms-render.md §5.1 slider 행: 값은 클램프하지 않고 rate 공간에서 읽으며 range 는 (int)(scale*range) 화면 픽셀. graph 행: '레퍼런스는 정수 절삭, R-BMS 는 연속값' → 해소. 값 0..1 클램프 제거, NaN 은 0, isRefNum 은 RateProperty 식(끝점 역순 포함)
- (W3-2) r2-rbms-render.md §5.3 '음수 id 참조 이미지' 행 → 구현됨(refs.rs). -100/-101/-102 는 FrameData.images, -110/-111 은 렌더 자체 1x1 텍스처, 그 외 음수 id 는 조건만 평가되고 그려지지 않는 객체
- (W3-5) r2-rbms-render.md §5.2: judgegraph 는 '판정 6종 막대/한 기둥' 모양이 아니라 원본 알고리즘으로 교체됨(초당 5px 열, 4x4 칩, 행 수 20~100, 바탕과 눈금, 750ms/50ms 갱신, 커서). bpmgraph 는 결과 화면 전용이 아니라 FrameSeries.bpm 이 있는 어느 화면에서도 그려짐. 두 그래프는 텍스처로 그려지며 SkinScreen::release 가 해제함
- (W3-5) r2-rbms-render.md §7.2-6: 결과 화면에서만 bpmgraph 데이터가 있다는 결함 해소(결정·선곡·플레이 캡처에서 두 그래프가 나옴)
- (W3-1c) r2-rbms-render.md §7(프레임 파이프라인): prepare 단계에서 타이머 함수는 프레임당 1회, 조건·값은 객체당 호출이라는 규칙과 skin_render/tests.rs 의 lua_functions 테스트 4건 추가
- (W3-3) r2-rbms-render.md §6.1: 스킨 TTF 텍스트는 더 이상 fill_rect 런 경로가 아님. `font/block.rs` 의 `TextContext::compose_block` 이 줄을 흰 글리프 RGBA 블록으로 합성하고 `skin_render/text.rs` 가 객체당 텍스처 1장(`rbms.skin.text.<n>`)으로 그림. 크기는 `px_for`(반올림·최소 8)를 거치지 않고 dst 높이(물리 픽셀) 그대로. 내장 화면과 폰트 없는 스킨 텍스트만 기존 경로
- (W3-3) r2-rbms-render.md §6.3 과 상단 W1-6 반영 사항: `load_font` 는 face 이름을 돌려줌(같은 family 의 두 번째 굵기는 'family weight'), 같은 바이트는 다시 등록하지 않음. 추가로 '기존에는 mgenplus black/medium 이 같은 typographic family(Mgen+ 1c)라 둘 다 medium 으로 그려졌다(W3-3 에서 해소)'를 기록
- (W3-3) r2-rbms-render.md §6.4 표: 'TTF 크기' 일치(반올림·최소 8 단서 삭제), '정렬 기준' 잉크 기준 줄 폭으로 구현, '세로 기준' 해소(dst 상단 = 대문자 윗선, 잔차 ±1px at size), 'overflow' 구현(0/1/2), 'wrapping' 구현(줄 나눔 규칙 차이 명시), '그림자(TTF)' 구현, '필터' 1:1 Nearest·축소 Linear(원본과 다름), 'fallback 폰트' FontDef.fallback 무시 유지 + 엔진 폴백(E5), '폰트 종류 분기' .fnt 는 대체 그리기 유지(웨이브 7)
- (W3-3) r2-rbms-render.md §5.1 표 text 행: prepare 는 값 읽기만, draw 는 text.rs 의 draw_text → draw_line → draw_composed/draw_stand_in. blend 는 자기 것이 아니라 `TextContext::inherited_blend`(draw.rs 의 `text::carry_blend` 가 기록). `SkinObject::release` 가 텍스트 텍스처도 해제
- (리뷰 수정) r2-rbms-render.md §5 (기본 객체): gauge 는 FLOAT_GROOVEGAUGE_1P 가 값 없음이면 빈 게이지로 읽는다고 적습니다. 구 어댑터 state.rs 가 image_index 를 구현(항상 0)한다는 점도 적습니다


## 웨이브 2B 반영 사항 (2026-10-10)

스킨 덤프 CLI, 앱의 스킨 팩 폴더 지정과 `.luaskin` 로드, 오버레이 총 크기 상한, 외부 스킨 첫 정지 프레임을 넣은 뒤의 상태다.

- (W2-9) r2-rbms-render.md §5.3: '음수 id 참조 이미지' 행에 실측 추가 — ModernChic 4화면에서 빠지는 개수는 decide 3, result 5, musicselect 17, play7_hw 13(+ 정의 없는 id 'lamp' 2). 같은 표에 새 행 'dst 없는 최상위 destination(songlist, notes)은 키프레임 0개라 prepare 가 None → 본체 미호출. 레퍼런스는 SkinBar/SkinNote 가 자체 목록으로 그리므로 destination 의 dst 가 필요 없다'를 추가
- (W2-9) r2-rbms-render.md §5.1: 표 아래에 '음수 폭/높이는 Placement::region(draw.rs)의 `fitted.w <= 0.0 || fitted.h <= 0.0` 에서 버려진다(뒤집기 없음). ModernChic 해당 destination: result 1, musicselect 2, play7_hw 7' 추가. graph 행에 'gra_fastRate, 스코어 막대가 이 때문에 나오지 않음(W2-9 캡처로 확인)' 추가
- (W2-9) r2-rbms-render.md §5.2: songlist 행과 note 행에 'ModernChic 에서는 현재 한 번도 그려지지 않는다(dst 없는 destination)' 추가. gauge 행의 '결과 화면에서는 그리지 않음'에 '캡처에서 (56,674,400,35)가 검은 빈 칸으로 확인' 추가. gaugegraph 행에 '배경을 불투명하게 칠해 같은 사각형의 judgegraph 를 가린다' 추가
- (W2-9) r2-rbms-render.md §7.1: '렌더 크레이트 수준에서 Lua 함수 값이 끝까지 평가됨을 crates/rbms-render/tests/skin_external.rs 가 확인. 프레임마다 SkinLua::frame 으로 호스트를 묶어 BoundFrame 을 SkinFrame.lua 로 넘기는 것이 사용법이고 skin_render/mod.rs 변경은 필요 없었다. 평가기 유무에 따른 차이: result 67 대 33 객체' 추가
- (W2-9) r2-rbms-render.md §3.4: '본체 로드 뒤 실제로 참조되는 source 는 화면당 decide 3/4, result 12/13, musicselect 11/11, play7_hw 23/27 이고 디코드 픽셀은 2.76M / 33.5M / 42.4M / 34.9M px(RGBA 약 11 / 134 / 170 / 140 MiB), 최대 3920x3800(result)·5190x2571(play). "전부 올리면 2.79 GiB"는 팩 전체 기준이고 Lua 스킨은 선택된 옵션의 source 만 선언하므로 화면 하나의 부담은 이 정도' 추가
- (W2-9) r2-rbms-render.md §9: CpuCanvas 1920x1080 디버그 빌드 실측 추가 — 프레임당 decide 0.28~0.56초, result 0.84~1.03초, musicselect 0.62~1.36초(1897 객체 중 약 100개 그림), play 1.05~1.12초. 빌드(디코드 포함) 0.17 / 1.36 / 1.62 / 1.52초


## 웨이브 2A 반영 사항 (2026-10-10)

Lua 5.2 런타임(`crates/rbms-skin/src/lua/`), `SkinHost`, Lua 값 변환기, 2패스 `.luaskin` 로더를 넣고 구 샌드박스(`skin.*`)를 삭제한 뒤의 상태다.

- (W2-0) r2-rbms-render.md §7.1 표(362행 `state` 행과 368행 트레이트 줄): `&dyn SkinStateSource` 를 `&dyn SkinHost` 로, 트레이트 목록을 `DrawStateSource::boolean(id) -> Option<bool>`, `SkinHost::{integer, rate, float, text, timer_us, now_us, ...}` 로. state.rs 의 다섯 어댑터가 `rate(id)` 를 `Some(float(id))` 로 답한다는 점 추가.
- (W2-1) r2-rbms-render.md §5.1 표 image 행과 상단 W1-4 반영 사항: animation_index 시그니처를 animation_index(count, now_us, timers, lua) 로, Sprite.timer 를 Option<TimerRef> 로
- (W2-1) r2-rbms-render.md §7.1 표(363행) lua 행: '컴파일된 식 평가(eval_draw, eval_integer, eval_float, eval_text)에 더해 상위 트레이트 LuaDrawEval 의 call_*/named_* 로 함수 값·이름을 평가. SkinFrame::script() 가 게이트·타이머용 평가기를 돌려준다' 로. ValueSource 설명이 있는 곳에 Function(LuaFnId)·Name(String) 변형과 Copy 상실을 추가
- (W2-5) r2-rbms-render.md §7(프레임·평가기 서술)과 SkinAssets 설명 절: `SkinExprEval` 트레이트와 `eval_integer/float/text`, `SkinAssets::expression`, `ValueSource::Expr` 가 삭제됨. `SkinFrame.lua`·`SkinDraw.lua` 는 `Option<&dyn LuaDrawEval>`. `ValueSource::new` 는 assets 인자를 받지 않음


## 웨이브 1B 반영 사항 (2026-10-10)

타이머 µs, stretch 11종, 장면 시계, 마우스 이벤트, GPU 논리 크기 런타임화와 색 공간, START/SELECT, 해상도 설정, 캡처 하니스를 넣은 뒤의 상태다.

- (W1-4) r2-rbms-render.md 상단 '웨이브 1A 반영 사항' 의 `new(view, now_ms, offsets, options_open)` 표기와 §7.1 표(335-336행 `now_ms` i64 프레임 시계)를 `now_us`(µs)로. 343행 트레이트 목록의 `SkinStateSource::{…, timer, now_ms}` 를 `{…, timer_us, now_us}` 로. 355행 '모든 어댑터의 `timer()` 는 `None`' 을 '`timer_us()` 는 `TIMER_OFF`' 로
- (W1-4) r2-rbms-render.md §5.1 표 image 행(243행): 셀 애니메이션 식 위치가 `object.rs` 의 `animation_index(count, now_us, timers)` 로 바뀌었고 시계·타이머를 각각 ms 로 절삭한다고 적는다
- (W1-4) r2-rbms-render.md §5.1 표 graph 행(249행): '소스도 같이 자름(draw.rs:364-387)' 의 구현이 픽셀 영역(`Sprite::region`)을 자른 뒤 `Placement::region` 으로 stretch 를 적용하는 순서로 바뀌었다. 정수 절삭 차이는 그대로 남음
- (W1-4) r2-rbms-render.md 252행 stretch 문단과 §10 표 D1 행(478행): '4종만 지원, 나머지는 경고 후 늘여 그린다' 를 '11종 전부 구현(W1-4). `Placement::region` 이 `stretch_rect` 가 돌려준 소스 사각형으로 UV(`Sprite::region_uv`)와 필터를 정한다. `is_supported` 와 빌드 경고 삭제' 로. D1 행은 해소로 표시
- (W1-7) r2-rbms-render.md §0 항목 3: '화면은 1280x720 논리 좌표에 고정' -> Gpu 논리 크기는 레터박스 뒤 뷰포트의 물리 픽셀이고, 내장 화면만 ScaledRenderer(Canvas)로 1280x720 좌표를 유지한다. 텍스트가 720p 기준 사각형이라는 서술은 내장 화면과 아직 Canvas 를 거치는 스킨 문서에 한해 유효.
- (W1-7) r2-rbms-render.md §0 항목 4: 'GPU 표면이 sRGB ... 실제 화면 확인은 하지 못했다' -> 해소. 오프스크린 실측으로 확인(sRGB 대상에서 CpuCanvas 대비 최대 73~75/255, 128 -> 188, (18,18,24) -> (75,75,86)). 표면은 Bgra8Unorm/Rgba8Unorm 을 우선 고르고, 없으면 sRGB 포맷을 비 sRGB 뷰로 렌더한다. 수정 뒤 차이는 최대 2/255.
- (W1-7) r2-rbms-render.md §1.1: Renderer 메서드 9개 -> 10개. max_texture_size() -> u32 추가(기본 구현 UNLIMITED_TEXTURE_SIZE = u32::MAX). register_texture 계약에 '한도를 넘는 이미지는 거부되고 돌려받은 핸들은 texture_size 가 None' 추가. lib.rs 줄 번호 전부 어긋남.
- (W1-7) r2-rbms-render.md §1.2: 보조 타입에 ScaledRenderer<'a, R>(new(inner, size), scale()), scale_between(from, to), scale_rect(rect, scale), UNLIMITED_TEXTURE_SIZE 추가.
- (W1-7) r2-rbms-render.md §1.3: '렌더 타깃/오프스크린: 없음' -> wgpu 쪽에 테스트 전용 오프스크린 대상과 읽기(Gpu::offscreen, Gpu::capture, Rgba8Unorm)가 있음.
- (W1-7) r2-rbms-render.md §1.4 '필터 선택' 행과 §4 의 1080p 충실도 항목 2: 1:1 판정이 논리 좌표라는 지적은 스킨 화면이 Canvas::native() 로 그리게 되면 해소된다(API 는 준비됨, 호출부 전환은 미완). 내장 배경 슬롯은 이미 물리 픽셀로 판정.
- (W1-7) r2-rbms-render.md §2 표: '논리 화면 CW/CH 상수' -> 뷰포트 크기(런타임), '유니폼 생성 시 한 번 기록' -> 뷰포트 크기가 바뀔 때마다 프레임 경계에서 갱신(Gpu::settle), 'size() 항상 (CW, CH)' -> viewport.size(), '표면 포맷 sRGB 우선' -> 비 sRGB 8비트 우선(choose_surface_formats), '배치 키 ... rotated' -> Textured { tex, blend, filter } + 클립, '뷰포트 16:9 고정 계산' -> 생성자 shape 비율 + 정수 픽셀 반올림, '마우스 역변환 logical_from_physical_in' -> position_in_space(x, y, space), '디바이스 한도 DeviceDescriptor::default(), 조회 없음' -> Limits::default().using_resolution(adapter.limits()) 로 요청하고 max_texture_dimension_2d 를 보관.
- (W1-7) r2-rbms-render.md §2 관찰: 'rotated 가 배치 키에 들어 있다' 삭제. 색 처리 문단은 '해소: 비 sRGB 대상, GPU == CpuCanvas(±2/255) 를 gpu/pixel_tests.rs 가 검증' 으로. 'GPU 백엔드에는 픽셀 테스트가 없다' -> gpu/pixel_tests.rs 11개(어댑터 없으면 건너뜀, RBMS_REQUIRE_GPU=1 이면 필수).
- (W1-7) r2-rbms-render.md §3.3 표: 'GPU 최대 텍스처 크기 확인: 없음' -> 있음(초과 시 경고 1회 후 거부, 빈 슬롯 핸들 반환).
- (W1-7) r2-rbms-render.md §3.4 해석 첫 항목: 'wgpu 기본 한도 8192 ... 미확인' -> 확인. wgpu 29 의 Limits::default() 는 max_texture_dimension_2d 8192 이고, 지금은 어댑터 한도로 디바이스를 만든다(이 기기 Metal 은 16384). ModernChic 최대 6400x3800 은 들어간다. 2048 한도 어댑터에서는 28장이 거부된다.
- (W1-7) r2-rbms-render.md §4 표: 'GPU 유니폼', 'GPU scissor', 'GPU size', '레터박스', '마우스 변환' 행은 해소(런타임 값). '창 생성 LogicalSize(CW, CH)' 는 유지(E6). 표 아래 '고칠 곳 제안' 3항목은 완료. 단 스킨 화면의 native 전환 호출부는 남음.
- (W1-7) r2-rbms-render.md §9: '불필요한 배치 분할 rotated' 행 삭제 또는 해소 표시.
- (W1-7) r2-rbms-render.md §10: A1, A2, A3, A5 완료. A4 는 한도 확인과 거부까지 완료, 'wgpu 오류 포착'은 미완. A3 의 실제 구현 위치는 rbms-render lib.rs 의 ScaledRenderer 와 stage/canvas.rs 의 Canvas, NativeCanvas, UI_SIZE.
- (W1-9) r2-rbms-render.md §215 부근 표('HeadlessCanvas::new(CW, CH) 다수'): HeadlessCanvas 가 임의 크기로 만들어져 capture.rs 에서 1920x1080 으로 쓰이고, HeadlessCanvas::rgba() 로 RGBA8 행을 그대로 내준다는 점을 추가해야 함.
- (리뷰 수정) r2-rbms-render.md §4·§10(A1~A5): 스킨 화면의 논리 크기 서술을 '선곡·결정·결과·키 설정 문서는 `Canvas::native()`(레터박스 뒤 물리 뷰포트) 위에 직접 그려 viewport 배율이 한 번만 걸린다. 플레이 문서만 W6 까지 1280x720 배율 래퍼 위에 그린다(note·cover 가 내장 `Skin` 의 720 공간 세로 지오메트리를 읽기 때문)'로 바꿔야 합니다. W1-7·W1-9 가 남긴 '두 번 배율' 메모는 플레이 화면 한정으로 좁혀야 합니다.


## 웨이브 1A 반영 사항 (2026-10-09)

혼합 합성과 구 번들을 삭제한 뒤의 상태다. 본문의 해당 절은 아래 내용으로 읽는다.

- (리뷰 수정) §8.1(내장 화면 렌더와 스킨 렌더의 결합 지점): `*_on_background` 계열과 lib.rs 재노출이 없어졌음을 반영. SkinScreen 공개 API 를 나열한 곳이 있으면 object_ids 를 빼고, SkinObject 가 문서 id 를 보관하지 않는다는 점(필드는 track, stretch, body)을 적습니다.
- (리뷰 수정) §7 부근(356~361행): SelectListState 필드 표를 `rows`, `sel`, `options_open` 으로 고쳐야 합니다(`detail` 과 `options` 는 없음). state.rs 행 번호(869-874, 887-898)도 현재 값(약 663-668, 686-692)으로 갱신.
- (W1-1b) r2-rbms-render.md §5.2(전용 객체 13종): densitygraph 객체(Body::Density, SkinObjectKind::Density, graphs.rs draw_density) 삭제. judge 객체는 judge.images 에 image/imageset 만 허용(text 분기 삭제). songlist 는 hot_rects·clickable 판정 삭제, 그리기만 남음.
- (W1-1b) r2-rbms-render.md §7.1: SelectViewState 설명에서 '자체 대역' 삭제, 생성자는 new(view, now_ms, offsets, options_open). ResultViewState·PlayViewState 의 자체 대역 문자열 삭제. SelectListState 필드 갱신. §7.2 4번(자체 id 대역 20_001~20_315) 항목 삭제.
- (W1-1b) r2-rbms-render.md §8.1(결합 지점 전수 표): 대체 플래그, 플래그 소비, 레이어(SkinLayer/draw_layer/draw_matching), 대체 요건 표, 대체 판정, 합성 질의, 레거시 필드 지오메트리(with_document_lanes/document_field), 핫스팟, 자체 id 대역 행을 삭제. '합성 방식' 행은 '모델에만 남음, 소비자 없음(W1-2 삭제 대상)'으로. '플레이/선택/결과 그리기' 행은 '문서가 그리면 반환, 아니면 내장' 2갈래로. '스킨 객체의 Skin 의존' 행에 '이제 내장 필드 그대로(문서로 덮지 않음)' 추가. '배경 슬롯 이중 경로' 는 문서 유무 2분기로.
- (W1-1b) r2-rbms-render.md §8.2: '걷어낼 것' 중 content.rs·ResultContent·*_with_content*, SkinLayer·draw_layer, hotspot·SkinHotAction, 자체 id 대역, with_document_lanes·document_field 는 완료. 남은 것은 SkinComposition·replace 모델 필드(W1-2)와 스킨 객체의 &Skin 의존(W6). ResultExtras 는 내장 결과 화면 입력으로 유지됨을 명시.
- (W1-1b) r2-rbms-render.md §1·§10: crates/rbms-render/src/lib.rs 의 재노출 목록에서 content 모듈, render_hud_with_content(_ctx), OptionsRows, SkinHotAction, SkinHotspot 삭제. render_select_screen 시그니처는 그대로(view 를 받음), render_result_screen 도 그대로.
- (W1-6) r2-rbms-render.md §6.1 마지막 줄 '패밀리 전환: set_family/reset_family 가 ... 캐시를 전부 비운다' 는 '레이아웃 캐시는 family->px->text 로 패밀리별 분기를 가지며 전환 시 아무것도 비우지 않는다. 런·아틀라스 캐시는 CacheKey 의 font_id 로 구분된다' 로 바뀌어야 합니다. 같은 절의 캐시 설명도 'px -> text -> Laid' 가 'family -> px -> text -> Laid' 로 바뀌었고 한도 512 는 전 패밀리 합계입니다.
- (W1-6) r2-rbms-render.md §6.2 성능 문제 절은 해소됨으로 표시하고(set_family/reset_family 의 캐시 전체 삭제 제거), 요약(§0)·권고(§10)의 해당 항목도 완료 처리해야 합니다.
- (W1-6) r2-rbms-render.md §6.3 폰트 로드 결함은 해소됨으로 바꿔야 합니다. load_font 는 fontdb load_font_source 가 돌려주는 새 face ID 로 성공을 판정하고 비폰트 바이트는 None 이며 fontdb 가 조용히 버리는 것도 실행 테스트로 확인되었습니다(미확인 문구 삭제). 이에 따라 skin_render/mod.rs:284-290 호출부는 .fnt 를 skin.fonts 로 받으면 이제 오류 분기로 갑니다.


조사 기준일 2026-10-09. 읽기 전용 조사. 경로는 별도 표기가 없으면 `/Users/hyunseokbyun/development/R-BMS` 기준이고, `beatoraja:` 접두는 `/Users/hyunseokbyun/development/beatoraja/src/bms/player/beatoraja` 기준이다.

## 0. 결론 요약

1. 그리기 프리미티브(사각형, 텍스처 쿼드, UV 부분 영역, 회전과 회전 중심, tint, 4종 블렌드, nearest/linear, 스택형 클립)는 CPU 캔버스와 wgpu 양쪽에 모두 구현돼 있고 레퍼런스의 `SkinObjectRenderer` 가 실제로 설정하는 블렌드 상태와 일치한다. 프리미티브 자체는 ModernChic 를 그리기에 대체로 충분하다.
2. 부족한 것은 프리미티브가 아니라 그 위 계층이다: 비트맵 폰트(.fnt)와 distance field 텍스트, 텍스트 overflow/wrapping/shadow/outline, 동영상 소스, 참조 이미지(-100/-101/-102/-110/-111), 클릭 이벤트, trimmed stretch 4종, 텍스처 지연 로드와 수명 관리, 화면 해상도 일반화, 상태(id) 공급 구조.
3. 화면은 1280x720 논리 좌표에 고정돼 있다. 텍스처 쿼드는 물리 해상도로 래스터되지만 텍스트는 논리 720p 픽셀 단위 사각형으로 그려져 1080p 스킨에서 흐려진다.
4. GPU 표면이 sRGB 포맷으로 선택되고 텍스처는 `Rgba8Unorm`, 색은 바이트/255 를 그대로 선형값으로 넘긴다. 코드상 PNG 바이트가 감마 변환을 한 번 더 거쳐 밝게 나오고 블렌딩이 선형 공간에서 일어난다. 레퍼런스(libGDX 기본 프레임버퍼, 바이트 통과)와 색이 달라지는 지점이다. 실제 화면 확인은 하지 못했다(코드 근거만).
5. 스킨 텍스트 경로는 폰트 패밀리를 바꿀 때마다 텍스트 캐시 전체를 비운다. 스킨 폰트를 하나라도 쓰면 매 프레임 재셰이핑·재래스터가 일어난다.
6. ModernChic 의 PNG 284장을 전부 RGBA8 로 올리면 약 2.79 GiB 다. 현재 구현은 문서가 선언한 `source` 를 전부 선행 로드하고 화면 전환 때 해제하지 않는다. 레퍼런스는 객체가 참조한 소스만 지연 로드한다.
7. 속성 id 공급은 내장 화면용 뷰 구조체(`HudView`/`SelectView`/`ResultView`)를 `match` 로 감싼 어댑터다. 선언된 id 약 800개 중 90여 개만 답한다. 풀 스킨에는 구조 자체를 바꿔야 한다.

## 1. Renderer 트레이트와 프리미티브

### 1.1 트레이트 정의

`crates/rbms-render/src/lib.rs:315-354`. 메서드 9개.

| 메서드 | 의미 | 근거 |
|---|---|---|
| `size() -> (u32, u32)` | 논리 화면 크기 | lib.rs:316 |
| `clear(color)` | 전체 지우기 + 클립 스택 비움 + 이번 프레임 제출분 폐기 | lib.rs:318-325 |
| `fill_rect(rect, color)` | 단색 사각형, 알파 블렌드만 | lib.rs:327 |
| `register_texture(key, rgba, w, h) -> TextureId` | RGBA8 비프리멀티플라이드 등록. 같은 key 는 픽셀 교체 후 같은 핸들 반환. 길이 부족은 투명 검정 패딩, 초과는 절단 | lib.rs:329-339 |
| `release_texture(tex)` | 해제. 모르는 핸들 무시 | lib.rs:341-342 |
| `texture_size(tex)` | 텍스처 픽셀 크기 | lib.rs:344 |
| `draw_textured_quad(tex, QuadParams)` | 텍스처 쿼드 | lib.rs:346 |
| `push_clip(rect)` / `pop_clip()` | 논리 좌표 클립. 중첩은 교집합 | lib.rs:348-353 |

계약: 제출 순서가 z 순서이며 백엔드는 인접 그리기를 합칠 수 있어도 재정렬하지 않는다(lib.rs:313-314).

### 1.2 보조 타입

| 타입 | 필드/값 | 근거 |
|---|---|---|
| `Color` | `r,g,b,a: u8` | lib.rs:47-53 |
| `Rect` | `x,y,w,h: f32`, `intersect` | lib.rs:71-93 |
| `UvRect` | `u0,v0,u1,v1` 정규화, `FULL`, `from_pixels(x,y,w,h,tex_w,tex_h)` | lib.rs:213-234 |
| `TextureId(u32)` | 재사용하지 않는 핸들 | lib.rs:236-239 |
| `QuadParams` | `dst: Rect`, `src: UvRect`, `tint: Color`, `blend: BlendMode`, `filter: TextureFilter`, `angle_deg: f32`(화면 시계 방향 양수), `center: (f32,f32)`(dst 좌상단 기준 픽셀) | lib.rs:242-258 |
| `BlendMode` | `Alpha`(기본), `Add`, `Multiply`, `InvertDst` | lib.rs:133-144 |
| `BlendFactor` | `Zero, One, SrcAlpha, OneMinusSrcAlpha, SrcColor, OneMinusDstColor` | lib.rs:106-114 |
| `TextureFilter` | `Nearest`(기본), `Linear` | lib.rs:202-209 |

블렌드 매핑 `BlendMode::from_skin_blend`(lib.rs:149-156): 2 → Add, 3 → Add, 4 → Multiply, 9 → InvertDst, 그 외 전부 Alpha.

블렌드 계수 `BlendMode::factors`(lib.rs:164-185):

| 모드 | src_color | dst_color | src_alpha | dst_alpha |
|---|---|---|---|---|
| Alpha | SrcAlpha | OneMinusSrcAlpha | One | OneMinusSrcAlpha |
| Add | SrcAlpha | One | Zero | One |
| Multiply | Zero | SrcColor | Zero | One |
| InvertDst | OneMinusDstColor | Zero | Zero | One |

회전 중심 `skin_center_offset(center, w, h)`(lib.rs:280-295): 표 `SKIN_CENTER_X = [0.5,0,0.5,1,0,0.5,1,0,0.5,1]`, `SKIN_CENTER_Y = [0.5,0,0,0,0.5,0.5,0.5,1,1,1]`(y-up 기준). 반환은 `(X[i]*w, (1-Y[i])*h)`. 0..9 범위 밖은 0(중앙)으로 처리.

`background_filter(dst, source)`(lib.rs:305-308): dst 크기가 소스 픽셀 크기와 정확히 같으면 Nearest, 아니면 Linear.

### 1.3 백엔드별 구현 여부

| 프리미티브 | CpuCanvas | wgpu(Gpu) | 비고 |
|---|---|---|---|
| 단색 사각형 | 구현. source-over, 결과 알파를 255 로 강제. 좌표는 floor..ceil 정수화 (cpu.rs:291-316) | 구현. 인스턴스 1개, `ALPHA_BLENDING` 파이프라인 (gpu/mod.rs:233, 537-539) | 사각형에는 블렌드 모드·회전 없음 |
| 텍스처 쿼드 + UV 부분 영역 | 구현 (cpu.rs:345-379, uv_at 271-274) | 구현 (gpu/mod.rs:40-67, 584-591) | |
| tint(색·알파 곱) | 구현 `apply_tint` (cpu.rs:25-28) | 프래그먼트 `textureSample * tint` (gpu/mod.rs:66) | |
| 회전 + 회전 중심 | 구현 `QuadMap`, 픽셀 중심 샘플링 (cpu.rs:206-268) | 버텍스 셰이더에서 `(cos, sin, cx, cy)` 로 회전 (gpu/mod.rs:55-57, batch.rs:47-56) | 각도 0 이면 회전 경로 생략 |
| 블렌드 4종 | 구현 `apply_blend` (cpu.rs:55-63) | 모드별 파이프라인 4개 (gpu/mod.rs:70-92, 279-308) | 같은 계수표 공유 |
| 감산 블렌드 | 없음(의도적) | 없음 | 레퍼런스가 blend 3 을 가산으로 그리기 때문 (lib.rs:128-132) |
| 필터 nearest/linear | 구현, 가장자리 clamp (cpu.rs:176-203) | 샘플러 2개, 텍스처당 바인드 그룹 2개 (gpu/mod.rs:95-101, 315-322, 459) | 밉맵 없음(`mip_level_count: 1`, gpu/mod.rs:485) |
| 클립 | 스택, 교집합, 정수 반올림 (cpu.rs:108-119, 381-393) | 배치별 scissor (batch.rs:70-90, 179-194; gpu/mod.rs:431-437) | 클립은 축 정렬 사각형만 |
| 텍스트 | 프리미티브 아님. `TextContext` 가 글리프를 가로 런 단위 `fill_rect` 로 내보냄 (font.rs:193-232) | 동일(백엔드 무관) | 아틀라스 경로 `draw_text_atlas` 는 구현돼 있으나 테스트에서만 호출 (font.rs:338-369; 사용처 crates/rbms-render/tests/primitives.rs:535-711) |
| 선·다각형·그라디언트 | 없음 | 없음 | 그래프는 1px 사각형으로 대체 (skin_render/graphs.rs:6-8) |
| 셰이더 교체(distance field, ffmpeg, bilinear) | 없음 | 없음 | 레퍼런스는 셰이더 6슬롯 (beatoraja:skin/Skin.java:529-558) |
| 렌더 타깃/오프스크린 | 없음 | 없음 | |

### 1.4 레퍼런스 대비 프리미티브 의미

| 항목 | 레퍼런스 | R-BMS | 판정 |
|---|---|---|---|
| blend 2 | `(SRC_ALPHA, ONE)` (Skin.java:656) | Add | 일치 |
| blend 3 | SUBTRACT 설정 후 곧바로 FUNC_ADD 복귀, 결과적으로 가산 (Skin.java:657-662) | Add | 일치 |
| blend 4 | `(ZERO, SRC_COLOR)` (Skin.java:663) | Multiply | 일치 |
| blend 9 | `(ONE_MINUS_DST_COLOR, ZERO)` (Skin.java:664) | InvertDst | 일치 |
| 그리기 후 복귀 | blend >= 2 면 알파 블렌드로 복귀 (Skin.java:680-682) | 쿼드별 모드 | 일치 |
| 회전 | `sprite.draw(image, x, y, cx*w, cy*h, w, h, 1, 1, angle)` (Skin.java:620-626), 각도 0 이면 비회전 경로 (SkinObject.java:638-642) | 로더가 부호를 뒤집어 화면 시계 방향으로 전달 (lib.rs:252-254) | 일치 |
| 회전 중심 표 | `CENTERX/CENTERY` (SkinObject.java:80-81), 0 이 아닌 첫 center 만 채택 (SkinObject.java:229-233) | 같은 표 | 일치(첫 값 채택 여부는 rbms-skin 로더 범위, 미확인) |
| 알파 0 생략 | `color.a == 0f` 면 그리지 않음 (SkinObject.java:624, 651) | `resolved.color.a == 0` 이면 반환 (skin_render/draw.rs:149-151) | 일치 |
| 필터 선택 | `dstfilter != 0` 이고 dst 크기가 region 크기와 다르면 BILINEAR 셰이더, 같으면 NORMAL (SkinObject.java:634-636) | `filtering_for`: filter 0 또는 1:1 이면 Nearest, 아니면 하드웨어 Linear (crates/rbms-skin/src/loader/stretch.rs:124-128) | 근사. 1:1 판정을 물리 픽셀이 아닌 1280x720 논리 좌표에서 함(4장 참고) |
| 좌표 미세 보정 | x,y 에 +0.01 (Skin.java:616, 624) | 없음 | 무시 가능 |
| 전체 오프셋 변환 | 플레이 스킨은 `OFFSET_ALL` 로 배치 행렬 이동·확대 (Skin.java:382-389, 720-729) | 없음 | 누락 |

## 2. GPU 백엔드 상세 (apps/rbms-player/src/gpu)

| 항목 | 값 | 근거 |
|---|---|---|
| 논리 화면 | `CW = 1280`, `CH = 720` 상수 | apps/rbms-player/src/lib.rs:134-135 |
| NDC 변환 | 유니폼 `screen = (CW, CH)` 를 생성 시 한 번 기록 | gpu/mod.rs:198, 30, 58 |
| `Renderer::size()` | 항상 `(CW, CH)` | gpu/mod.rs:525-527 |
| 표면 포맷 | 지원 포맷 중 sRGB 를 우선 선택 | gpu/mod.rs:179 |
| 텍스처 포맷 | `Rgba8Unorm`, 밉 1단, 샘플 1 | gpu/mod.rs:482-491 |
| present | `AutoVsync` | gpu/mod.rs:185 |
| 인스턴스 버퍼 | 초기 8192개, 넘치면 2의 거듭제곱으로 재할당 | gpu/mod.rs:108, 380-397 |
| 인스턴스 크기 | 단색 32바이트(`rect`,`color`), 텍스처 64바이트(`rect`,`uv`,`tint`,`rotation`) | gpu/batch.rs:21-26, 37-44 |
| 배치 키 | `Colored` 또는 `Textured { tex, blend, filter, rotated }` + 클립 상태 | gpu/batch.rs:61-65, 158-168 |
| 드로콜 | 배치당 `draw(0..6, first..first+count)` 1회 | gpu/mod.rs:445-464 |
| 뷰포트 | 늘이기(기본) 또는 레터박스(16:9 고정 계산) | gpu/mod.rs:612-620 |
| 마우스 역변환 | `logical_from_physical_in` 가 CW/CH 로 환산 | gpu/mod.rs:624-627 |
| 텍스처 등록 | 같은 key·같은 크기면 제자리 업로드, 크기가 다르면 새 텍스처로 교체 | gpu/mod.rs:541-564 |
| 업로드 행 정렬 | 행 바이트가 256 배수가 아니면 전체 복사 후 패딩 | gpu/batch.rs:203-226 |
| 배경 슬롯 | `clear` 시 배경 쿼드를 먼저 큐에 넣음. generation 으로 재업로드 생략 | gpu/mod.rs:529-535, gpu/background.rs:23-64 |
| 디바이스 한도 | `DeviceDescriptor::default()`. 한도 조회·오류 포착 코드 없음 | gpu/mod.rs:176 (한도 관련 grep 결과 없음) |

관찰:

- `rotated` 가 배치 키에 들어 있지만 회전 여부와 무관하게 같은 파이프라인을 쓴다(gpu/mod.rs:454-460). 회전 쿼드가 섞이면 배치만 끊긴다.
- 색 처리: 단색·tint·clear 색 모두 `바이트/255` 를 그대로 넘긴다(batch.rs:30-31, 48-53; mod.rs:416). 표면이 sRGB 이면 이 값이 선형으로 해석되어 기록 시 감마 인코딩된다. 텍스처도 `Rgba8Unorm` 이라 샘플 값이 선형으로 취급된다. 따라서 PNG 의 128 은 화면에 약 188 로 나오고, 알파·가산 블렌딩은 선형 공간에서 계산된다. CpuCanvas 는 바이트 그대로 계산하므로(cpu.rs:42-46) GPU 출력과 CPU 골든이 색에서 일치하지 않는다. 레퍼런스는 libGDX 기본 프레임버퍼에 바이트를 통과시킨다(Skin.java 어디에도 sRGB 설정 없음). 이 항목은 코드 근거이며 실제 화면 비교는 미확인.
- GPU 백엔드에는 픽셀 테스트가 없다. gpu/mod.rs 와 batch.rs 의 테스트는 뷰포트·scissor·배치·패딩 계산만 검증한다(gpu/mod.rs:639 이하, batch.rs:228 이하).

## 3. 텍스처 수명 관리

### 3.1 로드 경로

1. 화면 진입 시 `AppShared::prepare_skin` → `SkinScreens::sync` (apps/rbms-player/src/skin_screen.rs:573-583, 483-525).
2. 처음 보는 빌드면 `PendingScreen::start` 가 `document.sources` 전부와 `document.fonts` 전부를 디코드 잡으로 만든다(skin_screen.rs:427-434).
3. 워커 풀에서 `image::open(path).to_rgba8()` (apps/rbms-player/src/assets.rs:610-616). `image` 크레이트 기능은 png, bmp, jpeg 뿐(apps/rbms-player/Cargo.toml:36).
4. 전부 도착하면 `SkinScreen::build` 가 소스마다 `register_texture("rbms.skin.{serial}.source.{id}")` (crates/rbms-render/src/skin_render/mod.rs:266-282).
5. 디코드 실패는 경고 한 줄을 남기고 해당 소스를 쓰는 객체가 빠진다(mod.rs:280).

### 3.2 해제

- `SkinScreen::release` 가 보유 텍스처를 전부 반납(mod.rs:335-340).
- 호출 시점은 같은 화면 종류의 빌드 번호가 바뀔 때뿐(skin_screen.rs:486-489).
- `SkinScreens.built` 는 화면 종류별로 하나씩 계속 보유한다(skin_screen.rs:462-465). 선택 화면 문서는 플레이 중에도 업로드된 채 남는다(주석 skin_screen.rs:458-460).

### 3.3 한도·아틀라스

| 항목 | 현황 | 근거 |
|---|---|---|
| 이미지 크기 상한 | 없음. `SkinImage::new` 는 0 크기와 길이 불일치만 거른다 | skin_render/mod.rs:94-97 |
| GPU 최대 텍스처 크기 확인 | 없음 | gpu/mod.rs:176, 481-504 |
| 스킨 이미지 아틀라스 | 없음. 소스 1개 = 텍스처 1개 + 바인드 그룹 2개 | gpu/mod.rs:95-101 |
| 글리프 아틀라스 | 단일 페이지 2048 폭, 256 에서 시작해 2048 까지 배증, 가득 차면 비우고 재시작 | glyph_atlas.rs:27-30, 185-206, 165-171 |
| 압축 텍스처·밉맵 | 없음 | gpu/mod.rs:485-488 |
| 메모리 예산·계측 | 없음 | 해당 코드 없음 |

### 3.4 ModernChic 수치 (PNG 헤더 실측)

측정 스크립트: scratchpad/skin-research/tools/r2_png_stats.py (PNG IHDR 만 읽음).

| 지표 | 값 |
|---|---|
| PNG 수 | 284 |
| 전부 RGBA8 로 올릴 때 | 2,930,584,516 바이트 (약 2,794.8 MiB) |
| 최대 폭 / 최대 높이 | 6400 / 3800 |
| 한 변이 2048 초과 | 28장 |
| 한 변이 4096 초과 | 8장 |
| 한 변이 8192 초과 | 0장 |
| 디렉터리별 | Play 1068.1 MiB, Select 808.6 MiB, Result 585.9 MiB, Decide 306.4 MiB, SkinSelect 15.9 MiB, Root 9.8 MiB |

큰 파일: `Result/parts/prepare.png` 3920x3800(56.8 MiB), `Play/parts/common/fullcombo/#default.png` 5190x2571(50.9 MiB), `Select/parts/{jp,en,cn}/mainframe.png` 3200x3200(각 39.1 MiB), `Play/parts/common/bomb/*.png` 6400x1200(각 29.3 MiB, 7종).

해석:

- 6400 폭은 wgpu 기본 한도로 알려진 8192 안쪽이지만, 그 한도값은 이번 조사에서 소스로 확인하지 못했다(미확인). 코드가 한도를 조회하지 않으므로 다운레벨 어댑터(2048 한도)에서는 검증 오류가 난다.
- Select 는 jp/en/cn 3벌이 들어 있고 bomb 는 7종 중 하나만 쓰이므로, 실제 화면별 필요량은 위 합계보다 작다. 화면별 실제 `source` 목록은 Lua 실행 결과에 달려 있어 이 조사에서는 확정하지 못했다(미확인).
- 레퍼런스는 `getSource` 를 객체 생성 시점에 호출해 참조된 소스만 로드한다(beatoraja:skin/json/JSONSkinLoader.java:481-519, JsonSkinObjectLoader.java:44). 현재 R-BMS 는 선언된 소스를 전부 디코드·업로드한다(skin_screen.rs:428). 선언만 하고 조건부로 쓰는 소스가 많은 풀 스킨에서 차이가 커진다.
- 디코드 경로의 일시 메모리: 디코드 버퍼(Vec) → `register_texture` 로 전달 → 행 바이트가 256 배수가 아니면 패딩 복사본 1벌 추가(batch.rs:215-225). 6400x1200 은 25600 바이트/행으로 정렬돼 복사가 없고, 5190 폭은 복사가 생긴다.

### 3.5 동영상·동적 텍스처

- 동영상 디코더 의존성이 워크스페이스에 없다(Cargo.lock 에 영상 코덱 크레이트 없음. symphonia-format-isomp4 는 오디오 디먹서).
- BGA 도 정지 이미지만 디코드한다(assets.rs:654-663).
- 동적 텍스처 갱신 통로는 있다: 같은 key·같은 크기 재등록이 제자리 업로드다(gpu/mod.rs:542-549). 1920 폭 프레임은 7680 바이트/행으로 256 배수라 패딩 복사가 없다.
- ModernChic 는 mp4 8개(6 MB~48 MB)를 Select/Decide/Play 배경 선택지로 쓴다(ModernChic/decide.lua:236, Select/lua/background.lua:19-21, Play/lua/base.lua:231, 270).
- 레퍼런스: 소스 경로 확장자가 영상이면 `SkinSourceMovie`(FFmpegProcessor)로 만들고, `SkinImage` 가 `TYPE_FFMPEG` 셰이더로 그린다(JSONSkinLoader.java:495-510, SkinSourceMovie.java:29-55, SkinImage.java:71-75, 143-151). 첫 `getImage` 에서 루프 재생을 시작하고 프레임 텍스처가 없으면 그 프레임은 그리지 않는다.

## 4. 해상도 — 1280x720 고정 전제의 위치

| 위치 | 내용 | 근거 |
|---|---|---|
| 앱 상수 | `CW`/`CH` | apps/rbms-player/src/lib.rs:134-135 |
| 창 생성 | `LogicalSize::new(CW, CH)` | lib.rs:997 |
| GPU 유니폼 | 생성 시 1회 기록, 이후 갱신 없음 | gpu/mod.rs:198 |
| GPU scissor | `scissor_rect(..., (CW, CH), surface)` | gpu/mod.rs:432 |
| GPU size | `(CW, CH)` | gpu/mod.rs:525-527 |
| 레터박스 | CW:CH 비율로 계산 | gpu/mod.rs:617-618 |
| 마우스 변환 | CW/CH | gpu/mod.rs:626 |
| 레거시 필드 지오메트리 | `Skin::build(&cfg, mode, CW, CH)`, RON 의 세로 값은 1280x720 픽셀 | app_input.rs:288, crates/rbms-render/src/skin.rs:60-63, 138-159 |
| 문서 레인 → 화면 | `SkinViewport::new(authored, (CW, CH))` | app_input.rs:308 |
| HUD | `CW_REFERENCE = 1280.0` | hud.rs:20, 125, 229 |
| 선택 화면 레이아웃 검증 | `SELECT_CANVAS_WIDTH/HEIGHT` | theme.rs:15-16, 72-76, 138-139 |
| 결과 화면 | 고정 좌표 상수(`CONTENT_RIGHT = 1236` 등) | result.rs:206-222, 399 |
| 설정·키설정·로딩·분석 오버레이 | CW/CH 직접 사용 | settings_view.rs:10-11, stage/keyconfig.rs:191, stage/loading.rs:487-488, stage/play/mod.rs:666-685, app_options.rs:279, lib.rs:954 |
| 테스트 | `HeadlessCanvas::new(CW, CH)` 다수 | stage/render_tests*.rs 등 |

스킨 쪽은 이미 일반화돼 있다. `SkinViewport::new(authored, r.size())` 가 문서 좌표(y-up, 좌하단 원점)를 화면 좌표(y-down)로 뒤집고 x/y 를 독립 배율로 늘인다(skin_render/mod.rs:168-211, 411-412). 따라서 1920x1080 문서는 지금도 1280x720 논리 화면에 2/3 로 축소돼 들어간다.

1080p 충실도를 막는 것:

1. 텍스트 래스터가 논리 픽셀 기준이다. `px_for(scale) = round(scale * 8.5).max(8)` 로 정수 픽셀 크기를 정하고(font.rs:26-28), 글리프 원점을 정수로 반올림하며(font.rs:201), 글리프를 높이 1 논리 픽셀짜리 사각형 런으로 낸다(font.rs:221-227). 1080p 창에서는 2/3 크기로 래스터한 글자를 1.5배 사각형으로 키운 결과가 된다.
2. 필터의 1:1 판정이 논리 좌표에서 이뤄진다(skin_render/draw.rs:104-107). 레퍼런스는 창 해상도로 미리 곱한 좌표에서 판정한다(Skin.java:114-115, 135-141; SkinObject.java:634-636).
3. 노트 위치가 레거시 `Skin` 의 화면 픽셀(`judge_y`, `top_y`)에 묶여 있다(skin_render/notes.rs:170-171, 249-250).

레퍼런스의 해상도 모델: `dw = dst.width / org.width`, `dh = dst.height / org.height`(Skin.java:114-115)를 모든 destination 좌표에 곱해 창 해상도 좌표로 저장한다(Skin.java:135-189). 숫자 간격은 `space * dw`(SkinNumber.java:126-128, 209-211), 비트맵 폰트 크기는 `text.size * dstr.width / sk.w`(JsonSkinObjectLoader.java:652), 슬라이더 range 는 방향에 따라 dw 또는 dh(JsonSkinObjectLoader.java:402-412).

고칠 곳 제안(8장 변경 단위와 연결):

- Gpu 의 논리 크기를 런타임 값으로 바꾼다: 유니폼 갱신, `size()`, scissor, 레터박스 비율, 마우스 역변환(gpu/mod.rs:198, 432, 525-527, 617-618, 626).
- 스킨 화면은 논리 크기 = 물리 뷰포트 크기로 그린다. 그러면 `SkinViewport` 가 레퍼런스의 dw/dh 와 같은 일을 하고 필터 판정·텍스트 래스터가 물리 픽셀에서 이뤄진다.
- 스킨이 없는 내장 화면과 오버레이(토스트, 디버그, 대화상자)는 1280x720 좌표를 유지하되 좌표를 배율로 곱해 넘기는 래퍼 렌더러를 거친다. 한 프레임에 유니폼이 하나뿐이므로(gpu/mod.rs:428) 논리 공간 두 개를 섞으려면 이 방식이 가장 작다.

## 5. skin_render 객체별 구현 상태

객체 종류는 문서 레코드 기준 21종이고 그리기 본체 열거형은 20개다(`imageset` 이 `Image` 본체로 합쳐짐). `SkinObjectKind`(skin_render/object.rs:50-72), 디스패치(draw.rs:161-181), 빌드 순서(object.rs:561-626).

공통 처리(draw.rs:142-187): `prepare` 로 위치·색·각도·클립 해석 → 알파 0 이면 생략 → 클립 push → 본체 그리기 → pop. 배치는 `Placement::cell`(draw.rs:104-113): 뷰포트 배치 → stretch 적용 → 필터 결정 → 쿼드.

### 5.1 기본 6종 + 배경

| 종류 | R-BMS 구현 | 레퍼런스 대비 차이·누락 |
|---|---|---|
| image | `len > 1` 이면 셀을 len 그룹으로 나눠 `ref` 정수로 선택(object.rs:629-640). 셀 애니메이션 `(time*count/cycle) % count`(object.rs:189-204). | (1) 선택값이 음수면 레퍼런스는 그리지 않는다(SkinImage.java:128-131). R-BMS 는 `.max(0)` 으로 0번을 그린다(draw.rs:191). (2) 레퍼런스는 `getImageIndexProperty(ref)` 를 쓴다(SkinImage.java:48-50). R-BMS 는 일반 정수 속성으로 읽는다(object.rs:638). 두 팩토리의 차이는 미확인. (3) `act`/`click` 클릭 이벤트 미구현. 모델에는 필드가 있다(crates/rbms-skin/src/model/objects.rs:28-29). 레퍼런스는 JsonSkinObjectLoader.java:67-70. (4) 셀 타이머가 정수 id 일 때만 적용되고 식(함수) 타이머는 버려진다(object.rs:524-526). (5) 동영상 소스 미지원. |
| imageset | 각 이미지의 스프라이트를 변형으로 삼고 `value` 또는 `ref` 로 선택(object.rs:573-587). | 클릭 이벤트 미구현. 변형별 타이머는 스프라이트에 보존됨. |
| value(숫자) | 셀 수로 레이아웃 결정: 24 배수 → 12글리프 + 음수 절반, 10 배수 → 10글리프, 그 외 11글리프(object.rs:387-393). 자리 계산 `integer_glyphs`(object.rs:770-805). 정렬 이동 `digit_shift`(draw.rs:218-224). 자리별 offset(draw.rs:251-252). `i32::MIN/MAX` 는 그리지 않음(draw.rs:264-266). | 계산식은 SkinNumber.java:162-207 과 같다. 차이: 자릿수를 1..16 으로 제한(object.rs:653, 47). 자리별 offset 을 뷰포트로 스케일하는데 레퍼런스가 offset 을 dw/dh 로 스케일하는지는 미확인. |
| floatvalue | 26/24/22/12/11 셀 레이아웃(object.rs:401-432), `fraction_glyphs`(object.rs:808-850), gain 곱(draw.rs:290). | 레퍼런스 SkinFloat/FloatFormatter 는 이번 배정 목록 밖이라 대조하지 않았다(미확인). 코드 주석은 해당 클래스 기준으로 작성됨. |
| text | TTF/OTF 만. 높이 = dst 높이, 정렬 0 왼쪽/1 가운데/2 오른쪽 기준점은 dst.x (draw.rs:316-336). | 6장 참고. size, overflow, wrapping, shadow, outline, .fnt, fallback, editable 전부 미구현. |
| slider | 방향 0 위/1 오른쪽/2 아래/3 왼쪽, `travel = ratio * range`(draw.rs:348-360). | 그리기는 SkinSlider.draw 와 일치(beatoraja:skin/SkinSlider.java:113-118). `changeable` 마우스 조작 미구현(레퍼런스 mousePressed, SkinSlider.java:120 이하). |
| graph(바) | 방향 1 은 아래에서 위로 드러남, 그 외 왼쪽에서 오른쪽. 소스도 같이 자름(draw.rs:364-387). | SkinGraph.draw 와 일치(beatoraja:skin/SkinGraph.java:97-106). 레퍼런스는 소스 영역을 정수 픽셀로 절삭하고 R-BMS 는 연속값. |
| bga | 프레임이 넘긴 배경 텍스처를 그 자리에 그림(draw.rs:200-211). | 레퍼런스 SkinBGA(`bgaExpand`, 레이어, 미스 레이어)는 배정 범위 밖(미확인). 동영상 BGA 없음. |

stretch: `StretchKind` 11종 중 `Stretch`, `FitInner`, `FitOuter`, `NoResize` 4종만 지원으로 표시되고 나머지는 경고 후 늘여 그린다(crates/rbms-skin/src/loader/stretch.rs:61-63, object.rs:548-551). 레퍼런스는 11종 전부 구현하며 trimmed 4종은 소스 영역을 정수로 잘라낸다(beatoraja:skin/StretchType.java:16-104, 131-153). R-BMS `stretch_rect` 는 FitInner(1), FitOuter(2), NoResize(9)만 계산하고 나머지는 `_ => rect` 로 그대로 늘인다(stretch.rs:82-107). 미구현 7종: FitOuterTrimmed(3), FitWidth(4), FitWidthTrimmed(5), FitHeight(6), FitHeightTrimmed(7), NoExpanding(8), NoResizeTrimmed(10). id 매핑은 stretch.rs:40-54.

### 5.2 플레이·선택·결과 전용 13종

아래 종류의 레퍼런스 클래스(LaneRenderer, SkinGauge, SkinJudge, SkinBar, 각 그래프)는 이번 배정 비교 목록에 없어 직접 대조하지 않았다. "자체 선언 차이"는 R-BMS 소스 주석이 스스로 밝힌 차이다.

| 종류 | R-BMS 구현 | 자체 선언 차이·구조적 한계 |
|---|---|---|
| note | 레인별 `note`, `lnstart`, `lnend`, `lnbody`, `lnactive`, `mine`, `hidden` 스프라이트(notes.rs:43-55). 위치는 `visible_offsets`/`constant_offsets` + 레거시 `Skin` 의 `judge_y`/`top_y`(notes.rs:192-208, 170-171). 레인 클립(notes.rs:221). 마디선은 `note.group` 트랙(notes.rs:311-336). | expansion rate 는 박자 펄스 없이 상수 적용, `processed` 이미지 미구현(notes.rs:12-17). HCN 계열 이미지 필드가 구조체에 없다(notes.rs:43-55). 프레임에 플레이 상태가 없으면 그리지 않음(notes.rs:192-194). |
| gauge | 노드 4/8/12/36개를 36칸 표로 전개(gauge.rs:50-80, 165-201). 채움은 `FLOAT_GROOVEGAUGE_1P` 만 읽음(gauge.rs:250). | 무작위 애니메이션을 프레임 시계의 결정적 함수로 대체(gauge.rs:10-12, 207-219). 결과 화면에서는 그리지 않음(gauge.rs:235-248). 클리어 경계는 레거시 `Skin.gauge_clear_threshold`(gauge.rs:252). |
| judge | `images`/`numbers` 중첩 트랙, 콤보 숫자는 `NUMBER_COMBO` 로 강제(judge.rs:101-104), shift 는 콤보 폭의 절반(judge.rs:203-204). | `images` 에 text 도 허용하는 확장(judge.rs:11-16). 판정을 필드별이 아닌 런 하나로 처리. 플레이어 인덱스 2 이상은 두 번째로 접음(judge.rs:143-149). |
| songlist | 슬롯 고리, 슬롯 i 는 `rows[sel - center + i]`(songlist.rs:90-97). 바는 이미지 셀 0 고정(songlist.rs:43, 222), 제목·레벨·라벨은 텍스트, 램프는 단색 사각형(songlist.rs:267-278, 303-307). | 바 종류별 이미지 선택, 램프 이미지, 레벨 숫자 이미지, trophy, rivallamp, 분포 그래프가 없다. `rivallamp`/`trophy` 트랙은 길이 계산에만 쓰인다(songlist.rs:161-163). 바 애니메이션 없음. 내용은 내장 `SelectRow` 의 문자열·색에서 온다(select.rs:24-38). |
| hiddenCover / liftCover | 덮는 높이를 계산해 자르고 `disapearLine` 은 scissor 로 처리(covers.rs:116-173). | 위에서 내려오는 레인커버는 이 레코드가 아니며 `OFFSET_LANECOVER` 로 움직이는 일반 이미지다. 상태 쪽은 오프셋을 공급한다(state.rs:270-281). covers.rs:12-15 의 "오프셋 미공급" 주석은 state.rs 와 어긋난다. |
| gaugegraph | 배경 + 샘플당 1x2 사각형 + 테두리(graphs.rs:321-347). | 6개 게이지 색 중 groove 쌍만 사용(graphs.rs:143-154). |
| judgegraph | 판정별 막대 또는 누적 한 기둥(graphs.rs:350-404). | 레퍼런스의 초 단위 열 그래프가 아님. 색은 내장 팔레트(graphs.rs:156-168, 259). |
| bpmgraph | 계단선(graphs.rs:424-458). | 결과 프레임에서만 데이터가 온다(`FrameExtra::Result`, graphs.rs:432). 선택 화면에서는 그려지지 않는다. |
| timingdistributiongraph | 히스토그램 + 평균·편차선(graphs.rs:461-494). | |
| timingvisualizer | 판정별 최대 오차 밴드(graphs.rs:503-534). | 판정 윈도가 아니라 런에서 관측된 최대 오차를 그림(graphs.rs:192-196). |
| hiterrorvisualizer | 최근 히트 세로선 + EMA(graphs.rs:538-567). | |
| densitygraph | 초당 노트 수 히스토그램(graphs.rs:578-594). | 선택 프레임에서만 데이터가 온다. 레퍼런스 `SkinNoteDistributionGraph` 와 같은 레코드인지 미확인. |

### 5.3 스킨 본체 수준에서 없는 것

| 기능 | 레퍼런스 | R-BMS |
|---|---|---|
| 음수 id 참조 이미지 | destination id 가 음의 정수면 `new SkinImage(-id)`(JSONSkinLoader.java:313-322). 100 stagefile, 101 backbmp, 102 banner, 110 검정, 111 흰색(SkinSourceReference.java:36-43) | `build_body` 에 분기가 없어 "not a kind this build draws" 경고 후 누락(object.rs:561-626). id 상수만 존재(crates/rbms-skin/src/property/generated/misc.rs:238-242) |
| 그리지 않을 객체 사전 제거 | 정적 조건·옵션으로 로드 시 제거(Skin.java:199-236) | rbms-skin 로더 범위(미확인) |
| prepare 주기 제한 | `prepareFramePerSecond` (Skin.java:265-266, 316-323) | 매 프레임 해석 |
| 마우스 클릭 디스패치 | 역순으로 객체에 전달(Skin.java:394-411), 클릭 타입 0~3(SkinObject.java:671-704) | 문서 `hotspot` 표 + songlist 슬롯만(skin_render/mod.rs:359-397) |
| 텍스트 입력 | `SkinTextInput`(Skin.java:395-406) | 없음 |
| customEvents/customTimers 갱신 | `updateCustomObjects`(Skin.java:782-789) | 렌더 계층에는 없음. rbms-skin 범위(미확인) |
| fadeout/input/scene | Skin.java:468-490 | 모델 필드만 존재(crates/rbms-skin/src/model.rs:412-419). 사용처 미확인 |
| OFFSET_ALL | Skin.java:382-389 | 없음 |

## 6. 텍스트

### 6.1 현재 엔진

- cosmic-text 0.19 (crates/rbms-render/Cargo.toml:15). 번들 Inter + 시스템 폰트 폴백(font.rs:9-11, 123-127).
- 캐시: 레이아웃 `px -> text -> Laid` 최대 512개, 글리프 런 `(CacheKey, RGB)` 최대 4096개, LRU 로 75% 까지 정리(font.rs:63-71, 144-154, 378-385).
- 그리기: 글리프마다 같은 색·커버리지의 가로 런을 `fill_rect` 로 방출(font.rs:193-232, 392-410).
- 크기: `px = round(scale * 8.5).max(8)`(font.rs:26-28). 스킨 쪽은 `scale = dst.h / 8.5`(skin_render/mod.rs:60, draw.rs:329). 결과적으로 글자 크기 = 반올림한 dst 높이(논리 픽셀), 최소 8.
- 줄 높이 `px * 1.2`, 한 줄 전제(font.rs:164-177).
- 패밀리 전환: `set_family`/`reset_family` 가 패밀리가 바뀌면 레이아웃·런·아틀라스 캐시를 전부 비운다(font.rs:285-302).

### 6.2 성능 문제 — 캐시 무효화

스킨 텍스트 객체는 그릴 때마다 `ctx.text.set_family(family)` 또는 `reset_family()` 를 부른다(draw.rs:324-327, judge.rs:290-293, songlist.rs:251-254). 문서 한 번을 다 그린 뒤에는 `reset_family()` 를 다시 부른다(skin_render/mod.rs:422). 스킨 폰트가 기본 패밀리와 다르면 프레임마다 최소 두 번(진입, 종료) 캐시가 비워지고, 폰트 두 종을 번갈아 쓰면 객체마다 비워진다. 결과적으로 스킨 화면에서는 매 프레임 모든 문자열을 다시 셰이핑하고 글리프를 다시 래스터한다. ModernChic 은 화면당 TTF 2~3종과 fnt 2~3종을 쓴다(ModernChic/Select/lua/require/textproperty.lua:29-30, 78-80; Play/lua/require/textproperty.lua:25-26, 46-48).

### 6.3 폰트 로드 결함

`TextContext::load_font`(font.rs:279-282)는 데이터를 넣은 뒤 `faces().last()` 의 패밀리 이름을 돌려준다. 데이터가 폰트가 아니면 새 face 가 추가되지 않으므로 직전에 있던 마지막 face(시스템 폰트 또는 앞서 넣은 스킨 폰트)의 이름이 성공으로 반환된다. `.fnt` 경로가 `skin.fonts` 로 들어오면 경고 없이 엉뚱한 패밀리로 그려진다(호출부 skin_render/mod.rs:284-290). fontdb 가 비폰트 데이터를 조용히 버리는지는 실행으로 확인하지 못했다(미확인, 코드 흐름상 추정).

### 6.4 레퍼런스 의미론 대비

| 항목 | 레퍼런스 | R-BMS | 격차 |
|---|---|---|---|
| 폰트 종류 분기 | 경로가 `.fnt` 로 끝나면 `SkinTextBitmap`, 아니면 `SkinTextFont`(JsonSkinObjectLoader.java:636-661) | 전부 TTF 로 취급 | .fnt 미지원 |
| TTF 크기 | 생성 크기 `text.size`, 그릴 때 `scaleY = region.height / size`(SkinTextFont.java:92, 238) → 실효 em = dst 높이 | dst 높이 | 일치(반올림·최소 8 제외) |
| .fnt 크기 | `size = text.size * dstr.width / sk.w`, `scale = size / originalSize`(JsonSkinObjectLoader.java:652, SkinTextBitmap.java:226). dst 높이와 무관 | 없음 | 누락. `TextBody` 가 `size` 를 버린다(object.rs:476-490) |
| .fnt 메타 | 첫 줄 `size=`, 둘째 줄 `base`, `scaleW`, `scaleH`, 페이지 PNG(SkinTextBitmap.java:407-415) | 없음 | 누락 |
| .fnt 타입 | 0 표준(BILINEAR 셰이더), 1 distance field, 2 colored distance field(SkinTextBitmap.java:317-319, 93-108) | 없음 | 누락. ModernChic 는 Decide·Select 에서 `type = 1` 사용(Decide/lua/require/textproperty.lua:58-59, Select/.../textproperty.lua:79-80) |
| 정렬 기준 | `x = align 2 ? region.x - width : align 1 ? region.x - width/2 : region.x`, 레이아웃 폭 = region.width(SkinTextFont.java:167, 272-281) | 왼쪽: dst.x 에서 시작, 가운데: dst.x 중심, 오른쪽: dst.x 가 오른쪽 끝(draw.rs:330-334) | 한 줄·넘침 없음에서는 동일 |
| 세로 기준 | `y = region.y + region.height` 를 libGDX `font.draw` 의 y 로 전달(SkinTextFont.java:175) | 뷰포트로 뒤집은 dst.y(상단)를 줄 상자 상단으로 사용(draw.rs:328-331) | 기준선 정의가 달라 수 픽셀 수직 차 예상. 정확한 값 미확인 |
| overflow | 0 넘침, 1 가로 축소(`scaleX *= width / layout.width`), 2 절단(말줄임 없음)(SkinText.java:32-34, SkinTextFont.java:250-254, 276-279) | 없음. songlist 라벨만 말줄임 절단(songlist.rs:256) | 누락. ModernChic 사용 84곳 |
| wrapping | 폭 기준 줄바꿈(SkinTextFont.java:273-274) | 없음 | 누락 |
| 그림자(TTF·표준 fnt) | 오프셋이 0 이 아니면 색 `rgb/2`, 위치 `(+x, -y)` 로 먼저 그림(SkinTextFont.java:168-173, SkinTextBitmap.java:101-105) | 없음 | 누락. ModernChic shadowOffset 45곳 |
| 외곽선·그림자(DF) | 셰이더 유니폼 `u_outlineDistance = max(0.1, 0.5 - outlineWidth/2)`, `u_outlineColor`, `u_shadowColor`, `u_shadowSmoothing = smoothness/2`, `u_shadowOffset = offset / page 크기`(SkinTextBitmap.java:179-186) | 없음 | 누락. ModernChic outlineWidth 23곳, outlineColor 24곳 |
| 필터 | TTF: `filter != 0` 이면 LINEAR(SkinTextFont.java:165). fnt: 항상 BILINEAR 또는 DF | 없음(사각형 런) | 구조 차이 |
| fallback 폰트 | TTF·fnt 모두 지원(JsonSkinObjectLoader.java:648-661) | `FontDef.fallback` 무시(model.rs:184-190, skin_render/mod.rs:285) | 누락 |
| 빈 문자열 | 그리지 않음(SkinText.java:106-109) | 동일(draw.rs:321-323) | 일치 |
| editable | 입력 영역·writer(SkinText.java:142-169) | 없음 | 누락 |

ModernChic 의 fnt 는 AngelCode 텍스트 형식이고 페이지가 2048x2048 PNG 여러 장이다(Select/font/fnt/main.fnt 1~4행: `size=50`, `lineHeight=75 base=56 scaleW=2048 scaleH=2048 pages=11`).

## 7. 상태 공급 구조

### 7.1 현재 구조

프레임 입력 `SkinFrame`(skin_render/mod.rs:214-228):

| 필드 | 타입 | 역할 |
|---|---|---|
| `now_ms` | i64 | 프레임 시계 |
| `timers` | `&TimerState` | 타이머 시작 시각 표 |
| `state` | `&dyn SkinStateSource` | id → 값 |
| `lua` | `Option<&dyn SkinExprEval>` | 컴파일된 식 평가(`eval_draw`, `eval_integer`, `eval_float`, `eval_text`) (mod.rs:127-134) |
| `mouse` | `Option<(f32,f32)>` | 문서 좌표 포인터 |
| `background` | `Option<TextureId>` | BGA/커버 한 장 |
| `extra` | `FrameExtra` | id 로 못 싣는 화면 상태 |

트레이트: `OffsetSource::offset(id)`, `DrawStateSource::boolean(id)`(음수 id 는 부정), `SkinStateSource::{integer, float, string, timer, now_ms}`(crates/rbms-skin/src/dst.rs:76-90, property/mod.rs:134-154). 미구현 id 기본값: false, 0, 0.0, ""(property/mod.rs:83-92).

화면별 어댑터(전부 skin_render/state.rs):

| 어댑터 | 원천 | 답하는 것 |
|---|---|---|
| `PlayViewState` (200-374) | `HudView` + 곡 메타 몇 개 | boolean: autoplay, 판정 6x2, early/late, 게이지 종류, 현재 랭크. integer 약 25개. float 약 8개. string 6개. offset: LIFT, LANECOVER, HIDDEN_COVER |
| `SelectViewState` (377-504) | `SelectView` | boolean: FOLDERBAR, SONGBAR, PANEL1 + 자체 대역. integer: PLAYLEVEL 하나(455-460). float: 선택 위치, 베스트율. string: 제목류 + 자체 대역 |
| `ResultViewState` (507-646) | `ResultView` | boolean: CLEAR/FAIL, 랭크. integer 약 12개. float 약 6개. string: 제목·아티스트 + 자체 대역 |
| `DecideViewState` (649-726) | 로딩 진행률 | boolean 2, integer 2, float 2, string 5 |
| `KeyConfigViewState` (732-782) | 레인 바인딩 문자열 | string 키 이름 대역만 |

모든 어댑터의 `timer()` 는 `None`(state.rs:367-369 등). 타이머는 `TimerState` 로 따로 온다.

id 수(선언 대비 응답, grep 집계라 근사값):

| 종류 | 선언(crates/rbms-skin/src/property/generated) | state.rs 가 참조 |
|---|---|---|
| NUMBER_ | 273 | 35 |
| OPTION_ | 287 | 26 |
| STRING_ | 47 | 10 |
| RATE_ + FLOAT_ | 70 | 18 |
| BUTTON_ | 74 | 0 |
| SLIDER_ / BARGRAPH_ | 9 / 22 | 0 |
| OFFSET_ | 16 | 3(+ 사용자 조정) |
| 타이머 상수(timer/generated.rs) | 154 | screen.rs 에서 41 |

타이머 구동(skin_render/screen.rs): `PlayTimers`(241-347)가 PLAY/READY/FAILED, JUDGE/COMBO 1P·2P, FULLCOMBO_1P, GAUGE_INCLEASE_1P, GAUGE_MAX_1P, 레인별 BOMB/HOLD/KEYON/KEYOFF 를 전환한다. `SelectTimers`(351-376)는 SONGBAR_MOVE 계열 4개, `ResultTimers`(394-417)는 RESULTGRAPH_BEGIN/END, RESULT_UPDATESCORE.

`FrameExtra`(state.rs:887-898): `None`, `Play(&PlayObjectState)`, `Select(&SelectListState)`, `Result(&ResultSeriesState)`.

| 구조체 | 필드 | 근거 |
|---|---|---|
| `PlayObjectState` | `field: &Skin`, `playfield: &PlayfieldView`, `shade: LaneShade`, `gauge_kind`, `bomb`, `keys_down`, `recent_hits` | state.rs:844-858 |
| `SelectListState` | `rows: &[SelectRow]`, `sel`, `detail: &SelectDetail`, `options` | state.rs:869-874 |
| `ResultSeriesState` | `gauge_series`, `timing_hist`, `judge_dist`, `bpm_points` | state.rs:877-885 |

### 7.2 풀 스킨에 부적합한 이유

1. 원천이 내장 화면용 뷰다. `SelectView`/`SelectRow` 는 색과 문자열로 미리 풀린 표현이어서(select.rs:24-38, 84-99) 램프 종류 번호, 난이도 번호, 바 종류, 점수 상세 같은 id 를 만들 재료가 없다.
2. 어댑터가 렌더 크레이트에 있다. 점수 DB, 설정, IR, 코스, 라이벌처럼 앱만 아는 값에 닿을 수 없다. 수백 개 id 를 채우려면 앱 계층이나 별도 상태 크레이트로 옮겨야 한다.
3. id 를 `match` 로 한 건씩 답한다. 수백 개가 되면 화면별로 중복된다. 레퍼런스는 id 별 속성 객체를 한 번 만들어 `MainState` 하나에 질의한다(예: SkinImage.java:49, SkinNumber.java:80).
4. 자체 id 대역 20_001~20_315 가 섞여 있다(state.rs:101-149). beatoraja 스킨은 이 id 를 쓰지 않는다.
5. 쓰기 방향이 없다. 버튼 이벤트, 슬라이더 writer, 텍스트 writer 가 트레이트에 없다.
6. `FrameExtra` 가 화면당 한 종류만 실린다. 결과 화면의 게이지 객체는 플레이 상태가 없어 그려지지 않고(gauge.rs:246-248), 선택 화면의 bpmgraph 는 결과 상태가 없어 그려지지 않는다(graphs.rs:432).
7. 참조 이미지가 `background` 한 장뿐이다. stagefile, backbmp, banner 를 구분해 실을 자리가 없다(mod.rs:223-224).

권고 방향: 트레이트 시그니처(id → 값)는 유지하고, 구현을 "한 프레임의 게임 상태 스냅숏 + id 표"로 바꾼다. 스냅숏은 앱이 채우고, 렌더 크레이트는 트레이트만 본다. `FrameExtra` 는 화면 종류가 아니라 능력별 선택 필드(노트 필드, 곡 바 목록, 시계열, 참조 이미지 세트)로 푼다. 구체 설계는 속성·Lua 담당 조사와 합쳐 정해야 한다.

## 8. 내장 화면 렌더와 스킨 렌더의 결합 지점

### 8.1 결합 지점 전수

| 지점 | 내용 | 근거 |
|---|---|---|
| 대체 플래그 | `PlayContent`(8개), `SelectContent`(4개), `ResultContent`(9개), `ScreenContent` | content.rs:16-57, result.rs:186-204 |
| 플래그 소비 | HUD 블록 생략 | hud.rs:201-264 |
| | 선택 화면 블록·히트 영역 생략 | select.rs:269-296 |
| | 결과 화면 블록 생략 | result.rs:298 이하 |
| 레이어 | `SkinLayer::{Background, Foreground}`, `draw_layer`, `draw_matching` | crates/rbms-skin/src/model.rs:372-378, skin_render/mod.rs:400-424, screen.rs:60-64 |
| 합성 방식 | `SkinComposition::{Replace, Overlay, Layered}` | model.rs:359-366 |
| 대체 요건 표 | 블록 이름 → 요구 객체 id/종류/핫스팟 | apps/rbms-player/src/skin_screen.rs:161-320 |
| 대체 판정 | `screen_content_of`, `block_replaced` | skin_screen.rs:324-367 |
| 합성 질의 | `skin_uses_overlay`, `skin_uses_native_layout`, `skin_uses_layered_layout`, `screen_content`, `skin_draws_background` | skin_screen.rs:595-656 |
| 화면 게이트 | `render_{play,select,result,decide,keyconfig}_screen` | skin_render/screen.rs:69-131 |
| 플레이 그리기 | 문서 단독 → 아니면 레이어 배경 → 내장 필드/커버/봄/HUD → 레이어 전경 | stage/play/mod.rs:793-905 |
| 선택 그리기 | 같은 구조 + 히트 영역 병합 | stage/select/mod.rs:800-866 |
| 결과 그리기 | 같은 구조 | stage/result.rs:122-141 |
| 로딩(decide) | 문서가 그리면 반환, 아니면 내장 | stage/loading.rs:476-486 |
| 키 설정 | 문서가 그리면 반환, 아니면 내장 | stage/keyconfig.rs:183-190 |
| 레거시 필드 지오메트리 | 문서의 `note.dst` 로 `Skin` 을 덮어씀 | skin.rs:404-421, app_input.rs:282-329 |
| 스킨 객체의 `Skin` 의존 | 노트·게이지·커버·오프셋이 `judge_y`, `top_y`, `lane_height`, `gauge_clear_threshold`, `lift_height` 를 읽음 | notes.rs:170-171, 195-201, 249-250; gauge.rs:252; covers.rs:132; state.rs:270-281 |
| 핫스팟 | 문서 `hotspot` 표의 액션 이름 → `SelectHot` 대응 | state.rs:789-824, skin_render/mod.rs:359-397, songlist.rs:321-343 |
| 자체 id 대역 | 20_001 이후 | state.rs:101-149 |
| 텍스트 스케일 다리 | `TEXT_PIXELS_PER_SCALE = 8.5` | skin_render/mod.rs:60 |
| 내장 팔레트·행 색 차용 | judgegraph 색, songlist 라벨·램프 색 | graphs.rs:259, songlist.rs:303-307 |
| 배경 슬롯 이중 경로 | 내장 슬롯(`set_background`) vs 문서 `bga`(`background_texture`) | stage/play/mod.rs:808-822, stage/canvas.rs:146-181 |

### 8.2 스킨이 화면 전체를 단독으로 그릴 때

걷어낼 것:

- `content.rs` 전체와 `ResultContent`, 그리고 `*_with_content*` 진입점(hud.rs:182-201, select.rs:264-296, result.rs:288-298).
- `SkinLayer`, `draw_layer`, `SkinComposition::{Overlay, Layered}`, `replace` 목록과 대체 요건 표(skin_screen.rs:161-367, 595-626).
- 문서 `hotspot` 표와 `SkinHotAction` 이름 대응(state.rs:789-824). 레퍼런스 방식(객체 클릭 이벤트 id)으로 교체.
- 자체 id 대역과 그것을 쓰는 번들 문서, 관련 테스트(`stage/render_tests_skin_v3_*.rs` 4개, `stage/settings/skin_tests.rs`).
- `Skin::with_document_lanes` 와 `document_field`(skin.rs:404-421, app_input.rs:289-329). 레인 지오메트리는 문서에서 직접 계산해 스킨 객체에 넘긴다.
- 스킨 객체의 `&Skin` 의존(8.1 표). `PlayObjectState.field` 를 문서 기반 구조로 교체.

남길 것:

- `Renderer`, `CpuCanvas`, GPU 백엔드, `TextContext`, 글리프 아틀라스, 골든 하네스.
- 스킨이 없는 화면: 설정(settings_view.rs, stage/settings.rs), 표 관리(stage/tables.rs), 폴더 관리(stage/folders.rs), 연습(stage/practice.rs), 코스 결과(stage/course_result.rs), IR 패널·랭킹(ir_panel.rs, ir_ranking_view.rs), 옵션 패널(app_options.rs), 토스트(toast.rs), 대화상자(dialog.rs), 디버그 오버레이(lib.rs:932-954), 분석 오버레이(stage/play/mod.rs:661-688). 이들은 `theme.rs` 와 `RenderCtx` 를 계속 쓴다.
- 내장 play/select/result/loading/keyconfig 화면(hud.rs, playfield.rs, select.rs, result.rs)은 "스킨 로드 실패 시 폴백"으로 남길지 삭제할지 결정이 필요하다. 기본 스킨이 항상 번들된다면 삭제 후보다.

ModernChic 가 제공하는 화면: play5/7/10/14, musicselect, decide, result, course, keyconfig, skinselect. 설정·표·폴더·연습·IR 에 해당하는 스킨은 없다.

## 9. 성능

| 항목 | 현황 | 근거 | 평가 |
|---|---|---|---|
| 인스턴스 수 | 초기 8192, 자동 확장 | gpu/mod.rs:108, 380-397 | 프레임당 수천~수만 사각형 수용 가능 |
| 프레임당 업로드 | 단색 32B, 텍스처 64B × 개수를 매 프레임 `write_buffer` | gpu/mod.rs:398-403 | 5만 개여도 수 MB |
| 드로콜 수 | 텍스처·블렌드·필터·회전·클립이 바뀔 때마다 배치가 끊김 | batch.rs:158-168 | 소스가 다른 객체가 번갈아 나오면 객체당 1콜. 객체 1~3천 개면 1~3천 콜. 데스크톱에서는 감당 가능하나 여유는 측정 필요(미측정) |
| 불필요한 배치 분할 | `rotated` 가 키에 포함 | batch.rs:64 | 제거 가능 |
| 텍스트 사각형 수 | 글리프당 가로 런 수십 개 | font.rs:221-227 | 긴 제목 한 줄이 수천 사각형. 곡 바 20줄이면 수만 |
| 텍스트 캐시 | 패밀리 전환마다 전체 삭제 | font.rs:285-302, draw.rs:324-327, skin_render/mod.rs:422 | 스킨 화면에서 사실상 캐시 미적용(6.2) |
| 캐시 키 | 색마다 글리프 런을 따로 저장 | font.rs:90, 204 | 페이드(알파만 변함)는 RGB 가 같아 재사용. RGB 가 변하는 애니메이션은 프레임마다 새 항목 |
| 객체 해석 | 객체마다 매 프레임 `prepare` | draw.rs:142-147 | 레퍼런스는 주기 제한 옵션 보유(Skin.java:265) |
| 숫자 자리 | 인라인 배열 16칸, 할당 없음 | object.rs:212-221 | 양호 |
| songlist 문자열 | `fit_text` 가 매 프레임 `String` 생성, 글자 단위 폭 측정 | songlist.rs:256, font.rs:258-276 | 캐시가 살아 있으면 저렴, 지금은 6.2 로 비쌈 |
| notes | 프레임마다 `HashMap` 구성 | notes.rs:207 | 소규모 |
| 프레임 종료 정리 | `GlyphAtlasBinding::end_frame` 호출 주체가 프로덕션에 없음 | glyph_atlas.rs:266-274 | 아틀라스 경로를 켜면 배선 필요 |

## 10. 권고 — 변경 단위 목록

크기: S 는 한 파일 내 수십 줄, M 은 여러 파일 수백 줄, L 은 새 모듈 또는 구조 변경.

| 번호 | 변경 단위 | 대상 파일 | 크기 | 선행 |
|---|---|---|---|---|
| A1 | 색 공간 결정 반영: 표면을 비 sRGB 로 고르거나 텍스처·색을 sRGB 로 일관 처리 | apps/rbms-player/src/gpu/mod.rs:179, 482-491; batch.rs:28-56 | S(코드) / 영향 M(내장 테마 색감 변화) | 사용자 결정 |
| A2 | 논리 크기 런타임화: 유니폼 갱신, `size()`, scissor, 레터박스 비율, 마우스 역변환 | gpu/mod.rs:198, 432, 525-527, 612-627; batch.rs:179-194; lib.rs:134-135, 997 | M | 없음 |
| A3 | 내장 UI 용 배율 래퍼 렌더러(1280x720 좌표 → 물리 좌표). `Canvas` 열거형에 연결 | crates/rbms-render/src/lib.rs(신규 타입), apps/rbms-player/src/stage/canvas.rs:139-274 | M | A2 |
| A4 | 텍스처 한도 확인과 초과 처리(거부 또는 축소), wgpu 오류 포착 | gpu/mod.rs:176, 481-504, 541-564 | S | 없음 |
| A5 | 배치 키에서 `rotated` 제거 | gpu/batch.rs:61-65, gpu/mod.rs:589 | S | 없음 |
| B1 | 텍스트 캐시를 패밀리별로 분리(전환 시 삭제 금지) | crates/rbms-render/src/font.rs:81-97, 156-232, 285-302 | S~M | 없음 |
| B2 | `load_font` 성공 판정 수정(face 수 증가 확인) | font.rs:279-282 | S | 없음 |
| B3 | 스킨 텍스트를 아틀라스 쿼드 경로로 전환, 물리 픽셀 래스터, 다중 페이지, `end_frame` 배선 | font.rs:338-369, glyph_atlas.rs 전반, skin_render/draw.rs:316-336 | M | A2, B1 |
| B4 | 텍스트 의미론: `size`, overflow 0/1/2, wrapping, 그림자(색 rgb/2, 오프셋 +x/-y), 세로 기준 맞춤 | skin_render/object.rs:476-490, 727-734; draw.rs:316-336; judge.rs:282-302; songlist.rs:231-263 | M | B3 |
| B5 | 비트맵 폰트(.fnt) 로더와 그리기: info/common/page/char/kerning 파싱, 페이지 텍스처, `scale = text.size * dw / originalSize`, 타입 0 | 신규 `crates/rbms-render/src/bitmap_font.rs`, skin_render/mod.rs:284-290, object.rs | L | C1 |
| B6 | distance field 텍스트: 전용 파이프라인(외곽선·그림자 유니폼), CpuCanvas 대응 구현 또는 테스트 전략 | lib.rs(프리미티브 확장), gpu/mod.rs, cpu.rs | L | B5 |
| B7 | fallback 폰트 | skin_render/mod.rs:284-290, font.rs | S~M | B5 |
| C1 | 텍스처 관리자: 객체가 참조한 소스만 로드, 경로 기준 공유, 화면 이탈 시 해제 정책, 메모리 집계 | skin_render/mod.rs:266-296, apps/rbms-player/src/skin_screen.rs:412-552, assets.rs:592-629 | M~L | A4 |
| C2 | 참조 이미지: 음수 id destination, stagefile/backbmp/banner/검정/흰색 본체와 프레임 필드 | skin_render/object.rs:561-626, mod.rs:214-228, draw.rs | M | C1 |
| C3 | 동영상 소스: 디코더 도입, 프레임을 같은 key 로 갱신, 루프, 프레임 없을 때 생략 | 신규 모듈, skin_render/object.rs, apps/rbms-player/Cargo.toml | L | C1, 의존성 결정 |
| D1 | stretch 미구현 7종(3, 4, 5, 6, 7, 8, 10)과 trimmed 소스 절삭 | crates/rbms-skin/src/loader/stretch.rs:61-107, skin_render/draw.rs:104-126 | S~M | 없음 |
| D2 | image: 음수 선택값 숨김, 이미지 인덱스 속성, 식 타이머 | skin_render/draw.rs:190-197, object.rs:524-526, 629-640 | S | 속성 담당 조사 |
| D3 | 클릭 이벤트·슬라이더 조작·텍스트 입력 | skin_render 전반, 상태 트레이트 | M~L | E1 |
| D4 | OFFSET_ALL 전역 변환 | skin_render/mod.rs:404-424 | S | E1 |
| E1 | 상태 공급 재설계: 어댑터를 렌더 크레이트 밖으로, id 표 기반, `FrameExtra` 분해 | skin_render/state.rs 전체, screen.rs, mod.rs:214-228, apps/rbms-player/src/skin_screen.rs | L | 속성·Lua 담당 조사 |
| E2 | 노트·게이지·커버의 레거시 `Skin` 의존 제거 | skin_render/notes.rs, gauge.rs, covers.rs, state.rs:270-281; skin.rs:404-421; app_input.rs:282-329 | M | E1 |
| E3 | songlist 를 레퍼런스 바 구조로 재작성(바 종류, 램프·레벨·trophy 이미지, 그래프) | skin_render/songlist.rs 전체 | L | E1, 선택 화면 담당 조사 |
| F1 | 합성 계층 제거: content 플래그, 레이어, replace 표, 핫스팟 이름 표, 자체 id 대역 | content.rs, hud.rs, select.rs, result.rs, skin_render/screen.rs, state.rs:101-149, 789-824; skin_screen.rs:161-367, 595-626; stage/play/mod.rs:793-905; stage/select/mod.rs:800-866; stage/result.rs:122-141 | M | 새 기본 스킨이 화면을 단독으로 그릴 수 있게 된 뒤 |
| F2 | 구 번들·테스트 정리 | assets/skins/steel-neon-v3, stage/render_tests_skin_v3_*.rs | S~M | F1 |
| G1 | 문서 갱신 | docs/architecture.md:28, 52, 73, 81-85 | S | 각 단계 종료 시 |

의존 순서(직렬 축): A1·A2 결정 → A2 → A3 → B1·B2(독립, 즉시 가능) → C1 → B3 → B4 → B5 → B6. 병렬 축: A4, A5, D1 은 언제든 가능. E1 은 가장 크고 다른 조사 결과가 필요하므로 별도 흐름으로 일찍 시작한다. F1 은 마지막.

docs/architecture.md:52 의 "Everything drawn ... lowers to `Renderer::fill_rect` (plus one textured BGA pipeline)" 는 현재 코드와 맞지 않는다(텍스처 쿼드·클립·회전이 트레이트에 있음, lib.rs:315-354).

## 11. 읽은 범위

전부 읽음: crates/rbms-render/Cargo.toml, src/lib.rs, ctx.rs, cpu.rs, content.rs, font.rs, glyph_atlas.rs, skin_render/{mod, draw, object, state, screen, notes, gauge, judge, songlist, graphs, covers, color}.rs, apps/rbms-player/src/gpu/background.rs, stage/canvas.rs, docs/architecture.md.

부분만 읽음:

| 파일 | 읽은 범위 | 읽지 않은 범위 |
|---|---|---|
| crates/rbms-render/src/skin.rs | 1-520 | 521-810(테스트) |
| hud.rs | 50-264 + 상수 목록 | 265-466(판정 카운터·그래프 뒷부분, 테스트) |
| playfield.rs | 1-285 | 286-780(테스트) |
| select.rs | 1-300 + 함수 목록 | 300-704(각 블록 그리기 본문) |
| result.rs | 14-52, 160-205, 296-330 + 함수 목록 | 나머지 본문, result/grade.rs, result/graphs.rs |
| theme.rs | 구조(grep) | 본문 |
| toast.rs, golden.rs | 구조(grep) | 본문 |
| skin_render/tests*.rs | 테스트 수만(32, 25, 13) | 본문 |
| apps/rbms-player/src/gpu/mod.rs | 1-645 | 646-756(테스트) |
| gpu/batch.rs | 1-232 | 233-496(테스트) |
| apps/rbms-player/src/skin_screen.rs | 36-765 | 1-35, 766-910(화면별 draw 래퍼 뒷부분) |
| stage/play/mod.rs | 560-919 | 1-559 |
| stage/select/mod.rs, stage/result.rs, stage/loading.rs, stage/keyconfig.rs | 스킨 게이트 주변만 | 나머지 |
| crates/rbms-skin (배정 범위 밖) | loader/stretch.rs 1-128, dst.rs·property/mod.rs 의 트레이트 정의, model 의 정의부 | 로더·Lua·prepare 본문 |

레퍼런스: Skin.java, SkinObject.java, SkinImage.java, SkinNumber.java, SkinText.java, SkinSourceMovie.java, SkinSourceImage.java, StretchType.java 는 전부. SkinTextFont.java 1-330, SkinTextBitmap.java 58-282 와 376-455, SkinGraph.java 60-113, SkinSlider.java 95-150, JSONSkinLoader.java 300-345 와 480-530, JsonSkinObjectLoader.java 20-70 과 텍스트 생성부 grep 결과. SkinFloat, SkinGauge, SkinJudge, SkinBar, LaneRenderer, 각 그래프 클래스, 셰이더 소스(src/glsl)는 읽지 않았다.
