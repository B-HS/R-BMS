# 기본 스킨(rbms-default) 설계서

> 최종 갱신 2026-10-12 · 상태: 전 화면 구현 완료(문서 9개: 결정, 선곡, 플레이 5/7/9/10/14키, 결과, 코스 결과)와 통합 반영(공통 승격, 6절의 최종 좌표, 통합에서 고친 엔진 결함 2건), 웨이브 9A 반영(판정 양식을 IIDX 식으로, 램프·난이도 색과 클리어 표기를 ModernChic 의 값으로, FAST/SLOW 기본 켬. 3.1절, 6.2~6.5절), 웨이브 10A 반영(Lane Width 옵션과 Wide 배치: 2.3절, 6.3 (가-2), 6.4절. 선곡 패널 3 의 NOTE DISPLAY TIME 칸, 플레이의 TIMING AUTO 표지, 안내 문구의 키 이름: 6.2, 6.3, 6.5절) · 대응 단위: 웨이브 8A W8-2a, 웨이브 8B W8-2b~2d 와 통합, 웨이브 10A 단위 A 와 통합, 웨이브 10C 단위 S 와 통합(정적 판정 줄, 판정 위 줄, Judge Position·Judge Display·Score Difference, 부위 불투명도 19개: 2.3절, 3.5절, 6.3·6.4절, 8.2~8.4절. 통합에서 반투명을 시트와 색에 굽고 건반 표시 바탕의 겹침을 없앴다), 웨이브 10D(판정 줄 다시 놓기: 콤보를 글자 바로 뒤에 왼쪽 정렬로, 옅은 0 삭제, 모든 판정 글자와 위 줄의 왼쪽 끝 통일, 다섯 자리가 필드 안에 드는 크기 규칙. 엔진의 judge `numberAlign`: 3.5절, 5.2·5.3절, 6.3·6.4절, 7절, 8.4절), 웨이브 13 단위 S(판정선을 필드의 맨 아래에: 건반 표시와 그 오프셋 105 삭제, 노트 영역 840 → 904, 필드의 아래 끝에서 끊기는 폭발: 2.3절, 5.2절, 6.3·6.4절, 8.2·8.4절), 웨이브 13 통합(선곡 패널 3 의 JUDGE TIMING 동작 문구: 6.2절. 판정 오프셋이 노트 필드만 옮기게 된 뒤의 캡처와 최종 확인: 8.4절), 웨이브 14 단위 T(모노톤 팔레트와 색 옵션 Theme·Accent·Note Colors·Bomb Color, 판정선 가운데의 온전한 폭발 고리: 2.3절, 3절 머리말과 3.1절, 4.1·4.2절, 5.1~5.3절, 6.3절, 8.2~8.4절)
> 상위 사양: `docs/plan/2026-10-09-lua-skin-compat.md` §2(D1·D4·E9), §4.7, §6 웨이브 8
> 조사 근거: `docs/reference/skin-compat/`(색인 `README.md`). `[b4 §4]` 같은 표기는 그 폴더의 보고서 절이다.
> 이 문서와 `assets/skins/rbms-default/` 만 보고 화면을 고칠 수 있게 쓴다. 6절의 좌표는 전부 구현된 최종값이다.

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
| 용량 | 이미지 합계 목표 6 MiB 이하. 내장 한도 10 MiB 에는 사운드 0.56 MiB 와 폰트 0.84 MiB 가 포함된다. 이미지 합계 120,733 B(118 KiB. 5.2절) |
| 시트 크기 | 한 변 4096 초과 금지(생성 스크립트가 거부한다) |
| 생성물 | PNG 와 아틀라스 Lua 는 저장소에 커밋한다. 빌드와 CI 는 uv 없이 돈다 |

## 2. 팩 구조

### 2.1 파일

| 경로(`assets/skins/rbms-default/` 기준) | 내용 | 상태 |
| --- | --- | --- |
| `decide.luaskin` | 결정 화면 문서(type 6) | 완성 |
| `select.luaskin` | 선곡 문서(type 5) | 완성 |
| `play7.luaskin`, `play5.luaskin`, `play9.luaskin` | 플레이 SP 문서(type 0, 1, 4) | 완성 |
| `play14.luaskin`, `play10.luaskin` | 플레이 DP 문서(type 2, 3) | 완성 |
| `result.luaskin`, `courseresult.luaskin` | 결과(type 7), 코스 결과(type 15) | 완성 |
| `lua/theme.lua`, `lua/props.lua`, `lua/ui.lua`, `lua/motion.lua`, `lua/parts.lua` | 공통 모듈(4절) | 완성(통합에서 공통 승격분 추가) |
| `lua/atlas/<구역>.lua` | 생성 스크립트가 쓰는 시트 좌표표(손으로 고치지 않는다) | 5구역 완성 |
| `lua/decide/main.lua` | 결정 화면 조립 | 완성 |
| `lua/select/{main,list,info,controls,side,sheet,kit}.lua` | 선곡 전용 모듈(6.2절) | 완성 |
| `lua/play/{main,layout,field,info}.lua` | 플레이 전용 모듈. 다섯 문서가 같이 쓴다(6.3절, 6.4절) | 완성 |
| `lua/result/{main,layout,score,judges,graphs}.lua` | 결과 전용 모듈. 결과와 코스 결과가 같이 쓴다(6.5절, 6.6절) | 완성 |
| `image/<구역>.png` | 생성된 시트. 구역 = common, decide, select, play, result | 5구역 완성 |
| `font/Inter-Regular.ttf`, `font/Inter-OFL.txt` | `assets/fonts/` 의 사본 | 완성 |
| `sound/*.wav`(22개), `tools/generate-sounds.py` | 기존 자작 사운드. 손대지 않는다 | 기존 |
| `tools/generate-images.py`, `tools/sprites.py` | 이미지·아틀라스 생성 스크립트의 진입점(배치·출력)과 공통 그리기 헬퍼(5.3절) | 완성 |
| `tools/sections/<구역>.py` | 구역별 스프라이트 목록. 구역 하나에 파일 하나 | 5구역 완성 |

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

공통: `w = 1920`, `h = 1080`, `author = "R-BMS"`, `name = "R-BMS Default (<화면>)"`. 시간 단위는 ms. 플레이의 자동 오프셋 4개(10 All offset(%), 30 Notes offset, 32 Judge offset, 33 Judge Detail offset)는 엔진이 헤더에 붙이므로 문서가 선언하지 않는다(`loader.rs` `AUTOMATIC_OFFSETS`). 플레이 문서가 스스로 선언하는 오프셋은 아래 "플레이 문서의 선언 오프셋" 표의 19개(DP 는 18개)이고 id 는 100~119 가운데 105 를 뺀 것이다(엔진이 쓰는 1~5, 10, 30, 32, 33, 50, 51 과 겹치지 않고 유효 범위 1~199 안이다). 사용자 옵션의 op 는 문서마다 900 부터 쓴다(문서 사이에 겹쳐도 된다). 색 옵션 넷(Theme, Accent, Note Colors, Bomb Color. 웨이브 14)은 그것을 가진 모든 문서에서 이름, 항목 이름, op 가 같다(920~926, 930~936. 아래 "색 옵션" 표). 파일 슬롯(filepath)은 어느 문서에도 없다(자산 변종을 두지 않는다).

| 문서 | input | scene | fadeout | 플레이 전용 | 사용자 옵션(property): 항목(op), 기본값 먼저 | 선언 오프셋 |
| --- | --- | --- | --- | --- | --- | --- |
| decide | 300 | 3200 | 800 | - | Stage Image: Show(900)/Hide(901). Note Graph: Show(902)/Hide(903). Theme, Accent | 없음 |
| select | 300 | 0 | 0 | - | Stage Image: Show(900)/Hide(901). 결정과 같은 옵션이고 정의는 `parts.stage_image` 한 곳에 있다. Theme, Accent | 없음 |
| play5, play7 | 500 | 0 | 600 | loadend 1500, playstart 1000, close 1500, finishmargin 1000, judgetimer 1 | Lane Width: Normal(909)/Wide(910). Field Side: Left(900)/Right(901). Scratch Side: Left(902)/Right(903). Judge Detail: Fast-Slow(905)/Milliseconds(906)/Off(904). Score Difference: Target(911)/Best(912)/Off(913). Judge Display: Show(914)/Hide(915). Score Graph: On(907)/Off(908). Note Colors, Bomb Color, Theme, Accent | 100~119 의 19개(105 없음) + 자동 4개 |
| play9 | 500 | 0 | 600 | 위와 같음 | Lane Width(909/910), Judge Detail(905/906/904), Score Difference(911/912/913), Judge Display(914/915), Score Graph(907/908), Bomb Color, Theme, Accent. 필드는 가운데 고정이고 Note Colors 는 없다(버튼 색은 모드가 정한다) | 같음 |
| play10, play14 | 500 | 0 | 600 | 위와 같음 | Lane Width(909/910), Judge Detail(905/906/904), Score Difference(911/912/913), Judge Display(914/915), Note Colors, Bomb Color, Theme, Accent. 스크래치는 바깥쪽 고정, 그래프 없음 | 114(Score Graph Opacity)를 뺀 18개 + 자동 4개 |
| result, courseresult | 500 | 3600000 | 500 | - | Theme, Accent | 없음 |

색 옵션(웨이브 14. 항목은 기본값 먼저. 색 값은 3.1절). Theme 과 Accent 의 정의는 `lua/theme.lua`(`theme.OPTION`, `theme.properties`), Note Colors 와 Bomb Color 는 `lua/play/layout.lua`(`layout.OPTION`, `layout.NOTE_SETS`)와 `lua/play/main.lua` 에 있다.

| 옵션 | 항목(op) | 가진 문서 | 바꾸는 것 |
| --- | --- | --- | --- |
| Theme | Mono(920) / Slate(921) | 9개 전부 | 바탕·면·선·글자의 회색 단계 한 벌(`theme.color` 의 `bg`, `panel`, `raised`, `line`, `text`, `sub`, `muted`)과 그 색을 구운 조각(선곡 막대, NO PLAY 램프, 게이지 칸). Slate 는 웨이브 13 까지의 청회색 팔레트 그대로다 |
| Accent | White(922) / Sky(923) / Amber(924) / Green(925) / Pink(926) | 9개 전부 | 강조색 한 쌍(`accent`, `accent_dim`)만. Theme 과 따로 논다 |
| Note Colors | White-Blue(930) / White-Deep Blue(931) / White-Yellow(932) / White-Green(933) / Mono(934) | play5, play7, play10, play14 | 검은 건반 노트의 색(Mono 는 스크래치까지). 노트, LN 몸통, 키 빔이 따라간다 |
| Bomb Color | Note(935) / White(936) | 플레이 5개 | 폭발의 색. Note 는 그 레인의 노트 색, White 는 모든 레인이 흰 건반 노트의 색 |

- 행의 순서: 문서가 원래 갖고 있던 행 뒤에 Note Colors, Bomb Color, Theme, Accent 순으로 붙는다. Theme 과 Accent 는 모든 문서의 마지막 두 행이다.
- 웨이브 13 까지의 화면과 같은 모습은 Theme = Slate, Accent = Sky 다(폭발 고리만 온전한 고리로 달라졌다. 8.4절에서 픽셀로 비교했다).
- 옵션은 문서마다 따로 저장된다(엔진의 `skin.custom`). 이름과 항목 이름을 문서 사이에 맞춘 것은 SKIN 메뉴가 같은 이름의 옵션을 팩의 모든 화면에 한 번에 적용할 수 있게 하기 위해서다(메뉴 쪽 기능은 웨이브 14 의 다른 단위).

플레이 문서의 선언 오프셋(웨이브 10C. `lua/play/layout.lua` 의 `JUDGE_POSITION`, `PART`, `PART_ROWS`. 헤더에는 이 순서로 놓이고 자동 4개가 그 뒤에 붙는다)

| id | 이름 | 축 | 걸리는 것(6.3절의 부위 표) |
| --- | --- | --- | --- |
| 100 | Judge Position | y | 판정 글자·콤보와 그 위 줄(점수 차, FAST/SLOW 또는 ms). 값은 화면 픽셀이고 +면 위, -면 아래 |
| 101 | Lane Background Opacity | a | 레인 면 |
| 102 | Lane Divider Opacity | a | 레인 사이의 2px 선 |
| 103 | Judge Line Opacity | a | 판정선과 그 연출(펼쳐지는 선, 리듬 띠) |
| 104 | Key Beam Opacity | a | 건반을 누를 때 레인에 서는 빛 기둥 |
| 106 | Bomb Opacity | a | 폭발 |
| 107 | Note Opacity | a | 노트·LN·지뢰(note 객체) |
| 108 | Bar Line Opacity | a | 마디선 |
| 109 | Lane Cover Opacity | a | 레인 커버·히든·리프트의 면과 커버 끝의 강조선·수치 |
| 110 | Gauge Opacity | a | 게이지 막대와 수치 |
| 111 | Progress Bar Opacity | a | 필드 옆 진행 막대 |
| 112 | Song Header Opacity | a | 곡 머리 |
| 113 | BGA Opacity | a | BGA 면, BGA, 스테이지 그림, "NO BGA"·"BGA OFF" |
| 114 | Score Graph Opacity | a | 스코어 그래프(SP 문서만 선언한다) |
| 115 | Score Opacity | a | 점수 영역 |
| 116 | Judge Count Opacity | a | 판정 수 표 |
| 117 | Timing Opacity | a | BPM·남은 시간·TIMING 과 히트 에러 띠 |
| 118 | Distribution Graph Opacity | a | 분포 그래프와 그 범례 |
| 119 | Speed Display Opacity | a | 발치의 HI-SPEED·GREEN·COVER·LIFT 와 AUTOPLAY·REPLAY 표지 |

- 불투명도 오프셋은 알파 축만 가진 오프셋이다. 사용자가 보는 값은 0~100%(기본 100)이고 저장되는 알파는 형식 계약대로 더해지는 값이다: 100% = 0, 0% = -255, 그 사이는 `round((pct - 100) * 255 / 100)`. 메뉴는 이름이 아니라 축 구성(알파뿐)으로 이 행을 알아보고 이름은 문서가 준 그대로 쓴다.
- 더해지는 값이므로 0% 와 100% 만 모든 것에 똑같이 듣는다. 그 사이에서는 알파 255 인 destination 이 pct% 가 된다. 그래서 부위의 destination 은 키프레임 알파를 255 로 두고 반투명은 그림과 색에 굽는다(웨이브 10C 통합): 키 빔의 진하기는 시트의 `beam`(최대 알파 118), 리듬 띠는 시트의 `pulse`(최대 알파 71)다. 처음에는 destination 알파(키 빔 200, 리듬 띠 120)로 냈는데, 같은 양이 빠져 50% 에서 키 빔이 약 35% 로 보이고 20% 이하에서 사라졌다. 한 부위 안에서 겹쳐 그리는 것도 중간값을 어긋나게 한다. 남은 어긋남은 판정선(어두운 바탕 선 위에 밝은 선이 겹쳐 50% 에서 약 58%)과 가산 합성인 폭발, 페이드 도중의 프레임(알파가 내려가는 키프레임에 같은 양이 더 빠져 일찍 0 에 닿는다)이다. 0% 에서는 페이드 중인 것까지 전부 그려지지 않는다(8.4절).
- 105 는 비어 있다(웨이브 13). 판정선 아래의 건반 표시(Key Display Opacity)가 쓰던 번호이고, 그 부위를 없앤 뒤에도 다른 부위에 다시 쓰지 않는다: 사용자의 설정 파일에 남아 있는 105 의 값이 다른 부위에 걸리면 안 되기 때문이다. 남은 값은 어느 destination 도 그 id 를 걸지 않으므로 읽히기만 하고 아무것도 바꾸지 않는다(`play_rows.rs` 의 게이트가 105 에 알파 -255 를 넣고 문서가 경고 없이 그려지는지 본다).
- 부위에 속하지 않는 것(항상 100%): 화면 바탕, LOADING·READY·FULL COMBO 알림, 폐점 덮개와 "FAILED", 열고 닫는 덮개. 판정 글자·콤보와 그 위 줄은 불투명도가 아니라 Judge Display 가 맡는다(엔진의 Judge offset·Judge Detail offset 의 알파 축은 그대로 듣는다).
- 한 destination 이 부위 하나에만 속한다. `ui` 의 `s:within(id, build)` 가 `build` 안에서 만들어지는 모든 destination(note 의 `group`, judge 의 `images`·`numbers` 같은 중첩 destination 포함)의 `offsets` 끝에 그 id 를 붙인다(4.3절).
- Judge Position 은 엔진의 자동 오프셋 대신 따로 선언했다. 자동 오프셋은 둘(32 는 판정 글자, 33 은 그 위 줄)이라 한 값으로 두 줄을 함께 옮길 수 없기 때문이다. 부호는 맞다(오프셋의 y 는 화면 위쪽이 +다). 32 와 33 도 전과 같이 걸려 있어 각 줄을 따로 미세 조정할 수 있고, 세 값은 더해진다. 리프트(3)도 전과 같이 판정 줄을 올린다.

근거와 주의
- 결정: `nowtime > input` 에 STARTINPUT(타이머 1), `nowtime > scene` 에 FADEOUT(타이머 2), FADEOUT 경과 > `fadeout` 에 플레이로 넘어간다[b4 §4, `apps/rbms-player/src/stage/decide.rs`]. scene + fadeout = 4000 은 캡처 하니스의 마지막 시각 3950ms 보다 길어야 해서 정했다(`stage/capture.rs` `DECIDE_SHOTS_MS`).
- 선곡은 `scene`·`fadeout` 을 쓰지 않는다. STARTINPUT 은 입력 잠금이 아니다[b4 §1.5, 웨이브 5 반영 사항].
- 플레이의 `loadend`·`playstart`·`close`·`finishmargin` 은 note 객체가 배치된 문서에서만 적용된다[b3 §2]. PRELOAD 는 `loadend` 와 무관하게 최소 1초다[b3 웨이브 6 반영 사항].
- 스크래치 방향과 필드 좌우처럼 앱 설정과 겹치는 옵션은 스킨 옵션이 우선한다(사양 §4.5).
- Lane Width 는 다섯 플레이 문서 모두의 첫 행이다(웨이브 10A). Wide 는 건반·스크래치·버튼 레인의 폭이 Normal 의 1.5 배이고(레인 사이 2px 은 그대로), 필드 밖 영역은 6.3·6.4절의 Wide 표대로 다시 놓인다. 내장 화면의 DISPLAY > SKIN 프리셋(NORMAL/WIDE)과는 다른 설정이고, 스킨이 그리는 플레이 화면의 레인 폭은 이 행만 바꾼다. 기본값 Normal 은 이 행이 생기기 전의 화면과 픽셀까지 같다(8.4절).
- Judge Detail 의 기본값은 Fast-Slow 다(웨이브 9A. 그 전에는 Off 여서 첫 실행에서 FAST/SLOW 가 한 번도 뜨지 않았다). 옵션을 한 번도 건드리지 않은 사용자는 저장된 값이 없으므로 새 기본값을 따르고, Off 를 고른 사용자의 선택은 그대로다. op 번호는 바꾸지 않았다.
- Score Difference 와 Judge Display 는 웨이브 10C 에서 더했다. Score Difference 는 판정 글자 위 줄의 왼쪽 자리에 무엇을 쓸지다: Target(기본)은 PLAY 메뉴의 TARGET 이 정한 목표 대비 EX 점수 차(number 108. 스코어 그래프의 TARGET 숫자와 같은 값), Best 는 자기 최고 기록 대비(152. 그래프의 BEST 숫자와 같은 값), Off 는 쓰지 않는다. Judge Detail 은 같은 줄의 오른쪽 자리다. Judge Display 가 Hide 면 judge 객체와 그 위 줄을 문서에 넣지 않는다(판정 글자·콤보·점수 차·FAST/SLOW 가 전부 그려지지 않는다). 행의 순서는 Lane Width, Field Side, Scratch Side, Judge Detail, Score Difference, Judge Display, Score Graph 다.

## 3. 시각 규칙

방향: 순수한 검정 바탕, 파란 기가 없는 회색 단계의 면과 선, 흰색과 회색의 글자, 흰색 강조(모노톤. 웨이브 14 의 사용자 요청 "배경색의 기본 컬러는 검은색, 전체적으로 검/회색/흰의 모노톤"). 직각 평면이다. 그림자·광택·그라디언트·둥근 모서리·배경 이미지를 쓰지 않는다. 면은 단색 사각형(`-111` 흰색 참조 이미지에 색을 입힌 것)으로, 위계는 글자 크기와 세 단계 글자색으로 만든다. 색을 가진 것은 뜻이 색에 실린 것(판정, 빠름·느림, 램프, 난이도, 게이지, 그래프)과 플레이의 노트·키 빔·폭발뿐이고, 모노톤 바탕 위에서 그것들만 색으로 읽힌다(아래 "색으로 두는 것" 표). 값의 정의는 `lua/theme.lua` 한 곳에 있고, 어느 벌을 쓸지는 옵션 Theme·Accent 가 본체 패스에서 정한다(2.3절, 4.1절).

### 3.1 색

바탕과 글자의 단계(`theme.mono`, `theme.slate`. 옵션 Theme 이 고른 벌이 `theme.color.*` 가 된다)

| 이름(`theme.color.*`) | Mono(기본) | Slate | 용도 |
| --- | --- | --- | --- |
| `bg` | `000000` | `0C0F14` | 화면 바탕, 전환 덮개, 레인의 어두운 면 |
| `panel` | `161616` | `141922` | 패널 면(그래프 바탕, 이미지 자리, 레인의 밝은 면) |
| `raised` | `262626` | `1D2430` | 떠 있는 면(선택된 막대, 옵션 시트, 사이드 패널, 커버) |
| `line` | `3C3C3C` | `2B3545` | 2px 구분선, 테두리, 게이지의 꺼진 칸 |
| `text` | `F2F2F2` | `F1F4F8` | 1차 글자(제목, 값) |
| `sub` | `B8B8B8` | `A7B2C1` | 2차 글자(부제, 장르) |
| `muted` | `808080` | `667387` | 3차 글자(항목 이름, 안내) |
| `bar` | `C8C8C8` | `5C73A0` | 선곡 막대의 반투명 면(알파 26, missing 13). 시트에만 굽는다 |
| `lamp_off` | `444444` | `3A4556` | NO PLAY 램프(꺼진 색). `theme.lamp.noplay` 가 이 표를 가리킨다 |

- Mono 의 값은 전부 R = G = B 이고 자체 값이다(D1: ModernChic 의 색 표를 옮기지 않았다). 단계는 밝기 0, 22, 38, 60(면과 선)과 128, 184, 242(글자)다. 바탕 대비 명도 비는 `text` 18.8, `sub` 10.6, `muted` 5.3 이고, 가장 불리한 조합인 `raised` 위의 `muted` 가 3.8, `panel` 위의 `muted` 가 4.6 이다(Slate 에서는 바탕 위 4.0, `raised` 위 3.2 였다). 면끼리의 명도 비는 바탕 대 `panel` 1.16, `panel` 대 `raised` 1.20, 바탕 대 `line` 1.90 이다(Slate 1.09, 1.13, 1.55).
- Slate 는 웨이브 13 까지의 값 그대로다. 지우지 않고 한 벌로 남겼다.

강조색(`theme.accent`, `theme.accent_dim`. 옵션 Accent 가 고른 쌍이 `theme.color.accent`, `accent_dim` 이 된다)

| Accent | `accent` | `accent_dim` | 비고 |
| --- | --- | --- | --- |
| White(기본) | `FFFFFF` | `666666` | 모노톤의 강조. `text` 와 거의 같은 밝기라 강조는 색이 아니라 꼴(띠, 밑줄, 선)로 읽힌다 |
| Sky | `4CC9F0` | `1F5F78` | 웨이브 13 까지의 강조색 |
| Amber | `F2B33D` | `614818` | `theme.tone.yellow` 와 같은 값 |
| Green | `59D185` | `245435` | `theme.tone.green` 과 같은 값 |
| Pink | `F0689E` | `602A3F` | |

`accent` 가 쓰이는 곳: 진행선(결정의 진행 막대와 닫는 선, 플레이의 진행 막대와 로딩 막대), 선택 표시(선곡 가운데 막대의 띠, 리플레이 밑줄, 켜진 탭, 즐겨찾기 별, 폴더·코스에서의 곡 정보 머리 막대), 포커스와 시트·사이드 패널의 윗선, 판정선과 리듬 띠, 레인 커버 끝의 선, 표 이름(결정), 단계 글자·AUTOPLAY·REPLAY·FULL COMBO, 게이지가 가득 찼을 때의 수치, 스코어 그래프의 자기 점수 막대, 결과의 STAGE CLEARED 글자와 NEW RECORD·SAVED·좋아진 점수 차, 옵션 시트의 KEY 번호, 사이드 패널의 값, LN·CN·HCN 라벨, BPM 선의 기준 BPM, 결과 그래프의 경계선과 평균선. `accent_dim` 은 판정선의 바탕 선 하나다.

노트의 색(`theme.note` 와 `theme.tone`. 옵션 Note Colors 가 고른다. 시트에 색째로 굽고, 키 빔과 폭발은 같은 값으로 물들인다)

| Note Colors | 흰 건반 | 검은 건반 | 스크래치 |
| --- | --- | --- | --- |
| White-Blue(기본) | `F1F4F8` | `4CC9F0` | `F0564C` |
| White-Deep Blue | `F1F4F8` | `568CF5` | `F0564C` |
| White-Yellow | `F1F4F8` | `F2B33D` | `F0564C` |
| White-Green | `F1F4F8` | `59D185` | `F0564C` |
| Mono | `F1F4F8` | `8C8C8C` | `C4C4C4` |

- 기본값은 웨이브 13 까지의 노트와 바이트까지 같은 조각이다(사용자: "노트와 bomb 은 지금 그대로 흰색 파란색이 맞다"). 그 전에는 흰 건반이 `theme.color.text`, 검은 건반이 `theme.color.accent` 를 빌려 썼는데, Theme·Accent 를 바꿔도 노트가 따라 바뀌지 않도록 `theme.note`(`white`, `cyan`, `gray`, `silver`)로 갈라냈다. LN 몸통과 노트 아래 그늘은 색을 `theme.note.shade`(`0C0F14`. 전과 같은 값) 쪽으로 섞어 굽고, 이 값은 Theme 을 타지 않는다.
- 9키의 버튼 색(흰색, 노랑, 초록, 파랑, 빨강)은 모드가 정한 것이라 옵션이 없다.
- Bomb Color = White 의 폭발은 모든 레인이 `F1F4F8` 이다.

색으로 두는 것(Theme·Accent 와 무관하게 색을 가진 것. 그 밖의 모든 것은 위의 단계와 강조색만 쓴다)

| 것 | 색의 출처 | 바꾸는 옵션 |
| --- | --- | --- |
| 노트, LN 몸통, 키 빔, 폭발 | `theme.note`, `theme.tone`(위 표) | Note Colors, Bomb Color |
| 지뢰와 MINE 라벨 | `theme.tone.rose`, `theme.tone.red` | 없음 |
| 필드의 판정 글자와 콤보, 판정 수 표의 이름, 히트 에러 띠, 결과의 판정 막대 | `theme.judge`, `theme.judge_word` | 없음 |
| FAST / SLOW, 판정 상세의 ms | `theme.timing` | 없음 |
| 클리어 램프(막대의 램프, 폴더 분포 띠와 칩, 클리어 이름) | `theme.lamp`(NO PLAY 만 Theme 의 `lamp_off`) | 없음 |
| 난이도(막대, 이름, 레벨 숫자) | `theme.difficulty` | 없음 |
| 게이지 칸(종류별), 결과 게이지 그래프의 선 | `theme.lamp` 의 다섯 색과 `theme.tone.red`. 꺼진 칸과 어두운 단계만 Theme 을 탄다 | 없음 |
| judgegraph, timingdistributiongraph 의 판정 색과 그 범례 칩 | 엔진 고정 색(`parts.judge_graph_legend`) | 없음 |
| 스코어 그래프의 막대 | 자기 점수 `accent`, BEST `sub`, TARGET `theme.tone.yellow` | Accent(자기 점수만) |
| GREEN 숫자, FAILED·STAGE FAILED, 나빠진 점수 차, BPM 선의 최소·최대·그 밖·정지 | `theme.tone` 의 green, red, orange, blue·red·yellow·violet | 없음 |
| 코스 트로피(동, 은, 금) | `lua/select/list.lua` 의 `TROPHY_COLORS` | 없음 |
| 스테이지 그림, BGA | 곡의 것 | 없음 |

의미색(강조색과 별개로, 뜻이 정해진 곳에만 쓴다)

