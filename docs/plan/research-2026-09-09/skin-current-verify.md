# skin-render-inventory 반박 검증 (skeptic pass)

검증자: 회의적 검증자 에이전트. 대상 보고서: `research/skin-render-inventory.md` (findings 01~14).
검증 방식: 각 finding 이 인용한 rbms/레퍼런스 구현 파일을 직접 열어 (a) 인용 line 이 주장을 뒷받침하는지, (b) 수치가 맞는지, (c) 이미 구현된 것을 미구현이라 했는지, (d) severity/effort 과장 여부를 확인.
읽기 전용 준수(수정·생성 없음, cargo 미실행).

## 요약

| id | 판정 | 핵심 |
|----|------|------|
| 01 | partially | trait 3메서드 주장은 사실. 단 "텍스처 전무"는 과장 — gpu.rs 에 UV 샘플링 텍스처 파이프라인(BGA)이 이미 존재해 effort 는 L 보다 낮을 여지 |
| 02 | confirmed | BGA_DIM 256 단일 텍스처 1장, 커버/BGA 공유, 크기 불일치 silent return 전부 확인 |
| 03 | confirmed | 968개 상수 수치 정확(`grep -c` = 968). 단 "968줄"은 오기(파일 1068줄) |
| 04 | confirmed | 인용 상수·좌표 전부 실재. SkinConfig 에 select 필드 0개 확인 |
| 05 | confirmed | BEAM_RELEASE_US / BEAM_SLICES / 봄 성장곡선 / 로딩 점·스윕 전부 코드 상수 |
| 06 | confirmed | 4개 render_* 시그니처와 고정 호출 순서, stage match 확인 |
| 07 | confirmed | result.rs 자체 const, ResultView 에 Skin 없음, hud 는 skin.judge_colors 사용 — 팔레트 이원화 실재 |
| 08 | partially | 데드 스키마 주장은 사실이나 필드 수가 틀림(19/23, 20/24 아님). 데드 필드는 15개가 아니라 17개(dual_field/dual_gap 포함). default.ron 은 설정에서 선택 불가(임베드는 NORMAL/WIDE 2종) |
| 09 | confirmed | CW/CH 상수, size() 고정 반환, uniform 기록, resize 가 surface 만 재구성, 마우스 역스케일 전부 확인 |
| 10 | confirmed | GlyphRun run-length → fill_rect 재생, 캐시 무제한 증가 확인 |
| 11 | partially | 레이아웃 하드코딩·앱 크레이트 종속은 사실. "테마 어느 쪽으로도 제어 불가"는 과장 — 패널/버튼/텍스트는 theme() 사용, 강조색만 리터럴 |
| 12 | confirmed | 인용한 리터럴 전부 실재 |
| 13 | confirmed | 전량 디코드 후 HashMap 상주, 해제는 clear 한 곳. (단 `config.bga && skin.bga.is_some()` 일 때만 디코드 — 조건 누락) |
| 14 | confirmed | thread_local THEME, 테스트의 수동 복원 주석까지 확인 |

## finding 별 상세

### 01 — partially (핵심 주장 유지, 근거 일부 과장)

- 사실: `crates/rbms-render/src/lib.rs:59-65` 는 `size/clear/fill_rect` 3개뿐. `crates/rbms-render/src/cpu.rs:25-61`, `apps/rbms-player/src/gpu.rs:338-353` 두 구현체 모두 텍스처 진입점 없음. 스킨이 임의 이미지를 배치할 수 없다는 결론은 성립.
- 정정: "텍스처·UV 서브영역이 전무"는 **렌더 트레이트 한정**으로 읽어야 한다. wgpu 백엔드에는 이미 UV 샘플러 기반 텍스처 파이프라인이 존재한다 — `apps/rbms-player/src/gpu.rs:33-40` (BGA_SHADER: `texture_2d`+`sampler`+uv VsOut), `:178-190`(텍스처/뷰/샘플러 생성), `:312-316`(bga_pipeline draw). 권고의 "두 번째 파이프라인을 새로 만든다"는 사실상 **기존 BGA 파이프라인의 인스턴스화 확장**이므로 effort 는 L 보다 M 쪽에 가깝다(CpuCanvas 참조 구현·트레이트 확장·호출부 흡수 포함해도).
- severity critical 유지 타당.

### 02 — confirmed

