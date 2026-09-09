# rbms 스킨/렌더 데이터주도 vs 하드코딩 전수 인벤토리

조사 일자: 2026-09-09. 대상: `/Users/gkn/R-BMS` (읽기 전용), 레퍼런스 `/Users/gkn/beatoraja`.
조사 범위: `crates/rbms-render/src/{lib,skin,theme,playfield,hud,result,select,cpu,font}.rs` 전량 통독,
`apps/rbms-player/src/{gpu.rs, app_play.rs(렌더 블록 440~782), main.rs(상수·스킨·테마·BGA 로딩), app_select.rs(함수 목록)}`,
`assets/skins/{default,normal,wide}.ron` 전량. 시간 상한으로 미조사 범위는 §6.

---

## 1. 요약 결론

- 데이터화되어 있는 것은 **플레이 화면의 노트필드 + HUD 일부(SkinConfig 40필드)** 와 **UI chrome 색상(ThemeConfig 21색)** 뿐이다.
- **곡선택 / 결과 / 설정 / 키설정 / 폴더 / 난이도표 / 로딩 / 모달 / 디버그 오버레이의 좌표·크기·폰트스케일·문자열은 전부 Rust 코드 상수**다. RON으로 옮길 경로가 없다.
- `Renderer` trait 프리미티브는 `size / clear / fill_rect` **3개뿐**(`crates/rbms-render/src/lib.rs:61-65`). 텍스처 그리기 프리미티브가 없어서 이미지는 GPU 백엔드의 **BGA 슬롯 1개**(256x256 고정)로만 그려진다(`apps/rbms-player/src/gpu.rs:32,178-187,247-259`).
- 결과적으로 "사용자가 스킨 파일만으로 임의 이미지·위치·애니메이션을 정의"하는 beatoraja/LR2급 커스터마이징은 **현 아키텍처에서 불가능**하며, 프리미티브·리소스·바인딩·타이머 4개 층을 모두 신설해야 한다(§5).

---

## 2. 화면별 데이터주도 vs 하드코딩 전수표

### 2.1 플레이 (playfield + hud) — 부분 데이터주도

| 항목 | 상태 | 근거 |
|---|---|---|
| 레인 x/폭, 필드 x/폭, 스크래치 좌우, DP 2필드 분할·간격 | 데이터(RON) | `skin.rs:12-38,168-219` (`field_x`,`field_width`,`scratch_left`,`dual_field`,`dual_gap`) |
| top_y / judge_y / note_height / lift | 데이터 | `skin.rs:15-19,221` |
| 노트색(키/대체키/스크/지뢰), 레인배경·알파, 판정선색, 아웃라인·디바이더 색·알파 | 데이터 | `skin.rs:20-33,234-244` |
| 키빔 색/알파/높이비율 | 데이터 | `skin.rs:27-29,224` |
| 판정 팔레트 6색, 판정 라벨 6종·약어 6종 | 데이터 | `skin.rs:43-47`, 사용 `hud.rs:99,121` |
| 게이지 높이·클리어/경고 임계·3색 | 데이터 | `skin.rs:49-54`, 사용 `hud.rs:68-81` |
| 콤보/판정텍스트/FASTSLOW y오프셋 | 데이터 | `skin.rs:56-59`, 사용 `hud.rs:94,99,102` |
| 키봄 on/off·크기·지속(ms) | 데이터 | `skin.rs:62-64`, 사용 `playfield.rs:621-646` |
| BGA 사각형 (x,y,w,h) | 데이터(Option) | `skin.rs:35,223`; 그리기 `app_play.rs:536-539` |
| **키빔 릴리스 페이드 120ms, 그라디언트 슬라이스 6** | 하드코딩 | `playfield.rs:9,12` (`BEAM_RELEASE_US`, `BEAM_SLICES`) |
| **판정선 두께 4px** | 하드코딩 | `playfield.rs:32` |
| **LN 바디 알파 90** | 하드코딩 | `playfield.rs:68` |
| **아웃라인 두께 2px, 디바이더 1px** | 하드코딩 | `playfield.rs:150,154-158` |
| **키봄 최대폭 `w*1.7`, 성장곡선 0.45+0.75p, 코어 0.55·알파 200/235** | 하드코딩 | `playfield.rs:637-645` |
| **게이지 y = judge_y+10, 배경색 (28,28,36), 임계 틱 2px** | 하드코딩 | `hud.rs:68-80` |
| **EX/BEST/GREEN 텍스트 좌표 (14,14)/(14,48)/(14,72)·스케일 2.4/1.4/1.4** | 하드코딩 | `hud.rs:85-91` |
| **라이브 스코어그래프 전체**(x=field_right+28, 폭 clamp 0..170, 최소 80, 바폭 26·간격 12, A/AA/AAA 밴드선, CYAN 상수) | 하드코딩 | `hud.rs:5,29-53,107-112` |
| **판정 카운트 리스트 좌표(bga.x 또는 graph 우측+20), 행높이 16, 값 x오프셋 26** | 하드코딩 | `hud.rs:116-128` |
| 판정 카운트/FAST/SLOW 라벨 문자열 "FAST"/"SLOW" | 하드코딩 | `hud.rs:101,126-128` |
| 리플레이 분석 오버레이(높이 74, 바 40/CW-80, 최근 14개, 색) | 하드코딩 | `app_play.rs:575-595` |

