# 스킨 가이드

최종 갱신 2026-10-15 · 대응 커밋 ea698a9. S18·S19 는 스킨 문서·기본 팩·Lua 환경을 바꾸지 않았다(기본 팩 65개 파일, 1,772,662 B 를 ea698a9 에서 다시 셌다. 테스트 빌드가 Lua 의 프레임 시계를 멈추는 `SkinLua::hold_frame_clock` 은 출하 동작과 무관하다: `architecture.md` "Testing and the lint gate"). 반영한 웨이브(모두 커밋됨): 웨이브 16(스코어 그래프의 TARGET 범례가 실제로 쓰는 목표의 이름을 쓴다: 3.4.1절. 선곡 패널의 점수 칸은 램프만 있는 기록이면 `-`: 6절), 웨이브 18(SCRATCH AUTO 가 치는 레인의 노트를 엔진이 초록으로 그린다: 6절), 웨이브 15(앱이 덧그리는 UI 와 내장 화면의 무채색 팔레트, 값을 못박지 않는 `theme.ron` 템플릿: 3.4.3절, 6절), 웨이브 14(기본 스킨의 모노톤 팔레트와 색 옵션 넷, 온전한 폭발 고리, 발치의 LIFT 수치: 3.4절·3.4.3절. 조절한 팩의 내보내기·가져오기와 같은 옵션을 모든 화면에 적용: 2.8절), 웨이브 13(기본 스킨의 판정선을 필드 맨 아래로: 3.4.1·3.4.2절. 노트 필드를 판정 오프셋만큼 앞서 그림: 1.2절)

R-BMS 는 화면마다 스킨 문서 하나를 읽어 그 화면을 단독으로 그린다. 문서가 없거나 읽히지 않는 화면은 내장 화면이 그린다. 기본으로 설치되는 자작 스킨 `rbms-default` 가 첫 실행부터 선곡 · 결정 · 플레이 · 결과를 그리며, beatoraja 용으로 만든 외부 Lua 스킨(`.luaskin`)도 수정 없이 읽는다. 이 문서의 독자는 스킨을 쓰는 사용자와 스킨을 만드는 사람이다.

| 더 자세한 문서 | 내용 |
| --- | --- |
| `docs/plan/2026-10-09-lua-skin-compat.md` | 정본 사양(목표, 결정, 구조, 웨이브별 진행과 완료 기록) |
| `docs/plan/2026-10-11-default-skin-design.md` | 기본 스킨 설계서(격자, 공통 Lua 모듈 API, 화면별 최종 좌표, 엔진 지원 확인표, 검증 기록) |
| `docs/acknowledge/2026-10-09-lua-skin-compat-decisions.md` | 사용자 결정 D1~D10과 기각한 대안 |
| `docs/reference/skin-compat/README.md` | 조사 보고서 색인(beatoraja 의 스킨 엔진 계약, 속성 id 표 등) |
| `docs/plan/2026-10-11-skin-menu.md` | SKIN 메뉴 설계서(배치, 조작, 미리보기 엔진, 경계 API) |
| `docs/play-options.md` | 플레이 옵션의 정의와 PLAY 메뉴(7절. 플레이 스킨의 옵션·오프셋·불투명도를 플레이 화면 미리보기를 보며 바꾸는 화면), 기본 스킨의 판정 표시(8절) |
| `docs/theme.md` | 내장 화면의 색 테마 `theme.ron` |

## 1. 개요

### 1.1 지원 형식과 화면

| 항목 | 내용 |
| --- | --- |
| 문서 형식 | `.luaskin`(Lua 5.2 프로그램), `.json`, `.json5`. 팩 폴더 바로 아래의 이 확장자 파일이 문서다(대소문자 무시) |
| 화면 결정 | 파일 이름이 아니라 문서 헤더의 `type` 으로 화면을 정한다. 같은 type 의 문서가 둘이면 파일 이름순(대소문자 무시)으로 첫 문서다 |
| 크기 | 헤더 `w`, `h` 가 beatoraja 의 해상도 목록(640x480, 1280x720, 1920x1080, 3840x2160 등 15종)에 없으면 1280x720 으로 본다 |

| type | 화면 | 기본 스킨 문서 | 비고 |
| --- | --- | --- | --- |
| 5 | 선곡 | `select.luaskin` | |
| 6 | 결정 | `decide.luaskin` | 문서가 있을 때만 이 화면이 열린다 |
| 0, 1, 2, 3, 4 | 플레이 7키, 5키, 14키, 10키, 9키 | `play7`, `play5`, `play14`, `play10`, `play9` `.luaskin` | |
| 7 | 결과 | `result.luaskin` | |
| 15 | 코스 결과 | `courseresult.luaskin` | |
| 8 | 키 설정 | 없음 | 기본 스킨에는 문서가 없어 내장 화면. 외부 팩이 type 8 문서를 가지면 읽어 그린다 |
| 9 | 스킨 설정 | 없음 | 스킨 설정 화면은 내장 SKIN 메뉴다(§2.2). type 9 문서는 헤더를 읽지만 그리지 않고 LOADED 행이 `SCREEN NOT SUPPORTED - DRAWING THE BUILT-IN ONE` 을, 미리보기 자리가 `THIS SCREEN IS NOT DRAWN BY A SKIN` 을 보인다 |
| 16~18 | 24키 플레이(싱글, 더블, 배틀) | 없음 | 24키 모드는 BMSON 의 `mode_hint` 로만 생기며 항상 내장 필드로 그린다 |

내장 화면은 지우지 않았다. 스킨이 없는 화면, 24키, 스킨 로드 실패의 폴백이 이것이다.

### 1.2 beatoraja 와 같은 점

| 같은 것 | 근거 |
| --- | --- |
| 문서 모델(`SkinDef` 최상위 46필드), 속성 id 968개, 타이머 id, destination 보간(`acc`, `loop`, 오프셋) | `crates/rbms-skin/src/model.rs`, `dst.rs`, `property/generated/` |
| `.luaskin` 은 화면에 들어갈 때마다 새 Lua 상태에서 진입 파일을 두 번 실행한다(헤더 패스 `skin_config == nil`, 본체 패스) | `crates/rbms-skin/src/loader/lua_skin.rs` |
| 본체가 만든 함수 값(draw, value, timer, act)을 매 프레임 부른다. 오류는 기본값(false, 0, 빈 문자열)으로 대신하고 재호출한다 | `crates/rbms-skin/src/lua/main_state.rs`, `skin_render/object.rs` |
| 객체 종류: image, imageset, value, floatvalue, text, slider, graph, note, gauge, judge, songlist, 커버, bga, gaugegraph, judgegraph, bpmgraph, timingdistributiongraph, timingvisualizer, hiterrorvisualizer | `crates/rbms-render/src/skin_render/` |
| 장면 수명(`input`, `scene`, `fadeout`), 플레이 상태기계(PRELOAD, READY, PLAY, FAILED, FINISHED), 타이머 번호, 오프셋 1~5 | `apps/rbms-player/src/stage/scene_life.rs`, `stage/play/`, `skin_host/play_timers.rs` |
| 판정 타이밍(number 12, `judgetiming`)은 note 객체(노트 필드)를 그리는 시각에만 더해진다. +N 이면 노트와 마디선이 N ms 먼저 판정선에 닿고, 스킨이 읽는 현재 BPM(160)도 그 시각의 것이다. 판정, 소리, bga 객체, 타이머는 옮겨지지 않는다(웨이브 13. `LaneRenderer.java:300-301`, `SkinBGA.java:42-44`) | `apps/rbms-player/src/stage/play/mod.rs`(`drawn_chart_us`, `played_chart_us`), [play-options.md](./play-options.md) 3절 |

### 1.3 의도적으로 다른 점

사양 E1~E10(`docs/plan/2026-10-09-lua-skin-compat.md` §2)의 현재 구현 상태다.

| # | 다른 점 | 효과 |
| --- | --- | --- |
| E1 | Lua 격리. `require`, `dofile`, `loadfile`, io 읽기는 스킨 팩 폴더 안만 본다. io 쓰기는 팩이 아니라 쓰기 오버레이로 간다. 네트워크는 항상 실패한다 | 스킨이 팩 폴더나 시스템을 건드릴 수 없다(§2.6, §4) |
| E2 | Settings, SKIN 메뉴, PLAY 메뉴, Tables, Folders, Practice, 스캔 대기 화면은 내장이다. IR 랭킹 패널, 기록 모달, 필터 패널, 옵션 오버레이(F1), 첫 실행 안내, 토스트, 디버그 패널, 리플레이 분석 띠는 스킨 위에 앱이 덧그린다 | 선곡 위 오버레이는 스킨 칸과 따로 뜬다(§6) |
| E3 | 기존 키보드 단축키를 유지하고 START(기본 A), SELECT(기본 W)를 새로 둔다. 옵션 패널 1은 START, 패널 2는 SELECT, 패널 3은 둘을 함께(또는 숫자 5) 누르고 있는 동안 열린다 | 패드의 표준 Start/Select 버튼에도 묶인다 |
| E4 | 내장 play/select/result 화면이 폴백으로 남는다 | 문서가 없는 화면, 로드 실패, 24키 |
| E5 | 스킨 폰트에 없는 글자(한글, 일본어 등)는 앱의 다국어 폰트로 그린다. beatoraja 는 사각형으로 그린다 | |
| E6 | 기본 창 1280x720. SETTINGS 의 DISPLAY 묶음에 RESOLUTION(1280x720, 1600x900, 1920x1080, 2560x1440, 3840x2160)과 WINDOW MODE(WINDOWED, BORDERLESS)가 있다. 16:9 가 아니면 레터박스. BORDERLESS 는 항상 비율을 맞추고, 일반 창은 LETTERBOX 설정을 따른다. LETTERBOX 기본값은 켬이다 | 설정 판 3 이하의 파일은 읽을 때 LETTERBOX 를 한 번 켠다(그 판은 고르지 않은 기본값도 꺼짐으로 적었다. 원본은 `settings.ron.v3.bak`). 판 4 부터는 적힌 값을 따른다. 계정 동기화로 내려받은 이전 판 설정에도 같은 규칙을 쓴다 |
| E7 | 그리기와 타이밍은 beatoraja 체크아웃(HEAD `8320241d`) 그대로다. 입력 결함만 원작 의도로 고쳤다(luajava `Input.Keys` 의 UP 19, DOWN 20, LEFT 21, RIGHT 22) | |
| E8 | 스킨의 IR 타이머 172~174 와 관련 옵션은 실제 IR 상태를 답한다. 랭킹 목록 값(380~399 등)의 스킨 연결은 후속이다 | §6 |
| E9 | 9키는 기본 스킨에 문서가 있다. ModernChic 같은 9키 문서가 없는 팩에서는 9키만 내장 필드다 | |
| E10 | GPU 표면을 sRGB 가 아닌 포맷으로 골라 색을 바이트 그대로 통과시킨다 | 실제 창의 내장 화면이 이전보다 어둡고 대비가 강하다. 테마 색 재조정 여부는 미결(§6) |

그 밖의 차이(구현 중 확인된 것)

