# ADR: 선곡 목록 막대·MY RECORDS 줄·랭킹 줄의 램프를 어시스트 등급 색으로 그린다(`songlist` 확장 키 `lamp_graded`, 확장 숫자 21087~21102)

> 최종 갱신 2026-11-19 · 대응 커밋 `df80201`
> 상태: 유효(코드는 `dev` 에 있음. 사용자가 정한 것은 요구 한 줄뿐이고 M8 과 "구현에서 정한 것"은 메인·구현 단위의 결정이며 사용자 확인 전. 실창에서 색을 눈으로 확인하지 않음) · 결정일 2026-11-19 · 결정자 사용자(요구)와 메인(M8), 구현 단위 U4(구현에서 정한 것) · 커밋 `df80201`
> 관계: [어시스트 등급 ADR](2026-10-22-adr-assist-lamp-grade.md)을 확장한다. 그 ADR 은 램프 id 3(LIGHT ASSIST) 안의 등급 0~4 와 색·이름을 정했고, 결과 화면과 선곡 정보창 MY SCORE 에 등급을 보였다. 이 ADR 은 같은 등급 색을 목록 막대·MY RECORDS 줄·랭킹 줄까지 넓힌다. 확장 id 의 배정은 [확장 id 공간 ADR](2026-10-24-adr-rbms-extension-id-spaces.md)의 표에 더했다. 이 단위 뒤에 추가된 요구는 [램프 반 쪼개기](2026-11-19-adr-split-clear-lamp.md)가 이어받는다. 계획 U4, 조사 4절, 보고 [U4.md](../history/2026-11-19-feature-upgrade-unit-reports/U4.md).

## 배경

- 사용자 요청 요지(2026-11-19): 4 스크래치 오토로 깬 normal·hard·exhard 램프를 색으로 구분.
- 등급 정의와 색은 이미 있다. 등급 1~4 = ASSIST EASY(`#3ad19f`)·ASSIST CLEAR(`#c4a796`)·ASSIST HARD(`#afb8ef`)·ASSIST EXHARD(`#ca76a9`), 스킨 토큰 `theme.grade_lamp`·`theme.assist_grade`. 이 색은 선곡 정보창 MY SCORE 의 램프 이름과 결과 화면 램프 이름에만 쓰였다.
- 선곡 목록 막대의 램프(`songlist` 의 램프는 11칸, `BAR_LAMPS = 11`)와 `SongBar.lamp: i32` 에는 등급이 없었다. 그래서 SCRATCH AUTO 로 깬 곡은 게이지와 상관없이 모두 LIGHT ASSIST 색(`{101, 99, 255}`)으로 보였다. 사용자가 본 "다 같은 색"이 이것이다.
- 등급은 플레이어가 이미 갖고 있었다(`RecordLamp`, `SelectBar.assist_grade`). 버려지는 곳은 `stage/select/scene.rs` 의 막대 변환 한 줄이었다(검토 기록 3).

## 결정

### 1) 사용자가 정한 것

| 번호 | 결정 |
| --- | --- |
| D1 | SCRATCH AUTO 로 깬 램프를 게이지(NORMAL·HARD·EXHARD)별로 색으로 구분해 보인다 |

### 2) 메인 결정(사용자 미확인)

| 번호 | 결정 |
| --- | --- |
| M8 | 선곡 목록 막대 램프, MY RECORDS 줄, 랭킹 줄이 기존 등급 색 4개를 쓴다. 폴더 램프 분포와 스크롤 막대 분포는 11칸 그대로 둔다. 새 색은 만들지 않는다(웹·결과 화면과 어긋나므로) |

### 3) 구현에서 정한 것(구현 단위가 정함, 사용자 미확인)

