# M2 조사 보고서: ModernChic 싱글 플레이 스킨(5키 / 7키)

> 최종 갱신 2026-10-09 · 대응 단계: L1 조사(구현 전) · 기준 커밋 `9ce92bb` · 색인과 갱신 규칙은 [README.md](README.md)

작성 범위: `/Users/hyunseokbyun/Downloads/ModernChic` 의 `play7_hw.lua`, `play5_hw.lua`, `Play/lua/{base,background,close}.lua`, `Play/lua/require/*.lua`, `Play/lua/sp/**/*.lua`(detailinfo 포함), `Root/main*.lua`, `Root/custom*.lua`, `Root/define*.lua`, `config.lua`, `Play/parts/{sp_hw,common}` 파일 목록, `io/Play/sp`. 대조용으로 `/Users/hyunseokbyun/development/beatoraja/src/bms/player/beatoraja/{skin,skin/lua,skin/json,play}` 를 읽었다.

표기 규칙
- 경로 약칭: MC = `/Users/hyunseokbyun/Downloads/ModernChic`, BJ = `/Users/hyunseokbyun/development/beatoraja/src/bms/player/beatoraja`. 근거는 `파일:줄` 로 쓴다.
- 좌표계: 1920x1080, 원점은 화면 왼쪽 아래, y 는 위로 증가한다 (BJ/skin/json/JSONSkinLoader.java 의 setDestination 은 y 를 그대로 `dh` 배율만 곱해 전달하고 뒤집지 않는다. BJ/skin/Skin.java:135-148).
- 용어: P = `BASE.playsidePositionX`, I = `BASE.infoPositionX`, G = `BASE.gaugePositionX`, J = `BASE.NOTES_JUDGE_Y`(227).

## 0. 조사 방법과 한계 (먼저 읽을 것)

- 이 환경에는 Lua 인터프리터(lua, luajit, nvim, vim+lua)가 없고 cargo 실행이 금지라, 스킨을 실행해 테이블을 덤프하지 못했다. 모든 개수와 좌표는 소스를 직접 읽고 손으로 추적한 값이다. 개수 집계(절 2)는 오차가 있을 수 있으므로 구현 단계에서 R-BMS 가 만든 테이블 덤프와 대조해 확정해야 한다.
- 시각적 확인은 PNG 몇 장(`system.png`, `info.png`, `notes/#default.png`, `key/#default.png`, `judge/#default.png`, `score.png`, `bg/#default.png`, 게이지 프레임 잘라낸 것)만 직접 열어 보았다. beatoraja 를 실행해 실화면과 비교하지는 못했다.
- `Play/lua/require/dp_property.lua`(548줄)는 DP 전용이다. play7_hw/play5_hw 의 SP 경로에서 require 되지 않으므로 구조(상단 카운터 정의, 카테고리 삽입 위치 12)만 확인했고 전문은 읽지 않았다. `Play/lua/dp/*` 는 배정 밖이라 읽지 않았다.
- 읽지 못한 beatoraja 클래스(미확인): `SkinNoteDistributionGraph`, `SkinBPMGraph`, `SkinTimingVisualizer`, `SkinHitErrorVisualizer`, `SkinSourceMovie`, `SkinLoader.getPath`(와일드카드 해석), `IntegerPropertyFactory`/`BooleanPropertyFactory` 의 개별 id 구현, `RhythmTimerProcessor`/`JudgeManager`(타이머 발화 시점). 해당 id 의 값은 ModernChic 의 `Root/main*.lua` 와 `SkinProperty.java` 상수 대조까지만 했다.

## 1. 헤더

### 1.1 기본 필드 (`Play/lua/require/header.lua:8-27`, `play7_hw.lua:13`, `play5_hw.lua:13`)

| 필드 | 값 | 비고 |
|---|---|---|
| type | 7키 `0`, 5키 `1` | `load(0)` / `load(1)`. `header.lua:2-3` 주석에 type 표 |
| name | `"ModernChicPlay(SCURO)-" .. ver` = `ModernChicPlay(SCURO)-4.6` | ver 는 `Root/version.lua` (숫자 4.6) |
| author | `"KASAKO"` | `Root/author.lua` |
| w, h | 1920, 1080 | |
| loadend | 3500 | 로드 시작부터 로드 종료까지 최소 시간(ms) |
| playstart | 1000 | 로드 종료 후 곡 시작까지(ms) |
| scene | 3600000 | |
| input | 0 | |
| close | 3000 | 실패(폐점) 연출 시간 |
| fadeout | 500 | |
| judgetimer, finishmargin | 미지정 | beatoraja 기본값 judgetimer=1, finishmargin=0 (BJ/skin/json/JsonSkin.java:20-21) |
| property, filepath, offset, category | 절 1.2-1.5 | `PROPERTY.*` 를 그대로 대입 |

주의: `close/loadend/playstart/judgetimer/finishmargin` 은 beatoraja 에서 `note` 객체가 destination 에 있을 때만 PlaySkin 에 반영된다 (BJ/skin/json/JsonPlaySkinObjectLoader.java:36-44 의 주석 포함).

### 1.2 property (옵션): 7키 36개, 5키 37개 (op 900-995, 총 96개 항목)

op id 는 `Play/lua/require/sp_property.lua` 의 `customoption.chiled` 호출 순서대로 `899+n` 이 자동 부여된다 (`sp_property.lua:12,22-29,46-52`). 이 호출은 5키/7키와 무관하게 전부 실행되므로 5키 전용 항목(984/985)도 7키에서 번호가 소비된다. 따라서 두 스킨의 op id 는 동일하다. 번호는 모듈 상태 카운터이므로 `load()` 는 Lua 상태당 정확히 한 번만 호출돼야 한다 (재호출 시 번호가 밀린다).

category 번호는 `parent/filepath/offset` 호출 순서의 정수 `1..67` (`sp_property.lua:16-20`). beatoraja 는 category 의 item 과 property.category 를 문자열 `tojstring()` 으로 비교한다 (BJ/skin/json/JSONSkinLoader.java:143-165). 정수 `1` 은 `"1"` 이므로 R-BMS 는 정수를 정수 형식으로 문자열화해야 한다 (`"1.0"` 이 되면 불일치).

| cat | name (JP) | 의미(KR) | 항목: op | 기본값(def) |
|---|---|---|---|---|
| 1 | プレイサイド | 스크래치 방향 | 1P(좌스크): 900, 2P(우스크): 901 | 1P(좌스크) |
| 2 | プレイ位置 | 레인 위치 | 1P(좌측): 902, 2P(우측): 903 | 1P(좌측) |
| 3 | 画像フォント | 이미지(비트맵) 폰트 | 無効: 904, 有効(高負荷): 905 | 無効 |
| 4 | スコアグラフ、ノート分布、タイミングエリアの配置 | 하단 정보 배치 | TYPEA(그래프 좌/노트·타이밍 우): 906, TYPEB(반대): 907 | TYPEA |
| 5 | グラフバーの伸びる向き | 스코어 막대 방향 | 左方向: 908, 右方向: 909 | 左方向 |
| 6 | グラフエリア | 스코어 그래프 영역 | 表示: 910, 非表示: 911 | 表示 |
| 7 | BGA表示パターン | BGA 배치 | 1:1: 912, 16:9: 913, 1:1 x2: 914, 16:9 x4: 915, 無効: 916 | 16:9 |
| 8 | 汎用BGAの種類 | BGA 없는 곡의 대체 | 動画: 917, 画像: 918, 無効: 919 | 動画 |
| 9 | ターゲット差分 | 목표 차이 표시 | 非表示: 920, 表示: 921 | 非表示 |
| 10 | ターゲット差分の種類 | | 目標ランク: 922, 自己ベスト: 923 | 目標ランク |
| 11 | ターゲット差分表示位置 | | TYPE-A: 924, B: 925, C: 926, D: 927 | TYPE-A |
| 12 | 判定タイミング | FAST/SLOW 표시 | 非表示: 928, 表示: 929 | 非表示 |
| 13 | 判定タイミングの種類 | | FAST/SLOW: 930, +-ms: 931 | FAST/SLOW |
| 14 | 判定タイミング表示位置 | | TYPE-A: 932, B: 933, C: 934, D: 935 | TYPE-A |
| 15 | キービームの高さ | 키빔 높이 | 100%: 936, 90%: 937, 80%: 938, 70%: 939, 60%: 940, 50%: 941, 40%: 942, 30%: 943, 20%: 944, 10%: 945 | 50% |
| 16 | キービームの消失時間 | | 通常: 946, 短い: 947, 長い: 948 | 通常 |
| 17 | キービームの消失パターン | | TYPE-L: 949, TYPE-B: 950 | TYPE-B |
| 18 | ボムの種類 | | ModernChic規格: 951, OADX規格: 952 | ModernChic規格 |
| 19 | 判定タイミングボム | 빠름/늦음 색 구분 | 無効: 953, 使う: 954 | 無効 |
| 20 | グローランプ | 판정선 글로우 | 非表示: 955, 表示: 956 | 表示 |
| 21 | ゲージMAXインジケータ | | 非表示: 957, 表示: 958 | 表示 |
| 22 | ゲージ | 게이지 가림 | 表示: 959, 非表示: 960 | 表示 |
| 23 | ノート分布 | | 表示: 961, 非表示: 962 | 表示 |
| 24 | ノート分布パターン | | 判定: 963, FAST/SLOW: 964, ノート: 965 | 判定 |
| 25 | タイミンググラフ | | 表示: 966, 非表示: 967 | 表示 |
| 26 | タイミンググラフパターン | | 分布グラフ: 968, 棒グラフ: 969 | 分布グラフ |
| 27 | タイミンググラフ(分布)表示位置 | | BGA側: 970, プレイエリア側: 971 | BGA側 |
| 28 | タイミンググラフ(分布)倍率 | | 低(+-225ms): 972, 標準(+-150ms): 973, 高(+-75ms): 974 | 標準 |
| 29 | タイミンググラフ(分布)配色 | | 通常: 975, 赤: 976, 緑: 977, 青: 978 | 通常 |
| 30 | ヒットエラービジュアライザーパターン | | 通常: 979, 三角: 980, 強調: 981 | 通常 |
| 31 | オートプレイ＆リプレイ時の案内 | | 非表示: 982, 表示: 983 | 表示 |
| 32 | 5鍵用レーンカバー (5키만 property 로 노출) | | 非表示: 984, 表示: 985 | 表示 |
| 33 | 戦闘モード | 공격 모션 | 無効: 986, 有効: 987 | 無効 |
| 34 | 終了時にレーンカバーを下ろす | | 無効: 988, 有効: 989 | 無効 |
| 35 | プレイ状況詳細モード | | 無効: 990, 有効: 991 | 無効 |
| 36 | レーンカバーローテーション | | 無効: 992, 有効: 993 | 無効 |
| 37 | スコアフラップ | | 無効: 994, 有効: 995 | 無効 |

property 노출 순서(사용자에게 보이는 목록)는 `sp_property.lua:257-425` 의 table 순서이며 위 번호 순서와 다르다: 1, 2, 9, 10, 11, 12, 13, 14, 19, 18, 5, 4, 3, 20, 21, 22, 6, 23, 24, 25, 26, 27, 28, 29, 30, 31, 15, 16, 17, 7, 8, 33, 34, 35, 36, 37, (5키만 마지막에 32). 각 항목에서 사용하는 키는 `name, def, category, item[{name, op}]` 이다.

판정 방식: ModernChic 은 destination 의 `op` 에 900번대 id 를 쓰지 않는다. 대신 `skin_config.option[그룹이름] == op` 로 로드 시점에 구조를 분기한다 (`sp_property.lua:50` 의 `condition`). 따라서 옵션 변경 시 Lua 를 다시 실행해 스킨 테이블을 다시 만들어야 한다.

### 1.3 filepath (20개, `sp_property.lua:214-233, 428-449`)

