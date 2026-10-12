# 플레이 옵션

> 최종 갱신 2026-10-12 · 웨이브 13(판정 오프셋을 원본의 노트 표시 타이밍으로: 1~3절, 7.4절. 기본 스킨의 판정선을 필드 맨 아래로: 8절. 작업 트리 기준, 커밋 전) 기준. 웨이브 12 는 연습 패널의 숫자 행(9절)
> 대상: 플레이 중 스크롤·가림막·판정 타이밍을 정하는 옵션과 그 키, 그 옵션을 바꾸는 화면인 PLAY 메뉴(7절), 기본 스킨의 판정 표시(8절), 연습 패널(9절). 스킨이 그리는 옵션(Lane Width 등)과 부위별 불투명도는 [skin.md](./skin.md) 3.4절, 구현 위치는 [crates.md](./crates.md)를 본다. PLAY 메뉴의 설계는 [plan/2026-10-11-play-menu.md](./plan/2026-10-11-play-menu.md).
> beatoraja 원본의 같은 기능이 기준이다. 다르게 한 곳은 "원본과 다른 점" 열에 이유를 적는다.

## 1. 용어

| 용어 | 뜻 |
| --- | --- |
| 이동 시간(duration) | 노트가 레인 위쪽 끝(레인 커버가 켜져 있으면 커버 아래 끝)에서 나타나 판정선에 닿을 때까지의 시간. 단위 ms |
| 녹색 숫자(green number) | 이동 시간의 3/5 를 버림한 값(`duration * 3 / 5`). IIDX 의 녹색 숫자, beatoraja 의 `duration_green`(스킨 number 313)과 같다. 이동 시간 500 ms = 녹색 300 |
| 흰색 숫자(white number) | 레인 커버가 가린 비율의 1000 배를 반올림한 값(스킨 number 14). 커버 0.3 = 300. 리프트(314)와 HIDDEN+(315)의 숫자도 같은 방식이다 |
| 하이스피드(hispeed) | 스크롤 배율. `이동 시간 = 240000 / BPM / hispeed * (1 - 커버)` |
| 판정 오프셋(judge offset) | 노트를 곡보다 얼마나 앞서 그릴지. 단위 ms. beatoraja 의 `judgetiming`(노트 표시 타이밍)과 같은 값이다. +10 이면 노트가 자기 소리와 판정 시각보다 10 ms 먼저 판정선에 닿는다. 소리와 입력이 판정되는 시각은 옮기지 않는다. 늦게 치는 사람은 올리고(+) 빠르게 치는 사람은 내린다(-). 자세한 것은 3절 |

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
| JUDGE OFFSET | 판정 오프셋 | -200~+200 ms, PLAY 메뉴의 행은 1 ms 씩(Shift 10 ms) | 0 | PLAY 메뉴 > JUDGE, 선곡 패널 3 의 5·7번 키(±1 ms), 플레이 중 JUDGE OFFSET ±1 MS 키(출하 시 미지정) | `PlayerConfig.judgetiming`(`LaneRenderer.java:300-301`) | 뜻과 부호는 같다. 범위만 좁다(원본 -500~+500 ms) |
| AUTO ADJUST | 칠 때마다 JUDGE OFFSET 을 조금씩 움직인다 | 켬 / 끔 | 끔 | PLAY 메뉴 > JUDGE, 선곡 패널 3 의 3번 키, 플레이 중 AUTO ADJUST ON/OFF 키(출하 시 미지정) | `PlayerConfig.notesDisplayTimingAutoAdjust`(`JudgeManager.java:697-703`) | 없음(식과 부호가 같다) |

## 3. 판정 오프셋과 자동 보정(JUDGE OFFSET, AUTO ADJUST)

### 3.1 오프셋이 옮기는 것

JUDGE OFFSET 은 노트 필드를 그리는 시각에만 더해진다. 원본이 `judgetiming` 을 읽는 곳은 `LaneRenderer.java:300-301` 한 줄뿐이고(`time = 플레이 타이머 + judgetiming`), `JudgeManager` 와 키음·BGM 은 그 값을 읽지 않는다. R-BMS 도 같다(웨이브 13 부터).

| 오프셋이 +N ms 일 때 | 어디에 있나 |
| --- | --- |
| 노트의 소리, BGM, BGA(미스 레이어 포함), 스킨의 타이머 | 노트의 시각 T 그대로. BGA 는 플레이 타이머만 읽는다(`SkinBGA.java:42-44`, `PlayState::played_chart_us`) |
| PGREAT 창의 가운데(입력이 판정되는 기준) | T 그대로. 입력은 들어온 시각에 판정된다 |
| 손으로 친 노트의 키음 | 누른 순간 |
| 노트와 마디선이 판정선에 닿아 그려지는 시각, 스킨이 읽는 현재 BPM(number 160) | T - N (N ms 먼저). 원본도 현재 BPM 을 필드가 그리는 시각에서 구한다 |

1. 쓰임: 노트를 보고 쳤을 때 계속 SLOW 가 나면 올리고(+, 노트가 먼저 온다) 계속 FAST 가 나면 내린다(-, 노트가 늦게 온다). 화면의 표시 지연을 메우는 값이다.
2. 박자에 맞춰 친 입력은 어느 값에서도 PGREAT 다. 오프셋은 소리와 판정 사이를 벌리지 않는다.
3. 오토플레이와 리플레이도 노트는 보는 사람의 오프셋만큼 앞서 그려지고 판정(폭발, 판정 글자, 키음)은 T 에 난다. 노트는 판정 여부와 상관없이 필드의 시각이 노트에 닿을 때, 곧 판정선에 닿을 때 사라진다(`LaneRenderer.java:546`. 친 노트도 판정선까지는 그려진다). 그래서 +100 이면 노트가 판정선에서 사라진 뒤 100 ms 뒤에 판정이 나고, -100 이면 판정이 난 뒤에도 노트가 100 ms 동안 판정선까지 내려온다. 손으로 칠 때도 같다(원본과 같다. 캡처 `offset-plus-100-50ms-before-the-row`, `offset-minus-100-50ms-after-the-row`).
4. 차트가 시작하기 전(로딩·READY)에도 필드는 오프셋만큼 앞선 자리로 그려진다(원본과 같다).
5. 연습의 배속: 오프셋은 플레이어의 시간이다. 0.5 배속이면 차트 시간으로는 절반만큼 앞서 그린다(`PlayState::field_lead_us`).
6. 값이 바뀌면 다음 프레임부터 노트의 자리가 그만큼 옮겨진다. 자동 보정은 입력 한 번에 1~5 ms 를 옮긴다.

