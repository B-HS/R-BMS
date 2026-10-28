# 선곡 정보창 정돈과 밀도(NPS) 표시 — 구현 전 조사

> 최종 갱신 2026-10-29 · 대응 커밋 c1dc988
> 범위: (A) 기본 스킨 선곡 정보창이 "중구난방이고 text overflow 가 보여 보기 힘들다"는 지적의 현황, (B) 선곡 화면에 밀도(notes per second) 표시 추가.
> 읽기 전용 조사다. 코드는 바꾸지 않았다. 캡처 한 벌을 실제로 찍었다(3.3절).

---

## 1. 현재 구조

### 1.1 선곡 문서의 조립 순서

- `assets/skins/rbms-default/lua/select/main.lua:38-58`: `list.build` -> `bands` -> `controls.header` -> `info.build` -> `controls.footer` -> `side.build` -> `sheet.build`.
- 정보창은 `info.lua:681-691` 의 `info.build` 가 곡 머리(`head`), 곡 그림(`stage`), CHART DATA(`chart_data`), MY SCORE(`my_score`), `radar.build`, `records.build`, SCORE TARGETS·CLEAR RATE(`targets`), 폴더(`folder`), 코스(`course`)를 차례로 그린다. 곡 막대 위에서만 보이는 것은 `op = { ON_CHART }`(옵션 `SONGBAR`, `info.lua:14`)로 가린다.
- 좌표 상수의 단일 출처는 `lua/theme.lua:355-377` 의 `theme.select_layout` 이고, 레이더만 `theme.lua:319-347`(`RADAR_AREA`, `theme.radar`)에 있다. 화면은 1920x1080(`theme.lua:7-8`).
- 글자 크기 토큰 `theme.type`(`theme.lua:264-271`): display 96, title 64, heading 44, body 32, label 24, caption 18. 대문자 높이 비율 0.727(`theme.lua:273`).
- 숫자 스프라이트 칸 폭(`lua/atlas/common.lua:8-12`, `w / divx`): num18 12, num24 15, num32 19, num44 28, num64 40 px.

### 1.2 글자 상자의 의미(좌표 해석)

- `Builder:text`(`lua/ui.lua:299-301`) -> `text_box`(`ui.lua:288-297`): `top` 은 **대문자 윗선**이고 상자 높이 `h` 는 지정이 없으면 글자 크기와 같다. 정렬이 center/right 이면 엔진 규약대로 `x` 를 가운데/오른쪽 끝으로 옮긴다.
- `theme.text_top(baseline, size) = baseline - cap_height(size)`(`theme.lua:393-395`), `theme.centered_top(box, size)`(`theme.lua:397-399`).
- 엔진은 대문자 높이에서 줄을 건다(`crates/rbms-render/src/font/block.rs:466-468`, `baseline = (design.cap + index*line)*scale`). 그래서 내림자(g, y)와 CJK 글자(대문자보다 큼)는 `top` 위·아래로 상자 밖에 그려진다. CJK 는 대략 크기의 0.15 배만큼 `top` 위로 올라간다(추정. Inter 에 CJK 가 없어 시스템 대체 글꼴이 그린다, `docs/reference/font-rendering.md:131`).
- `Builder:number_box`(`ui.lua:346-370`): `digit` 칸 수 전체 폭(`span`)으로 정렬 위치를 정한다. 숫자 정렬값은 RIGHT 0, LEFT 1, CENTER 2(`props.lua:471-475`). `fit` 을 주면 그 폭에 맞게 줄인다(레인 숫자만 쓴다).

### 1.3 패널별 좌표·크기·글꼴·정렬

전부 1920x1080 설계 좌표 `(x, top, w, h)`. "넘침 처리"는 텍스트 객체의 `overflow` 값(1.4절)이다.

**곡 머리** (`info.lua:22-28, 138-153`. 영역 x 64~1096, top 136~336)

| 요소 | x | top | w | 크기 | 정렬 | 넘침 처리 |
| --- | --- | --- | --- | --- | --- | --- |
| 난이도 색 띠 | 64 | 136 | 8 | h 200 | - | - |
| GENRE(텍스트 13) | 104 | 154 | 992 | label 24, sub 색 | 왼쪽 | SHRINK |
| TITLE(텍스트 10) | 104 | 188 | 992 | title 64 | 왼쪽 | SHRINK |
| SUBTITLE(11) | 104 | 262 | 992 | label 24, sub 색 | 왼쪽 | SHRINK |
| FULLARTIST(16) | 104 | 294 | 992 | body 32 | 왼쪽 | SHRINK |
| 코스 제목 대신 "COURSE" | 104 | 188 | 992 | title 64 | 왼쪽 | 없음(OVERFLOW) |

**곡 그림**: (64, 352, 256, 256)(`theme.lua:363`, `info.lua:155-166`). 그림이 없으면 caption 18 가운데 "NO STAGE IMAGE".

**CHART DATA** (`theme.lua:364` 영역 (352, 352, 744, 256), `info.lua:53-74, 205-349`)

