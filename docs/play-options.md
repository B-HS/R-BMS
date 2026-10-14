# 플레이 옵션

> 최종 갱신 2026-10-14 · 대응 커밋 7095ef7. 반영한 변경: 2026-10-15 S19 의 기록 정의(규칙 세대 4: avgjudge·BP·코스 스테이지 콤보·풀콤·TOTAL 어시스트, Esc 판은 모두 저장과 코스 도중 Esc: 10.2절의 FAILED·TOTAL·Esc 행, 새 10.11절), 0.1.4 의 기록 동기화와 Esc 포기 판 저장(10.1·10.2·10.4·10.8절과 새 10.9절. 아직 커밋 전), 웨이브 20(점수 서버가 SCRATCH AUTO 판을 순위에 올리되 기본 목록에서는 숨긴다: 10.2절의 SCRATCH AUTO·코스 행. 결과 화면은 그 순위를 `IR: RANK #n (SCRATCH AUTO)` 로 쓴다), 웨이브 18(SCRATCH AUTO 의 새 규칙과 모든 판의 저장·전송: 10절, 같은 배치로 다시 하기: 10.7절), 웨이브 16(기록이 저장되는 조건과 SCRATCH AUTO: 10절, 목표 점수와 TARGET FALLBACK·TARGET RATE: 11절, 리플레이를 본 뒤 설정이 그대로 남는 것: 10.2절), 웨이브 14(플레이 중 조작: START + 턴테이블의 1 단위 걸음과 리프트 조합, 판정 오프셋의 출하 키와 누름 반복, 스크래치 역방향 키의 출하: 2절의 "바꾸는 곳" 열, 5절), 웨이브 13(판정 오프셋을 원본의 노트 표시 타이밍으로: 1~3절, 7.4절. 기본 스킨의 판정선을 필드 맨 아래로: 8절), 웨이브 12(연습 패널의 숫자 행: 9절)
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
| LANE COVER | 위에서 내려오는 가림막. 값이 커질수록 더 가린다 | 비율 0~1, 지정 키는 한 번에 0.01, 미세 단계 0.0001~0.01(기본 0.001). START + 턴테이블은 흰색 숫자 1(0.001) | 0, 켬 | PLAY 메뉴 > LANE, 플레이 중 `RIGHT`(더 가림)/`LEFT`(덜 가림), `BACKSLASH`(켬·끔), START + 턴테이블(흰색 숫자 1씩, 5.2절), START 두 번(켬·끔), 마우스 휠 | `PlayConfig.lanecover`, `enablelanecover`, IIDX 의 SUDDEN+ | 켬·끔 전용 키와 방향을 적은 행 이름을 더했다. START + 턴테이블의 걸음과 반복, 녹색 숫자 유지는 5.2절 |
| LIFT | 판정선을 올린다 | 비율 0~1. START + SELECT + 턴테이블은 1(0.001) | 0, 끔 | PLAY 메뉴 > LANE, 플레이 중 `RBRACKET`/`LBRACKET`, START + SELECT + 턴테이블(1씩, 5.2절), 켬·끔 키(출하 시 미지정) | `PlayConfig.lift`, IIDX 의 LIFT | 원본에는 리프트만 움직이는 조합이 없다(레인 커버를 꺼야 START + 턴테이블이 리프트로 간다). START + SELECT + 턴테이블을 더했다 |
| HIDDEN+ | 판정선에서 위로 올라오는 가림막 | 비율 0~1 | 0, 끔 | PLAY 메뉴 > LANE, 플레이 중 올림·내림·켬·끔 키(출하 시 미지정) | `PlayConfig.hidden`, IIDX 의 HIDDEN+ | 없음 |
| JUDGE OFFSET | 판정 오프셋 | -200~+200 ms, PLAY 메뉴의 행은 1 ms 씩(Shift 10 ms) | 0 | PLAY 메뉴 > JUDGE, 선곡 패널 3 의 5·7번 키(±1 ms), 플레이 중 `9`(-1 ms)/`0`(+1 ms. 누르고 있으면 반복, 5.3절) | `PlayerConfig.judgetiming`(`LaneRenderer.java:300-301`) | 뜻과 부호는 같다. 범위만 좁다(원본 -500~+500 ms). 원본은 플레이 중에 바꾸는 키가 없다 |
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
5. 판정 하나에 한 번만 움직인다. 일반 롱노트(LN)는 머리를 누른 입력이 아니라 끝의 판정(머리 판정과 끝 판정 중 나쁜 쪽)에서 한 번 움직이고, CN·HCN 은 머리와 끝이 각각 한 번이다(`JudgeManager.java:426-438, 697-703`). 끝까지 누르고 있다가 시계가 확정한 롱노트도 그 확정에서 한 번 움직인다. 전에는 일반 롱노트의 머리를 누를 때도 움직여서 롱노트마다 두 번 움직였고, 결과 화면의 판정 분포·FAST/SLOW 도 롱노트를 두 번 셌다.
6. 값은 입력 직후 설정에 반영되고 다음 프레임부터 그 값으로 그려진다. 클리어·폐점·중간에 나가기 어느 쪽으로 끝나도 저장된다.
7. 플레이 화면의 TIMING 숫자(스킨 number 12)가 현재 오프셋이다. 기본 스킨은 켜져 있는 동안 표지를 `TIMING AUTO` 로 쓴다(스킨 image index 75).

### 3.3 리플레이·IR·설정 파일

1. 리플레이: 입력은 들어온 시각에 판정되므로 새로 저장되는 리플레이는 `offset_ms: 0` 이고 `judge.offset_steps` 가 없다. 재생은 보는 사람의 JUDGE OFFSET 과 무관하게 같은 판정이 나온다(오프셋은 노트를 그리는 자리만 옮긴다).
2. 웨이브 12 까지의 리플레이: `offset_ms` 와 `judge.offset_steps` 는 "입력이 판정된 시각이 입력 시각에서 얼마나 떨어져 있었나"로 읽히고, 기록 당시와 같은 시각에 판정된다. 같은 필드를 같은 코드가 읽으므로 판 구분이 없다(새 리플레이는 그 값이 0 일 뿐이다). 그 빌드가 쓴 파일 셋(`crates/rbms-play/tests/fixtures/replay-wave12-*.ron`)이 그때의 집계 그대로 재생되는지 테스트가 본다.
3. 리플레이를 볼 때 사용자의 JUDGE OFFSET 을 리플레이의 값으로 덮어쓰지 않는다(웨이브 12 까지는 덮어썼다).
4. IR: 지금 친 판의 리플레이는 오프셋 0 과 입력 시각 그대로 올라간다. 웨이브 12 까지의 리플레이를 변환하면 전과 같이 각 입력의 시각을 `(그 입력의 오프셋 - 마지막 오프셋)` 만큼 옮긴다(`ir_replay::uploaded_time_us`). 점수 제출의 `options.offset_ms` 는 설정값(노트 표시 오프셋)이다.
5. 설정 파일: 스키마 5. 스키마 4 이하의 파일과 계정에서 내려받은 설정은 읽을 때 `judge.offset_ms` 의 부호를 한 번 뒤집는다(`Config::sanitise`). 전의 +26(입력을 26 ms 늦게 판정)은 지금의 -26(노트를 26 ms 늦게 그림)이고, 눈에 보이는 노트와 PGREAT 사이의 간격이 그대로다. 이전 파일은 `settings.ron.v4.bak` 로 남는다.
6. 키 설정의 행 이름: `JUDGE OFFSET +1 MS (HITTING LATE)`, `JUDGE OFFSET -1 MS (HITTING EARLY)`. 저장된 키 지정(`offset_up`·`offset_down`)은 그대로 숫자를 +1·-1 한다. 웨이브 14 부터 두 키는 `0`·`9` 에 출하된다(5.3절). 선곡 패널 3 의 5·7번 키도 원본과 같은 방향이다(5번 -1, 7번 +1). 기본 스킨은 그 두 칸의 동작 줄을 `NOTES 1 MS LATER`(5번), `NOTES 1 MS EARLIER`(7번)로 쓴다(웨이브 12 까지는 `1 MS EARLIER`·`1 MS LATER` 였다. 무엇이 옮겨지는지 적지 않아 PLAY 메뉴의 도움말과 반대로 읽혔다).
7. 같은 값을 가리키는 문구는 한 방향이다: PLAY 메뉴 도움말 `Draw the notes this many ms earlier (+) or later (-) than the song; raise it if you hit late, lower it if you hit early`, 키 설정의 `+1 MS (HITTING LATE)`·`-1 MS (HITTING EARLY)`, 선곡 패널 3 의 `NOTES 1 MS EARLIER`(+1)·`NOTES 1 MS LATER`(-1), 플레이 화면의 TIMING 숫자(설정값 그대로, 부호 포함), 자동 보정(늦게 치면 +). FAST/SLOW 는 입력이 노트의 시각(소리)보다 빨랐는지 늦었는지이고 오프셋과 무관하다.

### 3.4 입력의 시각과 프레임(S19, [ADR](acknowledge/2026-10-15-adr-event-time-judging.md))

판정은 화면의 프레임과 무관하다. 같은 입력은 60 Hz·144 Hz·프레임 드롭 어디서 쳐도, 그 판의 리플레이를 어느 주사율로 재생해도 같은 판정·게이지·램프가 나온다.

| 무엇 | 언제 |
| --- | --- |
| 키보드 입력 | 창이 키를 넘겨준 순간의 곡 시각(`AppShared::song_us_at`). 그 순간은 프레임 스레드가 그 키를 처리하는 순간과 같다. 창은 프레임(present 대기 포함)이 끝난 뒤에 키를 넘겨주므로, 프레임 스레드가 막혀 있던 시간은 판정에 그대로 남는다(감사 PERF-2, 미해결. 테스트 부채 3.53) |
| 컨트롤러 입력 | gilrs 가 그 변화를 받은 순간(`Event.time`). 예전 빌드(프레임마다 폴링한 순간)보다 60 Hz 에서 평균 약 8 ms, 최대 한 프레임 이르게 판정되므로 예전 빌드에서 맞춘 JUDGE OFFSET 은 다시 맞춘다. 한 프레임 안에 눌렀다 뗀 짧은 탭도 누름과 뗌이 각자 판정된다. 디바운스 16 ms 는 두 변화 사이의 시간으로 재고, 막힌 변화는 창이 열리는 순간에 그때도 그 상태면 받는다. 아날로그 턴테이블만 프레임마다 한 번 읽는다 |
| 놓친 노트(見逃し POOR), 지연 릴리스 확정, 일반 롱노트 끝, 지뢰 | 각자 기한이 된 순간. 지뢰는 그 순간 키가 눌려 있었는지로 깎는다 |
| HCN 게이지 틱 | 쥔(또는 놓은) 시간 200 ms 마다 한 번. 긴 프레임에서도 밀리지 않는다 |
| GAUGE AUTO SHIFT, FAILED | 게이지가 바뀐 모든 순간 뒤. NONE 에서 게이지가 비면 그 순간부터 판정하지 않는다 |
| 결과 화면의 게이지 이력 | 0.5초 경계마다 그 순간의 값 |

1. 같은 순간이면 시간으로 진행되는 판정이 먼저이고 입력이 나중이다. 입력이 그 순간을 기한으로 만든 것(마진 0 인 지연 릴리스)은 그 입력 바로 뒤에 확정한다.
2. 7키·5키·24키는 일반 롱노트 마진이 0 이다. 판정 창 밖에서 일찍 뗀 롱노트는 바로 BAD 로 확정되고, 몇 µs 뒤에 다시 눌러도 되살아나지 않는다. 원본은 한 프레임 안의 떼기를 보지 못해 되살아났다. 채터가 있는 키보드는 롱노트가 더 자주 끊길 수 있다. pop'n 은 마진 200 ms 안의 다시 누르기가 되살린다.
3. 리플레이: 새 리플레이는 `judge.clock: "EVENT"` 다. 이 값이 없는 옛 리플레이는 `FRAME` 으로 읽히고 그대로 재생되지만, 녹화한 기계의 프레임이 정했던 곳(한 프레임 안의 떼기·다시 누르기, 지뢰 앞뒤의 키, HCN 틱, 게이지가 빈 프레임 뒤의 입력)은 저장된 점수와 다르게 나올 수 있다. 웨이브 12 리플레이 셋(3.3절 2번)은 그대로 재생된다.
4. 남는 지연: winit 의 키 이벤트에는 하드웨어 시각이 없어, 창이 present 를 기다리는 동안 OS 에 쌓인 키는 그 대기가 끝나 넘겨받은 순간으로 판정된다. gilrs 의 Windows 백엔드는 자체 스레드가 8~10 ms 마다 폴링해 그만큼 양자화되고, 그 사이의 누름·뗌은 gilrs 안에서 합쳐질 수 있다.
5. 테스트: `crates/rbms-play/tests/replay_reproduction.rs`(`a_run_judges_the_same_at_every_frame_rate_and_so_does_its_replay` 등), `apps/rbms-player/src/stage/play/input_time_tests.rs`, `apps/rbms-player/src/gamepad/tests.rs`.

## 4. 녹색 숫자 유지(KEEP GREEN NUMBER)

1. 켜면 저장되는 값은 하이스피드가 아니라 이동 시간(`play.duration_ms`)이다. 하이스피드는 곡마다 `240000 / 기준 BPM / 이동 시간 * (1 - 레인 커버)` 로 정해진다. 레인 커버가 꺼져 있으면 커버는 0 으로 본다.
2. 기준 BPM 은 HI-SPEED FIX 가 가리키는 BPM(START / MAX / MAIN / MIN)이고, OFF 이면 MAIN(노트가 가장 많은 BPM)이다. CONSTANT 스크롤에서는 필드의 고정 템포 120 이다.
3. 플레이 중 레인 커버를 움직이거나 켜고 끄면 녹색 숫자는 그대로이고 하이스피드가 바뀐다.
4. 리프트와 HIDDEN+ 는 계산에 들어가지 않는다. 스크롤이 리프트로 줄어든 레인 높이에 맞춰지므로 리프트를 바꿔도 이동 시간이 그대로이고, HIDDEN+ 는 판정선 쪽을 가릴 뿐 노트가 나타나는 시각을 바꾸지 않는다(원본의 `currentduration` 도 커버만 반영한다). 실측: 커버 0.3·리프트 0.15 에서 150·200·100 BPM 모두 499.0 ms.
5. 하이스피드가 범위(0.01~20.00)를 넘어야 하는 목표이면 배율은 범위 끝에 머물고, 화면의 녹색 숫자는 실제 값을 보여 준다.
6. 꺼져 있으면 저장된 HI-SPEED 로 스크롤한다(이전과 같다). 이때 선곡 패널 3 의 NOTE DISPLAY TIME 칸은 `OFF` 로 표시되고 4·6번 키는 아무것도 바꾸지 않는다.
7. 속도를 잠그는 코스에서는 유지하지 않는다. 리플레이는 스크롤 설정을 기록하지 않으므로 보는 사람의 설정으로 스크롤한다.

## 5. 키

### 5.1 키 설정

