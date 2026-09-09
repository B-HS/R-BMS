# beatoraja 스킨 시스템 — Rust 재구현 스펙 추출

조사일 2026-09-09. 대상: `/Users/gkn/beatoraja/src/bms/player/beatoraja/skin/**`, `play/Skin*.java`, `/Users/gkn/beatoraja/skin/default`.
비교 대상: `/Users/gkn/R-BMS/crates/rbms-render/src/skin.rs` (541 LOC).

---

## 0. 요약 (한 문단)

beatoraja 스킨은 **"SkinObject 배열 + destination 키프레임 애니메이션 + 정수 프로퍼티 ID"** 3요소로 구성된다.
객체 타입 약 20종, 프로퍼티 ID 968개(`SkinProperty.java` 전수 카운트: OPTION 287 / NUMBER 273 / TIMER 151 / BUTTON 74 / STRING 43 / FLOAT 40 / RATE 30 / BARGRAPH 22 / VALUE 16 / OFFSET 16 / SLIDER 9 / IMAGE 5 / EVENT 2).
로더는 3종(JSON / LR2 CSV / Lua)이며 **Lua 로더는 JSON 로더의 서브클래스**(`LuaSkinLoader extends JSONSkinLoader`, lua/LuaSkinLoader.java:30)라 실질적으로 **모델은 하나(`JsonSkin.Skin`)**다.
현 rbms `SkinConfig`는 "플레이 필드 색/좌표 상수 묶음"으로, beatoraja 스킨 모델과 **개념적으로 겹치는 부분이 사실상 없다**(임의 객체 배치·타이머·애니메이션·프로퍼티 참조 전무).

---

## 1. SkinObject 모델

### 1.1 공통 베이스 (`skin/SkinObject.java`, 875 LOC)

| 필드 | 라인 | 의미 |
|---|---|---|
| `offset: int[]` | :23 | 참조할 SkinOffset ID 배열(런타임에 x/y/w/h/r/a 가산) |
| `relative: boolean` | :25 | true면 offset의 x/y를 좌표에 더하지 않고 크기만 적용 |
| `dsttimer: TimerProperty` | :29 | 애니메이션 기준 타이머. `timer.isOff(state)`면 **비표시** (:352) |
| `dstloop: int` | :35 | 루프 시작 시각(ms). `-1`이면 루프 없음 |
| `dstblend: int` | :39 | 2=가산, 9=반전 (주석 그대로) |
| `dstfilter: int` | :43 | 0=Nearest, 1=Linear |
| `dstcenter: int` | :48 | 회전 중심 0~9 (`CENTERX/CENTERY` 룩업, :75-76) |
| `acc: int` | :50 | 보간 방식. 1=ease-in(t²), 2=ease-out(1-(t-1)²), 3=보간 없음(계단), 그 외 선형 (:558-566) |
| `clickevent: Event`, `clickeventType: int` | :54,:62 | 클릭 이벤트. 0=plus만,1=minus만,2=좌우분할,3=상하분할 |
| `dstop: int[]`, `dstdraw: BooleanProperty[]` | :67,:68 | 표시 조건(정수 옵션 ID / Lua 불린식). `dstdraw` 하나라도 false면 미표시 (:592-596) |
| `mouseRect: Rectangle` | :72 | 클릭 판정 영역 |
| `stretch: StretchType` | :76 | 이미지 늘이기 방식(기본 STRETCH) |
| `dst: SkinObjectDestination[]` | :89 | **키프레임 배열** |

`SkinObjectDestination` = `(time, Rectangle region, Rectangle clip, Color color, angle, acc)` (:190-191).

### 1.2 키프레임 애니메이션 정확한 의미 (`prepareRegion`, :348-)

