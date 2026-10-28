# 플레이 옵션

> 최종 갱신 2026-10-29 · 대응 커밋 `cf8afe5`·`435839e`·`c38a7ea`·`9f1b484`(웨이브 1). 반영한 변경: 2026-10-29 선곡에서 배치를 써서 미리 보고 치는 LAYOUT PICK(새 10.16절, 5.1절의 키 행), 플레이 배속 PLAYBACK RATE(새 18절, 2절 표의 새 행, 7.2·7.4·7.5절의 새 행, 9.3절의 연습 수리, 10.2절의 판의 종류 새 행), 선곡 정보창의 새 배치와 NPS 평균·최고, 글자 넘침 overflow 3(17.2절), 선곡 음량 슬라이더가 진짜 마스터를 움직이고 미리듣기 키음이 Key 버스로 울리는 것(새 17.6절), 2026-10-24 UI 언어 LANGUAGE(새 15절), 선곡의 랭킹 모드와 설정 RANKING KEYS(새 16절), IIDX 식 기본 스킨 선곡 화면(새 17절), 리플레이의 배치로 치기(새 10.15절), 코스 막대 위에서 연습 키가 코스를 시작(9절, 13.3절의 모달 문구 언어), 2026-10-23 노트를 친 판의 Esc 는 결과 화면(코스 도중은 FAILED 코스 결과까지. 10.2절의 도중 포기 행, 10.6절 5번), SCRATCH AUTO COLOR 행과 색 접이(2절, 7.2절, 10.5절 끝), COURSE GAUGE 설정과 코스 시작 게이지·9칸 이월·ln/cn/hcn 제약·제출 게이지(새 10.14절, 10.13절 4번), 선곡 패널 키의 스킨 opt-in(12.2절, 12.7절), 없는 곡 보이기·다운로드 폴더·다운로드 모달(새 13절), 연습 패널의 GAUGE CATEGORY 에 IIDX(9.1절), 2026-10-22 선곡 옵션 패널 1~3 의 새 배치와 켬·끔 토글, F1 오버레이 홀드 키(새 12절, 2절 표의 "바꾸는 곳" 칸), 램프 등급과 코스 스테이지의 게이지(새 10.13절, 10.1·10.2·10.6·10.12절의 해당 칸), PLAY 메뉴의 색 행 접이(7.2·7.3·7.4절), 2026-10-20 과거 판 trace 소급(웹 AI 기능 W6, 10.12절의 소급·소급 실행·설정·테스트·남은 것 행, 10.1절의 `play_trace` 행), 2026-10-20 판별 trace(웹 AI 기능 W4 클라이언트, 10.1절의 `play_trace`·`ir_sync`·`pending-runs` 행, 10.9절의 trace 행, 새 10.12절), 2026-10-19 기록 목표의 진행 중 값이 그 기록 판의 노트별 판정을 따름(11.7절, 8절의 점수 차 행), 2026-10-19 노트를 하나도 치지 않은 판은 Esc·앱 종료로 남기지 않음(10.2절의 도중 포기·앱 종료 행), 2026-10-15 S19 의 입력 시각과 KEY·PAD DEBOUNCE(3.4절), 기록 정의(규칙 세대 4: avgjudge·BP·코스 스테이지 콤보·풀콤·TOTAL 어시스트, Esc 판은 모두 저장과 코스 도중 Esc: 10.2절의 FAILED·TOTAL·Esc 행, 새 10.11절), 0.1.4 의 기록 동기화와 Esc 포기 판 저장(10.1·10.2·10.4·10.8절과 새 10.9절, 커밋 `5f4a9a2`), 저장 스레드와 리플레이 보존(10.10절), 웨이브 20(점수 서버가 SCRATCH AUTO 판을 순위에 올리되 기본 목록에서는 숨긴다: 10.2절의 SCRATCH AUTO·코스 행. 결과 화면은 그 순위를 `IR: RANK #n (SCRATCH AUTO)` 로 쓴다), 웨이브 18(SCRATCH AUTO 의 새 규칙과 모든 판의 저장·전송: 10절, 같은 배치로 다시 하기: 10.7절), 웨이브 16(기록이 저장되는 조건과 SCRATCH AUTO: 10절, 목표 점수와 TARGET FALLBACK·TARGET RATE: 11절, 리플레이를 본 뒤 설정이 그대로 남는 것: 10.2절), 웨이브 14(플레이 중 조작: START + 턴테이블의 1 단위 걸음과 리프트 조합, 판정 오프셋의 출하 키와 누름 반복, 스크래치 역방향 키의 출하: 2절의 "바꾸는 곳" 열, 5절), 웨이브 13(판정 오프셋을 원본의 노트 표시 타이밍으로: 1~3절, 7.4절. 기본 스킨의 판정선을 필드 맨 아래로: 8절), 웨이브 12(연습 패널의 숫자 행: 9절)
> 대상: 플레이 중 스크롤·가림막·판정 타이밍을 정하는 옵션과 그 키, 그 옵션을 바꾸는 화면인 PLAY 메뉴(7절), 기본 스킨의 판정 표시(8절), 연습 패널(9절), 선곡의 옵션 패널 1~3(12절), 언어(15절), 랭킹 모드(16절), 선곡 화면의 새 구성과 음량(17절), 선곡의 배치 지정(10.16절), 플레이 배속(18절). 스킨이 그리는 옵션(Lane Width 등)과 부위별 불투명도는 [skin.md](./skin.md) 3.4절, 구현 위치는 [crates.md](./crates.md)를 본다. PLAY 메뉴의 설계는 [plan/2026-10-11-play-menu.md](./plan/2026-10-11-play-menu.md).
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
| HI-SPEED FIX | 녹색 숫자를 계산할 때 기준으로 삼는 BPM | OFF / START / MAX / MAIN / MIN | MAIN | PLAY 메뉴 > SPEED, 선곡 패널 1 의 5번 키(다음)·6번 키(이전. 12.2절) | `PlayConfig.fixhispeed` | KEEP GREEN NUMBER 가 꺼져 있으면 한 판 안에서만 이동 시간을 묶는다(곡 사이에는 HI-SPEED 가 그대로). 원본은 OFF 가 아니면 곡 사이에도 묶는다. 그 역할은 아래 KEEP GREEN NUMBER 가 맡는다 |
| KEEP GREEN NUMBER | 녹색 숫자를 곡과 곡 사이에 유지한다. 켜면 매 곡 시작과 매 프레임에 HI-SPEED 가 `240000 / 기준 BPM / 이동 시간 * (1 - 레인 커버)` 로 다시 계산된다 | 켬 / 끔 | 끔 | PLAY 메뉴 > SPEED, 플레이 중 KEEP GREEN NUMBER ON/OFF 키(출하 시 미지정), 선곡 패널 2 의 5번 키(켬·끔. 12.2절) | `fixhispeed != OFF` 일 때의 `LaneRenderer.resetHispeed`, IIDX 의 FHS | 별도 스위치로 두었다. 기존 설정 파일은 끔으로 읽혀 이전과 같은 속도로 스크롤된다. HI-SPEED FIX 가 OFF 이면 기준 BPM 은 MAIN, CONSTANT 이면 120 이다 |
| GREEN NUMBER | KEEP GREEN NUMBER 가 유지하는 목표 녹색 숫자 | 1~6000(이동 시간 1~10000 ms 로 저장), PLAY 메뉴의 행은 1씩(Shift 10) | 300(이동 시간 500 ms) | PLAY 메뉴 > SPEED, 플레이 중 하이스피드 키(±10, 미세 단계 ±1), SELECT + 건반 또는 GREEN NUMBER UP/DOWN 키(이동 시간 ±1 ms), 선곡 패널 3 의 4·6번 키(이동 시간 ±1 ms, 누르고 있으면 반복) | `PlayConfig.duration` | 원본의 하이스피드 키는 유지 모드에서 배율만 바꾸고 저장된 이동 시간은 그대로 둔다(다음 커버 이동이나 다음 곡에서 되돌아간다). 여기서는 키를 누른 것이 새 목표를 고른 것이므로 목표가 바뀌어 남는다 |
| LANE COVER | 위에서 내려오는 가림막. 값이 커질수록 더 가린다 | 비율 0~1, 지정 키는 한 번에 0.01, 미세 단계 0.0001~0.01(기본 0.001). START + 턴테이블은 흰색 숫자 1(0.001) | 0, 켬 | PLAY 메뉴 > LANE, 플레이 중 `RIGHT`(더 가림)/`LEFT`(덜 가림), `BACKSLASH`(켬·끔), START + 턴테이블(흰색 숫자 1씩, 5.2절), START 두 번(켬·끔), 마우스 휠, 선곡 패널 2 의 6번 키(켬·끔. 12.2절) | `PlayConfig.lanecover`, `enablelanecover`, IIDX 의 SUDDEN+ | 켬·끔 전용 키와 방향을 적은 행 이름을 더했다. START + 턴테이블의 걸음과 반복, 녹색 숫자 유지는 5.2절 |
| LIFT | 판정선을 올린다 | 비율 0~1. START + SELECT + 턴테이블은 1(0.001) | 0, 끔 | PLAY 메뉴 > LANE, 플레이 중 `RBRACKET`/`LBRACKET`, START + SELECT + 턴테이블(1씩, 5.2절), 켬·끔 키(출하 시 미지정), 선곡 패널 2 의 7번 키(켬·끔. 12.2절) | `PlayConfig.lift`, IIDX 의 LIFT | 원본에는 리프트만 움직이는 조합이 없다(레인 커버를 꺼야 START + 턴테이블이 리프트로 간다). START + SELECT + 턴테이블을 더했다 |
| HIDDEN+ | 판정선에서 위로 올라오는 가림막 | 비율 0~1 | 0, 끔 | PLAY 메뉴 > LANE, 플레이 중 올림·내림·켬·끔 키(출하 시 미지정), 선곡 패널 2 의 턴테이블(켬·끔. 12.2절) | `PlayConfig.hidden`, IIDX 의 HIDDEN+ | 없음 |
| JUDGE OFFSET | 판정 오프셋 | -200~+200 ms, PLAY 메뉴의 행은 1 ms 씩(Shift 10 ms) | 0 | PLAY 메뉴 > JUDGE, 선곡 패널 3 의 5·7번 키(±1 ms), 플레이 중 `9`(-1 ms)/`0`(+1 ms. 누르고 있으면 반복, 5.3절) | `PlayerConfig.judgetiming`(`LaneRenderer.java:300-301`) | 뜻과 부호는 같다. 범위만 좁다(원본 -500~+500 ms). 원본은 플레이 중에 바꾸는 키가 없다 |
| AUTO ADJUST | 칠 때마다 JUDGE OFFSET 을 조금씩 움직인다 | 켬 / 끔 | 끔 | PLAY 메뉴 > JUDGE, 선곡 패널 3 의 3번 키, 플레이 중 AUTO ADJUST ON/OFF 키(출하 시 미지정) | `PlayerConfig.notesDisplayTimingAutoAdjust`(`JudgeManager.java:697-703`) | 없음(식과 부호가 같다) |
| SCRATCH AUTO COLOR | SCRATCH AUTO 가 대신 치는 턴테이블 노트를 칠하는 색. 알파가 불투명도다 | RGBA(`#RRGGBBAA`), 채널 R·G·B·A 각 0~255 | 저장값 없음(출하 `#30E050FF`, 불투명 초록) | PLAY 메뉴 > CHART(7.2절) | 없음(R-BMS 확장. 원본은 상수 색) | 어느 스킨에서나(외부 스킨 포함)·스킨 없는 내장 필드에서도 같은 값이 이긴다. 스킨의 색 행(Scratch Note·Scratch LN)은 이 노트에 닿지 않는다. 롱노트 몸통은 사용자 알파 x 0xA0/255 라 머리와 구분된다. 저장값이 없으면 화소가 전과 같다 |
| PLAYBACK RATE | 차트를 재생하는 속도. 소리의 음높이도 함께 바뀐다. 100% 가 아닌 판은 저장·리플레이·전송되지 않는다 | 50~200%(차트 자신의 속도에 대한 비율), 한 칸 1%, PLAY 메뉴의 Shift 는 10% | 100 | PLAY 메뉴 > CHART(LEGACY NOTE 바로 아래), F1 오버레이의 마지막 행. 플레이 중 키는 없다(18절) | 없음(원본의 일반 플레이에는 배속이 없고 연습의 FREQUENCY 만 있다. 연습 FREQUENCY 의 범위와 같다) | 차트를 재배치하지 않고 시계를 늘린다. 판정 창은 실시간 길이이고 BPM·녹색 숫자는 실시간 기준이다. 코스와 리플레이 감상은 설정과 무관하게 100%, 연습은 자기 FREQUENCY. 사용자 결정(2026-10-29)으로 100% 가 아닌 판은 기록을 남기지 않는다(10.2절, 18절) |

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

### 3.4 입력의 시각과 프레임(S19, [ADR 판정](acknowledge/2026-10-15-adr-event-time-judging.md), [ADR 키보드 시각](acknowledge/2026-10-15-adr-keyboard-input-timing.md))

판정은 화면의 프레임과 무관하다. 같은 입력은 60 Hz·144 Hz·프레임 드롭 어디서 쳐도, 그 판의 리플레이를 어느 주사율로 재생해도 같은 판정·게이지·램프가 나온다.

| 무엇 | 언제 |
| --- | --- |
| 키보드 입력 | 이벤트 루프가 그 키를 받은 순간의 곡 시각(`AppShared::song_us_at`). 화면 대기(vsync)는 present 전용 스레드(`gpu/present.rs`)가 하고, 이벤트 루프는 프레임과 프레임 사이에 창 말고는 아무것도 기다리지 않으므로 키는 OS 가 창에 넘긴 즉시 받힌다. 남는 지연은 이벤트 루프가 프레임 하나를 갱신하고 그리는 CPU 시간뿐이다(아래 4번). 예전 빌드(present 대기가 끝난 뒤 받은 순간)보다 평균 약 반 프레임, 최대 한 프레임 이르게 판정되므로 예전 빌드에서 맞춘 JUDGE OFFSET 은 다시 맞춘다 |
| 키보드 디바운스(KEY DEBOUNCE, SETTINGS 의 INPUT 탭, 기본 16 ms, 0~100) | LR2 의 `inputinterval` 과 같은 방식이다(`En_input.cpp:276-292`). 레인 키마다, 누름은 들어온 순간 그대로 판정되고 디바운스가 늦추는 일이 없다. 누른 뒤 그 시간 안에 들어온 뗌만 창 끝까지 미뤄, 창 끝에 그때도 떼어져 있으면 창 끝의 시각으로 넘기고(다음 키·컨트롤러 이벤트·프레임이 먼저 확인한다), 그 전에 다시 눌렸으면 버린다. 넘겨진 뗌 뒤 2 ms(LR2 의 폴링 간격, 창이 더 짧으면 창) 안의 누름은 뗄 때 접점이 튄 것으로 보고 넘기지 않는다. 그래서 누를 때 튀는 채터는 한 번의 누름이 되고, 같은 레인 연타의 누름은 뗀 직후라도 노트 위 그대로 판정된다. 늦어지는 것은 누른 지 그 시간 안에 뗀 짧은 탭의 뗌뿐이다. 원본 키보드(`KeyBoardInputProcesseor.java:99-146`)·컨트롤러(`BMControllerInputProcessor.java:136`) 디바운스의 양방향 잠금과 다르다(그쪽은 뗀 직후 창 안의 재누름을 창 끝까지 늦춘다). 판의 레인 입력에만 걸고 메뉴·키 설정·글자 입력에는 걸지 않는다. 0 이면 모든 변화를 넘긴다 |
| 컨트롤러 입력 | gilrs 가 그 변화를 받은 순간(`Event.time`). 예전 빌드(프레임마다 폴링한 순간)보다 60 Hz 에서 평균 약 8 ms, 최대 한 프레임 이르게 판정되므로 예전 빌드에서 맞춘 JUDGE OFFSET 은 다시 맞춘다. 한 프레임 안에 눌렀다 뗀 짧은 탭도 누름과 뗌이 각자 판정된다. 디바운스(PAD DEBOUNCE, SETTINGS 의 INPUT 탭, 기본 16 ms, 0~100, 키 설정 파일 `keyconfig.ron` 의 `pad.debounce_ms` 에 저장)는 버튼과 두 키 축 방향에 키보드와 같은 LR2 방식으로 건다(`input_debounce.rs` 의 한 구현을 같이 쓴다): 누름은 늘 즉시, 누른 뒤 창 안의 뗌만 창 끝 시각으로 미루고, 넘겨진 뗌 뒤 2 ms 안의 누름은 튐으로 버린다. 시간은 두 변화의 시각 사이로 잰다. 아날로그 턴테이블만 프레임마다 한 번 읽고, 그 방향에는 원본의 양방향 잠금(받아들인 변화 뒤 창 동안 읽지 않음)을 그대로 둔다. 패드 입력은 한 곳에서 읽으므로 이 디바운스와 턴테이블 잠금은 플레이뿐 아니라 선곡·설정·결과 화면의 메뉴 입력에도 같이 걸린다(뗌 뒤 2 ms 넘게 튀는 버튼은 메뉴에서 두 번 눌린 것이 될 수 있다, 테스트 부채 3.71) |
| 놓친 노트(見逃し POOR), 지연 릴리스 확정, 일반 롱노트 끝, 지뢰 | 각자 기한이 된 순간. 지뢰는 그 순간 키가 눌려 있었는지로 깎는다 |
| HCN 게이지 틱 | 쥔(또는 놓은) 시간 200 ms 마다 한 번. 긴 프레임에서도 밀리지 않는다 |
| GAUGE AUTO SHIFT, FAILED | 게이지가 바뀐 모든 순간 뒤. NONE 에서 게이지가 비면 그 순간부터 판정하지 않는다 |
| 결과 화면의 게이지 이력 | 0.5초 경계마다 그 순간의 값 |

1. 같은 순간이면 시간으로 진행되는 판정이 먼저이고 입력이 나중이다. 입력이 그 순간을 기한으로 만든 것(마진 0 인 지연 릴리스)은 그 입력 바로 뒤에 확정한다.
2. 7키·5키·24키는 일반 롱노트 마진이 0 이다. 판정 창 밖에서 일찍 뗀 롱노트는 바로 BAD 로 확정되고, 몇 µs 뒤에 다시 눌러도 되살아나지 않는다. 원본은 한 프레임 안의 떼기를 보지 못해 되살아났다. 누르는 순간의 채터(누름 뒤 KEY DEBOUNCE 안의 뗌과 다시 누름)는 디바운스가 막는다. 누른 지 그 시간이 지난 뒤 접점이 잠깐 떨어지는 채터(뗌이 먼저 오는 채터)는 LR2·원본 디바운스와 마찬가지로 KEY DEBOUNCE·PAD DEBOUNCE 모두 막지 못해 롱노트가 끊긴다(2 ms 안에 다시 붙는 누름은 새 누름으로 받지 않는다. ADR 키보드 시각의 "남은 한계"). pop'n 은 마진 200 ms 안의 다시 누르기가 되살린다.
3. 리플레이: 새 리플레이는 `judge.clock: "EVENT"` 다. 이 값이 없는 옛 리플레이는 `FRAME` 으로 읽히고 그대로 재생되지만, 녹화한 기계의 프레임이 정했던 곳(한 프레임 안의 떼기·다시 누르기, 지뢰 앞뒤의 키, HCN 틱, 게이지가 빈 프레임 뒤의 입력)은 저장된 점수와 다르게 나올 수 있다. 웨이브 12 리플레이 셋(3.3절 2번)은 그대로 재생된다.
4. 남는 지연: winit 의 키 이벤트에는 하드웨어 시각이 없어 키 시각은 이벤트 루프가 받은 순간이다. 이벤트 루프가 프레임 하나를 갱신하고 그리는 동안(실제 플레이어에서는 재지 않았다. 스킨이 무거울수록 길다) 들어온 키는 그 일이 끝난 순간으로 판정된다. LR2 가 그리는 동안 폴링하지 않는 것과 같은 한계다. 같은 구조의 측정 하네스(M5 Max, 120 Hz, Metal)에서 이벤트가 들어와 받히기까지 p50 4.1 ms → 0.02 ms, p95 8.0 ms → 0.04 ms 로 줄었다(프레임마다 CPU 일 4 ms 를 넣으면 p95 3.7 ms, 곧 CPU 일만큼 남는다). OS 이벤트 자체의 시각(macOS `NSEvent.timestamp`, Windows `GetMessageTime`)은 FFI(unsafe)와 새 의존성이 있어야 받을 수 있어 쓰지 않았다. 키보드는 이벤트가 오는 즉시, 컨트롤러는 프레임 첫머리에서 처리되므로, 두 장치를 섞어 치면 컨트롤러의 이른 입력이 그보다 늦은 키 시각으로 끌어올려질 수 있다(곡 시계 단조 클램프, 테스트 부채 3.55. 어긋남의 크기는 예전보다 작다). gilrs 의 Windows 백엔드는 자체 스레드가 8~10 ms 마다 폴링해 그만큼 양자화되고, 그 사이의 누름·뗌은 gilrs 안에서 합쳐질 수 있다.
5. 테스트: `crates/rbms-play/tests/replay_reproduction.rs`(`a_run_judges_the_same_at_every_frame_rate_and_so_does_its_replay` 등), `apps/rbms-player/src/stage/play/input_time_tests.rs`(키 채터·디바운스 포함), `apps/rbms-player/src/stage/play/key_gate.rs`, `apps/rbms-player/src/gpu/present.rs`(화면이 이미지를 붙잡고 있어도 다음 이미지 요청이 바로 돌아온다), `apps/rbms-player/src/gamepad/tests.rs`(같은 버튼 32분 연타의 누름은 그 자리, 누른 채 튀는 접점은 뗌만, 누를 때 튀는 접점은 한 번의 누름), `apps/rbms-player/src/input_debounce.rs`, `input_time_tests::a_controller_button_hit_in_quick_succession_is_played_where_every_press_came`.

## 4. 녹색 숫자 유지(KEEP GREEN NUMBER)

1. 켜면 저장되는 값은 하이스피드가 아니라 이동 시간(`play.duration_ms`)이다. 하이스피드는 곡마다 `240000 / 기준 BPM / 이동 시간 * (1 - 레인 커버)` 로 정해진다. 레인 커버가 꺼져 있으면 커버는 0 으로 본다.
2. 기준 BPM 은 HI-SPEED FIX 가 가리키는 BPM(START / MAX / MAIN / MIN)이고, OFF 이면 MAIN(노트가 가장 많은 BPM)이다. CONSTANT 스크롤에서는 필드의 고정 템포 120 이다.
3. 플레이 중 레인 커버를 움직이거나 켜고 끄면 녹색 숫자는 그대로이고 하이스피드가 바뀐다.
4. 리프트와 HIDDEN+ 는 계산에 들어가지 않는다. 스크롤이 리프트로 줄어든 레인 높이에 맞춰지므로 리프트를 바꿔도 이동 시간이 그대로이고, HIDDEN+ 는 판정선 쪽을 가릴 뿐 노트가 나타나는 시각을 바꾸지 않는다(원본의 `currentduration` 도 커버만 반영한다). 실측: 커버 0.3·리프트 0.15 에서 150·200·100 BPM 모두 499.0 ms.
5. 하이스피드가 범위(0.01~20.00)를 넘어야 하는 목표이면 배율은 범위 끝에 머물고, 화면의 녹색 숫자는 실제 값을 보여 준다.
6. 꺼져 있으면 저장된 HI-SPEED 로 스크롤한다(이전과 같다). 이때 선곡 패널 3 의 NOTE DISPLAY TIME 칸은 `OFF` 로 표시되고 4·6번 키는 아무것도 바꾸지 않는다. 켜져 있어도 커서가 차트가 아닌 막대(폴더·표·실행 파일·못 치는 코스)에 있으면 칸이 `-` 로 표시되고 4·6번 키는 이동 시간을 바꾸지 않는다(12.3절).
7. 속도를 잠그는 코스에서는 유지하지 않는다. 리플레이는 스크롤 설정을 기록하지 않으므로 보는 사람의 설정으로 스크롤한다.

## 5. 키

### 5.1 키 설정

