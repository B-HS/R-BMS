# ADR: 선곡 정보창의 글자 넘침은 엔진의 새 overflow 값 3(SHRINK_CUT)으로 고치고, NPS 는 beatoraja 밀도 값을 그대로 쓴다

> 최종 갱신 2026-10-29 · 대응 커밋 `cf8afe5`
> 상태: 유효(D5 는 사용자 확인 2026-10-29. M10 과 "세부" 절은 메인 추천 또는 구현 단위의 결정, 사용자 미확인. 한국어·일본어 모습은 실창 확인 전) · 결정일 2026-10-29 · 결정자 사용자(요청과 D5)와 메인(M10), 구현 단위 U3(세부) · 커밋 `cf8afe5`
> 부분 변경하는 결정: [선곡 IIDX 식 ADR](2026-10-24-adr-select-iidx-layout.md)의 메인 결정 3 중 "곡 머리의 큰 BPM(BPM 은 CHART DATA 에만)을 뺐다"를 되돌린다(아래 3절). 그 ADR 의 나머지, 곧 IIDX 식 구성과 모노톤 + 강조색 넷(난이도색·등급·램프·메달·레이더 외 새 색 없음)은 그대로다.
> 관계: 글자 래스터는 [UI 글자 장치 화소 ADR](2026-10-22-adr-ui-text-device-pixels.md), 언어와 용어는 [UI 다국어 ADR](2026-10-24-adr-ui-language.md)·[용어집](../reference/2026-10-24-i18n-glossary.md). 설계 [plan/2026-10-29-feature-upgrade.md](../plan/2026-10-29-feature-upgrade.md), 조사 [reference/2026-10-29-feature-upgrade/select-info-density.md](../reference/2026-10-29-feature-upgrade/select-info-density.md).

## 배경

- 사용자 요청(2026-10-29): "곡 선택화면에서 곡 정보 표시하는거 조금 더 UI/UX 좋게 만들어봐 지금 너무 중구난방에 text overflow도보여서 좀 보기 힘들어" 와 "곡 선택화면에서 밀도(note per second) 도 표시가능하면 좋을듯하네".
- 정보창의 긴 글(곡 머리 네 줄, 목록 막대 제목, 패널 제목, 레이더 축 이름, 램프 이름, 기록 이름)은 엔진의 overflow 1(SHRINK)이었다. SHRINK 는 가로 축소에 하한이 없어 아주 긴 제목이 납작하게 뭉개지고, overflow 0 인 곳은 칸 밖으로 넘쳤다. 5자리 EX·MISS, 레이더 위쪽 라벨 군집, SCORE TARGETS 의 큰 숫자도 이웃 칸을 덮었다. 엔진의 SHRINK 에 하한을 두는 일은 엔진에서만 할 수 있었다.
- beatoraja 에는 선곡 밀도 값이 이미 정의돼 있다: 정수 id 364/365(평균, 정수부·소수 두 자리), 360/361(최고). 이 플레이어는 곡 DB 의 `song_detail` 에 평균·최고·끝 밀도를 갖고 있고 `skin_host/chart.rs` 가 이 id 에 답한다. 기본 스킨 선곡만 이 값을 그리지 않았다.

## 결정

### 1) 사용자가 정한 것

| 번호 | 결정 |
| --- | --- |
| D5 | NPS 는 beatoraja 밀도 값(정수 id 364/365 평균, 360/361 최고)을 그대로 쓴다. LN 몸통이 걸친 초마다 1개씩 센다(사용자 답 "2번으로"를 NPS·배치 기억 두 질문 모두 2번 선택지로 읽었다) |

### 2) 메인 추천(사용자 미확인)

| 번호 | 결정 |
| --- | --- |
| M10 | 선곡 정보창 넘침은 엔진의 새 overflow 값 3(하한 비율 0.75 까지 가로 축소 뒤 넘치면 잘라 `…`)으로 고치고, 정보창과 목록 막대 제목에 쓴다. 영어 상수 라벨(EX SCORE 등)은 용어집대로 원어를 유지한다 |

### 3) 엔진: overflow 3(`SHRINK_CUT`)