| 항목 | 내용 |
| --- | --- |
| 키 설정 화면을 여는 길 | 선곡에서 `F5`, Settings 의 `KEY CONFIG >` 단추 또는 `F5`, 스킨 브라우저의 숫자 `6` |
| 사용자 지정 범위 | 레인과 스크래치 역방향, 플레이 조작 20종, 선곡 단축키 10종(난이도표 다시 받기 `F6` 포함), 화면 여는 키 6종(PLAY 메뉴 `F8` 포함), 스킨 브라우저 숫자 키 8종, 결과 2종, 리플레이 분석 5종. 키보드와 컨트롤러 둘 다(옵션 오버레이 두 키와 상세 패널 홀드 키는 키보드 전용) |
| 고정 키 | `Esc`, `Enter`, 화살표, `H`(키 안내)와 각 메뉴 안의 키 |
| 레인 커버 조작 | `RIGHT` = 더 가림(LANE COVER DOWN), `LEFT` = 덜 가림(LANE COVER UP), `BACKSLASH` = 켬·끔. 커버가 꺼져 있을 때 값을 바꾸면 발치의 COVER 수치가 1.5초 동안 뜬다 |
| 미세 단계 | FINE STEP 키를 누르고 있는 동안 지정 키의 가림막은 미세 단계로, 유지 중인 녹색 숫자는 ±1 로 움직인다. 키를 주지 않았으면 레인으로 쓰지 않는 Shift 가 그 역할을 한다(7키 기본 배치에서는 오른쪽 Shift). START + 턴테이블의 걸음에는 관여하지 않는다(5.2절) |
| 판정 오프셋 | `9` = -1 ms, `0` = +1 ms(5.3절) |
| 스크래치 역방향 | 스크래치 레인마다 반대 방향 키가 하나 더 있다. 출하 키는 왼쪽 턴테이블 `LCTRL`, 10·14키의 오른쪽 턴테이블 `RCTRL`(5.4절). 선곡에서는 PREVIOUS BAR, 플레이에서는 스크래치의 반대 방향이고 START 와 함께 누르면 가림막을 올린다(5.2절) |
| 지금 설정된 키 보기 | 선곡에서 `H`. 플레이 조작은 IN PLAY, START·SELECT 조합은 IN PLAY, HELD TOGETHER 묶음에 지금 지정된 키로 나온다. 그 묶음의 첫 줄(`LSHIFT LCTRL  THE TURNTABLE: DOWN / UP`)이 아래 줄들의 TURNTABLE 이 키보드의 어느 두 키이고 각각 가림막을 어느 쪽으로 움직이는지 알려 준다 |
| 난이도표 다시 받기 | 선곡에서 `F6`(RELOAD DIFFICULTY TABLES, 설정 파일 키 `reload_tables`, 키 설정 화면에서 바꿀 수 있고 패드에도 줄 수 있다). 등록한 표를 전부 다시 받는다. 검색어를 치는 중에는 듣지 않는다. 표 하나만 받으려면 Tables 화면(`T`)에서 `R`, 전부는 `A`. 저장과 다시 받기의 규칙은 `README.md` 의 Difficulty tables 절 |
| 저장 | `~/.config/rbms/keyconfig.ron`. 이전 파일은 그대로 읽히고 없는 항목은 기본 키로 채워진다. 단, 파일에 없는 단축키의 기본 키를 같은 화면의 다른 단축키가 파일에서 이미 쓰고 있으면 그 단축키는 미지정으로 읽힌다(사용자가 고른 키가 이긴다. 예: FOLDERS 를 `F8` 로 옮겨 둔 파일에서 PLAY MENU 는 키 없음으로 나오고 키 설정 화면에서 새로 지정한다). 나중에 출하 키가 생긴 것(판정 오프셋 두 키, 스크래치 역방향 키)의 규칙은 5.3절과 5.4절 |

### 5.2 플레이 중 START·SELECT 조합

스킨이 그리는 플레이 화면에서 프레임마다 읽는다(`stage/play/controls.rs`). 원본은 `ControlInputProcessor.input`. START 의 키보드 기본값은 `A`, SELECT 는 `W`, 턴테이블은 스크래치 레인의 두 키(7키 기본: 정방향 `LSHIFT`, 역방향 `LCTRL`. 5.4절)와 패드의 턴테이블이다. 9키에는 턴테이블이 없어 8·9번 건반(`G`·`B`)이 그 자리다.

| 누른 것 | 동작 | 원본과 다른 점 |
| --- | --- | --- |
| START + 건반 | 하이스피드. 1·3·5·7번 건반은 느리게, 2·4·6번은 빠르게, 누를 때마다 한 번(HI-SPEED STEP). 녹색 숫자 유지 중이면 녹색 숫자 ±10 | 없음 |
| START + 턴테이블 | 플레이어가 쓰고 있는 가림막을 흰색 숫자 1(레인의 0.1%)씩. 정방향(스크래치 키)은 내리고(더 가림) 역방향은 올린다. 한 번 누르면 1, 누르고 있으면 0.3초 뒤부터 반복(아래 표). 아날로그 턴테이블은 틱마다 1. 그동안 녹색 숫자는 그대로이고 하이스피드가 바뀐다 | 원본은 누른 첫 프레임부터 50 ms 마다 `lanecovermarginlow`(기본 0.001)씩 움직여 보통 길이의 누름(80~120 ms)이 2 가 된다. 여기서는 누름 한 번이 정확히 1 이고, 걸음은 COVER FINE STEP 과 무관하게 1 이다. 원본은 HI-SPEED FIX 가 OFF 이면 녹색 숫자를 놓는다 |
| START + SELECT + 턴테이블 | 리프트를 1 씩. 정방향은 판정선을 내리고 역방향은 올린다(레인 커버와 같은 쪽: 정방향 = 아래로). 반복 규칙은 같다. 리프트는 녹색 숫자에 들어가지 않으므로 하이스피드도 그대로다(4절 4번) | 원본에 없는 조합이다. 원본은 START + SELECT 를 1초 누르면 곡을 끝내지만 이 플레이어에는 그 동작이 없다 |
| START 두 번(0.5초 안) | 레인 커버 켬·끔 | SELECT 를 뗀 것은 START 를 누른 것으로 세지 않는다. 원본은 START 를 누른 채 SELECT 를 눌렀다 떼는 것도 한 번으로 세어, 리프트 조합을 쓰다가 레인 커버가 꺼질 수 있다 |
| SELECT + 건반 | 고정된 이동 시간 ±1 ms(녹색 숫자 유지 중이면 유지 값) | 없음 |
| SELECT + 턴테이블 | 같은 값을 50 ms 마다 1 ms | 없음(전과 같다) |
| START + SELECT(누르는 순간) | 레인 커버가 꺼져 있고 리프트와 HIDDEN+ 가 둘 다 켜져 있을 때 START + 턴테이블이 움직일 쪽을 바꾼다 | 없음 |
| 마우스 휠 | 쓰고 있는 가림막을 한 칸에 0.5%(흰색 숫자 5). 움직인 뒤 1.5초 동안 가림막 수치가 뜬다 | 수치 표시를 더했다(원본은 START·SELECT 를 누르고 있을 때만 뜬다) |

"쓰고 있는 가림막"은 원본의 규칙이다(`setCoverValue`): 레인 커버가 켜져 있거나 리프트와 HIDDEN+ 가 둘 다 꺼져 있으면 레인 커버, 아니면 리프트가 켜져 있고 HIDDEN+ 가 꺼져 있거나 START + SELECT 가 리프트를 골랐으면 리프트, 그 밖에는 HIDDEN+. 리프트와 HIDDEN+ 는 레인 커버와 반대 부호로 움직인다.

START + 턴테이블의 세부

| 항목 | 동작 |
| --- | --- |
| 걸음 | PLAY 메뉴의 LANE COVER·LIFT 행과 같은 0.1% 격자 위의 한 칸(`rbms_config::move_number_steps`). 격자 밖에 있던 값은 가장 가까운 칸에서 한 칸 간다. 범위 끝(0, 1000)에서 멈춘다 |
| 키를 누르고 있을 때 | 누른 순간 1. 0.3초 뒤부터 0.05초마다 1(초당 20). 1.5초부터 한 번에 4(초당 80), 3초부터 한 번에 10(초당 200). 0 에서 1000 까지 약 7.3초. 지연과 빠르기는 상수다(`controls/repeat.rs` 의 `REPEAT_DELAY`, `REPEAT_EVERY`, `SHADE_PACE`). 걸음 수는 누른 시간만으로 정해져 프레임 속도와 무관하다 |
| 아날로그 턴테이블 | 돌린 틱 수만큼(원본의 `getAnalogDiff` 와 같은 틱). START 를 누르기 전에 돌린 것은 버린다. 아날로그 턴테이블이 있어도 키보드의 스크래치 키는 키로 읽는다 |
| 녹색 숫자 | 레인 커버를 움직이는 동안 화면의 녹색 숫자가 그대로이고 하이스피드가 따라 바뀐다. KEEP GREEN NUMBER 가 켜져 있으면 유지 값으로, 꺼져 있고 HI-SPEED FIX 가 OFF 가 아니면 그 판이 고정한 이동 시간으로(둘 다 전과 같다), 둘 다 아니면(HI-SPEED FIX OFF) START 를 누른 채 처음 커버를 움직일 때의 녹색 숫자로 묶는다. CONSTANT 에서도 같다(필드의 고정 템포 120 기준의 숫자가 그대로다). 묶음은 START 를 뗀 뒤에도 그 판 동안 남는다: 다음에 START 를 누르고 턴테이블을 돌리면 같은 숫자로 이어 간다. 턴테이블이 아닌 것이 속도나 레인 커버를 움직였으면(START + 건반, 지정 키, 휠, 켬·끔, PLAY 메뉴) 그때의 숫자로 다시 묶는다 |
| 하이스피드가 범위 끝일 때 | 0.01~20.00 을 넘어야 하면 범위 끝에 멈추고 녹색 숫자 표시는 실제 값을 보인다. 커버를 되돌리면 속도와 숫자도 되돌아온다. START 를 뗐다가 다시 눌러 되돌려도 같다(레인 커버를 1000 까지 내려 하이스피드가 0.01 이 된 뒤에도 원래 속도로 돌아온다). 범위 끝에 멈춘 채로 판을 끝내면 그 속도가 저장된다 |
| 꺼져 있는 가림막 | 값은 움직이고 켬·끔은 그대로다(원본이 꺼진 레인 커버를 다루는 방식). START + SELECT + 턴테이블은 리프트가 꺼져 있어도 리프트의 값을 움직인다. 켜려면 켬·끔 키나 PLAY 메뉴를 쓴다 |
| 화면 표시 | START 나 SELECT 를 누르고 있는 동안과 마지막으로 움직인 뒤 1.5초 동안 스킨의 "가림막을 바꾸는 중" 옵션(270)이 켜진다. 기본 스킨은 레인 커버 끝의 흰색 숫자와 녹색 숫자, 발치의 HI-SPEED·GREEN 과 바꾸는 동안만 뜨는 COVER·LIFT 를 보인다. 발치의 COVER·LIFT 는 가림막이 꺼져 있어도 뜨므로 꺼진 리프트를 START + SELECT + 턴테이블로 움직여도 값이 보인다. 판정선 아래의 리프트 수치는 리프트가 켜져 있을 때 항상 보인다 |
| 대상이 아닌 판 | 오토플레이와 리플레이에서는 START + 건반과 마찬가지로 움직이지 않는다(START 두 번과 휠은 듣는다). 속도를 잠그는 코스에서는 아무것도 움직이지 않는다 |
| 저장 | 설정의 `play.cover`·`play.lift`·`play.hispeed` 가 그 자리에서 바뀌고, 판이 끝나거나 나갈 때 파일에 쓴다 |

가림막을 움직이는 조작의 걸음(흰색 숫자 기준)

| 조작 | 한 번 | 누르고 있으면 | FINE STEP(또는 Shift)을 누른 채 |
| --- | --- | --- | --- |
| START + 턴테이블(레인 커버), START + SELECT + 턴테이블(리프트) | 1 | 0.3초 뒤부터 반복하고 빨라진다 | 그대로 1 |
| 지정 키: LANE COVER(`RIGHT`/`LEFT`), LIFT(`RBRACKET`/`LBRACKET`), HIDDEN+(미지정) | 10(1%) | 반복하지 않는다 | COVER FINE STEP(기본 0.001 = 1. PLAY 메뉴에서 0.1~10) |
| 마우스 휠 | 한 칸 5 | — | 그대로 5 |
| PLAY 메뉴의 LANE COVER·LIFT·HIDDEN+ 행 | 1 | — | Shift 10 |

실측(16 ms 프레임, 주입한 시계, 키를 누른 다음 프레임부터 뗄 때까지 프레임마다 읽음). "웨이브 13 까지" 열은 고치기 전 빌드에서 같은 방법으로 잰 값이다. 같은 동작을 고정하는 테스트는 `controls/turn_tests.rs`(누름 한 번 = 1, 반복 시각, 범위 끝)와 `controls/repeat.rs`(누른 시간별 걸음 수)다.

| 누른 것 | 누른 시간 | 웨이브 13 까지 | 지금 |
| --- | --- | --- | --- |
| START + 스크래치 키 | 30 ms | 레인 커버 1 | 1 |
| START + 스크래치 키 | 80~120 ms | 2 | 1 |
| START + 스크래치 키 | 0.5초 / 1초 / 3초 | 8 / 16 / 47(끝까지 약 64초) | 5 / 15 / 145(끝까지 약 7.3초) |
| START + SELECT + 스크래치 키 | 0.64초 | 아무것도 움직이지 않음 | 리프트 -8(판정선이 내려간다) |
| `RIGHT` 한 번 / Shift + `RIGHT` 한 번 | — | 10 / 1 | 10 / 1(그대로) |
| 휠 한 칸 | — | 5 | 5(그대로) |

### 5.3 판정 오프셋 키

