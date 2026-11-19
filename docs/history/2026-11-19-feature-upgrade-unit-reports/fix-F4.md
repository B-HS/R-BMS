# F4 수리 웨이브 보고 (wf_08935ad2-8ca)

> 수리 웨이브의 에이전트가 끝낼 때 낸 보고 원문이다.

- 상태: done

## 바꾼 파일

- apps/rbms-player/src/skin_host/play.rs
- apps/rbms-player/src/skin_host/play/tests.rs
- apps/rbms-player/src/stage/capture.rs
- apps/rbms-player/src/stage/capture/long_text.rs
- apps/rbms-player/src/stage/capture/play_rows.rs
- apps/rbms-player/src/stage/capture/side_panel.rs
- assets/skins/rbms-default/lua/motion.lua
- assets/skins/rbms-default/lua/play/info.lua
- assets/skins/rbms-default/lua/select/info.lua
- assets/skins/rbms-default/lua/select/panel.lua
- assets/skins/rbms-default/lua/select/side.lua
- assets/skins/rbms-default/lua/select/words.lua
- assets/skins/rbms-default/lua/theme.lua
- assets/skins/rbms-default/lua/ui.lua
- crates/rbms-skin/tests/default_pack.rs

## 요약

항목 1·4·5·6·7·8 을 고쳤고 2·3 은 범위 밖 사유로 넘깁니다. 항목 1: 게이지 표기를 EX-HARD 로 맞췄습니다(용어집·설정 GAUGE_LABELS·format.rs 와 같고 바꾼 곳이 5줄입니다). 항목 4: 사이드 패널 위쪽을 격자 136 으로 올렸고 패널 위 4px 를 바탕색으로 덮었습니다. 항목 5: 쓰이지 않던 radar.picture·words.algorithm_set_elsewhere·panel.WIRE 내보내기·motion.grow_up·ui.box 를 지웠습니다. 항목 6: 5자리 PLAY COUNT·CLEAR COUNT 에서 CLEAR COUNT 라벨이 숫자를 덮는 것을 재현했고, 값 자릿수에 따라 라벨 칸 폭을 줄여 고쳤습니다. 항목 7: 옵션 줄에 어시스트가 한 줄에 안 들어가면 둘째 줄로 내립니다. 짧은 조합은 94장 캡처가 모두 이전과 같습니다. 항목 2: 알림 문구의 일본어 띄어쓰기는 출력 지연 알림 한 쌍만 어긋나 있고 그것은 다른 에이전트 몫이라 건드리지 않았습니다. 항목 3: 검색 입력을 누르는 영역은 엔진이 글자 크기를 칸 높이에서 읽기 때문에 스킨만으로는 고칠 수 없어 deferred 입니다. 새 시험 3개(사이드 패널 격자, 옵션 줄 둘째 줄, 옵션 줄 글자 폭 합산)를 더했고 수정 전 실패를 확인했습니다. 선곡 쌍 시험은 세 쌍으로 넓혔고 CLEAR COUNT 에서 수정 전 실패했습니다. skin-dump strict 는 팩 전체와 9개 문서 × 세 언어 모두 통과했고 시트 검사도 통과했습니다.

## 항목별 결과

