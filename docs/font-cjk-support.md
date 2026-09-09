# CJK(일본어) 폰트 지원 — 아이디에이션

## 대상 / 현 상태
- `crates/rbms-render/src/font.rs`: 하드코딩된 **5×7 ASCII 비트맵**(`glyph(c)->[u8;7]`). 비ASCII는 `[0;7]`=공백.
- 텍스트는 `Renderer::fill_rect`로 픽셀당 사각형 1개씩 그림(전용 텍스처 파이프라인 없음, BGA만 텍스처).
- `draw_text`/`text_width` **호출처 78곳** → 시그니처 유지 필수.
- 증상: BMS 일본어 제목·난이도표명·★ 등이 빈칸(PROCESS §7 1순위 한계).

## 목표
임의의 일본어(JIS 한자+가나)·기호(★ 등)를 **여러 UI 크기**(scale 1.2~4.5)에서 렌더.

## 결정 포인트 ① — 글리프 렌더링 통합
| 방식 | 내용 | 장점 | 단점 |
|---|---|---|---|
| **A. 래스터→알파 사각형 (fill_rect 재사용)** ⭐ | TTF를 글자별로 커버리지 비트맵으로 래스터(캐시), 픽셀당 `fill_rect`(알파=커버리지) | **파이프라인/Renderer trait 무변경**, GPU·CpuCanvas·테스트 그대로, 78 호출처 무손실, AA 가능 | 글자당 quad 多(UI는 무방, 플레이필드는 텍스트 적음) |
| B. 글리프 아틀라스 텍스처 | 아틀라스 텍스처에 글리프 패킹, 텍스처 quad+UV 샘플 | 글자당 quad 1개(효율적) | Renderer trait·GPU 파이프라인·CpuCanvas 확장(큰 변경) |

→ **A 권장**: 가장 contained, 즉시 동작, 추후 B로 최적화 가능. quad 수는 UI 텍스트라 GPU 부담 미미(인스턴스 버퍼 자동확장).

## 결정 포인트 ② — 폰트/미관
| 옵션 | 폰트 예 | 장점 | 단점 |
|---|---|---|---|
| **A. 부드러운 가변폭 JP (AA)** ⭐ | M PLUS / Noto Sans JP (OFL·무료) | 모든 scale에서 깔끔, 한자 가독, ★ 포함 | 현 픽셀 ASCII 룩 → 부드러운 룩으로 교체, 폰트 수 MB 임베드 |
| B. 픽셀 JP 폰트(레트로 유지) | PixelMplus12 / Misaki(美咲) | 현 5×7 레트로 일관, 소형(~1–2MB/8px급은 더 작음) | 픽셀폰트는 정수배 크기에서 최적 → 분수 scale 스냅 필요 |
| C. 하이브리드 | ASCII=현 5×7 + 비ASCII만 TTF 폴백 | 기존 화면 변화 최소·layout 영향 최소 | 블록 ASCII+부드러운 한자 혼합 룩 |

## 권장안(요약)
**①-A(알파 사각형) + ②-A(부드러운 JP, AA)**. 근거: 78 호출처/Renderer trait 불변, 모든 UI 크기에서 견고, 한자/가나/★ 전부 해결. 비용: 픽셀 ASCII가 부드러운 AA로 바뀜, 폰트 파일(수 MB) 레포 임베드.

## 구현 스케치(①-A 채택 시)
- dep: `fontdue`(순수 Rust 래스터, 셰이핑 불필요) 또는 `ab_glyph`.
- 폰트: `assets/fonts/<font>.ttf` → `include_bytes!`. 라이선스 OFL/무료 확인. 필요시 오프라인 서브셋으로 용량↓.
- `font.rs`: `OnceCell<Font>` + thread_local `GlyphCache`(키=(char, px), 값=(metrics, 커버리지 Vec<u8>)). `scale`→`px = round(scale*7)`로 현 크기감 유지.
- `draw_char`: 캐시 비트맵의 픽셀당 `fill_rect`(color.a*=커버리지). `draw_text`는 글자별 `advance_width` 누적(가변폭: ASCII 반각/한자 전각). `text_width`도 advance 기반으로 교체(정렬 정확).
- ASCII는 동일 폰트로 통일(또는 ②-C면 5×7 유지+폴백). CpuCanvas/테스트: layout(text_width) 검증 위주, 픽셀 검증은 스냅샷 옵션.

