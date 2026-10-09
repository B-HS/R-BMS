# M1 조사 보고서 - ModernChic 공통 계층과 Lua 런타임 의존 전수 조사

> 최종 갱신 2026-10-10 · 대응 단계: 웨이브 2(Lua 런타임·로더·스킨 팩) 반영 · 본문은 기준 커밋 `9ce92bb` 시점 서술이며, 아래 "웨이브 … 반영 사항"이 최신 것부터 우선한다 · 색인과 갱신 규칙은 [README.md](README.md)

## 웨이브 2B 반영 사항 (2026-10-10)

스킨 덤프 CLI, 앱의 스킨 팩 폴더 지정과 `.luaskin` 로드, 오버레이 총 크기 상한, 외부 스킨 첫 정지 프레임을 넣은 뒤의 상태다.

- (W2-6) m1-root-config.md §3·§2.6, 사양 §6 웨이브 2 '덤프 기준' 객체 수: result 는 코스 제목을 비운 기준 시나리오에서 destination 194(수기와 같음)이고, 모든 문자열을 채우면 195 다. 나머지(play7 286, play5 277, play14 372, play10 354, select 1914, course 175, decide 69, skinselect 108)는 W2-5 실측과 같다. play7 은 NO_BGA(170)=true 일 때 287.
- (W2-6) m1-root-config.md 웨이브 2A 반영 사항의 W2-3 항목: 이미지 인덱스 55 는 '있어야' 가 아니라 2·3·4(HSFIX MAX/MAIN/MIN)여야 한다. Root/customslider.lua adjustedCover() 가 event 0·1 에서 nil 을 돌려줘 Play/lua/sp/cover.lua:29,35,39,43 과 dp/cover.lua:48,49,54,55,56 의 슬라이더 값 함수가 nil 인덱스 오류를 낸다(실제 게임은 op 조건이 거짓이면 값 함수를 부르지 않으므로 덤프처럼 전부 부를 때만 드러난다).
- (W2-6) m1-root-config.md §2.7: keyconfig 실패 위치(Decide/lua/require/textproperty.lua:37 isOutlineFont)가 skin-dump 로도 재현됨(헤더는 type 8 로 정상).
- (리뷰 수정) m1-root-config.md §3(헤더 요약): 덤프 출력으로 대조한 결과 scene/input/fadeout/loadend/playstart/close 6열이 9개 문서(keyconfig 제외) 모두 표와 일치함을 기록. keyconfig 는 본체 실패로 덤프에서 이 6열을 확인하지 못함


## 웨이브 2A 반영 사항 (2026-10-10)

Lua 5.2 런타임(`crates/rbms-skin/src/lua/`), `SkinHost`, Lua 값 변환기, 2패스 `.luaskin` 로더를 넣고 구 샌드박스(`skin.*`)를 삭제한 뒤의 상태다.

- (W2-2a) m1-root-config.md §2.5: '파트 오류는 생략 + 반드시 로그' 권고가 구현됨(LuaDiagnostics.swallowed 에 메시지·파일·줄·횟수)으로 갱신
- (W2-2a) m1-root-config.md §4: 항목 7(math.random 시드)에 '설정 시드로 고정 가능, 기본은 비결정'을, 항목 12(dofile 절대 경로)와 항목 20(스킨 폴더 제한)에 구현 완료와 SkinPaths 규칙을 반영. 헤더 단계 실측(10개 모두 성공, 0.2~0.8ms, 70~250KiB) 추가
- (W2-2a) m1-root-config.md §11: 'require/dofile/package/print/string.match,gsub,gmatch,find 제거' 격차 서술을 해소됨으로 갱신(남은 격차는 io·os·luajava·main_state)
- (W2-2b) m1-root-config.md §9 네트워크 문단: 'LuaJ 의 os.time() 이 소수 초를 반환하고 … 단서다(추정)' 에서 os.time() 소수 초 부분은 OsLib.time 바이트코드로 확인됐으므로 '(추정)' 을 숫자 문자열화 부분에만 남겨야 합니다.
- (W2-2b) m1-root-config.md §4 B.8: '존재하지 않는 파일 읽기는 nil, 메시지 반환' 에 실제 메시지 `io error: Lua file not found: <이름>` 과 루트 밖 `io error: Lua skin file access denied: <이름>` 을 적을 수 있습니다. '줄 읽기는 \n 구분' 뒤에 '줄 안의 \r 은 전부 버린다' 를 추가해야 합니다.
- (W2-2b) m1-root-config.md §9 'R-BMS 구현 시사점': '오버레이로 매핑하고 … 같은 가상 루트를 보게 해야 한다' 를 io 쪽은 구현 완료(lua/io.rs, 읽기 오버레이 → 루트, 쓰기 오버레이, 추가 시 복사)로 갱신해야 합니다.
- (W2-3) m1-root-config.md §3 또는 사양 §6 웨이브 2 '덤프 기준 시나리오': result·course 시나리오에 NUMBER 370(CLEAR), play 시나리오에 이미지 인덱스 55(hsfix)를 반드시 넣어야 한다는 조건 추가(없으면 각각 Root/customfunction.lua:359/362 의 nil 연결, Play/lua/sp|dp/cover.lua 슬라이더 함수의 nil 인덱스 오류).
- (W2-2c) m1-root-config.md §5.4: 'BJ 파사드는 ... 모르는 이름은 nil' 은 낡았습니다. valueOf 는 keyNames.get(name, -1) 이라 모르는 이름이 -1 이며 nil 이 아닙니다(R-BMS 도 -1).
- (W2-5) m1-root-config.md §2.6·§3 과 사양 §6 웨이브 2 '덤프 기준' 객체 수: 실측값으로 교체 — destination 수 play7 286(수기 약 288), play5 277, play14 372(수기 약 375), play10 354, select 1914, result 195(수기 194), course 175, decide 69, skinselect 108. 헤더 표(property/filepath/offset/category)는 실측과 일치(플레이 offset 은 10+4=14)
- (W2-5) m1-root-config.md §2.7: keyconfig 본체 실패 위치가 `Decide/lua/require/textproperty.lua:37: attempt to call field 'isOutlineFont' (a nil value)` 로 실행 확인됨(헤더는 type 8 로 정상 로드)


경로 약어
- MC = /Users/hyunseokbyun/Downloads/ModernChic
- BJ = /Users/hyunseokbyun/development/beatoraja/src/bms/player/beatoraja (beatoraja 사본 소스)
- RB = /Users/hyunseokbyun/development/R-BMS

표기: "파일:줄" 은 MC 기준 상대 경로(BJ/RB 로 표시한 것은 해당 루트 기준). "미확인" 은 직접 확인하지 못한 것. 식별자, 필드명, id 숫자는 원문 그대로 둔다.

중요 전제: BJ 사본은 업스트림 beatoraja 와 다른 포크다(LuaJ 샌드박스, `LegacySkinLuaApi`, 확장된 `main_state` API 포함). ModernChic 은 "beatoraja 0.8.8" 용으로 제작되었다(MC/History/information.txt 의 마지막 항목 문자열 `beatoraja 0.8.8..ModernChicSkin....ACCESS....`). 따라서 아래 "beatoraja 동작" 은 BJ 사본 소스에서 읽은 사실이며 업스트림과 다를 수 있는 부분은 별도로 표시한다.

---

## 0. 한 줄 결론

ModernChic 은 JSON 데이터가 아니라 "실행되는 Lua 프로그램"이다. 스킨 1개를 읽으려면 (1) 파일 단위 `require`/`dofile`, (2) `io.open` 읽기/쓰기, (3) `os.date`/`os.time`, (4) `luajava` 파사드(File/Gdx/Input/URL), (5) `main_state`/`timer_util`/`skin_config` 모듈, (6) 함수형 속성(draw/value/timer/act/text value)의 매 프레임 평가, (7) 스킨 폴더 내 쓰기 가능 영역이 필요하다. 현재 RB 의 `rbms-skin` Lua 샌드박스(한 줄 식 전용, `require/dofile/io/os/package/print/string.match,gsub,gmatch,find` 제거)로는 단 하나의 ModernChic 화면도 로드할 수 없다.

---

## 1. 읽은 범위와 읽지 못한 범위

전부 읽음(라인 단위)
- MC 루트: `*.luaskin` 10개, `config.lua`, `play5_hw/play7_hw(전체)/play10_hw(앞 70줄 + 호출 순서)/play14_hw(호출 순서)/musicselect/result/course/decide/keyconfig/skinselect.lua`, `difflist.txt`
- `Root/*.lua` 전부 (define, define2, main*.lua 9개, custom*.lua 8개, version, author)
- 각 화면 `lua/require/` 의 header/property/textproperty 전부: Play(header, common, sp_property, dp_property 전체, textproperty), Select(header, property, textproperty, http, settings), Decide, Result(header, property, textproperty, listchoice), KeyConfig, SkinSelect
- `Select/lua/versioncheck.lua`, `Select/lua/settings/*`, `Select/readme.txt`
- `io/` 트리와 내부 txt 전부, `History/` 2개 파일
- `Play/lua/base.lua`, `Play/lua/background.lua`, `Result/lua/base.lua`, `Result/lua/history.lua`, `Result/lua/diflist.lua`, `Play/lua/sp/detailinfo/nentyakuinfo.lua`

부분만 읽음(전역 API 집계는 grep 으로 전수, 내용은 발췌)
- `Select/lua/sidemenu.lua`(1-100), `Result/lua/background.lua`(1-120), `Result/lua/impression.lua`(60-90, 330-360), `Play/lua/sp/detailinfo/bgaareainfo.lua`(25-75, 140-175), `Play/lua/sp/cover.lua`(155-185)

읽지 않음(다른 조사자 범위. 단, 아래 5장의 전역 API/ID 집계는 이 파일들을 포함해 grep 으로 전수 수행)
- `Play/lua/sp/*.lua`, `Play/lua/dp/*.lua`, `Play/lua/close.lua`, `Play/lua/sp/detailinfo/{inputinformation,rendafrequency,trend,util}.lua`
- `Select/lua/*.lua` 나머지 파트, `Result/lua/{centerinfo,impression 나머지,irmenu,mainmenu,prepare,fadeout}.lua`

beatoraja 사본에서 읽은 파일: `BJ/skin/lua/{SkinLuaAccessor,LuaSkinLoader,LegacySkinLuaApi,MainStateAccessor,MainStatePropertyLuaApiExporter,SkinAudioLuaApiExporter,SkinFileLuaApiExporter,SkinHttpLuaApiExporter,TimerUtility,SkinLuaPathResolver}.java`, `BJ/skin/json/{JSONSkinLoader,JsonSkin}.java`(발췌), `BJ/skin/{SkinHeader,SkinLoader,SkinObject,SkinSourceReference,SkinType}.java`(발췌), `BJ/skin/SkinProperty.java`(대조용 grep).

보조 도구(읽기 전용 분석 스크립트, 재현용): `/private/tmp/claude-501/-Users-hyunseokbyun-development-R-BMS/41ddb2d1-97ab-47c9-943d-b6bc19d0d388/scratchpad/skin-research/tools/m1_*.py` (m1_undefined_ids, m1_used_ids, m1_vs_beatoraja, m1_header_counts, m1_dup_names, m1_lua_reachability, m1_png_sizes). Lua 인터프리터가 이 머신에 없어 스킨 실행은 하지 않았고, 모든 수치는 소스 정적 분석이다.

파일 형식: 모든 `.lua`/`.luaskin` 136개가 UTF-8(BOM 없음) + CRLF. 일본어 문자열 포함. 런타임이 생성하는 txt 는 LF.

---

## 2. 조립 구조 (.luaskin -> 본체 Lua -> Root/define -> 화면 파트)

### 2.1 .luaskin 10개 (전부 동일 5줄)

MC/play7_hw.luaskin:1-5 (나머지 9개는 `require` 인자만 다름: course, decide, keyconfig, musicselect, play10_hw, play14_hw, play5_hw, result, skinselect)

```
local t = require("play7_hw")
if skin_config then
	return t.main()
else
	return t.header
end
```

- `skin_config` 전역이 nil 이면 "헤더 단계"(`t.header` 반환), 아니면 "본체 단계"(`t.main()` 반환).
- 모듈 `require("play7_hw")` 은 `<스킨폴더>/play7_hw.lua`(루트의 본체 Lua)를 찾는다. `.luaskin` 자체는 모듈이 아니다.

### 2.2 beatoraja 가 이 파일을 두 번 실행하는 방식 (BJ 사본 근거)

1. 런처/스킨 목록: `new LuaSkinLoader()` (BJ/launcher/SkinConfigurationView.java:187) -> `loadHeader(p)`: `lua.setDirectory(p.getParent())`, `lua.execFile(p)` (skin_config == nil) -> `t.header` -> `fromLuaValue(JsonSkin.Skin.class, value)` -> `loadJsonSkinHeader`. (BJ/skin/lua/LuaSkinLoader.java:49-61). 이 경로의 `main_state`/`timer_util`/`event_util` 는 빈 테이블이다(SkinLuaAccessor.java:95-102).
2. 실제 화면 진입: `new LuaSkinLoader(state, config)` (BJ/skin/SkinLoader.java:68) -> 생성자에서 `exportMainStateAccessor(state)`, `exportUtilities(state)` 호출(BJ/skin/json/JSONSkinLoader.java:80-85). `load()` (LuaSkinLoader.java:68-95)는 (a) `loadHeader(p)` 재호출(모듈 캐시 적중), (b) `header.setSkinConfigProperty(property)` 로 옵션/파일/오프셋 선택값 적용, (c) 선택된 커스텀 파일로 `filemap` 구성, (d) `lua.exportSkinProperty(...)` 로 전역 `skin_config` 설정, (e) `lua.execFile(p)` 로 `.luaskin` 재실행 -> `t.main()` 호출, (f) 반환 테이블을 `JsonSkin.Skin` 으로 변환해 `loadJsonSkin`.
3. Lua VM 수명: `SkinLuaAccessor` 하나 = `Globals` 하나 = 스킨 로드 1회. 헤더 단계와 본체 단계는 같은 VM 을 공유하므로 모듈 캐시(`package.loaded`)가 유지되어, 본체 Lua 의 최상위 코드(전역 `PROPERTY` 설정, `header` 생성)는 한 번만 실행되고 `main()` 만 본체 단계에서 실행된다. 화면이 바뀌면(예: 선곡 -> 플레이) 새 VM 이다.
4. 본체 단계 `main()` 은 스킨 "구성" 시점(화면 진입 직후, 1회)에 실행된다. 이 시점에 `main_state.option/number/text` 를 읽는 코드가 있으므로(예: MC/Play/lua/base.lua:230 `main_state.option(MAIN.OP.NO_BGA)`, MC/Decide/lua/require/textproperty.lua:35 `diffOutlineColor()`, MC/Result/lua/require/textproperty.lua:20-25) R-BMS 는 "곡/결과 데이터가 확정된 뒤" 스킨을 구성해야 하고, 화면마다 새 VM 으로 다시 구성해야 한다.
5. 매 프레임 평가되는 것은 `main()` 이 만들어 둔 Lua 함수들뿐이다: `draw`, `value`, `timer`, `act`, 텍스트 `value`, `op` 에 쓰인 함수(7장 참조).