`gpu.rs:32`(BGA_DIM=256), `:178-187`(텍스처 1장), `:247-259`(set_bga 유일 업로드, `rgba.len() != DIM*DIM*4` 이면 조용히 return — :248-250 실측), `:261-263`(clear_bga 는 플래그만), `:312-316`(1장만 draw) 전부 인용대로. 공유 증거도 정확: `apps/rbms-player/src/app_play.rs:511`(커버 → `gpu.set_bga(rgba, cover_rect())`), `:537`(BGA → `gpu.set_bga(img, rect)`). 원본 강제 리사이즈 `main.rs:351-356` 확인.

### 03 — confirmed (수치 1건 오기)

- `grep -c "public static final int" skin/SkinProperty.java` = **968** → "상수 968개" 정확. 다만 파일은 1068줄이므로 "선언 968줄"은 오기.
- 팩토리 줄 수 실측: Boolean 758 / Integer 1509 / Float 596 / String 421 / Event 925 — 인용치 전부 일치.
- rbms 측: `hud.rs:8-24`(HudView), `result.rs:5-23`(ResultView), `select.rs:126-141`(SelectView) 모두 전용 구조체이고 조건부 표시는 Rust `if`(예: `hud.rs:93,97,100,110`, `result.rs:103,116`). 프로퍼티 바인딩 계층 부재 확인.

### 04 — confirmed

`select.rs:143-150` 상수 6종 값 일치(LIST_X 32 / LIST_W 584 / DETAIL_X 632 / DETAIL_W 616 / TOP 60 / BOTTOM 660 / ROW_H 36 / ROW_GAP 4), `:154-156` cover_rect 하드코딩, `:217-231` 상단바 52·타이틀 2.6·검색박스 290/240, `:236-256` 6버튼 배열, `:495-537` 모달(NAMES/COLS, 버튼 220x36, CLOSE 100x36) 전부 실측 일치. `skin.rs:12-65` 에 select 관련 필드 0개 — 확인.

### 05 — confirmed

`playfield.rs:9`(BEAM_RELEASE_US=120_000), `:12`(BEAM_SLICES=6), `beam_intensity` 선형 페이드, `:634-645`(0.45+0.75p, 코어 0.55, 알파 200/235) 전부 일치. `app_play.rs:477`(`frame_count/12%4`), `:498-500`(seg 96, `frame_count%120`) 확인. 스킨 제어 가능한 애니메이션 파라미터가 bomb 3개뿐인 것도 `skin.rs:62-64` 로 확인. 레퍼런스 구현 `SkinObject.java:124-168` setDestination 이 acc/loop/timer/op 를 받는 것도 원문 확인.

### 06 — confirmed

`select.rs:213`, `result.rs:86`, `hud.rs:55`, `playfield.rs:18` 시그니처 일치. `select.rs:258-266` 고정 호출 순서(list → detail(match) → modal) 확인. 히트테스트가 렌더 반환값 의존(`app_play.rs:514-529`)도 확인.

### 07 — confirmed

`result.rs:25-26` const JUDGE_COLORS/JUDGE_NAMES, `:82` PGREAT_PINK, `:140-141` 사용부 확인. `ResultView`(result.rs:5-23)에 Skin 참조 없음 확인. 반대로 `hud.rs:99,121` 은 `skin.judge_colors`/`skin.judge_labels(_short)` 사용 → 팔레트 이원화 실재. `select.rs:506-507` 모달 상수도 동일. 호출부가 `app_play.rs` 한 곳인 점도 맞아 effort S 타당.

### 08 — partially (수치·프레이밍 정정)

- 확인: `skin.rs:41-64` HUD 15필드가 세 RON 어디에도 없음. `#[serde(default)]`(skin.rs:11) 때문에 조용히 기본값.
- 정정 1 — 필드 수: default.ron 은 **19키**(보고서 20), normal.ron/wide.ron 은 **23키**(보고서 24). 실측 전문 기준(각각 field_x…bga).
- 정정 2 — 데드 스키마 범위: HUD 15필드뿐 아니라 **`dual_field`/`dual_gap`(skin.rs:36-39)도 세 RON 전부에 없음** → 미기재 필드는 17개. DP(10K/14K) 레이아웃 전환이 RON 예제로 노출되지 않는 것은 별도 문제.
- 정정 3 — "3종 스킨": 실행 시 선택 가능한 스킨은 **NORMAL/WIDE 2종만 임베드**(`main.rs:222-227`, `bundled_skin` 이 WIDE 아니면 전부 NORMAL). `assets/skins/default.ron` 은 `--skin` 경로로만 도달 가능하고 설정 UI(`app_select.rs:905`)의 SKIN 값에도 없다. "세 파일 차이" 서술은 사용자 관점에서 2종 차이로 읽어야 정확.