- [고침] 1. 게이지 이름 EX HARD 대 EX-HARD 통일: 조사: 설정 화면 GAUGE_LABELS(rbms-config settings.rs:149), 앱 format.rs:31·133, 용어집 1절은 모두 EX-HARD 이고, 스킨 옵션 패널·플레이 바닥 줄·결과 게이지 그래프(theme.gauge_name)와 skin_host/play.rs GAUGE_NAMES 만 EX HARD 였습니다(웹은 별개 표면이라 EX HARD, 보고서 findings 에 적음). 스킨 쪽을 바꾸면 5줄(theme.lua gauge_name [4]·[8], words.lua 의 ASSIST 규칙 행, GAUGE_NAMES, 시험 1줄), 반대로 맞추면 설정·format·용어집·골든을 모두 고쳐야 해서 EX-HARD 로 정했습니다. 같은 화면에 나란히 보이는 램프 이름 theme.clear[7] 도 EX-HARD CLEAR 로 맞췄습니다(범위를 한 줄 넓힘). 옵션 줄의 gauge_widths 는 84·148 로 다시 쟀습니다. 캡처로 옵션 패널 GAUGE 목록, 어시스트 규칙 행, 폴더 정보의 램프 이름이 칸 안에 들어가는 것을 확인했습니다. skin-dump 와 default_pack 통과. (시험: skin_host::play::tests::every_gauge_name_is_the_one_the_default_skin_s_option_panel_shows, the_gauge_text_follows_the_gauge_the_run_is_on_now(기대 문구를 EX-HARD 로 맞춤))
- [미룸] 2. 일본어 알림 자리표시자 앞뒤 공백: text/notices.rs 의 일본어를 전수 검사했습니다. 자리표시자나 영문·숫자 옆 공백은 출력 지연 알림 한 쌍(170·175행의 {1} ms 遅れて, {0} のために, 出力遅延 AUTO)에만 있고 이것은 다른 에이전트가 고치므로 건드리지 않았습니다. 나머지의 공백은 ダウンロード中 {0} / {1}, リプレイを同期中 {0}/{1} 같은 라벨 다음 값 꼴이고 기존 ステージ {0}·ベスト {0} 관례와 같아 결함이 아닙니다. 다수 관례 집계(카탈로그 44개 파일): 자리표시자 뒤 가나 붙임 104 대 공백 16, 가나 뒤 자리표시자 붙임 75 대 공백 29, 영문·숫자 뒤 가나 붙임 298 대 공백 155. 문장 안에서는 {0}の·{0}を 처럼 붙이는 것이 다수라 출력 지연 쌍도 붙이는 쪽(음が{1}ms遅れて 꼴)이 맞다고 판단합니다. 같은 꼴의 잔여는 범위 밖이라 보고만 합니다(text/select/notices.rs:21, text/menus/layout_pick.rs:60·74). (시험: 해당 없음(고친 것 없음))
- [미룸] 3. 선곡 검색 입력 누르는 영역 336x24 대 칸 360x40: 엔진 코드로 확인했습니다. 편집 가능한 글자의 누르는 영역은 글자 목적지 사각형 그대로이고(text_input.rs input_bounds), 그 높이가 곧 글자 크기(em)입니다(skin_render/text.rs 머리말). 브라우저는 SEARCHWORD 를 쓰는 첫 편집 글자 하나의 영역으로 누름을 받고 그 자리에 입력 중인 줄을 그립니다(select/search.rs skin_search_text, take_skin_text_press). 따라서 사각형을 40 으로 키우면 글자가 40px 이 되고, 큰 투명 글자를 더하면 그 누름이 브라우저의 것이 아니어서 스킨 입력이 시작됩니다. 별도 입력 영역을 주는 필드나 검색 시작 이벤트가 없습니다. 칸을 24 높이로 줄이면 머리 칩 한 줄 구조가 깨집니다. Rust 논리를 고치지 않는 조건에서는 스킨만으로 불가능하다고 판단해 손대지 않았습니다. 해결하려면 엔진에 TextDef 의 입력 영역 필드를 더하거나 검색 시작 확장 이벤트를 두어야 합니다. 밑줄(y 86~88)이 영역(60~84) 바로 밖이라 가장 눈에 띄는 누름 지점이 빗나간다는 점도 같이 적습니다. (시험: 없음(엔진 제약, 재현 시험 불가))
- [고침] 4. 선곡 사이드 패널 위쪽 128 대 격자 136: PANEL.top 을 theme.select_layout.song.top(136)으로, height 를 832 로 바꿔 아래 끝 968 은 그대로입니다. 안쪽 좌표(제목 168, 선 216, 첫 줄 240, 바닥 안내)는 모두 절대 좌표라 넘침이 없고 제목이 패널 위에서 32(=inset)가 되어 오히려 규칙적입니다. 캡처(설정·볼륨 패널)로 확인하던 중 아래 정보창 곡 머리의 장르 글자 윗머리 1px 가 패널 위(y=135)로 비쳐 보이는 것을 발견해 패널 위 4px 를 bg 색 사각형으로 덮었습니다(이것이 항목 8 의 한 건). 수정 전 캡처와 후 캡처를 직접 열어 비교했습니다. (시험: stage::capture::side_panel::the_default_packs_side_panels_start_and_end_on_the_lines_of_the_browsers_grid_with_the_ground_above_them(수정 전: 규칙이 136 행에 없어 실패. 덮개 사각형을 빼면 위쪽 바탕 검사에서 실패))
- [고침] 5. 쓰이지 않는 토큰·함수·i18n 키 삭제: Lua 전체와 luaskin, tools, Rust, 문서까지 검색해 참조가 없는 것만 지웠습니다. 삭제: theme.radar.picture(지역 RADAR_PICTURE 는 radius 계산에 남음), select/words.lua 의 algorithm_set_elsewhere, select/panel.lua 의 panel.WIRE 내보내기(지역 WIRE 는 남음), motion.grow_up, ui.box. i18n 키 237개는 동적 이름(listed·lines 의 접두어)까지 풀어 대조해 쓰이지 않는 키가 0개입니다. 처음에 Lua 만 검색해 theme.judge_word, theme.tone.rose, theme.note.shade 도 지웠다가 sheet 검사(generate-images.py --check)가 judge_word 를 못 찾는다고 실패해, 이 셋이 이미지 생성기(tools/sections/play.py)가 읽는 값임을 알고 되살렸습니다. props.extension_float 는 Rust 시험(skin_host/tests.rs:598)이 고정하고 있어 그대로 둡니다. (시험: uv run assets/skins/rbms-default/tools/generate-images.py --check(모든 시트 matches), default_pack 14개, skin-dump strict)
- [고침] 6. 선곡 영어 CLEAR COUNT 라벨 4자리 이상 겹침 의심: 재현했습니다. 긴 글 캡처에 한 차트 10,000판(모두 클리어)을 넣어 PLAY COUNT·CLEAR COUNT 를 5자리로 만들자 CLEAR COUNT 라벨 끝이 숫자 첫 자리를 덮었습니다(글자 덩어리 14개, 기대 15개). PLAY COUNT 는 숫자와 2~3px 로 붙었습니다. 넘침 규칙대로 MAX COMBO·PLAY COUNT·CLEAR COUNT 의 라벨을 값 자릿수 구간(1~3, 4, 5자리 이상) 세 갈래로 그리고 구간마다 라벨 칸 폭을 쌍 폭 192 에서 그 자릿수의 숫자 폭과 간격 8 을 뺀 값으로 줍니다. 라벨은 SHRINK_CUT 라 5자리에서 CLEAR COUNT 가 약 0.87 배로 줄고(하한 0.75 안) 겹치지 않습니다. 1~3자리 값의 모양은 이전과 같고, 한국어·일본어 라벨은 최악 가정(전각 1em)으로도 칸 안입니다(실제 글꼴 모양은 하니스가 CJK 를 못 그려 보지 못함). 수정 뒤 캡처를 직접 열어 세 쌍이 모두 떨어져 있는 것을 확인했습니다. (시험: stage::capture::long_text::the_default_packs_info_window_keeps_its_lines_apart_and_its_five_digit_numbers_whole_and_clear(세 쌍으로 넓힘. 수정 전 CLEAR COUNT 에서 실패, 수정 후 통과))
- [고침] 7. 플레이 옵션 줄 가장 긴 조합에서 SHRINK 로 읽을 수 없음: 가장 긴 조합(EX-HARD CLASS · R-RANDOM · SCRATCH AUTO · HCN · EX JUDGE · CONSTANT · LEGACY NOTE)을 캡처하는 하니스(RunStart)를 만들어 수정 전을 재현했습니다: 5키는 나머지 글자가 약 110px 로 눌려 읽을 수 없고 14키 첫 칸도 0.59 배였습니다. 발치는 첫 줄 맨 위 1036 에서 둘째 줄 맨 위 1060 이면 아래 끝이 1072 라 문서 높이 1080 안이고 AUTOPLAY·REPLAY 표지는 첫 줄이라 겹치지 않습니다. 첫 줄에 어시스트까지 들어가는지는 글자별 폭 합산(Inter 18px 표, 엔진 실측 대비 -1%~+3%)으로 가려, 안 들어가면 첫 줄은 게이지·배치만 쓰고 어시스트를 둘째 줄(칸 전체 폭)에 씁니다. 결과: 둘째 줄 눌림은 5키 0.67, 7키·14키 0.86, 넓은 레인·9키 1.0 입니다. 캡처 비교: 10개 모드·배치의 짧은 조합 10장은 이전과 픽셀 단위로 같고, 기본 플레이 캡처 게이트 94장도 이전과 같습니다. 어시스트가 있는 중간 조합은 5키 기본 폭의 MIRROR+SCRATCH AUTO 한 건만 달라졌고(이전에는 0.72 배로 눌려 한 줄이었음) 나머지는 같습니다. 5·7·9·10·14키, 넓은 레인, 필드 좌우 뒤집기를 모두 캡처로 열어 봤습니다. (시험: stage::capture::play_rows::the_default_pack_writes_the_assists_of_a_run_under_its_options_line_only_where_they_do_not_fit_beside_it(수정 전 5키 longest 가 1줄이라 실패, 수정 후 통과. 줄 수·화면 안·눌림 하한 0.65 확인) 와 rbms-skin default_pack::the_play_screen_adds_up_the_width_of_its_options_line_to_within_three_percent_over_the_text_engines)
- [고침] 8. 위를 고치며 보인 다른 어긋남: (a) 항목 4 에서 본 사이드 패널 위 장르 글자 윗머리 1px 비침을 덮개 사각형으로 가렸습니다(이유: 패널을 격자에 맞추면 이전에는 가려지던 글자 윗머리가 드러남). (b) 항목 6 의 PLAY COUNT 2~3px 접촉을 같은 규칙으로 고쳤습니다. (c) 선곡 캡처를 수정 전후로 비교하니 차이는 시계, 램프 이름 EX-HARD CLEAR, 옵션 패널 EX-HARD 뿐이었고, select-unmoved 의 선택 막대 왼쪽 띠 밝기 차이는 원본 팩으로 다시 떠도 같게 나오는 환경 요인(첫 실행 대 이후)이라 결함이 아닙니다. (시험: 항목 4·6 의 시험이 포함)

