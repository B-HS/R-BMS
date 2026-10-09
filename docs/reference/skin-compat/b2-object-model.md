# B2. beatoraja 스킨 객체 모델과 공통 그리기 의미론

> 최종 갱신 2026-10-10 · 대응 단계: 웨이브 3A(공통 그리기 의미론) 반영 · 본문은 작성 시점 서술이며, 아래 "웨이브 … 반영 사항"이 최신 것부터 우선한다 · 색인과 갱신 규칙은 [README.md](README.md)

## 웨이브 3A 반영 사항 (2026-10-10)

prepare/draw 2단계 파이프라인과 `SkinHost` 직접 그리기, 그리기 조건 의미론, 참조 이미지·음수 크기·이미지 인덱스·숫자·슬라이더·그래프 정합, TTF 텍스트, judgegraph·bpmgraph, Lua 함수 값 프레임 평가, 앱 호스트 군집 A·I·M 을 넣은 뒤의 상태다.

- (W3-1a) b2-object-model.md §3.2: 'dst 가 빈 배열이면 키프레임 0개 -> validate() false -> 제거' 뒤에 예외 추가 — SkinBar(select/SkinBar.java:105), SkinNote(play/SkinNote.java:32), SkinJudge(play/SkinJudge.java:55)는 생성자에서 setDestination(0, 0,0,0,0, 0, 0,255,255,255, 0,0,0,0,0,0, []) 로 키프레임 하나를 스스로 넣으므로 dst 없는 destination 으로도 유효. 이때 destination 의 timer/loop/blend/filter/center/op/draw/mouseRect 는 키프레임 루프 안에서만 적용되므로 전혀 적용되지 않고 offset·stretch 만 적용(JSONSkinLoader.java:422-478)
- (W3-1a) b2-object-model.md §7.3: R-BMS 구현 메모 추가 — prepare/draw 2단계는 SkinScreen::prepare -> PreparedFrame, SkinScreen::draw_prepared. Lua 는 prepare 에서만 호출되고 draw 는 기록된 답을 읽음
- (W3-1b) b2-object-model.md §3.4: '조건 등록은 첫 키프레임을 읽을 때 일어나므로 dst 가 빈 destination 은 조건도 dstop 도 등록하지 않는다(SkinObject.java:174-182 가 키프레임 루프 안에서만 호출됨)' 와 dstdraw 순서(내장 정수 op → op 의 함수·이름 → draw)의 R-BMS 대응(track.rs conditions)을 추가
- (W3-1b) b2-object-model.md §5.1: 2~4번 규칙이 R-BMS 에 구현됐음을 적고, 5.1 의 대상이 skin.objects(최상위)뿐이라 중첩 객체의 dstop 은 무시된다는 점을 명시. §18 의 3번(미지 op 는 부호와 무관하게 제거)을 구현 완료로
- (W3-2) b2-object-model.md 상단 반영 사항: §9.4 의 검정/흰색은 R-BMS 에서 2x1 한 장이 아니라 1x1 두 장(필터 시 이웃 텍셀 섞임 없음)이라는 의도적 차이, §6.6 의 +0.01 보정 미이식을 적을 것
- (W3-5) b2-object-model.md: JsonSkinObjectLoader 의 judgegraph 는 break 라 같은 id 의 뒤 종류(bpmgraph, hiterror, timingvisualizer, timingdistribution 순)가 이긴다. build_graph 순서를 이에 맞춤
- (W3-3) b2-object-model.md §12.2: [libGDX] 표시 서술 중 R-BMS 가 채택한 해석을 명시 — 줄 폭은 첫 글리프 잉크 왼쪽~마지막 글리프 잉크 오른쪽, overflow 2 는 '펜이 폭 안에 남는 접두 글리프까지'(마지막 글리프만 잉크 기준), 첫 글리프는 절단 판정에서 제외. 정렬값이 0/1/2 가 아니면 왼쪽으로 처리
- (W3-3) b2-object-model.md §12.3: 추가할 사실 — libGDX 커닝은 FreeType 의 `kern` 표만 읽으므로 GPOS 만 있는 mgenplus 는 커닝이 없다(R-BMS 도 kern 표 유무로 켠다). capHeight 는 AutoMedium 힌팅된 대문자 높이이며 R-BMS 는 윤곽 높이 x 소문자 맞춤 계수를 반올림해 근사한다(mgenplus black: size 25/40/70/90/170 → 18/29/51/66/124). `size` 0 은 폰트 생성 실패로 미표시
- (W3-3) b2-object-model.md §6.4 와 §18 #6·#13·#14: 구현됨으로 표시(blend 상속, TTF 는 dst h 가 글자 크기, ref 유효 시 constantText 무시). #6 에 '합성 객체는 자기 destination blend 를 남기는 근사' 단서 추가
- (W3-3) b2-object-model.md §20: 'GlyphLayout/BitmapFont 글리프 배치' 행에 R-BMS 의 남은 오차 수치(세로 ±1px at size, 가로 글리프당 최대 ±1px at size) 추가


## 웨이브 2A 반영 사항 (2026-10-10)

Lua 5.2 런타임(`crates/rbms-skin/src/lua/`), `SkinHost`, Lua 값 변환기, 2패스 `.luaskin` 로더를 넣고 구 샌드박스(`skin.*`)를 삭제한 뒤의 상태다.

- (W2-4) b2-object-model.md §17: '[luaj] 배열부 순서 보장은 지식 기반이며 Lua 로더 담당 조사에서 확정해야 한다'를 'R-BMS 는 양의 정수 키 오름차순 → 나머지 키 순서로 읽는다(from_lua.rs elements)'로 바꿔야 합니다.


## 웨이브 1B 반영 사항 (2026-10-10)

타이머 µs, stretch 11종, 장면 시계, 마우스 이벤트, GPU 논리 크기 런타임화와 색 공간, START/SELECT, 해상도 설정, 캡처 하니스를 넣은 뒤의 상태다.

- (W1-4) b2-object-model.md §4.1: 수정 불필요하지만 R-BMS 대응 주석이 있다면 '경과 ms = 시계와 타이머를 각각 /1000 한 차' 가 R-BMS dst.rs 에 그대로 옮겨졌다고 적을 수 있다


조사 대상 저장소: `/Users/hyunseokbyun/development/beatoraja` (HEAD `8320241d`, 작업 트리 변경 없음, origin `exch-bms2/beatoraja`).

## 0. 표기와 신뢰도

- 경로는 모두 `B = /Users/hyunseokbyun/development/beatoraja/src/bms/player/beatoraja` 기준 상대 경로다. 셰이더는 `/Users/hyunseokbyun/development/beatoraja/src/glsl/` 에 있다.
- 근거 표기는 `파일:줄` 이다.
- **[libGDX]** 표시는 libGDX 1.9.9 내부 동작에 대한 서술이다. 저장소에는 `lib/gdx.jar` (버전 문자열 `1.9.9`, `Version.class` 에서 확인)만 있고 소스가 없어 **조사자 지식 기반**이다. 구현 전에 libGDX 1.9.9 의 `SpriteBatch.java`, `BitmapFont.java`, `BitmapFontCache.java`, `GlyphLayout.java`, `TextureRegion.java`, `FreeTypeFontGenerator.java` 원문과 대조해야 한다.
- **[미확인]** 은 이번 범위에서 확정하지 못한 사항이다.
- 이 저장소의 beatoraja 는 `clip_*`, 편집 가능 텍스트(`SkinTextInput`), 폰트 fallback, 이름 기반 프로퍼티 등 최근 기능을 포함한다. ModernChic 이 이 기능들을 쓰는지는 19장 참고.

### 전부 읽은 파일

`skin/Skin.java`(790), `skin/SkinObject.java`(875), `skin/SkinImage.java`(181), `skin/SkinNumber.java`(233), `skin/SkinFloat.java`(239), `skin/FloatFormatter.java`(130), `skin/SkinText.java`(226), `skin/SkinTextFont.java`(591), `skin/SkinTextBitmap.java`(1005), `skin/SkinTextImage.java`(251), `skin/SkinTextInput.java`(192), `skin/BitmapFontCache.java`(128), `skin/SkinFontSource.java`(98), `skin/SkinSlider.java`(199), `skin/SkinGraph.java`(113), `skin/SkinSource.java`, `skin/SkinSourceImage.java`, `skin/SkinSourceImageSet.java`, `skin/SkinSourceMovie.java`, `skin/SkinSourceReference.java`, `skin/SkinSourceSet.java`, `skin/StretchType.java`(159), `skin/SkinPropertyMapper.java`(167), `skin/json/JsonSkinObjectLoader.java`(761), `ShaderManager.java`(32), `src/glsl/*.vert|frag` 8개.

근거 보강을 위해 추가로 읽은 파일: `skin/json/JsonSkin.java`(전부), `skin/json/JSONSkinLoader.java`(전부), `skin/json/JsonSkinSerializer.java`(전부), `skin/SkinLoader.java`(전부), `skin/lua/LuaSkinLoader.java`(전부), `MainState.java`(전부), `TimerManager.java`(전부), `ResourcePool.java`·`PixmapResourcePool.java`(전부), `skin/property/TimerProperty.java`·`TimerPropertyFactory.java`·`BooleanProperty.java`·`Event.java`(전부), 그리고 `MainController.java`, `input/KeyBoardInputProcesseor.java`, `video/FFmpegProcessor.java`, `skin/property/*Factory.java`, `Resolution.java` 의 관련 부분.

---

## 1. 전체 파이프라인

| 단계 | 내용 | 근거 |
| --- | --- | --- |
| 로드 | `.luaskin` 실행 결과 Lua 테이블을 `JsonSkin.Skin` 으로 리플렉션 변환한 뒤 JSON 스킨과 같은 `loadJsonSkin` 을 탄다 | `skin/lua/LuaSkinLoader.java:69-95, 169-208` |
| 객체 생성 | `sk.destination` 배열을 **선언 순서대로** 순회하며 항목마다 새 `SkinObject` 를 만들고 `skin.add(obj)` → `setDestination` | `skin/json/JSONSkinLoader.java:313-336` |
| 정적 제거 | 상태 전환 시 `newState.create()` 뒤 `skin.prepare(state)` 가 한 번 호출되어 절대 그려지지 않을 객체를 제거하고 `load()` 를 부른다 | `MainController.java:271-278`, `skin/Skin.java:199-267` |
| 프레임 | `timer.update()` → `current.render()` → `sprite.begin()` → `skin.updateCustomObjects()` → `skin.drawAllObjects()` → `sprite.end()` | `MainController.java:401-412` |
| 입력 | 그리기 뒤, 1ms 에 최대 1회 `current.input()` → 마우스 press/drag 를 스킨에 전달 | `MainController.java:494-508` |

---

## 2. 좌표계와 스케일

### 2.1 화면 좌표계

- `sprite = new SpriteBatch()` 를 기본 투영으로 쓴다 (`MainController.java:303`). 투영 행렬을 바꾸는 코드는 `config/SkinPreview.java:85,94` 뿐이다(grep 확인).
- **[libGDX]** 기본 투영은 `setToOrtho2D(0, 0, 창폭, 창높이)` 이다. 즉 **원점 좌하, y 는 위로 증가**, 단위는 생성 시점 창 픽셀이다. 창 크기는 `config.getResolution()` 으로 만든다 (`MainLoader.java:166-167`).
- `Skin` 은 좌표를 뒤집지 않는다 (`skin/Skin.java:150-156`). 따라서 JSON/Lua 스킨의 dst `x, y` 는 **사각형의 좌하 모서리**이고 `y` 는 화면 아래에서부터 잰다.
- 텍스트는 `region.y + region.height` 를 글자 윗선으로 쓴다 (`skin/SkinTextFont.java:175`). 좌하 원점임을 뒷받침한다.
- 좌상 원점 렌더러로 옮길 때: `top = H_dst - (y + h)`, 회전 방향과 center 번호(6.5절)도 함께 뒤집어 해석해야 한다.

### 2.2 소스 해상도와 스케일 (dw, dh)

- `dw = dst.width / org.width`, `dh = dst.height / org.height` (float) (`skin/Skin.java:108-115`).
- `dst` = `config.getResolution()` (`skin/json/JSONSkinLoader.java:80, 287`).
- `org`(소스 해상도)는 **`Resolution` enum 에 `sk.w × sk.h` 와 정확히 같은 항목이 있을 때만** 그 값이고, 없으면 `HD(1280×720)` 이다 (`skin/json/JSONSkinLoader.java:263-269`, `Resolution.java:9-23`). `sk.w`, `sk.h` 기본값은 1280, 720 (`skin/json/JsonSkin.java:12-13`). ModernChic 의 1920×1080 은 `FULLHD` 로 일치한다.
- 스케일 적용 위치 일람:

