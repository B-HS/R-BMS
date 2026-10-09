# m5. ModernChic 결정(decide) / 결과(result) / 코스 결과(course) 화면 스킨 조사 보고서

> 최종 갱신 2026-10-10 · 대응 단계: 웨이브 2(Lua 런타임·로더·스킨 팩) 반영 · 본문은 기준 커밋 `9ce92bb` 시점 서술이며, 아래 "웨이브 … 반영 사항"이 최신 것부터 우선한다 · 색인과 갱신 규칙은 [README.md](README.md)

## 웨이브 2B 반영 사항 (2026-10-10)

스킨 덤프 CLI, 앱의 스킨 팩 폴더 지정과 `.luaskin` 로드, 오버레이 총 크기 상한, 외부 스킨 첫 정지 프레임을 넣은 뒤의 상태다.

- (W2-9) m5-decide-result.md §9.1: 상단에 'W2-9 캡처 대조 결과: lockon 모서리, 텍스트 5종, 라벨, 레벨 숫자의 위치·색은 표와 일치. 검정 띠(-110), 스테이지 파일(-100), 그래프 2종, 페이드아웃 BLACK 은 미구현으로 빠짐' 추가
- (W2-9) m5-decide-result.md §9.3: 'W2-9 캡처 대조 결과: 5번 mainmenu 표의 메뉴1 항목과 3번 centerinfo, 7번 prepare, 8번 버튼은 좌표대로 나옴. remainNotesFrame 은 클리어 시나리오에서 보이지 않음(조건부로 추정, 표에 조건 명시 필요). 게이지 2001, gra_fastRate, 하단 BLACK 바는 빠짐' 추가
- (W2-9) m5-decide-result.md §4.1·§4.2: 손 계산 destination 수가 실측과 일치함을 표시(decide 69, result 194)


표기 규칙

- MC = /Users/hyunseokbyun/Downloads/ModernChic
- BJ = /Users/hyunseokbyun/development/beatoraja/src/bms/player/beatoraja
- 좌표는 전부 1920x1080 스킨 좌표, y 축은 화면 아래가 0 인 상향(libGDX 기준)이다. 근거는 11장.
- 객체 개수와 좌표 합산은 Lua 인터프리터가 없어 실행하지 못하고(cargo 도 금지) 소스를 손으로 읽어 유도한 값이다. 숫자 하나가 틀릴 수 있으므로 "대략 N" 이 아니라 "손 계산 N" 으로 읽는다.

---

## 0. 조사 범위와 한계

