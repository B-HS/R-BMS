# 플레이 옵션

> 최종 갱신 2026-10-12 · 웨이브 10D(판정 글자 뒤에 붙는 콤보. 작업 트리 기준, 커밋 전) 기준
> 대상: 플레이 중 스크롤·가림막·판정 타이밍을 정하는 옵션과 그 키, 그 옵션을 바꾸는 화면인 PLAY 메뉴(7절), 기본 스킨의 판정 표시(8절). 스킨이 그리는 옵션(Lane Width 등)과 부위별 불투명도는 [skin.md](./skin.md) 3.4절, 구현 위치는 [crates.md](./crates.md)를 본다. PLAY 메뉴의 설계는 [plan/2026-10-11-play-menu.md](./plan/2026-10-11-play-menu.md).
> beatoraja 원본의 같은 기능이 기준이다. 다르게 한 곳은 "원본과 다른 점" 열에 이유를 적는다.

## 1. 용어

| 용어 | 뜻 |
| --- | --- |
| 이동 시간(duration) | 노트가 레인 위쪽 끝(레인 커버가 켜져 있으면 커버 아래 끝)에서 나타나 판정선에 닿을 때까지의 시간. 단위 ms |
| 녹색 숫자(green number) | 이동 시간의 3/5 를 버림한 값(`duration * 3 / 5`). IIDX 의 녹색 숫자, beatoraja 의 `duration_green`(스킨 number 313)과 같다. 이동 시간 500 ms = 녹색 300 |
| 흰색 숫자(white number) | 레인 커버가 가린 비율의 1000 배를 반올림한 값(스킨 number 14). 커버 0.3 = 300. 리프트(314)와 HIDDEN+(315)의 숫자도 같은 방식이다 |
| 하이스피드(hispeed) | 스크롤 배율. `이동 시간 = 240000 / BPM / hispeed * (1 - 커버)` |
| 판정 오프셋(judge offset) | 입력이 판정되는 시각에 더하는 값. 단위 ms. 양수는 "빠르게 치는 사람"을 위한 보정이다(입력을 그만큼 늦게 친 것으로 본다) |

## 2. 옵션 표