| 값 | 스케일 | 근거 |
| --- | --- | --- |
| dst `x, w` / `y, h` | `× dw` / `× dh`, 키프레임 등록 시점 | `skin/Skin.java:154` |
| `mouseRect` | `× dw`, `× dh` | `skin/Skin.java:183-185` |
| `clip_x..clip_h` | `× dw`, `× dh` | `skin/Skin.java:187-189` |
| 숫자 `space` | `× dw` (x 스케일만) | `skin/SkinNumber.java:126-128, 209-211`, `skin/SkinFloat.java:141-143` |
| 슬라이더 `range` | `(int)(range × dst.width / sk.w)` (angle 1,3) 또는 `(int)(range × dst.height / sk.h)` (angle 0,2). enum 이 아니라 **`sk.w/sk.h` 원값**을 쓴다 | `skin/json/JsonSkinObjectLoader.java:401-402` |
| 비트맵 폰트 `size` | `size × dst.width / sk.w` | `skin/json/JsonSkinObjectLoader.java:652` |
| TTF `size` | **스케일하지 않음**. 래스터 크기로만 쓰이고 실제 크기는 dst `h` 가 결정 (13.3절) | `skin/json/JsonSkinObjectLoader.java:661` |
| 숫자 자릿수별 `offset[]` | 스케일하지 않음 | `skin/json/JsonSkinObjectLoader.java:127-137` |
| 텍스트 `shadowOffsetX/Y` | 스케일하지 않음 | `skin/json/JsonSkinObjectLoader.java:673` |
| 오프셋 값(`SkinOffset`) | 스케일하지 않음 (대상 해상도 픽셀) | `MainState.java:122-133`, `skin/SkinObject.java:404-413` |
| stretch 의 이미지 크기 | 소스 픽셀 그대로 (6.3절) | `skin/StretchType.java` |

### 2.3 전체 오프셋 (플레이 스킨 한정)

- 플레이 스킨(5/7/9/10/14/24/24D)에서만 `OFFSET_ALL(10)` 을 읽는다 (`skin/Skin.java:720-729`, `skin/SkinProperty.java:950`).
- 배치의 변환 행렬 = 평행이동 `(width × x/100, height × y/100)` + 스케일 `((w+100)/100, (h+100)/100)`. `width, height` 는 대상 해상도 (`skin/Skin.java:382-389`).
- **첫 그리기 때 한 번만** 계산한다 (`renderer != null` 이면 즉시 반환, `skin/Skin.java:377-380`).

### 2.4 마우스 좌표

- `mousex = x × resolution.width / 창폭`, `mousey = resolution.height − y × resolution.height / 창높이` (정수 연산) (`input/KeyBoardInputProcesseor.java:187-188, 208-209, 216-217`). 즉 **대상 해상도 기준, 좌하 원점**이며 스킨 region 과 같은 좌표계다.

---

## 3. destination 모델

### 3.1 JSON/Lua 필드와 기본값

`JsonSkin.Destination` (`skin/json/JsonSkin.java:453-481`):

| 필드 | 타입 | 기본값 | 의미 |
| --- | --- | --- | --- |
| `id` | String | null | 객체 정의 id. 음수 정수 문자열이면 참조 이미지(9.4절) |
| `blend` | int | 0 | 6.4절 |
| `filter` | int | 0 | 6.6절 |
| `timer` | TimerProperty | null | 기준 타이머. 숫자·함수·이름 문자열 |
| `loop` | int | 0 | 4.2절 |
| `center` | int | 0 | 회전 중심 0~9 |
| `offset` | int | 0 | 오프셋 id 1개 |
| `offsets` | int[] | `[]` | 오프셋 id 여러 개 |
| `stretch` | int | -1 | -1 이면 변경 안 함(기본 STRETCH) |
| `op` | DestinationOption[] | `[]` | 숫자 또는 BooleanProperty(이름·함수) |
| `draw` | BooleanProperty | null | Lua 함수 등 |
| `dst` | Animation[] | `[]` | 키프레임 |
| `mouseRect` | Rect{x,y,w,h} int | null | 호버 표시 영역 |

`JsonSkin.Animation` (`skin/json/JsonSkin.java:507-529`): `time, x, y, w, h, clip_x, clip_y, clip_w, clip_h, acc, a, r, g, b, angle` 모두 **int**, 기본값 `Integer.MIN_VALUE`(미지정 표지).

- **[libGDX/luaj]** Lua 수치는 `LuaValue::toint` 로 변환된다 (`skin/lua/LuaSkinLoader.java:101`). 소수 좌표는 정수로 잘린다(luaj 의 double→int 는 0 방향 절삭, 지식 기반).

### 3.2 키프레임 누락값 채우기

`skin/json/JSONSkinLoader.java:422-468` (동일 복사본 `skin/json/JsonSkinObjectLoader.java:702-748`):

| 필드 | 첫 키프레임 미지정 시 | 이후 키프레임 미지정 시 |
| --- | --- | --- |
| `time` | 0 | 직전 값 |
| `x, y, w, h` | 0 | 직전 값 |
| `acc` | 0 | 직전 값 |
| `angle` | 0 | 직전 값 |
| `a, r, g, b` | 255 | 직전 값 |
| `clip_*` | 채우지 않음(MIN_VALUE 유지) | 직전 값 |

- 키프레임마다 `skin.setDestination(obj, time, x, y, w, h, acc, a, r, g, b, dst.blend, dst.filter, angle, dst.center, dst.loop, dst.timer, dst.getOptionIds(), draw)` 를 호출한다 (`JSONSkinLoader.java:459-460`).
- `clip_*` 4개가 모두 지정된 키프레임만 `setDestinationClip` 을 호출한다 (`JSONSkinLoader.java:461-463`).
- `mouseRect` 는 키프레임마다 다시 설정되지만 값은 같다 (`JSONSkinLoader.java:464-466`).
- 루프가 끝난 뒤 `obj.setOffsetID(offsets + [offset])`, `stretch >= 0` 이면 `obj.setStretch(stretch)` (`JSONSkinLoader.java:470-478`).
- `dst` 가 빈 배열이면 키프레임이 0개가 되어 `validate()` 가 false → `Skin.prepare` 에서 제거된다 (`skin/SkinObject.java:577-579`, `skin/Skin.java:201-202`).

### 3.3 키프레임 저장 (`SkinObject.setDestination`, `skin/SkinObject.java:195-255`)

- 색은 `Color(r/255, g/255, b/255, a/255)` 로 저장한다 (`:197-198`). **[libGDX]** `Color` 생성자는 0~1 로 클램프한다.
- 키프레임은 **time 오름차순으로 삽입**한다. 새 항목은 "자기보다 time 이 큰 첫 항목 앞"에 들어가므로 같은 time 은 선언 순서를 유지한다 (`:240-254`).
- `starttime = dst[0].time`, `endtime = dst[last].time` (`:245-246, 253-254`).
- **객체 수준 속성은 "아직 0/null 일 때만 덮어쓰기"** 규칙이다 (`:218-239`):

| 속성 | 규칙 |
| --- | --- |
| `acc` | `this.acc == 0` 이면 대입. 즉 **첫 번째 0 이 아닌 acc 가 객체 전체에 적용**된다. 키프레임별 `acc` 는 저장만 되고 보간에 쓰이지 않는다 (`getRate` 는 `this.acc` 사용, `:559`) |
| `blend` | `dstblend == 0` 이면 대입 |
| `filter` | `dstfilter == 0` 이면 대입 |
| `center` | `dstcenter == 0 && 0 <= center < 10` 이면 대입하고 `centerx/centery` 갱신. center=0 이면 매번 (0.5, 0.5) 로 설정 |
| `timer` | `dsttimer == null` 이면 대입 |
| `loop` | `dstloop == 0` 이면 대입 |

- 고정값 최적화: 모든 키프레임의 region / clip / color / angle 이 같으면 `fixr / fixclip / fixc / fixa` 를 유지하고 보간을 건너뛴다 (`:199-217`). 동작 결과는 같지만 **알파 오프셋 적용 여부가 달라지는 버그성 차이**가 있다(4.5절).

### 3.4 그리기 조건 등록

- `SkinObject.setDestination(..., int[] op, BooleanProperty[] draw)` 는 **`dstop` 과 `dstdraw` 가 모두 비어 있을 때만** 조건을 등록한다 (`skin/SkinObject.java:174-182`). 사실상 첫 키프레임에서 한 번 등록된다.
- `setDrawCondition(int[])` (`:282-299`): 0 은 무시, 중복 제거. 각 id 를 `BooleanPropertyFactory.getBooleanProperty(id)` 로 조회해
  - 내장 프로퍼티가 있으면 `dstdraw` 에 추가,
  - 없으면 `dstop`(커스텀 옵션 번호)에 추가.
- 음수 id: `getBooleanProperty(-n)` 은 id `n` 의 프로퍼티를 **부정**으로 감싼다. `|id| >= 65536` 이면 null (`skin/property/BooleanPropertyFactory.java:25-68`).
- `op` 개수 상한은 JSON/Lua 경로에 **없다**(배열 길이 자유). 3개 제한은 LR2 형식 오버로드(`op1, op2, op3`)에만 해당한다 (`skin/SkinObject.java:124-136`).
- `op` 원소가 문자열/함수이면 `DestinationOption.property` 가 되어 `dstdraw` 에 들어간다. 문자열은 먼저 내장 이름(`!` 접두사는 부정)으로 찾고, 없으면 Lua 식으로 컴파일한다 (`skin/lua/LuaSkinLoader.java:142-149, 153-166`, `skin/property/BooleanPropertyFactory.java:70-85`).
- `dst.draw` 는 `dstdraw` 맨 뒤에 추가된다 (`JSONSkinLoader.java:454-458`).
- 모든 조건은 **AND** 이다 (`skin/SkinObject.java:592-597`).

---

## 4. 시간·보간·오프셋

### 4.1 시각 기준

- `prepare(time, state)` 의 `time` 은 `state.timer.getNowTime()` = 상태 시작 후 경과 ms(μs/1000 절삭) (`skin/Skin.java:317`, `TimerManager.java:33-35`).
- 타이머: `timer.isOff(state)` 이면 **그리지 않음**, 아니면 `time -= timer.get(state)` (ms) (`skin/SkinObject.java:352-358`).
- 숫자 타이머 id 는 `TimerPropertyFactory.getTimerProperty(id)` 로 바뀐다. `id < 0` 이면 null(타이머 없음), **`id == 0` 은 타이머 0 번을 읽는 유효한 프로퍼티**다 (`skin/property/TimerPropertyFactory.java:6-9`). 타이머 0 번을 켜는 코드는 grep 으로 찾지 못했고 상태 진입 시 전 타이머가 OFF 로 초기화되므로 (`TimerManager.java:104`), Lua 에서 `timer = 0` 을 **명시하면 그 객체는 그려지지 않는다**. 필드를 생략하면 null 이다. ModernChic 에는 `timer = 0` 리터럴이 없다(grep 0건).
- Lua 함수 타이머는 매 프레임 호출되어 **μs 단위 타이머 시작 시각**을 반환해야 하며 `Long.MIN_VALUE` 가 OFF 다 (`skin/lua/SkinLuaAccessor.java:735-746`, `skin/property/TimerProperty.java:6-23`).
- 커스텀 타이머(id 10000~19999)는 `Skin.getMicroCustomTimer` 로 읽는다 (`TimerManager.java:59-65`, `skin/Skin.java:752-758`).

### 4.2 loop 처리 (`skin/SkinObject.java:360-375`)

```
if loop == -1:
    if time > endtime: time = -1
elif endtime > 0 and time > loop:
    if endtime == loop: time = loop
    else: time = (time - loop) % (endtime - loop) + loop      # Java % (피제수 부호를 따름)
if starttime > time: draw = false; return
```

| 경우 | 결과 |
| --- | --- |
| `loop = 0`(기본), `endtime > 0` | `time % endtime` 으로 전체 반복 |
| `loop = endtime` | 끝난 뒤 마지막 키프레임에서 정지 |
| `0 < loop < endtime` | `[loop, endtime)` 구간 반복 |
| `loop = -1` | `time > endtime` 이면 `time = -1` → `starttime(>=0) > -1` 이므로 **끝나면 사라짐** |
| `endtime == 0` (키프레임이 time 0 하나뿐), `loop != -1` | 변형 없음, 항상 표시 |
| `endtime == 0`, `loop == -1` | `time > 0` 이면 사라짐(사실상 표시 안 됨) |
| 키프레임이 `time = T > 0` 하나뿐, `loop = 0` | `time % T < T = starttime` 이라 **영원히 표시 안 됨** |
| `loop > endtime` | `time <= loop` 구간은 변형 없이 `time > endtime` 이 되어 `getRate` 가 폴백(index 0, rate 0)으로 **첫 키프레임**을 그린다. `time > loop` 이후도 음수 제수의 `%` 때문에 `loop` 이상으로 유지되어 첫 키프레임 |
| 기타 음수 loop(-2 등) | `(time - loop) % (endtime - loop) + loop` 로 `[loop, endtime)` 반복. `time < starttime` 구간은 표시 안 됨 |