| cat | name | path 패턴 (MC 기준) | def | 대응 source id |
|---|---|---|---|---|
| 38 | 背景 | Play/parts/common/bg/*.png | #default | 3 |
| 39 | グラフバー用背景 | Play/parts/sp_hw/graphbg/*.png | #default | 25 |
| 40 | 汎用BGA（動画） | Play/parts/common/BGA/movie/*.mp4 | #default | 23 (NO_BGA 곡일 때만 선언) |
| 41 | 汎用BGA（画像） | Play/parts/common/BGA/image/*.png | #default | 27 (NO_BGA 곡일 때만 선언) |
| 42 | キービーム | Play/parts/common/keybeam/*.png | #default | 14 |
| 43 | ボム（ModernChic規格） | Play/parts/common/bomb/*.png | `diamond SCUROed.` | 11 |
| 44 | ボム（OADX規格） | Play/parts/common/oadx_bomb/*.png | `DEFAULT` | 28 |
| 45 | ノーツ | Play/parts/common/notes/*.png | #default | 6 |
| 46 | 判定文字 | Play/parts/common/judge/*.png | #default | 4 |
| 47 | レーンカバー | Play/parts/common/lanecover/*.png | #default | 17 |
| 48 | リフト | Play/parts/sp_hw/lift/*.png | #default | 18 |
| 49 | フルコンボエフェクト | Play/parts/common/fullcombo/*.png | #default | 13 |
| 50 | キーイメージ | Play/parts/common/key/*.png | `harf` | 15 |
| 51 | キーフラッシュ | Play/parts/common/keyflash/*.png | #default | 16 |
| 52 | 判定ライン色 | Play/parts/common/judgeline/*.png | #default | 5 |
| 53 | グローランプ | Play/parts/common/glow/*.png | #default | 9 |
| 54 | プログレスランプ | Play/parts/common/progress/*.png | #default | 10 |
| 55 | ゲージMAXインジケータランプ | Play/parts/common/lamp/*.png | #default | 24 |
| 56 | ゲージ | Play/parts/common/gauge/*.png | #default | 29 |
| 57 | スクラッチイメージ | Play/parts/common/scratch/*.png | #default | 30 |

엣지 케이스: def 는 확장자를 뗀 파일명이다. `diamond SCUROed..png` 의 이름은 끝에 점이 하나 더 있는 `diamond SCUROed.` 이다. OADX 폴더에는 `dummy.png` 밖에 없어 def `DEFAULT` 에 해당하는 파일이 없다 (선택하지 않으면 사용되지 않음). 같은 패턴 문자열(`Play/parts/common/xxx/*.png`)이 `skin.source[].path` 에도 쓰이며 같은 선택값으로 치환된다 (BJ JSONSkinLoader.java:231-235 의 filemap).

### 1.4 offset (10개, id 40-49, `sp_property.lua:235-254, 452-463`)

| cat | id | name | 허용 축 | 사용처 |
|---|---|---|---|---|
| 58 | 40 | 背景の明るさ 0~255 (255で真っ暗) | a | background.lua:23 (`offset=`) |
| 59 | 41 | グラフエリア背景画像の明るさ | a | scorebar.lua:169 |
| 60 | 42 | BGAの明るさ | a | info.lua:315 (`offsets=`) |
| 61 | 43 | ターゲット差分、判定タイミングの位置 | x, y | assist.lua 의 `offsets` |
| 62 | 44 | ボムの大きさ 1~100%(範囲外は100%) | w | bomb.lua:28,35 (`PROPERTY.offsetBombSize.width()`) |
| 63 | 45 | レーンの明るさ 0~255 | a | lane.lua:35 |
| 64 | 46 | 小節線の明るさ 0~255 | a | notes.lua:108 (`alpha()`) |
| 65 | 47 | 判定ラインの高さ(0以下は12) | h | lane.lua:6 (`height()`) |
| 66 | 48 | グローランプの高さ(0以下は48) | h | lane.lua:53 |
| 67 | 49 | タイミンググラフの位置(プレイエリア側時) | x, y | graph.lua:218,223 |

허용 축은 header 의 `a = 0`, `h = 0`, `x = 0, y = 0`, `w = 0` 로 표현된다. beatoraja 의 Offset 헤더 필드는 `boolean x,y,w,h,r,a` 이고 Lua 값을 `toboolean` 하므로 숫자 0 도 true 가 된다 (BJ/skin/json/JsonSkin.java:78-88, LuaSkinLoader.java:99-100). `category` 는 문자열 필드이며 숫자 라벨은 `"58"` 처럼 변환된다.

런타임 접근: `skin_config.offset[이름]` 은 `{x, y, w, h, r, a}` 6개 필드를 항상 가진 테이블이다 (BJ/skin/lua/SkinLuaAccessor.java:956-979). 미설정 값은 0 이다.

beatoraja 가 play 스킨 헤더에 자동 추가하는 오프셋 4개 (JSONSkinLoader.java:167-191): `All offset(%)`(id 10, x/y/w/h), `Notes offset`(id 30, h 만), `Judge offset`(id 32, x/y/w/h/a), `Judge Detail offset`(id 33). ModernChic 은 30 과 32 를 destination 에서 참조한다. id 1-5 (SCRATCHANGLE, LIFT=3, LANECOVER=4, HIDDEN_COVER=5)는 엔진이 매 프레임 갱신하는 런타임 오프셋이다 (절 11.4).

### 1.5 category (14개 그룹, `sp_property.lua:469-587`)

`{name, item={cat번호...}}` 형식. 항목 번호는 위 표의 cat 열이다.

| 순서 | name | item |
|---|---|---|
| 1 | メインオプション | 1, 2, 3, 4, 34, 37 |
| 2 | プレイ状況詳細モード | 35 |
| 3 | 背景 | 38, 58 |
| 4 | グラフエリア | 39, 5, 6, 59 |
| 5 | BGA | 7, 8, 40, 41, 60 |
| 6 | ターゲットと判定タイミング | 9, 10, 11, 12, 13, 14, 61 |
| 7 | レーンカバー | 47, 36, 48 |
| 8 | キービーム | 42, 15, 16, 17 |
| 9 | ボム | 18, 19, 43, 44, 62 |
| 10 | ノート分布グラフ | 23, 24 |
| 11 | 判定タイミンググラフ | 25, 26, 27, 28, 29, 30, 67 |
| 12 | パーツ選択 | 45, 46, 49, 50, 51, 52, 53, 54, 55, 56, 57 |
| 13 | パーツ表示有無 (`table.insert(category, #category, ...)` 로 끝에서 두 번째 위치) | 7키: 20, 21, 22, 31, 33 / 5키: 20, 21, 22, 31, 32, 33 |
| 14 | オフセット（位置、大きさ、明暗調整） | 63, 64, 65, 66 |

`table.insert(list, pos, v)` 의 pos 위치 삽입 형식을 쓴다 (`sp_property.lua:567,578`). R-BMS 의 Lua 5.4 에서도 동일하게 동작한다.

## 2. 스킨 테이블 구성

### 2.1 main() 의 조립 순서 = destination z 순서 (`play7_hw.lua:15-306`, `play5_hw.lua:15-315`)

beatoraja 는 `sk.destination` 배열 순서대로 객체를 만들고 그린다 (BJ/skin/json/JSONSkinLoader.java:313-336, Skin.java:344-362). 아래 순서가 곧 아래쪽부터 위쪽으로 쌓이는 z 순서다. 각 파트는 `pcall(function() return dofile(get_path(...)).load(...) end)` 로 감싸여 있어 실패하면 그 파트만 조용히 빠진다. 예외로 `detailinfo/bgaareainfo.lua` 는 pcall 이 없다 (`play7_hw.lua:258-270`).

| z | 파트 | 파일 | load 인자 | 비고 |
|---|---|---|---|---|
| 1 | 배경 | background.lua | 3 (source id) | |
| 2 | 정보 영역 | sp/info.lua | | source 추가 가능(`char`) |
| 3 | 진행바 | sp/progress.lua | | |
| 4 | 키 프레임/키플래시 | sp/keyflash.lua | 7 / 5 | |
| 5 | 게이지 | sp/gauge.lua | | |
| 6 | 레인/글로/판정선 | sp/lane.lua | | |
| 7 | 키빔 | sp/inputkey.lua | 7 / 5 | |
| 8 | 노트 | sp/notes.lua | 7 / 5 | `skin.note` 대입 |
| 9 | 커버(레인커버/리프트/히든) | sp/cover.lua | | |
| 9b | 5키 커버 | play5_hw.lua:154-164 | | 5키 전용, `is5keyLanecoverOn` |
| 10 | FAST/SLOW, 목표차 | sp/assist.lua | | 기본값은 둘 다 OFF 라 destination 0개 |
| 11 | 폭발 | sp/bomb.lua | 7 / 5 | |
| 12 | 레벨, 판정 카운터 | sp/info2.lua | | |
| 13 | 판정 문자/콤보 | sp/judge.lua | | `skin.judge` 대입 |
| 14 | 그래프(분포, 타이밍) | sp/graph.lua | | |
| 15 | 풀콤보 연출 | sp/fullcombo.lua | | |
| 16 | 스코어바(목표/자기베스트) | sp/scorebar.lua | | |
| 17 | 공격 모션 | sp/attack.lua | | `isAttackModeOn` 일 때만 |
| 18 | BGA 영역 상세 정보 | sp/detailinfo/bgaareainfo.lua | 8 / 6 | `isDetailInfoSwitchOn` 일 때만 |
| 19 | 준비(로딩/연습) | sp/prepare.lua | | |
| 20 | 폐점(실패) 연출 | close.lua | 21 (source id) | |
| 21 | 종료 페이드 | play7_hw.lua:298-303 | | BLACK, timer FADEOUT(2), loop 500, a 0 -> 255 |

`CUSTOM.ADD_ALL(list, t)` 는 `ipairs` 로 table 을 이어붙이는 함수다 (`Root/define2.lua:14-20`). `skin.gauge`, `skin.note`, `skin.judge` 는 대입/이어붙이기 방식이 파트마다 다르다 (gauge, note 는 대입, judge 는 `ADD_ALL`).

### 2.2 최상위 키와 개수 (기본 설정, 7키, 1P 좌스크 / 좌측, 수기 집계)

기본 설정이란 절 1.2 의 def 값 전부, 일반 곡(BGA 있음), 오토플레이 아님, `CONFIG.*` 기본값이다.

| 키 | 개수 | 대표 예 |
|---|---|---|
| source | 27 (+1: BGA 없는 곡이면 id 23 또는 27) | `{id=1, path="Play/parts/sp_hw/system.png"}`, `{id=3, path=".../bg/*.png"}` (`Play/lua/base.lua:191-235`). 숫자 id: 1,2,3,4,5,6,9,10,11,13,14,15,16,17,18,20,21,22,24,25,26,28,29,30,31 + 문자열 id `"hcnBomb"`, `"adjusted"`. 공격모션 시 `"parts_attack"`, `"parts_back"` |
| font | 2 (TTF) 또는 3 (`.fnt`) | `{id=0, path="Play/font/ttf/mgenplus-1c-black.ttf"}`, id 1 = medium (`textproperty.lua:24-27,45-49`) |
| text | 13 | `rivalname, genre, title, artist, difftbl, pretitle, pregenre, preartist, from, per, slash, laneCoverCount, preview` |
| image | 약 223 (+1) | `{id="bg", src=3, x=0,y=0,w=1920,h=1080}` 등 |
| imageset | 0 | |
| value | 약 63 | `{id="nowscore", src=1, x=0,y=950,w=308,h=36, divx=11,divy=1, digit=6, ref=POINT, zeropadding=1}` |
| floatvalue | 0 | |
| slider | 6 | `progress`, `lane_cover`, `adjusted_cover`, `adjusted_main/min/max` |
| graph | 7 | `graph-now/-now-best/-best-bg/-best/-target-bg/-target`, `loading_bar` |
| gauge | 1 (id 2001) | 36 nodes, parts 50 |
| note | 1 (id "notes") | |
| hiddenCover / liftCover | 각 1 | |
| judge | 5 정의, destination 참조 1 | `def`, `laneCoverRest_1/2`, `constantRest_1/2` |
| judgegraph / bpmgraph | 2 / 1 | `judgegraph`(graph.lua), `notesgraph`(prepare.lua) / `bpmgraph` |
| timingvisualizer / hiterrorvisualizer | 각 1 | `timing` / `hrv` |
| bga | `{id="bga"}` | |
| customEvents / customTimers | 없음 | |
| destination | 약 288 (5키는 약 279) | 파트별 합계는 아래 |

집계 근거(파트별 destination, 7키): 배경 2, info 29, progress 7, keyflash 17, gauge 28, lane 8, inputkey 17, notes 1, cover 15, assist 0, bomb 18, info2 24, judge 1, graph 15, fullcombo 3, scorebar 34, prepare 38, close 30, 종료 페이드 1.

5키의 차이(개수): keyflash dst 15, inputkey 13, bomb image 32 / dst 14, notes lane 6, `5keysFrame` image 1 / dst 1 추가.

### 2.3 image / value / graph 정의 카탈로그 (src 사각형, 7키 기본)

source 별 시트 크기: `system.png`(1)=1920x1400, `info.png`(2)=1350x1350, `score.png`(22)=810x390, `bgainfo.png`(31)=1024x1024. 아래 좌표는 모두 `x,y,w,h`(원본 시트 좌측 상단 기준 픽셀)다. divx/divy 는 소스 영역을 균등 분할하며 프레임 순서는 행 우선(`images[divx*row + col]`)이다 (BJ JsonSkinObjectLoader.java:601-621).

background.lua:20-23: `bg` src3 (0,0,1920,1080).

info.lua
- `infoTitleFrame` src2 (0, `BASE.titleFramePosY`, 1350, 46) (왼쪽 배치 y=0, 오른쪽 배치 y=58). `infoFrame` src2 (0,110,1350,811).
- 숫자: `min_time`/`sec_time` src1 (1400,81,297,20) div 11x1 digit 2; `maxcombo` src1 (0,950,308,36) div 11x1 digit 5 ref MAXCOMBO2; `nowscore` 동일 시트 digit 6 ref POINT zeropadding 1; `nowexscore` src1 (1400,101,270,20) div 11x1 digit 4 ref SCORE2 zeropadding 1 (destination 없음); `maxbpm/nowbpm/minbpm` src1 (0,950,280,36) div 10x1 digit 4 align CENTER.
- `effecter_frame` src1 (810,1050,308,37); `effecter_nonactive` src1 (600,0,227,28); `effecter_active` src1 (830,0,227,28); `btn-lnmode_nonactive` src1 (600,28,227,84) divy 3 len 3 ref 308; `btn-lnmode_active` src1 (830,28,227,84) divy 3 len 3 ref 308.
- `lamp_rhythm` src24 (0,0,152,108); `lamp_gaugeinclease`, `lamp_maxgauge` src24 (0,0,152,216) divy 2 cycle 50 + timer (42 / 44).
- BGA 없는 곡: `bga_hanyo` src23 또는 27 (0,0,1280,720), `soundonly` src1 (570,270,600,350).
- 점수 플랩(옵션 ON): `roulette_number_1/2/3` src1 (0,950,280,36) divx 10 cycle 100/200/400.
- 구역 점수(`CONFIG.play.sectionScore.sw` 기본 false) 및 캐릭터(`bpmLinkChar.sw` 기본 false)는 미생성.

progress.lua:9-15: `progress_frame` src1 (1880,0,16,807); slider `progress` src10 (0,0,24,37) angle DOWN range 773 type MUSIC_PROGRESS(6).

keyflash.lua: `keyBg` src15 (0,0,408,114), `keyFrame` (0,114,408,114), `scratchBg` (408,0,111,114), `scratchFrame` (408,114,111,114), `scratchFrame2` (408,228,111,114), `keyCoverTop` (0,228,408,57), `keyCoverBottom` (0,285,408,57); `keyflash_n` src16 (0,0,67,73); `scratchImage` src30 (0,0,92,92).

gauge.lua: `gaugeFrame_1P` src1 (0,861,556,69), `gaugeFrame_2P` (600,861,556,69), `gaugeCover_1P` (0,1180,556,69), `gaugeCover_2P` (600,1180,556,69), `afterdot` (326,950,10,36); 게이지 칸 src29 (8x35 셀): `gauge-r1/r3` (0,0), `r2` (16,0), `b1/b3` (8,0), `b2` (24,0), `g1/g3` (32,0), `g2` (48,0), `p1/p3` (8,35), `p2` (24,35), `y1/y3` (0,35), `y2` (16,35), `h1/h3` (64,0), `h2` (72,0); 모드 라벨 src1 190x15: assist (1200,0), easy (1200,15), normal (1200,30), hard (1200,45), exhard (1200,60), hazard (1200,75), grade (1390,0), exgrade (1390,15), exhardgrade (1390,30). 값: `gaugenumber` src1 (0,950,280,36) div 10 digit 3 ref GROOVEGAUGE; `gaugenumber_afterdot` digit 1 ref GROOVEGAUGE_AFTERDOT.

lane.lua: `lane` src1 (37,0,519,857); `lane_1P` (30,1160,513,10); `lane_2P` (600,1160,513,10); `judge_line` src5 (0,0,513,12); `glow` src9 (0,0,431,48).

inputkey.lua:52-57: `keybeam-w` src14 (0,0,60,564), `keybeam-b` (70,0,48,564), `keybeam-s` (130,0,108,564).

notes.lua:113-198 (src6, 324x360 시트). 폭: white 60 (x=216), blue 48 (x=276), scratch 108 (x=108), a=auto 108 (x=0, 정의만 있고 사용 안 함). 행(y): 일반 0, lne 36, lns 72, lna 108, lnb 144 (h 72, divy 2, cycle 200), hcne 216, hcns 252, hcnb/hcnd 288, hcnr 324. `hcna` y=108 h 18. 지뢰는 src26 (324x36) x 동일, y 0 h 36. 알림: `note-w/b/s/a`, `lne-*`, `lns-*`, `lna-*`, `lnb-*`, `hcne-*`, `hcns-*`, `hcna-*`, `hcnb-*`, `hcnd-*`, `hcnr-*` 각 w/b/s/a, `mine-w/b/s`, `section-line` src1 (1,0,1,1). `CONFIG.play.notesAnimation`(기본 true)이면 lnb/hcnb/hcnr/hcnd 가 divy 2 + cycle(200/200/100/200); false 이면 divy 없이 h 36 또는 18.

cover.lua: `hidden_cover` hiddenCover src1 (0,0,1,1) disapearLine 227; `lift_cover` liftCover src18 (0,0,513,853) disapearLine 227; slider `lane_cover` src17 (0,0,513,853) angle DOWN range 853 type LANECOVER(4); slider `adjusted_cover` src17 range 853 value=function; sliders `adjusted_main/min/max` src "adjusted" (0,0,541,853) range 853 value=function; `lane_cover2` src17 (0,0,513,853); `liftDisplaySwitch` src1 (2,0,1,1) act=function. 숫자: 모두 src1, 시트 (1400,161,270,20)=녹색 숫자 계열 / (1400,101,270,20)=흰색 계열, div 10, digit 4.

assist.lua: `fast` src1 (1590,410,110,20), `slow` (1590,430,110,20); 숫자 `diffMybestTarget`, `diffExscoreTarget` src1 (1400,201,324,40) div 12x2 digit 5 zeropadding 1; `judgeTiming-ms` (1400,121,324,40) digit 4 zeropadding 1.

bomb.lua: 아래 절 10.4 참고. 폭발 시트는 400x300 셀 16열 x 4행 (6400x1200).

info2.lua: `info_level_frame` src1 (1575,300,175,91), `info_count_frame` (1200,300,371,103), `lev_beginner/normal/hyper/another/insame/unknown` src1 (1200,101+20i,160,20), 숫자 `playlevel` (1400,101,270,20) digit 2, `count_pg/gr/gd/bd/pr` + `-early/-late` src1 (1400,101/121/141,297,20) div 11 digit 4.

judge.lua: 절 10.3.

graph.lua: `infoNotesDistributionFrame` src2 (808,926,519,142), `notesCover` (780,1130,519,140), `infoJudgegraphFrame` (808,1072,519,54), `timingCover` (780,1280,519,54), `veryeasy..veryhard` src22 (200,180+30i,210,30), `tgBFrame` src1 (600,630,472,30), `tgBFrame2` (600,660,472,30), graph `gra_fastRate` (600,690,468,26), `gra_slowRate` (600,716,468,26), `timingAdjustBtn` src1 (2,0,1,1) act=74, 숫자 `judgeTimingNum` (1400,121,324,40) div 12x2 digit 4 align CENTER.

fullcombo.lua: `fc` src13 (0,0,5190,2571) div 10x3 cycle 1500 timer FULLCOMBO_1P (48); `wd_full` src1 (1200,1050,400,90); `wd_combo` (1200,1140,400,90).

scorebar.lua:94-150: `infoScoregraphFrame` src2 (26,926,771,200), `graphCover` (0,1130,771,198), `graph-bg` src25 (0,0,428,183), `bar-1` src22 (60,200,7,185), `bar-2` (70,200,7,185), `bar-max-l` (0,200,50,185), `bar-max-r` (100,200,50,185), `label-nowscore` (450,0,172,51), `label-bestscore` (450,60,172,51), `label-target_flame` (450,120,172,51), `now-{AAA,AA,A,B,C,D,E,F}-l` (450,180+22k,75,22) / `-r` (530,...), `score-display-frame` (630,190,168,198); 숫자 `num-exscore_player/_tar` src1 (1400,101,297,20) div 11 digit 4 ref SCORE2 / TARGET_SCORE; `num-diffExscoreMybest/-Target` (1400,201,324,40) div 12x2 digit 5 ref DIFF_HIGHSCORE/DIFF_TARGETSCORE align (RIGHT/LEFT, 막대 방향에 따라); graph(6개): src22 (0/0/0 ... y 0/0/60/60/120/120, 415x51) `divx=415` `cycle=20000` 은 now/best/target 3개, `type` SCORERATE(110), SCORERATE_FINAL(111), BESTSCORERATE(113), BESTSCORERATE_NOW(112), TARGETSCORERATE(115), TARGETSCORERATE_NOW(114), `angle=0`.

prepare.lua: `emblem` src1 (0,1260,996,74), `preautoinfo` (0,1050,801,108), `loadingwindow` (1200,410,380,311), `loading_barframe` (700,950,304,30), `loading_notesinfo` (1500,730,184,86), `loading_wd1` (1200,950,497,46), `loading_wd2` (1200,996,497,46); judgegraph `notesgraph` type NOTES(0); graph `loading_bar` src1 (700,1000,300,26) angle 2 type LOAD_PROGRESS(102); 숫자 `loading_par` (1400,101,297,20) digit 3 align LEFT ref LOADING_PROGRESS(165), `totalnote-normal/-scr/-ln/-bss` digit 4 ref 350/352/351/353.

close.lua:12-24: src21 (2200x2100) `close_frame_top` (0,0,1920,565), `close_frame_bottom` (0,600,1920,565), `close_msg_window` (0,1200,1920,317), `close_msg_arrow` (0,1600,2200,105), `close_msg_arrow2` (0,1900,2200,66), `close_word_stage` (250,1750,700,150), `close_word_failed` (950,1750,730,150), `close_quickRetry` (0,1980,800,108), `close_beam` (1981,1,57,19) divx 3 cycle 100.

### 2.4 알려진 정의-사용 불일치 (그대로 재현해야 하는 동작)

- `info.lua:375` destination `id = "lamp"` 는 어느 image 에도 없다. beatoraja 는 id 불일치 destination 을 `obj == null` 로 건너뛴다 (JSONSkinLoader.java:323-336). R-BMS 도 미정의 id 를 조용히 무시해야 한다.
- `info.lua:212-216` 의 `soundonly` destination 은 `isNoBGA()`(옵션)일 때만 만들어지지만, `soundonly` image 는 `generalBGA()` 의 `isHanyoDisable` 분기에서만 정의된다. 조합에 따라 정의 없는 id 참조가 생기고 그 경우 무시된다.
- `bgaareainfo.lua:603` `id = "num"..i -1` 은 Lua 우선순위상 `"num"..(i-1)` 이다. destination 은 num0..num9, 정의는 num1..num10 이므로 num0 은 정의 없음(무시), num10 은 배치되지 않는다 (상세 모드 한정).
- 5키 폭발 LN 행 오프셋은 7키와 순서가 다르다: 5키 `modernchicLnPosY = {900,300,600,300,600,300}`, 7키 `{300,600,300,600,300,600,300,900}` (`bomb.lua:21,47`). 5키는 1번 키가 900 행(스크래치 LN 행), 스크래치가 300 행을 쓴다. 원본 동작 일치가 목표라면 이 순서를 그대로 둔다 (버그 호환).
- `info.lua:186` `draw = not CUSTOM.OP.isMainBpm` 은 함수가 아니라 Lua boolean `false` 가 된다. beatoraja 는 boolean 을 속성 값으로 인정하지 않고 null 로 처리하므로 draw 조건이 없는 것과 같다 (절 11.1). 결과적으로 `nowbpm` 기본 색 destination 은 항상 그려지고, `draw = CUSTOM.OP.isMainBpm`(함수) destination 이 그 위에 녹색(147,204,44)으로 덧그려진다.

## 3. 객체 기능 사용 목록

개수는 SP 플레이 관련 파일 전체(`play7_hw/play5_hw/background/close/sp/*/detailinfo`)에서 grep 한 값이다 (detailinfo 와 공격모션 포함, 기본 OFF 인 것도 포함).

### 3.1 destination 필드

| 필드 | 사용 | 값/대표 위치 |
|---|---|---|
| time, x, y, w, h | 전부 | 첫 keyframe 에서 생략된 값은 0 (a,r,g,b 는 255). 이후 keyframe 은 직전 값 상속 (JSONSkinLoader.java:422-468) |
| a, r, g, b | 다수 | info.lua:32 `r,g,b = RGB`(난이도 색), gauge.lua:196 `PERFECT_COLOR` 등 |
| angle | 소수 | play5_hw.lua:159 `angle=BASE.FIVEKEY_COVER_ANGLE`(180 또는 0), attack.lua 의 회전(-360, 360, -45, 45) |
| acc | 27 | `DECELERATE`(2) 20회, `ACCELERATION`(1) 5회, `CONSTANT`(0) 2회. 3(DISCONTINUOUS)은 미사용 |
| timer | 약 200 | JUDGE_1P(46) 66회, PLAY(41) 32회, ENDOFNOTE_1P 17회, FAILED 13회, RHYTHM 12회, READY 6회 등. 함수 timer 는 절 4 |
| loop | 약 130 | `-1` 89회, `1000` 12회, `0` 7회, `750` 6회, `3000` 6회, `1500` 4회, `500/600/800/2500` 및 변수(`time_keyoff`, `frameClosetime`) |
| blend | 32 | `ADDITION`(2) 24회, `ALPHA`(1) 7회, `blend = 9`(반전) 1회 (bgaareainfo.lua:650, 상세 모드) |
| filter | 11 | `FILTER.OFF`(0) 10회, `FILTER.ON`(1) 1회(keyflash.lua:72 스크래치 이미지) |
| op | 약 390 | 숫자만(절 5.1). 음수 포함. 배열 안 여러 개는 AND |
| draw | 82 | 함수 80, boolean 2 (info.lua:186 boolean `false`, :191 함수). 절 4 |
| offset | 92 | 단일 id. `MAIN.OFFSET.LIFT`(3), `NOTES_1P`(30), `SCRATCHANGLE_1P`(1), 사용자 id(40,45 등) |
| offsets | 41 | 배열: `{LIFT, JUDGE_1P}`, `{LIFT, 사용자id}`, `{사용자id}` |
| stretch | 9 | `FIT_OUTER_TRIMMED`(3) info.lua:251,259 / prepare.lua:200, `FIT_WIDTH_TRIMMED`(5) keyflash.lua:84,91,98,99 / prepare.lua:10 (STAGEFILE), `FIT_INNER`(1) prepare.lua:17 (BACKBMP) |
| center, clip_*, mouseRect | 0건 | 미사용 |

### 3.2 객체 필드

| 필드 | 사용 | 위치/값 |
|---|---|---|
| act | 2 | cover.lua:152 (`function() changeLiftDisplay() end`, 인자 0개), graph.lua:276 (`MAIN.BUTTON.JUDGE_TIMING` = 74) |
| click | 0 | 기본값 0 (plus only) |
| ref | 약 92 | 숫자 id. image 의 `ref` 는 info.lua:332-333 `MAIN.BUTTON.LNMODE`(308)로 이미지 세트 인덱스 선택 (`len=3`) |
| value (함수) | 약 61 | 절 4 |
| timer (image 정의) | 다수 | 숫자 (info.lua:371-372 `GAUGE_INCLEASE_1P`/`GAUGE_MAX_1P`, fullcombo.lua:10, bomb.lua 의 BOMB/HOLD 타이머, 5-8 번 cascade 함수 timer) |
| cycle | 51 | image/value/judge 이미지의 프레임 순환 주기(ms). 식: `index = (time * frames / cycle) % frames`, `time` 은 timer 가 있으면 timer 이후 경과, 없으면 스킨 시간 (BJ/skin/SkinSourceImage.java) |
| divx / divy | 118 / 135 | 균등 분할 |
| len | 2 | info.lua:332-333 |
| align | 숫자 9 / 텍스트 26 | 숫자: `N_ALIGN.RIGHT=0, LEFT=1, CENTER=2`. 텍스트: `T_ALIGN.CENTER=1` 26회 |
| zeropadding | info.lua:129-130, assist.lua:82-86, graph.lua:264(OFF) | 절 11.3 참고 (11분할 숫자는 loader 가 무시하고 2 로 강제) |
| digit | 117 | |
| padding, space, wrapping, isRefNum, min, max, changeable(플레이 일반), outlineColor | 0 | 상세 모드의 slider `changeable=false` 3회만 사용 |
| overflow | 16 | `T_OVERFLOW.SHRINK`(1) 전부 (`textproperty.lua`) |
| constantText | 10 | `from "~"`, `per "%"`, `slash "/"`, `laneCoverCount "LANECOVER"`, `preview "Pattern Preview..."` |
| shadowOffsetX/Y | 15 | 정수 2 또는 4 (`textproperty.lua:7-8`) |
| shadowColor | 10 | `"00000000"` (constantText 에만). 나머지는 기본 `"ffffff00"` |
| range | 9 | slider 의 이동 거리 px |
| type | 19 | slider/graph/judgegraph 의 종류 id |
| angle (slider/graph) | slider `DOWN`(2), graph `0`(오른쪽 성장) / `1`(아래에서 위 성장), prepare.lua:89 `angle = 2`(loading_bar, graph 이므로 "그 외"=오른쪽 성장) | BJ/skin/SkinGraph.java 는 `direction == 1` 만 수직으로 취급 |
| disapearLine | 4 | cover.lua:10,12, 227 |
| size, font | text | `font` 는 폰트 id 문자열 비교 (`"0"`, `"1"`) |

숫자 객체의 `ref` 와 `value` 는 둘 중 하나다. judge 안의 `numbers` value 는 ref 가 `MAXCOMBO` 로 적혀 있지만 SkinJudge 가 콤보 값을 직접 넣으므로 ref 는 쓰이지 않는다 (SkinJudge.java:124).

## 4. Lua 함수가 값으로 들어가는 자리 전수

beatoraja 규칙: 함수는 `BooleanProperty/IntegerProperty/FloatProperty/StringProperty/TimerProperty/Event/FloatWriter/StringWriter` 로 감싸지고 매 `prepare` 마다(보통 프레임당 1회) 호출된다. 실행 중 `RuntimeException`(Lua 오류 포함)은 잡혀서 Boolean=false, Integer=0, Float=0, String="", Timer=`Long.MIN_VALUE`(꺼짐) 가 반환되고 경고만 로그에 남는다 (BJ/skin/lua/SkinLuaAccessor.java:611-747). 함수에 부수효과(상태 변수 갱신, 오디오 재생, 파일 쓰기)가 있어도 호출 횟수가 프레임마다 반복되므로 R-BMS 도 beatoraja 와 같은 호출 횟수/조건(그리기 조건을 통과한 객체에 대해서만 timer/value 평가)을 지켜야 한다.

### 4.1 draw = function (기본 설정에서 실제로 실행되는 것)

| 위치 | 읽는 API | 부수효과 |
|---|---|---|
| info.lua:98-123 (`isRemainSec(60)` 4회) | `number(163)`, `number(164)` | 없음 |
| info.lua:191 (`CUSTOM.OP.isMainBpm`, 함수 참조 자체) | `number(160)`, `number(92)`, `number(91)`, `number(90)` | 없음 |
| info.lua:205-209 (BLACK 0x0 의 연습 플래그) | `option(1080)` | 클로저 변수 `pflg` 를 false 로 영구 변경 |
| info.lua:251,259,267,275 (`return pflg`/`not pflg`) | | pflg 읽기 |
| gauge.lua:123,130 (`isBrinkGauge`) | `gauge_type()` | 없음 |
| gauge.lua:215 (모드 라벨 9개) | `gauge_type()` | 없음 |
| gauge.lua:227 (게이지 가림) | `timer(143) == timer_off_value` | 없음 |
| assist.lua:108,116 | `float_number(113)`, `option(32)` | 없음 (기본 OFF) |
| bomb.lua:168,177 (`isHcnPattern`) | `event_index(308)`, `option(173)` | 없음 (`CONFIG.play.hcnBomb` 기본 false 라 미생성) |
| cover.lua:18 | upvalue `isLiftDisplay` | `act` 가 토글 |
| judge.lua:115-340 (함수 48개) | 아래 4.2 | 없음. 기본 설정에서는 `def` 만 참조하므로 실행 안 됨 |
| background.lua:6-8,17 | (비활성) | `CUSTOM.SOUND.fcSound`/`achievementVoice` 오디오 재생, 상태 플래그 변경 |
| attack.lua, bgaareainfo.lua | | 절 4.5 |

### 4.2 value = function

- cover.lua:29,35,39,43 (슬라이더 4개): `CUSTOM.SLIDER.adjustedCover()[1..4]`. 읽는 API: `number(14)`, `number(314)`, `option(272)`, `event_index(55)`, `number(160)`, `number(90)`, `number(91)`, `number(92)`. `event_index(55)` 가 2,3,4 가 아니면 nil 을 반환하므로 `[1]` 에서 Lua 오류가 난다. 해당 슬라이더 destination 에는 `op={CONSTANT(400), ...}` 가 있어 CONSTANT 가 꺼진 상태에서는 prepare 의 draw 조건 단계에서 탈락해 value 를 평가하지 않는다 (SkinObject.java:591-598 과 SkinSlider.prepare). 그래도 R-BMS 는 value 함수 오류 시 0 을 반환하고 로그를 매 프레임 쌓지 않아야 한다.
- cover.lua:172-173: `usedLaneCoverCount`, `allLaneCoverCount` (로테이션 ON 일 때만): 로드 시점에 계산한 상수 반환.
- info.lua:401-416 (구역 점수 16개): `CUSTOM.NUM.sectionScore*`, `sectionTime`. `number(101)`, `number(152/153)`, `number(1163/1164)`, `timer(41)`, `time()`. `sectionScoreN()` 는 최초 도달 시 상태 테이블(`CUSTOM.sectionScore.*`)을 변경한다. 기본 OFF.
- info.lua:428 (graph): `CUSTOM.GRAPH.sectionRemainRate()` 상태 테이블 변경. 기본 OFF.
- graph.lua:233-241 (막대형 타이밍): `CUSTOM.GRAPH.FastRate/SlowRate` (`number(423)`, `number(424)`). 분모 0 이면 0/0 = NaN 이 반환된다 (Lua 의 실수 나눗셈).
- attack.lua:56-79 (값 3개): `maxExscore()`, `number(101)`. 기본 OFF.
- textproperty.lua:36,58: `preartist` text 의 `value = function() return createFullArtist() end` (`text(15)`, `text(14)`). 기본 설정(`isOutlineFont`)에서 실제 사용.
- bgaareainfo.lua (31개): 상태 누적 함수들. 상세 모드 한정.

### 4.3 timer = function

- cover.lua:165-171: `exceptLanecover` (로테이션 ON): 반환값 nil 이므로 `tolong()` 이 0 이 되고(타이머가 켜진 것으로 간주) destination 은 a=0 이라 그려지지 않지만 함수 본문은 프레임마다 실행된다. 부수효과: `ENDOFNOTE_1P` 가 켜지면 1회 `CUSTOM.FUNC.randomChoiceStep2` (파일 읽기/쓰기).
- attack.lua:6-42,195-196,209-234: `createCascadeTimersPGreat/Great` 가 만드는 클로저 타이머 160개(= 8 x 10 x 2). `option(241)`, `option(1242)`, `option(1243)`, `timer(50+i-1)`. 상태는 클로저 변수.
- bgaareainfo.lua:574-757: 21개 함수 timer (`main_state.time() - ...` 식으로 값을 만들어 시간에 따른 색 변화를 애니메이션 키로 쓰는 트릭).
- 숫자 timer 배열: info.lua:148-170 (BOMB 타이머), bomb.lua, inputkey.lua (KEYON/KEYOFF), attack.lua:195 (`49+i`).

### 4.4 act

- cover.lua:152: 인자 0개 함수 (`loadEvent` 가 `function.narg()==0` 로 분기, SkinLuaAccessor.java:759-788). 좌클릭 시 리프트 표시 토글.
- graph.lua:276: 숫자 74 (`BUTTON_JUDGE_TIMING`). 좌/우 버튼 이벤트가 `+1/-1` 로 전달된다 (SkinObject.java:671-704).
- 두 객체 모두 1x1 투명 픽셀(`system.png` (2,0))을 원하는 영역에 늘려 그린 보이지 않는 클릭 영역이다. 이미지의 알파가 0 이어도 클릭 판정은 `draw` 플래그가 true 인 동안 작동하므로, R-BMS 의 히트 테스트는 이미지 알파가 아니라 draw 플래그/영역을 기준으로 해야 한다.

### 4.5 기타

- `customEvents`, `customTimers`: SP 플레이 경로에서 사용 없음.
- `skin.judge[].images[].draw`: judge.lua 의 `laneCoverRest/constantRest` (읽는 API: `number(14)`, `option(272)`, `number(314)`, `event_index(55)`, `number(160/90/92/91)`). 기본 설정(`valiableJUDGE.sw=false`)에서는 destination 이 참조하지 않는다.

## 5. 참조하는 속성 id 전수

의미는 `Root/main*.lua` 의 이름. 값은 BJ/skin/SkinProperty.java 와 대조해 일치함을 확인했다 (OPTION_GAUGE_HARD=43, OPTION_STATE_PRACTICE=1080, OPTION_POOR_EXIST=2245, TIMER_RHYTHM=140 등).

### 5.1 옵션 (op / `main_state.option`), 음수는 NOT

| id | 이름 | 쓰는 파일 |
|---|---|---|
| 32 | AUTOPLAYOFF | assist, background, close, info, inputkey, keyflash, bgaareainfo |
| 33 | AUTOPLAYON | inputkey, keyflash, prepare, bgaareainfo |
| 80 | NOW_LOADING | graph, prepare, progress |
| 81 | LOADED | keyflash, prepare, progress |
| 84 | REPLAY_PLAYING | prepare, bgaareainfo |
| 1080 | STATE_PRACTICE | info, prepare |
| 43 | GAUGE_HARD | gauge (음수형 포함) |
| 230, 231, 232 | GAUGE_1P_0_9 / 10_19 / 20_29 | gauge |
| 240 | GAUGE_1P_100 | info (`-240`) |
| 150-155 | DIFFICULTY0..5 | common(setRGB), info2 |
| 170 / 171 | NO_BGA / BGA | base, info |
| 172 / 173 | NO_LN / LN | info, customoption(isHcnPattern 등) |
| 176 / 177 | NO_BPMCHANGE / BPMCHANGE | info, cover |
| 180-184 | JUDGE_VERYHARD..VERYEASY | graph |
| 191 / 194 / 195 | STAGEFILE / NO_BACKBMP / BACKBMP | prepare |
| 200-207 | AAA_1P..F_1P | scorebar |
| 220, 221, 222 | AAA / AA / A (스코어 도달) | scorebar |
| 241 | PERFECT_1P | attack |
| 1242 / 1243 | EARLY_1P / LATE_1P | assist, attack, bomb |
| 2243 / 2244 / 2245 | GOOD_EXIST / BAD_EXIST / POOR_EXIST | gauge, progress |
| 270 / 271 / 272 | LANECOVER1_CHANGING / LANECOVER1_ON / LIFT1_ON | cover, prepare, customoption, customnumber |
| 290 | MODE_COURSE | close (`-290`) |
| 400 | CONSTANT | cover, judge |

`main_state.option(n)` 에서 `n < 0` 은 NOT, 알 수 없는 id 는 false (BJ BooleanPropertyFactory.java:29-67, MainStatePropertyLuaApiExporter.java:159-165).

### 5.2 숫자 (value ref / `main_state.number`)

PLAY 계열: 75 MAXCOMBO(judge), 105 MAXCOMBO2, 100 POINT, 101 SCORE2(EX), 121 TARGET_SCORE, 152 DIFF_HIGHSCORE, 153 DIFF_TARGETSCORE, 107 GROOVEGAUGE, 407 GROOVEGAUGE_AFTERDOT, 160 NOWBPM, 90 MAXBPM, 91 MINBPM, 92 MAINBPM, 163/164 TIMELEFT_MINUTE/SECOND, 96 PLAYLEVEL, 12 JUDGETIMING, 525 JUDGE_1P_DURATION, 165 LOADING_PROGRESS, 350-353 TOTALNOTE_NORMAL/LN/SCRATCH/BSS.

판정 카운터: 110/111/112/113/114 = PERFECT/GREAT/GOOD/BAD/POOR, 410/412/414/416/418 = EARLY_*, 411/413/415/417/419 = LATE_*, 423 TOTALEARLY, 424 TOTALLATE, 427 BAD_PLUS_POOR_PLUS_MISS(상세 모드).

레인커버/지속 시간: 14 LANECOVER1(0-1000), 314 LIFT1(0-1000), 312 DURATION, 313 DURATION_GREEN, 1312 DURATION_LANECOVER_ON, 1313 DURATION_GREEN_LANECOVER_ON, 1314 ..._OFF, 1315 GREEN_..._OFF, 1316-1319 MAINBPM_*(ON, GREEN_ON, OFF, GREEN_OFF), 1320 MINBPM_ON, 1321 MINBPM_GREEN_ON, 1322 MINBPM_OFF, 1324 MAXBPM_ON, 1325 MAXBPM_GREEN_ON, 1326 MAXBPM_OFF.

보조: 74 TOTALNOTES, 1163/1164 SONGLENGTH_MINUTE/SECOND, 161/162 PLAYTIME, 360-365 DENSITY_*, 21-26 TIME_*, 370 CLEAR, 371 TARGET_CLEAR, 179 IR_RANK, 182 IR_PREVRANK, 170/171 HIGHSCORE2/SCORE3. 마지막 그룹은 Root/custom*.lua 의 함수 안에서만 쓰이고 SP 플레이 destination 이 참조하지 않는다.

`value == -2147483648` 비교가 attack.lua:61 등에 있다 (`number(101)` 이 미정일 때 반환되는 값으로 추정, 미확인).

### 5.3 타이머

2 FADEOUT, 3 FAILED, 40 READY, 41 PLAY, 42 GAUGE_INCLEASE_1P, 44 GAUGE_MAX_1P, 46 JUDGE_1P, 48 FULLCOMBO_1P, 50 BOMB_1P_SCRATCH, 51-57 BOMB_1P_KEY1..7, 70 HOLD_1P_SCRATCH, 71-77 HOLD_1P_KEY1..7, 100 KEYON_1P_SCRATCH, 101-107 KEYON_1P_KEY1..7, 120 KEYOFF_1P_SCRATCH, 121-127 KEYOFF_1P_KEY1..7, 140 RHYTHM, 141 PREVIEW, 143 ENDOFNOTE_1P, 351 SCORE_BEST, 352 SCORE_TARGET (이 둘은 voice 설정이 켜진 경우 customsound 만 참조), 11 SONGBAR_CHANGE/172-174 IR_CONNECT_*(select/result 전용 함수).

타이머가 꺼진 값은 `Long.MIN_VALUE`, Lua 에는 `main_state.timer_off_value` 로 노출된다 (MainStatePropertyLuaApiExporter.java:20,39). 시간 단위는 마이크로초다 (`main_state.time()`, `timer()`, SkinLuaAccessor 의 주석 `start time in microseconds`).

### 5.4 문자열 (text ref / `main_state.text`)

1 RIVAL, 12 FULLTITLE, 13 GENRE, 14 ARTIST, 15 SUBARTIST, 1003 TABLE_FULL. 그 외 customfunction/customoption 내부: 1010 VERSION, 1020 IR_NAME, 150/155 COURSE1_TITLE/COURSE6_TITLE. bgaareainfo.lua:170 은 숫자 리터럴 `text(12)`.

### 5.5 버튼/이벤트

- `event_index`: 308 LNMODE(image ref 와 `isHcnPattern` 등: 0=LN, 1=CN, 2=HCN), 55 HSFIX(0 OFF, 1 START, 2 MAX, 3 MAIN, 4 MIN).
- `act`: 74 JUDGE_TIMING. 301-307 ASSIST_*, 350 등은 Root/customoption 의 `isAssistOn` 에만 있고 SP 플레이는 호출하지 않는다.
- 주석 처리된 `MAIN.BUTTON.TARGET`(77)은 scorebar.lua:109 에만 있다 (사용 안 함).

### 5.6 슬라이더/그래프 type, 이미지 id

- slider type: 4 LANECOVER, 6 MUSIC_PROGRESS (`MAIN.SLIDER.*`). 모두 BJ 상수와 일치.
- graph type: 102 LOAD_PROGRESS, 110 SCORERATE, 111 SCORERATE_FINAL, 112 BESTSCORERATE_NOW, 113 BESTSCORERATE, 114 TARGETSCORERATE_NOW, 115 TARGETSCORERATE. `main_state.float_number(113)` 도 사용한다 (isFirstPlay2).
- judgegraph type: 0 NOTES, 1 JUDGE, 2 FASTSLOW. `backTexOff`: 0 OFF, 1 ON.
- 내장 이미지 id (destination 의 `id` 숫자): -100 STAGEFILE, -101 BACKBMP, -110 BLACK, -111 WHITE. `Integer.parseInt(dst.id) < 0` 이면 `SkinImage(-id)` 로 처리 (JSONSkinLoader.java:316-321).
- offset: 1 SCRATCHANGLE_1P, 3 LIFT, 4 LANECOVER, 30 NOTES_1P, 32 JUDGE_1P, 사용자 40-49.

## 6. Lua 런타임 의존

### 6.1 표준 라이브러리 사용 (SP 플레이 + Root 경로, 실행 도달 기준)

| 라이브러리 | 사용 | 비고 |
|---|---|---|
| table | `insert`(668회, 위치 지정 형식 포함), `concat` | |
| string | `format("%02d")`, `sub`, `match`, `gsub`, `gmatch` | 패턴 `"Play.+%.png"`, `"\\"` -> `"/"`, `"([^\t]+)"` (customfunction.lua:125, bgaareainfo/nentyakuinfo) |
| math | `floor`, `random`(5), `modf`, `exp` | `math.random(a,b)` 는 키플래시 색(keyflash.lua:12-14)에서 로드마다 호출됨 |
| os | `date("%y%m%d")`, `date("%Y/%m/%d")`, `date("%H:%M:%S")`, `date('*t').wday`, `time()` | customtime.lua:6-10 은 `Root.customnumber` require 시 실행. `os` 가 없으면 로드 실패 |
| io | `open`(r/w/a), `file:lines/write/close` | `Root.customnumber` 최상위가 로드마다 10개 경로를 읽기 시도(없으면 시작값 반환). `CONFIG.infoOutput`(기본 true) 때문에 `History/information.txt` 를 매 플레이 로드마다 쓴다 (background.lua:29, customfunction.lua:319-367) |
| package/require | 점 구분 모듈명 | `require("Play.lua.require.sp_property")` 등 서로 다른 모듈명 35종(Play.lua 계열 10, Root 계열 22, `config`, `main_state`, `luajava`), `require("config")`, `require("main_state")`, `require("luajava")` |
| 기본 함수 | `pcall`(38), `dofile`(40), `pairs`, `ipairs`, `tostring`, `tonumber`, `print`(41) | `loadfile/loadstring/setfenv/getfenv/select/rawget/rawset/setmetatable/unpack` 전역 사용 없음 |
| luajava | `bindClass("java.io.File")`, `luajava.new(File, path)`, `dir:mkdir()`, `dir:listFiles()`, `#filelists`, `tostring(filelists[i])` | customfunction.lua:7-8,102-134. 이 파일은 `Root.define2` -> `Root.customnumber` -> `Root.customfunction` 로 main() 시작 시 항상 require 되며 최상위에서 `require("luajava")` 가 실행된다. 없으면 main() 전체가 실패한다 |
| debug, coroutine, bit32 | 사용 없음 | |

