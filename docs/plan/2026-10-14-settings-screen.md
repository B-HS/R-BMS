# SETTINGS 화면 — 묶음 레일, 종류별 행, 정보 패널 (웨이브 16 K3) 설계서

> 최종 갱신 2026-10-14 · 대응 커밋 7095ef7 · 작성 2026-10-14 · 상태: 구현 완료(웨이브 16 커밋 4bed711, 웨이브 18 커밋 8b11a17). 이 문서는 최종 구현을 적는다. 같은 날 웨이브 16 통합에서 패널이 입력 모드를 따르게 하고(10.1절) 넓힌 판정의 경고 문구를 고쳤다(10절). 웨이브 18(같은 날)에 사용자 결정으로 스위치·선택 행의 `Enter` 를 SKIN·PLAY 메뉴처럼 다음 값으로 바꾸고(4절, 12절), 넓힌 판정의 문구를 새 저장 규칙에 맞췄다(10절).
> 사용자 요청(원문): "Tab누르면 나오는 메뉴를 조금 더 UI/UX 좋게 고도화 못하나? 지금 너무 딱딱하고 불편해보여."
> 관련 문서: 짝이 되는 두 메뉴의 설계 `docs/plan/2026-10-11-skin-menu.md`·`docs/plan/2026-10-11-play-menu.md`, 옵션 정의 `docs/play-options.md`, 색 `docs/theme.md`

## 1. 목표

SETTINGS(선곡에서 `Tab`)를 SKIN 메뉴(F12)·PLAY 메뉴(F8)와 같은 수준, 같은 문법으로 올린다. 항목과 그 뜻, 설정 파일의 형식은 그대로다(행을 없애거나 이름을 바꾸지 않는다. `rbms_config::SETTINGS` 표와 `SettingId` 는 손대지 않았다).

| # | 요구 | 판정 기준 |
| --- | --- | --- |
| R1 | 왼쪽 묶음 레일, 가운데 행 목록, 오른쪽 정보 패널 | 레일에 묶음 이름·항목 수·바뀐 묶음 표지. 고른 묶음은 밝은 면. 패널에 커서 행의 설명, 값·기본값·범위·단위, 그 행이 듣는 키 |
| R2 | 값의 꼴이 종류별로 다르다 | 스위치, 칸(값이 넷 이하이고 다 들어갈 때), 화살표 선택, 슬라이더와 직접 입력, 글자 칸, 줄여 쓴 경로와 열기 표지, 동작 표지 |
| R3 | 기본값에서 바뀐 행이 보이고 되돌릴 수 있다 | 행 왼쪽의 점, `0` 은 그 행을 기본값으로, `Shift`+`0` 은 확인을 거쳐 묶음 전체를 |
| R4 | 마우스 | 행 클릭은 커서만, 스위치·칸·화살표 클릭은 값, 슬라이더 끌기, 휠은 목록, 레일 클릭은 묶음, 오른쪽 클릭은 한 칸 뒤로 |
| R5 | 긴 묶음 | 소제목으로 끊고, 화면보다 길면 스크롤 막대와 `n / m` |
| R6 | 바닥 안내는 지금 행이 실제로 듣는 키만 | 행의 키가 앞, 화면의 키가 뒤. 안내가 말하는 Enter 의 동작이 실제와 같다(테스트로 묶음) |
| R7 | 정적인 화면 | 움직이는 꾸밈 없음. 같은 장면은 같은 프레임 |
| R8 | 색은 `Theme` 의 필드만 | 새 색 없음. 경고(`warn`)는 판정 폭을 넓혔을 때 한 곳 |
| R9 | 1280x720 에서 잘리지 않는다 | 라벨, 소제목, 표지, 선택지, 숫자, 설명, 바닥 안내를 테스트로 잰다 |
| R10 | 기존 동작 보존 | 키보드로 하던 일 전부, 다른 화면으로 가는 길과 돌아오는 자리, 직접 입력, 계정 행, 저장 시점 |

## 2. 바꾸기 전 화면에서 본 문제

캡처(1280x720 과 1920x1080, 묶음마다)를 열어 적은 목록이다.