### 4.3 보간 계수 (`getRate`, `skin/SkinObject.java:545-575`)

```
if nowtime == dst[last].time: rate = 0; index = last
else: 뒤에서부터 dst[i].time <= nowtime < dst[i+1].time 인 i 를 찾음
      rate = (float)(nowtime - t1) / (t2 - t1)
      acc == 1: rate = rate * rate                  # 가속
      acc == 2: rate = 1 - (rate - 1) * (rate - 1)  # 감속
      index = i
찾지 못하면: rate = 0; index = 0
```

- `acc == 3` 은 `getRate` 가 아니라 호출부에서 처리한다: **보간 없이 `dst[index]` 값을 그대로**(계단) 쓴다 (`:388-393, 445, 496-502, 537`).
- 그 밖의 acc 값(0, 4 이상)은 선형이다.

### 4.4 region / color / angle

- region (`:383-413`): `rate == 0` 이면 `dst[index].region`, `acc == 3` 이면 `dst[index].region`, 그 외 `v = v1 + (v2 - v1) × rate` (x, y, w, h 각각).
- color (`:480-520`): 같은 방식으로 r, g, b, a 를 각각 보간.
- angle (`:526-543`): `rate == 0 || acc == 3` 이면 `dst[index].angle`, 그 외 `(int)(a1 + (a2 - a1) × rate)` (int 절삭) 뒤 오프셋 `r` 합산.

### 4.5 오프셋 적용

- 유효 id 는 `1..199` (`OFFSET_MAX = 199`), 0 과 범위 밖은 버린다. 집합으로 중복 제거. **첫 `setOffsetID` 호출만 유효**하다 (`skin/SkinObject.java:832-846`, `skin/SkinProperty.java:972`).
- 매 prepare 에서 `off[i] = state.getOffsetValue(id)` = `main.getOffset(id)` 를 읽는다 (`:379-381`, `MainState.java:146-148`). `SkinOffset` 은 float `x, y, w, h, r, a` (`:774-781`).
- 스킨 헤더의 커스텀 오프셋 설정값은 `setSkin` 시 `main.offset[id]` 로 복사된다 (`MainState.java:116-135`).
- 적용 순서와 식 (`:404-413`, clip 은 `:460-469`):

```
region.x += off.x - off.w / 2      # relative 가 false 일 때만
region.y += off.y - off.h / 2      # relative 가 false 일 때만
region.width  += off.w
region.height += off.h
```

  즉 `w, h` 오프셋은 **중심 기준으로 확대**한다. `relative` 는 기본 false 이며 판정 숫자 등 특정 로더만 true 로 둔다 (`skin/json/JsonPlaySkinObjectLoader.java:260`).
- 알파: `a = clamp(color.a + off.a / 255, 0, 1)` (`:483-489, 513-519`).
- 각도: `angle += off.r` (`:529-533, 538-542`).
- **주의(원본 동작)**: `prepareColor` 는 색이 키프레임 간 달라서 보간 경로를 탈 때 `rate != 0` 이면 `return` 으로 빠져 **알파 오프셋을 적용하지 않는다** (`:493-512`). 색이 고정(`fixc`)이거나 `rate == 0` 일 때만 적용된다.
- 여러 오프셋은 모두 누적(합산)된다.

### 4.6 clip (시저)

- 키프레임에 clip 이 있으면 region 과 같은 방식(같은 rate, acc 3 은 계단)으로 보간하고, 다음 키프레임에 clip 이 없으면 현재 clip 을 유지한다 (`skin/SkinObject.java:435-458`).
- 오프셋을 region 과 같은 식으로 더하고 `offsetX/offsetY` 를 더한 뒤 `clip = width > 0 && height > 0` (`:460-472`).
- 그릴 때 `renderer.pushClip` → `ScissorStack` 으로 시저를 걸고 그린 뒤 pop 한다. 시저 면적이 0 이면 그리지 않는다 (`skin/Skin.java:361-375, 567-580`).
- `setDestinationClip` 은 **같은 time 의 키프레임**에 clip 을 붙인다 (`skin/SkinObject.java:257-265`).

---

## 5. 그리기 여부 결정

### 5.1 로드 직후 정적 제거 (`Skin.prepare`, `skin/Skin.java:199-241`)

객체별로:

1. `validate()` 가 false 면 제거.
2. `dstdraw` 의 각 `BooleanProperty` 에 대해 `isStatic(state)` 이 true 인데 값이 false 면 **제거**. static 이고 true 인 조건은 목록에서 빼고, non-static 만 남긴다.
3. `dstop`(커스텀 옵션 번호) 각각에 대해
   - `op > 0`: `option.get(op, -1) != 1` 이면 제거
   - `op < 0`: `option.get(-op, -1) != 0` 이면 제거
   
   `option` 은 헤더의 커스텀 옵션 항목마다 "선택됨 1 / 아님 0" 을 담은 맵이다 (`skin/json/JSONSkinLoader.java:291-297`). **내장 프로퍼티도 아니고 커스텀 옵션에도 없는 번호는 양수든 음수든 객체가 제거된다**(기본값 -1 이 어느 조건도 만족하지 못함).
4. 이후 `dstop` 을 빈 배열로 바꾸고 `option` 맵을 비운다 (`:230, 237`).
5. 남은 객체에 `load()` 호출 (`:239-241`).

- static 여부는 프로퍼티 정의가 정한다(`TYPE_NO_STATIC`, `TYPE_STATIC_WITHOUT_MUSICSELECT`, `TYPE_STATIC_ON_RESULT`, `TYPE_STATIC_ALL`, `skin/property/BooleanPropertyFactory.java:218-221`). Lua 함수 조건은 항상 non-static 이다 (`skin/lua/SkinLuaAccessor.java:611-628`). 개별 프로퍼티의 static 분류는 이 보고서 범위 밖이다.

### 5.2 매 프레임 (`SkinObject.prepare`, `skin/SkinObject.java:591-611`)

```
1. dstdraw 중 하나라도 false → draw = false; return
2. draw = true; prepareRegion(time, state)        # 타이머 OFF 또는 time < starttime 이면 draw = false
3. region.x += offsetX; region.y += offsetY; prepareClip(offsetX, offsetY)
4. mouseRect != null 이고 !mouseRect.contains(mouseX - region.x, mouseY - region.y) → draw = false; return
5. prepareColor(); prepareAngle()
```

- `mouseRect` 는 **객체 region 좌하 기준 상대 좌표**의 호버 영역이다. 마우스가 그 안에 있을 때만 그린다. **[libGDX]** `Rectangle.contains` 는 경계 포함(`x <= px <= x+w`)이다.
- 2 에서 `draw = false` 가 되어도 3~5 는 계속 실행되지만 `draw` 가 다시 true 가 되지는 않는다.

### 5.3 하위 타입의 평가 순서 (Lua 함수 호출 순서에 영향)

| 타입 | 순서 | 근거 |
| --- | --- | --- |
| SkinImage | `ref.get()` → 공통 prepare → 이미지 선택(그리지 않아도 `getImage` 호출) | `skin/SkinImage.java:123-141` |
| SkinNumber / SkinFloat | `ref.get()` → 공통 prepare → `draw` 면 자릿수 계산 | `skin/SkinNumber.java:134-160`, `skin/SkinFloat.java:149-176` |
| SkinText | 공통 prepare → `ref.get()` (그리지 않아도 호출) | `skin/SkinText.java:100-110` |
| SkinSlider / SkinGraph | 공통 prepare → `draw` 일 때만 이미지·값 조회 | `skin/SkinSlider.java:104-114`, `skin/SkinGraph.java:86-96` |

---

## 6. 공통 그리기 (`SkinObject.draw`, `skin/SkinObject.java:649-669`)

```
if color == null or color.a == 0 or image == null: return
tmpRect = (x, y, w, h)
stretch.stretchRect(tmpRect, tmpImage, image)     # tmpImage 는 image 의 복사 region
sprite.setColor(color); sprite.setBlend(dstblend)
sprite.setType( dstfilter != 0 and imageType == TYPE_NORMAL
                  ? (tmpRect.width == tmpImage.regionWidth and tmpRect.height == tmpImage.regionHeight ? TYPE_NORMAL : TYPE_BILINEAR)
                  : imageType )
if angle != 0: sprite.draw(tmpImage, x, y, w, h, centerx, centery, angle)
else:          sprite.draw(tmpImage, x, y, w, h)
```

### 6.1 색

- `out = texture × color(r, g, b, a)` (정점 색 곱). **[libGDX]** 기본 셰이더는 `v_color × texture2D`, 정점 알파는 float 패킹 때문에 254/255 단위로 양자화된다.
- `color.a == 0` 이면 **그리기 자체를 생략**한다.

### 6.2 음수 w/h (뒤집기)

- region 을 변환 없이 `SpriteBatch.draw(region, x, y, w, h)` 에 넘긴다. **[libGDX]** 정점이 `(x, y)`~`(x+w, y+h)` 로 만들어지므로 `w < 0` 이면 `x` 를 기준으로 왼쪽으로 뻗으며 **좌우 반전**, `h < 0` 이면 `y` 를 기준으로 아래로 뻗으며 **상하 반전**된다.
- 부작용: `filter != 0` 일 때 크기 일치 비교가 실패해 BILINEAR 가 선택되고, 클릭 판정 `r.x <= x && r.x + r.width >= x` 는 음수 폭에서 절대 참이 되지 않는다 (`skin/SkinObject.java:678`).

### 6.3 stretch (`skin/StretchType.java`)

기호: 대상 사각형 `R(x, y, w, h)`, 이미지 region 크기 `iw × ih`(소스 픽셀, `getRegionWidth/Height`).

보조 함수 (`:119-153`):

```
fitWidth(R, width):   cx = R.x + R.w/2; R.w = width;  R.x = cx - R.w/2
fitHeight(R, height): cy = R.y + R.h/2; R.h = height; R.y = cy - R.h/2
fitWidthTrimmed(R, scale, img):
    width = scale * img.regionWidth
    if R.w < width:                      # 넘치면 소스를 가운데 기준으로 잘라냄
        cx = img.regionX + img.regionWidth * 0.5
        w  = R.w / scale
        img.regionX = (int)(cx - w * 0.5); img.regionWidth = (int)w
    else: fitWidth(R, width)
fitHeightTrimmed(R, scale, img): 위와 대칭 (regionY, regionHeight)
```

| id | 이름 | 계산 |
| --- | --- | --- |
| 0 | STRETCH | 그대로 채움(기본) |
| 1 | KEEP_ASPECT_RATIO_FIT_INNER | `sx = R.w/iw, sy = R.h/ih`; `sx <= sy` 면 `fitHeight(R, ih×sx)` 아니면 `fitWidth(R, iw×sy)` |
| 2 | KEEP_ASPECT_RATIO_FIT_OUTER | `sx >= sy` 면 `fitHeight(R, ih×sx)` 아니면 `fitWidth(R, iw×sy)` (사각형이 원래 영역 밖으로 커짐) |
| 3 | KEEP_ASPECT_RATIO_FIT_OUTER_TRIMMED | `sx >= sy` 면 `fitHeightTrimmed(R, sx, img)` 아니면 `fitWidthTrimmed(R, sy, img)` |
| 4 | KEEP_ASPECT_RATIO_FIT_WIDTH | `fitHeight(R, ih × R.w/iw)` |
| 5 | KEEP_ASPECT_RATIO_FIT_WIDTH_TRIMMED | `fitHeightTrimmed(R, R.w/iw, img)` |
| 6 | KEEP_ASPECT_RATIO_FIT_HEIGHT | `fitWidth(R, iw × R.h/ih)` |
| 7 | KEEP_ASPECT_RATIO_FIT_HEIGHT_TRIMMED | `fitWidthTrimmed(R, R.h/ih, img)` |
| 8 | KEEP_ASPECT_RATIO_NO_EXPANDING | `s = min(1, R.w/iw, R.h/ih)`; `fitWidth(R, iw×s)`; `fitHeight(R, ih×s)` |
| 9 | NO_RESIZE | `fitWidth(R, iw)`; `fitHeight(R, ih)` |
| 10 | NO_RESIZE_TRIMMED | `fitWidthTrimmed(R, 1, img)`; `fitHeightTrimmed(R, 1, img)` |

- `setStretch(int)` 은 음수면 무시, 일치하는 id 가 없으면 무시한다 (`skin/SkinObject.java:318-327`).
- stretch 는 모든 `draw(sprite, image, ...)` 호출(이미지, 숫자 각 자릿수, 슬라이더, 그래프)에 적용된다. 텍스트(TTF, fnt)는 이 경로를 타지 않는다.

