# ModernChic 선곡 화면 스킨 조사 보고서 (m4-select)

> 최종 갱신 2026-10-10 · 대응 단계: 웨이브 5(선곡) 반영 · 본문은 작성 시점 서술이며, 아래 "웨이브 … 반영 사항"이 최신 것부터 우선한다 · 색인과 갱신 규칙은 [README.md](README.md)

## 웨이브 5 반영 사항 (2026-10-10)

songlist 재작성, 선곡 상태 모델, 선곡 입력 키 표와 장면 수명, 패널 1~3 과 옵션 이벤트, 호스트 군집 F·H(선곡)·E, 슬라이더 쓰기와 편집 텍스트, 선곡 사운드, 스킨 위 시스템 오버레이를 넣은 뒤의 상태다.

- (W5-1) m4-select.md §8.3 상단 반영 사항: 'W5-1 캡처: 바 17칸에 제목(bar+130/+50)·레벨(bar+30, 가운데 정렬)·램프(bar-8)·트로피(bar+37)·라벨(bar-90, +525, +640)·폴더 분포 그래프(bar+40, 650x10)가 나옴. 상하 프레임에 가려져 슬롯 3~12 가 보임' 추가
- (W5-5) m4-select.md §9 '검색 입력' 행: '스킨에는 입력 이벤트가 없다, 표시만 한다'는 틀렸습니다. ModernChic 의 search text(Select/lua/require/textproperty.lua:40, ref=STRING.SEARCHWORD 만 있고 editable·event 없음)는 JsonSkinObjectLoader.createText 의 editable = text.editable || (text.event == null && writer != null) 규칙으로 자동 편집 가능하며 클릭 시 입력이 열린다. 00-synthesis.md S8 행(ModernChic 은 표시만)도 같이 고친다
- (W5-2) m4-select.md 상단: '웨이브 5 반영 사항(W5-2)' — 앱 경로 캡처에서 §5 의 바 좌표(중앙 x=1125 y=505, 그 외 x=1160), 텍스트 인덱스(신규 = 노랑, 폴더 x+50, 미보유 코스 회색), level·lamp·label(LN x-90, RANDOM x+525, MINE x+640)·graph(x+40, y+5, 650x10)와 §8.2 의 stagefile(80,406,480,360)·banner(700,785,300,80)가 일치함을 기록
- (W5-3a) m4-select.md §9 표의 '곡 목록', '선택 바 이동 효과', '옵션 패널 1/2/3' 행과 §6.3: R-BMS 상태 주석 추가 — 곡 바 클릭·휠·타이머 11 구현(W5-3a), 타이머 11 은 장면 첫 입력 프레임부터 켜져 커서를 움직이지 않아도 제목·스테이지파일이 표시됨(캡처 select-unmoved-3000ms), 패널은 START/SELECT 홀드 감지와 키 에지 소비까지만이고 동작·타이머는 W5-3b
- (W5-4) m4-select.md §6.1: 숫자 71·76·77·78·79·80~89·100~116·243~249·271·300·320~330·410~427·179~242 는 이제 군집 F·H(skin_host/select.rs, ir.rs)가 답함. 선택 막대의 ScoreData 는 BarManager 가 ScoreDataProperty.update(score, rival=null)로 넘기므로 최고 기록 EX 가 아니라 막대 점수의 파생값이며 라이벌 없음일 때 타깃 비율이 100%, 최고 기록·타깃 점수는 0 임을 적는다.
- (W5-4) m4-select.md §6.2: 옵션 1·2·3·5·21~23·100~105·1100~1104·197~198·1196~1204·1205~1208·200~207·220~227·300~307·320~327·340~347·352~354·603·604·606·608·624·625·1002~1017·1030·1031 값 연결. 저장됨(198·1198·1201·1204)은 선곡에서 '없음'과 같은 식(createReplayProperty type 2)임을 적는다. 패널 21~23 은 SelectShown.panel 이 Some 일 때만 답하며 Stage 연결은 W5-3a/3b.
- (W5-4) m4-select.md §6.4·§6.7: 이미지 인덱스 10~12(필터·정렬)·40·42·43·54·55·72·75·78·89·90·301~308·330~332·340·341·370·371 값 연결 상태와, R-BMS 에 원천이 없어 값 없음인 목록(303·305~307·321~324·342·343·350·351·360·361·400)을 추가.
- (W5-4) m4-select.md §6.5: 문자열 1·3·30·60~62·150~159·200~219·1000·1020~1021 연결. 30(검색어)은 원본 판독기가 항상 빈 문자열, 1000 은 'A > B > ' 형식, 150~159 는 곡이 라이브러리에 없으면 '(no song) ' 접두.
- (W5-4) m4-select.md §8.3·§8.4: 앱 경로 캡처로 확인된 것을 적는다. directory 텍스트 (990,715) 오른쪽 정렬, folder-totalsongs (870,498) 에 폴더 곡 수 0013, 폴더 클리어 현황표 (83,240,917x150) 11칸이 램프 분포와 일치, 코스 막대에서 단위 패널(op 3) 표시. §8.7 중앙 스코어창(EX/CB/MS x=1079, 카운트 x=569, 판정 막대, 랭크 문자, 점수율 (1170,160), fastcount (1069,82))이 계산값과 일치.
- (W5-4) m4-select.md §14: 'NUMBER 300 이 값 없음이어야 한다'(W2-9 메모)는 구현됨: 폴더가 아니면 300·320~330 은 값 없음, 폴더면 합계와 램프별 수.
- (W5-3b) m4-select.md §9 표: '옵션 패널 1/2/3' 행에 R-BMS 구현 상태(START = A, SELECT = W, 숫자 5, 패드 Start/Select)를, '버튼 열' 행에 AUTO = 1회 오토플레이, PRACTICE = Practice Stage, TEXT = 무동작, REPLAY 1~4 = 최근 리플레이, LN/KEY/SORT = 설정·필터 순환을 추가. '설정 패널' 행에 13 → 키 설정 Stage, 14 → 설정 SKIN 탭, 210 무동작, 321~324 무동작 기록
- (W5-7) m4-select.md §8.3·§8.5·§8.9 또는 §14: 오버레이 겹침 판단 기준(1280 좌표) 추가. 상단 프레임 높이 138, HELP 버튼 x 20~119 y 7~20, Ver 4.6 x 1227~1280 y 6~17, 우상단 버튼열 y 22~88, 하단 3창 y 584~713. 하단 좌측 x 16 y 572 위쪽이 빈 영역


## 웨이브 3A 반영 사항 (2026-10-10)

prepare/draw 2단계 파이프라인과 `SkinHost` 직접 그리기, 그리기 조건 의미론, 참조 이미지·음수 크기·이미지 인덱스·숫자·슬라이더·그래프 정합, TTF 텍스트, judgegraph·bpmgraph, Lua 함수 값 프레임 평가, 앱 호스트 군집 A·I·M 을 넣은 뒤의 상태다.

- (W3-1a) m4-select.md §8.3: W2-9 메모('곡 목록 바는 0개') 뒤에 'W3-1a 캡처: 바 이미지가 liston x=1125, h=70 위치에 나옴. 바 내부 요소는 W5-1' 추가
- (W3-1b) m4-select.md(선곡 객체 목록): 객체 "complete"(op {2, 602})는 602 가 원본 팩토리에 구현이 없는 상수라 원본에서도 로드 시 제거됨을 적는다. 선곡은 정적 범위가 50·51 뿐이라 정리로 빠지는 객체는 2개
- (W3-3) m4-select.md §14 #1(곡 정보 텍스트가 곡 바·부제와 겹치는지 렌더로 미확인): 캡처로 확인됨 — title 은 화면 y 439..491, x ..999 에 그려지고 170px 부제 워터마크가 그 뒤 띠 안을 지나감. 소스 좌표 그대로의 겹침


## 웨이브 2B 반영 사항 (2026-10-10)

스킨 덤프 CLI, 앱의 스킨 팩 폴더 지정과 `.luaskin` 로드, 오버레이 총 크기 상한, 외부 스킨 첫 정지 프레임을 넣은 뒤의 상태다.

- (W2-9) m4-select.md §8.3: 'W2-9 캡처: selectmusic-frame 과 mainframe 요소는 좌표대로 나오지만 곡 목록 바는 0개. 원인은 `{id = "songlist"}` destination 에 dst 가 없어 R-BMS 가 해석하지 못하는 것(songlist.lua:245)' 추가
- (W2-9) m4-select.md §8.4: 'folder-totalsongs destination 에는 op 가 없다(musicdisplay.lua:86). 곡 바에서 보이지 않는 것은 NUMBER 300 이 값 없음(Integer.MIN_VALUE)이기 때문이므로, 호스트는 폴더 바가 아닐 때 300 을 0 이 아니라 값 없음으로 답해야 한다(0 을 주면 (870,498)에 0000 이 그려짐을 캡처로 확인)' 추가
- (W2-9) m4-select.md §2.1·§4.1·§4.3: 실측 일치 표시 — destination 1914, draw 함수가 걸린 destination 1384, 함수 타이머 420, 등록 함수 1817
- (W2-6) m4-select.md §2.1(객체 수 수동 전개)와 00-synthesis.md 위험 5: 선곡 본체 실측 destination 1914, 객체 image 1438·imageset 49·value 154·text 65·slider 5·graph 111·judgegraph 1·bpmgraph 1·songlist 1, 함수 값 1817(boolean 1351, integer 4, float 25, text 1, timer 420, event 16). boolean+integer+float+text = 1381 로 '클로저 약 1380 + 타이머 약 420' 과 일치.


조사 대상: `/Users/hyunseokbyun/Downloads/ModernChic` 의 선곡 화면(`musicselect.luaskin`). 참조 구현: `/Users/hyunseokbyun/development/beatoraja/src/bms/player/beatoraja/`(이하 `B:`). 이 문서의 경로는 별도 표기가 없으면 ModernChic 루트 기준이며, 줄번호는 원본 파일 기준이다.

표기 규칙: 좌표는 1920x1080, 원점 좌측 하단, y 는 위로 증가(destination). 이미지 소스(`src` 의 x,y)는 좌측 상단 원점. `id` 숫자는 원문 그대로. "기본 옵션"은 `Select/lua/require/property.lua` 의 `def` 값 전부를 말한다(아래 1절).

---

## 0. 조사 범위와 한계

읽은 파일(전부): `musicselect.lua`, `musicselect.luaskin`, `config.lua`, `Select/lua/*.lua` 20개(assistoption, background, bmsanalysis, btnarea, cource, help, history, info, mainframe, musicdisplay, option, qco, rivalview, score, sidemenu, songlist, startanimation, suboption, versioncheck, volumecontrol), `Select/lua/require/*.lua` 5개(header, http, property, settings, textproperty), `Select/lua/settings/sidemenu`(`checkversion` 은 목록만 확인), `Root/*.lua` 22개 중 선곡 경로에서 로드되는 전부(define, define2, main*, custom*, version, author), `Select/readme.txt`, `io/Select`, `Select/bg|font|parts|sounds` 파일 목록. PNG 는 mainframe/songbar/scoreinfo/qco/sidemenu/op/help(jp)/#default.png 를 직접 열어 확인했고, 알파/색은 PNG 디코더(scratchpad/tools/pngsample.py)로 표본 추출했다.

beatoraja 쪽은 선곡 화면에 직접 영향을 주는 파일만 읽었다: `skin/lua/{LuaSkinLoader,SkinLuaAccessor,LegacySkinLuaApi,MainStatePropertyLuaApiExporter,TimerUtility}.java`, `skin/json/{JsonSkin,JSONSkinLoader,JsonSkinObjectLoader,JsonSelectSkinObjectLoader}.java`, `skin/{SkinObject,SkinLoader,SkinHeader,Skin,SkinNumber,SkinSourceImage}.java`(일부), `select/{SkinBar,BarRenderer,MusicSelectSkin,MusicSelector,MusicSelectInputProcessor}.java`(일부), `skin/property/{EventFactory,IntegerPropertyFactory,FloatPropertyFactory,BooleanPropertyFactory}.java`(일부), `skin/SkinProperty.java`.

한계(미확인 표시 대상):
- Lua 실행기가 없어(cargo 금지) 객체 개수는 소스를 수동 전개해 센 값이다. 정적 삽입 횟수는 grep 으로 교차검증했고 루프 곱셈은 손으로 계산했다. 오차가 있어도 ±수 개 수준이다(아래 3절 표).
- `Select/lua/require/http.lua`(175줄)는 앞 40줄만 읽었다. 이 파일은 `versioncheck.lua` 에서만 쓰이고 `versioncheck.lua` 호출부가 `musicselect.lua:216-227` 에서 블록 주석 처리되어 있어 죽은 코드다.
- Result/Decide/Play/SkinSelect/Course/KeyConfig 화면은 범위 밖이다. 단, `Root/*.lua` 공용 모듈은 선곡이 로드하는 범위에서 읽었다.
- 실제 화면 렌더(스크린샷)는 실행하지 않았다. 레이아웃은 Lua 좌표와 스프라이트 시트에서 재구성한 것이다. 텍스트와 요소가 겹치는 지점은 "소스 좌표 그대로"이며 시각 검증이 필요하다(15절).

---

## 1. 진입, 로딩 순서, 헤더 (요구 절 1)

### 1.1 진입 파일

`musicselect.luaskin:1-6`: `local t = require("musicselect")`; `skin_config` 가 있으면 `t.main()` 을, 없으면 `t.header` 를 반환한다. 즉 beatoraja 는 같은 파일을 두 번 실행한다(헤더 단계: `skin_config == nil`, 본 로드 단계: `skin_config` 존재). 근거: `B:skin/lua/LuaSkinLoader.java:50-61, 69-95`, `B:skin/lua/SkinLuaAccessor.java:925-960(exportSkinProperty)`.

`musicselect.lua` 최상단(`:8-11`)이 전역 `main_state = require("main_state")`, `timer_util = require("timer_util")`, `PROPERTY = require("Select.lua.require.property")` 를 대입하고 `header = require("Select.lua.require.header")` 를 로컬로 잡는다. 헤더 단계에서도 이 4개 require 는 성공해야 한다(beatoraja 는 헤더 단계에 `main_state`, `timer_util`, `event_util` 로 빈 테이블을 `package.loaded` 에 넣어 준다: `B:skin/lua/SkinLuaAccessor.java:95-101(initializeModules)`).

모듈 이름 -> 파일: `Select.lua.require.property` -> `Select/lua/require/property.lua` (점을 슬래시로, `package.path = <skinDir>/?.lua`). 최상위 `config` 는 `config.lua`.

### 1.2 `main()` 의 실행 순서 (`musicselect.lua:30-301`)

1. `MAIN = require("Root.define")`, `CUSTOM = require("Root.define2")`, `CONFIG = require("config")` 를 전역으로 대입(`:32-34`). `define2` 는 `Root.customfunction`(내부에서 `require("luajava")` 와 `luajava.bindClass("java.io.File")` 실행: `Root/customfunction.lua:7-8`), `customnumber`(로드 시점에 `io.open` 으로 파일 8개 읽기: `Root/customnumber.lua:289-303`), `customtime`(로드 시점 `os.date`: `Root/customtime.lua:6-10`), `customsound` 등을 연쇄 로드한다(`Root/define2.lua:29-37`).
2. `textProperty = require("Select.lua.require.textproperty").load(header.ver)` (`:36`). `header.ver` = `Root/version.lua` 의 `4.6`.
3. `skin = {}` 에 `CUSTOM.LOAD_HEADER(skin, header)` 로 헤더 필드 복사(`:38`, `Root/define2.lua:8-12`), `skin.source` = id 7/8/9 리터럴(`:39-43`) + `changeLang` 의 id 3,4,5,6,10,11(`:13-28`), 빈 `image/imageset/graph/slider/value/songlist/customTimers/judgegraph/bpmgraph/destination`(`:47-58`), `skin.font = textProperty.font`, `skin.text = textProperty.text`(`:52-53`).
4. `if CONFIG.infoOutput then CUSTOM.FUNC.infoOutput(5) end` (`:59`). `config.lua:222` 의 `m.infoOutput = true` 이므로 기본으로 실행된다. 이 호출은 `pcall` 밖이며 `History/information.txt` 를 `io.open(..., "w")` 로 쓴다(`Root/customfunction.lua:319-367`). `io.open` 이 nil 을 반환하거나 `io` 가 없으면 `f:write` 에서 에러가 나 스킨 로드 전체가 실패한다.
5. 섹션 로드는 모두 `pcall(function() return dofile(skin_config.get_path("Select/lua/X.lua")).load() end)` 형태이고 성공하면 반환 테이블의 `source/image/imageset/value/graph/slider/judgegraph/bpmgraph/songlist/destination` 을 `CUSTOM.ADD_ALL`(ipairs 로 `table.insert`) 로 이어붙인다. 실패하면 조용히 그 섹션이 사라진다(에러 로그 없음). 로드 순서가 곧 z 순서(destination 배열 순서)다.

