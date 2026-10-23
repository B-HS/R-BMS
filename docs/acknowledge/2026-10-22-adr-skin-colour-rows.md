# ADR: 스킨 색 행 (헤더 `color`, dst·노트 `tint`, 저장값이 있을 때만 곱셈)

> 최종 갱신 2026-10-22 · 대응 커밋 47a7f9c
> 상태: 유효(사용자 확인 2026-10-23: "1-11 추천대로") · 결정일 2026-10-22 · 결정자 메인(사용자 지시 "끝까지 다 구현해놔"에 따라 추천안 채택, 사용자 미확인) · 커밋 49dfbd3(데이터 층), 7459bf7(노트 곱셈·메뉴), 5192bbb(기본 스킨 색 행·순백 노트 시트), c1c992e(LN 전용 색·기본값·알파 순서)
> 관계: 기존 색 옵션 넷(Theme, Accent, Note Colors, Bomb Color)과 부위별 불투명도 18행은 그대로 두고 그 위에 얹는다. 우선순위는 사용자 색 행 > 프리셋 옵션 > 제작자 기본값이다.

## 배경

- 사용자 지시(2026-10-22, 요지. 원문이 아니라 `docs/PROCESS.md` 의 기록): 레인·노트 색상과 불투명도를 각각 조절하고 스킨 세부 조절을 넓힌다.
- 색이 PNG 에 구워진 부위는 노트 계열뿐이고 나머지(레인 면, 구분선, 마디선, 판정선, 커버, 키빔, 폭발)는 Lua 가 destination 색으로 입힌다. 엔진은 노트를 그릴 때 destination 색을 읽지 않고 알파만 남기며(beatoraja 도 같다), 오프셋은 위치·크기·각도·알파만 바꾸고 RGB 는 바꾸지 못한다. 사용자가 입력한 값을 색으로 받는 길이 없었다([skin-color-customization.md](../reference/skin-color-customization.md) 0절).
- 오프셋은 값이 바뀌어도 문서를 다시 읽지 않는다. 옵션은 다시 읽고 그동안 미리보기가 어두워진다(같은 문서 4.6절).

## 결정

### 1) 엔진 확장 + 기본 스킨 채택 (조사 문서 안 C)

색 행은 beatoraja 의 오프셋과 같은 id 방식이다. 값은 프레임마다 저장값에서 읽혀 문서를 다시 읽지 않는다.

- 헤더 `color` 행(R-BMS 확장): `ColorDef { category, name, label, id, def, drawn }`(`crates/rbms-skin/src/model.rs:333`). `def` 는 `RRGGBB` 또는 `RRGGBBAA`(`#` 허용), `label` 이 비면 `name` 을 보인다. 색 id 공간은 오프셋 id 와 따로이고 0 이상이며 색 행끼리 유일해야 한다. 음수·중복 id, 중복 이름, 읽을 수 없는 `def` 는 로드 경고이고 스킨은 정상 로드된다. JSON 과 Lua 모두 최상위 목록 `color` 다.
- dst 참조 `tint`: `Destination.tint: Option<i32>` 가 `DestinationTrack.tint` 로 옮겨진다(음수는 없음). 노트는 `NoteSet.tint: Vec<i32>`(레인과 나란한 색 id, 닿지 않는 레인과 `NO_TINT` = -1 은 틴트 없음)와 `NoteSet.long_tint`(롱노트·HCN 몫, 비어 있으면 `tint` 를 따른다)다. Lua 표는 구멍이 메워지므로 구멍 대신 -1 을 쓴다.
- 해석: `OffsetSource::color(id) -> Option<SkinColor>`(기본 `None`, 기존 구현체는 수정 없이 컴파일). `resolve` 가 색 id 가 있고 값이 있을 때만 `color.tinted(tint)` 를 한다(`crates/rbms-skin/src/dst.rs:631-633`). 틴트는 곱셈이다: 네 채널을 곱해 가장 가까운 값으로 반올림하므로(`multiply_channel`) 흰 바탕에 건 틴트는 정확히 그 색이 된다. 조사 문서 6.3절은 "r,g,b 를 바꾼다"고 적었지만 구현은 곱셈이다(`tinted` 한 곳만 바꾸면 교체 방식이 된다).
- 알파: 불투명도 오프셋을 먼저 더하고(`shifted_alpha`, `dst.rs:630`) 그 위에 틴트의 알파를 곱한다. 노트도 같은 순서다(필드 불투명도 뒤에 `lane_tinted`). 그래서 색의 알파는 "그 부위에 주어진 불투명도의 몇 분의 몇"이고 모든 부위에서 같다. 처음 구현(49dfbd3)은 dst 부위에서 곱셈을 먼저 하고 오프셋을 더했다. 통합 리뷰(colors-2)가 같은 설정에서 레인은 0%, 노트는 25% 가 되는 불일치를 짚어 c1c992e 에서 하나로 맞췄다. 틴트가 없거나 값이 없으면 코드가 건너뛰어 결과가 비트 단위로 같다.
- 실시간 반영: 저장소는 `SkinCustomisation.colors: BTreeMap<i32, SkinColor>`(`crates/rbms-config/src/schema.rs:323`)이고 `impl OffsetSource for SkinCustomisation` 이 `color(id)` 를 답한다. 앱은 `DocumentOffsets::color`(`skin_screen.rs:283`)가 저장값을 그대로 돌려주고(길이 스케일 없음) `ScreenHost::color`(`skin_host/mod.rs:946`)가 그것에 위임한다. 포워딩을 빠뜨리면 컴파일 오류 없이 색이 안 먹는 함정이 있어 `skin_host/tests.rs` 의 `a_stored_colour_tints_the_next_frame_through_the_host` 와 `skin_screen` 테스트가 지킨다.
- Lua: 본체 패스에서 `skin_config.color[이름] = { r, g, b, a, custom }`(헤더 패스에서는 `skin_config == nil`). 값은 저장값이고 없으면 행의 `def`, `custom` 은 플레이어가 저장한 값인지 여부다. 로드 시점 값이며 실행 중 갱신되지 않는다. beatoraja 는 `color` 목록과 `tint` 키를 무시하므로 겸용 스킨은 `(skin_config.color or {})` 로 막는다.

