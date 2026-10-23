# 선곡 화면 IIDX 식 개편 조사 (요구 5)

최종 갱신 2026-10-24 · 대응 커밋 `9fed3c0`(dev) · 읽기 전용 조사다(코드를 고치거나 빌드·실행하지 않았다, 운영 사이트에 요청하지 않았다). 외부 사실은 출처 URL 을 붙였고 확인하지 못한 것은 "확인 못 함"이라 적었다. 추정 수치는 "추정(미측정)"이라 표시했다.

## 1. 요구 해석

### 1.1 사용자 원문(요구 5)

"UI/UX 를 조금 더 꾸며봐봐. 또한 곡선택부분에 보면 곡 + 왼쪽에 스코어도 들어가있는데 이것도 구현하도록하고, 기타 화면에 있는것처럼 조금 더 iidx와 비슷하게끔 구현 진행해봐 현재 r-bms 에있는 내용도 표시 가능하게하고. rival 기능은 당장은 필요없는데 해당 UI 부분에 다른 수치를 넣는다던가 하면되겠지"

### 1.2 해석

1. 곡 막대에 왼쪽 등급 배지(스코어)와 오른쪽 자켓 썸네일을 넣는다. 이것이 "곡 + 왼쪽에 스코어"다.
2. 왼쪽 정보창을 IIDX 곡 선택 화면의 구성(MUSIC DATA, SCORE DATA 등)으로 다시 짠다. 지금 R-BMS 선곡 화면에 있는 값(LEVEL, BPM, NOTES, LENGTH, JUDGE, TOTAL, KEYS, 난이도, EX SCORE, MAX COMBO, PLAY COUNT, CLEAR COUNT, RATE, MISS COUNT, 램프, 판정 수, 폴더·코스 정보)은 계속 보여야 한다.
3. RIVAL SCORE DATA 자리는 라이벌 기능 없이 다른 수치로 채운다.
4. 표지 이미지 8번은 참고 화면 9번의 NOTES RADAR 를 잘라 낸 것이다. 사용자가 따로 첨부했으므로 레이더도 구현 대상으로 읽었다.
5. 이모지·아스키아트 금지, 자작 기본 스킨의 모노톤 방향과 "IIDX 와 비슷하게"가 부딪히는 지점은 5절 질문 1 로 묻는다.

### 1.3 참고 화면(`images/9.png`, `images/8.png`)에서 읽은 구성 요소 전체

| 기호 | 구성 요소 | 눈으로 읽은 내용 |
| --- | --- | --- |
| A1 | MUSIC SELECT 제목과 원형 엠블럼 | 왼쪽 위 |
| A2 | PLAY STYLE 배지 | `SP` |
| A3 | STAGE 표시 | `FINAL STAGE` |
| A4 | TIME REMAIN | 오른쪽 위 원형, 숫자 42 카운트다운 |
| A5 | 위치 경로 | `カテゴリ > フォルダ > 楽曲` 3칸, 마지막 칸이 주황으로 켜짐 |
| A6 | 카테고리 아이콘 탭 | カスタム, ノーツレーダー, 全フォルダ(선택), 曲名, DJ DATA |
| A7 | 카테고리 선택 조작 안내 | 건반 검은 키 길게 + 턴테이블 |
| A8 | 정렬 라벨 말풍선 | `ミスカウント` |
| B1 | 막대 왼쪽 등급 배지 | AAA 금색 면, AA 은회색 면, A 갈색 테두리 어두운 면. 기록 없는 막대는 비어 있다. 선택된 막대는 배지가 막대 바깥 왼쪽으로 돌출 |
| B2 | 램프 색 띠 | 배지 오른쪽, 레벨 칸 왼쪽의 얇은 세로 띠(초록, 하늘, 흰색 등) |
| B3 | 레벨 숫자 칸 | 난이도 색 테두리 상자 안의 큰 숫자 |
| B4 | 곡 제목 | 막대 안 글자, 길면 잘림 |
| B5 | 오른쪽 자켓 썸네일 | 막대 오른쪽 끝 정사각 |
| B6 | 선택 막대 강조 | 막대가 커지고 왼쪽으로 튀어나오며 초록 테두리와 밝은 면 |
| B7 | 선택 곡 자켓 확대 | 선택 막대 위쪽 왼쪽에 큰 그림이 뜸 |
| B8 | CLEAR VISUALIZATION | 오른쪽 세로 누적 막대와 세로 글자(NO PLAY, FAILED, ASSIST, EASY, CLEAR, HARD, EX-HARD, FULL COMBO). 아래쪽 초록이 크고 맨 아래 조금 파랑 |
| B9 | CURRENT MUSIC POSITION | 맨 오른쪽 가장자리의 보라색 세로선 |
| C1 | 왼쪽 큰 배경 일러스트 | 곡 쪽 그림이 왼쪽 전체 배경 |
| C2 | 버전·작곡자 로고 | 제목 위 |
| C3 | 제목 | 큰 글자 + 위에 작은 읽기·장르 줄(`エレクトロ`) |
| C4 | 아티스트 | 제목 아래 |
| C5 | BPM | 아티스트 아래 `124 BPM` |
| C6 | NOTES RADAR(이미지 8) | 육각형 6축 NOTES(분홍), PEAK(주황), SCRATCH(별 표지), SOF-LAN, CHARGE(남보라), CHORD(연두). 노란 반투명 다각형 |
| C7 | 마스코트 캐릭터 | 중앙 아래 |
| D1 | MUSIC DATA 패널 | `TOTAL NOTES 570` |
| D2 | DIFF 탭 | `1 3 [6] 9` 4칸, 선택 칸 주황 |
| D3 | 노트 밀도 그래프 | 시간 축 히스토그램 |
| D4 | PLAYER01 LANE | 레인별 노트 수 8개(`110 79 52 68 76 67 63 55`)와 막대(스크래치 빨강, 건반 흰·파랑) |
| D5 | PLAYER02 LANE | SP 에서는 비어 있음 |
| D6 | SCORE CLASS | 금·은·동 메달 3개와 점수 3개(`1119`, `1086`, `1040`으로 읽힘) |
| D7 | CLEAR RATE | `94.7%` |
| E1 | SCORE DATA 패널(왼쪽 아래) | `SCORE 0977`, `MISS COUNT 0002` |
| E2 | 큰 등급 글자 | `AA` |
| E3 | 램프 이름 | `EASY CLEAR` |
| E4 | 다음 등급까지 차이 | `AAA - 0037` |
| E5 | F~AAA 진행 막대 | F E D C B A AA AAA 칸, 현재까지 채움 |
| E6 | PLAYER 01 SIDE 표지 | 패널 아래 |
| F1 | RIVAL SCORE DATA 6행 | 순위, 이름, 등급 배지, 점수, 램프 이름(`E-CLEAR`, `EXH-CLEAR` 등) |
| F2 | 점포 순위 행 | `店内` 표지 + 이름 `MI-OH?` + 등급 AAA + 점수 1067 (이름 앞에 왕관 모양 아이콘이 있다) |
| G1 | 하단 INFORMATION 띠 | 선택 노브, 폴더를 닫는다·결정, START BUTTON ゲームオプション変更, EFFECT ON/OFF COMPRESSOR 3, VEFX CHANGE 難易度変更, 詳細情報表示 |
| H1 | PASELI, CREDIT 표시 | 아케이드 전용 |

C2, C7, H1 은 R-BMS 에 대응이 없거나(권리·자산) 의미가 없어 대상에서 뺀다(6절).

## 2. 현재 R-BMS 상태(관련 코드 지도)

### 2.1 화면을 그리는 두 경로

