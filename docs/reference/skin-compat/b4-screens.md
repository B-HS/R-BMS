# B4. beatoraja 선택·결정·결과·코스 결과·키 설정·스킨 설정 화면의 스킨 계약과 화면 수명주기

> 최종 갱신 2026-10-09 · 대응 단계: L1 조사(구현 전) · 기준 커밋 `9ce92bb` · 색인과 갱신 규칙은 [README.md](README.md)

조사 대상: `/Users/hyunseokbyun/development/beatoraja` (origin `exch-bms2/beatoraja`, HEAD `8320241d`, `MainController.VERSION = "beatoraja 0.8.9"` — `MainController.java:52`).
경로 표기는 별도 언급이 없으면 `src/bms/player/beatoraja/` 기준 상대 경로다.
"미확인"은 원본에서 확인하지 못한 항목이다. 읽지 못한 파일은 문서 끝 §12 에 적었다.

---

## 0. 구현자가 먼저 알아야 할 결론

| # | 결론 | 근거 |
|---|------|------|
| 1 | 화면 전환은 `current.shutdown()` → `current.setSkin(null)`(이전 스킨 dispose) → `newState.create()`(스킨을 동기 로드) → `skin.prepare(state)` → `timer.setMainState()`(모든 타이머 OFF, 기준 시각 리셋) → `newState.prepare()` 순서다. `create()` 안에서 켠 타이머는 `setMainState` 에서 전부 지워진다. | `MainController.java:270-284`, `TimerManager.java:101-107` |
| 2 | 한 프레임은 `timer.update()` → `current.render()`(상태 로직) → `skin.updateCustomObjects` → `skin.drawAllObjects` → (ms 당 최대 1회) `current.input()` → 스킨 `mousePressed` → `mouseDragged` 순서다. | `MainController.java:401-413`, `495-508` |
| 3 | JSON/Lua 스킨의 `scene` 을 생략하면 0 이 되어 결정·결과 화면이 첫 프레임에 바로 FADEOUT 으로 들어간다(`Skin.scene` 기본값 `3600000*24` 는 로더가 `sk.scene`(기본 0)으로 덮어쓴다). | `Skin.java:85`, `JsonSkin.java:16`, `JSONSkinLoader.java:305-307` |
| 4 | 선택 화면은 `fadeout`/`scene`/`TIMER_FADEOUT` 을 쓰지 않는다. 결정으로의 전환은 즉시다. | `MusicSelector.java:191-291`, `426`, `527` |
| 5 | `songlist` 의 막대 본체는 반드시 `imageset` id 로 지정해야 하고, imageset 의 `images[]` 인덱스가 막대 종류(0 곡, 1 폴더, 2 표/해시/랜덤, 3 코스, 4 곡 없음, 5 커맨드/컨테이너, 6 검색)다. | `JsonSelectSkinObjectLoader.java:44-77`, `BarRenderer.java:150-166`, `261` |
| 6 | 스크롤 보간은 스킨 타이머가 아니라 `System.currentTimeMillis()` 기반이며, 이웃 슬롯 좌표 차이에 선형 비율을 곱한다. x 는 [-1,1] 로 자르고 y 는 자르지 않는다. | `BarRenderer.java:113-142` |
| 7 | `TIMER_SONGBAR_MOVE(10)`, `_MOVE_UP(12)`, `_MOVE_DOWN(13)`, `_STOP(14)`, `TIMER_README_BEGIN/END(15/16)`, `TIMER_PANEL4~6_ON/OFF` 는 상수만 있고 어디서도 켜지 않는다. 선택 화면이 실제로 켜는 타이머는 `STARTINPUT(1)`, `SONGBAR_CHANGE(11)`, `PANEL1~3_ON(21~23)`, `PANEL1~3_OFF(31~33)`, `IR_CONNECT_BEGIN/SUCCESS/FAIL(172~174)` 뿐이다. | `skin/SkinProperty.java:15-34`, 전체 grep 결과(사용처 없음), `MusicSelector.java:193-198`, `248-250`, `553-565` |
| 8 | JSON/Lua 스킨은 결과 화면의 `ranktime` 을 설정할 수 없다(LR2 로더만 `setRankTime` 호출). 따라서 `TIMER_RESULT_UPDATESCORE(152)` 는 첫 render 프레임에 켜진다. | `result/MusicResult.java:165-167`, `skin/lr2/LR2ResultSkinLoader.java:41`, `JSONSkinLoader.java:305-307` |
| 9 | `gaugegraph` 는 JSON/Lua 에서 `delay`(1500ms)·`lineWidth`(2px)를 바꿀 수 없다. 그래프가 드러나는 진행률은 대상 객체의 dst 타이머가 아니라 화면 시작 후 경과 시간(`timer.getNowTime()`)으로 계산한다. | `result/SkinGaugeGraphObject.java:30-34`, `113-114`, `JsonSkin.java:244-261`, `Skin.java:317-320` |
| 10 | 키 설정 화면은 스킨 기반이 아니다. `KeyConfiguration.render()` 가 시스템 폰트와 `ShapeRenderer` 로 직접 그리고, 그 위에 스킨 객체가 덧그려진다. 기본 keyconfig 스킨과 ModernChic keyconfig 스킨은 destination 이 비어 있다. | `config/KeyConfiguration.java:33`, `130-316`, `MainController.java:407-413`, `skin/default/keyconfig/keyconfigmain.lua:28-46`, `ModernChic/keyconfig.lua:20-31` |
| 11 | 스킨 설정 화면은 완전히 스킨 기반이다. 이벤트 `190`(스킨 변경), `220~228`(커스텀 항목 변경, `229` 는 코드 버그로 제외), `170~185`·`386~388`(스킨 종류 선택), 문자열 `50/51/100~109/110~119`, 슬라이더 type `7`, `skinpreview` 객체가 계약이다. | `config/SkinConfiguration.java:114-149`, `skin/SkinPropertyMapper.java:101-130`, `skin/property/StringPropertyFactory.java:83-84`, `242-257`, `skin/property/FloatPropertyFactory.java:236-242` |
| 12 | 시스템 사운드는 스킨이 아니라 설정의 BGM/사운드 폴더 세트에서 온다. 선택 화면 `create()` 마다 세트를 무작위로 다시 고른다. | `SystemSoundManager.java:43-79`, `104-113`, `select/MusicSelector.java:157` |

---

## 1. 모든 화면 공통

### 1.1 MainState 가 제공하는 것

| 멤버 | 내용 | 근거 |
|------|------|------|
| `main`, `type`, `timer`, `resource` | 컨트롤러, 화면 종류, 공유 `TimerManager`, 공유 `PlayerResource` | `MainState.java:23-45` |
| `MainStateType` | `MUSICSELECT, DECIDE, PLAY, RESULT, COURSERESULT, CONFIG, SKINCONFIG` | `MainState.java:174-182` |
| 수명주기 훅 | `create()`(추상), `prepare()`, `shutdown()`, `render()`(추상), `input()`, `pause()`, `resume()`, `resize()`, `dispose()` | `MainState.java:47-80` |
| `setSkin(skin)` | 이전 스킨 dispose 후 교체. 새 스킨의 `getOffset()` 항목을 `main.getOffset(id)` 의 x,y,w,h,r,a 로 복사 | `MainState.java:116-135` |
| `loadSkin(SkinType)` | `setSkin(SkinLoader.load(this, type))` | `MainState.java:137-139` |
| `executeEvent(int id, arg1, arg2)` | 커스텀 이벤트 id 일 때만 `skin.executeCustomEvent` 호출. 그 외 id 는 기본 구현에서 무시 | `MainState.java:90-94` |
| `executeEvent(EventType e, arg1, arg2)` | 내장 이벤트 실행 | `MainState.java:104-106` |
| `getScoreDataProperty()` | 화면별 스코어 파생값 보관소 | `MainState.java:38`, `108-110` |
| `getJudgeCount(judge, fast)` | 기본은 `ScoreDataProperty` 의 스코어에서. 결과/코스 결과가 재정의 | `MainState.java:141-144`, `result/MusicResult.java:453-456`, `result/CourseResult.java:279-282` |
| `getOffsetValue(id)` | `main.getOffset(id)` (0..`OFFSET_MAX`=199) | `MainState.java:146-148`, `MainController.java:124-125`, `skin/SkinProperty.java:972` |
| `getStage()/setStage()` | scene2d Stage(텍스트 입력용). 있으면 입력 멀티플렉서 앞단에 붙는다 | `MainState.java:150-156`, `MainController.java:285-289` |
| `getSound/play/stop(SoundType)` | 시스템 사운드 위임 | `MainState.java:158-172` |

### 1.2 화면 전환(`MainController.changeState`)

`MainController.java:248-290` 의 실제 순서:

1. 대상 상태 객체 선택. `PLAY` 만 매번 새 `BMSPlayer` 를 만들고(이전 것은 `dispose()`), 나머지 6개 화면 객체는 `create()` 에서 한 번 만든 것을 재사용한다(`MainController.java:331-340`).
2. `newState != null && current != newState` 이면
   1. `current.shutdown()`
   2. `current.setSkin(null)` — 이전 스킨의 모든 객체·리소스 dispose (`Skin.java:422-441`)
   3. `newState.create()` — 여기서 각 화면이 `loadSkin(...)` 을 부른다(동기 로드, 로딩 화면 없음)
   4. `newState.getSkin().prepare(newState)` — 정적으로 그려지지 않을 객체 제거, `obj.load()` 호출 (`Skin.java:199-267`)
   5. `current = newState`
   6. `timer.setMainState(newState)` — 타이머 배열 전체를 `Long.MIN_VALUE`(OFF)로 채우고 `starttime = System.nanoTime()` (`TimerManager.java:101-107`)
   7. `current.prepare()`
   8. 상태 리스너 통지(Discord RPC 등)
3. 입력 프로세서 재설정: Stage 가 있으면 `InputMultiplexer(stage, keyboard)`, 없으면 키보드만.

주의:
- `MUSICSELECT` 로 가려는데 `bmsfile != null`(단일 곡 실행 모드)이면 `exit()` 를 호출한다(`MainController.java:250-255`).
- 스킨은 화면을 떠날 때마다 해제되고 들어올 때마다 다시 로드된다. 캐시는 픽스맵 풀(`SkinLoader.resource`, 세대 수 `config.skinPixmapGen` 기본 4)뿐이다(`skin/SkinLoader.java:26-33`, `65`, `Config.java:173`).
- 스킨 경로 해석: `playerConfig.getSkin()[skinType.getId()]` 가 유효하면 그것, 로드 실패 시 `SkinConfig.Default` 경로로 재시도(`skin/SkinLoader.java:42-54`). 확장자 `.json` → `JSONSkinLoader`, `.luaskin` → `LuaSkinLoader`, 그 외 → LR2 (`skin/SkinLoader.java:62-81`).
- 기본 스킨 경로: `SELECT=skin/default/select.json`, `DECIDE=skin/default/decide/decide.luaskin`, `RESULT=skin/default/result/result.luaskin`, `COURSERESULT=skin/default/graderesult.json`, `KEYCONFIG=skin/default/keyconfig/keyconfig.luaskin`, `SKINSELECT=skin/default/skinselect/skinselect.luaskin` (`SkinConfig.java:168-182`).

`SkinType` id (`skin/SkinType.java:12-30`): `PLAY_7KEYS 0, PLAY_5KEYS 1, PLAY_14KEYS 2, PLAY_10KEYS 3, PLAY_9KEYS 4, MUSIC_SELECT 5, DECIDE 6, RESULT 7, KEY_CONFIG 8, SKIN_SELECT 9, SOUND_SET 10, THEME 11, PLAY_7KEYS_BATTLE 12, PLAY_5KEYS_BATTLE 13, PLAY_9KEYS_BATTLE 14, COURSE_RESULT 15, PLAY_24KEYS 16, PLAY_24KEYS_DOUBLE 17, PLAY_24KEYS_BATTLE 18`.

타입별 객체 로더(`skin/json/JSONSkinLoader.java:275-284`): `MUSIC_SELECT → JsonSelectSkinObjectLoader`, `DECIDE → JsonDecideSkinObjectLoader`, `RESULT → JsonResultSkinObjectLoader`, `COURSE_RESULT → JsonCourseResultSkinObjectLoader`, `SKIN_SELECT → JsonSkinConfigurationSkinObjectLoader`, `KEY_CONFIG` 및 기타 → `JsonKeyConfigurationSkinObjectLoader`. 결정·결과·코스 결과·키 설정 로더는 전용 객체가 없고 스킨 클래스만 다르다(`JsonDecideSkinObjectLoader.java:12-15`, `JsonResultSkinObjectLoader.java:12-15`, `JsonCourseResultSkinObjectLoader.java:12-15`, `JsonKeyConfigurationSkinObjectLoader.java:12-15`). Lua 스킨도 Lua 테이블을 `JsonSkin.Skin` 으로 변환한 뒤 같은 `loadJsonSkin` 을 탄다(`skin/lua/LuaSkinLoader.java:68-95`).

### 1.3 렌더 루프와 입력 분배

`MainController.render()` (`MainController.java:401-588`):

| 순서 | 동작 | 줄 |
|------|------|----|
| 1 | `timer.update()` — `nowmicrotime = (nanoTime - starttime)/1000` | `403`, `TimerManager.java:109-111` |
| 2 | 화면 클리어 | `405` |
| 3 | `current.render()` — 화면 로직(타이머 켜기, 전환 판정) | `407` |
| 4 | `sprite.begin()` → `skin.updateCustomObjects(current)`(커스텀 타이머 → 커스텀 이벤트 순) → `skin.drawAllObjects(sprite, current)` → `sprite.end()` | `408-413`, `Skin.java:782-789` |
| 5 | Stage `act(min(delta, 1/30))` + `draw()` | `415-419` |
| 6 | FPS, 메시지 렌더러(좌표 x=100, y=해상도 높이-2) | `422-488` |
| 7 | `System.currentTimeMillis()` 가 직전 처리 시각보다 클 때만(1ms 당 1회): `current.input()` → 마우스 눌림이면 `skin.mousePressed(state, button, x, y)` → 드래그면 `skin.mouseDragged(...)` → 커서 숨김 판정 → 전역 단축키 | `495-587` |

전역 단축키(`input/BMSPlayerInputProcessor.java:391-424`): `F1` FPS 표시, `F2` 폴더 갱신, `F3` 탐색기 열기(Ctrl+F3 MD5 복사, Ctrl+Shift+F3 SHA256 복사), `F4` 전체화면 전환, `F6` 스크린샷, `F7` 트위터, `F8` 즐겨찾기 곡, `F9` 즐겨찾기 차트, `F10` 폴더 오토플레이, `F11` IR 열기, `F12` 스킨 설정.

`Skin.drawAllObjects` (`Skin.java:276-331`): `nextpreparetime <= nowMicroTime` 일 때만 모든 객체 `prepare(time=timer.getNowTime(), state)` 를 부르고(주기 `1000000/prepareFramePerSecond` us, 설정 0 이면 1us 즉 매 프레임 — `Skin.java:265`, `Config.java:75`), `obj.draw == true` 인 객체만 등록 순서대로 그린다.

마우스(`Skin.java:394-420`):
- `mousePressed`: 객체 배열을 역순(가장 위부터)으로 돌며, 편집 가능한 `SkinText` 가 클릭 좌표를 포함하면 텍스트 입력 포커스를 주고 중단. 아니면 `obj.draw && obj.mousePressed(...)` 가 true 인 첫 객체에서 중단.
- `mouseDragged`: `SkinSlider` 만 대상(역순, 첫 처리에서 중단).
- 기본 `SkinObject.mousePressed` 는 `clickevent` 가 있을 때 `region` 안 클릭이면 이벤트 실행(`skin/SkinObject.java:671-704`). 버튼→인자 표 `buttonEvents = {1,-1,1,1,-1}`. `click` 종류: `0` = `inc`, `1` = `-inc`, `2` = 영역 좌/우 반으로 `-1/+1`, `3` = 아래/위 반으로 `-1/+1`.
- `mouseRect` 가 있는 객체는 마우스가 `region` 기준 상대 사각형 밖이면 `draw=false`(호버 표시용) (`skin/SkinObject.java:603-607`).