LuaJ(5.2)/5.1 전용 문법 점검: `unpack`(util.lua 의 자체 함수 이름일 뿐 전역 `unpack` 호출은 없음), `setfenv`, `string.gfind`, `math.pow`, `table.getn`, `bit32`, `loadstring` 모두 없음. `#` 길이 연산은 정상 배열에만 쓴다. 5.4 에서 주의할 점은 다음과 같다.
- 숫자 서브타입: 5.4 는 정수와 실수가 구분된다. `main_state.time()/timer()` 를 i64 로 노출하면 `timer_off_value`(= `i64::MIN`)와 `now - timer` 에서 정수 래핑이 일어난다 (LuaJ 는 double 이라 거대한 양수가 됨). 영향이 있는 곳은 `CUSTOM.NUM.elapsedTimeFromStart` 와 `CUSTOM.GRAPH.sectionRemainRate`(기본 OFF 라 결과는 같음)이고, 권장은 `time()` 과 `timer()` 를 Lua 실수(f64)로 노출하는 것이다. `timer == timer_off_value` 비교는 정수/실수 혼합에서도 값이 같으면 참이다.
- 실수 -> 정수: destination 의 정수 필드에 실수가 들어간다 (`bombWidth / 2`, `(i-1)*lgwidth`, `bombCycle / 4 = 62.75`, `keybeam_height = 564*0.5`). beatoraja 는 `LuaValue.toint()` 로 0 방향 절삭한다 (LuaSkinLoader.java:101-102). 반올림이 아니다.
- 함수가 반환하는 실수 value(예: `songLimitSec / 60`)도 `toint()` 절삭이다. 숫자 `ref/value` 반환형은 `int`.
- `string.format("%d", float)` 호출은 없다. `string.format("%02d", #lined)` 는 정수다.

