# S3 테스트 웨이브 보고 (wf_3df278ff-bb7)

> 테스트 웨이브의 에이전트가 끝낼 때 낸 보고 원문이다. 메인이 병합하면서 바꾼 것은 계획서 9절에 있다.

- 상태: done

## 바꾼 파일

- apps/rbms-player/src/stage/capture/long_text.rs
- assets/skins/rbms-default/lua/select/controls.lua
- assets/skins/rbms-default/lua/select/records.lua
- assets/skins/rbms-default/lua/theme.lua
- crates/rbms-skin/tests/default_pack.rs

## 요약

U8(선곡 화면 개편) 수용 기준 검증을 끝냈습니다. 지침 1~5 와 상태별 캡처 전부를 직접 열어 봤고, 재현되는 결함 4건은 시험을 먼저 만든 뒤 스킨에서 고쳤습니다. 한국어·일본어 폭은 글꼴 메트릭으로 따졌고 모두 넘침 규칙 안입니다. skin-dump 세 언어, 시트 검사, 관련 시험은 모두 통과했습니다. 아래 "수정한 결함 4건"과 "확인했지만 고치지 않은 것" 순서로 정리합니다.

수정한 결함 4건
- 제목과 부제 줄이 붙었습니다. 제목 `g`·`y`·`p` 꼬리와 `[` 로 시작하는 부제 사이가 1px 이었고, 픽셀 행으로 보면 줄이 3개로 합쳐졌습니다. `theme.select_layout.song.baselines` 를 {153,199,228,263} 에서 {153,197,230,264} 로 바꿔 줄 사이를 5.7/5.0/4.8px 로 고르게 벌렸습니다.
- 긴 경로의 끝이 잘렸습니다. 말줄임이 끝에서 일어나 현재 폴더 이름이 사라졌습니다. 개편으로 경로 칸이 728px/18 에서 552px/24 로 줄어 더 쉽게 일어납니다. `controls.path_tail` 을 더해, 넘치면 앞 폴더를 `… > ` 로 줄이고 마지막 폴더를 남깁니다.
- MY RECORDS 의 MISS 가 5자리에서 잘렸습니다. 12345 가 2345 로 나왔습니다. 개편 전부터 있던 알려진 한계입니다. 10000 이상일 때만 5자리 칸(`fit` 48px)으로 그립니다.
- MAX COMBO 라벨이 5자리 콤보에 닿았습니다. 영어에서 `MAX COMBO` 와 첫 숫자 사이가 1px 이었습니다. 정보창 쌍을 폭 184/간격 32 에서 192/16 으로 바꿔 라벨과 값 사이를 9px 로 벌렸습니다. 오른쪽 끝(752)은 그대로입니다.

시험은 `long_text.rs` 의 한 시험(줄 4개, MISS 5자리, 콤보 간격)과 `default_pack.rs` 의 경로 시험입니다. 전부 수정 전에 실패하는 것을 확인했고 지금은 통과합니다. 새 시험은 Rust 파일에 더했고 Rust 제품 코드는 고치지 않았습니다. 긴 글 캡처 데이터에는 5자리 MAX COMBO(11111)를 더했습니다. 임시로 만든 캡처 모듈은 지웠고 `capture.rs` 는 바뀐 것이 없습니다.

참고 사항
- 한국어·일본어 칸 폭은 번들 Inter 에 macOS Apple SD Gothic Neo·Hiragino 메트릭을 쓰고, 최악 가정(CJK 1em)도 같이 계산했습니다. 모든 칸이 줄이지 않거나 하한 0.75 안에서 줄어듭니다. 말줄임이 나는 칸은 없습니다. 실제로 가장 빡빡한 칸은 일본어 정렬 값 `アーティスト` 144/136(0.94배 축소)와 FAVORITE 버튼 `お気に入り` 90/84(0.93배)입니다. 캡처 하니스가 CJK 를 그리지 못해 실제 모양은 보지 못했습니다.
- 개편 전에는 검색 강조색이 새로 쓰이지 않았는데, 이번에 검색 중 밑줄에 accent 가 새로 쓰였습니다. 켜짐 표지 규칙과 같다고 보고 통과로 판정했습니다. 모노톤 규칙에 어긋난다고 보시면 되돌려야 합니다.