| 요소 | x | top / baseline | w | 크기 | 정렬 | 넘침 |
| --- | --- | --- | --- | --- | --- | --- |
| 구분선 + 제목 "CHART DATA" | 352 | 352 / 글자 top 364 | 744 | caption 18 | 왼쪽 | 없음 |
| 난이도 탭 5칸 | 760 + 68s | 360 | 64 x h32 | num24 3자리 | 가운데 | - |
| NOTES 라벨 / 값 | 352 | 400 / baseline 448 | 120 | caption 18 / num32 5자리 | 왼쪽 | 라벨 SHRINK |
| LEVEL 라벨 / 값 | 488 | 400 / 448 | 96 | caption / num32 3자리 | 왼쪽 | 라벨 SHRINK |
| BPM 라벨 / 값 | 600 | 400 / 448(글자 top 425) | 152 | caption / body 32 문자열 | 왼쪽 | SHRINK |
| LENGTH | 784 | 400 / baseline 440 | 64 | caption / label 24 `m:ss` | 왼쪽 | SHRINK |
| TOTAL | 856 | 400 / 440 | 56 | caption / num24 4자리 | 왼쪽 | 라벨 SHRINK |
| KEYS | 920 | 400 / 440 | 72 | caption / label 24 ("7 KEYS") | 왼쪽 | SHRINK |
| JUDGE | 1000 | 400 / 440 | 88 | caption / label 24 ("VERY HARD") | 왼쪽 | SHRINK |
| 밀도 그래프(`judgegraph` type 0 + `bpmgraph`) | 352 | 464 | 400 x h136 | - | - | - |
| 레인 막대 8칸(피치 39) | 784 | 464 | 312 x h136 | num18 4자리 `fit` 39 | 가운데 | - |

**NOTES RADAR** (`theme.lua:319-347`, `radar.lua:17-27, 109-178`. 영역 (64, 640, 256, 256))

- 중심 (192, 776), 그림 한 변 144, 반지름 69.75. 꼭짓점: 1 위 (192, 706), 2·6 위쪽 옆 (252/132, 741), 3·5 아래쪽 옆 (252/132, 811), 4 아래 (192, 846).
- 축 라벨 caption 18, **OVERFLOW**(`radar.lua:132`), 값 num18 3자리. 위·아래 라벨은 꼭짓점 위/아래 8px 에 라벨(오른쪽 정렬, x 64~188)과 값(왼쪽 정렬, x 196~)으로 나뉜다. 옆 라벨은 폭 128 의 반쪽 상자(오른쪽 축은 192~320 오른쪽 정렬, 왼쪽 축은 64~192 왼쪽 정렬), 위쪽 옆은 꼭짓점 위 41(top 700), 아래쪽 옆은 아래 12(top 823), 값은 라벨 아래 6px.

**MY SCORE** (`theme.lua:365` 영역 (352, 640, 400, 256), `info.lua:76-87, 407-545`)

| 요소 | x | top / baseline | w | 크기 | 정렬 | 넘침 |
| --- | --- | --- | --- | --- | --- | --- |
| 제목 | 352 | 652 | 400 | caption 18 | 왼쪽 | 없음 |
| EX SCORE 라벨 / 값 | 352 | 680 / 728 | 120 | caption / num44 5자리 | 왼쪽 | 라벨 SHRINK |
| MISS COUNT 라벨 / 값 | 480 | 680 / 728 | 120 | caption / num44 5자리 | 왼쪽 | 라벨 SHRINK |
| 등급 배지 | 632 | 680 | 120 x h48 | heading 44 굵게 | 가운데 | 없음 |
| 램프 이름 | 352 | 740 | 280 | label 24 | 왼쪽 | SHRINK |
| 다음 등급 "AA -12" | 632 | 740 | 120 | label 24 | 오른쪽 | SHRINK |
| F~AAA 진행 막대 | 352 | 768 | 400 x h8 | 눈금 라벨 caption | 가운데 | SHRINK |
| MAX COMBO·PLAY COUNT·CLEAR COUNT·RATE | 352 / 456 / 560 / 672 | 808 / 846 | 96 / 96 / 104 / 72 | caption / num24 5자리, RATE 는 label 24 문자열 | 왼쪽 | 라벨·RATE SHRINK |
| 판정 5칸 | 352 + 80i | 858 / 892 | 80 | caption / num18 5자리 | 가운데 | 라벨 SHRINK |

**MY RECORDS / IR RANKING** (`theme.lua:366` 영역 (784, 640, 312, 256), 머리 40, 줄 6 x 36. `records.lua:29-41, 104-192`)

| 요소 | x | w | 크기 | 정렬 | 넘침 |
| --- | --- | --- | --- | --- | --- |
| 제목(top 652) | 784 | 312 | caption 18 | 왼쪽 | 없음 |
| 머리 "EX SCORE" / "MISS"(top 652) | 960 / 1048 | 80 / 48 | caption | 오른쪽 | SHRINK |
| 줄 n: top 680 + 36n + 2, h 32 | | | | | |
| 램프 띠 | 784 | 6 | - | - | - |
| BEST / 날짜(앞 10글자) | 798 | 100 | caption | 왼쪽 | SHRINK |
| DJ LEVEL 배지 | 902 | 52 (h 20) | caption 굵게 | 가운데 | 없음 |
| EX | 960 | 80 | num24 5자리 | 오른쪽 | - |
| MISS | 1048 | 48 | num18 4자리 | 오른쪽 | - |
| (랭킹) 순위 / 이름 / EX | 798 / 844 / 1000 | 36 / 150 / 96 | num24 / label 24 / num24 | 오른쪽 / 왼쪽 / 오른쪽 | 이름 SHRINK |

**SCORE TARGETS / CLEAR RATE** (`theme.lua:367` 영역 (352, 912, 744, 64), `info.lua:89-98, 593-628`)