### 1.4 TimerManager

| 항목 | 내용 | 근거 |
|------|------|------|
| 타이머 수 | `TIMER_MAX + 1 = 3000` 개 (`id 0..2999`) | `TimerManager.java:20-21`, `skin/SkinProperty.java:172` |
| OFF 값 | `Long.MIN_VALUE` | `TimerManager.java:67-69`, `75-77` |
| 값 의미 | 켠 시점의 `nowmicrotime`(us). 경과 = `now - timer` | `TimerManager.java:37-53`, `71-73` |
| `getNowTime(id)` | 켜져 있으면 경과 ms, 꺼져 있으면 0 | `TimerManager.java:37-42` |
| `switchTimer(id, on)` | on: 꺼져 있을 때만 현재 시각으로 켬(이미 켜져 있으면 유지). off: 끔 | `TimerManager.java:87-95` |
| `setTimerOn(id)` | 무조건 현재 시각으로 재설정(재시작) | `TimerManager.java:71-73` |
| 범위 밖 id | `id < 0 || id >= 3000` 이면 현재 스킨의 커스텀 타이머로 위임(`TIMER_CUSTOM_BEGIN=10000 ~ END=19999`) | `TimerManager.java:59-65`, `79-85`, `skin/SkinProperty.java:175-176` |
| 리셋 | `setMainState` 에서 전체 OFF + 기준 시각 리셋 | `TimerManager.java:101-107` |

### 1.5 Skin 의 input / scene / fadeout

| 필드 | 의미 | 기본값 | 근거 |
|------|------|--------|------|
| `input` | 입력 수용 시작 시각(ms). `nowTime > input` 이면 각 화면이 `TIMER_STARTINPUT(1)` 을 켠다 | 0 | `Skin.java:79-81`, `JsonSkin.java:15` |
| `scene` | 화면 지속 시간(ms). 초과 시 자동 FADEOUT | 클래스 기본 `3600000*24`, 로더가 `sk.scene`(생략 시 0)로 덮어씀 | `Skin.java:83-85`, `JsonSkin.java:16`, `JSONSkinLoader.java:307` |
| `fadeout` | `TIMER_FADEOUT(2)` 가 켜진 뒤 다음 화면으로 넘어가기까지의 시간(ms) | 0 | `Skin.java:87-89`, `JsonSkin.java:14` |

JSON 최상위의 `close`, `loadend`, `playstart`, `judgetimer`, `finishmargin` 은 플레이 스킨용이고 이 문서의 화면들은 읽지 않는다(`JsonSkin.java:17-21`, 본 조사 범위 화면 소스에 사용처 없음).

화면별 사용 여부:

| 화면 | `input` → STARTINPUT | `scene` → FADEOUT | `fadeout` → 전환 | 근거 |
|------|:---:|:---:|:---:|------|
| 선택 | 사용 | 미사용 | 미사용 | `select/MusicSelector.java:193-195` |
| 결정 | 사용 | 사용 | 사용 | `decide/MusicDecide.java:37-51` |
| 결과 | 사용 | 사용 | 사용 | `result/MusicResult.java:168-258` |
| 코스 결과 | 사용 | 사용 | 사용 | `result/CourseResult.java:175-191` |
| 키 설정 | 미사용 | 미사용 | 미사용 | `config/KeyConfiguration.java:130-316` |
| 스킨 설정 | 미사용 | 미사용 | 미사용 | `config/SkinConfiguration.java:61-76` |

예시 값: 기본 선택 `fadeout 500 / scene 3000 / input 500`(`skin/default/select.json:6-8`), 기본 결정 동일(`skin/default/decide.json:6-8`), 기본 결과 `fadeout 500 / scene 3600000 / input 500`(`skin/default/result.json:6-8`). ModernChic 결정 `fadeout 1000 / scene 3000 / input 500`(`ModernChic/Decide/lua/require/header.lua:12-14`), 결과 `scene 3600000 / input 2500 / fadeout 1000`(`ModernChic/Result/lua/require/header.lua:15-17`).

### 1.6 시스템 사운드

`SoundType` 과 파일명(`SystemSoundManager.java:133-155`):

| SoundType | 파일 | BGM 세트 여부 |
|-----------|------|:---:|
| `SCRATCH` | `scratch.wav` | 아니오 |
| `FOLDER_OPEN` | `f-open.wav` | 아니오 |
| `FOLDER_CLOSE` | `f-close.wav` | 아니오 |
| `OPTION_CHANGE` | `o-change.wav` | 아니오 |
| `OPTION_OPEN` | `o-open.wav` | 아니오 |
| `OPTION_CLOSE` | `o-close.wav` | 아니오 |
| `PLAY_READY` | `playready.wav` | 아니오 |
| `PLAY_STOP` | `playstop.wav` | 아니오 |
| `RESULT_CLEAR` | `clear.wav` | 아니오 |
| `RESULT_FAIL` | `fail.wav` | 아니오 |
| `RESULT_CLOSE` | `resultclose.wav` | 아니오 |
| `COURSE_CLEAR` | `course_clear.wav` | 아니오 |
| `COURSE_FAIL` | `course_fail.wav` | 아니오 |
| `COURSE_CLOSE` | `course_close.wav` | 아니오 |
| `GUIDESE_PG/GR/GD/BD/PR/MS` | `guide-pg.wav` 등 | 아니오 |
| `SELECT` | `select.wav` | 예 |
| `DECIDE` | `decide.wav` | 예 |

해석 규칙:
- 세트 탐색: `config.bgmpath` 아래에서 `select.*` 가 있는 디렉터리를 BGM 세트로, `config.soundpath` 아래에서 `clear.*` 가 있는 디렉터리를 사운드 세트로 재귀 수집(심볼릭 링크 제외) (`SystemSoundManager.java:43-53`, `89-102`).
- `shuffle()`: BGM 세트·사운드 세트를 각각 무작위 1개 선택하고, SoundType 마다 `[선택 세트]/파일` → `defaultsound/파일` 순으로 첫 존재 경로를 등록(`SystemSoundManager.java:55-79`, `104-113`).
- 확장자: 지정 파일이 없으면 `.wav, .flac, .ogg, .mp3` 순으로 같은 이름을 찾는다(`audio/AudioDriver.java:178-201`).
- 재생 음량은 `audioConfig.systemvolume` (`SystemSoundManager.java:119-124`).
- 참고: ModernChic `Sound/` 에는 `clear.ogg, fail.ogg, f-open.ogg, f-close.ogg, o-open.ogg, o-close.ogg, o-change.ogg, scratch.ogg, playready.ogg, playstop.ogg` 등이 있고 `select.*`·`decide.*` 는 없다(디렉터리 목록 확인). 즉 사운드 세트로는 인식되지만 BGM 세트는 아니다.

재생 지점 전체:

| 화면 | 시점 | 사운드 | 근거 |
|------|------|--------|------|
| 선택 | `create()` | `shuffle()` 후 `SELECT` 를 프리뷰 프로세서 기본곡으로 등록, `prepare()` 에서 루프 재생 시작 | `select/MusicSelector.java:157`, `173-174`, `187-189`, `select/PreviewMusicProcessor.java:83` |
| 선택 | 막대 이동 1칸마다 | `SCRATCH` | `select/BarRenderer.java:471-480` |
| 선택 | 폴더 열기 | `FOLDER_OPEN` | `select/MusicSelector.java:325-327`, `select/MusicSelectInputProcessor.java:310-312`, `select/MusicSelectCommand.java:136`, `141` |
| 선택 | 폴더 닫기 | `FOLDER_CLOSE` | `select/BarManager.java:500` |
| 선택 | 옵션 패널 열림(패널 키를 뗐다가 다시 누른 첫 프레임) | `OPTION_OPEN` | `select/MusicSelectInputProcessor.java:118-122`, `208-212`, `245-249` |
| 선택 | 옵션 패널 닫힘(패널 키를 모두 뗀 프레임) | `OPTION_CLOSE` | `select/MusicSelectInputProcessor.java:100-107` |
| 선택 | 옵션·정렬·모드·LN·리플레이 슬롯 변경 | `OPTION_CHANGE` | `skin/property/EventFactory.java:181`, `192`, `202`, `280`, `290` 등, `select/MusicSelectCommand.java:42`, `54`, `select/MusicSelectInputProcessor.java:215-239` |
| 선택 | 타깃 변경(패널1 + 스크래치/휠) | `SCRATCH` | `select/MusicSelectInputProcessor.java:194-203` |
| 선택 | `shutdown()` | 프리뷰 스레드 정지 → 기본 BGM 정지 | `select/MusicSelector.java:305-309`, `select/PreviewMusicProcessor.java:71-74`, `117`, `136` |
| 결정 | `prepare()` | `DECIDE` (루프 없음) | `decide/MusicDecide.java:32-35` |
| 결과 | `prepare()` | 클리어면 `RESULT_CLEAR`, 실패면 `RESULT_FAIL`. 루프 여부 `audioConfig.loopResultSound` | `result/MusicResult.java:149-151` |
| 결과 | FADEOUT 시작(입력 또는 scene 초과) | `RESULT_CLOSE` 가 등록돼 있으면 CLEAR/FAIL 정지 후 재생 | `result/MusicResult.java:250-257`, `291-296` |
| 결과 | `shutdown()` | `RESULT_CLEAR/FAIL/CLOSE` 정지 | `result/MusicResult.java:154-158` |
| 코스 결과 | `prepare()` | 클리어: `COURSE_CLEAR` 가 있으면 그것, 없으면 `RESULT_CLEAR`. 실패: `COURSE_FAIL` 또는 `RESULT_FAIL`. 루프 `loopCourseResultSound` | `result/CourseResult.java:159-160` |
| 코스 결과 | FADEOUT 시작 | `COURSE_CLOSE` 또는 `RESULT_CLOSE` 가 있으면 CLEAR/FAIL 정지 후 재생 | `result/CourseResult.java:184-190`, `216-222` |
| 코스 결과 | `shutdown()` | 위 3종 정지 | `result/CourseResult.java:163-167` |

결정 화면에는 `shutdown()` 재정의가 없어 `DECIDE` 사운드 정지 시점은 이 범위에서 확인하지 못했다(미확인, 플레이 화면 쪽 코드일 수 있다).

---

## 2. 선택 화면 — songlist(SkinBar) 객체

### 2.1 스키마(`JsonSkin.SongList`, `skin/json/JsonSkin.java:437-451`)

| 필드 | 타입 | 기본값 | 참조 대상 | 최대 개수 | 의미 |
|------|------|--------|-----------|-----------|------|
| `id` | string | — | 최상위 `destination` 의 id 와 일치해야 객체 생성 | — | `JsonSelectSkinObjectLoader.java:37` |
| `center` | int | 0 | — | — | 커서(선택) 슬롯 인덱스. `MusicSelectSkin.centerBar` |
| `clickable` | int[] | `[]` | — | — | 마우스 클릭을 받는 슬롯 인덱스 목록 |
| `listoff` | Destination[] | `[]` | `imageset` id | 60 (`SkinBar.BAR_COUNT`) | 비선택 상태 슬롯별 막대 dst |
| `liston` | Destination[] | `[]` | `imageset` id | 60 | 선택 상태 슬롯별 막대 dst |
| `text` | Destination[] | `[]` | `text` id | 11 (`BARTEXT_COUNT`) | 막대 제목 텍스트(종류별) |
| `level` | Destination[] | `[]` | `value` id | 7 (`BARLEVEL_COUNT`) | 난이도별 레벨 숫자 |
| `lamp` | Destination[] | `[]` | `image` id | 11 (`BARLAMP_COUNT`) | 클리어 램프 |
| `playerlamp` | Destination[] | `[]` | `image` id | 11 | 라이벌 비교 시 내 램프 |
| `rivallamp` | Destination[] | `[]` | `image` id | 11 | 라이벌 비교 시 라이벌 램프 |
| `trophy` | Destination[] | `[]` | `image` id | 3 (`BARTROPHY_COUNT`) | 코스 트로피 |
| `label` | Destination[] | `[]` | `image` id | 5 (`BARLABEL_COUNT`) | 차트 특성 라벨 |
| `graph` | Destination | null | `graph` id (type < 0) | 1 | 폴더 분포 그래프 |

상수 근거: `select/SkinBar.java:31`, `38`, `61`, `67`, `74`, `93`.

### 2.2 로더의 구성 규칙(`skin/json/JsonSelectSkinObjectLoader.java`)

1. `new SkinBar(0)` — position 0 (LR2 로더는 `SkinBar(1)` 로 y 에 막대 높이를 더한다. JSON/Lua 는 항상 0) (`38`, `skin/lr2/LR2SelectSkinLoader.java:24`, `select/BarRenderer.java:142`).
2. 막대 본체(`44-77`): 슬롯 `i` 마다 `liston[i].id` 와 같은 `imageset` 을 찾아, 그 `images[j]` 각각을 `image` 정의에서 찾아 `TextureRegion[j][]`(divx×divy 프레임)로 만든다. 타이머·cycle 은 처음 발견한 image 의 값을 쓴다. 같은 소스로 on/off `SkinImage` 두 개를 만들고 on 에는 `liston[i]`, off 에는 `listoff[i]` 의 dst 를 건다.
   - `listoff[i].id` 는 검사하지 않는다. 즉 off 도 `liston[i].id` 의 imageset 을 쓴다.
   - 루프 길이는 `liston.length` 다. `listoff` 가 더 짧으면 배열 범위를 넘는다(`40-41`, `72`).
   - id 가 일반 `image` 이면 매칭되지 않아 그 슬롯은 null 이 되고 그려지지 않는다.
3. `center`, `clickable` 을 스킨에 저장(`79-80`).
4. `lamp[i]`, `playerlamp[i]`, `rivallamp[i]`, `trophy[i]`, `label[i]` 는 `image` 정의만 검색한다(imageset 불가). 배열 인덱스 `i` 가 그대로 종류 인덱스다(`83-163`).
5. `text[i]` 는 `text` 정의를 검색해 `createText` 로 만든다(`166-177`).
6. `level[i]` 는 `value` 정의를 검색한다. 숫자 이미지 수가 10 의 배수면 `d=10`, 아니면 `d=11`. `SkinNumber(images, timer, cycle, digit, d>10 ? 2 : 0, space, ref, align)` (`180-203`).
7. `graph` 는 `graph` 정의 중 `type < 0` 인 것만. `type == -1` 이면 11분할(클리어 램프), 그 외(`-2`)면 28분할(스코어 랭크). 소스 이미지 분할은 `imgs[j][i] = images[i*len + j]` 로, 가로 `len` 칸이 종류, 세로가 애니메이션 프레임이다. 텍스처가 없으면 그래프는 생성되지 않는다(`206-230`).
8. 하위 객체는 `Skin.add` 되지 않으므로 스킨 `prepare` 의 정적 옵션 필터(§1.2-4)를 타지 않는다. 각자의 dst 타이머·loop·`draw` 조건은 `prepare` 때 평가된다(`select/SkinBar.java:257-317`).
9. `SkinBar` 자체는 생성자에서 `(time 0, x0 y0 w0 h0, a255)` dst 를 하나 갖는다(`select/SkinBar.java:103-106`). 최상위 `{"id":"songlist"}` destination 의 `op`/`timer`/`draw` 가 목록 전체의 표시 여부를 결정한다.

기본 스킨 예(`skin/default/select.json:150-151`, `219-222`): `{"id":"bar", "images":["bar-song","bar-folder","bar-table","bar-grade","bar-nograde","bar-command","bar-search","bar-nosong"]}`, `center:10`, `clickable:[2..19]`, 슬롯 22개. 인덱스 7(`bar-nosong`)은 코드가 값 7 을 만들지 않아 쓰이지 않는다.