### 6.4 blend (`SkinObjectRenderer.preDraw / postDraw`, `skin/Skin.java:642-683`)

S = 프래그먼트(텍스처 × 정점색), D = 프레임버퍼.

| blend | GL 설정 | 식 |
| --- | --- | --- |
| 0, 1, 5~8, 10 이상 | 기본 `(SRC_ALPHA, ONE_MINUS_SRC_ALPHA)` | `C = S.rgb × S.a + D.rgb × (1 − S.a)` |
| 2 | `(SRC_ALPHA, ONE)` | `C = S.rgb × S.a + D.rgb` (가산) |
| 3 | `glBlendEquation(FUNC_SUBTRACT)` → `setBlendFunction(SRC_ALPHA, ONE)` → `glBlendEquation(FUNC_ADD)` | 아래 주의 참고 |
| 4 | `(ZERO, SRC_COLOR)` | `C = D.rgb × S.rgb` (곱셈) |
| 9 | `(ONE_MINUS_DST_COLOR, ZERO)` | `C = S.rgb × (1 − D.rgb)` (반전) |

- `blend >= 2` 이면 그린 뒤 기본 블렌드로 되돌린다 (`:680-682`).
- 프리멀티플라이가 아닌 직선 알파 기준이다.
- blend 4 와 9 는 알파 채널로 페이드되지 않는다(계수에 `S.a` 가 없음). 투명 픽셀의 rgb 값이 결과에 그대로 반영된다.
- **blend 3 주의 [libGDX]**: 감산 방정식을 켠 직후 곧바로 ADD 로 되돌리므로, 해당 객체 자신의 쿼드는 나중에 플러시될 때 **가산(blend 2 와 동일)** 으로 그려진다. 대신 `setBlendFunction` 이 유발하는 플러시로 **직전에 배치돼 있던 스프라이트들이 감산 방정식으로 그려지는** 부작용이 생긴다. 소스 주석도 `TODO 減算描画は難しいか？` 이다 (`:658`). ModernChic 은 blend 3 을 쓰지 않는다(19장).
- **텍스트는 자신의 `blend` 를 설정하지 않는다.** `SkinTextFont.draw`, `SkinTextBitmap.draw` 는 `sprite.setType` 만 호출하고 `setBlend` 를 호출하지 않으므로 (`skin/SkinTextFont.java:158-176`, `skin/SkinTextBitmap.java:86-110`), 렌더러에 남아 있는 **직전 이미지 객체의 blend 값**으로 그려진다.

### 6.5 회전

- `CENTERX = {0.5, 0, 0.5, 1, 0, 0.5, 1, 0, 0.5, 1}`, `CENTERY = {0.5, 0, 0, 0, 0.5, 0.5, 0.5, 1, 1, 1}` (`skin/SkinObject.java:80-81`). y 가 위로 증가하는 좌표계에서:

| center | (cx, cy) | 위치 |
| --- | --- | --- |
| 0 | (0.5, 0.5) | 중앙 |
| 1 | (0, 0) | 좌하 |
| 2 | (0.5, 0) | 하단 중앙 |
| 3 | (1, 0) | 우하 |
| 4 | (0, 0.5) | 좌 중앙 |
| 5 | (0.5, 0.5) | 중앙 |
| 6 | (1, 0.5) | 우 중앙 |
| 7 | (0, 1) | 좌상 |
| 8 | (0.5, 1) | 상단 중앙 |
| 9 | (1, 1) | 우상 |

- `sprite.draw(image, x + 0.01, y + 0.01, cx × w, cy × h, w, h, 1, 1, angle)` (`skin/Skin.java:620-626`). **[libGDX]** 회전은 도 단위, y-up 좌표계에서 **반시계 방향**이며 원점은 `(x + cx×w, y + cy×h)` 이다.
- `angle == 0` 이면 회전 없는 경로를 탄다.
- 텍스트는 angle 과 center 를 무시한다.

### 6.6 filter 와 렌더 타입

`SkinObjectRenderer` 타입 (`skin/Skin.java:537-542, 554-557, 628-636`):

| 타입 | 값 | 셰이더 | 텍스처 필터 |
| --- | --- | --- | --- |
| TYPE_NORMAL | 0 | 기본 | 건드리지 않음(텍스처 기본값) |
| TYPE_LINEAR | 1 | 기본 | 그릴 때 `Linear, Linear` 로 설정 |
| TYPE_BILINEAR | 2 | `bilinear` | 건드리지 않음 |
| TYPE_FFMPEG | 3 | `ffmpeg` | `Linear, Linear` |
| TYPE_LAYER | 4 | `layer` | 건드리지 않음 |
| TYPE_DISTANCE_FIELD | 5 | `distance_field` | `Linear, Linear` |

- 이미지류: `filter == 0` → TYPE_NORMAL. `filter != 0` → 그려지는 크기가 region 픽셀 크기와 **정확히 같으면** TYPE_NORMAL, 다르면 TYPE_BILINEAR (`skin/SkinObject.java:660-662`).
- **[libGDX]** 스킨 텍스처는 `new Texture(pixmap, false)` 로 만들어 기본 필터가 `Nearest/Nearest` 다. 따라서 `filter = 0` 은 최근접 샘플링이다.
- `bilinear.frag` (`src/glsl/bilinear.frag:13-38`) 의 실제 계산:

```
center_a = texture(uv).a                       # 최근접 알파
if center_a > 0:
    네 이웃(±0.5 텍셀)을 샘플 p00, p10, p01, p11
    a = fract(uv.x * texW + 0.5); b = fract(uv.y * texH + 0.5)
    result = mix( mix(p00*p00.a, p10*p10.a, a), mix(p01*p01.a, p11*p11.a, a), b ) / center_a
    result.a = center_a
    out = v_color * result
else: out = 0
```

  즉 **색은 알파 가중 쌍선형, 알파는 최근접**이다. 일반 선형 필터와 가장자리 모양이 다르다.
- `ffmpeg.frag`: `v_color × vec4(c.b, c.g, c.r, c.a)` (R/B 교환) (`src/glsl/ffmpeg.frag:11-14`).
- `layer.frag`: rgb 가 모두 0 인 텍셀은 알파 0 으로 처리(검정 투과) (`src/glsl/layer.frag:11-18`). 스킨 공통 객체에서는 쓰이지 않고 BGA 레이어 전용이다 (`play/bga/BGAProcessor.java:367`).
- `sprite.draw(TextureRegion ...)` 는 x, y 에 `+0.01` 을 더한다(Windows 에서 0.5 좌표 어긋남 임시 대처) (`skin/Skin.java:612-626`).

---

## 7. 그리기 순서와 2단계 처리

### 7.1 순서

- **`sk.destination` 배열 순서 = 그리기 순서**. 뒤에 선언된 것이 위에 그려진다 (`skin/json/JSONSkinLoader.java:313-336`, `skin/Skin.java:325-329`).
- 객체 종류 간 별도 레이어나 z 값은 없다. 종류가 달라도 destination 순서만 따른다.
- **destination 항목 하나 = 객체 하나**. 같은 id 를 여러 destination 이 참조하면 서로 독립된 객체가 만들어진다.
- 정의를 찾지 못하거나 소스 로드에 실패한 destination 은 건너뛴다(객체 없음) (`JSONSkinLoader.java:332-335`).
- 노트·판정·곡 목록 바·게이지 같은 복합 객체는 자신의 `draw` 안에서 내부 요소를 그린다(범위 밖).

### 7.2 id 가 겹칠 때의 정의 탐색 순서 (`skin/json/JsonSkinObjectLoader.java:39-559`)

1. id 가 **음수 정수 문자열**이면 `new SkinImage(-id)` (참조 이미지) (`JSONSkinLoader.java:315-322`)
2. `image` → 3. `imageset` → 4. `value` → 5. `floatvalue` → 6. `text` → 7. `slider` → 8. `graph` → 9. `gaugegraph` → 10. `judgegraph` → 11. `bpmgraph` → 12. `hiterrorvisualizer` → 13. `timingvisualizer` → 14. `timingdistributiongraph` → 15. `gauge`
3. 공통 로더가 null 을 반환하면 화면별 로더가 이어서 찾는다(play: note, hidden/lift cover, practice, bga, judge, pmchara / select: songlist / skinselect: skinpreview) (`skin/json/JsonPlaySkinObjectLoader.java:30-325`, `JsonSelectSkinObjectLoader.java:31-37`, `JsonSkinConfigurationSkinObjectLoader.java:22-27`).

- `image` 배열에서 id 가 일치했는데 소스가 없어 `obj == null` 이면 **그 자리에서 null 을 반환**하고 뒤 종류를 찾지 않는다 (`JsonSkinObjectLoader.java:43-72`).

### 7.3 prepare / draw 2단계 (`skin/Skin.java:276-331`)

```
microtime = state.timer.getNowMicroTime()
if nextpreparetime <= microtime:
    time = state.timer.getNowTime()                 # ms
    for obj in objectarray: obj.prepare(time, state)
    nextpreparetime += ((microtime - nextpreparetime) / prepareduration + 1) * prepareduration
for obj in objectarray:
    if obj.draw: drawObject(obj)                    # clip 이 있으면 시저 push/pop
```

- `prepareduration = prepareFramePerSecond > 0 ? 1000000 / fps : 1` μs. 설정 기본값이 0 이므로 기본은 **매 프레임 prepare** 다 (`skin/Skin.java:265`, `Config.java:75`).
- **모든 객체의 prepare 가 끝난 뒤에 draw 가 시작**된다. Lua 의 `draw`/`value`/`timer` 함수는 prepare 단계에서 객체 순서대로 프레임당 1회씩 불린다.
- prepare 에서 계산되는 것: `draw` 플래그, `region`, `clip`, `color`, `angle`, 그리고 타입별 현재 이미지·자릿수 배열·문자열·슬라이더/그래프 값.
- draw 에서 계산되는 것: stretch, 블렌드·셰이더 선택, 텍스트 레이아웃(문자열이 바뀌었을 때만 `setText`).
- 그 앞에 매 프레임 `updateCustomObjects`(커스텀 타이머 → 커스텀 이벤트)가 돈다 (`MainController.java:409`, `skin/Skin.java:782-789`). 주석은 "ID 오름차순"이라 하지만 `IntMap` 순회라 순서 보장은 **[미확인]** 이다.
- 미리보기 화면만 예외를 삼키는 `drawAllObjectsSafely` 를 쓴다 (`config/SkinPreview.java:90`). 일반 화면은 prepare/draw 의 예외가 그대로 전파된다.

---

## 8. 소스 이미지 로드

### 8.1 source 정의와 지연 로드

- `sk.source[] = {id, path}` → `sourceMap[id] = SourceData(path)` (`skin/json/JSONSkinLoader.java:309-311`).
- 처음 참조될 때 로드하고 `loaded = true` 로 표시한다. 실패해도 `loaded = true` 가 되어 **재시도하지 않는다** (`JSONSkinLoader.java:481-520`, `JsonSkinObjectLoader.java:561-582`).
- 파일 경로 = `SkinLoader.getPath(스킨파일의 부모 디렉터리 + "/" + path, filemap)`.
- 두 가지 진입점이 `loaded` 플래그를 공유한다.
  - `loader.getSource` (동영상 인식): `image` 에서만 사용 (`JsonSkinObjectLoader.java:44, 80`)
  - `getTexture(srcid, p)` (텍스처 전용): `value`, `floatvalue`, `slider`, `graph`, `gauge`, 노트에서 사용 (`:101, 176, 396, 423, 526`)

### 8.2 경로 해석 (`SkinLoader.getPath`, `skin/SkinLoader.java:95-130`)

1. `filemap`(커스텀 파일 선택: 키 = `스킨디렉터리/패턴경로`, 값 = 선택 파일명)의 키 중 `imagepath.startsWith(key)` 인 것이 있으면
   `imagepath.substring(0, lastIndexOf('*')) + 선택파일명 + (imagepath 에서 key 를 뺀 나머지)` 로 치환하고 끝낸다.
2. 그렇지 않고 경로에 `*` 가 있으면
   - `ext = '*' 뒤 문자열`. `|` 가 있으면 `'*'~'|' 사이 + 마지막 '|' 뒤`.
   - 마지막 `/` 앞 디렉터리의 파일 중 소문자 경로가 `ext` 로 끝나는 것을 모아 **무작위(`Math.random`)로 하나** 고른다.
   - 없으면 원래 경로 그대로(존재하지 않는 파일).
3. `*` 가 없으면 그대로.

- **확장자 대체(예: png 가 없으면 jpg)는 스킨 소스에는 없다.** 파일이 없으면 그 source 는 null 이다.
- `filemap` 은 헤더 커스텀 파일 중 선택값이 있는 것만 채운다 (`skin/lua/LuaSkinLoader.java:78-83`).

