# 플레이 옵션

> 최종 갱신 2026-10-11 · 웨이브 10A(넓은 레인, 키 사용자 지정, 판정 자동 보정, 녹색 숫자 유지) 기준
> 대상: 플레이 중 스크롤·가림막·판정 타이밍을 정하는 옵션과 그 키. 스킨이 그리는 옵션(Lane Width 등)은 [skin.md](./skin.md) 3.4절, 구현 위치는 [crates.md](./crates.md)를 본다.
> beatoraja 원본의 같은 기능이 기준이다. 다르게 한 곳은 "원본과 다른 점" 열에 이유를 적는다.

## 1. 용어

| 용어 | 뜻 |
| --- | --- |
| 이동 시간(duration) | 노트가 레인 위쪽 끝(레인 커버가 켜져 있으면 커버 아래 끝)에서 나타나 판정선에 닿을 때까지의 시간. 단위 ms |
| 녹색 숫자(green number) | 이동 시간의 3/5 를 버림한 값(`duration * 3 / 5`). IIDX 의 녹색 숫자, beatoraja 의 `duration_green`(스킨 number 313)과 같다. 이동 시간 500 ms = 녹색 300 |
| 흰색 숫자(white number) | 레인 커버가 가린 비율의 1000 배(스킨 number 14). 커버 0.3 = 300 |
| 하이스피드(hispeed) | 스크롤 배율. `이동 시간 = 240000 / BPM / hispeed * (1 - 커버)` |
| 판정 오프셋(judge offset) | 입력이 판정되는 시각에 더하는 값. 단위 ms. 양수는 "빠르게 치는 사람"을 위한 보정이다(입력을 그만큼 늦게 친 것으로 본다) |

## 2. 옵션 표

