# ADR: 내장 UI 글자를 장치 배율의 물리 화소로 래스터, 굵기 선택, Inter Bold 에셋

> 최종 갱신 2026-10-22 · 대응 커밋 47a7f9c
> 상태: 유효(사용자 미확인) · 결정일 2026-10-22 · 결정자 메인(사용자 지시 "끝까지 다 구현해놔"에 따라 추천안 채택, 사용자 미확인) · 커밋 f03beb3(엔진), 0c450db(Inter Bold 에셋과 스킨 쪽 굵은 글꼴), f52e170(수리)
> 관계: 스킨 TTF 글자(경로 B)는 원래 화면에 닿는 크기로 래스터되어 이 ADR 의 변경 대상이 아니다. 굵은 글꼴이 보이는 곳은 [옵션 패널 ADR](2026-10-22-adr-option-panel-layout.md) 의 기본 스킨 패널이다.

## 배경

- 사용자 지시(2026-10-22, 요지. 원문이 아니라 `docs/PROCESS.md` 의 기록): 글꼴 가독성을 높인다(안티에일리어싱, 저화질 느낌 해소).
- 내장 화면(설정, PLAY 메뉴, SKIN 메뉴, 키 설정, 폴더·표, 연습, 로딩, 토스트, 선곡 위의 검색·필터·가이드·힌트, IR 순위 패널, 디버그)의 글자는 논리 1280x720 기준 정수 px(9~26)로 래스터한 뒤 글리프의 점 하나하나를 `fill_rect` 로 내고 `ScaledRenderer` 가 좌표에 배율을 곱했다. 창의 물리 크기가 논리의 2배(macOS Retina 기본 창)이면 글자 화소 하나가 2x2 블록이 되고, 1.5배(Windows 150%, 1920x1080)이면 1화소와 2화소 폭이 번갈아 나오는 격자가 됐다. 래스터는 물리 해상도로 한 번도 일어나지 않았다(확정: 코드, 헤드리스 바이트 비교, GPU 캡처. [font-rendering.md](../reference/font-rendering.md) 1·8절).
- 그 위에 원인이 겹쳤다: 소형 크기(9~15px), 굵기 400 하나(굵기를 고르는 API 가 없었다), 실효 없는 힌팅, 그레이스케일 커버리지만 쓰는 래스터, 비 sRGB 프레임버퍼에서의 부호화 값 혼합.
- 선곡의 옵션 패널은 내장 화면이 아니라 기본 스킨 Lua 가 스킨 TTF 경로로 그린다. 스킨 글꼴 정의(`FontDef`)에는 `id`·`path`·`fallback`·`type` 만 있고 굵기가 없다.

## 결정

### 1) 렌더러가 장치 배율을 알려 주고 글자 엔진이 물리 크기로 굽는다

- `Renderer` 트레이트에 기본 구현이 있는 메서드 둘을 더했다(`crates/rbms-render/src/lib.rs`): `device_scale() -> (f32, f32)`(이 렌더러의 한 단위가 맨 아래 타깃 화소 몇 개인가, 기본 `(1.0, 1.0)`)와 `fill_rect_device(rect, color)`(타깃 화소 좌표 사각형, 기본은 `device_scale` 로 나눠 `fill_rect`).
  - `ScaledRenderer` 는 `device_scale` 에 자기 배율을 곱해 보고하고 `fill_rect_device` 는 곱 없이 안쪽으로 전달한다. `RegionRenderer`(`region.rs`)는 영역 원점에 배율을 곱해 반올림한 값을 더해 전달한다(정수 격자 유지). 앱의 `Canvas`(`apps/rbms-player/src/stage/canvas.rs`)는 `native_size / UI_SIZE` 를 보고하고 `fill_rect_device` 는 `native().fill_rect` 로 보낸다. `NativeCanvas`·`HeadlessCanvas`·`Gpu` 는 기본값이다.
  - `OffsetRenderer`(스킨 `OFFSET_ALL` 늘이기)는 일부러 기본값 `(1, 1)` 을 유지한다. 전달하면 스탠드인 글자와 노트 라벨이 `OFFSET_ALL` 늘이기에서 화소가 바뀌어 "외부 Lua 스킨의 화소 불변" 비목표를 어긴다. 조사 문서 9절 O1 의 "OffsetRenderer 도 원점 이동을 더해 전달" 과 다른 선택이고 테스트 `text_drawn_through_a_stretch_is_the_unstretched_text_enlarged`(`skin_render/whole.rs`)가 고정한다.