### 09 — confirmed

`main.rs:52-53`(CW/CH), `gpu.rs:339-341`(size 고정 반환), `gpu.rs:125`(uniform 에 CW/CH), `gpu.rs:334-340`(resize 가 surface.configure 만), `main.rs:945`(마우스 역스케일) 전부 실측 일치. 스킨 스키마에 해상도 필드 없음도 확인. 추가로 `main.rs:859` 가 `Skin::default_for(MODE, CW, CH)` 로 스킨 빌드 자체를 컴파일 상수에 묶고 있어 주장이 오히려 더 강해진다.

### 10 — confirmed

`font.rs:25-38`(GlyphRun 정의+주석), `:101-120`(swash 래스터→runs 캐시→fill_rect 재생), `:45-55`(px→text 중첩 캐시, 무제한), `:10`(Inter 임베드) 확인. 레퍼런스 구현 대조 파일(SkinNumber/SkinTextBitmap/SkinTextImage/BitmapFontCache)도 실재.

### 11 — partially

- 사실: `app_play.rs:603-625/631-671/678-707/713-734/465-503/747-779` 의 PANEL_W·행높이·y좌표·문자열이 전부 앱 크레이트 stage match 안에 하드코딩. rbms-render 로 옮기지 않으면 CpuCanvas 헤드리스 회귀 테스트가 불가능하다는 지적은 타당.
- 정정: "테마·스킨 어느 쪽으로도 제어할 수 없다"는 과장. 이 화면들은 `th.panel`/`th.button`/`th.button_active`/`th.text`/`th.text_dim`/`th.text_muted` 를 실제로 사용한다(`app_play.rs:617,619,659,683,703,728` 등). 테마 미적용은 **강조색 리터럴 한정** — `Color::YELLOW`(:612 탭 활성, :623 값), `Color::rgb(150,150,165)`(:612 비활성 탭), `Color::rgb(120,230,255)`(:623 편집중), `Color::RED/GREEN`(:640-644, :706, :731). 또한 `draw_text`/`text_width` 는 rbms-render 함수이므로 "rbms-render 크레이트조차 거치지 않고"도 부정확(레이아웃만 앱 소유).
- 결과적으로 severity medium 은 유지하되 근거를 "테마 커버리지 구멍 + 헤드리스 테스트 불가"로 좁히는 게 맞다.

### 12 — confirmed

`hud.rs:5`(CYAN), `:35`(74,74,92), `:70`(28,28,36), `:85-91`(WHITE/GREEN/CYAN) 확인. select 리터럴(:313, :342/346, :350, :396, :415, :426, :507, :527/535) 확인. `theme.rs` 정의부 존재 확인.

### 13 — confirmed (조건 1건 보강)

`main.rs:351-356`(resize_exact 256 후 raw RGBA), `app_play.rs:108`(clear), `:115`(insert), `:123`(개수 출력), `:536-539`(조회) 일치. 해제 경로 clear 하나뿐 확인.
보강: 디코드는 `self.config.bga && self.skin.bga.is_some()` 일 때만 수행(`app_play.rs:109`) — BGA 를 끄거나 스킨에 bga 사각형이 없으면 상주 자체가 없다. 실사용 조건 하에서만 성립하므로 low 타당.

### 14 — confirmed

`theme.rs:164-176`(thread_local + set_theme/theme), `:204-212`(테스트가 직접 복원, 주석 포함), `main.rs:277`(시작 시 1회 set) 확인.

## 미조사 범위

- `render_list`/`render_song_detail`/`render_records`(select.rs 270~470) 는 인용된 라인만 확인하고 전체 통독은 하지 않음.
- CPU/GPU 픽셀 동등성은 코드 독해만 했고 cargo test 미실행(시간 상한).
- 레퍼런스 구현 측은 SkinObject/SkinProperty/SkinType/로더 디렉터리 구조까지만 확인, JSON 스키마 세부는 미확인.