측정(150 BPM, 노트 T = 1600 ms, `apps/rbms-player/src/stage/play/timing_tests.rs`). 판정선에 닿는 시각은 곡 시계를 1 ms 씩 옮기며 필드가 그 노트를 판정선에 그리는 첫 시각, PGREAT 창은 1 ms 간격으로 눌러 PGREAT 가 난 구간이다. T 기준 ms.

| 오프셋 | 소리 | 판정선에 닿음(웨이브 12 까지) | PGREAT 창(웨이브 12 까지) | 판정선에 닿음(지금) | PGREAT 창(지금) |
| --- | --- | --- | --- | --- | --- |
| 0 | 0 | 0 | -20~+20 | 0 | -20~+20 |
| +30 | 0 | 0 | -50~-10 | -30 | -20~+20 |
| -30 | 0 | 0 | +10~+50 | +30 | -20~+20 |
| +100 | 0 | 0 | -120~-80 | -100 | -20~+20 |
| -100 | 0 | 0 | +80~+120 | +100 | -20~+20 |

"지금" 두 열과 소리 열은 웨이브 13 통합의 최종 트리에서 다시 잰 값이고(테스트 `the_offset_moves_the_note_on_the_line_and_neither_the_sound_nor_the_window` 가 다섯 오프셋에서 스킨 필드와 내장 필드 둘 다 이 값을 단언한다), "웨이브 12 까지" 열은 고치기 전 빌드에서 잰 값이다(-100 행은 리뷰의 측정). BGA 는 `timing_capture_tests.rs` 의 `the_default_pack_changes_the_background_with_the_song_at_every_offset` 가 오프셋 0·+100·-100 에서 그림이 넘어가는 시각 T 의 50 ms 전에는 아직 없고 50 ms 뒤에는 있음을 본다.

웨이브 12 까지는 오프셋을 입력의 판정 시각에 더했다. 그래서 값을 줄수록 박자에 맞춰 친 입력이 FAST·SLOW 로 판정됐고, PGREAT 를 내려면 박자에서 그만큼 어긋나게 쳐야 했으며 그때 나는 키음도 곡에서 어긋났다. "판정 보정을 켜면 곡과 싱크가 안 맞는 느낌"의 원인이다.

### 3.2 자동 보정(AUTO ADJUST)

1. 대상은 손으로 치는 판(일반 플레이와 연습)뿐이다. 오토플레이와 리플레이에서는 오프셋이 움직이지 않는다.
2. GOOD 이상의 판정이고 오차가 ±150 ms 이내인 입력마다 오프셋에서 `(오차 ± 15 ms) / 30 ms` 를 0 쪽으로 버림한 값을 뺀다(오차는 빠르면 +. 원본의 식 `judgetiming - (mfast ± 15000) / 30000` 그대로). 빠르게 치면 내려가고(노트가 늦게 온다) 늦게 치면 올라간다. 오차가 ±15 ms 안이면 움직이지 않는다.
3. 노트를 보고 40 ms 빠르게 치는 사람은 한 번에 -1 ms 씩 내려가 -26 ms 에서 멈춘다(남는 오차 14 ms). 노트가 늦게 오는 만큼 늦게 치게 되기 때문이다.
4. 노트를 보지 않고 소리만 듣고 치는 사람은 노트의 자리가 바뀌어도 치는 시각이 그대로라 오차가 줄지 않고, 오프셋이 입력마다 계속 움직여 범위 끝(±200 ms)까지 간다. 원본의 규칙이 그렇다. 이런 경우에는 AUTO ADJUST 를 끄고 쓴다.
5. 값은 입력 직후 설정에 반영되고 다음 프레임부터 그 값으로 그려진다. 클리어·폐점·중간에 나가기 어느 쪽으로 끝나도 저장된다.
6. 플레이 화면의 TIMING 숫자(스킨 number 12)가 현재 오프셋이다. 기본 스킨은 켜져 있는 동안 표지를 `TIMING AUTO` 로 쓴다(스킨 image index 75).

### 3.3 리플레이·IR·설정 파일