| 차이 | 내용 |
| --- | --- |
| 텍스처 | 조립이 끝난 destination 이 참조하는 source 만 디코드한다(beatoraja 는 객체 생성 시 로드). 화면당 RGBA 1 GiB 예산(`SKIN_TEXTURE_BUDGET_BYTES`). 화면을 떠나면 놓는다 |
| 문서가 오는 동안 | Lua 읽기와 디코드가 끝날 때까지 장면 시계가 멈춘다. 10초(`SCENE_HOLD_LIMIT`)를 넘기면 포기하고 내장 화면으로 간다 |
| 결정 취소 | 결정에서 취소하면 선곡이 다시 시작하지 않고 떠나기 전 상태 그대로 이어진다 |
| 클릭 | 스킨 클릭은 직전 프레임의 입력 맵으로 판정해 다음 프레임에 실행한다 |
| 리플레이 | R-BMS 는 플레이마다 자동 저장하므로 스킨의 슬롯 저장 이벤트는 무동작이다. 슬롯 1 은 이번 플레이의 자동 저장 결과다 |
| 선곡 입력 잠금 | 선곡의 STARTINPUT 은 입력 잠금이 아니다 |
| 곡 검색 | 검색 상자는 하나다. 스킨에 검색 칸(검색어를 쓰는 editable text, ref 30)이 있으면 `/`, 숫자 0, 칸 클릭이 모두 그 칸에 치게 하고 앱의 검색 상자는 뜨지 않는다. 칸이 없는 스킨과 내장 화면에서는 앱의 검색 상자가 뜬다. 어느 쪽이든 글자마다 목록이 좁혀지고(beatoraja 는 확정할 때 한 번), 한글·가나는 입력기로 조합된다. 치는 중에는 글자·편집 키만 검색어가 갖는다: ↑↓·휠·F3·F9·스킨 버튼은 그대로 동작하고, F1·F2·F4·F8·F12·Tab 과 패드 버튼은 입력을 확정한 뒤 동작한다. Enter 는 확정(곡 시작은 한 번 더), Esc 는 취소. 확정한 결과 목록은 일반 목록과 같게 조작되고 Esc 로 검색을 연 폴더와 행으로 돌아간다. 폴더 경로(text 1000)는 beatoraja 의 검색 폴더처럼 `Search : '검색어' > ` 가 된다. 경로를 그리지 않는 스킨에는 앱이 `SEARCH <검색어> N HITS` 표식을 덧그린다 |
| 스킨 옵션이 앱 설정과 겹칠 때 | 스킨 옵션이 우선한다. 스킨이 그리는 화면에서 의미가 없어진 설정 행에는 표지를 붙인다(§2.5) |
| 오프셋의 알파 | 사용자가 준 오프셋의 알파(a)는 그 destination 의 모든 프레임에 더해진다. beatoraja 는 색이 키프레임 사이에서 보간되는 동안에는 더하지 않아(`SkinObject.prepareColor`) 페이드하는 객체가 0% 에서도 번쩍인다. 오프셋이 0 이면 결과가 같다 |
| note·judge 객체의 오프셋 알파 | note(노트 필드)와 judge(판정) 객체는 자기 destination 이 가리키는 오프셋의 알파만큼 전체가 흐려진다. beatoraja 는 이 두 객체의 색을 읽지 않아 알파 오프셋이 듣지 않는다. 마디선은 note 정의의 `group` destination 이 가리키는 오프셋을 따로 따른다. 선곡 휠(songlist) 객체는 원본대로다 |
| judge 객체의 `numberAlign` | judge(판정) 객체에 `numberAlign` 을 쓰면 콤보 숫자를 그 값대로 정렬한다: 0 오른쪽, 1 왼쪽(첫 자리가 고정되고 오른쪽으로 자란다), 그 밖은 가운데. beatoraja 에는 없는 키이고 beatoraja 는 콤보를 언제나 가운데로 모은다. 키를 쓰지 않은 스킨은 beatoraja 와 같게 그려진다. 숫자 value 의 `align` 은 beatoraja 처럼 읽지 않는다 |
| timingvisualizer 의 선 | 눈금자 위의 선도 그 객체의 destination 이 가리키는 오프셋의 알파만큼 흐려진다(beatoraja 는 선에 오프셋을 적용하지 않는다). 키프레임 색의 알파는 원본처럼 선에 적용하지 않는다 |

## 2. 스킨 팩 쓰기

스킨 팩은 `.luaskin`(또는 `.json`, `.json5`) 여러 개가 든 폴더 하나다. 문서는 팩 루트에만 둔다(폴더 안의 하위 폴더는 문서 검색 대상이 아니다. `PACK_SCAN_DEPTH` = 1).

### 2.1 팩을 고르는 법

| 방법 | 동작 |
| --- | --- |
| 아무것도 하지 않음 | 설치된 기본 팩(`<설정 폴더>/skin/rbms-default/`)을 쓴다 |
| SKIN 메뉴의 PACK FOLDER 행 | 오른쪽 키나 Enter 로 폴더 선택 창을 연다. 고른 경로는 `skin.pack` 으로 저장된다. 왼쪽 키로 비우면 기본 팩으로 돌아간다 |
| 환경 변수 `RBMS_SKIN_PACK` | 이번 실행만 그 폴더를 쓴다. 설정에 쓰지 않는다. 비어 있거나 공백이면 무시한다 |
| `--skin file.ron` | 팩이 아니라 내장 플레이 필드의 레이아웃 RON 이다(§2.4) |

우선순위는 `RBMS_SKIN_PACK` > `skin.pack` > 설치된 기본 팩이다. 화면마다 쓸 문서는 이렇게 정해진다.

1. SKIN 메뉴에서 그 화면에 손으로 고른 문서(`skin.selected`).
2. 효력 있는 팩에서 그 화면 type 을 선언하는 문서.
3. 둘 다 없으면 내장 화면. 사용자가 고른 팩에 없는 화면도 기본 스킨이 아니라 내장 화면이다(E9).

PACK FOLDER 행의 표시: 환경 변수가 있으면 `<경로> (RBMS_SKIN_PACK)`, 설정에 있으면 경로, 둘 다 없으면 `DEFAULT (<기본 팩 경로>)`, 기본 팩이 설치되지 않았으면 `(none)`. 지정한 폴더가 디스크에 없으면 맨 앞에 `(MISSING) ` 이 붙는다. 경로가 행보다 길면 경로의 앞부분을 `…` 로 줄이고 끝(폴더 이름)과 경로 앞의 표지를 남긴다(예: `DEFAULT (…skin/rbms-default)`). 커서를 올리면 미리보기 아래에 전문이 나온다.

### 2.2 SKIN 메뉴

스킨 설정은 SETTINGS 가 아니라 독립 화면인 SKIN 메뉴에 있다. 왼쪽이 설정 행 목록, 오른쪽이 지금 고른 화면의 스킨 미리보기이고, 값을 바꾸면 미리보기가 따라 바뀐다. 저장 위치와 형식은 이전의 SKIN 탭과 같다(`settings.ron` 의 `skin` 그룹. 이전 판의 설정 파일이 그대로 읽힌다).

| 들어가는 길 | 동작 |
| --- | --- |
| 선곡에서 F12 | SKIN 메뉴를 연다(내장 선곡과 스킨 선곡 모두. 검색어를 치는 중에는 입력을 확정한 뒤) |
| 스킨의 SKIN 버튼 | 스킨 선택 이벤트(beatoraja 의 id 14)를 부르는 버튼. 기본 스킨 사이드 메뉴의 SKIN 행, ModernChic 사이드 메뉴의 스킨 변경 버튼 |
| SETTINGS 에서 F12 또는 왼쪽 레일 아래 OTHER SCREENS 의 `SKIN MENU` 단추(그 위는 PLAY 메뉴를 여는 `PLAY MENU`, 아래는 `KEY CONFIG`. 웨이브 16) | SETTINGS 위에 연다. 글자를 치는 중이거나 RIVALS·IR PROFILES 목록이 열려 있으면 동작하지 않는다 |
| Esc | 설정을 저장하고 왔던 화면(선곡 또는 SETTINGS)으로 돌아간다 |

| 입력 | 동작 |
| --- | --- |
| ↑ ↓ | 행 커서 이동. 행이 17개를 넘으면 목록이 스크롤되고 오른쪽 아래에 `현재 / 전체` 가 나온다 |
| PageUp PageDown | 한 화면(17행)씩 이동 |
| ← → | 커서 행의 값을 한 칸. 오프셋 행은 1, 불투명도 행은 1%(0~100% 안에서. 아래 주의) |
| Shift + ← → | 커서가 오프셋 행이나 불투명도 행이면 열 칸(10, 10%). 다른 행은 한 칸 그대로다 |
| Enter | 오프셋·불투명도 행에서는 숫자를 직접 친다(아래). 그 밖의 행에서는 → 한 칸과 같다: PACK FOLDER 는 폴더 선택 창, RELOAD·RESET 은 실행. 화면을 떠나지 않는다 |
| 0 (숫자 줄, 넘패드) | 커서가 오프셋 행이면 0 으로, 불투명도 행이면 100% 로 되돌린다 |
| Tab / Shift+Tab | 미리보기 위 화면 띠(7K 5K 14K 10K 9K SELECT DECIDE RESULT COURSE KEYS)에서 다음 / 이전 화면. 띠에 없는 화면(24키 등)은 SCREEN 행의 ← → 로 간다 |
| 행 왼쪽 클릭 | 그 행으로 커서를 옮기고 한 칸 앞으로. 오프셋·불투명도 행은 이름 쪽을 눌렀을 때 그렇고, 슬라이더와 숫자 글자는 아래와 같다 |
| 행 오른쪽 클릭 | 그 행으로 커서를 옮기고 한 칸 뒤로(잘못 누른 클릭을 되돌린다. RELOAD·RESET 은 실행하지 않는다) |
| 화면 띠의 탭 클릭 | 그 화면으로 |
| 휠 | 포인터가 목록 위일 때 목록을 스크롤한다(커서는 그대로). 슬라이더 위에서도 스크롤만 하고 값은 바뀌지 않는다 |
| 숫자 행의 ← → | 그 행의 가장 잔 단위 한 칸(오프셋 1, 불투명도 1%). Shift 를 누르고 있으면 열 칸. 범위 끝에서 멈춘다(불투명도는 100% 에서 멈춘다) |
| 숫자 행의 Enter, 또는 숫자 글자 클릭 | 그 자리에서 숫자를 친다. 지금 값이 채워져 있고 전체가 선택되어 있어 첫 글자가 값을 갈아 쓴다(← → Home End 로 선택을 풀고 고친다). 받는 글자는 숫자·부호·소수점이고 단위(%, ms, x, px)는 쳐도 버린다. 퍼센트 행은 퍼센트 수를 친다 |
| 입력 중 Enter | 확정. 범위를 벗어나면 끝값, 가장 잔 단위로 반올림. 숫자가 아니면(빈 칸, `-`, `1.2.3`) 값을 바꾸지 않고 행에 1.5초 동안 `NOT A NUMBER` 를 보인다. 지금 값과 같은 수를 확정하면 저장값을 건드리지 않는다 |
| 입력 중 Esc | 입력만 취소한다(메뉴를 떠나지 않는다). 그 다음 Esc 가 저장하고 나간다 |
| 입력 중 ↑ ↓ PageUp PageDown Tab, 마우스 누름 | 친 값을 확정한 뒤 그 조작을 한다. 그 밖의 키는 입력이 삼킨다(0 키는 숫자 0 이다) |
| 슬라이더 누름·끌기 | 트랙을 누르면 누른 곳의 값으로 가고, 버튼을 뗄 때까지 포인터를 따라간다(포인터가 행·목록 밖으로 나가도 이어진다. 슬라이더 끝을 넘으면 슬라이더의 끝값). 손잡이를 누르면 값은 그대로이고 잡은 자리를 기준으로 포인터가 움직인 만큼만 따라온다(누르기만 하고 떼면 아무것도 바뀌지 않는다). 끄는 동안 미리보기가 다음 프레임부터 따라오고, 설정 파일은 버튼을 뗄 때 한 번 쓴다. 끄는 중에 키를 누르면 끌기가 거기서 끝난다 |
| 미리보기 위의 클릭·휠 | 아무 일도 없다(미리보는 스킨의 버튼은 동작하지 않는다) |

폴더 선택 창이 떠 있는 동안에는 키와 클릭을 받지 않는다. 미리보기 아래에는 LOADED 값 전문(두 줄까지)과 커서 행의 이름·값 전문, 그 행의 설명이 나온다(목록은 좁아서 긴 이름을 자른다). 숫자를 치는 동안에는 바닥 안내가 `TYPE A NUMBER   ENTER SET   ESC CANCEL   UP DOWN TAB SET AND MOVE ON` 으로 바뀐다. 키를 누르고 있어도 값이 반복되지 않으므로 오프셋을 멀리 옮길 때는 슬라이더로 대강 옮기고 키나 직접 입력으로 맞춘다: 위치·크기 오프셋의 범위는 4001칸이고 슬라이더의 자리는 92개뿐이라 슬라이더 한 픽셀이 약 43칸이다(그 행의 설명에 `The slider is coarse: arrows or ENTER set it exactly` 라고 적혀 있다).

