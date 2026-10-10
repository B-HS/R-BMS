# 기본 스킨(rbms-default) 설계서

> 최종 갱신 2026-10-11 · 상태: 골격 구현 완료(공통 모듈, 생성 스크립트, 결정 화면), 리뷰 반영(숫자 자릿수·정렬 규칙, 생성 스크립트의 구역 분리, 선곡·결과의 리플레이 슬롯) · 대응 단위: 웨이브 8A W8-2a
> 상위 사양: `docs/plan/2026-10-09-lua-skin-compat.md` §2(D1·D4·E9), §4.7, §6 웨이브 8
> 조사 근거: `docs/reference/skin-compat/`(색인 `README.md`). `[b4 §4]` 같은 표기는 그 폴더의 보고서 절이다.
> 이 문서와 `assets/skins/rbms-default/` 의 골격만 보고 나머지 화면을 만들 수 있게 쓴다. 구현 단위는 확정한 좌표를 보고에 적고, 메인이 통합할 때 6절의 해당 표에 반영한다(9절).

## 1. 목표와 제약

| 항목 | 내용 |
| --- | --- |
| 목표 | R-BMS 가 배포하는 기본 스킨 팩. beatoraja 형식 `.luaskin` 문서 9개(선곡, 결정, 플레이 5/7/9/10/14키, 결과, 코스 결과)로 첫 실행부터 전 화면을 그린다 |
| 자작(D1) | Lua, 이미지, 사운드 전부 이 저장소에서 새로 만든다. ModernChic 의 코드·이미지·폰트·사운드·동영상은 한 줄, 한 파일도 복사하거나 고쳐 넣지 않는다. 참고한 것은 "어느 화면에 무엇이 놓이는가"라는 구성뿐이고, 좌표는 3절의 자체 격자에서 다시 잡았다. Lua 는 beatoraja 스킨 형식 계약만 따르며 ModernChic 의 모듈 구조·함수 이름을 쓰지 않는다 |
| 범위(D4) | 문구는 영어. 선곡 사이드 메뉴는 설정·볼륨 2패널. 넣지 않는 것: BPM 연동 캐릭터, 음성, IR 메뉴, 플레이 이력 기록, 인프레션, 자산 변종, 로테이션, 버전 확인, 동영상 배경 |
| 표준 옵션 패널 | 선곡의 패널 1~3(플레이 옵션, 보조 옵션, 세부 옵션)은 사이드 메뉴가 아니라 엔진이 START/SELECT 로 여는 표준 기능이므로 포함한다(6.2절) |
| 저작 해상도 | 1920x1080 |
| 금지 API | luajava, 파일 쓰기(`io.open` 쓰기 모드, `main_state.file_write` 등), `os.date`. 읽기 전용 위치에서 동작해야 한다. 난수(`math.random`)도 쓰지 않는다(캡처 재현성) |
| 폰트 | Inter Regular 한 벌(OFL). 스킨 폴더 `font/` 에 사본을 둔다(4.6절) |
| 용량 | 이미지 합계 목표 6 MiB 이하. 내장 한도 10 MiB 에는 사운드 0.56 MiB 와 폰트 0.84 MiB 가 포함된다. 현재 이미지 합계 54 KiB, 전 화면 완성 시 예상 1 MiB 미만(5절) |
| 시트 크기 | 한 변 4096 초과 금지(생성 스크립트가 거부한다) |
| 생성물 | PNG 와 아틀라스 Lua 는 저장소에 커밋한다. 빌드와 CI 는 uv 없이 돈다 |

## 2. 팩 구조

### 2.1 파일

| 경로(`assets/skins/rbms-default/` 기준) | 내용 | 상태 |
| --- | --- | --- |
| `decide.luaskin` | 결정 화면 문서(type 6) | 완성 |
| `select.luaskin` | 선곡 문서(type 5) | 다음 단위 |
| `play7.luaskin`, `play5.luaskin`, `play9.luaskin` | 플레이 SP 문서(type 0, 1, 4) | 다음 단위 |
| `play14.luaskin`, `play10.luaskin` | 플레이 DP 문서(type 2, 3) | 다음 단위 |
| `result.luaskin`, `courseresult.luaskin` | 결과(type 7), 코스 결과(type 15) | 다음 단위 |
| `lua/theme.lua`, `lua/props.lua`, `lua/ui.lua`, `lua/motion.lua`, `lua/parts.lua` | 공통 모듈(4절) | 완성 |
| `lua/atlas/<구역>.lua` | 생성 스크립트가 쓰는 시트 좌표표(손으로 고치지 않는다) | common, decide 완성 |
| `lua/decide/main.lua` | 결정 화면 조립 | 완성 |
| `lua/select/**`, `lua/play/**`, `lua/result/**` | 화면 전용 모듈 | 다음 단위 |
| `image/<구역>.png` | 생성된 시트. 구역 = common, decide, select, play, result | common, decide 완성 |
| `font/Inter-Regular.ttf`, `font/Inter-OFL.txt` | `assets/fonts/` 의 사본 | 완성 |
| `sound/*.wav`(22개), `tools/generate-sounds.py` | 기존 자작 사운드. 손대지 않는다 | 기존 |
| `tools/generate-images.py`, `tools/sprites.py` | 이미지·아틀라스 생성 스크립트의 진입점(배치·출력)과 공통 그리기 헬퍼(5.3절) | 완성 |
| `tools/sections/<구역>.py` | 구역별 스프라이트 목록. 구역 하나에 파일 하나 | common, decide 완성. select, play, result 는 빈 틀 |

문서는 팩 루트에만 둔다. 앱의 팩 스캔 깊이가 1 이고(`apps/rbms-player/src/skin_select.rs` `PACK_SCAN_DEPTH`), 덤프 도구도 루트의 `.luaskin`·`.json`·`.json5` 만 문서로 본다(`apps/rbms-cli/src/skin_dump.rs` `documents`). 그래서 **루트에 `.json` 파일을 두지 않는다**(문서로 오인된다).

### 2.2 문서 type 과 화면

근거: `crates/rbms-skin/src/loader.rs` 의 `PLAY_SKIN_MODES`, `skin_type_mode`, `SCREEN_SKIN_TYPES`, `is_supported_skin_type`. 앱은 헤더 `type` 으로 화면을 고르며 파일 이름은 보지 않는다(같은 type 이 둘이면 파일 이름순 첫 문서).

| type | 화면 | 엔진의 모드 | 문서 |
| --- | --- | --- | --- |
| 0 | 플레이 7키 | `Mode::BEAT_7K` | `play7.luaskin` |
| 1 | 플레이 5키 | `Mode::BEAT_5K` | `play5.luaskin` |
| 2 | 플레이 14키 | `Mode::BEAT_14K` | `play14.luaskin` |
| 3 | 플레이 10키 | `Mode::BEAT_10K` | `play10.luaskin` |
| 4 | 플레이 9키 | `Mode::POPN_9K` | `play9.luaskin` |
| 5 | 선곡 | - | `select.luaskin` |
| 6 | 결정 | - | `decide.luaskin` |
| 7 | 결과 | - | `result.luaskin` |
| 15 | 코스 결과 | - | `courseresult.luaskin` |
| 8, 9, 16~18 | 키 설정, 스킨 설정, 24키 | - | 만들지 않는다(키 설정은 내장 화면, 24키는 내장 폴백. 사양 E4) |

### 2.3 문서별 헤더

공통: `w = 1920`, `h = 1080`, `author = "R-BMS"`, `name = "R-BMS Default (<화면>)"`. 시간 단위는 ms. 플레이의 자동 오프셋 4개(10 All offset(%), 30 Notes offset, 32 Judge offset, 33 Judge Detail offset)는 엔진이 헤더에 붙이므로 문서가 선언하지 않는다(`loader.rs` `AUTOMATIC_OFFSETS`). 사용자 옵션의 op 는 문서마다 900 부터 쓴다(문서 사이에 겹쳐도 된다). 파일 슬롯(filepath)은 어느 문서에도 없다(자산 변종을 두지 않는다).

| 문서 | input | scene | fadeout | 플레이 전용 | 사용자 옵션(property): 항목(op), 기본값 먼저 | 선언 오프셋 |
| --- | --- | --- | --- | --- | --- | --- |
| decide | 300 | 3200 | 800 | - | Stage Image: Show(900)/Hide(901). Note Graph: Show(902)/Hide(903) | 없음 |
| select | 300 | 0 | 0 | - | Stage Image: Show(900)/Hide(901) | 없음 |
| play5, play7 | 500 | 0 | 600 | loadend 1500, playstart 1000, close 1500, finishmargin 1000, judgetimer 1 | Field Side: Left(900)/Right(901). Scratch Side: Left(902)/Right(903). Judge Detail: Off(904)/Fast-Slow(905)/Milliseconds(906). Score Graph: On(907)/Off(908) | 자동 4개만 |
| play9 | 500 | 0 | 600 | 위와 같음 | Judge Detail(904~906), Score Graph(907/908). 필드는 가운데 고정 | 자동 4개만 |
| play10, play14 | 500 | 0 | 600 | 위와 같음 | Judge Detail(904~906). 스크래치는 바깥쪽 고정, 그래프 없음 | 자동 4개만 |
| result, courseresult | 500 | 3600000 | 500 | - | 없음 | 없음 |

근거와 주의
- 결정: `nowtime > input` 에 STARTINPUT(타이머 1), `nowtime > scene` 에 FADEOUT(타이머 2), FADEOUT 경과 > `fadeout` 에 플레이로 넘어간다[b4 §4, `apps/rbms-player/src/stage/decide.rs`]. scene + fadeout = 4000 은 캡처 하니스의 마지막 시각 3950ms 보다 길어야 해서 정했다(`stage/capture.rs` `DECIDE_SHOTS_MS`).
- 선곡은 `scene`·`fadeout` 을 쓰지 않는다. STARTINPUT 은 입력 잠금이 아니다[b4 §1.5, 웨이브 5 반영 사항].
- 플레이의 `loadend`·`playstart`·`close`·`finishmargin` 은 note 객체가 배치된 문서에서만 적용된다[b3 §2]. PRELOAD 는 `loadend` 와 무관하게 최소 1초다[b3 웨이브 6 반영 사항].
- 스크래치 방향과 필드 좌우처럼 앱 설정과 겹치는 옵션은 스킨 옵션이 우선한다(사양 §4.5).

## 3. 시각 규칙

방향: 어두운 슬레이트 바탕, 강조색 하나(하늘색), 직각 평면. 그림자·광택·그라디언트·둥근 모서리·배경 이미지를 쓰지 않는다. 면은 단색 사각형(`-111` 흰색 참조 이미지에 색을 입힌 것)으로, 위계는 글자 크기와 세 단계 글자색으로 만든다. 값의 정의는 `lua/theme.lua` 한 곳에 있다.

### 3.1 색

| 이름(`theme.color.*`) | 16진 | 용도 |
| --- | --- | --- |
| `bg` | `0C0F14` | 화면 바탕, 전환 덮개 |
| `panel` | `141922` | 패널 면(그래프 바탕, 이미지 자리) |
| `raised` | `1D2430` | 떠 있는 면(선택된 막대, 옵션 시트, 사이드 패널) |
| `line` | `2B3545` | 2px 구분선, 테두리 |
| `text` | `F1F4F8` | 1차 글자(제목, 값) |
| `sub` | `A7B2C1` | 2차 글자(부제, 장르) |
| `muted` | `667387` | 3차 글자(항목 이름, 안내) |
| `accent` | `4CC9F0` | 강조 하나: 진행선, 선택 표시, 포커스, 표 이름 |
| `accent_dim` | `1F5F78` | 강조의 눌린 상태·바탕 |

의미색(강조색과 별개로, 뜻이 정해진 곳에만 쓴다)

