# judge-gauge 감사 보고서 검증 (회의적 검증자)

대상: `scratchpad/research/judge-gauge.md` findings 18건.
방법: 각 finding 의 근거 file:line 을 레퍼런스 구현 Java 원본과 rbms Rust 코드 양쪽에서 직접 열어 대조.
검증 시각: 2026-09-09. 코드는 읽기만 했다(수정·빌드 없음).

## 총평

- 18건 중 **16건 confirmed, 2건 partially**. 반박(refuted)에 성공한 항목은 없다.
- 수치는 대체로 정확하나 **레퍼런스 구현 측 라인 번호가 1~2줄씩 밀린 인용이 다수**다(아래 §라인 드리프트). 주장 자체는 해당 구문을 정확히 가리킨다.
- 다만 보고서는 **"게이지가 전반적으로 어긋난다"는 인상**을 준다. 실제로 7K 게이지 6종의 수치·modifier(TOTAL/LIMIT_INCREMENT)·guts·클램프 게이트는 레퍼런스 구현과 **완전히 일치**한다(§놓친 항목 M7). 발산은 판정 슬롯 오배정(01) 하나에 집중돼 있다 — 01 의 critical 등급은 타당하다.

## 항목별 판정

| id | 판정 | 요지 |
|---|---|---|
| 01 | confirmed | 슬롯 오배정 실재 |
| 02 | confirmed | 5K/PMS/24K 윈도우 부재 |
| 03 | **partially** | 값 오류는 사실, 권고의 "코드 5" 는 틀림 |
| 04 | confirmed | LN_MARGIN 200ms 전 모드 고정 |
| 05 | confirmed | 기본 TOTAL 공식 미적용 |
| 06 | confirmed | 스크래치 윈도우/BSS 부재 |
| 07 | confirmed | JUDGE WIDTH 가 BD 까지 확대 |
| 08 | confirmed | 지뢰 데미지 미적용 |
| 09 | confirmed | 게이지 단일 인스턴스 |
| 10 | confirmed | Duration 고정, 기본은 Combo |
| 11 | confirmed | #DEFEXRANK 미파싱 |
| 12 | confirmed | PMS 판정 규칙 미구현 |
| 13 | confirmed | HCN 연속 게이지/CN deferral 부재 |
| 14 | **partially** | HARD_5 는 MODIFY_DAMAGE 가 아님 |
| 15 | confirmed | dm==0 방향 집계 반대 |
| 16 | confirmed | clamp vs NORMAL 폴백 |
| 17 | confirmed | 판정 완료 노트 재타격 무판정 |
| 18 | confirmed | 문서 stale |

---

### 01 — 見逃しPOOR 슬롯 오배정 (confirmed)

- 레퍼런스 구현 `JudgeManager.java:592-598` 見逃し 루프가 `:598` `updateMicro(..., 4, ...)` 로 코드 4 를 넘긴다. NOTE 경로만 `JudgeManager.java:394` 에서 `judge = (judge >= 4 ? judge + 1 : judge)` 로 시프트하므로 **MS 윈도우 히트 = 코드 5(空POOR), 見逃し = 코드 4(PR)** 가 확정된다.
- 게이지 배열 순서 주석은 `GaugeProperty.java:133` ("PG, GR, GD, BD, PR, MSの順"), NORMAL 은 `GaugeProperty.java:89` `{1,1,0.5,-3,-6,-2}` → PR(-6) / MS(-2). 보고서가 인용한 `:152`/`:90` 은 각각 `:133`/`:89` 다(드리프트).
- rbms: `crates/rbms-judge/src/matcher.rs:309` 및 `:312` 가 스윕 미스에 `Judge::Miss`(enum 인덱스 5) 를 push → `gauge.rs:104` `deltas[judge as usize]` → `gauge.rs:51` Normal `deltas[5] = -2.0`. **NORMAL 미스가 -6 대신 -2** 로 실측 확인.
- 空POOR 경로(`matcher.rs:233` `gauge.update(Judge::Miss)`)는 MS(-2)가 맞다 — 즉 두 경로가 같은 슬롯을 공유해 하나가 틀렸다.
- 파급 확인(보고서 권고가 맞음): `apps/rbms-player/src/app_play.rs:251-252,264-266` 이 `poor: c[4] / miss: c[5]`, `epr: early[4] / ems: early[5]` 로 매핑해 **IR 로 나가는 見逃しPOOR 가 전부 ms 필드로 보고**된다.

