> 최종 갱신 2026-10-22 · 대응 커밋 47a7f9c

# 글자 렌더링 조사 (요청 5: 글꼴 가독성, 저화질 느낌)

범위는 앱 UI 글자, 스킨 글자(TTF, BMFont), GPU 쪽 샘플러·포맷·블렌딩, 논리 1280x720 과 물리 픽셀의 관계, 저화질 느낌의 원인 순위, 개선안이다. 조사로 시작해 같은 날 구현한 뒤 현재 상태로 고친 살아 있는 문서다. 1절(결론), 2.1절, 3절의 배율 설명, 5절의 굵기 행, 6절, 7.4절, 9절 O1·O4, 12절, 14절 결정은 구현 뒤의 코드 기준이고, 나머지(경로 표 중 B~G, GPU 사실, 실증 7.1~7.3, 순위 8절의 확실성 열)는 조사 당시 그대로다. 구현: `f03beb3`(내장 UI 글자를 장치 배율의 물리 화소로 래스터하고 굵기 선택 추가), `0c450db`(기본 스킨에 Inter Bold 추가), `f52e170`(리뷰 수리 3건). 결정의 근거는 ADR [UI 글자 장치 화소](../acknowledge/2026-10-22-adr-ui-text-device-pixels.md)(결정은 사용자 지시 "끝까지 다 구현해놔" 에 따라 메인이 추천안으로 정했고 사용자는 아직 확인하지 않았다). "7. 실증" 의 이미지는 조사 세션의 scratchpad 에만 있고 저장소에는 없다(재현 명령 포함). 문서와 코드가 다른 곳은 "12. 문서와 코드의 차이" 에 모았다.

## 1. 결론

1. (조사 때) 내장 화면의 글자는 논리 1280x720 기준 정수 px(9~26)로 래스터한 뒤 글리프의 점 하나하나를 `fill_rect` 로 내보내고 `ScaledRenderer` 가 좌표에 배율을 곱했다. 창의 물리 크기가 논리의 2배(macOS Retina 기본 창)이면 글자의 화소 하나가 2x2 블록, 1.5배(Windows 150%, 1920x1080)이면 1화소 폭과 2화소 폭이 번갈아 나오는 격자였다. **구현 뒤:** 렌더러가 장치 배율을 알려 주면(`Renderer::device_scale`) 글자를 `논리 크기 x 배율` 의 물리 em 으로 구워 장치 화소 사각형(`fill_rect_device`)으로 낸다. 배율 1 과 비균등 배율(letterbox 끔)은 이전 경로 그대로다(2.1절).
2. 스킨 글자는 처음부터 화면에 닿는 크기로 래스터했다. TTF 는 `compose_block` 이 `em = 목적지 높이(물리 px)` 로 한 줄을 텍스처 하나에 구워 1:1 최근접으로 그린다. 스킨 화면은 `canvas.native()` 로 창 표면 화소에 직접 그리므로 늘리는 단계가 없다. 오프스크린 텍스처에 그린 뒤 창에 늘리는 구조는 어디에도 없다(렌더 패스는 표면 하나).
3. 래스터 자체는 약하다. 소형 크기(내장 UI 9~15px, 대문자 높이 6.5~11px), 그레이스케일 커버리지만 쓰고, 힌팅은 실효 없음, 블렌딩은 비 sRGB 프레임버퍼에서 부호화된 값 그대로 섞는다(밝은 글자 위 어두운 배경에서 가장자리가 얇고 어둡게 보이는 쪽). **구현 뒤:** UI 굵기를 고를 수 있다(`TextStyle`, REGULAR 400 ~ BOLD 700. 번들 Inter 가 가변 폰트라 새 에셋이 필요 없다). 기본 스킨 팩에는 스킨 글자용으로 정적 `Inter-Bold.ttf` 가 더해졌다(6절).
4. 순위(8절): 1위 내장 UI 논리 래스터 후 확대와 4위 비정수 배율의 화소 격자는 구현으로 해소됐다. 2위 소형·힌팅 없음, 3위 감마 공간 블렌딩, 5위 스킨 글자의 정수 픽셀 스냅과 SHRINK 선형 축소는 그대로다.
5. 비용(7.3, 7.4절): 사각형 수는 배율에 따라 늘지만(2배에서 약 2.4~2.7배, 3배에서 약 3.7~4.1배) 캐시 적중 프레임의 CPU 제출은 프레임당 약 75us 이내다(W1a 측정 최대 72us). 배율 1 에서 이전과 화소 단위로 같다(골든 서명 10개 불변). 외부 Lua 스킨 영향 없음. 새 의존성 없음.

## 2. 글자가 화면에 닿는 경로 전부

| 경로 | 쓰는 곳 | 래스터 크기 | 위치 결정 | GPU 에 닿는 모양 |
| --- | --- | --- | --- | --- |
| A. 내장 UI `TextContext::draw` | 내장 화면 전부, 토스트, 디버그, 선곡 오버레이 | 배율 1 또는 비균등: `px_for(scale) = round(scale*8.5).max(8)` 논리 px. 균등 배율: `px_for(scale) x 장치 배율`(반올림 없음, 물리 em) | 배율 1: 논리 정수 원점 + 글리프 x 4분 서브픽셀, y 정수. 균등 배율: 장치 좌표 `round(x*sx)`, `round(y*sy)` 와 장치 좌표 기준 x 서브픽셀 빈 | 켜진 화소 가로 연속 구간마다 사각형. 균등 배율은 장치 화소 사각형 `fill_rect_device`(배율 곱 없음), 그 외는 `fill_rect` 뒤 `ScaledRenderer` 가 배율을 곱함 |
| B. 스킨 TTF `compose_block` | 스킨 문서의 `text` 객체(폰트가 읽힌 경우) | em = 목적지 높이(물리 px, 분수 가능) | 정수 반올림, 서브픽셀 없음 | 줄마다 RGBA 텍스처 1장(흰 잉크), 1:1 최근접 쿼드(SHRINK 면 선형) |
| C. 스킨 BMFont type 0 | 스킨의 `.fnt` 글꼴 | 폰트가 만들어진 크기에 배율 `size * (화면폭/스킨폭) / 원본 size` | 레이아웃이 정하는 대로(픽셀 격자 없음) | 페이지 글리프를 잘라 선형 필터 쿼드 |
| D. 스킨 BMFont type 1, 2 | distance field 글꼴 | 위와 같음 | 위와 같음 | `draw_distance_field_quad`(원본 셰이더 식, 평활 폭 1/16 고정) |
| E. 스탠드인 `draw_stand_in` | 폰트가 안 읽힌 스킨 text | `dst.h / 8.5` 를 `px_for` 에 통과 | 정수 | A 와 같은 `fill_rect`, 단 네이티브 렌더러라 배율 1 |
| F. 노트 필드 라벨 | 스킨 노트 필드의 시간 라벨 | `LABEL_SCALE 2.1` -> 18px | 정수 | A 와 같은 `fill_rect`, 네이티브(배율 1) |
| G. 글리프 아틀라스 `draw_text_atlas` | 테스트만 | `px_for` | 정수 | 글리프당 텍스처 쿼드 1개. 앱에서 호출하는 곳이 없다 |

### 2.1 A. 내장 UI 글자 (`crates/rbms-render/src/font.rs`)

- 호출 형태: `draw_text(r, x, y, scale, color, text)`(`font.rs` 의 `draw`)와 굵기를 고르는 `*_styled`. `scale` 은 옛 5x7 시절의 단위이고 `px_for(scale)` 이 px 로 바꾼다. `TEXT_PIXELS_PER_SCALE = 8.5`(`skin_render/text.rs:63`)가 같은 비율이다. 굵기는 `FontWeight(u16)`(REGULAR 400, MEDIUM 500, SEMIBOLD 600, BOLD 700, `font.rs:116-123`)와 `TextStyle { scale, weight }`(`font.rs:131-145`)로 고른다. `TextContext` 메서드 `text_width_styled`, `draw_text_styled`, `draw_text_centered_styled`, `draw_text_right_styled`, `fit_text_styled`(`font.rs:457-502`)와 같은 이름의 자유 함수(`font.rs:821-841`)가 있고 `RenderCtx` 는 `ctx.text.*_styled` 로 쓴다. `weight` 를 인자로 따로 받지 않고 `TextStyle` 로 묶은 이유: 인자 8개가 clippy `too_many_arguments` 에 걸리고 새 `#[allow]` 는 금지다. 굵을수록 폭이 넓어지므로 정렬·`fit` 도 같은 style 로 재야 한다.
- 레이아웃: `ensure` 가 cosmic-text `Buffer` 에 `Metrics::new(px, px*1.2)` 로 한 줄을 셰이핑(`Shaping::Advanced`)한다. 레이아웃 캐시 키는 family → `(px, weight)` → text 이고 weight 0(`OWN_WEIGHT_KEY` `font.rs:149`)은 face 자기 굵기(plain 호출)다(`layout_key` `font.rs:154`). `Laid`(`width`, `glyphs`, `shaped`, `used`)는 글리프의 `LayoutGlyph` 와 `line_y` 를 함께 저장하므로, 레이아웃은 논리 px 로 한 번만 셰이핑하고 장치 배율마다 `CacheKey` 를 다시 만든다. 한도: 레이아웃 캐시 512줄(`LAYOUT_CACHE_LIMIT` `font.rs:223`), 글리프 구간 캐시 4096개(`RUN_CACHE_LIMIT` `font.rs:227`). 22줄 메뉴의 (레이아웃, 구간) 항목 수는 1x (22, 328), 2x (22, 326), 3x (22, 325) 로 배율에 따라 늘지 않고 항목당 run 수만 는다(`an_enlarged_menu_fits_both_caches_as_the_small_one_does` 가 고정). `Laid` 가 `LayoutGlyph` 를 더 들어 레이아웃 캐시 메모리가 늘었다(512줄 x 약 20글리프 x 약 80B, 1 MB 이하 추정, 측정은 안 함).
- 래스터: 기본 경로는 `SwashCache::get_image_uncached` 로 굽고 `lit_pixels(image, base)`(`font.rs:723`)로 켜진 화소 `(dx, dy, rgb, 커버리지)` 목록을 만든다. cosmic-text 0.19 `with_pixels` 와 같은 의미다(Mask: 계수를 base rgb 로 잉크, Color: rgba 그대로, SubpixelMask: 그리지 않음). 같은 색·같은 커버리지의 가로 이웃을 합친 구간(`merge_runs`)은 `(글리프 CacheKey, 색)` 으로 캐시하고(`GlyphPainter::paint` `font.rs:678-`), 픽셀 사본은 `evict_runs`(`font.rs:667`)로 한도를 관리하는 runs 캐시 하나만 둔다. swash 이미지 캐시(`SwashCache.image_cache`, 한도 없는 `HashMap`)는 쓰지 않는다(리뷰 font-1: 창을 끄는 동안 프레임마다 새 em 의 글리프가 남아 1,000개 크기에 이미지 279개가 271,503개, RSS 14.6 MB 가 139.2 MB 로 늘었다. `with_pixels` 로 임시 되돌리면 새 테스트 `a_window_drag_rasterizes_past_the_rasterizers_unbounded_image_cache` 가 '48 drag frames leave 13480 rasterized images beside 3443 glyph runs' 로 실패하고, 수정본은 이미지 캐시 0개·runs ≤ `RUN_CACHE_LIMIT` 로 통과). `lit_pixels` 와 `with_pixels` 의 동치는 골든 불변과 `primitives.rs` 의 `the_glyph_atlas_draws_the_same_pixels_as_the_fill_path` 가 고정한다.
- 그리기: `device_text_scale(r)`(`font.rs:175`)가 `Some(s)` 이면 새 경로, `None` 이면 이전 경로다.
  - 새 경로: em 은 `px_for(scale) x s`(반올림 안 함), 글리프 `g.physical((0, trunc(line_y) x s), s)`, 원점 `round(x x sx)`, `round(y x sy)`, x 서브픽셀 빈은 장치 좌표 기준이다. 구간마다 `r.fill_rect_device(...)` 로 장치 화소 사각형을 낸다. baseline 을 `trunc(line_y) x s` 로 둔 이유: 이전 확대 경로와 같은 세로 위치를 지키려는 것이다(`line_y x s` 로 두면 1.5배에서 약 1.5 장치 화소 내려가는 것을 실측했다). 그래서 물리 해상도에서 쓸 수 있는 1/s 화소 단위 baseline 정밀도는 쓰지 않는다.
  - 이전 경로(배율 1 또는 비균등): 원점 `(x.round(), y.round())`, 구간마다 `r.fill_rect(Rect(논리 좌표))` 뒤 `ScaledRenderer` 가 배율을 곱한다. 배율 1 에서 화소 단위로 이전과 같다.