1. 리플레이: 입력은 들어온 시각에 판정되므로 새로 저장되는 리플레이는 `offset_ms: 0` 이고 `judge.offset_steps` 가 없다. 재생은 보는 사람의 JUDGE OFFSET 과 무관하게 같은 판정이 나온다(오프셋은 노트를 그리는 자리만 옮긴다).
2. 웨이브 12 까지의 리플레이: `offset_ms` 와 `judge.offset_steps` 는 "입력이 판정된 시각이 입력 시각에서 얼마나 떨어져 있었나"로 읽히고, 기록 당시와 같은 시각에 판정된다. 같은 필드를 같은 코드가 읽으므로 판 구분이 없다(새 리플레이는 그 값이 0 일 뿐이다). 그 빌드가 쓴 파일 셋(`crates/rbms-play/tests/fixtures/replay-wave12-*.ron`)이 그때의 집계 그대로 재생되는지 테스트가 본다.
3. 리플레이를 볼 때 사용자의 JUDGE OFFSET 을 리플레이의 값으로 덮어쓰지 않는다(웨이브 12 까지는 덮어썼다).
4. IR: 지금 친 판의 리플레이는 오프셋 0 과 입력 시각 그대로 올라간다. 웨이브 12 까지의 리플레이를 변환하면 전과 같이 각 입력의 시각을 `(그 입력의 오프셋 - 마지막 오프셋)` 만큼 옮긴다(`ir_replay::uploaded_time_us`). 점수 제출의 `options.offset_ms` 는 설정값(노트 표시 오프셋)이다.
5. 설정 파일: 스키마 5. 스키마 4 이하의 파일과 계정에서 내려받은 설정은 읽을 때 `judge.offset_ms` 의 부호를 한 번 뒤집는다(`Config::sanitise`). 전의 +26(입력을 26 ms 늦게 판정)은 지금의 -26(노트를 26 ms 늦게 그림)이고, 눈에 보이는 노트와 PGREAT 사이의 간격이 그대로다. 이전 파일은 `settings.ron.v4.bak` 로 남는다.
6. 키 설정의 행 이름: `JUDGE OFFSET +1 MS (HITTING LATE)`, `JUDGE OFFSET -1 MS (HITTING EARLY)`. 저장된 키 지정(`offset_up`·`offset_down`)은 그대로 숫자를 +1·-1 한다. 선곡 패널 3 의 5·7번 키도 원본과 같은 방향이다(5번 -1, 7번 +1). 기본 스킨은 그 두 칸의 동작 줄을 `NOTES 1 MS LATER`(5번), `NOTES 1 MS EARLIER`(7번)로 쓴다(웨이브 12 까지는 `1 MS EARLIER`·`1 MS LATER` 였다. 무엇이 옮겨지는지 적지 않아 PLAY 메뉴의 도움말과 반대로 읽혔다).
7. 같은 값을 가리키는 문구는 한 방향이다: PLAY 메뉴 도움말 `Draw the notes this many ms earlier (+) or later (-) than the song; raise it if you hit late, lower it if you hit early`, 키 설정의 `+1 MS (HITTING LATE)`·`-1 MS (HITTING EARLY)`, 선곡 패널 3 의 `NOTES 1 MS EARLIER`(+1)·`NOTES 1 MS LATER`(-1), 플레이 화면의 TIMING 숫자(설정값 그대로, 부호 포함), 자동 보정(늦게 치면 +). FAST/SLOW 는 입력이 노트의 시각(소리)보다 빨랐는지 늦었는지이고 오프셋과 무관하다.

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
| 사용자 지정 범위 | 레인과 스크래치 역방향, 플레이 조작 20종, 선곡 단축키 10종(난이도표 다시 받기 `F6` 포함), 화면 여는 키 6종(PLAY 메뉴 `F8` 포함), 스킨 브라우저 숫자 키 8종, 결과 2종, 리플레이 분석 5종. 키보드와 컨트롤러 둘 다(옵션 오버레이 두 키와 상세 패널 홀드 키는 키보드 전용) |
| 고정 키 | `Esc`, `Enter`, 화살표, `H`(키 안내)와 각 메뉴 안의 키 |
| 레인 커버 조작 | `RIGHT` = 더 가림(LANE COVER DOWN), `LEFT` = 덜 가림(LANE COVER UP), `BACKSLASH` = 켬·끔. 커버가 꺼져 있을 때 값을 바꾸면 발치의 COVER 수치가 1.5초 동안 뜬다 |
| 미세 단계 | FINE STEP 키를 누르고 있는 동안 가림막은 미세 단계로, 유지 중인 녹색 숫자는 ±1 로 움직인다. 키를 주지 않았으면 레인으로 쓰지 않는 Shift 가 그 역할을 한다(7키 기본 배치에서는 오른쪽 Shift) |
| 지금 설정된 키 보기 | 선곡에서 `H` |
| 난이도표 다시 받기 | 선곡에서 `F6`(RELOAD DIFFICULTY TABLES, 설정 파일 키 `reload_tables`, 키 설정 화면에서 바꿀 수 있고 패드에도 줄 수 있다). 등록한 표를 전부 다시 받는다. 검색어를 치는 중에는 듣지 않는다. 표 하나만 받으려면 Tables 화면(`T`)에서 `R`, 전부는 `A`. 저장과 다시 받기의 규칙은 `README.md` 의 Difficulty tables 절 |
| 저장 | `~/.config/rbms/keyconfig.ron`. 이전 파일은 그대로 읽히고 없는 항목은 기본 키로 채워진다. 단, 파일에 없는 단축키의 기본 키를 같은 화면의 다른 단축키가 파일에서 이미 쓰고 있으면 그 단축키는 미지정으로 읽힌다(사용자가 고른 키가 이긴다. 예: FOLDERS 를 `F8` 로 옮겨 둔 파일에서 PLAY MENU 는 키 없음으로 나오고 키 설정 화면에서 새로 지정한다) |

## 6. 구현 위치