| 항목 | 내용 |
| --- | --- |
| 키 설정 화면을 여는 길 | 선곡에서 `F5`, Settings 의 `KEY CONFIG >` 단추 또는 `F5`, 스킨 브라우저의 숫자 `6` |
| 사용자 지정 범위 | 레인과 스크래치 역방향, 플레이 조작 20종, 선곡 단축키 11종(난이도표 다시 받기 `F6`, 배치 지정 `Y` 포함), 화면 여는 키 6종(PLAY 메뉴 `F8` 포함), 스킨 브라우저 숫자 키 8종, 결과 2종, 리플레이 분석 6종(배치로 치기 `J` 포함). 키보드와 컨트롤러 둘 다(옵션 오버레이 두 키와 상세 패널 홀드 키는 키보드 전용) |
| 고정 키 | `Esc`, `Enter`, 화살표, `H`(키 안내)와 각 메뉴 안의 키 |
| 레인 커버 조작 | `RIGHT` = 더 가림(LANE COVER DOWN), `LEFT` = 덜 가림(LANE COVER UP), `BACKSLASH` = 켬·끔. 커버가 꺼져 있을 때 값을 바꾸면 발치의 COVER 수치가 1.5초 동안 뜬다 |
| 미세 단계 | FINE STEP 키를 누르고 있는 동안 지정 키의 가림막은 미세 단계로, 유지 중인 녹색 숫자는 ±1 로 움직인다. 키를 주지 않았으면 레인으로 쓰지 않는 Shift 가 그 역할을 한다(7키 기본 배치에서는 오른쪽 Shift). START + 턴테이블의 걸음에는 관여하지 않는다(5.2절) |
| 판정 오프셋 | `9` = -1 ms, `0` = +1 ms(5.3절) |
| 스크래치 역방향 | 스크래치 레인마다 반대 방향 키가 하나 더 있다. 출하 키는 왼쪽 턴테이블 `LCTRL`, 10·14키의 오른쪽 턴테이블 `RCTRL`(5.4절). 선곡에서는 PREVIOUS BAR, 플레이에서는 스크래치의 반대 방향이고 START 와 함께 누르면 가림막을 올린다(5.2절) |
| 지금 설정된 키 보기 | 선곡에서 `H`. 플레이 조작은 IN PLAY, START·SELECT 조합은 IN PLAY, HELD TOGETHER 묶음에 지금 지정된 키로 나온다. 그 묶음의 첫 줄(`LSHIFT LCTRL  THE TURNTABLE: DOWN / UP`)이 아래 줄들의 TURNTABLE 이 키보드의 어느 두 키이고 각각 가림막을 어느 쪽으로 움직이는지 알려 준다 |
| 난이도표 다시 받기 | 선곡에서 `F6`(RELOAD DIFFICULTY TABLES, 설정 파일 키 `reload_tables`, 키 설정 화면에서 바꿀 수 있고 패드에도 줄 수 있다). 등록한 표를 전부 다시 받는다. 검색어를 치는 중에는 듣지 않는다. 표 하나만 받으려면 Tables 화면(`T`)에서 `R`, 전부는 `A`. 저장과 다시 받기의 규칙은 `README.md` 의 Difficulty tables 절 |
| 배치 지정 | 선곡에서 `Y`(LAYOUT PICK, 설정 파일 키 `layout_pick`, 키 설정 화면의 Browser 그룹에서 바꿀 수 있고 패드에도 줄 수 있다). 커서가 차트 막대일 때만 열린다. 검색어를 치는 중에 누르면 검색을 끝내고 연다. `Y` 는 출하 레인 키·플레이 조작 키·다른 선곡 단축키 어디에도 쓰이지 않는다. 이미 `Y` 를 다른 선곡 단축키에 둔 키 설정 파일에서는 LAYOUT PICK 이 키 없이 남는다. 화면은 10.16절 |
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
| 색 행과 색 채널 행(7.2절) | `apps/rbms-player/src/skin_select.rs`(`SkinRow::Color`·`ColorChannel`·`ColorChannels`, `color_for`·`set_color_for`·`reset_color_for`, 프리셋 `COLOR_PRESETS`), PLAY 메뉴의 접이 `stage/play_menu/rows.rs`(`Fold::Color`·`Fold::ColorChannels`), 16진 입력 `apps/rbms-player/src/number_row.rs`(`begin_hex_edit`, `typed_color`), 견본 사각형 `stage/preview_menu.rs`(`ColorCell`), 저장 `crates/rbms-config/src/schema.rs`(`SkinCustomisation.colors`) |
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
| CHART | PLAYBACK RATE | `100%`. 목록에서 RANDOM, GAUGE, LEGACY NOTE 다음이다 | 1% / 10%. 범위 50~200%. 0 키로 100%. 정보 패널의 설명은 "차트를 재생하는 속도로, 음높이도 함께 바뀝니다. 100%가 아닌 플레이는 저장·리플레이·전송되지 않습니다."(`RowDescriptionText::PlaybackRate`, 세 언어). 뜻은 18절 |
| CHART | SCRATCH AUTO COLOR | 색 견본 사각형과 값. 저장한 색이 있으면 `#RRGGBBAA`, 없으면 `DEFAULT`(출하 `#30E050FF`). SCRATCH AUTO 가 꺼져 있어도 늘 보인다 | 색 행(SKIN 묶음)과 같다: ← → 로 프리셋 16색(알파 유지), Enter 로 16진 `RRGGBB`·`RRGGBBAA`, 0 키로 저장값 지움 |
| CHART | 접이 `SCRATCH AUTO CHANNELS` | `HIDDEN` / `SHOWN`: 접힘 여부(든 행 4) | 접이 `COLOR CHANNELS` 와 같이 편다. 펴면 R, G, B, A 네 행(각 0~255, 한 칸 1, Shift 10)이 나오고 0 키는 그 채널만 출하 초록의 값으로 되돌린다. 닫힌 채로 열린다 |
| SKIN | 접이 `COLOR` | `HIDDEN (17)` / `SHOWN (17)`: 접힘 여부와 든 행 수(기본 스킨 7키) | Enter·← →·클릭으로 펴고 접는다. 그 문서가 `color` 로 선언한 색 행이 한 행씩 바로 아래에 나온다(기본 스킨 5·7·10·14키 17행, 9키 22행. 목록은 3.4.4절). 선언한 색이 없는 문서에는 접이가 없다. 닫힌 채로 열린다 |
| SKIN | 색 행(접이 `COLOR` 를 폈을 때. 기본 스킨: `LANE > WHITE KEY`, `NOTE > BLACK KEY`, `EFFECT > BOMB` 등) | 그 색의 견본 사각형(16px)과 값. 저장한 색이 있으면 `#RRGGBBAA`, 없으면 `DEFAULT` | ← → 로 프리셋 16색을 돈다(알파는 그대로. 프리셋 밖의 색에서는 → 가 첫 색, ← 가 끝 색). Enter 로 16진 `RRGGBB`·`RRGGBBAA` 를 쳐서 넣는다(`#` 는 받는다. 색이 아니면 행에 1.5초 동안 `NOT A COLOR`). 0 키는 `DEFAULT`(스킨이 정한 색으로 돌려놓고 저장값을 지운다). 저장값이 없다가 생기거나 있다가 없어지면 문서를 한 번 다시 읽고, 이미 저장된 색을 바꾸는 것은 다시 읽지 않고 다음 프레임에 반영한다. 저장값이 없는 행의 시작은 스킨이 지금 그 부위를 그리는 색이다(3.4.4절) |
| SKIN | 접이 `COLOR CHANNELS` | `HIDDEN (68)` / `SHOWN (68)`(기본 스킨 7키: 17색 x 4) | 접이 `COLOR` 와 같이 펴고 접는다. 펴면 색 행마다 `<행 이름> R`, `G`, `B`, `A` 네 행이 나온다. 각 행은 0~255 숫자 행(한 칸 1, Shift 10, 슬라이더·직접 입력은 7.3절 그대로)이고 0 키는 그 색이 출하된 값(헤더의 `def`, 팩이 조절값 파일로 색을 주었으면 그 색)의 그 채널로 되돌린다. 알파는 그 부위의 불투명도에 곱해진다. 접이 `COLOR` 와 같이 닫힌 채로 열린다 |
| SKIN | 접이 `OPACITY` | `HIDDEN (18)` / `SHOWN (18)`: 접힘 여부와 든 행 수 | Enter·← →·클릭으로 펴고 접는다. 펴면 그 문서의 불투명도 행이 바로 아래에 나온다(기본 스킨 7키 18행, 14키 17행. 웨이브 13 에서 Key Display Opacity 행이 없어져 하나씩 줄었다) |
| SKIN | 불투명도 행(접이를 폈을 때. 기본 스킨: `LANE BACKGROUND OPACITY` 등) | `100%` | 1% / 10%. 슬라이더와 ← → 는 0~100%. 0% 면 그 부위가 사라진다. 0 키로 100%. 직접 입력만 200% 까지 받는다(알파 0 의 덮개를 밝히는 스킨용. `docs/skin.md` 2.2절의 불투명도 행 주의) |
| SKIN | 접이 `POSITION AND SIZE` | `HIDDEN (15)` / `SHOWN (15)` | 엔진이 모든 플레이 스킨에 붙이는 오프셋 네 개의 행(`ALL OFFSET(%) X` … `JUDGE DETAIL OFFSET ALPHA`). 1 / 10 |

- KEEP GREEN NUMBER 가 켜져 있으면 GREEN NUMBER 가 조절 대상이고 HI-SPEED 는 예시 BPM 에서 계산된 값을 보이는 읽기 전용 행이다(범위 끝에 걸리면 정보 패널에 `20.00x (LIMIT) AUTO`). 꺼져 있으면 반대로 HI-SPEED 가 조절 대상이고 GREEN NUMBER 는 그 속도가 예시 BPM 에서 내는 녹색 숫자와 이동 시간을 보인다. 읽기 전용 행은 이름·값·슬라이더가 모두 흐리고(슬라이더는 계산된 값의 자리에 선다), 좌우·Enter·0·클릭·끌기·직접 입력에 움직이지 않으며, 정보 패널이 값 뒤에 `AUTO` 를 붙이고 이유를 말한다. 유지 중에는 저장된 HI-SPEED 가 쓰이지 않으므로 보이지 않는다(끄면 다시 보인다).
- LANE COVER·LIFT·HIDDEN+ 는 0.1% 격자 위에서 움직인다(`rbms_config::set_number_steps` 가 값을 걸음 수로 세어 `걸음 / 1000` 을 저장한다. 더해 가지 않으므로 천 번 옮겨도 오차가 쌓이지 않는다). 30.0% 는 흰색 숫자 300 이고 한 칸이 흰색 숫자 1 이다. 저장값은 스킨이 천분율로 잘라 읽어도 한 칸 모자라게 읽히지 않는 쪽으로 고른다(25.1% 는 250 이 아니라 251). 플레이 중 미세 단계로 격자 밖에 둔 값은 가장 가까운 칸으로 읽히고, 한 칸 움직이면 거기서 한 칸 간 값이 된다. 선곡 옵션 패널과 스킨 이벤트의 한 칸은 전과 같은 1% 다(`rbms_config::adjust`).
- SKIN 묶음은 목록의 마지막 묶음이고 접이 넷(`COLOR`, `COLOR CHANNELS`, `OPACITY`, `POSITION AND SIZE` 순)은 닫힌 채로 열린다(접이 상태는 저장되지 않는다). 접이를 펴면 든 행이 화면에 들어오도록 목록이 스크롤된다: 다 들어가면 전부, 넘치면 접이 행이 목록 첫 줄에 온다. 스킨이 색·불투명도·엔진 오프셋을 갖지 않으면 그 접이는 없고, 모두 없으면 묶음도 없다. 불투명도와 오프셋은 SKIN 메뉴의 같은 행과 같은 곳(그 문서의 `skin.custom` 오프셋)에, 색은 같은 곳의 `colors`(색 id 를 키로 한 RGBA)에 저장된다. 색 행은 SKIN 메뉴에서는 접이 없이 옵션·파일 행 바로 뒤에 평평하게 나오고 채널 행만 `COLOR CHANNELS` 접이 한 행 뒤에 든다(`docs/skin.md` 2.2절).
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
| 색 행의 ← → · Enter · 0 · 클릭 | ← → 는 프리셋 16색 한 칸(알파 유지), Enter 는 16진 색을 직접 치는 입력(`RRGGBB` 또는 `RRGGBBAA`, `#` 허용. 16진 숫자와 `#` 만 받는다. 바닥 안내는 `TYPE A HEX COLOR: RRGGBB OR RRGGBBAA   ENTER SET   ESC CANCEL   UP DOWN TAB SET AND MOVE ON`. 색이 아니면 `NOT A COLOR` 1.5초), 0 은 `DEFAULT`(저장한 색을 지움). 왼쪽 클릭은 프리셋 한 칸 앞, 오른쪽 클릭은 한 칸 뒤이고 값 영역을 눌러도 16진 입력은 열리지 않는다. 채널 행(`COLOR CHANNELS` 접이)은 위의 숫자 행 조작 그대로다 |
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
| 색 행과 색 채널 행(SKIN 묶음의 `COLOR`, `COLOR CHANNELS` 접이) | 이미 저장한 색을 바꾸면 문서를 다시 읽지 않고 다음 프레임부터 그 색으로 그린다. 저장한 색이 없던 행에 처음 색을 주거나 0 키로 지울 때만 문서를 한 번 다시 읽는다(Lua 스킨이 로드 때 그 부위를 칠할지 정하기 때문). 색의 알파는 그 부위의 불투명도 오프셋을 더한 뒤의 알파에 곱해진다 |
| JUDGE OFFSET, AUTO ADJUST | 노트가 오프셋만큼 앞서(음수면 늦게) 그려진다. 예시 손이 치는 시각은 그대로라 +면 노트가 판정선에서 사라진 뒤에 폭발과 판정 글자가 뜨고, -면 폭발이 뜬 뒤에도 노트가 판정선까지 내려온다. 스킨의 TIMING 숫자와 `TIMING AUTO` 표지도 따른다. 예시 판정은 달라지지 않는다 |
| GAUGE | 게이지의 종류와 색 |
| TARGET, TARGET FALLBACK, TARGET RATE | 점수 그래프의 타깃 막대와 `TARGET +n`, 그 막대의 이름표. 기록·IR·라이벌 없이 예시 차트의 노트 수만으로 정한 타깃이다. 그래서 LOCAL BEST·IR BEST·RIVAL 은 TARGET FALLBACK 으로 물러난 값이고, 이름표도 물러난 목표의 이름이다(웨이브 16 전에는 MAX 로 물러났다) |
| AUTOPLAY | 스킨의 AUTOPLAY 표지. 예시 손(적어 둔 입력)은 그대로다 |
| LEGACY NOTE | 롱노트가 일반 노트로 그려진다 |
| RANDOM, HI-SPEED FIX, HI-SPEED STEP, COVER FINE STEP, SCRATCH AUTO, SCRATCH AUTO COLOR, AUTO REPLAY, ESCAPE, PLAYBACK RATE | 보이지 않는다(SCRATCH AUTO COLOR 는 행의 색 견본 사각형으로 확인한다. PLAYBACK RATE 는 스킨 미리보기에서 확장 불린 21106 이 꺼져 있어 표지가 없다). 예시 차트의 배치와 손은 바꾸지 않고(RANDOM), 예시 차트가 한 템포라 기준 BPM 이 어느 것이든 같으며(HI-SPEED FIX), 나머지는 화면에 그릴 것이 없는 값이다 |

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
| SCRATCH AUTO COLOR, SCRATCH AUTO CHANNELS(접이) | 없음(2026-10-23 새 행) | PLAY 메뉴 > CHART(SCRATCH AUTO 바로 아래) |
| PLAYBACK RATE | 없음(2026-10-29 새 행) | PLAY 메뉴 > CHART(LEGACY NOTE 바로 아래)와 F1 오버레이의 마지막 행. SETTINGS 에는 없다(18절) |
| COURSE GAUGE | 없음(2026-10-23 새 행) | SETTINGS > JUDGE(GAUGE 구역의 GAUGE SET 바로 뒤. 10.14절) |
| MISSING CHARTS, DOWNLOAD FOLDER | 없음(2026-10-23 새 행) | SETTINGS > SELECT(MISSING CHARTS 는 SONG LIST 구역, DOWNLOAD FOLDER 는 DOWNLOADS 구역. 13절) |
| 5KEYS LAYOUT | SETTINGS > PLAY | SETTINGS > DISPLAY(내장 필드만 읽는 행. WHITE NUMBER, JUDGE TEXT Y 아래) |
| TOTAL | SETTINGS > GAUGE | SETTINGS > JUDGE(게이지 규칙 행들 아래) |
| JUDGE ALGORITHM, JUDGE WIDTH 6행, LN MARGIN, LN MODE, GAUGE SET, GAUGE AUTO SHIFT, BOTTOM SHIFTABLE | SETTINGS > JUDGE | 그대로. GAUGE SET(AUTO·LR2)은 단일 곡 전용이 되었고 코스는 COURSE GAUGE 가 표를 정한다 |
| SCRATCH SIDE, KEY CONFIG | SETTINGS > INPUT | 그대로. SETTINGS 의 SCRATCH SIDE 는 내장 필드의 값이고(스킨이 그리는 동안 행 옆에 `SKIN OPTION COMES FIRST` 표지가 붙는다), 스킨이 그리는 화면의 스크래치 쪽은 PLAY 메뉴 LANE 묶음의 스킨 옵션 SCRATCH SIDE 다. 이름이 같은 두 행은 다른 값이다 |
| DISPLAY 의 나머지, NETWORK, AUDIO, SELECT | — | 그대로 |

SETTINGS 의 탭은 JUDGE, DISPLAY, INPUT, NETWORK, AUDIO, SELECT 여섯이다(PLAY 탭과 GAUGE 탭은 없어졌다). 플레이 직전의 빠른 옵션(F1 오버레이. 2026-10-29 부터 열두 행이고 마지막이 PLAYBACK RATE)과 선곡의 옵션 패널 1~3(12절. 2026-10-22 에 키 배치가 바뀌었다)은 SETTINGS 와 같은 값을 고친다. `rbms-cli config` 덤프는 PLAY 메뉴의 묶음을 `[PLAY SPEED]`, `[PLAY LANE]`, `[PLAY JUDGE]`, `[PLAY CHART]` 로 적는다. PAD DEBOUNCE 는 값이 settings.ron 이 아니라 `keyconfig.ron` 에 있으므로 숫자 대신 `(in keyconfig.ron)` 으로 적는다.

## 8. 판정 표시(기본 스킨)

기본 스킨의 플레이 화면이 판정을 보이는 방식이다. 판정·점수 계산은 달라지지 않고 표시만의 설정이다. 조절하는 행은 PLAY 메뉴의 LANE 묶음과 SKIN 메뉴에 같은 값으로 있다.

| 항목 | 동작 | 행 |
| --- | --- | --- |
| 정적 표시 | 판정 글자와 콤보는 판정이 난 프레임부터 0.5초 동안 같은 자리·크기·색으로 서 있다가 한 번에 사라진다. 튀어나오거나 커지거나 깜빡이거나 서서히 사라지지 않는다. 콤보가 올라가도 글자의 자리는 그대로이고 숫자만 바뀐다. 콤보는 글자 바로 뒤에 붙고(`GREAT 67`) 자릿수가 늘면 오른쪽으로만 자란다. 앞자리 0 은 없다 | 없음(항상) |
| 위 줄 왼쪽: 점수 차 | 목표 대비 EX 점수 차를 부호와 네 자리로 쓴다(`+0003`, `-0125`). 스코어 그래프의 `TARGET` 숫자와 같은 값이고 CHART 묶음의 TARGET 이 목표를 정한다(목표의 점수가 없으면 TARGET FALLBACK. 11절). 견주는 쪽은 판이 지금까지 판정한 노트 수만큼에서 서 있는 값이다: TARGET 은 LOCAL BEST 와 이 기기에서 친 IR BEST 면 그 기록 판이 같은 수의 노트를 판정했을 때의 EX, 그 밖의 목표는 최종 EX 를 판정된 노트 수에 비례시킨 값이고, BEST 는 로컬 최고 판의 같은 시점 EX(그 판의 노트별 판정을 얻지 못하면 최고 EX 의 비례값. 11.7). PGREAT 에도 뜬다 | `SCORE DIFFERENCE`: TARGET(기본) / BEST(자기 최고 기록 대비) / OFF |
| 위 줄 오른쪽: FAST/SLOW | PGREAT 가 아닌 판정이 빨랐으면 FAST(파랑), 늦었으면 SLOW(빨강) | `JUDGE DETAIL`: FAST-SLOW(기본) / MILLISECONDS(어긋난 시간을 ms 로. 빠르면 `+`) / OFF |
| 위치 | 판정 글자·콤보·위 줄을 함께 옮긴다. + 는 위, - 는 아래. 한 칸 1px, Shift 로 10px, 0 키로 제자리. 스킨 좌표(1920x1080 기준)다 | `JUDGE POSITION Y`: -2000 ~ +2000, 기본 +0 |
| 숨김 | 판정 글자·콤보·위 줄을 그리지 않는다 | `JUDGE DISPLAY`: SHOW(기본) / HIDE |

- DP 는 친 쪽 필드에만 뜬다. BAD·POOR 는 콤보 없이 글자만 뜨고, 글자의 왼쪽 끝은 다른 판정과 같은 자리다(판정이 바뀌어도 글자가 좌우로 옮겨 다니지 않는다). 위 줄도 같은 왼쪽 끝에서 시작한다.
- 다른 스킨 팩은 자기 판정 객체와 자기 옵션을 쓴다(ModernChic 의 판정 연출은 그대로다). 위 행들은 기본 스킨 문서가 선언한 것이라 다른 팩에서는 나오지 않는다.
- 레인 면·레인 구분선·마디선 같은 부위를 지워 노트와 판정만 남기는 법은 `docs/skin.md` 3.4.2절(부위별 불투명도)에 있다.
- 판정선은 필드의 맨 아래에 붙어 있다(웨이브 13). 판정선 아래에 레인이 이어지던 건반 표시 칸은 없어졌고, 판정 글자와 위 줄은 필드와 함께 64px 내려왔다(판정선과의 거리는 전과 같다). 판정선을 올리려면 LIFT 를 쓴다. 좌표는 기본 스킨 설계서 6.3·6.4절.

## 9. 연습 패널(PRACTICE)

선곡에서 연습 키(기본 `F4`)로 연다. 차트를 플레이와 같은 길로 읽은 뒤 뜨는 내장 화면이고, 구간 플레이는 이 화면 위에 열렸다가 이 화면으로 돌아온다. 스킨이 그리는 플레이 화면 위에 겹쳐 뜨는 경로는 없다(스킨의 연습 상태 옵션 1080 은 항상 꺼져 있다). 포인터 좌표는 다른 내장 화면과 같은 통로(창 좌표 → 1280x720 논리 좌표, 레터박스 포함)를 탄다.

**코스 막대 위에서는 연습 키가 연습 패널이 아니라 코스를 시작한다**(2026-10-24, 사용자 결정 21, 원본 `MusicSelector.java:265-270` 에서 코스 막대의 PRACTICE 는 PLAY 로 바뀌어 코스를 읽는 것과 같다). 코스 탭의 코스 막대와 표 안 `COURSE` 폴더의 코스 막대가 같다. 내장 브라우저의 `F4`, 스킨 키 표의 연습 키, 스킨의 `practice` 버튼이 모두 `start_practice` 한 곳을 거쳐 코스 막대면 `select_enter` 로 시작하므로 세 길이 같다. 부수 효과: 내장 브라우저의 코스 탭에서 `F4` 가 뒤에 숨은 곡 목록의 차트를 연습하던 동작이 코스 시작으로 바뀌었다(원본과 같아짐, 미확인 목록 23번). AUTOPLAY·REPLAY 키의 코스 동작은 그대로다. 시험은 이 분기를 끈 상태에서 실패하는 것을 확인했다.

웨이브 12 에서 숫자 행 여섯을 PLAY 메뉴와 같은 방식(7.3절)으로 바꿨다: 슬라이더, 직접 입력, 가장 잔 단위의 ← →. 원본 `PracticeConfiguration` 의 굵은 걸음(TOTAL 5, 재생 속도 5%, 터보 키의 2.5초·25)은 없어졌다. 값의 범위와 값 사이의 제약은 원본 그대로다.

### 9.1 행

| 행 | 종류 | 범위 | 가장 잔 단위(← →) | Shift + ← → | 전의 한 칸 / Shift(터보) | 0 키(기본값) |
| --- | --- | --- | --- | --- | --- | --- |
| START | 동작 | — | — | — | 없던 행 | — |
| START TIME | 숫자(시각) | 0:00.0 ~ 마지막 타임라인 2초 전(0.1초로 내림. **차트마다 다름**) | 0.1초 | 1초 | 0.1초 / 2.5초 | 0:00.0 |
| END TIME | 숫자(시각) | START TIME + 1초(0.1초로 내림. **START TIME 에 따라 다름**) ~ 마지막 타임라인 1초 뒤(**차트마다 다름**) | 0.1초 | 1초 | 0.1초 / 2.5초 | 차트 끝(상한) |
| GAUGE TYPE | 고르기(9종, 돌아감) | ASSIST EASY ~ EXHARD GRADE | 한 칸 | 한 칸 | 그대로 | 움직이지 않음 |
| GAUGE CATEGORY | 고르기(6종, 돌아감) | 5KEYS 7KEYS PMS KEYBOARD LR2 IIDX | 한 칸 | 한 칸 | 그대로 | 움직이지 않음 |
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
| 재생 속도와 판정 | FREQUENCY 는 소리·그리기를 그 속도로 돌리지만 판정은 플레이어의 실제 시간으로 한다. 판정 창(PGREAT~空POOR)·롱노트 마진·후보 범위·놓친 노트 경계·HCN 틱(200 ms)이 재생 속도와 상관없이 같은 실제 시간 폭이고, FAST/SLOW 의 ms 값·자동 오프셋 조정·타이밍 통계도 실제 시간이다(`PlaySession::set_playback_rate`). 원본이 차트를 재배치하고(`BMSModelUtils.changeFrequency`) 같은 창으로 판정하는 것과 같다. 전에는 50% 에서 실제 30 ms 늦은 입력이 PGREAT, 200% 에서 10 ms 늦은 입력이 GREAT 였다 |
| 표시와 시계(2026-10-29) | 구간의 시계는 일반 판의 PLAYBACK RATE(18절)와 같은 `PracticeClock` 이다. 그래서 FREQUENCY 판에서도 스킨과 내장 HUD 의 BPM(현재·MAIN·MIN·MAX), 이동 시간, 녹색·흰색 숫자, KEEP GREEN NUMBER 의 기준 템포가 실시간 기준(BPM x 배율)이 되었다. 같은 HI-SPEED 에서 200% 면 노트가 실제로 두 배 빨리 내려오고 KEEP GREEN NUMBER 가 켜져 있으면 하이스피드가 절반이 되어 녹색 숫자가 유지된다. 전에는 차트 BPM 기준이었다(동작 변화, 사용자 확인 전). AUTO ADJUST 는 그대로 판정 오프셋을 바꾼다 |
| 고친 결함 둘(2026-10-29) | (1) 판정 그래프의 현재 위치 커서가 이미 차트 시각인 PLAY 타이머에 배율을 다시 곱해 FREQUENCY 만큼 어긋나던 것을 없앴다(`PlayCursor.speed` 삭제. 원본은 PLAY 타이머가 실시간이라 곱한다). (2) 리듬 타이머가 배율을 몰라 마디 안에서 박이 어긋나던 것을 고쳤다(`play_speed_percent` 가 분석 모드가 아니면 시계 배율을 돌려줘 원본의 `bpm * freq` 와 같게 한다) |
| 게이지가 비었을 때 | 구간은 끝까지 판정한다(게이지 잠금). GAUGE AUTO SHIFT 가 NONE 이면 연습에서만 CONTINUE 로 친다(`PracticeSession::gauge_shift`). 다른 GAUGE AUTO SHIFT 는 설정대로다. 원본은 연습 패널로 돌아간다(`BMSPlayer.java:735-737`). 전에는 HARD 이상 게이지가 비는 순간 판정이 멈춘 채 구간이 흘렀다 |

## 10. 기록 저장(어떤 판이 무엇을 남기나)

> 웨이브 16(2026-10-14) 추가, 웨이브 18(같은 날) 개정. 웨이브 16 은 "로그인하지 않으면 기록이 저장되지 않고 램프만 남는 것 같다"는 보고를 점검해, 기록 저장이 점수 서버 설정·로그인과 무관하고 램프만 남는 것은 SCRATCH AUTO 판이라는 것을 밝혔다. 웨이브 18 은 사용자의 결정(2026-10-14, 결정 12 와 "SC 의 점수는 빼고 저장"을 대신한다)대로 바꿨다: SCRATCH AUTO 의 턴테이블 노트는 초록으로 그려지고 아무 집계에도 들어가지 않으며, 사람이 친 판은 어시스트든 아니든 전부 저장되고 서버로 간다.

### 10.1 저장되는 곳