| 요소 | x | top | w | 크기 | 정렬 | 넘침 |
| --- | --- | --- | --- | --- | --- | --- |
| 제목 SCORE TARGETS | 352 | 920 | 528 | caption | 왼쪽 | 없음 |
| 메달 칩(AAA/AA/A) | 352 / 528 / 704 | 940 | 56 x h28 | caption 굵게 | 가운데 | 없음 |
| 필요 EX | 칩 x + 64 | 칩 가운데 | 48 | num24 5자리 | 오른쪽 | - |
| 차이 `+n` | 칩 x + 116 | 칩 가운데 | 48 | caption | 오른쪽 | SHRINK |
| 제목 CLEAR RATE / 값 | 912 | 920 / 약 945 | 184 | caption / label 24 | 왼쪽 / 오른쪽 | 값 SHRINK |

**폴더·코스 막대** (`info.lua:100-118, 630-679`): 폴더는 차트 수 num64(64, baseline 448), 램프 11행 2열(x 64·580, 행 피치 72, 이름 body 32 **OVERFLOW**). 코스는 스테이지 제목 5줄(x 64, top 400 + 56i, w 1032, body 32 SHRINK), 제약 줄(top 930, label 24 SHRINK).

### 1.4 엔진의 텍스트 넘침 수단

- 스킨 값: `props.text_overflow = { OVERFLOW = 0, SHRINK = 1, TRUNCATE = 2 }`(`props.lua:465-469`). beatoraja `SkinText.OVERFLOW_*` 와 같다. 텍스트 정의의 `overflow` 기본값은 OVERFLOW(`ui.lua:272`), `wrapping` 기본 false(`ui.lua:273`).
- 해석: `crates/rbms-render/src/skin_render/text.rs:77-83, 228-237`. `wrapping` 이 켜지면 `overflow` 와 상관없이 줄바꿈(`BlockFit::Wrap`)이 이긴다.
- 실제 동작(`crates/rbms-render/src/font/block.rs:94-106, 447-463`):
  - SHRINK: 가장 긴 줄이 폭을 넘으면 **가로로만** `width/widest` 배 눌러 맞춘다. **하한이 없다.** 아주 긴 제목은 읽을 수 없을 만큼 납작해진다(세로 크기는 그대로라 비율이 깨진다).
  - TRUNCATE: 폭 안에 드는 글리프까지만 남기고 자른다(`Line::truncate`, `block.rs:217-228`). **말줄임표가 없다**(beatoraja `GlyphLayout.truncate` 에 빈 대체 문자열을 준 것과 같다는 주석).
  - Wrap: cosmic-text `Wrap::WordOrGlyph`(`block.rs:325`)라 공백 없는 CJK 도 글자 단위로 꺾인다. **줄 수 상한이 없어** 아래 요소와 겹칠 수 있다.
  - 비트맵 글꼴 경로는 `crates/rbms-render/src/bitmap_font.rs:660-745` 가 같은 넷을 구현한다(기본 스킨은 TTF 라 해당 없음).
  - 클립(상자 밖 잘라내기), 최소 축소율, 말줄임, 최대 줄 수는 **없다**. Lua 쪽에서는 글자 폭을 잴 API 가 없다(`main_state` 목록, `docs/skin.md:650`).
- 시험: `crates/rbms-render/src/skin_render/text/tests.rs:185-230`(세 모드와 줄바꿈 우선), `crates/rbms-render/src/font/block_tests.rs:115-140`, `skin_render/text/bitmap/tests.rs:309-325`.
- 기본 스킨 사용 현황: 곡 머리 네 줄·BPM·작은 사실 값·램프 이름·다음 등급·RATE·차이·CLEAR RATE·날짜·랭킹 이름·코스 제목은 SHRINK. 패널 제목(`info.lua:171`, `radar.lua:162`, `records.lua:182-183`), 레이더 축 라벨(`radar.lua:132` 명시 OVERFLOW), 폴더 램프 이름(`info.lua:645`), 코스 "COURSE" 머리(`info.lua:152`)는 OVERFLOW. 목록 막대 제목도 SHRINK(`list.lua:29, 125`, 크기 28).

### 1.5 밀도(NPS) 관련 값이 이미 있는 곳