ModernChic 예(`ModernChic/Select/lua/songlist.lua:6-51`): `center = 8`, `clickable = {6,7,8,9,10}`, 슬롯 17개, `liston` 은 정적 dst(`x=1125`), `listoff` 는 슬롯마다 `loop` 과 2 키프레임(`time = start, x=1920` → `time = loop, x = songlist_x[i]`, `acc = DECELERATE`)을 가진 등장 애니메이션. `level` 6개, `label` 5개(ln, random, bomb, cn, hcn), `text` 11개(종류별 x·색), `graph = {id="graph-lamp", dst={{x=40,y=5,w=650,h=10}}}`.

### 2.3 슬롯과 막대의 대응

- 슬롯 `i` 에 그려지는 막대 인덱스: `(selectedindex + currentsongs.length*100 + i - center) % currentsongs.length` (`select/BarRenderer.java:145-147`). 목록은 원형으로 반복된다(막대 수가 슬롯 수보다 적으면 같은 막대가 여러 슬롯에 나온다).
- 슬롯 `i` 의 이미지: `on = (i == center)` 이면 `liston[i]`, 아니면 `listoff[i]` (`select/BarRenderer.java:124-130`, `select/SkinBar.java:113-115`).
- 슬롯 이미지의 `draw` 가 false 면 그 슬롯은 `value = -1` 로 건너뛴다(`select/BarRenderer.java:131`, `167-169`).

### 2.4 막대 종류 → 이미지 인덱스(`ba.value`)

`select/BarRenderer.java:150-166`:

| `value` | 조건(위에서부터 판정) |
|:---:|------|
| 2 | `TableBar` 또는 `HashBar` 또는 `ExecutableBar` |
| 3 | `GradeBar` 이고 `existsAllSongs()` |
| 4 | `GradeBar` 이고 곡 누락 |
| 2 | `RandomCourseBar` 이고 `existsAllSongs()` |
| 4 | `RandomCourseBar` 이고 스테이지 누락 |
| 1 | `FolderBar` |
| 0 | `SongBar` 이고 `existsSong()` (`song.getPath() != null`) |
| 4 | `SongBar` 이고 곡 없음 |
| 6 | `SearchWordBar` |
| 5 | `CommandBar` 또는 `ContainerBar` |
| -1 | 그 외(예: `SameFolderBar`) — 그리지 않음 |

판정 순서 주의: `HashBar`·`TableBar` 검사가 먼저이고 `FolderBar` 는 그 뒤다. `SameFolderBar` 는 `DirectoryBar` 직계라 어느 분기에도 걸리지 않아 `-1` 이 된다(`select/bar/SameFolderBar.java:14`).

그릴 때 `si.draw(sprite, time, select, ba.value, dx, dy)` → `SkinImage.prepare(time, state, value, ...)`: `value < 0` 이면 그리지 않고, `value >= image.length` 이면 0 으로 바꾼다(`skin/SkinImage.java:127-141`, `170-175`).

### 2.5 막대 텍스트 종류(`ba.text`)

`select/SkinBar.java:41-61`, `select/BarRenderer.java:171-197`:

| 인덱스 | 상수 | 대상 |
|:---:|------|------|
| 0 | `BARTEXT_NORMAL` | 기본 |
| 1 | `BARTEXT_NEW` | 신규 |
| 2 | `BARTEXT_SONG_NORMAL` | SongBar(일반) |
| 3 | `BARTEXT_SONG_NEW` | SongBar(신규) |
| 4 | `BARTEXT_FOLDER_NORMAL` | FolderBar(일반) |
| 5 | `BARTEXT_FOLDER_NEW` | FolderBar(신규) |
| 6 | `BARTEXT_TABLE` | TableBar / HashBar (value 2) |
| 7 | `BARTEXT_GRADE` | GradeBar(곡 보유) (value 3) |
| 8 | `BARTEXT_NO_SONGS` | 곡 미보유 SongBar/GradeBar (value 4) |
| 9 | `BARTEXT_COMMAND` | CommandBar / ContainerBar (value 5) |
| 10 | `BARTEXT_SEARCH` | SearchWordBar (value 6) |

계산:
- `value >= 2` → `songstatus = value + 4`. 해당 텍스트가 정의돼 있지 않으면 0.
- `value == 0`(곡) → 곡 데이터가 null 이거나 `현재초 > adddate + 86400` 이면 2, 아니면 3(추가 24시간 이내 = 신규). 정의가 없으면 3→1, 2→0 으로 대체.
- `value == 1`(폴더) → 같은 규칙으로 4 또는 5, 대체는 5→1, 4→0.

주의: `value == 4`(곡 없음)인 SongBar 는 `songstatus = 8` 경로를 타므로 신규 판정을 하지 않는다. `value == 2` 인 `ExecutableBar`·`RandomCourseBar` 는 텍스트 6 을 쓴다.

텍스트 내용은 `Bar.getTitle()` 이다(`select/BarRenderer.java:289`).

| Bar | `getTitle()` | 근거 |
|-----|--------------|------|
| SongBar | `song.getFullTitle()` | `select/bar/SongBar.java:60-63` |
| FolderBar | `folder.getTitle()` | `select/bar/FolderBar.java:34-37` |
| TableBar | `td.getName()` | `select/bar/TableBar.java:44-47` |
| HashBar / CommandBar / ContainerBar / SameFolderBar / ExecutableBar | 생성 시 받은 제목 | 각 파일 `getTitle()` |
| GradeBar / RandomCourseBar | `course.getName()` | `select/bar/GradeBar.java:41-44`, `select/bar/RandomCourseBar.java:22-25` |
| SearchWordBar | `"Search : '" + text + "'"` | `select/bar/SearchWordBar.java:19` |
| 랜덤 선택 ExecutableBar | `"[RANDOM] " + name` | `select/bar/ExecutableBar.java:118-120` |

### 2.6 하위 객체 인덱스

| 하위 객체 | 인덱스 계산 | 근거 |
|-----------|-------------|------|
| `lamp[n]` | `Bar.getLamp(true)` = `ClearType.id` 0~10 (`NoPlay 0, Failed 1, AssistEasy 2, LightAssistEasy 3, Easy 4, Normal 5, Hard 6, ExHard 7, FullCombo 8, Perfect 9, Max 10`). 라이벌이 없을 때만 사용 | `select/BarRenderer.java:331-336`, `ClearType.java:10-20` |
| `playerlamp[n]`, `rivallamp[n]` | `select.getRival() != null` 일 때 `getLamp(true)` / `getLamp(false)` | `select/BarRenderer.java:322-330` |
| `level[n]` | `SongBar && existsSong()` 일 때만. `difficulty` 가 0~6 이면 그 값, 아니면 0. 숫자 값은 `song.getLevel()` | `select/BarRenderer.java:346-353` |
| `trophy[n]` | `GradeBar` 의 `getTrophy().getName()` 이 `"bronzemedal"`=0, `"silvermedal"`=1, `"goldmedal"`=2 | `select/BarRenderer.java:32`, `299-312` |
| `label[0]` | LN | `select/BarRenderer.java:374-398` |
| `label[1]` | RANDOM (`FEATURE_RANDOM = 4`) | `select/BarRenderer.java:403-406`, `song/SongData.java:26` |
| `label[2]` | MINE (`FEATURE_MINENOTE = 2`) | `select/BarRenderer.java:399-402`, `song/SongData.java:25` |
| `label[3]` | CN | `select/BarRenderer.java:391` |
| `label[4]` | HCN | `select/BarRenderer.java:391` |

Bar 종류별 램프 값:
- `SongBar`: 해당 스코어의 `clear`, 스코어 없으면 0 (`select/bar/SongBar.java:65-71`).
- `GradeBar`: 일반·미러·랜덤 스코어의 clear 최댓값. 라이벌은 미구현으로 같은 값(`select/bar/GradeBar.java:91-104`).
- `DirectoryBar` 계열: `lamps[0..10]` 중 개수가 0 보다 큰 가장 낮은 인덱스(= 폴더 안 최저 클리어). 모두 0 이면 0 (`select/bar/DirectoryBar.java:60-68`). 집계는 `config.folderlamp` 일 때만 백그라운드 로더가 수행(`select/BarManager.java:692-696`).
- `ExecutableBar`, `RandomCourseBar`: 항상 0 (`select/bar/ExecutableBar.java:108-111`, `select/bar/RandomCourseBar.java:43-45`).

LN 라벨 결정(`select/BarRenderer.java:362-398`):
1. `flag` = SongBar(곡 존재)면 `song.getFeature()`, 곡이 다 있는 GradeBar 면 구성 곡 feature 의 OR.
2. `ln = -1`. `FEATURE_UNDEFINEDLN(1)` 이면 `ln = playerConfig.lnmode`(0 LN, 1 CN, 2 HCN). `FEATURE_LONGNOTE(8)` 이면 `max(ln,0)`, `FEATURE_CHARGENOTE(16)` 이면 `max(ln,1)`, `FEATURE_HELLCHARGENOTE(32)` 이면 `max(ln,2)`.
3. `ln >= 0` 이면 `lnindex = {0,3,4}[ln]` 의 라벨을 그리고, 그 라벨이 없으면 `label[0]` 로 대체.
4. 이어서 MINE(`label[2]`), RANDOM(`label[1]`) 순으로 그린다.

### 2.7 그리기 순서와 좌표

`select/BarRenderer.java:249-407`. 각 단계가 60 슬롯 전체를 한 번씩 돈다(단계 단위 레이어링).

1. 막대 본체(`value` 인덱스 이미지, 스크롤 오프셋 적용). 그린 뒤 `region.x/y` 를 원래 값으로 되돌린다(`258-263`).
2. 분포 그래프 — `ba.sd instanceof DirectoryBar` 이고 `graph.draw` 일 때(`270-281`).
3. 제목 텍스트(`284-290`).
4. 트로피(`293-313`).
5. 램프(`316-337`).
6. 레벨 숫자(`340-354`).
7. 라벨(`356-407`).

하위 객체 dst 는 막대 기준 상대 좌표다. 모든 하위 객체는 `(ba.x, ba.y)` 를 오프셋으로 받아 그려진다(`select/BarRenderer.java:278`, `289`, `306`, `325`, `329`, `334`, `351`, `393`, `401`, `405`). 예: 기본 스킨 텍스트 `{"x":80,"y":6,"w":18,"h":24}`(`skin/default/select.json:359-365`), ModernChic 텍스트 `{x = 130 또는 50, y = 15, w = 500, h = 35}`(`ModernChic/Select/lua/songlist.lua` 텍스트 블록).

### 2.8 스크롤 애니메이션 보간

상태 변수: `duration`(애니메이션 종료 벽시계 ms, 0 이면 없음), `angle`(부호 = 방향, 절댓값 = 1칸 이동 시간 ms) (`select/BarRenderer.java:36-44`).

`prepare()` (`select/BarRenderer.java:107-142`):

```
timeMillis    = System.currentTimeMillis()
applyMovement = duration != 0 && duration > timeMillis
angleLerp     = angle < 0 ? (timeMillis - duration) / angle
                          : (duration - timeMillis) / angle        (float)

슬롯 i 마다 (si1 = 슬롯 i 의 on/off 이미지, si1.draw 일 때):
  dx = dy = 0
  if applyMovement:
     nextindex = i + (angle >= 0 ? 1 : -1)
     si2 = nextindex >= 0 ? getBarImages(nextindex == center, nextindex) : null
     if si2 != null && si2.draw:
        dx = (si2.region.x - si1.region.x) * clamp(angleLerp, -1, 1)
        dy = (si2.region.y - si1.region.y) * angleLerp
  ba.x = (int)(si1.region.x + dx)
  ba.y = (int)(si1.region.y + dy + (position == 1 ? si1.region.height : 0))
```

해석:
- `angleLerp` 는 이동 직후 1 에서 시작해 0 으로 선형 감소한다. 선택 인덱스는 입력 즉시 바뀌고, 각 슬롯의 내용이 "이웃 슬롯 위치 → 자기 슬롯 위치"로 미끄러진다.
- `angle >= 0`(다음 곡으로 이동, `selectedindex++`)이면 이웃은 `i+1`, `angle < 0` 이면 `i-1`.
- 보간 대상 좌표는 그 시점의 이웃 슬롯 dst(자체 애니메이션이 진행 중이면 그 순간 값)다. 이웃이 center 면 `liston` 좌표를 쓴다.
- 휠로 2칸 누적된 경우 `angleLerp` 가 2 에서 시작한다. x 는 1 로 잘리지만 y 는 잘리지 않아 이웃 간격의 2배 위치에서 시작한다.
- 가장자리 슬롯(이웃 없음, 또는 이웃이 정의 안 됨/`draw` false)은 보간 없이 제자리에 그려진다. `getBarImages` 는 인덱스가 배열 길이 이상이면 null 을 준다(`select/SkinBar.java:113-115`).
- 좌표는 `int` 로 절삭된다.
- 가속/감속 없음(선형). 스킨 타이머와 무관하다.

### 2.9 입력에 따른 이동(`BarRenderer.input`, `select/BarRenderer.java:410-481`)

패널이 열려 있지 않을 때만 호출된다(`select/MusicSelectInputProcessor.java:289-290`). 패널이 열리면 대신 `bar.resetInput()` 이 불려 `now > duration` 이면 `duration = 0` 으로 정리된다(`select/BarRenderer.java:483-488`).

마우스 휠·아날로그(`415-435`):

```
mov = -input.getScroll()                     // getScroll() = (int)(-scrollY)
analogScrollBuffer += analog(UP) - analog(DOWN)
mov += analogScrollBuffer / analogTicksPerScroll   // 기본 3
analogScrollBuffer %= analogTicksPerScroll
if mov != 0:
   l = now
   remaining = angle == 0 ? 0 : (int)max(0, duration - l) / angle
   remaining = clamp(remaining + mov, -2, 2)
   if remaining == 0: angle = 0; duration = l
   else:
      scrollDuration = 120 / remaining / remaining   // ±1 → 120ms, ±2 → 30ms (정수 나눗셈)
      angle    = scrollDuration / remaining          // ±120 또는 ±15
      duration = l + scrollDuration
```

키 유지(`437-470`), 기본값 `scrolldurationlow = 300`, `scrolldurationhigh = 50` (`Config.java:87`, `91`):

| 조건 | 동작 |
|------|------|
| `MusicSelectKey.UP` 비아날로그 눌림 또는 키보드 `DOWN` 유지 | `duration == 0` 이면 `keyinput = true; mov = 1; duration = l + 300; angle = 300`. 이후 `l > duration && keyinput` 이면 `duration = l + 50; mov = 1; angle = 50` (반복) |
| `MusicSelectKey.DOWN` 비아날로그 눌림 또는 키보드 `UP` 유지 | 위와 대칭. `mov = -1`, `angle = -300` → `-50` |
| 둘 다 아님 | `keyinput = false` |
| 공통 후처리 | `l > duration && !keyinput` 이면 `duration = 0` |

이동 적용(`471-480`): `mov > 0` 인 동안 `manager.move(true)`(`selectedindex++`) + `SCRATCH`, `mov < 0` 인 동안 `manager.move(false)`(`selectedindex += len-1`) + `SCRATCH`. `move` 는 모듈러 순환이고 `ScoreDataProperty` 를 선택 막대의 스코어·라이벌 스코어로 갱신한다(`select/BarManager.java:538-547`).

이름이 뒤집혀 보이지만 실제 동작은 "키보드 아래 화살표 = 다음 곡(인덱스 증가)"이다.

### 2.10 마우스 클릭(`BarRenderer.mousePressed`, `select/BarRenderer.java:80-103`)