### 2.3 전역 변수 (본체 Lua 가 설정, 파트 파일이 소비)

| 전역 | 설정 위치 | 단계 | 내용 |
|---|---|---|---|
| `skin_config` | 엔진(BJ SkinLuaAccessor.java:925-980) | 본체 단계 | `get_path(rel)`, `option[이름]=선택 op`, `offset[이름]={x,y,w,h,r,a}`, `file_path[이름]`, `enabled_options[]` |
| `main_state` | 본체 Lua 최상위 `main_state = require("main_state")` (play*/musicselect/result/course/decide/keyconfig/skinselect 첫 줄 근처, 예: MC/play7_hw.lua:8) | 헤더 단계 포함 | 엔진 모듈. 헤더 단계에서는 빈 테이블 |
| `timer_util` | musicselect.lua:9, result.lua:9, course.lua:9 | 헤더 단계 포함 | 엔진 모듈. 사용하는 함수는 `timer_observe_boolean` 하나(148회) |
| `PROPERTY` | 본체 Lua 최상위 (예: play7_hw.lua:10 `require("Play.lua.require.sp_property").load(false)`) | 헤더 단계 | 사용자 정의 옵션/파일/오프셋/카테고리 정의 + `isXxx()` 조건 함수 |
| `COMMONFUNC` | play*_hw.lua:11 `require("Play.lua.require.common")` | 헤더 단계 | 플레이 전용 보조 함수(offsetBombSize, offsetBarlineBright, offsetJudgelineHeight, offsetGlowlampHeight, setKeybeamHeight, setTimeKeyOff, setRGB) |
| `MAIN` | 각 `main()` 첫 줄 `require("Root.define")` | 본체 | 상수 표(ACC, BLEND, FILTER, N_ALIGN, T_ALIGN, G_ANGLE, S_ANGLE, I_CLICK, T_WRAPPING, T_OVERFLOW, STRETCH, N_ZEROPADDING, JUDGEGRAPH, TIMINGDISTRIBUTIONGRAPH, TIMINGVISUALIZER) + OP/NUM/TIMER/STRING/BUTTON/OFFSET/GRAPH/SLIDER/IMAGE |
| `CUSTOM` | 각 `main()` `require("Root.define2")` | 본체 | LOAD_HEADER, ADD_ALL, sectionScore(상태), OP/NUM/GRAPH/SLIDER/FUNC/CLOCK/TEXT/SOUND/TIMER |
| `CONFIG` | 각 `main()` `require("config")` | 본체 | MC/config.lua 테이블(8장) |
| `BASE` | play*_hw.lua:20 `createBasePositionSP/DP(keys)` | 플레이 본체 | 레인/키플래시/게이지 좌표 기준 |
| `RESULT_BASE` | result.lua:19, course.lua:19 | 결과 본체 | MAIN_POS_X, SUB_POS_X, SCROLLBAR_POS_X, CENTER_POS_X |
| `DEBUG` | 항상 주석 처리(`-- DEBUG = true`), nentyakuinfo.lua:18 만 `DEBUG = false` 대입 | - | 사실상 nil. `if DEBUG then print(...)` 전부 비활성 |

`MAIN`/`CUSTOM`/`CONFIG` 는 `main()` 안에서만 설정되므로, 헤더 단계에 이들을 참조하는 코드가 있으면 안 된다(현재 없음). 단 `Root.define2` 의 하위 모듈 `customnumber.lua` 는 최상위에서 `skin_config.get_path` 와 `io.open` 을 호출한다(MC/Root/customnumber.lua:289-303). 그러므로 `Root.define2` 는 반드시 `skin_config` 가 존재하는 본체 단계에서만 `require` 되어야 한다.

### 2.4 Root/define.lua, define2.lua

MC/Root/define.lua
- 7-158: 상수 표. `ACC{CONSTANT=0,ACCELERATION=1,DECELERATE=2,DISCONTINUOUS=3}`, `BLEND{OFF=0,ALPHA=1,ADDITION=2,SUBTRACTION=3,MULTIPLY=4,XOR=6,MULTIPLYINVERSION=9,INVERSION=10,MULTIPLYALPHA=11}`, `FILTER{OFF=0,ON=1}`, `N_ALIGN{RIGHT=0,LEFT=1,CENTER=2}`, `T_ALIGN{LEFT=0,CENTER=1,RIGHT=2}`, `G_ANGLE{RIGHT=0,DOWN=1}`, `S_ANGLE{UP=0,RIGHT=1,DOWN=2,LEFT=3}`, `I_CLICK{OFF=0,ON=1,SEPARATE=2}`, `T_WRAPPING{OFF=false,ON=true}`, `T_OVERFLOW{NOTHING=0,SHRINK=1,TRUNCATION=2}`, `STRETCH{STRETCH=0,FIT_INNER=1,FIT_OUTER=2,FIT_OUTER_TRIMMED=3,FIT_WIDTH=4,FIT_WIDTH_TRIMMED=5,FIT_HEIGHT=6,FIT_HEIGHT_TRIMMED=7,NO_EXPANDING=8,NO_RESIZE=9,NO_RESIZE_TRIMMED=10}`, `N_ZEROPADDING{OFF=0,ON=1}`, `JUDGEGRAPH{NOGAP{OFF=0,ON=1},ORDERREVERSE{OFF=0,ON=1},TYPE{NOTES=0,JUDGE=1,FASTSLOW=2},BACKTEX{OFF=0,ON=1}}`, `TIMINGDISTRIBUTIONGRAPH{DRAW_AVERAGE{OFF=0,ON=1},DRAW_DEV{OFF=0,ON=1}}`, `TIMINGVISUALIZER{TRANSPARENT{...},DRAWDECAY{...}}`.
- 160-168: `m.OP/NUM/TIMER/STRING/BUTTON/OFFSET/GRAPH/SLIDER/IMAGE = require("Root.mainoption" ... "Root.mainimage")`.
- ModernChic 이 실제로 쓰는 열거값: ACC 3종(DECELERATE 81, ACCELERATION 8, CONSTANT 2), BLEND 2종(ADDITION 71, ALPHA 21), FILTER OFF 34/ON 3, N_ALIGN RIGHT 59/CENTER 23/LEFT 5, T_ALIGN CENTER 64/RIGHT 16/LEFT 13, G_ANGLE RIGHT 11/DOWN 6, S_ANGLE DOWN 20/RIGHT 7/UP 2, I_CLICK.SEPARATE 2, T_OVERFLOW.SHRINK 38 (숫자 리터럴 `overflow = 1` 도 다수), STRETCH FIT_WIDTH_TRIMMED 12/FIT_OUTER_TRIMMED 10/FIT_INNER 3, N_ZEROPADDING ON 38/OFF 2, JUDGEGRAPH.TYPE NOTES 6/JUDGE 3/FASTSLOW 3, BACKTEX OFF 9/ON 2, NOGAP.OFF 3, ORDERREVERSE OFF 2/ON 1, TIMINGDISTRIBUTIONGRAPH.DRAW_AVERAGE.ON 1, DRAW_DEV.ON 1. (T_WRAPPING, TIMINGVISUALIZER 는 미사용.)

MC/Root/define2.lua
- 8-12 `m.LOAD_HEADER(skin, header)`: `for i, v in pairs(header) do skin[i] = v end`. 헤더 테이블의 모든 키를 skin 테이블에 얕은 복사한다. 결과적으로 `main()` 이 반환하는 테이블에도 `type,name,w,h,scene,input,fadeout,loadend,playstart,close,property,filepath,offset,category,author(,ver)` 가 그대로 들어 있다.
- 14-20 `m.ADD_ALL(list, t)`: `t` 가 truthy 일 때 `for i, v in ipairs(t) do table.insert(list, v) end`. nil 안전, `ipairs` 이므로 첫 nil 에서 중단, 원본 순서 유지, 깊은 복사 없음.
- 22-27 `m.sectionScore`: 구간 점수 상태 테이블 4개(oneFour, twoFour, threeFour, fourFour), 각 `{nFlg=true,sFlg=true,gFlg=true,myScore=0,tgtDiff=0}`. VM 수명 동안 유지되는 가변 상태(customnumber/customsound/customgraph 가 변경).
- 29-37 서브모듈: `OP=Root.customoption, NUM=Root.customnumber, GRAPH=Root.customgraph, SLIDER=Root.customslider, FUNC=Root.customfunction, CLOCK=Root.customtime, TEXT=Root.customtext, SOUND=Root.customsound, TIMER=Root.customtimer`.
- define2 가 `require` 하는 순서 때문에 `customnumber` 최상위 코드(파일 읽기)는 `CUSTOM = require("Root.define2")` 시점(각 `main()` 의 두 번째 줄)에 즉시 실행된다.

### 2.5 파트 파일 로딩 패턴 (모든 화면 공통)

```
do
  local xxx_path = skin_config.get_path("Play/lua/sp/info.lua")
  local xxx_status, xxx_parts = pcall(function() return dofile(xxx_path).load() end)
  if xxx_status and xxx_parts then
    CUSTOM.ADD_ALL(skin.image, xxx_parts.image) ... (키별 병합)
    CUSTOM.ADD_ALL(skin.destination, xxx_parts.destination)
  end
end
```

- 집계: `pcall` 110회(스킨 본체 107 + Select/http.lua 3), `dofile` 109회(107 + 보호 없는 2회: play5_hw.lua:270, play7_hw.lua:260 의 `bgaareainfo.lua`).
- `skin_config.get_path(상대경로)` 는 절대 경로 문자열을 돌려주고 그것을 `dofile` 한다. 따라서 `dofile` 은 절대 경로를 지원해야 한다. `dofile` 은 캐시가 없어 매번 파일을 다시 실행한다. 각 파트 파일은 `return { load = load }` 형태.
- 오류 정책: `pcall` 이 오류를 삼키고 메시지를 버린다(상태가 false 면 해당 파트만 조용히 생략, 로그 없음). 보호 없는 `bgaareainfo` 오류와 `main()` 중간의 모든 비보호 오류는 `main()` 전체를 실패시키고, beatoraja 는 `catch Throwable -> printStackTrace -> null`(BJ/skin/lua/LuaSkinLoader.java:91-95)이라 스킨 로드 실패(기본 스킨으로 대체 추정)가 된다. R-BMS 는 "파트 오류는 생략 + 반드시 로그" 로 구현하는 것을 권한다(동작은 beatoraja 와 동일하되 진단 가능).
- 파트 병합 순서가 곧 `skin.destination` 의 그리기 순서(뒤가 위)다. 파트별 키 병합 목록은 각 화면의 본체 Lua 에 있으며 아래 2.6 에 순서를 정리한다.

### 2.6 화면별 skin 테이블 키와 파트 호출 순서

공통: 초기화되는 키는 아래와 같고, `skin.source` 는 본체 Lua 가 직접 + 파트가 추가한다.

| 화면 | main() 이 만드는 최상위 키 |
|---|---|
| Play (5/7/10/14) | source, font, text, image, note, value, slider, hiddenCover, liftCover, graph, bga(`{id="bga"}`), judgegraph, bpmgraph, timingvisualizer, hiterrorvisualizer, judge, gauge, destination |
| Select | source, font, text, image, imageset, graph, slider, value, songlist, customTimers(빈 테이블), judgegraph, bpmgraph, destination |
| Decide | source, font, text, image, value, judgegraph, bpmgraph, destination |
| Result / Course | source, font, text, image, imageset, value, slider, graph, gauge, gaugegraph, judgegraph, bpmgraph, timingdistributiongraph, customTimers(빈), customEvents(빈), destination |
| KeyConfig | source(빈), font, text, image, value, judgegraph, bpmgraph, destination(빈) |
| SkinSelect | source(2개), font, text, image, imageset, value, slider, destination (skinSelect 객체는 주석 처리) |

`customTimers`/`customEvents` 는 빈 테이블로만 존재한다. `CUSTOM.TIMER.GET_CUSTOMTIMER_ID()` 는 전체 스킨에서 호출되지 않는다(grep 0건). 즉 ModernChic 은 커스텀 타이머/이벤트를 하나도 정의하지 않는다.

파트 호출 순서(= 그리기 순서)
- play7/play5 (SP): background(load(3)) -> sp/info -> sp/progress -> sp/keyflash(keys) -> sp/gauge -> sp/lane -> sp/inputkey(keys) -> sp/notes(keys, `skin.note` 대입) -> sp/cover -> (play5 만 `5keysFrame` 직접 삽입, `is5keyLanecoverOn` 시) -> sp/assist -> sp/bomb(keys) -> sp/info2 -> sp/judge(`skin.judge` 병합) -> sp/graph -> sp/fullcombo -> sp/scorebar -> (AttackMode 켜짐 시) sp/attack -> (DetailInfo 켜짐 시, 비보호) sp/detailinfo/bgaareainfo(load(8 또는 5키는 6)) -> sp/prepare -> close(load(21)) -> 마지막에 `MAIN.IMAGE.BLACK` 페이드아웃 destination(timer=MAIN.TIMER.FADEOUT, loop=500, dst 2점 a:0->255) 직접 삽입. (play7_hw.lua:298-303)
- play10/play14 (DP): background(load(3)) -> dp/keyflash(keys) -> dp/info -> dp/progress -> dp/lane -> dp/gauge -> dp/inputkey(keys) -> dp/notes(keys) -> dp/cover -> dp/judge -> dp/prepare -> dp/scorebar -> dp/assist -> dp/bomb(keys) -> dp/graph -> dp/fullcombo -> close(load(21)) -> 페이드아웃. (DP 에는 info2/attack/bgaareainfo 없음, 순서가 SP 와 다름)
- musicselect: (changeLang 로 source 3,4,5,6,10,11 언어별 이미지 추가, 7/8/9 고정) -> infoOutput(5) 호출 -> background -> songlist -> mainframe -> musicdisplay -> btnarea -> cource -> info -> score -> bmsanalysis -> qco -> rivalview -> sidemenu -> (versioncheck 블록은 `--[[ ... ]]` 로 주석 처리됨, musicselect.lua:215-227) -> (viewHistory 켜짐 시) history -> help -> startanimation -> option -> assistoption -> suboption.
- result: infoOutput(7) -> `Result.lua.base.addSource`(source 0-8) -> background -> (`MAIN.IMAGE.BLACK` 1920x50 + `bottomResult` 텍스트 직접 삽입) -> centerinfo(load(0)) -> impression -> mainmenu(load(0), `skin.gauge` 대입) -> (IR 메뉴 켜짐 and `main_state.option(MAIN.OP.ONLINE)`) irmenu -> (StartAnimation 켜짐) prepare -> diflist -> fadeout -> (UpdateHistory 켜짐) history -> (`CONFIG.voice.result.sw`) `CUSTOM.SOUND.resultVoice()` 즉시 호출.
- course: result 와 동일하나 impression, diflist 없음, `bottomCourse`, centerinfo/mainmenu 인자 1, voice 없음.
- decide: 본체 Lua 내부 함수 background(source 1/2/3/4) -> lockonAnimation -> text -> notesGraph -> fadeout. 파트 파일 없음.
- skinselect: 본체 Lua 내부 함수 background -> skinMenu -> playSkin -> setItems -> skinSelector -> skinName -> skinAuthor -> slider. 파트 파일 없음.
- keyconfig: 본체는 빈 스킨을 반환한다(아래 결함 참조).