| 경로 | 해상도 | 위치 | 언제 |
| --- | --- | --- | --- |
| 기본 스킨 `rbms-default`(Lua) | 1920x1080 (`assets/skins/rbms-default/lua/theme.lua:5-6`) | `lua/select/{main,list,info,controls,side,sheet,panel,words,inform,kit}.lua`, 합계 2,070줄 | 평상시. 첫 실행부터 이 문서가 선곡을 그린다(`docs/skin.md` 1.1절) |
| 내장 선곡 렌더러 | 1280x720 고정(`docs/reference/ui-select-redesign.md` 1절) | `crates/rbms-render/src/select.rs`(`SelectRow`, `DetailView`, `SelectDetail`), 레이아웃 `select_layout`(`docs/theme.md` "Layout fields") | 스킨이 없거나 읽히지 않을 때의 폴백만 |

좌표 체계(기본 스킨): 격자 8px, 바깥 여백 64, 열 120·간격 32·12열, 머리 구분선 104, 바닥 구분선 1000(`theme.lua:234`). 글자 크기 단계 display 96, title 64, heading 44, body 32, label 24, caption 18(`theme.lua:223`). 글자 윗선은 크기의 0.727 로 환산한다(`theme.cap_height`).

### 2.2 기본 스킨 선곡 화면의 구성 요소 지도

| 영역 | 파일:줄 | 좌표(1920x1080) | 내용 |
| --- | --- | --- | --- |
| 머리 띠 | `lua/select/main.lua:30-35`, `controls.lua` | 0..104 | "MUSIC SELECT"(64,70), SEARCH 상자(456..1096), MODE·SORT·LN MODE 버튼(1128, 1368, 1608, 폭 224·224·248) |
| 곡 머리 | `info.lua:19-27` | x 64..1096, top 136..336 | 난이도 색 막대 8px, 경로 18, 장르 24, 제목 64, 부제 24, 아티스트 32 |
| 스테이지 그림 | `info.lua:29` | (64,368) 424x318 | `-100` 참조 그림. 없으면 "NO STAGE IMAGE" |
| 곡 항목 8칸 | `info.lua:31-40` | 열 x 520·712·904, 행 368·485·602 | LEVEL, BPM, NOTES, LENGTH, JUDGE, TOTAL, KEYS, DIFFICULTY |
| 기록 | `info.lua:42-66`, `265-330` | 구분선 702, 행 718·808·898 | RANK(96), CLEAR 이름, EX SCORE, MAX COMBO, PLAY COUNT, RATE, MISS COUNT, CLEAR COUNT, 판정 5행과 막대 |
| 폴더 정보 | `info.lua:80-97` | top 368..912 | CHARTS 수, 램프 11종 개수 표 |
| 코스 정보 | `info.lua:98` | top 368..960 | 단계 곡명 5줄, 제약 줄 |
| 곡 목록 | `list.lua:8` | x 1128, 폭 728, 막대 높이 64, 피치 72, 15칸(가운데 7번, 가운데 top 520) | `songlist` 한 개 |
| 막대 안 | `list.lua:15-47` | 램프 (0,0,8x64), 레벨 `num24` x 16, 트로피 x 104, 제목 x 80(곡)·136(그 밖), 라벨 오른쪽 끝 714 세로 3단, 분포 그래프 (136,54,576x4) | 등급 배지·자켓은 없다 |
| 선택 틀 | `list.lua:10`, `61` | x 1096, 폭 760 | `raised` 면 + 왼쪽 `accent` 띠 8px(`motion.pulse(2000,160)`) |
| 스크롤 막대 | `list.lua:47`, `scrollbar` | x 1872, top 136, 12x832 | 위치 슬라이더(`RATE_MUSICSELECT_POSITION`) |
| 바닥 | `controls.lua`(`HINT_ROWS`, `REPLAY`, `ACTIONS`) | 1000..1080 | 안내 2줄, REPLAY 4칸, AUTOPLAY·PRACTICE·FAVORITE |
| 사이드 탭·패널 | `side.lua` | x 0..48, 패널 (64,128,1032x840) | QUICK SETTINGS, VOLUME |
| 옵션 시트 | `sheet.lua`, `panel.lua`, `words.lua`, `inform.lua` | (64,312,1792x456) | START·SELECT 로 여는 옵션 패널 3장 |

현재 화면 캡처는 `docs/assets/screenshot-select.jpg` 다(막대에 등급·자켓이 없고 왼쪽 큰 상자가 스테이지 그림 자리라는 것을 확인했다).

### 2.3 데이터가 스킨까지 가는 길

1. 막대마다의 값: `SelectBar`(`apps/rbms-player/src/lib.rs:568-588`)가 막대 하나의 모델이다. `BarChart.best: Option<BarScore{ex,max_ex}>`(`lib.rs:497`, `516`)로 막대마다 최고 EX 가 이미 있고, `BarChart.images`(`lib.rs:508`)에 스테이지 그림·배너·배경 그림 경로가 스킨이 읽을 때 채워진다(`stage/select/list.rs:747` `chart_bar`). `wheel_bar`(`stage/select/scene.rs:86-99`)가 이를 `SongBar`(`crates/rbms-render/src/skin_render/songlist/bars.rs:203`)로 바꾼다. `SongBar` 에는 레벨, 난이도, 램프, 트로피, features 만 있고 등급과 그림은 없다.
2. 커서 곡의 값: `chart_under_cursor`(`list.rs:570`)가 `ChartMeta`(`skin_host/chart.rs`)를 만들고, 커서가 150ms 쉬면(`FOCUS_DETAIL_DEBOUNCE`, `lib.rs:192`) `refresh_focused_detail`(`stage/select/mod.rs:643-660`)이 `chart_detail`(`apps/rbms-player/src/library.rs:112`)로 곡 파일을 읽고 밀도·BPM 범위·길이를 채운다. 결과는 `song_detail` 표에 캐시된다.
3. 기록 값: `SelectState`(`skin_host/select.rs`)가 커서 막대의 최고 기록을 `ScoreSheet`·`ScoreStanding`(`skin_host/score/standing.rs`)으로 풀어 EX, MISS COUNT, 판정 수, 비율, 등급 옵션, 다음 등급까지 EX(`NUMBER_DIFF_NEXTRANK`, `select.rs:238`)를 답한다.
4. IR 값: `IrBrowser`(`skin_host/ir.rs:159-191`)가 커서 곡의 순위 상태·내 순위·참가자 수·램프별 개수를 답하지만 순위 목록(이름 120~129, 점수 380~389, 클리어 390~399)은 일부러 부재 값으로 답한다(`ir.rs:249`, `414`, `432`, `451`). 순위 값을 받아오는 것도 순위 패널이 열려 있을 때만이다(`stage/select/mod.rs:883` `update_ranking`).

### 2.4 곡 막대 그리기: 원본과의 대응

R-BMS 의 `songlist` 는 beatoraja 의 `SkinBar`·`BarRenderer` 를 옮긴 것이고(`crates/rbms-render/src/skin_render/songlist.rs` 머리 문서), 원본이 막대마다 그리는 것은 모두 지원한다: 막대 그림 on/off(종류 7), 제목 11종, 레벨 7종, 램프 11종, 트로피 3종, 라벨 5종, 폴더 분포 그래프, 라이벌 선택 때의 `playerlamp`·`rivallamp`(`SongBars.rival`, `bars.rs:464`. 이 앱은 라이벌을 고르지 않으므로 `rival_lamp` 는 항상 `None`, `lib.rs:564`). 원본에 없는 것(등급 배지, 막대별 그림)은 R-BMS 에도 없다.

### 2.5 디자인 방향 결정(IIDX 식 요구와의 충돌 지점)