| 묶음 | 값 |
| --- | --- |
| 난이도 `theme.difficulty[0..5]` | UNKNOWN `C3C3C3`, BEGINNER `06FF00`, NORMAL `12D2D7`, HYPER `FFC000`, ANOTHER `FF0000`, INSANE `A840AA`. ModernChic 의 난이도 기본색(`Root/customnumber.lua` `diffRGB`, `Play/lua/require/common.lua` `setRGB`)이다. INSANE 만 두 곳의 값이 다르고(148,44,150 과 168,64,170) 어두운 바탕에서 글자로 읽히는 플레이 쪽 값을 쓴다 |
| 판정 `theme.judge` | PGREAT `6BA6F5`, GREAT `F6AB22`, GOOD `F9B21F`, BAD `FB8116`, POOR `F25200`. 판정 수 표의 이름, 히트 에러 띠, 결과 화면이 쓴다. 값은 ModernChic 의 판정 시트(`Play/parts/common/judge/#default.png`. IIDX 양식)에서 글자 면의 평균색을 잰 것이다(PGREAT 와 POOR 는 면 전체, GREAT·GOOD·BAD 는 윗부분). GREAT 과 GOOD 이 거의 같은 금색인 것도 그 시트 그대로다. 여섯째 판정(헛침)은 색이 따로 없다: IIDX 처럼 POOR 다 |
| 필드의 판정 글자 `theme.judge_word` | PGREAT `84CEFF`(밝은 하늘색), GREAT `FFD633`(노랑). 필드 위의 판정 글자와 콤보 숫자에만 쓴다(웨이브 10C). 둘 다 "GREAT" 라고 쓰므로 색 하나로 갈려야 해서 `theme.judge` 의 값보다 밝고 서로 먼 자체 값을 골랐다. GOOD·BAD·POOR 의 필드 글자는 `theme.judge` 의 색 그대로다. 글자는 움직이지도 깜빡이지도 색을 돌지도 않는다(3.5절) |
| 빠름·느림 `theme.timing` | FAST `03A9FF`, SLOW `FF4503`. ModernChic 의 FAST·SLOW 글자(`Play/parts/sp_hw/system.png` 의 1590,410 과 1590,430)의 색이고 IIDX 의 파랑·빨강과 같다 |
| 램프 `theme.lamp` | NO PLAY 는 Theme 의 `lamp_off`(위 표), FAILED `FF0000`, ASSIST `FF63FD`, LIGHT ASSIST `6563FF`, EASY `00FF24`, NORMAL `FFBB00`, HARD `DFDFDF`, EX HARD `FF6E6E`, FULL COMBO `00DFFF`, PERFECT `F499F2`, MAX `EAF300`. ModernChic 의 곡 막대 램프(`Select/parts/songbar.png`)에서 읽었다. 그 스킨에서 NO PLAY 는 램프를 그리지 않으므로 꺼진 색(자체 값)을 두고, 여러 색을 도는 램프는 막대에서 그 프레임들을 그대로 돌린다(6.2절). 한 색만 쓸 수 있는 자리(폴더 정보의 칩, 분포 띠, 결과의 CLEAR 글자)에 놓는 대표색은 EX HARD 가 폴더 현황판의 Lua 값(`Select/lua/musicdisplay.lua` 의 255,110,110), FULL COMBO 가 그 램프에만 있는 프레임(청록), PERFECT 와 MAX 가 결과 화면 글자(`Result/parts/system.png` 의 clearType)의 분홍과 노랑이다(막대 램프는 둘이 같은 그림이라 갈리지 않는다) |
| 자체 신호색 `theme.tone` | red `F0564C`, orange `F0885A`, yellow `F2B33D`, green `59D185`, blue `568CF5`, violet `B26BF0`, rose `C2455E`. 난이도·판정·램프의 뜻과 무관하게 쓰는 색(스크래치와 9키 버튼의 노트·키 빔·폭발, Note Colors 의 대안 색, 지뢰, GREEN 숫자, TARGET 막대, BPM 선, "FAILED"·"STAGE FAILED" 글자, 나빠진 차의 색, MINE 라벨, 해저드 게이지)은 여기서 고른다. 웨이브 9A 에서 난이도·판정 색을 바꾸면서 이 쓰임들이 따라 바뀌지 않게 갈라냈고 값은 그 전의 것 그대로다 |

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
| 판정은 움직이지 않는다 | 플레이 필드의 판정 글자, 콤보 숫자, 그 위 줄(점수 차, FAST/SLOW·ms)은 판정이 난 프레임부터 같은 자리, 같은 크기, 같은 색, 같은 불투명도로 그려지고 500ms 뒤 프레임에 통째로 사라진다. 튀어나옴·확대·미끄러짐·깜빡임·색 돌림·페이드가 없다. 콤보가 올라가면 숫자만 바뀌고, 자릿수가 늘어도 글자와 숫자의 첫 자리는 그대로이며 숫자는 오른쪽으로만 자란다(6.3절). 판정이 바뀌어도 글자의 왼쪽 끝은 같은 자리다. 플레이 중 눈이 머무는 자리라 움직임이 노트를 읽는 데 방해가 되기 때문이다(웨이브 10C 의 사용자 요청) |

## 4. 공통 Lua 모듈

### 4.1 실행 모델

- Lua 5.2 다(mlua `lua52`). 수는 double 하나뿐이다: `7 / 2` 는 3.5 이고 정수 나눗셈 연산자가 없다. 좌표는 `ui` 가 반올림한다. 비트 연산은 `bit32`, 가변 인자 풀기는 `table.unpack` 이다.
- 엔진은 화면에 들어갈 때마다 진입 문서를 같은 Lua 상태에서 두 번 실행한다: 헤더 패스(`skin_config == nil`)와 본체 패스(`crates/rbms-skin/src/loader/lua_skin.rs`). `require` 한 모듈은 첫 패스에서 한 번만 실행되고 캐시된다.
- 그래서 **모듈 최상위에서는 `skin_config` 를 읽거나 `main_state` 함수를 부르지 않는다.** 스킨 목록용 헤더 전용 읽기에서는 `main_state` 가 빈 테이블이다. 둘 다 본체를 조립하는 함수 안에서만 쓴다.
- `require` 경로: 점을 `/` 로 바꿔 진입 문서가 있는 폴더(팩 루트) 아래에서만 찾는다(`crates/rbms-skin/src/lua/package.rs`). 모듈 이름은 항상 `lua.` 로 시작한다(예: `require("lua.ui")`, `require("lua.decide.main")`).
- 색은 본체 패스에서 정한다(웨이브 14). 화면 모듈의 `build` 가 `skin_config` 를 확인한 바로 다음에 `theme.apply(skin_config.option)` 을 부르고, 그 뒤에 조립기를 만든다. `theme.apply` 는 `theme.color` 의 색 표를 새로 만들지 않고 **그 자리에서 채널 값만 고쳐 쓴다**. 모듈은 헤더 패스에서 이미 실행되어 최상위의 표들이 `theme.color.text` 같은 색 표를 참조로 쥐고 있기 때문이다(예: `TITLE_KINDS`, `PACE.bars`). 그래서 지켜야 할 것이 둘이다. (1) 모듈 최상위에서는 색 표를 **참조로만** 쥔다: 채널을 꺼내 복사하거나 `theme.hex`·`theme.mix` 로 문자열·새 색을 만들어 두면 옵션을 따르지 않는다. 그런 계산은 조립 함수 안에서 한다(플레이의 hiterrorvisualizer, 결과의 gaugegraph·timingdistributiongraph 색이 그렇게 한다). (2) 테마에 따라 다른 구운 조각은 `theme.sprite(name)` 으로 이름을 고른다(5.1절). 헤더 패스와 스킨 목록용 헤더 읽기에서는 `theme.color` 가 기본값(Mono, White)이고 옵션 선언만 나간다.
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
    theme.apply(skin_config.option)
    local s = ui.new(header)
    ...
    return s:finish()