| 행 | 동작 |
| --- | --- |
| PACK FOLDER | §2.1 |
| SCREEN | 아래 행들이 설정하고 오른쪽에 미리보는 화면(PLAY 7KEYS, MUSIC SELECT, DECIDE, RESULT, COURSE RESULT 등 19종 이름) |
| SKIN | 그 화면에 쓸 문서. 팩이 고른 문서는 `PACK: <이름>`, 손으로 고른 문서는 이름만 보인다. 후보는 팩 문서와 `skin.folder`(기본 `<설정 폴더>/skin`, 깊이 3, 최대 512개) 아래 문서다. 폴더는 메뉴에 들어올 때 다시 훑는다 |
| LOADED | 읽은 문서의 크기, 파서, Lua 여부, 경고 수, 작성자. 못 읽었으면 이유의 첫 줄(Lua 의 스택 트레이스는 싣지 않는다), 내장 화면이면 `BUILT-IN SCREEN`. 문서가 없는 까닭이 팩에 있으면 뒤에 붙는다: `BUILT-IN SCREEN - THE PACK FOLDER IS NOT THERE`, `BUILT-IN SCREEN - <파일> DID NOT READ (header pass): <이유>` |
| 문서의 옵션 행 | 문서 헤더의 `property` 마다 한 행. 이름은 `<분류> > <이름>`. 값을 바꾸면 문서를 다시 읽는다 |
| 문서의 파일 행 | 헤더의 `filepath` 마다 한 행. 패턴에 맞는 파일 중에서 고른다. 값을 바꾸면 문서를 다시 읽는다 |
| 문서의 오프셋 행 | 헤더의 `offset` 마다 허용된 축별로 X, Y, W, H, ANGLE, ALPHA 행. 위치와 크기는 ±2000(픽셀. 플레이 화면 전체를 옮기는 오프셋만 퍼센트), 각도 ±360(도), 알파 ±255(-255 면 사라진다). 한 칸 1, Shift 로 10, 슬라이더와 직접 입력(위 표). 오프셋은 저장값을 바로 읽으므로 문서를 다시 읽지 않고 다음 프레임의 미리보기에 반영된다. 포커스한 오프셋 행은 숫자 0 키로 0 으로 되돌린다(슬라이더의 가운데가 0 이다). 스킨이 선언한 오프셋이 먼저, 엔진이 모든 플레이 스킨에 붙이는 오프셋(All offset(%), Notes offset, Judge offset, Judge Detail offset)이 그 뒤에 나오고, 엔진 오프셋 행은 설명이 `An offset the game adds to every play skin …` 으로 시작한다 |
| 문서의 불투명도 행 | 알파 축만 허용한 오프셋(`a = true` 하나뿐)은 축 이름 없이 스킨이 준 이름 그대로 한 행이 되고 값을 퍼센트로 읽는다. 100% 가 스킨 그대로(저장값 a = 0), 0% 가 숨김(a = -255)이고 그 사이는 `round((퍼센트 - 100) * 255 / 100)` 이다. 슬라이더와 ← → 는 0~100%, 한 칸 1%, Shift 로 10%, 0 키로 100%. 직접 입력(퍼센트 수를 친다)은 200%(a = +255)까지 받는다. 0~200 의 모든 퍼센트가 서로 다른 알파로 저장되고 그 퍼센트로 다시 읽힌다. 이름이 아니라 축 구성으로 판정한다. 불투명도 행은 스킨이 선언한 위치와 관계없이 다른 오프셋 행 뒤(RELOAD 앞)에 모여 나온다. 아래 주의 참고 |
| RELOAD | 폴더를 다시 훑고 문서를 다시 읽는다 |
| RESET | 이 문서에서 한 모든 선택을 버린다(팩이 조절값 파일을 갖고 있으면 그 값으로 돌아간다. §2.8) |
| EXPORT | 조절한 팩을 새 폴더로 내보낸다(§2.8). 팩이 있을 때만 보인다 |

행 순서는 고정 행(PACK FOLDER, SCREEN, SKIN, LOADED), 옵션, 파일, 스킨이 선언한 위치·크기 오프셋, 엔진 오프셋, 불투명도, RELOAD, RESET, EXPORT 다. 팩의 다른 화면도 같은 이름으로 가진 옵션·오프셋 행에서는 `A` 가 지금 값을 그 화면들에 놓는다(바닥 안내의 `A ALL SCREENS`. §2.8). 목록에 묶음 머리는 없다. PageDown 두 번이면 7키 기본 스킨의 불투명도 행에 닿는다.

불투명도 행 주의

| 항목 | 내용 |
| --- | --- |
| 범위 | 슬라이더와 ← → 는 0% ~ 100%(저장값 -255 ~ 0)다. 100% 를 넘는 값은 직접 입력으로만 들어가고 200%(저장값 +255)가 끝이다. 100% 를 넘으면 미리보기 아래 정보 패널이 더해지는 알파를 붙여 읽는다(`150% (+128)`. 목록의 행에는 `150%` 만 나온다). 그림이 이미 불투명한 부위에서는 100% 초과가 아무 효과도 없다(알파는 255 에서 잘린다). 이 범위는 알파 0 의 덮개에 양수를 더해 밝기를 올리는 스킨을 위한 것이다(ModernChic 의 밝기 다섯 행 `明るさ 0~255`. 스킨이 말하는 0~255 는 여기서 100%~200% 다: 128 을 주려면 `150` 을 친다) |
| 100% 를 넘은 행 | 슬라이더의 손잡이는 오른쪽 끝에 선다. ← → 는 그 자리에서 1% 씩(Shift 10%) 양쪽으로 움직이고 200% 에서 멈춘다. 손잡이를 잡아 끌면 0~100% 안으로 들어온다. 손잡이를 누르기만 하거나 지금 값과 같은 수를 확정하면 저장값은 그대로다. 100% 이하로 내려온 뒤에는 ← → 가 다시 100% 에서 멈춘다 |
| 걸음 | 1%(Shift 10%). 저장 알파는 `round((퍼센트 - 100) * 255 / 100)` 이고 1% 가 알파로 약 2.55 다. 퍼센트 사이의 저장값은 가까운 퍼센트로 읽히고(a = -100 은 `61%`), 한 칸 움직이면 거기서 1% 간 값이 된다. 읽히는 퍼센트와 같은 값으로 "설정"하는 조작(손잡이 누름, 같은 수 입력, 이미 100% 인 행의 0 키)은 퍼센트 사이의 저장값을 고쳐 쓰지 않는다 |
| 중간값 | 더하는 방식이라 0% 와 100% 만 모든 destination 에 똑같이 듣는다. 스킨이 한 부위를 여러 겹으로 겹쳐 그렸거나 destination 알파를 255 보다 낮게 준 곳은 중간값에서 퍼센트와 눈에 보이는 진하기가 어긋난다. 스킨 제작자는 반투명을 destination 알파가 아니라 그림에 구워 두면 어긋나지 않는다(기본 스킨이 그렇게 한다) |
| 클릭 | 0% 로 사라진 객체도 클릭은 받는다(beatoraja 의 규칙. 투명 버튼을 쓰는 스킨이 있다) |

스킨 제작자를 위한 규칙: 헤더의 `offset` 항목에 `a = true` 만 두면(`x`, `y`, `w`, `h`, `r` 없음) 두 메뉴가 그 오프셋을 퍼센트 행으로 보인다. 그 오프셋을 부위의 모든 destination 에 `offset`(또는 `offsets`)으로 걸면 사용자가 그 부위의 불투명도를 0~100% 로 조절한다(직접 입력으로는 200% 까지: 알파 0 으로 그린 덮개를 밝히는 용도). 다른 축을 함께 허용한 오프셋의 ALPHA 행은 더해지는 원시 값(-255 ~ +255)으로 남는다.

SKIN 메뉴와 PLAY 메뉴

| 항목 | SKIN 메뉴(F12) | PLAY 메뉴(F8, `docs/play-options.md` 7절) |
| --- | --- | --- |
| 왼쪽 행 | 팩 폴더, 화면별 문서, 그 문서의 옵션·파일·오프셋·불투명도 | 플레이 옵션(속도, 가림막, 판정 오프셋, 차트)과 미리보는 플레이 문서의 옵션·오프셋·불투명도 |
| 오른쪽 미리보기 | SCREEN 행이 가리키는 화면(플레이, 선곡, 결정, 결과 등). 예시 차트는 150 → 180 BPM 이고 끝나면 장면을 다시 시작한다 | 모드 띠가 고른 플레이 화면. 예시 차트는 PREVIEW BPM 의 한 템포이고 끝나지 않고 되풀이된다 |
| 플레이 문서의 옵션(기본 스킨: Lane Width, Field Side, Scratch Side, Judge Detail, Score Difference, Judge Display, Score Graph) | 옵션 행 | LANE 묶음의 끝에 같은 행. 두 메뉴가 같은 값을 읽고 쓴다(그 문서의 `skin.custom`). 어느 쪽에서 바꿔도 다른 쪽에 그대로 보인다 |
| 스킨이 선언한 위치·크기 오프셋(기본 스킨: JUDGE POSITION Y) | 오프셋 행 | LANE 묶음에서 옵션 바로 뒤에 같은 행. 접지 않고 항상 보인다 |
| 불투명도 행(기본 스킨: 7키 18개) | 목록 끝에 모인 행 | 목록 끝 SKIN 묶음의 접이 `OPACITY`. 닫힌 채로 열리고 Enter·← →·클릭으로 편다 |
| 엔진 오프셋(All offset(%), Notes offset, Judge offset, Judge Detail offset. 15행) | 오프셋 행 | SKIN 묶음의 접이 `POSITION AND SIZE` |
| 파일 행, 문서 고르기 | 있다 | 없다 |
| 고른 화면·모드 | `skin.screen` 에 저장된다 | 저장되지 않는다. `skin.screen` 을 읽지도 바꾸지도 않는다 |
| 화면 부품 | `apps/rbms-player/src/stage/preview_menu.rs` 의 같은 코드(배치, 탭 띠, 행 목록과 스크롤, 정보 패널, 미리보기 자리의 상태 문구, 클릭·휠 규칙) | 같다 |

미리보기

| 항목 | 내용 |
| --- | --- |
| 그리는 것 | SCREEN 행이 가리키는 화면의 문서를 실제 스킨 렌더러로 16:9 사각형 안에 그린다(그림 파일이나 축소 스크린샷이 아니다) |
| 데이터 | 지어낸 예시 데이터다. 선곡은 예시 곡 목록, 플레이는 그 키 수의 예시 차트 자동 연주(반복), 결과·코스 결과는 그 연주의 기록. 사용자의 라이브러리·점수·계정 이름을 읽지 않는다. HI-SPEED, 레인 커버 같은 플레이 설정은 실제 값을 쓴다 |
| 소리 | 내지 않는다. 스킨이 부르는 효과음과 이벤트, 값 쓰기도 실행하지 않는다 |
| 반영 | 오프셋은 다음 프레임(화면 전체 오프셋 포함. 문서를 다시 읽지 않는다). 옵션·파일·RELOAD·RESET 은 문서를 다시 읽고 장면은 멈췄던 자리에서 이어진다. 문서나 팩을 바꾸면 장면이 처음부터 시작한다 |
| 읽는 동안 | 처음 읽을 때는 `LOADING THE SKIN`. 방금까지 그려지던 문서를 다시 읽는 몇 프레임(옵션·파일 변경, 장면이 끝나 다시 시작할 때)은 글 없이 검게 둔다. 0.5초를 넘기면 `LOADING THE SKIN` 을 보인다 |
| 문서가 없는 화면 | `NO SKIN DOCUMENT FOR THIS SCREEN` / `THE BUILT-IN SCREEN DRAWS IT`(내장 화면이 그리는 화면. 고른 팩에 9키 문서가 없을 때, 기본 팩의 키 설정 등) |
| 팩 폴더가 없음 | `NO SKIN DOCUMENT FOR THIS SCREEN` / `THE PACK FOLDER IS NOT THERE`. PACK FOLDER 행에도 `(MISSING)` 이 붙는다 |
| 팩의 Lua 스킨이 헤더부터 읽히지 않음 | 그 화면에 문서가 없으면 `NO SKIN DOCUMENT FOR THIS SCREEN` 아래에 `<파일> DID NOT READ (header pass): <이유>`(팩 폴더 경로는 뺀다. 여럿이면 파일 이름순 첫 파일과 `(AND N MORE)`). 헤더가 읽히지 않으면 어느 화면의 문서인지 알 수 없어, 문서가 없는 모든 화면에 같은 문구가 나온다. 파일을 고친 뒤 RELOAD 하면 사라진다 |
| 스킨으로 그리지 않는 화면 | `THIS SCREEN IS NOT DRAWN BY A SKIN`(24키, 배틀, type 9 등) |
| 미리보기가 없는 화면 | `NO PREVIEW FOR THIS SCREEN`(키 설정 문서. 왼쪽 설정은 그대로 적용된다) |
| 읽기 실패 | `THE SKIN DID NOT LOAD` 와 이유(세 줄까지). 이유는 실패한 파일 이름으로 시작하고 팩 폴더 경로를 뺀 한 문장이다(예: `decide.luaskin (body pass): decide.luaskin:3: …`). 경로 전문은 LOADED 값에 있다. 알림은 `skin <파일> did not load: <이유>` 로 한 번 뜬다. 실패한 Lua 가 앱을 멈추지 않고, RELOAD 나 값 변경 전에는 다시 읽지 않는다 |
| 자원 | 화면을 바꾸면 이전 화면의 텍스처·동영상·Lua 상태를 놓고, 메뉴를 떠나면 전부 놓는다. 화면당 텍스처 예산은 실제 화면과 같다 |
| 메뉴 아래의 화면 | 메뉴를 연 화면(선곡)의 컴파일된 스킨과 Lua 상태는 그대로 둔다. Esc 로 돌아간 첫 프레임부터 그 화면이 그려진다. 그 화면 자신을 미리본 경우(선곡에서 열어 SELECT 를 미리봄)만 돌아갈 때 스킨을 다시 읽는다(예시 상태로 읽은 Lua 를 실제 화면에 쓰지 않기 위해서다) |