### 2) 저장값이 있는 부위만 틴트한다 (재로드 규칙)

- 저장값이 없으면 그 부위는 프리셋 색을 dst 에 굽고 `tint` 를 걸지 않는다. 화면은 이전과 화소 단위로 같다(기본 스킨 캡처 94장이 변경 전과 모두 동일).
- 저장값이 있으면(`custom` 참) 그 부위의 dst 기본색을 흰색으로 두고 `tint = 색 id` 를 건다. 노트는 순백 시트의 그림을 고르고 `note.tint` 의 그 칸에 id 를 넣는다. 아무 레인도 `custom` 이 아니면 `note.tint` 를 만들지 않는다.
- 문서를 다시 읽는 경우는 한 가지다: 효과 맞춤값(플레이어 값 위에 팩 조절값을 깐 값)에서 그 id 의 "있음" 여부가 바뀔 때(없음 → 있음, 있음 → 없음). 값이 다른 값으로 바뀔 때는 다시 읽지 않고 다음 프레임에 새 값이 곱해진다(`skin_select.rs` `put_color`). `0` 키(초기화)가 저장값을 지우면 재로드 1회이고, 지운 뒤 맞춤값이 비면 그 문서의 항목을 통째로 잊는다(`config.skin.forget`). `A` 키(다른 화면에 적용)는 받은 화면 전부를 stale 로 표시한다.
- 메뉴가 저장 전 색 행에 보이는 값은 `def` 가 아니라 `drawn`(그 문서가 지금 옵션으로 실제 그리는 색)이다. `def` 는 헤더 패스가 옵션을 볼 수 없어 Theme·Accent·Note Colors·Bomb Color 를 반영하지 못해(통합 리뷰 colors-1), 본체 패스(`colors.paint`)가 행마다 실제로 그리는 색을 계산해 `drawn` 으로 발행한다. `lua_skin.rs` 가 id 로 헤더 행에 합치고(`ColorDef.drawn`, `drawn_color()`), `SkinLibrary::unset_color` 가 불러온 문서에서 읽어 `color_for`·`number_for`·프리셋 출발값·견본에 쓴다. 문서에 `drawn` 이 없으면 `def` 로 물러난다.

### 3) 저장·내보내기 호환

- `settings.ron` 의 `skin.custom[문서 경로].colors` 에 id → 색으로 저장한다. `#[serde(default)]` 이고 스키마 판 6 은 그대로다. 색이 없는 옛 파일이 그대로 읽히고, 새 파일을 이전 빌드가 읽으면 `colors` 를 무시하고 다음 저장 때 잃는다.
- 직렬화는 조사 문서가 제안한 `[u8; 4]` 가 아니라 `SkinColor` 구조체다(RON 에서 `(r: 255, g: 0, b: 0, a: 255)`). `SkinOffset` 이 이미 같은 방식이고 읽기 쉬우며, 손으로 고칠 때 `a` 를 빼면 불투명(255), r·g·b 를 빼면 0 으로 읽혀 길이 3 튜플이 파일 전체 로드를 깨는 일이 없다. 의미는 같다.
- 내보낸 조절값 파일(`rbms-adjustments.ron`)에 `colors: 이름 → "#RRGGBBAA"` 를 더했다(`skin_select/adjustments.rs`). `ADJUSTMENTS_VERSION` 은 1 그대로이고, 가져올 때 id 는 가져오는 문서의 이름 → id 로 다시 매긴다. 문서가 선언하지 않는 이름과 읽을 수 없는 색 줄은 건너뛴다.