- `device_text_scale`(`crates/rbms-render/src/font.rs:175`)가 새 경로를 탈지 정한다. `(1, 1)` 이면 `None`(기존 코드 그대로)이고, 두 축의 차이가 반올림으로 설명되는 범위 안이면 `Some(min(sx, sy))`, 아니면 `None` 이다. 판정식은 `|sx - sy| x 긴 변 <= 0.5 x (1 + 긴 변 / 짧은 변)` 이다. 각 변을 따로 반올림한 창은 긴 변이 0.5, 짧은 변이 0.5 틀어지고 이를 긴 변 기준으로 재면 16:9 에서 약 1.39 화소까지 나오기 때문이다. 처음 구현의 여유 1.0 화소는 1280x720 의 x1.33 을 가로세로 따로 반올림한 1702x958 을 비균등으로 오판해 글자가 옛 늘림 경로로 돌아갔다(통합 리뷰 font-2). 레터박스를 끈 비균등 창은 지금처럼 늘린 글자를 쓴다(조사 14절 결정 6 A: 글자가 가로로 늘어난 모양이 레터박스 끔의 원래 의미).
- 새 경로의 산술: 레이아웃은 논리 px 로 한 번만 셰이핑하고 `Laid` 에 글리프와 줄 baseline 을 함께 보관한다. 그릴 때 글리프 크기 em 은 `px x s` 를 반올림 없이 쓰고(레이아웃 폭이 `논리 폭 x 배율` 과 일치), 서브픽셀 빈은 장치 좌표 기준이다. 원점은 `round(x x sx)`, `round(y x sy)`, baseline 은 `trunc(line_y) x s` 로 둔다(`line_y x s` 로 두면 1.5배에서 레거시 확대보다 약 1.5 장치 화소 내려가는 것을 실측했다).
- 배율 1 에서는 화소 단위로 이전과 같다. `golden_play`·`golden_result`·`golden_select`·`golden_shell` 의 서명 상수 10개가 그대로이고 1x 헤드리스 출력이 이전과 바이트 동일하다. 1.5배와 2배는 달라진다(2x2 계단과 1·2화소 격자가 사라진다). 새 의존성은 없다(cosmic-text, swash 이미 사용).
- 래스터는 `SwashCache::with_pixels` 대신 `get_image_uncached` + `lit_pixels` 로 구워 한도 있는 구간 캐시(`RUN_CACHE_LIMIT` 4096)만 쓴다(`font.rs` `GlyphPainter::paint`, `lit_pixels`). Mask 는 커버리지를 기본 색의 잉크로, Color 는 rgba 그대로 내고 SubpixelMask 는 그리지 않는다(cosmic-text 0.19 `with_pixels` 와 같은 뜻). 창 크기를 끄는 동안은 프레임마다 다른 분수 배율 때문에 모든 글리프의 `CacheKey` 가 새로 생기는데, 이전에는 swash 의 한도 없는 이미지 캐시에 쌓였다. 격리 사본의 측정에서 22줄 메뉴를 1,000개 창 크기로 그리면 이미지가 279개에서 271,503개(마스크 30.5MB)가 되고 RSS 가 14.6MB 에서 139.2MB 가 됐다(통합 리뷰 font-1). 지금은 이미지 캐시 0개, 구간 캐시는 한도 안이다. 테스트 `a_window_drag_rasterizes_past_the_rasterizers_unbounded_image_cache`.
- 비용: 22줄 메뉴 기준 사각형 수가 1x 22,275개, 1.25x 30,088, 1.5x 37,978, 2x 54,455, 3x 82,305 로 늘고 캐시가 맞은 프레임의 CPU 제출 시간은 45 → 53 → 56 → 61 → 72 us 다(릴리스 프로브). 캐시 항목 수는 배율에 따라 늘지 않고 항목당 run 수만 는다(레이아웃·구간 22·328 / 22·326 / 22·325, 테스트 `an_enlarged_menu_fits_both_caches_as_the_small_one_does`). 창을 끄는 동안 새 em 의 첫 래스터는 프레임 시간이 튄다(프로브 첫 프레임 1.3~2.4 ms).

### 2) 굵기를 고르는 API

