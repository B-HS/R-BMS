# ADR: LAYOUT PICK 의 미리보기는 실시간으로 흐르고, 결과 화면의 `P` 로 저장한 배치를 LAYOUT PICK 에서 골라 쳐 본다

> 최종 갱신 2026-11-19 · 대응 커밋 `37e8ddd`
> 상태: 유효(코드는 `dev` 에 있음. 사용자가 정한 것은 요구 두 줄뿐이고 M13~M15 와 "구현에서 정한 것"은 메인·구현 단위의 결정이며 사용자 확인 전. 실제 앱 창으로 화면을 보지 않았다) · 결정일 2026-11-19 · 결정자 사용자(요구)와 메인(M13~M15, 기본 키 `P` 정정), 구현 단위 U7(구현에서 정한 것) · 커밋 `37e8ddd`
> 관계: [LAYOUT PICK ADR](2026-10-29-adr-layout-pick.md)(2026-10-29)의 "4) 화면과 시작" 중 **미리보기** 항목(네 마디 정지 화면, `PREVIEW_HISPEED` 0.25)을 이 ADR 이 대체한다. 그 ADR 의 문자열 규칙·시드 역탐색·RANDOM + 시드 기록·시작 거절 규칙·기억은 그대로다. 배치를 빌려 판을 시작하는 길은 [배치 플레이 ADR](2026-10-24-adr-layout-play.md)의 `borrow_layout` 을 그대로 쓴다. 키 설정에서 단축키가 레인 키와 같은 키가 되는 규칙은 [조합과 ESC ADR](2026-11-19-adr-chord-bindings-and-escape.md). 계획 U7, 조사 8·9절, 보고 [U7.md](../history/2026-11-19-feature-upgrade-unit-reports/U7.md).

## 배경

- 사용자 요청 요지(2026-11-19): 8 LAYOUT PICK 에서 실제 재생 미리보기, 9 결과 화면에서 capture 하고 저장한 배치로 플레이(LAYOUT PICK 에 통합해도 됨).
- 미리보기는 내장 플레이필드(`render_playfield_view`)의 정지 화면이었다: `PREVIEW_HISPEED = 0.25`(네 마디가 보임)로 고정, Page Up/Down 으로 `MEASURES_PER_PAGE = 4` 씩 이동. 재생·사용자 속도 반영은 없었다.
- 플레이 스킨으로 표본 차트를 돌리는 미리보기(`stage/play/skin_preview.rs` 의 `PlaySample`, PLAY 메뉴·SKIN 메뉴가 쓴다, 소리 없음)가 따로 있다. 차트를 소리와 함께 자동 재생하는 코드는 선곡 미리듣기(`stage/select/preview.rs`)뿐이다.
- 한 판의 배치는 `(배치 옵션 이름, 시드)` 로 다시 만들 수 있다(`LayoutRequest { md5, random_label, seed, source }`, `AppShared::borrow_layout`). 레인 순서 표기 함수는 `rbms_chart::shuffle::{layout_of_permutation, layout_row_text, permutation_of_layout}`. 설정 폴더의 작은 파일 저장 선례는 `favorites.rs`(`write_atomic`).

## 결정

### 1) 사용자가 정한 것

| 번호 | 결정 |
| --- | --- |
| D1 | LAYOUT PICK 의 미리보기를 실제 재생으로 한다 |
| D2 | 결과 화면에서 배치를 capture 해 저장하고, 저장한 배치로 플레이할 수 있게 한다. LAYOUT PICK 과 통합해도 된다 |

### 2) 메인 결정(사용자 미확인)