end
```

### 4.2 모듈

| 파일 | 맡는 것 | 다른 모듈 의존 |
| --- | --- | --- |
| `lua/theme.lua` | 색, 글자 크기, 격자, 시간, 폰트 경로, 이름표(`difficulty_name`, `clear`: 클리어 0~10 의 열쇠·이름·램프 색·글자색, `rank_name`, `gauge_name`, `random_name`), 판정 글자의 색 `judge_word`(PGREAT 하늘색, GREAT 노랑. 판정 줄은 정적이라 순환 주기는 없다), 램프의 순환 주기 `lamp_cycle`(50ms), BPM 선의 색 묶음 `bpm_line`. 순수 데이터와 `theme.column`, `theme.content_width`, `theme.cap_height`, `theme.text_top(baseline, size)`(기준선에서 글자 윗선), `theme.centered_top(box, size)`(상자 가운데에 놓을 글자의 윗선), `theme.clear_text(entry)`(클리어 이름의 글자색. NO PLAY 만 `muted`), `theme.mix(color, toward, share)`(두 색을 섞은 색), `theme.hex(color, alpha)`(그래프 객체가 받는 `rrggbb`·`rrggbbaa` 문자열). 웨이브 14: 두 벌의 단계 `theme.mono`·`theme.slate`, 강조색 `theme.accent`·`theme.accent_dim`, 노트의 색 `theme.note`, 색 옵션의 op 와 이름 `theme.OPTION`·`theme.OPTION_NAME`, `theme.properties(list)`(헤더의 `property` 목록 끝에 Theme 과 Accent 행을 붙여 돌려준다. 인자를 생략하면 그 두 행만), `theme.apply(option)`(본체 패스에서 `skin_config.option` 을 받아 `theme.color` 의 채널 값과 `theme.sprite_suffix` 를 정한다. 고른 것이 없으면 Mono 와 White), `theme.sprite(name)`(테마마다 따로 구운 조각의 이름. Mono 는 그대로, Slate 는 `_slate` 를 붙인다) | 없음 |
| `lua/props.lua` | beatoraja 속성 id 의 이름표(매직넘버 금지). `skin_type`, `image`(참조 이미지 -100~-111), `timer`, `option`, `number`, `text`, `event`, `index`(이미지 인덱스), `rate`(슬라이더·그래프 type), `offset`, `blend`, `stretch`, `acc`, `text_align`, `text_overflow`, `number_align`, `loop`, `filter`, `padding`(value 의 0 채움), `graph_angle`(graph 가 자라는 방향), `slider_angle`, `judge_graph`(judgegraph 의 type), `back_texture`(judgegraph 의 backTexOff), `no_value`(숫자 속성의 "값 없음" 표지 둘). 이름은 beatoraja `SkinProperty` 의 상수 이름에서 접두사를 뗀 것이고, `stretch` 의 이름만 엔진의 `StretchKind`(`crates/rbms-skin/src/loader/stretch.rs`)를 따른다(레퍼런스의 이름이 `KEEP_ASPECT_RATIO_FIT_INNER` 꼴로 길다) | 없음 |
| `lua/ui.lua` | 스킨 테이블 조립기. 설계 좌표 변환, id 발급, 객체 정의와 배치 | theme, props |
| `lua/motion.lua` | 키프레임 프리셋 | theme, props, ui |
| `lua/parts.lua` | 여러 화면이 쓰는 조합 부품과, 속성 id 가 붙은 공통 표(`parts.judges`, `parts.stage_image`) | theme, props, ui, motion |
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
| `timer`, `loop`, `op`, `draw`, `blend`, `filter`, `center`, `stretch` | beatoraja destination 필드 그대로. `timer`·`draw` 는 Lua 함수도 된다 |
| `offset`, `offsets` | 오프셋 id 하나 또는 여럿. `ui` 는 둘을 `offsets` 한 배열로 합쳐 낸다(`offsets`, `offset`, 그리고 `s:within` 이 정한 부위 id 순. 레퍼런스의 `offsets + [offset]` 과 같은 집합이다) |
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
| `ui.has_value(value)` | `main_state.number` 가 준 수 | 값 없음 표지(`props.no_value`)도 nil 도 아니면 참 |
| `s:sprite_source(name)` | 스프라이트 이름 | `{ src, x, y, w, h, divx, divy }`. graph·slider·커버처럼 `s:object` 로 넣는 정의가 시트의 한 조각을 원본으로 쓸 때 합쳐 넣는다 |
| `s:define_text(spec)`, `s:text_box(spec, box)` | `s:text` 와 같음 | 배치 없이 텍스트 정의만 만들어 id 를 돌려주는 것과, 정렬에 맞는 배치 상자만 계산하는 것. 같은 텍스트를 두 번 배치하거나(선곡의 옵션 시트) songlist 안에 넣을 때 쓴다 |
| `s:define_number(spec, digits)` | `s:number` 와 같음 | 배치 없이 숫자 정의만 만든다. songlist 의 level, judge 의 콤보처럼 id 만 넘기는 곳에 쓴다 |
| `s:object(list, def, box, opts)` | 목록 이름("slider", "graph", "judgegraph", "bpmgraph", "gaugegraph", "imageset", "judge", ...) | `def` 를 그 목록에 넣고 배치한다. `box` 가 없으면 정의만 |
| `s:single(key, def, box, opts)` | "note", "gauge", "bga", "songlist" | 하나만 있는 객체를 넣고 배치한다. `box` 가 없으면 키프레임 없는 destination(note, songlist) |
| `s:dst(id, box, opts, parent)` | - | 스킨에 넣지 않고 destination 테이블만 만든다. songlist 의 `liston`·`text`, note 의 `group`, judge 의 `images` 처럼 객체 안에 들어가는 destination 용. `parent` 상자를 주면 그 상자 기준의 상대 좌표로 뒤집는다(막대 안의 글자 등) |
| `s:place(id, box, opts)` | - | 이미 정의된 id 를 배치한다 |
| `s:within(part, build)` | 오프셋 id, 인자 없는 함수 | `build()` 를 부르는 동안 `s:dst` 가 만드는 모든 destination 의 `offsets` 끝에 `part` 를 붙인다(배치 함수 전부와 중첩 destination 이 `s:dst` 를 거친다). 플레이 화면이 부위별 불투명도 오프셋을 거는 데 쓴다(2.3절). 겹쳐 부르면 안쪽 것만 붙고, 끝나면 바깥 것으로 돌아간다 |
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
| `parts.judges` | 판정 5행의 표(IIDX 의 줄 구성): `name`, `color`, `count`·`fast`·`slow`(숫자 id 의 목록. 여럿이면 더한다). PGREAT, GREAT, GOOD, BAD, POOR 순이다. POOR 는 헛침을 포함한다: 개수 426(POOR + 헛침), 빠름 418 + 421, 느림 419 + 422 이고 노트 수를 넘을 수 있어 `unbounded = true` 다 |
| `parts.combo_break`, `parts.empty_poor` | 판정이 아닌 두 집계의 이름과 숫자 id: "COMBO BREAK"(425 = BAD + POOR), "EMPTY POOR"(420 = 헛침). 결과의 판정 표만 쓴다(6.5절) |
| `parts.sum(main_state, refs)`, `parts.tally(main_state, refs, places)` | 숫자 id 여럿의 합(하나라도 값 없음이면 그 표지), 그 합을 `places` 자리의 최댓값에서 멈추는 value 의 값 함수 |
| `parts.judge_graph_legend`, `parts.legend(s, right_edge, top, items, opts)` | judgegraph 가 고정해 그리는 판정 색(5·7·10·14키 `beat`, 9키 `popn`)의 범례 항목과, 항목을 오른쪽 끝에서부터 색 칩 10x10 + 이름(크기 18, `muted`)으로 놓는 것. 플레이의 타이밍 영역과 결과의 오른쪽 열이 쓴다 |
| `parts.stage_image`, `parts.stage_image_property()`, `parts.stage_image_shown()` | Stage Image 옵션의 이름·op(900, 901)·그림 없음 문구, 그 옵션의 헤더 `property` 항목, 본체 패스에서 Show 인지. 결정과 선곡이 쓴다 |
| `parts.shown_or_hidden(name, show, hide)` | Show(기본)/Hide 두 항목의 `property` 항목 |
| `parts.choice(main_state, id, names, missing)` | `event_index(id)` 를 이름표에서 골라 주는 text 의 값 함수. 이름표에 없으면 `missing` |
| `parts.bpm_graph(lines)` | bpmgraph 정의(선 두께와 색 6개). `lines` 를 생략하면 `theme.bpm_line` |
| `parts.fade_from_black(s, duration)` | 장면 시작에 `bg` 덮개가 걷힘 |
| `parts.fade_to_black(s, start, duration)` | FADEOUT 타이머에 `bg` 덮개가 덮임 |

정적 조건(난이도, 키 수, 스킨 옵션)에 걸린 객체는 엔진이 로드 때 한 번 평가해 지운다. 그래서 6벌을 선언해도 화면에 남는 것은 한 벌이다(결정 화면: 선언 59, 남는 것 43). 선곡에서는 곡에 딸린 조건이 정적이 아니라서 전부 남는다(499).

공통으로 올리지 않은 것. 한 화면만 쓰는 것은 그 화면의 모듈에 둔다: 선곡의 `kit`(곡 이동 타이머의 등장, 호버 버튼, 가로 graph·slider 정의), 플레이의 `caption_on`·분:초 표기·발치의 열 계산, 결과의 `layout`(행 계산, 구역별 등장, NEW RECORD 표지)과 키 수로 가르는 안내 줄. 소수 표기(정수 띠 + 사각 점 + 소수 띠 + 단위)는 플레이와 결과가 각각 갖고 있다: 플레이는 오른쪽 끝에서 왼쪽으로 놓고 값이 프레임마다 바뀌며, 결과는 왼쪽에서 오른쪽으로 놓고 값이 정해져 있어 배치 계산이 다르다.

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
- 색을 구운 조각 가운데 Theme 에 따라 달라지는 것(선곡 막대 7종, 램프 띠, 게이지 칸 36)은 테마마다 한 벌씩 굽는다(웨이브 14). Mono 의 조각이 원래 이름이고 Slate 의 조각은 이름 끝에 `_slate` 가 붙는다(`bar_song_slate`, `gauge_0_3_slate`). Lua 는 `theme.sprite("bar_song")` 으로 고른다. 한 색을 입혀서는 두 색(막대의 면과 꼬리표, 게이지 칸의 여섯 상태)이 따로 가지 않아서 destination 색이 아니라 조각을 늘렸다. Accent 는 구운 조각이 없다(전부 흰 조각에 destination 색). Note Colors 는 노트 색마다 한 벌(여덟 색)을 굽고 레인이 제 색의 조각을 고른다.
- 배경, 패널, 선, 막대, 덮개는 이미지가 아니라 색 사각형이다.

### 5.2 시트 목록

| 파일 | 크기 | 스프라이트 | 쓰는 화면 | 용량(실측) |
| --- | --- | --- | --- | --- |
| `image/common.png` | 1024x161 | `num18`, `num24`, `num32`, `num44`, `num64`, `num96`(숫자 0~9 한 줄 10칸, 흰색), `snum24`, `snum32`(부호 있는 24칸: 12열 2행, 열 10 은 빈칸, 열 11 은 + 와 -), `block`(8x8 흰색) | 전부 | 39,312 B |
| `image/decide.png` | 1024x112 | `num144`(레벨 숫자) | 결정 | 16,043 B |
| `image/select.png` | 1024x941 | 31개. 막대 7종 `bar_song`, `bar_folder`, `bar_table`, `bar_course`, `bar_missing`, `bar_command`, `bar_search`(각 728x64. 반투명 면은 테마의 `bar` 색에 알파 26, missing 만 13. 왼쪽 x 20 에 종류 꼬리표 18px `muted`)와 그 Slate 판 7종(`bar_*_slate`), 라벨 글자 `label_ln`, `label_random`, `label_mine`, `label_cn`, `label_hcn`(18px 흰색, 높이 17), `lamp_strip`(11칸 x 8x8, NO PLAY 부터 MAX. 폴더 분포 그래프의 원본. 첫 칸이 테마의 `lamp_off` 라 Slate 판 `lamp_strip_slate` 가 따로 있다), 색을 도는 램프 5종 `lamp_failed`(빨강, 빈 칸), `lamp_exhard`(호박, 빨강), `lamp_fullcombo`(호박, 청록, 흰색), `lamp_perfect`·`lamp_max`(호박, 자홍, 흰색. 한 칸 8x8 을 가로로 이었다), `meter`(512x4 흰색. 가로 graph 의 원본), `icon_settings`, `icon_volume`(32x32, 도형으로 그린다), `star_on`, `star_off`(24x24) | 선곡 | 28,422 B |
| `image/play.png` | 1024x570 | 127개. 노트 8색(white, cyan, gray, silver, red, yellow, green, blue. 3.1절) x 5종(`note_`, `long_`, `held_`, `hell_`, `hell_held_`), `hell_drain`, `mine`, 게이지 칸 36(`gauge_<종류>_<상태>`, 4x8)과 그 Slate 판 36(`gauge_<종류>_<상태>_slate`), 판정 글자 5(크기 64, 색을 구운 한 장씩: `judge_pgreat`·`judge_great` 는 "GREAT", `judge_good`, `judge_bad`, `judge_poor`. 좌우를 잉크에 맞춰 잘라 칸의 왼쪽 끝이 곧 글자의 왼쪽 끝이다: 폭 GREAT 206, GOOD 185, POOR 173, BAD 123), 콤보 숫자 3(`combo_pgreat`, `combo_great`, `combo_good`. 판정 글자와 같은 색의 10칸 한 줄: 숫자 0~9), `bomb`(128x128 8프레임. 평면 고리 하나가 반지름 16 → 46, 두께 10 → 2, 알파 255 → 70 으로 퍼진다. 흰색으로 굽고 색은 destination 이 입힌다. 웨이브 13 에서 위 68행만 남겼던 것을 웨이브 14 에서 온전한 고리로 되돌렸다. 6.3절), `beam`(8x64 세로 알파 띠, 최대 알파 118. 둘레에 투명 1px 을 두른 10x66 으로 놓이고 좌표표는 안쪽 8x64 를 가리킨다), `pulse`(같은 띠의 최대 알파 71. 판정선의 리듬 띠), `bar_h`(256x4), `bar_v`(4x256) | 플레이 5종 | 36,843 B |
| `image/result.png` | 512x8 | `bar`(512x8 흰색. 판정 막대 graph 의 원본) | 결과, 코스 결과 | 113 B |

합계 120,733 B(118 KiB). 아틀라스는 common 1,365 B, decide 293 B, select 2,714 B, play 10,398 B, result 213 B 다. 목표 6 MiB 와의 차이는 예비다. 판정 글자와 콤보 숫자는 `play` 시트에 색째로 굽고(destination 의 색은 흰색이다), READY·FAILED·FULL COMBO·LOADING·FAST·SLOW, 랭크 글자와 클리어 이름은 TTF 다.

### 5.3 생성 스크립트 `tools/generate-images.py`

| 항목 | 내용 |
| --- | --- |
| 실행 | `uv run assets/skins/rbms-default/tools/generate-images.py [구역 ...] [--check]`. 구역을 생략하면 전부 |
| 의존성 | PEP 723 인라인 메타. `requires-python = ">=3.11"`, `pillow==12.3.0` 하나(2026-10-11 PyPI 최신 안정판, 2026-07-01 배포). 버전을 고정한 것은 래스터 결과를 고정하기 위해서다 |
| 폰트 | `font/Inter-Regular.ttf`(스킨 폴더 안의 것) |
| 구조 | 파일 셋으로 나뉜다. `tools/generate-images.py`(진입점: 등록표 `SECTIONS`, 선반 배치, PNG·아틀라스 출력, `main`), `tools/sprites.py`(공통 그리기 헬퍼와 `Sprite`), `tools/sections/<구역>.py`(구역마다 하나. `build() -> list[Sprite]` 와 그 구역의 상수). 진입점이 `sections` 와 `sprites` 를 import 한다(스크립트 폴더가 import 경로에 있다). 진입점은 `sys.dont_write_bytecode` 를 켜므로 `__pycache__` 가 생기지 않는다 |
| 공통 헬퍼(`sprites.py`) | `digit_strip(name, size, color)`(10칸 숫자 띠. 색을 주지 않으면 흰색이다. 콤보 숫자는 판정 색으로 굽는다), `signed_digit_strip(name, size)`(24칸), `word(name, text, size, color)`(글자 한 덩이), `block(name, w, h, color)`(단색 사각형), `tinted(mask, color)`(알파 마스크에 색), `guarded(sprite, margin)`(둘레에 투명 `margin` px 을 두르고 좌표표에는 안쪽 사각형만 적는다. 선형 보간으로 늘려 그리는 작은 그림이 시트에서 어디에 놓이든 가장자리가 같은 투명 픽셀과 섞이게 한다. `Sprite.margin`). 전부 `Sprite(name, image, divx, divy, meta)` 를 돌려준다. `palette(group)` 은 `lua/theme.lua` 의 `theme.<group>` 표에서 색을 읽어 `{ 이름: (r, g, b) }` 로 돌려준다(group 은 `mono`, `slate`, `accent`, `accent_dim`, `note`, `tone`, `difficulty`, `judge`, `judge_word`, `timing`, `lamp`. 표에 적힌 순서를 지킨다. `theme.color` 는 Lua 가 실행 중에 채우는 표라 읽지 않는다). `themes()` 는 테마마다 `{ 조각 이름의 접미사: 그 테마의 palette }` 를 돌려준다(`""` 가 Mono, `"_slate"` 가 Slate. `THEME_SUFFIXES`). 테마를 타는 조각은 이것을 돌며 한 벌씩 굽는다 |
| 색의 출처 | 시트에 굽는 색(선곡의 램프 띠·도는 램프와 꼬리표, 플레이의 노트·게이지 칸·지뢰·판정 글자·콤보 숫자)은 구역 파일에 다시 적지 않고 `palette` 로 `theme.lua` 에서 읽는다. 팔레트를 바꾸면 `theme.lua` 만 고치고 그 색을 굽는 구역을 다시 생성한다(`--check` 가 빠뜨린 구역을 알려 준다). 테마의 단계(`mono`, `slate`)를 굽는 것은 선곡의 막대(`bar`, `muted`)와 램프 띠(`lamp_off`), 플레이의 게이지 칸(`bg`, `line`)이고, 노트는 `note`·`tone` 만 읽는다. 강조색(`accent`)은 어느 조각에도 굽지 않는다 |
| 구역 파일 | 다섯 구역 모두 완성. 구역에만 필요한 그리기 함수(선곡의 아이콘, 플레이의 노트·폭발, 좌우를 잉크에 맞춰 자르는 판정 글자 `judge_word`)는 그 구역 파일 안에 있다 |
| 배치 | 높이 내림차순, 이름순으로 정렬해 선반식으로 놓는다. 스프라이트 사이 2px. 시트 폭은 128~4096 의 2의 거듭제곱 중 정사각형 안에 들어가는 가장 작은 것 |
| 출력 | `image/<구역>.png`(RGBA, 투명 픽셀도 RGB 는 흰색: 선형 보간 때 어두운 테두리가 생기지 않게)와 `lua/atlas/<구역>.lua`(이름순). 스프라이트가 없는 구역은 아무것도 쓰지 않는다 |
| 결정성 | 같은 입력에서 바이트가 같다(8절에서 확인). 시각·난수·환경 변수에 의존하지 않는다. 다른 OS 나 다른 Pillow 판에서의 바이트 동일성은 보장하지 않으므로 CI 에서 다시 만들지 않는다 |
| `--check` | 쓰지 않고, 만든 바이트가 저장소의 파일과 같은지 비교한다. 다르면 종료 코드 1. 스크립트만 고치고 생성물을 빠뜨린 커밋을 찾는 용도다 |

숫자 띠의 칸: 폭은 그 띠에 든 글자(숫자 10개, 부호 띠는 + 와 - 포함)의 잉크 폭 최댓값 + 양옆 여백(각각 em 의 0.04배를 반올림, 최소 1px), 높이는 숫자 글자의 위아래 끝 + 위아래 여백 2px. 각 글자는 잉크가 칸 가운데에 오도록(남는 1px 은 오른쪽), 같은 기준선에 놓인다. 칸 폭은 18 → 12, 24 → 15, 32 → 19, 44 → 28, 64 → 40, 96 → 59, 144 → 89 다. 아틀라스의 `size` 는 em 크기, `pad` 는 위아래 여백, `ink` 는 숫자 0~9 의 잉크 폭 10개다(`ui` 가 양끝 여백을 여기서 계산한다. 3.3절). beatoraja 의 프레임 수 규칙상 10칸은 숫자만, 24칸은 부호 있는 숫자(`zeropadding` 으로 채움 방식 지정)다[b2 §10.2].

## 6. 화면별 설계

좌표는 설계 좌표 `(x, top, w, h)`. 전부 구현된 최종값이다. 화면을 고치면 이 절의 표도 같이 고친다.

화면 사이에 같은 것: 머리 캡션(64, 70, 크기 18, `muted`)과 머리 구분선(top 104), 바닥 구분선(top 1000), 안내 줄(크기 18, `muted`, "키  동작" 을 공백 8칸으로 잇는다. 결정·결과는 top 1022 한 줄, 선곡은 1016 과 1046 두 줄), 항목 이름은 위·값은 아래, 난이도 막대 폭 8, 장면 시작 덮개 300ms(`parts.fade_from_black`), 그림이 없을 때의 문구는 `panel` 면 가운데 크기 18 `muted`("NO STAGE IMAGE", "NO BGA"), 판정 6행의 이름과 색(`parts.judges`), 클리어 이름과 색(`theme.clear`). 플레이는 머리·바닥 구분선과 캡션을 쓰지 않는다(필드가 위에서 아래까지 간다).

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
| 캡션, 항목 이름, 안내, "NO STAGE IMAGE" | text(constantText) | "NO STAGE IMAGE"(`parts.stage_image.missing`)는 op { 900, 190 } | 폰트 |
| 표 이름, 장르, 제목, 부제, 아티스트 | text(ref 1003, 13, 10, 11, 16) | overflow 1(축소) | 폰트 |
| 스테이지 그림 | image(-100) | op { 900, 191 }, stretch 1 | 곡의 그림 |
| 레벨 | value(ref 96) | 자릿수는 본체 패스에서 `main_state.number(96)` 로 정한다(3.3절의 `known`, `fit` 212). op 150~155 로 색 6벌 | `decide` 의 `num144` |
| 난이도 이름 | text | op 150~155 | 폰트 |
| BPM | value(ref 90, 91) | 최대 하나는 op 176, 최소·줄표·최대는 op 177. 두 값의 자릿수를 본체 패스에서 읽어(`known`) 줄표와 최대의 x, 줄일 크기를 정한다 | `common` 의 `num64` |
| 노트 수 | value(ref 74) | 자릿수는 본체 패스에서 정한다(`known`, `fit` 272) | `num64` |
| 판정 랭크, 키 수 | text | op 180~184, op 160~164 | 폰트 |
| 노트 분포 | judgegraph(type 0, backTexOff 1) | op 902 | 없음 |
| BPM 변화 | bpmgraph(`parts.bpm_graph()`: 선 두께 2, 색 6개) | op 902 | 없음 |

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

### 6.2 선곡(type 5) — 구현 완료

구현: `select.luaskin`, `lua/select/main.lua`(조립), `list.lua`(곡 목록), `info.lua`(곡·폴더·코스 정보), `controls.lua`(머리와 바닥의 버튼), `side.lua`(사이드 메뉴), `sheet.lua`(옵션 시트), `kit.lua`(선곡 전용 부품). destination 499개, 프레임마다 부르는 Lua 함수 55개.

(가) 영역

| 영역 | x | top | w | h | 내용 |
| --- | --- | --- | --- | --- | --- |
| 머리 띠 | 0 | 0 | 1920 | 104 | `bg` 불투명, 목록 위에 그린다. 구분선 top 104 |
| 머리: 캡션 | 64 | 70 | - | 18 | "MUSIC SELECT" |
| 머리: 검색 | 368 / 456 | 70 / 56 | 88 / 640 | 18 / 36 | 이름 "SEARCH". 입력 칸은 `panel` 면 + 아래 `line` 2px. editable text(ref 30, 크기 24)는 (468, 66, 616) |
| 머리: 필터 버튼 3개 | 1128, 1368, 1608 | 40 | 224, 224, 248 | 56 | MODE(text 60), SORT(text 61, 빈 문자열이면 "DEFAULT"), LN MODE(event_index 308). 이름 top 46(18), 값 top 66(24). 안쪽 여백 12. 호버 시 `raised` |
| 곡 머리 | 64 | 136 | 1032 | 200 | 막대 (64, 136, 8, 200): 바탕 `line`, 곡이면 난이도 색, 폴더·코스면 `accent`. 글줄 x 104, w 992: 경로(top 136, 18, `muted`), 장르(162, 24, `sub`), 제목(196, 64), 부제(270, 24, `sub`), 아티스트(302, 32). 코스 막대는 제목 자리에 "COURSE" |
| 스테이지 그림 | 64 | 368 | 424 | 318 | `panel` 면 + -100(FIT_INNER). op { 900, 2 }. 그림이 없으면 가운데에 "NO STAGE IMAGE"(크기 18, `muted`) |
| 곡 항목 | 520, 712, 904 | 368, 485, 602 | 176, 176, 192 | - | 이름은 행 top, 값 기준선은 행 top + 64. 1행 LEVEL, BPM, NOTES. 2행 LENGTH, JUDGE, TOTAL. 3행 KEYS, DIFFICULTY(셋째 칸 없음). Stage Image 가 Hide 면 칸 x 64, 408, 752 / w 312, 312, 344 |
| 기록 | 64 | 702~968 | 1032 | - | 구분선 top 702. RANK (64, 이름 718, 글자 top 752, 크기 96). CLEAR (64, 이름 898, 값 기준선 962, 크기 32, 램프 색, 폭 272 가로 축소. 이름은 `theme.clear`: NO PLAY, FAILED, ASSIST CLEAR, LIGHT ASSIST CLEAR, EASY CLEAR, CLEAR, HARD CLEAR, EX HARD CLEAR, FULL COMBO, PERFECT, MAX. ModernChic 의 선곡·결과 표기를 IIDX 식 "~ CLEAR" 로 풀어 적은 것이다). 숫자 2열 x 368 / 580, 행 718, 808, 898(값 기준선 +64, `num44`): EX SCORE·MAX COMBO·PLAY COUNT / RATE·MISS COUNT·CLEAR COUNT. 판정 5행 x 824, w 272, 첫 행 718, 피치 50: 이름(18, 판정 색. PGREAT, GREAT, GOOD, BAD, POOR), 개수(`num24` 오른쪽 정렬. POOR 는 헛침을 더한 426. 기록 없는 곡은 같은 자리에 오른쪽 정렬 줄표 18, `muted`), 막대 (top +26, h 6) |
| 폴더 정보 | 64 | 368 | 1032 | - | CHARTS(이름 368, `num64` 기준선 448). CLEAR LAMPS(이름 480). 11행 2열(x 64 / 580, w 484, 첫 행 512, 피치 72, 행 높이 40, 열당 6행 → y 512~912): 색 칩 8x40, 이름(x +24, 크기 32, `sub`), 개수(`num32` 오른쪽 정렬). 글자 윗선은 행 top + 8. MAX 부터 NO PLAY 순 |
| 코스 정보 | 64 | 368 | 1032 | - | STAGES(이름 368), 곡명 5줄(첫 줄 400, 피치 56, 크기 32). CONSTRAINTS(이름 898), 값 top 930(24, `sub`) |
| 곡 목록 | 1128 | 16~1088 | 728 | 64 | 15슬롯, 피치 72, 슬롯 i 의 top = 520 + (i - 7) x 72. `liston` 과 `listoff` 가 같은 자리·크기. 온전히 보이는 것은 슬롯 2~12(top 160~944)의 11개. clickable 3~11 |
| 목록 가림 | 1096 | 104 / 952 | 776 | 48 / 48 | `bg` 사각형 둘. 슬롯 1 과 13 을 가린다(슬롯 0 과 14 는 머리·바닥 띠가 가린다). 목록 뒤, 머리 구분선 앞에 그린다 |
| 선택 틀 | 1096 | 520 | 760 | 64 | `raised` 고정 면 + 왼쪽 `accent` 띠 8px(`motion.pulse(2000, 160)`). op 1 / 2 / 3 세 벌(목록이 비면 그리지 않는다) |
| 막대 안(막대 기준) | - | - | - | - | 램프 (0, 0, 8, 64. FAILED, EX HARD, FULL COMBO, PERFECT, MAX 는 `lamp_<이름>` 의 프레임을 주기 50ms 로 돌고 나머지는 한 색이다. ModernChic 의 `lamp-*` 와 같은 구성·같은 주기). 레벨 `num24` 3자리 가운데 정렬 (16, 21.5, 한 칸 15x21). 곡 제목 (80, 22, 528, 28). 코스 제목 (136, 22, 472, 28). 그 밖의 제목 (136, 22, 576, 28). 라벨은 오른쪽 끝 714 의 한 열에 세로 3단(피치 20, 높이 17): MINE top 3.5, LN·CN·HCN top 23.5, RANDOM top 43.5. 제목의 오른쪽 끝 608 = 714 - 가장 넓은 라벨 88 - 간격 18. 트로피 (104, 24, 16, 16). 분포 그래프 (136, 54, 576, 4). 종류 꼬리표는 막대 이미지 x 20 에 구웠다 |
| 스크롤 막대 | 1872 | 136 | 12 | 832 | `panel` 선로 + 손잡이 12x32(`muted`), slider type 1, angle 2, range 800 |
| 바닥 띠 | 0 | 1000 | 1920 | 80 | `bg`, 구분선 top 1000. 안내 2줄 (64, 1016 / 1046, w 1064, 크기 18) |
| 리플레이 | 1160 / 1244~ | 1020 | 4칸 x 40, 간격 4 | 40 | 이름 "REPLAY", 칸 "1"~"4"(크기 24), 선택 슬롯은 아래 변 `accent` 2px |
| 기능 버튼 | 1440, 1582, 1714 | 1020 | 130, 120, 142 | 40 | AUTOPLAY, PRACTICE, FAVORITE(별 24x24 + 글자) |
| 사이드 탭 | 0 | 444 / 540 | 48 | 96 | 톱니, 스피커 아이콘 32x32(`muted`, 열리면 `accent` + 오른쪽 변 2px) |
| 사이드 패널 | 64 | 128 | 1032 | 840 | `raised` + 위 `accent` 2px. 제목 top 168(32): "QUICK SETTINGS" / "VOLUME". 구분선 216, 바닥 안내 top 918. 왼쪽으로 1096 떨어진 곳에서 200ms 에 들어온다. 닫힘은 즉시 |
| QUICK SETTINGS 행 | 96 | 240 + 72 x i | 968 | 56 | 6행. 이름(24) 왼쪽, 값(24, `accent`) 오른쪽 정렬. JUDGE ALGORITHM 은 엔진 값이 목록(COMBO, DURATION, LOWEST) 밖이면 "SCORE (SET IN SETTINGS)" |
| VOLUME 행 | 96 | 256 + 128 x i | 968 | - | 이름(24), 백분율(`num32` 를 24 로 줄임, 오른쪽 정렬) + "%". 선로 (top +52, h 4), 채움 graph, 손잡이 12x32, range 956 |
| 옵션 시트 | 64 | 312 | 1792 | 456 | 전체 가림막 `bg` 알파 216 + `raised` 면 + 위 `accent` 2px. 제목 (96, 344, 32), 여는 키 안내 오른쪽 정렬 top 350, 구분선 400 |
| 시트 칸 | 96 + 248 x (k - 1) | 432~572 | 224 | - | "KEY k"(432, 18, `accent`), 항목 이름(464, 24), 값(508, 32), 동작(572, 18. 폭을 넘으면 줄어든다). 동작은 글자이거나 Lua 값 함수다. 값이 없는 항목은 이름 `muted` + "NOT AVAILABLE"(18) |
| 시트 아래 | 96 | 620~716 | 1728 | - | 세 패널 모두 구분선 620. 패널 1: "TURNTABLE / BUTTONS 8-9"(648, 18, `accent`), "TARGET"(680), 값 text 3(716, 32). 패널 2·3: 주석 한 줄 top 648(18, `muted`) |

옵션 시트의 칸(건반 순서). 값은 `main_state.event_index(id)` 를 읽는 Lua 값 함수(`parts.choice`)로 이름표에서 고른다(이미지 시트를 만들지 않는다). 엔진이 값을 주지 않는 항목은 "NOT AVAILABLE" 로 적는다(`apps/rbms-player/src/stage/select/panel.rs`, `skin_host/options.rs`).

| 패널 | 여는 키 | 칸 1~7 |
| --- | --- | --- |
| 1 PLAY OPTIONS | START | RANDOM(index 42, NEXT), RANDOM(42, PREVIOUS), GAUGE(40), DP OPTION(54), HI-SPEED FIX(55), RANDOM(42, PREVIOUS), RANDOM(42, NEXT). index 43 은 읽지 않는다 |
| 2 ASSIST OPTIONS | SELECT | EX JUDGE(index 301 의 OFF/ON, 동작 "SET IN SETTINGS"), CONSTANT(302), JUDGE AREA, LEGACY NOTE(304), MARK NOTE, BPM GUIDE, NO MINE. 값이 없는 넷은 "NOT AVAILABLE" |
| 3 DETAIL OPTIONS | START+SELECT 또는 상세 옵션 키(출하 시 숫자 5). 표지는 "HOLD START AND SELECT, OR THE DETAIL OPTIONS KEY (5 AS SHIPPED)" | BGA(72), GAUGE AUTO SHIFT(78), TIMING AUTO ADJUST(75), NOTE DISPLAY TIME(동작 "1 MS SHORTER"), JUDGE TIMING(number 12, 동작 "NOTES 1 MS LATER". 5번 키는 값을 -1 한다), NOTE DISPLAY TIME("1 MS LONGER"), JUDGE TIMING("NOTES 1 MS EARLIER". 7번 키는 +1). 웨이브 13 통합에서 JUDGE TIMING 의 동작 줄을 고쳤다(전에는 "1 MS EARLIER"·"1 MS LATER". 값이 노트를 그리는 자리만 옮기게 되면서, 무엇이 옮겨지는지 적지 않은 문구가 PLAY 메뉴의 도움말 `Draw the notes this many ms earlier (+) or later (-)` 과 반대로 읽혔다). 바닥 문구는 NOTE DISPLAY TIME 이 KEEP GREEN NUMBER 가 켜져 있을 때만 움직인다는 안내다 |

NOTE DISPLAY TIME 두 칸(웨이브 10A)은 number 312(이동 시간 ms)와 313(녹색 숫자)을 읽는 Lua 값 함수다. 엔진은 KEEP GREEN NUMBER 가 켜져 있을 때만 두 값을 준다(`skin_host/options.rs` 의 `ChartSettings::duration_ms`). 값이 있으면 값 줄에 "500 MS", 동작 줄에 "GREEN 300   1 MS SHORTER"(6번 키는 LONGER)를 쓰고, 없으면 값 줄에 "OFF", 동작 줄에 "KEEP GREEN NUMBER IS OFF" 를 쓴다. 4·6번 키는 누르고 있으면 반복된다(`stage/select/panel.rs` 의 `duration_moves`).

(나) 객체

| 객체 | 종류 | 값·타이머·조건 | 자산 |
| --- | --- | --- | --- |
| 곡 목록 | songlist | center 7, clickable 3~11. `listoff`/`liston` 15개(imageset `set.bar` 7종, filter 1). text 11종(크기 28. 일반·신규 모두 `text`, 곡 없음 `muted`, 명령·검색 `sub`). level 7(`num24`, digit 3, 가운데 정렬, 난이도 색). lamp 11(`theme.clear` 의 색. 시트에 `lamp_<열쇠>` 가 있는 다섯은 그 image 를 `cycle = 50` 으로), trophy 3(`img.block` 에 색). label 5(세로 3단). graph type -1 | `select` 의 bar_*, label_*, lamp_strip, lamp_* |
| 목록 가림 | image(-111 에 `bg`) 2개 | 조건 없음 | - |
| 스크롤 손잡이 | slider | type 1, angle 2, range 800, changeable | `img.block` |
| 곡 머리 글줄 | text(ref 1000, 13, 10, 11, 16) | 타이머 11, 왼쪽 16px 에서 150ms. overflow 1 | 폰트 |
| 곡 항목 | value(ref 96 digit 3, 74 digit 5, 368 digit 4, saturate), text(Lua: BPM "최소 - 최대", 길이 "분:초"), text(op 150~155, 160~164, 180~184) | op 2, 타이머 11, 150ms 등장. text 1003 과 op 1008 은 쓰지 않는다(선곡에서 값이 없다. 7절) | `num44`, 폰트 |
| 기록 | value(ref 71, 75, 76, 77, 78. digit 5, saturate), value(ref 110~113·426, op { 2, -100 }), text(Lua: RATE), text(랭크 op { 2, -100, 200~207 }, 클리어 op 100~105·1100~1104), 줄표(op { 2, 100 }. 판정 5행 포함) | op 2, 타이머 11 | `num44`, `num24` |
| 판정 막대 | graph(type 140~144, angle 0) | op 2, 타이머 11, 300ms 자람 | `select` 의 meter |
| 스테이지 그림 | image(-100) | op { 900, 2 }, 타이머 11, 200ms | - |
| 그림 없음 문구 | text | op { 900, 2, 190 }. 타이머 11 의 300ms 부터 200ms 등장(그림을 읽는 동안에는 보이지 않게 늦춘다) | 폰트 |
| 폴더 정보 | value(ref 300 digit 5 `num64`, 330~320 digit 5 `num32`, saturate) | op 1 | `num64`, `num32` |
| 코스 정보 | text(ref 150~154), text(Lua: op 1002~1017 을 이름으로 이음) | op 3 | 폰트 |
| 검색 | text(ref 30, editable) | - | 폰트 |
| 필터 버튼 | image(act 11, 12, 308) + text | 호버 `mouseRect` | `img.block` |
| 리플레이 | image(act 19, 316, 317, 318), text(op ±197, ±1197, ±1200, ±1203), 밑줄(op 1205~1208) | - | `img.block` |
| 기능 버튼 | image(act 16, 315, 90), imageset(ref 90: star_off, star_on) | - | `select` 의 star_* |
| 사이드 메뉴 | image(act = Lua 함수), text(Lua: event_index 308, 72, 78, 340), image(act 13, 14, 308, 72, 78, 340), slider(type 17, 18, 19, angle 1, changeable), graph(type 17~19, angle 0), value(ref 57~59, digit 3) | 타이머 = `timer_util.timer_observe_boolean` 함수 2개. 여닫을 때 `sound/o-open.wav`, `o-close.wav` | icon_settings, icon_volume, meter |
| 옵션 시트 | image, text(Lua: event_index 42, 40, 54, 55, 301, 302, 304, 72, 78, 75, number 12), text(ref 3) | 모든 객체를 두 번 배치한다. 등장: op 21~23 + 타이머 21~23, 24px 아래에서 150ms(감속). 퇴장: 타이머 31~33, 24px 아래로 150ms(가속), `loop = -1`. 가림막은 알파 0 ↔ 216. 텍스트 정의는 한 번만 만들어 두 배치가 같이 쓴다(`s:define_text` + `s:place`) | 폰트 |
| 덮개 | image | `parts.fade_from_black(300)` | - |

(다) 애니메이션: 장면 시작 덮개 300ms. 곡 이동(타이머 11)마다 곡 글줄 150ms 슬라이드, 그림 200ms 등장, 판정 막대 300ms 자람. 목록 스크롤 보간은 엔진이 한다. 선택 틀의 `accent` 띠는 `motion.pulse(2000, 160)`. 옵션 시트는 150ms 에 아래 24px 에서 올라오며 등장하고 같은 거리로 내려가며 퇴장한다. 사이드 패널은 200ms 에 들어오고 닫힘은 즉시다.

(라) 입력

| 객체 | 이벤트 |
| --- | --- |
| 곡 막대(슬롯 3~11) | 엔진의 막대 클릭(좌: 선택·열기, 우: 닫기). 슬롯 2 와 12 는 보이지만 클릭을 받지 않는다 |
| 스크롤 손잡이 | 슬라이더 드래그(type 1) |
| MODE, SORT, LN MODE | act 11, 12, 308(좌클릭 다음, 우클릭 이전) |
| AUTOPLAY, PRACTICE, FAVORITE | act 16, 315, 90 |
| 리플레이 칸 1~4 | act 19, 316, 317, 318(그 슬롯을 재생한다. 선택된 슬롯은 바꾸지 않는다). 선택된 슬롯은 숫자 키 4 로 넘기고 건반 7 이 재생한다(엔진. 사양 D8) |
| 사이드 탭 2개 | Lua 함수(패널 여닫기. 한 번에 하나만 열린다) |
| KEY CONFIG, SKIN | act 13, 14 |
| 볼륨 슬라이더 3개 | type 17, 18, 19 드래그 |
| 검색 | 검색 칸은 ref 30 의 editable text 다(`crates/rbms-render/src/skin_render/text_input.rs`). 선곡에서 text 30 을 읽으면 언제나 빈 문자열이므로(`skin_host/select.rs`) 평상시 칸은 비어 있다. 이 칸이 곧 검색 상자다: 칸 클릭, `/`, 숫자 키 0 이 모두 선곡 화면의 검색을 시작하고, 치는 줄은 선곡 화면이 갖고 이 칸이 자기 자리와 폰트로 그린다(`stage/select/search.rs`, `FrameData.entry`). 글자마다 목록이 좁혀지고 앱의 검색 상자는 뜨지 않는다. 확정한 검색은 폴더 경로 줄(ref 1000)에 `Search : '검색어' > ` 로 나온다. 칸 안에 안내 문구를 겹쳐 두지 않는다(입력 중인 줄과 같은 자리에 그려진다) |

(마) 뺀 것: BPM 연동 캐릭터, IR 램프·랭킹 패널과 IR 상자, 라이벌 비교, 도움말 장면, 배너, 서브타이틀 흐름 글자, 시작 연출의 셔터·줌, 목록 배치·언어 선택 옵션(D4), 표 이름·표 레벨(선곡에서는 엔진이 채우지 않는다. 표 안에서는 곡 머리의 경로가 표 이름을 보여 준다. 7절 보완 7), 신규 곡의 제목 색 구분(팔레트에 맞는 색이 없다), 사이드 패널의 닫힘 애니메이션. 종류별 노트 수·밀도·주 BPM 은 선곡에서 값이 없다(사양 웨이브 9). 리플레이 슬롯 4개는 빼지 않는다: 엔진이 네 슬롯의 유무·선택 옵션과 재생 이벤트를 모두 준다(`skin_host/select.rs` `REPLAY_OPTIONS`, `stage/select/events.rs`).

### 6.3 플레이 SP(type 0, 1, 4) — 구현 완료

구현: `play5.luaskin`, `play7.luaskin`, `play9.luaskin`, `lua/play/main.lua`(조립과 헤더), `layout.lua`(모드와 영역 계산, 옵션·오프셋의 id), `field.lua`(필드 안의 것), `info.lua`(정보 영역). `play.build(spec)` 하나를 다섯 문서가 매개변수만 바꿔 부른다. 문서는 `return require("lua.play.main").build({ keys = 7 })` 꼴이다. destination 은 play5 185, play7 197, play9 203 개이고 프레임마다 부르는 Lua 함수는 14개다(기본 옵션. 웨이브 10C 에서 레인 구분선을 레인 사이마다 하나씩으로 나누고 점수 차 숫자를 더해 늘었고, 웨이브 13 에서 건반 표시를 없애 줄었다).

`spec` 에서 정해지는 것

| 키 수 | type | 레인 배열(스킨 레인 인덱스 순) | 레인 폭 | 필드 폭 | 스크래치 |
| --- | --- | --- | --- | --- | --- |
| 7 | 0 | 건반 1~7, 스크래치(인덱스 7) | 흰 건반 58, 검은 건반 46, 스크래치 100, 사이 2 | 484 | 왼쪽 또는 오른쪽(옵션) |
| 5 | 1 | 건반 1~5, 스크래치(인덱스 5) | 같음 | 376 | 왼쪽 또는 오른쪽(옵션) |
| 9 | 4 | 버튼 1~9 | 버튼 62, 사이 2 | 574 | 없음 |

위 표는 Lane Width 가 Normal 일 때다. 레인 폭은 `layout.lua` 의 `LANE`(Normal 의 폭)에 `LANE_SCALE`(normal 1, wide 1.5) 한 값을 곱해 나오고, 필드 폭·레인 x·필드 안의 것 전부가 거기서 계산된다. Wide 의 값은 아래 (가-2)에 있다.

레인 인덱스와 타이머 번호는 [b3 §12] 의 표대로다: 5·7키는 건반이 1~, 스크래치가 0 이고 9키는 버튼이 1~9 다(`apps/rbms-player/src/skin_host/play_timers.rs` `lane_slots`). 화면에 놓는 순서는 스크래치 방향 옵션에 따라 `layout` 이 정하고, note 의 배열 순서는 언제나 레인 인덱스 순이다. 노트 높이는 16(9키 24)이다.

(가) 영역. 필드의 안쪽 변(정보 쪽)은 7·5키 548, 9키 1247 로 고정이고 필드 x 는 `안쪽 변 - 필드 폭` 이다. Graph 는 Score Graph 옵션이다.

| 영역 | 7키 | 5키 | 9키 |
| --- | --- | --- | --- |
| 필드(레인 면) | 64, 0, 484, 912 | 172, 0, 376, 912 | 673, 0, 574, 912 |
| 노트가 내려오는 영역(note 의 `dst`) | 레인별, top 0, h 904 | 같음 | 같음 |
| 레인 구분선(`line` 색, 레인 사이마다 2px) | top 0, h 912 | 같음 | 같음 |
| 판정선(offset 3) | 64, 904, 484, 8 | 172, 904, 376, 8 | 673, 904, 574, 8 |
| 게이지 막대 | 64, 928, 400, 24 (칸 8px) | 172, 928, 300, 24 (칸 6px) | 673, 928, 500, 24 (칸 10px) |
| 게이지 수치 | 오른쪽 끝 x 548, 기준선 949 | 오른쪽 끝 548 | 오른쪽 끝 1247 |
| 발치 | 64, 968, 484, 112 | 172, 968, 376, 112 | 673, 968, 574, 112 |
| 진행 막대 | 556, 0, 4, 912 | 같음 | 1255, 0, 4, 912 |
| 스코어 그래프(Graph On) | 584, 120, 136, 720 | 같음 | 1280, 120, 136, 720 |
| 곡 머리 | On 744, 40, 1112, 56. Off 584, 40, 1272, 56 | 같음 | 64, 40, 576, 56 |
| BGA | On 744, 120, 1112, 626. Off 584, 120, 1272, 626 | 같음 | 64, 120, 576, 576 |
| 점수 | On 744, 770, 352, 198. Off 584, 770, 408, 198 | 같음 | 64, 720, 576, 120 (2열, 열 폭 272) |
| 판정 수 | On 1128, 770, 352, 198. Off 1016, 770, 408, 198 | 같음 | On 1448, 120, 408, 330. Off 1280, 120, 576, 330 |
| 타이밍 | On 1512, 770, 344, 198. Off 1448, 770, 408, 198 | 같음 | On 1448, 474, 408, 366. Off 1280, 474, 576, 366 |

- 필드는 판정선에서 끝난다(웨이브 13). `layout.FIELD` 는 `top 0, height 904, line 8` 이고 `bottom = top + height + line = 912` 다. 노트가 내려오는 영역이 0~904, 판정선이 904~912 이고, 판정선의 아래 변이 곧 필드(레인 면, 레인 구분선, 진행 막대)의 아래 끝이다. 그 아래 16px 을 비우고 게이지(928)가 온다. 판정선 아래에는 레인 면도 구분선도 건반 표시도 없다(폭발만 판정선 아래로 번진다. 웨이브 14. 아래 표): 웨이브 13 전에는 판정선이 840 에 있고 그 아래 848~912 에 건반 표시(레인별 건반 면과 눌림 빛)가 있어 판정선이 필드 아래 끝에서 72px 떠 있었다. 필드의 아래 끝(912)과 게이지·발치의 자리는 그대로 두고 노트가 내려오는 영역을 840 에서 904 로 늘렸다. 누른 건반은 키 빔이 알린다. 판정선을 올리는 것은 플레이 옵션 LIFT 의 일이다: 리프트를 켜면 판정선·키 빔·폭발·판정 줄이 offset 3 만큼 올라가고, 그 아래 필드 아래 끝(912)까지는 리프트 커버가 덮는다.
- 7키 레인 x(스크래치 왼쪽): S 64, 건반 166, 226, 274, 334, 382, 442, 490. 스크래치 오른쪽이면 건반 64, 124, 172, 232, 280, 340, 388, S 448.
- 5키 레인 x(스크래치 왼쪽): S 172, 건반 274, 334, 382, 442, 490. 스크래치 오른쪽이면 건반 172, 232, 280, 340, 388, S 448.
- 9키: 673 + 64 x (n - 1).
- Field Side 가 Right 면 모든 상자를 `1920 - x - w` 로 뒤집는다(5키 필드 x 1372). 레인 순서는 Scratch Side 만 따른다.
- 게이지 칸 폭은 `floor((필드 폭 - 72) / 50)` 이다.

(가-2) Lane Width 가 Wide 일 때(웨이브 10A). 레인 폭만 1.5 배다: 흰 건반 87, 검은 건반 69, 스크래치 150, 버튼 93. 레인 사이 2px, 노트 높이(16, 9키 24), 필드의 세로 자리(노트 영역 높이 904, 판정선 904~912, 게이지 928, 발치 968)는 Normal 과 같다. 필드 폭은 7키 719, 5키 559, 9키 853 이고, 필드의 안쪽 변은 7·5키 783(바깥 여백 64 는 그대로), 9키 1387(필드는 가운데)이다.

| 영역 | 7키 | 5키 | 9키 |
| --- | --- | --- | --- |
| 필드(레인 면) | 64, 0, 719, 912 | 224, 0, 559, 912 | 534, 0, 853, 912 |
| 게이지 막대 | 64, 928, 600, 24 (칸 12px) | 224, 928, 450, 24 (칸 9px) | 534, 928, 750, 24 (칸 15px) |
| 게이지 수치 | 오른쪽 끝 x 783 | 오른쪽 끝 783 | 오른쪽 끝 1387 |
| 발치 | 64, 968, 719, 112 | 224, 968, 559, 112 | 534, 968, 853, 112 |
| 진행 막대 | 791, 0, 4, 912 | 같음 | 1395, 0, 4, 912 |
| 스코어 그래프(Graph On) | 820, 120, 136, 720 | 같음 | 1420, 120, 136, 720 |
| 곡 머리 | On 980, 40, 876, 56. Off 820, 40, 1036, 56 | 같음 | 64, 40, 440, 56 |
| BGA | On 980, 120, 876, 450. Off 820, 120, 1036, 626 | 같음 | 64, 120, 440, 440 |
| 점수 | On 980, 594, 426, 198. Off 820, 770, 328, 198 | 같음 | On 1588, 474, 268, 366. Off 1420, 474, 436, 366 |
| 판정 수 | On 1430, 594, 426, 198. Off 1172, 770, 328, 198 | 같음 | On 1588, 120, 268, 330. Off 1420, 120, 436, 330 |
| 타이밍 | On 980, 816, 876, 152. Off 1524, 770, 332, 198 | 같음 | 64, 584, 440, 256 |

- 7키 레인 x(스크래치 왼쪽): S 64, 건반 216, 305, 376, 465, 536, 625, 696. 스크래치 오른쪽이면 건반 64, 153, 224, 313, 384, 473, 544, S 633.
- 5키 레인 x(스크래치 왼쪽): S 224, 건반 376, 465, 536, 625, 696. 스크래치 오른쪽이면 건반 224, 313, 384, 473, 544, S 633.
- 9키: 534 + 95 x (n - 1).
- Field Side 가 Right 면 Normal 과 같이 모든 상자를 `1920 - x - w` 로 뒤집는다(7키 필드 x 1137).
- 7·5키 Graph On: 정보 영역의 폭이 876 으로 줄어 세 상자(점수·판정 수·타이밍)가 한 줄에 서지 않는다(타이밍은 범례 폭 약 285 와 3열 때문에 300 이상, 판정 수는 합계·FAST·SLOW 세 열 때문에 270 이상이 필요하다). 그래서 단을 둘로 나눈다: 점수와 판정 수가 윗단(426 씩), 타이밍이 전폭 아랫단(분포 그래프 876 x 46)이고 BGA 의 높이를 626 에서 450 으로 줄였다. Graph Off 는 폭 1036 에 세 상자(328, 328, 332)가 한 줄에 서므로 Normal 과 같은 구조이고 BGA 높이도 626 그대로다.
- 9키: 가운데 필드가 853 으로 넓어져 양옆 단이 440(왼쪽)과 436(오른쪽)으로 줄어든다. 오른쪽 단은 Graph On 일 때 그래프 136 을 빼면 268 이 남아 타이밍(범례)이 들어가지 않는다. 그래서 타이밍을 왼쪽 단의 BGA 아래로 옮기고(Normal 에서 그 자리에 있던 점수와 맞바꿨다), 오른쪽 단에 판정 수와 점수를 세로로 둔다. Graph On 과 Off 가 같은 구조이고 폭만 다르다. BGA 는 576 x 576 에서 440 x 440 이 된다. 점수는 폭이 560 미만이라 1열이다(On 3행 피치 72, Off 4행 피치 72).
- 필드 안에 겹쳐 그리는 것은 아래 표의 식 그대로다(전부 필드 x·폭과 레인 x·폭에서 계산한다). 달라지는 값은 하나다: 5키의 필드 폭이 480 이상이 되어 판정 글자가 64 로 그려진다(Normal 의 5키는 48).
- 노트·LN·지뢰 그림은 가로로만 늘어난다. 시트의 노트(4 x 8)와 지뢰는 가로 방향이 한 색이고 LN 몸통(8 x 4)은 양옆 1px 씩이 투명한 단색 띠라, 레인 폭으로 늘려도 가장자리가 흐려지지 않는다(8.4절의 실측). 그래서 시트에 넓은 판을 더하지 않았다. 폭발(128 x 128)과 리듬 띠·키 빔의 세로 크기도 그대로다.
- 곡 머리의 폭이 480 미만이면(Wide 의 9키 440, DP 352) 좁은 양식으로 그린다: 제목은 코스 중에도 상자 오른쪽 끝까지 쓰고, 단계 글자는 2행의 오른쪽 끝 128px 에 크기 18 로 놓으며, 그 자리를 비우기 위해 코스 중에는 아티스트를 그리지 않는다(op { 난이도, -290 }). 넓은 양식의 계산(제목 폭에서 224 를 뺀다)으로는 DP 의 코스 제목이 96px 밖에 남지 않기 때문이다.

필드 안에 겹쳐 그리는 것(필드 x 기준)

| 요소 | 위치 | 객체 |
| --- | --- | --- |
| 키 빔 | 레인 폭, top 624, h 280(아래 변이 판정선의 윗변 904) | image `beam`(반투명은 시트에 구움. destination 알파 255), 타이머 100+ (뗌 120+ 에 120ms 페이드 255 → 0), offset 3 |
| note | dst y 176, h 904. `group` 은 2px(top 902) | 구식 명명 10종 + mine, offset 30. `dst2` 는 쓰지 않는다(9키도 놓친 노트가 판정선에서 사라진다. 판정선 아래에 필드가 없다) |
| 레인 커버 | 필드 폭, top -904, h 904 | slider type 4, angle 2, range 904. 아래 변에 강조선 2px 과 수치 14(왼쪽)·313(오른쪽, 녹색), op 271, offset 4 |
| 히든·리프트 커버 | 필드 폭, top 904, h 904 | hiddenCover(disapearLine 176 = 판정선의 윗변, 리프트 연동), liftCover(disapearLine 168 = 필드의 아래 끝). 리프트 커버는 올라간 판정선의 윗변부터 필드 아래 끝(912)까지를 덮고 판정선은 그 위에 그려진다. 히든 수치 315 는 top 916, offsets 3·5. 리프트 수치 314 는 기준선 904 고정, op 272 |
| 판정선 연출 | 판정선 자리 | 기본 `accent_dim`. 타이머 40 에 `accent` 가 가운데에서 400ms 펼쳐진다. 리듬 띠 (top 888, h 16) image `pulse`(반투명은 시트에 구움), 타이머 140, 알파 255 → 0(500ms) |
| 폭발 | 레인 가운데, 128x128 (top 844 ~ 972). 고리의 중심은 y 908(판정선의 가운데): top = 판정선의 윗변 + (판정선 두께 - 조각 높이) / 2 | image `bomb` 8프레임, cycle 250, 가산. 타이머 50+(1회), 70+(누르는 동안), offset 3. 색은 레인의 노트 색(Bomb Color = White 면 `theme.note.white`). 온전한 고리다(웨이브 14): 아래 절반이 판정선 아래로 번져 필드 밖(912~972)의 게이지(928~952)와 발치의 캡션 위에 겹쳐 그려진다. 폭발은 250ms 짜리 효과이고 가산 합성이라 밑의 것이 비친다. 그리는 순서가 정보 영역(게이지 포함) → 필드 → 커버 → 판정선 → 폭발이라 폭발이 게이지 위에 온다(`play.build`: `info.build`, `field.under`, `field.over`). 웨이브 13 에서는 위 68행만 그려 필드 아래 끝에서 끊었는데, 반원으로 보여 사용자가 되돌려 달라고 했다 |
| 판정 + 콤보 | 글자 윗선 top 624(판정선의 윗변에서 280 위. 웨이브 13 에서 필드와 함께 64 내렸다). 가로 자리는 아래 "판정 줄" | judge(index = 필드 번호, `shift = false`, `numberAlign = 1`). 크기는 64 에서 2씩 줄여 콤보 다섯 자리가 필드 안에 드는 가장 큰 값(7키 Normal 60, 5키 Normal 46, 그 밖은 64). 콤보는 글자 바로 뒤에서 왼쪽 정렬로 다섯 자리까지. 타이머 46 에 키프레임 0 과 500 이 같은 값, `loop = -1`: 판정이 난 프레임부터 500ms 동안 그대로 서 있다가 사라진다(3.5절). PGREAT 와 GREAT 는 둘 다 "GREAT" 이고 색이 다르다(`theme.judge_word`: 하늘색, 노랑). GOOD·BAD·POOR 는 `theme.judge` 의 색. 헛침(여섯째)은 POOR 의 글자다. 콤보 숫자는 엔진이 PGREAT·GREAT·GOOD 에만 그리고 글자와 같은 색이다. 색은 시트에 구웠고 destination 의 색은 흰색이다. offsets 3·100·32 |
| 판정 위 줄 | top 584(판정 글자 바로 위), 크기 24. 가로 자리는 아래 "판정 줄" | 왼쪽 자리: 점수 차(Score Difference. Target = number 108, Best = 152). `snum24` 5자리(부호 + 4자리), `zeropadding = 1`, saturate: "+0003", "-0127", 넘치면 "+9999"·"-9999"(3.3절). 색 `text`. 조건은 타이머뿐이라 PGREAT 에도 뜬다. 오른쪽 자리: 판정 상세(Judge Detail). 905: "FAST"(op 1242, `theme.timing.fast` 파랑) / "SLOW"(op 1243, 빨강). 엔진은 이 옵션을 PGREAT 가 아닌 판정에만 켜고(레퍼런스 `getNowJudge > 1`), 오토플레이의 판정은 어긋남이 0 이라 뜨지 않는다. 906: number 525(`snum24` 4자리. 빠르면 +, 파랑. 느리면 -, 빨강. PGREAT 는 op 241 로 `sub`). 904: 그리지 않는다. 둘 다 타이머 46 에 0~500ms, `loop = -1`(판정 글자와 같은 프레임에 나오고 같은 프레임에 사라진다). offsets 3·100·33. DP 는 필드마다: 2P 는 1262/1263/261, 526, 타이머 47 |
| 로딩 | top 464 | LOADING(크기 24) + 막대 (x+48, 508, w-96, 4) graph type 102, op 80 |
| READY, FULL COMBO | top 464, 크기 44 | 타이머 40(0~600ms), 타이머 48(0~2000ms, 선은 top 520) |
| 폐점 | 화면 전체 | 타이머 3: 덮개 알파 0 → 200(400ms), FAILED 크기 96, 윗선 top 505 |
| 열기·닫기 | 화면 전체 | `parts.fade_from_black(300)`, 타이머 2 에 `parts.fade_to_black(0, 500)` |

정보 영역 안

| 영역 | 내용 |
| --- | --- |
| 곡 머리 | 폭 480 이상의 양식이다(480 미만은 (가-2)). 난이도 막대 8x56. 제목(text 12, 크기 32, 윗선 = 상자 top, 축소)은 두 벌: op { -290 } 은 x+32 부터 상자 오른쪽 끝까지(7키 On 1080, Off 1240, 9키 544, DP 728), op { 290 } 은 단계 글자 200 + 간격 24 를 뺀 폭(856, 1016, 320, 504). 2행 기준선 = 상자 아래 변, 크기 18: 난이도 이름, 레벨(`num18`, `known`), 아티스트(text 16, 오른쪽 끝까지). 오른쪽 끝 200px 에 단계 글자(크기 24, `accent`, op 280~283, 289) |
| BGA | `panel` 면. bga 는 op { 171, 41 }. 그 밖에는 안쪽 24px 에 -100(FIT_INNER, op 191) 또는 "NO BGA" / "BGA OFF"(op 190) |
| 점수 | 행: EX SCORE(`num44`, 101), RATE(102 . 103 %, `num32`), MAX COMBO(105, 5자리 saturate). 스코어 그래프가 없을 때(Graph Off, DP)만 4번째 행 TARGET(108, `snum32`). 첫 기준선 top + 32, 피치 `min(72, floor((h - 54) / (열당 행 수 - 1)))`: 7·5키 On 3행 72(기준선 802, 874, 946), Off 4행 48, DP 4행 55, 9키 66. 이름 왼쪽, 값 오른쪽 정렬. 폭 560 이상이면 2열(열당 `ceil(행 수 / 2)` 행. 9키 On 은 왼쪽 열 EX SCORE·RATE, 오른쪽 열 MAX COMBO 이고 오른쪽 아래 칸은 빈다) |
| 판정 수 | 머리행 top. 5행(`parts.judges`: PGREAT, GREAT, GOOD, BAD, POOR. IIDX 의 줄 구성이고 ModernChic 의 플레이도 다섯 줄이다. 이름은 판정 색), 피치 `min(36, floor((h - 26) / 5))`(7·5키 34, 9키·DP 36), 첫 기준선 top + 46. 합계(`num24`) 오른쪽 끝 = x + 0.52w, FAST(`num18`) = x + 0.76w, SLOW = x + w. 110~113 과 410~417 은 ref, POOR 는 헛침을 더한 값(426, 418 + 421, 419 + 422)을 Lua 함수로 4자리 이상에서 멈춘다 |
| 타이밍 | 3열. 이름 top, 값 기준선 top + 49: BPM(160, fit = 열 폭 - 24), TIME LEFT(163 : 164), TIMING(12, `snum24`). TIMING 의 표지는 image index 75(판정 타이밍 자동 보정)를 읽는 Lua 값 함수로, 켜져 있으면 "TIMING AUTO", 꺼져 있으면 "TIMING" 이다(폭 = 열 폭 - 8, 넘으면 줄어든다. 웨이브 10A: 플레이 화면도 index 75 를 답한다). TIME LEFT 의 분은 `num32` 2자리 saturate 오른쪽 정렬. 곡 길이(1163)가 9분 미만이면 분의 첫 칸을 캡션 x 왼쪽으로 한 칸(19px) 내어 한 자리 분이 캡션 x 에서 시작하고, 9분 이상이면 첫 칸이 캡션 x 에서 시작한다. hiterrorvisualizer (top + 64, h 16). 범례 (top + 86, 오른쪽 정렬, `parts.legend`: PG·GR·GD·BD·PR 을 엔진이 그래프에 쓰는 색 칩과 함께. 판정 수 표의 IIDX 색과 그래프의 고정 색이 달라서 그래프 바로 위에 둔다). judgegraph type 1 + bpmgraph (top + 106, 남은 높이) |
| 발치(SP) | 4열(폭을 넷으로 나눈다. 7키 121, 5키 94, 9키 143). HI-SPEED(text 함수 `%d.%02d`, 크기 24), GREEN(313), COVER(14), LIFT(314). 뒤의 둘은 op 270: 가림막을 바꾸는 동안만 뜨고, 켬·끔과 무관하게 뜨므로 꺼진 리프트를 START + SELECT + 턴테이블로 움직여도 값이 보인다. 웨이브 13 까지는 3열(HI-SPEED, GREEN, COVER)이었다: LIFT 칸을 더하면서 가장 좁은 5키 발치(376)에서도 네 캡션이 겹치지 않게 넷으로 나눴다(웨이브 14 통합. 5키에서 HI-SPEED 캡션 84px 이 94px 칸에 든다). 값 기준선 top + 44. AUTOPLAY(op 33)·REPLAY(op 84) top + 68 |
| 스코어 그래프 | 캡션 top. 막대 3개(w 40, 간격 8, top + 28, h = 전체 - 168): 110(`accent`), 112(`sub`), 114(`theme.tone.yellow`). BEST(152)·TARGET(108) 줄: 막대 아래 16, 84 |

판정 줄(웨이브 10C, 가로 자리는 웨이브 10D). 전부 정수 좌표다.

| 항목 | 식 | 7키 Normal(필드 64, 484) | 5키 Normal(필드 172, 376) |
| --- | --- | --- | --- |
| 크기 | 64, 62, 60, ... 가운데 콤보 다섯 자리의 오른쪽 끝이 필드 오른쪽 끝을 넘지 않는 가장 큰 값 | 60 | 46 |
| 배율 `s` | 크기 / 64 | 0.9375 | 0.71875 |
| 글자 폭 | `round(시트 폭 * s)`. 시트 폭은 잉크 폭이다: GREAT 206, GOOD 185, POOR 173, BAD 123 | GREAT 193, GOOD 173, POOR 162, BAD 115 | GREAT 148, GOOD 133, POOR 124, BAD 88 |
| 글자와 콤보 사이 | `round(크기 * 0.25)` | 15 | 12 |
| 콤보 한 칸 | `round(40 * s)` x `round(53 * s)` | 38 x 50 | 29 x 38 |
| 가운데에 두는 묶음 | GREAT 폭 + 사이 + 3칸 | 322 | 247 |
| 왼쪽 끝 `L` | `round(필드 x + (필드 폭 - 묶음 폭) / 2)` | 145 | 237 |
| 판정 글자(여섯 칸 모두) | x = `L`. BAD·POOR·헛침도 같다 | 145 | 237 |
| 콤보의 첫 자리 | x = `L` + 그 판정의 글자 폭 + 사이. 왼쪽 정렬이고 자릿수만큼만 그린다 | GREAT 353, GOOD 333 | GREAT 397, GOOD 382 |
| 다섯 자리의 오른쪽 끝 | `L` + GREAT 폭 + 사이 + 5칸. 필드 오른쪽 끝(548) 이하 | 543 | 542 |
| 위 줄(점수 차 + 상세) | 점수 차의 왼쪽 끝 = `L`(잉크 폭 73). 상세 자리는 `L` + 73 + 16 부터이고 글자·숫자는 왼쪽 정렬 | 점수 차 145~218, 상세 234 부터 | 점수 차 237~310, 상세 326 부터 |
| 위 줄에 하나만 있을 때 | Score Difference = Off 면 상세가, Judge Detail = Off 면 점수 차가 `L` 에서 시작한다 | 145 | 237 |

- 판정 글자의 왼쪽 끝은 판정 종류와 콤보에 상관없이 `L` 이고, 콤보는 그 판정의 글자 바로 뒤에서 왼쪽 정렬로 붙는다: `GREAT 7`, `GREAT 67`, `GREAT 648`, `GREAT 1234`. 자릿수가 늘면 숫자는 오른쪽으로만 자라고, 닿지 않은 자리에는 아무것도 그리지 않는다. `L` 은 "GREAT" 와 세 자리 콤보의 묶음이 필드 가운데에 오는 값이다(플레이 중 가장 오래 보이는 상태가 가운데에 온다). 넓은 레인, 5·9·10·14키, Field Side 에서도 그 필드의 x 와 폭으로 같은 식이다.
- 원본 형식의 judge 는 콤보를 그리는 자리 수만큼의 폭 안에서 언제나 가운데로 모은다(원본 로더가 숫자의 `align` 을 2 로 고정한다). 그대로는 자릿수가 늘 때마다 첫 자리가 반 칸씩 움직이므로, 엔진에 judge 의 `numberAlign` 을 더해 왼쪽 정렬로 그린다(7절, [b3 의 "웨이브 10D 반영 사항"]). 웨이브 10C 는 11칸 띠의 옅은 0 으로 다섯 자리 칸을 언제나 다 그려 자리를 고정했는데(`GREAT 00067`), 사용자 결정으로 옅은 0 을 없애고 이 배치로 바꿨다.
- 판정 글자의 스프라이트는 좌우를 잉크에 맞춰 잘랐다(`tools/sections/play.py` `judge_word`). 칸의 왼쪽 끝이 곧 잉크의 왼쪽 끝이어서 G·B·P 의 옆 여백 차이 없이 모든 판정이 같은 x 에서 시작하고, 위 줄의 왼쪽 끝과도 맞는다(위 줄의 `-` 부호는 제 칸 안에서 2px 안쪽에 놓인다).
- 크기 규칙(`field.lua` `judge_row`): 64 에서 2씩 줄이며 다섯 자리의 오른쪽 끝이 필드를 넘지 않는 첫 값을 쓴다. 폭 484(7키·14키 Normal)는 60, 폭 376(5키·10키 Normal)은 46, 폭 559 이상(Wide 전부, 9키)은 64 다.
- 숫자 칸은 폭이 같고 잉크가 칸 가운데에 있다. 그래서 글자와 첫 숫자 잉크의 사이는 첫 자리에 따라 조금 다르다(7키 Normal 에서 6 은 18px, 7 은 19px, 1 은 25px).
- `shift = false` 다. 엔진의 `shift` 는 글자를 숫자 폭의 절반만큼 왼쪽으로 민다(자릿수가 늘면 글자가 움직인다).
- 콤보 destination 의 x 는 엔진이 `w * digit / 2` 만큼 왼쪽으로 당기므로[b3 의 judge 절] 그만큼 더해 적는다: 글자 기준 x = 그 판정의 글자 폭 + 사이 + `floor(칸 폭 * 5 / 2)`.
- 세로 위치: 사용자가 정하는 Judge Position(100)이 글자·콤보·위 줄에 함께 걸린다. +100 이면 묶음 전체가 화면에서 100px 위로, -100 이면 100px 아래로 간다(창 크기와 무관하게 화면 픽셀이다. 엔진이 문서 단위로 바꿔 더한다). 콤보는 글자의 모서리 기준이라 글자를 따라간다.
- Judge Display = Hide 면 이 표의 것을 하나도 넣지 않는다.

부위(웨이브 10C. 2.3절의 불투명도 오프셋이 걸리는 단위)

| 부위(오프셋 id) | `field.lua`·`info.lua` 의 함수 | 들어 있는 것 |
| --- | --- | --- |
| Lane Background(101) | `lane_faces` | 레인마다 면 하나(`bg` 또는 `panel`. Mono 에서는 검정과 `161616`). 필드의 아래 끝(912)까지 간다: 판정선 뒤에도 면이 있어 Judge Line 을 0% 로 두면 레인이 필드 끝까지 이어져 보인다 |
| Lane Divider(102) | `lane_dividers` | 이웃한 레인 사이의 2px `line` 사각형(top 0, h 912) |
| Judge Line(103) | `judge_line` | 판정선(`accent_dim`), READY 에 펼쳐지는 `accent` 선, 리듬 띠 |
| Key Beam(104) | `key_beams` | 레인마다 누르는 동안의 빔과 뗀 뒤 120ms 페이드 빔 |
| Bomb(106) | `bombs` | 레인마다 폭발 둘(치는 순간, 누르는 동안). 필드 부위 가운데 이것만 판정선 아래로 내려간다 |
| Note(107) | `note_field` 의 note | note 객체 하나(전 레인의 노트·LN·지뢰) |
| Bar Line(108) | `note_field` 의 `group` | 필드마다 마디선 destination |
| Lane Cover(109) | `covers` | 레인 커버(slider), 히든·리프트 커버, 커버 끝의 강조선, 커버 수치와 녹색 숫자, 리프트·히든 수치 |
| Gauge(110) | `gauge` | gauge 객체와 수치(정수, 점, 소수) |
| Progress Bar(111) | `progress` | 필드 옆 4px 진행 막대(바탕과 채움) |
| Song Header(112) | `head` | 난이도 막대, 제목, 난이도 이름·레벨·아티스트, 단계 글자 |
| BGA(113) | `bga` | `panel` 면, bga, 스테이지 그림, "NO BGA"·"BGA OFF" |
| Score Graph(114) | `pace` | 캡션, 막대 3개와 바탕, BEST·TARGET 줄 |
| Score(115) | `score` | EX SCORE, RATE, MAX COMBO, (TARGET) |
| Judge Count(116) | `counts` | 머리행과 5행 |
| Timing(117) | `timing` | BPM, TIME LEFT, TIMING 과 히트 에러 띠 |
| Distribution Graph(118) | `distribution` | 범례, 그래프 바탕, judgegraph, bpmgraph |
| Speed Display(119) | `foot` | HI-SPEED, GREEN, COVER, LIFT, AUTOPLAY·REPLAY |

- 기본값(전부 100%)의 화면은 부위를 나누기 전과 같다. 나누면서 바뀐 것은 필드 바탕 하나(64, 0, 484, 912 의 `line` 사각형 한 장)를 레인 사이의 선(레인 구분선)과 건반 표시 바탕으로 쪼갠 것뿐이고, 같은 픽셀에 같은 색이 놓인다. 통합에서 건반 표시 바탕을 한 번 더 쪼갰다(판정선 자리의 띠와 건반 사이의 선. `field.lua` 의 `seams` 가 레인 구분선과 같은 틈을 낸다): 건반 면 밑에 바탕이 깔려 있으면 50% 에서 두 겹이 비쳐 89% 로 보였다. 반투명을 시트와 색에 구운 것(2.3절)까지 합쳐 100% 의 화면은 키 빔·리듬 띠·눌림 빛에서 채널 값 2 이내로만 달라졌다(8.4절). 이 항목은 웨이브 10C 의 기록이다: 건반 표시와 그 바탕은 웨이브 13 에서 없앴고, 지금은 레인 면과 레인 구분선이 필드의 아래 끝까지 간다.
- BAD·POOR 를 가운데에 둔 것은 의도다. 이 둘은 콤보 칸이 없어서 묶음의 왼쪽 끝에 두면 필드 왼쪽에 치우친 글자 하나만 남는다. 판정 종류마다 자리는 고정이고(같은 판정은 언제나 같은 자리), PGREAT·GREAT·GOOD 와 BAD·POOR 사이에서만 글자의 시작 열이 다르다(7키 148px, 124px). 세로 자리와 높이는 같다.
- 사용자의 예 "레인 선, 레인 배경, 몇몇 UI 를 0%, 노트·폭발·판정선은 100%": 101·102 와 원하는 UI 부위를 0% 로 두면 필드가 화면 바탕(`bg`) 위에 노트, 키 빔, 폭발, 판정선, 판정 글자만 남는다(8.4절).

(나) 객체

| 객체 | 종류 | 주요 필드 | 자산 |
| --- | --- | --- | --- |
| 노트 필드 | note | `note`, `lnstart`, `lnend`, `lnbody`, `lnactive`, `hcnstart`, `hcnend`, `hcnbody`, `hcnactive`, `hcndamage`, `hcnreactive`, `mine` 을 레인 수만큼(구식 명명으로 통일. LN 10종 배열은 전부 `lnend` 길이 이상이어야 한다[b3 §3.2]). `size` 는 레인마다 16(9키 24). `dst` 는 레인 사각형. `group` 은 마디선(2px, `line`). destination 에 `offset = 30` | `play` 의 노트 |
| 게이지 | gauge | `nodes` 6종 x 6상태(36칸 규칙[b3 §5]), `parts = 50`, `type = 0`, `range = 3`, `cycle = 33` | `play` 의 `gauge_<종류>_<상태>` |
| 판정 | judge(index 0) | `images` 6개(타이머 46, `loop = -1`, 키프레임 0·500 이 같은 값. image 는 한 칸이고 여섯째는 다섯째와 같은 image), `numbers` 6개(콤보. value 는 11칸 띠, `digit = 5`. 넷째부터는 엔진이 그리지 않는다), `shift = false`. Judge Display = Hide 면 없다 | `play` 의 `judge_*`, `combo_*` |
| BGA | bga | op { 171, 41 } | - |
| 커버 | slider(type 4), liftCover, hiddenCover | `disapearLine`: 히든은 판정선의 윗변(리프트 연동), 리프트는 필드의 아래 끝 | `img.block` |
| 폭발, 키 빔, 리듬 띠 | image(타이머 50~59, 70~79, 100~109, 140) | 폭발 `cycle = 250`, blend 2 | `play` 의 `bomb`, `beam`, `pulse` |
| 그래프 | graph(type 101, 102, 110, 112, 114), judgegraph(type 1), bpmgraph(`parts.bpm_graph()`), hiterrorvisualizer(색은 `theme.hex` 로 팔레트에서) | - | `play` 의 `bar_h`, `bar_v` |
| 숫자 | value | 위 표의 ref. 점수 차(108 또는 152)는 `snum24`, `zeropadding = 1` | `common` |
| 글자 | text(ref 12, 16), 난이도 부품, READY·FAILED·FULL COMBO·LOADING·FAST·SLOW | - | 폰트 |
| 오프셋 | 헤더의 `offset` | 2.3절의 20개. destination 마다 `offsets` 에 부위 id 하나 | - |

(다) 애니메이션: 장면 시작 덮개 300ms. PRELOAD 동안 로딩 막대. READY(타이머 40)에 판정선이 가운데에서 양쪽으로 400ms 펼쳐지고 "READY" 가 600ms 뒤 사라진다. 판정 글자·콤보와 그 위 줄은 타이머 46 에 500ms 동안 가만히 서 있다가 사라진다(움직임·깜빡임·색 돌림·페이드 없음. 3.5절). 리듬 타이머(140)에 판정선 위 띠가 알파 120 → 0. 게이지 최대(타이머 44)에 게이지 숫자가 `accent`.

(라) 입력: 클릭 가능한 객체 없음. 레인 커버·하이스피드 조작은 엔진이 한다.

(마) 뺀 것: 상세 모드, 공격 모션, 구역 점수와 음성, 캐릭터, 점수 플랩, 레인 커버 로테이션, 이력 기록, BGA 배치 변종과 동영상 배경, 노트·판정·폭발의 그림 변종(색만 옵션으로 고른다. 3.1절), 스크래치 회전판 그림(평면 사각형으로 대신한다), 타이밍 조정 클릭 영역, 리프트 표시 토글, 판정선 아래의 건반 표시(웨이브 13. 판정선이 필드의 맨 아래에 붙는다).

### 6.4 플레이 DP(type 2, 3) — 구현 완료

구현: `play14.luaskin`, `play10.luaskin`. 같은 `play.build(spec)` 를 `{ keys = 14 }`, `{ keys = 10 }` 으로 부른다. 필드가 둘이고 가운데에 정보가 놓인다. 필드의 안쪽 변은 548 과 1372 로 고정이다. destination 은 play10 232, play14 256 개이고 프레임마다 부르는 Lua 함수는 16개다(기본 옵션).

| 영역 | 14키 | 10키 |
| --- | --- | --- |
| 1P 필드(스크래치 왼쪽) | 64, 0, 484, 912 | 172, 0, 376, 912 |
| 2P 필드(스크래치 오른쪽) | 1372, 0, 484, 912 | 1372, 0, 376, 912 |
| 판정선 | 각 필드, top 904(h 8). 아래 변이 필드의 아래 끝이다(6.3절) | 같음 |
| 진행 막대 | 556 과 1360, 0, 4, 912 | 같음 |
| 발치(필드마다) | 필드 x, 928, 필드 폭, 152 | 같음 |
| 곡 머리 | 580, 40, 760, 56 | 같음 |
| BGA | 580, 120, 760, 428 | 같음 |
| 게이지 | 580, 572, 760, 24 (막대 650, 칸 13px). 하나뿐이다(2P 게이지 없음[b3 §12]) | 같음 |
| 점수 | 580, 620, 368, 220 (4행, 피치 55) | 같음 |
| 판정 수 | 972, 620, 368, 220 (5행, 피치 36) | 같음 |
| 타이밍·분포 | 580, 864, 760, 152 (범례 top 950, 그래프 top 970 h 46) | 같음 |

Lane Width 가 Wide 일 때(웨이브 10A). 두 필드가 각각 719(14키)·559(10키)로 넓어져 가운데가 좁아진다. 바깥 여백을 64 에서 32 로 줄여 필드의 안쪽 변을 751 과 1169 에 두면 가운데에 폭 352 의 단 하나가 남는다(진행 막대와의 간격 21).

| 영역 | 14키 | 10키 |
| --- | --- | --- |
| 1P 필드(스크래치 왼쪽) | 32, 0, 719, 912 | 192, 0, 559, 912 |
| 2P 필드(스크래치 오른쪽) | 1169, 0, 719, 912 | 1169, 0, 559, 912 |
| 진행 막대 | 759 와 1157, 0, 4, 912 | 같음 |
| 게이지 | 1P 필드 아래: 32, 928, 600, 24 (칸 12px), 수치 오른쪽 끝 751 | 192, 928, 450, 24 (칸 9px) |
| 발치(필드마다) | 필드 x, 968, 필드 폭, 112 | 같음 |
| 곡 머리 | 784, 40, 352, 56 (좁은 양식. 6.3절 (가-2)) | 같음 |
| BGA | 784, 120, 352, 264 | 같음 |
| 점수 | 784, 408, 352, 198 (4행, 피치 48) | 같음 |
| 판정 수 | 784, 630, 352, 206 (5행, 피치 36) | 같음 |
| 타이밍·분포 | 784, 860, 352, 156 (범례 top 946, 그래프 top 966 h 50) | 같음 |

- 14키 레인 x: 1P 는 S 32, 건반 184, 273, 344, 433, 504, 593, 664. 2P 는 건반 1169, 1258, 1329, 1418, 1489, 1578, 1649, S 1738. 10키: 1P 는 S 192, 건반 344, 433, 504, 593, 664. 2P 는 건반 1169, 1258, 1329, 1418, 1489, S 1578.
- 무엇을 줄였나: 가운데의 것을 전부 한 단에 세로로 쌓고(Normal 은 점수와 판정 수가 옆으로 나란하다), BGA 를 760 x 428 에서 352 x 264(4:3)로 줄였다. BGA 는 엔진이 비율을 지켜 상자 안에 맞추므로(`BgaExpand::KeepAspectRatio`) 정사각형 BGA 는 264 x 264 로 보인다.
- 무엇을 옮겼나: 게이지. 폭 352 에서는 막대가 250(칸 5px)밖에 되지 않아, SP 와 같이 1P 필드 아래(top 928)로 내리고 두 필드의 발치를 SP 와 같은 top 968 로 내렸다. 게이지는 하나뿐이므로 2P 필드 아래의 그 줄은 빈다.
- 숨긴 것은 없다. 점수 4행, 판정 수 5행과 FAST·SLOW, BPM·남은 시간·타이밍, 히트 에러 띠, 범례, 분포 그래프가 전부 남는다(판정 수와 타이밍의 폭 352 는 SP Normal 의 352·344 와 같다).
- 10키도 같은 가운데 단을 쓴다(Normal 과 같은 규칙: 필드의 안쪽 변이 키 수와 무관하게 고정). 10키는 바깥 여백이 192 씩 남지만 BGA 의 크기는 단의 폭이 아니라 세로로 쌓인 높이(264)가 정하므로, 단을 넓혀도 정사각형 BGA 는 커지지 않는다.

SP 와 다른 점

| 항목 | DP |
| --- | --- |
| note | 한 객체에 전 레인. 14키는 [1P 건반 1~7, 1P 스크래치, 2P 건반 1~7, 2P 스크래치] 16칸, 10키는 12칸. `group` 은 필드 수만큼 2개 |
| judge | index 0(타이머 46), index 1(타이머 47) 두 객체. 필드마다 하나 |
| 타이머 | 2P 쪽은 60~69(폭발), 80~89(홀드), 110~119(키 온) |
| 커버 | 같은 slider·커버 정의를 두 필드에 한 번씩 배치 |
| 발치 | 1P 발치에 HI-SPEED(1열)와 GREEN(2열)만, 2P 발치에 AUTOPLAY/REPLAY(위 행, 필드 x)와 COVER·LIFT(3·4열, op 270)만 그린다. 같은 값은 한 번만 나온다. 자동 플레이·리플레이가 아니고 커버를 조작하지 않을 때 2P 발치는 빈다 |
| 판정 위 줄 | 점수 차, FAST/SLOW 와 ms 숫자는 친 쪽 필드에만 뜬다(1P 는 op 1242/1243·number 525·타이머 46, 2P 는 1262/1263·526·47). 점수 차의 값은 두 필드가 같다(런 하나의 값이다) |
| 판정 줄의 자리 | 필드마다 6.3절의 식 그대로(14키 Normal 은 크기 60, 1P 의 `L` 145, 2P 는 1453. 10키 Normal 은 크기 46. Wide 는 크기 64) |
| 옵션 | Lane Width, Judge Detail, Score Difference, Judge Display. Field Side, Scratch Side, Score Graph 없음(점수 영역에 TARGET 행이 들어간다) |
| 오프셋 | Score Graph Opacity(114)가 없다. Judge Position 과 부위 불투명도는 두 필드에 같이 걸린다 |

### 6.5 결과(type 7) — 구현 완료

구현: `result.luaskin`, `lua/result/main.lua`(`result.build({ course = false })`: 조립, 곡 머리, 안내), `layout.lua`(열과 행의 계산, 등장 순서), `score.lua`(랭크와 점수), `judges.lua`(판정 표), `graphs.lua`(오른쪽 열). destination 147개, 프레임마다 부르는 Lua 함수 7개(막대 5, 게이지 이름 1, 게이지 값 유무 1. 런에 빠름·느림 값이 있으면 POOR 행의 합 2개가 더해진다. 게이지 값 유무 함수는 destination 2개에 걸려 프레임마다 9회 호출된다).

헤더: `input 500`, `scene 3600000`, `fadeout 500`, 사용자 옵션 없음. 왼쪽 열 x 64 w 1032(`theme.column(0, 7)`), 오른쪽 열 x 1128 w 728(`theme.column(7, 5)`).

행(이름 top / 값 기준선): 1행 336 / 411(`num64`), 2행 437 / 488(`num32`), 3행 514 / 574(`num44`), 4행 600 / 651(`num32`). 행 사이(값 기준선에서 다음 이름까지)는 모두 26. `layout.row` 가 곡 머리 아래 변(304) + 32 에서 차례로 계산한다. 랭크 칸과 점수 칸이 같은 행을 쓴다.

(가) 영역

| 영역 | x | top | w | h | 내용 |
| --- | --- | --- | --- | --- | --- |
| 머리 | 64 | 70 | 1792 | 36 | 캡션 "RESULT", 구분선 top 104. 오른쪽 끝은 비워 둔다(앱의 IR 오버레이 x 1726~1904, y 18~47) |
| 곡 머리 | 64 | 136 | 1032 | 168 | 난이도 막대 (64, 136, 8, 168). 판정 글자 (104, 136) 크기 24. "SCORE NOT SAVED" 오른쪽 정렬 top 140 크기 18. 제목 TITLE (104, w 992) 크기 64: 부제가 없으면 top 184, 있으면 top 166. 부제 SUBTITLE (104, top 234) 크기 32 `sub`, 폭은 아티스트와 같다. 기준선 296 줄: 아티스트(크기 24, `text`), 난이도 이름(크기 24, 폭 136 오른쪽 정렬, 레벨 잉크 12px 앞에서 끝남), 레벨 `num32`(상자 872 w 80 오른쪽 정렬), 키 수 (976, w 120) 오른쪽 정렬 |
| 랭크 | 64 | 336 | 424 | 315 | "RANK". 글자 크기 160, top 372(기준선 488 = 2행 값 기준선). 3행 "CLEAR" + 램프 이름 크기 44(`theme.clear` 의 이름과 대표색. 폭 424 가로 축소). 4행 왼쪽 절반 (64, w 196) "TO NEXT RANK" / "TO MAX" + `num32`, 오른쪽 절반 (292, w 196) "PREVIOUS CLEAR" + 이름 크기 24 |
| 점수 A | 520 | 336 | 272 | 315 | 1행 EX SCORE `num64`. 2행 BEST `num32` + 차 `snum32`(간격 16). 3행 MAX COMBO `num44` + 차 `snum24`. 4행 RANDOM 크기 24 |
| 점수 B | 824 | 336 | 272 | 315 | 1행 RATE: 정수 `num64`, 점 6x6, 소수 `num44` 2자리, "%" 크기 32. 2행 "TARGET" (x 824) + 이름 (x 906, w 190, 가로 축소) + `num32` + 차 `snum32`. 3행 MISS COUNT `num44` + 차 `snum24`. 4행 REPLAY 크기 24 |
| 구분선 | 64 | 666 | 1032 | 2 | `line`. 4행 값 기준선 + 15 |
| 판정 표 | 64 | 684 | 1032 | 284 | 머리 캡션 top 684: JUDGE(x 64), COUNT(상자 208 w 120), FAST(344 w 96), SLOW(456 w 96) 오른쪽 정렬, SHARE OF NOTES(x 584), 오른쪽 끝에 "TOTAL NOTES" + `num18`(x 1096 오른쪽 정렬). 판정 5행 top 714 + 38i(i 0~4), 기준선 top + 26: 이름 크기 24(판정 색, 폭 136 가로 축소. PGREAT, GREAT, GOOD, BAD, POOR), 숫자 `num24`. 막대 (584, top + 14, 512, 10). 여섯째 줄(top 904, 기준선 930)은 판정이 아닌 집계다: "COMBO BREAK"(x 64, 크기 18, `sub`) + 개수(COUNT 칸), 막대 자리에 "EMPTY POOR"(x 584, 크기 18, `sub`, 폭 112) + 개수(x 708 왼쪽 정렬, `sub`). 합계 선 (344, 944, 208, 2), "TOTAL EXCL. PGREAT" 과 423·424 기준선 967 |
| 게이지 | 1128 | 136 | 728 | 286 | 캡션 "GAUGE"(top 136), 게이지 이름 (1208, 136), 수치 오른쪽 끝(정수 3자리, 점 2x2, 소수 1자리, "%"). 그래프 (1128, 164, 728, 258) |
| 판정 추이 | 1128 | 446 | 728 | 158 | 캡션 "JUDGES OVER TIME", "+ BPM"(x 1320), 범례 오른쪽 정렬. 그래프 (1128, 474, 728, 130) |
| FAST/SLOW 추이 | 1128 | 628 | 728 | 158 | 캡션 "FAST / SLOW OVER TIME", 범례. 그래프 (1128, 656, 728, 130) |
| 타이밍 분포 | 1128 | 810 | 728 | 158 | 캡션 "TIMING DISTRIBUTION", 통계 x 1360 부터(기준선 823). 그래프 (1128, 838, 728, 130) |
| 바닥 | 64 | 1022 | 1792 | 18 | 안내 (64, w 1792). 구분선 top 1000. IR 상태 문구 없음 |

(나) 객체

| 객체 | 종류 | 값·조건 | 자산 |
| --- | --- | --- | --- |
| 판정 글자 | text | op 90 "STAGE CLEARED"(`accent`), op 91 "STAGE FAILED"(`theme.tone.red`) | 폰트 |
| 저장 안 됨 | text | op 60 | 폰트 |
| 제목, 부제, 아티스트 | text(ref 10, 11, 16) | overflow 1. 부제는 본체 패스에서 text 11 이 비어 있지 않을 때만 | 폰트 |
| 레벨 | value(ref 96) | `known`, `fit` 80, op 150~155 로 색 6벌 | `num32` |
| 랭크 | text 8벌(`theme.rank_name`) | op 300~307, F 는 `muted`. 타이머 150 에 200~500ms 등장 | 폰트 |
| 램프, 이전 램프 | text(상수) | 본체 패스의 `event_index(370)`, `event_index(371)` 로 `theme.clear` 에서 고른다 | 폰트 |
| 다음 랭크 | value(ref 154) | `known`, `fit` 196. 이름은 op -300 / op 300 | `num32` |
| EX, RATE | value(ref 171, 102, 103) | 171·102 는 `known`. 103 은 digit 2, `padding = 1` | `num64`, `num44` |
| BEST, TARGET | value(ref 170 + 172, 121 + 153) | 차는 `digit = 자릿수 + 1`. 둘이 칸을 넘으면 같은 비율로 줄인다. TARGET 은 text 3 이 비어 있지 않을 때만, 이름은 대문자로 바꿔 overflow 1 | `num32`, `snum32` |
| 콤보, 미스 | value(ref 174 + 175, 177 + 178) | 값 없음이면 차를 그리지 않는다. 차의 색: 나아짐 `accent`, 나빠짐 `theme.tone.orange`, 0 `sub` | `num44`, `snum24` |
| NEW RECORD | text | op { 61, 330 / 331 / 332 }, 타이머 152 에 400~700ms. 램프는 본체 패스에서 "FAILED 초과이고 이전보다 큼" | 폰트 |
| RANDOM | text(상수, `theme.random_name`) | `event_index(42)` | 폰트 |
| REPLAY | text 3벌 | op 198 "SAVED"(`accent`), 197 "STORED"(`sub`), 196 "NOT SAVED"(`muted`). 슬롯 2~4 는 엔진이 언제나 "없음"으로 답하므로 두지 않는다 | 폰트 |
| 판정 수 | value(ref 110~113, 426, 410~417, 418 + 421, 419 + 422, 425, 420, 423, 424) | `known`, `fit`, 오른쪽 정렬. 행은 `parts.judges` 다섯과 `parts.combo_break`, `parts.empty_poor`. IIDX 처럼 POOR 가 헛침을 포함하고(426), 콤보가 끊긴 수 COMBO BREAK(425 = BAD + POOR)를 따로 적는다. 엔진이 따로 세는 헛침(420)은 ModernChic 이 결과 표에만 "MISS" 줄로 두는 것을 따라 결과에만 두되, POOR 에 이미 들어 있는 수라 이름을 "EMPTY POOR" 로 적어 판정 줄과 구별한다. id 가 둘인 칸은 Lua 함수가 더한다 | `num24` |
| 총 노트 | value(ref 74) | `known`, 오른쪽 정렬, `sub` | `num18` |
| 판정 막대 | graph(`angle = 0`, `value` = Lua 함수) | 판정 수(표의 숫자와 같은 합) / number 74, 0~1 로 자름, 0 이 아니면 최소 2/512. 색은 판정 색. 타이머 150 에 500~1000ms 자람. 비율 type 140~144 는 선곡 전용이라 결과에서는 0 이다(`skin_host/score.rs` `rate`, [b5 §5]) | `result` 의 `bar` |
| 게이지 그래프 | gaugegraph(색 14개를 `theme.hex` 로 팔레트에서) | - | 없음 |
| 게이지 이름 | text(`value` = `parts.choice`) | `event_index(40)` → `theme.gauge_name` | 폰트 |
| 게이지 수치 | value(ref 107, 407) | digit 3 오른쪽 정렬, digit 1. 소수점과 "%" 는 draw 함수(number 107 이 값 없음이 아닐 때) | `num18` |
| 판정 추이 | judgegraph(type 1, backTexOff 1) + bpmgraph(op 177) | - | 없음 |
| FAST/SLOW 추이 | judgegraph(type 2, backTexOff 1) | - | 없음 |
| 범례 | image(-111 에 엔진 고정 색) + text(`parts.legend`) | op -164 / op 164 두 벌(9키는 엔진의 팔레트가 다르다). 판정 표의 색(IIDX)과 그래프의 색(엔진 고정)이 다르므로 그래프마다 제 캡션 줄에 범례를 둔다. ModernChic 은 판정 표를 무채색으로 두고 범례가 없다 | 폰트 |
| 타이밍 분포 | timingdistributiongraph(색 8개. 판정 폭의 띠 5색은 범례의 색을 `panel` 에 10% 섞은 것) | - | 없음 |
| 타이밍 통계 | text + value(ref 374, 375, 376, 377) | 부호는 "AVG FAST" / "AVG SLOW" / "AVG" 글자로. 값 없음이면 "NO JUDGED NOTES" | `num18` |
| 안내 | text 6벌 | 타이머 1 에 200ms 등장. 기본 "ENTER  CONTINUE        RETRY AND NEXT CHART  THEIR KEYS IN KEY CONFIG (R AND N AS SHIPPED)"(웨이브 10A: 두 키를 옮길 수 있게 되어 키 이름을 단정하지 않는다). op 160, 162, 164(7·14·9키) 각각에서는 뒤에 "        KEY 6  GAUGE", op 161, 163(5·10키) 각각에서는 "        TURNTABLE  GAUGE" 를 붙인 문구, 다섯 다 아니면 기본 문구 | 폰트 |
| 덮개 | image | `parts.fade_from_black(300)`, `parts.fade_to_black(0, 400)` | - |

(다) 애니메이션: 장면 시작 덮개 300ms. 구역별 300ms 등장을 100ms 간격으로(곡 머리 0, 랭크 100, 점수 200, 판정 표와 구분선 300, 오른쪽 열 400). 난이도 막대 0~350ms 자람, 제목은 왼쪽 32px 에서 들어오고 부제는 50ms 늦게 같은 방식. 게이지 그래프는 엔진이 1500ms 에 걸쳐 드러낸다. FADEOUT 에 `parts.fade_to_black(0, 400)`.

(라) 입력: 클릭 가능한 객체 없음. 다음(건반 1~4, Enter), 게이지 종류 전환, 다시 하기(R), 다음 곡(N)은 엔진이 한다[b4 §5.3, 사양 §4.5]. 게이지 전환은 7·14·9키에서 여섯째 건반, 5·10키에서 턴테이블을 앞으로 돌리는 것이다(`apps/rbms-player/src/stage/result.rs` `key_assignment`. 7절).

(마) 뺀 것: 클리어·실패 배경 그림과 캐릭터, 고리·빔 장식, IR 메뉴와 순위 목록, 인프레션 계산기, 플레이 이력, 난이도 목록 버튼, 메뉴 1·2 전환(전부 한 화면에 놓는다), 리플레이 슬롯 2~4 의 저장 표지와 저장 버튼, 시작 연출의 파도·띠. R·N 안내는 다시 하기·다음 곡이 불가능한 상황에서도 나온다(결과에서 그 가능 여부를 알리는 속성이 없다).

### 6.6 코스 결과(type 15) — 구현 완료

`courseresult.luaskin` 이 `result.build({ course = true })` 를 부른다. destination 84개. 결과와 같은 배치에서 아래만 다르다.

| 항목 | 코스 결과 |
| --- | --- |
| 머리 캡션 | "COURSE RESULT" |
| 곡 머리 | 막대는 `accent`(op 90) / `theme.tone.red`(op 91). 판정 글자는 "COURSE CLEARED" / "COURSE FAILED". 제목은 코스 이름(text 10, top 184). 기준선 296 줄은 "N STAGES"(크기 24, `sub`. 단계 제목이 전부 비면 그리지 않는다)와 키 수 |
| 점수 | 2행 A 는 TOTAL NOTES `num32`, 2행 B 는 RANDOM. 4행은 랭크 열의 TO NEXT RANK 만. BEST, TARGET, 차, NEW RECORD, REPLAY, PREVIOUS CLEAR 없음 |
| 판정 표 | 오른쪽 끝의 "TOTAL NOTES" 없음(점수 2행에 있다) |
| 게이지 그래프 | 단계별 이력을 이어 붙인 것(엔진이 공급[b4 웨이브 4 반영 사항]). 곡 경계는 엔진이 그린다. 높이는 386 + (10 - n) x 36(n = 그리는 단계 줄 수. 3단계 638, 10단계 386) |
| 단계 곡명 | (1128, 게이지 그래프 아래 변 + 32, 728) 캡션 "STAGES". 줄은 캡션 top + 28 + 36i(최대 10줄): 번호(x 1128, `muted`), 곡명 (1176, w 680, text 150~159) 크기 24. 마지막으로 비어 있지 않은 줄까지 그리고 빈 제목은 번호만. 3단계면 캡션 top 834, 10단계면 582 |
| 오른쪽 열의 나머지 | 판정 추이, FAST/SLOW 추이, 타이밍 분포 없음 |
| 안내 | 기본 "ENTER  CONTINUE"(다시 하기·다음 곡 없음). 게이지 전환 문구는 결과와 같은 규칙으로 붙는다 |

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
| `-100` 스테이지 파일과 stretch 1 | `skin_render/refs.rs`, `loader/stretch.rs` | 검증됨. 4:3 그림을 4:3 상자(결정, 선곡)와 넓은 상자(플레이의 BGA 자리 1064x578 안에 770x578 로)에 넣는 캡처로 확인 |
| op 190~195(스테이지 그림·배너·배경 그림 유무) | `apps/rbms-player/src/skin_host/chart.rs` `read_boolean`, 선곡은 `stage/select/list.rs` `chart_under_cursor` | 결정·플레이·결과는 로드한 차트의 그림으로 답한다. 선곡은 커서 곡의 그림이 그 프레임에 로드돼 있는지로 답한다(레퍼런스의 `MusicSelector.loadSelectedSongImages` 와 같다. 통합에서 고쳤다: 아래 표). 그림을 읽는 동안에는 "없음"이므로 선곡의 문구는 300ms 늦게 뜬다(6.2절) |
| 리플레이 슬롯 4개 | `skin_host/select.rs` `REPLAY_OPTIONS`(유무 197/1197/1200/1203, 없음 196/1196/1199/1202, 선택 1205~1208), `stage/select/events.rs`(act 19, 316~318 → 그 슬롯 재생), `skin_host/result.rs` `REPLAY_OPTIONS`(결과: 슬롯 1 만 상태가 바뀐다) | 엔진 테스트로 검증됨. 기본 스킨은 선곡(칸 4개)과 결과(슬롯 1 의 상태)가 쓴다. 리플레이가 있는 슬롯과 선택 밑줄은 캡처로 보지 못했다 |
| value 의 `value` 에 Lua 함수(3.3절의 `saturate`) | `skin_render/object.rs` `ValueSource::Function`, `draw.rs` `number_value` | 검증됨. 시험 팩으로 재확인(아래) |
| 코스 단계 op 280~283, 289, 290 | `skin_host/system.rs`(시스템 클러스터가 답한다) | 엔진 테스트로 검증됨. 플레이의 곡 머리가 쓴다. 실제 코스 런의 단계 글자는 캡처로 보지 못했다 |
| value 10칸 시트, 왼쪽 정렬, 색, `filter = 1` | `skin_render/object.rs`, `draw.rs` | 검증됨 |
| value 24칸 부호 시트 | 같음 | 검증됨. `snum24`·`snum32` 를 플레이(TARGET, BEST, TIMING)와 결과(차)가 쓰고 캡처로 확인 |
| judgegraph type 0, bpmgraph(색 지정) | `skin_render/graphs/notes_dist.rs`, `bpm.rs` | 검증됨 |
| 타이머 1·2, `loop`(멈춤, -1), `acc` | `dst.rs` `resolve` | 검증됨 |
| text 의 `value` 에 Lua 함수, `event_index` | `text.rs`, `main_state.rs` | 검증됨. 선곡(옵션 값, BPM 범위, 길이, RATE), 플레이(HI-SPEED), 결과(게이지 이름)가 쓰고 캡처로 확인 |
| image 의 `act`(id, Lua 함수), `mouseRect`, 슬라이더 드래그 | `skin_render/input.rs`, `dst.rs` | 검증됨 |
| songlist | `skin_render/songlist/` | 검증됨 |
| note, gauge, judge, bga, 커버, 오프셋 3·4·5·30·32·33 | `skin_render/notes.rs`, `gauge.rs`, `judge.rs`, `bga.rs`, `covers.rs` | 5/7/10/14키 검증됨 |
| judge 의 `numberAlign`(콤보 숫자의 정렬. 원본에 없는 R-BMS 확장) | `crates/rbms-skin/src/model/objects.rs` `JudgeDef.number_align`, `skin_render/judge.rs` `draw_count` | 기본 스킨이 처음 쓴다(웨이브 10D). 0 = 오른쪽, 1 = 왼쪽, 그 밖 = 가운데이고 쓰지 않으면 원본대로 가운데다. 엔진 테스트와 캡처로 확인. 같은 문서를 beatoraja 로 열면 콤보가 가운데 정렬로 그려진다 |
| gaugegraph, judgegraph type 1·2, timingdistributiongraph | `skin_render/graphs/` | 검증됨 |
| 9키 문서(type 4) | `loader.rs` `PLAY_SKIN_MODES`, `apps/rbms-player/src/stage/play/mod.rs`(`mode_skin_type(shared.mode)`), `skin_host/play_timers.rs` `POPN_9K_KEYS` | 기본 스킨이 처음 쓴다. 캡처 하니스에 9키 런을 넣어 그리기까지 확인(`play9-play-09000ms`): 9레인이 가운데, 노트·키 빔·폭발·판정이 제 레인에 |
| note 의 `dst2`, `expansionrate` | `skin_render/notes.rs` `fall_to`, `swell` | `dst2 = 168` 을 9키 문서가 쓰고 로드·그리기 확인. 놓친 노트가 하한까지 떨어지는 장면은 캡처로 보지 못했다(하니스의 9키 런은 자동 플레이다). `expansionrate` 는 기본값이라 쓰지 않는다 |
| 스킨 사운드 `main_state.audio_play("sound/*.wav")` | `apps/rbms-player/src/skin_host/audio.rs` | ogg 로 검증됨. 선곡의 사이드 메뉴가 wav 두 개를 쓴다. 호출이 실패 없이 도는 것까지 확인했고(덤프의 함수 실패 0) 소리는 하니스로 확인하지 못했다 |

처음 쓰이는 기능의 시험 결과(시험 팩은 저장소 밖 임시 폴더에 두었다)

| 시험 | 방법 | 결과 |
| --- | --- | --- |
| 문서 type 과 화면 대응 | type 6 문서와 type 4 문서를 한 팩에 두고 `rbms-cli skin-dump <팩> --strict --frames 30` | type 6 은 "decide", type 4 는 "play 9 keys" 로 읽힘. 둘 다 load ok, pcall 0, 경고 0 |
| 9키 로드 | type 4 문서에 9레인 note(LN·HCN·지뢰 배열 포함), gauge, judge 를 넣어 덤프 | 자동 오프셋 4개가 붙고 `image 1, note 1, gauge 1, judge 1` 로 조립됨. 경고 0 |
| 9키 그리기 | 캡처 하니스의 9키 런(`mode_chart` 에 채널 11~15, 22~25) | `play9-play-09000ms`: 필드 (673, 0, 574, 840) 에 9레인, 판정선과 건반 표시, 왼쪽에 곡 머리·BGA·점수, 오른쪽에 스코어 그래프·판정 수·타이밍 |
| 폰트 경로 | `font/Inter-Regular.ttf` 를 선언한 text 와, 선언하지 않은 폰트 id 를 쓰는 text 를 결정 장면 캡처로 비교 | 둘 다 그려진다. 선언한 쪽만 정식 경로다(4.6절). 선언하지 않은 쪽에 덤프 경고는 없다(아래 보완 3) |
| `require` 와 Lua 5.2 | 모듈이 헤더 패스에서 한 번만 실행되는지(`skin_config == nil` 을 모듈 최상위에서 기록), `7 / 2`, `_VERSION` 을 화면에 찍음 | `true`, `3.5`, `Lua 5.2` |
| 공통 모듈 | `ui`·`motion`·`parts` 의 좌표 변환, 정렬, 키프레임, 호버를 Lua 단언으로 검사하는 문서를 덤프 | load ok(단언 18개 통과) |
| 숫자의 자릿수 규칙 | 팩 사본에 `saturate`, `known`, `fit`, 오른쪽 정렬을 쓰는 문서를 두고 최대 BPM 655350, 레벨 123 인 시나리오로 덤프와 캡처 | 4자리 `saturate` 는 "9999"(붙이지 않으면 "5350"), 8자리는 "655350", 부호 띠 3자리는 "+99", 5자리는 "+123". 왼쪽 정렬 잉크는 안내선의 x 에서, `known` 오른쪽 정렬 잉크는 상자 오른쪽 끝에서 끝난다. 함수 값 8개, 실패 0, pcall 0 |

통합(웨이브 8B)에서 고친 엔진 결함. 둘 다 레퍼런스의 해당 줄을 확인하고 고쳤고 재현 테스트가 붙어 있다.

| 결함 | 원인 | 고친 것 | 테스트 |
| --- | --- | --- | --- |
| 5키·10키의 결과 화면에서 게이지 그래프를 바꿀 입력이 없었다(다시 하기 "같은 배치"도) | 레퍼런스의 결과 화면은 키 상태 배열의 인덱스로 역할을 읽는다(`ResultKeyProperty.BEAT_5K`: 인덱스 5 = 그래프 전환, 6 = 같은 배치로 다시). 5키의 키 상태 배열은 건반 5개 뒤에 턴테이블 두 방향이라(`PlayModeConfig.KeyboardConfig`) 그 두 역할이 턴테이블의 것이다. R-BMS 는 턴테이블을 모드와 무관하게 인덱스 7·8 로 읽으면서(`keyconfig.rs` `key_index_of`) 7키의 표를 그대로 써서, 5키에 없는 여섯째·일곱째 건반에 역할이 가 있었다 | `apps/rbms-player/src/stage/result.rs` 에 5키·10키용 표 `FIVE_KEYS`(인덱스 7 = 그래프 전환, 8 = 같은 배치로 다시). 10키의 2P 쪽은 다른 화면과 같이 1P 를 되풀이한다(레퍼런스는 10키에 14키의 표를 써서 2P 쪽 역할이 두 칸 어긋나 있다) | `stage/result/tests.rs` `every_mode_has_an_input_for_each_role`, `the_turntable_of_five_keys_turns_the_gauge_and_runs_the_chart_again`, `each_mode_assigns_its_keys_as_the_reference_does` |
| 선곡에서 op 190~195 가 전부 거짓이었다(그림이 있다고도 없다고도 답하지 않았다) | 커서 곡의 `ChartMeta` 가 그림 유무를 "모름"으로 두었다. 레퍼런스의 선곡은 커서 막대의 그림을 로드한 차트의 그림과 같은 자리에 넣고(`MusicSelector.loadSelectedSongImages` → `BMSResource.setStagefile`) 옵션은 그 자리가 비었는지로 답한다 | `stage/select/list.rs` `chart_under_cursor` 가 그 프레임에 그릴 그림(`ReferenceImages`)을 받아 세 그림의 유무를 채운다 | `stage/select/lent_tests.rs` `the_chart_under_the_cursor_has_the_pictures_that_are_loaded` |

엔진·하니스 보완 필요(고치지 않았다. 기본 스킨의 화면을 막지는 않는다)

| # | 내용 | 재현 | 제안 |
| --- | --- | --- | --- |
| 1 | (해결) 앱 캡처 하니스에 9키 플레이 런이 없었다 | - | 웨이브 8B 의 플레이 단위가 `stage/capture.rs` 에 9키 런을 넣었다 |
| 2 | (해결) 캡처 테스트가 문서 누락에서 멈추는 문제는 문서 9개가 다 있어 더는 걸리지 않는다 | - | - |
| 3 | `crates/rbms-render/tests/skin_external.rs` 는 진입 파일 이름이 ModernChic 의 것으로 고정돼 있다(`decide.luaskin`, `result.luaskin`, `course.luaskin`, `musicselect.luaskin`, `play7_hw.luaskin` 등). 앱 쪽에도 ModernChic 의 좌표·옵션에 묶인 환경 변수 테스트가 둘 있다: 검색 칸을 누르는 `the_search_box_of_a_skin_pack_named_by_the_environment_is_typed_into`(누를 자리가 상수)와 동영상 배경을 기대하는 `the_movies_of_a_skin_pack_named_by_the_environment_are_captured_as_they_play`. 기본 스킨에는 셋 다 실패한다 | `RBMS_SKIN_PACK=assets/skins/rbms-default cargo test -p rbms-render --test skin_external`, `cargo test -p rbms-player named_by_the_environment` | W8-5(테스트 재편)에서 헤더 type 으로 문서를 찾고 누를 자리를 스킨에서 읽게 일반화하거나 기본 스킨 전용 게이트 테스트를 둔다. 그때까지 기본 스킨의 화면 확인은 8.2절의 일곱 테스트로 한다 |
| 4 | 선언되지 않은 폰트 id 를 쓰는 text 가 경고 없이 대체 글꼴로 그려진다 | text 에 `font = 9`(미선언)를 주고 덤프: 경고 0 | 로더가 경고를 남기게 한다(낮은 우선순위) |
| 5 | (해결) 선곡에서 op 190·191 이 둘 다 거짓이었다 | - | 위 표의 둘째 줄 |
| 6 | `customTimers` 가 로드만 되고 평가되지 않는다. id 로 가리킨 커스텀 타이머(10000~)를 쓰는 destination 은 그려지지 않는다. `main_state.set_timer` 로 쓴 값도 렌더러는 읽지 않는다(`custom_timer_us` 의 호출처가 없다) | 문서에 `customTimers = { { id = 10000, timer = function() return main_state.time() end } }` 와 `timer = 10000` 인 destination 을 두면 덤프는 `customTimers 1`, 경고 0 인데 그려지지 않는다. 같은 함수를 destination 의 `timer` 에 직접 주면 그려진다 | 사양이 웨이브 9 로 미뤄 둔 항목이다(사양 §4.6 의 프레임 순서, §6 웨이브 9). 기본 스킨은 destination 의 `timer` 에 Lua 함수를 직접 준다(선곡의 사이드 메뉴) |
| 7 | 선곡에서 표 이름·표 레벨(text 1001~1003)이 언제나 빈 문자열이고 op 1008 이 언제나 거짓이다. `ChartMeta.table_name` 을 채우는 곳이 결정 화면뿐이다(`stage/decide.rs`). 레퍼런스도 선곡에서는 곡을 읽을 때 정한 값만 보이므로 결함이라기보다 미지원이다 | 선곡 문서에 `op = { 1008 }` 인 객체와 ref 1003 텍스트를 두고 표 안의 곡에 커서를 둔다 | 선곡에서 표 레벨을 보이려면 `skin_host/select/bridge.rs` 가 열린 표 폴더(`SelectView::TableLevel`)로 채우는 단위를 둔다 |
| 8 | 앱이 선곡 위에 그리는 것과 스킨의 정합. (a) 해결(웨이브 9A): `/` 와 숫자 0 은 스킨의 ref 30 칸에 치게 하고 앱의 검색 상자는 검색 칸이 없는 스킨에서만 뜬다. (b) 랭킹 패널이 x 972 까지만 덮어 정보 영역 오른쪽 124px 이 남는다. (c) 옵션 오버레이가 반투명이라 목록 제목이 뒤로 비친다. (d) 토스트가 목록 오른쪽 아래 막대 2~3개(y 837~990)를 덮는다 | `the_overlays_the_application_draws_over_the_browser` 캡처의 `overlay-search`, `overlay-ranking`, `overlay-options`, `overlay-system` | (b)~(d)는 스킨으로는 고칠 수 없다. 패널은 스킨 장면 위에서 불투명한 면을 깔게 하는 앱 단위를 둔다 |
| 9 | 캡처 하니스의 글꼴이 번들 Inter 하나뿐이라(`crates/rbms-render/src/font.rs` `embedded_only`) 한글·일본어가 빈 사각형으로 그려진다. 실제 앱은 시스템 글꼴로 대체하지만(사양 E5) CJK 제목의 폭·기준선·축소를 하니스로 확인할 수 없다 | 곡 제목이 CJK 인 차트로 어느 캡처든 | 하니스에 CJK 글꼴 하나를 옵션으로 싣는다(저장소에 글꼴 자산을 더하는 결정이 먼저다) |
| 10 | 판정 알고리즘이 넷째 값(Score)이면 index 340 이 "없음"이고 act 340 이 움직이지 않는다(`skin_host/options.rs` 의 번호 붙은 세 값, `stage/select/events.rs`). 레퍼런스의 번호 체계를 따른 것이다 | 앱 설정에서 Score 를 고른 뒤 선곡의 QUICK SETTINGS 행을 누른다(하니스로는 하지 못했다) | 스킨은 "SCORE (SET IN SETTINGS)" 로 알린다. 넷째 값에 번호를 줄지는 사양 결정이다 |
| 11 | judgegraph 는 색 지정을 받지 않는다(엔진 고정: 파랑, 초록, 노랑, 주황, 빨강. 9키는 다른 팔레트. 레퍼런스와 같다). IIDX 식 판정 색(3.1절)과 다르다 | 판정 수 표와 분포 그래프를 나란히 본다 | 기본 스킨은 그래프마다 범례를 둔다: 결과의 두 그래프는 캡션 줄에, 플레이의 분포 그래프는 바로 위에(`parts.legend`). 그래프의 색을 스킨이 정하게 하려면 judgegraph 에 색 필드를 더하는 엔진 단위가 필요하다(레퍼런스에 없는 확장) |
| 12 | 결과 화면에서 다시 하기·다음 곡이 가능한지를 알리는 속성이 없다 | 다시 할 수 없는 런(코스의 단곡 등)의 결과에서도 R·N 안내가 나온다 | 필요해지면 R-BMS 고유 옵션 id 를 두는 단위를 둔다 |

## 8. 검증 방법

### 8.1 덤프

```
cargo run -p rbms-cli -- skin-dump assets/skins/rbms-default --strict --frames 120
```

옵션은 `apps/rbms-cli/src/skin_dump/args.rs`: `--scenario <json>`, `--overlay <폴더>`, `--frames <N>`, `--seed <n>`, `--json`, `--probe-budget`, `--strict`, `--allow-failure <문서>`. `--strict` 는 로드 실패, 스킨의 pcall 이 삼킨 오류, 함수 값 실패, 프레임 예산 초과가 하나라도 있으면 실패한다(`skin_dump/gate.rs`). 경고는 실패가 아니므로 눈으로 0 인지 본다.

기대: 문서마다 `load ok`, `warnings 0`, `caught by the skin's own pcall: 0`, `function failures: 0`, 팩 폴더 `unchanged`.