| 묶음 | 값 |
| --- | --- |
| 난이도 `theme.difficulty[0..5]` | UNKNOWN `8A97A8`, BEGINNER `59D185`, NORMAL `568CF5`, HYPER `F2B33D`, ANOTHER `F0564C`, INSANE `B26BF0` |
| 판정 `theme.judge` | PGREAT `7FE3FF`, GREAT `F2C94C`, GOOD `8BD45F`, BAD `F0885A`, POOR `F0564C`, MISS `C2455E` |
| 램프 `theme.lamp` | NO PLAY `3A4556`, FAILED `B0413E`, ASSIST `B26BF0`, LIGHT ASSIST `D68CF0`, EASY `59D185`, NORMAL `568CF5`, HARD `F0F2F5`, EX HARD `F2B33D`, FULL COMBO `4CE0D0`, PERFECT `FFE08A`, MAX `FFFFFF` |

### 3.2 격자

좌표는 전부 **설계 좌표**로 적는다: 원점이 왼쪽 위, y 가 아래로 커지고 `(x, top, w, h)` 로 쓴다. 캡처 PNG 의 픽셀 좌표와 같다. beatoraja 좌표(원점 왼쪽 아래)로의 변환 `y = 1080 - top - h` 는 `lua/ui.lua` 가 한다.

| 항목(`theme.grid.*`) | 값 | 뜻 |
| --- | --- | --- |
| `unit` | 8 | 모든 간격의 기본 단위 |
| `margin` | 64 | 좌우 바깥 여백. 내용 폭 1792 |
| `column`, `gutter`, `columns` | 120, 32, 12 | 12열. `theme.column(i, span)` 이 `x = 64 + 152 i`, `w = 152 span - 32` 를 돌려준다 |
| `header_rule` | top 104 | 머리 구분선. 그 위 캡션은 top 70(크기 18) |
| `footer_rule` | top 1000 | 바닥 구분선. 그 아래 안내 글자는 top 1022 |
| `rule` | 2 | 선 두께 |

열 시작 x: 64, 216, 368, 520, 672, 824, 976, 1128, 1280, 1432, 1584, 1736. 플레이 화면은 머리·바닥 구분선을 쓰지 않는다(필드가 위에서 아래까지 간다).

### 3.3 글자

Inter Regular 한 굵기만 있다. 굵기 대신 크기와 색으로 위계를 만든다. TTF 텍스트는 destination 높이가 em 크기이고, 상자의 위 변이 대문자 윗선이다(`crates/rbms-render/src/skin_render/text.rs`). 대문자 높이는 em 의 0.727 배다(`theme.cap_height(size)`).

| 이름(`theme.type.*`) | 크기 | 쓰는 곳 |
| --- | --- | --- |
| `display` | 96 | 결정 화면의 곡 제목 |
| `title` | 64 | 선곡·결과의 곡 제목, 큰 값 |
| `heading` | 44 | 부제, 글자로 된 값(JUDGE, KEYS) |
| `body` | 32 | 아티스트, 난이도 이름, 막대 제목 |
| `label` | 24 | 장르, 표 이름, 버튼 |
| `caption` | 18 | 항목 이름(대문자, `muted`), 안내. 최소 크기(1280x720 창에서 12px) |

숫자는 value 객체가 이미지 시트를 요구하므로 Inter 로 렌더한 숫자 띠를 쓴다(5절). 숫자 띠의 이름은 `num<em 크기>` 이고 같은 em 크기의 TTF 글자와 대문자 윗선·기준선이 맞는다. 값이 프레임마다 바뀌지 않고 글자인 것(난이도 이름, 판정 랭크, 옵션 값)은 TTF 텍스트로 쓴다(어느 배율에서도 선명하다).

숫자의 자릿수와 정렬. value 객체는 선언한 `digit` 보다 큰 값의 아래 자리만 그린다(레퍼런스와 같은 동작이다: `crates/rbms-render/src/skin_render/object.rs` `integer_glyphs`). 틀린 값이 조용히 나오지 않도록 모든 숫자는 아래 둘 중 하나를 따른다.

| 경우 | 규칙 | `ui` 의 지정 |
| --- | --- | --- |
| 본체 패스에서 값이 정해져 있는 숫자(결정 화면 전부, 결과·코스 결과의 점수와 판정 수) | 자릿수를 값에서 정한다. 칸(항목의 폭)을 넘으면 em 크기를 줄여 칸 폭에 맞춘다. 기준선은 그대로다 | `known = main_state.number(id)`, `fit = <칸 폭>`, 상자는 `baseline` |
| 화면이 떠 있는 동안 값이 바뀌는 숫자(선곡의 곡 정보, 플레이의 점수·BPM·시간) | 자릿수를 칸에 들어가는 만큼으로 고정하고, 그 자릿수의 최댓값(4자리면 9999)에서 멈춘다. 음수는 같은 크기의 최솟값에서 멈춘다 | `digit = <자릿수>`, `saturate = true` |

- 부호 있는 띠(`snum*`)는 부호가 한 자리를 쓴다. `digit = 5` 면 숫자는 4자리이고 `saturate` 도 그렇게 센다.
- 값이 없다는 표지(`props.no_value.LOW`, `HIGH`)는 `saturate` 가 그대로 넘긴다. 그 값에서는 엔진이 아무것도 그리지 않는다. `saturate` 는 `main_state.number` 로 읽으므로 레퍼런스에 없는 id 는 0 으로 읽힌다(`crates/rbms-skin/src/lua/main_state.rs` `number`).
- 숫자 띠는 칸 폭이 같은 표 숫자다. 칸 폭은 가장 넓은 숫자의 잉크 폭 + 양옆 여백이고 숫자는 칸 가운데에 놓인다. 왼쪽 정렬한 숫자의 잉크가 항목 이름의 x 에서 시작하도록 `ui` 가 첫 칸의 왼쪽 여백만큼 x 를 당긴다(오른쪽 정렬은 마지막 칸의 오른쪽 여백만큼 민다). `known` 을 준 숫자는 실제 첫 숫자와 끝 숫자의 여백을 쓰므로 "1" 로 시작해도 정확히 맞고, 그렇지 않은 숫자는 가장 넓은 숫자의 여백을 쓴다(좁은 숫자는 잉크 폭 차이의 절반만큼 안으로 들어간다).
- `saturate` 를 붙이지 않아도 되는 것은 값의 범위가 `digit` 안에 있다고 정해진 것뿐이다(시간의 초 0~59, 백분율의 소수부 0~99 등). `saturate` 는 값을 Lua 함수로 읽으므로 그런 숫자에는 `ref` 만 준다.

### 3.4 면, 선, 모서리

| 규칙 | 내용 |
| --- | --- |
| 모서리 | 전부 직각. 둥근 모서리 이미지를 만들지 않는다 |
| 선 | 2px, `line` 색. 강조선은 같은 두께의 `accent` |
| 패널 | `panel` 단색 사각형. 테두리 없음. 안쪽 여백 16 또는 24 |
| 항목 | 이름(caption, `muted`, 대문자)이 위, 값이 아래. 값들은 한 기준선에 맞춘다 |
| 난이도 표시 | 폭 8 의 세로 막대를 난이도 색으로. 색 면을 크게 칠하지 않는다 |
| 이미지 확대 | 시트에서 온 이미지와 숫자는 destination `filter = 1`(1:1 이 아니면 선형 보간). `ui` 가 기본으로 넣는다. 엔진 기본값 0 은 최근접이라 1280x720 창에서 글자가 깨진다(`loader/stretch.rs` `filtering_for`) |

### 3.5 전환과 움직임

값은 `theme.time` 과 `lua/motion.lua` 에 있다. 움직임은 짧고 한 방향이다.

| 규칙 | 내용 |
| --- | --- |
| 등장 | 알파 0 → 255, 300ms. 글자 줄은 왼쪽으로 32px 떨어진 곳에서 감속(acc 2)하며 들어온다. 형제 요소는 50ms 씩 늦춘다 |
| 유지 | 키프레임이 둘 이상이면 `loop` = 마지막 키프레임 시각(끝 상태에서 멈춤). `ui` 의 기본값이다. beatoraja 기본 `loop = 0` 은 처음부터 반복이다 |
| 퇴장 | `loop = -1`(한 번 재생하고 사라짐) |
| 화면 닫기 | FADEOUT(타이머 2)에서 `bg` 색 덮개가 알파 0 → 255. 문서의 `fadeout` 안에 끝난다 |
| 화면 열기 | 결정은 요소별 등장. 선곡·결과·플레이는 `parts.fade_from_black` 300ms |
| 금지 | 무작위 값, 3초를 넘는 등장, 반복 깜박임(선택 표시의 맥동 `motion.pulse` 하나만 허용) |

## 4. 공통 Lua 모듈

### 4.1 실행 모델

- Lua 5.2 다(mlua `lua52`). 수는 double 하나뿐이다: `7 / 2` 는 3.5 이고 정수 나눗셈 연산자가 없다. 좌표는 `ui` 가 반올림한다. 비트 연산은 `bit32`, 가변 인자 풀기는 `table.unpack` 이다.
- 엔진은 화면에 들어갈 때마다 진입 문서를 같은 Lua 상태에서 두 번 실행한다: 헤더 패스(`skin_config == nil`)와 본체 패스(`crates/rbms-skin/src/loader/lua_skin.rs`). `require` 한 모듈은 첫 패스에서 한 번만 실행되고 캐시된다.
- 그래서 **모듈 최상위에서는 `skin_config` 를 읽거나 `main_state` 함수를 부르지 않는다.** 스킨 목록용 헤더 전용 읽기에서는 `main_state` 가 빈 테이블이다. 둘 다 본체를 조립하는 함수 안에서만 쓴다.
- `require` 경로: 점을 `/` 로 바꿔 진입 문서가 있는 폴더(팩 루트) 아래에서만 찾는다(`crates/rbms-skin/src/lua/package.rs`). 모듈 이름은 항상 `lua.` 로 시작한다(예: `require("lua.ui")`, `require("lua.decide.main")`).
- 본체 패스에서는 곡과 결과 상태가 이미 확정돼 있다. `main_state.number`·`text` 를 조립 중에 읽어 자릿수에 따른 배치를 정해도 된다(결정 화면의 BPM 범위가 그렇게 한다).

문서의 꼴은 한 줄이다. 헤더와 본체의 구분은 화면 모듈의 `build` 가 한다.

```lua
return require("lua.decide.main").build()
```

```lua
function decide.build()
    local header = decide.header()
    if skin_config == nil then
        return header
    end
    local main_state = require("main_state")
    local s = ui.new(header)
    ...
    return s:finish()
end
```

### 4.2 모듈

| 파일 | 맡는 것 | 다른 모듈 의존 |
| --- | --- | --- |
| `lua/theme.lua` | 색, 글자 크기, 격자, 시간, 폰트 경로. 순수 데이터와 `theme.column`, `theme.content_width`, `theme.cap_height` | 없음 |
| `lua/props.lua` | beatoraja 속성 id 의 이름표(매직넘버 금지). `skin_type`, `image`(참조 이미지 -100~-111), `timer`, `option`, `number`, `text`, `event`, `index`(이미지 인덱스), `rate`(슬라이더·그래프 type), `offset`, `blend`, `stretch`, `acc`, `text_align`, `text_overflow`, `number_align`, `loop`, `no_value`(숫자 속성의 "값 없음" 표지 둘). 이름은 beatoraja `SkinProperty` 의 상수 이름에서 접두사를 뗀 것이고, `stretch` 의 이름만 엔진의 `StretchKind`(`crates/rbms-skin/src/loader/stretch.rs`)를 따른다(레퍼런스의 이름이 `KEEP_ASPECT_RATIO_FIT_INNER` 꼴로 길다) | 없음 |
| `lua/ui.lua` | 스킨 테이블 조립기. 설계 좌표 변환, id 발급, 객체 정의와 배치 | theme, props |
| `lua/motion.lua` | 키프레임 프리셋 | theme, props, ui |
| `lua/parts.lua` | 여러 화면이 쓰는 조합 부품 | theme, props, ui, motion |
| `lua/atlas/<구역>.lua` | 생성된 시트 좌표표 | 없음 |

화면 전용 모듈은 `lua/<화면>/` 아래에 두고 공통 모듈만 `require` 한다. 화면 모듈끼리는 서로 부르지 않는다(결과와 코스 결과는 `lua/result/` 를 같이 쓴다).