- `docs/plan/2026-10-11-default-skin-design.md:133`: "순수한 검정 바탕, … 직각 평면이다. 그림자·광택·그라디언트·둥근 모서리·배경 이미지를 쓰지 않는다 … 색을 가진 것은 뜻이 색에 실린 것(판정, 빠름·느림, 램프, 난이도, 게이지, 그래프)과 플레이의 노트·키 빔·폭발뿐". 근거는 2026-10-12 사용자 요청 "전체적으로 검/회색/흰의 모노톤"(같은 문서 1200줄)과 D18(`docs/acknowledge/2026-10-09-lua-skin-compat-decisions.md:50`, 앱 UI 무채색)이다. `docs/theme.md` 도 같은 방향이다.
- 이미 색을 가진 선례: 코스 트로피(동·은·금)는 `TROPHY_COLORS` 로 막대에 색이 있다(`list.lua:19`). 등급 배지·메달의 금·은·동은 이 선례와 같은 "뜻이 색에 실린 것"으로 둘 수 있다.
- 충돌: IIDX 화면은 큰 배경 일러스트, 발광, 채색 패널이 특징이다. 구조(배치·정보 구성)를 따르고 색·질감은 모노톤으로 두는 안이 결정을 어기지 않는다. 사용자가 색까지 따르길 원하는지는 5절 질문 1 이다.
- 6절 이전의 설계서가 선곡에서 "뺀 것"으로 적은 것: BPM 연동 캐릭터, IR 램프·랭킹 패널, 라이벌 비교, 배너, 종류별 노트 수·밀도·주 BPM(`default-skin-design.md:626`).

### 2.6 문서와 코드가 어긋난 곳(고치지 않고 보고만)

- `docs/skin.md:698` "알려진 한계"는 "선곡: 종류별 노트 수(350~353), 밀도(360~365), 주 BPM(92)은 빈칸"이라 적었다. 코드에서는 밀도 360~365 가 커서가 150ms 쉰 뒤 채워진다(`skin_host/chart.rs` `with_detail`, `read_integer` 의 `NUMBER_DENSITY_*`, 테스트 `skin_host/chart/tests.rs:133-138`). 종류별 노트 수(350~353)는 정말 빈칸이다(`chart_under_cursor` 가 `note_counts` 를 채우지 않는다). 주 BPM 은 BPM 이 하나인 곡만 채운다(`list.rs:592`).
- 같은 한계 표는 IR 순위 값을 "스킨에 연결되지 않았다"고 적었고 코드와 맞다(`ir.rs:249`).

## 3. 원본(beatoraja)·IIDX 기준

### 3.1 beatoraja 선곡 스킨이 막대와 정보창에 쓰는 것(`/Users/hyunseokbyun/development/beatoraja/src`, 읽기만)

| 항목 | 원본 | 근거 |
| --- | --- | --- |
| 막대 한 개가 가진 그림 | 막대 on/off(최대 60종), 트로피 3, 텍스트 11, 레벨 7, 라벨 5, 램프 11 + 내 램프 11 + 라이벌 램프 11, 폴더 분포 그래프 1 | `bms/player/beatoraja/select/SkinBar.java:25-93` |
| 막대 그리는 순서 | 막대 그림 → 폴더 그래프 → 제목 → 트로피 → 램프(라이벌 선택 때는 내 램프+라이벌 램프) → 레벨 → 라벨 | `BarRenderer.java:249-415` |
| 막대별 등급 배지, 막대별 자켓 | 없다. 막대는 위 목록의 것만 그린다 | `SkinBar.java` 전체, `BarRenderer.java:249-415` |
| 선곡의 노트 분포·BPM 그래프 | `judgegraph` type 0 은 선택 곡의 `SongInformation` 분포를, `bpmgraph` 는 곡 정보로 그린다 | `skin/SkinNoteDistributionGraph.java:171-172`, `skin/SkinBPMGraph.java:110` |
| IR 순위 슬롯 | 이름 string 120~129(`ranking1..10_name`), EX number 380~389, 순위 index number 390~399, 이미지 인덱스 380~389(플레이어 종류)·390~399(클리어 종류). 목록이 스크롤되면 `rankingOffset` 을 더한다 | `skin/property/IntegerPropertyFactory.java:876-907`, `985-986`, `1000-1003`, `StringPropertyFactory.java:84` |
| ModernChic 선곡 | `bmsanalysis.lua` 가 judgegraph 1, bpmgraph 1 을 쓰고 `qco` 가 IR 순위 슬롯을 읽는다 | `docs/reference/skin-compat/m4-select.md:97`, `200-201`, `476-486` |

결론: 등급 배지와 막대 자켓은 원본 스킨 형식으로는 표현할 수 없다. 노트 분포·BPM 그래프·IR 순위 슬롯은 원본에 id 와 객체가 이미 있는데 R-BMS 선곡이 값을 주지 않는 것이다(`stage/select/mod.rs:1442` 의 `FrameData` 에 `notes`·`bpm` 이 없다. `ir.rs` 가 순위 슬롯을 부재로 답한다).

### 3.2 IIDX 쪽 외부 사실(RemyWiki 요약 도구 인용, 원문 대조는 하지 못함)

| 항목 | 확인한 내용 | 출처 |
| --- | --- | --- |
| NOTES RADAR 6축 | NOTES 평균 노트 밀도, PEAK 한 구간의 최고 노트 밀도, SCRATCH 평균 스크래치 밀도, CHARGE (헬) 차지 노트와 백스핀 스크래치 수, CHORD 동시치기 수, SOF-LAN BPM 변화의 횟수 또는 심한 정도. 가장 강한 축에 별. 레이더 색은 난이도에서 정한다. HEROIC VERSE 에서 도입 | https://remywiki.com/AC_HEROIC_VERSE |
| NOTES RADAR 값의 정규화 | "확인 못 함"(0~100 범위인지, 레벨 대비인지 문서에 없다) | 같은 문서 |
| SCORE CLASS | "SCORE CLASS medal system is introduced", "SCORE CLASS view will show the required score needed to earn a specific medal for each chart". 즉 곡마다 메달을 따는 데 필요한 점수를 보여 준다. 필요 점수를 정하는 방법은 "확인 못 함" | https://remywiki.com/AC_ZINRAI |
| CLEAR RATE | "ratio of clears to total attempts by all players on the e-amusement network on a given chart"(IIDX RED 부터) | https://remywiki.com/IIDX_Glossary |
| DJ LEVEL 경계 | AAA 부터 F, 각 등급이 최대 EX 의 1/9 씩, 2/9 까지. 정확한 컷 표는 문서에 없다. R-BMS 의 `dj_rank` 가 이미 9분의 1 경계(`crates/rbms-render/src/result.rs:127-139`)라 일치한다 | https://remywiki.com/IIDX_Glossary |
| CLEAR VISUALIZATION, RIVAL SCORE DATA, MUSIC DATA, 레인별 노트 수, DIFF 탭, TIME REMAIN | 위 세 문서에서 "확인 못 함". 화면에서 읽은 모양만으로 R-BMS 값에 대응시켰다 | https://remywiki.com/AC_ZINRAI, https://remywiki.com/AC_HEROIC_VERSE |

## 4. 설계안

### 4.1 구성 요소별 데이터 가용성(핵심 표)

"있음"은 R-BMS 가 이미 가진 데이터이고, "스킨 API"는 지금 Lua 스킨이 읽을 수 있는지다.