| 순서 | 파일 | `musicselect.lua` 줄 | 추가하는 키 | 조건 |
|---|---|---|---|---|
| 1 | background.lua | 61-71 | source, image, destination | 항상 |
| 2 | songlist.lua | 73-86 | image, imageset, value, graph, songlist, destination | 항상 |
| 3 | mainframe.lua | 88-100 | source, image, slider, imageset, destination | 항상 |
| 4 | musicdisplay.lua | 102-112 | image, value, destination | 항상 |
| 5 | btnarea.lua | 114-123 | image, destination | 항상 |
| 6 | cource.lua | 125-134 | image, destination | 항상 |
| 7 | info.lua | 136-147 | image, value, graph, destination | 항상 |
| 8 | score.lua | 149-160 | image, value, graph, destination | 항상 |
| 9 | bmsanalysis.lua | 162-174 | image, judgegraph, bpmgraph, value, destination | 항상 |
| 10 | qco.lua | 176-187 | image, imageset, value, destination | 항상 |
| 11 | rivalview.lua | 189-199 | image, value, destination | 항상 |
| 12 | sidemenu.lua | 201-214 | image, imageset, value, slider, graph, destination | 항상 |
| - | versioncheck.lua | 216-227 | (주석 처리) | 죽은 코드 |
| 13 | history.lua | 229-239 | image, value, destination | `PROPERTY.isviewHistoryOn()` 일 때만 |
| 14 | help.lua | 241-250 | image, destination | 항상 |
| 15 | startanimation.lua | 252-262 | image, value, destination | 항상 |
| 16 | option.lua | 264-274 | image, imageset, destination | 항상 |
| 17 | assistoption.lua | 276-286 | image, imageset, destination | 항상 |
| 18 | suboption.lua | 288-299 | image, value, imageset, destination | 항상 |

`volumecontrol.lua`(71줄)는 어디서도 로드되지 않는다(죽은 파일). `history.lua` 는 기본값에서 로드되지 않는다.

### 1.3 최상위 헤더 필드 (`Select/lua/require/header.lua:7-21`)

| 필드 | 값 |
|---|---|
| `type` | `5` (MUSIC_SELECT) |
| `name` | `"ModernChicSelect-" .. 4.6` = `"ModernChicSelect-4.6"` |
| `w`, `h` | `1920`, `1080` |
| `fadeout` | `500` |
| `scene` | `3000` |
| `input` | `500` (이 시점에 TIMER_STARTINPUT(1) 가 켜진다: `B:select/MusicSelector.java:192-194`) |
| `property` | `PROPERTY.property` (15개) |
| `filepath` | `PROPERTY.filepath` (3개) |
| `offset` | `PROPERTY.offset` (1개) |
| `category` | `PROPERTY.category` (4개) |
| `author` | `"KASAKO"` |
| (추가) `ver` | `4.6` (헤더 테이블에 들어 있어 `LOAD_HEADER` 로 skin 에 복사되지만 로더가 무시하는 알 수 없는 필드) |

### 1.4 property 15개 (`property.lua:76-195`)

옵션 op 번호는 `chiled()` 호출 순서대로 900 에서 시작해 1씩 증가한다(`property.lua:9, 20-26, 43-49`). 카테고리 라벨은 `parent()/filepath()/offset()` 호출 순서의 1부터 증가 정수이며, `Property.category`/`Category.item` 이 `String` 필드라서 로더에서 "1","2",... 문자열로 바뀌어 일치 비교된다(`B:skin/json/JsonSkin.java:59-93`, `B:skin/lua/LuaSkinLoader.java:112 의 String 직렬화 = tojstring`, `B:skin/json/JSONSkinLoader.java:143-150`).

| # | name (원문) | 뜻 | def | item (name -> op) | category 라벨 | 줄 |
|---|---|---|---|---|---|---|
| 1 | `背景の種類` | 배경 종류 | `静止画` | `静止画`->900, `動画`->901 | 1 | 76-78, 133-136 |
| 2 | `画像フォント` | 이미지 폰트 | `無効` | `無効`->902, `有効（高負荷）`->903 | 2 | 79-81, 137-140 |
| 3 | `ステージ＆バナーファイル` | 스테이지/배너 | `両方表示` | `両方表示`->904, `ステージファイルをフルサイズ表示`->905 | 3 | 82-84, 141-144 |
| 4 | `曲リストの並び` | 곡 목록 배치 | `直線` | `直線`->906, `曲線`->907, `斜め`->908 | 4 | 85-88, 145-149 |
| 5 | `サブタイトルのスクロール` | 서브타이틀 스크롤 | `有効` | `無効`->909, `有効`->910 | 5 | 89-91, 150-153 |
| 6 | `ビーム（装飾）` | 빔 장식 | `有効` | `無効`->911, `有効`->912 | 6 | 92-94, 154-157 |
| 7 | `開始パターン` | 시작 연출 | `フェードイン` | `フェードイン`->913, `シャッター`->914 | 7 | 95-97, 158-161 |
| 8 | `IR情報表示` | IR 표시 | `クリアレート&フルコンボレート` | 그 이름->915, `ランキング`->916 | 8 | 98-100, 162-165 |
| 9 | `サイドメニューの開閉状態を保持する` | 사이드메뉴 상태 유지 | `無効` | `無効`->917, `有効`->918 | 9 | 101-103, 166-169 |
| 10 | `スキン更新チェック` | 스킨 업데이트 체크 | `有効` | `無効`->919, `有効`->920 | 10 | 104-106, 170-173 |
| 11 | `言語` | 언어 | `日本語` | `日本語`->921, `English`->922, `Chinese`->923 | 11 | 107-110, 174-178 |
| 12 | `プレイ履歴表示` | 플레이 이력 | `無効` | `無効`->924, `有効`->925 | 12 | 111-113, 179-182 |
| 13 | `キーマップ表示` | 키맵 표시 | `有効` | `無効`->926, `有効`->927 | 13 | 114-116, 183-186 |
| 14 | `背景ローテーション` | 배경 로테이션 | `無効` | `無効`->928, `有効（選択した背景は無視されます）`->929 | 14 | 117-119, 187-190 |
| 15 | `BPM連動キャラクター` | BPM 연동 캐릭터 | `有効` | `無効`->930, `有効`->931 | 15 | 120-122, 191-194 |