### 2.7 알려진 결함: KeyConfig 는 beatoraja 에서도 로드 실패

MC/keyconfig.lua:19 가 `require("Decide.lua.require.textproperty")` 를 호출하는데, 그 모듈 최상위(MC/Decide/lua/require/textproperty.lua:37)는 `PROPERTY.isOutlineFont()` 를 호출한다. keyconfig.lua 의 `PROPERTY` 는 `KeyConfig.lua.require.property` 이고 이 모듈에는 `isOutlineFont` 가 없다(MC/KeyConfig/lua/require/property.lua:73-86: property/filepath/category 가 전부 빈 테이블). 따라서 `main()` 이 `attempt to call a nil value (field 'isOutlineFont')` 로 실패한다. 설령 성공해도 `destination` 이 비어 있어 아무것도 그리지 않는다. 결론: ModernChic 의 KeyConfig 는 사실상 미완성이므로 R-BMS 는 이 화면을 ModernChic 으로 그리려 하지 말고 내장 키 설정 화면(또는 기본 스킨)을 쓰는 것이 beatoraja 와 동일한 결과다. 또한 Decide 의 textproperty 는 `PROPERTY` 에 `isOutlineFont/isBitmapFont` 가 있어야만 동작한다.

---

## 3. 10개 .luaskin 헤더 요약

공통 값: `w=1920, h=1080`, `author="KASAKO"`(MC/Root/author.lua), `Root.version` = 4.6 (숫자, MC/Root/version.lua:2). `name` 은 `"..." .. ver` 이므로 "…-4.6".

| .luaskin | 본체 | type | name | scene | input | fadeout | loadend | playstart | close | property | filepath | offset(+엔진 내장) | category |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| play7_hw | play7_hw.lua | 0 | ModernChicPlay(SCURO)-4.6 | 3600000 | 0 | 500 | 3500 | 1000 | 3000 | 36 | 20 | 10 (+4) | 14 |
| play5_hw | play5_hw.lua | 1 | 동일 | 3600000 | 0 | 500 | 3500 | 1000 | 3000 | 37 (5키 커버 옵션 추가) | 20 | 10 (+4) | 14 |
| play14_hw | play14_hw.lua | 2 | 동일 | 3600000 | 0 | 500 | 3500 | 1000 | 3000 | 30 | 20 | 10 (+4) | 14 |
| play10_hw | play10_hw.lua | 3 | 동일 | 3600000 | 0 | 500 | 3500 | 1000 | 3000 | 31 (10키 커버 옵션 추가) | 20 | 10 (+4) | 14 |
| musicselect | musicselect.lua | 5 | ModernChicSelect-4.6 | 3000 | 500 | 500 | - | - | - | 15 | 3 | 1 | 4 |
| decide | decide.lua | 6 | ModernChicDecide-4.6 | 3000 | 500 | 1000 | - | - | - | 6 | 2 | 0 (키 없음) | 3 |
| result | result.lua | 7 | ModernChicResult-4.6 | 3600000 | 2500 | 1000 | - | - | - | 20 | 45 | 2 | 16 |
| course | course.lua | 15 | ModernChicResult-4.6 (result 와 동일 이름) | 3600000 | 2500 | 1000 | - | - | - | 17 | 25 | 2 | 12 |
| keyconfig | keyconfig.lua | 8 | ModernChicKeyConfig-4.6 | 3000 | 500 | 1000 | - | - | - | 0 | 0 | 0 (키 없음) | 0 |
| skinselect | skinselect.lua | 9 | ModernChicSkinSelect-4.6 | 3000 | 500 | 1000 | - | - | - | 0 | 0 | 0 (키 없음) | 0 |

근거: Play header MC/Play/lua/require/header.lua:8-27, Select MC/Select/lua/require/header.lua:7-21, Decide/KeyConfig/SkinSelect header 7-19, Result header 8-25. type 값은 BJ/skin/SkinType.java:12-30 와 일치(0=7K,1=5K,2=14K,3=10K,5=선곡,6=결정,7=결과,8=키설정,9=스킨선택,15=코스결과). 헤더 코멘트의 "2:12k" 는 오기이며 14K.

헤더 필드 처리 규칙(BJ/skin/json/JsonSkin.java:7-57)
- 읽는 필드: `type, name, author, w, h, fadeout, input, scene, close, loadend, playstart, judgetimer(기본 1), finishmargin(기본 0), category, property, filepath, offset`. `ver` 는 알 수 없는 필드로 무시된다(Json 의 ignoreUnknownFields, 그리고 Lua 변환은 이름이 일치하는 필드만 설정, LuaSkinLoader.java:169-206).
- `w,h` = 1920x1080 이고 BJ 해상도 열거형과 매치되면 그 해상도를 원본으로 삼는다(JSONSkinLoader.java:263-269).
- 유일한 "빈 값" 규칙: 키 자체가 없으면 필드 기본값(offset 은 길이 0 배열).

### 3.1 property / filepath / offset / category 구조 (헤더가 정의하는 사용자 설정)

- `property[i] = {name, category, def, item = {{name, op}, ...}}`
  - `category` 는 문자열 필드이며 ModernChic 은 정수 라벨(1,2,3,...)을 넣는다. Lua 숫자 -> 문자열 변환(`tojstring`)으로 "1","2",... 가 된다. `category[k].item` 도 라벨 정수 배열이며 문자열로 변환되어 `property.category` 와 `equals` 비교된다(JSONSkinLoader.java:143-149). 정수 라벨이 float(1.0) 로 들어오면 안 된다.
  - `def` 는 항목 `name`(문자열) 으로, 일치하는 항목의 `op` 가 기본 선택이 된다. 일치 항목이 없으면 첫 항목이 선택된다(BJ/skin/SkinHeader.java:294-300).
  - 사용자가 무엇도 저장하지 않았고 `option.value == -1`(OPTION_RANDOM_VALUE) 이면 항목 중 무작위 선택(SkinHeader.java:152-172).
- `filepath[i] = {name, path, category, def}`: `path` 는 스킨 폴더 기준 상대 경로이며 `*` 와일드카드를 포함(예: "Play/parts/common/bg/*.png"). `def` 는 "#default", "diamond SCUROed.", "DEFAULT", "harf", "yuki", "BGmovie01", "sample" 등 파일명 stem.
  - 선택 파일이 저장돼 있지 않으면 beatoraja 로더는 `def` 를 적용하지 않고 디렉터리에서 확장자가 맞는 파일 중 무작위로 고른다(BJ/skin/SkinLoader.java:107-127). `def` 해석(전체 파일명 또는 확장자 제외 stem, 대소문자 무시)은 설정 UI 쪽에만 있다(BJ/config/SkinConfiguration.java:327-340, launcher/SkinConfigurationView.java:404-412). R-BMS 는 로드 시점에 `def` 를 적용해야 결정적이다.
- `offset[i] = {name, id, category, x|y|w|h|a|r}`: ModernChic 은 `{name=..., id=40, category=.., a = 0}` 형태로 쓴다. 이 필드들의 Java 타입은 boolean(편집 가능 여부)이고 Lua 값 0 은 truthy 이므로 "해당 성분을 사용자가 편집할 수 있다"는 뜻이다(JsonSkin.java:78-88, LuaSkinLoader.java:98-99 `put(boolean.class, LuaValue::toboolean)`). 값 0 자체는 의미 없다. R-BMS 의 Lua->스키마 변환이 숫자 0 을 false 로 취급하면 오프셋 편집 항목이 전부 사라진다.
- 플레이 스킨 헤더에는 엔진이 오프셋 4개를 자동 추가한다: "All offset(%)" id=10 (x,y,w,h), "Notes offset" id=30 (h), "Judge offset" id=32 (x,y,w,h,a), "Judge Detail offset" id=33 (x,y,w,h,a) (BJ/skin/json/JSONSkinLoader.java:167-191). ModernChic 은 `MAIN.OFFSET.JUDGE_1P`(32) 를 180회, `LIFT`(3) 224회, `LANECOVER`(4) 10회, `SCRATCHANGLE_1P/2P`(1/2), `NOTES_1P`(30) 를 destination 의 `offset` 으로 쓴다. 이 내장 id 의 표(1,2,3,4,5,10,30,32,33)는 BJ 와 일치한다(SkinProperty.java:944-958).
- 번호 부여 규칙(모든 property.lua 공통): 사용자 정의 옵션 op 는 900부터 순차 증가(`customoptionNumber=899` 에서 +1, 한도 999 초과 시 print 경고만), 카테고리 라벨은 1부터(parent/filepath/offset 이 하나의 카운터 공유), 오프셋 id 는 40부터. 한 번 `load()` 가 끝나면 카운터는 되돌려지지 않는다(모듈 지역 변수). 같은 VM 에서 `load()` 를 두 번 호출하면 번호가 밀린다. 그래서 스킨 로드마다 새 VM 이 필요하다.

화면별 번호 대역(실측: `customoption.chiled/parent/filepath/offset` 호출 수)

| 속성 모듈 | parent | chiled(op 수) | 사용 op 대역 | filepath | offset | 카테고리 라벨 최댓값 | offset id |
|---|---|---|---|---|---|---|---|
| sp_property (5/7키 공유) | 37 | 96 | 900-995 | 20 | 10 | 67 | 40-49 |
| dp_property (10/14키 공유) | 31 | 82 | 900-981 | 20 | 10 | 61 | 40-49 |
| Select property | 15 | 32 | 900-931 | 3 | 1 | 19 | 40 |
| Decide property | 6 | 12 | 900-911 | 2 | 0 | 8 | - |
| Result property (result/course 공유) | 20 | 47 | 900-946 | 45 | 2 | 67 | 40-41 |
| KeyConfig / SkinSelect | 0 | 0 | - | 0 | 0 | 0 | - |

(Result 는 course 용 `load(false)` 에서도 parent 20/op 47 을 전부 할당하므로 result/course 의 op 번호가 동일. 사용되지 않는 parent 도 번호를 소비한다.)

- 조건 함수: `chiled()` 가 돌려주는 `condition = function() return skin_config.option[pName] == num end`. 즉 `skin_config.option` 은 "옵션 이름(일본어 문자열) -> 선택된 op 정수" 테이블이다. 호출 시점에 `skin_config` 가 있어야 한다.
- 오프셋 접근: `skin_config.offset[이름].a/w/h/x/y`(평가 시점). 이름 키는 `customoption.offset(name)` 에 준 일본어 문자열이며 `{x,y,w,h,r,a}` 모두 정수(설정에 없으면 0)로 노출된다(SkinLuaAccessor.java:956-979).
- 이름 중복(키 충돌) 주의: SP/DP 의 옵션 "ゲージ"(gaugeSwitch)와 파일 "ゲージ"(gaugeParts) 이름이 같고(서로 다른 종류라 `skin_config.option` 과 `file_path` 에서는 분리), Result 의 파일 이름 "背景（Failed）"와 "キャラクター（Failed）" 가 각각 두 번 정의되어(isclear/failed 와 clearType/failed) 이름 기준 선택값이 두 경로에 동시에 적용된다(SkinHeader.java:174-204 `customFile.name.equals(file.name)`). 선택 저장소를 이름으로 키잉하면 beatoraja 와 같은 동작이 된다.
- header.category: 플레이 SP 14개(13개 + 5키/7키에서 `table.insert(module.category, #module.category, {name="パーツ表示有無",...})` 로 끝에서 두 번째에 삽입; sp_property.lua:560-587), DP 14개(index 12 에 삽입; dp_property.lua:515-538), Select 4, Decide 3, Result 16(기본 12 + index 7, 12 삽입 + 말미 2 추가; property.lua:376-480), Course 12.

---

## 4. 한 스킨을 로드하는 데 필요한 최소 Lua 런타임 요구사항 체크리스트

A. 언어 수준
1. Lua 5.1/5.2 호환 문법이면 충분하다. goto, 정수 나눗셈 `//`, 비트 연산자, 메타테이블, 코루틴, 가변 인자(`...`), `select`, `setfenv/getfenv`, `loadstring`, `string:method` 형태의 문자열 메서드 호출, `unpack`(전역) 모두 사용되지 않는다(전수 grep 0건). BJ 의 실제 런타임은 LuaJ(Lua 5.2 의미)다.
2. 숫자/문자열 변환 의미: Lua 숫자를 필드 문자열로 바꾸는 곳이 있다(`src = 3`, `id = 0`, 카테고리 라벨, source/destination id 에 숫자 사용). 정수값 float 는 "960" 처럼 정수 표기여야 한다("960.0" 금지). `1920 / 2`, `27 * 12` 같은 좌표식이 float 로 나오는 것은 Lua 5.4 에서 float(960.0) 이므로 정수 필드로 변환할 때 절단(toward zero)이 필요하다.
3. 필드 변환 규칙(LuaSkinLoader.java:105-170): int=`toint`(절단), float=`tofloat`, boolean=`toboolean`(0 도 true), String=`tojstring`. 함수형 필드 6종: BooleanProperty(`draw`, `op` 항목), IntegerProperty(`value` 등), FloatProperty, StringProperty(텍스트 `value`), TimerProperty(`timer`), Event(`act`). 숫자면 해당 id, 문자열이면 이름/스크립트, 함수면 Lua 함수. `op` 배열의 숫자는 id(음수 = 부정, 예 `op = {-MAIN.OP.IR_WAITING}`), 함수/문자열도 가능(DestinationOption, LuaSkinLoader.java:146-154).

B. 표준 라이브러리 (실제 사용 함수만)
4. base: `print`(118회, 출력만), `pairs`, `ipairs`, `tostring`, `tonumber`, `pcall`, `require`, `dofile`. (미사용: `error`, `assert`, `type`, `select`, `next`, `rawget/rawset`, `setmetatable/getmetatable`, `xpcall`, `load`, `loadstring`, `loadfile`.)
5. string: `sub`(2), `format`(2: `"%02d"` 만), `match`(1), `gsub`(1), `gmatch`(1). `string.match(b, "Play.+%.png")` 같은 Lua 패턴 사용(customfunction.lua:125, listchoice 의 "Result.+%.png", background 의 "Select.+%.png"/".mp4").
6. table: `insert`(2200, 마지막 위치 삽입과 `table.insert(t, pos, v)` 모두 사용: sp_property.lua:567, dp_property.lua:522, Result property.lua:442,456), `concat`(2).
7. math: `random`(13: `random(n)` 와 `random(a,b)`), `floor`(12), `modf`(4), `max`(1), `exp`(1), `ceil`(1). 시드는 지정하지 않으며(비결정) 회전 기능의 무작위성이 이에 의존한다. RB 현재 샌드박스처럼 고정 시드(`math.randomseed(RANDOM_SEED)`, RB/crates/rbms-skin/src/lua.rs:37,121)로 하면 "무작위" 배경/레인커버/팁이 항상 같아진다.
8. io: `io.open` 45회, 모드는 읽기(기본/"r"), "w", "a" 3종. 파일 메서드 `lines`(9), `write`(41), `close`(42). 요구사항: `f:write()` 인자 0개 허용(customfunction.lua:172 `resetFile`), `f:write(x)` 가 파일 자신을 반환(`io.open(p,"a"):write(s):close()` 체인, customfunction.lua:235, listchoice.lua:44 등), 존재하지 않는 파일 읽기는 `nil, 메시지` 반환(`existFile` 이 이를 사용), 쓰기 모드는 상위 디렉터리를 자동 생성(BJ RestrictedIoLib.openFile:349-353). 줄 읽기는 "\n" 구분(런타임이 만든 파일은 LF 만).
9. os: `os.date`(5: "%y%m%d", "%Y/%m/%d", "%H:%M:%S", "%Y-%m-%d", `'*t'` 의 `.wday`), `os.time`(3). BJ 는 `os.execute/exit/getenv/remove/rename/tmpname` 만 제거(SkinLuaAccessor.java:138-150). 현지 시간대 기준(UTC 아님): History 폴더명이 `os.date("%y%m%d")` 이고 bgaareainfo 는 `os.time() - 3600*6` 로 "오전 6시 경계"의 날짜 파일을 만든다(bgaareainfo.lua:150).