| 내용 | 위치 |
| --- | --- |
| 이동 시간·녹색 숫자·유지 속도의 순수 계산 | `crates/rbms-config/src/speed.rs`(`held_speed`, `HeldTravel`, `HeldSpeed`, `green_of_duration`, `duration_of_green`, `duration_stepped`, `duration_stepped_by_green`, `travel_duration_ms`) |
| 설정 필드 | `crates/rbms-config/src/schema.rs`(`PlayOptions.keep_green_number`, `duration_ms`, `JudgeOptions.offset_ms`, `auto_offset`) |
| 자동 보정의 한 걸음 | `crates/rbms-play/src/session.rs`(`auto_adjust_step_ms`, `PlaySession::judge_offset_ms`, `set_judge_offset_us`). 세션은 오프셋을 판정에 쓰지 않는다 |
| 오프셋만큼 앞서 그리기 | `apps/rbms-player/src/stage/play/mod.rs`(`field_lead_us`, `drawn_chart_us`, `built_in_field_us`. 오프셋을 더하지 않은 플레이 타이머 시각은 `played_chart_us` 이고 BGA 가 이것을 읽는다), 미리보기는 `stage/play/skin_preview.rs`(`PlaySample::drawn_chart_us`) |
| 오프셋의 측정과 캡처 | `apps/rbms-player/src/stage/play/timing_tests.rs`, `timing_capture_tests.rs`(`RBMS_SKIN_CAPTURE_DIR` 을 주면 PNG 저장), `crates/rbms-play/tests/auto_adjust.rs` |
| 플레이 중 조작 | `apps/rbms-player/src/stage/play/controls.rs`, `apps/rbms-player/src/app_input.rs`(`apply_control`, `fine_pressed`) |
| 조작과 단축키의 종류, 저장 형식 | `apps/rbms-player/src/keyconfig.rs`(`ControlAction`, `Shortcut`, `ControlBinds`) |
| IR 리플레이 변환 | `apps/rbms-player/src/ir_replay.rs`(`uploaded_time_us`) |
| PLAY 메뉴(행 목록, 조작, 정보 패널) | `apps/rbms-player/src/stage/play_menu.rs`, `stage/play_menu/rows.rs`(숫자 행의 값과 목록 표시, 행의 종류, 표시 꼬리, 읽기 전용, 큰 단계, 기본값 복원). 숫자 행의 공통 부품은 `apps/rbms-player/src/number_row.rs`, 걸음 모델은 `crates/rbms-config/src/number.rs`·`settings.rs`(`number_spec`) |
| PLAY 메뉴와 SKIN 메뉴가 같이 쓰는 그리기·스크롤 부품 | `apps/rbms-player/src/stage/preview_menu.rs` |
| 미리보기의 대상과 계산 요약 | `apps/rbms-player/src/skin_preview/target.rs`(`PlayTarget`, `SpeedReadout`, `PlayTarget::readout`) |
| 행이 속한 묶음 | `crates/rbms-config/src/settings.rs`(`SettingTab::PLAY_MENU` = `PlaySpeed`·`PlayLane`·`PlayJudge`·`PlayChart`, `SettingTab::SHOWN` = Settings 의 탭 6개) |
| 화면을 지정하는 스킨 옵션 통로 | `apps/rbms-player/src/skin_select.rs`(`rows_for`, `line_for`, `step_for`, `reset_property_for`) |
| 연습 패널(9절) | 값의 범위·제약·저장은 `apps/rbms-player/src/practice.rs`(`PracticePanel::number`·`set_number`·`reset_number`·`cycle`, `PracticeBook`), 화면과 조작은 `apps/rbms-player/src/stage/practice.rs`. 숫자 행의 부품과 모델은 PLAY 메뉴와 같은 `number_row.rs`·`rbms_config::NumberSpec`(시각 행은 `NumberSpec::as_clock`) |

## 7. PLAY 메뉴

플레이할 때마다 만지는 옵션은 SETTINGS 가 아니라 독립 화면인 PLAY 메뉴에 있다. 왼쪽이 묶음별 행 목록, 오른쪽이 고른 모드의 플레이 화면을 그 모드의 스킨으로 그린 미리보기이고, 값을 바꾸면 그 프레임부터 미리보기에 반영된다. 저장 위치와 형식은 그대로다(`settings.ron` 의 `play`·`judge` 그룹과 `skin.custom`. 이전 판의 설정 파일이 그대로 읽힌다. PLAY 메뉴 때문에 스키마 판이 오르지는 않았고, 판정 오프셋의 스키마 5 는 3.3절).

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
| SPEED | GREEN NUMBER | 목록 `300`, 정보 패널 `300 (500 MS)`: 녹색 숫자와 이동 시간. 읽기 전용일 때 정보 패널 `480 (800 MS) AUTO` | 1 / 10. 범위 1~6000 |
| SPEED | HI-SPEED | `2.00x`. 읽기 전용일 때 정보 패널 `2.24x AUTO` | 0.01 / 0.10. 범위 0.01~20.00. HI-SPEED STEP 은 플레이 중 키와 선곡 옵션 패널의 걸음이고 이 메뉴의 한 칸이 아니다 |
| SPEED | HI-SPEED STEP | `0.25x` | 0.01 / 0.10. 범위 0.01~1.00 |
| SPEED | HI-SPEED FIX | OFF·START·MAX·MAIN·MIN | 한 항목 |
| SPEED | SPEED FIX | FLOATING·CONSTANT | 뒤집기 |
| LANE | LANE COVER ON, LIFT ON, HIDDEN+ ON | `ON` / `OFF` | 뒤집기 |
| LANE | LANE COVER | 목록 `30.0%`, 정보 패널 `30.0% (WHITE 300)`: 비율과 흰색 숫자 | 0.1%(흰색 숫자 1) / 1%. 범위 0~100% |
| LANE | LIFT, HIDDEN+ | `15.0%` | 0.1% / 1%. 범위 0~100% |
| LANE | COVER FINE STEP | 목록 `0.0010`, 정보 패널 `0.0010 (WHITE 1.0)`: 비율과 그 한 칸이 흰색 숫자로 얼마인지 | 0.0001 / 0.0010. 범위 0.0001~0.0100 |
| LANE | 미리보는 플레이 스킨 문서가 선언한 옵션(기본 스킨: Lane Width, Field Side, Scratch Side, Judge Detail, Score Difference, Judge Display, Score Graph) | 항목 이름 | 한 항목. 그 문서의 커스터마이즈(`skin.custom`)에 저장되고 문서를 다시 읽는다. SKIN 메뉴의 같은 행과 같은 값이다. 파일 칸은 SKIN 메뉴에만 있다 |
| LANE | 그 문서가 선언한 위치·크기 오프셋(기본 스킨: `JUDGE POSITION Y`). 옵션 바로 뒤 | `+0` | 1 / 10(픽셀. 각도 축은 도). 범위 ±2000. 문서를 다시 읽지 않고 다음 프레임의 미리보기에 반영된다. SKIN 메뉴의 같은 행과 같은 값이다 |
| JUDGE | JUDGE OFFSET | `+5 MS` | 1 ms / 10 ms. 범위 -200~+200 ms |
| JUDGE | AUTO ADJUST | `ON` / `OFF` | 뒤집기 |
| CHART | RANDOM, GAUGE, TARGET, ESCAPE | 항목 이름 | 한 항목 |
| CHART | LEGACY NOTE, AUTOPLAY, SCRATCH AUTO, AUTO REPLAY | `ON` / `OFF` | 뒤집기 |
| SKIN | 접이 `OPACITY` | `HIDDEN (18)` / `SHOWN (18)`: 접힘 여부와 든 행 수 | Enter·← →·클릭으로 펴고 접는다. 펴면 그 문서의 불투명도 행이 바로 아래에 나온다(기본 스킨 7키 18행, 14키 17행. 웨이브 13 에서 Key Display Opacity 행이 없어져 하나씩 줄었다) |
| SKIN | 불투명도 행(접이를 폈을 때. 기본 스킨: `LANE BACKGROUND OPACITY` 등) | `100%` | 1% / 10%. 슬라이더와 ← → 는 0~100%. 0% 면 그 부위가 사라진다. 0 키로 100%. 직접 입력만 200% 까지 받는다(알파 0 의 덮개를 밝히는 스킨용. `docs/skin.md` 2.2절의 불투명도 행 주의) |
| SKIN | 접이 `POSITION AND SIZE` | `HIDDEN (15)` / `SHOWN (15)` | 엔진이 모든 플레이 스킨에 붙이는 오프셋 네 개의 행(`ALL OFFSET(%) X` … `JUDGE DETAIL OFFSET ALPHA`). 1 / 10 |