| 항목 | 내용 |
| --- | --- |
| 값 | 텍스트 객체의 `overflow = 3`. 이 플레이어 전용이고 기본 스킨 `props.text_overflow.SHRINK_CUT` 이 이름을 붙인다. beatoraja 는 3 을 OVERFLOW(0)로 읽으므로 이 값을 쓴 외부 스킨은 원본에서 다르게 보인다 |
| 동작 | 줄이 폭을 넘으면 `폭 / 줄폭` 만큼 가로로 줄이되 하한 `SHRINK_CUT_FLOOR`(0.75)까지만. 그보다 줄여야 하면(줄이 `폭 / 0.75` 를 넘으면) 앞쪽 글리프만 남기고 U+2026 `…` 를 붙인 뒤 폭에 맞게 줄인다. 폭/줄폭이 0.75 이상이면 SHRINK 와 결과가 같다. 끝의 공백 글리프는 말줄임표 앞에서 뗀다 |
| 대체 | 글꼴과 대체 글꼴이 모두 U+2026 을 못 그리면 `...` 를 쓴다. 번들 Inter 는 U+2026 을 갖는다(시험이 확인했다) |
| 우선순위 | `wrapping` 이 켜지면 줄바꿈이 이긴다. 0·1·2·줄바꿈의 동작과 기존 골든은 바뀌지 않았다 |
| 비트맵 글꼴 | SHRINK 와 같게 줄이기만 하고 자르지 않는다 |
| 구현 | `crates/rbms-render/src/font/block.rs`(`BlockFit::ShrinkCut`, `SHRINK_CUT_FLOOR`, `cut_with_ellipsis`, `BlockFit::squeezes()`), `font.rs`(재수출), `skin_render/text.rs`(`TEXT_OVERFLOW_SHRINK_CUT`), `bitmap_font.rs` |

### 4) 기본 스킨 정보창(구현 단위가 정함, 사용자 미확인)

좌표의 단일 출처는 `lua/theme.lua` 의 `theme.select_layout`(`gap`, `song`, `facts`, `chart_view`, `score_rows`, `target_cells`)과 `theme.radar` 다. 공통 규칙은 칸 사이 간격 16 이상, 라벨(caption muted)을 값 위에 두고 같은 줄의 값은 같은 baseline, 긴 글은 모두 overflow 3.

| 영역 | 바뀐 점 |
| --- | --- |
| 곡 머리 | 글 열을 폭 792(x 104~896)로 줄이고 오른쪽 폭 168(x 928~1096)에 LEVEL(라벨 + `num64` 난이도 색)과 BPM(라벨 + body 32)을 오른쪽 맞춤으로 옮겼다. 곡 머리 오른쪽의 빈 공간을 쓰기 위해서다(U3 보고는 사용자의 "곡 머리 공간 활용" 요청을 근거로 들었고 그 원문은 확인하지 못했다). CHART DATA 의 LEVEL·BPM 칸은 없어졌고 위 ADR 의 "BPM 은 CHART DATA 에만"이 바뀐다. 그 항목은 사용자가 직접 정한 결정이 아니라 메인 결정이었다 |
| CHART DATA | 사실 칸 일곱을 두 블록으로: 왼쪽(x 352, 칸 폭 120, 간격 16) 첫 줄 NOTES·NPS AVG·NPS MAX 와 그 아래 밀도 그래프(352, 456, 400x152). 오른쪽(x 784, 칸 폭 144) 2x2 LENGTH·TOTAL / KEYS·JUDGE 와 그 아래 레인 막대(784, 516, 312x92). 값은 모두 24 크기, 첫 줄 baseline 440, 둘째 줄 496 |
| MY SCORE | EX SCORE(`num44`, 폭 144)·MISS COUNT(`num32`, 폭 96)·등급 배지(폭 128, 높이 56) 사이 16. 통계 넷은 2x2 라벨-값 쌍(쌍 폭 184, 쌍 사이 32): MAX COMBO·RATE / PLAY COUNT·CLEAR COUNT |
| 레이더 | 그림 한 변 144 → 160(반지름 69.75 → 77.5), 중심 아래로 8, 위쪽 옆 라벨 lift 41 → 33, 아래쪽 옆 라벨 drop 12 → 16, 아래 라벨 간격 8 → 18. 위 라벨과 옆 라벨이 2px 안에 붙던 것이 13px 떨어진다 |
| SCORE TARGETS | 칸 피치 176 → 200. 5자리 필요 EX 와 4자리 차이(`+2243`)가 칩·이웃 칸을 덮지 않는다. CLEAR RATE 는 x 984 폭 112 |
| 목록 막대 | 제목 텍스트 11종 overflow 1 → 3 |