| 관점 | 바꾸기 전 |
| --- | --- |
| 정보 밀도 | 720 폭의 가운데 기둥 하나. 좌우 280 씩은 비어 있다. 행 높이 42·간격 50 이라 한 화면에 10행. JUDGE(13행)·DISPLAY(14행)·NETWORK(14행)는 늘 스크롤해야 끝이 보인다 |
| 묶음과 위계 | 탭 안에서는 행이 표에 추가된 순서대로 한 줄로 늘어선다. DISPLAY 는 SKIN, FONT, SCORE GRAPH, … WINDOW MODE, BGA 처럼 창·화면·내장 필드 행이 섞여 있다 |
| 값의 종류 | 켬·끔(`ON`), 선택(`COMBO`), 경로(`(none)`), 동작(`>`), 글자(`guest`)가 전부 오른쪽 정렬한 글자 하나다. 숫자만 슬라이더가 있다. 눌러 보기 전에는 무엇이 일어날지 알 수 없다 |
| 기본값 | 지금 값이 기본값인지 알 길이 없다. `0` 은 숫자 행에서만 듣는다 |
| 설명 | 없다. JUDGE ALGORITHM 의 `COMBO` 가 무엇인지, JUDGE WIDTH 를 넓히면 점수가 남지 않는다는 것을 화면이 말하지 않는다 |
| 탭 이동 | `Tab` 은 앞으로만 돈다. 탭 띠 끝에 단추 둘, 그 위에 단추 하나가 따로 얹혀 있다 |
| 마우스 | 행 어디를 눌러도 값이 한 칸 넘어간다(커서를 옮기려던 클릭이 스위치를 뒤집는다). 휠은 듣지 않는다 |
| 위치 감각 | 스크롤 막대가 없고 오른쪽 아래의 작은 `1 / 13` 뿐이다. 마지막 보이는 행이 목록의 끝인지 알기 어렵다 |
| 키 안내 | 제목 아래 한 줄에 화면의 키 전부를 늘 같은 글로 적는다. 스위치 행에서도 `ENTER OPEN` 이라고 읽힌다(실제로는 저장하고 나간다) |

## 3. 참고한 것

| 대상 | 같은 문제를 푸는 방법 | 가져온 것과 가져오지 않은 것 |
| --- | --- | --- |
| 원본(beatoraja)의 설정 창 | 게임 밖의 JavaFX 창이다(`launcher/PlayConfigurationView.fxml` 의 `TabPane`: Video, Audio, Input, Resource, Music Select, Play Option, Skin, Other, IR, Table, Stream). 값의 꼴은 체크박스·콤보박스·숫자 스피너·슬라이더로 종류마다 다르고, 탭 안은 제목 붙은 칸으로 묶는다. 게임 안에는 설정 화면이 없다(키 설정과 스킨 설정만) | 종류마다 다른 꼴과 탭 안의 묶음. 게임 안 화면이라 동작이 같아야 할 원본이 없다: 묶음 구성(JUDGE·DISPLAY·INPUT·NETWORK·AUDIO·SELECT)은 R-BMS 의 것을 그대로 둔다 |
| IIDX INFINITAS 류의 설정 화면(아는 범위) | 왼쪽 분류 목록, 오른쪽 항목, 아래 설명 한 줄. 항목마다 지금 값이 보이고 좌우로 바꾼다 | 분류 목록과 설명 자리. 배치와 꾸밈은 베끼지 않았다(SKIN·PLAY 메뉴의 배치 상수를 따른다) |
| SKIN 메뉴·PLAY 메뉴 | 왼쪽 목록(묶음 이름이 한 칸), 슬라이더와 직접 입력, 오른쪽 정보 패널, 바닥 안내, 휠과 오른쪽 클릭 | 제목 자리와 크기, 여백 24, 바닥 안내 자리, 행 글자 크기, 소제목, 스크롤 막대와 `n / m`, 숫자 행 부품(`number_row.rs`), 줄바꿈(`wrapped`) |

## 4. 화면 배치 (1280x720 논리 좌표)

배치 상수는 `apps/rbms-player/src/settings_view.rs` 에 있다. 제목·여백·바닥 안내는 `stage/preview_menu.rs` 의 값을 그대로 읽는다.