| 옵션 | 뜻 | 단위와 범위 | 기본값 | 바꾸는 곳 | beatoraja·IIDX 대응 | 원본과 다른 점 |
| --- | --- | --- | --- | --- | --- | --- |
| HI-SPEED | 스크롤 배율 | 배율 0.01~20.00, 한 번에 HI-SPEED STEP(기본 0.25) | 2.00 | PLAY 메뉴 > SPEED, 플레이 중 `UP`/`DOWN`, START + 건반 | `PlayConfig.hispeed` | 없음 |
| HI-SPEED FIX | 녹색 숫자를 계산할 때 기준으로 삼는 BPM | OFF / START / MAX / MAIN / MIN | MAIN | PLAY 메뉴 > SPEED, 선곡 패널 1 의 5번 키 | `PlayConfig.fixhispeed` | KEEP GREEN NUMBER 가 꺼져 있으면 한 판 안에서만 이동 시간을 묶는다(곡 사이에는 HI-SPEED 가 그대로). 원본은 OFF 가 아니면 곡 사이에도 묶는다. 그 역할은 아래 KEEP GREEN NUMBER 가 맡는다 |
| KEEP GREEN NUMBER | 녹색 숫자를 곡과 곡 사이에 유지한다. 켜면 매 곡 시작과 매 프레임에 HI-SPEED 가 `240000 / 기준 BPM / 이동 시간 * (1 - 레인 커버)` 로 다시 계산된다 | 켬 / 끔 | 끔 | PLAY 메뉴 > SPEED, 플레이 중 KEEP GREEN NUMBER ON/OFF 키(출하 시 미지정) | `fixhispeed != OFF` 일 때의 `LaneRenderer.resetHispeed`, IIDX 의 FHS | 별도 스위치로 두었다. 기존 설정 파일은 끔으로 읽혀 이전과 같은 속도로 스크롤된다. HI-SPEED FIX 가 OFF 이면 기준 BPM 은 MAIN, CONSTANT 이면 120 이다 |
| GREEN NUMBER | KEEP GREEN NUMBER 가 유지하는 목표 녹색 숫자 | 1~6000(이동 시간 1~10000 ms 로 저장), PLAY 메뉴의 행은 1씩(Shift 10) | 300(이동 시간 500 ms) | PLAY 메뉴 > SPEED, 플레이 중 하이스피드 키(±10, 미세 단계 ±1), SELECT + 건반 또는 GREEN NUMBER UP/DOWN 키(이동 시간 ±1 ms), 선곡 패널 3 의 4·6번 키(이동 시간 ±1 ms, 누르고 있으면 반복) | `PlayConfig.duration` | 원본의 하이스피드 키는 유지 모드에서 배율만 바꾸고 저장된 이동 시간은 그대로 둔다(다음 커버 이동이나 다음 곡에서 되돌아간다). 여기서는 키를 누른 것이 새 목표를 고른 것이므로 목표가 바뀌어 남는다 |
| LANE COVER | 위에서 내려오는 가림막. 값이 커질수록 더 가린다 | 비율 0~1, 한 번에 0.01, 미세 단계 0.0001~0.01(기본 0.001) | 0, 켬 | PLAY 메뉴 > LANE, 플레이 중 `RIGHT`(더 가림)/`LEFT`(덜 가림), `BACKSLASH`(켬·끔), START + 턴테이블, START 두 번(켬·끔), 마우스 휠 | `PlayConfig.lanecover`, `enablelanecover`, IIDX 의 SUDDEN+ | 켬·끔 전용 키와 방향을 적은 행 이름을 더했다 |
| LIFT | 판정선을 올린다 | 비율 0~1 | 0, 끔 | PLAY 메뉴 > LANE, 플레이 중 `RBRACKET`/`LBRACKET`, 켬·끔 키(출하 시 미지정) | `PlayConfig.lift`, IIDX 의 LIFT | 없음 |
| HIDDEN+ | 판정선에서 위로 올라오는 가림막 | 비율 0~1 | 0, 끔 | PLAY 메뉴 > LANE, 플레이 중 올림·내림·켬·끔 키(출하 시 미지정) | `PlayConfig.hidden`, IIDX 의 HIDDEN+ | 없음 |
| JUDGE OFFSET | 판정 오프셋 | -200~+200 ms, PLAY 메뉴의 행은 1 ms 씩(Shift 10 ms) | 0 | PLAY 메뉴 > JUDGE, 선곡 패널 3 의 5·7번 키(±1 ms), 플레이 중 JUDGE OFFSET ±1 MS 키(출하 시 미지정) | `PlayerConfig.judgetiming` | 원본은 값을 노트 표시 시계에 더한다(빠르게 치면 노트를 늦게 그린다). 여기서는 입력의 판정 시각에 더하므로 부호가 반대다: 빠르게 치는 사람은 + |
| AUTO ADJUST | 칠 때마다 JUDGE OFFSET 을 조금씩 움직인다 | 켬 / 끔 | 끔 | PLAY 메뉴 > JUDGE, 선곡 패널 3 의 3번 키, 플레이 중 AUTO ADJUST ON/OFF 키(출하 시 미지정) | `PlayerConfig.notesDisplayTimingAutoAdjust`(`JudgeManager.java:697-703`) | 부호만 위와 같은 이유로 반대다 |

## 3. 판정 자동 보정(AUTO ADJUST)

1. 대상은 손으로 치는 판(일반 플레이와 연습)뿐이다. 오토플레이와 리플레이에서는 오프셋이 움직이지 않는다.
2. GOOD 이상의 판정이고 오차가 ±150 ms 이내인 입력마다 오프셋이 `(오차 ± 15 ms) / 30 ms` 를 0 쪽으로 버림한 만큼 움직인다. 오차가 ±15 ms 안이면 움직이지 않는다. 40 ms 빠르게 계속 치면 한 번에 +1 ms 씩 올라 +26 ms 에서 멈춘다(남는 오차 14 ms).
3. 움직인 값은 그 입력의 다음 입력부터 적용된다. 범위 끝(±200 ms)에서 멈춘다.
4. 값은 입력 직후 설정에 반영되고, 클리어·폐점·중간에 나가기 어느 쪽으로 끝나도 저장된다.
5. 플레이 화면의 TIMING 숫자(스킨 number 12)가 현재 오프셋이다. 기본 스킨은 켜져 있는 동안 표지를 `TIMING AUTO` 로 쓴다(스킨 image index 75).
6. 리플레이는 오프셋이 바뀐 이력을 `judge.offset_steps`(몇 번째 입력부터 몇 ms)에 기록해 같은 판정으로 재생·탐색된다. 이 필드가 없는 이전 리플레이는 한 오프셋으로 판정된다(이전과 같다).
7. IR 에 올리는 리플레이는 오프셋 하나만 실을 수 있어, 각 입력의 시각을 `(그 입력의 오프셋 - 마지막 오프셋)` 만큼 옮겨 올린다. 받는 쪽이 마지막 오프셋 하나로 판정해도 같은 시각에 판정된다. 한계: 놓친 노트를 쓸어 내는 시계는 오프셋의 영향을 받지 않으므로, 마지막 판정 창 끝에 걸친 입력(창 끝에서 오프셋이 움직인 폭 이내)은 다르게 판정될 수 있다.

## 4. 녹색 숫자 유지(KEEP GREEN NUMBER)