- 배율 판정 `device_text_scale`(`font.rs:157-186`): `(sx, sy) == (1, 1)` 이면 `None`. 그 외에 각 변이 따로 반올림될 때 생길 수 있는 오차 `ROUNDING_ERROR_PX(0.5) x (1 + 긴 변/짧은 변)`(16:9 에서 약 1.39 화소) 이내이고 유한·양수일 때 `|sx - sy| x 긴 변` 이 거기 들면 균등으로 보고 `Some(min(sx, sy))` 를 돌려준다. letterbox 끔의 16:9 가 아닌 창은 `None` 이라 이전처럼 늘어난 글자다(결정 6A). 레터박스 뷰포트의 최대 오차는 0.889 화소다(리뷰 font-2: 처음 구현의 여유 1.0 화소는 `LogicalSize` 를 가로세로 따로 반올림한 16:9 창, 예 1280x720 의 x1.33 인 1702x958 을 비균등으로 오판했다).
- 트레이트: `Renderer::device_scale(&self) -> (f32, f32)`(기본 (1,1), `lib.rs:442`)와 `fill_rect_device(&mut self, rect, color)`(기본은 `device_scale` 로 나눠 `fill_rect`, `lib.rs:454`). `ScaledRenderer`(`lib.rs:593,598`)는 `device_scale = inner x 자기 배율`, `fill_rect_device` 는 곱 없이 inner 로 전달한다. `RegionRenderer`(`region.rs:109,113`)는 inner 의 배율을 그대로, `fill_rect_device` 는 영역 원점 x 배율을 반올림해 더해 전달한다(정수 격자 유지). 앱 `Canvas`(`apps/rbms-player/src/stage/canvas.rs:414,418`)는 `scale_between(UI_SIZE, native_size)` 를 보고하고 `fill_rect_device` 를 `native().fill_rect` 로 보낸다. `NativeCanvas`, `HeadlessCanvas`, `Gpu` 는 기본값이다. `OffsetRenderer`(스킨 OFFSET_ALL 늘이기, `skin_render/whole.rs`)는 일부러 전달하지 않고 기본 (1,1) 을 유지한다: 전달하면 스탠드인 글자·노트 라벨이 OFFSET_ALL 늘이기에서 화소가 바뀌어 "외부 Lua 스킨 화소 불변" 비목표를 어긴다(`text_drawn_through_a_stretch_is_the_unstretched_text_enlarged`). 새 렌더러 래퍼가 `device_scale` 을 전달하지 않으면 그 아래는 이전 경로로 그려지고 그것도 항상 올바른 결과다.
- 폭 계산: `text_width`, `fit_text`, 가운데·오른쪽 정렬은 같은 논리 px 의 레이아웃 폭을 쓴다. 창 크기와 무관한 값이라 레이아웃이 안정적이다.

내장 화면의 크기 상수(`apps/rbms-player/src/stage/preview_menu.rs:41-184`)와 px 환산(Inter 대문자 높이 = 0.7275 em, OS/2 capHeight 1490/2048):

| scale | px | 대문자 높이(px) | 쓰는 곳 |
| --- | --- | --- | --- |
| 1.1 | 9 | 6.5 | FIGURE_NAME, FIGURE_TAG, FIGURE_NOTE |
| 1.2 | 10 | 7.3 | HINT, STATUS_DETAIL, COUNTER |
| 1.3 | 11 | 8.0 | LABEL(행 이름), INFO |
| 1.4 | 12 | 8.7 | VALUE(행 값) |
| 1.5 | 13 | 9.5 | TAB |
| 1.6 | 14 | 10.2 | SUBTITLE, FIGURE_VALUE |
| 1.8 | 15 | 10.9 | STATUS |
| 2.1 | 18 | 13.1 | 노트 라벨(네이티브) |
| 2.4 | 20 | 14.5 | TITLE |
| 3.0 | 26 | 18.9 | 로딩 제목 |

### 2.2 B. 스킨 TTF 글자 (`crates/rbms-render/src/font/block.rs`, `skin_render/text.rs`)

- 글꼴 읽기: 스킨 문서의 `font` 목록의 `.fnt` 가 아닌 파일은 `assets.font(path)` 의 바이트를 `TextContext::load_font` 로 등록한다(`skin_render/mod.rs:307`). 같은 바이트는 해시로 알아보아 데이터베이스에 두 번 넣지 않는다(`font.rs:411`).
- 크기: 목적지 사각형 높이가 em 이다(`text.rs:221` `em_px: dst.h`, `dst` 는 `viewport.place(rect)` 즉 이미 물리 px, `text.rs:363`). 문서의 `size` 는 `design_px` 로, 레퍼런스처럼 **advance 와 대문자 높이를 design 크기의 정수로 반올림한 뒤 배율을 곱하기 위해서만** 쓴다(`block.rs:420` `advance = (glyph.w/scale).round()*scale`, `block.rs:470` 기준선 `((cap + i*line)*scale).round()`).
- 래스터: `raster()`(`block.rs:347`)는 `CacheKey::new(..., (0.0, 0.0), ...)` 로 서브픽셀 없이 구운 `SwashImage` 를 쓴다. 한 줄을 흰 잉크 RGBA(`CLEAR_PIXEL` = 흰색·알파 0 바탕, `block.rs:81`)에 straight-alpha `over`(`block.rs:254`)로 합성하고 색은 그릴 때 `tint` 로 입힌다.
- 위치: 글리프 원점은 `(origin + glyph.origin).round()`, 줄 위치는 `(anchor + stamp.left).round()`, `(dst.y + stamp.top).round()`(`block.rs:473-474`, `text.rs:382`). 필터는 `drawn_width == width` 면 `Nearest`(1:1), 아니면 `Linear`(`text.rs:383`). 아니라는 것은 `overflow = SHRINK` 로 줄이 가로로 눌린 경우다.
- 다시 만들기: 문자열, em, 너비가 바뀔 때만 다시 합성한다(`text.rs:Line::texture`).
- 효과: 창 크기에 따라 em 이 달라지면(1280x720 창이면 caption 18 -> 12px, Retina 2560x1440 이면 24px) 그 크기로 새로 굽는다. 늘리기 단계가 없다.

기본 스킨(`assets/skins/rbms-default`) 글꼴 설정(1920x1080 기준 제작): `theme.type = display 96, title 64, heading 44, body 32, label 24, caption 18`(`lua/theme.lua:206-213`), 글꼴 id `inter`, 경로 `font/Inter-Regular.ttf`(`theme.lua:235-238`). 창 배율별 실제 em: 1280x720 비 HiDPI 는 x0.667(caption 12, label 16, body 21.3, heading 29.3, title 42.7, display 64), Retina 2560x1440 은 x1.333(caption 24, label 32, body 42.7, heading 58.7, title 85.3, display 128). 긴 제목·이름은 `SHRINK` 를 쓴다(`lua/decide/main.lua:73,108,143`, `lua/result/main.lua:107,136,144`, `lua/result/judges.lua:104,136` 등. `props.text_overflow.SHRINK` 또는 `TRUNCATE` 를 쓰는 줄이 `lua/` 아래 33곳, grep 확인).

### 2.3 C, D. 스킨 BMFont (`crates/rbms-render/src/bitmap_font.rs`, `skin_render/text/bitmap.rs`)