| 영역 | x | y | w | h | 내용 |
| --- | --- | --- | --- | --- | --- |
| 머리 | 24 | 16 | — | — | 제목 `SETTINGS`(배율 2.4). 그 옆(배율 1.6, `text_dim`)에 연 묶음이 무엇을 담는지 한 줄(`catalog::group_summary`) |
| 레일 | 24 | 72 | 188 | 604 | 묶음 여섯(칸 높이 34, 간격 38). 이름(배율 1.5)과 오른쪽에 항목 수. 바뀐 행이 있는 묶음은 수 앞에 점. 고른 묶음은 `button` 면과 `title_focus` 글자 |
| 묶음 되돌리기 | 24 | 310 | 188 | 30 | `RESET GROUP   SHIFT+0`. 연 묶음에 바뀐 행이 있을 때만 밝다 |
| 다른 화면 | 24 | 566 | 188 | 34×3 | `OTHER SCREENS` 아래 `PLAY MENU  F8 >`, `SKIN MENU  F12 >`, `KEY CONFIG  F5 >`(키 이름은 키 설정에서 읽는다) |
| 행 목록 | 228 | 72 | 668 | 604 | 소제목과 행 18칸(칸 높이 28, 간격 32). 행의 왼쪽 8 에 바뀜 점, 20 부터 라벨(배율 1.3), 오른쪽 300 은 값의 자리. 오른쪽 아래에 `n / m`(행 기준), 넘치면 오른쪽 끝에 스크롤 막대 |
| 정보 패널 | 912 | 72 | 344 | 604 | 커서 행: 이름(배율 1.6), 종류(`SWITCH`, `CHOICE 2 OF 4`, `NUMBER`, `TEXT`, `FILE`, `FOLDER`, `ACTION`), 설명(여섯 줄까지), 선, `VALUE`·`DEFAULT`·`RANGE`·`STEP`·`CHOICES`, 경고·표지 풀이, `KEYS`, 맨 아래 `STATUS`(AUDIO 의 출력 상태, 그 밖에는 서버 응답) |
| 바닥 안내 | 24 | 688 | 1232 | — | 키 안내 한 줄(배율 1.2) |

- 모든 묶음이 스크롤 없이 한 화면에 들어온다: 소제목을 넣은 칸 수가 JUDGE 16, DISPLAY 18, INPUT 3, NETWORK 18, AUDIO 13, SELECT 6 이고 보이는 칸은 18 이다. 행이 늘어 18 을 넘으면 스크롤 막대가 생긴다.
- RIVALS·IR PROFILES 목록과 묶음 되돌리기 확인은 행 목록 위에 테두리 있는 판으로 뜬다.
- 전환은 없다. 한 프레임은 장면(`SettingsScene`)이 말하는 것만 그린다.

## 5. 묶음과 소제목

행이 속한 묶음(`SettingTab`)은 표 그대로다. 묶음 안의 순서와 소제목은 화면의 것이고 `apps/rbms-player/src/stage/settings/catalog.rs` 의 `sections` 가 정한다. 표에 있는데 어느 소제목에도 없는 행은 그 묶음 끝의 `OTHER` 아래에 나온다(표에 행을 더하면 이 파일을 고치기 전에도 화면에 보인다).

| 묶음 | 소제목 | 행(위에서 아래) |
| --- | --- | --- |
| JUDGE | JUDGING | JUDGE ALGORITHM, JUDGE WIDTH KEY PG·GR·GD, JUDGE WIDTH SCR PG·GR·GD |
| | LONG NOTES | LN MARGIN, LN MODE |
| | GAUGE | GAUGE SET, GAUGE AUTO SHIFT, BOTTOM SHIFTABLE, TOTAL |
| DISPLAY | WINDOW | WINDOW MODE, RESOLUTION, LETTERBOX, FONT |
| | SCREENS | BGA, SCORE GRAPH, RESULT GRAPHS, REPLAY ANALYSIS, PREVIEW |
| | BUILT-IN PLAY FIELD | SKIN, 5KEYS LAYOUT, WHITE NUMBER, JUDGE TEXT Y |
| | DIAGNOSTICS | DEBUG MODE |
| INPUT | CONTROLS | SCRATCH SIDE, KEY CONFIG |
| NETWORK | SERVER | SERVER URL, PLAYER ID |
| | ACCOUNT | ACCOUNT, EMAIL, PASSWORD, LOGIN, REGISTER, LOGOUT |
| | SETTINGS SYNC | SYNC SETTINGS, UPLOAD SETTINGS NOW, DOWNLOAD SETTINGS NOW |
| | SCORES | AUTO UPLOAD REPLAY, RIVALS, IR PROFILES |
| AUDIO | VOLUME | MASTER VOL, KEY VOL, BGM VOL, SYSTEM VOL |
| | OUTPUT | AUDIO DEVICE, BUFFER SIZE, SAMPLE RATE, POLYPHONY |
| | SYSTEM SOUNDS | SOUND FOLDER, GUIDE SE |
| SELECT | SONG LIST | SORT, FAVORITE ONLY |
| | PREVIEW | PREVIEW VOLUME, PREVIEW FADE |

