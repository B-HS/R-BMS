# rbms-judge vs beatoraja 판정/게이지 함수 단위 대조

조사 일시: 2026-09-09 / 조사 상한 15분. 읽기 전용으로만 수행(파일 수정 없음).

대상 파일
- rbms: `crates/rbms-judge/src/{windows.rs,matcher.rs,gauge.rs,lib.rs}`, `crates/rbms-play/src/lib.rs`, `apps/rbms-player/src/{settings.rs,app_select.rs,app_input.rs,app_play.rs,format.rs}`
- beatoraja: `src/bms/player/beatoraja/play/{JudgeProperty,JudgeManager,JudgeAlgorithm,BMSPlayerRule,GrooveGauge,GaugeProperty}.java`, `PlayerConfig.java`, `PlayConfig.java`

---

## 0. 인덱스 규약 (가장 중요한 전제)

beatoraja의 판정 인덱스는 두 계열이 있다.

| 계열 | 인덱스 | 의미 |
|---|---|---|
| 윈도우 배열(`JudgeProperty.note` 등) | 0 PG, 1 GR, 2 GD, 3 BD, 4 MS | `JudgeWindow.getJudge`가 반환하는 5쌍 |
| 판정 코드(`updateMicro(judge)`) | 0 PG, 1 GR, 2 GD, 3 BD, **4 POOR(見逃し)**, **5 MS(空POOR)** | `judge = (judge >= 4 ? judge + 1 : judge)` (`JudgeManager.java:394`) |

게이지 `value[]` / `combo[]` / `judgeVanish[]` 배열은 **판정 코드 계열**(PG,GR,GD,BD,PR,MS)이다 (`GaugeProperty.java:152` 주석, `JudgeProperty.java:78`).

rbms `Judge` enum(`crates/rbms-judge/src/lib.rs:12-19`)은 `PerfectGreat, Great, Good, Bad, Poor, Miss` → 인덱스 4=Poor, 5=Miss. 즉 **rbms의 `Judge::Miss`는 beatoraja의 MS(空POOR) 슬롯**이고, beatoraja의 見逃しPOOR(코드 4)는 rbms의 `Judge::Poor` 슬롯에 대응한다. 이 매핑이 뒤집혀 있는 곳이 아래 D-01이다.

---

## 1. 판정 윈도우 테이블 대조

### 1.1 NOTE 윈도우 (µs, `{LATE 하한, EARLY 상한}`)

| 모드 | beatoraja | rbms | 판정 |
|---|---|---|---|
| SEVENKEYS NOTE | PG ±20000 / GR ±60000 / GD ±150000 / BD (-280000,220000) / MS (-150000,500000) (`JudgeProperty.java:21`) | 동일 (`windows.rs:20-26` `SEVENKEY_NOTE`) | **identical** |
| FIVEKEYS NOTE | PG ±20000 / GR ±50000 / GD ±100000 / BD ±150000 / MS (-150000,500000) (`JudgeProperty.java:12`) | 없음. `note_for_mode`가 BEAT_5K를 SEVENKEY 표로 보냄 (`windows.rs:60-65`) | **divergent** — GR 50k→60k, GD 100k→150k, BD ±150k→(-280k,220k). 5K가 7K보다 크게 관대해짐 |
| PMS NOTE | PG ±20000 / GR ±50000 / GD ±117000 / BD ±183000 / MS (-175000,500000) (`JudgeProperty.java:33`) | `POPN_NOTE`가 7K 값 그대로 복제 (`windows.rs:41-47`, 주석에 "pending verified" 명시) | **divergent** — GR/GD/BD/MS 전부 다름 |
| KEYBOARD(24K) NOTE | PG ±30000 / GR ±90000 / GD ±200000 / BD (-320000,240000) / MS (-200000,650000) (`JudgeProperty.java:44`) | 없음 (모드 자체 미대응, `note_for_mode` fallback = 7K) | **missing** |
| SCRATCH NOTE | SEVENKEYS 전용 별도 배열 PG ±30000 / GR ±70000 / GD ±160000 / BD (-290000,230000) / MS (-160000,500000) (`JudgeProperty.java:22`), FIVEKEYS도 별도(`:13`) | 스크래치 전용 윈도우 개념 없음. 스크래치 레인도 `SEVENKEY_NOTE` 사용 (`matcher.rs:180 press`는 lane 구분만) | **missing** — 스크래치가 키보다 10ms 관대해야 하는데 동일 |