### 4.3 `ui` — 조립기

상자 `box` 는 설계 좌표 `{ x, top, w, h }` 다. 선택 인자 `opts` 는 아래 표의 키를 갖는다.

| `opts` 키 | 뜻 |
| --- | --- |
| `color` | `{ r, g, b }`. 기본 흰색(이미지 원색) |
| `a` | 알파 0~255. 기본 255 |
| `keys` | 키프레임 목록. 각 키는 `time` 과 함께 절댓값 `x`, `top`, `w`, `h`, `a`, `color`, `angle` 또는 기준 상자에 더하는 `dx`, `dtop`, `dw`, `dh` 를 가진다. 생략한 값은 **기준 상자의 값**이다(앞 키프레임을 물려받지 않는다) |
| `acc` | 보간 곡선 `props.acc.*`. 객체 전체에 하나 |
| `timer`, `loop`, `op`, `draw`, `blend`, `filter`, `center`, `offset`, `offsets`, `stretch` | beatoraja destination 필드 그대로. `timer`·`draw` 는 Lua 함수도 된다 |
| `hover` | 마우스가 올라와 있을 때만 그리는 영역(`mouseRect`). 상자 왼쪽 위 기준의 `{ x, top, w, h }` |
| `def` | `s:image` 전용. 이미지 정의에 넣을 `id`, `timer`, `cycle`, `len`, `ref`, `act`, `click` |

| 함수 | 인자 | 하는 일 |
| --- | --- | --- |
| `ui.new(header)` | 헤더 테이블 | 조립기 `s` 를 만든다. 헤더 필드를 복사하고 폰트 `inter` 를 등록한다 |
| `ui.merge(opts, overrides)` | 두 테이블 | 얕게 합친 새 테이블. 넘겨받은 테이블을 고치지 않는다 |
| `ui.box(x, top, w, h)`, `ui.inset(box, left, top, right, bottom)` | - | 상자 만들기와 안쪽으로 줄이기 |
| `s:atlas(name)` | 구역 이름 | `lua/atlas/<name>.lua` 를 읽어 source `name` 을 등록하고 스프라이트를 이름으로 쓸 수 있게 한다 |
| `s:sprite(name)` | 스프라이트 이름 | `{ src, x, y, w, h, divx, divy, cell_w, cell_h, pad, size, ink }`. 없는 이름은 오류 |
| `s:rect(box, color, opts)` | - | 단색 사각형(`-111` 에 색) |
| `s:reference(image_id, box, opts)` | `props.image.*` | 스테이지 파일(-100), 배너(-102) 등 참조 이미지 |
| `s:image(sprite, box, opts)` | 스프라이트 이름 | 이미지 정의(없으면 만든다)와 배치. `box.w`, `box.h` 를 생략하면 스프라이트 한 칸 크기 |
| `s:define_image(sprite, def)` | - | 배치 없이 이미지 정의만. note, gauge, songlist 처럼 id 만 넘기는 곳에 쓴다. id 를 돌려준다 |
| `s:text(spec, box, opts)` | `spec = { text 또는 ref 또는 value, size, align, overflow, wrap, id }` | 텍스트 정의와 배치. `align` 은 "left"(기본), "center", "right" 이고 `box.x` 는 **언제나 상자의 왼쪽 끝**이다(기준점 변환은 `ui` 가 한다). `box.h` 를 생략하면 `size` |
| `s:number(spec, box, opts)` | `spec = { sprite, ref 또는 value, digit 또는 known, fit, saturate, align, size, padding, zeropadding, space, id }` | 숫자 정의와 배치. 3.3절의 자릿수 규칙대로 `known`(본체 패스에서 읽은 값. 자릿수와 양끝 여백을 여기서 정한다) 또는 `digit` + `saturate` 를 준다. `fit` 은 잉크 폭의 상한이고 넘으면 em 크기를 줄인다. 상자는 `{ x, top }` 또는 `{ x, baseline }` 이다: `top` 은 숫자 글자의 윗선(시트 여백은 `ui` 가 뺀다), `baseline` 은 기준선이다(크기가 줄어도 기준선이 유지된다). `box.x` 는 잉크의 왼쪽 끝이고, `align` 이 "right"/"center" 이면 `box.w` 안에서 잉크를 맞춘다. `size` 를 주면 시트를 그 em 크기로 늘리거나 줄인다. 배치한 상자(아래 `number_box` 의 결과)를 돌려준다 |
| `s:number_box(spec, box)` | `s:number` 와 같음 | 배치하지 않고 숫자가 놓일 상자만 계산한다: `x`, `top`, `w`, `h` 는 첫 칸, `digits` 는 자릿수, `span` 은 잉크 전체 폭, `rise` 는 숫자 글자의 높이. 숫자 뒤에 다른 것을 붙여 놓을 때 쓴다 |
| `ui.digit_count(value)` | 수 | 그 값을 적는 데 드는 자릿수(부호 제외, 값 없음 표지와 1 미만은 1) |
| `s:object(list, def, box, opts)` | 목록 이름("slider", "graph", "judgegraph", "bpmgraph", "gaugegraph", "imageset", "judge", ...) | `def` 를 그 목록에 넣고 배치한다. `box` 가 없으면 정의만 |
| `s:single(key, def, box, opts)` | "note", "gauge", "bga", "songlist" | 하나만 있는 객체를 넣고 배치한다. `box` 가 없으면 키프레임 없는 destination(note, songlist) |
| `s:dst(id, box, opts, parent)` | - | 스킨에 넣지 않고 destination 테이블만 만든다. songlist 의 `liston`·`text`, note 의 `group`, judge 의 `images` 처럼 객체 안에 들어가는 destination 용. `parent` 상자를 주면 그 상자 기준의 상대 좌표로 뒤집는다(막대 안의 글자 등) |
| `s:place(id, box, opts)` | - | 이미 정의된 id 를 배치한다 |
| `s:finish()` | - | 빈 목록을 지우고 스킨 테이블을 돌려준다 |

id 규칙: 스킨 전체에서 종류가 달라도 id 가 겹치면 안 된다(엔진이 목록을 차례로 찾는다). 그래서 종류 접두사를 붙인다.

| 종류 | id | 예 |
| --- | --- | --- |
| source | 구역 이름 | `common`, `decide` |
| font | `inter` 하나 | - |
| image | `img.<스프라이트 이름>` 또는 `def.id` | `img.block` |
| text, value | `txt.<이름>`, `num.<이름>`. `spec.id` 를 생략하면 `txt.#<일련번호>`, `num.#<일련번호>` | `txt.title` |
| 그 밖 | `<목록 이름>.#<일련번호>` 또는 직접 지정 | `judgegraph.#41` |

다른 객체가 id 로 가리켜야 하는 것(songlist 의 text, judge 의 number 등)만 이름을 직접 준다.

### 4.4 `motion` — 키프레임 프리셋

모두 `opts` 테이블을 돌려주며 마지막 인자로 받은 `opts` 와 합친다.

| 함수 | 움직임 |
| --- | --- |
| `motion.fade_in(start, duration, opts)` | `start` 전에는 보이지 않다가 알파 0 → 255. `duration` 기본 300 |
| `motion.fade_out(start, duration, opts)` | `start` 까지 보이다가 알파 255 → 0, 그 뒤 사라짐(`loop = -1`) |
| `motion.slide_in(start, duration, dx, dtop, opts)` | `(dx, dtop)` 만큼 떨어진 곳에서 알파 0 으로 시작해 제자리로. 감속 |
| `motion.grow_right(start, duration, opts)` | 폭 0 에서 오른쪽으로 자람. 감속 |
| `motion.grow_down(start, duration, opts)` | 높이 0 에서 아래로 자람. 감속 |
| `motion.grow_up(start, duration, height, opts)` | 아래 변을 고정하고 위로 자람 |
| `motion.progress_right(duration, opts)` | 0 에서 `duration` 까지 등속으로 폭이 참 |
| `motion.pulse(period, low, opts)` | 알파가 255 → `low` → 255 를 반복 |
| `motion.staggered(index, start)` | `start + index * 50` |

### 4.5 `parts` — 조합 부품

| 함수 | 그리는 것 |
| --- | --- |
| `parts.backdrop(s)` | 화면 전체 `bg` 사각형 |
| `parts.rule(s, top, opts)` | 내용 폭 전체의 2px 구분선 |
| `parts.frame(s, caption)` | 머리·바닥 구분선과 머리 캡션 |
| `parts.panel(s, box, opts)` | `panel` 색 면 |
| `parts.caption(s, label, box, opts)` | 항목 이름(크기 18, `muted`) |
| `parts.by_difficulty(place)` | 난이도 0~5 에 대해 `place(index, op, color, name)` 호출. op 는 150~155 |
| `parts.difficulty_rect(s, box, opts)` | 난이도 색 사각형 6개(각자 op 로 하나만 그려진다) |
| `parts.difficulty_text(s, spec, box, opts)` | 난이도 이름 6개(색 포함) |
| `parts.key_mode_text(s, spec, box, opts)` | "5 KEYS" ~ "14 KEYS"(op 160~164) |
| `parts.judge_rank_text(s, spec, box, opts)` | "VERY HARD" ~ "VERY EASY"(op 180~184) |
| `parts.fade_from_black(s, duration)` | 장면 시작에 `bg` 덮개가 걷힘 |
| `parts.fade_to_black(s, start, duration)` | FADEOUT 타이머에 `bg` 덮개가 덮임 |

정적 조건(난이도, 키 수, 스킨 옵션)에 걸린 객체는 엔진이 로드 때 한 번 평가해 지운다. 그래서 6벌을 선언해도 화면에 남는 것은 한 벌이다(결정 화면: 선언 59, 남는 것 43).

### 4.6 폰트

스킨 폴더 안의 파일을 쓴다: `font = { { id = "inter", path = "font/Inter-Regular.ttf" } }`.

| 확인한 것 | 근거 |
| --- | --- |
| 폰트 경로는 스킨 루트 안에서만 해석된다. `assets/fonts/` 를 직접 가리킬 수 없다 | `crates/rbms-skin/src/loader.rs` `assemble` 의 `skin.resolve(&pattern)` |
| 선언되지 않은 폰트 id 를 쓰는 text 는 앱 기본 글꼴로 그려지지만 대체 경로다: 축소(overflow)·줄바꿈·그림자가 적용되지 않는다 | `crates/rbms-render/src/skin_render/text.rs` `draw_stand_in` |
| 폴더 안 사본은 엔진의 정식 TTF 경로로 그려진다(높이 = em, 축소, 정렬) | 시험 팩 캡처와 결정 화면 캡처(8절) |
| 스킨 폰트에 없는 글자(한글, 일본어)는 앱의 다국어 폰트로 그려진다 | 사양 E5 |

`assets/fonts/Inter-Regular.ttf` 와 바이트가 같은 사본이므로 W8-3(내장)에서 같은 내장 바이트를 두 경로에 쓰면 실행 파일은 한 벌분만 커진다.

## 5. 자산

### 5.1 원칙

- 이미지는 전부 `tools/generate-images.py` 가 만든다. 손으로 그린 파일을 넣지 않는다.
- 구역 하나가 시트 한 장(`image/<구역>.png`)과 좌표표 한 장(`lua/atlas/<구역>.lua`)이다. Lua 는 좌표를 적지 않고 스프라이트 이름만 쓴다.
- 글자·숫자 스프라이트는 흰색으로 만들고 색은 destination 에서 입힌다. 색이 구조의 일부인 것(노트, 게이지 칸)만 색을 넣어 만든다.
- 배경, 패널, 선, 막대, 덮개는 이미지가 아니라 색 사각형이다.

### 5.2 시트 목록