DISPLAY 만 표의 순서와 다르다(창, 화면이 보이는 것, 내장 필드만 읽는 행, 진단 순으로 다시 모았다). 나머지 다섯 묶음은 표의 순서 그대로 끊기만 했다. `SettingId` 의 순서와 `settings_ui.rs` 의 `SHIPPED_ROWS` 표는 바뀌지 않았다.

### 5.1 TARGET 에 관한 행의 자리(K2 용)

TARGET 은 `SettingTab::PlayChart` 의 행이라 이 화면이 아니라 PLAY 메뉴(F8)의 CHART 묶음에 있다. 목표가 없을 때 물러날 점수(fallback)는 같은 묶음에서 TARGET 바로 아래의 TARGET FALLBACK, TARGET RATE 두 행으로 더해졌다(같은 웨이브 16. `rbms_config::SETTINGS` 의 순서가 PLAY 메뉴의 순서다). 이 화면의 묶음에는 넣지 않았다(§12). 넣는다면 `sections` 에 한 줄을 더하면 되고, 더하지 않아도 `OTHER` 아래에 나온다. JUDGE 는 18칸 중 16칸을 쓰므로 두 행까지는 스크롤 없이 들어간다.

## 6. 행의 종류와 꼴

종류는 표의 `SettingKind` 에서 온다(`AppShared::setting_control`, `settings_ui.rs`).

| 종류(`RowControl`) | 해당하는 행 | 꼴 |
| --- | --- | --- |
| `Switch` | 켬·끔(`Toggle`) | 값 글자(`ON`/`OFF`)와 스위치. 켜지면 밝은 면에 손잡이가 오른쪽, 꺼지면 어두운 면에 손잡이가 왼쪽 |
| `Segments` | 선택(`Cycle`) 중 값이 넷 이하이고 모두 칸에 들어가는 것: JUDGE ALGORITHM, LN MODE, GAUGE SET, BOTTOM SHIFTABLE, WINDOW MODE, SKIN, SCRATCH SIDE | 값 전부가 나란한 칸. 고른 칸이 밝은 면에 어두운 글자. SKIN 이 명령줄의 레이아웃 파일(`CUSTOM`)이면 어느 칸도 켜지지 않는다 |
| `Stepper` | 그 밖의 선택: GAUGE AUTO SHIFT, RESOLUTION, SORT, BUFFER SIZE, SAMPLE RATE, AUDIO DEVICE(값이 실행 중에만 정해져 늘 이 꼴) | `<` 값 `>` |
| `Slider` | 숫자(`number_spec` 이 있는 행) | 슬라이더(폭 200)와 수(자리 64). 직접 입력 중에는 그 자리에 입력 칸 |
| `Field` | 글자(`Text`): SERVER URL, PLAYER ID, EMAIL, PASSWORD | 검은 칸에 값. 비어 있으면 흐린 `(none)`. 입력 중에는 밝은 테두리. PASSWORD 는 전과 같이 길이를 가린 `*` 만 |
| `Path` | FONT, SOUND FOLDER | 앞을 줄인 경로(`…/sounds/default`)와 `>`. 고른 것이 없으면 흐린 `DEFAULT`·`(none)` |
| `Action` | 동작(`Action`): KEY CONFIG, ACCOUNT, LOGIN, REGISTER, LOGOUT, UPLOAD·DOWNLOAD SETTINGS NOW, RIVALS, IR PROFILES | `>`. 알릴 값이 있으면 그 앞에(계정 상태, 라이벌 수) |

- 스킨이 읽지 않는 행의 표지(`BUILT-IN SCREENS ONLY`, `SKIN OPTION COMES FIRST`)는 전처럼 라벨 뒤에 작게 쓰고, 정보 패널이 그 뜻을 문장으로 푼다.
- FONT 행은 고른 파일의 경로를 보인다(전에는 `CUSTOM`). 표가 말하는 값(`setting_line`)은 그대로 `CUSTOM` 이다.