### 6.2 전역 변수/모듈 상태

- 전역 대입: `main_state`(play7_hw.lua:8), `PROPERTY`, `COMMONFUNC`, `MAIN`, `CUSTOM`, `CONFIG`, `BASE`(main() 안에서 대입), `DEBUG`(nentyakuinfo.lua:18 `DEBUG = false`, 주석 처리된 `DEBUG = true`).
- 모듈 캐시 의존: 같은 Lua 상태에서 헤더 실행 -> `skin_config` 설정 -> 본 실행(`.luaskin` 재실행) 순서다. `require("play7_hw")` 는 두 번째 실행에서 캐시를 반환하므로 `PROPERTY.load(false)` 는 한 번만 실행된다 (SkinLuaAccessor.execFile 과 LuaSkinLoader.load: 헤더 -> `exportSkinProperty` -> 본 실행). 헤더 실행 시 `skin_config` 는 nil 이다 (`.luaskin` 이 `if skin_config then return t.main() else return t.header end` 로 분기, MC `play7_hw.luaskin:1-6`).
- 헤더 단계 요구: `require("main_state")` 가 빈 테이블을 돌려줘야 한다 (SkinLuaAccessor.java:95-102 가 `main_state/timer_util/event_util` 을 빈 테이블로 미리 등록). 헤더 단계에서 `main_state.*` 함수를 호출하는 코드는 없다.
- `require` 해석: 스킨 폴더를 `package.path` 에 `<skin폴더>/?.lua` 로 추가한다 (SkinLuaAccessor.setDirectory, 비샌드박스는 기존 path 뒤에 추가). `dofile` 은 `skin_config.get_path()` 로 얻은 절대 경로를 받는다.

### 6.3 `skin_config` / `main_state` API 실제 사용 (SP 플레이 도달 경로)

