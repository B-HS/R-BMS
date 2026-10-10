# M6 조사 보고서: ModernChic 키 설정 · 스킨 선택 스킨, 사운드, 전체 자산

> 최종 갱신 2026-10-10 · 대응 단계: 웨이브 5(선곡) 반영 · 본문은 작성 시점 서술이며, 아래 "웨이브 … 반영 사항"이 최신 것부터 우선한다 · 색인과 갱신 규칙은 [README.md](README.md)

## 웨이브 5 반영 사항 (2026-10-10)

songlist 재작성, 선곡 상태 모델, 선곡 입력 키 표와 장면 수명, 패널 1~3 과 옵션 이벤트, 호스트 군집 F·H(선곡)·E, 슬라이더 쓰기와 편집 텍스트, 선곡 사운드, 스킨 위 시스템 오버레이를 넣은 뒤의 상태다.

- (W5-6) m6-misc-assets.md §10: 사운드 세트는 스킨이 아니라 audio.sound_folder(원본 soundpath/bgmpath에 대응)로 정해지므로 ModernChic Sound/는 사용자가 폴더로 지정할 때만 쓰입니다. 이 폴더에는 select.*가 없어 BGM 루프는 생기지 않습니다.


조사 대상: `/Users/hyunseokbyun/Downloads/ModernChic` (이하 MC), 비교 기준: `/Users/hyunseokbyun/development/beatoraja` (이하 BJ, 2026-10-07 커밋 8320241d 기준의 포크, LuaJ 사용).
표기: `MC:파일:줄` = ModernChic 파일, `BJ:경로:줄` = beatoraja 소스(`src/bms/player/beatoraja/` 기준). 미확인은 "미확인"으로 표시.
좌표 규칙: 스킨 좌표의 y 는 화면 아래가 0 인 위쪽 증가(y-up)입니다. 상단 기준 y 는 `1080 - y - h` 로 환산해 별도 표기합니다.

---

## 0. 핵심 결론 (구현 사양에 직접 영향)

1. **keyconfig 스킨은 BJ 에서도 화면을 그리지 않는다.** `KeyConfiguration.render()` 는 스킨 객체를 그리지 않고 스킨의 scaleX/scaleY 만 읽는다(BJ:config/KeyConfiguration.java:98-114, 131-134). 게다가 MC 의 keyconfig.lua 는 본체 실행 시 Lua 오류로 실패한다(5.1 절). 결과적으로 BJ 는 `SkinLoader.load` 의 기본 스킨 대체(BJ:skin/SkinLoader.java:42-53) 또는 내장 HD 헤더(BJ:KeyConfiguration.java:100-105)로 떨어진다. R-BMS 는 키 설정 화면을 **스킨 무관 네이티브 UI** 로 두고, keyconfig 는 헤더(type 8, 1920x1080)만 읽으면 충분하다.
2. **skinselect 스킨(type 9)은 beatoraja 의 스킨 선택 화면 전용**이며 R-BMS 에는 이에 해당하는 스킨 구동 화면이 없다(R-BMS 는 SKIN 탭 `apps/rbms-player/src/skin_select.rs`, `SCREEN_SKIN_TYPES` 는 선택·결정·결과·키설정만: `crates/rbms-skin/src/loader.rs:221`). 따라서 skinselect.luaskin 은 지원 대상에서 제외하고 헤더만 인식하면 된다. 단 시각 동일성 검증용으로 7절에 전체 좌표를 기록했다.
3. **keyconfig/skinselect 도 로드 시 `Root.define2` 를 통해 전체 공용 모듈을 실행**한다. 이 과정에서 `luajava`(`bindClass("java.io.File")`), `io.open`(읽기), `os.date`, `skin_config.get_path` 가 **모듈 로드 시점**에 호출된다(MC:Root/customfunction.lua:7-8, Root/customtime.lua:6-10, Root/customnumber.lua:289-303). 다른 화면도 동일하므로 R-BMS 의 Lua 런타임은 이 4가지를 로드 시점부터 지원해야 한다.
4. **텍스트 좌표 의미**: `align=CENTER` 면 destination x 는 박스의 **중앙**, `RIGHT` 면 **오른쪽 끝**, `LEFT` 면 왼쪽 끝(BJ:skin/SkinTextFont.java:167). 글자 크기 배율은 `dst.h / text.size`(BJ:SkinTextFont.java:191 `scaleY = region.height / parameter.size`), 상단 기준 그리기는 `region.y + region.height`(BJ:SkinTextFont.java:176). MC 의 skinselect 레이아웃은 이 의미를 전제로 설계됐다(7절 근거).
5. **음수 destination id = 시스템 참조 이미지**: `Integer.parseInt(dst.id) < 0` 이면 `SkinImage(-id)`(BJ:skin/json/JSONSkinLoader.java:314-320). 참조 번호는 100 스테이지파일, 101 BACKBMP, 102 배너, 110 검정, 111 흰색, 그 외(105 포함)는 null(BJ:skin/SkinSourceReference.java:25-37). MC 의 `MAIN.IMAGE.*` 는 -100,-101,-102,-105,-110,-111(MC:Root/mainimage.lua:9-14).
6. **imageset 의 `ref` 에 버튼 id(170-185, 386-388)를 넣으면 "현재 선택된 스킨 타입이면 1, 아니면 0"** 이 된다(BJ:skin/property/IntegerPropertyFactory.java:939-943; SkinConfiguration 이 아닌 상태는 Integer.MIN_VALUE). 이것이 on/off 이미지 전환의 정체다.
7. **BJ 의 10번째 커스터마이즈 버튼 오프바이원 버그**: `isSkinCustomizeButton` 이 `id < BUTTON_SKIN_CUSTOMIZE10(229)` 로 229 를 제외한다(BJ:skin/SkinPropertyMapper.java:124-126). MC 의 주석 "10番目のクリックエリアが反応しない？"(MC:skinselect.lua:54)이 이 증상이다. 구현 시 고칠지 맞출지 결정 필요(openQuestions).
8. **자산 용량**: 총 574 파일, 476.4 MiB(499,520,960 B). 동영상 8개 192.8 MiB, 비트맵 폰트(fnt+페이지) 139.4 MiB, TTF 11개 54.7 MiB(3종 고유, 복사본 중복), PNG 나머지 약 89 MiB. 기본 설정에서 쓰지 않는 묶음을 빼면 약 40 MiB 안팎까지 줄일 수 있다(9절).
9. **라이선스**: "스킨에 포함된 데이터는 자유롭게 사용 가능하나, 허가 없이 스킨 자체를 2차 배포하는 것은 금지. 개조판 공개 시 readme 등에 원작자(KASAKO) 표기 필요." R-BMS 저장소에 번들로 포함해 공개 배포하려면 작가 허락이 필요할 수 있고, 제3자 자산(VOICEVOX 음성, 캐릭터 일러스트, 폰트, 랭크 글자)은 별도 조건이 있다(12절).

---

## 1. 헤더

### 1.1 진입 파일(.luaskin) 10개 공통 구조
- 내용은 모두 `local t = require("<화면 모듈>")` 후 `if skin_config then return t.main() else return t.header end`(예: MC:keyconfig.luaskin, skinselect.luaskin; 각 95-100 바이트).
- 즉 **헤더 로드 시에는 `skin_config` 가 nil**, 본체 로드 시에는 테이블이다. BJ 는 `loadHeader` 에서 `skin_config` 를 설정하지 않고 실행하고(BJ:skin/lua/LuaSkinLoader.java:55-65), `load` 에서 `exportSkinProperty` 로 `skin_config` 를 설정한 뒤 다시 실행한다(LuaSkinLoader.java:79-96). 헤더 로드 때 `require("main_state")`, `timer_util`, `event_util` 는 빈 테이블로 대체된다(BJ:skin/lua/SkinLuaAccessor.java:81-87).
- 화면별 모듈: course→course.lua, decide→decide.lua, keyconfig→keyconfig.lua, musicselect→musicselect.lua, play5/7/10/14_hw→각 파일, result→result.lua, skinselect→skinselect.lua.

### 1.2 이 보고서 담당 두 화면의 헤더
| 항목 | KeyConfig | SkinSelect | 근거 |
|---|---|---|---|
| type | 8 (KEY_CONFIG) | 9 (SKIN_SELECT) | MC:KeyConfig/lua/require/header.lua:8, SkinSelect/lua/require/header.lua:8; BJ:skin/SkinType.java:20-21 |
| name | `"ModernChicKeyConfig-" .. 4.6` → "ModernChicKeyConfig-4.6" | "ModernChicSkinSelect-4.6" | header.lua:9, Root/version.lua(`4.6` 숫자) |
| w / h | 1920 / 1080 | 1920 / 1080 | header.lua:10-11 |
| fadeout | 1000 | 1000 | header.lua:12 |
| scene | 3000 | 3000 | header.lua:13 |
| input | 500 | 500 | header.lua:14 |
| property | `PROPERTY.property` = `{}` | `{}` | KeyConfig/lua/require/property.lua:73-75, SkinSelect/lua/require/property.lua:73 |
| filepath | `{}` | `{}` | property.lua:77-78 / :74 |
| category | `{}` | `{}` | property.lua:84-86 / :80 |
| offset | 없음 | 없음 | header.lua 에 키 없음 |
| author | "KASAKO" | "KASAKO" | Root/author.lua:2 |

두 화면 모두 옵션(op id)·파일 슬롯·오프셋이 **하나도 없다**. property.lua 에는 다른 화면과 같은 헬퍼(`customoption.parent/chiled/filepath/offset`, 번호 시작: option 899→900-999, offset 39→40+)가 복붙돼 있으나 호출되지 않는다.
참고(다른 화면): Decide type 6, Select type 5, Result `load(type)`(type 7 또는 15), Play `load(type)`(0,1,2,3, …) — 전부 w=1920,h=1080(MC:Decide/Select/Result/Play 의 lua/require/header.lua). Result 는 scene=3600000,input=2500,fadeout=1000, Play 는 loadend=3500,playstart=1000,scene=3600000,input=0,close=3000,fadeout=500, Select 는 fadeout=500,scene=3000,input=500, Decide 는 fadeout=1000,scene=3000,input=500. Play 이름은 "ModernChicPlay(SCURO)-4.6".