### 1.2 LN 종단 윈도우

| 항목 | beatoraja | rbms | 판정 |
|---|---|---|---|
| SEVENKEYS LONGNOTE_END | 4쌍만: PG ±120000 / GR ±160000 / GD ±200000 / BD (-280000,220000) (`JudgeProperty.java:23`). MS 쌍 없음 → `getJudge`가 4를 반환하면 코드 5(空POOR) | PG ±120000 / GR ±150000 / GD ±200000 / BD ±250000 / MS (-250000,500000) (`windows.rs:30-36`) | **divergent** — GR 160k vs 150k, BD (-280k,220k) vs ±250k, 존재하지 않는 MS 쌍 추가 |
| FIVEKEYS LONGNOTE_END | PG ±120000 / GR ±150000 / GD ±200000 / BD ±250000 (`JudgeProperty.java:14`) | rbms `SEVENKEY_LN_END`가 실은 **FIVEKEYS 값**과 PG/GR/GD/BD가 일치 | 7K 표로 라벨링된 값이 실제로는 5K 표 |
| PMS LONGNOTE_END | PG ±120000 / GR ±150000 / GD ±217000 / BD ±283000 (`JudgeProperty.java:35`) | `POPN_LN_END`가 7K(=5K) 값 복제 (`windows.rs:51-57`) | **divergent** |
| LONGSCRATCH_END | SEVENKEYS 전용 배열 (`JudgeProperty.java:25`), KEYBOARD은 비대칭 (`:47`) | 없음 | **missing** |
| `longnoteMargin` (늦은 릴리스 유예) | SEVENKEYS/FIVEKEYS = **0**, PMS = **200000**, KEYBOARD = 0 (`JudgeProperty.java:24,15,36,48`) | `const LN_MARGIN: i64 = 200_000` 전 모드 고정 (`matcher.rs:7`) | **divergent** — 7K에 PMS 마진을 적용. LN 미릴리스 확정이 200ms 늦음 |
| `longscratchMargin` | 전 모드 0 | 없음 | missing |

### 1.3 judgerank / judgewindowrate

| 항목 | beatoraja | rbms | 판정 |
|---|---|---|---|
| `#RANK` → judgerank | `JudgeWindowRule.NORMAL.judgerank[rank][1]` = {25,50,75,100,125}, 범위 밖은 index 2(75) (`BMSPlayerRule.java:63-67`, `JudgeProperty.java:225`) | `rank_to_judgerank` = [25,50,75,100,125], 범위 밖은 **clamp(0,4)** (`windows.rs:104-107`) | **divergent(경미)** — 범위 밖 처리가 clamp(→25/125) vs 75 |
| `#DEFEXRANK` | `judgerank * 75 / 100` (`BMSPlayerRule.java:65`) | 파서가 `#DEFEXRANK`를 아예 안 읽음 (`crates/rbms-parser/src/lib.rs:245`는 `"RANK"`만) | **missing** |
| bmson judgerank | 그대로 사용, ≤0이면 100 (`BMSPlayerRule.java:66`) | 미확인(본 조사 범위 밖) | 미조사 |
| PMS windowrule | `PMS`: judgerank 배열 {100,33,33,100,100}…{100,133,133,100,100}, `fixjudge`={true,false,false,true,true} → PG/BD/MS는 rank 무관 고정, GR/GD만 스케일 (`JudgeProperty.java:207`) | `scaled()`가 PG/GR/GD/BD를 일률 스케일, MS만 고정 (`windows.rs:76-81`) — NORMAL 규칙만 존재 | **missing** — PMS 규칙 자체 없음 |
| NORMAL windowrule `fixjudge` | {false,false,false,false,true} → MS만 고정 (`JudgeProperty.java:206`) | `scaled()`가 동일하게 MS만 고정 | **identical** |
| `judgeWindowRate` (커스텀 판정) | PG/GR/GD **3개만** 각각 비율 적용 + MS 상한 클램프 + 단조성 클램프 (`JudgeProperty.java:255-266`), BD는 건드리지 않음 | `set_judge_rate`가 `scaled(rank*rate/100)`로 **PG~BD 전부** 비율 적용, 클램프 없음 (`crates/rbms-play/src/lib.rs:128-134`) | **divergent** — BD까지 넓어짐, MS/단조 클램프 없음 |
| `fixmin/fixmax` 클램프 | `create()`에서 고정 인덱스 기준 상·하한 클램프 (`JudgeProperty.java:236-253`) | 없음 | **missing** (PMS 규칙이 없어 현재는 무해) |