| 출처 | 정의 | 어디서 계산 | 스킨 노출 |
| --- | --- | --- | --- |
| `rbms_chart::note_density`(`crates/rbms-chart/src/lib.rs:430-520`) — beatoraja `SongInformation` 포트 | 1초 칸(bin)마다 노트를 센다. 일반 노트·롱노트 머리 + **롱노트 몸통이 걸친 매 초**(키·스크래치), 지뢰 제외(`lib.rs:465-495`). `peak` = 가장 바쁜 1초 칸. `avg` = `total_notes/bins/4` 이상인 칸만의 평균(조용한 구간은 빠진다). `end` = 게이지 경계(`total_notes*(1-100/TOTAL)`) 뒤 5초 창의 최대 평균. 마지막 노트가 표 한도를 넘으면 셋 다 0 | 스캔(`crates/rbms-library/src/scan.rs:337, 373`)이 곡 DB `DetailRow` 에 저장. 없으면 `compute_chart_detail`(`crates/rbms-library/src/lib.rs:79-120`) | 정수 360(PEAK)·361(소수 2자리)·362(END)·363·364(AVERAGE)·365, Float 360(PEAK)·362(END)·367(AVERAGE)(`apps/rbms-player/src/skin_host/chart.rs:72-80, 225-233, 326-365`, 라우팅 `skin_host/mod.rs:278-284`). 소수 2자리는 반올림이 아니라 버림(`chart.rs:44-46, 329`, 시험 `skin_host/chart/tests.rs:145-173`) |
| `rbms_chart::radar_raw`(`crates/rbms-chart/src/radar.rs:117-136, 319-331`) | 누름 이벤트(일반 노트 + 롱노트 머리, 꼬리·지뢰 아님, `radar.rs:167-169`)만 센다. `notes` = 누름 수 / (첫 누름~마지막 누름 간격, 최소 1초)(`radar.rs:221-227`). `peak` = 누름에서 시작하는 4초 창의 최대 누름 수(**나누지 않은 개수**, `radar.rs:229-241`). LN MODE 와 무관 | 스캔과 분석 작업자(`apps/rbms-player/src/stage/select/analysis.rs:263-268`)가 `RadarRow::measure`. 곡 DB `song_radar` 에 원시값 저장 | 표시값(0~200)만: 정수 21020~21025, Float 21020~21025(`skin_host/extension.rs:167-170, 213-216`). 원시값은 스킨에 없다. **5·7·10·14키만**(`radar.rs:157-165`), 9·24키는 None |
| 내장(스킨 없는) 선곡 | 위 `note_density` 의 셋 | - | "PEAK {0}  AVG {1}  END {2}  /s" 줄과 히스토그램(`crates/rbms-render/src/text/select.rs:42-46`, `crates/rbms-render/src/select.rs:47-55`) |
| 노트 수 `TOTALNOTES`(정수 74) | `count_playable_notes`: 일반 + 롱노트 머리 + CN·HCN 의 꼬리, 지뢰 제외(`crates/rbms-model/src/lib.rs:168-193`) | 스캔·상세 | `chart.rs` 의 `notes`(상세가 있을 때만, `chart.rs:225-229`) |
| 길이 `SONGLENGTH_MINUTE/SECOND`(1163·1164) | `duration_us` = **모델의 마지막 타임라인 시각**(`crates/rbms-library/src/lib.rs:94`, `scan.rs:334`). 마지막 노트가 아니라 뒤따르는 BGM·마디선 등 이벤트까지 포함할 수 있다 | 스캔·상세 | `chart.rs:228` `length_ms` |

- 값이 생기는 시점: `refresh_focused_detail`(`apps/rbms-player/src/stage/select/mod.rs:687-705`)이 커서가 150 ms(`apps/rbms-player/src/lib.rs:195` `FOCUS_DETAIL_DEBOUNCE`) 머문 뒤 곡 DB 캐시를 읽는다(캐시가 없으면 **프레임 스레드에서** `compute_chart_detail` 로 파싱 — play-options.md 17.3절의 "프레임 스레드는 파싱하지 않는다"는 분석 작업자에 대한 말이고, 상세 값은 예외다). 상세는 커서 차트와 같을 때만 스킨에 간다(`mod.rs:1570`). 그래서 360~365·74·1163·1164 는 커서 이동 중, 폴더·코스 막대, 코스 탭에서 부재다.
- 스킨 메뉴 미리보기(`apps/rbms-player/src/stage/select/skin_preview.rs:95-96, 383`)도 가짜 밀도(평균의 2배·1.5배)를 넣는다.
- **기본 스킨은 360~365 를 읽지 않는다.** `props.lua` 의 `props.number`(`props.lua:221-311`)에 DENSITY 이름이 없다. docs/skin.md 6절(`docs/skin.md:760`)은 "밀도(360~365)는 전부터 채워졌다"고 적었고 코드와 맞다.

### 1.6 문구 카탈로그

- 스킨: `lua/i18n.lua`(영역 목록 `i18n.AREAS`, `i18n.lua:12`)와 영역별 파일 하나에 세 언어 열(`{ en, ko, ja }`). 정보창 문구는 `lua/i18n/select_info.lua:1-21`(패널 제목·레이더 축·기록 머리·PLAY COUNT·CLEAR COUNT·CONSTRAINTS), 공용은 `lua/i18n/common.lua:3-23`(NOTES·LEVEL·LENGTH·KEYS·JUDGE·RATE·COURSE·STAGES·CHARTS·CLEAR LAMPS·NONE·NO STAGE IMAGE). 패널 제목과 레이더 축은 세 언어 모두 원어(용어집 결정).
- 하드코딩 문구(카탈로그 밖): KEYS 값 "5 KEYS"~"14 KEYS", JUDGE 값 "VERY HARD"~"VERY EASY"(`lua/parts.lua:11-25`), "BPM"(`info.lua:247`), "TOTAL"(`info.lua:67`), "EX SCORE"·"MISS COUNT"·"MAX COMBO"(`info.lua:419-420, 510`), 판정 이름(`parts.lua:27-31`), 코스 제약 라벨(`info.lua:119-134`).
- 키가 세 언어에 다 있는지, 두 영역에 중복이 없는지는 `crates/rbms-skin/tests/default_pack.rs:16, 539-580` 이 지킨다.
- 앱 내장 선곡의 문구는 `crates/rbms-render/src/text/select.rs`(`translations!`).

---

## 2. 요구사항별 구현 지점과 설계안

### 2.1 (A) 정보창 정돈 — 실제로 찾은 문제

캡처(3.3절, `panels-graded-best-3000ms.png`)와 좌표 계산으로 확인한 것. "확인"은 캡처에서 보인 것, "계산"은 좌표로만 낸 것.

