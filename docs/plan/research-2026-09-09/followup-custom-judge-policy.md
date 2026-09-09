# G-07 커스텀 판정·어시스트 스코어 취급 정책 (beatoraja 소스 확정 → rbms 대응 현황)

조사 범위: /Users/gkn/beatoraja/src (읽기 전용), /Users/gkn/R-BMS (읽기 전용). 모든 근거는 파일:라인.

## 0. 핵심 메커니즘 요약

beatoraja 는 **단일 boolean `score`(→ `PlayerResource.updateScore`)** 와 **정수 `assist`(0/1/2)** 두 값으로 전부를 제어한다.

- `BMSPlayer.java:50` `private int assist = 0;`, 같은 파일 `score` 플래그
- 최종 확정: `BMSPlayer.java:361-364`
  - `Logger... "アシストレベル : " + assist + " - スコア保存 : " + score;`
  - `resource.setUpdateScore(score);`
  - `resource.setUpdateCourseScore(resource.isUpdateCourseScore() && score);`
- `assist` 는 `BMSPlayer.java:732`/`:764` 에서 `resource.setAssist(assist)` 로 리절트에 전달.

`updateScore` 소비처 전수 (grep `isUpdateScore`):

| 소비처 | 파일:라인 | false 일 때 효과 |
|---|---|---|
| IR 송신 게이트 | MusicResult.java:86 `boolean send = resource.isUpdateScore();` | **IR 제출 안 함** |
| 리플레이 저장 | MusicResult.java:320 | 리플레이 파일 저장 안 함 |
| 동일 譜面 리플레이(재시도) | MusicResult.java:236, BMSPlayer.java:698 | "アシストモード時は同じ譜面でリプレイできません" → seed 재사용 불가 |
| DB 기록 | MusicResult.java:445-446 → `PlayDataAccessor.writeScoreData(..., resource.isUpdateScore())` | **EX/BP/avgjudge/combo 갱신 차단, 클리어램프·플레이카운트는 갱신됨** |
| 스킨 프로퍼티 | BooleanPropertyFactory.java:384/387/390 | 화면에 "스코어 미갱신" 표시 |

`writeScoreData` 내부 (`PlayDataAccessor.java:195-215`, `updateScore()` 422-458, `ScoreData.update` 531-585):

| 항목 | updateScore=false 일 때 |
|---|---|
| `score.setNotes()` | 갱신 안 함 (`:207-209`) |
| clearcount(+1) | **항상 증가** (`:211-213`, `newscore.getClear() > Failed.id` 조건만) |
| clear(램프) | **갱신됨** — `:427 if (score.getClear() < newscore.getClear())` 에 updateScore 가드 **없음** |
| exscore / minbp / avgjudge / combo | 전부 `&& updateScore` 로 차단 (`:433,439,445,451`) |

→ **결론: 어시스트/커스텀판정 플레이는 "스코어는 안 남지만 램프는 남는다".** 다만 그 램프는 아래 4장처럼 이미 AssistEasy 로 강등된 값이다.

---

## 1. (Q1) customJudge — 스코어/램프/IR 제한 조건

**정의·클램프**: `PlayerConfig.java:124-130` (`customJudge`, key/scratch × PG/GR/GD 3종, 기본 400/400/100), `:926-931` 클램프 (PG 25~400, GR/GD 0~400).

**판정폭 적용**: `JudgeManager.java:168-183`
```java
final int[] keyJudgeWindowRate = config.isCustomJudge()
    ? new int[]{...PerfectGreat, ...Great, ...Good} : new int[]{100,100,100};
```
`:184` `nreleasemargin = config.isCustomJudge() ? rule.longnoteMargin * config.getLongnoteMarginRate()/100L : rule.longnoteMargin;`

**제한 조건 (원문)**: `BMSPlayer.java:207-214`
```java
if (config.isCustomJudge() &&
    (getKeyJudgeWindowRatePerfectGreat() > 100 || ...Great > 100 || ...Good > 100
     || getScratchJudgeWindowRatePerfectGreat() > 100 || ...Great > 100 || ...Good > 100
     || config.getLongnoteMarginRate() > 100)) {
    assist = Math.max(assist, 2);   // = ASSIST
    score = false;
}
```