### 8.3 텍스처 생성과 캐시

- 확장자가 `mp4, m4v, wmv, webm, mpg, mpeg, m1v, m2v, avi` 면 `SkinSourceMovie`, 아니면 텍스처 (`JSONSkinLoader.java:496-515`, `video/VideoFormat.java:7-13`).
- 텍스처: `SkinLoader.getTexture(path, usecim=false)` → `new Texture(pixmap, useMipMaps=false)` (`skin/SkinLoader.java:132-178`, `JSONSkinLoader.java:66, 81`). **밉맵 없음, 필터는 libGDX 기본(Nearest)**. 필터를 Linear 로 바꾸는 것은 6.6절의 타입 1, 3, 5 뿐이다.
- 비트맵 폰트 페이지만 `useMipMaps = true` 로 만든다 (`skin/SkinTextBitmap.java:368-370, 398`).
- Pixmap 은 경로를 키로 하는 `PixmapResourcePool` 에 캐시된다. 스킨 로드가 끝날 때마다 `disposeOld()` 로 세대를 올리고 `maxgen`(=`config.getSkinPixmapGen()`) 을 넘긴 항목을 해제한다 (`skin/SkinLoader.java:65, 136-139`, `ResourcePool.java:87-103`, `MainController.java:304`).
- 디코드: jpg/jpeg 는 ImageIO, webp 는 FFmpeg(다중 프레임이면 스프라이트 시트로 합침), 그 외는 libGDX 네이티브 디코더 후 실패 시 ImageIO 재시도 (`PixmapResourcePool.java:210-264`).
- 로드된 텍스처·폰트·동영상은 `skin.addResource` 로 등록되어 스킨 dispose 때 해제된다 (`JSONSkinLoader.java:402-410`, `skin/Skin.java:422-466`).

### 8.4 region 분할 (`getSourceImage`, `skin/json/JsonSkinObjectLoader.java:601-621`)

```
w == -1 → 텍스처 폭;  h == -1 → 텍스처 높이
divx <= 0 → 1;  divy <= 0 → 1
images[divx * j + i] = TextureRegion(tex, x + (w / divx) * i, y + (h / divy) * j, w / divx, h / divy)   # 정수 나눗셈
  (i: 0..divx-1 열, j: 0..divy-1 행 → 행 우선 순서)
```

- `x, y, w, h` 기본값은 **0** 이다(`-1` 이 아님). `w, h` 를 생략하면 0×0 region 이 된다 (`skin/json/JsonSkin.java:115-118`).
- **[libGDX]** `TextureRegion(tex, x, y, w, h)` 의 좌표는 이미지 좌상 원점·y 아래 방향 픽셀이며, 음수 `w/h` 는 UV 가 뒤집힌 region 이 된다.

---

## 9. SkinImage

### 9.1 `image` 정의 (`skin/json/JsonSkin.java:112-127`)

| 필드 | 기본값 | 의미 |
| --- | --- | --- |
| `id`, `src` | null | |
| `x, y, w, h` | 0 | 소스 사각형. `-1` 이면 전체 |
| `divx, divy` | 1 | 분할 수 |
| `timer` | null | 애니메이션 기준 타이머 |
| `cycle` | 0 | 전체 프레임 1순환 시간(ms). 0 이면 첫 프레임 고정 |
| `len` | 0 | 2 이상이면 프레임을 `len` 개 세트로 나눔 |
| `ref` | 0 | 세트 선택용 이미지 인덱스 프로퍼티 id |
| `act` | null | 클릭 이벤트(숫자 id / 이름 / Lua 함수) |
| `click` | 0 | 클릭 판정 종류(15장) |

생성 규칙 (`JsonSkinObjectLoader.java:42-73`):

- source 가 동영상 → `new SkinImage(SkinSourceMovie)`.
- 텍스처이고 `len > 1` → 프레임 `N = divx×divy` 개를 `len` 세트로 나눈다. 세트 `i` = 프레임 `[i×(N/len) .. i×(N/len) + N/len − 1]`. `new SkinImage(tr, timer, cycle, ref)`, 세트 선택은 `IntegerPropertyFactory.getImageIndexProperty(ref)`.
- 텍스처이고 `len <= 1` → 전체 프레임이 한 세트. `new SkinImage(frames, timer, cycle)`.
- `act != null` 이면 `setClickevent(act)`, `setClickeventType(click)`.

### 9.2 `imageset` 정의 (`skin/json/JsonSkin.java:129-136`, 로더 `:74-98`)

| 필드 | 기본값 | 의미 |
| --- | --- | --- |
| `images` | `[]` | `image` id 목록. 각 원소가 한 소스(자체 `timer/cycle/div` 애니메이션 포함) |
| `ref` | 0 | 인덱스 프로퍼티 id |
| `value` | null | IntegerProperty(함수 등). 있으면 `ref` 대신 사용 |
| `act`, `click` | null, 0 | 클릭 |

- `images[k]` 의 id 를 `image` 배열에서 못 찾거나 텍스처가 없으면 그 칸은 null 이다. 원소 image 의 `len` 과 동영상은 무시된다.

### 9.3 프레임 선택과 그리기

- 세트 선택 (`skin/SkinImage.java:123-141`): `value = ref != null ? ref.get(state) : 0`.
  - `value < 0` → 그리지 않음.
  - `value >= image.length` → `value = 0`.
  - `image[value]` 가 null(제거된 소스)이거나 프레임이 null → 그리지 않음.
- `validate()`: 유효한 소스가 하나도 없으면 객체 제거 (`:95-117`).
- 애니메이션 프레임 (`skin/SkinSourceImage.java:73-89`):

```
if cycle == 0: return 0
if timer != null:
    if timer.isOff: return 0
    time -= timer.get(state)
if time < 0: return 0
return (int)((time * length / cycle) % length)
```

  여기서 `time` 은 **상태 경과 ms 원값**이다. dst 의 `timer` 나 `loop` 로 보정된 시간이 아니다. 즉 이미지 애니메이션 타이머(`image.timer`)와 dst 타이머는 서로 독립이다.
- `getImageIndexProperty(0)` 은 정의가 없어 null 이다(팩토리에서 id 0 항목 grep 0건) → `ref` 생략 시 항상 세트 0.
- 그리기: `draw(sprite, currentImage, region.x, region.y, region.width, region.height)` (`:143-151`).

### 9.4 참조 이미지 (`skin/SkinSourceReference.java:36-43`)

destination id 가 음수 정수면 `SkinImage(-id)`:

| destination id | 내용 | 없을 때 |
| --- | --- | --- |
| `-100` | 스테이지 파일 (`BMSResource.getStagefile()`) | null → 그리지 않음 |
| `-101` | 백 BMP (`getBackbmp()`) | null → 그리지 않음 |
| `-102` | 배너 (`getBanner()`) | null → 그리지 않음 |
| `-110` | 1×1 불투명 검정 | |
| `-111` | 1×1 불투명 흰색(색 채우기 사각형 용도) | |
| 그 외 음수 | 항상 null → 그리지 않음 | |

- 검정/흰색은 2×1 Pixmap 한 장의 두 텍셀이다 (`:26-34`).
- BGA 는 참조 이미지가 아니라 플레이 스킨의 별도 `bga` 객체다 (`skin/json/JsonSkin.java:46`).

### 9.5 동영상 소스 (`skin/SkinSourceMovie.java`, `video/FFmpegProcessor.java`)

- 생성 시 디코더 스레드를 시작한다 (`SkinSourceMovie.java:33-37`, `FFmpegProcessor.java:60-69`).
- **첫 `getImage` 호출 시 `play(time, loop=true)`**. 이후 매 호출 `getFrame(time)` 으로 현재 시각을 넘긴다 (`SkinSourceMovie.java:43-55`).
- `SkinImage.prepare` 는 `draw` 여부와 무관하게 `getImage` 를 부르므로 **스킨의 첫 prepare 에서 재생이 시작**되고, dst 타이머와 무관하게 상태 시각을 따라 진행한다 (`skin/SkinImage.java:132-136`).
- 재시작 시 `offset = grabber.getTimestamp() − time × 1000` 으로 현재 시각을 영상 0 초에 맞춘다 (`FFmpegProcessor.java:331-347`). EOF 이고 loop 면 다시 LOOP 명령을 넣어 처음부터 반복한다 (`FFmpegProcessor.java:198-202`).
- 프레임 준비 전에는 null → 그리지 않음 (`FFmpegProcessor.java:71-78`).
- 그릴 때 타입 3(FFMPEG 셰이더, R/B 교환, Linear)을 쓴다 (`skin/SkinImage.java:144-147`).
- `validate()` 는 항상 true (`SkinSourceMovie.java:39-41`).

---

## 10. SkinNumber

### 10.1 `value` 정의 (`skin/json/JsonSkin.java:138-157`)

| 필드 | 기본값 | 의미 |
| --- | --- | --- |
| `src, x, y, w, h, divx, divy, timer, cycle` | 이미지와 같음 | 숫자 시트 |
| `align` | 0 | 10.4절 |
| `digit` | 0 | 표시 자릿수(`keta`) |
| `padding` | 0 | 10/11 프레임 시트에서 쓰는 패딩 모드 |
| `zeropadding` | 0 | 24 프레임 시트에서 쓰는 패딩 모드 |
| `space` | 0 | 자릿수 사이 간격(소스 px, `× dw`) |
| `ref` | 0 | 정수 프로퍼티 id |
| `value` | null | IntegerProperty. 있으면 `ref` 대신 사용 |
| `offset` | null | 자릿수별 `{x, y, w, h}` 배열(왼쪽 칸부터) |

### 10.2 시트 프레임 수 규칙 (`skin/json/JsonSkinObjectLoader.java:99-172`)

`N = divx × divy`:

| 조건 | 해석 | 생성 |
| --- | --- | --- |
| `N % 24 == 0` | 세트당 24장 = 양수 12장(0~9, 10=뒷면 0, 11=부호) + 음수 12장. 세트 수 `N/24` | `SkinNumber(pn, mn, timer, cycle, digit, **zeropadding**, space, ref, align)` |
| 그 외, `N % 10 == 0` | 세트당 10장(0~9). 세트 수 `N/10` | `SkinNumber(nimages, timer, cycle, digit, **padding**, space, ref, align)`, 음수 시트 없음 |
| 그 외 | 세트당 11장(0~9, 10=뒷면 0). 세트 수 `N/11`(정수 나눗셈) | 패딩 모드를 **2 로 강제** |

- 24 의 배수 판정이 먼저다. 예: 120 장은 10×12 가 아니라 24×5 로 해석된다.
- 24 장 시트는 `zeropadding`, 10/11 장 시트는 `padding` 필드를 읽는다. 서로 다른 필드다.
- 세트는 `timer/cycle` 로 애니메이션된다 (`skin/SkinSourceImageSet.java:72-88`, 식은 9.3절과 같음).

### 10.3 자릿수 배치 (`skin/SkinNumber.java:138-194`)

- `value` 가 `Integer.MIN_VALUE` 또는 `MAX_VALUE` → 그리지 않음. `ref` 가 null 이면 MIN_VALUE 취급 (`:135, 139-143`).
- 시트 선택: `value >= 0 || mimage == null` 이면 양수 시트, 아니면 음수 시트 (`:144`).
- `v = abs(value)`, 칸 `j = keta−1 → 0` (오른쪽 칸부터):

```
A) 음수 시트가 있고 zeropadding > 0:
     j == 0                     → image[11]                       # 맨 왼쪽 칸은 항상 부호
     v > 0 or j == keta-1       → image[v % 10]
     else                       → image[zeropadding == 2 ? 10 : 0]
B) 그 외:
     v > 0 or j == keta-1       → image[v % 10]
     else zeropadding == 2      → image[10]
     else zeropadding == 1      → image[0]
     else (패딩 없음)            → 음수 시트가 있고 오른쪽 칸(j+1)이 숫자(부호·null 아님)이면 image[11], 아니면 null
   칸이 null 이면 shiftbase++
   v /= 10
```

- 자릿수가 `keta` 를 넘으면 **하위 `keta` 자리만** 표시된다(A 는 `keta−1` 자리).
- 패딩 없는 24 장 시트는 최상위 숫자 바로 왼쪽 한 칸에 부호가 붙는다.
- 10 장 시트에 패딩 모드 2 를 주면 `image[10]` 접근으로 `ArrayIndexOutOfBounds` 가 난다(원본 동작).
- 결과는 값과 세트가 바뀔 때만 다시 계산한다 (`:162`).

### 10.4 위치 (`:191-207`)