| # | 문제 | 근거 |
| --- | --- | --- |
| P1 | **작은 사실 줄이 붙어 읽힌다**(확인). LENGTH·TOTAL·KEYS·JUDGE 네 칸이 8px 간격이라 값이 "0:04 300 7 KEYS NORMAL" 한 덩어리로 보인다. 라벨 "LENGTH" 는 64px 칸에 눌린다. TOTAL 은 num24 4자리 폭(약 56)이 칸 56 을 꽉 채워 KEYS 와 8px 만 떨어진다 | `info.lua:57-71, 253-275` |
| P2 | **MY SCORE 통계 라벨이 붙는다**(확인). "MAX COMBO PLAY COUNT CLEAR COUNT RATE" 가 8px 간격 SHRINK 로 한 줄처럼 읽힌다. 한국어("플레이 횟수"·"클리어 횟수"), 일본어는 더 눌린다(계산) | `info.lua:85, 489-517`, `select_info.lua:18-19` |
| P3 | **SHRINK 에 하한이 없다**(엔진). 곡 머리 제목은 64px·폭 992 라 라틴 약 28자, CJK 약 15자를 넘으면 가로로 납작해진다. 아티스트(32px)는 라틴 약 56자, CJK 약 31자. 목록 막대 제목도 같은 방식(캡처의 "Autoplay Preview Demo (no #PREVIEW)" 가 눌려 보인다) | `block.rs:462`, `info.lua:148`, `list.lua:125` |
| P4 | **곡 머리의 균형**(확인). 네 줄이 x 104 에 왼쪽 정렬로 쌓이고 오른쪽 약 700px 가 빈다. 부제가 비면 262~294 가 비어 줄 간격이 들쭉날쭉해 보인다. 장르가 제목 위에 sub 색으로 붙는다 | `info.lua:22-28` |
| P5 | **레이더 위쪽 라벨이 몰린다**(확인·계산). NOTES 라벨(top 685~698, x 약 126~188), CHORD 라벨(700~713, x 64~118), 별(698~714)이 2px 안에 붙는다. 아래쪽 SOF-LAN 라벨(854~867)과 CHARGE·SCRATCH 값(842~855)도 0px 간격. 레이더 그림은 지름 약 140 으로 256 영역에 비해 작다 | `radar.lua:17-27, 109-126`, `theme.lua:319-322` |
| P6 | **번역 대비 넘침**(계산). 패널 제목·레이더 축 라벨·폴더 램프 이름·"COURSE" 머리가 OVERFLOW 라, 지금은 원어(짧은 영어)라 괜찮지만 번역을 넣거나 외부 글꼴로 바꾸면 그대로 넘친다 | 1.4절 목록 |
| P7 | **큰 숫자 넘침**(계산). EX SCORE num44 5자리(EX 10000 이상 = 5000노트 이상)는 352~약 485 로 MISS COUNT 칸(480)을 5px 침범한다. SCORE TARGETS 필요 EX num24 5자리(10000 이상 = AAA 기준 5625노트 이상)는 칩 x+40 에서 시작해 칩(x~x+56)을 16px 덮는다 | `info.lua:78-81, 91, 615-616`, `ui.lua:360-363` |
| P8 | 난이도 탭은 지금 차트 칸만 채워져 나머지 네 칸이 빈 상자로 보인다(확인). 같은 폴더·제목·모드에 형제 차트가 없으면 늘 그렇다 | `info.lua:205-224` |
| P9 | KEYS·JUDGE 값과 숫자 라벨 일부가 카탈로그 밖 영어 상수 | 1.6절 |

CJK 제목·아티스트의 실제 모양은 캡처 하니스가 번들 Inter 만 쓰게 해 두부(notdef)로 그려지므로 확인하지 못했다(5절 미확인 1).

### 2.2 (A) 텍스트 넘침 처리 설계안

| 안 | 내용 | 장점 | 단점 |
| --- | --- | --- | --- |
| E1 스킨만 | 크기를 줄이고(제목 64 -> 48~56) 폭을 확보, 칸 사이 간격을 16 이상으로, OVERFLOW 객체에 SHRINK 지정. 문자 수 상한(`utf8.len` 기반 자르기 + "...")을 값 함수로 | Rust 변경 없음. 외부 스킨 무관 | Lua 는 글자 폭을 모르므로 문자 수 상한은 라틴/CJK 폭 차이(약 2배)를 못 맞춘다. 납작해짐(P3)은 그대로 남는다 |
| E2 엔진 확장 넘침 값(추천) | `overflow = 3`(이 플레이어 전용, 예: `FIT`)을 더한다: 가로 축소를 하한(예: 0.75, 이름 있는 상수)까지만 하고, 그래도 넘치면 잘라내고 말줄임표 "…"(U+2026, 글꼴에 없으면 "...")를 붙인다. 구현 지점: `BlockFit` 에 변형 추가(`font/block.rs:94-106`), `compose_block` 의 자르기·축소 분기(`block.rs:454-463`), `TextBody::block_fit`(`skin_render/text.rs:228-237`)와 새 상수, 비트맵 경로(`bitmap_font.rs:660-745`)는 SHRINK 와 같게 두거나 같은 규칙, `props.lua` 의 `text_overflow` 에 이름 추가, 시험은 `skin_render/text/tests.rs` 와 `font/block_tests.rs` 에 | 긴 제목이 읽히는 크기를 지키고 끝이 잘렸음이 보인다. `TextDef.overflow` 는 이미 `i32` 라(`crates/rbms-skin/src/model/objects.rs:197`) 모델·로더 변경이 없다. 외부 스킨은 0~2 만 쓰므로 영향이 없다 | Rust 변경과 시험 추가. docs/skin.md 1.4절(확장 표)에 적어야 한다. beatoraja 에 없는 값이라 외부 스킨이 3 을 쓰면 원본과 다르게 보인다(원본은 3 을 OVERFLOW 로 읽음) |
| E3 줄바꿈 2줄 | 제목에 `wrap = true` 를 쓰고 상자 높이를 2줄로 | 엔진 변경 없음 | 줄 수 상한이 없어 3줄째가 부제·아티스트를 덮는다. 줄바꿈이 SHRINK 를 이긴다(`text.rs:232`) |