## 7. 기본값 표지와 되돌리기

| 항목 | 동작 |
| --- | --- |
| 되돌릴 수 있는 행 | 스위치, 선택, 숫자, FONT, SOUND FOLDER(`settings_ui::is_resettable`). 글자(서버 주소, 플레이어 ID, 이메일, 비밀번호)와 동작은 아니다 |
| 표지 | 지금 값이 새 설정(`Config::default()`)의 값과 다르면 행 왼쪽에 점. 레일의 그 묶음에도 점. 정보 패널의 `DEFAULT` 뒤에 `(0 PUTS IT BACK)` |
| `0` / 숫자패드 `0` | 커서 행을 기본값으로. 숫자는 출하 걸음으로, FONT·SOUND FOLDER·AUDIO DEVICE·SKIN 은 직접 지정, 스위치와 나머지 선택은 기본값을 읽을 때까지 앞으로 넘긴다(그 행의 한 칸이 하는 일이 그대로 따라온다: 볼륨 반영, 출력 다시 열기 예약, 가이드음 반영, NETWORK 스위치의 저장) |
| `Shift`+`0`, `RESET GROUP` 단추 | 연 묶음에 바뀐 행이 있으면 확인 판을 띄운다(`RESET AUDIO?  5 CHANGED ROWS GO BACK TO DEFAULT.`). `Enter` 또는 첫 단추 클릭이면 묶음의 되돌릴 수 있는 행을 전부 되돌리고 설정 파일을 쓴다. `Esc`, 둘째 단추, 판 밖 클릭은 그대로 둔다. 판이 떠 있는 동안 다른 키는 행에 닿지 않는다 |

원본의 설정 창에는 항목별 되돌리기가 없다. `0` 은 SKIN·PLAY 메뉴가 이미 쓰는 키라 그대로 따랐다.

## 8. 키

| 키 | 동작 | 전과 비교 |
| --- | --- | --- |
| `↑` `↓` | 커서(소제목은 건너뛴다) | 같다 |
| `PgUp` `PgDn` | 앞·뒤 소제목의 첫 행 | 새로 |
| `Home` `End` | 묶음의 첫·마지막 행 | 새로 |
| `←` `→` | 스위치 뒤집기, 선택 한 칸, 숫자 한 걸음(`Shift` 는 열 걸음). FONT 는 `→` 파일 고르기·`←` 기본, SOUND FOLDER 는 `→` 폴더 고르기·`←` 없음 | 같다 |
| `Enter` | 숫자: 직접 입력. KEY CONFIG: 키 설정. FONT: 파일 고르기. SOUND FOLDER: 폴더 고르기. NETWORK 의 행: 편집·실행·목록·스위치. 스위치: 뒤집기. 선택: 다음 값. 어느 행에서도 화면을 닫지 않는다 | 웨이브 18 에 바뀜: 전에는 스위치·선택·SOUND FOLDER 에서 저장하고 닫았다(§12) |
| `0` | 커서 행을 기본값으로 | 숫자 행만 듣던 것을 되돌릴 수 있는 행 전부로 |
| `Shift`+`0` | 묶음 되돌리기 확인 | 새로 |
| `Tab` / `Shift`+`Tab` | 다음 / 앞 묶음(끝에서 돈다) | `Shift`+`Tab` 이 새로 |
| `F8` `F12` `F5`(키 설정에서 바꾼 키) | PLAY 메뉴, SKIN 메뉴, 키 설정 | 같다 |
| `Esc` | 저장하고 닫기. 입력 중·목록·확인 판에서는 그것만 닫는다 | 같다 |

바닥 안내는 커서 행이 듣는 키를 먼저, 그 뒤에 화면의 키를 쓴다(`info::row_keys`, `info::hint`).

| 커서 행 | 안내의 앞부분 |
| --- | --- |
| 숫자 | `LEFT RIGHT 1 STEP   SHIFT x10   ENTER TYPE   0 DEFAULT   DRAG SLIDER` |
| 스위치 | `LEFT RIGHT SWITCH   ENTER SWITCH   0 DEFAULT` |
| 선택 | `LEFT RIGHT CHANGE   ENTER NEXT   0 DEFAULT` |
| 글자 | `ENTER EDIT` |
| 동작 | `ENTER RUN`, KEY CONFIG 는 `ENTER OPEN`, RIVALS·IR PROFILES 는 `ENTER OPEN THE LIST`, ACCOUNT 는 `ENTER SHOW ITS STATE` |
| FONT | `ENTER CHOOSE A FILE   0 DEFAULT` |
| SOUND FOLDER | `ENTER RIGHT CHOOSE A FOLDER   0 NONE` |