1. 켜면 저장되는 값은 하이스피드가 아니라 이동 시간(`play.duration_ms`)이다. 하이스피드는 곡마다 `240000 / 기준 BPM / 이동 시간 * (1 - 레인 커버)` 로 정해진다. 레인 커버가 꺼져 있으면 커버는 0 으로 본다.
2. 기준 BPM 은 HI-SPEED FIX 가 가리키는 BPM(START / MAX / MAIN / MIN)이고, OFF 이면 MAIN(노트가 가장 많은 BPM)이다. CONSTANT 스크롤에서는 필드의 고정 템포 120 이다.
3. 플레이 중 레인 커버를 움직이거나 켜고 끄면 녹색 숫자는 그대로이고 하이스피드가 바뀐다.
4. 리프트와 HIDDEN+ 는 계산에 들어가지 않는다. 스크롤이 리프트로 줄어든 레인 높이에 맞춰지므로 리프트를 바꿔도 이동 시간이 그대로이고, HIDDEN+ 는 판정선 쪽을 가릴 뿐 노트가 나타나는 시각을 바꾸지 않는다(원본의 `currentduration` 도 커버만 반영한다). 실측: 커버 0.3·리프트 0.15 에서 150·200·100 BPM 모두 499.0 ms.
5. 하이스피드가 범위(0.01~20.00)를 넘어야 하는 목표이면 배율은 범위 끝에 머물고, 화면의 녹색 숫자는 실제 값을 보여 준다.
6. 꺼져 있으면 저장된 HI-SPEED 로 스크롤한다(이전과 같다). 이때 선곡 패널 3 의 NOTE DISPLAY TIME 칸은 `OFF` 로 표시되고 4·6번 키는 아무것도 바꾸지 않는다.
7. 속도를 잠그는 코스에서는 유지하지 않는다. 리플레이는 스크롤 설정을 기록하지 않으므로 보는 사람의 설정으로 스크롤한다.

## 5. 키

| 항목 | 내용 |
| --- | --- |
| 키 설정 화면을 여는 길 | 선곡에서 `F5`, Settings 의 `KEY CONFIG >` 단추 또는 `F5`, 스킨 브라우저의 숫자 `6` |
| 사용자 지정 범위 | 레인과 스크래치 역방향, 플레이 조작 20종, 선곡 단축키 9종, 화면 여는 키 6종(PLAY 메뉴 `F8` 포함), 스킨 브라우저 숫자 키 8종, 결과 2종, 리플레이 분석 5종. 키보드와 컨트롤러 둘 다(옵션 오버레이 두 키와 상세 패널 홀드 키는 키보드 전용) |
| 고정 키 | `Esc`, `Enter`, 화살표, `H`(키 안내)와 각 메뉴 안의 키 |
| 레인 커버 조작 | `RIGHT` = 더 가림(LANE COVER DOWN), `LEFT` = 덜 가림(LANE COVER UP), `BACKSLASH` = 켬·끔. 커버가 꺼져 있을 때 값을 바꾸면 발치의 COVER 수치가 1.5초 동안 뜬다 |
| 미세 단계 | FINE STEP 키를 누르고 있는 동안 가림막은 미세 단계로, 유지 중인 녹색 숫자는 ±1 로 움직인다. 키를 주지 않았으면 레인으로 쓰지 않는 Shift 가 그 역할을 한다(7키 기본 배치에서는 오른쪽 Shift) |
| 지금 설정된 키 보기 | 선곡에서 `H` |
| 저장 | `~/.config/rbms/keyconfig.ron`. 이전 파일은 그대로 읽히고 없는 항목은 기본 키로 채워진다. 단, 파일에 없는 단축키의 기본 키를 같은 화면의 다른 단축키가 파일에서 이미 쓰고 있으면 그 단축키는 미지정으로 읽힌다(사용자가 고른 키가 이긴다. 예: FOLDERS 를 `F8` 로 옮겨 둔 파일에서 PLAY MENU 는 키 없음으로 나오고 키 설정 화면에서 새로 지정한다) |

## 6. 구현 위치