추천: **E2 + E1 의 배치 정리**. E2 를 넣지 않는다면 E1 만으로 P1·P2·P4·P5·P7 은 고칠 수 있고 P3 은 남는다.

### 2.3 (A) 배치 정리 방향(설계 초안, 좌표는 구현자가 확정)

- 원칙: 라벨(caption 18, muted)과 값의 2단 격자를 패널마다 같은 열 폭·같은 간격(최소 16px, 격자 단위 8 의 배수)으로 맞춘다. 가변 문자열(제목·아티스트·장르·이름)은 모두 E2(또는 SHRINK) 대상으로, 고정 영어 라벨만 OVERFLOW 허용.
- 곡 머리(P4): 장르를 제목 위 작은 칩/캡션으로 유지하되, 부제가 비면 아티스트를 위로 당기는 대신 고정 줄 위치를 쓰면 빈 줄이 생기므로 "제목 + 부제를 한 줄(부제는 작은 크기로 이어서)"로 묶거나 `FULLTITLE`(텍스트 12)을 쓰는 방법이 있다. 오른쪽 빈 공간에는 LEVEL·BPM·NPS 같은 큰 수치를 올려 CHART DATA 의 첫 줄을 비울 수 있다.
- CHART DATA(P1): 사실을 한 줄 6칸(NOTES·LEVEL·BPM·LENGTH·NPS·TOTAL 등, 744/6 = 124 폭)과 둘째 줄(KEYS·JUDGE)로 다시 나누거나, LENGTH·TOTAL·KEYS·JUDGE 를 2x2 격자로. 레인 패널·그래프 위치(top 464)는 유지 가능.
- MY SCORE(P2): 통계 넷을 2x2(폭 200씩)로 하거나 라벨을 위, 값을 아래로 두되 칸 폭을 100 균등 + 간격 16.
- 레이더(P5): 그림 한 변 144 -> 160~176 으로 키우고 위 라벨을 꼭짓점 위로 더 올리거나, 위쪽 옆 라벨의 lift(41)를 줄여 NOTES 라벨과 세로로 떨어뜨린다. 값은 라벨 오른쪽 같은 줄에 두면 세로 공간이 줄어든다.
- P7: EX 칸 폭을 140 으로(MISS 를 오른쪽으로 20 밀기) 하거나 num44 대신 5자리일 때 `fit` 을 쓴다. SCORE TARGETS 필요 EX 칸은 `number_x` 64 -> 64, `number_width` 48 -> 72 로 넓히고 차이 칸을 오른쪽으로.
- 참고 자료: [선곡 IIDX 식 ADR](../../acknowledge/2026-10-24-adr-select-iidx-layout.md), [IIDX 선곡 조사](../2026-10-24-select-ui-iidx-survey.md), [선곡 재설계 참고](../ui-select-redesign.md).

### 2.4 (B) 밀도(NPS) 표시 — 정의와 설계안

정의를 먼저 정해야 한다. 코드에 있는 두 정의는 결과가 다르다.

| 안 | 값 | 노트 정의 | 길이 정의 | 바뀔 곳 | 장점 | 단점 |
| --- | --- | --- | --- | --- | --- | --- |
| N1 기존 id(추천: 이번 범위) | 평균 = 364 + 365/100, 최고 = 360 + 361/100 (필요하면 END 362·363) | 일반 + LN 머리 + **LN 몸통이 걸친 초마다 1**, 지뢰 제외 | 평균은 "활동 초"(임계 `total_notes/bins/4` 이상)만, 최고는 1초 칸 | 스킨만: `props.lua` 의 `props.number` 에 `DENSITY_PEAK = 360` ~ `DENSITY_AVERAGE_AFTERDOT = 365` 이름 추가, `info.lua` 에 표시, 문구 키(`select_info.lua`), `apps/rbms-cli/scenarios/select.json` 에 360~365(와 74) 값 추가 | Rust 변경 0. 모든 모드(9·24키 포함). 곡 DB 캐시라 150 ms 뒤 바로 나옴. beatoraja 와 같은 id·의미라 외부 스킨과 일관. 내장 선곡의 PEAK/AVG/END 와 같은 수 | 엄밀한 "초당 노트"가 아니다: 긴 LN 이 많은 차트는 몸통 초 때문에 커지고, 평균은 조용한 초를 뺀다. 정수 + 버림 2자리 |
| N2 누름 기준 새 확장 값 | 평균 = 누름 수 / (첫~마지막 누름), 최고 = 1초(또는 4초/4) 창 최대 누름 | 일반 + LN 머리(꼬리·지뢰 제외), LN MODE 무관 | 첫 누름~마지막 누름(최소 1초) | `rbms-chart` 에 모드와 무관한 함수(레이더의 `collect_presses`·`span_seconds`·`peak_count` 재사용, 창 길이 상수), `ChartMeta` 새 필드(`skin_host/chart.rs:180-198`), 분석 작업자 `Analysed`(`analysis.rs:37-42, 263-268`)와 `describe`(`analysis.rs:212-224`), 확장 id(정수는 21083 다음 21084~ 또는 Float 21026~, `extension.rs:167-216` 과 그 시험의 `NUMBER_RUNS`/`FLOAT_RUNS`), `chart.rs` 응답, `props.lua` `extension_number/float`, docs/skin.md 1.4절 | 사용자가 말한 "note per second" 에 가장 가깝다. 레이더 NOTES 와 같은 노트 정의 | 분석 작업자가 파싱한 뒤에만 나옴(DB 에 없음. 레이더 원시값 `song_radar` 는 5·7·10·14키만이라 재사용하면 9·24키가 빔). 새 id 대역 소비 |
| N3 스킨 계산 | `TOTALNOTES / (SONGLENGTH_MINUTE*60 + SECOND)` | `count_playable_notes`(CN·HCN 꼬리 포함) | 마지막 타임라인(곡 끝 BGM 포함 가능), 초 단위 버림 | 스킨만 | Rust 변경 0, 모든 모드 | 길이가 마지막 노트가 아니라 과소 추정. 초 단위 버림으로 짧은 곡 오차 큼 |