뒤따르는 화면의 키: `UP DOWN MOVE   PGUP PGDN SECTION   TAB GROUP   SHIFT+0 RESET GROUP   F8 PLAY   F12 SKINS   F5 KEYS   ESC SAVE`. 숫자 입력 중, 글자 입력 중, 목록, 확인 판에서는 그때 듣는 키만 쓴다.

## 9. 마우스

| 조작 | 동작 | 전과 비교 |
| --- | --- | --- |
| 행의 라벨 쪽 클릭 | 커서만 옮긴다 | 전에는 값이 한 칸 넘어갔다 |
| 스위치(와 그 글자) 클릭 | 뒤집는다 | |
| 칸 클릭 | 그 값으로 | 새로 |
| `<` / 값·`>` 클릭 | 한 칸 뒤 / 앞 | 새로 |
| 슬라이더 누르기·끌기, 수 클릭 | 전과 같다(누른 자리로, 손잡이는 잡은 자리에서, 놓을 때 한 번 저장. 수를 누르면 직접 입력) | 같다 |
| 글자 칸·경로·동작의 값 자리(오른쪽 300) 클릭 | 편집·고르기·실행 | 전에는 행 어디든 |
| 오른쪽 클릭 | 값이 있는 행(스위치, 선택, 숫자)을 한 칸 뒤로. 글자·경로·동작은 듣지 않는다 | 전에는 숫자 행만 |
| 휠 | 포인터가 행 목록 위에 있을 때 목록을 굴린다(커서는 그대로) | 새로 |
| 레일의 묶음 | 그 묶음을 연다 | 탭 클릭과 같다 |
| `RESET GROUP`, 확인 판의 두 단추 | §7 | 새로 |
| `PLAY MENU`·`SKIN MENU`·`KEY CONFIG` | 그 화면을 연다 | 자리만 레일 아래로 |

## 10. 정보 패널의 문구

설명은 영어로 새로 썼다(`catalog::description`). 표의 한 줄 도움말(`SettingDescriptor::help`)과 코드·문서에서 확인한 사실만 적는다. 설명이 없는 행은 표의 도움말이 대신 나온다.

| 확인한 사실 | 근거 |
| --- | --- |
| JUDGE ALGORITHM 의 네 값이 하는 일 | `crates/rbms-judge/src/algorithm.rs` `JudgeAlgorithm` |
| JUDGE WIDTH·LN MARGIN 이 100% 를 넘으면 그 런은 커스텀 판정이라 최고 기록을 바꾸지 않고 전송되지 않으며 램프는 ASSIST EASY 까지. 이력·램프·플레이 횟수는 남는다(`docs/play-options.md` 10.2절) | `judge_setup.rs` `is_custom_judge`, `lib.rs` `updates_score`·`assisted_lamp`, `rbms-ir` `CUSTOM_JUDGE_ASSIST` |
| LN MODE 는 차트가 정하지 않은 롱노트에만 적용 | `crates/rbms-judge/src/ln.rs` `LnMode` |
| GAUGE AUTO SHIFT 의 다섯 값, BOTTOM SHIFTABLE 의 범위 | `crates/rbms-judge/src/gauge.rs` `GaugeAutoShift`, `BOTTOM_SHIFTABLE_GAUGES` |
| TOTAL 0 은 차트의 값, 1~5000 | `schema.rs` `TOTAL_FROM_CHART`, `settings.rs` `TOTAL_ROW_MAX` |
| BORDERLESS 는 늘 비율을 맞춘다 | `schema.rs` `DisplayOptions::fits_screen_shape` |
| 내장 필드만 읽는 네 행, SCRATCH SIDE 와 스킨 옵션의 관계 | `settings.rs` `unread_by_skins`, `docs/play-options.md` §7.5 |
| LOGIN 은 PLAYER ID 와 PASSWORD, REGISTER 는 EMAIL 도. 비밀번호는 쓰고 나면 지운다. LOGOUT 은 PLAYER ID 를 guest 로 | `app_network.rs` `start_auth`·`finish_logout` |
| 출력 장치·버퍼·샘플레이트·폴리포니는 출력을 다시 연다 | `settings.rs` `reopen`, `audio.rs` |