덤프는 옵션 행을 기본값으로만 읽는다. 행을 바꾼 배치(Lane Width = Wide 와 그 조합)는 덤프가 읽지 않으므로 8.2절의 `play_rows.rs` 게이트가 맡는다.

### 8.2 캡처

```
RBMS_SKIN_PACK=$PWD/assets/skins/rbms-default RBMS_SKIN_CAPTURE_DIR=<폴더> \
  cargo test -p rbms-player <테스트 이름> -- --nocapture
```

`RBMS_SKIN_PACK` 은 절대 경로로 준다. 테스트는 `apps/rbms-player/src/stage/capture.rs` 에 있고 헤더 type 으로 문서를 찾는다.

| 화면 | 테스트 이름(앞부분) | 저장되는 프레임 |
| --- | --- | --- |
| 결정 | `the_decide_scene_of_a_skin_pack_named_by_the_environment_is_captured_from` | `decide-0000ms` ~ `decide-3950ms` 7장(샘플 5키 차트), `decide-rich-1500ms`, `decide-rich-3750ms`(스테이지 그림·변속·난이도 있는 차트) |
| 선곡 | `the_browser_of_a_skin_pack_named_by_the_environment_is_captured_over` | `select-*`: 곡 목록(기록 있는 곡, 없는 곡), 루트, 표 안, 코스 탭, 패널 0~3 |
| 선곡 위 오버레이 | `the_overlays_the_application_draws_over_the_browser` | `overlay-*`: 검색, 필터, 옵션, 랭킹, 안내, 토스트 등 앱이 덧그리는 것 |
| 플레이 | `the_play_scene_of_a_skin_pack_named_by_the_environment_is_captured_through` | `play-sample-*`(5키 샘플: 로딩, 준비, 플레이), `play7-*`(로딩, 준비, 플레이 2장, 풀 콤보, 종료, 페이드, 폐점 3장), `play5`·`play9`·`play10`·`play14` 플레이 중 한 장씩 |
| 결과 | `the_result_scene_of_a_skin_pack_named_by_the_environment_is_captured_for` | `result-clear-*`, `result-fail-*`: 0·1000·3000ms, 페이드, `menu2`(왼쪽 화살표를 누른 뒤. 메뉴가 둘인 스킨을 위한 프레임이고 기본 스킨에서는 3000ms 와 같은 화면이다) |
| 코스 결과 | `the_course_result_scene_of_a_skin_pack_named_by_the_environment_is_captured_for` | `course-result-clear-*`, `course-result-fail-*`: 같은 구성 |
| 팩 전체 | `a_skin_pack_named_by_the_environment_is_captured_one_document_a_screen` | `pack-select`, `pack-play7` 과 GPU 판 |
| 손으로 친 플레이 | `the_play_scene_of_a_skin_pack_named_by_the_environment_is_captured_as_it_is_played_by_hand`(`stage/capture/by_hand.rs`) | `play7-hand-<선택>-<타>-a`, `-b` 와 `play14-hand-…`. 하니스가 노트 한 개씩을 정해진 만큼 빠르게·늦게 직접 친다(누름을 차트 시각과 함께 런에 넘긴다. 실제 시계로 재지 않으므로 어느 기계에서나 같은 판정이 나온다): `great-fast`(40ms 빠름), `great-slow`, `good-fast`(100ms), `good-slow`, `on-the-beat`. `<선택>` 은 `default`(팩의 기본값), `words`(Judge Detail 을 Fast-Slow 로. ModernChic 은 判定タイミング 을 表示 로), `figures`(Milliseconds / +-ms). `-b` 는 `-a` 의 40ms 뒤라 판정 글자가 깜빡이는 스킨은 둘 중 하나에 글자가 있다(기본 스킨은 웨이브 10C 부터 두 장이 같다). 14키는 2P 쪽 건반을 친다 |