1. `dsttimer`가 있으면 `time -= timer.get(state)` (:355). 타이머 OFF면 draw=false.
2. 루프: `dstloop == -1` → `time > endtime`이면 `time = -1`(=미표시).
   아니면 `time > dstloop`일 때 `time = (time - dstloop) % (endtime - dstloop) + dstloop` (:361-371).
   `endtime == dstloop`이면 `time = dstloop`로 고정.
3. `starttime > time`이면 draw=false (:372-375).
4. `getRate()` (:545-577): `dst[i].time <= nowtime < dst[i+1].time`인 구간 i를 뒤에서부터 탐색, `rate=(now-t1)/(t2-t1)`에 acc 커브 적용.
5. region/clip/color/angle을 각각 `v1 + (v2-v1)*rate`로 선형 보간 (:397-400, :452-455, :506-509, :537). **acc==3이면 보간 없이 dst[index] 값 고정**.
6. offset 반영: `region.x += off.x - off.w/2`, `region.width += off.w` (relative가 아닐 때만 x/y 가산) (:404-411). color는 `a += off.a/255`, angle은 `+= off.r`.
7. 최적화: dst가 1개뿐이면 `fixr/fixc/fixa/fixclip`로 캐시해 보간 스킵 (:100-103, :199-).

> 렌더 파라미터(blend/filter/stretch/center)는 **키프레임이 아니라 오브젝트 단위 고정값**이다. 즉 애니메이션되는 것은 x,y,w,h,clip,a,r,g,b,angle 뿐.

### 1.3 구상 타입 목록

| 클래스 | LOC | 주요 필드 | 역할 |
|---|---|---|---|
| `SkinImage` | 182 | `SkinSource[] image`, `IntegerProperty ref` | 이미지/이미지셋/무비. ref로 이미지 배열 인덱싱 |
| `SkinNumber` | 233 | `SkinSourceSet image/mimage`, `IntegerProperty ref`, keta/zeropadding/space/align, `shiftbase` | 숫자. mimage=마이너스용 별도 글리프 |
| `SkinFloat` | 239 | (SkinNumber 유사) iketa/fketa/gain/isSignvisible | 소수 표시 |
| `SkinText` | 226 | font/size/align/`StringProperty value`/`StringWriter event`/constantText/wrapping/overflow/outline·shadow | 텍스트 베이스 |
| `SkinTextBitmap` | 992 | LR2FONT 비트맵 폰트 | 최대 파일. LR2 폰트 호환 전용 |
| `SkinTextFont` | 595 | TTF(FreeType) | 폰트 렌더 |
| `SkinTextImage` | 251 | | 이미지 문자열 |
| `SkinTextInput` | 192 | editable 텍스트 | 검색창 등 |
| `SkinSlider` | 199 | source/direction/range/`FloatProperty ref`/`FloatWriter writer`/changeable | 드래그 가능 슬라이더 |
| `SkinGraph` | 113 | source/`FloatProperty ref`/direction/min/max | 바 그래프 |
| `SkinBPMGraph` | 241 | delay/lineWidth/main·min·max·other BPM 색/stop·transition 색 | 곡 BPM 분포 |
| `SkinNoteDistributionGraph` | 569 | type/backTexOff/delay/orderReverse/noGap | 노트 밀도 그래프 |
| `SkinTimingVisualizer` | 192 | width/judgeWidthMillis/PG·GR·GD·BD·PR 색/drawDecay | 판정 타이밍 실시간 |
| `SkinTimingDistributionGraph` | 149 | graph/average/dev 색 | 판정 분포 |
| `SkinHitErrorVisualizer` | 237 | colorMode/hiterrorMode/emaMode/alpha/windowLength | 히트에러 |
| `play/SkinNote` | 206 | `SkinLane[] lanes`, `LaneRenderer` | 노트 레인 전체 |
| `play/SkinGauge` | 256 | image/animationType/animationRange/duration/parts/type/starttime/endtime | 게이지 |
| `play/SkinJudge` | 146 | `SkinImage[7] judge`, `SkinNumber[7] count`, player, shift | 판정 표시(7단계) |
| `play/SkinBGA` | 64 | bgaExpand | BGA |
| `play/SkinHidden` | 138 | disapearLine/isDisapearLineLinkLift/timer/cycle | 레인커버·리프트커버 |
| `play/SkinPractice` | 161 | visibleItems, `SkinNoteDistributionGraph[]` | 연습모드 UI |
| `PomyuCharaLoader` | 580 | | pop'n 캐릭터(선택 구현) |