## 기준별 판정

- [pass] 지침 1: 머리 한 띠(왼쪽 화면 이름·경로, 가운데 검색, 오른쪽 MODE·SORT·LN MODE 칩, 끝 시계, 아래 선): 픽셀로 측정하면 경로·SEARCH·칩 이름·칩 값·시계의 글자 아래 끝이 모두 기준선 77 한 줄이었습니다(ink rows ..76/77). 칩은 같은 모양 3개(높이 40)이고 시계는 칩 없이 맨글자입니다. 해석 두 가지는 메인이 이미 승인했습니다. 머리 아래 선은 화면 전체가 아니라 64~1856 이고(원인은 오른쪽 카드 아랫변), 검색 비었을 때 "작게"는 크기가 아니라 조용한 모양(선 하나와 muted 라벨)입니다. 근거: controls.lua 머리 부분, 캡처 select-songs-3000ms, overlay-search, overlay-no-results.
- [pass] 지침 2: 제목 heading 크기와 SHRINK 유지, LEVEL·BPM 을 제목 블록에 붙임, 줄어든 높이를 정보창에 줌: 처음에는 fail 이었습니다. 글꼴 메트릭으로 계산하니 제목과 부제의 잉크 간격이 1.0px 였고, 긴 글 프레임의 줄 구간이 [134..159, 166..234, 239..270] 3개로 합쳐졌습니다. baselines 를 고친 뒤 4개로 분리되고 시험이 통과합니다. 블록 높이는 136, 2행은 296 으로 늘었고 LEVEL·BPM 은 색 막대 옆 104폭 칸에 있습니다.
- [pass] 지침 3: 바닥 한 줄(키 표지와 짧은 이름, 문장형 둘째 줄 제거, 오른쪽 버튼과 높이 맞춤): 바닥 모든 글자의 잉크 행이 1033..1045 로 같습니다(ENTER·PLAY·REPLAY·AUTOPLAY·PRACTICE·RANKING·FAVORITE). 키 표지(28)와 버튼(40)은 중심선 1040 을 공유합니다. 여기서 "높이를 맞춘다"를 같은 중심선과 기준선으로 해석했고, 같은 높이 40 으로 맞추라는 뜻이면 더 고쳐야 합니다. i18n 키 bar_hint_line·bar_hint_all_keys 는 없어졌습니다.
- [pass] 지침 4: 구획 머리 한 부품(kit.section), 라벨 위·값 아래 정렬, 겹침 제거, 간격 24·바깥 여백 64: CHART DATA·MY SCORE·NOTES RADAR·MY RECORDS·SCORE TARGETS·CLEAR RATE·CHARTS·CLEAR LAMPS·STAGES·CONSTRAINTS 가 모두 kit.section(선 + 12 아래 캡션)입니다. 세로 간격은 제목 272 → 296 → 576 → 896 으로 24 이고, 가로 격자 간격은 32 입니다. 겹침은 MAX COMBO 라벨과 5자리 콤보가 1px 로 닿는 것이 하나 있었고 고쳤습니다(위 결함 4).
- [pass] 지침 5: 모노톤과 강조색 규칙, 넘침 규칙, 세 언어, 외부 값 추가 없음: U8 커밋 51f4884 는 스킨 9개 파일만 바꿨고 Rust 는 없습니다. 새 색도 없습니다. 개편 전에는 없던 accent 사용이 검색 중 밑줄에 하나 새로 생겼고, 켜짐 표지로 보고 통과시켰습니다. 모노톤 규칙에 어긋난다고 보시면 되돌려야 합니다.
- [pass] 모든 상태: 곡·폴더·코스 막대, 표 폴더, 없는 곡 막대, 검색 입력 중·결과·결과 없음, 랭킹 모드, 옵션 패널 세 종류, 사이드 메뉴, 긴 제목·아티스트·경로, 다섯 자리 수치: 모든 상태를 캡처로 열어 봤습니다(select-songs/root/table/courses/unplayed, u8-missing, overlay-search/no-results, u8-search-long, u8-ranking-mode, select-panel1~3, u8-side-settings/volume, long-text, u8-longpath). 겹침·잘림 결함은 4건이고 모두 수정했습니다. 남은 것은 고치지 않은 항목(findings 의 fixed=false 5건)이고, 모두 개편 전부터 있었거나 Rust 영역입니다.
- [pass] 누르는 영역: 바닥 버튼·REPLAY 1~4·칩·슬라이더가 그려진 상자와 같은 자리: kit.button 은 클릭 정의(def.act)를 가진 이미지와 hover 이미지(mouseRect = 0,0,w,h)에 같은 box 를 씁니다. 칩·바닥 버튼·REPLAY 1~4·FAVORITE·난이도 탭·사이드 탭 모두 그 box 가 그려진 상자입니다. 사이드 볼륨 슬라이더의 이동 구간은 트랙 폭과 같습니다. 예외는 검색 입력입니다(findings 참고). 사이드 탭은 skin_pointer 로 실제 눌러 열림을 확인했고, 나머지 영역은 코드로만 대조했습니다.
- [pass] 한국어·일본어 칸 폭: 머리 칩 이름, 바닥 라벨, 버튼, 구획 머리, 정보창 라벨: Inter 와 macOS Apple SD Gothic Neo·Hiragino 로 53개 문구를 재고 최악 가정(CJK 1em)도 같이 계산했습니다(/tmp/u8scratch/tools/measure.py). 말줄임이 나는 문구는 없고 하한 0.75 안에서 줄어드는 것만 둘입니다. 일본어 SORT 값 アーティスト 144/136 은 0.94배, 일본어 FAVORITE お気に入り 90/84 는 0.93배이고, 영어 FAVORITE 도 0.95배입니다. 칩 이름은 일본어 モード 54/60·ソート 54/56 으로 빡빡하지만 들어갑니다. 구획 머리 문구는 모두 영어 원어라 해당이 없습니다.
- [pass] 쓰이지 않게 된 i18n 키·theme 토큰·함수 없음, 세 언어 카탈로그 키 일치: i18n 237개 키는 전부 en·ko·ja 를 갖고 쓰이지 않는 키는 0개입니다. rbms-skin default_pack 시험 13개가 키 일치와 값 자리 일치를 확인하며 통과합니다. theme.select_layout 에서 안 쓰는 토큰은 0개이고, select 모듈에도 안 쓰는 local 함수·변수가 없습니다. 개편 전부터 안 쓰이던 theme.radar.picture 하나는 findings 에 보고했습니다.
- [pass] skin-dump --strict --frames 120 세 언어(21150~21152)와 시트 검사: 전체 팩 strict 통과(select 문서의 destinations 는 2166 → 2172, pcall·fn fail·warn 0, 팩 88개 파일 unchanged). select.luaskin 을 en/ko/ja 시나리오로 각각 돌렸을 때 모두 exit 0 이고 pcall·fn fail·warn 이 0 입니다. `generate-images.py --check` 는 exit 0 이고 모든 시트가 matches 입니다.