- KEEP GREEN NUMBER 가 켜져 있으면 GREEN NUMBER 가 조절 대상이고 HI-SPEED 는 예시 BPM 에서 계산된 값을 보이는 읽기 전용 행이다(범위 끝에 걸리면 정보 패널에 `20.00x (LIMIT) AUTO`). 꺼져 있으면 반대로 HI-SPEED 가 조절 대상이고 GREEN NUMBER 는 그 속도가 예시 BPM 에서 내는 녹색 숫자와 이동 시간을 보인다. 읽기 전용 행은 이름·값·슬라이더가 모두 흐리고(슬라이더는 계산된 값의 자리에 선다), 좌우·Enter·0·클릭·끌기·직접 입력에 움직이지 않으며, 정보 패널이 값 뒤에 `AUTO` 를 붙이고 이유를 말한다. 유지 중에는 저장된 HI-SPEED 가 쓰이지 않으므로 보이지 않는다(끄면 다시 보인다).
- LANE COVER·LIFT·HIDDEN+ 는 0.1% 격자 위에서 움직인다(`rbms_config::set_number_steps` 가 값을 걸음 수로 세어 `걸음 / 1000` 을 저장한다. 더해 가지 않으므로 천 번 옮겨도 오차가 쌓이지 않는다). 30.0% 는 흰색 숫자 300 이고 한 칸이 흰색 숫자 1 이다. 저장값은 스킨이 천분율로 잘라 읽어도 한 칸 모자라게 읽히지 않는 쪽으로 고른다(25.1% 는 250 이 아니라 251). 플레이 중 미세 단계로 격자 밖에 둔 값은 가장 가까운 칸으로 읽히고, 한 칸 움직이면 거기서 한 칸 간 값이 된다. 선곡 옵션 패널과 스킨 이벤트의 한 칸은 전과 같은 1% 다(`rbms_config::adjust`).
- SKIN 묶음은 목록의 마지막 묶음이고 두 접이는 닫힌 채로 열린다(접이 상태는 저장되지 않는다). 접이를 펴면 든 행이 화면에 들어오도록 목록이 스크롤된다: 다 들어가면 전부, 넘치면 접이 행이 목록 첫 줄에 온다. 스킨이 불투명도나 엔진 오프셋을 갖지 않으면 그 접이는 없고, 둘 다 없으면 묶음도 없다. 불투명도와 오프셋은 SKIN 메뉴의 같은 행과 같은 곳(그 문서의 `skin.custom` 오프셋)에 저장된다.
- 이름이 비슷한 세 가지는 다른 값이다. JUDGE 묶음의 `JUDGE OFFSET`(ms)은 판정 타이밍이다. LANE 묶음의 `JUDGE POSITION Y` 는 기본 스킨이 판정 글자·콤보·위 줄을 함께 옮기는 오프셋이다(8절). 접이 안의 `JUDGE OFFSET X/Y/W/H/ALPHA` 와 `JUDGE DETAIL OFFSET …` 은 엔진이 스킨과 무관하게 붙이는 오프셋으로, 판정 글자와 그 위 줄을 따로 옮긴다.
- LANE OPTION 은 아직 플레이에 적용되지 않아 어디에도 보이지 않는다(값은 저장·복원된다).
- 큰 단계가 범위를 넘으면 범위 끝까지만 간다(HI-SPEED 는 0.01~20.00).
- JUDGE OFFSET 의 한 칸은 선곡 패널 3·플레이 중 키와 같은 1 ms 다(SETTINGS 의 옛 행은 5 ms 였다).

### 7.3 조작