| 곳 | 내용 | 읽는 화면 |
| --- | --- | --- |
| `scoredb.sqlite` 의 `scorelog` | 판마다 한 행. 판정 12칸(빠름·늦음), 노트 수, 콤보, BP, 램프, 게이지(코스 스테이지는 그 스테이지가 돈 코스 게이지 class·exclass·exhardclass 의 id 6~8. 10.13절), 어시스트 단계, 판정 규칙 세대(`rule_version`), 배치와 시드, 날짜, 리플레이 파일 이름, 판이 끝날 때 주 서버에 넘겼는지(`ir_submitted`. 참고용이다. 서버가 받았는지는 `ir_sync` 가 적고, 어떤 판단에도 이 값을 읽지 않는다) | 선곡의 기록 목록(R), 리플레이 슬롯 |
| `scoredb.sqlite` 의 `score` | 차트(와 모드, LN 종류)마다 한 행인 최고 기록. 램프, 램프 안의 어시스트 등급(`assist_grade` 열, 10.13절), 최고 EX 를 낸 판의 판정 12칸과 그 판의 노트 수, 최대 콤보, 최소 BP, 플레이 횟수, 클리어 횟수, 마지막 날짜 | 스킨이 그리는 선곡의 EX SCORE·RATE·RANK·MAX COMBO·MISS COUNT·판정 막대·PLAY COUNT·CLEAR COUNT |
| `scoredb.sqlite` 의 `player_stat` | 날짜별 누적(플레이 횟수, 판정 수, 플레이 시간) | - |
| `replays/<md5 앞 8자>-<시각>.ron` | 그 판의 입력과 SCRATCH AUTO·게이지·배치·시드·판정 설정. 판이 끝나는 프레임은 임시 파일만 만들고(폴더가 없거나 같은 이름의 파일이 막고 있거나 권한이 없으면 여기서 실패한다), 내용 쓰기·디스크 반영(fsync)·이름 바꾸기는 저장 스레드가 한다(10.10). 임시 파일을 만들 수 없으면 행에 리플레이 이름을 적지 않고(`replay_file` NULL, 결과 화면도 저장됨으로 말하지 않는다) `replay not saved: …` 오류를 띄운다. 저장 스레드에서 실패하면(디스크가 가득 참 등) 그 이름을 행과 기록 책에서 지우고 같은 오류를 띄운다. 어느 쪽이든 결과 화면의 전송은 메모리의 리플레이를 그대로 올린다 | 기록 목록의 리플레이, 결과 화면의 REPLAY |
| `scoredb.sqlite` 의 `play_submission` | 판마다 한 행. 판이 끝날 때 만든 점수 제출 본문(JSON)과 그 판을 친 계정(게스트면 NULL). 0.1.4 부터 적는다. `scorelog`·`score`·`player_stat` 과 한 트랜잭션으로 적으므로(`ScoreDb::record_play_kept`) 본문 없는 판이 생기지 않는다. 그 전의 판은 행이 없고 동기화가 기록 행으로 본문을 다시 만든다(10.9) | - |
| `scoredb.sqlite` 의 `play_trace` | 판마다 한 행. 그 판의 판별 trace(노트별 상태·오차, 게이지 9개, 속도 옵션 이력, 템포. 10.12) JSON 이고 아직 점수에 묶이지 않았다(`score_id` null). `scorelog` 행을 프레임 스레드가 적은 뒤, 저장 스레드가 trace 를 만들어(JSON 직렬화 포함) 따로 적는다(`SaveWorker::keep_trace` → `ScoreDb::keep_trace`). trace 를 적지 못해도 판 행은 남고, trace 는 `pending-runs` 파일로 옮겨 다음 DB 열기 때 다시 적는다. 2026-10-20 의 클라이언트부터 판마다 적는다. 그 전의 판은 시작 뒤 백그라운드 작업이 리플레이를 다시 판정해 판 행과 똑같이 나올 때만 채운다(10.12 의 소급 행) | - |
| `scoredb.sqlite` 의 `ir_sync` | 판과 점수 서버마다 한 행. 서버가 그 판을 받았는지(`accepted`), 거절했는지(`rejected`, 오류 코드·필드·이유·HTTP 상태와 거절받은 빌드 `refused_under`), 답을 못 받았는지(`retry`), 보내지 않았는지(`skipped`), 서버의 `score_id`, 보낸 횟수와 마지막 시각(밀리초), 리플레이 상태(거절이면 `replay_refused_under`), trace 상태(`trace_state`·`trace_id`·`trace_error`, 거절이면 `trace_refused_under`). 행이 없으면 "미확인"이다. 두 `*_refused_under` 열은 0.1.4 에서, trace 네 열은 2026-10-20 에 더했고 옛 DB 에는 열면서 `ALTER TABLE` 로 더한다(스키마 세대 1 그대로) | 선곡의 기록 상세(R)의 IR 줄 |
| `scoredb.sqlite` 의 `course_submission`, `course_sync` | 끝난 코스마다 한 행인 코스 제출 본문(JSON, 끝난 밀리초 포함)과 친 계정, 그리고 코스와 서버마다 서버의 답(`ir_sync` 와 같은 열, 리플레이 열 없음). 0.1.4 부터 적는다(10.9) | - |
| `pending-runs/<밀리초>-<md5 앞 8자>.ron` | 점수 DB 를 열 수 없거나 쓸 수 없을 때 친 판. 기록 행과 제출 본문, 계정, 최고 기록을 바꿀 수 있는지, 판별 trace(`trace`, 저장 스레드가 만들어 넣는다)를 담는다. 판 행은 적었는데 그 trace 를 DB 에 적지 못했을 때도 그 판의 파일(trace 만 쓰임)을 남긴다. 다음에 DB 가 열리면 한 트랜잭션씩 DB 로 읽어 들이고 그 뒤 trace 를 적은 다음 파일을 지운다(`ScoreDb::import_pending_runs`). DB 가 이미 가진 판(같은 차트·모드·LN 종류·끝난 초)은 다시 들이지 않고 trace 만 그 판에 적은 뒤 파일을 지운다. DB 가 거절한 판이나 trace 는 `kept run not read in` 경고를 띄우고 파일을 남기며, 그 뒤 파일은 계속 읽어 들인다. 그 판의 리플레이는 그대로 저장된다. 판을 따로 남기는 파일조차 만들지 못하면 그 판의 리플레이도 쓰지 않고 기록에 이름을 적지 않는다. 판마다 `score database not open: this run is kept on the side and goes into it when it opens` 오류가 뜬다 | - |
| 메모리의 기록 책(`ScoreBook`) | 시작할 때 `scorelog` 전체로 만들고 판이 끝날 때마다 한 행을 더한다. 행마다 최고 기록이 될 수 있는지를 어시스트 단계와 규칙 세대로 다시 읽는다(`rbms_store::sets_best`) | 플레이 화면의 LOCAL BEST 목표와 스코어 그래프의 BEST, 결과 화면의 이전 최고, 내장 선곡 화면 |

### 10.2 판의 종류별로 남는 것

"사람이 친 판"은 오토플레이도 리플레이 보기도 연습도 아닌 판이다. 2026-10-29 부터 PLAYBACK RATE 가 100% 가 아닌 판도 뺀다(사용자 결정. 아래 표의 해당 행과 18절). 사람이 친 판은 어떤 설정이든 기록 목록의 행·날짜 통계·리플레이(AUTO REPLAY 켬)를 남기고 점수 서버로 간다(주 서버가 기본 서버(`SERVER URL`)면 로그인했을 때, 주 서버가 IR 프로필이면 언제나. 보내지 않은 판은 `ir_submitted` 만 0 이고 나머지는 같다. 기본 서버가 설정돼 있어도 로그인하지 않았으면 보내지 않으므로 0 이다(`AppShared::sends_to_primary_ir_server`). 0.1.4 부터는 로그인하면(시작할 때 로그인 상태였으면 세션이 확인된 뒤) 서버가 아직 받지 않은 지난 판을 모두 보낸다: 10.9절의 기록 동기화).

| 판 | 기록 목록의 행·날짜 통계 | 최고 기록(EX·판정·콤보·BP) | 램프·플레이 횟수 | 리플레이(AUTO REPLAY 켬) | 점수 서버 | 원본과 다른 점 |
| --- | --- | --- | --- | --- | --- | --- |
| 손으로 친 판(로그인 여부와 무관) | 남는다 | 더 좋은 값이면 갱신 | 더 높은 램프면 갱신, 횟수 +1 | 저장 | 전송 | 없음 |
| FAILED 로 끝난 판 | 남는다. BP(`minbp`)는 게이지가 비어 도달하지 못한 노트까지 센다(`BMSPlayer.java:909`, 10.11) | EX 가 더 높으면 갱신(램프와 무관, `ScoreData.java:549`) | FAILED, 횟수 +1(클리어 횟수는 그대로) | 저장 | 전송 | 원본은 PG~BD 가 하나도 없는 판을 저장하지 않는다(`BMSPlayer.java:858-861`). 여기서는 FAILED 로 남는다 |
| 실패 화면에서 START·SELECT 로 곧장 다시 한 판 | 남는다. 다시 시작하기 전에 결과 화면이 남겼을 그대로(장면이 센 집계, FAILED) 남긴다(`app_result::record_retried_run`). 메시지 `failed run kept before starting again` | FAILED 판과 같다 | FAILED, 횟수 +1 | 저장 | 로그인했으면 그 판 하나만 기록 동기화로 곧장 보낸다, 게스트면 다음 로그인 때 | 원본은 이 판을 저장하지 않는다(`BMSPlayer.java:696-710`). "사람이 친 판은 모두 저장·전송"(사용자 결정 2026-10-14)으로 남긴다. START 를 누른 채 실패하면 곧장 다시 시작하는 것은 원본대로다 |
| SCRATCH AUTO 를 켠 판(어시스트 1). 그 판의 턴테이블 레인에 노트가 하나라도 있을 때만 | 남는다. 턴테이블 노트를 뺀 집계 그대로(10.5) | 더 좋은 값이면 갱신. 손으로 친 판과 한 행을 같이 쓴다 | LIGHT ASSIST 램프(클리어한 게이지에 따라 ASSIST EASY ~ ASSIST EXHARD 등급이 붙는다. 10.13절), 횟수 +1 | 저장(`scratch_auto: true`) | 전송. `options.scratch_auto = true`, `options.assist = ["AUTO_SCRATCH"]` 를 싣는다. 서버(웨이브 20)는 이 판을 ranked 로 저장하고 일반 판과 따로 센 최고 기록에 넣는다. 웹 순위 목록은 기본으로 일반 판만 보이고 "SCRATCH AUTO 포함" 을 켜면 A-SCR 표지와 함께 보인다. 응답은 `flags: ["SCRATCH_AUTO"]` 이고 순위는 그 포함 목록 기준이라, 결과 화면은 `IR: RANK #n (SCRATCH AUTO)` 로 쓴다(`ir_outcome::SCRATCH_AUTO_RANK_MARK`. `ScoreFlag::ScratchAuto` 는 랭킹을 막는 사유가 아니다). AUTO UPLOAD REPLAY 의 리플레이도 올라간다. 곡 선택의 IR RANKING 패널은 기본 목록을 쓴다 | 원본은 턴테이블 노트를 배경음으로 옮기고(같다) 그 판을 최고 기록·리플레이·IR 에서 뺀다(`score = false`). 사용자의 결정으로 다른 판과 똑같이 남기고 보낸다. 램프를 낮추는 것은 원본대로 유지했다(사용자가 램프는 말하지 않았다) |
| SCRATCH AUTO 를 켰지만 턴테이블 노트가 없는 차트, 턴테이블이 없는 모드(9키) | 손으로 친 판과 같다 | 손으로 친 판과 같다 | 손으로 친 판과 같다(램프를 낮추지 않는다) | 저장 | 전송 | 없음. 원본은 자동으로 친 레인에서 노트를 옮겼을 때만 어시스트를 올린다(`AutoplayModifier.modify`) |
| JUDGE WIDTH 나 LN MARGIN 을 100% 넘게 넓힌 판(어시스트 2) | 남는다 | 바뀌지 않는다 | ASSIST 램프(등급 없음. 10.13절), 횟수 +1 | 저장 | 전송. `options.assist = ["CUSTOM_JUDGE"]`. AUTO UPLOAD REPLAY 로 리플레이를 올리는 것은 여섯 판정 폭이 한 값이고 LN MARGIN 이 기본일 때뿐이다: 올리는 형식이 폭 하나만 싣고 LN MARGIN 을 싣지 않아, 다른 판은 내려받으면 다른 판정으로 재생되기 때문이다(`ir_replay::wire_carries_judge`). 로컬 리플레이는 전부 싣는다 | 원본은 리플레이를 남기지 않고 보내지도 않는다(`BMSPlayer.java:208-214`). "서버로 저장되지 않는 기록은 없다"는 결정으로 남기고 보낸다. 최고 기록을 바꾸지 않는 것은 원본대로 유지했다: 넓힌 판정의 EX 는 같은 차트를 정상 판정으로 친 EX 와 같은 잣대가 아니고, 사용자는 이것을 말하지 않았다 |
| TOTAL 을 직접 정한 판(JUDGE 탭의 TOTAL > 0, 어시스트 2) | 남는다 | 바뀌지 않는다 | ASSIST 램프(등급 없음. 10.13절), 횟수 +1. 결과 화면 `OWN TOTAL: SENT AND KEPT, BUT NO BEST SCORE` | 저장. 그 TOTAL 을 리플레이(`judge.total_override`)에 적고, 재생은 설정이 아니라 적힌 값으로 한다. 올리는 형식에 TOTAL 자리가 없어 AUTO UPLOAD REPLAY 로는 올리지 않는다 | 전송. `options.total_override` 와 `options.assist = ["CUSTOM_JUDGE"]`. 서버는 `TOTAL_OVERRIDE`·`ASSIST` 로 unranked. 이 스테이지가 낀 코스는 넓힌 판정 스테이지가 낀 코스와 같다 | 원본에는 이 설정이 없다(rbms 확장). 넓힌 판정과 같이 다룬다(사용자 결정 2026-10-15) |
| 게이지 종류, GAUGE AUTO SHIFT | 위와 같다. 게이지는 램프와, SCRATCH AUTO 판이면 램프 안의 등급(10.13절)을 정한다. 자동 전환된 판은 끝난 게이지로 적힌다 | - | - | - | - | 원본은 전환된 판의 게이지를 -1 로 적는다(`BMSPlayer.java:884`) |
| 코스의 각 스테이지 | 그 차트의 판으로 위 규칙대로 남는다(`enter_result` 가 다음 스테이지로 넘어가기 전에 적는다). 게이지는 설정의 게이지가 아니라 그 스테이지가 실제로 돈 코스 게이지(CLASS·EXCLASS·EXHARDCLASS. 시작 슬롯은 GAUGE 설정을 따른다. 10.14절)로 적히고 점수 본문의 `options.gauge` 도 그 이름을 싣는다(10.13절). 램프와 등급은 그 게이지로 정해진다 | 위와 같다 | 위와 같다 | 위와 같다 | 스테이지 점수는 위와 같다. 코스 점수도 어시스트 스테이지가 있든 없든 보낸다(사용자 결정 2026-10-14). 코스 전체로 접은 어시스트(`course_ir::CourseAssist`)를 단일 곡과 같은 어휘로 싣는다: SCRATCH AUTO 스테이지가 하나라도 있으면 `scratch_auto = true`, 태그는 `assist = ["AUTO_SCRATCH", "CUSTOM_JUDGE"]` 중 해당하는 것(각각 한 번). 어시스트가 없는 코스는 두 필드를 싣지 않아 전과 같은 본문이고, 두 필드를 모르는 옛 서버도 거절하지 않는다. 코스 최고 기록(코스 결과 화면의 "점수 저장" 여부, `course_ir::course_updates_score`)은 단일 곡 규칙과 같다: SCRATCH AUTO 스테이지만 낀 코스는 갱신하고, 넓힌 판정 스테이지가 낀 코스는 갱신하지 않는다. 서버(웨이브 20)는 SCRATCH AUTO 스테이지만 낀 코스를 단일 곡처럼 ranked 로 저장하고 SCRATCH AUTO 칸의 코스 베스트에 넣으며(웹 코스 순위는 토글을 켜야 보인다), 넓힌 판정 스테이지가 낀 코스는 unranked 로 저장하고 코스 순위와 코스 베스트에 넣지 않는다. 보낼 수 있는 코스(모든 스테이지가 사람이 친 판이고 공개 코스)는 끝날 때 점수 DB 의 `course_submission` 에 본문째 남기고, 로그인했으면 곧장 보내 답을 `course_sync` 에 적는다(메시지 `course sent` / `course rejected: …` / `course not sent yet (…): sent again on the next records sync`). 로그인하지 않았거나 서버가 답하지 않은 코스는 기록 동기화가 같은 본문(같은 `played_at`)으로 다시 보낸다. 서버는 (사용자, 코스 해시, `played_at`) 가 같으면 같은 코스로 본다 | 스테이지 램프를 원본과 대조하는 일은 하지 않았다(원본은 코스 중 스테이지 램프를 게이지로 정하지 않는다, `BMSPlayer.java:864-880`) |
| AUTOPLAY | 남지 않는다 | - | - | - | 보내지 않는다 | 없음(`MusicResult.updateScoreDatabase` 가 PLAY 모드만 적는다). 사람이 친 판이 아니다 |
| 리플레이 재생 | 남지 않는다 | - | - | - | 보내지 않는다 | 없음. 재생하는 동안 SCRATCH AUTO·GAUGE·RANDOM 은 리플레이에 적힌 값으로 돌고 끝나면 플레이어의 값으로 돌아온다(웨이브 16) |
| 연습 구간 | 남지 않는다 | - | - | - | 보내지 않는다 | 없음. 사람이 쳤지만 구간만 잘라 친 판이라 한 판의 기록이 아니다 |
| 재생 속도가 100% 가 아닌 판(PLAYBACK RATE, 2026-10-29) | 남지 않는다 | - | - | 저장하지 않는다(AUTO REPLAY 를 켜도) | 보내지 않는다. 기록 동기화도 보내지 않는다. 결과 화면의 IR 줄은 `IR: SKIPPED (playback rate / 재생 속도 변경 / 再生速度変更)` | 원본에는 일반 플레이의 배속이 없다. 사용자 결정(D2)으로 "사람이 친 판은 모두 저장·전송"(2026-10-14)의 예외다. 로컬 DB(`scorelog`·`score`·`player_stat`·`play_submission`·`play_trace`), `pending-runs` 파일, 리플레이 파일, 서버 제출, 기록 동기화가 모두 없다. 끝까지 친 판, 도중 포기(Esc), 앱 종료, 실패 화면에서 곧장 다시 하기가 모두 같다(`PlayState::keeps_no_record` 를 `record_run`·`keep_run_cut_short_by_quitting`·`record_retried_run` 이 쓴다). 연습과 달리 결과 화면은 보이고 NEW RECORD 가 없다(`updates_score` 거짓). 코스와 리플레이 감상은 설정과 무관하게 100% 라 이 행에 해당하지 않는다 |
| 도중 포기(Esc) | 사람이 친 판이면 판정한 노트 수와 상관없이 남는다(사용자 결정 2026-10-15). 단 노트를 하나도 치지 않은 판, 곧 PG·GR·GD·BD 가 모두 0 이고 쥔 롱노트가 없는 판은 지나간 노트의 POOR 나 빈 POOR 가 있어도 아무것도 남기지 않고 곧장 선곡으로 간다(사용자 결정 2026-10-19, `PlayState::has_judged_nothing`). 일반 LN 머리를 쥔 채 나간 판은 친 판이라 남는다. **노트를 하나라도 친 판의 Esc 는 로그인과 상관없이 결과 화면으로 간다**(사용자 결정 2026-10-23, [ADR](./acknowledge/2026-10-23-adr-escape-shows-the-result.md)). 닫힘 연출 없이 곧장 가고, PlayStop 소리는 Esc 를 누른 자리에서 울리며 곡 소리는 끊는다(`app_result::enter_result_of_a_left_run`). 남은 노트(아직 판정하지 않은 노트와 롱노트의 끝)는 모두 늦은 POOR(見逃し)로 세고 램프는 FAILED 다(`PlaySummary::left_unfinished`). 그래서 PG~POOR 합이 차트의 노트 수와 같고, 결과 화면의 판정 수·기록 목록 행·전송 본문·스킨 결과 화면이 읽는 기록(`run_records.last()`)이 모두 이 한 값을 쓴다. 결과 화면 그래프의 게이지 이력은 나간 자리부터 끝까지 0 으로 채운다(`PlaySession::left_record`. 게이지 사망으로 끝난 판은 사망 시각부터). 결과 화면의 판정 분포 그래프와 스킨의 초별 판정 표는 "판정된 입력" 기준이라 남은 노트의 POOR 를 넣지 않는다(끝까지 친 판에서도 같은 기존 의미). 노트를 다 처리한 뒤의 Esc(판정할 노트가 처음부터 없는 차트도 여기다. 10.5절), 그리고 이미 실패·끝으로 닫히는 중인 판의 Esc(ESCAPE 가 길게 누르기일 때 누르는 도중 게이지가 바닥나 길게 누르기가 닫히는 중에 끝나는 경우도 같다)는 원래부터 결과 화면으로 가고 위 규칙대로 남는다: 닫히는 판은 장면이 센 대로 남고 남은 노트를 見逃し POOR 로 더하지 않는다. 코스 도중의 Esc 는 그 스테이지에 남은 노트가 없으면 위처럼 결과로 가고 코스가 이어진다. 남은 노트가 있고 그 스테이지에서 노트를 하나라도 쳤으면 그 스테이지를 FAILED 로 접어 FAILED 코스 결과 화면까지 간다(스테이지 기록은 결과 화면의 전송이, 코스 기록은 `finish_course` 가 한 번만 남기고 보낸다). 그 스테이지에서 노트를 하나도 치지 않았으면 그 스테이지는 더하지 않고 코스가 그 자리에서 끝난다: 친 스테이지가 있으면 그 스테이지들로 FAILED 코스 기록을 남겨 보내고(`app_result::abandon_course`) 코스 결과 화면 없이 선곡으로 가며, 첫 스테이지였으면 코스 기록도 없다. 오토플레이·리플레이 보기·연습의 Esc 는 결과 화면 없이 선곡으로 간다 | EX 가 더 높으면 갱신(FAILED 판과 같다) | FAILED, 횟수 +1 | 나간 시점까지의 입력으로 저장 | 결과 화면의 전송 경로 하나로 보낸다(별도의 "화면 없는 전송"은 이 갈래에서 없앴다). 로그인했으면 보내고, 서버가 없거나 로그인하지 않았으면 결과 화면의 전송이 건너뜀(`IrStatus::Off`)이 되며 DB 에는 늘 남아 다음 로그인 때 기록 동기화가 보낸다. IR 프로필로의 전송은 다른 판과 같다. 알림 `run left mid-song kept as FAILED`·`course left part-way kept as FAILED` 는 없어졌다(결과 화면이 판이 남았음을 보인다) | 원본(`BMSPlayer.stopPlay`)도 남은 노트가 있으면 FAILED 로 바꿔 점수 데이터가 있을 때 결과 화면으로 간다(`BMSPlayer.java:693-742`)는 점은 같다. 다른 점: 원본은 닫힘 연출을 거치고 남은 노트를 판정 수에 넣지 않고 BP 에만 더한다(`BMSPlayer.java:909`). 여기서는 연출 없이 곧장 가고 남은 노트를 見逃し POOR 로 센다(사용자 결정 2026-10-14). 결과 화면의 다시 하기(R, N, 파란 건반 + START)를 허용한다(원본 결과 화면도 받는다). 리플레이를 재생하면 남은 노트가 지나가 見逃し POOR 가 되므로 판정 수는 기록과 같지만, 재생은 곡 끝까지 가고 게이지는 기록과 다르게 끝날 수 있다(쉬운 게이지면 클리어로 끝날 수도 있다). 알려진 한계다. Esc 직후 곡 소리가 실제로 멈추는지와 PlayStop 이 한 번만 나는지는 오디오 장치가 없는 헤드리스 테스트가 보지 못해 실기 점검 항목이다 |
| 플레이 중 앱 종료(창 닫기·Cmd+Q·메뉴 Quit·Dock 종료) | Esc 의 저장 규칙과 같고 결과 화면은 거치지 않는다(사용자 결정 2026-10-15 "서버로 저장되지 않는 기록은 없다", S19 2차 REC2-2): 사람이 친 판은 남은 노트가 있으면 남은 노트를 見逃し POOR 로 센 FAILED 판, 끝났거나 닫히는 판은 결과 화면이 남겼을 그대로 남는다. 남은 노트가 있어도 노트를 하나도 치지 않은 판은 Esc 처럼 남기지 않는다(사용자 결정 2026-10-19, `keep_run_cut_short_by_quitting`). 코스 도중이면 그 스테이지를 접고 코스를 남긴다(마지막 스테이지였거나 실패한 스테이지면 그대로, 아니면 FAILED 코스. 노트를 하나도 치지 않은 스테이지는 접지 않고 그 앞 스테이지까지의 FAILED 코스만 남긴다). 코스의 스테이지 사이(LOADING·결정 화면)에서 끄면 친 스테이지까지의 FAILED 코스가 남는다. 이벤트 루프의 `exiting` 에서 저장 스레드를 기다리기 전에 남기므로(`App::keep_what_quitting_cuts_short`, `app_result::keep_run_cut_short_by_quitting`) 리플레이까지 디스크에 놓인다. 플레이 화면에서 끄면 Esc 와 같이 판을 남기기 전에 설정도 저장한다(남기지 않는 오토플레이·연습도 같다): 그 판에서 키로 바꾼 하이스피드·커버·리프트·히든과 AUTO ADJUST 가 옮긴 판정 오프셋이 다음 실행에 남는다(S19 3차 R3-REC-1). 플레이 화면이 아닌 곳에서 끌 때는 설정을 쓰지 않는다. 이 빌드가 읽지 못한 설정 파일(더 새 빌드가 쓴 파일 등)을 기본값으로 덮지 않기 위해서다(`run` 의 "leaving the file alone"). 메시지 `run cut short by quitting kept; it is sent on the next start` | Esc 판과 같다 | Esc 판과 같다 | 저장 | 끄는 중에는 보내지 않는다(기록 동기화도 코스 전송도 시작하지 않는다). 다음 실행의 기록 동기화가 판과 코스를 보낸다. IR 프로필로는 보내지 않는다(테스트 부채 3.66) | 원본은 창을 닫으면 설정만 저장하고 진행 중인 판을 저장하지 않는다(`MainController.dispose`, `MainController.java:590-612`). 강제 종료(kill -9·정전·크래시)는 여전히 남지 않는다 |
| 코스 스테이지 사이에서 나감(다음 스테이지의 LOADING 에서 Esc, 결정 화면에서 나감, 스테이지 차트를 못 읽음, 라이브러리에 없음) | 코스가 그 자리에서 끝난다(S19 2차 REC2-1). 친 스테이지가 있으면 그 스테이지들로 FAILED 코스 기록을 남겨 보내고(`app_result::abandon_course`, 메시지 `course ended before its last stage kept as FAILED`), 서 있던 스테이지는 치지 않았으므로 아무것도 더하지 않는다. 친 스테이지가 없으면(1스테이지의 결정 화면) 남기지 않는다. 코스 제약으로 바뀐 설정은 플레이어의 값으로 돌아간다. 전에는 코스가 끝나지 않아 다음에 고른 단일 곡이 그 코스의 다음 스테이지로 접히고 엉뚱한 곡이 든 코스 기록이 보내졌다 | - | - | - | 코스는 곧장 보낸다(코스 결과 화면의 전송과 같다) | 없음(코스 도중 포기를 FAILED 코스로 남기는 것은 결정 9 다) |

### 10.3 최고 기록 한 행과 노트 수