```
cell   = region.width + space * dw
length = cell * (keta - shiftbase)
shift  = align == 0 ? 0 : align == 1 ? cell * shiftbase : cell * 0.5 * shiftbase
칸 j: x = region.x + cell * j - shift (+ offsets[j].x)
      y = region.y (+ offsets[j].y)
      w = region.width (+ offsets[j].w);  h = region.height (+ offsets[j].h)
```

- dst 의 `w` 는 **한 자릿수의 폭**이다. 전체 폭은 `cell × keta`.
- 칸 0 이 가장 왼쪽(최상위)이다.
- `align = 0`: 숫자가 상자 오른쪽에 붙음(왼쪽 빈칸). `align = 1`: 빈칸만큼 왼쪽으로 당겨 **왼쪽 정렬**. `align = 2`: 절반만 당겨 **가운데 정렬**.
- 각 칸은 공통 `draw` 로 그리므로 stretch, blend, filter, 회전(칸별 회전)이 그대로 적용된다.
- `getLength()` 는 현재 표시 폭을 준다(판정 숫자 배치 등에 사용) (`:225-227`).

---

## 11. SkinFloat 와 FloatFormatter

### 11.1 `floatvalue` 정의 (`skin/json/JsonSkin.java:159-181`)

`value` 와 같은 시트 필드에 더해: `iketa`(정수 자릿수, 0), `fketa`(소수 자릿수, 0), `gain`(배율, 1.0), `isSignvisible`(false), `zeropadding`(0), `padding`(0, 사용 안 함), `align`(0), `space`(0), `ref`(int, `FloatPropertyFactory.getFloatProperty(id)`), `value`(FloatProperty), `offset`.

### 11.2 시트 프레임 수 규칙 (`skin/json/JsonSkinObjectLoader.java:174-383`)

판정 순서대로:

| 조건 | 세트 구성 | 부호 |
| --- | --- | --- |
| `N % 26 == 0` | 양 13 + 음 13 (0~9, 10=뒷면 0, 11=소수점, 12=부호) | `isSignvisible` 반영 |
| `N % 24 == 0` | 양 12 + 음 12 (0~9, 10, 11) | 강제 false |
| `N % 22 == 0` | 양 11 + 음 11. `[10] = 프레임 0`(뒷면 0 공유), `[11] = 프레임 10`(소수점). 음수는 `+11` | 강제 false |
| `N % 12 == 0` | 12장(양·음 공용) | 강제 false |
| `N % 11 == 0` | 11장. `[10] = 프레임 0`, `[11] = 프레임 10` | 강제 false |
| 그 외 | 12 의 배수로 간주(`N/12` 세트) | 강제 false |

### 11.3 FloatFormatter (`skin/FloatFormatter.java`)

생성자 (`:55-71`):

```
iketa, fketa 는 음수면 0;  zeropadding 은 0/1/2 로 정규화;  sign = isSignvisible ? 1 : 0
if iketa >= 8 or fketa >= 8 or iketa + fketa >= 8:
    fketa = min(fketa, 8);  iketa = 8 - fketa
length = sign + iketa + fketa + (fketa != 0 ? 1 : 0)
digits = int[length + 1]  (인덱스 0 미사용);  base = sign + iketa
```

`calcuateAndGetDigits(value)` (`:73-128`), `value = abs(v)`:

```
digits 를 -1 로 채움
if iketa == 0 and fketa == 0 and sign == 1: digits[1] = 12; return
isSign = sign == 1 and value < 10^iketa
if zeropadding == 0:
    ival = (int)value
    base = min(iketa, (int)log10(ival != 0 ? ival : 1) + 1) + sign
fval = (long)(value * 10^fketa)                 # 절삭, 반올림 아님
nowketa = iketa == 0 ? fketa + sign + 1 : base + fketa + (fketa != 0 ? 1 : 0)
fcnt = fketa
while nowketa > sign:
    if fcnt > -1:  digits[nowketa] = fval % 10
    else:          digits[nowketa] = (fval == 0 and zeropadding == 2) ? 10 : fval % 10
    fcnt--
    if fcnt == 0: nowketa--; digits[nowketa] = 11          # 소수점
    fval /= 10; nowketa--
if nowketa == 1: digits[1] = isSign ? 12 : fval % 10       # 부호 칸. 자릿수 초과 시 숫자가 들어감
if iketa == 0 and sign == 1: digits[1] = 12
```

- 코드값: 0~9 숫자, 10 뒷면 0, 11 소수점, 12 부호, −1 빈칸.
- 숫자는 **배열 앞쪽(왼쪽)부터 채워지고 빈칸은 뒤(오른쪽)에 남는다**. SkinNumber 와 반대다.
- 일의 자리는 `fcnt > -1` 분기에서 쓰이므로 뒷면 0 이 되지 않는다.
- `zeropadding != 0` 이면 `base` 가 생성자 값(`sign + iketa`)으로 유지된다.

### 11.4 SkinFloat 준비와 위치 (`skin/SkinFloat.java:153-207`)

- `v = value × gain`. `value` 가 `Float.MIN_VALUE/MAX_VALUE`, `v` 가 무한·NaN·MIN/MAX, 또는 `keta == 0` 이면 그리지 않음.
- 시트: `mimage == null || v >= 0` 이면 양수, 아니면 음수.
- `currentImages[k−1] = digits[k] != −1 ? image[digits[k]] : null`, null 마다 `shiftbase++`.
- 위치:

```
cell  = region.width + space * dw
shift = align == 0 ? 0 : align == 1 ? cell * shiftbase : cell * 0.5 * shiftbase
칸 j: x = region.x + cell * j + shift (+ offsets[j].x)        # SkinNumber 는 "- shift"
```

- 빈칸이 오른쪽에 있고 `+ shift` 이므로: `align = 0` **왼쪽 정렬**, `align = 1` **오른쪽 정렬**, `align = 2` 가운데 정렬. **SkinNumber 의 0/1 과 의미가 반대**다.

---

## 12. 텍스트

### 12.1 `font` 과 `text` 정의

`font` (`skin/json/JsonSkin.java:100-110`): `id`, `path`, `fallback[]`(문자열 또는 `{path, type}`), `type`(0 standard, 1 distance field, 2 colored distance field).

`text` (`skin/json/JsonSkin.java:183-201`):

| 필드 | 기본값 | 의미 |
| --- | --- | --- |
| `font` | null | font id |
| `size` | 0 | 글자 크기(의미가 TTF/fnt 에서 다름) |
| `align` | 0 | 0 왼쪽, 1 가운데, 2 오른쪽 |
| `ref` | 0 | 문자열 프로퍼티 id |
| `value` | null | StringProperty(함수 등). 있으면 `ref` 대신 사용 |
| `event` | null | StringWriter(편집 결과 기록) |
| `constantText` | null | 고정 문자열 |
| `editable` | false | |
| `wrapping` | false | 줄바꿈 |
| `overflow` | 0 | 0 넘침 허용, 1 축소, 2 잘라냄 |
| `outlineColor` | `"ffffff00"` | RRGGBBAA |
| `outlineWidth` | 0 | |
| `shadowColor` | `"ffffff00"` | |
| `shadowOffsetX/Y` | 0 | |
| `shadowSmoothness` | 0 | |

생성 (`skin/json/JsonSkinObjectLoader.java:627-679`):

- 폰트 경로 = `스킨파일 부모.resolve(font.path)`. **와일드카드 해석(`getPath`)을 거치지 않는다.**
- 경로가 대소문자 무시 `.fnt` 로 끝나면 `SkinTextBitmap`, 아니면 `SkinTextFont`(FreeType).
- 프로퍼티 = `text.value ?? StringPropertyFactory.getStringProperty(text.ref)`.
- 색 파싱 실패 시 흰색(`Color.WHITE`) (`:681-687`).
- `writer = text.event ?? StringPropertyFactory.getStringWriter(text.ref)`, `editable = text.editable || (text.event == null && writer != null)`. 즉 **기록자가 내장된 ref 는 자동으로 편집 가능**해진다.
- 폰트 정의가 없으면 null → destination 건너뜀.

### 12.2 공통 동작 (`skin/SkinText.java`)

- 문자열 결정 (`:100-110`): `currentText = ref != null ? ref.get(state) : constantText`. **`ref` 가 null 이 아니면 `constantText` 는 무시**된다. null 은 `""` 로 바뀐다.
- 빈 문자열이면 그리지 않는다. 단 `editable && writer != null` 이면 빈 문자열도 그리기 대상으로 남긴다(클릭 영역 유지).
- 그릴 때 문자열이 바뀌었으면 `setText` → `prepareText` (`:112-117`).
- 앵커 x (`SkinTextFont.java:167`, `SkinTextBitmap.java:92`):

```
align == 2: x = region.x - region.width
align == 1: x = region.x - region.width / 2
align == 0: x = region.x
```

  레이아웃은 `targetWidth = region.width` 안에서 다시 같은 방향으로 정렬하므로, 결과적으로 **dst `x` 가 기준점**이 된다: 왼쪽 정렬은 글 왼쪽 끝, 가운데는 글 중앙, 오른쪽은 글 오른쪽 끝이 `region.x` 에 온다. dst `w` 는 최대 폭(축소·잘라냄·줄바꿈 기준)이다.
- 세로 위치: `font.draw(..., y = region.y + region.height)`. **[libGDX]** 이 y 는 **대문자 윗선(cap top)** 이다. 기준선은 `y − capHeight × scaleY`, 줄바꿈 시 다음 줄은 `lineHeight × scaleY` 만큼 아래.
- 레이아웃 (`SkinTextFont.java:272-281`, `SkinTextBitmap.java:262-271`):

| 조건 | 호출 | 결과 |
| --- | --- | --- |
| `wrapping` | `setText(font, text, WHITE, width, ALIGN[align], true)` | 폭 기준 줄바꿈. `overflow` 무시 |
| `overflow` 0 | `setText(..., width, ALIGN[align], false)` | 넘쳐도 그대로 |
| `overflow` 1 | 위와 같이 배치 후 `layout.width > region.width` 면 `scaleX *= region.width / layout.width` 로 **가로만 축소**하고 다시 배치 | 세로 크기 유지 |
| `overflow` 2 | `setText(font, text, 0, len, WHITE, width, ALIGN[align], false, "")` | **말줄임표 없이** 폭을 넘는 글리프를 잘라냄(truncate 문자열이 빈 문자열) |

- `ALIGN = {Align.left, Align.center, Align.right}` (`skin/SkinText.java:36`).
- 색: 레이아웃의 모든 run 색을 객체 색(r, g, b, a)으로 설정한다 (`SkinTextFont.java:283-287`).
- 텍스트는 stretch, angle, center, blend(6.4절 주의)를 쓰지 않는다.
- **[libGDX]** `GlyphLayout` 의 줄 폭은 첫 글리프의 왼쪽 여백과 마지막 글리프의 advance 여분을 다듬은 값이다(첫 글리프 `−xoffset×scaleX − padLeft`, 마지막 글리프 `(xoffset + width)×scaleX − padRight`). 정렬·축소 판정이 이 폭을 쓰므로 원문 대조가 필요하다.

### 12.3 TTF: SkinTextFont (`skin/SkinTextFont.java`)

- `parameter.size = text.size`(스케일 없음), `incremental = true`, `characters = ""` (`:88-93`).
- **실제 크기**: `scaleY = region.height / parameter.size`, `scaleX = scaleY` (`:238, 247-248`). 즉 **dst `h` 가 em 크기(px)** 이고 `size` 는 래스터 해상도일 뿐이다.
- 폰트 생성: 문자열이 바뀔 때마다 그 문자열의 문자만으로 `generator.generateFont(parameter)` 를 다시 만든다(`prepareFont` 로 미리 준비한 경우 제외) (`:131-155`). 직후 리플렉션으로 증분 생성을 끈다 (`:528-535`).
- **[libGDX]** FreeType 생성 폰트는 `integer = true` 라 글리프 사각형의 x, y, 폭, 높이가 **정수로 반올림**된다. 스케일이 1 이 아니면 글리프마다 반올림 오차가 생긴다.
- 필터: `filter != 0` → TYPE_LINEAR(텍스처를 Linear 로), 아니면 TYPE_NORMAL (`:165`).
- 그림자 (`:168-173`): `shadowOffset` 이 (0, 0) 이 아니면 먼저 색 `(r/2, g/2, b/2, a)` 로 `(x + shadowOffset.x, y − shadowOffset.y)` 에 그린다. y-up 이므로 양수 Y 는 **아래쪽**이다. `outline*`, `shadowColor`, `shadowSmoothness` 는 TTF 에서 쓰이지 않는다.
- BMP 밖 문자와 누락 글리프는 fallback 폰트 → 대체 글리프(□ 등) → 직접 그린 사각형 순으로 채운다 (`:310-467`). 원본 beatoraja 스킨 재현에는 필수 아님.
- `validate()`: 폰트 파일을 못 열면 객체 제거 (`:96-101`, `skin/SkinFontSource.java:33-59`).
- 같은 경로+fallback 조합은 `SkinFontSource`(FreeType face)를 공유한다 (`skin/json/JSONSkinLoader.java:393-420`).