합계 스킨 패키지 9,590 LOC (`wc -l skin/*.java`).

### 1.4 SkinSource (이미지 소스)

`SkinSource.getImage(time, state) -> TextureRegion` 추상 (SkinSource.java:16). 구현 4종:
- `SkinSourceImage` (98 LOC) — 정지/애니메이션 이미지, timer+cycle로 프레임 선택
- `SkinSourceImageSet` (98 LOC) — `IntegerProperty value`로 이미지 집합 선택
- `SkinSourceMovie` (63 LOC) — 동영상(JSONSkinLoader.java:460 `new SkinSourceMovie(...)`)
- `SkinSourceReference` (58 LOC) — 다른 소스 참조
`SkinSourceSet.getImages(...) -> TextureRegion[]` 는 숫자 글리프 세트용 (SkinSourceSet.java:15).

### 1.5 프로퍼티 체계 (`skin/property/*`)

5개 read 인터페이스 + 2개 write + 이벤트:
`BooleanProperty`(op), `IntegerProperty`(number), `FloatProperty`(slider/bargraph rate), `StringProperty`(text), `TimerProperty`(timer, µs), `FloatWriter`/`StringWriter`(슬라이더·입력 반영), `Event`(클릭 액션). 각각 `*Factory.getXxxProperty(id)`로 정수 ID → 구현 매핑.

**카테고리별 개수(전수 grep):**

| 접두사 | 개수 | 대표 ID |
|---|---:|---|
| OPTION_ | 287 | 21~26 PANEL1~6, 32/33 AUTOPLAY OFF/ON, 80 NOW_LOADING, 81 LOADED, 191 STAGEFILE, 200~207 1P_AAA~F, 230~ 1P_0_9.., 920/921 플레이사이드 |
| NUMBER_ | 273 | 10 HISPEED_LR2, 71 SCORE, 74 TOTALNOTES, 75 MAXCOMBO, 310/311 HISPEED(+소수) |
| TIMER_ | 151 | 1 STARTINPUT, 2 FADEOUT, 3 FAILED, 10~16 SONGBAR/README, 21~36 PANEL ON/OFF, 40 READY, 41 PLAY, 42~45 GAUGE_INCLEASE/MAX, 46/47/247 JUDGE_1P/2P/3P, 48/49 FULLCOMBO, 50~ BOMB_1P_*, 140 RHYTHM, 143/144 ENDOFNOTE, 250~ HCN_ACTIVE, 348~352 SCORE_A/AA/AAA/BEST/TARGET, 446~448 COMBO |
| BUTTON_ | 74 | SKINSELECT_7KEY.., SKIN_CUSTOMIZE1~10 |
| STRING_ | 43 | 1 RIVAL, 2 PLAYER, 10 TITLE, 11 SUBTITLE, 13 GENRE, 14 ARTIST, 30 SEARCHWORD, 50/51 SKIN_NAME/AUTHOR, 100~109 CUSTOMIZE_CATEGORY, 110~ CUSTOMIZE_ITEM |
| FLOAT_ | 40 | |
| RATE_ | 30 | |
| BARGRAPH_ | 22 | 101 MUSIC_PROGRESS, 102 LOAD_PROGRESS, 103 LEVEL, 110~115 SCORERATE 계열, 140~147 RATE_PGREAT..EXSCORE |
| VALUE_ | 16 | JUDGE_1P_SCRATCH 등 키별 판정값 |
| OFFSET_ | 16 | 1/2 SCRATCHANGLE_1P/2P, 3 LIFT, 4 LANECOVER, 5 HIDDEN_COVER, 10 ALL, 30 NOTES_1P, 32 JUDGE, 33 JUDGEDETAIL, 199 MAX |
| SLIDER_ | 9 | 1 MUSICSELECT_POSITION, 4/5 LANECOVER, 6 MUSIC_PROGRESS, 17~19 볼륨, 20 PRACTICE_POSITION |
| IMAGE_ | 5 | |
| EVENT_ | 2 | CUSTOM_BEGIN=1000, CUSTOM_END=1999 |
| **합계** | **968** | |

