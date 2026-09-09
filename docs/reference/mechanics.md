## rbms — Mechanics Reference (the reference implementation PLAY core, verbatim spec)

> "rbms가 반드시 재현해야 하는 것"의 단일 출처. 모든 수식·윈도우는 실제 레퍼런스 구현 소스(`jbms-parser.jar`, `<reference>/play/*`)에서 추출·검증되었다. 라인 참조는 `LR=LaneRenderer.java`, `JP=JudgeProperty.java`, `JM=JudgeManager.java`, `RT=RhythmTimerProcessor.java`. **시간 단위는 전부 마이크로초(µs, `i64`)로 고정한다.** 스킨은 범위 밖이지만 lane 사각형(`hu`/`hl`)은 렌더 추상화가 제공해야 한다.

---

### 0. 핵심 불변식 (가장 먼저 외울 것)

| 양 | 값 / 수식 | 의미 |
|---|---|---|
| 한 4/4 마디의 실제 시간 | `240_000_000 / bpm` µs | `RT:49`, `Section.getTimeLine` 모두 동일 상수 |
| 한 4/4 마디의 ms (hispeed 1.0) | `240000 / bpm` ms | green-number 분모 |
| section(measure index) | `double`, 마디당 정확히 `1.0` 증가 | BPM·시간과 독립 |
| 노트 절대 시각 | `note.time_us == owning_timeline.time_us` | `TimeLine.setNote`가 복사 |
| **판정 델타** | `dmtime = note.time_us − press_us` | `>0` = 노트가 미래 = **FAST/early**; `<0` = **SLOW/late** (`JM:673` `fast=(mfast>=0)`) |
| 화면 좌표계 | y는 아래가 0, 위(+y)가 먼 곳; `hl`=판정선, `hu`=레인 상단 | |

---

### 1. 차트 모델 (parse 결과 = 시간이 부여된 TimeLine 배열)

```
Model {
  mode: Mode, wavmap: Vec<String>, bgamap: Vec<String>,
  lnobj: i32, lnmode: i32, init_bpm: f64,
  timelines: Vec<TimeLine>,          // 절대 시각 오름차순 정렬
  md5: String, sha256: String,
}
TimeLine {
  time_us: i64,          // 절대 µs (getMicroTime)
  section: f64,          // 누적 마디 인덱스
  notes:   Vec<Option<Note>>,   // len = mode.key (플레이 레인)
  hidden:  Vec<Option<Note>>,   // 보이지 않는 키음 (채널 31-49)
  bgnotes: Vec<Note>,           // BGM/autoplay (채널 01)
  section_line: bool,    // 마디 시작선
  bpm: f64,              // 이 라인에서 유효한 BPM
  stop_us: i64,          // STOP 정지 µs (getMicroStop)
  scroll: f64,           // SCROLL 배율 (default 1.0)
  bga: i32, layer: i32,
}
Note {
  kind: Normal | Long{ ln_type, is_end, pair_idx } | Mine{ damage: f64 },
  wav: i32,              // wavmap 인덱스. <0 = 무음, ==wavcount = landmine 기본음
  start_us: i64,         // 키음 슬라이스 시작 (plain BMS=0)
  duration_us: i64,      // 슬라이스 길이 (plain BMS=0)
  time_us: i64, section: f64,
  layered: Vec<Note>,    // 동시 추가 키음 (addLayeredNote)
}
```

**Mode 정의** (`bms.model.Mode`): `BEAT_5K(key=6,p1,scratch={5})`, `BEAT_7K(key=8,p1,scratch={7})`, `BEAT_10K(key=12,p2,scratch={5,11})`, `BEAT_14K(key=16,p2,scratch={7,15})`, `POPN_5K(5,{})`, `POPN_9K(9,{})`, `KEYBOARD_24K(26,{24,25})`, `KEYBOARD_24K_DOUBLE(52,{24,25,50,51})`. `is_scratch_key(i)` = `i ∈ mode.scratch`.

---

### 2. 마디 → 절대 µs 타이밍 적분 (parse 시 1회 계산)

각 TimeLine은 `prev` 기반 체인으로 시각을 얻는다(`Section.getTimeLine` 바이트코드, `RT:44-49`와 동일):

```
scroll  = prev.scroll
bpm     = prev.bpm
time_us = prev.time_us + prev.stop_us + 240_000_000.0 * (section − prev.section) / bpm
```