| 입력 | 동작 |
| --- | --- |
| ↑ ↓ | 행 커서 이동. 묶음 이름은 건너뛴다. 목록은 17칸씩 보이고 넘치면 스크롤 막대와 `현재 / 전체`(묶음 이름을 포함한 칸 수)가 나온다 |
| PageUp PageDown | 한 화면(17칸)씩 |
| ← → | 커서 행의 값을 한 칸. 숫자 행은 가장 잔 단위 한 칸이다 |
| Shift + ← → | 숫자 행(설정 행, 스킨 오프셋, 불투명도)이면 열 칸. 켬·끔과 목록 행, 스킨 옵션 행, PREVIEW BPM, 접이는 한 칸 그대로다 |
| Enter | 숫자 행에서는 숫자를 직접 친다(아래). 그 밖의 행에서는 → 한 칸과 같고, 접이 행에서는 펴거나 접는다. 화면을 떠나지 않는다 |
| 0 (숫자 줄, 넘패드) | 커서 행을 기본값으로(새 설정 파일의 값. PREVIEW BPM 은 150, 스킨 옵션은 문서의 기본 항목, 스킨 오프셋은 0, 불투명도는 100%). 접이는 움직이지 않는다 |
| Tab / Shift+Tab | 미리보기 위 모드 띠(7K 5K 14K 10K 9K)에서 다음 / 이전 |
| 행 왼쪽 클릭 / 오른쪽 클릭 | 그 행으로 커서를 옮기고 한 칸 앞으로 / 뒤로. 숫자 행은 이름 쪽을 왼쪽 클릭하면 한 칸 앞으로, 행 어디든 오른쪽 클릭하면 한 칸 뒤로다 |
| 모드 띠의 탭 왼쪽 클릭 | 그 모드로 |
| 휠 | 포인터가 목록 위일 때 목록을 스크롤한다(커서는 그대로). 슬라이더 위에서도 스크롤만 하고 값은 바뀌지 않는다 |
| 숫자 행의 ← → | 그 행의 가장 잔 단위 한 칸(7.2 표). Shift 를 누르고 있으면 열 칸. 범위 끝에서 멈춘다 |
| 숫자 행의 Enter, 또는 숫자 글자 클릭 | 그 자리에서 숫자를 친다. 지금 값이 채워져 있고 전체가 선택되어 있어 첫 글자가 값을 갈아 쓴다(← → Home End 로 선택을 풀고 고친다). 받는 글자는 숫자·부호·소수점이고 단위(%, ms, x, px)는 쳐도 버린다. 퍼센트 행은 퍼센트 수를 친다 |
| 입력 중 Enter | 확정. 범위를 벗어나면 끝값, 가장 잔 단위로 반올림. 숫자가 아니면(빈 칸, `-`, `1.2.3`) 값을 바꾸지 않고 행에 1.5초 동안 `NOT A NUMBER` 를 보인다. 지금 값과 같은 수를 확정하면 저장값을 건드리지 않는다 |
| 입력 중 Esc | 입력만 취소한다(메뉴를 떠나지 않는다). 그 다음 Esc 가 저장하고 나간다 |
| 입력 중 ↑ ↓ PageUp PageDown Tab, 마우스 누름 | 친 값을 확정한 뒤 그 조작을 한다. 그 밖의 키는 입력이 삼킨다(0 키는 숫자 0 이다) |
| 슬라이더 누름·끌기 | 트랙을 누르면 누른 곳의 값으로 가고, 버튼을 뗄 때까지 포인터를 따라간다(포인터가 행·목록 밖으로 나가도 이어진다. 슬라이더 끝을 넘으면 슬라이더의 끝값). 손잡이를 누르면 값은 그대로이고, 잡은 자리를 기준으로 포인터가 움직인 만큼만 따라온다(누르기만 하고 떼면 아무것도 바뀌지 않는다). 끄는 동안 미리보기가 다음 프레임부터 따라오고, 설정 파일은 버튼을 뗄 때 한 번 쓴다. 끄는 중에 키를 누르면 끌기가 거기서 끝난다(값은 그대로 저장된다) |
| 미리보기와 묶음 이름 위의 클릭 | 아무 일도 없다 |

네 화면(PLAY 메뉴, SKIN 메뉴, SETTINGS, 연습 패널) 밖의 숫자 조정은 키만 받고 걸음도 그대로다. 플레이 중 조작과 맞춘 걸음이기 때문이다. 연습 패널은 웨이브 12 에서 네 번째 화면이 됐다(9절).

| 곳 | 걸음 | 까닭 |
| --- | --- | --- |
| 선곡의 옵션 패널 1~3, 스킨 이벤트, 플레이 중 키 | HI-SPEED 는 HI-SPEED STEP(기본 0.25), 가림막 1%(미세 단계 키를 누르면 COVER FINE STEP), 판정 오프셋 1 ms | 플레이 중 한 번 누름으로 움직이는 폭이다. 잔 값은 PLAY 메뉴에서 맞춘다(`rbms_config::adjust` 가 이 걸음을 갖고, 세 화면은 `set_number_steps` 통로를 탄다) |

### 7.4 미리보기와 정보 패널

| 항목 | 내용 |
| --- | --- |
| 모드 띠와 PREVIEW BPM | 미리보기의 예시 차트를 고른다. 설정 파일에 저장되지 않고 SKIN 메뉴가 고른 화면(`skin.screen`)도 바꾸지 않는다. 메뉴를 열 때마다 7K·150 BPM 이다 |
| 녹색 숫자 유지를 눈으로 보기 | KEEP GREEN NUMBER 를 켜고 PREVIEW BPM 을 바꾸면 노트가 내려오는 시간과 녹색 숫자는 그대로이고 HI-SPEED 표시가 바뀐다. 끄면 HI-SPEED 가 그대로이고 녹색 숫자와 노트 속도가 바뀐다 |
| 정보 패널 윗줄(요약) | 다섯 칸이 나란히 있고 칸마다 작은 이름 아래 큰 숫자가 있다: `BPM 150`, `HI-SPEED 2.24 (HELD)`, `TRAVEL 500 MS`, `GREEN 300`(초록), `WHITE 300`. 읽는 법은 아래 표 |
| 계산 | 플레이 화면과 같은 함수다(`rbms_config::held_speed`, `travel_duration_ms`, `green_of_duration`). 미리보기 스킨의 HI-SPEED·GREEN·레인 커버 숫자와 같은 값이 나온다 |
| 정보 패널 아래 줄 | 커서 행의 이름·값 전문과 설명 |
| 문서가 없는 모드, 읽기 실패 | 미리보기 자리에 이유를 글로 보인다(`NO SKIN DOCUMENT FOR THIS SCREEN` 등). 행은 그대로 조절된다 |
| 예시 판정 | 사용자의 JUDGE OFFSET 과 무관하고 그 값을 바꾸지도 않는다(AUTO ADJUST 가 켜져 있어도). 오프셋은 판정을 옮기지 않기 때문이다. 소리는 내지 않는다 |
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
| JUDGE OFFSET, AUTO ADJUST | 노트가 오프셋만큼 앞서(음수면 늦게) 그려진다. 예시 손이 치는 시각은 그대로라 +면 노트가 판정선에서 사라진 뒤에 폭발과 판정 글자가 뜨고, -면 폭발이 뜬 뒤에도 노트가 판정선까지 내려온다. 스킨의 TIMING 숫자와 `TIMING AUTO` 표지도 따른다. 예시 판정은 달라지지 않는다 |
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
- 레인 면·레인 구분선·마디선 같은 부위를 지워 노트와 판정만 남기는 법은 `docs/skin.md` 3.4.2절(부위별 불투명도)에 있다.
- 판정선은 필드의 맨 아래에 붙어 있다(웨이브 13). 판정선 아래에 레인이 이어지던 건반 표시 칸은 없어졌고, 판정 글자와 위 줄은 필드와 함께 64px 내려왔다(판정선과의 거리는 전과 같다). 판정선을 올리려면 LIFT 를 쓴다. 좌표는 기본 스킨 설계서 6.3·6.4절.