| 번호 | 결정 |
| --- | --- |
| M13 | LAYOUT PICK 미리보기가 실제 시간으로 흐른다: 사용자의 HI-SPEED·GREEN·COVER·LIFT 반영, Space 재생/정지, Page Up/Down 탐색, Home 처음으로, 끝나면 처음부터. 소리는 없다. 소리 포함은 선곡 미리듣기 내부를 뜯어야 해 위험이 커서 후속 후보로 기록한다 |
| M14 | capture 를 LAYOUT PICK 에 통합한다. 결과 화면 단축키(바꿀 수 있음)로 저장하고 LAYOUT PICK 의 CAPTURED 목록에서 골라 바로 플레이한다. 저장 대상은 레인 순열로 표현되는 배치다. 별도 모달은 미리보기·시작 흐름이 두 벌이 되므로 쓰지 않는다 |
| M15 | 저장 파일은 설정 폴더의 `captured_layouts.ron` 이다. 항목: md5, 제목, 모드, 배치 옵션 이름, 시드, 레인 순서 표기, 저장 시각, 그 판의 EX·램프. 점수 DB 표는 스키마 변경이 필요하고 기록과 무관한 데이터라 쓰지 않는다 |

### 3) 기본 키가 `C` 에서 `P` 로 바뀐 이유(정정)

M14 의 처음 계획은 `C` 였다. 계획 검토 5항은 "결과 화면에서 `C` 를 읽는 코드는 없다"고 적었으나 틀린 검토였다. 결과 화면은 모든 레인 키를 기준 구현의 키 표대로 읽는다. 기본 `C` 는 모든 기본 레인 배열에서 5번째 레인 키이고 결과 화면에서 5번째 키는 "새 배치로 다시 플레이"(확정·페이드)다. 기존 시험의 상수 `AFRESH_KEY: KeyCode = KeyCode::KeyC`(`stage/result/tests.rs`)가 바로 그 동작을 고정하고 있었다. `C` 로 저장하면 캡처와 동시에 결과 화면을 떠난다. 그래서 구현 단위가 기본 키를 `P`(`KeyCode::KeyP`)로 바꿨다. `P` 는 기존 LAYOUT PICK(`Y`)·`ReplayPlayLayout`(`J`)과 같은 규칙을 지킨다: 어느 모드의 기본 레인 키도 아니고 어떤 조작도 읽지 않으며 결과 화면의 다른 단축키와 겹치지 않는다. 이 규칙은 시험(`the_capture_key_ships_on_a_free_key_and_an_earlier_file_reads_as_it_did`)으로 고정했다. `C` 로 되돌리려면 `keyconfig.rs` 한 줄과 시험·스킨 문구를 함께 고치면 된다. 이 정정은 계획서 6절 5항에도 적혀 있다.

### 4) 구현에서 정한 것(구현 단위가 정함, 사용자 미확인)

#### 미리보기

| 항목 | 내용 |
| --- | --- |
| 방식 | 내장 플레이필드를 실시간으로 흘린다. `PlaySample` 을 임의 차트로 일반화하는 안은 버렸다(코드를 읽고 판단했고 줄 수를 재지는 않았다): (1) `PlaySample` 은 플레이 스킨 화면 전체(게이지 등 포함)를 사각형 안에 축소해 그려 레인이 사각형 너비의 약 1/4 이 되어 레인 아래 번호를 쓸 수 없다. (2) 시계가 앱 전체가 함께 쓰는 스킨 타이머 표(`shared.skin_timers` 의 PLAY 타이머)와 씬 상태에 묶여 일시정지·탐색이 없고 넣으려면 PLAY 메뉴·SKIN 메뉴 미리보기 경로를 고쳐야 한다. (3) 미리 적힌 손이 쳐서 판정이 나오는데 이 화면에 필요 없는 정보다 |
| 시계 | `stage/layout_pick/clock.rs` 의 `PreviewClock { at_us, playing }`. 프레임 dt 로 시간을 센다(dt 0 이면 멈춰 있어 시험에서 시계를 세울 수 있음). 차트를 읽으면 재생 중으로 시작하고, 마지막 타임라인 + 1 초(`END_TAIL_US` = 1,000,000 µs)를 지나면 0 으로 돌아가 처음부터 다시 흐른다. 배치를 고쳐 써도 흐르던 위치를 유지한다. `PREVIEW_HISPEED` 는 지웠다 |
| 속도 | 사용자의 HI-SPEED. 초록 숫자 고정(KEEP GREEN NUMBER)이 켜져 있으면 `BpmStats::held` + `held_speed` 로 계산한다(`PlaySample::shown_hispeed` 와 같은 식). 커버·히든은 `render_lane_cover`, 리프트는 `effective_lift`, CONSTANT 도 반영한다. 판정 오프셋(JUDGE OFFSET)은 미리보기 위치에 반영하지 않는다(소리가 없어 의미가 없다고 봄) |
| 조작 | `Space` 재생/정지, `PageUp`/`PageDown` 네 마디(`MEASURES_PER_PAGE`), `Home` 처음, `End` 마지막 마디, `↑`/`↓`·턴테이블·휠 한 마디. 멈춰 있으면 마디 표시 앞에 `STOPPED` 를 쓴다. 안내 줄을 고쳤다(세 언어) |