1. SCRATCH AUTO 판은 손으로 친 판과 한 행(`score`)을 같이 쓴다. 두 판의 노트 수가 다르다(턴테이블 노트만큼). 그래서 최고 기록 행의 노트 수는 그 행이 지닌 EX 를 낸 판의 것이다: 처음으로 최고 기록이 될 수 있는 판이 정하고, 그 뒤로는 EX 를 갱신한 판만 바꾼다(`merge_best`). RATE·RANK 는 언제나 같은 판의 EX 와 노트 수로 계산된다. 웨이브 18 전에는 최고 기록을 낼 수 있는 판마다 노트 수를 덮어써서, 노트 수가 다른 판이 섞이면 RATE 가 100% 를 넘을 수 있었다.
2. EX 는 노트 수가 달라도 그대로 비교한다(사용자의 결정: 따로 두지 않는다). SCRATCH AUTO 판은 턴테이블 노트만큼 EX 상한이 낮으므로, 같은 실력이면 손으로 친 판의 EX 가 이긴다.
3. 기록 목록(R)과 결과 화면은 판마다 그 판의 노트 수로 읽는다.
4. 노트 수의 정의는 디코더가 롱노트를 원본처럼 정리한 뒤의 값이다(2026-10-15): 롱노트 몸통 안의 일반 노트는 배경음이 되어 세지 않고, 끝나지 않은 롱노트 머리는 모델에서 지워져 세지 않는다. 이 둘이 있는 차트(감사 때 11,336 차트 중 수십 개)는 정리 전의 판과 노트 수가 다르다. 옛 판의 노트 수·EX 는 그 판 기록에 적힌 값 그대로 읽고(3번), 최고 기록 비교는 2번대로 EX 절대값이라 이런 차트에서는 옛 최고 기록이 더 높게 남을 수 있다. 곡 DB 의 `notes` 는 `PARSER_VERSION` 3 의 전체 재스캔으로 새 값이 된다(같은 릴리스 안에서 4·5 로 더 올라 지금은 5, `crates/rbms-library/src/songdb/schema.rs`). 규칙 세대(`SCORE_RULE_VERSION`)는 이 변경만으로 올리지 않았다(`docs/acknowledge/2026-10-15-adr-long-note-body-notes.md` 6항).

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
| 그리기 | 뺀 노트는 필드에만 남아(`PlaySession::field_timelines`) 단색으로 그려진다. 색은 SCRATCH AUTO COLOR 다(저장값이 없으면 출하 초록 `#30E050`, 알파 255. 2절). 일반 노트와 롱노트의 시작·끝은 그 색(출하값은 `AUTO_LANE_NOTE`), 롱노트 몸통은 사용자 알파 x 0xA0/255(출하값은 `AUTO_LANE_LONG_BODY`와 비트까지 같다). 스킨이 정한 불투명도(노트 알파와 Note Opacity 행)는 사용자 알파에 곱해진다. 스킨이 고른 노트 그림의 자리(크기·위치)에 엔진이 그리므로 기본 팩과 남의 Lua 스킨(ModernChic 캡처)이 같다. 판정선에 닿으면 사라지고, PAST NOTES·처리된 노트 표시·팝픈의 떨어지는 노트에 해당하지 않는다. 내장 필드(스킨 없음)도 같은 색이다. 스킨의 색 행(Scratch Note·Scratch LN)은 이 노트에 닿지 않는다 | 원본은 그리지 않는다(배경음이 된다). 스킨 그림에 색을 곱하는 대신 단색으로 그린 이유: 그림 색에 곱하는 방식은 빨간 턴테이블 노트(기본 팩과 대부분의 스킨)를 검게 만들고, 그림의 알파로 색을 바꾸는 그리기는 지금 렌더러(GPU 셰이더)에 없다 |
| 지뢰 | 턴테이블 레인의 지뢰는 판정에서도 그리기에서도 뺀다 | 같다(`moveToBackground` 는 지뢰를 배경음으로도 옮기지 않는다) |
| 키음 | 각 노트 머리(롱노트는 시작)의 키음이 제 시각에 키 버스로 울린다. 롱노트 끝과 지뢰는 울리지 않는다(오토플레이와 같다) | 원본은 배경음(BGM 버스)으로 울린다. 키 음량을 따르게 키 버스에 두었다 |
| 턴테이블 입력 | SCRATCH AUTO 가 켜져 있으면(그 차트에 턴테이블 노트가 없어도) 턴테이블 키의 누름·뗌을 판정에 넘기지 않는다. 공 POOR 도, 빔도, 키음도 없다. 리플레이에는 입력으로 적히지만 재생할 때도 같은 이유로 무시된다 | - |
| TOTAL | 차트의 #TOTAL, 없으면 턴테이블을 포함한 원래 노트 수로 정한 기본값 그대로. 노트 하나가 올리는 게이지는 TOTAL 을 판정되는 노트 수로 나눈 값이다 | 같다(원본은 디코드할 때 TOTAL 을 정하고 모디파이어를 그 뒤에 적용한다) |
| 턴테이블 노트만 있는 차트 | 판정되는 노트가 0 개라 BGM 만 있는 차트처럼 저절로 끝나지 않는다. 처음부터 남은 노트가 없으므로 Esc(와 START+SELECT)는 끝까지 친 판처럼 결과 화면으로 간다. 램프는 NO PLAY 이고 FAILED 로 남지 않는다(`PlaySession::all_notes_resolved`, 테스트 `leaving_a_chart_with_no_note_to_judge_ends_it_on_its_result`) | 원본도 `getPastNotes() == getNotes()`(0 = 0)라 끝난 판으로 넘긴다(`BMSPlayer.java:953-958`). 원본은 판정이 하나도 없는 단일 곡 판의 점수를 만들지 않지만(`:858-861`) 여기서는 사용자 결정(2026-10-15)대로 남기고 보낸다 |
| 리플레이 | 리플레이의 `scratch_auto` 로 재생하면 같은 차트를 같이 빼므로 같은 결과(턴테이블이 초록이고 세지 않는 것 포함)가 나온다. 분석의 되감기(`seek`)도 필드의 차트에서 다시 빼서 같다 | - |

### 10.6 선곡과 결과 화면

1. 선곡: SCRATCH AUTO 로 친 차트도 EX SCORE·RATE·RANK·판정 막대가 채워진다(그 판의 노트 수 기준. 캡처 `select-recorded-scratch-auto.png`). 넓힌 판정으로만 친 차트는 램프와 횟수만 있고 점수는 `-` 다(`select-recorded-widened.png`. 기본 스킨의 `holds_score`, 웨이브 16).
2. 결과 화면은 오른쪽 위에 `SCRATCH AUTO: TURNTABLE NOT COUNTED, ASSIST LAMP`(2026-10-22 전에는 `…, LIGHT ASSIST LAMP`. 넓힌 판정은 `WIDENED JUDGE: SENT AND KEPT, BUT NO BEST SCORE`)를 노란 글자로 쓴다. 스킨의 숫자만으로는 턴테이블이 빠진 것과 램프가 낮아진 것을 알 수 없기 때문이다. AUTOPLAY·리플레이 재생에는 쓰지 않는다(캡처 `result-recorded-scratch-auto.png`).
3. LOCAL BEST 목표와 스코어 그래프의 BEST, 결과 화면의 이전 점수(스킨과 내장 화면의 이전 EX·이전 최고 EX)는 SCRATCH AUTO 판도 센다. 넓힌 판정의 판과 세대 2 의 SCRATCH AUTO 판은 세지 않는다.
4. PLAY 메뉴의 SCRATCH AUTO 행 설명은 `Let the game play the scratch lane, drawn green; its notes are not counted`, 정보 패널의 덧말은 `WHILE ON: TURNTABLE NOT COUNTED, ASSIST LAMP BY GAUGE (E.G. ASSIST HARD)`(`stage/play_menu.rs`). 설정 화면의 JUDGE WIDTH·LN MARGIN 행 설명과 정보 패널은 넓힌 판정이 최고 기록만 바꾸지 않고 리플레이와 함께 남고 보내진다고 말한다.
5. 노트를 하나라도 치고 Esc 로 나간 판의 결과 화면은 끝까지 친 판과 같은 화면이다(10.2절의 도중 포기 행). 판정 수는 남은 노트를 見逃し POOR 로 센 값이고 게이지 그래프는 나간 자리부터 0 이다. 코스 도중이면 그 스테이지가 FAILED 로 접히고 코스 결과 화면(불합격)이 이어서 뜬다. 서버가 없거나 로그인하지 않았으면 전송 줄이 건너뜀이다.

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
| Esc 포기 | `apps/rbms-player/src/stage/play/mod.rs`(`leave_run`, `keeps_a_left_run`, `has_judged_nothing`), `apps/rbms-player/src/app_result.rs`(`RunEnd`, `show_result`, `enter_result_of_a_left_run`. `record_left_run`·`leave_course` 와 두 알림 상수는 지웠다), `crates/rbms-play/src/{session,record}.rs`(`PlaySummary::left_unfinished`, `PlaySession::left_record`, `GaugeLogRecorder::closed_left`), 테스트 `app_result/record_tests.rs`(`leaving_mid_song_shows_a_failed_result_with_the_counts_it_keeps`, `leaving_mid_song_signed_in_sends_the_run_once_from_its_result`, `leaving_mid_song_signed_out_still_shows_its_result`, `leaving_before_any_note_is_judged_keeps_nothing`, `leaving_after_only_an_empty_poor_keeps_nothing`, `leaving_after_a_note_went_by_unhit_keeps_nothing`, `leaving_while_holding_the_first_long_note_keeps_a_failed_run`, `leaving_a_chart_with_no_note_to_judge_ends_it_on_its_result`, `leaving_while_holding_a_long_note_counts_it_once`, `leaving_a_watched_replay_or_autoplay_keeps_nothing`, `leaving_mid_course_ends_on_a_failed_course_result`, `leaving_a_course_stage_with_no_note_left_ends_it_as_played`, `left_behind.rs` 의 `escape_on_a_course_stage_that_judged_nothing_ends_the_course_without_the_stage`·`escape_on_the_first_course_stage_that_judged_nothing_keeps_nothing`·`quitting_before_any_note_is_judged_keeps_nothing`·`quitting_in_a_course_stage_that_judged_nothing_keeps_the_course_without_the_stage`), `stage/play/screen_tests/run_tests.rs`(`escape_with_notes_left_on_a_skinned_run_goes_straight_to_its_result`), 닫히는 판 `stage/play/tests.rs`(`escape_held_through_a_closing_run_reaches_the_result_screen_with_the_notes_it_judged`), `crates/rbms-play/src/record/tests.rs`(`a_history_closed_as_left_falls_to_zero_from_where_it_stopped`, `a_record_left_part_way_counts_the_rest_as_poor`), `session.rs` 의 `a_run_left_unfinished_*` |
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
| 순서 | 오래된 판부터(`date`, `id` 순). 단 서버가 답하지 못한(`retry`) 판은 아직 묻지 않은 판 뒤로, 실패한 횟수가 적은 판부터 보낸다. 서버 쪽에서 늘 실패하는 옛 판 셋이 맨 앞을 차지해도 "세 번 연속 실패면 멈춤"에 걸려 뒤 판이 영영 못 나가는 일이 없다(S19, IR-12). 점수 다음에 코스를, 그 다음에 trace 를, 그 다음에 리플레이를 올린다 |
| 고르는 비용 | 보낼 판을 고르는 일은 일꾼 스레드가 읽기 전용 DB 연결을 따로 열어 한다(`ScoreDb::open_read_only`, WAL 이라 프레임 스레드의 쓰기와 함께 돈다). 프레임 스레드는 가장 새 판·코스 id 두 개만 읽고(`newest_play_id`, `newest_course_id`, 수 µs) 일꾼을 띄운다. 일꾼은 그 id 까지만 보내므로(`SyncWalk::up_to`) 동기화가 도는 사이 끝난 판은 그 판을 남긴 쪽(결과 화면의 전송, 포기한 판의 동기화)이 보낸다. 곡 라이브러리 대조는 일꾼이 고른 차트의 md5 만 프레임 스레드에 물어(`SyncEvent::Picked`) 라이브러리 색인으로 찾는다. 세대 2 이하 SCRATCH AUTO 판의 "보내지 않음" 표시는 DB 에 쓰지 않고 상태를 읽을 때 만든다(`ScoreDb::sync_states`) |
| 언제 | 시작할 때 로그인 상태면 세션이 확인된 뒤(`whoami` 성공. 오프라인이면 하지 않는다), 로그인·가입에 성공했을 때, 설정 NETWORK 의 SCORES 묶음 첫 줄 `SYNC RECORDS NOW` 를 눌렀을 때, 앱 종료로 끊긴 판이 다음 시작에 남아 있을 때. 게스트면 하지 않고, 수동 버튼은 `log in first`(서버가 없으면 `set SERVER URL first`)를 띄운다. 보내는 계정은 서버가 토큰의 주인으로 알려 준 login id(`whoami`·로그인 응답)뿐이다. 설정에 적은 PLAYER ID 는 대신 쓰지 않고(다르면 모든 판이 403 으로 거절된다), 토큰은 있는데 주인을 아직 모르면 수동 버튼은 `account not confirmed yet` 을 띄우고 자동 동기화는 하지 않는다. 이미 돌고 있으면 수동 버튼은 `records sync already running`, 결과 화면이 보낸 판은 지금 동기화가 끝난 뒤 이어서 보낸다 |
| 본문 | 판이 끝날 때 적어 둔 제출 본문(`play_submission.body`)을 그대로 보내고 `player.id` 만 동기화하는 계정으로 바꾼다. 본문이 없는 판(0.1.3 까지)은 기록 행으로 다시 만든다(아래 "재구성") |
| 속도 | 백그라운드 스레드에서 보내 화면 프레임을 막지 않는다. 점수는 1.5초에 하나(분당 40, 서버 한도 60), 리플레이는 7.5초에 하나(분당 8, 서버 한도 10), trace 는 2.5초에 하나(분당 24, 서버 한도는 기획 제안 30). 남는 몫은 동기화 중에 친 판의 몫이다(`record_sync.rs` 의 `SCORE_SEND_INTERVAL`, `REPLAY_SEND_INTERVAL`, `TRACE_SEND_INTERVAL`) |
| 답의 분류(`rbms_ir::submit_verdict`) | 2xx 수락은 `accepted`(중복 응답이면 "이미 있음"으로 센다). 4xx(400·403·404·409·413 등)는 `rejected` 로 오류 코드·필드·이유·HTTP 상태와 거절받은 두 빌드를 적고, 빌드가 바뀐 뒤의 동기화나 SYNC RECORDS NOW 에서 다시 보낸다(위 "대상"). 429 는 60초 기다렸다 같은 판을 다시 보내고, 두 번 기다려도 429 면 그 판을 `retry` 로 적고 동기화를 멈춘다. 네트워크 오류·응답 해석 실패·5xx 는 `retry` 이고 세 번 연속이면 서버가 닿지 않는 것으로 보고 멈춘다. 401·서버 없음은 그 판을 `retry` 로 적고 멈춘다 |
| 리플레이 | 수락된 판 중 로컬 리플레이 파일이 있고 AUTO UPLOAD REPLAY 가 켜져 있을 때 올린다. 서버의 `score_id` 를 실어 그 점수에 묶는다. 상태는 `pending`(아직 안 올림, 설정이 꺼져 있을 때 포함), `uploaded`, `failed`(일시 실패, 다음에 다시), `rejected`(점수의 거절과 같은 규칙: 빌드가 바뀌었거나 손으로 누를 때 다시. 413 처럼 서버 설정이 바뀌어야 받아지는 리플레이가 시작할 때마다 다시 올라가 업로드 몫을 쓰지 않는다), `skipped`(unranked 응답이거나 판정 폭을 와이어가 싣지 못하는 판, `ir_replay::wire_carries_judge`), `missing`(파일이 지워짐), `none`(리플레이 없음). 리플레이는 그 점수를 보낸 계정으로만 올린다. 리플레이 단계는 동기화가 시작되기 전에 저장 스레드에 넘긴 쓰기가 끝나기를 기다린 뒤 파일을 읽는다(`SaveWorker::settled`, 최대 10초 `REPLAY_SAVES_WAIT`). Esc·다시 하기로 남긴 판은 남기자마자 그 판 하나의 동기화를 띄우므로, 기다리지 않으면 아직 쓰이지 않은 파일을 `missing` 으로 영구히 적었다(S19 2차 THR2-3). 10초 안에 끝나지 않으면 리플레이를 읽지 않고 `pending` 으로 남겨 다음 동기화가 올린다 |
| trace | 수락된 판 중 `play_trace` 행이 있는 판의 trace 를, 서버가 `/health` 의 `capabilities.traces` 를 참으로 알리고 AUTO UPLOAD TRACE 가 켜져 있을 때 `POST /charts/{md5}/traces` 로 올린다(Bearer, 요청 제한 30초 `TRACE_REQUEST_TIMEOUT`). 서버가 답한 `score_id` 를 실어 그 점수에 묶는다. 서버에 `/health` 를 묻는 것은 그 동기화에 보낼 trace 가 있을 때(서버가 가진 판의 trace 후보, 또는 이번에 보낼 판 중 trace 가 있는 판)뿐이다. 보낼 것이 없으면 묻지 않아 자동 동기화가 늦어지지 않는다. 능력 플래그를 알리지 않는 옛 서버(답이 없거나 플래그가 없음)에는 보내지 않고 `pending` 으로 남겨, 서버가 플래그를 알린 뒤의 동기화가 보낸다. 저장 스레드가 trace 를 적기 전에 서버가 점수를 받은 판은 `none` 으로 적히지만, `play_trace` 행이 생긴 뒤의 전체 동기화가 후보로 집어 보낸다. 상태는 리플레이와 같은 이름(`pending`·`uploaded`·`failed`·`rejected`·`missing`·`none`)이다. 보낼 때 DB 를 그 순간 읽지 못하면(바쁨·I/O 오류) `missing` 이 아니라 `failed`(`kept trace not read: ...`)로 적어 다음 동기화가 다시 보낸다. `missing` 은 행이 정말 없을 때뿐이다. 그리고 `skipped` 는 보내지 않는 trace 다: 본문이 `TRACE_MAX_BYTES`(2 MiB)를 넘거나(`trace larger than the server takes`), 서버가 `score_id` 없이 점수를 받았거나, 남긴 본문을 읽을 수 없을 때. 넘는 trace 도 로컬에는 그대로 남는다. 거절(예: 서버 일관성 검사의 `IR_TRACE_MISMATCH`)은 리플레이와 같은 규칙으로 빌드가 바뀌거나 손으로 누를 때 다시 보낸다. 결과 화면이 보낸 판은 서버가 점수를 받은 뒤 그 판 하나의 동기화가 trace 를 올리고(`write_live_outcome`), Esc·다시 하기로 남긴 판은 그 판 하나의 동기화가 점수를 보낸 바로 뒤에 올린다. 이름 붙인 판만 보내는 동기화도 그 판들의 trace 는 보낸다. 저절로 시작한 이름 붙인 판의 동기화가 trace 만 보낼 때는 상태 줄과 메시지를 띄우지 않는다(결과 화면이 이미 IR 줄을 보였다. 멈춤은 띄운다) |
| 결과 화면의 전송 | 판이 끝날 때의 전송도 같은 `ir_sync` 열을 쓴다(`AppShared::write_live_outcome`). 그 전송이 끝나기 전에 다음 판이 제출되면 앞 판의 답은 버려지고(전과 같다), 그 판은 행이 없어 다음 동기화가 보낸다(멱등이라 무해). 전송이 아직 들고 있는 판은 그 사이에 시작하는 동기화(버튼, 게스트였다가 로그인)의 대상에서 뺀다. 같은 판을 둘이 겹쳐 보내면 서버가 같은 밀리초의 둘째를 500 으로 거절할 수 있고, 나중에 적는 쪽이 리플레이 상태를 덮기 때문이다 |
| 코스 | 끝날 때 남긴 코스(`course_submission`) 중 그 서버가 받지 않은 것을 점수 다음에 보낸다. 본문은 남긴 그대로이고 `player.id` 만 동기화 계정으로 바꾼다. 다른 계정으로 친 코스는 뺀다. 답의 분류와 거절의 다시 보내기 규칙은 판과 같다. 코스 엔드포인트가 없는 서버(`Unsupported`)는 `skipped`·`NO_COURSE_ENDPOINT` 로 적고 다음에 다시 본다. 이름 붙인 판만 보내는 동기화(포기한 판 등)는 코스를 보내지 않는다 |
| 멈춤·취소 | 로그아웃과 서버를 다시 만들 때(로그인으로 토큰이 바뀔 때, SERVER URL 변경) 돌던 동기화를 취소한다. 이미 받은 답은 그대로 적혀 있다. 보내는 중인 요청은 끊지 않는다(요청 제한 시간 5초) |
| 진행과 결과 | 상태 줄은 진행 중 `syncing records 12/340`·`syncing traces 5/120`·`syncing replays 3/80`, 끝나면 `records: 120 sent, 30 already on server, 5 rejected, 2 failed, 1 skipped; traces: 120 uploaded; replays: 80 uploaded, 3 failed`(0 인 항목은 뺀다. 보낸 판이 없으면 `records: nothing sent`), 멈추면 `records sync stopped: not signed in (401)`·`records sync stopped: server unreachable`·`records sync stopped: rate limited`·`records sync cancelled` 다. 같은 줄이 메시지로도 뜬다(거절·실패·멈춤이 있으면 경고). 보낼 판이 없으면 수동 버튼만 `records: nothing to send` 를 띄우고 자동 동기화는 조용하다. SYNC RECORDS NOW 행의 값은 도는 동안 `SENDING 12/340`·`TRACES 5/120`·`REPLAYS 3/80`, 아니면 `>` 다 |
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
| 다른 스레드의 기다림 | 기록 동기화 일꾼처럼 프레임 스레드가 아닌 스레드가 "지금까지 넘긴 쓰기가 끝났는지"를 기다려야 하면 `SaveWorker::settled` 가 그 표시를 넘긴다. 프레임 스레드는 기다리지 않는다 |
| 코스 도중의 설정 저장 | 코스 제약은 그 판에만 걸고 설정 파일에는 플레이어의 값을 쓴다(`AppShared::settings_to_save`, S19 2차 감사 누락). 원본도 제약을 `BMSPlayer` 에서만 건다. 되돌리는 것은 제약이 실제로 좁힌 값뿐이다(`CourseOverrides::put_back_into`, S19 3차 R3-REC-2): 단위 인정(CLASS·MIRROR·RANDOM)이면 배치, NO SPEED 면 하이스피드·CONSTANT·커버·리프트·히든과 그 켜짐. NO SPEED 가 없는 코스에서 플레이어가 코스 도중 바꾼 하이스피드·커버·리프트·히든은 코스 중 저장에도, 코스가 끝난 뒤(`end_course`)에도 플레이어의 값으로 남는다. 원본 `BMSPlayer.saveConfig`(831-847)가 NO_SPEED 일 때만 저장을 건너뛰고 그 밖에는 이 값들을 저장하는 것과 같다. 전에는 `config.play` 전체를 코스 시작 전 값으로 되돌려 이 값들을 잃었다. 전에는 코스 스테이지가 끝날 때마다 제약이 걸린 값이 저장돼, 코스 도중 앱이 죽거나 꺼지면 다음 실행이 하이스피드·커버·리프트·히든·랜덤을 잃은 채 시작했다. 코스가 남은 채 새 코스를 시작하면 앞 코스를 먼저 끝내 제약 걸린 값이 "원래 값"으로 잡히지 않는다(`start_course`). 앞 코스는 스테이지 사이에서 나간 코스와 같이 끝나, 손으로 친 스테이지가 있으면 FAILED 코스로 남겨 보낸다(`app_result::abandon_course`, S19 2차 리뷰 5) |
| 임시 파일 이름 | `write_atomic` 의 임시 파일은 `<이름>.<프로세스 id>.<프로세스 안 일련번호>.tmp` 다. 같은 프로세스의 두 스레드가 같은 파일을 저장해도 서로의 임시 파일을 덮지 않는다(`crates/rbms-store/src/atomic.rs`) |
| DB 쓰기 | 판 행과 제출 본문 쓰기(SQLite, WAL·`synchronous=NORMAL` 이라 한 판 약 0.1ms)와 동기화 답 쓰기는 프레임 스레드에 남겼다. 동기화 대상 선정은 일꾼이 한다(10.9 "고르는 비용") |
| 단위 테스트 | 단위 테스트의 앱은 저장을 부른 자리에서 바로 쓴다(`SaveWorker::for_this_build`). 저장 직후 파일을 읽는 테스트가 결정적이도록 하기 위해서다. 스레드 자체와 "프레임 스레드가 쓰지 않는다"는 것은 스레드를 쓰는 테스트가 따로 본다(`save_worker/tests.rs`, `record_tests.rs` 의 `entering_the_result_writes_no_file_on_the_frame_thread`) |
| 리플레이 보존 | 시작할 때의 리플레이 정리(차트당 최근 3개와 최고 기록, 합계 2GiB)는 서버가 설정돼 있고 로그인 계정이 있으며 AUTO UPLOAD REPLAY 가 켜져 있으면, 그 서버·계정으로 아직 올라가지 않은 리플레이(점수가 아직 수락되지 않았거나 리플레이가 `pending`·`failed`·`rejected` 인 판, 다른 계정의 판과 세대 2 이하 SCRATCH AUTO 판 제외)를 최근 것처럼 남긴다. 바이트 한도는 그대로 앞선다(`ReplayPolicy`, `UploadHold`, `scoredb_store::upload_hold`). 서버가 없거나 로그아웃 상태거나 업로드가 꺼져 있으면 전처럼 정리한다 |
| 테스트 | `app_result/record_tests/left_behind.rs`(저장 중인 리플레이를 기다려 올림, 코스 LOADING 의 Esc·라이브러리에 없는 스테이지·코스 위의 코스·코스 도중의 설정 파일, 앱 종료로 끊긴 판·오토플레이·코스·스테이지 사이, 종료 때의 설정 저장 `quitting_in_the_middle_of_a_run_keeps_the_settings_changed_in_it`·`quitting_in_the_middle_of_an_autoplay_keeps_the_settings_changed_in_it`·`quitting_outside_a_run_leaves_the_settings_file_alone`, 코스가 좁힌 값만 되돌림 `the_speed_set_during_a_course_that_does_not_lock_it_is_kept`·`a_course_that_locks_the_arrangement_puts_back_only_the_arrangement`), `save_worker/tests.rs`(대신한 설정 저장 건너뛰기, 순서, 실패 전달, 끝날 때 남김없이 쓰기, 플레이 중 도착한 응답의 설정 저장), `crates/rbms-store/src/tests.rs`(`threads_of_one_process_writing_one_file_never_share_a_temp_file`, `a_staged_write_fails_where_its_file_cannot_be_created_and_cleans_up_when_dropped`), `scoredb/tests.rs`(`replay_gc_keeps_a_replay_still_to_go_up`) |

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
| 판정 시각 | 입력은 그 입력의 시각, 시간으로 진행되는 판정은 기한이 된 순간에 판정한다. 게이지가 빈 순간부터 판정하지 않는다. 컨트롤러는 gilrs 가 찍은 시각, 키보드는 present 대기에 막히지 않는 이벤트 루프가 받은 시각이라 둘 다 예전보다 최대 한 프레임 이르다(3.4절) | 프레임 경계가 정하던 곳만 다르다. 그대로 비교한다 |
| 노트 수 | 같은 레인 롱노트 몸통 안의 일반 노트는 배경음, 끝나지 않는 머리는 지운다(곡 DB `PARSER_VERSION` 3, 지금은 5) | 그때의 노트 수(`max_ex`·판정 수)로 남고 그대로 비교한다. 영향받는 차트(수십 개 이하)만 EX 의 만점이 다르다 |

- 최고 기록의 `avgjudge`·`minbp` 는 DB 를 열 때마다 이력에서 다시 접는다(`ScoreDb::migrate` → `refold_reference_bests`). 옛 빌드가 그 사이에 같은 DB 에 옛 뜻의 값을 접어 넣어도 다음 실행에서 바로잡힌다. 값이 이미 맞는 행은 쓰지 않는다.
- 제출 본문은 `extra.rule_version` 에 그 판의 세대를 싣는다. 서버는 이것이 없는 행(옛 클라이언트)의 `avgjudge` 를 부호 있는 평균으로 읽는다.

### 10.12 판별 trace(2026-10-20, 웹 AI 기능 W4)

> 정본은 `docs/plan/2026-10-19-web-ai-features.md` 3.1·3.2절과 `docs/acknowledge/2026-10-19-adr-web-ai-decisions.md`(Q5 업로드 기본 켬, Q6 게이지 9개, Q11 GREEN 과 레인 통과 ms). 서버 경로는 이 절을 쓸 때 아직 없다.