- `clickable` 의 각 슬롯 `i` 에 대해 `getBarImages(i == center, i)` 의 현재 dst 사각형(`getDestination(nowTime, state)`, 스크롤 오프셋 미적용)에 클릭 좌표가 들어오면 처리한다.
- 왼쪽 버튼(`button == 0`): `select.select(sd)` — `sd` 가 `DirectoryBar` 면 `updateBar(dirbar)` 성공 시 `FOLDER_OPEN` + `RESET_REPLAY`. 그 외에는 `play = BMSPlayerMode.PLAY` 만 설정한다(`select/MusicSelector.java:323-332`). 이후 `render()` 가 `manager.getSelected()`(커서 막대)를 대상으로 처리하므로(`select/MusicSelector.java:192`, `252-290`), 커서가 아닌 곡 막대를 클릭해도 커서에 있는 막대가 시작된다.
- 그 외 버튼: `manager.close()`(상위 폴더로. 루트에서는 `sort` 이벤트 실행) (`select/BarManager.java:490-501`).
- 드래그는 막대에 영향이 없다(슬라이더 전용, §1.3). 목록 위치 슬라이더는 FloatProperty type `1`(`musicselect_position`): 읽기 `selectedindex / currentsongs.length`, 쓰기 `selectedBarMoved()` 후 `selectedindex = (int)(len * value)` (`skin/property/FloatPropertyFactory.java:224-232`, `select/BarManager.java:526-536`).

### 2.11 분포 그래프(`SkinDistributionGraph`)

`select/SkinDistributionGraph.java`:

- `type 0`(JSON `type:-1`): 클리어 램프 11칸. `type 1`(JSON `type:-2`): 스코어 랭크 28칸(`25-30`, `80-94`).
- `prepare`: `config.folderlamp` 가 꺼져 있으면 그리지 않는다(`112-123`).
- 그리기(`129-154`): `lamps = bar.getLamps()`(11), `ranks = bar.getRanks()`(28), `count = sum(lamps)`. `count != 0` 일 때
  - type 0: `i = 10..0` 순서로 왼쪽부터 `x = region.x + acc * region.width / count + offsetx`, 폭 `lamps[i] * region.width / count`, 높이 `region.height`. 즉 MAX 가 가장 왼쪽, NoPlay 가 가장 오른쪽.
  - type 1: `i = 27..0` 순서로 같은 방식(`ranks[i]`). 분모는 여전히 `sum(lamps)`.
- 랭크 구간: `rank = exscore * 27 / (notes * 2)`, 27 초과는 27. 스코어 없음/노트 0 은 0 (`select/bar/DirectoryBar.java:134-146`).
- 이미지가 없는 생성자용 기본 색(ARGB 표기 문자열을 `Color.valueOf` 로 해석): `LAMP = {ff404040, ff000080, ff800080, ffff00ff, ff40ff40, ff00c0f0, ffffffff, ff88ffff, ffffff88, ff8888ff, ff0000ff}`, `RANK` 28색(`39-45`). JSON 로더는 텍스처가 있을 때만 그래프를 만들므로(§2.2-7) 이 기본 색 경로는 JSON/Lua songlist 에서는 쓰이지 않는다.
- 최상위 destination 으로 단독 배치한 그래프(`skin/json/JsonSkinObjectLoader.java:436`)는 현재 선택 막대가 `DirectoryBar` 일 때 그 분포를 그린다(`select/SkinDistributionGraph.java:113-114`, `125-127`).

---

## 3. 선택 화면 — 상태, 타이머, 입력, 패널

### 3.1 수명주기

| 단계 | 동작 | 근거 |
|------|------|------|
| 생성자 | 스코어 캐시, `BarRenderer`(→ `manager.init()`: 난이도표·코스·즐겨찾기·커맨드 폴더·랜덤 폴더 로드), 배너/스테이지파일 픽스맵 풀, 입력 처리기 | `select/MusicSelector.java:105-133`, `select/BarRenderer.java:67-78`, `select/BarManager.java:95-263` |
| `create()` | `shuffle()` → `play = null`, `showNoteGraph = false` → 플레이어 데이터 재로드 → 직전 플레이 곡/코스의 스코어 캐시 갱신 → 프리뷰 프로세서 생성 + 기본곡 `SELECT` → 선택 화면 입력 모드(`musicselectinput` 0=7키, 1=9키, 그 외=14키)의 키 설정 적용 → `manager.updateBar()` → `loadSkin(MUSIC_SELECT)` | `select/MusicSelector.java:156-185` |
| `prepare()` | `preview.start(null)` (선택 BGM 루프 재생) | `select/MusicSelector.java:187-189` |
| `render()` | §3.2 | `select/MusicSelector.java:191-291` |
| `input()` | `NUM6` 유지 → 키 설정 화면, `F12` → 스킨 설정 화면, 이어서 `musicinput.input()` | `select/MusicSelector.java:293-303` |
| `shutdown()` | 프리뷰 정지, 배너·스테이지파일 풀 세대 정리 | `select/MusicSelector.java:305-309` |

### 3.2 `render()` 가 매 프레임 하는 일

`select/MusicSelector.java:191-291`:

1. `timer.getNowTime() > skin.getInput()` 이면 `switchTimer(TIMER_STARTINPUT, true)`.
2. `timer.getNowTime(TIMER_SONGBAR_CHANGE) < 0` 이면 `setTimerOn(TIMER_SONGBAR_CHANGE)` (타이머가 미래 시각일 때의 방어 코드).
3. `resource.setSongdata(선택이 SongBar 면 그 곡, 아니면 null)`, `resource.setCourseData(선택이 GradeBar 면 그 코스, 아니면 null)` — 스킨의 곡 정보 프로퍼티가 이 값을 읽는다.
4. 프리뷰: 선택이 SongBar 이고 설정이 `NONE` 이 아니며 `nowTime > SONGBAR_CHANGE 시각 + 400ms` 이고 `play == null` 이면 `preview.start(song)`.
5. 노트 그래프용 BMS 로드: `nowTime > SONGBAR_CHANGE 시각 + 350ms` 이고 아직 안 했으면 별도 스레드에서 `song.setBMSModel(...)`.
6. IR 랭킹 로드: `currentRankingDuration != -1` 이고 `nowTime > SONGBAR_CHANGE 시각 + currentRankingDuration` 이면 랭킹 요청. 대기 시간은 `rankingDuration 5000ms` + (캐시가 있으면 `max(10분 - 마지막 갱신 후 경과, 0)`) (`select/MusicSelector.java:76-79`, `619-633`).
7. `irstate = currentir != null ? currentir.getState() : -1` 로 `switchTimer(TIMER_IR_CONNECT_BEGIN, irstate == ACCESS(1))`, `switchTimer(TIMER_IR_CONNECT_SUCCESS, irstate == FINISH(2))`, `switchTimer(TIMER_IR_CONNECT_FAIL, irstate == FAIL(3))` (`ir/RankingData.java:61-64`).
8. `play != null` 이면 선택 막대 종류에 따라 시작:
   - SongBar(곡 존재) → `readChart` → 성공 시 `changeState(DECIDE)`, 실패 시 메시지 `"Failed to loading BMS : Song not found, or Song has error"`(1200ms, 빨강).
   - SongBar(곡 없음) → IPFS 다운로드 가능하면 다운로드, 아니면 `open_download_site` 이벤트.
   - ExecutableBar → 무작위 곡으로 `readChart`.
   - GradeBar / RandomCourseBar → PRACTICE 는 PLAY 로 바꾼 뒤 코스 로드 → `changeState(DECIDE)`.
   - DirectoryBar + AUTOPLAY → 폴더 내 곡 연속 오토플레이 → `changeState(DECIDE)`.
   - 마지막에 `play = null`.

### 3.3 타이머 전부

| 타이머 | id | 켜지는 시점 | 꺼지는 시점 | 근거 |
|--------|:--:|-------------|-------------|------|
| `TIMER_STARTINPUT` | 1 | `nowTime > skin.input` 인 첫 프레임 | 화면 전환 리셋 | `select/MusicSelector.java:193-195` |
| `TIMER_SONGBAR_CHANGE` | 11 | (a) `selectedBarMoved()` 마다 재시작. (b) 매 `input()` 끝에 `switchTimer(…, true)` — 꺼져 있을 때만 켬 | 화면 전환 리셋 | `select/MusicSelector.java:612`, `select/MusicSelectInputProcessor.java:352` |
| `TIMER_PANEL1_ON`~`3_ON` | 21~23 | `setPanelState(n)` 으로 패널 n 이 새로 열릴 때 재시작. 동시에 `PANELn_OFF` 를 끔 | 패널 n 이 닫힐 때 끔 | `select/MusicSelector.java:553-565` |
| `TIMER_PANEL1_OFF`~`3_OFF` | 31~33 | 패널 n 이 닫힐 때(다른 패널로 바뀔 때 포함) 재시작. 동시에 `PANELn_ON` 을 끔 | 패널 n 이 다시 열릴 때 끔 | 같은 곳 |
| `TIMER_IR_CONNECT_BEGIN` | 172 | 랭킹 상태가 ACCESS 인 동안 ON | 상태가 바뀌면 OFF | `select/MusicSelector.java:247-250` |
| `TIMER_IR_CONNECT_SUCCESS` | 173 | 랭킹 상태가 FINISH 인 동안 ON | 〃 | 〃 |
| `TIMER_IR_CONNECT_FAIL` | 174 | 랭킹 상태가 FAIL 인 동안 ON | 〃 | 〃 |

`selectedBarMoved()` 호출 지점(`select/MusicSelector.java:608-634`): `BarManager.updateBar` 성공 시(`select/BarManager.java:463`), 매 `input()` 에서 선택 막대 객체가 바뀌었을 때(`select/MusicSelectInputProcessor.java:348-351`), 목록 위치 슬라이더 쓰기(`skin/property/FloatPropertyFactory.java:229`). 수행 내용: `RESET_REPLAY`, 배너·스테이지파일 반영, `TIMER_SONGBAR_CHANGE` 재시작, 프리뷰가 다른 폴더 곡이면 기본 BGM 으로 복귀, `showNoteGraph = false`, IR 대기 시간 재계산.

`create()` 안의 `manager.updateBar()` 가 켠 `TIMER_SONGBAR_CHANGE` 는 직후 `setMainState` 리셋으로 사라지고, 첫 `input()` 의 `switchTimer` 로 다시 켜진다(§0-1).

켜지지 않는 타이머(§0-7): `TIMER_SONGBAR_MOVE 10`, `TIMER_SONGBAR_MOVE_UP 12`, `TIMER_SONGBAR_MOVE_DOWN 13`, `TIMER_SONGBAR_STOP 14`, `TIMER_README_BEGIN 15`, `TIMER_README_END 16`, `TIMER_PANEL4_ON~6_ON 24~26`, `TIMER_PANEL4_OFF~6_OFF 34~36`. `TIMER_FADEOUT 2` 도 선택 화면에서는 켜지 않는다. README 표시는 구현돼 있지 않고 `open_document`(이벤트 17)가 곡 폴더의 `.txt` 를 OS 기본 뷰어로 연다(`skin/property/EventFactory.java:254-271`).

### 3.4 패널(옵션 패널 1~3)

`panelstate` 는 0(없음)~3. 불리언 옵션 `OPTION_PANEL1/2/3 = 21/22/23` 이 `panelstate == n` 을 반환한다(`skin/SkinProperty.java:647-649`, `skin/property/BooleanPropertyFactory.java:494-499`). 패널은 키를 누르고 있는 동안만 열려 있고, 떼면 닫힌다(매 프레임 `setPanelState` 호출).

| 패널 | 여는 조건 | 내용 | 근거 |
|:---:|-----------|------|------|
| 1 | START 유지, SELECT 미유지 | 플레이 옵션(랜덤, 게이지, DP 옵션, HS 고정, 타깃) | `select/MusicSelectInputProcessor.java:114-203` |
| 2 | SELECT 유지, START 미유지 | 어시스트 옵션 | `204-240` |
| 3 | 키보드 `NUM5` 유지 또는 START+SELECT 동시 유지 | 상세 옵션(BGA, 게이지 자동 전환, 표시 타이밍, 표시 시간) | `241-288` |
| 0 | 위 어느 것도 아님 | 목록 조작 | `289-346` |

기본 스킨의 패널 표현 예(`skin/default/select.json:798-853`): `{"id":"option-panel1", "loop":300, "op":[21], "timer":21, "dst":[{"time":0,…,"a":0},{"time":300,"a":255}]}` — 옵션 21 로 표시 여부, 타이머 21 로 페이드인.

### 3.5 키 매핑

키 인덱스는 선택 화면 입력 모드의 키 배열 인덱스다(7키 모드: 0~6 = 1~7번 키, 7 = F-SCR, 8 = R-SCR — `config/KeyConfiguration.java:43`, `60`). `BEAT_7K` 배정(`select/MusicSelectKeyProperty.java:8-18`, `BEAT_14K` 는 2P 쪽에 같은 배정 반복 `30-49`, `POPN_9K` 는 `19-29`):

| 키 인덱스 | 패널 0(목록) | 패널 1(START) | 패널 2(SELECT) | 패널 3(NUM5 / START+SELECT) |
|:---:|------|------|------|------|
| 0 (1번) | `PLAY`, `FOLDER_OPEN` | `OPTION1_DOWN` → `option1p +1` | `JUDGEWINDOW_UP` → customJudge 토글 | `BGA_DOWN` → `bga` |
| 1 (2번) | `FOLDER_CLOSE` | `OPTION1_UP` → `option1p -1` | `CONSTANT` → scrollMode 0/1 토글 | `GAUGEAUTOSHIFT_DOWN` → `gaugeautoshift` |
| 2 (3번) | `PRACTICE`, `FOLDER_OPEN` | `GAUGE_DOWN` → `gauge1p +1` | `JUDGEAREA` → showjudgearea 토글 | `NOTESDISPLAYTIMING_AUTOADJUST` |
| 3 (4번) | `FOLDER_CLOSE` | `OPTIONDP_DOWN` → `optiondp +1` | `LEGACYNOTE` → longnoteMode 0/1 토글 | `DURATION_DOWN`(유지 반복) |
| 4 (5번) | `FOLDER_OPEN`, `AUTO` | `HSFIX_DOWN` → `hsfix +1` | `MARKNOTE` → markprocessednote 토글 | `NOTESDISPLAYTIMING_DOWN` → `-1` |
| 5 (6번) | `NEXT_REPLAY` | `OPTION2_UP` → `option2p -1` | `BPMGUIDE` → bpmguide 토글 | `DURATION_UP`(유지 반복) |
| 6 (7번) | `FOLDER_OPEN`, `REPLAY` | `OPTION2_DOWN` → `option2p +1` | `NOMINE` → mineMode 0/1 토글 | `NOTESDISPLAYTIMING_UP` → `+` |
| 7 (스크래치) | `UP` → 다음 곡 | `TARGET_UP` → `target -1` | — | — |
| 8 (스크래치) | `DOWN` → 이전 곡 | `TARGET_DOWN` → `target +1` | — | — |

패널 0 에서의 판정(`select/MusicSelectInputProcessor.java:289-346`):
- 선택이 `SelectableBar`: `PLAY` 또는 키보드 `RIGHT`/`ENTER` → `selectSong(PLAY)`. `PRACTICE` → PRACTICE(이벤트 모드면 PLAY). `AUTO` → AUTOPLAY(이벤트 모드면 PLAY). `REPLAY` → `selectedreplay >= 0` 이면 해당 슬롯 리플레이, 아니면 PLAY.
- 선택이 `DirectoryBar`: `FOLDER_OPEN` 또는 `RIGHT`/`ENTER` → `updateBar(dirbar)` 성공 시 `FOLDER_OPEN` 사운드.
- `FOLDER_CLOSE` 또는 키보드 `LEFT` → `resetKeyChangedTime(1)` 후 `manager.close()`.
- `NUM7` → `rival`, `NUM8` → `SHOW_SONGS_ON_SAME_FOLDER`, `NUM9` → `open_document`.
- `F10` → 선택이 폴더면 폴더 오토플레이. `F11` → `open_ir`. `F8` → `favorite_song`. `F9` → `favorite_chart`.