#### 저장소 `captured_layouts.rs`

| 항목 | 내용 |
| --- | --- |
| 파일 | 설정 폴더(`scores.ron` 옆)의 `captured_layouts.ron`(`CAPTURED_LAYOUTS_FILE`), `#[serde(default)]`, `write_atomic`. 읽지 못하면 알리고 빈 목록으로 시작한다(`NotLoaded`) |
| 항목 | `CapturedLayout { md5, title(제목 + 부제), mode(`Mode::name`), random_label(`NoteOption::label`), seed, layout(사이드별 레인 순서, `layout_row_text`), captured_at(ms), ex_score, lamp(`lamp_label_color`, 어시스트 등급 반영) }` |
| 중복 | 같은 (md5, 배치 옵션, 시드)는 md5·옵션 이름의 대소문자를 무시하고 한 번만 저장한다. 새 항목을 맨 앞에 둔다(최신순) |
| 저장 대상 | 레인 순열인 RANDOM·R-RANDOM·MIRROR·**ROTATE**(`captures_lanes`). ROTATE 는 계획의 셋에 없었으나 레인 순열이라 넣었다. OFF 는 저장할 배치가 없어 거절한다 |
| 거절 사유 | S-RANDOM·H-RANDOM·ALL-SCRATCH 같은 노트별 배치(`NoteByNote`), OFF, AUTOPLAY, 리플레이 보기, 코스·연습, LAYOUT PICK 이 지원하지 않는 모드, 판 없음. 성공·중복·거절 알림은 모두 세 언어이고 `notices.rs` 가 아니라 `text/menus/layout_pick.rs` 의 `CaptureText` 카탈로그에 둔다(`notify_text` 가 아무 카탈로그나 받음) |

#### 결과 화면 capture

| 항목 | 내용 |
| --- | --- |
| 단축키 | `Shortcut::CaptureLayout`(설정 키 `result_capture_layout`, 결과 그룹, 패드 가능, 기본 `P`). `Shortcut::ALL` 이 34개가 됐고 키 편집기·키 가이드에는 자동으로 나온다 |
| 값의 출처 | 캡처 정보는 판이 끝날 때 `ResultRun::of` 에서 실제 값(`config.play.random`(빌린 배치 포함), `session.seed()`)으로 만들어 두고, 키를 누른 시각을 저장 시각으로 쓴다 |
| 동작 | 내장 결과 화면·스킨 씬·스킨을 읽는 중 모두에서 바로 동작하고 씬을 건드리지 않는다(페이드가 시작되지 않는다) |
| 안내 | 스킨 결과 바닥 안내(`lua/i18n/result.lua` 의 `result_hint_chart`)를 "RETRY, NEXT CHART AND CAPTURE LAYOUT … (R, N AND P AS SHIPPED)" 로 바꿨다(기존 안내가 키 이름 대신 기본 키를 정적으로 적는 방식이라 그대로 따랐다). 내장(스킨 없는) 결과 화면의 안내 줄(`crates/rbms-render/src/text/result.rs` 의 `HintSelectRetryNext`)에는 `P` 가 없다(소유 밖이고 골든 시험이 문구를 고정함) |

#### LAYOUT PICK 의 CAPTURED 목록