옵션 행을 바꾼 플레이 화면은 기본 팩만 찍는다(`stage/capture/play_rows.rs`). `RBMS_SKIN_PACK` 없이 `RBMS_SKIN_CAPTURE_DIR` 만 준다. 웨이브 14 에서 색 옵션을 바꾼 16개 장면 52장이 더해져 전부 94장이다(`rows-play7-theme-slate-accent-sky-*`, `rows-play7-accent-<sky\|amber\|green\|pink>-*`, `rows-play7-notecolors-<white-deepblue\|white-yellow\|white-green\|mono>-*`, `rows-play7-notecolors-mono-bombcolor-white-*`, `rows-play14-notecolors-white-yellow-bombcolor-white-*`, 9·14키의 Slate + Sky). 선곡·결정·결과·코스 결과에 다른 색 옵션을 준 캡처는 하니스에 없다: 팩 사본의 옵션 기본값(`def`)을 바꾸고 그 사본을 `RBMS_SKIN_PACK` 으로 줘서 찍는다(8.4절).

```
RBMS_SKIN_CAPTURE_DIR=<폴더> cargo test -p rbms-player --lib play_rows -- --test-threads=1
```

| 테스트 | 하는 일 |
| --- | --- |
| `the_play_scene_of_the_default_pack_is_captured_with_its_rows_switched` | `RBMS_SKIN_CAPTURE_DIR` 이 있을 때만 그린다. 1920x1080 42장: `rows-<문서>[-<행>-<항목>…][-covers][-course][-fail]-<순간>`. 기본값 16장(5·7·9·10·14키의 `play`, 7키의 `loading`·`ready`·`held`·`fullcombo`, 7키 Field Side Right + Scratch Side Right + Score Graph Off, 9키 Score Graph Off, 7키 커버 3종, 7키 폐점, 7·9·14키 코스 마지막 스테이지)과 Lane Width = Wide 26장(다섯 문서의 `play`, 7·14키의 `loading`·`ready`·`held`·`fullcombo`, 7키의 Graph Off / Field Right / Scratch Right / 셋 다, 5키 Field Right + Scratch Right, 9키 Graph Off, 7·9·14키 커버 3종, 7·14키 폐점, 9·14키 코스). `held` 는 차트 7,500ms(LN 을 누르고 있는 순간), `play` 는 9,000ms, 커버는 레인 커버 30%·리프트 20%·히든 20%(웨이브 13 에서 리프트 15%·히든 10% 에서 올렸다) |
| `the_default_pack_draws_every_play_scene_with_lanes_half_as_wide_again_when_its_row_says_so` | 어디서나 돈다(640x360). 다섯 문서를 Normal 과 Wide 로 컴파일해: 실패·경고 없음, note 의 레인마다 Wide 폭 x 2 = Normal 폭 x 3, 레인의 y·h 는 같음, Wide 의 레인이 화면 안이고 서로 겹치지 않음, Wide 로 3초까지 플레이해 그려짐 |
| `the_default_pack_keeps_the_pictures_and_the_gauge_clear_of_wide_lanes_however_its_rows_are_switched` | 어디서나 돈다. Wide 에서 문서가 가진 나머지 행의 모든 조합(5·7키 8가지, 9키 2가지, 10·14키 1가지)을 컴파일해: 실패·경고 없음, bga 와 gauge 가 화면 안이고 어느 레인과도, 서로도 겹치지 않음 |
| `the_default_pack_draws_every_play_scene_on_black_whichever_colours_its_rows_say_unless_the_theme_is_slate` | 어디서나 돈다(640x360. 웨이브 14). 다섯 문서마다 색 옵션의 모든 항목(Theme 2, Accent 5, Bomb Color 2, 스크래치가 있는 네 문서는 Note Colors 5)을 하나씩 골라 컴파일해: 실패·경고 없음, 3초까지 플레이해 그려짐, 화면 왼쪽 위 픽셀이 검정(0, 0, 0)이고 Theme = Slate 일 때만 (12, 15, 20). 9키 문서에 Note Colors 행이 없는 것도 본다 |
| `the_default_pack_ends_every_field_on_the_foot_of_its_judgement_line` | 웨이브 14 에서 폭발의 조건이 바뀌었다: 폭발은 "판정선 아래로 내려가지 않는 부위"에서 빠지고, 대신 필드마다 레인 수 이상의 폭발이 있고 폭발의 상자가 정사각형(온전한 고리)이며 그 가운데 행이 판정선의 가운데 행인지 본다. 필드 아래 12px 이 화면 바탕인지 보는 프레임은 Bomb Opacity 를 0% 로 두고 그린다(폭발만 그 줄에 들어오므로). 나머지는 아래와 같다. 어디서나 돈다(640x360. 웨이브 13). Lane Width·Field Side·Scratch Side 의 모든 조합(5·7키 8가지, 9·10·14키 2가지)을 컴파일해 필드마다: note 의 레인이 전부 한 행에서 끝나고, 판정선(Judge Line 부위의 사각형 가운데 가장 낮은 것)이 그 행을 윗변으로 필드 폭 전체에 놓이며, 레인 면과 레인 구분선의 아래 변이 판정선의 아래 변과 같고, 레인 위에 그리는 부위(레인 면, 구분선, 판정선, 키 빔, 폭발)의 어느 사각형도 판정선의 아래 변보다 내려가지 않는지 본다. 이어서 설정에 남은 옛 오프셋 105 에 알파 -255 를 넣고 3초까지 플레이해: 실패·경고 없음, 필드 열의 판정선 아래 16px(게이지나 발치까지의 틈)이 화면 바탕 한 색인지 본다 |