추천: **N1 로 평균과 최고를 표시**(이번 요청의 "가능하면 표시"에 가장 싸고 즉시 나옴), 라벨은 "AVG NPS"·"MAX NPS" 또는 "DENSITY AVG / PEAK" 로 정의 차이를 숨기지 않는 이름. 엄밀한 누름 기준이 필요하면 N2 를 후속으로. 정의 선택은 사용자 결정(open question).

표시 위치 후보:

1. 밀도 그래프 패널 안 왼쪽 위에 caption 한 줄("AVG 7.52/s  MAX 15/s"), 그래프 (352, 464, 400, 136) 안 (360, 472) 근처. 의미상 맞고 다른 칸을 밀지 않는다.
2. 2.3절의 사실 줄을 6칸으로 재배치하며 NPS 칸 하나(`평균 / 최고`).

Lua 구현 메모:

- 정수 id 를 쓰고 `ui.has_value` 로 부재를 거른다(`ui.lua:54-56`). **Float 367 은 부재가 `FLOAT_ABSENT = f32::from_bits(1)`(`crates/rbms-skin/src/property/host.rs:43`)라 `ui.has_value` 가 못 거른다**(1.4e-45 가 "0.00" 으로 보인다).
- 소수 부분 365·361 은 버림 2자리라 `string.format("%d.%02d", whole, rest)` 로 합친다(`info.lua:351-360` 의 `score_rate` 와 같은 방식).
- 상세가 없을 때(커서 이동 중, 폴더·코스)는 그리지 않거나 `-`(기존 `dash` 패턴, `info.lua:380-382`).

---

## 3. 영향받는 기존 기능·시험·골든

### 3.1 시험·게이트

- `apps/rbms-cli` 의 엄격 덤프(선곡 포함 문서 9개, 경고 0, 팩 불변)와 시나리오 `apps/rbms-cli/scenarios/select.json`(지금 74·360~365 없음. NPS 를 그리면 값을 넣어야 덤프에 보인다).
- `apps/rbms-player/src/stage/capture/gate.rs:115-127` `the_default_pack_draws_the_browser`(컴파일된 스킨 장면, 경고 0, 객체 종류, 모서리 배경색). 배치만 바꾸면 영향 없음.
- `crates/rbms-skin/tests/default_pack.rs`(금지 API, 시트 대조, 세 언어 키 완비 `:539-580`). 새 문구 키는 세 언어를 다 써야 통과.
- E2 를 넣으면 `crates/rbms-render/src/skin_render/text/tests.rs:185-230`, `crates/rbms-render/src/font/block_tests.rs` 에 시험 추가. 기존 0~2 동작은 그대로여야 한다.
- N2 를 넣으면 `skin_host/extension.rs` 의 대역 시험(`extension.rs` 아래 `every_id_is_in_its_band_and_no_two_are_the_same`, `no_float_shares_a_number_with_a_rate`, `the_reference_has_no_id_of_these_and_every_gate_lets_them_through`, `every_run_is_as_long_as_what_it_counts`), `skin_host/chart/tests.rs`, `stage/select/analysis/tests.rs`, `rbms-chart` 시험.
- 밀도 기존 시험: `skin_host/chart/tests.rs:145-173`, `crates/rbms-chart/src/tests.rs:232-577`(note_density). N1 은 이것을 건드리지 않는다.

### 3.2 골든

- `crates/rbms-render/tests/golden_select.rs` 는 **내장(스킨 없는) 선곡** 골든이다. 기본 스킨 Lua 변경은 영향이 없다. E2 가 `BlockFit` 기본 동작을 바꾸지 않는 한 영향 없음.
- 기본 스킨 화면의 화소 골든은 없다(캡처는 사람이 보는 opt-in).

### 3.3 캡처 방법(실행해 봄)

```
RBMS_SKIN_CAPTURE_DIR=/private/tmp/claude-501/select-capture \
  cargo test -p rbms-player --lib --locked -- the_default_packs_option_panels_and_graded_lamps_are_captured --nocapture --test-threads=1
```