### 2.2 곡선택 (`crates/rbms-render/src/select.rs`) — 전부 하드코딩

레이아웃 상수는 파일 상단 8개 const + 함수 내 매직넘버 수십 개.

| 항목 | 값/좌표 | 근거 |
|---|---|---|
| 리스트 x/폭, 상세 x/폭, TOP/BOTTOM, 행높이/간격 | 32 / 584 / 632 / 616 / 60 / 660 / 36 / 4 | `select.rs:143-150` |
| 커버 사각형 | `(DETAIL_X+16, 78, 160, 160)` | `select.rs:154-156` |
| 상단바 높이 52, 타이틀 "MUSIC SELECT" 스케일 2.6 | 하드코딩 | `select.rs:217-218` |
| 검색박스 x=290 w=240 y=10 h=32 | 하드코딩 | `select.rs:221-225` |
| 헤더 텍스트 x=300, 스케일 1.3 | 하드코딩 | `select.rs:227` |
| 하단 내비 6버튼(라벨/단축키 문자열 배열, 높이 30, 간격 8) | 하드코딩 | `select.rs:236-256`, 버튼 렌더 `189-200` |
| 행 램프바 좌 6px / 우 8px, 포커스 레일 2px | 하드코딩 | `select.rs:305-310` |
| 모드/레벨 배지 크기(46x24, 42x24), 타이틀 x=+116, 포커스 스케일 1.7/1.5 | 하드코딩 | `select.rs:318-322` |
| 상세 타이틀 블록 x=DETAIL_X+192, y=86, 행간 30/24 | 하드코딩 | `select.rs:354-373` |
| 배지 y=196, 난이도명 x=+138 y=201 | 하드코딩 | `select.rs:374-376` |
| 스탯 그리드 3열x2행, 구분선 y=254, 시작 y=268, 행간 34 | 하드코딩 | `select.rs:379-388` |
| 밀도 히스토그램 박스 y=380 h=74, 구분선 348, 라벨 362, 10칸마다 눈금, 바 색 보간식 | 하드코딩 | `select.rs:391-432` |
| RECORDS 섹션 y=468/492/508/530/532/558, 행높이 32 | 하드코딩 | `select.rs:441-461` |
| 모달 크기 720x470 중앙, 오버레이 알파 180, 판정명/색 배열, 행간 30, 버튼 220x36 / 100x36 | 하드코딩 | `select.rs:495-537` |
| 모달 안내문 "UP DOWN PREV/NEXT ENTER REPLAY ESC CLOSE" | 하드코딩 | `select.rs:533` |
| 색상 일부만 theme 경유(`th.*`), 나머지는 리터럴 (예: `Color::rgb(16,18,28)`, `(90,96,120)`, `(10,11,17)`, `(26,28,38)`, `(220,220,150)`) | 혼재 | `select.rs:342,346,350,396,415,313` |

### 2.3 결과 (`crates/rbms-render/src/result.rs`) — 전부 하드코딩