---

## 2. 판정 로직 대조

| 항목 | beatoraja | rbms | 판정 |
|---|---|---|---|
| 노트 선택 알고리즘 | `JudgeAlgorithm` 3종 선택 가능: `Combo`(기본, GOOD 이상으로 아래 노트를 못 잡으면 위 노트로), `Duration`(시간차 최소), `Lowest`(항상 최하단). `PlayConfig.judgetype` 기본 `Combo` (`JudgeAlgorithm.java:16-38`, `PlayConfig.java:103`) | 후보 중 `|dm|` 최소 = `Duration` 고정 (`matcher.rs:205-220`) | **divergent** — 기본 알고리즘(Combo)과 다르고 선택지도 없음 |
| 후보 게이트 | `dmtime >= mjudgeend` break, `dmtime < mjudgestart` continue (`JudgeManager.java:376-382`) | `gate_early = ms.1`, `gate_late = bd.0` (`matcher.rs:200-201`) | **identical**(7K 기준. mjudgestart/end는 note·scratch 양쪽 최솟/최댓값이므로 스크래치 표가 생기면 달라짐) |
| 이미 판정된 노트 재타격 | `judgenote.getState() != 0`이면 MS 범위면 코드 5, 아니면 6(무시) (`JudgeManager.java:390-392`) | `!n.judged && !n.holding`으로 후보에서 제외만 (`matcher.rs:208`) | **divergent(경미)** — 판정 끝난 노트 위 空POOR가 안 남 |
| 空POOR(코드 5) | `judgeVanish[5]=false`(노트 소실 안 함), `combo[5]=true`(콤보 유지) SEVENKEYS (`JudgeProperty.java:28,30`). `score.addJudgeCount(5,...)`로 집계 | 별도 `empty_poor` 카운터, 노트 미소비, 콤보 유지, `gauge.update(Judge::Miss)`(=MS 슬롯) (`matcher.rs:229-237`) | **동작 identical**, 다만 rbms는 `counts[]`에 안 넣고 별도 필드로 관리(IR `empty_poor` 확장 필드) |
| `MissCondition.ONE` (PMS: 노트당 空POOR 1회) | `miss == ONE`이면 이미 PlayTime 있는 노트에 코드4 재부여 금지 + 후보 필터 (`JudgeManager.java:386-388`, `:645`) | 없음 | **missing** (PMS 미지원의 일부) |
| combo 규칙 | `combo[]` = SEVENKEYS {T,T,T,F,F,T}, FIVEKEYS/PMS {T,T,T,F,F,F} (`JudgeProperty.java:28,18,38`). `combocond[judge] && judge<5`일 때만 증가 | PG/GR/GD 증가, BD/Poor/Miss 리셋 (`matcher.rs:334-360`). 空POOR는 별도 경로라 콤보 유지 | **identical (7K)**, 5K/PMS는 空POOR가 콤보를 끊어야 하는데 rbms는 항상 유지 → divergent |
| `judgeVanish` | SEVENKEYS {T,T,T,T,T,F} — BD/POOR도 노트를 소실시킴 (`JudgeProperty.java:30`). PMS는 {T,T,T,F,T,F}(BD가 노트를 소실 안 함) (`:40`) | BD는 노트 소비, MS는 미소비 (`matcher.rs:229,250`) | **identical (7K)**, PMS divergent |
| 見逃しPOOR(자동 MISS) 타이밍 | `note.getMicroTime() < mtime + window.getTime(noteType,3,false)` = `note - now < bd.LATE` (`JudgeManager.java:591`) | `n.head_us - now_us < miss_bound`, `miss_bound = windows.bd.0` (`matcher.rs:282,306`) | **identical** |
| 見逃しPOOR 판정 코드 | **4 (POOR)** → 게이지 `value[4]` (`JudgeManager.java:595`) | `Judge::Miss` = 인덱스 5 → 게이지 `deltas[5]`(= beatoraja MS 값) (`matcher.rs:311`) | **divergent (D-01, 최중요)** |
| CN/HCN 머리 | `updateMicro` 즉시 호출(판정 카운트) + `processing` 설정 (`JudgeManager.java:460-471`) | `is_charge`면 press 시 `apply(judge)` (`matcher.rs:236-247`) | **identical** |
| CN/HCN 종단 | 릴리스 윈도우 판정으로 확정, head로 capping 안 함. 단 `judge>=3 && dmtime>0`이면 **deferral**(재홀드로 회복 가능) (`JudgeManager.java:499-520`) | 릴리스 즉시 확정, deferral 없음 (`matcher.rs:258-275`) | **divergent** — 이른 릴리스 후 재누름 회복 미구현(문서에 기록됨) |
| LN 종단 | `judge = max(judge, lnstartJudge)`, `|lnstartDuration| > |dmtime|`이면 dmtime 교체, `judge>=3 && dmtime>0`이면 deferral (`JudgeManager.java:534-548`) | `worse(head, end)` (`matcher.rs:266`), dmtime 교체·deferral 없음 | **부분 divergent** — worse 캡은 동일, dmtime 교체/deferral 없음 |
| LN 미릴리스 | `processing.getMicroTime() < mtime`이면 `lnstartJudge`로 확정 (`JudgeManager.java:575-580`) | `now > end + LN_MARGIN(200ms)`일 때 `ln_end.judge(end-now)`로 확정, LN은 worse(head,end) (`matcher.rs:288-297`) | **divergent** — beatoraja 7K는 마진 0이고 **머리 판정으로 확정**, rbms는 200ms 뒤 종단 윈도우 판정 |
| BSS / MSS (스크래치 LN) | `sckey[]` 추적, 다른 스크 키로 종단, 중간 떼기 무시 (`JudgeManager.java:359-372`, `:502-511`) | 없음 | **missing** |
| 스크래치 회전 | 좌/우 두 키 + `sckey` 전환 로직 | 스크래치를 일반 레인처럼 처리 | **missing** |
| 지뢰(mine) 데미지 | 눌린 상태로 지뢰 통과 시 `gauge.addValue(-damage)` + 키음 (`JudgeManager.java:243-248`) | 지뢰를 판정 대상에서 완전 제외, 데미지 없음 (`matcher.rs:145`, `crates/rbms-play/src/lib.rs:66`) | **missing** |
| HCN 연속 게이지 | `hcnmduration=200000`µs마다 홀드 중 `gauge.update(1, 0.5)`, 비홀드 중 `gauge.update(3, 0.5)` (`JudgeManager.java:305-345`) | 없음 (문서상 Phase 7 잔여) | **missing** |
| fast/slow 집계 | `score.addJudgeCount(judge, mfast >= 0, 1)` — **dm==0은 EARLY** (`JudgeManager.java:648`) | `dm > 0`만 early, `dm <= 0`은 late (`matcher.rs:376-382`) | **divergent(경미)** — 정확히 0µs일 때 반대로 집계 |
| autoplay 처리 | note 통과 시 `updateMicro(...,0,0,true)` = PG (`JudgeManager.java:252-283`) | 동일하게 PG 부여 (`crates/rbms-play/src/lib.rs:48-73`) | identical |
| `notesDisplayTimingAutoAdjust` | 판정 ≤GD & |mfast|≤150ms일 때 judgetiming 자동 보정 (`JudgeManager.java:690-699`) | `auto_offset` + `calibrated_offset`(`app_play.rs:364-366`) 존재, 공식은 다름(평균 기반) | divergent(경미) |