| 항목 | 동작 |
| --- | --- |
| 어떤 판 | 기록이 남는 모든 사람 판: 결과 화면, Esc 로 남긴 판, 앱 종료로 끊긴 판, 실패 뒤 곧장 다시 하기, 코스 스테이지. 모두 `record_run` 을 지난다. 노트를 하나도 치지 않아 남기지 않는 판과 오토플레이·리플레이 보기·연습은 만들지 않는다. 판정 폭을 넓힌 판도 만든다(리플레이의 `wire_carries_judge` 조건에 묶지 않는다) |
| 내용 | 형식 `rbms-trace` 세대 1(`rbms_ir::PlayTrace`). 차트 md5·sha256, 모드, `lntype`, 배치(점수의 `options.random` 과 같은 표기), SCRATCH AUTO 가 실제로 친 레인 마스크(친 노트가 없으면 빈 배열). 노트 열은 판정되는 차트(배치 적용 뒤, SCRATCH AUTO 레인 노트 제외)의 판정 객체 전부를 차트 시각·레인 순으로: `time_us`, `lane`, `kind`(0 일반, 1 롱노트 머리, 2 일반 LN 끝, 3 CN·HCN 끝), `state`(0 아무것도 소비하지 않음, 1 PG … 5 POOR), `err_us`(그 상태를 정한 판정의 오차, 양수가 빠름. 뒤에 같은 노트에서 난 空POOR 가 덮지 않는다. `NoteMark::judged_us`). 空POOR 는 누른 시각과 레인(`JudgedNote::note_us - delta_us`). 게이지 9개의 0.5초 열(실패한 판은 실패 시점부터 0 으로 채운 결과 화면용 열 그대로), 끝난 게이지, 9개의 경계, `failed_at_ms`, `play_time_ms`. 템포는 첫 BPM 과 BPM·SCROLL 이 바뀌거나 STOP 이 있는 타임라인. 템포 점은 `TRACE_MAX_TEMPO_POINTS`(4,096)개까지만 싣고, 넘으면 처음 2,048개와 끝 2,048개를 남긴다(잘렸다는 필드는 형식에 없어 더하지 않았다). 넘어도 trace 를 `skipped` 로 만들지 않고 보낸다 |
| 속도 옵션 | 시작값·끝값과 그 사이 바뀐 필드마다 `{t_ms, field, value}`(켬은 1). 필드는 `hispeed`, `constant`, `lane_cover`·`lift`·`hidden`(보이는 몫, 꺼져 있으면 0), `offset_ms`, `auto_offset`, `green_ms`(첫 BPM·SCROLL 1 에서 커버 아래를 뺀 레인 통과 시간. 점수의 `green_number` 와 같은 식이되 커버는 보이는 몫), `green`(스킨이 보이는 `NUMBER_DURATION_GREEN` 과 같은 식·자료형: 레인 전체 통과 시간을 단정밀도 hi-speed 로 구해 커버의 열린 몫(`1.0_f32 - cover`)을 곱해 Java 식으로 반올림한 정수 ms 의 3/5 를 정수 나눗셈으로 버림. `skin_host::play::covered_duration_ms`·`duration_green` 을 스킨과 함께 쓴다. 예: 768 ms 면 461 이 아니라 460). 플레이 화면이 프레임마다 설정을 보고(`PlayState::speed_log`, `play_trace::SpeedLog`) 바뀐 프레임의 곡 시각으로 적으므로 키·휠·턴테이블·자동 보정 어디서 바뀌어도 남는다. 시작값은 곡이 흐르기 시작한 첫 프레임의 값이다. 바뀜은 `TRACE_MAX_SPEED_CHANGES`(4,096)개까지만 담고, 넘으면 처음 2,048개와 마지막 2,048개를 남긴다(`SpeedLog` 가 플레이 중에 그만큼만 들고 있어 메모리도 묶인다). 이때 바뀜 목록만으로는 시작값에서 끝값에 이르지 않을 수 있고, 시작값·끝값은 늘 온전하다. 넘어도 trace 를 `skipped` 로 만들지 않는다 |
| 합계 | 판정 객체(일반 LN 끝 제외) 수 = 점수의 `total_notes`. state 1~5 의 수 = PG·GR·GD·BD·POOR 이고 끝까지 친 판은 state 0 이 없다. 중단한 판과 게이지가 비어 끝난 판은 도달하지 못한 노트를 POOR 로 바꾸지 않고 state 0·`err_us` 0 으로 둔다(`play_trace::TraceSource` 문서). 점수는 그 노트를 놓친 POOR 로 세므로, 서버는 판정된 노트(state 1~5)만 점수와 비교한다. 중단한 판은 state 0(일반 LN 끝 제외)의 수가 `left_unfinished` 가 POOR 로 더한 수와 같다. state 1~4 의 `err_us >= 0` 수와 `< 0` 수 = `epg/lpg … ebd/lbd`. 空POOR 수 = `empty_poor`. 9키(pop'n)는 노트를 남기는 BAD 와 세지 않는 見逃し POOR 때문에 이 등식이 맞지 않는다(아래 "남은 것") |
| 보관 | `play_trace` 표. 프레임 스레드는 판 요약에 필요한 최소 자료만 떼어(`TraceSource::of`: 노트 표식·空POOR·게이지 열·경계·템포 점·속도 로그) 저장 스레드로 넘기고, 노트 정렬·속도 로그 마무리·JSON 직렬화·`play_trace` 쓰기는 저장 스레드가 한다(`SaveWorker::keep_trace`, 판 행과 따로 쓰는 트랜잭션). 그 쓰기가 실패하면 판 행은 그대로 두고 trace 를 그 판의 `pending-runs` 파일에 옮겨 다음 DB 열기 때 다시 적는다(`trace of the run not stored: ...; kept on the side ...` 오류). DB 를 열 수 없어 판이 통째로 옆에 남을 때도 trace 는 저장 스레드가 만들어 그 파일의 `trace` 에 싣는다. 압축하지 않는다(새 의존성 없음). 지우는 규칙은 아직 없다 |
| 소급(W6, ADR Q7) | 이 기능 전에 남은 판(`play_trace` 행이 없고 리플레이 파일이 있는 `scorelog` 행)의 trace 를 리플레이를 다시 판정해 만든다(`apps/rbms-player/src/trace_backfill.rs`). 다시 판정은 고스트와 같은 경로다(`ghost::session_of_replay` 뒤 `PlaySession::play_out`, `docs/reference/record-target-pace.md` 4절). 영영 만들지 않는 판(표시가 지나간다): 제출 본문이 없거나 읽히지 않는 판, 리플레이 파일을 남기지 않은 판, 리플레이의 md5 가 행과 다른 판, 리플레이가 FRAME 시계인 판(`REPLAY_EVENT_CLOCK` 이 아님), 차트 파일의 sha256 이 행과 다른 판, 코스 스테이지(아래), 그리고 다시 판정한 결과가 행과 하나라도 다른 판: 램프, 판정 12칸(早/遅 포함, 空POOR 는 MISS 칸), 노트 수, 콤보, 게이지 종류·끝 값(비트 단위), 어시스트, 모드, LN 종류 키, 시드, 배치, 본문의 FAST·SLOW·EX·空POOR·`lntype`·모드, 규칙 세대 4 이상이면 `minbp`·`avgjudge`. 허용 오차는 없다. 지금만 못 만드는 판(표시가 지나가지 않는다, ADR W6): 리플레이 파일이 읽히지 않는 판, 차트 파일이 리플레이에 적힌 경로에도 라이브러리(`songdb.sqlite` 의 같은 md5)에도 없는 판, Esc 판의 탐색이 일한 시간 상한(20초)을 넘은 판. 한 실행에서는 한 번만 보고 다음 실행이나 라이브러리 스캔 뒤 다시 본다. Esc·앱 종료로 남긴 판: 행이 FAILED 인데 게이지가 비어 끝나지 않았으면 마지막 입력 시각부터 1 ms 씩 판정을 이어 가며 `left_unfinished` 로 셈한 결과가 행과 같아지는 첫 시각을 찾는다. 행의 `playtime` 은 차트 길이라 멈춘 시각이 아니고 리플레이에도 Esc 시각이 없다. 그 시각 뒤 노트가 하나 더 지나간 뒤에도 행과 같으면(게이지가 바닥에 붙어 놓친 노트가 게이지를 깎지 않은 판) 어디서 멈췄는지 알 수 없어 만들지 않는다. 찾은 판의 trace 는 도달하지 못한 노트를 state 0 으로 두고 `failed_at_ms` 는 null 이라 서버 검사가 그 노트를 POOR 로 센다(점수 행과 같은 셈). 탐색은 1,000 걸음(곡 1초)마다 멈춤·취소를 보고, 행이 지닌 판정·게이지와 같을 때만 기록(`PlayRecord`)을 셈하며, 걸음 상한 120만(곡 20분)·일한 시간 상한 20초를 둔다. 게이지가 비어 끝난 판은 엔진이 그 시각에 멈추므로 끝까지 판정한 결과 그대로이고 `failed_at_ms` 가 숫자다. 코스 스테이지(ADR W6): 리플레이에 코스 게이지·이월 콤보가 없어 같은 판을 재현할 수 없으므로 만들지 않는다. `scorelog` 에는 코스를 가리키는 열이 없다. 이 클라이언트는 2026-10-22 부터 스테이지 행을 그 스테이지가 돈 코스 게이지(class·exclass·exhardclass)로 적으므로(10.13절) 그 게이지로 적힌 행(다른 클라이언트에서 가져온 행도 같다)이 스테이지다. 그 전 빌드는 스테이지 행도 설정의 게이지로 적었으므로, 남아 있는 코스(`course_submission`)의 끝 시각까지 마지막으로 저장된 행이 그 코스의 차트 순서와 맞으면 그 행들도 스테이지로 보고(`course_stages`) 뺀다(DB 를 열 때 그 행들은 CLASS 로 옮겨진다. 10.13절). 남지 않은 코스(보낼 수 없던 코스)의 스테이지는 다시 판정이 행과 다르게 나와(코스 게이지 값과 이월 콤보) 대부분 빠지고, 남는 위험은 테스트 부채 3.86. 속도 옵션은 리플레이에 없어 제출 본문의 끝값(hispeed·constant·offset_ms·auto_offset)을 시작값과 끝값으로 싣고 바뀜은 비운다. 커버·리프트는 라이브와 같은 규칙(`effective_cover`·`effective_lift` 가 쓰는 `app_input::shade_shown`)으로 지금 설정의 ENABLE COVER·ENABLE LIFT 가 꺼져 있으면 0, 켜져 있으면 본문의 값이다. 본문은 HIDDEN 을 싣지 않아 0 이다(테스트 부채 3.83). 형식 필드는 바꾸지 않았다 |
| 소급 실행 | 앱이 뜬 뒤 `rbms-trace-backfill` 스레드가 30초 기다렸다가 오래된 판부터 16판(`BACKFILL_RUNS_PER_BATCH`)씩, 판마다 0.25초 쉬며 다시 판정한다. 결정·로딩·플레이·연습 화면에 있는 동안은 멈춰 기다린다. 대상은 시작할 때의 가장 새 판까지다(그 뒤 판은 스스로 trace 를 남긴다). 점수 DB 는 읽기 전용 연결로 읽고, 한 묶음이 끝나면 프레임 스레드가 `try_recv` 로 받아 저장 스레드에 넘긴다(`SaveWorker::keep_backfilled_traces` 에서 `ScoreDb::keep_backfilled_traces`, `BackfillBatch { traces, refused, from, through }`). 저장 스레드는 한 트랜잭션에 `INSERT OR IGNORE` 로 trace 를 적고(그 사이 trace 가 생긴 판은 자기 trace 를 지킨다), 영영 만들지 않을 판을 `trace_backfill_refused` 에 적고, `meta` 의 `trace_backfill_through` 가 그 묶음을 만들 때의 표시(`from`)와 같을 때만 표시를 `through` 로 올린다(그때 표시 아래의 `trace_backfill_refused` 행은 지운다). 그래서 쓰지 못한 묶음이 있으면 그 뒤 묶음은 표시를 올리지 못하고, 다음 시작이 쓰지 못한 묶음의 판부터 다시 본다. `through` 는 그 실행에서 처음 "지금만 못 만든" 판의 바로 앞까지다. 다시 시작하면 표시 다음 판부터 trace·거절 기록이 없는 판만 본다. 라이브러리 스캔이 끝나면 지난 실행이 "지금만 못 만든" 판을 남겼을 때 같은 속도로 다시 시작한다(스레드가 돌고 있으면 끝난 뒤). 모두 끝나면 채운 판이 있을 때 메시지 한 줄 `play traces filled in for N earlier runs from their replays` 를 띄우고, AUTO UPLOAD TRACE 가 켜져 있으면 저장 스레드가 그 쓰기를 끝낸 뒤(동기화가 돌고 있으면 끝난 뒤) 전체 기록 동기화를 한 번 띄워 10.9절의 trace 행 그대로 보낸다. 꺼져 있으면 만들어 남기기만 한다. 판마다 만들지 않은 까닭은 콘솔에 `run <id> given no trace: <why>`(지금만이면 `given no trace for now`)로 적는다 |
| 전송 | 10.9절의 trace 행 |
| 설정 | NETWORK 의 SCORES 묶음 `AUTO UPLOAD TRACE`(기본 켬, `network.auto_upload_trace`). 꺼도 trace 는 로컬에 남고 `pending` 으로 기다린다. 설명 문구(2026-10-20 W6 에서 고침)는 지금 사실만 말한다: 본인만 읽고, 웹 설정에서 공유를 켜면 서로 라이벌인(상대도 나를 라이벌로 둔) 플레이어만 구간 요약을 보며, 이 기기에 늘 남고, 예전 판은 리플레이가 그대로 재생될 때 만든다. 이 행이 들어가며 NETWORK 그룹이 한 화면에 들어가도록 SETTINGS SYNC 묶음의 세 줄을 SERVER AND ACCOUNT 묶음 끝으로 옮겼다 |
| 위치 | 만들기 `apps/rbms-player/src/play_trace.rs`, 남기기 `app_result.rs`(`record_run`, `trace_source`)·`save_worker.rs`(`keep_trace`, `keep_run_on_the_side`, `StoredRun`), 보내기 `record_sync.rs`(`send_traces`, `trace_upload_of`, `takes_traces`)·`app_ranking.rs`(`PlayedRun`, `write_live_outcome`), 와이어 `crates/rbms-ir/src/dto/trace.rs`·`http.rs`(`upload_trace`), 표 `crates/rbms-store/src/scoredb/schema.sql`·`sync.rs`(`trace_candidates`, `trace_body`, `put_trace_state`) |
| 테스트 | `play_trace/tests.rs`(일반·空POOR·중단·RANDOM·SCRATCH AUTO·CN·HCN 판의 합계, LN 종류, 템포, 속도 이력, 속도 바뀜·템포 점 상한), `skin_host/play/tests.rs`(`the_green_number_of_a_play_trace_is_the_one_the_skin_shows`), `save_worker/tests.rs`(저장 스레드가 trace 를 쓰고 그 전에는 판 행만 있음, DB 가 받지 않은 trace 가 옆 파일로 가서 다시 읽힘), `record_sync/tests.rs`(점수에 묶어 올림, 옛 서버, 설정 끔, 거절의 다시 보내기, 크기 상한, 결과 화면 판, 보낼 trace 가 있을 때만 `/health`, DB 읽기 실패는 `failed`), `app_result/record_tests.rs`(남긴 판마다 trace, `pending-runs`, 오토플레이, 결과 화면이 프레임 스레드에서 trace 를 쓰지 않음), 계약 `contract_requests.rs`(`trace-normal`·`trace-abandoned`·`trace-random`·`trace-scratch-auto`), `crates/rbms-store/src/scoredb/sync_tests.rs`, `crates/rbms-ir`(`upload_trace_*`, `dto::trace`), `crates/rbms-judge/src/tests.rs`(`an_empty_poor_on_a_judged_note_leaves_the_judged_timing_and_names_the_note`); 소급 `trace_backfill/tests.rs`(끝까지 친 판·Esc 판·Esc 전에 노트가 지나간 판·게이지가 빈 판·LN·CN·HCN·SCRATCH AUTO·RANDOM 시드 판을 다시 판정한 trace 가 그 판이 라이브로 남긴 trace 와 같음, 속도 이력과 Esc 판 게이지 열의 끝은 빼고. 꺼 둔 커버·리프트는 0. 게이지가 바닥에 붙은 채 남긴 Esc 판은 만들지 않음. 탐색이 취소를 봄. 행·본문·리플레이·차트가 다르면 만들지 않고 리플레이·차트가 없으면 지금만. 코스 스테이지와 코스 게이지 행은 빼고 그 뒤 판은 만듦. 차트가 없던 판은 다음 시작에 다시 봄. 쓰지 못한 묶음 뒤에는 표시가 오르지 않음. 스레드가 남기고 표시하고 동기화가 보냄, 설정 끔이면 보내지 않음, 빈 기록), `crates/rbms-store/src/scoredb/backfill_tests.rs`, 계약 `score-trace-normal`(`trace-normal` 의 짝 점수 본문, 같은 판에서 캡처)·`trace-bad`·`trace-long-note`·`trace-charge-note`·`trace-hell-charge-note`·`trace-failed`·`trace-empty-poor-exceeds-poor`·`trace-popn-9k` 와 짝 점수 파일 |
| 남은 것 | 9키: 판정 규칙은 그대로 두고 서버가 9키의 BAD·POOR 를 검사에서 뺀다(ADR 메인 결정). 9키 BAD 의 state 가 비는 문제는 테스트 부채 3.80. 로컬 `play_trace` 의 보관 상한. 소급의 남은 위험은 테스트 부채 3.82·3.83·3.85·3.86(3.84 는 해소) |

### 10.13 램프 등급(ASSIST EASY ~ ASSIST EXHARD, 2026-10-22)

SCRATCH AUTO 로 클리어한 판의 램프는 클리어한 게이지에 따라 다섯 가지로 갈린다. 램프 id(0~10)와 와이어 이름, 강등 규칙, 규칙 세대(`SCORE_RULE_VERSION` 4), 점수 제출 본문의 모양은 그대로이고, 램프 id 3 안에 등급(0~4)이 하나 더 붙는다. 조사는 [reference/clear-lamps.md](./reference/clear-lamps.md)(조사 당시 문구가 남아 있어 코드와 다른 곳은 이 절과 코드가 맞다), 결정은 [어시스트 등급 ADR](./acknowledge/2026-10-22-adr-assist-lamp-grade.md).

| 램프 | 이름 | 언제 |
| --- | --- | --- |
| id 2 | `ASSIST` | 넓힌 판정이나 TOTAL 직접 지정(어시스트 2)으로 클리어한 판. 등급 없음. 앱은 전에 이것을 `ASSIST EASY` 라 불렀고, 그 이름은 지금 등급 1 의 것이다 |
| id 3, 등급 0 | `LIGHT ASSIST` | SCRATCH AUTO(어시스트 1)로 ASSIST EASY 게이지에서 클리어한 판. 규칙 세대 2 미만이거나 어시스트 단계를 기록하지 않은 옛 행도 등급 없음 |
| id 3, 등급 1 | `ASSIST EASY` | EASY 게이지로 클리어 |
| id 3, 등급 2 | `ASSIST CLEAR` | NORMAL 게이지로 클리어. 코스 게이지 CLASS |
| id 3, 등급 3 | `ASSIST HARD` | HARD 게이지로 클리어. 코스 게이지 EXCLASS |
| id 3, 등급 4 | `ASSIST EXHARD` | EX-HARD·HAZARD 게이지로 클리어. 코스 게이지 EXHARDCLASS |

1. 등급은 램프 id 3 에만 있다. 값은 끝난 게이지(GAUGE AUTO SHIFT 가 옮겼으면 옮겨진 게이지)로 정한다: `rbms_judge::assist_grade(램프, 끝난 게이지)`, 열거형 `AssistGrade`(None·Easy·Clear·Hard·ExHard, id 0~4). 점수 기록 크레이트는 판정 크레이트에 기대지 않으므로 같은 표를 정수로 따로 가진다(`rbms_store::assist_grade_of`, `RecordLamp { clear, assist_grade }`). 서버도 표를 따로 가진다(`web/src/server/service/domain/score/assist-grade.ts`). 클라이언트 테스트 `format::tests::the_score_store_grades_every_stored_run_as_the_engine_does` 가 램프 11종 x 게이지 9종에서 엔진과 기록 쪽이 같은 답을 내는지 본다.
2. 비교는 (램프 id, 등급) 순서다. 같은 id 3 안에서 ASSIST HARD 는 ASSIST CLEAR 를 이기고 둘 다 EASY CLEAR(id 4) 아래다. 최고 기록 행(`score`)은 `assist_grade` 열(0~4, 기본 0)을 갖고, 램프 쌍이 오르면 `merge_best` 가 `clear`·`assist_grade`·`option`·`seed`·`gauge`·`assist` 를 그 판의 값으로 바꾼다(id 가 같고 등급만 올라도). `ScoreDb::best` 의 순서는 `clear DESC, assist_grade DESC, EX DESC, minbp ASC` 다. 선곡의 정렬 Clear 은 (램프, 등급) 순이고, CLEARED·NOT CLEARED 필터는 전과 같이 램프 id 2 이상이 기준이다.
3. 소급: DB 를 열 때마다 `refold_assist_grades` 가 각 `score` 행의 등급을 같은 램프의 `scorelog` 행 가운데 가장 높은 등급(동률이면 가장 먼저 친 판)으로 맞춘다. 등급이 0 보다 크면 그 판의 게이지·어시스트도 쓴다. 램프 id 3 이 아닌 행은 등급 0 으로 정리한다. 이미 맞는 행은 쓰지 않아 두 번째 열기는 아무것도 쓰지 않고, 훑는 범위는 램프 id 3 의 행뿐이다. 이전 빌드가 같은 DB 의 `score` 행을 다시 써서 등급이 0 이 돼도 다음 열기에서 복구된다. 새 열 `assist_grade` 는 기존 `ADDED_COLUMNS` 방식으로 더했고 스키마 세대는 1 그대로다.
4. 코스 스테이지: 코스의 스테이지는 언제나 코스 게이지(CLASS·EXCLASS·EXHARDCLASS)로 돈다. 2026-10-23 부터 시작 슬롯은 GAUGE 설정을 따른다(ASSIST EASY·EASY·NORMAL 은 CLASS, HARD 는 EXCLASS, EXHARD·HAZARD 는 EXHARDCLASS. 10.14절). 전에는 늘 CLASS 로 시작했다. 2026-10-22 의 수리(`242397f`) 전에는 결과 화면의 등급은 코스 게이지로 정하면서 저장·전송하는 게이지는 설정의 게이지였다. 그래서 설정이 EXHARD 인 채 SCRATCH AUTO 로 친 코스 스테이지가 결과 화면에는 ASSIST CLEAR, 기록과 서버에는 ASSIST EXHARD 로 남았다(설정이 ASSIST EASY 면 등급이 아예 없었다).
   - 지금은 `record_run` 이 `PlayRecord::finished_gauge` 를 그대로 저장한다(`FinishedPlay.gauge` 와 `PlayLog.gauge` 가 게이지 번호이고 코스 게이지는 6~8). 점수 본문의 `options.gauge` 는 `rbms_ir::mapping::ir_gauge_of_index` 로 `class`·`exclass`·`exhardclass` 를 싣고, 서버의 `deriveAssistGrade` 는 이 이름들을 2·3·4 로 읽는다. 기록 동기화의 다시 보내기도 저장된 게이지 번호로 같은 이름을 보낸다(`record_sync.rs`).
   - 이미 저장된 행은 DB 를 열 때 `put_course_stages_on_class` 가 옮긴다. 남아 있는 코스(`course_submission`)의 스테이지 행(`trace_backfill::course_stages`) 가운데 게이지 번호가 6 미만인 행을 CLASS 로 바꾸고 등급을 다시 접는다. 두 번째 열기는 쓰지 않는다. 남지 않은 코스의 스테이지는 알아낼 길이 없어 그대로다. 옮기면 알림 `N course stages stored on the gauge the settings chose are on the course gauge they ran on now` 가 한 번 뜬다.
   - 코스의 스테이지는 코스 기록과 별도로 곡 단위로도 최고 기록 규칙을 지난다(위 10.2절의 코스 행). 코스 기록의 등급: 2026-10-23 부터 코스 제출 본문이 끝난 게이지를 싣고(10.14절) 서버가 `course_best.assist_grade` 를 채운다([ADR](./acknowledge/2026-10-23-adr-course-submission-gauge.md)). 전에는 본문에 게이지가 없어 항상 0 이었다(테스트 부채 3.93).
5. 서버: `chart_best.assist_grade`·`course_best.assist_grade` 열과 인덱스 `chart_best_rank_grade_idx`(마이그레이션 `web/drizzle/0011_assist_grade.sql`), 순위 키 (clear, assist_grade, EX, BP). 순위·활동·목록 응답에 `assist_grade` 0~4 정수가 추가로 실린다(없으면 0이고 이전 클라이언트는 무시한다). 2026-10-22 에 0011 을 운영에 적용했고(web-db 실행 39693988962) 낡은 등급 행이 0 임을 확인했다(실행 39733088817. 등급이 매겨진 `chart_best` 398행). 낡은 행을 세고 고치는 액션은 `web-db.yml` 의 `assist-grade-backfill-count`, `assist-grade-backfill` 이다.
6. 표시: 앱의 결과 화면, 선곡 상세의 최고 기록 라벨, 기록 모달, 기록 목록 행, 내장 선곡 막대의 LED, IR 순위 패널(서버가 준 `assist_grade`. `skin_host/ir.rs` 의 램프 분포는 라벨 글자가 아니라 램프 id 로 센다). 기본 스킨은 선곡 정보창과 결과 화면이 확장 옵션 21110~21134 로 등급을 읽어 그린다([skin.md](./skin.md) 3.4.5절). 스킨이 읽는 램프 id(이미지 인덱스 370·371)와 선곡의 폴더 분포 11칸은 그대로 0~10 이다. 등급 라벨 색은 `3ad19f`(ASSIST EASY), `c4a796`(ASSIST CLEAR), `afb8ef`(ASSIST HARD), `ca76a9`(ASSIST EXHARD)이고 웹과 같다(`format.rs`, 테스트가 웹의 hex 와 맞춘다).
7. 남은 것: (a) 리플레이 파일 헤더(`replay.gauge`)와 AUTO UPLOAD REPLAY 로 올리는 리플레이는 설정의 게이지, 곧 재생이 시작하는 게이지를 적는다. 코스 스테이지의 리플레이는 코스 게이지와 이월한 게이지 값·콤보를 담지 않아 충실한 재생이 아니다(전과 같다. 테스트 부채 3.94). (b) 이미 서버에 올라간 코스 스테이지 행은 설정 게이지로 낸 등급을 서버가 그대로 갖고 있다. 다시 보내지 않으면 고쳐지지 않는다(테스트 부채 3.94). (c) 설정 화면의 JUDGE WIDTH·LN MARGIN 행 설명(`stage/settings/catalog.rs` 147·169줄)은 `the ASSIST lamp at best` 로 고쳤다.

### 10.14 코스 게이지(COURSE GAUGE, 시작 슬롯, 9칸 이월, ln·cn·hcn 제약. 2026-10-23)

결정은 [코스 게이지 ADR](./acknowledge/2026-10-23-adr-course-gauge-modes.md), [시작 슬롯과 LN 제약 ADR](./acknowledge/2026-10-23-adr-course-stage-start-and-ln.md), [제출 게이지 ADR](./acknowledge/2026-10-23-adr-course-submission-gauge.md). 사용자가 정한 것은 모드 셋(LR2 기준, 설정에서 변경)과 IIDX 구현이고, 아래 수치의 세부와 시작 슬롯·이월·LN 강제는 메인 판단이라 사용자가 직접 확인하지 않았다.

1. 설정: SETTINGS > JUDGE 의 GAUGE 구역에서 GAUGE SET 바로 뒤 행 `COURSE GAUGE`(`judge.course_gauge`). 값은 `LR2`(기본) / `BEATORAJA` / `IIDX` 이고, 저장 파일에 없거나 모르는 값이면 LR2 다. 설정 스키마 세대는 6 그대로이고 계정 설정 묶음(`settings-blob.json`)에 `course_gauge` 한 줄이 더해졌다.
2. 코스는 이 설정의 표로만 돈다.

| COURSE GAUGE | 코스 스테이지가 쓰는 게이지 표 |
| --- | --- |
| LR2 | LR2 세트(차트 모드와 무관) |
| BEATORAJA | 차트 모드의 표: 5키·10키 FIVEKEYS, 9키 PMS, 24키 KEYBOARD, 그 밖 SEVENKEYS |
| IIDX | IIDX 세트(차트 모드와 무관, 5·9·24키에도 같은 수치) |

3. 코스가 선언한 게이지 제약(`gauge_lr2` 등)은 막대 배지에만 남고 표를 정하지 않는다. 설정이 제약보다 우선하며, 제약이 없는 사용자 코스 파일도 기본은 LR2 표다. 설정 GAUGE SET(AUTO·LR2)은 단일 곡 전용이 되었다. 원본은 제약이 표를 정한다(`GrooveGauge.java:128-147`). 그래서 BEATORAJA 에서는 발광·Stella 코스(모두 `gauge_lr2` 제약)가 LR2 가 아니라 7키 CLASS 로 돈다. 코스 결과 화면에는 GAUGE 행 바로 뒤에 `GAUGE RULE` 행이 있다.
4. 규칙은 코스를 시작할 때 한 번 읽어 고정한다(`CourseFacts.gauge_rule`, `lib.rs` 의 `start_course`). 코스 도중 설정을 바꿔도 이어지는 스테이지는 시작 때의 규칙으로 판정된다(`app_play.rs` 의 `stage_judge_setup`). 제출 본문이 규칙을 한 값(`course_gauge`)으로 말하므로 모든 스테이지가 그 규칙으로 판정돼야 본문과 판정이 맞기 때문이다.
5. IIDX 표(출처는 iidx.org 한 곳, 다른 공식 자료는 열리지 않았다): 시작 100, 0% 에서 실패(클리어는 0 초과. iidx.org 의 gauge compendium 은 0%, beginner 는 2% 미만으로 서로 다르며 사용자가 0% 를 골랐다).

| 슬롯 | 값 |
| --- | --- |
| Class (DAN) | PG +0.16, GR +0.16, GD +0.04, BD -1.5, POOR -2.5, 빈 POOR -1.5. 게이지가 30% 이하이면 BAD·POOR·빈 POOR 감소량을 절반(-0.75, -1.25, -0.75). TOTAL 보정 없음 |
| ExClass (EX-DAN) | 같은 증가량, BD -3.0, POOR -5.0, 빈 POOR -3.0. 30% 보정 없음 |
| ExHardClass | EX-DAN 값의 복사(코스 상한이 ExClass 라 코스에서는 닿지 않는다) |
| AssistEasy ~ Hazard | 7키 세트의 복사(코스에서 선택되지 않는 자리) |

   "30% 이하"는 새 열거형 `GutsEdge::AtOrBelow` 로만 표현한다. 기존 모든 행은 원본대로 엄격 미만(`Below`)이라 기존 LR2·beatoraja 판정은 수치·화소 모두 그대로다. 연습 패널의 GAUGE CATEGORY 순환에도 IIDX 가 들어갔다(9.1절).
6. 시작 슬롯은 사용자 GAUGE 설정을 따른다: ASSIST EASY·EASY·NORMAL 은 CLASS(DAN), HARD 는 EXCLASS, EXHARD·HAZARD 는 EXHARDCLASS. 고른 표의 상한(`GaugeSetId::course_ceiling`)으로 자르므로 IIDX 표는 EXCLASS 까지다. BEST CLEAR·SELECT TO UNDER 의 자동 이동도 이 상한을 넘지 않는다. 코스를 시작할 때의 첫 값은 그 슬롯의 `init`(모든 표의 코스 슬롯이 100)이다. 전에는 항상 CLASS 로 시작해서 GAUGE 설정이 코스에서 의미가 없었다. SCRATCH AUTO 로 HARD 설정의 코스를 치면 EXCLASS 로 달려 어시스트 등급 3 이 된다.
7. 곡 사이에 9칸을 모두 이월한다. 스테이지가 끝나면 아홉 게이지의 판 끝 값(`PlaySession::gauge_values`)을 `CourseRun.carry_gauges` 에 담고(`fold_course_stage`), 다음 스테이지는 `SessionOptions.carried_gauges` 로 아홉 슬롯 전부를 그 값으로 시작한다(시작 값은 `carried[시작 슬롯]`, 분석 seek 뒤에도 유지, 0 이 된 슬롯은 0). 전에는 선택 게이지의 끝 값 하나만 CLASS 로 이월했다. `carry_gauge`(한 값)는 코스 결과의 GAUGE 행과 제출 `gauge_value` 용으로 남는다.
8. 코스 제약 `ln`·`cn`·`hcn` 은 그 코스의 모든 스테이지 판정에 LN MODE 를 LN·CN·HCN 으로 강제한다(`CourseOverrides::ln_mode_for`). 설정 파일의 LN MODE 는 바꾸지 않고(RANDOM·속도 제약처럼 복사본에만 적용) 코스가 끝나면 플레이어의 값 그대로다. CN·HCN 설정의 사용자가 발광·Stella·GENOSIDE 코스(모두 `ln` 제약)를 치면 이제 LN 으로 친다. 코스 제출 최상위 `lntype` 은 LN 제약이 있으면 그 값(0 LN, 1 CN, 2 HCN), 없으면 첫 스테이지가 실제로 친 값이다. NO GOOD·NO GREAT 는 대상 코스가 없어 배지만 있고, DP MIRROR 코스의 FLIP 은 플레이가 `LaneOption` 을 적용하지 않아 지키지 못한다(테스트 부채).
9. 코스 제출 본문에 생략 가능한 세 필드가 더해졌다: `course_gauge`(`LR2`·`BEATORAJA`·`IIDX`), `gauge`(끝난 게이지의 `GaugeType` 이름. `Class`·`ExClass`·`ExHardClass` 등), `stages`(친 순서대로 스테이지마다 `gauge`·`random`(코스 제약이 좁힌 뒤의 실제 값)·`lntype`). 값은 각 스테이지 자신의 점수 본문 `options` 에서 복사하고, 결과 진입·앱 종료로 끊김·코스 도중 Esc 세 경로가 같다. 서버는 이것으로 `course_best.assist_grade` 를 채우고 단위 인정(미러·LN 제약 준수)을 판정한다.
10. 구현: `crates/rbms-judge/src/{gauge_tables,gauge,data}.rs`·`data/gauge.ron`("IIDX" 키), `crates/rbms-config/src/{judge,schema,settings}.rs`, `apps/rbms-player/src/{course_ui,app_play,app_result,lib,practice,course_ir}.rs`, `apps/rbms-player/src/stage/{settings/catalog,course_result}.rs`, `crates/rbms-play/src/{lib,session}.rs`, `crates/rbms-course/src/run.rs`. 시험: `gauge_tables::tests::{the_iidx_dan_gauges_pin_their_published_values, only_the_iidx_dan_gauge_softens_at_or_below_its_band, the_iidx_set_tops_out_at_exclass}`, `course_ui::tests::{the_course_gauge_setting_outranks_a_gauge_constraint, a_hard_player_starts_a_course_on_exclass, a_later_stage_starts_on_every_gauge_the_stage_before_left, the_long_note_constraints_each_force_their_flavour}`, `record_tests::course_gauge::{an_ln_course_plays_long_notes_whatever_the_ln_mode_setting, the_next_stage_starts_on_every_gauge_the_last_one_left}`.
11. 확인하지 못한 것: IIDX 의 내부 단위·반올림(실제 IIDX 와 다를 수 있다), LR2 본체의 시작값·실패선, 차트 자체의 `#LNMODE` 가 LN 종류를 고정한 곡이 LN 제약 코스에 있을 때 서버가 단위를 인정하는지(GENOSIDE·Stella 곡의 `#LNMODE` 사용 여부).

### 10.15 리플레이의 배치로 치기(PLAY THIS LAYOUT, 2026-10-24)

리플레이를 보다가, 또는 선곡의 기록에서, 그 리플레이의 **배치(RANDOM 계열 옵션과 시드)** 를 빌려 내가 새 판을 친다(사용자 요청 "리플레이를 볼때 해당 리플레이의 배치를 활용하여 나도 플레이하는 모드"). 두 사람이 동시에 치는 배틀이 아니다. 결정과 근거는 [배치 플레이 ADR](./acknowledge/2026-10-24-adr-layout-play.md).

| 항목 | 내용 |
| --- | --- |
| 진입 | (1) 리플레이를 보는 중 키 `J`(키 설정 Replay 그룹의 `ReplayPlayLayout`, 기본 `J`, 키 설정에서 바꾼다. 보는 중인 판 자리에 새 판이 로딩된다). (2) 선곡의 기록 모달: 리플레이가 있는 기록에만 PLAY REPLAY 와 CLOSE 사이에 PLAY THIS LAYOUT 단추(키 `J`, 마우스). (3) IR 랭킹 패널과 랭킹 모드(16절): 줄에서 `J` 로 그 줄의 리플레이를 내려받아 시작. 파란 건반을 쥐고 REPLAY 를 누르는 길은 없다 |
| 빌리는 것 | 옵션(RANDOM 계열)과 시드만. 게이지·SCRATCH AUTO·LN MODE·판정 설정은 내 것이다. 판이 도는 동안 설정의 RANDOM 만 빌린 값이고 판 도중 저장돼도 내 값이 저장된다 |
| 거절 | 코스·연습·오토플레이(출하 기본값이 AUTOPLAY 켬이다)는 `only a run played by hand ... can borrow one`. 다른 차트(md5 대소문자 무시). 모르는 옵션이거나 CONVERGE. 시드가 0 이고 OFF 가 아님. 이 기기 기록이 아닌 리플레이가 OFF 로 보임(서버가 CONVERGE·옵션 없음을 OFF 로 바꿔 보내므로 배치를 알 수 없다. `--replay` 로 연 기록 없는 로컬 파일도 같다). 거절은 화면을 그대로 두고 경고 알림만 낸다. OFF 와 DP 는 허용 |
| 기록 | 일반 판과 같이 저장·전송한다(표지 없음). 점수·최고 기록 의미는 바뀌지 않는다 |
| 유지와 버림 | 같은 배치 재시도(파란 건반 + START, R·7번 키, 실패 직후 SELECT)에서는 유지, 선곡·다음 곡·다른 배치·시드 없는 재시도(5번 키, 내장 R)에서는 버리고 내 RANDOM 으로 돌아간다 |
| 고스트 | 로컬 리플레이에서 빌린 판은 이번 판의 TARGET 이 그 리플레이(이름 `REPLAY <기록 시각>`)가 된다. 결과 화면의 TARGET 도 같다. 설정의 TARGET 행은 건드리지 않는다. 다시 판정할 수 없으면(FRAME 시계 리플레이, 노트 수·EX 불일치) 비례 페이스로 물러나고 콘솔에 이유를 남긴다. 서버 리플레이는 고스트가 없다 |
| 알림 | 빌릴 때 Info `layout of <출처> (<옵션>)`. 노트 위치가 LN 처리에 따라 달라지는 옵션(S-RANDOM·H-RANDOM·ALL-SCRATCH)을 정의 안 된 LN 이 있는 차트에서 빌리면 LN MODE 와 상관없이 경고만 낸다 |
| 구현·시험 | `apps/rbms-player/src/{borrowed_layout,app_play,app_result,ghost,target}.rs`, `stage/play/mod.rs`, `stage/select/layout_play.rs`, 키 `keyconfig.rs`. 배치 순열 골든은 `crates/rbms-chart/src/shuffle.rs` 의 시험 모듈(옵션 8개 x 7K·14K·9K) |

알려진 위험: LOADING 에서 Esc 로 취소하면 다음 `load` 까지 설정 화면에 빌린 RANDOM 이 보일 수 있다. 실창에서 `J` 키를 눌러 본 확인은 없다(QA 3.23, 테스트 부채 3.119).

### 10.16 LAYOUT PICK 배치 지정(2026-10-29)

선곡에서 차트의 레인 배치를 직접 써 넣고, 그 배치를 적용한 실제 노트를 미리 본 뒤, 그 배치로 친다(사용자 요청 "시작 전에 배치를 내가 정하고 미리보기를 하고 그 배치대로 플레이 가능한 기능"). 계속 RANDOM 을 뽑다 보면 나올 배치를 편하게 얻는 기능이라 기록은 RANDOM 판이다. 결정과 근거는 [LAYOUT PICK ADR](./acknowledge/2026-10-29-adr-layout-pick.md).

| 항목 | 내용 |
| --- | --- |
| 여는 키 | 선곡에서 `Y`(키 설정의 Browser 그룹 LAYOUT PICK / 배치 지정 / 配置指定, 설정 파일 키 `layout_pick`, 패드 지정 가능). 커서가 차트 막대일 때만 열린다. 폴더·코스·라이브러리에 없는 차트에서는 열리지 않고 알림만 낸다. 검색어를 치는 중에 누르면 검색을 끝내고 연다. 선곡 위에 열리는 독립 화면이라 옵션 오버레이가 위에 열리지 않는다 |
| 문자열 규칙 | i 번째 숫자는 그 쪽의 왼쪽 i 번째 건반에 오는 원래 레인 번호이고 쪽마다 1부터 센다(beatoraja 의 lane order 표시와 같다). `1234567` 은 OFF 모양, `7654321` 은 MIRROR 모양이다. 스크래치는 고정이라 문자열에 넣지 않는다. DP 는 1P·2P 두 줄이며 둘 다 지정한다(기본은 둘 다 순서대로). 9키는 9자리다. 5키·7키·9키·10키·14키를 지원하고 24키는 지원하지 않는다 |
| 입력 | 숫자 1~9(넘패드 포함)는 현재 쪽의 다음 자리에 들어간다. 그 차트 모드의 레인 키는 그 레인이 속한 쪽의 그 쪽 번호로 들어가고 현재 쪽도 그쪽으로 바뀐다(패드 레인도 같다. 마지막에 로드한 모드가 차트 모드와 같을 때만 받는다). 남은 자리는 쓰지 않은 번호를 오름차순으로 채우므로 키 하나를 칠 때마다 배치 전체가 완성된 상태가 된다. 다 채운 줄에 다시 치면 그 번호로 새로 시작한다. 이미 넣은 번호나 범위 밖 번호는 거절하고 2초 알린다. `Backspace` 는 한 자리, `Delete` 는 그 쪽 입력 전부를 지우고 `0` 은 순서대로(OFF 모양) 채운다. `Tab`(패드 SELECT)은 쪽을 바꾼다 |
| 미리보기 | 배치를 적용한 실제 노트를 내장 플레이필드로 그린다. 한 화면에 네 마디(하이스피드 0.25)가 보이고 마디 시작은 마디선이다. 각 레인 아래에 그 레인에 오는 원래 레인 번호를 쓰고, 입력한 칸은 밝게 자동으로 채운 칸은 흐리게 보인다. 오른쪽 아래에 `마디 n / 전체`. SCRATCH AUTO 가 켜져 있으면 턴테이블 노트는 초록이다. 시드를 찾기 전에도 입력한 배치로 바로 그린다 |
| 미리보기 조작 | `↑` 다음 마디, `↓` 이전 마디, `PageUp`/`PageDown` 네 마디, `Home`/`End` 처음/끝. 스크래치 정방향 키는 다음 마디, 역방향 키는 이전 마디. 휠은 한 줄에 한 마디(사용자 쪽으로 굴리면 앞, 곧 이전 마디) |
| 시작 | `Enter`, 키보드 START(기본 `A`), 패드 START. 판은 RANDOM + 역탐색한 시드로 시작한다. 시드는 1부터 오름차순의 첫 시드라 같은 배치는 늘 같은 시드다(예: 7키 `3517246` 은 4761). 이 화면 자리에 판이 서고 아래의 선곡은 그대로다 |
| 시드를 찾는 동안 | 차트 읽기와 시드 탐색은 각자 이름 있는 스레드에서 돈다. 배치를 바꾸면 진행 중인 탐색을 취소하고 다시 찾는다. 상태줄에 `RANDOM 시드를 찾는 중`, 찾으면 `RANDOM 시드 n로 플레이`. 시드는 5키·7키·9키 2^23, 10키 2^21, 14키 2^29 까지 찾는다(전부 측정한 최댓값을 넘는다). 릴리스 14키 최악은 약 2.2초로 추정하며 디버그 빌드는 수십 초일 수 있다 |
| 시작 거절(화면 문구, 세 언어) | 순서대로: AUTOPLAY 사용 중(미리보기는 된다. 출하 기본값이 AUTOPLAY 켬이다), 차트를 읽는 중 또는 읽기 실패, 라이브러리를 스캔한 뒤 차트 파일이 바뀜(md5 나 모드가 실제 파일과 다름), 24키, 10키에서 RANDOM 으로 만들 수 없는 쌍(예: `12345` / `51234`. 14,400쌍 중 1,800쌍), 시드를 아직 못 찾음. 그 밖의 거절은 `borrow_layout` 이 낸 것이 화면에 그대로 보인다(코스·연습은 `NotFreePlay`) |
| 기록 | 일반 RANDOM 판과 같이 저장·전송한다(표지 없음). 기록 행·점수 제출 본문·저장 리플레이의 옵션과 시드가 RANDOM + 찾은 시드이므로 로컬 DB, 리플레이, IR 이 모두 RANDOM + 그 시드로 남고 서버·웹·리플레이 형식은 바뀌지 않았다. 시작 알림 문구는 `지정한 배치 3517246의 배치 (RANDOM)` 꼴이다(기존 배치 빌리기의 알림 문구를 쓴다) |
| 유지와 버림 | 배치 빌리기(10.15절)의 규칙 그대로다. 같은 배치 재시도(파란 건반 + START, R·7번 키, 실패 직후 SELECT)에서는 유지하고 선곡으로 돌아오면 버려 내 RANDOM 으로 돌아간다 |
| 기억 | 앱이 켜져 있는 동안 모드별로 마지막에 쓴 배치를 기억하고 다음에 열 때 그 배치로 채워 연다. 화면을 나갈 때(시작 포함) 저장한다. 설정 파일에는 쓰지 않으므로 앱을 다시 켜면 기본(순서대로)이다 |
| 구현·시험 | `crates/rbms-chart/src/shuffle.rs`(`random_seed_for`, `random_seed_limit`, `permutation_of_layout`, `layout_of_permutation`, `parse_layout_row`, `layout_row_text`, `is_layout_row`, `apply_lane_permutation`), `apps/rbms-player/src/stage/layout_pick.rs`(화면. `Stage::LayoutPick`, `AppShared.picked_layouts`), `text/menus/layout_pick.rs`(세 언어 문구), `stage/select/mod.rs`(`open_layout_pick`), `keyconfig.rs`(`Shortcut::LayoutPick`). 시험은 `shuffle.rs` 의 시드 역탐색 시험들(보고서의 알려진 시드, 5키 전 순열, 14키·10키 쌍, 불가능 쌍, 취소·상한)과 `stage/layout_pick/tests.rs`, `keyconfig_tests.rs` 의 `the_layout_pick_key_ships_on_a_free_key_and_an_earlier_file_reads_as_it_did` |

원본과 다른 점: 원본(beatoraja)에는 이 화면이 없다. 문자열 표기만 원본의 lane order 표시와 같다.

알려진 위험: 실창에서 화면을 본 적이 없다(헤더 글줄 위치, 필드와 레인 번호 위치, 일본어·한국어 안내 줄이 1280 폭을 넘는지. QA 3.25 V1~V5, 테스트 부채 3.130~3.132). 시작 알림 문구가 어색하다. 미리보기 배치를 바꿀 때마다 차트 모델을 복제하는 비용을 대형 차트에서 재지 않았다. 선곡 BGM·미리듣기가 이 화면에 들어오고 나갈 때의 동작을 확인하지 않았다(PLAY 메뉴처럼 Open 으로 열리는 다른 화면과 같을 것으로 본다). BMS 파일은 늘 14키로 판별되어 10키는 bmson 차트에서만 나온다.

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

플레이 중의 값(점수 차, 그래프의 TARGET 막대가 "지금까지" 서 있는 곳)은 LOCAL BEST 와 이 기기에서 친 IR BEST 에서 그 기록 판의 노트별 판정을 따르고, 나머지 목표는 최종 EX 를 판정된 노트 수에 비례시킨 값이다(11.7, 2026-10-19).

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

기록 판을 따르는 진행 중 값(11.7)의 위치와 테스트는 [record-target-pace.md](reference/record-target-pace.md) 5절·7절에 있다.

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

### 11.7 진행 중 값: 기록 판의 노트별 판정(2026-10-19)

> 사용자 요청(2026-10-19): MY BEST·LOCAL BEST 같은 기록 목표는 퍼센트가 아니라 그 기록 판의 노트 하나하나, 리플레이 그대로를 기준으로. 판정을 얻을 수 없으면 지금처럼 비례로 물러난다. 결정은 [ADR](acknowledge/2026-10-19-adr-record-target-follows-its-run.md).

판이 노트 k 개를 판정했을 때 목표가 서 있는 값:

| 목표(실제로 쓰는 것) | 진행 중 값 |
| --- | --- |
| LOCAL BEST | 그 최고 기록 판이 노트 k 개를 판정했을 때의 EX |
| IR BEST(이 기기에서 친 같은 판이 있을 때) | 그 판이 노트 k 개를 판정했을 때의 EX. 같은 판은 같은 차트·LN 종류, 같은 초에 친, 같은 EX 의 로컬 기록이다 |
| IR BEST(다른 기기에서 친 판), RIVAL, MAX, RATE A- ~ MAX-, RANK NEXT, RATE CUSTOM | 최종 EX x k / 전체 노트 수(내림). 지금까지와 같다 |

- k 는 시각이 아니라 지금까지 판정된 노트 수다(원본 `passnotes`). 空POOR 는 노트를 소비하지 않으므로 세지 않는다.
- 스킨의 BEST 쪽 숫자(최고 기록 대비 점수 차 152/172, rate 112, Lua `rate_best`)도 같은 방법으로 로컬 최고 판을 따른다. 원본과 같다. 결과 화면의 값, BEST·TARGET 타이머, 내장 HUD 그래프의 BEST 선은 최종 값 그대로다.
- 노트별 판정은 저장된 리플레이를 차트를 읽는 동안 다시 판정해 얻는다. 로딩 화면(스킨이 그리면 스킨의 로딩 상태)이 이것을 기다리므로 판 도중 값이 바뀌지 않는다. 같은 EX 의 최고 기록이 여럿이면 그 EX 를 처음 낸 판, 그 판에 리플레이가 없으면 다음 같은 EX 판이다.
- 비례로 물러나는 경우: 리플레이가 없음(AUTO REPLAY 꺼짐, 저장 실패, 정리됨), 리플레이가 다른 차트, 다시 판정한 노트 수나 PGREAT·GREAT 수가 기록과 다름(SCRATCH AUTO 로 친 최고를 SCRATCH AUTO 없이 칠 때, 옛 프레임 시계 리플레이, 코스 스테이지 판, 긴 노트를 쥔 채 Esc 로 끝난 판), 연습. 화면은 물러났다고 따로 표시하지 않는다.

## 12. 선곡의 옵션 패널 1~3

스킨이 그리는 선곡에서 키를 누르고 있는 동안 올라오는 세 패널이다. 내장 선곡(스킨 없음)에는 패널이 없고, 같은 값을 옵션 오버레이(F1, 12.5절)와 PLAY 메뉴(7절)·SETTINGS 가 고친다. 패널은 설정 필드를 새로 만들지 않고 기존 값을 고친다. 2026-10-22 에 두 가지를 바꿨다. 원본(beatoraja)에서 이 플레이어에 대응하는 설정이 없어 아무것도 하지 않던 칸에 SCRATCH AUTO·KEEP GREEN NUMBER·LANE COVER·LIFT·HIDDEN+·JUDGE ALGORITHM 을 배정했고, 기본 스킨의 패널을 IIDX 옵션 화면 식으로 다시 그렸다([skin.md](./skin.md) 3.4.5절). 2026-10-23 부터 이 재배정은 **스킨이 선곡 문서 헤더에 `rbms_extensions`(정수 1 이상)를 선언했을 때만** 적용된다. 선언이 없는 외부 스킨은 원본 키 표를 쓴다(12.2절, 12.7절, [패널 키 opt-in ADR](./acknowledge/2026-10-23-adr-panel-keys-opt-in.md)). 조사는 [reference/option-panels.md](./reference/option-panels.md), 결정은 [옵션 패널 ADR](./acknowledge/2026-10-22-adr-option-panel-layout.md)와 [확장 id ADR](./acknowledge/2026-10-22-adr-rbms-extension-ids.md).

### 12.1 여는 길

| 누르고 있는 것 | 패널 |
| --- | --- |
| START(기본 `A`, 패드의 START) | 1 PLAY OPTIONS |
| SELECT(기본 `W`, 패드의 SELECT) | 2 ASSIST OPTIONS |
| START 와 SELECT 를 함께, 또는 상세 옵션 키(기본 숫자 `5`) | 3 DETAIL OPTIONS |

누르고 있는 동안만 열려 있고 놓으면 닫힌다. 키로 값이 바뀌면 OptionChange 소리가 나고, 값은 패널이 닫힐 때(또는 선곡을 떠날 때) 설정 파일에 저장된다.

### 12.2 키 배치

아래 첫 표와 "원본과 바꾼 칸" 표는 **`rbms_extensions` 를 선언한 스킨(기본 스킨)** 의 배치다. 선언이 없는 스킨의 배치는 이 절 끝의 "선언 없는 스킨" 표를 본다. 건반 번호는 왼쪽부터 1~7이다(키 인덱스 0~6). 턴테이블은 스크래치 레인의 두 키이고(7키 기본: 정방향 `LSHIFT`, 역방향 `LCTRL`. 5.4절) 키 인덱스 7(정방향)·8(역방향)이다. 9키(pop'n)에는 턴테이블이 없어 8·9번 버튼이 그 자리다. 표의 값은 설정의 같은 값을 고친다. 아래 "바꾼 칸"의 것을 빼고는 beatoraja 의 `MusicSelectKeyProperty` 와 같다.

| 키 | 패널 1 (START) | 패널 2 (SELECT) | 패널 3 (START + SELECT) |
| --- | --- | --- | --- |
| 1 | RANDOM 다음 | EX JUDGE(읽기 전용: 눌러도 바뀌지 않는다) | BGA 켬·끔 |
| 2 | RANDOM 이전 | CONSTANT 켬·끔 | GAUGE AUTO SHIFT 다음 값 |
| 3 | GAUGE 다음 | SCRATCH AUTO 켬·끔 | AUTO ADJUST 켬·끔 |
| 4 | GAUGE 이전 | LEGACY NOTE 켬·끔 | NOTE DISPLAY TIME 줄임(12.3절) |
| 5 | HI-SPEED FIX 다음 | KEEP GREEN NUMBER 켬·끔 | JUDGE TIMING -1 ms |
| 6 | HI-SPEED FIX 이전 | LANE COVER 켬·끔 | NOTE DISPLAY TIME 늘림(12.3절) |
| 7 | RANDOM 다음 | LIFT 켬·끔 | JUDGE TIMING +1 ms |
| 턴테이블 정방향 | TARGET 이동(방향키와 휠도 같다) | HIDDEN+ 켬·끔 | JUDGE ALGORITHM 이전 |
| 턴테이블 역방향 | TARGET 이동 | HIDDEN+ 켬·끔 | JUDGE ALGORITHM 다음 |

원본과 바꾼 칸(`stage/select/keys.rs` 의 테스트 `CHANGED_SLOTS` 가 이 목록 밖의 칸이 원본과 같음을 고정한다. 확장을 선언한 스킨의 표다)

| 칸 | 원본 | 지금 | 까닭 |
| --- | --- | --- | --- |
| 패널 1, 4번 | DP OPTION 다음 | GAUGE 이전 | 이 플레이어에는 DP OPTION 을 적용하는 곳이 없다. 값(`play.lane_option`)과 설정 행, 스킨 이벤트 54 와 인덱스 54 의 응답은 그대로다 |
| 패널 1, 6번 | 2P RANDOM 이전 | HI-SPEED FIX 이전 | 1P·2P 가 RANDOM 값 하나를 같이 쓰므로 2번 키와 같은 일이었다 |
| 패널 1, 7번 | 2P RANDOM 다음 | 그대로(RANDOM 다음) | HI-SPEED FIX 이전이 6번에 생겨 비울 까닭이 없다. 동작하고, 원본·ModernChic 의 7번 라벨과 어긋나지 않는다 |
| 패널 2, 3번 | JUDGE AREA | SCRATCH AUTO | 대응하는 설정이 없다 |
| 패널 2, 5번 | MARK NOTE | KEEP GREEN NUMBER | 같다 |
| 패널 2, 6번 | BPM GUIDE | LANE COVER 켬·끔 | 같다 |
| 패널 2, 7번 | NO MINE | LIFT 켬·끔 | 같다 |
| 패널 2, 턴테이블 | 없음 | HIDDEN+ 켬·끔 | 어느 방향이든 한 번 뒤집는다 |
| 패널 3, 턴테이블 | 없음 | JUDGE ALGORITHM 이전(정방향)·다음(역방향) | 패널 1 의 TARGET 과 같은 방향 관례 |

원본 표를 쓰는 선언 없는 스킨(`REFERENCE_BEAT_TABLE`·`REFERENCE_POPN_TABLE`. 원본 `MusicSelectKeyProperty` 에서 `NEXT_REPLAY` 만 뺀 것)은 위 표에서 아래 칸만 다르다. 나머지 칸은 같다.

| 칸 | 선언 없는 스킨(원본 역할) |
| --- | --- |
| 패널 1, 4번 | DP OPTION 다음. 값(`play.lane_option`)은 저장되지만 플레이에 적용되지 않는 기존 차이는 그대로다 |
| 패널 1, 6번 | RANDOM 이전(원본의 2P RANDOM 이전. 1P·2P 가 한 값을 쓴다) |
| 패널 1, 7번 | RANDOM 다음 |
| 패널 2, 3·5·6·7번 | JUDGE AREA·MARK NOTE·BPM GUIDE·NO MINE. 대응하는 설정이 없어 **죽은 칸**이다: 패널이 누름을 가져가고 아무 일도, 소리도 없다 |
| 패널 2·3 의 턴테이블 | 역할 없음(패널 1 의 턴테이블은 TARGET) |

- 선언 없는 스킨에서는 SCRATCH AUTO·KEEP GREEN NUMBER·LANE COVER·LIFT·HIDDEN+·JUDGE ALGORITHM 을 이 키로 바꾸지 않으므로, 화면에 변화가 없는데 OptionChange 소리와 함께 저장 설정이 바뀌는 일(다음 판이 SCRATCH AUTO 로 시작해 어시스트 램프가 되는 일)이 없다. 키 안내(`H`)의 패널 줄 중 `SELECT + TURNTABLE = HIDDEN+ ON/OFF`·`START+SELECT + TURNTABLE = JUDGE ALGORITHM` 두 줄은 재배정 표일 때만 보인다.
- 어느 표를 쓰는지는 선곡을 그리는 문서의 선언으로 정한다(`KeyLayout::of_browser`: 모드의 턴테이블 유무 x 선언). 선곡 문서를 아직 읽는 중이면 원본 표이고(그동안 스킨 선곡의 입력이 돌지 않는다), 스킨 없는 내장 선곡의 목록 역할은 두 표가 같다. 기본 스킨은 `lua/props.lua` 의 `props.rbms_extensions = 1` 을 `lua/select/main.lua` 의 `screen.header()` 에 실어 보낸다([skin.md](./skin.md) 1.4절).
- 확장 id 자체(켬끔 값, 어시스트 등급, 닿지 않는 키. 이벤트 21000~21099, 불린 21100~21199)는 선언과 상관없이 모든 스킨에 답한다. 선언은 키 재배정만 가른다.

- 키 표에서 `NEXT_REPLAY` 는 뺐다. 원본은 6번 키에 두지만 여기서는 6번 키가 즐겨찾기만 하고, 리플레이 슬롯은 숫자 `4` 로만 넘긴다.
- BGA 는 켬·끔 둘뿐이라(`display.bga`) 인덱스 72 는 0(ON) 또는 2(OFF)만 답하고, 원본의 AUTOPLAY ONLY(1)는 저장될 수 없어 나오지 않는다.
- HI-SPEED FIX·LANE COVER·LIFT·HIDDEN+·JUDGE ALGORITHM 은 그 플레이의 설정이라, 차트가 다 없는 코스 위에서는 키가 움직이지 않는다(원본의 `getSelectedBarPlayConfig` 규칙). SCRATCH AUTO·KEEP GREEN NUMBER 는 전역 설정이라 막지 않는다.
- JUDGE ALGORITHM 은 COMBO·DURATION·LOWEST 세 가지만 돈다. 지금 값이 SCORE 이면 키가 움직이지 않는다(SETTINGS > JUDGE 에서 정한다).
- 켬·끔 칸은 키를 누를 때마다 한 번 뒤집는다. 이벤트 경로로 실행되므로 소리, 변경 표시, 닫힐 때의 저장이 기존 칸과 같다.

### 12.3 NOTE DISPLAY TIME(패널 3의 4·6번 키)

| 상태 | 칸에 보이는 것 | 4·6번 키 |
| --- | --- | --- |
| KEEP GREEN NUMBER 꺼짐 | `OFF` 와 `KEEP GREEN NUMBER IS OFF (SELECT + KEY 5)` | 아무것도 바꾸지 않는다 |
| 켜짐, 커서가 차트(또는 차트가 다 있는 코스) | 이동 시간(ms)과 녹색 숫자 | 1 ms 씩. 누르고 있으면 반복하고 50번 반복한 뒤에는 한 번에 10 ms |
| 켜짐, 커서가 폴더·표·실행 파일·차트가 다 없는 코스 | `-` 와 `MOVE THE CURSOR TO A CHART TO SET IT` | 바꾸지 않는다 |

마지막 줄은 2026-10-22 에 고쳤다. 전에는 이 경우에도 칸이 `OFF` 와 KEEP GREEN NUMBER 가 꺼져 있다는 경고를 보이면서 키는 보이지 않는 이동 시간을 바꿨다. 선곡의 `NUMBER_DURATION` 이 차트가 아닌 막대에서 값 없음을 답하기 때문이다.

### 12.4 턴테이블을 누름으로 읽는 칸

패널 1 의 TARGET 은 방향을 누르고 있는 동안 움직이고(원본과 같다), 패널 2 의 HIDDEN+ 와 패널 3 의 JUDGE ALGORITHM 은 누름 한 번에 한 번이다. 목록을 턴테이블로 넘기다 패널을 열면 그 방향이 눌린 채 남아 있으므로, 패널이 바뀔 때(처음 열릴 때, 패널에서 패널로, START 를 놓아 3번에서 2번으로)는 이미 눌려 있던 턴테이블 방향의 변화를 버린다(`SelectKeys::forget_turntable_changes`, `show_panel` 이 부른다). 패널이 열린 뒤나 열리는 그 프레임에 돌린 것은 센다. 전에는 목록을 넘기던 방향 그대로 SELECT 를 누르면 HIDDEN+ 가 켜졌다.

### 12.5 F1 오버레이의 홀드 키

선곡의 옵션 오버레이(`app_options.rs`)는 홀드 키(키 설정 OPTIONS OVERLAY (HOLD), 출하 왼쪽 Shift)를 누르는 동안 열리고 핀 키(`F1`)로 고정된다. 7키 출하 배치에서 홀드 키는 턴테이블 정방향 키와 같다. 그래서 패널에서 턴테이블을 돌리면 오버레이가 같이 열려 TARGET·HIDDEN+·JUDGE ALGORITHM 을 쓸 수 없었다. 지금은 START·SELECT·상세 옵션 키 가운데 하나라도 눌려 있으면(패널이 열려 있으면) 홀드 키는 오버레이를 열지 않고 선곡으로 넘어간다(`open_key`, `an_option_panel_is_called_up`). 핀 키는 그대로이고, 패널이 없으면 전처럼 열린다. 스킨 없는 내장 선곡에서도 START 를 누른 채 홀드 키를 누르면 오버레이가 열리지 않는다(영향 작음).

### 12.6 5키·10키

한쪽 6레인(건반 5 + 스크래치)은 키 인덱스 0~4 와 7·8 로만 접히므로(`keyconfig.rs` 의 `key_index_of`) 인덱스 5·6 에 닿는 키가 키보드에도 컨트롤러에도 없다. 그 모드에서 닿지 않는 역할은 패널 1 의 HI-SPEED FIX 이전(6)·RANDOM 다음(7), 패널 2 의 LANE COVER(6)·LIFT(7), 패널 3 의 NOTE DISPLAY TIME 늘림(6)·JUDGE TIMING +1(7)이다. 동작은 바꾸지 않았다(5키·10키에서는 LANE COVER·LIFT 를 PLAY 메뉴나 플레이 중 키로 켜고 끈다). 기본 스킨은 이 칸의 키와 열을 어둡게 그려 닿지 않음을 알린다(확장 옵션 21140~21148. skin.md 3.4.5절).

### 12.7 외부 스킨과 알려진 한계

- 키 재배정은 `rbms_extensions` 를 선언한 스킨에만 적용된다(2026-10-23. 사용자 결정 5). beatoraja 용 외부 스킨(ModernChic 등)은 선언이 없어 원본 표를 쓰므로 패널 1 의 4번 키는 DP OPTION, 6번 키는 RANDOM 이전이고 패널 2 의 JUDGE AREA·MARK NOTE·BPM GUIDE·NO MINE 칸은 아무것도 바꾸지 않는다(12.2절의 표). 이전(2026-10-22)에는 이 칸이 설정을 바꿨으나 외부 스킨에는 표시가 없어 이를 막을 통로가 없었다. 남은 위험: (a) 사용자가 고쳐 둔 기본 팩의 `lua/select/main.lua` 사본(설치 관리자가 덮지 않는 파일)에는 선언이 없어 원본 표가 되고, 기본 스킨의 패널 그림(확장 칸을 그린다)과 키가 어긋난다. (b) JSON 스킨이 `rbms_extensions` 에 정수가 아닌 값을 쓰면 문서가 거절될 수 있다(시험하지 않았다). (c) 외부 팩으로 키를 누르는 확인은 `RBMS_SKIN_PACK` 이 있을 때만 돌아 CI 에서 하지 않았고 실제 창에서도 보지 않았다.
- 폴더 위에서 패널 1 의 HI-SPEED FIX 칸은 값 없음(`-`)으로 읽히지만 5·6번 키는 그 값을 바꾼다(차트가 다 없는 코스에서만 막힌다). 12.3절의 NOTE DISPLAY TIME 과 같은 불일치가 이 칸에 남아 있다(테스트 부채 3.91).
- 같은 프레임에 옵션 둘이 바뀌면(예: 1·3번 키를 한꺼번에) 기본 스킨의 INFORMATION 줄은 감시 순서상 마지막 것만 보인다.

### 12.8 구현 위치

| 내용 | 위치 |
| --- | --- |
| 키 표(`BEAT_TABLE`, `POPN_TABLE`, 선언 없는 스킨의 `REFERENCE_BEAT_TABLE`·`REFERENCE_POPN_TABLE`), 어느 표인지 정하는 `KeyLayout::of_browser`, 패널을 부르는 키(`Panel::held`), 누름과 눌러둠을 가르는 읽기 | `apps/rbms-player/src/stage/select/keys.rs` |
| 패널의 열고 닫음, 키가 하는 일(`press_act`, `run_panel_act`), 턴테이블 버림(`forget_turntable_changes`를 부르는 `show_panel`) | `stage/select/panel.rs` |
| 설정 이벤트(`run_setting_event`, `run_duration_event`, `step_setting`, `step_duration`) | `stage/select/events.rs` |
| 오버레이의 홀드 키 | `apps/rbms-player/src/app_options.rs`(`open_key`) |
| H 키 안내(`{START} + TURNTABLE = TARGET`, `{SELECT} + TURNTABLE = HIDDEN+ ON/OFF`, `{START}+{SELECT} + TURNTABLE = JUDGE ALGORITHM`) | `stage/select/guide.rs` |
| 스킨이 켬·끔을 일으키고 읽는 확장 id | `apps/rbms-player/src/skin_host/extension.rs` |
| 선언(`rbms_extensions`)을 읽는 곳 | `crates/rbms-skin/src/model.rs`(`SkinDef::knows_rbms_extensions`), `crates/rbms-skin/src/loader/lua_skin.rs`(헤더 값 복사) |
| 테스트 | `stage/select/keys.rs`(원본과 같은 칸·바뀐 칸), `panel_tests.rs`(턴테이블 버림, NOTE DISPLAY TIME), `events_tests.rs`, `app_options.rs`(홀드 키와 핀 키), `skin_host/select/tests.rs`(5키·10키 옵션) |


## 13. 없는 곡 보이기와 다운로드(2026-10-23)

결정은 [없는 곡과 다운로드 ADR](./acknowledge/2026-10-23-adr-missing-charts-and-download.md). 표에는 있는데 라이브러리에 없는 곡을 흐린 막대로 보이고, 그 곡을 플레이하려 하면 안내 모달을 띄워 차분(차트 파일)을 받게 한다.

### 13.1 설정

| 행 | 위치 | 값 | 비고 |
| --- | --- | --- | --- |
| MISSING CHARTS | SETTINGS > SELECT 의 SONG LIST 구역 | 켬(기본) / 끔 (`library.show_missing_charts`) | 켜짐이 기본인 것은 요구가 직접 요청했기 때문이다. 기호라 계정 설정 묶음과 같이 동기화된다 |
| DOWNLOAD FOLDER | 같은 탭의 DOWNLOADS 구역 | 폴더 고르기, 0 키로 없음 (`library.download_folder`, 기본 없음) | 곡 폴더 목록에 자동으로 들어가 곡 폴더와 같이 스캔된다(`LibraryOptions::scan_roots`, 곡 폴더와 같은 경로면 한 번). 기기 고유라 설정 묶음 올리기에서 빼고 내려받을 때도 이 기기의 값을 둔다. 첫 받기에서 폴더를 고른다(고르기 창 제목 `Select download folder`) |

곡 폴더 화면에는 곡 폴더들 뒤, `+ ADD FOLDER` 앞에 `DOWNLOADS  <경로>` 행이 보이고 거기서는 지울 수 없다(안내 `the download folder is changed in SETTINGS > SELECT`).

### 13.2 없는 곡 막대

1. 표 레벨 폴더 안에 라이브러리에 없는 곡이 가진 곡과 함께 흐린 막대로 보인다. 대상은 표의 md5 가 라이브러리에 없는 항목이고, 한 레벨 안에서 같은 md5 는 한 번만 나온다. sha256 만 있는 항목은 보이지 않는다(기존 한계).
2. 기본 화면은 글자를 흐리게 그리고 램프 띠와 레벨 칸을 비우며, 세부 패널에 제목·아티스트·`NOT IN THE LIBRARY - ENTER FOR THE DOWNLOAD` 를 보인다. 스킨에는 `exists: false` 인 곡 막대로 전달된다([skin.md](./skin.md) 1.4절).
3. 검색에는 나오지 않는다(검색은 라이브러리만 본다). 즐겨찾기 필터와 모드·레벨 범위 필터가 켜지면 빠지고, 클리어 필터는 기록이 없는 차트가 통과하는 값일 때만 남는다. 정렬은 제목 순이면 가진 곡 사이에 섞이고, 정렬 없음이면 가진 곡 뒤에 표 순서로, 그 밖이면 가진 곡 뒤에 제목 순으로 온다. 레벨 막대의 `(n)` 과 루트·묶음 합계, 폴더 램프 분포는 가진 곡만 센다.
4. 정체는 가진 곡과 같은 md5 라서 받은 뒤 그 줄이 가진 곡이 되어도 커서가 머문다.

### 13.3 안내 모달

- 없는 곡 막대에서 재생·오토·연습·리플레이 이벤트나 Enter 가 모달을 연다(원본은 모드와 상관없이 다운로드 사이트를 연다). 모달은 키(컨트롤러 키 포함)와 마우스를 먼저 받고, 열 때 디스크와 네트워크를 읽지 않는다. 모달 문구는 UI 언어를 따른다(15절, `text/select/download_modal.rs`).
- 제목 `NOT IN THE LIBRARY`, 안내 두 줄: 표의 DL 은 차트 파일만(차분)이고 소리·그림은 곡 URL 에서 따로 받아 같은 폴더에 두어야 한다. 곡 제목·아티스트·`표 이름 레벨 제목`도 보인다. 키 안내 `LEFT RIGHT CHOOSE   ENTER PRESS   ESC CLOSE`.
- 버튼 줄: `GET CHART`(링크가 직접 zip 이고 다른 받기가 없을 때만 켜짐. 받는 중에는 `CANCEL`), `OPEN SONG URL`, `OPEN CHART URL`(각각 브라우저에 넘겨도 되는 주소일 때만 켜짐), `CLOSE`. 꺼진 버튼은 흐리게 그리고 커서가 건너뛴다. 받는 중 줄은 `DOWNLOADING 1.2 MB / 3.4 MB`(크기를 모르면 받은 양만), `EXTRACTING`, `SCANNING`, 끝나면 결과 문구, 다른 곡을 받는 중이면 `ANOTHER DOWNLOAD IS RUNNING`. 예·아니오 확인창이 아니라 이 버튼 줄이 "받겠느냐"는 물음이다.

### 13.4 어떤 링크를 앱이 받나

- 경로(쿼리 제외)만 본다(`classify_link`): http·https 가 아니면 쓸 수 없음, 호스트 `stellabms.xyz` 의 `/upload/...` 이거나 경로가 대소문자 무시 `.zip` 으로 끝나면 직접 zip, 그 밖은 브라우저. `page.html?file=a.zip` 은 페이지다.
- rar·7z·lzh, 단일 `.bms` 같은 차트 파일 링크, HTML·클라우드 페이지는 앱이 받지 않고 브라우저로 넘긴다(이때 `GET CHART` 가 꺼진다). 사용자 결정은 "직접 zip 링크만 앱이 받아 풀고 나머지는 브라우저"다.
- 브라우저에는 `browsable_url` 을 통과한 주소만 넘긴다(http·https, 호스트 있음, 공백·제어 문자 없음, 2,048자 이하). 셸을 거치지 않고 열기 프로그램을 따로 실행한다: macOS `open`, Windows `rundll32 url.dll,FileProtocolHandler`(`cmd /C start` 는 `&`·`|`·`^`·`%` 를 해석한다), 그 밖은 `xdg-open`. 실패는 `could not open the browser: 이유` 로 알린다.

### 13.5 받기·풀기와 안전 장치(크레이트 `rbms-download`)

| 구분 | 값 |
| --- | --- |
| 받기 상한 | 512 MiB, 무응답 30초, 전체 15분(조각마다 남은 시간과 취소를 확인), 리다이렉트 10회, 최종 주소도 http·https |
| 받은 파일 검사 | 처음 4바이트가 zip 머리(`PK\x03\x04`, 빈 보관 `PK\x05\x06`)가 아니면 거절(HTML 응답 포함). 알린 크기가 상한을 넘으면 읽기 전에 거절하고 받은 바이트로도 센다. 실패하면 파일을 남기지 않는다 |
| 풀기 상한 | 항목 10,000개, 항목 하나 1 GiB, 전체 2 GiB. 쓴 바이트로 센다(머리에 적힌 크기를 믿지 않는다) |
| 이름·경로 | 항목 이름은 UTF-8 이거나 아니면 Shift_JIS 로 읽는다. 빈 이름·`.`·`..`·제어 문자·Windows 금지 문자·끝의 공백·점·Windows 장치 이름인 항목은 건너뛰고, 심볼릭 링크 항목도 건너뛴다 |
| 쓰는 파일 | 차트(`bms bme bml pms bmson`), 소리, 그림, 영상, `txt` 확장자만. 실행 파일·스크립트는 쓰지 않는다 |
| 덮어쓰기 | 없다(이미 있는 이름은 건너뛴다). 풀린 폴더는 다운로드 폴더 안에 `제목 [md5 앞 8자]` 로 옮기고, 있으면 ` (2)`, ` (3)`... |
| 차트가 없는 압축 | 풀린 폴더를 지운다. 압축에 차트는 있지만 이 md5 가 없으면 폴더는 두고 알린다 |

- 작업은 한 번에 하나다. 다운로드 폴더 안의 `.rbms-partial` 에서 받고 풀어 최종 폴더로 옮기며, 끝나면 `.rbms-partial` 은 지워진다.
- 끝난 일은 그 폴더만 부분 스캔하고(`ScanRequest::partial`, 곡 DB 의 파서 버전을 기록하지 않아 전체 재스캔이 끝난 것으로 취급되지 않는다) 표를 다시 맞춘다. 선곡이 열린 폴더와 커서를 지킨 채 목록을 받아들이고, 플레이 중에 끝난 일은 선곡으로 돌아왔을 때 받아들인다.
- 결과 알림: `downloaded: 제목 (n chart(s))`, `the archive had n chart(s) but not this one`, `download failed: 이유`, `download cancelled`. 시작할 수 없는 이유는 `another download is running`, `the chart's link is not a zip archive`, `no download folder is chosen`.
- 새 의존성은 `zip` 9.0.0 과 `typed-path` 0.12.3 이다(crates.io 게시 14일이 지난 판만 쓴다는 작업 지시를 따랐고, 이 규칙의 출처는 저장소 규칙 문서에서 찾지 못했다). 곡 미러(음원 파일 제공)는 이 기능에 없다.
- 확인하지 못한 것: 실제 Stella·BMS 사이트에서 받아 본 결과(직접 zip 링크의 실제 응답), Windows 에서 폴더가 막은 이름의 동작(`a6dc73c` 가 접근 거부를 건너뛰게 고쳤다).

## 14. Discord 상태 표시 (2026-10-23)

- 설정 NETWORK > DISCORD > DISCORD STATUS(기본 켬). 켜져 있으면 Discord 데스크톱 앱 프로필에 "R-BMS" 하는 중이 보이고, 선곡·곡 플레이(곡 제목, `7K ANOTHER 12` 꼴, 오토플레이·리플레이·연습 표시)·코스(`n/m · 코스 이름`)·결과에 따라 줄이 바뀌며 경과 시간이 붙는다. 끄면 표시를 지우고 연결도 하지 않는다. 프로필을 볼 수 있는 사람은 곡 제목을 본다.
- Discord 가 꺼져 있으면 조용히 기다렸다가 다시 붙는다(0.5초부터 60초까지). 앱을 끄면 표시를 지운다. 플레이 중에는 게임 스레드가 새 표시를 보내지 않고, Discord 스레드가 이미 받아들여진 표시를 20초마다 다시 보낼 뿐이다.
- 근거와 세부: [Discord ADR](acknowledge/2026-10-23-adr-discord-rich-presence.md), 설계 `docs/plan/2026-10-23-discord-presence.md`.


## 15. 언어(LANGUAGE, 2026-10-24)

앱의 글이 쓰이는 언어를 영어·한국어·일본어에서 고른다. 구조와 결정은 [UI 다국어 ADR](./acknowledge/2026-10-24-adr-ui-language.md), 용어는 [용어집](./reference/2026-10-24-i18n-glossary.md).

| 항목 | 내용 |
| --- | --- |
| 설정 | SETTINGS > DISPLAY 탭 맨 위 LANGUAGE 행. 값 `AUTO`·`ENGLISH`·`한국어`·`日本語`(언어 이름은 그 언어로). 저장은 `display.language` 의 `AUTO`·`ENGLISH`·`KOREAN`·`JAPANESE`(없으면 AUTO). 계정 설정 묶음과 함께 동기화된다 |
| AUTO | 환경변수 `RBMS_LANGUAGE`(`en`·`ko`·`ja`. 설정이 AUTO 일 때만) → OS 로캘의 첫 선호 언어 → 영어. 로캘은 언어 부분만 본다(`ko-KR`·`ja_JP.UTF-8` 은 그 언어, `en-KR` 은 영어). 지원 밖(`zh-TW`, `fr`, 빈 값, `C`)은 영어. 환경변수가 세 언어 어디에도 닿지 않으면 건너뛰고 OS 로캘이 정한다. 환경변수와 OS 로캘은 AUTO 를 처음 정할 때 한 번 읽는다 |
| 적용 시점 | 값을 바꾼 같은 프레임에 모든 화면이 따른다. 설정 화면은 목록을 다시 만들고 선곡의 장면 캐시와 폴더 이름도 다시 만든다. 기본 스킨은 다음 프레임에 바뀐다(확장 불린 21150~21152 와 `t(key)`, [skin.md](./skin.md) 1.4절) |
| 번역되는 것 | 설정(탭·행 이름·설명·값 일부·확인 문장), 선곡(가이드, 필터, 장면, 기록 모달, 랭킹 오버레이, 다운로드 모달, 폴더 이름), 결과·HUD, PLAY·SKIN 메뉴, 표·폴더·코스·연습·키 편집기·로딩, 알림(사용자가 한 일의 결과), 기본 스킨의 고정 문구(키 234개) |
| 원어로 두는 것 | 게임 용어(판정·게이지·램프·옵션·점수 이름, 키 이름, BPM, TOTAL), 설정 값(`ON`·`OFF`·`AUTO`·`DEFAULT`, 모드·게이지·알고리즘 이름. 설정 화면이 영어 정본으로 비교한다), 패널 제목(CHART DATA, MY SCORE, NOTES RADAR 등)과 레이더 축 이름 |
| 번역하지 않는 것 | 곡 제목·아티스트·표·폴더 이름 같은 데이터, 외부 스킨의 글자, 스킨 메뉴의 옵션 이름(헤더 읽기 단계라 영어), 서버가 주는 오류 문구(한국어 그대로), Discord 상태 문구(영어), CLI·stderr 로그와 진단성 알림, 표본 데이터 |
| 한국어·일본어 글꼴 | 번들 Inter 에 없어 시스템 글꼴 폴백이다. 캡처 하니스는 번들 글꼴만 써서 CJK 모양을 볼 수 없다(글자 폭·줄 높이는 실창 점검, QA 3.23) |
| 알려진 한계 | 같은 화면에 영어가 섞여 남는 곳이 있다(스킨 메뉴 상단 탭 이름, 표 화면의 `MY FOLDERS` 행 라벨 등. ADR 의 영향 절). 한국어는 사용자 검수 전, 일본어는 원어민 감수 전이다 |

개발에서 언어를 고정하려면 `RBMS_LANGUAGE` 를 쓴다([development.md](./development.md)). 시험은 영어를 명시하므로 호스트 언어와 무관하다.

## 16. 랭킹 모드(RANKING KEYS, 2026-10-24)

선곡에서 곡의 IR 순위를 건반으로 열어 본다(사용자 요청 "2, 4, 6 셋중 하나 누르면 해당 곡의 랭킹들을 볼수있어짐"). 결정과 근거는 [랭킹 모드 ADR](./acknowledge/2026-10-24-adr-ranking-mode.md). 설정 RANKING KEYS(SELECT 탭 SONG LIST 구역, 기본 켬)가 켜져 있고, 차트 막대 위이며, 서버가 설정돼 있고, 패널·모달·검색이 없을 때 쓴다.

| 항목 | 내용 |
| --- | --- |
| 열기 | 파란 건반을 300 ms(`SCRATCH_REPEAT_DELAY`) 이상 누른다. 7키 2·4·6, 5키·10키 2·4, DP 양쪽, 9키 버튼 2·8, 24키 없음. 키보드는 `S`·`D`(6번 `F` 는 즐겨찾기 그대로). 목록 역할이 없는 키(7키 컨트롤러 6번, 9키 2·8번)는 누르는 즉시 열린다. 짧게 누르면 뗄 때 전과 같다(폴더 닫기, 최상위에서는 정렬). 내장 선곡의 9키는 버튼 2 만 랭킹 키다 |
| 모드 안 | 턴테이블·위아래·휠은 줄 이동. 흰 건반·Enter 는 그 줄의 리플레이 재생, `J` 는 그 줄의 배치로 치기(10.15절). SELECT 는 내 줄로, START 는 다음 쪽(100줄씩), `Q`·`E` 는 프로필 전환 |
| 닫기 | 파란 건반 짧게, `Esc`, `I`, 왼쪽 |
| 자동 받기 | Songs 탭의 차트에 5초 머물면 패널이 닫혀 있어도 순위를 받는다(외부 스킨의 순위 슬롯을 위해). 열려 있으면 150 ms 뒤, 열면 즉시 |
| 설정을 끄면 | 키로 여닫는 것만 꺼진다. `I` 키, 스킨 이벤트 21002, 모드 안 조작은 그대로 작동한다 |
| 줄 | 순위 · 이름과 단위 배지(최대 2개) · DJ LEVEL · EX · 램프(등급 포함) · 날짜 · REP. 내 줄은 밝은 레일과 옅은 바탕, 라이벌 줄은 어두운 레일. 머리는 `IR RANKING`, `YOU #r`, 곡 제목과 키 안내. 쪽 안내 줄: 더 받을 쪽이 있으면 `TOP n SHOWN, MORE ON THE NEXT PAGE`, 내 순위가 있는 쪽을 따로 받았으면 `YOUR PAGE, FROM RANK n` |
| 데이터 | 100줄 쪽, 보드 10분(`RANKING_READY_LIFETIME`) 뒤 만료, 점수가 받아들여지면 그 곡만 무효화, 실패는 30초 뒤 다시. 순위는 서버의 `rank`(램프 → 어시스트 등급 → EX → BP) 그대로다. 줄의 단위 배지는 웹 `GET /api/charts/{hash}/ranking` 의 `extra.dans` 에서 온다([api-spec.md](./backend/api-spec.md) 4절) |
| 외부 스킨 | 순위 슬롯 값(정수 380~389 EX·390~399 순위, 이미지 인덱스 380~389 종류·390~399 램프, 텍스트 120~129 이름, Rate 8 읽기·쓰기)이 선곡에서 값을 낸다. Rate 8 에 쓰면 목록이 스크롤되고 오프셋은 곡이 바뀌면 0 이다 |
| 기본 스킨 | 랭킹 모드(불린 21105)에서 MY RECORDS 자리가 IR RANKING 6줄이 된다. 그러나 앱 오버레이가 정보창 자리를 덮어 대부분 가려진다(미확인 목록 25번, 테스트 부채 3.122) |
| 구현·시험 | `apps/rbms-player/src/stage/select/{ranking_mode,ranking_scroll,mod,skinned,events,overlay}.rs`, `ir_ranking.rs`, `app_ranking.rs`, `ir_ranking_view.rs`, `skin_host/{ir,select,select/bridge,writers}.rs`. 시험은 가짜 시계의 상태 기계와 가짜 채널을 쓴다 |

알려진 위험: 300 ms 길게 누름과 "뗄 때" 동작의 손맛은 실기로 보지 않았다. 서버가 있으면 차트에 5초 머물 때마다 순위 요청이 간다.

## 17. 선곡 화면의 새 구성(기본 스킨, 2026-10-24)

기본 스킨의 선곡이 IIDX 식으로 바뀌었다(사용자 요청 "곡선택부분에 보면 곡 + 왼쪽에 스코어도 들어가있는데 이것도 구현하도록하고 ... iidx와 비슷하게"). 결정은 [선곡 IIDX 식 ADR](./acknowledge/2026-10-24-adr-select-iidx-layout.md), 스킨 쪽 설명은 [skin.md](./skin.md) 3.4.6절. 바탕·면·글자는 모노톤 그대로이고 강조색은 등급 배지, 램프 띠, SCORE TARGETS 메달, 레이더 칠하기에만 쓴다. 이 절의 값은 전부 커서가 있는 곡에서 온다.

### 17.1 곡 막대

| 칸 | 내용 |
| --- | --- |
| 등급 배지 | 왼쪽 첫 칸에 그 곡의 DJ LEVEL(F ~ AAA). **보조 판을 뺀** 최고 기록 기준이라 정보창의 RANK 와 같다. 기록 없는 곡·폴더·코스·없는 곡에는 없다. 내장 선곡의 행 DJ LEVEL 도 같은 기준으로 바뀌었다(보조 판으로만 최고인 곡은 낮아지거나 사라진다. 미확인 목록 21번) |
| 램프 띠 | 배지 오른쪽 8px 띠(기존 램프 색) |
| 레벨 | 크게(32 크기 숫자) |
| 제목 | 레벨 오른쪽. 폴더·표 막대는 더 오른쪽에서 시작한다 |
| 썸네일 | 오른쪽 끝 64x64. 스테이지 그림(없으면 배너)의 가운데 정사각을 128x128 로 줄인 것. 첫 프레임에는 없다가 작업자가 읽은 뒤 나타난다. 커서 위 11·아래 12 막대만 읽고 최근 48장을 둔다 |
| 선택 막대 | 가운데 칸이 16px 왼쪽으로 돌출하고 테두리와 띠가 붙는다 |
| 스크롤 막대 | 트랙이 열린 폴더의 램프 분포(램프별 차트 수)를 누적해 칠한다(아래 MAX, 위 NO PLAY) |

### 17.2 정보창

| 패널 | 내용 |
| --- | --- |
| 곡 머리와 그림 | 장르·제목·부제·아티스트 네 줄은 글 열(x 104~896, 폭 792)에 왼쪽 맞춤이고 모두 overflow 3(아래 "글자가 넘칠 때"). 오른쪽 열(x 928~1096, 폭 168)에 LEVEL(라벨과 난이도 색의 큰 숫자)과 BPM(라벨과 `150` 또는 `120 - 180` 꼴의 글자)이 오른쪽 맞춤으로 선다. 코스 막대에서는 제목 자리에 `COURSE` 가 보인다. 256 정사각 곡 그림(그림 숨김이면 비움) |
| CHART DATA | 난이도 탭 5칸(같은 폴더·제목·모드의 `#DIFFICULTY` 1~5, 클릭하면 그 차트로 이동. 현재 목록에 있는 차트만). 왼쪽 블록: 첫 줄 NOTES·NPS AVG·NPS MAX(칸 폭 120, 칸 사이 16, 값은 24 크기)와 그 아래 밀도 그래프(400 x 152). 오른쪽 블록: 2x2 LENGTH·TOTAL / KEYS·JUDGE(칸 폭 144)와 그 아래 레인 막대(312 x 92. 5·7키는 스크래치 왼쪽, 10·14키는 두 줄). LEVEL·BPM 은 곡 머리로 옮겼다. NPS 의 정의는 아래 "NPS 평균·최고" |
| NOTES RADAR | 6각형(NOTES·PEAK·SCRATCH·SOF-LAN·CHARGE·CHORD, 표시값 0~200, 가장 큰 축에 별). 그림 한 변 160(반지름 77.5), 중심은 영역 위에서 144 아래. 위쪽 라벨과 옆 라벨이 13px 이상 떨어진다. 9키·24키는 `-`. 값의 정의와 한계는 17.4절 |
| MY SCORE | 첫 줄 EX SCORE(큰 숫자, 폭 144)·MISS COUNT(폭 96)·등급 배지(폭 128 x 높이 56), 사이 16. 램프 이름, 다음 등급까지의 차이, F~AAA 진행 막대, 통계 넷은 2x2 라벨-값 쌍(MAX COMBO·RATE / PLAY COUNT·CLEAR COUNT. 쌍 폭 184, 쌍 사이 32), 판정 5행 |
| SCORE TARGETS | AAA·AA·A 의 필요 EX(`ceil(TOTAL NOTES x 2 x k/9)`, k = 8·7·6)를 금·은·동으로, 달성하면 메달 면. 칸 피치 200(칩 52, 필요 EX 상자 74, 차이 상자 52)이라 5자리 필요 EX 와 4자리 차이(`+2243`)가 칩과 이웃 칸을 덮지 않는다 |
| CLEAR RATE | CLEAR COUNT / PLAY COUNT(소수 1자리) |
| MY RECORDS | BEST 1줄 + 최근 5판(최신순, 보조 판 포함, 읽기 한도 1000판), 줄마다 램프·DJ LEVEL·EX·MISS(도달하지 못한 노트 포함)·날짜(앞 10글자). BEST 는 기록을 깰 수 있는 판이 없으면 비어 있다. 랭킹 모드에서는 IR RANKING 6줄(16절). MISS 는 4자리 칸이라 10000 이상이면 아래 4자리만 보인다(기존 한계) |
| 폴더·코스 | 폴더·코스 막대의 정보는 전과 같다 |

**NPS 평균·최고(2026-10-29)**: beatoraja 의 선곡 밀도 값을 그대로 쓴다. 평균은 정수 id 364(정수부)와 365(소수 두 자리), 최고는 360과 361 이고 `NPS AVG`(한국어 `NPS 평균`, 일본어 `NPS 平均`), `NPS MAX`(`NPS 최대`, `NPS 最大`)로 보인다. 차트를 1초 칸으로 나눠 세는데 일반 노트와 롱노트 머리는 시작한 초에 1, 롱노트 몸통은 걸친 초마다 1, 지뢰는 세지 않는다. 최고는 가장 바쁜 1초의 수이고, 평균은 `전체 노트 수 / 초 칸 수 / 4`(정수 나눗셈) 이상인 활동 초의 평균이다. 그래서 LN 이 많은 차트는 엄밀한 초당 누름 수보다 크고, 조용한 초는 평균에서 빠진다. 소수 두 자리는 반올림이 아니라 버림이다. 값이 없으면(커서가 움직이는 중, 폴더·코스 막대) 그리지 않고, 평균과 최고가 모두 0.00 이면 `-` 를 보인다(노트가 없거나 마지막 노트가 24시간 이상 뒤라 초 칸 표를 만들지 않는 차트. 곡 DB 의 `song_detail` 값이고 새 확장 id 는 없다). 구현: `crates/rbms-chart/src/lib.rs` 의 `note_density`, `apps/rbms-player/src/skin_host/chart.rs`, 기본 스킨 `lua/select/info.lua` 의 `density`. 값이 속한 id 는 [skin.md](./skin.md) 3.4.6절.

**글자가 넘칠 때(overflow 3, 2026-10-29)**: 정보창과 목록 막대의 긴 글(곡 머리 네 줄, 막대 제목, 패널 제목, 레이더 축 이름, 폴더 램프 이름, 코스와 기록의 이름·머리)은 엔진의 새 overflow 값 3(`SHRINK_CUT`)으로 그린다. 줄이 폭을 넘으면 가로로 줄이되 원래 폭의 0.75 까지만 줄이고, 그보다 줄여야 하면 앞쪽 글자만 남기고 `…` 를 붙인다. 이전에는 하한 없는 축소(overflow 1)라 아주 긴 제목이 납작해졌다. 대가로 아주 긴 제목의 뒷부분은 이제 보이지 않는다. EX SCORE·MAX COMBO 같은 영어 상수 라벨은 용어집대로 원어를 유지하며 번역하지 않는다. 한국어·일본어는 한 글자가 글자 크기만큼의 폭이라 라틴보다 일찍 말줄임이 나온다(곡 머리 글 열이 792 로 줄었다). 캡처 하니스는 CJK 를 그리지 못해 실창 확인이 필요하다(QA 3.25 V10). 엔진 값의 정의는 [skin.md](./skin.md) 1.4절.

머리에는 PLAY STYLE 칩(SP·DP·9KEYS), FREE PLAY 또는 COURSE 칩, 시각 칩, 줄인 검색 상자와 정렬·모드·LN MODE 단추가 있고, 목록 위에 열린 폴더의 경로가 보인다. 바닥은 키 이름 상자 안내(ENTER·ESC·UP DOWN·H)와 REPLAY·AUTOPLAY·PRACTICE·RANKING·FAVORITE 단추다. RANKING 단추는 랭킹 모드를 여닫는다(이벤트 21002, 켜짐 불린 21105).

### 17.3 분석 값이 생기는 때

레이더·레인·밀도 그래프·종류별 노트 수·주 BPM 은 커서가 150 ms 머문 차트를 **분석 작업자 스레드**가 파싱해 낸다(프레임 스레드는 파싱하지 않는다). 레이더는 곡 DB 의 `song_radar` 에 현재 버전 행이 있으면 즉시 나오고 없으면 작업자가 재서 저장한다. 커서가 움직이는 중, 폴더·코스 막대 위, 코스 탭, 읽을 수 없는 파일(드라이브가 빠짐)에서는 값이 없고 기본 스킨은 그리지 않는다. 스킨 없는 내장 선곡 화면은 이 분석을 쓰지 않는다.

### 17.4 NOTES RADAR 의 정의와 한계

- 누름 이벤트(일반 노트와 롱노트 머리, 꼬리와 지뢰는 아님)를 센다. 그래서 LN MODE 와 상관없이 같은 값이다.
- NOTES(초당), PEAK(4초 창의 최대), SCRATCH(스크래치 레인 초당)는 공개 구간표(IIDX 의 역공학 자료, 원문과 12칸씩 대조)로 0~200 에 대응한다. SOF-LAN(속도 변화 뒤 노트의 가중 합)·CHARGE(롱노트 점수)·CHORD(동시 누름 가중 합)는 가설 정의와 이 라이브러리의 분포로 정한 임시 스케일(13.0·3.0·300.0)이다. **IIDX 의 값과 수십 % 달라도 이상하지 않다.**
- 5·7키는 SP 구간, 10·14키는 DP 구간, 9키·24키는 없다. `rbms-cli` 의 차트 요약이 원시값과 표시값을 출력한다(`radar ver`, `radar raw`, `radar disp`).
- 곡 DB 의 새 표 `song_radar` 는 스캔이 새·바뀐 차트에만 쓴다. 기존 곡은 커서가 머물 때 채워진다. 전체 재스캔은 강제하지 않는다.
- 구현·시험: `crates/rbms-chart/src/radar.rs`, `crates/rbms-library/src/songdb/`, `crates/rbms-render/src/skin_render/graphs/radar.rs`, `apps/rbms-player/src/stage/select/{analysis,browse_facts,thumbs}.rs`, `skin_host/{chart,select/browse}.rs`, 기본 스킨 `lua/select/{list,controls,info,radar,records}.lua`. 결정과 정의는 [NOTES RADAR ADR](./acknowledge/2026-10-24-adr-notes-radar.md).

### 17.5 외부 스킨과 내장 선곡

- 막대 확장 키(`rank`·`thumb`)는 기본 스킨만 쓴다. 외부 스킨은 키가 없어 그리기와 호출 순서가 전과 같다.
- 선곡에서 원본 id 350~353·92·`judgegraph`·`bpmgraph`·순위 슬롯이 값을 내서 외부 스킨이 보는 선곡이 바뀐다(원본과 같아짐).
- 내장(스킨 없는) 선곡 렌더러는 그대로다(배지·썸네일·육각형 없음). 단 행의 DJ LEVEL 은 보조 판 제외 기준으로 바뀌었다.

### 17.6 선곡의 음량 슬라이더(기본 스킨, 2026-10-29)

기본 스킨 선곡의 사이드 메뉴(스피커 아이콘)에는 VOLUME 슬라이더 셋이 있다. 사용자 보고 "사운드 조절 (곡선택화면)이 제대로 먹히긴하나 ? 안먹는 느낌인데"의 원인은 슬라이더가 움직이는 버스와 선곡에서 들리는 소리가 타는 버스가 어긋난 것이었다(MASTER 가 시스템 버스를 움직여 미리듣기에 닿지 않았다). 결정과 근거는 [선곡 음량 ADR](./acknowledge/2026-10-29-adr-select-volume-master.md).

| 슬라이더(스킨 ID) | 움직이는 값 | 선곡에서 영향받는 소리 |
| --- | --- | --- |
| MASTER(rate 17, 숫자 57) | `audio.master`. SETTINGS > AUDIO > MASTER VOL 과 같은 값(기본 100%) | 모든 소리: 미리듣기(`#PREVIEW` 클립과 자동 재생 모두), 시스템 효과음, 선곡 음악 |
| KEY(rate 18, 숫자 58) | `audio.key`(KEY VOL, 기본 50%) | 자동 재생 미리듣기(`#PREVIEW` 가 없는 차트)의 노트 키음. `#PREVIEW` 클립은 섞인 녹음이라 닿지 않는다 |
| BGM(rate 19, 숫자 59) | `audio.bg`(BGM VOL, 기본 50%) | `#PREVIEW` 클립과 자동 재생 미리듣기의 BGM |

- 드래그한 값은 그 프레임 끝에 소리에 반영되고 버튼을 뗄 때 설정 파일에 한 번 쓴다. 값 곡선은 선형 이득이다(바꾸지 않았다).
- 미리듣기: 자동 재생 미리듣기는 각 소리를 플레이와 같은 버스로 울린다(노트 키음은 Key 버스, 배경음은 Bg 버스. `play_sink::bus_for`). 미리듣기 페이드는 두 버스에 같은 레벨을 곱하고 미리듣기가 끝나면 설정값으로 되돌린다. 시스템 버스는 인터페이스 소리의 것이라 건드리지 않는다.
- 레퍼런스와 다른 점: beatoraja 의 rate 17 / 숫자 57 은 `AudioConfig.systemvolume` 이고 선곡 미리듣기가 그 음량으로 나온다. 마스터 음량이 따로 없다. R-BMS 에서는 이 두 id 가 마스터를 움직이고 보인다. 이 id 를 쓰는 외부 beatoraja 스킨도 마스터를 보인다. 시스템(효과음) 음량은 SETTINGS > AUDIO > SYSTEM VOL 과 Lua `main_state.set_volume_sys` 로만 바꾸고, Lua `volume_sys`·`volume_key`·`volume_bg` 의 뜻은 시스템·키·배경 그대로다. 그래서 `volume_sys` 로 자기 슬라이더를 만든 스킨은 같은 스킨 안에서 마스터와 다른 값을 보일 수 있다.
- 업데이트 뒤 보이는 변화: 이전의 MASTER 슬라이더(= `audio.system`)를 내려 효과음을 줄이던 사용자는 슬라이더가 `audio.master`(기본 100%)를 보이는 것을 본다. 저장된 `audio.system` 값은 그대로 남는다. 효과음 음량은 SETTINGS 의 SYSTEM VOL 로 바꾼다.
- 남은 것: 미리듣기 페이드(약 200 ms) 중에 슬라이더를 끌면 Key·Bg 가 한 프레임 동안 전체 값으로 돌아갔다가 페이드가 다시 맞춘다(엔진의 3 ms 슬루로 희미한 튐이 들릴 수 있다. 테스트 부채 3.139). KEY 는 `#PREVIEW` 클립이 있는 차트에서 아무 효과가 없다. 믹서 시험은 이득 사슬만 증명하므로 실제 체감은 출력 장치로 본다(QA 3.25 V12).
- 구현·시험: `apps/rbms-player/src/skin_host/{writers,system}.rs`(`VolumeTarget`, `rate_volume`, `Volumes::of_audio`), `skin_screen.rs`·`skin_screen/preview.rs`, `stage/select/preview.rs`(`preview_bus_gains`, `autoplay_preview_schedule`, `put_buses_at`). 시험은 같은 파일들의 `writers/tests.rs`·`system/tests.rs`·`skin_host/tests.rs`·`skin_screen/{tests,typing_tests}.rs`와 `preview.rs` 의 `the_master_key_and_bgm_sliders_each_turn_a_preview_down`(실제 믹서에 마스터·버스 이득을 넣어 잰다).

## 18. 재생 속도(PLAYBACK RATE, 2026-10-29)

차트를 100% 가 아닌 속도로 재생해 치는 기능이다(사용자 요청 "플레이 배속기능 (100퍼가 기본이고 1퍼씩 조절해서 줄이거나 높이거나 하는 기능)"). 재생 속도 자체를 바꾸므로 소리의 음높이도 함께 바뀐다. **100% 가 아닌 판은 기록을 남기지 않는다**(사용자 결정). 결정과 근거는 [PLAYBACK RATE ADR](./acknowledge/2026-10-29-adr-playback-rate-unrecorded.md).

| 항목 | 내용 |
| --- | --- |
| 값 | `play.playback_percent`(설정 파일 `settings.ron` 의 `play` 묶음). 50~200 의 정수 %, 기본 100. 범위 밖 값과 이전 설정 파일은 읽을 때 범위로 자르거나 100 으로 읽는다. 상수는 `PLAYBACK_RATE_MIN_PERCENT`(50)·`MAX`(200)·`STEP`(1)·`UNCHANGED`(100)이고 연습의 FREQUENCY 범위와 같은 상수를 가리킨다 |
| 바꾸는 곳 | PLAY 메뉴 CHART 묶음의 PLAYBACK RATE 행(7.2절. ← → 1%, Shift 10%, 0 키 100%, 슬라이더와 직접 입력은 7.3절 그대로), F1 오버레이의 마지막 행(열두 번째. ← → 1%). 플레이 중에 바꾸는 키는 없다. 판을 시작할 때의 값이 그 판의 배율이다 |
| 소리 | 배율을 목소리마다 피치로 곱한다(오디오는 미리 디코딩한 PCM 을 소수 보폭으로 선형 보간해 읽는다). 믹서와 디코더는 바꾸지 않았다. 보폭이 1 을 넘는 200% 근처에서는 선형 보간의 에일리어싱이 생길 수 있고 연습의 200% 와 같은 수준이다(실창에서 듣지 않았다) |
| 판정 | 판정 창(PGREAT~空POOR)·롱노트 마진·HCN 틱(200 ms)·FAST/SLOW 의 ms 값·판정 오프셋·자동 오프셋 조정·타이밍 통계가 모두 실제 시간(플레이어의 시간) 길이다. 입력은 오디오 시각을 차트 시각으로 바꿔 세션에 넣는다. 연습 FREQUENCY 와 같은 경로다(9.3절). 200% 에서 PGREAT 창은 실제 ±20 ms 다 |
| 표시 | 스킨과 내장 화면의 BPM(현재·MAIN·MIN·MAX), 이동 시간, 녹색·흰색 숫자, KEEP GREEN NUMBER 의 기준 템포가 실시간 기준(BPM x 배율)이다. 같은 HI-SPEED 에서 200% 면 노트가 실제로 두 배 빨리 내려오고, KEEP GREEN NUMBER 가 켜져 있으면 하이스피드가 절반이 되어 녹색 숫자가 유지된다. AUTO ADJUST 는 그대로 동작하고 판정 오프셋 설정을 바꾼다. 남은 시간(스킨 숫자 163·164)은 차트 시간 그대로다. 곡 BGA 는 차트 시각을 따르고 스킨 동영상과 장면 타이머는 실시간이다 |
| 코스와 리플레이 | 코스와 리플레이 감상은 설정과 무관하게 100% 다(`AppShared::run_playback_percent`). 코스는 스테이지마다 기록을 남기고, 리플레이는 100% 로 기록된 판이라 다른 배율로 재생하면 판정이 달라지기 때문이다. 코스를 시작할 때 설정이 100 이 아니면 알림 `a course is played at 100%: PLAYBACK RATE n% is set aside until it ends`(세 언어)를 띄운다. 오토플레이는 배율을 따른다(어차피 기록이 없다) |
| 연습 | 연습 구간은 자기 FREQUENCY 만 쓰고 전역 배율을 무시한다(두 배율이 곱해지지 않는다). 연습 FREQUENCY 판에도 위 "표시"가 적용된다(동작 변화. 9.3절) |
| 기록 | 100% 가 아닌 판은 로컬 DB·리플레이·trace·보류 판·서버 전송·기록 동기화가 모두 없다. 끝까지 친 판, 도중 포기, 앱 종료, 실패 화면에서 곧장 다시 하기가 같다. 사유는 `playback rate`(`practice::PLAYBACK_RATE_BLOCK_REASON`). 10.2절의 해당 행 |
| 결과 화면 | 결과 화면은 보인다(연습과 다름). NEW RECORD 가 없고(`updates_score` 거짓이라 스킨의 ENABLE_SAVE_SCORE 도 꺼진다) 어시스트 줄 자리에 노란 줄 `PLAYBACK RATE n%: NOT SAVED, NOT SENT` / `재생 속도 n%: 저장·전송 안 됨` / `再生速度n%: 保存・送信なし`, IR 줄은 `IR: SKIPPED (playback rate / 재생 속도 변경 / 再生速度変更)`. R 다시 하기는 같은 배율로 다시 시작하지만 `updates_score` 가 거짓이라 배치 시드를 잇지 않는다(넓힌 판정과 같은 규칙). 파란 건반 + START 는 판의 종류를 가리지 않아 같은 배치로 다시 한다(10.7절) |
| 스킨 | 확장 불린 21106 `RBMS_OPTION_PLAYBACK_RATE_CHANGED`(배율이 100 이 아님)와 정수 21084 `RBMS_NUMBER_PLAYBACK_PERCENT`(그 배율 %, 꺼짐이면 100)를 답한다. 결과 화면은 끝난 판의 배율, 플레이 화면은 진행 중인 판의 시계 배율(연습 FREQUENCY 포함), 선곡과 그 밖의 화면은 설정값을 답하고 스킨 미리보기는 꺼짐이다([skin.md](./skin.md) 1.4절). 기본 스킨은 플레이 화면 머리 오른쪽에 노란 `RATE n%`, 결과 화면의 판정 줄에 `SCORE NOT SAVED` 대신 노란 `NOT SAVED (RATE n%)`, 선곡 정보창의 CHART DATA 머리 줄(난이도 탭 왼쪽)에 차트 막대 위에서 노란 `RATE n%` 를 그린다(3.4.8절) |
| 구현 | 설정 `crates/rbms-config/src/{schema,settings}.rs`(`PlayOptions.playback_percent`, `SettingId::PlaybackRate`), 시계 `apps/rbms-player/src/stage/play/mod.rs`(`whole_run_clock`, `set_playback_rate`, `changed_playback_percent`, `keeps_no_record`)와 `practice.rs`(`PracticeClock::heard_bpm`·`heard_length`), 배율의 출처 `app_play.rs`(`run_playback_percent`, `PendingChart.playback_percent`), 기록 차단 `app_result.rs`·`lib.rs`(`updates_score`, `ir_submission_block_reason`), 결과 화면 `stage/result.rs`, 스킨 `skin_host/{extension,options,play,result}.rs` |
| 시험 | `app_result/record_tests/playback_rate.rs`(결과 화면·IR SKIPPED·DB 행·리플레이 파일·전송·동기화가 없음, 도중 포기, 실패 뒤 다시 하기, 코스와 리플레이 감상은 100%, 연습은 자기 FREQUENCY), `stage/play/timing_tests.rs`(200% 에서 판정 창이 실시간 길이, 판정 오프셋이 차트 시간으로는 2배), `stage/play/hold_tests.rs`(녹색 숫자 유지, NOWBPM 2배), `stage/play_menu/tests.rs`·`tests/numbers.rs`·`app_options.rs`(행의 걸음), `stage/result/tests.rs`, `skin_host/{options,result}/tests.rs`, `rbms-config` 의 `the_playback_rate_reads_from_older_files_and_stays_in_its_range` |

원본과 다른 점: 원본(beatoraja)에는 일반 플레이의 배속이 없고 연습에만 FREQUENCY 가 있다. 원본의 연습은 차트를 다시 배치하지만(`BMSModelUtils.changeFrequency`) 여기서는 차트를 그대로 두고 시계를 늘리므로 리플레이·고스트·기록의 차트 시각이 그대로다. 녹색 숫자의 실시간 기준은 원본의 `changeFrequency` 가 BPM 을 곱한다는 전제로 맞췄고 이 전제는 원본 소스로 확인하지 못했다.

알려진 위험: 배속 소리의 음질, 플레이·결과 화면 노란 표지의 위치와 겹침(좁은 레이아웃, 긴 제목), F1 오버레이가 열두 행으로 커진 것은 실창에서 보지 않았다(QA 3.25 V6~V9, 테스트 부채 3.133~3.135). 설정 동기화(SYNC SETTINGS)로 다른 기기의 배율이 내려올 수 있으므로 선곡 표지와 F1·PLAY 메뉴로 현재 배율을 확인할 수 있다. 연습 FREQUENCY 판의 표시가 바뀐 것은 사용자 확인이 필요하다.