| 항목 | 값 | 근거 |
|---|---|---|
| 좌측 DJ LEVEL 중심 x=232, 랭크 y=96 스케일 7.0, rate y=214 | 하드코딩 | `result.rs:96-100` |
| 클리어램프 박스 (44,258,376,48) | 하드코딩 | `result.rs:101-102` |
| 랭크바 (44,340,376,18), 델타 y=384 행간 28 | 하드코딩 | `result.rs:104-120` |
| 우측 리포트 x=480, 우끝 1236, 시작 y=80, 행간 42/30/34 | 하드코딩 | `result.rs:124-146` |
| 판정색 `JUDGE_COLORS`·판정명 `JUDGE_NAMES` — **스킨 `judge_colors`/`judge_labels`를 쓰지 않음**(HUD와 팔레트 이원화) | 하드코딩 | `result.rs:25-26,140-141` vs `skin.rs:43-47` |
| PGREAT 핫핑크 (255,40,150) 별도 | 하드코딩 | `result.rs:82,140` |
| DJ 랭크 밴드 8종 색·경계(ninths) | 하드코딩 | `result.rs:30-43` |
| 하단 안내문 "ENTER / ESC SELECT" y=692 | 하드코딩 | `result.rs:152` |

### 2.4 설정 탭 / 키설정 / 폴더 / 난이도표 (`apps/rbms-player/src/app_play.rs`) — 전부 하드코딩

| 화면 | 하드코딩 항목 | 근거 |
|---|---|---|
| SETTINGS | 패널폭 720, 제목 y=36 스케일 3.0, 안내문 y=78, 탭바 y=104 h=34 간격 8, 행 y=160+50i h=42, 값 스케일 2.0, 선택색 `Color::YELLOW`/`(150,150,165)`/`(120,230,255)` | `app_play.rs:603-625` |
| KEY CONFIG | 패널폭 760, 제목 y=40, 힌트 y=82, 행높이 28+2, 가시행 17 고정, 중복키 RED | `app_play.rs:631-671` |
| TABLES | 패널폭 900, 행 y=124+44i h=36, 입력박스 (x0,116,PANEL_W,40), URL 표시 70자 절단 | `app_play.rs:678-707` |
| FOLDERS | 패널폭 980, 행 y=124+44i h=36, 경로 78자 절단 | `app_play.rs:713-734` |
| LOADING | 중앙 (CW/2, CH/2), 제목 y=-36 스케일 3.0, 서브 y=+14, 진행바 360x8 y=+56, 점 애니메이션 `frame_count/12%4`, 스캔 스윕 세그 96·주기 120프레임 | `app_play.rs:475-503` |
| 서버 연결 표시등 | (CW-22,10,10,10) 초록/빨강 | `app_play.rs:743-746` |
| 디버그 오버레이 | 패널 (6,6,320,ph), 행높이 16, 텍스트 x=14 스케일 1.2, 색 YELLOW/(120,240,140), 표시 항목 전부 코드 고정 | `app_play.rs:747-779` |
| 화면 논리 해상도 | `CW=1280 / CH=720` 컴파일 상수, GPU가 `Renderer::size()`로 항상 이 값을 반환 → 창 리사이즈 시 리플로우 없음(스트레치) | `main.rs:52-53`, `gpu.rs:339-341`, 리사이즈 `main.rs:937`, 마우스 좌표 역변환 `main.rs:945` |

### 2.5 테마(chrome 색) — 데이터주도지만 커버리지 부분적

- `ThemeConfig` 21색 전부 optional, RON 부분 지정 가능, 파싱 실패 시 기본값 (`theme.rs:101-161`).
- 앱 첫 실행 시 `~/.config/rbms/theme.ron` 템플릿 자동 생성 (`main.rs:233-280`).
- 저장은 `thread_local! RefCell<Theme>` 전역 (`theme.rs:164-176`) — 렌더 스레드 한정, 런타임 교체는 `set_theme` 뿐.
- **커버리지 구멍**: select/result/hud의 상당수 색이 `th.*`가 아닌 리터럴이라 테마로 바꿀 수 없다(§2.2·2.3 마지막 행, `hud.rs:5,35,70`).

---