판정 줄과 부위 불투명도는 `stage/capture/play_parts.rs` 가 맡는다(웨이브 10C). 행은 `play_rows.rs` 와 같은 통로로, 오프셋은 SKIN 메뉴가 저장하는 자리(`config.skin.customise(문서).offsets[id]`)에 이름으로 찾아 넣는다. 오프셋은 프레임마다 저장된 값에서 읽히므로 런 도중에 바꿔 같은 순간을 다시 그릴 수 있다. 런은 하니스가 박자마다 한 타씩 직접 치거나(120 BPM, 4분음표마다 노트 하나. 누름을 차트 시각과 함께 넘긴다) 엔진이 친다.

```
RBMS_SKIN_CAPTURE_DIR=<폴더> cargo test -p rbms-player --lib play_parts -- --nocapture
```

| 테스트 | 하는 일 |
| --- | --- |
| `the_default_pack_holds_a_judgement_still_for_as_long_as_it_is_shown` | 어디서나 돈다(960x540). 필드 위의 것을 불투명도 0% 로 치워 판정 줄만 남기고, 누른 프레임과 그 뒤 0·16·32·48·64·80·96·250·440·496ms 의 필드(레인 열 x 판정선 위)가 픽셀까지 같은지, 잉크 상자가 같은지, 540ms 에는 아무것도 없는지 본다. 위 줄 없이(Score Difference Off, Judge Detail Off, PGREAT) 한 번, 기본값으로(40ms 빠른 GREAT. 점수 차와 FAST 가 뜬다) 한 번 |
| `the_default_pack_keeps_the_word_of_a_judgement_in_place_as_its_combo_grows` | 어디서나 돈다. 박자에 맞춰 100타를 쳐 콤보 9, 10, 100 의 프레임에서 판정 줄의 잉크 상자가 같고 글자의 열(문서의 judge 첫 image 가 놓인 열)이 픽셀까지 같은지 본다 |
| `the_default_pack_moves_its_judgement_by_its_position_and_hides_it_when_told_to` | 어디서나 돈다. Judge Position +100 이면 잉크 상자가 위로 정확히 100px, -100 이면 아래로 100px 이고 좌우·크기는 그대로인지, Judge Display Hide 면 잉크가 없는지 본다 |
| `the_default_pack_draws_nothing_of_a_part_at_no_opacity` | 어디서나 돈다. 커버 3종을 켜고 Judge Display Hide, 모든 부위 0% 로: 건반을 뗀 뒤 40ms(키 빔이 페이드 중)와 건반을 누르고 있는 프레임 전체가 한 색인지 본다. 이어서 부위 하나만 100% 로 되돌린 프레임이 한 색이 아닌지를 문서가 선언한 부위마다 본다(부위마다 제 오프셋으로만 사라진다). 한 destination 이라도 부위에 걸리지 않았거나 엔진이 0% 에서 무엇을 남기면 실패한다 |
| `the_keys_of_the_skin_menu_take_the_parts_of_the_default_pack_off_the_field` | 어디서나 돈다(웨이브 10C 통합). 손으로 치는 7키 런에 SKIN 메뉴를 열어(런의 앱 설정 위에서, 화면은 바꾸지 않고) 키만으로 문서의 모든 부위를 내린다: 행까지 ↓, Shift+← 네 번, 행이 `0%` 인지 확인, 끝에 Esc. 그 뒤 건반을 누르고 있는 프레임 전체가 한 색인지, 저장된 오프셋이 부위마다 a = -255 인지, Esc 가 쓴 설정 파일을 다시 읽은 값이 같은지 본다. 메뉴가 쓰는 곳과 플레이가 읽는 곳이 어긋나면 실패한다 |
| `the_play_field_of_the_default_pack_is_captured_with_its_judgement_and_its_parts_set` | `RBMS_SKIN_CAPTURE_DIR` 이 있을 때만 그린다. 1920x1080 135장(통합에서 `parts-play7-half-<부위>` 19장과 SKIN 메뉴 키로 부위를 내린 `parts-menu-bare-<before\|after>` 2장을 더했다. 아래 설명의 114장에 더해진다). `parts-frames-<on-the-beat\|early>-<ms>`(판정 뒤 11프레임), `parts-judge[-ms\|-wide\|-play5\|-play14\|-play14-wide]-<combo-009\|combo-010\|combo-100\|great-fast\|great-slow\|good-fast\|good-slow\|missed\|on-the-beat>`, `parts-line-<behind\|detail-only\|difference-only>`, `parts-position-<up\|down\|hidden>`, `parts-play7-<whole\|bare\|without-<부위>\|half>`(엔진이 치는 런의 9,000ms. `bare` 는 사용자의 예, `without-*` 은 부위 하나만 0%, `half` 는 전부 50%), `parts-<play7-covers\|play7-wide\|play9\|play14\|play14-wide>-<whole\|bare>` |
| `a_judgement_of_a_skin_pack_named_by_the_environment_is_captured_frame_by_frame` | `RBMS_SKIN_PACK` 이 있을 때만. 그 팩의 7키 판정을 같은 11프레임으로 찍는다(`pack-judgement-*`). 고치기 전 팩이나 다른 팩의 판정 움직임을 나란히 보는 용도다 |

필터를 여럿 줄 때는 `--` 뒤에 적는다: `cargo test -p rbms-player --lib -- <이름 1> <이름 2> --nocapture --test-threads=1`. 캡처는 결정적이다(같은 팩으로 두 번 찍은 80장이 픽셀까지 같다. `pack-*` 의 진행 막대와 맥동만 실행 시각을 탄다).

환경 변수 없이 어디서나 도는 게이트가 둘 더 있다(`by_hand.rs`): `the_default_pack_draws_an_early_press_and_a_late_one_apart_on_the_field` 와 `…_on_the_field_they_were_made_on`. 기본 팩을 기본 옵션으로 띄우고 같은 노트를 40ms 빠르게 친 프레임과 40ms 늦게 친 프레임을 그려, 필드(문서의 note 레인이 차지하는 열)가 서로 다르게 그려지는지를 본다(DP 는 친 쪽 필드만 다르고 반대쪽은 같아야 한다). Judge Detail 의 기본값이 Off 로 돌아가면 실패한다. `RBMS_SKIN_CAPTURE_DIR` 을 주면 `held-single-*`, `held-double-*` 로 남는다.

`cargo test -p rbms-render --test skin_external` 과, 이름이 `named_by_the_environment` 로 끝나는 검색 칸·동영상 테스트 둘은 기본 스킨에 쓸 수 없다(7절 보완 3).

### 8.3 화면별 확인 항목

| 화면 | 캡처를 열어 볼 것 |
| --- | --- |
| 색(웨이브 14) | 기본값에서 바탕 픽셀이 (0, 0, 0)인가. 면·선·글자의 픽셀이 R = G = B 인가(색을 가진 픽셀은 3.1절 "색으로 두는 것"에 있는 것뿐인가). 면의 단계(바탕, `panel`, `raised`)가 서로 구분되는가. `muted` 글자가 `raised` 면 위에서 읽히는가. 폭발이 온전한 고리이고 판정선에 중심이 있는가. Theme = Slate + Accent = Sky 가 웨이브 13 의 화면과 같은가 |
| 공통 | 글자가 읽히는가(최소 18). 요소가 겹치거나 잘리지 않는가. 값의 기준선이 맞는가. 등장·퇴장이 3.5절대로인가. 마지막 페이드 프레임이 `bg` 단색인가. 자릿수 상한을 넘는 값에서 아래 자리만 나오지 않는가(3.3절. 앱 캡처의 샘플 차트로는 볼 수 없으므로 시나리오 값을 바꿔 그린다) |
| 결정 | 0ms 에 머리와 구분선만, 1500ms 에 전부, 3500ms 에 항목 구분선 자리의 `accent` 선과 옅은 덮개, 3950ms 에 단색. 긴 제목이 스테이지 그림(x 1280)을 넘지 않는가. 변속 곡에서 최소·줄표·최대가 붙어 있는가. 숫자의 잉크가 항목 이름의 x 에서 시작하는가 |
| 선곡 | 가운데 막대가 top 520 에 있는가. 막대 종류·램프·레벨·라벨이 6.2절대로인가. 폴더·코스 막대에서 정보 영역이 바뀌는가. 그림 없는 곡에 "NO STAGE IMAGE" 가 뜨는가. 패널 1~3 의 칸이 건반 순서와 맞는가 |
| 플레이(Lane Width = Wide) | 레인의 아래쪽 줄(y 880)에서 잰 레인 폭이 Normal 의 1.5 배인가(87, 69, 150, 93). 노트가 레인 폭을 꽉 채우고 가장자리가 한 픽셀에 끊기는가. 누르고 있는 LN 의 아래 끝이 판정선에 닿는가. 판정 글자와 세 자리 콤보의 묶음이 필드 가운데인가. 정보 영역의 상자가 서로, 필드와, 화면 끝과 겹치지 않는가(7키 Graph On 은 2단, 9키는 타이밍이 왼쪽, DP 는 게이지가 1P 필드 아래). 코스에서 좁은 곡 머리의 단계 글자가 2행 오른쪽에 있는가. Normal 의 캡처가 고치기 전과 바이트까지 같은가 |
| 플레이 | 노트 아래 변이 판정선에 닿는가. 레인 수와 순서(스크래치 방향 옵션 포함). 판정 묶음(글자 + 콤보 세 자리)이 필드 가운데이고 콤보 다섯 자리가 필드 안인가. 판정이 바뀌어도 글자의 왼쪽 끝이 같은가. 게이지 50칸. DP 에서 두 필드가 대칭인가. 9키에서 9레인이 가운데인가. 손으로 친 캡처에서: 빠른 타에 파란 "FAST", 느린 타에 빨간 "SLOW" 가 판정 글자 위 줄의 오른쪽 자리에 뜨고 PGREAT 에는 뜨지 않는가. 같은 줄 왼쪽에 부호와 네 자리의 점수 차가 뜨고 스코어 그래프의 TARGET 숫자와 같은 값인가. PGREAT 의 "GREAT" 가 하늘색, GREAT 가 노랑인가. DP 에서 친 쪽 필드에만 뜨는가. 판정 수 표가 5행이고 분포 그래프 위에 범례가 있는가 |
| 플레이(판정 줄과 부위. `play_parts.rs`) | `parts-frames-*` 11장에서 판정 줄이 같은 자리·크기·색인가(540ms 에는 없는가). 콤보 9·10·100 에서 글자가 같은 자리인가. Judge Position ±100 에서 묶음이 통째로 100px 움직이는가. `bare` 가 화면 바탕 위에 노트·키 빔·폭발·판정선·판정 줄만 남기는가. `without-<부위>` 마다 그 부위만 사라지고 가장자리·잔상이 없는가 |
| 결과 | 클리어와 실패의 머리 글자·색. 랭크, 점수 차이의 부호, 판정 표 5행 15칸과 COMBO BREAK·EMPTY POOR, 그래프 4개와 그래프마다의 범례. 안내의 게이지 전환 문구가 키 수에 맞는가 |

### 8.4 실제로 확인한 것

웨이브 8A(골격과 결정 화면)

| 확인 | 결과 |
| --- | --- |
| 결정 캡처 9장 | 0ms: 캡션과 두 구분선만. 500ms: 글줄이 들어오는 중, 항목 3개가 반투명. 1500ms·3000ms: 전부 보임, 진행선이 47%·94%. 3500ms: 항목 구분선 자리의 `accent` 선과 20% 덮개(선이 글줄과 스테이지 그림을 지나지 않는다). 3750ms: 70% 덮개. 3950ms: 단색. 변속 차트(rich): "12 ANOTHER", "75 – 300", "1020", "NORMAL", "7 KEYS", 스테이지 그림, 노트 분포와 BPM 선. 겹침·잘림 없음 |
| 숫자 정렬(rich 1500ms 실측) | LEVEL 이름 x 64, 숫자 "12" 잉크 x 64. BPM 이름 520, 숫자 520. NOTES 이름 976, 숫자 976. 숫자 기준선 835~836 |
| 자릿수 초과(렌더 하니스 사본에 시나리오 값만 바꿔 그림) | 레벨 123, BPM 1 – 655350, 노트 123456: 전부 온전히 그려지고 레벨만 칸의 절반에 맞게 줄었다. 레벨 1234, BPM 12345678 – 99999999, 노트 123456789: 전부 온전하고 각자 칸 안에 줄어 들어간다(BPM 의 두 값은 같은 크기) |
| Stage Image Hide(팩 사본의 기본값만 Hide 로 바꿔 그림) | 87자 제목이 x 1856 까지 쓰인다(Show 에서는 x 1216 까지). 스테이지 면과 문구는 그려지지 않는다 |

웨이브 8B 통합(문서 9개 전부. 2026-10-11)