| 조건 | assist | score | 결과 |
|---|---|---|---|
| customJudge=false | 0 | true | 정상 |
| customJudge=true, 모든 rate ≤ 100 (판정 **좁힘**) | 0 | true | **정상 기록·IR 제출 가능** |
| customJudge=true, rate 중 하나라도 > 100 또는 longnoteMarginRate > 100 | **2 (ASSIST)** | **false** | EX 미갱신, 램프는 AssistEasy 로 기록, **IR 미제출**, 리플레이 미저장 |

이 블록은 `if (autoplay.mode == PLAY || AUTOPLAY)` 안에만 있다 (`BMSPlayer.java:200`).

**제한 없는 옵션 (확인)**:

| 옵션 | 근거 | 스코어 영향 |
|---|---|---|
| `judgetiming`(노트 표시 타이밍) | PlayerConfig.java:82,288-294; 사용처는 LaneRenderer.java:301(묘사 오프셋)·JudgeManager.java:701(auto-adjust)뿐 | **없음** (순수 표시/오프셋) |
| `gaugeAutoShift` | PlayerConfig.java:157,312-317; BMSPlayer.java:639-660 (리트라이 시 게이지 종류 변경) | **없음** — `setUpdateScore` 호출은 코드 전체에서 `BMSPlayer.java:363` 단 1곳뿐 |
| `lnmode` (0 LN/1 CN/2 HCN) | PlayDataAccessor.java:200,204 `scoredb.getScoreData(hash, model.containsUndefinedLongNote() ? lnmode : 0)`; IRScoreData.java:21 `lntype` | 제한 아님 — **스코어 행을 lnmode 별로 분리** (`#LNTYPE` 미지정 譜面일 때만; 지정 譜面은 항상 0) |

---

## 2. (Q2) assist 레벨 / autoplay / 리플레이 규칙 전부

`PatternModifier.AssistLevel` 정의: `pattern/PatternModifier.java:170-172` → `NONE, LIGHT_ASSIST, ASSIST`.
매핑: `BMSPlayer.java:233`/`:331` `assist = Math.max(assist, mod.getAssistLevel() == ASSIST ? 2 : 1); score = false;`

### 2-1. assist 를 올리는 모든 지점

| 트리거 | 파일:라인 | assist | score |
|---|---|---|---|
| BPM 가이드(`isBpmguide()` + BPM 변화 있는 譜面) | BMSPlayer.java:202-206 | 1 | false |
| customJudge rate > 100 (1장) | BMSPlayer.java:207-214 | 2 | false |
| ScrollSpeed / LongNote / MineNote / ExtraNote 譜面옵션 (assist!=NONE 인 경우) | BMSPlayer.java:216-235 | 1 or 2 | false |
| BATTLE (`doubleoption>=2`) — 여기에 `doubleoption==3` 이면 **AUTO SCRATCH**(`new AutoplayModifier(model.getMode().scratchKey)`, `:248`) 포함 | BMSPlayer.java:238-255 (특히 `:251-252`) | 1 (LIGHT) | false |
| SP/DP 랜덤·7to9 등 PatternModifier 중 assist!=NONE | BMSPlayer.java:326-334 | 1 or 2 | false |

각 Modifier 의 assist 등급:

| Modifier | AssistLevel | 근거 |
|---|---|---|
| MIRROR/ROTATE/RANDOM (`*_EX` = 스크래치 레인 포함) | EX 계열만 LIGHT_ASSIST, 아니면 NONE | LaneShuffleModifier.java:116,132,152 |
| FLIP(PlayerFlipModifier) | NONE | LaneShuffleModifier.java:172 |
| BATTLE(PlayerBattleModifier) | ASSIST | LaneShuffleModifier.java:190,200 |
| CROSS / PLAYABLE-RANDOM | LIGHT_ASSIST | LaneShuffleModifier.java:210,229 |
| H-RANDOM / S-RANDOM-EX | LIGHT_ASSIST, S-RANDOM(+NoThreshold) 는 NONE | Randomizer.java:158-162 |
| ModeModifier(7to9) | LIGHT_ASSIST | ModeModifier.java:33 |
| PracticeModifier | ASSIST | PracticeModifier.java:24 |
| LongNoteModifier (LN 제거/추가 = LEGACY NOTE 계열) | 실제로 변형이 일어나면 ASSIST | LongNoteModifier.java:28-37 |
| MineNoteModifier | 변형 시 LIGHT_ASSIST | MineNoteModifier.java:23-33 |
| AutoplayModifier (AUTO SCRATCH 등 레인 자동연주) | 실제 노트를 뺏으면 ASSIST | AutoplayModifier.java:29,74,80 |