| 항목 | 내용 |
| --- | --- |
| 자리 | 필드 오른쪽(`LIST_X` 928 부터, 위는 레인 위쪽 `LIST_TOP` 184). 기본은 이 차트의 저장분이고 `Tab`(목록에서)으로 전체를 본다. 한 번에 열 줄(`VISIBLE_ROWS`) |
| 포커스 이동 | 키보드 `→`/`←` 로 입력란과 목록 사이를 오간다. SELECT(키보드·패드)는 1P → 2P → 목록 → 1P 로 돈다. 목록에 있을 때 레인 키(패드 포함)는 행을 쓰지 않는다: 턴테이블과 KEY 1·2 는 커서 이동, KEY 3 은 범위 전환, KEY 4 두 번은 삭제 |
| 고르기 | 커서가 이 차트 항목에 오면 행 입력란과 미리보기가 그 배치가 되고 시드 탐색 없이 `SeedLook::Captured` 가 된다. 목록에 들어가면 커서의 첫 저장분이 바로 행에 쓰여 그 전에 직접 입력하던 행은 사라진다("고르면 행도 그 배치로") |
| 시작 | `Enter`/START 는 저장된 옵션 이름과 시드로 `borrow_layout` 을 부른다. 알림 출처는 저장 시각(`fmt_datetime`)이다. 행을 다르게 고쳐 쓰면 다시 RANDOM 탐색으로 돌아가고 같은 배치로 다시 쓰면 다시 저장분 조건으로 돌아온다 |
| 삭제 | `Delete` 두 번. 첫 누름에 확인 문구가 나오고 다른 키를 누르면 취소된다 |
| 다른 차트 | 다른 차트의 항목에서 `Enter` 를 누르면 라이브러리에서 md5 로 찾아 그 차트로 옮겨 가고 없으면 알린다. 커서를 움직일 때마다 차트를 다시 읽지 않으려고 커서만으로는 옮겨 가지 않는다 |

## 이유

- 실시간 미리보기는 사용자의 속도·커버로 그려야 "이 배치가 내 설정으로 어떻게 내려오는지"를 볼 수 있다. 시계를 화면이 직접 세어야 정지·탐색·끝에서 처음으로 가는 동작과 시험이 가능하다.
- capture 를 LAYOUT PICK 에 통합하면 미리보기·시작 흐름이 한 벌이다. 저장 항목은 `LayoutRequest` 가 요구하는 값(md5, 옵션 이름, 시드)이라 저장분으로 시작해도 기록이 RANDOM + 시드로 남는다.
- 기본 키가 레인 키와 겹치지 않아야 캡처가 화면 전환을 일으키지 않는다.

## 기각된 대안

| 대안 | 기각 이유 |
| --- | --- |
| `PlaySample` 을 임의 차트로 일반화 | 위 "방식"의 세 가지 이유. 일반화하면 PLAY 메뉴·SKIN 메뉴 미리보기를 건드려야 한다 |
| 미리보기에 소리 포함 | 선곡 미리듣기 내부를 뜯어야 해 위험이 크다. 후속 후보 |
| 별도 capture 모달 | 미리보기·시작 흐름이 두 벌이 된다 |
| 점수 DB 에 저장 | 스키마 변경이 필요하고 기록과 무관한 데이터다 |
| 결과 화면 기본 키 `C` | 5번째 레인 키(새 배치로 다시 플레이)라서 캡처와 동시에 화면을 떠난다 |
| 다른 차트의 항목을 커서만으로 이동 | 커서를 움직일 때마다 차트를 다시 읽는다 |

## 영향과 관련 파일