## 찾은 것

- [medium, 고침] 제목과 부제 줄이 1px 간격으로 붙어 있었음: 긴 글 프레임의 줄 구간이 [134..159, 166..234, 239..270] 3개였습니다. 제목과 부제가 하나로 합쳐진 것입니다. 글꼴 메트릭으로는 제목 아래 끝 208.5, 부제 위 끝 209.5(`]`)라 간격이 1.0px 입니다. BMS 에서 흔한 `[SP ANOTHER]` 부제와 `g/y/p` 제목에서 생깁니다. baselines 를 {153,197,230,264} 로 고쳐 줄 사이를 5.7/5.0/4.8px 로 벌렸고 마지막 줄 아래 끝은 270.9(블록 272 안)입니다. (시험: apps/rbms-player/src/stage/capture/long_text.rs::the_default_packs_info_window_keeps_its_lines_apart_and_its_five_digit_numbers_whole_and_clear (줄 4개 확인, 수정 전 3개로 실패))
- [medium, 고침] 긴 폴더 경로의 끝(현재 폴더)이 말줄임으로 사라짐: table 이름과 level 이름이 긴 캡처에서 경로가 `A DIFFICULTY TABLE WITH A VERY LONG NAME THAT KEEPS...` 로 끝나 현재 폴더(LV…)가 안 보였습니다. 개편으로 경로 칸이 728px/18 에서 552px/24 로 줄어 같은 경로에서 더 쉽게 일어납니다. `controls.path_tail` 을 더해, 한 줄 폭 예산(narrow 10, 넓은 글자 16, 예산 420)을 넘으면 앞 폴더를 `… > ` 로 줄이고 마지막 폴더를 남깁니다. 한 폴더 이름 자체가 길면 엔진의 SHRINK_CUT 이 그 이름의 끝을 자릅니다. 수정 뒤 캡처에서 `… > LV LEVEL ONE OF THE TABLE…` 로 보입니다. (시험: crates/rbms-skin/tests/default_pack.rs::a_folder_path_too_long_for_the_browsers_header_keeps_the_folders_it_ends_with (함수가 없어 수정 전 실패, 6가지 경우 확인))
- [low, 고침] MY RECORDS 의 MISS 가 10000 이상이면 아래 4자리만 보임(12345 → 2345): 개편 전부터 있던 알려진 한계입니다(ADR 2026-10-29 의 "기존 한계"). 값이 틀리게 보이므로 고쳤습니다. 긴 글 프레임에서 MISS 칸의 숫자 덩어리가 4개였습니다. 10000 이상일 때만 5자리 `fit` 48px 숫자를 그리고(0.75배), 나머지는 이전 그대로입니다. `centered_number` 는 `number_box` 의 rise 를 쓰도록 바꿔 줄인 숫자도 가운데에 놓입니다. (시험: long_text.rs 같은 시험(MISS 칸에 숫자 덩어리 5개, 수정 전 4개로 실패))
- [low, 고침] 영어 MAX COMBO 라벨이 5자리 콤보에 1px 로 닿음: 긴 글 데이터의 콤보가 0 이라 보이지 않았습니다. 최고 기록에 max_combo 11111 을 더하자 `MAX COMBO11111` 로 닿았습니다(글자 덩어리 열 [452..464, 465..472, 480..487, ...] 에서 라벨 끝 464 와 첫 숫자 465). 쌍 폭을 192, 간격을 16 으로 바꿔 9px 가 되었고 오른쪽 끝은 같습니다. 10000 콤보 이상 차트에서만 생깁니다. (시험: long_text.rs 같은 시험(라벨 마지막 글자와 첫 숫자 사이 3px 이상, 수정 전 1px 로 실패))
- [low, 안 고침] 앱이 덧그리는 패널이 정보창을 덮음(Rust 영역): 검색 상태 칩 `SEARCH bor 1 HITS ESC CLEARS` 가 (24..348, 816..858) 에 고정돼 레이더 아래 라벨(SOF-LAN)과 PGREAT 줄 왼쪽 끝을 덮습니다. 코스 탭 칩 `COURSE SHIFT+TAB FOR SONGS` 도 같은 자리입니다. 랭킹 오버레이는 정보창 왼쪽 절반을 덮어 스킨의 IR RANKING 줄이 오른쪽 EX SCORE 열만 보입니다. 이 마지막은 HANDOFF 8절 2번에 이미 있는 테스트 부채입니다. 범위 밖이라 고치지 않았습니다. (시험: 스킨 범위 밖 Rust(app overlay)라 시험 없음. 캡처 first-run-search-skin-text-on-show, u8-ranking-mode)
- [low, 안 고침] 검색 입력을 누르는 영역(336x24)이 그려진 칸(360x40)보다 작음: 검색 시작은 SEARCHWORD 텍스트의 도착 사각형을 누르는 영역으로 씁니다(search.rs `search_text_of`). 칸의 안쪽 여백 12px 와 위 12px·아래 4px 는 눌러도 반응이 없을 수 있습니다. 엔진은 텍스트 도착 사각형의 높이를 글자 크기(em)로 읽으므로(skin_render/text.rs 머리말) 스킨이 사각형을 키우면 글자가 커집니다. 스킨만으로는 고칠 수 없습니다. 칸 모서리를 눌러 보는 실측은 하지 못했고 코드로만 확인했습니다. (시험: 시험 없음(엔진 제약))
- [low, 안 고침] 사이드 패널 위쪽이 격자(136)보다 8px 높음(128): 사이드 메뉴 패널의 상단 선이 y=128 이고 정보창·스크롤 트랙은 136 에서 시작하며 아래 끝은 968 로 같습니다. 개편 전에도 같은 값이었고(side.lua PANEL.top 128) U8 이 바꾼 곳이 아닙니다. 안쪽 좌표가 모두 PANEL.top 기준이라 이 단위에서는 고치지 않았습니다. (시험: 시험 없음(개편 전부터 있던 값))
- [low, 안 고침] 개편 전부터 안 쓰이는 theme.radar.picture 토큰: `theme.radar.picture` 를 읽는 코드가 없고(RADAR_PICTURE 지역 상수는 radius 계산에 쓰임) 개편 전 커밋에도 같은 상태입니다. 요청 밖 기존 dead 코드라 보고만 합니다. (시험: 시험 없음)
- [low, 안 고침] 영어 CLEAR COUNT 라벨은 4자리 이상 값에서 겹칠 수 있음(의심): 글꼴 메트릭으로는 CLEAR COUNT 129px + 4자리 숫자 62px = 191px 라 쌍 폭 192 에 겨우 들어가고, 5자리는 202px 로 넘칩니다. 한 차트의 클리어 횟수가 만 번 이상인 경우는 현실적이지 않아 시험으로 재현하지 못했습니다. 재현하지 못했으므로 의심으로만 보고합니다. (시험: 재현 시험 없음(비현실적 입력))