- `FontWeight(u16)`(`REGULAR` 400, `MEDIUM` 500, `SEMIBOLD` 600, `BOLD` 700)와 `TextStyle { scale, weight }`(`TextStyle::new`, `TextStyle::regular`)를 `rbms-render` 가 내보낸다. `TextContext` 와 자유 함수에 `text_width_styled`·`draw_text_styled`·`draw_text_centered_styled`·`draw_text_right_styled`·`fit_text_styled` 가 있다. 인자를 따로 받으면 `clippy::too_many_arguments`(8개)에 걸리고 새 `#[allow]` 는 금지라서 `TextStyle` 로 묶었다.
- 굵기를 쓰는 줄은 정렬과 `fit` 도 같은 style 로 재야 한다(굵을수록 폭이 넓다: 700 에서 +1.2%). 레이아웃 캐시 키는 `(px, 굵기)`이고 굵기를 요청하지 않은 줄은 face 자기 굵기 키를 쓴다. 굵기 400 호출은 이전과 같은 화소다.
- 내장 화면은 이 API 를 아직 쓰지 않는다(호출은 `rbms-render` 안의 테스트뿐이다). 굵은 글자가 실제로 보이는 곳은 아래 3) 의 기본 스킨 패널이다.

### 3) Inter Bold 에셋 (기본 스킨 팩)

- `assets/skins/rbms-default/font/Inter-Bold.ttf`(420,428 바이트)를 더했다. Inter 4.1 공식 `Inter-4.1.zip`(sha256 앞 8자리 `9883fdd4`)의 `extras/ttf/Inter-Bold.ttf` 정적 컷이고 수정하지 않았다(family Inter, Bold, OS/2 weight 700). 라이선스는 SIL OFL 1.1 이다. `Inter-OFL.txt` 는 같은 저자·같은 OFL 1.1(이름표의 저작권 연도만 2016, 본문 2020 으로 다르다)이라 두 컷을 함께 덮는다고 판단하고 `THIRD-PARTY-NOTICES.md` 3절에 Bold 문장을 더했다.
- 왜 정적 파일을 추가했나: 조사 문서는 번들 Inter 가 가변 폰트라 새 에셋 없이 굵기를 고를 수 있다고 적었고(결정 4 A) 그것이 위 2) 의 내장 UI 경로다. 스킨 TTF 경로(`block.rs`)는 굵기를 고를 수 없고 `FontDef` 에 굵기 항목이 없어, 기본 스킨 패널이 굵은 글자를 쓰려면 굵은 컷이 별도 글꼴 파일이어야 했다.
- Lua 쪽: `theme.font.bold = { id = "inter-bold", path = "font/Inter-Bold.ttf" }`(`lua/theme.lua`), `Builder:font_id(weight)`(`lua/ui.lua`)가 `weight == "bold"` 인 글자 명세가 있을 때만 그 문서의 글꼴 목록에 한 번 추가한다. 굵은 글자를 쓰지 않는 문서는 이전과 같다. 굵은 글자는 패널의 옵션 이름·강조 값·INFORMATION 제목과 값·태그·건반 표지에 쓰고, 크기는 값 24 이상, 보조 18 이상(문서 단위, 1280x720 창에서 2/3 배)이다(조사 14절 결정 3 A 의 기준).
- 글꼴 이름 규칙(`TextContext::name_for`): 한 가족의 일반 컷(굵기 400, 정자, 보통 폭)만 가족 이름을 받고 다른 컷은 `<가족> <굵기>` 이름을 받는다(Bold 는 `Inter 700`). 처음 구현은 "먼저 올라온 face 가 가족 이름을 갖는다"여서 기본 UI 가 Regular 로 남는 이유가 글꼴 id 의 알파벳 순서(`inter` < `inter-bold`) 하나뿐이었다. 순서가 뒤집히면 내장 UI 전체가 굵어졌다(통합 리뷰 font-3). 지금은 Bold 를 먼저 올리거나 Bold 만 올려도 내장 UI 는 변하지 않는다. 테스트 `the_built_in_screens_stay_regular_whichever_cut_of_their_family_a_skin_loads_first` 가 기본 스킨의 실제 `Inter-Bold.ttf` 로 네 경우의 폭과 화소를 고정한다.
- 팩 크기 영향: 420 KB 가 늘었다(10 MB 예산 안).

### 4) 하지 않은 것

[font-rendering.md](../reference/font-rendering.md) 9절의 선택 항목이다.