## 그 밖에 찾은 것

- [low, 안 고침] 웹에 EX HARD 표기가 남아 있음(앱·스킨은 EX-HARD): web/messages/{en,ko,ja}/browse.json:466, web/src/shared/constants/clear-lamp.ts:28, web/src/entities/ai.type.ts:366·370, web/tests/components/lamp-label.test.tsx:18. IIDX 공식 일본어 도움말도 EX HARD 라서 웹 쪽 표기 결정은 웹 소유자와 따로 해야 하고 범위 밖이라 고치지 않았습니다.
- [low, 안 고침] 일본어 자리표시자 앞뒤 공백이 같은 꼴로 남은 곳: apps/rbms-player/src/text/select/notices.rs:21 의 リプレイ {0} をダウンロード中..., apps/rbms-player/src/text/menus/layout_pick.rs:60·74 의 {0} シード {1} でプレー·RANDOM シード {0} でプレー. 문장 안 조사 앞 공백이라 붙이는 것이 다수 관례(104 대 16)에 맞습니다. 요청한 범위(text/notices.rs)가 아니고 다른 에이전트가 같은 카탈로그를 고칠 수 있어 건드리지 않았습니다.
- [low, 안 고침] 출력 지연 알림 한 쌍의 일본어 공백(text/notices.rs 170·175행): {1} ms 遅れて, {0} のために, 出力遅延 AUTO 가 공백으로 쪼개져 있습니다. 지시대로 다른 에이전트 몫이라 건드리지 않았습니다. 붙이는 꼴(音が{1}ms遅れて 등)이 카탈로그 다수 관례와 맞습니다.
- [low, 안 고침] 검색 입력 누르는 영역이 그려진 칸보다 작음(엔진 제약): 항목 3 참고. 영역은 336x24(y 60~84)이고 밑줄은 y 86~88 이라 가장 자연스러운 누름 지점이 빗나갑니다. 엔진 변경 없이는 불가.
- [low, 고침] 이미지 생성기가 읽는 theme 값을 Lua 만 검색해 죽은 값으로 오판할 뻔함: theme.judge_word, theme.tone.rose, theme.note.shade 는 tools/sections/play.py 가 읽습니다. 처음 지웠다가 generate-images.py --check 실패로 알고 되살렸고 최종 상태는 시트 검사 통과입니다. 삭제 후보 판정은 tools 폴더까지 포함해야 합니다.