- 첫(0번) 라인 시드: `bpm = init_bpm`, `scroll = 1.0`, `time_us = 0`.
- **STOP**: `prev.stop_us`를 다음 라인 시각에 더한다 → 재생 헤드가 그만큼 정지. (`STOP`은 시간에 영향, `SCROLL`은 시간에 무영향.)
- **measure-length(#xxx02, rate)**: 마디 내 분수 위치 `f`의 section은 `sectionnum + rate*f`, 다음 마디 `sectionnum`은 `+rate` 누적 → 마디 길이를 늘이거나 줄인다.
- **STOP 값 변환**: `#STOPxx`의 정수 S(192 = 한 마디)는 `stop_us = 240_000_000 * S / bpm`(해당 라인 BPM 기준)으로 저장.
- 적분 순서(마디당): ① section_line 라인 생성 → ② BPM/STOP/SCROLL 이벤트를 마디 내 분수 오름차순으로 병합(누적 체인 단조 보장) → ③ 노트 배치(`getTimeLine(sectionnum+rate*f)`). 모든 라인이 같은 prev 체인을 공유하므로 절대 시각이 단조 증가.

> **검증**: 알려진 BPM/마디 수의 차트로 마지막 TimeLine `time_us`를 손계산과 대조. STOP 한 개 넣고 정확히 `240_000_000*S/bpm` µs 밀리는지 확인.

---

### 3. 채널 → 레인 매핑

채널 `#xxxCC`의 CC는 `parseInt36(c0,c1)` (base-36, **대소문자 동일**: `A==a==10`, 그 외 `−1`). 그룹 base를 빼서 raw 레인 인덱스(0..17)를 얻고 모드별 배열로 실제 레인에 매핑:

```
CHANNELASSIGN_BEAT7 = [0,1,2,3,4,7,-1,5,6, 8,9,10,11,12,15,-1,13,14]   // BEAT_7K / BEAT_14K
CHANNELASSIGN_BEAT5 = [0,1,2,3,4,5,-1,-1,-1, 6,7,8,9,10,11,-1,-1,-1]    // BEAT_5K / BEAT_10K
CHANNELASSIGN_POPN  = [0,1,2,3,4,-1,-1,-1,-1, -1,5,6,7,8,-1,-1,-1,-1]   // POPN_9K
```
`−1` = 그 모드에서 무시. 7K에서 레인 7 = scratch(채널 '16'), 레인 0..6 = 7키.

**그룹 base** (`Section` 상수): P1 visible '11'=37, P2 '21'=73, P1 invisible '31'=109, P2 '41'=145, P1 long '51'=181, P2 '61'=217, P1 mine 'D1'=469, P2 'E1'=505, SCROLL 'SC'=1020. 제어 채널: 01=BGM→`bgnotes`, 02=measure-rate, 03=BPM(hex), 04=BGA-base, 06=POOR-layer, 07=BGA-layer, 08=BPM-extend(#BPMxx), 09=STOP. invisible→`hidden[lane]`, mine→`Mine` in `notes[lane]`.

---

### 4. 노트 타입 (judge 동작 차이)

- **Normal**: 단타. NOTE 윈도우로 판정.
- **LN (release-judged, LNTYPE 또는 type=1)**: head를 NOTE 윈도우로 임시 판정(`judgeVanish[judge]`면 provisional로 저장, head는 아직 카운트 안 함, laser color=8). release(key-up)를 LN_END 윈도우로 판정, 최종 = `max(endJudge, startJudge)`이며 더 나쁜 DURATION 사용. end judge ≥ BAD 이고 `dmtime>0`(너무 일찍 뗌)이면 `releasemargin`(longnoteMargin) 경과 후 finalize.
- **CN (charge, type=2)**: head 소비, release를 end 윈도우로 판정. end held until cnendmjudge.
- **HCN (hell charge, type=3)**: CN + 게이지 틱. 누르는 동안 `hcnmduration=200_000µs(200ms)`마다 `gauge.update(GREAT, 0.5)`, 안 누르면 200ms마다 `gauge.update(BAD, 0.5)` 데미지.
- **Mine**: 누르면 `gauge.addValue(−damage)` 직접(모든 게이지). 판정/콤보 영향 없음.
- **BSS(백스핀 스크래치/LONGSCRATCH)**: 반대 방향 scratch 키-down으로 종료(`scnendmjudge`). 같은 키 중도 release는 judge==4 영역에서만 인정.

`judgeVanish[]`(head가 즉시 사라지나): 7K/5K/KB = `{T,T,T,T,T,F}`; PMS = `{T,T,T,F,T,F}`(PMS는 BAD에서 안 사라짐).

---

### 5. 노트 화면 좌표 + Hi-Speed + Green Number

**시간 정규화** (`LR:252-258`):
```
time_ms   = (TIMER_PLAY on ? clock_ms − timer(TIMER_PLAY) : 0) + judgetiming_offset_ms
microtime = time_ms * 1000            // 현재 재생 위치 µs
```

**프레임 상수** (`LR:261-280`):
```
hispeed = playconfig.hispeed                          // 0.01..20, default 1.0
nbpm,nscroll = microtime 이하 마지막 TimeLine의 bpm,scroll   // 판정선 아래 헤드 상태
region  = nscroll>0 ? (240000/nbpm/hispeed)/nscroll : 0     // ms; green-number 기반
hu = lane.y + lane.height                             // 레인 상단(먼 곳); 이 너머 컬링
hl = enablelift ? lane.y + lane.height*lift : lane.y   // 판정선(노트 도착)
rxhs = (hu − hl) * hispeed                             // 한 마디의 화면 픽셀 길이 (hispeed 포함!)
currentduration = round(region * (1 − lanecover))      // 표시 green number
```

**Y 적분** (`LR:472-483`) — `pos` 커서부터 `y<=hu` 동안 전진, `y=hl`에서 시작:
```
for tl = timelines[i] (i = pos..):
  if tl.time_us >= microtime:                          // 미래 끝점
    if i > 0:
      prev = timelines[i-1]
      dSection = tl.section − prev.section
      if prev.time_us + prev.stop_us > microtime:       // STOP 중 → 전 구간 고정 높이
        y += dSection * prev.scroll * rxhs
      else:                                             // 부분 구간 → 미경과 분수 비례
        y += dSection * prev.scroll
             * (tl.time_us − microtime)
             / (tl.time_us − prev.time_us − prev.stop_us)
             * rxhs
    else:  // i == 0
      y += tl.section * (tl.time_us − microtime) / tl.time_us * rxhs
  dsty = y + offsetY − offsetH/2                         // 최종 노트 픽셀 Y
  // tl.time_us >= microtime 인 노트만 그림(판정선 위/도착)
```

**등가 닫힌형** (now~노트 사이 BPM/SCROLL/STOP 변화 없을 때):
```
Δt_us = note.time_us − microtime   (≥0)
y_note = hl + Δt_us * bpm * scroll * hispeed * (hu−hl) / 240_000_000
픽셀/µs = bpm * scroll * hispeed * (hu−hl) / 240_000_000
```
즉 노트는 위 속도로 내려와 `Δt_us=0`에서 정확히 `y=hl`(판정선)에 도착, `y>hu`에서 컬링. (`LR:600-601` PMS miss-poor fallback이 `pixels/sec = rxhs*BPM/240`로 명시.)

**Green Number(노트 가시 ms)** (`LR:271,280`, `IntegerPropertyFactory:56-57`):
```
greennumber = round( (240000/bpm/hispeed) / scroll * (1 − lanecover) )
```
화면 표시값은 bpm 선택(now/main/min/max)과 3/5 sub-lane용 `*0.6` 옵션 존재.

**Fixed Hi-Speed로 green number 고정** (`LR:193-197`):
```
hispeed = (2400 / (basebpm/100) / duration) * (1 − (enablelanecover ? lanecover : 0))
        = (240000/basebpm/duration) * (1 − lanecover)
```
`fixhispeed ∈ {OFF, STARTBPM, MAXBPM, MAINBPM(default), MINBPM}` → `basebpm` 선택. `mainbpm` = 노트가 가장 많은 BPM(`LR:121-135`). fixed 모드에서 사용자는 `duration`(고정 green number, default 500ms)을 편집, hispeed는 자동 유도. lanecover 변경 시도 `resetHispeed(basebpm)` 재실행. `hispeedAutoAdjust`면 현재 BPM에 다시 핀(`resetHispeed(getNowBPM())`). OFF면 raw hispeed 직접 편집(±hispeedmargin, default 0.25), green number는 BPM 따라 변동.

**Lane cover/Lift/Hidden/Constant**: lanecover 0..1(default 0.2)는 `(1−lanecover)`로 green number 축소 + fixed 모드에서 hispeed에 접힘. lift는 `hl` 상승 → `(hu−hl)` 축소. hidden은 판정선 근처 알파 커버만(좌표 무영향). CONSTANT는 `microtime + duration*1000` µs 너머 노트 페이드(가시 클립, 좌표 수식 동일).

> **검증**: hispeed 2배→화면 노트 속도 2배·가시시간 1/2. fixed 모드에서 BPM 변동 차트의 green number가 일정 유지. STOP 구간에서 노트 정지.

---

### 6. 판정 윈도우 (µs, judgerank=100 기준)

`{LATE하한(음수), EARLY상한(양수)}` 배열. 7K(SEVENKEYS, default) NOTE:
```
PG[-20000,+20000] GR[-60000,+60000] GD[-150000,+150000] BD[-280000,+220000(비대칭)] MS[-150000,+500000]
```
7K SCRATCH: `PG±30000 GR±70000 GD±160000 BD[-290000,+230000] MS[-160000,+500000]`.
7K LN_END: `PG±120000 GR±160000 GD±200000 BD[-280000,+220000]`.
7K LS_END(BSS): `PG±130000 GR±170000 GD±210000 BD[-290000,+230000]`.
5K NOTE: `PG±20000 GR±50000 GD±100000 BD±150000 MS[-150000,+500000]`(대칭).
PMS NOTE: `PG±20000 GR±50000 GD±117000 BD±183000 MS[-175000,+500000]`; scratch/LN/LS 빈 배열, `longnoteMargin=200000`.
KEYBOARD NOTE: `PG±30000 GR±90000 GD±200000 BD[-320000,+240000] MS[-200000,+650000]`; LN_END `PG[-160000,+25000] GR[-200000,+75000] GD[-260000,+140000] BD[-320000,+240000]`(비대칭, late 관대).

**스캔 경계**: `mjudgestart = min 모든 judge[i][0]`, `mjudgeend = max 모든 judge[i][1]` (NOTE+SCRATCH 합쳐서). 7K → `[-290000, +500000]`.

**judgerank 스케일링** (`JP:168-214`): judgerank는 %. `judge[i][j] = fixjudge[i] ? org : org * judgerank/100`. `#RANK`→judgerank% 변환(`BMSPlayerRule:57-71`): BMS_RANK 인덱스 0..4 → `NORMAL.judgerank={25,50,75,100,125}`(VERYHARD/HARD/NORMAL/EASY/VERYEASY), 범위 밖 → 인덱스2(=75). PMS judgerank `{33,50,70,100,133}`. BMSON_JUDGERANK>0 → raw % 그대로(100=위 표 그대로). `fixjudge`: NORMAL `{F,F,F,F,T}`(MS만 고정), PMS `{T,F,F,T,T}`(PG·BD·MS 고정). 스케일 후 단조성 클램프(고정 이웃 사이에 중첩). custom judge-window-rate는 judgerank 후 `*rate[i]/100`, BD 윈도우 내로 클램프. 코스 NO_GREAT→GR&GD rate=0, NO_GOOD→GD rate=0.

---

### 7. 노트 매칭 알고리즘 (`JM:393-506`)

레인당 시간정렬 `Note[]` + 이동 커서. key-down at `pmtime`:
1. scratch면 `smjudge` 아니면 `nmjudge` 선택. base 커서부터 순회: `dmtime = note.time_us − pmtime`. `dmtime >= mjudgeend` → break(미래). `dmtime < mjudgestart` → skip(과거). MineNote·LN_END skip.
2. **후보 선택**: 현 타겟 `tnote`를 새 노트로 교체하는 조건 = `tnote==null OR tnote.state!=0 OR algorithm.compare(tnote, judgenote, pmtime, mjudge)`. `JudgeAlgorithm`:
   - `Combo`: t2 미판정 AND `t1.micro < ptime+table[2][0]` AND `t2.micro <= ptime+table[2][1]` (콤보 유지 노트 우선, GOOD 윈도우 기준).
   - `Duration`: `|t1−ptime| > |t2−ptime|` AND t2 미판정 (시간상 최근접 우선).
   - `Lowest`: 항상 false (첫/최하 노트 유지).
   - `Score`: Combo와 유사하나 table[1](GREAT) 사용.
   - default = `{Combo, Duration, Lowest}` (`PlayConfig.getJudgetype()`로 선택).
3. **판정 계산** (`JM:411-434`): 후보가 이미 판정됨(state!=0) → `dmtime`이 table[4](empty-poor) 내면 5(MS) 아니면 6(skip). 미판정 → index 0..len 선형 스캔, `dmtime ∈ [table[j][0],table[j][1]]`인 j; `judge = (judge>=4 ? judge+1 : judge)` (0=PG,1=GR,2=GD,3=BD, index4(MS매치)→5(empty MISS)); 무매치 → `judge = len(>=6)`. `judge<4`인 후보는 첫 번째이거나 더 가까울 때만 `tnote`로 채택; `judge>=6`이면 `tnote` 클리어.
4. **見逃しPOOR(놓침)** (`JM:612-649`): 미판정 NormalNote/LN-head가 `note.time_us + mjudge[3][0]`(BAD-late) 지나면 강제 4(POOR).
5. **빈 입력(空POOR)**: `tnote==null`이면 기본 7K/5K/KB 규칙에선 판정 카운터 증가 없음(가까운 통과 노트 키음만 재생). PMS는 `MissCondition.ONE`으로 노트당 1회 empty-POOR가 콤보 브레이크.

---

### 8. 게이지 / 콤보 / EX-Score

**게이지** (`GrooveGauge`, `GaugeProperty`): `gauge.update(judge, rate)`가 raw judge 인덱스 `{0:PG,1:GR,2:GD,3:BD,4:PR,5:MS}`로 value[] 증감. 음수 증분은 guts 테이블로 손실 완화. value≤0이면 사망 고정. 8(+1) 타입: ASSISTEASY/EASY/NORMAL/HARD/EXHARD/HAZARD/CLASS/EXCLASS/EXHARDCLASS. 7K NORMAL: `init=20 border=80 value={+1.0,+1.0,+0.5,-3.0,-6.0,-2.0} modifier=TOTAL`. HARD: `init=100 border=0 {+0.15,+0.12,+0.03,-5.0,-10.0,-5.0} LIMIT_INCREMENT guts={{10,0.4}..{50,0.8}}`. HAZARD: BAD/POOR 즉사(-100). 클리어 = `value>0 && value>=border`. modifier: TOTAL은 양수 증분에 `total/totalNotes` 스케일; LIMIT_INCREMENT는 `clamp(min(0.15,(2*total-320)/totalNotes),0)/0.15`; MODIFY_DAMAGE는 음수에 차트 TOTAL 기반 fix 테이블 스케일.

**콤보** (`JM:681-690`): `combocond[judge] && judge<5` → combo++; `!combocond[judge]` → combo=0. `combocond`: 7K/KB `{T,T,T,F,F,T}`(PG/GR/GD 유지, BD/PR 브레이크, MS 유지), 5K/PMS `{T,T,T,F,F,F}`(MS도 브레이크). GOOD은 절대 콤보 안 깸, BAD·POOR는 항상 깸.

**FAST/SLOW**: `fast = (mfast>=0)`, `mfast=dmtime`. judge별 e*(fast)/l*(slow) 분리 카운트. `recentJudges` 링버퍼(100)에 `mfast/1000` ms 저장(judge<4만).

**EX-Score** (`ScoreData:405-406`): `EXSCORE = (epg+lpg)*2 + (egr+lgr)` (PG=2, GR=1, 그 외 0). 랭크: AAA=24/27, AA=21/27, A=18/27 (×EX/max). maxcombo, minbp 추적. `Note.setState(judge+1)`로 소비 표시(state 0=미판정).

---

### 9. 키음 트리거 (`AbstractAudioDriver`)

각 Note의 `wav` = wavmap 0-based 인덱스(`<0` 무음, `==wavcount` landmine 기본음). `setModel`: 모든 TimeLine 순회, 레인별 `note + layered`, `hidden`, `bgnotes`를 `wav`별 `IntMap<List<Note>>`에 등록(`(start_us, duration_us)`로 dedup). `start==0 && duration==0` → 전체파일 키음(`wavmap[id]`), 아니면 슬라이스(`slicesound[id]`, bmson 音切り). 재생: `play(note,vol,pitch)`가 layered까지 재생. **믹서 채널 = `id*256 + pitchShift + 128`** (wav+pitch당 안정 voice, `stop(id,channel)`로 이전 인스턴스 컷 = retrigger cutoff). 판정 시 `keysound.play(note,vol,0)`; BGM(채널01)은 `AutoplayThread`가 `TimeLine.time_us` 도달 시 자동 재생. plain BMS는 start/duration 항상 0이라 wavmap만 사용.

---

### 10. 입력/판정 클럭 (포팅 시 고치는 부분 — 상세는 stackRef)

레퍼런스 구현는 `System.nanoTime()/1000`(µs) 단일 단조 클럭에서 3 스레드: render(vsync, `timer.update`로 `TIMER_PLAY` 전진), input poll(~1kHz, `Gdx.input.isKeyPressed` 샘플링 → ~1ms 양자화 지터), judge(µs 변화마다 spin, `dmtime=note.time−press` 순수 µs). 판정은 프레임 시간과 무관(이미 올바름). **남은 결함**: ① 입력 poll ~1ms 양자화, ② 재생 위치가 nanoTime 기반(하드웨어 오디오 클럭 없음) → A/V vs judge 드리프트. rbms는 (1) 오디오 디바이스 클럭을 마스터로, (2) OS 타임스탬프 이벤트 입력으로 교체한다(stackRef 참조).