C. 모듈/경로
10. `package.path` 에 `<스킨폴더>/?.lua` 포함, `require("A.B.C")` 는 `A/B/C.lua`(점 -> 디렉터리 구분자). 모듈 캐시 유지(`package.loaded`). 사용되는 모듈 이름 전수: main_state, timer_util, luajava, Root.define, Root.define2, Root.main{option,number,timer,string,button,offset,graph,slider,image}, Root.custom{option,number,graph,slider,function,time,text,sound,timer}, Root.version, Root.author, config, Play.lua.base, Play.lua.require.{header,common,sp_property,dp_property,textproperty}, Play.lua.sp.detailinfo.{util,trend,rendafrequency,nentyakuinfo,inputinformation}, Select.lua.require.{header,property,textproperty,http,settings}, Decide.lua.require.{header,property,textproperty}, KeyConfig.lua.require.{header,property}, SkinSelect.lua.require.{header,property,textproperty}, Result.lua.base, Result.lua.require.{header,property,textproperty,listchoice}, 루트의 play5_hw/play7_hw/play10_hw/play14_hw/musicselect/decide/result/course/keyconfig/skinselect.
11. `require("main_state")`, `require("timer_util")` 가 헤더 단계에서 빈 테이블로도 성공해야 한다. `event_util` 은 ModernChic 이 쓰지 않는다. `require("luajava")` 가 성공해야 한다(Root/customfunction.lua:7 이 `Root.define2` 를 통해 모든 본체 단계에서 로드되므로 필수).
12. `dofile(절대경로)` 지원. 같은 VM 의 같은 전역 환경에서 실행.
13. 헤더 단계 최소 요건: `.luaskin` 실행 + `require` + 속성 모듈 `load()` 만으로 header 가 만들어진다. 이 단계에는 `skin_config` 가 nil 이어야 한다(그렇지 않으면 `t.main()` 이 호출되어 버린다).

D. 엔진이 노출해야 하는 API
14. `skin_config`: `get_path(rel)`(238회), `option[이름]`(7), `offset[이름].{a,w,h,x,y}`(35). 세부는 5.2.
15. `main_state`: `option, number, text, time, timer, timer_off_value, event_index, gauge_type, judge, volume_sys/key/bg, set_volume_sys/key/bg, rate, float_number, gauge, exscore, audio_play` (5.1).
16. `timer_util.timer_observe_boolean(func)` (5.3).
17. `luajava` 파사드 (5.4).
18. 평가 오류 정책: BJ 는 속성 평가 중 오류를 잡아 경고 로그만 남기고 값을 기본값으로 대체한다(boolean=false, int=0, float=0, string="", timer=`Long.MIN_VALUE`(off), event=무시; SkinLuaAccessor.java:611-835). R-BMS 도 프레임 루프에서 패닉 없이 동일하게 처리해야 한다.
19. 타이머/불리언 반환 규칙: timer 함수의 반환값은 `tolong()` 이라 `nil`(반환 없음)이면 0(켜짐, 시작 시각 0)이 된다. 예: `Play/lua/sp/cover.lua:165-170` 의 `timer = function() ... end` 는 아무것도 반환하지 않는 "부수효과 훅"이다. `draw` 함수의 `nil`/`false` 는 거짓, 그 외(0 포함)는 참.
20. 모든 파일 접근은 스킨 폴더 내부로 제한되어야 하고(밖이면 오류), 쓰기가 가능한 위치여야 한다(9장).

---

## 5. 전역 API 사용 전수 집계

### 5.1 main_state (엔진 -> Lua). 총 683회 (grep -o 출현 횟수, 주석 안의 출현 포함)

| 함수 | 사용 횟수 / 파일 수 | 대표 위치 | BJ 구현 (MainStatePropertyLuaApiExporter / SkinAudioLuaApiExporter) |
|---|---|---|---|
| option(id) | 230 / 28 | result.lua:111, Play/lua/base.lua:230 | id 숫자 또는 이름 문자열 -> BooleanProperty.get(state). 부정 id 지원은 BooleanPropertyFactory 쪽(범위 외) |
| number(id) | 207 / 19 | Result/lua/centerinfo.lua:103 | IntegerProperty.get (정수). 숫자 id 만 사용(`379 + i`, `389 + indexNum`, `74`, `368`, `180` 같은 계산/리터럴 id 포함) |
| text(id) | 62 / 14 | Decide/lua/require/textproperty.lua:25 | StringProperty.get, 없으면 "" . `text(12)`(전체 제목), `text(119 + indexNum)`, `text(op[i])` 포함 |
| time() | 22 / 4 | Result/lua/irmenu.lua:70, Root/customoption.lua:100 | 현재 마이크로초(`state.timer.getNowMicroTime()`) |
| timer(id) | 16 / 5 | Root/customnumber.lua:282 | `state.timer.getMicroTimer(id)`: 켜진 시작 마이크로초 또는 꺼짐 = `Long.MIN_VALUE` |
| timer_off_value | 15 / 4 | Root/customoption.lua:45 | `Long.MIN_VALUE` 상수(-9223372036854775808). 비교용 |
| event_index(id) | 14 / 3 | Root/customoption.lua:53, Root/customslider.lua:25, Result/lua/require/listchoice.lua:72 | `IntegerPropertyFactory.getImageIndexProperty(id).get(state)` (버튼 현재 인덱스) |
| gauge_type() | 11 / 3 | Root/customoption.lua:9-41, Play/lua/sp/gauge.lua:216 | `BMSPlayer.getGauge().getType()`, 플레이어가 아니면 0 |
| judge(n) | 6 / 1 | Root/customnumber.lua:49-69 | `state.getJudgeCount(n,true)+getJudgeCount(n,false)` (0=PG,1=GR,2=GD,3=BD,4=PR,5=MS) |
| volume_sys/key/bg() | 각 2 | Root/customnumber.lua:25-33, Select/lua/volumecontrol.lua(미사용 파일) | 설정 AudioConfig 의 float(0~1) |
| set_volume_sys/key/bg(v) | 각 2 | Select/lua/sidemenu.lua:101-106 (`act = function() main_state.set_volume_sys(0) end`) | AudioConfig 에 즉시 기록, true 반환 |
| rate() | 2 | Root/customnumber.lua:37, Play/lua/sp/detailinfo/bgaareainfo.lua:527 | `ScoreDataProperty.getNowRate()` (0~1 float) |
| float_number(id) | 2 | Root/customoption.lua:92,96 (`MAIN.GRAPH.BESTSCORERATE`=113) | FloatProperty (그래프/비율 id). float 반환 |
| gauge() | 1 | Root/customnumber.lua:45 | 현재 게이지 값 float |
| exscore() | 1 | Root/customnumber.lua:41 | `getNowEXScore()` |
| audio_play(path[,vol]) | 82 / 1 (Root/customsound.lua) | Root/customsound.lua:28 이하 | 9장 "사운드 재생" 문단 참조 |

R-BMS 가 구현해야 하는 최소 집합 = 위 20개(함수 19 + 상수 `timer_off_value`). BJ 사본이 추가로 제공하지만 ModernChic 이 쓰지 않는 함수: `offset, numbers, timer_is_on/off, timer_elapsed(_ms,_seconds), set_timer, event_exec, key_pressed, screen_width/height, rate_best, exscore_best, rate_rival, exscore_rival, audio_loop/preload/stop/dispose, file_*, http_get(_lines)` (MainStatePropertyLuaApiExporter.java:31-64, SkinFileLuaApiExporter.java:23-65, SkinHttpLuaApiExporter.java:23-50). ModernChic 은 파일/HTTP 를 이 확장 API 가 아니라 표준 `io` + `luajava` 로 한다.

### 5.2 skin_config

| 항목 | 횟수 | 의미 / BJ 구현 |
|---|---|---|
| get_path(rel) | 238 | `getPath(<스킨폴더>/rel, filemap).getPath()`. 사용자 선택 파일이 있으면 `*` 부분을 선택 파일명으로 치환하고, 없으면 디렉터리에서 무작위 파일(BJ/skin/SkinLoader.java:95-130). 반환은 절대 경로이며 `new File(...).getPath()` 이므로 끝의 `/` 가 사라진다(`History/250811/` -> `.../History/250811`). 파일이 없어도 오류 없이 문자열 반환 |
| option | 7 (각 속성 모듈의 condition) | 이름 -> 선택 op 정수. 미저장 시 `def`/첫 항목/무작위 |
| offset | 35 (각 속성 모듈 5함수 x 7) | 이름 -> `{x,y,w,h,r,a}` 정수 |
| file_path / enabled_options | 0 | BJ 는 제공하지만 ModernChic 은 쓰지 않음 |

### 5.3 timer_util / event_util

- `timer_util.timer_observe_boolean(function)` 148회. 전부 destination 의 `timer = ...` 값으로만 쓰인다(sidemenu 71, mainmenu 41, irmenu 32, help 2, versioncheck 1, mainframe 1). 의미(BJ/skin/lua/TimerUtility.java:89-120): 인자 함수를 "타이머가 폴링될 때마다" 호출해 참이 되는 순간의 `state.timer.getNowMicroTime()` 을 타이머 값으로 래치하고, 거짓이 되면 `Long.MIN_VALUE`(꺼짐) 로 되돌리는 함수형 타이머를 만든다. 각 호출마다 독립 상태를 가진다.
- `event_util`: 사용 0회. 그 외 timer_util 함수(`now_timer, is_timer_on, is_timer_off, timer_function, new_passive_timer`)도 0회.

### 5.4 luajava (4개 파일)

| 파일:줄 | 사용 | 파사드가 지원해야 하는 것 (BJ LegacySkinLuaApi) |
|---|---|---|
| Root/customfunction.lua:7-8,103,114 | `luajava.bindClass("java.io.File")`, `luajava.new(File, path)`, `dir:mkdir()`, `dir:listFiles()` | `new` 는 File 만 허용. `mkdir` 는 단일 디렉터리 생성(`Files.createDirectory`, 부모 없으면 false), `listFiles` 는 **디렉터리 항목의 절대 경로 문자열 테이블**(구분자 `/`)을 돌려주고 실패 시 nil (LegacySkinLuaApi.java:118-148) |
| Result/lua/mainmenu.lua:5-7, Result/lua/irmenu.lua:6-8 | `bindClass("com.badlogic.gdx.Gdx")`, `bindClass("com.badlogic.gdx.Input")`, 그리고 `Gdx.input:isKeyPressed(input.Keys.RIGHT/LEFT/UP/DOWN)` | `Gdx.input:isKeyPressed(code)`, `Input.Keys.<NAME>` 은 libGDX 키코드 정수(BJ 파사드는 `Input.Keys.valueOf(이름)` 을 `__index` 메타메서드로 노출, 모르는 이름은 nil). mainmenu.lua:986-989, irmenu.lua:654-657 에서 UP/DOWN/LEFT/RIGHT 4개만 사용. libGDX 표준 코드는 UP=19, DOWN=20, LEFT=21, RIGHT=22 (표준값 기억 기반, BJ 사본 소스로는 직접 확인하지 않음). `Gdx` 는 `Gdx.input:isKeyPressed(code)` 한 함수만 필요하며 콜론 호출이므로 첫 인자(self)를 무시하고 마지막 인자를 키코드로 취급한다(LegacySkinLuaApi.java:313-318, 379-381) |
| Select/lua/require/http.lua:8,22,32 | `luajava.newInstance("java.net.URL", url)`, `openConnection/setRequestMethod("GET")/setConnectTimeout(200)/connect/getResponseCode/getInputStream`, `InputStreamReader`, `BufferedReader:readLine()` | 이 경로는 versioncheck.lua(주석 처리됨)에서만 도달 가능. 기본 설정에서는 실행되지 않음 |

BJ 사본의 `LegacySkinLuaApi` 주석에 "暫定対応, 将来削除予定" 이라 적혀 있다(LegacySkinLuaApi.java:32-38). 즉 beatoraja 쪽에서도 곧 제거될 수 있는 호환 계층이며, ModernChic 이 요구하는 정확한 API 가 이 파사드 범위와 일치한다(File/Gdx/Input/URL/InputStreamReader/BufferedReader 만 허용, 그 외 `bindClass` 는 오류).

### 5.5 require / dofile / pcall 등 로더 계열

- `require` 문자열 집계(상위): "main_state" 11, "Root.define2" 10, "Root.define" 10, "config" 10, "Play.lua.base" 8, "Root.version" 7, "Root.author" 6, "luajava" 4, "timer_util" 3, 그 외 1~4회.
- `dofile` 109, `pcall` 110 (2.5 참조). `loadfile/loadstring/setfenv/getfenv/xpcall/debug/package./bit32/collectgarbage/coroutine` 사용 0. `debug` 는 BJ 가 `debug.getmetatable` 파사드를 설치하지만 ModernChic 은 호출하지 않는다.

### 5.6 io / os 호출 위치

`io.open` 45회 전체 위치(파일:줄): Result/lua/diflist.lua:20; Result/lua/history.lua:12,30,43,71,96,117,138; Result/lua/impression.lua:76,342; Result/lua/require/listchoice.lua:44,50,56,65,73,122,128,134,143,151; Root/customfunction.lua:13,19,31,137,144,151,171,235,304,326; Select/lua/require/http.lua:58,74,86,94; Select/lua/require/settings.lua:7,16,41,64; Play/lua/sp/detailinfo/nentyakuinfo.lua:38,46,50,62; Play/lua/sp/detailinfo/bgaareainfo.lua:43,54,64.
`os.*`: Root/customtime.lua:6,8,10; Play/lua/sp/detailinfo/bgaareainfo.lua:148,150(x2); Select/lua/require/http.lua:87,121.

---

## 6. Root/main*.lua 이름표 (id 상수)

모두 BJ SkinProperty 상수와 같은 값이며(MC 각 파일 상단에 SkinProperty.java 링크), 선언 형태는 `local m = {}; m.NAME = 숫자; return m` 이다.

(사용 횟수 합은 주석 안의 출현을 포함한 grep -o 값이며, 서로 다른 이름 수와 부록 A 의 개별 횟수는 주석 줄을 제외한 값이다.)