## 남은 위험

- 한국어·일본어의 실제 모양은 캡처 하니스가 CJK 를 못 그려 보지 못했습니다. 폭은 macOS 글꼴 메트릭과 최악 가정(CJK 1em)으로만 따졌고, Windows·Linux 의 대체 글꼴 폭은 재지 않았습니다. 실창 확인이 필요합니다(QA 3.25 V10·V11).
- 경로 예산(420, 글자당 narrow 10·넓은 글자 16)은 24px Inter 평균 폭에서 어림한 값입니다. 대문자만으로 된 긴 이름은 0.8배까지 줄고, CJK 는 한 줄에 26자 안팎입니다. 앞 폴더를 줄이는 기준이 글자 수 근사라 실제 폭과 어긋날 수 있으며 가장 긴 마지막 폴더는 여전히 엔진이 끝을 자릅니다.
- MY RECORDS MISS 가 10000 이상이면 숫자가 0.75배로 작아집니다. 9999 와 10000 사이에서 숫자 크기가 바뀌는 것이 보입니다.
- 정보창 쌍 사이 간격이 32 에서 16 으로 줄었습니다(오른쪽 끝 752 는 그대로). MY SCORE 카드 안 EX/MISS 열 간격 16 과 같습니다. ADR 의 "격자 gutter 32" 서술과 달라졌습니다.
- 검색 결과를 보는 동안 검색칸이 채워진 채 비어 있는 모양이고 검색어는 왼쪽 경로 자리에만 보입니다. 설계 의도(원본 제목 형식 "Search : '" 로 판단)이지만 사용자가 혼란스러워할 수 있습니다.
- 없는 곡 막대·검색 결과 직후에는 정보창에 값 없는 캡션만 남는 뼈대가 보이고, 결과 없음에서는 왼쪽 위에 회색 막대 조각이 하나 남습니다. 둘 다 개편 전부터 있던 모습입니다. 캡처로 보고 그대로 두었습니다.