---

## 3. 게이지 대조

### 3.1 모드별 게이지 세트

| 모드 | beatoraja | rbms |
|---|---|---|
| SEVENKEYS | 9종 (ASSIST_EASY/EASY/NORMAL/HARD/EXHARD/HAZARD/**CLASS/EXCLASS/EXHARDCLASS**) (`GaugeProperty.java:22-31`) | 6종만 (`gauge.rs:4-11`) — CLASS 계열 3종 **missing** |
| FIVEKEYS | `*_5` 9종 (`GaugeProperty.java:11-20`) | 없음 (7K 값 사용) — **missing** |
| PMS | `*_PMS` 9종, max 120·min 30 등 별도 (`GaugeProperty.java:32-41`) | 없음 — **missing** |
| KEYBOARD | `*_KB` 9종 | 없음 — **missing** |
| LR2 | `*_LR2` 9종 (rule NORMAL vs LR2 분기, `BMSPlayerRule.java:21`) | 없음 — **missing** |

### 3.2 SEVENKEYS 게이지 수치 (rbms가 구현한 6종)

| 게이지 | beatoraja (modifier, min, max, init, border, value[6], guts) | rbms | 판정 |
|---|---|---|---|
| ASSIST_EASY | TOTAL,2,100,20,60,{1,1,0.5,-1.5,-3,-0.5},[] (`GaugeProperty.java:88`) | 동일 (`gauge.rs:49`) | identical |
| EASY | TOTAL,2,100,20,80,{1,1,0.5,-1.5,-4.5,-1},[] (`:89`) | 동일 (`gauge.rs:50`) | identical |
| NORMAL | TOTAL,2,100,20,80,{1,1,0.5,-3,-6,-2},[] (`:90`) | 동일 (`gauge.rs:51`) | identical |
| HARD | LIMIT_INCREMENT,0,100,100,0,{0.15,0.12,0.03,-5,-10,-5}, guts {10,.4}{20,.5}{30,.6}{40,.7}{50,.8} (`:91`) | 동일 (`gauge.rs:52`, `HARD_GUTS` `:43`) | identical |
| EXHARD | LIMIT_INCREMENT,0,100,100,0,{0.15,0.06,0,-8,-16,-8},[] (`:92`) | 동일 (`gauge.rs:53`) | identical |
| HAZARD | null,0,100,100,0,{0.15,0.06,0,-100,-100,-10},[] (`:93`) | 동일 (`gauge.rs:54`) | identical |

### 3.3 게이지 동작

| 항목 | beatoraja | rbms | 판정 |
|---|---|---|---|
| TOTAL modifier | `f > 0 ? f * total / totalNotes : f` (`GrooveGauge.java:262`) | 동일, 단 `total<=0`이면 **200.0** fallback (`gauge.rs:77-86`) | **divergent** — beatoraja는 `BMSPlayerRule.validate`가 미리 `max(260, 7.605n/(0.01n+6.5))`로 채움 (`BMSPlayerRule.java:88-93`). rbms는 `meta.total`에 그 공식을 적용하지 않고(`crates/rbms-chart/src/lib.rs:196`은 헤더 값 or 0.0) 200을 씀. 같은 공식이 rbms-chart:406에 있으나 노트밀도 계산 전용 |
| LIMIT_INCREMENT | `pg = clamp((2*total-320)/notes, 0, 0.15)`, `f>0`이면 `f *= pg/0.15` (`GrooveGauge.java:266-273`) | 동일 (`gauge.rs:87-93`) | identical |
| MODIFY_DAMAGE | TOTAL/노트수 기반 데미지 증폭 (fix1table 10단계 + fix2 루프) (`GrooveGauge.java:277-297`). FIVEKEYS HARD/EXHARD, LR2 HARD/EXHARD가 사용 | 없음 (`Modifier` enum에 Total/LimitIncrement/None만, `gauge.rs:27-31`) | **missing** (5K/LR2 게이지 미지원이라 현재는 미발현) |
| guts (하한 감소 완화) | `inc<0`일 때 `value < gut[0]`이면 `inc *= gut[1]`, 첫 매치에서 break (`GrooveGauge.java:229-236`) | 동일 (`gauge.rs:105-112`) | identical |
| 사망 후 동결 | `setValue`가 `this.value > 0f`일 때만 clamp 적용 (`GrooveGauge.java:213-217`) | `update`가 `value <= 0.0`이면 early return (`gauge.rs:101-103`) | identical(동등) |
| 클리어 판정 | `value > 0 && value >= border` (`GrooveGauge.java:243`) | 동일 (`gauge.rs:124-126`) | identical |
| 게이지 변경 시 전 게이지 병렬 갱신 | `GrooveGauge.update()`가 **9개 게이지 전부** 갱신, 선택 타입만 표시 (`GrooveGauge.java:56-60`) | 단일 `Gauge` 인스턴스만 (`matcher.rs:56`) | **divergent** — 결과 화면에서 "다른 게이지였다면" 표시 / GAS 불가 |
| 클리어 램프 | `ClearType.getClearTypeByGauge(gaugetype)` (`ClearType.java:51`) | `clear_lamp`가 Hazard를 ExHard로 매핑 (`gauge.rs:147-153`) | 미확인(ClearType 열거 상세 미조사) |
| EX스코어 | 2·PG + 1·GR | 동일 (`matcher.rs:336,341`) | identical |
| DJ 랭크 | 미조사 | `format.rs:74 rank_label`은 `#RANK` 표기용이며 DJ랭크 계산과 별개 | 미조사 |

---

## 4. 사용자 설정 항목 대조 (질문 3)

beatoraja `PlayerConfig`/`PlayConfig` 중 판정·게이지에 영향 있는 항목:

| beatoraja 설정 | 위치 | rbms 대응 | 상태 / 효력 |
|---|---|---|---|
| `judgetype` (JudgeAlgorithm Combo/Duration/Lowest) | `PlayConfig.java:103` | 없음 (Duration 고정) | **미노출·미구현**. 밀집 배치·연타 구간의 노트 선택이 달라짐 |
| `judgetiming` (판정 오프셋 ms) | `PlayerConfig.java:82` | `offset_ms` (`settings.rs:20`, UI `app_select.rs:900,936`, 적용 `app_input.rs:217`) | **구현**. 범위 ±200ms/5ms step |
| `customJudge` + key/scratch `JudgeWindowRate{PG,GR,GD}` (기본 400/400/100) | `PlayerConfig.java:124-130` | `judge_rate` 단일 값 (`settings.rs:22`, UI 12번) | **부분 구현** — 판정별 분리 없음, 스크래치 분리 없음, 적용 범위가 BD까지(§1.3) |
| `longnoteMarginRate` | `PlayerConfig` (JudgeManager.java:175에서 사용) | 없음 (`LN_MARGIN` 상수 고정) | **미노출** |
| `gauge` (시작 게이지 종류) | `PlayerConfig.java:53` | `gauge` 문자열 + `GAUGE_CYCLE` (`settings.rs:12`, UI 4번) | **구현**(6종. CLASS 계열 없음) |
| `gaugeAutoShift` (NONE/CONTINUE/SURVIVAL_TO_GROOVE/BESTCLEAR/SELECT_TO_UNDER) | `PlayerConfig.java:157,163-167` | 없음 | **미구현**. 9게이지 병렬 유지가 전제라 §3.3의 단일 게이지 구조부터 바꿔야 함 |
| `bottomShiftableGauge` (기본 ASSISTEASY) | `PlayerConfig.java:161` | 없음 | **미구현** (GAS 종속) |
| `lnmode` (0 undefined/1 LN/2 CN/3 HCN 강제) | `PlayerConfig.java:109` | 없음. 차트의 `#LNMODE`만 따름 (`docs/reference/cn-hcn-judgment.md`) | **미노출** — 사용자가 LN→CN 강제 변환 불가 |
| `misslayerDuration` | `PlayerConfig.java:104` | 없음 | 미구현(연출 항목, 판정 무관) |
| `showjudgearea` | `PlayerConfig.java:146` | 없음 | 미구현(표시 전용) |
| `notesDisplayTimingAutoAdjust` | `JudgeManager.java:690` | `auto_offset` (`settings.rs:21`, UI 15번 AUTO CAL) | **구현**(공식은 다름) |
| 코스 제약 NO_GREAT / NO_GOOD (judgeWindowRate 0) | `JudgeManager.java:167-175` | 없음 | 미구현 |
| — (rbms 고유) | — | `total_override` (`settings.rs:23`, UI 13번) | beatoraja에 없는 rbms 확장 |

---

## 5. 문서 stale 점검 (질문 2)

| 문서 | 상태 |
|---|---|
| `docs/reference/cn-hcn-judgment.md` | 본문 내용은 코드와 **일치**(2-판정 모델, `is_charge` 게이트, HCN 연속 게이지·CN deferral·BSS 잔여 명시). 단 원본 경로가 `/Users/hyunseokbyun/beatoraja/...`로 적혀 있어 실제 경로(`/Users/gkn/beatoraja`)와 다름 — 경로 stale |
| `docs/acknowledge/beatoraja-divergences.md` | CN/HCN 항목(27-32행)은 코드와 일치. 그러나 §1~§3에서 확인된 **판정 윈도우(5K/PMS/스크래치/LN-end)·게이지 세트(5K/PMS/KB/LR2/CLASS)·見逃しPOOR 게이지 인덱스·MODIFY_DAMAGE·지뢰 데미지·JudgeAlgorithm** 발산이 전혀 기록돼 있지 않음 → **불완전(누락형 stale)** |
| `docs/reference/mechanics.md` | 시간 상한으로 통독 못 함 — **미조사** |
| `docs/PROCESS.md` §7 한계 | 시간 상한으로 통독 못 함 — **미조사** |
| `windows.rs:22` 독스트링 "Values are beatoraja's JudgeProperty 7K NOTE table" | NOTE는 사실. 그러나 `SEVENKEY_LN_END`(`:29` "beatoraja JudgeProperty longnote end, 7K")는 실제로는 **FIVEKEYS longnote 값** → 주석 stale |

---

## 6. 미조사 범위 (정직)

- `docs/reference/mechanics.md`, `docs/PROCESS.md` 전문 미통독.
- bmson `judgerank`/`total` 변환 경로(`BMSPlayerRule.validate`의 BMSON 분기 대응물) 미확인.
- `ClearType.java` 열거 전문 및 rbms `clear_lamp`의 FC/Perfect/Max 조건이 beatoraja와 같은지 미대조.
- DJ 랭크(AAA~F) 산출식 양쪽 미대조.
- `apps/rbms-player/src/app_select.rs` 설정 렌더/입력 전 구간 중 880-945행만 확인(그 밖 항목 존재 가능성).
- `JudgeManager.java:760-888` 미통독(스코어 getter 위주로 추정되나 확인 안 함).
- rbms 테스트 실행(`cargo test`) 미수행 — 위 수치 대조는 소스 독해 기반.