| 파일 | 크기 | 스프라이트 | 쓰는 화면 | 용량 |
| --- | --- | --- | --- | --- |
| `image/common.png` | 1024x161 | `num18`, `num24`, `num32`, `num44`, `num64`, `num96`(숫자 0~9 한 줄 10칸, 흰색), `snum24`, `snum32`(부호 있는 24칸: 12열 2행, 열 10 은 빈칸, 열 11 은 + 와 -), `block`(8x8 흰색) | 전부 | 39,312 B(실측) |
| `image/decide.png` | 1024x112 | `num144`(레벨 숫자) | 결정 | 16,043 B(실측) |
| `image/select.png` | 1024x512 이내 | 막대 7종(곡, 폴더, 표, 코스, 곡 없음, 명령, 검색. 각 760x64 단색 + 왼쪽 띠), 라벨 글자 5종(LN, RANDOM, MINE, CN, HCN. 크기 18), 폴더 분포 그래프용 11색 띠(11x1 칸), 스크롤 손잡이, 사이드 탭 아이콘 2종(톱니, 스피커. 도형으로 그린다), 볼륨 손잡이, 즐겨찾기 표시 | 선곡 | 약 100 KB(예상) |
| `image/play.png` | 2048x1024 이내 | 노트(레인 색 4종: 흰 건반, 검은 건반, 스크래치, 9키용 5색. 종류별: 일반, LN 시작·끝·본체·누름, HCN 4종, 지뢰), 판정 글자 6종(PGREAT 는 2프레임), 콤보 숫자(크기 96, 10칸), 판정 상세 글자(FAST, SLOW)와 부호 숫자, 게이지 칸 6종(4칸 폭), 폭발 8프레임(256x256, 평면 고리), 키 빔(세로 알파 띠), 큰 글자(READY, FAILED, FULL COMBO, LOADING) | 플레이 5종 | 약 600 KB(예상) |
| `image/result.png` | 512x256 이내 | 판정 표의 작은 표지 등 TTF 로 못 쓰는 것만. 랭크 글자와 클리어 이름은 TTF | 결과, 코스 결과 | 약 30 KB(예상) |

합계 예상 1 MiB 미만. 목표 6 MiB 와의 차이는 예비다.

### 5.3 생성 스크립트 `tools/generate-images.py`

| 항목 | 내용 |
| --- | --- |
| 실행 | `uv run assets/skins/rbms-default/tools/generate-images.py [구역 ...] [--check]`. 구역을 생략하면 전부 |
| 의존성 | PEP 723 인라인 메타. `requires-python = ">=3.11"`, `pillow==12.3.0` 하나(2026-10-11 PyPI 최신 안정판, 2026-07-01 배포). 버전을 고정한 것은 래스터 결과를 고정하기 위해서다 |
| 폰트 | `font/Inter-Regular.ttf`(스킨 폴더 안의 것) |
| 구조 | 파일 셋으로 나뉜다. `tools/generate-images.py`(진입점: 등록표 `SECTIONS`, 선반 배치, PNG·아틀라스 출력, `main`), `tools/sprites.py`(공통 그리기 헬퍼와 `Sprite`), `tools/sections/<구역>.py`(구역마다 하나. `build() -> list[Sprite]` 와 그 구역의 상수). 진입점이 `sections` 와 `sprites` 를 import 한다(스크립트 폴더가 import 경로에 있다). 진입점은 `sys.dont_write_bytecode` 를 켜므로 `__pycache__` 가 생기지 않는다 |
| 공통 헬퍼(`sprites.py`) | `digit_strip(name, size)`(10칸 숫자 띠), `signed_digit_strip(name, size)`(24칸), `word(name, text, size, color)`(글자 한 덩이), `block(name, w, h, color)`(단색 사각형), `tinted(mask, color)`(알파 마스크에 색). 전부 `Sprite(name, image, divx, divy, meta)` 를 돌려준다 |
| 구역 파일 | `sections/common.py`, `sections/decide.py` 는 완성. `sections/select.py`, `sections/play.py`, `sections/result.py` 는 빈 목록을 돌려주는 틀이다. 다음 단위는 **자기 구역 파일 하나**만 고친다. 진입점과 `sprites.py` 는 고치지 않는다(필요한 그리기 함수가 없으면 자기 구역 파일 안에 둔다) |
| 배치 | 높이 내림차순, 이름순으로 정렬해 선반식으로 놓는다. 스프라이트 사이 2px. 시트 폭은 128~4096 의 2의 거듭제곱 중 정사각형 안에 들어가는 가장 작은 것 |
| 출력 | `image/<구역>.png`(RGBA, 투명 픽셀도 RGB 는 흰색: 선형 보간 때 어두운 테두리가 생기지 않게)와 `lua/atlas/<구역>.lua`(이름순). 스프라이트가 없는 구역은 아무것도 쓰지 않는다 |
| 결정성 | 같은 입력에서 바이트가 같다(8절에서 확인). 시각·난수·환경 변수에 의존하지 않는다. 다른 OS 나 다른 Pillow 판에서의 바이트 동일성은 보장하지 않으므로 CI 에서 다시 만들지 않는다 |
| `--check` | 쓰지 않고, 만든 바이트가 저장소의 파일과 같은지 비교한다. 다르면 종료 코드 1. 스크립트만 고치고 생성물을 빠뜨린 커밋을 찾는 용도다 |

숫자 띠의 칸: 폭은 그 띠에 든 글자(숫자 10개, 부호 띠는 + 와 - 포함)의 잉크 폭 최댓값 + 양옆 여백(각각 em 의 0.04배를 반올림, 최소 1px), 높이는 숫자 글자의 위아래 끝 + 위아래 여백 2px. 각 글자는 잉크가 칸 가운데에 오도록(남는 1px 은 오른쪽), 같은 기준선에 놓인다. 칸 폭은 18 → 12, 24 → 15, 32 → 19, 44 → 28, 64 → 40, 96 → 59, 144 → 89 다. 아틀라스의 `size` 는 em 크기, `pad` 는 위아래 여백, `ink` 는 숫자 0~9 의 잉크 폭 10개다(`ui` 가 양끝 여백을 여기서 계산한다. 3.3절). beatoraja 의 프레임 수 규칙상 10칸은 숫자만, 24칸은 부호 있는 숫자(`zeropadding` 으로 채움 방식 지정)다[b2 §10.2].

## 6. 화면별 설계

좌표는 설계 좌표 `(x, top, w, h)`. 결정 화면은 최종값이고, 나머지는 영역 수준이다. 구현 단위는 캡처를 보며 확정한 좌표를 보고에 적고, 이 절의 표는 메인이 통합할 때 고친다(9절).

### 6.1 결정(type 6) — 구현 완료

구현: `decide.luaskin`, `lua/decide/main.lua`. 캡처: 8.3절.

(가) 영역

| 영역 | x | top | w | h | 내용 |
| --- | --- | --- | --- | --- | --- |
| 머리 | 64 | 70 | 1792 | 36 | 캡션 "CHART SELECTED"(top 70, 크기 18), 구분선(top 104, h 2) |
| 난이도 막대 | 64 | 256 | 8 | 320 | 난이도 색. 표 이름이 없는 곡이면 top 296, h 280 으로 줄인다 |
| 곡 글줄 | 104 | 256~568 | 1112 | - | 표 이름(top 256, 크기 24, `accent`), 장르(296, 24, `sub`), 제목(344, 96, `text`), 부제(464, 44, `sub`), 아티스트(536, 32, `text`). 전부 왼쪽 정렬, 넘치면 가로 축소. Stage Image 옵션이 Hide 면 폭이 내용 폭 끝까지인 1752 다(본체 패스에서 `skin_config.option["Stage Image"]` 를 읽는다) |
| 스테이지 그림 | 1280 | 200 | 576 | 432 | `panel` 면 위에 -100 을 비율 유지(stretch 1)로. 그림이 없으면 "NO STAGE IMAGE"(크기 18, `muted`, 가운데) |
| 항목 구분선 | 64 | 672 | 1792 | 2 | - |
| LEVEL | 64 | 696 | 424 | 140 | 이름(top 696). 숫자 `num144`(윗선 top 728, 기준선 836, 난이도 색). 잉크 폭이 칸의 절반(212)을 넘으면 그 폭에 맞게 줄인다(2자리까지는 줄지 않는다). 숫자 잉크의 오른쪽 끝에서 24px 뒤에 난이도 이름(크기 32, 기준선 836, 난이도 색) |
| BPM | 520 | 696 | 424 | 140 | 숫자 `num64`(윗선 top 787, 기준선 836). 변속이 없으면 최대 BPM 하나(`text`). 있으면 최소(`sub`), 폭 24 높이 4 의 줄표(`muted`. 양옆 간격 12), 최대(`text`). 전체가 칸 폭을 넘으면 최소와 최대를 같은 크기로 줄인다(합쳐 9자리까지는 줄지 않는다) |
| NOTES | 976 | 696 | 272 | 140 | 총 노트 수 `num64`. 칸 폭을 넘으면 줄인다(6자리까지는 줄지 않는다) |
| JUDGE | 1280 | 696 | 272 | 140 | 판정 랭크 이름(크기 44, 기준선 836) |
| KEYS | 1584 | 696 | 272 | 140 | "7 KEYS" 등(크기 44, 기준선 836) |
| 노트 분포 | 64 | 872 | 1792 | 112 | 캡션 "NOTE DISTRIBUTION"(top 872). 그래프 상자 (64, 900, 1792, 84): `panel` 면, judgegraph, bpmgraph |
| 진행선 | 64 | 1000 | 1792 | 2 | `line` 색 바닥 구분선 위에 `accent` 선이 왼쪽부터 찬다 |
| 안내 | 64 | 1022 | 1792 | 18 | "ENTER  SKIP        ESC  BACK"(`muted`) |
| 닫기 | 0 | 0 | 1920 | 1080 | 항목 구분선 자리 (64, 672, 1792, 2) 에 `accent` 선, 전체 덮개 `bg`. 선은 글줄과 스테이지 그림을 지나지 않는다 |

(나) 객체

| 객체 | 종류 | 값·조건 | 자산 |
| --- | --- | --- | --- |
| 바탕, 구분선, 패널, 막대, 줄표, 진행선, 덮개 | image(-111 에 색) | 난이도 막대는 op 150~155 로 6벌 | 없음 |
| 캡션, 항목 이름, 안내, "NO STAGE IMAGE" | text(constantText) | "NO STAGE IMAGE" 는 op { 900, 190 } | 폰트 |
| 표 이름, 장르, 제목, 부제, 아티스트 | text(ref 1003, 13, 10, 11, 16) | overflow 1(축소) | 폰트 |
| 스테이지 그림 | image(-100) | op { 900, 191 }, stretch 1 | 곡의 그림 |
| 레벨 | value(ref 96) | 자릿수는 본체 패스에서 `main_state.number(96)` 로 정한다(3.3절의 `known`, `fit` 212). op 150~155 로 색 6벌 | `decide` 의 `num144` |
| 난이도 이름 | text | op 150~155 | 폰트 |
| BPM | value(ref 90, 91) | 최대 하나는 op 176, 최소·줄표·최대는 op 177. 두 값의 자릿수를 본체 패스에서 읽어(`known`) 줄표와 최대의 x, 줄일 크기를 정한다 | `common` 의 `num64` |
| 노트 수 | value(ref 74) | 자릿수는 본체 패스에서 정한다(`known`, `fit` 272) | `num64` |
| 판정 랭크, 키 수 | text | op 180~184, op 160~164 | 폰트 |
| 노트 분포 | judgegraph(type 0, backTexOff 1) | op 902 | 없음 |
| BPM 변화 | bpmgraph(선 두께 2, 색 6개 지정) | op 902 | 없음 |

(다) 애니메이션(장면 시계, ms)

| 시각 | 움직임 |
| --- | --- |
| 0~350 | 난이도 막대가 위에서 아래로 자란다 |
| 100~600 | 곡 글줄 5개가 50ms 간격으로, 왼쪽 32px 에서 300ms 동안 들어온다 |
| 150~450, 250~650 | 스테이지 패널, 스테이지 그림 등장 |
| 50~350 | 항목 구분선이 오른쪽으로 자란다 |
| 350~850 | 항목 5개가 50ms 간격으로 등장(각 300ms) |
| 500~800 | 노트 분포 등장 |
| 0~3200 | 진행선이 등속으로 찬다(`scene` 과 같다) |
| STARTINPUT + 0~200 | 안내 등장 |
| FADEOUT + 0~300 | 항목 구분선 위로 `accent` 선이 왼쪽에서 오른쪽으로 자란다 |
| FADEOUT + 200~700 | 덮개가 덮인다. 800 에 엔진이 화면을 넘긴다 |