| 모듈 | 정의 수 | ModernChic 이 실제로 참조하는 서로 다른 이름 수 | 사용 횟수 합 |
|---|---|---|---|
| OP (mainoption.lua) | 287 | 139 | 806 |
| NUM (mainnumber.lua) | 291 | 187 | 630 |
| TIMER (maintimer.lua) | 166 | 88 | 792 |
| STRING (mainstring.lua) | 82 | 60 | 143 |
| BUTTON (mainbutton.lua) | 81 | 70 | 125 |
| OFFSET (mainoffset.lua) | 13 | 6 | 419 |
| GRAPH (maingraph.lua) | 22 | 13 | 22 |
| SLIDER (mainslider.lua) | 9 | 7 | 16 |
| IMAGE (mainimage.lua) | 6 | 6 | 65 |

검증 결과(스크립트 m1_undefined_ids / m1_vs_beatoraja)
- 사용된 `MAIN.<ns>.<NAME>` 중 정의되지 않은 이름: 0개. 주석 처리된 `MAIN.BUTTON.TARGET`(MC/Play/lua/sp/scorebar.lua:109, dp/scorebar.lua:34)만 정의 없음(`--廃止 m.TARGET = 77`, mainbutton.lua:32).
- 같은 값을 갖는 중복 정의: NUM `RANKINGn_CLEAR` 와 `RANKINGn_INDEX` (390-399, 동일 값), OFFSET `JUDGE_1P/2P/3P`=32 와 `JUDGEDETAIL_1P/2P/3P`=33. NUM `JUDGERANK`=400 과 OP `CONSTANT`=400 과 BUTTON `CONSTANT`=400 은 서로 다른 네임스페이스.
- BJ SkinProperty 에 값이 없거나 범위/문자열 변환으로만 존재하는 사용 id (R-BMS 가 필요 시 직접 구현해야 하거나 빈 값으로 두어야 함):
  - STRING: `SELECTED_TARGET`=3, `TARGET_FORWARD1..10`=200-209, `TARGET_BACKWARD1..10`=210-219 -> BJ 사본에는 상수/구현이 없다(StringPropertyFactory 에 해당 항목 없음). 사본에서는 `main_state.text` 가 빈 문자열을 돌려주므로 Select 의 라이벌 목록 텍스트(`s_rival`, `f_rival1..10`, `b_rival1..10`)는 사본 기준으로 빈 텍스트다. 업스트림 0.8.8 에 기능이 있는지는 미확인.
  - STRING `SKIN_CUSTOMIZE_CATEGORY2..9`, `ITEM2..9`, `RANKING2..9_NAME`: BJ 는 1과 10 만 상수로 두고 범위로 처리(STRING_SKIN_CUSTOMIZE_CATEGORY1=100 ... 10=109).
  - NUM `RANKING2..9_EXSCORE/CLEAR/INDEX`: 동일하게 범위(380-389, 390-399).
  - TIMER `HOLD_1P_KEY2..7`, `HOLD_2P_KEY2..7`: 키 번호 기반 함수 `holdTimerId(player,key)=70+key+player*10` 로 해석(SkinPropertyMapper.java:18-23).
  - TIMER `PREVIEW`=141: 상수는 없으나 엔진이 사용한다. BMSPlayer 가 `config.isChartPreview()` 일 때 STATE_PRELOAD 에서 시작/선택 키를 누르면 141 번 타이머를 설정해 레인을 미리 보여준다(BJ/play/BMSPlayer.java:476-505, LaneRenderer.java:298-301). ModernChic 의 "Pattern Preview..." 표시(`CUSTOM.OP.isPreviewOFF` = timer 141 꺼짐 and `OP.NOW_LOADING`, customoption.lua:182-190)가 이 의미에 의존한다.
  - BUTTON `SKIN_CUSTOMIZE2..9`(221-229): 범위(BUTTON_SKIN_CUSTOMIZE1=220..10=229), `JUDGE_TIMING_AUTO_ADJUST`=75, `CONSTANT`=400(OPTION_CONSTANT=400 의 이벤트, EventFactory.java:794), SLIDER `IR_POSITION`=8 (= RATE_RANKING_POSITION=8).
  - OP `COURSE_STAGE5..9`=284-288: 정의만 있고 미사용.
  - IMAGE: `STAGEFILE`=-100, `BACKBMP`=-101, `BANNER`=-102, `SKINTHUMBNAIL`=-105, `BLACK`=-110, `WHITE`=-111. destination 의 id 로만 쓰이며 로더가 `Integer.parseInt(dst.id)` 후 음수면 `new SkinImage(-id)` 로 시스템 이미지를 만든다(BJ/skin/json/JSONSkinLoader.java:314-320). 100/101/102/110/111 만 정의되어 있고 `SKINTHUMBNAIL`(105)은 null 이미지(그리지 않음). Skinselect 의 썸네일 코드는 주석 처리됨(skinselect.lua:160).
- 사용된 id 의 전수 목록은 부록 A (이름=값(사용횟수)).

헬퍼 순서 의존: 어떤 이름표 파일도 부수효과가 없다(상수만).

---

## 7. custom*.lua 줄 단위 설명

공통: 모두 `main()` 단계에서 `Root.define2` 를 통해 로드된다. 상태/전역 의존: `MAIN`, `CUSTOM`, `CONFIG`, `main_state`, `skin_config`.

### 7.1 customtimer.lua (13줄), customtime.lua (10줄), author/version
- customtimer.lua:6-12: `CUSTOMTIMER_ID = 9999`, `GET_CUSTOMTIMER_ID()` 가 호출마다 +1 한 값(10000부터)을 반환. BJ 커스텀 타이머 구간 10000-19999(`TIMER_CUSTOM_BEGIN/END`, SkinProperty.java:175-176). 어디서도 호출되지 않는다.
- customtime.lua:6-10: 모듈 로드 시점에 `DATE=os.date("%y%m%d")`, `DATE2=os.date("%Y/%m/%d")`, `TIME=os.date("%H:%M:%S")` 를 한 번 계산(이후 갱신 없음). 스킨 구성 시각이므로 History 로그의 시각은 쓰기 시각이 아니다.
- version.lua:2 `4.6`, author.lua:2 `"KASAKO"`.

### 7.2 customfunction.lua (383줄) - 파일 I/O, 디렉터리, 정보 출력
전역 요구: `luajava`(File), `io`, `skin_config.get_path`, `main_state`, `MAIN`, `CONFIG`, `PROPERTY`, `CUSTOM`.

| 줄 | 함수 | 동작 |
|---|---|---|
| 7-8 | (최상위) | `local luajava = require("luajava"); local File = luajava.bindClass("java.io.File")` |
| 12-16 | createFile(path) | `io.open(path,"w")` 후 close. 부모 디렉터리는 BJ io 가 자동 생성 |
| 18-28 | existFile(path) | `io.open(path,"r")` 이 nil 이면 false, 아니면 close 후 true |
| 30-39 | countRecords(path,start) | 파일을 열어 `lines()` 줄 수 + start. 파일이 없으면 `f:lines` 에서 오류(호출 전에 existFile 로 보호) |
| 41-51 | countFileRecords(path,start) | 파일 없으면 start, 있으면 countRecords |
| 53-79 | rampConverter(n) | 0 NOPLAY,1 FAILED,2 LASSIST,3 ASSIST,4 EASY,5 NORMAL,6 HARD,7 EXHARD,8 FULLCOMBO,9 PERFECT,10 MAX, 그 외 nil |
| 81-97 | judgeLevel() | OP.JUDGE_VERYEASY/EASY/NORMAL/HARD/VERYHARD 중 참인 것 -> "判定レベル XXX _ " 문자열, 없으면 UNKNOWN |
| 102-110 | mkdir(path) | `luajava.new(File,path):mkdir()` (단일 레벨) |
| 112-134 | getSearchFiles(path,regexp) | `dir:listFiles()` 의 각 항목을 `tostring`, 역슬래시를 `/` 로 치환, `string.match(b, regexp)` 가 nil 이 아니면 결과를 `\n` 로 이어 붙이고 개수 증가. `(con, count)` 반환. 패턴은 절대 경로 문자열의 **첫 번째 매치 위치부터 끝까지**를 잘라내므로(예 "Play.+%.png"), 스킨이 설치된 경로에 "Play"/"Select"/"Result" 문자열이 먼저 나오면(예 `/Games/Play/ModernChic/...`) 잘못된 상대 경로가 만들어진다 |
| 136-141 | writeFile(path,con) | "w" 로 con 쓰기 |
| 143-148 | postscriptFile(path,con) | "a" 로 con 추가 |
| 150-160 | loadFile(path) | 줄 목록 테이블 반환(`lines()`) |
| 162-168 | storeFile(tbl) | 테이블 요소를 이어 붙인 문자열(구분자 없음) |
| 170-175 | resetFile(path) | "w" 로 열어 `f:write()`(인자 없음) 후 close |
| 177-193 | choiceList(all, exclude) | all 에서 exclude 에 없는 요소만 |
| 201-239 | randomChoice(partsFolder, logFolder, regexp) | 로테이션(1단계): `parts=get_path(partsFolder)`, `all=get_path(logFolder.."pathList.txt")`, `exclusion=get_path(logFolder.."excludeList.txt")`. 파일 없으면 생성. 폴더 파일 수(`fcount`)가 pathList 줄 수와 다르면 pathList 갱신 + exclude 초기화. exclude 줄 수 >= all 줄 수면 한 바퀴 돈 것으로 보고 exclude 초기화. 제외 후 `math.random(#list)` 로 선택하고 exclude 에 즉시 추가. 선택된 상대 경로 반환. 호출처: Select/lua/background.lua:14 ("Select/bg/image/", "io/Select/bg/image/", "Select.+%.png"), :21 (movie, ".mp4") |
| 248-295 | randomChoiceStep1(parts, log, regexp, init) | 2단계 로테이션 1단계: 위와 같지만 `temp.txt` 를 사용. 선택 결과를 `temp.txt` 에 추가만 하고 exclude 에는 넣지 않음. `init==true` 이면 시작 전에 temp 초기화. 호출처: Play/lua/base.lua:222 (SP, init=true), :265 (DP 왼쪽 init=true), :266 (DP 오른쪽 init=false) |
| 300-313 | randomChoiceStep2(logFolder) | `temp.txt` 의 모든 줄을 읽어 exclude 에 추가(2단계). 호출처: Play/lua/sp/cover.lua:168, dp/cover.lua:150 (곡의 마지막 노트 타이머 `ENDOFNOTE_1P` 가 켜진 첫 프레임에 `timer = function()` 안에서 호출. 매 프레임 평가되는 timer 함수 내 부수효과) |
| 319-367 | infoOutput(flg) | `History/information.txt` 를 **"w" 로 덮어써** 방송용 한 줄을 쓴다. flg 0=플레이(제목/아티스트/장르[+밀도/판정/시간/노트수], `CONFIG.play.playscreenMessage` 0/1/2), 5=선곡(`PROPERTY.isviewHistoryOff()` 이면 "Welcome to <VERSION> world!!! ModernChicSkin.<IR_NAME>..ACCESS...." , On 이면 날짜 + 오늘의 플레이 수/ランプ更新数/スコア更新数/ミスカン更新数 + 버전), 7=결과(ランプ 이름), 15=코스. `CONFIG.infoOutput` 이 true(기본)면 select(musicselect.lua:59), result(41), course(41), play(Play/lua/background.lua:29)의 구성 시점마다 실행 |
| 372-382 | selectBpmLinkChar() | `CONFIG.bpmLinkChar.charctor`: 0 -> "*"(와일드카드), 1 zundamon, 2 zundamon2, 3 zundamon3, 4 tsumugi, 5 yuki |

버그/주의: 112-134 `#filelists` 는 `listFiles()` 가 nil(디렉터리 없음)이면 "attempt to get length of nil" 오류 -> 호출한 `main()` 비보호 구간이면 스킨 로드 실패. 235 `io.open(...):write(choicePath.."\n"):close()` 는 `choicePath` 가 nil(폴더가 비어 있음)이면 문자열 연결 오류.