## 9. 연습 패널(PRACTICE)

선곡에서 연습 키(기본 `F4`)로 연다. 차트를 플레이와 같은 길로 읽은 뒤 뜨는 내장 화면이고, 구간 플레이는 이 화면 위에 열렸다가 이 화면으로 돌아온다. 스킨이 그리는 플레이 화면 위에 겹쳐 뜨는 경로는 없다(스킨의 연습 상태 옵션 1080 은 항상 꺼져 있다). 포인터 좌표는 다른 내장 화면과 같은 통로(창 좌표 → 1280x720 논리 좌표, 레터박스 포함)를 탄다.

웨이브 12 에서 숫자 행 여섯을 PLAY 메뉴와 같은 방식(7.3절)으로 바꿨다: 슬라이더, 직접 입력, 가장 잔 단위의 ← →. 원본 `PracticeConfiguration` 의 굵은 걸음(TOTAL 5, 재생 속도 5%, 터보 키의 2.5초·25)은 없어졌다. 값의 범위와 값 사이의 제약은 원본 그대로다.

### 9.1 행

| 행 | 종류 | 범위 | 가장 잔 단위(← →) | Shift + ← → | 전의 한 칸 / Shift(터보) | 0 키(기본값) |
| --- | --- | --- | --- | --- | --- | --- |
| START | 동작 | — | — | — | 없던 행 | — |
| START TIME | 숫자(시각) | 0:00.0 ~ 마지막 타임라인 2초 전(0.1초로 내림. **차트마다 다름**) | 0.1초 | 1초 | 0.1초 / 2.5초 | 0:00.0 |
| END TIME | 숫자(시각) | START TIME + 1초(0.1초로 내림. **START TIME 에 따라 다름**) ~ 마지막 타임라인 1초 뒤(**차트마다 다름**) | 0.1초 | 1초 | 0.1초 / 2.5초 | 차트 끝(상한) |
| GAUGE TYPE | 고르기(9종, 돌아감) | ASSIST EASY ~ EXHARD GRADE | 한 칸 | 한 칸 | 그대로 | 움직이지 않음 |
| GAUGE CATEGORY | 고르기(5종, 돌아감) | 5KEYS 7KEYS PMS KEYBOARD LR2 | 한 칸 | 한 칸 | 그대로 | 움직이지 않음 |
| GAUGE VALUE | 숫자 | 1 ~ 고른 게이지의 상한(**GAUGE TYPE 과 GAUGE CATEGORY 에 따라 다름**. 예: 7KEYS NORMAL 100, PMS NORMAL 120) | 1 | 10 | 1 / 10 | 20 |
| JUDGERANK | 숫자 | 1 ~ 400(판정 폭 %) | 1 | 10 | 1 / 25 | 100 |
| TOTAL | 숫자 | 10 ~ 5000. **슬라이더와 ← → 는 10 ~ 1000, 그 위는 직접 입력** | 1 | 10 | 5 / 25 | 차트의 TOTAL |
| FREQUENCY | 숫자 | 50 ~ 200(재생 속도 %) | 1 | 10 | 5 / 25 | 100 |
| OPTION-1P | 고르기(돌아감) | OFF ~ ALL-SCRATCH(스크래치 없는 모드는 ALL-SCRATCH 제외) | 한 칸 | 한 칸 | 그대로 | 움직이지 않음 |

- 켬·끔 행은 없다. 원본의 OPTION-2P, OPTION-DP, GRAPHTYPE 행은 이 플레이어에 읽는 곳이 없어 두지 않았다(전과 같다).
- 시각은 `분:초.십분의 일초` 로 보인다(`1:05.5`). 원본과 같이 0.1초 격자다(`TIME_ROUND_MS` 100).
- 값 사이의 제약은 키·끌기·직접 입력 어느 길로 움직여도 같다. END TIME 은 START TIME + 1초 아래로 내려가지 않고 거기서 멈춘다(슬라이더를 더 끌어도, 더 작은 수를 쳐도 그 값). START TIME 을 올려 END TIME 에 닿으면 END TIME 을 1초 간격으로 밀고 올라간다. 범위 밖의 수를 치면 가까운 끝값이 된다.
- START TIME 이 민 END TIME 이 돌아오는지는 조작에 따라 다르다. **키와 직접 입력**은 원본대로다: 한 번 밀린 END TIME 은 START TIME 을 다시 내려도 밀린 자리에 남는다(원본의 비대칭). **슬라이더를 끄는 동안**에는 남지 않는다: END TIME 은 슬라이더를 잡았을 때의 값과 START TIME + 1초 가운데 큰 쪽이고, START TIME 이 그 아래로 돌아오면 잡았을 때의 값으로 돌아온다. 끌기는 지나가는 자리를 전부 값으로 쓰므로, 포인터가 한 번 지나친 것만으로 맞춰 둔 구간 끝이 사라지지 않게 한 것이다. 버튼을 뗀 자리의 값은 그대로 남고, 다음 끌기는 그 값에서 시작한다(트랙의 END TIME 오른쪽을 눌렀다 떼면 END TIME 은 밀린 채다).
- TOTAL 의 슬라이더와 ← → 는 1000 에서 멈춘다(`TOTAL_SLIDER_MAX`). 차트의 TOTAL 은 대개 수백이라 10 ~ 5000 전체를 슬라이더에 펴면 1픽셀이 24 가 되어 쓸 수 없기 때문이다(1000 까지면 1픽셀이 5 미만). 1000 을 넘는 값은 직접 입력으로 넣고, 그렇게 넣은 행은 손잡이가 슬라이더 끝에 서며 ← → 가 그 자리에서 양쪽으로 한 칸씩 움직인다(SKIN 메뉴의 불투명도 행과 같은 규칙).
- END TIME 의 슬라이더 눈금은 `0:01.0 ~ 차트 끝`으로 고정이다. START TIME 이 움직여도 눈금이 바뀌지 않고, START TIME + 1초보다 왼쪽은 손잡이가 따라오지 않는 구간이다.
- GAUGE TYPE 이나 GAUGE CATEGORY 를 바꾸면 GAUGE VALUE 의 상한이 그 게이지의 것으로 바뀐다(전과 같이 CATEGORY 를 바꾸면 값은 그 게이지의 초기값이 되고, PMS 차트에서 생존 게이지로 가면 100 으로 눌린다).
- 2초보다 짧은 차트는 START TIME 이 0:00.0 한 칸뿐이다. 원본은 이때 상한을 음수로 계산해 START TIME 을 음수로 보냈고 전의 구현도 그대로 따랐는데, 차트 앞의 구간을 가리키는 값이라 0 에서 멈추게 고쳤다.
- 행이 서 있는 걸음과 같은 값으로 놓으면(손잡이를 누르기만 함, 보이는 수를 그대로 확정) 저장값을 건드리지 않는다. 차트의 TOTAL 이 260.5 처럼 소수면 행은 260 으로 보이고, 행을 움직이기 전에는 260.5 가 그대로 쓰인다. 0 키도 260.5 로 되돌린다.