| 내용 | 위치 |
| --- | --- |
| 이동 시간·녹색 숫자·유지 속도의 순수 계산 | `crates/rbms-config/src/speed.rs`(`held_speed`, `HeldTravel`, `HeldSpeed`, `green_of_duration`, `duration_of_green`, `duration_stepped`, `duration_stepped_by_green`, `travel_duration_ms`) |
| 설정 필드 | `crates/rbms-config/src/schema.rs`(`PlayOptions.keep_green_number`, `duration_ms`, `JudgeOptions.offset_ms`, `auto_offset`) |
| 자동 보정의 한 걸음 | `crates/rbms-play/src/session.rs`(`auto_adjust_step_ms`, `PlaySession::judge_offset_ms`, `recorded_offset_steps`) |
| 플레이 중 조작 | `apps/rbms-player/src/stage/play/controls.rs`, `apps/rbms-player/src/app_input.rs`(`apply_control`, `fine_pressed`) |
| 조작과 단축키의 종류, 저장 형식 | `apps/rbms-player/src/keyconfig.rs`(`ControlAction`, `Shortcut`, `ControlBinds`) |
| IR 리플레이 변환 | `apps/rbms-player/src/ir_replay.rs`(`uploaded_time_us`) |
| PLAY 메뉴(행 목록, 조작, 정보 패널) | `apps/rbms-player/src/stage/play_menu.rs`, `stage/play_menu/rows.rs`(행의 종류, 표시 꼬리, 읽기 전용, 큰 단계, 기본값 복원) |
| PLAY 메뉴와 SKIN 메뉴가 같이 쓰는 그리기·스크롤 부품 | `apps/rbms-player/src/stage/preview_menu.rs` |
| 미리보기의 대상과 계산 요약 | `apps/rbms-player/src/skin_preview/target.rs`(`PlayTarget`, `SpeedReadout`, `PlayTarget::readout`) |
| 행이 속한 묶음 | `crates/rbms-config/src/settings.rs`(`SettingTab::PLAY_MENU` = `PlaySpeed`·`PlayLane`·`PlayJudge`·`PlayChart`, `SettingTab::SHOWN` = Settings 의 탭 6개) |
| 화면을 지정하는 스킨 옵션 통로 | `apps/rbms-player/src/skin_select.rs`(`rows_for`, `line_for`, `step_for`, `reset_property_for`) |

## 7. PLAY 메뉴

플레이할 때마다 만지는 옵션은 SETTINGS 가 아니라 독립 화면인 PLAY 메뉴에 있다. 왼쪽이 묶음별 행 목록, 오른쪽이 고른 모드의 플레이 화면을 그 모드의 스킨으로 그린 미리보기이고, 값을 바꾸면 그 프레임부터 미리보기에 반영된다. 저장 위치와 형식은 그대로다(`settings.ron` 의 `play`·`judge` 그룹과 `skin.custom`. 이전 판의 설정 파일이 그대로 읽히고 스키마 판도 그대로다).

### 7.1 들어가고 나가는 길

| 길 | 동작 |
| --- | --- |
| 선곡에서 `F8` | PLAY 메뉴를 연다(내장 선곡과 스킨 선곡 모두. 검색어를 치는 중에는 입력을 확정한 뒤). 키 설정 화면에서 바꿀 수 있다(PLAY MENU, 설정 파일 키 `play_menu`) |
| SETTINGS 에서 같은 키 또는 탭 띠의 `PLAY MENU >` 단추(`SKIN MENU >` 앞) | SETTINGS 위에 연다. 글자를 치는 중이거나 RIVALS·IR PROFILES 목록이 열려 있으면 동작하지 않는다. 제목 아래 안내 줄에 지금 묶인 키가 `F8 PLAY` 처럼 나온다 |
| 스킨의 버튼 | 없다. 원본(beatoraja)에 플레이 옵션 화면을 여는 이벤트가 없어(화면을 바꾸는 이벤트는 키 설정 13, 스킨 설정 14 뿐) 기본 스킨 사이드 메뉴에 행을 두지 않았다 |
| `Esc` | 설정을 저장하고 왔던 화면(선곡 또는 SETTINGS)으로 돌아간다. 미리보기가 쓰던 텍스처와 Lua 상태는 이때 놓는다 |

### 7.2 행