선택은 `settings.ron` 의 `skin.custom` 에 문서 경로를 키로, 안쪽은 옵션 이름, 파일 슬롯 이름, 오프셋 id 를 키로 저장된다. 문서가 새 판에서 행을 더하거나 빼도 남아 있는 행의 선택은 유지된다.

### 2.3 문서가 없는 화면, 로드 실패

| 상황 | 동작 |
| --- | --- |
| 효력 있는 팩에 그 화면 문서가 없음 | 내장 화면. 알림 없음 |
| 팩 폴더가 없거나 읽히지 않음 | 문서가 없는 것과 같다. SKIN 메뉴는 PACK FOLDER 행과 미리보기 자리에 폴더가 없다고 알린다 |
| `.luaskin` 이 헤더부터 읽히지 않음 | 파일 판마다 한 번 알리고 건너뛴다. SKIN 메뉴는 문서가 없는 화면의 미리보기 자리와 LOADED 값에 그 파일과 이유를 계속 보인다 |
| `.json`, `.json5` 가 헤더부터 읽히지 않음 | 알림 없이 건너뛴다(스킨이 아닌 파일일 수 있어 구분할 수 없다) |
| 문서를 골랐으나 본문이 실패함(문법 오류, 본체 오류, 예산 초과, 파일 없음) | 내장 화면. 알림 한 번: `skin <파일> did not load, drawing the built-in screen: <이유>`(토스트가 88자에서 잘려도 어떤 파일인지 남도록 앞에 둔다. 전체 이유는 터미널에 나간다). SKIN 메뉴의 미리보기가 읽다 실패한 경우는 대신 그려지는 화면이 없으므로 `skin <파일> did not load: <이유>` 다 |
| 24키, type 9 | 내장 화면 |
| 문서를 10초 넘게 기다림 | 포기하고 내장 화면 |

SETTINGS(선곡에서 Tab)에는 스킨 설정이 없다. 스킨이 화면을 그리는 구성에서 SETTINGS 의 일부 행은 내장 화면에만 효과가 있다. 그 행의 라벨 옆에 작은 글씨로 표지가 붙고 값이 흐려진다(조정은 그대로 가능). 문서가 읽기에 실패한 화면은 표지를 붙이지 않는다.

| 표지 | 행 |
| --- | --- |
| `BUILT-IN SCREENS ONLY` | DISPLAY 의 SKIN 프리셋, RESULT GRAPHS, WHITE NUMBER, JUDGE TEXT Y, 5KEYS LAYOUT |
| `SKIN OPTION COMES FIRST` | INPUT 의 SCRATCH SIDE(스킨의 스크래치 방향 옵션이 우선) |

SCORE GRAPH 는 스킨 선곡 위의 기록 모달이 읽으므로 표지 없이 도움말만 고쳤다.

### 2.4 `--skin` 과 내장 플레이 필드

`--skin file.ron` 으로 시작한 실행은 플레이 화면을 내장 필드로 그린다. 기본 스킨이 설치돼 있어도, 사용자가 고른 팩이나 손으로 고른 플레이 문서가 있어도 제치고 그린다(선곡, 결정, 결과는 스킨 그대로). 이때 SKIN 메뉴의 플레이 화면 SKIN 행은 `DEFAULT (--skin)` 을 보이고, SETTINGS 의 DISPLAY 묶음의 SKIN 행을 바꾸면 문서로 돌아간다. 내장 화면으로 가는 별도 설정 행은 없다.

### 2.5 쓰기 오버레이

스킨의 파일 쓰기는 팩 폴더에 닿지 않고 오버레이로 간다.

| 항목 | 값 |
| --- | --- |
| 위치 | `<설정 폴더>/skin-data/<팩 식별자>/`. 식별자는 팩 폴더 이름(영숫자, `-`, `_` 이외의 글자는 `_` 로 바꾸고 32자까지)에 폴더 경로의 SHA-256 앞 8바이트(16진수)를 `-` 로 붙인 것이다. 같은 폴더를 다른 철자로 불러도 같은 팩이다. 스킨이 처음 쓸 때 만든다 |
| 읽기 | 오버레이의 파일이 팩의 같은 이름 파일보다 먼저 보인다 |
| 한도 | 오버레이 전체 64 MiB, 항목 4,096개, 동시에 열린 파일 64개, 한 번의 읽기 64 MiB |
| 한도 초과 | 쓰기가 실패한다(`the skin's write overlay is full` 또는 `holds too many files`) |