| 기호 | R-BMS 데이터 | 스킨 API(속성 id) | 새 값·새 id | 판정 |
| --- | --- | --- | --- | --- |
| A2 PLAY STYLE | 곡 모드 | 옵션 `OPTION_7KEYSONG` 등(`chart.rs` `read_boolean`), 텍스트 `MODE_FILTER` | 없음(Lua 에서 SP/DP 로 묶음) | 바로 가능 |
| A3 STAGE | 자유 선곡에는 단계가 없다. 코스 막대 옵션만 있다 | `OPTION_GRADEBAR` | 없음 | "FREE PLAY"/"COURSE" 표지로 대체 |
| A4 TIME REMAIN | 제한 시간 없음. 시각과 앱 가동 시간은 있다 | `NUMBER_TIME_HOUR/MINUTE`(24, 25), `NUMBER_OPERATING_TIME_*`(27~29)(`skin_host/system.rs:167-172`) | 없음 | 현재 시각 또는 세션 시간으로 대체 |
| A5 위치 경로 | 열린 폴더 경로 | 텍스트 `STRING_DIRECTORY`(`select.rs`, `directory_path()`) | 없음(Lua 가 `>` 로 나눔) | 바로 가능 |
| A6 카테고리 탭 | 폴더 트리와 탭 2개(`SelectTab::Songs/Courses`, `course_ui.rs:50`). 카테고리 아이콘 구조는 없다 | - | 대응 없음 | 생략 권고(폴더 막대가 같은 일을 한다) |
| A8 정렬 라벨 | 정렬 이름 | 텍스트 `STRING_SORT`(`select.rs`) | 없음 | 있음(SORT 버튼에 이미 표시) |
| B1 등급 배지 | 막대별 최고 EX·최대 EX(`BarScore`, `lib.rs:497`), 등급 계산 `dj_rank`(`result.rs:133`) | 막대별 값은 스킨 API 에 없다 | `songlist` 확장 키 `rank`(4.2절) | 엔진 확장 필요 |
| B2 램프 띠 | 막대별 램프 | `songlist.lamp`(`list.lua:15`) | 없음 | 있음 |
| B3 레벨 칸 | 막대별 레벨·난이도 | `songlist.level`(`list.lua:17`) | 없음 | 있음(칸·테두리만 새로 그림) |
| B5 자켓 썸네일 | 막대별 그림 경로(`BarChart.images`), 디코드는 커서 곡 3장뿐(`images.rs:29` `SLOTS = 3`) | 없음 | `songlist` 확장 키 `thumb`(4.2절) | 엔진 확장 필요 |
| B6 선택 강조 | `liston`·선택 틀(`list.lua:61`) | `songlist.liston` | 없음 | Lua 만(크기·돌출·테두리) |
| B7 자켓 확대 | 커서 곡의 스테이지 그림 | 참조 이미지 `-100`(`info.lua:stage`) | 없음 | Lua 만 |
| B8 CLEAR VISUALIZATION | 열린 폴더의 모든 막대 램프. 지금 스킨이 읽는 것은 커서가 폴더 막대에 있을 때의 그 폴더 분포뿐(`select.rs` `folder_lamps`, `NUMBER_FOLDER_*`) | 부분 | 새 숫자·비율 id 12개(4.6절) | 엔진 값 필요 |
| B9 위치 | 목록 위치 비율 | `RATE_MUSICSELECT_POSITION`(`select.rs` rate) | 없음 | 있음(`list.lua:47`) |
| C1 큰 배경 그림 | 스테이지 그림 | 참조 이미지 `-100` | 없음 | 가능하나 디자인 결정 필요(질문 11) |
| C3~C5 제목·장르·아티스트·BPM | 있음 | `TITLE`, `GENRE`, `FULLARTIST`, `MINBPM/MAXBPM`(`info.lua:20-26`, `bpm_range`) | 없음 | 있음(BPM 만 글자 키움) |
| C6 NOTES RADAR | NOTES=평균 밀도, PEAK=최고 밀도: 있음. SCRATCH, CHARGE, CHORD, SOF-LAN: 계산 없음(곡 모델에는 재료가 있다: 노트 종류·레인, 변속 목록 `overview.rs:275` `speed_changes`) | `NUMBER_DENSITY_AVERAGE/PEAK`(있음), `NUMBER_TOTALNOTE_SCRATCH/BSS/LN`(350~353, 선곡에서 빈칸) | 새 값 6개(4.3절) | 엔진 값 필요 |
| D1 TOTAL NOTES | 있음 | `NUMBER_TOTALNOTES`(74, `info.lua:fact_number`) | 없음 | 있음 |
| D2 DIFF 탭 | 같은 곡의 다른 난이도 차트: 라이브러리에는 있으나 묶는 계산이 없다 | 현재 곡의 난이도 옵션 `OPTION_DIFFICULTY1..5`만 | 새 값 5개 + 이동 이벤트(4.6절) | 엔진 값 필요 |
| D3 밀도 그래프 | `ChartDetail.density`(`rbms-library/src/lib.rs:47`)는 내장 화면만 쓴다 | `judgegraph` type 0 은 선곡에서 값이 없어 그려지지 않는다(`FrameData` 에 `notes` 없음) | id 없음, 객체 연결 | 엔진 연결 필요(원본 호환) |
| D4~D5 레인별 노트 수 | 곡 모델에 레인별 노트가 있다. 저장·노출은 없다 | 없음 | 새 값 16개(4.6절) | 엔진 값 필요 |
| D6 SCORE CLASS | 필요 EX = 총 노트 x 2 x k/9 로 계산 가능, 달성 여부는 내 EX 와 비교 | `NUMBER_TOTALNOTES`, `NUMBER_SCORE`, 등급 옵션 200~207 | 없음 | Lua 계산(질문 6) |
| D7 CLEAR RATE | 내 클리어율 = CLEAR COUNT / PLAY COUNT | `NUMBER_CLEARCOUNT`·`PLAYCOUNT`(`select.rs:225`) | 없음 | Lua 계산. 전체 플레이어 비율은 서버 통계가 없다(`docs/reference/ir-api.md` 에 곡 통계 경로 없음). IR 줄 기준의 `NUMBER_IR_PLAYER_TOTAL_CLEAR_RATE` 는 상위 10줄(`ir_ranking.rs:17`)만 센다 |
| E1~E2 SCORE, MISS COUNT | 있음 | `NUMBER_SCORE`(71), `NUMBER_MISSCOUNT`(`select.rs:223`) | 없음 | 있음 |
| E3 큰 등급 | 있음 | 등급 옵션 200~207(`info.lua:rank`) | 없음 | 있음 |
| E4 램프 이름 | 있음(어시스트 등급 포함) | 옵션 100~105·1100~1104, 21110~21114(`info.lua:clear`) | 없음 | 있음 |
| E5 다음 등급까지 | 있음 | `NUMBER_DIFF_NEXTRANK`(154, `select.rs:238`) | 없음(다음 등급 이름은 Lua) | 바로 가능 |
| E6 F~AAA 진행 막대 | 있음 | `RATE_SCORE`(`select.rs:388`) + 1/9 눈금을 Lua 로 그림 | 없음 | 바로 가능 |
| F1~F2 RIVAL 6행 | 로컬 기록 전부(`ScoreRecord`: 날짜, 램프, EX, 최대 EX, 판정, 최대 콤보 `rbms-store/src/score.rs:161`). IR 상위는 패널을 열어야 받는다 | IR 슬롯 120~129·380~399 는 부재로 답한다 | 내 기록 6줄: 새 값 24개 + 텍스트 6개. IR 상위: 원본 id 연결(새 id 없음) | 4.5절 |
| G1 하단 안내 | 정적 글자 | - | 없음 | 있음(`controls.lua:HINT_ROWS`) |

현재 화면에 있는 값 중 IIDX 구성에 칸이 없는 것(MAX COMBO, PLAY COUNT, CLEAR COUNT, RATE, 판정 5행, LENGTH, JUDGE, TOTAL, KEYS)은 MUSIC DATA 머리줄과 SCORE DATA 오른쪽으로 옮겨 모두 남긴다. 폴더·코스 정보(`info.lua:80-98`)는 조건(`ON_FOLDER`, `ON_COURSE`)이 달라 새 곡 패널과 겹치지 않는다.

### 4.2 막대 확장(등급 배지와 자켓 썸네일)

설계: `songlist` 객체에 R-BMS 확장 키 두 개를 더한다. beatoraja 는 모르는 키를 버리므로(`docs/skin.md` 1.4절 `rbms_extensions` 설명과 같은 사정) 겸용 스킨에 넣어도 안전하고 외부 스킨은 영향이 없다.

| 키 | 모양 | 그리는 것 |
| --- | --- | --- |
| `rank` | `lamp` 와 같은 목록(destination 8개, 등급 F~AAA 순서) | 막대의 최고 기록 등급 한 장. 기록 없는 막대는 아무것도 그리지 않는다 |
| `thumb` | destination 하나 | 막대의 그림 한 장(스테이지 그림, 없으면 배너). 막대 기준 상대 좌표 |

바뀌는 곳:

1. `crates/rbms-skin/src/model/objects.rs:597`(`SongList` 에 필드 `rank: Vec<Destination>`, `thumb: Option<Destination>`)와 로더(`loader/from_lua.rs`, JSON 경로는 같은 serde 구조라 필드만 더하면 읽힌다. `#[serde(default)]` 이미 적용).
2. `crates/rbms-render/src/skin_render/songlist/build.rs`, `songlist.rs`(`SongListBody` 에 `rank`, `thumb`), `bars.rs`(`SongBar` 에 `rank: Option<u8>`, `thumb: Option<TextureId>`), 그리기 순서는 "램프 다음, 레벨 앞"(원본 패스 순서 `BarRenderer.java` 의 램프·레벨 사이)에 한 패스씩.
3. 앱: `scene.rs:86` `wheel_bar` 가 `rank` 를 `bar.chart.best` 로 계산, 새 파일 `stage/select/thumbs.rs`(썸네일 작업자와 캐시), `stage/select/mod.rs:1442`(`SongBars` 에 썸네일 표를 실어 보냄).

등급 계산의 정합: 정보창의 RANK 는 커서 막대의 최고 기록(`focused_records.best()`)에서 오고, 막대의 `best_score`(`list.rs:698`)는 `for_md5` 로 모든 판(보조 판 포함, LN 모드 무관)에서 최고 EX 를 고른다. 램프는 보조 판·낡은 규칙 판을 따로 거른다(`rbms-store/src/score.rs:392-404`). 배지가 정보창과 어긋나지 않도록 `best_ex_for_md5` 계열(보조 판 제외, `score.rs:350`)로 맞춰야 한다. 확인 필요(6절).

### 4.3 곡 분석 값(레이더, 밀도 그래프, 레인별 노트 수, 종류별 노트 수)

설계: 커서가 150ms 쉬면 곡을 한 번 파싱해 `ChartOverview`(`skin_host/overview.rs:44`, 종류별 노트 수 `note_counts`, 초당 종류별 분포 `kinds`, 변속 목록 `speeds`, 주 BPM `main_bpm`를 이미 가진다)를 만든다. 이것을 작은 캐시(곡 해시 키, 최근 몇 곡)에 두고 다음에 쓴다.

1. `ChartMeta.note_counts`(`chart.rs:137`)와 `main_bpm` 을 채운다. 종류별 노트 수 350~353 과 주 BPM 92 가 선곡에서도 답한다(`docs/skin.md` 6절의 D9 "songdb 열 추가와 재스캔" 없이 해결).
2. `FrameData.notes`(`NoteDistribution{kinds}`)와 `FrameData.bpm`(`BpmTimeline`)을 선곡 프레임에 실어 원본의 `judgegraph` type 0·`bpmgraph` 가 선곡에서 그려지게 한다(`overview.rs:157` `series()` 가 같은 모양을 이미 만든다. 플레이·결정 화면이 쓴다). 기본 스킨의 밀도 그래프와 ModernChic 의 `bmsanalysis` 가 같은 경로를 쓴다.
3. 새 값: 레인별 노트 수(레인마다 한 개), 레이더 6축. 레이더는 R-BMS 자체 정의를 쓴다(IIDX 의 정규화는 확인 못 함): NOTES=평균 밀도, PEAK=최고 밀도, SCRATCH=스크래치 레인 평균 밀도, CHARGE=롱노트(CN·HCN·BSS) 수, CHORD=동시치기(같은 시각에 2개 이상) 횟수, SOF-LAN=변속 횟수와 폭. 각 축은 이름 붙은 상한 상수로 나눠 0~100 으로 만든다(매직넘버 금지 규칙).
4. 파싱 위치: 지금의 `refresh_focused_detail` 은 프레임 스레드에서 파싱한다(`mod.rs:649-660`). 큰 곡에서 프레임이 튈 수 있으므로 새 분석은 그림 작업자(`images.rs`)와 같은 방식의 작업자 스레드에 두는 안을 권한다. 곡 파일이 두 번 파싱되는(기존 `compute_chart_detail` 과 `ChartOverview`) 중복은 한쪽으로 합칠지 구현 단위에서 정한다. 비용은 추정(미측정)이며 `docs/reference/frame-cost.md` 의 방식으로 재야 한다.
5. DB: `song_detail` 표는 건드리지 않는다(캐시 안 함, 마이그레이션 없음). 느리면 후속으로 `song_detail` 에 열을 더하고 `SCHEMA_VERSION` 을 올린다(`rbms-library` 의 `songdb/schema`).

### 4.4 레이아웃 제안(1920x1080, 구체 좌표는 구현 때 캡처로 확정)

| 영역 | 제안 | 비고 |
| --- | --- | --- |
| 머리 띠 | MUSIC SELECT(64), PLAY STYLE 칩, FREE PLAY/COURSE 칩, 검색 상자를 x 640..976 으로 줄임, 시각 칩 x 992..1096, 필터 3개는 그대로 | A1~A4, A8 |
| 곡 머리 | 현재 유지(136..336)에 BPM 큰 글자를 아티스트 줄 오른쪽에 추가 | C3~C5 |
| 자켓 | 256x256 정사각을 (64,352)에 둔다(지금 424x318 상자를 줄임) | B7·C1. 큰 배경 그림은 질문 11 |
| MUSIC DATA | (352,352) 폭 744 높이 256. 위 줄: 난이도 탭 5칸(레벨, 선택 칸 강조), TOTAL NOTES, LEVEL, BPM. 가운데: 밀도 그래프 + BPM 띠. 오른쪽: 레인별 노트 막대 | D1~D5. 지금의 LENGTH·JUDGE·TOTAL·KEYS 는 머리줄 작은 글자로 |
| 레이더 | (64,640) 256x256 | C6 |
| SCORE DATA | (352,640) 폭 400. SCORE, MISS COUNT, 큰 등급, 램프 이름, 다음 등급까지, F~AAA 진행 막대, 그 아래 MAX COMBO·PLAY·CLEAR COUNT·RATE | E1~E5. 판정 5행은 오른쪽 열로 |
| 기록 6줄 | (784,640) 폭 312 | F1~F2 의 대체(4.5절). SCORE CLASS 와 CLEAR RATE 는 이 패널 아래 한 줄 |
| 곡 목록 | 막대 728x64 유지. 안쪽: 왼쪽 등급 배지 56 폭(0..56), 램프 띠(56..64), 레벨 칸(64..128), 제목(144..), 오른쪽 자켓 64x64(664..728), 라벨은 자켓 왼쪽으로 이동 | B1~B5 |
| 선택 막대 | 선택 틀을 왼쪽으로 돌출(x 1096 에서 더 왼쪽)·높이 키움, 배지도 틀 위에 | B6. `liston` 은 `listoff` 와 크기가 달라도 되는 객체다(원본 계약) |
| 오른쪽 가장자리 | 스크롤 막대의 트랙(x 1872, 12x832)을 램프 11종의 누적 막대로 칠하고 손잡이가 위치를 표시. 세로 글자 표지는 생략 | B8·B9. 값은 열린 폴더 전체의 램프 분포(4.6절) |
| 하단 | 안내 2줄을 키 이름 상자 모양으로(순수 글자·사각형) | G1 |

### 4.5 RIVAL 자리에 넣을 수치 후보와 추천

| 후보 | 데이터 | 네트워크 | 모든 곡에서 채워지나 | 새 값 | 비고 |
| --- | --- | --- | --- | --- | --- |
| A 내 기록 6줄(BEST 1줄 + 최근 5판: 날짜, 램프, EX, 등급, MISS) | 로컬 `ScoreRecord`(`score.rs:161`) | 없음 | 친 곡만 | 값 24개 + 날짜 글자 6개 | 오프라인 가능, 서버 부하 없음 |
| B IR 상위 6명(순위, 이름, 등급, EX, 램프) | `RankingBoard.rows`(`ir_ranking.rs:36`, 한 번에 10줄 요청) | 선곡 커서가 쉴 때마다 요청(지금은 패널이 열렸을 때만, `mod.rs:883`) | IR 설정과 서버 응답이 있을 때만 | 새 id 없음(원본 120~129, 380~399 연결, 클리어는 이미지 인덱스) | 서버 요청 증가. 요구 3 의 연결과 겹친다 |
| C 목표 대비(내 BEST 대 RANK A·AA·AAA·MAX 목표의 필요 EX 와 차이) | `target.rs` 의 `RATE_TARGETS`, `band_target_ex` | 없음 | 친 곡만 | Lua 계산(TOTALNOTES, SCORE) | SCORE CLASS 와 내용이 겹친다 |