### 1.3 user 설정 파일 config.lua (헤더가 아닌 Lua 파일 옵션)
`require("config")`(MC:config.lua, 222줄)를 모든 화면이 로드한다. 사용자가 직접 편집하는 값(옵션 UI 아님). 키와 기본값:
| 경로 | 기본값 | 의미 |
|---|---|---|
| play.valiableSUD.sw / alfa | true / 100 | CONSTANT 시 추종 레인커버 사용/투명도 |
| play.valiableJUDGE.sw / rest / alfa | false / 0.6 / 50 | 레인커버 위치에 따른 판정 문자 투명도 |
| play.sectionScore.sw / type / sound.sw / sound.type | false / 1 / true / 2 | 4구간 점수 차 표시(1 타깃, 2 자기베스트), 구간 전환음 |
| play.notesAnimation | true | LN/CN/HCN/BSS 애니메이션 |
| play.judgeAnimation | true | 판정 문자 점멸 |
| play.hcnBomb | false | HCN 전용 폭발 |
| play.liftDisplay | true | 리프트 커버 표시 |
| play.smallGauge | false | 게이지 100칸(1%) |
| play.playscreenMessage | 0 | 게시판 출력(0 통상,1 확장,2 풀) |
| play.detailInfoAlfa | 180 | 상세정보 시 BGA 어둡기 |
| play.fcEffect | false | 풀콤보 효과 |
| voice.charctor | 1 | 1 ずんだもん, 2 春日部つむぎ |
| voice.result.sw / type | false / 3 | 결과 음성(1 랭크, 2 게이지 상태, 3 하이스코어) |
| voice.play.sw/achievement/mybest/target/harfRemain | false/true/true/false/false | 플레이 음성 |
| bpmLinkChar.charctor / sw / playside / type | 0 / false / true / 1 | BPM 연동 캐릭터(0 랜덤) |
| impression.sw/type/songLimit/bgaLimit/etcLimit/msg | false/0/500/400/100/"" | 결과 화면 간이 인프레 |
| result.irCover | false | IR 커버 초기 표시 |
| infoOutput | true | History/information.txt 에 상황 기록 |

---

## 2. 스킨 테이블 구성

### 2.1 keyconfig.lua 가 반환하려는 테이블 (MC:keyconfig.lua:17-30)
`CUSTOM.LOAD_HEADER(skin, header)` 로 헤더 키 전부 복사 후:
- `source = {}`, `image = {}`, `value = {}`, `judgegraph = {}`, `bpmgraph = {}`, `destination = {}` (모두 빈 테이블)
- `font = textProperty.font`, `text = textProperty.text` — **Decide 의 textproperty 를 재사용**(MC:keyconfig.lua:19-20, `require("Decide.lua.require.textproperty")`).
- **실패 원인**: Decide/lua/require/textproperty.lua 는 모듈 로드 시 `PROPERTY.isOutlineFont()`(MC:Decide/lua/require/textproperty.lua:37)를 호출하는데, keyconfig 의 전역 `PROPERTY` 는 KeyConfig/lua/require/property.lua 의 모듈로 `isOutlineFont` 가 없다(그 함수는 Decide/lua/require/property.lua:77 에만 정의). 따라서 본체 로드 시 "attempt to call a nil value" 가 발생한다. BJ 는 `catch (Throwable)` 후 null 반환(BJ:LuaSkinLoader.java:96-98 근방의 `e.printStackTrace()`)→기본 스킨 대체.
- 헤더만 읽을 때(`skin_config == nil`)는 오류가 없다. R-BMS 는 type 8 문서를 헤더 전용으로 처리해도 동작이 동일하다.
- Decide 텍스트가 모듈 로드 시 `main_state.option(MAIN.OP.DIFFICULTY1..5,0)` 를 호출한다(MC:Decide/.../textproperty.lua:8-21,35) — 로드 시점 `main_state` 접근 필요(다른 화면에서도 동일, 3.4절).

### 2.2 skinselect.lua 테이블 (MC:skinselect.lua:133-165, 본체 로드 후 실제 개수)
| 키 | 개수 | 대표 예 |
|---|---|---|
| source | 2 | `{id="background", path="SkinSelect/parts/bg.png"}`, `{id="system", path="SkinSelect/parts/system.png"}` (skinselect.lua:143-144) |
| font | 2 | `{id="medium", path="SkinSelect/font/ttf/mgenplus-1c-medium.ttf"}`, `{id="black", ...black.ttf}` (SkinSelect/lua/require/textproperty.lua:25-28). 문자열 id 사용. `black` 은 어떤 text 도 쓰지 않음 |
| text | 22 | `t_skinName`(size35,ref50), `t_skinAuthor`(size22,ref51), `t_category1..10`(size25, ref100..109), `t_item1..10`(size25, ref110..119). 전부 font="medium", `overflow=SHRINK(1)`, `align=CENTER(1)` (textproperty.lua:7-22) |
| image | 58 | bg 1, 메뉴 on/off 16, 플레이스킨 on/off 20, iFrame 1, 커서 4(l/r 일반·hover), clickArea1..10, skinChangeArea 1, 스킨 커서 4, slider_flame 1 |
| imageset | 18 | 메뉴 8(`musicselect`,`decide`,`result`,`courseresult`,`soundset`,`keyconfig`,`skinselect`,`theme`), 플레이 10(`"7"`,`"7battle"`,`"14"`,`"5"`,`"10"`,`"9"`,`"9battle"`,`"24"`,`"24battle"`,`"48"`) |
| value | 0 | `{}` |
| slider | 1 | `slider_body` |
| destination | 108 | bg 1 + 메뉴 8 + 플레이 10 + 행 10×8=80 + 선택기 5 + 이름 1 + 작성자 1 + 슬라이더 2 |
| 그 외 | — | `skin.skinSelect` 는 주석 처리(skinselect.lua:153)이며 `skinThumbnail()` 호출도 주석(:160) — 썸네일 객체 없음 |
`image` 에서 id 가 문자열이며 `"7"`,`"5"`,`"10"` 같은 숫자형 문자열 id 가 있다. `Integer.parseInt` 는 성공하지만 음수가 아니므로 참조 이미지로 오인되지 않는다(BJ:JSONSkinLoader.java:316-319). R-BMS 도 "id 문자열이 **음의 정수**일 때만" 시스템 참조로 취급해야 한다.

### 2.3 스킨 전체(참고, 담당 외)
.luaskin 10, Lua 126 파일 25,512줄(Root 22파일 2,718줄, Select 25파일 5,343줄, Play 46파일 10,655줄, Result 14파일 3,785줄, Decide 3파일 247줄, KeyConfig 2파일 111줄, SkinSelect 3파일 139줄, 최상위 11파일 2,514줄).

---

## 3. 객체 기능 사용 목록 (두 화면 한정)

### 3.1 destination 필드
| 필드 | keyconfig | skinselect | 사용 위치 |
|---|---|---|---|
| id | — | 사용 | skinselect.lua 전역 |
| dst[] 키프레임 | — | x,y,w,h 모두 사용; `time,a` 는 slider_body 한 곳 | skinselect.lua:124-130 |
| timer / loop / blend / filter | — | **미사용** | — |
| op | — | `op = {}` 빈 배열 1건(slider_flame) | skinselect.lua:123 |
| draw / offset(s) / stretch / center / acc / r,g,b / angle | — | **미사용** | — |
| mouseRect | — | 22건 (`l_cursor_hover`×10, `r_cursor_hover`×10, `skin_l_cursor_hover`, `skin_r_cursor_hover`) | skinselect.lua:73,75,93,95 |
키프레임 기본값/상속 규칙(BJ:JSONSkinLoader.java:421-452): 첫 키프레임 time=0,x=y=w=h=0,acc=0,angle=0,a=r=g=b=255, 이후 생략 필드는 직전 값 상속. slider_body 의 2,3번째 키프레임은 `time` 과 `a` 만 바꾼다.
시간 처리(BJ:skin/SkinObject.java:357-372): destination 의 `loop` 기본 0 → 마지막 time 으로 주기 반복(`time % lastTime`). slider_body 는 0→1500→3000ms 에 a=200→255→200 로 **3000ms 주기로 계속 점멸**한다. `loop=-1` 이면 종료 후 숨김.

### 3.2 객체 필드
| 객체 | 사용 필드 | 줄 |
|---|---|---|
| image | id, src, x, y, w, h, **act**(숫자 id), **click**(2=SEPARATE) | skinselect.lua:27-28,45-46,56-61,69,85,88-91,99,121 |
| imageset | id, images[2], **act**=ref=버튼 id | :29,47 |
| slider | id, src, x,y,w,h, **type=7**, **range=590**, **angle=2**(DOWN), **changeable=true** | :122 |
| text | id, font(문자열), size, **ref**(숫자), **overflow**=1, **align**=1 | textproperty.lua:8,12,19-20 |
| 미사용 | value 함수, timer 함수, cycle, divx/divy, len, zeropadding, wrapping, constantText, isRefNum, min/max, direction, shadow/outline | — |
- click 타입 의미(BJ:SkinObject.java:60-65,674-695): 0 좌클릭=+1/우클릭=-1(`buttonEvents {1,-1,1,1,-1}`), 1 반대, **2 좌우 분할(오브젝트 중앙보다 x 가 크거나 같으면 +1, 작으면 -1)**, 3 상하 분할. `act` 의 숫자 id 가 이벤트 실행에 쓰이며 imageset 은 click 미지정(=0).
- 클릭 디스패치: 마지막에 그려진 오브젝트부터 역순으로 `draw==true && mousePressed` 첫 성공에서 중단(BJ:Skin.java:398-410). 슬라이더 드래그는 슬라이더 오브젝트만 대상(Skin.java:413-419).
- slider: 방향 2(DOWN)는 y 를 `-value*range` 만큼 이동해 그린다(BJ:SkinSlider.java:116-119), 히트 영역은 x∈[region.x, +w], y∈[region.y-range, region.y]이며 `value=(region.y - y)/range`(BJ:SkinSlider.java:154-166), 양끝 1px 이내면 0/1 로 스냅. `range` 는 `dstr.height/sk.h` 로 스케일(BJ:JsonSkinObjectLoader.java:~410).
- mouseRect: 마우스 좌표(목적 해상도 픽셀, y-up)에서 오브젝트 region 원점을 뺀 값이 사각형 안에 있을 때만 그린다(BJ:SkinObject.java:603-607, 스케일 `x*dw,y*dh`: BJ:Skin.java:183-185).