- 지원한다. BMFont 텍스트 형식 `.fnt` 를 libGDX 1.9.9 조판 규칙대로 읽고, 쓰이는 페이지만 비동기로 불러온다. type 0 은 커버리지 글리프를 선형 필터로, type 1·2 는 distance field 셰이더로 그린다(`bitmap.rs` 머리말).
- 배율은 목적지 높이가 아니라 `size * (화면폭/스킨폭) / 원본 size` 로 정한다(레퍼런스 `SkinTextBitmap`). 글리프는 "픽셀 격자 위에 놓이지 않는다". 즉 비트맵 글꼴은 레퍼런스처럼 미리 구운 이미지를 확대·축소해 그린다. 글꼴 제작 크기와 화면 배율이 어긋나면 선형 보간으로 흐려진다(레퍼런스와 같은 동작이다).
- distance field 평활 폭은 텍스처 단위 `1/16` 고정(`lib.rs:319`)이라 화면에서 작게 그리면 가장자리가 1화소보다 좁아진다(`lib.rs:317-318` 주석). 즉 지금 있는 distance field 경로는 소형 UI 글자에 재사용할 수 없다.
- 외부 스킨 원본(ModernChic)의 글꼴: TTF 는 Mgen+ 계열(`mgenplus-1c-black/medium/bold.ttf`), 비트맵 `.fnt` 도 폴더마다 다수(Decide, Play, Result, Select). 문서: `docs/reference/skin-compat/m6-misc-assets.md` §8.

### 2.4 G. 아틀라스 경로는 앱에서 쓰이지 않는다

`TextContext::draw_text_atlas`(`font.rs:501`)와 `GlyphAtlas`, `GlyphAtlasBinding`(`glyph_atlas.rs`)은 `crates/rbms-render/tests/primitives.rs` 의 테스트만 부른다(`grep` 확인: 앱·렌더 크레이트의 비테스트 코드에 호출 0). `docs/architecture.md:76`, `docs/crates.md:239` 의 "글리프 아틀라스로 래스터" 서술은 코드와 다르다(12절). 이 경로는 흰 잉크가 아니라 색별로 구운 RGBA 를 아틀라스에 넣고(`(CacheKey, rgb)` 키, `font.rs:501-535`), 호출마다 `glyphs.clone()` 으로 `Vec` 을 복제한다. 그대로 승격하려면 이 두 가지를 손봐야 한다(9절 O2).

## 3. 논리 1280x720 과 물리 픽셀

| 상황 | 창(논리) | 표면(물리) | 내장 UI 배율 | 스킨 배율(1920x1080 제작) |
| --- | --- | --- | --- | --- |
| Windows 100% 기본 창 | 1280x720 | 1280x720 | 1.0 | 0.667 |
| macOS Retina 기본 창 | 1280x720 | 2560x1440 | 2.0 | 1.333 |
| Windows 125% / 150% | 1280x720 | 1600x900 / 1920x1080 | 1.25 / 1.5 | 0.833 / 1.0 |
| 해상도 행 1920x1080(논리), Retina | 1920x1080 | 3840x2160 | 3.0 | 2.0 |
| 전체 화면(borderless), 1920x1080 모니터 | 모니터 | 1920x1080 | 1.5 | 1.0 |
| 전체 화면, 3024x1964(MacBook 14인치 기본 해상도) | 모니터 | 3024x1964 -> 16:9 맞춤 3024x1701 | 2.3625 | 1.575 |
| 창을 손으로 크기 변경 | 임의 | 임의 | `Resized` 마다 바뀜 | 바뀜 |

근거와 규칙:

- 창은 `LogicalSize` 로 만든다(`window_mode.rs:50`, 해상도 행 값은 "logical pixels", `crates/rbms-config/src/window.rs:31-40`). `Gpu::new` 는 `window.inner_size()`(물리)로 표면을 설정하고(`gpu/mod.rs:465`), `WindowEvent::Resized` 가 `Gpu::resize(물리 w, h)` 를 부른다(`lib.rs:1470`). 앱은 `scale_factor()` 를 직접 읽지 않고(`grep` 확인 0건) `ScaleFactorChanged` 도 따로 처리하지 않는다. 배율은 항상 `scale_between(UI_SIZE, 뷰포트 크기)` 로 얻어지므로 HiDPI, 전체 화면, 리사이즈를 구분 없이 덮는다.
- 뷰포트: `letterbox`(기본 켬, `DEFAULT_LETTERBOX = true`)이면 16:9 를 지키는 가장 큰 사각형을 가운데에 두고 나머지는 바탕색 막대다(`surface_viewport`, `gpu/mod.rs:1271`). 꺼 있으면 표면 전체로 늘어나며 가로·세로 배율이 다를 수 있다. 뷰포트 크기가 곧 `Renderer::size()` 이고, 셰이더의 `screen` 유니폼이다(`write_screen_uniform`, `gpu/mod.rs:1161`). 1 단위 = 1 물리 화소다.
- 스킨 화면: `SkinScreen::draw_native` 가 `canvas.native()`(뷰포트 크기의 네이티브 렌더러)에 그린다(`skin_screen.rs:425`). 문서 좌표 -> 화면 좌표는 `SkinViewport` 가 한 번에 곱한다. 글자는 이 단계 뒤의 물리 px 로 구워진다.
- 내장 화면: `Canvas` 가 UI_SIZE 를 보고하고(`canvas.rs:396`), 그린 좌표는 위의 `ScaledRenderer` 가 곱한다. 포인터 위치는 반대 방향으로 `position_in_viewport` 가 1280x720 로 되돌린다(`lib.rs:1475`).
- macOS 번들은 `NSHighResolutionCapable = true`(`scripts/macos-app.sh:62`)라 저해상도 확대 모드가 아니다.
- 비균등 배율(letterbox 끔, 16:9 가 아닌 창)에서는 가로·세로 배율이 다르다. 구현은 이 경우 이전 경로(늘어난 글자)를 그대로 쓴다. 균등 판정의 여유는 각 변을 따로 반올림한 오차까지다(2.1절). 같은 16:9 라도 `LogicalSize` 를 가로세로 따로 반올림한 창은 균등으로 판정된다.

## 4. GPU 쪽 사실 (`apps/rbms-player/src/gpu/`)

| 항목 | 값 | 근거 |
| --- | --- | --- |
| 표면 포맷 | 비 sRGB(Bgra8Unorm 우선, Rgba8Unorm). sRGB 만 있으면 뷰를 비 sRGB 로 재해석 | `gpu/mod.rs:241`, `choose_surface_formats` `:341` |
| 텍스처 포맷 | `Rgba8Unorm`(비 sRGB), 밉맵 없음(`mip_level_count: 1`) | `gpu/mod.rs:1070` |
| 샘플러 | `Nearest` 와 `Linear` 두 개, 밉맵 필터 Nearest 이나 밉이 없어 무의미, 주소 모드는 기본(Clamp), 이방성 없음 | `gpu/mod.rs:697-711` |
| 색 사각형 | 셰이더는 색을 그대로 반환. 블렌드 `ALPHA_BLENDING`(색 SrcAlpha/OneMinusSrcAlpha, 알파 One/OneMinusSrcAlpha) | `gpu/mod.rs:42-60`, `:577` |
| 텍스처 쿼드 | `textureSample(t, s, uv) * tint`. 모드별 파이프라인 4개(Alpha, Add, Multiply, InvertDst). 텍스처는 straight alpha | `gpu/mod.rs:62-88`, `lib.rs:181-203` |
| 인스턴스 크기 | 색 사각형 32바이트(`rect 4f + color 4f`), 텍스처 쿼드 64바이트 | `gpu/batch.rs:28`, `:72` |
| 래스터 규칙 | 안티에일리어싱 없음(MSAA 없음). 사각형 변은 화소 중심 규칙으로 정수 화소에 붙는다 | 파이프라인 `multisample` 기본 |
| 패스 | 프레임당 렌더 패스 1개, 오프스크린 중간 타깃 없음 | `Gpu::encode`, `gpu/mod.rs:935` 이후 |
| 제출 | 한 프레임의 그리기를 `DrawList` 에 쌓았다가 `render()` 에서 한 번에 제출(제출 순서는 z-order 라 재배열 안 함) | `gpu/mod.rs:855`, `docs/acknowledge/reference-divergences.md` E-D1 |

결과: 글자의 가장자리 값은 "부호화된 화소값 그대로의 선형 보간"이다(감마 보정 없음). 흰 글자의 커버리지 0.5 화소는 어두운 바탕 위에서 부호화 값 0.5, 물리적으로 맞는 선형광 혼합이었다면 약 0.735 다. 밝은 글자·어두운 배경 UI 에서 가장자리가 약 32% 어둡게 나온다(가는 글씨로 보이는 이유의 하나).

`CpuCanvas`(헤드리스 캡처, 테스트)는 `fill_rect` 를 `floor`/`ceil` 로 바깥쪽 맞춤한다(`cpu.rs:424-427`). GPU 의 화소 중심 규칙과 다르다. 정수 배율에서는 같지만 1.5배 같은 비정수 배율에서는 헤드리스 이미지가 GPU 와 다르다(이 사실은 `stage/capture.rs` 의 `GPU_EDGE_PIXELS_ONE_IN` 주석에도 적혀 있다). 구현 뒤 내장 글자는 정수 장치 사각형(`fill_rect_device`)이라 CPU·GPU 규칙이 일치한다. 강제 GPU 일치 테스트(`RBMS_REQUIRE_GPU=1`, 1280x720 과 1920x1080)가 허용치 안에서 통과했다. 텍스처 쿼드는 양쪽 다 화소 중심에서 샘플해 일치한다(`cpu.rs:248` `paint_quad`).

## 5. 래스터 세부

| 항목 | 내장 UI(A) | 스킨 TTF(B) |
| --- | --- | --- |
| 힌팅 | swash `hint(true)`(`cosmic-text-0.19.0/src/swash.rs`, `DISABLE_HINTING` 안 켬). 실효 없음(아래) | 같음 |
| 서브픽셀 | x 만 4분(0, .25, .5, .75), y 정수 | 없음(x, y 모두 정수 반올림) |
| 소수 좌표 | 원점 정수 반올림, 사각형은 배율 곱 뒤 소수 | 줄 위치 정수 반올림 |
| 안티에일리어싱 | 그레이스케일 커버리지(`Format::Alpha`), LCD 서브픽셀 안 씀 | 같음 |
| 감마 | 없음. 커버리지를 알파로 쓰고 부호화 공간에서 섞음 | 같음(`over` 도 부호화 값 산술) |
| 알파 | straight alpha, 색은 `Color.a * 커버리지 / 255` | 흰 잉크 + `tint`, 흰 바탕 알파 0 이라 가장자리에 어두운 테두리가 끼지 않음 |
| 가변 폰트 축 | 요청한 굵기만 `wght` 로 셰이핑과 래스터에 적용한다. UI 는 `TextStyle` 로 굵기를 고르고 plain 호출은 face 자기 굵기(번들 Inter 는 400)다. `opsz` 는 기본 14 고정 | 같음(굵기 400, 스킨 `FontDef` 에 굵기 칸이 없다. 굵은 글씨는 정적 Bold TTF 를 별도 글꼴 id 로) |