추천은 A(오프라인·서버 부하 없음·친 모든 곡). B 는 2차 단위로 두고, 연결 방식은 요구 3 조사와 같은 경로(원본 순위 슬롯)를 쓴다.

### 4.6 확장 id 계획과 남은 범위

현재 사용 중인 확장 번호(`apps/rbms-player/src/skin_host/extension.rs`, `docs/acknowledge/2026-10-22-adr-rbms-extension-ids.md`):

| 공간 | 범위 | 사용 | 남은 번호 |
| --- | --- | --- | --- |
| 이벤트 | 21000~21099 | 21000, 21001 | 21002~21099(98개) |
| 불린(옵션) | 21100~21199 | 21100~21104, 21110~21114, 21120~21124, 21130~21134, 21140~21148 | 21105~21109, 21115~21119, 21125~21129, 21135~21139, 21149~21199(71개) |
| 정수·실수(rate)·문자열 | (아직 없음) | 0 | - |

정수·rate·문자열은 불린·이벤트와 id 공간이 따로다(ADR "옵션과 이벤트는 id 공간이 따로다"). 같은 숫자 범위 21000~21099 를 이 공간들에 써도 번호가 겹치지 않는다. 계획(번호는 제안):

| 공간 | 번호 | 뜻 | 개수 |
| --- | --- | --- | --- |
| 정수 | 21000~21015 | 레인별 노트 수(모드의 레인 순서, 14키까지. 24키는 부재) | 16 |
| 정수 | 21020~21025 | 레이더 6축(0~100) | 6 |
| 정수 | 21030~21034 | 같은 곡 형제 차트의 레벨(난이도 슬롯 1~5, 없으면 부재) | 5 |
| 정수 | 21040~21050, 21051 | 열린 폴더 램프별 개수 11개, 전체 수 | 12 |
| 정수 | 21060~21083 | 내 기록 6줄의 EX, 램프, MISS, 등급 | 24 |
| rate | 21000~21015 | 레인 막대 비율(레인 노트 수 / 최대 레인 노트 수) | 16 |
| rate | 21020~21025 | 레이더 축 비율 | 6 |
| rate | 21040~21050 | 열린 폴더 램프 비율 | 11 |
| 문자열 | 21000~21005 | 내 기록 6줄의 날짜 | 6 |
| 이벤트 | 21010~21014 | 형제 난이도 슬롯 k 로 커서 이동(21002 는 요구 3 조사가 쓴다) | 5 |
| 불린 | 21115~21119 | 형제 난이도 슬롯 k 의 차트가 있다(21105 는 요구 3 조사가 쓴다) | 5 |

합계: 정수 63, rate 33, 문자열 6, 이벤트 5, 불린 5(대역 안). 원본이 선언한 id 는 쓰지 않는다.

열어야 하는 문(원본 선언 확인 게이트): `crates/rbms-skin/src/lua/main_state.rs:857-861`(`number`), `866-870`(`float_number`), `rbms-render` 의 `draw.rs:312`, `text.rs:191`, `songlist/build.rs:168`. 텍스트 읽기 경로(`main_state.rs:874`)와 `graph` 의 `type` 에 rate id 를 쓰는 곳의 게이트 여부는 코드로 끝까지 따라가지 못했다(확인 못 함). ADR 가 불린에서 한 방식(함수 안에서 읽기)을 정수·rate·텍스트에 확장하려면 `reference_implements` 를 건드리지 않고 "확장 id 인가" 판정 함수 하나를 두고 이 호출부를 그 함수로 바꾸는 방식이 가장 작다. 대안: 막대를 여러 칸의 사각형으로 쪼개 `draw` 함수로 그린다(엔진 문을 열지 않지만 destination 이 수백 개 늘고 프레임 함수 호출이 늘어난다. 지금 destination 499개, 함수 55개).

### 4.7 바뀌는 파일 목록, DB·API·설정 변경 여부

| 구분 | 파일 | 새 스킨 id | 설정 필드 | DB 마이그레이션 | 서버 API |
| --- | --- | --- | --- | --- | --- |
| 계약(U0) | `skin_host/extension.rs`, `skin_host/mod.rs`(라우트), `crates/rbms-skin/src/lua/main_state.rs`, `property/names.rs`, `rbms-render` `draw.rs`·`text.rs`·`songlist/build.rs`, `lua/props.lua` | 4.6절 | 없음 | 없음 | 없음 |
| 막대 확장(U1) | `crates/rbms-skin/src/model/objects.rs`, `loader/from_lua.rs`, `crates/rbms-render/src/skin_render/songlist.rs`·`songlist/{bars,build}.rs`·`frame.rs`, `apps/rbms-player/src/stage/select/{scene,mod}.rs`, 새 `thumbs.rs` | 없음(`songlist` 키) | 없음 | 없음 | 없음 |
| 곡 분석(U2) | `skin_host/{overview,chart}.rs`, `stage/select/{list,mod}.rs`, 새 분석 작업자 파일, 새 레이더 계산 파일 | 정수 22개 + rate 22개(레인·레이더) | 없음 | 없음(캐시 안 함) | 없음 |
| 선곡 보조 값(U3) | `stage/select/{scene,list,events}.rs`, `skin_host/select.rs`, `skin_host/select/shown.rs` | 형제 5+5+5, 폴더 램프 12+11, 내 기록 24+6 | 없음 | 없음 | 없음 |
| 기본 스킨(U4) | `assets/skins/rbms-default/lua/select/*.lua`, `lua/theme.lua`, `lua/props.lua`, `tools/sections/select.py`, `image/select.png`, `lua/atlas/select.lua` | 위 id 를 읽음 | 없음 | 없음 | 없음 |
| 검증·문서(U5) | `apps/rbms-cli/scenarios/select.json`, `stage/capture/gate.rs`, `docs/skin.md`·`play-options.md`·`acknowledge/`·`quality-assurance/`, `docs/assets/screenshot-select.jpg` | - | 없음 | 없음 | 없음 |
| 2차(U6, 질문 6a) | 요구 3 조사의 U5(`skin_host/ir.rs`, `skin_host/select/bridge.rs`)가 순위 슬롯을 연결한 뒤 기본 스킨 RIVAL 자리를 그 값으로 바꾸는 `lua/select/*` 몇 군데 | 없음(원본 id 120~129, 380~399) | 없음(자동 받기 설정은 요구 3 의 결정) | 없음 | 없음 |

웹(`web/`)과 운영 DB 는 1차(U0~U5)에서 건드리지 않는다.

### 4.8 자켓 이미지를 막대 썸네일로 쓰는 비용

현재: 그림은 커서 곡의 스테이지 그림·배너·배경 그림 세 장만 디코드하고(`images.rs:29` `SLOTS = 3`, 파일 머리 문서가 "목록 전체의 그림을 디코드한 채로 들고 있는 것은 감당할 수 없다"고 적었다), 텍스처 이름은 슬롯마다 하나다(`TEXTURE_KEYS`). 막대별 경로는 이미 스킨이 읽을 때 채워진다(`list.rs:747`).

설계(추정, 미측정):