### 4) 노트 틴트의 대상과 제외

- 틴트를 받는 것: 일반 노트, 떨어지는 놓친 노트, LN·CN·HCN 의 머리·몸통·끝(`NoteLane.tint`, `long_tint`; `crates/rbms-render/src/skin_render/notes.rs`).
- 받지 않는 것(의미색): 지뢰, hidden, processed(처리한 노트 표시), HCN 몸통 중 닳는 중(`HELL_BODY_DRAINING`), SCRATCH AUTO 의 초록 노트(`AUTO_NOTE_IMAGE`, `AUTO_LONG_IMAGES`). SCRATCH AUTO 초록은 사용자 색보다 우선한다(기존 동작 유지).
- 곱셈 틴트의 한계: HCN 을 누르는 중 가운데 레일은 원래 노트색보다 밝은 하이라이트였지만 순백 시트에서는 흰색 x 사용자 색이라 노트색 그대로 나온다.
- 롱노트는 처음에 노트 행 하나로 함께 칠했다(엔진이 레인당 `tint` 하나만 받았기 때문). 통합 수리에서 `NoteSet.long_tint` 를 더해 LN 행이 따로 생겼다. 비어 있거나 LN 행이 저장되지 않았으면 노트 색을 따른다.

### 5) 메뉴 (SKIN·PLAY)

- 색 행 한 줄: 라벨, `#RRGGBBAA` 와 색 견본 사각형(16px), 저장값이 없으면 `DEFAULT`. ← → 는 16색 프리셋(`FFFFFF C0C0C0 808080 404040 000000 FF4040 FF9020 FFE040 A0FF40 30E050 40E0FF 40A0FF 4060FF A060FF FF40C0 FF90C0`, 알파 유지, 목록 밖 색은 오른쪽이 첫 항목, 왼쪽이 마지막 항목)를 돈다. Enter 는 16진 입력(6자리 또는 8자리, `#` 생략 가능, 잘못된 입력은 `NOT A COLOR` 1.5초), `0` 은 `DEFAULT`, `A` 는 같은 이름의 색 행을 가진 다른 화면에 적용(`DEFAULT` 를 적용하면 상대의 저장값을 지운다)이다.
- `COLOR CHANNELS` 접이 행(닫힌 채 시작, 저장하지 않음) 아래에 색 행마다 R·G·B·A 숫자 행(0~255, 슬라이더 + 직접 입력, `0` 키는 출하값)이 있다. SKIN 메뉴는 색 행을 property·file 뒤에 평평하게 두고, PLAY 메뉴는 SKIN 그룹 맨 앞에 접이 `COLOR`·`COLOR CHANNELS` 를 둔다. 색 행 마우스 클릭은 프리셋 한 칸 이동이고 값 영역 클릭으로 16진 입력을 여는 동작은 없다.
- 도움말: "Color of this part. Left/right: preset, ENTER: type a hex color, 0: back to the skin default. Alpha multiplies its opacity". 이 문구의 "alpha multiplies its opacity" 는 위 알파 순서와 같은 뜻이다.

### 6) 기본 스킨 색 행

행 목록·id·기본값은 `assets/skins/rbms-default/lua/play/colors.lua` 한 곳에 있다. id 와 이름은 모든 플레이 문서에서 같다(문서마다 해당하는 행만 선언). 5·7·10·14키 문서는 17행, 9키 문서는 22행이다. 카테고리는 문자열 `Lane`·`Note`·`Effect` 이고 메뉴에는 `LANE > WHITE KEY` 처럼 보인다.

| 묶음 | 행(id) |
| --- | --- |
| Lane | White Key Lane 1, Black Key Lane 2, Scratch Lane 3, Lane Divider 4, Bar Line 5, Judge Line 6, Lane Cover 7 |
| Note | White Key Note 10, Black Key Note 11, Scratch Note 12. 9키: White·Yellow·Green·Blue·Red Button Note 13~17 |
| Note (롱노트) | White Key LN 40, Black Key LN 41, Scratch LN 42. 9키: White·Yellow·Green·Blue·Red Button LN 43~47 |
| Effect | White Key Beam 20, Black Key Beam 21, Scratch Beam 22. 9키: White·Yellow·Green·Blue·Red Button Beam 23~27. 공통 Bomb 30 |