### 5) NPS 표시(D5 의 구현)

- 값: `props.number.DENSITY_AVERAGE`(364)·`DENSITY_AVERAGE_AFTERDOT`(365)와 `DENSITY_PEAK`(360)·`DENSITY_PEAK_AFTERDOT`(361). 새 id 는 만들지 않았다(기존 beatoraja id 에 이름만 붙였다). 정수 id 만 써서 Float 의 부재값 함정을 피했다.
- 정의(`rbms_chart::note_density`): 1초 칸으로 나눠 센다. 일반 노트·롱노트 머리는 시작한 초에 1, 롱노트 몸통은 머리 다음 초부터 끝나는 초까지 걸친 초마다 1, 지뢰는 세지 않는다. NPS MAX 는 가장 바쁜 1초, NPS AVG 는 `전체 노트 수 / 초 칸 수 / 4`(정수 나눗셈) 이상인 활동 초의 평균이다. 그래서 LN 이 많은 차트에서는 엄밀한 초당 누름 수보다 크다.
- 표시: `%d.%02d` 로 합친다. 소수 두 자리는 반올림이 아니라 버림이다(`digits_after_dot`). 정수부·소수부 중 하나라도 값이 없으면 그리지 않고(커서 이동 중, 폴더·코스 막대) 둘 다 0 이면 `-` 를 보인다. 평균·최고가 모두 0 이 되는 것은 노트가 없거나 마지막 노트가 24시간(`MAX_TABLED_SECONDS` 86,400 초) 이상이라 초 칸 표를 만들지 않는 차트다.
- 라벨: `info_nps_average`(NPS AVG / NPS 평균 / NPS 平均)·`info_nps_peak`(NPS MAX / NPS 최대 / NPS 最大). NPS 는 원어이고 한국어는 사용자 검수 전, 일본어는 원어민 감수 전이다.
- 스킨 덤프 시나리오 `apps/rbms-cli/scenarios/select.json` 에 360~365·368·1163·1164 의 값을 더했다.

## 이유

- 엔진의 SHRINK 에 하한이 없다는 문제는 스킨 쪽(Lua)에서 고칠 수 없다. 새 overflow 값은 기존 0/1/2 와 겹치지 않아 기존 스킨과 골든이 그대로다.
- 0.75 까지 줄이고 그 아래는 자르는 것은 긴 제목이 읽을 수 있는 크기를 유지하면서 칸을 넘지 않게 한다. 말줄임이 정보를 잃는 쪽(아주 긴 제목의 뒷부분)은 사용자 요청 "보기 좋게"와 같은 방향의 의도된 동작이다.
- beatoraja 의 밀도 id 를 그대로 쓰면 외부 스킨과 같은 값을 보고 새 id 공간이 필요 없다.
- 영어 상수 라벨은 용어집이 원어를 유지하기로 한 것(게임 용어)이다.

## 기각된 대안

| 대안 | 기각 이유 |
| --- | --- |
| 스킨만으로 고침(크기 축소, 간격 확보, 문자 수 상한 + `...`) | Lua 는 글자 폭을 몰라 문자 수 상한이 라틴과 CJK 의 폭 차이(약 2배)를 못 맞춘다. 납작해짐(SHRINK 에 하한 없음)은 그대로 남는다. 배치 정리는 이 방식으로 하고 넘침은 엔진 확장과 함께 쓴다 |
| 제목에 줄바꿈 2줄 | 줄 수 상한이 없어 3줄째가 부제·아티스트를 덮고, 줄바꿈이 SHRINK 를 이긴다 |
| 밀도를 누름 기준(LN 몸통 제외)으로 새로 정의하는 확장 값 | 새 확장 id 와 분석 작업자 의존이 생기고 beatoraja 값과 달라진다. D5 가 beatoraja 값 그대로를 정했다 |
| 스킨에서 `TOTALNOTES / 곡 길이` 로 계산 | 길이가 마지막 노트가 아니라 과소 추정이고 초 단위 버림으로 짧은 곡의 오차가 크다 |
| 영어 상수 라벨(EX SCORE 등)도 번역 | 용어집이 원어를 유지한다 |