### 02 — 모드별 NOTE 윈도우 부재 (confirmed)

- 레퍼런스 구현 실측: `JudgeProperty.java:12` FIVEKEYS `{±20k, ±50k, ±100k, ±150k, (-150k,500k)}`, `:34` PMS `{±20k, ±50k, ±117k, ±183k, (-175k,500k)}`, `:45` KEYBOARD `{±30k, ±90k, ±200k, (-320k,240k), (-200k,650k)}`. 보고서 수치 전부 일치(PMS/KEYBOARD 라인만 1줄 드리프트).
- rbms: `windows.rs:20-26` SEVENKEY_NOTE 는 레퍼런스 구현 `:23` SEVENKEYS 와 정확히 동일 → 7K 는 정상. `windows.rs:36-44` POPN_NOTE 가 7K 값 복제(주석에 "pending verified" 명시), `windows.rs:60-64` `note_for_mode` 가 `"POPN_9K"` 외 전부 SEVENKEY_NOTE. 확인.

### 03 — SEVENKEY_LN_END 값 오류 (partially)

- 핵심 주장은 **사실**: 레퍼런스 구현 `JudgeProperty.java:25` SEVENKEYS longnote = `{-120k,120k, -160k,160k, -200k,200k, -280k,220k}` (4쌍, MS 없음). rbms `windows.rs:29-35` 는 `gr ±150k, bd ±250k` 로 **`JudgeProperty.java:14` FIVEKEYS longnote 와 일치**. 독스트링이 "7K longnote end" 라고 잘못 표기한 것도 사실.
- **권고 중 오류**: "윈도우 밖 = 코드 5(空POOR)" 는 틀렸다. `JudgeProperty.java:184-188` `getJudge` 는 매칭 실패 시 `mjudge.length/2` (= longnote 는 4) 를 반환하고, LN 종단 경로(`JudgeManager.java:363`, `:487`)는 **`+1` 시프트를 하지 않는다** → 윈도우 밖 = **코드 4(見逃しPOOR)**. 즉 rbms `matcher.rs:279`/`:296` 의 `unwrap_or(Judge::Poor)`(인덱스 4)는 우연히 **정확**하다. 정정 후 SEVENKEY_LN_END 의 `ms` 필드는 제거하고 "밖 = Poor" 폴백만 남기는 것이 맞다.

### 04 — LN 릴리스 마진 (confirmed)

- `JudgeProperty.java:15/26/37/48` = FIVEKEYS 0 / SEVENKEYS 0 / PMS 200000 / KEYBOARD 0. `JudgeManager.java:186` `nreleasemargin = rule.longnoteMargin (× rate)`, `:559` 가 이를 사용.
- rbms `matcher.rs:7` `LN_MARGIN = 200_000` 전역 상수, `:294` 에서 모드 무관 적용. 확인.
- 부수 주장도 사실: `JudgeManager.java:572-577` 은 plain LN 미릴리스를 **`lnstartJudge`(머리 판정)** 로 확정하지만 rbms `matcher.rs:295-299` 는 ln_end 윈도우로 재판정한다.

### 05 — 기본 TOTAL 공식 (confirmed)