힌팅이 실효 없는 이유: 번들 Inter 는 Inter 4.001 **가변 폰트**(`fvar`: `opsz` 14~32, `wght` 100~900)이고 `fpgm` 이 없으며 `maxp.maxSizeOfInstructions = 0`, `prep` 은 7바이트(`SCANCTRL`, `SCANTYPE` 만)다. swash 0.2.10 은 skrifa 0.44 의 `HintingInstance` 를 `HintingMode::Smooth{lcd_subpixel: Some(Horizontal), preserve_linear_metrics: true}` 로 만든다(`swash-0.2.10/src/scale/hinting_cache.rs`). skrifa 는 `prep` 이 비어 있지 않으면 인터프리터를 고르는데(`skrifa-0.44.0/src/outline/glyf/mod.rs:115` `prefer_interpreter`), 글리프에 명령이 없으니 윤곽이 그대로 나온다. 즉 정수 격자 맞춤 없이 해석적 커버리지로 구워진다. 이 판단은 폰트 테이블과 소스 읽기에 근거하고, 힌팅 켬/끔의 화소 비교는 하지 않았다.

가변 폰트의 굵은 서체: 번들 Inter 가 가변이라 같은 파일로 굵은 서체를 얻을 수 있다. cosmic-text 0.19 는 셰이핑에서도(`src/font/mod.rs:143` 가 `wght` 로 harfrust 위치를 만든다), 폰트 고르기에서도(`src/font/system.rs:41` `variable_weight_match`), 래스터에서도(`src/swash.rs`) `wght` 를 쓴다. 실측(프로브 `probe/src/bin/weight.rs`, 번들 Inter 하나만 든 데이터베이스, `HI-SPEED FIX` 24px): 굵기 400 -> 500 -> 600 -> 700 -> 800 에서 줄 폭 155.95 -> 156.57 -> 157.27 -> 157.88 -> 158.66px, 잉크 합(커버리지 합) 203,003 -> 239,126 -> 275,537 -> 310,178 -> 352,427(700 에서 +53%). 폭이 함께 자라므로 글자가 붙지 않는다. 조사 때 막혀 있던 엔진 API 는 구현됐다(2.1절): `TextStyle`/`*_styled` 가 굵기를 받고 레이아웃 캐시 키가 `family -> (px, weight) -> text` 다. `opsz` 축은 설정하지 않아 기본 14로 고정이다. 가변 축이 없는 사용자 글꼴(`--font`)에 굵기를 요청하면 fontdb 가 가장 가까운 컷을 골라 굵어지지 않을 수 있다(측정 안 함). 시스템 대체 글꼴(CJK)의 굵기 동작도 측정하지 않았다.

시스템 폰트: `TextContext::new()` 는 설치된 폰트를 스캔해 번들 Inter 가 없는 글자(CJK, 태국어, 아랍어 등)를 대체한다. 한자 매칭 로케일은 `ja` 로 고정(중국어 로케일 호스트만 예외, `font.rs:44-55`). 결정적 테스트는 `use_embedded_fonts_only()`(`font.rs:643`)로 번들 폰트만 쓴다.

## 6. 글꼴 파일과 크기 설정

| 파일 | 크기 | 내용 |
| --- | --- | --- |
| `assets/fonts/Inter-Regular.ttf` | 876,576 바이트, sha1 `97d4f3722d62c6cc5340bf6220ada3d64ca367b1` | Inter 4.001 가변, 2048 upem, glyph 2,933, 라이선스 SIL OFL 1.1(`assets/fonts/Inter-OFL.txt` 4,377 바이트) |
| `assets/skins/rbms-default/font/Inter-Regular.ttf` | 같은 파일(바이트 동일, `cmp` 확인) | 기본 스킨 팩이 복사본을 들고 있다. 바이너리 안에서는 `BUNDLED_FONT` 정적 하나를 공유한다(`font.rs:25`, `apps/rbms-player/build.rs:50`), 설치 때 팩 폴더에 써 낸다 |
| `assets/skins/rbms-default/font/Inter-Bold.ttf` | 420,428 바이트 | Inter 4.1 공식 zip(sha256 `9883fdd4...`)의 `extras/ttf/Inter-Bold.ttf`, 정적, 무수정(name: family Inter, Bold, OS/2 weight 700). `theme.font.bold = { id = "inter-bold", path = "font/Inter-Bold.ttf" }`(`theme.lua:255`). 라이선스는 같은 저자·같은 OFL 1.1(연도 표기만 2020/2016 차이)이라 `Inter-OFL.txt` 가 덮는다고 판단하고 THIRD-PARTY-NOTICES 3절에 Bold 문장을 더했다 |

- 바이너리에는 `include_bytes!` 로 876KB 가 들어간다. 스킨의 `inter` 글꼴과 내장 UI 글꼴은 같은 바이트라 `load_font` 가 같은 이름으로 알아본다(`font.rs:411`, 테스트 `block_tests.rs:308`).
- 글꼴 이름 규칙(리뷰 font-3): `TextContext::load_font` 가 face 에 주는 이름에서 가족 이름(`Inter`)은 그 가족의 일반 cut(굵기 400, 정자, 보통 폭)만 받고 다른 cut 은 `<가족> <굵기>`(예 `Inter 700`)다(`name_for` `font.rs:316-326`). 처음에는 먼저 올라온 face 가 가족 이름을 가졌고 기본 스킨의 글꼴 id 알파벳 순서(`inter` < `inter-bold`)가 Regular 를 지켜 주었을 뿐이라, 순서가 뒤집히면 내장 UI 전체가 굵어질 수 있었다. 테스트 `the_built_in_screens_stay_regular_whichever_cut_of_their_family_a_skin_loads_first`(기본 스킨의 실제 `Inter-Bold.ttf` 로 [없음]/[Regular, Bold]/[Bold, Regular]/[Bold 만] 네 경우의 폭과 화소가 같음).

- 사용자 글꼴(`Config.display.font_path`, SETTINGS 의 DISPLAY 묶음 FONT 행 / `--font`)은 내장 UI 의 우선 family 만 바꾼다(`set_ui_family`). 스킨 글자는 각 스킨의 글꼴 id 를 따른다.
- 내장 UI 의 굵은 서체는 번들 가변 Inter 의 `wght` 축으로 낸다(5절 실측, 새 에셋 없음). 스킨 글자용으로는 기본 스킨 팩에 정적 `Inter-Bold.ttf` 를 더했다(아래 표). 다국어 대체는 호스트 폰트에 의존한다.
- 크기 설정은 UI 쪽은 `px_for(scale)` 정수, 스킨 쪽은 문서의 `size`(design)와 목적지 높이(em)다. 코드에서 정하는 별도의 "글자 품질" 설정은 없다(`DisplayOptions` 에 글자 관련은 `font_path` 뿐).

## 7. 실증

모두 헤드리스다. 창을 띄우지 않았고, 실행은 `HOME` 과 `TMPDIR` 을 scratch 로 돌렸다. `rbms-player` 바이너리나 GUI 는 실행하지 않았다.

### 7.1 만든 방법

1. 앱 테스트 바이너리 빌드: `cargo test --locked -p rbms-player --no-run`(성공, 캐시 적중). 저장소에 남긴 변경은 이 문서 하나뿐이다(캡처와 프로브 산출물은 모두 scratchpad).
2. 기존 캡처 테스트를 직접 실행(작업 디렉터리 `apps/rbms-player`, `RBMS_SKIN_CAPTURE_DIR` 로 PNG 저장):
   - 내장 설정 화면 1280x720 과 1920x1080: `the_button_to_the_key_editor_is_drawn_where_it_is_pressed`, `every_group_and_state_of_the_screen_can_be_captured`
   - 내장 선곡 화면을 GPU 오프스크린(창 없음)으로: `a_capture_taken_on_the_gpu_agrees_with_the_headless_one`
   - 기본 스킨 결정 화면, 결과 화면을 1920x1080 네이티브로: `RBMS_SKIN_PACK=<저장소>/assets/skins/rbms-default` 로 `the_decide_scene_of_a_skin_pack_named_by_the_environment_is_captured_from_its_opening_to_its_fade`, `the_result_scene_of_a_skin_pack_named_by_the_environment_is_captured_for_a_clear_and_a_failure`
3. 프로브: scratch 에 별도 크레이트(렌더 크레이트를 경로 의존성으로, 렌더러 래퍼가 사각형 수를 세거나 `CpuCanvas` 에 그린다)를 만들어 같은 22행 메뉴 화면을 (a) 지금 경로(`ScaledRenderer` 위), (b) 물리 해상도로 직접 그린 경로로 그렸다. 같은 크레이트의 두 번째 바이너리(`probe/src/bin/weight.rs`)는 `cosmic-text 0.19.0` 을 직접 불러 번들 Inter 하나만 든 데이터베이스에서 굵기 400~800 의 줄 폭과 잉크 합을 쟀다(5절). 이 프로브는 저장소 밖이며 복사한 `Cargo.lock` 이 `--locked` 를 통과하지 못해(미사용 항목 가지치기도 갱신으로 본다) `cargo build --release --offline` 로 빌드했다(`cosmic-text 0.19.0`, `swash 0.2.10`, `skrifa 0.44.0` 은 저장소와 같다, `ron` 만 0.12.2 로 달라졌고 글자와 무관). 저장소의 `--locked` 규칙은 저장소 빌드(위 1번)에만 적용했다.

경로(이 세션 한정): `/private/tmp/claude-501/-Users-hyunseokbyun-development-R-BMS/35559457-6343-4207-8f85-ff5841239c87/scratchpad/font/` 아래 `cap/`(설정 화면), `cap2/`(GPU 선곡), `cap3/`(기본 스킨), `out/`(프로브 이미지), `probe/src/main.rs`(프로브), `pngtool.py`, `ppmtool.py`(crop 과 정수 배 확대).

### 7.2 관찰