1. 작업자 스레드가 보이는 막대 11~15개의 그림을 디코드하고 정사각 128x128(중앙 자르기, 또는 질문 3 의 FIT) 로 줄여 보낸다. 한 장 64 KiB(RGBA), 가장 최근 48장 LRU 면 약 3 MiB 다. BGA 상주 상한 256 MiB(`docs/acknowledge/2026-10-15-adr-bga-resident-texture-cap.md`)와 비교해 무시할 크기다.
2. 텍스처는 이름 붙은 풀(예: 24개)을 쓰고 막대가 풀의 자리를 돌려쓴다. 한 번 줄이면 CPU 원본을 버린다(디코드 원본 보관 금지).
3. 첫 폴더 진입 때 11장 디코드: 한 장 3~15ms 로 가정하면 작업자에서 합계 약 30~170ms(추정, 미측정). 프레임 스레드는 기다리지 않는다. 휠을 빨리 돌리면 지나간 막대의 요청은 `images.rs` 처럼 건너뛴다.
4. 이미지 축소는 `image::imageops`(`apps/rbms-player/Cargo.toml:44` 의 `image 0.25.10` 에 포함. 이 저장소에는 사용처가 아직 없어 컴파일로 확인하지 못했다)나 직접 쓴 박스 필터로 한다. 새 의존성은 필요 없다.
5. 곡 파일이 외부 SSD(`/Volumes/SSD/bms`)에 있어 첫 읽기가 느릴 수 있다. 그림이 없는 곡은 비워 둔다.
6. 측정 계획: `skin-dump --frames 120` 의 프레임 비용 표(`docs/skin.md` 5.1절)와 `docs/reference/frame-cost.md` 의 방식으로 휠을 돌리는 동안의 p99 를 잰다.

### 4.9 테스트 방법

| 변경 | 확인 |
| --- | --- |
| 스킨(Lua) | `cargo run -p rbms-cli -- skin-dump assets/skins/rbms-default --strict --frames 120`: 문서 9개 `load ok`, `warn 0`, `pcall 0`, `fn fail 0`, `over 0`, 팩 `unchanged`(`docs/skin.md` 5.1절). 새 id 는 시나리오(`apps/rbms-cli/scenarios/select.json` 은 지금 불린 2 와 타이머 1 만 있다)에 값을 더해 읽히게 한다 |
| 시트 | `uv run assets/skins/rbms-default/tools/generate-images.py --check`(시트·`lua/atlas/select.lua` 를 대조하는 도구, 등급 배지 8종과 필요하면 새 조각 추가) |
| 화면 캡처 | `RBMS_SKIN_PACK=$PWD/assets/skins/rbms-default RBMS_SKIN_CAPTURE_DIR=<폴더> cargo test -p rbms-player --lib --locked the_browser_of_a_skin_pack_named_by_the_environment_is_captured_over_a_scanned_library`(`stage/capture.rs:1796`)와 첫 실행 캡처 `the_browser_of_a_first_run_is_captured_over_a_scanned_library`(같은 파일). `docs/HANDOFF.md` 7절의 README 캡처 명령과 같은 계열 |
| 게이트 | `stage/capture/gate.rs`(선곡을 640x360 으로 그려 객체 종류·경고 0 을 단언): 새 객체가 들어가면 기대 목록을 고친다 |
| 엔진 | `songlist` 의 배지·썸네일 그리기 단위 테스트(`songlist/tests.rs` 에 이어서), 분석 값(`skin_host/chart/tests.rs`, `skin_host/select/tests.rs`), 레이더·레인 계산 단위 테스트, 썸네일 캐시의 LRU·건너뛰기 테스트(`stage/select/lent_tests.rs` 방식) |
| 외부 스킨 회귀 | ModernChic 으로 선곡 캡처(`RBMS_SKIN_PACK`)를 전후로 비교. judgegraph·bpmgraph 가 새로 그려지므로 달라지는 것이 정상이고 나머지는 같아야 한다 |
| 전체 | 변경 위험을 직접 덮는 최소 검증만(`docs/HANDOFF.md` 7절 게이트). 이 조사에서는 하나도 돌리지 않았다 |

### 4.10 대안

1. 막대 확장 없이 왼쪽 정보창만 IIDX 식으로 바꾼다: 공수가 가장 작고(U1 불필요) 요구 1 의 "왼쪽 스코어"가 빠진다. 비추천.
2. 등급 배지를 `trophy` 에 얹어(코스 막대 전용 코드 `trophy_index`를 곡에도 적용) 확장 키를 만들지 않는다: 원본 `trophy` 의미(코스 메달)를 바꾸므로 외부 스킨에서 곡 막대에 트로피가 그려진다. 비추천.
3. 내장 선곡 렌더러(1280x720)도 같이 바꾼다: 폴백이고 골든 테스트가 있다(`docs/theme.md`). 비추천(질문 10).
4. 레이더·레인 막대를 새 객체 종류로 엔진에 둔다: 스킨이 값만 읽는 지금 구조와 어긋나고 외부 스킨에도 노출된다. 정수·rate id 안이 더 작다.

## 5. 사용자 결정이 필요한 질문

1. 색 방침: **A. 구조는 IIDX 식, 색은 모노톤 유지하되 등급 배지·SCORE CLASS 메달만 금·은·동(코스 트로피 `TROPHY_COLORS` 선례, `list.lua:19`)(추천)** / B. 등급 배지도 밝기 위계만(흰·밝은 회색·중간 회색) / C. IIDX 식 전면 채색(2026-10-12 모노톤 결정을 되돌림). 추천 이유: 모노톤 결정을 지키면서 등급은 뜻이 색에 실린 정보라 선례가 있다.
2. 선곡 레이아웃: **A. 지금 레이아웃을 IIDX 식으로 교체(추천)** / B. 옵션(헤더 속성)으로 현재 것과 IIDX 식 둘 다 유지. 추천 이유: 둘 다 유지하면 Lua·시트·캡처가 두 배다.
3. 막대 확장: **A. `songlist` 확장 키 `rank`·`thumb` 추가(추천, 외부 스킨 영향 없음)** / B. 막대는 두고 정보창만 변경. 자켓 모양은 **중앙 자르기 정사각(추천)** / 4:3 FIT. 추천 이유: 요구 1 의 핵심이고 beatoraja 는 모르는 키를 버린다.
4. 확장 id: **A. 정수·rate·문자열도 21000~21099 숫자 범위를 쓴다(공간이 달라 충돌 없음, 추천)** / B. 21200 이상의 별도 대역. 추천 이유: "21000~21199 안" 규칙을 지킨다.
5. RIVAL 자리: **A. 내 기록 6줄(BEST + 최근 5판)(추천)** / B. IR 상위 6명 / C. 목표 대비 6줄. 추천 이유: 서버 요청 없이 오프라인·모든 친 곡에서 채워진다.
6. (6a) IR 상위 6명을 RIVAL 자리에 넣는 2차 작업: **A. 이번에는 하지 않고 요구 3 조사(`docs/reference/2026-10-24-select-ranking-survey.md`, U5 가 순위 슬롯 연결과 자동 받기를 맡는다)가 끝난 뒤 결정한다(추천)** / B. 이번 범위에 넣는다. 추천 이유: 순위를 커서가 쉴 때마다 받으면 서버 요청이 늘고 같은 코드(`skin_host/ir.rs`, `stage/select/mod.rs:883`)를 요구 3 이 이미 소유한다. (6b) SCORE CLASS: **A. 등급 경계 필요 점수(AAA·AA·A = 최대 EX 의 8/9, 7/9, 6/9)를 금·은·동으로, 달성 표시(추천)** / B. 목표 점수 설정 기준 / C. 생략. 추천 이유: 새 데이터 없이 정확하고 IIDX 의 "메달에 필요한 점수"와 뜻이 같다(`https://remywiki.com/AC_ZINRAI`, 정확한 산출 방식은 확인 못 함).
7. CLEAR RATE: **A. 내 클리어율 = CLEAR COUNT / PLAY COUNT(추천)** / B. IR 전체 클리어율(서버에 곡 통계 경로 추가 필요) / C. 생략. 추천 이유: 서버 변경 없이 값이 정확하다.
8. NOTES RADAR: **A. 구현, 6축 이름과 측정 의미는 IIDX 와 같고 정규화 상한은 R-BMS 자체 상수(추천)** / B. 후순위로 미룸. 추천 이유: 사용자가 이미지 8번으로 따로 지정했고 값은 곡 모델로 계산된다.
9. 문구와 상표: **A. 일반어로 쓴다(CHART RADAR, TARGET SCORES, CLEAR RATE 등)(추천)** / B. IIDX 문구(SCORE CLASS, NOTES RADAR, CLEAR VISUALIZATION) 그대로. 추천 이유: 공개 저장소이고 다른 사이트의 흔적을 싫어한다는 방침(`docs/HANDOFF.md` 5절)이 있다. 엠블럼·마스코트·로고는 쓰지 않는다.
10. 소항목 일괄: (a) 내장 선곡 렌더러는 그대로 둔다(추천) / 같이 맞춘다, (b) TIME REMAIN 자리는 현재 시각 칩(추천) / 세션 시간 / 생략, (c) 카테고리 아이콘 탭은 생략하고 위치 경로만(추천) / 아이콘 탭 신설.
11. 큰 곡 이미지: **A. 지금처럼 상자(256 정사각)로 두고 선택 막대 옆 확대는 하지 않음(추천)** / B. 왼쪽 전체 배경에 어둡게 깔기(IIDX 식, "배경 이미지 금지" 방침을 곡 그림에 한해 풀어야 함) / C. 선택 막대 옆 확대 팝업 추가. 추천 이유: 방침을 어기지 않고 공수가 가장 작다.