- `skin_config.option[이름]` -> 선택된 op id (정수). `skin_config.offset[이름].{a,w,h,x,y}` (SkinLuaAccessor.java:937-979). `skin_config.get_path(상대경로)` (146회): 스킨 폴더 + 경로, 와일드카드는 선택 파일로 치환한 절대 경로 문자열. `skin_config.file_path[이름]` 은 SP 플레이에서 사용 없음. `skin_config.enabled_options` 도 사용 없음.
- `main_state`: `number`(127 호출 위치), `option`(88), `time`(20), `text`(17), `timer`(16), `timer_off_value`(15), `event_index`(12), `gauge_type`(10), `float_number`(2), 기능이 켜진 경우만 `audio_play`(82, 경로 + 볼륨, 볼륨 0.0001 은 프리로드), `judge`, `rate`, `exscore`, `volume_sys/key/bg`, `gauge`(customnumber 안의 함수, SP 플레이가 호출하지 않음).
- 반환형: `option` boolean, `number` 정수, `float_number` 실수, `text` 문자열, `timer` 마이크로초(켜짐) / `Long.MIN_VALUE`(꺼짐), `time` 마이크로초, `gauge_type` 0..8, `event_index` 정수.

### 6.4 beatoraja 포크가 제공하는 보조 API (ModernChic 이 의존)