- 내장 설정 화면 1280x720: 대문자 라벨 "JUDGE ALGORITHM" 의 글자 높이가 약 8~9화소이고 세로 줄기는 대부분 1화소 폭의 중간 밝기 회색이다. 완전한 밝은색 화소가 드물다(정량: 아래).
- 같은 화면 1920x1080(배율 1.5, 헤드리스): 같은 글자에서 1화소 폭과 2화소 폭이 번갈아 나오는 가로·세로 격자가 생기고 획 일부가 눈에 띄게 어둡다(`z-1920-label.png`). 일부는 `CpuCanvas` 의 floor/ceil 로 겹친 반투명 화소가 이중 합성된 헤드리스 특유의 어두움이다.
- 내장 선곡 화면 1920x1080(GPU 오프스크린, 실제 GPU 규칙): 하단 버튼 "SEARCH", "SORT: DEFAULT" 라벨이 같은 격자 무늬를 보인다. 획 폭이 1화소와 2화소로 불규칙하고 "/" 같은 사선이 계단지다(`z-gpu1920-small.png`). 1280x720 의 같은 라벨은 균일한 1화소 계열이다(`z-gpu1280-small.png`).
- 정수 2배(Retina 기본)는 정확히 재현된다. 프로브가 `ScaledRenderer` 를 2560x1440 `CpuCanvas` 위에 얹어 그린 화면(`ui-shipped-2x.ppm`)은 1280x720 화면을 최근접으로 2배 복제한 것과 **1440행 모두 바이트 동일**하다. 가로 막대(H, E 의 중간 획)가 2화소 두께의 덩어리가 되고 가장자리가 계단져 보인다(`z-shipped-2x.png`). 같은 문자열을 같은 위치에 물리 해상도로 직접 구운 화면(`ui-native-2x.ppm`)은 같은 확대에서도 획 두께가 일정하고 모서리가 부드럽다(`z-native-2x.png`).
- 정량(프로브, 메뉴 22행의 라벨 영역, 켜진 화소 중 커버리지 90% 이상 비율): 논리 래스터(지금 Retina 에서 확대되는 것) 19.4%, 물리 2배 래스터 43.2%. 커버리지 분포(0~20, 20~40, 40~60, 60~80, 80~100%): 논리 18.4/15.4/18.4/16.7/31.1, 물리 14.9/9.4/10.8/12.4/52.5. 지금 글자는 가장자리 화소 비중이 높아 "얇고 번진 느낌"이 난다.
- 스킨 글자 1920x1080(기본 스킨 결정 화면, 헤드리스): "CHART SELECTED"(caption 18px)가 획 두께 일정, 가장자리 매끄러움(`z-skin1920-small.png`). 같은 크기 대의 내장 글자와 대비해 격자 무늬가 없다.
- 감마·대비 모의: 물리 2배 래스터 이미지의 커버리지에 `a^(1/g)` 를 적용해 g = 1.0, 1.45, 1.8 을 쌓아 봤다(`z-native-2x-gamma-stack.png`). 효과는 획이 약간 굵어지는 정도라 미세하다. 물리 래스터 전환 앞에서는 부차적이다.

### 7.3 비용 측정 (프로브, 릴리스 빌드, `use_embedded_fonts_only`)

화면: 22행의 PLAY 메뉴 모사(라벨 scale 1.3, 값 1.4, 탭 1.5, 제목 2.4, 힌트 1.2, 비공백 글리프 329개). `Sink` 는 호출만 세는 렌더러다(GPU 제출 비용은 제외).

| 경로 | 창 크기(배율) | `fill_rect` 수 | 글리프당 | 인스턴스 바이트(32B) | CPU 제출 프레임당(캐시 적중) |
| --- | --- | --- | --- | --- | --- |
| 지금(논리 래스터, `ScaledRenderer`) | 1280x720 (1.0) | 12,081 | 36.7 | 387KB | 38.6us |
| 지금 | 1920x1080 (1.5) | 12,081 | 36.7 | 387KB | 35.7us |
| 지금 | 2560x1440 (2.0) | 12,081 | 36.7 | 387KB | 34.5us |
| 지금 | 3840x2160 (3.0) | 12,081 | 36.7 | 387KB | 33.2us |
| 물리 래스터 직접 | 1.0 | 12,081 | 36.7 | 387KB | 31.3us |
| 물리 래스터 직접 | 1.5 | 22,225 | 67.5 | 711KB | 36.3us |
| 물리 래스터 직접 | 2.0 | 32,308 | 98.2 | 1.03MB | 41.1us |
| 물리 래스터 직접 | 3.0 | 49,181 | 149.5 | 1.57MB | 50.9us |

- 처음 한 프레임(캐시 비어 있음, 글리프 래스터 포함)은 모든 경우 0.65~1.5ms 다. 창 크기를 끌어 바꾸는 동안 배율이 연속으로 바뀌면 프레임마다 이 정도가 다시 든다.
- 아틀라스 경로(글리프당 쿼드 1개, 64B)는 이 화면에서 329개, 약 21KB 다. 물리 래스터 직접(2배)의 3% 이하다.
- 플레이 화면의 비용 기준(`docs/reference/frame-cost.md`: 플레이 단계 갱신+그리기 p99 1ms 미만, 프레임 할당 256KB 미만)에는 영향이 없다. 기본 스킨으로 플레이할 때 글자는 스킨 텍스트(줄 텍스처 캐시)이고, 내장 글자가 플레이 중에 나오는 곳은 토스트, 디버그, ANALYSIS 오버레이(`stage/play/mod.rs:1278`)와, 스킨 문서가 없을 때의 내장 HUD 다. 결과 화면에서는 IR 줄(`stage/result.rs:1110,1251`)이 내장 글자다. 손대는 경로(A)는 캐시 적중 때 프레임당 할당이 없다(`pixels` 벡터는 캐시 미스에서만).

### 7.4 구현 뒤 측정과 검증 (2026-10-22)

| 항목 | 결과 |
| --- | --- |
| 22줄 메뉴(굵기 행 포함 화면, 릴리스, 호출만 세는 `Sink`)의 사각형 수와 캐시 적중 프레임 CPU 시간(W1a) | 1x 22,275개 / 45us, 1.25x 30,088 / 53us, 1.5x 37,978 / 56us, 2x 54,455 / 61us, 3x 82,305 / 72us. 7.3절의 12,081개 화면과 화면이 달라 수치를 직접 비교하지 않는다 |
| 이전/새 경로 비교(scratch 프로브, 릴리스) | 1x 는 바이트 동일, 1.5x·2x 는 다르다. 2배·3배 확대 크롭 PNG 에서 이전은 2x2 계단, 새는 매끄러운 획이고 1.5배에서 이전은 1·2화소 격자, 새는 일정한 획(눈으로 확인). 굵기 400/500/600/700 차이도 눈으로 확인 |
| `cargo test -p rbms-render --locked` | 875 통과 0 실패(골든 서명 10개 불변). 새 테스트 `primitives` 6, `font` 3, `whole` 1 |
| `cargo test -p rbms-player --locked --lib`, 강제 GPU 일치 | 2,615 통과(W1a 시점). `RBMS_REQUIRE_GPU=1` 일치 테스트 1920x1080 통과(어댑터 있음, 건너뛰지 않음) |
| 수리 font-1 (이미지 캐시) | 임시 되돌림으로 새 테스트 실패 확인 후 수정본 통과. 창을 끄는 동안 새 em 의 글리프를 굽는 CPU 비용(프레임당 수백 us~1 ms)은 그대로고 메모리 증가만 막았다. 같은 크기가 반복되는 동안 runs 캐시가 재사용된다. runs 캐시가 4096개를 넘어 밀려난 글리프는 이제 다시 굽는다(22행 메뉴는 4x 에서도 1,024개 이하) |
| 수리 font-2 (균등 판정) | 임시 복원으로 `(1306, 734)`(x1.02)와 `(1702, 958)` 이 비균등으로 오판되어 두 테스트 실패 확인 후 통과. 대표 뷰포트 1366x768, 1600x900, 1702x958, 2560x1440, 2880x1620, 3456x1944, x1.01~x3.00 의 1% 단위 200개 반올림 창이 모두 균등, 1920x720·2560x720·1280x1080·1920x1090·1920x1081 은 `None` |
| 수리 font-3 (이름 규칙) | 임시로 옛 `name_for` 로 되돌리면 폭이 `230.20752 != 226.6919` 로 실패, 수정본 통과 |
| 수리 뒤 `cargo test -p rbms-render --locked` | lib 785 + 골든 등 전부 통과, 골든 파일 변경 없음. 마지막 통합 게이트 Rust 6,269 통과·0 실패 |

실기(Retina 실제 창, Windows 125/150%, 전체 화면 비정수 배율)에서 눈으로 본 것은 없다. 근거는 헤드리스 이미지와 강제 GPU 일치 테스트(1920x1080)뿐이다.

## 8. 저화질 느낌의 원인 후보 순위

| 순위 | 원인 | 확실성 | 근거 | 영향 범위 |
| --- | --- | --- | --- | --- |
| 1 | 내장 UI 글자를 논리 해상도 정수 px 로 래스터한 뒤 확대(Retina 2배 = 화소 복제, 1.25/1.5/2.36배 = 불균일 격자) | 확정(코드, 헤드리스 바이트 비교, GPU 캡처) | `font.rs:316-349`, `canvas.rs:215,405`, `lib.rs:557`, 7.2 | 내장 화면 전부(설정, PLAY 메뉴, SKIN 메뉴, F1 오버레이 등). 선곡의 A, W, A+W 옵션 패널은 이 경로가 아니다: 패널 상태는 스킨이 그리는 선곡에만 있고(`stage/select/mod.rs:219`) 그림은 기본 스킨 Lua(`lua/select/sheet.lua`)가 스킨 TTF 경로 B 로 그린다. 패널을 내장 화면으로 새로 만들 때만 이 문제를 겪는다(종합 검토 정정) |
| 2 | 소형(9~15px, 대문자 높이 6.5~11px) + 굵기 400 만(굵기 API 없음) + 실효 없는 힌팅 + 그레이스케일 AA 만 | 확정(크기, 폰트 테이블), 영향 정도는 시각 판단 | 2.1 의 환산표, 5절 | 내장 UI 와 작은 스킨 글자 |
| 3 | 비 sRGB 프레임버퍼에서 커버리지를 부호화 공간에서 혼합(어두운 배경의 밝은 글자가 얇고 어두움) | 확정(구조), 체감 정도는 모의로 미세 | `gpu/mod.rs:5-12,241,577`, 7.2 감마 모의 | 모든 글자. 스킨 글자는 레퍼런스와 같은 부호화 공간 혼합이라 의도와 겹친다 |
| 4 | 비정수 배율에서 화소 규칙 불일치(GPU 는 화소 중심 규칙으로 1/2화소 폭 교대, 헤드리스는 floor/ceil 로 이중 합성) | 확정(GPU 캡처), 1위의 부분집합 | `cpu.rs:424-427`, 7.2 | Windows 125/150%, 전체 화면 비정수, 리사이즈 중 |
| 5 | 스킨 글자: 정수 반올림 배치(서브픽셀 없음), advance 를 design 크기 정수로 반올림해 배율을 곱함, `SHRINK` 로 눌린 줄은 밉맵 없는 선형 축소 | 코드 근거 확정, 영향은 작음(작은 em 에서 자간 불균일) | `block.rs:351,420,473`, `text.rs:383`, `lua/decide/main.lua:73` 등 | 기본 스킨 긴 제목, 결과 화면 곡명, 외부 스킨 |
| 6 | 아틀라스 해상도, 샘플러 필터, 알파 처리 | 해당 없음으로 판단 | 내장 글자는 아틀라스를 안 쓴다(2.4). 스킨 글자는 1:1 최근접, 흰 바탕 알파 0 이라 가장자리 오염 없음(`block.rs:81`). 텍스처에 밉맵이 없어도 글자 텍스처는 1:1 | 없음 |