- `BMSPlayerRule.java:69-72` (`TotalType.BMS` 에서 `total<=0` → `calculateDefaultTotal`), `:84-89` `max(260, 7.605n/(0.01n+6.5))` / KEYBOARD 는 `max(300, 7.605(n+100)/(0.01n+6.5))`. 확인.
- rbms `crates/rbms-chart/src/lib.rs:196` `total: src.headers.total.unwrap_or(0.0)`, `matcher.rs:161` `set_gauge(Normal, model.meta.total)`, `gauge.rs:77` `if total > 0.0 { total } else { 200.0 }`. 같은 공식이 `chart/lib.rs:406` 에 존재하나 `note_density` 전용이라 게이지에 안 쓰인다 — 보고서 지적대로다.
- "최소 23% 손실" 도 산술적으로 맞다(200/260 = 0.769).

### 06 — 스크래치 윈도우/BSS (confirmed)

- `JudgeProperty.java:24` SEVENKEYS scratch = `{±30k, ±70k, ±160k, (-290k,230k), (-160k,500k)}`, `:27` longscratch. `JudgeManager.java:352-372` BSS 종단(`sckey` 대조), `:498-511` BSS 중간 떼기 무시. 확인(라인 2줄 드리프트).
- rbms `windows.rs` 에 scratch 상수 없음, `matcher.rs:194-289` press/release 가 레인 종류를 구분하지 않음. 확인.

### 07 — JUDGE WIDTH (confirmed)

