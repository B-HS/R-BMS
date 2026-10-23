# ADR: SCRATCH AUTO 노트 색은 PLAY 전역 RGBA 한 값 (알파 = 불투명도)

> 최종 갱신 2026-10-23 · 대응 커밋 5a0eacf
> 상태: 유효(결정 8 은 사용자 결정, 세부 M8-1~M8-8 은 메인 결정이라 사용자 미확인) · 결정일 2026-10-23 · 결정자 사용자(결정 8) + 메인(세부) · 커밋 `9e39262`(제목은 마이그레이션 0012 이지만 웨이브 1 의 클라이언트 변경 전체가 들어 있다), `db0aa7c`(설정 묶음 계약 재생성)
> 관계: [스킨 색 행 ADR](2026-10-22-adr-skin-colour-rows.md) 4절("SCRATCH AUTO 초록은 사용자 색보다 우선한다")을 이어 받는다. 그 절의 "스킨 색 행이 AUTO 노트에 닿지 않는다"는 그대로 유효하고, "초록이 고정"이라는 부분만 바뀐다. 아래 "관계" 절 참고.

## 배경

- 사용자 요구 1(아래 원문): SCRATCH AUTO 의 색과 opacity 도 조절할 수 있어야 한다.
- 그 전의 SCRATCH AUTO 노트는 렌더러 상수 `AUTO_LANE_NOTE`(`#30E050`, 알파 255)와 롱노트 몸통 `AUTO_LANE_LONG_BODY`(같은 초록, 알파 0xA0)로만 그려졌다. 스킨의 노트 그림을 쓰지 않고 그 그림이 차지할 사각형에 단색을 칠한다. 색 행의 틴트(Scratch Note id 12, Scratch LN id 42 등)는 닿지 않았다(틴트 `None`). 스킨 없는 내장 필드(`PlayfieldView`)도 같은 상수를 썼다.
- 불투명도는 AUTO 노트에도 이미 스킨의 노트 불투명도(destination 알파 + 오프셋, 기본 스킨의 Note Opacity 행)가 곱해졌다. AUTO 만 따로 조절하는 값은 없었다.

## 결정

1. **저장 형태.** `play.scratch_auto_color: Option<SkinColor>`(`PlayOptions`, `SCRATCH AUTO` 바로 뒤). `None` 이 출하 초록 `#30E050FF`다. 스키마 버전 6 은 그대로이고 `#[serde(default)]` 라 이 필드가 없는 옛 `settings.ron` 이 그대로 읽힌다.
2. **어디에 적용되나.** 어느 스킨에서든(외부 스킨 포함), 스킨 없는 내장 필드에서도 같은 값이다. 알파가 불투명도다. 노트 알파는 사용자 알파 × 그리기 알파(스킨의 노트 객체 알파 + 오프셋 + Note Opacity)이고, 롱노트 몸통은 `auto_long_body`(사용자 알파 × 0xA0/255)에 같은 그리기 알파를 곱한다. 머리·끝·HCN 의 머리·끝은 노트 색이다. 출하 초록에서는 몸통이 지금과 비트까지 같다(255 × 160 / 255 = 160).
3. **저장값이 없으면 화소가 지금과 같다.** 테스트가 고정한다: `the_shipped_scratch_auto_colours_are_the_ones_drawn_before`, `a_settings_file_without_a_scratch_auto_colour_holds_the_shipped_one`.
4. **PLAY 메뉴 CHART 묶음.** `SCRATCH AUTO` 행 바로 아래에 `SCRATCH AUTO COLOR` 행과 닫힌 채 시작하는 접이 `SCRATCH AUTO CHANNELS`(R·G·B·A, 각 0~255)를 둔다. 색 행은 ←/→ 로 16색 프리셋(알파 유지), Enter 로 16진 입력(6자리 또는 8자리), 0 키로 저장값 지움(`DEFAULT`)이다. 채널 행의 0 키는 그 채널만 출하 초록의 값으로 돌린다. 값은 엔진이 프레임마다 읽으므로 스킨 문서를 다시 읽지 않는다. 파일은 메뉴를 떠날 때와 끌기가 끝날 때 쓴다. 프리셋 한 칸 찾기는 SKIN 메뉴의 색 행 코드(`preset_after`)를 공통화해 같이 쓴다.
5. **계약.** 계정 설정 묶음은 RON 본문 전체를 문자열로 올리므로 `contract/client-requests/settings-blob.json` 이 `scratch_auto_color: None,` 한 줄만큼 바뀐다(재생성). 웹은 이 본문을 문자열로만 다뤄 변경이 없다. 옛 빌드가 새 묶음을 받으면 모르는 필드를 버린다(옛 빌드로 시험하지 않았다). 판정·리플레이·trace·점수 본문에는 싣지 않는다(표시 전용).