주의: 선곡 Lua 는 `skin_config.option[name]` 를 읽어 로드 시점에 분기한다(`property.lua:47`: `skin_config.option[pName] == num`). `destination.op` 에 900번대를 쓰는 곳은 선곡 스킨에 없다. 따라서 옵션 변경 = 스킨 재로드가 필요하다. `skin_config.option` 은 option `name`(일본어 문자열) 을 키로, 선택된 `op` 정수를 값으로 가진다(`B:skin/lua/SkinLuaAccessor.java:exportSkinPropertyToTable`). `スキン更新チェック`(#10)는 `versioncheck.lua` 가 죽어 있으므로 선곡에서는 효과가 없다.

### 1.5 filepath 3개 (`property.lua:124-126, 197-201`)

| name | path | def | category | 쓰이는 곳 |
|---|---|---|---|---|
| `背景（静止画）Select/bg/image/*.png` | `Select/bg/image/*.png` | `#default` | 16 | `background.lua:12` source id 1 |
| `背景（動画）Select/bg/movie/*.mp4` | `Select/bg/movie/*.mp4` | `BGmovie01` | 17 | `background.lua:19` source id 1 |
| `BPM連動キャラクター Root/image` | `Root/image/*.png` | `yuki` | 18 | `mainframe.lua:165` source id `"char"` |

`def` 해석: 설정 UI 가 파일 이름(확장자 유무, 대소문자 무시)과 비교한다(`B:config/SkinConfiguration.java:327-335`). 선택된 값은 확장자를 포함한 파일명으로 저장되고 `getPath` 가 `*` 를 그 이름으로 치환한다(`B:skin/SkinLoader.java:getPath`). 선택값이 없으면 `*` 와 일치하는 파일 중 무작위 1개가 선택된다.

### 1.6 offset 1개 (`property.lua:128-129, 203-206`)

`{name = "背景の明るさ 0~255 (255で真っ暗になります)", category = 19, id = 40, a = 0}`. 중요한 Lua 진리값 함정: `JsonSkin.Offset` 의 `x,y,w,h,r,a` 는 `boolean` 이고 Lua 로더가 `LuaValue::toboolean` 으로 변환하므로(`B:skin/lua/LuaSkinLoader.java:99`) `a = 0` 은 Lua 에서 truthy -> `a = true`(알파 오프셋만 활성). 구현체가 정수 0 을 false 로 해석하면 안 된다. 쓰임: `background.lua:37` BLACK 오버레이 `offsets = {40}`; 사용자 값이 알파에 `+a/255` 로 더해진다(`B:skin/SkinObject.java:480-495`, 오프셋 id 는 `1..199` 만 유효: `:828-845`).

### 1.7 category 4개 (`property.lua:212-241`)

| # | name | item(라벨) |
|---|---|---|
| 1 | `メインオプション` | 11(언어), 2(이미지폰트), 3(스테이지/배너), 4(곡 목록 배치), 5(서브타이틀), 13(키맵), 6(빔), 7(시작 연출), 8(IR 표시), 9(사이드메뉴 유지), 10(업데이트 체크) |
| 2 | `BPM連動キャラクター` | 15(캐릭터 on/off), 18(캐릭터 파일) |
| 3 | `プレイ履歴表示（ModernChicResultスキンの使用と『プレイ履歴の保存』を有効にする必要があります。）` | 12 |
| 4 | `背景` | 1(배경 종류), 16(정지화), 17(동영상), 14(로테이션), 19(밝기 오프셋) |

### 1.8 로더 의미 요약 (구현 필수)

- 필드 매핑은 `JsonSkin.*` 클래스의 필드 이름/타입 리플렉션이다. 알 수 없는 키는 무시. 타입: `int` 필드는 `toint()`(Lua 실수는 소수 버림), `String` 필드는 `tojstring()`(숫자 id/src/font 가 문자열이 된다), `boolean` 은 `toboolean()`. `B:skin/lua/LuaSkinLoader.java:97-208`.
- 함수/숫자/문자열을 받는 필드(`BooleanProperty/IntegerProperty/FloatProperty/StringProperty/TimerProperty/Event/FloatWriter/StringWriter`)는 Lua 함수 -> 매 프레임 호출, 숫자 -> 내장 id 조회, 문자열 -> 이름 조회 또는 Lua 식. `:118-141, 153-166`.
- 배열 필드는 `table.keys()` 순서로 변환된다(연속 정수 키 1..n 이면 삽입 순서).
- destination 한 줄마다 **새 SkinObject 를 생성**한다(`B:skin/json/JSONSkinLoader.java:313-336`). 같은 `id` 를 여러 번 destination 으로 쓰면 각각 독립 인스턴스(독립 `timer/act/cycle` 상태)다. 알 수 없는 id 의 destination 은 조용히 건너뛴다.
- id 조회 우선순위: image -> imageset -> value -> floatvalue -> text -> slider -> graph -> (gaugegraph, judgegraph, bpmgraph, ...) -> 선곡 전용 songlist (`B:skin/json/JsonSkinObjectLoader.java:loadSkinObject`, `JsonSelectSkinObjectLoader.java:31-37`). 선곡 스킨 내 종류 간 id 충돌은 없음(교차 검사 완료). 숫자 문자열 음수 id(`-100` 등)는 `new SkinImage(-id)` 로 참조 이미지가 된다(`JSONSkinLoader.java:316-321`).
- `sk.destination` 이 nil 이면 로드 실패(NPE)이므로 항상 배열이어야 한다.

---

## 2. 스킨 테이블 구성 (요구 절 2)

### 2.1 키별 개수 (기본 옵션, 수동 전개 값)

| 키 | 개수 | 비고 |
|---|---|---|
| `source` | 11 | id 1(배경), 3,4,5,6,7,8,9,10,11, `"char"`. 옵션에 따라 `char` 제외 시 10 |
| `font` | 2 (이미지폰트 ON 시 3) | id 0/1 ttf (`textproperty.lua:28-31`), ON: id 0/1/2 fnt (`:77-81`) |
| `text` | 65 | 14 리터럴 + course1..10 + irRankName1..10 + s_irRankName1..10 + s_rival + f_rival1..10 + b_rival1..10 |
| `image` | 1438 (char 제외 436) | 아래 파일별 표 |
| `imageset` | 49 | songlist 1, mainframe 1, qco 10, sidemenu 10, option 10, assist 7, suboption 10 |
| `value` | 154 | 파일별 표 |
| `graph` | 111 | songlist 1, info 2, score 17, sidemenu 91 |
| `slider` | 5 | mainframe 1(scroll-lamp), sidemenu 4 |
| `judgegraph` | 1 | `{id="notes-graph", noGap=0, type=0}` (`bmsanalysis.lua:15-17`) |
| `bpmgraph` | 1 | `{id="bpmgraph"}` (`:19-21`) |
| `songlist` | 1 | 5절 |
| `customTimers` | 0 (`{}`) | `musicselect.lua:55` |
| `destination` | 1914 (char 제외 912) | 파일별 표 |
| `floatvalue`, `gaugegraph`, `hiterrorvisualizer`, `timingvisualizer`, `timingdistributiongraph`, `note`, `gauge`, `skinSelect`, `customEvents` 등 | 0 | 선곡에서 미사용 |

### 2.2 파일별 개수 (기본 옵션)

| 파일 | image | imageset | value | graph | slider | 기타 | destination |
|---|---|---|---|---|---|---|---|
| background | 2 | - | - | - | - | source 1 | 6 |
| songlist | 29 | 1 | 6 | 1 | - | songlist 1 | 4 |
| mainframe | 1022 (비-char 22) | 1 | - | - | 1 | source 1 | 1028 (비-char 26) |
| musicdisplay | 21 | - | 14 | - | - | - | 52 |
| btnarea | 33 | - | - | - | - | - | 36 |
| cource | 17 | - | - | - | - | - | 33 |
| info | 14 | - | 16 | 2 | - | - | 30 |
| score | 24 | - | 12 | 17 | - | - | 54 |
| bmsanalysis | 1 | - | 7 | - | - | judgegraph 1, bpmgraph 1 | 11 |
| qco | 29 | 10 | 27 | - | - | - | 18 (ranking 모드 74) |
| rivalview | 22 | - | 12 | - | - | - | 57 |
| sidemenu | 78 | 10 | 56 | 91 | 4 | - | 418 |
| history (기본 OFF) | 4 | - | 7 | - | - | - | 13 |
| help | 2 | - | - | - | - | - | 3 |
| startanimation | 5 | - | 1 | - | - | - | 5 (이력 ON +2) |
| option | 86 | 10 | - | - | - | - | 73 |
| assistoption | 20 | 7 | - | - | - | - | 36 |
| suboption | 33 | 10 | 3 | - | - | - | 50 |
| textproperty | - | - | - | - | - | font 2, text 65 | - |

옵션별 증감: char 애니 OFF(-1002 image, -1002 dest, -1 source), 키맵 OFF(dest -1), 빔 OFF(image -1, dest -4), 서브타이틀 스크롤 OFF(dest -2), 스테이지 풀사이즈(dest -1), 배경이 동영상이면 시작연출의 배경 줌 3개 제외(dest -3), IR 랭킹 모드(dest +56), 플레이 이력 ON(image +4, value +7, dest +15), 시작 패턴 셔터(개수 동일, 대상만 교체).

### 2.3 대표 예

- source: `{id = 5, path = "Select/parts/jp/mainframe.png"}` (3200x3200). `{id = "char", path = "Root/image/*.png"}`.
- font: `{id = 0, path = "Select/font/ttf/mgenplus-1c-black.ttf"}`.
- text: `{id="title", font=0, size=70, ref=10, overflow=1, align=2, shadowOffsetX=4, shadowOffsetY=4}` (`textproperty.lua:33`).
- image: `{id="lamp-failed", src=7, x=41, y=660, w=82, h=84, divx=2, cycle=50}` (`songlist.lua:204`); `{id="btn-keymode", src=5, x=416, y=1470, w=104, h=730, divy=8, len=8, ref=11, act=11}` (`btnarea.lua:26`).
- imageset: `{id="bar", images={"bar-song","bar-folder","bar-table","bar-grade","bar-nosong","bar-command","bar-search"}}` (`songlist.lua:227`).
- value: `{id="exscore", src=8, x=900, y=700, w=253, h=30, divx=11, digit=4, ref=71}` (`score.lua:184`).
- graph: `{id="bar-exscore-hard", src=8, x=0, y=..., w=690, h=33, value=147, angle=0}` (`score.lua:72`); 분포 그래프 `{id="graph-lamp", src=7, x=1010, y=90, w=11, h=60, divx=11, divy=6, cycle=100, type=-1}` (`songlist.lua:239`).
- slider: `{id="scroll-lamp", src=7, x=1010, y=0, w=35, h=45, type=1, range=600, angle=2, changeable=true}` (`mainframe.lua:22`).
- destination: `{id="main-top", dst={{x=0, y=874, w=1920, h=207}}}` (`mainframe.lua:92`).

### 2.4 선곡 전용/비표준 객체

`songlist`(5절), `judgegraph`(노트 분포 히스토그램, 색은 스킨이 아닌 엔진 고정 팔레트), `bpmgraph`(BPM 추이), 분포 그래프(`graph.type = -1`, 곡 목록 바 안의 램프 분포), 참조 이미지(`id` 가 음수 숫자 문자열: STAGEFILE -100, BANNER -102, BLACK -110, WHITE -111; 선곡 destination 이 쓰는 것은 `-100, -102, -110, -111` 4종, `-101` BACKBMP 와 `-105` 는 선곡에서 미사용: `Root/mainimage.lua:9-14`).

---

## 3. 객체 기능 사용 목록 (요구 절 3)

### 3.1 destination 필드 (`JsonSkin.Destination`, `B:skin/json/JsonSkin.java:453-481`)

| 필드 | 선곡에서의 사용 | 대표 위치 |
|---|---|---|
| `id` | 문자열, 숫자 음수(참조 이미지), `"bar"` 등 | 전체 |
| `dst` 키프레임 `time,x,y,w,h,a,r,g,b,acc` | 전부 사용. `angle`, `clip_*` 는 미사용 | - |
| `timer` (숫자) | `11`(SONGBAR_CHANGE), `21/22/23`(PANELn_ON), `31/32/33`(PANELn_OFF) | `musicdisplay.lua:22-34`, `option.lua:20-83`, `cource.lua:115`, `score.lua:51,78,129` |
| `timer` (Lua 함수) | `timer_util.timer_observe_boolean(...)` 만 | `sidemenu.lua`(417회 전개), `help.lua:24,30`, `mainframe.lua:198` |
| `loop` | `0`(전체 반복), 양수(해당 시각부터 마지막 키프레임까지 반복, 마지막==loop 이면 정지), `-1`(1회 재생 후 사라짐) | `musicdisplay.lua:178-263`(-1), `startanimation.lua`(-1), `songlist.lua:46`, `help.lua:30`(-1) |
| `blend` | `2`(ADDITION)만 23곳 | `mainframe.lua:86,250`, `info.lua:177,186`, `score.lua:20,25`, `sidemenu.lua:418`, `option.lua`, `assistoption.lua`, `suboption.lua:151,156,161`, `songlist.lua:250,257` |
| `filter` | `0`(OFF)만 4곳 | `mainframe.lua:107,171,182,198` |
| `op` | 양수/음수/복수. 예: `{-2}`, `{2, 5}`, `{21, -162, 2}`(`option.lua:282`), `{3, 1004}`, `{184, 2}` | 6절 id 표 |
| `draw` | Lua 함수 | 4절 |
| `offset` / `offsets` | `offsets={40}` 1곳 | `background.lua:37` |
| `mouseRect` | `{x=0,y=0,w=<dst w>,h=<dst h>}` 형태만 | `btnarea.lua`(17곳), `sidemenu.lua:242,258,274,290,861,905,955`, `suboption.lua:280` |
| `acc` | `2`(DECELERATE)만 35곳 | `songlist.lua:46,..`, `musicdisplay.lua`, `startanimation.lua:54,61`, `sidemenu.lua` |
| `center`, `stretch`, `angle`, `clip` | 미사용(destination 레벨). `songlist.center` 만 존재 | - |

키프레임 기본값(로더가 채움): 첫 키프레임의 누락 필드는 `time=0, x=y=w=h=0, acc=0, angle=0, a=r=g=b=255`, 이후 키프레임은 이전 값을 상속하며 `time` 도 상속한다(`B:skin/json/JSONSkinLoader.java:422-453`). 스킨은 이 상속에 의존한다(예: `{time=1000}` 만 있는 키프레임).

### 3.2 destination 시맨틱(구현 필수, `B:skin/SkinObject.java`)

- `loop`/`timer`/`blend`/`filter`/`center` 는 **객체당 첫 키프레임 호출에서만** 채택된다(`:218-241`, 이미 0이 아니면 유지). `acc` 도 객체 단위이며 키프레임 중 처음 나온 0이 아닌 값이 전 구간에 적용된다(`:218-220`, 보간 `:545-575`: acc 1=제곱, 2=`1-(r-1)^2`, 3=보간 없이 계단).
- 시간 계산 `prepareRegion`(`:348-386`): `timer` 가 있으면 꺼져 있을 때 그리지 않고, 켜졌으면 `time -= timerStart`. 그 뒤 `loop==-1` 이면 `time > 마지막시각` 일 때 숨김(1회 재생). 그 외 `마지막시각 > 0 && time > loop` 이면 `마지막==loop` 일 때 `time=loop` 로 정지, 아니면 `(time-loop) % (마지막-loop) + loop` 로 반복. 그 후 `첫키프레임시각 > time` 이면 숨김. 키프레임이 1개뿐이고 시각이 0 이면 항상 표시.
- 스킨 곳곳의 "`loop = 200` + 단일 키프레임 `time = 200`" 패턴은 "타이머가 켜진 뒤 200ms 에 나타나 유지"를 뜻한다(사이드메뉴/옵션 패널 내용물).
- 그리기 조건 `op`(양수=켜짐, 음수=꺼짐)는 `BooleanPropertyFactory` 의 선곡용 옵션 id 를 쓰고, 정적으로 false 인 객체는 로드 시 제거되며 900번대 커스텀 옵션은 `skin.option` 맵으로 평가한다(`B:skin/Skin.java:199-232`). 선곡 스킨은 900번대 op 를 쓰지 않는다.
- `mouseRect`: 매 prepare 에서 `mouseRect.contains(mouseX - region.x, mouseY - region.y)` 가 아니면 그리지 않는다(`SkinObject.java:603`). 마우스 좌표는 스킨 좌표계(원점 좌하단)다. 선곡의 "...-rect" 이미지들은 호버 하이라이트다.
- 클릭: `Skin.mousePressed` 는 `objects` 를 **역순(맨 위 z 부터)** 순회하며 `draw==true` 인 객체의 `mousePressed` 를 호출하고 처리되면 중단한다(`Skin.java:394-410`). 이벤트 인자는 버튼 0(좌)=+1, 1(우)=-1, 2=+1, 3=+1, 4=-1(`SkinObject.java:671-703`). 클릭 영역은 현재 `region`(destination 보간 결과)이다.
- 오프셋: `off.x - off.w/2`, `off.y - off.h/2` 를 위치에 더하고 w,h 는 더하며, 알파는 `+off.a/255` 후 [0,1] 클램프(`:393-450, 480-495`).

### 3.3 오브젝트 필드 사용 현황

| 종류 | 사용 필드 | 미사용 필드 |
|---|---|---|
| source | `id`(숫자/문자열), `path`(와일드카드 `*` 포함) | - |
| font | `id`, `path`, `type`(bitmap 시 1) | `fallback` |
| text | `id, font, size, ref, value(함수), constantText, overflow(1), align(0/1/2), outlineColor, outlineWidth, shadowOffsetX, shadowOffsetY` | `wrapping, editable, event, shadowColor, shadowSmoothness` |
| image | `id, src, x, y, w, h, divx, divy, cycle, timer(숫자: 11), len, ref, act` | `click` |
| imageset | `id, images, ref, act` | `value, click` |
| value | `id, src, x, y, w, h, divx, divy(ir_eliteRank만), cycle(ir_eliteRank만), digit, align, ref, value(함수)` | `padding, zeropadding, space, offset, timer` |
| slider | `id, src, x, y, w, h, type, range, angle, changeable` | `value, event, isRefNum, min, max, divx/divy, cycle, timer` |
| graph | `id, src, x, y, w, h, divx, divy, cycle, type, angle, value(숫자/함수)` | `isRefNum, min, max, timer` |
| judgegraph | `id, noGap, type` | `delay, backTexOff, orderReverse, noGapX` (기본값 사용: delay 500) |
| bpmgraph | `id` | 나머지 전부 기본값 |
| songlist | 5절 전체 | - |

`textproperty.lua` 의 `shadowOffsetX/Y`: 제목 `4,4`, 나머지 `2,2`(ttf 모드); bitmap 모드는 `outlineColor="111111ff", outlineWidth=0.8`(제목/버전/sepa/repositoryname 등) 또는 `"222222ff", 1`(나머지).

세부 규약:

- **번호(value)**: `divx` 열 수가 10 이면 숫자 0-9 만, 11 이면 11번째 칸이 "선행 0 자리 표시 글리프"다. 11열 시트는 `zeropadding=2` 로 취급되어 `digit` 칸이 모두 채워지고(선행 칸은 11번째 글리프) `align` 이 위치에 영향을 주지 않는다. 10열 시트는 선행 0 이 공백이고 `align` 0=오른쪽, 1=왼쪽, 2=가운데로 `digit` 칸 안에서 정렬된다. `value == Integer.MIN_VALUE`(=-2147483648) 이면 그리지 않는다. `B:skin/json/JsonSkinObjectLoader.java:140-150`, `B:skin/SkinNumber.java:139-193`. 목적지 사각형은 한 칸의 크기이고 칸 간격은 `w`(+space 0).
- **이미지 프레임**: `idx = floor(time * 프레임수 / cycle) % 프레임수`, `cycle==0` 이면 0, 타이머가 지정돼 있으면 `time -= timer`, 타이머 꺼짐이면 0(`B:skin/SkinSourceImage.java:72-88`). `timer` 없이 `cycle` 만 있으면 스킨 전역 시간을 쓴다.
- **이미지 `len`/`ref`**: `len>1` 이면 `divx*divy` 칸을 `len` 개 그룹으로 나눠 `ref` 가 가리키는 "이미지 인덱스 속성" 값으로 그룹을 고른다. 범위를 벗어나면 그리지 않는다(`JsonSkinObjectLoader.java:46-61`, `SkinImage.java:124-138`). imageset 도 같은 방식(`ref` 값 = 이미지 번호).
- 이미지에 `act` 가 있으면 destination 영역이 클릭 영역이 된다(`JsonSkinObjectLoader.java:66-70`).
- 그래프 `angle`: 0 = 오른쪽으로 길어짐(`G_ANGLE.RIGHT`), 1 = 아래(기본값이 1 이므로 `angle` 를 안 쓴 그래프는 아래 방향). 선곡 스킨의 모든 그래프는 `angle=0` 을 명시한다. `bar-fastRate` 는 dst `w=-180` 로 좌우 반전(`score.lua:26`). 슬라이더 `angle`: 0=위,1=오른쪽,2=아래,3=왼쪽, `range` 는 해당 방향 이동 픽셀.
- 분포 그래프(`type=-1`): 시트 `divx=11, divy=6` -> 11(램프 종류) x 6(프레임) 로 재배열, `cycle=100`(`JsonSkinObjectLoader.java:420-432`).

---

## 4. Lua 함수가 값으로 들어가는 자리 전수 (요구 절 4)

`B:skin/lua/SkinLuaAccessor.java` 의 `load*Property/loadEvent` 는 함수를 매 프레임(prepare) 호출한다. 결과 변환: 불리언은 `toboolean`, 정수는 `toint`(소수 버림), 실수는 `tofloat`, 문자열은 `tojstring`, 타이머는 `tolong`(마이크로초 시작 시각, 꺼짐=`Long.MIN_VALUE`). 예외 발생 시 경고 후 false/0/""/타이머 꺼짐. 이벤트는 `function.narg()` 로 인자 수(0,1,2)를 판별한다.

선곡에서 호출되는 `main_state` API 는 `option, number, text, event_index, time, timer, timer_off_value, volume_sys/key/bg, set_volume_sys/key/bg, audio_play`(+ `float_number, gauge, gauge_type, judge, rate, exscore` 는 play/result 쪽 함수에만 있어 선곡 경로에서 호출되지 않음)이다.

### 4.1 draw = function (정적 사이트 62곳, 전개 시 약 1380 클로저)

| 파일:줄 | 조건 식 | 읽는 API | 전개 수 |
|---|---|---|---|
| `mainframe.lua:10-12` | `CUSTOM.OP.isAssistOn()` | `event_index(301..307)` 7회 | 1 |
| `mainframe.lua:182-184` | `i == number(92) and not isNeglect(30)` | `number(MAINBPM)`, `time()`, `timer(11)` | 1000 (i=1..1000) |
| `cource.lua:97-165` | `isCounseWithin5()` / `isCounseOver6()` | `option(3)`, `text(155)` | 19 |
| `musicdisplay.lua:135-137, 142-144` | `option(1) and number(folderStatusRef[i]) ~= 0` | option, number(320..330) | 22 |
| `info.lua:140-210` | `option(2)` 와 `calcTotal()±20` 대 `number(368)` | option, number | 7 |
| `qco.lua:140-355` | `(option(2) or option(3)) and option(51) [and ...]` | option(2,3,51,606), number(179,180,379+i,74), text 없음 | 18 (랭킹 모드 74) |
| `rivalview.lua:58-91, 105-113` | `option(2) and option(625) and achievementRate(...)`, `isYouWin/isRivalWin` | option, number(71,271) | 40 |
| `sidemenu.lua:356-375, 478-545, 663-737, 747, 910` | `option(op[i]) == true/false`, `number(379+i)` 구간, `isMyFrame(i)`, `option(2) or option(3)` | option, number(179,380-399,74), text(120-129) | 276 |

`isNeglect(sec)` = `(main_state.time() - main_state.timer(11)) / 1000000 > sec`(`Root/customoption.lua:99-101`). 타이머 11 은 선곡 시작 시 켜지고 곡 이동마다 재설정된다(`B:select/MusicSelector.java:196-198, 612`). 꺼져 있으면(`Long.MIN_VALUE`) 결과가 매우 커져 true.

### 4.2 value / text / graph value 함수

| 위치 | 반환 | 읽는 API |
|---|---|---|
| `sidemenu.lua:202-215` (value 3) | `volume_sys()*100` 등 | `volume_sys/key/bg` |
| `startanimation.lua:74-76` (value) | `todaySongUpdateCount + 1` (로드 시 파일 레코드 수) | 없음(로드 시 상수) |
| `history.lua:47,69,90,111` (value 4, 기본 OFF) | 로드 시 상수 | 없음 |
| `textproperty.lua:37,87` (text `fullArtist`) | `text(15)==""` 이면 `text(14)`, 아니면 `text(14).." "..text(15)` | `text(14,15)` |
| `info.lua:115-126` (graph `gr_totalLow`) | `(50 + (number(368) - calcTotal())/2)/100` 을 [0.01, 0.99] 로 클램프 | number(368,74) |
| `info.lua:129-135` (graph `gr_totalHigh`) | 위 값 무클램프 | 동일 |
| `score.lua:10-11` (graph `bar-slowRate/fastRate`) | `number(424)/(424+423)`, `number(423)/(...)` | number(423,424) |
| `sidemenu.lua:402-410` (graph 11) | `rate == -2147483648 ? 0 : rate/100` | number(203..) |
| `sidemenu.lua:630-639` (graph 80) | `score == -2147483648 ? 0 : score/(number(74)*2)` | number(380-389,74) |

주의: `score.lua:10-11` 은 둘 다 0 이면 `0/0 = NaN` 을 반환한다(미플레이 곡). 구현은 NaN 을 0 으로 처리해야 한다.

`calcTotal()`(`Root/customnumber.lua:158-171`): `n<1 -> 0`, `n<400 -> 200+n/5`, `n<600 -> 280+(n-400)/2.5`, 그 외 `360+(n-600)/5` (`n=number(74)`).

### 4.3 timer = function

모두 `timer_util.timer_observe_boolean(f)`: 반환 함수는 호출될 때 `f()` 가 true 가 되는 순간 현재 마이크로초를 기록하고 false 가 되면 `Long.MIN_VALUE` 로 돌아간다(`B:skin/lua/TimerUtility.java:observe_boolean`). 객체마다 자신의 클로저 인스턴스(관측 상태)를 가진다.

| 파일:줄 | 관측 식 | 부수효과 |
|---|---|---|
| `sidemenu.lua:235-296` (12) | `isLampMenuOpen` 등 로컬 변수 (또는 `not ...`) | 없음 |
| `sidemenu.lua:300-933` (약 405) | 메뉴 열림 로컬 변수, `isTimerOn(173) and isLampMenuOpen`, `irLoading(...)`, `irOffline(...)` | 없음(`isTimerOn` 은 `main_state.timer(173) ~= timer_off_value`) |
| `help.lua:24,30` | `isMenuOpen` / `not isMenuOpen` | 없음 |
| `mainframe.lua:198` | `isNeglect(30) and option(2)` | 없음 |

### 4.4 act = function (이벤트 인자 0개)

| 위치 | 동작 | 부수효과 |
|---|---|---|
| `sidemenu.lua:162-163,166-167,170-171,174-175` (8 이미지) | `rampMenuSwitch/rankingMenuSwitch/volumeMenuSwitch/settingMenuSwitch` | 로컬 불리언 4개 갱신, 보존 옵션 ON 이면 `Select/lua/settings/sidemenu` 4줄 파일 쓰기(`io.open(...,"w")`), `CUSTOM.SOUND.windowMotionSound` 로 `Root/sounds/open.ogg` 또는 `close.ogg` 재생 |
| `sidemenu.lua:101-106` (6 이미지) | `main_state.set_volume_sys/key/bg(0 또는 1)` | 설정 볼륨 변경 |
| `help.lua:17-18` (2 이미지) | `menuSwitch()` | `isMenuOpen` 반전, `audio_play(open.ogg/close.ogg)` |

`windowMotionSound(f1..f4)`(`Root/customsound.lua:68-74`): `if flg and f1==false and f2==false and f3==false and f4==false then close.ogg else open.ogg`. 여기서 `flg` 는 모듈 상단 테이블(`:11-19`)이라 항상 truthy 이므로 "네 메뉴가 모두 닫혔을 때 close, 그 외 open". `helpMotionSound(flg)` 는 매개변수 `flg` 가 테이블을 가려 false 일 때 close, true 일 때 open.

### 4.5 숫자/식 id 를 받는 곳에서 함수가 아닌 것

`customTimers = {}`, `customEvents` 없음. `act` 에 숫자 id 를 쓰는 곳은 6절 BUTTON 표 참조.

---

## 5. songlist 객체 전체 필드 (`Select/lua/songlist.lua`)

`createSonglistProperty()`(`:5-186`) 결과를 그대로 옮긴다. 이 객체는 destination `{id="songlist"}`(`:245`) 하나로 배치된다. 좌표는 모두 **바 한 칸 기준 상대 좌표**(lamp/label/text/level/trophy/graph)이거나 절대 좌표(liston/listoff)다.

```
songlist = {
  id = "songlist", center = 8, clickable = {6, 7, 8, 9, 10},
  liston  = 17개: { id = "bar", dst = {{ x = 1125, y = Y[i], w = 960, h = 70 }} }
  listoff = 17개: { id = "bar", loop = L[i],
                    dst = {{ time = S[i], x = 1920, y = Y[i], w = 960, h = 70, acc = 2 },
                           { time = L[i], x = X[i] }} }
  level   = 6개 : id "level-unknown|beginner|normal|hyper|another|insane", dst {{x=30, y=13, w=35, h=42}}
  label   = 5개
  text    = 11개
  lamp, playerlamp, rivallamp = 각 11개, trophy = 3개
  graph   = { id = "graph-lamp", dst = {{ x = 40, y = 5, w = 650, h = 10 }} }
}
```

배열 상수 (인덱스 0..16, `songlist.lua:23-36`):

| 이름 | 값 |
|---|---|
| `Y[i]` | 1065, 995, 925, 855, 785, 715, 645, 575, 505, 435, 365, 295, 225, 155, 85, 15, -55 (70씩 감소) |
| `S[i]` (listoff 첫 키프레임 time) | 1400, 1350, 1300, 1250, 1200, 1150, 1100, 1050, 1000, 1050, 1100, 1150, 1200, 1250, 1300, 1350, 1400 |
| `L[i]` (listoff `loop` 및 둘째 키프레임 time) | 1650, 1600, 1550, 1500, 1450, 1400, 1350, 1300, 1250, 1300, 1350, 1400, 1450, 1500, 1550, 1600, 1650 |
| `X[i]` 직선(기본) | 1160 x8, **1125**(인덱스 8), 1160 x8 |
| `X[i]` 곡선 | 1288, 1249, 1215, 1186, 1168, 1151, 1139, 1128, 1125, 1128, 1139, 1151, 1168, 1186, 1215, 1249, 1288 |
| `X[i]` 사선 | 1325 - 25*i (1325, 1300, ..., 925) |

- 바 17개, 중앙(center)은 인덱스 8(y=505). `liston[i]` 는 중앙 바만 그린다(`B:select/BarRenderer.java:124, 252` 가 `on = (i == center)` 로 on/off 이미지를 고름). `listoff[i]` 는 비중앙 바. 각 바는 높이 70, 폭 960 이며 오른쪽 끝(x+960)이 화면 밖(>1920)이다. 선곡 로더가 허용하는 최대 바 수는 60(`SkinBar.BAR_COUNT`), 이 스킨은 17개만 정의한다.
- listoff 키프레임: 장면 시작 후 `S[i]` ms 에 x=1920 에서 시작해 `L[i]`(=S+250ms) 에 `X[i]` 에 도달하고 이후 정지(마지막 time == loop). `acc=2`(DECELERATE)가 객체 전체에 적용된다. `S[i] > 현재시각` 구간은 숨김. 시간 기준은 스킨 전역 시간(타이머 없음)이며, 시작 연출(0~1000ms)이 끝난 직후 중앙 바부터 순서대로 안으로 미끄러져 들어온다. 곡 이동 중 바 사이 이동은 엔진(BarRenderer)이 인접 바 위치를 선형 보간한다(`BarRenderer.java:114-140`).
- 클릭 가능 바 인덱스 `{6,7,8,9,10}`: 좌클릭 = 해당 바 `select(bar)`, 그 외 버튼 = `manager.close()`(폴더 닫기)(`BarRenderer.java:80-103`).
- 바 종류 -> `bar` imageset 인덱스(`BarRenderer.java:146-167`, `SkinBar` 문서): 0 = 곡(존재), 1 = 폴더, 2 = 테이블/해시/실행 가능 바, 3 = 단위(코스) 곡 전부 보유, 4 = 곡/단위 미보유(nosong), 5 = 커맨드/컨테이너, 6 = 검색어. imageset 순서 = `bar-song, bar-folder, bar-table, bar-grade, bar-nosong, bar-command, bar-search`(`songlist.lua:227`). 이미지는 `src=7`(songbar.png) 의 `x=0, y=0|70|140|210|280|350|420, w=960, h=70`(`:196-202`). 알파 표본: 바 이미지 평균 알파 약 233(230 최소) -> 반투명.
- `text` 인덱스(11개, 모두 `id="bartext"`, 상대 좌표 `{x=posX[i], y=15, w=500, h=35, r,g,b}`; `songlist.lua:113-123`):

| idx | 의미(`BarRenderer.java:172-200`) | posX | r,g,b |
|---|---|---|---|
| 0 | 일반 | 130 | 255,255,255 |
| 1 | 신규(24시간 이내) | 130 | 255,255,0 |
| 2 | 곡 바(일반) | 130 | 255,255,255 |
| 3 | 곡 바(신규) | 130 | 255,255,0 |
| 4 | 폴더(일반) | 50 | 255,255,255 |
| 5 | 폴더(신규) | 50 | 255,255,0 |
| 6 | 테이블/해시 | 50 | 255,255,255 |
| 7 | 단위 곡 보유 | 130 | 255,255,255 |
| 8 | 곡/단위 미보유 | 130 | 200,200,200 |
| 9 | 커맨드/컨테이너 | 50 | 255,255,255 |
| 10 | 검색어 | 50 | 255,255,255 |

  폰트는 `bartext` 텍스트 정의(ttf 모드: font 1 medium, size 35, shadow 2/2; bitmap 모드: font 0 bartext.fnt, size 35). 정렬/overflow 미지정 = 왼쪽, overflow 기본값.
- `level` 배열: 곡의 난이도(0..6, 7 이상은 0)로 인덱스를 고르고 `song.getLevel()` 을 숫자로 그린다(`ref=96` 은 무시됨: `BarRenderer.java:339-352`, `JsonSelectSkinObjectLoader.java:180-203`). 숫자 시트는 `songbar.png` `x=400, y=702|744|786|828|870|912, w=420, h=42, divx=10, digit=2, align=2(가운데)`. 인덱스 순서는 unknown(0, y=912), beginner(1, y=702), normal(2), hyper(3), another(4), insane(5).
- `label` 5개(인덱스 0 LN, 1 RANDOM, 2 MINE(bomb), 3 CN, 4 HCN: `BarRenderer.java:360-410`): `label-ln {x=-90,y=5,w=85,h=54}`, `label-random {0: x=525,y=6,w=111,h=54,a=255; 1000: a=180; 2000: a=255}`, `label-bomb {0: x=640,y=6,w=90,h=54,a=255; 1000: a=180; 2000: a=255}`, `label-cn {x=-90,y=5,w=85,h=54}`, `label-hcn {x=-90,y=5,w=85,h=54}`. 소스: songbar.png `x=0|85|170|530|620, y=499` (`songlist.lua:214-218`). CN/HCN 정의가 없으면 LN 라벨로 대체한다. 라벨 알파 펄스는 2초 주기(loop 0).
- `lamp`(내 램프, 인덱스 = 클리어 타입 0..10: 0 noplay, 1 failed, 2 assist, 3 laassist, 4 easy, 5 normal, 6 hard, 7 exhard, 8 fullcombo, 9 perfect, 10 max): 0번 `{x=-8,y=-7,w=41,h=84}`, 1..10번 `{0: x=-8,y=-7,w=41,h=84; 1000: a=200; 2000: a=255}`(2초 펄스). 이름 배열 `{"failed","assist","laassist","easy","normal","hard","exhard","fullcombo","perfect","max"}`. 이미지(`songlist.lua:203-213`): noplay `(123,660,41,84)`, failed `(41,660,82,84, divx=2, cycle=50)`, assist `(0,1332)`, laassist `(0,1248)`, easy `(0,744)`, normal `(0,828)`, hard `(0,912)`, exhard `(0,996,82x84, divx=2, cycle=50)`, fullcombo `(0,1080,123x84, divx=3, cycle=50)`, perfect와 max 는 같은 이미지 `(0,1164,123x84, divx=3, cycle=50)`.
- `playerlamp`/`rivallamp`(라이벌 선택 시): 위와 같은 구조에서 높이만 절반 `{x=-8, y=30, w=41, h=42}`(내 램프), `{x=-8, y=-3, w=41, h=42}`(라이벌 램프). 라이벌가 선택된 경우(`select.getRival() != null`)에만 이 둘이, 아니면 `lamp` 가 그려진다(`BarRenderer.java:315-337`).
- `trophy`(인덱스 0 bronze, 1 silver, 2 gold): `{x=37, y=8, w=54, h=54}`, 이미지 `x=363|417|471, y=499, 54x54`. 단위 바의 trophy 이름(`bronzemedal/silvermedal/goldmedal`)과 매칭.
- `graph`(폴더 바 안의 클리어 램프 분포): 상대 `{x=40, y=5, w=650, h=10}`, 시트 `graph-lamp` `src=7 (1010,90,11x60, divx=11, divy=6, cycle=100, type=-1)`. 디렉터리 바에서만 그린다(`BarRenderer.java:269-281`).
- 선곡 프레임 오버레이(`songlist.lua:243-265`): `selectmusic-frame`(`src=7 (0,560,990x82)`) -> `(1112,499,990,82)` 정적; `selectmusicFrameTop`(`(0,560,17x82)`) 가산 블렌드, `loop=1000`, 키프레임 `{1000: x=1112,y=499,w=17,h=82,a=0}, {1200: a=255}, {2000}, {3000: a=0}, {4000}`; `selectmusicFrameBottom`(`(17,560,963x82)`) 가산 블렌드, `{1000: x=1129,w=0,a=0}, {1200: w=200,a=255}, {2000: w=963}, {3000: a=0}, {4000}`. 둘 다 t=1000 이전엔 숨김, 이후 3초 주기로 반복(loop=1000 에서 4000 구간).

글로벌 영향: `selectmusic-frame` 이 중앙 바(1125,505,960x70)보다 사방 13~15px 크다(1112,499 / 990x82).

---

## 6. 참조 속성 id 전수 (요구 절 5)

이름은 `Root/main*.lua`(beatoraja `SkinProperty` 와 값 일치 확인: `B:skin/SkinProperty.java`). "파일" 열은 선곡에서 쓰는 파일. 의미 열은 선곡 컨텍스트에서의 의미다.

### 6.1 숫자(value `ref`, `main_state.number`)

범위 스코프 주의: 같은 id 라도 **value 스코프**(`ref`, `number()`)와 **이미지 인덱스 스코프**(image/imageset `ref`, `event_index()`)의 의미가 다르다. 390..399 와 380..389 가 대표 사례다. 엔진은 `PropertyScope.VALUE` / `IMAGE_INDEX` 로 구분한다(`B:skin/property/IntegerPropertyFactory.java:985-1003`).

| id | 이름 | 의미 | 파일 |
|---|---|---|---|
| 33,34,35,36,37 | TOTALPERFECT..POOR | 플레이어 누적 PG/GR/GD/BD/PR | info |
| 333 | TOTALPLAYNOTES | 누적 노트 수 | info |
| 21,22,23 | TIME_YEAR/MONTH/DAY | 현재 일자 | history(기본 OFF) |
| 71 | SCORE | 선택 바의 최고 EX 점수 | score, rivalview |
| 74 | TOTALNOTES | 선택 곡 노트 수(`ref=74`, `number(74)`) | info, qco, sidemenu |
| 76 | MISSCOUNT | 미스 카운트 | score |
| 77 | PLAYCOUNT | 플레이 횟수 | info |
| 90,91,92 | MAXBPM, MINBPM, MAINBPM | BPM | musicdisplay, mainframe |
| 96 | PLAYLEVEL | 레벨(songlist level 의 ref, 실제는 엔진이 값을 직접 전달) | songlist |
| 102,103 | SCORE_RATE, _AFTERDOT | 점수율 정수/소수 | score |
| 110..114 | PERFECT..POOR | 선택 바 스코어의 판정 수 | score, rivalview |
| 12, 312, 313 | JUDGETIMING(부호 있는 24칸 시트, divx=12 divy=2), DURATION, DURATION_GREEN | 옵션 서브패널 수치 | suboption |
| 179,180,181 | IR_RANK, IR_TOTALPLAYER, IR_CLEARRATE | IR 순위/참가자/클리어율 | qco, info(181 은 value 정의만 있고 destination 없음) |
| 202..219, 222..225 | IR_PLAYER_NOPLAY(202)/_RATE(203), ASSIST(204)/205, LIGHTASSIST(206)/207, EXHARD(208)/209, FAILED(210)/211, EASY(212)/213, NORMAL(214)/215, HARD(216)/217, FULLCOMBO(218)/219, PERFECT(222)/223, MAX(224)/225 | IR 클리어 종류별 인원/비율 | sidemenu |
| 227,229 | IR_PLAYER_TOTAL_CLEAR_RATE, _TOTAL_FULLCOMBO_RATE | IR 전체 클리어율/풀콤율 | qco |
| 230..240 | IR_PLAYER_*_RATE_AFTERDOT (NOPLAY 230, ASSIST 231, LASSIST 232, EXHARD 233, FAILED 234, EASY 235, NORMAL 236, HARD 237, FULLCOMBO 238, PERFECT 239, MAX 240) | 비율 소수 1자리 | sidemenu |
| 241,242 | IR_PLAYER_TOTAL_CLEAR_RATE_AFTERDOT, _FULLCOMBO_ | 소수 | qco |
| 220 | IR_UPDATE_WAITING_TIME | IR 갱신 대기 초 | info |
| 271 | RIVAL_SCORE | 라이벌 EX 점수 | rivalview |
| 280..284 | RIVAL_PERFECT..POOR | 라이벌 판정 수 | rivalview |
| 300 | FOLDER_TOTALSONGS | 폴더 내 곡 수 | musicdisplay |
| 320..330 | FOLDER_NOPLAY(320), FAILED, ASSIST, LASSIST, EASY, GROOOVE(325 노멀), HARD, EXHARD, FULLCOMBO, PERFECT, MAX(330) | 폴더 내 클리어별 곡 수 | musicdisplay |
| 350,351,352,353 | TOTALNOTE_NORMAL, _LN, _SCRATCH, _BSS | 노트 종류별 수 | bmsanalysis |
| 360,362,364 | DENSITY_PEAK, _END, _AVERAGE | 밀도(정수부) | bmsanalysis |
| 368 | SONGGAUGE_TOTAL | TOTAL 값 | info |
| 380..389 | RANKINGn_EXSCORE | IR 순위 슬롯 n 의 EX 점수(value 스코프) | qco, sidemenu |
| 390..399 | `RANKINGn_INDEX`(value 스코프: 슬롯 n 의 순위 숫자). 같은 id 가 이미지 인덱스 스코프에선 `RANKINGn_CLEAR`(슬롯의 클리어 타입) | qco, sidemenu |
| 400 | JUDGERANK | 판정 레벨 숫자 | info |
| 423,424,425 | TOTALEARLY, TOTALLATE, COMBOBREAK | 타이밍/콤보 브레이크 | score |
| 1163,1164 | SONGLENGTH_MINUTE, _SECOND | 곡 길이 | info |

`main_state.number()` 에 직접 쓰이는 리터럴: `368, 180, 179, 74`, `379 + i`(RANKING_i_EXSCORE, i=1..10), `389 + i`(RANKING_i_INDEX).
`main_state.text()` 리터럴: `119 + i`(RANKINGi_NAME, i=1..10), `155`(COURSE6_TITLE).
볼륨 값은 숫자 id 가 아니라 `volume_sys/key/bg()` 함수로 읽는다(57/58/59 id 는 미사용).

부가: 순위 슬롯은 엔진이 `getRankingOffset()`(IR 목록 스크롤) 을 더해 계산한다(`IntegerPropertyFactory.java:876-907`).

### 6.2 옵션(op, `main_state.option`)

| id | 이름 | 의미 | 파일 |
|---|---|---|---|
| 1, 2, 3, 5 | FOLDERBAR, SONGBAR, GRADEBAR, PLAYABLEBAR | 선택 바 종류(폴더/곡/단위/플레이 가능) | 다수 |
| 21, 22, 23 | PANEL1, PANEL2, PANEL3 | 옵션 패널 열림 상태 | option, assistoption, suboption |
| 50, 51 | OFFLINE, ONLINE | IR 온라인 여부 | info, musicdisplay, qco, sidemenu |
| 100, 101, 102, 103, 104, 105 | SELECT_BAR_NOT_PLAYED, FAILED, EASY_CLEARED, NORMAL_CLEARED, HARD_CLEARED, FULL_COMBO_CLEARED | 선택 바 클리어 상태 | score, sidemenu |
| 1100, 1101, 1102, 1103, 1104 | ..ASSIST_EASY, LIGHT_ASSIST_EASY, EXHARD, PERFECT, MAX_CLEARED | 〃 | score, sidemenu |
| 160, 161, 162, 163, 164, 1160, 1161 | SONG7KEY, 5KEY, 14KEY, 10KEY, 9KEY, 24KEY, 24KEYDP | 선택 곡 모드 | mainframe(모드 표시), option(DP 전용 패널) |
| 170, 171 | NO_BGA, BGA | BGA 유무 | musicdisplay |
| 175 | TEXT | 텍스트 파일 유무 | btnarea |
| 176, 177 | NO_BPMCHANGE, BPMCHANGE | BPM 변화 | musicdisplay |
| 180..184 | JUDGE_VERYHARD(180)..VERYEASY(184) | 판정 레벨 | info(`op={184,183,182,181,180}`) |
| 197, 1197, 1200, 1203 | REPLAYDATA, 2, 3, 4 | 리플레이 슬롯 존재 | btnarea, sidemenu |
| 1205..1208 | SELECT_REPLAYDATA..4 | 현재 선택된 리플레이 슬롯 | btnarea, sidemenu |
| 200..207 | AAA_1P..F_1P | 선택 바 스코어의 랭크 | score |
| 352, 353 | WIN_1P, WIN_2P | 라이벌 비교 승패 | rivalview |
| 602, 606 | IR_LOADED, IR_WAITING | IR 로딩 상태 | info, qco, sidemenu |
| 625 | COMPARE_RIVAL | 라이벌 비교 모드 | rivalview |
| 1003..1005, 1006, 1007 | GRADEBAR_MIRROR, RANDOM, NOSPEED, NOGOOD, NOGREAT | 단위(코스) 제약 | cource(1006/1007 은 리터럴) |
| 1010..1014 | GRADEBAR_GAUGE_LR2, 5KEYS, 7KEYS, 9KEYS, 24KEYS | 단위 게이지 | cource |
| 1015, 1016, 1017 | GRADEBAR_LN, CN, HCN | 단위 LN 제약 | cource |
| 1030 | RANDOMSELECTBAR | 랜덤 선곡 바 | musicdisplay |

`main_state.option()` 으로도 `1,2,3,51,606,625,2(리터럴 option(2))` 가 호출된다.

### 6.3 타이머

| id | 이름 | 의미 |
|---|---|---|
| 11 | SONGBAR_CHANGE | 선택 바 이동 시 재설정(켜져 있음) |
| 21,22,23 | PANEL1/2/3_ON | 옵션 패널 열림 시 켜짐 |
| 31,32,33 | PANEL1/2/3_OFF | 옵션 패널 닫힘 시 켜짐 |
| 173 | IR_CONNECT_SUCCESS | IR 데이터 수신 완료(`isTimerOn(173)`) |
| (함수) | `timer_observe_boolean` | 위 사이드메뉴/도움말/char_wait |

패널 전이: 상태 p->q 일 때 p!=0 이면 `PANELp_OFF` 켜고 `PANELp_ON` 끄며, q!=0 이면 `PANELq_ON` 켜고 `PANELq_OFF` 끈다(`B:select/MusicSelector.java:553-564`). 입력: START 누름 -> 패널1, SELECT 누름 -> 패널2, 키보드 `5` 또는 START+SELECT -> 패널3, 아무것도 없으면 0(`B:select/MusicSelectInputProcessor.java:112-292`).

### 6.4 버튼/이벤트 id (`act`, 이미지 `ref`/`len`)

| id | 이름 | 쓰임 | 동작(beatoraja) |
|---|---|---|---|
| 11 | MODE | `btn-keymode` act+ref(8프레임), `btn-modeset` imageset ref | 키 모드 필터 순환(클릭 인자 ±1) |
| 12 | SORT | `btn-sortmode` act+ref(8프레임) | 정렬 순환 |
| 13 | KEYCONFIG | `s_wdKeyconfig` act | 키 컨피그 화면 |
| 14 | SKINSELECT | `s_wdSkinChange` act | 스킨 선택 화면 |
| 16 | AUTOPLAY | `btn-autoplay-rect` act | 오토 플레이 |
| 17 | READTEXT(open_document) | `btn-text-rect` act (리터럴 17) | 텍스트 파일 열기 |
| 19, 316, 317, 318 | REPLAY, 2, 3, 4 | `btn-replayN-on`, `s_autoReplayOnN` act | 리플레이 슬롯 N 재생 |
| 40 | GAUGE_1P | imageset `option-gauge(-info)` ref | 게이지 타입(6) |
| 42, 43 | RANDOM_1P, 2P | imageset ref(10) | 랜덤 옵션 |
| 54 | DPOPTION | imageset ref(4) | DP 옵션 |
| 55 | HSFIX | imageset ref(5) | 하이스피드 고정 |
| 72 | BGA | imageset `option-bga` ref(3) | BGA 모드 |
| 74 | JUDGE_TIMING(notesdisplaytiming) | `timingAdjustBtn` act(1x1 이미지 영역 클릭) | 노트 표시 시간 조정 ±1 |
| 75 | JUDGE_TIMING_AUTO_ADJUST | imageset `option-NDTA` ref(2) | 자동 조정 on/off |
| 78 | GAUGEAUTOSHIFT | imageset `option-gas` ref(5) | 게이지 오토 시프트 |
| 79 | RIVAL | `compare-rivalSelector_on` act | 라이벌 변경 |
| 90 | FAVORITTE_CHART | `favinv` act+ref(3프레임) | 즐겨찾기 순환 |
| 210 | OPEN_IR_WEBSITE | `s_wdIrpageOpen` act | IR 페이지 열기 |
| 301..307 | ASSIST_EXJUDGE, CONSTANT, JUDGEAREA, LEGACY, MARKNOTE, BPMGUIDE, NOMINE | assist imageset ref(2), `assist-*SW` act | ref 는 동작(`IntegerPropertyFactory` 의 301/302/303/304/305/306/307 이미지 인덱스). **act 는 이 beatoraja 에서 처리기가 없다**(`EventFactory.EventType` 에 301..307 없음, 폴백 `MainState.executeEvent` 는 커스텀 이벤트만 처리) -> 클릭해도 무반응. 키보드 입력으로만 토글. |
| 308 | LNMODE | `btn-lnmode` act+ref(3프레임) | LN/CN/HCN 순환 |
| 315 | PRACTICE | `btn-practice-rect` act | 연습 모드 |
| 321..324 | AUTOSAVEREPLAY_1..4 | `s_autoReplaySettingN` act+ref(len 11) | 자동 저장 조건 순환(11단계) |
| 330, 331, 332 | LANECOVER, LIFT, HIDDEN | imageset ref+act(2) | 토글 |
| 340 | JUDGEALGORITHM | imageset ref+act(3) | 판정 알고리즘 순환 |
| 341 | BOTTOMSIFTABLEFGAUGE | imageset ref+act(3) | 하한 게이지 |
| 342 | HISPEEDAUTOADJUST | imageset ref+act(2) | HS 자동 조정 |
| 400 | CONSTANT | imageset ref+act(2) | 가변 SUD(constant) |

### 6.5 문자열(text `ref`)

| id | 이름 | text id |
|---|---|---|
| 1 | RIVAL | rivalname |
| 2 | PLAYER | yourname |
| 3 | SELECTED_TARGET | s_rival |
| 10, 11 | TITLE, SUBTITLE | title, subtitle |
| 13, 14, 15 | GENRE, ARTIST, SUBARTIST | genre, artist, subartist (fullArtist 는 함수) |
| 30 | SEARCHWORD | search |
| 120..129 | RANKING1..10_NAME | irRankName1..10, s_irRankName1..10 |
| 150..159 | COURSE1..10_TITLE | course1..10 |
| 200..209 | TARGET_FORWARD1..10 | f_rival1..10 |
| 210..219 | TARGET_BACKWARD1..10 | b_rival1..10 |
| 1000 | DIRECTORY | directory |
| 1020 | IR_NAME | repositoryname |
| (상수) | - | version(`"Ver 4.6"`), sepa(`"/"`) |

### 6.6 슬라이더 type, 그래프 type, 이미지 참조

슬라이더: `1` MUSICSELECT_POSITION(`mainframe.lua:22`), `8` ranking position(`sidemenu.lua:222`, 엔진 `FloatPropertyFactory.java:243` 에 `ranking_position(8)`), `17/18/19` MASTER/KEY/BGM_VOLUME(`sidemenu.lua:218-220`).
그래프: `140..144` RATE_PGREAT..POOR, `147` RATE_EXSCORE(`score.lua:72,120-124`), `-1` 곡 바 램프 분포(`songlist.lua:239`). 값이 함수인 그래프는 4.2절.
참조 이미지(destination id): `-100` STAGEFILE, `-102` BANNER, `-110` BLACK, `-111` WHITE (`Root/mainimage.lua`; 엔진은 부호를 뒤집어 `SkinProperty.IMAGE_*` 100/102/110/111).

### 6.7 이미지 인덱스 계열 (`event_index`)

`MAIN.BUTTON.ASSIST_*`(301..307): `mainframe.lua` 의 `isAssistOn()` 이 7개를 읽어 하나라도 1 이면 경고 이미지를 띄운다(`Root/customoption.lua:139-149`). 값 1 = 활성.

---

## 7. Lua 런타임 의존 (요구 절 6)

### 7.1 모듈과 전역

| 대상 | 쓰임 | 비고 |
|---|---|---|
| `require("main_state")` | 전역 `main_state` | 헤더 단계에선 빈 테이블이어도 됨 |
| `require("timer_util")` | 전역 `timer_util`, 쓰는 함수는 `timer_observe_boolean` 하나 | 본 로드 단계에서만 호출 |
| `require("luajava")` | `Root/customfunction.lua:7` -> `luajava.bindClass("java.io.File")` (`:8`) | 선곡 로드 시 **반드시 성공해야 함**. 선곡 경로에서 실제 호출되는 것은 `bindClass` 하나(로드 시). `luajava.new(File, path)`, `dir:mkdir()`, `dir:listFiles()` 는 배경 로테이션/`mkdir` 에서만. `luajava.newInstance("java.net.URL"...)` 는 죽은 http.lua 만 |
| `require` 상대 모듈 | `Root.define`, `Root.define2`, `Root.main*`(9), `Root.custom*`(9), `Root.version`, `Root.author`, `Select.lua.require.{header,property,textproperty,settings,http}`, `config` | 점 구분, `?.lua` 검색 |
| `dofile(abs_path)` | 활성 18회(`musicselect.lua`) | `skin_config.get_path` 가 반환하는 **절대 경로** 를 받는다. `dofile` 이 없으면 18개 섹션이 전부 조용히 사라진다 |
| `pcall` | 섹션당 1회(18) | 예외 삼킴 |
| 전역 대입 | `MAIN, CUSTOM, CONFIG, PROPERTY, main_state, timer_util` (스킨이 대입), `DEBUG`(읽기만, nil) | 같은 Lua 상태를 헤더/본로드에 재사용하면 `Root.customnumber` 의 로드 시점 값(`todaySongUpdateCount` 등)과 `package.loaded` 캐시가 오래된다. beatoraja 는 로더마다 새 상태를 만든다 |

### 7.2 skin_config

- `skin_config.get_path(rel)`: 스킨 디렉터리 기준 상대 경로 -> 절대 경로(파일 선택 `filemap` 적용). 123회 호출(선곡 핵심은 섹션 18 + `customnumber` 로드 시 9 + `infoOutput` 1 + 사운드 재생 시).
- `skin_config.option[<옵션 name>]`: `property.lua:47`(함수 안, 지연 평가) -> 정수 op.
- `skin_config.offset[<오프셋 name>]`: `.a/.x/.y/.w/.h` 접근자 함수가 정의돼 있으나(`property.lua:66-70`) 선곡에서는 호출되지 않는다. `PROPERTY.offsetBgBrightness.num`(=40)만 `background.lua:37` 에서 읽는다.
- `skin_config.file_path`, `enabled_options` 미사용.

### 7.3 표준 라이브러리

| 라이브러리 | 선곡 경로에서의 사용 |
|---|---|
| `table.insert` | 700여 회(전부) |
| `ipairs`, `pairs`, `tostring`, `print` | `ADD_ALL`/`LOAD_HEADER`, 설정 파일 로그(`print` 51회 중 대부분 `DEBUG` 가드 또는 settings/http) |
| `math.random(a,b)` | 로드 시 3회(`beamBottom` r,g,b 100..255, `mainframe.lua:87`) + 로테이션 |
| `os.date` | `%y%m%d`, `%Y/%m/%d`, `%H:%M:%S` 를 모듈 로드 시 1회(`customtime.lua:6-10`) |
| `io.open` | 9개 읽기(`customnumber.lua:289-303`: `io/Play/sp/lanecover/` 와 `io/Play/dp/lanecover/` 아래의 `pathList.txt`, `excludeList.txt` 각 2개(합 4), `History/<yymmdd>/{history,clear,score,miss}.txt`, `History/impression.txt`). `infoOutput(5)` 의 읽기 1 + 생성/쓰기 2(`History/information.txt`). 사이드메뉴 보존 ON 일 때 `Select/lua/settings/sidemenu` 읽기/쓰기. 로테이션 ON 일 때 `io/Select/bg/image/` 또는 `io/Select/bg/movie/` 아래의 `pathList.txt`, `excludeList.txt` 읽기/쓰기/추가 |
| `string.gsub/match`, `math.floor/ceil/max` | 로테이션(`customfunction.lua:124-125`)과 죽은 http.lua 에서만 |
| `bit32`, `unpack`, `loadstring`, `setfenv`, `getfenv`, `table.getn`, `string.gfind`, `math.pow`, `goto` | 사용 없음 |
| `debug`, `coroutine`, `package.*` 직접 호출 | 없음 |
| `loadfile`, `load`, `_G`/`_ENV` 조작 | 없음 (`_G` 문자열 일치는 `MAIN.*_GAUGE` 같은 이름) |

LuaJ(5.2) 전용 문법/함수는 없다. 그러나 mlua(Lua 5.4) 로 옮길 때 주의: 정수/실수 구분(`200 / 1.3` 같은 실수가 `w`,`h`,`time`,`cycle`(int 필드)로 들어감: `mainframe.lua:157-208` 의 char 애니메이션, `oneBeat2(4,i)` 의 `(60/i)*1000*4`). beatoraja 는 `toint()`(소수 버림)로 받는다. R-BMS 는 int 필드의 Lua 실수를 버림으로 받아야 하고 "no integer representation" 오류로 거부하면 안 된다. `#` 길이 연산자는 `filelists`(로테이션)와 `#diff`, `#lampName` 등 연속 배열에만 쓴다. `string` 이어붙이기의 숫자는 정수(`"char" .. i`)라 문제없다.

### 7.4 부수효과 함수 정리

`main_state.audio_play(path[, volume])`: 선곡에서는 `Root/sounds/open.ogg`, `close.ogg` 만 호출. 볼륨 생략(nil) = 1.0, 시스템 볼륨 곱, [0,2] 클램프(`B:skin/lua/SkinAudioLuaApiExporter.java:play`). `set_volume_sys/key/bg(x)`: 설정 볼륨(float)을 즉시 변경.

---

## 8. 1920x1080 레이아웃 (요구 절 7)

좌표 `(x, y, w, h)`, 원점 좌하단, y 위로 증가. "z" 는 그리는 순서(큰 번호가 위). 선곡은 **전체가 한 장짜리 풀스크린 합성**이며 내장 화면과 섞이지 않는다. 아래 좌표는 Lua 리터럴 그대로다.

### 8.1 z 순서 그룹

| z | 파일 | 내용 |
|---|---|---|
| 1 | background | 배경, 밝기 오버레이, 스테이지파일/배너, 서브타이틀 띠 |
| 2 | songlist | 곡 목록 바, 선택 프레임 |
| 3 | mainframe | 상단 프레임, 하단 프레임, (char), 검색, 모드 표시, 빔, 키 안내, 스크롤바, 어시스트 경고 |
| 4 | musicdisplay | 곡 정보 텍스트, BPM, 곡 수, 폴더 클리어 현황, 인디케이터 |
| 5 | btnarea | 기능 버튼 열 |
| 6 | cource | 단위(코스) 정보 |
| 7 | info | 좌하단 정보창 |
| 8 | score | 중앙 하단 스코어창 |
| 9 | bmsanalysis | 우하단 분석창 |
| 10 | qco | IR 박스 |
| 11 | rivalview | 라이벌 비교 |
| 12 | sidemenu | 좌측 사이드메뉴 |
| 13 | history | (옵션) |
| 14 | help | 도움말 버튼/장면 |
| 15 | startanimation | 시작 연출 |
| 16 | option(패널1) | |
| 17 | assistoption(패널2) | |
| 18 | suboption(패널3) | |

### 8.2 z1 background (`background.lua`)

| 요소 | 사각형 | 비고 |
|---|---|---|
| `background` | (0,0,1920,1080) | 이미지 `Select/bg/image/*.png` 또는 mp4. 시작 연출이 이 id 를 추가 3회 그린다(8.9) |
| BLACK(-110) | (0,0,1920,1080) a=0 | `offsets={40}`: 사용자 오프셋 a 만큼 불투명도 증가(255 = 완전 검정) |
| stagefile(-100) (기본 "両方表示") | 키프레임 `{0: x=120,y=406,w=480,h=360,a=0},{300: x=80,a=255}`, `loop=300`, `timer=11` | 곡 이동 때마다 왼쪽에서 300ms 페이드인 |
| banner(-102) | `{0: x=700,y=785,w=300,h=80,a=0},{300: a=255}`, `loop=300, timer=11` | |
| stagefile 풀사이즈(옵션) | `{0: x=0,y=0,w=1920,h=1080,a=0},{300: a=255}`, `loop=300, timer=11` | 배너 없음 |
| `subtitle` 텍스트 (서브타이틀 스크롤 ON) | `{0: x=1920,y=500,w=1920,h=170,r=255,g=255,b=0,a=170},{10000: x=-1920}`, `loop=0` | 10초 주기 우->좌 스크롤. 폰트 id 0(black) size 170, overflow=1 |
| `main-bg` | (0,406,1920,360) | `mainframe.png (1401,1240,50x50)`: 검정, 알파 128 (표본 확인). 서브타이틀 ON 일 때만. **subtitle 텍스트 뒤가 아니라 위에** 그려진다(삽입 순서) |

### 8.3 z2 songlist (5절) 와 z3 mainframe

곡 목록: x 약 1125..2085 (화면 우측 약 800px 가 보임), y -55..1135(위아래로 일부 잘림). 오버레이: `selectmusic-frame` (1112,499,990,82).

mainframe (`mainframe.lua`):

| 요소 | 사각형/키프레임 |
|---|---|
| `beamBottom` | `{0: x=0,y=0,w=1920,h=30,a=150,r,g,b=random(100..255)},{5000: y=540,a=0}`, 가산 블렌드, 이미지 `(0,2950,960x30)`(회색 그라디언트) |
| `main-top` | (0,874,1920,207) `src(0,2400,1920x207)` |
| `main-dia` x2 | `{0: x=0,y=964,w=2200,h=75,a=150},{50000: x=-2200}` / `{0: x=2200,...},{50000: x=0}`, loop 0 |
| `main-title` | (40,987,540,30) `src(0,1290,540x30)` |
| `skinname` | (1430,1057,396,18) |
| `version`(text) | (1840,1055,80,16), `filter=0`, 폰트 black size 16, `"Ver 4.6"` |
| `keyinfo` | (680,963,350,113) (키맵 표시 ON) |
| `main-bottom` | (0,0,1920,270) `src(0,2616,1920x270)` |
| 검색 | `main-search` (5,885,463,45) `src(0,1240,463x45)`, `search`(text, ref 30) (50,895,418,30), 폰트 medium size 30 |
| 모드 표시 | (1527,213,306,39): op 161->`5keys`(src y=39), 160->`7keys`(78), 163->`10keys`(117), 162->`14keys`(156), 164->`9keys`(195), 1160->`24keys`(234), 1161->`48keys`(273). `allkeys`(y=0) 와 imageset `btn-modeset` 은 destination 이 없다 |
| 빔 가이드(장식) 4개 | 19x19 점이 화면을 횡단: 상단 2개 `(1920,920)->(466,920)@10000ms/6000ms -> y 869 @+500/+250ms -> x=0 @15000/8500ms`; 하단 2개 `(0,206)->(1434,206) -> y 258 -> x=1920`, loop 0. 이미지 `(2250,1300,57x19, divx=3, cycle=100)` |
| 키 안내 캐러셀 `info` | (90,777,500,90), 이미지 `(900,1470,500x630, divy=7, cycle=14000)` -> 90px 7프레임을 2초마다 교체("1鍵盤でプレイ開始" 등) |
| 스크롤바 프레임 | `{0: x=2000,y=278,w=28,h=636},{1000: x=1860}` DECELERATE, loop=1000, `src7 (980,0,28x636)` |
| 스크롤 램프(slider) | dst `{1000: x=1857,y=873,w=35,h=45},{2000: a=150},{3000: a=255}` loop=1000, 슬라이더 `type=1, range=600, angle=2(아래)`. t=1000 이전 숨김 |
| 어시스트 경고 | `{0: x=600,y=773,w=400,h=100},{500: a=220},{1000: a=255}`, draw `isAssistOn()` |

BPM 연동 캐릭터(`animationChar`, 기본 ON, 중요도 낮음): 기준 `body={x=1700, w=200/1.3}`, `pre={y=265, h=305/1.3}`, `post={y=270, h=305/1.3+3}`. `char_normal`(op -2 = 곡 바가 아님): `{0: x=1700,y=265,w=153,h=234,acc=2},{4800},{5000: y=270,h=237},{5200: y=265,h=234}` loop 0. 곡 바 선택 시 `char{i}`(i = 정수 MAINBPM 1..1000, `timer=11`, 이미지 `(0,0,400x610, divx=2, divy=2, cycle=240000/i)` = 4프레임/4박) 가 비트마다(1/8 주기) y 를 pre/post 로 교대. `draw` 는 `i == MAINBPM and not isNeglect(30)`. 30초 방치(`isNeglect(30)` and 곡 바)면 `char_wait`(src (400,0,200x305)) 가 `x: 1700 -> 1400(4750) -> 반전하여 1553(5000, w=-153) -> 1850(9750) -> 1700(10000)` 으로 왕복(음수 폭 = 좌우 반전, 키프레임 사이 w 보간 포함). 모두 `filter=0`(최근접).

### 8.4 z4 musicdisplay (`musicdisplay.lua`)

| 요소 | 사각형/키프레임 | 조건 |
|---|---|---|
| `directory`(text 1000) | (990,715,890,25), 색 (240,240,240), 오른쪽 정렬, size 25 | 항상 |
| `genre`(text 13) | `{0: x=980,y=660,w=920,h=30,r=g=b=255},{150: x=1000}` loop=150, timer=11, 오른쪽 정렬 size 30 | 항상 |
| `title`(text 10) | `{0: x=1000,y=550,w=920,h=70,r=216,g=255,b=0},{150: y=570}`, size 70, 오른쪽 정렬, overflow shrink, shadow 4/4 | 항상 |
| `fullArtist`(text 함수) | `{0: x=980,y=520,w=920,h=30},{150: x=1000}` | 항상 |
| BPM 프레임 `bpm-frame` | (835,442,180,50) `src5 (1500,1240,180x50)` | 곡 바 |
| BPM 변화 있음(op {2,177}) | `minbpm`(380,442,60,50) 숫자 3자리(셀 폭 60), `bpm-while`(575,442,60,50) `src (1860,1240)`, `maxbpm`(650,442,60,50). 룰렛 오버레이 `bpm-roulette-3/2/1` 가 곡 이동 후 0..199ms 동안 각 자리(x+0,+60,+120)에서 도는 효과(`loop=-1`, `timer=11`, `cycle` 150/100/50) | |
| BPM 변화 없음(op {2,176}) | `maxbpm`(650,442,60,50) `loop=200,timer=11`, 룰렛 3개 (650/710/770) | |
| `totalsongs-frame` | (630,491,240,50) + `folder-totalsongs`(870,498,34,42, digit 4, ref 300) | 폴더 바 |
| `randomselect-frame` | (400,530,610,120) | op 1030 |
| 폴더 클리어 현황 | 프레임 `src5(2000,2900,917x150)` (83,240,917,150), 11개 색 바 `(83+x[i], 240+y[i], w[i], 35, a=150, 색[i])` (-111 WHITE 사용), 숫자 `(83+adjX[i], 240+adjY[i], 22,15)` digit 4 (ref 320..330), 풀콤 `(365,353,203,35)` 3프레임 cycle 300, EXH `(570,353,160,35)` 2프레임 cycle 200. 각 항목은 `option(1) and number(ref) ~= 0` 일 때만. `x=[649,487,282,120,2,779,649,487,282,120,2]`, `y=[2,2,2,2,2,76,76,76,76,76,76]`, `w=[266,160,203,160,116,136,128,160,203,160,116]`, `adjX=[740,525,340,155,15,805,670,525,340,155,15]`, `adjY=[10,10,10,10,10,85,85,85,85,85,85]`, 색 r=[120,130,200,100,100,100,200,255,160,255,255] g=[30,22,80,120,140,200,200,110,120,150,200] b=[120,15,200,200,10,200,200,110,0,0,0] (순서: noplay, failed, assist, lassist, easy, clear, hard, exhard, fullcombo, perfect, max) | 폴더 바 |
| 인디케이터 | OFFLINE/ONLINE (486,880,168,40), NO_BGA/BGA (659,880,168,40), `favinv`(832,880,168,40, 3프레임 act 90) | |

### 8.5 z5 btnarea (`btnarea.lua`) - 우상단 버튼 열

모든 버튼 104x91, 라벨 `tex-*` 110x11 (y=937). 호버 이미지 색은 `(216,255,0)`.

| 버튼 | 사각형 | 소스(`src5`) | act |
|---|---|---|---|
| AUTO | (1074,956) / 라벨 (1074,937) | `(0,1470)` | 16 (호버 오버레이 `btn-autoplay-rect`) |
| PRACTICE | (1194,956) / 라벨 (1202,937) | `(104,1470)` | 315 |
| TEXT | (1315,956) / 라벨 (1343,937) | no-text `(208,1470)`, text `(208,1561)` (op 175) | 17 (op 175 일 때만) |
| REPLAY 1~4 | 53x46: (1433,1004), (1490,1004), (1433,953), (1490,953) / 라벨 (1452,937) | off `(630/683/736/789, 1516)`, on `(.., 1470)`, rect/select `(.., 1562)` | 19/316/317/318 (op 197/1197/1200/1203), select 오버레이 op 1205..1208 |
| LN | (1555,956) 두 번(두 번째는 호버 색 입힘) / 라벨 (1568,937) | `(312,1470, 104x273, divy=3, len=3)` ref 308 | 308 |
| KEY | (1675,956) 두 번 / 라벨 (1682,937) | `(416,1470, 104x730, divy=8, len=8)` ref 11 | 11 |
| SORT | (1795,956) 두 번 / 라벨 (1796,937) | `(520,1470, 104x730, divy=8, len=8)` ref 12 | 12 |

### 8.6 z6 cource (op 3 = 단위 바, `cource.lua`)

`course-frame` (30,410,1005,345) `src5(1500,1470,1005x345)`. 제약 아이콘 192x28, y=417: random/mirror x=49, nospeed x=243, nogood/nogreat x=437, 게이지 종류 x=631, ln/cn/hcn x=825 (각 op 조합 `{3, 1004|1003|1005|1006|1007|1010..1014|1015..1017}`). 곡 수 5 이하(`isCounseWithin5`): `course-1-5`(68,470,87,258) + 곡명 `course1..5` (584, y=685/633/581/529/477, 850x30, 가운데 정렬). 6곡 이상(`isCounseOver6`): `timer=11`, 3초 주기로 1-5 / 6-10 교대(키프레임 `0..2999`, `3000: a=0`, `6000`; 6-10 은 반대), 인디케이터 `course-indicator`(950,731,57,19; `src9 (980,80,57x57, divy=3, cycle=3000, timer=11)`).

### 8.7 z7~z9 하단 3창 (y=10..204)

좌하단 info (`info.lua`, base (10,10)):
- `info-frame` (10,10,429,194) `src5(2401,312,429x194)`.
- 곡이 아닐 때(op -2): `info-frame2`(17,12,180,190) + 플레이어 누적 PG/GR/GD/BD/PR/노트 숫자 6개 `x=210`, y=`176,146,114,83,51,21`(22x15, digit 10).
- 곡일 때(op 2): `info-frame3`(17,12,270,190); `totalnotes`(150,177,20,15, digit 4, align 1); `songlen-min`(150,115) `songlen-sec`(213,115) (digit 2); `playcount`(150,85); TOTAL 값 `total`(150,146,20,15, digit 4, align 1)을 3분기로 색칠(`calcTotal()-20 < total` 이면 (0,120,255), `calcTotal()+20 >= total` 이면 (255,120,0), 사이면 기본색); TOTAL 게이지 프레임(240,143,186,24)+막대(243,146,180,18)(`gr_totalLow/High`, 가산), 라벨 `totalFrameInfoLow/High`(286,148,93,16); 판정 레벨 이미지(240,53,165,16, op {180..184, 2}) + 숫자(150,53,22,15, ref 400); IR 대기 시간(235,21,20,15, ref 220, op {2, 51}), `complete`(235,21,130,20, op {2,602}), `offline`(225,19,130,20, op {2,50}).

중앙 스코어창 (`score.lua`, window (454,10)): `score-frame`(454,10,895,194); 판정 막대 5개 x=652, y=`61,91,120,149,178`(w 0->300, 300ms, DECELERATE, timer 11, h 18, 소스 y=`72,54,36,18,0`); 카운트 숫자 x=569 y=`62,92,121,149,179`(19x15 digit 4); EX/CB/MS 숫자 x=1079, y=`172,143,114`(18x30); 랭크바 `score-bar`(460,17,696,40) + 클리어 종류별 `bar-exscore-*`(463,20, w 0->690, h 33, 300ms, op 클리어) + `score-bar-info`(460,17,696,40) 위에; 랭크 문자(1160, y 79->69 로 10px 내려오며 알파 0->255, 187x73, op `{200..207, -100}`); 클리어 종류 이미지 `sel-bar-*`(1160, y=29(미플레이 85), 187x33); 점수율 `scorelate-frame`(1250,160,83,25), `scorelate`(1170,160,26,25, digit 3, align 0), `scorelate-afterdot`(1262,160,20,20, digit 2) (`loop=1000`, op -100); 타이밍 그래프 프레임(964,57,186,60) + `bar-slowRate`(967,80,180,34) + `bar-fastRate`(1147,80,**-180**,34) + 수치 `fastcount`(1069,82,18,30), `slowcount`(974,82).

우하단 분석창 (`bmsanalysis.lua`, window (1360,10)): `detail-frame`(1360,10,551,194) `src5(2410,600,551x194)`; BLACK(-110) (1366,97,415,100); `notes-graph` 와 `bpmgraph` (1366,97,415,100) 둘 다 op `{2, 5}`; 노트 종류 숫자 x=1831, y=`170`(normal) `121`(scratch) `76`(ln) `27`(bss)(18x15, digit 4); 밀도 x=1600, y=`75`(peak) `48`(end) `17`(average).

### 8.8 z10 IR 박스 (`qco.lua`, 기본 `クリアレート&フルコンボレート`)

draw 조건 공통: `(option(2) or option(3)) and option(51)`. `qco_ir_frame` (18,235,560,160) `src9(0,0,560x160)`; `repositoryname`(568,240,560,20, 오른쪽 정렬, 색 (237,177,10)); 순위 `ir_rank2`(37,268,43,42, digit 5, align 2; `rank/total*100 >= 10`) 또는 `ir_eliteRank`(상위 10% 이내, 3프레임 100ms 순환); `ir_totalplayer2`(318,268,43,42); `qcoLoading`(108,265,75,44) (rank==0 또는 IR_WAITING 시 2개 깜박임 `90/370`). 클리어율 `qco_cr_frame`(590,325,410,70)+`%`(910,330,100,70)+숫자(793,340,43,42)+소수(932,340,31,31)+로딩; 풀콤율 `qco_fc_frame`(590,235,410,70) 동일 배치(y=235). `ランキング` 모드: `qco_ranking_frame`(590,235,410,160), 슬롯 5행 y=`112,85,59,33,6`(+235): 인덱스 숫자(595,.,13x15), 이름(670,.-3,100x16), EX 점수(765,.,13x15), 클리어 이미지(885,.-3,110x22), 랭크 문자(830,.-3,59x22), 본인 하이라이트 `qcoMyRank`(584,.-12,420x40).

### 8.9 z11~z15

라이벌 비교 (`rivalview.lua`, window (10,220), op 625): 프레임 (10,220,1022,655,a=230); 이름 `yourname`(280,720,300,35) `rivalname`(780,720); 승/패 이미지(190/695, 790, 170x50); 랭크 문자 (190/695,655,170x50); 항목 6개(score, pgreat, great, good, bad, poor) 숫자 x=175(내)/685(라이벌), y=`595,530,465,400,335,270`(34x42, digit 5, 이긴 쪽 (0,192,242) 진 쪽 (238,67,0)); 라벨 8개(440, y=`720, 655, 590, 525, 460, 395, 330, 265`, 170x50); 라이벌 선택 `compare-rivalSelector_off/on`(442,798,168,40, act 79, 500ms 알파 펄스).

사이드메뉴 (`sidemenu.lua`, posX=80, posY=228, openTime=200): 4 아이콘 62x62 x=2, y=`710`(램프) `630`(랭킹) `550`(볼륨) `470`(설정); 셀렉터 `s_selector`(70x469) `{1300: x=-72,y=380},{1500: x=-2}`(loop 1500); 프레임 `s_menu`(930x636) 열림 `{0: x=-930,y=228},{200: x=80}`(acc 2), 닫힘 `{0: x=-80},{200: x=-930}`; 램프 패널(IR 램프): `s_ramp`(280,246,694x603), `s_rampName`(480,246,245x603), `s_percentFrame`(853,246,120x603), 11행(y=`805` 에서 55씩 감소; max, perfect, fullcombo, exhard, hard, normal, easy, assist, lassist, failed, noplay): 인원 숫자 `x=100`(31x36, digit 5), 퍼센트 `x=772`(digit 3)+소수 `x=872`(digit 1) y=`804` 부터, 막대 `x=282, y=798` 부터(h 49, w 0->690/500ms, 가산, IR 수신 후 타이머 173 와 메뉴 열림 조건), 본인 클리어와 같은 행은 색 (255,161,3). 랭킹 패널: 10행 `s_rankingFrame`(80, 800-63*i, 930,64), 점수 그래프(87, +2, w 0->916, h 21, 랭크 구간별 색), 순위 `x=100, y+30`(20x24, 색 (g=255,b=9, a=201)), 이름 `x=250, y+26`(300x25), 점수 `x=690, y+30`, 랭크 문자 `x=810, y+32`(59x22), 클리어 `x=880, y+32`(110x22), 본인 `s_rankingMyPostion`(77, y-5, 936x70); 스크롤 프레임(1010,228,28,636)+램프 `type=8`(1007,823,35x45, range 600, 아래). 볼륨 패널: `s_volumeFrame`(80,228,930,635), 슬라이더 3개 x=350, y=`615`(master) `502`(key) `380`(bgm)(15x39, range 385, 오른쪽), 숫자 x=850 y=`627,512,392`(22x15, digit 3), 최소/최대 아이콘 x=275/770 y=`605,492,370`(64x61). 설정 패널(y 기준 228): 버튼 231x61 x=`200,430,658` y=614 (키컨피그, 스킨 변경, IR 페이지 열기(곡/단위 바 때만, 아니면 Ng 이미지)), 자동저장 4 슬롯 x=`91,317,542,768` y=390(설정)/328(재생). IR 로딩/오프라인 오버레이: BLACK a=200 (80,228,930,636) + 로딩 이미지 4프레임 / `IR接続不可`.

도움말: 버튼 `helpBtn`(10,1050,168,20) `src5(1570,2230,168x20)`; 화면 `helpScene`(0,0,1920,1080) 클릭 시 닫힘, 열림 `{0: a=0},{200: a=255}` (loop 200, 타이머 `isMenuOpen`), 닫힘 `{0: a=255},{200: a=0}` (loop -1, 타이머 `not isMenuOpen`).

시작 연출 (`startanimation.lua`, 스킨 전역 시간, 모두 `loop=-1` = 1회): fade-in(기본): `fade-bg`(검정 1x1 픽셀) `{0: (0,0,1920,1080)},{800},{1000: y=540,h=0}`; `fade-tex`(325,513,1250,50) `{0: a=0},{300: a=255},{800}`. 셔터: `shutter-l`(0,0,1004,1080)->`{800},{1000: x=-1004}`, `shutter-r`(915,0,1003,1080)->`{800},{1000: x=1919}` (DECELERATE). 배경 줌(정지화 배경일 때, 3개): `{1000: (-960,-540,3840,2160,a=100)},{1250: (0,0,1920,1080,a=0)}`; `{1000:...},{1500:...}`; `{1000: (-9600,-5400,21120,11880,a=100)},{2000:...}`. 이력 ON: `todayPlayCountFrame2`(750,420,250,31)+숫자(1030,420,52,31).

패널 3종(공통): 열림 `{0: top y=1080 / bottom y=-565},{100: top y=515 / bottom y=0}`(acc 2, loop 100, op 패널n, 타이머 `PANELn_ON`), 닫힘 반대(타이머 `PANELn_OFF`, op 없음). 상/하 프레임은 1920x565 이며 `op-top` y=515..1080, `op-bottom` y=0..565(50px 겹침). 내용물은 `loop=100` 단일 키프레임 `time=100`(= 열린 뒤 100ms 에 표시): 
- 패널1 `option.lua`: `op-dia` 4(`(0|-1950|1950, 58|970, 1950x50)`, 50초 스크롤), `op-title`(533,58,853,50), `op-menu`(351,156,1273,680), `op-info`(487,866,1001,91), 화살표 깜박임 up x=`850,1062` y=781, down x=`940,780,887,994,1101` y=`781/665`, 선택기 10종(가산): 랜덤1P 정보(495,198,156,273)+목록(353,204,143,240), 랜덤2P(1468,198)+(1326,204), 게이지(801,293,196,178)+(659,299,143,144), DP(961,157,196,130)+(819,162,143,96), HS고정(1147,317,171,154)+(1006,323,143,120), 타겟 목록(40, 770 에서 27씩 감소 x 21행, 295x30; 전반 10 + 선택 1 + 후반 10), DP 전용 커버(1013,157,627,698) op `{21,-162,2}`.
- 패널2 `assistoption.lua`: `assist-menu`(651,201,618,642), `assist-info`(646,915,628,83), `assist-config`(52,915,558,86)/(1311,915,558,86), `assist-info-l`(56,162,550,732), `assist-info-r`(1315,162,550,732), 선택기(가산 143x48) (653,204)(810,204)(967,204)(1124,204)(673,738)(889,738)(1106,738), 깜박임 스위치 70x18 7곳, 클릭 영역 147x108(1x1 이미지, act 301..307: 이 엔진에서는 무반응).
- 패널3 `suboption.lua`: `subop-menu`(651,216,619,632), `subop-info`(646,915,628,83), `subop-menu-l/info-l`(1315,162,550,732)/(1311,915,558,86), `subop-menu-r/info-r`(56,150,550,744)/(52,915,558,86), 수치 `judgetiming`(1087,272,9,14 digit 3, 24칸 부호 시트), `duration-green`(1130,775,9,14 digit 4), `duration`(1060,775), 선택기 GAS(653,676,143,120 가산)/BGA(653,220,143,72)/NDTA(810,244,143,48), 시스템 버튼 7행 x=363 y=`492+57*i`(230x60, 순서 VS, 판정알고리즘, HAA, hidden, lift, lane, LLOG; 호버 `subop-btnRect` mouseRect), 판정 타이밍 클릭 영역(964,216,306,128, act 74).

### 8.10 옵션에 따른 배치 변형 요약

| 옵션 | 변화 |
|---|---|
| 곡 목록 배치 | 5절의 `X[i]` 3종만 (바 `x`). 그 외 좌표 동일 |
| 스테이지/배너 | 8.2 |
| 서브타이틀 스크롤 | 8.2 (띠+텍스트 제거) |
| 키맵 표시 | `keyinfo` 1개 제거 |
| 빔 | `beam-guide` 4개 제거 |
| 시작 패턴 | 8.9 |
| IR 표시 | 8.8 (cfrate/ranking) |
| BPM 캐릭터 | 8.3 |
| 언어 | 이미지 시트만 교체(좌표/크기 동일). 6장(assistop, subop, mainframe, op, help, sidemenu) |

---

## 9. UI 요소별 동작: 이벤트, 타이머, 마우스 영역

| 기능 | 트리거 | 마우스 영역 / 타이머 / 이벤트 |
|---|---|---|
| 버튼 열(AUTO, PRACTICE, TEXT, REPLAY 1-4, LN, KEY, SORT) | 좌클릭(+1)/우클릭(-1). 호버 시 `*-rect`(또는 두 번째 인스턴스)가 `mouseRect` 로 색(216,255,0)을 입혀 그려짐 | 8.5. act id 는 6.4. 클릭 판정은 z 역순 첫 `draw=true` 객체 |
| 곡 목록 | 바 클릭(좌) / 우클릭 = 폴더 닫기 | `clickable={6..10}` 의 바 영역(현재 보간 위치) |
| 스크롤바 | 슬라이더 `type=1`(MUSICSELECT_POSITION) 드래그(`changeable`), 위->아래 `range=600` | `Skin.mouseDragged` 가 `SkinSlider` 만 처리(`Skin.java:413-420`). 스킨은 y=873 에서 아래로 600px |
| 사이드메뉴 4종 | 아이콘 클릭 -> `rampMenuSwitch` 등 (한 번에 한 메뉴만 열림, 같은 아이콘 재클릭 = 닫기) | 아이콘 62x62, off/rect/on 3종 이미지를 타이머(`isXOpen`)로 교대. 열림 `loop=200` 애니, 효과음 open/close(4.4) |
| 램프 패널 | IR 데이터 | 막대는 `timer_util.timer_observe_boolean(isTimerOn(173) and open)`(173=IR_CONNECT_SUCCESS) |
| 랭킹 패널 | `s_rankingScrollLamp` 슬라이더 `type=8` 드래그로 IR 목록 스크롤 | 슬롯 값은 엔진의 `rankingOffset` 반영 |
| 볼륨 | 슬라이더 3개(type 17/18/19)와 최소/최대 아이콘 클릭 -> `set_volume_*` | 숫자는 `volume_*()*100` |
| 설정 패널 | KEYCONFIG(13), SKINSELECT(14), OPEN_IR(210) 버튼, 자동저장 4 슬롯(321..324 순환), 리플레이 재생 4(19,316..318) | 버튼 231x61, 호버 `s_btnRect`(mouseRect) |
| 옵션 패널 1/2/3 | **키 입력**(START / SELECT / `5`) 으로 열림. 패널 안 선택기는 표시 전용(클릭 act 없음). 단 패널3 은 `act` 로 7개 시스템 토글 + 판정 타이밍 | 타이머 `PANELn_ON/OFF`, op 21/22/23 |
| 라이벌 | op 625(비교 모드)에서만 그려짐, 선택기 클릭 act 79 | 8.9절 윈도 (10,220,1022,655) |
| 코스(단위) | 단위 바 선택 시(op 3) 표시, 6곡 이상은 3초 교대 | 타이머 11 |
| 검색 입력 | 스킨에는 입력 이벤트가 없다. `search` 텍스트는 현재 검색어(ref 30) 표시만 한다. 입력은 엔진의 키보드(`0` 키 팝업: `B:select/MusicSelectInputProcessor.java:70-83`) | 영역 (50,895,418,30) |
| 도움말 | `helpBtn`(10,1050) 클릭 -> 열림, `helpScene` 전체 클릭 -> 닫힘. 효과음 open/close | 타이머 `isMenuOpen`/`not isMenuOpen` |
| 시작 애니메이션 | 장면 시작(타이머 없음, `loop=-1`) | 8.9 |
| 볼륨 외 효과음 | `Root/sounds/open.ogg`, `close.ogg` 만 | `main_state.audio_play(path)` |
| 선택 바 이동 효과 | 곡 이동 시 타이머 11 재설정 -> 제목/아티스트 150ms 이동, 스테이지파일 300ms 페이드, 스코어 막대 300ms, BPM 룰렛 200ms, 코스 교대 3초 주기 리셋 | `B:select/MusicSelector.java:612` |

---

## 10. 곡 목록 배치 3종, 언어 3종, 기타 옵션의 영향도

곡 목록 배치(`曲リストの並び`): 영향 범위는 `songlist.lua:28-34` 의 `X[i]` 17개 값뿐이다(5절 표). `liston`(중앙 바 x=1125)과 `selectmusic-frame` 위치는 모든 배치에서 동일하다. 직선에서 비중앙 바 x=1160, 곡선은 중앙에서 멀수록 최대 +163(1288), 사선은 위쪽이 오른쪽(+200)이고 아래쪽이 왼쪽(-200). 개수/객체 수는 변하지 않는다.

언어(`言語`): `changeLang()`(`musicselect.lua:13-28`)이 소스 id 3,4,5,6,10,11 의 경로를 `Select/parts/{jp|en|cn}/...` 로 바꾸는 것이 전부다. Lua 문자열이나 폰트는 언어 영향이 없다(모든 가시 문구는 시트 이미지에 구워져 있다). 세 폴더의 시트 크기는 동일하다: assistop 2000x2200, subop 2880x2300, mainframe 3200x3200, op 2000x3000, help 1920x1080, sidemenu 3700x1800. 언어 옵션이 일본어/영어/중국어가 아닌 값이면 `lang=nil` 로 문자열 결합 오류가 난다(3값 외 선택 불가).

기타: `画像フォント` ON 이면 폰트 id 0(`bartext.fnt`), 1(`main.fnt`, type 1), 2(`sub.fnt`, type 1)로 바뀌고 `bartext/title/artist/...` 텍스트의 font 참조와 `outlineColor/outlineWidth` 가 달라진다(`textproperty.lua:72-122`). `サイドメニューの開閉状態を保持する` ON 이면 시작 시 `Select/lua/settings/sidemenu` 를 읽어 4개 불리언을 복원하고 토글마다 다시 쓴다. `IR情報表示` 는 8.8. `スキン更新チェック`는 선곡에서 죽은 옵션.

---

## 11. 자산 (요구 절 8)

### 11.1 실제 참조되는 자산(기본 옵션)

| 경로 | 크기 | 용도 |
|---|---|---|
| `Select/parts/jp/mainframe.png` | 615,393 B (3200x3200) | source 5: 메인 프레임, 버튼, 숫자 시트, 모든 장식 |
| `Select/parts/jp/op.png` | 706,493 (2000x3000) | source 6: 옵션 패널1 |
| `Select/parts/jp/subop.png` | 239,916 (2880x2300) | source 4: 패널3 |
| `Select/parts/jp/assistop.png` | 208,067 (2000x2200) | source 3: 패널2 |
| `Select/parts/jp/sidemenu.png` | 196,407 (3700x1800) | source 11: 사이드메뉴 |
| `Select/parts/jp/help.png` | 598,092 (1920x1080) | source 10: 도움말 장면 |
| `Select/parts/songbar.png` | 103,369 (1050x1500) | source 7: 바, 램프, 라벨, 트로피, 레벨 숫자, 스크롤 |
| `Select/parts/scoreinfo.png` | 123,185 (1700x800) | source 8: 스코어창 |
| `Select/parts/qco.png` | 84,583 (1400x750) | source 9: IR 박스/이력/랭킹 |
| `Select/bg/image/#default.png` | 2,205,891 (1920x1080 RGB) | source 1 (정지화 배경, 기본) |
| `Select/font/ttf/mgenplus-1c-black.ttf` | 5,183,500 | 폰트 id 0 |
| `Select/font/ttf/mgenplus-1c-medium.ttf` | 5,246,920 | 폰트 id 1 |
| `Root/image/yuki.png` | 219,919 (600x610) | source `"char"` (기본 선택) |
| `Root/sounds/open.ogg`, `close.ogg` | 46,206 + 23,005 | 사이드메뉴/도움말 효과음 |

필수 최소 합계는 약 15.8 MB. 폰트는 ttf 모드에서만 로드된다. 와일드카드 파일 슬롯 3개: `Select/bg/image/*.png`(1개 .png), `Select/bg/movie/*.mp4`(2개), `Root/image/*.png`(7개). 선택값이 없을 때는 무작위(1.5절, `B:skin/SkinLoader.java:getPath`).

### 11.2 옵션별로만 필요한 자산

| 자산 | 조건 | 크기 |
|---|---|---|
| `Select/parts/en/*.png` (6장) | 언어 English | 2,639,920 B |
| `Select/parts/cn/*.png` (6장) | 언어 Chinese | 2,633,213 B |
| `Select/bg/movie/BGmovie01.mp4`, `BGmovie02.mp4` | 배경 동영상 | 21,231,110 + 24,991,514 |
| `Select/font/fnt/bartext.fnt`+png 5 | 이미지 폰트 | 약 10.3 MB |
| `Select/font/fnt/main.fnt`+png 11 | 〃 | 약 23.4 MB |
| `Select/font/fnt/sub.fnt`+png 7 | 〃 | 약 13.7 MB |
| `Root/image/{mesugaki, metan, tsumugi, zundamon, zundamon2, zundamon3}.png` | 캐릭터 파일 선택 | 합 1,102,147 B (yuki 제외) |
| `io/Select/bg/{image,movie}/` (빈 디렉터리), `History/*` | 배경 로테이션, 정보 출력(쓰기) | 런타임 파일 |
| `Select/lua/settings/sidemenu` | 사이드메뉴 보존 ON | 23 B, 읽기/쓰기 |

### 11.3 참조되지 않는 자산

`Select/parts/rival.png`(68,679), `Select/parts/dummy/-`(58), `Select/sounds/open.ogg|close.ogg`(`Root/sounds` 와 바이트 동일), `Select/font/ttf/mgenplus-1c-bold.ttf`(5,232,132), `Select/lua/settings/checkversion`(죽은 versioncheck 용), `Select/lua/require/http.lua`, `Select/lua/versioncheck.lua`, `Select/lua/volumecontrol.lua`, `Select/readme.txt`, 라이선스 txt. `Root/sounds/{change,click,enter,favorite,fullcombo,section*,vo/**}` 는 결과/플레이 화면용이며 선곡에서는 쓰이지 않는다. 시트 내부에 destination 이 없는 이미지 정의: `md-judge-*`(5), `bpm-roulette-4`, `loading`(info), `compare-frame-small`, `assist-keyflash`(`assistoption.lua:231-236` 주석), `btn-modeset`(imageset, destination 없음), `allkeys`, history OFF 시 `todayPlayCount*`, 값 정의만 있는 `ir_rank, ir_totalplayer, ir_clearrate`(info.lua:42-47).

---

## 12. 단순화 후보 (요구 절 9)

판정 기준: 화면 핵심(곡 목록, 선택 곡 정보, 점수/클리어, 버튼 열, 옵션 패널, 스크롤바)이 유지되는가, 그리고 R-BMS 가 지원해야 하는 Lua/엔진 기능을 줄여 주는가.

| 후보 | 제거 시 영향 | 이득 | 판정 |
|---|---|---|---|
| BPM 연동 캐릭터(`animationChar`) | 우측 중단 장식 소실 | image/dest 각 -1002, `draw` 함수 1000개, `Root/image` 1.3MB, `main_state.time/timer` 의존 | 제거 권장 |
| 배경 동영상 + 로테이션 | 동영상 배경 옵션 소실 | mp4 44MB, `luajava.File`, io 쓰기, `SkinSourceMovie` | 제거 권장(정지화 1장) |
| 이미지 폰트(bitmap fnt) | 폰트 옵션 소실 | 45MB, `Font.type=1` 처리 | 제거 권장 |
| 플레이 이력(`history.lua`) + `todayPlayCount*` | 이력 표시 소실 | `History/*` io 읽기, `customnumber` 로드 시 io | 제거 권장 |
| 버전 확인(versioncheck/http.lua, 스킨 업데이트 옵션) | 이미 죽은 코드 | `luajava.newInstance` 계열 | 제거 |
| `infoOutput` 파일 쓰기(`config.lua:222`, `musicselect.lua:59`) | OBS 용 텍스트 출력 소실 | 로드 시 io 쓰기, 실패 시 로드 전체 실패 위험 | 제거 필수(스킨 로드 안정성) |
| 사이드메뉴 보존 옵션(`settings.lua`) | 개폐 상태가 세션 간 유지 안 됨 | io 읽기/쓰기 | 제거 권장 |
| 언어 en/cn | 영어/중국어 시트 소실 | 5.3MB, 옵션 1개 | 제거(일본어 또는 영어 한 벌만) |
| 서브타이틀 스크롤 띠, 빔 장식, 시작 줌 고스트 3개 | 장식 소실 | dest 약 9 | 선택(유지해도 비용 낮음) |
| 사이드메뉴 | 상세 IR 현황/랭킹/볼륨/설정 소실 | 객체 약 800개, `timer_observe_boolean` 417 클로저, 슬라이더 4, 쓰기 act | 대폭 축소 권장(예: 설정+볼륨 2패널만, 또는 정적 구성) |
| IR 랭킹 모드(`qco` ranking 분기) | cfrate 박스만 남음 | dest -56 | 제거 가능 |
| 라이벌 비교(`rivalview`) | 라이벌 UI 소실 | dest 57, value 12 | 엔진이 라이벌 기능을 제공하지 않으면 제거 |
| 도움말 | 도움말 장면 소실 | 이미지 2, act 함수 2, 타이머 함수 2, 600KB | 선택 |
| 어시스트 패널(패널2)의 클릭 영역 7개 | 이 엔진에서 무반응 | dest 7, image 7 | 제거 |
| `volumecontrol.lua`, `Select/sounds`, `rival.png`, bold ttf, dummy | 이미 죽음 | 번들 크기 | 제거 |

반드시 유지(빼면 선곡 화면이 성립하지 않음): `songlist`(바 imageset 7종, 램프 11, 라벨, 트로피, 레벨 숫자, 클리어 분포 그래프, 중앙 바 프레임), 상/하단 프레임, 제목/아티스트/장르/디렉터리/BPM/폴더 곡 수, 스테이지파일+배너(참조 이미지 -100/-102), 버튼 열(모드/정렬/LN/오토/연습/리플레이), 스크롤바(슬라이더 type 1), 스코어창(EX/판정 분포/클리어 막대/랭크/점수율/타이밍), 선택 곡 정보창(노트 수/길이/TOTAL/플레이 횟수/판정 레벨), 분석창(노트 분포 그래프+BPM 그래프+밀도), 폴더 클리어 현황, 옵션 패널 1/2/3(키 입력으로 열림; 이미지만으로 구성되어 Lua 함수 불필요), 검색 표시, 곡 이동 애니메이션(타이머 11), 시작 연출(최소 한 종).

중요한 설계 제약: 기본 스킨판은 `dofile`, `luajava`, `io`, `os`, `timer_util` 에 의존하지 않도록 평탄한 Lua 테이블로 작성하는 편이 R-BMS 샌드박스와 맞다. `draw/timer/value` 함수 클로저는 위 "유지 필수" 목록에서는 `mainframe`(어시스트 경고, `isAssistOn`), `info`(TOTAL 색/게이지), `musicdisplay`(폴더 현황), `score`(타이밍 비율), `cource`(5/6곡 분기), `qco` 정도에만 필요하다. 이들은 모두 `main_state.option/number/text/event_index` 4개 API 와 단순 산술이다.

---

## 13. R-BMS 구현 체크리스트 (이 스킨을 수정 없이 읽으려면)

R-BMS 현황은 `crates/rbms-skin`, `crates/rbms-render/src/skin_render`, `apps/rbms-player/src` 를 grep 수준으로만 확인했다(미확인: 상세). 확인된 것: `songlist`/`imageset` 모델과 렌더(`songlist.rs`)가 있고, `mouseRect` 는 모델에만 있으며, `timer_observe_boolean`, `luajava`, `skin_config` 는 코드에 없다. `lua.rs` 에 `dofile` 이 등장한다(샌드박스 제거 목적으로 추정, 미확인).

로더/Lua:
1. 2단계 실행(헤더: `skin_config=nil`; 본로드: `skin_config` 존재)을 Lua 상태를 새로 만들어 각각 수행. 헤더 단계에서 `require("main_state"|"timer_util"|"event_util")` 는 빈 테이블로 성공.
2. 제공할 전역/모듈: `require`(점 구분 -> 스킨 상대 경로 `?.lua`), `dofile(절대경로)`, `pcall`, `print`, `ipairs/pairs/tostring/tonumber`, `table.insert`, `math.random/floor/ceil/max`, `string.gsub/match`, `os.date`, `io.open`(스킨 디렉터리 안의 읽기/쓰기/추가), `luajava.bindClass`(최소 스텁), `skin_config.{get_path, option, offset, file_path, enabled_options}`, `main_state.{option, number, text, event_index, time, timer, timer_off_value, volume_sys/key/bg, set_volume_*, audio_play}`, `timer_util.timer_observe_boolean`.
3. 값 변환: int 필드는 Lua 실수를 버림, `String` 필드는 숫자를 문자열화, `boolean` 필드는 Lua 진리값(0 도 true), 함수/숫자/문자열 3종 입력 지원(숫자 = 내장 id, 함수 = 매 프레임 호출).
4. destination: 한 항목 = 한 객체 인스턴스, 음수 숫자 id = 참조 이미지(-100,-102,-110,-111), 알 수 없는 id 무시, 키프레임 상속/기본값, 객체 단위 `acc/loop/timer/blend/filter/center`(첫 호출 채택), `loop=-1`, 단일 키프레임+`loop`, `mouseRect`, `offsets={40}`(알파), `draw` 함수 + `op` 정수 AND 결합, z 순서 = 배열 순서, 클릭은 z 역순 첫 `draw=true` 처리.
5. 객체: `image`(`divx/divy/cycle/timer/len/ref/act`), `imageset`(`ref/act`), `value`(10/11/12/24열 시트 규칙, `digit/align`, MIN_VALUE 숨김), `graph`(`angle`, 음수 폭 반전, `value` 숫자/함수, `type=-1` 분포), `slider`(`type` 1/8/17/18/19, `range`, `angle`, 드래그), `text`(ttf/bitmap, shadow/outline, overflow shrink, align, 함수 값), `songlist`(5절 전부), `judgegraph`, `bpmgraph`.
6. 엔진 속성: 6절 전체. 특히 (a) 380/390 계열 스코프 이중성, (b) 클리어 타입 인덱스 0..10, (c) 순위 슬롯 오프셋, (d) 폴더 클리어 카운트 11종, (e) 옵션 21/22/23 패널 + 타이머 21..23/31..33 전이 규칙, (f) 타이머 11(곡 이동), 173(IR 수신), (g) op 1100..1104/100..105(선택 바 클리어), 197/1197/1200/1203/1205..1208(리플레이), 175(TEXT), 625(비교), 1030(랜덤 선곡), 160..164/1160/1161(모드), 170/171, 176/177, 180..184, 200..207.
7. 이벤트: 6.4 표의 id 들(특히 11, 12, 16, 17, 19, 74, 75, 78, 79, 90, 308, 315..318, 321..324, 330..332, 340..342, 400, 40, 42, 43, 54, 55, 72). 301..307 클릭은 beatoraja 도 무동작.
8. 입력: START/SELECT/`5` 로 패널 1/2/3, 마우스 휠/드래그로 스크롤, 마우스 좌표(`getMouseX/Y`)를 스킨 좌표계로 제공.
9. 텍스처: 3200x3200, 3700x1800, 2880x2300 시트를 올릴 수 있어야 한다(GPU 최대 텍스처 크기 >= 4096 가정).

시각 동일성 검증 제안(전부 미실행): 고정 시각 t=0, 500, 1000, 1250, 3000ms 에서 (곡 바 선택, 폴더 바 선택, 단위 바 선택)x(스코어 있음/없음)x(패널 0/1/2/3)의 12~16 장면을 같은 데이터로 beatoraja 와 R-BMS 에서 캡처해 비교. 비교 단위는 8절의 사각형 표.

---

## 14. 위험

1. 로드 중단 위험: `musicselect.lua:59` 의 `infoOutput(5)` 는 `pcall` 밖에서 파일을 쓴다(`History/information.txt`). `io` 가 없거나 쓰기 실패면 스킨 전체가 로드되지 않는다. `Root/customfunction.lua:7-8` 의 `require("luajava")` 와 `Root/customnumber.lua:289-303` 의 `io.open` 도 같은 성격이다.
2. 조용한 누락: 18개 섹션 모두 `pcall(dofile)` 이므로 R-BMS 샌드박스에서 `dofile`/`timer_util`/`io` 하나가 빠지면 해당 섹션(예: sidemenu 전체)이 에러 없이 사라진다. 로드 로그를 반드시 남겨야 한다.
3. 성능: 기본값에서 매 프레임 Lua 클로저 약 1380개(char 1000 + sidemenu 276 + 기타) + 타이머 함수 약 420개를 호출한다. char 애니와 사이드메뉴 축소로 약 90% 제거 가능.
4. 수치 변환: 실수 -> int 버림, `a = 0` 진리값, NaN(`score.lua:10-11`, 0/0)이 구현에서 에러 또는 오표시를 일으킬 수 있다.
5. 스코프 이중성: id 380..399 가 value 와 image-index 에서 다르다. 한 맵으로 합치면 IR 랭킹 슬롯의 순위/클리어 표시가 틀어진다.
6. 클릭 시맨틱: 같은 id 의 호버 인스턴스(`mouseRect`)와 일반 인스턴스가 겹쳐 있다. z 역순 + `draw` 조건을 지키지 않으면 클릭이 반복/누락된다. 보이지 않는 1x1 이미지에 `act` 가 달린 영역(`assist-*SW`, `timingAdjustBtn`)도 영역 클릭이다.
7. 시간 기준: 곡 목록/시작 연출/스크롤바 등은 타이머 없이 "장면 시작으로부터의 ms"를 쓴다. 선곡 장면 재진입마다 0 으로 초기화돼야 한다(`B:select/MusicSelector.java:189-198`).
8. 파일 선택: 와일드카드 슬롯의 선택값은 확장자를 포함한 파일명이어야 하고 미선택이면 무작위다. 기본 선택은 설정 UI 가 `def` 로 정한다(런타임에서 `def` 를 읽지 않음).
9. 오버레이 순서: 서브타이틀 띠(`main-bg`)는 서브타이틀 텍스트 위에 그려진다(삽입 순서 그대로). 이 순서를 바꾸면 시각 결과가 달라진다.

---

## 15. 열린 질문 (미확인)

1. 곡 정보 텍스트(title y=570 등)가 곡 목록 바(중앙 y=505, 인접 y=575/645)와 같은 영역(x 1000~1920)에서 겹쳐 그려진다. 좌표는 소스 그대로이며 의도된 오버레이인지, 실제 beatoraja 화면에서 어떻게 보이는지 렌더로 확인하지 못했다.
2. beatoraja 쪽 참조는 사용자 로컬 포크이며(`LegacySkinLuaApi`, 샌드박스 IO 등이 추가됨) ModernChic 4.6 이 겨냥한 공식 beatoraja 와 일부 다를 수 있다. 예: 어시스트 패널 `act`(301..307) 처리기 없음, `luajava` 는 `File/URL/BufferedReader` 파사드만 지원.
3. `SkinText` 의 overflow/shadow/outline 렌더 세부와 `SkinSlider`, `SkinGraph`, 분포 그래프의 정확한 그리기(자르기, 방향)는 읽지 않았다(다른 조사자 범위). 선곡 스킨이 의존하는 사용 형태는 3절에 정리했다.
4. `StretchType`(기본 `STRETCH`)과 `filter` 의 실제 렌더 차이, `SkinImage` 의 `imageType`(`TYPE_NORMAL`) 기본 필터링은 읽지 않았다.
5. 객체 개수는 수동 전개 값이다(2.1절). 정확한 값이 필요하면 실제 Lua 실행(R-BMS 구현 후 덤프)으로 재집계해야 한다.
6. `Select/lua/require/http.lua` 41~175 줄과 `Select/lua/settings/checkversion` 내용은 읽지 않았다(죽은 코드/설정).
7. `bartext` 같은 한 `id` 를 11개 `songlist.text` 항목이 공유할 때 엔진이 각 항목을 독립 `SkinText` 로 만든다(`JsonSelectSkinObjectLoader.java:166-177`, `createText`). R-BMS 도 항목별 독립 인스턴스여야 하는지(폰트 캐시 공유 가능)는 구현 결정 사항이다.

---

## 16. 읽지 못한 파일/범위 요약

- 읽지 못함: `Select/lua/require/http.lua` 41~175줄, `Select/lua/settings/checkversion`(17 B), `Select/bg/movie/*.mp4`(바이너리), `Select/font/fnt/*.fnt`(1.3MB x3 텍스트 폰트 정의, 구조만 확인), `Root/sounds/vo/**`(선곡 미사용).
- 열어 본 PNG: jp `mainframe/op/sidemenu/help`, `songbar`, `scoreinfo`, `qco`, `#default.png`. 미열람: `subop.png`, `assistop.png`, en/cn 시트(크기만 확인), `Root/image/*.png`.
- 범위 밖: Result/Decide/Play/Course/KeyConfig/SkinSelect 스킨, beatoraja 의 SkinText/SkinSlider/SkinGraph 그리기 구현.