## 갱신이 필요한 문서

- docs/skin.md 1.4절·3.4.6절: 머리 경로의 `controls.path_tail` 규칙(앞 폴더를 `… > ` 로 줄임), 제목 블록 baselines 197/230/264, 정보창 쌍 폭 192·간격 16, MY RECORDS MISS 10000 이상 5자리 칸
- docs/acknowledge/2026-10-29-adr-select-info-overflow.md 4절 표와 "알려진 위험": MY RECORDS MISS 4자리 한계가 해소됨, 통계 쌍 폭 184/간격 32 → 192/16, 경로 칸은 SHRINK_CUT 이 끝을 자르므로 앞 폴더를 줄이는 규칙을 둠
- docs/acknowledge/2026-10-24-adr-select-iidx-layout.md: 머리 경로 줄임 규칙과 개편 이후 값의 부분 변경 표기
- docs/reference/2026-11-19-feature-upgrade-12/README.md 12절(현재 모습)
- docs/plan/2026-11-19-feature-upgrade-12.md 8절 U8 행: 수용 기준 검증에서 고친 4건(제목·부제 간격, 경로 끝 보존, MISS 5자리, MAX COMBO 간격)과 개편 전부터 있던 한계가 해소된 것
- docs/quality-assurance: 선곡 실기 점검(한국어·일본어 칩·바닥·버튼 폭, 긴 경로, 5자리 수치)과 테스트 부채(앱이 덧그리는 칩·랭킹 오버레이가 정보창을 덮음, 검색 입력 클릭 영역, 사이드 패널 위쪽 8px)
- docs/assets/screenshot-select.jpg: 제목 baselines 와 정보창 쌍 간격이 바뀌었으니 README 캡처를 새로 뜸(메인이 병합 뒤)

