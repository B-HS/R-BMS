# b5. beatoraja 속성 id 전수 목록과 R-BMS 대조

> 최종 갱신 2026-10-10 · 대응 단계: 웨이브 2A(Lua 런타임과 로더) 반영 · 본문은 기준 커밋 `9ce92bb` 시점 서술이며, 아래 "웨이브 … 반영 사항"이 최신 것부터 우선한다 · 색인과 갱신 규칙은 [README.md](README.md)

## 웨이브 2A 반영 사항 (2026-10-10)

Lua 5.2 런타임(`crates/rbms-skin/src/lua/`), `SkinHost`, Lua 값 변환기, 2패스 `.luaskin` 로더를 넣고 구 샌드박스(`skin.*`)를 삭제한 뒤의 상태다.

- (W2-0) b5-properties.md §2.5 첫 항목: '`SkinStateSource` 가 integer/float/string/timer/now_ms … 이미지 인덱스, 이벤트, writer 종류가 없다' 를 '계약 자리는 생겼다(`SkinHost::image_index`, `exec_event`, `write_rate`, `write_text`), 값 연결은 미구현' 으로. 셋째 항목 '구현 없는 음수 id 는 true' 는 구 어댑터에 한한 서술임을 밝히고 계약상 미구현은 부호와 무관하게 None 임을 적는다.
- (W2-0) b5-properties.md §1-7: 'R-BMS 는 `UNMAPPED_INTEGER=0`' 뒤에 '계약 센티널 `INTEGER_ABSENT = i32::MIN` 추가, 구 어댑터만 0 유지(W3-2 에서 전환)' 를 덧붙인다. §137행·§1045행의 `SkinStateSource::integer`/`::timer` 표기도 `SkinHost` 로.
- (W2-3) b5-properties.md §1 9번과 §2.4: 정적 분류가 이제 생성 표로 있다 — property/generated/names.rs 의 STATIC_OUTSIDE_SELECT 58개, STATIC_ON_RESULT 57개, STATIC_ALWAYS 2개(50, 51). 조회는 property::static_scope(id).holds_on(StaticScreen).
- (W2-3) b5-properties.md §2.2: 이름 조회의 R-BMS 대응 추가 — property::id_of_name(NameSpace, name). enum 이름표는 생성(BooleanType 216, ValueType 148, IndexType 62, RateType 31, FloatType 29, StringType 26, EventType 54), 번호 붙은 이름군은 property/names.rs 수기 표. '!' 접두는 음수 id 로 돌려준다.
- (W2-3) b5-properties.md §2.5: 'R-BMS 현재 구조 요약' 갱신 — 종류는 PropertyKind 5개 외에 NameSpace 7개(Boolean, Integer, ImageIndex, Rate, Float, Text, Event)가 생겼고, SkinStateSource 는 SkinHost 로 바뀌었으며 이미지 인덱스·이벤트·writer 가 계약에 있다. writer 가 있는 id 목록은 WRITABLE_RATES(1, 7, 8, 17, 18, 19, 20), WRITABLE_STRINGS(30).
- (W2-3) b5-properties.md §2.6: 집계 수치의 교차 확인 결과 추가 — reference_implements 로 센 구현 id 수가 OPTION 248, NUMBER 263, RateType 31, FloatType 40, STRING 188 로 이 절의 수치와 일치.
- (W2-3) b5-properties.md §9: 'R-BMS: 없음(이미지 인덱스 공간 자체가 없음)' 열을 '계약 있음(SkinHost::image_index, main_state.event_index), 값 연결은 앱 호스트 몫' 으로. event_index 는 원본에 인덱스 속성이 없는 id 에서 오류를 낸다.


작성 범위: beatoraja `skin/SkinProperty.java`, `skin/SkinPropertyMapper.java`, `skin/property/*`(Integer/Boolean/Float/String/Timer 팩토리, EventFactory, 인터페이스 8개)를 끝까지 읽고, R-BMS `crates/rbms-skin/src/property/{mod.rs,generated/*}`, `timer/generated.rs`, `tools/gen-skin-{property,timer}.rs`, `crates/rbms-render/src/skin_render/{state.rs,screen.rs}`, `apps/rbms-player/src/skin_screen.rs` 및 타이머 호출부와 대조했다. 값의 계산 근거를 정확히 적기 위해 `ScoreDataProperty.java`, `MainState.java`, `TimerManager.java`, `JudgeManager.java`/`KeyInputProccessor.java`/`LaneRenderer.java`/`BMSPlayer.java`/`MusicSelector.java`/`MusicResult.java`/`MusicDecide.java`의 관련 줄, `SkinObject.java`/`Skin.java`/`SkinNumber.java`/`SkinSlider.java`의 소비 규칙, Lua 노출 계층(`MainStatePropertyLuaApiExporter.java`, `LuaSkinLoader.java`, `JsonSkinSerializer.java`)도 읽었다. 비교 표본으로 ModernChic `Root/main*.lua`의 id 표와 `MAIN.*` 참조를 전체 Lua(126개)에서 집계했다.

표기 규칙: `파일:줄`은 근거, `MIN_VALUE`는 `Integer.MIN_VALUE`(-2147483648), `Float.MIN_VALUE`는 양의 최소 float(1.4e-45)다. 식별자와 id 숫자는 원문 그대로다. 표의 `ModernChic 사용`은 `MAIN.<표>.<이름>` 참조를 id로 환산한 횟수와 사용 폴더(Play/Select/Result/Decide/Root 또는 최상위 진입 파일)이며 괄호 안은 역할(ref=표시 참조, act=클릭, script=main_state 호출, timer=타이머 필드, offset, op=조건 배열)이다.

---

## 1. 구현 사양에 직접 영향을 주는 핵심 결론

1. **R-BMS의 id 상수표는 완전하다.** 생성 표 968개(OPTION 287, NUMBER 273, TIMER 151, BUTTON 74, STRING 43, FLOAT 40, RATE 30, BARGRAPH 22, VALUE 16, OFFSET 16, SLIDER 9, IMAGE 5, EVENT 2)는 `SkinProperty.java`와 이름과 값이 전부 일치한다(차이 0). 부족한 것은 id가 아니라 **값 연결**이다.
2. **상수 목록은 구현 목록이 아니다.** 팩토리가 실제로 값을 계산하는 id는 상수와 다르다. 상수만 있고 구현이 없는 id: NUMBER 29개, OPTION 67개, RateType 1개(146). 반대로 상수가 없는데 구현된 id: NUMBER 19개(ranking 2..9번, 525-527), OPTION 28개(practice item), STRING 145개(패턴 id와 3, 60-62, 86). 구현 사양의 기준은 이 보고서의 3~9장 표다.
3. **R-BMS가 실제로 값을 채우는 id는 극히 일부다.** beatoraja 팩토리 구현 대비: NUMBER 263개 중 34개, OPTION 248개 중 34개, RateType 31개 중 12개, FloatType 40개 중 4개, STRING 188개 중 일부(제목/아티스트/검색어/디렉터리 + 키컨피그 이름). `property/mod.rs`의 `MAPPINGS`/`source_of`/`UnmappedLog`는 어디에서도 호출되지 않는 선언일 뿐이어서(grep 결과 소비처 없음) "등록만" id가 NUMBER 64, OPTION 148, RateType 18, FloatType 30, STRING 43개다.
4. **ModernChic 사용 id 기준 격차.** 숫자 177종 중 30종, 불리언 139종 중 23종, 문자열 60종 중 7종, 타이머 88종 중 이름표 기준 구동 75종(대부분 키 입력/폭탄 타이머), 슬라이더 7종 중 1종만 R-BMS가 값을 채운다(15장).
5. **이미지 참조 인덱스(`ref`/`event_index`) 공간이 R-BMS에 없다.** beatoraja는 NUMBER와 분리된 `IndexType` 공간(IntegerPropertyFactory.java:920-974)을 쓰지만 R-BMS는 이미지 `select`를 `state.integer()`로 읽는다(object.rs:280). ModernChic는 `ref = MAIN.BUTTON.*`을 42회, `main_state.event_index`를 14회 쓴다. 같은 번호가 NUMBER에서 다른 뜻이거나 무값이어서 전부 잘못 선택된다.
6. **이벤트/쓰기 계층이 R-BMS에 없다.** `BUTTON_*` 클릭 이벤트(EventFactory), 슬라이더 드래그용 `FloatWriter`, 검색어용 `StringWriter`, `main_state.event_exec`, 커스텀 이벤트(1000-1999) 대응이 없고 R-BMS는 이름 붙은 hotspot 액션(search/sort/folders/tables/records/settings/modal-*)만 있다.
7. **"값 없음" 의미가 다르다.** beatoraja는 구현 없는 NUMBER와 데이터 없는 NUMBER 모두 `MIN_VALUE`를 돌려 숫자 객체를 아예 그리지 않는다. ModernChic Lua는 이 값을 직접 검사한다(`main_state.number(MAIN.NUM.SCORE2) == -2147483648`, Play/lua/sp/attack.lua:61,148,164,180, Result/lua/irmenu.lua:241,484,541, Select/lua/sidemenu.lua:404,634). R-BMS는 `UNMAPPED_INTEGER=0`(mod.rs:86)과 `unwrap_or_default()`로 0을 그린다.
8. **타이머 구동이 beatoraja와 다르다.** 상세는 7장. 요지: beatoraja가 켜는 타이머 중 R-BMS가 안 켜는 것 = STARTINPUT(1), FADEOUT(2), PANEL2/3 ON/OFF(22,23,32,33), RHYTHM(140), ENDOFNOTE_1P(143), IR_CONNECT_*(172-174), JUDGE_3P/COMBO_3P(247,448), SCORE_A/AA/AAA/BEST/TARGET(348-352), HCN 계열(250-289,1810-1999,2010-2199), 포뮤 캐릭터(900-909). R-BMS가 켜는데 beatoraja는 켜지 않는 것 = SONGBAR_MOVE(10), SONGBAR_MOVE_UP/DOWN(12,13), GAUGE_INCLEASE_1P(42). 의미가 다른 것 = READY(R-BMS는 플레이 시작에 끔, beatoraja는 켜 둔 채 유지), RESULTGRAPH_END(R-BMS는 1초 뒤, beatoraja는 즉시), RESULT_UPDATESCORE(R-BMS는 신기록일 때, beatoraja는 skin rankTime/확인 입력 기준), BOMB(R-BMS는 연소 구간이 끝나면 끔, beatoraja는 켜 둔 채 재시작만).
9. **정적 평가 규칙이 있다.** `TYPE_STATIC_WITHOUT_MUSICSELECT/ON_RESULT/ALL` 불리언은 `Skin.prepare` 시점에 한 번 평가해 false면 객체를 영구 제거한다(Skin.java:197-229). R-BMS dst는 draw 조건을 `state.boolean(id)`로 그때그때 평가한다(dst.rs:438). 곡 정보 기반 옵션(stagefile/banner/BGA 유무, 난이도, 키 수, 코스 스테이지)과 결과 랭크 옵션은 평가 시점(준비 시 1회 vs 매프레임)을 beatoraja에 맞출지 결정해야 한다. 값이 준비 이후 바뀌는 경우(이미지가 늦게 로드되는 stagefile/banner 등)에만 결과가 갈린다.
10. **구현 없는 OPTION id의 의미.** 팩토리가 null이면 id는 스킨 커스터마이즈 옵션 맵과 대조되고 일치하지 않으면 객체가 제거된다(SkinObject.java:282-298, Skin.java:217-229). R-BMS는 false(음수는 true)로 답한다. 특히 OPTION 242-246/262-266(판정 GREAT 이하 개별)·44/45/1047(2P 게이지 종류)은 beatoraja에 구현이 없는데 R-BMS는 값을 돌려준다.
11. **판정/랭크 불리언의 정의가 다르다.** 1242/1243(early/late)은 beatoraja에서 "PGREAT가 아닌 판정의 FAST/SLOW"이고 R-BMS는 PG를 포함한다. 플레이의 `rank_now_*`(340-347)는 beatoraja가 통과 노트 기준 rate(ScoreDataProperty.nowrate), R-BMS는 전체 노트 기준이다. `FLOAT_SCORE_RATE`(1102)도 같은 이유로 다르고, `FLOAT_GROOVEGAUGE_1P`(1107)는 beatoraja 0..100, R-BMS 0..1이다. `NUMBER_GROOVEGAUGE_AFTERDOT`(407)는 beatoraja 소수 1자리, R-BMS 2자리다.
12. **R-BMS KeyConfig 문자열 결함.** `KeyConfigViewState::string`은 id 240 이상이면 `id-240`을, 40 이상이면 `id-40`을 레인 인덱스로 쓴다(state.rs:766-772). beatoraja는 확장 키 이름(240-283)을 `getKeyAssign(id-240+10)`으로 읽는다(StringPropertyFactory.java:81). R-BMS는 10칸 어긋나고, 50-239 범위(스킨 이름/커스터마이즈/랭킹/코스/타겟 등)에도 키 라벨을 돌려준다.
13. **beatoraja의 알려진 결함도 동작 호환 대상이다.** `isSkinCustomizeButton`이 229(BUTTON_SKIN_CUSTOMIZE10)를 제외(SkinPropertyMapper.java:124-126), `RateType.level` 계열의 switch fall-through로 최대 레벨이 항상 10(FloatPropertyFactory.java:574-586), `ir_busy`(608)가 `ir_failed`(604)와 같은 식, `no_save_clear`(62)가 미구현(`enable_save_score`와 같은 값), `OFFSET_JUDGE_1P/2P/3P`(=32)와 `OFFSET_JUDGEDETAIL_*`(=33)의 id 중복.
14. **오프셋은 5개만 갱신된다.** beatoraja가 런타임에 쓰는 오프셋은 1,2(스크래치 각도 r), 3(LIFT y), 4(LANECOVER y), 5(HIDDEN_COVER y/a)뿐이고 나머지 id는 사용자 설정이다. R-BMS는 3,4,5만 계산하고 스크래치 각도(1,2)는 없다.
15. **시간 단위.** Lua `main_state.time()`/`timer()`는 마이크로초이고 꺼진 타이머는 `Long.MIN_VALUE`(`main_state.timer_off_value`)다(MainStatePropertyLuaApiExporter.java:39,225-250). ModernChic는 `/1000000`과 `timer_off_value` 비교를 쓴다(Root/customnumber.lua:282, customoption.lua:45,100,169). R-BMS `skin.time()`은 ms, `skin.timer()`는 켜진 시각 `Option<i64>`(lua.rs:208-209)다.

---

## 2. 공통 규칙

### 2.1 id 공간은 종류별로 독립이다

| 종류 | 소비 객체/함수 | 팩토리 함수 | 비고 |
|---|---|---|---|
| 정수 NUMBER | SkinNumber `ref`, Lua `main_state.number/numbers`, `RateProperty(type,min,max)` | `IntegerPropertyFactory.getIntegerProperty(int\|String)` | 무값 `MIN_VALUE` |
| 불리언 OPTION | 모든 SkinObject `op`(draw 조건), Lua `main_state.option` | `BooleanPropertyFactory.getBooleanProperty(int\|String)` | 음수 id = 부정, 정적 구분 |
| 실수 RATE | SkinSlider `type`, SkinGraph `type`, Lua/JSON `FloatProperty` 필드 | `FloatPropertyFactory.getRateProperty` | 슬라이더/그래프는 이 공간만 |
| 실수 FLOAT | SkinFloat `ref`, Lua `main_state.float_number` | `FloatPropertyFactory.getFloatProperty` (FloatType 우선, 없으면 Rate) | 무값 `Float.MIN_VALUE` |
| 문자열 STRING | SkinText `ref`, Lua `main_state.text` | `StringPropertyFactory.getStringProperty` | 쓰기 `getStringWriter` |
| 타이머 TIMER | 모든 SkinObject `timer`, Lua `main_state.timer*` | `TimerPropertyFactory.getTimerProperty` | `timer>0`일 때만 사용 |
| 이벤트 BUTTON | SkinObject `act`/`click`, Lua `event_exec` | `EventFactory.getEvent(int\|String)` | arg1 부호=방향 |
| 이미지 인덱스 | SkinImage/ImageSet `ref`, Lua `event_index` | `IntegerPropertyFactory.getImageIndexProperty` | NUMBER와 별개 공간 |
| 오프셋 OFFSET | SkinObject `offset` 배열 | `MainController.getOffset(i)` (0<i<200) | x,y,w,h,r,a |

같은 숫자라도 종류가 다르면 다른 속성이다(예: 370은 NUMBER_CLEAR, 이미지 인덱스 `cleartype`, 둘이 우연히 같은 값을 돌려줄 뿐 별개 구현, 400은 NUMBER judgerank / 인덱스 constant / OPTION constant).

### 2.2 Lua/JSON 스킨의 속성 필드 해석

`LuaSkinLoader.serializeLuaScript`(LuaSkinLoader.java:152-165)와 `JsonSkinSerializer`는 속성 필드 값을 다음 순서로 해석한다: Lua 함수이면 프레임마다 호출하는 스크립트 프로퍼티, 숫자이면 `byId` 팩토리, 문자열이면 먼저 `byName`(enum 이름 또는 패턴 이름, 예 `"score"`, `"!autoplay_on"`, `"practice_item3_value"`) 조회 후 실패하면 Lua 식 문자열로 컴파일. 단 `TimerProperty`의 문자열 조회는 `null`(이름 불가)이고 `StringWriter`는 id 조회가 없다. `DestinationOption`(draw 조건)은 숫자면 `int` 옵션 배열로 모아 `SkinObject.setDrawCondition(int[])`(팩토리 구현이 있으면 `BooleanProperty`, 없으면 스킨 옵션 맵용 정수로 분리)를, 이름/스크립트면 `BooleanProperty[]`를 추가한다. 이 draw 조건은 **객체의 첫 destination 키프레임에서만 한 번 설정**되고(`if (dstop.length == 0 && dstdraw.length == 0)`, SkinObject.java:162-190) 이후 키프레임의 op는 무시된다. FloatProperty 필드는 `getRateProperty`(RateType)만 쓰므로 `FLOAT_*` 전용 id를 슬라이더/그래프에 줘도 null이다.

### 2.3 해석 실패와 센티널

| 종류 | 구현 없음 | 데이터 없음 | 소비자 동작 |
|---|---|---|---|
| NUMBER | 팩토리 null -> SkinNumber는 `MIN_VALUE`로 취급 | 구현은 있으나 값이 없으면 `MIN_VALUE` | SkinNumber는 `MIN_VALUE` 또는 `MAX_VALUE`면 그리지 않음 (SkinNumber.java:139); Lua `number()`는 null이면 0, 있으면 `MIN_VALUE` 그대로 |
| OPTION | null -> `Skin.option` 맵 대조 후 제거 | false | Lua `option()`은 null이면 false(음수 id도) |
| RATE/FLOAT | null | 0 또는 `Float.MIN_VALUE` | SkinSlider는 0, SkinFloat은 `Float.MIN_VALUE/MAX_VALUE/무한/NaN`이면 미표시 (SkinFloat.java:155) |
| STRING | null | 빈 문자열 | Lua `text()`는 null이면 "" |
| TIMER | 범위 밖은 커스텀 타이머 조회 | 꺼짐 = `Long.MIN_VALUE` | 객체: 타이머 꺼짐이면 그리지 않음 (SkinObject.java:352-355) |

### 2.4 불리언 정적 평가

구분과 영향은 4장 서두와 1장 9번 참고. 정적 평가는 `Skin.prepare`(스킨 준비) 시점 한 번이고, 정적 속성이 true인 객체는 이후 이 조건을 다시 검사하지 않는다.

### 2.5 R-BMS 현재 구조 요약

- 종류는 `PropertyKind` 5개(Boolean/Integer/Float/String/Timer)뿐이며 `SkinStateSource`가 `integer/float/string/timer/now_ms`, 상위 `DrawStateSource::boolean`, `OffsetSource::offset`을 제공한다(mod.rs:134-154, dst.rs). 이미지 인덱스, 이벤트, writer 종류가 없다.
- 구현체는 화면별 어댑터 5개: `PlayViewState`(state.rs:200), `SelectViewState`(:377), `ResultViewState`(:507), `DecideViewState`(:649), `KeyConfigViewState`(:732). 모든 어댑터의 `timer()`는 `None`이고 실제 타이머는 `TimerState`(timer.rs)가 별도 인자로 전달된다.
- 어댑터는 자기 화면 id만 답하고 나머지는 `UNMAPPED_*`(false/0/0.0/"")를 돌려준다. 음수 불리언 id는 모든 어댑터에서 `!answer`이므로 구현 없는 음수 id는 true가 된다(beatoraja Lua는 false).
- R-BMS 전용 확장 id가 어댑터에 섞여 있다: 20001-20011(결과 텍스트), 20101-20161(선곡 옵션 패널 행 라벨/값/포커스), 20201-20205(플레이 목표 이름/차이, 선곡/결과 힌트, IR 상태), 20301-20316(선곡 상세 통계/기록 칸) (state.rs:101-148). beatoraja id 공간 밖이므로 ModernChic와 충돌하거나 쓰이지 않는다. 기본 스킨에서 유지할지는 결정이 필요하다.
- 플레이/결과 어댑터는 `ResultView`/`HudView`가 가진 값만 쓴다(EX, 콤보, 판정 수, 게이지, best_ex, target_ex 등). 플레이어 누적 통계, IR, 라이벌, 날짜, 곡 메타(난이도/키 수/BGA 유무/밀도 등)는 어댑터가 받지 않는다.

### 2.6 대조 결과 요약 (자동 집계)

아래 집계는 id 단위이며 "beatoraja 구현"은 팩토리가 값을 계산하는 id다(상수 유무와 무관). 개별 id의 상태는 3~9장 표의 `R-BMS` 열에 있다.

### 정수 NUMBER

- beatoraja 구현 id 263개 중 R-BMS가 값을 채우는 것: **34개** (14, 71-72, 74-75, 90-92, 96, 107, 110-114, 121, 150, 152-153, 160-165, 310-311, 313, 407, 410-411, 420, 423-424)
- 등록만(MAPPINGS에 라우팅 선언, 값 미연결): **64개** (10, 12, 17-19, 30-37, 45-49, 57-59, 76-89, 100-103, 105-106, 108, 115-116, 122-123, 128, 135-136, 151, 154-158, 170-178)
- 선언은 되어 있으나 라우팅도 없음: **146개** (20-29, 179-180, 182-184, 200, 202-220, 222-249, 271, 280-289, 300, 312, 314-316, 320-330, 333, 350-353, 360-365, 368, 370-377, 380, 389-390, 399-400, 412-419, 421-422, 425-427, 1163-1164, 1312-1327)
- R-BMS 생성 표에 id 자체가 없음(미선언): **19개** (381-388, 391-398, 525-527)
- 구현이 없는 상수(beatoraja 팩토리에도 없음): 104, 181, 201, 272, 274-279, 354, 450-466, 469
- beatoraja에는 구현이 없는데 R-BMS만 응답하는 id: 104

### 불리언 OPTION

- beatoraja 구현 id 248개 중 R-BMS가 값을 채우는 것: **34개** (1-2, 21, 32-33, 42-43, 80-81, 90-91, 241, 261, 300-307, 340-347, 1046, 1242-1243, 1262-1263)
- 등록만(MAPPINGS에 라우팅 선언, 값 미연결): **148개** (3, 5, 22-23, 40-41, 50-51, 60-62, 100-105, 118-119, 121, 125-131, 170-184, 190-195, 200-207, 220-227, 230-240, 270-273, 280-283, 289-290, 320-327, 330-332, 335-336, 352-354, 361, 400, 603-604, 606, 608, 1002-1008, 1010-1017, 1030-1031, 1100-1104, 1128-1131, 1177, 1240, 1330-1332, 1335-1336, 1362-1363, 2241-2246)
- 선언은 되어 있으나 라우팅도 없음: **38개** (82, 84, 150-155, 160-164, 196-198, 624-625, 1080, 1160-1161, 1196-1208, 3000, 3015, 3020, 3035)
- R-BMS 생성 표에 id 자체가 없음(미선언): **28개** (3001-3014, 3021-3034)
- 구현이 없는 상수(beatoraja 팩토리에도 없음): -1, 30-31, 34-39, 44-45, 63-66, 70-79, 83, 120, 124, 210-217, 242-246, 262-263, 291-293, 308, 310-318, 333-334, 350-351, 362-363, 601-602, 605, 607, 1047
- beatoraja에는 구현이 없는데 R-BMS만 응답하는 id: 44-45, 242-246, 262-266, 1047

### 실수 RateType

- beatoraja 구현 id 31개 중 R-BMS가 값을 채우는 것: **12개** (1, 6, 101-102, 113, 115, 140-144, 147)
- 등록만(MAPPINGS에 라우팅 선언, 값 미연결): **18개** (4-5, 7-8, 17-19, 103, 105-112, 114, 145)
- 선언은 되어 있으나 라우팅도 없음: **1개** (20)
- R-BMS 생성 표에 id 자체가 없음(미선언): **0개** ()
- 구현이 없는 상수(beatoraja 팩토리에도 없음): 146
- beatoraja에는 구현이 없는데 R-BMS만 응답하는 id: 없음

### 실수 FloatType/IR

- beatoraja 구현 id 40개 중 R-BMS가 값을 채우는 것: **4개** (135, 165, 1102, 1107)
- 등록만(MAPPINGS에 라우팅 선언, 값 미연결): **30개** (85-89, 155, 157, 183, 203, 205, 207, 209, 211, 213, 215, 217, 219, 223, 225, 227, 229, 310, 360, 362, 367-368, 372, 374, 376, 1115)
- 선언은 되어 있으나 라우팅도 없음: **6개** (122, 285-289)
- R-BMS 생성 표에 id 자체가 없음(미선언): **0개** ()
- 구현이 없는 상수(beatoraja 팩토리에도 없음): 없음
- beatoraja에는 구현이 없는데 R-BMS만 응답하는 id: 없음

### 문자열 STRING

- beatoraja 구현 id 188개 중 R-BMS가 값을 채우는 것: **62개** (10-14, 16, 30, 40-49, 240-283, 1000)
- 등록만(MAPPINGS에 라우팅 선언, 값 미연결): **43개** (1-2, 15, 50-51, 100-129, 1001-1003, 1010, 1020-1021, 1030-1031)
- 선언은 되어 있으나 라우팅도 없음: **16개** (150-159, 1040, 1055, 1060, 1075, 1080, 1095)
- R-BMS 생성 표에 id 자체가 없음(미선언): **67개** (3, 60-62, 86, 200-219, 1041-1054, 1061-1074, 1081-1094)
- 구현이 없는 상수(beatoraja 팩토리에도 없음): 없음
- beatoraja에는 구현이 없는데 R-BMS만 응답하는 id: 없음

### 타이머 TIMER

- 상수 151개 중 경계 상수 3개를 뺀 148개. `SkinPropertyMapper` 식으로 만들어지는 id(상수로 선언되지 않은 중간 키 포함)까지 합쳐 beatoraja가 실제로 켜는 id는 **1243개**(1-3, 11, 21-23, 31-33, 40-41, 44, 46-48, 50-89, 100-140, 143, 150-152, 172-174, 247, 250-289, 348-352, 446-448, 900-909, 1010-1099, 1110-1199, 1210-1299, 1310-1399, 1410-1499, 1510-1599, 1610-1699, 1710-1799, 1810-1899, 1910-1999, 2010-2099, 2110-2199); 나머지는 선언만 되어 있고 beatoraja에서도 항상 꺼져 있다.
- R-BMS가 `TimerState`에 쓰는 id는 **819개**(3, 10-13, 21, 31, 40-42, 44, 46-48, 50-89, 100-139, 150-152, 446-447, 1010-1099, 1110-1199, 1210-1299, 1310-1399, 1410-1499, 1510-1599, 1610-1699, 1710-1799).
- beatoraja가 켜는데 R-BMS가 안 켜는 id: 1-2, 22-23, 32-33, 140, 143, 172-174, 247, 250-289, 348-352, 448, 900-909, 1810-1899, 1910-1999, 2010-2099, 2110-2199
- R-BMS가 켜는데 beatoraja는 켜지 않는 id: 10, 12-13, 42

---

## 3. 정수 속성 (NUMBER_*)

조회 함수: `IntegerPropertyFactory.getIntegerProperty(int)` (IntegerPropertyFactory.java:35). 해석 순서는 캐시 -> `IntegerPropertyPattern`(VALUE 범위: ranking/IR/folder) -> `ValueType.getProperty(id)`(범위 id 처리 후 enum 선형 탐색). `id<0` 또는 `id>=65536`이면 null. 구현이 없는 id는 null이며 `SkinNumber`에서 값 MIN_VALUE로 취급되어 아무것도 그리지 않는다. "무값"은 `Integer.MIN_VALUE`(-2147483648)이고 `SkinNumber.prepare`는 MIN_VALUE 또는 MAX_VALUE를 받으면 draw=false (SkinNumber.java:139). Lua `main_state.number(id)`는 MIN_VALUE를 그대로 Lua 정수로 돌려주고(`MainStatePropertyLuaApiExporter.java:155`), null 프로퍼티는 0을 돌려준다.

R-BMS 열: `구현(플/선/결/결정/키)` = 해당 화면의 `SkinStateSource::integer`가 값을 돌려줌(state.rs), `등록만(원천)` = `MAPPINGS`(property/mod.rs:279)에 라우팅만 선언되고 어떤 화면 어댑터도 값을 채우지 않음, `없음` = 선언된 상수이나 라우팅도 없음, `미선언` = R-BMS 생성 표에 id 자체가 없음. 미구현 id는 R-BMS에서 `UNMAPPED_INTEGER=0`을 돌려(mod.rs:86) 0이 그려진다. beatoraja는 아무것도 그리지 않는다. draw.rs:56,264는 i32::MIN/i32::MAX를 무값으로 처리하는 경로가 있으나 어떤 어댑터도 그 값을 돌려주지 않는다(최고기록 없음도 `unwrap_or_default()`로 0).

| id | 상수 | beatoraja 구현(enum/패턴 이름) | 화면 | 값 계산 근거 | 쓰기 | R-BMS | ModernChic 사용 |
|---|---|---|---|---|---|---|---|
| 10 | NUMBER_HISPEED_LR2 | hispeed_lr2 | 플레이,그 외 | BMSPlayer는 LaneRenderer.getHispeed, 그 외 화면은 songdata 모드의 PlayConfig.getHispeed. 값=(int)(hispeed*100). 곡 없으면 MIN_VALUE | - | 등록만(PlayerConfig) |  |
| 12 | NUMBER_JUDGETIMING | notesdisplaytiming | 공통 | PlayerConfig.getJudgetiming() (판정 타이밍 보정 ms) | - | 등록만(PlayerConfig) | 3회 [Play,Select] (ref3) |
| 14 | NUMBER_LANECOVER1 | lanecover1 | 플레이 | (int)(LaneRenderer.getLanecover()*1000), 플레이 외 MIN_VALUE | - | 구현(플) **불일치: R-BMS 값=hud.white_number(흰 숫자), beatoraja=lanecover*1000(천분율). 의미 다름** | 5회 [Play,Root] (script3,ref2) |
| 17 | NUMBER_TOTALPLAYTIME_HOUR | playtime_total_hour | 공통 | PlayerData.getPlaytime()(초)/3600 | - | 등록만(ScoreStore) |  |
| 18 | NUMBER_TOTALPLAYTIME_MINUTE | playtime_total_minute | 공통 | (playtime/60)%60 | - | 등록만(ScoreStore) |  |
| 19 | NUMBER_TOTALPLAYTIME_SECOND | playtime_totla_saecond | 공통 | playtime%60 (enum 이름 오타 그대로: totla_saecond) | - | 등록만(ScoreStore) |  |
| 20 | NUMBER_CURRENT_FPS | current_fps | 공통 | Gdx.graphics.getFramesPerSecond() | - | 없음 |  |
| 21 | NUMBER_TIME_YEAR | currenttime_year | 공통 | main.getCurrnetTime() Calendar.YEAR (로컬 시각) | - | 없음 | 5회 [Play,Result,Root,Select] (ref3,script2) |
| 22 | NUMBER_TIME_MONTH | currenttime_month | 공통 | Calendar.MONTH+1 | - | 없음 | 5회 [Play,Result,Root,Select] (ref3,script2) |
| 23 | NUMBER_TIME_DAY | currenttime_day | 공통 | Calendar.DATE | - | 없음 | 5회 [Play,Result,Root,Select] (ref3,script2) |
| 24 | NUMBER_TIME_HOUR | currenttime_hour | 공통 | Calendar.HOUR_OF_DAY (0-23) | - | 없음 | 2회 [Play,Result] (ref2) |
| 25 | NUMBER_TIME_MINUTE | currenttime_minute | 공통 | Calendar.MINUTE | - | 없음 | 2회 [Play,Result] (ref2) |
| 26 | NUMBER_TIME_SECOND | currenttime_saecond | 공통 | Calendar.SECOND (enum 이름 오타) | - | 없음 | 2회 [Play,Result] (ref2) |
| 27 | NUMBER_OPERATING_TIME_HOUR | boottime_hour | 공통 | main.getPlayTime()(ms, 앱 가동시간)/3600000 | - | 없음 |  |
| 28 | NUMBER_OPERATING_TIME_MINUTE | boottime_minute | 공통 | (getPlayTime/60000)%60 | - | 없음 |  |
| 29 | NUMBER_OPERATING_TIME_SECOND | boottime_second | 공통 | (getPlayTime/1000)%60 | - | 없음 |  |
| 30 | NUMBER_TOTALPLAYCOUNT | player_playcount | 공통 | PlayerData.getPlaycount() | - | 등록만(ScoreStore) |  |
| 31 | NUMBER_TOTALCLEARCOUNT | player_clearcount | 공통 | PlayerData.getClear() | - | 등록만(ScoreStore) |  |
| 32 | NUMBER_TOTALFAILCOUNT | player_failcount | 공통 | playcount-clear | - | 등록만(ScoreStore) |  |
| 33 | NUMBER_TOTALPERFECT | player_perfect | 공통 | PlayerData.getJudgeCount(0) 누적 | - | 등록만(ScoreStore) |  |
| 34 | NUMBER_TOTALGREAT | player_great | 공통 | PlayerData.getJudgeCount(1) | - | 등록만(ScoreStore) |  |
| 35 | NUMBER_TOTALGOOD | player_good | 공통 | PlayerData.getJudgeCount(2) | - | 등록만(ScoreStore) |  |
| 36 | NUMBER_TOTALBAD | player_bad | 공통 | PlayerData.getJudgeCount(3) | - | 등록만(ScoreStore) |  |
| 37 | NUMBER_TOTALPOOR | player_poor | 공통 | PlayerData.getJudgeCount(4) | - | 등록만(ScoreStore) |  |
| 45 | NUMBER_FOLDER_BEGINNER | folder_level_beginner | 공통 | createPlayLevelProperty = 96과 같은 값 (난이도별 구분 없음) | - | 등록만(SongSelect) |  |
| 46 | NUMBER_FOLDER_NORMAL | folder_level_normal | 공통 | 96과 동일 | - | 등록만(SongSelect) |  |
| 47 | NUMBER_FOLDER_HYPER | folder_level_hyper | 공통 | 96과 동일 | - | 등록만(SongSelect) |  |
| 48 | NUMBER_FOLDER_ANOTHER | folder_level_another | 공통 | 96과 동일 | - | 등록만(SongSelect) |  |
| 49 | NUMBER_FOLDER_INSANE | folder_level_insane | 공통 | 96과 동일 | - | 등록만(SongSelect) |  |
| 57 | NUMBER_MASTER_VOLUME | volume_system | 공통 | (int)(AudioConfig.systemvolume*100) | - | 등록만(PlayerConfig) |  |
| 58 | NUMBER_KEY_VOLUME | volume_key | 공통 | (int)(keyvolume*100) | - | 등록만(PlayerConfig) |  |
| 59 | NUMBER_BGM_VOLUME | volume_background | 공통 | (int)(bgvolume*100) | - | 등록만(PlayerConfig) |  |
| 71 | NUMBER_SCORE | score | 플레이,결과,기타 | 결과=newScore.exscore, 그 외 ScoreDataProperty.getNowEXScore() (scoreData null이면 MIN_VALUE) | - | 구현(플,결) | 6회 [Root,Select] (script2,other2,ref2) |
| 72 | NUMBER_MAXSCORE | maxscore | 공통 | ScoreDataProperty.scoreData.getNotes()*2 (scoreData null이면 0) | - | 구현(플,결) |  |
| 74 | NUMBER_TOTALNOTES | totalnotes | 공통 | 코스결과=코스 전 곡 노트 합, 곡 있으면 songdata.getNotes(), 코스데이터 있으면 곡별 노트 합, 없으면 MIN_VALUE | - | 구현(플,결) | 7회 [Play,Result,Root] (script5,ref2) |
| 75 | NUMBER_MAXCOMBO | maxcombo | 선곡,플레이,결과 | 선곡=선택바 score.combo(null이면 MIN), 플레이=JudgeManager.scoreData.combo(최대 콤보), 결과=newScore.combo, 그 외 MIN_VALUE | - | 구현(결) | 24회 [Play] (ref24) |
| 76 | NUMBER_MISSCOUNT | misscount | 선곡,결과 | score.minbp (선곡=선택바, 결과=newScore), 그 외 MIN_VALUE | - | 등록만(ScoreStore) | 1회 [Select] (ref1) |
| 77 | NUMBER_PLAYCOUNT | playcount | 선곡 | 선택바 score.playcount (score null이면 MIN_VALUE), 선곡 외 MIN_VALUE | - | 등록만(ScoreStore) |  |
| 78 | NUMBER_CLEARCOUNT | clearcount | 선곡 | 선택바 score.clearcount | - | 등록만(ScoreStore) |  |
| 79 | NUMBER_FAILCOUNT | failcount | 선곡 | score.playcount-score.clearcount | - | 등록만(ScoreStore) |  |
| 80 | NUMBER_PERFECT2 | perfect2 (범위 처리) | 선곡,플레이,결과 | ScoreDataProperty.scoreData.getJudgeCount(0) (scoreData 없으면 MIN_VALUE) | - | 등록만(ScoreStore) |  |
| 81 | NUMBER_GREAT2 | great2 (범위 처리) | 선곡,플레이,결과 | ScoreDataProperty.scoreData.getJudgeCount(1) (scoreData 없으면 MIN_VALUE) | - | 등록만(ScoreStore) |  |
| 82 | NUMBER_GOOD2 | good2 (범위 처리) | 선곡,플레이,결과 | ScoreDataProperty.scoreData.getJudgeCount(2) (scoreData 없으면 MIN_VALUE) | - | 등록만(ScoreStore) |  |
| 83 | NUMBER_BAD2 | bad2 (범위 처리) | 선곡,플레이,결과 | ScoreDataProperty.scoreData.getJudgeCount(3) (scoreData 없으면 MIN_VALUE) | - | 등록만(ScoreStore) |  |
| 84 | NUMBER_POOR2 | poor2 (범위 처리) | 선곡,플레이,결과 | ScoreDataProperty.scoreData.getJudgeCount(4) (scoreData 없으면 MIN_VALUE) | - | 등록만(ScoreStore) |  |
| 85 | NUMBER_PERFECT_RATE | perfect_rate (범위 처리) | 선곡,플레이,결과 | scoreData.notes>0이면 judgeCount(0)*100/notes (정수 %), 아니면 MIN_VALUE | - | 등록만(ScoreStore) |  |
| 86 | NUMBER_GREAT_RATE | great_rate (범위 처리) | 선곡,플레이,결과 | scoreData.notes>0이면 judgeCount(1)*100/notes (정수 %), 아니면 MIN_VALUE | - | 등록만(ScoreStore) |  |
| 87 | NUMBER_GOOD_RATE | good_rate (범위 처리) | 선곡,플레이,결과 | scoreData.notes>0이면 judgeCount(2)*100/notes (정수 %), 아니면 MIN_VALUE | - | 등록만(ScoreStore) |  |
| 88 | NUMBER_BAD_RATE | bad_rate (범위 처리) | 선곡,플레이,결과 | scoreData.notes>0이면 judgeCount(3)*100/notes (정수 %), 아니면 MIN_VALUE | - | 등록만(ScoreStore) |  |
| 89 | NUMBER_POOR_RATE | poor_rate (범위 처리) | 선곡,플레이,결과 | scoreData.notes>0이면 judgeCount(4)*100/notes (정수 %), 아니면 MIN_VALUE | - | 등록만(ScoreStore) |  |
| 90 | NUMBER_MAXBPM | maxbpm | 공통 | songdata.getMaxbpm() (int), 없으면 MIN_VALUE | - | 구현(플) | 12회 [Play,Root,Select] (script9,ref3) |
| 91 | NUMBER_MINBPM | minbpm | 공통 | songdata.getMinbpm() | - | 구현(플) | 12회 [Play,Root,Select] (script9,ref3) |
| 92 | NUMBER_MAINBPM | mainbpm | 공통 | songdata.getInformation().getMainbpm() (information 없으면 MIN_VALUE) | - | 구현(플) | 11회 [Root,Select] (script11) |
| 96 | NUMBER_PLAYLEVEL | playlevel | 공통 | songdata.getLevel(), songdata 없으면 MIN_VALUE | - | 구현(플,선,결정) | 10회 [Play,Result,decide] (ref8,script2) |
| 100 | NUMBER_POINT | point | 플레이,결과 | ScoreDataProperty.getNowScore(): 모드별 환산(7K/14K: (150000*PG+100000*GR+20000*GD)/notes + 50000*combo/notes, 5K/10K: (100000*PG+100000*GR+50000*GD)/notes, POPN: (100000/70000/40000), 그 외: 1000000/700000/400000 계수) | - | 등록만(PlaySession) | 2회 [Play] (ref2) |
| 101 | NUMBER_SCORE2 | score2 | 플레이,결과 | 71과 동일 | - | 등록만(PlaySession) | 17회 [Play,Root] (script13,ref4) |
| 102 | NUMBER_SCORE_RATE | score_rate | 플레이,결과 | scoreData!=null이면 ScoreDataProperty.getNowRateInt() = (int)(nowrate*100), nowrate=ex/(통과노트*2); 없으면 MIN_VALUE | - | 등록만(PlaySession) | 2회 [Result,Select] (ref2) |
| 103 | NUMBER_SCORE_RATE_AFTERDOT | score_rate_afterdot | 플레이,결과 | getNowRateAfterDot() = ((int)(nowrate*10000))%100 | - | 등록만(PlaySession) | 2회 [Result,Select] (ref2) |
| 104 | NUMBER_COMBO | **구현 없음(상수만)** | - | IntegerPropertyFactory에 해당 id 분기가 없다. Lua는 0, JSON/LR2 number 객체는 무값 처리(미표시) | - | 구현(플) **불일치: beatoraja에는 NUMBER_COMBO 구현이 없다(콤보 숫자는 SkinJudge 내부 number). R-BMS는 콤보를 응답** **[beatoraja엔 없는데 R-BMS만 응답]** |  |
| 105 | NUMBER_MAXCOMBO2 | maxcombo2 | 선곡,플레이,결과 | 75와 동일 | - | 등록만(PlaySession) | 3회 [Play,Result] (ref3) |
| 106 | NUMBER_TOTALNOTES2 | totalnotes2 | 공통 | 74와 동일 | - | 등록만(PlaySession) |  |
| 107 | NUMBER_GROOVEGAUGE | groovegauge | 플레이,결과 | 플레이=(int)gauge.getValue(), 결과=gauge log 마지막 값(선택된 gaugeType), 그 외 MIN_VALUE | - | 구현(플,결) | 6회 [Play,Result,Root] (ref3,script3) |
| 108 | NUMBER_DIFF_EXSCORE | diff_exscore | 플레이,결과 | nowEXScore-nowRivalScore | - | 등록만(PlaySession) |  |
| 110 | NUMBER_PERFECT | perfect (범위 처리) | 공통 | MainState.getJudgeCount(0,true)+getJudgeCount(0,false) = early+late 합 (scoreData 없으면 0) | - | 구현(플,결) | 8회 [Play,Result,Root,Select] (ref5,script3) |
| 111 | NUMBER_GREAT | great (범위 처리) | 공통 | MainState.getJudgeCount(1,true)+getJudgeCount(1,false) = early+late 합 (scoreData 없으면 0) | - | 구현(플,결) | 8회 [Play,Result,Root,Select] (ref5,script3) |
| 112 | NUMBER_GOOD | good (범위 처리) | 공통 | MainState.getJudgeCount(2,true)+getJudgeCount(2,false) = early+late 합 (scoreData 없으면 0) | - | 구현(플,결) | 8회 [Play,Result,Root,Select] (ref5,script3) |
| 113 | NUMBER_BAD | bad (범위 처리) | 공통 | MainState.getJudgeCount(3,true)+getJudgeCount(3,false) = early+late 합 (scoreData 없으면 0) | - | 구현(플,결) | 8회 [Play,Result,Root,Select] (ref5,script3) |
| 114 | NUMBER_POOR | poor (범위 처리) | 공통 | MainState.getJudgeCount(4,true)+getJudgeCount(4,false) = early+late 합 (scoreData 없으면 0) | - | 구현(플,결) | 8회 [Play,Result,Root,Select] (ref5,script3) |
| 115 | NUMBER_TOTAL_RATE | total_rate | 공통 | scoreData!=null이면 getRateInt() = (int)(rate*100), rate=ex/(scoreData.notes*2); 없으면 MIN_VALUE | - | 등록만(PlaySession) |  |
| 116 | NUMBER_TOTAL_RATE_AFTERDOT | total_rate_afterdot | 공통 | getRateAfterDot() = ((int)(rate*10000))%100 | - | 등록만(PlaySession) |  |
| 121 | NUMBER_TARGET_SCORE | target_score | 공통 | ScoreDataProperty.getRivalScore() (목표 EX) | - | 구현(플,결) | 2회 [Play] (ref2) |
| 122 | NUMBER_TARGET_SCORE_RATE | target_score_rate | 공통 | getRivalRateInt() = (int)(rivalscorerate*100) | - | 등록만(ScoreResult) |  |
| 123 | NUMBER_TARGET_SCORE_RATE_AFTERDOT | target_score_rate_afterdot | 공통 | getRivalRateAfterDot() = ((int)(rivalscorerate*10000))%100 | - | 등록만(ScoreResult) |  |
| 128 | NUMBER_DIFF_EXSCORE2 | diff_exscore2 | 플레이,결과 | 108과 동일 | - | 등록만(ScoreResult) |  |
| 135 | NUMBER_TARGET_TOTAL_RATE | target_total_rate | 공통 | getRivalRateInt() (122와 동일) | - | 등록만(ScoreResult) |  |
| 136 | NUMBER_TARGET_TOTAL_RATE_AFTERDOT | target_total_rate_afterdot | 공통 | getRivalRateAfterDot() (123과 동일) | - | 등록만(ScoreResult) |  |
| 150 | NUMBER_HIGHSCORE | highscore | 공통 | 결과=oldScore.exscore, 그 외 getBestScore() | - | 구현(플,결) |  |
| 151 | NUMBER_TARGET_SCORE2 | target_score2 | 공통 | getRivalScore() | - | 등록만(ScoreResult) |  |
| 152 | NUMBER_DIFF_HIGHSCORE | diff_highscore | 플레이,결과 | nowEXScore-nowBestScore | - | 구현(플,결) **불일치: R-BMS=ex-best_ex(전체 최고기록), beatoraja=nowEX-nowBestScore(통과 노트 수 비례 환산). 불일치** | 5회 [Play,Root] (ref4,other1) |
| 153 | NUMBER_DIFF_TARGETSCORE | diff_targetscore | 플레이,결과 | nowEXScore-nowRivalScore | - | 구현(플,결) **불일치: R-BMS 플레이=hud.pace.delta(정의 미확인), 결과=ex-target.ex. beatoraja 플레이=nowEX-nowRivalScore(통과 비례)** | 5회 [Play,Root] (ref4,other1) |
| 154 | NUMBER_DIFF_NEXTRANK | diff_nextrank | 플레이,결과 | ScoreDataProperty.getNextRank(): 다음 3단위 랭크 경계(27분할 중 i%3==0)까지 필요한 EX, 없으면 (notes*2)-ex | - | 등록만(ScoreResult) |  |
| 155 | NUMBER_SCORE_RATE2 | score_rate2 | 공통 | 115와 동일 (getRateInt) | - | 등록만(ScoreResult) |  |
| 156 | NUMBER_SCORE_RATE_AFTERDOT2 | score_rate_afterdot2 | 공통 | 116과 동일 | - | 등록만(ScoreResult) |  |
| 157 | NUMBER_TARGET_SCORE_RATE2 | target_score_rate2 | 공통 | 122와 동일 | - | 등록만(ScoreResult) |  |
| 158 | NUMBER_TARGET_SCORE_RATE_AFTERDOT2 | target_score_rate_afterdot2 | 공통 | 123과 동일 | - | 등록만(ScoreResult) |  |
| 160 | NUMBER_NOWBPM | nowbpm | 플레이 | (int)LaneRenderer.getNowBPM(), 플레이 외 MIN_VALUE | - | 구현(플) | 9회 [Play,Root] (script7,ref2) |
| 161 | NUMBER_PLAYTIME_MINUTE | playtime_minute | 공통 | TIMER_PLAY 켜짐이면 getNowTime(PLAY)/60000, 꺼짐이면 0 | - | 구현(플) | 1회 [Root] (script1) |
| 162 | NUMBER_PLAYTIME_SECOND | playtime_second | 공통 | (getNowTime(PLAY)/1000)%60 | - | 구현(플) | 1회 [Root] (script1) |
| 163 | NUMBER_TIMELEFT_MINUTE | timeleft_minute | 플레이 | max(BMSPlayer.getPlaytime()-nowTime(PLAY)+1000,0)/60000, 플레이 외 MIN_VALUE | - | 구현(플) | 4회 [Play,Root] (script2,ref2) |
| 164 | NUMBER_TIMELEFT_SECOND | timeleft_second | 플레이 | (max(playtime-nowTime(PLAY)+1000,0)/1000)%60 | - | 구현(플) | 4회 [Play,Root] (script2,ref2) |
| 165 | NUMBER_LOADING_PROGRESS | loading_progress | 공통 | (int)((BGA켜짐?(bga.progress+audio.progress)/2:audio.progress)*100) | - | 구현(결정) | 2회 [Play] (ref2) |
| 170 | NUMBER_HIGHSCORE2 | highscore2 | 공통 | 150과 동일 | - | 등록만(ScoreResult) | 3회 [Result,Root] (script3) |
| 171 | NUMBER_SCORE3 | score3 | 공통 | 71과 동일 | - | 등록만(ScoreResult) | 3회 [Result,Root] (script2,ref1) |
| 172 | NUMBER_DIFF_HIGHSCORE2 | diff_highscore2 | 플레이,결과 | 152와 동일 | - | 등록만(ScoreResult) | 2회 [Result] (script1,ref1) |
| 173 | NUMBER_TARGET_MAXCOMBO | target_maxcombo | 결과 | oldScore.combo>0이면 그 값, 아니면 MIN_VALUE | - | 등록만(ScoreResult) |  |
| 174 | NUMBER_MAXCOMBO3 | maxcombo3 | 선곡,플레이,결과 | 75와 동일 | - | 등록만(ScoreResult) |  |
| 175 | NUMBER_DIFF_MAXCOMBO | diff_maxcombo | 결과 | old.combo>0이면 new.combo-old.combo, 아니면 MIN_VALUE | - | 등록만(ScoreResult) | 1회 [Result] (ref1) |
| 176 | NUMBER_TARGET_MISSCOUNT | target_misscount | 결과 | old.minbp!=Integer.MAX_VALUE이면 old.minbp, 아니면 MIN_VALUE | - | 등록만(ScoreResult) | 1회 [Result] (script1) |
| 177 | NUMBER_MISSCOUNT2 | misscount2 | 선곡,결과 | 76과 동일 | - | 등록만(ScoreResult) | 2회 [Result] (script1,ref1) |
| 178 | NUMBER_DIFF_MISSCOUNT | diff_misscount | 결과 | old.minbp 유효하면 new.minbp-old.minbp, 아니면 MIN_VALUE | - | 등록만(ScoreResult) | 2회 [Result] (script1,ref1) |
| 179 | NUMBER_IR_RANK | ir_rank | 선곡,결과 | 선곡=RankingData FINISH일 때 rank, 결과=오프라인 아니면 IRRank, 그 외 MIN_VALUE | - | 없음 | 18회 [Result,Root,Select] (script14,ref4) |
| 180 | NUMBER_IR_TOTALPLAYER | ir_totalplayer | 선곡,결과 | 선곡=RankingData FINISH일 때 totalPlayer, 결과=오프라인 아니면 IRTotalPlayer | - | 없음 | 7회 [Result,Select] (script5,ref2) |
| 181 | NUMBER_IR_CLEARRATE | **구현 없음(상수만)** | - | IntegerPropertyFactory에 해당 id 분기가 없다. Lua는 0, JSON/LR2 number 객체는 무값 처리(미표시) | - | 없음 |  |
| 182 | NUMBER_IR_PREVRANK | ir_prevrank | 결과 | 오프라인 아니면 getOldIRRank() | - | 없음 | 7회 [Result,Root] (script6,ref1) |
| 183 | NUMBER_BEST_RATE | best_rate | 공통 | ScoreDataProperty.getBestRateInt() = (int)(bestscorerate*100) | - | 없음 |  |
| 184 | NUMBER_BEST_RATE_AFTERDOT | best_rate_afterdot | 공통 | getBestRateAfterDot() = ((int)(bestscorerate*10000))%100 | - | 없음 |  |
| 200 | NUMBER_IR_TOTALPLAYER2 | ir_totalplayer2 | 선곡,결과 | 180과 동일 | - | 없음 |  |
| 201 | NUMBER_IR_TOTALPLAYCOUNT | **구현 없음(상수만)** | - | IntegerPropertyFactory에 해당 id 분기가 없다. Lua는 0, JSON/LR2 number 객체는 무값 처리(미표시) | - | 없음 |  |
| 202 | NUMBER_IR_PLAYER_NOPLAY | ir_player_noplay (패턴) | 선곡,결과 | RankingData FINISH일 때 getClearCount(ClearType 0) (인원), 아니면 MIN_VALUE | - | 없음 | 2회 [Result,Select] (other2) |
| 203 | NUMBER_IR_PLAYER_NOPLAY_RATE | ir_player_noplay_rate (패턴) | 선곡,결과 | FINISH이고 totalPlayer>0일 때 count(0)*100/totalPlayer | - | 없음 | 4회 [Result,Select] (other4) |
| 204 | NUMBER_IR_PLAYER_ASSIST | ir_player_assist (패턴) | 선곡,결과 | RankingData FINISH일 때 getClearCount(ClearType 2) (인원), 아니면 MIN_VALUE | - | 없음 | 2회 [Result,Select] (other2) |
| 205 | NUMBER_IR_PLAYER_ASSIST_RATE | ir_player_assist_rate (패턴) | 선곡,결과 | FINISH이고 totalPlayer>0일 때 count(2)*100/totalPlayer | - | 없음 | 4회 [Result,Select] (other4) |
| 206 | NUMBER_IR_PLAYER_LIGHTASSIST | ir_player_lightassist (패턴) | 선곡,결과 | RankingData FINISH일 때 getClearCount(ClearType 3) (인원), 아니면 MIN_VALUE | - | 없음 | 2회 [Result,Select] (other2) |
| 207 | NUMBER_IR_PLAYER_LIGHTASSIST_RATE | ir_player_lightassist_rate (패턴) | 선곡,결과 | FINISH이고 totalPlayer>0일 때 count(3)*100/totalPlayer | - | 없음 | 4회 [Result,Select] (other4) |
| 208 | NUMBER_IR_PLAYER_EXHARD | ir_player_exhard (패턴) | 선곡,결과 | RankingData FINISH일 때 getClearCount(ClearType 7) (인원), 아니면 MIN_VALUE | - | 없음 | 2회 [Result,Select] (other2) |
| 209 | NUMBER_IR_PLAYER_EXHARD_RATE | ir_player_exhard_rate (패턴) | 선곡,결과 | FINISH이고 totalPlayer>0일 때 count(7)*100/totalPlayer | - | 없음 | 4회 [Result,Select] (other4) |
| 210 | NUMBER_IR_PLAYER_FAILED | ir_player_failed (패턴) | 선곡,결과 | RankingData FINISH일 때 getClearCount(ClearType 1) (인원), 아니면 MIN_VALUE | - | 없음 | 2회 [Result,Select] (other2) |
| 211 | NUMBER_IR_PLAYER_FAILED_RATE | ir_player_failed_rate (패턴) | 선곡,결과 | FINISH이고 totalPlayer>0일 때 count(1)*100/totalPlayer | - | 없음 | 4회 [Result,Select] (other4) |
| 212 | NUMBER_IR_PLAYER_EASY | ir_player_easy (패턴) | 선곡,결과 | RankingData FINISH일 때 getClearCount(ClearType 4) (인원), 아니면 MIN_VALUE | - | 없음 | 2회 [Result,Select] (other2) |
| 213 | NUMBER_IR_PLAYER_EASY_RATE | ir_player_easy_rate (패턴) | 선곡,결과 | FINISH이고 totalPlayer>0일 때 count(4)*100/totalPlayer | - | 없음 | 4회 [Result,Select] (other4) |
| 214 | NUMBER_IR_PLAYER_NORMAL | ir_player_normal (패턴) | 선곡,결과 | RankingData FINISH일 때 getClearCount(ClearType 5) (인원), 아니면 MIN_VALUE | - | 없음 | 2회 [Result,Select] (other2) |
| 215 | NUMBER_IR_PLAYER_NORMAL_RATE | ir_player_normal_rate (패턴) | 선곡,결과 | FINISH이고 totalPlayer>0일 때 count(5)*100/totalPlayer | - | 없음 | 4회 [Result,Select] (other4) |
| 216 | NUMBER_IR_PLAYER_HARD | ir_player_hard (패턴) | 선곡,결과 | RankingData FINISH일 때 getClearCount(ClearType 6) (인원), 아니면 MIN_VALUE | - | 없음 | 2회 [Result,Select] (other2) |
| 217 | NUMBER_IR_PLAYER_HARD_RATE | ir_player_hard_rate (패턴) | 선곡,결과 | FINISH이고 totalPlayer>0일 때 count(6)*100/totalPlayer | - | 없음 | 4회 [Result,Select] (other4) |
| 218 | NUMBER_IR_PLAYER_FULLCOMBO | ir_player_fullcombo (패턴) | 선곡,결과 | RankingData FINISH일 때 getClearCount(ClearType 8) (인원), 아니면 MIN_VALUE | - | 없음 | 2회 [Result,Select] (other2) |
| 219 | NUMBER_IR_PLAYER_FULLCOMBO_RATE | ir_player_fullcombo_rate (패턴) | 선곡,결과 | FINISH이고 totalPlayer>0일 때 count(8)*100/totalPlayer | - | 없음 | 4회 [Result,Select] (other4) |
| 220 | NUMBER_IR_UPDATE_WAITING_TIME | ir_update_waiting | 선곡 | currentRankingDuration 기준 다음 IR 갱신까지 남은 초(올림), 해당 없으면 MIN_VALUE | - | 없음 |  |
| 222 | NUMBER_IR_PLAYER_PERFECT | ir_player_perfect (패턴) | 선곡,결과 | RankingData FINISH일 때 getClearCount(ClearType 9) (인원), 아니면 MIN_VALUE | - | 없음 | 2회 [Result,Select] (other2) |
| 223 | NUMBER_IR_PLAYER_PERFECT_RATE | ir_player_perfect_rate (패턴) | 선곡,결과 | FINISH이고 totalPlayer>0일 때 count(9)*100/totalPlayer | - | 없음 | 4회 [Result,Select] (other4) |
| 224 | NUMBER_IR_PLAYER_MAX | ir_player_max (패턴) | 선곡,결과 | RankingData FINISH일 때 getClearCount(ClearType 10) (인원), 아니면 MIN_VALUE | - | 없음 | 2회 [Result,Select] (op2) |
| 225 | NUMBER_IR_PLAYER_MAX_RATE | ir_player_max_rate (패턴) | 선곡,결과 | FINISH이고 totalPlayer>0일 때 count(10)*100/totalPlayer | - | 없음 | 4회 [Result,Select] (op4) |
| 226 | NUMBER_IR_PLAYER_TOTAL_CLEAR | ir_totalclear | 선곡,결과 | RankingData FINISH일 때 clearType 2..10 합계 | - | 없음 |  |
| 227 | NUMBER_IR_PLAYER_TOTAL_CLEAR_RATE | ir_totalclearrate | 선곡,결과 | clearType 2..10 합*100/totalPlayer (정수 %) | - | 없음 | 1회 [Select] (ref1) |
| 228 | NUMBER_IR_PLAYER_TOTAL_FULLCOMBO | ir_totalfullcombo | 선곡,결과 | clearType 8,9,10 합계 | - | 없음 |  |
| 229 | NUMBER_IR_PLAYER_TOTAL_FULLCOMBO_RATE | ir_totalfullcomborate | 선곡,결과 | clearType 8,9,10 합*100/totalPlayer | - | 없음 | 1회 [Select] (ref1) |
| 230 | NUMBER_IR_PLAYER_NOPLAY_RATE_AFTERDOT | ir_player_noplay_rate_afterdot (패턴) | 선곡,결과 | (count(0)*1000/totalPlayer)%10 | - | 없음 | 2회 [Result,Select] (other2) |
| 231 | NUMBER_IR_PLAYER_ASSIST_RATE_AFTERDOT | ir_player_assist_rate_afterdot (패턴) | 선곡,결과 | (count(2)*1000/totalPlayer)%10 | - | 없음 | 2회 [Result,Select] (other2) |
| 232 | NUMBER_IR_PLAYER_LIGHTASSIST_RATE_AFTERDOT | ir_player_lightassist_rate_afterdot (패턴) | 선곡,결과 | (count(3)*1000/totalPlayer)%10 | - | 없음 | 2회 [Result,Select] (other2) |
| 233 | NUMBER_IR_PLAYER_EXHARD_RATE_AFTERDOT | ir_player_exhard_rate_afterdot (패턴) | 선곡,결과 | (count(7)*1000/totalPlayer)%10 | - | 없음 | 2회 [Result,Select] (other2) |
| 234 | NUMBER_IR_PLAYER_FAILED_RATE_AFTERDOT | ir_player_failed_rate_afterdot (패턴) | 선곡,결과 | (count(1)*1000/totalPlayer)%10 | - | 없음 | 2회 [Result,Select] (other2) |
| 235 | NUMBER_IR_PLAYER_EASY_RATE_AFTERDOT | ir_player_easy_rate_afterdot (패턴) | 선곡,결과 | (count(4)*1000/totalPlayer)%10 | - | 없음 | 2회 [Result,Select] (other2) |
| 236 | NUMBER_IR_PLAYER_NORMAL_RATE_AFTERDOT | ir_player_normal_rate_afterdot (패턴) | 선곡,결과 | (count(5)*1000/totalPlayer)%10 | - | 없음 | 2회 [Result,Select] (other2) |
| 237 | NUMBER_IR_PLAYER_HARD_RATE_AFTERDOT | ir_player_hard_rate_afterdot (패턴) | 선곡,결과 | (count(6)*1000/totalPlayer)%10 | - | 없음 | 2회 [Result,Select] (other2) |
| 238 | NUMBER_IR_PLAYER_FULLCOMBO_RATE_AFTERDOT | ir_player_fullcombo_rate_afterdot (패턴) | 선곡,결과 | (count(8)*1000/totalPlayer)%10 | - | 없음 | 2회 [Result,Select] (other2) |
| 239 | NUMBER_IR_PLAYER_PERFECT_RATE_AFTERDOT | ir_player_perfect_rate_afterdot (패턴) | 선곡,결과 | (count(9)*1000/totalPlayer)%10 | - | 없음 | 2회 [Result,Select] (other2) |
| 240 | NUMBER_IR_PLAYER_MAX_RATE_AFTERDOT | ir_player_max_rate_afterdot (패턴) | 선곡,결과 | (count(10)*1000/totalPlayer)%10 | - | 없음 | 2회 [Result,Select] (op2) |
| 241 | NUMBER_IR_PLAYER_TOTAL_CLEAR_RATE_AFTERDOT | ir_totalclearrate_afterdot | 선곡,결과 | (합*1000/totalPlayer)%10 | - | 없음 | 1회 [Select] (ref1) |
| 242 | NUMBER_IR_PLAYER_TOTAL_FULLCOMBO_RATE_AFTERDOT | ir_totalfullcomborate_afterdot | 선곡,결과 | (합*1000/totalPlayer)%10 | - | 없음 | 1회 [Select] (ref1) |
| 243 | NUMBER_LASTPLAY_TIMESTAMP | lastplay_timestamp | 선곡,결과 | 선곡=선택바 score.date, 그 외 scoreData.date (초 단위 epoch). 0 이하 또는 int 범위 초과 시 MIN_VALUE | - | 없음 |  |
| 244 | NUMBER_LASTPLAY_YEAR | lastplay_year | 선곡,결과 | date*1000 -> Calendar.YEAR | - | 없음 |  |
| 245 | NUMBER_LASTPLAY_MONTH | lastplay_month | 선곡,결과 | Calendar.MONTH+1 | - | 없음 |  |
| 246 | NUMBER_LASTPLAY_DAY | lastplay_day | 선곡,결과 | Calendar.DATE | - | 없음 |  |
| 247 | NUMBER_LASTPLAY_HOUR | lastplay_hour | 선곡,결과 | Calendar.HOUR_OF_DAY | - | 없음 |  |
| 248 | NUMBER_LASTPLAY_MINUTE | lastplay_minute | 선곡,결과 | Calendar.MINUTE | - | 없음 |  |
| 249 | NUMBER_LASTPLAY_SECOND | lastplay_second | 선곡,결과 | Calendar.SECOND | - | 없음 |  |
| 271 | NUMBER_RIVAL_SCORE | rival_score | 공통 | ScoreDataProperty.getRivalScore() (121과 동일; RIVAL 계열 중 이 id만 구현) | - | 없음 | 5회 [Root,Select] (script2,other2,ref1) |
| 272 | NUMBER_RIVAL_MAXSCORE | **구현 없음(상수만)** | - | IntegerPropertyFactory에 해당 id 분기가 없다. Lua는 0, JSON/LR2 number 객체는 무값 처리(미표시) | - | 없음 |  |
| 274 | NUMBER_RIVAL_TOTALNOTES | **구현 없음(상수만)** | - | IntegerPropertyFactory에 해당 id 분기가 없다. Lua는 0, JSON/LR2 number 객체는 무값 처리(미표시) | - | 없음 |  |
| 275 | NUMBER_RIVAL_MAXCOMBO | **구현 없음(상수만)** | - | IntegerPropertyFactory에 해당 id 분기가 없다. Lua는 0, JSON/LR2 number 객체는 무값 처리(미표시) | - | 없음 |  |
| 276 | NUMBER_RIVAL_MISSCOUNT | **구현 없음(상수만)** | - | IntegerPropertyFactory에 해당 id 분기가 없다. Lua는 0, JSON/LR2 number 객체는 무값 처리(미표시) | - | 없음 |  |
| 277 | NUMBER_RIVAL_PLAYCOUNT | **구현 없음(상수만)** | - | IntegerPropertyFactory에 해당 id 분기가 없다. Lua는 0, JSON/LR2 number 객체는 무값 처리(미표시) | - | 없음 |  |
| 278 | NUMBER_RIVAL_CLEARCOUNT | **구현 없음(상수만)** | - | IntegerPropertyFactory에 해당 id 분기가 없다. Lua는 0, JSON/LR2 number 객체는 무값 처리(미표시) | - | 없음 |  |
| 279 | NUMBER_RIVAL_FAILCOUNT | **구현 없음(상수만)** | - | IntegerPropertyFactory에 해당 id 분기가 없다. Lua는 0, JSON/LR2 number 객체는 무값 처리(미표시) | - | 없음 |  |
| 280 | NUMBER_RIVAL_PERFECT | rival_perfect (범위 처리) | 선곡,결과 | ScoreDataProperty.getRivalScoreData().getJudgeCount(0) (rival 없으면 MIN_VALUE) | - | 없음 | 1회 [Select] (ref1) |
| 281 | NUMBER_RIVAL_GREAT | rival_great (범위 처리) | 선곡,결과 | ScoreDataProperty.getRivalScoreData().getJudgeCount(1) (rival 없으면 MIN_VALUE) | - | 없음 | 1회 [Select] (ref1) |
| 282 | NUMBER_RIVAL_GOOD | rival_good (범위 처리) | 선곡,결과 | ScoreDataProperty.getRivalScoreData().getJudgeCount(2) (rival 없으면 MIN_VALUE) | - | 없음 | 1회 [Select] (ref1) |
| 283 | NUMBER_RIVAL_BAD | rival_bad (범위 처리) | 선곡,결과 | ScoreDataProperty.getRivalScoreData().getJudgeCount(3) (rival 없으면 MIN_VALUE) | - | 없음 | 1회 [Select] (ref1) |
| 284 | NUMBER_RIVAL_POOR | rival_poor (범위 처리) | 선곡,결과 | ScoreDataProperty.getRivalScoreData().getJudgeCount(4) (rival 없으면 MIN_VALUE) | - | 없음 | 1회 [Select] (ref1) |
| 285 | NUMBER_RIVAL_PERFECT_RATE | rival_perfect_rate (범위 처리) | 선곡,결과 | rival.notes>0이면 judgeCount(0)*100/notes, 아니면 MIN_VALUE | - | 없음 |  |
| 286 | NUMBER_RIVAL_GREAT_RATE | rival_great_rate (범위 처리) | 선곡,결과 | rival.notes>0이면 judgeCount(1)*100/notes, 아니면 MIN_VALUE | - | 없음 |  |
| 287 | NUMBER_RIVAL_GOOD_RATE | rival_good_rate (범위 처리) | 선곡,결과 | rival.notes>0이면 judgeCount(2)*100/notes, 아니면 MIN_VALUE | - | 없음 |  |
| 288 | NUMBER_RIVAL_BAD_RATE | rival_bad_rate (범위 처리) | 선곡,결과 | rival.notes>0이면 judgeCount(3)*100/notes, 아니면 MIN_VALUE | - | 없음 |  |
| 289 | NUMBER_RIVAL_POOR_RATE | rival_poor_rate (범위 처리) | 선곡,결과 | rival.notes>0이면 judgeCount(4)*100/notes, 아니면 MIN_VALUE | - | 없음 |  |
| 300 | NUMBER_FOLDER_TOTALSONGS | folder_totalsongs | 선곡 | 선택바가 DirectoryBar이면 lamps[0..10] 합, 아니면 MIN_VALUE | - | 없음 | 1회 [Select] (ref1) |
| 310 | NUMBER_HISPEED | hispeed | 공통 | hispeed 정수부 (int)hispeed. BMSPlayer=LaneRenderer, 그 외=songdata 모드 PlayConfig | - | 구현(플) |  |
| 311 | NUMBER_HISPEED_AFTERDOT | hispeed_afterdot | 공통 | (int)(hispeed*100)%100 | - | 구현(플) **불일치: R-BMS=f64 fract*100, beatoraja=(int)(f32 hs*100)%100. 부동소수 경계 차이 가능** |  |
| 312 | NUMBER_DURATION | duration | 선곡,플레이,기타 | 선곡=선택바 PlayConfig.duration, 플레이=LaneRenderer.getCurrentDuration(), 그 외 songdata 있으면 PlayConfig.duration | - | 없음 | 3회 [Play,Select] (ref3) |
| 313 | NUMBER_DURATION_GREEN | duration_green | 선곡,플레이,기타 | duration*3/5 (정수 나눗셈) | - | 구현(플) **불일치: R-BMS 값=hud.green_number; beatoraja=duration*3/5. 동일 정의인지 미확인** | 3회 [Play,Select] (ref3) |
| 314 | NUMBER_LIFT1 | lift1 | 플레이 | (int)(LaneRenderer.getLiftRegion()*1000), 플레이 외 MIN_VALUE | - | 없음 | 5회 [Play,Root] (script3,ref2) |
| 315 | NUMBER_HIDDEN1 | hidden1 | 플레이 | (int)(LaneRenderer.getHiddenCover()*1000), 플레이 외 MIN_VALUE | - | 없음 |  |
| 316 | NUMBER_LANECOVER2 | lanecover2 | 플레이 | (int)((1-liftRegion)*lanecover*1000), 플레이 외 MIN_VALUE | - | 없음 |  |
| 320 | NUMBER_FOLDER_NOPLAY | folder_noplay (패턴) | 선곡 | 선택바가 DirectoryBar이면 lamps[0] (해당 클리어타입 곡 수), 아니면 MIN_VALUE | - | 없음 | 1회 [Select] (op1) |
| 321 | NUMBER_FOLDER_FAILED | folder_failed (패턴) | 선곡 | 선택바가 DirectoryBar이면 lamps[1] (해당 클리어타입 곡 수), 아니면 MIN_VALUE | - | 없음 | 1회 [Select] (other1) |
| 322 | NUMBER_FOLDER_ASSIST | folder_assist (패턴) | 선곡 | 선택바가 DirectoryBar이면 lamps[2] (해당 클리어타입 곡 수), 아니면 MIN_VALUE | - | 없음 | 1회 [Select] (other1) |
| 323 | NUMBER_FOLDER_LASSIST | folder_lightassist (패턴) | 선곡 | 선택바가 DirectoryBar이면 lamps[3] (해당 클리어타입 곡 수), 아니면 MIN_VALUE | - | 없음 | 1회 [Select] (other1) |
| 324 | NUMBER_FOLDER_EASY | folder_easy (패턴) | 선곡 | 선택바가 DirectoryBar이면 lamps[4] (해당 클리어타입 곡 수), 아니면 MIN_VALUE | - | 없음 | 1회 [Select] (other1) |
| 325 | NUMBER_FOLDER_GROOOVE | folder_normal (패턴) | 선곡 | 선택바가 DirectoryBar이면 lamps[5] (해당 클리어타입 곡 수), 아니면 MIN_VALUE | - | 없음 | 1회 [Select] (other1) |
| 326 | NUMBER_FOLDER_HARD | folder_hard (패턴) | 선곡 | 선택바가 DirectoryBar이면 lamps[6] (해당 클리어타입 곡 수), 아니면 MIN_VALUE | - | 없음 | 1회 [Select] (other1) |
| 327 | NUMBER_FOLDER_EXHARD | folder_exhard (패턴) | 선곡 | 선택바가 DirectoryBar이면 lamps[7] (해당 클리어타입 곡 수), 아니면 MIN_VALUE | - | 없음 | 1회 [Select] (other1) |
| 328 | NUMBER_FOLDER_FULLCOMBO | folder_fullcombo (패턴) | 선곡 | 선택바가 DirectoryBar이면 lamps[8] (해당 클리어타입 곡 수), 아니면 MIN_VALUE | - | 없음 | 1회 [Select] (other1) |
| 329 | NUMBER_FOLDER_PERFECT | folder_prefect (패턴) | 선곡 | 선택바가 DirectoryBar이면 lamps[9] (해당 클리어타입 곡 수), 아니면 MIN_VALUE | - | 없음 | 1회 [Select] (other1) |
| 330 | NUMBER_FOLDER_MAX | folder_max (패턴) | 선곡 | 선택바가 DirectoryBar이면 lamps[10] (해당 클리어타입 곡 수), 아니면 MIN_VALUE | - | 없음 | 1회 [Select] (other1) |
| 333 | NUMBER_TOTALPLAYNOTES | player_notes | 공통 | judgeCount(0)+(1)+(2)+(3) 합 (POOR 제외) | - | 없음 |  |
| 350 | NUMBER_TOTALNOTE_NORMAL | chart_totalnote_n | 공통 | songdata.getInformation().getN() (일반노트 수), 없으면 MIN_VALUE | - | 없음 | 3회 [Play,Select] (ref3) |
| 351 | NUMBER_TOTALNOTE_LN | chart_totalnote_ln | 공통 | information.getLn() | - | 없음 | 3회 [Play,Select] (ref3) |
| 352 | NUMBER_TOTALNOTE_SCRATCH | chart_totalnote_s | 공통 | information.getS() (스크래치) | - | 없음 | 3회 [Play,Select] (ref3) |
| 353 | NUMBER_TOTALNOTE_BSS | chart_totalnote_ls | 공통 | information.getLs() (LN 스크래치; 상수명 NUMBER_TOTALNOTE_BSS) | - | 없음 | 3회 [Play,Select] (ref3) |
| 354 | NUMBER_TOTALNOTE_MINE | **구현 없음(상수만)** | - | IntegerPropertyFactory에 해당 id 분기가 없다. Lua는 0, JSON/LR2 number 객체는 무값 처리(미표시) | - | 없음 |  |
| 360 | NUMBER_DENSITY_PEAK | chart_peakdensity | 공통 | (int)information.getPeakdensity() | - | 없음 | 2회 [Root,Select] (script1,ref1) |
| 361 | NUMBER_DENSITY_PEAK_AFTERDOT | chart_peakdensity_afterdot | 공통 | ((int)(peakdensity*100))%100 | - | 없음 | 1회 [Root] (script1) |
| 362 | NUMBER_DENSITY_END | chart_enddensity | 공통 | (int)information.getEnddensity() | - | 없음 | 1회 [Select] (ref1) |
| 363 | NUMBER_DENSITY_END_AFTERDOT | chart_enddensity_peak | 공통 | ((int)(enddensity*100))%100 (이름은 peak지만 end density 소수부) | - | 없음 |  |
| 364 | NUMBER_DENSITY_AVERAGE | chart_averagedensity | 공통 | (int)information.getDensity() | - | 없음 | 2회 [Root,Select] (script1,ref1) |
| 365 | NUMBER_DENSITY_AVERAGE_AFTERDOT | chart_averagedensity_afterdot | 공통 | ((int)(density*100))%100 | - | 없음 | 1회 [Root] (script1) |
| 368 | NUMBER_SONGGAUGE_TOTAL | chart_totalgauge | 공통 | (int)information.getTotal() | - | 없음 | 11회 [Result,Select] (script10,ref1) |
| 370 | NUMBER_CLEAR | clear | 결과 | newScore.clear (ClearType id), 그 외 MIN_VALUE | - | 없음 | 25회 [Result,Root] (script19,ref4,act2) |
| 371 | NUMBER_TARGET_CLEAR | target_clear | 결과 | oldScore.clear | - | 없음 | 11회 [Result,Root] (script9,ref1,act1) |
| 372 | NUMBER_AVERAGE_DURATION | duration_average | 결과 | (int)(AbstractResult.getAverageDuration()/1000) | - | 없음 |  |
| 373 | NUMBER_AVERAGE_DURATION_AFTERDOT | duration_average_afterdot | 결과 | (int)((getAverageDuration()/10)%100) | - | 없음 |  |
| 374 | NUMBER_AVERAGE_TIMING | timing_average | 결과 | (int)TimingDistribution.getAverage() | - | 없음 | 1회 [Result] (ref1) |
| 375 | NUMBER_AVERAGE_TIMING_AFTERDOT | timing_average_afterdot | 결과 | 부호 보존 소수 2자리: avg>=0 ? (int)(avg*100)%100 : -( | - | 없음 | 1회 [Result] (ref1) |
| 376 | NUMBER_STDDEV_TIMING | timing_stddev | 결과 | (int)TimingDistribution.getStdDev() | - | 없음 | 1회 [Result] (ref1) |
| 377 | NUMBER_STDDEV_TIMING_AFTERDOT | timing_atddev_afterdot | 결과 | (int)(stddev*100)%100 (enum 이름 오타 atddev) | - | 없음 | 1회 [Result] (ref1) |
| 380 | NUMBER_RANKING1_EXSCORE | ranking_exscore1 (패턴) | 선곡,결과 | RankingData.getScore(index0+rankingOffset).getExscore(), 없으면 MIN_VALUE | - | 없음 | 3회 [Result,Select] (op3) |
| 381 | (상수 없음) | ranking_exscore2 (패턴) | 선곡,결과 | RankingData.getScore(index1+rankingOffset).getExscore(), 없으면 MIN_VALUE | - | 미선언 | 3회 [Result,Select] (other3) |
| 382 | (상수 없음) | ranking_exscore3 (패턴) | 선곡,결과 | RankingData.getScore(index2+rankingOffset).getExscore(), 없으면 MIN_VALUE | - | 미선언 | 3회 [Result,Select] (other3) |
| 383 | (상수 없음) | ranking_exscore4 (패턴) | 선곡,결과 | RankingData.getScore(index3+rankingOffset).getExscore(), 없으면 MIN_VALUE | - | 미선언 | 3회 [Result,Select] (other3) |
| 384 | (상수 없음) | ranking_exscore5 (패턴) | 선곡,결과 | RankingData.getScore(index4+rankingOffset).getExscore(), 없으면 MIN_VALUE | - | 미선언 | 3회 [Result,Select] (other3) |
| 385 | (상수 없음) | ranking_exscore6 (패턴) | 선곡,결과 | RankingData.getScore(index5+rankingOffset).getExscore(), 없으면 MIN_VALUE | - | 미선언 | 3회 [Result,Select] (other3) |
| 386 | (상수 없음) | ranking_exscore7 (패턴) | 선곡,결과 | RankingData.getScore(index6+rankingOffset).getExscore(), 없으면 MIN_VALUE | - | 미선언 | 3회 [Result,Select] (other3) |
| 387 | (상수 없음) | ranking_exscore8 (패턴) | 선곡,결과 | RankingData.getScore(index7+rankingOffset).getExscore(), 없으면 MIN_VALUE | - | 미선언 | 3회 [Result,Select] (other3) |
| 388 | (상수 없음) | ranking_exscore9 (패턴) | 선곡,결과 | RankingData.getScore(index8+rankingOffset).getExscore(), 없으면 MIN_VALUE | - | 미선언 | 3회 [Result,Select] (other3) |
| 389 | NUMBER_RANKING10_EXSCORE | ranking_exscore10 (패턴) | 선곡,결과 | RankingData.getScore(index9+rankingOffset).getExscore(), 없으면 MIN_VALUE | - | 없음 | 3회 [Result,Select] (other3) |
| 390 | NUMBER_RANKING1_CLEAR | ranking_index1 (패턴) | 선곡,결과 | RankingData.getScoreRanking(index0+rankingOffset), 없으면 MIN_VALUE | - | 없음 | 6회 [Result,Select] (op6) |
| 391 | (상수 없음) | ranking_index2 (패턴) | 선곡,결과 | RankingData.getScoreRanking(index1+rankingOffset), 없으면 MIN_VALUE | - | 미선언 | 6회 [Result,Select] (other6) |
| 392 | (상수 없음) | ranking_index3 (패턴) | 선곡,결과 | RankingData.getScoreRanking(index2+rankingOffset), 없으면 MIN_VALUE | - | 미선언 | 6회 [Result,Select] (other6) |
| 393 | (상수 없음) | ranking_index4 (패턴) | 선곡,결과 | RankingData.getScoreRanking(index3+rankingOffset), 없으면 MIN_VALUE | - | 미선언 | 6회 [Result,Select] (other6) |
| 394 | (상수 없음) | ranking_index5 (패턴) | 선곡,결과 | RankingData.getScoreRanking(index4+rankingOffset), 없으면 MIN_VALUE | - | 미선언 | 6회 [Result,Select] (other6) |
| 395 | (상수 없음) | ranking_index6 (패턴) | 선곡,결과 | RankingData.getScoreRanking(index5+rankingOffset), 없으면 MIN_VALUE | - | 미선언 | 6회 [Result,Select] (other6) |
| 396 | (상수 없음) | ranking_index7 (패턴) | 선곡,결과 | RankingData.getScoreRanking(index6+rankingOffset), 없으면 MIN_VALUE | - | 미선언 | 6회 [Result,Select] (other6) |
| 397 | (상수 없음) | ranking_index8 (패턴) | 선곡,결과 | RankingData.getScoreRanking(index7+rankingOffset), 없으면 MIN_VALUE | - | 미선언 | 6회 [Result,Select] (other6) |
| 398 | (상수 없음) | ranking_index9 (패턴) | 선곡,결과 | RankingData.getScoreRanking(index8+rankingOffset), 없으면 MIN_VALUE | - | 미선언 | 6회 [Result,Select] (other6) |
| 399 | NUMBER_RANKING10_CLEAR | ranking_index10 (패턴) | 선곡,결과 | RankingData.getScoreRanking(index9+rankingOffset), 없으면 MIN_VALUE | - | 없음 | 6회 [Result,Select] (other6) |
| 400 | NUMBER_JUDGERANK | judgerank | 공통 | songdata.getJudge() | - | 없음 | 1회 [Select] (ref1) |
| 407 | NUMBER_GROOVEGAUGE_AFTERDOT | groovegauge_afterdot | 플레이,결과 | 플레이=value가 0 초과 0.1 미만이면 1, 아니면 ((int)(value*10))%10 (소수 1자리). 결과=동일 계산(값<1이면 1로 올림) | - | 구현(플,결) **불일치: R-BMS afterdot=소수 2자리(0..99), beatoraja=소수 1자리(0..9; 0<v<0.1이면 1). 자릿수 불일치** | 3회 [Play,Result] (ref3) |
| 410 | NUMBER_EARLY_PERFECT | early_perfect (범위 처리) | 공통 | MainState.getJudgeCount(0,true) (판정 perfect의 FAST 개수) | - | 구현(결) **불일치: R-BMS(결과)=lane_kind_total(fast) 전체 FAST 합; beatoraja=PERFECT의 FAST 개수만. 불일치** | 1회 [Play] (ref1) |
| 411 | NUMBER_LATE_PERFECT | late_perfect (범위 처리) | 공통 | MainState.getJudgeCount(0,false) (판정 perfect의 SLOW 개수) | - | 구현(결) **불일치: R-BMS(결과)=lane_kind_total(slow) 전체 SLOW 합; beatoraja=PERFECT의 SLOW 개수만. 불일치** | 1회 [Play] (ref1) |
| 412 | NUMBER_EARLY_GREAT | early_great (범위 처리) | 공통 | MainState.getJudgeCount(1,true) (판정 great의 FAST 개수) | - | 없음 | 3회 [Play,Result] (ref3) |
| 413 | NUMBER_LATE_GREAT | late_great (범위 처리) | 공통 | MainState.getJudgeCount(1,false) (판정 great의 SLOW 개수) | - | 없음 | 3회 [Play,Result] (ref3) |
| 414 | NUMBER_EARLY_GOOD | early_good (범위 처리) | 공통 | MainState.getJudgeCount(2,true) (판정 good의 FAST 개수) | - | 없음 | 3회 [Play,Result] (ref3) |
| 415 | NUMBER_LATE_GOOD | late_good (범위 처리) | 공통 | MainState.getJudgeCount(2,false) (판정 good의 SLOW 개수) | - | 없음 | 3회 [Play,Result] (ref3) |
| 416 | NUMBER_EARLY_BAD | early_bad (범위 처리) | 공통 | MainState.getJudgeCount(3,true) (판정 bad의 FAST 개수) | - | 없음 | 3회 [Play,Result] (ref3) |
| 417 | NUMBER_LATE_BAD | late_bad (범위 처리) | 공통 | MainState.getJudgeCount(3,false) (판정 bad의 SLOW 개수) | - | 없음 | 3회 [Play,Result] (ref3) |
| 418 | NUMBER_EARLY_POOR | early_poor (범위 처리) | 공통 | MainState.getJudgeCount(4,true) (판정 poor의 FAST 개수) | - | 없음 | 3회 [Play,Result] (ref3) |
| 419 | NUMBER_LATE_POOR | late_poor (범위 처리) | 공통 | MainState.getJudgeCount(4,false) (판정 poor의 SLOW 개수) | - | 없음 | 3회 [Play,Result] (ref3) |
| 420 | NUMBER_MISS | miss | 공통 | getJudgeCount(5,true)+getJudgeCount(5,false) (MainState: ScoreDataProperty.scoreData 없으면 0) | - | 구현(플,결) | 1회 [Result] (ref1) |
| 421 | NUMBER_EARLY_MISS | early_miss | 공통 | getJudgeCount(5,true) | - | 없음 | 1회 [Result] (ref1) |
| 422 | NUMBER_LATE_MISS | late_miss | 공통 | getJudgeCount(5,false) | - | 없음 | 1회 [Result] (ref1) |
| 423 | NUMBER_TOTALEARLY | totalearly | 공통 | 판정 1..5(GR,GD,BD,PR,MS)의 early 합 (PG 제외) | - | 구현(플) **불일치: R-BMS=lane_kind_total(hud.fast)(키+스크래치 FAST 합, PG 포함 여부 미확인); beatoraja=PG 제외 판정1..5 early 합** | 5회 [Play,Result,Root,Select] (script4,ref1) |
| 424 | NUMBER_TOTALLATE | totallate | 공통 | 판정 1..5의 late 합 (PG 제외) | - | 구현(플) **불일치: R-BMS=lane_kind_total(hud.slow); beatoraja=PG 제외 판정1..5 late 합. 의미 불일치 가능** | 5회 [Play,Result,Root,Select] (script4,ref1) |
| 425 | NUMBER_COMBOBREAK | combobreak | 공통 | BD+PR의 early+late 합 | - | 없음 | 1회 [Select] (ref1) |
| 426 | NUMBER_POOR_PLUS_MISS | poor_plus_miss | 공통 | PR+MS 합 | - | 없음 |  |
| 427 | NUMBER_BAD_PLUS_POOR_PLUS_MISS | bad_plus_poor_plus_miss | 공통 | BD+PR+MS 합 | - | 없음 | 1회 [Play] (script1) |
| 450 | NUMBER_RANDOM_1P_1KEY | **구현 없음(상수만)** | - | IntegerPropertyFactory에 해당 id 분기가 없다. Lua는 0, JSON/LR2 number 객체는 무값 처리(미표시) | - | 없음 |  |
| 451 | NUMBER_RANDOM_1P_2KEY | **구현 없음(상수만)** | - | IntegerPropertyFactory에 해당 id 분기가 없다. Lua는 0, JSON/LR2 number 객체는 무값 처리(미표시) | - | 없음 |  |
| 452 | NUMBER_RANDOM_1P_3KEY | **구현 없음(상수만)** | - | IntegerPropertyFactory에 해당 id 분기가 없다. Lua는 0, JSON/LR2 number 객체는 무값 처리(미표시) | - | 없음 |  |
| 453 | NUMBER_RANDOM_1P_4KEY | **구현 없음(상수만)** | - | IntegerPropertyFactory에 해당 id 분기가 없다. Lua는 0, JSON/LR2 number 객체는 무값 처리(미표시) | - | 없음 |  |
| 454 | NUMBER_RANDOM_1P_5KEY | **구현 없음(상수만)** | - | IntegerPropertyFactory에 해당 id 분기가 없다. Lua는 0, JSON/LR2 number 객체는 무값 처리(미표시) | - | 없음 |  |
| 455 | NUMBER_RANDOM_1P_6KEY | **구현 없음(상수만)** | - | IntegerPropertyFactory에 해당 id 분기가 없다. Lua는 0, JSON/LR2 number 객체는 무값 처리(미표시) | - | 없음 |  |
| 456 | NUMBER_RANDOM_1P_7KEY | **구현 없음(상수만)** | - | IntegerPropertyFactory에 해당 id 분기가 없다. Lua는 0, JSON/LR2 number 객체는 무값 처리(미표시) | - | 없음 |  |
| 457 | NUMBER_RANDOM_1P_8KEY | **구현 없음(상수만)** | - | IntegerPropertyFactory에 해당 id 분기가 없다. Lua는 0, JSON/LR2 number 객체는 무값 처리(미표시) | - | 없음 |  |
| 458 | NUMBER_RANDOM_1P_9KEY | **구현 없음(상수만)** | - | IntegerPropertyFactory에 해당 id 분기가 없다. Lua는 0, JSON/LR2 number 객체는 무값 처리(미표시) | - | 없음 |  |
| 459 | NUMBER_RANDOM_1P_SCR | **구현 없음(상수만)** | - | IntegerPropertyFactory에 해당 id 분기가 없다. Lua는 0, JSON/LR2 number 객체는 무값 처리(미표시) | - | 없음 |  |
| 460 | NUMBER_RANDOM_2P_1KEY | **구현 없음(상수만)** | - | IntegerPropertyFactory에 해당 id 분기가 없다. Lua는 0, JSON/LR2 number 객체는 무값 처리(미표시) | - | 없음 |  |
| 461 | NUMBER_RANDOM_2P_2KEY | **구현 없음(상수만)** | - | IntegerPropertyFactory에 해당 id 분기가 없다. Lua는 0, JSON/LR2 number 객체는 무값 처리(미표시) | - | 없음 |  |
| 462 | NUMBER_RANDOM_2P_3KEY | **구현 없음(상수만)** | - | IntegerPropertyFactory에 해당 id 분기가 없다. Lua는 0, JSON/LR2 number 객체는 무값 처리(미표시) | - | 없음 |  |
| 463 | NUMBER_RANDOM_2P_4KEY | **구현 없음(상수만)** | - | IntegerPropertyFactory에 해당 id 분기가 없다. Lua는 0, JSON/LR2 number 객체는 무값 처리(미표시) | - | 없음 |  |
| 464 | NUMBER_RANDOM_2P_5KEY | **구현 없음(상수만)** | - | IntegerPropertyFactory에 해당 id 분기가 없다. Lua는 0, JSON/LR2 number 객체는 무값 처리(미표시) | - | 없음 |  |
| 465 | NUMBER_RANDOM_2P_6KEY | **구현 없음(상수만)** | - | IntegerPropertyFactory에 해당 id 분기가 없다. Lua는 0, JSON/LR2 number 객체는 무값 처리(미표시) | - | 없음 |  |
| 466 | NUMBER_RANDOM_2P_7KEY | **구현 없음(상수만)** | - | IntegerPropertyFactory에 해당 id 분기가 없다. Lua는 0, JSON/LR2 number 객체는 무값 처리(미표시) | - | 없음 |  |
| 469 | NUMBER_RANDOM_2P_SCR | **구현 없음(상수만)** | - | IntegerPropertyFactory에 해당 id 분기가 없다. Lua는 0, JSON/LR2 number 객체는 무값 처리(미표시) | - | 없음 |  |
| 525 | (상수 없음) | judge_duration1 | 플레이 | JudgeManager.getRecentJudgeTiming(0) (ms, 양수=FAST), 플레이 외 0 | - | 미선언 | 2회 [Play] (ref2) |
| 526 | (상수 없음) | judge_duration2 | 플레이 | getRecentJudgeTiming(1) | - | 미선언 | 1회 [Play] (ref1) |
| 527 | (상수 없음) | judge_duration3 | 플레이 | getRecentJudgeTiming(2) | - | 미선언 |  |
| 1163 | NUMBER_SONGLENGTH_MINUTE | chartlength_minute | 공통 | (songdata.getLength()/60000)%60 | - | 없음 | 3회 [Root] (script3) |
| 1164 | NUMBER_SONGLENGTH_SECOND | chartlength_second | 공통 | (songdata.getLength()/1000)%60 | - | 없음 | 3회 [Root] (script3) |
| 1312 | NUMBER_DURATION_LANECOVER_ON | duration_lanecover_on (범위 처리) | 플레이 | (int)round((240000/nowBPM/hispeed)*(1-lanecover)*(1)). BPM=LaneRenderer.getNowBPM; 플레이 외 0 (MIN_VALUE 아님) | - | 없음 | 2회 [Play] (ref2) |
| 1313 | NUMBER_DURATION_GREEN_LANECOVER_ON | duration_green_lanecover_on (범위 처리) | 플레이 | (int)round((240000/nowBPM/hispeed)*(1-lanecover)*(0.6)). BPM=LaneRenderer.getNowBPM; 플레이 외 0 (MIN_VALUE 아님) | - | 없음 | 2회 [Play] (ref2) |
| 1314 | NUMBER_DURATION_LANECOVER_OFF | duration_lanecover_off (범위 처리) | 플레이 | (int)round((240000/nowBPM/hispeed)*(1)*(1)). BPM=LaneRenderer.getNowBPM; 플레이 외 0 (MIN_VALUE 아님) | - | 없음 | 2회 [Play] (ref2) |
| 1315 | NUMBER_DURATION_GREEN_LANECOVER_OFF | duration_green_lanecover_off (범위 처리) | 플레이 | (int)round((240000/nowBPM/hispeed)*(1)*(0.6)). BPM=LaneRenderer.getNowBPM; 플레이 외 0 (MIN_VALUE 아님) | - | 없음 | 2회 [Play] (ref2) |
| 1316 | NUMBER_MAINBPM_DURATION_LANECOVER_ON | mainbpm_duration_lanecover_on (범위 처리) | 플레이 | (int)round((240000/mainBPM/hispeed)*(1-lanecover)*(1)). BPM=getMainBPM; 플레이 외 0 (MIN_VALUE 아님) | - | 없음 | 2회 [Play] (ref2) |
| 1317 | NUMBER_MAINBPM_DURATION_GREEN_LANECOVER_ON | mainbpm_duration_green_lanecover_on (범위 처리) | 플레이 | (int)round((240000/mainBPM/hispeed)*(1-lanecover)*(0.6)). BPM=getMainBPM; 플레이 외 0 (MIN_VALUE 아님) | - | 없음 | 2회 [Play] (ref2) |
| 1318 | NUMBER_MAINBPM_DURATION_LANECOVER_OFF | mainbpm_duration_lanecover_off (범위 처리) | 플레이 | (int)round((240000/mainBPM/hispeed)*(1)*(1)). BPM=getMainBPM; 플레이 외 0 (MIN_VALUE 아님) | - | 없음 | 2회 [Play] (ref2) |
| 1319 | NUMBER_MAINBPM_DURATION_GREEN_LANECOVER_OFF | mainbpm_duration_green_lanecover_off (범위 처리) | 플레이 | (int)round((240000/mainBPM/hispeed)*(1)*(0.6)). BPM=getMainBPM; 플레이 외 0 (MIN_VALUE 아님) | - | 없음 | 2회 [Play] (ref2) |
| 1320 | NUMBER_MINBPM_DURATION_LANECOVER_ON | minbpm_duration_lanecover_on (범위 처리) | 플레이 | (int)round((240000/minBPM/hispeed)*(1-lanecover)*(1)). BPM=getMinBPM; 플레이 외 0 (MIN_VALUE 아님) | - | 없음 | 2회 [Play] (ref2) |
| 1321 | NUMBER_MINBPM_DURATION_GREEN_LANECOVER_ON | minbpm_duration_green_lanecover_on (범위 처리) | 플레이 | (int)round((240000/minBPM/hispeed)*(1-lanecover)*(0.6)). BPM=getMinBPM; 플레이 외 0 (MIN_VALUE 아님) | - | 없음 | 2회 [Play] (ref2) |
| 1322 | NUMBER_MINBPM_DURATION_LANECOVER_OFF | minbpm_duration_lanecover_off (범위 처리) | 플레이 | (int)round((240000/minBPM/hispeed)*(1)*(1)). BPM=getMinBPM; 플레이 외 0 (MIN_VALUE 아님) | - | 없음 | 2회 [Play] (ref2) |
| 1323 | NUMBER_MINBPM_DURATION_GREEN_LANECOVER_OFF | minbpm_duration_green_lanecover_off (범위 처리) | 플레이 | (int)round((240000/minBPM/hispeed)*(1)*(0.6)). BPM=getMinBPM; 플레이 외 0 (MIN_VALUE 아님) | - | 없음 |  |
| 1324 | NUMBER_MAXBPM_DURATION_LANECOVER_ON | maxbpm_duration_lanecover_on (범위 처리) | 플레이 | (int)round((240000/maxBPM/hispeed)*(1-lanecover)*(1)). BPM=getMaxBPM; 플레이 외 0 (MIN_VALUE 아님) | - | 없음 | 2회 [Play] (ref2) |
| 1325 | NUMBER_MAXBPM_DURATION_GREEN_LANECOVER_ON | maxbpm_duration_green_lanecover_on (범위 처리) | 플레이 | (int)round((240000/maxBPM/hispeed)*(1-lanecover)*(0.6)). BPM=getMaxBPM; 플레이 외 0 (MIN_VALUE 아님) | - | 없음 | 2회 [Play] (ref2) |
| 1326 | NUMBER_MAXBPM_DURATION_LANECOVER_OFF | maxbpm_duration_lanecover_off (범위 처리) | 플레이 | (int)round((240000/maxBPM/hispeed)*(1)*(1)). BPM=getMaxBPM; 플레이 외 0 (MIN_VALUE 아님) | - | 없음 | 2회 [Play] (ref2) |
| 1327 | NUMBER_MAXBPM_DURATION_GREEN_LANECOVER_OFF | maxbpm_duration_green_lanecover_off (범위 처리) | 플레이 | (int)round((240000/maxBPM/hispeed)*(1)*(0.6)). BPM=getMaxBPM; 플레이 외 0 (MIN_VALUE 아님) | - | 없음 |  |

## 4. 불리언 속성 (OPTION_*)

조회 함수: `BooleanPropertyFactory.getBooleanProperty(int)` (BooleanPropertyFactory.java:29). `Math.abs(id)`로 조회하고 음수 id는 결과를 부정(`!get`)하는 래퍼를 만든다(같은 파일:49-60). 문자열 이름 조회는 `!이름` 접두 부정과 `practice_itemN`/`practice_itemN_selected` 패턴을 지원한다(:70-85, :114-173). 팩토리가 null을 돌려주는 id(구현 없음)는 `SkinObject.setDrawCondition(int[])`에서 `dstop`(정수 옵션 배열)에 남고(SkinObject.java:282-298), `Skin.prepare`에서 스킨 커스터마이즈 옵션 맵 `Skin.option`과 대조된다: 양수 id는 맵 값이 1이어야, 음수 id는 0이어야 통과하며 맵에 없으면(-1) 객체가 영구 제거된다(Skin.java:217-229). 따라서 구현 없는 id를 조건으로 가진 객체는 beatoraja에서 사용자 옵션으로 정의되지 않는 한 절대 그려지지 않는다. Lua `main_state.option(id)`는 null 프로퍼티면 음수 id여도 false를 돌려준다.

정적 구분: `매프레임`=TYPE_NO_STATIC(프레임마다 평가), `정적(선곡 외)`=TYPE_STATIC_WITHOUT_MUSICSELECT(MusicSelector가 아닌 화면에서는 `Skin.prepare` 시점에 1회 평가해 false면 객체를 영구 제거, 선곡에서는 매프레임), `정적(결과)`=TYPE_STATIC_ON_RESULT(MusicResult/CourseResult에서만 1회 평가), `정적(전체)`=TYPE_STATIC_ALL (BooleanPropertyFactory.java:214-232, Skin.java:197-211).

| id | 상수 | beatoraja 구현(enum 이름) | 정적 구분 | 화면 | 값 계산 근거 | R-BMS | ModernChic 사용 |
|---|---|---|---|---|---|---|---|
| -1 | OPTION_RANDOM_VALUE | **구현 없음(상수만)** | - | - | 팩토리 null -> 스킨 옵션 맵 대조(사용자 정의 옵션 id 전용), 일치 없으면 객체 제거 | 없음 |  |
| 1 | OPTION_FOLDERBAR | select_folderbar | 매프레임 | 선곡 | 선택바가 DirectoryBar | 구현(선) | 6회 [Select] (op4,script2) |
| 2 | OPTION_SONGBAR | select_somgbar | 매프레임 | 선곡 | 선택바가 SongBar (enum 이름 오타 somgbar) | 구현(선) | 67회 [Select] (script35,op30,other2) |
| 3 | OPTION_GRADEBAR | select_coursebar | 매프레임 | 선곡 | 선택바가 GradeBar | 등록만(SongSelect) | 35회 [Root,Select] (script28,op6,other1) |
| 5 | OPTION_PLAYABLEBAR | playablebar | 매프레임 | 선곡 | SongBar(path 있음) 또는 GradeBar(existsAllSongs) 또는 RandomCourseBar(existsAllSongs) 또는 ExecutableBar | 등록만(SongSelect) | 2회 [Select] (op2) |
| 21 | OPTION_PANEL1 | select_panel1 | 매프레임 | 선곡 | MusicSelector.getPanelState()==1 | 구현(선) | 2회 [Select] (op2) |
| 22 | OPTION_PANEL2 | select_panel2 | 매프레임 | 선곡 | getPanelState()==2 | 등록만(SongSelect) | 1회 [Select] (op1) |
| 23 | OPTION_PANEL3 | select_panel3 | 매프레임 | 선곡 | getPanelState()==3 | 등록만(SongSelect) | 1회 [Select] (op1) |
| 30 | OPTION_BGANORMAL | **구현 없음(상수만)** | - | - | 팩토리 null -> 스킨 옵션 맵 대조(사용자 정의 옵션 id 전용), 일치 없으면 객체 제거 | 등록만(PlayerConfig) |  |
| 31 | OPTION_BGAEXTEND | **구현 없음(상수만)** | - | - | 팩토리 null -> 스킨 옵션 맵 대조(사용자 정의 옵션 id 전용), 일치 없으면 객체 제거 | 등록만(PlayerConfig) |  |
| 32 | OPTION_AUTOPLAYOFF | autoplay_off | 매프레임 | 플레이 | 플레이 화면에서 playMode.mode != AUTOPLAY (플레이 외 false) | 구현(플) | 27회 [Play,Root] (op19,script8) |
| 33 | OPTION_AUTOPLAYON | autoplay_on | 매프레임 | 플레이 | 플레이 화면에서 playMode.mode == AUTOPLAY | 구현(플) | 14회 [Play] (op11,script3) |
| 34 | OPTION_GHOST_OFF | **구현 없음(상수만)** | - | - | 팩토리 null -> 스킨 옵션 맵 대조(사용자 정의 옵션 id 전용), 일치 없으면 객체 제거 | 등록만(PlayerConfig) |  |
| 35 | OPTION_GHOST_A | **구현 없음(상수만)** | - | - | 팩토리 null -> 스킨 옵션 맵 대조(사용자 정의 옵션 id 전용), 일치 없으면 객체 제거 | 등록만(PlayerConfig) |  |
| 36 | OPTION_GHOST_B | **구현 없음(상수만)** | - | - | 팩토리 null -> 스킨 옵션 맵 대조(사용자 정의 옵션 id 전용), 일치 없으면 객체 제거 | 등록만(PlayerConfig) |  |
| 37 | OPTION_GHOST_C | **구현 없음(상수만)** | - | - | 팩토리 null -> 스킨 옵션 맵 대조(사용자 정의 옵션 id 전용), 일치 없으면 객체 제거 | 등록만(PlayerConfig) |  |
| 38 | OPTION_SCOREGRAPHOFF | **구현 없음(상수만)** | - | - | 팩토리 null -> 스킨 옵션 맵 대조(사용자 정의 옵션 id 전용), 일치 없으면 객체 제거 | 등록만(PlayerConfig) |  |
| 39 | OPTION_SCOREGRAPHON | **구현 없음(상수만)** | - | - | 팩토리 null -> 스킨 옵션 맵 대조(사용자 정의 옵션 id 전용), 일치 없으면 객체 제거 | 등록만(PlayerConfig) |  |
| 40 | OPTION_BGAOFF | bgaoff | 정적(선곡 외) | 공통 | !BMSResource.isBGAOn() | 등록만(PlayerConfig) |  |
| 41 | OPTION_BGAON | bgaon | 정적(선곡 외) | 공통 | BMSResource.isBGAOn() | 등록만(PlayerConfig) |  |
| 42 | OPTION_GAUGE_GROOVE | gauge_groove | 매프레임 | 플레이,결과 | gaugeType<=2 (ASSISTEASY0,EASY1,NORMAL2). 플레이=GrooveGauge.getType, 결과=getGaugeType, 그 외 false | 구현(플) |  |
| 43 | OPTION_GAUGE_HARD | gauge_hard | 매프레임 | 플레이,결과 | gaugeType>=3 (HARD3,EXHARD4,HAZARD5,CLASS6,EXCLASS7,EXHARDCLASS8) | 구현(플) | 12회 [Play] (op12) |
| 44 | OPTION_GAUGE_GROOVE_2P | **구현 없음(상수만)** | - | - | 팩토리 null -> 스킨 옵션 맵 대조(사용자 정의 옵션 id 전용), 일치 없으면 객체 제거 | 구현(플) **[beatoraja엔 없는데 R-BMS만 응답]** |  |
| 45 | OPTION_GAUGE_HARD_2P | **구현 없음(상수만)** | - | - | 팩토리 null -> 스킨 옵션 맵 대조(사용자 정의 옵션 id 전용), 일치 없으면 객체 제거 | 구현(플) **[beatoraja엔 없는데 R-BMS만 응답]** |  |
| 50 | OPTION_OFFLINE | ir_offline | 정적(전체) | 공통 | main.getIRStatus().length==0 | 등록만(InternetRanking) | 3회 [Select] (op2,script1) |
| 51 | OPTION_ONLINE | ir_online | 정적(전체) | 공통 | getIRStatus().length>0 | 등록만(InternetRanking) | 30회 [Select,course,result] (script28,op2) |
| 60 | OPTION_DISABLE_SAVE_SCORE | disable_save_score | 매프레임 | 공통 | !PlayerResource.isUpdateScore() | 등록만(PlayerConfig) |  |
| 61 | OPTION_ENABLE_SAVE_SCORE | enable_save_score | 매프레임 | 공통 | PlayerResource.isUpdateScore() | 등록만(PlayerConfig) |  |
| 62 | OPTION_NO_SAVE_CLEAR | no_save_clear | 매프레임 | 공통 | 미구현(TODO): 61과 같은 isUpdateScore() 반환 | 등록만(ScoreResult) |  |
| 63 | OPTION_EASY_SAVE_CLEAR | **구현 없음(상수만)** | - | - | 팩토리 null -> 스킨 옵션 맵 대조(사용자 정의 옵션 id 전용), 일치 없으면 객체 제거 | 등록만(ScoreResult) |  |
| 64 | OPTION_NORMAL_SAVE_CLEAR | **구현 없음(상수만)** | - | - | 팩토리 null -> 스킨 옵션 맵 대조(사용자 정의 옵션 id 전용), 일치 없으면 객체 제거 | 등록만(ScoreResult) |  |
| 65 | OPTION_HARD_SAVE_CLEAR | **구현 없음(상수만)** | - | - | 팩토리 null -> 스킨 옵션 맵 대조(사용자 정의 옵션 id 전용), 일치 없으면 객체 제거 | 등록만(ScoreResult) |  |
| 66 | OPTION_FULLCOMBO_SAVE_CLEAR | **구현 없음(상수만)** | - | - | 팩토리 null -> 스킨 옵션 맵 대조(사용자 정의 옵션 id 전용), 일치 없으면 객체 제거 | 등록만(ScoreResult) |  |
| 70 | OPTION_LEVEL_BEGINNER | **구현 없음(상수만)** | - | - | 팩토리 null -> 스킨 옵션 맵 대조(사용자 정의 옵션 id 전용), 일치 없으면 객체 제거 | 등록만(SongSelect) |  |
| 71 | OPTION_LEVEL_NORMAL | **구현 없음(상수만)** | - | - | 팩토리 null -> 스킨 옵션 맵 대조(사용자 정의 옵션 id 전용), 일치 없으면 객체 제거 | 등록만(SongSelect) |  |
| 72 | OPTION_LEVEL_HYPER | **구현 없음(상수만)** | - | - | 팩토리 null -> 스킨 옵션 맵 대조(사용자 정의 옵션 id 전용), 일치 없으면 객체 제거 | 등록만(SongSelect) |  |
| 73 | OPTION_LEVEL_ANOTHER | **구현 없음(상수만)** | - | - | 팩토리 null -> 스킨 옵션 맵 대조(사용자 정의 옵션 id 전용), 일치 없으면 객체 제거 | 등록만(SongSelect) |  |
| 74 | OPTION_LEVEL_INSANE | **구현 없음(상수만)** | - | - | 팩토리 null -> 스킨 옵션 맵 대조(사용자 정의 옵션 id 전용), 일치 없으면 객체 제거 | 등록만(SongSelect) |  |
| 75 | OPTION_LEVEL_BEGINNER_EXCEED | **구현 없음(상수만)** | - | - | 팩토리 null -> 스킨 옵션 맵 대조(사용자 정의 옵션 id 전용), 일치 없으면 객체 제거 | 등록만(SongSelect) |  |
| 76 | OPTION_LEVEL_NORMAL_EXCEED | **구현 없음(상수만)** | - | - | 팩토리 null -> 스킨 옵션 맵 대조(사용자 정의 옵션 id 전용), 일치 없으면 객체 제거 | 등록만(SongSelect) |  |
| 77 | OPTION_LEVEL_HYPER_EXCEED | **구현 없음(상수만)** | - | - | 팩토리 null -> 스킨 옵션 맵 대조(사용자 정의 옵션 id 전용), 일치 없으면 객체 제거 | 등록만(SongSelect) |  |
| 78 | OPTION_LEVEL_ANOTHER_EXCEED | **구현 없음(상수만)** | - | - | 팩토리 null -> 스킨 옵션 맵 대조(사용자 정의 옵션 id 전용), 일치 없으면 객체 제거 | 등록만(SongSelect) |  |
| 79 | OPTION_LEVEL_INSANE_EXCEED | **구현 없음(상수만)** | - | - | 팩토리 null -> 스킨 옵션 맵 대조(사용자 정의 옵션 id 전용), 일치 없으면 객체 제거 | 등록만(SongSelect) |  |
| 80 | OPTION_NOW_LOADING | now_loading | 매프레임 | 플레이 | BMSPlayer.state==STATE_PRELOAD(0). 결정/선곡 화면은 false | 구현(결정) **불일치: R-BMS=로딩 화면(Decide 대체)에서 응답. beatoraja=BMSPlayer.state==PRELOAD (플레이 화면에서)** | 42회 [Play,Root] (op41,script1) |
| 81 | OPTION_LOADED | loaded | 매프레임 | 플레이 | BMSPlayer.state!=STATE_PRELOAD. 결정/선곡 화면은 false | 구현(결정) **불일치: R-BMS=로딩 화면에서 응답. beatoraja=BMSPlayer.state!=PRELOAD** | 28회 [Play] (op18,other10) |
| 82 | OPTION_REPLAY_OFF | replay_off | 매프레임 | 플레이 | playMode.mode==PLAY 또는 PRACTICE | 없음 |  |
| 83 | OPTION_REPLAY_RECORDING | **구현 없음(상수만)** | - | - | 팩토리 null -> 스킨 옵션 맵 대조(사용자 정의 옵션 id 전용), 일치 없으면 객체 제거 | 없음 |  |
| 84 | OPTION_REPLAY_PLAYING | replay_playing | 매프레임 | 플레이 | playMode.mode==REPLAY | 없음 | 3회 [Play] (op2,script1) |
| 90 | OPTION_RESULT_CLEAR | result_clear | 매프레임 | 결과 | scoreData.clear!=Failed(1) 이고 (courseScore null 또는 courseScore.clear!=Failed) | 구현(결) | 7회 [Result] (op4,other2,script1) |
| 91 | OPTION_RESULT_FAIL | result_fail | 매프레임 | 결과 | scoreData.clear==Failed 또는 courseScore.clear==Failed | 구현(결) | 6회 [Result,Root] (other3,script2,op1) |
| 100 | OPTION_SELECT_BAR_NOT_PLAYED | select_bar_not_played | 매프레임 | 선곡 | 선택바가 SongBar/GradeBar 이고 score null 또는 clear==NoPlay(0) | 등록만(SongSelect) | 8회 [Select] (op6,other2) |
| 101 | OPTION_SELECT_BAR_FAILED | select_bar_failed | 매프레임 | 선곡 | 선택바 score.clear==Failed(1) | 등록만(SongSelect) | 4회 [Select] (other3,op1) |
| 102 | OPTION_SELECT_BAR_EASY_CLEARED | select_bar_easy | 매프레임 | 선곡 | 선택바 score.clear==Easy(4) | 등록만(SongSelect) | 4회 [Select] (other4) |
| 103 | OPTION_SELECT_BAR_NORMAL_CLEARED | select_bar_normal | 매프레임 | 선곡 | 선택바 score.clear==Normal(5) | 등록만(SongSelect) | 4회 [Select] (other4) |
| 104 | OPTION_SELECT_BAR_HARD_CLEARED | select_bar_hard | 매프레임 | 선곡 | 선택바 score.clear==Hard(6) | 등록만(SongSelect) | 4회 [Select] (other4) |
| 105 | OPTION_SELECT_BAR_FULL_COMBO_CLEARED | select_bar_fullcombo | 매프레임 | 선곡 | 선택바 score.clear==FullCombo(8) | 등록만(SongSelect) | 4회 [Select] (other4) |
| 118 | OPTION_CLEAR_GROOVE | trophy_gauge_normal | 정적(선곡 외) | 공통 | scoreData.trophy 문자열에 'G' 포함 (SongTrophy.GROOVE) | 등록만(SongSelect) |  |
| 119 | OPTION_CLEAR_HARD | trophy_gauge_hard | 정적(선곡 외) | 공통 | trophy에 'h' 포함 (HARD) | 등록만(SongSelect) |  |
| 120 | OPTION_CLEAR_HAZARD | **구현 없음(상수만)** | - | - | 팩토리 null -> 스킨 옵션 맵 대조(사용자 정의 옵션 id 전용), 일치 없으면 객체 제거 | 등록만(SongSelect) |  |
| 121 | OPTION_CLEAR_EASY | trophy_gauge_easy | 정적(선곡 외) | 공통 | trophy에 'g' 포함 (EASY) | 등록만(SongSelect) |  |
| 124 | OPTION_CLEAR_AEASY | **구현 없음(상수만)** | - | - | 팩토리 null -> 스킨 옵션 맵 대조(사용자 정의 옵션 id 전용), 일치 없으면 객체 제거 | 등록만(SongSelect) |  |
| 125 | OPTION_CLEAR_EXHARD | trophy_gauge_exhard | 정적(선곡 외) | 공통 | trophy에 'H' 포함 (EXHARD) | 등록만(SongSelect) |  |
| 126 | OPTION_CLEAR_NORMAL | trophy_option_normal | 정적(선곡 외) | 공통 | trophy에 'n' 포함 | 등록만(SongSelect) |  |
| 127 | OPTION_CLEAR_MIRROR | trophy_option_mirror | 정적(선곡 외) | 공통 | trophy에 'm' 포함 | 등록만(SongSelect) |  |
| 128 | OPTION_CLEAR_RANDOM | trophy_option_random | 정적(선곡 외) | 공통 | trophy에 'r' 포함 | 등록만(SongSelect) |  |
| 129 | OPTION_CLEAR_SRANDOM | trophy_option_srandom | 정적(선곡 외) | 공통 | trophy에 's' 포함 (S_RANDOM) | 등록만(SongSelect) |  |
| 130 | OPTION_CLEAR_HRANDOM | trophy_option_hrandom | 정적(선곡 외) | 공통 | trophy에 'p' 포함 (H_RANDOM) | 등록만(SongSelect) |  |
| 131 | OPTION_CLEAR_ALLSCR | trophy_option_allscr | 정적(선곡 외) | 공통 | trophy에 'a' 포함 (ALL_SCR) | 등록만(SongSelect) |  |
| 150 | OPTION_DIFFICULTY0 | chart_difficulty_0 | 정적(선곡 외) | 공통 | songdata.getDifficulty()<=0 또는 >5 | 없음 | 7회 [Decide,Play,Result,Root,decide] (other3,script3,op1) |
| 151 | OPTION_DIFFICULTY1 | chart_difficulty_1 | 정적(선곡 외) | 공통 | difficulty==1 | 없음 | 7회 [Decide,Play,Result,Root,decide] (op3,script3,other1) |
| 152 | OPTION_DIFFICULTY2 | chart_difficulty_2 | 정적(선곡 외) | 공통 | difficulty==2 | 없음 | 7회 [Decide,Play,Result,Root,decide] (other4,script3) |
| 153 | OPTION_DIFFICULTY3 | chart_difficulty_3 | 정적(선곡 외) | 공통 | difficulty==3 | 없음 | 7회 [Decide,Play,Result,Root,decide] (other4,script3) |
| 154 | OPTION_DIFFICULTY4 | chart_difficulty_4 | 정적(선곡 외) | 공통 | difficulty==4 | 없음 | 7회 [Decide,Play,Result,Root,decide] (other4,script3) |
| 155 | OPTION_DIFFICULTY5 | chart_difficulty_5 | 정적(선곡 외) | 공통 | difficulty==5 | 없음 | 7회 [Decide,Play,Result,Root,decide] (other4,script3) |
| 160 | OPTION_7KEYSONG | chart_7key | 정적(선곡 외) | 공통 | songdata.getMode()==BEAT_7K.id | 없음 | 2회 [Result,Select] (op2) |
| 161 | OPTION_5KEYSONG | chart_5key | 정적(선곡 외) | 공통 | mode==BEAT_5K.id | 없음 | 2회 [Result,Select] (op2) |
| 162 | OPTION_14KEYSONG | chart_14key | 정적(선곡 외) | 공통 | mode==BEAT_14K.id | 없음 | 4회 [Result,Select] (other2,script1,op1) |
| 163 | OPTION_10KEYSONG | chart_10key | 정적(선곡 외) | 공통 | mode==BEAT_10K.id | 없음 | 3회 [Result,Select] (other2,script1) |
| 164 | OPTION_9KEYSONG | chart_9key | 정적(선곡 외) | 공통 | mode==POPN_9K.id | 없음 | 2회 [Result,Select] (other2) |
| 170 | OPTION_NO_BGA | song_no_bga | 정적(선곡 외) | 공통 | !songdata.hasBGA() | 등록만(SongSelect) | 16회 [Play,Select] (script13,op3) |
| 171 | OPTION_BGA | song_bga | 정적(선곡 외) | 공통 | songdata.hasBGA() | 등록만(SongSelect) | 2회 [Play,Select] (op2) |
| 172 | OPTION_NO_LN | chart_no_ln | 정적(선곡 외) | 공통 | !songdata.hasAnyLongNote() | 등록만(SongSelect) | 2회 [Play] (script2) |
| 173 | OPTION_LN | chart_ln | 정적(선곡 외) | 공통 | songdata.hasAnyLongNote() | 등록만(SongSelect) | 5회 [Play,Root] (script5) |
| 174 | OPTION_NO_TEXT | song_no_text | 정적(선곡 외) | 공통 | !songdata.hasDocument() | 등록만(SongSelect) |  |
| 175 | OPTION_TEXT | song_text | 정적(선곡 외) | 공통 | songdata.hasDocument() | 등록만(SongSelect) | 2회 [Select] (op2) |
| 176 | OPTION_NO_BPMCHANGE | chart_no_bpmchange | 정적(선곡 외) | 공통 | minbpm==maxbpm | 등록만(SongSelect) | 5회 [Play,Select] (op4,other1) |
| 177 | OPTION_BPMCHANGE | chart_bpmchange | 정적(선곡 외) | 공통 | minbpm<maxbpm | 등록만(SongSelect) | 17회 [Play,Select] (op10,other7) |
| 178 | OPTION_NO_RANDOMSEQUENCE | chart_no_randomsequence | 정적(선곡 외) | 공통 | !songdata.hasRandomSequence() | 등록만(SongSelect) |  |
| 179 | OPTION_RANDOMSEQUENCE | chart_randomsequence | 정적(선곡 외) | 공통 | songdata.hasRandomSequence() | 등록만(SongSelect) |  |
| 180 | OPTION_JUDGE_VERYHARD | chart_judge_veryhard | 정적(선곡 외) | 공통 | songdata.getJudge()==0 또는 10<=judge<35 | 등록만(PlayerConfig) | 4회 [Play,Result,Root] (other2,script1,op1) |
| 181 | OPTION_JUDGE_HARD | chart_judge_hard | 정적(선곡 외) | 공통 | judge==1 또는 35<=judge<60 | 등록만(PlayerConfig) | 4회 [Play,Result,Root] (other3,script1) |
| 182 | OPTION_JUDGE_NORMAL | chart_judge_normal | 정적(선곡 외) | 공통 | judge==2 또는 60<=judge<85 | 등록만(PlayerConfig) | 4회 [Play,Result,Root] (other3,script1) |
| 183 | OPTION_JUDGE_EASY | chart_judge_easy | 정적(선곡 외) | 공통 | judge==3 또는 85<=judge<110 | 등록만(PlayerConfig) | 4회 [Play,Result,Root] (other3,script1) |
| 184 | OPTION_JUDGE_VERYEASY | chart_judge_veryeasy | 정적(선곡 외) | 공통 | judge==4 또는 judge>=110 | 등록만(PlayerConfig) | 4회 [Play,Result,Root] (op2,script1,other1) |
| 190 | OPTION_NO_STAGEFILE | no_stagefile | 정적(선곡 외) | 공통 | BMSResource.getStagefile()==null | 등록만(SongSelect) | 1회 [Result] (op1) |
| 191 | OPTION_STAGEFILE | stagefile | 정적(선곡 외) | 공통 | getStagefile()!=null | 등록만(SongSelect) | 4회 [Play,Result] (op3,other1) |
| 192 | OPTION_NO_BANNER | no_banner | 정적(선곡 외) | 공통 | getBanner()==null | 등록만(SongSelect) |  |
| 193 | OPTION_BANNER | banner | 정적(선곡 외) | 공통 | getBanner()!=null | 등록만(SongSelect) |  |
| 194 | OPTION_NO_BACKBMP | no_backbmp | 정적(선곡 외) | 공통 | getBackbmp()==null | 등록만(SongSelect) | 4회 [Play] (other3,op1) |
| 195 | OPTION_BACKBMP | backbmp | 정적(선곡 외) | 공통 | getBackbmp()!=null | 등록만(SongSelect) | 2회 [Play] (other2) |
| 196 | OPTION_NO_REPLAYDATA | replaydata_no_1 | 매프레임 | 선곡,결과 | 선곡=선택바(SelectableBar)에 리플레이 슬롯0 없음, 결과=ReplayStatus.NOT_EXIST(슬롯0) | 없음 | 1회 [Result] (op1) |
| 197 | OPTION_REPLAYDATA | replaydata_exist_1 | 매프레임 | 선곡,결과 | 선곡=슬롯0 리플레이 존재, 결과=ReplayStatus.EXIST(슬롯0) | 없음 | 5회 [Result,Root,Select] (op4,script1) |
| 198 | OPTION_REPLAYDATA_SAVED | replaydata_saved_1 | 매프레임 | 결과 | 결과=ReplayStatus.SAVED(슬롯0). 선곡에서는 항상 false | 없음 | 1회 [Result] (op1) |
| 200 | OPTION_1P_AAA | rank_1p_aaa | 정적(결과) | 플레이,결과 | createNowRank(7): qualifyNowRank(24) 이고 상한 없음 (AAA) | 등록만(PlaySession) | 3회 [Play,Select] (op3) |
| 201 | OPTION_1P_AA | rank_1p_aa | 정적(결과) | 플레이,결과 | createNowRank(6): nowRank 21 이상 24 미만 | 등록만(PlaySession) | 3회 [Play,Select] (op3) |
| 202 | OPTION_1P_A | rank_1p_a | 정적(결과) | 플레이,결과 | createNowRank(5): 18 이상 21 미만 | 등록만(PlaySession) | 3회 [Play,Select] (other2,op1) |
| 203 | OPTION_1P_B | rank_1p_b | 정적(결과) | 플레이,결과 | createNowRank(4): 15 이상 18 미만 | 등록만(PlaySession) | 3회 [Play,Select] (other2,op1) |
| 204 | OPTION_1P_C | rank_1p_c | 정적(결과) | 플레이,결과 | createNowRank(3): 12 이상 15 미만 | 등록만(PlaySession) | 3회 [Play,Select] (other2,op1) |
| 205 | OPTION_1P_D | rank_1p_d | 정적(결과) | 플레이,결과 | createNowRank(2): 9 이상 12 미만 | 등록만(PlaySession) | 3회 [Play,Select] (other2,op1) |
| 206 | OPTION_1P_E | rank_1p_e | 정적(결과) | 플레이,결과 | createNowRank(1): 6 이상 9 미만 | 등록만(PlaySession) | 3회 [Play,Select] (other2,op1) |
| 207 | OPTION_1P_F | rank_1p_f | 정적(결과) | 플레이,결과 | createNowRank(0): 0 이상 6 미만 | 등록만(PlaySession) | 3회 [Play,Select] (other2,op1) |
| 210 | OPTION_2P_AAA | **구현 없음(상수만)** | - | - | 팩토리 null -> 스킨 옵션 맵 대조(사용자 정의 옵션 id 전용), 일치 없으면 객체 제거 | 등록만(PlaySession) |  |
| 211 | OPTION_2P_AA | **구현 없음(상수만)** | - | - | 팩토리 null -> 스킨 옵션 맵 대조(사용자 정의 옵션 id 전용), 일치 없으면 객체 제거 | 등록만(PlaySession) |  |
| 212 | OPTION_2P_A | **구현 없음(상수만)** | - | - | 팩토리 null -> 스킨 옵션 맵 대조(사용자 정의 옵션 id 전용), 일치 없으면 객체 제거 | 등록만(PlaySession) |  |
| 213 | OPTION_2P_B | **구현 없음(상수만)** | - | - | 팩토리 null -> 스킨 옵션 맵 대조(사용자 정의 옵션 id 전용), 일치 없으면 객체 제거 | 등록만(PlaySession) |  |
| 214 | OPTION_2P_C | **구현 없음(상수만)** | - | - | 팩토리 null -> 스킨 옵션 맵 대조(사용자 정의 옵션 id 전용), 일치 없으면 객체 제거 | 등록만(PlaySession) |  |
| 215 | OPTION_2P_D | **구현 없음(상수만)** | - | - | 팩토리 null -> 스킨 옵션 맵 대조(사용자 정의 옵션 id 전용), 일치 없으면 객체 제거 | 등록만(PlaySession) |  |
| 216 | OPTION_2P_E | **구현 없음(상수만)** | - | - | 팩토리 null -> 스킨 옵션 맵 대조(사용자 정의 옵션 id 전용), 일치 없으면 객체 제거 | 등록만(PlaySession) |  |
| 217 | OPTION_2P_F | **구현 없음(상수만)** | - | - | 팩토리 null -> 스킨 옵션 맵 대조(사용자 정의 옵션 id 전용), 일치 없으면 객체 제거 | 등록만(PlaySession) |  |
| 220 | OPTION_AAA | rank_aaa | 정적(결과) | 플레이,결과 | qualifyRank(24): rate>=24/27 (누적형, 상위 랭크 도달 시 하위도 true) | 등록만(PlaySession) | 4회 [Play,Root] (op3,script1) |
| 221 | OPTION_AA | rank_aa | 정적(결과) | 플레이,결과 | qualifyRank(21): rate>=21/27 | 등록만(PlaySession) | 4회 [Play,Root] (op3,script1) |
| 222 | OPTION_A | rank_a | 정적(결과) | 플레이,결과 | qualifyRank(18): rate>=18/27 | 등록만(PlaySession) | 4회 [Play,Root] (op3,script1) |
| 223 | OPTION_B | rank_b | 정적(결과) | 플레이,결과 | qualifyRank(15): rate>=15/27 | 등록만(PlaySession) |  |
| 224 | OPTION_C | rank_c | 정적(결과) | 플레이,결과 | qualifyRank(12): rate>=12/27 | 등록만(PlaySession) |  |
| 225 | OPTION_D | rank_d | 정적(결과) | 플레이,결과 | qualifyRank(9): rate>=9/27 | 등록만(PlaySession) |  |
| 226 | OPTION_E | rank_e | 정적(결과) | 플레이,결과 | qualifyRank(6): rate>=6/27 | 등록만(PlaySession) |  |
| 227 | OPTION_F | rank_f | 정적(결과) | 플레이,결과 | qualifyRank(0): totalnotes!=0 이면 항상 true | 등록만(PlaySession) |  |
| 230 | OPTION_1P_0_9 | gauge_1p_0_10 | 정적(결과) | 플레이 | 플레이 게이지 value가 [0,10%*max) 구간. 플레이 외 false | 등록만(PlaySession) | 5회 [Play] (other5) |
| 231 | OPTION_1P_10_19 | gauge_1p_10_20 | 정적(결과) | 플레이 | [10,20%) | 등록만(PlaySession) | 5회 [Play] (other5) |
| 232 | OPTION_1P_20_29 | gauge_1p_20_30 | 정적(결과) | 플레이 | [20,30%) | 등록만(PlaySession) | 5회 [Play] (other4,op1) |
| 233 | OPTION_1P_30_39 | gauge_1p_30_40 | 정적(결과) | 플레이 | [30,40%) | 등록만(PlaySession) |  |
| 234 | OPTION_1P_40_49 | gauge_1p_40_50 | 정적(결과) | 플레이 | [40,50%) | 등록만(PlaySession) |  |
| 235 | OPTION_1P_50_59 | gauge_1p_50_60 | 정적(결과) | 플레이 | [50,60%) | 등록만(PlaySession) |  |
| 236 | OPTION_1P_60_69 | gauge_1p_60_70 | 정적(결과) | 플레이 | [60,70%) | 등록만(PlaySession) |  |
| 237 | OPTION_1P_70_79 | gauge_1p_70_80 | 정적(결과) | 플레이 | [70,80%) | 등록만(PlaySession) |  |
| 238 | OPTION_1P_80_89 | gauge_1p_80_90 | 정적(결과) | 플레이 | [80,90%) | 등록만(PlaySession) |  |
| 239 | OPTION_1P_90_99 | gauge_1p_90_100 | 정적(결과) | 플레이 | [90,100%) | 등록만(PlaySession) |  |
| 240 | OPTION_1P_100 | gauge_1p_100 | 정적(결과) | 플레이 | [100%*max, 110%*max) 즉 게이지 만땅(구간 상한 포함 안 됨 주의) | 등록만(PlaySession) | 2회 [Play] (op2) |
| 241 | OPTION_1P_PERFECT | judge_1p_perfect | 매프레임 | 플레이 | JudgeManager.getNowJudge(0)==1 (직전 판정 PGREAT) | 구현(플) **불일치: beatoraja는 PGREAT만 구현(242..246 구현 없음). R-BMS는 241..246을 모두 응답** | 1회 [Play] (script1) |
| 242 | OPTION_1P_GREAT | **구현 없음(상수만)** | - | - | 팩토리 null -> 스킨 옵션 맵 대조(사용자 정의 옵션 id 전용), 일치 없으면 객체 제거 | 구현(플) **[beatoraja엔 없는데 R-BMS만 응답]** |  |
| 243 | OPTION_1P_GOOD | **구현 없음(상수만)** | - | - | 팩토리 null -> 스킨 옵션 맵 대조(사용자 정의 옵션 id 전용), 일치 없으면 객체 제거 | 구현(플) **[beatoraja엔 없는데 R-BMS만 응답]** |  |
| 244 | OPTION_1P_BAD | **구현 없음(상수만)** | - | - | 팩토리 null -> 스킨 옵션 맵 대조(사용자 정의 옵션 id 전용), 일치 없으면 객체 제거 | 구현(플) **[beatoraja엔 없는데 R-BMS만 응답]** |  |
| 245 | OPTION_1P_POOR | **구현 없음(상수만)** | - | - | 팩토리 null -> 스킨 옵션 맵 대조(사용자 정의 옵션 id 전용), 일치 없으면 객체 제거 | 구현(플) **[beatoraja엔 없는데 R-BMS만 응답]** |  |
| 246 | OPTION_1P_MISS | **구현 없음(상수만)** | - | - | 팩토리 null -> 스킨 옵션 맵 대조(사용자 정의 옵션 id 전용), 일치 없으면 객체 제거 | 구현(플) **[beatoraja엔 없는데 R-BMS만 응답]** |  |
| 261 | OPTION_2P_PERFECT | judge_2p_perfect | 매프레임 | 플레이 | getNowJudge(1)==1 | 구현(플) |  |
| 262 | OPTION_2P_GREAT | **구현 없음(상수만)** | - | - | 팩토리 null -> 스킨 옵션 맵 대조(사용자 정의 옵션 id 전용), 일치 없으면 객체 제거 | 구현(플) **[beatoraja엔 없는데 R-BMS만 응답]** |  |
| 263 | OPTION_2P_GOOD | **구현 없음(상수만)** | - | - | 팩토리 null -> 스킨 옵션 맵 대조(사용자 정의 옵션 id 전용), 일치 없으면 객체 제거 | 구현(플) **[beatoraja엔 없는데 R-BMS만 응답]** |  |
| 270 | OPTION_LANECOVER1_CHANGING | lanecover1_changing | 매프레임 | 공통 | input.startPressed() 또는 isSelectPressed() | 등록만(PlayerConfig) | 22회 [Play] (op18,other4) |
| 271 | OPTION_LANECOVER1_ON | lanecover1_on | 매프레임 | 플레이 | LaneRenderer.getPlayConfig().isEnablelanecover() | 등록만(PlayerConfig) | 2회 [Play] (op2) |
| 272 | OPTION_LIFT1_ON | lift1_on | 매프레임 | 플레이 | PlayConfig.isEnablelift() | 등록만(PlayerConfig) | 3회 [Root] (script3) |
| 273 | OPTION_HIDDEN1_ON | hidden1_on | 매프레임 | 플레이 | PlayConfig.isEnablehidden() | 등록만(PlayerConfig) |  |
| 280 | OPTION_COURSE_STAGE1 | course_stage1 | 정적(선곡 외) | 공통 | 코스 진행 중이고 courseIndex==0 이며 마지막 스테이지가 아님. 선곡에서는 비정적 | 등록만(SongSelect) |  |
| 281 | OPTION_COURSE_STAGE2 | course_stage2 | 정적(선곡 외) | 공통 | courseIndex==1 이며 마지막 아님 | 등록만(SongSelect) |  |
| 282 | OPTION_COURSE_STAGE3 | course_stage3 | 정적(선곡 외) | 공통 | courseIndex==2 이며 마지막 아님 | 등록만(SongSelect) |  |
| 283 | OPTION_COURSE_STAGE4 | course_stage4 | 정적(선곡 외) | 공통 | courseIndex==3 이며 마지막 아님 | 등록만(SongSelect) |  |
| 289 | OPTION_COURSE_STAGE_FINAL | course_stage_final | 정적(선곡 외) | 공통 | courseIndex==마지막 스테이지 | 등록만(SongSelect) | 2회 [Result] (op2) |
| 290 | OPTION_MODE_COURSE | mode_course | 정적(선곡 외) | 공통 | PlayerResource.getCourseData()!=null | 등록만(SongSelect) | 3회 [Play,Result] (other2,script1) |
| 291 | OPTION_MODE_NONSTOP | **구현 없음(상수만)** | - | - | 팩토리 null -> 스킨 옵션 맵 대조(사용자 정의 옵션 id 전용), 일치 없으면 객체 제거 | 등록만(SongSelect) |  |
| 292 | OPTION_MODE_EXPERT | **구현 없음(상수만)** | - | - | 팩토리 null -> 스킨 옵션 맵 대조(사용자 정의 옵션 id 전용), 일치 없으면 객체 제거 | 등록만(SongSelect) |  |
| 293 | OPTION_MODE_GRADE | **구현 없음(상수만)** | - | - | 팩토리 null -> 스킨 옵션 맵 대조(사용자 정의 옵션 id 전용), 일치 없으면 객체 제거 | 등록만(SongSelect) |  |
| 300 | OPTION_RESULT_AAA_1P | rank_result_1p_aaa | 정적(결과) | 플레이,결과 | createNowRank(7) (200과 같은 식) | 구현(결) | 6회 [Result,Root] (op4,script2) |
| 301 | OPTION_RESULT_AA_1P | rank_result_1p_aa | 정적(결과) | 플레이,결과 | createNowRank(6) | 구현(결) | 6회 [Result,Root] (other4,script2) |
| 302 | OPTION_RESULT_A_1P | rank_result_1p_a | 정적(결과) | 플레이,결과 | createNowRank(5) | 구현(결) | 6회 [Result,Root] (other4,script2) |
| 303 | OPTION_RESULT_B_1P | rank_result_1p_b | 정적(결과) | 플레이,결과 | createNowRank(4) | 구현(결) | 6회 [Result,Root] (other4,script2) |
| 304 | OPTION_RESULT_C_1P | rank_result_1p_c | 정적(결과) | 플레이,결과 | createNowRank(3) | 구현(결) | 6회 [Result,Root] (other4,script2) |
| 305 | OPTION_RESULT_D_1P | rank_result_1p_d | 정적(결과) | 플레이,결과 | createNowRank(2) | 구현(결) | 6회 [Result,Root] (other4,script2) |
| 306 | OPTION_RESULT_E_1P | rank_result_1p_e | 정적(결과) | 플레이,결과 | createNowRank(1) | 구현(결) | 6회 [Result,Root] (other4,script2) |
| 307 | OPTION_RESULT_F_1P | rank_result_1p_f | 정적(결과) | 플레이,결과 | createNowRank(0) | 구현(결) | 6회 [Result,Root] (other4,script2) |
| 308 | OPTION_RESULT_0_1P | **구현 없음(상수만)** | - | - | 팩토리 null -> 스킨 옵션 맵 대조(사용자 정의 옵션 id 전용), 일치 없으면 객체 제거 | 등록만(ScoreResult) |  |
| 310 | OPTION_RESULT_AAA_2P | **구현 없음(상수만)** | - | - | 팩토리 null -> 스킨 옵션 맵 대조(사용자 정의 옵션 id 전용), 일치 없으면 객체 제거 | 등록만(ScoreResult) |  |
| 311 | OPTION_RESULT_AA_2P | **구현 없음(상수만)** | - | - | 팩토리 null -> 스킨 옵션 맵 대조(사용자 정의 옵션 id 전용), 일치 없으면 객체 제거 | 등록만(ScoreResult) |  |
| 312 | OPTION_RESULT_A_2P | **구현 없음(상수만)** | - | - | 팩토리 null -> 스킨 옵션 맵 대조(사용자 정의 옵션 id 전용), 일치 없으면 객체 제거 | 등록만(ScoreResult) |  |
| 313 | OPTION_RESULT_B_2P | **구현 없음(상수만)** | - | - | 팩토리 null -> 스킨 옵션 맵 대조(사용자 정의 옵션 id 전용), 일치 없으면 객체 제거 | 등록만(ScoreResult) |  |
| 314 | OPTION_RESULT_C_2P | **구현 없음(상수만)** | - | - | 팩토리 null -> 스킨 옵션 맵 대조(사용자 정의 옵션 id 전용), 일치 없으면 객체 제거 | 등록만(ScoreResult) |  |
| 315 | OPTION_RESULT_D_2P | **구현 없음(상수만)** | - | - | 팩토리 null -> 스킨 옵션 맵 대조(사용자 정의 옵션 id 전용), 일치 없으면 객체 제거 | 등록만(ScoreResult) |  |
| 316 | OPTION_RESULT_E_2P | **구현 없음(상수만)** | - | - | 팩토리 null -> 스킨 옵션 맵 대조(사용자 정의 옵션 id 전용), 일치 없으면 객체 제거 | 등록만(ScoreResult) |  |
| 317 | OPTION_RESULT_F_2P | **구현 없음(상수만)** | - | - | 팩토리 null -> 스킨 옵션 맵 대조(사용자 정의 옵션 id 전용), 일치 없으면 객체 제거 | 등록만(ScoreResult) |  |
| 318 | OPTION_RESULT_0_2P | **구현 없음(상수만)** | - | - | 팩토리 null -> 스킨 옵션 맵 대조(사용자 정의 옵션 id 전용), 일치 없으면 객체 제거 | 등록만(ScoreResult) |  |
| 320 | OPTION_BEST_AAA_1P | rank_best_1p_aaa | 정적(결과) | 플레이,결과 | createBestRank(7): qualifyBestRank(24) (최고기록 기준) | 등록만(ScoreResult) | 1회 [Root] (script1) |
| 321 | OPTION_BEST_AA_1P | rank_best_1p_aa | 정적(결과) | 플레이,결과 | createBestRank(6) | 등록만(ScoreResult) | 1회 [Root] (script1) |
| 322 | OPTION_BEST_A_1P | rank_best_1p_a | 정적(결과) | 플레이,결과 | createBestRank(5) | 등록만(ScoreResult) | 1회 [Root] (script1) |
| 323 | OPTION_BEST_B_1P | rank_best_1p_b | 정적(결과) | 플레이,결과 | createBestRank(4) | 등록만(ScoreResult) | 1회 [Root] (script1) |
| 324 | OPTION_BEST_C_1P | rank_best_1p_c | 정적(결과) | 플레이,결과 | createBestRank(3) | 등록만(ScoreResult) | 1회 [Root] (script1) |
| 325 | OPTION_BEST_D_1P | rank_best_1p_d | 정적(결과) | 플레이,결과 | createBestRank(2) | 등록만(ScoreResult) | 1회 [Root] (script1) |
| 326 | OPTION_BEST_E_1P | rank_best_1p_e | 정적(결과) | 플레이,결과 | createBestRank(1) | 등록만(ScoreResult) | 1회 [Root] (script1) |
| 327 | OPTION_BEST_F_1P | rank_best_1p_f | 정적(결과) | 플레이,결과 | createBestRank(0) | 등록만(ScoreResult) | 1회 [Root] (script1) |
| 330 | OPTION_UPDATE_SCORE | update_score | 매프레임 | 결과 | newScore.exscore > oldScore.exscore | 등록만(ScoreResult) | 2회 [Result,Root] (op1,script1) |
| 331 | OPTION_UPDATE_MAXCOMBO | update_maxcombo | 매프레임 | 결과 | new.combo > old.combo | 등록만(ScoreResult) | 1회 [Result] (op1) |
| 332 | OPTION_UPDATE_MISSCOUNT | update_misscount | 매프레임 | 결과 | new.minbp < old.minbp | 등록만(ScoreResult) | 1회 [Result] (op1) |
| 333 | OPTION_UPDATE_TRIAL | **구현 없음(상수만)** | - | - | 팩토리 null -> 스킨 옵션 맵 대조(사용자 정의 옵션 id 전용), 일치 없으면 객체 제거 | 등록만(ScoreResult) |  |
| 334 | OPTION_UPDATE_IRRANK | **구현 없음(상수만)** | - | - | 팩토리 null -> 스킨 옵션 맵 대조(사용자 정의 옵션 id 전용), 일치 없으면 객체 제거 | 등록만(ScoreResult) |  |
| 335 | OPTION_UPDATE_SCORERANK | update_scorerank | 매프레임 | 공통 | ScoreDataProperty.getNowRate() > getBestScoreRate() | 등록만(ScoreResult) |  |
| 336 | OPTION_UPDATE_TARGET | update_target | 매프레임 | 결과 | PlayerResource.getScoreData().exscore > ScoreDataProperty.getRivalScore() | 등록만(ScoreResult) |  |
| 340 | OPTION_NOW_AAA_1P | rank_now_1p_aaa | 정적(결과) | 플레이,결과 | createNowRank(7) | 구현(플,결) **불일치: R-BMS(플레이)=ex/max_ex(전체 노트 기준), beatoraja=nowrate(통과 노트 기준). 곡 초반 랭크 불일치** |  |
| 341 | OPTION_NOW_AA_1P | rank_now_1p_aa | 정적(결과) | 플레이,결과 | createNowRank(6) | 구현(플,결) |  |
| 342 | OPTION_NOW_A_1P | rank_now_1p_a | 정적(결과) | 플레이,결과 | createNowRank(5) | 구현(플,결) |  |
| 343 | OPTION_NOW_B_1P | rank_now_1p_b | 정적(결과) | 플레이,결과 | createNowRank(4) | 구현(플,결) |  |
| 344 | OPTION_NOW_C_1P | rank_now_1p_c | 정적(결과) | 플레이,결과 | createNowRank(3) | 구현(플,결) |  |
| 345 | OPTION_NOW_D_1P | rank_now_1p_d | 정적(결과) | 플레이,결과 | createNowRank(2) | 구현(플,결) |  |
| 346 | OPTION_NOW_E_1P | rank_now_1p_e | 정적(결과) | 플레이,결과 | createNowRank(1) | 구현(플,결) |  |
| 347 | OPTION_NOW_F_1P | rank_now_1p_f | 정적(결과) | 플레이,결과 | createNowRank(0) | 구현(플,결) |  |
| 350 | OPTION_DISABLE_RESULTFLIP | **구현 없음(상수만)** | - | - | 팩토리 null -> 스킨 옵션 맵 대조(사용자 정의 옵션 id 전용), 일치 없으면 객체 제거 | 등록만(ScoreResult) |  |
| 351 | OPTION_ENABLE_RESULTFLIP | **구현 없음(상수만)** | - | - | 팩토리 null -> 스킨 옵션 맵 대조(사용자 정의 옵션 id 전용), 일치 없으면 객체 제거 | 등록만(ScoreResult) |  |
| 352 | OPTION_1PWIN | result_1pwin | 매프레임 | 결과 | nowEXScore > rivalScore | 등록만(ScoreResult) | 2회 [Select] (other2) |
| 353 | OPTION_2PWIN | result_2pwin | 매프레임 | 결과 | nowEXScore < rivalScore | 등록만(ScoreResult) | 2회 [Select] (other2) |
| 354 | OPTION_DRAW | result_draw | 매프레임 | 결과 | nowEXScore == rivalScore | 등록만(ScoreResult) |  |
| 361 | OPTION_3P_PERFECT | judge_3p_perfect | 매프레임 | 플레이 | getNowJudge(2)==1 | 등록만(PlaySession) |  |
| 362 | OPTION_3P_GREAT | **구현 없음(상수만)** | - | - | 팩토리 null -> 스킨 옵션 맵 대조(사용자 정의 옵션 id 전용), 일치 없으면 객체 제거 | 등록만(PlaySession) |  |
| 363 | OPTION_3P_GOOD | **구현 없음(상수만)** | - | - | 팩토리 null -> 스킨 옵션 맵 대조(사용자 정의 옵션 id 전용), 일치 없으면 객체 제거 | 등록만(PlaySession) |  |
| 400 | OPTION_CONSTANT | constant | 매프레임 | 선곡,플레이 | 선곡=선택바 PlayConfig.isEnableConstant, 플레이=PlayConfig.isEnableConstant, 그 외 false | 등록만(PlayerConfig) | 14회 [Play] (op10,script4) |
| 601 | OPTION_IR_LOADING | **구현 없음(상수만)** | - | - | 팩토리 null -> 스킨 옵션 맵 대조(사용자 정의 옵션 id 전용), 일치 없으면 객체 제거 | 등록만(InternetRanking) |  |
| 602 | OPTION_IR_LOADED | **구현 없음(상수만)** | - | - | 팩토리 null -> 스킨 옵션 맵 대조(사용자 정의 옵션 id 전용), 일치 없으면 객체 제거 | 등록만(InternetRanking) | 1회 [Select] (op1) |
| 603 | OPTION_IR_NOPLAYER | ir_no_player | 매프레임 | 선곡 | RankingData FINISH 이고 totalPlayer==0 | 등록만(InternetRanking) |  |
| 604 | OPTION_IR_FAILED | ir_failed | 매프레임 | 선곡 | RankingData.state==FAIL | 등록만(InternetRanking) |  |
| 605 | OPTION_IR_BANNED | **구현 없음(상수만)** | - | - | 팩토리 null -> 스킨 옵션 맵 대조(사용자 정의 옵션 id 전용), 일치 없으면 객체 제거 | 등록만(InternetRanking) |  |
| 606 | OPTION_IR_WAITING | ir_waiting | 매프레임 | 선곡 | getCurrentRankingData()==null | 등록만(InternetRanking) | 12회 [Result,Select] (op6,script6) |
| 607 | OPTION_IR_ACCESSING | **구현 없음(상수만)** | - | - | 팩토리 null -> 스킨 옵션 맵 대조(사용자 정의 옵션 id 전용), 일치 없으면 객체 제거 | 등록만(InternetRanking) |  |
| 608 | OPTION_IR_BUSY | ir_busy | 매프레임 | 선곡 | 604와 동일 식(state==FAIL) 중복 구현 — 이름과 달리 busy 판정 없음 | 등록만(InternetRanking) |  |
| 624 | OPTION_NOT_COMPARE_RIVAL | not_compare_rival | 매프레임 | 선곡 | MusicSelector.getRival()==null | 없음 |  |
| 625 | OPTION_COMPARE_RIVAL | compare_rival | 매프레임 | 선곡 | getRival()!=null | 없음 | 21회 [Root,Select] (op10,other7,script4) |
| 1002 | OPTION_GRADEBAR_CLASS | course_class | 매프레임 | 선곡 | 선택된 코스에 CLASS 제약 존재 (MusicSelector.existsConstraint) | 등록만(SongSelect) |  |
| 1003 | OPTION_GRADEBAR_MIRROR | course_mirror | 매프레임 | 선곡 | MIRROR 제약 | 등록만(SongSelect) | 1회 [Select] (other1) |
| 1004 | OPTION_GRADEBAR_RANDOM | course_random | 매프레임 | 선곡 | RANDOM 제약 | 등록만(SongSelect) | 1회 [Select] (op1) |
| 1005 | OPTION_GRADEBAR_NOSPEED | course_nospeed | 매프레임 | 선곡 | NO_SPEED 제약 | 등록만(SongSelect) | 1회 [Select] (op1) |
| 1006 | OPTION_GRADEBAR_NOGOOD | course_nogood | 매프레임 | 선곡 | NO_GOOD 제약 | 등록만(SongSelect) |  |
| 1007 | OPTION_GRADEBAR_NOGREAT | course_nogreat | 매프레임 | 선곡 | NO_GREAT 제약 | 등록만(SongSelect) |  |
| 1008 | OPTION_TABLE_SONG | table_song | 정적(선곡 외) | 공통 | PlayerResource.getTablename().length()!=0 | 등록만(SongSelect) |  |
| 1010 | OPTION_GRADEBAR_GAUGE_LR2 | course_gauge_lr2 | 매프레임 | 선곡 | GAUGE_LR2 제약 | 등록만(SongSelect) | 1회 [Select] (op1) |
| 1011 | OPTION_GRADEBAR_GAUGE_5KEYS | course_gauge_5keys | 매프레임 | 선곡 | GAUGE_5KEYS 제약 | 등록만(SongSelect) | 1회 [Select] (other1) |
| 1012 | OPTION_GRADEBAR_GAUGE_7KEYS | course_gauge_7keys | 매프레임 | 선곡 | GAUGE_7KEYS 제약 | 등록만(SongSelect) | 1회 [Select] (other1) |
| 1013 | OPTION_GRADEBAR_GAUGE_9KEYS | course_gauge_9keys | 매프레임 | 선곡 | GAUGE_9KEYS 제약 | 등록만(SongSelect) | 1회 [Select] (other1) |
| 1014 | OPTION_GRADEBAR_GAUGE_24KEYS | course_gauge_24keys | 매프레임 | 선곡 | GAUGE_24KEYS 제약 | 등록만(SongSelect) | 1회 [Select] (other1) |
| 1015 | OPTION_GRADEBAR_LN | course_ln | 매프레임 | 선곡 | LN 제약 | 등록만(SongSelect) | 1회 [Select] (op1) |
| 1016 | OPTION_GRADEBAR_CN | course_cn | 매프레임 | 선곡 | CN 제약 | 등록만(SongSelect) | 1회 [Select] (other1) |
| 1017 | OPTION_GRADEBAR_HCN | course_hcn | 매프레임 | 선곡 | HCN 제약 | 등록만(SongSelect) | 1회 [Select] (other1) |
| 1030 | OPTION_RANDOMSELECTBAR | randomselectbar | 매프레임 | 선곡 | 선택바가 ExecutableBar | 등록만(SongSelect) | 1회 [Select] (op1) |
| 1031 | OPTION_RANDOMCOURSEBAR | randomcoursebar | 매프레임 | 선곡 | 선택바가 RandomCourseBar | 등록만(SongSelect) |  |
| 1046 | OPTION_GAUGE_EX | gauge_ex | 매프레임 | 플레이,결과 | gaugeType in {0,1,4,5,7,8} (EX 판정 계열 게이지) | 구현(플) |  |
| 1047 | OPTION_GAUGE_EX_2P | **구현 없음(상수만)** | - | - | 팩토리 null -> 스킨 옵션 맵 대조(사용자 정의 옵션 id 전용), 일치 없으면 객체 제거 | 구현(플) **[beatoraja엔 없는데 R-BMS만 응답]** |  |
| 1080 | OPTION_STATE_PRACTICE | state_practice | 매프레임 | 플레이 | BMSPlayer.state==STATE_PRACTICE(1) | 없음 | 12회 [Play] (op10,script2) |
| 1100 | OPTION_SELECT_BAR_ASSIST_EASY_CLEARED | select_bar_assist_easy | 매프레임 | 선곡 | 선택바 score.clear==AssistEasy(2) | 등록만(SongSelect) | 4회 [Select] (other4) |
| 1101 | OPTION_SELECT_BAR_LIGHT_ASSIST_EASY_CLEARED | select_bar_light_assist_easy | 매프레임 | 선곡 | clear==LightAssistEasy(3) | 등록만(SongSelect) | 4회 [Select] (other4) |
| 1102 | OPTION_SELECT_BAR_EXHARD_CLEARED | select_bar_exhard | 매프레임 | 선곡 | clear==ExHard(7) | 등록만(SongSelect) | 4회 [Select] (other4) |
| 1103 | OPTION_SELECT_BAR_PERFECT_CLEARED | select_bar_perfect | 매프레임 | 선곡 | clear==Perfect(9) | 등록만(SongSelect) | 4회 [Select] (other4) |
| 1104 | OPTION_SELECT_BAR_MAX_CLEARED | select_bar_max | 매프레임 | 선곡 | clear==Max(10) | 등록만(SongSelect) | 4회 [Select] (op2,other2) |
| 1128 | OPTION_CLEAR_RRANDOM | trophy_option_rrandom | 정적(선곡 외) | 공통 | trophy에 'o' 포함 (R_RANDOM) | 등록만(SongSelect) |  |
| 1129 | OPTION_CLEAR_SPIRAL | trophy_option_spiral | 정적(선곡 외) | 공통 | trophy에 'P' 포함 | 등록만(SongSelect) |  |
| 1130 | OPTION_CLEAR_EXRANDOM | trophy_option_exrandom | 정적(선곡 외) | 공통 | trophy에 'R' 포함 | 등록만(SongSelect) |  |
| 1131 | OPTION_CLEAR_EXSRANDOM | trophy_option_exsrandom | 정적(선곡 외) | 공통 | trophy에 'S' 포함 | 등록만(SongSelect) |  |
| 1160 | OPTION_24KEYSONG | chart_24key | 정적(선곡 외) | 공통 | songdata.getMode()==KEYBOARD_24K.id | 없음 | 2회 [Result,Select] (other2) |
| 1161 | OPTION_24KEYDPSONG | chart_48key | 정적(선곡 외) | 공통 | mode==KEYBOARD_24K_DOUBLE.id | 없음 | 3회 [Result,Select] (other2,script1) |
| 1177 | OPTION_BPMSTOP | chart_bpmstop | 정적(선곡 외) | 공통 | songdata.isBpmstop() | 등록만(SongSelect) |  |
| 1196 | OPTION_NO_REPLAYDATA2 | replaydata_no_2 | 매프레임 | 선곡,결과 | 슬롯1 리플레이 없음 | 없음 | 1회 [Result] (other1) |
| 1197 | OPTION_REPLAYDATA2 | replaydata_exist_2 | 매프레임 | 선곡,결과 | 슬롯1 존재 | 없음 | 5회 [Result,Root,Select] (other2,op2,script1) |
| 1198 | OPTION_REPLAYDATA2_SAVED | replaydata_saved_2 | 매프레임 | 결과 | 슬롯1 SAVED | 없음 | 1회 [Result] (other1) |
| 1199 | OPTION_NO_REPLAYDATA3 | replaydata_no_3 | 매프레임 | 선곡,결과 | 슬롯2 없음 | 없음 | 1회 [Result] (other1) |
| 1200 | OPTION_REPLAYDATA3 | replaydata_exist_3 | 매프레임 | 선곡,결과 | 슬롯2 존재 | 없음 | 5회 [Result,Root,Select] (other2,op2,script1) |
| 1201 | OPTION_REPLAYDATA3_SAVED | replaydata_saved_3 | 매프레임 | 결과 | 슬롯2 SAVED | 없음 | 1회 [Result] (other1) |
| 1202 | OPTION_NO_REPLAYDATA4 | replaydata_no_4 | 매프레임 | 선곡,결과 | 슬롯3 없음 | 없음 | 1회 [Result] (other1) |
| 1203 | OPTION_REPLAYDATA4 | replaydata_exist_4 | 매프레임 | 선곡,결과 | 슬롯3 존재 | 없음 | 5회 [Result,Root,Select] (other2,op2,script1) |
| 1204 | OPTION_REPLAYDATA4_SAVED | replaydata_saved_4 | 매프레임 | 결과 | 슬롯3 SAVED | 없음 | 1회 [Result] (other1) |
| 1205 | OPTION_SELECT_REPLAYDATA | select_replaydata_1 | 매프레임 | 선곡 | MusicSelector.getSelectedReplay()==0 | 없음 | 2회 [Select] (op2) |
| 1206 | OPTION_SELECT_REPLAYDATA2 | select_replaydata_2 | 매프레임 | 선곡 | getSelectedReplay()==1 | 없음 | 2회 [Select] (other1,op1) |
| 1207 | OPTION_SELECT_REPLAYDATA3 | select_replaydata_3 | 매프레임 | 선곡 | getSelectedReplay()==2 | 없음 | 2회 [Select] (other1,op1) |
| 1208 | OPTION_SELECT_REPLAYDATA4 | select_replaydata_4 | 매프레임 | 선곡 | getSelectedReplay()==3 | 없음 | 2회 [Select] (other1,op1) |
| 1240 | OPTION_1P_BORDER_OR_MORE | border_or_more_1p | 매프레임 | 플레이 | GrooveGauge.getGauge().isQualified() (클리어 보더 이상) | 등록만(PlaySession) |  |
| 1242 | OPTION_1P_EARLY | judge_1p_early | 매프레임 | 플레이 | getNowJudge(0)>1 이고 getRecentJudgeTiming(0)>0 (PGREAT가 아닌 판정이 FAST). PG의 early는 아님 | 구현(플) **불일치: R-BMS=판정 있음&&FAST(PG 포함); beatoraja=PG 제외 판정이면서 timing>0. 불일치** | 7회 [Play] (op6,script1) |
| 1243 | OPTION_1P_LATE | judge_1p_late | 매프레임 | 플레이 | getNowJudge(0)>1 이고 recentJudgeTiming<0 (PG 아닌 SLOW) | 구현(플) **불일치: R-BMS=판정 있음&&!FAST(PG 포함); beatoraja=PG 제외 판정이면서 timing<0. 불일치** | 7회 [Play] (op4,other2,script1) |
| 1262 | OPTION_2P_EARLY | judge_2p_early | 매프레임 | 플레이 | getNowJudge(1)>1 이고 timing>0 | 구현(플) **불일치: R-BMS 위와 같음(2P)** | 3회 [Play] (op2,other1) |
| 1263 | OPTION_2P_LATE | judge_2p_late | 매프레임 | 플레이 | getNowJudge(1)>1 이고 timing<0 | 구현(플) **불일치: R-BMS 위와 같음(2P)** | 3회 [Play] (other2,op1) |
| 1330 | OPTION_DRAW_SCORE | draw_score | 매프레임 | 결과 | new.exscore == old.exscore | 등록만(ScoreResult) |  |
| 1331 | OPTION_DRAW_MAXCOMBO | draw_maxcombo | 매프레임 | 결과 | new.combo == old.combo | 등록만(ScoreResult) |  |
| 1332 | OPTION_DRAW_MISSCOUNT | draw_misscount | 매프레임 | 결과 | new.minbp == old.minbp | 등록만(ScoreResult) |  |
| 1335 | OPTION_DRAW_SCORERANK | draw_scorerank | 매프레임 | 공통 | getNowRate()==getBestScoreRate() | 등록만(ScoreResult) |  |
| 1336 | OPTION_DRAW_TARGET | draw_target | 매프레임 | 결과 | PlayerResource.getScoreData().exscore == rivalScore | 등록만(ScoreResult) |  |
| 1362 | OPTION_3P_EARLY | judge_3p_early | 매프레임 | 플레이 | getNowJudge(2)>1 이고 timing>0 | 등록만(PlaySession) |  |
| 1363 | OPTION_3P_LATE | judge_3p_late | 매프레임 | 플레이 | getNowJudge(2)>1 이고 timing<0 | 등록만(PlaySession) |  |
| 2241 | OPTION_PERFECT_EXIST | judge_perfect_exist | 정적(결과) | 플레이,결과 | getJudgeCount(0,true)+(0,false)>0 | 등록만(JudgeCounters) |  |
| 2242 | OPTION_GREAT_EXIST | judge_great_exist | 정적(결과) | 플레이,결과 | 판정1 합>0 | 등록만(JudgeCounters) |  |
| 2243 | OPTION_GOOD_EXIST | judge_good_exist | 정적(결과) | 플레이,결과 | 판정2 합>0 | 등록만(JudgeCounters) | 16회 [Play] (other16) |
| 2244 | OPTION_BAD_EXIST | judge_bad_exist | 정적(결과) | 플레이,결과 | 판정3 합>0 | 등록만(JudgeCounters) | 20회 [Play] (other18,op2) |
| 2245 | OPTION_POOR_EXIST | judge_poor_exist | 정적(결과) | 플레이,결과 | 판정4 합>0 | 등록만(JudgeCounters) | 18회 [Play] (op15,other3) |
| 2246 | OPTION_MISS_EXIST | judge_miss_exist | 정적(결과) | 플레이,결과 | 판정5 합>0 | 등록만(JudgeCounters) |  |
| 3000 | OPTION_PRACTICE_ITEM1 | practice_item1 (패턴) | 매프레임 | 플레이 | BMSPlayer.state==STATE_PRACTICE 이고 PracticeConfiguration.isVisibleItemAvailable(0) | 없음 |  |
| 3001 | (상수 없음) | practice_item2 (패턴) | 매프레임 | 플레이 | BMSPlayer.state==STATE_PRACTICE 이고 PracticeConfiguration.isVisibleItemAvailable(1) | 미선언 |  |
| 3002 | (상수 없음) | practice_item3 (패턴) | 매프레임 | 플레이 | BMSPlayer.state==STATE_PRACTICE 이고 PracticeConfiguration.isVisibleItemAvailable(2) | 미선언 |  |
| 3003 | (상수 없음) | practice_item4 (패턴) | 매프레임 | 플레이 | BMSPlayer.state==STATE_PRACTICE 이고 PracticeConfiguration.isVisibleItemAvailable(3) | 미선언 |  |
| 3004 | (상수 없음) | practice_item5 (패턴) | 매프레임 | 플레이 | BMSPlayer.state==STATE_PRACTICE 이고 PracticeConfiguration.isVisibleItemAvailable(4) | 미선언 |  |
| 3005 | (상수 없음) | practice_item6 (패턴) | 매프레임 | 플레이 | BMSPlayer.state==STATE_PRACTICE 이고 PracticeConfiguration.isVisibleItemAvailable(5) | 미선언 |  |
| 3006 | (상수 없음) | practice_item7 (패턴) | 매프레임 | 플레이 | BMSPlayer.state==STATE_PRACTICE 이고 PracticeConfiguration.isVisibleItemAvailable(6) | 미선언 |  |
| 3007 | (상수 없음) | practice_item8 (패턴) | 매프레임 | 플레이 | BMSPlayer.state==STATE_PRACTICE 이고 PracticeConfiguration.isVisibleItemAvailable(7) | 미선언 |  |
| 3008 | (상수 없음) | practice_item9 (패턴) | 매프레임 | 플레이 | BMSPlayer.state==STATE_PRACTICE 이고 PracticeConfiguration.isVisibleItemAvailable(8) | 미선언 |  |
| 3009 | (상수 없음) | practice_item10 (패턴) | 매프레임 | 플레이 | BMSPlayer.state==STATE_PRACTICE 이고 PracticeConfiguration.isVisibleItemAvailable(9) | 미선언 |  |
| 3010 | (상수 없음) | practice_item11 (패턴) | 매프레임 | 플레이 | BMSPlayer.state==STATE_PRACTICE 이고 PracticeConfiguration.isVisibleItemAvailable(10) | 미선언 |  |
| 3011 | (상수 없음) | practice_item12 (패턴) | 매프레임 | 플레이 | BMSPlayer.state==STATE_PRACTICE 이고 PracticeConfiguration.isVisibleItemAvailable(11) | 미선언 |  |
| 3012 | (상수 없음) | practice_item13 (패턴) | 매프레임 | 플레이 | BMSPlayer.state==STATE_PRACTICE 이고 PracticeConfiguration.isVisibleItemAvailable(12) | 미선언 |  |
| 3013 | (상수 없음) | practice_item14 (패턴) | 매프레임 | 플레이 | BMSPlayer.state==STATE_PRACTICE 이고 PracticeConfiguration.isVisibleItemAvailable(13) | 미선언 |  |
| 3014 | (상수 없음) | practice_item15 (패턴) | 매프레임 | 플레이 | BMSPlayer.state==STATE_PRACTICE 이고 PracticeConfiguration.isVisibleItemAvailable(14) | 미선언 |  |
| 3015 | OPTION_PRACTICE_ITEM16 | practice_item16 (패턴) | 매프레임 | 플레이 | BMSPlayer.state==STATE_PRACTICE 이고 PracticeConfiguration.isVisibleItemAvailable(15) | 없음 |  |
| 3020 | OPTION_PRACTICE_ITEM1_SELECTED | practice_item1_selected (패턴) | 매프레임 | 플레이 | STATE_PRACTICE 이고 isVisibleItemSelected(0) | 없음 |  |
| 3021 | (상수 없음) | practice_item2_selected (패턴) | 매프레임 | 플레이 | STATE_PRACTICE 이고 isVisibleItemSelected(1) | 미선언 |  |
| 3022 | (상수 없음) | practice_item3_selected (패턴) | 매프레임 | 플레이 | STATE_PRACTICE 이고 isVisibleItemSelected(2) | 미선언 |  |
| 3023 | (상수 없음) | practice_item4_selected (패턴) | 매프레임 | 플레이 | STATE_PRACTICE 이고 isVisibleItemSelected(3) | 미선언 |  |
| 3024 | (상수 없음) | practice_item5_selected (패턴) | 매프레임 | 플레이 | STATE_PRACTICE 이고 isVisibleItemSelected(4) | 미선언 |  |
| 3025 | (상수 없음) | practice_item6_selected (패턴) | 매프레임 | 플레이 | STATE_PRACTICE 이고 isVisibleItemSelected(5) | 미선언 |  |
| 3026 | (상수 없음) | practice_item7_selected (패턴) | 매프레임 | 플레이 | STATE_PRACTICE 이고 isVisibleItemSelected(6) | 미선언 |  |
| 3027 | (상수 없음) | practice_item8_selected (패턴) | 매프레임 | 플레이 | STATE_PRACTICE 이고 isVisibleItemSelected(7) | 미선언 |  |
| 3028 | (상수 없음) | practice_item9_selected (패턴) | 매프레임 | 플레이 | STATE_PRACTICE 이고 isVisibleItemSelected(8) | 미선언 |  |
| 3029 | (상수 없음) | practice_item10_selected (패턴) | 매프레임 | 플레이 | STATE_PRACTICE 이고 isVisibleItemSelected(9) | 미선언 |  |
| 3030 | (상수 없음) | practice_item11_selected (패턴) | 매프레임 | 플레이 | STATE_PRACTICE 이고 isVisibleItemSelected(10) | 미선언 |  |
| 3031 | (상수 없음) | practice_item12_selected (패턴) | 매프레임 | 플레이 | STATE_PRACTICE 이고 isVisibleItemSelected(11) | 미선언 |  |
| 3032 | (상수 없음) | practice_item13_selected (패턴) | 매프레임 | 플레이 | STATE_PRACTICE 이고 isVisibleItemSelected(12) | 미선언 |  |
| 3033 | (상수 없음) | practice_item14_selected (패턴) | 매프레임 | 플레이 | STATE_PRACTICE 이고 isVisibleItemSelected(13) | 미선언 |  |
| 3034 | (상수 없음) | practice_item15_selected (패턴) | 매프레임 | 플레이 | STATE_PRACTICE 이고 isVisibleItemSelected(14) | 미선언 |  |
| 3035 | OPTION_PRACTICE_ITEM16_SELECTED | practice_item16_selected (패턴) | 매프레임 | 플레이 | STATE_PRACTICE 이고 isVisibleItemSelected(15) | 없음 |  |

## 5-A. 실수(비율) 속성: RateType (RATE_*, SLIDER_*, BARGRAPH_*)

조회 함수: `FloatPropertyFactory.getRateProperty(int|String)` / 쓰기 `getRateWriter` (FloatPropertyFactory.java:58-94). `SkinSlider.type`, `SkinGraph.type`, Lua/JSON의 `FloatProperty`/`FloatWriter` 필드(`ref`, `value`)는 모두 이 RateType 공간만 본다(SkinSlider.java:49-50, SkinGraph.java:39, LuaSkinLoader.java:126,135, JsonSkinSerializer.java:106,110). SLIDER_*와 BARGRAPH_*는 RATE_*와 같은 id 공간의 별칭이다. 구현 없는 id는 슬라이더/그래프에서 값 0 (`ref!=null?ref.get:0`, SkinSlider.java:113). 값 범위 클램프는 없다.

| id | 상수(RATE/SLIDER/BARGRAPH 별칭) | beatoraja 구현(enum 이름) | 화면 | 값 계산 근거 | 쓰기(FloatWriter) | R-BMS | ModernChic 사용(GRAPH/SLIDER) |
|---|---|---|---|---|---|---|---|
| 1 | RATE_MUSICSELECT_POSITION,SLIDER_MUSICSELECT_POSITION | musicselect_position | 선곡 | BarManager.getSelectedPosition() (선택 바 위치 0..1) | 있음: setSelectedPosition + selectedBarMoved() 호출 | 구현(선) |  |
| 4 | RATE_LANECOVER,SLIDER_LANECOVER | lanecover | 플레이 | PlayConfig.lanecover (lanecover 사용 시), lift 사용 시 lanecover*(1-lift). 비사용/비플레이 0 | - | 등록만(PlayerConfig) | SLIDER 5회 [Play] (type5) |
| 5 | RATE_LANECOVER2,SLIDER_LANECOVER2 | lanecover2 | 플레이 | id 4와 같은 createLanecover() 인스턴스 로직 (값 동일) | - | 등록만(PlayerConfig) |  |
| 6 | RATE_MUSIC_PROGRESS,SLIDER_MUSIC_PROGRESS | music_progress | 플레이 | TIMER_PLAY 켜짐이면 min(nowTime(PLAY)/BMSPlayer.getPlaytime(),1), 아니면 0 | - | 구현(플) **불일치: R-BMS=song_ms/duration_ms; beatoraja=nowTime(PLAY)/BMSPlayer.getPlaytime(). 분모 정의 동일한지 미확인** | SLIDER 3회 [Play] (type3) |
| 7 | RATE_SKINSELECT_POSITION,SLIDER_SKINSELECT_POSITION | skinselect_position | 스킨설정 | SkinConfiguration.getSkinSelectPosition() | 있음: setSkinSelectPosition | 등록만(SkinCustomize) | SLIDER 1회 [skinselect] (type1) |
| 8 | RATE_RANKING_POSITION | ranking_position | 선곡,결과 | MusicSelector/AbstractResult.getRankingPosition() | 있음: setRankingPosition | 등록만(InternetRanking) | SLIDER 1회 [Result] (type1) |
| 17 | RATE_MASTERVOLUME,SLIDER_MASTER_VOLUME | mastervolume | 공통 | AudioConfig.systemvolume (0..1) | 있음: setSystemvolume | 등록만(PlayerConfig) | SLIDER 2회 [Select] (type2) |
| 18 | RATE_KEYVOLUME,SLIDER_KEY_VOLUME | keyvolume | 공통 | AudioConfig.keyvolume | 있음: setKeyvolume | 등록만(PlayerConfig) | SLIDER 2회 [Select] (type2) |
| 19 | RATE_BGMVOLUME,SLIDER_BGM_VOLUME | bgmvolume | 공통 | AudioConfig.bgvolume | 있음: setBgvolume | 등록만(PlayerConfig) | SLIDER 2회 [Select] (type2) |
| 20 | SLIDER_PRACTICE_POSITION | practice_position | 플레이 | BMSPlayer.getPracticeConfiguration().getItemScrollPosition() | 있음: setItemScrollPosition | 없음 |  |
| 101 | RATE_MUSIC_PROGRESS_BAR,BARGRAPH_MUSIC_PROGRESS | music_progress_bar | 플레이 | id 6과 동일 createMusicProgress() | - | 구현(플) |  |
| 102 | RATE_LOAD_PROGRESS,BARGRAPH_LOAD_PROGRESS | load_progress | 공통 | BGA켜짐?(bga.progress+audio.progress)/2:audio.progress | - | 구현(결정) | GRAPH 2회 [Play] (type2) |
| 103 | RATE_LEVEL,BARGRAPH_LEVEL | level | 선곡 | 선택바 SongBar 난이도 무관 레벨/최대레벨. 최대레벨 계산에 switch fall-through 버그: 모드 5,10,7,14,9,25,50 어느 쪽이든 maxLevel=10으로 귀결 (그 외 모드 0) | - | 등록만(SongSelect) |  |
| 105 | RATE_LEVEL_BEGINNER,BARGRAPH_LEVEL_BEGINNER | level_beginner | 선곡 | 103과 같되 songdata.difficulty==1일 때만(아니면 0) | - | 등록만(SongSelect) |  |
| 106 | RATE_LEVEL_NORMAL,BARGRAPH_LEVEL_NORMAL | level_normal | 선곡 | difficulty==2일 때만 | - | 등록만(SongSelect) |  |
| 107 | RATE_LEVEL_HYPER,BARGRAPH_LEVEL_HYPER | level_hyper | 선곡 | difficulty==3일 때만 | - | 등록만(SongSelect) |  |
| 108 | RATE_LEVEL_ANOTHER,BARGRAPH_LEVEL_ANOTHER | level_another | 선곡 | difficulty==4일 때만 | - | 등록만(SongSelect) |  |
| 109 | RATE_LEVEL_INSANE,BARGRAPH_LEVEL_INSANE | level_insane | 선곡 | difficulty==5일 때만 | - | 등록만(SongSelect) |  |
| 110 | RATE_SCORE,BARGRAPH_SCORERATE | scorerate | 플레이,결과 | ScoreDataProperty.getRate() = ex/(scoreData.notes*2) (totalnotes==0이면 1.0) | - | 등록만(PlaySession) | GRAPH 2회 [Play] (type2) |
| 111 | RATE_SCORE_FINAL,BARGRAPH_SCORERATE_FINAL | scorerate_final | 플레이,결과 | ScoreDataProperty.getNowRate() = ex/(통과노트*2) | - | 등록만(PlaySession) | GRAPH 2회 [Play] (type2) |
| 112 | RATE_BESTSCORE_NOW,BARGRAPH_BESTSCORERATE_NOW | bestscorerate_now | 플레이 | getNowBestScoreRate() (베스트 고스트/비례 진행값) | - | 등록만(ScoreResult) | GRAPH 2회 [Play] (type2) |
| 113 | RATE_BESTSCORE,BARGRAPH_BESTSCORERATE | bestscorerate | 공통 | getBestScoreRate() = bestscore/(totalnotes*2) | - | 구현(플,선,결) | GRAPH 4회 [Play,Root] (script2,type2) |
| 114 | RATE_TARGETSCORE_NOW,BARGRAPH_TARGETSCORERATE_NOW | targetscorerate_now | 플레이 | getNowRivalScoreRate() | - | 등록만(ScoreResult) | GRAPH 2회 [Play] (type2) |
| 115 | RATE_TARGETSCORE,BARGRAPH_TARGETSCORERATE | targetscorerate | 공통 | getRivalScoreRate() | - | 구현(플,결) | GRAPH 2회 [Play] (type2) |
| 140 | RATE_PGREAT,BARGRAPH_RATE_PGREAT | rate_pgreat | 선곡 | 선택바 SongBar/GradeBar score.judgeCount(0)/songNotes (곡 노트 수; 코스는 전곡 노트 합). 그 외 0 | - | 구현(플,결) **불일치: beatoraja=선곡 전용(선택바 score/노트). R-BMS는 플레이/결과 판정 비율을 응답** | GRAPH 1회 [Select] (value1) |
| 141 | RATE_GREAT,BARGRAPH_RATE_GREAT | rate_great | 선곡 | judgeCount(1)/notes | - | 구현(플,결) **불일치: 위와 같음** | GRAPH 1회 [Select] (value1) |
| 142 | RATE_GOOD,BARGRAPH_RATE_GOOD | rate_good | 선곡 | judgeCount(2)/notes | - | 구현(플,결) **불일치: 위와 같음** | GRAPH 1회 [Select] (value1) |
| 143 | RATE_BAD,BARGRAPH_RATE_BAD | rate_bad | 선곡 | judgeCount(3)/notes | - | 구현(플,결) **불일치: 위와 같음** | GRAPH 1회 [Select] (value1) |
| 144 | RATE_POOR,BARGRAPH_RATE_POOR | rate_poor | 선곡 | judgeCount(4)/notes | - | 구현(플,결) **불일치: 위와 같음** | GRAPH 1회 [Select] (value1) |
| 145 | RATE_MAXCOMBO,BARGRAPH_RATE_MAXCOMBO | rate_maxcombo | 선곡 | score.combo/notes | - | 등록만(JudgeCounters) |  |
| 146 | BARGRAPH_RATE_SCORE | **구현 없음(상수만)** | - | RateType에 없음 -> 값 0 | - | 등록만(JudgeCounters) |  |
| 147 | RATE_EXSCORE,BARGRAPH_RATE_EXSCORE | rate_exscore | 선곡 | score.exscore/notes/2 (SongBar), GradeBar는 /notes (2 나눗셈 없음) | - | 구현(플,결) **불일치: beatoraja=선곡 전용(선택바 score), 플레이/결과는 0. R-BMS는 플레이/결과에서 ex/max 응답** | GRAPH 1회 [Select] (value1) |

## 5-B. 실수 속성: FloatType (FLOAT_*) 와 IR 비율 패턴

조회 함수: `FloatPropertyFactory.getFloatProperty(int|String)` (FloatPropertyFactory.java:103-126). 해석 순서는 `FloatPropertyPattern`(IR 비율 11개) -> `FloatType` -> 없으면 RateType. `SkinFloat`(floatvalue 객체, SkinFloat.java:99,103)와 Lua `main_state.float_number`(MainStatePropertyLuaApiExporter.java:144)가 이 함수를 쓴다. 무값은 `Float.MIN_VALUE`(양의 최소 float 1.4e-45, 음수가 아님)이며 `SkinFloat`은 MIN_VALUE/MAX_VALUE/무한/NaN이면 그리지 않는다(SkinFloat.java:155). FloatType에서 읽을 때 같은 id가 NUMBER와 의미가 다른 경우: 372/374/376(정수는 ms 반올림, 실수는 초 단위 또는 배열중심), 1107(퍼센트 0..100).

| id | 상수 | beatoraja 구현(enum 이름) | 화면 | 값 계산 근거 | R-BMS | ModernChic 사용 |
|---|---|---|---|---|---|---|
| 85 | FLOAT_PERFECT_RATE | perfect_rate | 공통 | scoreData.notes>0이면 judgeCount(0)/notes (0..1), 없으면 Float.MIN_VALUE | 등록만(JudgeCounters) | - |
| 86 | FLOAT_GREAT_RATE | great_rate | 공통 | judgeCount(1)/notes | 등록만(JudgeCounters) | - |
| 87 | FLOAT_GOOD_RATE | good_rate | 공통 | judgeCount(2)/notes | 등록만(JudgeCounters) | - |
| 88 | FLOAT_BAD_RATE | bad_rate | 공통 | judgeCount(3)/notes | 등록만(JudgeCounters) | - |
| 89 | FLOAT_POOR_RATE | poor_rate | 공통 | judgeCount(4)/notes | 등록만(JudgeCounters) | - |
| 122 | FLOAT_RIVAL_RATE | rival_rate | 공통 | ScoreDataProperty.getRivalScoreRate() | 없음 | - |
| 135 | FLOAT_TARGET_RATE | target_rate | 공통 | rival_rate와 같은 프로퍼티 인스턴스 | 구현(플,결) | - |
| 155 | FLOAT_SCORE_RATE2 | score_rate2 | 공통 | total_rate(1115)와 같은 프로퍼티 (getRate) | 등록만(ScoreResult) | - |
| 157 | FLOAT_TARGET_RATE2 | target_rate2 | 공통 | rival_rate와 같은 인스턴스 | 등록만(ScoreResult) | - |
| 165 | FLOAT_LOADING_PROGRESS | loading_progress | 공통 | BGA켜짐?(bga.progress+audio.progress)/2:audio.progress (0..1) | 구현(결정) | - |
| 183 | FLOAT_BEST_RATE | best_rate | 공통 | getBestScoreRate() | 등록만(ScoreResult) | - |
| 203 | FLOAT_IR_PLAYER_NOPLAY_RATE | ir_player_noplay_rate (패턴) | 선곡,결과 | RankingData FINISH이고 totalPlayer>0일 때 count(0)/totalPlayer (0..1), 아니면 Float.MIN_VALUE | 등록만(InternetRanking) | - |
| 205 | FLOAT_IR_PLAYER_ASSIST_RATE | ir_player_assist_rate (패턴) | 선곡,결과 | RankingData FINISH이고 totalPlayer>0일 때 count(2)/totalPlayer (0..1), 아니면 Float.MIN_VALUE | 등록만(InternetRanking) | - |
| 207 | FLOAT_IR_PLAYER_LIGHTASSIST_RATE | ir_player_lightassist_rate (패턴) | 선곡,결과 | RankingData FINISH이고 totalPlayer>0일 때 count(3)/totalPlayer (0..1), 아니면 Float.MIN_VALUE | 등록만(InternetRanking) | - |
| 209 | FLOAT_IR_PLAYER_EXHARD_RATE | ir_player_exhard_rate (패턴) | 선곡,결과 | RankingData FINISH이고 totalPlayer>0일 때 count(7)/totalPlayer (0..1), 아니면 Float.MIN_VALUE | 등록만(InternetRanking) | - |
| 211 | FLOAT_IR_PLAYER_FAILED_RATE | ir_player_failed_rate (패턴) | 선곡,결과 | RankingData FINISH이고 totalPlayer>0일 때 count(1)/totalPlayer (0..1), 아니면 Float.MIN_VALUE | 등록만(InternetRanking) | - |
| 213 | FLOAT_IR_PLAYER_EASY_RATE | ir_player_easy_rate (패턴) | 선곡,결과 | RankingData FINISH이고 totalPlayer>0일 때 count(4)/totalPlayer (0..1), 아니면 Float.MIN_VALUE | 등록만(InternetRanking) | - |
| 215 | FLOAT_IR_PLAYER_NORMAL_RATE | ir_player_normal_rate (패턴) | 선곡,결과 | RankingData FINISH이고 totalPlayer>0일 때 count(5)/totalPlayer (0..1), 아니면 Float.MIN_VALUE | 등록만(InternetRanking) | - |
| 217 | FLOAT_IR_PLAYER_HARD_RATE | ir_player_hard_rate (패턴) | 선곡,결과 | RankingData FINISH이고 totalPlayer>0일 때 count(6)/totalPlayer (0..1), 아니면 Float.MIN_VALUE | 등록만(InternetRanking) | - |
| 219 | FLOAT_IR_PLAYER_FULLCOMBO_RATE | ir_player_fullcombo_rate (패턴) | 선곡,결과 | RankingData FINISH이고 totalPlayer>0일 때 count(8)/totalPlayer (0..1), 아니면 Float.MIN_VALUE | 등록만(InternetRanking) | - |
| 223 | FLOAT_IR_PLAYER_PERFECT_RATE | ir_player_perfect_rate (패턴) | 선곡,결과 | RankingData FINISH이고 totalPlayer>0일 때 count(9)/totalPlayer (0..1), 아니면 Float.MIN_VALUE | 등록만(InternetRanking) | - |
| 225 | FLOAT_IR_PLAYER_MAX_RATE | ir_player_max_rate (패턴) | 선곡,결과 | RankingData FINISH이고 totalPlayer>0일 때 count(10)/totalPlayer (0..1), 아니면 Float.MIN_VALUE | 등록만(InternetRanking) | - |
| 227 | FLOAT_IR_TOTALCLEARRATE | ir_totalclearrate | 선곡,결과 | clearType 2..10 합/totalPlayer (0..1), 데이터 없으면 Float.MIN_VALUE | 등록만(InternetRanking) | - |
| 229 | FLOAT_IR_TOTALFULLCOMBORATE | ir_totalfullcomborate | 선곡,결과 | clearType 8,9,10 합/totalPlayer | 등록만(InternetRanking) | - |
| 285 | FLOAT_RIVAL_PERFECT_RATE | rival_perfect_rate | 공통 | rivalScoreData.judgeCount(0)/notes | 없음 | - |
| 286 | FLOAT_RIVAL_GREAT_RATE | rival_great_rate | 공통 | judgeCount(1)/notes | 없음 | - |
| 287 | FLOAT_RIVAL_GOOD_RATE | rival_good_rate | 공통 | judgeCount(2)/notes | 없음 | - |
| 288 | FLOAT_RIVAL_BAD_RATE | rival_bad_rate | 공통 | judgeCount(3)/notes | 없음 | - |
| 289 | FLOAT_RIVAL_POOR_RATE | rival_poor_rate | 공통 | judgeCount(4)/notes | 없음 | - |
| 310 | FLOAT_HISPEED | hispeed | 공통 | 플레이=LaneRenderer.getHispeed(), 그 외 songdata 모드 PlayConfig.hispeed, 없으면 Float.MIN_VALUE | 등록만(PlayerConfig) | - |
| 360 | FLOAT_CHART_PEAKDENSITY | chart_peakdensity | 공통 | information.getPeakdensity() (float) | 등록만(SongSelect) | - |
| 362 | FLOAT_CHART_ENDDENSITY | chart_enddensity | 공통 | information.getEnddensity() | 등록만(SongSelect) | - |
| 367 | FLOAT_CHART_AVERAGEDENSITY | chart_averagedensity | 공통 | information.getDensity() | 등록만(SongSelect) | - |
| 368 | FLOAT_CHART_TOTALGAUGE | chart_totalgauge | 공통 | information.getTotal() | 등록만(SongSelect) | - |
| 372 | FLOAT_DURATION_AVERAGE | duration_average | 결과 | AbstractResult.getAverageDuration()/1000.0f | 등록만(JudgeCounters) | - |
| 374 | FLOAT_TIMING_AVERAGE | timing_average | 결과 | TimingDistribution.getArrayCenter()/1000.0f (정수 NUMBER 374는 getAverage()와 다른 값 사용) | 등록만(JudgeCounters) | - |
| 376 | FLOAT_TIMIGN_STDDEV | timign_stddev | 결과 | TimingDistribution.getStdDev() (enum 이름 오타 timign) | 등록만(JudgeCounters) | - |
| 1102 | FLOAT_SCORE_RATE | score_rate | 공통 | scoreData!=null이면 getNowRate() (ex/(통과노트*2)), 없으면 Float.MIN_VALUE | 구현(플,결) **불일치: R-BMS=ex/max_ex(전체 기준), beatoraja=getNowRate(통과 노트 기준). 불일치** | - |
| 1107 | FLOAT_GROOVEGAUGE_1P | groovegauge_1p | 플레이,결과 | 플레이=gauge.getValue() (0..100 퍼센트 값), 결과=gauge log 마지막 값 (0..1이 아님) | 구현(플,결) **불일치: R-BMS=gauge/100 (0..1), beatoraja=게이지 값 그대로(0..100). 단위 불일치** | - |
| 1115 | FLOAT_TOTAL_RATE | total_rate | 공통 | getRate() (ex/(scoreData.notes*2)) | 등록만(PlaySession) | - |

## 6. 문자열 속성 (STRING_*)

조회 함수: `StringPropertyFactory.getStringProperty(int|String)` / 쓰기 `getStringWriter` (StringPropertyFactory.java:42-74). 순서는 `StringPropertyPattern`(번호형 패턴) -> `StringType`. 구현 없는 id는 null이며 SkinText는 빈 문자열을 그린다(`getStringProperty(-1)`은 null -> 호출부 의존). Lua `main_state.text(id)`는 null이면 빈 문자열(MainStatePropertyLuaApiExporter.java:185). `STRING_SKIN_CUSTOMIZE_CATEGORY1..10`/`ITEM1..10`은 `SkinPropertyMapper.isSkinCustomize*`로 판정(양끝 포함, SkinPropertyMapper.java:132-146).

| id | 상수 | beatoraja 구현(enum/패턴 이름) | 화면 | 값 계산 근거 | 쓰기(StringWriter) | R-BMS | ModernChic 사용 |
|---|---|---|---|---|---|---|---|
| 1 | STRING_RIVAL | rival | 선곡,기타 | 선곡=선택된 라이벌 PlayerInformation.getName(), 그 외 PlayerResource.getTargetScoreData().getPlayer() | - | 등록만(PlayerConfig) | 4회 [Play,Select] (ref4) |
| 2 | STRING_PLAYER | player | 공통 | PlayerConfig.getName() | - | 등록만(PlayerConfig) | 3회 [Result,Select] (ref2,script1) |
| 3 | (상수 없음) | target | 선곡,기타 | 선곡=TargetProperty.getTargetName(playerConfig.targetid), 그 외 targetScoreData.getPlayer() (SkinProperty에 상수 없음) | - | 미선언 | 2회 [Select] (ref2) |
| 10 | STRING_TITLE | title | 공통 | 선곡에서 선택바가 DirectoryBar면 바 제목; 결정/코스결과에서 코스 제목이 있으면 코스 제목; 그 외 songdata.getTitle() | - | 구현(플,선,결,결정) | 12회 [Result,Select] (script10,ref2) |
| 11 | STRING_SUBTITLE | subtitle | 공통 | songdata.getSubtitle() | - | 구현(선,결정) | 2회 [Select] (ref2) |
| 12 | STRING_FULLTITLE | fulltitle | 공통 | 10과 같은 우선순위, 곡은 songdata.getFullTitle() (title+subtitle) | - | 구현(플,선,결,결정) | 13회 [Decide,Play,Result,Root] (script7,ref6) |
| 13 | STRING_GENRE | genre | 공통 | songdata.getGenre() | - | 구현(선,결정) | 10회 [Decide,Play,Result,Root,Select] (ref8,script2) |
| 14 | STRING_ARTIST | artist | 공통 | songdata.getArtist() | - | 구현(플,선,결,결정) | 14회 [Decide,Play,Result,Root,Select] (script10,ref4) |
| 15 | STRING_SUBARTIST | subartist | 공통 | songdata.getSubartist() | - | 등록만(SongSelect) | 8회 [Decide,Play,Select] (script6,ref2) |
| 16 | STRING_FULLARTIST | fullartist | 공통 | songdata.getFullArtist() | - | 구현(플,선,결,결정) |  |
| 30 | STRING_SEARCHWORD | searchword | 선곡 | 읽기는 항상 빈 문자열. 쓰기 시 MusicSelector.search(value) | 있음 | 구현(선) **불일치: beatoraja 읽기는 항상 빈 문자열(쓰기 전용). R-BMS는 현재 검색어를 돌려줌** | 2회 [Select] (ref2) |
| 40 | (상수 없음) | key1 (패턴) | 키컨피그 | KeyConfiguration.getKeyAssign(0) (키 1번째 할당 문자열) | - | 구현(키) |  |
| 41 | (상수 없음) | key2 (패턴) | 키컨피그 | KeyConfiguration.getKeyAssign(1) (키 2번째 할당 문자열) | - | 구현(키) |  |
| 42 | (상수 없음) | key3 (패턴) | 키컨피그 | KeyConfiguration.getKeyAssign(2) (키 3번째 할당 문자열) | - | 구현(키) |  |
| 43 | (상수 없음) | key4 (패턴) | 키컨피그 | KeyConfiguration.getKeyAssign(3) (키 4번째 할당 문자열) | - | 구현(키) |  |
| 44 | (상수 없음) | key5 (패턴) | 키컨피그 | KeyConfiguration.getKeyAssign(4) (키 5번째 할당 문자열) | - | 구현(키) |  |
| 45 | (상수 없음) | key6 (패턴) | 키컨피그 | KeyConfiguration.getKeyAssign(5) (키 6번째 할당 문자열) | - | 구현(키) |  |
| 46 | (상수 없음) | key7 (패턴) | 키컨피그 | KeyConfiguration.getKeyAssign(6) (키 7번째 할당 문자열) | - | 구현(키) |  |
| 47 | (상수 없음) | key8 (패턴) | 키컨피그 | KeyConfiguration.getKeyAssign(7) (키 8번째 할당 문자열) | - | 구현(키) |  |
| 48 | (상수 없음) | key9 (패턴) | 키컨피그 | KeyConfiguration.getKeyAssign(8) (키 9번째 할당 문자열) | - | 구현(키) |  |
| 49 | (상수 없음) | key10 (패턴) | 키컨피그 | KeyConfiguration.getKeyAssign(9) (키 10번째 할당 문자열) | - | 구현(키) |  |
| 50 | STRING_SKIN_NAME | skinname | 공통 | 스킨설정=선택한 SkinHeader 이름, 그 외 현재 스킨 header.getName() | - | 등록만(SkinCustomize) | 1회 [SkinSelect] (ref1) |
| 51 | STRING_SKIN_AUTHOR | skinauthor | 공통 | SkinHeader.getAuthor() | - | 등록만(SkinCustomize) | 1회 [SkinSelect] (ref1) |
| 60 | (상수 없음) | mode | 선곡 | PlayerConfig.getModeFilter().getDisplayName() (SkinProperty에 상수 없음) | - | 미선언 |  |
| 61 | (상수 없음) | sort | 선곡 | PlayerConfig.getSortid() (BarSorter 이름, 상수 없음) | - | 미선언 |  |
| 62 | (상수 없음) | difficulty | 선곡 | PlayerConfig.getDifficultyFilter().getDisplayName() (상수 없음) | - | 미선언 |  |
| 86 | (상수 없음) | chartreplication | 선곡 | PlayerConfig.getChartReplicationMode() (상수 없음) | - | 미선언 |  |
| 100 | STRING_SKIN_CUSTOMIZE_CATEGORY1 | skincategory1 (패턴) | 스킨설정 | SkinConfiguration.getCategoryName(0) (customOptionOffset 반영) | - | 등록만(SkinCustomize) | 1회 [SkinSelect] (op1) |
| 101 | (상수 없음) | skincategory2 (패턴) | 스킨설정 | SkinConfiguration.getCategoryName(1) (customOptionOffset 반영) | - | 등록만(SkinCustomize) | 1회 [SkinSelect] (other1) |
| 102 | (상수 없음) | skincategory3 (패턴) | 스킨설정 | SkinConfiguration.getCategoryName(2) (customOptionOffset 반영) | - | 등록만(SkinCustomize) | 1회 [SkinSelect] (other1) |
| 103 | (상수 없음) | skincategory4 (패턴) | 스킨설정 | SkinConfiguration.getCategoryName(3) (customOptionOffset 반영) | - | 등록만(SkinCustomize) | 1회 [SkinSelect] (other1) |
| 104 | (상수 없음) | skincategory5 (패턴) | 스킨설정 | SkinConfiguration.getCategoryName(4) (customOptionOffset 반영) | - | 등록만(SkinCustomize) | 1회 [SkinSelect] (other1) |
| 105 | (상수 없음) | skincategory6 (패턴) | 스킨설정 | SkinConfiguration.getCategoryName(5) (customOptionOffset 반영) | - | 등록만(SkinCustomize) | 1회 [SkinSelect] (other1) |
| 106 | (상수 없음) | skincategory7 (패턴) | 스킨설정 | SkinConfiguration.getCategoryName(6) (customOptionOffset 반영) | - | 등록만(SkinCustomize) | 1회 [SkinSelect] (other1) |
| 107 | (상수 없음) | skincategory8 (패턴) | 스킨설정 | SkinConfiguration.getCategoryName(7) (customOptionOffset 반영) | - | 등록만(SkinCustomize) | 1회 [SkinSelect] (other1) |
| 108 | (상수 없음) | skincategory9 (패턴) | 스킨설정 | SkinConfiguration.getCategoryName(8) (customOptionOffset 반영) | - | 등록만(SkinCustomize) | 1회 [SkinSelect] (other1) |
| 109 | STRING_SKIN_CUSTOMIZE_CATEGORY10 | skincategory10 (패턴) | 스킨설정 | SkinConfiguration.getCategoryName(9) (customOptionOffset 반영) | - | 등록만(SkinCustomize) | 1회 [SkinSelect] (other1) |
| 110 | STRING_SKIN_CUSTOMIZE_ITEM1 | skinitem1 (패턴) | 스킨설정 | SkinConfiguration.getDisplayValue(0) | - | 등록만(SkinCustomize) | 1회 [SkinSelect] (op1) |
| 111 | (상수 없음) | skinitem2 (패턴) | 스킨설정 | SkinConfiguration.getDisplayValue(1) | - | 등록만(SkinCustomize) | 1회 [SkinSelect] (other1) |
| 112 | (상수 없음) | skinitem3 (패턴) | 스킨설정 | SkinConfiguration.getDisplayValue(2) | - | 등록만(SkinCustomize) | 1회 [SkinSelect] (other1) |
| 113 | (상수 없음) | skinitem4 (패턴) | 스킨설정 | SkinConfiguration.getDisplayValue(3) | - | 등록만(SkinCustomize) | 1회 [SkinSelect] (other1) |
| 114 | (상수 없음) | skinitem5 (패턴) | 스킨설정 | SkinConfiguration.getDisplayValue(4) | - | 등록만(SkinCustomize) | 1회 [SkinSelect] (other1) |
| 115 | (상수 없음) | skinitem6 (패턴) | 스킨설정 | SkinConfiguration.getDisplayValue(5) | - | 등록만(SkinCustomize) | 1회 [SkinSelect] (other1) |
| 116 | (상수 없음) | skinitem7 (패턴) | 스킨설정 | SkinConfiguration.getDisplayValue(6) | - | 등록만(SkinCustomize) | 1회 [SkinSelect] (other1) |
| 117 | (상수 없음) | skinitem8 (패턴) | 스킨설정 | SkinConfiguration.getDisplayValue(7) | - | 등록만(SkinCustomize) | 1회 [SkinSelect] (other1) |
| 118 | (상수 없음) | skinitem9 (패턴) | 스킨설정 | SkinConfiguration.getDisplayValue(8) | - | 등록만(SkinCustomize) | 1회 [SkinSelect] (other1) |
| 119 | STRING_SKIN_CUSTOMIZE_ITEM10 | skinitem10 (패턴) | 스킨설정 | SkinConfiguration.getDisplayValue(9) | - | 등록만(SkinCustomize) | 1회 [SkinSelect] (other1) |
| 120 | STRING_RANKING1_NAME | rankingname1 (패턴) | 선곡,결과 | RankingData.getScore(index0+rankingOffset).player, 없으면 빈 문자열 | - | 등록만(InternetRanking) |  |
| 121 | (상수 없음) | rankingname2 (패턴) | 선곡,결과 | RankingData.getScore(index1+rankingOffset).player, 없으면 빈 문자열 | - | 등록만(InternetRanking) |  |
| 122 | (상수 없음) | rankingname3 (패턴) | 선곡,결과 | RankingData.getScore(index2+rankingOffset).player, 없으면 빈 문자열 | - | 등록만(InternetRanking) |  |
| 123 | (상수 없음) | rankingname4 (패턴) | 선곡,결과 | RankingData.getScore(index3+rankingOffset).player, 없으면 빈 문자열 | - | 등록만(InternetRanking) |  |
| 124 | (상수 없음) | rankingname5 (패턴) | 선곡,결과 | RankingData.getScore(index4+rankingOffset).player, 없으면 빈 문자열 | - | 등록만(InternetRanking) |  |
| 125 | (상수 없음) | rankingname6 (패턴) | 선곡,결과 | RankingData.getScore(index5+rankingOffset).player, 없으면 빈 문자열 | - | 등록만(InternetRanking) |  |
| 126 | (상수 없음) | rankingname7 (패턴) | 선곡,결과 | RankingData.getScore(index6+rankingOffset).player, 없으면 빈 문자열 | - | 등록만(InternetRanking) |  |
| 127 | (상수 없음) | rankingname8 (패턴) | 선곡,결과 | RankingData.getScore(index7+rankingOffset).player, 없으면 빈 문자열 | - | 등록만(InternetRanking) |  |
| 128 | (상수 없음) | rankingname9 (패턴) | 선곡,결과 | RankingData.getScore(index8+rankingOffset).player, 없으면 빈 문자열 | - | 등록만(InternetRanking) |  |
| 129 | STRING_RANKING10_NAME | rankingname10 (패턴) | 선곡,결과 | RankingData.getScore(index9+rankingOffset).player, 없으면 빈 문자열 | - | 등록만(InternetRanking) |  |
| 150 | STRING_COURSE1_TITLE | coursetitle1 (패턴) | 선곡,기타 | 선곡=선택바 GradeBar의 1번째 곡 제목("(no song) "접두 규칙)/RandomCourseBar 스테이지 제목, 그 외 PlayerResource.getCourseData().getSong()[0].getTitle() | - | 없음 | 3회 [Result,Root] (script3) |
| 151 | STRING_COURSE2_TITLE | coursetitle2 (패턴) | 선곡,기타 | 선곡=선택바 GradeBar의 2번째 곡 제목("(no song) "접두 규칙)/RandomCourseBar 스테이지 제목, 그 외 PlayerResource.getCourseData().getSong()[1].getTitle() | - | 없음 |  |
| 152 | STRING_COURSE3_TITLE | coursetitle3 (패턴) | 선곡,기타 | 선곡=선택바 GradeBar의 3번째 곡 제목("(no song) "접두 규칙)/RandomCourseBar 스테이지 제목, 그 외 PlayerResource.getCourseData().getSong()[2].getTitle() | - | 없음 |  |
| 153 | STRING_COURSE4_TITLE | coursetitle4 (패턴) | 선곡,기타 | 선곡=선택바 GradeBar의 4번째 곡 제목("(no song) "접두 규칙)/RandomCourseBar 스테이지 제목, 그 외 PlayerResource.getCourseData().getSong()[3].getTitle() | - | 없음 |  |
| 154 | STRING_COURSE5_TITLE | coursetitle5 (패턴) | 선곡,기타 | 선곡=선택바 GradeBar의 5번째 곡 제목("(no song) "접두 규칙)/RandomCourseBar 스테이지 제목, 그 외 PlayerResource.getCourseData().getSong()[4].getTitle() | - | 없음 |  |
| 155 | STRING_COURSE6_TITLE | coursetitle6 (패턴) | 선곡,기타 | 선곡=선택바 GradeBar의 6번째 곡 제목("(no song) "접두 규칙)/RandomCourseBar 스테이지 제목, 그 외 PlayerResource.getCourseData().getSong()[5].getTitle() | - | 없음 | 8회 [Result,Root] (script8) |
| 156 | STRING_COURSE7_TITLE | coursetitle7 (패턴) | 선곡,기타 | 선곡=선택바 GradeBar의 7번째 곡 제목("(no song) "접두 규칙)/RandomCourseBar 스테이지 제목, 그 외 PlayerResource.getCourseData().getSong()[6].getTitle() | - | 없음 |  |
| 157 | STRING_COURSE8_TITLE | coursetitle8 (패턴) | 선곡,기타 | 선곡=선택바 GradeBar의 8번째 곡 제목("(no song) "접두 규칙)/RandomCourseBar 스테이지 제목, 그 외 PlayerResource.getCourseData().getSong()[7].getTitle() | - | 없음 |  |
| 158 | STRING_COURSE9_TITLE | coursetitle9 (패턴) | 선곡,기타 | 선곡=선택바 GradeBar의 9번째 곡 제목("(no song) "접두 규칙)/RandomCourseBar 스테이지 제목, 그 외 PlayerResource.getCourseData().getSong()[8].getTitle() | - | 없음 |  |
| 159 | STRING_COURSE10_TITLE | coursetitle10 (패턴) | 선곡,기타 | 선곡=선택바 GradeBar의 10번째 곡 제목("(no song) "접두 규칙)/RandomCourseBar 스테이지 제목, 그 외 PlayerResource.getCourseData().getSong()[9].getTitle() | - | 없음 |  |
| 200 | (상수 없음) | targetnamep10 (패턴) | 공통 | TargetProperty 목록에서 현재 target 기준 10칸 이전 목록의 표시 이름 (원형 순환) | - | 미선언 | 1회 [Select] (op1) |
| 201 | (상수 없음) | targetnamep9 (패턴) | 공통 | TargetProperty 목록에서 현재 target 기준 9칸 이전 목록의 표시 이름 (원형 순환) | - | 미선언 | 1회 [Select] (other1) |
| 202 | (상수 없음) | targetnamep8 (패턴) | 공통 | TargetProperty 목록에서 현재 target 기준 8칸 이전 목록의 표시 이름 (원형 순환) | - | 미선언 | 1회 [Select] (other1) |
| 203 | (상수 없음) | targetnamep7 (패턴) | 공통 | TargetProperty 목록에서 현재 target 기준 7칸 이전 목록의 표시 이름 (원형 순환) | - | 미선언 | 1회 [Select] (other1) |
| 204 | (상수 없음) | targetnamep6 (패턴) | 공통 | TargetProperty 목록에서 현재 target 기준 6칸 이전 목록의 표시 이름 (원형 순환) | - | 미선언 | 1회 [Select] (other1) |
| 205 | (상수 없음) | targetnamep5 (패턴) | 공통 | TargetProperty 목록에서 현재 target 기준 5칸 이전 목록의 표시 이름 (원형 순환) | - | 미선언 | 1회 [Select] (other1) |
| 206 | (상수 없음) | targetnamep4 (패턴) | 공통 | TargetProperty 목록에서 현재 target 기준 4칸 이전 목록의 표시 이름 (원형 순환) | - | 미선언 | 1회 [Select] (other1) |
| 207 | (상수 없음) | targetnamep3 (패턴) | 공통 | TargetProperty 목록에서 현재 target 기준 3칸 이전 목록의 표시 이름 (원형 순환) | - | 미선언 | 1회 [Select] (other1) |
| 208 | (상수 없음) | targetnamep2 (패턴) | 공통 | TargetProperty 목록에서 현재 target 기준 2칸 이전 목록의 표시 이름 (원형 순환) | - | 미선언 | 1회 [Select] (other1) |
| 209 | (상수 없음) | targetnamep1 (패턴) | 공통 | TargetProperty 목록에서 현재 target 기준 1칸 이전 목록의 표시 이름 (원형 순환) | - | 미선언 | 1회 [Select] (other1) |
| 210 | (상수 없음) | targetnamen1 (패턴) | 공통 | 현재 target 기준 1칸 다음 목록의 표시 이름 (원형 순환) | - | 미선언 | 1회 [Select] (op1) |
| 211 | (상수 없음) | targetnamen2 (패턴) | 공통 | 현재 target 기준 2칸 다음 목록의 표시 이름 (원형 순환) | - | 미선언 | 1회 [Select] (other1) |
| 212 | (상수 없음) | targetnamen3 (패턴) | 공통 | 현재 target 기준 3칸 다음 목록의 표시 이름 (원형 순환) | - | 미선언 | 1회 [Select] (other1) |
| 213 | (상수 없음) | targetnamen4 (패턴) | 공통 | 현재 target 기준 4칸 다음 목록의 표시 이름 (원형 순환) | - | 미선언 | 1회 [Select] (other1) |
| 214 | (상수 없음) | targetnamen5 (패턴) | 공통 | 현재 target 기준 5칸 다음 목록의 표시 이름 (원형 순환) | - | 미선언 | 1회 [Select] (other1) |
| 215 | (상수 없음) | targetnamen6 (패턴) | 공통 | 현재 target 기준 6칸 다음 목록의 표시 이름 (원형 순환) | - | 미선언 | 1회 [Select] (other1) |
| 216 | (상수 없음) | targetnamen7 (패턴) | 공통 | 현재 target 기준 7칸 다음 목록의 표시 이름 (원형 순환) | - | 미선언 | 1회 [Select] (other1) |
| 217 | (상수 없음) | targetnamen8 (패턴) | 공통 | 현재 target 기준 8칸 다음 목록의 표시 이름 (원형 순환) | - | 미선언 | 1회 [Select] (other1) |
| 218 | (상수 없음) | targetnamen9 (패턴) | 공통 | 현재 target 기준 9칸 다음 목록의 표시 이름 (원형 순환) | - | 미선언 | 1회 [Select] (other1) |
| 219 | (상수 없음) | targetnamen10 (패턴) | 공통 | 현재 target 기준 10칸 다음 목록의 표시 이름 (원형 순환) | - | 미선언 | 1회 [Select] (other1) |
| 240 | (상수 없음) | key11 (패턴) | 키컨피그 | KeyConfiguration.getKeyAssign(10) (id-240 에 10을 더한 인덱스) | - | 구현(키) **불일치: R-BMS 키 인덱스=id-240 (beatoraja는 id-240+10: 확장 키 이름은 11번째 키부터). 10칸 어긋남** |  |
| 241 | (상수 없음) | key12 (패턴) | 키컨피그 | KeyConfiguration.getKeyAssign(11) (id-240 에 10을 더한 인덱스) | - | 구현(키) **불일치: R-BMS 키 인덱스=id-240 (beatoraja는 id-240+10: 확장 키 이름은 11번째 키부터). 10칸 어긋남** |  |
| 242 | (상수 없음) | key13 (패턴) | 키컨피그 | KeyConfiguration.getKeyAssign(12) (id-240 에 10을 더한 인덱스) | - | 구현(키) **불일치: R-BMS 키 인덱스=id-240 (beatoraja는 id-240+10: 확장 키 이름은 11번째 키부터). 10칸 어긋남** |  |
| 243 | (상수 없음) | key14 (패턴) | 키컨피그 | KeyConfiguration.getKeyAssign(13) (id-240 에 10을 더한 인덱스) | - | 구현(키) **불일치: R-BMS 키 인덱스=id-240 (beatoraja는 id-240+10: 확장 키 이름은 11번째 키부터). 10칸 어긋남** |  |
| 244 | (상수 없음) | key15 (패턴) | 키컨피그 | KeyConfiguration.getKeyAssign(14) (id-240 에 10을 더한 인덱스) | - | 구현(키) **불일치: R-BMS 키 인덱스=id-240 (beatoraja는 id-240+10: 확장 키 이름은 11번째 키부터). 10칸 어긋남** |  |
| 245 | (상수 없음) | key16 (패턴) | 키컨피그 | KeyConfiguration.getKeyAssign(15) (id-240 에 10을 더한 인덱스) | - | 구현(키) **불일치: R-BMS 키 인덱스=id-240 (beatoraja는 id-240+10: 확장 키 이름은 11번째 키부터). 10칸 어긋남** |  |
| 246 | (상수 없음) | key17 (패턴) | 키컨피그 | KeyConfiguration.getKeyAssign(16) (id-240 에 10을 더한 인덱스) | - | 구현(키) **불일치: R-BMS 키 인덱스=id-240 (beatoraja는 id-240+10: 확장 키 이름은 11번째 키부터). 10칸 어긋남** |  |
| 247 | (상수 없음) | key18 (패턴) | 키컨피그 | KeyConfiguration.getKeyAssign(17) (id-240 에 10을 더한 인덱스) | - | 구현(키) **불일치: R-BMS 키 인덱스=id-240 (beatoraja는 id-240+10: 확장 키 이름은 11번째 키부터). 10칸 어긋남** |  |
| 248 | (상수 없음) | key19 (패턴) | 키컨피그 | KeyConfiguration.getKeyAssign(18) (id-240 에 10을 더한 인덱스) | - | 구현(키) **불일치: R-BMS 키 인덱스=id-240 (beatoraja는 id-240+10: 확장 키 이름은 11번째 키부터). 10칸 어긋남** |  |
| 249 | (상수 없음) | key20 (패턴) | 키컨피그 | KeyConfiguration.getKeyAssign(19) (id-240 에 10을 더한 인덱스) | - | 구현(키) **불일치: R-BMS 키 인덱스=id-240 (beatoraja는 id-240+10: 확장 키 이름은 11번째 키부터). 10칸 어긋남** |  |
| 250 | (상수 없음) | key21 (패턴) | 키컨피그 | KeyConfiguration.getKeyAssign(20) (id-240 에 10을 더한 인덱스) | - | 구현(키) **불일치: R-BMS 키 인덱스=id-240 (beatoraja는 id-240+10: 확장 키 이름은 11번째 키부터). 10칸 어긋남** |  |
| 251 | (상수 없음) | key22 (패턴) | 키컨피그 | KeyConfiguration.getKeyAssign(21) (id-240 에 10을 더한 인덱스) | - | 구현(키) **불일치: R-BMS 키 인덱스=id-240 (beatoraja는 id-240+10: 확장 키 이름은 11번째 키부터). 10칸 어긋남** |  |
| 252 | (상수 없음) | key23 (패턴) | 키컨피그 | KeyConfiguration.getKeyAssign(22) (id-240 에 10을 더한 인덱스) | - | 구현(키) **불일치: R-BMS 키 인덱스=id-240 (beatoraja는 id-240+10: 확장 키 이름은 11번째 키부터). 10칸 어긋남** |  |
| 253 | (상수 없음) | key24 (패턴) | 키컨피그 | KeyConfiguration.getKeyAssign(23) (id-240 에 10을 더한 인덱스) | - | 구현(키) **불일치: R-BMS 키 인덱스=id-240 (beatoraja는 id-240+10: 확장 키 이름은 11번째 키부터). 10칸 어긋남** |  |
| 254 | (상수 없음) | key25 (패턴) | 키컨피그 | KeyConfiguration.getKeyAssign(24) (id-240 에 10을 더한 인덱스) | - | 구현(키) **불일치: R-BMS 키 인덱스=id-240 (beatoraja는 id-240+10: 확장 키 이름은 11번째 키부터). 10칸 어긋남** |  |
| 255 | (상수 없음) | key26 (패턴) | 키컨피그 | KeyConfiguration.getKeyAssign(25) (id-240 에 10을 더한 인덱스) | - | 구현(키) **불일치: R-BMS 키 인덱스=id-240 (beatoraja는 id-240+10: 확장 키 이름은 11번째 키부터). 10칸 어긋남** |  |
| 256 | (상수 없음) | key27 (패턴) | 키컨피그 | KeyConfiguration.getKeyAssign(26) (id-240 에 10을 더한 인덱스) | - | 구현(키) **불일치: R-BMS 키 인덱스=id-240 (beatoraja는 id-240+10: 확장 키 이름은 11번째 키부터). 10칸 어긋남** |  |
| 257 | (상수 없음) | key28 (패턴) | 키컨피그 | KeyConfiguration.getKeyAssign(27) (id-240 에 10을 더한 인덱스) | - | 구현(키) **불일치: R-BMS 키 인덱스=id-240 (beatoraja는 id-240+10: 확장 키 이름은 11번째 키부터). 10칸 어긋남** |  |
| 258 | (상수 없음) | key29 (패턴) | 키컨피그 | KeyConfiguration.getKeyAssign(28) (id-240 에 10을 더한 인덱스) | - | 구현(키) **불일치: R-BMS 키 인덱스=id-240 (beatoraja는 id-240+10: 확장 키 이름은 11번째 키부터). 10칸 어긋남** |  |
| 259 | (상수 없음) | key30 (패턴) | 키컨피그 | KeyConfiguration.getKeyAssign(29) (id-240 에 10을 더한 인덱스) | - | 구현(키) **불일치: R-BMS 키 인덱스=id-240 (beatoraja는 id-240+10: 확장 키 이름은 11번째 키부터). 10칸 어긋남** |  |
| 260 | (상수 없음) | key31 (패턴) | 키컨피그 | KeyConfiguration.getKeyAssign(30) (id-240 에 10을 더한 인덱스) | - | 구현(키) **불일치: R-BMS 키 인덱스=id-240 (beatoraja는 id-240+10: 확장 키 이름은 11번째 키부터). 10칸 어긋남** |  |
| 261 | (상수 없음) | key32 (패턴) | 키컨피그 | KeyConfiguration.getKeyAssign(31) (id-240 에 10을 더한 인덱스) | - | 구현(키) **불일치: R-BMS 키 인덱스=id-240 (beatoraja는 id-240+10: 확장 키 이름은 11번째 키부터). 10칸 어긋남** |  |
| 262 | (상수 없음) | key33 (패턴) | 키컨피그 | KeyConfiguration.getKeyAssign(32) (id-240 에 10을 더한 인덱스) | - | 구현(키) **불일치: R-BMS 키 인덱스=id-240 (beatoraja는 id-240+10: 확장 키 이름은 11번째 키부터). 10칸 어긋남** |  |
| 263 | (상수 없음) | key34 (패턴) | 키컨피그 | KeyConfiguration.getKeyAssign(33) (id-240 에 10을 더한 인덱스) | - | 구현(키) **불일치: R-BMS 키 인덱스=id-240 (beatoraja는 id-240+10: 확장 키 이름은 11번째 키부터). 10칸 어긋남** |  |
| 264 | (상수 없음) | key35 (패턴) | 키컨피그 | KeyConfiguration.getKeyAssign(34) (id-240 에 10을 더한 인덱스) | - | 구현(키) **불일치: R-BMS 키 인덱스=id-240 (beatoraja는 id-240+10: 확장 키 이름은 11번째 키부터). 10칸 어긋남** |  |
| 265 | (상수 없음) | key36 (패턴) | 키컨피그 | KeyConfiguration.getKeyAssign(35) (id-240 에 10을 더한 인덱스) | - | 구현(키) **불일치: R-BMS 키 인덱스=id-240 (beatoraja는 id-240+10: 확장 키 이름은 11번째 키부터). 10칸 어긋남** |  |
| 266 | (상수 없음) | key37 (패턴) | 키컨피그 | KeyConfiguration.getKeyAssign(36) (id-240 에 10을 더한 인덱스) | - | 구현(키) **불일치: R-BMS 키 인덱스=id-240 (beatoraja는 id-240+10: 확장 키 이름은 11번째 키부터). 10칸 어긋남** |  |
| 267 | (상수 없음) | key38 (패턴) | 키컨피그 | KeyConfiguration.getKeyAssign(37) (id-240 에 10을 더한 인덱스) | - | 구현(키) **불일치: R-BMS 키 인덱스=id-240 (beatoraja는 id-240+10: 확장 키 이름은 11번째 키부터). 10칸 어긋남** |  |
| 268 | (상수 없음) | key39 (패턴) | 키컨피그 | KeyConfiguration.getKeyAssign(38) (id-240 에 10을 더한 인덱스) | - | 구현(키) **불일치: R-BMS 키 인덱스=id-240 (beatoraja는 id-240+10: 확장 키 이름은 11번째 키부터). 10칸 어긋남** |  |
| 269 | (상수 없음) | key40 (패턴) | 키컨피그 | KeyConfiguration.getKeyAssign(39) (id-240 에 10을 더한 인덱스) | - | 구현(키) **불일치: R-BMS 키 인덱스=id-240 (beatoraja는 id-240+10: 확장 키 이름은 11번째 키부터). 10칸 어긋남** |  |
| 270 | (상수 없음) | key41 (패턴) | 키컨피그 | KeyConfiguration.getKeyAssign(40) (id-240 에 10을 더한 인덱스) | - | 구현(키) **불일치: R-BMS 키 인덱스=id-240 (beatoraja는 id-240+10: 확장 키 이름은 11번째 키부터). 10칸 어긋남** |  |
| 271 | (상수 없음) | key42 (패턴) | 키컨피그 | KeyConfiguration.getKeyAssign(41) (id-240 에 10을 더한 인덱스) | - | 구현(키) **불일치: R-BMS 키 인덱스=id-240 (beatoraja는 id-240+10: 확장 키 이름은 11번째 키부터). 10칸 어긋남** |  |
| 272 | (상수 없음) | key43 (패턴) | 키컨피그 | KeyConfiguration.getKeyAssign(42) (id-240 에 10을 더한 인덱스) | - | 구현(키) **불일치: R-BMS 키 인덱스=id-240 (beatoraja는 id-240+10: 확장 키 이름은 11번째 키부터). 10칸 어긋남** |  |
| 273 | (상수 없음) | key44 (패턴) | 키컨피그 | KeyConfiguration.getKeyAssign(43) (id-240 에 10을 더한 인덱스) | - | 구현(키) **불일치: R-BMS 키 인덱스=id-240 (beatoraja는 id-240+10: 확장 키 이름은 11번째 키부터). 10칸 어긋남** |  |
| 274 | (상수 없음) | key45 (패턴) | 키컨피그 | KeyConfiguration.getKeyAssign(44) (id-240 에 10을 더한 인덱스) | - | 구현(키) **불일치: R-BMS 키 인덱스=id-240 (beatoraja는 id-240+10: 확장 키 이름은 11번째 키부터). 10칸 어긋남** |  |
| 275 | (상수 없음) | key46 (패턴) | 키컨피그 | KeyConfiguration.getKeyAssign(45) (id-240 에 10을 더한 인덱스) | - | 구현(키) **불일치: R-BMS 키 인덱스=id-240 (beatoraja는 id-240+10: 확장 키 이름은 11번째 키부터). 10칸 어긋남** |  |
| 276 | (상수 없음) | key47 (패턴) | 키컨피그 | KeyConfiguration.getKeyAssign(46) (id-240 에 10을 더한 인덱스) | - | 구현(키) **불일치: R-BMS 키 인덱스=id-240 (beatoraja는 id-240+10: 확장 키 이름은 11번째 키부터). 10칸 어긋남** |  |
| 277 | (상수 없음) | key48 (패턴) | 키컨피그 | KeyConfiguration.getKeyAssign(47) (id-240 에 10을 더한 인덱스) | - | 구현(키) **불일치: R-BMS 키 인덱스=id-240 (beatoraja는 id-240+10: 확장 키 이름은 11번째 키부터). 10칸 어긋남** |  |
| 278 | (상수 없음) | key49 (패턴) | 키컨피그 | KeyConfiguration.getKeyAssign(48) (id-240 에 10을 더한 인덱스) | - | 구현(키) **불일치: R-BMS 키 인덱스=id-240 (beatoraja는 id-240+10: 확장 키 이름은 11번째 키부터). 10칸 어긋남** |  |
| 279 | (상수 없음) | key50 (패턴) | 키컨피그 | KeyConfiguration.getKeyAssign(49) (id-240 에 10을 더한 인덱스) | - | 구현(키) **불일치: R-BMS 키 인덱스=id-240 (beatoraja는 id-240+10: 확장 키 이름은 11번째 키부터). 10칸 어긋남** |  |
| 280 | (상수 없음) | key51 (패턴) | 키컨피그 | KeyConfiguration.getKeyAssign(50) (id-240 에 10을 더한 인덱스) | - | 구현(키) **불일치: R-BMS 키 인덱스=id-240 (beatoraja는 id-240+10: 확장 키 이름은 11번째 키부터). 10칸 어긋남** |  |
| 281 | (상수 없음) | key52 (패턴) | 키컨피그 | KeyConfiguration.getKeyAssign(51) (id-240 에 10을 더한 인덱스) | - | 구현(키) **불일치: R-BMS 키 인덱스=id-240 (beatoraja는 id-240+10: 확장 키 이름은 11번째 키부터). 10칸 어긋남** |  |
| 282 | (상수 없음) | key53 (패턴) | 키컨피그 | KeyConfiguration.getKeyAssign(52) (id-240 에 10을 더한 인덱스) | - | 구현(키) **불일치: R-BMS 키 인덱스=id-240 (beatoraja는 id-240+10: 확장 키 이름은 11번째 키부터). 10칸 어긋남** |  |
| 283 | (상수 없음) | key54 (패턴) | 키컨피그 | KeyConfiguration.getKeyAssign(53) (id-240 에 10을 더한 인덱스) | - | 구현(키) **불일치: R-BMS 키 인덱스=id-240 (beatoraja는 id-240+10: 확장 키 이름은 11번째 키부터). 10칸 어긋남** |  |
| 1000 | STRING_DIRECTORY | directory | 선곡 | BarManager.getDirectoryString() (선곡 외 빈 문자열) | - | 구현(선) | 2회 [Select] (ref2) |
| 1001 | STRING_TABLE_NAME | tablename | 공통 | PlayerResource.getTablename() | - | 등록만(SongSelect) |  |
| 1002 | STRING_TABLE_LEVEL | tablelevel | 공통 | PlayerResource.getTablelevel() | - | 등록만(SongSelect) |  |
| 1003 | STRING_TABLE_FULL | tablefull | 공통 | PlayerResource.getTableFullname() | - | 등록만(SongSelect) | 6회 [Decide,Play,Result] (ref4,script2) |
| 1010 | STRING_VERSION | version | 공통 | MainController.getVersion() | - | 등록만(PlayerConfig) | 2회 [Root] (script2) |
| 1020 | STRING_IR_NAME | irname | 공통 | PlayerConfig.irconfig[0].getIrname() (없으면 빈 문자열) | - | 등록만(InternetRanking) | 6회 [Result,Root,Select] (ref4,script2) |
| 1021 | STRING_IR_USER_NAME | irUserName | 공통 | MainController.getIRStatus()[0].player.name (없으면 빈 문자열; enum 이름 camelCase irUserName) | - | 등록만(InternetRanking) |  |
| 1030 | STRING_SONG_HASH_MD5 | songhashmd5 | 공통 | songdata.getMd5() | - | 등록만(SongSelect) | 2회 [Result] (script2) |
| 1031 | STRING_SONG_HASH_SHA256 | songhashsha256 | 공통 | songdata.getSha256() | - | 등록만(SongSelect) | 2회 [Result] (script2) |
| 1040 | STRING_PRACTICE_ITEM1 | practice_item1 (패턴) | 플레이 | PracticeConfiguration.getVisibleItemText(0); 플레이 외 빈 문자열 | - | 없음 |  |
| 1041 | (상수 없음) | practice_item2 (패턴) | 플레이 | PracticeConfiguration.getVisibleItemText(1); 플레이 외 빈 문자열 | - | 미선언 |  |
| 1042 | (상수 없음) | practice_item3 (패턴) | 플레이 | PracticeConfiguration.getVisibleItemText(2); 플레이 외 빈 문자열 | - | 미선언 |  |
| 1043 | (상수 없음) | practice_item4 (패턴) | 플레이 | PracticeConfiguration.getVisibleItemText(3); 플레이 외 빈 문자열 | - | 미선언 |  |
| 1044 | (상수 없음) | practice_item5 (패턴) | 플레이 | PracticeConfiguration.getVisibleItemText(4); 플레이 외 빈 문자열 | - | 미선언 |  |
| 1045 | (상수 없음) | practice_item6 (패턴) | 플레이 | PracticeConfiguration.getVisibleItemText(5); 플레이 외 빈 문자열 | - | 미선언 |  |
| 1046 | (상수 없음) | practice_item7 (패턴) | 플레이 | PracticeConfiguration.getVisibleItemText(6); 플레이 외 빈 문자열 | - | 미선언 |  |
| 1047 | (상수 없음) | practice_item8 (패턴) | 플레이 | PracticeConfiguration.getVisibleItemText(7); 플레이 외 빈 문자열 | - | 미선언 |  |
| 1048 | (상수 없음) | practice_item9 (패턴) | 플레이 | PracticeConfiguration.getVisibleItemText(8); 플레이 외 빈 문자열 | - | 미선언 |  |
| 1049 | (상수 없음) | practice_item10 (패턴) | 플레이 | PracticeConfiguration.getVisibleItemText(9); 플레이 외 빈 문자열 | - | 미선언 |  |
| 1050 | (상수 없음) | practice_item11 (패턴) | 플레이 | PracticeConfiguration.getVisibleItemText(10); 플레이 외 빈 문자열 | - | 미선언 |  |
| 1051 | (상수 없음) | practice_item12 (패턴) | 플레이 | PracticeConfiguration.getVisibleItemText(11); 플레이 외 빈 문자열 | - | 미선언 |  |
| 1052 | (상수 없음) | practice_item13 (패턴) | 플레이 | PracticeConfiguration.getVisibleItemText(12); 플레이 외 빈 문자열 | - | 미선언 |  |
| 1053 | (상수 없음) | practice_item14 (패턴) | 플레이 | PracticeConfiguration.getVisibleItemText(13); 플레이 외 빈 문자열 | - | 미선언 |  |
| 1054 | (상수 없음) | practice_item15 (패턴) | 플레이 | PracticeConfiguration.getVisibleItemText(14); 플레이 외 빈 문자열 | - | 미선언 |  |
| 1055 | STRING_PRACTICE_ITEM16 | practice_item16 (패턴) | 플레이 | PracticeConfiguration.getVisibleItemText(15); 플레이 외 빈 문자열 | - | 없음 |  |
| 1060 | STRING_PRACTICE_ITEM_LABEL1 | practice_item1_label / practice_item_label1 (패턴) | 플레이 | getVisibleItemLabel(0) | - | 없음 |  |
| 1061 | (상수 없음) | practice_item2_label / practice_item_label2 (패턴) | 플레이 | getVisibleItemLabel(1) | - | 미선언 |  |
| 1062 | (상수 없음) | practice_item3_label / practice_item_label3 (패턴) | 플레이 | getVisibleItemLabel(2) | - | 미선언 |  |
| 1063 | (상수 없음) | practice_item4_label / practice_item_label4 (패턴) | 플레이 | getVisibleItemLabel(3) | - | 미선언 |  |
| 1064 | (상수 없음) | practice_item5_label / practice_item_label5 (패턴) | 플레이 | getVisibleItemLabel(4) | - | 미선언 |  |
| 1065 | (상수 없음) | practice_item6_label / practice_item_label6 (패턴) | 플레이 | getVisibleItemLabel(5) | - | 미선언 |  |
| 1066 | (상수 없음) | practice_item7_label / practice_item_label7 (패턴) | 플레이 | getVisibleItemLabel(6) | - | 미선언 |  |
| 1067 | (상수 없음) | practice_item8_label / practice_item_label8 (패턴) | 플레이 | getVisibleItemLabel(7) | - | 미선언 |  |
| 1068 | (상수 없음) | practice_item9_label / practice_item_label9 (패턴) | 플레이 | getVisibleItemLabel(8) | - | 미선언 |  |
| 1069 | (상수 없음) | practice_item10_label / practice_item_label10 (패턴) | 플레이 | getVisibleItemLabel(9) | - | 미선언 |  |
| 1070 | (상수 없음) | practice_item11_label / practice_item_label11 (패턴) | 플레이 | getVisibleItemLabel(10) | - | 미선언 |  |
| 1071 | (상수 없음) | practice_item12_label / practice_item_label12 (패턴) | 플레이 | getVisibleItemLabel(11) | - | 미선언 |  |
| 1072 | (상수 없음) | practice_item13_label / practice_item_label13 (패턴) | 플레이 | getVisibleItemLabel(12) | - | 미선언 |  |
| 1073 | (상수 없음) | practice_item14_label / practice_item_label14 (패턴) | 플레이 | getVisibleItemLabel(13) | - | 미선언 |  |
| 1074 | (상수 없음) | practice_item15_label / practice_item_label15 (패턴) | 플레이 | getVisibleItemLabel(14) | - | 미선언 |  |
| 1075 | STRING_PRACTICE_ITEM_LABEL16 | practice_item16_label / practice_item_label16 (패턴) | 플레이 | getVisibleItemLabel(15) | - | 없음 |  |
| 1080 | STRING_PRACTICE_ITEM_VALUE1 | practice_item1_value / practice_item_value1 (패턴) | 플레이 | getVisibleItemValue(0) | - | 없음 |  |
| 1081 | (상수 없음) | practice_item2_value / practice_item_value2 (패턴) | 플레이 | getVisibleItemValue(1) | - | 미선언 |  |
| 1082 | (상수 없음) | practice_item3_value / practice_item_value3 (패턴) | 플레이 | getVisibleItemValue(2) | - | 미선언 |  |
| 1083 | (상수 없음) | practice_item4_value / practice_item_value4 (패턴) | 플레이 | getVisibleItemValue(3) | - | 미선언 |  |
| 1084 | (상수 없음) | practice_item5_value / practice_item_value5 (패턴) | 플레이 | getVisibleItemValue(4) | - | 미선언 |  |
| 1085 | (상수 없음) | practice_item6_value / practice_item_value6 (패턴) | 플레이 | getVisibleItemValue(5) | - | 미선언 |  |
| 1086 | (상수 없음) | practice_item7_value / practice_item_value7 (패턴) | 플레이 | getVisibleItemValue(6) | - | 미선언 |  |
| 1087 | (상수 없음) | practice_item8_value / practice_item_value8 (패턴) | 플레이 | getVisibleItemValue(7) | - | 미선언 |  |
| 1088 | (상수 없음) | practice_item9_value / practice_item_value9 (패턴) | 플레이 | getVisibleItemValue(8) | - | 미선언 |  |
| 1089 | (상수 없음) | practice_item10_value / practice_item_value10 (패턴) | 플레이 | getVisibleItemValue(9) | - | 미선언 |  |
| 1090 | (상수 없음) | practice_item11_value / practice_item_value11 (패턴) | 플레이 | getVisibleItemValue(10) | - | 미선언 |  |
| 1091 | (상수 없음) | practice_item12_value / practice_item_value12 (패턴) | 플레이 | getVisibleItemValue(11) | - | 미선언 |  |
| 1092 | (상수 없음) | practice_item13_value / practice_item_value13 (패턴) | 플레이 | getVisibleItemValue(12) | - | 미선언 |  |
| 1093 | (상수 없음) | practice_item14_value / practice_item_value14 (패턴) | 플레이 | getVisibleItemValue(13) | - | 미선언 |  |
| 1094 | (상수 없음) | practice_item15_value / practice_item_value15 (패턴) | 플레이 | getVisibleItemValue(14) | - | 미선언 |  |
| 1095 | STRING_PRACTICE_ITEM_VALUE16 | practice_item16_value / practice_item_value16 (패턴) | 플레이 | getVisibleItemValue(15) | - | 없음 |  |

## 7. 타이머 (TIMER_*)

조회 함수: `TimerPropertyFactory.getTimerProperty(int)` (TimerPropertyFactory.java:6). `id<0`이면 null, 그 외는 항상 `TimerManager`를 읽는 래퍼를 만든다(구현 유무 개념 없음). 스킨 객체는 `timer>0`일 때만 타이머 래퍼를 만든다(SkinObject.java:127-150, Skin.java:140): timer id 0은 "타이머 없음 = 화면 시작 기준 시간"이다. `TimerManager` (TimerManager.java:12-110): 내부 배열 `long[3000]`(id 0..2999), 꺼짐 = `Long.MIN_VALUE`, `getNowTime(id)` = 켜졌으면 (nowmicrotime - timer[id])/1000 ms, 꺼졌으면 0. 범위 밖 id(<0 또는 >=3000)는 `Skin.getMicroCustomTimer`로 위임되어 커스텀 타이머(10000..19999)만 값을 갖고 그 외는 꺼짐. 스킨이 쓸 수 있는 타이머는 커스텀 구간뿐이다(`SkinPropertyMapper.isTimerWritableBySkin`, :163; Lua `set_timer`). 화면이 바뀌면 `setMainState`가 전부 `Long.MIN_VALUE`로 초기화한다(TimerManager.java:70-76). Lua `main_state.timer(id)`는 마이크로초 단위 값 또는 `Long.MIN_VALUE`(`timer_off_value`)를 돌려주고 `main_state.time()`은 마이크로초(`getNowMicroTime`)이다.

근거 줄: BMSPlayer.java:471(STARTINPUT), 514/591(READY), 609-610(PLAY,RHYTHM), 624(PLAY 보정), 636(GAUGE_MAX), 656(FAILED), 681-689(MUSIC_END,ENDOFNOTE), 941-963(stopPlay FADEOUT/FAILED), 1028-1037(FULLCOMBO,SCORE_*); JudgeManager.java:308-333(HCN), 632(HOLD), 674(BOMB), 680-687(JUDGE,COMBO); KeyInputProccessor.java:75-88,115-122(KEYON/OFF); MusicSelector.java:194-197(STARTINPUT,SONGBAR_CHANGE),248-250(IR),556-561(PANEL),612(SONGBAR_CHANGE); MusicDecide.java:40-61; MusicResult.java:112-129(IR),162-169(RESULTGRAPH,UPDATESCORE,STARTINPUT),251,267,288-291(FADEOUT,UPDATESCORE). 이 줄들에서 확인되지 않은 타이머는 beatoraja에서 켜지지 않는다.

R-BMS 열: 이 표의 "R-BMS"는 `SkinStateSource::timer`가 아니라 앱이 `TimerState`에 실제로 쓰는지(skin_screen.rs / render/skin_render/screen.rs의 `PlayTimers`/`SelectTimers`/`ResultTimers`, app_options.rs:209)를 기준으로 한다. 모든 `*ViewState::timer()`는 `None`이며(state.rs:367,497,639,719,775) 실제 타이머는 `TimerState`가 별도 인자로 `resolve`에 전달된다(skin_screen.rs:691,718).

| id | 상수 | beatoraja가 켜는 곳/조건 | R-BMS가 켜는가 | ModernChic 사용 |
|---|---|---|---|---|
| 1 | TIMER_STARTINPUT | 선곡/결정/플레이/결과/코스결과: 상태 시작 후 경과가 skin.getInput()(ms)을 넘으면 switchTimer(true) (플레이는 micronow > input*1000). 켜진 뒤 입력 허용 | 안 켬(항상 off) |  |
| 2 | TIMER_FADEOUT | 결정/플레이/결과: 종료 트리거(키 입력, 경과>scene, 곡 종료, stopPlay)에서 setTimerOn. 경과가 skin.getFadeout()을 넘으면 다음 상태로 전환. 플레이 연습 재시작 시 off | 안 켬(항상 off) | 8회 [Result,decide,play10_hw,play5_hw,play7_hw] (timer8) |
| 3 | TIMER_FAILED | 플레이: 게이지 0(GAUGEAUTOSHIFT_NONE)이거나 stopPlay로 중단 시 setTimerOn. 연습 재시작 시 off. 경과>skin.getClose()면 결과로 전환 | 켬(플) | 13회 [Play] (timer12,script1) |
| 10 | TIMER_SONGBAR_MOVE | 선언만: beatoraja 소스 어디서도 켜지 않음(항상 off) | 켬(선) **[beatoraja는 켜지 않음]** |  |
| 11 | TIMER_SONGBAR_CHANGE | 선곡: selectedBarMoved()에서 setTimerOn(막대 이동마다 재시작), 입력 처리 후 switchTimer(true), 렌더에서 꺼져 있으면 켬 | 켬(선) | 28회 [Root,Select] (timer27,script1) |
| 12 | TIMER_SONGBAR_MOVE_UP | 선언만: 켜지 않음 | 켬(선) **[beatoraja는 켜지 않음]** |  |
| 13 | TIMER_SONGBAR_MOVE_DOWN | 선언만: 켜지 않음 | 켬(선) **[beatoraja는 켜지 않음]** |  |
| 14 | TIMER_SONGBAR_STOP | 선언만: 켜지 않음 | 안 켬(항상 off) |  |
| 15 | TIMER_README_BEGIN | 선언만: 켜지 않음 | 안 켬(항상 off) |  |
| 16 | TIMER_README_END | 선언만: 켜지 않음 | 안 켬(항상 off) |  |
| 21 | TIMER_PANEL1_ON | 선곡: setPanelState(n)로 패널 n 진입 시 setTimerOn(PANEL1_ON+n-1)+setTimerOff(PANEL1_OFF+n-1), 나갈 때 반대 | 켬(선) | 30회 [Select] (timer30) |
| 22 | TIMER_PANEL2_ON | 선곡: setPanelState(n)로 패널 n 진입 시 setTimerOn(PANEL1_ON+n-1)+setTimerOff(PANEL1_OFF+n-1), 나갈 때 반대 | 안 켬(항상 off) | 28회 [Select] (timer28) |
| 23 | TIMER_PANEL3_ON | 선곡: setPanelState(n)로 패널 n 진입 시 setTimerOn(PANEL1_ON+n-1)+setTimerOff(PANEL1_OFF+n-1), 나갈 때 반대 | 안 켬(항상 off) | 30회 [Select] (timer30) |
| 24 | TIMER_PANEL4_ON | 선언만: panelstate는 1..3만 사용하므로 켜지지 않음 | 안 켬(항상 off) |  |
| 25 | TIMER_PANEL5_ON | 선언만: panelstate는 1..3만 사용하므로 켜지지 않음 | 안 켬(항상 off) |  |
| 26 | TIMER_PANEL6_ON | 선언만: panelstate는 1..3만 사용하므로 켜지지 않음 | 안 켬(항상 off) |  |
| 31 | TIMER_PANEL1_OFF | 선곡: 위 PANEL ON과 쌍(진입 시 off, 이탈 시 on) | 켬(선) | 2회 [Select] (timer2) |
| 32 | TIMER_PANEL2_OFF | 선곡: 위 PANEL ON과 쌍(진입 시 off, 이탈 시 on) | 안 켬(항상 off) | 2회 [Select] (timer2) |
| 33 | TIMER_PANEL3_OFF | 선곡: 위 PANEL ON과 쌍(진입 시 off, 이탈 시 on) | 안 켬(항상 off) | 2회 [Select] (timer2) |
| 34 | TIMER_PANEL4_OFF | 선언만: 켜지지 않음 | 안 켬(항상 off) |  |
| 35 | TIMER_PANEL5_OFF | 선언만: 켜지지 않음 | 안 켬(항상 off) |  |
| 36 | TIMER_PANEL6_OFF | 선언만: 켜지지 않음 | 안 켬(항상 off) |  |
| 40 | TIMER_READY | 플레이: STATE_PRELOAD->STATE_READY 전환 시 setTimerOn (연습 시작 시 timer.update 후 재설정). 이후 끄는 코드가 없어 플레이 내내 켜진 채 유지 | 켬(플) **[READY: R-BMS는 플레이 시작 시 끔]** | 11회 [Play] (timer11) |
| 41 | TIMER_PLAY | 플레이: READY 경과>skin.getPlaystart()에서 setMicroTimer(PLAY, now - starttimeoffset*1000), STATE_PLAY 프레임마다 deltatime*(100-playspeed)/100 보정. 연습 재시작 시 off | 켬(플) | 50회 [Play,Root] (timer48,script2) |
| 42 | TIMER_GAUGE_INCLEASE_1P | 선언만: 켜지 않음 | 켬(플) **[beatoraja는 켜지 않음]** | 2회 [Play] (timer2) |
| 43 | TIMER_GAUGE_INCLEASE_2P | 선언만: 켜지 않음 | 안 켬(항상 off) |  |
| 44 | TIMER_GAUGE_MAX_1P | 플레이: STATE_PLAY 프레임마다 switchTimer(players 루프의 gauge.isMax()) (2P 게이지가 있으면 마지막 플레이어 값이 이김) | 켬(플) | 5회 [Play] (timer5) |
| 45 | TIMER_GAUGE_MAX_2P | 선언만: 켜지 않음 | 안 켬(항상 off) |  |
| 46 | TIMER_JUDGE_1P | 플레이: JudgeManager.updateMicro가 판정마다 setTimerOn(JUDGE_TIMER[judgeindex]), judgeindex=lane/(lanes/judgenow.length) | 켬(플) | 129회 [Play] (timer128,op1) |
| 47 | TIMER_JUDGE_2P | 플레이: 위와 같음 (인덱스 1) | 켬(플) | 63회 [Play] (timer62,other1) |
| 48 | TIMER_FULLCOMBO_1P | 플레이: BMSPlayer.update()마다 switchTimer(passedNotes==songNotes && passedNotes==combo) | 켬(플) | 9회 [Play,Root] (timer8,script1) |
| 49 | TIMER_FULLCOMBO_2P | 선언만: 켜지 않음 | 안 켬(항상 off) |  |
| 50 | TIMER_BOMB_1P_SCRATCH | 플레이: JudgeManager가 판정 judge<=PlaySkin.judgetimer(기본 1: PG,GR)일 때 setTimerOn(bombTimerId). 끄는 코드는 없음(애니메이션 경과로 소멸) | 켬(플) | 9회 [Play] (other6,op3) |
| 51 | TIMER_BOMB_1P_KEY1 | 플레이: JudgeManager가 판정 judge<=PlaySkin.judgetimer(기본 1: PG,GR)일 때 setTimerOn(bombTimerId). 끄는 코드는 없음(애니메이션 경과로 소멸) | 켬(플) | 9회 [Play] (op6,other3) |
| 52 | TIMER_BOMB_1P_KEY2 | 플레이: JudgeManager가 판정 judge<=PlaySkin.judgetimer(기본 1: PG,GR)일 때 setTimerOn(bombTimerId). 끄는 코드는 없음(애니메이션 경과로 소멸) | 켬(플) | 9회 [Play] (other9) |
| 53 | TIMER_BOMB_1P_KEY3 | 플레이: JudgeManager가 판정 judge<=PlaySkin.judgetimer(기본 1: PG,GR)일 때 setTimerOn(bombTimerId). 끄는 코드는 없음(애니메이션 경과로 소멸) | 켬(플) | 9회 [Play] (other9) |
| 54 | TIMER_BOMB_1P_KEY4 | 플레이: JudgeManager가 판정 judge<=PlaySkin.judgetimer(기본 1: PG,GR)일 때 setTimerOn(bombTimerId). 끄는 코드는 없음(애니메이션 경과로 소멸) | 켬(플) | 9회 [Play] (other9) |
| 55 | TIMER_BOMB_1P_KEY5 | 플레이: JudgeManager가 판정 judge<=PlaySkin.judgetimer(기본 1: PG,GR)일 때 setTimerOn(bombTimerId). 끄는 코드는 없음(애니메이션 경과로 소멸) | 켬(플) | 9회 [Play] (other9) |
| 56 | TIMER_BOMB_1P_KEY6 | 플레이: JudgeManager가 판정 judge<=PlaySkin.judgetimer(기본 1: PG,GR)일 때 setTimerOn(bombTimerId). 끄는 코드는 없음(애니메이션 경과로 소멸) | 켬(플) | 5회 [Play] (other5) |
| 57 | TIMER_BOMB_1P_KEY7 | 플레이: JudgeManager가 판정 judge<=PlaySkin.judgetimer(기본 1: PG,GR)일 때 setTimerOn(bombTimerId). 끄는 코드는 없음(애니메이션 경과로 소멸) | 켬(플) | 5회 [Play] (other5) |
| 58 | TIMER_BOMB_1P_KEY8 | 플레이: JudgeManager가 판정 judge<=PlaySkin.judgetimer(기본 1: PG,GR)일 때 setTimerOn(bombTimerId). 끄는 코드는 없음(애니메이션 경과로 소멸) | 켬(플) |  |
| 59 | TIMER_BOMB_1P_KEY9 | 플레이: JudgeManager가 판정 judge<=PlaySkin.judgetimer(기본 1: PG,GR)일 때 setTimerOn(bombTimerId). 끄는 코드는 없음(애니메이션 경과로 소멸) | 켬(플) |  |
| 60 | TIMER_BOMB_2P_SCRATCH | 플레이: JudgeManager가 판정 judge<=PlaySkin.judgetimer(기본 1: PG,GR)일 때 setTimerOn(bombTimerId). 끄는 코드는 없음(애니메이션 경과로 소멸) | 켬(플) | 4회 [Play] (other4) |
| 61 | TIMER_BOMB_2P_KEY1 | 플레이: JudgeManager가 판정 judge<=PlaySkin.judgetimer(기본 1: PG,GR)일 때 setTimerOn(bombTimerId). 끄는 코드는 없음(애니메이션 경과로 소멸) | 켬(플) | 4회 [Play] (other2,op2) |
| 62 | TIMER_BOMB_2P_KEY2 | 플레이: JudgeManager가 판정 judge<=PlaySkin.judgetimer(기본 1: PG,GR)일 때 setTimerOn(bombTimerId). 끄는 코드는 없음(애니메이션 경과로 소멸) | 켬(플) | 4회 [Play] (other4) |
| 63 | TIMER_BOMB_2P_KEY3 | 플레이: JudgeManager가 판정 judge<=PlaySkin.judgetimer(기본 1: PG,GR)일 때 setTimerOn(bombTimerId). 끄는 코드는 없음(애니메이션 경과로 소멸) | 켬(플) | 4회 [Play] (other4) |
| 64 | TIMER_BOMB_2P_KEY4 | 플레이: JudgeManager가 판정 judge<=PlaySkin.judgetimer(기본 1: PG,GR)일 때 setTimerOn(bombTimerId). 끄는 코드는 없음(애니메이션 경과로 소멸) | 켬(플) | 4회 [Play] (other4) |
| 65 | TIMER_BOMB_2P_KEY5 | 플레이: JudgeManager가 판정 judge<=PlaySkin.judgetimer(기본 1: PG,GR)일 때 setTimerOn(bombTimerId). 끄는 코드는 없음(애니메이션 경과로 소멸) | 켬(플) | 4회 [Play] (other4) |
| 66 | TIMER_BOMB_2P_KEY6 | 플레이: JudgeManager가 판정 judge<=PlaySkin.judgetimer(기본 1: PG,GR)일 때 setTimerOn(bombTimerId). 끄는 코드는 없음(애니메이션 경과로 소멸) | 켬(플) | 2회 [Play] (other2) |
| 67 | TIMER_BOMB_2P_KEY7 | 플레이: JudgeManager가 판정 judge<=PlaySkin.judgetimer(기본 1: PG,GR)일 때 setTimerOn(bombTimerId). 끄는 코드는 없음(애니메이션 경과로 소멸) | 켬(플) | 2회 [Play] (other2) |
| 68 | TIMER_BOMB_2P_KEY8 | 플레이: JudgeManager가 판정 judge<=PlaySkin.judgetimer(기본 1: PG,GR)일 때 setTimerOn(bombTimerId). 끄는 코드는 없음(애니메이션 경과로 소멸) | 켬(플) |  |
| 69 | TIMER_BOMB_2P_KEY9 | 플레이: JudgeManager가 판정 judge<=PlaySkin.judgetimer(기본 1: PG,GR)일 때 setTimerOn(bombTimerId). 끄는 코드는 없음(애니메이션 경과로 소멸) | 켬(플) |  |
| 70 | TIMER_HOLD_1P_SCRATCH | 플레이: JudgeManager가 매 프레임 switchTimer(processing!=null // (passing!=null && inclease)) = 롱노트 누르는 동안 on | 켬(플) | 6회 [Play] (other6) |
| 71 | TIMER_HOLD_1P_KEY1 | 플레이: JudgeManager가 매 프레임 switchTimer(processing!=null // (passing!=null && inclease)) = 롱노트 누르는 동안 on | 켬(플) | 6회 [Play] (op6) |
| 80 | TIMER_HOLD_2P_SCRATCH | 플레이: JudgeManager가 매 프레임 switchTimer(processing!=null // (passing!=null && inclease)) = 롱노트 누르는 동안 on | 켬(플) | 4회 [Play] (other4) |
| 81 | TIMER_HOLD_2P_KEY1 | 플레이: JudgeManager가 매 프레임 switchTimer(processing!=null // (passing!=null && inclease)) = 롱노트 누르는 동안 on | 켬(플) | 4회 [Play] (other2,op2) |
| 100 | TIMER_KEYON_1P_SCRATCH | 플레이: KeyInputProccessor가 키 누름 에지에서 setTimerOn(KEYON)+setTimerOff(KEYOFF) (스크래치 방향 전환 시도 재시작) | 켬(플) | 8회 [Play] (other4,timer2,op2) |
| 101 | TIMER_KEYON_1P_KEY1 | 플레이: KeyInputProccessor가 키 누름 에지에서 setTimerOn(KEYON)+setTimerOff(KEYOFF) (스크래치 방향 전환 시도 재시작) | 켬(플) | 10회 [Play] (op8,other2) |
| 102 | TIMER_KEYON_1P_KEY2 | 플레이: KeyInputProccessor가 키 누름 에지에서 setTimerOn(KEYON)+setTimerOff(KEYOFF) (스크래치 방향 전환 시도 재시작) | 켬(플) | 10회 [Play] (other10) |
| 103 | TIMER_KEYON_1P_KEY3 | 플레이: KeyInputProccessor가 키 누름 에지에서 setTimerOn(KEYON)+setTimerOff(KEYOFF) (스크래치 방향 전환 시도 재시작) | 켬(플) | 10회 [Play] (other10) |
| 104 | TIMER_KEYON_1P_KEY4 | 플레이: KeyInputProccessor가 키 누름 에지에서 setTimerOn(KEYON)+setTimerOff(KEYOFF) (스크래치 방향 전환 시도 재시작) | 켬(플) | 10회 [Play] (other10) |
| 105 | TIMER_KEYON_1P_KEY5 | 플레이: KeyInputProccessor가 키 누름 에지에서 setTimerOn(KEYON)+setTimerOff(KEYOFF) (스크래치 방향 전환 시도 재시작) | 켬(플) | 10회 [Play] (other10) |
| 106 | TIMER_KEYON_1P_KEY6 | 플레이: KeyInputProccessor가 키 누름 에지에서 setTimerOn(KEYON)+setTimerOff(KEYOFF) (스크래치 방향 전환 시도 재시작) | 켬(플) | 5회 [Play] (other5) |
| 107 | TIMER_KEYON_1P_KEY7 | 플레이: KeyInputProccessor가 키 누름 에지에서 setTimerOn(KEYON)+setTimerOff(KEYOFF) (스크래치 방향 전환 시도 재시작) | 켬(플) | 5회 [Play] (other5) |
| 108 | TIMER_KEYON_1P_KEY8 | 플레이: KeyInputProccessor가 키 누름 에지에서 setTimerOn(KEYON)+setTimerOff(KEYOFF) (스크래치 방향 전환 시도 재시작) | 켬(플) |  |
| 109 | TIMER_KEYON_1P_KEY9 | 플레이: KeyInputProccessor가 키 누름 에지에서 setTimerOn(KEYON)+setTimerOff(KEYOFF) (스크래치 방향 전환 시도 재시작) | 켬(플) |  |
| 110 | TIMER_KEYON_2P_SCRATCH | 플레이: KeyInputProccessor가 키 누름 에지에서 setTimerOn(KEYON)+setTimerOff(KEYOFF) (스크래치 방향 전환 시도 재시작) | 켬(플) | 2회 [Play] (other2) |
| 111 | TIMER_KEYON_2P_KEY1 | 플레이: KeyInputProccessor가 키 누름 에지에서 setTimerOn(KEYON)+setTimerOff(KEYOFF) (스크래치 방향 전환 시도 재시작) | 켬(플) | 4회 [Play] (op4) |
| 112 | TIMER_KEYON_2P_KEY2 | 플레이: KeyInputProccessor가 키 누름 에지에서 setTimerOn(KEYON)+setTimerOff(KEYOFF) (스크래치 방향 전환 시도 재시작) | 켬(플) | 4회 [Play] (other4) |
| 113 | TIMER_KEYON_2P_KEY3 | 플레이: KeyInputProccessor가 키 누름 에지에서 setTimerOn(KEYON)+setTimerOff(KEYOFF) (스크래치 방향 전환 시도 재시작) | 켬(플) | 4회 [Play] (other4) |
| 114 | TIMER_KEYON_2P_KEY4 | 플레이: KeyInputProccessor가 키 누름 에지에서 setTimerOn(KEYON)+setTimerOff(KEYOFF) (스크래치 방향 전환 시도 재시작) | 켬(플) | 4회 [Play] (other4) |
| 115 | TIMER_KEYON_2P_KEY5 | 플레이: KeyInputProccessor가 키 누름 에지에서 setTimerOn(KEYON)+setTimerOff(KEYOFF) (스크래치 방향 전환 시도 재시작) | 켬(플) | 4회 [Play] (other4) |
| 116 | TIMER_KEYON_2P_KEY6 | 플레이: KeyInputProccessor가 키 누름 에지에서 setTimerOn(KEYON)+setTimerOff(KEYOFF) (스크래치 방향 전환 시도 재시작) | 켬(플) | 2회 [Play] (other2) |
| 117 | TIMER_KEYON_2P_KEY7 | 플레이: KeyInputProccessor가 키 누름 에지에서 setTimerOn(KEYON)+setTimerOff(KEYOFF) (스크래치 방향 전환 시도 재시작) | 켬(플) | 2회 [Play] (other2) |
| 118 | TIMER_KEYON_2P_KEY8 | 플레이: KeyInputProccessor가 키 누름 에지에서 setTimerOn(KEYON)+setTimerOff(KEYOFF) (스크래치 방향 전환 시도 재시작) | 켬(플) |  |
| 119 | TIMER_KEYON_2P_KEY9 | 플레이: KeyInputProccessor가 키 누름 에지에서 setTimerOn(KEYON)+setTimerOff(KEYOFF) (스크래치 방향 전환 시도 재시작) | 켬(플) |  |
| 120 | TIMER_KEYOFF_1P_SCRATCH | 플레이: KeyInputProccessor가 키 뗌 에지에서 setTimerOn(KEYOFF)+setTimerOff(KEYON) | 켬(플) | 4회 [Play] (other4) |
| 121 | TIMER_KEYOFF_1P_KEY1 | 플레이: KeyInputProccessor가 키 뗌 에지에서 setTimerOn(KEYOFF)+setTimerOff(KEYON) | 켬(플) | 4회 [Play] (op4) |
| 122 | TIMER_KEYOFF_1P_KEY2 | 플레이: KeyInputProccessor가 키 뗌 에지에서 setTimerOn(KEYOFF)+setTimerOff(KEYON) | 켬(플) | 4회 [Play] (other4) |
| 123 | TIMER_KEYOFF_1P_KEY3 | 플레이: KeyInputProccessor가 키 뗌 에지에서 setTimerOn(KEYOFF)+setTimerOff(KEYON) | 켬(플) | 4회 [Play] (other4) |
| 124 | TIMER_KEYOFF_1P_KEY4 | 플레이: KeyInputProccessor가 키 뗌 에지에서 setTimerOn(KEYOFF)+setTimerOff(KEYON) | 켬(플) | 4회 [Play] (other4) |
| 125 | TIMER_KEYOFF_1P_KEY5 | 플레이: KeyInputProccessor가 키 뗌 에지에서 setTimerOn(KEYOFF)+setTimerOff(KEYON) | 켬(플) | 4회 [Play] (other4) |
| 126 | TIMER_KEYOFF_1P_KEY6 | 플레이: KeyInputProccessor가 키 뗌 에지에서 setTimerOn(KEYOFF)+setTimerOff(KEYON) | 켬(플) | 2회 [Play] (other2) |
| 127 | TIMER_KEYOFF_1P_KEY7 | 플레이: KeyInputProccessor가 키 뗌 에지에서 setTimerOn(KEYOFF)+setTimerOff(KEYON) | 켬(플) | 2회 [Play] (other2) |
| 128 | TIMER_KEYOFF_1P_KEY8 | 플레이: KeyInputProccessor가 키 뗌 에지에서 setTimerOn(KEYOFF)+setTimerOff(KEYON) | 켬(플) |  |
| 129 | TIMER_KEYOFF_1P_KEY9 | 플레이: KeyInputProccessor가 키 뗌 에지에서 setTimerOn(KEYOFF)+setTimerOff(KEYON) | 켬(플) |  |
| 130 | TIMER_KEYOFF_2P_SCRATCH | 플레이: KeyInputProccessor가 키 뗌 에지에서 setTimerOn(KEYOFF)+setTimerOff(KEYON) | 켬(플) | 2회 [Play] (other2) |
| 131 | TIMER_KEYOFF_2P_KEY1 | 플레이: KeyInputProccessor가 키 뗌 에지에서 setTimerOn(KEYOFF)+setTimerOff(KEYON) | 켬(플) | 2회 [Play] (op2) |
| 132 | TIMER_KEYOFF_2P_KEY2 | 플레이: KeyInputProccessor가 키 뗌 에지에서 setTimerOn(KEYOFF)+setTimerOff(KEYON) | 켬(플) | 2회 [Play] (other2) |
| 133 | TIMER_KEYOFF_2P_KEY3 | 플레이: KeyInputProccessor가 키 뗌 에지에서 setTimerOn(KEYOFF)+setTimerOff(KEYON) | 켬(플) | 2회 [Play] (other2) |
| 134 | TIMER_KEYOFF_2P_KEY4 | 플레이: KeyInputProccessor가 키 뗌 에지에서 setTimerOn(KEYOFF)+setTimerOff(KEYON) | 켬(플) | 2회 [Play] (other2) |
| 135 | TIMER_KEYOFF_2P_KEY5 | 플레이: KeyInputProccessor가 키 뗌 에지에서 setTimerOn(KEYOFF)+setTimerOff(KEYON) | 켬(플) | 2회 [Play] (other2) |
| 136 | TIMER_KEYOFF_2P_KEY6 | 플레이: KeyInputProccessor가 키 뗌 에지에서 setTimerOn(KEYOFF)+setTimerOff(KEYON) | 켬(플) | 1회 [Play] (other1) |
| 137 | TIMER_KEYOFF_2P_KEY7 | 플레이: KeyInputProccessor가 키 뗌 에지에서 setTimerOn(KEYOFF)+setTimerOff(KEYON) | 켬(플) | 1회 [Play] (other1) |
| 138 | TIMER_KEYOFF_2P_KEY8 | 플레이: KeyInputProccessor가 키 뗌 에지에서 setTimerOn(KEYOFF)+setTimerOff(KEYON) | 켬(플) |  |
| 139 | TIMER_KEYOFF_2P_KEY9 | 플레이: KeyInputProccessor가 키 뗌 에지에서 setTimerOn(KEYOFF)+setTimerOff(KEYON) | 켬(플) |  |
| 140 | TIMER_RHYTHM | 플레이: READY->PLAY 전환 시 PLAY와 같은 값으로 setMicroTimer, 이후 RhythmTimerProcessor가 마디/4분음표 경계마다 setTimerOn(RHYTHM)(BPM 동기 박자) | 안 켬(항상 off) | 25회 [Play] (timer25) |
| 143 | TIMER_ENDOFNOTE_1P | 플레이: STATE_PLAY에서 playtime-TIME_MARGIN < ptime이면 switchTimer(true) (곡 끝 직전 마진 구간). 연습 재시작 시 off | 안 켬(항상 off) | 25회 [Play] (timer15,other5,script5) |
| 144 | TIMER_ENDOFNOTE_2P | 선언만: 켜지 않음 | 안 켬(항상 off) |  |
| 150 | TIMER_RESULTGRAPH_BEGIN | 결과/코스결과: 화면 시작 후 매 프레임 switchTimer(true) (진입 즉시 켜짐) | 켬(결) |  |
| 151 | TIMER_RESULTGRAPH_END | 결과/코스결과: RESULTGRAPH_BEGIN과 같은 프레임에 같이 switchTimer(true) (진입 즉시 켜짐, 지연 없음) | 켬(결) |  |
| 152 | TIMER_RESULT_UPDATESCORE | 결과: skin.getRankTime()==0이면 진입 즉시 켬, 아니면 확인 입력(ok)에서 처음 켬(그 뒤 입력이 FADEOUT). 코스결과도 유사 | 켬(결) |  |
| 172 | TIMER_IR_CONNECT_BEGIN | 결과: IR 전송 스레드 시작 시. 선곡: switchTimer(rankingData.state==ACCESS) | 안 켬(항상 off) | 8회 [Result,Root] (other7,timer1) |
| 173 | TIMER_IR_CONNECT_SUCCESS | 결과: 전송 성공 시. 선곡: switchTimer(state==FINISH) | 안 켬(항상 off) | 13회 [Result,Root,Select] (other11,timer2) |
| 174 | TIMER_IR_CONNECT_FAIL | 결과: 전송 실패 시. 선곡: switchTimer(state==FAIL) | 안 켬(항상 off) | 1회 [Result] (timer1) |
| 247 | TIMER_JUDGE_3P | 플레이: 위와 같음 (인덱스 2) | 안 켬(항상 off) |  |
| 250 | TIMER_HCN_ACTIVE_1P_SCRATCH | 플레이: HCN(헬차지노트) 통과 중 증가 상태에서 switchTimer(true), 아니면 off | 안 켬(항상 off) |  |
| 251 | TIMER_HCN_ACTIVE_1P_KEY1 | 플레이: HCN(헬차지노트) 통과 중 증가 상태에서 switchTimer(true), 아니면 off | 안 켬(항상 off) |  |
| 270 | TIMER_HCN_DAMAGE_1P_SCRATCH | 플레이: HCN 통과 중 감소 상태에서 switchTimer(true), 아니면 off | 안 켬(항상 off) |  |
| 271 | TIMER_HCN_DAMAGE_1P_KEY1 | 플레이: HCN 통과 중 감소 상태에서 switchTimer(true), 아니면 off | 안 켬(항상 off) |  |
| 348 | TIMER_SCORE_A | 플레이: update()마다 switchTimer(qualifyRank(18)) | 안 켬(항상 off) |  |
| 349 | TIMER_SCORE_AA | 플레이: switchTimer(qualifyRank(21)) | 안 켬(항상 off) |  |
| 350 | TIMER_SCORE_AAA | 플레이: switchTimer(qualifyRank(24)) | 안 켬(항상 off) |  |
| 351 | TIMER_SCORE_BEST | 플레이: switchTimer(exscore >= bestScore) | 안 켬(항상 off) | 1회 [Root] (other1) |
| 352 | TIMER_SCORE_TARGET | 플레이: switchTimer(exscore >= rivalScore) | 안 켬(항상 off) | 1회 [Root] (other1) |
| 446 | TIMER_COMBO_1P | 플레이: 판정마다 setTimerOn(COMBO_TIMER[idx]), judgenow.length>=3이면 다른 인덱스 콤보 타이머는 off | 켬(플) |  |
| 447 | TIMER_COMBO_2P | 플레이: 위와 같음 (인덱스 1) | 켬(플) |  |
| 448 | TIMER_COMBO_3P | 플레이: 위와 같음 (인덱스 2) | 안 켬(항상 off) |  |
| 900 | TIMER_PM_CHARA_1P_NEUTRAL | 플레이: 포뮤캐릭터(PomyuCharaProcessor/BMSPlayer) 모션 타이머. 판정/게이지/곡 종료에 따라 전환 | 안 켬(항상 off) |  |
| 901 | TIMER_PM_CHARA_1P_FEVER | 플레이: 포뮤캐릭터(PomyuCharaProcessor/BMSPlayer) 모션 타이머. 판정/게이지/곡 종료에 따라 전환 | 안 켬(항상 off) |  |
| 902 | TIMER_PM_CHARA_1P_GREAT | 플레이: 포뮤캐릭터(PomyuCharaProcessor/BMSPlayer) 모션 타이머. 판정/게이지/곡 종료에 따라 전환 | 안 켬(항상 off) |  |
| 903 | TIMER_PM_CHARA_1P_GOOD | 플레이: 포뮤캐릭터(PomyuCharaProcessor/BMSPlayer) 모션 타이머. 판정/게이지/곡 종료에 따라 전환 | 안 켬(항상 off) |  |
| 904 | TIMER_PM_CHARA_1P_BAD | 플레이: 포뮤캐릭터(PomyuCharaProcessor/BMSPlayer) 모션 타이머. 판정/게이지/곡 종료에 따라 전환 | 안 켬(항상 off) |  |
| 905 | TIMER_PM_CHARA_2P_NEUTRAL | 플레이: 포뮤캐릭터(PomyuCharaProcessor/BMSPlayer) 모션 타이머. 판정/게이지/곡 종료에 따라 전환 | 안 켬(항상 off) |  |
| 906 | TIMER_PM_CHARA_2P_GREAT | 플레이: 포뮤캐릭터(PomyuCharaProcessor/BMSPlayer) 모션 타이머. 판정/게이지/곡 종료에 따라 전환 | 안 켬(항상 off) |  |
| 907 | TIMER_PM_CHARA_2P_BAD | 플레이: 포뮤캐릭터(PomyuCharaProcessor/BMSPlayer) 모션 타이머. 판정/게이지/곡 종료에 따라 전환 | 안 켬(항상 off) |  |
| 908 | TIMER_MUSIC_END | 플레이: STATE_PLAY->STATE_FINISHED 전환 시 setTimerOn (포뮤 캐릭터 타이머 정리). 경과>skin.getFinishMargin()이면 FADEOUT 켬 | 안 켬(항상 off) |  |
| 909 | TIMER_PM_CHARA_DANCE | 플레이: 포뮤캐릭터(PomyuCharaProcessor/BMSPlayer) 모션 타이머. 판정/게이지/곡 종료에 따라 전환 | 안 켬(항상 off) |  |
| 1010 | TIMER_BOMB_1P_KEY10 | 플레이: JudgeManager가 판정 judge<=PlaySkin.judgetimer(기본 1: PG,GR)일 때 setTimerOn(bombTimerId). 끄는 코드는 없음(애니메이션 경과로 소멸) | 켬(플) |  |
| 1099 | TIMER_BOMB_1P_KEY99 | 플레이: JudgeManager가 판정 judge<=PlaySkin.judgetimer(기본 1: PG,GR)일 때 setTimerOn(bombTimerId). 끄는 코드는 없음(애니메이션 경과로 소멸) | 켬(플) |  |
| 1110 | TIMER_BOMB_2P_KEY10 | 플레이: JudgeManager가 판정 judge<=PlaySkin.judgetimer(기본 1: PG,GR)일 때 setTimerOn(bombTimerId). 끄는 코드는 없음(애니메이션 경과로 소멸) | 켬(플) |  |
| 1199 | TIMER_BOMB_2P_KEY99 | 플레이: JudgeManager가 판정 judge<=PlaySkin.judgetimer(기본 1: PG,GR)일 때 setTimerOn(bombTimerId). 끄는 코드는 없음(애니메이션 경과로 소멸) | 켬(플) |  |
| 1210 | TIMER_HOLD_1P_KEY10 | 플레이: JudgeManager가 매 프레임 switchTimer(processing!=null // (passing!=null && inclease)) = 롱노트 누르는 동안 on | 켬(플) |  |
| 1299 | TIMER_HOLD_1P_KEY99 | 플레이: JudgeManager가 매 프레임 switchTimer(processing!=null // (passing!=null && inclease)) = 롱노트 누르는 동안 on | 켬(플) |  |
| 1310 | TIMER_HOLD_2P_KEY10 | 플레이: JudgeManager가 매 프레임 switchTimer(processing!=null // (passing!=null && inclease)) = 롱노트 누르는 동안 on | 켬(플) |  |
| 1399 | TIMER_HOLD_2P_KEY99 | 플레이: JudgeManager가 매 프레임 switchTimer(processing!=null // (passing!=null && inclease)) = 롱노트 누르는 동안 on | 켬(플) |  |
| 1410 | TIMER_KEYON_1P_KEY10 | 플레이: KeyInputProccessor가 키 누름 에지에서 setTimerOn(KEYON)+setTimerOff(KEYOFF) (스크래치 방향 전환 시도 재시작) | 켬(플) |  |
| 1499 | TIMER_KEYON_1P_KEY99 | 플레이: KeyInputProccessor가 키 누름 에지에서 setTimerOn(KEYON)+setTimerOff(KEYOFF) (스크래치 방향 전환 시도 재시작) | 켬(플) |  |
| 1510 | TIMER_KEYON_2P_KEY10 | 플레이: KeyInputProccessor가 키 누름 에지에서 setTimerOn(KEYON)+setTimerOff(KEYOFF) (스크래치 방향 전환 시도 재시작) | 켬(플) |  |
| 1599 | TIMER_KEYON_2P_KEY99 | 플레이: KeyInputProccessor가 키 누름 에지에서 setTimerOn(KEYON)+setTimerOff(KEYOFF) (스크래치 방향 전환 시도 재시작) | 켬(플) |  |
| 1610 | TIMER_KEYOFF_1P_KEY10 | 플레이: KeyInputProccessor가 키 뗌 에지에서 setTimerOn(KEYOFF)+setTimerOff(KEYON) | 켬(플) |  |
| 1699 | TIMER_KEYOFF_1P_KEY99 | 플레이: KeyInputProccessor가 키 뗌 에지에서 setTimerOn(KEYOFF)+setTimerOff(KEYON) | 켬(플) |  |
| 1710 | TIMER_KEYOFF_2P_KEY10 | 플레이: KeyInputProccessor가 키 뗌 에지에서 setTimerOn(KEYOFF)+setTimerOff(KEYON) | 켬(플) |  |
| 1799 | TIMER_KEYOFF_2P_KEY99 | 플레이: KeyInputProccessor가 키 뗌 에지에서 setTimerOn(KEYOFF)+setTimerOff(KEYON) | 켬(플) |  |
| 1810 | TIMER_HCN_ACTIVE_1P_KEY10 | 플레이: HCN(헬차지노트) 통과 중 증가 상태에서 switchTimer(true), 아니면 off | 안 켬(항상 off) |  |
| 1910 | TIMER_HCN_ACTIVE_2P_KEY10 | 플레이: HCN(헬차지노트) 통과 중 증가 상태에서 switchTimer(true), 아니면 off | 안 켬(항상 off) |  |
| 2010 | TIMER_HCN_DAMAGE_1P_KEY10 | 플레이: HCN 통과 중 감소 상태에서 switchTimer(true), 아니면 off | 안 켬(항상 off) |  |
| 2110 | TIMER_HCN_DAMAGE_2P_KEY10 | 플레이: HCN 통과 중 감소 상태에서 switchTimer(true), 아니면 off | 안 켬(항상 off) |  |
| 2999 | TIMER_MAX | 배열 경계 상수(2999): TimerManager.timerCount=MAX+1. 타이머가 아님 | - |  |
| 10000 | TIMER_CUSTOM_BEGIN | 경계 상수(10000): 스킨 정의 커스텀 타이머 구간 시작 | - |  |
| 19999 | TIMER_CUSTOM_END | 경계 상수(19999) | - |  |

## 8. 이벤트 (BUTTON_*, EVENT_*) - 쓰기 가능한 속성

조회 함수: `EventFactory.getEvent(int|String)` (EventFactory.java:47-79). 해석 순서는 `EventPattern`(keyassign 54개, practice_item 16개) -> `EventType` enum -> 둘 다 아니면 **항상 이벤트를 만들어** `state.executeEvent(eventId, arg1, arg2)`로 위임한다(:59). 이름 조회는 enum 이름 또는 `keyassignN`/`practice_itemN`만 지원하고 모르는 이름이면 null. `Event.exec(state, arg1, arg2)`에서 **arg1의 부호가 방향**이다(`arg1>=0` 다음, `<0` 이전). `duration1p`만 arg2를 증가량으로 쓴다. 대부분의 EventType은 `state instanceof MusicSelector`일 때만 동작하고 다른 화면에서는 무동작이다(예외: 74, 75, 89, 90, 19/316-318, 210).

`MainState.executeEvent(int,...)`의 기본 구현은 id가 커스텀 범위(1000..1999, `SkinPropertyMapper.isCustomEventId`)일 때만 `Skin.executeCustomEvent`로 스킨이 등록한 이벤트를 실행하고 그 외 id는 무시한다(MainState.java:90-94). `SkinConfiguration.executeEvent`는 `BUTTON_CHANGE_SKIN(190)`(arg1>=0 다음 스킨/<0 이전)과 `BUTTON_SKIN_CUSTOMIZE1..9`와 `BUTTON_SKINSELECT_*`를 처리한다(SkinConfiguration.java:114-150). **주의(beatoraja 결함):** `isSkinCustomizeButton`은 `id < BUTTON_SKIN_CUSTOMIZE10`(220..228)이라 `BUTTON_SKIN_CUSTOMIZE10(229)`는 동작하지 않는다(SkinPropertyMapper.java:124-126). `EVENT_CUSTOM_BEGIN..END`(1000..1999)는 스킨이 `CustomEvent`로 등록한 id만 실행된다. Lua `main_state.event_exec(id[,a1[,a2]])`도 같은 경로다(`isEventRunnableBySkin`은 항상 true).

| id | 상수 | beatoraja 구현 | 화면 | 동작 | R-BMS | ModernChic 사용(BUTTON) |
|---|---|---|---|---|---|---|
| 10 | (상수 없음) | EventType.difficulty | 선곡 | 난이도 필터를 다음/이전으로 순환(MusicSelector.DIFFICULTY), 바 갱신+OPTION_CHANGE 소리 | 없음 |  |
| 11 | BUTTON_MODE | EventType.mode | 선곡 | ModeFilter.nextEnabled로 모드 필터 순환, 바 갱신 | 없음 | 2회 [Select] (ref1,act1) |
| 12 | BUTTON_SORT | EventType.sort | 선곡 | BarSorter.defaultSorter 길이로 정렬 순환, 바 갱신 | 없음 | 2회 [Select] (ref1,act1) |
| 13 | BUTTON_KEYCONFIG | EventType.keyconfig | 선곡 | MainStateType.CONFIG로 전환 | 없음 | 1회 [Select] (act1) |
| 14 | BUTTON_SKINSELECT | EventType.skinconfig | 선곡 | MainStateType.SKINCONFIG로 전환 | 없음 | 1회 [Select] (act1) |
| 15 | BUTTON_PLAY | EventType.play | 선곡 | selectSong(PLAY) | 없음 |  |
| 16 | BUTTON_AUTOPLAY | EventType.autoplay | 선곡 | selectSong(AUTOPLAY) | 없음 | 1회 [Select] (act1) |
| 17 | BUTTON_READTEXT | EventType.open_document | 선곡 | 선택 곡 폴더의 .txt 파일을 OS 기본 뷰어로 염 (BUTTON_READTEXT) | 없음 |  |
| 19 | BUTTON_REPLAY | EventType.replay1 | 선곡,결과,코스결과 | 선곡=리플레이 슬롯0으로 곡 선택(selectSong), 결과=saveReplayData(0) | 없음 | 2회 [Select] (op1,act1) |
| 40 | BUTTON_GAUGE_1P | EventType.gauge1p | 선곡 | PlayerConfig.gauge를 (g+1)%6 또는 (g+5)%6 | 없음 | 2회 [Select] (ref2) |
| 41 | BUTTON_GAUGE_2P | **구현 없음(상수만)** | - | 팩토리 기본 경로 -> MainState.executeEvent -> 무시(무동작) | 없음 |  |
| 42 | BUTTON_RANDOM_1P | EventType.option1p | 선곡 | PlayerConfig.random (g±)%10 | 없음 | 5회 [Play,Result,Select] (ref4,act1) |
| 43 | BUTTON_RANDOM_2P | EventType.option2p | 선곡 | PlayerConfig.random2 %10 | 없음 | 5회 [Play,Result,Select] (ref4,act1) |
| 54 | BUTTON_DPOPTION | EventType.optiondp | 선곡 | doubleoption %4 | 없음 | 2회 [Select] (ref2) |
| 55 | BUTTON_HSFIX | EventType.hsfix | 선곡 | 선택 바 PlayConfig.fixhispeed %5 | 없음 | 4회 [Root,Select] (script2,ref2) |
| 57 | (상수 없음) | EventType.hispeed1p | 선곡 | PlayConfig.hispeed += ±hispeedMargin, [HISPEED_MIN,HISPEED_MAX]로 clamp | 없음 |  |
| 59 | (상수 없음) | EventType.duration1p | 선곡 | duration += ±(arg2>0?arg2:1), [DURATION_MIN,DURATION_MAX]로 clamp (arg2가 증가량) | 없음 |  |
| 72 | BUTTON_BGA | EventType.bga | 선곡 | Config.bga (b±)%3 | 없음 | 1회 [Select] (ref1) |
| 73 | (상수 없음) | EventType.bgaexpand | 선곡 | Config.bgaExpand %3 | 없음 |  |
| 74 | BUTTON_JUDGE_TIMING | EventType.notesdisplaytiming | 공통 | PlayerConfig.judgetiming ±1 (범위 JUDGETIMING_MIN..MAX) (BUTTON_JUDGE_TIMING) | 없음 | 3회 [Play,Select] (act3) |
| 75 | (상수 없음) | EventType.notesdisplaytimingautoadjust | 공통 | 자동 보정 토글 | 없음 | 1회 [Select] (ref1) |
| 77 | BUTTON_TARGET | EventType.target | 선곡 | TargetProperty.getTargets()에서 targetid 순환 (BUTTON_TARGET) | 없음 |  |
| 78 | BUTTON_GAUGEAUTOSHIFT | EventType.gaugeautoshift | 선곡 | gaugeAutoShift %5 | 없음 | 1회 [Select] (ref1) |
| 79 | BUTTON_RIVAL | EventType.rival | 선곡 | 라이벌 목록 순환(없음 포함) | 없음 | 1회 [Select] (act1) |
| 89 | BUTTON_FAVORITTE_SONG | EventType.favorite_song | 선곡,결과 | 같은 폴더 곡 전체에 FAVORITE_SONG/INVISIBLE_SONG 3상태 순환 | 없음 |  |
| 90 | BUTTON_FAVORITTE_CHART | EventType.favorite_chart | 선곡,결과 | 선택 곡 FAVORITE_CHART/INVISIBLE_CHART 3상태 순환 + 토스트 | 없음 | 4회 [Result,Select] (ref2,act2) |
| 101 | (상수 없음) | EventPattern.KEY_ASSIGN (keyassign1) | 키컨피그 | `changeKeyAssign`의 본문이 비어 있어 실제 동작 없음(키 할당 변경은 KeyConfiguration 자체 입력으로 처리) | 없음 |  |
| 102 | (상수 없음) | EventPattern.KEY_ASSIGN (keyassign2) | 키컨피그 | `changeKeyAssign`의 본문이 비어 있어 실제 동작 없음(키 할당 변경은 KeyConfiguration 자체 입력으로 처리) | 없음 |  |
| 103 | (상수 없음) | EventPattern.KEY_ASSIGN (keyassign3) | 키컨피그 | `changeKeyAssign`의 본문이 비어 있어 실제 동작 없음(키 할당 변경은 KeyConfiguration 자체 입력으로 처리) | 없음 |  |
| 104 | (상수 없음) | EventPattern.KEY_ASSIGN (keyassign4) | 키컨피그 | `changeKeyAssign`의 본문이 비어 있어 실제 동작 없음(키 할당 변경은 KeyConfiguration 자체 입력으로 처리) | 없음 |  |
| 105 | (상수 없음) | EventPattern.KEY_ASSIGN (keyassign5) | 키컨피그 | `changeKeyAssign`의 본문이 비어 있어 실제 동작 없음(키 할당 변경은 KeyConfiguration 자체 입력으로 처리) | 없음 |  |
| 106 | (상수 없음) | EventPattern.KEY_ASSIGN (keyassign6) | 키컨피그 | `changeKeyAssign`의 본문이 비어 있어 실제 동작 없음(키 할당 변경은 KeyConfiguration 자체 입력으로 처리) | 없음 |  |
| 107 | (상수 없음) | EventPattern.KEY_ASSIGN (keyassign7) | 키컨피그 | `changeKeyAssign`의 본문이 비어 있어 실제 동작 없음(키 할당 변경은 KeyConfiguration 자체 입력으로 처리) | 없음 |  |
| 108 | (상수 없음) | EventPattern.KEY_ASSIGN (keyassign8) | 키컨피그 | `changeKeyAssign`의 본문이 비어 있어 실제 동작 없음(키 할당 변경은 KeyConfiguration 자체 입력으로 처리) | 없음 |  |
| 109 | (상수 없음) | EventPattern.KEY_ASSIGN (keyassign9) | 키컨피그 | `changeKeyAssign`의 본문이 비어 있어 실제 동작 없음(키 할당 변경은 KeyConfiguration 자체 입력으로 처리) | 없음 |  |
| 110 | (상수 없음) | EventPattern.KEY_ASSIGN (keyassign10) | 키컨피그 | `changeKeyAssign`의 본문이 비어 있어 실제 동작 없음(키 할당 변경은 KeyConfiguration 자체 입력으로 처리) | 없음 |  |
| 111 | (상수 없음) | EventPattern.KEY_ASSIGN (keyassign11) | 키컨피그 | `changeKeyAssign`의 본문이 비어 있어 실제 동작 없음(키 할당 변경은 KeyConfiguration 자체 입력으로 처리) | 없음 |  |
| 112 | (상수 없음) | EventPattern.KEY_ASSIGN (keyassign12) | 키컨피그 | `changeKeyAssign`의 본문이 비어 있어 실제 동작 없음(키 할당 변경은 KeyConfiguration 자체 입력으로 처리) | 없음 |  |
| 113 | (상수 없음) | EventPattern.KEY_ASSIGN (keyassign13) | 키컨피그 | `changeKeyAssign`의 본문이 비어 있어 실제 동작 없음(키 할당 변경은 KeyConfiguration 자체 입력으로 처리) | 없음 |  |
| 114 | (상수 없음) | EventPattern.KEY_ASSIGN (keyassign14) | 키컨피그 | `changeKeyAssign`의 본문이 비어 있어 실제 동작 없음(키 할당 변경은 KeyConfiguration 자체 입력으로 처리) | 없음 |  |
| 115 | (상수 없음) | EventPattern.KEY_ASSIGN (keyassign15) | 키컨피그 | `changeKeyAssign`의 본문이 비어 있어 실제 동작 없음(키 할당 변경은 KeyConfiguration 자체 입력으로 처리) | 없음 |  |
| 116 | (상수 없음) | EventPattern.KEY_ASSIGN (keyassign16) | 키컨피그 | `changeKeyAssign`의 본문이 비어 있어 실제 동작 없음(키 할당 변경은 KeyConfiguration 자체 입력으로 처리) | 없음 |  |
| 117 | (상수 없음) | EventPattern.KEY_ASSIGN (keyassign17) | 키컨피그 | `changeKeyAssign`의 본문이 비어 있어 실제 동작 없음(키 할당 변경은 KeyConfiguration 자체 입력으로 처리) | 없음 |  |
| 118 | (상수 없음) | EventPattern.KEY_ASSIGN (keyassign18) | 키컨피그 | `changeKeyAssign`의 본문이 비어 있어 실제 동작 없음(키 할당 변경은 KeyConfiguration 자체 입력으로 처리) | 없음 |  |
| 119 | (상수 없음) | EventPattern.KEY_ASSIGN (keyassign19) | 키컨피그 | `changeKeyAssign`의 본문이 비어 있어 실제 동작 없음(키 할당 변경은 KeyConfiguration 자체 입력으로 처리) | 없음 |  |
| 120 | (상수 없음) | EventPattern.KEY_ASSIGN (keyassign20) | 키컨피그 | `changeKeyAssign`의 본문이 비어 있어 실제 동작 없음(키 할당 변경은 KeyConfiguration 자체 입력으로 처리) | 없음 |  |
| 121 | (상수 없음) | EventPattern.KEY_ASSIGN (keyassign21) | 키컨피그 | `changeKeyAssign`의 본문이 비어 있어 실제 동작 없음(키 할당 변경은 KeyConfiguration 자체 입력으로 처리) | 없음 |  |
| 122 | (상수 없음) | EventPattern.KEY_ASSIGN (keyassign22) | 키컨피그 | `changeKeyAssign`의 본문이 비어 있어 실제 동작 없음(키 할당 변경은 KeyConfiguration 자체 입력으로 처리) | 없음 |  |
| 123 | (상수 없음) | EventPattern.KEY_ASSIGN (keyassign23) | 키컨피그 | `changeKeyAssign`의 본문이 비어 있어 실제 동작 없음(키 할당 변경은 KeyConfiguration 자체 입력으로 처리) | 없음 |  |
| 124 | (상수 없음) | EventPattern.KEY_ASSIGN (keyassign24) | 키컨피그 | `changeKeyAssign`의 본문이 비어 있어 실제 동작 없음(키 할당 변경은 KeyConfiguration 자체 입력으로 처리) | 없음 |  |
| 125 | (상수 없음) | EventPattern.KEY_ASSIGN (keyassign25) | 키컨피그 | `changeKeyAssign`의 본문이 비어 있어 실제 동작 없음(키 할당 변경은 KeyConfiguration 자체 입력으로 처리) | 없음 |  |
| 126 | (상수 없음) | EventPattern.KEY_ASSIGN (keyassign26) | 키컨피그 | `changeKeyAssign`의 본문이 비어 있어 실제 동작 없음(키 할당 변경은 KeyConfiguration 자체 입력으로 처리) | 없음 |  |
| 127 | (상수 없음) | EventPattern.KEY_ASSIGN (keyassign27) | 키컨피그 | `changeKeyAssign`의 본문이 비어 있어 실제 동작 없음(키 할당 변경은 KeyConfiguration 자체 입력으로 처리) | 없음 |  |
| 128 | (상수 없음) | EventPattern.KEY_ASSIGN (keyassign28) | 키컨피그 | `changeKeyAssign`의 본문이 비어 있어 실제 동작 없음(키 할당 변경은 KeyConfiguration 자체 입력으로 처리) | 없음 |  |
| 129 | (상수 없음) | EventPattern.KEY_ASSIGN (keyassign29) | 키컨피그 | `changeKeyAssign`의 본문이 비어 있어 실제 동작 없음(키 할당 변경은 KeyConfiguration 자체 입력으로 처리) | 없음 |  |
| 130 | (상수 없음) | EventPattern.KEY_ASSIGN (keyassign30) | 키컨피그 | `changeKeyAssign`의 본문이 비어 있어 실제 동작 없음(키 할당 변경은 KeyConfiguration 자체 입력으로 처리) | 없음 |  |
| 131 | (상수 없음) | EventPattern.KEY_ASSIGN (keyassign31) | 키컨피그 | `changeKeyAssign`의 본문이 비어 있어 실제 동작 없음(키 할당 변경은 KeyConfiguration 자체 입력으로 처리) | 없음 |  |
| 132 | (상수 없음) | EventPattern.KEY_ASSIGN (keyassign32) | 키컨피그 | `changeKeyAssign`의 본문이 비어 있어 실제 동작 없음(키 할당 변경은 KeyConfiguration 자체 입력으로 처리) | 없음 |  |
| 133 | (상수 없음) | EventPattern.KEY_ASSIGN (keyassign33) | 키컨피그 | `changeKeyAssign`의 본문이 비어 있어 실제 동작 없음(키 할당 변경은 KeyConfiguration 자체 입력으로 처리) | 없음 |  |
| 134 | (상수 없음) | EventPattern.KEY_ASSIGN (keyassign34) | 키컨피그 | `changeKeyAssign`의 본문이 비어 있어 실제 동작 없음(키 할당 변경은 KeyConfiguration 자체 입력으로 처리) | 없음 |  |
| 135 | (상수 없음) | EventPattern.KEY_ASSIGN (keyassign35) | 키컨피그 | `changeKeyAssign`의 본문이 비어 있어 실제 동작 없음(키 할당 변경은 KeyConfiguration 자체 입력으로 처리) | 없음 |  |
| 136 | (상수 없음) | EventPattern.KEY_ASSIGN (keyassign36) | 키컨피그 | `changeKeyAssign`의 본문이 비어 있어 실제 동작 없음(키 할당 변경은 KeyConfiguration 자체 입력으로 처리) | 없음 |  |
| 137 | (상수 없음) | EventPattern.KEY_ASSIGN (keyassign37) | 키컨피그 | `changeKeyAssign`의 본문이 비어 있어 실제 동작 없음(키 할당 변경은 KeyConfiguration 자체 입력으로 처리) | 없음 |  |
| 138 | (상수 없음) | EventPattern.KEY_ASSIGN (keyassign38) | 키컨피그 | `changeKeyAssign`의 본문이 비어 있어 실제 동작 없음(키 할당 변경은 KeyConfiguration 자체 입력으로 처리) | 없음 |  |
| 139 | (상수 없음) | EventPattern.KEY_ASSIGN (keyassign39) | 키컨피그 | `changeKeyAssign`의 본문이 비어 있어 실제 동작 없음(키 할당 변경은 KeyConfiguration 자체 입력으로 처리) | 없음 |  |
| 150 | (상수 없음) | EventPattern.KEY_ASSIGN (keyassign40) | 키컨피그 | `changeKeyAssign`의 본문이 비어 있어 실제 동작 없음(키 할당 변경은 KeyConfiguration 자체 입력으로 처리) | 없음 |  |
| 151 | (상수 없음) | EventPattern.KEY_ASSIGN (keyassign41) | 키컨피그 | `changeKeyAssign`의 본문이 비어 있어 실제 동작 없음(키 할당 변경은 KeyConfiguration 자체 입력으로 처리) | 없음 |  |
| 152 | (상수 없음) | EventPattern.KEY_ASSIGN (keyassign42) | 키컨피그 | `changeKeyAssign`의 본문이 비어 있어 실제 동작 없음(키 할당 변경은 KeyConfiguration 자체 입력으로 처리) | 없음 |  |
| 153 | (상수 없음) | EventPattern.KEY_ASSIGN (keyassign43) | 키컨피그 | `changeKeyAssign`의 본문이 비어 있어 실제 동작 없음(키 할당 변경은 KeyConfiguration 자체 입력으로 처리) | 없음 |  |
| 154 | (상수 없음) | EventPattern.KEY_ASSIGN (keyassign44) | 키컨피그 | `changeKeyAssign`의 본문이 비어 있어 실제 동작 없음(키 할당 변경은 KeyConfiguration 자체 입력으로 처리) | 없음 |  |
| 155 | (상수 없음) | EventPattern.KEY_ASSIGN (keyassign45) | 키컨피그 | `changeKeyAssign`의 본문이 비어 있어 실제 동작 없음(키 할당 변경은 KeyConfiguration 자체 입력으로 처리) | 없음 |  |
| 156 | (상수 없음) | EventPattern.KEY_ASSIGN (keyassign46) | 키컨피그 | `changeKeyAssign`의 본문이 비어 있어 실제 동작 없음(키 할당 변경은 KeyConfiguration 자체 입력으로 처리) | 없음 |  |
| 157 | (상수 없음) | EventPattern.KEY_ASSIGN (keyassign47) | 키컨피그 | `changeKeyAssign`의 본문이 비어 있어 실제 동작 없음(키 할당 변경은 KeyConfiguration 자체 입력으로 처리) | 없음 |  |
| 158 | (상수 없음) | EventPattern.KEY_ASSIGN (keyassign48) | 키컨피그 | `changeKeyAssign`의 본문이 비어 있어 실제 동작 없음(키 할당 변경은 KeyConfiguration 자체 입력으로 처리) | 없음 |  |
| 159 | (상수 없음) | EventPattern.KEY_ASSIGN (keyassign49) | 키컨피그 | `changeKeyAssign`의 본문이 비어 있어 실제 동작 없음(키 할당 변경은 KeyConfiguration 자체 입력으로 처리) | 없음 |  |
| 160 | (상수 없음) | EventPattern.KEY_ASSIGN (keyassign50) | 키컨피그 | `changeKeyAssign`의 본문이 비어 있어 실제 동작 없음(키 할당 변경은 KeyConfiguration 자체 입력으로 처리) | 없음 |  |
| 161 | (상수 없음) | EventPattern.KEY_ASSIGN (keyassign51) | 키컨피그 | `changeKeyAssign`의 본문이 비어 있어 실제 동작 없음(키 할당 변경은 KeyConfiguration 자체 입력으로 처리) | 없음 |  |
| 162 | (상수 없음) | EventPattern.KEY_ASSIGN (keyassign52) | 키컨피그 | `changeKeyAssign`의 본문이 비어 있어 실제 동작 없음(키 할당 변경은 KeyConfiguration 자체 입력으로 처리) | 없음 |  |
| 163 | (상수 없음) | EventPattern.KEY_ASSIGN (keyassign53) | 키컨피그 | `changeKeyAssign`의 본문이 비어 있어 실제 동작 없음(키 할당 변경은 KeyConfiguration 자체 입력으로 처리) | 없음 |  |
| 170 | BUTTON_SKINSELECT_7KEY | 팩토리 기본 경로 -> SkinConfiguration.executeEvent | 스킨설정 | changeSkinType(SkinType PLAY_7KEYS) = 편집 대상 스킨 종류 전환 | 없음 | 1회 [skinselect] (op1) |
| 171 | BUTTON_SKINSELECT_5KEY | 팩토리 기본 경로 -> SkinConfiguration.executeEvent | 스킨설정 | changeSkinType(SkinType PLAY_5KEYS) = 편집 대상 스킨 종류 전환 | 없음 | 1회 [skinselect] (other1) |
| 172 | BUTTON_SKINSELECT_14KEY | 팩토리 기본 경로 -> SkinConfiguration.executeEvent | 스킨설정 | changeSkinType(SkinType PLAY_14KEYS) = 편집 대상 스킨 종류 전환 | 없음 | 1회 [skinselect] (other1) |
| 173 | BUTTON_SKINSELECT_10KEY | 팩토리 기본 경로 -> SkinConfiguration.executeEvent | 스킨설정 | changeSkinType(SkinType PLAY_10KEYS) = 편집 대상 스킨 종류 전환 | 없음 | 1회 [skinselect] (other1) |
| 174 | BUTTON_SKINSELECT_9KEY | 팩토리 기본 경로 -> SkinConfiguration.executeEvent | 스킨설정 | changeSkinType(SkinType PLAY_9KEYS) = 편집 대상 스킨 종류 전환 | 없음 | 1회 [skinselect] (other1) |
| 175 | BUTTON_SKINSELECT_MUSIC_SELECT | 팩토리 기본 경로 -> SkinConfiguration.executeEvent | 스킨설정 | changeSkinType(SkinType MUSIC_SELECT) = 편집 대상 스킨 종류 전환 | 없음 | 1회 [skinselect] (op1) |
| 176 | BUTTON_SKINSELECT_DECIDE | 팩토리 기본 경로 -> SkinConfiguration.executeEvent | 스킨설정 | changeSkinType(SkinType DECIDE) = 편집 대상 스킨 종류 전환 | 없음 | 1회 [skinselect] (other1) |
| 177 | BUTTON_SKINSELECT_RESULT | 팩토리 기본 경로 -> SkinConfiguration.executeEvent | 스킨설정 | changeSkinType(SkinType RESULT) = 편집 대상 스킨 종류 전환 | 없음 | 1회 [skinselect] (other1) |
| 178 | BUTTON_SKINSELECT_KEY_CONFIG | 팩토리 기본 경로 -> SkinConfiguration.executeEvent | 스킨설정 | changeSkinType(SkinType KEY_CONFIG) = 편집 대상 스킨 종류 전환 | 없음 | 1회 [skinselect] (other1) |
| 179 | BUTTON_SKINSELECT_SKIN_SELECT | 팩토리 기본 경로 -> SkinConfiguration.executeEvent | 스킨설정 | changeSkinType(SkinType SKIN_SELECT) = 편집 대상 스킨 종류 전환 | 없음 | 1회 [skinselect] (other1) |
| 180 | BUTTON_SKINSELECT_SOUND_SET | 팩토리 기본 경로 -> SkinConfiguration.executeEvent | 스킨설정 | changeSkinType(SkinType SOUND_SET) = 편집 대상 스킨 종류 전환 | 없음 | 1회 [skinselect] (other1) |
| 181 | BUTTON_SKINSELECT_THEME | 팩토리 기본 경로 -> SkinConfiguration.executeEvent | 스킨설정 | changeSkinType(SkinType THEME) = 편집 대상 스킨 종류 전환 | 없음 | 1회 [skinselect] (other1) |
| 182 | BUTTON_SKINSELECT_BATTLE7 | 팩토리 기본 경로 -> SkinConfiguration.executeEvent | 스킨설정 | changeSkinType(SkinType PLAY_7KEYS_BATTLE) = 편집 대상 스킨 종류 전환 | 없음 | 1회 [skinselect] (other1) |
| 183 | BUTTON_SKINSELECT_BATTLE5 | 팩토리 기본 경로 -> SkinConfiguration.executeEvent | 스킨설정 | changeSkinType(SkinType PLAY_5KEYS_BATTLE) = 편집 대상 스킨 종류 전환 | 없음 |  |
| 184 | BUTTON_SKINSELECT_BATTLE9 | 팩토리 기본 경로 -> SkinConfiguration.executeEvent | 스킨설정 | changeSkinType(SkinType PLAY_9KEYS_BATTLE) = 편집 대상 스킨 종류 전환 | 없음 | 1회 [skinselect] (other1) |
| 185 | BUTTON_SKINSELECT_COURSE_RESULT | 팩토리 기본 경로 -> SkinConfiguration.executeEvent | 스킨설정 | changeSkinType(SkinType COURSE_RESULT) = 편집 대상 스킨 종류 전환 | 없음 | 1회 [skinselect] (other1) |
| 190 | BUTTON_CHANGE_SKIN | 팩토리 기본 경로 -> SkinConfiguration.executeEvent | 스킨설정 | BUTTON_CHANGE_SKIN: arg1>=0 다음 스킨, <0 이전 스킨 (순환) | 없음 | 1회 [skinselect] (act1) |
| 210 | BUTTON_OPEN_IR_WEBSITE | EventType.open_ir | 선곡,결과,코스결과 | IR 사이트의 곡/코스 URL을 브라우저로 염 (BUTTON_OPEN_IR_WEBSITE) | 없음 | 1회 [Select] (act1) |
| 211 | (상수 없음) | EventType.update_folder | 선곡 | 선택된 폴더/난이도표/곡 폴더를 재스캔 | 없음 |  |
| 212 | (상수 없음) | EventType.open_with_explorer | 선곡 | 곡 파일 위치를 OS 탐색기로 염 | 없음 |  |
| 213 | (상수 없음) | EventType.open_download_site | 선곡 | 곡 URL/추가 URL을 브라우저로 염 | 없음 |  |
| 220 | BUTTON_SKIN_CUSTOMIZE1 | 팩토리 기본 경로 -> SkinConfiguration.executeEvent | 스킨설정 | 커스텀 옵션 항목 1 값을 arg1>=0 +1 / <0 -1 (최대에서 최소로 순환) | 없음 | 1회 [skinselect] (op1) |
| 229 | BUTTON_SKIN_CUSTOMIZE10 | 팩토리 기본 경로 -> SkinConfiguration.executeEvent | 스킨설정 | **동작 안 함**(isSkinCustomizeButton의 id<229 경계 결함) | 없음 | 1회 [skinselect] (other1) |
| 301 | BUTTON_ASSIST_EXJUDGE | **구현 없음(상수만)** | - | 팩토리 기본 경로 -> MainState.executeEvent -> 무시(무동작) | 없음 | 3회 [Root,Select] (script1,ref1,act1) |
| 302 | BUTTON_ASSIST_CONSTANT | **구현 없음(상수만)** | - | 팩토리 기본 경로 -> MainState.executeEvent -> 무시(무동작) | 없음 | 3회 [Root,Select] (script1,ref1,act1) |
| 303 | BUTTON_ASSIST_JUDGEAREA | **구현 없음(상수만)** | - | 팩토리 기본 경로 -> MainState.executeEvent -> 무시(무동작) | 없음 | 3회 [Root,Select] (script1,ref1,act1) |
| 304 | BUTTON_ASSIST_LEGACY | **구현 없음(상수만)** | - | 팩토리 기본 경로 -> MainState.executeEvent -> 무시(무동작) | 없음 | 3회 [Root,Select] (script1,ref1,act1) |
| 305 | BUTTON_ASSIST_MARKNOTE | **구현 없음(상수만)** | - | 팩토리 기본 경로 -> MainState.executeEvent -> 무시(무동작) | 없음 | 3회 [Root,Select] (script1,ref1,act1) |
| 306 | BUTTON_ASSIST_BPMGUIDE | **구현 없음(상수만)** | - | 팩토리 기본 경로 -> MainState.executeEvent -> 무시(무동작) | 없음 | 3회 [Root,Select] (script1,ref1,act1) |
| 307 | BUTTON_ASSIST_NOMINE | **구현 없음(상수만)** | - | 팩토리 기본 경로 -> MainState.executeEvent -> 무시(무동작) | 없음 | 3회 [Root,Select] (script1,ref1,act1) |
| 308 | BUTTON_LNMODE | EventType.lnmode | 선곡 | PlayerConfig.lnmode %3 | 없음 | 9회 [Play,Root,Select] (ref5,script3,act1) |
| 312 | (상수 없음) | EventType.songbar_sort | 선곡 | BarSorter.allSorter에서 현재 sortid 다음/이전으로 이동 | 없음 |  |
| 315 | BUTTON_PRACTICE | EventType.practice | 선곡 | selectSong(PRACTICE) | 없음 | 1회 [Select] (act1) |
| 316 | BUTTON_REPLAY2 | EventType.replay2 | 선곡,결과,코스결과 | 슬롯1 | 없음 | 2회 [Select] (other1,act1) |
| 317 | BUTTON_REPLAY3 | EventType.replay3 | 선곡,결과,코스결과 | 슬롯2 | 없음 | 2회 [Select] (other1,act1) |
| 318 | BUTTON_REPLAY4 | EventType.replay4 | 선곡,결과,코스결과 | 슬롯3 | 없음 | 2회 [Select] (other1,act1) |
| 321 | BUTTON_AUTOSAVEREPLAY_1 | EventType.autosavereplay1 | 선곡 | autoSaveReplay[0] ReplayAutoSaveConstraint 순환 | 없음 | 1회 [Select] (op1) |
| 322 | BUTTON_AUTOSAVEREPLAY_2 | EventType.autosavereplay2 | 선곡 | [1] | 없음 | 1회 [Select] (other1) |
| 323 | BUTTON_AUTOSAVEREPLAY_3 | EventType.autosavereplay3 | 선곡 | [2] | 없음 | 1회 [Select] (other1) |
| 324 | BUTTON_AUTOSAVEREPLAY_4 | EventType.autosavereplay4 | 선곡 | [3] | 없음 | 1회 [Select] (other1) |
| 330 | BUTTON_LANECOVER | EventType.lanecover | 선곡 | PlayConfig.enablelanecover 토글 | 없음 | 2회 [Select] (act1,ref1) |
| 331 | BUTTON_LIFT | EventType.lift | 선곡 | enablelift 토글 | 없음 | 2회 [Select] (act1,ref1) |
| 332 | BUTTON_HIDDEN | EventType.hidden | 선곡 | enablehidden 토글 | 없음 | 2회 [Select] (act1,ref1) |
| 340 | BUTTON_JUDGEALGORITHM | EventType.judgealgorithm | 선곡 | JudgeAlgorithm.defaultAlgorithm 순환 | 없음 | 2회 [Select] (act1,ref1) |
| 341 | BUTTON_BOTTOMSIFTABLEFGAUGE | EventType.bottomshiftablegauge | 선곡 | bottomShiftableGauge %3 | 없음 | 2회 [Select] (act1,ref1) |
| 342 | BUTTON_HISPEEDAUTOADJUST | EventType.hispeedautoadjust | 선곡 | PlayConfig.hispeedAutoAdjust 토글 | 없음 | 2회 [Select] (act1,ref1) |
| 343 | (상수 없음) | EventType.guidese | 선곡 | guideSE 토글 | 없음 |  |
| 344 | (상수 없음) | EventType.chartreplicationmode | 선곡 | ChartReplicationMode 순환 (상수 없음) | 없음 |  |
| 350 | BUTTON_EXTRANOTE | EventType.extranotedepth | 선곡 | %4 | 없음 |  |
| 351 | BUTTON_MINEMODE | EventType.minemode | 선곡 | %5 | 없음 |  |
| 352 | BUTTON_SCROLLMODE | EventType.scrollmode | 선곡 | %3 | 없음 |  |
| 353 | BUTTON_LONGNOTEMODE | EventType.longnotemode | 선곡 | %6 | 없음 |  |
| 360 | BUTTON_SEVENTONINE_PATTERN | EventType.seventonine_pattern | 선곡 | %7 | 없음 |  |
| 361 | BUTTON_SEVENTONINE_TYPE | EventType.seventonine_type | 선곡 | %3 | 없음 |  |
| 370 | BUTTON_PRACTICE_ITEM1 | EventPattern.PRACTICE_ITEM (practice_item1) | 플레이 | state.executeEvent로 위임 -> MainState 기본은 무시 (연습 항목은 PracticeConfiguration이 직접 입력 처리) | 없음 |  |
| 371 | (상수 없음) | EventPattern.PRACTICE_ITEM (practice_item2) | 플레이 | state.executeEvent로 위임 -> MainState 기본은 무시 (연습 항목은 PracticeConfiguration이 직접 입력 처리) | 없음 |  |
| 372 | (상수 없음) | EventPattern.PRACTICE_ITEM (practice_item3) | 플레이 | state.executeEvent로 위임 -> MainState 기본은 무시 (연습 항목은 PracticeConfiguration이 직접 입력 처리) | 없음 |  |
| 373 | (상수 없음) | EventPattern.PRACTICE_ITEM (practice_item4) | 플레이 | state.executeEvent로 위임 -> MainState 기본은 무시 (연습 항목은 PracticeConfiguration이 직접 입력 처리) | 없음 |  |
| 374 | (상수 없음) | EventPattern.PRACTICE_ITEM (practice_item5) | 플레이 | state.executeEvent로 위임 -> MainState 기본은 무시 (연습 항목은 PracticeConfiguration이 직접 입력 처리) | 없음 |  |
| 375 | (상수 없음) | EventPattern.PRACTICE_ITEM (practice_item6) | 플레이 | state.executeEvent로 위임 -> MainState 기본은 무시 (연습 항목은 PracticeConfiguration이 직접 입력 처리) | 없음 |  |
| 376 | (상수 없음) | EventPattern.PRACTICE_ITEM (practice_item7) | 플레이 | state.executeEvent로 위임 -> MainState 기본은 무시 (연습 항목은 PracticeConfiguration이 직접 입력 처리) | 없음 |  |
| 377 | (상수 없음) | EventPattern.PRACTICE_ITEM (practice_item8) | 플레이 | state.executeEvent로 위임 -> MainState 기본은 무시 (연습 항목은 PracticeConfiguration이 직접 입력 처리) | 없음 |  |
| 378 | (상수 없음) | EventPattern.PRACTICE_ITEM (practice_item9) | 플레이 | state.executeEvent로 위임 -> MainState 기본은 무시 (연습 항목은 PracticeConfiguration이 직접 입력 처리) | 없음 |  |
| 379 | (상수 없음) | EventPattern.PRACTICE_ITEM (practice_item10) | 플레이 | state.executeEvent로 위임 -> MainState 기본은 무시 (연습 항목은 PracticeConfiguration이 직접 입력 처리) | 없음 |  |
| 380 | (상수 없음) | EventPattern.PRACTICE_ITEM (practice_item11) | 플레이 | state.executeEvent로 위임 -> MainState 기본은 무시 (연습 항목은 PracticeConfiguration이 직접 입력 처리) | 없음 |  |
| 381 | (상수 없음) | EventPattern.PRACTICE_ITEM (practice_item12) | 플레이 | state.executeEvent로 위임 -> MainState 기본은 무시 (연습 항목은 PracticeConfiguration이 직접 입력 처리) | 없음 |  |
| 382 | (상수 없음) | EventPattern.PRACTICE_ITEM (practice_item13) | 플레이 | state.executeEvent로 위임 -> MainState 기본은 무시 (연습 항목은 PracticeConfiguration이 직접 입력 처리) | 없음 |  |
| 383 | (상수 없음) | EventPattern.PRACTICE_ITEM (practice_item14) | 플레이 | state.executeEvent로 위임 -> MainState 기본은 무시 (연습 항목은 PracticeConfiguration이 직접 입력 처리) | 없음 |  |
| 384 | (상수 없음) | EventPattern.PRACTICE_ITEM (practice_item15) | 플레이 | state.executeEvent로 위임 -> MainState 기본은 무시 (연습 항목은 PracticeConfiguration이 직접 입력 처리) | 없음 |  |
| 385 | BUTTON_PRACTICE_ITEM16 | EventPattern.PRACTICE_ITEM (practice_item16) | 플레이 | state.executeEvent로 위임 -> MainState 기본은 무시 (연습 항목은 PracticeConfiguration이 직접 입력 처리) | 없음 |  |
| 386 | BUTTON_SKINSELECT_24KEY | 팩토리 기본 경로 -> SkinConfiguration.executeEvent | 스킨설정 | changeSkinType(SkinType PLAY_24KEYS) = 편집 대상 스킨 종류 전환 | 없음 | 1회 [skinselect] (other1) |
| 387 | BUTTON_SKINSELECT_24KEY_DOUBLE | 팩토리 기본 경로 -> SkinConfiguration.executeEvent | 스킨설정 | changeSkinType(SkinType PLAY_24KEYS_DOUBLE) = 편집 대상 스킨 종류 전환 | 없음 | 1회 [skinselect] (other1) |
| 388 | BUTTON_SKINSELECT_24KEY_BATTLE | 팩토리 기본 경로 -> SkinConfiguration.executeEvent | 스킨설정 | changeSkinType(SkinType PLAY_24KEYS_BATTLE) = 편집 대상 스킨 종류 전환 | 없음 | 1회 [skinselect] (other1) |
| 400 | (상수 없음) | EventType.constant | 선곡 | PlayConfig.enableConstant 토글 (OPTION_CONSTANT 값 재사용) | 없음 | 2회 [Select] (act1,ref1) |
| 1000 | EVENT_CUSTOM_BEGIN | 팩토리 기본 경로 | 공통 | Skin.executeCustomEvent: 스킨이 등록한 CustomEvent만 실행 | 없음 |  |
| 1999 | EVENT_CUSTOM_END | 팩토리 기본 경로 | 공통 | Skin.executeCustomEvent: 스킨이 등록한 CustomEvent만 실행 | 없음 |  |

## 9. 이미지 참조 인덱스 (SkinImage `ref` / `main_state.event_index`)

조회 함수: `IntegerPropertyFactory.getImageIndexProperty(int|String)` (IntegerPropertyFactory.java:920-974). **NUMBER 공간과 완전히 별개의 id 공간**이다(`icache` vs `vcache`). 값은 이미지 후보 배열의 선택 인덱스(`SkinImage(images, ..., ref)`)다. 우선순위: (1) `VALUE_JUDGE_1P_SCRATCH(500)..VALUE_JUDGE_2P_KEY9(519)`, `VALUE_JUDGE_1P_KEY10(1510)..VALUE_JUDGE_2P_KEY99(1699)` -> 플레이에서 `JudgeManager.getJudge(id)` (키별 마지막 판정 이미지 인덱스), (2) 스킨선택 종류 id(170..185, 386..388) -> 스킨설정에서 `SkinConfiguration.getSkinType()==해당 종류 ? 1 : 0`, 그 외 화면 MIN_VALUE, (3) `IntegerPropertyPattern`(IMAGE_INDEX: playertype_ranking, cleartype_ranking), (4) `IndexType` enum. 목록에 없는 id는 null이며 이 경우 SkinImage는 첫 후보를 그린다. Lua `main_state.event_index(id)`는 null이면 NullPointerException이므로(MainStatePropertyLuaApiExporter.java:300) 스킨은 존재하는 id만 부른다. **ModernChic는 `ref = MAIN.BUTTON.X`를 이미지의 이 공간으로 42회, `main_state.event_index` 14회 사용한다.**

키별 판정 이미지 값(`VALUE_JUDGE_*`): `JudgeManager.judge[player][offset]` = 판정 PG이면 1, 그 외 `judge*2 + (mfast>0 ? 0 : 1)` (GR early=2/late=3, GD 4/5, BD 6/7, MS 10/11), 판정이 POOR(4)이면 갱신하지 않음(JudgeManager.java:672). id->(player, offset): `getKeyJudgeValuePlayer = (id-500)/10` (500..519) 또는 `(id-1510)/100`, `getKeyJudgeValueOffset = (id-500)%10` 또는 `(id-1510)%100+10` (SkinPropertyMapper.java:74-99). 520/521/522(VALUE_JUDGE_1P/2P/3P)와 525..527(VALUE_JUDGE_*_DURATION)은 이미지 인덱스가 아니라 525..527은 정수 `judge_duration1..3`이고 520..522는 팩토리에 구현이 없다.

| id | beatoraja 구현 | 화면 | 값 | 겹치는 NUMBER/기타 상수 | R-BMS | ModernChic 사용 |
|---|---|---|---|---|---|---|
| 10 | IndexType.difficulty | 선곡 | PlayerConfig.getDifficultyFilter().getSkinNumber() | NUMBER_HISPEED_LR2 | **없음(이미지 인덱스 공간 자체가 R-BMS에 없음, state.integer로 대체됨)** |  |
| 11 | IndexType.mode | 선곡 | getModeFilter().getSkinNumber() | BUTTON_MODE | **없음(이미지 인덱스 공간 자체가 R-BMS에 없음, state.integer로 대체됨)** | 2회 [Select] (ref1,act1) |
| 12 | IndexType.sort | 선곡 | MusicSelector.getSort() | NUMBER_JUDGETIMING,BUTTON_SORT | **없음(이미지 인덱스 공간 자체가 R-BMS에 없음, state.integer로 대체됨)** | 2회 [Select] (ref1,act1) |
| 40 | IndexType.gaugetype_1p | 공통 | 플레이=GrooveGauge.getType(), 결과=getGaugeType(), 그 외 PlayerConfig.getGauge() | BUTTON_GAUGE_1P,OPTION_BGAOFF | **없음(이미지 인덱스 공간 자체가 R-BMS에 없음, state.integer로 대체됨)** | 2회 [Select] (ref2) |
| 42 | IndexType.option_1p | 공통 | 플레이=OptionInformation.randomoption, 결과=ReplayData.randomoption, 그 외 PlayerConfig.getRandom() | BUTTON_RANDOM_1P,OPTION_GAUGE_GROOVE | **없음(이미지 인덱스 공간 자체가 R-BMS에 없음, state.integer로 대체됨)** | 5회 [Play,Result,Select] (ref4,act1) |
| 43 | IndexType.option_2p | 공통 | 플레이=randomoption2, 결과=replay.randomoption2, 그 외 PlayerConfig.getRandom2() | BUTTON_RANDOM_2P,OPTION_GAUGE_HARD | **없음(이미지 인덱스 공간 자체가 R-BMS에 없음, state.integer로 대체됨)** | 5회 [Play,Result,Select] (ref4,act1) |
| 54 | IndexType.option_dp | 공통 | doubleoption (같은 3분기) | BUTTON_DPOPTION | **없음(이미지 인덱스 공간 자체가 R-BMS에 없음, state.integer로 대체됨)** | 2회 [Select] (ref2) |
| 55 | IndexType.hsfix | 공통 | songdata 모드 PlayConfig.getFixhispeed(); 곡 없고 코스면 전곡 PlayConfig가 같을 때 그 값, 아니면 MIN_VALUE | BUTTON_HSFIX | **없음(이미지 인덱스 공간 자체가 R-BMS에 없음, state.integer로 대체됨)** | 4회 [Root,Select] (script2,ref2) |
| 61 | IndexType.option_target1_1p | 공통 | 목표 ScoreData.option %10 (옵션 1P) | OPTION_ENABLE_SAVE_SCORE | **없음(이미지 인덱스 공간 자체가 R-BMS에 없음, state.integer로 대체됨)** |  |
| 62 | IndexType.option_target1_2p | 공통 | (option/10)%10 | OPTION_NO_SAVE_CLEAR | **없음(이미지 인덱스 공간 자체가 R-BMS에 없음, state.integer로 대체됨)** |  |
| 63 | IndexType.option_target1_dp | 공통 | (option/100)%10 | OPTION_EASY_SAVE_CLEAR | **없음(이미지 인덱스 공간 자체가 R-BMS에 없음, state.integer로 대체됨)** |  |
| 72 | IndexType.bga | 공통 | Config.getBga() (0..2) | NUMBER_MAXSCORE,BUTTON_BGA,OPTION_LEVEL_HYPER | **없음(이미지 인덱스 공간 자체가 R-BMS에 없음, state.integer로 대체됨)** | 1회 [Select] (ref1) |
| 75 | IndexType.notesdisplaytimingautoadjust | 공통 | 1/0 | NUMBER_MAXCOMBO,OPTION_LEVEL_BEGINNER_EXCEED | **없음(이미지 인덱스 공간 자체가 R-BMS에 없음, state.integer로 대체됨)** | 1회 [Select] (ref1) |
| 78 | IndexType.gaugeautoshift | 공통 | PlayerConfig.getGaugeAutoShift() (0..4) | NUMBER_CLEARCOUNT,BUTTON_GAUGEAUTOSHIFT,OPTION_LEVEL_ANOTHER_EXCEED | **없음(이미지 인덱스 공간 자체가 R-BMS에 없음, state.integer로 대체됨)** | 1회 [Select] (ref1) |
| 89 | IndexType.favorite_song | 공통 | songdata favorite 플래그: 0=없음, 1=FAVORITE_SONG, 2=INVISIBLE_SONG | NUMBER_POOR_RATE,BUTTON_FAVORITTE_SONG | **없음(이미지 인덱스 공간 자체가 R-BMS에 없음, state.integer로 대체됨)** |  |
| 90 | IndexType.favorite_chart | 공통 | 0/1/2 (FAVORITE_CHART/INVISIBLE_CHART) | NUMBER_MAXBPM,BUTTON_FAVORITTE_CHART,OPTION_RESULT_CLEAR | **없음(이미지 인덱스 공간 자체가 R-BMS에 없음, state.integer로 대체됨)** | 4회 [Result,Select] (ref2,act2) |
| 301 | IndexType.customjudge | 공통 | isCustomJudge() ? 1 : 0 | BUTTON_ASSIST_EXJUDGE,OPTION_RESULT_AA_1P | **없음(이미지 인덱스 공간 자체가 R-BMS에 없음, state.integer로 대체됨)** | 3회 [Root,Select] (script1,ref1,act1) |
| 302 | IndexType.assist_constant | 공통 | (구) scrollMode==1 ? 1 : 0 | BUTTON_ASSIST_CONSTANT,OPTION_RESULT_A_1P | **없음(이미지 인덱스 공간 자체가 R-BMS에 없음, state.integer로 대체됨)** | 3회 [Root,Select] (script1,ref1,act1) |
| 303 | IndexType.showjudgearea | 공통 | PlayerConfig.isShowjudgearea() ? 1 : 0 | BUTTON_ASSIST_JUDGEAREA,OPTION_RESULT_B_1P | **없음(이미지 인덱스 공간 자체가 R-BMS에 없음, state.integer로 대체됨)** | 3회 [Root,Select] (script1,ref1,act1) |
| 304 | IndexType.assist_legacy | 공통 | (구) longnoteMode==1 ? 1 : 0 | BUTTON_ASSIST_LEGACY,OPTION_RESULT_C_1P | **없음(이미지 인덱스 공간 자체가 R-BMS에 없음, state.integer로 대체됨)** | 3회 [Root,Select] (script1,ref1,act1) |
| 305 | IndexType.markprocessednote | 공통 | isMarkprocessednote() ? 1 : 0 | BUTTON_ASSIST_MARKNOTE,OPTION_RESULT_D_1P | **없음(이미지 인덱스 공간 자체가 R-BMS에 없음, state.integer로 대체됨)** | 3회 [Root,Select] (script1,ref1,act1) |
| 306 | IndexType.bpmguide | 공통 | isBpmguide() ? 1 : 0 | BUTTON_ASSIST_BPMGUIDE,OPTION_RESULT_E_1P | **없음(이미지 인덱스 공간 자체가 R-BMS에 없음, state.integer로 대체됨)** | 3회 [Root,Select] (script1,ref1,act1) |
| 307 | IndexType.assist_nomine | 공통 | (구) mineMode==1 ? 1 : 0 | BUTTON_ASSIST_NOMINE,OPTION_RESULT_F_1P | **없음(이미지 인덱스 공간 자체가 R-BMS에 없음, state.integer로 대체됨)** | 3회 [Root,Select] (script1,ref1,act1) |
| 308 | IndexType.lnmode | 공통 | 플레이/MusicResult이고 곡에 LN이 있고 #LNMODE 정의가 있으면 0(LN),1(CN),2(HCN) 중 곡 정의, 아니면 PlayerConfig.lnmode | BUTTON_LNMODE,OPTION_RESULT_0_1P | **없음(이미지 인덱스 공간 자체가 R-BMS에 없음, state.integer로 대체됨)** | 9회 [Play,Root,Select] (ref5,script3,act1) |
| 321 | IndexType.autosave_replay1 | 공통 | PlayerConfig.getAutoSaveReplay()[0] | NUMBER_FOLDER_FAILED,BUTTON_AUTOSAVEREPLAY_1,OPTION_BEST_AA_1P | **없음(이미지 인덱스 공간 자체가 R-BMS에 없음, state.integer로 대체됨)** | 1회 [Select] (op1) |
| 322 | IndexType.autosave_replay2 | 공통 | [1] | NUMBER_FOLDER_ASSIST,BUTTON_AUTOSAVEREPLAY_2,OPTION_BEST_A_1P | **없음(이미지 인덱스 공간 자체가 R-BMS에 없음, state.integer로 대체됨)** | 1회 [Select] (other1) |
| 323 | IndexType.autosave_replay3 | 공통 | [2] | NUMBER_FOLDER_LASSIST,BUTTON_AUTOSAVEREPLAY_3,OPTION_BEST_B_1P | **없음(이미지 인덱스 공간 자체가 R-BMS에 없음, state.integer로 대체됨)** | 1회 [Select] (other1) |
| 324 | IndexType.autosave_replay4 | 공통 | [3] | NUMBER_FOLDER_EASY,BUTTON_AUTOSAVEREPLAY_4,OPTION_BEST_C_1P | **없음(이미지 인덱스 공간 자체가 R-BMS에 없음, state.integer로 대체됨)** | 1회 [Select] (other1) |
| 330 | IndexType.lanecover | 공통 | 선곡=선택바 PlayConfig, 그 외 현재 모드 PlayConfig.isEnablelanecover() ? 1 : 0 | NUMBER_FOLDER_MAX,BUTTON_LANECOVER,OPTION_UPDATE_SCORE | **없음(이미지 인덱스 공간 자체가 R-BMS에 없음, state.integer로 대체됨)** | 2회 [Select] (act1,ref1) |
| 331 | IndexType.lift | 공통 | enablelift | BUTTON_LIFT,OPTION_UPDATE_MAXCOMBO | **없음(이미지 인덱스 공간 자체가 R-BMS에 없음, state.integer로 대체됨)** | 2회 [Select] (act1,ref1) |
| 332 | IndexType.hidden | 공통 | enablehidden | BUTTON_HIDDEN,OPTION_UPDATE_MISSCOUNT | **없음(이미지 인덱스 공간 자체가 R-BMS에 없음, state.integer로 대체됨)** | 2회 [Select] (act1,ref1) |
| 340 | IndexType.judgealgorithm | 공통 | PlayConfig.judgetype 이름이 Combo/Duration/Lowest 중 몇 번째인지 0..2, 없으면 MIN_VALUE | BUTTON_JUDGEALGORITHM,OPTION_NOW_AAA_1P | **없음(이미지 인덱스 공간 자체가 R-BMS에 없음, state.integer로 대체됨)** | 2회 [Select] (act1,ref1) |
| 341 | IndexType.bottomshiftablegauge | 공통 | getBottomShiftableGauge() (0..2) | BUTTON_BOTTOMSIFTABLEFGAUGE,OPTION_NOW_AA_1P | **없음(이미지 인덱스 공간 자체가 R-BMS에 없음, state.integer로 대체됨)** | 2회 [Select] (act1,ref1) |
| 342 | IndexType.hispeedautoadjust | 공통 | PlayConfig.isEnableHispeedAutoAdjust() ? 1 : 0 | BUTTON_HISPEEDAUTOADJUST,OPTION_NOW_A_1P | **없음(이미지 인덱스 공간 자체가 R-BMS에 없음, state.integer로 대체됨)** | 2회 [Select] (act1,ref1) |
| 343 | IndexType.guidese | 공통 | isGuideSE() ? 1 : 0 | OPTION_NOW_B_1P | **없음(이미지 인덱스 공간 자체가 R-BMS에 없음, state.integer로 대체됨)** |  |
| 350 | IndexType.extranotedepth | 공통 | PlayerConfig.getExtranoteDepth() | NUMBER_TOTALNOTE_NORMAL,BUTTON_EXTRANOTE,OPTION_DISABLE_RESULTFLIP | **없음(이미지 인덱스 공간 자체가 R-BMS에 없음, state.integer로 대체됨)** |  |
| 351 | IndexType.minemode | 공통 | getMineMode() | NUMBER_TOTALNOTE_LN,BUTTON_MINEMODE,OPTION_ENABLE_RESULTFLIP | **없음(이미지 인덱스 공간 자체가 R-BMS에 없음, state.integer로 대체됨)** |  |
| 352 | IndexType.scrollmode | 공통 | getScrollMode() | NUMBER_TOTALNOTE_SCRATCH,BUTTON_SCROLLMODE,OPTION_1PWIN | **없음(이미지 인덱스 공간 자체가 R-BMS에 없음, state.integer로 대체됨)** |  |
| 353 | IndexType.longnotemode | 공통 | getLongnoteMode() | NUMBER_TOTALNOTE_BSS,BUTTON_LONGNOTEMODE,OPTION_2PWIN | **없음(이미지 인덱스 공간 자체가 R-BMS에 없음, state.integer로 대체됨)** |  |
| 360 | IndexType.seventonine_pattern | 공통 | getSevenToNinePattern() | NUMBER_DENSITY_PEAK,BUTTON_SEVENTONINE_PATTERN | **없음(이미지 인덱스 공간 자체가 R-BMS에 없음, state.integer로 대체됨)** |  |
| 361 | IndexType.seventonine_type | 공통 | getSevenToNineType() | NUMBER_DENSITY_PEAK_AFTERDOT,BUTTON_SEVENTONINE_TYPE,OPTION_3P_PERFECT | **없음(이미지 인덱스 공간 자체가 R-BMS에 없음, state.integer로 대체됨)** |  |
| 370 | IndexType.cleartype | 선곡,결과 | 선곡=선택바 score.clear(없으면 MIN_VALUE), 결과=newScore.clear | NUMBER_CLEAR,BUTTON_PRACTICE_ITEM1 | **없음(이미지 인덱스 공간 자체가 R-BMS에 없음, state.integer로 대체됨)** |  |
| 371 | IndexType.cleartype_target | 선곡,결과 | 선곡=선택바 rivalScore.clear, 결과=oldScore.clear | NUMBER_TARGET_CLEAR | **없음(이미지 인덱스 공간 자체가 R-BMS에 없음, state.integer로 대체됨)** |  |
| 380 | IndexType.playertype_ranking1 (패턴) | 선곡,결과 | RankingData.getPlayerType(index+rankingOffset), 없으면 MIN_VALUE | NUMBER_RANKING1_EXSCORE | **없음(이미지 인덱스 공간 자체가 R-BMS에 없음, state.integer로 대체됨)** |  |
| 381 | IndexType.playertype_ranking2 (패턴) | 선곡,결과 | RankingData.getPlayerType(index+rankingOffset), 없으면 MIN_VALUE | - | **없음(이미지 인덱스 공간 자체가 R-BMS에 없음, state.integer로 대체됨)** |  |
| 382 | IndexType.playertype_ranking3 (패턴) | 선곡,결과 | RankingData.getPlayerType(index+rankingOffset), 없으면 MIN_VALUE | - | **없음(이미지 인덱스 공간 자체가 R-BMS에 없음, state.integer로 대체됨)** |  |
| 383 | IndexType.playertype_ranking4 (패턴) | 선곡,결과 | RankingData.getPlayerType(index+rankingOffset), 없으면 MIN_VALUE | - | **없음(이미지 인덱스 공간 자체가 R-BMS에 없음, state.integer로 대체됨)** |  |
| 384 | IndexType.playertype_ranking5 (패턴) | 선곡,결과 | RankingData.getPlayerType(index+rankingOffset), 없으면 MIN_VALUE | - | **없음(이미지 인덱스 공간 자체가 R-BMS에 없음, state.integer로 대체됨)** |  |
| 385 | IndexType.playertype_ranking6 (패턴) | 선곡,결과 | RankingData.getPlayerType(index+rankingOffset), 없으면 MIN_VALUE | BUTTON_PRACTICE_ITEM16 | **없음(이미지 인덱스 공간 자체가 R-BMS에 없음, state.integer로 대체됨)** |  |
| 386 | IndexType.playertype_ranking7 (패턴) | 선곡,결과 | RankingData.getPlayerType(index+rankingOffset), 없으면 MIN_VALUE | BUTTON_SKINSELECT_24KEY | **없음(이미지 인덱스 공간 자체가 R-BMS에 없음, state.integer로 대체됨)** | 1회 [skinselect] (other1) |
| 387 | IndexType.playertype_ranking8 (패턴) | 선곡,결과 | RankingData.getPlayerType(index+rankingOffset), 없으면 MIN_VALUE | BUTTON_SKINSELECT_24KEY_DOUBLE | **없음(이미지 인덱스 공간 자체가 R-BMS에 없음, state.integer로 대체됨)** | 1회 [skinselect] (other1) |
| 388 | IndexType.playertype_ranking9 (패턴) | 선곡,결과 | RankingData.getPlayerType(index+rankingOffset), 없으면 MIN_VALUE | BUTTON_SKINSELECT_24KEY_BATTLE | **없음(이미지 인덱스 공간 자체가 R-BMS에 없음, state.integer로 대체됨)** | 1회 [skinselect] (other1) |
| 389 | IndexType.playertype_ranking10 (패턴) | 선곡,결과 | RankingData.getPlayerType(index+rankingOffset), 없으면 MIN_VALUE | NUMBER_RANKING10_EXSCORE | **없음(이미지 인덱스 공간 자체가 R-BMS에 없음, state.integer로 대체됨)** |  |
| 390 | IndexType.cleartype_ranking1 (패턴) | 선곡,결과 | RankingData.getScore(index+offset).clear.id, 없으면 MIN_VALUE | NUMBER_RANKING1_CLEAR | **없음(이미지 인덱스 공간 자체가 R-BMS에 없음, state.integer로 대체됨)** |  |
| 391 | IndexType.cleartype_ranking2 (패턴) | 선곡,결과 | RankingData.getScore(index+offset).clear.id, 없으면 MIN_VALUE | - | **없음(이미지 인덱스 공간 자체가 R-BMS에 없음, state.integer로 대체됨)** |  |
| 392 | IndexType.cleartype_ranking3 (패턴) | 선곡,결과 | RankingData.getScore(index+offset).clear.id, 없으면 MIN_VALUE | - | **없음(이미지 인덱스 공간 자체가 R-BMS에 없음, state.integer로 대체됨)** |  |
| 393 | IndexType.cleartype_ranking4 (패턴) | 선곡,결과 | RankingData.getScore(index+offset).clear.id, 없으면 MIN_VALUE | - | **없음(이미지 인덱스 공간 자체가 R-BMS에 없음, state.integer로 대체됨)** |  |
| 394 | IndexType.cleartype_ranking5 (패턴) | 선곡,결과 | RankingData.getScore(index+offset).clear.id, 없으면 MIN_VALUE | - | **없음(이미지 인덱스 공간 자체가 R-BMS에 없음, state.integer로 대체됨)** |  |
| 395 | IndexType.cleartype_ranking6 (패턴) | 선곡,결과 | RankingData.getScore(index+offset).clear.id, 없으면 MIN_VALUE | - | **없음(이미지 인덱스 공간 자체가 R-BMS에 없음, state.integer로 대체됨)** |  |
| 396 | IndexType.cleartype_ranking7 (패턴) | 선곡,결과 | RankingData.getScore(index+offset).clear.id, 없으면 MIN_VALUE | - | **없음(이미지 인덱스 공간 자체가 R-BMS에 없음, state.integer로 대체됨)** |  |
| 397 | IndexType.cleartype_ranking8 (패턴) | 선곡,결과 | RankingData.getScore(index+offset).clear.id, 없으면 MIN_VALUE | - | **없음(이미지 인덱스 공간 자체가 R-BMS에 없음, state.integer로 대체됨)** |  |
| 398 | IndexType.cleartype_ranking9 (패턴) | 선곡,결과 | RankingData.getScore(index+offset).clear.id, 없으면 MIN_VALUE | - | **없음(이미지 인덱스 공간 자체가 R-BMS에 없음, state.integer로 대체됨)** |  |
| 399 | IndexType.cleartype_ranking10 (패턴) | 선곡,결과 | RankingData.getScore(index+offset).clear.id, 없으면 MIN_VALUE | NUMBER_RANKING10_CLEAR | **없음(이미지 인덱스 공간 자체가 R-BMS에 없음, state.integer로 대체됨)** |  |
| 400 | IndexType.constant | 선곡,플레이 | PlayConfig.isEnableConstant() ? 1 : 0, 그 외 -1 | NUMBER_JUDGERANK,OPTION_CONSTANT | **없음(이미지 인덱스 공간 자체가 R-BMS에 없음, state.integer로 대체됨)** | 2회 [Select] (act1,ref1) |
| 450 | IndexType.pattern_1p_1 | 결과 | MusicResult에서 랜덤옵션(RANDOM/ROTATE/CROSS/RANDOM_EX)으로 1번 레인에 배정된 원래 레인 번호(1부터, 해당 없으면 0). 결과 외 0 | NUMBER_RANDOM_1P_1KEY | **없음(이미지 인덱스 공간 자체가 R-BMS에 없음, state.integer로 대체됨)** |  |
| 451 | IndexType.pattern_1p_2 | 결과 | MusicResult에서 랜덤옵션(RANDOM/ROTATE/CROSS/RANDOM_EX)으로 2번 레인에 배정된 원래 레인 번호(1부터, 해당 없으면 0). 결과 외 0 | NUMBER_RANDOM_1P_2KEY | **없음(이미지 인덱스 공간 자체가 R-BMS에 없음, state.integer로 대체됨)** |  |
| 452 | IndexType.pattern_1p_3 | 결과 | MusicResult에서 랜덤옵션(RANDOM/ROTATE/CROSS/RANDOM_EX)으로 3번 레인에 배정된 원래 레인 번호(1부터, 해당 없으면 0). 결과 외 0 | NUMBER_RANDOM_1P_3KEY | **없음(이미지 인덱스 공간 자체가 R-BMS에 없음, state.integer로 대체됨)** |  |
| 453 | IndexType.pattern_1p_4 | 결과 | MusicResult에서 랜덤옵션(RANDOM/ROTATE/CROSS/RANDOM_EX)으로 4번 레인에 배정된 원래 레인 번호(1부터, 해당 없으면 0). 결과 외 0 | NUMBER_RANDOM_1P_4KEY | **없음(이미지 인덱스 공간 자체가 R-BMS에 없음, state.integer로 대체됨)** |  |
| 454 | IndexType.pattern_1p_5 | 결과 | MusicResult에서 랜덤옵션(RANDOM/ROTATE/CROSS/RANDOM_EX)으로 5번 레인에 배정된 원래 레인 번호(1부터, 해당 없으면 0). 결과 외 0 | NUMBER_RANDOM_1P_5KEY | **없음(이미지 인덱스 공간 자체가 R-BMS에 없음, state.integer로 대체됨)** |  |
| 455 | IndexType.pattern_1p_6 | 결과 | MusicResult에서 랜덤옵션(RANDOM/ROTATE/CROSS/RANDOM_EX)으로 6번 레인에 배정된 원래 레인 번호(1부터, 해당 없으면 0). 결과 외 0 | NUMBER_RANDOM_1P_6KEY | **없음(이미지 인덱스 공간 자체가 R-BMS에 없음, state.integer로 대체됨)** |  |
| 456 | IndexType.pattern_1p_7 | 결과 | MusicResult에서 랜덤옵션(RANDOM/ROTATE/CROSS/RANDOM_EX)으로 7번 레인에 배정된 원래 레인 번호(1부터, 해당 없으면 0). 결과 외 0 | NUMBER_RANDOM_1P_7KEY | **없음(이미지 인덱스 공간 자체가 R-BMS에 없음, state.integer로 대체됨)** |  |
| 457 | IndexType.pattern_1p_8 | 결과 | MusicResult에서 랜덤옵션(RANDOM/ROTATE/CROSS/RANDOM_EX)으로 8번 레인에 배정된 원래 레인 번호(1부터, 해당 없으면 0). 결과 외 0 | NUMBER_RANDOM_1P_8KEY | **없음(이미지 인덱스 공간 자체가 R-BMS에 없음, state.integer로 대체됨)** |  |
| 458 | IndexType.pattern_1p_9 | 결과 | MusicResult에서 랜덤옵션(RANDOM/ROTATE/CROSS/RANDOM_EX)으로 9번 레인에 배정된 원래 레인 번호(1부터, 해당 없으면 0). 결과 외 0 | NUMBER_RANDOM_1P_9KEY | **없음(이미지 인덱스 공간 자체가 R-BMS에 없음, state.integer로 대체됨)** |  |
| 459 | IndexType.pattern_1p_SCR | 결과 | RANDOM_EX일 때 스크래치에 배정된 레인, 아니면 0 | NUMBER_RANDOM_1P_SCR | **없음(이미지 인덱스 공간 자체가 R-BMS에 없음, state.integer로 대체됨)** |  |
| 460 | IndexType.pattern_2p_1 | 결과 | 2P 쪽 같은 계산 (battle 아님이면 0) | NUMBER_RANDOM_2P_1KEY | **없음(이미지 인덱스 공간 자체가 R-BMS에 없음, state.integer로 대체됨)** |  |
| 461 | IndexType.pattern_2p_2 | 결과 | 2P 쪽 같은 계산 (battle 아님이면 0) | NUMBER_RANDOM_2P_2KEY | **없음(이미지 인덱스 공간 자체가 R-BMS에 없음, state.integer로 대체됨)** |  |
| 462 | IndexType.pattern_2p_3 | 결과 | 2P 쪽 같은 계산 (battle 아님이면 0) | NUMBER_RANDOM_2P_3KEY | **없음(이미지 인덱스 공간 자체가 R-BMS에 없음, state.integer로 대체됨)** |  |
| 463 | IndexType.pattern_2p_4 | 결과 | 2P 쪽 같은 계산 (battle 아님이면 0) | NUMBER_RANDOM_2P_4KEY | **없음(이미지 인덱스 공간 자체가 R-BMS에 없음, state.integer로 대체됨)** |  |
| 464 | IndexType.pattern_2p_5 | 결과 | 2P 쪽 같은 계산 (battle 아님이면 0) | NUMBER_RANDOM_2P_5KEY | **없음(이미지 인덱스 공간 자체가 R-BMS에 없음, state.integer로 대체됨)** |  |
| 465 | IndexType.pattern_2p_6 | 결과 | 2P 쪽 같은 계산 (battle 아님이면 0) | NUMBER_RANDOM_2P_6KEY | **없음(이미지 인덱스 공간 자체가 R-BMS에 없음, state.integer로 대체됨)** |  |
| 466 | IndexType.pattern_2p_7 | 결과 | 2P 쪽 같은 계산 (battle 아님이면 0) | NUMBER_RANDOM_2P_7KEY | **없음(이미지 인덱스 공간 자체가 R-BMS에 없음, state.integer로 대체됨)** |  |
| 469 | IndexType.pattern_2p_SCR | 결과 | 2P 스크래치 배정 | NUMBER_RANDOM_2P_SCR | **없음(이미지 인덱스 공간 자체가 R-BMS에 없음, state.integer로 대체됨)** |  |
| 500..519, 1510..1699 | VALUE_JUDGE_* 키별 판정 | 플레이 | 위 설명 | VALUE_JUDGE_1P_SCRATCH.. | 없음 | - |
| 170..185, 386..388 | BUTTON_SKINSELECT_* | 스킨설정 | skinType==해당 종류 ? 1 : 0 | BUTTON_SKINSELECT_* | 없음 | - |

## 10. 오프셋 (OFFSET_*)

저장소: `MainController.offset[OFFSET_MAX+1=200]` 각 `SkinOffset{x,y,w,h,r,a}`(MainController.java:124-137, `getOffset(i)`). 스킨 로드 시 `MainState.setSkin`이 스킨 헤더의 커스텀 오프셋 사용자 값(SkinConfig.Offset)을 이 배열에 덮어쓴다(MainState.java:134-150). 객체는 `setOffsetID`로 id 목록을 받는데 `0 < id < 200`인 것만 유효하다(SkinObject.java:834-846). 적용식(`relative`가 아닐 때, `SkinObject.prepareRegion` :380-420): `region.x += off.x - off.w/2; region.y += off.y - off.h/2; region.width += off.w; region.height += off.h;` 클립 영역도 같은 식(:445-455), 알파 `color.a += off.a/255`(:485,515), 각도 `angle += off.r`(:530-545). 여러 오프셋 id가 있으면 순서대로 모두 누적한다. `Lua main_state.offset(id)`는 `{x,y,w,h,r,a}` 테이블을 돌려준다. **OFFSET_ALL(10)**은 객체가 아니라 플레이 화면 전체 이동에 쓰인다(`Skin.getOffsetAll`: PLAY_5/7/9/10/14/24 종류에서만 `getOffsetValue(OFFSET_ALL)`, Skin.java:722-731).

| id | 상수 | 쓰는 쪽 / 갱신 식 | 단위 | R-BMS | ModernChic 사용 |
|---|---|---|---|---|---|
| 1 | OFFSET_SCRATCHANGLE_1P | KeyInputProccessor.java:92-106: 프레임마다 `scratch[0]` 누적(기본 -delta, 키 입력 시 방향 반전, 2160 주기) 후 `offset[1].r = scratch/6` (도, 0..360) | 각도 r | 쓰지 않음(어댑터에 없음) | 2회 [Play] (offset2) |
| 2 | OFFSET_SCRATCHANGLE_2P | 위와 같음 (`OFFSET_SCRATCHANGLE_1P + s`, s=1은 기본 +delta) | 각도 r | 쓰지 않음 | 1회 [Play] (offset1) |
| 3 | OFFSET_LIFT | LaneRenderer.java:332: `offset[3].y = hl - lanes[0].region.y` (lift 켜짐이면 lift*레인높이, 아니면 0) | px y | 구현: PlayViewState.cover_offset이 field.lift_height | 224회 [Play] (op117,offset107) |
| 4 | OFFSET_LANECOVER | LaneRenderer.java:333: `offset[4].y = (hl - hu) * lanecover` (음수, lanecover 비사용 시 0) | px y | 구현: `-height * shade.cover` | 10회 [Play] (offset10) |
| 5 | OFFSET_HIDDEN_COVER | LaneRenderer.java:335-345: hidden 켜짐이면 `a = 0`, `y = hidden*레인높이` (lift도 켜졌으면 (1-lift)*hidden*높이), 꺼짐이면 `a = -255` | px y, 알파 | 구현: shade.hidden 기준 y 또는 a=-255 |  |
| 10 | OFFSET_ALL | 사용자 설정(헤더 CustomOffset "All offset(%)")만. 플레이 화면에서 `Skin.getOffsetAll`로 전체 이동에 사용 | x,y,w,h | 사용자 nudge만(SkinCustomisation.offsets) |  |
| 30 | OFFSET_NOTES_1P | 사용자 설정만 (CustomOffset "Notes offset", 위치 y만 허용). 헤더에 선언되어야 객체에 연결 | y | 사용자 nudge만 | 2회 [Play] (offset2) |
| 32 | OFFSET_JUDGE_1P (=OFFSET_JUDGE_2P=OFFSET_JUDGE_3P) | 사용자 설정만 (CustomOffset "Judge offset"). 상수 값이 세 개 모두 32 | x,y,w,h,a | 사용자 nudge만 | 180회 [Play] (other90,offset90) |
| 33 | OFFSET_JUDGEDETAIL_1P (=2P=3P) | 사용자 설정만 (CustomOffset "Judge Detail offset"). 세 개 모두 33 | x,y,w,h,a | 사용자 nudge만 |  |
| 50 | OFFSET_LIFT_OBSOLETE | 폐기 예정. 갱신하는 코드 없음 | - | - |  |
| 51 | OFFSET_LANECOVER_OBSOLETE | 폐기 예정. 갱신하는 코드 없음 | - | - |  |
| 199 | OFFSET_MAX | 배열 상한(199). 40 이상은 스킨 정의 사용자 오프셋용으로 문서화됨 | - | - |  |

ModernChic 쪽 오프셋 정의(Root/mainoffset.lua) 중 beatoraja 상수에 없는 값은 14장의 "ModernChic 자체 id 정의" 목록을 본다.

## 11. 범위 매핑 (SkinPropertyMapper)

`SkinPropertyMapper.java` 전체 식. `player`는 0(1P)/1(2P), `key`는 0=스크래치, 1..9=건반, 10..99=확장 건반. 반환 -1은 범위 밖이다. 플레이어 인덱스 2(3P)는 타이머 계열에서 지원하지 않는다(`player<2`).

| 함수 | 식 | 비고 |
|---|---|---|
| bombTimerId(p,k) | k<10: `TIMER_BOMB_1P_SCRATCH(50) + k + p*10`; 10<=k<100: `TIMER_BOMB_1P_KEY10(1010) + (k-10) + p*100` | 1P: 50,51..59,1010..1099 / 2P: 60,61..69,1110..1199 |
| holdTimerId(p,k) | k<10: `TIMER_HOLD_1P_SCRATCH(70) + k + p*10`; 확장: `TIMER_HOLD_1P_KEY10(1210) + (k-10) + p*100` | 1P: 70,71..79,1210..1299 / 2P: 80,81..89,1310..1399 |
| hcnActiveTimerId(p,k) | k<10: `TIMER_HCN_ACTIVE_1P_SCRATCH(250) + k + p*10`; 확장: `TIMER_HCN_ACTIVE_1P_KEY10(1810) + (k-10) + p*100` | 1P: 250,251..259 / 2P: 260,261..269 (상수는 250,251만 선언), 확장 1810.., 2P 1910.. |
| hcnDamageTimerId(p,k) | k<10: `TIMER_HCN_DAMAGE_1P_SCRATCH(270) + k + p*10`; 확장: `TIMER_HCN_DAMAGE_1P_KEY10(2010) + (k-10) + p*100` | 1P: 270..279 / 2P: 280..289, 확장 2010.., 2110.. |
| keyOnTimerId(p,k) | k<10: `TIMER_KEYON_1P_SCRATCH(100) + k + p*10`; 확장: `TIMER_KEYON_1P_KEY10(1410) + (k-10) + p*100` | 1P: 100,101..109,1410..1499 / 2P: 110,111..119,1510..1599 |
| keyOffTimerId(p,k) | k<10: `TIMER_KEYOFF_1P_SCRATCH(120) + k + p*10`; 확장: `TIMER_KEYOFF_1P_KEY10(1610) + (k-10) + p*100` | 1P: 120..129,1610..1699 / 2P: 130..139,1710..1799 |
| keyJudgeValueId(p,k) | k<10: `VALUE_JUDGE_1P_SCRATCH(500) + k + p*10`; 확장: `VALUE_JUDGE_1P_KEY10(1510) + (k-10) + p*100` | 1P: 500..509,1510..1599 / 2P: 510..519,1610..1699 |
| getKeyJudgeValuePlayer(id) | 500<=id<=519: `(id-500)/10`; 그 외 `(id-1510)/100` | |
| getKeyJudgeValueOffset(id) | 500<=id<=519: `(id-500)%10`; 그 외 `(id-1510)%100 + 10` | |
| isSkinSelectTypeId(id) | `170<=id<=185` 또는 `386<=id<=388` | |
| getSkinSelectType(id) | 170..185: `SkinType.getSkinTypeById(id-170)`; 386..388: `getSkinTypeById(id-386+16)` | SkinType id: 0 PLAY_7KEYS,1 PLAY_5KEYS,2 PLAY_14KEYS,3 PLAY_10KEYS,4 PLAY_9KEYS,5 MUSIC_SELECT,6 DECIDE,7 RESULT,8 KEY_CONFIG,9 SKIN_SELECT,10 SOUND_SET,11 THEME,12 PLAY_7KEYS_BATTLE,13 PLAY_5KEYS_BATTLE,14 PLAY_9KEYS_BATTLE,15 COURSE_RESULT,16 PLAY_24KEYS,17 PLAY_24KEYS_DOUBLE,18 PLAY_24KEYS_BATTLE |
| skinSelectTypeId(type) | type.id<=15: `170 + id`; 그 외 `386 + id - 16` | |
| isSkinCustomizeButton(id) | `220 <= id < 229` (**229 제외 결함**) | getSkinCustomizeIndex = id-220 |
| isSkinCustomizeCategory(id) | `100 <= id <= 109` | index = id-100 |
| isSkinCustomizeItem(id) | `110 <= id <= 119` | index = id-110 |
| isCustomEventId(id) | `1000 <= id <= 1999` | isEventRunnableBySkin: 항상 true |
| isCustomTimerId(id) | `10000 <= id <= 19999` | isTimerWritableBySkin = isCustomTimerId |
| 레인 -> (player, offset) | `LaneProperty.getLanePlayer()[lane]`, `getLaneSkinOffset()[lane]` (모드별 표) | JudgeManager.LaneState, KeyInputProccessor가 위 함수에 넣어 타이머 id 산출. 스크래치 레인의 offset은 0, 건반은 1부터 |

## 12. 슬라이더(SLIDER_*)와 그래프(BARGRAPH_*) type 매핑

둘 다 RateType 공간이다(위 5-A). `SkinSlider(type, changeable)`는 `getRateProperty(type)`로 읽고 `changeable`이면 `getRateWriter(type)`로 마우스 드래그 값을 쓴다. 쓰기가 되는 RateType은 1(선곡 위치), 7(스킨선택 위치), 8(랭킹 위치), 17/18/19(볼륨), 20(연습 위치)뿐이고 나머지는 읽기 전용이다. 슬라이더 이동량은 `currentValue*range` px(방향 angle 0 위/1 오른/2 아래/3 왼), 값은 클램프되지 않는다(SkinSlider.java:113-119). 그래프 type이 음수면 분포 그래프(-1: 11칸 `SkinDistributionGraph(0)`, 그 외 음수: 28칸 `SkinDistributionGraph(1)`)로 분기한다(JsonSkinObjectLoader.java:419-436).

| 별칭 상수 | id | RateType 이름 |
|---|---|---|
| SLIDER_MUSICSELECT_POSITION | 1 | musicselect_position |
| SLIDER_MUSIC_PROGRESS | 6 | music_progress |
| SLIDER_SKINSELECT_POSITION | 7 | skinselect_position |
| SLIDER_MASTER_VOLUME | 17 | mastervolume |
| SLIDER_KEY_VOLUME | 18 | keyvolume |
| SLIDER_BGM_VOLUME | 19 | bgmvolume |
| SLIDER_PRACTICE_POSITION | 20 | practice_position |
| SLIDER_LANECOVER | 4 | lanecover |
| SLIDER_LANECOVER2 | 5 | lanecover2 |
| BARGRAPH_MUSIC_PROGRESS | 101 | music_progress_bar |
| BARGRAPH_LOAD_PROGRESS | 102 | load_progress |
| BARGRAPH_LEVEL | 103 | level |
| BARGRAPH_LEVEL_BEGINNER | 105 | level_beginner |
| BARGRAPH_LEVEL_NORMAL | 106 | level_normal |
| BARGRAPH_LEVEL_HYPER | 107 | level_hyper |
| BARGRAPH_LEVEL_ANOTHER | 108 | level_another |
| BARGRAPH_LEVEL_INSANE | 109 | level_insane |
| BARGRAPH_SCORERATE | 110 | scorerate |
| BARGRAPH_SCORERATE_FINAL | 111 | scorerate_final |
| BARGRAPH_BESTSCORERATE_NOW | 112 | bestscorerate_now |
| BARGRAPH_BESTSCORERATE | 113 | bestscorerate |
| BARGRAPH_TARGETSCORERATE_NOW | 114 | targetscorerate_now |
| BARGRAPH_TARGETSCORERATE | 115 | targetscorerate |
| BARGRAPH_RATE_PGREAT | 140 | rate_pgreat |
| BARGRAPH_RATE_GREAT | 141 | rate_great |
| BARGRAPH_RATE_GOOD | 142 | rate_good |
| BARGRAPH_RATE_BAD | 143 | rate_bad |
| BARGRAPH_RATE_POOR | 144 | rate_poor |
| BARGRAPH_RATE_MAXCOMBO | 145 | rate_maxcombo |
| BARGRAPH_RATE_SCORE | 146 | (RateType 없음) |
| BARGRAPH_RATE_EXSCORE | 147 | rate_exscore |

## 13. 기타 상수 (IMAGE_*, VALUE_*, OPTION 특수값)

| 상수 | id | 의미/사용 |
|---|---|---|
| IMAGE_STAGEFILE | 100 | `SkinSourceReference(100)`: `BMSResource.getStagefile()` 텍스처. JSON destination의 id가 **음수 정수 문자열**(예 "-100")이면 `new SkinImage(-id)`로 이 참조 이미지 객체가 만들어진다(JSONSkinLoader.java:316-319). ModernChic는 `MAIN.IMAGE.STAGEFILE = -100` |
| IMAGE_BACKBMP | 101 | `getBackbmp()` |
| IMAGE_BANNER | 102 | `getBanner()` |
| IMAGE_BLACK | 110 | 1x1 검정 `TextureRegion` |
| IMAGE_WHITE | 111 | 1x1 흰색 |
| (ModernChic -105) | 105 | `MAIN.IMAGE.SKINTHUMBNAIL = -105`: `SkinSourceReference`에 105 분기가 없어 `null` 이미지(그려지지 않음) |
| OPTION_RANDOM_VALUE | -1 | 조회 시 id 음수 규칙에 걸림. 상수만 있고 조회 의미 없음 |
| EVENT_CUSTOM_BEGIN/END | 1000/1999 | 스킨 정의 커스텀 이벤트 구간 |
| TIMER_CUSTOM_BEGIN/END | 10000/19999 | 스킨 정의 커스텀 타이머 구간 |

## 14. ModernChic 자체 id 정의 중 SkinProperty 상수에 없는 값

ModernChic는 `Root/main*.lua`에 id 표를 직접 복제해 쓴다(`MAIN.OP/NUM/TIMER/STRING/BUTTON/OFFSET/GRAPH/SLIDER/IMAGE`). 아래는 각 표의 값 중 beatoraja `SkinProperty.java`의 같은 종류 상수에 없는 것(= R-BMS 생성 표에도 없는 id)이다.

| 표 | 이름 | 값 | 사용 | beatoraja에서 이 id가 유효한가 |
|---|---|---|---|---|
| OP | COURSE_STAGE5 | 284 | 0회 | **무효(상수도 패턴도 아님: beatoraja에서 null/항상 off)** |
| OP | COURSE_STAGE6 | 285 | 0회 | **무효(상수도 패턴도 아님: beatoraja에서 null/항상 off)** |
| OP | COURSE_STAGE7 | 286 | 0회 | **무효(상수도 패턴도 아님: beatoraja에서 null/항상 off)** |
| OP | COURSE_STAGE8 | 287 | 0회 | **무효(상수도 패턴도 아님: beatoraja에서 null/항상 off)** |
| OP | COURSE_STAGE9 | 288 | 0회 | **무효(상수도 패턴도 아님: beatoraja에서 null/항상 off)** |
| NUM | RANKING2_EXSCORE | 381 | 3회 | 유효(패턴/범위/식으로 해석됨) |
| NUM | RANKING3_EXSCORE | 382 | 3회 | 유효(패턴/범위/식으로 해석됨) |
| NUM | RANKING4_EXSCORE | 383 | 3회 | 유효(패턴/범위/식으로 해석됨) |
| NUM | RANKING5_EXSCORE | 384 | 3회 | 유효(패턴/범위/식으로 해석됨) |
| NUM | RANKING6_EXSCORE | 385 | 3회 | 유효(패턴/범위/식으로 해석됨) |
| NUM | RANKING7_EXSCORE | 386 | 3회 | 유효(패턴/범위/식으로 해석됨) |
| NUM | RANKING8_EXSCORE | 387 | 3회 | 유효(패턴/범위/식으로 해석됨) |
| NUM | RANKING9_EXSCORE | 388 | 3회 | 유효(패턴/범위/식으로 해석됨) |
| NUM | RANKING2_CLEAR | 391 | 6회 | 유효(패턴/범위/식으로 해석됨) |
| NUM | RANKING3_CLEAR | 392 | 6회 | 유효(패턴/범위/식으로 해석됨) |
| NUM | RANKING4_CLEAR | 393 | 6회 | 유효(패턴/범위/식으로 해석됨) |
| NUM | RANKING5_CLEAR | 394 | 6회 | 유효(패턴/범위/식으로 해석됨) |
| NUM | RANKING6_CLEAR | 395 | 6회 | 유효(패턴/범위/식으로 해석됨) |
| NUM | RANKING7_CLEAR | 396 | 6회 | 유효(패턴/범위/식으로 해석됨) |
| NUM | RANKING8_CLEAR | 397 | 6회 | 유효(패턴/범위/식으로 해석됨) |
| NUM | RANKING9_CLEAR | 398 | 6회 | 유효(패턴/범위/식으로 해석됨) |
| NUM | RANKING2_INDEX | 391 | 6회 | 유효(패턴/범위/식으로 해석됨) |
| NUM | RANKING3_INDEX | 392 | 6회 | 유효(패턴/범위/식으로 해석됨) |
| NUM | RANKING4_INDEX | 393 | 6회 | 유효(패턴/범위/식으로 해석됨) |
| NUM | RANKING5_INDEX | 394 | 6회 | 유효(패턴/범위/식으로 해석됨) |
| NUM | RANKING6_INDEX | 395 | 6회 | 유효(패턴/범위/식으로 해석됨) |
| NUM | RANKING7_INDEX | 396 | 6회 | 유효(패턴/범위/식으로 해석됨) |
| NUM | RANKING8_INDEX | 397 | 6회 | 유효(패턴/범위/식으로 해석됨) |
| NUM | RANKING9_INDEX | 398 | 6회 | 유효(패턴/범위/식으로 해석됨) |
| NUM | JUDGE_1P_DURATION | 525 | 2회 | 유효(패턴/범위/식으로 해석됨) |
| NUM | JUDGE_2P_DURATION | 526 | 1회 | 유효(패턴/범위/식으로 해석됨) |
| NUM | JUDGE_3P_DURATION | 527 | 0회 | 유효(패턴/범위/식으로 해석됨) |
| TIMER | HOLD_1P_KEY2 | 72 | 6회 | 유효(패턴/범위/식으로 해석됨) |
| TIMER | HOLD_1P_KEY3 | 73 | 6회 | 유효(패턴/범위/식으로 해석됨) |
| TIMER | HOLD_1P_KEY4 | 74 | 6회 | 유효(패턴/범위/식으로 해석됨) |
| TIMER | HOLD_1P_KEY5 | 75 | 6회 | 유효(패턴/범위/식으로 해석됨) |
| TIMER | HOLD_1P_KEY6 | 76 | 3회 | 유효(패턴/범위/식으로 해석됨) |
| TIMER | HOLD_1P_KEY7 | 77 | 3회 | 유효(패턴/범위/식으로 해석됨) |
| TIMER | HOLD_1P_KEY8 | 78 | 0회 | 유효(패턴/범위/식으로 해석됨) |
| TIMER | HOLD_1P_KEY9 | 79 | 0회 | 유효(패턴/범위/식으로 해석됨) |
| TIMER | HOLD_2P_KEY2 | 82 | 4회 | 유효(패턴/범위/식으로 해석됨) |
| TIMER | HOLD_2P_KEY3 | 83 | 4회 | 유효(패턴/범위/식으로 해석됨) |
| TIMER | HOLD_2P_KEY4 | 84 | 4회 | 유효(패턴/범위/식으로 해석됨) |
| TIMER | HOLD_2P_KEY5 | 85 | 4회 | 유효(패턴/범위/식으로 해석됨) |
| TIMER | HOLD_2P_KEY6 | 86 | 2회 | 유효(패턴/범위/식으로 해석됨) |
| TIMER | HOLD_2P_KEY7 | 87 | 2회 | 유효(패턴/범위/식으로 해석됨) |
| TIMER | HOLD_2P_KEY8 | 88 | 0회 | 유효(패턴/범위/식으로 해석됨) |
| TIMER | HOLD_2P_KEY9 | 89 | 0회 | 유효(패턴/범위/식으로 해석됨) |
| TIMER | PREVIEW | 141 | 8회 | **무효(상수도 패턴도 아님: beatoraja에서 null/항상 off)** |
| STRING | SELECTED_TARGET | 3 | 2회 | 유효(패턴/범위/식으로 해석됨) |
| STRING | SKIN_CUSTOMIZE_CATEGORY2 | 101 | 1회 | 유효(패턴/범위/식으로 해석됨) |
| STRING | SKIN_CUSTOMIZE_CATEGORY3 | 102 | 1회 | 유효(패턴/범위/식으로 해석됨) |
| STRING | SKIN_CUSTOMIZE_CATEGORY4 | 103 | 1회 | 유효(패턴/범위/식으로 해석됨) |
| STRING | SKIN_CUSTOMIZE_CATEGORY5 | 104 | 1회 | 유효(패턴/범위/식으로 해석됨) |
| STRING | SKIN_CUSTOMIZE_CATEGORY6 | 105 | 1회 | 유효(패턴/범위/식으로 해석됨) |
| STRING | SKIN_CUSTOMIZE_CATEGORY7 | 106 | 1회 | 유효(패턴/범위/식으로 해석됨) |
| STRING | SKIN_CUSTOMIZE_CATEGORY8 | 107 | 1회 | 유효(패턴/범위/식으로 해석됨) |
| STRING | SKIN_CUSTOMIZE_CATEGORY9 | 108 | 1회 | 유효(패턴/범위/식으로 해석됨) |
| STRING | SKIN_CUSTOMIZE_ITEM2 | 111 | 1회 | 유효(패턴/범위/식으로 해석됨) |
| STRING | SKIN_CUSTOMIZE_ITEM3 | 112 | 1회 | 유효(패턴/범위/식으로 해석됨) |
| STRING | SKIN_CUSTOMIZE_ITEM4 | 113 | 1회 | 유효(패턴/범위/식으로 해석됨) |
| STRING | SKIN_CUSTOMIZE_ITEM5 | 114 | 1회 | 유효(패턴/범위/식으로 해석됨) |
| STRING | SKIN_CUSTOMIZE_ITEM6 | 115 | 1회 | 유효(패턴/범위/식으로 해석됨) |
| STRING | SKIN_CUSTOMIZE_ITEM7 | 116 | 1회 | 유효(패턴/범위/식으로 해석됨) |
| STRING | SKIN_CUSTOMIZE_ITEM8 | 117 | 1회 | 유효(패턴/범위/식으로 해석됨) |
| STRING | SKIN_CUSTOMIZE_ITEM9 | 118 | 1회 | 유효(패턴/범위/식으로 해석됨) |
| STRING | RANKING2_NAME | 121 | 0회 | 유효(패턴/범위/식으로 해석됨) |
| STRING | RANKING3_NAME | 122 | 0회 | 유효(패턴/범위/식으로 해석됨) |
| STRING | RANKING4_NAME | 123 | 0회 | 유효(패턴/범위/식으로 해석됨) |
| STRING | RANKING5_NAME | 124 | 0회 | 유효(패턴/범위/식으로 해석됨) |
| STRING | RANKING6_NAME | 125 | 0회 | 유효(패턴/범위/식으로 해석됨) |
| STRING | RANKING7_NAME | 126 | 0회 | 유효(패턴/범위/식으로 해석됨) |
| STRING | RANKING8_NAME | 127 | 0회 | 유효(패턴/범위/식으로 해석됨) |
| STRING | RANKING9_NAME | 128 | 0회 | 유효(패턴/범위/식으로 해석됨) |
| STRING | TARGET_FORWARD1 | 200 | 1회 | 유효(패턴/범위/식으로 해석됨) |
| STRING | TARGET_FORWARD2 | 201 | 1회 | 유효(패턴/범위/식으로 해석됨) |
| STRING | TARGET_FORWARD3 | 202 | 1회 | 유효(패턴/범위/식으로 해석됨) |
| STRING | TARGET_FORWARD4 | 203 | 1회 | 유효(패턴/범위/식으로 해석됨) |
| STRING | TARGET_FORWARD5 | 204 | 1회 | 유효(패턴/범위/식으로 해석됨) |
| STRING | TARGET_FORWARD6 | 205 | 1회 | 유효(패턴/범위/식으로 해석됨) |
| STRING | TARGET_FORWARD7 | 206 | 1회 | 유효(패턴/범위/식으로 해석됨) |
| STRING | TARGET_FORWARD8 | 207 | 1회 | 유효(패턴/범위/식으로 해석됨) |
| STRING | TARGET_FORWARD9 | 208 | 1회 | 유효(패턴/범위/식으로 해석됨) |
| STRING | TARGET_FORWARD10 | 209 | 1회 | 유효(패턴/범위/식으로 해석됨) |
| STRING | TARGET_BACKWARD1 | 210 | 1회 | 유효(패턴/범위/식으로 해석됨) |
| STRING | TARGET_BACKWARD2 | 211 | 1회 | 유효(패턴/범위/식으로 해석됨) |
| STRING | TARGET_BACKWARD3 | 212 | 1회 | 유효(패턴/범위/식으로 해석됨) |
| STRING | TARGET_BACKWARD4 | 213 | 1회 | 유효(패턴/범위/식으로 해석됨) |
| STRING | TARGET_BACKWARD5 | 214 | 1회 | 유효(패턴/범위/식으로 해석됨) |
| STRING | TARGET_BACKWARD6 | 215 | 1회 | 유효(패턴/범위/식으로 해석됨) |
| STRING | TARGET_BACKWARD7 | 216 | 1회 | 유효(패턴/범위/식으로 해석됨) |
| STRING | TARGET_BACKWARD8 | 217 | 1회 | 유효(패턴/범위/식으로 해석됨) |
| STRING | TARGET_BACKWARD9 | 218 | 1회 | 유효(패턴/범위/식으로 해석됨) |
| STRING | TARGET_BACKWARD10 | 219 | 1회 | 유효(패턴/범위/식으로 해석됨) |
| BUTTON | JUDGE_TIMING_AUTO_ADJUST | 75 | 1회 | 유효(패턴/범위/식으로 해석됨) |
| BUTTON | SKIN_CUSTOMIZE2 | 221 | 1회 | 유효(패턴/범위/식으로 해석됨) |
| BUTTON | SKIN_CUSTOMIZE3 | 222 | 1회 | 유효(패턴/범위/식으로 해석됨) |
| BUTTON | SKIN_CUSTOMIZE4 | 223 | 1회 | 유효(패턴/범위/식으로 해석됨) |
| BUTTON | SKIN_CUSTOMIZE5 | 224 | 1회 | 유효(패턴/범위/식으로 해석됨) |
| BUTTON | SKIN_CUSTOMIZE6 | 225 | 1회 | 유효(패턴/범위/식으로 해석됨) |
| BUTTON | SKIN_CUSTOMIZE7 | 226 | 1회 | 유효(패턴/범위/식으로 해석됨) |
| BUTTON | SKIN_CUSTOMIZE8 | 227 | 1회 | 유효(패턴/범위/식으로 해석됨) |
| BUTTON | SKIN_CUSTOMIZE9 | 228 | 1회 | 유효(패턴/범위/식으로 해석됨) |
| BUTTON | CONSTANT | 400 | 2회 | 유효(패턴/범위/식으로 해석됨) |

---

## 15. ModernChic 사용 id 와 R-BMS 격차

집계 방법: `Root/main{option,number,timer,string,button,offset,graph,slider,image}.lua`가 정의한 `MAIN.OP/NUM/TIMER/STRING/BUTTON/OFFSET/GRAPH/SLIDER/IMAGE` 이름을 id로 환산하고, 전체 Lua(126개, `.luaskin` 포함)에서 `MAIN.<표>.<이름>` 참조 횟수를 센다. 상태 열: `beatoraja 구현` = 3~9장 팩토리 구현표에 id가 있음, `R-BMS 값 채움` = 어댑터가 응답(타이머는 앱이 켜는 id), `등록만` = MAPPINGS 라우팅만 있음, `없음` = 라우팅도 없음. 역할 분류가 필요한 BUTTON은 `ref`(이미지 인덱스), `act`(클릭 이벤트), `script`(`event_index` 호출)로 나눴다.

핵심 수치(아래 첫 표): OP 139종 중 23종, NUM 177종 중 30종, STRING 60종 중 7종, GRAPH 13종 중 9종, SLIDER 7종 중 1종만 R-BMS가 값을 채운다. 구현 순서를 정할 때는 각 표를 사용 횟수 내림차순으로 위에서부터 채우면 ModernChic 화면 대부분이 먼저 맞춰진다. 빈 칸이 아닌 "무값" 처리(1장 7번)를 도입하면 IR/라이벌/날짜 계열 수십 개 id가 `MIN_VALUE` 한 가지로 beatoraja와 같은 결과(미표시 + Lua `== -2147483648` 분기)를 낸다.

| 종류 | 사용 id 수 | beatoraja가 구현한 id | R-BMS가 값을 채우는 id | R-BMS 등록만 | R-BMS 없음 |
|---|---|---|---|---|---|
| MAIN.OP | 139 | 138 | 23 | 84 | 32 |
| MAIN.NUM | 177 | 177 | 30 | 14 | 133 |
| MAIN.STRING | 60 | 60 | 7 | 30 | 23 |
| MAIN.GRAPH | 13 | 13 | 9 | 4 | 0 |
| MAIN.SLIDER | 7 | 7 | 1 | 6 | 0 |
| MAIN.TIMER | 88 | 87 | 75 | 12 | 1 |

### MAIN.OP 중 R-BMS가 값을 채우지 않는 사용 id (사용 횟수 내림차순)

| id | ModernChic 이름 | 사용 | beatoraja 구현 | R-BMS | 사용 위치 |
|---|---|---|---|---|---|
| 3 | GRADEBAR | 35 | 구현 | 등록만 | Root,Select |
| 51 | ONLINE | 30 | 구현 | 등록만 | Select,course,result |
| 270 | LANECOVER1_CHANGING | 22 | 구현 | 등록만 | Play |
| 625 | COMPARE_RIVAL | 21 | 구현 | 없음 | Root,Select |
| 2244 | BAD_EXIST | 20 | 구현 | 등록만 | Play |
| 2245 | POOR_EXIST | 18 | 구현 | 등록만 | Play |
| 177 | BPMCHANGE | 17 | 구현 | 등록만 | Play,Select |
| 2243 | GOOD_EXIST | 16 | 구현 | 등록만 | Play |
| 170 | NO_BGA | 16 | 구현 | 등록만 | Play,Select |
| 400 | CONSTANT | 14 | 구현 | 등록만 | Play |
| 1080 | STATE_PRACTICE | 12 | 구현 | 없음 | Play |
| 606 | IR_WAITING | 12 | 구현 | 등록만 | Result,Select |
| 100 | SELECT_BAR_NOT_PLAYED | 8 | 구현 | 등록만 | Select |
| 155 | DIFFICULTY5 | 7 | 구현 | 없음 | Decide,Play,Result,Root,decide |
| 154 | DIFFICULTY4 | 7 | 구현 | 없음 | Decide,Play,Result,Root,decide |
| 153 | DIFFICULTY3 | 7 | 구현 | 없음 | Decide,Play,Result,Root,decide |
| 152 | DIFFICULTY2 | 7 | 구현 | 없음 | Decide,Play,Result,Root,decide |
| 151 | DIFFICULTY1 | 7 | 구현 | 없음 | Decide,Play,Result,Root,decide |
| 150 | DIFFICULTY0 | 7 | 구현 | 없음 | Decide,Play,Result,Root,decide |
| 1203 | REPLAYDATA4 | 5 | 구현 | 없음 | Result,Root,Select |
| 1200 | REPLAYDATA3 | 5 | 구현 | 없음 | Result,Root,Select |
| 1197 | REPLAYDATA2 | 5 | 구현 | 없음 | Result,Root,Select |
| 232 | GAUGE_1P_20_29 | 5 | 구현 | 등록만 | Play |
| 231 | GAUGE_1P_10_19 | 5 | 구현 | 등록만 | Play |
| 230 | GAUGE_1P_0_9 | 5 | 구현 | 등록만 | Play |
| 197 | REPLAYDATA | 5 | 구현 | 없음 | Result,Root,Select |
| 176 | NO_BPMCHANGE | 5 | 구현 | 등록만 | Play,Select |
| 173 | LN | 5 | 구현 | 등록만 | Play,Root |
| 1104 | SELECT_BAR_MAX_CLEARED | 4 | 구현 | 등록만 | Select |
| 1103 | SELECT_BAR_PERFECT_CLEARED | 4 | 구현 | 등록만 | Select |
| 1102 | SELECT_BAR_EXHARD_CLEARED | 4 | 구현 | 등록만 | Select |
| 1101 | SELECT_BAR_LIGHT_ASSIST_EASY_CLEARED | 4 | 구현 | 등록만 | Select |
| 1100 | SELECT_BAR_ASSIST_EASY_CLEARED | 4 | 구현 | 등록만 | Select |
| 222 | A | 4 | 구현 | 등록만 | Play,Root |
| 221 | AA | 4 | 구현 | 등록만 | Play,Root |
| 220 | AAA | 4 | 구현 | 등록만 | Play,Root |
| 194 | NO_BACKBMP | 4 | 구현 | 등록만 | Play |
| 191 | STAGEFILE | 4 | 구현 | 등록만 | Play,Result |
| 184 | JUDGE_VERYEASY | 4 | 구현 | 등록만 | Play,Result,Root |
| 183 | JUDGE_EASY | 4 | 구현 | 등록만 | Play,Result,Root |
| 182 | JUDGE_NORMAL | 4 | 구현 | 등록만 | Play,Result,Root |
| 181 | JUDGE_HARD | 4 | 구현 | 등록만 | Play,Result,Root |
| 180 | JUDGE_VERYHARD | 4 | 구현 | 등록만 | Play,Result,Root |
| 162 | SONG14KEY | 4 | 구현 | 없음 | Result,Select |
| 105 | SELECT_BAR_FULL_COMBO_CLEARED | 4 | 구현 | 등록만 | Select |
| 104 | SELECT_BAR_HARD_CLEARED | 4 | 구현 | 등록만 | Select |
| 103 | SELECT_BAR_NORMAL_CLEARED | 4 | 구현 | 등록만 | Select |
| 102 | SELECT_BAR_EASY_CLEARED | 4 | 구현 | 등록만 | Select |
| 101 | SELECT_BAR_FAILED | 4 | 구현 | 등록만 | Select |
| 1161 | SONG24KEYDP | 3 | 구현 | 없음 | Result,Select |
| 290 | MODE_COURSE | 3 | 구현 | 등록만 | Play,Result |
| 272 | LIFT1_ON | 3 | 구현 | 등록만 | Root |
| 207 | F_1P | 3 | 구현 | 등록만 | Play,Select |
| 206 | E_1P | 3 | 구현 | 등록만 | Play,Select |
| 205 | D_1P | 3 | 구현 | 등록만 | Play,Select |
| 204 | C_1P | 3 | 구현 | 등록만 | Play,Select |
| 203 | B_1P | 3 | 구현 | 등록만 | Play,Select |
| 202 | A_1P | 3 | 구현 | 등록만 | Play,Select |
| 201 | AA_1P | 3 | 구현 | 등록만 | Play,Select |
| 200 | AAA_1P | 3 | 구현 | 등록만 | Play,Select |
| 163 | SONG10KEY | 3 | 구현 | 없음 | Result,Select |
| 84 | REPLAY_PLAYING | 3 | 구현 | 없음 | Play |
| 50 | OFFLINE | 3 | 구현 | 등록만 | Select |
| 1208 | SELECT_REPLAYDATA4 | 2 | 구현 | 없음 | Select |
| 1207 | SELECT_REPLAYDATA3 | 2 | 구현 | 없음 | Select |
| 1206 | SELECT_REPLAYDATA2 | 2 | 구현 | 없음 | Select |
| 1205 | SELECT_REPLAYDATA | 2 | 구현 | 없음 | Select |
| 1160 | SONG24KEY | 2 | 구현 | 없음 | Result,Select |
| 353 | WIN_2P | 2 | 구현 | 등록만 | Select |
| 352 | WIN_1P | 2 | 구현 | 등록만 | Select |
| 330 | UPDATE_SCORE | 2 | 구현 | 등록만 | Result,Root |
| 289 | COURSE_STAGE_FINAL | 2 | 구현 | 등록만 | Result |
| 271 | LANECOVER1_ON | 2 | 구현 | 등록만 | Play |
| 240 | GAUGE_1P_100 | 2 | 구현 | 등록만 | Play |
| 195 | BACKBMP | 2 | 구현 | 등록만 | Play |
| 175 | TEXT | 2 | 구현 | 등록만 | Select |
| 172 | NO_LN | 2 | 구현 | 등록만 | Play |
| 171 | BGA | 2 | 구현 | 등록만 | Play,Select |
| 164 | SONG9KEY | 2 | 구현 | 없음 | Result,Select |
| 161 | SONG5KEY | 2 | 구현 | 없음 | Result,Select |
| 160 | SONG7KEY | 2 | 구현 | 없음 | Result,Select |
| 5 | PLAYABLEBAR | 2 | 구현 | 등록만 | Select |
| 1204 | REPLAYDATA4_SAVED | 1 | 구현 | 없음 | Result |
| 1202 | NO_REPLAYDATA4 | 1 | 구현 | 없음 | Result |
| 1201 | REPLAYDATA3_SAVED | 1 | 구현 | 없음 | Result |
| 1199 | NO_REPLAYDATA3 | 1 | 구현 | 없음 | Result |
| 1198 | REPLAYDATA2_SAVED | 1 | 구현 | 없음 | Result |
| 1196 | NO_REPLAYDATA2 | 1 | 구현 | 없음 | Result |
| 1030 | RANDOMSELECTBAR | 1 | 구현 | 등록만 | Select |
| 1017 | GRADEBAR_HCN | 1 | 구현 | 등록만 | Select |
| 1016 | GRADEBAR_CN | 1 | 구현 | 등록만 | Select |
| 1015 | GRADEBAR_LN | 1 | 구현 | 등록만 | Select |
| 1014 | GRADEBAR_GAUGE_24KEYS | 1 | 구현 | 등록만 | Select |
| 1013 | GRADEBAR_GAUGE_9KEYS | 1 | 구현 | 등록만 | Select |
| 1012 | GRADEBAR_GAUGE_7KEYS | 1 | 구현 | 등록만 | Select |
| 1011 | GRADEBAR_GAUGE_5KEYS | 1 | 구현 | 등록만 | Select |
| 1010 | GRADEBAR_GAUGE_LR2 | 1 | 구현 | 등록만 | Select |
| 1005 | GRADEBAR_NOSPEED | 1 | 구현 | 등록만 | Select |
| 1004 | GRADEBAR_RANDOM | 1 | 구현 | 등록만 | Select |
| 1003 | GRADEBAR_MIRROR | 1 | 구현 | 등록만 | Select |
| 602 | IR_LOADED | 1 | **구현 없음(사용자 옵션 id 가능성)** | 등록만 | Select |
| 332 | UPDATE_MISSCOUNT | 1 | 구현 | 등록만 | Result |
| 331 | UPDATE_MAXCOMBO | 1 | 구현 | 등록만 | Result |
| 327 | BEST_F_1P | 1 | 구현 | 등록만 | Root |
| 326 | BEST_E_1P | 1 | 구현 | 등록만 | Root |
| 325 | BEST_D_1P | 1 | 구현 | 등록만 | Root |
| 324 | BEST_C_1P | 1 | 구현 | 등록만 | Root |
| 323 | BEST_B_1P | 1 | 구현 | 등록만 | Root |
| 322 | BEST_A_1P | 1 | 구현 | 등록만 | Root |
| 321 | BEST_AA_1P | 1 | 구현 | 등록만 | Root |
| 320 | BEST_AAA_1P | 1 | 구현 | 등록만 | Root |
| 198 | REPLAYDATA_SAVED | 1 | 구현 | 없음 | Result |
| 196 | NO_REPLAYDATA | 1 | 구현 | 없음 | Result |
| 190 | NO_STAGEFILE | 1 | 구현 | 등록만 | Result |
| 23 | PANEL3 | 1 | 구현 | 등록만 | Select |
| 22 | PANEL2 | 1 | 구현 | 등록만 | Select |

### MAIN.NUM 중 R-BMS가 값을 채우지 않는 사용 id (사용 횟수 내림차순)

| id | ModernChic 이름 | 사용 | beatoraja 구현 | R-BMS | 사용 위치 |
|---|---|---|---|---|---|
| 370 | CLEAR | 25 | 구현 | 없음 | Result,Root |
| 179 | IR_RANK | 18 | 구현 | 없음 | Result,Root,Select |
| 101 | SCORE2 | 17 | 구현 | 등록만 | Play,Root |
| 371 | TARGET_CLEAR | 11 | 구현 | 없음 | Result,Root |
| 368 | SONGGAUGE_TOTAL | 11 | 구현 | 없음 | Result,Select |
| 182 | IR_PREVRANK | 7 | 구현 | 없음 | Result,Root |
| 180 | IR_TOTALPLAYER | 7 | 구현 | 없음 | Result,Select |
| 399 | RANKING10_INDEX | 6 | 구현 | 없음 | Result,Select |
| 398 | RANKING9_INDEX | 6 | 구현 | 없음 | Result,Select |
| 397 | RANKING8_INDEX | 6 | 구현 | 없음 | Result,Select |
| 396 | RANKING7_INDEX | 6 | 구현 | 없음 | Result,Select |
| 395 | RANKING6_INDEX | 6 | 구현 | 없음 | Result,Select |
| 394 | RANKING5_INDEX | 6 | 구현 | 없음 | Result,Select |
| 393 | RANKING4_INDEX | 6 | 구현 | 없음 | Result,Select |
| 392 | RANKING3_INDEX | 6 | 구현 | 없음 | Result,Select |
| 391 | RANKING2_INDEX | 6 | 구현 | 없음 | Result,Select |
| 390 | RANKING1_INDEX | 6 | 구현 | 없음 | Result,Select |
| 314 | LIFT1 | 5 | 구현 | 없음 | Play,Root |
| 271 | RIVAL_SCORE | 5 | 구현 | 없음 | Root,Select |
| 23 | TIME_DAY | 5 | 구현 | 없음 | Play,Result,Root,Select |
| 22 | TIME_MONTH | 5 | 구현 | 없음 | Play,Result,Root,Select |
| 21 | TIME_YEAR | 5 | 구현 | 없음 | Play,Result,Root,Select |
| 225 | IR_PLAYER_MAX_RATE | 4 | 구현 | 없음 | Result,Select |
| 223 | IR_PLAYER_PERFECT_RATE | 4 | 구현 | 없음 | Result,Select |
| 219 | IR_PLAYER_FULLCOMBO_RATE | 4 | 구현 | 없음 | Result,Select |
| 217 | IR_PLAYER_HARD_RATE | 4 | 구현 | 없음 | Result,Select |
| 215 | IR_PLAYER_NORMAL_RATE | 4 | 구현 | 없음 | Result,Select |
| 213 | IR_PLAYER_EASY_RATE | 4 | 구현 | 없음 | Result,Select |
| 211 | IR_PLAYER_FAILED_RATE | 4 | 구현 | 없음 | Result,Select |
| 209 | IR_PLAYER_EXHARD_RATE | 4 | 구현 | 없음 | Result,Select |
| 207 | IR_PLAYER_LIGHTASSIST_RATE | 4 | 구현 | 없음 | Result,Select |
| 205 | IR_PLAYER_ASSIST_RATE | 4 | 구현 | 없음 | Result,Select |
| 203 | IR_PLAYER_NOPLAY_RATE | 4 | 구현 | 없음 | Result,Select |
| 1164 | SONGLENGTH_SECOND | 3 | 구현 | 없음 | Root |
| 1163 | SONGLENGTH_MINUTE | 3 | 구현 | 없음 | Root |
| 419 | LATE_POOR | 3 | 구현 | 없음 | Play,Result |
| 418 | EARLY_POOR | 3 | 구현 | 없음 | Play,Result |
| 417 | LATE_BAD | 3 | 구현 | 없음 | Play,Result |
| 416 | EARLY_BAD | 3 | 구현 | 없음 | Play,Result |
| 415 | LATE_GOOD | 3 | 구현 | 없음 | Play,Result |
| 414 | EARLY_GOOD | 3 | 구현 | 없음 | Play,Result |
| 413 | LATE_GREAT | 3 | 구현 | 없음 | Play,Result |
| 412 | EARLY_GREAT | 3 | 구현 | 없음 | Play,Result |
| 389 | RANKING10_EXSCORE | 3 | 구현 | 없음 | Result,Select |
| 388 | RANKING9_EXSCORE | 3 | 구현 | 없음 | Result,Select |
| 387 | RANKING8_EXSCORE | 3 | 구현 | 없음 | Result,Select |
| 386 | RANKING7_EXSCORE | 3 | 구현 | 없음 | Result,Select |
| 385 | RANKING6_EXSCORE | 3 | 구현 | 없음 | Result,Select |
| 384 | RANKING5_EXSCORE | 3 | 구현 | 없음 | Result,Select |
| 383 | RANKING4_EXSCORE | 3 | 구현 | 없음 | Result,Select |
| 382 | RANKING3_EXSCORE | 3 | 구현 | 없음 | Result,Select |
| 381 | RANKING2_EXSCORE | 3 | 구현 | 없음 | Result,Select |
| 380 | RANKING1_EXSCORE | 3 | 구현 | 없음 | Result,Select |
| 353 | TOTALNOTE_BSS | 3 | 구현 | 없음 | Play,Select |
| 352 | TOTALNOTE_SCRATCH | 3 | 구현 | 없음 | Play,Select |
| 351 | TOTALNOTE_LN | 3 | 구현 | 없음 | Play,Select |
| 350 | TOTALNOTE_NORMAL | 3 | 구현 | 없음 | Play,Select |
| 312 | DURATION | 3 | 구현 | 없음 | Play,Select |
| 171 | SCORE3 | 3 | 구현 | 등록만 | Result,Root |
| 170 | HIGHSCORE2 | 3 | 구현 | 등록만 | Result,Root |
| 105 | MAXCOMBO2 | 3 | 구현 | 등록만 | Play,Result |
| 12 | JUDGETIMING | 3 | 구현 | 등록만 | Play,Select |
| 1326 | MAXBPM_DURATION_LANECOVER_OFF | 2 | 구현 | 없음 | Play |
| 1325 | MAXBPM_DURATION_GREEN_LANECOVER_ON | 2 | 구현 | 없음 | Play |
| 1324 | MAXBPM_DURATION_LANECOVER_ON | 2 | 구현 | 없음 | Play |
| 1322 | MINBPM_DURATION_LANECOVER_OFF | 2 | 구현 | 없음 | Play |
| 1321 | MINBPM_DURATION_GREEN_LANECOVER_ON | 2 | 구현 | 없음 | Play |
| 1320 | MINBPM_DURATION_LANECOVER_ON | 2 | 구현 | 없음 | Play |
| 1319 | MAINBPM_DURATION_GREEN_LANECOVER_OFF | 2 | 구현 | 없음 | Play |
| 1318 | MAINBPM_DURATION_LANECOVER_OFF | 2 | 구현 | 없음 | Play |
| 1317 | MAINBPM_DURATION_GREEN_LANECOVER_ON | 2 | 구현 | 없음 | Play |
| 1316 | MAINBPM_DURATION_LANECOVER_ON | 2 | 구현 | 없음 | Play |
| 1315 | DURATION_GREEN_LANECOVER_OFF | 2 | 구현 | 없음 | Play |
| 1314 | DURATION_LANECOVER_OFF | 2 | 구현 | 없음 | Play |
| 1313 | DURATION_GREEN_LANECOVER_ON | 2 | 구현 | 없음 | Play |
| 1312 | DURATION_LANECOVER_ON | 2 | 구현 | 없음 | Play |
| 525 | JUDGE_1P_DURATION | 2 | 구현 | 없음 | Play |
| 364 | DENSITY_AVERAGE | 2 | 구현 | 없음 | Root,Select |
| 360 | DENSITY_PEAK | 2 | 구현 | 없음 | Root,Select |
| 240 | IR_PLAYER_MAX_RATE_AFTERDOT | 2 | 구현 | 없음 | Result,Select |
| 239 | IR_PLAYER_PERFECT_RATE_AFTERDOT | 2 | 구현 | 없음 | Result,Select |
| 238 | IR_PLAYER_FULLCOMBO_RATE_AFTERDOT | 2 | 구현 | 없음 | Result,Select |
| 237 | IR_PLAYER_HARD_RATE_AFTERDOT | 2 | 구현 | 없음 | Result,Select |
| 236 | IR_PLAYER_NORMAL_RATE_AFTERDOT | 2 | 구현 | 없음 | Result,Select |
| 235 | IR_PLAYER_EASY_RATE_AFTERDOT | 2 | 구현 | 없음 | Result,Select |
| 234 | IR_PLAYER_FAILED_RATE_AFTERDOT | 2 | 구현 | 없음 | Result,Select |
| 233 | IR_PLAYER_EXHARD_RATE_AFTERDOT | 2 | 구현 | 없음 | Result,Select |
| 232 | IR_PLAYER_LIGHTASSIST_RATE_AFTERDOT | 2 | 구현 | 없음 | Result,Select |
| 231 | IR_PLAYER_ASSIST_RATE_AFTERDOT | 2 | 구현 | 없음 | Result,Select |
| 230 | IR_PLAYER_NOPLAY_RATE_AFTERDOT | 2 | 구현 | 없음 | Result,Select |
| 224 | IR_PLAYER_MAX | 2 | 구현 | 없음 | Result,Select |
| 222 | IR_PLAYER_PERFECT | 2 | 구현 | 없음 | Result,Select |
| 218 | IR_PLAYER_FULLCOMBO | 2 | 구현 | 없음 | Result,Select |
| 216 | IR_PLAYER_HARD | 2 | 구현 | 없음 | Result,Select |
| 214 | IR_PLAYER_NORMAL | 2 | 구현 | 없음 | Result,Select |
| 212 | IR_PLAYER_EASY | 2 | 구현 | 없음 | Result,Select |
| 210 | IR_PLAYER_FAILED | 2 | 구현 | 없음 | Result,Select |
| 208 | IR_PLAYER_EXHARD | 2 | 구현 | 없음 | Result,Select |
| 206 | IR_PLAYER_LIGHTASSIST | 2 | 구현 | 없음 | Result,Select |
| 204 | IR_PLAYER_ASSIST | 2 | 구현 | 없음 | Result,Select |
| 202 | IR_PLAYER_NOPLAY | 2 | 구현 | 없음 | Result,Select |
| 178 | DIFF_MISSCOUNT | 2 | 구현 | 등록만 | Result |
| 177 | MISSCOUNT2 | 2 | 구현 | 등록만 | Result |
| 172 | DIFF_HIGHSCORE2 | 2 | 구현 | 등록만 | Result |
| 103 | SCORE_RATE_AFTERDOT | 2 | 구현 | 등록만 | Result,Select |
| 102 | SCORE_RATE | 2 | 구현 | 등록만 | Result,Select |
| 100 | POINT | 2 | 구현 | 등록만 | Play |
| 26 | TIME_SECOND | 2 | 구현 | 없음 | Play,Result |
| 25 | TIME_MINUTE | 2 | 구현 | 없음 | Play,Result |
| 24 | TIME_HOUR | 2 | 구현 | 없음 | Play,Result |
| 526 | JUDGE_2P_DURATION | 1 | 구현 | 없음 | Play |
| 427 | BAD_PLUS_POOR_PLUS_MISS | 1 | 구현 | 없음 | Play |
| 425 | COMBOBREAK | 1 | 구현 | 없음 | Select |
| 422 | LATE_MISS | 1 | 구현 | 없음 | Result |
| 421 | EARLY_MISS | 1 | 구현 | 없음 | Result |
| 400 | JUDGERANK | 1 | 구현 | 없음 | Select |
| 377 | STDDEV_TIMING_AFTERDOT | 1 | 구현 | 없음 | Result |
| 376 | STDDEV_TIMING | 1 | 구현 | 없음 | Result |
| 375 | AVERAGE_TIMING_AFTERDOT | 1 | 구현 | 없음 | Result |
| 374 | AVERAGE_TIMING | 1 | 구현 | 없음 | Result |
| 365 | DENSITY_AVERAGE_AFTERDOT | 1 | 구현 | 없음 | Root |
| 362 | DENSITY_END | 1 | 구현 | 없음 | Select |
| 361 | DENSITY_PEAK_AFTERDOT | 1 | 구현 | 없음 | Root |
| 330 | FOLDER_MAX | 1 | 구현 | 없음 | Select |
| 329 | FOLDER_PERFECT | 1 | 구현 | 없음 | Select |
| 328 | FOLDER_FULLCOMBO | 1 | 구현 | 없음 | Select |
| 327 | FOLDER_EXHARD | 1 | 구현 | 없음 | Select |
| 326 | FOLDER_HARD | 1 | 구현 | 없음 | Select |
| 325 | FOLDER_GROOOVE | 1 | 구현 | 없음 | Select |
| 324 | FOLDER_EASY | 1 | 구현 | 없음 | Select |
| 323 | FOLDER_LASSIST | 1 | 구현 | 없음 | Select |
| 322 | FOLDER_ASSIST | 1 | 구현 | 없음 | Select |
| 321 | FOLDER_FAILED | 1 | 구현 | 없음 | Select |
| 320 | FOLDER_NOPLAY | 1 | 구현 | 없음 | Select |
| 300 | FOLDER_TOTALSONGS | 1 | 구현 | 없음 | Select |
| 284 | RIVAL_POOR | 1 | 구현 | 없음 | Select |
| 283 | RIVAL_BAD | 1 | 구현 | 없음 | Select |
| 282 | RIVAL_GOOD | 1 | 구현 | 없음 | Select |
| 281 | RIVAL_GREAT | 1 | 구현 | 없음 | Select |
| 280 | RIVAL_PERFECT | 1 | 구현 | 없음 | Select |
| 242 | IR_PLAYER_TOTAL_FULLCOMBO_RATE_AFTERDOT | 1 | 구현 | 없음 | Select |
| 241 | IR_PLAYER_TOTAL_CLEAR_RATE_AFTERDOT | 1 | 구현 | 없음 | Select |
| 229 | IR_PLAYER_TOTAL_FULLCOMBO_RATE | 1 | 구현 | 없음 | Select |
| 227 | IR_PLAYER_TOTAL_CLEAR_RATE | 1 | 구현 | 없음 | Select |
| 176 | TARGET_MISSCOUNT | 1 | 구현 | 등록만 | Result |
| 175 | DIFF_MAXCOMBO | 1 | 구현 | 등록만 | Result |
| 76 | MISSCOUNT | 1 | 구현 | 등록만 | Select |

### MAIN.STRING 중 R-BMS가 값을 채우지 않는 사용 id (사용 횟수 내림차순)

| id | ModernChic 이름 | 사용 | beatoraja 구현 | R-BMS | 사용 위치 |
|---|---|---|---|---|---|
| 155 | COURSE6_TITLE | 8 | 구현 | 없음 | Result,Root |
| 15 | SUBARTIST | 8 | 구현 | 등록만 | Decide,Play,Select |
| 1020 | IR_NAME | 6 | 구현 | 등록만 | Result,Root,Select |
| 1003 | TABLE_FULL | 6 | 구현 | 등록만 | Decide,Play,Result |
| 1 | RIVAL | 4 | 구현 | 등록만 | Play,Select |
| 150 | COURSE1_TITLE | 3 | 구현 | 없음 | Result,Root |
| 2 | PLAYER | 3 | 구현 | 등록만 | Result,Select |
| 1031 | SONG_HASH_SHA256 | 2 | 구현 | 등록만 | Result |
| 1030 | SONG_HASH_MD5 | 2 | 구현 | 등록만 | Result |
| 1010 | VERSION | 2 | 구현 | 등록만 | Root |
| 3 | SELECTED_TARGET | 2 | 구현 | 없음 | Select |
| 219 | TARGET_BACKWARD10 | 1 | 구현 | 없음 | Select |
| 218 | TARGET_BACKWARD9 | 1 | 구현 | 없음 | Select |
| 217 | TARGET_BACKWARD8 | 1 | 구현 | 없음 | Select |
| 216 | TARGET_BACKWARD7 | 1 | 구현 | 없음 | Select |
| 215 | TARGET_BACKWARD6 | 1 | 구현 | 없음 | Select |
| 214 | TARGET_BACKWARD5 | 1 | 구현 | 없음 | Select |
| 213 | TARGET_BACKWARD4 | 1 | 구현 | 없음 | Select |
| 212 | TARGET_BACKWARD3 | 1 | 구현 | 없음 | Select |
| 211 | TARGET_BACKWARD2 | 1 | 구현 | 없음 | Select |
| 210 | TARGET_BACKWARD1 | 1 | 구현 | 없음 | Select |
| 209 | TARGET_FORWARD10 | 1 | 구현 | 없음 | Select |
| 208 | TARGET_FORWARD9 | 1 | 구현 | 없음 | Select |
| 207 | TARGET_FORWARD8 | 1 | 구현 | 없음 | Select |
| 206 | TARGET_FORWARD7 | 1 | 구현 | 없음 | Select |
| 205 | TARGET_FORWARD6 | 1 | 구현 | 없음 | Select |
| 204 | TARGET_FORWARD5 | 1 | 구현 | 없음 | Select |
| 203 | TARGET_FORWARD4 | 1 | 구현 | 없음 | Select |
| 202 | TARGET_FORWARD3 | 1 | 구현 | 없음 | Select |
| 201 | TARGET_FORWARD2 | 1 | 구현 | 없음 | Select |
| 200 | TARGET_FORWARD1 | 1 | 구현 | 없음 | Select |
| 119 | SKIN_CUSTOMIZE_ITEM10 | 1 | 구현 | 등록만 | SkinSelect |
| 118 | SKIN_CUSTOMIZE_ITEM9 | 1 | 구현 | 등록만 | SkinSelect |
| 117 | SKIN_CUSTOMIZE_ITEM8 | 1 | 구현 | 등록만 | SkinSelect |
| 116 | SKIN_CUSTOMIZE_ITEM7 | 1 | 구현 | 등록만 | SkinSelect |
| 115 | SKIN_CUSTOMIZE_ITEM6 | 1 | 구현 | 등록만 | SkinSelect |
| 114 | SKIN_CUSTOMIZE_ITEM5 | 1 | 구현 | 등록만 | SkinSelect |
| 113 | SKIN_CUSTOMIZE_ITEM4 | 1 | 구현 | 등록만 | SkinSelect |
| 112 | SKIN_CUSTOMIZE_ITEM3 | 1 | 구현 | 등록만 | SkinSelect |
| 111 | SKIN_CUSTOMIZE_ITEM2 | 1 | 구현 | 등록만 | SkinSelect |
| 110 | SKIN_CUSTOMIZE_ITEM1 | 1 | 구현 | 등록만 | SkinSelect |
| 109 | SKIN_CUSTOMIZE_CATEGORY10 | 1 | 구현 | 등록만 | SkinSelect |
| 108 | SKIN_CUSTOMIZE_CATEGORY9 | 1 | 구현 | 등록만 | SkinSelect |
| 107 | SKIN_CUSTOMIZE_CATEGORY8 | 1 | 구현 | 등록만 | SkinSelect |
| 106 | SKIN_CUSTOMIZE_CATEGORY7 | 1 | 구현 | 등록만 | SkinSelect |
| 105 | SKIN_CUSTOMIZE_CATEGORY6 | 1 | 구현 | 등록만 | SkinSelect |
| 104 | SKIN_CUSTOMIZE_CATEGORY5 | 1 | 구현 | 등록만 | SkinSelect |
| 103 | SKIN_CUSTOMIZE_CATEGORY4 | 1 | 구현 | 등록만 | SkinSelect |
| 102 | SKIN_CUSTOMIZE_CATEGORY3 | 1 | 구현 | 등록만 | SkinSelect |
| 101 | SKIN_CUSTOMIZE_CATEGORY2 | 1 | 구현 | 등록만 | SkinSelect |
| 100 | SKIN_CUSTOMIZE_CATEGORY1 | 1 | 구현 | 등록만 | SkinSelect |
| 51 | SKIN_AUTHOR | 1 | 구현 | 등록만 | SkinSelect |
| 50 | SKIN_NAME | 1 | 구현 | 등록만 | SkinSelect |

### MAIN.GRAPH 중 R-BMS가 값을 채우지 않는 사용 id (사용 횟수 내림차순)

| id | ModernChic 이름 | 사용 | beatoraja 구현 | R-BMS | 사용 위치 |
|---|---|---|---|---|---|
| 114 | TARGETSCORERATE_NOW | 2 | 구현 | 등록만 | Play |
| 112 | BESTSCORERATE_NOW | 2 | 구현 | 등록만 | Play |
| 111 | SCORERATE_FINAL | 2 | 구현 | 등록만 | Play |
| 110 | SCORERATE | 2 | 구현 | 등록만 | Play |

### MAIN.SLIDER 중 R-BMS가 값을 채우지 않는 사용 id (사용 횟수 내림차순)

| id | ModernChic 이름 | 사용 | beatoraja 구현 | R-BMS | 사용 위치 |
|---|---|---|---|---|---|
| 4 | LANECOVER | 5 | 구현 | 등록만 | Play |
| 19 | BGM_VOLUME | 2 | 구현 | 등록만 | Select |
| 18 | KEY_VOLUME | 2 | 구현 | 등록만 | Select |
| 17 | MASTER_VOLUME | 2 | 구현 | 등록만 | Select |
| 8 | IR_POSITION | 1 | 구현 | 등록만 | Result |
| 7 | SKINSELECT_POSITION | 1 | 구현 | 등록만 | skinselect |

### MAIN.TIMER 중 R-BMS가 값을 채우지 않는 사용 id (사용 횟수 내림차순)

| id | ModernChic 이름 | 사용 | beatoraja 구현 | R-BMS | 사용 위치 |
|---|---|---|---|---|---|
| 23 | PANEL3_ON | 30 | 구현 | 등록만 | Select |
| 22 | PANEL2_ON | 28 | 구현 | 등록만 | Select |
| 143 | ENDOFNOTE_1P | 25 | 구현 | 등록만 | Play |
| 140 | RHYTHM | 25 | 구현 | 등록만 | Play |
| 173 | IR_CONNECT_SUCCESS | 13 | 구현 | 등록만 | Result,Root,Select |
| 172 | IR_CONNECT_BEGIN | 8 | 구현 | 등록만 | Result,Root |
| 141 | PREVIEW | 8 | **구현 없음(사용자 옵션 id 가능성)** | 없음 | Play,Root |
| 2 | FADEOUT | 8 | 구현 | 등록만 | Result,decide,play10_hw,play5_hw,play7_hw |
| 33 | PANEL3_OFF | 2 | 구현 | 등록만 | Select |
| 32 | PANEL2_OFF | 2 | 구현 | 등록만 | Select |
| 352 | SCORE_TARGET | 1 | 구현 | 등록만 | Root |
| 351 | SCORE_BEST | 1 | 구현 | 등록만 | Root |
| 174 | IR_CONNECT_FAIL | 1 | 구현 | 등록만 | Result |

### MAIN.BUTTON (이미지 인덱스 ref / 클릭 act / 스크립트 event_index)

| id | 이름 | 사용 | 역할별 | beatoraja 구현 | R-BMS |
|---|---|---|---|---|---|
| 308 | LNMODE | 9 | ref5,script3,act1 | 이미지인덱스+이벤트 | 이벤트 없음 / 이미지 인덱스 공간 없음 |
| 42 | RANDOM_1P | 5 | ref4,act1 | 이미지인덱스+이벤트 | 이벤트 없음 / 이미지 인덱스 공간 없음 |
| 43 | RANDOM_2P | 5 | ref4,act1 | 이미지인덱스+이벤트 | 이벤트 없음 / 이미지 인덱스 공간 없음 |
| 90 | FAVORITTE_CHART | 4 | ref2,act2 | 이미지인덱스+이벤트 | 이벤트 없음 / 이미지 인덱스 공간 없음 |
| 55 | HSFIX | 4 | script2,ref2 | 이미지인덱스+이벤트 | 이벤트 없음 / 이미지 인덱스 공간 없음 |
| 301 | ASSIST_EXJUDGE | 3 | script1,ref1,act1 | 이미지인덱스 | 이벤트 없음 / 이미지 인덱스 공간 없음 |
| 303 | ASSIST_JUDGEAREA | 3 | script1,ref1,act1 | 이미지인덱스 | 이벤트 없음 / 이미지 인덱스 공간 없음 |
| 305 | ASSIST_MARKNOTE | 3 | script1,ref1,act1 | 이미지인덱스 | 이벤트 없음 / 이미지 인덱스 공간 없음 |
| 307 | ASSIST_NOMINE | 3 | script1,ref1,act1 | 이미지인덱스 | 이벤트 없음 / 이미지 인덱스 공간 없음 |
| 302 | ASSIST_CONSTANT | 3 | script1,ref1,act1 | 이미지인덱스 | 이벤트 없음 / 이미지 인덱스 공간 없음 |
| 304 | ASSIST_LEGACY | 3 | script1,ref1,act1 | 이미지인덱스 | 이벤트 없음 / 이미지 인덱스 공간 없음 |
| 306 | ASSIST_BPMGUIDE | 3 | script1,ref1,act1 | 이미지인덱스 | 이벤트 없음 / 이미지 인덱스 공간 없음 |
| 74 | JUDGE_TIMING | 3 | act3 | 이벤트 | 이벤트 없음 / 이미지 인덱스 공간 없음 |
| 19 | REPLAY | 2 | op1,act1 | 이벤트 | 이벤트 없음 / 이미지 인덱스 공간 없음 |
| 316 | REPLAY2 | 2 | other1,act1 | 이벤트 | 이벤트 없음 / 이미지 인덱스 공간 없음 |
| 317 | REPLAY3 | 2 | other1,act1 | 이벤트 | 이벤트 없음 / 이미지 인덱스 공간 없음 |
| 318 | REPLAY4 | 2 | other1,act1 | 이벤트 | 이벤트 없음 / 이미지 인덱스 공간 없음 |
| 40 | GAUGE_1P | 2 | ref2 | 이미지인덱스+이벤트 | 이벤트 없음 / 이미지 인덱스 공간 없음 |
| 54 | DPOPTION | 2 | ref2 | 이미지인덱스+이벤트 | 이벤트 없음 / 이미지 인덱스 공간 없음 |
| 400 | CONSTANT | 2 | act1,ref1 | 이미지인덱스+이벤트 | 이벤트 없음 / 이미지 인덱스 공간 없음 |
| 341 | BOTTOMSIFTABLEFGAUGE | 2 | act1,ref1 | 이미지인덱스+이벤트 | 이벤트 없음 / 이미지 인덱스 공간 없음 |
| 330 | LANECOVER | 2 | act1,ref1 | 이미지인덱스+이벤트 | 이벤트 없음 / 이미지 인덱스 공간 없음 |
| 331 | LIFT | 2 | act1,ref1 | 이미지인덱스+이벤트 | 이벤트 없음 / 이미지 인덱스 공간 없음 |
| 332 | HIDDEN | 2 | act1,ref1 | 이미지인덱스+이벤트 | 이벤트 없음 / 이미지 인덱스 공간 없음 |
| 342 | HISPEEDAUTOADJUST | 2 | act1,ref1 | 이미지인덱스+이벤트 | 이벤트 없음 / 이미지 인덱스 공간 없음 |
| 340 | JUDGEALGORITHM | 2 | act1,ref1 | 이미지인덱스+이벤트 | 이벤트 없음 / 이미지 인덱스 공간 없음 |
| 11 | MODE | 2 | ref1,act1 | 이미지인덱스+이벤트 | 이벤트 없음 / 이미지 인덱스 공간 없음 |
| 12 | SORT | 2 | ref1,act1 | 이미지인덱스+이벤트 | 이벤트 없음 / 이미지 인덱스 공간 없음 |
| 175 | SKINSELECT_MUSIC_SELECT | 1 | op1 | 이미지인덱스+이벤트 | 이벤트 없음 / 이미지 인덱스 공간 없음 |
| 176 | SKINSELECT_DECIDE | 1 | other1 | 이미지인덱스+이벤트 | 이벤트 없음 / 이미지 인덱스 공간 없음 |
| 177 | SKINSELECT_RESULT | 1 | other1 | 이미지인덱스+이벤트 | 이벤트 없음 / 이미지 인덱스 공간 없음 |
| 185 | SKINSELECT_COURSE_RESULT | 1 | other1 | 이미지인덱스+이벤트 | 이벤트 없음 / 이미지 인덱스 공간 없음 |
| 180 | SKINSELECT_SOUND_SET | 1 | other1 | 이미지인덱스+이벤트 | 이벤트 없음 / 이미지 인덱스 공간 없음 |
| 178 | SKINSELECT_KEY_CONFIG | 1 | other1 | 이미지인덱스+이벤트 | 이벤트 없음 / 이미지 인덱스 공간 없음 |
| 179 | SKINSELECT_SKIN_SELECT | 1 | other1 | 이미지인덱스+이벤트 | 이벤트 없음 / 이미지 인덱스 공간 없음 |
| 181 | SKINSELECT_THEME | 1 | other1 | 이미지인덱스+이벤트 | 이벤트 없음 / 이미지 인덱스 공간 없음 |
| 170 | SKINSELECT_7KEY | 1 | op1 | 이미지인덱스+이벤트 | 이벤트 없음 / 이미지 인덱스 공간 없음 |
| 182 | SKINSELECT_BATTLE7 | 1 | other1 | 이미지인덱스+이벤트 | 이벤트 없음 / 이미지 인덱스 공간 없음 |
| 172 | SKINSELECT_14KEY | 1 | other1 | 이미지인덱스+이벤트 | 이벤트 없음 / 이미지 인덱스 공간 없음 |
| 171 | SKINSELECT_5KEY | 1 | other1 | 이미지인덱스+이벤트 | 이벤트 없음 / 이미지 인덱스 공간 없음 |
| 173 | SKINSELECT_10KEY | 1 | other1 | 이미지인덱스+이벤트 | 이벤트 없음 / 이미지 인덱스 공간 없음 |
| 174 | SKINSELECT_9KEY | 1 | other1 | 이미지인덱스+이벤트 | 이벤트 없음 / 이미지 인덱스 공간 없음 |
| 184 | SKINSELECT_BATTLE9 | 1 | other1 | 이미지인덱스+이벤트 | 이벤트 없음 / 이미지 인덱스 공간 없음 |
| 386 | SKINSELECT_24KEY | 1 | other1 | 이미지인덱스+이벤트 | 이벤트 없음 / 이미지 인덱스 공간 없음 |
| 388 | SKINSELECT_24KEY_BATTLE | 1 | other1 | 이미지인덱스+이벤트 | 이벤트 없음 / 이미지 인덱스 공간 없음 |
| 387 | SKINSELECT_24KEY_DOUBLE | 1 | other1 | 이미지인덱스+이벤트 | 이벤트 없음 / 이미지 인덱스 공간 없음 |
| 220 | SKIN_CUSTOMIZE1 | 1 | op1 | 이벤트 | 이벤트 없음 / 이미지 인덱스 공간 없음 |
| 221 | SKIN_CUSTOMIZE2 | 1 | other1 | 이벤트 | 이벤트 없음 / 이미지 인덱스 공간 없음 |
| 222 | SKIN_CUSTOMIZE3 | 1 | other1 | 이벤트 | 이벤트 없음 / 이미지 인덱스 공간 없음 |
| 223 | SKIN_CUSTOMIZE4 | 1 | other1 | 이벤트 | 이벤트 없음 / 이미지 인덱스 공간 없음 |
| 224 | SKIN_CUSTOMIZE5 | 1 | other1 | 이벤트 | 이벤트 없음 / 이미지 인덱스 공간 없음 |
| 225 | SKIN_CUSTOMIZE6 | 1 | other1 | 이벤트 | 이벤트 없음 / 이미지 인덱스 공간 없음 |
| 226 | SKIN_CUSTOMIZE7 | 1 | other1 | 이벤트 | 이벤트 없음 / 이미지 인덱스 공간 없음 |
| 227 | SKIN_CUSTOMIZE8 | 1 | other1 | 이벤트 | 이벤트 없음 / 이미지 인덱스 공간 없음 |
| 228 | SKIN_CUSTOMIZE9 | 1 | other1 | 이벤트 | 이벤트 없음 / 이미지 인덱스 공간 없음 |
| 229 | SKIN_CUSTOMIZE10 | 1 | other1 | 이벤트 | 이벤트 없음 / 이미지 인덱스 공간 없음 |
| 190 | CHANGE_SKIN | 1 | act1 | 이벤트 | 이벤트 없음 / 이미지 인덱스 공간 없음 |
| 13 | KEYCONFIG | 1 | act1 | 이벤트 | 이벤트 없음 / 이미지 인덱스 공간 없음 |
| 14 | SKINSELECT | 1 | act1 | 이벤트 | 이벤트 없음 / 이미지 인덱스 공간 없음 |
| 210 | OPEN_IR_WEBSITE | 1 | act1 | 이벤트 | 이벤트 없음 / 이미지 인덱스 공간 없음 |
| 321 | AUTOSAVEREPLAY_1 | 1 | op1 | 이미지인덱스+이벤트 | 이벤트 없음 / 이미지 인덱스 공간 없음 |
| 322 | AUTOSAVEREPLAY_2 | 1 | other1 | 이미지인덱스+이벤트 | 이벤트 없음 / 이미지 인덱스 공간 없음 |
| 323 | AUTOSAVEREPLAY_3 | 1 | other1 | 이미지인덱스+이벤트 | 이벤트 없음 / 이미지 인덱스 공간 없음 |
| 324 | AUTOSAVEREPLAY_4 | 1 | other1 | 이미지인덱스+이벤트 | 이벤트 없음 / 이미지 인덱스 공간 없음 |
| 79 | RIVAL | 1 | act1 | 이벤트 | 이벤트 없음 / 이미지 인덱스 공간 없음 |
| 78 | GAUGEAUTOSHIFT | 1 | ref1 | 이미지인덱스+이벤트 | 이벤트 없음 / 이미지 인덱스 공간 없음 |
| 72 | BGA | 1 | ref1 | 이미지인덱스+이벤트 | 이벤트 없음 / 이미지 인덱스 공간 없음 |
| 75 | JUDGE_TIMING_AUTO_ADJUST | 1 | ref1 | 이미지인덱스+이벤트 | 이벤트 없음 / 이미지 인덱스 공간 없음 |
| 16 | AUTOPLAY | 1 | act1 | 이벤트 | 이벤트 없음 / 이미지 인덱스 공간 없음 |
| 315 | PRACTICE | 1 | act1 | 이벤트 | 이벤트 없음 / 이미지 인덱스 공간 없음 |

### 리터럴 id (이름 표를 거치지 않고 숫자로 직접 쓴 곳)

- timer=: 2(x1)
- option: 2(x2), 90(x1), 293(x2)
- text: 10(x1), 12(x1), 119(x2)
- number: 74(x1), 179(x1), 180(x1), 368(x1), 379(x4), 389(x2)
- ref: 11(x1), 33(x1), 34(x1), 35(x1), 36(x1), 37(x1), 74(x1), 77(x1), 96(x6), 179(x1), 180(x1), 181(x1), 220(x1), 227(x1), 241(x1), 333(x1), 368(x1), 1163(x1), 1164(x1)

---

## 16. R-BMS 쪽에서 확인된 결함과 정의 불일치 (근거 줄)

| 번호 | 위치 | 내용 |
|---|---|---|
| 1 | property/mod.rs:279, 405-408, 431 | `MAPPINGS`/`source_of`/`UnmappedLog::check`는 정의만 있고 호출처가 없다. `MAPPINGS`가 라우팅을 선언한 id 중 값을 채우는 어댑터가 없는 id가 "등록만"이다 |
| 2 | property/mod.rs:86, state.rs 전체 | 미구현/데이터 없음이 `0`, `""`, `false`, 0.0이다. beatoraja는 NUMBER `MIN_VALUE`, FLOAT `Float.MIN_VALUE`, 타이머 `Long.MIN_VALUE`로 구분하고 소비자가 "그리지 않음"으로 처리한다. draw.rs:56,264,287은 센티널 처리 경로가 있으나 어댑터가 센티널을 돌려주지 않는다 |
| 3 | state.rs:293-305, 441-450, 575-585, 684-689, 751-754 | 음수 불리언 id는 `!answer`라서 구현 없는 id의 음수가 true. beatoraja Lua `option(-id)`는 팩토리 null이면 false이고 draw 조건으로는 스킨 옵션 맵 대조 후 제거 |
| 4 | state.rs:296-298 | `OPTION_1P_EARLY/LATE`(1242/1243)가 `judged_on && last_fast`(PG 포함). beatoraja: `getNowJudge>1`(PG 제외) && recentTiming 부호 (BooleanPropertyFactory.java:234-249) |
| 5 | state.rs:298, 46, 178 | `OPTION_1P_PERFECT`부터 6개(241-246)와 2P 261-266을 모두 응답. beatoraja는 241/261/361(PGREAT)만 구현하고 242-246은 팩토리 null (스킨이 쓰면 객체가 제거됨) |
| 6 | state.rs:299-301 | `OPTION_GAUGE_GROOVE_2P/HARD_2P/EX_2P`(44/45/1047)를 응답. beatoraja에는 구현이 없다. 1P(42/43/1046)는 gaugeType 기준이 동일(`<=2`, `>=3`, `{0,1,4,5,7,8}`) |
| 7 | state.rs:302, 580-581 | 플레이의 `OPTION_NOW_*_1P`(340-347)를 `ex_score/max_ex`(전체 노트)로 판정. beatoraja `createNowRank`는 `ScoreDataProperty.nowrate = ex/(통과 노트*2)` 기준 (ScoreDataProperty.java:97, 110-112). 결과 화면은 통과 노트=전체 노트라 동일 |
| 8 | state.rs:326 | `NUMBER_LANECOVER1`(14)에 `hud.white_number`(흰 숫자)를 돌려준다. beatoraja는 `lanecover*1000`(IntegerPropertyFactory.java:68-73). `NUMBER_LIFT1`(314), `NUMBER_HIDDEN1`(315), `NUMBER_LANECOVER2`(316)는 미구현 |
| 9 | state.rs:321, 172-174 | `NUMBER_GROOVEGAUGE_AFTERDOT`(407)를 소수 2자리(0..99)로 돌려준다. beatoraja는 소수 1자리(`((int)(value*10))%10`, 0<v<0.1이면 1) (IntegerPropertyFactory.java:672-688) |
| 10 | state.rs:319 | `NUMBER_DIFF_HIGHSCORE`(152) = `ex - best_ex`(전체). beatoraja는 `nowEXScore - nowBestScore`(최고기록을 통과 노트 수로 비례 환산) (ScoreDataProperty.java:119-134, IntegerPropertyFactory.java:702-705) |
| 11 | state.rs:348, 351 | `FLOAT_SCORE_RATE`(1102)=`ex/max_ex`, beatoraja=`getNowRate`(통과 노트 기준). `FLOAT_GROOVEGAUGE_1P`(1107)=게이지/100(0..1), beatoraja=게이지 원값(0..100) |
| 12 | state.rs:344-346, 611-613 | 판정 비율 `RATE_PGREAT..POOR`(140-144)를 플레이/결과에서 응답. beatoraja는 선곡 전용(선택 바 score)이며 플레이/결과에서는 0 |
| 13 | state.rs:766-772 | KeyConfig 문자열: 240-283을 `id-240`으로 읽어 10칸 어긋나고, 50-239도 키 라벨을 돌려준다 (2.6/6장 참고) |
| 14 | state.rs:604-605 | 결과의 `NUMBER_EARLY_PERFECT/LATE_PERFECT`(410/411)에 전체 FAST/SLOW 합(`lane_kind_total`)을 돌려준다. beatoraja는 PERFECT의 FAST/SLOW 개수 |
| 15 | state.rs:332-335 | `NUMBER_TOTALEARLY/LATE`(423/424)에 `lane_kind_total(fast/slow)`. beatoraja는 판정 1..5(PG 제외)의 early/late 합 |
| 16 | state.rs:314 | `NUMBER_COMBO`(104)를 응답하지만 beatoraja에는 이 id의 팩토리 구현이 없다 (콤보 숫자는 SkinJudge 내부) |
| 17 | screen.rs:262-265 | 플레이 시작 시 `TIMER_READY`를 끈다. beatoraja는 켠 채 유지 (BMSPlayer.java:514,591, 끄는 코드 없음) |
| 18 | screen.rs:366-371 | `SONGBAR_MOVE`(10), `MOVE_UP`(12), `MOVE_DOWN`(13)을 막대 이동 시 켠다. beatoraja는 이 타이머를 켜지 않는다(항상 off) -> 이 타이머로 보이도록 만든 스킨 객체가 beatoraja에서는 절대 안 보임 |
| 19 | screen.rs:292-293 | `GAUGE_INCLEASE_1P`(42)를 켠다. beatoraja는 켜지 않는다 |
| 20 | screen.rs:402-416 | `RESULTGRAPH_END`를 1초 뒤에 켠다(beatoraja: BEGIN과 같은 프레임에 즉시). `RESULT_UPDATESCORE`를 신기록일 때 켠다(beatoraja: skin `rankTime==0`이면 즉시, 아니면 확인 입력 시, MusicResult.java:164-169,288-291) |
| 21 | screen.rs:311-343 | 폭탄 타이머를 "연소 중" 에지로 켜고 끈다. beatoraja는 `judge <= skin.judgetimer(기본 1)`인 모든 판정마다 `setTimerOn`(재시작)하고 끄지 않는다 (JudgeManager.java:674). 같은 레인 연타 시 R-BMS는 재시작하지 않음 |
| 22 | object.rs:280 | 이미지 선택 인덱스를 `state.integer`로 읽는다(이미지 인덱스 공간 부재, 1장 5번) |
| 23 | lua.rs:203-210 | Lua 노출은 `skin.boolean/number/float/text/timer/time` 6개. ModernChic가 쓰는 `main_state.option/number/text/timer/time/event_index/gauge_type/judge/volume_*/rate/exscore/float_number/gauge/audio_play`와 이름과 단위(µs, `timer_off_value`)가 다르다. 이 장에서는 속성 id 쪽만 다루며 Lua 표면은 별도 조사 대상 |
| 24 | 해당 코드 없음 | `OFFSET_SCRATCHANGLE_1P/2P`(1,2) 갱신이 없다. LIFT/LANECOVER/HIDDEN_COVER는 `PlayViewState::cover_offset`(state.rs:270-281)로 구현 |
| 25 | 해당 코드 없음 | `main_state.offset()`에 해당하는 읽기, `OFFSET_ALL`(10) 플레이 전체 이동, `Skin.getOffsetAll` 대응 코드가 없다(`SkinCustomisation.offsets` 사용자 nudge만) |

---

## 17. 구현 군집별 정리 (구현자가 한 번에 묶어 만들 수 있는 단위)

각 군집의 id 범위는 3~9장 표를 따른다. 군집 기준은 값의 출처가 같은가이다.

| 군집 | 값의 출처 | 해당 id (종류: 범위) | R-BMS 현황 |
|---|---|---|---|
| A. 곡/차트 메타 | `SongData` + `information`(밀도/총게이지/노트 종류별) | NUMBER 45-49, 90-92, 96, 350-353, 360-365, 368, 400, 1163-1164; FLOAT 360, 362, 367, 368; OPTION 150-155, 160-164, 170-184, 190-195, 1160-1161, 1177, 1008; STRING 11-16, 1030-1031 | 선곡/결정에서 제목/부제/아티스트/장르/레벨 일부만 |
| B. 라이브 플레이 점수 | `ScoreDataProperty` + `JudgeManager.scoreData` | NUMBER 71-72, 74-75, 100-108, 115-116, 121-123, 128, 135-136, 150-158, 170-178, 183-184, 410-427; FLOAT 85-89, 155, 157, 183, 1102, 1115; RATE 110-115; OPTION 200-207, 220-227, 230-240, 300-307, 320-327, 340-347, 2241-2246 | 일부(EX/최고기록/목표/판정 수/게이지) |
| C. 판정 표시 상태 | `JudgeManager.nowJudge/recentJudgeTiming/judge[][]` | OPTION 241/261/361, 1242/1243/1262/1263/1362/1363; NUMBER 525-527; 이미지 인덱스 500-519, 1510-1699; 타이머 46/47/247, 446-448 | PG~MISS 개별 옵션을 구현했으나 의미 다름 |
| D. 입력/레인 상태 | `KeyInputProccessor`, `JudgeManager.LaneState`, `LaneRenderer` | 타이머 KEYON/KEYOFF/BOMB/HOLD/HCN(식 11장); OFFSET 1-5; OPTION 270-273; NUMBER 14, 314-316, 1312-1327; RATE 4, 5 | 키/폭탄/홀드 타이머와 LIFT/COVER/HIDDEN 오프셋만 |
| E. 플레이어 설정 | `PlayerConfig`/`PlayConfig`/`Config` | 이미지 인덱스 10-12, 40-43, 54-55, 61-63, 72, 75, 78, 89-90, 301-308, 321-324, 330-332, 340-343, 350-353, 360-361, 400; NUMBER 10, 12, 57-59, 310-313; FLOAT 310; RATE 17-19; OPTION 60-62; 이벤트 대응 | 거의 없음 |
| F. 선곡 바/패널 상태 | `MusicSelector`/`BarManager` | OPTION 1-5, 21-23, 100-105, 624-625, 1002-1017, 1030-1031, 1100-1104, 1205-1208, 603-608; NUMBER 77-79, 243-249, 300, 320-330; RATE 1, 8; 타이머 10-16, 21-36(1-3번 패널만 실사용) | 폴더/곡 막대, 패널1만 |
| G. 결과/비교 | `AbstractResult` newScore/oldScore | NUMBER 76, 170-178, 370-377; OPTION 90-91, 330-336, 1330-1336, 352-354; 이미지 인덱스 370-371; 타이머 150-152 | 일부(결과/실패, 랭크, 최고기록 차이) |
| H. IR/랭킹/라이벌 | `RankingData`, `PlayerInformation` | NUMBER 179-182, 200-242, 271-289, 380-399; FLOAT 203-229, 285-289; OPTION 50-51, 601-608; STRING 1, 120-129, 1020-1021; 타이머 172-174 | 없음 |
| I. 시간/통계/시스템 | OS 시계, `PlayerData`, 오디오 설정 | NUMBER 17-37, 57-59, 333; STRING 1010 | 없음 |
| J. 스킨 설정 UI | `SkinConfiguration` | STRING 50-51, 100-119; RATE 7; 이벤트 170-190, 220-228; 이미지 인덱스 170-185, 386-388 | 없음 |
| K. 키 컨피그 | `KeyConfiguration` | STRING 40-49, 240-283; 이벤트 101-139, 150-163(무동작) | 키 라벨 일부(오프셋 결함) |
| L. 연습 모드 | `PracticeConfiguration` | OPTION 1080, 3000-3035; STRING 1040-1095; RATE 20; 이벤트 370-385 | 없음 |
| M. 로딩/진행 | `BMSResource` | NUMBER 165; FLOAT 165; RATE 102; OPTION 80-81 | 결정 화면에서 구현 |

---

## 18. 미확인 항목과 읽지 못한 범위

읽지 못했거나 일부만 읽은 파일(구현 시 재확인 필요):

- `PlayerData`, `ScoreData` 클래스 본문(필드 의미는 호출 식에서만 확인). `RankingData`, `TargetProperty`, `BarManager`, `PracticeConfiguration` 본문. 해당 id의 "값 계산 근거"는 팩토리 식 수준이다.
- `SkinLuaAccessor.java`(Lua `main_state` 외 `skin_config`, `timer_util` 등 다른 노출 계층). 이 보고서는 `MainStatePropertyLuaApiExporter`만 읽었다.
- `LR2` 로더들(`LR2SkinLoader.java` 등)의 옵션 id 변환 규칙. ModernChic는 LR2 스킨이 아니므로 제외했다.
- `JudgeManager`의 판정 창/미스 조건과 `BMSPlayer.getPlaytime()`(= playtime 정의), `TIME_MARGIN` 값. `RATE_MUSIC_PROGRESS` 분모 정의는 R-BMS와 동일한지 미확인.
- R-BMS `HudView`/`ResultView`의 필드 정의(`green_number`, `pace`, `fast/slow` 배열, `genre_maker`)는 이름과 사용처만 확인했고 값 정의는 미확인이다. 해당 id에 "미확인"을 표시했다.
- R-BMS `skin_select.rs`(스킨 선택 화면)에는 속성 어댑터가 없고, `STRING_SKIN_*`/`RATE_SKINSELECT_POSITION`/`BUTTON_SKINSELECT_*`를 채우는 코드를 찾지 못했다(grep). 스킨 선택 화면 자체는 다른 조사 범위다.
- ModernChic 집계는 `Root/main*.lua` 이름 표와 `MAIN.<표>.<이름>` 정규식 참조 및 `main_state.*(숫자)` 리터럴을 기준으로 했다. 변수에 담아 간접 참조하거나 `+ i` 같은 계산으로 만든 id(예: `main_state.number(379 + i)` = RANKING 1..10 EXSCORE)는 이름 집계에서 빠지고 15장 리터럴 목록에만 나온다. 실제 사용 id는 이보다 약간 많다.
- 코드 실행(cargo/bun)은 하지 않았다. R-BMS 동작 판단은 소스 읽기로만 했다.