사용자가 보고 있는 화면이 Retina 기본 창일 가능성이 가장 높고(이 저장소의 맥 번들과 해상도 행 기본값), 그 경우 1위가 단독으로 "저화질"을 설명한다. 확인하지 못한 것: 사용자 화면의 실제 배율, 실제 GPU 캡처의 2배 이미지(2배는 CPU 바이트 동일성으로 대체), Windows 125/150% 실측.

구현 뒤(2026-10-22): 순위 1(내장 UI 논리 래스터 후 확대)과 4(비정수 배율의 화소 격자)는 해소됐다. 2는 굵기 선택이 생겼고(5절) 소형·힌팅 없음은 그대로, 3(감마 공간 블렌딩)과 5(스킨 글자의 정수 스냅, `SHRINK` 선형 축소)는 건드리지 않았다.

## 9. 개선안

### O1. 내장 UI 글자를 물리 픽셀로 래스터 (1단계, 권장) — 구현됨(`f03beb3`, 수리 `f52e170`)

구현 결과는 2.1절이다. 아래는 조사 당시 설계이고 구현과 다른 곳: (1) `OffsetRenderer` 는 원점 이동을 더해 전달하지 않고 기본 (1,1) 을 유지한다. (2) 균등 판정은 `|sx - sy| < eps` 가 아니라 각 변을 따로 반올림한 오차 `0.5 x (1 + 긴 변/짧은 변)` 이내이고 배율은 `min(sx, sy)` 다. (3) baseline 은 `trunc(line_y) x s` 다. (4) `Laid` 가 `LayoutGlyph` 와 `line_y` 를 함께 저장한다. (5) 래스터는 swash 이미지 캐시를 쓰지 않는다. (6) 레이아웃 캐시 키에 굵기가 들어갔다(O4). 캐시 한도는 바꾸지 않았다(22줄 메뉴의 항목 수는 배율에 따라 늘지 않는다).

내용: 렌더러 트레이트가 "내 한 단위가 장치 화소 몇 개인가"를 알려 주고, 글자 엔진이 논리 글자 크기에 그 배율을 곱한 em 으로 구워 장치 화소 격자에 맞춘 사각형을 낸다.

- `Renderer` 에 기본 구현이 있는 메서드 두 개를 더한다: `fn device_scale(&self) -> (f32, f32)`(기본 `(1.0, 1.0)`)와 `fn fill_rect_device(&mut self, rect: Rect, color: Color)`(기본은 `device_scale` 이 1일 때 `fill_rect` 와 같고, 아니면 배율로 나눠 `fill_rect`). `ScaledRenderer` 는 `device_scale = inner.device_scale() * 자기 배율`, `fill_rect_device` 는 **배율 곱 없이** `inner.fill_rect_device` 로 전달한다. `Canvas`(`canvas.rs:396`)는 `native_size/UI_SIZE` 를 보고하고 `fill_rect_device` 는 `self.native().fill_rect` 로 보낸다. `RegionRenderer`(`region.rs:91`), `OffsetRenderer`(`skin_render/whole.rs:111`)는 원점 이동을 더해 전달한다. 이렇게 하면 `(물리/배율)*배율` 부동소수 오차가 `CpuCanvas` 의 floor/ceil 에서 1화소 새는 일이 없다.
- `TextContext::draw` 는 `device_scale` 이 균등(`|sx - sy| < eps`)이고 1 이 아닐 때만 새 경로를 탄다. 아니면(배율 1, 또는 letterbox 끔으로 비균등) 지금과 같은 코드를 그대로 탄다. 배율 1 에서 화소 단위로 지금과 같아야 한다는 것이 불변식이다.
- 새 경로: em 은 `px_for(scale) * 배율`을 **반올림하지 않고 그대로** 쓴다(레이아웃 폭이 `논리 폭 x 배율` 과 일치, 캐시 키는 `font_size_bits` 가 f32 라 분수 em 가능). 레이아웃은 한 번만 논리 px 로 셰이핑하고, `Laid` 에 `(x, y, font_id, glyph_id, weight, flags)` 를 f32 로 남겨 두어 호출마다 `CacheKey::new(.., em_물리, (x*배율 의 분수, 0), ..)` 로 서브픽셀 빈을 계산한다(레이아웃 캐시 두 벌을 피한다). 원점은 `round(x*배율)`, `round(y*배율)`. y 는 물리 정수로 반올림.
- 글리프 구간 캐시 `RUN_CACHE_LIMIT 4096` 와 레이아웃 캐시 한도는 물리 크기에서 항목이 늘어 한도 재검토(프로브에서 3x 사각형 수는 4.1배).
- 변경 파일: `crates/rbms-render/src/lib.rs`(트레이트와 `ScaledRenderer`), `region.rs`, `skin_render/whole.rs`, `font.rs`(`Laid`, `draw`, 새 경로), `apps/rbms-player/src/stage/canvas.rs`(`Canvas`, 테스트), 테스트 `crates/rbms-render/tests/primitives.rs`, `font.rs` 의 테스트 모듈.
- 새 의존성: 없음(cosmic-text, swash 이미 사용).
- 성능: 7.3. 사각형 수는 2배에서 2.7배, 3배에서 4.1배로 늘고 CPU 제출은 프레임당 50us 이내, GPU 인스턴스는 최대 1.6MB. 플레이 프레임 기준과 무관.
- 골든 영향: 배율 1 에서 불변이라 `crates/rbms-render/tests/golden_*.rs` 의 서명 상수 10개와 앱의 1280x720 픽셀 체크섬은 그대로다. 1920x1080 을 그리는 앱 테스트(설정 `DRAWN_SIZES`, 연습, `stage/capture.rs` 의 `a_larger_capture_is_the_same_screen_filling_the_larger_target`, GPU 일치 테스트)는 글자 래스터가 바뀌므로 다시 돌려야 한다(블록 서명 허용치 1단계, 정수 배 이상 차이 가능성은 낮음).
- 외부 Lua 스킨 영향: 없음. 스킨 글자는 B, C, D 경로이고 E, F 는 네이티브 렌더러(배율 1)라 지금과 같다.
- 위험: 창 크기를 끄는 동안 새 em 마다 래스터(프레임당 약 1ms). 비균등 배율은 레거시 경로 유지로 회피.

### O2. 내장 UI 글자를 아틀라스 텍스처 쿼드로 (2단계, 선택)

- 내용: `draw_text_atlas`(`font.rs:501`)를 앱 경로로 승격. 글리프 쿼드 1개가 1글리프(수십 개 사각형 대신).
- 바꿀 것: 아틀라스는 색별 RGBA 가 아니라 **흰 잉크 커버리지**를 `CacheKey` 키로 저장하고 `tint` 로 색을 입힌다(`block.rs` 의 방식). `glyphs.clone()` 제거. `GlyphAtlasBinding` 을 `TextContext` 가 들고, 프레임이 끝날 때 `GlyphAtlasBinding::end_frame`(`glyph_atlas.rs:270`)을 부르는 훅을 `Gpu::render` 근처(`gpu/mod.rs:855`)나 앱 프레임 끝에 추가(지연 제출 백엔드라 페이지가 커질 때 이전 페이지를 프레임 끝까지 살려 두어야 한다, `docs/acknowledge/reference-divergences.md` E-D21). `BatchKind::Textured` 는 텍스처가 바뀔 때마다 드로우 콜이 끊기므로 사각형 패널과 글자가 번갈아 나오는 화면은 드로우 콜이 늘어난다(수십 개 수준).
- 이득: 인스턴스 3% 이하, CPU·GPU 일치(텍스처는 양쪽이 화소 중심 샘플), `fill_rect` floor/ceil 불일치 소멸.
- 변경 파일: `font.rs`, `glyph_atlas.rs`, `canvas.rs`/`gpu/mod.rs`(프레임 끝 훅), 테스트 다수(`primitives.rs` 의 아틀라스 테스트는 이미 있음).
- 새 의존성 없음. 골든: 배율 1 에서 `draw_text_atlas` 가 `draw_text` 와 화소 동일하다는 기존 테스트(`primitives.rs:535-565`)가 있어 이를 유지하면 불변. 외부 스킨 영향 없음.
- 권장: O1 로 품질을 먼저 확보하고, 실기 프레임 시간이나 3배 이상 배율에서 인스턴스가 문제가 될 때 올린다.

### O3. 커버리지 감마·대비 보정 (선택)

- 내용: 글리프 커버리지에 `a' = a^(1/g)`(g 약 1.3~1.8) 룩업을 적용해 밝은 글자의 가장자리를 키운다. UI 경로는 `merge_runs` 에 넣기 전, 스킨 경로는 `block.rs` 의 `glyph_pixel`/`over` 에 넣는다.
- 변경 파일: `font.rs`(UI), `block.rs`(스킨, 선택).
- 새 의존성 없음. 성능은 래스터 때 1회(캐시됨).
- 골든 영향: **배율 1 에서도 화소가 바뀐다.** `golden_play`(3), `golden_result`(3), `golden_select`(1), `golden_shell`(3) 서명 상수 10개를 다시 만들어야 하고(실패 출력이 붙여 넣을 줄을 준다), 앱의 글자 포함 체크섬 비교 일부도 영향.
- 외부 스킨: 스킨 경로에 적용하면 레퍼런스(libGDX 의 부호화 공간 혼합)와 화소가 달라진다. UI 경로에만 적용하면 영향 없음.
- 효과가 7.2 모의에서 미세해서 O1 이후 실기로 판단하는 것을 권장.