패널과 무관하게 항상 처리(`select/MusicSelectInputProcessor.java:71-113`, `348-372`):
- `NUM0` → OS 텍스트 입력 팝업(제목 `"Search"`, 힌트 `"Search bms title"`) → `select.search(text)`.
- `NUM1` → `mode`, `NUM2` → `sort`, `NUM3` → `lnmode`.
- `NUM4` 또는 (패널 키 없음 + `NEXT_REPLAY`) → `NEXT_REPLAY`.
- `F2` → `update_folder`, `F3` → `open_with_explorer`, `Ctrl+F3` → MD5 복사, `Ctrl+Shift+F3` → SHA256 복사.
- `ESCAPE` → `main.exit()` (앱 종료).

패널 1 의 타깃 스크롤(`154-203`): 휠·아날로그·스크래치 유지(300ms 후 50ms 반복)로 `mov` 를 만들고, `mov > 0` 마다 `target -1` + `SCRATCH`, `mov < 0` 마다 `target +1` + `SCRATCH`.

패널 3 의 표시 시간 반복(`259-282`): 첫 프레임 1 단위, 300ms 후부터 50ms 마다 반복하며 반복 횟수가 50 을 넘으면 증분 10(`executeEvent(duration1p, ±1, 10)`).

### 3.6 이벤트 id(스킨 버튼의 `act`)

`skin/property/EventFactory.java:169-803`. 대부분 `state instanceof MusicSelector` 일 때만 동작하고 `OPTION_CHANGE` 를 재생한다. `arg1 >= 0` 이면 다음, 음수면 이전.

| id | 이름 | 동작 |
|:--:|------|------|
| 10 | `difficulty` | 난이도 필터 순환 + `updateBar()` |
| 11 | `mode` | 모드 필터 `ModeFilter.nextEnabled` + `updateBar()` |
| 12 | `sort` | `sort = (sort ± 1) % 8`, `sortid = BarSorter.defaultSorter[sort].name()` + `updateBar()` |
| 312 | `songbar_sort` | `BarSorter.allSorter`(12종) 순환 + `updateBar()` |
| 13 | `keyconfig` | `changeState(CONFIG)` |
| 14 | `skinconfig` | `changeState(SKINCONFIG)` |
| 15 | `play` | `selectSong(PLAY)` |
| 16 | `autoplay` | `selectSong(AUTOPLAY)` |
| 315 | `practice` | `selectSong(PRACTICE)` |
| 17 | `open_document` | 곡 폴더의 `.txt` 를 OS 뷰어로 열기 |
| 19 / 316 / 317 / 318 | `replay1~4` | 선택: 해당 슬롯 리플레이 재생. 결과·코스 결과: 해당 슬롯에 리플레이 저장 |
| 40 | `gauge1p` | `gauge = (gauge ± 1) % 6` |
| 42 | `option1p` | `random = (random ± 1) % 10` |
| 43 | `option2p` | `random2 = (random2 ± 1) % 10` |
| 54 | `optiondp` | `doubleoption = (… ± 1) % 4` |
| 55 | `hsfix` | 선택 막대의 PlayConfig `fixhispeed = (… ± 1) % 5` |
| 57 | `hispeed1p` | `hispeed ± hispeedMargin` (범위 클램프) |
| 59 | `duration1p` | `duration ± (arg2 > 0 ? arg2 : 1)` (범위 클램프) |
| 342 | `hispeedautoadjust` | 토글 |
| 72 | `bga` | `bga = (… ± 1) % 3` |
| 73 | `bgaexpand` | `% 3` |
| 74 | `notesdisplaytiming` | `judgetiming ± 1` (범위 내). 선택 화면 밖에서도 동작하되 사운드는 선택 화면만 |
| 75 | `notesdisplaytimingautoadjust` | 토글 |
| 77 | `target` | 타깃 목록 순환(사운드 없음) |
| 78 | `gaugeautoshift` | `% 5` |
| 341 | `bottomshiftablegauge` | `% 3` |
| 79 | `rival` | 라이벌 순환(없음 포함) → `setRival` → `updateBar()` |
| 89 | `favorite_song` | 즐겨찾기/비표시(곡) 3상 순환 + 메시지 + `updateBar()` |
| 90 | `favorite_chart` | 즐겨찾기/비표시(차트) 3상 순환 |
| 210 | `open_ir` | IR 페이지를 브라우저로(선택·결과·코스 결과) |
| 211 | `update_folder` | 폴더/표/곡 폴더 갱신 |
| 212 | `open_with_explorer` | 파일 탐색기 열기 |
| 213 | `open_download_site` | 곡 URL 열기 |
| 308 | `lnmode` | `lnmode = (… ± 1) % 3` + `updateBar()` |
| 321~324 | `autosavereplay1~4` | 자동 저장 조건 순환(11종) |
| 330 / 331 / 332 | `lanecover` / `lift` / `hidden` | 토글 |
| 340 | `judgealgorithm` | 순환 |
| 343 | `guidese` | 토글 |
| 344 | `chartreplicationmode` | 순환(코드가 `sortid` 를 읽고 쓰는 것으로 보이는 결함 있음 — `733-744`) |
| 350 | `extranotedepth` | `% 4` |
| 351 | `minemode` | `% 5` |
| 352 | `scrollmode` | `% 3` |
| 353 | `longnotemode` | `% 6` |
| 360 / 361 | `seventonine_pattern` / `_type` | `% 7` / `% 3` |
| `OPTION_CONSTANT` | `constant` | 토글 |
| 101~139, 150~164 | `keyassign1~54` | 키 설정 화면용이나 본문이 비어 있어 아무 동작도 하지 않는다(`84-86`, `152-161`, `859-865`) |

목록에 없는 id 는 `state.executeEvent(id, arg1, arg2)` 로 넘어가 화면별 재정의 또는 커스텀 이벤트로 처리된다(`skin/property/EventFactory.java:47-60`).

### 3.7 검색 입력

- 문자열 프로퍼티 `searchword` id `30`: 읽기는 항상 빈 문자열, 쓰기는 `selector.search(value)` (`skin/property/StringPropertyFactory.java:230-234`).
- 텍스트 객체는 `editable` 이 true 이거나, `event` 가 없고 `ref` 에 writer 가 있으면 편집 가능이 된다(`skin/json/JsonSkinObjectLoader.java:663-666`). 따라서 `{"id":"search","font":0,"size":24,"ref":30}`(`skin/default/select.json:212`) 는 자동으로 편집 가능하다.
- 편집 가능한 텍스트는 내용이 비어 있어도 그려진다(`skin/SkinText.java:100-110`). 클릭 영역은 dst 사각형이며 정렬에 따라 x 가 보정된다(`skin/SkinText.java:162-169`).
- 클릭 시 scene2d `TextField` 를 띄우고(시스템 폰트, 크기 = dst 높이), Enter 또는 포커스 상실 또는 영역 밖 클릭 시 `writer.set(state, text)` 로 확정한다(`skin/SkinTextInput.java:35-81`, `147-162`, `Skin.java:394-406`).
- `search(text)`: 공백이면 무시. `SearchWordBar` 의 자식이 1개 이상이면 검색 막대 목록에 추가(같은 제목은 교체, 최대 `maxSearchBarCount` 기본 10, 초과 시 가장 오래된 것 제거) → 루트로 이동 → 그 검색 막대를 선택(`select/MusicSelector.java:311-321`, `select/BarManager.java:557-568`, `Config.java:79`).
- `MusicSelectSkin.searchTextRegion` 은 설정하는 코드가 없다(setter 호출처 없음 — `select/MusicSelectSkin.java:21`, `43-51`, 전체 grep).

### 3.8 정렬·모드·LN 모드·난이도 필터

| 항목 | 값 | 스킨 연결 | 근거 |
|------|----|-----------|------|
| 정렬(`sort` 0~7) | `TITLE, ARTIST, BPM, LENGTH, LEVEL, CLEAR, SCORE, MISSCOUNT` | 이미지 인덱스 `ref 12`, 문자열 `61` = `sortid` | `select/BarSorter.java:260`, `skin/default/select.json:99`, `skin/property/StringPropertyFactory.java:237` |
| 전체 정렬(`songbar_sort`) | 위 8종 + `DURATION, LASTUPDATE, RIVALCOMPARE_CLEAR, RIVALCOMPARE_SCORE` | 문자열 `61` | `select/BarSorter.java:19-262` |
| 모드 필터 | `ALL 0, BEAT_7K 2, BEAT_14K 4, POPN_9K 5, BEAT_5K 1, BEAT_10K 3, KEYBOARD_24K 6, KEYBOARD_24K_DOUBLE 7, BEAT_5K_7K 8, BEAT_10K_14K 9`(뒤 숫자 = `skinNumber`) | 이미지 인덱스 `ref 11`, 문자열 `60` = 표시명(`"ALL","7KEY","14KEY","9KEY","5KEY","10KEY","24KEY","48KEY","SINGLE","DOUBLE"`) | `select/ModeFilter.java:12-21`, `skin/default/select.json:152`, `skin/property/StringPropertyFactory.java:235` |
| LN 모드 | 0 LN, 1 CN, 2 HCN | 이미지 인덱스 `ref 308` | `skin/property/EventFactory.java:670-678`, `skin/default/select.json:102` |
| 난이도 필터 | `ALL 0, BEGINNER 1, NORMAL 2, HYPER 3, ANOTHER 4, INSANE 5, SCRATCH_CHART 6, LONG_NOTE_CHART 7, SPEED_CHANGE_CHART 8` | 문자열 `62` = 표시명 | `select/DifficultyFilter.java:16-24`, `skin/property/StringPropertyFactory.java:238` |

정렬 규칙 요점: `TITLE` 은 `SongBar`/`FolderBar` 를 앞에, 그 외 막대를 뒤에 둔다. 곡끼리는 제목(대소문자 무시) → 같으면 `difficulty`. 다른 정렬은 양쪽이 모두 `SongBar` 가 아니면 `TITLE` 비교로 넘어가고, 곡 없음/스코어 없음을 뒤로 보낸다(`select/BarSorter.java:19-43`, `47-61` 등).

### 3.9 막대 목록 구성(`BarManager.updateBar`)

`select/BarManager.java:272-475`:

1. 루트(`bar == null`) 구성 순서: 루트 폴더들(`FolderBar(null, "e2977170").getChildren()`) → 코스 표(`TableBar "COURSE"`) → 즐겨찾기(`HashBar` 들) → 외부 추가 폴더 → 난이도표(`TableBar` 들) → 커맨드(`"LAMP UPDATE"`, `"SCORE UPDATE"` 컨테이너 각 30일분 + `folder/default.json` 정의) → 검색 막대(`284-295`, `227-262`).
2. 하위 폴더면 `bar.getChildren()`. 컨테이너이고 랜덤 코스 결과가 있으면 같은 경로의 결과 코스를 덧붙인다(`296-321`).
3. `showNoSongExistingBar` 가 꺼져 있으면 곡 없는 SongBar·곡 누락 GradeBar 제거(`323-331`).
4. 필터: 현재 난이도 필터부터, 그 안에서 현재 모드 필터부터 순환하며, SongBar 중 비표시 곡·모드 불일치·난이도 불일치를 제거한다. 전부 제거되지 않는 첫 조합을 채택하고 그 조합을 설정에 저장한다(필터가 자동으로 바뀔 수 있다) (`333-371`).
5. 폴더 스택(`dir`)과 진입 원본 막대(`sourcebars`) 갱신(`373-378`).
6. 캐시된 스코어 주입(`380-388`).
7. `bar.isSortable()` 이면 `sortid` 의 정렬기로 정렬(없으면 `TITLE`) (`390-393`).
8. `randomSelect` 설정이면 랜덤 폴더 정의마다 조건을 만족하는 곡이 충분할 때(필터 있으면 1곡 이상, 없으면 2곡 이상) `ExecutableBar` 를 목록 맨 앞에 추가(`395-418`).
9. `selectedindex = 0` 후 커서 복원: 이전(또는 진입 원본) 막대가 곡이면 같은 `sha256`, 아니면 같은 클래스·같은 제목(`424-447`).
10. 백그라운드 로더 시작(스코어, 라이벌 스코어, 리플레이 존재 4슬롯, 폴더 램프 집계, 곡 정보, 배너, 스테이지파일) (`449-453`, `632-742`).
11. `ScoreDataProperty` 갱신, 경로 문자열 `dirString = "제목 > 제목 > "` 생성(문자열 프로퍼티 `1000`), `selectedBarMoved()` (`454-465`, `skin/property/StringPropertyFactory.java:258`).
12. 결과가 0개면 이전 폴더로 되돌리고 false 반환(`468-474`).

`close()`: 루트면 `sort` 이벤트, 아니면 상위로 `updateBar` + `FOLDER_CLOSE` (`490-501`).

### 3.10 선택 화면이 스킨에 주는 그 밖의 상태

| 상태 | 값 | 옵션/프로퍼티 | 근거 |
|------|----|---------------|------|
| 선택 막대 종류 | 폴더 / 곡 / 코스 / 재생 가능 | `OPTION_FOLDERBAR 1`, `OPTION_SONGBAR 2`, `OPTION_GRADEBAR 3`, `OPTION_PLAYABLEBAR 5` | `skin/SkinProperty.java:623-625`, `643`, `skin/property/BooleanPropertyFactory.java:356-367`, `500-502` |
| 코스 제약 | `existsConstraint` | `OPTION_GRADEBAR_CLASS 1002` … `OPTION_GRADEBAR_HCN 1017` | `skin/SkinProperty.java:626-639`, `select/MusicSelector.java:571-587` |
| 리플레이 존재 | 슬롯 0~3 | `OPTION_NO_REPLAYDATA 196 / REPLAYDATA 197`, `1196/1197`, `1199/1200`, `1202/1203` | `skin/SkinProperty.java:773-784`, `skin/property/BooleanPropertyFactory.java:333-347` |
| 선택된 리플레이 슬롯 | `selectedreplay` -1~3 | `OPTION_SELECT_REPLAYDATA 1205` ~ `1208` | `skin/SkinProperty.java:785-788`, `skin/property/BooleanPropertyFactory.java:484-491` |
| 라이벌 | `getRival()` | `OPTION_NOT_COMPARE_RIVAL` / `OPTION_COMPARE_RIVAL` | `skin/property/BooleanPropertyFactory.java:368-371` |
| 선택 막대 클리어 상태 | — | `OPTION_SELECT_BAR_NOT_PLAYED 100` 등(`101, 1100, 1101, 102, 103, 104, 1102, 105, 1103, 1104`) | `skin/SkinProperty.java:704-714` |
| IR 랭킹 | `currentir`, `rankingOffset` | 숫자 `179`(순위), 랭킹 이름/점수 프로퍼티, `OPTION_IR_NOPLAYER 603`, `IR_FAILED 604`, `IR_BUSY 608`, `IR_WAITING 606`, 슬라이더 type `8` | `skin/property/IntegerPropertyFactory.java:239-250`, `skin/property/BooleanPropertyFactory.java:665-689`, `skin/property/FloatPropertyFactory.java:243-260` |
| 코스 곡 제목 | 선택 코스의 n 번째 곡 | 문자열 `150~159` (없는 곡은 `"(no song) "` 접두) | `skin/property/StringPropertyFactory.java:384-410` |

리플레이 슬롯 규칙(`select/MusicSelectCommand.java:25-59`): `RESET_REPLAY` 는 존재하는 가장 낮은 슬롯(없으면 -1), `NEXT_REPLAY`/`PREV_REPLAY` 는 존재하는 다음/이전 슬롯으로 순환하며 `OPTION_CHANGE` 재생. 최대 슬롯 수 `MusicSelector.REPLAY = 4` (`select/MusicSelector.java:49`).

---

## 4. 결정 화면

`decide/MusicDecide.java` 전체(70줄). 전용 스킨 객체 없음(`decide/MusicDecideSkin.java:10-15`).