| 묶음 | 행 | 표시 | 한 칸 / Shift |
| --- | --- | --- | --- |
| PREVIEW | PREVIEW BPM | `150 BPM` | 100·120·150·180·200 을 돈다. 설정이 아니다(7.4) |
| SPEED | KEEP GREEN NUMBER | `ON` / `OFF` | 뒤집기 |
| SPEED | GREEN NUMBER | `300 (500 MS)`: 녹색 숫자와 이동 시간. 읽기 전용일 때 `480 (800 MS) AUTO` | 1 / 10 |
| SPEED | HI-SPEED | `2.00x`. 읽기 전용일 때 `2.24x AUTO` | HI-SPEED STEP / 그 열 배 |
| SPEED | HI-SPEED STEP | `0.25x` | 0.01 / 0.10 |
| SPEED | HI-SPEED FIX | OFF·START·MAX·MAIN·MIN | 한 항목 |
| SPEED | SPEED FIX | FLOATING·CONSTANT | 뒤집기 |
| LANE | LANE COVER ON, LIFT ON, HIDDEN+ ON | `ON` / `OFF` | 뒤집기 |
| LANE | LANE COVER | `30% (WHITE 300)`: 비율과 흰색 숫자 | 1% / 10% |
| LANE | LIFT, HIDDEN+ | `15%` | 1% / 10% |
| LANE | COVER FINE STEP | `0.0010 (WHITE 1.0)`: 비율과 그 한 칸이 흰색 숫자로 얼마인지 | 0.0001 / 0.0010 |
| LANE | 미리보는 플레이 스킨 문서가 선언한 옵션(기본 스킨: Lane Width, Field Side, Scratch Side, Judge Detail, Score Difference, Judge Display, Score Graph) | 항목 이름 | 한 항목. 그 문서의 커스터마이즈(`skin.custom`)에 저장되고 문서를 다시 읽는다. SKIN 메뉴의 같은 행과 같은 값이다. 파일 칸은 SKIN 메뉴에만 있다 |
| LANE | 그 문서가 선언한 위치·크기 오프셋(기본 스킨: `JUDGE POSITION Y`). 옵션 바로 뒤 | `+0` | 1 / 10(픽셀. 각도 축은 도). 범위 ±2000. 문서를 다시 읽지 않고 다음 프레임의 미리보기에 반영된다. SKIN 메뉴의 같은 행과 같은 값이다 |
| JUDGE | JUDGE OFFSET | `+5 MS` | 1 ms / 10 ms |
| JUDGE | AUTO ADJUST | `ON` / `OFF` | 뒤집기 |
| CHART | RANDOM, GAUGE, TARGET, ESCAPE | 항목 이름 | 한 항목 |
| CHART | LEGACY NOTE, AUTOPLAY, SCRATCH AUTO, AUTO REPLAY | `ON` / `OFF` | 뒤집기 |
| SKIN | 접이 `OPACITY` | `HIDDEN (19)` / `SHOWN (19)`: 접힘 여부와 든 행 수 | Enter·← →·클릭으로 펴고 접는다. 펴면 그 문서의 불투명도 행이 바로 아래에 나온다(기본 스킨 7키 19행, 14키 18행) |
| SKIN | 불투명도 행(접이를 폈을 때. 기본 스킨: `LANE BACKGROUND OPACITY` 등) | `100%` | 5% / 25%. 0% 면 그 부위가 사라진다. 0 키로 100%. 100% 를 넘으면 `105% (+13)` 처럼 더해지는 알파가 붙는다(`docs/skin.md` 2.2절) |
| SKIN | 접이 `POSITION AND SIZE` | `HIDDEN (15)` / `SHOWN (15)` | 엔진이 모든 플레이 스킨에 붙이는 오프셋 네 개의 행(`ALL OFFSET(%) X` … `JUDGE DETAIL OFFSET ALPHA`). 1 / 10 |

- KEEP GREEN NUMBER 가 켜져 있으면 GREEN NUMBER 가 조절 대상이고 HI-SPEED 는 예시 BPM 에서 계산된 값을 보이는 읽기 전용 행이다(범위 끝에 걸리면 `20.00x (LIMIT) AUTO`). 꺼져 있으면 반대로 HI-SPEED 가 조절 대상이고 GREEN NUMBER 는 그 속도가 예시 BPM 에서 내는 녹색 숫자와 이동 시간을 보인다. 읽기 전용 행은 이름과 값이 모두 흐리고 값 뒤에 `AUTO` 가 붙으며, 좌우·Enter·0·클릭에 움직이지 않고, 정보 패널이 이유를 말한다. 유지 중에는 저장된 HI-SPEED 가 쓰이지 않으므로 보이지 않는다(끄면 다시 보인다).
- LANE COVER·LIFT·HIDDEN+ 는 1% 격자 위에서 움직인다(`rbms_config::adjust` 가 값을 걸음 수로 세어 옮긴다). 30% 는 흰색 숫자 300 이고, 플레이 중 미세 단계로 격자 밖에 둔 값(30.5% 등)에서 한 칸 움직이면 가장 가까운 격자에서 한 칸 간 값이 된다.
- SKIN 묶음은 목록의 마지막 묶음이고 두 접이는 닫힌 채로 열린다(접이 상태는 저장되지 않는다). 접이를 펴면 든 행이 화면에 들어오도록 목록이 스크롤된다: 다 들어가면 전부, 넘치면 접이 행이 목록 첫 줄에 온다. 스킨이 불투명도나 엔진 오프셋을 갖지 않으면 그 접이는 없고, 둘 다 없으면 묶음도 없다. 불투명도와 오프셋은 SKIN 메뉴의 같은 행과 같은 곳(그 문서의 `skin.custom` 오프셋)에 저장된다.
- 이름이 비슷한 세 가지는 다른 값이다. JUDGE 묶음의 `JUDGE OFFSET`(ms)은 판정 타이밍이다. LANE 묶음의 `JUDGE POSITION Y` 는 기본 스킨이 판정 글자·콤보·위 줄을 함께 옮기는 오프셋이다(8절). 접이 안의 `JUDGE OFFSET X/Y/W/H/ALPHA` 와 `JUDGE DETAIL OFFSET …` 은 엔진이 스킨과 무관하게 붙이는 오프셋으로, 판정 글자와 그 위 줄을 따로 옮긴다.
- LANE OPTION 은 아직 플레이에 적용되지 않아 어디에도 보이지 않는다(값은 저장·복원된다).
- 큰 단계가 범위를 넘으면 범위 끝까지만 간다(HI-SPEED 는 0.25~19.75 처럼 한 칸씩 갈 수 있는 데까지).
- JUDGE OFFSET 의 한 칸은 선곡 패널 3·플레이 중 키와 같은 1 ms 다(SETTINGS 의 옛 행은 5 ms 였다). 표의 걸음(`JUDGE_OFFSET_STEP_MS`)이 1 이고 PLAY 메뉴도 다른 숫자 행처럼 `adjust` 를 탄다.