판정 폭이 100% 를 넘는 행에 커서가 있으면 패널이 `warn` 색으로 경고한다(`OVER 100%: A RUN PLAYED LIKE THIS SETS NO BEST SCORE. IT IS STILL KEPT, WITH ITS REPLAY, AND SENT.`). JUDGE WIDTH 여섯 행과 LN MARGIN 행의 설명(`catalog.rs`)도 같은 말이다(`no best score, the ASSIST lamp at best; the run is still kept, with its replay, and sent`. 2026-10-22 에 id 2 램프 이름이 `ASSIST` 가 되어 `ASSIST EASY at best` 에서 고쳤다). 웨이브 18 전의 문구는 `... SETS NO BEST SCORE AND IS NOT SENT. ITS LAMP AND HISTORY ARE KEPT.` 였고, 사용자 결정(2026-10-14, 사람이 친 판은 모두 저장·전송)으로 넓힌 판정의 판도 리플레이와 함께 남고 보내지게 되어 고쳤다. 웨이브 16 통합 전의 문구는 `... IS NOT SCORED OR SENT.` 였는데, 실제로는 이력·램프·횟수가 남아 "기록이 저장되지 않는다"는 오해를 되풀이했다. 행 설명과 README 의 같은 문장도 함께 고쳤다.

### 10.1 입력 모드 중의 패널

숫자나 글자를 치는 동안, RIVALS·IR PROFILES 목록이 열린 동안, 묶음 되돌리기를 묻는 동안 패널의 KEYS 는 그 모드가 듣는 키로 바뀐다(`info::NUMBER_EDIT_KEYS`, `TEXT_EDIT_KEYS`, `LIST_KEYS`, `CONFIRM_KEYS`). 숫자와 글자를 치는 중이면 VALUE 는 치고 있는 값이고, 비밀번호는 칸에 그리는 것과 같은 마스크(`editor_display`)로 쓴다. 고르는 순서는 바닥 안내(`SettingsState::hint`)와 같다(`SettingsState::busy_info`). 웨이브 16 통합 전에는 바닥 안내만 바뀌고 패널은 편집 전 상태(`ENTER OPEN THE LIST`, `VALUE (none)`)로 남았다.

## 11. 구현 위치

| 파일 | 내용 |
| --- | --- |
| `apps/rbms-player/src/settings_view.rs` | 장면(`SettingsScene`, `SettingsLine`, `RowControl`, `InfoScene`), 배치, 그리기, 누른 자리가 행의 어느 부분인지(`control_part_at`) |
| `apps/rbms-player/src/stage/settings.rs` | 화면의 상태와 키·마우스·휠. 되돌리기(`reset_row`, `reset_group`), 칸 고르기(`choose`), 스크롤(`reveal_cursor`) |
| `apps/rbms-player/src/stage/settings/catalog.rs` | 소제목(`sections`, `arranged`), 묶음 한 줄 요약, 행 설명 |
| `apps/rbms-player/src/stage/settings/info.rs` | 정보 패널의 내용과 바닥 안내(`row_keys`, `hint`, `info`) |
| `apps/rbms-player/src/settings_ui.rs` | 행의 꼴(`setting_control`), 바뀜 여부(`setting_changed`, `changed_rows`), `is_resettable`, `shipped_value` |
| `apps/rbms-player/src/stage/preview_menu.rs` | 스크롤 계산과 막대를 보이는 칸 수를 받는 꼴로 넓혔다(`last_top_of`, `revealed_top_of`, `wheel_scrolled_over`, `scroll_thumb_in`). 기존 함수는 17칸을 넘겨 부르므로 SKIN·PLAY 메뉴의 동작은 같다 |

`Hot`(lib.rs)에는 변형을 더하지 않았다. 레일의 묶음은 `Hot::SettingTab`, 행은 `Hot::SettingRow`, 두 메뉴 단추는 전과 같은 변형이고, 키 설정 단추·묶음 되돌리기 단추·확인 판의 단추는 배치 함수(`key_config_link`, `reset_group_rect`, `confirm_yes_rect`)에 대고 누른 자리를 잰다.

