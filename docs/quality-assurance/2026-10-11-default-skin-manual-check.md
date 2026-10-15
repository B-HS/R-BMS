# 기본 스킨 수동 확인 목록

최종 갱신 2026-10-14 · 대응 커밋 7095ef7 · 대응 단계: `docs/PROCESS.md` L6(최종 검증). 3절은 웨이브 8C 부터 20 까지 기능마다 쌓였고(3.1~3.17절, 웨이브 10A~18 의 구현은 모두 커밋됐다), 아래 목록의 항목은 아직 하나도 사람이 실행해 체크하지 않았다(미체크 114줄, 실행 대상 ID 116개. N9 와 W8 은 다른 절로 옮긴 표지라 실행하지 않는다).

자동 검사는 헤드리스 캔버스와 테스트 안의 앱으로 화면을 그린다. 실제 창(GPU, 마우스, 키보드, 소리, 시스템 글꼴, 다른 창 비율)에서만 드러나는 것은 사람이 확인해야 한다. 이 문서는 자동으로 확인된 것과 사람만 확인할 수 있는 것을 나누고, 후자를 체크박스로 추적한다. 체크하지 못한 항목이나 실패한 항목은 증상, 재현 조건, 확인한 날을 항목 아래에 적고, 결함이면 `docs/bug/` 에 따로 기록한다. 항목의 정본(방법·기대 결과 전문)은 3절이고, 아래 0절의 표는 찾기 위한 요약이다.

## 0. 실제 창에서 확인할 것 (미체크 일람)

### 0.1 먼저 확인할 것 (v0.1.3 게시 전후)

릴리스 직전에 최소한으로 보는 목록이다. 앞의 둘은 사용자가 이미 기다리는 일이다.

| 순서 | 항목 | 이유 |
| --- | --- | --- |
| 1 | W3 → X3: 로그인한 채 한 판을 쳐서 결과 화면에 `IR: RANK` 가 나오는지 | v0.1.2 의 400(큰 시드, `docs/bug/2026-10-14-score-submit-400-large-seed.md`)이 운영 서버에서 실제로 풀렸는지 본 사람이 없다 |
| 2 | A1, A2: dmg 로 설치한 `R-BMS.app` 을 Finder 에서 열어 첫 실행·재실행 | 앱 번들 배포는 v0.1.2 부터이고 Finder 로 처음 열 때 붙을 수 있는 `-psn_...` 인자는 곡 경로로 읽지 않게 고쳤지만(단위 테스트만 있음) 실제 Finder 실행의 기록이 없다. 설정 폴더(`$HOME/.config/rbms/`)가 만들어지는지도 함께 본다 |
| 3 | X1, X2, X4 | SCRATCH AUTO 새 규칙(초록 노트, 키음, 같은 배치 다시 하기)은 웨이브 18 의 사용자 요청 그대로이고 GPU·소리·컨트롤러에서만 확인된다 |
| 4 | X7, X8, X9 | 웹 연동(기본 발광 표, 설정 스키마 6, MY FOLDERS)은 실제 서버가 필요하다 |
| 5 | D1 | 번들 글꼴은 라틴뿐이라 한글·일본어 제목은 시스템 글꼴 폴백이 유일한 경로이고 자동 확인이 없다 |
| 6 | B2 | 7키 한 곡의 선곡 → 결정 → 플레이 → 결과 전체 흐름 한 번 |
| 7 | N3 | 아날로그 턴테이블 실기기(START + 디스크). 실기기 없이는 확인되지 않는다 |
| 8 | Windows 확인(`docs/reference/windows-compat.md` 의 미체크 7항목) | Windows zip 을 게시하기 전. 아직 실제 Windows 에서 연 기록이 없다 |

### 0.2 전체 표

열: 절·ID / 무엇을 / 어떻게 / 기대 결과. 체크는 3절의 체크박스에 한다(이 표는 요약이므로 상태를 두지 않는다).