- 코드: `apps/rbms-player/src/captured_layouts.rs`·`captured_layouts/tests.rs`(새), `stage/layout_pick.rs`·`layout_pick/{clock,captured,tests}.rs`, `stage/result.rs`·`result/tests.rs`, `text/menus/layout_pick.rs`, `keyconfig.rs`·`keyconfig_tests.rs`, `lib.rs`(`mod captured_layouts;`, `AppShared` 필드와 초기화), `stage/select/{mod,search}.rs`(`Shortcut` 전체 match 에 `| Shortcut::CaptureLayout` 한 줄씩), 기본 스킨 `lua/i18n/result.lua`.
- 시험: `captured_layouts/tests.rs` 5개(`a_capture_writes_the_lane_order_the_run_was_laid_out_in`, `a_run_that_is_no_lane_order_of_the_players_is_not_captured`, `a_capture_is_kept_once_newest_first_and_survives_its_file`, `a_capture_is_removed_from_its_file`, `a_file_that_is_missing_or_broken_is_no_captures`), `layout_pick/tests.rs`(시계 `the_preview_plays_on_the_screens_clock_and_begins_again_at_the_end`, 저장분 표시·시작 `a_captured_layout_is_shown_and_played_on_its_own_arrangement_and_seed`, 고쳐 쓰기, 삭제, 다른 차트, 패드), 결과 화면 캡처 end-to-end, 키 설정 `the_capture_key_ships_on_a_free_key_and_an_earlier_file_reads_as_it_did`.
- 알려진 위험(테스트 부채 3.178·3.179, 실기 QA 3.29 G10·G11):
  - 캡처 키를 사용자가 레인 키와 같은 키로 다시 지정하면 결과 화면이 그 레인의 키 표 역할(확정·다시 플레이)도 함께 실행한다. 기존 충돌 규칙(`Bindable::read_with`)이 단축키와 레인을 같이 읽히는 것으로 보지 않아 거절하지 않는다. R·N 단축키도 같은 처지다.
  - 내장 결과 화면의 안내 줄에 `P` 가 없다.
  - 미리보기는 사용자 HI-SPEED 를 그대로 쓰므로 속도가 낮은 사용자에게는 예전(네 마디 고정)보다 한 화면에 보이는 마디가 적다(요구된 동작).
  - 실제 창에서 세 언어 문구의 길이와 겹침을 보지 않았다. 한·일 문구는 `HeadlessCanvas` 로 그리지 않았고 목록 문구는 `fit_text` 로 잘리게 해 두었다. 글자를 쓰는 test 하니스로 영어 화면을 PNG 로 저장해 확인했다.
  - 화면에 들어온 뒤 커버·리프트 값을 바꿀 방법이 없다(옵션 오버레이가 이 화면에서 열리지 않음). 값은 들어올 때와 그릴 때 설정값을 읽는다.
- 문서: [play-options.md](../play-options.md) 10.16절(미리보기 줄을 고침)·23절, [crates.md](../crates.md) `Shortcut` 34개·`captured_layouts.rs`.

## 날짜와 사용자 원문

2026-11-19.

- 사용자 요청 요지(PROCESS 체크리스트): "8 Layout pick 실제 재생 미리보기, 9 결과 화면 capture 와 저장 배치 플레이(Layout pick 통합 가능)". 원문 전문은 이 저장소에 없다.

## 후속(2026-11-19 테스트·수리 웨이브, 커밋 `a23074f`·`733ba62`·`864ef42`)

- 목록에서 레인 키(KEY 4) 두 번으로 삭제가 되게 고쳤다. MIRROR·ROTATE·R-RANDOM 처럼 시드가 달라도 같은 배치가 되는 경우를 한 번만 저장한다. 읽지 못한 저장 파일(UTF-8 아님, 권한 오류)은 알리고 덮어쓰지 않는다. 쓰기에 실패하면 실패 알림만 낸다. capture 단축키를 해제해 둔 경우 빈 목록 안내가 키 설정에서 지정하라고 말한다.
- 내장(스킨 없는) 결과 화면의 안내 줄에도 capture 키가 나온다.
- 관련 수리: 빌린 배치(저장한 배치 포함)로 친 판을 START 로 재시도하면 그 판이 사용자 배치 옵션으로 기록되던 이전부터의 결함을 고쳤다([버그 기록](../bug/2026-11-19-borrowed-layout-retry-recorded-with-own-option.md)).