| 확인 | 결과 |
| --- | --- |
| 덤프(`--strict --frames 120`) | 문서 9개 전부 load ok, 경고 0, pcall 0, 함수 실패 0, 팩 폴더 73 files unchanged. destination: decide 59, select 499, play5 190, play7 206, play9 214, play10 248, play14 280, result 148, courseresult 85. 쓰기 권한을 뺀 사본에서도 같다 |
| 프레임 비용(`--probe-budget`, 120프레임) | 평균 / p99 / 최대(ms): decide 0.041 / 0.061 / 0.143, select 0.103 / 0.130 / 0.165(프레임당 함수 53회), play5 0.054 / 0.057 / 0.075, play7 0.054 / 0.056 / 0.058, play9 0.054 / 0.058 / 0.075, play10 0.056 / 0.060 / 0.077, play14 0.056 / 0.068 / 0.087, result 0.051 / 0.062 / 0.069, courseresult 0.084 / 0.124 / 0.174. 예산 초과 0 |
| 캡처 80장(8.2절의 일곱 테스트) | 전부 통과. 공통 승격 전후의 캡처를 픽셀로 비교해 의도한 차이(판정 색 순서, MAX 램프 색, 판정 여섯째 행 이름, 5키 결과의 안내, 선곡의 그림 없음 문구)만 남는 것을 확인했고, 같은 팩으로 다시 찍은 것은 `pack-play7` 의 진행 막대 1줄 말고 같다 |
| 결정 | 웨이브 8A 의 캡처와 픽셀까지 같다 |
| 선곡 | 가운데 막대 top 520, 선택 틀과 `accent` 띠. 곡 막대의 램프·레벨(난이도 색)·라벨 3단(MINE, LN, RANDOM). 기록 있는 곡: 랭크 B, EASY, 숫자 6칸, 판정 5행과 막대. 기록 없는 곡: 줄표, "NO PLAY", 그림 없는 곡에 "NO STAGE IMAGE". 루트·표 안: 폴더 정보(CHARTS, 램프 11행)와 막대의 분포 띠(MAX 가 분홍으로 HARD 의 흰색과 갈린다). 코스 탭: STAGES 와 CONSTRAINTS. 패널 1~3: 칸 7개가 건반 순서, 값이 없는 칸은 "NOT AVAILABLE" |
| 플레이 | 7키: 로딩(막대), READY, 플레이 2장(노트 아래 변이 판정선에, 판정 글자와 콤보가 필드 가운데, 게이지 50칸, 스코어 그래프 3막대, 판정 수 6행의 색이 분포 그래프와 같은 순서), 풀 콤보, 종료, 페이드, 폐점 3장("FAILED" 와 어두워지는 덮개). 5키: 필드 x 172, 폭 376. 9키: 9레인이 가운데, 왼쪽에 곡 머리·BGA·점수. 10·14키: 두 필드가 대칭, 판정이 필드마다, 가운데에 정보. "EMPTY POOR" 가 합계 칸과 겹치지 않는다(7키 On 에서 이름 끝 x 1245, 4자리 합계의 시작 x 1250) |
| 결과, 코스 결과 | 클리어: "STAGE CLEARED"(`accent`), AAA, MAX(분홍), 차 "+8"(`accent`), 판정 표 6행과 막대, 그래프 4개와 범례. 실패: "STAGE FAILED"(POOR 색), F(`muted`), FAILED, 차 "-8"(BAD 색), POOR 막대. 5키 차트라 안내 끝에 "TURNTABLE  GAUGE". 코스: "COURSE CLEARED", "3 STAGES", 이어 붙인 게이지 그래프와 곡 경계, STAGES 3줄 |
| 화면 사이 | 머리 캡션과 구분선, 바닥 구분선과 안내 줄의 자리·크기·색이 결정·선곡·결과에서 같다. 판정 이름의 색 순서가 선곡·플레이·결과와 엔진의 그래프에서 같다 |
| 생성 스크립트 | 전체 `--check` 종료 코드 0(10개 파일 일치). 색을 `theme.lua` 에서 읽게 바꾼 뒤 play 시트가 바이트까지 같고, MAX 색을 바꾼 select 시트만 다시 만들었다. `__pycache__` 없음 |
| 용량 | `image/` 93,069 B(`du -sh` 100K), `font/` 868 KiB, `sound/` 572 KiB, `lua/` 236 KiB |
| 금지 API | 팩의 Lua 전체(4,706줄)에 `luajava`, `io.`, `os.`, `http`, `math.random`, 주석(`--`), 비 ASCII 문자가 없다 |
| 엔진 게이트 | `cargo fmt --all`, `cargo clippy --workspace --all-targets --all-features -- -D warnings` 통과. `cargo test --workspace --no-fail-fast`: 4,815 통과, 실패 0, ignored 5 |

웨이브 9A(판정 양식과 색. 2026-10-11)

| 확인 | 결과 |
| --- | --- |
| FAST/SLOW 가 안 보인 원인 | 재현 테스트 두 개(8.2절의 게이트)가 고치기 전 팩에서 실패했다: 40ms 빠른 GREAT 과 40ms 늦은 GREAT 의 필드가 픽셀까지 같았다. 같은 런을 Judge Detail = Fast-Slow 로 찍으면 "FAST" 가 떴고(고치기 전 `play7-hand-words-*`), ModernChic 도 判定タイミング = 表示 에서 FAST(파랑)·SLOW(빨강)·`-040` 이 떴다(기본값 非表示 에서는 뜨지 않는다). 엔진은 op 1242/1243·1262/1263 과 number 525/526 을 레퍼런스대로 주고 있었고(`skin_host/play.rs`), 원인은 기본값 Off 하나다 |
| 손으로 친 캡처 60장(7키·14키 x default·words·figures) | default: 빠른 GREAT·GOOD 위에 파란 "FAST", 느린 쪽에 빨간 "SLOW", 박자에 맞춘 PGREAT 에는 없음. GREAT·GOOD 은 금색 글자 + 콤보이고 `-a`·`-b`(40ms 차) 중 한 장에서는 글자가 꺼져 FAST/SLOW 만 남는다(깜빡임). PGREAT 는 두 장의 색이 다르다(파랑, 분홍, 민트 중 둘). figures: `+39`(파랑), `-40`(빨강), `+99`, `-100`, PGREAT `+0`(`sub`). 14키: 2P 필드에만 뜨고 1P 필드는 빈다. 판정 수 PG/GR/GD/BD/PR/MS 는 타마다 [0,1,0,0,0,0] → [0,2,0,0,0,0] → [0,2,1,0,0,0] → [0,2,2,0,0,0] → [1,2,2,0,0,0] |
| 플레이 | 판정 수 5행(PGREAT 파랑, GREAT·GOOD 금색, BAD 주황, POOR 주홍), FAST·SLOW 칸이 타마다 1씩 오른다. 타이밍 영역: 히트 에러 띠 아래에 범례(PG 파랑, GR 초록, GD 노랑, BD 주황, PR 빨강), 그 아래 분포 그래프. 게이지는 NORMAL 호박색, HARD 밝은 회색. 노트·건반·스크래치 색은 그대로다 |
| 결과 | 판정 표 5행 + "COMBO BREAK n" 과 "EMPTY POOR n". 실패 런(노트 4개를 전부 놓침): POOR 4, 느림 4, COMBO BREAK 4, EMPTY POOR 0, 막대 가득. CLEAR 글자: MAX 노랑, FAILED 빨강. 타이밍 분포의 띠가 범례의 색 순서다 |
| 선곡 | 막대 램프: EASY 초록, NORMAL 호박, HARD 밝은 회색, ASSIST 자홍, 도는 램프는 프레임 중 하나로 찍힌다. 폴더 정보의 11행: MAX 노랑, PERFECT 분홍, FULL COMBO 청록, EX HARD CLEAR 연한 빨강, HARD CLEAR 회색, CLEAR 호박, EASY CLEAR 초록, LIGHT ASSIST CLEAR 남보라, ASSIST CLEAR 자홍, FAILED 빨강, NO PLAY 꺼진 색. 레벨 숫자와 난이도 이름이 ModernChic 의 난이도 색. 기록의 CLEAR "EASY CLEAR"(초록), 판정 5행 |
| 덤프(`--strict --frames 120 --probe-budget`) | 문서 9개 load ok, 경고 0, pcall 0, 함수 실패 0, 예산 초과 0, 팩 폴더 73 files unchanged. destination: decide 59, select 499, play5 198, play7 214, play9 222, play10 258, play14 290, result 147, courseresult 84. 쓰기 권한을 뺀 사본에서도 같다 |
| 생성 스크립트 | `uv run tools/generate-images.py play select` 로 두 시트를 다시 만들었고 전체 `--check` 종료 코드 0. common·decide·result 는 바이트까지 그대로다 |
| 엔진 게이트 | `cargo test -p rbms-player -p rbms-cli -p rbms-skin -p rbms-render --no-fail-fast` 실패 0(rbms-player 1,765, rbms-render 714 와 골든). 내장 화면의 골든은 갱신하지 않았다 |

웨이브 10A(넓은 레인. 2026-10-11)

| 확인 | 결과 |
| --- | --- |
| 요청과 현재 상태 | 스킨이 그리는 플레이 화면에는 레인 폭을 바꿀 방법이 없었다. 내장 화면의 DISPLAY > SKIN 프리셋(NORMAL/WIDE)은 스킨 화면에 효과가 없다는 표지대로 기본 팩의 레인을 바꾸지 않는다. 고치기 전 팩에서 `play_rows.rs` 의 게이트 둘이 "the document has no row \"Lane Width\"" 로 실패했다 |
| 레인 폭 실측(1920x1080 캡처의 y 880 줄) | 7키 Normal: S 64 부터 100, 건반 58·46·58·46·58·46·58. Wide: S 64 부터 150, 건반 216 부터 87·69·87·69·87·69·87(끝 783). Scratch Right: 건반 64 부터, S 633 부터 150. 5키 Wide: S 224 부터 150, 건반 376 부터 87·69·87·69·87. 9키: 62 → 93(534 부터 95 간격 9개, 끝 1387). 14키 Wide: 1P S 32, 2P S 1738(끝 1888). 10키 Wide: 1P S 192, 2P 건반 1169 부터, S 1578. 전부 정확히 1.5 배 |
| 노트 | Wide 7키의 흰 건반 노트: x 216~302(87px), y 404~419(16px. Normal 과 같은 행), 위 14행 밝은 색 + 아래 2행 그늘이 Normal 과 같고 좌우 가장자리가 레인 경계에서 한 픽셀에 끊긴다(중간 색 없음). 누르고 있는 LN 몸통: Normal 은 레인 46 중 34(340~373), Wide 는 69 중 51(474~524)로 같은 비율이고 가장자리가 한 픽셀에 끊긴다. 몸통의 아래 끝이 판정선(840)에 닿고 그 위에 누르는 동안의 폭발이 레인 가운데에 있다 |
| 판정 글자와 콤보 | 잉크의 좌우 가운데와 필드 가운데의 차: 7키 +1.0, 5키 +1.5, 9키 +1.0, 14키 1P +0.5 / 2P +2.0, 10키 1P -2.5 / 2P +0.5, 7키 Field Right +1.0(px. 글자 옆 여백의 차이). 5키 Wide 는 크기 64 로 그려진다 |
| 필드를 따라가는 것 | Wide 7·14키의 `loading`(LOADING 과 막대가 필드 안쪽 48px 여백으로), `ready`(READY 가 필드 가운데, 판정선이 필드 폭 전체), `fullcombo`(글자와 선이 필드 폭), 커버 3종(레인 커버·리프트·히든이 필드 폭, 흰 숫자 300 이 왼쪽·녹색 숫자 360 이 오른쪽 끝, 리프트 150·히든 100), 건반 눌림과 키 빔이 레인 폭, 게이지 50칸(7키 칸 12px), 발치의 HI-SPEED·GREEN 3열. 폐점은 화면 전체다 |
| 배치(겹침·잘림) | 7키 Graph On: 그래프 820~956, BGA 980~1856 x 120~570, 점수·판정 수 594~792, 타이밍 816~968. Graph Off: BGA 820~1856 x 120~746, 세 상자 770~968(TIME LEFT 와 TIMING 캡션 사이 18px). Field Right 는 좌우가 뒤집혀 필드 1137~1856. 9키: 왼쪽 단 64~504(머리, BGA, 타이밍), 필드 534~1387, 오른쪽 단 그래프 1420~1556 + 판정 수·점수 1588~1856(FAST 와 SLOW 머리글 사이 12px). 14키: 필드 32~751 과 1169~1888, 진행 막대 759·1157, 가운데 단 784~1136 에 머리·BGA·점수 4행·판정 수 5행·타이밍, 게이지는 1P 필드 아래. 10키: 필드 192~751 과 1169~1728, 가운데는 14키와 같다. 어느 캡처에서도 상자가 서로·필드·화면 끝과 겹치지 않는다 |
| 코스의 좁은 곡 머리 | 9키 Wide(440)와 14키 Wide(352): 제목이 전폭, "FINAL STAGE"(크기 18, `accent`)가 2행 오른쪽 끝, 아티스트 없음. Normal 의 7·9·14키는 전과 같이 제목 줄 오른쪽 끝(크기 24)이고 아티스트가 있다 |
| Normal 이 그대로인가 | 고치기 전 팩(HEAD 의 `lua/play/*.lua`)과 고친 팩을 같은 하니스로 찍은 기본값 16장이 PNG 바이트까지 같다. 같은 팩으로 두 번 찍은 것도 같다(캡처는 결정적이다) |
| 덤프(`--strict --frames 120`) | 문서 9개 load ok, 경고 0, pcall 0, 함수 실패 0, 예산 초과 0, 팩 폴더 73 files unchanged. destination 은 그대로다(play5 198, play7 214, play9 222, play10 258, play14 290). 쓰기 권한을 뺀 사본에서도 같다 |
| 생성 스크립트 | 시트는 고치지 않았다. 전체 `--check` 종료 코드 0(10개 파일 일치) |
| 엔진 게이트 | `cargo test -p rbms-skin --test default_pack` 4 통과. `cargo test -p rbms-player capture` 42 통과(새 게이트 둘 포함). `cargo clippy -p rbms-player --all-targets -- -D warnings` 통과. 내장 화면의 골든은 건드리지 않았다 |

웨이브 10C(정적인 판정, 판정 위 줄, 판정 위치와 숨김, 부위별 불투명도. 2026-10-11)

| 확인 | 결과 |
| --- | --- |
| 고치기 전에 무엇이 움직였나 | HEAD 의 팩을 사본으로 꺼내 `a_judgement_of_a_skin_pack_named_by_the_environment_is_captured_frame_by_frame` 로 판정 뒤 11프레임을 찍었다. PGREAT: 0ms 에 글자가 1.15배로 크고 숫자가 오른쪽으로 밀려 있다가 48~64ms 에 제 크기·제 자리가 되고, 프레임마다 분홍·민트·파랑으로 색이 바뀌고, 496ms 에 거의 투명하다(페이드). GREAT: 같은 확대에 더해 440ms 프레임에서 글자와 숫자가 꺼져 있다(깜빡임). 콤보의 자릿수가 늘면 `shift` 로 글자가 왼쪽으로 반 칸씩 밀린다. 움직임은 전부 스킨의 키프레임(`popping`, `beside_popping`), 시트의 프레임(`cycle` 120·80), `shift = true` 였고 엔진의 judge 객체가 스스로 넣는 움직임은 없다(`skin_render/judge.rs` 는 각 destination 을 그대로 놓는다) |
| 판정이 가만히 있는가(1920x1080 실측) | `parts-frames-on-the-beat-000ms`~`496ms` 10장 모두: 글자 잉크(색 `84CEFF` 인 픽셀) x 97, y 561, 205 x 47, 2,661픽셀. 콤보 "1" 의 잉크 x 495, y 561, 16 x 47, 287픽셀. 점수 차 "+0001" 의 윗선 y 520. 540ms 프레임에는 판정 줄의 픽셀이 없다. `parts-frames-early-*`(40ms 빠른 GREAT, 색 `FFD633`): 글자가 같은 x 97, y 561, 205 x 47, 2,661픽셀이고 "FAST" 는 x 316, y 520, 57 x 17 로 10장이 같다 |
| 콤보가 늘 때 | `parts-judge-combo-009`, `-010`, `-100`: 글자 잉크가 세 장 모두 x 97, y 561, 205 x 47, 2,661픽셀. 숫자의 오른쪽 끝은 9 가 x 517, 10 과 100 이 517 로 같은 칸이고 왼쪽으로만 자리가 늘어난다(잉크 x 488, 455, 415). 빈 자리에는 옅은 0 |
| 판정 위 줄 | `parts-judge-*`: 박자에 맞춘 PGREAT 는 점수 차만(+0002, +0003, +0022), 40ms 빠른 GREAT 는 "+0021 FAST"(FAST 파랑), 40ms 늦은 GREAT "+0020 SLOW"(빨강), GOOD "+0019 FAST"·"+0017 SLOW", 놓친 노트 "POOR"(가운데) 위에 "+0015 SLOW". 같은 프레임의 스코어 그래프 TARGET 숫자가 +21, +20 으로 같다. `parts-line-behind`(GOOD 70타 뒤): "-0125 FAST". `parts-judge-ms-*`(Milliseconds + Best): "+0201 +40", "+0202 -100", PGREAT "+0204 +0"(`sub`). `parts-line-detail-only`: FAST 가 필드 가운데(x 278~334). `parts-line-difference-only`: 점수 차가 필드 가운데 |
| 위치와 숨김 | `parts-position-up`(Judge Position +100): 글자 y 461, 위 줄 y 420 으로 기본(561, 520)에서 정확히 100px 위, x 와 크기는 같다. `parts-position-down`(-100): y 661, 620. `parts-position-hidden`(Judge Display Hide): 필드에 판정 줄의 픽셀이 없다. 레인 커버 30%·리프트 15%·히든 10% 의 캡처(`parts-play7-covers-whole`)에서 판정 줄은 리프트만큼 올라가 있고 커버 수치(300·360, 150, 100)와 겹치지 않는다 |
| 부위 불투명도(7키, 엔진이 치는 런 9,000ms) | `parts-play7-whole`: 고치기 전과 같은 화면. `parts-play7-bare`(레인 배경·구분선·건반 표시·마디선·진행 막대·스코어 그래프·곡 머리·발치·타이밍 0%): 필드 자리가 화면 바탕 `0C0F14` 이고 그 위에 노트, 키 빔, 폭발, 판정선, 판정 줄만 있다(레인 면 자리 (120, 300), 구분선 (165, 300), 건반 표시 (120, 880) 가 전부 `0C0F14`). `parts-play7-without-<부위>` 19장: 부위마다 그것만 사라지고 다른 부위와 그 가장자리는 그대로다. `parts-play7-half`(전부 50%): 노트 `F1F4F8` → (127, 131, 136), 판정선 → (51, 132, 160), 분포 그래프 막대 (0, 136, 255) → (7, 77, 140), 곡 머리의 난이도 막대 → (133, 103, 10) 으로 note·graph·judgegraph 같은 특수 객체도 중간값을 받는다 |
| 넓은 레인과 DP | `parts-judge-wide-*`(Wide + Field Right): 판정 묶음이 넓은 필드 가운데, 크기 64. `parts-judge-play5-*`: 크기 48. `parts-judge-play14-*`, `-wide-*`: 친 2P 필드에만 판정 줄이 뜬다. `parts-play7-wide`, `parts-play9`, `parts-play14`, `parts-play14-wide` 의 `bare`: 어느 문서에서도 레인 면·선·건반 표시가 남지 않는다. 14키 `whole`: 두 필드의 판정 묶음이 각 필드 가운데(1P 콤보 132, 2P 134 는 각 필드가 마지막으로 판정된 때의 콤보다) |
| 덤프(`--strict --frames 120`) | 문서 9개 load ok, 경고 0, pcall 0, 함수 실패 0, 예산 초과 0, 팩 폴더 73 files unchanged. destination: play5 204, play7 222, play9 231, play10 270, play14 306(나머지는 그대로. 통합에서 건반 표시 바탕을 쪼갠 뒤에는 play5 209, play7 229, play9 239, play10 280, play14 320). 헤더: play5·7 옵션 7 + 오프셋 20(+ 자동 4), play9 옵션 5 + 20, play10·14 옵션 4 + 19. 쓰기 권한을 뺀 사본에서도 같다 |
| 기본값의 화면이 그대로인가 | HEAD 의 팩과 고친 팩을 같은 하니스·같은 차트로 찍어 비교했다(1920x1080). 판정이 없는 프레임: `early-540ms` 는 화면 전체가 픽셀까지 같고, `on-the-beat-540ms` 는 리듬 띠의 맨 아래 한 줄(y 839)에서 채널 값 1 차이뿐이다(리듬 띠는 실행 시각을 탄다. 8.2절). 판정이 떠 있는 프레임 9장(`on-the-beat` 의 0·16·64·96·250·440ms, `early` 의 0·96·250ms)은 판정 줄의 자리(x 64~548, y 500~620) 밖이 같거나 같은 한 줄의 1 차이다: 노트, 키 빔과 그 페이드, 건반 눌림, 폭발, 판정선, 게이지, 정보 영역이 그대로다. 처음 비교에서는 리듬 띠의 왼쪽 끝 30px 이 달랐다: 판정 프레임을 줄이면서 시트의 배치가 바뀌어 `beam` 이 시트 왼쪽 끝(x 0)에 놓였고, 필드 폭으로 늘려 그릴 때 왼쪽 가장자리가 투명 픽셀과 섞이지 않게 된 것이다. `beam` 에 투명 둘레 1px 을 둘러(`guarded`) 놓이는 자리와 무관하게 전과 같은 가장자리가 되게 했다 |
| 생성 스크립트 | `uv run tools/generate-images.py play` 로 play 시트만 다시 만들었다(56,482 → 37,530 B). 전체 `--check` 종료 코드 0(10개 파일 일치) |
| 부위의 중간값(웨이브 10C 통합. `parts-play7-<whole\|without-<부위>\|half-<부위>>` 의 (half - without) / (whole - without) 중앙값) | 반투명을 굽기 전: Key Beam 0.35(20% 이하에서 사라짐), Key Display 0.89, Judge Line 0.58, 나머지 13개 부위 0.48~0.51. 구운 뒤: Key Beam 0.48, Key Display 0.48, Judge Line 0.58(겹친 두 선. 그대로 둠), BGA 0.51, 분포 그래프 0.51, 게이지 0.49, 판정 수 0.49, 레인 구분선 0.48, 노트 0.50, 진행 막대 0.48, 스코어 그래프 0.51, 점수 0.50, 곡 머리 0.50, 발치 0.49, 타이밍 0.49. 폭발은 가산이라 밝은 자리에서 1.00(포화), 어두운 자리에서 0.48. 100% 의 화면은 굽기 전과 채널 값 2 이내(`parts-play7-whole` 15,519픽셀이 키 빔 자리 x 274~547, y 591~839 에서 최대 2, 9키는 건반 눌림 자리까지 최대 2, `parts-judge-on-the-beat` 는 리듬 띠 884픽셀이 최대 1) |
| 메뉴 조작으로 만든 플레이 화면(웨이브 10C 통합) | `parts-menu-bare-before`·`-after`: 엔진이 치는 7키 런 9,000ms 에서 SKIN 메뉴의 키(행마다 Shift+← 네 번)로 Lane Background, Lane Divider, Key Display, Bar Line, Progress Bar, Score Graph, Song Header, Speed Display, Timing 을 0% 로 내린 앞뒤. 뒤 장은 `parts-play7-bare`(오프셋을 직접 적은 것)와 같은 구성이다 |
| 엔진 게이트 | `cargo test -p rbms-skin --test default_pack` 4 통과. `cargo test -p rbms-player capture` 52 통과(새 게이트 넷 포함. 같은 웨이브의 엔진·메뉴 단위가 고친 작업 트리 위에서 돌렸다). 내장 화면의 골든은 건드리지 않았다 |

웨이브 10D(판정 줄 다시 놓기. 2026-10-12). 위 웨이브 10C 표의 "콤보가 늘 때", "판정 위 줄"의 자리, "넓은 레인과 DP"의 크기와 그 앞 웨이브의 "판정 글자와 콤보" 줄은 이 표가 대신한다.

| 확인 | 결과 |
| --- | --- |
| 고치기 전(HEAD 6bc9c41, 1920x1080) | `parts-judge-combo-009`·`-010`·`-100`: 글자 잉크 x 96~301 로 같고, 숫자 잉크의 왼쪽 끝이 487, 455, 415 로 자릿수마다 왼쪽으로 가며 글자와의 사이가 186, 154, 114px 이었다(리뷰 실측). 빈 자리에 옅은 0. BAD·POOR 는 필드 가운데(칸의 x 238, 214), 위 줄은 x 226 에서 시작해 글자의 왼쪽 끝과 130px 어긋났다 |
| 콤보가 늘 때(7키 Normal) | `parts-judge-combo-00007`, `-00067`, `-00648`, `-01234`, `-12345`(코스의 이어받은 콤보로 시작해 한 타): "GREAT 7", "GREAT 67", "GREAT 648", "GREAT 1234", "GREAT 12345". 글자 잉크가 다섯 장 모두 x 145~337. 숫자 잉크의 왼쪽 끝 357, 356, 356, 363, 363(첫 자리 7, 6, 6, 1, 1 이 같은 칸 353~391 안), 오른쪽 끝 386, 424, 463, 502, 538 로 한 자리에 38~39px 씩 오른쪽으로만 자란다. 다섯 자리의 끝 538 은 필드 끝 548 안이다. 옅은 0 이 없다 |
| 판정 다섯 가지(7키 Normal) | `parts-judge-on-the-beat`(PGREAT "GREAT 648"), `-great-fast`("GREAT 649"), `-great-slow`, `-good-fast`, `-good-slow`("GOOD 652"), `-bad-fast`("BAD"), `-missed`("POOR"): 글자 잉크의 왼쪽 끝이 일곱 장 모두 x 145. GOOD 의 숫자는 GOOD 뒤(x 336)에서 시작한다. 오른쪽 끝은 GREAT 337, GOOD 317, BAD 259, POOR 306 |
| 위 줄 | 같은 캡처들: 점수 차의 잉크가 `+` 는 x 145, `-` 는 147 에서 시작하고 FAST·SLOW 는 x 234 에서 시작한다. `parts-line-detail-only`: "FAST" 가 x 145 부터. `parts-line-difference-only`: "-0001" 이 x 147 부터. `parts-judge-ms-*`(Milliseconds + Best): PGREAT "+0002 +0", GREAT "+0003 +40", GOOD "+0004 -100", BAD "+0004 +210", POOR "+0004 -290" 이 같은 자리(점수 차 x 145, ms 는 x 234 부터) |
| 가만히 있는가 | `parts-frames-on-the-beat-000ms`~`496ms` 10장: 색 `84CEFF` 인 픽셀이 x 146~378, y 561~604, 2,012개로 모두 같고 `540ms` 에는 없다. `parts-frames-early-*`(색 `FFD633`): x 146~384, 2,141개로 10장이 같고 `540ms` 에 없다 |
| 위치와 숨김 | `parts-position-up`(+100): 글자 y 461~504, `-down`(-100): y 661~704 로 기본(561~604)에서 정확히 100px, x 는 같다. `parts-position-hidden`: 판정 줄의 픽셀이 없다 |
| 넓은 레인과 다른 모드 | `parts-judge-wide-*`(7키 Wide + Field Right, 필드 1137~1856): 크기 64, 글자 x 1326, 다섯 자리의 끝 1742. `parts-judge-play5-*`(필드 172~548): 크기 46, 글자 x 237, 다섯 자리의 끝 538, BAD·POOR 도 237. `parts-judge-play14-*`(친 2P 필드 1372~1856): 크기 60, 글자 x 1453, 다섯 자리의 끝 1846. `parts-judge-play14-wide-*`(1169~1888): 크기 64, 글자 x 1358, 다섯 자리의 끝 1774. 어느 것도 친 필드에만 뜬다 |
| 게이트(`stage/capture/play_parts.rs`, 960x540) | `the_default_pack_writes_a_combo_behind_the_word_of_a_judgement_and_grows_it_to_the_right`(콤보 7, 67, 648, 1234, 12345: 글자가 픽셀까지 같고, 숫자의 첫 잉크가 첫 칸 안, 마지막 잉크가 그 자릿수의 칸 안, 필드 안), `the_default_pack_starts_every_judgement_and_the_line_over_it_on_one_column`(PGREAT·GREAT·GOOD·BAD·POOR 의 글자와 위 줄이 같은 열에서 1px 안), `the_default_pack_centres_a_judgement_on_every_field_and_keeps_its_combo_inside`(5·7·9·10·14키 x Normal·Wide 와 7키 Field Right: 필드마다 `numberAlign = 1`, 여섯 글자의 x 가 같음, 글자 + 세 자리가 가운데(좌우 차 1px 이하), 다섯 자리가 필드 안). 고치기 전의 팩에서는 셋 다 실패한다 |
| 생성 스크립트 | `uv run tools/generate-images.py play` 로 play 시트만 다시 만들었다(37,578 → 36,694 B, 좌표표 6,666 → 6,665 B). 전체 `--check` 종료 코드 0(10개 파일 일치) |

웨이브 13 단위 S(판정선을 필드의 맨 아래에. 2026-10-12). 필드의 세로 자리에 관한 앞 웨이브들의 수치(판정선 840, 건반 표시 848~912, 키 빔 y 591~839, 판정 글자 y 561 등)는 이 표와 6.3·6.4절이 대신한다.

| 확인 | 결과 |
| --- | --- |
| 요청과 고치기 전 | 사용자: "판정선이 떠있네 판정선은 맨 아래에 딱 붙는 스킨으로". 고치기 전 캡처(`rows-play7-play`, x 80)에서 판정선 840~847, 그 아래 848~911 이 건반 표시(레인이 이어지는 칸), 화면 바탕은 912 부터, 게이지 928 부터였다 |
| 판정선과 필드의 아래 끝(1920x1080 캡처의 세로 한 줄을 픽셀로 잼) | 판정선 904~911(8px), 그 바로 위 903 은 레인 면, 화면 바탕은 912 부터, 그다음 것은 928(게이지) 또는 발치의 글자(DP Normal 은 928 에 놓인 캡션의 윗가장자리 927. 전과 같은 자리다). 5키(x 200), 7키(80, 200), 9키(700, 1200), 10키(200, 1700), 14키(80, 1800), 7키 Wide(100), 5키 Wide + Field Right + Scratch Right(1150), 9키 Wide(560, 1300), 10키 Wide(250, 1600), 14키 Wide(60, 1800), 7키 Field Right + Scratch Right + Graph Off(1400) 전부 같은 값이다. 판정선 아래에 레인 면·구분선·건반 표시가 없다: 커버를 켜지 않은 `rows-*-play`·`held`·`fullcombo` 29장에서 판정선이 놓인 열 전부의 912~927 행을 세면 화면 바탕이 아닌 픽셀이 0개다(DP Normal 3장만 캡션의 윗가장자리 927 행에 116픽셀) |
| 필드를 따라간 것(캡처를 열어 봄) | `rows-play7-play`·`play5`·`play9`·`play10`·`play14` 와 Wide: 노트가 0 에서 904 까지 내려오고(HI-SPEED 2.00 에 GREEN 514 로 전과 같다. 녹색 숫자는 이동 시간이라 영역의 픽셀 높이와 무관하다), 키 빔이 판정선 위 280px 에 서고, 판정 글자(top 624)와 위 줄(584)이 필드와 함께 64 내려왔다. `rows-play7-held`: 누르고 있는 LN 몸통의 아래 끝이 판정선의 윗변에 닿는다. `rows-play7-loading`·`ready`: LOADING 과 막대, READY 가 필드 가운데(top 464), 판정선이 필드 맨 아래에서 펼쳐진다. `rows-play7-fullcombo`, `rows-play7-fail-failed`(폐점 덮개와 FAILED 는 화면 전체) |
| 폭발 | 고리의 중심이 판정선의 가운데(908)이고 912 아래로는 한 픽셀도 그려지지 않는다(위 줄의 집계. `rows-play7-held` 는 누르는 동안의 폭발과 스크래치의 폭발이 떠 있는 프레임이다). 게이지·발치와 겹치지 않는다. 128x128 을 그대로 판정선 가운데에 두면 상자가 844~972 가 되어 게이지(928~952) 위에 걸친다(계산) |
| 커버 3종(`rows-play7-covers-play`: 레인 커버 30%, 리프트 20%, 히든 20%) | 판정선 723~730(904 - 904 x 0.2 = 723), 그 아래 731~911 이 리프트 커버, 912 부터 화면 바탕, 928 게이지. 리프트 수치 200 은 필드 아래 끝 바로 위(기준선 904), 히든 수치 200 은 히든 커버의 위쪽, 레인 커버 수치 300 과 녹색 360 은 커버 끝. 판정 글자·키 빔·폭발이 리프트만큼 올라간다. `rows-play9-lanewidth-wide-covers-play`, `rows-play14-lanewidth-wide-covers-play` 도 같다 |
| 손으로 친 판정 줄 | `parts-judge-great-fast`·`great-slow`·`good-fast`·`good-slow`·`bad-fast`·`missed`·`on-the-beat`, `parts-judge-play5-*`, `parts-judge-play14-*`, `parts-judge-wide-*`: 판정 글자와 FAST/SLOW·점수 차 줄이 새 자리(글자 624, 위 줄 584)에 뜨고 필드 안에 든다. `parts-position-up`·`down`·`hidden` 도 그 자리에서 ±100 |
| 메뉴 미리보기 | `skin-menu-shipped-7k-1920x1080`(SKIN 메뉴)와 `play-menu-default-1920x1080`(PLAY 메뉴, 레인 커버 30%): 미리보기의 판정선이 필드 맨 아래에 붙고 바로 아래에 게이지가 온다. SKIN 메뉴의 목록은 47행이고 Key Display Opacity 행이 없다 |
| 부위 | `parts-play7-bare`·`parts-menu-bare-after`: Key Display 없이 여덟 부위를 0% 로 내린 화면. Key Display Opacity 행은 메뉴에 없다 |
| 덤프(`--strict --frames 120`) | 문서 9개 load ok, 경고 0, pcall 0, 함수 실패 0, 예산 초과 0, 팩 폴더 73 files unchanged. destination: play5 185, play7 197, play9 203, play10 232, play14 256. 읽기 전용 사본도 같다 |
| 생성 스크립트 | `uv run tools/generate-images.py play` 로 play 시트만 다시 만들었다(폭발 128x128 → 128x68. 시트 1024x570 → 1024x510, 36,694 → 32,463 B, 좌표표 6,665 → 6,664 B). 전체 `--check` 종료 코드 0(10개 파일 일치) |
| 게이트 | 새 게이트 `the_default_pack_ends_every_field_on_the_foot_of_its_judgement_line`(`play_rows.rs`. 8.2절) 통과. `cargo test -p rbms-skin --test default_pack` 4 통과. 내장 화면의 골든은 건드리지 않았다 |