### 7.3 조작

| 입력 | 동작 |
| --- | --- |
| ↑ ↓ | 행 커서 이동. 묶음 이름은 건너뛴다. 목록은 17칸씩 보이고 넘치면 스크롤 막대와 `현재 / 전체`(묶음 이름을 포함한 칸 수)가 나온다 |
| PageUp PageDown | 한 화면(17칸)씩 |
| ← → | 커서 행의 값을 한 칸 |
| Shift + ← → | 숫자 행과 스킨 오프셋 행이면 열 칸, 불투명도 행이면 25%. 켬·끔과 목록 행, 스킨 옵션 행, PREVIEW BPM, 접이는 한 칸 그대로다 |
| Enter | → 한 칸과 같다. 접이 행에서는 펴거나 접는다. 화면을 떠나지 않는다 |
| 0 (숫자 줄, 넘패드) | 커서 행을 기본값으로(새 설정 파일의 값. PREVIEW BPM 은 150, 스킨 옵션은 문서의 기본 항목, 스킨 오프셋은 0, 불투명도는 100%). 접이는 움직이지 않는다 |
| Tab / Shift+Tab | 미리보기 위 모드 띠(7K 5K 14K 10K 9K)에서 다음 / 이전 |
| 행 왼쪽 클릭 / 오른쪽 클릭 | 그 행으로 커서를 옮기고 한 칸 앞으로 / 뒤로 |
| 모드 띠의 탭 왼쪽 클릭 | 그 모드로 |
| 휠 | 포인터가 목록 위일 때 목록을 스크롤한다(커서는 그대로) |
| 미리보기와 묶음 이름 위의 클릭 | 아무 일도 없다 |

### 7.4 미리보기와 정보 패널

| 항목 | 내용 |
| --- | --- |
| 모드 띠와 PREVIEW BPM | 미리보기의 예시 차트를 고른다. 설정 파일에 저장되지 않고 SKIN 메뉴가 고른 화면(`skin.screen`)도 바꾸지 않는다. 메뉴를 열 때마다 7K·150 BPM 이다 |
| 녹색 숫자 유지를 눈으로 보기 | KEEP GREEN NUMBER 를 켜고 PREVIEW BPM 을 바꾸면 노트가 내려오는 시간과 녹색 숫자는 그대로이고 HI-SPEED 표시가 바뀐다. 끄면 HI-SPEED 가 그대로이고 녹색 숫자와 노트 속도가 바뀐다 |
| 정보 패널 윗줄(요약) | 다섯 칸이 나란히 있고 칸마다 작은 이름 아래 큰 숫자가 있다: `BPM 150`, `HI-SPEED 2.24 (HELD)`, `TRAVEL 500 MS`, `GREEN 300`(초록), `WHITE 300`. 읽는 법은 아래 표 |
| 계산 | 플레이 화면과 같은 함수다(`rbms_config::held_speed`, `travel_duration_ms`, `green_of_duration`). 미리보기 스킨의 HI-SPEED·GREEN·레인 커버 숫자와 같은 값이 나온다 |
| 정보 패널 아래 줄 | 커서 행의 이름·값 전문과 설명 |
| 문서가 없는 모드, 읽기 실패 | 미리보기 자리에 이유를 글로 보인다(`NO SKIN DOCUMENT FOR THIS SCREEN` 등). 행은 그대로 조절된다 |
| 예시 판정 | 사용자의 JUDGE OFFSET 을 읽지도 바꾸지도 않는다(AUTO ADJUST 가 켜져 있어도). 소리는 내지 않는다 |
| 끊임없는 예시 | 차트가 끝나면 마지막 노트 약 1초 뒤 같은 화면에서 처음부터 다시 내려온다. PREVIEW BPM 과 GAUGE 를 바꾸면 로딩 없이 같은 진행 비율에서 그 템포·그 게이지로 이어 간다(콤보·점수 유지). 모드를 바꾸면 다른 스킨 문서이므로 새 장면이다 |