### 12.4 비트맵 폰트: SkinTextBitmap (`skin/SkinTextBitmap.java`)

- 로드 (`:379-449`): `.fnt` 를 libGDX `BitmapFontData` 로 읽고, 페이지 이미지를 `SkinLoader.getTexture(path, usecim, useMipMaps=true)` 로 로드한다. **[libGDX]** 페이지 경로는 `.fnt` 의 부모 디렉터리 기준이다.
- `originalSize` = `.fnt` 첫 줄 `size=` 의 정수, `pageWidth/pageHeight` = 둘째 줄 `scaleW=`/`scaleH=` (`:408-415`). 파싱 실패 시 `lineHeight` 와 첫 페이지 크기로 대체 (`:416-424`).
- 크기: 객체의 `size = text.size × dst.width / sk.w`, `scale = size / originalSize`, `scaleX = scaleY = scale` (`:226, 236-237`). **dst `h` 는 글자 크기에 영향이 없고** 세로 기준선 위치와 입력 영역에만 쓰인다.
- 그린 뒤 `font.getData().setScale(1)` 로 되돌린다(폰트 데이터 공유) (`:109`).
- 실제 그리기용 폰트는 `new BitmapFont(fontData, regions, false)` → 정수 반올림 **없음** (`:472`).
- 타입별 (`:93-108`):

| font.type | 렌더 타입 | 그림자·외곽선 |
| --- | --- | --- |
| 0 (standard) | TYPE_BILINEAR(`filter` 와 무관) | 그림자만: TTF 와 같은 방식(색 절반, 오프셋). 외곽선 없음 |
| 1, 2 (distance field) | TYPE_DISTANCE_FIELD(Linear) | 셰이더 유니폼으로 외곽선·그림자 |

- 타입 1 과 2 는 그리기 경로가 같다(`colored` 전용 처리는 없음).
- 유니폼 (`:179-186`):

```
u_outlineDistance = max(0.1, 0.5 - outlineWidth / 2)
u_outlineColor    = outlineColor
u_shadowColor     = shadowColor
u_shadowSmoothing = shadowSmoothness / 2
u_shadowOffset    = (shadowOffset.x / pageWidth, shadowOffset.y / pageHeight)
```

- `distance_field.frag` (`src/glsl/distance_field.frag:15-29`), `smoothing = 1/16` 고정:

```
d            = texture(uv).a
outlineF     = smoothstep(0.5 - s, 0.5 + s, d)
color        = mix(u_outlineColor, v_color, outlineF)
alpha        = smoothstep(u_outlineDistance - s, u_outlineDistance + s, d)
main         = vec4(color.rgb, color.a * alpha)
shadowD      = texture(uv - u_shadowOffset).a
shadowA      = smoothstep(0.5 - u_shadowSmoothing, 0.5 + u_shadowSmoothing, shadowD)
shadow       = vec4(u_shadowColor.rgb, u_shadowColor.a * shadowA)
out          = mix(shadow, main, main.a)
```

  기본값(외곽선·그림자 알파 0)에서도 `out.a = main.a²`, `out.rgb = mix(흰색, main.rgb, main.a)` 가 되어 가장자리가 일반 SDF 와 다르다.
- **[libGDX]** `.fnt` 파싱 규칙(구현 시 대조 필요): `padding=상,우,하,좌` 를 읽어 `capHeight -= padTop + padBottom`, `ascent = base − capHeight`, 글리프 `yoffset = −(height + 파일 yoffset)`, 줄 간격 `down = −lineHeight`, `id > 0xFFFF` 글리프는 무시. ModernChic 의 fnt 는 모두 `padding=4|8|10`, `spacing=−8|−16|−20` 인 Hiero 형식이라 이 패딩 처리가 위치에 직접 영향을 준다(19장).
- 폰트 캐시: 스킨 내에서는 font id 별 1개(`bitmapSourceMap`), 전역으로는 `(경로, type)` 키의 참조 카운트 캐시 (`skin/json/JsonSkinObjectLoader.java:637-651`, `skin/BitmapFontCache.java:52-77`).
- BMP 밖 문자는 사설 영역(U+E000~F8FF)으로 재매핑, `〜`(U+301C)와 `～`(U+FF5E)는 상호 대체, 누락 글리프는 □ 계열 대체 (`:485-515, 611-737`).

### 12.5 이미지 폰트: SkinTextImage (`skin/SkinTextImage.java`)

- LR2 형식 전용이다. JSON/Lua `createText` 는 `.fnt` 와 TTF 만 만든다 (`skin/json/JsonSkinObjectLoader.java:636-662`).
- 참고 식 (`:106-125`): `width = Σ글리프폭 × region.height / size + margin × 글자수`, `scale = region.width < width ? region.width / width : 1`, 앵커는 12.2절과 같되 폭이 `width × scale`, 글자마다 `tw = regionWidth × scale × region.height / size`, `dx += tw + margin × scale`.

### 12.6 편집 가능한 텍스트 (`skin/SkinTextInput.java`, `skin/Skin.java:394-411`)

- 클릭 시 위에서부터 훑다가 `text.isEditable() && text.draw && text.getInputBounds().contains(x, y)` 인 텍스트를 만나면 포커스하고 전파를 멈춘다.
- 입력 영역 = 12.2절 앵커 x, `region.y`, `region.width`, `region.height` (`skin/SkinText.java:162-169`).
- `writer` 가 없으면 포커스하지 않는다 (`SkinTextInput.java:36-38`).
- 구현은 libGDX scene2d `TextField` 오버레이다. 시스템 폰트(`config.getSystemfontpath()`)를 `round(region.height)` 크기로 만들고 글자색은 객체 색 (`:42-51, 122-135`).
- 확정: Enter, 영역 밖 클릭, 포커스 상실 → `writer.set(state, text)` 후 `target.setText(text)` (`:54-81, 147-162`).
- 포커스 중에는 키보드 입력 프로세서를 텍스트 입력 모드로 바꾼다 (`:51, 69`).

---

## 13. SkinSlider

### 13.1 `slider` 정의 (`skin/json/JsonSkin.java:203-223`)

| 필드 | 기본값 | 의미 |
| --- | --- | --- |
| 시트 필드 | 이미지와 같음 | 손잡이 이미지 |
| `angle` | 0 | 이동 방향: 0 위, 1 오른쪽, 2 아래, 3 왼쪽 |
| `range` | 0 | 이동 범위(소스 px, 2.2절 스케일) |
| `type` | 0 | 비율 프로퍼티 id |
| `changeable` | **true** | 드래그로 값 쓰기 허용 |
| `value` | null | FloatProperty |
| `event` | null | FloatWriter |
| `isRefNum` | false | `type` 을 정수 프로퍼티로 보고 `min~max` 비율로 변환 |
| `min`, `max` | 0 | |

생성 우선순위 (`skin/json/JsonSkinObjectLoader.java:394-418`):

1. `value != null` → 읽기 `value`, 쓰기 `event`(null 가능).
2. `isRefNum` → `RateProperty(type, min, max)`, 쓰기 없음.
3. 그 외 → `getRateProperty(type)`, 쓰기 `changeable ? getRateWriter(type) : null`.

### 13.2 그리기 (`skin/SkinSlider.java:116-121`)

```
x = region.x + (dir == 1 ?  v * range : dir == 3 ? -v * range : 0)
y = region.y + (dir == 0 ?  v * range : dir == 2 ? -v * range : 0)
draw(image, x, y, region.width, region.height)
```

`v` 는 프로퍼티 값(보통 0~1). 이미지 애니메이션은 9.3절과 같다.

### 13.3 쓰기 (`mousePressed`, `:123-185`)

`writer != null` 일 때만. region 은 이동 전 기준 위치다.

| dir | 판정 영역 | 값 |
| --- | --- | --- |
| 0 | `region.x <= x <= region.x + width`, `region.y <= y <= region.y + range` | `(y − region.y) / range` |
| 1 | `region.x <= x <= region.x + range`, `region.y <= y <= region.y + height` | `(x − region.x) / range` |
| 2 | `region.x <= x <= region.x + width`, `region.y − range <= y <= region.y` | `(region.y − y) / range` |
| 3 | `region.x − range <= x <= region.x`, `region.y <= y <= region.y + height` | `(region.x − x) / range` |

- 시작점에서 1px 미만이면 0, 끝점에서 1px 미만이면 1 로 스냅한다.
- 판정 영역은 이동 축 방향으로 `range` 만이고 손잡이 자체 크기는 포함하지 않는다.
- 클릭과 드래그 모두 이 메서드를 탄다 (`skin/Skin.java:407, 413-420`).

### 13.4 RateProperty (`skin/SkinObject.java:788-820`)

```
value = IntegerProperty(type).get(state)   (프로퍼티 없으면 0)
min < max:  value > max → 1;  value < min → 0;  else |(value - min) / (max - min)|
min >= max: value < max → 1;  value > min → 0;  else |(value - min) / (max - min)|
```

`min == max == value` 이면 0/0 으로 NaN 이 된다.

---

## 14. SkinGraph

### 14.1 `graph` 정의 (`skin/json/JsonSkin.java:225-242`)

시트 필드 + `angle`(**기본 1**), `type`(0), `value`(null), `isRefNum`(false), `min`/`max`(0).

생성 (`skin/json/JsonSkinObjectLoader.java:420-455`):

- `type < 0` → 곡 선택용 분포 그래프 `SkinDistributionGraph`(범위 밖). `type == −1` 은 11열, 그 외 음수는 28열로 프레임을 나눈다.
- `type >= 0` → `value` > `isRefNum`(RateProperty) > `getRateProperty(type)` 순.

### 14.2 그리기 (`skin/SkinGraph.java:98-107`)

`v` = 비율, 이미지 region 크기 `iw × ih`:

```
direction == 1:
    src = image 의 (0, ih - (int)(ih * v), iw, (int)(ih * v))     # 소스의 아래쪽 부분
    draw(src, region.x, region.y, region.width, region.height * v)
그 외:
    src = image 의 (0, 0, (int)(iw * v), ih)                      # 소스의 왼쪽 부분
    draw(src, region.x, region.y, region.width * v, region.height)
```

- `direction == 1`: `region.y`(아래 변)에 고정되어 **위로 자라며** 소스는 아래에서부터 드러난다. 주석은 "1: 下" 라고 적혀 있으나 식은 위와 같다.
- 그 외: 왼쪽 변에 고정되어 오른쪽으로 자란다.
- SkinGraph 자체는 `v` 를 클램프하지 않는다.
- 소스 좌표는 int 절삭, 대상 크기는 float 이다.

---

## 15. 마우스 이벤트

### 15.1 전달 (`MainController.java:494-508`)

- `touchDown` → `mousepressed = true`, `touchDragged` → `mousedragged = true`. 프레임 끝에서 플래그를 지우고 스킨에 전달한다. 버튼을 뗄 때(`touchUp`)는 아무 처리도 하지 않는다 (`input/KeyBoardInputProcesseor.java:206-225`).
- 현재 상태의 `input()` 이 먼저 불리고 그 다음 스킨이 받는다.

### 15.2 press (`Skin.mousePressed`, `skin/Skin.java:394-411`)

```
textInput 이 있으면 영역 밖 클릭 시 확정
for obj in objectarray 역순(위에 그려진 것부터):
    편집 가능한 텍스트이고 draw 이고 입력 영역에 포함 → 포커스; break
    obj.draw and obj.mousePressed(state, button, x, y) → break
```

- **가장 위에 그려진, 클릭을 처리한 객체 하나만** 이벤트를 받는다.
- `clickevent` 가 없는 객체는 false 를 반환하므로 **위에 덮여 있어도 클릭을 막지 않는다**.
- `draw` 는 마지막 prepare 결과다(조건·타이머·mouseRect 반영).

### 15.3 클릭 판정 (`SkinObject.mousePressed`, `skin/SkinObject.java:671-704`)

- 영역: `r.x <= x <= r.x + r.width && r.y <= y <= r.y + r.height`. `r` 은 오프셋까지 적용된 region(stretch 적용 전).
- `inc = buttonEvents[button]`, `buttonEvents = {1, −1, 1, 1, −1}`(범위 밖 0). **[libGDX]** 버튼 0 좌, 1 우, 2 중, 3 back, 4 forward.

| `click` | 인자 |
| --- | --- |
| 0 | `exec(state, inc)` (좌클릭 +1, 우클릭 −1) |
| 1 | `exec(state, −inc)` |
| 2 | `exec(state, x >= r.x + r.width / 2 ? 1 : −1)` (오른쪽 절반 +1, 왼쪽 −1) |
| 3 | `exec(state, y >= r.y + r.height / 2 ? 1 : −1)` (위 절반 +1, 아래 −1) |