- 커버리지 감마·대비 보정(O3): 하지 않았다. 모의에서 효과가 미세하고, 배율 1 에서도 화소가 바뀌어 골든 서명 상수 10개를 다시 만들어야 하며, 스킨 경로에 적용하면 원본(libGDX 의 부호화 공간 혼합)과 화소가 달라진다. 조사는 O1 이후 실기로 판단하라고 했다. 설정 행으로 노출하지 않는다(결정 5 A, 7 A).
- 서브픽셀: 두 가지 모두 하지 않았다. (a) 스킨 TTF 글자의 서브픽셀 배치와 `SHRINK` 줄의 래스터(O5)는 글리프가 최대 0.5화소 움직이고 레퍼런스 정합 문서의 레이아웃 계약과 대조해야 해서 이번 범위에서 뺐다. (b) LCD 서브픽셀 렌더링은 쓰지 않는다. 커버리지는 그레이스케일 하나이고 `SubpixelMask` 이미지는 그리지 않는다.
- 아틀라스 텍스처 쿼드(O2), SDF·MSDF(O6, 비권장), 내장 화면 전체의 글자 크기 일괄 상향(O7, 범위 밖)도 하지 않았다.
- 테스트 부채: 감마·대비 보정(O3)과 스킨 TTF 서브픽셀 배치(O5)는 즉시 시험하지 않고 [테스트 부채 문서](../quality-assurance/2026-10-14-test-debt.md)에 올릴 대상이다. 재현 조건은 Retina 2배 창과 Windows 125/150% 창에서 내장 화면 글자의 가장자리 두께를 눈으로 보는 것이다. 생략 이유는 모의에서 효과가 미세하고 배율 1 화소가 바뀌어 골든 서명 10개를 다시 만들어야 하는 것(O3), 레퍼런스 정합 계약과 대조해야 하는 것(O5)이다. 남은 위험은 어두운 배경 위 밝은 글자의 가장자리가 얇게 보이는 것이다. 실행이 필요해지는 시점은 사용자가 O1 이후에도 글자가 얇다고 보고할 때다. 항목은 [테스트 부채 3.87](../quality-assurance/2026-10-14-test-debt.md)이다.

## 이유

- 저화질의 확정 원인(조사 8절 1위)을 가장 작은 변경으로 없앤다. 내장 화면 전부가 트레이트 수준에서 자동 적용되고 스킨 글자는 이미 물리 해상도라 손대지 않는다(결정 1 A).
- 구간 경로를 물리 해상도로 굽는 방식은 새 텍스처 수명이 없고 배율 1 에서 불변이라 위험이 작다. 아틀라스 승격은 프레임 끝 훅과 렌더러별 바인딩 수명 관리가 필요해 큰 변경이다(결정 2 A).
- 3배 배율에서도 CPU 제출은 프레임당 72 us 라 플레이 프레임 기준과 무관하다. 외부 Lua 스킨은 영향이 없다: 스킨 글자는 경로 B·C·D 이고 E·F 는 배율 1 의 네이티브 렌더러다.
- 글자 품질 해상도는 사용자가 고를 이유가 없어 노출하지 않는다.

## 기각된 대안

| 대안 | 기각 이유 |
| --- | --- |
| 처음부터 아틀라스 텍스처 쿼드(O1 + O2, 결정 2 B) | 프레임 끝 훅과 바인딩 수명 관리가 필요해 변경이 크다. 이득은 인스턴스 수와 CPU·GPU 일치인데 O1 로 품질이 먼저 확보된다 |
| 감마·대비 보정을 UI 경로에 고정 상수로(결정 5 B) | 배율 1 에서 화소가 바뀌어 골든 10개를 다시 만들어야 하고 모의에서 효과가 미세하다 |
| 레터박스 끔 + 비 16:9 창에서도 세로 배율 기준으로 균등 래스터(결정 6 B) | 간격 계산이 복잡하고 글자가 가로로 늘어난 모양이 레터박스 끔의 원래 의미다 |
| 정적 Bold TTF 를 내장 UI 에도 번들(결정 4 B) | 내장 UI 는 가변 Inter 의 굵기 축으로 충분하다. 정적 컷은 스킨 TTF 경로에만 필요했다 |
| 래스터 배율을 1/32 같은 격자로 양자화해 이미지 캐시 증가를 막음 | 글리프 크기가 최대 약 5% 틀어져 화소가 바뀐다 |
| 내장 글자 크기를 일괄 상향(결정 3 B) | 레이아웃과 골든을 모두 건드린다. 새 패널의 글자 기준(값 24 이상, 보조 18 이상)만 정했다 |