웨이브 13 통합(2026-10-12). 판정 오프셋이 노트 필드만 옮기게 된 뒤(`docs/play-options.md` 3절) 기본 팩으로 다시 찍은 캡처다. 1920x1080, `RBMS_SKIN_PACK=<기본 팩> RBMS_SKIN_CAPTURE_DIR=<폴더> cargo test -p rbms-player --lib --locked`(2158 통과, 0 실패).

| 확인 | 결과 |
| --- | --- |
| 판정선과 필드의 아래 끝(다시 잼) | `rows-play5`·`play7`·`play9`·`play10`·`play14-play` 와 각 `lanewidth-wide-play`, `rows-play7-fieldside-right-scratchside-right-scoregraph-off-play`, `rows-play7-held`·`fullcombo`: 판정선 904~911, 912 부터 화면 바탕, 928 게이지. 판정선이 놓인 열의 912~927 행에 화면 바탕이 아닌 픽셀 0개(DP Normal 두 장만 927 행의 캡션 윗가장자리 116픽셀. 전과 같다). `rows-play7-covers-play`: 리프트 20% 에서 판정선 723~730, 그 아래 리프트 면이 912 까지 |
| 오프셋 0 과 +100 의 같은 곡 시각(150 BPM, HS 2.00, 줄이 판정선에 닿기 100 ms 전. `timing_capture_tests.rs`) | `offset-0-100ms-before-the-row`: 다음 줄 y 775~788, 그 위 줄 323~336, TIMING `+0`. `offset-plus-100-100ms-before-the-row`: 같은 줄이 888~901 로 판정선(904) 바로 위, 그 위 줄 436~449, TIMING `+100`. 113px 아래 = 1.13 px/ms x 100 ms. 오프셋 0 의 1 ms 전 프레임(`offset-0-1ms-before-the-row`: 887~900, 435~448)과 1px 안에서 같다. 두 판의 판정 수는 같다(PGREAT 24) |
| 판정선과 판정 사이 | `offset-plus-100-50ms-before-the-row`: 줄이 판정선에서 이미 사라졌고(남은 줄 40~53, 492~505 는 오프셋 0 의 50 ms 뒤 프레임과 같다) 판정은 아직이다(콤보 24, 폭발 없음). `offset-minus-100-50ms-after-the-row`: 판정이 났고(콤보 27, 폭발과 키 빔) 그 줄은 아직 판정선 위 831~844 에서 내려오는 중이다. 노트는 판정과 무관하게 판정선에 닿을 때 사라진다(원본 `LaneRenderer.java:546`) |
| 자동 보정 | `adjusted-offset-forty-early`(40 ms 빠른 입력 6번): 표지 `TIMING AUTO`, 숫자 `-6`, GREAT 6 이 전부 FAST |
| 문구 | `select-panel3`: 5번 칸 `JUDGE TIMING` / `0 MS` / `NOTES 1 MS LATER`, 7번 칸 `NOTES 1 MS EARLIER`. 칸 폭(224) 안에 들고 옆 칸과 겹치지 않는다. `keys-2-play-1920x1080`: `JUDGE OFFSET +1 MS (HITTING LATE)`. `play-menu-number-offset-selected-1920x1080`: JUDGE OFFSET 의 도움말 `Draw the notes this many ms earlier (+) or later (-) than the song; raise it if you hit late, lower it if you hit early`, 미리보기의 판정선이 필드 맨 아래 |
| 덤프·생성 스크립트 | `--strict --frames 120`: 문서 9개 load ok, 경고·pcall·함수 실패·예산 초과 0, 73 files unchanged, destination 수는 위 표와 같다(select 503). `generate-images.py --check` 종료 코드 0 |

웨이브 14 단위 T(모노톤 팔레트와 색 옵션, 온전한 폭발 고리. 2026-10-12). 캡처는 1920x1080 이고 `RBMS_SKIN_PACK=<기본 팩> RBMS_SKIN_CAPTURE_DIR=<폴더> cargo test -p rbms-player --locked capture`(65 통과, 569장)로 찍었다. 고치기 전 팩(작업 전 사본)과 옵션 기본값을 바꾼 사본 둘(Theme Slate + Accent Sky, Accent Amber + Note Colors Mono + Bomb Color White)은 `named_by_the_environment` 테스트(16 통과)로 찍어 비교했다.

| 확인 | 결과 |
| --- | --- |
| 요청 | 사용자: "스킨의 배경색의 기본 컬러는 검은색이여야해 지금은 약간 파란색이네 전체적으로 검/회색/흰 의 모노톤으로 어레인지해봐 물론 플레이하는 노트와 bomb 은 지금 그대로 흰색 파란색이 맞긴하고. 혹은 이거의 색상을 조절할수도있게끔해두고". 고치기 전의 바탕은 `0C0F14`, 면·선·글자 전부가 파란 기를 띤 청회색이었다 |
| 바탕(기본값. 픽셀 통계) | 선곡(`select-*`, `pack-search-*`, `first-run-*`), 결정 9장, 플레이(`play5`·`play7`·`play9`·`play10`·`play14`, `rows-*`, 손으로 친 `play7-hand-*`·`play14-hand-*`), 결과 10장, 코스 결과 10장 전부의 왼쪽 위 픽셀이 (0, 0, 0)이다. 무채색 픽셀의 밝기 분포는 0, 22, 38, 60, 128, 184, 242, 255 와 선곡 막대의 20(바탕 위)·54(선택된 막대의 `raised` 위)에 몰려 있다(3.1절의 단계 그대로). 결정의 페이드 끝과 결과의 0ms 는 전부 0 이다 |
| 색을 가진 픽셀(채널 차 8 초과의 비율) | 그림이 없는 결정(`decide-1500ms`) 0.00%, 코스 결과 0.6%(판정 이름·막대·FAILED·게이지 선), 결과 5.4%(타이밍 분포 그래프의 판정 띠), 스테이지 그림이 있는 선곡 7.1%·결정 12.6%, BGA 가 없는 7키 플레이 0.3~1.2%(노트, 판정 글자, 게이지, GREEN, 범례), BGA 자리의 스테이지 그림이 있는 `play7-*` 22~24%. 캡처를 열어 본 것(`decide-rich-1500ms`, `select-songs-3000ms`, `select-courses-3000ms`, `select-panel1`, `rows-play7-held`, `rows-play9-play`, `play7-fullcombo`, `result-clear-3000ms`, `course-result-fail-3000ms`)에서 색은 3.1절 "색으로 두는 것"의 것뿐이다: 난이도 막대·레벨·이름, 램프, 판정 이름과 막대, 클리어 이름, MINE, 노트·키 빔·폭발, 판정 글자, 게이지, GREEN, TARGET 막대, 그래프와 범례, 스테이지 그림. 채널 차 1~8 의 옅은 색은 흰 건반 노트·키 빔(`F1F4F8`)과 그 주변의 보간뿐이다 |
| 면의 구분과 글자 | `rows-play7-held`: 레인이 검정과 `161616` 으로 번갈아 보이고 그 사이 선 `3C3C3C` 가 선다. BGA 면·스코어 그래프의 바탕(`161616`)이 화면 바탕과 갈린다. `select-songs-3000ms`: 막대(`141414`)가 바탕 위에, 선택된 막대(`262626` 위 `363636`)가 다른 막대 위에 뜬다. `select-panel1`: 시트(`262626`) 위의 항목 이름·안내(`808080`)와 값(`F2F2F2`)이 읽힌다. `result-clear-3000ms`: 그래프 면과 캡션 |
| 노트와 폭발의 조각 | 고치기 전 시트와 조각을 이름별로 픽셀 비교: 노트 6색 x 5종 30개, `hell_drain`, `mine`, 판정 글자 5, 콤보 숫자 3, `beam`, `pulse`, `bar_h`, `bar_v`, 게이지의 켜진 칸(상태 0, 4, 5) 18개가 같다. 달라진 것은 `bomb`(128x68 → 128x128)와 Mono 의 게이지 상태 1~3(바탕·선 쪽으로 섞는 칸) 18개, 선곡의 막대 7종과 `lamp_strip` 이다. `_slate` 조각(게이지 36, 막대 7, 램프 띠)은 고치기 전의 같은 이름 조각과 전부 같다 |
| 폭발(`rows-play7-held`, `play7-play-06500ms`, `rows-play14-notecolors-white-yellow-bombcolor-white-held` 를 열어 봄) | 고리가 온전한 원이고 중심이 판정선(908)에 있다. 아래 절반이 게이지 위에 겹쳐 그려지고(게이지 칸이 고리 안으로 비친다) DP Normal 에서는 발치의 HI-SPEED 캡션 위에 걸친다. 스크래치의 폭발은 빨강, 검은 건반은 하늘색, 흰 건반은 흰색이다. 게이트(`…ends_every_field_on_the_foot_of_its_judgement_line`)가 22가지 배치에서 폭발 상자가 정사각형이고 가운데 행이 판정선의 가운데 행인지 본다 |
| Theme = Slate + Accent = Sky 와 고치기 전(같은 하니스·같은 차트 169장) | 89장이 픽셀까지 같다(선곡 18장과 선곡 위의 검색 9장·오버레이 10장, 결정 9장, 결과 10장, 코스 결과 10장, 플레이의 로딩·준비·풀 콤보·종료·페이드·폐점 9장, 손으로 친 프레임 12장, `pack-*` 2장). 다른 80장: 폭발이 떠 있는 플레이 프레임 7장(`play5`·`play7` 2장·`play9`·`play10`·`play14`·`play-sample`)은 필드 아래 끝(912) 위에서 채널 값 1 인 픽셀 0~9개(판정 글자의 보간. 시트 안의 자리가 바뀌어서), 912 부터 아래에서 293~3,188픽셀(고리의 아래 절반). `play7-fail-close-1` 은 채널 값 1 인 픽셀 1개. 손으로 친 48장과 판정 프레임 20장은 판정 글자 자리의 채널 값 1 인 픽셀 1~3개 또는 폭발의 아래 절반. `pack-select`·`pack-play7` 은 맥동과 진행 막대(실행 시각). `overlay-guide`·`overlay-system` 은 앱이 그리는 안내의 문구(다른 단위의 변경) |
| Accent(7키 `rows-play7-accent-<sky\|amber\|green\|pink>-play`, Amber 는 로딩·준비·`held`·풀 콤보·커버까지) | 왼쪽 위 (0, 0, 0). `rows-play7-accent-amber-covers-play` 를 열어 봄: 판정선, 레인 커버 끝의 선, 진행 막대, 스코어 그래프의 자기 점수 막대, 가득 찬 게이지의 수치, AUTOPLAY 가 호박색이고 노트·판정 글자·GREEN·TARGET 막대·게이지 칸은 그대로다. Amber 사본의 `select-songs-3000ms`: 선택 막대의 띠, LN 라벨, 켜진 별이 호박색이고 나머지는 Mono 그대로다 |
| Note Colors 와 Bomb Color | `rows-play7-notecolors-<white-deepblue\|white-yellow\|white-green\|mono>-<loading\|ready\|held\|play\|fullcombo>`, `…-mono-bombcolor-white-*`, `rows-play14-notecolors-white-yellow-bombcolor-white-*`. 열어 본 것: Mono 는 검은 건반 노트와 그 LN 몸통이 회색, 스크래치 노트가 밝은 회색이고 폭발이 전부 흰 고리(Bomb Color White). 14키 White-Yellow 는 두 필드의 검은 건반 노트·LN 이 노랑, 스크래치는 빨강, 폭발은 흰색 |
| 덤프(`--strict --frames 120`) | 저장소의 팩, 읽기 전용 사본, 옵션 기본값을 바꾼 사본 둘 모두 문서 9개 load ok, 경고·pcall·함수 실패·예산 초과 0, 73 files unchanged. destination 수는 전과 같다(decide 59, select 503, play5 185, play7 197, play9 203, play10 232, play14 256, result 147, courseresult 84). 옵션 행은 decide 4, select 3, play5·7 11, play9 8, play10·14 8, result·courseresult 2 |
| 생성 스크립트 | `uv run tools/generate-images.py` 로 전부 다시 만들었다(select 1024x479 → 1024x941, 15,660 → 28,422 B. play 1024x510 → 1024x570, 32,463 → 36,843 B. common·decide·result 는 바이트가 그대로다). `--check` 종료 코드 0(10개 파일 일치). 이미지 합계 120,733 B |
| 게이트 | `cargo test -p rbms-skin --test default_pack --locked` 4 통과. `play_rows` 의 게이트 넷 통과(새 게이트 `…on_black_whichever_colours_its_rows_say_unless_the_theme_is_slate` 는 다섯 문서 x 색 옵션의 모든 항목 65가지를 컴파일해 그린다). `gate.rs` 는 `theme.lua` 의 첫 `bg` 줄(Mono)을 바탕색으로 읽어 모서리를 본다. 내장 화면의 골든은 건드리지 않았다 |

확인하지 못한 것

| 화면 | 내용 |
| 색(웨이브 14) | 실제 창과 실제 모니터에서의 단계 구분(캡처의 픽셀 값과 명도 비 계산으로만 봤다. 어두운 방의 밝은 패널에서 `161616` 과 검정이 얼마나 갈리는지는 기기마다 다르다), SKIN 메뉴에서 색 옵션을 고르는 조작과 "모든 화면에 적용"(메뉴 단위의 일이다), Accent 의 Green·Pink 와 Note Colors 의 대안을 준 선곡·결정·결과(강조색은 Amber 사본으로, 노트 색은 플레이에만 있다), 5·10키의 Note Colors, 9키의 Bomb Color White, 실제 BGA 위에 겹친 폭발, 색각 이상에서의 구분(Accent White 는 색이 아니라 꼴로 가른다) |
| 플레이(웨이브 13) | 실제 창에서의 모습(캡처 하니스의 1920x1080 과 640x360 으로만 봤다), 실제 BGA 가 있는 차트, 9키에서 놓친 노트가 판정선에서 사라지는 장면(전에는 건반 표시의 아래 변까지 떨어졌다. `dst2` 를 뺀 것은 덤프와 게이트로만 확인했다) |
| --- | --- |
| 플레이(웨이브 10C) | 실제 BGA 가 있는 차트에서 BGA Opacity 의 중간값(하니스의 차트에는 BGA 가 없어 면과 "NO BGA" 로만 봤다), 커버 3종의 중간값, BAD 의 필드 글자, 콤보 10,000 이상, SKIN 메뉴(F12)에서 불투명도 행을 움직이는 조작과 그 미리보기(메뉴 단위의 일이다), 실제 창(1280x720)에서의 판정 줄. Judge Position 을 크게 옮긴 때의 겹침은 계산으로만 봤다: 약 -210 아래로 내리면 판정선·리프트·히든 수치의 자리(y 820~852)에 닿고, 레인 커버를 판정 줄 높이(기본 위치에서 커버 62~73%)까지 내리면 커버 끝의 수치와 겹친다. 사용자가 정한 위치이므로 그대로 둔다 |
| 공통 | 한글·일본어 제목의 폭·기준선·축소(하니스의 글꼴이 Inter 뿐이다. 7절 보완 9). 실제 창(1280x720)에서의 모습 |
| 선곡 | 호버 강조, 버튼·리플레이 칸·막대 클릭, 스크롤·볼륨 슬라이더 드래그, 검색 칸 클릭 입력, 리플레이가 있는 슬롯과 선택 밑줄, 즐겨찾기 켜진 별, 코스 트로피, 9버튼 배치의 시트, 스크롤 중 가림 경계에서 막대가 잘리는 모습, MINE 과 RANDOM 이 같이 붙은 곡과 CN·HCN 라벨, 사이드 메뉴의 소리. 검색 오버레이의 캡처는 곡이 바뀐 직후(타이머 11 의 0ms)라 곡 정보가 아직 투명한 프레임이다. "SCORE (SET IN SETTINGS)" 와 옵션 시트의 등장·퇴장 중간은 팩 사본으로만 봤다 |
| 플레이(Wide) | 실제 창과 실제 BGA(그림·동영상)가 있는 차트에서의 모습, 4자리 판정 수·5자리 EX SCORE·10분 이상의 남은 시간이 좁아진 상자(9키 Graph On 의 판정 수 268, 7키 Graph Off 의 타이밍 332)에 들어가는 모습(숫자는 `fit` 으로 줄어들게 되어 있고 폭은 계산으로만 확인했다), Milliseconds·Fast-Slow 판정 상세(손으로 치는 하니스는 Normal 로만 돈다. 필드 폭에서 가운데를 잡는 같은 코드다), SKIN 메뉴(F12)의 미리보기에서 행을 바꾸는 조작 |
| 플레이 | 실제 코스 런의 단계 글자, BAD·POOR 의 필드 글자(하니스의 손 플레이는 GREAT·GOOD·PGREAT 만 친다), HCN 감쇠·CN 끝, 어시스트·이지·EX 하드·해저드 게이지, 놓친 노트가 9키의 하한까지 떨어지는 장면, 리플레이 분석 띠가 발치를 덮는지, 남은 시간이 10분 이상인 실제 차트 |
| 결과 | 앱에서 건반·턴테이블로 게이지를 넘긴 뒤의 그래프(넘어가는 것 자체는 장면 테스트 `the_turntable_of_five_keys_turns_the_gauge_and_runs_the_chart_again`, `the_graph_key_turns_the_gauge_to_the_next_kind` 로 확인했다. 캡처에는 그 프레임이 없다), 실제 서버에 붙은 IR 오버레이 여러 줄, 앱에서의 DP·9키 결과 |

## 9. 단위 분할과 진행

| 단위 | 내용 | 소유 파일 | 상태 |
| --- | --- | --- | --- |
| W8-2a 골격 | 공통 모듈, 생성 스크립트, 결정 화면, 이 설계서 | `lua/{theme,props,ui,motion,parts}.lua`, `tools/**`, `decide.luaskin`, `lua/decide/**` | 완료(웨이브 8A) |
| W8-2b 선곡 | 6.2절 전부(곡 목록, 곡 정보, 필터 버튼, 리플레이 슬롯, 사이드 메뉴 2패널, 옵션 시트 3종) | `select.luaskin`, `lua/select/**`, `tools/sections/select.py`, `image/select.png`, `lua/atlas/select.lua` | 완료(웨이브 8B) |
| W8-2c 플레이 | 6.3·6.4절. `lua/play/main.lua` 하나와 문서 5개, 캡처 하니스의 9키 런 | `play5.luaskin`, `play7.luaskin`, `play9.luaskin`, `play10.luaskin`, `play14.luaskin`, `lua/play/**`, `tools/sections/play.py`, `image/play.png`, `lua/atlas/play.lua` | 완료(웨이브 8B) |
| W8-2d 결과 | 6.5·6.6절. `lua/result/main.lua` 하나와 문서 2개 | `result.luaskin`, `courseresult.luaskin`, `lua/result/**`, `tools/sections/result.py`, `image/result.png`, `lua/atlas/result.lua` | 완료(웨이브 8B) |
| 통합 | 엔진 결함 2건 수정(7절), 공통 승격(4절), 화면 사이 일관성(3.1절의 판정 색 순서와 MAX 램프 색, 6절 머리말), 이 설계서의 최종 좌표, 게이트와 캡처 전체(8.4절) | 공통 파일 전부와 이 설계서 | 완료(웨이브 8B) |
| 웨이브 9A U4 | FAST/SLOW 기본 켬, IIDX 식 판정(글자·색·5행), ModernChic 의 램프·난이도 색과 클리어 표기, 손으로 치는 캡처 하니스 | 팩의 `lua/**`, `tools/**`, `image/{play,select}.png`, `lua/atlas/{play,select}.lua`, `apps/rbms-player/src/stage/capture/by_hand.rs`, 이 설계서 | 완료(웨이브 9A) |
| 웨이브 10A 단위 A | Lane Width 옵션(Normal / Wide. 레인 1.5 배)과 Wide 의 배치, 좁은 곡 머리, 옵션을 바꿔 찍는 캡처와 게이트 | `lua/play/{layout,main,info}.lua`, `apps/rbms-player/src/stage/capture/play_rows.rs`(`capture.rs` 에는 mod 한 줄), 이 설계서 | 완료(웨이브 10A) |
| 웨이브 10C 단위 S | 판정 줄을 정적으로(확대·미끄러짐·깜빡임·색 돌림·페이드 제거, 글자 왼쪽 끝 고정, 콤보 고정 칸), 판정 위 줄(점수 차 + FAST/SLOW), 옵션 Score Difference·Judge Display, 오프셋 Judge Position 과 부위 불투명도 19개, `ui` 의 `s:within`, 생성 스크립트의 `counter_strip`·`guarded`, 판정 줄·부위 캡처와 게이트 | 팩의 `lua/play/**`, `lua/{ui,theme}.lua`, `tools/{generate-images.py,sprites.py,sections/play.py}`, `image/play.png`, `lua/atlas/play.lua`, `apps/rbms-player/src/stage/capture/play_parts.rs`(`capture.rs` 에는 mod 한 줄), 이 설계서 | 완료(웨이브 10C) |
| 웨이브 10D | 판정 줄 다시 놓기: 콤보를 글자 바로 뒤에 왼쪽 정렬로(옅은 0, 11칸 띠, `counter_strip` 삭제), 모든 판정 글자와 위 줄의 왼쪽 끝 `L`, 다섯 자리가 필드 안에 드는 크기 규칙, 좌우를 잉크에 맞춘 판정 글자. 엔진: judge 의 `numberAlign`. 하니스: 이어받은 콤보로 시작하는 런, 모드별 BAD 타 | 팩의 `lua/play/field.lua`, `tools/{sprites.py,sections/play.py}`, `image/play.png`, `lua/atlas/play.lua`, `crates/rbms-skin/src/model/objects.rs`, `crates/rbms-render/src/skin_render/{judge,draw}.rs`, `apps/rbms-player/src/stage/capture.rs`, `capture/{play_parts,by_hand}.rs`, 이 설계서 | 완료(웨이브 10D) |
| 웨이브 10A 통합 | 선곡 패널 3 의 NOTE DISPLAY TIME 두 칸(이동 시간과 녹색 숫자, 꺼짐 표기), 플레이의 TIMING 표지(AUTO), 결과와 패널 3 의 안내 문구에서 옮길 수 있는 키 이름 정리. 엔진: 플레이 화면이 image index 75 를 답한다, 묶인 가림막 키를 누른 뒤 1.5초 동안 op 270(LANECOVER1_CHANGING)이 켜져 발치의 COVER 수치가 뜬다 | `lua/select/sheet.lua`, `lua/play/info.lua`, `lua/result/main.lua`, `apps/rbms-player/src/skin_host/play.rs`, `stage/play/mod.rs` | 완료(웨이브 10A) |
| 웨이브 13 단위 S | 판정선을 필드의 맨 아래에: 건반 표시(Key Display)와 오프셋 105 삭제, 노트 영역 840 → 904, 필드 안의 것을 64 내림, 필드 아래 끝에서 끊기는 폭발, 9키의 `dst2` 삭제, "판정선의 아래 변 = 필드의 아래 끝" 게이트 | 팩의 `lua/play/{layout,field}.lua`, `tools/sections/play.py`, `image/play.png`, `lua/atlas/play.lua`, `apps/rbms-player/src/stage/capture/{play_rows,play_parts}.rs`, `stage/skin_menu/tests/opacity.rs`, 이 설계서, `docs/skin.md` 의 부위 목록 | 완료(웨이브 13) |
| 웨이브 13 통합 | 선곡 패널 3 의 JUDGE TIMING 두 칸의 동작 문구(`NOTES 1 MS LATER`·`NOTES 1 MS EARLIER`), 자동 보정 부호가 원본으로 돌아간 뒤의 `the_default_pack_is_drawn_for_a_judge_offset_that_adjusted_itself` 기대값(-6), BGA 를 오프셋과 무관하게 플레이 타이머로(엔진), 최종 캡처와 문서 | `lua/select/sheet.lua`, `apps/rbms-player/src/stage/play/{mod.rs,timing_capture_tests.rs,timing_tests.rs}`, `stage/play/screen_tests/default_pack_tests.rs`, 이 설계서 | 완료(웨이브 13). 폭발의 모양은 사용자 결정 대기 |
| 웨이브 14 단위 T | 모노톤 팔레트(Theme: Mono 기본, Slate 보존), 강조색 옵션(Accent), 노트 색 옵션(Note Colors)과 폭발 색 옵션(Bomb Color), 판정선 가운데의 온전한 폭발 고리, 색을 본체 패스에서 정하는 구조(`theme.apply`, `theme.sprite`), 테마별 구운 조각(`_slate`), 색 옵션 게이트와 캡처 | 팩의 `lua/theme.lua`, `lua/play/{main,layout,field,info}.lua`, `lua/select/{main,list}.lua`, `lua/decide/main.lua`, `lua/result/{main,graphs}.lua`, `tools/{sprites.py,sections/play.py,sections/select.py}`, `image/{play,select}.png`, `lua/atlas/{play,select}.lua`, `apps/rbms-player/src/stage/capture/{play_rows,gate}.rs`, 이 설계서, `docs/skin.md` 3절 | 완료(웨이브 14) |
| 웨이브 14 통합 | 발치를 3열에서 4열로 바꿔 COVER 옆에 LIFT 수치(number 314, op 270)를 둔다(GREEN 의 자리가 7키 기준 224 → 185 로 옮겨진다): 꺼진 리프트를 START + SELECT + 턴테이블로 움직일 때 화면에 값이 없던 것(리뷰 발견). 가림막을 바꾸는 동안에는 켬·끔과 무관하게 두 값이 함께 뜬다. 내보낸 팩의 오프셋·불투명도가 프레임에 그려지게 한 엔진 수정과 그 회귀 테스트, 최종 캡처와 문서 | 팩의 `lua/play/info.lua`, `apps/rbms-player/src/{skin_screen.rs,skin_preview.rs}`, `stage/capture/play_parts.rs`, `stage/play/controls/capture_tests.rs`, 이 설계서 | 완료(웨이브 14) |

통합에서 공통으로 올린 것: 값 없음 표지 검사(`ui.has_value`), 시트 조각을 graph·slider 의 원본으로 옮기는 것(`s:sprite_source`), 기준선·가운데 정렬의 글자 윗선(`theme.text_top`, `theme.centered_top`), 팔레트의 16진 문자열(`theme.hex`), 이름표(`theme.clear`, `rank_name`, `gauge_name`, `random_name`), 판정 6행(`parts.judges`), Stage Image 옵션(`parts.stage_image*`), `event_index` 이름 고르기(`parts.choice`), bpmgraph 정의(`parts.bpm_graph`), 그래프·숫자의 열거값(`props.judge_graph`, `back_texture`, `padding`, `graph_angle`, `slider_angle`, `filter`)과 화면 모듈에 흩어져 있던 속성 id(`props.option` 의 60·61·261·1262·1263, `props.number` 의 421·422, `props.index` 의 301·371), 생성 스크립트의 팔레트 읽기(`sprites.palette`).

다음: W8-3(내장·설치). 내장 표는 `font/`, `image/`, `lua/`, `sound/`, `*.luaskin` 을 담고 `tools/` 는 담지 않는다. 화면을 고칠 때의 끝: 덤프 `--strict` 통과, 고친 화면의 캡처를 직접 열어 8.3절 항목 확인, 생성 스크립트 `--check` 통과, 6절의 표 갱신.