| 옵션 | 뜻 | 단위와 범위 | 기본값 | 바꾸는 곳 | beatoraja·IIDX 대응 | 원본과 다른 점 |
| --- | --- | --- | --- | --- | --- | --- |
| HI-SPEED | 스크롤 배율 | 배율 0.01~20.00, 한 번에 HI-SPEED STEP(기본 0.25) | 2.00 | Settings > PLAY, 플레이 중 `UP`/`DOWN`, START + 건반 | `PlayConfig.hispeed` | 없음 |
| HI-SPEED FIX | 녹색 숫자를 계산할 때 기준으로 삼는 BPM | OFF / START / MAX / MAIN / MIN | MAIN | Settings > PLAY, 선곡 패널 1 의 5번 키 | `PlayConfig.fixhispeed` | KEEP GREEN NUMBER 가 꺼져 있으면 한 판 안에서만 이동 시간을 묶는다(곡 사이에는 HI-SPEED 가 그대로). 원본은 OFF 가 아니면 곡 사이에도 묶는다. 그 역할은 아래 KEEP GREEN NUMBER 가 맡는다 |
| KEEP GREEN NUMBER | 녹색 숫자를 곡과 곡 사이에 유지한다. 켜면 매 곡 시작과 매 프레임에 HI-SPEED 가 `240000 / 기준 BPM / 이동 시간 * (1 - 레인 커버)` 로 다시 계산된다 | 켬 / 끔 | 끔 | Settings > PLAY, 플레이 중 KEEP GREEN NUMBER ON/OFF 키(출하 시 미지정) | `fixhispeed != OFF` 일 때의 `LaneRenderer.resetHispeed`, IIDX 의 FHS | 별도 스위치로 두었다. 기존 설정 파일은 끔으로 읽혀 이전과 같은 속도로 스크롤된다. HI-SPEED FIX 가 OFF 이면 기준 BPM 은 MAIN, CONSTANT 이면 120 이다 |
| GREEN NUMBER | KEEP GREEN NUMBER 가 유지하는 목표 녹색 숫자 | 1~6000(이동 시간 1~10000 ms 로 저장), Settings 행은 1씩 | 300(이동 시간 500 ms) | Settings > PLAY, 플레이 중 하이스피드 키(±10, 미세 단계 ±1), SELECT + 건반 또는 GREEN NUMBER UP/DOWN 키(이동 시간 ±1 ms), 선곡 패널 3 의 4·6번 키(이동 시간 ±1 ms, 누르고 있으면 반복) | `PlayConfig.duration` | 원본의 하이스피드 키는 유지 모드에서 배율만 바꾸고 저장된 이동 시간은 그대로 둔다(다음 커버 이동이나 다음 곡에서 되돌아간다). 여기서는 키를 누른 것이 새 목표를 고른 것이므로 목표가 바뀌어 남는다 |
| LANE COVER | 위에서 내려오는 가림막. 값이 커질수록 더 가린다 | 비율 0~1, 한 번에 0.01, 미세 단계 0.0001~0.01(기본 0.001) | 0, 켬 | Settings > PLAY, 플레이 중 `RIGHT`(더 가림)/`LEFT`(덜 가림), `BACKSLASH`(켬·끔), START + 턴테이블, START 두 번(켬·끔), 마우스 휠 | `PlayConfig.lanecover`, `enablelanecover`, IIDX 의 SUDDEN+ | 켬·끔 전용 키와 방향을 적은 행 이름을 더했다 |
| LIFT | 판정선을 올린다 | 비율 0~1 | 0, 끔 | Settings > PLAY, 플레이 중 `RBRACKET`/`LBRACKET`, 켬·끔 키(출하 시 미지정) | `PlayConfig.lift`, IIDX 의 LIFT | 없음 |
| HIDDEN+ | 판정선에서 위로 올라오는 가림막 | 비율 0~1 | 0, 끔 | Settings > PLAY, 플레이 중 올림·내림·켬·끔 키(출하 시 미지정) | `PlayConfig.hidden`, IIDX 의 HIDDEN+ | 없음 |
| JUDGE OFFSET | 판정 오프셋 | -200~+200 ms, Settings 행은 5 ms 씩 | 0 | Settings > JUDGE, 선곡 패널 3 의 5·7번 키(±1 ms), 플레이 중 JUDGE OFFSET ±1 MS 키(출하 시 미지정) | `PlayerConfig.judgetiming` | 원본은 값을 노트 표시 시계에 더한다(빠르게 치면 노트를 늦게 그린다). 여기서는 입력의 판정 시각에 더하므로 부호가 반대다: 빠르게 치는 사람은 + |
| AUTO ADJUST | 칠 때마다 JUDGE OFFSET 을 조금씩 움직인다 | 켬 / 끔 | 끔 | Settings > JUDGE, 선곡 패널 3 의 3번 키, 플레이 중 AUTO ADJUST ON/OFF 키(출하 시 미지정) | `PlayerConfig.notesDisplayTimingAutoAdjust`(`JudgeManager.java:697-703`) | 부호만 위와 같은 이유로 반대다 |

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
| 사용자 지정 범위 | 레인과 스크래치 역방향, 플레이 조작 20종, 선곡 단축키 9종, 화면 여는 키 5종, 스킨 브라우저 숫자 키 8종, 결과 2종, 리플레이 분석 5종. 키보드와 컨트롤러 둘 다(옵션 오버레이 두 키와 상세 패널 홀드 키는 키보드 전용) |
| 고정 키 | `Esc`, `Enter`, 화살표, `H`(키 안내)와 각 메뉴 안의 키 |
| 레인 커버 조작 | `RIGHT` = 더 가림(LANE COVER DOWN), `LEFT` = 덜 가림(LANE COVER UP), `BACKSLASH` = 켬·끔. 커버가 꺼져 있을 때 값을 바꾸면 발치의 COVER 수치가 1.5초 동안 뜬다 |
| 미세 단계 | FINE STEP 키를 누르고 있는 동안 가림막은 미세 단계로, 유지 중인 녹색 숫자는 ±1 로 움직인다. 키를 주지 않았으면 레인으로 쓰지 않는 Shift 가 그 역할을 한다(7키 기본 배치에서는 오른쪽 Shift) |
| 지금 설정된 키 보기 | 선곡에서 `H` |
| 저장 | `~/.config/rbms/keyconfig.ron`. 이전 파일은 그대로 읽히고 없는 항목은 기본 키로 채워진다 |

## 6. 구현 위치

| 내용 | 위치 |
| --- | --- |
| 이동 시간·녹색 숫자·유지 속도의 순수 계산 | `crates/rbms-config/src/speed.rs`(`held_speed`, `HeldTravel`, `HeldSpeed`, `green_of_duration`, `duration_of_green`, `duration_stepped`, `duration_stepped_by_green`, `travel_duration_ms`) |
| 설정 필드 | `crates/rbms-config/src/schema.rs`(`PlayOptions.keep_green_number`, `duration_ms`, `JudgeOptions.offset_ms`, `auto_offset`) |
| 자동 보정의 한 걸음 | `crates/rbms-play/src/session.rs`(`auto_adjust_step_ms`, `PlaySession::judge_offset_ms`, `recorded_offset_steps`) |
| 플레이 중 조작 | `apps/rbms-player/src/stage/play/controls.rs`, `apps/rbms-player/src/app_input.rs`(`apply_control`, `fine_pressed`) |
| 조작과 단축키의 종류, 저장 형식 | `apps/rbms-player/src/keyconfig.rs`(`ControlAction`, `Shortcut`, `ControlBinds`) |
| IR 리플레이 변환 | `apps/rbms-player/src/ir_replay.rs`(`uploaded_time_us`) |