## 돌린 검사

- `cargo test -p rbms-player --lib --locked the_default_packs_info_window_keeps  (수정 전, 단계마다)` → 제목·부제 줄 3개(기대 4), MISS 숫자 덩어리 4개(기대 5), MAX COMBO 와 첫 숫자 1px(기대 3 이상)로 각각 실패를 재현했습니다. 수정 뒤 통과합니다.
- `cargo test -p rbms-skin --locked --test default_pack` → 13개 통과(경로 새 시험 포함, 언어 표 일치·금지 이름 검사 포함)
- `cargo test -p rbms-player --lib --locked select / default_pack / stage::capture` → select 599개, default_pack 73개, stage::capture 62개 모두 통과(1 ignored 는 기존)
- `cargo test -p rbms-cli --locked default_skin_pack` → 1개 통과(strict 게이트)
- `cargo run -q -p rbms-cli --locked -- skin-dump assets/skins/rbms-default --strict --frames 120` → exit 0. 문서 9개 load ok, pcall·fn fail·warn 0, select 의 destinations 2172·fns 477, 평균 0.70ms, 팩 88개 파일 unchanged
- `skin-dump select.luaskin --strict --frames 120 --scenario <base+select, 21150/21151/21152 를 각각 켠 en/ko/ja>` → 세 언어 모두 exit 0, pcall·fn fail·warn 0, over budget 0
- `uv run assets/skins/rbms-default/tools/generate-images.py --check` → exit 0, 모든 시트와 atlas 가 matches
- `cargo fmt --all --check` → 통과(long_text.rs 는 cargo fmt -p rbms-player 로 정리)
- `cargo clippy -p rbms-player -p rbms-skin --all-targets --locked -- -D warnings` → exit 0
- `캡처: RBMS_SKIN_PACK 과 RBMS_SKIN_CAPTURE_DIR 로 the_browser_of_a_skin_pack_named_by_the_environment, the_default_packs_info_window_is_captured, recorded_tests, 오버레이, 첫 실행 검색, 옵션 패널, 임시 스크래치 시험(사이드 메뉴·긴 검색어·랭킹 모드·긴 경로·없는 곡 막대)` → PNG 를 Read 로 열어 확인했습니다. 스크래치 시험은 확인 뒤 지웠고 capture.rs 는 원래대로입니다. 캡처 하니스는 CJK 를 못 그려 한국어·일본어 모양은 보지 못했습니다.