## 남은 위험

- 한국어·일본어의 실제 모양은 캡처 하니스가 CJK 를 못 그려 보지 못했습니다. 카운트 쌍 라벨은 전각 1em 최악 가정으로 칸 안임을 계산했고, 옵션 줄의 배치·어시스트 이름은 영어 상수라 언어와 무관합니다. 실창 확인이 필요합니다.
- 옵션 줄의 폭 합산표는 번들 Inter 18px 에 맞춘 값입니다. 글꼴이나 caption 크기를 바꾸면 표를 다시 뽑아야 하고, 엔진 실측 대비 -1%~+3% 를 지키는지 default_pack 시험이 잡습니다. 표가 약간 크게 나오는 쪽(kerning 차이)이라 둘째 줄로 내리는 쪽으로 기웁니다.
- 5키 기본 폭에서 가장 긴 어시스트 줄은 둘째 줄에서도 0.67 배로 눌립니다(이전 0.14 배에서 개선, 눌림 하한 시험 0.65). 5키 기본 폭의 MIRROR + SCRATCH AUTO 는 이전에는 0.72 배로 눌린 한 줄이었는데 이제 두 줄입니다.
- 사이드 패널 위 4px 덮개는 패널이 열려 있는 동안 x 64~1096 의 y 132~136 을 bg 색으로 가립니다. 지금은 아래 글자 윗머리만 가리지만 그 자리에 뭔가 새로 그려지면 가려집니다.
- theme.clear[7] 이름을 EX-HARD CLEAR 로 바꾼 것은 요청한 게이지 이름 밖의 한 줄입니다(같은 화면에 나란히 보여서). 결과 화면 램프 칸은 SHRINK 라 폭 증가(약 2%)는 흡수됩니다.
- 검색 입력 누르는 영역은 해결되지 않았습니다(엔진 변경 필요).
- 긴 글 캡처 데이터가 한 차트 10,000판으로 늘어 해당 시험이 약 9초가 됩니다(이전 수 초).