읽은 파일(전부): MC/decide.lua, MC/result.lua, MC/course.lua, MC/*.luaskin(decide, result, course), MC/config.lua, MC/difflist.txt, Decide/lua/require/{header,property,textproperty}.lua, Result/lua/{background,base,centerinfo,diflist,fadeout,history,impression,irmenu,mainmenu,prepare}.lua, Result/lua/require/{header,listchoice,property,textproperty}.lua, Root/{define,define2,mainoption,mainnumber,mainstring,maintimer,mainbutton,mainimage,mainoffset,mainslider,maingraph,customnumber,customfunction,customoption,customtext,customtime,customtimer,customgraph,customslider,version,author}.lua, Root/customsound.lua(76-160, 236-272 만 읽음), Decide/readme.txt, Result/readme.txt, Result/parts 하위 안내 txt 전부, io/Result 하위(전부 빈 폴더).

PNG 직접 확인: Decide/parts/parts.png, parts2.png, Decide/bg/image/sample.png, Result/parts/{system,parts,number,prepare,lamp,ring}.png, Result/parts/rank/#default.png, Result/parts/bg/isclear/clear/#default.png, Result/parts/char/isclear/clear/yuki.png.

엔진 참조(beatoraja 소스): BJ/skin/lua/{LuaSkinLoader,SkinLuaAccessor,TimerUtility,LegacySkinLuaApi,MainStatePropertyLuaApiExporter}.java, BJ/skin/json/{JsonSkin,JSONSkinLoader,JsonSkinObjectLoader}.java, BJ/skin/{SkinObject,SkinNumber,SkinGraph,SkinText*,SkinHeader,SkinLoader}.java, BJ/skin/property/{IntegerPropertyFactory,EventFactory}.java, BJ/decide/MusicDecide.java, BJ/result/MusicResult.java.

읽지 못한 것: Select/Play/KeyConfig/SkinSelect 의 Lua(다른 조사자 배정), mp4/ogg/ttf/fnt 의 내용, Root/customsound.lua 의 160-236 줄(play 스킨 전용 함수), 개별 gauge/*.png 시트의 실제 모양(크기만 확인), Result/parts/rank/ 의 #default 이외 6개 이미지 내용.

중요한 전제 하나: 이 로컬 BJ 소스는 보통의 beatoraja 가 아니라 Lua 샌드박스가 강화된 포크다(10.1 절). ModernChic 은 원래 stock beatoraja(LuaJ 3.0 + 완전한 luajava) 용이므로, "정답 동작"은 stock 의미론이고 포크의 facade 는 ModernChic 이 실제로 쓰는 표면(범위)을 알려주는 근거로만 쓴다.

---

## 1. 구현 사양에 직접 영향을 주는 결론 (요약)

1. decide/result/course 의 `main()` 은 화면 진입 때마다(BJ/decide/MusicDecide.java:24-28, BJ/result/MusicResult.java:42-71 `create()` 안의 `loadSkin`) 라이브 상태 위에서 새로 실행된다. 곡 제목·난이도 색·클리어 여부·코스 여부·플레이어 이름·현재 날짜가 로드 시점에 `main_state.option/text/number` 로 읽혀 문자열 상수나 분기로 굳는다(2.4 절). 따라서 이 스킨은 "문서 한 번 파싱 후 재사용"으로는 동작하지 않고 화면 진입마다 Lua 를 다시 평가해야 한다.
2. `.luaskin` 은 헤더 모드(`skin_config == nil`)와 본 모드(`skin_config` 존재)를 둘 다 지원해야 한다. 헤더 모드에서 `main_state`, `timer_util`, `event_util` 는 빈 테이블이어야 하고(BJ/skin/lua/SkinLuaAccessor.java:95-101), 최상위에서 `main_state.xxx` 를 호출하면 안 된다. ModernChic 은 이 조건을 지킨다.
3. 헤더 값은 Lua 테이블 그대로 JsonSkin 구조체에 리플렉션 매핑된다. 숫자는 `int` 필드에 truncation(소수점 버림, 반올림 아님), `String` 필드에 숫자가 오면 `tojstring()`(property 의 `category` = 1 -> "1"), `boolean` 필드에 숫자 `0` 이 오면 true(Lua 에서 0 은 truthy)다(BJ/skin/lua/LuaSkinLoader.java:97-151). offset 헤더 `{a = 0}` 은 "alpha 축 허용" 이라는 뜻이다.
4. 한 화면 스킨이 `dofile(skin_config.get_path(...))` 로 하위 Lua 10개 안팎을 읽고, 각각을 `pcall` 로 감싸서 실패하면 조용히 그 부분만 빠진다(result.lua:46-172). 즉 런타임에 `luajava`, `Gdx` 같은 심이 없으면 로그만 남고 mainmenu(점수 패널 전체)가 통째로 사라질 수 있다. 샌드박스는 `dofile`, `io.open`(읽기/쓰기/추가, 체인 호출 `io.open(p,"a"):write(s):close()` 포함), `os.date`, `luajava.bindClass/new`, `Gdx.input:isKeyPressed` 까지 최소 심이 필요하다(8장).
5. 값(number)은 이미지 장수로 렌더 방식이 갈린다. 10장 = 공백 패딩, 11장 = 11번째 이미지를 패딩에 고정, 24장 = 부호 있는 +/- 두 세트. 특히 `zeropadding` 필드는 10/11장 시트에서는 무시되고(JsonSkinObjectLoader.java:132-154) 24장 시트에서만 쓰인다(5.3 절).
6. `act` 는 함수/숫자 둘 다 온다. 클릭 판정은 "그려지고 있는(draw=true) 가장 위 객체의 destination 사각형" 이며 좌/우 버튼 인자는 +1/-1 이다(BJ/skin/SkinObject.java:672-704, Skin.java:394-411). Lua 함수 `act` 는 `narg()` 가 1 이므로 인자 1개로 호출된다. 숫자 `act` 는 `EventFactory.getEvent(id)`; 370/371 은 알려지지 않은 id 라 사실상 무동작이다.
7. `draw = function` 은 부수효과 있는 관용구로 쓰인다. mainmenu.lua:984-996 과 irmenu.lua:652-664 는 0x0 짜리 `-110`(BLACK) destination 의 `draw` 안에서 `Gdx.input:isKeyPressed` 로 화살표키를 읽고 메뉴 상태 변수를 바꾼다(`return nil` 이므로 그려지지는 않는다). `draw` 는 해당 객체에 대해 매 프레임 평가되어야 한다.
8. `timer = timer_util.timer_observe_boolean(f)` 는 "f() 가 true 가 된 순간 on, false 가 되면 off" 인 타이머이고, destination 시간은 그 on 시각 기준 상대 시간이다(TimerUtility.java:89-111). 결과 메인 메뉴 1/2 전환, IR 상태 표시가 전부 이것에 의존한다.
9. `loop` 의미(SkinObject.java:355-372): `loop=0`(기본)은 첫 프레임부터 전체 반복, `loop=-1` 은 1회 재생 후 사라짐, `loop=N`(N>0) 은 마지막 키프레임 시간까지 간 뒤 N~마지막 구간만 반복(N == 마지막 시간이면 마지막 프레임 정지). `acc` 는 객체 전체에 하나(첫 비0 값)이고 1=제곱 가속, 2=감속, 3=계단(보간 없음).
10. 텍스트 정렬 기준점: align CENTER 이면 `region.x` 가 가운데, RIGHT 이면 `region.x` 가 오른쪽 끝, LEFT 이면 왼쪽 끝이다. 글자 높이는 `region.height / font.size` 로 스케일되고 상단 기준(y + h)으로 그려진다. Decide 의 제목(x=960, w=1720)이 대표 사례다.
11. 결과 화면 상태 의존이 매우 넓다: option id 90/91, 300-307, 320-327, 330-332, 150-155, 160-164/1160/1161, 180-184, 191, 196-204/1196-1204, 280-282/290/-289, 51, -606; number id 약 55개; string id 20여개; timer 2 와 172-174. 전수는 7장.
12. 같은 숫자 id 가 사용처에 따라 다른 속성이다(BJ IntegerPropertyFactory.java:984-1004): 380-389 는 값이면 IR 순위별 EXSCORE, 이미지 인덱스(imageset `ref`)면 플레이어 타입; 390-399 는 값이면 IR 순위 번호, imageset `ref` 면 클리어 타입. 370(`CLEAR`)도 값(숫자)과 이미지/imageset 인덱스(클리어 타입 0..10)로 둘 다 쓰인다.
13. filepath 의 `def`(예 "#default", "sample", "yuki")는 엔진 로더가 아니라 설정 UI 쪽에서 적용된다(BJ/launcher/SkinConfigurationView.java:404-418, BJ/config/SkinConfiguration.java:327-340). 설정이 없으면 `getPath`(BJ/skin/SkinLoader.java:95-130)는 와일드카드 디렉터리에서 무작위로 한 파일을 고른다. R-BMS 는 `def` 를 자체 적용해야 하며(파일명 또는 확장자 제외 이름의 대소문자 무시 일치, 없으면 Random), 그렇지 않으면 char/isclear/clear 에서 투명 `#default.png` 와 `yuki.png` 중 무작위가 뽑힌다.
14. 시각 동일성이 걸린 비자명 사실: (a) `a` 가 아닌 `offsets` alpha 는 단일 키프레임/정지 구간에서만 적용된다(prepareColor 의 조기 return, SkinObject.java:481-515); (b) 음수 `w`(-246) 는 좌우 반전 그래프(gra_fastRate)에 쓰인다; (c) 미완성 소수 좌표(481.5, 805.5, -960.0)는 Java int 로 truncation; (d) replay 아이콘 destination 이 for-i 루프 안에 있어 3중으로 그려진다(mainmenu.lua:662-687); (e) `gra_*Rate` 값은 early+late=0 이면 0/0=NaN.
15. 라이선스: Result/readme.txt 는 "본 스킨 자체의 2차 배포 금지(허가 시 제외), 개조판 공개 시 원작자 KASAKO 표기 필수", 주1 로 Result/parts/rank 의 `#default`, `Damage`, `Formal`, `Underworld` 는 자유 사용 대상에서 제외한다고 적는다. 즉 기본 번들에 그대로 넣을 수 없는 이미지가 기본 선택(#default)이다(15장, 17장 미결).
16. 용량 지배 요소: Result/parts/bg 39.9MB(clearType 25.1MB), Decide/font 46MB 중 fnt 약 36MB(비트맵 폰트, 옵션 시만 필요), Decide/bg/movie 18.5MB(동영상, 옵션 시만 필요), ttf 10.4MB(Decide 와 Result 가 바이트 동일 2종). 기본 옵션으로 실제 필요한 자산은 약 20MB 이하다(14장).
17. 결정 화면의 상태 요구는 작다: option 150-155, number 96(PLAYLEVEL), string 10-15, 1003, 스테이지파일(-100), 노트 분포/BPM 그래프 객체, timer 2(FADEOUT). 결정 화면부터 정확히 맞추는 것이 가장 싸고 시각 검증이 쉽다.

---

## 2. 실행 모델

### 2.1 진입점과 헤더 모드

- MC/decide.luaskin, result.luaskin, course.luaskin 은 모두 같은 4줄이다. `local t = require("<decide|result|course>")` 후 `if skin_config then return t.main() else return t.header end`. 근거: MC/decide.luaskin:1-5, MC/result.luaskin:1-5, MC/course.luaskin:1-5.
- `require("decide")` 는 MC/decide.lua(skin 루트)로 해석되어야 한다. `Result.lua.base` 같은 모듈명은 점을 `/` 로 바꿔 skin 루트 기준 `Result/lua/base.lua` 로 해석한다(경로 템플릿 `<skin>/?.lua;<skin>/?/init.lua`, SkinLuaAccessor.java:855-871 의 의미). 파일 `result.lua` 와 폴더 `Result/` 가 대소문자만 달라 대소문자 구분 FS 에서도 `require("result")` 는 파일을 가리킨다.
- 헤더 모드에서 실행되는 최상위 코드: decide.lua:8-10(`main_state = require`, `PROPERTY = require(...)`, `header = require(...)`), result.lua:8-12, course.lua:8-12. 여기서 `Root.version`(4.6), `Root.author`("KASAKO") 를 읽고 `PROPERTY.property/filepath/offset/category` 로 헤더 테이블을 만든다. `main_state` 호출은 없다.
- 본 모드 `main()` 은 `Root.define`, `Root.define2`, `config` 를 `require` 하고(decide.lua:227-229, result.lua:16-18) 전역 `MAIN`, `CUSTOM`, `CONFIG`, `RESULT_BASE`(result/course 만), `PROPERTY`, `main_state`, `timer_util` 을 만든다. 이후 `CUSTOM.LOAD_HEADER(skin, header)` 로 헤더 필드를 스킨 테이블에 복사한다(Root/define2.lua:7-9).

### 2.2 로드 시점에 실행되는 부수효과 (require 체인)

`Root.define2`(Root/define2.lua:22-30) 가 `Root.customoption/customnumber/customgraph/customslider/customfunction/customtime/customtext/customsound/customtimer` 를 전부 require 한다. 이 중 로드 즉시 실행되는 것들.

| 위치 | 로드 시 실행되는 동작 | 필요한 런타임 |
|---|---|---|
| Root/customfunction.lua:7-8 | `require("luajava")`, `luajava.bindClass("java.io.File")` | luajava 심, `bindClass("java.io.File")` 가 값을 반환 |
| Root/customtime.lua:6,8,10 | `os.date("%y%m%d")`, `os.date("%Y/%m/%d")`, `os.date("%H:%M:%S")` | `os.date` |
| Root/customnumber.lua:289-303 | `FUNC.countFileRecords(skin_config.get_path(path), n)` 9회: io/Play/sp,dp/lanecover/pathList.txt, excludeList.txt, History/<yymmdd>/{history,clear,score,miss}.txt, History/impression.txt | `skin_config.get_path`, `io.open(path,"r")` 이 없는 파일에 대해 에러가 아닌 `nil` 반환, 있으면 `f:lines()`, `f:close()` |
| Root/customsound.lua 전체(모듈 로드) | 함수 정의만(로드시 오디오 재생 없음). 단 result.lua:173-175 에서 `CONFIG.voice.result.sw`(기본 false)면 `CUSTOM.SOUND.resultVoice()` 실행 | `main_state.audio_play` |
| result.lua:41, course.lua:41 | `CONFIG.infoOutput`(config.lua:243, 기본 true) 이면 `CUSTOM.FUNC.infoOutput(7|15)` -> History/information.txt 를 `io.open(path,"w")` 로 쓴다(Root/customfunction.lua:319-370) | `io.open` 쓰기 |
| Result textproperty.lua:20-25 | `main_state.text(FULLTITLE/ARTIST/GENRE/TABLE_FULL/PLAYER)`, `main_state.number(TIME_YEAR/MONTH/DAY)` 를 문자열 상수로 굳힘 | 로드 시점 라이브 상태 |
| Decide textproperty.lua:35 | `diffOutlineColor()` 가 `main_state.option(150..155)` 를 읽음 | 로드 시점 option |
| Decide textproperty.lua:53,71 | `CUSTOM.TEXT.choiceTips()`(Root/customtext.lua:8-33) 가 `math.random` 으로 22개 팁 중 하나를 고름 | `math.random` |
| decide.lua:44-46,52, 145 | `CUSTOM.NUM.randNum(0,255)` 3회(=`math.random`), `CUSTOM.NUM.diffRGB()` 2회 | `math.random`, option 150-155 중 정확히 하나가 true 여야 함 |
| result.lua:111, course.lua:99 | `main_state.option(MAIN.OP.ONLINE)` (51) | option 51 |
| background.lua:320 | beam 색 `createColorNum()` 이 `main_state.option(90/91)` 와 `math.random` 을 사용 | option 90/91 |

`diffRGB()` 는 150-155 중 어느 것도 true 가 아니면 `nil` 을 반환하고 곧바로 `RGB[1]` 인덱싱에서 Lua 에러가 난다(Root/customnumber.lua:217-232, decide.lua:166). 이때 decide 화면 스킨 전체 로드가 실패한다. R-BMS 의 결정 화면 상태는 곡 난이도가 없을 때도 option 150(DIFFICULTY0)을 true 로 줘야 한다.

### 2.3 전역 변수와 상태 수명

- 스크립트가 쓰는 전역: `main_state`, `timer_util`(result/course 만), `PROPERTY`, `MAIN`, `CUSTOM`, `CONFIG`, `RESULT_BASE`, `DEBUG`(미정의 nil). decide 의 `PROPERTY` 와 result 의 `PROPERTY` 는 서로 다른 모듈이므로 Lua 상태(Globals)를 화면 로드마다 새로 만들거나 최소한 전역을 초기화해야 한다. `Root.customoption.isCharDisplayOn`(Root/customoption.lua:61)은 모듈 레벨 가변 boolean 이고 `Root.customnumber` 의 `CUSTOM.sectionScore` 도 모듈 상태다.
- mainmenu.lua 의 `isInfoMenu1/isInfoMenu2`, irmenu.lua 의 `isIrRanking/isIrClear/screenHidden`, impression/diflist 의 `sw/flg/favbtn/num/inputNumber/difBtn` 는 `dofile(...).load()` 호출마다 새로 만들어지는 로컬(upvalue)이다. 화면 진입마다 초기값으로 리셋된다.

### 2.4 로드 시점에 굳는 라이브 값 목록 (스냅샷)

| 값 | 위치 | 내용 |
|---|---|---|
| bottomResult | Result textproperty.lua:20 | `FULLTITLE .. " / " .. ARTIST .. " / " .. GENRE .. " / " .. TABLE_FULL` |
| bottomCourse | textproperty.lua:21 | `FULLTITLE`(코스 이름) |
| playerName | textproperty.lua:22 | `"Player : " .. main_state.text(PLAYER)` |
| dete | textproperty.lua:25 | `"<year> / <month> / <day>"` (정수 연결, 0 패딩 없음) |
| magnification | textproperty.lua:6-14 | 옵션에 따라 "+-225ms"/"+-150ms"/"+-75ms", 코스에선 nil |
| Decide level 색, 텍스트 색 | decide.lua:145 | `diffRGB()` |
| Decide tips | Decide textproperty.lua:53 | 임의 팁 1개 |
| Decide stage | textproperty.lua:50-52 | `"STAGE" .. todaySongUpdateCount + 1` (연산자 우선순위상 `..` 보다 `+` 가 먼저) |
| beam 색 | background.lua:320 | clear 시 r 50-100, g 50-100, b 150-255 / fail 시 r 150-255, g 50-100, b 50-100 |
| lockon 시작색 | decide.lua:44-46 | 매 로드 무작위 RGB 3값(20개 destination 공용) |
| isCourse() | background.lua:146, prepare.lua:59 | `COURSE1_TITLE ~= ""` 로 코스 여부 판정 |
| 클리어 타입 상수 | (없음) | clear 값은 draw 시점에 `ref=370` 로 읽힘(굳지 않음) |

### 2.5 pcall 격리 구조 (result.lua, course.lua)

모든 하위 파트는 `local status, parts = pcall(function() return dofile(path).load(...) end); if status and parts then CUSTOM.ADD_ALL(...)` 형식이다. 실패하면 로그 없이 조용히 생략된다. 조합 순서(= destination 의 z 순서)는 11.3 절.

---

## 3. 헤더 (1)

### 3.1 공통 키

| 키 | Decide (Decide/lua/require/header.lua:7-19) | Result (Result/lua/require/header.lua:9-23) | Course |
|---|---|---|---|
| type | 6 | 7 | 15 (`load(15)`, course.lua:12) |
| name | "ModernChicDecide-4.6" | "ModernChicResult-4.6" | 동일 |
| w, h | 1920, 1080 | 1920, 1080 | 동일 |
| fadeout | 1000 | 1000 | 1000 |
| scene | 3000 | 3600000 | 3600000 |
| input | 500 | 2500 | 2500 |
| author | "KASAKO" | "KASAKO" | 동일 |
| property | 6개 | 20개 | 17개 (3.3 표) |
| filepath | 2개 | 45개 | 25개 (ClearType 20개 제외) |
| offset | 없음 | 2개 | 2개 |
| category | 3개 | 16개 | 12개 |

의미(엔진 쪽, BJ/decide/MusicDecide.java:34-60, BJ/result/MusicResult.java:168-173): `input` 은 STARTINPUT 타이머(1) 가 켜지는 시각(ms), `scene` 은 자동 진행 시각, `fadeout` 은 FADEOUT 타이머(2) 켜진 뒤 다음 화면으로 넘어가기까지의 시간. 결과 화면은 scene 이 사실상 무제한(1시간), 입력 2.5초 뒤에 키 입력으로 나간다. 입력 허용 시각 2500 은 시작 애니메이션 길이(prepare.lua 2.5초)와 일치한다.

property 가 `{name, def, category, item = {{name, op}...}}` 이므로 `skin_config.option[name]` 은 선택된 item 의 `op` 값(int)이다. `PROPERTY.isXxx()` 는 전부 `skin_config.option[부모이름] == op` 이다(Decide property.lua:44, Result property.lua:46).

### 3.2 Decide property (Decide/lua/require/property.lua:73-121)

op 번호는 899 에서 시작해 자식 하나당 +1(property.lua:9,19-25). category 번호는 부모/filepath 를 만들 때 +1.

| category label | name (skin_config.option 키) | item(name = op) | def |
|---|---|---|---|
| 1 | 背景の種類 | 静止画 = 900 / 動画 = 901 | 静止画 |
| 2 | 画像フォント | 無効 = 902 / 有効（高負荷） = 903 | 無効 |
| 3 | ステージファイル | 無効 = 904 / 有効 = 905 | 有効 |
| 4 | ノーツ分布グラフ | 無効 = 906 / 有効 = 907 | 有効 |
| 5 | ステージ数表示 | 無効 = 908 / 有効 = 909 | 無効 |
| 6 | お役立ち情報表示 | 無効 = 910 / 有効 = 911 | 有効 |

filepath (property.lua:92-93, 123-126):

| category | name | path | def |
|---|---|---|---|
| 7 | 背景（静止画）Decide/bg/image/*.png | Decide/bg/image/*.png | sample |
| 8 | 背景（動画）Decide/bg/movie/*.mp4 | Decide/bg/movie/*.mp4 | sample |

category (property.lua:132-148): "メインオプション" = {2, 3, 4, 6}, "背景パターン" = {1, 7, 8}, "ステージ数表示（ModernChicResultスキンの使用と『プレイ履歴の保存』を有効にする必要があります。）" = {5}.

offset: 없음. category/item 값이 숫자 라벨이므로 문자열 "1", "2", ... 로 변환되어 property/filepath 의 `category` 와 문자열로 매칭된다(JSONSkinLoader.java:154-175).

### 3.3 Result property (Result/lua/require/property.lua:75-504)

op 는 900 부터 순서대로, category label 은 1 부터. 아래는 호출 순서로 계산한 확정값.

| label | name | item = op | def | 결과 | 코스 |
|---|---|---|---|---|---|
| 1 | 画像フォント | 無効 900 / 有効（高負荷） 901 | 無効 | O | O |
| 2 | グラフ&スコア表示位置 | 左 902 / 右 903 | 左 | O | O |
| 3 | 項目表示切り替え | コンボ数表示 904 / ミスカウント表示 905 | コンボ数表示 | O | O |
| 4 | 開始時アニメーション | 無効 906 / 有効 907 | 有効 | O | O |
| 5 | ステージファイル表示 | 無効 908 / 有効 909 | 無効 | O | X |
| 6 | タイムスタンプ | 無効 910 / 有効 911 | 有効 | O | O |
| 7 | 修飾（ビーム） | 無効 912 / 有効 913 | 有効 | O | O |
| 8 | 修飾（リング） | 無効 914 / 有効 915 | 有効 | O | O |
| 9 | 背景表示パターン | Clear or Failed 916 / ALL 917 / Rank 918 / ClearType 919 | Clear or Failed | O(4) | O(3, ClearType 없음) |
| 10 | キャラクター表示 | 無効 920 / 有効 921 | 有効 | O | O |
| 11 | キャラクター表示パターン | Clear or Failed 922 / ALL 923 / Rank 924 / ClearType 925 | Clear or Failed | O(4) | O(3) |
| 12 | IRメニュー表示 | 無効 926 / 有効（オンライン時のみ表示されます） 927 | 有効 | O | O |
| 13 | IRメニューの種類 | IRランキングTOP10 928 / IRクリア状況 929 | IRランキングTOP10 | O | O |
| 14 | タイミンググラフ倍率 | 低倍率（+-225ms） 930 / 標準倍率（+-150ms） 931 / 高倍率（+-75ms） 932 | 標準 | O | X |
| 15 | タイミンググラフ配色パターン | 通常 933 / 赤基調 934 / 緑基調 935 / 青基調 936 | 通常 | O | X |
| 16 | クリアランクイメージ表示パターン | フェードアウト 937 / 一定間隔表示 938 | フェードアウト | O | O |
| 17 | プレイ履歴の保存 | 無効 939 / 有効（ModernChic/History内に保存されます） 940 | 無効 | O | O |
| 18 | NOPLAYからのランプ更新保存 | 無効 941 / 有効 942 | 無効 | O | O |
| 19 | キャラクターローテーション | 無効 943 / 有効（選択したキャラクターは無視されます） 944 | 無効 | O | O |
| 20 | 背景ローテーション | 無効 945 / 有効（選択した背景は無視されます） 946 | 無効 | O | O |

주: 코스에서는 표의 X 항목 option 이름이 `skin_config.option` 에 없다 -> `PROPERTY.isStageFileOn()` 등은 `nil == 909` = false 가 된다(property.lua:5 의 함수는 존재하지만 항상 false). `isTDGMagnification*` 도 전부 false 라서 mainmenu.lua:16 의 `m.graphMagnification` 이 nil -> `width = nil` -> 엔진 기본 301(JsonSkin.TimingDistributionGraph.width=301).

property 배열 선언 순서: 1,2,3,12,13,4,6,7,8,10,16,17,18,19,20 순으로 먼저(property.lua:202-264), 이어서 결과는 9,11,5,14,15 를, 코스는 9,11 을 추가(property.lua:376-418, 481-497).

filepath (property.lua:144-194, 266-293, 420-440). label 은 21 부터. 전부 `def = "#default"` 이고 예외 두 개만 `def = "yuki"`.

| label | name | path | def |
|---|---|---|---|
| 21 | クリアランクイメージ | Result/parts/rank/*.png | #default |
| 22 | IRカバー | Result/parts/irmask/*.png | #default |
| 23 | ゲージ | Result/parts/gauge/*.png | #default |
| 24 | 背景（Clear） | Result/parts/bg/isclear/clear/*.png | #default |
| 25 | 背景（Failed） | Result/parts/bg/isclear/failed/*.png | #default |
| 26 | 背景（ALL） | Result/parts/bg/all/*.png | #default |
| 27-34 | 背景（AAA/AA/A/B/C/D/E/F） | Result/parts/bg/rank/<AAA..F>/*.png | #default |
| 35-44 (결과만) | 背景（Failed）(2번째), （Assist）, （LaAssist）, （Easy）, （Normal and NoPlay）, （Hard）, （Exhard）, （FullCombo）, （Perfect）, （Max） | Result/parts/bg/clearType/<failed,assist,laassist,easy,normal,hard,exhard,fullcombo,perfect,max>/*.png | #default |
| 45 | キャラクター（Clear） | Result/parts/char/isclear/clear/*.png | yuki |
| 46 | キャラクター（Failed） | Result/parts/char/isclear/failed/*.png | yuki |
| 47 | キャラクター（ALL） | Result/parts/char/all/*.png | #default |
| 48-55 | キャラクター（AAA..F） | Result/parts/char/rank/<AAA..F>/*.png | #default |
| 56-65 (결과만) | キャラクター（Failed）(2번째), Assist, LaAssist, Easy, Normal and NoPlay, Hard, Exhard, FullCombo, Perfect, Max | Result/parts/char/clearType/<...>/*.png | #default |

주의: 이름 "背景（Failed）" 와 "キャラクター（Failed）" 가 clear/failed 용과 clearType/failed 용으로 각각 두 번 나온다(`background.failed` 와 `background.failed2`). 설정 저장이 이름 키(`FilePath.name`)이므로 이름이 같은 두 항목은 같은 값을 공유하는 UI 위험이 있다(엔진도 동일 문제). 구현자는 이름이 아니라 path 를 기본 키로 삼는 편이 안전하다.

label/op 번호는 결과와 코스에서 동일하다. `background.failed2` 등 ClearType 용 `customoption.filepath(...)` 호출(property.lua:161-170, 185-194)은 `isResult` 와 무관하게 항상 실행되어 label 을 소비하고, `isResult` 일 때만 `module.filepath`/`module.category` 에 등록된다. 따라서 코스에서도 label 35-44, 56-65 는 비어 있는 번호이고 offset label 은 66, 67 그대로다. 마찬가지로 `bgPattern`, `charPattern`, `stageFile`, `TDG*` 의 op 번호(916-936)도 코스에서 그대로 소비되며, 코스에 등록되지 않은 property 이름은 `skin_config.option` 에 없다.

offset (property.lua:196-199, 296-299). label 은 filepath 다음 순번 66, 67(코스도 동일).

| 이름 | id | 허용 축 | 의미 |
|---|---|---|---|
| 背景の明るさ 0~255 (255で真っ暗になります) | 40 | a (값 0 이 true) | 배경 위 BLACK(-110) 덧칠 alpha 오프셋(background.lua:153-157) |
| キャラクター表示位置調整 | 41 | x, y | 캐릭터 겹침 이미지 이동(background.lua:182, 204, 240, 279) |

offset id 는 40 부터(property.lua:8 `offsetNumber = 39`). 결과 offset 으로 쓰이는 id 는 `PROPERTY.offsetBgBrightness.num`(=40), `PROPERTY.offsetCharPosition.num`(=41) 뿐이다.

category (결과, 최종 순서; property.lua:304-374 에 `table.insert(module.category, 7, ...)`, `12`, append 2개 적용):

1. メインオプション = {1, 2, 3, 4, 6, 7, 8, 23}
2. プレイ履歴保存機能 = {17, 18}
3. クリアランクイメージ = {16, 21}
4. 背景パターン = {9, 20, 66}
5. 背景選択（Clear or Failed） = {24, 25}
6. 背景選択（ALL） = {26}
7. 背景選択（ClearType）[결과만] = {35..44}
8. 背景選択（Rank） = {27..34}
9. キャラクター表示 = {10, 11, 19, 67}
10. キャラクター選択（Clear or Failed） = {45, 46}
11. キャラクター選択（ALL） = {47}
12. キャラクター選択（ClearType）[결과만] = {56..65}
13. キャラクター選択（Rank） = {48..55}
14. IRメニュー = {12, 13, 22}
15. ステージファイル表示[결과만] = {5}
16. グラフ関連[결과만] = {14, 15}

코스 category 는 위 목록에서 7번, 12번, 15번, 16번이 없다(총 12개). 코스의 property 는 앞의 15개 + bgPattern(3 item) + charPattern(3 item) = 17개다.

주: 이 category 설계는 label 번호가 각 property/filepath/offset 의 `category` 에 중복 없이 하나씩만 매칭되는 전제다.

### 3.4 헤더 의미론 정리 (구현 체크리스트)

- `skin_config.option` 키는 property `name` 문자열(일본어), 값은 선택된 item 의 `op`. 기본값은 `def` 와 이름이 일치하는 item 의 op(없으면 첫 item).
- `skin_config.file_path` 는 `FilePath.name -> 선택된 파일명`. `skin_config.get_path(rel)` 는 `<skin dir>/rel` 에 선택된 파일을 적용한 실제 경로 문자열을 반환한다(LuaSkinLoader.java:85-87, SkinLuaAccessor.java:937-941). 와일드카드가 없는 경로(`Result/lua/background.lua`)는 그대로 돌려준다. 디렉터리 경로(`Result/parts/bg/all/`)는 와일드카드가 없으므로 그대로 돌려준다(listchoice.lua, 회전 옵션 전용).
- 결과 스킨은 `skin_config.offset[name].a/x/y/w/h/r` 도 있지만 ModernChic 결정/결과는 `offsets = {id}` 숫자 참조만 쓴다(`.num`).

---

## 4. 스킨 테이블 구성 (2) — 기본 옵션 기준 손 계산

### 4.1 Decide (decide.lua:225-254) — 기본 옵션(静止画/無効/ステージファイル有効/ノーツ分布有効/ステージ数無効/お役立ち有効)

| 테이블 | 개수 | 내용 |
|---|---|---|
| source | 4 | id 1 = Decide/bg/image/*.png, id 2 = Decide/bg/movie/*.mp4, id 3 = Decide/parts/parts.png, id 4 = Decide/parts/parts2.png (decide.lua:234-239). id 가 숫자이므로 문자열 "1".."4". |
| font | 2 | ttf: id 1 mgenplus-1c-medium.ttf, id 0 mgenplus-1c-black.ttf (textproperty.lua:39-42). 비트맵 옵션이면 id 0 main.fnt, id 1 sub.fnt 에 `type = 1`(textproperty.lua:57-60) |
| text | 6 | `tablename&tablelevel`(ref 1003, font 0, size 35), `genre`(ref 13, font 1, size 40), `title`(ref 12, font 0, size 90), `artist`(font 1, size 40, value 함수), `stage`(font 1, size 50, value 함수), `tips`(font 1, size 25, constantText). 전부 `overflow = 1`, `align = 1` |
| image | 12 | bg 1, lockon-lu/ru/ld/rd 4, getready 1, 난이도 라벨 6 (beginner/normal/hyper/another/insane/unknown) |
| value | 6 | levelBeginner/Normal/Hyper/Another/Insane/Unknown (ref 96) |
| judgegraph | 1 | notes-graph (`noGap = 0, type = 0`) |
| bpmgraph | 1 | bpmgraph (id 만) |
| destination | 69 | bg 1, stagefile 1, 검정 띠 1, lockon 20 (4 모서리 x 5 반복), 텍스트 42 (6 난이도 x [tablename, genre, title, artist, 카테고리 라벨, 레벨, tips]), 그래프 2, 페이드아웃 2 |

변형: `ステージ数表示 = 有効` 면 난이도당 `stage` 가 +1(총 +6), `お役立ち情報表示 = 無効` 면 난이도당 `tips` 가 빠져 -6, `ステージファイル = 無効` 면 -1, `ノーツ分布グラフ = 無効` 면 judgegraph/bpmgraph 와 destination 2개 빠짐. 이 4개 키 외에 skin 테이블에 `imageset`, `slider`, `graph`, `gauge`, `gaugegraph`, `customEvents`, `customTimers` 는 없다.

### 4.2 Result (result.lua:14-177) — 기본 옵션, 오프라인(`ONLINE` false), 일반 결과

| 테이블 | 개수 | 내용 |
|---|---|---|
| source | 13 | base 9개(Result/parts/ring.png id 0, irmask/*.png id 1, system.png id 2, prepare.png id 3, number.png id 4, lamp.png id 5, parts.png id 6, rank/*.png id 7, gauge/*.png id 8; base.lua:25-35) + 배경 2(id "bgClear","bgFailed") + 캐릭터 2(id "charClear","charFailed") |
| font | 2 | ttf: id 0 black, id 1 medium (Result textproperty.lua:29-32). 비트맵: id 0 main.fnt, id 1 sub.fnt(`type` 없음) |
| text | 29 | 기본 9 + course1..10 + irRankName1..10 |
| image | 손 계산 125 | 배경 계열 6 + centerinfo 18 + impression 16 + mainmenu 77 + prepare 5 + diflist 1 + fadeout 2 (IR/history 제외) |
| imageset | 0 | (ClearType 배경/캐릭터 패턴 또는 IR 사용 시에만) |
| value | 약 44 | centerinfo 7 + impression 2 + mainmenu 35 |
| slider | 0 | (IR 사용 시 1) |
| graph | 2 | gra_fastRate, gra_slowRate (IR 사용 시 +11) |
| gauge | 1 | `{id = 2001, parts = 50, nodes = 36개}` (mainmenu.lua:87-109) 단일 객체 대입(`skin.gauge = parts.gauge`, result.lua:99) |
| gaugegraph | 1 | grooveGaugeGraph |
| judgegraph | 3 | judgesGraph, notesGraph, fsGraph |
| bpmgraph | 1 | bpmgraph |
| timingdistributiongraph | 1 | timingdistributiongraph |
| customTimers / customEvents | 0 / 0 | 빈 테이블 선언만(result.lua:38-39) |
| destination | 손 계산 194 | 배경 7 + 하단 바 2 + centerinfo 26 + impression 18 + mainmenu 132 + prepare 5 + diflist 1 + fadeout 3 |

손 계산 근거: mainmenu 132 = 안내 2 + 그래프 프레임 1 + judgesGraph 1 + gaugegraph 1 + assistInfo 1 + 랭크 이미지 8 + 정보/옵션/총노트 4 + 게이트 3 + 메뉴1(프레임 1, EXSCORE 5, 항목 4, PG 1, GR~MS 15, 타이밍 카운터 5) + 메뉴2(프레임 1, 판정레벨 5, 리플레이 36, TOTAL 3, 그래프 계열 11) + 메뉴 버튼 2 + 이번 랭크 8 + 랭크 갱신 1 + 클리어 타입 1 + 램프 갱신 1 + 장식 10 + 키보드 더미 1.

### 4.3 Course (course.lua:14-151) — 기본 옵션

source 13(배경/캐릭터 2+2, 위와 동일), text 29, destination 손 계산 175(= 배경 7 + 하단 2 + centerinfo 19 + mainmenu 138 + prepare 6 + fadeout 3), image 손 계산 106(= 배경 6 + centerinfo 16 + mainmenu 77 + prepare 5 + fadeout 2), value 44. 차이는 14장.

### 4.4 IR 메뉴가 켜졌을 때(온라인) 추가분 (irmenu.lua)

image 30, imageset 10(`clearTypeIr1..10`), value 68, graph 11(`s_barmax` ... `s_barnoplay`), slider 1(`scrollBar`, `type = 8` IR_POSITION, `range = 655`, `angle = 2`, `changeable = true`). 일반 결과에서 destination 약 130개.

---

## 5. 객체 기능 사용 목록 (3)

### 5.1 destination 필드 (실사용만)

| 필드 | 사용 | 대표 위치 |
|---|---|---|
| id | 문자열 id, 숫자 id(`MAIN.IMAGE.STAGEFILE=-100`, `BLACK=-110`), 문자열 숫자("2001" gauge) | decide.lua:28,36 / mainmenu.lua:440 |
| time, x, y, w, h | 키프레임. 첫 키프레임에서 빠진 time/x/y/w/h/acc/angle = 0, a/r/g/b = 255, 이후 키프레임은 직전 값 상속(JsonSkinObjectLoader.java:702-735) | 전체 |
| a, r, g, b | 알파/틴트 | decide.lua:37,60 / impression.lua:251 |
| angle | 키프레임 `angle = 360`(링 회전) | background.lua:315 |
| acc | 1 가속(lockon, decide.lua:51), 2 감속(prepare.lua:43,53,64, irmenu.lua:556) | decide.lua:60-82 |
| loop | 0(미지정), -1, 양수 | decide.lua:28,36,59 / prepare.lua:24 |
| timer | 숫자(FADEOUT=2, IR_CONNECT_*=172/173/174) 또는 함수(`timer_util.timer_observe_boolean`) | decide.lua:113 / mainmenu.lua:468 / irmenu.lua:270 |
| blend | `MAIN.BLEND.ADDITION`(2) 만 사용 | background.lua:313,322 / mainmenu.lua:954 / irmenu.lua:555 |
| op | 정수 배열(음수 포함) | decide.lua:165 / centerinfo.lua:143 / irmenu.lua:318 |
| draw | Lua 함수 | 6장 |
| offsets | `{40}`, `{41}` (복수형 배열) | background.lua:154,182,204,240,279 |
| mouseRect | `{x=0,y=0,w=234,h=43}` | mainmenu.lua:884 / irmenu.lua:400,527 |
| stretch | 활성 코드에는 없음(mainmenu.lua:782-790 은 주석 처리된 블록 안) | - |
| center, filter, offset(단수), clip_* | 사용 안 함 | - |

destination 의 정수 필드는 Java `int` 로 truncation 된다. 예: `RESULT_BASE.MAIN_POS_X + 535 - (107 / 2)` = 35+535-53.5 = 516.5 -> 516, `RESULT_BASE.CENTER_POS_X + ((520 - 309) / 2)` = 805.5 -> 805, `0 - (1920 / 2)` = -960.0 -> -960. 음수 너비 `w = -246`(mainmenu.lua:604) 도 정상값이다.

### 5.2 객체 필드 (실사용만)

| 객체 | 필드 | 사용 |
|---|---|---|
| image | id, src, x, y, w, h, divx, divy | 전 이미지 |
| image | cycle | mainInfo(9000), clearType(192), scoreUpdate(200), lampUpdate/rankUpdate(100), irMyPositionFrame(100), irFailed/irExhard(50), irFullCombo/irPerfect(50), screenShot(200) |
| image | len + ref | clearType/clearType2(len 11, ref 370/371), preClearType(len 11, ref 370), chartBtn(len 3, ref 90), useOption1P/2P(len 10, ref 42/43) |
| image | act | 함수: mainMenu, charswitch, difBtn, calculatorswitch, favbtn_off/on, number0-9, calculatorenter, next1/next2, saveSearchUrlBtn, screenShot, irTop10LabelRect, irClearLabelRect. 숫자: clearType(370), clearType2(371), preClearType(370), chartBtn(90), useOption1P/2P(42/43) |
| image | timer, click | 사용 안 함 |
| imageset | id, ref, images | bgClearType, charClearType(ref 370), clearTypeIr1..10(ref 390..399) |
| value | id, src, x, y, w, h, divx, divy, digit, align, ref, value(함수), zeropadding | 5.3 |
| value | divy | 2(mainExscoreDiff 등 12x2=24장 세트), 3(irEliteRank) |
| value | cycle | irEliteRank(50) |
| value | padding, space, offset, timer | 사용 안 함 |
| text | id, font, size, ref, align, overflow, constantText, value(함수), shadowOffsetX/Y, outlineColor, outlineWidth | 5.4 |
| text | wrapping, editable, event, shadowColor, shadowSmoothness, outlineColor(ttf) | 사용 안 함 |
| slider | id, src, x, y, w, h, type, range, angle, changeable | scrollBar 하나(irmenu.lua:263) |
| slider | isRefNum, min, max, value, event | 사용 안 함 |
| graph | id, src, x, y, w, h, angle, value(함수) | gra_fastRate/slowRate, s_bar* |
| graph | type, isRefNum, min, max | 사용 안 함 (type 기본 0 이므로 value 함수 분기로 감) |
| gauge | id, parts, nodes | 결과 게이지 (type, range, cycle, starttime, endtime 는 JsonSkin 기본값 0, 3, 33, 0, 500) |
| gaugegraph | 색 필드 14개 | 10.1 |
| judgegraph | id, noGap, orderReverse, type, backTexOff | 10.2 |
| bpmgraph | id | 기본값 |
| timingdistributiongraph | width, lineWidth, 색 9개, drawAverage, drawDev | 10.3 |
| decide judgegraph | id, noGap, type | decide.lua:95 |

### 5.3 value 이미지 시트 유형 (JsonSkinObjectLoader.java:99-154 기준)

장수 = divx * divy.

| 장수 | 처리 | ModernChic 사용 |
|---|---|---|
| 24의 배수 | 12장씩 두 세트: 앞 12 = 양수(0-9, 10, 11 = "+"), 뒤 12 = 음수(0-9, 10, 11 = "-"). `value.zeropadding` 사용. 음수/부호 있는 diff 숫자 | mainExscoreDiff, mainComboDiff, numMisscountDiff, exscoreIrDiff, irRankDiff (divx 12 x divy 2) |
| 10의 배수 | 10장(0-9). zeropadding 인자 = `value.padding`(기본 0) -> 빈 칸 | numExscoreRate(divx 10), totalNotes, remainNotes, numGauge, numGaugeAfterdot, irMyRank(10), irTotalPlayer, exscoreIr, s_*rate, Decide level(10) |
| 그 외(11 등) | 11장(0-9 + 11번째). zeropadding 인자 고정 2 = 앞쪽 빈 자리를 11번째 이미지로 채움. Lua 의 `zeropadding = ON` 은 무시 | mainExscore, mainCombo, numMisscount, numPG..numMS, *_ER/_SL, timingFast/SlowNum, numTOTAL, numRefTOTAL, stddev*, aveTim*, year..sec, indexIr*, s_*num |

자릿수 렌더(BJ/skin/SkinNumber.java:130-205): 각 자리 폭 = destination w, 자리 수 = `digit`. align 0(RIGHT) 은 고정 폭 오른쪽 정렬, 1(LEFT) 은 빈 자리만큼 왼쪽으로 이동, 2(CENTER) 는 반만 이동. 값이 `Integer.MIN_VALUE`(-2147483648) 면 그리지 않는다(irmenu.lua:241,484 가 이 값에 대비). 24장 세트는 양수일 때 앞에 "+", 음수일 때 "-" 를 붙인다(0 은 "+0").

### 5.4 텍스트 (Decide textproperty.lua:37-72, Result textproperty.lua:27-79)

- ttf(기본): shadowOffsetX/Y = 4(제목류 `title`, Decide 의 main) / 2(그 외). ttf 쪽 shadowColor 는 쓰이지 않고, 그림자는 "글자 색 RGB 를 1/2 한 같은 알파"다(SkinTextFont.java:167-173).
- 비트맵(옵션): Decide 는 `outlineColor = diffOutlineColor()`(문자열 "165423ff" 등 8자리 RGBA 16진), `outlineWidth = 1`(tips 는 outlineWidth 없음). `type = 1` 인 fnt 는 거리장(distance field) 방식으로 보이며(BJ SkinTextBitmap.java:177-185 의 셰이더 유니폼 u_outlineDistance), outline 은 이 폰트에서만 의미가 있다. Result 의 비트맵 설정에는 outline/shadow 가 없다.
- 공통: overflow 1 = SHRINK(가로로 압축), Result 의 `T_OVERFLOW.SHRINK` 도 1. `align` 은 T_ALIGN(0 LEFT, 1 CENTER, 2 RIGHT).
- text 객체 id 목록은 6장/7장.

---

## 6. Lua 함수가 값으로 들어가는 자리 전수 (4)

표기: R = main_state 읽기, W = 부수효과.

### 6.1 Decide

| 자리 | 위치 | 동작 |
|---|---|---|
| text `artist` value | Decide textproperty.lua:47-49, 65-67 | R: `text(SUBARTIST)`, `text(ARTIST)`. 서브아티스트가 빈 문자열이면 ARTIST, 아니면 `ARTIST .. " " .. SUBARTIST` |
| text `stage` value | textproperty.lua:50-52, 68-70 | R 없음. `"STAGE" .. CUSTOM.NUM.todaySongUpdateCount + 1` 를 매 프레임 계산(값은 로드 시점 고정) |
| (draw/timer/act/customTimers/customEvents) | - | 없음 |

### 6.2 Result / Course

| 자리 | 위치 | 읽는 main_state / 부수효과 |
|---|---|---|
| value `remainNotes` | centerinfo.lua:102-104 | R: number 74, 110-114 (TOTALNOTES - (PG+GR+GD+BD+PR)) |
| value `timingFastNum`/`timingSlowNum` | mainmenu.lua:246-251 | R: number 423 / 424 |
| value `numRefTOTAL` | mainmenu.lua:255-257 | R: number 74 (CUSTOM.NUM.calcTotal, Root/customnumber.lua:158-171) |
| value `impnum`,`impcount`,`simpnum`,`bimpnum`,`eimpnum`,`songLimit`,`bgaLimit`,`etcLimit` | impression.lua:89,132,219-225,367 | R 없음(지역 상태, CONFIG, CUSTOM.NUM.impressionCount) |
| value `exscoreIrDiff1..10` | irmenu.lua:238-247 | R: number 380..389, `CUSTOM.NUM.myScoreBest()`(number 170, 171) |
| value `irRankDiff` | irmenu.lua:253-257 | R: timer 172/173, number 179/182 |
| graph `gra_fastRate`/`gra_slowRate` | mainmenu.lua:302-308 | R: number 423/424. FastRate = early/(late+early), SlowRate = late/(late+early). 합이 0 이면 NaN |
| graph `s_bar*`(IR) | irmenu.lua:538-548 | R: number 224/225...(IR 비율). 값 `-2147483648` 이면 0, 아니면 `rate / 100` |
| draw `char*` | background.lua:180-182, 204-206, 238-240, 279-281 | R: option 90/91 또는 300-307 + `CUSTOM.OP.isCharDisplayOn` (모듈 변수) |
| draw `remainNotesFrame`/`remainNotes` | centerinfo.lua:107-115 | R: `CUSTOM.OP.isInTheMiddleFailed()` -> option 91, number 74, 110-114 |
| draw `mainInfo` / `mainInfo2` | mainmenu.lua:317-326 | R: text 150 가 빈 문자열 여부 |
| draw `assistInfo` | mainmenu.lua:360-362 | R: `isAssistOn()` -> event_index of buttons 301, 303, 305, 307, 302, 304, 306 (`== 1`) |
| draw `useOption2P` | mainmenu.lua:425-427 | R: option 162, 163, 1161 |
| draw `numTOTAL`(2개) | mainmenu.lua:692-701 | R: number 368 과 `calcTotal()` 비교(>= 파랑, < 빨강) |
| draw `course_1-5`, `course1..5`, `course_6-10`, `course6..10` | mainmenu.lua:808-871 | R: text 155(COURSE6_TITLE) 빈지 여부 |
| draw `rankUpdate` | mainmenu.lua:917-921 | R: number 371, option 320-327(bestRank), 300-307(nowRank). `(bestRank < nowRank) and (prev ~= 0)` |
| draw `lampUpdate` | mainmenu.lua:936-943 | R: number 371, 370. `prev < now and prev ~= 0` |
| draw (BLACK 0x0, 키보드) | mainmenu.lua:984-996 | W: `Gdx.input:isKeyPressed(Keys.RIGHT/LEFT)` 로 isInfoMenu1/2 를 설정. 반환 nil(안 그림) |
| draw `difBtn` | diflist.lua:34-36 | 지역 boolean(`difBtn`) 반환. 클릭하면 false 가 되어 사라짐 |
| draw `calculatorbody` 외 impression 전부 | impression.lua:91-372 | 지역 boolean 조합(`flg and sw`, ...) |
| draw (IR) `irMyPositionFrame`, `irRankName*`, `exscoreIr*`, `exscoreIrDiff*`, `irRankDiff`, `irMyPositionFrame2`, `irTop10Cover` | irmenu.lua:360-495, 635 | R: number 379+i, 389+i, 179, text 119+i("YOU"), 소지역 상태, `screenHidden.get` |
| draw (IR) BLACK 0x0 | irmenu.lua:652-664 | W: UP/DOWN 키로 isIrClear/isIrRanking 설정 |
| timer `timer_observe_boolean(function() return isInfoMenu1 end)` 등 | mainmenu.lua 24곳, irmenu.lua 약 60곳 | 지역 boolean/IR 상태 함수 관찰 |
| timer 숫자 | fadeout.lua:16,23,30(`MAIN.TIMER.FADEOUT`=2), irmenu.lua:270,276,282,369(172,173,174,173) | 엔진 타이머 |
| act `infoMenuSwitch` | mainmenu.lua:121-123 | W: isInfoMenu1/2 토글, `CUSTOM.SOUND.menuChangeSound()` -> `main_state.audio_play("<skin>/Root/sounds/change.ogg")` |
| act `charswitch` | centerinfo.lua:162-164 | W: `CUSTOM.OP.isCharDisplayOn` 토글 |
| act `difBtn` | diflist.lua:29-33 | W: 지역 토글, 파일 쓰기(`difflist.txt`), `calculatorChangeSound()` |
| act `calculatorswitch` | impression.lua:23-26 | W: `sw` 토글, 소리 |
| act `favbtn_off/on` | impression.lua:33-38 | W: `favbtn` 토글 |
| act `number0..9`, `snum/bnum/enum` | impression.lua:46-66, 145-216 | W: 입력 버퍼 갱신, `clickSound()` |
| act `calculatorenter`/`next1`/`next2` | impression.lua:69-87, 313-353 | W: `History/impression.txt` 에 추가 쓰기, `enterSound()` |
| act `saveSearchUrlBtn` | history.lua:55 | W: `History/search.html` 쓰기 |
| act `screenShot`, `irTop10LabelRect`, `irClearLabelRect` | irmenu.lua:141,155-157,164-166 | W: IR 커버/메뉴 토글 |
| customTimers/customEvents | result.lua:38-39 | 비어 있음 |

---

## 7. 참조하는 속성 id 전수 (5)

이름은 MC/Root/main*.lua 의 이름이고 값은 BJ SkinProperty 와 일치함을 확인한 것(370, 371, 171, 172, 179, 380-399, 90, 191, 330, 150-155 등)이다.

### 7.1 number (value `ref`, `main_state.number`, `main_state.event_index`)

| id | 이름 | 사용 |
|---|---|---|
| 96 | PLAYLEVEL | Decide 레벨 숫자 6개; diflist.lua(JSON 문자열) |
| 74 | TOTALNOTES | totalNotes, remainNotes, calcTotal |
| 102, 103 | SCORE_RATE, SCORE_RATE_AFTERDOT | numExscoreRate, numExscoreRateAfterdot |
| 105 | MAXCOMBO2 | mainCombo |
| 107, 407 | GROOVEGAUGE, GROOVEGAUGE_AFTERDOT | numGauge, numGaugeAfterdot |
| 110-114 | PERFECT, GREAT, GOOD, BAD, POOR | numPG..numPR, isInTheMiddleFailed, remainNotes |
| 412, 413 | EARLY_GREAT, LATE_GREAT | numGR_ER, numGR_SL |
| 414, 415 | EARLY_GOOD, LATE_GOOD | numGD_ER/SL |
| 416, 417 | EARLY_BAD, LATE_BAD | numBD_ER/SL |
| 418, 419 | EARLY_POOR, LATE_POOR | numPR_ER/SL |
| 420, 421, 422 | MISS, EARLY_MISS, LATE_MISS | numMS, numMS_ER, numMS_SL |
| 423, 424 | TOTALEARLY, TOTALLATE | timingFastNum/SlowNum, gra_*Rate |
| 170 | HIGHSCORE2 | CUSTOM.OP.isNotFirstPlay, myScoreBest |
| 171 | SCORE3 | mainExscore, myScoreBest |
| 172 | DIFF_HIGHSCORE2 | mainExscoreDiff, history.lua:128 |
| 176 | TARGET_MISSCOUNT | history.lua:114 만 |
| 173, 174 | TARGET_MAXCOMBO, MAXCOMBO3 | 정의만(사용 안 함) |
| 175 | DIFF_MAXCOMBO | mainComboDiff |
| 177 | MISSCOUNT2 | numMisscount, history.lua:115 |
| 178 | DIFF_MISSCOUNT | numMisscountDiff, history.lua:107 |
| 368 | SONGGAUGE_TOTAL | numTOTAL, 비교 |
| 370 | CLEAR | clearType, preClearType, bg/char ClearType imageset(ref), mybestClearType, rampConverter, history |
| 371 | TARGET_CLEAR | clearType2(미배치), rankUpdate, lampUpdate, history |
| 374-377 | AVERAGE_TIMING(+AFTERDOT), STDDEV_TIMING(+AFTERDOT) | aveTimRate*, stddevRate* |
| 21-26 | TIME_YEAR..SECOND | year..sec(미배치), dete(YEAR, MONTH, DAY) |
| 179, 182 | IR_RANK, IR_PREVRANK | IR 전용 |
| 180 | IR_TOTALPLAYER | IR 전용 |
| 202-242 | IR_PLAYER_*(count, rate, afterdot) | IR 클리어 상황 |
| 271 | RIVAL_SCORE | isYouWin 정의만 |
| 380-389 | RANKING1..10_EXSCORE(값) | IR |
| 390-399 | RANKING1..10_INDEX(값) / RANKING*_CLEAR(imageset ref) | IR |

### 7.2 option (destination `op`, `main_state.option`, `draw`)

| id | 이름 | 사용 |
|---|---|---|
| 150-155 | DIFFICULTY0..5 | Decide 난이도별 세트 6개, diffRGB, 비트맵 outline 색, centerinfo 난이도 라벨 |
| 160-164, 1160, 1161 | SONG7KEY, SONG5KEY, SONG14KEY, SONG10KEY, SONG9KEY, SONG24KEY, SONG24KEYDP | centerinfo 모드 라벨 7개, useOption2P |
| 51 | ONLINE | result.lua:111 (로드 시) |
| 90, 91 | RESULT_CLEAR, RESULT_FAIL | 배경/캐릭터 CF, fadeout 글자, prepare(코스), beam 색, isInTheMiddleFailed |
| 300-307 | RESULT_AAA_1P..F_1P | 랭크 배경/캐릭터, 랭크 이미지, thisTime, nowRank |
| 320-327 | BEST_AAA_1P..F_1P | bestRank |
| 330, 331, 332 | UPDATE_SCORE, UPDATE_MAXCOMBO, UPDATE_MISSCOUNT | UPDATE 도장 |
| 180-184 | JUDGE_VERYHARD..VERYEASY | 판정 레벨 이미지 |
| 196, 197, 198 / 1196-1198 / 1199-1201 / 1202-1204 | NO_REPLAYDATA/REPLAYDATA/REPLAYDATA_SAVED (1-4번 슬롯) | 리플레이 아이콘 |
| 191 | STAGEFILE | centerinfo 스테이지 파일(옵션 ON 일 때) |
| 280, 281, 282, -289, 290 | COURSE_STAGE1..3, -COURSE_STAGE_FINAL, MODE_COURSE | nextStageFrame, course(i+1) 텍스트 (centerinfo.lua:143,148) |
| -606 | -IR_WAITING | IR 순위 표시 |
| 625 | COMPARE_RIVAL | 정의만(isYouWin) |
| 1330 이상 | DRAW_* | 사용 안 함 |

### 7.3 timer

| id | 이름 | 사용 |
|---|---|---|
| 2 | FADEOUT | Decide 페이드아웃(decide.lua:113,120), Result fadeout.lua:16,23,30 |
| 172, 173, 174 | IR_CONNECT_BEGIN, SUCCESS, FAIL | IR 상태 표시, `isTimerOff/On` |

`main_state.timer(id) == main_state.timer_off_value`(Long.MIN_VALUE) 로 on/off 를 판단한다(Root/customoption.lua:44-50). 사용자 정의 타이머 id(`MAIN.TIMER`, `CUSTOMTIMER_ID` 9999~)는 이 화면들에서는 쓰이지 않는다.

### 7.4 event / button id (`act` 숫자, imageset/이미지 `ref`)

| id | 이름 | 의미 |
|---|---|---|
| 42, 43 | BUTTON_RANDOM_1P/2P | image 인덱스: 결과에서는 리플레이 데이터의 randomoption(0..9) / randomoption2. 클릭 이벤트는 MusicSelector 에서만 동작하므로 결과에서는 무동작 |
| 90 | BUTTON_FAVORITTE_CHART | image 인덱스 0(없음) 1(즐겨찾기) 2(invisible). 클릭 시 결과 화면에서도 `changeFav` 로 즐겨찾기 상태 순환(EventFactory.java:562-612) |
| 370, 371 | NUM.CLEAR / TARGET_CLEAR | `act` 로 쓰였지만 이벤트로는 존재하지 않음(무동작) |
| 301-307 | ASSIST_* | `main_state.event_index` 로 assistInfo 표시 조건 |

### 7.5 string

| id | 이름 | 사용 |
|---|---|---|
| 2 | PLAYER | playerName |
| 10 | TITLE | impression, diflist |
| 12 | FULLTITLE | Decide title, bottomResult/bottomCourse, history |
| 13 | GENRE | Decide genre, bottomResult |
| 14, 15 | ARTIST, SUBARTIST | Decide artist, bottomResult |
| 120-129 | RANKING1..10_NAME | IR 이름 텍스트 irRankName1..10, `main_state.text(119+i)` |
| 150-159 | COURSE1..10_TITLE | course1..10 텍스트, isCourse(150), COURSE6_TITLE(155) |
| 1003 | TABLE_FULL | Decide tablename&tablelevel, bottomResult |
| 1020 | IR_NAME | repositoryname |
| 1030, 1031 | SONG_HASH_MD5, SONG_HASH_SHA256 | diflist 파일 쓰기 |
| 1010 | VERSION | infoOutput(select)에서만 |

### 7.6 slider type, graph type, 이미지 특수 id

- slider `type = 8`(`MAIN.SLIDER.IR_POSITION`) 하나(IR 스크롤바).
- graph 는 `value` 함수만 사용하므로 `MAIN.GRAPH.*` 숫자 id 는 쓰이지 않는다. gauge 타입 번호(`gauge_type`)도 이 화면에서는 쓰이지 않는다.
- destination 이미지 특수 id: -100(STAGEFILE), -110(BLACK). 음수 id 는 `SkinImage(-id)` 로 처리된다(JSONSkinLoader.java:313-322).

---

## 8. Lua 런타임 의존 (6)

### 8.1 main_state

사용 함수: `option(id)`, `number(id)`, `text(id)`, `event_index(id)`, `timer(id)`, `time()`(인자 있어도 무시되는 0 인자 함수, MainStatePropertyLuaApiExporter.java:`TimeFunction extends ZeroArgFunction`), `timer_off_value`, `audio_play(path[, volume])`. 이 화면들에서 호출되지 않는 것: `float_number`, `gauge`, `gauge_type`, `judge`, `rate`, `exscore`, `volume_*`, `offset`, `set_timer`, `event_exec`, `key_pressed`, `numbers`(Root 의 play 전용 함수 안에만 존재).

`audio_play` 호출은 두 형태: 1인자(`change.ogg`, `click.ogg`, `enter.ogg` 등, 볼륨 기본 1) 와 2인자(결과 보이스, 볼륨 2). 로드 시점 호출(`resultVoice`)은 기본 꺼져 있다(config.lua:92-94).

### 8.2 skin_config

`skin_config.get_path(string)`, `skin_config.option[name]`. (`skin_config.file_path`, `skin_config.offset`, `skin_config.enabled_options` 는 이 화면 코드에서 직접 쓰이지 않는다. 단 property.lua 의 `offset` 헬퍼는 `skin_config.offset[name].a` 를 읽는 함수를 만들지만 호출되지 않는다.)

### 8.3 timer_util / event_util / luajava / 표준 라이브러리

- `timer_util.timer_observe_boolean` 만 사용. `event_util` 는 사용하지 않는다.
- `require("luajava")`: Root/customfunction.lua:7, mainmenu.lua:5, irmenu.lua:6. 사용 표면: `luajava.bindClass("java.io.File")`, `luajava.new(File, path)` + `:mkdir()` + `:listFiles()`(`#t`, `t[i]`, `tostring`), `luajava.bindClass("com.badlogic.gdx.Gdx")`, `luajava.bindClass("com.badlogic.gdx.Input")`, `Gdx.input:isKeyPressed(code)`, `Input.Keys.RIGHT/LEFT/UP/DOWN`(libGDX 키코드 22/21/19/20). 포크의 facade(LegacySkinLuaApi.java:51-64, 295-340)는 정확히 이 표면만 허용한다.
- `io.open(path, "r"|"w"|"a")`, `f:lines()`, `f:write`, `f:close()`, 체인 `io.open(p,"a"):write(s):close()`(listchoice.lua:44 등), `string.sub/gsub/match`, `tostring`, `table.insert`, `ipairs`, `pairs`, `print`, `pcall`, `dofile`, `require`, `math.random`, `os.date`.
- 사용하지 않는 것: `package`, `debug`(포크는 `debug.getmetatable` 만 남김), `loadfile`, `loadstring`, `setfenv`, `unpack`, `string.gfind`, `table.getn`, `bit32`, `coroutine`, `os.getenv/execute/exit/remove/rename`. 결정/결과 범위 파일에서 LuaJ 5.1 전용 문법은 발견되지 않았다(전체 스킨에서 `unpack` 은 Play/lua/sp/detailinfo/util.lua:7 의 사용자 정의 함수 이름일 뿐이다).
- Lua 5.4 와의 차이로 문제될 수 있는 것: (1) `1920 / 2` 같은 나눗셈이 정수가 아니라 float(960.0) -> int 필드는 truncation; (2) 숫자 -> 문자열 변환은 `"STAGE" .. 3`(정수) 안전, float 연결(`..` 에 float)은 이 화면에서 쓰이지 않음; (3) `#` 연산자는 contiguous 배열에만 사용; (4) `math.random(m, n)` 인자 순서 정상.
- 샌드박스 요구 정리(최소): `dofile`(skin 폴더 한정, 상대/절대), `require` 모듈 해석, `io.open`(skin 폴더 한정, 읽기 실패 시 nil 반환), `os.date`, `math.random`, 위 `luajava` 최소 facade. 파일 기록은 기본 옵션에서 `infoOutput`(History/information.txt)만 항상 발생한다.

### 8.3.1 `luajava` 없이 제거하거나 대체할 수 있는 부분

- Root/customfunction.lua 의 File/mkdir/listFiles 는 회전(rotation) 옵션과 history 에서만 호출된다. 모듈 로드 시 `bindClass("java.io.File")` 만 성공하면 된다.
- mainmenu.lua:5-7 과 irmenu.lua:6-8 의 `Gdx`/`input` 은 화살표키 메뉴 전환 하나를 위해서다. 이 키 전환을 R-BMS 입력 시스템으로 옮기면 해당 destination(BLACK 0x0 객체)은 필요 없다.

---

## 9. 1920x1080 레이아웃 (7)

공통: 좌표 = (x, y, w, h), y 는 아래가 0. 사각형은 [x, x+w] x [y, y+h].

### 9.1 Decide

z 순서(= destination 순서)와 규칙:

1. `bg`: (0,0,1920,1080). 기본 `Decide/bg/image/sample.png`(청록/주황 HUD 풍 1920x1080). `PROPERTY.isBgImage()` 면 src 1(`*.png`), 아니면 src 2(`*.mp4` 영상)(decide.lua:14-18).
2. 스테이지 파일(-100): 옵션 ON 일 때, `loop = 1500`, 키프레임 (500: 910,490,1,1) -> (1500: 640,300,640,480). 500ms 에 점에서 시작해 1.0초 동안 4:3 사각형으로 커지고 이후 정지(decide.lua:26-33).
3. 검정 띠(-110): `loop = 500`, (0: 960,585,1,1, a=200) -> (500: 0,200,1920,680). 반투명(a=200) 이므로 스테이지 파일이 비쳐 보인다(decide.lua:35-40).
4. lockon 모서리 5세트 x 4(decide.lua:43-89): 각 destination `loop = loopTime`, 키프레임 3개.
   - 세트 k(0..4): startTime = 200k, loopTime = 1000 + 100k, `acc = 1`.
   - 키프레임 0 (time = startTime): 모서리별 (lu: 0,580 / ru: 1420,580 / ld: 0,0 / rd: 1420,0), 크기 500x500, a = 30, r/g/b = 무작위 3값(로드 시 한 번).
   - 키프레임 1 (time = loopTime): 모서리별 (lu: 627,743 / ru: 1243,743 / ld: 627,288 / rd: 1243,288), 크기 50x50, a = 255, r/g/b = `diffRGB()`.
   - 키프레임 2 (time = loopTime + 100): a = 0. 이후 `loop` 규칙에 따라 마지막 100ms 구간(255->0, 제곱 가속)이 무한 반복되어 깜박인다.
   - 이미지: parts2.png 의 (0,50),(60,50),(120,50),(180,50) 각 50x50.
5. 텍스트와 레벨(난이도 6세트 중 해당 `op` 하나만 그려짐). 모두 `loop = 500`, 키프레임 (0: a=0) -> (500: a=255). 앵커 x=960 중심(center), 너비 1720.

| 요소 | 사각형 | 색 | 비고 |
|---|---|---|---|
| stage(옵션) | (960,710->700,1720,50) | 난이도 색 | a 0->255 와 동시에 y +10 -> 0 |
| tablename&tablelevel | (960,815,1720,35) | 난이도 색 | font 0(black), size 35 |
| genre | (960,610,1720,40) | 흰색 | font 1(medium), size 40 |
| title | (960,490,1720,90) | 난이도 색 | font 0(black), size 90, 그림자 4px |
| artist | (960,430,1720,40) | 흰색 | font 1, size 40, 그림자 2px |
| 카테고리 라벨 | (840,370,240,45) | - | parts.png (0, 558/603/648/693/738/783, 240, 45) |
| 레벨 숫자(2자리) | (890,270,71,93) | - | parts.png 행 y = 0/93/186/279/372/465, 폭 71 x 10자 |
| tips(옵션) | (960,220,1720,25) | (150,150,150) | font 1, size 25 |

난이도 색(`diffRGB`): beginner(op 151) = (6,255,0), normal(152) = (18,210,215), hyper(153) = (255,192,0), another(154) = (255,0,0), insane(155) = (148,44,150), unknown(150) = (195,195,195). 레벨 시트 행 순서도 같은 순서(초록, 청록, 주황, 빨강, 보라, 회색)이다. 비트맵 폰트 외곽선 색: "165423ff", "105e60ff", "644f0fff", "521313ff", "401340ff", "444444ff".
   참고: 5번째 라벨 이미지(insane)는 시트에서 "ANOTHER"(보라) 글자로 되어 있다(원작 시트의 특성).

6. 노트 분포 그래프(옵션): judgegraph `notes-graph` (460,30,1000,150) + `bpmgraph` 같은 사각형(decide.lua:91-108). 띠(y 200-880) 아래쪽이다.
7. 페이드아웃(`timer = 2`): BLACK `loop = 1000` (0: 0,540,1920,0) -> (500: 동일) -> (1000: y=0, h=1080), 즉 500-1000ms 에 가운데에서 위아래로 열려 검정 화면을 만든다. `getready`(parts2.png (0,0,730,50)) 는 500ms 부터 (598,514,730,50) 에서 a 0/255 깜박(600:0, 700:255, 800:0, 850:255, 900:0, 950:255), `loop = 1000`(마지막 키프레임 950 < loop 이므로 마지막 값 유지).

시간선: scene 3000ms 후 자동으로 FADEOUT 타이머가 켜지고 fadeout 1000ms 후 플레이로 진행. 입력은 500ms 이후 허용(BJ/decide/MusicDecide.java:34-60).

### 9.2 Result — 좌표 계산 기준

`RESULT_BASE`(Result/lua/base.lua:7-23): 기본(왼쪽 메인) `MAIN_POS_X = 35`, `SUB_POS_X = 1220`, `SCROLLBAR_POS_X = 1885`; 오른쪽 메인이면 `MAIN_POS_X = 1220`, `SUB_POS_X = 35`, `SCROLLBAR_POS_X = 7`. `CENTER_POS_X = 700` 은 고정. 아래 표는 `M = MAIN_POS_X`(기본 35) 로 쓴다.

### 9.3 Result — z 순서와 영역별 좌표 (일반 결과, 기본 옵션)

1. 배경 파트(background.lua)
   - bgClear(op 90) / bgFailed(op 91): (0,0,1920,1080), 이미지는 각 `*.png` 선택 파일 전체(1920x1080).
   - 밝기 BLACK(-110): `offsets = {40}`, (0,0,1920,1080, a=0) + offset 40 의 alpha.
   - ring(옵션 ON): `blend = 2(ADD)`, (0,0,1920,1080, a=30) -> (20000: angle=360) 반복. 이미지 ring.png(1920x1080).
   - beam(옵션 ON): `blend = 2`, (0,0,1920,30, 틴트, a=200) -> (5000: y=540, a=0) -> (6500). 이미지 system.png (0,1778,960,30). 위로 올라가며 사라지는 6.5초 주기.
   - 캐릭터(옵션 ON, 기본 CF): charClear(op 90 + 토글) / charFailed(op 91 + 토글): `offsets = {41}`, `loop = 200`, (0: -960,-540,3840,2160) -> (200: 0,0,1920,1080). 입력 시 2배에서 정상 크기로 줄어듦.
2. 하단 바: BLACK (0,0,1920,50), `bottomResult` 텍스트 (960,10,1600,25) CENTER, font 0 size 30, SHRINK.
3. centerinfo (CENTER_POS_X C = 700, 폭 520):

| 요소 | 사각형 | 이미지/값 |
|---|---|---|
| diffFrame | (C, 50, 520, 60) | parts.png (0,320,520,60) |
| 키 모드 라벨(op 161/160/163/162/164/1160/1161) | (C+40, 58, 190, 40) | parts.png (0, 40..280 step 40, 190, 40); 순서 5keys, 7keys, 10keys, 14keys, 9keys, 24keys, 48keys |
| 난이도 라벨(op 150..155) | (C+250, 58, 250, 40) | parts.png (190, 0..200 step 40, 250, 40); 순서 unknown, beginner, normal, hyper, another, insane |
| dete 텍스트 | (C+260=960, tsY+65, 520, 30) CENTER | tsY = 910(결과) / 970(코스) |
| playerName 텍스트 | (960, tsY+25, 520, 30) CENTER | |
| 스테이지 파일(-100, op 191, 옵션) | (820, 120, 280, 210) | 기본 꺼짐 |
| remainNotesFrame | (C-3=697, 115, 526, 126) | parts.png (0,600,526,126) |
| remainNotes 숫자 | (C+237=937, 162, 30, 36) 5자리, a 0->150->255 | number.png (440,276,310,36) x10 |
| chartBtn | (795, 1010, 330, 64) | parts.png (0,852,330,192) divy 3, ref 90 (3 프레임) |
| nextStageFrame (op 280/281/282, -289, 290, 90 모두 true) x3 | (697, 115, 526, 126) | parts.png (0,726,526,126) |
| course(i+1) 텍스트 | (C+10+250=960, 145, 500, 30) CENTER, a 200->255 | text ref 151/152/153 |
| charswitch | (1800, 0, 60, 50), a 255 -> 100(2s) -> 255(4s) | parts.png (560,650,60,50) |

   `chartBtn` 과 `nextStageFrame`(+ course(i+1) 텍스트)은 일반 결과(`flg == 0`)에서만 만들어지고, 코스 결과에는 없다.
4. impression(계산기, 기본 type 0 총점): 본체 (805,340,309,554) parts.png (700,0,309,554); 스위치 (0,0,60,50) parts.png (560,600,60,50); `draw` = `flg and sw`(sw 기본 false = config.lua:136) 이므로 기본 화면에서는 스위치 버튼만 보인다. 키 10개 (805+x, 340+y, 75, 95) x: 21/118/214, y: 20(0), 124(1,2,3), 228(4,5,6), 333(7,8,9) 영역이 투명 1x1 이미지(parts.png (0,0,1,1))에 `act` 로 클릭을 받는다. enter (929,360,172,95). 숫자 표시 (855,797,44,60) number.png (0,504,440,60). 저장 후 `calculatorbody_saved` + impcount.
5. mainmenu (좌측 패널 M = 35, 폭 665):

| 요소 | 사각형 | 비고 |
|---|---|---|
| mainInfo(op: 코스 아님) | (M, 1024, 665, 50) | system.png (0,0,665,150) divy 3 cycle 9000 (3초마다 줄 전환). 코스면 mainInfo2 (0,100,665,50) 고정 |
| mainGraphFrame | (M, 659, 665, 356) | system.png (0,160,665,356) |
| judgesGraph(일반만) | (M+5, 774, 655, 236) | |
| grooveGaugeGraph | (M+5, 774, 655, 236) | 10.1 |
| assistInfo | (M+5, 970, 655, 40) | loop 5000, 4000->5000 에서 a 0. 어시스트 옵션이 켜진 때만 |
| 클리어 랭크 이미지 8종(op 300-307) | 기본(페이드아웃) (M+60, 800, 400, 504->168), a 0 -> 255 | loop -1, 250ms 에 높이 504 -> 168, 4000 정지, 5000 에 소멸. rank 시트 400x1344(8행 x 168) |
| infoFrame | (M+8, 727, 649, 44) | parts.png (0,530,649,44) |
| useOption1P | (M+26, 726, 175, 40) | parts.png (520,0,175,400) divy 10, ref 42 |
| useOption2P(14/10/24DP) | (M+211, 726, 175, 40) | ref 43 |
| totalNotes 숫자 | (M+373, 730, 31, 36) 5자리 | number.png (440,96,310,36) x10 |
| 게이지 2001 | (M+21, 674, 400, 35) | 노드 폭 8 x 50개 |
| numGauge | (M+409, 674, 44, 36) 3자리 | number.png (0,96,440,36) x10 |
| numGaugeAfterdot | (M+559, 674, 44, 36) 1자리 | number.png (0,132,440,36) x10 |
| mainJudgeFrame(메뉴1) | (M, 70, 665, 582) | system.png (0,520,665,582), 타이머 = isInfoMenu1 |
| mainExscore | (M+222, 545, 28, 36) 5자리 | |
| mainExscoreDiff | (M+380, 535, 22, 28) 6자리(+/-) | |
| numExscoreRate | (M+380, 563, 22, 28) 3자리 | |
| numExscoreRateAfterdot | (M+455, 563, 16, 23) 2자리 | |
| scoreUpdate(op 330) | (M+535, 545, 107, 36) | loop 3000, 2500->3000 에 2배 -> 1배 |
| 항목행 y=480 | wdCombo (M, 467, 210, 60) / mainCombo (M+222, 480, 28, 36) / mainComboDiff (M+380, 485, 22, 28) / scoreUpdate(op 331) (M+535, 480, 107, 36) 또는 wdMisscount/numMisscount/numMisscountDiff/scoreUpdate(op 332) | 옵션 `項目表示切り替え` |
| numPG | (M+222, 415, 28, 36) | 타이머 = isInfoMenu1 |
| numGR/GD/BD/PR/MS | y = 350, 285, 220, 155, 90. 값 (M+222, y, 28, 36), `_SL` (M+390, y, 28, 36) 4자리, `_ER` (M+530, y, 28, 36) 4자리 | 전부 isInfoMenu1 |
| timingGraphFrame | (M+390, 400, 250, 60) | system.png (0,1910,250,60) |
| gra_slowRate | (M+392, 427, 246, 26) | system.png (0,1996,246,26), angle 0, 왼쪽에서 오른쪽 |
| gra_fastRate | (M+638, 427, -246, 26) | system.png (0,1970,246,26), 음수 폭: 오른쪽에서 왼쪽 |
| timingSlowNum / timingFastNum | (M+415, 430, 16, 20) / (M+555, 430, 16, 20) | 4자리 |
| mainJudgeFrame2(메뉴2) | (M, 70, 665, 582) | system.png (2000,0,665,582), 타이머 = isInfoMenu2 |
| 판정 레벨 이미지 5종 | (M+180, 530, 230, 60), 타이머 = isInfoMenu2 | system.png (2000, 590/650/710/770/830, 230, 60); op 184 VERYEASY, 183 EASY, 182 NORMAL, 181 HARD, 180 VERYHARD 순 |
| 리플레이 아이콘 4 | x = M+415+58j, y=540, 53x46 | 3상태 x 4슬롯 x **3중 루프**; saved 만 a 255 -> 80 -> 255 |
| numTOTAL | (M+187, 480, 28, 36), 파랑 (83,180,248) / 빨강 (248,83,83) | draw 로 2개 중 하나 |
| numRefTOTAL | (M+500, 480, 28, 36) | |
| graphFrame(일반) | (M, 78, 670, 385) | system.png (2000,1750,670,385) |
| notesGraph / bpmgraph | (M+16, 341, 633, 88) | |
| fsGraph | (M+16, 211, 633, 88) | |
| timingdistributiongraph | (M+16, 81, 633, 88) | |
| timFS | (M, 140, 652, 25, a=200) | system.png (2000,2135,652,25) |
| magnification 텍스트 | (M+120, 177, 100, 18) LEFT | size 18 |
| stddevRate / afterdot | (M+354, 179, 15, 18) / (M+396, 179, 15, 18) 2자리 | |
| aveTimRate / afterdot | (M+567, 179, 15, 18) / (M+609, 179, 15, 18) 2자리 | |
| mainMenu | (M+11, 601, 234, 43) | system.png (2000,890,234,43), `act = infoMenuSwitch` |
| mainMenuRect | 같은 사각형 | system.png (2000,933,234,43), 마우스가 올라가 있을 때만 보이는 강조(mouseRect) |
| thisTime 랭크 8종 | (M+510, 601, 142, 43) | system.png (0, 1110+43k, 142, 43) |
| rankUpdate | (M+499, 585, 165, 74) | system.png (924,1630,165,148) divy 2 cycle 100 |
| clearType | (M+261, 601, 234, 43) | system.png (150,1110,702,473) divx 3 divy 11 len 11 cycle 192 ref 370 |
| lampUpdate | (M+252, 585, 254, 74) | system.png (670,1630,254,148) divy 2 cycle 100 |
| lampGreen 장식 x 8(메뉴1) | (M+4, y, 38, 76), y = 525/460/395/330/265/200/135/70, 지연 100ms 씩 | lamp.png (0,760,38,76), ADD, a 255 -> 50 -> 255 (4000ms 주기) |
| lampGreen 장식 x 2(메뉴2) | y = 525, 460 | 같은 방식 |
| 키보드 더미 | (0,0,0,0) BLACK | 8.3 |

   주: y 가 `y = M-기준 상수 + 오프셋` 이 아니라 모두 절대 y 이다. `M` 은 x 에만 더해진다.
6. IR 메뉴(온라인일 때만, 7장 이후 요약): SUB_POS_X(1220) 에 순위 프레임 (1220,806,665,210), 목록 프레임 (1220,70,665,714), 10행 y = 677 - 65i(i=0..9), 스크롤바 (1885,70,28,714) 등. 온라인이 아니거나 옵션이 꺼져 있으면 아무 것도 그려지지 않는다(result.lua:111).
7. prepare(시작 애니메이션, 기본 켜짐): (프레임선 2 + 반투명 띠 + 파도 + 클리어타입 글자). 12.2 절.
8. diflist 버튼: (1860, 0, 60, 50) parts.png (560,750,60,50), a 255 -> 100(2s) -> 255(4s), `draw = difBtn`(클릭 후 사라짐). charswitch 는 (1800,0,60,50), calculatorswitch 는 (0,0,60,50).
9. fadeout: BLACK `timer = 2`, `loop = 1000`: (0: 0,540,1920,0) -> (500: y=0, h=1080) -> (1000). 글자 finishClear (op 90)/finishFailed (op 91) (375,513,1200,50), a 0 -> 255 (300->600ms), `loop = -1`; 이미지 system.png (0,1808,1200,50)/(0,1858,1200,50).
10. history(옵션): saveSearchUrlBtn (700,50,520,60) 투명 1x1, 가장 위.

가운데 열(x 700-1220)에 IR 가 없고 메인이 왼쪽이면 오른쪽 열(x 1220-1920)은 비어 있고 배경/캐릭터만 보인다.

### 9.4 상대적 크기 참고

- 좌측 메인 패널 x 35-700, y 70-1074, 중앙 열 x 697-1223, 오른쪽 열 x 1220-1885.
- 판정 숫자 칸: 1자리 폭 28, 높이 36, PG~MS 5자리 -> 폭 140, early/late 4자리 -> 폭 112.

---

## 10. 결과 전용 객체 값 (원문 그대로)

### 10.1 gaugegraph `grooveGaugeGraph` (mainmenu.lua:268-284)

```
id = "grooveGaugeGraph"
assistClearBGColor = "44004455"
assistAndEasyFailBGColor = "00444455"
grooveFailBGColor = "00440055"
grooveClearAndHardBGColor = "44000055"
exHardBGColor = "44440055"
hazardBGColor = "44444455"
assistClearLineColor = "ff00ff"
assistAndEasyFailLineColor = "00ffff"
grooveFailLineColor = "00ff00"
grooveClearAndHardLineColor = "ff0000"
exHardLineColor = "ffff00"
hazardLineColor = "cccccc"
borderlineColor = "ff0000"
borderColor = "44000055"
```
(8자리 = RRGGBBAA, 6자리 = RRGGBB 로 알파 ff. 배경 알파 0x55 = 약 33%.) 배치는 (M+5,774,655,236).

### 10.2 judgegraph 3종 (mainmenu.lua:286-293), 나머지 필드는 JsonSkin 기본값(delay 500, noGapX 0)

| id | noGap | orderReverse | type | backTexOff | 배치 |
|---|---|---|---|---|---|
| judgesGraph | 0 (OFF) | 1 (ON) | 1 (JUDGE) | 1 (ON) | (M+5,774,655,236), 메뉴 공통, 일반 결과만 |
| notesGraph | 0 | 0 | 0 (NOTES) | 0 | (M+16,341,633,88), 메뉴2 |
| fsGraph | 0 | 0 | 2 (FASTSLOW) | 0 | (M+16,211,633,88), 메뉴2 |

Decide 의 노트 분포: `{id = "notes-graph", noGap = 0, type = 0}` (decide.lua:95).

### 10.3 timingdistributiongraph (mainmenu.lua:14-75)

`id = "timingdistributiongraph"`, `width` = 450(저배율 +-225ms) / 300(표준 +-150ms) / 150(고배율 +-75ms) / nil(코스), `lineWidth = 1`, `averageColor = "FFFFFFFF"`, `devColor = "AAAAAAFF"`, `drawAverage = 1`, `drawDev = 1`.

| 배색 | graphColor | PGColor | GRColor | GDColor | BDColor | PRColor |
|---|---|---|---|---|---|---|
| 통상(기본) | 00FF00EE | 000088FF | 008800FF | 888800FF | 880000FF | 000000FF |
| 赤基調 | E286A7EE | 550000FF | 990000FF | 550000FF | 990000FF | 000000FF |
| 緑基調 | 86E088EE | 005500FF | 008800FF | 005500FF | 008800FF | 000000FF |
| 青基調 | 89DDDCEE | 000055FF | 000099FF | 000055FF | 000099FF | 000000FF |

### 10.4 bpmgraph / gauge

- `bpmgraph = {{id = "bpmgraph"}}` -> JsonSkin 기본값(delay 0, lineWidth 2, mainBPM "00ff00", minBPM "0000ff", maxBPM "ff0000", otherBPM "ffff00", stopLine "ff00ff", transitionLine "7f7f7f").
- gauge(mainmenu.lua:87-109): `id = 2001, parts = 50`(smallGauge 설정 시 100), `nodes` 36개. 6개 게이지(AssistEasy, Easy, Normal, Hard, ExHard, Hazard) x 6, 각 6개 순서는 `[over(명), under(명), over(암), under(암), 선단(명), 선단(암)]`.
  - 0-5: gauge-r1, p1, r2, p2, r3, p3
  - 6-11: gauge-r1, g1, r2, g2, r3, g3
  - 12-17: gauge-r1, b1, r2, b2, r3, b3
  - 18-23: gauge-r1, p1, r2, p2, r3, p3
  - 24-29: gauge-y1, p1, y2, p2, y3, p3
  - 30-35: gauge-h1, p1, h2, p2, h3, p3
  - 노드 이미지(src 8 = gauge/*.png, 80x70, 각 8x35): r1 (0,0), b1 (8,0), r2 (16,0), b2 (24,0), g1 (32,0), p1 (40,0), g2 (48,0), p2 (56,0), r3 (0,0), b3 (8,0), g3 (32,0), p3 (40,0), y1 (0,35), y2 (16,35), y3 (0,35), h1 (64,0), h2 (72,0), h3 (64,0).
  - 36개 노드 -> `indexmap = null`(JsonSkinObjectLoader.java:507) 이므로 i 번째 노드가 게이지 텍스처 인덱스 i 에 그대로 대응.
- graph: `gra_fastRate`(system.png (0,1970,246,26)), `gra_slowRate`(system.png (0,1996,246,26)), 둘 다 `angle = 0`(가로).

---

## 11. 엔진에서 확인한 의미론(시각 동일성에 직접 필요)

### 11.1 좌표와 스케일
- `Skin.setDestination` 은 `x*dw, y*dh, w*dw, h*dh` 만 곱한다(BJ/skin/Skin.java:135-168). y 반전이 없으므로 skin y 는 libGDX 좌표(아래가 0). 소스 이미지 좌표는 이미지 좌상단 기준(`getSourceImage`, JsonSkinObjectLoader.java:601-620: 분할은 `images[divx*j + i]`, 즉 행 우선).
- `getSourceImage` 는 `w == -1` 이면 텍스처 전체 폭. 분할 크기는 `w/divx` 정수 나눗셈.

### 11.2 SkinObject (BJ/skin/SkinObject.java)
- `prepare`: `draw` 조건(`op` 를 BooleanProperty 로 바꾼 것 + `draw` 함수)을 순서대로 모두 만족해야 그려지고, 이어서 `prepareRegion`, `mouseRect` 검사, 색, 각도 순서(SkinObject.java:591-614).
- 타이머 있는 destination: `timer.isOff` 면 그리지 않음, 아니면 `time -= timer.get`.
- 보간: 인접 키프레임 선형, `acc = 1` 이면 rate^2, `acc = 2` 이면 1-(rate-1)^2, `acc = 3` 이면 보간 없이 이전 키프레임 값(SkinObject.java:545-571, 389-413). `acc` 는 오브젝트 단위로 한 번만 정해진다(첫 비0 값).
- `prepareColor` 에서 offset alpha 는 키프레임이 1개이거나 rate == 0 인 경우에만 더해지고, 보간 중(rate != 0)에는 `return` 으로 건너뛴다(SkinObject.java:481-515). 배경 밝기 offset 40 은 단일 키프레임 BLACK 이라 정상 동작한다.
- offset 의 `x, y` 는 `region.x += off.x - off.w/2`, `region.y += off.y - off.h/2`, `width += off.w`, `height += off.h` (SkinObject.java:389-413). 캐릭터 위치 offset 41 은 x, y 이동만 허용.
- `mouseRect`: 마우스가 `region` 기준 상대 사각형 밖이면 `draw = false` -> 호버 시에만 보이는 강조 이미지(SkinObject.java:603-607).
- 클릭: 위에서부터(역순) 처음으로 `draw` 중이고 `clickevent` 가 있고 사각형 안인 객체가 처리(소비)한다(Skin.java:394-411, SkinObject.java:672-704). 인자 `inc` 는 버튼 0 = +1, 1 = -1, 2/3 = +1, 4 = -1.

### 11.3 result.lua / course.lua 의 destination 최종 z 순서
배경 -> 하단 바(BLACK + 텍스트) -> 중앙 정보 -> 인프레션 -> 메인 메뉴 -> (IR 메뉴) -> 시작 애니메이션 -> (diflist 버튼, 결과만) -> 페이드아웃 -> (history 버튼). 모든 `parts.destination` 이 `CUSTOM.ADD_ALL` 로 이 순서에 이어 붙는다(result.lua:44-172). 같은 `id` 가 여러 destination 으로 재사용될 수 있고 id 조회는 image -> imageset -> value -> floatvalue -> text -> slider -> graph -> gaugegraph -> judgegraph -> bpmgraph -> hiterrorvisualizer -> timingvisualizer -> timingdistributiongraph -> gauge 순으로 첫 일치를 쓴다(JsonSkinObjectLoader.java:45-512). 정의되지 않은 id 는 조용히 생략된다(JSONSkinLoader.java:324-338).

### 11.4 텍스트 (BJ/skin/SkinTextFont.java:160-175, 224-232, SkinTextBitmap.java:92-107, 157-173)
앵커/정렬은 1장 10번. `overflow = 1` 이면 줄바꿈 없이 `layout.width > region.width` 일 때 `scaleX *= region.width / layout.width`. 그림자는 `(x + shadowX, region.y - shadowY)` 에 글자색의 1/2 로 먼저 그린다. 텍스트 높이는 `region.height / size`.

### 11.5 number (BJ/skin/SkinNumber.java:130-205)
5.3 의 표. 추가로 값 변경이 없고 이미지 세트가 같으면 자리 이미지 배열을 재계산하지 않는다(성능). 

### 11.6 graph (BJ/skin/SkinGraph.java:84-98)
`angle = 0`(가로): 소스를 가로로 `w * value` 만큼 자르고 `region.width * value` 폭으로 그린다. 음수 `region.width` 면 왼쪽으로 뻗는다. NaN 값은 처리되지 않는다(0 으로 취급하는 편이 안전).

### 11.7 gauge / imageset / image
- gauge: 36 노드, 한 줄에 `parts` 개를 그려 결과 화면 게이지를 표시(JsonSkinObjectLoader.java:505-552).
- image `len > 1`: `TextureRegion[len][frames]`, `ref` 로 행(len 중 하나) 선택, 행 안 `frames` 장은 `cycle` ms 로 순환(JsonSkinObjectLoader.java:51-66). 인덱스가 `Integer.MIN_VALUE` 또는 범위 밖일 때의 처리는 SkinImage.java 를 읽지 않았다(미확인).
- imageset: `ref` 번째 이미지(범위 밖이면 미확인). ClearType 배경은 `images` 배열의 0 번 자리에 "bgNormal"(NoPlay 용)이 온다(background.lua:128).

### 11.8 wildcard 경로 해석 (BJ/skin/SkinLoader.java:95-130)
`filemap` 키(= `<skin dir>/<filepath.path>`)가 이미지 경로의 접두사면 `*` 를 선택된 파일명(확장자 포함, 예 `#default.png`)으로 치환. 그렇지 않으면 디렉터리를 나열해 확장자(`*` 뒤 문자열, 소문자 비교)가 일치하는 파일 중 무작위. `|` 구분자는 확장자 대안 목록이다(ModernChic 은 미사용). 폴더 안의 `*.txt`(1920x1080のpng.txt)는 `*.png` 와 일치하지 않아 무시된다.

### 11.9 Lua 속성 변환 (BJ/skin/lua/LuaSkinLoader.java:97-166)
함수 -> Lua 함수 속성, 숫자 -> id 기반 속성(`getIntegerProperty(id)`, `getTimerProperty(id)` 등), 문자열 -> 이름 속성 또는 Lua 스크립트 문자열(`"return " + script`). `draw`/`value`/`timer` 함수의 예외는 잡혀서 `false`/`0`/`""`/타이머 OFF 로 처리되고 경고만 남는다(SkinLuaAccessor.java:611-747). `act` 함수는 `function.narg()` 로 0/1/2 인자 이벤트를 만드는데 LuaClosure 의 `narg()` 는 1 이다(SkinLuaAccessor.java:759-788).

---

## 12. 파트별 설명

### 12.1 mainmenu.lua (Result/lua/mainmenu.lua)

무엇을 그리나: 결과 화면 좌측(또는 우측) 패널 전체. 헤더 줄, 게이지 이력 그래프(gaugegraph) + 판정 그래프, 클리어 랭크 이미지, 사용 옵션, 총 노트 수, 게이지 바 + 잔량, 그리고 두 개의 "메뉴" 가 번갈아 표시된다.

- 메뉴 1(Info 가 아닌 쪽, 기본): EXSCORE/diff/rate, 항목행(콤보 또는 미스카운트 선택), PG, GR, GD, BD, PR, MS 행(early/late 포함), FAST/SLOW 균형 막대.
- 메뉴 2("Info" 클릭): 판정 레벨, 리플레이 4슬롯 상태, TOTAL(실제 vs 참고값 색 비교), notesGraph, bpmgraph, fsGraph, 타이밍 분포 그래프와 표준편차/평균. 코스에서는 코스 곡 목록(5곡 이하 정지, 6곡 이상 2초마다 1-5 <-> 6-10 페이드 전환).
- 상태: `isInfoMenu1 = true`, `isInfoMenu2 = false`(로컬 변수). 전환 입력은 (a) "Info" 버튼 클릭(`mainMenu` 이미지의 `act`), (b) 화살표 RIGHT(= 메뉴1), LEFT(= 메뉴2) 키. 키 입력은 `Gdx.input:isKeyPressed` 를 draw 함수에서 폴링.
- 메뉴 전환 시 `timer_observe_boolean` 이 새로 켜지므로 해당 메뉴의 키프레임 애니메이션(lampGreen 의 맥동, 리플레이 saved 의 깜박임 등)이 처음부터 다시 시작한다.
- 파일 기록 없음. 소리: 클릭 시 change.ogg.
- 의존 상태: 7장 전부. 외부 의존: `luajava`(Gdx 키), `timer_util`.
- 시각 주의: 메뉴 2 전용 destination 에는 `timer` 가 걸려 있지만, mainExscore/mainExscoreDiff/numExscoreRate/numExscoreRateAfterdot, 항목행, clearType/랭크, 게이지 이미지/숫자, 클리어 랭크 이미지는 타이머가 없어 두 메뉴에서 모두 보인다. 메뉴 1 의 `mainJudgeFrame`(배경 프레임)만 메뉴 1 에서 보인다.

### 12.2 prepare.lua (시작 애니메이션, 옵션 `開始時アニメーション`, 기본 켜짐)

모든 destination 은 `loop = -1`(1회 후 소멸). 이미지: frameDot(prepare.png (2000,0,1,1)), preClearType(prepare.png (0,0,1920,3630) divy 11 len 11 ref 370), stripe((0,3630,2500,170)), stageFailed((2000,330,1920,330)), stagePass((2000,660,1920,330)).

타임라인(ms):
- 0: 위 선 (0,379,1,3) -> 500: w=1920 -> 1500 유지 -> 2000: (0,...,w=1). 아래 선 (1920,697,1,3) -> 500: (x=0,w=1920) -> 1500 -> 2000: (1920, w=1).
- 500: BLACK (0,540,1920,1,a=200,acc=감속) -> 1000: (y=382,h=315) -> 1500 -> 2000: 다시 (y=540,h=1).
- 1000: stripe (0,455,2500,170,a=150,감속) -> 1500: x=-100 -> 2500: a=0.
- 일반 결과: preClearType (1000: -960,205,3840,660, a=50, 감속) -> 1200: (0,370,1920,330,a=255) -> 1500 -> 2000: a=0. `ref = 370` 이므로 현재 클리어 타입 행이 선택된다(시트: 0 NOPLAY(공백) 1 FAILED, 2 ASSISTED CLEAR(보라), 3 ASSISTED CLEAR(파랑), 4 EASY CLEAR, 5 CLEAR, 6 HARD CLEAR, 7 EXHARD CLEAR, 8 FULLCOMBO, 9 PERFECT, 10 MAXCLEAR!!!).
- 코스/코스 도중(`isCourse()`): op 90 이면 stagePass, op 91 이면 stageFailed 가 같은 방식으로 등장.

주의: `prepare.png` 가 3920x3800 이라는 점(약 59MB RGBA). 쓰이는 영역은 x 0-2500 의 11행(0-3630) + stripe 행 + x 2000-3920 의 2개 행 + 1px 점.

### 12.3 irmenu.lua (IR 메뉴, 온라인 + 옵션일 때만)

`PROPERTY.isIrmenuOn() and main_state.option(51)` 가 로드 시점에 참일 때만 포함된다(result.lua:111). 우측(SUB_POS_X) 에 (1) IR 전송 상태 바(timer 172/173/174), (2) 내 순위 프레임(순위 갱신 애니메이션, 상위 10%면 irEliteRank, 상승 시 IrOvertake), (3) TOP10 목록 또는 클리어 상황 그래프(전환: 라벨 클릭 또는 UP/DOWN 키), (4) 카메라 아이콘 클릭으로 이름 가림 커버(`irTop10Cover`), (5) 스크롤바(slider type 8). 파일 기록 없음, 소리 change.ogg. 상태: `isIrRanking/isIrClear`(초기 옵션 `IRメニューの種類`), `screenHidden`(config.lua:144 `result.irCover` 초기값). 의존: number 179/180/182, 200-242, 380-399, text 120-129 와 1020, option 51/-606, timer 172-174. R-BMS 에 IR 가 없다면 통째로 생략해도 나머지 화면에 영향이 없다.

### 12.4 impression.lua (인프레 계산기)

클릭으로 숫자를 입력해 `History/impression.txt` 에 `제목_점수pts_메시지[★★★]` 한 줄을 추가하는 기능(총점 모드, 기본) 또는 SONG/BGA/ETC 3항목 합산 모드(`config.lua:136-150` 의 `impression.type`). 기본 `sw = false` 이므로 화면 좌하단(0,0,60,50)의 토글 버튼만 보이고, 누르면 가운데에 계산기(805,340,309,554)가 나타난다. 의존: 파일 읽기/쓰기, `CUSTOM.SOUND`(click.ogg, enter.ogg, change.ogg), `CONFIG.impression.*`, `History/impression.txt` 의 줄 수(`impressionCount`). 이 기능은 게임 플레이 핵심이 아니다.

### 12.5 history.lua (플레이 이력, 옵션 `プレイ履歴の保存`, 기본 꺼짐)

로드 시점에 `History/<yymmdd>/{history,clear,score,miss}.txt` 에 행을 추가하고 `History/recent.txt`(역순), `History/search.html` 을 쓴다. 그리고 우측 하단 투명 1x1 버튼(700,50,520,60). 의존: `CUSTOM.CLOCK`(os.date), mkdir(luajava File), io 읽기/쓰기 전부, number 370/371/172/176/177/178/170/171. 주의: 리플레이 중에도 기록되는 문제가 소스 주석(result.lua:161-162)에 TODO 로 남아 있다. `Decide` 의 `ステージ数表示` 가 이 이력 파일의 줄 수에 의존한다.

### 12.6 diflist.lua

우측 하단(1860,0,60,50) 아이콘. 클릭하면 `difflist.txt`(skin 루트)에 `[{"title":..,"artist":..,"level":..,"md5":..,"sha256":..}]` 형식 JSON 을 추가하고(이미 있으면 마지막 `]` 를 제거 후 이어붙임) 소리를 낸다. 이후 `difBtn = false` 가 되어 사라진다. 현재 MC/difflist.txt 는 이전 사용으로 남은 한 레코드("ヤラレ[7keys NORMAL]", md5 c2e1e8ce..., 215 바이트)다(MC/difflist.txt). 코스에는 없다. 의존: 쓰기 + number 96, text 10/14/1030/1031.

### 12.7 centerinfo.lua, background.lua, fadeout.lua

9장에서 좌표를 정리했다. background 의 rotation(`bgRotationSwitch`, `charRotationSwitch`) 은 listchoice.lua 로 폴더의 png 목록을 `io/Result/{bg,char}/<종류>/{path}_pathList.txt`, `_excludeList.txt` 에 기록하며 무작위 비복원 추출을 하고 그 `source.path` 를 특정 파일 하나로 바꾼다(background.lua:40,60,90,123,176,200,234,271). `luajava File:listFiles` 와 쓰기가 필요하고 기본은 꺼져 있다.

---

## 13. 클리어 여부, 랭크, 클리어 타입에 따른 배경과 캐릭터 선택 규칙

| 패턴 옵션 | 배경(background.lua) | 캐릭터(같은 파일 하반부) |
|---|---|---|
| Clear or Failed (기본) | bgClear(op 90), bgFailed(op 91). 소스 `Result/parts/bg/isclear/{clear,failed}/*.png` | charClear(op 90, 소스 char/isclear/clear), charFailed(op 91). 선택 기본 파일 `yuki` |
| ALL | bgAll(조건 없음, 소스 id 10 `bg/all/*.png`) | charAll(조건 없음, 소스 `char/all/*.png`) |
| Rank | 8개 이미지(AAA, AA, A, B, C, D, E, F), op 300-307. 소스 `bg/rank/<랭크>/*.png`. 랭크 option 이 어느 것도 true 가 아니면(예 RESULT_0_1P 308) 아무 것도 안 그려짐 | 같은 구조, 소스 `char/rank/<랭크>/*.png` |
| ClearType (결과만) | `imageset bgClearType`: `ref = 370`, images 인덱스 0..10 = [bgNormal(NoPlay), bgFailed2, bgAssist, bgLaAssist, bgEasy, bgNormal, bgHard, bgExhard, bgFullCombo, bgPerfect, bgMax] | `charClearType` 같은 순서 + 오프셋 41 + draw 에 `isCharDisplayOn`. 클리어 id 의 의미: 0 NoPlay, 1 Failed, 2 AssistEasy, 3 LightAssistEasy, 4 Easy, 5 Normal, 6 Hard, 7 ExHard, 8 FullCombo, 9 Perfect, 10 Max(BJ/ClearType.java:10-20). 코스 도중(`isCourse()`) 에서 ClearType 를 고르면 Clear or Failed 로 대체 |

캐릭터 공통: `PROPERTY.isCharOn()` 이고 `CUSTOM.OP.isCharDisplayOn`(런타임 토글, 초기 true) 일 때만, `loop = 200` 으로 2배 크기에서 1배로 줄어드는 등장 애니메이션, `offsets = {41}`. 캐릭터 기본 선택: isclear/clear, isclear/failed 는 `yuki.png`(1920x1080, 중앙 인물 한 명 투명 배경), 나머지는 `#default.png`(1920x1080 완전 투명으로 보이는 11414바이트 동일 파일, md5 02856cf6...). 즉 기본 설정에서는 Clear/Failed 패턴일 때만 yuki 가 보이고 ALL/Rank/ClearType 로 바꾸면 투명 이미지만 올라간다.

배경 기본: isclear/clear = 청록 회로 원형 HUD(1.9MB), isclear/failed = 같은 계열(1.9MB). 다른 패턴의 `#default.png` 도 전부 1920x1080 실제 그림이다(총 40MB).

beam 색: 클리어면 청색 계열(r 50-100, g 50-100, b 150-255), 실패면 적색 계열(r 150-255, g 50-100, b 50-100).

### 13.1 클리어 랭크 이미지 선택

- 게이지 영역의 큰 랭크 글자(rankAAA..F): 시트 `Result/parts/rank/<선택>.png` 400x1344, 행 i(0..7) = AAA, AA, A, B, C, D, E, F, 행 높이 168. AAA 는 글자 3개, AA 는 2개, 나머지 1개(#default 확인).
- `thisTime` 작은 랭크 라벨: system.png (0, 1110+43i, 142, 43).
- 랭크 갱신 표시(`rankUpdate`)는 `bestRank < nowRank and prev ~= 0`(이전 클리어가 NOPLAY 가 아닐 때만).

---

## 14. 결과 vs 코스 결과 차이 정리

| 항목 | 결과(type 7) | 코스 결과(type 15) |
|---|---|---|
| 하단 텍스트 | bottomResult(제목 / 아티스트 / 장르 / 난이도표) | bottomCourse(코스 이름만) |
| property | 20개 | 17개(label 5, 14, 15 없음, 배경/캐릭터 패턴 각 3 item) |
| filepath | 45개(ClearType 20개 포함) | 25개 |
| category | 16 | 12 |
| centerinfo | `load(0)`: 즐겨찾기 버튼 chartBtn, 코스 도중 다음 스테이지 프레임, tsY = 910 | `load(1)`: 둘 다 없음, tsY = 970 |
| impression | 있음 | 없음 |
| mainmenu | `load(0)`: mainInfo 순환, judgesGraph, 메뉴2 에 그래프들 | `load(1)`: mainInfo2 고정, judgesGraph 없음, 메뉴2 에 코스 곡 목록(courseFrame, course_1-5, course_6-10) |
| diflist 버튼 | 있음 | 없음 |
| prepare | preClearType (ref 370) | stagePass/stageFailed (op 90/91) |
| infoOutput(7|15), history | 있음 / 옵션 | 있음 / 옵션 |
| voice(`resultVoice`) | `CONFIG.voice.result.sw`(기본 false) 일 때 호출 | course.lua 에는 호출 코드 자체가 없음 |
| 타이밍 분포 배율/색 | 옵션 | 옵션 키 없음 -> 기본 폭 301, 통상 색 |

일반 결과 도중에도 `isCourse()`(COURSE1_TITLE 비어있지 않음) 로 코스 진행 중인 스테이지 결과를 구분한다.

---

## 15. 자산 (8)

### 15.1 기본 옵션에서 실제로 필요한 것

Decide: Decide/parts/parts.png(710x850, 50KB), parts2.png(750x110, 2KB), Decide/bg/image/sample.png(1920x1080, 882KB), ttf 2종(mgenplus-1c-black, mgenplus-1c-medium 각 약 5.2MB; Result 폴더 사본과 md5 동일: d057a653..., a38034d3...).

Result: Result/parts/{ring.png 1920x1080 540KB, system.png 2700x2200 465KB, prepare.png 3920x3800 562KB, number.png 812x580 28KB, lamp.png 114x1064 27KB, parts.png 1050x1120 179KB}, rank/#default.png(400x1344, 122KB), gauge/#default.png(80x70, 1KB), irmask/#default.png(512x645, 5KB; IR 전용), bg/isclear/{clear,failed}/#default.png(각 1.9MB, 1920x1080), char/isclear/{clear,failed}/yuki.png(294KB, 190KB), Root/sounds/{change,click,enter}.ogg(결과 클릭음; 모든 사운드 합 약 327KB), ttf 2종(Decide 와 동일).

기본 옵션의 실제 사용량(손 합산): ttf 약 10.4MB(Decide/Result 공유 시 한 벌) + png/ogg 약 7.6MB(그 중 결과 배경 2장이 3.9MB) = 약 18MB.

### 15.2 와일드카드 파일 슬롯 목록(파일 선택 UI 슬롯)

Decide 2슬롯, Result 45슬롯(rank/gauge/irmask 3 + 배경 21 + 캐릭터 21), 코스 25슬롯. 각 슬롯 폴더와 파일:

| 슬롯 | 폴더(MC 기준) | 파일 |
|---|---|---|
| クリアランクイメージ | Result/parts/rank/ | #default, CoolDiagonal, CuteRound, Damage, Formal, Separate, Underworld (각 400x1344, 59-138KB) |
| ゲージ | Result/parts/gauge/ | #default, classic, diagonal, gradation, old, separate (각 80x70, 1-3KB) |
| IRカバー | Result/parts/irmask/ | #default(512x645) |
| 背景 isclear | bg/isclear/clear, failed | #default 각 1.9MB |
| 背景 all | bg/all | #default 1.2MB |
| 背景 rank | bg/rank/{AAA,AA,A,B,C,D,E,F} | #default 각 1.1-1.5MB (합 10.6MB) |
| 背景 clearType | bg/clearType/{failed,assist,laassist,easy,normal,hard,exhard,fullcombo,perfect,max} | #default 각 0.34-5.0MB (합 25.1MB, fullcombo 5.0MB, perfect 3.4MB, easy 3.1MB, normal 2.9MB, assist 2.7MB) |
| キャラ isclear | char/isclear/{clear,failed} | #default(투명 11KB), yuki 각 294KB/190KB |
| キャラ all, rank, clearType | char/{all, rank/*, clearType/*}/ | #default(투명 11KB 동일 파일) 총 19개 |

참조되지 않는/활성화 안 된 자산: Decide bg/movie(18.5MB), sample2.png/sample2.mp4, 비트맵 폰트 전체(Decide 36MB, Result 16.5MB), Result/parts/dummy/-(0바이트), 각 폴더의 안내 txt, io/Result/**(회전 로그용 빈 폴더 8개), History/, difflist.txt, Root/image/(캐릭터 PNG 7개, play 전용), Root/sounds/vo/(보이스, 결과 보이스는 config 로 꺼져 있음).

### 15.3 비트맵 폰트 (옵션 시만)
Decide/font/fnt/{main,sub}.fnt(각 1.3MB, 11180 글리프) + main1-11.png(11페이지) + sub1-7.png(7페이지), 모두 2048x2048 PNG(페이지당 약 2MB, 합 36MB). Result/font/fnt 는 main1-4, sub1-4 (16.5MB). 비용이 커서 원작도 "高負荷" 로 표기한다.

### 15.4 시트 구조 (오프셋 근거)
- Decide/parts.png: 6행(각 93) x 10열(각 71)의 숫자 시트(초록, 청록, 주황, 빨강, 보라, 회색), 그 아래 (0,558+45k,240,45) 6개 라벨(BEGINNER, NORMAL, HYPER, ANOTHER, ANOTHER, UNKNOWN).
- Decide/parts2.png: 흰색 반투명 전용 시트(밝은 배경에서는 안 보임). (0,0,730,50) GET READY 계열 텍스트, (0,50)(60,50)(120,50)(180,50) 50x50 모서리 4개.
- Result/system.png 2700x2200: 가장 큰 시트(UI 프레임, 판정 라벨, 랭크/클리어 라벨 11행 x 3프레임, IR 프레임, 그래프 프레임 등). 좌표는 9장의 각 항목 참조.
- Result/parts.png 1050x1120: 키 모드/난이도 라벨(0-520 x 0-320), 회색 프레임(0,320,520,60), UPDATE 도장(199,380,214,36 x2), 리플레이/옵션 목록(520,0,175,400), 계산기(700,0,309,554 / 700,560), 다음 곡 프레임(0,726,526,126), 즐겨찾기 버튼(0,852,330,192) 등.
- Result/number.png 812x580: 숫자 시트. (0,0) 녹색 큰 숫자 11장 계열, (0,96)/(440,96) 흰색 10/11장, (440,60), (440,276), (440,312), (440,348), (440,504 ...) 등 색/크기별 행. 11번째 칸은 회색 어두운 0 으로 보이며 앞자리 패딩에 쓰인다.

---

## 16. 단순화 후보 (9)

기본 스킨(ModernChic 단순판)에서 빼도 화면 핵심이 유지되는 것

| 후보 | 근거 | 효과 |
|---|---|---|
| Decide 동영상 배경, sample2.png/mp4, bg 선택 옵션 | 옵션 기본이 정지 이미지. 스킨 전체 mp4 8개 중 Decide 2개(18.5MB)가 이 화면 소속 | 디코더 불필요, 용량 -18.5MB |
| 비트맵 폰트 옵션 전체(Decide/Result fnt, 거리장 outline) | 기본 ttf, 합 약 52MB | 용량 대폭 감소, 텍스트 경로 하나 |
| Decide `stage`(STAGE n) | History 파일의 줄 수에 의존, 기본 꺼짐 | io 불필요 |
| Decide `tips` | 일본어 22개 문구, beatoraja 단축키를 가리킴 | 문자열/난수 불필요 |
| lockon 5중 반복 | 20개 destination, 로드마다 무작위 색. 1세트(4개)로 줄여도 인상 유지 | 객체 수 -16 |
| 결과 IR 메뉴 전체(irmenu.lua, 671줄) | 온라인 + IR 구현이 필요, R-BMS 에는 IR 가 없으면 항상 숨겨짐 | image 30 / imageset 10 / value 68 / graph 11 / slider 1 제거 |
| 인프레션(impression.lua), history.lua, diflist.lua | 전부 파일 쓰기 중심 보조 기능, 결과 정보와 무관 | io 쓰기 불필요, History 폴더 불필요 |
| 회전(rotation) 옵션과 listchoice.lua, io/Result | 파일 목록 기록과 `luajava File` 필요 | luajava File 불필요 |
| infoOutput(History/information.txt), config.lua 의 voice/impression/bpmLinkChar/play 설정 | 결정/결과에서는 voice, infoOutput 만 의미 있음 | 전역 CONFIG 의존 축소 |
| 키보드 더미(BLACK 0x0 draw 폴링) | 메뉴 전환을 R-BMS 입력 처리로 옮기면 Gdx 불필요 | luajava Gdx 불필요 |
| 어시스트 안내(assistInfo), 시작 애니메이션의 stripe | 장식. 필요하면 유지 가능 | - |
| 리플레이 3중 루프 중복 destination | 같은 객체가 3번 그려짐 | 36 -> 12 |
| ClearType 배경/캐릭터 20개 슬롯, Rank 배경 8개 슬롯 | 25MB + 10.6MB. CF + ALL 만 남겨도 충분 | 용량 -35MB, filepath 슬롯 단순화 |
| 캐릭터 패턴(Rank, ClearType, ALL) | `#default` 는 전부 투명 | 슬롯 정리 |
| 라이선스 제한 이미지: rank #default, Damage, Formal, Underworld, yuki(ごｎ 作) | readme 주1 및 특별 감사 | 사용 가능 이미지로 교체 또는 허락 확인 필요(17장) |
| 24키/48키 모드 라벨 | R-BMS 가 해당 모드를 지원하지 않으면 | 미사용 라벨 |
| 한국어/영어 다국어 | 이 스킨은 일본어 하드코딩(옵션 이름, 안내 이미지, 팁) | 이미지 시트 안의 일본어 문구(system.png 상단 안내 3줄, "ラストまであと", "位" 등)는 교체 대상 |

빼면 안 되는 것(화면 인상과 정보량을 결정)

1. Decide: 난이도별 색(RGB), 큰 제목/장르/아티스트/레벨 숫자/난이도 라벨의 배치와 500ms 페이드 인, 검정 띠 + 스테이지 파일 확대, lockon 모서리(최소 1세트), 노트 분포 + BPM 그래프, 페이드아웃.
2. Result: 배경 + 링 + 빔 + 캐릭터 겹침, 게이지 이력 그래프 + 판정 그래프 + 게이지 바/잔량, 클리어 랭크 큰 글자 + 이번 랭크 라벨 + 클리어 타입 라벨 + UPDATE 도장 + 랭크/램프 갱신 표시, EXSCORE/diff/rate, 콤보/미스카운트 행, PG..MS(early/late) 행, FAST/SLOW 막대, 정보 메뉴(Info) 전환, 중앙 열의 모드/난이도/플레이어/날짜/즐겨찾기/남은 노트/스테이지 파일, 시작 애니메이션, 페이드아웃, 코스 패널, 리플레이 상태, TOTAL 비교, 타이밍 분포 그래프.
3. 엔진 동작: timer_observe_boolean 기반 메뉴 전환, `op`/`draw`/`mouseRect`/`acc`/`loop`/`offsets` 의미, 음수 폭 그래프, 임의 알파 `blend = 2`, 숫자 시트 3종, imageset/len+ref 선택, 텍스트 앵커 규칙.

---

## 17. 위험과 미확인, 미결

위험

1. 로드 시점 평가 요구(1장 1번)와 pcall 침묵 실패(1장 4번): 어느 파트가 조용히 빠져도 로그 없이 화면이 비정상적으로 보일 수 있다. 개발 중에는 pcall 실패를 반드시 경고 로그로 남겨야 한다.
2. `diffRGB()` nil 크래시, `main_state.option(51)`, `main_state.text(150)` 등 라이브 상태 계약. 결정 화면 option 150-155 중 정확히 하나가 true 여야 한다.
3. 같은 번호 id 가 값/이미지 인덱스에서 다른 의미(380-399, 370, 42, 43, 90): R-BMS 의 속성 레지스트리가 "값"과 "이미지 인덱스" 스코프를 구분해야 한다.
4. 라이선스: 2차 배포 금지(허가 시 제외), 개조판 공개 시 KASAKO 크레딧 필수, rank 이미지 4종 제외, 폰트 Mgen+(SIL OFL), 랭크 폰트 ゆうたONE 이용, 보이스 VOICEVOX(ずんだもん/春日部つむぎ), 캐릭터 yuki 는 ごｎ 작. 기본 번들로 저장소에 포함하는 것이 허용되는지 확인되지 않았다.
5. prepare.png 3920x3800 (RGBA 약 59MB) 와 system.png 2700x2200: GPU 최대 텍스처 크기 제한에 걸리는 환경이 있다. 단순화판에서 시트 분할이 필요할 수 있다.
6. 구현 시 `loop` 의미와 음수 `w`, alpha offset 의 조기 return 등 beatoraja 의 비직관 동작을 그대로 복제하지 않으면 시각이 달라진다. 11장을 근거로 한다.
7. Lua 5.4(mlua) 와 LuaJ 의 차이(정수 나눗셈 결과 float, 반올림이 아닌 truncation): destination 필드에서 float -> int 변환 규칙을 통일해야 한다.
8. NaN: `gra_fastRate/gra_slowRate` 가 early+late = 0 일 때 NaN. beatoraja 도 처리하지 않으므로 R-BMS 가 0 으로 처리해도 시각 차이는 사실상 없다(완전 PERFECT 플레이에서 막대가 전혀 안 보임).
9. 필터 UI 의 이름 충돌: 두 번 나오는 "背景（Failed）", "キャラクター（Failed）".
10. `act` 숫자 370/371 은 beatoraja 에서 사실상 무동작이지만 일부 구현은 미정의 이벤트에서 오류를 낼 수 있다. 오류 없이 무시해야 한다.

미확인

- SkinImage 의 ref 인덱스가 범위를 벗어나거나 MIN_VALUE 일 때의 동작, SkinGauge 의 상세 보간/애니메이션(starttime 0, endtime 500, cycle 33), SkinNoteDistributionGraph 와 SkinBPMGraph 의 내부 렌더, SkinTimingDistributionGraph 의 렌더(색 해석 외).
- mp4 와 ogg 의 실제 내용, ttf/fnt 의 글리프 범위(일본어 + 한글 포함 여부), Result/parts/gauge/*.png 와 rank 6종의 시각 모습.
- Decide `bg/movie` 동영상 해상도/코덱.
- 이 보고서에서 읽지 않은 파일: Select/**, Play/**, KeyConfig/**, SkinSelect/**, Root/image/**, Root/sounds/vo/**, History/** 의 Lua(없음)와 데이터.

미결(사용자 결정이 필요한 것)

1. ModernChic 을 R-BMS 저장소의 기본 번들로 포함해도 되는가(라이선스, 크레딧 표기, 제외 이미지 교체).
2. IR(온라인) 지원 여부: 없다면 irmenu 전체를 제거하고 `main_state.option(51)` 은 항상 false 로 둔다.
3. 결과 화면 키 입력(화살표) 처리와 Info 전환의 책임(Lua 심 vs R-BMS 입력 처리).
4. 단순화판에서 캐릭터(yuki) 와 ClearType/Rank 배경을 어디까지 유지할지.
5. 비트맵 폰트 / outline 지원 여부(기본 ttf 만 지원하는 쪽 권장).
6. 한국어 UI 문구 교체 범위(system.png 안의 일본어 텍스트 이미지 포함 여부).

---

## 부록 A. 결과 화면 option/number 상태 요구(요약 체크리스트)

R-BMS 결과 상태가 제공해야 하는 값(ModernChic 결과/코스 결과 기준):

- option: 90/91, 300-307, 320-327, 330/331/332, 180-184, 196-198 + 1196-1204, 160-164 + 1160/1161, 150-155, 191(스테이지 파일 존재), 280-282 + 290 + 289(마지막 스테이지), 51(온라인), 606(IR 대기).
- number: 74, 102, 103, 105, 107, 407, 110-114, 412-422, 423, 424, 368, 370, 371, 374-377, 170-172, 175, 177, 178, 21-26, 96.
- string: 2, 10, 12-15, 150-159, 1003, 1020, 1030, 1031, 120-129.
- image index: 370(clear), 42/43(random option), 90(favorite chart), 301-307(어시스트 버튼).
- timer: 2(FADEOUT), 172-174(IR), 1(STARTINPUT 은 엔진 내부).
- events: 90(favorite chart 순환), 나머지 클릭은 전부 Lua `act`.
- 객체: gauge(36 노드), gaugegraph, judgegraph(0/1/2), bpmgraph, timingdistributiongraph, graph(value 함수), 스테이지 파일(-100), BLACK(-110).

## 부록 B. Decide 화면 상태 요구

- option 150-155(정확히 하나 true), number 96, string 10-15 + 1003, 이미지 -100(스테이지 파일, 없으면 그리지 않음), judgegraph(노트 분포), bpmgraph, timer 2(FADEOUT).
- 호출 시점 값: 난이도 색, 곡 정보는 로드 시점이 아니라 매 프레임 `ref` 로 읽히는 쪽이다(텍스트 ref 가 있는 것은 draw 시 갱신: title, genre, tablename). 로드 시점 스냅샷은 난이도 색과 tips 뿐이다.