## 영향과 관련 파일

- 엔진: `crates/rbms-render/src/{lib.rs,font.rs,region.rs}`, `crates/rbms-render/src/skin_render/whole.rs`, `apps/rbms-player/src/stage/canvas.rs`.
- 에셋·고지: `assets/skins/rbms-default/font/Inter-Bold.ttf`, `THIRD-PARTY-NOTICES.md` 3절, `lua/{theme,ui}.lua`.
- 테스트: `font.rs` 의 `a_window_drag_rasterizes_past_the_rasterizers_unbounded_image_cache`·`the_built_in_screens_stay_regular_whichever_cut_of_their_family_a_skin_loads_first`·`an_enlarged_menu_fits_both_caches_as_the_small_one_does`·균등 판정 테스트(1366x768, 1600x900, 1702x958, 2560x1440, 2880x1620, 3456x1944 와 x1.01~x3.00 의 1% 단위 200개 반올림 창이 균등, 1920x720·2560x720·1280x1080·1920x1090·1920x1081 은 비균등), `tests/primitives.rs` 의 `the_glyph_atlas_draws_the_same_pixels_as_the_fill_path`, 1920x1080 앱 캡처·강제 GPU 일치 테스트(`RBMS_REQUIRE_GPU=1`, 허용치 안에서 일치).
- 사용자 확인이 필요한 것(미확인): 실제 창에서 글자가 나아졌는지. Retina 실제 창, Windows 125/150%, 전체 화면의 비정수 배율은 눈으로 보지 않았고 근거는 헤드리스 이미지와 강제 GPU 일치 테스트(1920x1080)뿐이다. 감마·대비 보정(O3)을 넣을지는 실기를 본 뒤 정한다.
- 남은 위험:
  - 창 크기를 끄는 동안 배율마다 글리프를 다시 굽는 CPU 비용(프레임당 수백 us~1 ms)은 그대로다. 메모리 증가만 막았다. 구간 캐시가 4,096개를 넘어 밀려난 글리프는 이전에는 swash 이미지 캐시에서 되살렸지만 이제 다시 굽는다(22줄 메뉴는 4x 에서도 1,024개 이하).
  - 아틀라스 경로(`glyph_atlas.rs` 의 `glyph_bitmap`)는 아직 `with_pixels` 를 쓴다. 앱이 쓰지 않아 재현되지 않지만 같은 이중 보관이 남는다.
  - 가변 축이 없는 사용자 글꼴(`--font`)에 굵기를 요청하면 fontdb 가 가장 가까운 컷을 고르므로 굵어지지 않을 수 있다. 시스템 대체 글꼴(CJK)의 굵기 동작은 재지 않았다. 가족에 Regular 가 없고 굵은 컷만 있는 글꼴은 `<가족> <굵기>` 이름을 받아 시작 로그의 `font: path (family)` 이름이 바뀔 수 있다.
  - `device_text_scale` 의 여유는 렌더러 자신의 `size()` 모양에 의존한다. 극단적으로 가늘고 긴 `RegionRenderer` 위 글자는 약간 느슨하게 균등으로 판정될 수 있다(지금 그런 호출은 없다).
  - 새 경로의 baseline 이 `trunc(line_y) x s` 라서 물리 해상도의 1/s 화소 단위 baseline 정밀도는 쓰지 않는다.
  - `Laid` 가 글리프 사본을 더 들어 레이아웃 캐시 메모리가 늘었다(추정 1MB 이하, 측정하지 않았다).
- 문서: 조사 문서 2.1·3·4·5·9절(O1·O4)·12·14절은 구현 전 서술이라 고쳐야 한다(OffsetRenderer 는 전달하지 않음, 균등 판정식, 이미지 캐시 우회, 이름 규칙). `docs/architecture.md`·`docs/crates.md` 의 글자 경로 서술도 같다.

## 날짜와 사용자 원문

2026-10-22.

- 지시 요지(`docs/PROCESS.md` 의 기록, 원문 아님): "(5) 글꼴 가독성(안티에일리어싱, 저화질 느낌 해소)".
- 설계 확정 지시: "끝까지 다 구현해놔"