| 항목 | 내용 |
| --- | --- |
| 렌더 쪽 | `SongBar` 에 `assist_grade: u8`(0~4, `ASSIST_GRADE_NONE` = 0)를 더했다(`crates/rbms-render/src/skin_render/songlist/bars.rs`). `songlist` 객체에 R-BMS 확장 키 `lamp_graded`(등급 1~4 의 그림 네 개, `BAR_ASSIST_GRADES` = 4)를 더했다. 기존 확장 키 `rank`·`thumb` 와 같은 경로다: 모델 `SongList.lamp_graded`(`crates/rbms-skin/src/model/objects.rs`), 로더(`loader/branch.rs` 의 `NESTED_ARRAY_FIELDS`, `from_lua.rs`·`script.rs` 의 두 walk, `loader.rs`), 렌더 `songlist/build.rs`. `skin_render/textures.rs` 에 `name_all(&mut named, &list.lamp_graded)` 한 줄을 더해야 `lamp_graded` 만 쓰는 원본 이미지가 로드 대상에서 빠지지 않는다 |
| 그리는 규칙 | 막대의 램프 id 가 3(`LAMP_LIGHT_ASSIST`)이고 등급이 1 이상이며 그 등급의 그림이 있으면, 기존 램프 패스 안에서 `lamp` 항목 대신 그 그림을 그린다(`SongBar::graded_lamp_index`). 새 패스는 없고 prepare 는 `rank` 다음 맨 끝이다. 키가 없는 외부 스킨은 그리기·Lua 호출 순서와 화소가 전과 같다(회귀 시험이 화소 전체를 견준다). 라이벌 모드(`list.rival`)에서는 `playerlamp` 의 위치가 따로라서 대신하지 않는다(시험으로 고정) |
| 플레이어 쪽 | `wheel_bar`(`stage/select/scene.rs`)가 `SelectBar.assist_grade` 를 `SongBar` 로 넘긴다. 차트 막대에만 등급이 채워지고 폴더·코스 막대는 0 이다 |
| MY RECORDS 줄 | 스킨이 받는 값에 등급이 없어서 확장 숫자를 더했다. 숫자 21087~21092(`MY_RECORD_ASSIST_GRADE_FIRST` + 줄 0~5, 줄 0 = BEST, 1~5 = 최근 판, Select 클러스터, `BrowseFacts`·`RecordRow`). `RecordRow::of` 에 등급 인자를 더했다: 최고 기록 줄은 `BestScore.assist_grade`, 최근 판은 `PlayLog::assist_grade()` 를 쓴다 |
| 랭킹 줄 | 숫자 21093~21102(`RANKING_ASSIST_GRADE_FIRST` + 슬롯 0~9, Ir 클러스터). `IrScore.assist_grade` 를 `RankingRow.record_lamp` 에서 채운다. 결과 화면 링크에서는 칸이 없어 `INTEGER_ABSENT` 를 돌려준다 |
| 기본 스킨 | `lua/select/list.lua` 가 `lamp_graded = graded_lamps(s)` 로 `theme.assist_grade[1..4].color` 의 단색 블록 넷을 낸다. `lua/select/records.lua` 는 `lamp_strip` 셀 위에 등급 색 블록을 덮는다: 기록 줄은 램프 id 가 3 이고 등급이 g 일 때, 랭킹 줄은 이미지 인덱스 `390 + 줄` 이 3 이고 등급이 g 일 때 그린다. 램프가 3 인지도 함께 확인해 등급 값이 잘못 와도 다른 램프를 덮지 않는다. `lua/props.lua` 에 `MY_RECORD_ASSIST_GRADE_FIRST`·`RANKING_ASSIST_GRADE_FIRST` 두 이름을 더했다 |
| 건드리지 않은 것 | 폴더 분포 그래프, 스크롤 막대 분포, 내장(스킨 없는) 선곡 막대의 LED(이미 `stored_lamp_label_color` 가 등급을 따른다), 라이벌 모드의 `playerlamp`. 서버·웹 |

### 확장 숫자 id 의 범위를 넘은 점

계획은 21087~21099(13개)였으나 기록 6줄과 랭킹 10칸에 16개가 필요해 랭킹 run 이 21102 까지 간다. 정수 확장 대역은 21000~21199(`EXTENSION_VALUE_LAST`)라 정의상 맞고, 불린 21100 대와는 공간이 달라 충돌하지 않는다. 21085(`PLAY_NPS`)와도 겹치지 않는다. 범위 안에 두려면 랭킹 칸을 7칸으로 줄여야 해서 택하지 않았다. 정수 21086(계획의 `PLAY_GAUGE_KIND`)은 만들지 않았다([플레이 화면 ADR](2026-11-19-adr-play-screen-run-facts.md)). 정수 공간의 다음 빈 번호는 21103 이다.

## 이유