### O4. 굵은 서체 선택 (선택, IIDX 식 패널과 관련) — 구현됨(`f03beb3`)

구현: `FontWeight`, `TextStyle`, `*_styled`, 레이아웃 캐시 키 `family -> (px, weight) -> text`(2.1절). 글리프 구간 캐시는 `CacheKey` 가 `font_weight` 와 em 비트를 이미 갖고 있어 그대로 뒀다. 새 패널은 `TextStyle` 로 굵기를 고르고 정렬·`fit` 도 같은 style 로 잰다. 스킨 글자(기본 스킨 선곡 옵션 패널)는 `FontDef` 에 굵기 칸이 없어 이 API 로 굵어지지 않는다. 대신 정적 `Inter-Bold.ttf` 를 팩 `font/` 에 더하는 방법(12절)을 썼다.

- 필요한 이유: 참고 이미지(IIDX 옵션 화면)의 라벨과 값은 굵은 서체다. 지금 UI 는 Regular 400 만 그린다.
- 새 에셋이 필요 없다: 번들 Inter 는 가변이고 cosmic-text 0.19 가 셰이핑과 래스터에서 `wght` 를 쓴다(5절 실측: 700 에서 폭 +1.2%, 잉크 +53%).
- 내용: 엔진에 굵기 개념을 노출한다. `Face.weight` 는 이미 있다(`font.rs:71`). 레이아웃 캐시 키(`font.rs:163`)와 글리프 구간 캐시(`CacheKey` 는 이미 `font_weight` 를 갖는다)에 굵기를 넣고, 내장 UI 호출에 굵기를 고르는 방법(예: `set_ui_weight` 또는 굵기를 받는 `draw_text` 변형)을 더한다. `opsz` 축(14~32)은 기본 14 이므로 큰 제목은 `Display` 광학 크기를 쓰지 못하는 점은 그대로 둔다.
- 스킨 쪽은 이번 범위 밖이다. 스킨 문서의 글꼴 항목(`FontDef`)에는 `id`, `path`, `fallback`, `type` 만 있고 굵기가 없다(`crates/rbms-skin/src/model.rs:345-351`).
- 변경 파일: `crates/rbms-render/src/font.rs`(캐시 키, API), 해당 UI 상수(새 패널), 테스트.
- 새 의존성: 없음, 에셋도 없음. 용량 증가 없음.
- 성능: 굵은 서체를 쓰는 문자열만큼 레이아웃·구간 캐시가 늘 뿐이다. 골든: 굵기 400 호출은 불변. 외부 Lua 스킨: 없음.
- 주의: 굵기가 커지면 advance 가 자라므로(실측 +1.2%) `fit_text` 와 정렬은 굵기를 넘겨 측정해야 한다.

### O5. 스킨 TTF 글자의 서브픽셀 위치와 SHRINK 래스터 (후속 후보)

- 내용: `block.rs` 의 `raster()` 에 x 4분 서브픽셀 빈을 주고(`CacheKey` 의 `x_bin`), `SHRINK` 로 눌린 줄은 선형 축소 대신 가로 배율을 적용해 다시 굽는다.
- 변경 파일: `block.rs`, `skin_render/text.rs`, `block_tests.rs`(위치 기대값 357줄 중 다수), 외부 스킨 캡처.
- 새 의존성 없음. 외부 스킨 영향: 글리프가 최대 0.5화소 이동하고 `SHRINK` 줄의 모양이 바뀐다. 레퍼런스 정합 문서(`docs/reference/skin-compat/b2-object-model.md` §12)와 대조해야 하는 레이아웃 계약이라 이번 범위로는 권장하지 않는다. `docs/quality-assurance` 에 테스트 부채로 남긴다.

### O6. SDF 또는 MSDF (비권장)

- 이점: 임의 배율에서 한 번만 굽고 외곽선·그림자를 거의 공짜로 얻는다(IIDX 식 외곽선 라벨).
- 이유로 비권장: 지금의 distance field 셰이더는 평활 폭이 텍스처 단위 `1/16` 고정이라(`lib.rs:319`) 소형 글자에서 가장자리가 1화소보다 좁아 계단이 생긴다. 소형 UI 에는 `fwidth` 기반 새 셰이더가 필요하고, CPU 캔버스(`distance_field_fragment`)와 같은 식을 유지해야 테스트 일치가 지켜진다. 생성기(거리 변환)도 직접 짜야 한다(swash 는 SDF 를 주지 않는다). 새 의존성 없이 가능하지만 파이프라인 4개(블렌드별)와 테스트를 더하는 큰 작업이고, 9~15px 소형 글자에는 직접 래스터가 더 낫다.

### O7. 글자 크기 상향 (설계)

- 지금 내장 라벨 대부분이 논리 9~14px 다(2.1). 새 옵션 패널은 행 이름·값에 12px(scale 1.4) 이상, 제목에 20px 이상을 쓰는 편이 안전하다. IIDX 참고 이미지는 값 글자가 행 높이의 약 40% 를 차지한다. 기존 화면 전체의 일괄 상향은 요청 범위 밖이다.

### 설정 노출

- 글자 래스터 해상도는 사용자가 고를 이유가 없다(항상 물리 해상도). 노출하지 않는다. O3 를 넣는다면 고정 상수로 두고 행을 만들지 않는 것을 권장한다.

## 10. 최소 검증 안 (구현자용)

1. 배율 1 불변: `ScaledRenderer` 없이 그린 화면과 새 경로가 `CpuCanvas` 에서 같다는 단위 테스트(`font.rs` 테스트 모듈), 기존 `cargo test --locked -p rbms-render`(골든 4개 파일, `primitives`, `font`)가 그대로 통과.
2. 배율 2: `ScaledRenderer` 위에 그린 결과가 `px_for(scale) * 2` em 의 직접 래스터와 화소 동일, 레이아웃 폭이 `논리 폭 x 2` 와 일치.
3. 장치 사각형 전달: `ScaledRenderer::fill_rect_device` 가 곱 없이 내려가는지(정수 좌표가 1화소도 새지 않는지, 배율 1.5 로).
4. 앱: `cargo test --locked -p rbms-player stage::capture::a_larger_capture_is_the_same_screen_filling_the_larger_target`, `stage::capture::a_capture_taken_on_the_gpu_agrees_with_the_headless_one`, 설정 화면 크기 테스트(`stage::settings`)를 1회. GPU 어댑터가 없는 환경은 후자가 통과한 척 건너뛴다(`RBMS_REQUIRE_GPU` 로 강제 가능).
5. 실기: Retina 기본 창과 전체 화면에서 설정, PLAY 메뉴를 눈으로. 비균등(letterbox 끔)은 레거시와 같은 모양인지.

## 11. 외부 Lua 스킨과 골든에 미치는 영향 요약

| 안 | 외부 Lua 스킨 | 골든·체크섬 |
| --- | --- | --- |
| O1 | 없음 | 배율 1 불변, 앱의 1.5배 크기 테스트만 재확인 |
| O2 | 없음 | 기존 아틀라스 동일성 테스트 유지, 프레임 끝 훅 테스트 추가 |
| O3 | UI 에만 적용하면 없음, 스킨 포함 시 레퍼런스와 화소 차이 | 서명 상수 10개 재생성 |
| O4 | 없음(스킨 문서 형식은 안 건드림) | 굵기 400 호출은 불변 |
| O5 | 글리프 위치 최대 0.5화소, SHRINK 모양 변화 | `block_tests.rs`, 외부 스킨 캡처 |

## 12. 문서와 코드의 차이

- `docs/architecture.md:76` 은 "텍스트는 cosmic-text 로 셰이핑하고 글리프 아틀라스로 래스터한다", `docs/crates.md:239` 는 "glyph_atlas: GPU glyph atlas for shaped text" 라고 쓴다. 코드의 앱 경로는 `fill_rect` 구간이고 아틀라스는 테스트에서만 부른다(2.4). 코드가 맞다. 구현 뒤 내장 UI 경로는 '장치 배율 em 으로 구운 켜진 화소 구간을 `fill_rect_device` 로 낸다' 로 더 달라졌고, 두 문서는 아직 고치지 않았다(이 문서 소유 밖).
- `docs/reference/ui-design.md:12` 는 "scale 단위 = 14px x scale" 이라고 쓰지만 코드는 `px_for(scale) = round(scale*8.5).max(8)`(`font.rs:61`)이다. `docs/font-cjk-support.md` 머리말은 코드와 일치한다.
- `docs/font-cjk-support.md` 와 `docs/reference/skin-compat/00-synthesis.md` 가 말하는 "물리 픽셀 래스터"는 스킨 TTF 에 해당하고 내장 UI 에는 해당하지 않는다. 두 구현이 공존한다는 점이 문서에 명시돼 있지 않다.
- 종합 검토 정정(2026-10-22): 이 문서 초안의 요약은 "새 A/W 옵션 패널도 내장 UI 경로" 라고 읽혔으나, 선곡 옵션 패널은 스킨이 그리는 선곡에만 있고(`apps/rbms-player/src/stage/select/mod.rs:219`, `PanelState`) 그림은 기본 스킨 `lua/select/sheet.lua` 가 스킨 TTF 경로 B(물리 px 래스터)로 그린다. 패널의 글자 품질은 O1 과 무관하고, 가독성은 `theme.type` 크기와 굵기가 정한다.
- 굵은 서체(O4)의 범위: O4 는 내장 UI 호출에만 굵기를 준다. 스킨 글자는 `FontDef`(`crates/rbms-skin/src/model.rs:345-351`)에 굵기 칸이 없어 O4 로 굵어지지 않는다. 스킨 패널을 굵게 하려면 (a) 정적 굵은 TTF(Inter Bold, OFL)를 스킨 팩 `font/` 에 더해 두 번째 글꼴 id 로 선언하거나(엔진 변경 없음: `load_font` 가 파일의 `info.weight` 로 별도 face 를 등록), (b) `FontDef` 에 R-BMS 확장 굵기 키를 더해야 한다(원본은 모르는 키를 무시). 구현은 (a) 를 택했다(`font/Inter-Bold.ttf`, 6절). 임시 테스트로 `TextContext::load_font` 가 Regular 를 `Inter`, Bold 를 `Inter 700` 별도 face 로 등록하고 폭이 다름(168.95 vs 171.03)을 확인한 뒤 삭제했다. Bold 가 가족 이름을 받지 않는다는 것은 `font.rs` 의 `the_built_in_screens_stay_regular_...` 가 확인한다.