(라) 입력: 클릭 가능한 객체 없음. 건너뛰기(STARTINPUT 뒤 건반 1·3·5·7 또는 Enter)와 취소(Esc 또는 START+SELECT)는 엔진이 한다[b4 §4].

(마) 뺀 것

| 뺀 것 | 이유 |
| --- | --- |
| 배경 그림·동영상 | D4, 3절의 평면 양식 |
| 모서리 조준 장식 | 양식에 없는 장식 |
| 코스 단계 수 표시 | 코스 진행은 플레이 화면의 곡 머리에 둔다(6.3절, 6.4절의 곡 머리) |
| 도움말(tips) 문구 | 범위 밖 |
| 곡 길이, TOTAL | 선곡 화면에서 이미 보여 준다. 결정은 다섯 항목으로 고정 |

### 6.2 선곡(type 5)

(가) 영역

| 영역 | x | top | w | h | 내용 |
| --- | --- | --- | --- | --- | --- |
| 머리 띠 | 0 | 0 | 1920 | 104 | `bg` 불투명. 곡 목록 위에 그린다(목록의 위쪽을 가린다) |
| 머리: 캡션 | 64 | 70 | 288 | 18 | "MUSIC SELECT" |
| 머리: 검색 | 368 | 56 | 728 | 32 | 이름 "SEARCH"(caption, `muted`) + 입력 칸(`panel` 면, 아래 변에 `line` 선) 위의 editable text(ref 30). 평상시 칸은 비어 있다((라) 참고) |
| 머리: 필터 버튼 3개 | 1128, 1368, 1608 | 40 | 224, 224, 248 | 56 | MODE(text 60), SORT(text 61), LN MODE(Lua 값 함수: `event_index(308)` → 이름). 이름(caption) 위, 값(label) 아래 |
| 곡 머리 | 64 | 136 | 1032 | 200 | 난이도 막대 + 폴더 경로(text 1000), 장르, 제목(64), 아티스트 |
| 스테이지 그림 | 64 | 368 | 424 | 318 | `panel` 면 위에 -100, 비율 유지. 그림이 없는 곡은 면만 남는다. 옵션 Hide 면 면도 그리지 않는다 |
| 곡 항목 | 520 | 368 | 576 | 318 | 2행 3칸: LEVEL(+난이도 이름), BPM, NOTES / LENGTH(분:초), JUDGE, TOTAL |
| 기록(곡 막대일 때) | 64 | 718 | 1032 | 250 | CLEAR(램프 이름, 램프 색), RANK(AAA~F, 크기 96), EX SCORE, RATE, MAX COMBO, MISS COUNT, PLAY COUNT, 판정 5칸과 가로 막대 |
| 폴더 정보(폴더 막대일 때) | 64 | 368 | 1032 | 600 | 곡 수(number 300), 램프 11종 개수(320~330)와 색 띠 |
| 코스 정보(코스 막대일 때) | 64 | 368 | 1032 | 600 | 단계 곡명(text 150~154), 제약 표지(op 1002~1017 을 글자 칩으로) |
| 곡 목록 | 1128 | 104 | 728 | 896 | 막대 높이 64, 간격 8(피치 72). 슬롯 15개, 가운데 슬롯 7 의 top 508. 슬롯 i 의 top = 508 + (i - 7) x 72. 선택된 막대는 x 1096, w 760, `raised` 면 + 왼쪽 `accent` 띠 |
| 스크롤 막대 | 1872 | 136 | 8 | 832 | slider type 1(아래로, 드래그 가능) |
| 바닥 띠 | 0 | 1000 | 1920 | 80 | `bg` 불투명. 안내 글자 (64, 1022) |
| 사이드 탭 | 0 | 444 | 48 | 192 | 48x96 버튼 2개: SETTINGS, VOLUME |
| 사이드 패널 | 64 | 136 | 728 | 832 | `raised` 면. 왼쪽 밖(x -728)에서 200ms 에 들어온다. 곡 정보 위에 덮는다 |
| 옵션 시트(패널 1~3) | 64 | 312 | 1792 | 456 | `raised` 면 + 위쪽 `accent` 선. 뒤에 `bg` 알파 160 의 전체 가림막 |

(나) 객체

| 객체 | 종류 | 값·타이머·조건 | 자산 |
| --- | --- | --- | --- |
| 곡 목록 | songlist | `center = 7`, `clickable = { 3..11 }`. `listoff`/`liston` 15개(imageset 7종), `text` 11종(크기 28. 일반 `text`, 신규 `accent`, 곡 없음 `muted`), `level` 7종(`num24`, 난이도 색), `lamp` 11종(`img.block` 8x64 에 램프 색), `trophy` 3종(`img.block` 에 동·은·금색), `label` 5종, `graph`(type -1, 11칸) | `select` 시트 |
| 스크롤 손잡이 | slider(type 1, angle 2, range 800, changeable) | - | `select` |
| 곡 정보 글자 | text(ref 1000, 13, 10, 16, 1003) | 타이머 11(SONGBAR_CHANGE)로 150ms 등장 | 폰트 |
| 곡 항목·기록 숫자 | value(ref 96, 90, 91, 74, 1163, 1164, 368, 71, 102, 103, 75, 76, 77, 80~84) | 곡 막대 op 2 | `common` |
| 판정 가로 막대 | graph(type 140~144) | op 2, 타이머 11 | `img.block` 에 판정 색 |
| 램프 이름, 랭크 | text | op 100~105, 1100~1104 / op 200~207 | 폰트 |
| 스테이지 그림 | image(-100) | op { 900 } 만. 선곡에서는 op 190·191 을 걸지 않는다(엔진이 선곡의 커서 곡에 대해 답하지 않아 둘 다 거짓이다. 7절). 그림이 없으면 참조 이미지는 아무것도 그리지 않는다. 타이머 11 로 200ms 등장 | - |
| 폴더·코스 정보 | value(ref 300, 320~330), text(ref 150~154) | op 1 / op 3 | `common` |
| 필터 버튼 | image(act 11, 12, 308) + text | 호버 강조(`hover`) | `img.block` |
| 기능 버튼(안내 줄 오른쪽) | image(act 16 AUTOPLAY, 315 PRACTICE, 90 FAVORITE) + text | - | `img.block` |
| 리플레이 슬롯(기능 버튼 옆) | 이름 "REPLAY" + 칸 4개("1"~"4". 각 40x32) | 칸 n 의 글자색: 리플레이가 있으면(op 197, 1197, 1200, 1203) `text`, 없으면(op 196, 1196, 1199, 1202) `muted`. 선택된 슬롯(op 1205~1208)은 칸 아래 변에 `accent` 선 2px. 칸 n 의 클릭은 act 19, 316, 317, 318(그 슬롯의 리플레이를 재생. 없는 슬롯은 엔진이 무시한다) | `img.block` |
| 사이드 메뉴 | image(act = Lua 함수), text, slider(type 17, 18, 19), value(ref 57, 58, 59) | 열림 상태는 Lua 지역 변수 + `timer_util.timer_observe_boolean`. SETTINGS: KEY CONFIG(act 13), SKIN(act 14), LN MODE(308), BGA(72), GAUGE AUTO SHIFT(78), JUDGE ALGORITHM(340). VOLUME: MASTER, KEY, BGM 슬라이더와 백분율 | `select` 의 아이콘, 손잡이 |
| 옵션 시트 | image, text(Lua 값 함수) | 패널 n: op 20 + n, 등장 타이머 20 + n, 퇴장 타이머 30 + n | 폰트 |
| 덮개 | image | `parts.fade_from_black(300)` | - |

옵션 시트의 내용. 칸은 건반 1~7 과 같은 순서로 놓고 칸마다 "KEY n", 항목 이름, 현재 값을 적는다. 값은 `main_state.event_index(id)` 를 읽는 Lua 값 함수로 이름표에서 고른다(이미지 시트를 만들지 않는다). 엔진이 값을 주지 않는 항목은 "NOT AVAILABLE"(`muted`)로 적는다(`apps/rbms-player/src/stage/select/panel.rs`, `skin_host/options.rs`).

| 패널 | 여는 키 | 칸(건반 순서) |
| --- | --- | --- |
| 1 PLAY OPTIONS | START | RANDOM(index 42. 건반 1·2), GAUGE(40. 건반 3), DP OPTION(54. 건반 4), HI-SPEED FIX(55. 건반 5), RANDOM 2P(43. 건반 6·7), TARGET(text 3. 스크래치) |
| 2 ASSIST OPTIONS | SELECT | EX JUDGE, CONSTANT(index 302), JUDGE AREA, LEGACY NOTE(304), MARK NOTE, BPM GUIDE, NO MINE. R-BMS 에 설정이 있는 것은 CONSTANT 와 LEGACY NOTE 둘이고 나머지 다섯은 "NOT AVAILABLE" |
| 3 DETAIL OPTIONS | START+SELECT | BGA(index 72. 건반 1), GAUGE AUTO SHIFT(78. 건반 2), TIMING AUTO ADJUST(75. 건반 3), JUDGE TIMING(number 12. 건반 5·7), NOTE DISPLAY TIME(건반 4·6. "NOT AVAILABLE") |

(다) 애니메이션: 장면 시작 덮개 300ms. 곡 이동(타이머 11)마다 곡 글줄 150ms 슬라이드, 그림 200ms 등장, 판정 막대 300ms 자람. 목록 스크롤 보간은 엔진이 한다. 선택 막대의 `accent` 띠는 `motion.pulse(2000, 160)`. 옵션 시트는 150ms 에 아래 24px 에서 올라오며 등장, 퇴장은 반대(`loop = -1`).

(라) 입력

| 객체 | 이벤트 |
| --- | --- |
| 곡 막대(슬롯 3~11) | 엔진의 막대 클릭(좌: 선택·열기, 우: 닫기) |
| 스크롤 손잡이 | 슬라이더 드래그(type 1) |
| MODE, SORT, LN MODE | act 11, 12, 308(좌클릭 다음, 우클릭 이전) |
| AUTOPLAY, PRACTICE, FAVORITE | act 16, 315, 90 |
| 리플레이 칸 1~4 | act 19, 316, 317, 318(그 슬롯을 재생한다. 선택된 슬롯은 바꾸지 않는다). 선택된 슬롯은 숫자 키 4 로 넘기고 건반 7 이 재생한다(엔진. 사양 D8) |
| 사이드 탭 2개 | Lua 함수(패널 여닫기. 한 번에 하나만 열림) |
| KEY CONFIG, SKIN | act 13, 14 |
| 볼륨 슬라이더 3개 | type 17, 18, 19 드래그 |
| 검색 | 검색 칸은 ref 30 의 editable text 다: 클릭하면 그 객체가 입력 중인 줄을 자기 자리에 직접 그리고, 확정하면 엔진에 검색어로 기록된다(`crates/rbms-render/src/skin_render/text_input.rs`, `skin_host/writers.rs`). 선곡에서 text 30 을 읽으면 언제나 빈 문자열이므로(`skin_host/select.rs`) 평상시 칸은 비어 있다. 숫자 키 0 으로 여는 엔진 검색의 검색어는 스킨이 읽을 수 없고 시스템 오버레이가 보여 준다. 칸 안에 안내 문구를 겹쳐 두지 않는다(입력 중인 줄과 같은 자리에 그려진다). 안내는 바닥 안내 줄의 "0  SEARCH" 로 한다 |

(마) 뺀 것: BPM 연동 캐릭터, IR 램프·랭킹 패널과 IR 상자, 라이벌 비교, 도움말 장면, 배너, 서브타이틀 흐름 글자, 시작 연출의 셔터·줌, 목록 배치·언어 선택 옵션(D4), "NO STAGE IMAGE" 문구(선곡에서는 그림 유무를 알 수 없다. 7절). 종류별 노트 수·밀도·주 BPM 은 선곡에서 값이 없다(사양 웨이브 9). 리플레이 슬롯 4개는 빼지 않는다: 엔진이 네 슬롯의 유무·선택 옵션과 재생 이벤트를 모두 준다(`skin_host/select.rs` `REPLAY_OPTIONS`, `stage/select/events.rs`).