- `Event.exec(state, arg1)` 은 `exec(state, arg1, 0)` 이다 (`skin/property/Event.java:14-18`).
- 숫자 `act` 는 `EventFactory.getEvent(id)` 로, 내장에 없으면 `state.executeEvent(id, arg1, arg2)`(커스텀 이벤트)로 연결된다 (`skin/property/EventFactory.java:47-60`, `MainState.java:90-94`).
- JSON/Lua 에서 `act`/`click` 을 받는 것은 **`image` 와 `imageset` 뿐**이다 (`skin/json/JsonSkinObjectLoader.java:67-70, 92-95`).

### 15.4 drag (`Skin.mouseDragged`, `skin/Skin.java:413-420`)

- 역순으로 훑어 **`SkinSlider` 이면서 `draw` 인 객체**에만 `mousePressed` 를 호출하고, 처리되면 멈춘다. 다른 객체는 드래그를 받지 않는다.

### 15.5 호버

- 별도 이벤트는 없다. `mouseRect`(5.2절)가 매 prepare 에서 현재 마우스 위치를 읽어 표시 여부를 정한다.

---

## 16. 기타

### 16.1 SkinPropertyMapper (`skin/SkinPropertyMapper.java`)

플레이어 `p`(0, 1), 키 `k` 에 대한 id 계산. 모두 `p >= 2` 또는 `k >= 100` 이면 −1.

| 함수 | `k < 10` | `10 <= k < 100` |
| --- | --- | --- |
| `bombTimerId` | `TIMER_BOMB_1P_SCRATCH + k + p×10` | `TIMER_BOMB_1P_KEY10 + k − 10 + p×100` |
| `holdTimerId` | `TIMER_HOLD_1P_SCRATCH + ...` | `TIMER_HOLD_1P_KEY10 + ...` |
| `hcnActiveTimerId` | `TIMER_HCN_ACTIVE_1P_SCRATCH + ...` | `TIMER_HCN_ACTIVE_1P_KEY10 + ...` |
| `hcnDamageTimerId` | `TIMER_HCN_DAMAGE_1P_SCRATCH + ...` | `TIMER_HCN_DAMAGE_1P_KEY10 + ...` |
| `keyOnTimerId` | `TIMER_KEYON_1P_SCRATCH + ...` | `TIMER_KEYON_1P_KEY10 + ...` |
| `keyOffTimerId` | `TIMER_KEYOFF_1P_SCRATCH + ...` | `TIMER_KEYOFF_1P_KEY10 + ...` |
| `keyJudgeValueId` | `VALUE_JUDGE_1P_SCRATCH + ...` | `VALUE_JUDGE_1P_KEY10 + ...` |

그 밖: 스킨 선택 버튼 id ↔ `SkinType` 변환, 커스터마이즈 버튼/카테고리/항목 인덱스, `isCustomEventId`(`EVENT_CUSTOM_BEGIN..END`), `isCustomTimerId`(`TIMER_CUSTOM_BEGIN=10000..END=19999`), `isTimerWritableBySkin`(커스텀 타이머만 true), `isEventRunnableBySkin`(항상 true) (`:101-166`). 상수 값 자체는 `skin/SkinProperty.java` 조사 범위다.

### 16.2 ShaderManager (`ShaderManager.java:13-23`)

- 이름별로 `glsl/<name>.vert`, `glsl/<name>.frag` 를 클래스패스에서 읽어 컴파일하고 전역 맵에 캐시한다. 컴파일 실패 시 null(기본 셰이더로 대체되는 효과).
- 사용 이름: `bilinear`, `ffmpeg`, `layer`, `distance_field` (`skin/Skin.java:554-557`).
- 정점 셰이더는 네 개 모두 `gl_Position = u_projTrans × a_position`, 색·UV 전달만 한다.

### 16.3 해제

- `SkinObject` 는 `DisposableObject` 를 상속하고, `Skin.dispose` 가 살아 있는 객체·제거된 객체·등록 리소스를 모두 해제한다 (`skin/Skin.java:422-441`).

---

## 17. Lua 테이블 → JsonSkin 변환에서 객체 모델에 영향을 주는 규칙

(`skin/lua/LuaSkinLoader.java:97-208`)

- 필드 이름이 Java 필드와 같을 때만 대입된다. 없는 키는 Java 기본값 유지.
- `int` 는 `toint`, `float` 는 `tofloat`, `boolean` 은 `toboolean`, `String` 은 `tojstring`.
- 프로퍼티 타입 필드는 **함수 → Lua 함수 래핑 / 숫자 → id 조회 / 문자열 → 이름 조회 후 실패 시 Lua 식 컴파일** 순서다 (`:153-166`).

| 필드 타입 | 숫자일 때 | 문자열일 때(이름) |
| --- | --- | --- |
| `BooleanProperty`(`draw`, `op` 원소) | `getBooleanProperty(id)` | `getBooleanProperty(name)` |
| `IntegerProperty`(`value.value`, `imageset.value`) | `getIntegerProperty(id)` | 같은 이름 조회 |
| `FloatProperty`(`slider.value`, `graph.value`, `floatvalue.value`) | `getRateProperty(id)` | `getRateProperty(name)` |
| `StringProperty`(`text.value`) | `getStringProperty(id)` | 같은 이름 조회 |
| `TimerProperty`(`timer`) | `getTimerProperty(id)` | 이름 조회 없음(식 컴파일) |
| `FloatWriter`(`slider.event`) | `getRateWriter(id)` | `getRateWriter(name)` |
| `StringWriter`(`text.event`) | 없음(null) | `getStringWriter(name)` |
| `Event`(`act`) | `getEvent(id)` | `getEvent(name)` |

- 배열은 `table.keys()` 순서로 변환된다 (`:172-181`). **[luaj]** 배열부 순서 보장은 지식 기반이며 Lua 로더 담당 조사에서 확정해야 한다.
- JSON 파일의 `if`/`value`/`values`/`include` 조건 분기는 JSON 직렬화기 전용이고 (`skin/json/JsonSkinSerializer.java:171-339`) Lua 경로에는 적용되지 않는다.

---

## 18. 구현 시 반드시 재현해야 할 원본 특이 동작 목록

1. `acc`, `blend`, `filter`, `center`, `timer`, `loop` 는 키프레임이 아니라 **객체 단위**이며 "처음 나온 0 이 아닌 값"이 채택된다(3.3절).
2. `time > 0` 키프레임 하나뿐이고 `loop = 0` 인 객체는 표시되지 않는다(4.2절).
3. 내장도 커스텀 옵션도 아닌 `op` 번호는 부호와 무관하게 객체를 제거한다(5.1절).
4. `timer = 0` 을 명시하면 꺼진 타이머 0 번에 묶여 표시되지 않는다(4.1절).
5. 색 보간 중(`rate != 0`)에는 알파 오프셋이 적용되지 않는다(4.5절).
6. 텍스트는 자신의 `blend` 를 쓰지 않고 직전 객체의 blend 를 물려받는다(6.4절).
7. `filter != 0` 은 선형 필터가 아니라 "크기가 다를 때만 `bilinear` 셰이더(알파 최근접)"다(6.6절).
8. SkinNumber 와 SkinFloat 의 `align` 0/1 의미가 서로 반대다(10.4, 11.4절).
9. 숫자 시트 해석은 프레임 수의 24 → 10 → 11 판정 순서, float 은 26 → 24 → 22 → 12 → 11 → 기타 순서다. `padding` 과 `zeropadding` 은 시트 종류에 따라 서로 다른 필드가 읽힌다(10.2, 11.2절).
10. 이미지 `w/h` 생략은 0 이다(전체는 −1)(8.4절).
11. 이미지 애니메이션 시간은 dst 타이머·loop 와 무관한 상태 경과 시간이다(9.3절).
12. 동영상은 표시 여부와 무관하게 첫 prepare 부터 재생·반복된다(9.5절).
13. TTF 는 dst `h` 가 글자 크기, fnt 는 `size × dw` 가 글자 크기다(12.3, 12.4절).
14. `ref` 가 유효하면 `constantText` 는 무시된다(12.2절).
15. 클릭은 `clickevent` 가 있는 최상위 객체 하나만 받고, 드래그는 슬라이더만 받는다(15장).
16. 음수 `w/h` 는 반전되어 그려지지만 클릭되지 않는다(6.2절).
17. 소스 해상도가 `Resolution` enum 에 없으면 1280×720 으로 간주한다(2.2절).
18. 전체 오프셋(`OFFSET_ALL`)은 첫 그리기에 고정된다(2.3절).

---

## 19. 참고: ModernChic 이 실제로 쓰는 필드(grep 집계)

배정 범위 밖이지만 우선순위 판단을 위해 `/Users/hyunseokbyun/Downloads/ModernChic` 의 `*.lua` 를 grep 한 결과다. 변수 경유 사용은 일부 누락될 수 있다.

| 항목 | 관찰 |
| --- | --- |
| `blend` | `MAIN.BLEND.ADDITION` 71건, `MAIN.BLEND.ALPHA` 21건, 리터럴 `9` 1건. 3, 4 리터럴 없음 |
| `filter` | `MAIN.FILTER.OFF` 34건, `MAIN.FILTER.ON` 3건 |
| `acc` | `DECELERATE` 80건, `ACCELERATION` 7건, `CONSTANT` 2건, 변수 6건 |
| `center` | 리터럴 `8` 1건 |
| `stretch` | `FIT_WIDTH_TRIMMED` 12건, `FIT_OUTER_TRIMMED` 10건, `FIT_INNER` 3건 |
| `loop` | `−1` 271건, 양수 다수(1000, 3000, 500, 1500 등), `0` 19건 |
| `timer = 0` 리터럴 | 0건 |
| 텍스트 | `overflow` 84건, `constantText` 32건, `shadowOffsetX/Y` 각 45건, `outlineWidth` 23건 |
| 기타 | `offsets` 130건, `mouseRect` 25건, `changeable` 13건, `zeropadding` 42건, `padding` 1건 |
| `clip_*`, `editable`, `wrapping`, `isRefNum`, `iketa/fketa/gain` | 0건 |
| 폰트 | TTF(mgenplus) 와 fnt 를 함께 사용. fnt 10개 중 Select/Decide 의 main·sub 4개가 `type = 1`(distance field), 나머지 6개(Select bartext, Play title·info·top, Result main·sub)는 type 생략(0) |
| fnt 헤더 | 전부 `padding=4,4,4,4`/`8,8,8,8`/`10,10,10,10` + 음수 `spacing`, `size` 는 25~90 양수 |

`MAIN.BLEND.*`, `MAIN.ACC.*`, `MAIN.STRETCH.*` 등의 숫자 값은 ModernChic 의 Lua 상수 정의를 조사하는 쪽에서 확정해야 한다 **[미확인]**.

---

## 20. 미확인·범위 밖 정리

| 항목 | 상태 |
| --- | --- |
| libGDX 1.9.9 `SpriteBatch`/`GlyphLayout`/`BitmapFont`/`BitmapFontCache`/`FreeTypeFontGenerator` 의 세부(글리프 배치, 정수 반올림, 패딩 스케일, 플러시 시점, 회전 방향) | 소스 없음. 지식 기반 서술이며 원문 대조 필요 |
| luaj 의 `toint` 절삭 방향, `table.keys()` 순서 | 지식 기반 |
| blend 3 의 부작용(직전 배치가 감산으로 그려짐) | libGDX 플러시 동작에 의존하는 추론 |
| `Skin.updateCustomObjects` 의 ID 순회 순서 | `IntMap` 순회라 주석대로인지 미확인 |
| 각 `BooleanProperty` 의 static 분류, 프로퍼티 id 목록, `SkinProperty` 상수 | 범위 밖 |
| 복합 객체(노트, 판정, 게이지, 곡 목록 바, BGA, 각종 그래프·비주얼라이저)의 내부 그리기 | 범위 밖. 로더가 만드는 지점만 확인 |
| `SkinLuaAccessor` 의 값·이벤트·쓰기 함수 래핑 세부 | `loadBooleanProperty`, `loadTimerProperty`, `loadEvent` 앞부분만 확인 |
| `video/FFmpegProcessor.java` | 재생·루프·프레임 전달 부분만 확인(418줄 중 약 150줄) |
| `MainController.java`, `MainLoader.java`, `Config.java`, `BMSResource.java`, `skin/property/*Factory.java` | 관련 부분만 발췌 확인 |
| 이 저장소의 beatoraja 가 ModernChic 제작 시점 버전과 어떻게 다른지 | 미확인 |

배정된 파일은 모두 끝까지 읽었다. 읽지 못한 배정 파일은 없다.