## 갱신된 요구사항 (사용자, 2026-05-31)
1. **CJK만이 아니라 모든 언어** 지원(풀 유니코드: 라틴·키릴·그리스·한중일·한글·태국·아랍 등).
2. 현재 5×7 비트맵이 **딱딱함** → 폰트 교체로 더 나은 룩.
3. **라이선스 상업적 사용 가능**(프로젝트가 MIT → MIT/OFL 폰트·MIT/Apache 크레이트).
4. **폰트 교체 가능** + 향후 **웹폰트**(URL) 지원.
5. 레퍼런스 구현 참고.

## 레퍼런스 구현 폰트 시스템 (정밀 파악)
- `Config.systemfontpath`/`messagefontpath`(기본 `font/VL-Gothic-Regular.ttf`, **4MB JP폰트**) — **설정으로 교체 가능**.
- libGDX **FreeType 런타임 래스터**(`FreeTypeFontGenerator.generateFont`), **표시 문자열 단위 지연 글리프 생성**(`parameter.characters=text`, 텍스트 바뀌면 atlas 재생성).
- `GlyphLayout`로 정렬/줄바꿈/overflow(shrink·truncate)·그림자.
- **자동 다중폰트 폴백 없음** — 단일 폰트 커버 언어만. 다른 언어는 폰트 교체로 해결. (LR2 비트맵폰트 `LR2FontLoader`는 레거시 별도.)
- 시사점: "설정형 단일 폰트 + 런타임 래스터 + 지연 글리프 + 레이아웃 메트릭"이 검증된 패턴. 단 **우리는 폴백/시스템폰트로 '모든 언어'를 자동화해 한 단계 개선**한다.

## 최종 결정 (요구사항 충족 매핑)
**텍스트 엔진 = `cosmic-text`**(pure Rust, MIT/Apache). 단일 스택이 요구사항 5개를 모두 충족:
| 요구 | cosmic-text 충족 방식 |
|---|---|
| 모든 언어 | `Shaping::Advanced`(rustybuzz 셰이핑·bidi) + `fontdb` **폰트 폴백/시스템폰트** → 라틴·CJK·한글·태국·아랍·이모지 |
| 더 나은 룩 | swash AA 래스터·힌팅·커닝 → 딱딱함 해소 |
| 라이선스 | cosmic-text/fontdb/rustybuzz/swash/ttf-parser 전부 MIT/Apache |
| 교체 가능 | `FontSystem` + `db_mut().load_font_data(bytes)` / 선호 family 지정 → 설정 `font_path`(=레퍼런스 구현 systemfontpath) |
| 웹폰트(향후) | URL→bytes→`load_font_data(bytes)`(테이블 fetch와 동형). 아키텍처상 자명, 실제 fetch는 후속 |

**통합 = ①-A(알파 사각형)**: `SwashCache::with_pixels(.., |dx,dy,color|…)` 픽셀 콜백을 기존 `Renderer::fill_rect`로 흘림 → **GPU 파이프라인/Renderer trait/CpuCanvas 무변경, 78 호출처 시그니처 유지**. (후속 최적화로 글리프 아틀라스 텍스처화 가능, 호출처 불변.)

**번들 기본폰트 = 라틴 핵심 1개(Inter 등 OFL, 소형) + 시스템폰트 폴백**: 작은 번들로 핵심 룩 통일 + OS 설치 폰트로 CJK/태국/아랍 커버(맥/윈은 CJK 기본 탑재) = 실데스크톱 "모든 언어". *미결 포인트 A* 참조.