### 6.3 플레이 SP(type 0, 1, 4)

구조: `lua/play/main.lua` 의 `play.build(spec)` 하나를 다섯 문서가 매개변수만 바꿔 부른다. 문서는 `return require("lua.play.main").build({ keys = 7 })` 꼴이다.

`spec` 에서 정해지는 것

| 키 수 | type | 레인 배열(스킨 레인 인덱스 순) | 레인 폭 | 필드 폭 | 스크래치 |
| --- | --- | --- | --- | --- | --- |
| 7 | 0 | 건반 1~7, 스크래치(인덱스 7) | 흰 건반 58, 검은 건반 46, 스크래치 100, 사이 2 | 484 | 왼쪽 또는 오른쪽(옵션) |
| 5 | 1 | 건반 1~5, 스크래치(인덱스 5) | 같음 | 376 | 왼쪽 또는 오른쪽(옵션) |
| 9 | 4 | 버튼 1~9 | 버튼 62, 사이 2 | 574 | 없음 |

레인 인덱스와 타이머 번호는 [b3 §12] 의 표대로다: 5·7키는 건반이 1~, 스크래치가 0 이고 9키는 버튼이 1~9 다(`apps/rbms-player/src/skin_host/play_timers.rs` `lane_slots`). 화면에 놓는 순서는 스크래치 방향 옵션에 따라 `spec` 이 정하고, note 의 배열 순서는 언제나 레인 인덱스 순이다.

(가) 영역(7키, 필드 왼쪽 기준. 필드 오른쪽 옵션이면 모든 x 를 `1920 - x - w` 로 뒤집는다. 5키는 필드 폭만 376)

| 영역 | x | top | w | h | 내용 |
| --- | --- | --- | --- | --- | --- |
| 레인 | 64 | 0 | 484 | 840 | 레인 바탕(`panel`), 레인 사이 선, 마디선, 노트 |
| 판정선 | 64 | 840 | 484 | 8 | `accent`. offset 3(LIFT) |
| 건반 표시 | 64 | 848 | 484 | 64 | 레인별 사각형. KEYON 타이머에 밝아진다 |
| 게이지 | 64 | 928 | 484 | 24 | gauge 50칸(폭 400) + 숫자(107, 407) |
| 필드 발치 | 64 | 968 | 484 | 96 | 하이스피드(310, 311), 표시 시간(313), 레인 커버 값(14. op 270 일 때) |
| 진행 막대 | 556 | 0 | 4 | 840 | graph type 101 세로 |
| 점수 그래프(옵션 On) | 584 | 120 | 136 | 720 | 세로 막대 3개: 현재(110), 최고(112), 목표(114). 차이(108, 153) |
| 곡 머리 | 744 | 40 | 1112 | 56 | 제목(text 12), 난이도 이름·레벨. 코스 중이면 오른쪽 끝에 단계 표시(크기 24, `accent`, 오른쪽 정렬): "STAGE 1"~"STAGE 4"(op 280~283), 마지막 곡은 "FINAL STAGE"(op 289). 코스가 아니면 다섯 옵션이 모두 거짓이라 아무것도 그려지지 않는다(`skin_host/system.rs`) |
| BGA | 744 | 120 | 1112 | 626 | bga 객체(16:9 상자, 비율은 엔진 설정) |
| 점수 | 744 | 770 | 352 | 198 | EX SCORE(101), 비율(102, 103), MAX COMBO(105) |
| 판정 수 | 1128 | 770 | 352 | 198 | PG, GR, GD, BD, PR, MS 와 FAST/SLOW(110~114, 420, 410~419, 421, 422) |
| 타이밍 | 1512 | 770 | 344 | 198 | BPM(160, 91, 90), 남은 시간(163, 164), 판정 타이밍(12), hiterrorvisualizer, 노트 분포(judgegraph type 1 + bpmgraph) |

9키: 필드가 가운데 (673, 0, 574, 840). 왼쪽에 곡 머리 (64, 40, 576, 56. 코스 단계 표시 포함), BGA (64, 120, 576, 432), 점수 (64, 576, 576, 264). 오른쪽에 점수 그래프 (1280, 120, 136, 720), 판정 수 (1448, 120, 408, 330), 타이밍 (1448, 474, 408, 366). 게이지와 발치는 필드 아래 같은 top.

필드 안에 겹쳐 그리는 것(필드 x 기준)

| 요소 | 위치 | 내용 |
| --- | --- | --- |
| 판정 글자 + 콤보 | 필드 가운데, top 560, 글자 높이 64 | judge 객체. offsets { 3, 32 } |
| 판정 상세 | top 520 | FAST/SLOW 글자(op 1242, 1243) 또는 ms 숫자(number 525). 옵션 904~906. offsets { 3, 33 } |
| 폭발 | 레인 가운데, 판정선 중심, 128x128 | BOMB 타이머(50~59), HOLD 타이머(70~79) 가산 블렌드. offset 3 |
| 키 빔 | 레인 폭, 판정선 위로 280 | KEYON 타이머(100~109). offset 3 |
| 레인 커버 | 필드 폭, 높이 840 | slider type 4(아래로, range 840). `raised` 면 |
| 리프트·히든 커버 | 필드 폭 | liftCover, hiddenCover |
| 풀 콤보 | 필드 가운데 | "FULL COMBO" 글자, 타이머 48 |
| 로딩 | 필드 가운데, top 400 | "LOADING" + graph type 102(op 80). READY(타이머 40)에 "READY" |
| 폐점 | 화면 전체 | 타이머 3: `bg` 덮개 알파 0 → 200(400ms), "FAILED"(크기 96, POOR 색). `close` 1500 안에 끝난다 |
| 닫기 | 화면 전체 | 타이머 2: `parts.fade_to_black(0, 500)` |

(나) 객체

| 객체 | 종류 | 주요 필드 | 자산 |
| --- | --- | --- | --- |
| 노트 필드 | note | `note`, `lnstart`, `lnend`, `lnbody`, `lnactive`, `hcnstart`, `hcnend`, `hcnbody`, `hcnactive`, `hcndamage`, `hcnreactive`, `mine` 을 레인 수만큼(구식 명명으로 통일. LN 10종 배열은 전부 `lnend` 길이 이상이어야 한다[b3 §3.2]). `size` 는 레인마다 16(9키 24). `dst` 는 레인 사각형. `group` 은 마디선(2px, `line`). destination 에 `offset = 30` | `play` 의 노트 |
| 9키 전용 | note | `dst2`(놓친 노트가 떨어지는 하한 = 건반 표시의 아래 변). `expansionrate` 는 기본 { 100, 100 } | - |
| 게이지 | gauge | `nodes` 6종 x 종류(36칸 규칙[b3 §5]), `parts = 50`, `type = 0`, `range = 3`, `cycle = 33` | `play` 의 게이지 칸 |
| 판정 | judge(index 0) | `images` 6개(PG, GR, GD, BD, PR, MS. 타이머 46, `loop = -1`), `numbers` 6개(콤보), `shift = true` | `play` 의 판정 글자, 콤보 숫자 |
| BGA | bga | - | - |
| 커버 | slider(type 4), liftCover, hiddenCover | `disapearLine` = 판정선 | `img.block` |
| 폭발, 키 빔 | image(타이머 50~59, 70~79, 100~109) | 폭발 `cycle = 250`, blend 2 | `play` |
| 그래프 | graph(type 101, 102, 110, 112, 114), judgegraph, bpmgraph, hiterrorvisualizer | - | `img.block` |
| 숫자 | value | 위 표의 ref | `common`, `play` |
| 글자 | text(ref 12), 난이도·키 수 부품 | - | 폰트 |

(다) 애니메이션: 장면 시작 덮개 300ms. PRELOAD 동안 로딩 막대. READY(타이머 40)에 판정선이 가운데에서 양쪽으로 400ms 펼쳐지고 "READY" 가 600ms 뒤 사라진다. 판정 글자는 타이머 46 에 60ms 동안 1.15배에서 1배로 줄고 500ms 에 사라진다. 리듬 타이머(140)에 판정선 위 띠가 알파 120 → 0. 게이지 최대(타이머 44)에 게이지 숫자가 `accent`.

(라) 입력: 클릭 가능한 객체 없음. 레인 커버·하이스피드 조작은 엔진이 한다.

(마) 뺀 것: 상세 모드, 공격 모션, 구역 점수와 음성, 캐릭터, 점수 플랩, 레인 커버 로테이션, 이력 기록, BGA 배치 변종과 동영상 배경, 노트·판정·폭발의 변종, 스크래치 회전판 그림(평면 사각형으로 대신한다), 타이밍 조정 클릭 영역, 리프트 표시 토글.

### 6.4 플레이 DP(type 2, 3)

같은 `play.build(spec)` 를 `{ keys = 14 }`, `{ keys = 10 }` 으로 부른다. 필드가 둘이고 가운데에 정보가 놓인다.

| 영역 | x | top | w | h | 내용 |
| --- | --- | --- | --- | --- | --- |
| 1P 필드 | 64 | 0 | 484(10키 376) | 840 | 스크래치가 왼쪽 바깥 |
| 2P 필드 | 1372(10키 1480) | 0 | 484(10키 376) | 840 | 스크래치가 오른쪽 바깥 |
| 판정선, 건반 표시 | 각 필드 | 840, 848 | 필드 폭 | 8, 64 | SP 와 같음 |
| 곡 머리 | 580 | 40 | 760 | 56 | SP 와 같음(코스 단계 표시 포함) |
| BGA | 580 | 120 | 760 | 428 | - |
| 게이지 | 580 | 572 | 760 | 24 | 하나뿐이다(2P 게이지 없음[b3 §12]) |
| 점수 | 580 | 620 | 368 | 220 | - |
| 판정 수 | 972 | 620 | 368 | 220 | - |
| 타이밍·분포 | 580 | 864 | 760 | 152 | - |

SP 와 다른 점

| 항목 | DP |
| --- | --- |
| note | 한 객체에 전 레인. 14키는 [1P 건반 1~7, 1P 스크래치, 2P 건반 1~7, 2P 스크래치] 16칸, 10키는 12칸. `group` 은 필드 수만큼 2개 |
| judge | index 0(타이머 46), index 1(타이머 47) 두 객체. 필드마다 하나 |
| 타이머 | 2P 쪽은 60~69(폭발), 80~89(홀드), 110~119(키 온) |
| 커버 | 같은 slider·커버 정의를 두 필드에 한 번씩 배치 |
| 옵션 | Field Side, Scratch Side, Score Graph 없음 |

### 6.5 결과(type 7)

구현 위치: `result.luaskin`, `lua/result/main.lua`(`result.build({ course = false })`).

(가) 영역

| 영역 | x | top | w | h | 내용 |
| --- | --- | --- | --- | --- | --- |
| 머리 | 64 | 70 | 1792 | 36 | 캡션 "RESULT". 오른쪽 끝에 "STAGE CLEARED"(op 90, `accent`) 또는 "STAGE FAILED"(op 91, POOR 색), 크기 24 |
| 곡 머리 | 64 | 136 | 1032 | 160 | 난이도 막대 + 제목(64), 아티스트, 난이도 이름·레벨·키 수 |
| 랭크 | 64 | 328 | 424 | 318 | 랭크 글자(text, 크기 240, op 300~307), 클리어 이름(number 370 의 값으로 고르는 Lua 값 함수, 램프 색), 갱신 표지 |
| 점수 | 520 | 328 | 576 | 318 | EX SCORE(171) + 차이(172, 부호 숫자) + 이전 최고(170), RATE(102, 103), MAX COMBO(174) + 차이(175), MISS COUNT(177) + 차이(178), 목표 차이(153) |
| 판정 표 | 64 | 678 | 1032 | 290 | 6행(PG, GR, GD, BD, PR, MS) x 3열(합계, FAST, SLOW) + 가로 막대. FAST/SLOW 합계(423, 424), 평균 타이밍(374, 375), 표준편차(376, 377). 막대는 graph 의 `value` 에 Lua 함수(판정 수 / 총 노트 수)를 준다. 비율 type 140~144 는 선곡 전용이라 플레이·결과에서는 0 이다(`skin_host/score.rs` `rate`, [b5 §5]) |
| 그래프 열 | 1128 | 136 | 728 | 832 | 위에서부터: 게이지 그래프 (1128, 176, 728, 300) gaugegraph + 게이지 값(107, 407), 판정 추이 (1128, 516, 728, 130) judgegraph type 1, FAST/SLOW 추이 (1128, 676, 728, 130) judgegraph type 2, 타이밍 분포 (1128, 836, 728, 130) timingdistributiongraph. 각 그래프 위에 캡션 |
| 바닥 | 64 | 1022 | 1792 | 18 | 안내 "ENTER  NEXT        R  RETRY        N  NEXT CHART" |

