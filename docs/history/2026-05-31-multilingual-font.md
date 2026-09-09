# 2026-05-31 (세션 B) — 다국어 폰트 지원 (cosmic-text)

기존 5×7 ASCII 비트맵(`font.rs`)이라 일본어/한국어/★ 등 비ASCII가 전부 공백이던 문제. 사용자 요구:
모든 언어·더 나은 룩·MIT 라이선스·폰트 교체 가능·향후 웹폰트. 레퍼런스 구현 참고.

## 레퍼런스 구현 폰트 시스템 (대조)
`Config.systemfontpath`(기본 VL-Gothic-Regular.ttf 4MB) **설정형 단일 폰트** + libGDX FreeType 런타임 래스터(`SkinTextFont`, `parameter.characters=text` 문자열 단위 지연 글리프) + `GlyphLayout`(정렬/줄바꿈/overflow/그림자). **자동 다중폰트 폴백 없음** → 단일 폰트 커버 언어만. 우리는 폴백/시스템폰트로 한 단계 개선.

## 결정
- 엔진 **cosmic-text 0.19**(MIT/Apache, 셰이핑+bidi+fontdb 폴백+시스템폰트+swash AA). 요구 5개를 단일 스택으로 충족.
- 통합 **알파 사각형**: `SwashCache::with_pixels` 픽셀 콜백 → 기존 `Renderer::fill_rect`. GPU 파이프라인/Renderer trait/CpuCanvas 무변경, `draw_text` 78 호출처 시그니처 유지.
- 번들 **Inter(OFL, 876KB) + 시스템 폴백**(A1). 맥/윈 시스템 CJK로 모든 언어.

## 구현
### P1 — `crates/rbms-render/src/font.rs` 전면 교체
- thread_local `TextEngine{ fs:FontSystem, swash:SwashCache, family, default_family, cache:HashMap<(String,u32),Laid> }`.
- `Laid{ width, glyphs:Vec<(i32,i32,CacheKey)> }` — `(text,px)`별 1회 셰이핑 캐시.
- `px_for(scale)=round(scale*8.5).max(8)`로 레거시 scale→픽셀 매핑(현 크기감 유지).
- `Attrs::new().family(Family::Name(번들패밀리))` + `Shaping::Advanced`, `set_size(None,None)`(무줄바꿈), `shape_until_scroll`.
- 그리기: glyph `physical((0,line_y),1.0)` 펜위치 + `with_pixels(base=틴트)` 픽셀당 `fill_rect`(알파=커버리지×틴트α). Mask=틴트, Color(이모지)=rgba 그대로 — base에 틴트 rgb를 넘기므로 둘 다 자연 처리.
- `assets/fonts/Inter-Regular.ttf`(`include_bytes!`) + `Inter-OFL.txt`.
- **검증**: `examples/text_render.rs`→CpuCanvas PNG. 일/한/중(간·번)/키릴·그리스·베트남/태국·아랍(RTL)·데바나가리/★·화살표 전부 AA 렌더 확인(헤드리스). 단위테스트 `shapes_and_rasterizes_multilingual_text`.

### P2 — 폰트 교체
- rbms-render: `load_font(Vec<u8>)->Option<family>`·`set_ui_family(name)`(캐시 클리어)·`reset_ui_family()` export.
- player: `PlaySettings.font_path:Option<String>`(`#[serde(default)]`) + `--font PATH` + main() 시작 로드. 설정 DISPLAY 탭 `FONT`(idx 18): Enter/우/클릭=rfd 파일선택 라이브 적용(`pick_font`), 좌=기본복귀(`reset_font`). `load()` 무관 — 텍스트 엔진은 전역.
- **검증**: `--font "Arial Black.ttf"` 로드·family 설정·무패닉 스모크.

## 결과
모든 언어 렌더(이전 공백). `cargo test --workspace` 100/0, debug/release 빌드 경고 0.

## 후속
- P3: 픽셀당 quad→행 run-length 병합(quad↓), 캐시키 `to_string` 할당 제거, 패널 말줄임, `px_for`/세로정렬 인앱 튜닝.
- P4: 웹폰트(URL→bytes→load_font+캐시).