### 9.2 조작

| 입력 | 동작 |
| --- | --- |
| ↑ ↓, Tab / Shift + Tab | 행 커서(Tab 은 ↓, Shift + Tab 은 ↑ 와 같다). 끝에서 반대쪽으로 돌아간다. 패널을 열면 커서는 START 행에 있다 |
| ← → | 숫자 행은 가장 잔 단위 한 칸, 고르기 행은 다음 / 이전. START 행에서는 아무 일도 없다 |
| Shift + ← → | 숫자 행은 열 칸. 고르기 행은 한 칸 그대로 |
| Enter | **숫자 행에서는 직접 입력을 연다. 그 밖의 행(START, 고르기 행)에서는 구간 플레이를 시작한다.** 전에는 어느 행에서나 시작이었다 |
| 0 (숫자 줄, 넘패드) | 숫자 행을 기본값으로(9.1 표). 고르기 행과 START 행은 그대로 |
| 직접 입력 | PLAY 메뉴와 같다(7.3절): 지금 값이 전체 선택된 채 열리고, Enter 확정, Esc 는 입력만 취소(패널을 닫지 않는다), ↑ ↓ Tab 과 마우스 누름은 확정한 뒤 동작(입력 중 안내 줄의 `UP DOWN TAB SET AND MOVE ON` 그대로다), 숫자가 아니면 `NOT A NUMBER` 를 1.5초. 시각 행은 초(`90`, `61.2`) 또는 분:초(`1:30`, `1:30.5`, `2:5`)로 친다. 초가 60 을 넘으면 분으로 올려 읽는다(`1:75` = 2:15.0). 콜론은 시각 행에서만 받는다 |
| 슬라이더 누름·끌기 | PLAY 메뉴와 같다: 트랙을 누르면 그 값으로, 손잡이를 누르면 값은 그대로이고 움직인 만큼만. 포인터가 행 밖으로 나가도 버튼을 뗄 때까지 이어지고, 끄는 중에 키를 누르면 끌기가 끝난다 |
| 행 왼쪽 클릭 | 커서를 옮기고 한 칸 앞으로(숫자 행은 이름 쪽, 숫자 글자를 누르면 직접 입력, 슬라이더를 누르면 끌기). START 행은 구간 플레이 시작 |
| 행 오른쪽 클릭 | 커서를 옮기고 한 칸 뒤로(숫자 행과 고르기 행. 행 어디를 눌러도). START 행은 커서만 |
| 휠 | 아무 일도 없다(스크롤할 목록이 없고 값은 바꾸지 않는다) |
| Esc | 선곡으로 돌아간다. 숫자를 치는 중이면 입력만 취소한다 |
| 패드(건반·턴테이블) | **받지 않는다.** 원본은 연습 화면을 건반과 턴테이블로 조작하지만(`PracticeConfiguration.java:264-475`), 이 패널은 전에도 지금도 패드 이벤트를 받는 경로가 없다(`PracticeState` 에 `handle_pad` 가 없다). 그래서 원본의 패드용 걸음(턴테이블 아날로그 TOTAL 1·터보 20, 재생 속도 1%·터보 10%, 구간 터보 1초)은 쓰이는 곳이 없어 코드에서 뺐다. 패드 조작을 넣을 때의 원본 걸음은 `PracticeConfiguration.java:531-611` 에 있다 |

### 9.3 저장과 반영

| 항목 | 내용 |
| --- | --- |
| 저장 위치 | 설정 파일이 아니라 그 옆의 `practice.ron`(차트 md5 별). 형식은 그대로다(밀리초·정수·`total` 실수). 이전 파일이 그대로 읽힌다 |
| 언제 쓰나 | 패널을 떠날 때 한 번(구간을 시작할 때와 Esc 로 나갈 때). 끌기나 입력 한 번마다 쓰지 않는다. 패널에 떠 있는 값은 그 방문의 상태다 |
| 플레이에 반영 | 구간을 시작하는 순간의 값으로 구간을 자른다(`PracticePanel::start` → `PracticeSession`: 구간 시작·끝, 게이지와 시작량, 판정 폭, 재생 속도, TOTAL, 배치). 돌고 있는 구간에는 반영되지 않고 다음 시작부터다. 반영 경로는 전과 같다 |
| 기록 | 연습은 기록·IR 전송을 하지 않는다(전과 같다) |