(나) 객체: 종류는 image(색 사각형), text, value(`common` 의 `num*`, `snum*`), graph, gaugegraph(색 14개를 3절 팔레트로 지정), judgegraph, timingdistributiongraph, bpmgraph. 갱신 표지는 op 330(점수), 331(콤보), 332(미스)에 "NEW RECORD"(`accent`). 리플레이 표지는 슬롯 1 의 상태다: "REPLAY SAVED"(op 198, `accent`), "REPLAY STORED"(op 197, `sub`), 없으면(op 196) 그리지 않는다. 결과 화면의 슬롯 2~4 는 엔진이 언제나 "없음"으로 답하므로 표지를 두지 않는다: R-BMS 는 런이 끝나면 리플레이를 스스로 저장하고 슬롯 1 이 그 런의 리플레이를 알린다(`apps/rbms-player/src/stage/result.rs` 머리말과 `scene_facts`).

(다) 애니메이션: 장면 시작 덮개 300ms. 곡 머리 → 랭크 → 점수 → 판정 표 순으로 100ms 간격 등장. 랭크 글자는 타이머 150(RESULTGRAPH_BEGIN)에 300ms 동안 알파 0 → 255. 판정 막대는 타이머 150 에 500ms 자람. 갱신 표지는 타이머 152 에 등장. FADEOUT 에 `parts.fade_to_black(0, 400)`.

(라) 입력: 클릭 가능한 객체 없음. 다음(건반 0~3, Enter), 게이지 종류 전환(건반 5), 다시 하기(R), 다음 곡(N)은 엔진이 한다[b4 §5.3, 사양 §4.5].

(마) 뺀 것: 클리어·실패 배경 그림과 캐릭터, 고리·빔 장식, IR 메뉴와 순위 목록, 인프레션 계산기, 플레이 이력, 난이도 목록 버튼, 메뉴 1·2 전환(전부 한 화면에 놓는다), 리플레이 슬롯 2~4 의 저장 표지와 저장 버튼(위 (나)의 이유), 시작 연출의 파도·띠.

### 6.6 코스 결과(type 15)

`courseresult.luaskin` 이 `result.build({ course = true })` 를 부른다. 결과와 같은 배치에서 아래만 다르다.

| 항목 | 코스 결과 |
| --- | --- |
| 머리 캡션 | "COURSE RESULT" |
| 곡 머리 | 코스 이름(text 10). 그 아래 단계 곡명(text 150~153)을 크기 24 로 4줄 |
| 게이지 그래프 | 단계별 이력을 이어 붙인 것(엔진이 공급[b4 웨이브 4 반영 사항]). 곡 경계는 엔진이 그린다 |
| 안내 | "ENTER  NEXT"(다시 하기·다음 곡 없음) |
| 차이 | 다시 하기 관련 표지 없음 |

## 7. 엔진 지원 확인표

"검증됨"은 ModernChic 으로 이미 그려 본 기능, "처음"은 기본 스킨이 처음 쓰는 기능이다.

| 기능 | 구현 위치 | 상태 |
| --- | --- | --- |
| `.luaskin` 2패스, 같은 Lua 상태 | `crates/rbms-skin/src/loader/lua_skin.rs` | 검증됨 |
| `require("lua.x.y")` 를 팩 루트 아래에서만 해석, 캐시 | `crates/rbms-skin/src/lua/package.rs` | 검증됨. 이번 덤프로 재확인 |
| 본체 패스의 `main_state.number`·`text` | `crates/rbms-skin/src/lua/main_state.rs` | 검증됨. 결정 화면이 사용 |
| 헤더 `property` 와 op 900~ 의 정적 평가 | `crates/rbms-skin/src/loader.rs` `merge_header`, `assemble` | 검증됨 |
| 스킨 폴더의 TTF, 높이 = em, 정렬, overflow 1 | `loader.rs` 폰트 해석, `crates/rbms-render/src/skin_render/text.rs` | 검증됨. Inter 로는 처음: 캡처로 확인 |
| 색 입힌 `-111`·`-110`, 알파·색 보간 | `skin_render/refs.rs`, `crates/rbms-skin/src/dst.rs` | 검증됨 |
| `-100` 스테이지 파일과 stretch 1 | `skin_render/refs.rs`, `loader/stretch.rs` | 검증됨(stretch 1 은 처음: 4:3 그림을 4:3 상자에 넣는 캡처로만 확인. 다른 비율은 선곡 단위가 확인) |
| op 190·191(스테이지 그림 유무) | `apps/rbms-player/src/skin_host/chart.rs` `read_boolean` | 결정·플레이·결과에서는 답한다(결정 캡처로 확인). **선곡에서는 답하지 않는다**: 커서 곡의 `ChartMeta` 가 `stagefile: None` 이라(`stage/select/list.rs` `chart_under_cursor`) 190 과 191 이 둘 다 거짓으로 읽힌다(`crates/rbms-skin/src/dst.rs` `condition_holds`). 그림 텍스처 자체는 선곡에서도 공급된다(`stage/select/images.rs`). 그래서 선곡은 이 둘을 조건으로 쓰지 않는다(6.2절) |
| 리플레이 슬롯 4개 | `skin_host/select.rs` `REPLAY_OPTIONS`(유무 197/1197/1200/1203, 없음 196/1196/1199/1202, 선택 1205~1208), `stage/select/events.rs`(act 19, 316~318 → 그 슬롯 재생), `skin_host/result.rs` `REPLAY_OPTIONS`(결과: 슬롯 1 만 상태가 바뀐다) | 엔진 테스트로 검증됨. 기본 스킨은 선곡·결과 단위가 처음 쓴다 |
| value 의 `value` 에 Lua 함수(3.3절의 `saturate`) | `skin_render/object.rs` `ValueSource::Function`, `draw.rs` `number_value` | 검증됨. 시험 팩으로 재확인(아래) |
| 코스 단계 op 280~283, 289, 290 | `skin_host/system.rs`(시스템 클러스터가 답한다) | 엔진 테스트로 검증됨. 기본 스킨은 플레이 단위가 처음 쓴다 |
| value 10칸 시트, 왼쪽 정렬, 색, `filter = 1` | `skin_render/object.rs`, `draw.rs` | 검증됨 |
| value 24칸 부호 시트 | 같음 | 검증됨. 기본 스킨의 `snum*` 는 결과 단위가 처음 쓴다 |
| judgegraph type 0, bpmgraph(색 지정) | `skin_render/graphs/notes_dist.rs`, `bpm.rs` | 검증됨 |
| 타이머 1·2, `loop`(멈춤, -1), `acc` | `dst.rs` `resolve` | 검증됨 |
| text 의 `value` 에 Lua 함수, `event_index` | `text.rs`, `main_state.rs` | 검증됨. 기본 스킨은 선곡 단위가 처음 쓴다 |
| image 의 `act`(id, Lua 함수), `mouseRect`, 슬라이더 드래그 | `skin_render/input.rs`, `dst.rs` | 검증됨 |
| songlist | `skin_render/songlist/` | 검증됨 |
| note, gauge, judge, bga, 커버, 오프셋 3·4·5·30·32·33 | `skin_render/notes.rs`, `gauge.rs`, `judge.rs`, `bga.rs`, `covers.rs` | 5/7/10/14키 검증됨 |
| gaugegraph, judgegraph type 1·2, timingdistributiongraph | `skin_render/graphs/` | 검증됨 |
| **9키 문서(type 4)** | `loader.rs` `PLAY_SKIN_MODES`, `apps/rbms-player/src/stage/play/mod.rs`(`mode_skin_type(shared.mode)`), `skin_host/play_timers.rs` `POPN_9K_KEYS` | **처음. 로드만 확인**(아래) |
| **note 의 `dst2`, `expansionrate`** | `skin_render/notes.rs` `fall_to`, `swell` | **처음. 단위 테스트만 있고 실제 스킨으로 그려 본 적 없다** |
| 스킨 사운드 `main_state.audio_play("sound/*.wav")` | `apps/rbms-player/src/skin_host/audio.rs` | ogg 로 검증됨. wav 는 사이드 메뉴 단위가 확인 |

처음 쓰이는 기능의 시험 결과(시험 팩은 저장소 밖 임시 폴더에 두었다)

| 시험 | 방법 | 결과 |
| --- | --- | --- |
| 문서 type 과 화면 대응 | type 6 문서와 type 4 문서를 한 팩에 두고 `rbms-cli skin-dump <팩> --strict --frames 30` | type 6 은 "decide", type 4 는 "play 9 keys" 로 읽힘. 둘 다 load ok, pcall 0, 경고 0 |
| 9키 로드 | type 4 문서에 9레인 note(LN·HCN·지뢰 배열 포함), gauge, judge 를 넣어 덤프 | 자동 오프셋 4개가 붙고 `image 1, note 1, gauge 1, judge 1` 로 조립됨. 경고 0 |
| 9키 그리기 | - | **확인하지 못했다.** 캡처 하니스에 9키 런이 없다(아래 보완 1) |
| 폰트 경로 | `font/Inter-Regular.ttf` 를 선언한 text 와, 선언하지 않은 폰트 id 를 쓰는 text 를 결정 장면 캡처로 비교 | 둘 다 그려진다. 선언한 쪽만 정식 경로다(4.6절). 선언하지 않은 쪽에 덤프 경고는 없다(아래 보완 3) |
| `require` 와 Lua 5.2 | 모듈이 헤더 패스에서 한 번만 실행되는지(`skin_config == nil` 을 모듈 최상위에서 기록), `7 / 2`, `_VERSION` 을 화면에 찍음 | `true`, `3.5`, `Lua 5.2` |
| 공통 모듈 | `ui`·`motion`·`parts` 의 좌표 변환, 정렬, 키프레임, 호버를 Lua 단언으로 검사하는 문서를 덤프 | load ok(단언 18개 통과) |
| 숫자의 자릿수 규칙 | 팩 사본에 `saturate`, `known`, `fit`, 오른쪽 정렬을 쓰는 문서를 두고 최대 BPM 655350, 레벨 123 인 시나리오로 덤프와 캡처 | 4자리 `saturate` 는 "9999"(붙이지 않으면 "5350"), 8자리는 "655350", 부호 띠 3자리는 "+99", 5자리는 "+123". 왼쪽 정렬 잉크는 안내선의 x 에서, `known` 오른쪽 정렬 잉크는 상자 오른쪽 끝에서 끝난다. 함수 값 8개, 실패 0, pcall 0 |

엔진·하니스 보완 필요(이 단위에서는 고치지 않았다)