## 우선순위(단계)
- **P1 핵심 엔진 교체(기반)**: rbms-render에 cosmic-text 도입, `font.rs` 내부를 알파-사각형 경로로 교체(같은 `draw_text`/`text_width` 시그니처). thread_local `TextEngine`(FontSystem+SwashCache+(text,px)→글리프 캐시), `scale→px` 매핑으로 현 크기감 유지, 시스템 폰트 폴백 on. 번들 Inter. **검증: CpuCanvas로 다국어 문자열 PNG 렌더해 육안 확인(권한 불필요!).**
- **P2 폰트 교체(설정)**: `PlaySettings.font_path`(+`--font`), 사용자 TTF 로드·선호 family. 설정 UI 항목. 레퍼런스 구현 systemfontpath 대응.
- **P3 폴리시**: 픽셀당 quad→행 run-length 병합으로 quad 절감, (text,px) 레이아웃 캐시(프레임 비용↓), overflow 말줄임(패널 맞춤, cosmic 폭제한 레이아웃), 그림자/아웃라인 옵션.
- **P4(향후) 웹폰트**: URL fetch→캐시→`load_font_data`. 후속.

## 확정 결정 (사용자, 2026-05-31)
- 엔진 **cosmic-text** + 통합 **①-A(알파 사각형, fill_rect)** 확정.
- 번들 정책 **A1(소형 라틴 Inter 번들 + 시스템 폴백)** 선택. 맥/윈은 시스템 CJK로 모든 언어 커버.

## 구현 완료 (P1 + P2, 검증됨)
- **P1 핵심 엔진**: `crates/rbms-render/src/font.rs`를 cosmic-text 0.19로 전면 교체. `draw_text`/`draw_text_centered`/`draw_text_right`/`text_width` **시그니처 유지**(78 호출처 무수정). thread_local `TextEngine`(FontSystem+SwashCache+`(text,px)→Laid` 캐시), `scale→px=round(scale*8.5)` 매핑, `Attrs::new().family(Family::Name(bundled))`+`Shaping::Advanced`+시스템 폴백, `SwashCache::with_pixels` 픽셀 콜백→`fill_rect`(알파=커버리지, 컬러 글리프=이모지 rgba 그대로). 번들 `assets/fonts/Inter-Regular.ttf`(OFL, 876KB)+`Inter-OFL.txt`.
  - **검증**: `crates/rbms-render/examples/text_render.rs` → CpuCanvas PNG. 일/한/중(간·번체)/키릴·그리스·베트남/태국·아랍(RTL 셰이핑)·데바나가리/★·화살표 전부 부드러운 AA 렌더 확인(헤드리스, GUI 권한 불필요). `cargo test -p rbms-render`(다국어 width>0·load_font Some·픽셀 래스터) 통과.
- **P2 폰트 교체**: rbms-render `load_font(Vec<u8>)->Option<family>`·`set_ui_family(name)`·`reset_ui_family()` 추가(export). player `PlaySettings.font_path`+`--font PATH` CLI+시작 로드, 설정 DISPLAY 탭 `FONT`(DEFAULT/CUSTOM, Enter/우/클릭=rfd 파일선택 라이브 적용, 좌=기본복귀). **검증**: `--font "Arial Black.ttf"` 로드·family 설정·무패닉 스모크.

## 남은 후속 (P3 / P4)
- **P3 폴리시**: 픽셀당 quad → 행 run-length 병합으로 quad 절감, `(text,px)` 캐시키 매 호출 `to_string` 할당 제거(보로우 키), 패널 폭 말줄임(cosmic 폭제한), 그림자/아웃라인, `px_for` 계수·세로 정렬 미세조정(인앱 레이아웃 보고 튜닝).
- **P4 웹폰트**: URL→bytes→`load_font`(+캐시, 난이도표 fetch와 동형).