### 7.3 customsound.lua (272줄) - 사운드 재생 API 사용
- 11-19 `flg`(상태 테이블: isAchievementA/AA/AAA/Mybest/Harf/Target, isFullcombo 전부 true). VM 수명 동안 유지.
- 20-25 selectCharctorVoice: `CONFIG.voice.charctor` 1 -> "zundamon", 2 -> "tsumugi", 그 외 nil.
- 26-32 selectSectionEffect(diff): diff >= 0 이면 "Root/sounds/section-win.ogg" 아니면 "section-lose.ogg", 볼륨 2.
- 36-50: calculatorChangeSound/menuChangeSound -> change.ogg, clickSound -> click.ogg, enterSound -> enter.ogg (볼륨 인자 없음 = 1.0).
- 52-58 fcSound: `main_state.timer(MAIN.TIMER.FULLCOMBO_1P) ~= main_state.timer_off_value` 이고 `flg.isFullcombo` 이면 fullcombo.ogg(볼륨 2) 재생 후 플래그 반전. 항상 `return false`(그리기 훅). Play/lua/background.lua:6-8 이 `draw = function() return CUSTOM.SOUND.fcSound() end` 로 연결(`CONFIG.play.fcEffect` 가 true 일 때).
- 60-66 helpMotionSound(flg): false -> close.ogg, 아니면 open.ogg.
- 68-74 windowMotionSound(f1..f4): 모듈 지역 테이블 `flg`(항상 참)와 f1..f4 가 전부 false 이면 close.ogg, 아니면 open.ogg.
- 76-134 resultVoice(): 코스면(`CUSTOM.OP.isCourse()`) 게이지 80 초과 great / 40 초과 good / 0 초과 bad / 그 외 failed; `CONFIG.voice.result.type` 1 -> 랭크 음성(F..AAA, `OP.RESULT_*_1P`), 2 -> `main_state.number(NUM.CLEAR)` 1..10 -> failed/assistop/assisteasy/easy/clear/hard/exhard/fullcombo/perfect/max, 3 -> `CUSTOM.OP.isNotFirstPlay()` 일 때 update/hiscore.ogg. 호출처 result.lua:173-175 (`CONFIG.voice.result.sw` 가 true 일 때, 구성 시점에 1회). 볼륨 2.
- 137-164 achievementVoice(): 매 프레임 훅. `CONFIG.voice.play.achievement` 일 때 `OP.A/AA/AAA` 최초 도달 시 rank/A,AA,AAA.ogg; `CUSTOM.OP.isNotFirstPlay2()` and 타이머 `SCORE_BEST` 켜짐 and mybest -> update/hiscore.ogg; 타이머 `SCORE_TARGET` 켜짐 and target -> update/target.ogg; `isSectionfRemain(1,2)` (곡 길이의 1/2 경과) and harfRemain -> harf.ogg. 항상 `return false`. 연결: Play/lua/background.lua:15-18 (`CONFIG.voice.play.sw and OP.AUTOPLAYOFF` 일 때 `draw` 훅).
- 167-201 sectionScoreEffect(): 4구간 각각 경과 시 `CONFIG.play.sectionScore.sound.type` 1 -> section.ogg, 2 -> selectSectionEffect(tgtDiff). `return false`. 연결: Play/lua/{sp,dp}/info.lua:434-435,457-458.
- 203-227 init*: `main_state.audio_play(path, 0.0001)` 로 파일을 미리 로드(디코드 캐시)하는 용도. 사용되는 것: initAchievementVoice, initSectionSE, initFcSE. 정의만 있고 미사용: initResultSE, initResultVoice, initSelectSE.
- 모든 경로는 `skin_config.get_path("Root/sounds/...")` (절대 경로). 파일 목록: Root/sounds/{change,click,close,enter,favorite,fullcombo,open,section-lose,section-win,section}.ogg + vo/{zundamon,tsumugi}/{clear/*,course/*,rank/*,update/*,harf}.ogg. `favorite.ogg` 는 코드에서 참조되지 않는다.

### 7.4 customoption.lua (240줄) - 불리언 헬퍼 (draw/op/조건에 쓰임)
isGaugeAssistEasy..isGaugeExhardGrade (gauge_type 0..8), isTimerOff/isTimerOn(id), isLnPattern/isCnPattern/isHcnPattern (`event_index(BUTTON.LNMODE)` 0/1/2 and `OP.LN`), isInTheMiddleFailed (RESULT_FAIL and 총 노트 != 판정 합), isMainBpm, isCourse (`text(COURSE1_TITLE) ~= ""`), isNotFirstPlay (OP.UPDATE_SCORE and NUM.HIGHSCORE2 != 0), isNotFirstPlay2/isFirstPlay2 (`float_number(GRAPH.BESTSCORERATE)` != 0 / == 0, and AUTOPLAYOFF), isNeglect(sec) (`(time() - timer(SONGBAR_CHANGE)) / 1000000 > sec`), `isCharDisplayOn`(변경 가능한 불리언 값; 함수 아님), isMyFrame(idx) (`number(389+idx)` == `number(IR_RANK)` and `text(119+idx) == "YOU"`), isYouWin/isRivalWin (`OP.COMPARE_RIVAL` 와 점수 비교), isAssistOn (ASSIST_* 7개 버튼의 event_index == 1), isCounseWithin5/isCounseOver6, isRemainSec(sec), isSectionfRemain(n,d), isBrinkGauge, isPreviewOFF, laneCoverRest(per), constantRest(per) (HSFIX 2/3/4 에서 레인커버/리프트/BPM 비로 계산; 이벤트가 2/3/4 가 아니면 nil 반환). laneCoverRest/constantRest 는 72회씩 호출되는 판정문자 투명도 draw 조건.
수식(레인커버): `lanecover_area = NUM.LANECOVER1/1000`, `liftcover_area = OP.LIFT1_ON ? NUM.LIFT1/1000 : 0`, `lanecover_fix = (1 - lift) * lanecover`, `laneCoverRest(per) = per > lanecover_fix + lift`. `constantRest` 는 `space = 1 - lanecover_fix - lift`, 이벤트 2: `per > fix + lift + (space - space * NOWBPM / MAXBPM)` (3: MAINBPM, 4: MINBPM).

### 7.5 customnumber.lua (303줄) - 정수 value 함수 + 로드 시점 파일 읽기
- 6-17 지역 상태와 `selectTarget()` (`CONFIG.play.sectionScore.type` 1 또는 첫 플레이면 `NUM.DIFF_TARGETSCORE`, 2 이면 `DIFF_HIGHSCORE`).
- 24-87: masterVolumeNum/keyVolumeNum/bgmVolumeNum (`volume_*()*100`), nowScoreRateNum(`rate()`), nowExScoreNum(`exscore()`), nowGaugePercentNum(`gauge()`), nowPGCountNum..nowMissCountNum (`judge(0..5)`).
- 72-100: randNum(min,max)=`math.random`, oneBeat(split)=`(60/NUM.MAINBPM)*1000*split`, oneBeat2(split,bpm), maxExscore=`TOTALNOTES*2`, myScoreBest (HIGHSCORE2 와 SCORE3 중 큰 값).
- 102-120 irRankDiff, 122-155 mybestClearType (clear 값을 1..11 로 재매핑 후 작은 값), 158-171 calcTotal (참고 TOTAL: 노트 <400: 200+n/5, <600: 280+(n-400)/2.5, >=600: 360+(n-600)/5), 173-215 bestRank/nowRank(0..8), 217-231 diffRGB (난이도별 RGB 테이블: 1 {6,255,0}, 2 {18,210,215}, 3 {255,192,0}, 4 {255,0,0}, 5 {148,44,150}, 0 {195,195,195}; 해당 없음이면 nil), 233-241 achievementRate, 244-278 sectionTime/sectionScore1_4..4_4 (래치 플래그 `nFlg` 로 구간 종료 순간 `SCORE2`/선택 대상 차이를 캡처하고 `{myScore, tgtDiff}` 반환), 281-287 elapsedTimeFromStart (`(time() - timer(PLAY)) / 1000000`), elapsedTimeFromStart2.
- **289-303 모듈 로드 시점에 즉시 실행되는 파일 읽기**: `allLaneCoverCountSP/DP` = `FUNC.countFileRecords(get_path("io/Play/{sp,dp}/lanecover/pathList.txt"), 0)`, `usedLaneCoverCountSP/DP` = `.../excludeList.txt`, `todaySongUpdateCount/todayClearUpdateCount/todayScoreUpdateCount/todayMissUpdateCount` = `History/<DATE>/{history,clear,score,miss}.txt` 줄 수, `impressionCount` = `History/impression.txt` 줄 수(시작값 1). 파일이 없으면 시작값(0 또는 1).
- 주의: 이 값들은 VM 수명 동안 고정(정수 상수)이며 value 함수가 반환하는 것은 로드 시점 값이다.

### 7.6 customgraph.lua / customslider.lua / customtext.lua
- customgraph.lua:10-22 SlowRate/FastRate = `TOTALLATE/(TOTALLATE+TOTALEARLY)` 등(둘 다 0 이면 0/0 = NaN float). 24-64 sectionRemainRate: 곡 길이 1/4 구간 남은 비율(`gFlg` 래치 4개), 58행 `CUSTOM.sectionScore.oneFour.gFlg` 를 반전시키는 복사 오류가 있어 마지막 구간 이후 값은 0 으로 고정(무해).
- customslider.lua:7-47 adjustedCover(): HSFIX 이벤트 2/3/4 에서 `{조정값, 최소, 최대, 메인BPM}` 4개의 레인커버 위치(0~1) 배열 반환, 그 외는 nil(호출부 `...()[1]` 가 nil 인덱싱 오류 -> 슬라이더 value 오류로 처리).
- customtext.lua:8-34 choiceTips(): 22개 팁 중 `math.random(1,#msg)` 하나를 "Tips:" 접두로 반환. Decide 텍스트 `tips` 의 `constantText` 로 구성 시점에 1회 고정(Decide/lua/require/textproperty.lua:53,71).

---

## 8. config.lua (222줄) - 사용자 설정 테이블

`require("config")` 는 `<스킨폴더>/config.lua` 를 로드한다. 값은 정적 테이블이며 스킨이 읽기만 한다(사용자가 파일을 직접 편집). 필드 전수:

| 키 | 기본값 | 읽는 곳(횟수) |
|---|---|---|
| play.valiableSUD.sw / alfa | true / 100 | Play/lua/sp/cover.lua:28,31 등 (각 2) |
| play.valiableJUDGE.sw / rest / alfa | false / 0.6 / 50 | 판정 문자 투명도 조건 (sw 4, rest 144, alfa 72) |
| play.sectionScore.sw / type / sound.sw / sound.type | false / 1 / true / 2 | sw 2, type 2, sound.sw 2, sound.type 8 |
| play.notesAnimation | true | sp/notes.lua:157 (2) |
| play.judgeAnimation | true | sp/judge.lua:19 (2) |
| play.hcnBomb | false | sp/bomb.lua:164 (2) |
| play.liftDisplay | true | sp/cover.lua:5 (2) |
| play.smallGauge | false | sp/gauge.lua:108, Result/lua/mainmenu.lua:106 (3) |
| play.playscreenMessage | 0 | Root/customfunction.lua:336-340 (3) |
| play.detailInfoAlfa | 180 | sp/info.lua:322 (1) |
| play.fcEffect | false | Play/lua/background.lua:25 (1) |
| voice.charctor | 1 | customsound.lua:22-23 (2) |
| voice.result.sw / type | false / 3 | result.lua:173, customsound.lua:89,107,129 (sw 1, type 6) |
| voice.play.sw / achievement / mybest / target / harfRemain | false / true / true / false / false | customsound.lua (각 1) |
| bpmLinkChar.charctor / sw / playside / type | 0 / false / true / 1 | customfunction.lua:372-382, Play/lua/base.lua:136,147 (charctor 6, sw 2, playside 2, type 4) |
| impression.sw / type / songLimit / bgaLimit / etcLimit / msg | false / 0 / 500 / 400 / 100 / "" | Result/lua/impression.lua (sw 1, type 2, 각 limit 4, msg 4) |
| result.irCover | false | 1 |
| infoOutput | true | musicselect.lua:59, result.lua:41, course.lua:41, Play/lua/background.lua:29 (4) |

주의: 필드 이름의 오타(`valiable`, `charctor`, `harfRemain`)를 그대로 유지해야 한다. 값이 바뀌어도 `main()` 구성 시점에 읽는다(스킨 재구성 전에는 반영 안 됨).

---

## 9. 파일 읽기/쓰기 지도 (언제, 어디에, 무엇을)

제약(BJ): 모든 `io.open` 경로는 `<스킨폴더>` 하위여야 하고, 밖이면 `IOException`("Lua skin file access denied")으로 `io.open` 이 nil 을 돌려준다(SkinLuaAccessor.java:376-392). 쓰기는 상위 디렉터리를 만든다(349-353).

| 시점 | 파일(스킨 폴더 기준) | 동작 | 주체 |
|---|---|---|---|
| 선곡/결과/코스/플레이 구성 시 | `History/information.txt` | 덮어쓰기(w). 방송용 한 줄 | customfunction.infoOutput (기본 켜짐) |
| 구성 시 | `io/Play/{sp,dp}/lanecover/{pathList,excludeList}.txt`, `io/Play/sp/nentyaku.txt`, `History/<yymmdd>/{history,clear,score,miss}.txt`, `History/impression.txt` | 읽기(줄 수 세기) | customnumber 최상위 (없으면 0/1) |
| 구성 시, 선곡 배경 로테이션 켜졌을 때 | `io/Select/bg/{image,movie}/{pathList,excludeList}.txt` | 생성/읽기/쓰기 | randomChoice (Select/lua/background.lua:14,21; 조건은 `isbgRotationOn`) |
| 구성 시, 플레이 레인커버 로테이션 켜졌을 때 | `io/Play/{sp,dp}/lanecover/{pathList,excludeList,temp}.txt` | 생성/읽기/쓰기 | randomChoiceStep1 (Play/lua/base.lua:222,265,266), 곡 종료 시 Step2 로 temp -> exclude 추가 |
| 구성 시, 결과 배경/캐릭터 로테이션 켜졌을 때 | `io/Result/{bg,char}/{isclear,all,rank,clearType}/<name>_{pathList,excludeList}.txt` | 생성/읽기/쓰기 | Result/lua/require/listchoice.lua |
| 선곡 구성 시 사이드메뉴 상태 유지 옵션 켜짐 | `Select/lua/settings/sidemenu` | 읽기. 없으면 "false\nfalse\nfalse\nfalse" 로 생성. 버튼 클릭 시마다 4줄 덮어쓰기 | Select/lua/require/settings.lua:6-69 |
| 결과 구성 시 "플레이 履歴の保存" 켜짐(기본 꺼짐) | `History/<yymmdd>/history.txt`(추가), `clear.txt`, `score.txt`, `miss.txt`(조건 충족 시 추가), `History/recent.txt`(당일 history 를 역순으로 덮어쓰기), `History/search.html`(버튼 클릭 시 덮어쓰기: YouTube/niconico 검색 링크) | 쓰기 | Result/lua/history.lua:6-159. 구성 1회당 한 번 기록되므로 같은 결과 화면을 재구성하면 중복 기록 |
| 결과 화면 버튼 클릭 | `difflist.txt`(스킨 루트) | 없으면 `[{"title":...}]` 생성, 있으면 끝의 `]` 제거 후 `,{...}]` 추가. 파일에 제목/아티스트/레벨/md5/sha256 | Result/lua/diflist.lua:6-24. 주석: 확장자가 json 이면 스킨으로 오인되어 강제 종료되므로 txt 사용 |
| 결과 인프레 입력 확정 클릭 | `History/impression.txt` | 추가 | Result/lua/impression.lua:70-84, 314-... |
| 플레이 "상세 모드"(기본 꺼짐) 구성/곡 종료 | `io/Play/sp/log/<날짜>.txt`, `io/Play/sp/lnlog/<날짜>.txt`, `total.txt`(각 8줄 정수), `io/Play/sp/nentyaku.txt`(제목+`NN\t회수\t도달노트\t...` 줄) | 읽기/쓰기 | bgaareainfo.lua:42-70,150, nentyakuinfo.lua:32-90 |
| 선곡 버전 체크(주석 처리됨) | `Select/lua/settings/checkversion` (1행 다음 확인 시각, 2행 true/false) | 읽기/쓰기 | http.lua(도달 불가) |

배포본에 포함된 런타임 파일(참고): `io/Play/{sp,dp}/lanecover/{pathList,excludeList,temp}.txt`(pathList 는 `Play/parts/common/lanecover/#default.png` 한 줄), `io/Play/sp/{log,lnlog}/{2023-05-25,2024-06-23,total}.txt`, `io/Play/sp/nentyaku.txt`(곡 "G59 [ANOTHER]"), `Select/lua/settings/{sidemenu,checkversion}`, `History/{information,recent}.txt`, 빈 디렉터리 `io/Result/{bg,char}/{all,clearType,isclear,rank}`, `io/Select/bg/{image,movie}`, `Select/parts/dummy/-`, `Play/parts/dummy/-`, `Result/parts/dummy/-`(58바이트 더미).

R-BMS 구현 시사점: 스킨 폴더가 읽기 전용(앱 번들/리소스)이면 ModernChic 이 쓰는 위 경로를 "스킨 폴더 하위로 보이는 쓰기 가능한 오버레이"(사용자 데이터 디렉터리)로 매핑하고, `dir:listFiles()`/`io.open`/`get_path` 가 같은 가상 루트를 보게 해야 한다. 기본 스킨(단순화판)은 이 쓰기 기능 대부분(History, 로테이션, 메시지 출력, 인프레, 난이도표, 상세 로그)을 아예 제거하는 것이 현실적이다.

네트워크: 사용처는 `Select/lua/require/http.lua` 하나뿐이다. 대상 URL `https://access.kasacontent.com/mcskin/select`(http.lua:139), 용도는 스킨 신규 버전 확인(응답 첫 줄 숫자와 `Root.version` 비교, 7일마다, 연결 타임아웃 200ms). 호출 경로는 `Select/lua/versioncheck.lua` -> musicselect.lua:215-227 의 블록 주석에 의해 비활성이다(코드 도달 불가). BJ 파사드는 HTTP GET 만 허용하고 타임아웃 1~5000ms, 최대 1024줄/65536자로 제한한다(LegacySkinLuaApi.java:46-49,219-293). 참고: 배포본의 `Select/lua/settings/checkversion` 내용 `1.7200937E9` 는 과거에 이 기능이 실행됐다는 흔적이며, LuaJ 의 `os.time()` 이 소수 초를 반환하고 숫자->문자열 변환이 Java float 표기를 따른다는 단서다(추정).

사운드 재생: `main_state.audio_play(path, volume)` 82회(전부 customsound.lua). 파일은 `skin_config.get_path("Root/sounds/..." )` 로 만든 절대 경로. BJ 구현(SkinAudioLuaApiExporter.java:15-69): `play(path, volume, loop=false)`; `vol = clamp(volume nil ? 1 : volume, 0, 2)`; 실제 볼륨 = `AudioConfig.systemvolume * vol`; 경로는 스킨 폴더 하위만 허용(밖이면 LuaError). 반환 true. 사용된 볼륨 값: 생략(1.0), 2, 0.0001(프리로드). ogg 73개 중 Root/sounds 가 사용하는 것은 10개 효과음 + 캐릭터 음성 2종. 스킨 루트의 `Sound/*.ogg` 14개(clear, fail, playready, scratch 등)와 `Select/sounds/{open,close}.ogg` 는 Lua 에서 참조되지 않는다(beatoraja 사운드셋/기타 용도로 추정, 미확인).

---

## 10. 화면 구성 시 엔진 상태 의존 (구성 시점에 읽는 값)

| 시점 | 읽는 값 | 위치 |
|---|---|---|
| Play 구성 | `OP.NO_BGA`(170), `OP.AUTOPLAYOFF`(32) | Play/lua/base.lua:230-233, Play/lua/background.lua:15 |
| Decide 구성 | `OP.DIFFICULTY0..5`(난이도 색), `NUM.todaySongUpdateCount`(STAGEn+1) | Decide/lua/require/textproperty.lua:7-35,50 |
| Result 구성 | 제목/아티스트/장르/난이도표, `NUM.TIME_YEAR/MONTH/DAY`, `STRING.PLAYER`, `OP.ONLINE`, `OP.RESULT_CLEAR/FAIL`(배경 색), 코스 여부(`text(COURSE1_TITLE)`) | Result/lua/require/textproperty.lua:20-25, result.lua:111 |
| Select 구성 | `STRING.VERSION`, `STRING.IR_NAME`(infoOutput) | Root/customfunction.lua:346 |

즉 R-BMS 는 스킨 구성 호출 직전에 곡/결과/IR 상태가 `main_state.option/number/text` 로 조회 가능해야 한다.

---

## 11. R-BMS 현재 상태와의 격차 (근거: RB/crates/rbms-skin)

- RB/crates/rbms-skin/Cargo.toml:19 `mlua 0.12.1 { lua54, vendored }`, 선택 기능 `lua`.
- RB/crates/rbms-skin/src/lua.rs:110 `Lua::new_with(MATH | STRING | TABLE)`; 55-74 `FORBIDDEN_GLOBALS` = dofile, loadfile, load, loadstring, require, collectgarbage, rawset, rawget, rawequal, rawlen, setmetatable, getmetatable, newproxy, print, io, os, package, debug; 83 `FORBIDDEN_STRING_FUNCTIONS` = dump, find, gmatch, gsub, match; 37,121 고정 시드 `RANDOM_SEED = 0x5eed_5eed` 로 `math.randomseed`; 193-215 호출마다 `skin` 테이블(boolean/number/float/text/timer/time 6개)을 만들고, 식 하나(`EXPRESSION_CHUNK_NAME`)만 평가하며 명령어 예산 훅으로 제한.
- ModernChic 이 필요로 하지만 현재 금지된 것: `require`, `dofile`, `io`, `os`, `package`, `print`, `string.match/gsub/gmatch`, `pcall`(base 는 로드되지만 정책 확인 필요), 멀티 라인 파일 실행, 전역 대입, 클로저 보관(함수형 속성 최초 구성 시 생성해 매 프레임 호출).
- 필요한 새 구성요소: (a) 스킨별 새 VM 에서 `.luaskin` 을 두 단계로 실행하는 로더, (b) 스킨 루트 제한 `io`/`os` 구현과 쓰기 오버레이, (c) `main_state`/`timer_util`/`skin_config`/`luajava` 파사드, (d) 함수형 속성을 데이터 모델의 "Lua 콜백 핸들"로 보관하는 스키마 확장, (e) Lua 테이블 -> 스킨 스키마 변환기(문자열화/절단/truthiness 규칙, 3장과 4장 A), (f) 오류 시 파트 단위 생략 + 로그.
- mlua 는 `lua52` 기능도 제공하므로, BJ(LuaJ=5.2 의미)와 숫자 문자열화/정수 나눗셈 차이를 줄이려면 후보가 되나 ModernChic 코드는 5.1 호환이라 lua54 로도 읽힌다. `..` 로 `/` 결과 float 를 문자열 연결하는 곳은 전수 grep 으로 발견되지 않았다(미확인 항목 있음: 읽지 않은 파트 파일).

---

## 12. 위험 / 모호 / 결함 목록

1. 첫 매치 패턴 문제: `string.match(abs, "Play.+%.png")`, "Select.+%.png", "Select.+%.mp4", "Result.+%.png" 가 절대 경로의 첫 등장 지점부터 자르므로 스킨이 설치된 상위 경로에 이 단어가 있으면 로테이션 결과가 틀린다(customfunction.lua:125, listchoice.lua:14,91). R-BMS 의 `listFiles` 가 돌려주는 경로 형태와 설치 경로가 영향을 준다.
2. `listFiles` 반환 순서는 비결정(DirectoryStream). pathList.txt 줄 순서는 의미 없음(개수만 비교).
3. 파일 선택 기본값: beatoraja 로더는 `def` 를 적용하지 않고 저장 안 된 항목을 무작위로 고른다(SkinLoader.java:107-127). 번들 폴더 안에 파일이 여러 개인 항목(폭발 이펙트 6400x1200 이미지 7개 등)은 R-BMS 에서 `def` 를 적용해 결정적으로 고르는 정책이 필요.
4. 이름 기반 선택 키 충돌(3.1 마지막): Result 의 "背景（Failed）", "キャラクター（Failed）" 두 개씩, SP/DP 의 "ゲージ" 옵션/파일 이름 동일.
5. `Select/lua/require/http.lua` 는 도달 불가이지만 파일에 남아 있으므로 로더가 모든 Lua 를 정적으로 검사하지는 않아야 한다(미사용 파일에서 오류가 나도 무관).
6. KeyConfig 스킨은 로드 불가 + 비어 있음(2.7). Decide 용 textproperty 가 KeyConfig 의 `PROPERTY` 로 호출됨.
7. `Select/lua/volumecontrol.lua` 는 어디서도 로드되지 않는다(정적 도달성 분석, m1_lua_reachability). `versioncheck.lua` 와 `http.lua` 는 musicselect.lua 의 블록 주석 때문에 도달 불가.
8. 시간대: `os.date` 는 현지 시간. R-BMS 컨벤션(UTC)과 달라도 ModernChic 호환을 위해 현지 시간이어야 History 폴더명이 beatoraja 와 같다. 기본 스킨에서 History 를 제거하면 무관.
9. 대용량 텍스처(m1_png_sizes): PNG 284개 중 한 변이 4096 을 넘는 것은 8개(Play/parts/common/bomb/*.png 7개 6400x1200, Play/parts/common/fullcombo/#default.png 5190x2571). 4096 이하이지만 큰 것: Result/parts/prepare.png 3920x3800, Select/parts/{jp,en,cn}/sidemenu.png 3700x1800, Play/parts/common/attack/hud.png 3600x772, Select/parts/{jp,en,cn}/mainframe.png 3200x3200. 최대 변 6400, 8192 초과 없음. GPU 최대 텍스처 크기 한도 점검 필요.
10. 라이선스: `Select/readme.txt` 는 "개조 스킨 공개 시 원작자명(KASAKO) 표기 필요", "스킨 자체의 2차 배포는 허가 없이 금지" 라고 적는다. 기본 스킨으로 번들할 경우 허가/표기 정책 결정이 필요하다. 포함 자산의 별도 출처: 폰트 Mgen+ (SIL OFL 1.1, 각 화면 폴더에 라이선스 파일), 캐릭터 일러스트(ずんだもん, 春日部つむぎ, 四国めたん 및 개인 제공 일러스트), 음성, BGM(DOVA-SYNDROME 31줄 목록 Sound/使用させていただいた楽曲.txt), mp4 8개. 단순화 기본 스킨에서는 캐릭터/음성/동영상을 제외하는 것이 안전하다.
11. 함수형 `act` 호출 규약(추정): LuaJ 에서 함수 값의 `narg()` 는 1 이므로 `loadEvent(LuaFunction)` 의 switch 가 항상 1인자 이벤트로 귀결될 가능성이 높다(SkinLuaAccessor.java:759-787). ModernChic 의 `act = function() ... end` 35개는 인자를 받지 않으므로 어느 쪽이든 동일하게 동작한다.
12. `act = MAIN.NUM.CLEAR`(370), `act = MAIN.NUM.TARGET_CLEAR`(371): NUM id 를 이벤트 id 로 쓴 것(Result/lua/mainmenu.lua:128,130, prepare.lua:13). BJ 에서 370/371 은 `BUTTON_PRACTICE_ITEM1` 등 다른 이벤트와 겹친다(SkinProperty.java:1043). 클릭 시 의도치 않은 이벤트가 실행될 수 있으나 결과 화면에서는 무해한 것으로 추정(미확인).
13. `main_state.event_index(MAIN.NUM.CLEAR)` (listchoice.lua:72,150): 버튼 인덱스 API 에 NUM id 를 넘기는 오용. 로테이션이 clearType 패턴일 때만 영향(기본값 아님).
14. 사용자 설정 제목에 대한 비결정: `math.random` 비시드 -> 팁/로테이션/Decide 락온 애니메이션 색상(`CUSTOM.NUM.randNum(0,255)` x3) 이 로드마다 달라진다. 완전 재현 테스트 시 시드 고정이 필요(단 프로덕션은 비시드).
15. 구성 시점 부수효과: 결과 구성 시 `history.lua` 가 즉시 파일을 쓴다(옵션 켜진 경우). R-BMS 는 스킨 구성을 화면 진입당 정확히 1회만 수행해야 중복 기록이 없다.

---

## 13. 부록 A - ModernChic 이 실제 참조하는 id (이름=값(사용횟수)), 값 오름차순

### OP (139)
FOLDERBAR=1(x6), SONGBAR=2(x67), GRADEBAR=3(x35), PLAYABLEBAR=5(x2), PANEL1=21(x2), PANEL2=22(x1), PANEL3=23(x1), AUTOPLAYOFF=32(x27), AUTOPLAYON=33(x14), GAUGE_HARD=43(x12), OFFLINE=50(x3), ONLINE=51(x30), NOW_LOADING=80(x42), LOADED=81(x28), REPLAY_PLAYING=84(x3), RESULT_CLEAR=90(x7), RESULT_FAIL=91(x6), SELECT_BAR_NOT_PLAYED=100(x8), SELECT_BAR_FAILED=101(x4), SELECT_BAR_EASY_CLEARED=102(x4), SELECT_BAR_NORMAL_CLEARED=103(x4), SELECT_BAR_HARD_CLEARED=104(x4), SELECT_BAR_FULL_COMBO_CLEARED=105(x4), DIFFICULTY0=150(x7), DIFFICULTY1=151(x7), DIFFICULTY2=152(x7), DIFFICULTY3=153(x7), DIFFICULTY4=154(x7), DIFFICULTY5=155(x7), SONG7KEY=160(x2), SONG5KEY=161(x2), SONG14KEY=162(x4), SONG10KEY=163(x3), SONG9KEY=164(x2), NO_BGA=170(x16), BGA=171(x2), NO_LN=172(x2), LN=173(x5), TEXT=175(x2), NO_BPMCHANGE=176(x5), BPMCHANGE=177(x17), JUDGE_VERYHARD=180(x4), JUDGE_HARD=181(x4), JUDGE_NORMAL=182(x4), JUDGE_EASY=183(x4), JUDGE_VERYEASY=184(x4), NO_STAGEFILE=190(x1), STAGEFILE=191(x4), NO_BACKBMP=194(x4), BACKBMP=195(x2), NO_REPLAYDATA=196(x1), REPLAYDATA=197(x5), REPLAYDATA_SAVED=198(x1), AAA_1P=200(x3), AA_1P=201(x3), A_1P=202(x3), B_1P=203(x3), C_1P=204(x3), D_1P=205(x3), E_1P=206(x3), F_1P=207(x3), AAA=220(x4), AA=221(x4), A=222(x4), GAUGE_1P_0_9=230(x5), GAUGE_1P_10_19=231(x5), GAUGE_1P_20_29=232(x5), GAUGE_1P_100=240(x2), PERFECT_1P=241(x1), LANECOVER1_CHANGING=270(x22), LANECOVER1_ON=271(x2), LIFT1_ON=272(x3), COURSE_STAGE_FINAL=289(x2), MODE_COURSE=290(x3), RESULT_AAA_1P=300 ... RESULT_F_1P=307 (각 x6), BEST_AAA_1P=320 ... BEST_F_1P=327 (각 x1), UPDATE_SCORE=330(x2), UPDATE_MAXCOMBO=331(x1), UPDATE_MISSCOUNT=332(x1), WIN_1P=352(x2), WIN_2P=353(x2), CONSTANT=400(x14), IR_LOADED=602(x1), IR_WAITING=606(x12), COMPARE_RIVAL=625(x21), GRADEBAR_MIRROR=1003, GRADEBAR_RANDOM=1004, GRADEBAR_NOSPEED=1005, GRADEBAR_GAUGE_LR2=1010, GRADEBAR_GAUGE_5KEYS=1011, GRADEBAR_GAUGE_7KEYS=1012, GRADEBAR_GAUGE_9KEYS=1013, GRADEBAR_GAUGE_24KEYS=1014, GRADEBAR_LN=1015, GRADEBAR_CN=1016, GRADEBAR_HCN=1017, RANDOMSELECTBAR=1030 (각 x1), STATE_PRACTICE=1080(x12), SELECT_BAR_ASSIST_EASY_CLEARED=1100, SELECT_BAR_LIGHT_ASSIST_EASY_CLEARED=1101, SELECT_BAR_EXHARD_CLEARED=1102, SELECT_BAR_PERFECT_CLEARED=1103, SELECT_BAR_MAX_CLEARED=1104 (각 x4), SONG24KEY=1160(x2), SONG24KEYDP=1161(x3), NO_REPLAYDATA2=1196, REPLAYDATA2=1197(x5), REPLAYDATA2_SAVED=1198, NO_REPLAYDATA3=1199, REPLAYDATA3=1200(x5), REPLAYDATA3_SAVED=1201, NO_REPLAYDATA4=1202, REPLAYDATA4=1203(x5), REPLAYDATA4_SAVED=1204, SELECT_REPLAYDATA=1205 ... SELECT_REPLAYDATA4=1208 (각 x2), EARLY_1P=1242(x7), LATE_1P=1243(x7), EARLY_2P=1262(x3), LATE_2P=1263(x3), GOOD_EXIST=2243(x16), BAD_EXIST=2244(x20), POOR_EXIST=2245(x18)

### NUM (187)
JUDGETIMING=12, LANECOVER1=14, TIME_YEAR=21, TIME_MONTH=22, TIME_DAY=23, TIME_HOUR=24, TIME_MINUTE=25, TIME_SECOND=26, SCORE=71, TOTALNOTES=74, MAXCOMBO=75, MISSCOUNT=76, MAXBPM=90, MINBPM=91, MAINBPM=92, PLAYLEVEL=96, POINT=100, SCORE2=101, SCORE_RATE=102, SCORE_RATE_AFTERDOT=103, MAXCOMBO2=105, GROOVEGAUGE=107, PERFECT=110, GREAT=111, GOOD=112, BAD=113, POOR=114, TARGET_SCORE=121, DIFF_HIGHSCORE=152, DIFF_TARGETSCORE=153, NOWBPM=160, PLAYTIME_MINUTE=161, PLAYTIME_SECOND=162, TIMELEFT_MINUTE=163, TIMELEFT_SECOND=164, LOADING_PROGRESS=165, HIGHSCORE2=170, SCORE3=171, DIFF_HIGHSCORE2=172, DIFF_MAXCOMBO=175, TARGET_MISSCOUNT=176, MISSCOUNT2=177, DIFF_MISSCOUNT=178, IR_RANK=179, IR_TOTALPLAYER=180, IR_PREVRANK=182, IR_PLAYER_{NOPLAY,ASSIST,LIGHTASSIST,EXHARD,FAILED,EASY,NORMAL,HARD,FULLCOMBO}=202,204,206,208,210,212,214,216,218 및 각 _RATE=203,205,...,219, IR_PLAYER_PERFECT=222 / _RATE=223, IR_PLAYER_MAX=224 / _RATE=225, IR_PLAYER_TOTAL_CLEAR_RATE=227, IR_PLAYER_TOTAL_FULLCOMBO_RATE=229, 각 _RATE_AFTERDOT=230-242, RIVAL_SCORE=271, RIVAL_PERFECT..RIVAL_POOR=280-284, FOLDER_TOTALSONGS=300, DURATION=312, DURATION_GREEN=313, LIFT1=314, FOLDER_NOPLAY=320, FOLDER_FAILED=321, FOLDER_ASSIST=322, FOLDER_LASSIST=323, FOLDER_EASY=324, FOLDER_GROOOVE=325, FOLDER_HARD=326, FOLDER_EXHARD=327, FOLDER_FULLCOMBO=328, FOLDER_PERFECT=329, FOLDER_MAX=330, TOTALNOTE_NORMAL=350, TOTALNOTE_LN=351, TOTALNOTE_SCRATCH=352, TOTALNOTE_BSS=353, DENSITY_PEAK=360, DENSITY_PEAK_AFTERDOT=361, DENSITY_END=362, DENSITY_AVERAGE=364, DENSITY_AVERAGE_AFTERDOT=365, SONGGAUGE_TOTAL=368, CLEAR=370, TARGET_CLEAR=371, AVERAGE_TIMING=374, AVERAGE_TIMING_AFTERDOT=375, STDDEV_TIMING=376, STDDEV_TIMING_AFTERDOT=377, RANKING1..10_EXSCORE=380-389, RANKING1..10_CLEAR/INDEX=390-399, JUDGERANK=400, GROOVEGAUGE_AFTERDOT=407, EARLY/LATE_PERFECT=410/411, EARLY/LATE_GREAT=412/413, EARLY/LATE_GOOD=414/415, EARLY/LATE_BAD=416/417, EARLY/LATE_POOR=418/419, MISS=420, EARLY_MISS=421, LATE_MISS=422, TOTALEARLY=423, TOTALLATE=424, COMBOBREAK=425, BAD_PLUS_POOR_PLUS_MISS=427, JUDGE_1P_DURATION=525, JUDGE_2P_DURATION=526, SONGLENGTH_MINUTE=1163, SONGLENGTH_SECOND=1164, DURATION_LANECOVER_ON/GREEN_ON/OFF/GREEN_OFF=1312-1315, MAINBPM_DURATION_*=1316-1319, MINBPM_DURATION_LANECOVER_ON/GREEN_ON/OFF=1320-1322, MAXBPM_DURATION_LANECOVER_ON/GREEN_ON/OFF=1324-1326

### TIMER (88)
FADEOUT=2(x8), FAILED=3(x13), SONGBAR_CHANGE=11(x28), PANEL1_ON/PANEL2_ON/PANEL3_ON=21/22/23(x30/28/30), PANEL1_OFF/2/3=31/32/33(x2), READY=40(x11), PLAY=41(x50), GAUGE_INCLEASE_1P=42(x2), GAUGE_MAX_1P=44(x4), JUDGE_1P=46(x129), JUDGE_2P=47(x63), FULLCOMBO_1P=48(x9), BOMB_1P_SCRATCH..KEY7=50-57, BOMB_2P_SCRATCH..KEY7=60-67, HOLD_1P_SCRATCH..KEY7=70-77, HOLD_2P_SCRATCH..KEY7=80-87, KEYON_1P_SCRATCH..KEY7=100-107, KEYON_2P_SCRATCH..KEY7=110-117, KEYOFF_1P_SCRATCH..KEY7=120-127, KEYOFF_2P_SCRATCH..KEY7=130-137, RHYTHM=140(x25), PREVIEW=141(x8), ENDOFNOTE_1P=143(x25), IR_CONNECT_BEGIN=172(x8), IR_CONNECT_SUCCESS=173(x13), IR_CONNECT_FAIL=174(x1), SCORE_BEST=351(x1), SCORE_TARGET=352(x1)
(주의: 14키용 `KEY8..KEY9`, `KEY10..99` 타이머와 HCN_ACTIVE/DAMAGE 타이머는 Root/maintimer.lua 에 정의되어 있으나 ModernChic 이 이름으로 참조하지 않는다. DP 파트는 키 번호를 계산해 숫자 id 로 쓰는지 여부 미확인 - 읽지 않은 파일.)

### STRING (60)
RIVAL=1, PLAYER=2, SELECTED_TARGET=3, TITLE=10, SUBTITLE=11, FULLTITLE=12, GENRE=13, ARTIST=14, SUBARTIST=15, SEARCHWORD=30, SKIN_NAME=50, SKIN_AUTHOR=51, SKIN_CUSTOMIZE_CATEGORY1..10=100-109, SKIN_CUSTOMIZE_ITEM1..10=110-119, COURSE1_TITLE=150, COURSE6_TITLE=155, TARGET_FORWARD1..10=200-209, TARGET_BACKWARD1..10=210-219, DIRECTORY=1000, TABLE_FULL=1003, VERSION=1010, IR_NAME=1020, SONG_HASH_MD5=1030, SONG_HASH_SHA256=1031 (그 외 `ref = 150..159` 코스 제목과 `120..129` IR 랭킹 이름은 숫자 배열 리터럴로 직접 참조: Select/lua/require/textproperty.lua:19-20, Result/lua/require/textproperty.lua:23-24)

### BUTTON (70)
MODE=11, SORT=12, KEYCONFIG=13, SKINSELECT=14, AUTOPLAY=16, REPLAY=19, GAUGE_1P=40, RANDOM_1P=42, RANDOM_2P=43, DPOPTION=54, HSFIX=55, BGA=72, JUDGE_TIMING=74, JUDGE_TIMING_AUTO_ADJUST=75, GAUGEAUTOSHIFT=78, RIVAL=79, FAVORITTE_CHART=90, SKINSELECT_7KEY=170, _5KEY=171, _14KEY=172, _10KEY=173, _9KEY=174, _MUSIC_SELECT=175, _DECIDE=176, _RESULT=177, _KEY_CONFIG=178, _SKIN_SELECT=179, _SOUND_SET=180, _THEME=181, _BATTLE7=182, _BATTLE9=184, _COURSE_RESULT=185, CHANGE_SKIN=190, OPEN_IR_WEBSITE=210, SKIN_CUSTOMIZE1..10=220-229, ASSIST_EXJUDGE=301, ASSIST_CONSTANT=302, ASSIST_JUDGEAREA=303, ASSIST_LEGACY=304, ASSIST_MARKNOTE=305, ASSIST_BPMGUIDE=306, ASSIST_NOMINE=307, LNMODE=308, PRACTICE=315, REPLAY2/3/4=316/317/318, AUTOSAVEREPLAY_1..4=321-324, LANECOVER=330, LIFT=331, HIDDEN=332, JUDGEALGORITHM=340, BOTTOMSIFTABLEFGAUGE=341, HISPEEDAUTOADJUST=342, SKINSELECT_24KEY=386, _24KEY_DOUBLE=387, _24KEY_BATTLE=388, CONSTANT=400

### OFFSET / GRAPH / SLIDER / IMAGE
OFFSET: SCRATCHANGLE_1P=1(x2), SCRATCHANGLE_2P=2(x1), LIFT=3(x224), LANECOVER=4(x10), NOTES_1P=30(x2), JUDGE_1P=32(x180)
GRAPH: LOAD_PROGRESS=102(x2), SCORERATE=110(x2), SCORERATE_FINAL=111(x2), BESTSCORERATE_NOW=112(x2), BESTSCORERATE=113(x4), TARGETSCORERATE_NOW=114(x2), TARGETSCORERATE=115(x2), RATE_PGREAT=140, RATE_GREAT=141, RATE_GOOD=142, RATE_BAD=143, RATE_POOR=144, RATE_EXSCORE=147 (각 x1)
SLIDER: LANECOVER=4(x5), MUSIC_PROGRESS=6(x3), SKINSELECT_POSITION=7(x1), IR_POSITION=8(x1), MASTER_VOLUME=17(x2), KEY_VOLUME=18(x2), BGM_VOLUME=19(x2)
IMAGE: STAGEFILE=-100(x7), BACKBMP=-101(x2), BANNER=-102(x1), SKINTHUMBNAIL=-105(x1), BLACK=-110(x43), WHITE=-111(x11)

---

## 14. 부록 B - 속성 모듈이 정의하는 옵션 이름표 (SP 기준, 기본값 `def` 포함)

(op 번호는 선언 순서의 900부터 연속; 필요 시 sp_property.lua:80-212 의 `chiled` 호출 순서로 계산)

SP 옵션 이름(def): プレイサイド(1P（左スクラッチ）), プレイ位置(1P（左側表示）), ターゲット差分(非表示), ターゲット差分の種類(目標ランク), ターゲット差分表示位置(TYPE-A), 判定タイミング(非表示), 判定タイミングの種類(FAST/SLOW), 判定タイミング表示位置(TYPE-A), 判定タイミングボム(無効), ボムの種類(ModernChic規格), グラフバーの伸びる向き(左方向), スコアグラフ、ノート分布、タイミングエリアの配置(TYPEA), 画像フォント(無効), グローランプ(表示), ゲージMAXインジケータ(表示), ゲージ(表示), グラフエリア(表示), ノート分布(表示), ノート分布パターン(判定), タイミンググラフ(表示), タイミンググラフパターン(分布グラフ), タイミンググラフ（分布グラフ）表示位置(BGA側), 同 倍率(標準倍率), 同 配色パターン(通常), ヒットエラービジュアライザーパターン（分布グラフ）(通常), オートプレイ＆リプレイ時の案内(表示), キービームの高さ(50%（短い）), キービームの消失時間(通常), キービームの消失パターン(TYPE-B), BGA表示パターン(16:9), 汎用BGAの種類(動画), 戦闘モード(無効), 終了時にレーンカバーを下ろす(無効), プレイ状況詳細モード(無効), レーンカバーローテーション(無効), スコアフラップ(無効), (5키만) 5鍵用レーンカバー（5鍵モード時のみ）(表示).
SP 파일 이름(def, path 상대): 背景(#default, Play/parts/common/bg/*.png), グラフバー用背景(#default, Play/parts/sp_hw/graphbg/*.png), 汎用BGA（動画）(#default, .../common/BGA/movie/*.mp4), 汎用BGA（画像）(#default, .../BGA/image/*.png), ノーツ(#default), 判定文字(#default), レーンカバー(#default), リフト(#default, Play/parts/sp_hw/lift/*.png), ボム（ModernChic規格）(diamond SCUROed., Play/parts/common/bomb/*.png), ボム（OADX規格）(DEFAULT, .../oadx_bomb/*.png), フルコンボエフェクト(#default), キービーム(#default), キーイメージ(harf), キーフラッシュ(#default), 判定ライン色(#default), グローランプ（判定ライン上のやつ）(#default), プログレスランプ（進捗バーのあれ）(#default), ゲージMAXインジケータランプ(#default, .../lamp/*.png), ゲージ(#default, .../gauge/*.png), スクラッチイメージ(#default, .../scratch/*.png).
SP 오프셋(이름 -> id, 편집 성분): BGAの明るさ(a), 背景の明るさ(a), レーンの明るさ(a), 小節線の明るさ(a), グラフエリア背景画像の明るさ(a), 判定ラインの高さ(h), グローランプの高さ(h), ターゲット差分、判定タイミングの位置(x,y), ボムの大きさ(w), タイミンググラフの位置（プレイエリア側時に有効）(x,y). id 는 정의 순서 40..49 (bgBrightness=40, graphBrightness=41, bgaBrightness=42, tarjudgeOffset=43, bombSize=44, laneBrightness=45, barlineBrightness=46, judgelineHeight=47, glowlampHeight=48, timinggraphOffset=49; `module.offset` 배열 순서와 id 순서는 다름).
Select: 옵션 15(背景の種類 def 静止画, 画像フォント 無効, ステージ＆バナーファイル 両方表示, 曲リストの並び 直線, サブタイトルのスクロール 有効, ビーム（装飾）有効, 開始パターン フェードイン, IR情報表示 クリアレート&フルコンボレート, サイドメニューの開閉状態を保持する 無効, スキン更新チェック 有効, 言語 日本語, プレイ履歴表示 無効, キーマップ表示 有効, 背景ローテーション 無効, BPM連動キャラクター 有効), 파일 3(背景（静止画）Select/bg/image/*.png #default, 背景（動画）Select/bg/movie/*.mp4 BGmovie01, BPM連動キャラクター Root/image/*.png yuki), 오프셋 1(背景の明るさ 0~255 (255で真っ暗になります), a, id 40).
Decide: 옵션 6(背景の種類 静止画, 画像フォント 無効, ステージファイル 有効, ノーツ分布グラフ 有効, ステージ数表示 無効, お役立ち情報表示 有効), 파일 2(Decide/bg/image/*.png sample, Decide/bg/movie/*.mp4 sample).
Result: 옵션 15 기본(画像フォント 無効, グラフ&スコア表示位置 左, 項目表示切り替え コンボ数表示, IRメニュー表示 有効（オンライン時のみ表示されます）, IRメニューの種類 IRランキングTOP10, 開始時アニメーション 有効, タイムスタンプ 有効, 修飾（ビーム）有効, 修飾（リング）有効, キャラクター表示 有効, クリアランクイメージ表示パターン フェードアウト, プレイ履歴の保存 無効, NOPLAYからのランプ更新保存 無効, キャラクターローテーション 無効, 背景ローテーション 無効) + (result 만) 背景表示パターン(Clear or Failed), キャラクター表示パターン(Clear or Failed), ステージファイル表示(無効), タイミンググラフ倍率(標準倍率), タイミンググラフ配色パターン(通常) / (course) 위 두 패턴만(항목 3개). 파일 25 기본 + result 20, 오프셋 2(背景の明るさ(a)=40, キャラクター表示位置調整(x,y)=41).