| 단계 | 동작 | 줄 |
|------|------|----|
| `create()` | `cancel = false` → `loadSkin(DECIDE)` → `resource.setOrgGaugeOption(playerConfig.gauge)` | `24-30` |
| `prepare()` | `play(DECIDE)` | `32-35` |
| `render()` | `nowtime > skin.input` → `switchTimer(TIMER_STARTINPUT, true)`. `TIMER_FADEOUT` 이 켜져 있으면 `getNowTime(TIMER_FADEOUT) > skin.fadeout` 일 때 `changeState(cancel ? MUSICSELECT : PLAY)`. 꺼져 있으면 `nowtime > skin.scene` 일 때 `setTimerOn(TIMER_FADEOUT)` | `37-51` |
| `input()` | `TIMER_FADEOUT` 꺼짐 && `TIMER_STARTINPUT` 켜짐일 때만: 키 인덱스 `0, 2, 4, 6` 중 하나가 눌린 상태이거나 `ENTER` 눌림 → `setTimerOn(TIMER_FADEOUT)`(건너뛰기). `ESCAPE` 눌림 또는 START+SELECT 동시 → `cancel = true` + `setTimerOn(TIMER_FADEOUT)` | `53-64` |

타이머: `TIMER_STARTINPUT(1)`, `TIMER_FADEOUT(2)` 두 개뿐.
건너뛰기 키는 "눌린 상태" 판정이라, 선택 화면에서 누른 키를 계속 누르고 있으면 `input` 시간이 지나자마자 건너뛴다.
취소해도 `fadeout` 시간만큼 기다린 뒤 선택 화면으로 돌아간다.

기본 결정 스킨 예(`skin/default/decide.json:36-39`): `{"id":"blank", "loop":500, "timer":2, "dst":[{"time":0,…,"a":0},{"time":500,"a":255}]}` — `TIMER_FADEOUT` 에 묶인 페이드아웃 덮개.

---

## 5. 결과 화면

### 5.1 수명주기(`result/MusicResult.java`)

| 단계 | 동작 | 줄 |
|------|------|----|
| `create()` | 리플레이 슬롯 0~3 의 상태 초기화(`EXIST`/`NOT_EXIST`) → 키 배정 `ResultKeyProperty.get(mode)` → `updateScoreDatabase()` → PLAY 모드면 슬롯별 자동 저장 조건 충족 시 `saveReplayData(i)` → 코스 중이면 리플레이·게이지 이력 누적 → `gaugeType = grooveGauge.getType()` → `loadSkin(RESULT)` | `42-72` |
| `prepare()` | `state = STATE_OFFLINE` → 랭킹 데이터 준비, `rankingOffset = 0` → IR 계정이 있고 PLAY 모드면 `state = STATE_IR_PROCESSING` 후 전송 스레드 시작 → 클리어/실패 사운드 | `74-152` |
| `render()` | §5.2 | `160-260` |
| `input()` | §5.3 | `262-315` |
| `shutdown()` | 결과 사운드 3종 정지 | `154-158` |

### 5.2 타이머와 전환

`result/MusicResult.java:160-260`, `74-147`:

| 타이머 | id | 켜지는 시점 |
|--------|:--:|-------------|
| `TIMER_RESULTGRAPH_BEGIN` | 150 | 매 render 에서 `switchTimer(…, true)` → 첫 프레임에 켜짐 |
| `TIMER_RESULTGRAPH_END` | 151 | 같음(시작과 동시에 켜짐) |
| `TIMER_RESULT_UPDATESCORE` | 152 | `ranktime == 0` 이면 첫 프레임. `ranktime != 0` 이면 확인 키를 처음 눌렀을 때(JSON/Lua 스킨은 항상 0) |
| `TIMER_STARTINPUT` | 1 | `time > skin.input` |
| `TIMER_FADEOUT` | 2 | 확인 입력(§5.3) 또는 `time > skin.scene` |
| `TIMER_IR_CONNECT_BEGIN` | 172 | IR 전송 스레드가 첫 전송을 시작할 때 |
| `TIMER_IR_CONNECT_SUCCESS` | 173 | 전송이 모두 성공으로 끝났을 때 |
| `TIMER_IR_CONNECT_FAIL` | 174 | 전송 중 실패가 있었을 때 |

FADEOUT 완료(`getNowTime(TIMER_FADEOUT) > skin.fadeout`) 시의 다음 화면(`172-248`):

| 조건 | 다음 화면 |
|------|-----------|
| 코스 중 + 현재 게이지 종류의 마지막 값 `<= 0` + 코스 스코어 있음 | 남은 곡 노트를 POOR 로 가산 후 `COURSERESULT` |
| 코스 중 + 게이지 0 이하 + 코스 스코어 없음 | `MUSICSELECT` |
| 코스 중 + 다음 곡 있음(`resource.nextCourse()`) | `PLAY` |
| 코스 중 + 마지막 곡 | `COURSERESULT` |
| 단곡 + PLAY 모드 + `REPLAY_DIFFERENT` 키를 누른 상태 | 랜덤 시드 -1 로 재배치 후 `PLAY` |
| 단곡 + PLAY 모드 + `REPLAY_SAME` 키를 누른 상태 | 같은 배치로 `PLAY`(스코어 갱신 불가 상태면 시드 -1) |
| 그 외 | `MUSICSELECT` |

단곡 분기에서는 먼저 `playerConfig.gauge` 를 결정 화면에서 저장한 원래 값으로 되돌린다(`214`). 전환 직전에 키음 정지, 키 변경 시각 초기화를 한다(`174-177`).

### 5.3 입력

키 배정(`result/ResultKeyProperty.java:9-16`), 7키 기준 인덱스:

| 키 인덱스 | 역할 |
|:---:|------|
| 0~3 (1~4번 키) | `OK` |
| 4 (5번 키) | `REPLAY_DIFFERENT` |
| 5 (6번 키) | `CHANGE_GRAPH` |
| 6 (7번 키) | `REPLAY_SAME` |
| 7, 8 (스크래치) | 없음 |

`BEAT_5K` 는 같고, `BEAT_10K`/`BEAT_14K` 는 2P 쪽에 반복, `POPN_9K` 는 7·8 도 `OK`, 24키는 별도 배열. 모드를 찾지 못하면 `BEAT_7K`.

`input()` (`result/MusicResult.java:262-315`, 공통부 `result/AbstractResult.java:247-255`):

1. 항상: 마우스 휠로 `rankingOffset` 을 `[0, max(1, totalPlayer) - 1]` 범위에서 이동.
2. `TIMER_FADEOUT` 꺼짐 && `TIMER_STARTINPUT` 켜짐 && `time > skin.input` 일 때만 이하 처리.
3. `CHANGE_GRAPH` 키가 새로 눌리면 게이지 종류 순환: 현재가 `0~5`(ASSISTEASY~HAZARD)이면 `(g + 1) % 6`, 아니면 `(g - 5) % 3 + 6`(6→7→8→6).
4. 그 외 배정된 키(`OK`, `REPLAY_*`)가 새로 눌리면 `ok = true`. `ESCAPE` 또는 `ENTER` 도 `ok = true`.
5. `resource.getScoreData() == null || ok` 이면
   - `ranktime != 0` 이고 `TIMER_RESULT_UPDATESCORE` 가 꺼져 있으면 그것만 켠다.
   - 아니고 `state == STATE_OFFLINE(0)` 또는 `STATE_IR_FINISHED(2)` 이면 `TIMER_FADEOUT` 켜고 닫는 사운드 처리. IR 처리 중(`STATE_IR_PROCESSING 1`)에는 닫히지 않는다.
6. `NUM1~NUM4` → `saveReplayData(0~3)`.
7. `F11` → `open_ir`.

스코어 데이터가 없으면(예: 스코어 미생성 종료) 입력 없이도 `input` 시간 직후 닫힌다.

### 5.4 리플레이 저장 슬롯

- 상태 `ReplayStatus { EXIST, NOT_EXIST, SAVED }`, 슬롯 수 `REPLAY_SIZE = 4` (`result/AbstractResult.java:56-57`, `205-207`).
- 저장 조건(`result/MusicResult.java:317-327`): PLAY 모드 && 코스 아님 && 스코어 있음 && 그 슬롯이 아직 `SAVED` 가 아님 && `resource.isUpdateScore()`. 저장 후 `SAVED`.
- 자동 저장 조건 `ReplayAutoSaveConstraint`(인덱스 순): `NOTHING, SCORE_UPDATE, SCORE_UPDATE_OR_EQUAL, MISSCOUNT_UPDATE, MISSCOUNT_UPDATE_OR_EQUAL, MAXCOMBO_UPDATE, MAXCOMBO_UPDATE_OR_EQUAL, CLEAR_UPDATE, CLEAR_UPDATE_OR_EQUAL, ANYONE_UPDATE, ALWAYS` (`result/AbstractResult.java:80-198`).
- 스킨 옵션(슬롯 1~4): 없음 `196 / 1196 / 1199 / 1202`, 있음 `197 / 1197 / 1200 / 1203`, 저장됨 `198 / 1198 / 1201 / 1204` (`skin/SkinProperty.java:773-784`, `skin/property/BooleanPropertyFactory.java:341-344`).
- 저장 버튼 이벤트: `19, 316, 317, 318` (`skin/property/EventFactory.java:364-367`, `867-879`). 기본 스킨 예: `{"id":200,…,"act":19}` 와 `{"id":200,"op":[196,-198],…}` / `[197,-198]` / `[198]` 3상 표시(`skin/default/result.json:20-23`, `157-165`).

### 5.5 결과 화면이 스킨에 주는 상태

| 상태 | 내용 | 근거 |
|------|------|------|
| `state` | `STATE_OFFLINE 0`, `STATE_IR_PROCESSING 1`, `STATE_IR_FINISHED 2` | `result/AbstractResult.java:20-30` |
| `gaugeType` | 그래프·게이지 표시 대상 게이지 종류 0~8 (`ASSISTEASY 0, EASY 1, NORMAL 2, HARD 3, EXHARD 4, HAZARD 5, CLASS 6, EXCLASS 7, EXHARDCLASS 8`) | `result/AbstractResult.java:59`, `209-211`, `play/GrooveGauge.java:20-28` |
| 게이지 옵션 | `42`(gauge_groove) = `type <= 2`, `43`(gauge_hard) = `type >= 3`, `1046`(gauge_ex) = `type ∈ {0,1,4,5,7,8}` — 결과 화면에서는 `gaugeType` 기준 | `skin/property/BooleanPropertyFactory.java:396-417`, `693-706` |
| 게이지 값 | FloatProperty `1107` = `resource.getGauge()[gaugeType]` 의 마지막 값 | `skin/property/FloatPropertyFactory.java:449-458` |
| 스코어 | `ScoreDataProperty` 에 새 스코어·이전 최고(`oldscore`)·타깃 반영 | `result/MusicResult.java:339-344` |
| 타이밍 분포 | 범위 ±150ms, 평균·표준편차 | `result/AbstractResult.java:47-52`, `278-340`, `result/MusicResult.java:345-365` |
| IR 순위 | `getIRRank()`, `getOldIRRank()`, `getIRTotalPlayer()`, `rankingOffset`(순위 > 10 이면 `순위 - 5` 로 초기화) | `result/AbstractResult.java:221-231`, `result/MusicResult.java:133-134` |
| 랭킹 위치 슬라이더 | type `8`: 읽기 `rankingOffset / max(1,total)`, 쓰기 `(int)(max * value)` | `result/AbstractResult.java:261-271`, `skin/property/FloatPropertyFactory.java:243-260` |
| 클리어/실패 | `OPTION_RESULT_CLEAR 90`, `OPTION_RESULT_FAIL 91` | `skin/SkinProperty.java:701-702` |
| 갱신 여부 | `OPTION_UPDATE_SCORE 330`, `MAXCOMBO 331`, `MISSCOUNT 332`, `TRIAL 333`, `IRRANK 334`, `SCORERANK 335`, `TARGET 336` 및 `OPTION_DRAW_* 1330/1331/1332/1335/1336` | `skin/SkinProperty.java:902-913` |

### 5.6 gaugegraph

스키마(`skin/json/JsonSkin.java:244-261`):

| 필드 | 기본값 |
|------|--------|
| `id` | — |
| `color` | null (문자열 배열, 최대 24개) |
| `assistClearBGColor` | `"440044"` |
| `assistAndEasyFailBGColor` | `"004444"` |
| `grooveFailBGColor` | `"004400"` |
| `grooveClearAndHardBGColor` | `"440000"` |
| `exHardBGColor` | `"444400"` |
| `hazardBGColor` | `"444444"` |
| `assistClearLineColor` | `"ff00ff"` |
| `assistAndEasyFailLineColor` | `"00ffff"` |
| `grooveFailLineColor` | `"00ff00"` |
| `grooveClearAndHardLineColor` | `"ff0000"` |
| `exHardLineColor` | `"ffff00"` |
| `hazardLineColor` | `"cccccc"` |
| `borderlineColor` | `"ff0000"` |
| `borderColor` | `"440000"` |

`delay`, `lineWidth` 필드는 없다(각각 1500ms, 2px 고정 — `result/SkinGaugeGraphObject.java:30-34`). 색 문자열은 `Color.valueOf` 로 해석하므로 `RRGGBB` 또는 `RRGGBBAA` 다. ModernChic 은 `"44004455"` 같은 8자리 알파 포함 값을 쓴다(`ModernChic/Result/lua/mainmenu.lua:268-284`).

내부 색 테이블(6종 × 4색, `result/SkinGaugeGraphObject.java:36-51`):
- `graphcolor[t]`: 보더 아래 배경, `graphline[t]`: 보더 아래 선, `bordercolor[t]`: 보더 위 배경, `borderline[t]`: 보더 위 선.

생성 규칙(`skin/json/JsonSkinObjectLoader.java:457-474`):
- `color` 가 있으면 `colors[i/4][i%4] = Color.valueOf(color[i])` (i < 24). 각 종류의 4색은 순서대로 `[0] borderline, [1] bordercolor, [2] graphline, [3] graphcolor` 이고 빠진 값은 `"000000"` (`result/SkinGaugeGraphObject.java:74-85`).
- `color` 가 없으면 이름 붙은 14개 필드 사용(`result/SkinGaugeGraphObject.java:87-111`):

| 종류 인덱스 `t` | `graphcolor` | `graphline` | `bordercolor` | `borderline` |
|:---:|------|------|------|------|
| 0 | `assistClearBGColor` | `assistClearLineColor` | `borderColor` | `borderlineColor` |
| 1 | `assistAndEasyFailBGColor` | `assistAndEasyFailLineColor` | `borderColor` | `borderlineColor` |
| 2 | `grooveFailBGColor` | `grooveFailLineColor` | `borderColor` | `borderlineColor` |
| 3 | = bordercolor | = borderline | `grooveClearAndHardBGColor` | `grooveClearAndHardLineColor` |
| 4 | = bordercolor | = borderline | `exHardBGColor` | `exHardLineColor` |
| 5 | = bordercolor | = borderline | `hazardBGColor` | `hazardLineColor` |

게이지 종류 → 색 인덱스: `typetable = {0,1,2,3,4,5,3,4,5,3}` (`result/SkinGaugeGraphObject.java:53`). 즉 `CLASS 6 → 3`, `EXCLASS 7 → 4`, `EXHARDCLASS 8 → 5`.

`prepare` (`result/SkinGaugeGraphObject.java:113-137`):
- `render = time >= delay ? 1.0 : time / delay` — `time` 은 화면 경과 ms.
- 표시 종류: 기본은 `resource.getGrooveGauge().getType()`, 결과/코스 결과 화면이면 `state.gaugeType`.
- 종류가 바뀌면 다시 그린다. 이력 `gaugehistory = resource.getGauge()[type]`. 코스 결과면 `resource.getCourseGauge()` 의 곡별 이력을 이어 붙이고 곡 경계 누적 인덱스를 `section` 에 기록.
- `gg = grooveGauge.getGauge(type)` 에서 `border`, `max` 를 얻는다.