- `def` 는 제작자 기본값이고 기본 테마(Mono, Accent White, Note Colors White-Blue) 값이다. 면 `161616`/`000000`, 선 `3C3C3C`, 판정선 `FFFFFF`, 커버 `262626`, 노트·빔 `F1F4F8`·`4CC9F0`·`F0564C`(9키는 `F2B33D`·`59D185`·`568CF5` 도), 폭발 `F1F4F8`. 메뉴가 실제로 보이는 값은 위 2) 의 `drawn` 이다.
- 9키: 레인 면은 문서의 실제 명암(홀수 버튼은 panel, 짝수 버튼은 bg)을 따라 이름은 7키와 같은 White Key Lane·Black Key Lane 이고 라벨만 `Odd Buttons`·`Even Buttons` 다. 노트·LN·빔은 실제 버튼 색 다섯 묶음(흰 1·9, 노랑 2·8, 초록 3·7, 파랑 4·6, 빨강 5)으로 나눈다. 스크래치 행과 흰·검 건반 노트·빔 행은 9키에서 선언하지 않는다.
- 노트 시트: `image/play.png` 에 순백 바탕의 `note`·`long`·`held`·`hell`·`hell_held` 다섯 장(`*_pure`)을 더했다. 그림자는 회색 사다리로 섞고 화소는 모두 회색조다. 시트 크기 1024x570 은 그대로이고 기존 스프라이트 127개는 위치가 바뀌어도 화소가 하나도 다르지 않다(`tools/generate-images.py --check`). 기존에 색별로 구운 노트 묶음은 그대로 두어 저장값이 없을 때 쓴다.
- 판정선의 어두운 바닥 막대는 저장값이 있을 때 흰색 대신 회색 `theme.accent_dim.white`(102,102,102)를 바탕으로 곱한다. 강조색과 어두운 짝의 관계를 사용자 색에서도 유지하려는 것이다. 커버 가장자리 강조선과 커버 수치는 다른 행에 묶지 않았다. 부위 불투명도 오프셋은 바꾸지 않았다.

## 이유

- 노트까지 자유 색이 되는 유일한 안이고, 확장 키는 beatoraja 에서 무시되어 호환을 해치지 않는다(조사 8절 결정 1).
- 실시간이라 슬라이더를 끌 때 미리보기가 어두워지지 않는다(결정 5). 문서를 다시 읽는 것은 "있음" 여부가 바뀔 때 한 번뿐이다.
- 프리셋 옵션을 유지하고 색 행이 덮으므로 저장된 설정(`Note Colors` 등)과 `A` 키 동작이 그대로 호환된다(결정 8). 0 키가 프리셋으로 되돌린다.
- 흰 바탕 x 곱셈은 키빔·폭발·펄스·`block`·`bar_*`·숫자 시트가 이미 순백이어서 어떤 RGB 도 정확히 나온다(조사 3절 실측). 노트 시트를 순백으로 다시 생성하면 같은 규칙으로 노트까지 칠한다(결정 10).
- 저장값이 없을 때 화소가 같으므로 기본 사용자와 기존 캡처에 변화가 없다.

## 기각된 대안

조사 문서 6절의 안이다.

| 대안 | 기각 이유 |
| --- | --- |
| A: 기본 스킨 전용, Lua 가 본체 패스에서 `skin_config.color` 로 적용하고 값이 바뀌면 문서를 다시 읽음 | 슬라이더를 끌 때마다 문서를 다시 읽어 미리보기가 어두워지고 노트를 칠할 수 없다 |
| B: 엔진 수준 덮어쓰기(설정의 색을 엔진이 노트에 칠함) | 외부 스킨의 그림을 망가뜨리고 레인 종류 판정을 엔진이 해야 하며 스킨별 값을 가질 수 없다 |
| D: 순백 가산 3겹의 알파를 불투명도 행으로 받음 | 색마다 행 3개, 16진 입력 불가, 노트 불가, 겹치면 합산된다 |
| E: 프리셋 확대 | 자유 색이 없고 시트 스프라이트가 늘어난다 |
| F: 파일 변종(filepath) | 기본 스킨은 자산 변종을 두지 않기로 했고 색마다 PNG 가 필요하며 16진 입력이 없다 |
| 노트 행 하나로 LN 까지 칠하기(처음 구현) | 엔진의 레인당 틴트 하나 제약 때문이었다. LN 행을 따로 두라는 요구로 `long_tint` 를 더했다 |
| `[u8; 4]` 저장 | 위 3) 의 이유로 `SkinColor` 구조체 |
| 틴트 RGB 교체(replace) | 흰 바탕 x 곱셈이 같은 결과를 내고 알파를 같은 식으로 다룰 수 있다. 교체가 필요하면 `tinted` 호출 한 곳을 바꾼다 |

