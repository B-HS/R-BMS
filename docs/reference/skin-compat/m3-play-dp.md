# M3 조사 보고서: ModernChic 더블 플레이 스킨 (10키 / 14키)

> 최종 갱신 2026-10-09 · 대응 단계: L1 조사(구현 전) · 기준 커밋 `9ce92bb` · 색인과 갱신 규칙은 [README.md](README.md)

작성 근거는 전부 /Users/hyunseokbyun/Downloads/ModernChic 아래 파일이며 `파일:줄` 형식으로 표기한다. 경로 접두 `Play/lua/`는 `P/`, `Play/lua/dp/`는 `dp/`, `Play/lua/require/`는 `req/`로 줄여 쓰는 곳이 있다. beatoraja 쪽 근거는 /Users/hyunseokbyun/development/beatoraja/src/bms/player/beatoraja 아래이며 `bj:`(= 그 경로) 접두를 쓴다.

주의: 이 환경에는 Lua 인터프리터와 JRE가 없어 스킨을 실제로 실행하지 못했다. 객체 개수와 좌표는 전부 정적 분석(소스 전수 읽기 + 산술)으로 구한 값이며 "정적 추정"으로 표시한다. 실행 확인이 필요한 항목은 "미확인"으로 표시했다.

---

## 0. 한눈에 보는 결론

- DP 플레이 스킨은 `play14_hw.luaskin` / `play10_hw.luaskin` -> `play14_hw.lua` / `play10_hw.lua` -> `Play/lua/dp/*.lua` 15개 부품 + `Play/lua/background.lua`, `close.lua` 로 조립된다. 14키와 10키 본체 파일은 diff 기준 차이가 9곳뿐이다 (아래 10절).
- 각 부품은 `pcall(function() return dofile(path).load(...) end)` 로 감싸져 있어(`play14_hw.lua:47-255`, 파일당 17회, 두 파일 합 34회) 부품 하나가 Lua 오류를 내면 그 부품만 조용히 빠진다. 반면 본체 상단의 `require("Root.define2")` 체인은 pcall 밖이므로 여기서 실패하면 스킨 전체가 로드되지 않는다.
- 로드 실패의 1순위 원인은 `Root/customfunction.lua:7` 의 `require("luajava")` 와 `:8` 의 `luajava.bindClass("java.io.File")` 이다. 이 모듈은 `Root/define2.lua:33` 에서 항상 로드된다. `luajava` 스텁(최소 `bindClass`)이 없으면 DP/SP/선곡 등 모든 화면이 죽는다.
- 로드 시점에 파일 I/O 가 일어난다: `Root/customnumber.lua:289-303`(`countFileRecords` 9회 = io.open 읽기 시도 9회), `Root/customtime.lua:6-10`(os.date), `Play/lua/background.lua:29` 의 `CUSTOM.FUNC.infoOutput(0)`(History/information.txt 쓰기). 쓰기 실패 시 `f:close()` 에서 nil 호출 오류가 나고 `background.lua` 의 pcall 에 의해 배경 이미지와 배경 밝기 오버레이가 통째로 사라진다. R-BMS 는 쓰기 가능한 가짜 파일 핸들이 필요하다.
- `dp_property.lua` 의 `load()` 는 멱등이 아니다. `customoptionNumber`/`categoryNumber`/`offsetNumber` 가 모듈 지역 변수라 같은 Lua 상태에서 두 번 호출되면 op id 가 900번 대에서 밀린다(`req/dp_property.lua:12-14`). 스킨 파일 하나당 새 Lua 상태를 쓰거나 호출을 정확히 1회로 보장해야 한다.
- 좌표는 y 가 아래에서 위로 증가하는 1920x1080 이다(레인 이미지 y=224, h=856 이면 상단이 정확히 1080). 정수 필드(x,y,w,h,a,r,g,b,time,acc,angle)는 beatoraja 에서 `int` 이므로 Lua 의 `634.5` 같은 값은 truncation 된다(`bj:skin/json/JsonSkin.java` Animation 필드가 int).
- `loop = -1` 은 "마지막 프레임에서 정지"가 아니라 "끝나면 사라지는 1회 재생"이다(`bj:skin/SkinObject.java:prepareRegion`, `time > endtime` 이면 `time=-1` 로 만들어 `starttime > time` 으로 미표시). 판정 문자, 폭발, 키 플래시 전부 이 의미에 의존한다.
- 라이선스: `Play/readme.txt` 는 "本スキンそのものを二次配布することは禁止(허가 시 제외)", "改変スキンを公開する場合は原作者名(KASAKO)を記述" 라고 적는다. 기본 번들에 단순화본을 넣는 계획은 허가 또는 별도 자작 에셋 교체가 필요하다. 폰트 Mgen+ 는 `Play/SIL_Open_Font_License_1.1.txt` 가 동봉되어 재배포 가능성이 높다(미확인: 폰트 파일 자체의 저작권 표기).

---

## 1. 헤더

### 1.1 `.luaskin` 과 진입 구조

| 항목 | 값 | 근거 |
|---|---|---|
| 진입 파일 | `play14_hw.luaskin`, `play10_hw.luaskin` (각 6줄) | `play14_hw.luaskin:1-6` |
| 동작 | `local t = require("play14_hw")`; `skin_config` 가 있으면 `return t.main()`, 없으면 `return t.header` | 같은 파일 |
| 모듈 반환 | `return{ header = header, main = main }` | `play14_hw.lua:269-272` |
| 헤더 생성 | `require("Play.lua.require.header").load(2)` (14키) / `.load(3)` (10키) | `play14_hw.lua:13`, `play10_hw.lua:13` |
| 전역 변수 | 비 local 로 대입: `main_state`, `PROPERTY`, `COMMONFUNC`(상단), `MAIN`, `CUSTOM`, `CONFIG`, `BASE`(main 내부) | `play14_hw.lua:8-11,17-20` |
| 헤더 단계 순서 | 모듈 최상단에서 `PROPERTY = ...load(false)` 가 헤더 단계에서도 실행된다. `skin_config` 가 nil 이어도 안전해야 한다(옵션 판별 함수는 호출 시점에만 `skin_config` 를 읽음) | `req/dp_property.lua:50` |
| `load(arg)` 의 인자 | 14키 `false`, 10키 `true` = `is10keys`. 10키에서만 "10鍵用レーンカバー" 옵션과 "パーツ表示有無" 카테고리 구성이 달라짐 | `req/dp_property.lua:515-538` |

### 1.2 헤더 필드 (`Play/lua/require/header.lua:8-26`)

| 필드 | 값 | 비고 |
|---|---|---|
| type | 14키 = 2, 10키 = 3 | 주석(3줄)은 "2:12k"로 적혀 있으나 실제 의미는 DP 14키(= beatoraja SkinType PLAY_14KEYS). 번호 표: 0:7k 1:5k 2:14k 3:10k 4:9k 5:select 6:decide 7:result 15:course result 16:24k 17:24k double |
| name | `"ModernChicPlay(SCURO)-" .. 4.6` | `Root/version.lua:2` 가 `4.6`(float). 문자열 결합 결과 `ModernChicPlay(SCURO)-4.6` |
| w, h | 1920, 1080 | |
| loadend | 3500 | 로드 시작 후 최소 로딩 시간(ms) |
| playstart | 1000 | 로드 종료 후 곡 시작까지(ms), timer 40(READY)/41(PLAY) 간격 |
| scene | 3600000 | |
| input | 0 | 스킵 가능 timer 1 |
| close | 3000 | 실패(閉店) 연출 시간, timer 3(FAILED) |
| fadeout | 500 | 종료 페이드, timer 2(FADEOUT) |
| property / filepath / offset / category | `PROPERTY.property` 등 (1.3~1.6) | |
| author | `"KASAKO"` (`Root/author.lua:2`) | |
| 미지정 필드 | `judgetimer`(기본 1), `finishmargin`(기본 0) | `bj:skin/json/JsonSkin.java:19-20` |

### 1.3 property (커스텀 옵션) 전수

모든 op 번호는 `req/dp_property.lua` 의 `customoption.chiled` 호출 순서로 900부터 1씩 증가한다(10키/14키 동일, `chiled` 호출은 `load()` 안에서 항상 실행됨). 기본값은 `def` 가 `item.name` 문자열이고 숫자가 아니다. R-BMS 는 이름 일치로 기본 항목을 결정해야 한다. category 번호는 `parent`/`filepath`/`offset` 호출 순서로 1씩 증가한다(1~31 option, 32~51 filepath, 52~61 offset). 판별 함수는 `skin_config.option[parentName] == op` 이다(`req/dp_property.lua:49-51`, 키가 parent 의 `name` 문자열).

| # | property name | category | 항목 (op:이름) | 기본값 op | 판별 함수 | 줄 |
|---|---|---|---|---|---|---|
| 1 | ターゲット差分 | 1 | 900:非表示; 901:表示 | 900 | isDiffTargetOff/On | 80 |
| 2 | ターゲット差分の種類 | 2 | 902:目標ランク; 903:自己ベスト | 902 | isTargetRank/Mybest | 83 |
| 3 | ターゲット差分表示位置 | 3 | 904:TYPE-A(判定文字上); 905:TYPE-B（判定文字横・外側）; 906:TYPE-C（判定文字横・内側） | 904 | isDiffTargetTypeA/B/C | 86 |
| 4 | 判定タイミング | 4 | 907:非表示; 908:表示 | 907 | isJudgeTimingOff/On | 90 |
| 5 | 判定タイミングの種類 | 5 | 909:FAST/SLOW; 910:+-ms | 909 | isJudgeTimingWord/Ms | 93 |
| 6 | 判定タイミング表示位置 | 6 | 911:TYPE-A(判定文字上); 912:TYPE-B（判定文字横・外側）; 913:TYPE-C（判定文字横・内側） | 911 | isJudgeTimingTypeA/B/C | 96 |
| 7 | 判定タイミングボム | 7 | 914:無効; 915:使う（早いか遅いかでボムの色が変化します） | 914 | isJudgeTimingBombOff/On | 100 |
| 8 | ボムの種類 | 8 | 916:ModernChic規格; 917:OADX規格 | 916 | isModernChicBomb/isOADXBomb | 103 |
| 9 | グラフバー表示位置 | 9 | 918:左; 919:右; 920:無効 | 918 | isGraphPositionLeft/Right, isGraphareaNone | 106 |
| 10 | プレイ位置 | 10 | 921:左側表示; 922:中央表示; 923:右側表示 | 922 | isLeftPosition/isCenterPosition/isRightPosition | 110 |
| 11 | 画像フォント | 11 | 924:無効; 925:有効（高負荷） | 924 | isOutlineFont/isBitmapFont | 114 |
| 12 | グローランプ | 12 | 926:非表示; 927:表示 | 927 | isGlowlampOff/On | 117 |
| 13 | ゲージMAXインジケータ | 13 | 928:非表示; 929:表示 | 929 | isGaugeMaxIndicatorOff/On | 120 |
| 14 | ゲージ | 14 | 930:表示; 931:非表示 (off=표시, on=숨김. 이름과 Off/On 이 반대) | 930 | isGaugeCoverOff/On | 123 |
| 15 | ノート分布 | 15 | 932:表示; 933:非表示 (같은 반전) | 932 | isnotesDistributionCoverOff/On | 127 |
| 16 | ノート分布パターン | 16 | 934:判定; 935:FAST/SLOW; 936:ノート | 934 | isnotesDistributionTypeA/B/C | 130 |
| 17 | タイミンググラフ | 17 | 937:表示; 938:非表示 (반전) | 937 | isTiminggraphCoverOff/On | 135 |
| 18 | タイミンググラフ表示位置 | 18 | 939:BGA側; 940:プレイエリア側 | 939 | isTiminggraphDisplayTypeA/B | 138 |
| 19 | タイミンググラフ倍率 | 19 | 941:低倍率（+-225ms）; 942:標準倍率（+-150ms）; 943:高倍率（+-75ms） | 942 | isTiminggraphMagnificationLow/Normal/High | 141 |
| 20 | タイミンググラフ配色パターン | 20 | 944:通常; 945:赤基調; 946:緑基調; 947:青基調 | 944 | isTiminggraphColorNormal/Red/Green/Blue | 145 |
| 21 | ヒットエラービジュアライザーパターン | 21 | 948:通常; 949:三角; 950:強調 | 948 | isHitErrorVisualizerPatternNormal/Triangle/Emphasis | 150 |
| 22 | オートプレイ＆リプレイ時の案内 | 22 | 951:非表示; 952:表示 | 952 | isAutoplayInfoOff/On | 155 |
| 23 | 10鍵用レーンカバー（10鍵モード時のみ） | 23 | 953:非表示; 954:表示 | 954 (10키만 property 목록에 등록, `req/dp_property.lua:515-521`) | is10keyLanecoverOff/On | 158 |
| 24 | キービームの高さ | 24 | 955:100%; 956:90%; 957:80%; 958:70%; 959:60%; 960:50%（短い）; 961:40%; 962:30%（とても短い）; 963:20%; 964:10% | 960 | isBeamHeight100..10 | 161 |
| 25 | キービームの消失時間 | 25 | 965:通常; 966:短い; 967:長い | 965 | isBeamDisappearanceTimeNormal/Short/Long | 172 |
| 26 | キービームの消失パターン | 26 | 968:TYPE-L; 969:TYPE-B | 969 | isBeamDisappearanceTypeL/B | 176 |
| 27 | グルーヴゲージの向き | 27 | 970:右方向; 971:左方向 | 970 | isGaugeStretchDirectionRight/Left | 179 |
| 28 | BGA表示パターン | 28 | 972:1:1; 973:16:9; 974:無効 | 972 | isBgaPattern1_1/isBgaPattern16_9/isNoBGA | 182 |
| 29 | 汎用BGAの種類 | 29 | 975:動画; 976:画像; 977:無効 | 975 | isHanyoTypeMovie/Image, isHanyoDisable | 186 |
| 30 | 左右のレーンカバーをローテーション表示 | 30 | 978:無効; 979:有効（選択したレーンカバーは無視されます） | 978 | islanecoverRotationSwitchOff/On | 190 |
| 31 | 終了時にレーンカバーを下ろす | 31 | 980:無効; 981:有効 | 981 | isFinishCoverOff/On | 193 |