`draw` (`result/SkinGaugeGraphObject.java:139-233`), 캔버스 크기 = dst `(int)width × (int)height`, 크기가 바뀌거나 `redraw` 면 재생성:

1. 배경: 전체를 `graphcolor[c]` 로 채우고, `(0, height*border/max)` 부터 높이 `height*(max-border)/max` 를 `bordercolor[c]` 로 채운다.
2. 선: `i = 0..size-1`
   - `section` 에 `i` 가 있으면 x = `width*(i-1)/size` 에 흰색 세로선(곡 경계).
   - 이전 값 `f1` 이 있으면 `x1 = width*(i-1)/size`, `x2 = width*i/size`, `y1 = (f1/max)*(height - lineWidth)`, `y2 = (f2/max)*(height - lineWidth)`, `yb = (border/max)*(height - lineWidth)`.

| `f1` | `f2` | 그리는 사각형(색) |
|------|------|-------------------|
| `< border` | `< border` | `(x1, min(y1,y2), lineWidth, |y2-y1| + lineWidth)` 와 `(x1, y2, x2-x1, lineWidth)` — `graphline` |
| `< border` | `>= border` | `(x1, y1, lineWidth, yb-y1)` — `graphline`; `(x1, yb, lineWidth, y2-yb+lineWidth)` 와 `(x1, y2, x2-x1, lineWidth)` — `borderline` |
| `>= border` | `>= border` | `(x1, min(y1,y2), lineWidth, |y2-y1| + lineWidth)` 와 `(x1, y2, x2-x1, lineWidth)` — `borderline` |
| `>= border` | `< border` | `(x1, yb, lineWidth, y1-yb+lineWidth)` — `borderline`; `(x1, y2, lineWidth, yb-y2)` 와 `(x1, y2, x2-x1, lineWidth)` — `graphline` |

3. 마지막 값이 있으면 `(lastX, lastY, width - lastX, lineWidth)` 를 마지막 값 기준 색(`< border` 면 `graphline`, 아니면 `borderline`)으로 채워 오른쪽 끝까지 잇는다.
4. 출력: 배경 텍스처를 `(x, y + height, width, -height)` 로 그린다(상하 반전 → 캔버스 y=0 이 화면 아래쪽). 선 텍스처는 왼쪽 `(int)(width * render)` 픽셀만 같은 방식으로 그린다(왼쪽에서 오른쪽으로 1500ms 에 걸쳐 드러남).

계단형 그래프(수평 구간 + 수직 연결)이며 안티에일리어싱이 없다. 이력 샘플 간격은 이 파일에서 확인되지 않는다(코스 결과의 빈 곡 채움 코드가 `(lastNoteTime + 500) / 500` 개를 쓰는 것으로 보아 500ms 로 추정되나 미확인 — `result/CourseResult.java:43-52`).

기본 스킨 예: `"gaugegraph":[{"id":2000}]`(`skin/default/result.json:74-76`), Lua 판은 `color = {"ff8888","442222","ff00ff","440044", …}` 형식(`skin/default/result/resultmain.lua:103-104`).

---

## 6. 코스 결과 화면

`result/CourseResult.java`. 결과 화면과의 차이만 적는다. 전용 스킨 객체 없음(`result/CourseResultSkin.java:5-21`).

| 항목 | 내용 | 줄 |
|------|------|----|
| `create()` | 코스 리플레이 슬롯 상태 초기화 → 플레이하지 못한 곡의 게이지 이력을 0 으로 채움(곡마다 `(lastNoteTime + 500) / 500` 개) → 키 배정 → `updateScoreDatabase()` → 자동 저장 → `gaugeType` → `loadSkin(COURSE_RESULT)` | `37-73` |
| `render()` | 첫 프레임에 `TIMER_RESULTGRAPH_BEGIN`, `TIMER_RESULTGRAPH_END`, `TIMER_RESULT_UPDATESCORE` 를 모두 켠다(ranktime 조건 없음). `time > input` → STARTINPUT. FADEOUT 완료 시 게이지 옵션 복원 후 항상 `MUSICSELECT`. `time > scene` → FADEOUT + 닫는 사운드 | `169-193` |
| `input()` | 조건은 `TIMER_FADEOUT` 꺼짐 && `TIMER_STARTINPUT` 켜짐(`time > input` 재검사 없음). `CHANGE_GRAPH` → `gaugeType = (gaugeType - 5) % 3 + 6`. 나머지는 결과 화면과 같음(확인 키, ESC/ENTER, IR 상태 대기, NUM1~4 저장, F11) | `195-240` |
| 게이지 순환 | 식이 하나뿐이라 현재 종류가 6~8 이 아닐 때는 6 으로 가거나 범위 밖 값이 될 수 있다(예: 3 → `(-2) % 3 + 6 = 4`). 코스는 보통 6~8 이라 6→7→8→6 | `203` |
| 리플레이 저장 | PLAY 모드 && 코스 스코어 있음 && 슬롯 미저장 && `isUpdateCourseScore()`. 각 곡 리플레이의 `gauge` 를 현재 설정으로 채워 저장 | `289-302` |
| 스코어 | `getNewScore() = resource.getCourseScoreData()`, `getJudgeCount` 도 코스 스코어 기준 | `279-282`, `304-306` |
| IR | 코스 전송. 타이머 `172/173/174` 동작은 결과와 같음 | `82-157` |
| 그래프 | `SkinGaugeGraphObject` 가 곡별 이력을 이어 붙이고 곡 경계에 흰 세로선 | `result/SkinGaugeGraphObject.java:127-133`, `176-180` |

---

## 7. 키 설정 화면

`config/KeyConfiguration.java`. 클래스 주석에 `// TODO スキンベースへ移行`(`33`).

| 항목 | 내용 | 줄 |
|------|------|----|
| `create()` | `loadSkin(KEY_CONFIG)`. 스킨이 null 이면 빈 `KeyConfigurationSkin`(원본 해상도 HD, 대상 = 설정 해상도). 시스템 폰트 크기 `20 * skin.scaleY`. `ShapeRenderer` 생성. 컨트롤러 활성화. `setMode(0)` | `98-128` |
| `render()` | 화면 클리어 → 입력 처리 → 직접 그리기. `input()` 재정의 없음 | `130-316` |
| 타이머 | 사용 없음. `scene/input/fadeout` 무시 | 같은 범위 |
| 종료 | `ESCAPE` → `main.saveConfig()` → `changeState(MUSICSELECT)` | `242-245` |

그리기 순서: `render()` 안에서 자체 `SpriteBatch.begin/end` 와 `ShapeRenderer` 로 먼저 그린 뒤(`253-315`), 컨트롤러가 스킨 객체를 그 위에 그린다(`MainController.java:407-413`). 불투명 배경을 가진 스킨을 쓰면 내장 UI 가 가려진다.

입력(키 대기 상태가 아닐 때, `174-251`):

| 키 | 동작 |
|----|------|
| `LEFT` / `RIGHT` | 모드 전환(`"5 KEYS","7 KEYS","9 KEYS","10 KEYS","14 KEYS","24 KEYS","24 KEYS DOUBLE"` 순환) |
| `UP` / `DOWN` | 커서 이동(순환) |
| `NUM1` | 선택 화면 입력 모드 순환(`"7 KEYS","9 KEYS","14 KEYS"`) |
| `NUM2` / `NUM3` | 컨트롤러 장치 1 / 2 변경 |
| `NUM7` / `NUM8` / `NUM9` | 키보드 / 컨트롤러 / MIDI 기본값 복원 |
| `ENTER` | 현재 항목 키 대기 시작 |
| `DEL` | 현재 항목 배정 삭제 |
| `ESCAPE` | 저장 후 선택 화면 |

키 대기 상태(`148-173`): 키보드 마지막 눌림, 마우스 스크래치, 컨트롤러 버튼, MIDI 입력 중 먼저 들어온 것을 배정하고 대기 해제. 예약 키는 무시(`399-401`).

내장 UI 좌표(1280×720 기준, `scaleX/scaleY` 곱, `254-315`): 제목 `"<-- MODE -->"` (80, 650) 청록, 열 제목 `Keyboard` (180, 620) / `Controller1` (330, 620) / `Controller2` (480, 620) / `MIDI` (630, 620) 노랑, 안내문 (750, 620) 등 주황, 항목 행 `y = 576 - (i - scrollpos) * 24`, 항목명 x=50, 값 x=202 / 352 / 502 / 652, 커서 사각형 폭 80 높이 24(대기 중 빨강, 평소 파랑). 스크롤은 커서가 25행을 넘을 때.

스킨에 노출되는 것:
- 문자열 `key1~key10` = id `40~49`, `key11~key54` = id `240~283` → `keyconfig.getKeyAssign(index)`: 키보드 → 마우스 스크래치 → 컨트롤러1 → 컨트롤러2 → MIDI 순으로 처음 배정된 이름, 없으면 `"---"`, 범위 밖이면 `"!!!"` (`skin/property/StringPropertyFactory.java:81-82`, `412-419`, `config/KeyConfiguration.java:334-367`).
- 이벤트 `keyassign1~54`(id `101~139`, `150~164`)는 등록만 돼 있고 동작이 비어 있다(`skin/property/EventFactory.java:85-86`, `152-161`, `859-865`). `setKeyAssignMode(index)` 는 공개 메서드지만 호출처가 내부 `ENTER` 처리뿐이다(`config/KeyConfiguration.java:238-240`, `318-327`).

결론: 이 버전에서 키 설정 스킨은 배경·장식 외에는 기능이 없다. 기본 스킨과 ModernChic 모두 destination 이 비어 있어 내장 UI 만 보인다(`skin/default/keyconfig/keyconfigmain.lua:28-46`, `ModernChic/keyconfig.lua:13-32`). ModernChic 헤더는 `fadeout 1000 / scene 3000 / input 500` 을 선언하지만 쓰이지 않는다(`ModernChic/KeyConfig/lua/require/header.lua:12-14`).

---

## 8. 스킨 설정 화면

### 8.1 수명주기(`config/SkinConfiguration.java`)

| 단계 | 동작 | 줄 |
|------|------|----|
| `create()` | `loadSkin(SKIN_SELECT)` → `loadAllSkins()` → `changeSkinType(SkinType.getSkinTypeById(skin.getDefaultSkinType()))` | `49-54` |
| `render()` | `ESCAPE` → `saveConfig()` → `changeState(MUSICSELECT)` | `61-67` |
| `input()` | 휠: `customOptionOffset = clamp(offset + mov, 0, customOptionOffsetMax)` | `69-76` |
| `shutdown()` | 미리보기 스킨 해제 | `56-59` |
| 타이머 | 사용 없음. `scene/input/fadeout` 무시 | `61-76` |

`loadAllSkins()` (`489-549`): `skin/` 을 재귀 탐색(심볼릭 링크 제외)해 `.lr2skin`, `.luaskin`, `.json` 파일의 헤더를 읽는다. LR2 의 7키/14키 스킨은 5키/10키용으로도 한 번 더 등록한다.

### 8.2 스킨 전용 설정(`skinSelect`)

`skin/json/JsonSkin.java:539-544`, `skin/json/JSONSkinLoader.java:338-363`:

| 필드 | 기본값 | 의미 |
|------|--------|------|
| `customBMS` | null | 샘플 BMS 목록(읽는 쪽 없음 — `getSampleBMS` 사용처 없음) |
| `defaultCategory` | 0 | 처음 보여 줄 `SkinType` id |
| `customPropertyCount` | -1 | 한 화면에 보이는 커스텀 항목 수 |
| `customOffsetStyle` | 0 | 읽는 쪽 없음(`getCustomOffsetStyle` 사용처 없음) |

- `customPropertyCount <= 0` 이면 `image`/`imageset` 중 `act` 가 커스텀 버튼(§8.3)인 것의 최대 인덱스 + 1 로 계산한다. 이 계산은 `image.act.getEventId()` 를 조건 없이 부른다. `act` 가 없는 정의에서 어떻게 되는지는 직렬화기를 읽지 않아 미확인이다(`skin/json/JSONSkinLoader.java:346-361`).
- `skinSelect` 자체가 없으면 위 블록이 통째로 건너뛰어져 `customPropertyCount = -1`, `defaultSkinType = 0`(7키 플레이)이 된다(`config/SkinConfigurationSkin.java:8-9`).
- 기본 스킨은 `skinSelect = {defaultType = 6, customOffsetStyle = 0, customPropertyCount = 6, sampleBMS = {}}` 로 쓰는데(`skin/default/skinselect/skinselectmain.lua:367-372`), 필드명이 `defaultCategory`·`customBMS` 가 아니라서 `defaultType`·`sampleBMS` 는 무시된다. 실제 첫 종류는 0(7키)이다.
- ModernChic 은 `skin.skinSelect` 를 주석 처리했다(`ModernChic/skinselect.lua:153`). 따라서 `customPropertyCount = -1` 이고 `customOptionOffsetMax = max(0, 항목수 + 1)` 이 되어(§8.4), 보이는 행 수(10)를 빼지 않은 채 오프셋이 `항목수 + 1` 까지 올라간다. 즉 모든 행이 빈 문자열이 될 때까지 스크롤된다.

### 8.3 이벤트(`executeEvent`, `config/SkinConfiguration.java:114-149`)

| id | 상수 | 동작 |
|:--:|------|------|
| 190 | `BUTTON_CHANGE_SKIN` | `arg1 >= 0` 다음 스킨, 음수면 이전 스킨 |
| 220~228 | `BUTTON_SKIN_CUSTOMIZE1`~`9` | 화면상 n 번째 항목(`index = id - 220 + customOptionOffset`) 값 변경. `arg1 >= 0`: `value < max` 이면 +1, 아니면 `min`. 음수: `value > min` 이면 -1, 아니면 `max` |
| 229 | `BUTTON_SKIN_CUSTOMIZE10` | `isSkinCustomizeButton` 이 `id >= 220 && id < 229` 라서 인식되지 않는다. ModernChic 소스에도 "10번째 클릭 영역이 반응하지 않는다"는 주석이 있다 | 
| 170~185 | `BUTTON_SKINSELECT_7KEY`~`COURSE_RESULT` | `SkinType id = id - 170` 으로 종류 변경 |
| 386~388 | `BUTTON_SKINSELECT_24KEY`, `_24KEY_DOUBLE`, `_24KEY_BATTLE` | `SkinType id = id - 386 + 16` |
| 그 외 | — | `super.executeEvent` (커스텀 이벤트) |

근거: `skin/SkinProperty.java:1001-1020`, `1046-1048`, `skin/SkinPropertyMapper.java:101-130`, `ModernChic/skinselect.lua:53`(주석).

종류 버튼 id 대응: `170 7KEY, 171 5KEY, 172 14KEY, 173 10KEY, 174 9KEY, 175 MUSIC_SELECT, 176 DECIDE, 177 RESULT, 178 KEY_CONFIG, 179 SKIN_SELECT, 180 SOUND_SET, 181 THEME, 182 BATTLE7, 183 BATTLE5, 184 BATTLE9, 185 COURSE_RESULT, 386 24KEY, 387 24KEY_DOUBLE, 388 24KEY_BATTLE`.

클릭 방향은 스킨이 `click = 2`(영역 좌/우 반)로 지정하는 것이 관례다(`skin/default/skinselect/skinselectmain.lua:53-59`, §1.3).

### 8.4 상태 → 스킨