## 영향과 관련 파일

- 엔진: `crates/rbms-skin/src/{dst.rs,model.rs,model/objects.rs,loader.rs,loader/track.rs,loader/lua_skin.rs,lua/env.rs,lua/mod.rs}`, `crates/rbms-render/src/skin_render/{notes.rs,object.rs}`(`self_placed_track` 이 `tint` 를 보존), `crates/rbms-config/src/schema.rs`.
- 앱: `apps/rbms-player/src/{skin_select.rs,skin_select/{adjustments,number_row,color_tests,color_fixtures}.rs,skin_screen.rs,skin_host/mod.rs}`, `stage/{skin_menu,play_menu,preview_menu}.rs`.
- 기본 스킨: `assets/skins/rbms-default/lua/play/{colors,main,field}.lua`, `tools/sections/play.py`, `image/play.png`, `lua/atlas/play.lua`.
- 테스트: `crates/rbms-skin/src/dst/tests.rs`(`the_tints_alpha_is_a_share_of_the_alpha_the_offsets_leave`), `rbms-render` 의 노트 틴트 테스트(`a_colours_alpha_and_an_opacity_offset_meet_on_a_note_as_they_do_on_a_part`), `skin_select/default_pack_tests.rs`(`an_unset_colour_row_of_the_default_pack_starts_from_the_colour_its_options_draw`), 캡처 `stage/capture/play_rows/colours.rs`, `skin-dump --strict` 게이트.
- 사용자 확인이 필요한 것(미확인):
  - 9키 `Odd Buttons`·`Even Buttons` 가 7키 `White Key Lane`·`Black Key Lane` 과 같은 이름이라 `A` 키로 7키의 흰 건반 면 색이 9키 홀수 버튼 면에 적용된다. 의도에 맞는지.
  - 판정선 바닥 막대가 회색 102 를 바탕으로 곱해지는 것, HCN 누르는 중 하이라이트가 사라지는 것.
  - 알파 순서를 바꾼 결과 저장된 비불투명 색과 100% 가 아닌 불투명도를 함께 쓴 부위는 이전 구현보다 더 불투명하게 나온다.
- 남은 위험:
  - `drawn` 은 로드 시점의 스냅샷이다. 이미 저장된 노트 행의 색을 바꿔도 문서를 다시 읽지 않으므로 그 LN 행의 견본·채널 출발값은 다음 로드까지 이전 노트 색을 보인다(그리기는 프레임마다 틴트를 읽어 영향이 없다). 옵션을 바꾼 직후 아직 다시 읽지 않았으면 이전 옵션의 `drawn` 이 잠깐 보인다.
  - Bomb Color 가 Note(기본)이면 폭발이 레인 색을 따라 단일 색이 없어 `drawn` 은 흰색이다. 폭발 알파만 바꿔도 모든 레인 폭발이 흰색이 된다.
  - 실제 GPU 창에서 SKIN·PLAY 메뉴의 행과 실시간 반영을 보지 않았다. 헤드리스 캡처와 단위·문서 수준 단언만 했다. 외부 스킨을 고르는 선택 테스트(`RBMS_SKIN_PACK`)는 환경 변수가 없어 실행하지 못했다.
  - `settings.ron` 이 문서 절대 경로를 키로 쓰므로 색도 기기 간에 옮겨지지 않는다(내보낸 조절값 파일로는 옮긴다).
  - 노트 틴트의 화소 검증은 샘플 차트 노트 위치를 특정하기 어려워 패널 화소로만 했고, 노트 경로는 `rbms-render` 단위 테스트가 덮는다.
- 문서: `docs/skin.md`(헤더 확장 키 `color`·`tint`, `skin_config.color`, 팩 파일 수)와 조사 문서 4.4·5.1~5.3·6.3절은 구현 뒤의 값으로 고쳐야 한다.

## 날짜와 사용자 원문

2026-10-22.

- 지시 요지(`docs/PROCESS.md` 의 기록, 원문 아님): "(4) 레인·노트 색상과 불투명도를 각각 조절, 스킨 세부 조절 확대".
- 설계 확정 지시: "끝까지 다 구현해놔"