- 결과: 통과(43.4초, 빌드 제외). 저장소 기본 팩으로 선곡 정보창을 찍는다. 선곡 정보창 캡처는 `/private/tmp/claude-501/select-capture/panels-graded-best-3000ms.png`(1920x1080, 차트 "Iris" 커서). 같은 폴더에 `-1300ms`, 패널 1·2·3, 결과 화면 캡처, 확대 조각 `crop-a.png`(CHART DATA 머리), `crop-b.png`(레이더·MY SCORE)가 있다.
- 하니스: `apps/rbms-player/src/stage/capture.rs`(`browse_app` `:1557-1590`, 고정 차트 `BROWSE_CHARTS` `:1509-1521`, `capture_browser_view` `:1634-1656`), `capture/panels.rs:109-121`. 팩 지정 일반 캡처는 `RBMS_SKIN_PACK=<절대경로>` + `a_skin_pack_named_by_the_environment_is_captured_one_document_a_screen`(`capture.rs:630`). 환경 변수 표는 `docs/skin.md:737-748`.
- 한계: 고정 차트 제목이 모두 짧은 영어라 긴 제목·CJK·긴 장르·큰 숫자를 재현하지 않는다. 캡처는 `use_embedded_fonts_only()`(`capture.rs:225`, `crates/rbms-render/src/font.rs:880-890`)로 번들 Inter 만 쓰므로 CJK 는 두부로 그려진다. 넘침 수리를 눈으로 확인하려면 긴 라틴 제목·부제·아티스트·장르, 5000노트 이상 차트를 가진 캡처 고정물을 더하는 것이 좋다(구현 시 추가 후보).
- 고정 차트의 기록은 노트 6개 차트에 EX 500 처럼 실제로 나올 수 없는 값이다("MAX -56" 등). 캡처 평가 시 무시한다.

### 3.4 문서

- `docs/play-options.md` 17.2절(정보창 표), `docs/skin.md` 3.4.6절(모듈 표, 비용 수치 destination·함수 수), E2 시 `docs/skin.md` 1.4절, N2 시 확장 id 표와 [확장 id 공간 ADR](../../acknowledge/2026-10-24-adr-rbms-extension-id-spaces.md).

---

## 4. 위험·엣지 케이스

1. N1 의 평균·최고는 LN 몸통을 초마다 센다. LN 위주 차트에서 사용자가 체감하는 "초당 노트"보다 크다. 라벨과 문서(play-options.md)에 정의를 적어야 혼동이 없다.
2. `note_density` 는 마지막 노트가 표 한도(`MAX_TABLED_SECONDS`)를 넘으면 셋 다 0 이다(`crates/rbms-chart/src/lib.rs:447-452`). 0 을 "0.00/s" 로 보일지 `-` 로 보일지 정한다.
3. 곡 DB 의 밀도는 스캔 세대(`songdb/schema.rs:15-30`)마다 다시 계산됐다. 오래된 행이 있으면 값이 다를 수 있다(세대 갱신으로 이미 정리됨, 확인은 안 함).
4. 상세 캐시가 없는 차트는 `chart_detail` 이 **프레임 스레드에서** 파싱한다(`stage/select/mod.rs:701-704`, `library.rs:118-133`). 큰 차트에서 첫 방문 프레임이 늦을 수 있다(기존 동작, NPS 와 무관하게 이미 그렇다).
5. E2 의 말줄임표: Inter 에 U+2026 이 있는지 확인 필요(미확인 3). 시스템 대체 글꼴이 그리면 높이가 다를 수 있다.
6. E2 하한 축소율과 줄 자르기는 정렬(center/right)과 함께 시험해야 한다. `slack` 계산이 축소 후 폭을 쓴다(`block.rs:466`).
7. SHRINK 를 E2 로 바꾸면 지금은 끝까지 보이던 긴 제목의 뒷부분이 잘린다. 목록 막대(크기 28, 폭 약 512)까지 바꿀지 범위를 정해야 한다.
8. 문구 키 추가 시 `i18n.merge` 가 두 영역 중복을 오류로 낸다(`i18n.lua:17-27`).
9. `props.lua` 에 360~365 이름을 넣을 때 `props.number` 는 원본 id 이름 체계(`NUMBER_` 접두 없이)를 따른다.
10. 9키·24키는 레이더가 없다. N2 를 레이더 원시값에서 꺼내면 이 모드에 NPS 가 빈다. N1 은 모든 모드에 값이 있다.
11. 코스 탭·폴더 막대에서는 상세가 없어 NPS 가 비어야 한다(`op = { ON_CHART }` 와 값 부재 둘 다).
12. 랭킹 모드(21105)는 MY RECORDS 자리만 바꾼다. 배치를 옮길 때 같은 자리의 두 상태를 함께 맞춰야 한다(`records.lua:9-13`).

---

## 5. 미확인

1. CJK 제목·아티스트·장르가 실제 앱(시스템 대체 글꼴)에서 몇 자부터 눌리는지, 위로 얼마나 솟는지. 캡처 하니스가 번들 글꼴만 써서 재현하지 못했다. 0.15 x 크기는 추정치다.
2. 한국어·일본어 UI 언어에서의 정보창 캡처. 캡처 고정물은 영어로만 찍힌다. 언어를 바꿔 찍는 기존 캡처는 찾지 못했다.
3. Inter-Regular.ttf 에 U+2026(말줄임표) 글리프가 있는지.
4. 선곡의 `TOTALNOTES`(74)가 LN MODE(LN/CN/HCN) 선택에 따라 바뀌는지. 스캔은 LN 종류가 미정인 롱노트를 일반 LN 으로 센다는 주석(`crates/rbms-model/src/lib.rs:173-175`)만 확인했다.
5. 사용자가 본 "text overflow" 가 정확히 어느 문자열인지(곡 머리·목록 막대·통계 라벨 중). 지적 문구만으로는 특정할 수 없어 1.3·2.1절에 후보를 모두 적었다.
6. 실제 라이브러리(긴 제목, 5000노트 이상) 로 그린 선곡 화면. 조사 범위에서 사용자 데이터에 접근하지 않았다.