## 6. 위험과 확인 못 한 것

1. IIDX 쪽 수치 정의는 외부 문서 요약에서만 읽었다. 확인 못 함: NOTES RADAR 의 정규화, SCORE CLASS 의 필요 점수 산출 방식, CLEAR VISUALIZATION·RIVAL SCORE DATA·MUSIC DATA·DIFF 탭·TIME REMAIN 의 공식 설명, 화면 속 숫자(`1119/1086/1040`)가 실제로 무엇인지. R-BMS 안은 화면 모양과 R-BMS 데이터에 맞춘 해석이다.
2. 배지 등급과 정보창 RANK 의 어긋남(보조 판, LN 모드)은 코드를 읽고 쓴 가설이다. 구현 단위에서 `best_ex_for_md5` 와 `best_score` 의 차이를 테스트로 확인해야 한다.
3. 형제 난이도(DIFF 탭)를 묶는 기준은 "같은 폴더·같은 제목·같은 모드"로 제안하지만 이 저장소의 라이브러리에는 곡 단위 묶음이 없고(`select_tree.rs` 의 폴더는 곡 폴더가 아니라 논리 폴더), 한 폴더에 여러 곡이 든 팩에서 어긋날 수 있다. 확인 못 함.
4. 곡 분석 파싱과 썸네일 디코드의 프레임 비용은 추정이고 측정하지 않았다(프로젝트 규칙상 빌드·실행 금지). 큰 곡 파싱이 프레임 스레드에서 도는 현 구조(`mod.rs:649-660`)를 그대로 두면 새 분석이 같은 곳에서 튈 수 있다.
5. 외부 스킨 영향: judgegraph·bpmgraph·종류별 노트 수가 선곡에서 값을 받으면 ModernChic 의 분석 칸이 새로 그려진다(정상 동작이지만 화면이 달라진다). 프레임 비용 증가는 측정하지 못했다.
6. 한글·일본어 곡명의 폭·잘림은 캡처 하니스가 번들 Inter 하나뿐이라 확인할 수 없다(`docs/skin.md` 6절 "확인" 행). IIDX 식 큰 제목에서 특히 중요하다.
7. `imageops` 사용 가능 여부는 컴파일 전이라 확인 못 함(4.8절).
8. 문서와 코드 어긋남 2건은 2.6절. 이 보고서는 고치지 않았다.
9. 내장 선곡 렌더러와 기본 스킨의 값이 한동안 달라진다(질문 10a 를 추천대로 하면). 폴백은 스킨이 없을 때만 보인다.
10. 요구 3(IR 순위)과의 겹침: `docs/reference/2026-10-24-select-ranking-survey.md` 는 확장 id 이벤트 21002 와 불린 21105 를 쓰고 `skin_host/ir.rs`·`stage/select/mod.rs`·기본 스킨 `lua/select/*` 를 소유한다. 이 보고서는 그 번호를 피해 21010~21014, 21115~21119 로 잡았다. 질문 5·6a 의 B 안은 그 단위(U5)의 결과를 쓰므로 두 작업의 병합 순서와 `stage/select/mod.rs`·`lua/select/*` 소유권을 메인이 정해야 한다. 요구 3 의 U4(그리기)는 "요구 5 의 컴팩트 패널과 같은 파일"이라고 적었다(`ir_ranking_view.rs`, `overlay.rs`).

## 7. 구현 단위 분해

규모는 테스트 포함 줄 수의 추정이다(미측정). 모든 단위는 worktree 병렬이 가능하고 메인이 병합·게이트·커밋한다. 에이전트는 커밋·푸시하지 않는다.

| 단위 | 내용 | 파일 소유 | 의존 | 규모(추정) |
| --- | --- | --- | --- | --- |
| U0 계약 | 확장 id 상수, 게이트 판정 함수, 라우트, `props.lua` 이름표, id 표 문서 | `skin_host/{extension,mod}.rs`, `crates/rbms-skin/src/{lua/main_state,property/names}.rs`, `rbms-render` `draw.rs`·`text.rs`·`songlist/build.rs`, `lua/props.lua` | 없음. 먼저 직렬 | 약 250줄 |
| U1 막대 확장 | `rank`·`thumb` 키, 그리기 패스, 등급 계산, 썸네일 작업자·캐시 | `crates/rbms-skin/src/model/objects.rs`, `loader/from_lua.rs`, `rbms-render` `songlist*`·`frame.rs`, `stage/select/{scene,images 이웃}.rs`, 새 `thumbs.rs` | U0 | 약 1,100줄 |
| U2 곡 분석 | 분석 작업자·캐시, `note_counts`·`main_bpm`, judgegraph·bpmgraph 연결, 레인별 노트 수, 레이더 계산 | `skin_host/{overview,chart}.rs`, `stage/select/list.rs`, 새 분석·레이더 파일 | U0 | 약 1,300줄 |
| U3 선곡 보조 값 | 형제 난이도 값·이동 이벤트, 열린 폴더 램프 분포, 내 기록 6줄 | `stage/select/{scene,events}.rs`, `skin_host/select.rs`, `skin_host/select/shown.rs` | U0 | 약 600줄 |
| U4 기본 스킨 | IIDX 식 레이아웃, 배지·레이더·레인·기록 패널, 배지 조각, 시트·atlas 재생성 | `assets/skins/rbms-default/lua/select/*`, `lua/theme.lua`, `tools/sections/select.py`, `image/select.png`, `lua/atlas/select.lua` | U0 확정 뒤 시작(스텁 id), 시각 확인은 U1~U3 병합 뒤 | 약 1,200줄 Lua + 시트 |
| U5 검증·문서 | 시나리오, 게이트 기대, 캡처, ADR, 사용자 문서, 실기 점검표 | `apps/rbms-cli/scenarios/select.json`, `stage/capture/gate.rs`, `docs/**`, `docs/assets/screenshot-select.jpg` | U1~U4 | 약 400줄 문서 |
| U6 2차(질문 6a B) | RIVAL 자리를 IR 상위 6명으로 바꾸는 기본 스킨 변경(새 엔진 값 없음). 순위 슬롯 연결과 자동 받기는 요구 3 의 U5 | `assets/skins/rbms-default/lua/select/*` | 요구 3 의 U5 | 약 150줄 Lua |

순서: U0 → (U1, U2, U3 병렬; U4 는 U0 직후 스텁으로 병렬 시작) → U4 시각 맞춤 → U5. `stage/select/mod.rs` 는 U1·U2·U3 가 각각 한 군데(프레임 구성 `mod.rs:1442` 와 필드 추가)를 고치므로 병합 때 충돌한다. 각 단위는 자기 파일에 로직을 두고 `mod.rs` 에는 필드 하나와 호출 한 줄만 더하게 하고, 병합은 `git apply`(`--3way` 금지)로 한 단위씩 합친다(`docs/HANDOFF.md` 5절).