## 영향과 관련 파일

- 엔진: `crates/rbms-render/src/{font.rs, font/block.rs, bitmap_font.rs, skin_render/text.rs}`와 각 시험.
- 기본 스킨: `lua/{theme,props}.lua`, `lua/select/{info,radar,records,list}.lua`, `lua/i18n/select_info.lua`, `apps/rbms-cli/scenarios/select.json`. 새 토큰 `theme.select_layout.{gap, song, facts, chart_view, score_rows, target_cells}`, `theme.radar.label_gap_below`. `apps/rbms-player/src/stage/capture.rs` 에 `mod long_text;` 한 줄과 새 캡처 `capture/long_text.rs`(긴 제목·부제·아티스트·장르, 긴 막대 제목, 6272 노트 차트에 EX 12000·MISS 12345 인 기록).
- 시험: `font/block_tests.rs` 의 `shrink_cut_*` 넷과 `a_line_that_fits_is_left_alone_by_every_overflow_rule`(ShrinkCut 추가), `skin_render/text/tests.rs` 의 `overflow_three_squeezes_to_the_floor_then_cuts_with_an_ellipsis_and_zero_to_two_are_unchanged`, `bitmap_font/tests.rs` 의 `shrinking_and_cutting_squeezes_a_bitmap_line_the_way_shrinking_does`, 캡처 `the_default_packs_info_window_is_captured_with_texts_and_numbers_longer_than_its_boxes`, 스킨 덤프(strict)와 `default_pack` 시험.
- 모노톤 결정: 새 색을 만들지 않았다. IIDX 의 문구·로고를 쓰지 않았다.
- 알려진 위험(테스트 부채 3.136~3.138, 실기 QA 3.25 V10·V11):
  - 캡처 하니스는 번들 Inter 만 써서 CJK 를 두부로 그린다. 한국어·일본어 UI 의 실제 모습, 곡 머리 글 열이 792 로 줄어(전 992) CJK 제목이 라틴보다 일찍 말줄임이 나오는지, 통계 쌍 라벨이 잘리거나 겹치는지는 실창에서만 본다.
  - 목록 막대 제목과 곡 머리를 overflow 3 으로 바꿔, 이전에는 납작하게라도 끝까지 보이던 아주 긴 제목의 뒷부분이 이제 잘린다(정보 손실).
  - DP(10·14키) 레인 패널을 높이 136 에서 92 로 줄였다. 계산상 겹침은 없으나(줄 높이 43, 막대 높이 18) 실창 캡처로 보지 않았다.
  - MY RECORDS 의 MISS 는 `num18` 4자리 칸이라 10000 이상이면 아래 4자리만 보인다(기존 한계, 이번에 손대지 않았다).
  - 글꼴이 U+2026 을 못 그릴 때 `...` 로 바뀌는 경로는 재현할 글꼴이 없어 시험하지 못했다.
  - 통계 쌍의 라벨(MAX COMBO 등)은 원어 상수라 카탈로그 밖 하드코딩이다(기존과 같음).
- 문서: [play-options.md](../play-options.md) 17.2절, [skin.md](../skin.md) 1.4절·3.4.6절.

## 날짜와 사용자 원문

2026-10-29.

- 사용자 요청: "2. 곡 선택화면에서 곡 정보 표시하는거 조금 더 UI/UX 좋게 만들어봐 지금 너무 중구난방에 text overflow도보여서 좀 보기 힘들어 3. 곡 선택화면에서 밀도(note per second) 도 표시가능하면 좋을듯하네"
- 사용자 결정: NPS 는 beatoraja 밀도 값 그대로(평균·최고)("2번으로"). 밀도 표시 형태(평균·최고 NPS)는 메인이 정한다고 위임했다.