| # | 내용 | 재현 | 제안 |
| --- | --- | --- | --- |
| 1 | 앱 캡처 하니스에 9키 플레이 런이 없다. `the_play_scene_of_a_skin_pack_named_by_the_environment_is_captured_through_its_states` 의 `runs` 는 5·7·10·14키뿐이다 | `apps/rbms-player/src/stage/capture.rs` 의 `runs` 배열 | 플레이 단위를 시작하기 전에 9키 차트 런을 추가한다(`mode_chart` 에 9버튼 채널 11~15, 22~25). 테스트 코드라 엔진 동작은 바뀌지 않는다 |
| 2 | 같은 테스트가 5키 샘플 런부터 돌고 문서가 하나라도 없으면 그 자리에서 실패한다. 팩 전체 캡처(`a_skin_pack_..._one_document_a_screen`)도 선곡·키 설정·7키 중 하나가 있어야 통과한다 | 결정 문서만 있는 지금의 팩으로 두 테스트를 돌리면 실패 | 플레이 단위는 다섯 문서를 먼저 얇게 만들어 두고 시작한다(9절) |
| 3 | `crates/rbms-render/tests/skin_external.rs` 는 진입 파일 이름이 ModernChic 의 것으로 고정돼 있다(`decide.luaskin`, `result.luaskin`, `course.luaskin`, `musicselect.luaskin`, `play7_hw.luaskin` 등). 기본 스킨에는 `decide` 다음 샷에서 실패한다 | `RBMS_SKIN_PACK=assets/skins/rbms-default cargo test -p rbms-render --test skin_external` | W8-5(테스트 재편)에서 헤더 type 으로 문서를 찾게 일반화하거나 기본 스킨 전용 게이트 테스트를 둔다. 그때까지 기본 스킨의 화면 확인은 앱 캡처 하니스로 한다 |
| 4 | 선언되지 않은 폰트 id 를 쓰는 text 가 경고 없이 대체 글꼴로 그려진다 | text 에 `font = 9`(미선언)를 주고 덤프: 경고 0 | 로더가 경고를 남기게 한다(낮은 우선순위) |
| 5 | 선곡에서 op 190·191 이 둘 다 거짓이다(위 표). 스킨이 선곡에서 "그림 없음" 문구를 낼 수 없다 | 선곡 문서에 `op = { 191 }` 인 -100 을 두면 그림이 있는 곡에서도 그려지지 않는다 | 기본 스킨은 쓰지 않으므로 막히지 않는다. 필요해지면 선곡이 로드한 그림의 유무로 190·191 을 답하는 엔진 단위를 따로 둔다 |

## 8. 검증 방법

### 8.1 덤프

```
cargo run -p rbms-cli -- skin-dump assets/skins/rbms-default --strict --frames 120
```

옵션은 `apps/rbms-cli/src/skin_dump/args.rs`: `--scenario <json>`, `--overlay <폴더>`, `--frames <N>`, `--seed <n>`, `--json`, `--probe-budget`, `--strict`, `--allow-failure <문서>`. `--strict` 는 로드 실패, 스킨의 pcall 이 삼킨 오류, 함수 값 실패, 프레임 예산 초과가 하나라도 있으면 실패한다(`skin_dump/gate.rs`). 경고는 실패가 아니므로 눈으로 0 인지 본다.

기대: 문서마다 `load ok`, `warnings 0`, `caught by the skin's own pcall: 0`, `function failures: 0`, 팩 폴더 `unchanged`.

### 8.2 캡처

```
RBMS_SKIN_PACK=$PWD/assets/skins/rbms-default RBMS_SKIN_CAPTURE_DIR=<폴더> \
  cargo test -p rbms-player <테스트 이름> -- --nocapture
```

`RBMS_SKIN_PACK` 은 절대 경로로 준다. 테스트는 `apps/rbms-player/src/stage/capture.rs` 에 있고 헤더 type 으로 문서를 찾는다.

| 화면 | 테스트 이름(앞부분) | 저장되는 프레임 |
| --- | --- | --- |
| 결정 | `the_decide_scene_of_a_skin_pack_named_by_the_environment_is_captured_from` | `decide-0000ms` ~ `decide-3950ms` 7장(샘플 5키 차트), `decide-rich-1500ms`, `decide-rich-3750ms`(스테이지 그림·변속·난이도 있는 차트) |
| 선곡 | `the_browser_of_a_skin_pack_named_by_the_environment_is_captured_over` | 곡 목록, 루트, 표 안, 코스 탭, 패널 1~3 |
| 선곡 위 오버레이 | `the_overlays_the_application_draws_over_the_browser` | 시스템 오버레이 |
| 플레이 | `the_play_scene_of_a_skin_pack_named_by_the_environment_is_captured_through` | 로딩, 준비, 플레이, 풀 콤보, 종료, 페이드, 폐점(7키), 5·10·14키 플레이 중 |
| 결과 | `the_result_scene_of_a_skin_pack_named_by_the_environment_is_captured_for` | 클리어와 실패 |
| 코스 결과 | `the_course_result_scene_of_a_skin_pack_named_by_the_environment_is_captured_for` | 클리어와 실패 |

`cargo test -p rbms-render --test skin_external` 은 기본 스킨에 쓸 수 없다(7절 보완 3).

### 8.3 화면별 확인 항목

| 화면 | 캡처를 열어 볼 것 |
| --- | --- |
| 공통 | 글자가 읽히는가(최소 18). 요소가 겹치거나 잘리지 않는가. 값의 기준선이 맞는가. 등장·퇴장이 3.5절대로인가. 마지막 페이드 프레임이 `bg` 단색인가. 자릿수 상한을 넘는 값에서 아래 자리만 나오지 않는가(3.3절. 앱 캡처의 샘플 차트로는 볼 수 없으므로 시나리오 값을 바꿔 그린다) |
| 결정 | 0ms 에 머리와 구분선만, 1500ms 에 전부, 3500ms 에 항목 구분선 자리의 `accent` 선과 옅은 덮개, 3950ms 에 단색. 긴 제목이 스테이지 그림(x 1280)을 넘지 않는가. 변속 곡에서 최소·줄표·최대가 붙어 있는가. 숫자의 잉크가 항목 이름의 x 에서 시작하는가 |
| 선곡 | 가운데 막대가 top 508 에 있는가. 막대 종류·램프·레벨·라벨이 6.2절대로인가. 폴더·코스 막대에서 정보 영역이 바뀌는가. 패널 1~3 의 칸이 건반 순서와 맞는가 |
| 플레이 | 노트 아래 변이 판정선에 닿는가. 레인 수와 순서(스크래치 방향 옵션 포함). 판정 글자와 콤보가 필드 가운데인가. 게이지 50칸. DP 에서 두 필드가 대칭인가. 9키에서 9레인이 가운데인가 |
| 결과 | 클리어와 실패의 머리 글자·색. 랭크, 점수 차이의 부호, 판정 표 18칸, 그래프 4개 |

### 8.4 이 단위에서 실제로 확인한 것

| 확인 | 결과 |
| --- | --- |
| 덤프(`--strict --frames 120`) | `decide.luaskin`: type 6, load ok, destination 59(정적 조건 제거 뒤 43), 함수 값 0, pcall 0, 경고 0, 팩 폴더 unchanged |
| 결정 캡처 9장 | 0ms: 캡션과 두 구분선만. 500ms: 글줄이 들어오는 중, 항목 3개가 반투명. 1500ms·3000ms: 전부 보임, 진행선이 47%·94%. 3500ms: 항목 구분선 자리의 `accent` 선과 20% 덮개(선이 글줄과 스테이지 그림을 지나지 않는다). 3750ms: 70% 덮개. 3950ms: 단색. 변속 차트(rich): "12 ANOTHER", "75 – 300", "1020", "NORMAL", "7 KEYS", 스테이지 그림, 노트 분포와 BPM 선. 겹침·잘림 없음 |
| 숫자 정렬(rich 1500ms 실측) | LEVEL 이름 x 64, 숫자 "12" 잉크 x 64. BPM 이름 520, 숫자 520. NOTES 이름 976, 숫자 976. 숫자 기준선 835~836 |
| 자릿수 초과(렌더 하니스 사본에 시나리오 값만 바꿔 그림) | 레벨 123, BPM 1 – 655350, 노트 123456: 전부 온전히 그려지고 레벨만 칸의 절반에 맞게 줄었다. 레벨 1234, BPM 12345678 – 99999999, 노트 123456789: 전부 온전하고 각자 칸 안에 줄어 들어간다(BPM 의 두 값은 같은 크기) |
| Stage Image Hide(팩 사본의 기본값만 Hide 로 바꿔 그림) | 87자 제목이 x 1856 까지 쓰인다(Show 에서는 x 1216 까지). 스테이지 면과 문구는 그려지지 않는다 |
| 생성 스크립트 재현성 | 사본에서 생성물 4개를 지우고 다시 만든 PNG 2개와 아틀라스 2개가 저장소의 것과 바이트가 같음. `--check` 종료 코드 0. 실행 뒤 `tools/` 에 `__pycache__` 가 생기지 않음 |
| 용량 | `image/` 55,355 B, `font/` 868 KiB, `sound/` 572 KiB |

## 9. 다음 단위 분할 제안

공통 파일은 동결한다: `lua/theme.lua`, `lua/props.lua`, `lua/ui.lua`, `lua/motion.lua`, `lua/parts.lua`, `lua/atlas/common.lua`, `image/common.png`, `font/**`, `tools/generate-images.py`, `tools/sprites.py`, `tools/sections/common.py`, 그리고 이 설계서. 화면 단위는 이 파일들을 고치지 않는다. 공통에 없는 것이 필요하면 자기 화면 모듈에 두고 보고에 적는다. 두 화면 이상이 같은 것을 만들었으면 메인이 통합 뒤 공통으로 올린다.

| 단위 | 내용 | 소유 파일 | 선행 | 캡처 |
| --- | --- | --- | --- | --- |
| W8-2b 선곡 | 6.2절 전부(곡 목록, 곡 정보, 필터 버튼, 리플레이 슬롯, 사이드 메뉴 2패널, 옵션 시트 3종) | `select.luaskin`, `lua/select/**`, `tools/sections/select.py`, `image/select.png`, `lua/atlas/select.lua` | W8-2a | 선곡, 선곡 위 오버레이 |
| W8-2c 플레이 | 6.3·6.4절. `lua/play/main.lua` 하나와 문서 5개 | `play5.luaskin`, `play7.luaskin`, `play9.luaskin`, `play10.luaskin`, `play14.luaskin`, `lua/play/**`, `tools/sections/play.py`, `image/play.png`, `lua/atlas/play.lua` | W8-2a, 캡처 하니스의 9키 런(7절 보완 1) | 플레이 |
| W8-2d 결과 | 6.5·6.6절. `lua/result/main.lua` 하나와 문서 2개 | `result.luaskin`, `courseresult.luaskin`, `lua/result/**`, `tools/sections/result.py`, `image/result.png`, `lua/atlas/result.lua` | W8-2a | 결과, 코스 결과 |

병렬 규칙
- 세 단위는 서로 파일이 겹치지 않는다(사양 §6 공통 규칙). 생성 스크립트는 구역마다 파일이 따로이고(`tools/sections/<구역>.py`), 진입점의 등록표에는 세 구역이 이미 올라 있어 고칠 일이 없다. 이 설계서도 화면 단위가 고치지 않는다.
- 생성은 자기 구역만 한다: `uv run tools/generate-images.py select`. 다른 구역의 PNG 는 건드리지 않는다.
- 플레이는 한 모듈을 다섯 문서가 쓰므로 SP 와 DP 를 나누지 않는다. 나눠야 하면 직렬로 한다(7키 → 5·9키 → 10·14키). 캡처 테스트가 문서 누락에서 멈추므로 다섯 문서를 먼저 얇게 만들어 둔다(7절 보완 2).
- 각 단위의 끝: 덤프 `--strict` 통과, 자기 화면 캡처를 직접 열어 8.3절 항목 확인, 자기 구역의 생성 스크립트 `--check` 통과. 보고에 최종 좌표(6절의 자기 화면 표와 달라진 것 전부), 5.2절에 적을 시트 크기와 용량, 공통에 없어 자기 모듈에 둔 것을 적는다.
- 통합 뒤 메인: 단위 보고의 좌표를 이 문서 6절과 5.2절에 반영, 덤프 전체, 캡처 전체, `du -sh image`, 6 MiB 예산 확인. 그 뒤 W8-3(내장·설치)으로 간다. 내장 표는 `font/`, `image/`, `lua/`, `sound/`, `*.luaskin` 을 담고 `tools/` 는 담지 않는다.