- 같은 정보(어떤 게이지로 깼나)를 결과 화면과 정보창은 색으로 구분하는데 목록에서만 구분이 없는 불일치를 없앤다. 새 색이 없어 웹·결과 화면과 같다.
- 렌더러에 등급 그림 목록을 더하는 방식은 기존 확장 키와 같은 경로라 외부 스킨에 영향이 없고(키가 없으면 전과 같다) 다른 스킨도 같은 키를 쓸 수 있다.
- 폴더 분포 11칸과 스크롤 막대 분포는 램프 id 0~10 의 개수이므로 등급으로 쪼개지 않는다. 그래프의 의미와 그 값을 읽는 외부 스킨을 바꾸지 않기 위해서다.

## 기각된 대안

| 대안 | 기각 이유 |
| --- | --- |
| 새 색 | 웹·결과 화면의 등급 색과 어긋난다 |
| 램프 칸을 15칸(11 + 등급 4)으로 늘리기 | 분포 그래프와 외부 스킨이 읽는 `BAR_LAMPS` 를 바꾼다 |
| 별도 패스로 등급 그림을 위에 덧그리기 | 키가 없는 문서의 그리기 순서가 달라질 수 있다. 램프 패스 안에서 대신 그리면 순서가 전과 같다 |
| MY RECORDS·랭킹 줄에 등급 값이 오도록 기존 램프 값 바꾸기 | 램프 id(0~10)를 외부 스킨이 읽는다. 별도 확장 숫자로 둔다 |
| 랭킹 칸을 7칸으로 줄여 21099 안에 두기 | 랭킹 슬롯 10칸이 줄어든다 |

## 영향과 관련 파일

- 코드: `crates/rbms-render/src/skin_render/{songlist.rs, songlist/{bars,build,tests}.rs, textures.rs}`, `crates/rbms-skin/src/{model/objects.rs, loader.rs, loader/{branch,from_lua,script}.rs}`, `crates/rbms-skin/tests/{skin_from_lua,skin_luaskin}.rs`(songlist 필드 목록), `apps/rbms-player/src/skin_host/{extension.rs, ir.rs, mod.rs, select/browse.rs}`(라우트 2줄), `stage/select/{scene.rs, browse_facts.rs, bars_tests.rs}`, `assets/skins/rbms-default/lua/{props.lua, select/list.lua, select/records.lua}`.
- 시험: 렌더 `every_assist_grade_of_a_light_assist_lamp_draws_a_lamp_of_its_own`(등급 0~4 가 다른 그림·화소 색), `a_graded_lamp_stands_in_only_for_a_light_assist_lamp_it_has_an_image_for`, `a_wheel_without_graded_lamps_draws_every_grade_as_the_light_assist_lamp`(키 없는 문서는 화소 전체가 같음), `skin_host/extension.rs` 의 확장 id 대역 시험(`ids_inside`), `skin-dump --strict --frames 120` 통과.
- 알려진 위험(테스트 부채 3.171·3.172, 실기 QA 3.29 G7):
  - 실제 창이나 캡처로 막대와 기록 줄의 등급 색을 눈으로 확인하지 않았다. `apps/rbms-cli/scenarios/select.json` 에 램프 3 이나 등급 값이 없어 스킨 덤프는 등급 블록이 그려지는 경로를 지나지 않았다.
  - 랭킹 줄의 등급은 서버 응답의 `assist_grade` 에 기댄다. 구 서버가 0 을 보내면 LIGHT ASSIST 색으로 남는다.
  - 선곡 화면 개편(U8)이 `list.lua`·`records.lua` 를 같이 고쳤다. U8 단위가 개편 뒤에도 등급 블록이 `cell()`·`row_box()` 를 따라 램프 칸에 겹치는 것을 `recorded-scratch-auto` 캡처로 확인했다고 보고했다. 이 문서를 쓴 쪽은 그 캡처를 다시 보지 않았다.
- 문서: [어시스트 등급 ADR](2026-10-22-adr-assist-lamp-grade.md) 4절·8절에 표시 목록을 덧붙였고, [skin.md](../skin.md) 1.4절에 `lamp_graded` 와 새 숫자를 적었다.

## 날짜와 사용자 원문

2026-11-19.

- 사용자 요청 요지(PROCESS 체크리스트): "4 스크래치 오토로 깬 normal·hard·exhard 램프 색 구분". 원문 전문은 이 저장소에 없다.