### 관계: 스킨 색 행 ADR 4절

- 4절은 "받지 않는 것(의미색)"에 SCRATCH AUTO 초록 노트를 넣고 "사용자 색보다 우선"이라 적었다. 이번 결정 뒤에도 스킨의 색 행(Scratch Note 12, Scratch LN 42 포함)과 `tint` 는 AUTO 노트에 닿지 않는다(M8-8). AUTO 노트가 따르는 것은 이 전용 값 하나다.
- 바뀐 것은 "초록이 상수"라는 부분이다. 초록은 이제 저장값이 없을 때의 기본이고, 사용자가 정하면 그 값이 어느 스킨에서나 이긴다. 우선순위는 전용 값 > 출하 초록이고, 스킨의 색 행·옵션과는 겹치지 않는다.
- 2026-10-22 ADR 파일은 고치지 않았다. 4절을 읽을 때 이 절을 함께 본다.

### 메인 결정(설계서 client-small 8절, 사용자 미확인)

| id | 결정 | 이유 |
| --- | --- | --- |
| M8-1 | 저장 필드는 `Option<SkinColor>` | 결정 8 의 "저장값 없으면 `#30E050`"을 "저장값 없음"으로 그대로 표현한다. 색 행의 `DEFAULT` 규칙과 같다 |
| M8-2 | 채널 행은 색 행 바로 뒤의 접이(닫힌 채 시작) | 기존 색 행 UI 와 같고 CHART 묶음이 닫힌 상태에서 두 행만 늘어난다 |
| M8-3 | 0 키: 색 행은 저장값 지움, 채널 행은 그 채널만 출하값 | 기존 색 행·채널 행의 0 키와 같다 |
| M8-4 | PLAY 메뉴 미리보기는 SCRATCH AUTO 를 그리지 않음 | 미리보기는 예시 손이 치는 판이고 턴테이블 손을 빼면 미리보기 체감이 바뀐다. 행의 색 견본 사각형으로 확인한다 |
| M8-5 | 설정 업로드 계약을 재생성 | 다른 설정 필드를 더할 때와 같은 방식이다. `rbms-config` 에 `skip_serializing_if` 선례가 없다 |
| M8-6 | 몸통 알파는 사용자 알파 × 0xA0/255 | 머리와 몸통 구분을 유지하고 출하 화소가 같다(조사 보고의 추천을 사용자가 "추천대로"로 받았다) |
| M8-7 | SCRATCH AUTO 가 꺼져 있어도 색 행은 늘 보임 | 켜기 전에 색을 정할 수 있고 행 목록이 설정에 따라 흔들리지 않는다 |
| M8-8 | 기본 스킨 Scratch Note 색 행(id 12/42)은 AUTO 노트에 닿지 않음 | 의미색 규칙 유지 |

## 이유

- 결정 8: 조사 보고의 결정 지점(전역 play 설정, RGBA 한 값에 알파 = 불투명도, 몸통은 0xA0 비율 유지, 저장값 없으면 `#30E050` 알파 255 로 화소 동일, PLAY 메뉴 CHART 의 SCRATCH AUTO 행 옆 + 16진 입력, Scratch Note 색 행은 적용하지 않음)에 사용자가 "1-11 추천대로"로 답했다. 위 결정 1~5 와 M8-6·M8-8 이 그 추천이다.
- 색이 PNG 가 아니라 엔진의 단색이라 스킨 쪽 색 행으로는 바꿀 수 없다. 전역 한 값이어야 외부 스킨과 내장 필드를 한꺼번에 덮는다.
- 알파를 불투명도로 쓰면 기존 색 행의 규칙(알파 = 그 부위에 주어진 불투명도의 몇 분의 몇)과 같아 새 개념이 없다.