property 배열에 실제로 들어가는 개수: 14키 30개(# 23 제외), 10키 31개. 배열 순서는 `req/dp_property.lua:243-383` 이며 위 표와 다르다(예: 判定タイミングボム 뒤 ボムの種類, 그다음 グラフバー表示位置, プレイ位置, 画像フォント, タイミンググラフ倍率 ...). 순서가 UI 표시 순서에 영향을 줄 수 있으나 category 가 있어 영향은 작다(미확인).

`req/dp_property.lua:25-27` 은 op 번호가 999 를 넘으면 print 만 한다(현재 최대 981).

### 1.4 filepath (20개, `req/dp_property.lua:385-406`)

| category | name | path | def |
|---|---|---|---|
| 32 | 背景 | `Play/parts/common/bg/*.png` | `#default` |
| 33 | グラフバー用背景 | `Play/parts/dp_hw/graphbg/*.png` | `#default` |
| 34 | 汎用BGA（動画） | `Play/parts/common/BGA/movie/*.mp4` | `#default` |
| 35 | 汎用BGA（画像） | `Play/parts/common/BGA/image/*.png` | `#default` |
| 36 | ノーツ | `Play/parts/common/notes/*.png` | `#default` |
| 37 | 判定文字 | `Play/parts/common/judge/*.png` | `#default` |
| 38 | レーンカバー | `Play/parts/common/lanecover/*.png` | `#default` |
| 39 | リフトカバー | `Play/parts/dp_hw/lift/*.png` | `#default` |
| 40 | ボム（ModernChic規格） | `Play/parts/common/bomb/*.png` | `diamond SCUROed.` (파일명 `diamond SCUROed..png` 에서 확장자만 제거한 값) |
| 41 | ボム（OADX規格） | `Play/parts/common/oadx_bomb/*.png` | `DEFAULT` (해당 파일 없음. 폴더에는 `dummy.png` 만 있음) |
| 42 | フルコンボエフェクト | `Play/parts/common/fullcombo/*.png` | `#default` |
| 43 | キービーム | `Play/parts/common/keybeam/*.png` | `#default` |
| 44 | キーイメージ | `Play/parts/common/key/*.png` | `harf` |
| 45 | キーフラッシュ | `Play/parts/common/keyflash/*.png` | `#default` |
| 46 | 判定ライン色 | `Play/parts/common/judgeline/*.png` | `#default` |
| 47 | グローランプ（判定ライン上のやつ） | `Play/parts/common/glow/*.png` | `#default` |
| 48 | プログレスランプ（進捗バーのあれ） | `Play/parts/common/progress/*.png` | `#default` |
| 49 | ゲージMAXインジケータランプ | `Play/parts/common/lamp/*.png` | `#default` |
| 50 | ゲージ | `Play/parts/common/gauge/*.png` | `#default` |
| 51 | スクラッチイメージ | `Play/parts/common/scratch/*.png` | `#default` |

`POMYU Chara` 항목은 주석 처리(`req/dp_property.lua:405`). 이 path 문자열은 `Play/lua/base.lua:238-273` 의 `skin.source[].path` 와 문자 그대로 같아야 와일드카드가 선택된 파일로 치환된다(beatoraja 는 `filemap.put(customFile.path, selectedFilename)` 로 치환, `bj:skin/lua/LuaSkinLoader.java:load`). 대응: bg=source 3, graphBg=25, BGA 동영상/이미지=0(둘 중 하나), notes=6, judge=4, lanecover=17(회전 ON 시 18 도 같은 path), lift=19, bomb=11, oadx=28, fc=13, keybeam=14, key=15, keyflash=16, judgeline=5, glow=9, progress=10, lamp=24, gauge=29, scratch=30.

### 1.5 offset (10개, `req/dp_property.lua:410-421`, 선언 220-239)

axis 값은 `a = 0`, `h = 0` 처럼 숫자 0 이다. beatoraja 는 `JsonSkin.Offset` 의 x,y,w,h,r,a 가 boolean 이고 Lua 숫자 0 은 true 이므로 "키가 존재하면 해당 축 활성"이다(`bj:skin/json/JsonSkin.java:60-82`, `LuaSkinLoader` 의 `boolean.class -> LuaValue::toboolean`). R-BMS 도 "존재 = 활성, 값 무관"으로 읽어야 한다.

| id | name | 활성 축 | category | 사용처 |
|---|---|---|---|---|
| 40 | 背景の明るさ 0~255 (255で真っ暗になります) | a | 52 | `background.lua:23` offset=40 의 BLACK 오버레이 a 가산 |
| 41 | グラフエリア背景画像の明るさ 0~255 (255で真っ暗になります) | a | 53 | `dp/scorebar.lua:126` |
| 42 | BGAの明るさ 0~255 (255で真っ暗になります) | a | 54 | `dp/info.lua:175,222,270` (offsets) |
| 43 | ターゲット差分、判定タイミングの位置 | x, y | 55 | `dp/assist.lua` offsets 두번째 원소 (LIFT 와 함께) |
| 44 | ボムの大きさ 1~100%（範囲外は100%になります） | w | 56 | `dp/bomb.lua:47,53` 로드 시점에 `skin_config.offset[name].w` 읽음 |
| 45 | レーンの明るさ 0~255（255で真っ暗になります） | a | 57 | `dp/lane.lua:34-41` |
| 46 | 小節線の明るさ 0~255（255で見えなくなります） | a | 58 | `dp/notes.lua:143` 로드 시점에 `.a` 읽음 |
| 47 | 判定ラインの高さ（0以下はデフォルト値12になります） | h | 59 | `dp/lane.lua:57` 로드 시점에 `.h` 읽음 |
| 48 | グローランプの高さ（0以下はデフォルト値48になります） | h | 60 | `dp/lane.lua:56` |
| 49 | タイミンググラフの位置（プレイエリア側時に有効） | x, y | 61 | `dp/graph.lua:187,192` offsets |

런타임 의미(beatoraja): destination 의 `offset`/`offsets` 가 가리키는 값에 대해 `region.x += off.x - off.w/2`, `region.y += off.y - off.h/2`, `region.width += off.w`, `region.height += off.h`, 색 알파는 `a += off.a/255`, 각도는 `+= off.r`(`bj:skin/SkinObject.java:394-402, 485, 515, 531`). 로드 시점 읽기 오프셋(44,46,47,48)은 `.alpha()/.width()/.height()` 로 즉시 값이 굳는다. 값 0 이하 보정은 `req/common.lua:41-57`(12, 48), 폭발 크기는 `req/common.lua:26-39`(1~100 만 유효, 그 외 원본 크기), 마디선은 `req/common.lua:8-23`(0 또는 범위 밖이면 255, 1~255 면 255-값).

### 1.6 category (14개, `req/dp_property.lua:427-538`)

각 category 의 `item` 은 property/filepath/offset 의 category 번호(숫자 -> 문자열로 변환되어 비교)이다. `bj:skin/json/JSONSkinLoader.java:126-204` 가 `category.item[i].equals(pr.category)` 로 묶는다.

| 순서 | 이름 | item(번호) |
|---|---|---|
| 1 | メインオプション | 10(プレイ位置), 11(画像フォント), 31(終了時にレーンカバーを下ろす) |
| 2 | 背景 | 32, 52 |
| 3 | グラフエリア | 9(グラフバー表示位置), 33, 53 |
| 4 | BGA | 28, 29, 34, 35, 54 |
| 5 | ターゲットと判定タイミング | 1, 2, 3, 4, 5, 6, 55 |
| 6 | ゲージ | 14, 50, 27 |
| 7 | レーンカバー | 38, 30, 39 |
| 8 | キービーム | 43, 24, 25, 26 |
| 9 | ボム | 8, 7, 40, 41, 56 |
| 10 | ノート分布グラフ | 15, 16 |
| 11 | 判定タイミンググラフ | 17, 18, 19, 20, 21, 61 |
| 12 | パーツ表示有無 (`table.insert(module.category, 12, ...)`) | 14키: 12, 13, 22 / 10키: 12, 13, 22, 23 |
| 13 | パーツ選択 | 36, 37, 42, 44, 45, 46, 47, 48, 49, 51 |
| 14 | オフセット（位置、大きさ、明暗調整） | 57, 58, 59, 60 |

(위 표 순서가 최종 배열 순서다. "パーツ表示有無"는 `table.insert(module.category, 12, ...)` 로 12번째에 삽입되어 パーツ選択, オフセット 이 13, 14번째가 된다. 삽입 전 13개 + 삽입 1개 = 14개.)

---

## 2. 스킨 테이블 구성 (14키, 기본 옵션, 정적 추정)

기본 옵션 가정: playPosition=922(중앙), graph=918(좌), BGA=972(1:1), 汎用BGA=975(動画), 画像フォント=924(TTF), ボム=916, 判定タイミングボム=914, 폰트/보조 옵션 전부 기본, `config.lua` 기본값(valiableSUD.sw=true, alfa=100 / valiableJUDGE.sw=false / sectionScore.sw=false / notesAnimation=true / judgeAnimation=true / hcnBomb=false / liftDisplay=true / smallGauge=false / fcEffect=false / voice.play.sw=false / bpmLinkChar.sw=false / infoOutput=true, `config.lua:11-222`).

| 키 | 개수 | 대표 예 |
|---|---|---|
| source | 25 (BGA 동영상 조건 충족 시 +1 = id 0, 곡이 BGA 없음 + 汎用BGA=動画) | `{id=1, path="Play/parts/dp_hw/system.png"}`, `{id=3, path=".../bg/*.png"}`, `{id="hcnBomb", path=".../hcn/hcn.png"}` (`P/base.lua:237-274`) |
| font | 2 (TTF) 또는 3 (fnt) | `{id=0, path="Play/font/ttf/mgenplus-1c-black.ttf"}` (`req/textproperty.lua:25-26`) |
| text | 13 | `{id="title", font=1, size=25, ref=MAIN.STRING.FULLTITLE, overflow=1, align=1, shadowOffsetX=2, shadowOffsetY=2}` |
| image | 263 | 폭발 82, 노트 48, 정보 34, 점수막대 23, 게이지 22, 키플래시 9, close 9, 판정 6 ... |
| value | 63 | `{id="nowscore", src=1, x=1008, y=940, w=330, h=40, divx=11, divy=1, digit=6, ref=MAIN.NUM.POINT, zeropadding=1}` (`dp/info.lua:549`) |
| slider | 9 | `{id="progress", src=10, x=0, y=0, w=24, h=37, angle=MAIN.S_ANGLE.DOWN, range=773, type=MAIN.SLIDER.MUSIC_PROGRESS}` (`dp/progress.lua:14`), 레인커버 2 + adjusted 5 |
| graph | 7 | `{id="graph-now", src=22, x=580, y=0, w=78, h=523, divy=523, cycle=20000, type=MAIN.GRAPH.SCORERATE, angle=MAIN.G_ANGLE.DOWN}` (`dp/scorebar.lua:60`) |
| judge | 10 | `def1`, `def2`, `laneCoverRest1_1/1_2/2_1/2_2`, `constantRest1_1/1_2/2_1/2_2` (`dp/judge.lua:55-650`). 기본 옵션에서 destination 은 `def1`,`def2` 만 사용 (`:663-665`) |
| judgegraph | 2 | `{id="notesgraph", type=NOTES}`(`dp/prepare.lua:18`, destination 없음), `{id="judgegraph", type=JUDGE, backTexOff=0}`(`dp/graph.lua:119`) |
| bpmgraph | 1 | `{id="bpmgraph"}` (`dp/graph.lua:117`) |
| timingvisualizer | 1 | id `timing` (`dp/graph.lua:95-109`) |
| hiterrorvisualizer | 1 | id `hrv` (`dp/graph.lua:25-44`) |
| note | 1 | id `notes`. 키 목록과 값은 12절 |
| gauge | 1 | `{id=2001, parts=50, nodes={36개 id}}` (`dp/gauge.lua:180-199`) |
| hiddenCover | 2 | `{id="hiddenCoverLeft", src=1, x=0, y=0, w=1, h=1, disapearLine=227}` (`dp/cover.lua:20-21`) |
| liftCover | 2 | `{id="liftCoverLeft", src=19, x=0, y=0, w=513, h=853, disapearLine=227}` (`dp/cover.lua:16-17`) |
| bga | 1 객체 | `{id="bga"}` (`play14_hw.lua:35`) |
| pmchara | 14키만 `{}` (비어 있음) | `play14_hw.lua:42` |
| destination | 약 375 (정적 추정). 10키 약 357 | 부품별 추정: background 2, keyflash 34, info 74(BGA 있는 곡)~78, progress 17, lane 16, gauge 20, inputkey 34, notes 1, cover 31, judge 2, prepare 30, scorebar 33, assist 0, bomb 34, graph 10, fullcombo 6, close 30, 페이드 1 |

사용하지 않는 JsonSkin 키: imageset, floatvalue, gaugegraph, timingdistributiongraph, skinpreview, practice, songlist, customEvents, customTimers, skinSelect (`dp` 코드 전체 grep 0건). 즉 R-BMS 가 ModernChic DP 를 위해 최소한 지원해야 하는 객체 종류는: source, font, image, value, text, slider, graph, judge, judgegraph, bpmgraph, hiterrorvisualizer, timingvisualizer, note, gauge, hiddenCover, liftCover, bga, destination 이다.

정의됐지만 어떤 destination 에서도 쓰이지 않는(또는 주석 처리된 destination 에서만 쓰이는) 객체: `nowexscore`, `duration*` 9종(`dp/cover.lua:103-111`), `greennumberLanecoverOn/Off`, `greennumberMainbpmLanecoverOn/Off`(`dp/cover.lua:94-97`), `numDiffExscoreMybest`, `numDiffExscoreTarget`, `lamp_gaugeinclease`, `loadingwindow`, `loading_notesinfo`, `loading_bar`, `loading_barframe`, `notesgraph`, 텍스트 `difftbl`, `per`, 노트 `*-a` 11종(`dp/notes.lua:153,158,163,168,173,178,183,197,202,207,212`). 단순화본에서 제거 가능.

---

## 3. 객체 기능 사용 목록

### 3.1 destination 필드

| 필드 | 사용 값 | 대표 위치 |
|---|---|---|
| id | 문자열, 내장 음수 id(-110 BLACK, -111 WHITE, -101 BACKBMP), 숫자 문자열 `"2001"`(gauge) | `dp/gauge.lua:100`, `dp/prepare.lua:117`, `dp/scorebar.lua:183` |
| dst (키프레임) | `{time, x, y, w, h, acc, a, r, g, b, angle}`. 생략 필드는 이전 키프레임에서 상속, 첫 프레임 기본값 x/y/w/h=0, a/r/g/b=255, acc=0, angle=0 (`bj:skin/json/JSONSkinLoader.java:426-452`) | 전체 |
| dst 생략 | `{id="notes", offset=...}`, `{id="def1"}` 처럼 dst 없는 destination 이 존재(note, judge 는 자체 dst 보유) | `dp/notes.lua:290`, `dp/judge.lua:655-665` |
| time | 정수 ms. 다음 값 존재: 0, 500, 1000, 1500, 18000 등 | |
| timer (정수) | 40 READY(`dp/lane.lua:17`), 141 PREVIEW(`:29`), 140 RHYTHM(`:66`), 41 PLAY(`dp/info.lua:147`), 46/47 JUDGE(`dp/assist.lua:113`), 3 FAILED(`P/close.lua:30`), 143 ENDOFNOTE_1P(`dp/cover.lua:82`), 48 FULLCOMBO_1P(`dp/fullcombo.lua:22`), 100-117 KEYON, 120-137 KEYOFF(`dp/inputkey.lua:36-39`), 50-67 BOMB, 70-87 HOLD(`dp/bomb.lua:29-30`), 2 FADEOUT(`play14_hw.lua:260`, 리터럴 `2`; 10키는 `MAIN.TIMER.FADEOUT`) | |
| timer (함수) | `dp/cover.lua:147` 단 1곳(레인커버 로테이션 ON 일 때만). 반환값 없음(nil)이며 부수효과가 목적. beatoraja 는 `call().tolong()` 이라 nil 이 0 이 되어 타이머가 켜진 것으로 취급한다(`bj:skin/lua/SkinLuaAccessor.java:740`). 대상 이미지는 BLACK 1x1 a=0 이라 보이지 않음 | |
| loop | -1(1회 재생 후 소멸), 0(`dp/info.lua:28` 전체 반복), 양수 N(N 이후 구간 반복) | `bj:skin/SkinObject.java:prepareRegion` |
| blend | `MAIN.BLEND.ADDITION`(2): 폭발/글로우/키플래시/풀콤보, `ALPHA`(1): 키빔 | `dp/bomb.lua:172`, `dp/inputkey.lua:73` |
| filter | `FILTER.OFF`(0) 폭발/글로우/텍스트, `FILTER.ON`(1) 스크래치 회전 이미지 | `dp/keyflash.lua:117,122` |
| op | 정수 리스트, 음수 = NOT, 5절 전수 | |
| draw | Lua 함수(129곳). 4절 | |
| offset / offsets | 단일 `offset = N`(예: LIFT=3, 40) 또는 `offsets = {a, b}` | `dp/judge.lua:61`, `dp/lane.lua:34` |
| stretch | `FIT_INNER`(1) `dp/prepare.lua:117`, `FIT_OUTER_TRIMMED`(3) `dp/info.lua:135` 외, `FIT_WIDTH_TRIMMED`(5) `dp/info.lua:152`, `dp/keyflash.lua:132` | |
| acc | `ACCELERATION`(1), `DECELERATE`(2), `CONSTANT`(0). 첫 비0 acc 가 객체 전체 구간에 적용(`bj:skin/SkinObject.java:setDestination`, `if (this.acc == 0) this.acc = acc`) | `P/close.lua:31,45,51`, `dp/prepare.lua:36`, `dp/gauge.lua:114`, `dp/cover.lua:40`, `dp/keyflash.lua:133`, `dp/inputkey.lua:125` |
| 음수 w | 좌우 반전. 게이지(`dp/gauge.lua:96`: w=-500), 스크래치(`dp/keyflash.lua:123`), 판정 그래프(`dp/graph.lua:177,193`) | |
| r,g,b | 틴트: `dp/keyflash.lua:73-87`(무작위 + 100), `dp/info.lua:29`(난이도 색) 등 | |
| angle | `play10_hw.lua:159,166`(키프레임 값 180 / 0) | |
| 미사용 | `center`, `mouseRect`, `click`, `customTimers`, `customEvents`, `clip_*` | 0건 |

### 3.2 객체별 필드

| 객체 | 사용 필드 |
|---|---|
| image | id, src(정수 또는 문자열 id: `"hcnBomb"`, `"adjusted"`, `"char"`), x, y, w, h(`w=-1,h=-1` = 시트 전체, `dp/bomb.lua:97`), divx, divy, cycle, timer(정수, `dp/bomb.lua:101`), len+ref(버튼 ref: `dp/info.lua:516,517,543,544`), act(정수 `dp/graph.lua:155` 또는 함수 `dp/cover.lua:133`) |
| value | id, src, x,y,w,h, divx, divy, digit, ref 또는 value(함수), align(0 오른쪽, 1 왼쪽, 2 가운데; `MAIN.N_ALIGN`), zeropadding(0/1), cycle(`dp/judge.lua:30-35`). divx 10, 11(숫자 + 공백/기호 열), 12(+/- 포함) 사용, divy 2 는 음수/색 행 |
| text | id, font, size, ref 또는 value(함수 `preartist`), constantText, align(0/1/2; `MAIN.T_ALIGN`), overflow(1=SHRINK), shadowOffsetX/Y(2 또는 4), shadowColor `"00000000"`. wrapping/outline 미사용 |
| slider | id, src, x,y,w,h, angle(0 UP, 1 RIGHT, 2 DOWN, 3 LEFT), range, type(4 LANECOVER, 6 MUSIC_PROGRESS) 또는 value(함수) |
| graph | id, src, x,y,w,h, divy, cycle, angle(0 RIGHT, 1 DOWN; `loading_bar` 는 2), type 또는 value(함수 `gra_section`) |
| judge | id, index(0/1), images[], numbers[], shift=true. images/numbers 각 원소는 destination 형식(`{id, loop, timer, offsets, draw, dst}`) |
| note | 12절 |
| gauge | id, parts(50, smallGauge 시 100), nodes(36) |
| hiddenCover / liftCover | id, src, x,y,w,h, disapearLine=227 |
| judgegraph | id, type(0 NOTES, 1 JUDGE, 2 FASTSLOW), backTexOff(0 = OFF) |
| bpmgraph | id 만 |
| timingvisualizer | id, width=300, judgeWidthMillis(225/150/75), lineWidth=1, lineColor, centerColor, PGColor..PRColor, transparent=0, drawDecay=1 (`dp/graph.lua:95-109`). 색 문자열은 8자리 RRGGBBAA. 기본: lineColor `00FF00FF`, PG `00008855`, GR `00880055`, GD `88880055`, BD `88000055`, PR `00000055` |
| hiterrorvisualizer | id=`hrv`, width=300, judgeWidthMillis, lineWidth=1, colorMode=1, hiterrorMode=0, emaMode(1/2/3), lineColor `99CCFF80`, centerColor `FFFFFFFF`, PGColor `99CCFF80`, GRColor `F2CB3080`, GDColor `14CC8f80`, BDColor `FF1AB380`, PRColor `CC292980`, emaColor `FF0000FF`, windowLength=30, transparent=1, drawDecay=1 (`dp/graph.lua:25-44`) |
| bga | `{id="bga"}` |
| 호출 순서가 의미 있는 항목 | destination 배열 순서 = z 순서 |

---

## 4. Lua 함수가 값으로 들어가는 자리 전부

### 4.1 draw / timer / act / value 함수

| 위치 | 함수가 읽는 API / 부수효과 | 활성 조건(기본 옵션) |
|---|---|---|
| `P/background.lua:6-8` fcEffect 의 draw | `CUSTOM.SOUND.fcSound()` -> `main_state.timer(48)`, `main_state.audio_play(path, 2)` 1회, 항상 `false` 반환 | `CONFIG.play.fcEffect=true` 일 때만 (기본 false) |
| `P/background.lua:17` draw | `CUSTOM.SOUND.achievementVoice()` -> `main_state.option(A/AA/AAA)`, `timer(SCORE_BEST/TARGET)`, `audio_play`. 항상 `false` | `CONFIG.voice.play.sw=true` 일 때만 (기본 false) |
| `dp/cover.lua:37` liftCover draw | upvalue `isLiftDisplay` 반환 | 항상. 초기값 `CONFIG.play.liftDisplay=true` |
| `dp/cover.lua:133` image act | `changeLiftDisplay()` = upvalue 토글 | 1x1 이미지 `liftDisplaySwitch`(src 1, 0,900), destination `{x=laneL, y=227, w=1164, h=856}`. 영역 클릭 시 토글. 좌클릭 inc=+1, 우클릭 -1 이지만 함수는 인자 무시 |
| `dp/cover.lua:147` timer | `CUSTOM.OP.isTimerOn(143)` 후 `randomChoiceStep2` (파일 쓰기) | 레인커버 로테이션 ON(979)일 때만 |
| `dp/cover.lua:154-155` value | `CUSTOM.NUM.usedLaneCoverCountDP` / `allLaneCoverCountDP` (모듈 로드 시 계산된 정수) | 로테이션 ON |
| `dp/cover.lua:48-56` slider value | `CUSTOM.SLIDER.adjustedCover()[1..4]` -> `main_state.number`(LANECOVER1 14, LIFT1 314, NOWBPM 160, MAXBPM 90, MAINBPM 92, MINBPM 91), `option(272)`, `event_index(55)`. 이벤트 값이 2,3,4 가 아니면 `nil` 을 반환하므로 `[1]` 인덱싱에서 Lua 오류 | destination 의 op `{400(CONSTANT), 270 또는 177}` 이 충족될 때만 호출되어야 함. 따라서 value 함수는 객체가 그려질 때만 지연 평가해야 한다 |
| `dp/gauge.lua:79-81, 120` draw | `PROPERTY.isGaugeCoverOn()` and `CUSTOM.OP.isTimerOff(143)` | 게이지 숨김 옵션 931 일 때 true |
| `dp/gauge.lua:100,107` draw | `CUSTOM.OP.isBrinkGauge()` -> `main_state.gauge_type()`(4,5,7,8 이면 점멸) | 항상. 게이지 본체 2개 destination 중 하나가 선택됨 |
| `dp/assist.lua:151-181` draw | `CUSTOM.OP.isFirstPlay2()/isNotFirstPlay2()` -> `main_state.float_number(113)`, `option(32)` | 판정 차이 표시 ON(901)일 때만 destination 생성 |
| `dp/bomb.lua:244-266` draw | `CUSTOM.OP.isHcnPattern()` -> `event_index(308)`, `option(173)` | `CONFIG.play.hcnBomb=true` 일 때만 (기본 false) |
| `dp/info.lua:69-89` draw ×4 | `CUSTOM.OP.isRemainSec(60)` -> `number(163)*60+number(164) < 60` | 항상. 남은 시간 60초 미만이면 분/초 숫자 색 `{253,126,0}` |
| `dp/info.lua:113-117` draw | upvalue `pflg` 를 `option(1080)` 로 갱신하는 부수효과. 반환 nil | 항상. destination `{id=BLACK, dst={{x=0,y=0,w=0,h=0}}}` |
| `dp/info.lua:135,140,169,189,194,237,242` draw | `return pflg` / `return not pflg` | 항상(BGA 표시 / 연습용) |
| `dp/info.lua:369-371` draw ×9 | `main_state.gauge_type() == i - 1` | 항상(게이지 종류 이미지 선택) |
| `dp/info.lua:452-455` draw ×4 | `CUSTOM.sectionScore.*.gFlg` | `sectionScore.sw=true` 일 때만 |
| `dp/info.lua:458` draw | `CUSTOM.SOUND.sectionScoreEffect()` | 위와 같음 |
| `dp/info.lua:424-439` value ×16 | `CUSTOM.NUM.sectionScore1_4()[n]`, `sectionTime(n, 4)[n]` (`main_state.number`, `timer(41)`, `time()`) | `sectionScore.sw=true` 일 때만 |
| `dp/info.lua:451` graph value | `CUSTOM.GRAPH.sectionRemainRate()` | 위와 같음 |
| `dp/judge.lua:181-405` draw 96개 | `CUSTOM.OP.laneCoverRest(CONFIG.play.valiableJUDGE.rest)` 또는 `constantRest(...)` 또는 `not` 형 | `valiableJUDGE.sw=true` 일 때만 destination 이 참조됨 (기본 false) |
| `req/textproperty.lua:36,58` text value | `createFullArtist()` -> `main_state.text(15)`(SUBARTIST), `text(14)`(ARTIST) | 항상. id `preartist` |

### 4.2 함수로 계산되는 로드 시점 값 (함수가 값 자리에 들어가지는 않지만 로드 중 호출됨)

| 위치 | 호출 | 의미 |
|---|---|---|
| `P/base.lua:230,232,269,271` | `main_state.option(170)`(NO_BGA) | 곡에 BGA 가 없을 때만 BGA source id 0 을 추가. 곡마다 스킨이 다시 로드되어야 함 |
| `dp/info.lua:145,157,199,210,247,258` | `main_state.option(170)` | 汎用BGA 표시 destination 생성 여부 |
| `dp/info.lua:295,307` | `option(172)`, `option(173)` | LN 모드 버튼 / 이펙터 그림 선택 |
| `dp/info.lua:422` | `option(32)` | sectionScore |
| `P/close.lua:83-84` | `option(32)`, `option(-290)` | 실패 화면 퀵리트라이 안내. 음수 id(NOT) 를 `main_state.option` 에 넘김 |
| `dp/judge.lua:654,659` | `option(-400)`, `option(400)` | 판정 변형 선택 |
| `req/common.lua:105-125` | `option(150..155)` | 제목 색 (난이도) |
| `dp/info.lua:5` | `COMMONFUNC.setRGB()` | 로드 시 1회 |
| `dp/keyflash.lua:32-34` | `CUSTOM.NUM.randNum(50,155)` ×3 | 곡을 열 때마다 스크래치 틴트 색이 달라짐 |
| `P/background.lua:29` | `CUSTOM.FUNC.infoOutput(0)` | `History/information.txt` 쓰기 |

### 4.3 beatoraja 의 Lua 함수 평가 의미 (구현 시 동일하게 맞출 것)

근거 `bj:skin/lua/SkinLuaAccessor.java:611-700,735-747`.

| 자리 | 변환 | 예외(오류) 시 |
|---|---|---|
| draw / op 대체 함수 (BooleanProperty) | `function.call().toboolean()` (nil, false 만 false) | false (해당 destination 미표시), 경고 로그 |
| value (IntegerProperty) | `call().toint()` | 0 |
| slider/graph value (FloatProperty) | `call().tofloat()` | 0 |
| text value (StringProperty) | `call().tojstring()` | 빈 문자열 |
| timer (TimerProperty) | `call().tolong()` (마이크로초) | `Long.MIN_VALUE` (꺼짐) |
| act (Event, 인자 0개 함수) | `function.call()` | 예외 흡수 후 경고 |

평가 순서(`bj:skin/SkinObject.java:setDrawCondition/prepare`): destination 의 `op` 에서 BooleanPropertyFactory 가 해석하는 id 는 draw 조건 목록 앞쪽에, `draw` 함수는 뒤쪽에 들어가고 `prepare` 가 앞에서부터 순서대로 평가해 하나라도 false 면 중단한다. 따라서 `draw` 함수는 op 조건이 통과했을 때만 매 프레임 호출되고(부수효과가 있어도 그때만 실행), slider/value 함수는 그리기 단계에서만 호출된다. 호출 횟수는 프레임당 1회 이상이므로 `dp/info.lua:113` 의 `pflg` 같은 상태 갱신 draw 는 op 가 없는 destination 에 붙어 있어 항상 매 프레임 실행된다.

### 4.4 customTimers / customEvents

사용 0건.

---

## 5. 참조하는 속성 id 전수

(`MAIN.*` 이름 -> id 는 `Root/main*.lua` 에서 해석. 사용 횟수와 줄은 `dp/*`, `P/*`, `req/*` 전수 grep 결과. 줄 번호는 첫 4개까지)

### 5.1 옵션 (destination op, 음수 = NOT, 로드 시 option() 호출)

| id | 이름 | 의미 | 쓰는 곳 |
|---|---|---|---|
| 32 | AUTOPLAYOFF | 오토플레이 아님 | background:15; close:83; assist(OP 6곳); info:422; inputkey:86,95; keyflash:79 |
| 33 | AUTOPLAYON | 오토플레이 | inputkey:104,110,169,175; keyflash:86; prepare:156 (+ 숫자 리터럴 `{33}` `dp/inputkey.lua:139,145`) |
| 43 | GAUGE_HARD | 하드계열 게이지(1P) | gauge:22,27,33,38,48,54 |
| 80 | NOW_LOADING | 로딩 중 | prepare 20곳; progress:31,77 |
| 81 | LOADED | 로딩 완료 | info:147,152,201,206; keyflash:79,86; prepare:43,156,163; progress:39,46,53,60,67,85 |
| 84 | REPLAY_PLAYING | 리플레이 | prepare:163 |
| 150-155 | DIFFICULTY0..5 | 난이도 unknown, beginner, normal, hyper, another, insane | info:351; common:108-118 |
| 170 | NO_BGA | BGA 없는 곡 | base:230,232,269,271; info 12곳 |
| 172 / 173 | NO_LN / LN | LN 포함 여부 | info:295 / info:307 (+ customoption:53,57,61) |
| 176 / 177 | NO_BPMCHANGE / BPMCHANGE | BPM 변화 없음/있음 | info:470,478 / cover:65,68,71,122,123,124; info:403,413 |
| 180-184 | JUDGE_VERYHARD..VERYEASY | 판정 난이도 | scorebar:108 |
| 195 | BACKBMP | backbmp 이미지 있음 | prepare:117 |
| 200-207 | AAA_1P..F_1P | 현재 랭크 | scorebar:203 |
| 220-222 | AAA, AA, A | 랭크 도달 플래그(막대 색 전환) | scorebar:149-176 |
| 230-232 | GAUGE_1P_0_9, 10_19, 20_29 | 게이지 구간 (숫자 리터럴 `{232,231,230}` 도 사용 `dp/gauge.lua:45`) | gauge:33,38,45 |
| 240 | GAUGE_1P_100 | 게이지 100% | info:335 |
| 270 | LANECOVER1_CHANGING | 레인커버 조작 중 | cover:118-128; prepare:12,35 |
| 271 | LANECOVER1_ON | 레인커버 사용 | cover:60 |
| 290 | MODE_COURSE | 코스 | close:84 |
| 400 | CONSTANT | HS-FIX 모드 | cover:39,60,65,68,71; judge:654,659 |
| 1080 | STATE_PRACTICE | 연습 모드 | info:115,147,152,201,206; prepare:177,182 |
| 1242 / 1243 | EARLY_1P / LATE_1P | FAST/SLOW 1P | assist:119,120; bomb:190,207,224 |
| 1262 / 1263 | EARLY_2P / LATE_2P | FAST/SLOW 2P | assist; bomb:197,214,231 |
| 2243 / 2244 / 2245 | GOOD_EXIST / BAD_EXIST / POOR_EXIST | 해당 판정이 한 번이라도 있었음 | gauge:63-73; progress:31-67 |
| 900-981 | 커스텀 옵션 | destination op 로는 쓰이지 않음. 전부 Lua 로드 시 분기용 | 1.3 |

### 5.2 숫자 (value ref, 로드/런타임 `main_state.number`)

| id | 이름 | 쓰는 곳 |
|---|---|---|
| 12 | JUDGETIMING | graph:156 (클릭으로 조정, 아래 act 74) |
| 14 | LANECOVER1 | cover:113; customoption:196,217; customslider:10 |
| 74 | TOTALNOTES | info:567 |
| 75 | MAXCOMBO | judge:30-50 (콤보 숫자) |
| 90 / 91 / 92 | MAXBPM / MINBPM / MAINBPM | info:556,560; customslider |
| 96 | PLAYLEVEL | info:565 |
| 100 | POINT | info:549 (6자리 zeropadding) |
| 101 | SCORE2 (EX) | info:553; scorebar:48 |
| 105 | MAXCOMBO2 | info:551 |
| 107 / 407 | GROOVEGAUGE / _AFTERDOT | gauge:175,177 |
| 110-114 | PERFECT, GREAT, GOOD, BAD, POOR | info:569-583 |
| 121 | TARGET_SCORE | scorebar:50 |
| 152 / 153 | DIFF_HIGHSCORE / DIFF_TARGETSCORE | assist:98,100; scorebar:53,55 |
| 160 | NOWBPM | info:558 |
| 163 / 164 | TIMELEFT_MINUTE / SECOND | info:562,563 |
| 165 | LOADING_PROGRESS | prepare:26 |
| 312, 313, 314 | DURATION, DURATION_GREEN, LIFT1 | cover:103,93,115 |
| 350-353 | TOTALNOTE_NORMAL, LN, SCRATCH, BSS | prepare:27-30 |
| 412-419 | EARLY/LATE_GREAT, GOOD, BAD, POOR | info:572-585 |
| 525 / 526 | JUDGE_1P_DURATION / 2P | assist:102,103 |
| 1312-1327 | `*_DURATION*LANECOVER_*` 계열 14종 | cover:93-111 (사용되는 것은 `greennumber`(313), `greennumberMinbpmLanecoverOn`(1321), `greennumberMaxbpmLanecoverOn`(1325), `lanecoverNumber`(14), `liftNumber`(314) 뿐) |

런타임 Lua 에서 `main_state.number()` 로 읽는 id: TIMELEFT_MINUTE/SECOND, LANECOVER1, LIFT1, NOWBPM, MAXBPM, MINBPM, MAINBPM (`Root/customoption.lua:159,196-217`, `Root/customslider.lua:10-44`), GROOVEGAUGE(`Root/customsound.lua`, DP 에서는 호출 안 됨), DENSITY_*, TIME_*(infoOutput: 기본 코드 경로에서는 flg=0 만 사용하므로 TOTALNOTES, DENSITY_PEAK/AVERAGE, TIMELEFT_* `Root/customfunction.lua:328-334`).

### 5.3 타이머

| id | 이름 | 비고 |
|---|---|---|
| 2 FADEOUT, 3 FAILED | 페이드/실패 | `play14_hw.lua:260`, `P/close.lua` 12곳 |
| 40 READY, 41 PLAY | 로드 완료 / 곡 시작 | lane:17,23; progress:39,85; info:147...; graph:187 |
| 42 GAUGE_INCLEASE_1P, 44 GAUGE_MAX_1P | 게이지 상승 / MAX | info:342,521-523 |
| 46 / 47 JUDGE_1P/2P | 판정 | judge 120곳, assist |
| 48 FULLCOMBO_1P | 풀콤보 | fullcombo:10,22,29,37 |
| 50-57, 60-67 | BOMB_1P/2P SCRATCH, KEY1..7 | bomb:29,34,61 (14키는 KEY1..7, 10키는 KEY1..5) |
| 70-77, 80-87 | HOLD_1P/2P | bomb:30,36,62 |
| 100-107, 110-117 | KEYON_1P/2P | inputkey:36-48; keyflash:16-23; base:41,50 |
| 120-127, 130-137 | KEYOFF_1P/2P | inputkey:38-50 |
| 140 RHYTHM | 박자 | gauge, info, lane:66, progress |
| 141 PREVIEW | 프리뷰 | lane:29,31,43,48 |
| 143 ENDOFNOTE_1P | 마지막 노트 이후 | cover:82,148,157,162; gauge:80,120 |
| 351 SCORE_BEST, 352 SCORE_TARGET, 11 SONGBAR_CHANGE, 172/173 IR_CONNECT_* | 런타임 Lua 만 | `Root/customsound.lua:151,155`, `Root/customoption.lua:100`, `Root/customnumber.lua:104` (DP 기본 경로 비활성) |

### 5.4 이벤트(버튼) / 오프셋 / 그래프 / 슬라이더 / 문자열 / 이미지

| 종류 | id | 이름 | 쓰는 곳 |
|---|---|---|---|
| 버튼 ref (image ref, 프레임 선택) | 42 / 43 | RANDOM_1P / 2P | info:543,544 (len=10) |
| | 308 | LNMODE | info:516,517 (len=3) + `event_index` |
| 버튼 act | 74 | JUDGE_TIMING | graph:155 |
| event_index | 55 HSFIX, 301-307 ASSIST_*, 308 LNMODE | `Root/customoption.lua:55,140-146,232`, `Root/customslider.lua:25` |
| 오프셋 | 3 LIFT(139곳), 4 LANECOVER(5), 30 NOTES_1P, 32 JUDGE_1P(120), 1/2 SCRATCHANGLE_1P/2P (keyflash:117,122), 40-49 커스텀 | |
| 그래프 type | 102 LOAD_PROGRESS(prepare:19, 사용처 없음), 110 SCORERATE, 111 SCORERATE_FINAL, 112 BESTSCORERATE_NOW, 113 BESTSCORERATE, 114 TARGETSCORERATE_NOW, 115 TARGETSCORERATE | scorebar:60-67 |
| 슬라이더 type | 4 LANECOVER (cover:25,26,30,31), 6 MUSIC_PROGRESS (progress:14,15) | |
| 문자열 ref | 1 RIVAL, 12 FULLTITLE, 13 GENRE, 14 ARTIST, 15 SUBARTIST(함수 안), 1003 TABLE_FULL(`difftbl`, 미사용) | `req/textproperty.lua` |
| 이미지 id | -110 BLACK(18곳), -111 WHITE(scorebar:183), -101 BACKBMP(prepare:117) | |
| 소스 숫자 id | 0(BGA), 1(system), 2(lane), 3(bg), 4(judge), 5(judgeline), 6(notes), 9(glow), 10(progress), 11(bomb), 13(fullcombo), 14(keybeam), 15(key), 16(keyflash), 17/18(lanecover), 19(lift), 21(close), 22(score), 24(lamp), 25(graphbg), 26(mine), 28(oadx bomb), 29(gauge), 30(scratch), `"hcnBomb"`, `"adjusted"`, `"char"` | `P/base.lua:237-274` |

Root 쪽 custom* 가 DP 런타임에서 참조하는 나머지 id: OPTION 150-155, 173(LN), 180-184, 272(LIFT1_ON), 32(AUTOPLAYOFF), `GRADEBAR`(3), `RESULT_*`(결과 화면용 비활성), BUTTON 55,301-308, GRAPH 113(BESTSCORERATE), STRING 12/13/14, 150/155(COURSE*_TITLE, 코스 판정 `isCourse`)

---

## 6. Lua 런타임 의존

### 6.1 main_state

| 함수 | 호출 횟수 (ModernChic 전체 중 DP 경로 관련 파일) | 비고 |
|---|---|---|
| `main_state.number(id)` | 110 | 정수(LuaInteger). `Root/customfunction.lua`, `customnumber.lua`, `customoption.lua`, `customslider.lua`, dp 일부 |
| `main_state.option(id)` | 82 | 음수 id 허용 (`-MAIN.OP.MODE_COURSE`, `-MAIN.OP.CONSTANT`). 존재하지 않는 id 는 false |
| `main_state.audio_play(path, vol)` | 82 | vol 은 0..2 클램프, nil 이면 1. 프리로드는 vol=0.0001. DP 기본 경로에서 호출되지 않음(비활성) |
| `main_state.text(id)` | 15 | 문자열, 없으면 `""` |
| `main_state.event_index(id)` | 12 | 버튼 현재 인덱스 (HSFIX 0..4, LNMODE 0..2, ASSIST 0/1) |
| `main_state.gauge_type()` | 10 | 0 assist-easy, 1 easy, 2 normal, 3 hard, 4 exhard, 5 hazard, 6..8 class 계열. beatoraja 는 double 반환 (`bj:skin/lua/MainStatePropertyLuaApiExporter.java`) |
| `main_state.timer(id)` | 7 | 마이크로초 단위 raw 값, 꺼짐은 `main_state.timer_off_value` (Long.MIN_VALUE) |
| `main_state.time()` | 2 | 마이크로초. `Root/customoption.lua:100`(선곡), `customnumber.lua:282` 에서 `/1000000` 로 초 환산 |
| `main_state.judge(n)` | 6 | `Root/customnumber.lua:49-69` (기본 DP 경로 비활성) |
| `main_state.float_number(id)` | 2 | `Root/customoption.lua:92,96` (BESTSCORERATE 113) |
| `main_state.volume_sys/key/bg()`, `rate()`, `gauge()`, `exscore()` | 각 1 | `Root/customnumber.lua:25-46` (비활성) |
| `main_state.timer_off_value` | 5 | |
| 미사용 | `timer_util`, `event_util`, `set_timer`, `event_exec`, `numbers`, `key_pressed` 등 | 0건 |

### 6.2 skin_config

| 사용 | 위치 | 비고 |
|---|---|---|
| `skin_config.get_path(rel)` | 135회 (`play14_hw.lua` 각 부품, `Root/customnumber.lua:289-303`, `Root/customfunction.lua`) | 스킨 폴더 기준 절대 경로 문자열 반환. `dofile` 과 `io.open` 에 그대로 사용되므로 OS 절대 경로여야 한다. 존재하지 않는 경로도 문자열은 반환 |
| `skin_config.option[parentName]` | 1곳(`req/dp_property.lua:50`) + 옵션 판별 함수 전부 | 키는 property 의 name(일본어 문자열), 값은 선택된 op 정수 |
| `skin_config.offset[name].{a,h,w,x,y}` | 5곳(`req/dp_property.lua:69-73`) | 값 정수 (기본 0) |
| `skin_config.file_path`, `enabled_options` | 0회 | |

`bj:skin/lua/SkinLuaAccessor.java:931-980` 가 위 구조를 만든다(`offset[name]` 은 x,y,w,h,r,a 6개 모두 존재).

### 6.3 표준 라이브러리

| 라이브러리 | 사용 | 위치 |
|---|---|---|
| `table.insert` | 530회. 위치 지정 삽입 1회 `table.insert(module.category, 12, {...})` | 전체; `req/dp_property.lua:522,531` |
| `ipairs`, `pairs` | `Root/define2.lua:9-20` (`ADD_ALL`, `LOAD_HEADER`) | |
| `math.random` | 4회 | `Root/customnumber.lua:73`(randNum), `Root/customfunction.lua:233,289`(로테이션), `Root/customtext.lua:33`(선곡) |
| `string.gsub`, `string.match` | `Root/customfunction.lua:124-125` (로테이션 경로) | 패턴 `"Play.+%.png"` |
| `os.date` | `Root/customtime.lua:6,8,10` | 로드 시 항상 실행 |
| `io.open` (r/w/a), `file:lines()`, `file:write()`, `file:close()` | `Root/customfunction.lua` 다수, 로드 시 `Root/customnumber.lua:289-303` 가 읽기 시도 9회, `P/background.lua:29` 가 쓰기 | 쓰기 경로: `History/information.txt` |
| `luajava` | `Root/customfunction.lua:7-8,103-105,112-121` | `bindClass("java.io.File")` 는 로드 시 항상, `new(File, path)`, `:mkdir()`, `:listFiles()`, `#filelists` 는 로테이션 경로에서만 |
| `require` | 모듈명: `main_state`, `Play.lua.require.{dp_property,sp_property,common,header,textproperty}`, `Play.lua.base`, `Root.{define,define2,version,author,mainoption,mainnumber,maintimer,mainstring,mainbutton,mainoffset,maingraph,mainslider,mainimage,customoption,customnumber,customgraph,customslider,customfunction,customtime,customtext,customsound,customtimer}`, `config`, `luajava` | `.` -> `/`, 스킨 루트 기준 `?.lua` |
| `pcall`, `dofile` | 파일당 각 17회 (`play14_hw.lua`, `play10_hw.lua` 합 34회) | `dofile(skin_config.get_path(...))` |
| `print` | `DEBUG` 가드 또는 로테이션 경로 `print("CF:ALL_EXCLUDE_INIT")` 등, `req/dp_property.lua:26,541` | 출력만 |
| 전역 변수 | `DEBUG`(미정의 nil 읽음), 위 1절 | |
| 미사용 | `package.*`, `debug.*`, `loadfile`, `loadstring`, `setfenv`, `getfenv`, `unpack`, `bit32`, `string.gfind`, `table.getn`, `math.pow`, `goto`, `//` | grep 0건 |

### 6.4 LuaJ(5.1/5.2) 전용 / 5.4(mlua) 주의점

- 5.1 전용 문법·함수 사용 없음.
- `#info.init / 2` 는 5.4 에서 float 이지만 for 루프 변수는 정수로 유지되어 `info.initLeft[i]` 인덱싱이 안전하다(`dp/bomb.lua:169,187,204,221,241,258,274`).
- 숫자 -> 문자열 결합은 정수(`"section" ..i .."-4_min"`, `dp/info.lua:444-447`)에서만 일어난다. 이 루프 변수가 float 이면 `"section1.0-4_min"` 이 되어 id 불일치가 난다. 5.4 에서는 `for i = 1, num, 1`(정수 시작/정수 step) 이라 안전.
- id 정규화: 게이지 객체 `id = 2001`(정수, `dp/gauge.lua:181`)과 destination `id = "2001"`(문자열, `:100,107`)이 섞여 있다. beatoraja 는 `String.class -> tojstring` 으로 통일한다. R-BMS 는 모든 id/src 를 문자열로 정규화해야 한다. source id 도 정수(`1`)와 문자열(`"hcnBomb"`)이 섞임.
- 소수 좌표: `(posx + 10) + 311 / 2 = 175.5`, `laneL + (513/2) = 634.5`(`dp/lane.lua:44`), `bombWidth / 2`, `info.bombHeight / 2` 등. beatoraja 는 `int` 로 truncation(Java `(int)` 캐스트).
- `a = notesInfo.barlineBright` 정수 0..255, `r = red + 100`(50..155 + 100 = 150..255).
- `req/dp_property.lua` 의 `load()` 비멱등성(0절 참고).
- `skin_config.option[...] == num` 비교는 정수끼리여야 한다(LuaJ 와 mlua 모두 문제 없음. 문자열로 오면 항상 false).
- `Root/customoption.lua:100` 등은 `main_state.time()` 이 마이크로초라고 가정한다(`/ 1000000`).
- `main_state.timer(id)` 반환값이 꺼져 있을 때 `main_state.timer_off_value` 와 `==` 비교(`Root/customoption.lua:45,49`). 같은 sentinel 이어야 한다.

---

## 7. 1920x1080 레이아웃 (14키, 기본 옵션)

좌표계: 원점 좌하단, y 위쪽 증가, 단위 px. 표기 `(x, y, w, h)`. 아래 값은 `BASE.laneLeftPosX = L`, `BASE.laneRightPosX = R = L + 645`, `BASE.NOTES_JUDGE_Y = 227`, `BASE.LANE_LENGTH = 853`(`P/base.lua:161-189`).

### 7.1 playPosition 별 기준 (`P/base.lua:164-173`)

| 옵션 | L | R | subPosX {좌열, 우열} | 레인 전체 폭 |
|---|---|---|---|---|
| 922 중앙(기본) | 378 | 1023 | {10, 1580} | 378..1542 (1164) |
| 921 좌측 | 40 | 685 | {1240, 1580} | 40..1204 |
| 923 우측 | 720 | 1365 | {10, 350} | 720..1884 |

graph 위치: 918(좌) 이면 `sectionScoreFrame = {x=subPosX[2], y=790}`, charAnimation `{subPosX[2]+70, 290}`; 919(우) 이면 `sectionScoreFrame.x = subPosX[1]`; 920(없음) 도 `subPosX[2]`(`P/base.lua:178-187`).

### 7.2 중앙 레이아웃 영역 (L=378, R=1023)

| 영역 | 사각형 | 근거 |
|---|---|---|
| 배경 | (0, 0, 1920, 1080), 위에 BLACK 오버레이 같은 크기, offset 40 의 a | `P/background.lua:20-23` |
| 1P 레인(이미지) | (L, 224, 519, 856) = (378, 224, 519, 856) | `dp/lane.lua:13`, 소스 `lane.png`(520x900) (0,0,519,856) |
| 2P 레인 | (R, 224, 519, 856) = (1023, 224, 519, 856) | |
| 레인 안쪽 구분선 이미지 | `lane_1P` (L+3, 227, 513, 853)(READY 타이머 1000ms 동안 h 1 -> 853), `lane_2P` (R+3, ...) | `dp/lane.lua:16-31` |
| 레인 밝기 오버레이 | BLACK, (L+3, 227, 513, 854) 및 (R+3, ...), offset 45 의 a | `dp/lane.lua:33-42` |
| 중앙 패널 | centerFrame (L+519, 180, 126, 900) = (897, 180, 126, 900), 소스 (0,0,8,900) 을 126 폭으로 신장 | `dp/info.lua:7-11,495` |
| 하단 패널 | bottomFrame (L, 0, 1164, 180), 소스 (30,758,1164,180) | `dp/info.lua:12-16,496` |
| 키보드 프레임 | keyBg/keyFrame 1P (L+111, 110, 408, 114), 2P (R, 110, 408, 114); scratchBg/Frame 1P (L, 110, 111, 114), 2P (R+408, 110, 111, 114) | `dp/keyflash.lua:36-48,51-69` |
| 키 플래시 | 1P x = L + {110,167,224,281,338,395,452}, 2P x = R + {-1,56,113,170,227,284,341}, y = white 137 / black 147 (7개 패턴 W,B,W,B,W,B,W), 크기 67x73 | `dp/keyflash.lua:6-29,94-114` |
| 스크래치 회전 이미지 | 1P (L+10, 122, 90, 90), 2P (R+508, 122, -90, 90) (좌우반전), `offset = SCRATCHANGLE_1P/2P`(1,2) | `dp/keyflash.lua:116-125` |
| 키 커버(닫힘 연출) | top 1P (L+112, 167, 408, 57), bottom (L+112, 110, 408, 57), 2P 는 x=R; 0..1000ms 정지 후 1300ms 에 접힘 | `dp/keyflash.lua:128-159` |
| 판정선 | (L+3, 227, 513, judgeHeight=12), 500..1000ms 중앙에서 폭 0 -> 513. offset 3(LIFT) | `dp/lane.lua:80-85` |
| 글로우 | (L+3, 227+judgeHeight, 513, glowHeight=48), 가산 블렌드. 박자 타이머 140 로 a 255 -> 0 (1000ms), 초기 글로우 a 0 -> 120 (1000..1500ms) | `dp/lane.lua:65-77` |
| 진행 바(세로) | frame (L-28, 247, 16, 807) = (350,...), (L+1176, ...) = (1554,...); 손잡이 24x37 가 y 247 -> 1020 (range 773), x = frame.x - 4 | `dp/progress.lua:21-72` |
| 진행 바(가로) | 손잡이 (L+322, 148, 24, 37), range 486 오른쪽 | `dp/progress.lua:75-95` |
| 키 빔 | 각 키 위 (L + keyLeftPosX[i], 227, w, h) h = 564 x 비율(기본 50% = 282). 키별 x: 14키 1P {114,177,228,291,342,405,456} 폭 {60,48,60,48,60,48,60}, 스크래치 x=3 폭 108; 2P {3,66,117,180,231,294,345}, 스크래치 408(`dp/inputkey.lua`는 407) | `dp/inputkey.lua:42-53` |
| 노트 | 12절 | |
| 게이지 본체 | (L+328, 117, 500, 35) = (706, 117, 500, 35), 50 파츠, 우방향. 좌방향 옵션 971 은 x=L+828, w=-500 | `dp/gauge.lua:91-110` |
| 게이지 % 프레임 | perFrame (L+519, 180, 126, 90), 정수부 gaugenumber (L+534, 220, 30, 40) 3자리, 소수점 afterdot (L+545, 194, 8, 8), 소수 1자리 (L+559, 185, 21, 40) | `dp/gauge.lua:9-30` |
| 게이지 숨김 | gaugeCover (L+328, 117, 500, 35), perFrame2 (L+519, 180, 126, 90), 옵션 931 + 마지막 노트 전 | `dp/gauge.lua:77-84,118-123` |
| 점수/콤보/BPM 패널 | nowscore (L+162, 39, 30x40 x 6자리), maxcombo (L+814, 39, 5자리), minbpm (L+412, 58, 21x40), nowbpm (L+524, 63, 30x40, 가운데정렬), maxbpm (L+670, 58) | `dp/info.lua:390-415` |
| 레벨/난이도 | lev_* (L+22, 117, 165, 20), playlevel (L+257, 119, 24x20) | `dp/info.lua:349-363` |
| 게이지/옵션 라벨 | gauge{종류} (L+846, 145, 155, 20), useOptionLeft (L+852, 117, 130, 20), useOptionRight (L+1002, 117, 130, 20) | `dp/info.lua:365-387` |
| LN 모드/이펙터 | (L+470, 6, 227, 28) | `dp/info.lua:294-320` |
| 램프 | (L-18, 0, 152, 108), (L+1008, 0, 152, 108). 박자 타이머로 a 255 -> 150, 게이지 MAX(`timer 44`)시 lamp_maxgauge | `dp/info.lua:321-348` |
| 판정 문자 | 1P (L+130, 380, 227, 84), 2P (R+130, 380, ...). 콤보 숫자는 판정 문자 기준 상대 (237, 0, 55, 84). judgefont_position_y = 227 + 153. `shift = true` | `dp/judge.lua:14,57-175` |
| 레인커버 | 슬라이더 (L+3, 1080, 513, 853) 에서 아래로 range 853. 등장 연출: 500..1000ms y 1933 -> 1080 | `dp/cover.lua:39-42` |
| 리프트 커버 | (L+3, -626, 513, 853) 엔진이 disapearLine=227 과 LIFT 오프셋으로 이동 | `dp/cover.lua:37` |
| hidden 커버 | (L+3, -626, 513, 853) | `dp/cover.lua:35` |
| 레인커버/리프트 숫자 | 조작 중(op 270)만: lanecoverNumber (L+100, 1090, 24x20), greennumber (L+270, 1090), 최소/최대 BPM 녹색 (L+225, 1120) / (L+355, 1120) 과 "~" 텍스트 `from` (L+335, 1117, 24x20, 색 {84,255,0}); 리프트 숫자 (L+100, 197, 24x20), (L+270, 197) | `dp/cover.lua:118-128` |
| 종료 시 레인커버 | ENDOFNOTE_1P 시 y 1080 -> 227, 1000ms, 감속 (옵션 981) | `dp/cover.lua:78-89` |
| 좌열(10..341) 위부터 | titleFrame (10, 969, 331, 92) 와 title/genre 텍스트 (중심 x = 10+10+155.5, y = 994, 311x25, 0~2000ms 페이드인 ... 18000ms 순환); 아래 BGA 프레임 (10, 620, 331, 331), (10, 272, 331, 331) (내부 +5, 321x321); 노트 분포 프레임 (10, 90, 331, 140), 그래프 (20, 100, 311, 108); 타이밍 프레임 (10, 23, 331, 54), 판정 타이밍 그래프 (20, 28, 311, 30) | `dp/info.lua:18-44,228-291`, `dp/graph.lua:114-222` |
| 좌열 점수 그래프(graph=좌) | graFrame (10, 882, 331, 198), graFrame2 (10, 239, 331, 112), 배경 graBg (10, 351, 331, 531) a=245, 그래프 3열 w=78 h=523 x = 10+{80, 161, 243} (target, best, now), y=351, 기준선 MAX (10, 852, 331, 29), AAA/AA/A 선 y = 351+{464, 406, 348}(=815, 757, 699), 현재 랭크 문자 (254, 364 ...), EX 점수 (110, 1016) / 목표 (110, 905), 라이벌 이름 텍스트 (92, 255, 140x25) | `dp/scorebar.lua:72-238` |
| 우열(1580..1911) | artistFrame (1580, 969, 331, 92) + artist 텍스트 (중심 1745.5, 994); BGA 프레임 (1580, 620), (1580, 272); infoFrame (1580, 5, 328, 225) 와 숫자: 분 (1662, 205), 초 (1720, 205), 총노트 (1657, 173), 판정 카운트 x = 1657 / 1740 / 1824, y = 142(PG) 111(GR) 78(GD) 46(BD) 15(PR) 각 20x20 4자리 | `dp/info.lua:45-109` |
| 자동/리플레이 안내 | preautoinfo (subPosX[1]+7, 773, 318, 165) 와 (subPosX[2]+7, 773, ...), AUTOPLAYON 또는 REPLAY_PLAYING 이고 LOADED | `dp/prepare.lua:150-171` |
| 연습 모드 | BLACK (L, 0, 1165, 700, a=100), bga (L, 0, 1165, 700) | `dp/prepare.lua:173-186` |
| 로딩 화면 | pretitle (center = L+582 = 960, y=600, w=1164, h=90), pregenre (960, 790, 1164, 40), preartist (960, 430, 1164, 40), emblem (L+84, 914, 996, 74), BACKBMP (L, 400, 1164, 500) a 100 | `dp/prepare.lua:7-147` |
| 풀콤보 | fc (L, 227, 519, 857) 시트 5190x2571, divx 10, divy 3, cycle 1500, 가산; "FULL" wd_full / "COMBO" wd_combo 가 R 쪽에서 슬라이드 | `dp/fullcombo.lua:5-45` |
| 폭발 | 레인별 중심 x = L + {144,201,258,315,372,429,486, 스크래치 57}, R + {33,90,147,204,261,318,375, 스크래치 462}, 크기 400x300 (offset 44 로 축소), y = 227 - 0 - 150 = 77, 가산, loop=-1, 타이머 BOMB_* | `dp/bomb.lua:77-92,163-291` |
| 실패 연출 | close_frame_top/bottom (0, 1080 -> 515 / -565 -> 0), msg_window, 화살표, STAGE/FAILED 단어, 빔 20개, 2500..3000ms BLACK 페이드. 전부 timer 3 | `P/close.lua:6-123` |
| 최종 페이드 | BLACK (0,0,1920,1080) a 0 -> 255, 500ms, timer 2 | `play14_hw.lua:259-265` |

### 7.3 옵션에 따른 배치 변형

| 옵션 | 변형 |
|---|---|
| BGA 972(1:1) | 4 프레임 (331x331): 좌열 y=620,272 / 우열 y=620,272. BGA 객체를 4번 그림 |
| BGA 973(16:9) | 6 프레임 (331x191): y=760,516,272 좌 3, 우 3 |
| BGA 974(無効) | 4 프레임 + soundOnly1_1 (321x321) |
| playPosition 921/923 + graph 920(없음) | 큰 BGA 프레임 670x670 (subPosX[1], 278) + BGA (subPosX[1]+11, 289, 648x648) (`dp/info.lua:119-178`) |
| graph 918/919 | 점수 그래프 위치가 `subPosX[1]`(좌) 또는 `subPosX[2]`(우). 그러나 노트 분포/타이밍 그래프는 항상 `subPosX[1]` 기준(`dp/graph.lua:114,152`) |
| 타이밍 그래프 940(플레이 영역 측) | (L+23, 310, 472, 30) 및 (R+23, 310, 472, 30), 타이머 PLAY, offsets {LIFT, 49} |
| 판정 문자 위치 | 옵션 TYPE 에 따라 판정 차이/FAST-SLOW 의 x,y 가 달라짐(`dp/assist.lua:9-78`): 1P TypeB dpx=-165, TypeC dpx=520; 2P TypeB dpx=565 |
| 게이지 971(좌방향) | x=L+828, w=-500 |
| 게이지 숨김 931 | gaugeCover + perFrame2 덮개 |
| 10키 | 10절 |

### 7.4 z 순서 (14키, 뒤 -> 앞, `play14_hw.lua:45-265`)

1. background (bg, BLACK 오버레이) 2. keyflash (프레임, 플래시, 키 커버) 3. info (패널, 제목, 정보, BGA, 이펙터, 램프, 레벨, 사용옵션, 점수/BPM) 4. progress 5. lane (레인, 구분선, 밝기, 프리뷰 텍스트, 글로우, 판정선) 6. gauge 7. inputkey (키 빔) 8. note(`{id="notes", offset=30}`) 9. cover (hidden, lift, 레인커버, adjusted, 종료 커버, 숫자, 리프트 토글 영역) 10. judge 11. prepare (로딩/연습) 12. scorebar 13. assist 14. bomb 15. graph (노트 분포, 타이밍) 16. fullcombo 17. close 18. 최종 페이드.

주의(미확인): 좌열에서 `titleFrame`(10, 969, 331x92)과 `graFrame`(10, 882, 331x198)이 y 969~1061 에서 겹치고 info 가 먼저 그려져 점수 그래프 프레임이 제목을 덮는 것으로 보인다. `score.png` 의 해당 영역은 RGBA (159,204,78,230) 라 거의 불투명이다. 원작 실제 화면과 같은지는 beatoraja 렌더 비교가 필요하다. 마찬가지로 `bottomFrame`(y 0..180)이 `keyFrame`(y 110..224)의 하단과 겹친다.

---

## 8. 자산

### 8.1 DP 에서 실제 참조되는 자산 (기본 옵션)

| 경로 | 크기(px) | 용량 | 용도 |
|---|---|---|---|
| `Play/parts/dp_hw/system.png` | 1700x1500 RGBA | 165 KB | UI 시트(숫자, 프레임, 라벨 전부). 슬라이스는 `dp/*.lua` 의 image/value 정의 |
| `Play/parts/dp_hw/lane.png` | 520x900 | 4 KB | 레인 배경 + 구분선 |
| `Play/parts/dp_hw/score.png` | 660x1030 | 31 KB | 점수 그래프 프레임/막대/랭크 문자 |
| `Play/parts/dp_hw/adjusted.png` | 541x853 | 4 KB | 가변 SUD 표식 |
| `Play/parts/dp_hw/graphbg/#default.png` | 331x531 RGB | 3 KB | 점수 그래프 배경 |
| `Play/parts/dp_hw/lift/#default.png` | 513x853 RGBA | 62 KB | 리프트 커버 |
| `common/bg/#default.png` | 1920x1080 RGB | 28 KB | 배경 |
| `common/judge/#default.png` | 777x588 | 92 KB | 판정 문자/콤보 숫자 |
| `common/judgeline/#default.png` | 513x12 | 1 KB | 판정선 |
| `common/notes/#default.png` | 324x360 | 73 KB | 노트 전체 |
| `common/glow/#default.png` | 431x48 | 3 KB | 글로우 |
| `common/progress/#default.png` | 24x37 | 2 KB | 진행 손잡이 |
| `common/bomb/diamond SCUROed..png` | 6400x1200 RGB | 1.4 MB | 폭발 (4행 x 16열 400x300) |
| `common/fullcombo/#default.png` | 5190x2571 RGBA | 10.5 MB | 풀콤보 (10x3 프레임) |
| `common/keybeam/#default.png` | 238x564 | 9 KB | 키 빔 (w 60, b 48, s 108) |
| `common/key/harf.png` | 519x342 | 23 KB | 키보드 |
| `common/keyflash/#default.png` | 67x73 | 4 KB | 키 플래시 |
| `common/close/close.png` | 2200x2100 RGBA | 820 KB | 실패 연출 (GPU 18 MB) |
| `common/lamp/#default.png` | 152x216 | 5 KB | 램프 |
| `common/lanecover/#default.png` | 513x853 RGB | 364 KB | 레인커버 |
| `common/mine/#default.png` | 324x36 | 3 KB | 지뢰 노트 |
| `common/gauge/#default.png` | 80x70 | 1 KB | 게이지 파츠 (8px 열, 35px 행 2개) |
| `common/scratch/#default.png` | 92x92 | 2 KB | 스크래치 회전 |
| `common/hcn/hcn.png` | 1600x1200 | 286 KB | HCN 폭발 (4x4) |
| `Play/font/ttf/mgenplus-1c-black.ttf`, `-medium.ttf` | | 5.2 + 5.3 MB | 텍스트 (제목 등) |
| 조건부: `common/BGA/movie/#default.mp4` | 1280x720 | 19.9 MB | BGA 없는 곡 + 汎用BGA 動画 |
| 조건부: `common/BGA/image/#default.png` | 1280x720 | 725 KB | BGA 없는 곡 + 汎用BGA 画像 |

`oadx_bomb/dummy.png`(2896x768, 7 KB)는 OADX 옵션 선택 시 로드(def `DEFAULT` 없음).

### 8.2 와일드카드 슬롯과 선택지 수

| 폴더 | 변형 개수 | 용량 |
|---|---|---|
| bg | 7 (#default, blue, gray, green, orange, purple, red) | 7.0 MB |
| bomb | 7 | 8.3 MB |
| fullcombo | 1 | 10.8 MB |
| judge | 4 | 240 KB |
| notes | 10 | 640 KB |
| keybeam | 6 | 68 KB |
| key | 2 | 44 KB |
| keyflash | 5 | 20 KB |
| gauge | 6 | 24 KB |
| glow | 4 | 52 KB |
| lamp | 5 | 40 KB |
| progress | 5 | 20 KB |
| scratch | 8 | 36 KB |
| judgeline | 3 | 12 KB |
| BGA/movie | 4 mp4 (#default, cyber, NOSTALGIC, travel) | 134 MB (폴더 전체) |

### 8.3 참조되지 않는 자산

- `Play/parts/sp_hw/*`(332 KB), `Play/parts/common/attack/*`(SP 전용), `Play/parts/common/POMYU Chara/Off`, `Play/font/fnt/*`(info.fnt, title.fnt, top.fnt + 약 40 png, 43 MB; "画像フォント 有効" 옵션 925 일 때만), `Root/image/*`(1.3 MB), `Root/sounds/*`(1.5 MB; DP 기본 경로에서는 `main_state.audio_play` 가 호출되지 않음), `io/Result|Select/*`, `History/*`, `Sound/`, `Decide/`, `Result/`, `Select/`, `SkinSelect/`, `KeyConfig/`, `Root` 의 선곡/결과용 모듈.
- 용량 합계(`du`): `Play` 214 MB, `Play/parts` 161 MB, 그 중 `common/BGA` 134 MB, `Play/font` 53 MB, `Play/lua` 612 KB.

### 8.4 `io/Play/dp`

`io/Play/dp/lanecover/pathList.txt`(내용 `Play/parts/common/lanecover/#default.png` 1줄), `excludeList.txt`(빈 파일), `temp.txt`(같은 경로 2줄). 레인커버 로테이션(979) 전용. 기본 옵션에서는 `Root/customnumber.lua:290,293` 의 `countFileRecords` 가 읽기만 한다.

---

## 9. 단순화 후보

### 9.1 빼도 화면 핵심이 유지되는 것

| 후보 | 근거 | 절감 |
|---|---|---|
| 汎用BGA 동영상 4개와 `source id 0 mp4` 경로 | 곡에 BGA 가 없을 때만 사용 | 134 MB 및 동영상 디코더 의존 |
| `Play/font/fnt` 전체와 "画像フォント" 옵션 | 기본은 TTF | 43 MB, 옵션 1개, `req/textproperty.lua:44-66` |
| BGA 16:9, 1:1 x4 복제 레이아웃 | 같은 BGA 를 4~6번 그림(`dp/info.lua:179-291`). 단일 BGA 프레임 하나로 충분 | destination 약 40개 |
| 풀콤보 시트 10.5 MB (`fullcombo/#default.png`) | 연출 | 10.5 MB, GPU 53 MB |
| OADX 폭발 (옵션 917, `common/oadx_bomb`) | 선택지 | 분기 절반 |
| 判定タイミングボム(FAST/SLOW 색 폭발) | 기본 off | `slowbomb-*`, `fastbomb-*` 32 이미지 + destination 32 |
| `hcnBomb`(HCN 폭발 16 이미지 + 코드) | 기본 `CONFIG.play.hcnBomb=false` | |
| 判定 변형 8종 (`laneCoverRest*`, `constantRest*`) | 기본 `valiableJUDGE.sw=false`, judge.lua 673줄의 80% | judge.lua 를 def1/def2 로 축소 |
| sectionScore, voice, 효과음(`customsound.lua`), `bpmLinkChar` | 기본 비활성 | 코드 다수, 오디오 의존 |
| `lanecover` 로테이션 (옵션 979, `randomChoice*`, `io/Play/dp`, luajava) | 기본 off | io/luajava 의존 제거 |
| `History/information.txt` 출력 (`CONFIG.infoOutput`, `infoOutput(0)`) | OBS 연동용 | 로드 시 파일 쓰기 제거, 0절의 background 실패 위험 제거 |
| 버전 확인 http, 다국어(번역) | DP 와 `Root/*` 공통 코드에는 http 호출이 없고(주석 URL 뿐) 번역 기능도 없음 (선곡/결과 화면은 이 조사 범위 밖) | 해당 없음 |
| 색/종류 변형 폴더 (notes 10, bg 7, gauge 6, judge 4 등) | 기본 `#default` 만 있으면 됨 | 약 10 MB |
| property 31개 | 핵심은 playPosition, 키빔 높이, ゲージ向き 등 6~8개 | |
| `Root/define2` 체인 (customoption/number/graph/slider/function/time/text/sound/timer) | DP 기본 경로가 쓰는 것은 일부(isRemainSec, isBrinkGauge, adjustedCover, laneCoverRest 등)뿐 | `luajava`, `os.date`, `io` 의존 제거 가능 |
| 노트 `*-a`(accent) 이미지 11개와 미사용 객체(2절 목록) | destination 없음 | |

### 9.2 빼면 안 되는 것

- 레인(1P/2P) + 노트 12/16 레인 정의 + LN/CN/HCN/지뢰 이미지 + 마디선 + `note.group/time/bpm/stop`.
- 판정 문자 + 콤보(판정 시 `loop=-1` 500ms 1회 표시), 판정 오프셋 id 32.
- 폭발(일반/LN), 키 빔, 키 플래시, 키 프레임과 스크래치 이미지.
- 게이지(50파츠, 6종 x 6 노드 36개, % 표시, 하드계열 점멸 op 230-232, 0 근처 색 변화).
- 점수/최대콤보/BPM(min,now,max)/남은 시간(60초 미만 색 변화)/판정 카운트 패널.
- 레인커버(슬라이더 type 4), 리프트(offset 3, liftCover, 클릭 토글), hidden, hi-speed 숫자(녹색/흰색 duration), 가변 SUD(adjusted) 와 `CONSTANT` 연동.
- 점수 그래프(SCORERATE 계열 graph type 110-115 + 랭크 막대 + 라이벌 이름).
- BGA(`bga` 객체, FIT_OUTER_TRIMMED 두 겹 그리기), 로딩 화면(`NOW_LOADING`/`LOADED`/`BACKBMP`), 진행 바(slider 6), 실패(close) 연출, 최종 페이드 timer 2, 프리뷰(timer 141).
- 헤더의 `loadend`, `playstart`, `close`, `fadeout`, `scene`, `input`, offset 10개(최소 40, 42, 45, 46, 47, 48) 와 category.

---

## 10. 10키 vs 14키 차이

| 항목 | 14키 | 10키 | 근거 |
|---|---|---|---|
| header type | 2 | 3 | `play14_hw.lua:13`, `play10_hw.lua:13` |
| `dp_property.load` | `false` | `true`: property 31번째 "10鍵用レーンカバー" 등록, category "パーツ表示有無" 에 23 추가 | `req/dp_property.lua:515-538` |
| BASE | `createBasePositionDP(14)` | `(10)` (레인 위치 계산은 key 인자를 쓰지 않음) | `P/base.lua:161` |
| `skin.pmchara = {}` | 있음 | 없음 | `play14_hw.lua:42` |
| keyflash | 7키, 우측 x {-1,56,...,341} | 5키, 좌 {110,167,224,281,338}, 우 {113,170,227,284,341}, 타이머 KEY1..5 | `dp/keyflash.lua:15-27` |
| inputkey | 8 항목 (W,B,W,B,W,B,W,S) | 6 항목 (W,B,W,B,W,S). 우측 x {117,180,231,294,345,407} | `dp/inputkey.lua:31-53` |
| notes | 16 레인, 우측 {3,66,117,180,231,294,345,408} | 12 레인, 우측 {117,180,231,294,345,408}. 1P 측은 키 5개 (114..342) + 스크래치 3 | `dp/notes.lua:24-113` |
| bomb | 16개 | 12개. 중심 x 1P {144,201,258,315,372, 57}, 2P {147,204,261,318,375, 462} (ModernChic 폭발) | `dp/bomb.lua:27-58` |
| 추가 객체 | | `10keysFrame` image (src 1, 1500,340,114x900) 와 destination 2개: 1P (L+405, 1080 -> 180, 114x900, angle 180, a 240, 감속 1000+500ms), 2P (R, ...). 옵션 954 일 때만 | `play10_hw.lua:154-171` |
| 페이드 timer | 정수 리터럴 `2` | `MAIN.TIMER.FADEOUT` (= 2) | `play14_hw.lua:260`, `play10_hw.lua:277` |
| 나머지 | 동일 (레인 이미지, 게이지, info, judge, cover, 점수 그래프 등) | | 두 파일 diff 9곳 |

---

## 11. 싱글 스킨(Play/lua/sp)과의 공유/차이

| 구분 | 파일 |
|---|---|
| DP 와 SP 가 공유 | `Play/lua/base.lua`(함수만 DP/SP 로 분리: `createBasePositionSP/DP`, `addSourceSP/DP`), `Play/lua/background.lua`, `Play/lua/close.lua`, `req/header.lua`, `req/common.lua`, `req/textproperty.lua`, `Root/*`, `config.lua`, `Play/parts/common/*` |
| DP 전용 | `req/dp_property.lua`, `dp/*.lua` 15개, `Play/parts/dp_hw/*` |
| SP 전용 | `req/sp_property.lua`, `sp/*.lua`(assist, attack, bomb, cover, fullcombo, gauge, graph, info, info2, inputkey, judge, keyflash, lane, notes, prepare, progress, scorebar), `sp/detailinfo/*`(bgaareainfo 964줄 등 6개), `Play/parts/sp_hw/*`, `common/attack/*` |
| 같은 이름 부품의 diff 줄 수 | assist 22, bomb 48, cover 27, fullcombo 3, gauge 15, graph 28, inputkey 34, judge 69, keyflash 12, lane 11, notes 7, prepare 21, progress 8, scorebar 35, info 53 (diff hunk 수) |

주요 차이:
- property: SP 는 `プレイサイド`(1P 左スクラッチ / 2P 右스크래치) 옵션이 있고 `isLeftScratch/isRightScratch` 로 `KEY_POSITION` 등을 분기한다(`P/base.lua:5-60`). op 번호 체계도 별개(SP 가 `플레이サイド` 를 먼저 선언해 번호가 밀림). BGA 패턴이 5종(1:1, 16:9, 1:1 x2, 16:9 x4, 無効), 판정 위치 TYPE-D 존재, 타이밍그래프 패턴(분포/막대) 추가. DP 에만 있는 옵션: グラフバー表示位置, プレイ位置 3종, ゲージMAX表示 등 일부, 10键 커버.
- 본체: SP(`play7_hw.lua`) 는 info2, attack, bgaareainfo(detailinfo, `source id 8` 추가) 를 추가 로드한다(`play7_hw.lua:182,248,260`). DP 는 이 부품들이 없고 `keyflash` 를 `info` 앞에 두며 `lane` 이 `progress` 뒤에 온다.
- judge: SP 는 판정 객체 1개 `def`(위치 `BASE.playsidePositionX + 130`), DP 는 좌우 2개 x 변형 5세트.
- 게이지: SP 는 프레임 `gaugeFrame_1P/2P`(556x69) 와 `GAUGE` 위치 계산 `createGauge()`, DP 는 중앙 패널 + 500px 본체 하나.
- 시트: SP 는 `sp_hw/system.png`(크기 미확인, 읽지 않음), DP 는 `dp_hw/system.png`(1700x1500).

---

## 12. 좌·우 필드의 note / gauge / judge 객체 필드 값 (기본 옵션, L=378, R=1023)

### 12.1 note (`dp/notes.lua`)

공통 필드: `id = "notes"`; 나머지 12개 배열은 레인 순서대로 아래 이름을 가진다. 14키 레인 순서(왼쪽 필드 L1..L7, L스크래치, 오른쪽 필드 R1..R7, R스크래치): 흰 키 W(1,3,5,7), 파랑 키 B(2,4,6), 스크래치 S.

| 배열 | 14키 값(16개) | 10키 값(12개) |
|---|---|---|
| note | note-w,note-b,note-w,note-b,note-w,note-b,note-w,note-s 를 두 번 | note-w,note-b,note-w,note-b,note-w,note-s 를 두 번 |
| lnend | lne-* | 동일 접두 |
| lnstart | lns-* | |
| lnbody | lnb-* (hold 중) | |
| lnactive | lna-* (hold 아님) | |
| hcnend / hcnstart / hcnbody / hcnactive / hcndamage / hcnreactive | hcne-*, hcns-*, hcnb-*, hcna-*, hcnd-*, hcnr-* | |
| mine | mine-w,b,s | |
| hidden, processed, size | `{}` (빈 배열) | `{}` |
| dst (14키) | y = 227 - 12 = 215, h = 853 - 12 = 841 공통. x/w: L1 492/60, L2 555/48, L3 606/60, L4 669/48, L5 720/60, L6 783/48, L7 834/60, L스크래치 381/108, R1 1026/60, R2 1089/48, R3 1140/60, R4 1203/48, R5 1254/60, R6 1317/48, R7 1368/60, R스크래치 1431/108 | L1 492, L2 555, L3 606, L4 669, L5 720, L스크래치 381; R1 1140, R2 1203, R3 1254, R4 1317, R5 1368, R스크래치 1431 (w 는 60/48/60/48/60/108) |
| group | 마디선 `{id="section-line", offset=3, dst={{x=L+3, y=227, w=513, h=3, a=barlineBright}}}` 좌/우 (`dp/notes.lua:255-262`) | 동일 |
| time | `{id="section-line", offset=3, dst={{x=L+3, y=227, w=513, h=15, r=100, g=100, b=255}}}` 좌/우 | 동일 |
| bpm | 같은 형식, h=15, r=100, g=255, b=100 | 동일 |
| stop | 같은 형식, h=15, r=255, g=100, b=100 | 동일 |

beatoraja 쪽 해석(`bj:play/LaneRenderer.java:701-731`): `lnbody` 는 해당 LN 을 처리 중(hold)일 때, `lnactive` 는 처리 중이 아닐 때 그린다(`bj:skin/json/JsonPlaySkinObjectLoader.java:50-55`, `lnbodyActive` 가 없을 때 `lns[2]=lnbody`, `lns[3]=lnactive`). LN(롱노트, 비 CN) 은 `lnend` 를 그리지 않는다. HCN 은 `lnbody` 대신 `hcnbody`(처리 중 index 6), `hcnactive`(index 7), 통과 중이며 상태가 0 이 아닐 때 `getHellChargeJudge` 에 따라 `hcndamage`(8) 또는 `hcnreactive`(9). 이름과 ModernChic 주석이 어긋날 수 있으므로 시각 확인 필요. 노트 높이 `size` 가 비어 있어 각 레인의 `note[i]` 이미지 높이(36)를 사용한다(`bj:skin/json/JsonPlaySkinObjectLoader.java:92-94`).

노트 이미지(`notes/#default.png` 324x360): 흰 x=216,w=60 / 파랑 x=276,w=48 / 스크래치 x=108,w=108 / acc x=0,w=108. 행 y: note 0, lne 36, lns 72, lna 108(h 36), lnb 144(h 72, divy 2, cycle 200), hcne 216, hcns 252, hcnb 288(h 36 divy 2, cycle 200), hcnr 324(divy 2, cycle 100), hcnd 288, hcna 108(h 18). `notesAnimation=false` 이면 divy/cycle 없이 h 절반(`dp/notes.lua:192-234`). 지뢰는 `src 26` (mine/#default.png 324x36).

### 12.2 gauge (`dp/gauge.lua:180-199`)

```
parts.gauge = { id = 2001, parts = 50, nodes = { 36개 문자열 } }
smallGauge=true 이면 parts = 100
nodes (6종 x 6개, 순서: overclear 명, underclear 명, overclear 암, underclear 암, 선단 명, 선단 암)
 assist-easy : gauge-r1,gauge-p1,gauge-r2,gauge-p2,gauge-r3,gauge-p3
 easy        : gauge-r1,gauge-g1,gauge-r2,gauge-g2,gauge-r3,gauge-g3
 normal      : gauge-r1,gauge-b1,gauge-r2,gauge-b2,gauge-r3,gauge-b3
 hard        : gauge-r1,gauge-p1,gauge-r2,gauge-p2,gauge-r3,gauge-p3
 exhard      : gauge-y1,gauge-p1,gauge-y2,gauge-p2,gauge-y3,gauge-p3
 hazard      : gauge-h1,gauge-p1,gauge-h2,gauge-p2,gauge-h3,gauge-p3
```
`gauge-*` 는 src 29 (gauge/#default.png 80x70) 의 8x35 조각: r1 (0,0), r2 (16,0), b1 (8,0), b2 (24,0), g1 (32,0), g2 (48,0), p1 (8,35), p2 (24,35), y1 (0,35), y2 (16,35), h1 (64,0), h2 (72,0), `*3` 은 `*1` 과 같은 좌표. beatoraja 는 class 게이지(type >= 6)를 `(type-3)*6` 으로 사상한다(`bj:play/SkinGauge.java:182`). destination `"2001"`: 좌표 (L+328, 117, 500, 35) = (706, 117, 500, 35), 점멸용 사본은 time 0 a=255 -> 500 a=200 -> 1000 a=255. 위에 BLACK 이 같은 영역에서 폭이 줄어들며 "게이지가 차오르는" 연출(2500ms, 감속). 좌방향은 x=1206, w=-500.

### 12.3 judge (`dp/judge.lua:55-175`)

```
{ id="def1", index=0, shift=true,
  images  = 6개 { id = judgef-pg|gr|gd|bd|pr|ms, loop=-1, timer=46, offsets={3,32},
                  dst={ {time=0, x=L+130=508, y=227+153=380, w=227, h=84}, {time=500} } },
  numbers = 6개 { id = judgen-pg|gr|gd|bd|pr|ms, loop=-1, offset=32, timer=46,
                  dst={ {time=0, x=237, y=0, w=55, h=84}, {time=500} } } }
{ id="def2", index=1, shift=true, 동일 구조, x=R+130=1153, timer=47 (offsets 는 2P 에서도 {3,32}, numbers 의 offset 도 32) }
```
`judgef-*` 이미지 소스: pg (0,0,227x252 divy 3, cycle 120), gr (0,252,227x168 divy 2, cycle 80), gd (0,420,...), bd (227,420,...), pr (454,420,...), ms 는 pr 와 같은 영역. `judgen-*`: pg (227,0,550x252 divx 10 divy 3 digit 6 ref MAXCOMBO cycle 120), 나머지 (227,252,550x168 divx 10 divy 2 digit 6 cycle 80). `judgeAnimation=false` 이면 cycle 제거. 나머지 8개 judge(`laneCoverRest*`, `constantRest*`)는 같은 구조에 `draw` 함수와 `a = CONFIG.play.valiableJUDGE.alfa`(50) 변형이다. destination 선택은 로드 시 `main_state.option(-400)`/`option(400)` 과 `valiableJUDGE.sw` 로 결정(`dp/judge.lua:654-665`).

---

## 13. 위험, 미확인, 읽지 못한 파일

### 13.1 위험

1. `luajava` 스텁 부재 시 스킨 전체 로드 실패(0절).
2. 로드 시 파일 쓰기 실패 -> 배경 사라짐(0절). 읽기 실패 시 `io.open` 이 nil 을 돌려줘야 하고 예외를 던지면 안 됨(`Root/customfunction.lua:18-28` 가 nil 비교).
3. `dp_property.load()` 비멱등. 헤더 단계와 본 단계에서 같은 번호가 나오려면 Lua 상태/모듈 캐시 정책을 beatoraja 와 같게 해야 한다.
4. `loop = -1` 의미, offset 의 `x -= w/2` 보정, 정수 truncation, 음수 w 반전을 beatoraja 와 똑같이 구현해야 시각이 같다.
5. 인자 평가 시점: `main_state.option(170)` 등이 로드 시점에 호출되므로 스킨은 곡 선택 후 플레이 진입 때 다시 평가되어야 한다. 정적 JSON 변환으로는 NO_BGA 분기를 표현할 수 없다.
6. `slider.value` / `draw` 함수는 객체가 실제로 그려질 때만 평가되어야 한다(`adjustedCover()` 가 nil 반환 -> 인덱싱 오류). 오류는 프레임 단위로 격리해야 하며 프레임 루프를 죽이면 안 된다.
7. 라이선스(0절). 기본 번들에 ModernChic 에셋을 그대로 넣는 것은 금지 사항에 해당할 수 있다.
8. 이름 반전 옵션: ゲージ(930 = 표시 = `Off`), ノート分布, タイミンググラフ. `isXxxCoverOff` 가 "숨기지 않음(= 표시)"이다. 옵션 이름과 함수 이름을 혼동하면 반대로 동작한다.
9. 좌열 제목/BGA/그래프 겹침, 하단 패널과 키보드 겹침은 z 순서만으로는 의도 확인 불가(7.4 미확인).
10. `inputkey.lua` 의 14키 우측 스크래치 x 는 407, `notes.lua` 는 408 (`dp/inputkey.lua:45`, `dp/notes.lua:67`). 1px 차이는 원본 그대로 유지해야 같은 화면이 된다.

### 13.2 미확인

- Lua 를 실제로 실행하지 못해 destination 총 개수(약 375)와 image 개수(263)는 정적 추정이다.
- `Play/parts/sp_hw/system.png` 크기, `common/*` 변형 파일들의 시각 차이, `Play/font/fnt/*` 내용은 보지 않았다.
- property 배열 순서가 UI 에 미치는 영향, `shift = true` 판정/콤보 정렬의 정확한 레이아웃 계산(beatoraja `SkinJudge`)은 구현 단계에서 beatoraja 소스(`bj:play/SkinJudge.java`)로 확인 필요.
- 숫자 객체 `divx=11` / `12` 의 11, 12번째 셀 의미(공백/콜론/+/-)와 `align` 기본 동작은 `bj:skin/SkinNumber.java:140-193` 근거로만 확인(+/- 렌더 세부는 미확인).
- 로드 타이밍이 곡마다 다시 일어나는지(beatoraja 에서의 스킨 캐시 정책) 미확인.

### 13.3 읽은 범위

읽음(전수): `play14_hw.lua`, `play10_hw.lua`, `.luaskin` 2개, `Play/lua/base.lua`, `background.lua`, `close.lua`, `Play/lua/require/{dp_property,header,common,textproperty}.lua`, `Play/lua/dp/*.lua` 15개 전부(`judge.lua` 340~650줄은 동형 반복이라 grep 으로 값 집합만 검증), `config.lua`, `Root/{define,define2,version,author,main*,custom*}.lua` 전부, `Play/readme.txt` 일부, `Play/parts` 목록, `io/Play/dp`. `sp_property.lua`, `sp/*.lua` 는 diff/핵심부만. PNG 는 `system.png`, `score.png`, `lane.png`, `notes/#default.png`, `key/harf.png`, `judge/#default.png`, `bg/#default.png`, `lamp/#default.png`, `gauge/#default.png` 를 직접 확인.

읽지 못함: `Play/lua/sp/*.lua` 대부분의 본문(info2, attack, detailinfo 등), `Play/lua/require/sp_property.lua` 후반, `Root/image/*`, `Root/sounds/*`, `Play/parts/common/*` 의 변형 이미지 대부분, `Play/font/fnt/*`.