## 13. 확인하지 못한 것과 한계

- 사용자 실제 화면(배율, 모니터)은 보지 못했다. Retina 2배는 `ScaledRenderer` 정수 배가 최근접 복제와 바이트 동일하다는 CPU 증거와, GPU 1.5배 캡처로 대신한다. Windows 125/150% 는 winit 기본 DPI 동작을 가정한 것이다.
- 헤드리스 비정수 배율 이미지는 GPU 와 화소가 다르다(4절). 1.5배 판단은 GPU 오프스크린 캡처(`cap2/`)를 우선 근거로 했다.
- 시스템 폰트 대체(일본어 등) 글자의 래스터 품질은 측정하지 않았다(모든 캡처가 번들 폰트만 쓴다).
- 힌팅 켬/끔의 화소 비교는 하지 않았다(폰트 테이블과 소스 근거만).
- 프로브의 시간은 호출만 세는 렌더러 기준이라 GPU 비용을 포함하지 않는다. 인스턴스 바이트는 구조체 크기에서 계산한 값이다.
- 스킨 글자의 실제 외부 스킨(ModernChic) 화면은 이 조사에서 그리지 않았다(글꼴 목록만 확인).

구현 뒤 추가(2026-10-22):

- 실기 확인 없음: Retina 실제 창, Windows 125/150%, 전체 화면 비정수 배율(7.4절).
- 창 크기를 끄는 동안 배율마다 새 em 의 글리프를 다시 굽는 CPU 비용(프레임당 수백 us~1 ms)은 그대로다. 격자 양자화는 글리프 크기가 최대 약 5% 틀어져 화소가 바뀌어 택하지 않았다.
- 아틀라스 경로(`glyph_atlas.rs` 의 `glyph_bitmap` 이 `with_pixels` 사용)는 장치 배율 글리프를 굽지 않고 앱에서 쓰이지 않아 한도 없는 증가는 재현되지 않으나 같은 이중 보관이 남아 있다. 필요해지면 `lit_pixels` 로 옮길 후보다.
- `device_text_scale` 의 여유는 렌더러 자신의 `size()` 모양(긴 변/짧은 변)에 의존한다. `Canvas` 와 `ScaledRenderer` 는 항상 1280x720 이라 16:9 다. 극단적으로 가늘고 긴 `RegionRenderer` 위 텍스트는 여유가 커져 균등으로 느슨하게 판정될 수 있다(그런 호출은 없다).
- 스킨 가족에 Regular 가 없고 Bold/Medium 만 있는 글꼴은 이제 `<가족> <굵기>` 이름을 받는다. 이름은 `load_font` 반환값으로만 쓰여 동작은 같으나 시작 로그 `font: path (family)` 의 이름이 바뀔 수 있다.
- 굵은 글꼴 파일 420 KB 가 내장 팩에 더해졌다(팩 10 MB 예산 안).
- 결과 화면 `PREVIOUS CLEAR` 글자색을 램프 색으로 바꾼 작은 시각 변화는 `clear-lamps.md` 4.2절.

## 14. 결정 지점

1. **적용 범위**
   - 선택지: A. 내장 UI 글자만(트레이트 수준이라 내장 화면 전부 자동 적용, 스킨 글자는 이미 물리 해상도라 손대지 않음) / B. A + 스킨 TTF 보완(서브픽셀, SHRINK, O5) / C. B + 감마·대비(O3).
   - 메인 추천: A.
   - 이유: 저화질의 확정 원인(1위)을 가장 작은 변경으로 없앤다. B, C 는 외부 스킨 레퍼런스 정합과 골든 재녹화 비용이 든다.
   - 영향 범위: A 는 `rbms-render` 트레이트·`font.rs`, `canvas.rs`. 스킨 팩과 외부 스킨은 불변.
   - 결정됨(2026-10-22, 메인이 정함, 사용자 미확인): A. 내장 UI 글자만(트레이트 수준이라 내장 화면 전부 자동 적용, 스킨 글자는 손대지 않음). 단 `OffsetRenderer` 는 일부러 전달하지 않아 외부 스킨의 스탠드인 글자·노트 라벨 화소가 바뀌지 않는다. ADR [UI 글자 장치 화소](../acknowledge/2026-10-22-adr-ui-text-device-pixels.md).
2. **구현 방식**
   - 선택지: A. 1단계로 기존 구간 경로를 물리 해상도로(O1), 아틀라스는 나중 / B. 처음부터 아틀라스(O1+O2) / C. 배율 1 경로만 유지하고 레거시 확대 유지(이번에 하지 않음).
   - 메인 추천: A.
   - 이유: 프레임 끝 훅과 렌더러별 바인딩 수명 관리가 필요한 B 는 변경이 크다. A 는 새 텍스처 수명이 없고 배율 1 에서 불변이라 위험이 작다. 7.3 의 비용으로 3배 배율에서도 CPU 제출은 50us 이내다.
   - 영향 범위: A 는 `font.rs`, `lib.rs`, `region.rs`, `whole.rs`, `canvas.rs`. B 는 `gpu/mod.rs` 프레임 끝 훅과 `glyph_atlas.rs` 가 더해진다.
   - 결정됨(2026-10-22, 메인이 정함, 사용자 미확인): A. 기존 구간 경로를 물리 해상도로(O1), 아틀라스는 나중.
3. **글자 크기 상향과 새 패널의 글자 기준**
   - 선택지: A. 새 옵션 패널부터 논리 12px 이상(scale 1.4+), 제목 20px 이상 / B. 내장 화면 전부 일괄 상향 / C. 현행 유지.
   - 메인 추천: A.
   - 이유: 일괄 상향은 레이아웃과 골든을 다 건드린다. 새 패널은 어차피 새로 짠다.
   - 영향 범위: A 는 새 패널의 상수, B 는 `preview_menu.rs`, `settings_view.rs` 등의 상수와 골든.
   - 종합 검토 정정: 선곡 옵션 패널은 내장 화면이 아니라 기본 스킨 Lua 가 1920x1080 문서 좌표로 그린다(8절 1위 행). 그래서 패널의 기준은 "논리 px" 가 아니라 `theme.type` 크기다. 1280x720 비 HiDPI 창에서 `caption` 18 은 12px, `label` 24 는 16px 이 되므로(2.2절) 패널의 값 글자는 `label`(24) 이상, 보조 글자는 `caption`(18)을 최소로 두는 것이 이 항목의 A 에 해당한다.
   - 결정됨(2026-10-22, 메인이 정함, 사용자 미확인): A. 새 옵션 패널부터 값 글자는 `label`(24) 이상, 보조는 `caption`(18) 이상으로 새로 짰다. 내장 화면의 일괄 상향은 하지 않았다.
4. **굵은 서체**
   - 선택지: A. 번들 Inter 의 가변 굵기 축으로 굵은 서체를 고르는 엔진 API 를 추가(새 에셋 없음, O4) / B. 정적 Bold TTF 를 따로 번들(용량 증가) / C. Regular 만 쓴다.
   - 메인 추천: A(새 패널의 값·헤더 용도. O1 이후 육안 비교 뒤 굵기 값을 정해도 된다).
   - 이유: 같은 파일로 굵기 400~800 이 셰이핑까지 정상 동작함을 실측했고, 새 에셋·빌드 스크립트 변경이 없다.
   - 영향 범위: `font.rs` 캐시 키와 API 만. 기본 스킨 팩과 외부 스킨은 불변.
   - 결정됨(2026-10-22, 메인이 정함, 사용자 미확인): A(내장 UI 는 가변 굵기 축 API, `TextStyle`)와 함께 스킨 글자용으로 B(정적 Bold TTF 번들, 420 KB)도 했다. 스킨 글자는 `FontDef` 에 굵기 칸이 없어 A 로는 굵어지지 않기 때문이다(12절).
5. **감마·대비 보정(O3) 적용과 설정 노출**
   - 선택지: A. 이번에는 하지 않고 O1 이후 실기로 판단 / B. UI 경로에 고정 상수로 적용 / C. DISPLAY 에 행 노출.
   - 메인 추천: A.
   - 이유: 모의에서 효과가 미세하고 배율 1 화소가 바뀌어 골든 10개를 재생성해야 한다. 노출은 사용자가 고를 이유가 없다.
   - 영향 범위: B, C 는 `font.rs`, 골든 10개, (C) `rbms-config` 와 설정 화면.
   - 결정됨(2026-10-22, 메인이 정함, 사용자 미확인): A. 하지 않았다.
6. **비균등 배율(letterbox 끔 + 비 16:9 창)의 글자**
   - 선택지: A. 레거시 확대(지금과 같은 모양) 유지 / B. 세로 배율 기준으로 균등 래스터 후 가로 위치만 곱함.
   - 메인 추천: A.
   - 이유: 글자가 가로로 늘어난 모양이 레터박스 끔의 원래 의미이고, B 는 간격 계산이 복잡하다.
   - 영향 범위: `font.rs` 의 분기 하나.
   - 결정됨(2026-10-22, 메인이 정함, 사용자 미확인): A. 레거시 확대 유지. 균등 판정의 여유는 각 변을 따로 반올림한 오차까지다.
7. **글자 품질·성능 설정 노출**
   - 선택지: A. 노출하지 않음(항상 물리 해상도) / B. "UI 글자 해상도" 행 추가.
   - 메인 추천: A.
   - 이유: 비용이 작고(7.3) 고를 이유가 없다.
   - 영향 범위: A 는 없음.
   - 결정됨(2026-10-22, 메인이 정함, 사용자 미확인): A. 노출하지 않았다.

### 14.1 사용자 확인 대기 목록(미확인)

1. 굵은 글꼴 에셋 `Inter-Bold.ttf`(420 KB) 추가와 라이선스 표기 방식.
2. `OffsetRenderer` 를 전달에서 뺀 선택(외부 스킨 화소 불변을 우선한 것).
3. 실기(Retina 창, Windows 125/150%, 전체 화면)에서 글자 모양과 창 끌기 중 프레임 시간.