| 프로퍼티 | id | 값 | 근거 |
|----------|:--:|----|------|
| 스킨 이름 | 문자열 `50` | 선택된 스킨 헤더의 이름(없으면 `""`). 스킨 설정 화면이 아니면 현재 화면 스킨의 이름 | `skin/property/StringPropertyFactory.java:242-249` |
| 스킨 제작자 | 문자열 `51` | 같은 규칙 | `250-257` |
| 항목 이름 n | 문자열 `100~109` (`skincategory1~10`) | `customOptions[n + offset].categoryName`, 범위 밖 `""` | `83`, `332-339`, `config/SkinConfiguration.java:92-97` |
| 항목 값 n | 문자열 `110~119` (`skinitem1~10`) | `customOptions[n + offset].displayValue`, 범위 밖 `""` | `84`, `341-348`, `config/SkinConfiguration.java:99-104` |
| 종류 선택 여부 | 이미지 인덱스 `ref 170~185, 386~388` | 현재 종류면 1, 아니면 0 | `skin/property/IntegerPropertyFactory.java:939-944` |
| 스크롤 위치 | 슬라이더 type `7` (`skinselect_position`) | 읽기 `customOptionOffset / customOptionOffsetMax`, 쓰기 `(int)(max * value)` (`0 <= value < 1` 일 때) | `skin/property/FloatPropertyFactory.java:236-242`, `config/SkinConfiguration.java:82-90` |

`customOptionOffsetMax = max(0, customOptions.size() - skin.getCustomPropertyCount())` (`config/SkinConfiguration.java:217`). max 가 0 이면 읽기 식은 0/0 이 된다(부동소수 NaN — 코드상 방어 없음, `82-84`).

커스텀 항목 목록(`customOptions`) 구성 순서(`config/SkinConfiguration.java:198-224`):

1. 옵션(`updateCustomOptions`, `249-296`): 헤더의 `property` 마다 1항목. 현재 값은 저장된 설정 → `def` 이름 일치 → 첫 항목 순으로 결정. 선택지 끝에 `"Random"`(값 `OPTION_RANDOM_VALUE = -1`)을 추가한다.
2. 파일(`updateCustomFiles`, `298-352`): 헤더의 `filepath` 마다 1항목. 경로의 마지막 `/` 뒤를 글롭 패턴으로 디렉터리를 열거(대문자·소문자 패턴 둘 다), 끝에 `"Random"` 추가. `|` 가 있는 경로는 `|` 앞부분 + 마지막 `|` 뒤를 패턴으로 쓴다. 표시 값은 확장자를 뗀 파일명. 디렉터리가 없으면 항목을 만들지 않는다.
3. 오프셋(`updateCustomOffsets`, `354-381`): 헤더의 `offset` 마다 허용된 축(`x,y,w,h,r,a`)별로 1항목. 이름은 `"오프셋이름 - 축"`, 범위 `-9999 ~ 9999`, 표시 값은 숫자 문자열. 플레이 스킨에는 로더가 `"All offset(%)"`, `"Notes offset"`, `"Judge offset"`, `"Judge Detail offset"` 4개를 덧붙인다(`skin/json/JSONSkinLoader.java:183-190`).

값이 바뀔 때마다 설정 객체에 반영하고 미리보기를 다시 로드한다(`610-667`).

스킨 종류 변경(`changeSkinType`, `151-173`): 직전 스킨 설정을 이력에 저장 → 해당 종류의 헤더만 추림 → 현재 설정 경로와 같은 헤더가 있으면 선택, 없으면 `selectSkin(-1)`(이름 `""`, 항목 없음, 미리보기 없음).
스킨 변경(`setOtherSkin`, `179-196`): 후보가 없으면 경고만. 선택이 없던 상태면 0 번, 아니면 순환. 설정 경로를 바꾸고 속성을 비운 뒤, 이력에 같은 경로가 있으면 그 속성을 복원한다(`198-213`).

### 8.5 스킨 미리보기(`skinpreview`)

- 스키마: 최상위 `skinpreview = { id = "..." }` 와 같은 id 의 destination (`skin/json/JsonSkin.java:47`, `414-416`). `SKIN_SELECT` 타입에서만 객체가 만들어진다(`skin/json/JsonSkinConfigurationSkinObjectLoader.java:21-28`).
- 미리보기 스킨 로드(`config/SkinConfiguration.java:454-474`): 선택 스킨이 없거나 종류가 `SKIN_SELECT`, `RESULT`, `COURSE_RESULT` 이면 미리보기 없음. 그 외에는 현재 편집 중 설정으로 `SkinLoader.load(this, type, previewConfig)` 후 `preview.prepare(this)`. 예외는 삼키고 미리보기 없음으로 처리.
- 그리기(`config/SkinPreview.java:41-116`): 미리보기 스킨의 `width × height`(대상 해상도) 크기 프레임버퍼에 투명으로 클리어 후 `updateCustomObjects` + `drawAllObjectsSafely`(객체별 예외 시 그 객체만 비활성)를 수행하고, 그 텍스처를 상하 반전해 자신의 dst 사각형에 그린다. 그리기 중 예외가 나면 그 미리보기 스킨에 대해서는 이후 그리지 않는다.
- 미리보기 스킨의 프로퍼티는 `SkinConfiguration` 상태를 대상으로 평가된다(플레이 상태가 아니므로 곡·스코어 관련 값은 대부분 기본값).
- ModernChic skinselect 는 `skinpreview` 를 쓰지 않는다(썸네일 함수 호출도 주석 처리 — `ModernChic/skinselect.lua:99-102`, `163`).

### 8.6 ModernChic skinselect 가 실제로 쓰는 계약

`ModernChic/skinselect.lua:12-145`:
- 배경 이미지 1장.
- 종류 메뉴: `imageset {images = {off, on}, act = ref[i], ref = ref[i]}` 형식으로 `MUSIC_SELECT, DECIDE, RESULT, COURSE_RESULT, SOUND_SET, KEY_CONFIG, SKIN_SELECT, THEME` 8개와 플레이 스킨 `7KEY, BATTLE7, 14KEY, 5KEY, 10KEY, 9KEY, BATTLE9, 24KEY, 24KEY_BATTLE, 24KEY_DOUBLE` 10개.
- 항목 10행: 텍스트 `t_category1~10`, `t_item1~10`, 클릭 영역 `act = SKIN_CUSTOMIZE1~10`, `click = SEPARATE`, 좌우 커서 이미지와 `mouseRect` 호버 이미지.
- 스킨 변경 영역 `act = CHANGE_SKIN`, `click = SEPARATE`, 좌우 커서와 호버.
- 스킨 이름·제작자 텍스트.
- 슬라이더 `type = SKINSELECT_POSITION`, `range = 590`, `angle = 2`, `changeable = true`.

---

## 9. 화면 전이 요약

| 출발 | 조건 | 도착 | 근거 |
|------|------|------|------|
| 시작 | 일반 실행 | `MUSICSELECT` | `MainController.java:349-351` |
| 선택 | 곡/코스 시작 성공 | `DECIDE` (즉시) | `select/MusicSelector.java:426`, `527`, `284` |
| 선택 | `NUM6` 유지 또는 이벤트 13 | `CONFIG` | `select/MusicSelector.java:296-297`, `skin/property/EventFactory.java:223-227` |
| 선택 | `F12` 또는 이벤트 14 | `SKINCONFIG` | `select/MusicSelector.java:298-299`, `skin/property/EventFactory.java:231-235` |
| 선택 | `ESCAPE` | 앱 종료 | `select/MusicSelectInputProcessor.java:370-372` |
| 결정 | FADEOUT 완료, 취소 아님 | `PLAY` | `decide/MusicDecide.java:42-45` |
| 결정 | FADEOUT 완료, 취소 | `MUSICSELECT` | 같은 곳 |
| 결과 | FADEOUT 완료 | §5.2 표 | `result/MusicResult.java:172-248` |
| 코스 결과 | FADEOUT 완료 | `MUSICSELECT` | `result/CourseResult.java:179-183` |
| 키 설정 | `ESCAPE` | `MUSICSELECT` (설정 저장) | `config/KeyConfiguration.java:242-245` |
| 스킨 설정 | `ESCAPE` | `MUSICSELECT` (설정 저장) | `config/SkinConfiguration.java:63-66` |

---

## 10. 구현 시 재현해야 하는 엣지 케이스 목록

1. `scene` 생략 = 0 → 결정·결과가 즉시 FADEOUT (§1.5).
2. 선택 화면에서 `TIMER_SONGBAR_CHANGE` 는 화면 진입 첫 `input()` 에서 켜진다. 스킨이 이 타이머로 등장 연출을 하면 진입 직후와 막대 이동 때마다 재생된다(§3.3).
3. 패널 타이머는 "열림 = ON 재시작 + OFF 끔", "닫힘 = OFF 재시작 + ON 끔" 으로 항상 쌍으로 움직인다. 패널 1 → 2 로 바로 바뀌면 `PANEL1_OFF` 와 `PANEL2_ON` 이 같은 프레임에 켜진다(§3.3).
4. 막대 본체는 imageset 필수, off 도 `liston[i].id` 의 imageset 을 쓴다(§2.2).
5. 값 2(표/해시)에 `ExecutableBar` 와 `RandomCourseBar` 가 섞인다. `SameFolderBar` 는 그려지지 않는다(§2.4).
6. 스크롤 보간은 벽시계 기준, y 는 미클램프(§2.8).
7. 커서가 아닌 곡 막대를 클릭해도 커서 막대가 시작된다(§2.10).
8. 오른쪽 클릭(왼쪽 외 버튼)은 폴더 닫기, 루트에서는 정렬 변경(§2.10).
9. 필터 결과가 비면 필터가 자동으로 다음 값으로 바뀌고 저장된다(§3.9-4).
10. 결정 화면 건너뛰기는 눌린 상태 판정(§4).
11. 결과 화면은 IR 처리 중에는 닫히지 않는다(§5.3-5).
12. 결과 화면 FADEOUT 완료 순간에 5번/7번 키를 누르고 있으면 재플레이(§5.2).
13. `gaugegraph` 드러남은 화면 경과 시간 기준 1500ms, dst 타이머와 무관(§5.6).
14. 코스 결과는 `TIMER_RESULT_UPDATESCORE` 를 무조건 첫 프레임에 켠다(§6).
15. 키 설정 화면은 내장 UI + 스킨 덧그리기(§7).
16. 스킨 설정의 10번째 커스텀 버튼(229)은 동작하지 않는다(§8.3).
17. `skinSelect` 가 없으면 `customPropertyCount = -1` 이라 스크롤 최대치가 항목 수 + 1 이 된다. ModernChic 이 이 경우다(§8.2).
18. `create()` 중 켠 타이머는 `setMainState` 리셋으로 사라진다(§1.2).

---

## 11. 조사 중 확인한 ModernChic 관련 위험 신호(범위 밖이지만 화면 구현에 영향)

- `ModernChic/Result/lua/mainmenu.lua:5-7`, `986-989` 와 `ModernChic/Result/lua/irmenu.lua:6-8`, `654-657` 가 `luajava.bindClass("com.badlogic.gdx.Gdx")`, `"com.badlogic.gdx.Input"` 로 키보드(`LEFT/RIGHT/UP/DOWN`)를 직접 읽는다. `Root/customfunction.lua`, `Select/lua/require/http.lua` 에도 `luajava` 사용이 있다(파일별 건수만 확인, 내용 미확인).
- 결과 스킨은 `input = 2500` 이라 2.5초 동안 확인 입력이 막힌다(`ModernChic/Result/lua/require/header.lua:16`).

---

## 12. 읽은 범위와 읽지 못한 것

전부 읽은 파일(배정 목록): `MainState.java`, `MainController.java`, `TimerManager.java`, `SystemSoundManager.java`, `skin/json/JsonSelectSkinObjectLoader.java`, `JsonDecideSkinObjectLoader.java`, `JsonResultSkinObjectLoader.java`, `JsonCourseResultSkinObjectLoader.java`, `JsonKeyConfigurationSkinObjectLoader.java`, `JsonSkinConfigurationSkinObjectLoader.java`, `select/MusicSelectSkin.java`, `select/SkinBar.java`, `select/BarRenderer.java`, `select/BarManager.java`, `select/SkinDistributionGraph.java`, `select/MusicSelector.java`, `select/MusicSelectInputProcessor.java`, `select/MusicSelectCommand.java`, `select/PreviewMusicProcessor.java`, `select/bar/*.java`(14개 전부), `decide/MusicDecide.java`, `decide/MusicDecideSkin.java`, `result/AbstractResult.java`, `result/MusicResult.java`, `result/CourseResult.java`, `result/MusicResultSkin.java`, `result/CourseResultSkin.java`, `result/SkinGaugeGraphObject.java`, `result/ResultKeyProperty.java`, `config/KeyConfiguration.java`, `config/KeyConfigurationSkin.java`, `config/SkinConfiguration.java`, `config/SkinConfigurationSkin.java`, `config/SkinPreview.java`.

추가로 전부 읽은 파일: `select/MusicSelectKeyProperty.java`, `select/BarSorter.java`, `select/ModeFilter.java`, `select/DifficultyFilter.java`, `skin/SkinTextInput.java`, `skin/SkinPropertyMapper.java`, `skin/SkinType.java`, `skin/default/decide.json`, `skin/default/keyconfig/keyconfigmain.lua`, `skin/default/skinselect/skinselectmain.lua`, `ModernChic/keyconfig.lua`.

일부만 읽은 파일(근거로 인용한 구간만 확인): `skin/Skin.java`(60-499, 725-790), `skin/SkinObject.java`(596-612, 668-704), `skin/SkinImage.java`(90-181), `skin/SkinText.java`(95-175), `skin/SkinLoader.java`(1-140), `skin/SkinProperty.java`(grep 결과), `skin/json/JsonSkin.java`(필드 선언 전체와 410-557), `skin/json/JSONSkinLoader.java`(150-440), `skin/json/JsonSkinObjectLoader.java`(440-484, 610-672), `skin/lua/LuaSkinLoader.java`(60-209), `skin/property/EventFactory.java`(20-939), `skin/property/StringPropertyFactory.java`(30-110, 215-262, 322-440), `skin/property/BooleanPropertyFactory.java`(325-420, 485-502, 660-712), `skin/property/IntegerPropertyFactory.java`(236-252, 872-946), `skin/property/FloatPropertyFactory.java`(218-262, 445-462), `input/BMSPlayerInputProcessor.java`(175-504), `input/KeyBoardInputProcesseor.java`(256-296), `input/KeyCommand.java`, `Config.java`·`SkinConfig.java`·`ClearType.java`·`song/SongData.java`·`ir/RankingData.java`·`play/GrooveGauge.java`·`audio/AudioDriver.java`(grep 구간), `skin/default/select.json`(1-40, 92-150, 196-232, 284-296, 355-392, 420-430, 493-525, 750-760, 795-876), `skin/default/result.json`(1-84, 148-165), `ModernChic/Select/lua/songlist.lua`(1-140), `ModernChic/skinselect.lua`(1-169), `ModernChic/Result/lua/mainmenu.lua`(262-285).

읽지 못한 것(이 문서에서 다루지 않았거나 미확인으로 표시한 근거):
- `select/RandomCourseData.java`, `select/RandomStageData.java`, `select/ScoreDataCache.java` (배정 목록에 없음, 미열람).
- `skin/json/JsonSkinSerializer.java` — `act` 미지정 시 기본값, `skinSelect` 역직렬화 세부.
- `skin/SkinNumber.java`, `skin/SkinSlider.java`, `skin/SkinObject.java` 의 나머지 — 하위 객체가 `draw == false` 일 때 `draw(sprite, offsetX, offsetY)` 가 실제로 무엇을 그리는지.
- `skin/default/graderesult.json`, `skin/default/result/resultmain.lua`, `skin/default/decide/decidemain.lua` 의 본문(헤더와 gaugegraph 줄만 grep).
- `ModernChic/Select/lua/songlist.lua` 140줄 이후, `ModernChic/musicselect.lua`·`decide.lua`·`result.lua`·`course.lua` 본문.
- 플레이 화면(`play/BMSPlayer.java`)에서 결과 화면으로 넘어가는 조건과 `DECIDE` 사운드 정지 지점.
- 게이지 이력 샘플링 간격(플레이 화면 쪽 코드).