요약 읽는 법

| 칸 | 뜻 | 뒤에 붙는 말 |
| --- | --- | --- |
| BPM | 스크롤을 계산한 템포. 예시 차트의 템포(PREVIEW BPM) | `(CONSTANT)`: SPEED FIX 가 CONSTANT 라 예시 템포가 아니라 필드의 고정 템포 120 으로 계산했다 |
| HI-SPEED | 그 템포에서 실제로 쓰이는 배율. 미리보기 스킨의 HI-SPEED 숫자와 같다 | `(HELD)`: 녹색 숫자를 유지하려고 계산된 값. `(LIMIT)`(노랑): 유지할 배율이 0.01~20.00 밖이라 범위 끝에 멈췄다. 이때 숫자 줄 아래에 `HI-SPEED IS AT THE END OF ITS RANGE: NO SPEED HOLDS THIS GREEN NUMBER AT THIS TEMPO` 한 줄이 더 나오고, GREEN 칸은 목표가 아니라 실제 값을 보인다 |
| TRAVEL | 이동 시간(ms). 레인 커버가 켜져 있으면 커버 아래 끝부터 판정선까지 | `MS` |
| GREEN | 녹색 숫자(이동 시간의 3/5). 유지 중이면 PREVIEW BPM 을 바꿔도 그대로다 | 없음 |
| WHITE | 흰색 숫자(레인 커버 비율의 1000 배) | `(COVER OFF)`: 레인 커버가 꺼져 있어 계산에 들어가지 않았다 |

미리보기에 반영되는 행과 보이지 않는 행

| 행 | 미리보기 |
| --- | --- |
| KEEP GREEN NUMBER, GREEN NUMBER, HI-SPEED, SPEED FIX | 노트가 내려오는 속도와 스킨의 HI-SPEED·GREEN 숫자 |
| LANE COVER·LIFT·HIDDEN+ 와 각 ON 행 | 가림막의 위치와 스킨의 숫자(커버 300, 리프트 150 등) |
| 스킨 옵션(Lane Width, Judge Display 등) | 문서를 다시 읽어 그린다(잠깐 어두워졌다가 같은 위치에서 이어 간다) |
| 스킨 오프셋(JUDGE POSITION Y 등)과 불투명도 | 문서를 다시 읽지 않는다. 값이 바뀐 다음 프레임부터 그 위치·그 진하기로 그린다 |
| JUDGE OFFSET, AUTO ADJUST | 스킨의 TIMING 숫자와 `TIMING AUTO` 표지. 예시 판정은 달라지지 않는다 |
| GAUGE | 게이지의 종류와 색 |
| TARGET | 점수 그래프의 타깃 막대와 `TARGET +n`. 기록·IR·라이벌 없이 예시 차트의 노트 수만으로 정한 타깃이다(MY BEST·IR·RIVAL 은 MAX 로 떨어진다) |
| AUTOPLAY | 스킨의 AUTOPLAY 표지. 예시 손(적어 둔 입력)은 그대로다 |
| LEGACY NOTE | 롱노트가 일반 노트로 그려진다 |
| RANDOM, HI-SPEED FIX, HI-SPEED STEP, COVER FINE STEP, SCRATCH AUTO, AUTO REPLAY, ESCAPE | 보이지 않는다. 예시 차트의 배치와 손은 바꾸지 않고(RANDOM), 예시 차트가 한 템포라 기준 BPM 이 어느 것이든 같으며(HI-SPEED FIX), 나머지는 화면에 그릴 것이 없는 값이다 |

### 7.5 SETTINGS 에 남은 것과 옮긴 것