## 갱신이 필요한 문서

- docs/skin.md: 21011 게이지 이름 목록(EX-HARD·EX-HARD CLASS), 선곡 1.4절·3.4.6절의 MY SCORE 쌍 라벨(값 자릿수 세 구간으로 라벨 칸 폭을 줄임)과 사이드 패널 위쪽 136·위 4px 덮개, 플레이 옵션 줄의 둘째 줄 규칙(info.text_width 폭 합산표, 줄 간격 24, 첫 줄에 안 들어가면 어시스트를 둘째 줄로)과 gauge_widths 84·148, 삭제된 토큰·함수(theme.radar.picture, words.algorithm_set_elsewhere, panel.WIRE, motion.grow_up, ui.box)
- docs/reference/2026-10-24-i18n-glossary.md: 7절에 'EX HARD(스킨·플레이 화면) 대 EX-HARD' 어긋남이 해소됨(스킨도 EX-HARD, 웹만 EX HARD)을 기록, 1절 클리어 램프 행 옆에 스킨 램프 이름 EX-HARD CLEAR 주석
- docs/reference/option-panels.md: 98·210행 스킨 GAUGE 목록 EX HARD 를 EX-HARD 로, 274행 'GAUGE 이름이 화면마다 다르다'와 737행 통일 여부 질문을 해소로 정정
- docs/reference/clear-lamps.md: 41·46행 스킨 램프 이름 EX HARD CLEAR 를 EX-HARD CLEAR 로(앱 EX-HARD, 웹 EX HARD 와의 표기 차이 설명 갱신)
- docs/plan/2026-10-11-default-skin-design.md: 339행 ui.box 와 384행 motion.grow_up 을 API 표에서 삭제, 206·564행 램프 이름 갱신
- docs/acknowledge/2026-10-29-adr-select-info-overflow.md: 4절 MY SCORE 통계 쌍에 라벨 칸 폭 규칙(자릿수 구간별 쌍 폭에서 숫자 폭과 간격 8 을 뺌, SHRINK_CUT)과 5자리 PLAY COUNT·CLEAR COUNT 겹침 해소를 추가
- docs/plan/2026-11-19-feature-upgrade-12.md: 9절 결과에 이번 다듬기(게이지 표기 통일, 사이드 패널 격자, 카운트 라벨, 옵션 줄 둘째 줄, 데드 토큰 삭제, 검색 클릭 영역은 엔진 제약으로 보류)를 한 절로 기록
- docs/quality-assurance: 실창 점검 항목 추가(세 언어에서 5자리 PLAY COUNT·CLEAR COUNT 라벨, 가장 긴 옵션 조합의 두 줄 옵션 줄을 5·7·9·10·14키와 넓은 레인·필드 뒤집기로, 사이드 패널 두 종류의 위쪽 선), 테스트 부채: 검색 입력 누르는 영역(엔진에 입력 영역 필드 필요, 밑줄이 영역 밖), 한·일 옵션 줄 글자 폭 실측, 글꼴 바뀔 때 옵션 줄 폭표 재생성
- docs/history/2026-11-19-feature-upgrade-unit-reports/test-S3.md·test-S4.md·test-O4.md: 원문은 그대로 두고 계획서 결과 절에서 이 항목들이 닫혔음을 가리킴