| 항목 | 내용 |
| --- | --- |
| 출하 키 | `9` = JUDGE OFFSET -1 MS(빠르게 칠 때), `0` = JUDGE OFFSET +1 MS(늦게 칠 때). 숫자 줄의 오른쪽 끝, 리프트(`[` `]`)와 레인 커버 켬·끔(`\`) 위다. 출하 배치의 어느 모드도 숫자 키로 레인을 치지 않고, 플레이 조작·리플레이 분석 키·Esc 와 겹치지 않으며, 텐키 없는 키보드에도 있다. 선곡의 숫자 키(`0` 검색 등)는 플레이 화면에서 읽지 않는다. 키 설정 화면에서 바꾸거나 뗄 수 있고 패드에도 줄 수 있다 |
| 누르고 있으면 | 누른 순간 1 ms. 0.3초 뒤부터 0.05초마다 1 ms(초당 20). 범위가 ±200 이라 빨라지지 않는다(`STEADY_PACE`). 범위 끝에서 멈춘다 |
| 화면 | 스킨의 TIMING 숫자(number 12)가 다음 프레임에 새 값을 보이고, 노트도 그 프레임부터 새 자리로 그려진다(3.1절 6번). 기본 스킨은 TIMING 숫자를 항상 보인다. 알림(토스트)은 플레이를 가리므로 띄우지 않는다 |
| 대상 | 손으로 치는 판(일반 플레이, 연습)에서만 바뀐다. 오토플레이와 리플레이에서는 키도 반복도 무시한다(전과 같은 규칙) |
| 리플레이 | 오프셋은 판정을 옮기지 않으므로 기록할 것이 없다. 플레이 중에 바꿔도 리플레이는 `offset_ms: 0` 이고 `judge.offset_steps` 가 없다(3.3절 1번) |
| AUTO ADJUST 와 함께 | 키로 바꾼 값에서 자동 보정이 이어 간다(입력마다 세션이 설정값을 다시 읽는다) |
| AUTO ADJUST ON/OFF 키 | 출하 시 미지정으로 둔다. 실수로 눌리면 오프셋이 입력마다 움직이기 시작하고 화면에는 `TIMING AUTO` 표지만 남는다. 한 번 정하면 그대로 두는 값이라 PLAY 메뉴와 선곡 패널 3 에서 바꾼다 |
| 기존 키 설정 파일 | 그대로 읽힌다. 웨이브 13 까지의 파일은 두 키를 빈 값으로 적어 두었는데, 그때는 출하 키가 없어 빈 값밖에 적을 수 없었으므로 그런 파일은 두 키를 `9`·`0` 에서 읽는다. 단, 그 파일이 `9` 나 `0` 을 플레이 중에 함께 읽는 다른 것(어느 모드든 레인·스크래치 역방향, 다른 조작, 리플레이 분석 키)에 주었으면 그 키는 사용자가 준 쪽에 남고 오프셋 키는 미지정으로 나온다(키 설정 화면에서 지정). 사용자가 오프셋 키에 직접 준 키는 그대로다 |
| 파일 형식 | `keyconfig.ron` 에 `generation` 이 더해졌다(없으면 0 으로 읽는다. 1 = 판정 오프셋 두 키, 2 = 스크래치 역방향 키. 지금 판은 2). 출하 키가 나중에 생긴 것을 "그때는 키가 없었다"와 "사용자가 키를 뗐다"로 가르는 값이다. 키를 하나라도 바꾸면 그때 읽히던 키가 파일에 적히고 `generation` 이 지금 판이 되며, 그 뒤로는 뗀 키가 뗀 채로 남는다. 읽기만 한 파일은 다시 쓰지 않는다 |

### 5.4 스크래치 역방향 키

| 항목 | 내용 |
| --- | --- |
| 출하 키 | 스크래치 레인이 출하되는 Shift 옆의 Control: 5·7키와 10·14키의 왼쪽 턴테이블은 `LCTRL`, 10·14키의 오른쪽 턴테이블은 `RCTRL`. 9키에는 턴테이블이 없다. 원본의 키보드 기본값과 같다(`PlayModeConfig.KeyboardConfig.setKeyAssign`: `BEAT_7K -> { Z, S, X, D, C, F, V, SHIFT_LEFT, CONTROL_LEFT }`, 2P 는 `SHIFT_RIGHT, CONTROL_RIGHT`) |
| 왜 출하하는가 | 키가 하나면 턴테이블이 한 방향으로만 돈다. START + 턴테이블이 레인 커버를 내리기만 하고, START + SELECT + 턴테이블이 리프트를 내리기만 해 기본값 0 인 리프트를 그 조합으로 올릴 수 없었다(웨이브 14 리뷰). 웨이브 13 까지는 미지정으로 출하했다 |
| 하는 일 | 플레이: 스크래치 레인을 반대 방향으로 친다(차지 노트의 끝에서 방향이 갈린다). START 와 함께: 가림막을 올린다(정방향은 내린다). 선곡: PREVIOUS BAR(정방향은 NEXT BAR). 검색어·숫자·경로를 치는 동안의 Control(붙여넣기 등)은 전과 같이 글자 입력 쪽이 받는다 |
| 기존 키 설정 파일 | 그대로 읽힌다. `generation` 이 2 보다 작은 파일에서 역방향 키가 빈 값인 스크래치 레인은 출하 키에서 읽는다. 스크래치 자체를 다른 키로 옮긴 파일(예: 7키 스크래치가 `1`)에도 `LCTRL` 이 붙는다. 단, 그 파일이 그 Control 키를 같은 모드의 레인·다른 역방향 칸, 플레이 조작(START·SELECT·FINE STEP 포함), 리플레이 분석 키에 주었으면 그 키는 사용자가 준 쪽에 남고 역방향은 미지정으로 나온다. 파일이 역방향에 직접 준 키는 그대로다. `scratch_reverse` 항목이 아예 없는 더 오래된 파일도 같은 규칙으로 읽는다 |
| 떼기 | 키 설정 화면(`F5`)의 SCRATCH n REVERSE 행에서 `Del`. 한 번 뗀 키는 `generation` 과 함께 저장되어 다시 붙지 않는다. `Backspace` 는 출하 키로 되돌린다 |
| 컨트롤러 | 패드의 역방향 지정은 전과 같다(아날로그 턴테이블은 두 방향을 스스로 낸다) |

## 6. 구현 위치

| 내용 | 위치 |
| --- | --- |
| 이동 시간·녹색 숫자·유지 속도의 순수 계산 | `crates/rbms-config/src/speed.rs`(`held_speed`, `HeldTravel`, `HeldSpeed`, `green_of_duration`, `duration_of_green`, `duration_stepped`, `duration_stepped_by_green`, `travel_duration_ms`) |
| 설정 필드 | `crates/rbms-config/src/schema.rs`(`PlayOptions.keep_green_number`, `duration_ms`, `JudgeOptions.offset_ms`, `auto_offset`) |
| 자동 보정의 한 걸음 | `crates/rbms-play/src/session.rs`(`auto_adjust_step_ms`, `PlaySession::judge_offset_ms`, `set_judge_offset_us`). 세션은 오프셋을 판정에 쓰지 않는다 |
| 오프셋만큼 앞서 그리기 | `apps/rbms-player/src/stage/play/mod.rs`(`field_lead_us`, `drawn_chart_us`, `built_in_field_us`. 오프셋을 더하지 않은 플레이 타이머 시각은 `played_chart_us` 이고 BGA 가 이것을 읽는다), 미리보기는 `stage/play/skin_preview.rs`(`PlaySample::drawn_chart_us`) |
| 오프셋의 측정과 캡처 | `apps/rbms-player/src/stage/play/timing_tests.rs`, `timing_capture_tests.rs`(`RBMS_SKIN_CAPTURE_DIR` 을 주면 PNG 저장), `crates/rbms-play/tests/auto_adjust.rs` |
| 플레이 중 조작 | `apps/rbms-player/src/stage/play/controls.rs`(`read_held`, `turn_shade`, `turn_lane_cover`, `TurnPin`, `repeat_held_keys`), 누름 반복은 `stage/play/controls/repeat.rs`(`HeldRepeat`, `Pace`), `apps/rbms-player/src/app_input.rs`(`apply_control`, `fine_pressed`, `control_repeating`, `take_turntable_ticks`), 아날로그 턴테이블의 틱은 `apps/rbms-player/src/gamepad.rs`(`PadMapper::take_turn`) |
| 조작과 단축키의 종류, 저장 형식 | `apps/rbms-player/src/keyconfig.rs`(`ControlAction`, `Shortcut`, `ControlBinds`, `KeyConfig::generation`, `ControlAction::given_later`, `KeyConfig::control_key`) |
| 플레이 중 조작의 측정과 캡처 | `apps/rbms-player/src/stage/play/controls/turn_tests.rs`, `controls/capture_tests.rs`(`RBMS_SKIN_CAPTURE_DIR` 을 주면 PNG 저장) |
| IR 리플레이 변환 | `apps/rbms-player/src/ir_replay.rs`(`uploaded_time_us`) |
| PLAY 메뉴(행 목록, 조작, 정보 패널) | `apps/rbms-player/src/stage/play_menu.rs`, `stage/play_menu/rows.rs`(숫자 행의 값과 목록 표시, 행의 종류, 표시 꼬리, 읽기 전용, 큰 단계, 기본값 복원). 숫자 행의 공통 부품은 `apps/rbms-player/src/number_row.rs`, 걸음 모델은 `crates/rbms-config/src/number.rs`·`settings.rs`(`number_spec`) |
| PLAY 메뉴와 SKIN 메뉴가 같이 쓰는 그리기·스크롤 부품 | `apps/rbms-player/src/stage/preview_menu.rs` |
| 목표 점수(TARGET, TARGET FALLBACK, TARGET RATE. 11절) | 목표와 물러나는 순서 `apps/rbms-player/src/target.rs`(`resolve`, `settle`, `target_ex`, `target_name`), 설정 필드와 어휘 `crates/rbms-config/src/judge.rs`(`ScoreTarget`, `FALLBACKS`, `label_then`)·`schema.rs`(`JudgeOptions.target_fallback`, `target_rate_percent`), 행 `settings.rs`(`SettingId::TargetFallback`, `TargetRate`), 판이 쓰는 곳 `app_result.rs`(`run_target`, `target_context`)와 `stage/play/mod.rs`(`ensure_pace_target`), 스킨에 넘기는 곳 `skin_host/options.rs`(`OptionsState::paced_by`, `SettingsView::target_text`)·`skin_screen.rs`(`PlayDraw::paced_by`)·`skin_host/select/bridge.rs`(`target_stand_in_on`) |
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
| SETTINGS 에서 같은 키 또는 왼쪽 레일 아래 OTHER SCREENS 의 `PLAY MENU` 단추(`SKIN MENU` 위) | SETTINGS 위에 연다. 글자를 치는 중이거나 RIVALS·IR PROFILES 목록이 열려 있으면 동작하지 않는다. 단추 오른쪽과 바닥 안내 줄에 지금 묶인 키가 `F8 >`, `F8 PLAY` 처럼 나온다 |
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
| CHART | RANDOM, GAUGE, TARGET, TARGET FALLBACK, ESCAPE | 항목 이름. TARGET 은 점수를 이름으로 삼는 목표(LOCAL BEST·IR BEST·RIVAL)일 때 `LOCAL BEST > AAA` 처럼 점수가 없을 때 쓰는 것을 뒤에 붙인다(11절) | 한 항목 |
| CHART | TARGET RATE | `85%` | 1% / 10%. 범위 1~100%. RATE CUSTOM 목표가 요구하는 몫(11절) |
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
| TARGET, TARGET FALLBACK, TARGET RATE | 점수 그래프의 타깃 막대와 `TARGET +n`, 그 막대의 이름표. 기록·IR·라이벌 없이 예시 차트의 노트 수만으로 정한 타깃이다. 그래서 LOCAL BEST·IR BEST·RIVAL 은 TARGET FALLBACK 으로 물러난 값이고, 이름표도 물러난 목표의 이름이다(웨이브 16 전에는 MAX 로 물러났다) |
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
| 위 줄 왼쪽: 점수 차 | 목표 대비 EX 점수 차를 부호와 네 자리로 쓴다(`+0003`, `-0125`). 스코어 그래프의 `TARGET` 숫자와 같은 값이고 CHART 묶음의 TARGET 이 목표를 정한다(목표의 점수가 없으면 TARGET FALLBACK. 11절). PGREAT 에도 뜬다 | `SCORE DIFFERENCE`: TARGET(기본) / BEST(자기 최고 기록 대비) / OFF |
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

## 10. 기록 저장(어떤 판이 무엇을 남기나)

> 웨이브 16(2026-10-14) 추가, 웨이브 18(같은 날) 개정. 웨이브 16 은 "로그인하지 않으면 기록이 저장되지 않고 램프만 남는 것 같다"는 보고를 점검해, 기록 저장이 점수 서버 설정·로그인과 무관하고 램프만 남는 것은 SCRATCH AUTO 판이라는 것을 밝혔다. 웨이브 18 은 사용자의 결정(2026-10-14, 결정 12 와 "SC 의 점수는 빼고 저장"을 대신한다)대로 바꿨다: SCRATCH AUTO 의 턴테이블 노트는 초록으로 그려지고 아무 집계에도 들어가지 않으며, 사람이 친 판은 어시스트든 아니든 전부 저장되고 서버로 간다.

### 10.1 저장되는 곳

| 곳 | 내용 | 읽는 화면 |
| --- | --- | --- |
| `scoredb.sqlite` 의 `scorelog` | 판마다 한 행. 판정 12칸(빠름·늦음), 노트 수, 콤보, BP, 램프, 게이지, 어시스트 단계, 판정 규칙 세대(`rule_version`), 배치와 시드, 날짜, 리플레이 파일 이름, 판이 끝날 때 주 서버에 넘겼는지(`ir_submitted`. 참고용이다. 서버가 받았는지는 `ir_sync` 가 적고, 어떤 판단에도 이 값을 읽지 않는다) | 선곡의 기록 목록(R), 리플레이 슬롯 |
| `scoredb.sqlite` 의 `score` | 차트(와 모드, LN 종류)마다 한 행인 최고 기록. 램프, 최고 EX 를 낸 판의 판정 12칸과 그 판의 노트 수, 최대 콤보, 최소 BP, 플레이 횟수, 클리어 횟수, 마지막 날짜 | 스킨이 그리는 선곡의 EX SCORE·RATE·RANK·MAX COMBO·MISS COUNT·판정 막대·PLAY COUNT·CLEAR COUNT |
| `scoredb.sqlite` 의 `player_stat` | 날짜별 누적(플레이 횟수, 판정 수, 플레이 시간) | - |
| `replays/<md5 앞 8자>-<시각>.ron` | 그 판의 입력과 SCRATCH AUTO·게이지·배치·시드·판정 설정. 판이 끝나는 프레임은 임시 파일만 만들고(폴더가 없거나 같은 이름의 파일이 막고 있거나 권한이 없으면 여기서 실패한다), 내용 쓰기·디스크 반영(fsync)·이름 바꾸기는 저장 스레드가 한다(10.10). 임시 파일을 만들 수 없으면 행에 리플레이 이름을 적지 않고(`replay_file` NULL, 결과 화면도 저장됨으로 말하지 않는다) `replay not saved: …` 오류를 띄운다. 저장 스레드에서 실패하면(디스크가 가득 참 등) 그 이름을 행과 기록 책에서 지우고 같은 오류를 띄운다. 어느 쪽이든 결과 화면의 전송은 메모리의 리플레이를 그대로 올린다 | 기록 목록의 리플레이, 결과 화면의 REPLAY |
| `scoredb.sqlite` 의 `play_submission` | 판마다 한 행. 판이 끝날 때 만든 점수 제출 본문(JSON)과 그 판을 친 계정(게스트면 NULL). 0.1.4 부터 적는다. `scorelog`·`score`·`player_stat` 과 한 트랜잭션으로 적으므로(`ScoreDb::record_play_kept`) 본문 없는 판이 생기지 않는다. 그 전의 판은 행이 없고 동기화가 기록 행으로 본문을 다시 만든다(10.9) | - |
| `scoredb.sqlite` 의 `ir_sync` | 판과 점수 서버마다 한 행. 서버가 그 판을 받았는지(`accepted`), 거절했는지(`rejected`, 오류 코드·필드·이유·HTTP 상태와 거절받은 빌드 `refused_under`), 답을 못 받았는지(`retry`), 보내지 않았는지(`skipped`), 서버의 `score_id`, 보낸 횟수와 마지막 시각(밀리초), 리플레이 상태(거절이면 `replay_refused_under`). 행이 없으면 "미확인"이다. 두 `*_refused_under` 열은 0.1.4 에서 더했고 옛 DB 에는 열면서 `ALTER TABLE` 로 더한다(스키마 세대 1 그대로) | 선곡의 기록 상세(R)의 IR 줄 |
| `scoredb.sqlite` 의 `course_submission`, `course_sync` | 끝난 코스마다 한 행인 코스 제출 본문(JSON, 끝난 밀리초 포함)과 친 계정, 그리고 코스와 서버마다 서버의 답(`ir_sync` 와 같은 열, 리플레이 열 없음). 0.1.4 부터 적는다(10.9) | - |
| `pending-runs/<밀리초>-<md5 앞 8자>.ron` | 점수 DB 를 열 수 없거나 쓸 수 없을 때 친 판. 기록 행과 제출 본문, 계정, 최고 기록을 바꿀 수 있는지를 담는다. 다음에 DB 가 열리면 한 트랜잭션씩 DB 로 읽어 들이고 파일을 지운다(`ScoreDb::import_pending_runs`). DB 가 이미 가진 판(같은 차트·모드·LN 종류·끝난 초)은 다시 들이지 않고 파일만 지운다. DB 가 거절한 판은 `kept run not read in` 경고를 띄우고 파일을 남기며, 그 뒤 파일은 계속 읽어 들인다. 그 판의 리플레이는 그대로 저장된다. 판을 따로 남기는 파일조차 만들지 못하면 그 판의 리플레이도 쓰지 않고 기록에 이름을 적지 않는다. 판마다 `score database not open: this run is kept on the side and goes into it when it opens` 오류가 뜬다 | - |
| 메모리의 기록 책(`ScoreBook`) | 시작할 때 `scorelog` 전체로 만들고 판이 끝날 때마다 한 행을 더한다. 행마다 최고 기록이 될 수 있는지를 어시스트 단계와 규칙 세대로 다시 읽는다(`rbms_store::sets_best`) | 플레이 화면의 LOCAL BEST 목표와 스코어 그래프의 BEST, 결과 화면의 이전 최고, 내장 선곡 화면 |

### 10.2 판의 종류별로 남는 것

"사람이 친 판"은 오토플레이도 리플레이 보기도 연습도 아닌 판이다. 사람이 친 판은 어떤 설정이든 기록 목록의 행·날짜 통계·리플레이(AUTO REPLAY 켬)를 남기고 점수 서버로 간다(주 서버가 기본 서버(`SERVER URL`)면 로그인했을 때, 주 서버가 IR 프로필이면 언제나. 보내지 않은 판은 `ir_submitted` 만 0 이고 나머지는 같다. 기본 서버가 설정돼 있어도 로그인하지 않았으면 보내지 않으므로 0 이다(`AppShared::sends_to_primary_ir_server`). 0.1.4 부터는 로그인하면(시작할 때 로그인 상태였으면 세션이 확인된 뒤) 서버가 아직 받지 않은 지난 판을 모두 보낸다: 10.9절의 기록 동기화).

| 판 | 기록 목록의 행·날짜 통계 | 최고 기록(EX·판정·콤보·BP) | 램프·플레이 횟수 | 리플레이(AUTO REPLAY 켬) | 점수 서버 | 원본과 다른 점 |
| --- | --- | --- | --- | --- | --- | --- |
| 손으로 친 판(로그인 여부와 무관) | 남는다 | 더 좋은 값이면 갱신 | 더 높은 램프면 갱신, 횟수 +1 | 저장 | 전송 | 없음 |
| FAILED 로 끝난 판 | 남는다. BP(`minbp`)는 게이지가 비어 도달하지 못한 노트까지 센다(`BMSPlayer.java:909`, 10.11) | EX 가 더 높으면 갱신(램프와 무관, `ScoreData.java:549`) | FAILED, 횟수 +1(클리어 횟수는 그대로) | 저장 | 전송 | 원본은 PG~BD 가 하나도 없는 판을 저장하지 않는다(`BMSPlayer.java:858-861`). 여기서는 FAILED 로 남는다 |
| 실패 화면에서 START·SELECT 로 곧장 다시 한 판 | 남는다. 다시 시작하기 전에 결과 화면이 남겼을 그대로(장면이 센 집계, FAILED) 남긴다(`app_result::record_retried_run`). 메시지 `failed run kept before starting again` | FAILED 판과 같다 | FAILED, 횟수 +1 | 저장 | 로그인했으면 그 판 하나만 기록 동기화로 곧장 보낸다(Esc 포기 판과 같다), 게스트면 다음 로그인 때 | 원본은 이 판을 저장하지 않는다(`BMSPlayer.java:696-710`). "사람이 친 판은 모두 저장·전송"(사용자 결정 2026-10-14)으로 남긴다. START 를 누른 채 실패하면 곧장 다시 시작하는 것은 원본대로다 |
| SCRATCH AUTO 를 켠 판(어시스트 1). 그 판의 턴테이블 레인에 노트가 하나라도 있을 때만 | 남는다. 턴테이블 노트를 뺀 집계 그대로(10.5) | 더 좋은 값이면 갱신. 손으로 친 판과 한 행을 같이 쓴다 | LIGHT ASSIST CLEAR 로 낮춘 램프, 횟수 +1 | 저장(`scratch_auto: true`) | 전송. `options.scratch_auto = true`, `options.assist = ["AUTO_SCRATCH"]` 를 싣는다. 서버(웨이브 20)는 이 판을 ranked 로 저장하고 일반 판과 따로 센 최고 기록에 넣는다. 웹 순위 목록은 기본으로 일반 판만 보이고 "SCRATCH AUTO 포함" 을 켜면 A-SCR 표지와 함께 보인다. 응답은 `flags: ["SCRATCH_AUTO"]` 이고 순위는 그 포함 목록 기준이라, 결과 화면은 `IR: RANK #n (SCRATCH AUTO)` 로 쓴다(`ir_outcome::SCRATCH_AUTO_RANK_MARK`. `ScoreFlag::ScratchAuto` 는 랭킹을 막는 사유가 아니다). AUTO UPLOAD REPLAY 의 리플레이도 올라간다. 곡 선택의 IR RANKING 패널은 기본 목록을 쓴다 | 원본은 턴테이블 노트를 배경음으로 옮기고(같다) 그 판을 최고 기록·리플레이·IR 에서 뺀다(`score = false`). 사용자의 결정으로 다른 판과 똑같이 남기고 보낸다. 램프를 낮추는 것은 원본대로 유지했다(사용자가 램프는 말하지 않았다) |
| SCRATCH AUTO 를 켰지만 턴테이블 노트가 없는 차트, 턴테이블이 없는 모드(9키) | 손으로 친 판과 같다 | 손으로 친 판과 같다 | 손으로 친 판과 같다(램프를 낮추지 않는다) | 저장 | 전송 | 없음. 원본은 자동으로 친 레인에서 노트를 옮겼을 때만 어시스트를 올린다(`AutoplayModifier.modify`) |
| JUDGE WIDTH 나 LN MARGIN 을 100% 넘게 넓힌 판(어시스트 2) | 남는다 | 바뀌지 않는다 | ASSIST CLEAR 로 낮춘 램프, 횟수 +1 | 저장 | 전송. `options.assist = ["CUSTOM_JUDGE"]`. AUTO UPLOAD REPLAY 로 리플레이를 올리는 것은 여섯 판정 폭이 한 값이고 LN MARGIN 이 기본일 때뿐이다: 올리는 형식이 폭 하나만 싣고 LN MARGIN 을 싣지 않아, 다른 판은 내려받으면 다른 판정으로 재생되기 때문이다(`ir_replay::wire_carries_judge`). 로컬 리플레이는 전부 싣는다 | 원본은 리플레이를 남기지 않고 보내지도 않는다(`BMSPlayer.java:208-214`). "서버로 저장되지 않는 기록은 없다"는 결정으로 남기고 보낸다. 최고 기록을 바꾸지 않는 것은 원본대로 유지했다: 넓힌 판정의 EX 는 같은 차트를 정상 판정으로 친 EX 와 같은 잣대가 아니고, 사용자는 이것을 말하지 않았다 |
| TOTAL 을 직접 정한 판(JUDGE 탭의 TOTAL > 0, 어시스트 2) | 남는다 | 바뀌지 않는다 | ASSIST CLEAR 로 낮춘 램프, 횟수 +1. 결과 화면 `OWN TOTAL: SENT AND KEPT, BUT NO BEST SCORE` | 저장. 그 TOTAL 을 리플레이(`judge.total_override`)에 적고, 재생은 설정이 아니라 적힌 값으로 한다. 올리는 형식에 TOTAL 자리가 없어 AUTO UPLOAD REPLAY 로는 올리지 않는다 | 전송. `options.total_override` 와 `options.assist = ["CUSTOM_JUDGE"]`. 서버는 `TOTAL_OVERRIDE`·`ASSIST` 로 unranked. 이 스테이지가 낀 코스는 넓힌 판정 스테이지가 낀 코스와 같다 | 원본에는 이 설정이 없다(rbms 확장). 넓힌 판정과 같이 다룬다(사용자 결정 2026-10-15) |
| 게이지 종류, GAUGE AUTO SHIFT | 위와 같다. 게이지는 램프만 정한다. 자동 전환된 판은 끝난 게이지로 적힌다 | - | - | - | - | 원본은 전환된 판의 게이지를 -1 로 적는다(`BMSPlayer.java:884`) |
| 코스의 각 스테이지 | 그 차트의 판으로 위 규칙대로 남는다(`enter_result` 가 다음 스테이지로 넘어가기 전에 적는다) | 위와 같다 | 위와 같다 | 위와 같다 | 스테이지 점수는 위와 같다. 코스 점수도 어시스트 스테이지가 있든 없든 보낸다(사용자 결정 2026-10-14). 코스 전체로 접은 어시스트(`course_ir::CourseAssist`)를 단일 곡과 같은 어휘로 싣는다: SCRATCH AUTO 스테이지가 하나라도 있으면 `scratch_auto = true`, 태그는 `assist = ["AUTO_SCRATCH", "CUSTOM_JUDGE"]` 중 해당하는 것(각각 한 번). 어시스트가 없는 코스는 두 필드를 싣지 않아 전과 같은 본문이고, 두 필드를 모르는 옛 서버도 거절하지 않는다. 코스 최고 기록(코스 결과 화면의 "점수 저장" 여부, `course_ir::course_updates_score`)은 단일 곡 규칙과 같다: SCRATCH AUTO 스테이지만 낀 코스는 갱신하고, 넓힌 판정 스테이지가 낀 코스는 갱신하지 않는다. 서버(웨이브 20)는 SCRATCH AUTO 스테이지만 낀 코스를 단일 곡처럼 ranked 로 저장하고 SCRATCH AUTO 칸의 코스 베스트에 넣으며(웹 코스 순위는 토글을 켜야 보인다), 넓힌 판정 스테이지가 낀 코스는 unranked 로 저장하고 코스 순위와 코스 베스트에 넣지 않는다. 보낼 수 있는 코스(모든 스테이지가 사람이 친 판이고 공개 코스)는 끝날 때 점수 DB 의 `course_submission` 에 본문째 남기고, 로그인했으면 곧장 보내 답을 `course_sync` 에 적는다(메시지 `course sent` / `course rejected: …` / `course not sent yet (…): sent again on the next records sync`). 로그인하지 않았거나 서버가 답하지 않은 코스는 기록 동기화가 같은 본문(같은 `played_at`)으로 다시 보낸다. 서버는 (사용자, 코스 해시, `played_at`) 가 같으면 같은 코스로 본다 | 스테이지 램프를 원본과 대조하는 일은 하지 않았다(원본은 코스 중 스테이지 램프를 게이지로 정하지 않는다, `BMSPlayer.java:864-880`) |
| AUTOPLAY | 남지 않는다 | - | - | - | 보내지 않는다 | 없음(`MusicResult.updateScoreDatabase` 가 PLAY 모드만 적는다). 사람이 친 판이 아니다 |
| 리플레이 재생 | 남지 않는다 | - | - | - | 보내지 않는다 | 없음. 재생하는 동안 SCRATCH AUTO·GAUGE·RANDOM 은 리플레이에 적힌 값으로 돌고 끝나면 플레이어의 값으로 돌아온다(웨이브 16) |
| 연습 구간 | 남지 않는다 | - | - | - | 보내지 않는다 | 없음. 사람이 쳤지만 구간만 잘라 친 판이라 한 판의 기록이 아니다 |
| 도중 포기(Esc) | 사람이 친 판이면 판정한 노트 수와 상관없이 남는다(사용자 결정 2026-10-15, 판정이 하나도 없는 판도 FAILED 로 남는다). 결과 화면을 거치지 않고 곧장 선곡으로 간다(Esc 는 지금도 바로 나가는 키다). 남은 노트(아직 판정하지 않은 노트와 롱노트의 끝)는 모두 늦은 POOR(見逃し)로 세고 램프는 FAILED 다(`PlaySummary::left_unfinished`). 그래서 PG~POOR 합이 차트의 노트 수와 같다. 노트를 다 처리한 뒤의 Esc(판정할 노트가 처음부터 없는 차트도 여기다. 10.5절), 그리고 이미 실패·끝으로 닫히는 중인 판의 Esc(ESCAPE 가 길게 누르기일 때 누르는 도중 게이지가 바닥나 길게 누르기가 닫히는 중에 끝나는 경우도 같다)는 결과 화면으로 가고 위 규칙대로 남는다: 닫히는 판은 장면이 센 대로 남고 남은 노트를 見逃し POOR 로 더하지 않는다. 코스 도중의 Esc 는 그 스테이지에 남은 노트가 없으면 위처럼 결과로 가고 코스가 이어진다. 남은 노트가 있으면 그 스테이지를 위 규칙대로 FAILED 곡 기록으로 남기고, 코스를 끝낸 스테이지와 함께 FAILED 코스 기록으로 남겨 보낸 뒤(`app_result::leave_course`, 메시지 `course left part-way kept as FAILED`) 선곡으로 나간다 | EX 가 더 높으면 갱신(FAILED 판과 같다) | FAILED, 횟수 +1 | 나간 시점까지의 입력으로 저장 | 로그인했으면 그 판 하나만 기록 동기화로 곧장 보낸다(결과 화면이 없어 결과 화면의 전송은 쓰지 않는다). 게스트면 다음 로그인 때 동기화가 보낸다. IR 프로필로의 전송은 다른 판과 같다. 메시지 `run left mid-song kept as FAILED` 가 뜬다 | 원본(`BMSPlayer.stopPlay`)은 FAILED 로 바꿔 결과 화면을 거쳐 저장하고, 남은 노트는 판정 수에 넣지 않고 BP 에만 더한다(`BMSPlayer.java:909`). 여기서는 결과 화면을 거치지 않고(위) 남은 노트를 見逃し POOR 로 센다(사용자 결정 2026-10-14). 리플레이를 재생하면 남은 노트가 지나가 見逃し POOR 가 되므로 판정 수는 기록과 같지만, 재생은 곡 끝까지 가고 게이지는 기록과 다르게 끝날 수 있다(쉬운 게이지면 클리어로 끝날 수도 있다). 알려진 한계다 |

### 10.3 최고 기록 한 행과 노트 수

1. SCRATCH AUTO 판은 손으로 친 판과 한 행(`score`)을 같이 쓴다. 두 판의 노트 수가 다르다(턴테이블 노트만큼). 그래서 최고 기록 행의 노트 수는 그 행이 지닌 EX 를 낸 판의 것이다: 처음으로 최고 기록이 될 수 있는 판이 정하고, 그 뒤로는 EX 를 갱신한 판만 바꾼다(`merge_best`). RATE·RANK 는 언제나 같은 판의 EX 와 노트 수로 계산된다. 웨이브 18 전에는 최고 기록을 낼 수 있는 판마다 노트 수를 덮어써서, 노트 수가 다른 판이 섞이면 RATE 가 100% 를 넘을 수 있었다.
2. EX 는 노트 수가 달라도 그대로 비교한다(사용자의 결정: 따로 두지 않는다). SCRATCH AUTO 판은 턴테이블 노트만큼 EX 상한이 낮으므로, 같은 실력이면 손으로 친 판의 EX 가 이긴다.
3. 기록 목록(R)과 결과 화면은 판마다 그 판의 노트 수로 읽는다.
4. 노트 수의 정의는 디코더가 롱노트를 원본처럼 정리한 뒤의 값이다(2026-10-15): 롱노트 몸통 안의 일반 노트는 배경음이 되어 세지 않고, 끝나지 않은 롱노트 머리는 모델에서 지워져 세지 않는다. 이 둘이 있는 차트(감사 때 11,336 차트 중 수십 개)는 정리 전의 판과 노트 수가 다르다. 옛 판의 노트 수·EX 는 그 판 기록에 적힌 값 그대로 읽고(3번), 최고 기록 비교는 2번대로 EX 절대값이라 이런 차트에서는 옛 최고 기록이 더 높게 남을 수 있다. 곡 DB 의 `notes` 는 `PARSER_VERSION` 3 의 전체 재스캔으로 새 값이 된다. 규칙 세대(`SCORE_RULE_VERSION`)는 이 변경만으로 올리지 않았다(`docs/acknowledge/2026-10-15-adr-long-note-body-notes.md` 6항).

### 10.4 웨이브 16 까지의 SCRATCH AUTO 판(사용자의 37판)

1. 웨이브 16 까지의 SCRATCH AUTO 판은 턴테이블 노트를 사람 대신 PGREAT 로 쳐서 셌다. 노트 수·EX·콤보에 턴테이블이 들어 있어 새 규칙의 판과 비교할 수 없다. 사용자의 결정은 "없어져도 괜찮으니(램프만 있어도 됨) 잘 처리해 둔다"이다.
2. 처리: 지우지도 고쳐 쓰지도 않는다. 새로 저장하는 판은 판정 규칙 세대 3(`SCORE_RULE_VERSION`)으로 적고, 기록 책은 행마다 `rbms_store::sets_best(어시스트, 세대)` 로 최고 기록이 될 수 있는지 읽는다: 어시스트 0 은 언제나, SCRATCH AUTO(어시스트 1)는 세대 3 부터만, 넓힌 판정(2)과 옮겨 온 기록의 알 수 없는 어시스트(-1)는 언제나 아니다. 세대 2 의 SCRATCH AUTO 행은 그래서 이력과 램프로만 남고 LOCAL BEST·스코어 그래프의 BEST·결과 화면의 이전 최고·선곡의 EX 에 들어가지 않는다. `score` 행은 그 판들이 애초에 점수를 남기지 않았으므로(웨이브 16 의 규칙) 고칠 것이 없다. 그래서 앱이 시작할 때 한 번 하는 마이그레이션은 필요 없다.
3. 세대 3 은 SCRATCH AUTO 만 바꾼다. 손으로 친 세대 2 의 행은 지금과 같은 판정이라 "OLD RULE"(`is_stale_rule_version`, 세대 2 미만)로 표시하지 않는다. 세대 1 이하의 기준은 그대로다.
4. 세대 2 이하의 SCRATCH AUTO 판은 기록 동기화(10.9)에서도 보내지 않는다. 동기화는 그 판들에 `ir_sync` 행을 `skipped`·`OLD_SCRATCH_AUTO_RULE` 로 적어 기록 상세가 `IR: NOT SENT (OLD SCRATCH AUTO RULE)` 로 이유를 말하게 한다. `scores.ron` 에서 옮겨 온 판은 어시스트가 있었다는 것만 남아(`assist = -1`) SCRATCH AUTO 였는지 알 수 없으므로, 세대 2 이하의 그런 판도 같은 이유로 보내지 않는다(넓힌 판정이었던 옛 판이 섞여 있을 수 있고, 이 판들은 서버에서도 순위에 올리지 않을 판이라 보내지 않아도 잃는 것은 이력 한 줄이다).
5. 사용자 DB 의 사본으로 잰 값(테스트 `a_players_own_database_reads_its_old_scratch_auto_rows_as_lamps_alone`, `#[ignore]` 라 `--ignored` 와 `RBMS_SCOREDB_COPY=<사본>` 으로 손으로 돌린다): 37판 모두 어시스트 1·세대 2, 29 차트. 이 빌드로 읽으면 LOCAL BEST 가 있는 차트 0, 램프가 있는 차트 29, 점수가 있는 최고 기록 행 0. 같은 사본에 새 규칙의 SCRATCH AUTO 판을 하나 적으면 그 판이 그 차트의 첫 최고 기록이 되고 램프는 그대로다.
6. 웨이브 16 까지의 SCRATCH AUTO 리플레이를 지금 보면 새 규칙으로 재생된다(턴테이블이 세지 않는다). 그래서 그 판의 이력 행과 숫자가 다르다.

### 10.5 SCRATCH AUTO 의 판정과 그리기

| 항목 | 동작 | 원본과 다른 점 |
| --- | --- | --- |
| 판정에 들어가는 차트 | 배치(RANDOM 등)를 적용한 뒤 턴테이블 레인의 노트를 모두 뺀 차트. 판정·콤보·게이지·EX·판정 수·FAST/SLOW·BP·노트 수(만점의 기준)·진행률·스코어 그래프·스킨이 읽는 노트 수와 남은 노트 수가 모두 이 차트로 센다. RATE·랭크·최대 콤보도 같다(`rbms_play::Player::with_auto_lanes`) | 같다(`AutoplayModifier.modify` 가 `moveToBackground` 로 차트에서 뺀다) |
| 그리기 | 뺀 노트는 필드에만 남아(`PlaySession::field_timelines`) 초록으로 그려진다. 일반 노트와 롱노트의 시작·끝은 `AUTO_LANE_NOTE`(#30E050), 롱노트 몸통은 `AUTO_LANE_LONG_BODY`(같은 초록, 불투명도 0xA0). 스킨이 고른 노트 그림의 자리(크기·위치)에 엔진이 그리므로 기본 팩과 남의 Lua 스킨(ModernChic 캡처)이 같다. 판정선에 닿으면 사라지고, PAST NOTES·처리된 노트 표시·팝픈의 떨어지는 노트에 해당하지 않는다. 내장 필드(스킨 없음)도 같은 색이다 | 원본은 그리지 않는다(배경음이 된다). 스킨 그림에 색을 곱하는 대신 단색으로 그린 이유: 그림 색에 곱하는 방식은 빨간 턴테이블 노트(기본 팩과 대부분의 스킨)를 검게 만들고, 그림의 알파로 색을 바꾸는 그리기는 지금 렌더러(GPU 셰이더)에 없다 |
| 지뢰 | 턴테이블 레인의 지뢰는 판정에서도 그리기에서도 뺀다 | 같다(`moveToBackground` 는 지뢰를 배경음으로도 옮기지 않는다) |
| 키음 | 각 노트 머리(롱노트는 시작)의 키음이 제 시각에 키 버스로 울린다. 롱노트 끝과 지뢰는 울리지 않는다(오토플레이와 같다) | 원본은 배경음(BGM 버스)으로 울린다. 키 음량을 따르게 키 버스에 두었다 |
| 턴테이블 입력 | SCRATCH AUTO 가 켜져 있으면(그 차트에 턴테이블 노트가 없어도) 턴테이블 키의 누름·뗌을 판정에 넘기지 않는다. 공 POOR 도, 빔도, 키음도 없다. 리플레이에는 입력으로 적히지만 재생할 때도 같은 이유로 무시된다 | - |
| TOTAL | 차트의 #TOTAL, 없으면 턴테이블을 포함한 원래 노트 수로 정한 기본값 그대로. 노트 하나가 올리는 게이지는 TOTAL 을 판정되는 노트 수로 나눈 값이다 | 같다(원본은 디코드할 때 TOTAL 을 정하고 모디파이어를 그 뒤에 적용한다) |
| 턴테이블 노트만 있는 차트 | 판정되는 노트가 0 개라 BGM 만 있는 차트처럼 저절로 끝나지 않는다. 처음부터 남은 노트가 없으므로 Esc(와 START+SELECT)는 끝까지 친 판처럼 결과 화면으로 간다. 램프는 NO PLAY 이고 FAILED 로 남지 않는다(`PlaySession::all_notes_resolved`, 테스트 `leaving_a_chart_with_no_note_to_judge_ends_it_on_its_result`) | 원본도 `getPastNotes() == getNotes()`(0 = 0)라 끝난 판으로 넘긴다(`BMSPlayer.java:953-958`). 원본은 판정이 하나도 없는 단일 곡 판의 점수를 만들지 않지만(`:858-861`) 여기서는 사용자 결정(2026-10-15)대로 남기고 보낸다 |
| 리플레이 | 리플레이의 `scratch_auto` 로 재생하면 같은 차트를 같이 빼므로 같은 결과(턴테이블이 초록이고 세지 않는 것 포함)가 나온다. 분석의 되감기(`seek`)도 필드의 차트에서 다시 빼서 같다 | - |

### 10.6 선곡과 결과 화면

1. 선곡: SCRATCH AUTO 로 친 차트도 EX SCORE·RATE·RANK·판정 막대가 채워진다(그 판의 노트 수 기준. 캡처 `select-recorded-scratch-auto.png`). 넓힌 판정으로만 친 차트는 램프와 횟수만 있고 점수는 `-` 다(`select-recorded-widened.png`. 기본 스킨의 `holds_score`, 웨이브 16).
2. 결과 화면은 오른쪽 위에 `SCRATCH AUTO: TURNTABLE NOT COUNTED, LIGHT ASSIST LAMP`(넓힌 판정은 `WIDENED JUDGE: SENT AND KEPT, BUT NO BEST SCORE`)를 노란 글자로 쓴다. 스킨의 숫자만으로는 턴테이블이 빠진 것과 램프가 낮아진 것을 알 수 없기 때문이다. AUTOPLAY·리플레이 재생에는 쓰지 않는다(캡처 `result-recorded-scratch-auto.png`).
3. LOCAL BEST 목표와 스코어 그래프의 BEST, 결과 화면의 이전 점수(스킨과 내장 화면의 이전 EX·이전 최고 EX)는 SCRATCH AUTO 판도 센다. 넓힌 판정의 판과 세대 2 의 SCRATCH AUTO 판은 세지 않는다.
4. PLAY 메뉴의 SCRATCH AUTO 행 설명은 `Let the game play the scratch lane, drawn green; its notes are not counted`, 정보 패널의 덧말은 `WHILE ON: TURNTABLE NOT COUNTED, LIGHT ASSIST LAMP`(`stage/play_menu.rs`). 설정 화면의 JUDGE WIDTH·LN MARGIN 행 설명과 정보 패널은 넓힌 판정이 최고 기록만 바꾸지 않고 리플레이와 함께 남고 보내진다고 말한다.

### 10.7 같은 배치로 다시 하기(파란 건반 + START)

결과 화면에서 파란 건반을 하나라도 누른 채 START 를 누르면 같은 차트를 같은 시드와 같은 옵션으로 다시 시작한다(사용자 요청, 2026-10-14). 이 플레이어의 것이고 원본에는 없다.

| 항목 | 내용 |
| --- | --- |
| 파란 건반 | 7키·14키: 2·4·6번(키 인덱스 1·3·5), 5키·10키: 2·4번(1·3), 9키: 4·6번(3·5. 팝픈 배치는 흰·노랑·초록·파랑·빨강·파랑·초록·노랑·흰), 24키: 없음. DP 는 양쪽 어느 파란 건반이든 된다 |
| START | 키보드의 START 키(기본 `A`)나 패드의 START. 결과 화면이 열릴 때 이미 눌려 있던 START 는 누름으로 세지 않는다 |
| 같은 배치 | 그 판의 시드를 다음 로드에 넘긴다(`Leave::RunSameLayout` → `retry_seed`). 옵션은 설정에 있는 그대로라 그 판과 같다. 7번 키(ReplaySame)와 달리 판의 종류를 가리지 않는다(넓힌 판정 판도 같은 배치) |
| 새 판의 시드 | 다시 하기가 아닌 새 판의 배치 시드는 시계에서 만들되 2^53 미만이고 0 이 아니다(`apps/rbms-player/src/app_play.rs` `fresh_layout_seed`, `LAYOUT_SEED_MASK`). 시드는 점수 서버에 JSON 숫자로 가는데 2^53 을 넘으면 정확한 정수가 아니어서, v0.1.2 까지는(시계의 나노초를 그대로 써서) 서버의 검사가 판 전체를 거절했다(HTTP 400. `docs/bug/2026-10-14-score-submit-400-large-seed.md`). 서버는 이제 안전한 정수 밖의 시드를 0 으로 읽고 판을 받는다(`web/src/server/dto/common.ts` `seedSchema`). 리플레이나 다시 하기로 넘어온 시드는 기록된 그대로 쓴다 |
| 파란 건반의 원래 역할 | 원본 표에서 확인(OK)인 파란 건반(7키의 2·4번 등)은 누를 때가 아니라 뗄 때 확인한다. START 를 옆에서 누를 시간을 주기 위해서다. 혼자 눌렀다 떼면 전처럼 선곡으로 간다. 6번 키(그래프 바꾸기)는 누를 때 그대로 바꾼다 |
| START 만 | 아무것도 하지 않는다(지금 동작 그대로) |
| 7번 키(ReplaySame)·5번 키(ReplayDifferent)·R·N | 그대로 |
| 쓸 수 없는 곳 | 다시 할 수 없는 결과 화면(오토플레이, 리플레이 보기, 라이브러리에 없는 차트)에서는 R 과 같이 아무것도 하지 않는다. 코스는 스테이지마다 결과 화면이 없고(바로 다음 스테이지), 코스 결과 화면에는 이 조합이 없다. 연습은 결과 화면 없이 패널로 돌아간다 |
| 내장 결과 화면 | 같은 조합을 읽는다. 바닥 안내 글자(`ENTER / ESC  SELECT    R  RETRY    N  NEXT SONG`)는 내장 화면의 골든을 바꾸지 않기 위해 고치지 않았다. 결과 화면에는 H 키 안내가 없다 |
| 구현과 테스트 | `apps/rbms-player/src/stage/result.rs`(`blue_keys`, `Scene::step`, `ResultState::built_in_frame`, `leave`), 테스트 `stage/result/tests.rs`(`start_with_a_blue_key_held_runs_the_chart_again_laid_out_as_it_was` 등) |

### 10.8 구현 위치와 테스트

| 내용 | 위치 |
| --- | --- |
| 판이 끝날 때의 저장·전송·리플레이 | `apps/rbms-player/src/app_result.rs`(`enter_result`, 저장은 `record_run`, 포기한 판은 `record_left_run`, 실패 화면에서 다시 한 판은 `record_retried_run`, DB 를 쓸 수 없을 때 `keep_on_the_side`), 행으로 바꾸기는 `apps/rbms-player/src/scoredb_store.rs`(`play_log`, `record_finished_play`, `record_of`, `import_pending_runs`), 파일 쓰기는 `apps/rbms-player/src/save_worker.rs`(10.10) |
| 저장의 실패 경로 | 테스트 `app_result/record_tests.rs`(`a_replay_that_cannot_be_written_leaves_no_replay_on_the_row`, `a_replay_the_save_thread_could_not_write_is_let_go`, `a_run_played_while_the_score_database_is_unavailable_is_kept_and_read_in_later`, `retrying_a_failed_run_keeps_the_failed_run`), `stage/play/screen_tests.rs`(`a_failed_run_started_again_is_kept_first`), `crates/rbms-store/src/scoredb/sync_tests.rs`(`a_play_and_its_submission_body_are_written_together`), `scoredb/tests.rs`(`plays_kept_on_the_side_are_read_in_and_their_files_removed`) |
| Esc 포기 | `apps/rbms-player/src/stage/play/mod.rs`(`leave_run`, `keeps_a_left_run`), `crates/rbms-play/src/session.rs`(`PlaySummary::left_unfinished`), 테스트 `app_result/record_tests.rs`(`leaving_mid_song_keeps_a_failed_run_with_the_rest_as_poor`, `leaving_mid_song_signed_in_sends_the_run`, `leaving_before_any_note_is_judged_keeps_a_failed_run`, `leaving_a_chart_with_no_note_to_judge_ends_it_on_its_result`, `leaving_while_holding_a_long_note_counts_it_once`, `leaving_a_watched_replay_or_autoplay_keeps_nothing`, `leaving_mid_course_keeps_the_stage_and_a_failed_course`, `leaving_a_course_stage_with_no_note_left_ends_it_as_played`), 닫히는 판 `stage/play/tests.rs`(`escape_held_through_a_closing_run_reaches_the_result_screen_with_the_notes_it_judged`), `session.rs` 의 `a_run_left_unfinished_*` |
| 기록 동기화 | 10.9절의 마지막 표 |
| 최고 기록에 합치는 규칙 | `crates/rbms-store/src/scoredb.rs`(`ScoreDb::record_play`, `merge_best`), 행이 최고 기록이 될 수 있는지 `crates/rbms-store/src/score.rs`(`sets_best`, `SCORE_RULE_VERSION`) |
| 최고 기록을 갱신해도 되는 판, 보낼 판 | `apps/rbms-player/src/lib.rs`(`updates_score`, `ir_submission_block_reason`, `assisted_lamp`), 실제로 보내는지 `apps/rbms-player/src/app_ranking.rs`(`sends_to_primary_ir_server`), 코스의 어시스트와 코스 최고 기록 `apps/rbms-player/src/course_ir.rs`(`CourseAssist`, `course_updates_score`, `build_course_submission`) |
| 턴테이블을 판정에서 빼기 | `crates/rbms-play/src/lib.rs`(`Player::with_auto_lanes`, `take_auto_lanes`), `crates/rbms-play/src/session.rs`(`field_timelines`, `auto_lanes`, `has_auto_played_notes`), 테스트 `crates/rbms-play/tests/scratch_auto.rs`(스크래치 없는 차트와 같은 집계, 턴테이블 키, 키음, 리플레이와 되감기, DP, 9키) |
| 초록 그리기 | `crates/rbms-render/src/skin_render/notes.rs`(`LaneNotes::auto_lanes`, `AUTO_LANE_NOTE`), 내장 필드 `crates/rbms-render/src/playfield.rs`. 플레이 화면은 `apps/rbms-player/src/stage/play/mod.rs` 에서 필드의 차트와 레인을 넘긴다 |
| SCRATCH AUTO 가 어시스트인지 | `apps/rbms-player/src/app_result.rs`(`scratch_auto_played`: 세션이 노트를 뺐는지) |
| 손으로 친 판을 끝까지 돌려 DB 를 읽는 테스트 | `apps/rbms-player/src/app_result/record_tests.rs`(로그인 없음, 로그인 있음, 서버는 있고 로그인하지 않음(`ir_submitted` 0), 다음 실행, SCRATCH AUTO 저장과 최고 기록, SCRATCH AUTO 전송 내용, SCRATCH AUTO 스테이지가 든 코스와 넓힌 판정 스테이지가 든 코스(코스로 보내지고 어시스트가 실리는지, 코스 최고 기록), 넓힌 판정, 세대 2 의 행과 결과 화면의 이전 점수, 턴테이블 노트가 없는 차트, AUTOPLAY, FAILED, 리플레이를 본 뒤의 설정, 사용자 DB 사본(`#[ignore]`, `RBMS_SCOREDB_COPY=<사본> cargo test -p rbms-player --lib -- --ignored a_players_own_database`)). 올릴 수 있는 판정 폭은 `ir_replay.rs` 의 `only_a_judge_the_wire_can_state_is_uploaded` |
| 캡처 | `RBMS_SKIN_CAPTURE_DIR` 을 주면 `scratch-auto-play-rbms-default.png`(와 `RBMS_SKIN_PACK` 의 팩 이름), `result-recorded-scratch-auto.png`, `select-recorded-scratch-auto.png`, `select-recorded-widened.png` 를 쓴다(`stage/play/screen_tests/default_pack_tests.rs`, `stage/result/tests.rs`, `stage/select/recorded_tests.rs`) |

### 10.9 기록 동기화(SYNC RECORDS NOW)

> 0.1.4(2026-10-14) 추가. 사용자 결정(2026-10-14): "그냥 서버에 전체 보내서 실패하면 실패한 거만 잘 처리해 놔". 서버 쪽 계약은 `docs/plan/2026-10-14-record-sync.md` 2절이다.

이 기계에 남은 판 중 점수 서버가 아직 받지 않은 판을 모두 서버로 보낸다. 서버는 같은 사용자·차트·같은 1초 안의 `played_at` 을 같은 판으로 보고, 이미 가진 판은 `message: "duplicate submission ignored"` 와 기존 `score_id` 로 받는다(멱등). 그래서 이미 받은 판을 다시 보내도 해가 없고, 판별로 서버의 답을 `ir_sync` 에 적어 다음 동기화가 무엇을 보낼지 정한다. 옛 `ir_submitted` 는 "넘겼음"일 뿐이라 읽지 않는다. 0.1.3 까지의 판은 `ir_sync` 행이 없으므로 "미확인"이고 첫 동기화에서 한 번씩 간다.

| 항목 | 동작 |
| --- | --- |
| 대상 | 기본 서버(`SERVER URL`)에서 `accepted` 가 아닌 판 전부(거절·일시 실패·건너뜀·미확인). 단 저절로 시작한 동기화(시작·로그인·포기한 판)는 서버가 지금과 같은 두 빌드(이 클라이언트의 버전과 실행 파일 해시, 서버가 `/version` 으로 알려 주는 commit. commit 이 없으면 서버 이름과 API 세대, 답이 없으면 빈 값) 아래에서 거절한 판을 다시 보내지 않는다: 같은 클라이언트가 같은 서버에 같은 본문을 보내면 같은 거절이 돌아오기 때문이다(`record_sync::refusal_key`, `SyncWalk::refused_under`). 클라이언트나 서버의 빌드가 바뀌면 다시 보내고, SYNC RECORDS NOW 는 거절된 판을 모두 다시 보낸다(사용자 결정 7 의 "거절된 판도 다음 동기화에서 다시 보낸다"를 "빌드가 바뀌었거나 손으로 누르면"으로 좁혔다. S19, REC-5). 다른 계정으로 친 판(`play_submission.played_as` 가 다른 판)은 뺀다. 게스트로 친 판과 0.1.4 이전의 판(`played_as` NULL)은 처음 동기화하는 계정으로 간다. 세대 2 이하의 SCRATCH AUTO 판(옮겨 온 어시스트 판 포함)은 보내지 않는다(10.4). IR 프로필(`ir_profiles`)로는 동기화하지 않는다. 서버는 SERVER URL 의 앞뒤 공백과 끝의 `/` 를 뗀 문자열로 구별하므로, 서버를 바꾸면 상태가 따로 쌓인다 |
| 순서 | 오래된 판부터(`date`, `id` 순). 단 서버가 답하지 못한(`retry`) 판은 아직 묻지 않은 판 뒤로, 실패한 횟수가 적은 판부터 보낸다. 서버 쪽에서 늘 실패하는 옛 판 셋이 맨 앞을 차지해도 "세 번 연속 실패면 멈춤"에 걸려 뒤 판이 영영 못 나가는 일이 없다(S19, IR-12). 점수 다음에 코스를, 그 다음에 리플레이를 올린다 |
| 고르는 비용 | 보낼 판을 고르는 일은 일꾼 스레드가 읽기 전용 DB 연결을 따로 열어 한다(`ScoreDb::open_read_only`, WAL 이라 프레임 스레드의 쓰기와 함께 돈다). 프레임 스레드는 가장 새 판·코스 id 두 개만 읽고(`newest_play_id`, `newest_course_id`, 수 µs) 일꾼을 띄운다. 일꾼은 그 id 까지만 보내므로(`SyncWalk::up_to`) 동기화가 도는 사이 끝난 판은 그 판을 남긴 쪽(결과 화면의 전송, 포기한 판의 동기화)이 보낸다. 곡 라이브러리 대조는 일꾼이 고른 차트의 md5 만 프레임 스레드에 물어(`SyncEvent::Picked`) 라이브러리 색인으로 찾는다. 세대 2 이하 SCRATCH AUTO 판의 "보내지 않음" 표시는 DB 에 쓰지 않고 상태를 읽을 때 만든다(`ScoreDb::sync_states`) |
| 언제 | 시작할 때 로그인 상태면 세션이 확인된 뒤(`whoami` 성공. 오프라인이면 하지 않는다), 로그인·가입에 성공했을 때, 설정 NETWORK 의 SCORES 묶음 첫 줄 `SYNC RECORDS NOW` 를 눌렀을 때, Esc 로 포기한 판이 남았을 때(그 판 하나). 게스트면 하지 않고, 수동 버튼은 `log in first`(서버가 없으면 `set SERVER URL first`)를 띄운다. 보내는 계정은 서버가 토큰의 주인으로 알려 준 login id(`whoami`·로그인 응답)뿐이다. 설정에 적은 PLAYER ID 는 대신 쓰지 않고(다르면 모든 판이 403 으로 거절된다), 토큰은 있는데 주인을 아직 모르면 수동 버튼은 `account not confirmed yet` 을 띄우고 자동 동기화는 하지 않는다. 이미 돌고 있으면 수동 버튼은 `records sync already running`, 포기한 판은 지금 동기화가 끝난 뒤 이어서 보낸다 |
| 본문 | 판이 끝날 때 적어 둔 제출 본문(`play_submission.body`)을 그대로 보내고 `player.id` 만 동기화하는 계정으로 바꾼다. 본문이 없는 판(0.1.3 까지)은 기록 행으로 다시 만든다(아래 "재구성") |
| 속도 | 백그라운드 스레드에서 보내 화면 프레임을 막지 않는다. 점수는 1.5초에 하나(분당 40, 서버 한도 60), 리플레이는 7.5초에 하나(분당 8, 서버 한도 10). 남는 몫은 동기화 중에 친 판의 몫이다(`record_sync.rs` 의 `SCORE_SEND_INTERVAL`, `REPLAY_SEND_INTERVAL`) |
| 답의 분류(`rbms_ir::submit_verdict`) | 2xx 수락은 `accepted`(중복 응답이면 "이미 있음"으로 센다). 4xx(400·403·404·409·413 등)는 `rejected` 로 오류 코드·필드·이유·HTTP 상태와 거절받은 두 빌드를 적고, 빌드가 바뀐 뒤의 동기화나 SYNC RECORDS NOW 에서 다시 보낸다(위 "대상"). 429 는 60초 기다렸다 같은 판을 다시 보내고, 두 번 기다려도 429 면 그 판을 `retry` 로 적고 동기화를 멈춘다. 네트워크 오류·응답 해석 실패·5xx 는 `retry` 이고 세 번 연속이면 서버가 닿지 않는 것으로 보고 멈춘다. 401·서버 없음은 그 판을 `retry` 로 적고 멈춘다 |
| 리플레이 | 수락된 판 중 로컬 리플레이 파일이 있고 AUTO UPLOAD REPLAY 가 켜져 있을 때 올린다. 서버의 `score_id` 를 실어 그 점수에 묶는다. 상태는 `pending`(아직 안 올림, 설정이 꺼져 있을 때 포함), `uploaded`, `failed`(일시 실패, 다음에 다시), `rejected`(점수의 거절과 같은 규칙: 빌드가 바뀌었거나 손으로 누를 때 다시. 413 처럼 서버 설정이 바뀌어야 받아지는 리플레이가 시작할 때마다 다시 올라가 업로드 몫을 쓰지 않는다), `skipped`(unranked 응답이거나 판정 폭을 와이어가 싣지 못하는 판, `ir_replay::wire_carries_judge`), `missing`(파일이 지워짐), `none`(리플레이 없음). 리플레이는 그 점수를 보낸 계정으로만 올린다 |
| 결과 화면의 전송 | 판이 끝날 때의 전송도 같은 `ir_sync` 열을 쓴다(`AppShared::write_live_outcome`). 그 전송이 끝나기 전에 다음 판이 제출되면 앞 판의 답은 버려지고(전과 같다), 그 판은 행이 없어 다음 동기화가 보낸다(멱등이라 무해). 전송이 아직 들고 있는 판은 그 사이에 시작하는 동기화(버튼, 게스트였다가 로그인)의 대상에서 뺀다. 같은 판을 둘이 겹쳐 보내면 서버가 같은 밀리초의 둘째를 500 으로 거절할 수 있고, 나중에 적는 쪽이 리플레이 상태를 덮기 때문이다 |
| 코스 | 끝날 때 남긴 코스(`course_submission`) 중 그 서버가 받지 않은 것을 점수 다음에 보낸다. 본문은 남긴 그대로이고 `player.id` 만 동기화 계정으로 바꾼다. 다른 계정으로 친 코스는 뺀다. 답의 분류와 거절의 다시 보내기 규칙은 판과 같다. 코스 엔드포인트가 없는 서버(`Unsupported`)는 `skipped`·`NO_COURSE_ENDPOINT` 로 적고 다음에 다시 본다. 이름 붙인 판만 보내는 동기화(포기한 판 등)는 코스를 보내지 않는다 |
| 멈춤·취소 | 로그아웃과 서버를 다시 만들 때(로그인으로 토큰이 바뀔 때, SERVER URL 변경) 돌던 동기화를 취소한다. 이미 받은 답은 그대로 적혀 있다. 보내는 중인 요청은 끊지 않는다(요청 제한 시간 5초) |
| 진행과 결과 | 상태 줄은 진행 중 `syncing records 12/340`·`syncing replays 3/80`, 끝나면 `records: 120 sent, 30 already on server, 5 rejected, 2 failed, 1 skipped; replays: 80 uploaded, 3 failed`(0 인 항목은 뺀다. 보낸 판이 없으면 `records: nothing sent`), 멈추면 `records sync stopped: not signed in (401)`·`records sync stopped: server unreachable`·`records sync stopped: rate limited`·`records sync cancelled` 다. 같은 줄이 메시지로도 뜬다(거절·실패·멈춤이 있으면 경고). 보낼 판이 없으면 수동 버튼만 `records: nothing to send` 를 띄우고 자동 동기화는 조용하다. SYNC RECORDS NOW 행의 값은 도는 동안 `SENDING 12/340`·`REPLAYS 3/80`, 아니면 `>` 다 |
| 판별 이유 | 선곡의 기록 상세(R) 아래쪽 한 줄(56자에서 자른다): `IR: ON SERVER`(초록. 지금 로그인한 계정이 아닌 다른 계정으로 받은 판이면 `IR: ON SERVER (AS <계정>)`(회색)로 말한다. 행은 서버로만 나뉘어 쌓이기 때문이다), `IR: REJECTED VALIDATION_ERROR: judge (judge count exceed...`(빨강. 56자에서 자르므로 이유가 길면 이렇게 끝난다. 필드·이유가 없으면 `IR: REJECTED <코드>`, 코드도 없으면 `IR: REJECTED (HTTP 400)`), `IR: NOT SENT YET (WILL RETRY)`(노랑), `IR: NOT SENT (OLD SCRATCH AUTO RULE)`, `IR: NOT SENT (CHART NOT IN LIBRARY)`(노랑), 행이 없으면 `IR: NOT SENT YET`(로그인) 또는 `IR: NOT SENT (NOT SIGNED IN)`(게스트). 서버 URL 이 없으면 줄이 없다. 리플레이 상태는 줄에 넣지 않는다(모달 높이) |
| 결과 화면의 오류 줄 | 서버의 오류 봉투에 `details.field` 가 있으면 `IR ERROR: server error 400: VALIDATION_ERROR: judge (judge count exceeds total_notes)` 처럼 코드·필드·이유를 쓴다(필드만 있으면 `코드: 필드`, 필드가 없으면 전처럼 `메시지 (코드)`). 결과 줄은 56자에서 `...` 로 자르므로 화면에는 `IR ERROR: server error 400: VALIDATION_ERROR: judge (judge count e...` 로 보인다. 운영 서버는 프로덕션에서 `details` 중 `field`·`reason` 만 내보낸다 |

재구성(본문이 없는 판, `record_sync::rebuilt_submission`)과 그 한계:

1. 기록 행에 있는 것: 판정 12칸, 노트 수, 콤보, BP, 램프, 게이지 종류와 값, 배치(RANDOM)와 시드, 어시스트 단계, 날짜(초), 리플레이 파일 이름. 아티스트·레벨은 곡 라이브러리에서 md5 로 찾는다(없으면 빈 값).
2. 없는 것: fast/slow(0 으로 보낸다), 하이스피드·리프트·커버·녹색 숫자 같은 옵션, 판정 폭(어시스트 2 면 `CUSTOM_JUDGE` 태그로 이미 unranked), 빌드 해시와 플랫폼, 판정 알고리즘, 스킨. 본문의 `extra.reconstructed = true` 가 다시 만든 본문임을 말한다.
3. 시각: 리플레이 파일 이름(`<md5 앞 8자>-<밀리초>.ron`)의 밀리초가 기록의 초와 같으면 그 밀리초, 아니면 `날짜 × 1000`. 서버는 같은 1초 안을 같은 판으로 보므로 이미 받은 판이 중복 저장되지 않는다(서버가 이 규칙을 배포한 뒤에만. 배포 순서는 계약 문서 2.3절).
4. 어시스트: 1 이면 `scratch_auto = true` 와 `AUTO_SCRATCH`, 2 면 `CUSTOM_JUDGE`. RON 에서 옮겨 온 행의 알 수 없는 어시스트(-1)는 `UNRECORDED_ASSIST` 태그로 보내 서버가 unranked 로 받는다.
5. LN 종류: 기록의 LN 키가 LN·CN·HCN 이면 그대로, 차트가 롱노트를 정한 판(`CHART`)은 LN(0)으로 보낸다. CN·HCN 을 선언한 차트의 옛 판은 서버에 LN 으로 적힌다(곡 라이브러리에 `#LNMODE` 가 없어 알 수 없다).
6. sha256: `scores.ron` 에서 옮겨 온 행은 sha256 이 비어 있다. 라이브러리에서 md5 로 차트 파일을 찾아 다시 해시하고 md5 가 같으면 그 sha256 을 쓰고 기록 행에도 채운다(`backfill_sha256`). 차트가 라이브러리에 없으면 보내지 않고 `skipped`·`NO_CHART_HASH` 로 적어 다음 동기화에서 다시 본다.
7. 운영 서버가 빌드 해시를 요구하도록 설정돼 있으면(`REQUIRE_BUILD_HASH`) 재구성한 판은 `client_build_sha256` 으로 거절돼 `rejected` 로 남는다. 운영 값은 확인하지 않았다.

| 내용 | 위치 |
| --- | --- |
| 표와 쿼리 | `crates/rbms-store/src/scoredb/schema.sql`(`play_submission`, `ir_sync`, `course_submission`, `course_sync`. 스키마 세대는 1 그대로다: 표와 열만 더하는 변경이라 0.1.3 으로 되돌려도 DB 가 열린다. 열은 `schema.rs` 의 `ADDED_COLUMNS`), `crates/rbms-store/src/scoredb/sync.rs`(`SyncWalk`, `sync_candidates`, `replay_candidates`, `put_sync_state`, `put_replay_state`, `sync_states`, `backfill_sha256`, `put_submission`, `KeptSubmission`), `scoredb/courses.rs`(`put_course`, `course_candidates`, `put_course_sync_state`), 테스트 `scoredb/sync_tests.rs` |
| 답의 분류와 오류 줄 | `crates/rbms-ir/src/sync.rs`(`submit_verdict`, `upload_verdict`), `crates/rbms-ir/src/error.rs`(`ServerReason`, `readable_detail`), `crates/rbms-ir/src/dto/score.rs`(`DUPLICATE_SUBMISSION_MESSAGE`, `mark_reconstructed`) |
| 일꾼과 화면 | `apps/rbms-player/src/record_sync.rs`(`spawn_planned_sync`, `run_record_sync`, `refusal_key`, `course_outcome`, `rebuilt_submission`, `AppShared::start_record_sync`·`poll_record_sync`·`sync_records_now`·`record_ir_line`), 끝난 코스 남기기와 전송 `app_ranking.rs`(`keep_and_send_course`, `poll_course_submits`), 결과 화면 전송의 상태 `apps/rbms-player/src/app_ranking.rs`(`LiveSubmit`, `write_live_outcome`), 자동 시작 `app_network.rs`(`poll_auth`, `poll_whoami`), 버튼 `stage/settings.rs`·`stage/settings/catalog.rs`, 기록 상세 `stage/select/scene.rs` 와 `crates/rbms-render/src/select.rs`(`SelectModal::ir_line`) |
| 테스트 | `apps/rbms-player/src/record_sync/tests.rs`(가짜 서버로 본문·재구성·시각·어시스트·sha256 채우기·리플레이·429·일시 실패·401·거절·취소·로그인과 시작 때의 자동 시작·게스트·로그아웃·이어 보내기·화면 줄·입력한 PLAYER ID 를 계정으로 쓰지 않음·결과 화면 전송 중인 판 빼기·이름 붙인 판의 동기화가 상태를 다시 읽지 않음·다른 계정으로 받은 판의 줄), `app_result/record_tests.rs`(`a_finished_run_stores_its_submission_body`, `a_live_submission_writes_its_sync_state`, `a_finished_course_is_kept_even_when_the_server_does_not_answer`, `a_course_the_server_failed_to_take_is_written_down_to_send_again`), S19 의 거절·순서·선정 테스트 `record_sync/tests.rs`(`an_automatic_sync_does_not_resend_a_rejection_the_same_server_build_gave`, `a_new_client_build_resends_every_rejection`, `a_rejected_replay_is_not_uploaded_again_by_an_automatic_sync`, `plays_that_always_fail_do_not_hold_back_the_plays_after_them`, `starting_a_full_sync_picks_nothing_on_the_frame_thread`), `scoredb/sync_tests.rs`(`a_refusal_under_the_same_builds_is_left_out_of_a_walk_under_them`, `plays_the_server_keeps_failing_go_behind_the_rest`, `a_walk_reaches_no_further_than_the_play_it_was_bounded_by`, `a_kept_course_is_sent_until_the_server_holds_it`), `stage/settings/layout_tests.rs`(`the_scores_section_starts_with_sync_records_now`, `sync_records_now_while_signed_out_says_log_in_first`), `crates/rbms-ir/src/error.rs`·`sync.rs`, `crates/rbms-store/src/scoredb/sync_tests.rs` |

### 10.10 저장 스레드와 리플레이 보존

> S19(2026-10-15) 추가. 결과 진입·Esc 이탈·다시 하기·플레이 중 도착한 네트워크 응답이 프레임 스레드에서 fsync 를 기다리던 것(PERF-4·PERF-5)과, 리플레이 정리가 아직 올라가지 않은 리플레이를 지우던 것(REC-7)을 고쳤다.

| 항목 | 동작 |
| --- | --- |
| 저장 스레드 | 설정 파일 저장(`AppShared::save_settings`), 리플레이 쓰기, DB 를 쓸 수 없을 때 따로 남기는 판은 모두 한 스레드(`rbms-save`)에 넘기고 프레임은 기다리지 않는다(`apps/rbms-player/src/save_worker.rs`). 한 스레드가 넘긴 순서대로 쓰므로 같은 파일을 두 스레드가 겹쳐 쓰는 일이 없다. 아직 쓰지 않은 설정 저장을 더 새 설정 저장이 대신하면 앞의 것은 쓰지 않는다. 단 `flush` 앞에 넘긴 저장은 그 뒤의 저장이 대신하지 않으므로 `flush` 가 돌아오면 그 앞의 쓰기는 디스크에 있다. 실패는 다음 프레임에 프레임 스레드로 돌아와 메시지를 띄운다(`AppShared::poll_saves`). 이벤트 루프가 끝날 때(`ApplicationHandler::exiting`) 남은 쓰기를 모두 끝낸다(`AppShared::finish_saves`). macOS 의 메뉴 Quit·Cmd+Q·Dock 종료는 루프 안에서 곧장 프로세스를 끝내 `run` 으로 돌아오지 않고 `Drop` 도 돌지 않으므로 이 자리가 마지막 기회다. `run` 의 `finish_saves` 와 `SaveWorker` 의 `Drop` 도 그대로 남는다 |
| 임시 파일 이름 | `write_atomic` 의 임시 파일은 `<이름>.<프로세스 id>.<프로세스 안 일련번호>.tmp` 다. 같은 프로세스의 두 스레드가 같은 파일을 저장해도 서로의 임시 파일을 덮지 않는다(`crates/rbms-store/src/atomic.rs`) |
| DB 쓰기 | 판 행과 제출 본문 쓰기(SQLite, WAL·`synchronous=NORMAL` 이라 한 판 약 0.1ms)와 동기화 답 쓰기는 프레임 스레드에 남겼다. 동기화 대상 선정은 일꾼이 한다(10.9 "고르는 비용") |
| 단위 테스트 | 단위 테스트의 앱은 저장을 부른 자리에서 바로 쓴다(`SaveWorker::for_this_build`). 저장 직후 파일을 읽는 테스트가 결정적이도록 하기 위해서다. 스레드 자체와 "프레임 스레드가 쓰지 않는다"는 것은 스레드를 쓰는 테스트가 따로 본다(`save_worker/tests.rs`, `record_tests.rs` 의 `entering_the_result_writes_no_file_on_the_frame_thread`) |
| 리플레이 보존 | 시작할 때의 리플레이 정리(차트당 최근 3개와 최고 기록, 합계 2GiB)는 서버가 설정돼 있고 로그인 계정이 있으며 AUTO UPLOAD REPLAY 가 켜져 있으면, 그 서버·계정으로 아직 올라가지 않은 리플레이(점수가 아직 수락되지 않았거나 리플레이가 `pending`·`failed`·`rejected` 인 판, 다른 계정의 판과 세대 2 이하 SCRATCH AUTO 판 제외)를 최근 것처럼 남긴다. 바이트 한도는 그대로 앞선다(`ReplayPolicy`, `UploadHold`, `scoredb_store::upload_hold`). 서버가 없거나 로그아웃 상태거나 업로드가 꺼져 있으면 전처럼 정리한다 |
| 테스트 | `save_worker/tests.rs`(대신한 설정 저장 건너뛰기, 순서, 실패 전달, 끝날 때 남김없이 쓰기, 플레이 중 도착한 응답의 설정 저장), `crates/rbms-store/src/tests.rs`(`threads_of_one_process_writing_one_file_never_share_a_temp_file`, `a_staged_write_fails_where_its_file_cannot_be_created_and_cleans_up_when_dropped`), `scoredb/tests.rs`(`replay_gc_keeps_a_replay_still_to_go_up`) |

설정 화면: NETWORK 묶음이 스크롤 없이 한 화면에 다 보이도록(설정 화면의 규칙이다, `layout_tests.rs` 의 `every_group_lists_its_rows_under_the_headings_of_its_sections`) 이 행을 더하면서 SERVER 와 ACCOUNT 두 머리를 `SERVER AND ACCOUNT` 하나로 합쳤다.


### 10.11 기록하는 값의 정의와 규칙 세대 4(2026-10-15)

규칙 세대 `SCORE_RULE_VERSION` 4(`REFERENCE_RECORD_RULE_VERSION`)부터 한 판의 기록은 원본이 기록하는 값이다(`BMSPlayer.java:863-930`). 결정은 `docs/acknowledge/2026-10-15-adr-*.md` 다섯 건. 같은 릴리스의 엔진 변경(입력 시각 판정, 롱노트 정리로 바뀐 노트 수)도 세대 4 에 든다(아래 표의 마지막 두 행, ADR 세 건).

| 값 | 세대 4 의 정의 | 세대 3 이하 행 |
| --- | --- | --- |
| `avgjudge` | 노트마다 입력과의 거리(놓친 노트·도달하지 못한 노트는 1초)의 평균, µs(`PlayRecord::avg_duration_us`). 작을수록 정확하다. 기록 DB, 결과 화면의 전송, 기록 동기화가 다시 만든 본문이 같은 값을 낸다. 부호 있는 평균(빠르면 양수)은 본문 `extra.mean_offset_us` 로 따로 보낸다 | 맞은 입력의 부호 있는 평균의 절댓값. 이력으로 남고 최고 기록 경쟁에 들어가지 않는다. 다시 보낼 때는 0(미상) |
| `minbp` | BAD+POOR+空POOR 에 게이지가 비어 도달하지 못한 노트를 더한 값(`PlaySummary::min_bp_with_unreached`) | 판정 수에서 도달하지 못한 노트(노트 수 − PG·GR·GD·BD·POOR, pop'n 은 BD 를 빼고 센다)를 계산해 더한 값으로 비교·재전송한다(`rbms_store::scoredb::charged_minbp`) |
| `combo`(코스 스테이지) | 그 스테이지만의 최대 콤보. 화면의 콤보와 다음 스테이지는 코스 콤보를 이어간다 | 이어받은 콤보가 섞였을 수 있다. 그대로 둔다 |
| 풀콤 램프 | 끝에 남은 콤보가 지나간 노트 수와 같을 때(5K·pop'n 의 마지막 노트 뒤 空POOR 는 풀콤을 깬다) | 그대로 둔다 |
| 코스 램프 | 코스의 최장 연속이 지금까지의 모든 노트면 풀콤 계열, 아니면 게이지 램프(`PlaySession::course_lamp`) | 해당 없음(코스 기록은 서버로만 보낸다) |
| TOTAL 을 직접 정한 판 | 어시스트 2(10.2) | 다른 판과 구분할 수 없어 그대로 둔다 |
| 판정 시각 | 입력은 그 입력의 시각, 시간으로 진행되는 판정은 기한이 된 순간에 판정한다. 게이지가 빈 순간부터 판정하지 않는다. 컨트롤러는 gilrs 가 찍은 시각이라 예전보다 최대 한 프레임 이르다(3.4절) | 프레임 경계가 정하던 곳만 다르다. 그대로 비교한다 |
| 노트 수 | 같은 레인 롱노트 몸통 안의 일반 노트는 배경음, 끝나지 않는 머리는 지운다(곡 DB `PARSER_VERSION` 3) | 그때의 노트 수(`max_ex`·판정 수)로 남고 그대로 비교한다. 영향받는 차트(수십 개 이하)만 EX 의 만점이 다르다 |

- 최고 기록의 `avgjudge`·`minbp` 는 DB 를 열 때마다 이력에서 다시 접는다(`ScoreDb::migrate` → `refold_reference_bests`). 옛 빌드가 그 사이에 같은 DB 에 옛 뜻의 값을 접어 넣어도 다음 실행에서 바로잡힌다. 값이 이미 맞는 행은 쓰지 않는다.
- 제출 본문은 `extra.rule_version` 에 그 판의 세대를 싣는다. 서버는 이것이 없는 행(옛 클라이언트)의 `avgjudge` 를 부호 있는 평균으로 읽는다.
## 11. 목표 점수(TARGET, TARGET FALLBACK, TARGET RATE)

> 웨이브 16(2026-10-14) 추가. "TARGET 을 LOCAL BEST 로 두어도 AAA 를 목표로 점수가 계산된다"는 보고를 점검하고, 목표가 없을 때 무엇으로 물러날지를 고르게 한 것이다.

### 11.1 목표 16종과 기준

목표는 플레이 중 점수 그래프의 TARGET 막대, 판정 글자 위의 점수 차(`+0004`), 결과 화면의 TARGET 이 모두 같은 값으로 쓴다. 고르는 곳은 PLAY 메뉴 > CHART 의 TARGET 행, 선곡의 옵션 패널 1(타깃 키), 스킨의 TARGET 버튼이다. 모두 같은 값(`judge.target`)을 고친다.

| 목표 | 기준 | 필요한 것 | 점수가 없을 수 있나 |
| --- | --- | --- | --- |
| MAX | 차트의 이론 최대 EX(노트 수 x 2) | 노트 수 | 아니오 |
| RATE A- ~ RATE MAX-(10종) | 이론 최대의 17/27 ~ 26/27(원본 `RANK A-` ~ `RANK MAX-`). 정수로만 나눠 올림 | 노트 수 | 아니오 |
| RANK NEXT | 내 최고 기록 바로 위의 등급 경계(없으면 처음부터) | 노트 수, 로컬 기록 | 아니오 |
| RATE CUSTOM | 이론 최대의 TARGET RATE 몫(올림). 이름은 `SCORE RATE 85%` | 노트 수, TARGET RATE | 아니오 |
| LOCAL BEST | 이 차트의 로컬 최고 EX(같은 LN 종류). 넓힌 판정 폭의 판과 웨이브 16 까지의 SCRATCH AUTO 판은 최고 기록이 아니라서 세지 않는다. 웨이브 18 부터의 SCRATCH AUTO 판은 센다(10절) | 로컬 기록 | 예: 처음 치는 차트, 넓힌 판정이나 웨이브 16 까지의 SCRATCH AUTO 로만 친 차트 |
| IR BEST | 점수 서버가 가진 내 최고 EX | 선곡 패널이 받아 둔 순위표 | 예: 서버 없음, 로그인 안 함, 아직 못 받음 |
| RIVAL | 라이벌 가운데 가장 높은 점수. 이름은 `RIVAL <이름>` | 같은 순위표의 라이벌 줄 | 예: 라이벌이 그 차트를 안 쳤음 |

### 11.2 물러나는 순서와 TARGET FALLBACK

점수를 이름으로 삼는 목표에 그 점수가 없으면 추측하지 않고 물러난다. 화면은 실제로 쓰는 목표의 이름을 쓴다.

| 고른 목표 | 첫 번째 | 두 번째 | 마지막 |
| --- | --- | --- | --- |
| LOCAL BEST | 로컬 최고 | TARGET FALLBACK | 이론 최대(안전망. 행이 내놓지 않는 값일 때만) |
| IR BEST | 서버의 내 최고 | 로컬 최고(같은 사람의 최고라서 같은 종류의 숫자다) | TARGET FALLBACK |
| RIVAL | 라이벌 점수 | TARGET FALLBACK(내 기록은 남의 점수의 대신이 아니다) | 이론 최대 |
| 그 밖의 목표 | 자기 자신 | - | - |

- TARGET FALLBACK 은 언제나 계산되는 목표만 고른다: MAX, RATE A- ~ RATE MAX-, RANK NEXT, RATE CUSTOM(13종). 기본값은 RATE AAA 다. 파일에 점수를 이름으로 삼는 목표가 적혀 있으면 기본값으로 읽는다.
- **동작이 바뀐 곳**: 웨이브 16 전에는 물러나는 끝이 이론 최대(MAX)였다. 지금은 기본이 RATE AAA 다. LOCAL BEST 를 고르고 어시스트로만 치는 사람은 이전에는 MAX 를 따라 달렸고 지금은 RATE AAA 를 따라 달린다. 이전 동작이 필요하면 TARGET FALLBACK 을 MAX 로 둔다. 기존 설정 파일에는 `target_fallback` 이 없고 기본값(RATE AAA)으로 읽는다.
- **원본과 다른 점**: 원본에는 LOCAL BEST 가 목표로 없다(자기 최고는 BEST 막대가 따로 그린다). 라이벌·IR 목표에 점수가 없으면 원본은 이름을 `NO DATA`/`NO RIVAL` 로 하고 목표를 0 으로 둔다(`TargetProperty.java:232-240`). 여기서는 0 을 쓰지 않고 물러난다. 물러나는 순서를 끄는 설정은 두지 않았다: IR BEST 가 로컬 최고를 거치는 것은 같은 종류의 숫자라서이고, RIVAL 이 거치지 않는 것은 남의 점수를 내 기록으로 바꾸면 비교가 아니기 때문이다.
- 한 판의 목표는 시작할 때 한 번 정해지고 판이 끝날 때까지 그대로다. 결과 화면은 같은 규칙으로 같은 값을 쓴다. 손으로 친 판, AUTOPLAY, 리플레이 재생, 코스의 스테이지 모두 같은 규칙이다(코스의 스테이지는 차트마다 다시 정한다).

### 11.3 화면이 말하는 것

| 곳 | 보이는 것 |
| --- | --- |
| 기본 스킨 플레이 화면, 스코어 그래프의 범례 | 실제로 쓰는 목표의 이름(`LOCAL BEST`, `RATE AAA`, `SCORE RATE 70%`, `RIVAL friend`). 고른 목표에서 물러났으면 무엇이 없었는지를 붙인다(`RATE AAA (NO RECORD)`, `LOCAL BEST (NO IR SCORE)`, 웨이브 16 통합). 스킨 문자열 id 3(`STRING_TARGET_NAME`)이 플레이 중에는 이 글자다. 다른 스킨 팩도 같은 id 로 읽는다. 결과 화면의 TARGET 은 이름만 쓴다 |
| 판정 글자 위의 점수 차, 그래프의 TARGET 숫자 | 그 목표 대비 EX 차이(11.1의 값) |
| 결과 화면의 TARGET | 같은 목표의 이름과 EX |
| 선곡의 옵션 패널 1 의 TARGET | 고른 목표. 커서가 있는 차트에 그 점수가 없으면 `LOCAL BEST (NO RECORD: AAA)`, `IR BEST (NO IR SCORE: LOCAL BEST)`, `RIVAL (NO RIVAL SCORE: AAA)` 처럼 대신 쓰는 것을 붙인다. 로컬 기록은 플레이가 셀 기록만 센다: 지금 LN MODE 로 친 기록과 차트가 LN 종류를 적은 기록(`CHART`)의 최고다(`local_best_before_reading`). 한 차트의 기록은 둘 중 한 꼴뿐이라 플레이가 세는 값과 같다. 웨이브 16 통합 전에는 LN 종류와 무관하게 세서, 다른 LN MODE 로만 친 차트에서 선곡은 기록이 있다고 말하고 플레이는 물러났다 |
| PLAY 메뉴의 TARGET 행 | `LOCAL BEST > AAA` 꼴로 점수가 없을 때 쓰는 것을 붙이고(예시 차트에는 기록이 없으므로 미리보기가 그 값이다), 정보 패널이 `NO RECORD: PACED AGAINST RATE AAA INSTEAD` 를 한 줄 더 쓴다. IR BEST 는 실제 순서대로 `IR BEST > LOCAL > AAA` 와 `NO IR SCORE: YOUR LOCAL BEST, THEN RATE AAA` 다(웨이브 16 통합 전에는 로컬 기록 단계를 빠뜨렸다) |
| PLAY 메뉴의 TARGET FALLBACK, TARGET RATE 행 | TARGET 바로 아래. 각 목표가 무엇을 기준으로 하는지와 물러나는 규칙은 행의 설명에 있다. 미리보기가 바뀐 값을 바로 따른다. TARGET 과 TARGET FALLBACK 이 모두 RATE CUSTOM 이 아니면 TARGET RATE 의 정보 패널이 `NOT IN USE: NO TARGET IS RATE CUSTOM` 을 쓴다 |

TARGET 과 두 행은 같은 묶음 하나에만 둘 수 있어서(`SETTINGS` 표는 행마다 한 묶음) PLAY 메뉴 > CHART 에만 있다. SETTINGS(Tab)에서는 왼쪽 레일의 PLAY MENU 단추로 간다.

### 11.4 점검 결과: LOCAL BEST 인데 AAA 로 보인다는 보고

사용자의 기록 DB 복사본과 설정으로 재현한 값이다(`apps/rbms-player/src/app_result/target_tests.rs`).

| 상황 | 이전의 목표 | 이름표 |
| --- | --- | --- |
| LOCAL BEST, 손으로 친 판이 있는 차트(EX 21) | LOCAL BEST = 21 | 이름표 없음(기본 스킨은 "TARGET" 고정) |
| LOCAL BEST, 어시스트로만 친 차트(이 사용자: 웨이브 16 점검 때 판 전부가 SCRATCH AUTO. 웨이브 18 의 재측정은 37판·29 차트, 10.4) | MAX = 36(차트 이론 최대) | 이름표 없음 |

- 원인: 이 사용자는 모든 판을 SCRATCH AUTO 로 쳐서 어느 차트에도 로컬 최고가 없다(어시스트 판은 최고 기록이 아니다. 10절). 그래서 LOCAL BEST 는 늘 물러나고, 기본 스킨은 "TARGET" 이라는 고정 글자만 그려 무엇으로 물러났는지 알 수 없었다.
- **재현되지 않은 것**: 코드의 어느 길도 LOCAL BEST 를 RATE AAA 로 바꾸지 않는다. 보고의 "AAA" 는 코드로 설명되지 않는다. 가능한 근거는 두 가지이고 모두 고쳤다. (1) 파일의 목표 글자가 알 수 없는 꼴이면 기본값 RATE AAA 로 읽혔다: 이제 라벨(`LOCAL BEST`, `local_best`)도 같은 목표로 읽는다. (2) 물러나는 값이 이제 기본으로 RATE AAA 이므로 같은 상황에서 화면이 RATE AAA 를 따르고, 그 이름을 이름표가 말한다.
- 결과 화면의 TARGET 은 웨이브 16 의 앞 단위에서 플레이 화면과 같은 값(어시스트 제외, 같은 LN 종류의 최고)으로 맞췄다(10.6).
- 웨이브 18 부터는 SCRATCH AUTO 판도 최고 기록이 되므로, 이 사용자가 새로 치는 판은 LOCAL BEST 가 된다. 웨이브 16 까지의 판(37판)은 그대로 세지 않는다(10.4).

### 11.5 원본의 목표 목록과 빠진 것

원본(`TargetProperty.java`)의 목표를 이 플레이어와 대조한 표다.

| 원본 id | 원본 이름 | 필요한 것 | 이 플레이어 |
| --- | --- | --- | --- |
| `MAX`, `RATE_A-` ~ `RATE_MAX-` | `MAX`, `RANK A-` ~ `RANK MAX-` | 노트 수 | 있음(MAX, RATE A- ~ RATE MAX-) |
| `RATE_<0~100 실수>` | `SCORE RATE n%` | 노트 수 | **웨이브 16 에서 더함**: RATE CUSTOM 과 TARGET RATE(정수 1~100%, 원본은 실수 0~100. `TargetProperty.java:142-146`) |
| `RANK_NEXT` | `NEXT RANK` | 내 기록, 노트 수 | 있음(RANK NEXT) |
| `RIVAL_n` | `RIVAL <이름>` | 라이벌 n 번째의 점수 | 가장 높은 라이벌 하나만(RIVAL). 순번은 없음. 필요한 것: 라이벌 목록의 순서를 고르는 설정 |
| `RIVAL_RANK_n` | `RIVAL RANK n` | 라이벌들과 내 기록을 점수로 정렬 | 없음. 필요한 것: 라이벌 점수 전부(순위표 캐시의 라이벌 줄에 있다)와 내 로컬 기록, 순번을 고르는 행 |
| `RIVAL_NEXT_n` | `RIVAL NEXT n` | 같음. 내 바로 위 n 번째 | 없음. 필요한 것은 위와 같음 |
| `IR_NEXT_n` | `IR NEXT nRANK` | 서버 순위표 전체와 내 점수 | 없음. 필요한 것: 순위표 전체(지금 캐시는 상위 일부와 내 줄, 라이벌 줄) |
| `IR_RANK_n` | `IR RANK n` | 서버 순위표의 n 위 | 없음. 필요한 것: 순위표 n 위까지 |
| `IR_RANKRATE_n` | `IR RANK TOP n%` | 순위표와 총 인원 | 없음. 필요한 것: 총 인원과 순위표 |
| (원본에 없음) | - | 로컬 기록 | LOCAL BEST. 원본은 자기 최고를 BEST 막대로 따로 그린다 |
| (원본에 없음) | - | 서버의 내 최고 | IR BEST |

서버 자료가 필요한 것은 이번에 서버를 고치지 않아 더하지 않았다. 순위표 전체를 받는 길이 생기면 `target::TargetContext` 에 순위표를 넓히고 `ScoreTarget` 에 순번을 가진 목표를 더한다.

### 11.6 구현 위치와 테스트

| 내용 | 위치 |
| --- | --- |
| 값 계산과 물러나는 순서 | `apps/rbms-player/src/target.rs`(`resolve`, `settle`, `target_ex`, `target_name`. 단위 테스트가 목표 16종 x 대신 13종을 훑는다) |
| 설정 필드, 토큰, 라벨, 짧은 이름 | `crates/rbms-config/src/judge.rs`, `schema.rs`, `settings.rs`, 테스트 `tests.rs`(옛 파일과 라벨로 적힌 파일, 파일 왕복) |
| 한 판의 목표(손으로 친 판·어시스트·순위표) | `apps/rbms-player/src/app_result/target_tests.rs` |
| 플레이 화면의 목표(모든 숫자와 이름, 리플레이·AUTOPLAY·코스) | `apps/rbms-player/src/stage/play/target_tests.rs`, 기본 스킨 캡처 `stage/play/screen_tests/default_pack_tests.rs` |
| 스킨 문자열 | `apps/rbms-player/src/skin_host/options/tests.rs` |
| 선곡의 TARGET 문구와 옵션 패널 캡처 | `apps/rbms-player/src/stage/select/target_tests.rs` |
| PLAY 메뉴의 두 행과 정보 패널의 덧말(IR BEST 순서, TARGET RATE 미사용, SCRATCH AUTO) | `apps/rbms-player/src/stage/play_menu/tests/target.rs` |
| 선곡이 세는 로컬 기록(LN 종류) | `apps/rbms-player/src/app_result/target_tests.rs`(`the_local_best_the_browser_counts_is_the_one_a_run_would_count`) |
| 캡처 | `RBMS_SKIN_CAPTURE_DIR` 을 주면 `target-play-local-best.png`, `target-play-no-record.png`, `target-select-local-best.png`, `target-select-no-record.png`, `target-play-menu-*.png`(웨이브 16 통합에서 `-ir-best-note`, `-rate-unused`, `-scratch-auto` 를 더함) 를 쓴다 |