### 2-2. 클리어타입 강등 규칙 (핵심)

`BMSPlayer.createScoreData()` `:862-882`
```java
ClearType clear = ClearType.Failed;
if (state != STATE_FAILED && gauge.isQualified()) {
    if (assist > 0) {
        if(resource.getCourseBMSModels() == null)
            clear = assist == 1 ? ClearType.LightAssistEasy : ClearType.AssistEasy;
    } else {
        ... FullCombo / Perfect / Max / gauge.getClearType()
    }
}
```
→ **assist>0 이면 풀콤보·PERFECT·MAX 판정 분기 자체를 타지 않는다.** assist=1 → `LightAssistEasy(id 3)`, assist=2 → `AssistEasy(id 2)`. (ClearType.java:12-13)

코스 모드: `MusicResult.java:403-406` — assist==1 이고 기존 코스램프가 AssistEasy 가 아니면 LightAssistEasy, 그 외엔 AssistEasy 로 덮어씀. 코스에서는 `BMSPlayer.java:866` 의 `getCourseBMSModels()==null` 조건 때문에 곡 단위 강등은 생략되고 코스 누적에서 처리한다.

### 2-3. autoplay / replay / practice

| 모드 | 스코어 객체 생성 | DB 기록 | IR 제출 | 근거 |
|---|---|---|---|---|
| PLAY | O | O | O(그 외 조건 통과 시) | BMSPlayer.java:753(`PLAY \|\| REPLAY`), MusicResult.java:444-448, :82 |
| REPLAY | O (표시용) | **X** | **X** | MusicResult.java:444 `if (mode == PLAY)` else 로그 "スコア登録はされません"; :82 `&& mode == PLAY` |
| AUTOPLAY | **X** (`createScoreData` 미호출; `:716`,`:753` 조건이 PLAY/REPLAY) | X | X | BMSPlayer.java:716,753 / JudgeManager.java:198 `this.autoplay = ...Mode.AUTOPLAY` |
| PRACTICE | X (`state = STATE_PRACTICE`) | X | X | BMSPlayer.java:733-735, :765-767 |

IR 송신의 추가 게이트 (`MusicResult.java:81-97`): IR 등록 존재 + `mode == PLAY` + `send = isUpdateScore()` 에 `IRConfig` 정책(`IR_SEND_ALWAYS` / `IR_SEND_COMPLETE_SONG`(게이지 끝값>0) / `IR_SEND_UPDATE_SCORE`(자기베스트 갱신))을 AND. 즉 **어시스트가 붙으면 IRConfig 가 ALWAYS 여도 제출되지 않는다.**

---

## 3. (Q3) rbms 대응 현황