**타이머 키 매핑 규칙** (`SkinPropertyMapper.java:7-67): key<10이면 `BASE + key + player*10`, 10<=key<100이면 `BASE_KEY10 + key-10 + player*100`. bomb/hold/hcnActive/hcnDamage/keyOn/keyOff 6종이 같은 규칙.

**CustomTimer / CustomEvent:**
- `TIMER_CUSTOM_BEGIN=10000 .. TIMER_CUSTOM_END=19999` (SkinProperty.java:175-176). 스킨이 **쓰기 가능한 것은 커스텀 타이머뿐**(`isTimerWritableBySkin`, SkinPropertyMapper.java:163-166 — "組み込みタイマーはゲームプレイに影響するため").
- `CustomTimer`(48 LOC)는 능동(Lua 함수가 매 프레임 값 계산) / 수동(외부에서 `setMicroTimer`) 두 모드. `isPassive()` = `timerFunc == null`.
- `EVENT_CUSTOM_BEGIN=1000..1999` (SkinProperty.java:1051-1052). `CustomEvent`(39 LOC)는 `condition` 충족 + `minInterval` ms 경과 시 자동 실행, 또는 ID로 명시 실행.

**custom option / file / offset** (스킨 설정 UI):
- `JsonSkin.Property{category,name,item[{name,op}],def}` — 사용자가 고르는 옵션(선택 시 그 `op` ID가 활성 옵션 집합에 들어감)
- `JsonSkin.Filepath{category,name,path,def}` — 와일드카드 경로를 사용자 선택 파일로 치환(`filemap`)
- `JsonSkin.Offset{category,name,id,x,y,w,h,r,a}` — 어떤 축을 사용자 조절 가능하게 할지

---

## 2. 3개 로더와 JSON 스키마

### 2.1 로더 비교

| 로더 | 파일 | 형식 | 비고 |
|---|---|---|---|
| JSON | `json/JSONSkinLoader.java` (+ `JsonSkin.java` 557 LOC, `JsonSkinSerializer.java` 371 LOC) | libgdx `Json`(관대한 JSON: 트레일링 콤마 허용) | `.json` |
| Lua | `lua/LuaSkinLoader.java` | **JSONSkinLoader 상속** (:30). Lua 스크립트가 테이블 반환 → `fromLuaValue(JsonSkin.Skin.class, value)` (:53) | `.luaskin` |
| LR2 | `lr2/LR2SkinCSVLoader.java` (899 LOC) + 화면별 6개 서브로더 | CSV. `CommandWord` 등록식, 약 100개 명령어 | 그 외 확장자 |

LR2 CSV 명령 전수(grep `new CommandWord("...")`, 6파일): SRC_IMAGE/IMAGESET/NUMBER/TEXT/SLIDER/BARGRAPH/BUTTON/GROOVEGAUGE(+_EX)/JUDGELINE/LINE/NOTE/MINE/LN_START·END·BODY(+ACTIVE/INACTIVE)/HCN_START·END·BODY(+ACTIVE/INACTIVE)·DAMAGE·REACTIVE/HIDDEN/LIFT/BGA/BPMCHART/NOTECHART/NOWJUDGE_1P~3P/NOWCOMBO_1P~3P/ONMOUSE/README/TIMING_1P/PM_CHARA_IMAGE/BAR_BODY·FLASH·LEVEL·LAMP·TITLE·RANK·MY_LAMP·RIVAL_LAMP·TROPHY·GRAPH·LABEL + 대응 DST_* + BAR_CENTER/BAR_AVAILABLE/IMAGE/IMAGESET/LR2FONT/INCLUDE/SAMPLEBMS/JUDGETIMER/FINISHMARGIN/DST_NOTE_EXPANSION_RATE/DST_PM_CHARA_*. 전처리 지시자: `#IF/#ELSEIF/#ELSE/#ENDIF/#SETOPTION`.

**중요: JSON 스킨도 Lua 식을 값으로 받는다.** `JsonSkinSerializer.java:100-110`이 `BooleanProperty/IntegerProperty/FloatProperty/StringProperty/TimerProperty/FloatWriter`에 `LuaScriptSerializer`를 등록 — 문자열이면 먼저 `XxxPropertyFactory.getXxxProperty(문자열)`(이름 기반 상수) 시도, 실패 시 `lua.loadBooleanProperty(문자열)`로 **Lua 컴파일**(:121-126). 실제 default 스킨 `play7.json`에 `"draw":"gauge() >= 75"` 가 등장한다. 즉 **JSON 파서만으로는 default 스킨조차 완전히 못 읽는다.**

### 2.2 JsonSkin.Skin 전체 스키마 (`json/JsonSkin.java:7-57`)

헤더: `type`(SkinType id, -1이면 무효) `name` `author` `w=1280` `h=720` `fadeout` `input` `scene` `close` `loadend` `playstart` `judgetimer=1` `finishmargin=0`
설정: `category[]` `property[]` `filepath[]` `offset[]`
리소스: `source[]{id,path}` `font[]{id,path,fallback[],type}`
객체 정의: `image[]` `imageset[]` `value[]` `floatvalue[]` `text[]` `slider[]` `graph[]` `gaugegraph[]` `judgegraph[]` `bpmgraph[]` `hiterrorvisualizer[]` `timingvisualizer[]` `timingdistributiongraph[]` `note`(NoteSet) `gauge` `hiddenCover[]` `liftCover[]` `bga` `skinpreview` `practice` `judge[]` `songlist` `pmchara[]` `skinSelect`
확장: `customEvents[]` `customTimers[]`
배치: `destination[]`

주요 내부 클래스 필드:
- `Image{id,src,x,y,w,h,divx=1,divy=1,timer,cycle,len,ref,act,click}` (:112-127)
- `ImageSet{id,ref,value:IntegerProperty,images[],act,click}` (:129-136)
- `Value{id,src,x,y,w,h,divx,divy,timer,cycle,align,digit,padding,zeropadding,space,ref,value:IntegerProperty,offset:Value[]}` (:138-157)
- `FloatValue{... fketa,iketa,gain=1.0,isSignvisible, value:FloatProperty}` (:159-181)
- `Text{id,font,size,align,ref,value:StringProperty,event:StringWriter,constantText,editable,wrapping,overflow,outlineColor="ffffff00",outlineWidth,shadowColor,shadowOffsetX/Y,shadowSmoothness}` (:183-201)
- `Slider{... angle,range,type,changeable=true,value:FloatProperty,event:FloatWriter,isRefNum,min,max}` (:203-223)
- `Graph{... angle=1,type,value:FloatProperty,isRefNum,min,max}` (:225-242)
- `GaugeGraph{id,color[], + 6종 BG색 + 6종 Line색 + borderlineColor,borderColor}` (:244-261)
- `JudgeGraph{id,type,backTexOff,delay=500,orderReverse,noGap,noGapX}` (:263-271)
- `BPMGraph{id,delay,lineWidth=2,main/min/max/otherBPMColor,stopLineColor,transitionLineColor}` (:273-283)
- `HitErrorVisualizer{id,width=301,judgeWidthMillis=150,lineWidth,colorMode=1,hiterrorMode=1,emaMode=1,lineColor,centerColor,PG/GR/GD/BD/PRColor,emaColor,alpha=0.1,windowLength=30,transparent,drawDecay=1}` (:285-306)
- `TimingVisualizer{id,width=301,judgeWidthMillis=150,lineWidth,lineColor,centerColor,PG..PRColor,transparent,drawDecay}` (:308-322)
- `TimingDistributionGraph{id,width,lineWidth,graphColor,averageColor,devColor,PG..PRColor,drawAverage=1,drawDev=1}` (:324-336)
- `NoteSet{id, note[],lnstart[],lnend[],lnbody[],lnbodyActive[],lnactive[], hcnstart[],hcnend[],hcnbody[],hcnactive[],hcnbodyActive[],hcndamage[],hcnbodyMiss[],hcnreactive[],hcnbodyReactive[], mine[],hidden[],processed[], dst:Animation[], dst2, expansionrate={100,100}, size[], group[],bpm[],stop[],time[]}` (:338-366)
- `Gauge{id,nodes[],parts=50,type,range=3,cycle=33,starttime=0,endtime=500}` (:368-377)
- `HiddenCover/LiftCover{id,src,x,y,w,h,divx,divy,timer,cycle,disapearLine=-1,isDisapearLineLinkLift}` (:379-403)
- `BGA{id}` / `SkinPreview{id}` / `Practice{id,visibleItems=10}` (:405-424)
- `Judge{id,index,images:Destination[],numbers:Destination[],shift}` (:426-432)
- `SongList{id,center,clickable[],listoff,liston,text,level,lamp,playerlamp,rivallamp,trophy,label,graph}` (:434-448)
- **`Destination{id,blend,filter,timer,loop,center,offset,offsets[],stretch=-1,op:DestinationOption[],draw:BooleanProperty,dst:Animation[],mouseRect}`** (:450-472)
- `DestinationOption{id:int | property:BooleanProperty}` (:475-489) — 숫자 op ID와 Lua 불린식을 한 배열에 혼재 가능
- **`Animation{time,x,y,w,h,clip_x,clip_y,clip_w,clip_h,acc,a,r,g,b,angle}` — 전부 기본값 `Integer.MIN_VALUE`(=미지정, 직전 키프레임 상속)** (:497-518)
- `PMchara{id,src,color=1,type,side=1}` / `SkinConfigurationProperty{customBMS[],defaultCategory,customPropertyCount=-1,customOffsetStyle}` / `CustomEvent{id,action:Event,condition:BooleanProperty,minInterval}` / `CustomTimer{id,timer:TimerProperty}` (:520-556)

### 2.3 default 스킨 실제 구성 — 포맷 판정 근거

`ls /Users/gkn/beatoraja/skin/default`: JSON 12개(총 430KB), Lua 4쌍(`.luaskin` + `*main.lua`), 이미지 다수.

`SkinConfig.java:169-181` (기본 스킨 경로 상수):

| SkinType | 기본 경로 | 포맷 |
|---|---|---|
| PLAY_7KEYS | `skin/default/play/play7.luaskin` | **Lua** |
| PLAY_5/14/10/9KEYS | `play5.json` / `play14.json` / `play10.json` / `play9.json` | JSON |
| MUSIC_SELECT | `select.json` | JSON |
| DECIDE | `decide/decide.luaskin` | **Lua** |
| RESULT | `result/result.luaskin` | **Lua** |
| COURSE_RESULT | `graderesult.json` | JSON |
| PLAY_24KEYS / DOUBLE | `play24.json` / `play24double.json` | JSON |
| KEY_CONFIG | `keyconfig/keyconfig.luaskin` | **Lua** |
| SKIN_SELECT | `skinselect/skinselect.luaskin` | **Lua** |

**판정**: 1차 지원 대상은 **JSON 스키마(= JsonSkin.Skin 모델)** 가 맞다. Lua 스킨도 파싱 결과는 동일 모델이고, LR2 CSV는 별도 100개 명령 파서라 비용이 훨씬 크다. 다만 위 표대로 **default 스킨의 7K 플레이·결과·판정·키설정 화면은 Lua 전용**이며, JSON 파일 안에도 `"draw":"gauge() >= 75"`(play7.json destination 첫 항목) 같은 Lua 식이 들어 있으므로 **"JSON만 지원"으로는 default 스킨 전체를 못 돌린다**. 커뮤니티 스킨 다수는 LR2 CSV이거나 JSON이며, 최소 실행 가능 집합은 "JSON 스키마 + 소형 Lua 식 평가기".

### 2.4 이미지 소스 로딩 규칙 (`SkinLoader.java:95-131`)

1. `filemap`(사용자가 고른 custom file)에 등록된 prefix로 시작하면 `path[0..lastIndexOf('*')] + filemap값 + 나머지`로 치환 (:97-105).
2. 그렇지 않고 경로에 `*`가 있으면: `*` 뒤를 확장자로 보고(`|`가 있으면 `*`~`|` + `|` 뒤를 합성) 해당 디렉터리를 스캔, 매칭 파일 중 **랜덤 1개** 선택 (:111-129).
3. 텍스처는 `PixmapResourcePool` 캐시 + 선택적 `.cim`(수정시각 포함 파일명) 캐시 (:139-179).

default `play7.json`의 `"source":[{"id":1,"path":"play/background/*.png"}, ...]` + `"filepath":[{"name":"Background","path":"play/background/*.png"}, ...]`가 이 규칙의 실사용 예다(파일 3개 중 랜덤/사용자 선택).

---

## 3. Rust 재구현 최소 실행 가능 집합

### 3.1 화면별 필수 객체 타입

| 화면 | 필수 | 선택 |
|---|---|---|
| play | Image, Value(Number), Text, NoteSet, Gauge, Judge, HiddenCover/LiftCover, BGA, Graph | FloatValue, Slider, TimingVisualizer, HitErrorVisualizer, NoteDistributionGraph, Practice, PMchara |
| select | Image, ImageSet, Value, Text, SongList, Graph, BPMGraph, JudgeGraph, Slider | preview |
| result | Image, Value, Text, GaugeGraph, JudgeGraph | TimingDistributionGraph |
| decide | Image, Text (default `decide.json`은 `source/font/image/text/destination`만) | - |
| keyconfig | Image, Text, (버튼용 Image + click event) | - |
| config(skinselect) | Image, ImageSet, Value, Text, Slider, `skinSelect` | - |

근거: `skin/default/*.json` 최상위 키 전수(본문 §2.3 조사) — `decide.json`은 12키, `result.json`/`graderesult.json`은 gaugegraph·judgegraph 포함, `select.json`은 songlist/bpmgraph/judgegraph/graph, play*.json은 note/gauge/judge/bga/hiddenCover/slider/graph/imageset 포함.

### 3.2 최소 프로퍼티/타이머 집합 (플레이 화면 기준)

- Timer: 1 STARTINPUT, 2 FADEOUT, 3 FAILED, 40 READY, 41 PLAY, 42~45 GAUGE_INCLEASE/MAX, 46/47 JUDGE, 48/49 FULLCOMBO, 50~69 BOMB_*P_*, 140 RHYTHM, 143/144 ENDOFNOTE, 446/447 COMBO. (+ HOLD/KEYON/KEYOFF는 `SkinPropertyMapper` 규칙으로 생성)
- Number: SCORE(71), TOTALNOTES(74), MAXCOMBO(75), HISPEED(310/311) 및 판정 카운트군.
- Option: 920/921(플레이 사이드), 32/33(AUTOPLAY), 80/81(LOADING/LOADED), 200~207(랭크), 21~26(PANEL).
- Offset: 3 LIFT, 4 LANECOVER, 5 HIDDEN_COVER, 10 ALL, 30 NOTES_1P, 32 JUDGE, 33 JUDGEDETAIL, 1/2 SCRATCHANGLE.
- Bargraph: 101 MUSIC_PROGRESS, 102 LOAD_PROGRESS, 110~115 SCORERATE.
- String: 10 TITLE, 11 SUBTITLE, 13 GENRE, 14 ARTIST, 2 PLAYER.

플레이 화면만 정확히 돌리는 데 필요한 프로퍼티는 대략 **100~150개**(전체 968개의 10~15%). select/result까지 포함하면 250~350개.

### 3.3 규모 추정

| 모듈 | 예상 LOC | 근거 |
|---|---:|---|
| JsonSkin 스키마 미러(serde) | 700~900 | JsonSkin.java 557 LOC + serde 어트리뷰트/기본값 |
| SkinObject 공통(dst 보간·루프·offset·조건) | 500~700 | SkinObject.java 875 LOC 중 렌더 백엔드 제외 |
| 구상 객체 12종(Image/Number/Float/Text/Slider/Graph/Note/Gauge/Judge/Hidden/BGA/SongList) | 1,800~2,500 | 원본 대응 클래스 합 약 2,400 LOC |
| 프로퍼티 레지스트리(정수 ID → 상태 조회) | 1,200~1,800 | SkinProperty 1,068 LOC + 4개 Factory |
| 그래프류 5종(BPM/NoteDist/Timing/TimingDist/HitError) | 800~1,200 | 원본 1,388 LOC |
| Lua 식 평가(mlua 또는 소형 인터프리터) | 300~600 | LuaScriptSerializer 경로 |
| 로더/리소스(와일드카드·filemap·텍스처 캐시) | 300~400 | SkinLoader.java 179 LOC |
| **합계(JSON+Lua식, LR2 제외)** | **5,600~8,100** | |
| LR2 CSV 로더 추가 시 | +1,500~2,500 | 원본 lr2/ 패키지 |

### 3.4 현 rbms와의 격차

`crates/rbms-render/src/skin.rs:12-64` `SkinConfig`는 `field_x/field_width/top_y/judge_y/note_height/key_color/.../bomb_duration_ms` 등 **약 40개의 스칼라 상수**다. beatoraja 스킨의 핵심(임의 개수 객체 배치, dst 키프레임 보간, 타이머 참조, 968개 프로퍼티 ID, 이미지 소스 분할/애니메이션, 조건부 표시)은 **하나도 구현되어 있지 않다**. `docs/PROCESS.md:176`도 "남은 UI 후속: 스킨 데이터화"로 미완을 인정하고 있어 문서-코드 불일치는 없다.

---

## 4. 미조사 범위

- `SkinTextBitmap`(992 LOC) LR2FONT 포맷 상세, `SkinTextFont` FreeType 파라미터.
- `PomyuCharaLoader`(580 LOC) 및 pop'n 캐릭터 스펙.
- `property/` 하위 각 Factory의 ID→구현 매핑 본문(대표 ID만 확인, 968개 전수 의미 확인은 미수행).
- LR2 CSV 각 명령의 인자 순서(명령 이름만 전수 확인, 파라미터 스펙 미확인).
- `SkinLuaAccessor`/`MainStatePropertyLuaApiExporter`가 Lua에 노출하는 API 전체 목록(`gauge()` 등 함수명 일부만 확인).
- `Skin.java`의 렌더 파이프라인(shader 6종, scissor clip) 상세 (:505-660).
- rbms `theme.rs`/`playfield.rs`의 현재 데이터 주도 범위 정밀 대조.