### 3.3 헤더 키(본체 수준)
`fadeout/scene/input` 사용. `loadstart/loadend/playstart/close` 는 Play 에서만 사용(MC:Play/lua/require/header.lua:14-19).

### 3.4 (참고) 모듈 로드 시점 부수 효과
Decide/Select/Result/Play 의 textproperty.lua 는 로드 시 `main_state.text(FULLTITLE)`, `main_state.number(TIME_YEAR..)` 등을 **상수 문자열로 즉시 평가**한다(MC:Result/lua/require/textproperty.lua:20-24 `bottomResult`,`dete`; Select/lua/require/textproperty.lua 의 `version`). 스킨 Lua 는 **화면 진입 시마다 새 Lua 상태에서 다시 실행**돼야 값이 맞는다(BJ 는 `SkinLoader.load` 시마다 새 `LuaSkinLoader` 생성: BJ:SkinLoader.java:68).

---

## 4. Lua 함수가 값으로 들어가는 자리 (두 화면)
- **skinselect: 0건.** `draw=function`, `value=function`, `timer=function`, `act=function`, `customTimers`, `customEvents` 모두 없음. 모든 `act`/`ref` 는 숫자 상수.
- keyconfig 본체(실패 경로)에서 닿는 Decide textproperty 의 `value = function()`: `artist`(→`createFullArtist`: `main_state.text(SUBARTIST)==""` 분기 후 `ARTIST` + " " + `SUBARTIST`), `stage`(→`"STAGE" .. CUSTOM.NUM.todaySongUpdateCount + 1`) (MC:Decide/lua/require/textproperty.lua:47,50). 부수효과 없음, 단 `CUSTOM` 전역 의존.
- 참고(전체 스킨): `main_state.audio_play` 호출 82건, `skin_config.get_path` 238건, `timer_util.timer_observe_boolean` 148건(조건→타이머 변환), `main_state.option` 230건, `main_state.number` 207건, `main_state.text` 62건. `act = function() ... end` 은 Result 의 버튼(예: Result/lua/diflist.lua `difBtn`, 파일 기록과 사운드 재생)에서 쓰이며 skinselect 와 무관.

---

## 5. 참조 속성 id 전수 (두 화면)

### 5.1 버튼/이벤트 id (MC:Root/mainbutton.lua, BJ:skin/SkinProperty.java:977-1048)
skinselect 가 쓰는 id 와 BJ 의미(`SkinPropertyMapper.getSkinSelectType`: 170-185 → `SkinType.id = id-170`, 386-388 → `id-386+16`; BJ:SkinPropertyMapper.java:101-114):
| id | MC 이름 | SkinType | 사용 이미지 id |
|---|---|---|---|
| 175 | SKINSELECT_MUSIC_SELECT | 5 MUSIC_SELECT | musicselect |
| 176 | SKINSELECT_DECIDE | 6 | decide |
| 177 | SKINSELECT_RESULT | 7 | result |
| 185 | SKINSELECT_COURSE_RESULT | 15 | courseresult |
| 180 | SKINSELECT_SOUND_SET | 10 | soundset |
| 178 | SKINSELECT_KEY_CONFIG | 8 | keyconfig |
| 179 | SKINSELECT_SKIN_SELECT | 9 | skinselect |
| 181 | SKINSELECT_THEME | 11 | theme |
| 170 | SKINSELECT_7KEY | 0 PLAY_7KEYS | "7" |
| 182 | SKINSELECT_BATTLE7 | 12 | "7battle" |
| 172 | SKINSELECT_14KEY | 2 | "14" |
| 171 | SKINSELECT_5KEY | 1 | "5" |
| 173 | SKINSELECT_10KEY | 3 | "10" |
| 174 | SKINSELECT_9KEY | 4 | "9" |
| 184 | SKINSELECT_BATTLE9 | 14 | "9battle" |
| 386 | SKINSELECT_24KEY | 16 | "24" |
| 388 | SKINSELECT_24KEY_BATTLE | 18 | "24battle" |
| 387 | SKINSELECT_24KEY_DOUBLE | 17 | "48" (라벨은 "24KEY DP") |
| 190 | CHANGE_SKIN | — | skinChangeArea(click=2: 좌=이전, 우=다음; BJ:SkinConfiguration.java:116-123) |
| 220..229 | SKIN_CUSTOMIZE1..10 | — | clickArea1..10 (**229 는 BJ 에서 무시됨**) |
- 미사용 정의: 183(BATTLE5)은 이미지가 없음(MC 시트에 "5KEY BATTLE" 없음). 나머지 mainbutton 항목(11-19,40-90,301-361,400 등)은 skinselect 에서 쓰이지 않음.
- 이미지셋 `ref`(=같은 id)는 "현재 타입이면 1" 로 images[1](on) 을 선택(BJ:IntegerPropertyFactory.java:939-943). 초기 타입은 `skinSelect.defaultCategory` 가 없으므로 0(PLAY_7KEYS)(BJ:SkinConfiguration.java:53, JsonSkin.java:539-543 기본값 0). 즉 처음엔 "7KEY" 가 노란 on 이미지.
- 이벤트 실행 결과: 타입 버튼→`changeSkinType`(BJ:SkinConfiguration.java:142-144), CHANGE_SKIN→`setNextSkin/PrevSkin`.

### 5.2 문자열 id (text ref) (MC:Root/mainstring.lua:20-44, BJ:SkinProperty.java:224-229)
| id | 이름 | 사용 |
|---|---|---|
| 50 | SKIN_NAME | t_skinName |
| 51 | SKIN_AUTHOR | t_skinAuthor |
| 100-109 | SKIN_CUSTOMIZE_CATEGORY1..10 | t_category1..10 (`getCategoryName(index)`: `customOptions[index+offset]`, 없으면 "": BJ:SkinConfiguration.java 의 getCategoryName) |
| 110-119 | SKIN_CUSTOMIZE_ITEM1..10 | t_item1..10 (`getDisplayValue`) |

### 5.3 슬라이더/그래프/이미지 id
- 슬라이더 type 7 = `SKINSELECT_POSITION`(MC:Root/mainslider.lua:13). BJ 구현: `getSkinSelectPosition()=customOptionOffset/customOptionOffsetMax`, `setSkinSelectPosition(v)`(0≤v<1 일 때 `customOptionOffsetMax*v`)(BJ:FloatPropertyFactory.java:236-241, SkinConfiguration.java:82-90). 마우스 휠로도 스크롤(`mov=-getScroll()`, 0..offsetMax 클램프: SkinConfiguration.java:71-75).
- **주의**: MC 는 `skinSelect` 테이블을 두지 않아 `customPropertyCount=-1`(BJ:JSONSkinLoader.java:338-362 는 `sk.skinSelect != null` 일 때만 계산) → `customOptionOffsetMax = max(0, size - (-1)) = size+1`(SkinConfiguration.java:217). 따라서 BJ 는 목록 끝을 지나 한 칸 더(모든 행이 빈) 스크롤된다. 이 동작을 복제할지 10 으로 둘지는 결정 사항.
- 옵션 id(op), 타이머, 숫자(value ref), 그래프 id: **skinselect/keyconfig 는 사용하지 않음.**
- 이미지 참조 id(음수): 정의만 있음(`MAIN.IMAGE.SKINTHUMBNAIL = -105` 포함, MC:Root/mainimage.lua:12). skinselect 는 `skinThumbnail()` 이 주석이라 사용하지 않는다. BJ 에서도 105 는 `SkinSourceReference` 의 default 분기라 null.

---

## 6. Lua 런타임 의존

### 6.1 두 화면 직접 사용
- `require("main_state")` (skinselect.lua:8, keyconfig.lua:8)만 하고 실제 호출은 없음(keyconfig 는 Decide textproperty 경유).
- 전역 변수: `PROPERTY`(화면별 모듈), `MAIN`(=`Root.define`), `CUSTOM`(=`Root.define2`), `CONFIG`(=`config`), `DEBUG`(미정의=nil). 파일 간 **전역 공유에 의존**하므로 R-BMS 도 `_G` 를 같은 Lua 상태 안에서 유지해야 한다.
- 로컬 `require` 이름 변환: `require("SkinSelect.lua.require.property")` → 스킨 폴더 기준 `SkinSelect/lua/require/property.lua`(점→슬래시, `?.lua`). `require("config")` → 스킨 루트 `config.lua`.
- `skin_config.*`: 본체 직접 사용 없음. 단 property.lua 의 클로저가 `skin_config.option[...]`, `skin_config.offset[...]` 를 쓰지만 호출되지 않음.

### 6.2 Root.define / Root.define2 로드 체인이 부르는 것 (두 화면 모두 로드)
Root.define2(MC:Root/define2.lua:29-37)가 `customoption, customnumber, customgraph, customslider, customfunction, customtime, customtext, customsound, customtimer` 를 `require`. 이 중 **로드 시점에 실행되는 부수 효과**:
| 위치 | 호출 | 필요 기능 |
|---|---|---|
| Root/customfunction.lua:7-8 | `require("luajava")`, `luajava.bindClass("java.io.File")` | luajava 모듈 + File 퍼사드 |
| Root/customtime.lua:6,8,10 | `os.date("%y%m%d")`, `os.date("%Y/%m/%d")`, `os.date("%H:%M:%S")` | os.date |
| Root/customnumber.lua:289-290 | `FUNC.countFileRecords(skin_config.get_path("io/Play/sp/lanecover/pathList.txt"),0)` 및 dp | `skin_config.get_path`, `io.open(path,"r")`(없으면 nil 허용) |
| Root/customnumber.lua:292-303 | 같은 방식으로 `excludeList.txt`, `History/<yymmdd>/history|clear|score|miss.txt`, `History/impression.txt`(startCount=1) | 동일 |
- 이 호출들은 파일이 없어도 `existFile` 가 `io.open` 실패(nil)를 처리한다(MC:customfunction.lua:18-28). 읽기만 하므로 샌드박스 읽기 전용 io 로도 로드는 통과한다.
- BJ 의 미리보기용 샌드박스(SkinConfiguration.loadSelectedSkinPreview 경로)는 `os=nil`, `luajava=nil`, `io` 쓰기 무시이므로(BJ:SkinLuaAccessor.java:66-71,114-122,181-198) MC 가 그 경로에서는 `os.date` 에서 실패한다. 실제 플레이 경로(`new LuaSkinLoader(state, config)`)는 표준 globals 에 `luajava` 퍼사드를 설치한다(LuaSkinLoader.java:38, SkinLuaAccessor.java:50-58, LegacySkinLuaApi.java:50-62).