| 항목 | beatoraja 규칙 | rbms 현재 동작 | 근거 (rbms) | 판정 |
|---|---|---|---|---|
| 판정폭 커스텀 | rate>100 → assist=2, score=false | `judge_rate` 50~200% 자유 설정, **제한 로직 없음** | app_play.rs:131 `player.set_judge_rate(self.config.judge_rate)`; app_select.rs:938 `clamp(50,200)`; main.rs:149 | **미대응** |
| customJudge 세분화(PG/GR/GD, key/scratch 분리, longnoteMarginRate) | 6+1 파라미터 | 단일 `judge_rate` 하나뿐 | settings.rs:22 | **미대응(단순화)** |
| AUTO SCRATCH | assist=1(LIGHT), score=false → 램프 LightAssistEasy, IR 미제출 | `scratch_auto` 는 플레이에만 반영, 램프·기록·제출 무영향 | app_play.rs:132; :279 (`options.scratch_auto` 로 메타만 전송) | **미대응** |
| autoplay | 스코어 객체 자체를 만들지 않음 | 로컬 기록은 제외하지만 **IR 제출은 그대로 수행** | app_play.rs:340 `if self.replay.is_none() && !self.autoplay {` (로컬만); :306-311 제출 스레드는 무조건 실행 | **부분 대응 (제출 미차단 = 버그성)** |
| replay 재생 | DB·IR 모두 제외 | 로컬 기록 제외, **IR 제출은 수행** | app_play.rs:340 / :306-311 | **부분 대응** |
| `updateScore` 상당 플래그 | 존재 (`PlayerResource.updateScore`) | **존재하지 않음** | app_play.rs 전체에 게이트 없음 | **미대응** |
| assist 레벨 개념 | 0/1/2 + 램프 강등 | 없음. IR 제출 필드 `assist: vec![]` 하드코딩 | app_play.rs:282 | **미대응** |
| clear lamp 산출 | assist>0 이면 FC/PERFECT/MAX 분기 스킵 → Assist(Light)Easy | 게이지 종류만 보고 산출, assist 개념 없음 | gauge.rs:130-154 (`clear_lamp`) | **미대응** |
| LightAssistEasy(id 3) | 존재 | **rbms 는 절대 생성하지 않음**(테스트로 고정) | ir_map.rs:175-179 `ir_clear_never_emits_light_assist_easy`; format.rs:234 "id 3 is skipped" | 의도된 갭 |
| 램프는 남고 스코어만 막힘 | writeScoreData 의 clear 갱신에 가드 없음 | 로컬 기록은 all-or-nothing (autoplay/replay 면 통째 스킵) | PlayDataAccessor.java:427 vs app_play.rs:340 | **의미 차이 있음** |
| lnmode 별 스코어 행 분리 | `containsUndefinedLongNote() ? lnmode : 0` 키 | `ScoreRecord` 는 md5 기준, lntype 은 IR 옵션 필드로만 전송 | app_play.rs:280 `lntype: self.chart_lntype`; scores.rs 레코드에 lnmode 없음 | **미대응** |
| judgetiming/offset, gauge auto shift | 스코어 무제한 | `offset_ms`/`auto_offset` 도 무제한 | app_play.rs:285-286, :291 | 일치 |

### 구현 시 최소 이식안 (제안, 미구현)

1. `PlaySettings` 로부터 `update_score: bool` 과 `assist_level: u8` 을 플레이 시작 시 1회 산출한다. 규칙: `judge_rate > 100` → assist=2, `scratch_auto` → assist=1, (향후 LN 제거/노트 옵션 추가 시 동일 표 적용).
2. `enter_result`: `assist > 0` 이면 `lamp` 를 `AssistEasy`(2) / `LightAssistEasy`(1) 로 강등한 뒤 결과·기록·제출에 사용. 현재 `ir_map.rs` 가 LightAssistEasy 를 못 만들므로 매핑 확장 필요.
3. `ScoreSubmission` 전송(app_play.rs:306-311)을 `!autoplay && replay.is_none() && update_score` 로 게이트.
4. 로컬 `ScoreRecord`(app_play.rs:340)는 beatoraja 처럼 "램프·플레이카운트는 남기고 EX 는 갱신 안 함" 으로 나눌지, 현행 all-or-nothing 을 유지할지 결정 필요 (**미확정 정책**).

---

## 4. 미확인

- `ScoreDatabaseAccessor.java` 는 이번에 직접 통독하지 않음(=`writeScoreData` 가 넘긴 `ScoreData` 를 그대로 upsert 한다고 가정). **미확인**.
- `PlayerConfig.getLongnoteMarginRate()` 의 정의 라인은 확인하지 않음(사용처 BMSPlayer.java:213, JudgeManager.java:184 만 확인). **미확인**.
- rbms 서버(`rbms-ir`) 측이 `options.judge_rate`/`autoplay` 를 받아 서버에서 거르는지 여부는 조사 범위 밖. **미확인**.