`<설정 폴더>` 는 `~/.config/rbms/` 이다(Windows 는 `%USERPROFILE%\.config\rbms\`).

### 2.6 동영상

| 항목 | 내용 |
| --- | --- |
| 형식 | mp4/m4v 안의 H.264 만 재생한다(8비트 4:2:0 프로그레시브. High 프로파일, CABAC, B 프레임 포함). 인터레이스는 지원하지 않는다 |
| 다른 확장자 | wmv, webm, mpg, mpeg, m1v, m2v, avi 는 동영상으로 분류되지만 디코더가 없어 경고 한 번 뒤 그리지 않는다 |
| 재생 | 장면 시계가 재생 시각이다. 반복 재생한다. 시계가 멈추면 영상도 멈춘다 |
| 빌드 | cargo 피처 `video`(rbms-player 에서 기본 켬, 릴리스에도 포함). 끄면 동영상 자리는 비어 있다 |
| 라이선스 | 디코더는 OpenH264(BSD-2-Clause)를 소스에서 빌드해 정적으로 링크한다. 고지는 저장소 루트 `THIRD-PARTY-NOTICES.md` |
| 한계 | 곡(BMS) 자체의 BGA 동영상은 이 디코더로 그리지 않는다(§6) |

### 2.7 폰트

| 형식 | 내용 |
| --- | --- |
| TTF | 문서의 `font` 항목이 가리키는 파일. 경로는 스킨 루트 안에서만 해석된다. 글자 높이 = 문서의 크기(em), 정렬 기준점, overflow 0/1/2, 줄바꿈, 그림자를 지원한다 |
| `.fnt` | BMFont 텍스트 형식의 비트맵 폰트(type 0)와 distance field(type 1, 2). 페이지 이미지는 `.fnt` 옆에서만 찾는다 |
| 선언하지 않은 폰트 id | 앱 기본 글꼴로 그려지지만 축소, 줄바꿈, 그림자가 적용되지 않는 대체 경로다. 폰트는 반드시 선언한다 |
| 폰트에 없는 글자 | 앱의 다국어 폰트로 대신 그린다(E5) |

### 2.8 조절한 스킨 내보내기, 가져오기, 모든 화면에 적용

SKIN 메뉴에서 조절한 팩을 폴더 하나로 내보내 다른 컴퓨터에 가져가거나 남에게 줄 수 있다. beatoraja 에는 같은 기능이 없다(원본은 조절값을 플레이어 설정 `config_player.json` 의 `skin[]` 에만 둔다). R-BMS 고유 기능이다.

#### 내보내기

| 항목 | 내용 |
| --- | --- |
| 여는 법 | SKIN 메뉴 목록 맨 아래의 `EXPORT` 행(RESET 아래)에서 Enter 또는 →. 팩이 있을 때만 행이 보인다(기본 팩 포함. 팩 폴더가 디스크에 없으면 보이지 않는다) |
| 위치 | 폴더 선택 창(제목 `Select folder to export the skin into`)에서 고른 폴더 아래에 `<팩 폴더 이름>-export` 를 새로 만든다. 그 이름이 있으면 `-export-2`, `-export-3` … (999 까지). 이미 있는 폴더에는 쓰지 않고 지우지도 않는다 |
| 들어가는 것 | 지금 효력 있는 팩의 파일 전부(문서, Lua, 이미지, 폰트, 사운드)의 사본과 조절값 파일 `rbms-adjustments.ron` 하나 |
| 빠지는 것 | 이름이 `.` 으로 시작하는 파일·폴더(`.DS_Store` 등), 팩 루트의 `install-manifest.txt`(기본 팩의 설치 기록), 팩이 가지고 있던 `rbms-adjustments.ron`(새 것으로 바뀐다), 손으로 고른 팩 밖 문서의 조절값 |
| 링크 | 팩 안의 심볼릭 링크는 팩 밖으로 따라가지 않는다. 팩 안의 파일을 가리키는 링크는 그 파일의 사본으로 쓰고, 팩 밖·폴더·끊어진 링크는 건너뛰며 개수를 알린다 |
| 진행과 취소 | 복사는 프레임 루프 밖에서 돈다. 미리보기 아래에 `EXPORTING 812 / 2431 FILES - 212 / 640 MB - ESC CANCELS` 가 뜨고 메뉴는 계속 조작된다. 내보내는 동안의 Esc 는 메뉴를 떠나지 않고 내보내기를 취소한다. 메뉴를 떠나면 취소된다 |
| 끝 | 미리보기 아래에 `EXPORTED <n> FILES (<m> MB) TO <경로>` 와 알림 한 줄. 다음 키나 클릭에 지워진다 |
| 완성 표지 | 쓰는 동안 폴더 이름은 `<이름>-export.incomplete` 이고 모든 파일이 들어간 뒤에 `.incomplete` 를 뗀다. 실패하거나 취소하면 쓰던 폴더를 지운다. 지우지 못해 남았다면 이름의 `.incomplete` 로 완성본이 아님을 알 수 있다 |
| 거절 | 고른 폴더가 팩 폴더 자신이거나 그 안이면 거절한다(`EXPORT FAILED`). 팩이 4 GiB 또는 50,000 파일을 넘어도, 폴더 깊이가 32 를 넘어도 쓰기 전에 거절하고 이유를 알린다 |
| 원본 | 내보내기는 팩 폴더를 읽기만 한다 |

조절값 파일은 설정 파일과 같은 RON 텍스트이고 사람이 읽고 고칠 수 있다. 문서는 절대 경로가 아니라 팩 안의 파일 이름으로, 옵션은 op 번호가 아니라 이름과 항목 이름으로, 파일 슬롯은 팩 안의 상대 경로로, 오프셋은 이름과 축별 값으로 적는다. 그래서 폴더를 어디로 옮겨도, 스킨 제작자가 op 번호를 바꿔도 읽힌다.

```ron
(
    version: 1,
    documents: {
        "play7.luaskin": (
            options: { "Lane Width": "Wide", "Theme": "Mono" },
            files: { "Background": File("bg/night.png"), "Bomb": Random },
            offsets: { "Judge Position": (x: 0.0, y: 12.0, w: 0.0, h: 0.0, r: 0.0, a: 0.0) },
        ),
    },
)
```

불투명도 행은 알파만 움직이는 오프셋이므로 `offsets` 의 `a` 로 적힌다(§2.2 의 환산). 아무것도 조절하지 않은 문서는 적지 않는다.

#### 가져오기

따로 누르는 것이 없다. 팩 폴더(PACK FOLDER 행, `RBMS_SKIN_PACK`, 기본 팩 자리)에 `rbms-adjustments.ron` 이 있으면 팩을 읽을 때 함께 읽어 그 팩의 기본값으로 쓴다. 스킨의 Lua 는 이 파일을 읽지 않는다.

| 항목 | 내용 |
| --- | --- |
| 우선순위 | 행마다 사용자가 저장한 값 > 팩의 조절값 > 스킨 제작자의 기본값. 사용자가 저장하지 않은 행만 팩의 값을 따른다 |
| 저장 | 팩의 값은 `settings.ron` 에 쓰이지 않는다. 메뉴에서 바꾼 행만 지금까지처럼 `skin.custom` 에 저장된다(설정 파일 형식 불변) |
| RESET, 0 키 | RESET 은 사용자가 저장한 값을 지워 팩의 조절값으로 돌아간다(조절값 파일이 없는 팩은 지금처럼 제작자의 기본값). 오프셋·불투명도 행의 0 키도 팩의 값으로 간다 |
| 모르는 이름 | 문서, 옵션, 항목, 파일 슬롯, 파일, 오프셋 이름이 그 팩에 없으면 그 줄만 무시한다. 오프셋 값은 행의 범위(위치·크기 ±2000, 각도 ±360, 알파 ±255)로 묶는다 |
| 쓸 수 없는 파일 | RON 으로 읽히지 않거나, `version` 이 이 빌드가 아는 것(1)보다 높거나, 512 KiB 또는 16,384 줄을 넘거나, 일반 파일이 아니면(링크 포함) 통째로 무시하고 알림을 한 번 낸다. 팩은 제작자의 기본값으로 그려진다 |
| 적용 범위 | 옵션과 파일은 문서를 읽을 때, 오프셋과 불투명도는 프레임마다 반영된다. 실제 화면, SKIN·PLAY 메뉴의 미리보기, 메뉴의 행이 모두 같은 값(`SkinLibrary::customisation_for`)을 읽는다. 내보낸 팩을 새 설정에서 열면 조절한 화면과 픽셀까지 같은 프레임이 나온다(`stage/capture/play_parts.rs` 의 `a_run_that_names_an_exported_default_pack_draws_the_offsets_it_was_exported_with`) |

#### beatoraja 에서 열기

내보낸 폴더는 beatoraja 에서도 스킨으로 열린다. 스킨 자체(문서와 자원)는 원본 그대로이고, beatoraja 의 스킨 목록은 `.lr2skin`·`.luaskin`·`.json` 만 문서로 읽으므로(`launcher/SkinConfigurationView.java:230-232`) `rbms-adjustments.ron` 은 무시된다. 조절값은 beatoraja 로 넘어가지 않는다: 그쪽에서는 제작자의 기본값으로 열리고 스킨 설정 화면에서 다시 고른다. R-BMS 도 같은 이유로 `.luaskin`·`.json`·`.json5` 만 문서로 읽는다. 팩 루트에 다른 `.json` 을 두면 양쪽 모두 문서로 오인할 수 있으니 조절값 파일의 확장자를 바꾸지 않는다.

#### 제3자 스킨을 내보낼 때

내보내기는 스킨 파일을 그대로 복사한다. 남이 만든 스킨(예: ModernChic)의 내보낸 폴더를 남에게 주는 것은 그 스킨을 재배포하는 것이고, 조건은 원작자가 정한 것을 따른다(readme·배포 페이지의 재배포·개조 조항). R-BMS 는 이를 확인하지 않는다. 재배포가 허용되지 않는 스킨이라면 `rbms-adjustments.ron` 한 파일만 건네고 받는 쪽이 자기 팩 폴더에 넣으면 같은 결과가 된다. R-BMS 의 기본 스킨 `rbms-default` 는 저장소와 같은 GPL-3.0-or-later 다.

#### 같은 옵션을 모든 화면에 적용

팩의 문서들은 화면마다 따로 조절값을 가진다. 여러 문서가 같은 이름의 옵션이나 오프셋을 선언하고 있으면(기본 스킨의 Theme·Accent, 플레이 5종의 Lane Width 등) 한 번에 맞출 수 있다.

| 항목 | 내용 |
| --- | --- |
| 안내 | SKIN 메뉴와 PLAY 메뉴에서 그런 행에 커서를 두면 미리보기 아래에 `ALSO ON 4 OTHER SCREENS - A APPLIES THIS VALUE TO THEM ALL` 이 뜬다 |
| 키 | `A`. 확인 없이 지금 값을 그 화면들에 저장하고 설정 파일을 쓴다. 알림 한 줄: `THEME: MONO applied to 8 other screens` |
| 옵션 | 이름이 같은 옵션에, 이름이 같은 항목으로 맞춘다(op 번호는 문서마다 달라도 된다). 그 이름의 항목이 없는 화면은 건너뛰고 알림에 개수를 적는다(`(1 skipped: no item of that name)`) |
| 오프셋 | 이름이 같고 그 행의 축을 허용하는 오프셋에, 그 축의 값만 맞춘다. 불투명도 행은 알파 |
| 하지 않는 것 | 자동으로 묶지 않는다(화면마다 다르게 두고 싶은 옵션이 있다). 파일 슬롯은 대상이 아니다(후보 파일이 문서마다 다르다). 숫자를 치는 중에는 `A` 가 듣지 않는다 |

## 3. 기본 스킨 `rbms-default`

### 3.1 위치

| 곳 | 경로 |
| --- | --- |
| 저장소 | `assets/skins/rbms-default/` |
| 설치 폴더 | `<설정 폴더>/skin/rbms-default/`. `skin.folder` 설정과 무관하게 항상 이 위치다 |
| 바이너리 | 빌드 스크립트 `apps/rbms-player/build.rs` 가 저장소 폴더를 걸어 만든 표를 싣는다. 65개 파일, 1,772,662 B(7095ef7 실측, 폴더 안 파일 크기의 합. 웨이브 14 의 1,771,429 B 에서 웨이브 16 의 Lua 변경만큼 늘었다). 폰트 `font/Inter-Regular.ttf` 는 렌더러의 `BUNDLED_FONT` 를 가리켜 한 벌만 실린다(두 파일이 달라지면 빌드 실패). 릴리스 바이너리 크기는 +317,760 B(macOS arm64 실측) |

### 3.2 구조

| 경로 | 내용 |
| --- | --- |
| `decide.luaskin`, `select.luaskin`, `play5`, `play7`, `play9`, `play10`, `play14` `.luaskin`, `result.luaskin`, `courseresult.luaskin` | 화면 문서 9개. 한 줄짜리 진입 파일이다(`return require("lua.<화면>.main").build(...)`) |
| `lua/` | 공통 모듈 `theme.lua`(색, 격자, 이름표), `props.lua`(속성 id 이름표), `ui.lua`(조립기), `motion.lua`(키프레임 프리셋), `parts.lua`(조합 부품), 화면 모듈 `lua/<화면>/`, 생성된 시트 좌표표 `lua/atlas/` |
| `image/` | 생성된 시트 5장 `common`, `decide`, `select`, `play`, `result` `.png`. 합계 120,733 B |
| `font/` | `Inter-Regular.ttf`, `Inter-OFL.txt` |
| `sound/` | 시스템 사운드 22개 `.wav`. 스킨의 `sound/…` 와 시스템 사운드 폴더가 같은 설치본이다 |
| `tools/` | 이미지와 사운드 생성 스크립트. 설치본에는 들어가지 않는다 |

설치 폴더에는 `install-manifest.txt` 가 추가로 생긴다.

### 3.3 설치와 갱신 규칙

앱은 실행할 때마다 설치 폴더를 이번 빌드의 표와 맞춘다(`install_default_pack`, `apps/rbms-player/src/assets/default_pack.rs`).

| 파일의 상태 | 처리 |
| --- | --- |
| 없음 | 쓴다(지운 파일은 다시 생긴다) |
| 이전 빌드가 설치했고 손대지 않았으며 이번 빌드가 다른 판을 싣는다 | 새 판으로 바꾼다 |
| 이전 빌드가 설치했고 손대지 않았으며 이번 빌드가 더는 싣지 않는다 | 지운다. 빈 폴더는 남긴다 |
| 사용자가 고쳤거나 직접 둔 파일 | 그대로 둔다 |
| 팩 안의 폴더가 다른 곳으로 가는 링크이거나 폴더가 아님 | 그 안으로 쓰지도 지우지도 않는다(팩 폴더 자체가 링크인 것은 따라간다) |
| 쓰다 중단된 `<이름>.<번호>.rbms-partial` | 쓰기, 삭제, 실패가 있었던 실행 끝에 치운다 |

구별 기준은 `install-manifest.txt`(설치한 파일마다 SHA-256)다. 기록이 이번 빌드와 같으면 설치본을 읽지 않는다(기록 1회 읽기와 파일마다 stat). 디버그 빌드 실측은 첫 설치 10~27 ms, 변화 없을 때 0.3~0.46 ms. 사용자가 고친 파일은 실행마다 그 파일 하나를 다시 해시하되 기록은 옛 지문으로 두어, 원래대로 되돌리면 다시 갱신 대상이 된다. 설정 폴더에 쓸 수 없으면 `default skin not fully installed: N files could not be written beside …` 경고를 한 번 내고 내장 화면으로 뜬다.

### 3.4 사용자 옵션

SKIN 메뉴의 옵션 행으로 고른다. 플레이 문서의 옵션은 PLAY 메뉴(선곡에서 F8)의 LANE 묶음에도 같은 행으로 나오고, 어느 쪽에서 바꿔도 같은 곳(그 문서의 `skin.custom`)에 저장된다(`docs/play-options.md` 7절). 기본값은 먼저 적힌 쪽이다. 파일 행(filepath)은 없다(자산 변종을 두지 않았다). 플레이 문서의 오프셋과 부위별 불투명도는 §3.4.1, §3.4.2 에 있다.

| 문서 | 옵션 |
| --- | --- |
| decide | Stage Image: Show / Hide. Note Graph: Show / Hide. Theme. Accent |
| select | Stage Image: Show / Hide. Theme. Accent |
| play5, play7 | Lane Width: Normal / Wide. Field Side: Left / Right. Scratch Side: Left / Right. Judge Detail: Fast-Slow / Milliseconds / Off. Score Difference: Target / Best / Off. Judge Display: Show / Hide. Score Graph: On / Off. Note Colors. Bomb Color. Theme. Accent |
| play9 | Lane Width. Judge Detail. Score Difference. Judge Display. Score Graph. Bomb Color. Theme. Accent |
| play10, play14 | Lane Width. Judge Detail. Score Difference. Judge Display. Note Colors. Bomb Color. Theme. Accent |
| result, courseresult | Theme. Accent |

색 옵션 넷(Theme, Accent, Note Colors, Bomb Color)은 §3.4.3 에 있다. 이 넷은 그것을 가진 모든 문서에서 이름과 항목 이름이 같다.

Lane Width 는 다섯 플레이 문서의 첫 행이다. Wide 는 건반·스크래치·버튼 레인을 Normal 의 1.5 배로 넓힌다(1920 기준 흰 건반 58 → 87, 검은 건반 46 → 69, 스크래치 100 → 150, 9키 버튼 62 → 93. 레인 사이 2px 과 노트 두께는 그대로). 노트, 판정선, 레인 커버, 리프트, HIDDEN+, 판정 글자, 게이지, 발치의 HI-SPEED·GREEN 은 넓어진 필드를 따라간다. 필드 밖은 다시 놓인다.

| 문서 | Wide 에서 달라지는 배치 |
| --- | --- |
| play7, play5 | Score Graph 가 On 이면 BGA 높이가 626 → 450 으로 줄고 점수·판정 수 아래에 타이밍이 한 줄 전체로 놓인다. Off 이면 Normal 과 같은 1단이다 |
| play9 | BGA 가 440x440 으로 줄고 타이밍이 왼쪽 BGA 아래, 판정 수와 점수가 오른쪽에 놓인다 |
| play10, play14 | 두 필드가 화면 가장자리 쪽으로 넓어지고(바깥 여백 64 → 32) 가운데 폭 352 에 정보가 한 단으로 쌓이며, 게이지가 1P 필드 아래로 내려간다 |

Normal 은 이 행이 생기기 전의 화면과 같다. 내장 화면의 DISPLAY > SKIN 프리셋(NORMAL / WIDE)은 다른 설정이고 스킨이 그리는 화면에는 효과가 없다. 폭 480 미만의 곡 머리(Wide 의 9·10·14키)에서는 코스 단계 글자가 난이도 줄 오른쪽에 뜨고 코스 중 아티스트가 생략된다.

#### 3.4.1 판정 표시

필드는 판정선에서 끝난다(웨이브 13). 1920x1080 기준으로 노트가 내려오는 영역이 y 0~904, 판정선이 904~912 이고, 판정선의 아래 변이 레인 면·레인 구분선·진행 막대의 아래 끝이다. 그 아래에는 레인이 없고 16px 뒤(928)에 게이지가 온다. 5·7·9·10·14키와 Lane Width Normal·Wide 가 같다. LIFT 를 켜면 판정선이 리프트만큼 올라가고 그 아래는 리프트 면이 필드 끝(912)까지 덮는다. 폭발은 판정선 가운데(908)에 중심을 둔 온전한 고리(128x128)다(웨이브 14). 아래 절반은 판정선 아래로 번져 게이지와 발치 위에 겹쳐 그려진다(짧게 뜨는 가산 합성 효과라 밑의 것이 비친다). 필드의 부위 가운데 판정선 아래로 내려가는 것은 폭발뿐이다. 놓친 노트는 판정선 아래로 내려가지 않고 판정선에서 사라진다(9키도 같다). 좌표의 전부는 설계서 6.3·6.4절.

판정 글자와 콤보는 움직이지 않는다. 판정이 난 프레임부터 0.5초 동안 같은 자리·크기·색으로 서 있다가 한 번에 사라진다(튀어나오기, 확대, 깜빡임, 색 돌림, 페이드가 없다). 다음 판정이 그 전에 나면 그 자리에서 바뀐다.

| 요소 | 내용 |
| --- | --- |
| 판정 글자 | PGREAT 와 GREAT 는 둘 다 `GREAT` 로 쓰고 색으로 가른다(PGREAT 하늘색 `84CEFF`, GREAT 노랑 `FFD633`). GOOD·BAD·POOR 는 주황에서 붉은 주황으로 가는 각자의 색이다. 글자의 왼쪽 끝은 판정 종류와 콤보 자릿수에 상관없이 같은 자리다. 그 자리는 `GREAT` 와 세 자리 콤보의 묶음이 필드 가운데에 오는 곳이다 |
| 콤보 | PGREAT·GREAT·GOOD 의 글자 바로 뒤에 붙는다: `GREAT 7`, `GREAT 67`, `GREAT 648`, `GREAT 1234`. 첫 자리는 제자리에 있고 자릿수가 늘면 오른쪽으로만 자란다. 앞자리 0 은 없다. 다섯 자리까지 필드 안에 들도록 좁은 필드에서는 글자가 조금 작다(크기 64 기준으로 7·14키 기본 폭은 60, 5·10키 기본 폭은 46, 넓은 레인과 9키는 64). 콤보가 끊기는 BAD·POOR 는 숫자 없이 글자만 같은 왼쪽 끝에서 뜬다 |
| 위 줄 왼쪽: 점수 차 | Score Difference 가 Target 이면 목표(TARGET 설정, 스코어 그래프의 TARGET 과 같은 값) 대비 EX 점수 차, Best 이면 자기 최고 기록 대비 차를 부호와 네 자리로 쓴다(`+0003`, `-0125`. ±9999 에서 멈춘다). PGREAT 에도 뜬다. Off 면 없다 |
| 위 줄 오른쪽: 타이밍 | Judge Detail 이 Fast-Slow 이면 PGREAT 가 아닌 판정에서 FAST(파랑) 또는 SLOW(빨강), Milliseconds 이면 어긋난 시간을 ms 로 쓴다(빠르면 `+`, PGREAT 는 회색). Off 면 없다. 위 줄의 왼쪽 끝은 판정 글자의 왼쪽 끝과 같고, 둘 중 하나만 켜면 켠 쪽이 그 왼쪽 끝에서 시작한다 |
| 위치 | 오프셋 Judge Position(메뉴의 `JUDGE POSITION Y`). +면 판정 글자·콤보·위 줄이 함께 그 픽셀만큼 위로, -면 아래로 간다. 한 칸 1px, Shift 로 10px, 범위 ±2000, 0 키로 제자리. 좌표는 스킨의 1920x1080 기준이다 |
| 숨김 | 옵션 Judge Display 를 Hide 로 두면 판정 글자·콤보·위 줄이 모두 그려지지 않는다 |

DP 는 친 쪽 필드에만 뜬다. 엔진 오프셋 Judge offset(판정 글자와 콤보)과 Judge Detail offset(위 줄)은 따로 남아 있고 Judge Position 과 더해진다. Judge Position 을 약 -210 아래로 내리면 판정선·리프트 수치 자리에 닿는다.

스코어 그래프의 TARGET 막대 아래 범례는 고정 글자 `TARGET` 이 아니라 그 판이 실제로 쓰는 목표의 이름이다(웨이브 16. 문자열 id 3 `STRING_TARGET_NAME`, 스킨의 `props.text.TARGET_NAME`. `lua/play/info.lua` 의 `PACE`). 고른 목표에 점수가 없어 물러났으면 무엇이 없었는지가 붙는다(`RATE AAA (NO RECORD)`, `LOCAL BEST (NO IR SCORE)`). 같은 id 를 읽는 다른 팩도 같은 글자를 받는다. 물러나는 규칙은 `docs/play-options.md` 11절.

#### 3.4.2 부위별 불투명도

플레이 문서는 화면의 부위마다 불투명도 오프셋을 하나씩 선언한다(알파 축만 가진 오프셋. §2.2 의 불투명도 행). SKIN 메뉴에서는 목록 끝에, PLAY 메뉴에서는 SKIN 묶음의 접이 `OPACITY` 안에 `100%` 로 나온다. 0% 면 그 부위가 완전히 사라진다.

| 오프셋 이름(id) | 덮는 것 |
| --- | --- |
| Lane Background Opacity(101) | 레인 면 |
| Lane Divider Opacity(102) | 레인 사이 세로선 |
| Judge Line Opacity(103) | 판정선과 그 위의 리듬 띠 |
| Key Beam Opacity(104) | 키를 누를 때 레인에 서는 빛 기둥 |
| Bomb Opacity(106) | 판정 폭발 |
| Note Opacity(107) | 노트, 롱노트, 지뢰 |
| Bar Line Opacity(108) | 마디선 |
| Lane Cover Opacity(109) | 레인 커버·리프트·HIDDEN+ 의 면과 그 수치 |
| Gauge Opacity(110) | 게이지와 게이지 숫자 |
| Progress Bar Opacity(111) | 필드 옆 진행 막대 |
| Song Header Opacity(112) | 곡 머리(제목, 아티스트, 난이도) |
| BGA Opacity(113) | BGA 와 그 틀 |
| Score Graph Opacity(114) | 스코어 그래프(SP 문서만. DP 문서에는 이 행이 없다) |
| Score Opacity(115) | 점수 패널 |
| Judge Count Opacity(116) | 판정 수 패널 |
| Timing Opacity(117) | 타이밍 패널 |
| Distribution Graph Opacity(118) | 타이밍 분포 그래프 |
| Speed Display Opacity(119) | 필드 발치의 HI-SPEED·GREEN 표시, 가림막을 바꾸는 동안 뜨는 COVER·LIFT 수치, 상태 글자 |

105 는 비어 있다. 판정선 아래에 있던 건반 표시(Key Display Opacity)의 번호이고, 그 부위는 없어졌다: 판정선이 필드의 맨 아래에 붙어 있고 그 아래에는 레인이 없다. 누른 건반은 Key Beam 이 알리고, 판정선을 올리려면 PLAY 메뉴의 LIFT 를 쓴다. 설정 파일에 남아 있는 105 의 값은 읽히기만 하고 아무것도 바꾸지 않는다.

판정 글자·콤보·위 줄은 불투명도 부위가 아니다(Judge Display 로 숨긴다). LOADING·READY·FULL COMBO 알림, 폐점 덮개와 FAILED, 화면 바탕, 장면 페이드는 어느 부위에도 속하지 않아 항상 100% 다.

예: 노트와 판정만 남기기. SKIN 메뉴(F12)에서 7K 탭을 고르고 Lane Background, Lane Divider, Bar Line, Progress Bar, Score Graph, Song Header, Speed Display, Timing 행에서 각각 0% 로 둔다(슬라이더를 왼쪽 끝으로 끌거나, Enter 를 누르고 `0` 을 친 뒤 Enter, 또는 Shift+← 열 번). Note, Bomb, Judge Line, Key Beam 은 100% 그대로 둔다. 검은 바탕에 노트·키 빔·폭발·판정선·판정 글자만 남는다. 미리보기가 누를 때마다 따라 바뀌고, Esc 로 저장하면 다음 플레이부터 같은 화면이다. 한 행을 되돌리려면 그 행에서 0 키, 전부 되돌리려면 RESET 이다.

중간값: 판정선(Judge Line)은 어두운 바탕 선 위에 밝은 선을 겹쳐 그리므로 50% 에서 약 58% 로 보인다. 폭발(Bomb)은 가산 합성이라 밝은 곳에서는 50% 로 낮춰도 차이가 작다. 나머지 부위는 50% 에서 48~51% 다(1920x1080 캡처 실측).

#### 3.4.3 색

기본 스킨은 모노톤이다(웨이브 14). 화면 바탕은 순수한 검정(`000000`)이고, 면과 선은 파란 기가 없는 회색 단계, 글자는 흰색과 회색, 강조도 흰색이다. 색을 가진 것은 뜻이 색에 실린 것(판정 글자와 판정 수 표, FAST 파랑 / SLOW 빨강, 클리어 램프, 난이도, 게이지 종류, 그래프의 판정 색, 스코어 그래프의 TARGET 막대, GREEN 숫자)과 플레이의 노트·키 빔·폭발뿐이다. 색 옵션 넷으로 바꾼다. 기본값은 먼저 적힌 쪽이고, 색 값의 전부와 "색으로 두는 것"의 표는 설계서 3.1절에 있다.

| 옵션 | 항목 | 가진 문서 | 바꾸는 것 |
| --- | --- | --- | --- |
| Theme | Mono / Slate | 9개 전부 | 바탕·면·선·글자의 단계 한 벌. Mono: 바탕 `000000`, 면 `161616`·`262626`, 선 `3C3C3C`, 글자 `F2F2F2`·`B8B8B8`·`808080`. Slate: 웨이브 13 까지의 어두운 청회색(바탕 `0C0F14`, 면 `141922`·`1D2430`, 선 `2B3545`, 글자 `F1F4F8`·`A7B2C1`·`667387`) |
| Accent | White / Sky / Amber / Green / Pink | 9개 전부 | 강조색 하나(선택 표시, 진행선, 포커스, 판정선, 표 이름, 켜진 상태 글자). White `FFFFFF`, Sky `4CC9F0`, Amber `F2B33D`, Green `59D185`, Pink `F0689E` |
| Note Colors | White-Blue / White-Deep Blue / White-Yellow / White-Green / Mono | play5, play7, play10, play14 | 검은 건반 노트의 색: `4CC9F0`, `568CF5`, `F2B33D`, `59D185`, Mono 는 회색 `8C8C8C`. 흰 건반은 언제나 `F1F4F8`, 스크래치는 빨강 `F0564C`(Mono 만 밝은 회색 `C4C4C4`). LN 몸통과 키 빔이 같은 색을 따른다. 9키의 버튼 색은 모드가 정해서 이 행이 없다 |
| Bomb Color | Note / White | 플레이 5개 | 폭발의 색. Note 는 그 레인의 노트 색, White 는 모든 레인이 흰색 |

- 행은 문서가 원래 갖고 있던 옵션 뒤에 Note Colors, Bomb Color, Theme, Accent 순으로 나온다.
- 옵션은 문서(화면)마다 따로 저장된다. Theme 과 Accent 는 아홉 문서가, Note Colors 와 Bomb Color 는 그것을 가진 플레이 문서가 같은 이름과 같은 항목 이름으로 선언한다. 같은 이름의 옵션을 팩의 모든 화면에 한 번에 적용하는 기능(웨이브 14. §2.8)이 이 이름으로 화면들을 묶는다: SKIN 메뉴(F12)의 어느 화면 탭에서든 THEME 행의 값을 바꾸고 `A` 를 누르면 나머지 여덟 화면이 같은 값이 된다(`THEME: SLATE applied to 8 other screens`). ACCENT 도 같고, NOTE COLORS 는 나머지 플레이 셋, BOMB COLOR 는 나머지 플레이 넷에 적용된다.
- 웨이브 13 까지의 화면으로 돌아가려면 Theme = Slate, Accent = Sky 다(각각 값을 고르고 `A`). 노트와 폭발의 색은 기본값(White-Blue, Note)이 그때와 같다.
- 고른 색은 EXPORT 로 내보낸 팩에 함께 실린다(§2.8). 그 폴더를 새 설정에서 팩으로 열면 아홉 화면이 고른 색으로 열린다.
- 스킨 위에 앱이 덧그리는 것(SKIN·PLAY 메뉴, `H` 키 안내, 알림, 코스 탭 표지, 선곡의 옵션·검색·필터 오버레이)과 스킨이 없는 내장 화면은 스킨이 아니라 앱 테마(`crates/rbms-render/src/theme.rs`, [theme.md](./theme.md))의 색이다. 앱 테마의 기본값도 무채색이다(웨이브 15): 바탕 `000000`, 면 `161616`·`262626`, 선 `3C3C3C`, 글자 `F2F2F2`·`B8B8B8` 는 Mono 와 같은 값이고, 작은 안내 글자만 `909090`(Mono 는 `808080`)으로 한 단계 밝다. 선택된 행·탭·포커스는 색이 아니라 밝기(면 `505050`, 흰 글자, 흰 띠)로 구분된다. 색이 남는 것은 스킨과 같은 종류다: 판정 색, FAST/SLOW, 클리어 램프와 이름, 난이도·모드 배지·랭크, 점수 차, 그래프, 게이지, GREEN 숫자, 알림과 키 설정의 경고(주황·빨강·호박색). 앱 테마는 스킨의 Theme·Accent 옵션을 따르지 않는다: Slate 나 다른 강조색을 골라도 앱의 메뉴는 무채색 그대로이고, 메뉴의 색은 `theme.ron` 으로 바꾼다([theme.md](./theme.md)).
- 색 옵션은 화면이 다시 읽힐 때 적용된다(다른 옵션과 같다). SKIN 메뉴와 PLAY 메뉴의 미리보기는 고르는 즉시 따라 바뀐다.

선곡의 사이드 메뉴(톱니와 스피커 아이콘)는 QUICK SETTINGS 6행과 VOLUME 3개를 연다. 패널 1~3(플레이 옵션, 보조 옵션, 세부 옵션)은 START(기본 A), SELECT(기본 W), 둘을 함께(또는 상세 옵션 키. 기본 숫자 5)를 누르고 있는 동안 열려 있다(beatoraja 와 같다). 패널 3 의 4·6번 키 칸(NOTE DISPLAY TIME)은 KEEP GREEN NUMBER 가 켜져 있으면 유지 중인 이동 시간(ms)과 녹색 숫자를 보이고 1 ms 씩 바꾸며, 꺼져 있으면 `OFF` 와 `KEEP GREEN NUMBER IS OFF` 를 보인다. 패널 3 의 5·7번 키 칸(JUDGE TIMING)은 판정 오프셋을 1 ms 씩 내리고 올리며, 동작 줄에 그 키가 노트를 어느 쪽으로 옮기는지를 쓴다(5번 `NOTES 1 MS LATER` = -1, 7번 `NOTES 1 MS EARLIER` = +1). 플레이 화면의 TIMING 은 판정 오프셋(설정값 그대로)이고 AUTO ADJUST 가 켜져 있으면 표지가 `TIMING AUTO` 가 된다. 이 옵션들의 뜻과 단위는 [play-options.md](./play-options.md)에 있다.

바닥의 안내 문구는 바뀔 수 있는 키의 이름을 고정해 적지 않는다. 선곡 바닥은 `H`(지금 설정된 키 전체)를 가리키고, 결과 화면은 다시 하기·다음 곡의 키가 키 설정에 있음을 출하 키(R, N)와 함께 적는다.

### 3.5 이미지와 사운드를 다시 만드는 법

| 대상 | 명령 |
| --- | --- |
| 이미지 전부 | `uv run assets/skins/rbms-default/tools/generate-images.py` |
| 특정 구역만 | `uv run assets/skins/rbms-default/tools/generate-images.py select play` (구역: common, decide, select, play, result) |
| 저장소 파일과 일치 확인 | `uv run assets/skins/rbms-default/tools/generate-images.py --check` (다르면 종료 코드 1) |
| 사운드 22개 | `uv run assets/skins/rbms-default/tools/generate-sounds.py` |

이미지 스크립트는 PEP 723 인라인 메타(`requires-python >= 3.11`, `pillow==12.3.0`)로 의존성을 고정한다. 같은 입력이면 바이트가 같다. 색은 `lua/theme.lua` 에서 읽으므로 팔레트를 바꾸면 `theme.lua` 만 고치고 그 색을 굽는 구역을 다시 생성한다(테마의 단계 `theme.mono`·`theme.slate` 는 선곡의 막대와 램프 띠, 플레이의 게이지 칸에 굽고, 노트의 색 `theme.note` 는 플레이의 노트에 굽는다. 강조색은 굽지 않는다). 생성물(PNG, `lua/atlas/*.lua`)은 저장소에 커밋하며 빌드와 CI 는 uv 없이 돈다. 다른 OS 나 다른 Pillow 판에서의 바이트 동일성은 보장하지 않는다.

### 3.6 고쳐 쓰는 법

| 하고 싶은 것 | 방법 |
| --- | --- |
| 설치본의 일부만 바꾸기 | 설치 폴더의 파일을 직접 고친다. 앱 업데이트가 그 파일을 덮어쓰지 않는다 |
| 통째로 새 스킨 만들기 | 팩 폴더를 새로 만들어 PACK FOLDER 로 지정한다. 문서가 없는 화면은 내장 화면이 된다 |
| 기본 스킨 원본 고치기 | 저장소의 `assets/skins/rbms-default/` 를 고친다. 색은 `lua/theme.lua`(테마 두 벌 `theme.mono`·`theme.slate`, 강조색 `theme.accent`, 노트 `theme.note`, 의미색. 화면 모듈은 색 표를 참조로만 쥐고 `theme.apply` 가 본체 패스에서 값을 정한다: 설계서 4.1절), 좌표는 화면 모듈, 시트는 `tools/sections/<구역>.py`. 다음 빌드가 새 표를 싣는다 |
| 고친 뒤 확인 | `cargo run -p rbms-cli -- skin-dump assets/skins/rbms-default --strict --frames 120`(§5) |

원칙은 설계서 §1, §3, §4 를 따른다. 좌표는 1920x1080 설계 좌표 `(x, top, w, h)` 이고, 글자는 영어이며, 금지 API(luajava, 파일 쓰기, `os.date`, `math.random`)를 쓰지 않는다. 문서마다 헤더와 본체를 한 모듈의 `build` 가 만들고, 모듈 최상위에서는 `skin_config` 와 `main_state` 를 부르지 않는다(헤더 전용 읽기에서 `main_state` 가 빈 테이블이기 때문). 공통 모듈의 API 는 설계서 §4 에 있다. 이 팩은 자작이며 ModernChic 의 Lua, 이미지, 폰트, 사운드를 한 줄도 복사하지 않았다(D1).

## 4. Lua 환경

스킨의 Lua 는 mlua 의 `lua52` 피처(번들 빌드)로 돈다. 수는 double 하나뿐이고 정수 나눗셈 연산자가 없으며 비트 연산은 `bit32` 다.

### 4.1 쓸 수 있는 것

| 구분 | 내용 |
| --- | --- |
| 표준 라이브러리 | base, table, string, math, bit32, coroutine. 패턴 함수(find, match, gmatch, gsub)는 예산에 청구되는 자체 구현이다. `print` 는 로그로 간다. `load` 는 텍스트 청크만 받는다 |
| `require` | 점을 `/` 로 바꿔 스킨 팩 폴더 안에서만 찾고 캐시한다. 첫 패스에서 한 번 실행된 모듈은 본체 패스가 재사용한다 |
| `dofile`, `loadfile` | 절대 경로나 팩 상대 경로. 정규화해 팩 밖이면 오류. 캐시 없음 |
| `io` | 읽기는 오버레이 다음 팩, 쓰기는 오버레이(§2.5). 없는 파일은 `nil, 메시지` |
| `os` | `clock`, `date`(현지 시각), `difftime`, `time`, `setlocale`(항상 `"C"`)만 |
| `main_state` | 스킨이 게임을 보는 전부. `option`, `number`, `numbers`, `float_number`, `text`, `event_index`, `offset`, `timer`, `time`, `timer_is_on`, `timer_is_off`, `timer_elapsed(_ms/_seconds)`, `set_timer`, `event_exec`, `key_pressed`, `screen_width`, `screen_height`, `rate`·`exscore` 계열, `gauge`, `gauge_type`, `judge`, `volume_*`, `audio_play/loop/preload/stop/dispose`, `file_*` 헬퍼. id 는 숫자나 이름이다 |
| `timer_util` | `now_timer`, `is_timer_on`, `is_timer_off`, `timer_function`, `timer_observe_boolean`, `new_passive_timer` |
| `event_util` | `event_observe_turn_true`, `event_observe_timer`, `event_observe_timer_on`, `event_observe_timer_off`, `event_min_interval` |
| `luajava` facade | `bindClass` 는 `java.io.File`, `com.badlogic.gdx.Gdx`, `com.badlogic.gdx.Input`, `com.badlogic.gdx.controllers.Controllers`, `Controller` 만. `File` 은 `mkdir`, `listFiles`(오버레이 규칙). `Gdx.graphics:getWidth/getHeight`, `Gdx.input:isKeyPressed(code)`(호스트 키 상태), `Input.Keys.<이름>`(libGDX 키 코드), `Controllers.getControllers()` 는 빈 목록. `luajava.new`/`newInstance` 는 `java.io.File`, `java.net.URL`, `InputStreamReader`, `BufferedReader` 만 |
| 난수 | `math.random` 과 와일드카드 파일 선택은 벽시계 시각(나노초)으로 시드한다. 환경 변수 `RBMS_SKIN_SEED`(정수)로 고정할 수 있다(테스트, 캡처) |

### 4.2 금지이거나 무동작인 것

| 대상 | 동작 |
| --- | --- |
| 네트워크 | `main_state.http_get`, `http_get_lines` 는 항상 `nil, 메시지`. luajava `URL` 의 `connect()`, `getResponseCode()`, `getInputStream()` 는 항상 `network access is not available to skins` 로 실패한다. 소켓을 열지 않는다 |
| 팩 밖 파일 | `require`, `dofile`, `loadfile`, `io`, `File`, `audio_*` 모두 `skin file access denied` 나 `Lua skin file access denied` 로 거부 |
| 팩에 쓰기 | 오버레이로 간다. 팩 폴더는 절대 쓰지 않는다 |
| 바이트코드 | `string.dump` 제거, `load` 는 텍스트만 |
| `io.popen` | 거부 |
| `os.execute`, `exit`, `getenv`, `remove`, `rename`, `tmpname` | 존재하지 않음 |
| `debug` | 한 함수짜리 빈 껍데기 |
| `package.loadlib` | 없음. 네이티브 코드를 읽을 방법이 없다 |
| `__gc` | 종료자는 돌지 않는다 |
| 컨트롤러 | 없는 것으로 답한다 |
| `set_timer` | 값은 저장되지만 렌더러가 읽지 않는다(customTimers 미평가, §6) |

### 4.3 예산

기본값은 `crates/rbms-skin/src/lua/budget.rs`, 앱은 기본값 그대로 쓴다. 초과하면 그 로드는 실패하고(내장 화면), 프레임 초과는 그 프레임의 나머지 호출이 직전 값을 돌려준다.

| 구분 | 한도 | 상수 |
| --- | --- | --- |
| 로드 한 패스(헤더, 본체 각각) | 명령 100,000,000개, 10초 | `DEFAULT_LOAD_INSTRUCTIONS`, `DEFAULT_LOAD_MICROS` |
| 프레임 | 호출 16,384회, Lua 안에서 쓴 시간 50 ms | `DEFAULT_FRAME_CALLS`, `DEFAULT_FRAME_MICROS` |
| 호출 하나 | 명령 1,000,000개 | `DEFAULT_CALL_INSTRUCTIONS` |
| 인터프리터 메모리 | 256 MiB | `DEFAULT_MEMORY_BYTES` |
| 쓰기 오버레이 | 64 MiB, 4,096개 항목 | `DEFAULT_OVERLAY_MAX_BYTES`, `DEFAULT_OVERLAY_MAX_ENTRIES` |
| 파일 | 동시에 64개 열기, 한 번에 64 MiB 읽기 | `MAX_OPEN_FILES`, `MAX_READ_BYTES` |

실측 여유: 풀 스킨 선곡(destination 1,914개)의 로드 한 패스는 약 32만 명령, 프레임당 함수 값 전부를 한 번씩 부르는 비용은 평균 0.28 ms 이다. 기본 스킨은 최대 0.174 ms(코스 결과)다. 예산 안에서도 `table.sort`, `string.rep`, `table.concat` 처럼 C 라이브러리 한 번의 호출 안의 시간은 명령 수 훅이 보지 못하고 메모리 한도로만 길이가 묶인다.

## 5. 도구

### 5.1 `rbms-cli skin-dump`

게임 없이 스킨 문서를 읽어 무엇이 나왔는지 보고한다. 문서마다 화면의 상태를 흉내 낸 시나리오(`apps/rbms-cli/scenarios/*.json`)로 읽는다.

```
cargo run -p rbms-cli -- skin-dump <팩 폴더 또는 문서> [옵션]
```

| 옵션 | 뜻 |
| --- | --- |
| `--scenario <json>` | 내장 시나리오 대신 쓸 시나리오 파일(속성 id 를 키로 한 표들의 JSON) |
| `--overlay <폴더>` | 스킨의 쓰기를 받을 폴더. 없으면 임시 폴더를 만들어 끝에 지운다 |
| `--frames <N>` | 함수 값 전부를 N 프레임 부른 비용을 잰다(기본 0, 최대 1,000,000) |
| `--seed <n>` | 무작위 선택 고정(기본 1) |
| `--json` | 표 대신 기계가 읽는 JSON |
| `--probe-budget` | 각 문서가 돌 수 있는 가장 작은 명령 예산을 찾는다 |
| `--strict` | 깨끗하지 않으면 실패한다. 로드 실패, 스킨의 `pcall` 이 삼킨 오류, 함수 값 실패, 프레임 예산 초과가 하나라도 있으면 종료 코드 1 |
| `--allow-failure <문서>` | 로드되지 않아도 되는 문서를 지정한다(`--strict` 를 함께 켠다). 로드되면 그것도 실패다 |

출력 읽는 법

| 줄 | 뜻 |
| --- | --- |
| `header` | 선언한 type 과 화면 이름, 이름과 작성자, 크기, 읽은 파서 |
| `rows` | 옵션 행, 파일 슬롯, 오프셋(자동 오프셋 별도), 분류 수 |
| `load` | `ok in N ms` 또는 `FAILED`와 오류, 사용한 시나리오 |
| `timings` | scene, input, fadeout, loadend, playstart, close |
| `objects`, `functions` | destination 수(조립된 수)와 객체 종류별 수, 등록된 함수 값 수 |
| `files` | 선언한 source/폰트 수, 풀린 수, 디스크에 있는 수, 없는 파일 |
| `diagnostics` | 경고 수, `caught by the skin's own pcall`(삼켜진 오류), `function failures`, `printed` |
| `frames` | `--frames` 의 호출 수, 평균, 중앙값, p99, 최대, 예산 초과 프레임 수 |
| 마지막 `summary` 표 | 문서마다 한 줄: type, load, dest, fns, pcall, fn fail, warn, lua KiB, load ms, calls, mean, p99, max, over |
| 마지막 `pack folder` | 덤프 전후 파일 수와 `unchanged`. 스킨이 팩을 건드리면 `CHANGED` 와 함께 실패한다 |

기본 스킨의 기대값: 9개 문서 모두 `load ok`, `warn 0`, `pcall 0`, `fn fail 0`, `over 0`, 팩 `unchanged`.

```
cargo run -p rbms-cli -- skin-dump assets/skins/rbms-default --strict --frames 120
```

ModernChic 은 키 설정 문서(`keyconfig`)만 원본 결함(`Decide/lua/require/textproperty.lua:37`)으로 본문이 실패한다. `--allow-failure` 로 그 문서를 지정해 게이트로 쓴다.

### 5.2 캡처 하니스와 환경 변수

캡처는 `cargo test` 안에서 돈다(`apps/rbms-player/src/stage/capture.rs`). 실행 파일의 기능이 아니다.

| 환경 변수 | 효과 |
| --- | --- |
| `RBMS_SKIN_PACK` | 앱에서는 이번 실행의 팩. 테스트에서는 이름이 `named_by_the_environment` 로 끝나는 선택 테스트가 이 팩을 그린다(없으면 아무것도 하지 않고 통과). 절대 경로로 준다 |
| `RBMS_SKIN_CAPTURE_DIR` | 있으면 캡처 프레임을 `<폴더>/<이름>.png` 로 저장한다. GPU 판은 `<이름>.gpu.png`. 없으면 아무것도 쓰지 않는다 |
| `RBMS_REQUIRE_GPU` | GPU 어댑터가 없는 기계에서 GPU 비교 테스트가 조용히 건너뛰는 대신 실패하게 한다 |
| `RBMS_SKIN_SEED` | 와일드카드 파일 선택과 무작위 옵션을 고정한다 |
| `RBMS_GOLDEN_UPDATE` | 내장 화면 골든 기대값을 다시 만든다(`crates/rbms-render/tests`) |

```
RBMS_SKIN_PACK=$PWD/assets/skins/rbms-default RBMS_SKIN_CAPTURE_DIR=<폴더> \
  cargo test -p rbms-player --lib -- <테스트 이름> --nocapture --test-threads=1
```

환경 변수 없이 도는 기본 스킨 게이트는 다음이 지킨다: `apps/rbms-cli` 의 엄격 덤프 테스트(문서 9개 로드와 type, 경고 0, 팩 불변), `stage/capture/gate.rs`(선곡, 결정, 플레이 5/7/9/10/14, 결과, 코스 결과를 640x360 으로 그려 컴파일된 스킨 장면, 경고 0, 객체 종류, 모서리 배경색을 단언), `crates/rbms-skin/tests/default_pack.rs`(금지 API 점검, 시트 대조), `apps/rbms-player/src/assets/default_pack/tests.rs`(설치 규칙). 화면별 캡처 테스트 이름과 확인 항목은 설계서 §8 에 있다.

## 6. 알려진 한계

| 구분 | 한계 | 사정 |
| --- | --- | --- |
| 엔진 | `customTimers`, `customEvents` 는 로드만 되고 평가되지 않는다. id 로 가리킨 커스텀 타이머를 쓰는 destination 은 그려지지 않는다. destination 의 `timer` 에 Lua 함수를 직접 주면 그려진다 | 사양 웨이브 9. ModernChic 에서는 빈 테이블 |
| 엔진 | IR 랭킹 목록·라이벌 값(380~399 등)은 스킨에 연결되지 않았다. IR 상태 타이머(172~174)와 옵션(51 등)은 실제 값이다. IR 서버가 설정된 환경에서는 option 51 이 참이 되어 ModernChic 이 IR 메뉴를 구성에 넣는데 그 구성의 캡처는 찍지 못했다 | 웨이브 9 |
| 선곡 | 종류별 노트 수(350~353), 밀도(360~365), 주 BPM(92)은 빈칸이다 | songdb 열 추가와 전체 재스캔이 필요해 별도 작업(D9) |
| 선곡 | 표 이름·표 레벨(text 1001~1003, op 1008)은 선곡에서 빈칸이다. 결정 화면에서는 채워진다 | 미지원 |
| 선곡 | 패널 2의 BPM GUIDE, JUDGE AREA, MARK NOTE, NO MINE 과 패널 3의 일부 수치는 대응하는 설정이 없어 `NOT AVAILABLE` 이다 | |
| 선곡 | 앱이 스킨 위에 덧그리는 것과 스킨의 정합: IR 랭킹 패널이 정보 영역 오른쪽 124px 을 덮지 못한다, 옵션 오버레이가 반투명이라 뒤가 비친다, 토스트가 목록 아래쪽 막대를 덮는다 | 스킨으로 고칠 수 없는 앱 단위 작업 |
| 선곡 | 판정 알고리즘이 Score 일 때 QUICK SETTINGS 의 해당 행이 움직이지 않고 `SCORE (SET IN SETTINGS)` 로 알린다 | 번호 체계가 beatoraja 와 같다 |
| 플레이 | 미스 레이어 분리, BACKBMP·배너, 키별 판정 이미지 인덱스, bgaExpand 설정, 오토플레이 중 숫자키 배속, PMS 의 박자 확대는 원천 데이터가 없어 빠졌다 | |
| 플레이 | 곡(BMS)의 BGA 동영상은 그리지 않는다. 스킨의 동영상 source 만 디코드한다 | 같은 디코더로 후속(웨이브 9) |
| 플레이 | SCRATCH AUTO 가 치는 레인(턴테이블)의 노트는 스킨의 노트 그림 대신 엔진이 단색 초록(`#30E050`, 롱노트 몸통은 같은 초록의 불투명도 0xA0)으로 그린다. 스킨이 고른 그림의 자리에 칠하므로 어느 스킨이든 같고, 스킨은 이 색을 바꾸거나 모양을 살릴 수 없다. 그림 모양 그대로 칠하려면 그림의 알파로 색을 입히는 셰이더가 필요한데 지금 렌더러에 없다 | `docs/play-options.md` 10.5절. 후속: `apps/rbms-player/src/gpu` 에 그 셰이더 |
| 선곡 | 기본 스킨의 점수 칸(EX SCORE, RATE, RANK, 판정 막대)은 점수가 있는 기록이 있을 때만 채워진다. 넓힌 판정으로만 친 차트는 램프와 횟수만 있고 점수는 `-` 다(`lua/select/info.lua` 의 `holds_score`). 웨이브 18 부터 SCRATCH AUTO 판은 점수 기록이 된다 | `docs/play-options.md` 10.6절 |
| 결과 | 다시 하기, 다음 곡이 가능한지 알리는 속성이 없어 불가능한 런에서도 R, N 안내가 나온다. judgegraph 는 색 지정을 받지 않는다(엔진 고정 팔레트) | |
| 화면 | 24키 문서, type 9 스킨 설정 문서(설정 화면은 내장 SKIN 메뉴가 대신한다), 스킨 문서 안의 skinpreview 객체, `filter != 0` 전용 bilinear 셰이더는 없다 | 웨이브 9 |
| 설정 | 스킨 선택·옵션(`skin` 그룹 전체)은 기기 로컬이다. 계정 동기화로 올리거나 내려받지 않는다. 문서 절대 경로가 키라서 기기 간에 옮길 수 없다 | 조절한 팩을 다른 기기로 옮기려면 SKIN 메뉴의 EXPORT 로 내보낸 폴더를 가져간다(§2.8). 동기화에 싣는 것은 팩 이름 기준 키로 바꾸는 별도 작업이다 |
| 색 | 앱이 스킨 위에 덧그리는 것과 내장 화면은 무채색이 됐다(웨이브 15. §3.4.3). 남은 것은 하나다: 앱 테마는 스킨의 Theme·Accent 옵션을 따르지 않는다. 스킨을 Slate 나 Sky 로 바꿔도 앱의 메뉴는 무채색이다. 스킨 팩 없이 그리는 내장 플레이 화면의 레인 바탕·필드 테두리·레인 구분선도 회색이다(번들 `assets/skins/{normal,wide,default}.ron` 이 `SkinConfig` 의 기본값과 같은 `lane_bg (22, 22, 22)`, `outline_color (96, 96, 96)`, `divider_color (60, 60, 60)` 를 준다) | `theme.ron` 으로 사용자가 맞춘다([theme.md](./theme.md)). 스킨의 옵션을 앱 테마에 잇는 것은 정하지 않았다 |
| 설정 | 판 3 이하의 설정 파일에서 일부러 꺼 둔 LETTERBOX 도 업데이트 뒤 한 번 켜진다 | 그 판은 고른 값과 기본값을 구별해 적지 않았다. 다시 끄면 유지된다 |
| 선곡 | 곡 검색어를 치는 중에는 마우스로 다른 곳을 눌러도 입력이 끝나지 않는다(beatoraja 는 확정한다). 키를 누르고 있어도 글자가 반복되지 않는다 | Enter 로 확정, Esc 로 취소 |
| 곡 정보 | 짧은 한글만 든 EUC-KR 차트 일부(모든 바이트가 반각 가나 범위인 경우)와 Shift_JIS 파일에 다른 인코딩 한 줄이 섞인 차트는 반각 가나로 보일 수 있다 | 인코딩은 파일 전체에 한 번 판정하고 동점은 Shift_JIS 다(`crates/rbms-parser/src/encoding.rs`) |
| 확인 | 한글·일본어 제목의 폭, 기준선, 축소는 캡처로 확인하지 못했다(하니스에는 번들 Inter 하나뿐이고 실제 앱은 시스템 글꼴을 쓴다) | 수동 점검 항목에 포함 |
| 확인 | 실제 GPU 창에서의 첫 실행, 마우스 조작, 소리, 1280x720 창은 사람이 확인해야 한다 | `docs/quality-assurance/2026-10-11-default-skin-manual-check.md` |
| 확인 | Windows 와 Linux 에서의 결과는 푸시 뒤 CI 로 확인한다 | `docs/ci-release.md` |