## 돌린 검사

- `cargo run -q -p rbms-cli --locked -- skin-dump assets/skins/rbms-default --strict --frames 120` → exit 0. 문서 9개 load ok, pcall·fn fail·warn 0, over budget 0, 팩 88개 파일 unchanged(select destinations 2204)
- `세 언어(시나리오 불린 21150~21152, base.json + 문서별 시나리오 병합)로 문서 9개 × 3언어 skin-dump --strict --frames 120(select·play5·play7·play9·play10·play14·result·decide·courseresult)` → 27개 모두 exit 0, pcall·fn fail·warn 0, over budget 0
- `uv run assets/skins/rbms-default/tools/generate-images.py --check` → exit 0, 모든 시트와 atlas matches(중간에 judge_word 삭제로 실패했다가 복구 뒤 통과)
- `cargo test -p rbms-skin --locked` → 전부 통과(default_pack 14개, 새 시험 1개 포함)
- `cargo test -p rbms-player --lib --locked stage::capture` → 65개 통과(새 시험: 사이드 패널, 옵션 줄 둘째 줄, 긴 글 쌍 시험 확장)
- `cargo test -p rbms-player --lib --locked skin_host / stage::select / stage::play` → 403개 / 420개 / 302개 통과
- `cargo test -p rbms-cli --locked default_skin_pack` → 1개 통과(strict 게이트)
- `cargo clippy -p rbms-player -p rbms-skin --all-targets --locked -- -D warnings` → 경고 0(type_complexity 한 건은 type 별칭으로 고침)
- `cargo fmt --all --check` → 통과
- `RBMS_SKIN_PACK=... RBMS_SKIN_CAPTURE_DIR=... cargo test -p rbms-player --lib --locked the_browser_of_a_skin_pack_named_by_the_environment, the_play_scene_of_the_default_pack_is_captured_with_its_rows_switched, the_default_packs_info_window_is_captured..., the_options_line_of_the_default_pack_is_captured...(opt-in)` → 수정 전·후 PNG 를 모두 열어 비교. 플레이 게이트 94장 전부 수정 전과 동일, 옵션 줄 짧은 조합 10장 동일, 선곡은 시계·EX-HARD 표기 외 동일
- `수정 전 실패 확인(원본 Lua 를 git show HEAD 로 잠시 되돌려 실행)` → 사이드 패널 시험·옵션 줄 시험·긴 글 쌍 시험이 각각 실패하고 복구 뒤 통과