## 기각된 대안

| 대안 | 기각 이유 |
| --- | --- |
| `SkinColor` 를 바로 저장(기본 `30E050FF`) | `DEFAULT` 표시와 0 키 "지움"을 구별하지 못한다 |
| 기본 스킨 색 행만 / 엔진 자동 색 행 | 외부 스킨과 내장 필드에 닿지 않는다 / 모든 play 스킨에 새 행이 붙는 큰 변경 |
| 색과 별도 불투명도 행 | 알파 한 값이 같은 뜻이고 UI 가 는다 |
| 몸통 알파를 사용자 알파 그대로, 또는 몸통 색 행 따로 | 머리와 몸통 구분이 사라지거나 행이 늘어난다 |
| 채널 네 행을 늘 보임 / 채널 없음 | CHART 묶음이 길어진다 / 정밀한 값을 못 정한다 |
| 미리보기에 SCRATCH AUTO 적용 | 미리보기 판정과 체감이 바뀐다(M8-4) |
| `skip_serializing_if` 로 계약 불변 | 설정 파일에 새 패턴을 들인다(M8-5) |
| 켜져 있을 때만 색 행을 보임 | 행 목록이 설정에 따라 흔들린다 |
| Scratch Note 색 행이 AUTO 노트를 따름 | 의미색(AUTO = 사람이 안 치는 노트) 규칙을 깬다 |

## 영향과 관련 파일

- 설정: `crates/rbms-config/src/schema.rs`(`PlayOptions.scratch_auto_color`), 계약 `contract/client-requests/settings-blob.json`.
- 그리기: `crates/rbms-render/src/skin_render/notes.rs`(`auto_long_body`, `auto_long_images`, `LaneNotes.auto_note`; 옛 `AUTO_NOTE_IMAGE`·`AUTO_LONG_IMAGES` 상수는 삭제), `crates/rbms-render/src/playfield.rs`(`PlayfieldView.auto_note`), `apps/rbms-player/src/app_play.rs`(`SCRATCH_AUTO_SHIPPED_COLOR`, `scratch_auto_color`), `stage/play/mod.rs`·`stage/play/skin_preview.rs`(값을 넘김).
- 메뉴: `apps/rbms-player/src/stage/play_menu.rs`·`play_menu/rows.rs`(`Row::AutoColor`, `Row::AutoChannel`, `Fold::AutoChannels`), `skin_select.rs`(`preset_after`, `ColorChannel`, `CHANNEL_STEPS`, `color_value` 공개).
- 테스트: `stage/play_menu/tests/scratch_auto_color.rs`, `skin_render/notes/tests.rs`(`a_long_body_keeps_its_share_of_the_alpha_it_is_given` 외), `playfield.rs`(`a_note_the_chart_plays_itself_on_the_built_in_field_takes_the_colour_it_is_given`), `stage/play/screen_tests/default_pack_tests.rs`(`the_turntable_scratch_auto_plays_is_drawn_in_the_colour_the_settings_hold`), `app_play.rs`(`scratch_auto_colour_is_the_shipped_green_until_one_is_set`).
- 확인하지 못한 것: 실제 GPU 창에서 본 색과 불투명도, 외부 스킨에서의 확인(`RBMS_SKIN_PACK` 이 있을 때만 도는 테스트라 CI 는 기본 팩만 본다. QA 3.17 X1 실기 항목), 옛 클라이언트 빌드가 새 설정 묶음을 받았을 때의 동작(serde 규칙에 기댄 판단이고 시험하지 않았다. 옛 빌드가 다시 올리면 색이 출하 초록으로 돌아간다).
- 설계서와 다른 점: 설정·그리기·메뉴의 변경을 `docs/plan/2026-10-23-client-small.md` 3.1절과 대조한 범위에서는 없다.

## 날짜와 사용자 원문

2026-10-23.

- 요구 1(원문): "스크래치 오토의 색상과 opacity 도 조절 가능해야 해"
- 결정 회신(원문): "1-11 추천대로". 결정 8 의 요지(`docs/PROCESS.md` 의 요약, 원문 아님): SCRATCH AUTO 노트 색 = PLAY 전역 RGBA 한 값(외부 스킨 포함, 알파 = 불투명도, 저장값 없으면 `#30E050`).