### 6.3 BJ 가 MC 용으로 제공하는 호환 API (R-BMS 가 대응해야 할 목록)
- `luajava.bindClass`: `com.badlogic.gdx.Gdx`, `Input`, `controllers.Controllers`, `Controller`, `java.io.File` 만 허용(BJ:LegacySkinLuaApi.java:68-76). `luajava.new(File, path)` → `mkdir()`, `listFiles()`(경로 문자열 배열, `\`→`/` 치환: LegacySkinLuaApi.java:~145-160). `luajava.newInstance("java.net.URL"/BufferedReader/InputStreamReader)` → HTTP(타임아웃 기본 1000ms/최대 5000ms, 최대 1024줄/65536자: LegacySkinLuaApi.java:~40-44,107-114).
- MC 가 실제 쓰는 것: `File:mkdir/listFiles`(MC:customfunction.lua:103-134), `Gdx.input:isKeyPressed(input.Keys.RIGHT|LEFT|UP|DOWN)`(MC:Result/lua/mainmenu.lua:6-7,986-989, irmenu.lua:6-7,654-657), `URL/BufferedReader`(MC:Select/lua/require/http.lua:22,32). `Controllers`, `Gdx.graphics` 는 MC 에서 **사용 없음**.
- `main_state` 사용 함수(전체 스킨 호출 수): option 230, number 207, audio_play 82, text 62, time 22, timer 16, timer_off_value 15, event_index 14, gauge_type 11, judge 6, volume_sys/key/bg 각 2, set_volume_sys/key/bg 각 2, rate 2, float_number 2, gauge 1, exscore 1. `audio_loop/preload/stop/dispose` 는 **미사용**.
- `skin_config` 사용: `get_path` 238, `option[name]` 7, `offset[name].{x,y,w,h,a}` 각 7. BJ 가 노출하는 필드는 `file_path`, `get_path`, `option`, `enabled_options`, `offset{x,y,w,h,r,a}`(BJ:SkinLuaAccessor.java:931-980); MC 는 `file_path`, `enabled_options`, `offset.r` 를 쓰지 않음.
- `timer_util.timer_observe_boolean(function)` 만 사용(148건; `event_util` 은 미사용). 선언만: `timer_util = require("timer_util")` (musicselect.lua:9, result.lua:9, course.lua:9).
- `skin_config.get_path(path)`: **절대 경로 문자열**을 돌려준다(`parent + "/" + path` 후 와일드카드 해석: BJ:LuaSkinLoader.java:80-82 → SkinLoader.getPath:112-148). `main_state.audio_play(path, vol)`: vol 은 0..2 로 클램프하고 시스템 볼륨을 곱함, nil 이면 1(BJ:SkinAudioLuaApiExporter.java:~53-56). MC 는 2(음성/섹션/풀콤보)와 0.0001(미리 로드 요령: Root/customsound.lua:204-270)을 넘긴다.
- `require` 이름 목록(전체): main_state 11, Root.define/define2 각 10, config 10, Play.lua.base 8, Root.version 7, Root.author 6, luajava 4, timer_util 3 등(위 grep 합계). 외부 모듈 없음 — 전부 스킨 내부 파일.
- `dofile(skin_config.get_path(...))` + `pcall` 조합이 화면 본문 파일 10개의 모든 하위 파트 로드에 쓰인다(MC:play7_hw.lua:47-… , musicselect.lua:63-291, result.lua:46-164, course.lua:46-82). 즉 **`dofile` 과 `pcall` 이 필수**이며 `dofile` 인자는 `get_path` 가 돌려준 절대 경로다.
- io: `io.open` 45회(읽기 "r", 쓰기 "w", 추가 "a"), `file:lines/write/close`. 쓰는 경로는 전부 `skin_config.get_path(...)` 하위(History/, io/, Select/lua/settings/, difflist.txt)(MC:Result/lua/history.lua, impression.lua, diflist.lua; Select/lua/require/settings.lua:37,63; http.lua:140; Root/customfunction.lua:135-175; Play/lua/sp/detailinfo/nentyakuinfo.lua:21).
- os: `os.date`(포맷 문자열 및 `'*t'` 표: Play/lua/sp/detailinfo/bgaareainfo.lua:148), `os.time()`(Select/lua/require/http.lua:87,121; bgaareainfo.lua:150).
- math: `math.random` 13(Result/lua/background.lua:13-19, listchoice.lua, Root/customnumber.lua:73, customtext.lua:33), `floor` 12, `modf` 4(bgaareainfo.lua:337-401), `exp/ceil/max` 각 1. string: `sub 2, format 2("%02d"), match 1, gsub 1, gmatch 1`. table: `insert` 2,200, `concat` 2.
- **Lua 5.1/5.2 전용 구문**: `unpack`, `setfenv/getfenv`, `loadstring`, `table.getn/maxn`, `math.pow/mod/log10`, `string.gfind`, `bit32` 사용은 **grep 결과 0건**. `goto` 도 0건. 따라서 mlua(Lua 5.4)로 구문 호환 문제는 낮다. 주의할 5.4 차이: ① 정수/실수 구분으로 `10/2` 가 `5.0` 이 되어 문자열 연결 시 `"5.0"`이 된다(LuaJ 는 `5`). 연결 대상: `os.time() + 60*60*24*7 .. "\n"`(정수라 무방), `main_state.number(..) .. "." .. ..`, `"STAGE" .. n + 1`(정수)은 안전하지만 나눗셈 결과를 `..` 하는 곳이 있는지 전수 검사 필요(미확인: Play/lua/sp/detailinfo/*.lua, Result/lua/mainmenu.lua 의 `/` 사용부 520/2 는 숫자 좌표라 무방). ② `math.floor` 반환이 정수형이라 `string.format("%d")` 는 오히려 안전. ③ `math.random(#list)` 는 5.4 에서도 유효.
- 키보드 상수 `input.Keys.RIGHT/LEFT/UP/DOWN` 은 BJ 퍼사드가 libGDX 키코드를 돌려준다(LegacySkinLuaApi.java:~184-200).
- `print` 는 디버그용 다수(`if DEBUG then print(...)`), `print("CF:ALL_EXCLUDE_INIT")` 등 무조건 출력도 있다.
- `table.insert(parts.image, {...})` 로 객체를 누적하는 패턴: R-BMS 변환기는 Lua 테이블 → 문서 모델 변환 시 키 순서가 아니라 **배열 순서가 z 순서**임을 유지해야 한다(7.4절).
- Lua → Java 변환 규칙(BJ:LuaSkinLoader.java:103-206): 필드 타입별 변환(숫자 id/함수/문자열 스크립트 모두 허용). `destination.op` 요소는 숫자이거나 함수(BooleanProperty)(JsonSkin.java:484-498). 숫자 id → 문자열 `tojstring` 이므로 `id=7` 과 `"7"` 이 동일 id.

---

## 7. 1920x1080 레이아웃

### 7.1 keyconfig
그려지는 객체 0개(destination 빈 테이블). 키 설정 화면 UI 는 BJ 가 `ShapeRenderer` 와 시스템 폰트로 직접 그린다(BJ:KeyConfiguration.java:107-114,130+). R-BMS 는 네이티브 `keyconfig.rs` 를 유지하면 됨.

### 7.2 skinselect 화면 좌표 (y-up / 상단 기준 y)
배경: `bg` image/destination 0,0,1920,1080(skinselect.lua:14-15). 배경 PNG 에 정적 글자가 구워져 있음: 좌상단 "SKIN SELECT"(약 x40,y상단55-105) + 선, "SKIN MENU"(좌 x30,y상단 약705)와 선(y상단 735), "PLAY SKIN"(x900,y상단705)와 선(y상단 735). 오른쪽에 つむぎ 캐릭터 일러스트(회색 하프톤)가 크게 깔림(`SkinSelect/parts/bg.png`, 1920x1080, 590,929 B).

| 영역 | x | y(y-up) | w | h | 상단 y | 근거 |
|---|---|---|---|---|---|---|
| 메뉴 1열(좌) | 25 | 252/174/95/17 | 395 | 71 | 757/835/914/992 | skinselect.lua:21-32 |
| 메뉴 2열 | 439 | 252/174/95/17 | 395 | 71 | 같음 | 같음 |
| 플레이 스킨 1열 | 900 | 252/174/95/17 | 325 | 71 | 같음 | :39-51 |
| 플레이 스킨 2열 | 1240 | 252/174/95/17 | 325 | 71 | 같음 | 같음 |
| 플레이 스킨 3열 | 1575 | 252/174 | 325 | 71 | 757/835 | 같음 |
| 커스터마이즈 행 i=1..10 iFrame | 900 | 1012-65(i-1) | 920 | 56 | 12+65(i-1) | :56,63-77 |
| 행 y-up 값 | | 1012,947,882,817,752,687,622,557,492,427 | | | 12,77,142,207,272,337,402,467,532,597 | |
| 카테고리 텍스트(CENTER) | 중앙 1093 | y+12 | 360 | 25 | 박스 x 913-1273 | :66 |
| 항목 텍스트(CENTER) | 중앙 1556 | y+12 | 426 | 25 | 박스 x 1343-1769 (오른쪽 끝 1769<1920) | :67 |
| clickArea(투명) | 1343 | y+3 | 426 | 50 | | :69-70 (소스 1x1: system.png x1440,y0 = RGBA(255,255,255,0) 투명 확인) |
| 좌 커서 / hover | 1312 / 1312 | y+16 | 21 | 23 | | :72-73 |
| 우 커서 / hover | 1778 / 1778 | y+16 | 21 | 23 | | :74-75 |
| 좌 hover mouseRect | | rel x20,y-13,w226,h50 → 절대 x 1332-1558, y y+3..y+53 (행의 왼쪽 절반) | | | | :73 |
| 우 hover mouseRect | | rel x-220,y-13,w226,h50 → 절대 x 1558-1784 | | | | :75 |
| 스킨 썸네일/클릭 영역 skinChangeArea | 111 | 523 | 687 | 388 | 169 | :85-86 (system.png (0,970,687,388)는 RGBA(0,0,0,252) 거의 검정 사각형이 그려짐) |
| 스킨 좌 커서 / hover | 72 | 706 | 21 | 23 | | :92-93 |
| 스킨 우 커서 / hover | 817 | 706 | 21 | 23 | | :94-95 |
| 스킨 좌 hover mouseRect | | rel x35,y-183,w344,h388 → 절대 x 107-451, y 523-911 | | | | :93 |
| 스킨 우 hover mouseRect | | rel x-367,y-183,w344,h388 → 절대 x 450-794 | | | | :95 |
| 스킨 이름 | 중앙 454 | 460 | 759 | 35 | 상단 585 | :104-108 |
| 작성자 | 중앙 454 | 425 | 759 | 22 | 상단 633 | :111-117 |
| 슬라이더 프레임 | 1858 | 423 | 13 | 644 | 13 | :120-123 (system.png (1450,0,13,644): RGBA(0,0,0,240) 검정) |
| 슬라이더 손잡이 | 1847(=1858-11) | 1003(=423+580) | 35 | 72 | 5 | :124-130 (system.png (1463,0,35,72), 연두 RGBA(180,255,0,255)) |

- 텍스트 박스가 중앙 기준이라는 근거: 이름 텍스트 중앙 x=454 가 썸네일 사각형 중앙(111+687/2=454.5)과 일치, 항목 텍스트 중앙 1556 이 클릭 영역 중앙(1343+213=1556)과 일치(7.2 계산). 구현 시 left-edge 로 해석하면 레이아웃이 크게 어긋난다.
- 슬라이더: `range=590`, 값 0 일 때 손잡이 하단 y=1003, 값 1 일 때 413. 손잡이 알파 200→255→200 점멸(3000ms 주기).
- 슬라이더 hit 영역: x 1847-1882, y 413-1003. 위치 값은 `skinselect_position`(7.. 5.3절).
- 오브젝트 id 번호형 문자열("7","5","10"...)과 imageset id 가 이미지 id 와 **별개 네임스페이스**(imageset 우선 탐색 순서: image → imageset → value … → text → slider: BJ:JsonSkinObjectLoader.java:42-100,388-410). 같은 id 가 이미지와 imageset 양쪽에 있으면 image 가 우선.

### 7.3 스프라이트 시트 `SkinSelect/parts/system.png` (1500x1400, 93,734 B) 구성
| 영역 (x,y,w,h) | 내용 |
|---|---|
| (0..395,0..567) 8행×71 | 메뉴 off(회색 버튼, 흰 글자): MUSIC SELECT, DECIDE, RESULT, COURSE RESULT, SOUND SET, KEY CONFIG, SKIN SELECT, THEME |
| (395..790, 0..567) | 메뉴 on(초록 글자·테두리) 같은 순서 |
| (790..1115, 0..851) 12행×71 | 플레이 off: 7KEY, 7KEY BATTLE, 14KEY, 5KEY, 10KEY, 9KEY, 9KEY BATTLE, 24KEY, 24KEY BATTLE, 24KEY DP, 빈칸 2 |
| (1115..1440, 0..851) | 플레이 on(노란 글자·테두리) |
| (1450,0,13,644) | 슬라이더 프레임 |
| (1463,0,35,72) | 슬라이더 손잡이 |
| (0,854,21,23)/(21,854,21,23) | 커서 hover(좌/우) |
| (0,877,21,23)/(21,877,21,23) | 커서 일반(좌/우) |
| (0,900,920,56) | iFrame(회색 행 프레임, 가운데 약 x385 에 구분선) |
| (0,970,687,388) | 썸네일 자리(거의 검정) |
- 라벨 순서가 코드의 `wd` 순서와 일치해야 함: 메뉴는 이미지 y 오프셋 `ad = 71*i`(skinselect.lua:25-33), 플레이도 동일(:43-51). 메뉴의 on 은 x=395, 플레이의 off x=790, on x=1115.

### 7.4 z 순서 (destination 선언 순서 = 그리는 순서)
1. bg → 2. 메뉴 8개 → 3. 플레이 10개 → 4. 행 1..10 각각 [iFrame, t_category, t_item, clickArea, l_cursor, l_cursor_hover, r_cursor, r_cursor_hover] → 5. skinChangeArea → 6. skin_l_cursor, skin_l_cursor_hover, skin_r_cursor, skin_r_cursor_hover → 7. t_skinName → 8. t_skinAuthor → 9. slider_flame → 10. slider_body.
- 옵션에 의한 배치 변형: 없음.
- 미리보기: MC 는 `skinpreview` 키를 정의하지 않으므로 BJ 최신 포크에서도 **미리보기는 그려지지 않고 검은 사각형만** 보인다(BJ:skin/json/JsonSkinConfigurationSkinObjectLoader.java:24-27, JsonSkin.java:47,414-416). 이 포크의 미리보기는 실험적 기능이다(커밋 b211fb72).

---

## 8. 자산

### 8.1 폴더별 파일 수 / 용량 (`du -sk` 및 stat 합계)
| 폴더 | 파일 | KB(du) | 비고 |
|---|---|---|---|
| Play | 204 | 219,000 | 동영상 130.6 MiB, 폰트 fnt+페이지 43.7 MB, 폰트 ttf 10.2 MB |
| Result | 104 | 70,280 | bg 38.9 MiB |
| Select | 86 | 117,328 | 동영상 44.1 MiB, fnt 46.3 MB, ttf 15.3 MB |
| Decide | 33 | 67,144 | 동영상 18.1 MiB, fnt 36.2 MB |
| Root | 87 | 3,008 | 소리 1.4 MiB, 이미지 1.26 MiB, Lua 2.7k줄 |
| SkinSelect | 7 | 10,872 | ttf 2개 10.4 MB, bg/system |
| Sound | 14 | 1,200 | 시스템 사운드 세트 |
| io | 13 | 44 | 런타임 기록 파일(406 B) |
| KeyConfig | 2 | 8 | Lua 2개뿐 |
| History | 2 | 8 | 런타임 기록(182 B) |
최상위 파일: 화면 .lua 10 + config.lua + .luaskin 10 + difflist.txt.

### 8.2 확장자별 합계 (stat 합계)
| 확장자 | 개수 | 바이트 | MiB |
|---|---|---|---|
| png | 284 | 222,650,468 | 212.3 (이 중 폰트 페이지 88개 132,767,842 B) |
| mp4 | 8 | 202,177,916 | 192.8 |
| ttf | 11 | 57,384,232 | 54.7 (고유 3종) |
| fnt | 10 | 13,423,085 | 12.8 |
| ogg | 73 | 2,731,393 | 2.6 |
| lua | 126 | 1,105,766 | 1.05 |
| luaskin | 10 | 972 | — |
| txt | 46 | 46,970 | 안내/기록 |
| chp | 1 | 2 | 더미 |
| 확장자 없음 | 5 | — | 아래 8.7 |
전체 574 파일 499,520,960 B = 476.38 MiB.

### 8.3 10MB 이상 파일 및 mp4 8개
| 경로 | 크기 | 용도 | 사양 |
|---|---|---|---|
| Play/parts/common/BGA/movie/NOSTALGIC.mp4 | 48,388,349 | 플레이 범용 BGA(BGA 없는 곡) | H.264+AAC, 1280x720, 123.6s |
| Play/parts/common/BGA/movie/cyber.mp4 | 44,981,048 | 동일 | H.264, 1280x720, 145.9s |
| Select/bg/movie/BGmovie02.mp4 | 24,991,514 | 선곡 배경(동영상 옵션) | H.264, 1920x1080, 25s |
| Play/parts/common/BGA/movie/travel.mp4 | 23,691,954 | 플레이 범용 BGA | H.264, 1280x720, 120.3s |
| Select/bg/movie/BGmovie01.mp4 | 21,231,110 | 선곡 배경(동영상) 기본값 `BGmovie01`(Select/lua/require/property.lua:199) | H.264, 1920x1080, 30.7s |
| Play/parts/common/BGA/movie/#default.mp4 | 19,886,632 | 플레이 범용 BGA 기본 | H.264+AAC, 1280x720, 85.5s |
| Decide/bg/movie/sample2.mp4 | 12,863,187 | 결정 배경(동영상) | H.264+AAC, 1920x1080, 20.0s |
| Decide/bg/movie/sample.mp4 | 6,144,122 | 동일 | H.264, 1920x1080, 20.0s |
| Play/parts/common/fullcombo/#default.png | 11,025,184 | 풀콤보 효과 시트 5190x2571 | PNG RGBA |
- 합계: Play BGA 동영상 136,947,983 B, Select 46,222,624 B, Decide 19,007,309 B. 오디오 트랙이 있는 영상은 3개(#default, NOSTALGIC, sample2).
- 기본 선택: Decide/Select 배경 = 정지화(`def` 가 "静止画"), 플레이 범용 BGA 기본 = **동영상**(sp/dp_property.lua:400, 378 `generalBgaPattern.movie`) 이므로 **기본 상태에서도 Play 동영상 1개는 쓰인다**.
- 큰 PNG: Result/parts/bg/clearType/*/#default.png 12개가 1.7-5.0 MB, Result/parts/bg/rank/*/#default.png 8개가 1.1-1.5 MB, Play/parts/common/bomb/engine.png 3.1 MB, Play/parts/common/bg/*.png 6개가 각 1.15-1.22 MB. 텍스처 크기 주의: fullcombo 5190x2571, Select/parts/{jp,en,cn}/mainframe.png 3200x3200, sidemenu.png 3700x1800, subop.png 2880x2300, Result/parts/system.png 2700x2200 — 4096 초과 가로(5190)가 있어 GPU 최대 텍스처 크기 확인 필요(미확인: R-BMS 의 wgpu 한도).

### 8.4 TTF 11개 (고유 3종, 라이선스 SIL OFL 1.1)
| 파일 | 크기 | md5 | 쓰는 화면 |
|---|---|---|---|
| mgenplus-1c-medium.ttf | 5,246,920 | a38034d3… | Decide, Play, Result, Select, SkinSelect (5개 동일 복사본) |
| mgenplus-1c-black.ttf | 5,183,500 | d057a653… | Decide, Play, Result, Select, SkinSelect (5개 동일 복사본) |
| mgenplus-1c-bold.ttf | 5,232,132 | 1774f556… | **Select/font/ttf 한 곳, 참조 0건**(미사용) |
- 중복 낭비: 복사본 8개 ≈ 41.9 MB, 미사용 bold 5.2 MB. 한 벌(medium+black) 10.4 MB 로 모두 대체 가능.
- 사용 크기(예): Decide title 90/genre 40/artist 40/stage 50/tips 25/tablename 35, Select title 70/subtitle 170/artist 30/genre 30/bartext 35/…, Result 30/25/18, Play 25/90/40/18, SkinSelect 35/22/25. 글자 그림자: ttf 사용 시 `shadowOffsetX/Y`(대제목 4, 부제 2)(Decide/textproperty.lua:33-34, 44-53).

### 8.5 FNT 10개 (BMFont **텍스트 형식**, 글리프 수는 모두 11,180, 2048x2048 RGBA 페이지)
| fnt | face/size | padding/spacing | lineHeight/base | 페이지 수 | 페이지 PNG 합 | fnt 크기 | md5 |
|---|---|---|---|---|---|---|---|
| Decide/font/fnt/main.fnt | Mgen+ 1c black / 50 | 8,8,8,8 / -16,-16 | 75/56 | 11 | 22,053,244 | 1,341,721 | 789ddb… (= Select main) |
| Decide/font/fnt/sub.fnt | medium / 35 | 8 / -16 | 53/40 | 7 | 12,313,554 | 1,341,441 | f100c8… (= Select sub) |
| Select/font/fnt/main.fnt | black / 50 | 8 / -16 | 75/56 | 11 | 22,053,244 | 1,341,721 | 〃 |
| Select/font/fnt/sub.fnt | medium / 35 | 8 / -16 | 53/40 | 7 | 12,313,554 | 1,341,441 | 〃 |
| Select/font/fnt/bartext.fnt | bold / 35 | 4 / -8 | 53/40 | 5 | 8,961,805 | 1,341,403 | 40c964… |
| Result/font/fnt/main.fnt | black / 30 | 4 / -8 | 45/34 | 4 | 7,010,010 | 1,337,603 | a9150e… |
| Result/font/fnt/sub.fnt | medium / 30 | 4 / -8 | 45/34 | 4 | 7,458,187 | 1,337,600 | 569ddf… |
| Play/font/fnt/title.fnt | black / 90 | 10 / -20 | 134/101 | 30 | 25,118,637 | 1,384,176 | 0a29ec… |
| Play/font/fnt/info.fnt | medium / 40 | 4 / -8 | 60/45 | 6 | 9,958,599 | 1,341,418 | 81036b… |
| Play/font/fnt/top.fnt | medium / 25 | 4 / -8 | 38/28 | 3 | 5,527,008 | 1,314,561 | 7599b6… |
- 각 fnt 는 `chars count=11180`, kerning 약 2,311(main.fnt 기준, 줄 형식 `char id=… x y width height xoffset yoffset xadvance page chnl`, `kerning first second amount`).
- 어떤 화면이 쓰나: Decide(main:id0 type1, sub:id1 type1), Select(bartext:id0, main:id1 type1, sub:id2 type1), Result(main:id0, sub:id1), Play(title:id0, info:id1, top:id2). **전부 "画像フォント(비트맵 폰트)" 옵션이 켜졌을 때만 사용**(`PROPERTY.isBitmapFont()`), 기본값은 "無効"(Decide property:101, Select:137, Result:204, Play sp:310/dp:287).
- `type = 1`: BJ 는 이를 **거리장(distance field) 비트맵**으로 취급해 별도 셰이더로 그린다(BJ:SkinTextBitmap.java:93-96,317-319; `type` 0 표준, 1 DF, 2 컬러 DF). Select/Decide 의 main/sub 에만 지정(Decide textproperty.lua:58-59, Select textproperty.lua:79-80). 페이지 알파 미리보기(Select/font/fnt/main1.png)가 두꺼운 둥근 덩어리 형태라 DF 로 보인다(추정; 셰이더 사양은 미확인).
- 외곽선: 비트맵 모드는 `outlineColor`(RGBA hex 문자열, 난이도별 색) / `outlineWidth`(0.8-1)를 쓰고, TTF 모드는 `shadowOffsetX/Y` 를 쓴다. 두 모드가 같은 id 의 text 에서 서로 다른 필드를 쓴다.

### 8.6 폰트 사용 결론
기본 설정(= 비트맵 폰트 무효)에서는 **TTF 2종(medium/black)** 만 필요하다. fnt 10개+페이지 88개(합 146,190,927 B = 139.4 MiB)는 옵션을 켜지 않는 한 로드되지 않는다.

### 8.7 기타 파일
- `.chp` 1개: `Play/parts/common/POMYU Chara/Off/dummy.chp`(2 B, 공백+개행). 용도는 beatoraja 의 PMS 캐릭터(ポムユ) 파일 슬롯인데 MC 의 해당 옵션(`POMYU Chara 1P`)은 주석 처리돼 있다(MC:Play/lua/require/dp_property.lua:405). **미사용 더미**.
- 확장자 없는 파일 `-`: `Play/parts/dummy/-`(58 B), `Select/parts/dummy/-`(58 B), `Result/parts/dummy/-`(0 B). 용도 불명(과거 구조의 자리표시), 참조하는 Lua 없음.
- `Select/lua/settings/sidemenu`(23 B, "false\nfalse\nfalse\nfalse"), `checkversion`(17 B, 타임스탬프 `1.7200937E9` + "false"): 런타임 상태 저장(Select/lua/require/settings.lua:37,63; http.lua:140). 기본판에서는 제외 대상.
- `difflist.txt`: 결과 화면이 "자작 난이도표" 용으로 곡 정보를 JSON 문자열로 추기(Result/lua/diflist.lua:8-30). `.json` 으로 쓰면 스킨으로 오인돼 강제 종료되므로 txt 로 회피한다는 TODO 주석이 있다(diflist.lua:9).
- `io/Play/{sp,dp}/lanecover/pathList.txt`, `excludeList.txt`, `temp.txt`, `io/Play/sp/lnlog|log/*.txt`, `io/Play/sp/nentyaku.txt`: 레인커버 로테이션과 플레이 로그. `History/information.txt`, `History/recent.txt`: 최근 플레이 문자열. **전부 실행 중 생성되는 사용자 상태**이며 스킨 번들에 넣을 자산이 아니다.

### 8.8 참조되는/안 되는 자산 (스크립트 `tools/assetref.py` 로 Lua 문자열 리터럴·와일드카드 매칭)
- 총 386 미디어 파일(png/mp4/ogg/fnt/ttf) 중 321 참조, 65 미참조(7,656,028 B). 미참조의 내역:
  - `Root/sounds/vo/{zundamon,tsumugi}/**` 48개: 경로가 `"…/" .. char .. "/rank/AA.ogg"` 로 **동적 조립**돼 정적 매칭에 안 잡혔을 뿐 `config.voice.charctor` 에 따라 쓰인다(MC:Root/customsound.lua:20-25,76-134). 실질 사용 대상.
  - 진짜 미사용: `Root/sounds/favorite.ogg`(19,075; `Sound/o-change.ogg` 와 동일 md5), `Select/font/ttf/mgenplus-1c-bold.ttf`(5.2 MB), `Select/sounds/close.ogg`/`open.ogg`(Root/sounds 와 동일 md5 중복), `Sound/` 14개(beatoraja 시스템 사운드 세트 폴더: 스킨 Lua 가 직접 쓰지 않음).
- 와일드카드 파일 슬롯과 소속(옵션으로 선택. 기본값 `#default` 우선): Play/parts/common 의 bg(7), judge(4), notes(10), key(2), keybeam(6), keyflash(5), judgeline(3), glow(4), progress(5), lamp(5), gauge(6), scratch(8), bomb(7, `bomb/*.png`), oadx_bomb(`dummy.png`), fullcombo(1), lanecover(1), mine(1), hcn(1), close(1), attack(2), BGA/image(`#default.png`), BGA/movie(4 mp4); sp_hw/dp_hw 의 graphbg·lift 각 1; Select/bg/image(`#default.png`), Select/bg/movie(2), Root/image(7 캐릭터 600x610), Decide/bg/image(sample,sample2), Decide/bg/movie(2); Result/parts 의 rank(7, 400x1344), gauge(6), irmask(1), bg/{isclear 2 + clearType 12 + rank 8 + all 1}, char/{동일 구성}.
- 모든 와일드카드는 `"*.png"` 규칙. 소스 경로 와일드카드는 헤더 `filepath` 선언이 없어도 BJ 가 **디렉터리 목록에서 확장자(소문자 비교)로 필터 후 무작위 선택**한다(BJ:SkinLoader.java:112-148). `Root/image/*.png`(Select/lua/mainframe.lua:165)가 그 예이다. `|` 구문(예: `"…/*|1P|"`)도 같은 함수에서 처리(주석 처리된 POMYU 옵션 포함).
- 폰트 경로는 와일드카드가 아닌 `skinPath.getParent().resolve(font.path)`(BJ:JsonSkinObjectLoader.java:628-630).

### 8.9 PNG 시트 인상 (직접 열어 확인)
- `SkinSelect/parts/system.png`: 회색 버튼 시트, on 상태는 초록(메뉴)/노랑(플레이) 글자. 7.3 절.
- `SkinSelect/parts/bg.png`: 짙은 회색 도트 패턴 위 つむぎ(春日部つむぎ) 일러스트, 제목 3개(SKIN SELECT/SKIN MENU/PLAY SKIN)가 구워져 있음. 제3자 일러스트이므로 기본 스킨 번들에서는 제외 대상.
- `Select/bg/image/#default.png`: 네온 블루/오렌지 복도 CG 배경 1920x1080(2,205,891 B).
- `Select/font/fnt/main1.png`: 2048x2048, 알파만 있는 글리프 아틀라스(첫 페이지에 기호·라틴·한자 일부).

---

## 9. 단순화 후보와 절감 추정

### 9.1 묶음별 절감 (정확 합계: `tools/bundles.py`)
| 묶음 | 파일 | 바이트 | MiB | 판단 |
|---|---|---|---|---|
| Play BGA 동영상 4개 | 4 | 136,947,983 | 130.6 | **제외**(범용 BGA 는 정지 이미지 `#default.png` 0.71 MiB 로 대체) |
| Select 배경 동영상 2개 | 2 | 46,222,624 | 44.1 | 제외 |
| Decide 배경 동영상 2개 | 2 | 19,007,309 | 18.1 | 제외 |
| 비트맵 폰트 fnt+페이지 | 98 | 146,190,927 | 139.4 | 제외(옵션 기본 OFF) |
| TTF 중복 복사본 8개 | 8 | 약 43.9M | 41.9 | 공용 1벌로 통합 |
| TTF bold 미사용 | 1 | 5,232,132 | 5.0 | 제외 |
| Result 배경(clearType 12, rank 8, isclear 2, all 1) | 27 | 40,766,712 | 38.9 | 1-2장(`all`)으로 축소, 나머지 제외 |
| Play fullcombo 효과 | 1 | 11,025,184 | 10.5 | 제외 또는 축소(풀콤보 효과 기본 비활성) |
| Play bomb 7종 | 7 | 8,516,607 | 8.1 | 1-2종만 유지(기본 사용분) |
| Play bg 7종 | 7 | 7,211,484 | 6.9 | 1장 유지 |
| Select 언어 3종 중 2개 | 12 | 약 5.15M | 4.9 | 제외(en/cn) 또는 jp 만 유지 |
| SkinSelect 폴더 전체 | 7 | 11,120,089 | 10.6 | **제외**(R-BMS 는 SKIN 탭 사용; 폰트 10.4 MB 포함) |
| Root 음성 vo(2 캐릭터) | 48 | 1,143,388 | 1.1 | 제외(VOICEVOX 약관) |
| Root/image BPM 캐릭터 7 | 7 | 1,322,066 | 1.3 | 제외 |
| Sound/ 시스템 사운드 | 14 | 1,193,301 | 1.1 | 필요한 8-10개만 유지(10절) |
| Select/sounds 중복 | 2 | 69,211 | 0.07 | 제외 |
| io/, History/, Select/lua/settings | 다수 | 약 1 KB | — | 제외(런타임 상태) |
- 합산 상한: 동영상 192.8 + fnt 139.4 + 중복/미사용 TTF 약 47 + Result bg 약 36 + fullcombo 10.5 + SkinSelect 10.6 + 기타 ≈ **약 440 MiB 절감 가능**, 남는 크기 약 **35-45 MiB**(TTF 한 벌 10.4 + Result/Select/Play/Decide PNG 약 25 + 소리 약 1.5). 정확 수치는 어떤 슬롯을 유지하느냐에 따라 달라지는 추정.

### 9.2 화면 핵심을 유지하며 뺄 수 있는 것
- 동영상 배경(8개), 비트맵 폰트(전체), BPM 연동 캐릭터, 결과 캐릭터/음성(VOICEVOX), 버전 확인 HTTP(`Select/lua/require/http.lua`, `versioncheck.lua`), 플레이 기록/인프레 파일 기록(`History/`, `io/`, `difflist.txt`, `Result/lua/history.lua`, `impression.lua`, `diflist.lua`, `Root/customfunction.lua` 의 randomChoice 계열), 다국어(en/cn), 레인커버 로테이션(`Play/lua/require/listchoice` 관련), 3종 언어 파일, 사이드메뉴 상태 저장, 정보 게시판 출력(`infoOutput`), `luajava` 사용부(Result 의 `Gdx.input:isKeyPressed` 로 방향키 스크롤 정도).
- **빼면 안 되는 것**: 1920x1080 배경 구조(각 화면 bg), 플레이 레인/노트/판정/게이지/콤보/BPM/스코어 바, 선곡 곡 리스트 바와 메인프레임·옵션 패널(`mainframe/op/subop/assistop/sidemenu` 시트), 결과 화면의 게이지 그래프·판정 수·랭크 이미지, 결정 화면의 제목/아티스트/장르, 시스템 사운드, TTF 폰트(medium/black), 옵션 헤더(`property/filepath/offset/category`) 구조(사용자 커스터마이즈의 핵심), `main_state/skin_config/require/dofile/pcall` 런타임.
- 모듈 로드 시 파일 읽기(customnumber.lua:289-303)는 제거 대상: 기본판에서는 값이 필요 없는 화면이라면 해당 줄을 삭제해도 동작이 같다.

---

## 10. Sound 폴더와 `Root/sounds` 의 beatoraja 시스템 사운드 대응

BJ 의 `SystemSoundManager.SoundType`(BJ:SystemSoundManager.java:130-153)의 파일명과 `AudioDriver.getPaths`(BJ:audio/AudioDriver.java:178-201)는 **확장자를 `.wav → .flac → .ogg → .mp3` 순으로 바꿔 탐색**한다. 사운드 세트로 인식되려면 해당 디렉터리에 `clear.*` 가 있어야 하고(`scan(…, "clear.wav")` 라인 45-49), BGM 세트는 `select.*` 가 있어야 한다.

| MC Sound/ 파일 | 크기 | SoundType(파일명) | 호출 | 비고 |
|---|---|---|---|---|
| clear.ogg | 251,852 | RESULT_CLEAR(clear.wav) | 결과 클리어 | 세트 감지 키 파일 |
| fail.ogg | 639,308 | RESULT_FAIL(fail.wav) | 결과 실패 | 639 KB, 가장 큼 |
| f-open.ogg | 14,416 | FOLDER_OPEN(f-open.wav) | 폴더 열기 | |
| f-close.ogg | 21,393 | FOLDER_CLOSE(f-close.wav) | 폴더 닫기 | |
| o-change.ogg | 19,075 | OPTION_CHANGE(o-change.wav) | 옵션 변경(가장 빈번: EventFactory 다수) | Root/sounds/favorite.ogg 와 동일 |
| o-open.ogg | 46,206 | OPTION_OPEN(o-open.wav) | 옵션 패널 열기 | Root/sounds/open.ogg, Select/sounds/open.ogg 와 동일 |
| o-close.ogg | 23,005 | OPTION_CLOSE(o-close.wav) | 옵션 패널 닫기 | Root/sounds/close.ogg, Select/sounds/close.ogg 와 동일 |
| playready.ogg | 46,529 | PLAY_READY(playready.wav) | 플레이 준비 | |
| playstop.ogg | 51,354 | PLAY_STOP(playstop.wav) | 플레이 중단 | |
| scratch.ogg | 13,962 | SCRATCH(scratch.wav) | 선곡 스크래치 이동 | |
| scratch2.ogg | 17,559 | **대응 없음** | — | Root/sounds/change.ogg 와 동일 md5 |
| f-close2.ogg | 26,729 | **대응 없음** | — | |
| screenshot.ogg | 20,834 | **대응 없음**(BJ 는 스크린샷 소리 재생 코드 없음: MainController.java:551-575) | — | |
| (없음) | — | RESULT_CLOSE, COURSE_CLEAR/FAIL/CLOSE, GUIDESE_PG/GR/GD/BD/PR/MS(guide-*.wav) | | 폴백: COURSE_* 없으면 RESULT_* 사용(CourseResult.java:159-189) |
| (없음) | — | BGM SELECT(select.wav), DECIDE(decide.wav) | | `Sound/使用させていただいた楽曲.txt` 는 select(雨の日), decide(明日またこの場所で…)를 크레딧하지만 **음원 파일이 번들에 없다** |
- `Sound/*.txt` 크레딧(출처, MC:Sound/使用させていただいた楽曲.txt:1-27): DOVA-SYNDROME(https://dova-s.jp/) 의 BGM/효과음. clear=Impressive_days(bgm/play12728), fail=Phantom_Apartment_2(bgm/play12442), f-close=システム音_2(se/play1003), f-close2=システム決定音_4(se/play806), f-open=システム決定音_8(se/play1023), o-change=システム決定音_9_3(se/play1026), o-close=STAR_2_(se/play1320), o-open=システム決定音_7(se/play961), playready=おしゃれなテロップ表示音(se/play974), playstop=力をためる(se/play245), scratch=セレクト音_3_3(se/play672), scratch2=セレクト音_4_3(se/play843), screenshot=フィルムカメラのシャッター音(se/play1190), select=雨の日(bgm/play10305), decide=明日またこの場所で…(bgm/play9823).
- **`Root/sounds/*` 는 시스템 사운드가 아니다.** 스킨 Lua 가 `main_state.audio_play(skin_config.get_path("Root/sounds/xxx.ogg"), vol)` 로 직접 재생하는 스킨 전용 효과음이다(MC:Root/customsound.lua). 파일: change(결과 메뉴/인프레 전환), click, enter, open, close(도움말/사이드메뉴), favorite(미사용), fullcombo(128 KB), section/section-win/section-lose(구간 점수), vo/<char>/{clear 10, course 3, rank 8, update 2, harf 1}=24개 ×2 캐릭터.
- 선곡 화면의 사이드메뉴 등 `Select/sounds/` 2개는 참조 0건(중복).

---

## 11. 비교: readme 가 설명하는 사용자 옵션 (화면별 전체)

### 11.1 결정(Decide) 화면 (MC:Decide/readme.txt)
- 메인: 画像フォント(비트맵 폰트) 사용 — 0.7.5 이하는 불가라 "사용 안 함" 권장; ステージファイル(스테이지 이미지 표시); ノーツ分布グラフ(하단 노트 분포).
- 배경 패턴: 背景の種類(정지화 .png / 동영상 .mp4), 背景(静止画), 背景(動画).
- (실제 property 에는 추가로) ステージ数表示, お役立ち情報表示가 있다(Decide/lua/require/property.lua:87-91).

### 11.2 결과(Result) 화면 (MC:Result/readme.txt)
- 메인: 画像フォント, グラフ＆スコア表示位置, 項目表示切り替え(최대 콤보/미스 수), 開始時アニメーション, タイムスタンプ(현재 시각·플레이어명), 修飾(ビーム), 修飾(リング), クリアランクイメージ, ゲージ(이미지).
- 배경 패턴: 背景表示パターン(클리어 여부/전체/랭크별/클리어 상황), 背景の明るさ(255 = 검정).
- 배경 선택 4종(Clear or Failed / ALL / RANK / ClearType), キャラクター表示(겹침 그림) 및 패턴·위치 조정·선택 4종.
- IRメニュー(IR 연결 시 IRランキング/TOP10, 카메라 아이콘으로 이름 숨김), ステージファイル表示, グラフ関連(INFO 클릭: タイミンググラフ倍率 ±225/150/75ms, 配色パターン 4종), バージョンチェック.

### 11.3 선곡(Select) 화면 (MC:Select/readme.txt)
- 메인: 言語(일/영/중), 画像フォント, ステージ＆バナーファイル(둘 다/이미지만 확대), 曲リストの並び(3종), サブタイトルのスクロール, ビーム(装飾), 開始パターン, IR情報表示(클리어레이트&풀콤보레이트 / IRTOP10), サイドメニューの開閉状態を保持, スキン更新チェック.
- 배경: 背景の種類(정지화/동영상), 背景(静止画 Select/bg/image/*.png), 背景(動画 Select/bg/movie/*.mp4), 背景の明るさ.

### 11.4 플레이(Play) 화면 (MC:Play/readme.txt)
- メインオプション: プレイサイド(좌/우 스크래치), プレイ位置, 画像フォント, スコアグラフ/ノート分布/タイミングエリア 배치, 終了時にレーンカバーを下ろす.
- 背景: 背景, 背景の明るさ. グラフエリア: 背景, 伸びる向き, 隠し, 背景明るさ.
- BGA: 表示パターン, 汎用BGAの種類(動画/画像/なし), 汎用BGA(動画/画像), BGAの明るさ.
- ターゲットと判定タイミング: ターゲット差分 表示/種類/位置, 判定タイミング 表示/種類(FAST/SLOW か ms)/位置.
- キービーム: 有無, 高さ(10단계), 消失時間, 消失パターン.
- ボム: 種類(ModernChic 規格 / OADX 規格), 判定タイミングボム, ボム(各規格), 大きさ(1-100%).
- 判定タイミンググラフ: パターン, 隠し, 倍率(±225/150/75ms), 配色パターン 4종.
- パーツ選択 13종(ノーツ, 判定文字, レーンカバー, リフト, フルコンボエフェクト, キーイメージ, キーフラッシュ, 判定ライン色, グローランプ, プログレスランプ, ゲージMAXインジケータランプ, ゲージ, スクラッチイメージ).
- パーツ表示有無: グローランプ, ゲージMAXインジケータ, ゲージ隠し, ノート分布隠し, オートプレイ＆リプレイ時の案内, 戦闘モード.
- オフセット: レーンの明るさ, 小節線の明るさ, 判定ラインの高さ, グローランプの高さ. Other: Notes offset, Judge offset.

### 11.5 KeyConfig / SkinSelect
readme 없음(두 폴더에 readme 가 없고 옵션도 없음).

---

## 12. 라이선스와 제3자 자산

### 12.1 원문 요지 (4개 readme 공통, 일본어 + 영어)
- 일본어: 「スキンに含まれるデータについてはどのように使用して頂いても構いませんが、許可した場合を除き、本スキンそのものを二次配布することは禁止」, 「改変したスキンを公開する場合はreadme等で原作者名（KASAKO）の記述」, 「改変スキンを使い何か問題が発生した場合はその責任は負いかねます」.
- 영어: "It doesn't matter how you use the data contained in the skin. Secondary distribution of this skin itself is prohibited except when permitted. If you want to publish the modified skin, please describe the original author name (KASAKO) in the readme etc. … we are not responsible for any problems caused by using modified skins."
- Result readme 주석 1: `Result/parts/rank` 내 `#default`, `Damage`, `Formal`, `Underworld` 는 위 "자유 사용" 의 **예외**(제3자 랭크 글자).
- 해석: R-BMS 저장소에 MC 를 수정한 "기본 스킨"을 포함해 공개 배포하는 것은 **2차 배포에 해당할 수 있다**. 작가 허락 확인과 readme 크레딧(KASAKO), 제3자 자산 제외가 필요(openQuestions).

### 12.2 포함된 제3자 자산
| 자산 | 출처/조건 | 위치 |
|---|---|---|
| Mgen+ 1c 폰트 | 自家製フォント工房 / Adobe / M+ FONTS PROJECT, SIL OFL 1.1 (4개 폴더에 라이선스 전문 4,301 B 동일) | */font/ttf, fnt 페이지 |
| 랭크 글자 이미지 | ゆうたONE 님 (https://twitter.com/yuta_ptv) 폰트 사용 | Result/parts/rank (+ Result readme) |
| 음성 | VOICEVOX:ずんだもん, VOICEVOX:春日部つむぎ (https://voicevox.hiroshiba.jp/) — VOICEVOX 이용 약관(크레딧 표기 필요) | Root/sounds/vo |
| 立ち絵 | ずんだもん 立ち絵素材(seiga im10788496), 春日部つむぎ 立ち絵素材(seiga im10849150), 四国めたん 立ち絵素材(pixiv 92641379) | Root/image, SkinSelect/bg, Select/Play 관련 |
| 캐릭터 그림 | ごｎ 님 (결과 캐릭터, BPM 연동 캐릭터 작화), メスガキに分からせられたい 님 (mesugaki 소재) | Result/parts/char, Root/image |
| 시스템 사운드/BGM | DOVA-SYNDROME 14곡(위 10절 목록) — DOVA 이용 약관(미확인, 별도 확인 필요) | Sound/ |
| 번역 | Mr.Mary, AYhaz, marie, Zris 님 (Select 영/중 번역) | Select/parts/en,cn |
| 폭탄(bomb) 이미지 | 파일명에 "SCUROed" 포함(작가/출처 readme 에 없음: 미확인), STB 님/Rio 님 스페셜 땡스 | Play/parts/common/bomb |
| 배경/BGA 동영상 | 출처·조건 readme 에 기재 없음(미확인) | Play/Select/Decide 동영상 8개 |
| 선곡 배경 PNG(네온 복도) | 출처 기재 없음(미확인) | Select/bg/image |
- 스페셜 땡스(Play readme): STB, Rio, けえ, ごｎ, メスガキに分からせられたい.

---

## 13. 구현 시 주의/위험

1. keyconfig/skinselect 를 R-BMS 가 읽을 때 헤더만 쓴다면 위 8.x 의 Lua 호환 작업은 다른 화면에도 필요하다는 점이 핵심. 두 화면 때문에 추가로 필요한 것은 없다.
2. MC 가 `io.open(path, "w"/"a")` 로 **스킨 디렉터리에 쓴다**. R-BMS 가 스킨 디렉터리를 읽기 전용(번들 리소스)으로 둔다면 쓰기를 별도 사용자 데이터 디렉터리로 리다이렉트하거나 기록 기능을 제거해야 한다. 단순화판은 제거 권장.
3. `os.date` 가 모듈 로드 시 호출되고 `History/<yymmdd>/…` 경로를 만든다: 날짜 경계에서 값이 바뀜(기록 기능 제거 시 무관).
4. 와일드카드 소스 경로의 **무작위 선택**과 `Random` 선택 옵션: 같은 화면 진입마다 달라질 수 있다.
5. 텍스트 앵커/스케일 의미(0절 4번)와 마우스 좌표계(y-up, 목적 해상도 기준)를 틀리면 skinselect 수준의 단순 화면에서도 위치가 어긋난다.
6. `dofile` 에 절대 경로가 들어가므로 샌드박스는 "스킨 폴더 하위만 허용"이 필요하고, `require` 는 스킨 폴더 `?.lua` 로 매핑해야 한다. 기존 R-BMS 샌드박스는 `require/dofile/io` 를 제거(`mlua lua54`)한다.
7. 크기: 5190x2571, 3700x1800 텍스처는 GPU 한계 점검 필요(미확인).

## 14. 읽지 못한/표본으로만 본 항목
- 읽은 것: keyconfig.lua, KeyConfig/**, skinselect.lua, SkinSelect/**(Lua 전부, PNG 2장 열람, TTF 목록), Root/*.lua 중 define/define2/author/version/mainbutton/mainslider/mainimage/mainoffset/mainstring/customfunction/customsound/customtime/customtimer/customtext, customnumber/customoption/customgraph/customslider 의 로드 시점 호출부, config.lua, difflist.txt, History/*.txt, Sound/*.txt, 4개 readme, SIL 라이선스 헤더, 4개 화면의 textproperty.lua, 4개 화면 property.lua 의 filepath/기본값 grep, Decide/lua/require/property.lua 전체.
- **읽지 않음(다른 조사자 담당)**: Play/Select/Result/Decide 의 나머지 Lua 본체(Play 46, Select 25, Result 14 파일), Root/mainnumber.lua, mainoption.lua, maintimer.lua, maingraph.lua 의 전체 내용, Root/customoption.lua 와 customnumber.lua 본문(로드 시점 줄만 확인), Select/lua/require/http.lua·settings.lua 전체.
- PNG 시트는 SkinSelect 2장, Select/bg/image/#default.png, Select/font/fnt/main1.png 만 직접 열람. 나머지 PNG 는 크기·치수(sips)와 이름 기준.
- mp4 의 코덱/길이는 macOS `mdls` 값(ffprobe 없음). 프레임레이트·비트레이트 세부는 `kMDItemTotalBitRate`(kbps)만 확인.
- 도구/스크립트(읽기 전용 분석): `/private/tmp/claude-501/-Users-hyunseokbyun-development-R-BMS/41ddb2d1-97ab-47c9-943d-b6bc19d0d388/scratchpad/skin-research/tools/{assetref.py,bundles.py,pngpix.py}`.