| 절·ID | 무엇을 | 어떻게 | 기대 결과 |
| --- | --- | --- | --- |
| 3.1 A1 | 첫 실행 설치 | 빈 임시 HOME 으로 곡 폴더를 주고 실행, 종료 뒤 `$HOME/.config/rbms/skin/rbms-default/` 확인 | 선곡이 기본 스킨으로 그려진다. 문서 9, `lua/` 27, `image/` 5, `font/` 2, `sound/` 22, `install-manifest.txt` 가 있고 `tools/` 는 없다. 창이 뜨기까지 지연이 없다 |
| 3.1 A2 | 재실행 | 같은 HOME 으로 다시 실행하고 `ls -l` | 설치 파일의 수정 시각이 안 바뀌고 경고 알림이 없다 |
| 3.1 A3 | 고친 파일 보존과 지운 파일 복구 | `lua/theme.lua` 에 공백 줄을 더하고 `image/common.png` 를 지운 뒤 다시 실행 | `theme.lua` 는 그대로, `common.png` 는 다시 생기고 화면이 정상 |
| 3.1 A4 | 쓰기 불가 | 설정 폴더를 읽기 전용으로(또는 `skin` 이름의 일반 파일을 만들어) 두고 실행 | `default skin not fully installed: N files could not be written beside …` 알림이 한 번 뜨고 내장 화면으로 그려지며 앱이 안 죽는다 |
| 3.2 B1~B5 | 키 수별 전체 흐름(5/7/9/10/14키) | 선곡 → Enter → 결정 → 플레이 → 완주(또는 실패) → 결과 → Enter 로 선곡 복귀. 5키는 `samples/preview-demo` | 결정 약 3초 뒤 플레이, 판정·콤보·게이지 갱신, 실패 시 `FAILED` 와 어두워짐, 결과의 랭크·점수·판정 표·그래프 4개, 5·10키는 턴테이블로 게이지 그래프 전환, 멈춤·깜박임·검은 프레임 없음 |
| 3.2 B6 | 코스 | Shift+Tab 으로 COURSE 탭, 2스테이지 이상 코스 시작 | 첫 스테이지만 결정 화면, 스테이지 사이 결과 없음, 마지막에 COURSE RESULT |
| 3.3 C1 | 옵션 패널 1~3 | 선곡에서 A(START) 누른 채 → W(SELECT) → 둘 함께(또는 숫자 5) | 누르는 동안만 패널, 칸 7개와 값, 건반으로 바꾼 값이 플레이에 반영 |
| 3.3 C2 | 사이드 메뉴와 볼륨 | 왼쪽 가장자리의 톱니와 스피커 클릭, 볼륨 슬라이더 드래그 | QUICK SETTINGS 6행 / VOLUME 3개, 한 번에 하나, 효과음, 실제 소리 크기가 바뀜 |
| 3.3 C3 | 마우스 호버·클릭·드래그 | 버튼, 곡 막대, 스크롤 손잡이, 휠 | 호버 강조, 좌클릭 다음 / 우클릭 이전, 곡 막대 좌클릭 선택 / 우클릭 닫기, 스킨 밖 클릭 무반응 |
| 3.3 C4 | 검색 | SEARCH 칸 클릭 또는 `/`, 한글·일본어 IME, 치는 중 F1·방향키, Enter, Esc | 스킨 검색 칸에 입력, 조합 글자 밑줄, 경로 줄 `Search : '검색어' >`, Esc 로 원래 폴더·행 복귀 |
| 3.3 C5 | 기록 표시 | 기록 있는 곡 / 없는 곡 / 그림 없는 곡을 봄 | 랭크·판정 5행·숫자 6칸, 없으면 줄표와 NO PLAY, `NO STAGE IMAGE` |
| 3.4 D1 | 한글·일본어 제목 | CJK 곡을 선곡 막대·곡 머리·결정·결과에서 봄 | 빈 사각형 없이 시스템 글꼴, 긴 제목은 폭에 맞게 줄고 겹치지 않음 |
| 3.4 D2 | 1280x720 창 | 기본 창으로 모든 화면 | 18픽셀 글자가 읽히고 잘림 없음 |
| 3.4 D3 | 다른 해상도·비율 | DISPLAY 탭 RESOLUTION 1920x1080/2560x1440, BORDERLESS, 창을 4:3·21:9 로, LETTERBOX 켬/끔 | 켬: 16:9 가 비율 유지로 가운데, 여백은 검정. 끔: 창에 맞게 늘어남. BORDERLESS 는 항상 비율 유지. 클릭 위치가 그려진 요소와 일치 |
| 3.4 D4 | 새 설치의 LETTERBOX 기본값 | 빈 HOME 에서 DISPLAY 탭 | LETTERBOX ON (저장된 설정이 있으면 그 값) |
| 3.5 E1 | 외부 팩으로 바꾸기 | F12 → SKIN 메뉴 → PACK FOLDER 에서 ModernChic 선택, 결정·플레이·결과 | 문서가 있는 화면은 ModernChic, 9키는 내장 필드, 키 설정을 열면 `keyconfig.luaskin` 실패 알림이 한 번 |
| 3.5 E2 | 되돌리기 | PACK FOLDER 행에서 왼쪽 키로 비움, `RBMS_SKIN_PACK=<폴더>` 로도 실행 | `DEFAULT (<경로>)`, 모든 화면이 기본 스킨. 환경 변수로 쓰면 `(RBMS_SKIN_PACK)` 가 붙고 설정 파일은 불변 |
| 3.5 E3 | 동영상이 있는 외부 스킨 | ModernChic 의 결정·선곡·플레이를 수 분 켜 둠 | 영상이 반복 재생되고 끊기지 않으며 느려지거나 메모리가 계속 늘지 않음 |
| 3.5 E4 | 스킨 옵션과 오프셋 | SKIN 메뉴 7K 에서 옵션·오프셋을 바꾸고 Esc, 플레이, 오프셋 행에서 0 | 옵션이 미리보기와 실제 플레이에 같게 반영, 오프셋 즉시 반영, 0 키가 0 으로, RESET 이 모든 선택을 버림 |
| 3.5 E5 | 로드 실패 알림 | 문법 오류(`return {{{`) `.luaskin` 팩을 PACK FOLDER 로 지정 | 미리보기에 `NO SKIN DOCUMENT FOR THIS SCREEN` 과 `DID NOT READ (header pass)`, Esc 후 `skin <파일> did not load…` 토스트 한 번과 내장 화면 |
| 3.6 F1 | 소리 | 선곡 이동, 결정, 패널·사이드 메뉴 여닫기, 결과 효과음, SOUND FOLDER 비움 | 제때 나고 키음·BGM 이 끊기지 않음, 비우면 설치본 `sound/` 사용 |
| 3.6 F2 | 내장 화면의 밝기(E10) | SETTINGS, KEY CONFIG, TABLES, FOLDERS, PRACTICE, 로딩 화면 | 모두 읽을 수 있음(어색하면 화면과 느낌을 기록) |
| 3.6 F3 | 설정 행 표지 | 기본 스킨 상태에서 PLAY·DISPLAY·INPUT 탭의 표지 행 | `BUILT-IN SCREENS ONLY` / `SKIN OPTION COMES FIRST` 표지와 흐린 값, 값은 바뀜, 내장만 쓰는 구성에선 표지 없음 |
| 3.6 F4 | 오버레이 | 선곡 위에서 H, F1, I, 토스트 | 뜨고 닫히며 스킨 위에서 읽힘(알려진 한계 3가지는 3.6절) |
| 3.7 G1 | SKIN 메뉴 들어가고 나가기 | 선곡 F12, 사이드 메뉴 SKIN 행, SETTINGS 의 F12 와 `SKIN MENU >`, Esc | 네 길 모두 열리고 Esc 는 왔던 화면(SETTINGS 는 보던 탭), 복귀 순간 검은 화면이나 여는 연출 반복 없음 |
| 3.7 G2 | 미리보기가 움직임 | 화면 띠를 Tab·클릭으로 7K 5K 14K 10K 9K SELECT DECIDE RESULT COURSE | 16:9 사각형에 스킨이 예시 데이터로 움직임, 제목 `SAMPLE …`, 소리 없음, 반복 때 `LOADING THE SKIN` 번쩍임 없음 |
| 3.7 G3 | 값이 바로 반영 | 7K 의 Lane Width·Field Side·Judge Detail·Score Graph, 오프셋 행(←→, Shift, 0, 오른쪽 클릭) | 옵션은 잠깐 뒤 이어서 바뀌고 오프셋은 즉시, Shift 열 칸, 오른쪽 클릭 한 칸 뒤, UI 멈춤 없음 |
| 3.7 G4 | 목록과 마우스 | ModernChic 7K(행 89개)를 ↑↓, PageUp/Down, 휠(목록 위·미리보기 위)로 | 스크롤 막대와 `현재 / 전체`, 커서 행이 항상 보임, 휠은 목록 위에서만, 일본어 옵션명이 시스템 글꼴로 |
| 3.7 G5 | 폴더 선택 창 | PACK FOLDER 에서 Enter, 창이 떠 있는 동안 키·클릭, 폴더 선택 또는 취소 | 창이 떠 있는 동안 메뉴가 그려지되 반응 없음, 고르면 경로(앞을 `…` 로 줄임)와 미리보기 갱신, 취소는 무변화 |
| 3.7 G6 | 없는 문서와 실패 | ModernChic 9K·KEYS·SCREEN 으로 PLAY 24KEYS, 없는 폴더를 `skin.pack` 에, `error("x")` 를 내는 `.luaskin` 팩 | 차례로 `NO SKIN DOCUMENT…`, `NO PREVIEW FOR THIS SCREEN`, `THE PACK FOLDER IS NOT THERE` + `(MISSING)`, `THE SKIN DID NOT LOAD`. 겹침·멈춤 없음 |
| 3.7 G7 | 창 크기 | 1920x1080, BORDERLESS, 4:3 창(LETTERBOX 켬/끔)에서 메뉴 | 미리보기가 테두리 안에 꼭 맞음, LETTERBOX 끔이면 메뉴 전체와 미리보기가 같이 늘어남 |
| 3.7 G8 | 동영상과 자원 | ModernChic 7K·SELECT 를 미리보고 화면 종류를 여러 번 바꾼 뒤 나감, DEBUG MODE 패널 | 동영상이 사각형 안에서 재생, `SKIN TEX` 가 화면 종류를 바꿀 때마다 한 화면분으로 돌아오고 계속 늘지 않음 |
| 3.8 H1 | 넓은 레인 | F12 에서 7K Lane Width Wide 로 BGA 있는 곡 플레이. 5K·9K·10K·14K, Score Graph Off, Field Side Right 도 | 레인·노트·판정선·커버·판정 글자·게이지가 폭을 따라가고 BGA·점수·판정 수·타이밍·그래프가 안 겹침, Normal 로 되돌리면 이전과 같음 |
| 3.8 H2 | 키 설정 화면 | 선곡 F5, Settings F5, `KEY CONFIG >`, 묶음마다 Enter 로 바인딩, 중복 키, Delete·Backspace, 전체 복원, 컨트롤러 버튼·턴테이블 축 | `PRESS...` 대기 뒤 키가 뜸, 같은 화면의 충돌은 거절되고 빨간 문구, 나갔다 다시 열거나 재실행해도 유지 |
| 3.8 H3 | 옮긴 키가 동작 | 검색(/)·정렬(F3)·스킨 메뉴(F12)·다시 하기(R)를 다른 키로 옮기고 각 화면에서 누름 | 옛 키는 무반응, 새 키는 동작, H 가이드가 새 키를 보임, 검색어 입력 중엔 글자 키가 검색어로 |
| 3.8 H4 | 레인 커버 조작 | 플레이 중 →←, `\`, START 두 번, START + 턴테이블, 휠, Shift + →← | → 가 더 가림 / ← 가 덜 가림, 흰·녹색 숫자가 따라감, 꺼진 상태에서 COVER 수치가 잠깐, Shift 는 더 잘게 |
| 3.8 H5 | 판정 자동 보정 | F8 > JUDGE 에서 AUTO ADJUST 켜고 일부러 빠르게(또는 느리게) 침. 완주·Esc·폐점, 리플레이 재생 | `TIMING AUTO`, 빠르면 -, 늦으면 +, 세 경우 모두 JUDGE OFFSET 에 마지막 값, 리플레이 판정 수 일치, 오토플레이·리플레이에선 안 움직임 |
| 3.8 H6 | 녹색 숫자 유지 | F8 > SPEED 에서 KEEP GREEN NUMBER, BPM 이 다른 곡 둘을 커버·리프트 켠 채. ↑↓, SELECT + 건반, 패널 3 의 4·6번 | 두 곡 모두 발치 GREEN 이 정한 숫자, 커버를 움직여도 GREEN 불변 / HI-SPEED 변동, ↑↓ 는 10(Shift 1), SELECT + 건반은 1 ms, 꺼져 있으면 패널 3 이 `OFF` |
| 3.8 H7 | 이전 설정 파일 | 이 판 이전의 `settings.ron`, `keyconfig.ron`, 리플레이로 실행 | 키·설정이 이전과 같고 KEEP GREEN NUMBER 꺼짐, 이전 리플레이가 같은 결과로 재생 |
| 3.9 I1 | PLAY 메뉴 들어가고 나가기 | 선곡 F8, SETTINGS 에서 F8 과 `PLAY MENU >`, 검색 중 F8, Esc | 세 길 모두 열리고 Esc 는 왔던 화면, SETTINGS 탭 6개와 세 단추가 안 겹침, 복귀 때 검은 화면 없음 |
| 3.9 I2 | 녹색 숫자 유지를 눈으로 | KEEP GREEN NUMBER 켠/끈 채 PREVIEW BPM 을 100 → 150 → 200 | 켠 동안 낙하 시간·GREEN 고정 / HI-SPEED 만 바뀜(`AUTO`), 끄면 반대, BPM 을 바꿀 때 검은 화면·READY 없음 |
| 3.9 I3 | 가림막과 숫자 | LANE COVER 를 →·Shift+→, LIFT·HIDDEN+ 도 같게, 0 키 | 정보 패널·요약·미리보기의 숫자가 모두 같은 값(`30.0% (WHITE 300)`), 리프트 15% 는 150, 0 키는 기본값 |
| 3.9 I4 | 나머지 행 | SPEED FIX CONSTANT, GAUGE, TARGET, AUTOPLAY, JUDGE OFFSET, AUTO ADJUST, LEGACY NOTE 를 바꿈 | CONSTANT 는 BPM 칸 `120 (CONSTANT)`, GAUGE 색, TARGET 막대·`TARGET +n`, AUTOPLAY 표지, JUDGE OFFSET 은 1 ms(Shift 10 ms)씩이고 예시 판정은 그대로 |
| 3.9 I5 | 모드와 스킨 옵션 | Tab·모드 띠 클릭으로 7K 5K 14K 10K 9K, LANE WIDTH 를 WIDE 로 하고 F12 의 같은 행, ModernChic 팩 | 모드마다 그 키 수 화면, LANE WIDTH 가 두 메뉴에서 같고 실제 플레이에도 같음, 문서 없는 모드는 `NO SKIN DOCUMENT…`, 다시 열면 7K·150 |
| 3.9 I6 | 마우스와 긴 목록 | 행 좌우 클릭, 목록 위 휠, 미리보기·묶음 이름 클릭, PageUp/Down, 창 4:3·21:9·1920x1080 | 왼쪽 클릭 한 칸 앞 / 오른쪽 뒤, 휠은 목록만, 미리보기·묶음 이름 무반응, 글자가 안 잘리고 16:9 미리보기 |
| 3.9 I7 | 실제 플레이와 같은가 | PLAY 메뉴에서 유지 켬·커버 25%·리프트 20%·녹색 350 을 정하고 BPM 100 곡 플레이 | HI-SPEED·GREEN·커버·리프트 숫자가 미리보기와 같고 F1 오버레이·패널 1~3 에도 같음 |
| 3.9 I8 | 이전 키 설정 | `keyconfig.ron` 에서 FOLDERS 를 F8 로 옮겨 둔 채 실행 | F8 은 옮긴 단축키, PLAY MENU 는 키 없음(단추로는 열림), SETTINGS 안내 줄 `- PLAY`, 다른 키를 주면 그 키로 열림 |
| 3.10 J1 | 판정이 가만히 있는가 | 7키 곡을 손으로(오토플레이로도 한 번) | 판정 글자·콤보가 튀거나 커지거나 깜빡이지 않고 같은 자리, 0.5초 뒤 한 번에 사라짐, PGREAT 하늘색 / GREAT 노랑, 콤보는 글자 바로 뒤(`GREAT 67`), 글자 크기 60 이 작은지 기록 |
| 3.10 J2 | 판정 위 줄 | 일부러 빠르게·늦게, F8 의 TARGET, SCORE DIFFERENCE(BEST/OFF), JUDGE DETAIL(MILLISECONDS/OFF) | 판정 위 왼쪽 `+0003` 점수 차 / 오른쪽 FAST(파랑)·SLOW(빨강), 점수 차는 스코어 그래프의 TARGET 과 같음, 하나를 끄면 남은 쪽이 왼쪽 끝에서 시작 |
| 3.10 J3 | 판정 위치와 숨김 | F8 의 JUDGE POSITION Y(→, Shift+→, 음수까지), JUDGE DISPLAY HIDE, F12 의 같은 행, 5·9·14키 | + 면 위로 - 면 아래로, 두 메뉴가 같은 값, HIDE 면 판정 줄 없음, 0 키는 제자리 |
| 3.10 J4 | 노트와 판정만 남기기 | F12 에서 7K 불투명도 행(Lane Background, Lane Divider, Bar Line, Progress Bar, Score Graph, Song Header, Speed Display, Timing)을 0% 로 두고 Esc, 플레이 | 검은 바탕에 노트·키 빔·폭발·판정선·판정 글자(와 남긴 BGA·점수·판정 수·게이지)만, 잔상 없음, 재실행해도 유지, 0 키·RESET 으로 복귀 |
| 3.10 J5 | 중간값 | Note, Key Beam, Lane Cover, BGA 를 50%·25%, Enter 후 `150` | 눈에 절반·4분의 1 쯤, 빔이 안 번쩍임, → 와 슬라이더는 100% 에서 멈춤, 직접 입력 `150% (+128)`, 0 키로 100% |
| 3.10 J6 | PLAY 메뉴의 접이 | SKIN 묶음의 OPACITY 와 POSITION AND SIZE 를 Enter·클릭으로 폄 | 펴는 순간 든 행이 화면에 들어옴, `SHOWN (18)`(14K 는 17), SKIN 메뉴의 같은 행과 같은 값 |
| 3.10 J7 | 외부 팩 | ModernChic 을 팩으로 고르고 SKIN 메뉴 7K 목록 끝, 플레이 | `明るさ` 다섯 행이 `100%`, → 와 슬라이더로는 안 올라가고 `150` 입력 시 `150% (+128)` 로 덮개가 밝아짐, ModernChic 판정 연출은 전과 같음 |
| 3.11 K1 | 슬라이더 끌기 | PLAY 메뉴 LANE COVER 트랙을 누르고 끌기·행 밖으로 끌기, 손잡이만 잡았다 떼기. SKIN 메뉴 불투명도, SETTINGS 의 BGM VOL·POLYPHONY 도 | 트랙을 누르면 손잡이가 그 자리로 와서 따라옴, 행 밖에서도 이어지고 끝값에 머묾, 잡기만 하면 값 불변, 미리보기 끊김 없음, 볼륨은 소리가 바로 바뀜 |
| 3.11 K2 | 끌기의 끝 | BGM VOL 을 끄는 중 창 밖에서 뗌, 초점을 옮겼다 돌아옴, POLYPHONY 를 끄는 중 Esc | 창 밖에서 떼어도 끌기가 끝남, 못 본 경우 다음 클릭·키에서 끝나 그때 값이 저장됨, Esc 후 재진입하면 끌던 값 / 오디오가 그 값으로 다시 열림 |
| 3.11 K3 | 직접 입력 | 숫자 행에서 Enter 또는 글자 클릭, `42%`·`-5`·`999`·빈 칸·`1.2.3`, 입력 중 Esc·↑↓·Tab·다른 행 클릭, 한글 입력기·넘패드 | 현재 값이 선택된 채 채워짐, 단위 무시·범위 밖은 끝값, 숫자가 아니면 `NOT A NUMBER` 1.5초, Esc 는 입력만 취소, ↑↓·Tab·클릭은 확정 뒤 이동, 입력기가 켜져도 숫자가 들어감 |
| 3.11 K4 | 가장 잔 단위와 Shift | HI-SPEED, LANE COVER, JUDGE OFFSET, 불투명도, 볼륨, JUDGE WIDTH, PREVIEW FADE, TOTAL 에서 →·Shift+→·0·오른쪽 클릭·키 누르고 있기 | 한 칸이 0.01 / 0.1% / 1 ms / 1% / 1% / 1% / 1 ms / 1, Shift 는 열 배, 0 은 기본값, 오른쪽 클릭은 한 칸 뒤, 누르고 있어도 반복 안 됨 |
| 3.11 K5 | 읽기 전용 행과 휠 | KEEP GREEN NUMBER 를 켠 채 HI-SPEED 행 클릭·끌기·Enter, 슬라이더 위 휠 | HI-SPEED 행은 흐리고 어떤 조작에도 안 움직이며 입력 상자가 안 열림, 휠은 목록만 스크롤 |
| 3.11 K6 | 표가 저장되는가 | Tables(T)에서 `+ ADD TABLE (URL)` 로 `https://darksabun.club/table/archive/insane1/` 등록, 앱을 끄고 Wi-Fi 를 끈 채 재실행 | 등록해도 LOADING 으로 안 넘어가고 `FETCHING...`, 도착하면 `~/.config/rbms/tables/` 에 `.json` 하나, 오프라인에서도 MATCHING TABLES 가 1초 안에 지나가고 `SAVED JUST NOW` 가 보임 |
| 3.11 K7 | 다시 받기 | 네트워크를 켠 채 Tables 에서 R(여럿이면 A), 선곡에서 F6, 받는 중 조작 | 화면이 안 멈추고 흰 `RELOADING...` 과 `FETCHING THE TABLE AGAIN …`, 끝나면 `difficulty tables reloaded: N updated, M failed` 와 `SAVED JUST NOW`, 받는 중 재요청은 `the tables are being fetched already` 만 |
| 3.11 K8 | 다시 받기 실패와 삭제 | 네트워크를 끊고 R, 표를 D 로 지움, R 직후 D | `RELOAD FAILED - …` 와 사유(폴더·레벨·`SAVED …` 는 그대로), 지운 표의 `.json` 이 사라지고 받는 중 지운 표는 되살아나지 않음 |
| 3.11 K9 | 오래된 저장본 | `tables/` 저장본의 `"fetched_at"` 을 40일 전으로 고치고 실행 | `SAVED 40 DAYS AGO` 경고색, 커서를 올리면 `THE SAVED COPY IS 40 DAYS OLD - R FETCHES THE TABLE AGAIN`, 스스로 다시 받지 않음 |
| 3.12 L1 | 응답 없는 표 주소로 시작 | Tables 에서 `https://10.255.255.1/table/` 등록 후 재실행 | MATCHING TABLES 가 바로 지나가 선곡이 뜸, 직후 `fetching 1 difficulty table(s) for the first time`, 약 15초 뒤 `table fetch failed (…)` 한 줄, 재스캔해도 다시 받지 않음, 행은 `LOAD FAILED`, R 로 재시도 |
| 3.12 L2 | 표 폴더가 뒤늦게 나타남 | 표 하나의 `.json` 만 지우고 켜서 폴더 / 레벨 폴더 / ALL SONGS / 검색 중 상태에서 기다림 | 선곡이 기다리지 않고 뜨고 `difficulty tables fetched for the first time: 1 added, 0 failed` 와 함께 폴더가 등록 순서 자리에 나타나며 커서는 보던 폴더에 그대로, 목록이 안 깜빡이고 미리듣기가 안 끊김 |
| 3.12 L3 | 새 등록 | Tables 에서 `+ ADD TABLE (URL)`·`(FILE)`, 틀린 주소, 받는 중 D | LOADING 으로 안 넘어가고 행이 바로 생김, 도착하면 `N OF M CHARTS OWNED - K LEVELS` 와 `table added: …`, 틀린 주소는 `LOAD FAILED`, 받는 중 지우면 파일이 안 생김 |
| 3.12 L4 | 받는 중 종료 | L1 상태에서 15초 전에 창을 닫음 | 창이 바로 닫히고 `tables/` 에 반쯤 쓴 파일이나 임시 파일이 안 남음 |
| 3.12 L5 | 연습 패널의 슬라이더 | 곡에서 F4, START TIME·END TIME·GAUGE VALUE·JUDGERANK·TOTAL·FREQUENCY 슬라이더를 누르기·끌기·행 밖과 창 밖으로, 전체 화면과 16:9 가 아닌 창 | 손잡이가 누른 자리로 오고 따라옴(레터박스에서도 어긋나지 않음), 끄는 행은 트랙 노랑 + 손잡이 흰 테두리, 잡기만 하면 값 불변, END TIME 은 START TIME + 1초에서 멈춤, TOTAL 슬라이더 끝은 1000 |
| 3.12 L6 | START TIME 을 END TIME 너머로 | START 0:20.0 / END 0:40.0 에서 START 손잡이를 END 너머로 끌었다 돌아옴, Shift+→ 로 0:45.0 까지 올렸다 내림 | 끄는 동안 END 가 START + 1초로 밀렸다가 START 가 0:39.0 아래로 돌아오면 0:40.0 으로 복귀, 키로 올렸다 내리면 END 가 0:46.0 에 남음 |
| 3.12 L7 | 연습 패널의 직접 입력 | START TIME 에 `90`·`1:30.5`·`2:5`·`1:75`, TOTAL 에 `3000`·`99999`·빈 칸·`-`, 입력 중 Esc·↑↓·Tab·Shift+Tab·클릭, 한글 입력기 | `90` → 1:30.0, `2:5` → 2:05.0, `1:75` → 2:15.0, TOTAL 3000 은 3000 / 99999 는 5000, 숫자가 아니면 `NOT A NUMBER`, Esc 는 입력만 취소(두 번째 Esc 가 선곡으로) |
| 3.12 L8 | 연습 패널의 Enter 와 START 행 | 패널을 연 직후 Enter, START TIME·GAUGE TYPE 에서 Enter, START 행 좌우 클릭, 구간 뒤 재진입 | 커서가 맨 위 START 에서 Enter 가 구간 시작, 숫자 행 Enter 는 입력 상자, 고르기 행 Enter 는 시작, 값이 `practice.ron` 에 남고 구간 플레이가 패널 값과 같음 |
| 3.13 M1 | 판정선이 필드 맨 아래 | 기본 스킨 7K(5K·9K·10K·14K, Wide·Field Right·Scratch Side Right), 창 1280x720 / 1920x1080 / 전체 / 비16:9 | 판정선(굵은 하늘색 띠)이 레인 맨 아래, 아래에 건반 표시가 없고 바로 게이지, 레인 색 줄이 안 남음, 노트가 판정선에서 사라짐 |
| 3.13 M2 | LIFT 와 커버 | F8 > LANE 에서 LIFT 20%, LANE COVER 30%, HIDDEN+ 20% 로 플레이, `]` `[` 로 리프트 | 판정선이 리프트만큼 올라가고 그 아래는 리프트 면이 덮음(레인 안 비침), 리프트·히든·커버 수치와 녹색 숫자가 제 자리, 판정 글자·키 빔·폭발이 판정선을 따라 올라감 |
| 3.13 M3 | 폭발의 모양 | 플레이 중 판정선 위 폭발 고리와 LN 누르는 동안의 폭발 | 판정선에 중심을 둔 온전한 원이고 아래 절반이 게이지 위에 비침(밀집 구간에서 게이지가 읽기 어려우면 기록) |
| 3.13 M4 | 오프셋을 준 플레이의 소리·노트·판정 | JUDGE OFFSET 0 / +30 / -30 으로 한 판씩, +100·-100 은 오토플레이로 | 박자에 친 입력은 어느 값에서도 PGREAT 이고 키음이 곡과 어긋나지 않음, +30 은 노트가 먼저 / -30 은 늦게, TIMING 숫자가 `+30`·`-30` |
| 3.13 M5 | 자기 손에 맞는 값 찾기 | 오프셋 0·AUTO ADJUST 끔으로 한 판 뒤 FAST·SLOW 수 확인, 키 설정(F5)의 `JUDGE OFFSET +1 MS`·`-1 MS` 키, 패널 3 의 5·7번 | 늦게 치면 값을 올려 FAST·SLOW 가 고르게, 패널 3 의 5번 `NOTES 1 MS LATER` / 7번 `NOTES 1 MS EARLIER`, 세 곳이 같은 방향을 말함 |
| 3.13 M6 | 자동 보정 | AUTO ADJUST 를 켜고 보며 빠르게(다른 판은 늦게), 한 판은 소리만 듣고 침 | `TIMING AUTO`, 빠르면 숫자가 내려가고 늦으면 올라감, 보고 치면 몇십 타 안에 멈춤, 소리만 듣고 치면 한쪽으로 계속 감, 오토플레이·리플레이에선 불변 |
| 3.13 M7 | 이전 설정과 이전 리플레이 | 이 판 전에 JUDGE OFFSET +26 으로 저장한 `settings.ron` 과 리플레이로 실행(계정 동기화 쓰면 다른 기기 설정도) | 처음 켰을 때 `-26` 으로 부호만 뒤집혀 보이고 치는 느낌은 같음, `settings.ron.v4.bak` 생성, 다시 켜도 안 뒤집힘, 이전 리플레이가 같은 판정 수로 재생, 리플레이를 보고 나와도 내 JUDGE OFFSET 불변 |
| 3.13 M8 | BGA 와 오프셋 | BGA 가 박자에 맞춰 넘어가는 곡을 JUDGE OFFSET 0 / +200 / -200 으로 오토플레이 | BGA 가 넘어가는 순간이 세 값에서 모두 소리에 맞음, 노트만 앞서거나 늦음, 미스 레이어도 놓친 순간에 뜸 |
| 3.13 M9 | PLAY 메뉴의 미리보기 | F8 > JUDGE 의 JUDGE OFFSET 을 Shift+→·← 로 ±100 까지 | 미리보기 노트 전체가 값만큼 아래(+)·위(-)로 옮겨지고 TIMING 숫자가 따라옴, 예시 판정은 그대로 |
| 3.14 N1 | START + 턴테이블로 레인 커버 1 씩 | 7K 를 손으로 치며 커버를 켠 채 `A` 를 누르고 왼쪽 Shift·Ctrl 을 짧게 여러 번, Shift 3초 | Shift 한 번에 흰 숫자 +1 / Ctrl 한 번에 -1, 녹색 숫자 불변 / HI-SPEED 가 따라감, 누르고 있으면 0.3초 뒤부터 점점 빨라짐, `A` 를 떼면 약 1.5초 뒤 COVER·LIFT 수치가 사라짐 |
| 3.14 N2 | START + SELECT + 턴테이블로 리프트 1 씩 | `A`+`W` 를 누른 채 왼쪽 Ctrl(올림)·Shift(내림), 리프트 꺼짐·켜짐, 3초 이상 | 꺼져 있어도 LIFT 수치가 1 씩 바뀜(켜지진 않음), 켜져 있으면 판정선이 1 씩, 커버·녹색 숫자·HI-SPEED 불변, `W` 만 떼도 커버가 안 꺼짐, `A` 두 번은 전처럼 커버 토글 |
| 3.14 N3 | 실제 턴테이블(컨트롤러) | 아날로그 턴테이블 컨트롤러에서 START 를 누른 채 디스크를 천천히·빠르게, START + SELECT 도 | 틱마다 1 씩 움직이고 방향이 디스크와 맞음, START 전에 돌리던 것은 반영 안 됨 |
| 3.14 N4 | 판정 오프셋 키 | 플레이 중 `0` 3번, `9` 3번, `0` 2초 누르기, 선곡에서 `0`(검색)·`9` | TIMING 이 +1 / -1 되고 노트 전체가 옮겨짐, 누르고 있으면 0.3초 뒤 초당 20 씩 ±200 에서 멈춤, 오토플레이·리플레이에선 불변, 선곡의 `0`·`9` 는 전과 같음 |
| 3.14 N5 | 기존 키 설정 파일 | 이 판 이전의 `keyconfig.ron` 으로 실행해 선곡 `H`·`F5`, SCRATCH 8 REVERSE 를 Del 로 떼고 재실행 | 가이드에 `9 0  JUDGE OFFSET -1 / +1 MS (HOLD)` 와 `THE TURNTABLE: DOWN / UP`, 이미 다른 데 쓰던 키는 `-`, 뗀 키는 다시 켜도 `-`, 다른 키는 그대로 |
| 3.14 N6 | 왼쪽 Ctrl 이 선곡에서 목록을 올림 | 기본 스킨 선곡에서 왼쪽 Ctrl, `/` 검색 중 Ctrl+V, Settings 글자 칸 Ctrl+V | 선곡에선 왼쪽 Shift 가 아래 / 왼쪽 Ctrl 이 위로 한 칸, 검색어·글자 칸에선 붙여넣기만 되고 목록이 안 움직임 |
| 3.14 N7 | 모노톤 화면 | 설정을 비운 첫 실행으로 선곡 → 결정 → 플레이 → 결과, 밝기가 다른 모니터 둘과 밝고 어두운 방 | 바탕이 순수 검정이고 파란 기가 없음, 면 단계가 구분되고 작은 회색 글자가 읽힘, 노트·판정 색은 전과 같음, 선택한 곡이 흰 표시로 가려짐 |
| 3.14 N8 | 색 옵션과 모든 화면에 적용 | F12 7K 에서 THEME 을 SLATE 로 + `A`, ACCENT 를 AMBER 로 + `A`, NOTE COLORS WHITE-YELLOW, BOMB COLOR WHITE 각각 `A`, Tab 으로 다른 화면, 실제 선곡 → 플레이 → 결과, F8 LANE 묶음에서도 | 행에 `ALSO ON 8 OTHER SCREENS - A APPLIES THIS VALUE TO THEM ALL`, `A` 뒤 `APPLIED TO 8 OTHER SCREENS`(NOTE COLORS 3, BOMB COLOR 4), 모든 미리보기와 실제 화면이 고른 색, THEME SLATE + ACCENT SKY 는 웨이브 13 까지의 화면과 같음 |
| 3.14 N10 | 내보내기 | N8 상태에서 불투명도·JUDGE POSITION Y 도 바꾼 뒤 SKIN 메뉴 맨 아래 EXPORT, 폴더 선택, 같은 폴더로 한 번 더, 팩 폴더 자신, ModernChic 내보내다 Esc | 창 제목 `Select folder to export the skin into`, 창이 떠 있는 동안 메뉴가 안 멈춤, `rbms-default-export`(두 번째는 `-export-2`) 생성과 `EXPORTED 66 FILES (2 MB) TO <경로>`, 팩 폴더 안을 고르면 `EXPORT FAILED`, 진행 중 Esc 는 취소하고 반쯤 쓴 폴더가 안 남음 |
| 3.14 N11 | 내보낸 팩을 새 설정에서 열기 | `HOME=$(mktemp -d)` 새 설정에서 PACK FOLDER 로 N10 폴더를 고르고 선곡 → 플레이 → 결과, 행 변경·0·RESET, 다른 OS 로 옮겨서도 | 모든 화면이 N8·N10 에서 맞춘 그대로, 메뉴 행이 같은 값, 0·RESET 은 내보낸 값으로 돌아감, `rbms-adjustments.ron` 에 사용자 이름이나 절대 경로가 없음 |
| 3.14 N12 | 깨진 조절값 파일 | N10 폴더의 `rbms-adjustments.ron` 에서 괄호 하나를 지우고 그 폴더를 팩으로 | 알림이 한 번 뜨고 제작자 기본값(모노톤)으로 그려지며 앱이 안 죽음 |
| 3.15 O1 | 메뉴와 스킨이 한 벌로 보이는가 | 선곡에서 F12, F8, F5, Tab, T, O 를 차례로, 밝기가 다른 모니터 둘, 어둡고 밝은 방 | 바탕이 순수 검정이고 파란·노란 기가 없음, 메뉴의 면이 스킨의 면과 같은 회색, 넘어갈 때 색온도가 안 바뀌어 보임 |
| 3.15 O2 | 고른 것이 밝기만으로 가려지는가 | 각 메뉴에서 커서·탭 이동, 슬라이더 끌기, 숫자 입력, 키 대기 | 커서 행은 한 단계 밝은 면 + 흰 글자, 고른 탭은 밝은 면, 슬라이더 손잡이 흰색(끄는 동안 커짐), 숫자 칸 흰 테두리·캐럿, 키 대기 칸은 흰 면에 검은 `PRESS...`, 읽기 전용 행은 흐림 |
| 3.15 O3 | 색이 남은 곳이 뜻이 있는 곳뿐인가 | Tables 의 틀린 주소·오래된 저장본, GREEN NUMBER 1, 키 충돌, 알림, `--skin assets/skins/normal.ron` 으로 내장 화면 | 색은 실패·오래됨·한계(호박), 키 충돌(빨강), 알림 경고(주황)·오류(빨강), GREEN 숫자, 판정 색, FAST/SLOW, 램프·난이도·모드 배지, 랭크, 점수 차, 그래프, 게이지, 서버 점에만 있음 |
| 3.15 O4 | 기존 설정 폴더의 `theme.ron` | 처음 실행 전에 `~/.config/rbms/theme.ron` 을 복사해 두고 실행해 `theme:` 줄과 파일을 본 뒤 한 번 더 실행 | 안 고친 파일이면 첫 실행 줄 끝에 `(the template of an earlier version was replaced; the colours follow the defaults)` 가 붙고 새 템플릿(모든 색 줄이 `//`), 두 번째에는 문구·변경 없음, 한 값이라도 고친 파일이면 문구 없이 그대로 |
| 3.15 O5 | 내장 플레이 화면의 레인 | 스킨 팩 없이 7K 곡을 플레이(NORMAL 과 WIDE) | 노트·빔·폭발·판정선은 전과 같고 점수 그래프 눈금선·게이지 빈 칸·레인 바탕·필드 테두리·구분선이 회색(22 / 96 / 60), 파란 기 없음 |
| 3.16 W1 | 내 설정에 SCRATCH AUTO 가 켜져 있는지 | F8 > CHART 의 SCRATCH AUTO 행에 커서 | 정보 패널에 `WHILE ON: TURNTABLE NOT COUNTED, LIGHT ASSIST LAMP`. 켜 둘지는 취향 |
| 3.16 W2 | 리플레이를 본 뒤에도 설정이 그대로인가 | SCRATCH AUTO 를 끈 채 예전 SCRATCH AUTO 판의 리플레이를 끝까지 보고 Esc, F8, 앱 재시작 | 재생 중 턴테이블이 자동으로 쳐지고, 돌아온 뒤 SCRATCH AUTO·GAUGE·RANDOM 이 재생 전 값이며 재시작 뒤에도 같음 |
| 3.16 W3 | 로그인 없이 한 판을 친 뒤 | LOGOUT(또는 SERVER URL 비움)·SCRATCH AUTO 끔으로 7K 곡을 끝까지 침 | 결과에 `SCRATCH AUTO:` 줄이 없고 `IR: SKIPPED (not logged in)` 와 실패 알림 없음, 선곡의 RANK·EX SCORE·RATE·MAX COMBO·MISS COUNT·판정 막대가 채워지고 PLAY COUNT +1, 재시작해도 같음 |
| 3.16 W4 | 어시스트로만 친 차트의 선곡 | SCRATCH AUTO 로만 쳤던 차트에 커서 | CLEAR 는 LIGHT ASSIST CLEAR, PLAY COUNT·CLEAR COUNT 는 숫자, RANK·EX SCORE·RATE·MAX COMBO·MISS COUNT·판정 수는 `-` |
| 3.16 W5 | 다음 판의 LOCAL BEST 목표 | TARGET 을 LOCAL BEST 로 두고 W3 의 곡을 다시, 이어서 한 번도 손으로 치지 않은 곡을 침 | W3 곡은 범례 `LOCAL BEST`, 판정 위 숫자는 W3 EX 와의 차이, 결과 TARGET 도 같은 EX. 안 친 곡은 범례 `RATE AAA (NO RECORD)`, 패널 1 `LOCAL BEST (NO RECORD: AAA)` |
| 3.16 W6 | TARGET FALLBACK 과 TARGET RATE | F8 > CHART 에서 TARGET FALLBACK 을 MAX / RATE A / RATE CUSTOM, TARGET RATE 70%, TARGET 을 IR BEST, TARGET RATE AA + FALLBACK RATE AAA 에 TARGET RATE 커서, 안 친 곡 플레이 | TARGET 행이 `LOCAL BEST > MAX`·`> A`·`> 70%`, 범례가 따라감, IR BEST 는 `IR BEST > LOCAL > AAA` 와 `NO IR SCORE: YOUR LOCAL BEST, THEN RATE AAA`, 마지막은 `NOT IN USE: NO TARGET IS RATE CUSTOM`, 실제 판의 범례·숫자·결과 TARGET 이 고른 fallback |
| 3.16 W7 | 설정 화면(Tab)을 손으로 | Tab·Shift+Tab 묶음, PgUp/PgDn/Home/End, 휠, 라벨·스위치·칸·화살표 클릭, 오른쪽 클릭, 숫자·EMAIL·PASSWORD 입력 중, RIVALS 목록, Shift+0 확인 중의 오른쪽 패널. 1280x720 과 전체 화면 | 모든 묶음이 스크롤 없이 한 화면이고 잘린 글자 없음, 각 모드 중 패널 KEYS 가 그 모드의 키(`DIGITS TYPE A NUMBER`, `KEYS TYPE`, `D REMOVE`, `ENTER RESET THE GROUP`)이고 VALUE 가 치는 값(비밀번호는 가려짐), 바닥 안내와 패널이 같은 말 |
| 3.17 X1 | 초록 턴테이블(GPU) | SCRATCH AUTO 를 켜고 턴테이블 노트·롱노트·지뢰가 있는 7K 를 기본 팩으로, 이어 ModernChic 으로, 14K 도 한 번 | 턴테이블 노트와 롱노트 시작·끝이 단색 초록(#30E050), 롱노트 몸통은 반투명 초록이고 판정선에서 사라짐, 지뢰는 안 보임, 키를 눌러도 빔·폭발·공 POOR·소리 없음, 판정 수·콤보·게이지가 턴테이블 노트에 안 움직임 |
| 3.17 X2 | 턴테이블 키음 | X1 의 곡을 소리 장치로 듣고 KEY 음량을 내렸다 올림 | 턴테이블 노트의 키음이 제 박자에 한 번씩 울리고 KEY 음량을 따름 |
| 3.17 X3 | SCRATCH AUTO 판의 기록과 전송 | 로그인한 채 X1 의 곡을 끝까지 치고 결과 화면·선곡의 점수·기록 목록(R)의 리플레이·웹(bms.hyuns.uk)을 봄 | 결과 오른쪽 위 `SCRATCH AUTO: TURNTABLE NOT COUNTED, LIGHT ASSIST LAMP`, 램프 LIGHT ASSIST CLEAR, 선곡의 EX·RATE·RANK 가 채워짐, 리플레이가 같은 결과로 재생, 서버에 SCRATCH AUTO 표시와 함께 판이 있음(순위 노출은 보이는 대로 기록) |
| 3.17 X4 | 파란 건반 + START(실제 컨트롤러) | RANDOM 으로 한 판 친 뒤 결과 화면에서 2번(또는 4·6번) 건반을 누른 채 START, 5K·9K·14K 도, 파란 건반만 눌렀다 떼기, START 만 | 같은 곡이 같은 노트 배치로 바로 다시 시작, 7K 에서 2·4번만 눌렀다 떼면 선곡으로(뗄 때), 6번은 그래프만 바꿈, START 만으로는 무반응 |
| 3.17 X5 | 설정 화면의 Enter | Tab 으로 열고 스위치 / 선택 / SOUND FOLDER / 숫자 / KEY CONFIG 행에서 차례로 Enter | 스위치는 뒤집힘, 선택은 다음 값, SOUND FOLDER 는 OS 폴더 고르기 창, 숫자는 직접 입력, KEY CONFIG 는 키 설정. 어느 Enter 도 화면을 안 닫고 Esc 만 저장하고 닫음. 바닥 안내가 `ENTER SWITCH`·`ENTER NEXT`·`ENTER RIGHT CHOOSE A FOLDER` |
| 3.17 X6 | 내 기록 DB 가 이 빌드에서 어떻게 보이나 | 앱을 켜기 전에 설정 폴더의 `scoredb.sqlite` 를 복사해 두고(원하면 `RBMS_SCOREDB_COPY=<사본> cargo test -p rbms-player --lib --locked -- --ignored a_players_own_database --nocapture`), 앱에서 예전 SCRATCH AUTO 곡을 봄 | 그 곡들의 램프(LIGHT ASSIST CLEAR)와 횟수가 그대로이고 점수는 `-`, 새로 친 판이 첫 최고 기록, 결과 화면이 옛 판의 EX 를 이전 점수로 안 보임(웨이브 16·18 측정: 37판·29차트·LOCAL BEST 0·램프 29) |
| 3.17 X7 | 첫 실행의 발광 표 | 설정 폴더를 옮겨 새 설치처럼 만든 뒤(끝나면 되돌림) 네트워크를 끊고 켬, 이어서 네트워크를 잇고 Tables(T)에서 그 표에 R | 선곡 뿌리에 ALL SONGS 와 발광 표(로그인 전이라 MY FOLDERS 없음), 표 안에 26개 레벨 폴더, Tables 에서 표 줄이 `SAVED`, R 뒤 `https://bms.hyuns.uk/tables/insane1` 에서 다시 받음, D 로 지우고 재시작해도 안 돌아옴 |
| 3.17 X8 | 설정 파일 스키마 5 에서 6 으로 | 기존 설정 폴더로 이 빌드를 처음 켬 | `settings.ron.v5.bak` 생성, NETWORK 의 SERVER URL 이 비어 있었으면 `https://bms.hyuns.uk/api` 가 들어가고 있었으면 그대로, 비우고 재시작하면 비어 있는 채로 유지 |
| 3.17 X9 | MY FOLDERS(실제 서버) | 웹에서 폴더 둘을 만들고 곡을 넣은 뒤 앱에서 로그인, F6·Tables 의 R, 웹에서 이름을 바꾼 뒤 F6, 로그아웃 | 선곡 뿌리에 `MY FOLDERS (n)`, 안에 `이름 (가진 곡/전체)`, 폴더 안엔 가진 곡만, F6 뒤 바뀐 이름, Tables 맨 끝 줄에 받은 시각, 로그아웃하면 줄이 사라지고 일본어 이름이 제대로 보임 |
| Phase H AUDIO | 오디오 | 곡선택에서 프리뷰가 있는 곡과 없는 곡을 번갈아 포커스한 뒤 interactive 단일 차트로 레인 키를 누름. 차트 시작 직후 BGM·키음, 프리뷰 전환 반복 (`2026-09-16-phase-h-manual-checks.md`) | 키음이 눈에 띄는 지연 없이 들리고 BGM 시작이 차트 시작과 맞으며 프리뷰 전환·재생 중 클릭·팝·끊김 없음 |
| Phase H Practice | 연습(F4) | LN 의 시작 또는 끝이 범위 경계를 넘고 범위 안에 BGA 변화가 있는 곡에서 START TIME·END TIME 을 경계 양쪽으로 설정해 Enter, 같은 설정으로 한 번 더 | 경계를 가로지르는 LN 의 시작·종료 판정이 유지, 범위 안 BGA 가 표시, 반복 실행에서 설정·BGA 가 누적되거나 사라지지 않음, 연습 결과는 로컬 기록과 IR 제출에 반영 안 됨 |
| Phase H Course | 코스 진행 | Shift+Tab 으로 COURSE 탭, 2스테이지 이상 코스를 시작해 첫 스테이지를 비최대 게이지와 콤보로 끝내고 둘째 시작 상태와 COURSE RESULT 확인 | 둘째 스테이지의 초기 게이지·콤보가 첫 스테이지 종료 상태에서 이어지고 COURSE RESULT 집계가 실제 진행과 일치 |
| Phase H Multi-IR | 다중 IR | SETTINGS NETWORK 에서 primary 와 추가 enabled 프로필을 준비하고 `I` 로 IR RANKING 을 열어 `Q`·`E` 로 primary 전환, 리플레이 있는 행을 선택해 재생 | 프로필 라벨과 랭킹이 같은 primary, 이전 서버의 늦은 응답이 새 랭킹을 안 덮어씀, 리플레이 다운로드·재생이 선택한 primary 서버를 사용, 추가 프로필만 있어도 점수 fan-out 상태가 수렴 |
| Windows 7항목 | 실제 Windows 런타임 | `docs/reference/windows-compat.md` 의 미체크 7항목(GUI 렌더, 오디오, 입력, 파일 대화상자, CJK 폰트, HiDPI, 경로 해석)을 실제 Windows 에서 | 그 문서의 각 항목 설명을 따름 |

이 표에서 뺀 것: N9, W8(다른 절로 옮긴 표지), `2026-09-17-skin-system/checklist.md` 3절의 미체크 4개(대체됨. 그 문서 머리 참조).

## 1. 준비

| 항목 | 내용 |
| --- | --- |
| 빌드 | `cargo build --release --locked -p rbms-player` (macOS arm64 에서 약 1분) |
| 격리 실행 | `HOME=$(mktemp -d) ./target/release/rbms-player <곡 폴더>`. 사용자 설정을 건드리지 않는다. 설정 폴더는 `$HOME/.config/rbms/` 이다. 확인 중 같은 임시 폴더를 계속 쓰려면 `export TMPHOME=$(mktemp -d)` 로 정해 두고 `HOME=$TMPHOME` 로 준다 |
| 곡 폴더 | 저장소의 `samples/preview-demo` 는 5키 차트뿐이다. 7키, 9키(PMS), 10키, 14키 차트는 직접 준비한다. 제목이 한글이거나 일본어인 차트, 스테이지 그림과 BGA 가 있는 차트, 이미 플레이 기록이 있는 곡과 없는 곡이 섞여 있으면 좋다 |
| 외부 스킨 | `/Users/hyunseokbyun/Downloads/ModernChic` (읽기 전용으로만 쓴다. 팩 폴더에 아무것도 쓰지 않는다) |
| 기본 조작 | 선곡: 위·아래 이동, Enter 열기/시작, Esc 뒤로, Tab 설정, `/` 검색, F1 옵션 오버레이, H 키 안내. START = A, SELECT = W. 결정: Enter 건너뛰기, Esc 취소. 플레이 건반: 7키 `Z S X D C F V` + 왼쪽 Shift(스크래치. 반대 방향은 왼쪽 Ctrl), 9키 `Z S X D C F V G B`, 14키는 오른쪽에 `M K , L . ; /` + 오른쪽 Shift(반대 방향은 오른쪽 Ctrl) (키 설정 화면에서 바꿀 수 있다) |

## 2. 자동으로 확인된 것

사람이 다시 하지 않아도 되는 것이다. 근거는 웨이브 8B~8C 단위 보고서의 실행 출력이다. 이 환경(macOS arm64, 헤드리스)에서의 결과이며 Windows 와 Linux 는 푸시 뒤 CI 로 확인한다.

| 항목 | 확인한 것 | 수단 |
| --- | --- | --- |
| 문서 9개 로드 | 문서 9개 모두 `load ok`, 경고 0, pcall 0, 함수 실패 0, 예산 초과 0, 팩 폴더 불변(읽기 전용 사본 포함) | `cargo run -p rbms-cli -- skin-dump assets/skins/rbms-default --strict --frames 120`, `apps/rbms-cli/src/skin_dump/tests.rs` 의 엄격 게이트 테스트 |
| 프레임 비용 | 문서별 평균 0.04~0.10 ms, 최대 0.174 ms (코스 결과) | 같은 덤프의 `--probe-budget` |
| 화면 9종 그리기 | 선곡, 결정, 플레이 5/7/9/10/14키, 결과, 코스 결과를 640x360 으로 그려 스킨 장면이 컴파일되고 경고 0, 객체 종류, 네 모서리가 바탕색임을 단언 | `apps/rbms-player/src/stage/capture/gate.rs` (환경 변수 불필요) |
| 화면 캡처 | 선곡(곡/폴더/코스 탭, 패널 0~3), 플레이 7/9/14키, 결과를 메인이 PNG 로 열어 확인. 겹침과 잘림 없음. 80장은 같은 팩으로 다시 찍어도 픽셀까지 같다(`pack-*` 의 진행 막대와 맥동 제외) | `RBMS_SKIN_PACK=<기본 팩> RBMS_SKIN_CAPTURE_DIR=<폴더> cargo test -p rbms-player …` (설계서 §8.2) |
| 첫 실행 선곡 | 팩 지정 없이 설치본으로 선곡이 기본 스킨으로 그려진 PNG 를 열어 확인(곡 목록, 스테이지 그림, 기록, 판정 막대) | `the_browser_of_a_first_run…` 테스트의 캡처 |
| 금지 API | 팩의 Lua 에 luajava, io, os, http, 난수, load 류, `_G`/`_ENV` 접근이 없다 | `crates/rbms-skin/tests/default_pack.rs` |
| 시트 대조 | 이미지 시트 5장이 아틀라스와 일치(파일 존재, PNG 크기, 스프라이트 범위, 칸 분할) | 같은 테스트 |
| 설치 규칙 | 첫 설치에 전 파일 존재, 재실행 쓰기 0(파일·폴더 mtime 불변), 사용자가 고친 파일 보존, 손대지 않은 구판 갱신, 더는 싣지 않는 파일 제거, 기록 없는 이전 설치본 인식, 팩 밖을 가리키는 기록 무시, 링크 폴더 너머로 쓰지 않음, 중단 파일 청소, 쓰기 불가에서 패닉 없음 | `apps/rbms-player/src/assets/default_pack/tests.rs` |
| 기본 선택 | 지정 없음은 기본 팩, `RBMS_SKIN_PACK` > `skin.pack` > 기본 팩, PACK FOLDER 행 표시(`DEFAULT (<경로>)`)와 비우기, 식별자와 오버레이 폴더 안정, 옵션 저장과 복원 | `apps/rbms-player/src/skin_select/default_pack_tests.rs` |
| 폴백 | 로드 실패 알림 4종(문법 오류, 본체 오류, 예산 초과, 파일 없음)과 알림 한 번, 24키와 키 설정이 팩 없는 앱과 픽셀 동일 | `apps/rbms-player/src/skin_screen/fallback_tests.rs` |
| 설정 표지 | 표지가 붙는 모든 행에서 표지가 라벨과 값 사이에 잘리지 않고 들어간다. 스킨이 없는 구성의 화면은 전과 같다. 동기화가 `skin` 그룹을 올리지도 받지도 않는다 | `crates/rbms-config`, `apps/rbms-player/src/settings_view.rs`, `stage/settings/note_tests.rs`, `ir_sync.rs` 테스트 |
| 결과 입력 | 5키·10키 결과에서 턴테이블이 게이지 전환과 같은 배치 재시작을 한다 | `apps/rbms-player/src/stage/result/tests.rs` |
| GPU 대 CPU | 같은 그리기 목록의 GPU 출력과 CpuCanvas 가 2/255 이내 | `apps/rbms-player/src/gpu/pixel_tests.rs` (이 기기의 Metal 에서 실제 실행) |
| 동영상 | ModernChic mp4 8개를 끝까지 디코드해 프레임 수 일치, 시스템 디코더와 화소 대조 일치, 1080p 프레임당 6~8 ms | `crates/rbms-video` 테스트 (release 92초) |
| 외부 팩 | `RBMS_SKIN_PACK` 으로 기본 팩과 ModernChic 각각을 준 전체 테스트가 통과(ModernChic 은 키 설정 문서만 원본 결함) | `cargo test --workspace --no-fail-fast` |
| PLAY 메뉴(웨이브 10B) | 행·조작·요약·저장·자원 해제와 미리보기의 반영(§3.9 의 머리말). 캡처를 PNG 로 열어 유지 켬의 BPM 100·150·200, 유지 끔, 가림막 전후, Wide, 14키, 문서 없음, 스크롤 끝, ModernChic 7키, Settings 를 확인 | `apps/rbms-player/src/stage/play_menu/tests.rs`, `stage/settings/play_tests.rs`, `skin_preview/tests.rs`, `stage/play/skin_preview/tests.rs`, `skin_preview/target.rs` |

## 3. 사람만 확인할 수 있는 것

체크박스 앞의 번호로 이 문서를 가리킨다. "방법" 의 화면 이름은 SKIN 메뉴(선곡에서 F12), 선곡 등 앱의 것이다.

### 3.1 첫 실행과 설치

- [ ] **A1 첫 실행 설치.** 방법: 비어 있는 임시 HOME 으로 곡 폴더를 주고 실행한다. 종료 뒤 `$HOME/.config/rbms/skin/rbms-default/` 를 연다. 기대: 창이 뜨자 선곡이 기본 스킨(어두운 평면 바탕, 영어 문구)으로 그려진다. 폴더에 문서 9개(`decide`, `select`, `play5/7/9/10/14`, `result`, `courseresult` 의 `.luaskin`), `lua/`(27개), `image/`(5장), `font/`(2개), `sound/`(22개), `install-manifest.txt` 가 있고 `tools/` 는 없다. 창이 뜨기까지 눈에 띄는 지연이 없다.
- [ ] **A2 재실행.** 방법: 같은 HOME 으로 다시 실행한다. 기대: `install-manifest.txt` 와 다른 파일의 수정 시각이 바뀌지 않는다(`ls -l`). 경고 알림이 없다.
- [ ] **A3 고친 파일 보존과 지운 파일 복구.** 방법: `lua/theme.lua` 에 공백 줄 하나를 더하고 `image/common.png` 를 지운 뒤 다시 실행한다. 기대: `theme.lua` 는 그대로 남고 `common.png` 는 다시 생긴다. 화면이 정상이다.
- [ ] **A4 쓰기 불가.** 방법: 설정 폴더를 읽기 전용으로 두고(또는 `skin` 이라는 이름의 일반 파일을 만들어 두고) 실행한다. 기대: `default skin not fully installed: N files could not be written beside …` 알림이 한 번 뜨고, 선곡 등 화면이 내장 화면으로 그려지며 앱이 죽지 않는다.

### 3.2 화면 흐름 (키 수별)

각 키 수마다 아래를 처음부터 끝까지 한 번 한다. 5키는 `samples/preview-demo`, 나머지는 준비한 차트로 한다.

- [ ] **B1 5키.** [ ] **B2 7키.** [ ] **B3 9키.** [ ] **B4 10키.** [ ] **B5 14키.**
  방법: 선곡에서 차트를 고르고 Enter → 결정 화면 → 플레이 → 완주(또는 일부러 실패) → 결과 → Enter 로 선곡 복귀. 기대:
  - 결정: 제목·장르·아티스트, LEVEL/BPM/NOTES/JUDGE/KEYS, 노트 분포, 스테이지 그림(있으면)이 보이고 약 3초 뒤 어두워지며 플레이로 넘어간다. Enter 로 건너뛰고 Esc 로 선곡에 돌아가며, 돌아간 선곡의 커서·검색·탭 상태가 떠날 때 그대로다.
  - 플레이: 로딩 막대 → READY → 노트가 판정선에 닿는 순간 키 입력이 판정되고 판정 글자·콤보·게이지·스코어 그래프·판정 수가 갱신된다. 9키는 필드가 가운데, 10/14키는 두 필드가 대칭이고 스크래치가 바깥쪽. 실패하면 `FAILED` 와 어두워지는 덮개, 완주하면 페이드.
  - 결과: 클리어 또는 실패 머리 글자와 색, 랭크, 점수와 차이 부호, 판정 표, 그래프 4개, 게이지 이름. 5키·10키 결과에서 턴테이블(왼쪽 Shift)로 게이지 그래프가 바뀐다. R 은 같은 곡 다시, N 은 다음 곡.
  - 어디서도 화면이 멈추거나 깜박이거나 검은 프레임이 길게 남지 않는다.
- [ ] **B6 코스.** 방법: Shift+Tab 으로 COURSE 탭에서 2스테이지 이상 코스를 시작한다. 기대: 첫 스테이지만 결정 화면을 거치고, 스테이지 사이에는 결과 없이 이어지며, 마지막에 COURSE RESULT(스테이지 목록, 이어 붙은 게이지 그래프)가 뜬다.

### 3.3 선곡의 조작

- [ ] **C1 옵션 패널 1~3.** 방법: 선곡에서 A(START)를 누르고 있는다 → W(SELECT) → 둘을 함께(또는 숫자 5). 기대: 누르고 있는 동안만 패널이 아래에서 올라온다. 패널 1(PLAY OPTIONS), 2(ASSIST OPTIONS), 3(DETAIL OPTIONS)의 칸 7개가 건반 순서로 있고 값이 읽힌다. 값이 없는 칸은 `NOT AVAILABLE`. 건반을 눌러 RANDOM·GAUGE 등을 바꾸면 칸의 값이 바뀌고, 놓은 뒤 플레이에 반영된다.
- [ ] **C2 사이드 메뉴와 볼륨.** 방법: 왼쪽 가장자리의 톱니와 스피커 아이콘을 클릭한다. 기대: 톱니는 QUICK SETTINGS 6행, 스피커는 VOLUME 3개를 연다. 한 번에 하나만 열린다. 여닫을 때 효과음이 난다. 볼륨 슬라이더를 드래그하면 숫자(%)와 채움이 따라 움직이고 실제 소리 크기가 바뀐다.
- [ ] **C3 마우스 호버·클릭·드래그.** 기대: MODE/SORT/LN MODE 버튼과 AUTOPLAY/PRACTICE/FAVORITE, 리플레이 칸에 커서를 올리면 강조되고, 클릭하면 동작한다(좌클릭 다음, 우클릭 이전). 곡 막대의 좌클릭은 선택·열기, 우클릭은 닫기. 오른쪽 스크롤 손잡이 드래그와 마우스 휠로 목록이 움직인다. 스킨이 없는 곳을 클릭해도 앱이 오동작하지 않는다.
- [ ] **C4 검색.** 방법: 머리의 SEARCH 칸을 클릭해 입력하고, `/` 도 눌러 본다. 한글·일본어를 IME 로 입력해 본다. 치는 중에 F1, 방향키를 눌러 보고, Enter 로 확정한 뒤 패널 1~3 과 Tab 을 열어 본다. 기대: `/` 와 클릭이 같은 스킨 검색 칸에 입력되고 앱의 검색 상자는 따로 뜨지 않는다. 글자마다 목록이 걸러지고 조합 중인 글자가 밑줄로 보인다. 확정하면 경로 줄이 `Search : '검색어' >` 가 되고 결과 목록은 일반 목록과 똑같이 조작된다. Esc 로 검색을 풀면 원래 폴더와 행으로 돌아온다.
- [ ] **C5 기록 표시.** 기대: 기록이 있는 곡은 랭크·판정 5행·숫자 6칸이 보이고 없는 곡은 줄표와 NO PLAY. 그림이 없는 곡에는 `NO STAGE IMAGE`.

### 3.4 글자와 창

- [ ] **D1 한글·일본어 제목.** 방법: CJK 제목 곡을 선곡 목록(막대), 곡 머리, 결정, 결과에서 본다. 기대: 빈 사각형 없이 시스템 글꼴로 그려지고, 긴 제목은 막대·글줄 폭에 맞게 줄어들며 이웃 요소와 겹치지 않는다. 기준선이 라틴 글자와 크게 어긋나지 않는다. (자동 확인은 번들 Inter 하나뿐이라 이 항목이 유일한 확인이다.)
- [ ] **D2 1280x720 창.** 방법: 기본 창으로 모든 화면. 기대: 18픽셀 글자가 읽히고 잘림이 없다.
- [ ] **D3 다른 해상도·비율.** 방법: DISPLAY 탭에서 RESOLUTION 을 1920x1080, 2560x1440 으로, WINDOW MODE 를 BORDERLESS 로 바꾼다. 일반 창을 마우스로 늘려 4:3 에 가깝게, 21:9 에 가깝게 만든다. LETTERBOX 를 켬/끔으로 바꿔 본다. 기대: 켬이면 16:9 화면이 비율을 유지한 채 가운데에 놓이고 남는 곳은 검다. 끄면 화면이 창에 맞게 늘어난다. BORDERLESS 는 항상 비율을 맞춘다. 마우스 클릭 위치가 레터박스 안에서 그려진 요소와 일치한다(호버·클릭이 어긋나지 않는다).
- [ ] **D4 새 설치의 LETTERBOX 기본값.** 기대: 비어 있는 HOME 에서 DISPLAY 탭의 LETTERBOX 가 ON. 이미 저장된 설정이 있으면 그 값을 따른다.

### 3.5 외부 스킨 팩

- [ ] **E1 외부 팩으로 바꾸기.** 방법: 선곡에서 F12 → SKIN 메뉴 → PACK FOLDER 에서 Enter 로 ModernChic 폴더를 고른다. Esc 로 선곡에 돌아가 결정·플레이·결과까지 한 번 한다. 기대: 문서가 있는 화면은 ModernChic 으로, 9키 플레이는 문서가 없어 기본 스킨이 아니라 내장 필드로 그려진다. PACK FOLDER 행에 고른 경로가 보인다. ModernChic 의 `keyconfig.luaskin` 은 원본 결함으로 본문이 실패하므로, 키 설정 화면을 열면 그 파일의 실패 알림이 한 번 뜨고 화면은 내장으로 그려진다.
- [ ] **E2 되돌리기.** 방법: PACK FOLDER 행에서 왼쪽 키로 비운다. 기대: 행이 `DEFAULT (<기본 팩 경로>)` 로 바뀌고 모든 화면이 기본 스킨으로 돌아온다. `RBMS_SKIN_PACK=<폴더>` 로 실행하면 행에 `(RBMS_SKIN_PACK)` 가 붙고 설정 파일은 바뀌지 않는다.
- [ ] **E3 동영상이 있는 외부 스킨.** 방법: ModernChic 의 결정·선곡·플레이를 본다(범용 BGA 와 배경 동영상). 기대: 영상이 반복 재생되고 끊기지 않는다. 화면을 오래(수 분) 켜 두어도 느려지거나 메모리가 계속 늘지 않는다(DISPLAY 탭의 DEBUG MODE 패널 또는 활동 모니터로 확인). 영상 없이 빌드한 `--no-default-features` 는 영상 자리가 비는 것까지만 확인하면 된다.
- [ ] **E4 스킨 옵션과 오프셋.** 방법: SKIN 메뉴에서 화면 띠의 7K 를 고르고 기본 스킨의 옵션(Lane Width, Field Side, Scratch Side, Judge Detail, Score Graph)과 오프셋 행을 바꾼 뒤 Esc 로 나가 플레이한다. 포커스한 오프셋 행에서 숫자 0 을 누른다. 기대: 옵션 값이 미리보기와 실제 플레이 화면에 같게 반영되고, 오프셋이 즉시 반영되며, 0 키가 그 행을 0 으로 되돌린다. RESET 이 모든 선택을 버린다.
- [ ] **E5 로드 실패 알림.** 방법: 임시 팩 폴더에 문법 오류가 있는 `.luaskin`(예: 내용이 `return {{{`)을 두고 SKIN 메뉴의 PACK FOLDER 로 지정한다. 기대: 미리보기 자리에 `NO SKIN DOCUMENT FOR THIS SCREEN` 과 `<파일> DID NOT READ (header pass): <이유>` 가 나오고 LOADED 값에도 같은 문구가 붙는다. Esc 로 나가 화면을 열면 `skin <파일> did not load, drawing the built-in screen: …` 토스트가 한 번 뜨고 그 화면은 내장 화면으로 그려진다.

### 3.6 소리와 내장 화면

- [ ] **F1 소리.** 기대: 선곡 이동, 곡 결정, 결정 화면, 패널과 사이드 메뉴 여닫기, 결과(클리어/실패/닫기) 효과음이 제 때 난다. 플레이 중 키음과 BGM 은 이전과 같고 끊김이 없다. SOUND FOLDER 를 비워 두면 설치본의 `sound/` 가 쓰인다.
- [ ] **F2 내장 화면의 밝기(E10).** 배경: 색을 바이트 그대로 통과시키는 표면으로 바꾸면서 실제 창의 내장 화면이 전보다 어둡고 대비가 강해졌을 수 있다(헤드리스 골든과는 일치). 방법: SETTINGS, KEY CONFIG, TABLES, FOLDERS, PRACTICE, 로딩 화면을 본다. 기대: 모두 읽을 수 있다. 읽기 어렵거나 어색하면 화면과 느낌을 적어 둔다 — 그 기록이 `theme.ron` 색 재조정 작업의 근거가 된다(사양은 범위 밖으로 두었다).
- [ ] **F3 설정 행 표지.** 방법: 기본 스킨 상태에서 PLAY 탭의 5KEYS LAYOUT, DISPLAY 탭의 SKIN/WHITE NUMBER/JUDGE TEXT Y/RESULT GRAPHS, INPUT 탭의 SCRATCH SIDE 를 본다. 기대: 라벨 옆에 작은 글씨로 `BUILT-IN SCREENS ONLY` 또는 `SKIN OPTION COMES FIRST` 가 있고 값이 흐리다(포커스하면 평소 색). 값은 그대로 바꿀 수 있다. 내장 화면만 쓰는 구성(다른 팩에 문서 없음)에서는 표지가 없다.
- [ ] **F4 오버레이.** 기대: 선곡 위에서 `H`(키 안내), F1(옵션 오버레이), I(IR 랭킹), 토스트가 뜨고 닫히며 스킨 위에서 읽을 수 있다. (알려진 한계: 랭킹 패널이 정보 영역 오른쪽 124px 을 덮지 못함, 옵션 오버레이가 반투명이라 뒤가 비침, 토스트가 아래쪽 막대를 덮음.)

### 3.7 SKIN 메뉴

자동 검사는 헤드리스 캔버스와 오프스크린 GPU 로 메뉴와 미리보기를 그려 확인했다(테두리 밖으로 새지 않음, 화면 종류 9개의 예시 장면, 옵션·오프셋 반영, 실패 문구, 떠난 뒤 텍스처 0). 아래는 실제 창에서만 드러나는 것이다.

- [ ] **G1 들어가고 나가기.** 방법: 선곡에서 F12, 기본 스킨 사이드 메뉴(톱니)의 SKIN 행, SETTINGS(Tab)에서 F12 와 탭 띠 오른쪽 끝 `SKIN MENU >` 단추로 각각 연다. Esc 로 나간다. 기대: 네 길 모두 SKIN 메뉴가 열리고, Esc 는 왔던 화면으로 돌아간다(SETTINGS 는 보던 탭 그대로). SETTINGS 탭 띠에 SKIN 탭이 없다. 선곡으로 돌아온 첫 순간에 화면이 검게 비거나 선곡 스킨의 여는 연출이 다시 나오지 않는다(SELECT 를 미리본 뒤 돌아올 때만 다시 읽는다).
- [ ] **G2 미리보기가 움직인다.** 방법: 화면 띠를 Tab 과 클릭으로 7K 5K 14K 10K 9K SELECT DECIDE RESULT COURSE 로 옮긴다. 기대: 오른쪽 16:9 사각형에 그 화면의 스킨이 그려지고 예시 데이터로 움직인다(플레이는 노트가 내려오고 판정·콤보·게이지·점수가 변한 뒤 반복, 선곡은 커서가 내려감). 곡 제목은 `SAMPLE …` 이고 내 라이브러리의 곡·점수·플레이어 이름이 보이지 않는다. 소리가 나지 않는다. 장면이 끝나 다시 시작할 때 `LOADING THE SKIN` 글자가 번쩍이지 않는다(잠깐 검다).
- [ ] **G3 값이 바로 반영된다.** 방법: 7K 에서 Lane Width, Field Side, Judge Detail, Score Graph 를 ← → 와 클릭으로 바꾼다. 오프셋 행(판정 위치, ALL OFFSET)을 ← →, Shift+← →, 숫자 0 으로 움직인다. 행을 오른쪽 클릭한다. 기대: 옵션은 잠깐 뒤(검은 화면 몇 프레임) 장면이 이어지며 바뀌고, 오프셋은 누르는 즉시 움직인다(ALL OFFSET 포함, 깜빡임 없음). Shift 는 열 칸, 오른쪽 클릭은 한 칸 뒤로. 조작 중 UI 가 멈추지 않는다.
- [ ] **G4 목록과 마우스.** 방법: ModernChic 을 팩으로 두고 7K 를 연다(행 89개). ↑ ↓, PageUp/PageDown, 휠(포인터를 목록 위와 미리보기 위에 각각 두고)로 움직인다. 기대: 스크롤 막대와 `현재 / 전체` 가 보이고 커서 행이 항상 화면 안에 있다. 휠은 목록 위에서만 동작한다. 미리보기 위의 클릭은 아무 일도 하지 않는다. 커서 행의 긴 이름·값 전문이 미리보기 아래에 읽힌다. 일본어 옵션 이름이 시스템 글꼴로 그려진다(헤드리스 캡처는 내장 글꼴뿐이라 빈 사각형이었다).
- [ ] **G5 폴더 선택 창.** 방법: PACK FOLDER 에서 Enter. 창이 떠 있는 동안 메뉴에 키를 누르고 클릭한다. 폴더를 고르거나 취소한다. 기대: 창이 떠 있는 동안 메뉴가 계속 그려지되 키·클릭에 반응하지 않는다. 고르면 행에 경로가 보이고(긴 경로는 앞을 `…` 로 줄여 폴더 이름이 남는다) 미리보기가 그 팩으로 바뀐다. 취소하면 변화가 없다.
- [ ] **G6 없는 문서와 실패.** 방법: ModernChic 에서 9K, KEYS, SCREEN 행으로 PLAY 24KEYS 를 본다. 존재하지 않는 폴더 경로를 `settings.ron` 의 `skin.pack` 에 적고 실행한다. 본문에서 `error("x")` 를 내는 `.luaskin` 을 든 팩을 지정한다. 기대: 차례로 `NO SKIN DOCUMENT FOR THIS SCREEN`, `NO PREVIEW FOR THIS SCREEN`, `NO SKIN DOCUMENT…`, `THE PACK FOLDER IS NOT THERE` 와 PACK FOLDER 행의 `(MISSING)`, `THE SKIN DID NOT LOAD` 와 파일 이름으로 시작하는 이유. 글자가 서로 겹치지 않고 앱이 멈추지 않는다.
- [ ] **G7 창 크기.** 방법: 1920x1080, BORDERLESS, 4:3 에 가깝게 늘린 창(LETTERBOX 켬/끔)에서 메뉴를 연다. 기대: 미리보기가 테두리 안에 꼭 맞고 밖으로 새지 않는다. LETTERBOX 를 끈 창에서는 메뉴 전체가 창 모양으로 늘어나며 미리보기도 같이 늘어난다(실제 스킨 화면이 그 창에서 그려지는 모양과 같다).
- [ ] **G8 동영상과 자원.** 방법: ModernChic 의 7K·SELECT 를 미리본다(동영상 BGA·배경). 화면 종류를 여러 번 바꾸고 메뉴를 나간다. DISPLAY 탭의 DEBUG MODE 패널을 켜 둔다. 기대: 동영상이 사각형 안에서 재생된다. `SKIN TEX` 값이 화면 종류를 바꿀 때마다 한 화면분으로 돌아오고 계속 늘지 않는다.

### 3.8 웨이브 10A: 넓은 레인, 키, 판정 자동 보정, 녹색 숫자 유지

자동 검사는 헤드리스로 확인했다: Wide 의 레인 폭이 Normal 의 1.5 배이고 상자가 겹치지 않음(5·7·9·10·14키와 옵션 조합), 키 설정의 바인딩·거절·복원과 이전 파일 읽기, 40 ms 빠른 입력에 오프셋이 1 씩 움직임(웨이브 13 부터 -1 씩)과 그 리플레이·IR 왕복의 재현, 150·200 BPM 차트가 같은 녹색 숫자로 시작함. 아래는 실제 창·실제 손·실제 컨트롤러에서만 드러나는 것이다. 옵션의 뜻은 `docs/play-options.md`.

- [ ] **H1 넓은 레인.** 방법: SKIN 메뉴(F12)에서 7K 의 Lane Width 를 Wide 로 바꾸고 BGA(그림·동영상)가 있는 곡을 플레이한다. 5K, 9K, 10K, 14K 도 한 번씩, 7K 는 Score Graph Off 와 Field Side Right 도 본다. 기대: 레인이 눈에 띄게 넓고 노트·판정선·레인 커버·판정 글자·게이지가 필드 폭을 따라간다. BGA, 점수, 판정 수, 타이밍, 그래프가 서로 겹치거나 잘리지 않는다. 4자리 판정 수와 10분 이상의 남은 시간도 상자 안에 들어간다. Normal 로 되돌리면 이전 화면과 같다.
- [ ] **H2 키 설정 화면.** 방법: 선곡에서 F5, Settings 에서 F5 와 `KEY CONFIG >` 단추(클릭)로 각각 연다. 레인, PLAY 묶음의 조작(LANE COVER ON/OFF, FINE STEP 포함), SELECT·SCREENS·숫자 키·RESULT·REPLAY ANALYSIS 묶음에서 하나씩 Enter 로 바인딩한다. 이미 쓰는 키를 주어 본다. Delete, Backspace, 맨 아래 전체 복원(Enter 두 번)을 쓴다. 컨트롤러 칸에서 실제 패드 버튼과 턴테이블 축을 준다. 기대: `PRESS...` 대기 뒤 누른 키·버튼이 칸에 뜬다. 같은 화면에서 읽히는 것끼리는 거절되고 무엇이 그 키를 쓰는지 빨간 문구가 뜬다. Esc 로 나갔다 다시 열어도 그대로이고 앱을 다시 켜도 그대로다.
- [ ] **H3 옮긴 키가 실제로 동작한다.** 방법: 검색(/), 정렬(F3), 스킨 메뉴(F12), 다시 하기(R)를 다른 키로 옮기고 각 화면에서 누른다. 선곡에서 H 를 연다. 기대: 옛 키는 아무 일도 하지 않고 새 키가 동작한다. H 가이드가 새 키를 보여 준다. 검색어를 치는 중에는 글자 키가 검색어로 들어간다.
- [ ] **H4 레인 커버 조작.** 방법: 플레이 중 → ←, `\`(켬·끔), START 두 번, START + 턴테이블, 마우스 휠, Shift(레인이 아닌 쪽)를 누른 채 → ← 를 쓴다. 커버를 끈 상태에서 → 를 누른다. 기대: → 가 더 가리고 ← 가 덜 가린다. 커버 끝에 흰 숫자(왼쪽)와 녹색 숫자(오른쪽)가 따라 움직인다. 꺼진 상태에서 값을 바꾸면 발치에 COVER 수치가 잠깐 뜬다. Shift 를 누르면 더 잘게 움직인다.
- [ ] **H5 판정 자동 보정.** 방법: PLAY 메뉴(F8) > JUDGE 에서 AUTO ADJUST 를 켜고 일부러 조금 빠르게(또는 느리게) 친다. 곡을 끝까지 치고, 다른 판은 중간에 Esc 로 나가고, 또 다른 판은 폐점한다. 방금 친 판의 리플레이를 재생한다. 기대: 플레이 화면의 표지가 `TIMING AUTO` 이고 숫자가 칠 때마다 조금씩 움직인다(웨이브 13 부터 빠르게 치면 -, 늦게 치면 +. 원본과 같은 부호다. 노트가 그만큼 늦게·먼저 온다: M6). 세 경우 모두 PLAY 메뉴의 JUDGE OFFSET 에 마지막 값이 남는다. 리플레이의 판정 수가 원래 판과 같다. 오토플레이와 리플레이에서는 숫자가 움직이지 않는다.
- [ ] **H6 녹색 숫자 유지.** 방법: PLAY 메뉴(F8) > SPEED 에서 KEEP GREEN NUMBER 를 켜고 GREEN NUMBER 를 정한 뒤 BPM 이 다른 곡 둘을 레인 커버·리프트를 켠 채 플레이한다. 플레이 중 커버를 움직이고 켜고 끈다. ↑ ↓ 와 SELECT + 건반을 눌러 본다. 변속이 있는 곡과 HI-SPEED FIX 를 바꾼 경우도 본다. 선곡에서 패널 3(START+SELECT)의 4·6번 키를 누르고 있는다. 기대: 두 곡 모두 발치의 GREEN 이 정한 숫자이고 HI-SPEED 만 다르다. 커버를 움직여도 GREEN 은 그대로이고 HI-SPEED 가 바뀐다. ↑ ↓ 는 GREEN 을 10 씩(Shift 로 1 씩), SELECT + 건반은 이동 시간을 1 ms 씩 바꾸고 다음 곡에도 남는다. 패널 3 의 NOTE DISPLAY TIME 칸이 이동 시간과 녹색 숫자를 보이고, 꺼져 있으면 `OFF` 다. 노트가 실제로 같은 시간 동안 보이는지 눈으로 느낀다.
- [ ] **H7 이전 설정 파일.** 방법: 이 판을 설치하기 전의 `settings.ron`, `keyconfig.ron`, 리플레이 파일을 그대로 둔 채 실행한다. 기대: 키와 설정이 이전과 같다. KEEP GREEN NUMBER 는 꺼져 있다. 이전 리플레이가 이전과 같은 결과로 재생된다. 웨이브 13 부터는 설정 스키마가 5 로 올라 `settings.ron.v4.bak` 이 생기고 JUDGE OFFSET 의 부호가 한 번 뒤집혀 보인다(M7).

### 3.9 웨이브 10B: PLAY 메뉴

자동 검사는 헤드리스로 확인했다: 행 목록과 묶음, 읽기 전용 전환, 단위, Shift 열 칸과 범위 끝, 0 기본값, 클릭·오른쪽 클릭·휠, 키로 올린 레인 커버 1~100% 의 흰색 숫자가 비율의 10 배임, 요약이 `rbms_config::held_speed` 등과 같음, 유지 켬·끔에서 노트 낙하 시간의 픽셀 측정, 예시 판정이 판정 오프셋과 무관함, 차트가 돌아도 장면이 이어짐, 모드·BPM 이 저장되지 않음, Esc 뒤 설정 파일 왕복, 떠난 뒤 텍스처 0, Settings 에 PLAY 메뉴의 행이 없음, 이전 `keyconfig.ron` 에서 F8 을 다른 단축키가 쓰고 있으면 PLAY MENU 가 미지정으로 읽힘. 아래는 실제 창에서만 드러나는 것이다. 화면의 뜻은 `docs/play-options.md` 7절.

- [ ] **I1 들어가고 나가기.** 방법: 선곡에서 F8, SETTINGS(Tab)에서 F8 과 탭 띠의 `PLAY MENU >` 단추(클릭)로 각각 연다. Esc 로 나간다. 검색어를 치는 중에 F8 을 눌러 본다. macOS 노트북은 fn + F8 이다. 기대: 세 길 모두 PLAY 메뉴가 열리고 Esc 는 왔던 화면으로 돌아간다(SETTINGS 는 보던 탭 그대로). SETTINGS 의 탭은 JUDGE DISPLAY INPUT NETWORK AUDIO SELECT 여섯이고 `PLAY MENU >`, `SKIN MENU >`, `KEY CONFIG >` 단추가 서로 겹치지 않는다. 선곡으로 돌아온 첫 순간에 화면이 검게 비거나 선곡 스킨의 여는 연출이 다시 나오지 않는다.
- [ ] **I2 녹색 숫자 유지를 눈으로 보기.** 방법: KEEP GREEN NUMBER 를 켜고 GREEN NUMBER 를 정한 뒤 PREVIEW BPM 을 100 → 150 → 200 으로 바꾼다. 다음에는 끄고 같은 순서로 바꾼다. 기대: 켠 동안에는 노트가 내려오는 시간과 미리보기·요약의 GREEN 이 그대로이고 HI-SPEED 만 바뀐다(HI-SPEED 행은 흐리고 `AUTO`). 끈 동안에는 HI-SPEED 가 그대로이고 노트 속도와 GREEN 이 바뀐다(GREEN NUMBER 행이 흐리고 `AUTO`). BPM 을 바꿀 때 검은 화면이나 READY 가 끼지 않고 콤보·점수가 이어진다.
- [ ] **I3 가림막과 숫자.** 방법: LANE COVER 를 → 로 한 칸씩, Shift + → 로 열 칸씩 올린다. LIFT ON·LIFT, HIDDEN+ ON·HIDDEN+ 도 같게 한다. 기대: 정보 패널의 `30.0% (WHITE 300)`(목록의 행은 `30.0%`. 웨이브 11 부터 한 칸이 0.1% 이고 Shift 가 1% 다), 요약의 WHITE, 미리보기 스킨의 커버 숫자가 모두 같은 값이다(299 처럼 하나 모자라지 않는다). 리프트 15% 의 스킨 숫자는 150 이다. 가림막과 판정선이 그 프레임부터 움직인다. 0 키가 그 행을 기본값으로 되돌린다.
- [ ] **I4 나머지 행.** 방법: SPEED FIX 를 CONSTANT 로, GAUGE 를 하나씩, TARGET 을 하나씩, AUTOPLAY, JUDGE OFFSET, AUTO ADJUST, LEGACY NOTE 를 바꾼다. 기대: CONSTANT 는 요약의 BPM 칸이 `120 (CONSTANT)` 이다. GAUGE 는 게이지 색이 바뀌고 진행 위치가 유지된다. TARGET 은 점수 그래프의 타깃 막대와 `TARGET +n` 이 바뀐다. AUTOPLAY 는 표지가 켜지고 꺼진다. JUDGE OFFSET 은 TIMING 숫자가 1 ms 씩(Shift 10 ms) 바뀌고 예시 판정은 그대로다. RANDOM·SCRATCH AUTO·AUTO REPLAY·ESCAPE 는 미리보기가 그대로인 것이 정상이다.
- [ ] **I5 모드와 스킨 옵션.** 방법: Tab 과 모드 띠의 클릭으로 7K 5K 14K 10K 9K 를 돈다. LANE 묶음의 LANE WIDTH 를 WIDE 로 바꾸고 SKIN 메뉴(F12)의 같은 행을 본다. ModernChic 을 팩으로 고른 뒤에도 연다. 기대: 모드마다 그 키 수의 플레이 화면이 그려지고 스킨 옵션 행이 그 문서의 것으로 바뀐다. LANE WIDTH 는 두 메뉴에서 같은 값이고 실제 플레이에도 같다. 문서가 없는 모드(ModernChic 의 9K 등)는 미리보기 자리에 `NO SKIN DOCUMENT FOR THIS SCREEN` 이 나오고 행은 그대로 조절된다. 모드와 PREVIEW BPM 은 메뉴를 다시 열면 7K·150 이다.
- [ ] **I6 마우스와 긴 목록.** 방법: 행을 왼쪽·오른쪽 클릭하고, 목록 위에서 휠을 돌리고, 미리보기와 묶음 이름을 클릭한다. PageUp·PageDown 을 쓴다. 창을 4:3, 21:9, 1920x1080 으로 바꾼다. 기대: 왼쪽 클릭은 한 칸 앞으로, 오른쪽 클릭은 뒤로 간다. 휠은 목록만 움직인다. 미리보기와 묶음 이름은 반응하지 않는다. 어느 창 크기에서도 글자가 잘리거나 겹치지 않고 미리보기가 16:9 로 그려진다.
- [ ] **I7 실제 플레이와 같은가.** 방법: PLAY 메뉴에서 유지 켬·커버 25%·리프트 20%·녹색 350 을 정하고 BPM 100 의 미리보기 숫자를 적은 뒤, BPM 100 인 곡을 플레이한다. 기대: 플레이 화면의 HI-SPEED·GREEN·커버·리프트 숫자가 미리보기와 같다. 메뉴에서 바꾼 값이 F1 오버레이와 선곡 패널 1~3 에도 같은 값으로 보인다.
- [ ] **I8 이전 키 설정.** 방법: 이 판 전의 `keyconfig.ron` 에서 선곡 단축키 하나(예: FOLDERS)를 F8 로 옮겨 둔 채 실행한다. 기대: F8 은 옮겨 둔 단축키로 동작한다. 키 설정 화면과 H 가이드에서 PLAY MENU 가 키 없음으로 나오고, SETTINGS 안내 줄이 `- PLAY` 이며 단추로는 열린다. 다른 키를 주면 그 키로 열린다.

### 3.10 웨이브 10C: 판정 표시와 부위별 불투명도

자동 검사는 헤드리스로 확인했다: 판정 줄이 0~496ms 동안 픽셀까지 같고 540ms 에 없음, 콤보 7·67·648·1234·12345 에서 글자가 같고 숫자가 글자 뒤 첫 칸에서 시작해 오른쪽으로만 자라며 필드 안에 있음, PGREAT·GREAT·GOOD·BAD·POOR 의 글자와 위 줄이 같은 열에서 시작함, 5·7·9·10·14키와 Wide·Field Right 의 모든 필드에서 글자 + 세 자리가 가운데이고 다섯 자리가 필드 안임, Judge Position ±100 에서 정확히 100px 이동, Judge Display Hide 에서 판정 줄 없음, 모든 부위 0% + Hide 에서 프레임이 한 색, SKIN 메뉴의 키만으로 부위를 내린 런의 프레임이 한 색이고 Esc 가 쓴 설정 파일이 같은 값으로 읽힘(`stage/capture/play_parts.rs`), 두 메뉴의 불투명도 행(퍼센트, 5%·25%, 0 키, 저장값, 다음 프레임 반영, 문서 재읽기 없음)과 접이, 객체 종류별 알파 오프셋(`crates/rbms-render` 의 opacity 테스트). 캡처는 PNG 로 열어 확인했다(설계서 8.4절). 아래는 실제 창과 실제 차트에서만 보이는 것이다.

- [ ] **J1 판정이 가만히 있는가.** 방법: 7키 곡을 손으로 플레이한다(오토플레이로도 한 번). 기대: 판정 글자와 콤보가 튀어나오거나 커지거나 깜빡이지 않고 같은 자리에 서 있으며 콤보 숫자만 바뀐다. 0.5초 동안 다음 판정이 없으면 한 번에 사라진다. PGREAT 는 하늘색 GREAT, GREAT 는 노란 GREAT 다. 콤보는 글자 바로 뒤에 붙고(`GREAT 67`) 앞자리 0 이 없으며, 10·100·1000 을 넘을 때 글자와 앞 숫자가 움직이지 않고 숫자가 오른쪽으로만 는다. BAD·POOR 가 뜰 때 글자의 왼쪽 끝이 GREAT 와 같은 자리다. 7키 기본 폭에서 글자 크기(60)가 작게 느껴지는지 적는다.
- [ ] **J2 판정 위 줄.** 방법: 일부러 빠르게, 늦게 친다. PLAY 메뉴(F8)의 CHART > TARGET 을 바꾸고, LANE 의 SCORE DIFFERENCE 를 BEST 와 OFF 로, JUDGE DETAIL 을 MILLISECONDS 와 OFF 로 바꿔 다시 친다. 기대: 판정 글자 바로 위 왼쪽에 `+0003` 같은 점수 차(부호와 네 자리), 오른쪽에 FAST(파랑)·SLOW(빨강)가 뜬다. 점수 차는 같은 순간 스코어 그래프의 TARGET(BEST 를 고르면 BEST) 숫자와 같다. 기록이 있는 곡에서 BEST 가 자기 최고 기록 대비인지 본다. 위 줄의 왼쪽 끝이 판정 글자의 왼쪽 끝과 맞는다. 둘 중 하나를 끄면 남은 쪽이 그 왼쪽 끝에서 시작한다.
- [ ] **J3 판정 위치와 숨김.** 방법: PLAY 메뉴 LANE 묶음의 JUDGE POSITION Y 를 → 와 Shift + → 로 올리고, ← 로 음수까지 내린다. JUDGE DISPLAY 를 HIDE 로 바꾼다. SKIN 메뉴(F12)의 같은 행을 본다. 그대로 플레이한다. 기대: + 면 판정 글자·콤보·위 줄이 함께 위로, - 면 아래로 간다. 미리보기가 누를 때마다 따라온다(검은 프레임 없이). 두 메뉴가 같은 값을 보이고 실제 플레이에서도 같은 자리다. HIDE 면 판정 줄이 전혀 없다. 0 키로 제자리. 5키·9키·14키(양쪽 필드)에서도 같은지 본다.
- [ ] **J4 노트와 판정만 남기기.** 방법: SKIN 메뉴에서 7K 를 고르고 PageDown 으로 목록 끝의 불투명도 행에 가서 Lane Background, Lane Divider, Bar Line, Progress Bar, Score Graph, Song Header, Speed Display, Timing 을 0% 로 둔다(슬라이더를 왼쪽 끝으로 끌거나, Enter 를 누르고 `0` 을 친 뒤 Enter. Shift + ← 는 10% 씩이라 열 번이다). Esc 로 저장하고 플레이한다. 기대: 검은 바탕에 노트, 키 빔, 폭발, 판정선, 판정 글자(와 남겨 둔 BGA·점수·판정 수·게이지)만 보인다. 사라진 부위의 가장자리 선이나 잔상이 없다. 앱을 껐다 켜도 그대로다. 각 행에서 0 키, 또는 RESET 으로 돌아온다.
- [ ] **J5 중간값.** 방법: Note, Key Beam, Lane Cover(커버를 켠 채), BGA(BGA 가 있는 곡)를 50% 와 25% 로 둔다(Key Display 행은 웨이브 13 에서 없어졌다). 기대: 눈에 절반·4분의 1 쯤으로 보인다(Judge Line 은 조금 더 진하다). 키를 누르고 뗄 때 빔이 번쩍이지 않는다. → 와 슬라이더는 100% 에서 멈춘다. Enter 를 누르고 `150` 을 쳐 넣으면 정보 패널의 값이 `150% (+128)` 로 읽히고(목록의 행은 `150%`, 손잡이는 오른쪽 끝) 화면은 달라지지 않는다. 0 키로 100% 로 돌아온다.
- [ ] **J6 PLAY 메뉴의 접이.** 방법: PLAY 메뉴 목록 끝의 SKIN 묶음에서 OPACITY 와 POSITION AND SIZE 를 Enter 와 클릭으로 편다. 기대: 펴는 순간 든 행이 화면에 들어온다(OPACITY 는 접이가 목록 첫 줄에 오고 아래로 행이 이어진다). 값이 `SHOWN (18)` 로 바뀐다(웨이브 13 에서 Key Display Opacity 가 없어져 19 에서 줄었다). 불투명도 행을 바꾸면 SKIN 메뉴의 같은 행이 같은 값이다. 14K 탭에서는 18 이 아니라 17 이다(Score Graph 없음).
- [ ] **J7 외부 팩.** 방법: ModernChic 을 팩으로 고르고 SKIN 메뉴 7K 의 목록 끝을 본다. 플레이한다. 기대: `明るさ` 다섯 행이 `100%` 로 나오고 → 와 슬라이더로는 올라가지 않는다. Enter 를 누르고 `150` 을 치면 정보 패널이 `150% (+128)` 로 읽히고 그 덮개가 밝아진다(이름의 0~255 는 괄호 안 숫자다. 200 을 치면 +255). 그 뒤 ← → 는 그 자리에서 1% 씩 움직인다. ModernChic 의 판정 연출은 전과 같다(기본 스킨의 Judge Display 등은 나오지 않는다).

### 3.11 웨이브 11: 숫자 행의 슬라이더와 직접 입력, 난이도표 저장과 다시 받기

자동 검사는 헤드리스로 확인했다: 세 화면(SKIN 메뉴, PLAY 메뉴, SETTINGS)의 모든 숫자 행에서 한 칸이 가장 잔 단위이고 Shift 가 열 칸임, 슬라이더의 양 끝이 범위의 양 끝임, 손잡이의 왼쪽 끝·가운데·오른쪽 끝을 누르고 떼어도 값이 그대로임, 끄는 동안 설정 파일을 쓰지 않고 뗄 때 한 번 씀, 끄는 중의 키가 끌기를 끝내고 값을 저장함(POLYPHONY 포함), 직접 입력의 확정·취소·범위 묶기·반올림·거절, 읽는 값으로의 설정이 저장값을 건드리지 않음, 불투명도 0~200% 의 저장 왕복, 표를 받으면 저장본이 생기고 저장본이 있으면 받기 0회, 깨진 저장본은 다시 받음, 다시 받기의 성공·실패·빈 응답, 받는 중 삭제에서 저장본이 다시 생기지 않음, 보던 레벨을 이름으로 따라감, F6 과 검색 중 규칙. 아래는 실제 창과 실제 서버에서만 보이는 것이다.

- [ ] **K1 슬라이더 끌기.** 방법: PLAY 메뉴(F8)에서 LANE COVER 의 슬라이더 트랙을 누르고, 누른 채 좌우로, 그리고 행 밖(미리보기 위, 창 밖)으로 끈다. 손잡이를 정확히 잡고 움직이지 않은 채 뗀다. SKIN 메뉴(F12)의 불투명도 행, SETTINGS(Tab)의 AUDIO > BGM VOL 과 POLYPHONY 에서도 같게 한다. 기대: 트랙을 누르면 손잡이가 그 자리로 오고 포인터를 따라온다. 행 밖으로 나가도 버튼을 뗄 때까지 이어지고 슬라이더 끝을 넘으면 끝값에 머문다. 손잡이를 잡기만 하고 떼면 값이 1도 바뀌지 않는다(HI-SPEED, 오프셋 ±2000 행에서도). 끄는 동안 미리보기의 커버·부위가 끊김 없이 따라오고(검은 프레임 없음) 볼륨은 소리가 바로 바뀐다. 끄는 행의 손잡이에 흰 테두리가 보인다.
- [ ] **K2 끌기의 끝.** 방법: BGM VOL 을 끄는 중에 창 밖에서 버튼을 뗀다. 끄는 중에 다른 창으로 초점을 옮겼다가(Cmd/Alt + Tab) 돌아와 포인터를 움직인다. POLYPHONY 를 끄는 중(버튼을 누른 채) Esc 를 누른다. 기대: 창 밖에서 떼어도 끌기가 끝난다. 버튼을 뗀 것을 앱이 못 본 경우(초점을 잃은 동안 뗌) 돌아와 포인터를 움직이면 값이 따라올 수 있는데, 다음 클릭이나 키에서 끌기가 끝나고 그때의 값이 저장된다(이 동작이 거슬리면 기록한다). Esc 로 나간 뒤 다시 들어오면 POLYPHONY 가 끌던 값이고 오디오가 그 값으로 다시 열린다.
- [ ] **K3 직접 입력.** 방법: 숫자 행에서 Enter 를 누르고 숫자를 친 뒤 Enter. 숫자 글자를 클릭해서도 연다. `42%`, `-5`, `999`, 빈 칸, `1.2.3` 을 넣어 본다. 입력 중 Esc, ↑ ↓, Tab, 다른 행 클릭을 해 본다. 한글 입력기를 켠 채, 넘패드로도 친다. 기대: 열면 지금 값이 선택된 채 채워져 있고 첫 글자가 값을 갈아 쓴다. 단위는 무시되고 범위를 넘으면 끝값이 된다. 숫자가 아니면 행에 `NOT A NUMBER` 가 1.5초 뜨고 값은 그대로다. Esc 는 입력만 취소하고(메뉴가 닫히지 않는다) ↑ ↓·Tab·클릭은 확정한 뒤 움직인다. 입력 상자의 글자와 커서가 행 안에 들어가고 바닥(SETTINGS 는 제목 아래) 안내가 `TYPE A NUMBER …` 로 바뀐다. 입력기가 켜져 있어도 숫자가 들어간다(안 들어가면 기록한다: 붙여넣기와 입력기는 받지 않는 것이 알려진 한계다).
- [ ] **K4 가장 잔 단위와 Shift.** 방법: HI-SPEED, LANE COVER, JUDGE OFFSET, 불투명도, 볼륨, JUDGE WIDTH, PREVIEW FADE, TOTAL 에서 → 한 번과 Shift + → 한 번을 누른다. 0 키와 오른쪽 클릭을 누른다. 키를 누르고 있어 본다. 기대: 한 칸이 0.01 / 0.1% / 1 ms / 1% / 1% / 1% / 1 ms / 1 이고 Shift 는 그 열 배다. 0 키는 기본값, 오른쪽 클릭은 한 칸 뒤로. 키를 누르고 있어도 반복되지 않는다(먼 값은 슬라이더나 직접 입력으로 간다). SETTINGS 의 숫자 행 위에서도 제목 아래 안내에 `F8 PLAY   F12 SKINS   F5 KEYS` 가 보이고 그 키들이 듣는다.
- [ ] **K5 읽기 전용 행과 휠.** 방법: KEEP GREEN NUMBER 를 켜고 HI-SPEED 행을 클릭·끌기·Enter 한다. 슬라이더 위에서 휠을 돌린다. 기대: HI-SPEED 행은 흐리고 어떤 조작에도 움직이지 않으며 입력 상자가 열리지 않는다. 휠은 목록만 스크롤하고 값은 그대로다.
- [ ] **K6 표가 저장되는가.** 방법: Tables(선곡에서 T)에서 `+ ADD TABLE (URL)` 로 `https://darksabun.club/table/archive/insane1/` 을 등록한다. 앱을 끄고 네트워크를 끊은 뒤(Wi-Fi 끔) 다시 켠다. 기대: 등록해도 LOADING 으로 넘어가지 않는다(웨이브 12. 행이 바로 생기고 `FETCHING...` 이 보인다: L3). 표가 도착하면 `~/.config/rbms/tables/` 에 `.json` 파일 하나가 생긴다. 네트워크 없이 켜도 MATCHING TABLES 가 바로 지나가고(1초 안) 표 폴더와 레벨이 그대로 나온다. Tables 의 그 행 끝에 `SAVED JUST NOW`(시간이 지나면 `SAVED 5 MINUTES AGO`, `SAVED 2 HOURS AGO`, `SAVED 3 DAYS AGO`)가 보이고, 일본어 표 이름과 한 줄에 겹치지 않는다.
- [ ] **K7 다시 받기.** 방법: 네트워크를 켠 채 Tables 에서 그 표에 커서를 두고 R, 표가 여럿이면 A. 선곡에서 F6. 받는 동안 커서를 움직이고 화면을 드나든다. 기대: 화면이 멈추지 않는다. 행에 흰 `RELOADING...` 이 뜨고(웨이브 15 전에는 노랑) 목록 아래에 `FETCHING THE TABLE AGAIN …` 줄이 나온다. 끝나면 `difficulty tables reloaded: N updated, M failed` 알림이 뜨고 `SAVED JUST NOW` 가 된다. 선곡에서 F6 을 눌렀을 때는 선곡을 그대로 쓸 수 있고, 표의 레벨 폴더 안을 보고 있었다면 같은 레벨에 그대로 있다. 받는 중에 R·A·F6 을 또 눌러도 `the tables are being fetched already` 만 뜬다(웨이브 12 에서 문구가 바뀌었다). H 가이드와 키 설정 화면에 `F6 RELOAD DIFFICULTY TABLES` 가 있다.
- [ ] **K8 다시 받기 실패와 삭제.** 방법: 네트워크를 끊고 R 을 누른다. 표를 하나 D 로 지우고 `tables/` 폴더를 본다. R 을 누른 직후 D 로 지워 본다. 기대: 실패하면 행이 `RELOAD FAILED - …` 가 되고 아래에 사유가 나오며, 표 폴더와 레벨, `SAVED …` 는 그대로다. 지운 표의 `.json` 이 폴더에서 사라진다. 받는 중에 지운 표는 되살아나지 않고 그 파일도 다시 생기지 않으며, 알림이 그 표를 `updated` 로 세지 않는다.
- [ ] **K9 오래된 저장본.** 방법: `tables/` 의 저장본 한 개에서 `"fetched_at"` 값을 40일 전의 유닉스 시각으로 고치고 켠다(또는 30일 뒤에 본다). 기대: 그 행의 `SAVED 40 DAYS AGO` 가 경고색이고, 커서를 올리면 아래에 `THE SAVED COPY IS 40 DAYS OLD - R FETCHES THE TABLE AGAIN` 이 나온다. 표는 스스로 다시 받아지지 않는다.

### 3.12 웨이브 12: 저장본 없는 표를 선곡 뒤에서 받기, 연습 패널의 슬라이더와 직접 입력

자동 검사는 헤드리스로 확인했다: 로딩의 표 단계가 어느 주소에도 묻지 않음(받기 0회), 저장본 없는 표는 선곡의 첫 프레임에 한 번 받기 시작하고 도착하면 폴더가 순서대로 끼어들며 커서가 보던 폴더를 따라감, 다른 표의 레벨 안·필터가 걸린 목록·검색 결과는 움직이지 않음, 검색을 나가면 검색을 시작한 폴더(레벨은 이름으로)로 돌아감, 실패한 주소는 그 실행에서 다시 받지 않고 알림이 한 줄임, 새 등록이 로딩으로 가지 않고 행이 바로 생김, 받는 중 삭제·재등록·중복 등록·재스캔·R·A·F6, 연습 패널 숫자 행 여섯의 한 칸·Shift·0 키·범위 끝, 슬라이더 누름·끌기·손잡이 누름, 직접 입력(초, 분:초, 범위 밖, 거절, Esc), 두 시각의 제약, START TIME 을 END TIME 너머로 끌었다 돌아오면 END TIME 이 제자리임, TOTAL 슬라이더가 1000 에서 끝남, Tab, 세 크기(1280x720, 1920x1080, 1280x1024)에서 행의 클릭 영역과 손잡이 픽셀. 아래는 실제 창과 실제 서버에서만 보이는 것이다.

- [ ] **L1 응답 없는 표 주소로 시작.** 방법: 격리한 HOME 의 Tables 에서 `+ ADD TABLE (URL)` 로 `https://10.255.255.1/table/`(응답이 없는 주소)을 등록하고 앱을 껐다 켠다. 기대: 로딩의 MATCHING TABLES 가 바로 지나가고 선곡이 뜬다(전에는 여기서 15초를 기다렸다). 선곡이 뜬 직후 `fetching 1 difficulty table(s) for the first time` 알림이 뜨고, 그동안 커서 이동·폴더 드나들기·플레이 시작이 평소처럼 된다. 약 15초 뒤 `table fetch failed (https://10.255.255.1/table/): …` 알림이 **한 줄만** 뜬다(집계 줄이 따로 붙지 않는다). 그 뒤 폴더 화면(O)에서 Esc 로 재스캔하거나 화면을 드나들어도 다시 받지 않고, 알림도 되풀이되지 않는다. Tables 의 그 행은 `LOAD FAILED` 이고 커서를 올리면 아래에 사유가 나온다. R 을 누르면 다시 시도한다.
- [ ] **L2 표 폴더가 뒤늦게 나타남.** 방법: 격리한 HOME 에 실제 표 주소(`https://darksabun.club/table/archive/insane1/`) 하나와 이미 저장본이 있는 표 하나를 두고, `tables/` 에서 앞의 표의 `.json` 만 지운 뒤 켠다. 선곡이 뜨면 저장본이 있는 표의 폴더에 커서를 둔다. 같은 일을 표의 레벨 폴더 안에서, ALL SONGS 안에서, 검색어를 치는 중(`/`)에 한 번씩 한다. 기대: 선곡이 표를 기다리지 않고 뜬다. 몇 초 뒤 `difficulty tables fetched for the first time: 1 added, 0 failed` 알림과 함께 폴더가 등록 순서의 자리에 나타나고, 커서는 보고 있던 폴더에 그대로 있다(한 칸 밀려 다른 폴더로 가지 않는다). 레벨 안·ALL SONGS·검색 결과에서는 목록과 커서가 움직이지 않고, 검색을 Esc 로 나가면 검색을 시작한 폴더에 커서가 있다. 목록이 깜빡이거나 곡 미리듣기가 끊기지 않는다(끊기면 기록한다). 다음 실행에는 그 표가 저장본에서 바로 읽힌다.
- [ ] **L3 새 등록.** 방법: Tables 에서 `+ ADD TABLE (URL)` 로 표 주소를 넣고 Enter. 바로 Esc 로 선곡에 나간다. 한 번은 Tables 에 남아 지켜본다. `+ ADD TABLE (FILE)` 로 `data.json` 도 등록한다. 틀린 주소도 넣어 본다. 기대: 어느 것도 LOADING 화면으로 넘어가지 않는다. 행이 바로 생기고 커서가 그 행에 가며 노란 `FETCHING...`(파일은 `READING...`)과 목록 아래의 `FETCHING THE TABLE - ITS FOLDER APPEARS IN THE SONG LIST WHEN IT ARRIVES` 가 보인다. 도착하면 행이 `N OF M CHARTS OWNED - K LEVELS` 로 바뀌고 `table added: 이름 (N local charts)` 알림이 뜬다. 선곡에 나가 있었다면 거기서 폴더가 나타난다. 틀린 주소는 행이 `LOAD FAILED` 로 남고 사유가 나오며 D 로 지울 수 있다. 받는 중(`FETCHING...`)에 D 로 지우면 `tables/` 에 그 표의 파일이 생기지 않는다. 바닥 안내 세 줄이 화면 밖으로 나가거나 행과 겹치지 않는다.
- [ ] **L4 받는 중 종료.** 방법: L1 의 상태(응답 없는 주소를 받는 중)에서 15초가 되기 전에 창을 닫는다. 기대: 창이 바로 닫힌다(받기를 기다리지 않는다). `tables/` 에 반쯤 쓰인 파일이나 이름이 이상한 임시 파일이 남지 않는다.
- [ ] **L5 연습 패널의 슬라이더.** 방법: 선곡에서 곡에 커서를 두고 F4. START TIME, END TIME, GAUGE VALUE, JUDGERANK, TOTAL, FREQUENCY 의 슬라이더 트랙을 누르고, 누른 채 좌우로, 행 밖과 창 밖으로 끈다. 손잡이를 정확히 잡고 움직이지 않은 채 뗀다. 전체 화면과 16:9 가 아닌 창 크기에서도 한다. 기대: 트랙을 누르면 손잡이가 그 자리로 오고 포인터를 따라온다(레터박스가 있어도 누른 자리와 손잡이가 어긋나지 않는다). 끄는 행은 트랙이 노랗고 손잡이에 흰 테두리가 보인다. 손잡이를 잡기만 하고 떼면 값이 그대로다. END TIME 은 START TIME + 1초에서 멈추고 더 왼쪽으로는 손잡이가 따라오지 않는다. TOTAL 은 차트의 값(예: 260)에서 손잡이가 트랙의 4분의 1쯤에 있고 슬라이더 오른쪽 끝이 1000 이다. 휠은 아무것도 바꾸지 않는다.
- [ ] **L6 START TIME 을 END TIME 너머로.** 방법: START TIME 0:20.0, END TIME 0:40.0 으로 맞춘다. START TIME 의 손잡이를 잡고 END TIME 을 지나 오른쪽 끝까지 끌었다가, 버튼을 떼지 않고 원래 자리로 돌아온다. 그 다음 Shift + → 를 여러 번 눌러 START TIME 을 0:45.0 까지 올렸다가 Shift + ← 로 내린다. 기대: 끄는 동안 END TIME 은 START TIME + 1초로 밀려 올라갔다가, START TIME 이 0:39.0 아래로 돌아오면 0:40.0 으로 돌아온다(구간 끝이 사라지지 않는다). 키로 올렸다 내린 경우에는 END TIME 이 0:46.0 에 남는다(원본과 같은 동작. 헷갈리면 기록한다).
- [ ] **L7 연습 패널의 직접 입력.** 방법: 숫자 행에서 Enter, 또는 숫자 글자를 클릭한다. START TIME 에 `90`, `1:30.5`, `2:5`, `1:75` 를, TOTAL 에 `3000`, `99999`, 빈 칸, `-` 를 넣는다. 입력 중 Esc, ↑ ↓, Tab, Shift + Tab, 다른 행 클릭을 해 본다. 한글 입력기를 켠 채로도 친다. 기대: 열면 지금 값(시각은 `0:20.0` 꼴)이 선택된 채 채워져 있고 안내 줄이 `TYPE SECONDS OR MIN:SEC …`(시각 행) 또는 `TYPE A NUMBER …` 로 바뀐다. `90` 은 1:30.0, `2:5` 는 2:05.0, `1:75` 는 2:15.0 이 된다. TOTAL `3000` 은 3000 이 되고 손잡이가 슬라이더 끝에 서며 ← → 가 거기서 한 칸씩 움직인다. `99999` 는 5000. 숫자가 아니면 `NOT A NUMBER` 가 1.5초 뜨고 값은 그대로다. Esc 는 입력만 취소하고 패널은 닫히지 않으며, 두 번째 Esc 가 선곡으로 나간다. ↑ ↓ 와 Tab 은 확정한 뒤 커서를 옮긴다.
- [ ] **L8 연습 패널의 Enter 와 START 행.** 방법: 패널을 연 직후 Enter. 다시 열어 ↓ 로 START TIME 에 가서 Enter, GAUGE TYPE 에 가서 Enter. START 행을 왼쪽 클릭, 오른쪽 클릭. 구간이 끝나거나 Esc 로 구간을 나온 뒤 패널을 본다. 패널을 나갔다 같은 곡으로 다시 연다. 기대: 열면 커서가 맨 위 START 행에 있고 Enter 가 구간을 시작한다. START TIME(숫자 행)의 Enter 는 구간을 시작하지 않고 입력 상자를 연다(전에는 어느 행에서나 시작이었다. 불편하면 기록한다: 결정 대기 항목이다). GAUGE TYPE(고르기 행)의 Enter 는 구간을 시작한다. START 행 왼쪽 클릭은 시작, 오른쪽 클릭은 커서만. 구간에서 돌아오면 값이 그대로이고, 다시 열어도 그대로다(`practice.ron`). 구간 플레이의 시작·끝·게이지·판정 폭·속도·TOTAL 이 패널의 값과 같다.

### 3.13 웨이브 13: 판정선을 필드 맨 아래로, 판정 오프셋이 노트만 옮김

자동 검사는 헤드리스로 확인했다: 5·7·9·10·14키와 Lane Width·Field Side·Scratch Side 22조합에서 판정선의 아래 변이 레인 면·구분선의 아래 끝이고 그 아래 12px 이 화면 바탕 한 색임, 오프셋 0·+30·-30·+100·-100 에서 소리의 시각과 PGREAT 창(노트의 시각 ±20 ms)이 그대로이고 노트가 판정선에 닿는 시각만 오프셋만큼 앞섬(스킨 필드와 내장 필드 둘 다), 박자에 친 입력이 어느 오프셋에서도 오차 0 의 PGREAT 이고 키음이 즉시 남, BGA 가 넘어가는 시각이 오프셋 0·+100·-100 에서 같음, 자동 보정의 걸음과 부호(40 ms 빠르면 -1 씩 -26 까지), 스키마 4 이하 설정의 부호가 한 번만 뒤집힘, 웨이브 12 빌드로 만든 리플레이 셋이 그때의 집계로 재생됨. 아래는 실제 창·실제 손·실제 소리에서만 드러나는 것이다. 값의 뜻은 `docs/play-options.md` 3절.

- [ ] **M1 판정선이 필드 맨 아래에 붙는다.** 방법: 기본 스킨으로 7K 곡을 플레이한다. 5K, 9K, 10K, 14K 도 한 번씩, 7K 는 Lane Width Wide·Field Side Right·Scratch Side Right 도 본다. 창 크기를 1280x720, 1920x1080, 전체 화면, 16:9 가 아닌 크기로 바꿔 본다. 기대: 판정선(굵은 하늘색 띠)이 레인의 맨 아래 끝이다. 판정선 아래로 레인이 이어지는 칸(전의 건반 표시)이 없고 바로 아래에 게이지가 온다. 어느 창 크기에서도 판정선과 게이지 사이에 레인 색 줄이 한 줄도 남지 않는다. 노트가 판정선에서 사라지고 그 아래로 내려가지 않는다(9K 도).
- [ ] **M2 LIFT 와 커버.** 방법: PLAY 메뉴(F8) > LANE 에서 LIFT 를 20% 로 켜고, LANE COVER 30%, HIDDEN+ 20% 도 켠 채 플레이한다. 플레이 중 `]` `[` 로 리프트를 움직인다. 기대: 판정선이 필드 아래 끝에서 리프트만큼 올라가고, 그 아래는 리프트 면이 필드 끝까지 덮는다(레인이 비쳐 보이지 않는다). 리프트 수치가 필드 아래 끝 바로 위에, 히든 수치가 히든 커버 위쪽에, 커버 수치와 녹색 숫자가 레인 커버 끝에 보인다. 판정 글자·키 빔·폭발이 판정선을 따라 올라간다.
- [ ] **M3 폭발의 모양.** 방법: 플레이하며 판정선 위의 폭발 고리를 본다. LN 을 누르고 있는 동안의 폭발도 본다. 기대(웨이브 14 에서 정해졌다: 온전한 고리를 게이지 위에 겹쳐 그린다): 고리가 판정선에 중심을 둔 온전한 원이고 아래 절반이 게이지 위에 비쳐 그려진다. 밀집 구간에서 게이지가 읽기 어려우면 기록한다(Bomb Opacity 로 낮출 수 있다).
- [ ] **M4 오프셋을 준 플레이에서 소리·노트·판정.** 방법: 키음이 뚜렷한 곡을 고른다. (1) PLAY 메뉴 > JUDGE 의 JUDGE OFFSET 을 0 으로, AUTO ADJUST 를 끄고 한 판 친다. (2) JUDGE OFFSET 을 +30 으로 올려 같은 곡을 친다. (3) -30 으로 내려 친다. (4) +100 과 -100 으로 같은 구간을 오토플레이(AUTOPLAY 켬)로 본다. 각 판에서 곡의 박자(BGM)에 맞춰 치는 것과 노트가 판정선에 닿는 것을 보고 치는 것을 나눠 해 본다. 기대: 어느 값에서도 박자에 맞춰 친 입력은 PGREAT 이고 친 노트의 키음이 곡과 어긋나 들리지 않는다(웨이브 12 까지는 값을 줄수록 박자에 친 입력이 FAST·SLOW 가 되고, PGREAT 를 내려고 어긋나게 치면 키음도 어긋났다: 보고된 증상). +30 에서는 노트가 박자보다 조금 먼저 판정선에 닿고(노트를 보고 치면 FAST 쪽), -30 에서는 조금 늦게 닿는다(SLOW 쪽). 플레이 화면의 TIMING 숫자가 `+30`·`-30` 이다. 오토플레이의 +100 은 노트가 판정선에서 사라진 조금 뒤에 폭발과 키음이 나고, -100 은 폭발이 난 뒤에도 노트가 판정선까지 내려온다(원본과 같은 동작이다. 거슬리면 기록한다). 0 으로 되돌리면 노트·소리·폭발이 한 순간에 맞는다.
- [ ] **M5 자기 손에 맞는 값 찾기.** 방법: JUDGE OFFSET 0, AUTO ADJUST 끔으로 한 판 치고 결과의 FAST·SLOW 수와 평균 타이밍을 본다. SLOW 가 많으면 값을 올리고(+) FAST 가 많으면 내린다(-). 키 설정(F5)에서 `JUDGE OFFSET +1 MS (HITTING LATE)`·`-1 MS (HITTING EARLY)` 에 키를 주고 플레이 중에 눌러 본다. 선곡에서 패널 3(START+SELECT)의 5·7번 키도 누른다. 기대: 늦게 치는 사람이 값을 올리면 노트가 먼저 와서 FAST·SLOW 가 고르게 된다. 패널 3 의 5번 칸은 `NOTES 1 MS LATER`, 7번 칸은 `NOTES 1 MS EARLIER` 이고 값이 -1·+1 된다. PLAY 메뉴의 도움말, 키 설정의 행 이름, 패널 3 의 문구가 같은 방향을 말한다.
- [ ] **M6 자동 보정.** 방법: AUTO ADJUST 를 켜고 노트를 보며 일부러 조금 빠르게(다른 판은 늦게) 친다. 한 판은 화면을 보지 않고 소리만 듣고 친다. 기대: 표지가 `TIMING AUTO` 이고, 빠르게 치면 숫자가 내려가고(-) 늦게 치면 올라간다(+). 노트를 보고 치는 판은 몇십 타 안에 숫자가 멈춘다. 숫자가 바뀌는 순간 노트 전체가 몇 픽셀 튀는 것이 보일 수 있다(원본과 같다. 거슬리면 기록한다). 소리만 듣고 치는 판은 숫자가 멈추지 않고 계속 한쪽으로 간다(원본 규칙. 이렇게 치는 사람은 AUTO ADJUST 를 끈다). 오토플레이와 리플레이에서는 숫자가 움직이지 않는다.
- [ ] **M7 이전 설정과 이전 리플레이.** 방법: 이 판을 설치하기 전에 JUDGE OFFSET 을 0 이 아닌 값(예: +26)으로 두고 저장한 `settings.ron` 과 그때 친 리플레이를 둔 채 실행한다. 계정 설정 동기화를 쓰면 다른 기기의 이전 판 설정도 내려받아 본다. 기대: 처음 켰을 때 JUDGE OFFSET 이 부호만 뒤집혀 `-26` 으로 보이고, 노트와 PGREAT 사이의 간격(치는 느낌)은 전과 같다. 설정 폴더에 `settings.ron.v4.bak` 이 생긴다. 다시 켜도 값이 또 뒤집히지 않는다. 이전 리플레이가 그때와 같은 판정 수로 재생된다. 리플레이를 보고 나와도 내 JUDGE OFFSET 이 리플레이의 값으로 바뀌지 않는다(전에는 바뀌었다).
- [ ] **M8 BGA 와 오프셋.** 방법: BGA(그림 여러 장 또는 동영상)가 박자에 맞춰 넘어가는 곡을 JUDGE OFFSET 0, +200, -200 으로 오토플레이한다. 기대: BGA 가 넘어가는 순간이 세 값에서 모두 곡(소리)에 맞는다. 노트만 앞서거나 늦는다. 미스 레이어도 놓친 순간에 뜬다.
- [ ] **M9 PLAY 메뉴의 미리보기.** 방법: PLAY 메뉴(F8) > JUDGE 의 JUDGE OFFSET 을 Shift + → 로 +100 까지, Shift + ← 로 -100 까지 움직이며 미리보기를 본다. 기대: 미리보기의 노트 전체가 값만큼 아래(+)·위(-)로 옮겨지고 TIMING 숫자가 따라온다. 예시 판정(GREAT·콤보)은 달라지지 않는다. +에서는 노트가 판정선에서 사라진 뒤에 폭발이 뜬다.

### 3.14 웨이브 14: 플레이 중 조작, 모노톤 스킨과 색 옵션, 내보내기

자동 검사는 헤드리스로 확인했다: START + 스크래치 키 한 번이 누른 길이(16~250 ms)와 무관하게 흰 숫자 1, 출하 키(`LSHIFT`·`LCTRL`)로 양방향, START + SELECT 로 리프트 1 씩 양방향, 누르고 있을 때의 반복 시각과 범위 끝, 녹색 숫자 불변(KEEP GREEN NUMBER 켬 / HI-SPEED FIX MAIN / OFF, 각각 CONSTANT 포함), 레인 커버를 끝까지 내려 하이스피드가 0.01 이 된 뒤 START 를 뗐다 다시 눌러 되돌려도 원래 속도로 돌아옴, `9`·`0` 의 누름과 반복, 옛 키 설정 파일이 `9`·`0`·`LCTRL`·`RCTRL` 을 겹치지 않을 때만 읽음, 기본 스킨의 바탕이 (0,0,0) 이고 색 옵션 65조합이 경고 없이 그려짐, 내보낸 폴더가 팩의 파일 전부와 조절값 파일 하나임, 내보낸 기본 팩을 새 설정에서 연 플레이 프레임이 조절한 프레임과 픽셀까지 같음(오프셋·불투명도 포함), THEME·ACCENT 를 한 화면에서 바꿔 여덟 화면에 적용하고 내보내면 아홉 화면이 그 값으로 열림. 아래는 실제 창·실제 손에서만 드러나는 것이다. 값의 뜻은 `docs/play-options.md` 5절, `docs/skin.md` 2.8·3.4.3절.

- [ ] **N1 START + 턴테이블로 레인 커버를 1 씩.** 방법: 기본 스킨으로 7K 곡을 손으로 친다(AUTOPLAY 끔). 레인 커버를 켠 채 `A` 를 누르고 있으면서 왼쪽 Shift 를 짧게 여러 번, 왼쪽 Ctrl 을 짧게 여러 번 누른다. 그다음 Shift 를 3초 누르고 있는다. 기대: Shift 한 번에 레인 커버 끝의 흰 숫자가 정확히 1 늘고(더 가림) Ctrl 한 번에 1 준다. 그동안 녹색 숫자는 그대로이고 발치의 HI-SPEED 가 따라 바뀐다. 발치에 COVER 와 LIFT 수치가 뜬다. 누르고 있으면 0.3초 뒤부터 움직이고 점점 빨라진다. `A` 를 떼면 약 1.5초 뒤 COVER·LIFT 수치가 사라진다. 스크래치 노트가 내려오는 구간에서 하면 그 입력이 스크래치도 친다(원본과 같다).
- [ ] **N2 START + SELECT + 턴테이블로 리프트를 1 씩.** 방법: `A` 와 `W` 를 함께 누른 채 왼쪽 Ctrl 을 여러 번(올림), 왼쪽 Shift 를 여러 번(내림) 누른다. 리프트가 꺼진 채로도, 켠 채로도 해 본다. 3초 이상 누르고 있어 본다. 기대: 리프트가 꺼져 있어도 발치의 LIFT 수치가 1 씩 바뀐다(리프트는 켜지지 않는다). 켜져 있으면 판정선이 1 씩 움직이고 판정선 아래 수치도 바뀐다. 레인 커버·녹색 숫자·HI-SPEED 는 그대로다. 오래 눌러도 곡이 끝나지 않는다. `W` 만 떼도 레인 커버가 꺼지지 않는다. `A` 를 0.5초 안에 두 번 누르면 전과 같이 레인 커버가 켜지고 꺼진다.
- [ ] **N3 실제 턴테이블(컨트롤러).** 방법: 아날로그 턴테이블이 있는 컨트롤러로 START 를 누른 채 디스크를 천천히, 빠르게 양쪽으로 돌린다. START + SELECT 로도 돌린다. 기대: 틱마다 1 씩 움직이고 방향이 디스크와 맞는다(자동 검사는 주입한 틱으로만 확인했다). START 를 누르기 전에 돌리던 것은 반영되지 않는다.
- [ ] **N4 판정 오프셋 키.** 방법: 플레이 중 `0` 을 세 번, `9` 를 세 번 누르고, `0` 을 2초 누르고 있는다. 선곡에서 `0`(검색)과 `9` 가 전과 같이 동작하는지도 본다. 기대: 누를 때마다 오른쪽 아래 TIMING 이 +1 / -1 되고 노트 전체가 그만큼 옮겨진다(+ 면 노트가 먼저 온다). 누르고 있으면 0.3초 뒤부터 초당 20 씩 가고 ±200 에서 멈춘다. 알림이 플레이를 가리지 않는다. 오토플레이와 리플레이에서는 눌러도 바뀌지 않는다.
- [ ] **N5 기존 키 설정 파일.** 방법: 이 판을 설치하기 전의 `keyconfig.ron`(스크래치를 다른 키로 옮긴 것이면 더 좋다)을 둔 채 실행하고 선곡에서 `H`, `F5` 를 본다. `F5` 에서 SCRATCH 8 REVERSE 행의 키를 `Del` 로 떼고 저장한 뒤 다시 켠다. 기대: 가이드의 IN PLAY 에 `9 0  JUDGE OFFSET -1 / +1 MS (HOLD)`, HELD TOGETHER 첫 줄에 스크래치 키와 `LCTRL` 이 `THE TURNTABLE: DOWN / UP` 으로, KEYS 묶음의 PREVIOUS BAR 에 `LCTRL` 이 나온다. 그 키들을 이미 다른 데 주었던 파일에서는 해당 칸이 `-` 다. 뗀 키는 다시 켜도 `-` 로 남는다. 내가 바꿔 둔 다른 키는 그대로다.
- [ ] **N6 왼쪽 Ctrl 이 선곡에서 목록을 올린다.** 방법: 기본 스킨의 선곡에서 왼쪽 Ctrl 을 누르고, 누르고 있는다. `/` 로 검색어를 치는 중에 Ctrl+V 로 붙여넣는다. Settings 의 글자 칸에서도 Ctrl+V 를 쓴다. 기대: 선곡에서는 왼쪽 Shift 가 아래로, 왼쪽 Ctrl 이 위로 한 칸씩 옮긴다(원본과 같다). 검색어와 글자 칸에서는 붙여넣기만 되고 목록이 움직이지 않는다. 거슬리면 기록한다(`F5` 에서 뗄 수 있다).
- [ ] **N7 모노톤 화면.** 방법: 설정을 비운 첫 실행으로 선곡 → 결정 → 플레이 → 결과를 돈다. 밝기가 다른 모니터 둘, 어두운 방과 밝은 방에서 본다. 기대: 바탕이 순수한 검정이고 면·선·글자에 파란 기가 없다. 면의 단계(바탕 / 패널 / 올라온 면)가 서로 구분되고 작은 회색 글자가 읽힌다. 노트·키 빔·폭발·판정 글자·FAST/SLOW·램프·난이도·게이지·그래프·GREEN 숫자는 전과 같은 색이다. 선택한 곡이 흰 표시로 분명히 가려진다. 단계가 묻히면 어느 화면의 무엇인지 기록한다.
- [ ] **N8 색 옵션과 모든 화면에 적용.** 방법: SKIN 메뉴(F12)의 7K 탭에서 THEME 을 SLATE 로 바꾸고 `A`, ACCENT 를 AMBER 로 바꾸고 `A` 를 누른다. Tab 으로 다른 화면 탭을 돌아본다. NOTE COLORS 를 WHITE-YELLOW, BOMB COLOR 를 WHITE 로 바꾸고 각각 `A`. Esc 로 나가 실제로 선곡 → 플레이 → 결과를 돈다. PLAY 메뉴(F8)의 LANE 묶음에서도 같은 행과 `A` 를 써 본다. 기대: THEME·ACCENT 행에 커서를 두면 미리보기 아래에 `ALSO ON 8 OTHER SCREENS - A APPLIES THIS VALUE TO THEM ALL` 이 뜨고, `A` 뒤에 `APPLIED TO 8 OTHER SCREENS` 가 나온다(NOTE COLORS 3, BOMB COLOR 4). 모든 탭의 미리보기와 실제 화면이 고른 색이다. THEME SLATE + ACCENT SKY 는 웨이브 13 까지의 화면과 같다. ACCENT WHITE 에서 판정선과 흰 건반 노트가 같은 색인 것이 거슬리면 기록한다.
- [ ] **N9 앱이 덧그리는 UI 의 색.** 웨이브 15 에서 (나)로 정해 앱 테마의 기본값을 무채색으로 바꿨다(D18). 확인 항목은 3.15절 O1~O5 다.
- [ ] **N10 내보내기.** 방법: N8 의 상태에서 플레이 탭의 불투명도 몇 개와 JUDGE POSITION Y 도 바꾼 뒤 SKIN 메뉴 맨 아래 EXPORT 에서 Enter. 뜨는 창에서 바탕화면 같은 폴더를 고른다. 같은 폴더로 한 번 더 내보낸다. 팩 폴더 자신을 골라도 본다. 큰 팩(ModernChic)으로도 내보내며 진행 중에 Esc 를 눌러 본다. 기대: 창의 제목이 `Select folder to export the skin into` 다. 창이 떠 있는 동안 메뉴가 멈추지 않는다. 고른 폴더에 `rbms-default-export` 가 생기고(두 번째는 `-export-2`) 미리보기 아래에 `EXPORTED 66 FILES (2 MB) TO <경로>` 가 뜬다(설치된 기본 팩 65개 + 조절값 파일 1개. 저장소의 `assets/skins/rbms-default` 를 `RBMS_SKIN_PACK` 으로 지정해 내보내면 `tools/` 8개가 더해져 74개다). 팩 폴더 안을 고르면 `EXPORT FAILED` 와 이유가 나온다. 진행 중 Esc 는 메뉴를 떠나지 않고 취소하며 반쯤 쓴 폴더가 남지 않는다. 원래 팩 폴더는 바뀌지 않는다.
- [ ] **N11 내보낸 팩을 새 설정에서 열기.** 방법: `HOME=$(mktemp -d)` 로 새 설정을 만들어 실행하고 SKIN 메뉴의 PACK FOLDER 에서 N10 의 폴더를 고른다. 선곡 → 플레이 → 결과를 돈다. 행 하나를 바꾸고, 그 행에서 `0`, 그리고 RESET 을 눌러 본다. 다른 컴퓨터(다른 OS)로 폴더를 옮겨서도 해 본다. 기대: 모든 화면이 N8·N10 에서 맞춘 그대로다(색, 노트 색, 불투명도, 판정 위치까지). 메뉴의 행도 그 값을 보인다. 바꾼 행은 내 값으로 저장되고 `0` 과 RESET 은 스킨 제작자 기본값이 아니라 내보낸 값으로 돌아간다. 폴더 안의 `rbms-adjustments.ron` 을 문서 편집기로 열면 옵션 이름과 값이 읽히고 내 사용자 이름이나 절대 경로가 없다.
- [ ] **N12 깨진 조절값 파일.** 방법: N10 의 폴더에서 `rbms-adjustments.ron` 의 괄호 하나를 지우고 그 폴더를 팩으로 연다. 기대: 알림이 한 번 뜨고 스킨은 제작자 기본값(모노톤)으로 그려진다. 앱이 죽지 않는다.

### 3.15 웨이브 15: 앱 자체 UI 의 무채색 팔레트

배경: 사용자 요청 "배경색의 기본 컬러는 검은색, 전체적으로 검/회색/흰의 모노톤"을 앱이 직접 그리는 화면까지 넓혔다(D18). 헤드리스 캡처와 픽셀 통계로는 확인했고, 아래는 실제 창과 실제 설정 폴더에서 봐야 하는 것이다. 값과 대비 계산은 `docs/theme.md` 에 있다.

- [ ] **O1 메뉴와 스킨이 한 벌로 보이는가.** 방법: 기본 스킨의 선곡에서 F12(SKIN 메뉴), F8(PLAY 메뉴), F5(키 설정), Tab(설정), T(Tables), O(Folders)를 차례로 연다. 밝기가 다른 모니터 둘, 어두운 방과 밝은 방에서 본다. 기대: 어느 화면도 바탕이 순수한 검정이고 면·선·글자에 파란 기나 노란 기가 없다. 메뉴의 면(패널, 고른 행)이 스킨 화면의 면과 같은 회색으로 보이고, 스킨에서 메뉴로 넘어갈 때 색온도가 바뀌어 보이지 않는다. 어긋나 보이면 어느 화면의 무엇인지 기록한다.
- [ ] **O2 고른 것이 밝기만으로 가려지는가.** 방법: 각 메뉴에서 커서를 위아래로 옮기고 탭을 바꾼다. 숫자 행에서 슬라이더를 끌고, Enter 로 숫자를 치고, 키 설정에서 Enter 로 키를 기다리게 한다. 기대: 커서가 선 행은 한 단계 밝은 면과 흰 글자로, 고른 탭은 밝은 면으로 한눈에 가려진다. 커서가 선 행의 슬라이더는 손잡이가 흰색이고 끄는 동안 채운 부분까지 흰색에 손잡이가 커진다. 숫자를 치는 칸은 흰 테두리와 흰 캐럿이 보인다. 키를 기다리는 칸은 통째로 흰 면에 검은 `PRESS...` 다. 읽기 전용 행(유지 모드의 HI-SPEED 등)은 슬라이더와 값이 흐리다. 밝은 방에서 커서 행(면 `505050`)이 다른 행(`161616`)과 가려지지 않으면 기록한다: 면의 단계를 더 벌릴 항목이다.
- [ ] **O3 색이 남은 곳이 뜻이 있는 곳뿐인가.** 방법: Tables 에서 틀린 주소를 등록하고 오래된 저장본이 있는 표를 본다. PLAY 메뉴에서 GREEN NUMBER 를 1 로 내려 HI-SPEED 가 범위 끝에 걸리게 한다. 키 설정에서 이미 쓰는 키를 다른 행에 준다. 알림이 뜨는 상황(Tables 의 다시 받기가 끝났을 때와 실패했을 때 등)에서 정보·경고·오류 알림을 본다. 스킨 팩 없이(`--skin assets/skins/normal.ron` 으로 실행) 내장 선곡·플레이·결과를 돈다. 기대: 색이 있는 것은 실패·오래됨·한계와 키 설정의 되돌림·되묻기 안내(호박색), 키 충돌과 거절(빨강), 알림의 경고(주황)·오류(빨강) 띠, GREEN 숫자(초록), 판정 색, FAST(청록)/SLOW(주황), 클리어 램프와 이름, 난이도·모드 배지, 랭크, 점수 차(+ 초록 / - 빨강), 그래프, 게이지, 서버 연결 점뿐이다. 그 밖에 색이 보이면 기록한다.
- [ ] **O4 기존 설정 폴더의 `theme.ron`.** 방법: 이 빌드를 처음 실행하기 전에 설정 폴더(`~/.config/rbms`)의 `theme.ron` 을 다른 곳에 복사해 둔다. 실행한 뒤 터미널의 `theme:` 줄과 파일을 본다. 한 번 더 실행한다. 기대: 파일을 고친 적이 없으면 첫 실행의 줄 끝에 `(the template of an earlier version was replaced; the colours follow the defaults)` 가 붙고, 파일은 모든 색 줄이 `//` 로 시작하는 새 템플릿이 되며, 화면은 무채색이다. 두 번째 실행에서는 그 문구가 없고 파일이 바뀌지 않는다. 한 값이라도 고쳤던 파일이면 문구가 없고 파일이 그대로이며 화면은 그 파일의 색(전의 청색 계열)이다: 무채색으로 바꾸려면 파일을 지우거나 고치지 않은 줄을 지운다(`docs/theme.md`).
- [ ] **O5 내장 플레이 화면의 레인.** 방법: 스킨 팩 없이 7K 곡을 플레이한다(NORMAL 과 WIDE). 기대: 노트·키 빔·폭발·판정선은 전과 같고, 점수 그래프의 눈금선, 게이지의 빈 칸, 레인 바탕, 필드 테두리, 레인 구분선이 모두 회색이다(번들 `normal.ron`·`wide.ron`·`default.ron` 의 세 값이 `SkinConfig::default` 와 같은 회색 22 / 96 / 60 이다). 파란 기가 보이면 기록한다.

### 3.16 웨이브 16: 기록이 저장되는 조건, 목표 점수와 TARGET FALLBACK, 설정 화면

배경: 사용자 요청 세 가지(로그인하지 않으면 기록이 저장되지 않는 것 같다, LOCAL BEST 로 두어도 AAA 를 목표로 계산된다, Tab 메뉴가 딱딱하고 불편하다). 헤드리스 테스트와 캡처로는 확인했다: 로그인 없이 손으로 친 판이 이력·최고 기록·리플레이·다음 판의 LOCAL BEST 로 전부 남음, SCRATCH AUTO 판은 이력·램프·횟수만 남음, 리플레이를 본 뒤 SCRATCH AUTO·GAUGE·RANDOM 과 설정 파일이 플레이어의 값 그대로임, 턴테이블 노트가 없는 차트의 SCRATCH AUTO 는 어시스트가 아님, 목표 해석 표, 설정 화면의 레이아웃과 입력 모드 중의 패널. 아래는 실제 창·실제 설정 폴더·실제 손에서만 드러나는 것이다. 값과 규칙은 `docs/play-options.md` 10·11절, 화면 설계는 `docs/plan/2026-10-14-settings-screen.md`.

- [ ] **W1 내 설정에 SCRATCH AUTO 가 켜져 있는지.** 방법: 선곡에서 F8(PLAY 메뉴) → CHART 의 SCRATCH AUTO 행에 커서를 둔다. 기대: 정보 패널에 `WHILE ON: TURNTABLE NOT COUNTED, LIGHT ASSIST LAMP` 가 나온다(웨이브 18 전에는 `WHILE ON: NO BEST SCORE, LAMP AND HISTORY ONLY`). 켜 둘지는 이제 취향이다: 웨이브 18 부터 SCRATCH AUTO 판도 최고 기록·리플레이·전송이 남는다(3.17절). 이번 점검에서 본 실제 설정은 켜져 있었다(웨이브 16 이전의 모든 판이 어시스트였던 이유).
- [ ] **W2 리플레이를 본 뒤에도 설정이 그대로인가.** 방법: SCRATCH AUTO 를 끈 채, 예전에 SCRATCH AUTO 로 친 판의 리플레이를 기록 목록(R)이나 REPLAY 슬롯에서 끝까지 본다. 결과 화면에서 Esc 로 선곡에 돌아와 F8 을 열고, 앱을 껐다 켜서 다시 본다. 기대: 재생 중에는 턴테이블이 자동으로 쳐지고(리플레이 그대로), 돌아온 뒤 SCRATCH AUTO·GAUGE·RANDOM 이 재생 전 값이다. 재시작 뒤에도 같다.
- [ ] **W3 로그인 없이 한 판을 친 뒤.** 방법: NETWORK 에서 LOGOUT 한 상태(또는 SERVER URL 이 빈 상태)로 SCRATCH AUTO 를 끄고 7K 곡을 손으로 끝까지 친다. 선곡으로 돌아와 같은 곡을 본다. 기대: 결과 화면 오른쪽 위에 `SCRATCH AUTO:` 줄이 없다. 로그아웃 상태면 전송 줄이 `IR: SKIPPED (not logged in)` 이고 실패 알림이 없다. 선곡의 RANK·EX SCORE·RATE·MAX COMBO·MISS COUNT·판정 막대가 채워지고 PLAY COUNT 가 1 늘었다. 앱을 껐다 켜도 같다.
- [ ] **W4 어시스트로만 친 차트의 선곡.** 방법: SCRATCH AUTO 로만 쳤던 차트(이 사용자의 기존 기록 전부)에 커서를 둔다. 기대: CLEAR 가 LIGHT ASSIST CLEAR, PLAY COUNT·CLEAR COUNT 는 숫자, RANK·EX SCORE·RATE·MAX COMBO·MISS COUNT·판정 수는 `-` 다(전에는 EX 0, RATE 100.00%). 한 번도 치지 않은 차트도 전처럼 `-` 다.
- [ ] **W5 다음 판의 LOCAL BEST 목표.** 방법: TARGET 을 LOCAL BEST 로 둔다(F8 → CHART 또는 선곡의 옵션 패널 1). W3 에서 친 곡을 다시 친다. 그다음 한 번도 손으로 치지 않은 곡을 친다. 기대: W3 의 곡에서는 점수 그래프 범례가 `LOCAL BEST` 이고 판정 글자 위의 숫자가 W3 의 EX 와의 차이다. 결과 화면 TARGET 도 `LOCAL BEST` 와 같은 EX. 치지 않은 곡에서는 범례가 `RATE AAA (NO RECORD)` 이고 선곡 옵션 패널 1 이 `LOCAL BEST (NO RECORD: AAA)` 다.
- [ ] **W6 TARGET FALLBACK 과 TARGET RATE.** 방법: F8 → CHART 에서 TARGET FALLBACK 을 MAX, RATE A, RATE CUSTOM 으로 바꿔 가며 미리보기를 보고, RATE CUSTOM 일 때 TARGET RATE 를 70% 로 내린다. TARGET 을 IR BEST 로 바꿔 본다. TARGET 을 RATE AA 로, FALLBACK 을 RATE AAA 로 두고 TARGET RATE 에 커서를 둔다. 실제로 치지 않은 곡을 한 번 친다. 기대: TARGET 행이 `LOCAL BEST > MAX`, `> A`, `> 70%` 로 바뀌고 미리보기 범례가 그 목표를 따른다. IR BEST 는 `IR BEST > LOCAL > AAA` 와 `NO IR SCORE: YOUR LOCAL BEST, THEN RATE AAA`. 마지막 상태의 TARGET RATE 패널은 `NOT IN USE: NO TARGET IS RATE CUSTOM`. 실제 판의 범례·숫자·결과 화면 TARGET 이 고른 fallback 이다.
- [ ] **W7 설정 화면(Tab)을 손으로.** 방법: Tab 으로 연다. Tab·Shift+Tab 으로 묶음을 돌고, PgUp/PgDn/Home/End, 휠, 라벨 클릭과 스위치·칸·화살표 클릭, 오른쪽 클릭을 써 본다. 숫자를 Enter 로 치는 중, EMAIL·PASSWORD 를 치는 중, RIVALS 목록을 연 중, Shift+0 으로 묶음 되돌리기를 묻는 중에 오른쪽 패널을 본다. 1280x720 창과 전체 화면에서 본다. 기대: 모든 묶음이 스크롤 없이 한 화면이고 잘린 글자가 없다. 각 모드 중에는 패널의 KEYS 가 그 모드의 키(`DIGITS TYPE A NUMBER`, `KEYS TYPE`, `D REMOVE`, `ENTER RESET THE GROUP`)이고 VALUE 가 치는 값이다(비밀번호는 가려짐). 바닥 안내와 패널이 같은 말을 한다. 불편하거나 읽기 어려운 곳(작은 글자 등)은 기록한다.
- [ ] **W8 스위치에서 Enter.** 웨이브 18 에 사용자 결정으로 바뀌었다. 확인은 3.17절 X5 로 옮겼다.

### 3.17 웨이브 18: SCRATCH AUTO 의 새 규칙, 같은 배치로 다시 하기, 설정 화면의 Enter, 기본 표와 MY FOLDERS

배경: 사용자 결정(2026-10-14) "scratch auto 를 하면 scratch 가 초록색이 된다, 플레이 중에 카운트되지 않는다, 진짜로 존재하지 않는 노트처럼 처리된다, 서버로 똑같이 보낸다, 37판은 램프만 있어도 된다, 리플레이도 잘 저장" 과 요청 "곡이 끝나고 파란 건반(2, 4, 6키)을 누른 채로 START 를 누르면 똑같은 랜덤 배치로 다시 시작". 헤드리스 테스트와 CPU 캔버스 캡처로는 확인했다: 판정·콤보·게이지·EX·노트 수가 턴테이블 노트가 없는 차트와 같음(`crates/rbms-play/tests/scratch_auto.rs`), 초록 노트(기본 팩·ModernChic), 결과 화면 문구, 저장·최고 기록·리플레이·가짜 서버로 간 제출, 세대 2 행과 결과 화면의 이전 점수, 파란 건반 + START 의 시드, 설정 화면 Enter 의 동작과 바닥 안내, 첫 실행의 표와 MY FOLDERS. 아래는 실제 창(GPU)·소리 장치·컨트롤러·실제 설정 폴더·실제 서버에서만 드러나는 것이다. 규칙은 `docs/play-options.md` 10절, 폴더는 `docs/plan/2026-10-14-web-user-folders.md` 13절.

- [ ] **X1 초록 턴테이블(GPU).** 방법: SCRATCH AUTO 를 켜고 턴테이블 노트·롱노트·지뢰가 있는 7K 곡을 기본 팩으로, 그다음 ModernChic 으로 친다. 14K 곡도 한 번. 기대: 턴테이블 레인의 노트와 롱노트 시작·끝이 단색 초록(#30E050), 롱노트 몸통이 반투명 초록이고 판정선에서 사라진다. 지뢰는 보이지 않는다. 턴테이블 키를 눌러도 빔·폭발·공 POOR·소리가 없다. 판정 수·콤보·게이지가 턴테이블 노트에 움직이지 않는다. 노트 그림이 다른 크기로 그려지거나 깜박이면 기록한다.
- [ ] **X2 턴테이블 키음.** 방법: X1 과 같은 곡을 소리 장치로 듣는다. KEY 음량을 내렸다 올린다. 기대: 턴테이블 노트의 키음이 제 박자에 한 번씩 울리고 KEY 음량을 따른다. 빠지거나 두 번 울리면 기록한다.
- [ ] **X3 SCRATCH AUTO 판의 기록과 전송.** 방법: 로그인한 상태로 X1 의 곡을 끝까지 치고 결과 화면, 선곡의 점수, 기록 목록(R)의 리플레이를 본다. 웹(bms.hyuns.uk)에서 그 판을 찾는다. 기대: 결과 화면 오른쪽 위 `SCRATCH AUTO: TURNTABLE NOT COUNTED, LIGHT ASSIST LAMP`, 램프 LIGHT ASSIST CLEAR, 선곡의 EX·RATE·RANK 가 채워짐, 리플레이가 같은 결과로 재생됨, 서버에 그 판이 SCRATCH AUTO 표시와 함께 있음. 서버가 그 판을 어떻게 보여 주는지(순위에 넣는지)는 서버의 일이라 보이는 대로 기록한다.
- [ ] **X4 파란 건반 + START(실제 컨트롤러).** 방법: RANDOM 을 켜고 한 판을 친 뒤 결과 화면에서 컨트롤러의 2번(또는 4·6번) 건반을 누른 채 START 를 누른다. 5K·9K(팝픈)·14K 에서도 한 번씩. 파란 건반만 눌렀다 떼 본다. START 만 눌러 본다. 기대: 같은 곡이 같은 노트 배치로 바로 다시 시작한다. 7K 에서 2·4번 건반만 눌렀다 떼면 선곡으로 간다(누를 때가 아니라 뗄 때. 6번은 그래프만 바꾼다). START 만으로는 아무 일도 없다. 키보드로 할 때는 START 키와 파란 건반 키를 함께 누른다.
- [ ] **X5 설정 화면의 Enter.** 방법: Tab 으로 열고 스위치 행, 선택 행, SOUND FOLDER 행, 숫자 행, KEY CONFIG 에서 차례로 Enter. 기대: 스위치는 뒤집히고, 선택은 다음 값, SOUND FOLDER 는 OS 의 폴더 고르기 대화상자, 숫자는 직접 입력, KEY CONFIG 는 키 설정. 어느 Enter 도 화면을 닫지 않고 Esc 만 저장하고 닫는다. 바닥 안내가 `ENTER SWITCH`, `ENTER NEXT`, `ENTER RIGHT CHOOSE A FOLDER` 를 말한다.
- [ ] **X6 내 기록 DB 가 이 빌드에서 어떻게 보이나.** 방법: 앱을 켜기 전에 설정 폴더의 `scoredb.sqlite` 를 다른 곳에 복사해 둔다. 원하면 그 사본으로 `RBMS_SCOREDB_COPY=<사본> cargo test -p rbms-player --lib --locked -- --ignored a_players_own_database --nocapture` 를 돌려 숫자를 본다(테스트가 사본을 제 폴더로 다시 복사해 읽으므로 사본도 바뀌지 않는다). 앱을 켜고 예전에 SCRATCH AUTO 로 친 곡을 본다. 기대: 그 곡들의 램프(LIGHT ASSIST CLEAR)와 횟수가 그대로이고 점수는 `-`. 새로 한 판 치면 그 판이 첫 최고 기록이 되고 결과 화면이 옛 판의 EX 를 이전 점수로 보이지 않는다. 웨이브 16·18 의 측정은 37판·29차트·LOCAL BEST 0·램프 29 였다.
- [ ] **X7 첫 실행의 발광 표.** 방법: 설정 폴더를 다른 곳으로 옮겨 새 설치처럼 만든 뒤(끝나면 되돌린다), 네트워크를 끊고 켠다. 그다음 네트워크를 잇고 Tables(T)에서 그 표에 R. 기대: 선곡 뿌리에 ALL SONGS 와 발광 표가 있고, 로그인 전이라 MY FOLDERS 는 없다. 표 안에 26개 레벨 폴더. Tables 화면의 표 줄이 `SAVED` 이고, R 뒤에 `https://bms.hyuns.uk/tables/insane1` 에서 다시 받아진다. 표를 D 로 지우고 다시 켜면 돌아오지 않는다.
- [ ] **X8 설정 파일 스키마 5 에서 6 으로.** 방법: 기존 설정 폴더로 이 빌드를 처음 켠다. 기대: `settings.ron.v5.bak` 이 생기고, NETWORK 의 SERVER URL 이 비어 있었으면 `https://bms.hyuns.uk/api` 가 들어가며, 있었으면 그대로다. 그 뒤 SERVER URL 을 비우고 다시 켜면 비어 있는 채로 남는다. v0.1.0 으로 되돌아가야 하면 `.v5.bak` 을 `settings.ron` 으로 되돌린다.
- [ ] **X9 MY FOLDERS(실제 서버).** 방법: 웹에서 폴더 둘을 만들고 곡을 넣은 뒤 앱에서 로그인한다. F6, Tables 의 R 을 눌러 보고, 웹에서 폴더 이름을 바꾼 뒤 F6. 로그아웃한다. 기대: 로그인하면 선곡 뿌리에 `MY FOLDERS (n)` 줄, 안에 `이름 (가진 곡/전체)`, 폴더 안에 가진 곡만. F6 뒤 바뀐 이름. Tables 화면 맨 끝의 MY FOLDERS 줄이 받은 시각을 말한다. 로그아웃하면 줄이 사라진다. 일본어 이름이 시스템 글꼴로 제대로 보이는지 함께 본다.

### 3.18 S19-2b: 화면 대기를 뗀 키 입력 시각, KEY DEBOUNCE

배경: 사용자 결정(2026-10-15) 2번 "beatmania iidx, lunatic rave 2(lr2)등에서 쓰이는방법으로 제대로 정확하게 구현해", 3번 "키보드에도 짧은 디바운스(설정 가능)". present 대기를 전용 스레드(`gpu/present.rs`)로 떼고 이벤트 루프는 `ControlFlow::Wait` 로 프레임 사이에 창만 기다리게 했다([ADR](../acknowledge/2026-10-15-adr-keyboard-input-timing.md)). 헤드리스 테스트로 확인한 것: 화면이 이미지를 붙잡고 있어도 다음 이미지 요청이 바로 돌아오는 것, 레인 키 디바운스의 동작, 설정 표. 같은 구조의 하네스(winit 0.30.13·wgpu 29.0.4, M5 Max 120 Hz, Metal)에서 이벤트 지연 p95 8.0 ms → 0.04 ms. 실제 플레이어 창·실제 키보드·다른 OS 에서는 돌려 보지 않았다. 아래는 그것이다.

- [ ] **K1 화면이 그대로 나오는가(macOS).** 방법: 기본 팩으로 선곡 → 7K 곡 한 판 → 결과까지 간다. DEBUG MODE 를 켜고 FPS 를 본다. 기대: 이전 빌드와 같은 FPS(모니터 주사율)이고 깜박임·찢김·멈춤이 없다. 터미널에 wgpu·AppKit 경고가 없다(특히 메인 스레드 밖에서 `NSView`·`NSWindow` 를 읽었다는 경고. Xcode 의 Main Thread Checker 로 띄우면 더 확실하다).
- [ ] **K2 창 크기·모드 바꾸기.** 방법: 플레이 중과 선곡에서 창 모서리를 끌어 크기를 바꾸고, 설정의 RESOLUTION·WINDOW MODE 를 바꾼다(창 ↔ 테두리 없는 전체 화면). 기대: 바꾸는 동안과 뒤에 그림이 늘어나거나 멈추지 않고, 앱이 멈추거나 죽지 않는다. 크기를 바꿀 때 이벤트 루프는 화면을 기다리지 않고, 요청해 둔 이미지가 오면 그것을 버리고 표면을 다시 맞춘다(그래서 바꾼 직후 한 프레임이 빠질 수 있다). 끄는 동안 그림이 잠깐 멈췄다가 놓으면 이어져도 괜찮다(그렇다면 기록한다).
- [ ] **K3 가려진 창·최소화·다른 앱으로 전환.** 방법: 한 판 치는 도중 창을 최소화했다 되돌리고, 다른 앱으로 Cmd+Tab 했다 돌아온다. 롱노트를 누른 채로도 한 번 한다. 기대: 돌아오면 그대로 이어지고(곡은 계속 흘렀다), 롱노트는 포커스를 잃은 순간 뗀 것으로 판정된다(PLAY-13). CPU 사용률이 이전 빌드보다 높지 않다.
- [ ] **K4 종료.** 방법: 한 판 친 뒤 결과 화면에서 Esc 로 선곡에 돌아와 Cmd+Q, 다시 켜서 창 닫기 버튼, 한 판 도중 Cmd+Q. 기대: 바로 끝나고(멈춘 채 남지 않는다. 화면 대기 스레드를 기다리는 것은 최대 1 초다), 다시 켰을 때 그 판의 기록·리플레이·설정이 남아 있다. 터미널에 "rbms-present thread did not finish" 경고가 없다.
- [ ] **K5 키 지연이 줄었는가.** 방법(간단): 이전 빌드와 이 빌드로 같은 곡을 키보드로 쳐 FAST/SLOW 분포를 본다. 방법(정밀): `--timing-csv` 로 한 판을 남겨 `input_at_us` 를 고속 카메라나 루프백 장치의 시각과 비교한다. 60 Hz 모니터에서도 한 번. 기대: 이 빌드가 평균 약 반 프레임(60 Hz 에서 약 8 ms, 120 Hz 에서 약 4 ms) FAST 쪽으로 옮겨지고 흩어짐이 줄어든다. 예전 빌드에서 맞춘 JUDGE OFFSET 은 다시 맞춰야 한다.
- [ ] **K6 KEY DEBOUNCE.** 방법: Tab → INPUT 에서 KEY DEBOUNCE 가 16 MS 인지 본다. 롱노트가 많은 곡을 키보드로 친다. 채터가 있는 키보드가 있으면 그 키로. 값을 0 과 100 으로 바꿔 한 판씩. 기대: 16 에서 누르는 순간의 채터로 롱노트가 끊기지 않는다. 16 에서 같은 레인 32분 연타(200 BPM 안팎)의 FAST/SLOW 가 0 일 때와 같다(누름은 늦춰지지 않는다). 100 에서는 같은 키를 빠르게 연타하면 일부가 빠진다(누른 지 100 ms 안의 뗌이 미뤄지는 동안 다시 누르면 누른 채로 남는다. 그것이 창의 뜻). 0 은 예전과 같다. 누른 채 있다가 접점이 잠깐 떨어지는 채터로는 여전히 끊길 수 있다(ADR "남은 한계" 2). 메뉴·서버 주소 입력·IME 입력은 값과 상관없이 그대로다.
- [ ] **K7 Windows·Linux.** 방법: Windows(DX12, 그리고 가능하면 Vulkan)와 Linux(X11·Wayland)에서 K1~K4 를 한 번씩. 기대: 같다. 다른 스레드에서의 이미지 획득·present 는 wgpu 가 허용하지만 이 OS 들에서 돌려 보지 않았다. Windows DX12 에서 창 크기를 끌어 바꾸는 동안 멈추지 않는지(Present 중 창 스레드로 오는 동기 메시지), Wayland 에서 다른 작업 공간에 숨긴 창의 크기를 바꿨다 돌아왔을 때 이어지는지를 함께 본다.
- [ ] **K8 Main Thread Checker(macOS, 알려진 위험 확인).** 방법: `DYLD_INSERT_LIBRARIES=$(xcode-select -p)/usr/lib/libMainThreadChecker.dylib MTC_RESET_INSERT_LIBRARIES=0 ./rbms-player` 로 5초 띄운 뒤 stderr 의 `Main Thread Checker` 줄을 센다. 기대: wgpu Metal acquire 의 두 건(`-[NSView window]`, `-[NSWindow occlusionState]`, 스레드 `rbms-present`)만 나온다(상류 gfx-rs/wgpu#10627, ADR keyboard-input-timing 남은 한계 6). 그 밖의 위반이 나오면 결함이다. wgpu 를 올린 뒤에는 이 두 건이 사라졌는지 본다.

## 4. 기록

| 날짜 | 확인자 | 기기·OS | 결과 | 비고 |
| --- | --- | --- | --- | --- |
| | | | | |