| 행 | 이전 | 지금 |
| --- | --- | --- |
| HI-SPEED, HI-SPEED STEP, SPEED FIX, HI-SPEED FIX, KEEP GREEN NUMBER, GREEN NUMBER | SETTINGS > PLAY | PLAY 메뉴 > SPEED |
| LANE COVER·LIFT·HIDDEN+ 와 각 ON 행, COVER FINE STEP | SETTINGS > DISPLAY | PLAY 메뉴 > LANE |
| JUDGE OFFSET, AUTO ADJUST | SETTINGS > JUDGE | PLAY 메뉴 > JUDGE |
| RANDOM, LEGACY NOTE, AUTOPLAY, AUTO REPLAY, ESCAPE | SETTINGS > PLAY | PLAY 메뉴 > CHART |
| GAUGE | SETTINGS > GAUGE | PLAY 메뉴 > CHART |
| TARGET | SETTINGS > JUDGE | PLAY 메뉴 > CHART |
| SCRATCH AUTO | SETTINGS > INPUT | PLAY 메뉴 > CHART |
| 5KEYS LAYOUT | SETTINGS > PLAY | SETTINGS > DISPLAY(내장 필드만 읽는 행. WHITE NUMBER, JUDGE TEXT Y 아래) |
| TOTAL | SETTINGS > GAUGE | SETTINGS > JUDGE(게이지 규칙 행들 아래) |
| JUDGE ALGORITHM, JUDGE WIDTH 6행, LN MARGIN, LN MODE, GAUGE SET, GAUGE AUTO SHIFT, BOTTOM SHIFTABLE | SETTINGS > JUDGE | 그대로 |
| SCRATCH SIDE, KEY CONFIG | SETTINGS > INPUT | 그대로. SETTINGS 의 SCRATCH SIDE 는 내장 필드의 값이고(스킨이 그리는 동안 행 옆에 `SKIN OPTION COMES FIRST` 표지가 붙는다), 스킨이 그리는 화면의 스크래치 쪽은 PLAY 메뉴 LANE 묶음의 스킨 옵션 SCRATCH SIDE 다. 이름이 같은 두 행은 다른 값이다 |
| DISPLAY 의 나머지, NETWORK, AUDIO, SELECT | — | 그대로 |

SETTINGS 의 탭은 JUDGE, DISPLAY, INPUT, NETWORK, AUDIO, SELECT 여섯이다(PLAY 탭과 GAUGE 탭은 없어졌다). 플레이 직전의 빠른 옵션(F1 오버레이)과 선곡의 옵션 패널 1~3 은 그대로이고 같은 값을 고친다. `rbms-cli config` 덤프는 PLAY 메뉴의 묶음을 `[PLAY SPEED]`, `[PLAY LANE]`, `[PLAY JUDGE]`, `[PLAY CHART]` 로 적는다.

## 8. 판정 표시(기본 스킨)

기본 스킨의 플레이 화면이 판정을 보이는 방식이다. 판정·점수 계산은 달라지지 않고 표시만의 설정이다. 조절하는 행은 PLAY 메뉴의 LANE 묶음과 SKIN 메뉴에 같은 값으로 있다.

| 항목 | 동작 | 행 |
| --- | --- | --- |
| 정적 표시 | 판정 글자와 콤보는 판정이 난 프레임부터 0.5초 동안 같은 자리·크기·색으로 서 있다가 한 번에 사라진다. 튀어나오거나 커지거나 깜빡이거나 서서히 사라지지 않는다. 콤보가 올라가도 글자의 자리는 그대로이고 숫자만 바뀐다. 콤보는 글자 바로 뒤에 붙고(`GREAT 67`) 자릿수가 늘면 오른쪽으로만 자란다. 앞자리 0 은 없다 | 없음(항상) |
| 위 줄 왼쪽: 점수 차 | 목표 대비 EX 점수 차를 부호와 네 자리로 쓴다(`+0003`, `-0125`). 스코어 그래프의 `TARGET` 숫자와 같은 값이고 CHART 묶음의 TARGET 이 목표를 정한다. PGREAT 에도 뜬다 | `SCORE DIFFERENCE`: TARGET(기본) / BEST(자기 최고 기록 대비) / OFF |
| 위 줄 오른쪽: FAST/SLOW | PGREAT 가 아닌 판정이 빨랐으면 FAST(파랑), 늦었으면 SLOW(빨강) | `JUDGE DETAIL`: FAST-SLOW(기본) / MILLISECONDS(어긋난 시간을 ms 로. 빠르면 `+`) / OFF |
| 위치 | 판정 글자·콤보·위 줄을 함께 옮긴다. + 는 위, - 는 아래. 한 칸 1px, Shift 로 10px, 0 키로 제자리. 스킨 좌표(1920x1080 기준)다 | `JUDGE POSITION Y`: -2000 ~ +2000, 기본 +0 |
| 숨김 | 판정 글자·콤보·위 줄을 그리지 않는다 | `JUDGE DISPLAY`: SHOW(기본) / HIDE |

- DP 는 친 쪽 필드에만 뜬다. BAD·POOR 는 콤보 없이 글자만 뜨고, 글자의 왼쪽 끝은 다른 판정과 같은 자리다(판정이 바뀌어도 글자가 좌우로 옮겨 다니지 않는다). 위 줄도 같은 왼쪽 끝에서 시작한다.
- 다른 스킨 팩은 자기 판정 객체와 자기 옵션을 쓴다(ModernChic 의 판정 연출은 그대로다). 위 행들은 기본 스킨 문서가 선언한 것이라 다른 팩에서는 나오지 않는다.
- 레인 면·선·건반 표시 같은 부위를 지워 노트와 판정만 남기는 법은 `docs/skin.md` 3.4.2절(부위별 불투명도)에 있다.