- `JudgeProperty.java:262-273` 이 judgeWindowRate 를 `Math.min(org.length, 3)` (PG/GR/GD) 에만 적용하고, MS(index 6/7) 상한 + 직전 판정 하한으로 클램프한다. `JudgeManager.java:168-173` 도 rate 배열이 3개다.
- rbms `crates/rbms-play/src/lib.rs:128-134` `set_judge_rate` 는 judgerank 와 rate 를 곱해 `windows.rs:74-81` `scaled()` 하나로 PG/GR/GD/**BD** 를 일괄 확대하고 클램프가 없다. 확인.
- 보조 주장("GD 가 MS 를 넘을 수 있다")도 성립: 7K 기본에서 `gd = ±150k`, `ms.late = -150k` 로 이미 경계이므로 rate>100 이면 GD LATE 가 MS LATE 를 넘는다.

### 08 — 지뢰 데미지 (confirmed)

- `JudgeManager.java:244-247` `else if (note instanceof MineNote mnote && pressed) { main.getGauge().addValue(-mnote.getDamage()); }`. `GrooveGauge.java:63-67` `addValue` 는 전 게이지에 setValue 적용.
- rbms: `NoteKind::Mine { damage: f64 }` 가 모델에는 있으나(`rbms-model/src/lib.rs:27`), `matcher.rs:145` 와 `rbms-play/src/lib.rs:66` 에서 스킵되고 `gauge.rs` 에 `add_value` 상당 API 없음. 확인.

### 09 — 게이지 단일 인스턴스 (confirmed)

- `GrooveGauge.java:36-41` 이 `property.values` 전부 인스턴스화, `:53-61` `update` 가 전 게이지 갱신. `GaugeProperty.java:12-59` 각 모드 9종. `PlayerConfig.java:157,161,163-167` gaugeAutoShift 5종 + bottomShiftableGauge. 확인.
- rbms `gauge.rs:4-11` 6종, `matcher.rs:60` `pub gauge: Gauge` 단일, `apps/rbms-player/src/main.rs:591` `GAUGE_CYCLE` 6종(보고서가 인용한 `format.rs:162` 는 테스트용 `ALL_GAUGES` 배열이다 — 인용 오류지만 주장은 유효).

### 10 — JudgeAlgorithm (confirmed)

- `JudgeAlgorithm.java:17-36` Combo/Duration/Lowest/Score, `:42` `defaultAlgorithm`, `PlayConfig.java:103` `judgetype = Combo`. `JudgeManager.java:383` 이 `algorithm.function.compare` 로 후보를 고른다.
- rbms `matcher.rs:210-220` 은 `a < best_abs` 절대 시간차 최소 = Duration 고정, 선택 API 없음. 확인.

### 11 — #DEFEXRANK (confirmed)

- `BMSPlayerRule.java:63` `case BMS_DEFEXRANK -> judgerank > 0 ? judgerank * judgerank[2][1] / 100 : ...`.
- rbms `crates/rbms-parser/src/lib.rs:245` 는 `"RANK"` 만 처리, `DEFEXRANK` 분기 없음(grep 0건). 확인.

### 12 — PMS 판정 규칙 (confirmed)

- `JudgeProperty.java:40` PMS combo `{T,T,T,F,F,F}`(7K 는 `:29` `{T,T,T,F,F,T}`), `:41` `MissCondition.ONE`, `:42` judgeVanish `{T,T,T,F,T,F}`, `:211` `JudgeWindowRule.PMS` judgerank `{100,33,33,100,100}` 계열 + fixjudge `{T,F,F,T,T}`. 확인(라인 1~2줄 드리프트).
- rbms `matcher.rs:229-236` 空POOR 는 항상 콤보 유지·재발생 가능, `:255` BAD 는 항상 노트 소비, `windows.rs:74-81` 은 NORMAL 규칙 하나뿐. 확인.

### 13 — HCN 연속 게이지 / CN deferral (confirmed)

- `JudgeManager.java:81` `hcnmduration = 200000`, `:313-318` `mpassingcount > hcnmduration → gauge.update(1, 0.5f)`, `:327-330` 미홀드 시 `update(3, 0.5f)`, `:498-506` CN 의 `judge >= 3 && dmtime > 0` deferral(`releasetime`/`lnendJudge`). `GrooveGauge.java:57`/`:229` `update(judge, rate)` 오버로드 존재.
- rbms `gauge.rs:102` `update(&mut self, judge: Judge)` 에 rate 인자 없음, `matcher.rs:272-287` release 는 즉시 확정. 확인.

### 14 — MODIFY_DAMAGE (partially)

- **오류**: `GaugeProperty.java:80` `HARD_5(LIMIT_INCREMENT, ...)` — HARD_5 는 MODIFY_DAMAGE 가 **아니다**. MODIFY_DAMAGE 를 쓰는 것은 `:81 EXHARD_5`, `:120 HARD_LR2`, `:121 EXHARD_LR2` 3종이다. 보고서의 "FIVEKEYS와 LR2의 HARD/EXHARD" 는 과다 진술이고 인용 라인(:79/:80)도 밀렸다.
- 나머지는 사실: `GrooveGauge.java:274-288` MODIFY_DAMAGE(fix1total/fix1table 10구간 + fix2 노트수 루프, 최대 10배), rbms `gauge.rs:27-31` `Modifier { Total, LimitIncrement, None }` 에 대응 분기 없음.
- 등급은 오히려 과대: 현재 rbms 는 5K/LR2 게이지 세트 자체가 없어 **적용 대상이 0** 이다. 09 의 선행 없이는 무의미하므로 info/low 로 낮춰도 된다.

### 15 — dm==0 방향 집계 (confirmed)

- `JudgeManager.java:648` `score.addJudgeCount(judge, mfast >= 0, 1)`, `:681` `keysound.play(judge, mfast >= 0)` → **0 은 EARLY**.
- rbms `matcher.rs:378-382` `if dm > 0 { early } else { late }` → 0 은 LATE. `:366-373` fast/slow 도 `dm > 0` / `dm < 0` 만 세므로 0 은 양쪽 어디에도 안 들어간다. 확인. 실질 영향은 µs 단위 정확 일치에 한정돼 low 가 타당.

### 16 — #RANK 범위 밖 (confirmed)

- `BMSPlayerRule.java:62` `judgerank >= 0 && ... < 5 ? table[judgerank][1] : table[2][1]` → 범위 밖은 **NORMAL(75)**.
- rbms `windows.rs:106` `TABLE.get(rank.clamp(0,4) as usize)` → -1→25(VERYHARD), 5→125(VERYEASY). 확인. 보고서 지적대로 `windows.rs` 의 `rank_clamps_out_of_range` 테스트가 잘못된 규약을 고정하고 있다.

### 17 — 판정 완료 노트 재타격 (confirmed)

- `JudgeManager.java:390-391` `if (judgenote.getState() != 0) { judge = (MS 범위) ? 5 : 6; }` → 코드 5 는 `:419` 이후 `updateMicro(..., judgeVanish[5]=false)` 로 흘러 空POOR 로 집계된다.
- rbms `matcher.rs:212` `if !n.judged && !n.holding && ...` 로 후보에서 배제 → 무판정. 확인.

### 18 — 문서 stale (confirmed)

- `docs/acknowledge/reference-divergences.md` 에는 파서/BGA/LNTYPE2/CN·HCN 항목만 있고 판정 윈도우·게이지 섹션이 없다(전문 확인).
- `docs/reference/cn-hcn-judgment.md:8` 원본 경로가 `/Users/hyunseokbyun/<reference>/...` 로 실제 경로(`<reference>/`)와 불일치. `windows.rs:28` 독스트링이 FIVEKEYS 값을 "7K longnote end" 로 표기. 전부 확인.

---

## 라인 드리프트 (인용 정정)

| finding 인용 | 실제 |
|---|---|
| JudgeProperty.java:22 scratch / :23 longnote / :24 margin / :25 longscratch (7K) | :24 / :25 / :26 / :27 |
| JudgeProperty.java:33 PMS note / :36 margin / :38 combo / :39 miss / :40 vanish / :207 rule | :34 / :37 / :40 / :41 / :42 / :211 |
| JudgeProperty.java:44 KEYBOARD note | :45 |
| JudgeProperty.java:255-266 rate 루프 | :262-273 |
| GaugeProperty.java:90 NORMAL / :152 주석 / :79 HARD_5 / :80 EXHARD_5 / :131,:132 LR2 / :22-31 7K 9종 | :89 / :133 / :80 / :81 / :120,:121 / :22-31(확인) |
| GrooveGauge.java:277-297 MODIFY_DAMAGE | :274-288 |
| BMSPlayerRule.java:78-84 / :88-93 | :69-72 / :84-89 |
| JudgeManager.java:243-248 지뢰 / :305-313 HCN / :499-506 CN / :575-580 LN / :595 見逃し | :244-247 / :313-318 / :498-506 / :572-577 / :598 |
| apps/rbms-player/src/format.rs:162-163 GAUGE_CYCLE | main.rs:591 (format.rs 는 테스트 `ALL_GAUGES`) |

주장 자체는 모두 해당 구문을 정확히 지목한다. 후속 작업 시 위 실제 라인을 쓴다.

## 놓친 항목 (같은 관점)

| # | 항목 | 근거 |
|---|---|---|
| M1 | **空POOR 가 판정 카운트에 안 들어간다.** 레퍼런스 구현 는 코드 5 도 `addJudgeCount(5)` 로 `ems/lms` 에 집계(`ScoreData.java:262`, `JudgeManager.java:648`)하지만 rbms 는 `matcher.rs:231-236` 에서 `empty_poor` 전용 카운터만 올리고 `counts[]` 를 건드리지 않아 `app_play.rs:266` 의 `ems` 에 0 이 나간다. 01 과 합치면 IR 의 pr/ms 4필드가 전부 어긋난다 | ScoreData.java:262 / matcher.rs:231-236 / app_play.rs:264-266 |
| M2 | **`combocond` 테이블 미구현.** 7K 는 `combo[5]=true`(空POOR 는 콤보 유지), `combo[4]=false`(見逃し만 끊음)인데 rbms `apply()` 는 Poor·Miss 둘 다 `combo = 0`. 현재는 空POOR 가 `apply()` 를 안 타서 마스킹되지만, 01 을 슬롯 교체만으로 고치면 **空POOR 가 콤보를 끊는 회귀**가 즉시 표면화한다 | JudgeProperty.java:29 / matcher.rs:229-236, :355-362 |
| M3 | **모드별 게이지 세트 부재(09 와 별개).** PMS 게이지는 종류 수뿐 아니라 스펙이 다르다: `NORMAL_PMS(min 2, max 120, init 30, border 85, {1,1,0.5,-2,-6,-6})`. KB/LR2/5K 도 각각 다르다. rbms `gauge.rs:44-52` 는 7K 표 하나뿐이라 POPN 차트도 7K 게이지로 돈다 | GaugeProperty.java:97-105, 107-115, 117-125 / gauge.rs:44-52 |
| M4 | **`fixjudge` 가 per-index 인데 rbms `scaled()` 는 표현 불가.** `JudgeWindowRule.PMS` 는 `judgerank {100,33,33,100,100}` + `fixjudge {T,F,F,T,T}` 로 PG/BD/MS 고정·GR/GD 만 스케일, 게다가 `create()` 는 fixmin/fixmax 단조 클램프까지 돈다. `windows.rs:74-81` 의 "pg~bd 일괄 × rank, ms 고정" 구조로는 PMS 를 표현할 수 없어 02·12 수정 시 시그니처 변경이 필요하다 | JudgeProperty.java:211, :224-260 / windows.rs:74-81 |
| M5 | **키/스크래치 JUDGE WIDTH 분리 설정 부재.** 레퍼런스 구현 는 `keyJudgeWindowRate*` 3개와 `scratchJudgeWindowRate*` 3개를 따로 받는다. rbms `set_judge_rate` 는 단일 스칼라 | JudgeManager.java:168-173 / rbms-play/src/lib.rs:128-134 |
| M6 | **bmson `#TOTAL` 퍼센트 정규화 미확인.** `validate` 의 `case BMSON -> total > 0 ? total/100*defaultTotal : defaultTotal` 이 05 의 권고 문장에만 있고 finding 본문/근거에 없다. rbms bmson 경로의 total 취급은 이번 검증에서 **미확인** | BMSPlayerRule.java:74-78 |
| M7 | **(과대평가 방지) 7K 게이지 수치는 완전 일치.** ASSIST_EASY/EASY/NORMAL/HARD/EXHARD/HAZARD 6종의 min/max/init/border/deltas/guts 가 `GaugeProperty.java:87-92`(ASSIST_EASY :87 ~ HAZARD :92) 와 정확히 같고, `TOTAL`·`LIMIT_INCREMENT` 공식(`GrooveGauge.java:260,264-271`)과 guts 적용 순서, `value <= 0` 이면 갱신 중단(`GrooveGauge.java:218-222` `setValue` vs `gauge.rs:103`)까지 동일하다. 게이지 발산은 01 의 슬롯 오배정 1건에 집중된다 | GaugeProperty.java:87-92 / gauge.rs:44-52, :77-97 |
| M8 | **(검증 완료, 발산 아님) 후보 게이트·見逃し 경계·miss 방향.** `mjudgestart/mjudgeend`(`JudgeManager.java:189-196`)는 note 윈도우의 min/max = `(bd.late, ms.early)` 로 rbms `matcher.rs:203-204` 의 `gate_late/gate_early` 와 동일. 見逃し 스윕 경계(`JudgeManager.java:592` `note.time < mtime + getTime(type,3,false)`)도 `matcher.rs:305` 와 동일하고, 見逃し 는 양쪽 모두 항상 LATE 로 집계된다 | JudgeManager.java:189-196, :592, :648 / matcher.rs:203-204, :305, :318-321 |

## 미조사 범위

- `rbms-play`/`apps/rbms-player` 의 오토플레이·리플레이 경로에서의 판정 재현
- bmson 파서의 `#TOTAL`/judgerank 취급 (M6)
- `clear_lamp` ↔ `ClearType.getClearTypeByGauge` 대조
- 코스(段位) 게이지·`gaugeAutoShift` 런타임 동작