## 3. SkinConfig 스키마 및 3종 RON 실제 차이

### 3.1 필드 스키마 (총 40필드, 전부 `#[serde(default)]`)

기하 7: `field_x, field_width, top_y, judge_y, note_height, scratch_left, lift`
색 15: `key_color, key_color_alt, scratch_color, mine_color, lane_bg, lane_bg_alpha, judge_line, beam_color, beam_alpha, beam_height_frac, outline_color, outline_alpha, divider_color, divider_alpha, bg`
영역/DP 3: `bga(Option<[f32;4]>), dual_field, dual_gap`
HUD 15: `judge_colors[6][3], judge_labels[6], judge_labels_short[6], gauge_height, gauge_clear_threshold, gauge_warn_threshold, gauge_color_clear, gauge_color_warn, gauge_color_fail, combo_y_offset, judge_text_y_offset, fastslow_y_offset, bomb_enabled, bomb_height, bomb_duration_ms`
(`skin.rs:12-65`, 기본값 `67-112`, 로드 `114-119`)

**스키마에 없는 것**: 이미지 경로, 폰트, 애니메이션 곡선/타이머, 조건부 표시, 곡선택/결과/메뉴 좌표, 숫자·그래프 오브젝트, 레이어/z-order.

### 3.2 3종 RON 차이 (실측)

| 필드 | default.ron | normal.ron | wide.ron |
|---|---|---|---|
| field_x | 0.05 | 0.281 | 0.18 |
| field_width | 0.28 | 0.281 | 0.36 |
| top_y / judge_y | 28 / 672 | 60 / 620 | 60 / 620 |
| note_height | 14 | 14 | 16 |
| beam_alpha | 150 | 150 | 160 |
| bga | (470,104,512,512) | (905,360,350,255) | (920,360,340,255) |
| outline/divider 키 | **없음**(기본값 상속) | 명시 | 명시 |
| 나머지 34필드 | 전부 동일 | 동일 | 동일 |

즉 3종 스킨의 차이는 **스칼라 6~7개**뿐이고, 구조적 차이는 0이다. HUD 필드 15개는 세 파일 어디에도 기재되지 않아 항상 기본값이다(`default.ron:1-24`, `normal.ron`, `wide.ron` 전문 확인).
번들 선택은 이름 2개(`WIDE` 아니면 NORMAL) 하드코딩 (`main.rs:226-229`), 임의 파일은 `--skin` 경로로 `SkinConfig::load` (`skin.rs:115-118`).

### 3.3 이미지/텍스처/폰트 리소스 관리