## 12. 그대로 둔 것과 고친 것

| 항목 | 내용 |
| --- | --- |
| `Enter` | 웨이브 18 에 고쳤다(사용자 결정 2026-10-14). 스위치는 뒤집고 선택은 다음 값으로 바꾸며(오른쪽 화살표와 같은 한 칸), SOUND FOLDER 는 폴더를 고른다. SKIN·PLAY 메뉴와 같다. 닫기는 `Esc` 만 한다. 숫자·KEY CONFIG·FONT·NETWORK 의 행은 전과 같다(LOGOUT 은 전처럼 확인 없이 바로 로그아웃한다). 구현: `stage/settings.rs` 의 `Enter` 가지가 `step(.., ROW_STEP_FORWARD, ..)` 로 가고, 바닥 안내는 `info::row_keys`. 테스트 `the_line_of_keys_says_what_the_row_under_the_cursor_hears` |
| 저장 시점 | 전과 같다: `Esc`, 드래그를 놓을 때, 글자 확정, NETWORK 스위치, FONT. 묶음 되돌리기는 확정하면 쓴다 |
| GUIDE SE | 고쳤다. 스위치를 바꿔도 가이드음 사용 여부가 시스템 사운드에 전달되지 않아 다음 실행이나 사운드 폴더를 다시 읽을 때까지 반영되지 않았다(`run_action` 의 닿지 않는 가지에만 `set_guide_enabled` 가 있었다). 이제 한 칸 움직인 뒤에 바로 전달한다 |
| 판정·오디오·형식 | 판정 창과 알고리즘, 오디오 시계, 설정 파일·키 설정·기록 형식은 건드리지 않았다. 내장 화면의 골든 서명 중 이 화면의 것은 없다 |
| TARGET·TARGET FALLBACK·TARGET RATE | 이 화면에 두지 않았다. 표의 행은 묶음 하나에만 속하고(`rbms_config::tab_rows` 의 단일 소속), 세 행은 PLAY 메뉴 > CHART 에서 미리보기와 함께 바뀐다. 이 화면에서는 레일 아래 `PLAY MENU` 단추나 F8 로 간다(`docs/play-options.md` 11.3절) |

## 13. 검증

| 대상 | 테스트(파일) |
| --- | --- |
| 묶음·소제목이 표의 행을 빠짐없이 한 번씩 담는다, 모든 행에 설명이 있다 | `stage/settings/catalog.rs` |
| 세 기둥과 레일의 부분이 겹치지 않는다, 프레임이 내는 영역, 긴 목록과 막대, 누른 자리의 판별, 칸으로 그릴지의 판단, 종류별 꼴, 1280x720 에서 잘리지 않음, 설명이 패널에 다 들어감, 경로 줄임, 점·표지가 제 행 안에만 | `stage/settings/view_tests.rs`(`settings_view` 의 테스트) |
| 레일과 바뀐 수, 스위치·칸·화살표·글자 칸·동작의 키와 마우스, SKIN 의 `CUSTOM`, `0` 과 표지(되돌릴 수 있는 행 전부), 묶음 되돌리기와 확인, 스크롤과 휠, 바닥 안내와 `Enter` 의 실제 동작, 패널의 내용과 경고, 입력 모드 중의 패널(`the_panel_follows_what_has_the_keys`) | `stage/settings/layout_tests.rs` |
| 숫자 행(키, 끌기, 직접 입력, 손잡이), 라벨 클릭과 오른쪽 클릭 | `stage/settings/number_tests.rs` |
| SKIN·PLAY 메뉴로 가는 길과 돌아오는 자리, 입력 중에는 닫힘. 테스트 이름의 `tab_strip` 은 통합에서 `rail` 로 고쳤다 | `stage/settings/skin_tests.rs`, `play_tests.rs` |
| 스킨이 읽지 않는 행의 표지 | `stage/settings/note_tests.rs` |

캡처: `RBMS_SKIN_CAPTURE_DIR=<폴더> cargo test -p rbms-player --locked --lib -- stage::settings` 가 `settings-group-<묶음>-<크기>.png` 와 `settings-state-<상태>-<크기>.png`(바뀐 행, 확인 판, 긴 목록, 끄는 중, 숫자 입력 중, 글자 입력 중, 라이벌 목록, 스킨이 읽지 않는 행)를 쓴다.