현재 BJ 소스는 표준 LuaJ 와 다르다. 플레이 로딩은 `new LuaSkinLoader(state, config)` -> `SkinLuaAccessor(false)`(비샌드박스) 경로다 (BJ/skin/SkinLoader.java:68). 이 경로는 다음을 제공한다 (SkinLuaAccessor.java:46-93, LegacySkinLuaApi.java:51-146).
- `luajava` 모듈 대체물: `bindClass("java.io.File")`(+ Gdx/Input/Controllers 일부), `new(File, path)` 가 `{mkdir, listFiles}` 테이블 반환. `listFiles` 는 1부터 시작하는 경로 문자열 배열(`\` 를 `/` 로)을 반환하고 실패 시 nil. 스킨 폴더 밖 경로는 오류.
- `debug` 는 `getmetatable` 만, `os` 는 `execute/exit/getenv/remove/rename/tmpname` 만 제거, `io` 는 스킨 폴더 안으로 제한(읽기 실패 시 `io.open` 이 nil 반환).
- 별도 `LuaSkinLoader.sandboxed(path)`(스킨 선택 화면의 헤더 읽기, SkinConfiguration.java:502)는 `os`, `luajava`, `debug` 를 아예 제거하고 `io` 쓰기를 메모리에 버린다. ModernChic 의 헤더 경로(`play7_hw.lua` 최상위, `sp_property`, `header`, `common`)는 이 제한 안에서도 동작한다 (os/luajava/io 를 호출하지 않음).
- `main_state` 확장: `file_exists/file_mkdir/file_list/file_read_lines/file_write/file_append/file_clear/file_count_lines`, `http_get_lines/http_get`, `audio_play/audio_loop/audio_preload/audio_stop/audio_dispose`, `timer_is_on/timer_is_off/timer_elapsed*`, `set_timer`, `event_exec`, `offset`, `numbers`, `key_pressed`, `screen_width/height` (MainStatePropertyLuaApiExporter.java:33-93). ModernChic 은 이 확장 API 를 쓰지 않는다 (표준 `io`/`luajava` 를 쓴다).
- `audio_play(path, volume)`: volume nil 이면 1, `[0,2]` 로 clamp, 시스템 볼륨을 곱한다 (SkinAudioLuaApiExporter.java:57-60).

## 7. 1920x1080 레이아웃

### 7.1 BASE 계산식 (`Play/lua/base.lua:117-159`, SP 만)

공통 상수: `NOTES_JUDGE_Y = 227`, `LANE_LENGTH = 853`.

플레이 위치(레인 위치)에 따른 분기 (`PROPERTY.isLeftPosition / isRightPosition`):

| 값 | 좌측 표시 (op 902) | 우측 표시 (op 903) |
|---|---|---|
| playsidePositionX (P) | 51 | 1350 |
| infoPositionX (I) | 570 | 0 |
| progressbarPositionX | 23 | 1881 |
| gaugePositionX (G) | 14 | 1350 |
| titleFramePosY (info.png 의 제목바 원본 y) | 0 | 58 |
| topInfoPosX (제목 중심 오프셋) | 775 | 575 |
| minPosX / secPosX | 100 / 170 | 1200 / 1270 |
| sectionScoreFrame | {30, 855} | {990, 855} |
| charAnimation | `bpmLinkChar.playside` 에 따라 {80,300} 또는 {1080,300} (좌) / 반대 (우) | |

주의: 우측 표시에서 `minPosX/secPosX` 는 이미 절대 x 값(1200,1270)인데도 info.lua 가 `I + minPosX`(= 0 + 1200)로 쓰므로 결과가 같다. 좌측 표시는 `570 + 100 = 670`.

플레이 사이드(스크래치 방향)에 따른 분기 (`base.lua:5-113`):

| 항목 | 7키 1P(좌스크, op 900) | 7키 2P(우스크, op 901) | 5키 1P | 5키 2P |
|---|---|---|---|---|
| NOTES.KEY_POSITION (P 기준 x, 인덱스 1-7 키, 마지막 스크래치) | 114,177,228,291,342,405,456, 3 | 3,66,117,180,231,294,345, 408 | 114,177,228,291,342, 3 | 117,180,231,294,345, 408 |
| 노트 폭 (홀수키 w / 짝수키 b / 스크) | 60 / 48 / 108 | 동일 | 동일 | 동일 |
| KEYFLASH.keyFramePosX | 111 | 0 | 111 | 0 |
| KEYFLASH.scratchFramePosX | 0 | 408 | 0 | 408 |
| KEYFLASH.scratchImagePosX | 10 | 418 | 10 | 418 |
| KEYFLASH.KEY_X | 110,167,224,281,338,395,452 | -1,56,113,170,227,284,341 | 110,167,224,281,338 | 113,170,227,284,341 |
| KEYFLASH.KEY_Y | 137,147,137,147,137,147,137 | 동일 | 137,147,137,147,137 | 동일 |
| KEYFLASH 크기 | 일반 67x73, 스크 92x92 | 동일 | 동일 | 동일 |
| inputkey posX (키빔 x) | 114,177,228,291,342,405,456, 3 | 3,66,117,180,231,294,345, 407 | 114,177,228,291,342, 3 | 117,180,231,294,345, 407 |
| inputkey move_x (TYPE-B 소멸 후 x) | 144,201,258,315,372,429,486, 57 | 33,90,147,204,261,318,375, 461 | 144,201,258,315,372, 57 | 147,204,261,318,375, 461 |
| bomb 중심 x (ModernChic 규격) | 144,201,258,315,372,429,486, 57 | 33,90,147,204,261,318,375, 462 | 144,201,258,315,372, 57 | 147,204,261,318,375, 462 |
| bomb 중심 x (OADX 규격) | 167,222,282,337,397,452,512, 80 | 55,110,169,225,285,340,400, 484 | 167,222,282,337,397, 80 | 169,225,285,340,400, 484 |
| GAUGE.WIDTH | 400 | -400 | 400 | -400 |
| GAUGE.POS_X | 11 | 545 | 11 | 545 |
| GAUGE.NUM_X / AFTERDOT_X / AFTERDOT_NUM_X | 415 / 503 / 513 | 5 / 93 / 103 | 동일 | 동일 |
| GAUGE.TYPE_X | 230 | 144 | 230 | 144 |
| GAUGE.PS (프레임 선택) | "1P" -> `gaugeFrame_1P/gaugeCover_1P` | "2P" | "1P" | "2P" |
| FIVEKEY_COVER_POS_X / ANGLE | - | - | 405 / 180 | 0 / 0 |

공통: `GAUGE.POS_Y = 110`, `GAUGE.PERFECT_COLOR = {147,204,44}`. KEYCOUNT(x,y,r,g,b)는 상세 모드에서만 쓰인다 (`bgaareainfo.lua:835`).

옵션 연동 분기:
- 하단 정보 배치(op 906/907): graphFramePosX = `I + 26`(A) 또는 `I + 553`(B) (scorebar.lua:36-40). 분포/타이밍은 `posX = 808, judgegraphPosX = 832`(A) / `23, 47`(B) (graph.lua:114-121).
- 스코어 막대 방향(op 908/909, scorebar.lua:44-92): 왼쪽 성장이면 `bar_width = -415`, 오른쪽이면 `415`. 라벨 x = `graphFramePosX + 599` 또는 `+0`, 표시 프레임 x = `+0` 또는 `+603`, 기준선 x = `+596` 또는 `+172`, 숫자 x = `+34` 또는 `+634`, barA/AA/AAA/MAX x = `+315/+269/+222/+174`(왼쪽) 또는 `+449/+495/+541/+548`(오른쪽), gra_now/best/target x = `+596`(왼) / `+175`(오), rank x = `+514`(왼) / `+184`(오), diffExscore x = `+454`(왼) / `+179`(오). rank 이미지 접미사 `-l`/`-r`.
- 목표/타이밍 표시 위치(assist.lua:7-67): 좌측 표시 dpx=fpx=520, 우측 표시 dpx=-140, fpx=-113. TYPE-A `diff {165, 253}`, `timing {185, 250}`, TYPE-B `{dpx,160}`/`{fpx,190}`, TYPE-C `{dpx,-20}`/`{fpx,10}`, TYPE-D `{165,123}`/`{185,120}`. 둘 다 TYPE-A 이면 `diff {100,253}`, `timing {250,253}`. 둘 다 TYPE-D 이면 `{100,123}`/`{250,123}`. 기준은 `x = P + 값[1]`, `y = J + 값[2]`, offsets `{LIFT, 사용자43}`.
- BGA 패턴(info.lua:223-247): 1:1 (I+316, 290, 720x720); 16:9 (I+36, 290, 1280x720); 1:1 x2 ((I+43, 335), (I+679, 335), 630x630); 16:9 x4 ((I+50, 297), (I+679, 297), (I+50, 652), (I+679, 652), 624x351); 무효 시 `soundonly` (I+376, 451, 600x350, blend ADDITION).

### 7.2 영역별 사각형 (기본: 좌측 표시 P=51, I=570, G=14, 1P 좌스크, 7키, 기본 옵션)

모든 좌표는 `(x, y, w, h)` 좌하단 기준. 위쪽 객체일수록 z 가 큼.

| z 순 | 영역 | 사각형 | 근거 |
|---|---|---|---|
| 1 | 배경 | (0,0,1920,1080) | background.lua:21 |
| 1b | 배경 밝기 오버레이 | BLACK (0,0,1920,1080) a=0 + offset 40 | background.lua:23 |
| 2 | 정보 영역 제목바 | (570,1026,1350,46) | info.lua:11-15 |
| 2 | 정보 본체 프레임 | (570,209,1350,811) | info.lua:17-21 |
| 2 | 제목/아티스트/장르 텍스트 | 중심 x=1345, y=1033, w=1040, h=25, 가운데 정렬 | info.lua:30-91 |
| 2 | 남은 시간 분/초 | (670,1031,27x20) / (740,1031,27x20) | info.lua:99-123 |
| 2 | 점수 / 최대 콤보 숫자 | (820,222,28x36, 6자리) / (1520,222,28x36, 5자리) | info.lua:131-141 |
| 2 | BPM 최소/현재/최대 | (1077,232,21x27) / (1191,232,28x36) / (1333,232,21x27), 각 4자리 가운데 | info.lua:180-199 |
| 2 | 이펙터 | 프레임 (1092,279,308x37), 내용 (1133,283,227x28) | info.lua:334-364 |
| 2 | 램프 장식 | (608,180,152x108), (1734,180,152x108) (`lamp_rhythm`, `lamp_maxgauge`) | info.lua:370-393 |
| 2 | BGA 16:9 | (606,290,1280x720). 먼저 a=100 FIT_OUTER_TRIMMED 로 한 번, 그 위에 같은 사각형을 config 의 bgaExpand 방식(기본 STRETCH)으로 한 번 | info.lua:249-272 |
| 2 | BGA 밝기 오버레이 | BLACK (606,290,1280x720) a=0 + offset 42 (상세 모드 OFF 일 때) | info.lua:313-318 |
| 3 | 진행바 프레임 | (23,247,16x807) | progress.lua:19-21 |
| 3 | 진행바 마커 | x=19, y 247 -> 1020, 24x37 (slider range 773) | progress.lua:23-52 |
| 4 | 키 베드 | (162,110,408x114) | keyflash.lua:17-25 |
| 4 | 스크래치 베드/프레임 | (51,110,111x114) | keyflash.lua:27-35 |
| 4 | 키플래시 | x = 51 + KEY_X[i], y = 137/147, 67x73, ADDITION, timer KEYON | keyflash.lua:63-69 |
| 4 | 스크래치 이미지 | (61,122,90x90), filter ON, offset SCRATCHANGLE_1P | keyflash.lua:71-75 |
| 4 | 키 커버 상/하 | 상 (162,167,408x57) 하 (162,110,408x57), 1000ms 후 1300ms 까지 높이 0 (TRIMMED) | keyflash.lua:83-96 |
| 5 | 게이지 프레임 | (14,110,556x69) | gauge.lua:115-119 |
| 5 | 게이지 바 | (25,119,400x35), 50칸, 칸 폭 8 | gauge.lua:122-133 |
| 5 | 게이지 로드 마스크 | BLACK (25,119,400x35) -> 2500ms 동안 x=425, w=0 으로 줄임 | gauge.lua:136-141 |
| 5 | 게이지 숫자 | 정수 (429,136,28x36, 3자리), 점 (517,136,10x36), 소수 1자리 (527,136,21x27), 모드 라벨 (244,160,190x15) | gauge.lua:145-223 |
| 6 | 레인 | (51,224,519x857) | lane.lua:14 |
| 6 | 레인 안쪽 | (54,227,513x1 -> 854) READY 타이머 1000ms 로 확장 | lane.lua:16-22 |
| 6 | 레인 밝기 오버레이 | BLACK (54,227,513x854) a=0 + offset 45 | lane.lua:35 |
| 6 | 판정선 | 중심에서 펼침: 500ms (x=54+256.5, w=0) -> 1000ms (x=54, w=513), (y=227, h=judgeHeight(기본 12)), offset LIFT | lane.lua:44-51 |
| 6 | 글로 램프 | (54, 227+judgeHeight, 513x glowHeight(기본 48)), RHYTHM 타이머로 a 255 -> 0 (1000ms), 초기 글로 a 0 -> 120 | lane.lua:52-71 |
| 7 | 키빔 | x = 51 + posX, y = 227, w = 키 폭, h = 564 x 옵션%(기본 50% = 282), offset LIFT | inputkey.lua:60-128 |
| 8 | 노트 | 레인별 region (x = 51 + KEY_POSITION, y = 215, w = 폭, h = 841), offset NOTES_1P(30) | notes.lua:36-77,243 |
| 9 | 히든 커버 | (54,-626,513x853) 이미지 1x1 검정, disapearLine 227 | cover.lua:16 |
| 9 | 리프트 커버 | (54,-626,513x853), disapearLine 227, `isLiftDisplay` 일 때 | cover.lua:18 |
| 9 | 레인커버 | (54, 1080 -> 슬라이더로 아래로 최대 853), 등장 연출 500-1000ms 에 y 1933 -> 1080 | cover.lua:20-23 |
| 9 | 가변 SUD | 동일 크기, a=100, op CONSTANT+LANECOVER1_ON | cover.lua:30-33 |
| 9 | 레인커버 숫자 | 흰색 (141,1090,27x20), 녹색 (311,1090,27x20), 범위 최소 (271,1120) / 최대 (391,1120) (각 21x18), `from` 글자 (371,1117) | cover.lua:89-115 |
| 9 | 리프트 숫자 | (141, 197, 27x20), (311, 197, 27x20) (상세 OFF) | cover.lua:116-142 |
| 9 | 리프트 표시 토글 클릭 영역 | (51,224,519x857) 투명 | cover.lua:150-158 |
| 9b | 5키 커버 | x = P+405(좌스크) 또는 P+0, y 1080 -> 179, 114x901, a=240 | play5_hw.lua:154-164 |
| 11 | 폭발 | x = P + bombPosX - w/2, y = 227 - adjustPosY - h/2, w x h = 400x300 (ModernChic) 또는 376x300 (OADX, adjustPosY 18), offset LIFT, ADDITION | bomb.lua:121-194 |
| 12 | 레벨 프레임 | (14,8,175x91) | info2.lua:51-55 |
| 12 | 카운트 프레임 | (199,4,371x103) | info2.lua:56-60 |
| 12 | 판정 카운터 | x = 274 / 374 / 474 (총합/early/late), y = 85, 65, 45, 25, 5, 22x20, 4자리 | info2.lua:61-83 |
| 12 | 난이도 라벨 | (24,64,160x20) | info2.lua:84-95 |
| 12 | 레벨 숫자 | (109,27,27x20) | info2.lua:96-101 |
| 13 | 판정 문자/콤보 | 판정 문자 (181, 380, 227x84) offset {LIFT, JUDGE_1P}, 콤보는 문자 기준 x +237, y 0, 55x84 (절 10.3) | judge.lua:56-107 |
| 14 | 노트 분포 프레임 | (1378,62,519x142) | graph.lua:124-129 |
| 14 | 판정/BPM 그래프 | (1402,71,472x107) | graph.lua:141-150 |
| 14 | 타이밍 프레임 | (1378,4,519x54) | graph.lua:172-178 |
| 14 | 타이밍 검은 바탕 | BLACK (1402,8,472x30) | graph.lua:182-186 |
| 14 | 판정 레벨 라벨 | 가운데 (1533,8,210x30) | graph.lua:195-200 |
| 14 | 타이밍 분포 | `timing` (1402,8,472x30), `hrv` (1874,8,-472x30) (좌우 반전, timer PLAY) | graph.lua:201-211 |
| 14 | 판정 타이밍 숫자 | (1402,37,27x20), (1752,37,27x20), 4자리 가운데 | graph.lua:264-274 |
| 14 | 타이밍 조정 클릭 영역 | (1402,4,519x54) 투명 | graph.lua:276-281 |
| 15 | 풀콤보 | `fc` (51,227,519x857) ADDITION, `wd_full`, `wd_combo` (P 기준 슬라이드 인/아웃) | fullcombo.lua:19-35 |
| 16 | 스코어 그래프 프레임 | (596,4,771x200) | scorebar.lua:156-160 |
| 16 | 그래프 배경 | (768,5,428x183) + 밝기 오버레이 offset 41 | scorebar.lua:163-171 |
| 16 | 라벨 | nowscore (1195,133,172x51), bestscore (1195,80,...), target (1195,27,...) | scorebar.lua:174-190 |
| 16 | 막대(왼 성장) | now (1192,133,-415x51), best (1192,80), target (1192,27), 현재 랭크 (1110,145,75x22) | scorebar.lua:278-331 |
| 16 | 기준선/등급 선 | 흰선 (1192,4,3x185), A/AA/AAA (911,865,818 -> x), MAX (770) | scorebar.lua:206-258 |
| 19 | 로딩 창 | (121,420,380x311) 등 (절 10.5) | prepare.lua:80-169 |
| 20 | 폐점 연출 | 상/하 프레임 슬라이드, 메시지 창 (0,381,1920,317), 화살표, `STAGE FAILED` 글자, 빔 20개 | close.lua:28-113 |
| 21 | 종료 페이드 | BLACK (0,0,1920,1080), a 0 -> 255 (500ms) | play7_hw.lua:298-303 |

이 표는 y 축이 위로 증가함을 전제로 작성했다. 레인 가운데는 x = 54 + 256.5 = 310.5 이고 레인 바닥(판정선 하단) y = 227, 노트는 하단이 y = 215 에서 시작해 높이 36 이므로 노트 중심이 y = 233(판정선 227-239의 중앙)이 되도록 맞춰져 있다 (노트 region.y = J - 12, LaneRenderer.java:521-524 에서 `dsty = y`, 높이 = 이미지 높이 36).

### 7.3 우측 표시 / 우스크 변형 요약

- 우측 표시(op 903): P=1350, I=0, G=1350, 진행바 x=1881. 정보 영역이 화면 왼쪽(0..1350), 레인이 오른쪽(1350..1869)으로 바뀐다. `infoTitleFrame` 의 원본 y 가 58 인 제목바(오른쪽에 노치가 있는 버전)를 쓴다 (info.png 두 번째 행).
- 2P(우스크, op 901): 스크래치/키 순서가 뒤집힌 `KEY_POSITION`, `KEYFLASH`, `GAUGE`(w = -400 로 게이지가 오른쪽에서 왼쪽으로 차오름), 프레임은 `gaugeFrame_2P/gaugeCover_2P`. 게이지 칸의 음수 폭은 SkinGauge 가 `region.x + region.width*(i-1)/parts` 로 각 칸을 그릴 때 음수 폭으로 좌우 반전 렌더한다 (SkinGauge.java:190-196). R-BMS 렌더러는 음수 w/h 를 반전 렌더로 지원해야 한다 (timing `hrv`, 그래프 `-415`, `-468`, 캐릭터 `-200`, 키빔 등에서도 사용).
- 5키: 레인 폭은 7키와 같은 513이다. 7키 시트를 그대로 쓰므로 5키 키 프레임(408 폭 7개 키 베드)의 오른쪽 2키분(좌스크일 때 x = P+405..P+519)이 `5keysFrame` 이미지(src1 (1760,0,114,901))로 가려진다. 이 이미지는 y 1080 -> `J - 48 = 179` 로 1000-1500ms 에 내려온다.

### 7.4 해결하지 못한 관찰 (실화면 검증 필요)

`gauge.lua:115-119` 의 게이지 프레임 (14,110,556x69)과 `keyflash.lua:17-35` 의 키 베드 (162,110,408x114)/스크래치 (51,110,111x114)가 좌표상 x 51..570, y 110..179 에서 겹친다. destination 순서도 keyflash(4) -> gauge(5) 라 게이지가 키 아래쪽 약 69px 을 덮는 계산이 된다. 5키 커버가 `y = J - 48 = 179`(게이지 프레임 상단)에서 시작하는 점, 키 아트의 삼각 표시가 y 약 123-143 에 있다는 점이 서로 모순이어서 의도인지 확신하지 못했다. beatoraja 에서 같은 스킨의 실제 화면 캡처와 비교하기 전에는 이 겹침의 시각 결과를 단정하지 않는다 (구현은 좌표 그대로 따르면 beatoraja 와 동일해진다).

## 8. 자산

### 8.1 SP 플레이에서 참조되는 에셋 (기본 옵션 선택 기준)

디스크 크기는 파일 크기, 디코드 크기는 RGBA 8bit 가정 (w x h x 4).

| 용도 | 경로 | 크기 | 디코드 | 비고 |
|---|---|---|---|---|
| 시스템 시트 | Play/parts/sp_hw/system.png (1920x1400) | 142KB | 10.7MB | 필수 (source 1) |
| 정보 시트 | Play/parts/sp_hw/info.png (1350x1350) | 28KB | 7.3MB | 필수 (2) |
| 스코어 시트 | Play/parts/sp_hw/score.png (810x390) | 30KB | 1.3MB | 필수 (22) |
| 그래프 배경 | Play/parts/sp_hw/graphbg/#default.png (428x183) | 1.5KB | | 선택형 (25) |
| 리프트 | Play/parts/sp_hw/lift/#default.png (513x853) | 52KB | 1.7MB | 선택형 (18) |
| 가변 SUD 시트 | Play/parts/sp_hw/adjusted.png (541x853) | 4.4KB | | 필수 ("adjusted") |
| 배경 | Play/parts/common/bg/#default.png (1920x1080) | 28KB | 8.3MB | 선택형 (3). 다른 5색은 각 1.2MB |
| 판정 문자 | common/judge/#default.png (777x588) | 92KB | | 선택형 (4) |
| 판정선 | common/judgeline/#default.png (513x12) | 1KB | | 선택형 (5) |
| 노트 | common/notes/#default.png (324x360) | 73KB | | 선택형 (6) |
| 지뢰 | common/mine/#default.png (324x36) | 2.9KB | | 고정 (26) |
| 글로 | common/glow/#default.png (431x48) | 3KB | | 선택형 (9) |
| 진행 마커 | common/progress/#default.png (24x37) | 1.7KB | | 선택형 (10) |
| 폭발 | common/bomb/diamond SCUROed..png (6400x1200) | 1.45MB | 30.7MB | 선택형 (11), 가장 무거운 쪽 중 하나 |
| 풀콤보 | common/fullcombo/#default.png (5190x2571) | 11MB | 53.4MB | 선택형 (13), 가장 무거움 |
| 키빔 | common/keybeam/#default.png (238x564) | 9KB | | 선택형 (14) |
| 키 이미지 | common/key/harf.png (519x342) | 23KB | | 선택형 (15) 기본 `harf` |
| 키플래시 | common/keyflash/#default.png (67x73) | 3.7KB | | 선택형 (16) |
| 레인커버 | common/lanecover/#default.png (513x853) | 373KB | 1.7MB | 선택형 (17) |
| 램프 | common/lamp/#default.png (152x216) | 4.8KB | | 선택형 (24) |
| 게이지 | common/gauge/#default.png (80x70) | 1.2KB | | 선택형 (29) |
| 스크래치 | common/scratch/#default.png (92x92) | 2KB | | 선택형 (30) |
| 폐점 | common/close/close.png (2200x2100) | 840KB | 18.5MB | 고정 (21) |
| 헥 폭발 | common/hcn/hcn.png (1600x1200) | 293KB | 7.7MB | source 선언만 (`hcnBomb`), `CONFIG.play.hcnBomb=false` 라 destination 이 없어 로드되지 않음(beatoraja 는 destination 이 있는 객체만 텍스처를 로드) |
| 폰트 TTF | Play/font/ttf/mgenplus-1c-black.ttf, mgenplus-1c-medium.ttf | 각 5.2MB | | 기본 (`isOutlineFont`) |
| 폰트 비트맵 | Play/font/fnt/{title,info,top}.fnt + 페이지 png 36장 | 43MB | | `isBitmapFont`(op 905) 때만 |
| BGA 동영상 | common/BGA/movie/#default.mp4 | 19.9MB | | BGA 없는 곡 + 動画 일 때만 (id 23). 그 외 cyber 45MB, NOSTALGIC 48MB, travel 23.7MB 는 선택형 |
| BGA 이미지 | common/BGA/image/#default.png (1280x720) | 742KB | | BGA 없는 곡 + 画像 일 때만 (id 27) |
| 음성/효과음 | Root/sounds/{fullcombo,section*.ogg}, Root/sounds/vo/{zundamon,tsumugi}/** | 합 1.5MB | | 기본 설정(`CONFIG.voice.play.sw=false`, `fcEffect=false`)에서는 호출되지 않음 |

mp4 총 8개 중 Play 에 4개(131MB), Select 2개, Decide 2개. SP 플레이는 BGA 없는 곡의 대체 영상 하나만 쓴다. ogg 73개 중 Root/sounds 가 58개(공통 효과음 10 + 캐릭터 음성 2명 x 24), 나머지 15개는 `Sound/`(13개) 등 다른 폴더다. SP 플레이는 Root/sounds 만, 그것도 음성/효과음 설정이 켜진 경우에만 참조한다.

### 8.2 조건부로만 필요한 것

- 공격 모션 ON: `common/attack/attack.png` (2000x1200, 342KB), `common/attack/hud.png` (3600x772, 215KB).
- 상세 모드 ON: `sp_hw/bgainfo.png` (1024x1024, 44KB, source 31).
- OADX 폭발 선택 시: `common/oadx_bomb/*.png` (현재 `dummy.png` 하나, 포맷 설명은 `仕様.txt`: 181x192 셀 16열 x 4행 = 2896x768).
- 5키 전용 부가 이미지는 system.png 안(1760..1874, 0..901).
- 캐릭터 연동 ON: `Root/image/{zundamon,zundamon2,zundamon3,tsumugi,yuki}.png`.

### 8.3 참조되지 않는 파일

- `Play/parts/sp_hw/adjusted_bk.png` (1082x853): 어떤 Lua 도 참조하지 않는다.
- `Play/parts/sp_hw/bga.png` (1500x1600): source id 20 으로 선언되지만 `src = 20` 을 쓰는 객체가 없다 (`base.lua:206`).
- `Play/parts/common/POMYU Chara/Off/dummy.chp`: SP 플레이 스킨에서 `pmchara` 를 쓰지 않는다.
- `Play/parts/dp_hw/*`, `Play/parts/dummy/-`: SP 와 무관.
- `Play/lua/dp/*`: SP 와 무관.
- `.txt` 안내 파일들(`仕様.txt`, `1920x1080のpng.txt` 등).

### 8.4 와일드카드 파일 슬롯

20개 슬롯 전부 `*.png`(BGA 동영상만 `*.mp4`). 선택값은 파일명(확장자 제외)으로 저장되고 `skin_config.get_path` 가 그 파일로 해석한다. 이름에 공백, 마침표, 일본어가 있다 (`DR simple SCUROed..png`, `Splash[Re] SCUROed.png`, `1920x1080のpng.txt`).

### 8.5 `io/Play/sp` (스킨 폴더 안 상태 파일)

`lanecover/{pathList,excludeList,temp}.txt`, `log/*.txt`(8줄, 타건 누적), `lnlog/*.txt`, `nentyaku.txt`(2줄). 모두 로테이션/상세 모드가 쓰는 상태 파일이며 기본 설정에서는 읽기만 (`Root.customnumber` 의 `countFileRecords`) 발생한다.

## 9. 단순화 후보

### 9.1 빼도 화면 핵심이 유지되는 것

| 후보 | 근거 | 절약 |
|---|---|---|
| 상세 모드 전체 (`detailinfo/*`, bgaareainfo 964줄, 상태 누적 함수 31개, 파일 쓰기) | 옵션 기본 OFF. Lua 부수효과/성능 의존, `performance` 값 함수가 매 프레임 상태를 갱신 | Lua 약 1,400줄, `bgainfo.png` |
| 공격 모션 (`attack.lua`, attack/hud png, cascade timer 160개) | 기본 OFF | 540KB, 클로저 160개 |
| 구역 점수 + 음성/효과음 훅 (`CONFIG.play.sectionScore`, `CUSTOM.SOUND.*`, `fcSound`, `achievementVoice`) | 기본 OFF, 오디오 부수효과 | Root/sounds 1.5MB |
| 캐릭터 연동 (`charAnimation`, `Root/image/*`) | 기본 OFF | |
| 점수 플랩 (`roulette_number_*`) | 기본 OFF | |
| 레인커버 로테이션 (io 로 목록 관리) | 기본 OFF, 파일 쓰기 필요 | |
| `History/information.txt` 기록 (`CONFIG.infoOutput`) | 파일 쓰기 | |
| `luajava`/`io`/`os` 의존 (`Root.customfunction`, `customnumber` 의 파일 카운트, `customtime`) | 기본 경로에서는 `existFile` 안전 가드만 사용. 순수 상수 스킨으로 바꾸면 불필요 | R-BMS 런타임 요구사항 대폭 감소 |
| `CONFIG` 파일 (`config.lua`) | 기본값 상수로 고정 가능 | |
| BGA 패턴 4종 중 3종, 汎用BGA 동영상 | 기본 16:9. 동영상은 20MB, 디코더 필요 | 20MB 이상 |
| 옵션 군: 폭발 규격(OADX), 판정 타이밍 폭발, 키빔 높이 10단계, 키빔 소멸 시간/패턴, 타이밍 그래프 4색/3배율/3패턴, 목표/타이밍 표시 위치 TYPE-A..D x2, 노트 분포 3패턴, 비트맵 폰트 | 선택지만 늘린다. 기본값 하나로 고정하면 분기 코드와 에셋이 사라짐 | |
| 에셋 변종 (배경 5색, 노트 9종, 판정 3종, 폭발 6종, 풀콤보 등) | 기본 1종만 남김 | bomb 8.1MB, bg 6.9MB, fullcombo 11MB |
| 풀콤보 시트 (11MB, 53MB 디코드) | 연출 하나 때문에 가장 큼. 해상도 축소 또는 프레임 수 축소 | |
| 폐점 시트 (840KB, 18.5MB 디코드) | 연출 유지 시 해상도 축소 가능 | |
| 비트맵 폰트 (43MB) | TTF 로 충분 | 43MB |

### 9.2 빼면 안 되는 것 (화면 핵심 또는 엔진 계약)

- 레인 프레임, 판정선, 글로, 키 베드/키플래시/키빔/스크래치 이미지, 노트 7+1 레인 정의, LN/HCN/지뢰/히든/프로세스 이미지(note 객체의 11종 배열), 판정 문자 + 콤보 숫자(`judge` 객체의 shift 포함), 게이지(36 노드, 50칸, 숫자 + 소수점 + 모드 라벨), 폭발(일반/LN), 레인커버 슬라이더 + 리프트 + 히든 + 가변 SUD 3종(CONSTANT 지원 시), 진행바, 정보 영역의 제목/아티스트/장르, BPM, 남은 시간, 점수, 최대 콤보, BGA 프레임과 `bga` 객체, 스코어 그래프(now/best/target 막대와 순위), 판정 카운터(PG..PR early/late), 로딩 UI(`prepare`), 폐점 연출, 종료 페이드.
- 레인 위치(좌/우)와 스크래치 방향(좌/우) 옵션과 이를 지탱하는 `BASE` 계산 (플레이어 선호 핵심).
- 5키 전용 처리(`5keysFrame`).
- 노트 분포 그래프와 BPM 그래프(judgegraph/bpmgraph), 타이밍 분포(`timing`/`hrv`) 중 하나는 기본 화면 일부이므로 유지 후보. 어느 정도까지 남길지는 사용자 결정 사항이다 (절 12 질문).
- `stretch` TRIMMED 3종, 음수 w/h 반전, `offsets` 배열(LIFT 포함) 같은 렌더러 기능은 단순화 후에도 레인커버/판정/게이지 구현에 필요하다.

### 9.3 단순화 시 유의

- 기본 옵션 구성이 만드는 테이블은 R-BMS 가 요구하는 지원 기능의 합집합이다. 기능을 먼저 가려내고 에셋을 줄여야 한다.
- 키플래시/스크래치 프레임 색(`keyflash.lua:12-14`)은 로드마다 `math.random(50,155)` 로 바뀐다. 시각 일치 검증 시 시드를 고정하거나 이 영역을 마스킹해야 한다.

## 10. 객체 필드 원문 (note / gauge / judge / bga / hidden / lift / 그래프류)

### 10.1 note (notes.lua:201-244)

```
note = {
  id = "notes",
  note       = 7키 {"note-w","note-b","note-w","note-b","note-w","note-b","note-w","note-s"}
               5키 {"note-w","note-b","note-w","note-b","note-w","note-s"},
  lnend      = lne-* 동형,  lnstart = lns-* 동형,  lnbody = lnb-* 동형,  lnactive = lna-* 동형,
  hcnend     = hcne-*,  hcnstart = hcns-*,  hcnbody = hcnb-*,  hcnactive = hcna-*,
  hcndamage  = hcnd-*,  hcnreactive = hcnr-*,  mine = mine-*,
  hidden = {}, processed = {}, size = {},
  dst = [ {x=P+KEY_POSITION[k], y=J-12 (=215), w=폭(60|48|108), h=LANE_LENGTH-12 (=841)} x 레인수 ],
  group = { {id="section-line", offset=LIFT(3), dst={{x=P+3, y=J, w=513, h=3, a=barlineBright}}} },
  time  = { {id="section-line", offset=LIFT, dst={{x=P+3, y=J, w=513, h=15, r=100,g=100,b=255}}} },
  bpm   = { {id="section-line", offset=LIFT, dst={{x=P+3, y=J, w=513, h=15, r=100,g=255,b=100}}} },
  stop  = { {id="section-line", offset=LIFT, dst={{x=P+3, y=J, w=513, h=15, r=255,g=100,b=100}}} }
}
destination: { {id="notes", offset=NOTES_1P(30)} }
```

- `barlineBright` = `offsetBarlineBright(alpha)`: alpha == 0 -> 255, 0 < alpha <= 255 -> 255 - alpha, 그 외 255 (common.lua:8-23).
- `lnbodyActive/hcnbodyActive/hcnbodyMiss/hcnbodyReactive/dst2/expansionrate` 는 지정하지 않는다. beatoraja 는 `lnbodyActive` 가 비어 있으면 슬롯을 `[2]=lnbody, [3]=lnactive, [6]=hcnbody, [7]=hcnactive, [8]=hcndamage, [9]=hcnreactive` 로 배치한다 (JsonPlaySkinObjectLoader.java:46-69). 슬롯 인덱스: 0 lnend, 1 lnstart, 2 lnbody, 3 lnactive, 4 hcnend, 5 hcnstart, 6 hcnbody, 7 hcnactive, 8 hcndamage, 9 hcnreactive.
- 이미지 높이: 일반/ln 계열 36, lna 36, hcna 18, lnb 72(divy 2 시 프레임당 36), 노트 하나의 높이는 `size` 가 비어 있으므로 노트 이미지(`note-*`)의 높이 36 이 된다 (JsonPlaySkinObjectLoader.java:86-93).
- 노트 그리기: 일반 노트의 하단은 레인 region.y(=215, 리프트 시 + lift)이고 높이 36, LN 본체는 시작 노트와 끝 노트 사이를 늘려 그린다 (LaneRenderer.java:516-584).
- 레인 수는 `dst` 길이로 결정된다 (7키 8, 5키 6). 모드의 레인 수와 다르면 엔진이 오동작할 수 있으므로 일치해야 한다.

### 10.2 gauge (gauge.lua:89-110)

```
gauge = {
  id = 2001,           -- destination 의 id "2001" 과 문자열 일치
  parts = 50,          -- CONFIG.play.smallGauge 가 true 이면 100
  nodes = {            -- 36개 = 6 게이지종 x 6 슬롯
    -- 슬롯 순서: [0]=켜짐/클리어 영역, [1]=켜짐/미달 영역, [2]=꺼짐/클리어, [3]=꺼짐/미달, [4]=선두/클리어, [5]=선두/미달
    "gauge-r1","gauge-p1","gauge-r2","gauge-p2","gauge-r3","gauge-p3",   -- 0 ASSIST_EASY
    "gauge-r1","gauge-g1","gauge-r2","gauge-g2","gauge-r3","gauge-g3",   -- 1 EASY
    "gauge-r1","gauge-b1","gauge-r2","gauge-b2","gauge-r3","gauge-b3",   -- 2 NORMAL
    "gauge-r1","gauge-p1","gauge-r2","gauge-p2","gauge-r3","gauge-p3",   -- 3 HARD
    "gauge-y1","gauge-p1","gauge-y2","gauge-p2","gauge-y3","gauge-p3",   -- 4 EXHARD
    "gauge-h1","gauge-p1","gauge-h2","gauge-p2","gauge-h3","gauge-p3"    -- 5 HAZARD
  }
}
```

- type/range/cycle 는 미지정이라 beatoraja 기본값 `type=0(ANIMATION_RANDOM), range=3, cycle=33, starttime=0, endtime=500` (JsonSkin.java:369-378).
- 그리기 규칙 (SkinGauge.java:182-196): `exgauge = (gaugeType >= CLASS ? gaugeType - 3 : gaugeType) * 6`(grade 계열 6,7,8 은 3,4,5 의 노드 재사용), `notes = value > 0 ? max(1, int(value*parts/max)) : 0`, 칸 i 의 이미지는 `exgauge + (notes == i ? 4 : (notes - animation > i ? 0 : 2)) + (i*max/parts < border ? 1 : 0)`. 칸 폭은 `region.width / parts`.
- `gauge-X1/X3` 는 같은 이미지(밝음)이고 `X2` 가 어두운 칸이다 (gauge.lua:21-60).

### 10.3 judge (judge.lua:51-360)

`parts.judge` 5개 정의. `id`: `def`, `laneCoverRest_1`, `laneCoverRest_2`, `constantRest_1`, `constantRest_2`. 모두 `index = 0`, `shift = true`. 기본 설정 destination 은 `{id = "def"}` 하나.

`def.images` (6개, 순서 = PG, GR, GD, BD, PR, MS):
```
{id="judgef-pg"|"-gr"|"-gd"|"-bd"|"-pr"|"-ms", loop=-1, timer=JUDGE_1P(46),
 offsets={LIFT(3), JUDGE_1P(32)},
 dst={{time=0, x=P+130, y=J+153 (=380), w=227, h=84}, {time=500}}}
```
`def.numbers` (6개 동일 구조):
```
{id="judgen-pg"|..., loop=-1, offset=JUDGE_1P(32), timer=JUDGE_1P(46),
 dst={{time=0, x=237, y=0, w=55, h=84}, {time=500}}}
```
- 이미지 정의: `judgef-pg` src4 (0,0,227,252) divy 3 cycle 120; `judgef-gr` (0,252,227,168) divy 2 cycle 80; `judgef-gd` (0,420,...); `judgef-bd` (227,420,...); `judgef-pr` (454,420,...); `judgef-ms` = `judgef-pr` 와 같은 영역. `CONFIG.play.judgeAnimation`(기본 true)이 false 이면 cycle 제거.
- 콤보 숫자 정의: `judgen-pg` src4 (227,0,550,252) divx 10 divy 3 digit 6 cycle 120, `judgen-gr/gd/bd/pr/ms` (227,252,550,168) divx 10 divy 2 digit 6 cycle 80.
- beatoraja 동작 (JsonPlaySkinObjectLoader.java:244-276, SkinJudge.java:97-131): 콤보 숫자는 align 2(가운데), `relative` 로 로드되고 로드 시 `dst.x -= w * digit / 2` (= 55*6/2 = 165) 가 적용된다. prepare 에서 콤보 숫자 객체는 판정 문자의 region 좌표(x,y)를 기준 오프셋으로 받아 그려지고, 이어서 `judgeImage.region.x += shift ? -numberLength/2 : 0` 으로 판정 문자를 콤보 길이의 절반만큼 왼쪽으로 민다. `numberLength = 55 * (표시 자릿수)`. 따라서 판정 문자 (227폭)와 콤보 자릿수는 10px 간격으로 항상 붙어 있고 전체가 중심 정렬된다. 콤보 숫자는 `judgenow < 3`(PG/GR/GD)일 때만 그려진다. `judge[6]` 이 없으면 게이지 MAX 일 때 PG 를 그대로 쓴다.
- 판정 이미지/숫자 프레임 순환에는 timer 가 없으므로 `cycle` 이 스킨 시간(전역)을 기준으로 돈다. judge 의 destination timer 는 표시 시작/종료(500ms 후 loop -1 로 사라짐)만 결정한다.

`laneCoverRest_*`/`constantRest_*` 는 `valiableJUDGE.sw` 가 true 일 때만 destination 이 참조한다 (judge.lua:350-358).

### 10.4 bomb (bomb.lua)

- 상수: `bombCycle = 251`, `lnbombCycle = 160`. ModernChic 규격 셀 400x300, 16열 x 4행: 1행 일반, 2행 LN 흰건반/SLOW, 3행 LN 검은건반/FAST, 4행 LN 스크래치 (`bomb/仕様.txt`).
- image: `bomb`/`lnbomb`(프리로드용 `w=-1,h=-1`), `bomb-{1..7,s}` (0,0,6400,300 divx 16 cycle 251 timer BOMB), `lnbomb-*` (0, 행오프셋, 3200,300 divx 8 cycle 160 timer HOLD), `slowbomb-*` (0,300,...), `fastbomb-*` (0,600,...), `hcnbomb-*` src "hcnBomb" (0,0,1600,1200) divx 4 divy 4 cycle 640. 7키 행오프셋 `{300,600,300,600,300,600,300,900}`(판정 타이밍 폭발 OFF), ON 이면 전부 0. OADX 셀은 `w=2896,h=192`, LN 폭 1448, 행오프셋 `{192,384,192,384,192,384,192,576}`.
- destination: `{id="bomb-N", offset=LIFT, loop=-1, filter=OFF, timer=BOMB_N, blend=ADDITION, dst={{time=0, x=P+bombPosX-w/2, y=J-adjustPosY-h/2, w, h}, {time=250}}}`. 판정 타이밍 폭발 ON 이면 EARLY/LATE 에 따라 `fastbomb-N`/`slowbomb-N`, 그 외 `bomb-N` (op `-1242`, `-1243`). LN 폭발은 `lnbomb-N` timer HOLD_N (loop 없음, 같은 좌표, 끝 시간 159).
- `bombSize` 오프셋 w 가 1..100 이면 w, h, adjustPosY 에 `w/100` 배율 (common.lua:26-38).

### 10.5 bga, hiddenCover, liftCover, 로딩 UI 좌표

```
bga = {id = "bga"}                       -- destination 에서 객체 하나가 destination 마다 별개 SkinBGA 로 생성된다
hiddenCover = { {id="hidden_cover", src=1, x=0,y=0,w=1,h=1, disapearLine=227} }   -- isDisapearLineLinkLift 기본 true
liftCover   = { {id="lift_cover",   src=18, x=0,y=0,w=513,h=853, disapearLine=227} } -- 기본 false
destination:
  {id="hidden_cover", dst={{x=P+3, y=-626, w=513, h=853}}}
  {id="lift_cover", draw=function() return isLiftDisplay end, dst={{x=P+3, y=-626, w=513, h=853}}}
```
`SkinHidden` 은 `OFFSET_LIFT`(+히든은 `OFFSET_HIDDEN_COVER`)를 객체 오프셋에 자동으로 더하고, region 상단이 `disapearLine` 위로 올라온 부분만 clip 으로 그린다 (JsonPlaySkinObjectLoader.java:183-214, SkinHidden.java:44-104). `isDisapearLineLinkLift` 가 true 면 소실선이 `227 + lift` 로 따라 움직인다.

BGA 객체의 stretch: destination 에 `stretch` 가 없으면 `Config.bgaExpand` 에 따른 방식(FULL=STRETCH, KEEP_ASPECT_RATIO=FIT_INNER, OFF=NO_EXPANDING)이 쓰인다 (SkinBGA.java:21-27). 16:9 패턴은 먼저 FIT_OUTER_TRIMMED + a=100, 그 위에 설정 기반 방식으로 한 번 더 그린다.

로딩 UI (prepare.lua:80-169): `loadingWindowPosX = P + 70`(=121), `loadingWindowPosY = 420`. `op = {NOW_LOADING, ±LANECOVER1_CHANGING}` 와 `alpha = {255, 0}` 두 벌을 만들어 레인커버를 조작 중일 때 로딩 창을 투명하게 만든다. 로딩 창 안의 `notesgraph`(x+38, y+161, 304x100), `bpmgraph` 같은 크기, `loading_bar`(x+40, y+14, 300x26), `loading_par`(x+130, y+16), `per`(x+225, y+16) 등.

### 10.6 judgegraph, bpmgraph, timingvisualizer, hiterrorvisualizer (graph.lua:6-110, 113-160, prepare.lua:88)

```
judgegraph {id="judgegraph", type = (상세 ON ? NOTES(0) : 분포패턴 A:JUDGE(1) / B:FASTSLOW(2) / C:NOTES(0)), backTexOff = 0}
judgegraph {id="notesgraph", type = NOTES(0)}                -- prepare.lua, 나머지 기본값 (delay 500, orderReverse 0, noGap 0)
bpmgraph   {id="bpmgraph"}                                    -- 모든 필드 기본값
timingvisualizer {id="timing", width=300, judgeWidthMillis=225|150|75, lineWidth=1,
   lineColor="00FF00FF" (red "E286A7FF", green "86E088FF", blue "89DDDCFF"), centerColor="FFFFFFFF",
   PGColor="00008855", GRColor="00880055", GDColor="88880055", BDColor="88000055", PRColor="00000055",
   transparent=0, drawDecay=0}
   -- 색 변형 (PG,GR,GD,BD,PR): red {44000055,99000055,44000055,99000055,00000055}
   --                          green {00440055,00880055,00440055,00880055,00000055}
   --                          blue {22224455,22229955,22224455,22229955,00000055}
hiterrorvisualizer {id="hrv", width=300, judgeWidthMillis=225|150|75, lineWidth=1, colorMode=1, hiterrorMode=0,
   emaMode=1|2|3 (normal/triangle/emphasis), lineColor="99CCFF80", centerColor="FFFFFFFF",
   PGColor="99CCFF80", GRColor="F2CB3080", GDColor="14CC8f80", BDColor="FF1AB380", PRColor="CC292980",
   emaColor="FF0000FF", windowLength=30, transparent=1, drawDecay=1}  -- alpha 기본 0.1
```
`timing`, `hrv` destination: 상세 위치 A(BGA 쪽)는 `(I+832, 8, 472x30)` 와 `(I+832+472, 8, -472x30)`, 위치 B(플레이 쪽)는 `(P+ (518-472)/2, 310, 472x30)` 와 `(P+23+472, 310, -472x30)`, offsets `{LIFT, 사용자49}`, timer PLAY.

색 문자열은 8자리 RGBA 16진 (예: `"00008855"` = rgb 000088, a 0x55). 이 시각화 객체의 그리기 로직은 읽지 못했다 (미확인).

### 10.7 5키 / 7키 차이 정리

1. 헤더 type 1/0, `PROPERTY.load(true/false)`: 5키는 property 에 `5鍵用レーンカバー`(op 984/985) 추가, category 13 에 32 번 추가.
2. `BASE.createBasePositionSP(5)` 는 `KEY_POSITION`(6개), `KEYFLASH.flashTimer/KEY_X/KEY_Y`(5개), `KEYCOUNT`(6개), `FIVEKEY_COVER_POS_X/ANGLE` 를 추가로 만든다.
3. `keyflash.load/inputkey.load/notes.load/bomb.load` 인자 5. 배열 크기와 `note-w/b/w/b/w/s` 구성이 바뀐다.
4. 5키 전용 `5keysFrame`.
5. `bgaareainfo.load(6)`.
6. 5키 폭발 LN 행 오프셋 순서는 7키와 다르게 적혀 있다 (절 2.4).
7. 나머지(info, progress, gauge, lane, cover, assist, info2, judge, graph, fullcombo, scorebar, attack, prepare, close)는 동일하다. 5키도 7키 키 베드 시트를 쓴다.

## 11. 구현 사양에 직접 영향을 주는 beatoraja 로더/엔진 동작 (확인된 것)

### 11.1 Lua -> 스킨 구조 변환 (LuaSkinLoader.java:97-209)

- Lua 최상위 테이블이 `JsonSkin.Skin` 의 public 필드에 리플렉션으로 매핑된다. 알 수 없는 키는 무시한다. 배열 필드는 `table.keys()` 순서대로 변환된다.
- 숫자/정수 필드: `LuaValue.toint()`(실수는 0 방향 절삭). 문자열 필드: `tojstring()`(숫자도 문자열로, 정수는 정수형식). 따라서 `id = 0`, `font = 1`, `src = 1` 같은 숫자 id 는 `"0"`, `"1"` 문자열로 비교된다. `destination.id` 가 `"2001"`(문자열)이고 `gauge.id = 2001`(정수)인 경우도 같은 방식으로 일치한다.
- 속성형 필드(`draw`, `value`, `timer`, `act`, `op` 의 원소 등)는 함수 / 숫자(내장 id) / 문자열(`"return " .. 문자열` 을 컴파일한 Lua 식)만 받는다. 그 외(예: Lua boolean `true/false`)는 `null` 이 되어 해당 속성이 없는 것과 같다 (serializeLuaScript, LuaSkinLoader.java:153-166). 문자열 형태는 이번 스킨에서 쓰이지 않지만, 기존 R-BMS 가 지원하는 "한 줄 Lua 식" 과 호환된다.
- 숫자 `op` 원소: `JsonSkin.DestinationOption(int)`. 0 은 무시된다 (JsonSkin.java:468-473).
- 알 수 없는 destination id 는 건너뛴다. `src` 가 없거나 파일이 없으면 그 이미지는 객체가 만들어지지 않거나 `validate()` 실패로 제거된다.
- 객체는 destination 이 있는 것만 만들어진다 (`loadSkinObject(skin, sk, dst, p)` 는 destination 마다 호출). 정의만 있고 destination 이 없는 image/value 는 텍스처도 로드되지 않는다. 단 `note` 객체가 참조하는 이미지는 note 로딩 시 텍스처가 로드된다.

### 11.2 destination 평가 (SkinObject.java)

- 첫 keyframe 의 생략값: time/x/y/w/h/acc/angle 0, a/r/g/b 255. 이후 keyframe 은 직전 값 상속.
- 시간 계산 (prepareRegion, 360-371): timer 가 있으면 `time -= timer`, timer 가 꺼져 있으면 draw=false. `loop == -1`: 마지막 keyframe 시간을 넘으면 사라진다. 그 외: `lasttime > 0 && time > loop` 이면 `time = (time - loop) % (lasttime - loop) + loop`. `loop == 0`(기본)은 0 부터 `lasttime` 주기로 반복한다. 첫 keyframe 시간보다 이르면 draw=false.
- 보간: 선형, `acc` 1 이면 `rate^2`, 2 이면 `1-(rate-1)^2`, 3 이면 보간 없이 직전 값 유지.
- 오프셋: `region.x += off.x - off.w/2`, `region.y += off.y - off.h/2`, `region.width += off.w`, `region.height += off.h` (객체의 `relative` 가 false 일 때). `a` 는 `color.a += off.a/255` (0..1 로 clamp), `angle += off.r`. 알파 오프셋은 단일 keyframe(고정색) 또는 `rate == 0` 일 때만 반영되고 보간 구간(rate != 0)에서는 반영되지 않는 코드 구조다 (SkinObject.java:480-520). 밝기 오버레이(BLACK + offset)가 고정 단일 keyframe 이라 동작한다.
- 정렬: draw 조건(`dstdraw`)을 먼저 평가하고 하나라도 false 이면 즉시 draw=false (591-598).
- stretch: `StretchType` 11종 (STRETCH 0 ... NO_RESIZE_TRIMMED 10) 은 BJ/skin/StretchType.java 에 정의. TRIMMED 는 이미지를 자르며, 스케일하지 않고 영역에 맞춘다.

### 11.3 숫자 객체 (SkinNumber.java, JsonSkinObjectLoader.java:99-171)

- 이미지 수가 24의 배수이면 부호 있는 숫자(양수 12 + 음수 12), 아니면 `d = (이미지수 % 10 == 0) ? 10 : 11`. 11분할이면 `zeropadding` 은 JSON/Lua 필드 값과 무관하게 2 가 된다 (`d > 10 ? 2 : value.padding`). 즉 `nowscore`(11분할)는 선행 0 자리를 11번째 글리프(`0` 대신 사각 테두리 "裏0")로 채운다. 10분할은 `value.padding`(기본 0)이라 선행 공백이 비워진다. 24분할(부호)에서는 `value.zeropadding`(1: 0 으로 채움, 2: 裏0)이 쓰인다.
- 정렬 (SkinNumber.java:192-193): `keta` 칸의 오른쪽부터 숫자를 채우고 앞쪽은 null 이다. `align 0(RIGHT)`: 이동 없음(칸의 오른쪽에 붙음), `1(LEFT)`: `(w+space)*빈칸수` 만큼 왼쪽으로 이동, `2(CENTER)`: 그 절반. dst `x` 는 `keta` 칸 전체의 왼쪽 끝이다.
- 부호 숫자: 12분할 한 행 = 숫자 0-9, [10] = 裏0 또는 공백, [11] = 부호 글자. `zeropadding > 0` 이면 항상 첫 칸에 [11]. `zeropadding == 0` 이면 최상위 숫자 바로 앞에 부호가 붙는다.
- 값이 `Integer.MIN_VALUE`/`MAX_VALUE` 이면 그리지 않는다.

### 11.4 런타임 오프셋 (LaneRenderer.java:329-346, KeyInputProccessor.java:105)

- `OFFSET_LIFT(3).y = hl - region.y` (리프트 높이 px, 위 방향 +).
- `OFFSET_LANECOVER(4).y = (hl - hu) * lanecover` (음수, 레인커버만큼 아래로).
- `OFFSET_HIDDEN_COVER(5)`: 히든 활성 시 `a = 0`, `y = (lift 사용 시 (1-lift)*hidden*laneHeight, 아니면 hidden*laneHeight)`, 비활성 시 `a = -255`.
- `OFFSET_SCRATCHANGLE_1P(1).r = scratch / 6` (스크래치 디스크 회전, 도).
- 사용자 오프셋 값은 스킨 설정에서 `MainState.setSkin` 이 전역 오프셋 배열(0..199)에 복사한다 (MainState.java:113-134).

### 11.5 클릭/마우스 (SkinObject.java:671-704)

버튼 인덱스 0 = 좌(+1), 1 = 우(-1), 2/3 = +1, 4 = -1. `click` 0: 영역 안에서 해당 부호, 1: 반대 부호, 2: 좌우 분할, 3: 상하 분할. 슬라이더는 `changeable` 이면 마우스로 값을 쓴다 (레인커버: `SLIDER.LANECOVER` 가 값을 쓴다).

### 11.6 슬라이더/그래프 방향

- 슬라이더 이동: `angle` 0 위, 1 오른쪽, 2 아래, 3 왼쪽. 그리기 y = `region.y - value * range` (DOWN) 등 (SkinSlider.java:99-103). `range` 는 `dstr/sk` 배율이 곱해진 px. 레인커버: `y = 1080 - v * 853` (v=1 이면 커버 하단이 y=227 판정선에 닿는다).
- 그래프: `angle == 1` 이면 아래에서 위로(소스 이미지의 아래쪽 `value` 비율을 잘라 `height * value` 로 그림), 그 외는 왼쪽에서 오른쪽(소스의 왼쪽 `value` 비율, `width * value`). 음수 폭이면 반대 방향으로 그려진다 (SkinGraph.java:96-107).
- 스코어 그래프의 `divx=415, w=415` 는 폭 1px 짜리 소스 영역 415개로 나뉜다. 이미지 인덱스는 시간에 따라 `(time*415/20000) % 415` 로 바뀌고 소스 폭이 1 이므로 `(int)(1*value) = 0` 폭의 영역이 된다. libGDX 가 폭 0 영역을 해당 열의 색으로 늘려 그린다. R-BMS 는 1px 소스에서의 value 비율 잘라내기를 최소 1px 로 처리해야 같은 모습이 나온다 (이 처리의 정확한 렌더링 결과는 실행으로 확인하지 못함).

### 11.7 텍스트 (JsonSkinObjectLoader.java:627-679, SkinTextFont.java:168-172)

- `.fnt` 확장자면 `SkinTextBitmap`, 그 외 TTF 는 `SkinTextFont`. `font` 는 `Font.id` 문자열과 비교한다.
- `overflow` SHRINK(1)는 영역을 넘으면 글자를 줄여 맞춘다. 그림자는 offset 이 0 이 아니면 글자색의 반(`color/2`, 알파 동일)으로 `(+offsetX, -offsetY)` 에 먼저 그린다. `shadowColor` 설정은 이 경로에서 읽히지 않는다 (SkinTextFont.java:168-172). 기본 shadowColor `"ffffff00"` 은 알파 0 이지만 그림자 알파는 글자색에서 오므로 보인다.
- `constantText` 는 고정 문자열.

### 11.8 Skin 시간/종료 필드

`fadeout 500`(종료 후 페이드 시간), `scene 3600000`, `input 0`, 헤더 `close 3000`/`loadend 3500`/`playstart 1000` 은 `JsonPlaySkinObjectLoader` 가 note 로딩 시 PlaySkin 에 복사한다.

## 12. 위험, 미확인, 질문

### 12.1 위험

1. 라이선스: `Play/readme.txt` 에 "本スキンそのものを二次配布することは禁止(허가 시 예외)" 와 "改変スキンを公開する場合は原作者名(KASAKO)を記述" 가 있다. ModernChic 에서 파생한 기본 스킨을 R-BMS 에 번들하면 2차 배포에 해당할 수 있다. 사용자(소유자)가 허가를 받았거나 에셋을 새로 만들었다는 확인이 필요하다.
2. 런타임 요구: 풀 ModernChic 은 `require`(점 구분 모듈), `dofile`, `pcall`, `io`(읽기+쓰기), `os.date/time`, `luajava` 대체 모듈이 필요하다. 하나라도 빠지면 `Root.define2` 로딩 단계에서 main() 전체가 실패한다 (customfunction.lua:7 `require("luajava")`).
3. 텍스처 메모리: 폭발 30.7MB, 풀콤보 53MB, 폐점 18.5MB, hcn 7.7MB (디코드). 6400 폭 텍스처는 일부 GPU 의 최대 텍스처 크기를 넘을 수 있다. 풀콤보와 폭발은 단순화 대상 1순위.
4. Lua 5.4 정수 래핑: 절 6.1.
5. 속성 함수 오류: 오류 시 기본값 반환과 로그 폭주 방지가 필요 (`adjustedCover()` 는 HSFIX 가 OFF 면 nil 반환).
6. 속성 값이 Lua boolean 인 경우를 null 로 취급해야 한다 (info.lua:186). 이 규칙을 지키지 않고 boolean 을 그대로 해석하면 `nowbpm` 두 destination 중 하나가 사라진다.
7. 로드 시점 의존: 구조(옵션 분기, `main_state.option(NO_BGA/LN/AUTOPLAYOFF)`, `main_state.text(TABLE_FULL)`, `RGB` 색)가 로드 시점의 곡/모드/설정에 따라 결정된다. 곡 선택 직후, 플레이 시작 전에 스킨을 (재)구성해야 한다.
8. 비결정성: 키플래시 색 `math.random`. 시각 비교 시 시드 고정 필요.
9. 상세 모드/공격 모션의 프레임당 Lua 호출량이 크다 (함수 timer 160개 + 21개 + 값 31개). 단순화 시 제거하는 쪽이 안전하다.
10. 5키 폭발 LN 행 오프셋 순서(원본 버그) 처리 방침이 필요 (버그 호환 vs 수정).

### 12.2 미확인

- 절 0 의 beatoraja 클래스들(타이머 발화 시점, BPMGraph/TimingVisualizer/HitErrorVisualizer 그리기, 동영상 소스, 와일드카드 경로 해석).
- 게이지와 키 베드의 겹침 결과(절 7.4).
- 개수 집계(절 2.2)는 수기 집계이며 R-BMS 덤프로 확정 필요.
- `main_state.number(101)` 이 `-2147483648` 을 반환하는 조건(attack.lua:61).
- dp_property.lua 전문, Play/lua/dp/*.

### 12.3 질문 (구현 방향 결정에 필요)

1. 기본 스킨을 Lua 로 유지할지, JSON 으로 변환(스킨 로더 단순화)할지?
2. 상세 모드, 공격 모션, 구역 점수, 음성 훅을 완전히 제거해도 되는지?
3. 노트 분포/타이밍 분포 그래프(판정 타이밍 시각화)를 기본 스킨에 남길지?
4. BGA 없는 곡의 대체 영상(mp4)을 지원할지, 정적 이미지로 대체할지?
5. 라이선스 확인과 크레딧 문구 위치(KASAKO)?
6. 게이지/키 베드 겹침을 확인할 beatoraja 실행 캡처를 받을 수 있는지?

## 부록 A. 근거 파일 목록 (읽은 파일)

MC: `play7_hw.lua`, `play7_hw.luaskin`, `play5_hw.lua`, `config.lua`, `Play/lua/base.lua`, `Play/lua/background.lua`, `Play/lua/close.lua`, `Play/lua/require/{header,sp_property,common,textproperty}.lua`, `Play/lua/sp/{assist,attack,bomb,cover,fullcombo,gauge,graph,info,info2,inputkey,judge,keyflash,lane,notes,prepare,progress,scorebar}.lua`, `Play/lua/sp/detailinfo/{bgaareainfo,inputinformation,nentyakuinfo,rendafrequency,trend,util}.lua`, `Root/{main*,custom*,define,define2,version,author}.lua`, `Play/readme.txt`, `Play/parts/common/{bomb,oadx_bomb}/仕様.txt`, PNG 8장.

BJ: `skin/lua/{LuaSkinLoader,SkinLuaAccessor,SkinLuaPathResolver,MainStateAccessor,MainStatePropertyLuaApiExporter,SkinAudioLuaApiExporter,SkinFileLuaApiExporter,LegacySkinLuaApi}.java`(일부 구간), `skin/json/{JsonSkin,JSONSkinLoader,JsonSkinObjectLoader,JsonPlaySkinObjectLoader}.java`, `skin/{SkinObject,SkinNumber,SkinImage,SkinSlider,SkinGraph,SkinSourceImage,StretchType,Skin}.java`(일부 구간), `play/{SkinJudge,SkinNote,SkinGauge,SkinBGA,SkinHidden,LaneRenderer,LaneProperty}.java`, `MainState.java`, `skin/property/BooleanPropertyFactory.java`(일부).

읽지 못한 것: BJ `SkinNoteDistributionGraph`, `SkinBPMGraph`, `SkinTimingVisualizer`, `SkinHitErrorVisualizer`, `SkinSourceMovie`, `SkinLoader.getPath`, `IntegerPropertyFactory` 개별 id 구현, `RhythmTimerProcessor`, `JudgeManager`; MC `Play/lua/dp/*`, `dp_property.lua` 전문, `Select/Result/Decide/Course/KeyConfig/SkinSelect` 폴더.