| 리소스 | 로드 | 캐시 | 해제 | 근거 |
|---|---|---|---|---|
| BGA 이미지 | 곡 로드시 전부 디코드 후 **256x256 RGBA로 리사이즈**해 `HashMap<i32, Vec<u8>>` | 전곡 메모리 상주(장당 256KB) | 다음 곡 로드시 `clear()` 만, 재생 중 eviction 없음 | `main.rs:351-356,671`, `app_play.rs:108,115,123` |
| BGA GPU 텍스처 | `Gpu::set_bga`가 매 전환마다 `write_texture` (동일 텍스처 1장 덮어쓰기) | 텍스처 1장 고정(`BGA_DIM=256`) | 없음(`clear_bga`는 플래그만) | `gpu.rs:32,178-187,247-263,312-316` |
| 커버(#STAGEFILE/#BANNER) | 포커스 곡 1회 디코드 → `cover_rgba: Option<Vec<u8>>` | 1장 | 포커스 이동 시 교체 | `main.rs:688`, `app_play.rs:510-513` |
| **BGA와 커버가 같은 슬롯 공유** | 곡선택은 커버, 플레이는 BGA를 같은 1개 텍스처에 업로드 | — | — | `app_play.rs:511 vs 537` |
| 폰트 | Inter 임베드 + cosmic-text 시스템 폴백 | `(px→text→Laid)` 레이아웃 캐시 + `(glyph,color)→RunLength` 캐시, 무제한 증가 | 패밀리 변경 시만 clear | `font.rs:10,45-55,70-89,101-120` |
| 글리프 → 화면 | swash로 래스터한 픽셀을 **가로 run 병합 후 `fill_rect` 쿼드로 방출** (텍스처 아틀라스 없음) | run 캐시로 재래스터는 회피 | — | `font.rs:25-38,111-120` |

### 3.4 Renderer trait 프리미티브

```
size(&self) -> (u32,u32)
clear(&mut self, Color)
fill_rect(&mut self, Rect, Color)
```
(`lib.rs:59-65`) — 이게 전부다. 구현체는 `CpuCanvas`(`cpu.rs:25-61`, 소스오버 알파블렌드만)와 `Gpu`(`gpu.rs:338-353`, `fill_rect` 1개 = 인스턴스 1개, 단일 인스턴스드 드로우콜).
없는 것: 텍스처 드로우, UV/서브영역, 회전, 스케일 매트릭스, 블렌드 모드 선택, 클립/시저, 9-patch, 라인/원, z-order/레이어.

---

## 4. beatoraja 레퍼런스와의 격차 (근거)

| beatoraja | rbms |
|---|---|
| `SkinObject.setDestination(time, x,y,w,h, acc, a,r,g,b, blend, filter, angle, center, loop, timer, op1,op2,op3, offset)` — **키프레임 애니메이션 + 가감속 + 블렌드 + 필터 + 회전 + 앵커 + 루프 + 타이머 + 조건(op) 3개**가 오브젝트 목적지 1개의 스펙 (`/Users/gkn/beatoraja/src/bms/player/beatoraja/skin/SkinObject.java:124-168`) | `Rect + Color`만 (`lib.rs:45-57,64`) |
| 타이머 체계: `TIMER_STARTINPUT/FADEOUT/FAILED/SONGBAR_MOVE/PANEL1_ON…` 등 (`SkinProperty.java:11-27`), `TimerProperty`/`TimerPropertyFactory` | 타이머 개념 없음. 애니메이션은 각 렌더 함수가 `microtime`/`frame_count`로 직접 계산 (`playfield.rs:179-193`, `app_play.rs:477,498`) |
| 프로퍼티 바인딩: `SkinProperty` 상수 968개(`grep "public static final int" SkinProperty.java` = 968줄), `BooleanPropertyFactory`(758줄)/`IntegerPropertyFactory`(1509줄)/`FloatPropertyFactory`(596줄)/`StringPropertyFactory`(421줄)/`EventFactory`(925줄) | 게임상태→스킨 프로퍼티 바인딩 레이어 없음. 각 화면이 전용 View 구조체(`HudView`,`ResultView`,`SelectView`)로 필드를 손으로 나열 (`hud.rs:8-24`, `result.rs:5-23`, `select.rs:126-141`) |
| 오브젝트 타입: `SkinImage/SkinNumber/SkinText/SkinSlider/SkinGraph/SkinBPMGraph/SkinNoteDistributionGraph/SkinTimingVisualizer/SkinHitErrorVisualizer/SkinTextInput/…` | 오브젝트 타입 개념 없음. 전부 Rust 함수 호출 |
| 로더: JSON 스킨(`json/JSONSkinLoader.java` 483줄, `JsonSkinObjectLoader.java` 759줄) + LR2 CSV(`lr2/LR2SkinCSVLoader.java` 등 10개 로더) | RON 1개 스키마(`SkinConfig`), 로더 3줄(`skin.rs:115-118`) |
| 리소스: `SkinSourceImage/SkinSourceImageSet/SkinSourceMovie/SkinSourceReference/SkinSourceSet`, 기본 스킨에 `bomb.png/gauge.png/judge.png/number.png/lamp.png/lanecover.png/panel.png` 등 이미지 자산 | 이미지 자산 0개(폰트 1개만). 모든 시각요소가 단색 사각형 |

---

## 5. "완전 커스터마이징" 저해 지점과 리팩토링 규모

| # | 저해 지점 | 상세 | 규모 |
|---|---|---|---|
| A | Renderer 프리미티브가 `fill_rect` 뿐 | 텍스처 그리기·UV 서브영역·회전·스케일·블렌드모드·클립이 전무. 스킨 이미지·스프라이트시트 숫자·9-patch 모두 불가 | L (trait 확장 + wgpu 파이프라인 2~3개 + CpuCanvas 대응 + 기존 900여 테스트 영향 검토) |
| B | 텍스처 자원 관리 부재 | 텍스처 1장(256x256) 고정, 아틀라스/핸들/수명관리 없음. BGA와 커버가 같은 슬롯을 공유 | L (TextureId 핸들 + 아틀라스/배열텍스처 + 로드·eviction 정책) |
| C | 타이머/이벤트 개념 부재 | beatoraja의 `TIMER_*` + `loop`/`acc` 키프레임에 대응하는 것이 없음. 애니메이션이 각 함수에 인라인 상수로 박혀 있음(`BEAM_RELEASE_US`, `bomb` 곡선, 로딩 점) | M (타이머 레지스트리 + 키프레임 보간기) |
| D | 게임상태→스킨 프로퍼티 바인딩 부재 | 조건부 표시(op)·숫자/문자열 소스 지정이 불가. 현재는 View 구조체 필드를 코드가 직접 읽어 그림 | L (프로퍼티 ID 열거 + 상태 수집기 + Boolean/Int/Float/String 4종 팩토리) |
| E | 화면별 렌더 함수 시그니처가 고정 레이아웃 전제 | `render_select(r,&SelectView)`/`render_result(r,&ResultView)`/`render_hud(r,&Skin,&HudView)`가 "레이아웃 코드 + 데이터"를 한 함수에 묶음. 오브젝트 리스트 순회 구조가 아님 | L (화면당 오브젝트 그래프 인터프리터로 재작성) |
| F | 스킨 스키마가 플레이필드 전용 | select/result/menu 좌표를 표현할 필드가 SkinConfig에 아예 없음. 3종 RON도 스칼라 6개만 다름 | M (스키마 확장) — 단 A~E 없이 좌표만 데이터화하면 반쪽짜리 |
| G | 논리 해상도 1280x720 컴파일 고정 | `Renderer::size()`가 항상 CW/CH 반환, 리사이즈는 스트레치. 스킨이 해상도/종횡비를 선언할 수 없음 | M |
| H | 폰트가 `fill_rect` 방출 방식 | 텍스트 1글자가 여러 쿼드. 비트맵 폰트/글리프 아틀라스(beatoraja `SkinTextBitmap`) 대응 불가 | M (A 완료 후 아틀라스 전환) |
| I | 테마 전역이 `thread_local` 단일 값 | 화면별/스킨별 팔레트 분리·핫리로드가 구조적으로 어려움 | S |
| J | 팔레트 이원화 | result가 skin의 `judge_colors`/`judge_labels`를 무시하고 자체 상수 사용 → 같은 스킨인데 화면 간 색이 다름 | S |

전체 "LR2/beatoraja급 완전 커스터마이징"은 A+B+C+D+E 동시 필요 → **XL(수주)**. 단계적 접근이면 A(프리미티브) → B(텍스처) → C(타이머) → D(바인딩) → E(오브젝트 인터프리터) 순.

---

## 6. 미조사 범위 (시간 상한)

- `apps/rbms-player/src/app_select.rs` 전문(967줄) — 함수 시그니처 목록만 확인. `build_select_view`(98~265)의 View 조립부에 추가 하드코딩(라벨 문자열·색 매핑)이 있을 가능성 높음. **미확인**.
- `apps/rbms-player/src/format.rs`(434줄), `app_input.rs`, `scores.rs`, `folders.rs` — 미조사.
- `crates/rbms-render/src/font.rs` 120줄 이후(draw_text/fit_text/text_width 세부), `playfield.rs` 200~620(테스트 위주로 추정) — 미조사.
- beatoraja `JsonSkin.java`/`JsonSkinObjectLoader.java` 상세 스키마, LR2 CSV 명령어 집합 — 파일 크기·존재만 확인, 내용 미통독.
- 문서(`docs/PROCESS.md`)는 "데이터 주도 HUD 스킨", "(선택/BLOCKED) 결과·메뉴 레이아웃 RON화", "(선택) 테마 chrome 색 완성"으로 현 상태를 정확히 기술하고 있어 **stale 아님**(PROCESS.md Phase 7 항목과 §2 실측 일치).
