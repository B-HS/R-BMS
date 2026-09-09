# Phase D 상세 설계 — 판정 패리티 완성 + JUDGE 설정 노출

> 대상: `docs/plan/2026-09-09-enhancement-plan.md` §2 Phase D. 근거 결정: `docs/acknowledge/2026-09-09-enhancement-decisions.md` 결정 12.
> 원칙: 레퍼런스 구현의 수치 테이블을 **손으로 재해석하지 않고 그대로 상수화**하고, 테스트가 그 배열을 byte 단위로 고정한다.
> 레퍼런스 구현 인용은 bare 파일명 + 라인으로만 한다(`JudgeProperty.java:210` 등).
> 병행 워크플로 주의: Phase I 가 `crates/rbms-ir` · `apps/rbms-player` 를 동시에 고치고 있다. 이 문서의 앱 측 라인 번호는 **Phase I 이후 재확인** 대상이며, 함수명으로 앵커한다.

---

## 0. 현재 rbms 판정 스택 (앵커)

| 요소 | 위치 | 현 상태 |
|---|---|---|
| `Judge` 6단계 enum | `crates/rbms-judge/src/lib.rs:14-21` (같은 파일 23행부터 대형 `#[cfg(test)] mod tests` — 매처 테스트가 여기 산다) | PG/GR/GD/BD/Poor(見逃し)/Miss(空POOR). 레퍼런스 judge code 0..5 와 인덱스 일치 |
| `JudgeWindows` / `JudgeProperty` | `crates/rbms-judge/src/windows.rs:13-50` | 4 테이블 + margin + combo/vanish/miss_condition. FIVEKEYS/SEVENKEYS/PMS/KEYBOARD 4행 이식 완료(`windows.rs:62-114`) |
| judgerank 스케일 | `windows.rs:178-182` (`scaled`) | 전 인덱스 균일 배율, `fixjudge` per-index 없음, `fixmin/fixmax` 클램프 없음 |
| JUDGE WIDTH rate | `windows.rs:188` (`with_window_rate`) + `crates/rbms-play/src/lib.rs:133-140` (`set_judge_window_rates`) | PG/GR/GD 3티어 + BAD/이전티어 클램프까지 이식됨. 키/스크 배열 인자는 이미 분리 |
| 게이지 | `crates/rbms-judge/src/gauge.rs:3-136` | 단일 `Gauge` 1개, 6종(`GaugeKind`), 7K 세트만, `MODIFY_DAMAGE` 없음 |
| DJ 랭크 | `crates/rbms-render/src/result.rs:77-100` (`RANK_BANDS`/`RANK_BOUNDS`/`dj_rank`) | 8밴드 9분위. 레퍼런스와 경계 동일 — §6.5 대조 |
| 클리어 램프 | `gauge.rs:139-163` (`clear_lamp`) | `ClearType` 10종이나 `LightAssistEasy` 없음, gauge→lamp 매핑이 레퍼런스와 다름 |
| 후보 선택 | `crates/rbms-judge/src/matcher.rs:317-390` (`press`), 후보 루프 `335-354` | `|dm|` 최소 = Duration 알고리즘 고정. `best` 는 `!judged && !holding` 만(`343-347`), `judged` 는 재타격 후보로 따로 수집(`348-351`) |
| CN/HCN | `matcher.rs:369-381` (press 내 `is_cn` 분기), `matcher.rs:399` (`release`, 본문 399-420; `update` 는 425-) | 두-판정 모델은 있으나 **deferral(releasetime/lnendJudge) 없음**, HCN 연속 게이지 없음, BSS/MSS 없음 |
| 모드 선택 | `windows.rs:117-123` (`for_mode`) | BEAT_5K/10K→FIVEKEYS, POPN_9K→PMS, 나머지→SEVENKEYS. **KEYBOARD(24K) 도달 경로 없음** |
| 스크래치 레인 | `matcher.rs:246` (`is_scratch`) + `Mode::is_scratch` | 레인당 1키. 정/역 2키 없음 |
| 앱 설정 행 | `apps/rbms-player/src/app_select.rs` `setting_line` / `adjust_setting` (현재 index 0..23, JUDGE WIDTH = 12) | 단일 `judge_rate` 50~200% 하나뿐 |
| assist 플래그 | `apps/rbms-player/src/ir_map.rs` `assist_flags`, `apps/rbms-player/src/main.rs` `ir_submission_block_reason` / `updates_score` | rate>100 → 제출 차단·기록 제외까지는 구현. **램프 강등 미구현** |

---

## 1. J17 — JudgeAlgorithm 4종

### 현재
`matcher.rs:335-354`: 후보 중 **미판정** 노트의 `|dm|` 최소 1개를 고른다(`343-347`) → 레퍼런스 `Duration` 과 동치. 선택지 없음. 판정완료 노트는 같은 루프에서 재타격 후보로만 수집된다(`348-351`, 소비처 `empty_poor_hit` `392-397`).

### 레퍼런스
`JudgeAlgorithm.java:17-37`. 비교 함수 `compare(t1, t2, ptime, window, type) -> bool` 이 **true 면 t2 채택**. 후보를 레인 앞에서부터 순차 순회하며 누적 비교한다.
`JudgeWindow.getTime(type, j, early)` = `judges[j*2 + (early ? 1 : 0)]` (`JudgeProperty.java:175-182`). 즉 인덱스 0 = LATE(음수) 하한, 1 = EARLY(양수) 상한. 판정 인덱스 1 = GREAT, 2 = GOOD.
`difftime = note.time - ptime` (rbms `dm` 과 부호 동일, `JudgeProperty.java:184-189`).

| 알고리즘 | 조건(true = t2 채택) | 의미 |
|---|---|---|
| `Combo` (기본) | `t2.state == 0 && t1.time < ptime + good_late && t2.time <= ptime + good_early` | 아래 노트를 GOOD 이상으로 못 잡으면 위 노트로 넘어간다 |
| `Duration` | `abs(t1.time - ptime) > abs(t2.time - ptime) && t2.state == 0` | 시간차 최소 |
| `Lowest` | `false` | 항상 최하(가장 앞) 노트 |
| `Score` | `t2.state == 0 && t1.time < ptime + great_late && t2.time <= ptime + great_early` | GREAT 기준으로 같은 판단 |

`defaultAlgorithm = {Combo, Duration, Lowest}` (`JudgeAlgorithm.java:42`) — 설정 UI 에 노출되는 순서는 이 3종이고 `Score` 는 배열 밖이다. **rbms 는 4종 전부 노출하되 기본값은 `Combo`.**

### 변경 (Rust)

새 파일 `crates/rbms-judge/src/algorithm.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum JudgeAlgorithm {
    #[default]
    Combo,
    Duration,
    Lowest,
    Score,
}

impl JudgeAlgorithm {
    pub const ALL: [JudgeAlgorithm; 4] =
        [JudgeAlgorithm::Combo, JudgeAlgorithm::Duration, JudgeAlgorithm::Lowest, JudgeAlgorithm::Score];

    pub fn label(self) -> &'static str { /* "COMBO" | "DURATION" | "LOWEST" | "SCORE" */ }

    /// `true` = adopt `t2` over `t1`. `t1_us`/`t2_us` are note head times, `press_us` the key time,
    /// `w` the window table for the note type being judged, `t2_unjudged` mirrors `t2.getState() == 0`.
    pub fn prefers_second(self, t1_us: i64, t2_us: i64, press_us: i64, w: &JudgeWindows, t2_unjudged: bool) -> bool {
        match self {
            JudgeAlgorithm::Combo => t2_unjudged && t1_us < press_us + w.gd.0 && t2_us <= press_us + w.gd.1,
            JudgeAlgorithm::Duration => (t1_us - press_us).abs() > (t2_us - press_us).abs() && t2_unjudged,
            JudgeAlgorithm::Lowest => false,
            JudgeAlgorithm::Score => t2_unjudged && t1_us < press_us + w.gr.0 && t2_us <= press_us + w.gr.1,
        }
    }
}
```

`matcher.rs` 의 후보 루프는 "min by |dm|" 대신 **폴드**로 바꾼다.

```rust
// Mirrors JudgeManager.java:385-415 exactly. Two stages per candidate:
//   stage 1 (:396) adopt-if  = chosen.is_none() || chosen_is_judged || algorithm.prefers_second(..)
//   stage 2 (:407-413) judge-code refinement, which can also cancel the whole press (code == 6).
let mut chosen: Option<usize> = None;
for i in candidate_range {
    let n = &l.notes[i];
    let dm = n.head_us - press_us;
    if dm >= gate_early { break; }      // JudgeManager.java:387 `dmtime >= mjudgeend`
    if dm < gate_late { continue; }     // :390
    if n.is_mine || n.is_ln_end { continue; }   // :393
    let unjudged = !n.judged && !n.holding;     // mirrors `getState() == 0`

    let chosen_is_judged = matches!(chosen, Some(c) if l.notes[c].judged || l.notes[c].holding);
    let adopt_stage1 = chosen.is_none()
        || chosen_is_judged
        || self.algorithm.prefers_second(l.notes[chosen.unwrap()].head_us, n.head_us, press_us, &w, unjudged);
    if !adopt_stage1 { continue; }

    // MissCondition::One filter (JudgeManager.java:397-399) — see §3-B.
    if self.miss_condition == MissCondition::One
        && (!unjudged || (n.play_time_us.is_some() && !w.in_good_band(dm))) { continue; }

    // Judge code in the reference's 0..6 space (JudgeManager.java:400-404):
    //   already-judged note -> 5 (空POOR) when inside the MS band, else 6 (ignore)
    //   unjudged note       -> getJudge(); 4 (MS band) remaps to 5, 5 (outside) to 6
    let code = if !unjudged { if w.in_ms_band(dm) { 5 } else { 6 } }
               else { let c = w.judge_code(dm); if c >= 4 { c + 1 } else { c } };

    if code < 6 {
        // :408-410 — a 空POOR candidate (code 4/5) only wins on strictly smaller |dm|.
        if code < 4 || chosen.is_none()
            || (l.notes[chosen.unwrap()].head_us - press_us).abs() > dm.abs() { chosen = Some(i); }
    } else {
        chosen = None;      // :412 — a note this far out cancels the press entirely
    }
}
```

핵심 3가지(모두 레퍼런스 구조 그대로, 임의 단순화 금지):

1. **첫 후보를 무조건 채택하면 안 된다.** rbms 후보 루프(`matcher.rs:335-354`)는 재타격/空POOR 탐색을 위해 `judged`/`holding` 노트도 순회한다. 레퍼런스는 `tnote == null || tnote.getState() != 0 || algorithm.compare(...)`(`JudgeManager.java:396`)이라, **`t1` 이 판정완료면 알고리즘과 무관하게 다음 후보로 교체**된다. `None => chosen = Some(i)` 로만 쓰면 `Lowest`(항상 false)에서 판정완료 노트가 `t1` 로 고정되어 재판정되고 `empty_poor_hit` 경로가 우회된다.
2. **stage 2 가 空POOR/무시를 흡수한다.** 현 rbms 의 별도 `rehit` 수집(`matcher.rs:348-351`)과 `empty_poor_hit`(`392-397`)은 이 code 4/5/6 분기로 통합하는 것이 정본이다. 통합을 D3 안에서 미루려면 stage 1 만 이식하고 기존 `best`/`rehit` 2분기를 유지해도 되지만, 그 경우에도 **stage 1 의 `chosen_is_judged` 절을 반드시 넣는다**.
3. `w.judge_code(dm)` 는 `JudgeWindow.getJudge`(`JudgeProperty.java:186-191`) 대응으로, 어떤 밴드에도 안 들면 `테이블 쌍 수`(노트 5, LN 종단 4)를 반환한다. 신규 헬퍼 `judge_code` / `in_good_band` 는 `JudgeWindows` 에 추가한다(**D2 소유**).

`JudgeEngine` 에 `algorithm: JudgeAlgorithm` 필드 + `set_algorithm(&mut self, a: JudgeAlgorithm)` 추가, 기본값 `Combo`.

### 테스트
- `algorithm_table_pins`: 4종 각각에 대해 `(t1,t2,press)` 고정 픽스처 6조합의 `prefers_second` 결과를 배열로 박는다.
- `combo_prefers_upper_when_lower_past_good`: 아래 노트가 GOOD late 를 지난 상황에서 `Combo` 는 위 노트, `Duration` 은 아래 노트를 고르는 것을 같은 입력으로 대조.
- `lowest_always_first`: 후보 3개 중 항상 최소 인덱스.
- 회귀: 기존 matcher 테스트를 `set_algorithm(Duration)` 로 고정해 통과 유지 → 그 뒤 기본값 `Combo` 로 바꿔 달라지는 케이스만 신규 테스트로 분리.

---

## 2. J20/J21/J22/J26 — 9게이지 병렬 + 5세트 + MODIFY_DAMAGE + GAS

### 현재
`gauge.rs:45-56` 의 `spec()` 이 6종만, 값은 SEVENKEYS 0..5 와 동일. 인스턴스는 `JudgeEngine` 에 1개(`matcher.rs:269` `set_gauge`).

### 레퍼런스
`GrooveGauge.java:20-28` 인덱스 상수: 0 ASSISTEASY, 1 EASY, 2 NORMAL, 3 HARD, 4 EXHARD, 5 HAZARD, 6 CLASS, 7 EXCLASS, 8 EXHARDCLASS.
`GrooveGauge.java:35-41`: **9개를 전부 생성해 동시에 갱신**(`update` `GrooveGauge.java:57-61`, `addValue` :63-67), 표시/판정만 `type` 인덱스로 고른다.
세트는 `GaugeProperty.java:12-61` 의 5종(FIVEKEYS/SEVENKEYS/PMS/KEYBOARD/LR2), 원소 정의는 `GaugeProperty.java:77-125`.

### 정확 테이블 (Rust const 그대로 사용 가능)

공통 guts:

```rust
/// GaugeProperty.java:90 등. (threshold, multiplier) — value < threshold 인 첫 행의 배율을 damage 에 곱한다.
const GUTS_HARD:  &[(f32, f32)] = &[(10.0, 0.4), (20.0, 0.5), (30.0, 0.6), (40.0, 0.7), (50.0, 0.8)];
/// GaugeProperty.java:93 등 (CLASS 계열).
const GUTS_CLASS: &[(f32, f32)] = &[(5.0, 0.4), (10.0, 0.5), (15.0, 0.6), (20.0, 0.7), (25.0, 0.8)];
/// GaugeProperty.java:120 등 (LR2 HARD/CLASS/EXCLASS).
const GUTS_LR2:   &[(f32, f32)] = &[(30.0, 0.6)];
const GUTS_NONE:  &[(f32, f32)] = &[];
```

원소 스펙 필드 순서 = `(modifier, min, max, init, border, value[6], guts)`.
`value` 인덱스 = judge code 순서 **PG, GR, GD, BD, PR(見逃し), MS(空POOR)** (`GaugeProperty.java:133`).
`modifier` 는 `Some(Total) | Some(LimitIncrement) | Some(ModifyDamage) | None`.

각 세트의 배열 인덱스 0..8 = 위 GrooveGauge 상수 순서.

#### FIVEKEYS (`GaugeProperty.java:77-85`)

| idx | 이름 | modifier | min | max | init | border | value[PG,GR,GD,BD,PR,MS] | guts |
|---|---|---|---|---|---|---|---|---|
| 0 | ASSIST_EASY_5 | TOTAL | 2 | 100 | 20 | 50 | 1.0, 1.0, 0.5, -1.5, -3.0, -0.5 | NONE |
| 1 | EASY_5 | TOTAL | 2 | 100 | 20 | 75 | 1.0, 1.0, 0.5, -1.5, -4.5, -1.0 | NONE |
| 2 | NORMAL_5 | TOTAL | 2 | 100 | 20 | 75 | 1.0, 1.0, 0.5, -3.0, -6.0, -2.0 | NONE |
| 3 | HARD_5 | LIMIT_INCREMENT | 0 | 100 | 100 | 0 | 0, 0, 0, -5.0, -10.0, -5.0 | NONE |
| 4 | EXHARD_5 | MODIFY_DAMAGE | 0 | 100 | 100 | 0 | 0, 0, 0, -10.0, -20.0, -10.0 | NONE |
| 5 | HAZARD_5 | None | 0 | 100 | 100 | 0 | 0, 0, 0, -100.0, -100.0, -100.0 | NONE |
| 6 | CLASS_5 | None | 0 | 100 | 100 | 0 | 0.01, 0.01, 0, -0.5, -1.0, -0.5 | NONE |
| 7 | EXCLASS_5 | None | 0 | 100 | 100 | 0 | 0.01, 0.01, 0, -1.0, -2.0, -1.0 | NONE |
| 8 | EXHARDCLASS_5 | None | 0 | 100 | 100 | 0 | 0.01, 0.01, 0, -2.5, -5.0, -2.5 | NONE |

#### SEVENKEYS (`GaugeProperty.java:87-95`)

| idx | 이름 | modifier | min | max | init | border | value | guts |
|---|---|---|---|---|---|---|---|---|
| 0 | ASSIST_EASY | TOTAL | 2 | 100 | 20 | 60 | 1.0, 1.0, 0.5, -1.5, -3.0, -0.5 | NONE |
| 1 | EASY | TOTAL | 2 | 100 | 20 | 80 | 1.0, 1.0, 0.5, -1.5, -4.5, -1.0 | NONE |
| 2 | NORMAL | TOTAL | 2 | 100 | 20 | 80 | 1.0, 1.0, 0.5, -3.0, -6.0, -2.0 | NONE |
| 3 | HARD | LIMIT_INCREMENT | 0 | 100 | 100 | 0 | 0.15, 0.12, 0.03, -5.0, -10.0, -5.0 | HARD |
| 4 | EXHARD | LIMIT_INCREMENT | 0 | 100 | 100 | 0 | 0.15, 0.06, 0, -8.0, -16.0, -8.0 | NONE |
| 5 | HAZARD | None | 0 | 100 | 100 | 0 | 0.15, 0.06, 0, -100.0, -100.0, -10.0 | NONE |
| 6 | CLASS | None | 0 | 100 | 100 | 0 | 0.15, 0.12, 0.06, -1.5, -3.0, -1.5 | CLASS |
| 7 | EXCLASS | None | 0 | 100 | 100 | 0 | 0.15, 0.12, 0.03, -3.0, -6.0, -3.0 | NONE |
| 8 | EXHARDCLASS | None | 0 | 100 | 100 | 0 | 0.15, 0.06, 0, -5.0, -10.0, -5.0 | NONE |

> 현 rbms `gauge.rs:49-54` 는 이 표의 0..5 와 정확히 일치한다(HAZARD 의 MS 가 -10 인 것 포함). 즉 기존 6종은 SEVENKEYS 부분집합이므로 **7K 플레이 결과는 회귀 없이** 확장된다.

#### PMS (`GaugeProperty.java:97-105`)

| idx | 이름 | modifier | min | max | init | border | value | guts |
|---|---|---|---|---|---|---|---|---|
| 0 | ASSIST_EASY_PMS | TOTAL | 2 | 120 | 30 | 65 | 1.0, 1.0, 0.5, -1.0, -2.0, -2.0 | NONE |
| 1 | EASY_PMS | TOTAL | 2 | 120 | 30 | 85 | 1.0, 1.0, 0.5, -1.0, -3.0, -3.0 | NONE |
| 2 | NORMAL_PMS | TOTAL | 2 | 120 | 30 | 85 | 1.0, 1.0, 0.5, -2.0, -6.0, -6.0 | NONE |
| 3 | HARD_PMS | LIMIT_INCREMENT | 0 | 100 | 100 | 0 | 0.15, 0.12, 0.03, -5.0, -10.0, -10.0 | HARD |
| 4 | EXHARD_PMS | LIMIT_INCREMENT | 0 | 100 | 100 | 0 | 0.15, 0.06, 0, -10.0, -15.0, -15.0 | NONE |
| 5 | HAZARD_PMS | None | 0 | 100 | 100 | 0 | 0.15, 0.06, 0, -100.0, -100.0, -100.0 | NONE |
| 6 | CLASS_PMS | None | 0 | 100 | 100 | 0 | 0.15, 0.12, 0.06, -1.5, -3.0, -3.0 | CLASS |
| 7 | EXCLASS_PMS | None | 0 | 100 | 100 | 0 | 0.15, 0.12, 0.03, -3.0, -6.0, -6.0 | NONE |
| 8 | EXHARDCLASS_PMS | None | 0 | 100 | 100 | 0 | 0.15, 0.06, 0, -5.0, -10.0, -10.0 | NONE |

> PMS 0..2 는 **max 120 · init 30** 이다(다른 세트는 100/20). 계획 §1 J21 행의 "min2/max120/init30/border85" 와 일치.

#### KEYBOARD / 24K (`GaugeProperty.java:107-115`)

| idx | 이름 | modifier | min | max | init | border | value | guts |
|---|---|---|---|---|---|---|---|---|
| 0 | ASSIST_EASY_KB | TOTAL | 2 | 100 | 30 | 50 | 1.0, 1.0, 0.5, -1.0, -2.0, -1.0 | NONE |
| 1 | EASY_KB | TOTAL | 2 | 100 | 20 | 70 | 1.0, 1.0, 0.5, -1.0, -3.0, -1.0 | NONE |
| 2 | NORMAL_KB | TOTAL | 2 | 100 | 20 | 70 | 1.0, 1.0, 0.5, -2.0, -4.0, -2.0 | NONE |
| 3 | HARD_KB | LIMIT_INCREMENT | 0 | 100 | 100 | 0 | 0.2, 0.2, 0.1, -4.0, -8.0, -4.0 | HARD |
| 4 | EXHARD_KB | LIMIT_INCREMENT | 0 | 100 | 100 | 0 | 0.2, 0.1, 0, -6.0, -12.0, -6.0 | NONE |
| 5 | HAZARD_KB | None | 0 | 100 | 100 | 0 | 0.2, 0.1, 0, -100.0, -100.0, -100.0 | NONE |
| 6 | CLASS_KB | None | 0 | 100 | 100 | 0 | 0.2, 0.2, 0.1, -1.5, -3.0, -1.5 | CLASS |
| 7 | EXCLASS_KB | None | 0 | 100 | 100 | 0 | 0.2, 0.2, 0.1, -3.0, -6.0, -3.0 | NONE |
| 8 | EXHARDCLASS_KB | None | 0 | 100 | 100 | 0 | 0.2, 0.1, 0, -5.0, -10.0, -5.0 | NONE |

> KB 0 만 init 30, 1·2 는 init 20 이다(레퍼런스 그대로. 오타처럼 보이지만 **수정하지 않는다**).

#### LR2 (`GaugeProperty.java:117-125`)

| idx | 이름 | modifier | min | max | init | border | value | guts |
|---|---|---|---|---|---|---|---|---|
| 0 | ASSIST_EASY_LR2 | TOTAL | 2 | 100 | 20 | 60 | 1.2, 1.2, 0.6, -3.2, -4.8, -1.6 | NONE |
| 1 | EASY_LR2 | TOTAL | 2 | 100 | 20 | 80 | 1.2, 1.2, 0.6, -3.2, -4.8, -1.6 | NONE |
| 2 | NORMAL_LR2 | TOTAL | 2 | 100 | 20 | 80 | 1.0, 1.0, 0.5, -4.0, -6.0, -2.0 | NONE |
| 3 | HARD_LR2 | MODIFY_DAMAGE | 0 | 100 | 100 | 0 | 0.1, 0.1, 0.05, -6.0, -10.0, -2.0 | LR2 |
| 4 | EXHARD_LR2 | MODIFY_DAMAGE | 0 | 100 | 100 | 0 | 0.1, 0.1, 0.05, -12.0, -20.0, -2.0 | NONE |
| 5 | HAZARD_LR2 | None | 0 | 100 | 100 | 0 | 0.15, 0.06, 0, -100.0, -100.0, -10.0 | NONE |
| 6 | CLASS_LR2 | None | 0 | 100 | 100 | 0 | 0.10, 0.10, 0.05, -2.0, -3.0, -2.0 | LR2 |
| 7 | EXCLASS_LR2 | None | 0 | 100 | 100 | 0 | 0.10, 0.10, 0.05, -6.0, -10.0, -2.0 | LR2 |
| 8 | EXHARDCLASS_LR2 | None | 0 | 100 | 100 | 0 | 0.10, 0.10, 0.05, -12.0, -20.0, -2.0 | NONE |

### J22 — GaugeModifier 3종 (`GrooveGauge.java:255-294`)

TOTAL / LIMIT_INCREMENT 는 이미 `gauge.rs:79-95` 에 있고 레퍼런스와 동치. **MODIFY_DAMAGE 만 신규**:

```rust
/// GrooveGauge.java:274-291. Damage (f < 0) only; gains pass through.
const MODIFY_DAMAGE_FIX1_TOTAL: [f64; 10] = [240.0, 230.0, 210.0, 200.0, 180.0, 160.0, 150.0, 130.0, 120.0, 0.0];
const MODIFY_DAMAGE_FIX1_TABLE: [f32; 10] = [1.0, 1.11, 1.25, 1.5, 1.666, 2.0, 2.5, 3.333, 5.0, 10.0];

fn modify_damage(f: f32, total: f64, total_notes: usize) -> f32 {
    if f >= 0.0 { return f; }
    let mut i = 0usize;
    while i < MODIFY_DAMAGE_FIX1_TOTAL.len() - 1 && total < MODIFY_DAMAGE_FIX1_TOTAL[i] { i += 1; }
    let mut fix2 = 1.0f32;
    let mut note = 1000i32;
    let mut m = 0.002f32;
    let n = total_notes as i32;
    while note > n || note > 1 {
        fix2 += m * (note - n.max(note / 2)) as f32;
        note /= 2;
        m *= 2.0;
    }
    f * MODIFY_DAMAGE_FIX1_TABLE[i].max(fix2)
}
```

**적용 시점 — 게이지 생성 시 1회.** 레퍼런스는 `GaugeModifier` 를 판정마다가 아니라 `Gauge` 생성자에서 `value[]` 배열 전체에 적용한다(`GrooveGauge.java:260-291`. TOTAL/LIMIT_INCREMENT 와 같은 자리 = 현 `gauge.rs:73-92` `Gauge::new` 의 modifier 분기). **판정마다 호출하면 값이 달라진다.**

주의 3가지:
1. 루프 조건이 `note > totalNotes || note > 1` 이라 **totalNotes 가 커도 note 가 1 이 될 때까지 계속 돈다**(레퍼런스 그대로 옮긴다. 조기 종료로 "최적화"하면 값이 달라진다).
2. `fix1table` 값 `1.666` · `3.333` 은 f32 리터럴 그대로 쓴다(1/0.6 등으로 바꾸지 않는다).
3. `Math.max(fix1table[i], fix2)` 는 **곱셈 계수의 max** 이고, `f` 가 음수라 결과적으로 damage 가 더 커지는 쪽이 선택된다.

### J20 — 병렬 갱신 구조 (Rust)

`crates/rbms-judge/src/gauge.rs` 를 아래 형태로 재구성한다. 기존 `Gauge` public API(`new`/`update`/`add_value`/`value`/`kind`/`is_cleared`)는 **그대로 유지**해 호출부(`matcher.rs:269-273`)를 깨지 않는다.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GaugeSet { FiveKeys, SevenKeys, Pms, Keyboard, Lr2 }

/// GrooveGauge.java:20-28 index order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GaugeKind { AssistEasy, Easy, Normal, Hard, ExHard, Hazard, Class, ExClass, ExHardClass }

impl GaugeKind {
    pub const ALL: [GaugeKind; 9] = /* index 0..8 */;
    pub fn index(self) -> usize;
    pub fn from_index(i: usize) -> Option<GaugeKind>;
}

pub struct GaugeElement { modifier: Option<Modifier>, min: f32, max: f32, init: f32, border: f32, value: [f32; 6], guts: &'static [(f32, f32)] }

/// The 5 x 9 table above; new file `crates/rbms-judge/src/gauge_tables.rs`.
pub const GAUGE_TABLE: [[GaugeElement; 9]; 5];

/// GrooveGauge.java:35-121. All nine gauges advance on every judgment; `type` only selects
/// which one is displayed and which decides clear.
pub struct GrooveGauge {
    set: GaugeSet,
    kind: GaugeKind,
    gauges: [Gauge; 9],
}

impl GrooveGauge {
    pub fn new(set: GaugeSet, kind: GaugeKind, total: f64, notes: usize) -> Self;
    /// GrooveGauge.java:57-61. `rate` is 1.0 for a normal judgment, 0.5 for the HCN tick (J24).
    pub fn update(&mut self, judge: Judge, rate: f32);
    /// GrooveGauge.java:63-67 — mine damage hits every gauge.
    pub fn add_value(&mut self, delta: f32);
    pub fn value(&self) -> f32;              // selected
    pub fn value_of(&self, k: GaugeKind) -> f32;
    pub fn is_qualified(&self) -> bool;      // GrooveGauge.java:87-89
    pub fn kind(&self) -> GaugeKind;
    pub fn set_kind(&mut self, k: GaugeKind); // GAS shift
    pub fn is_type_changed(&self) -> bool;   // GrooveGauge.java:99-101
    pub fn is_course_gauge(&self) -> bool;   // type in Class..=ExHardClass
}
```

`GaugeSet::for_mode(mode: &Mode) -> GaugeSet` — BEAT_5K/10K → FiveKeys, BEAT_7K/14K → SevenKeys, POPN_9K → Pms, KEYBOARD_24K → Keyboard. **LR2 세트는 모드가 아니라 사용자 설정(J21 GAUGE SET = AUTO/LR2)** 으로만 선택된다(`GrooveGauge.java:123-149` 는 코스 제약으로 고르므로 rbms 는 설정 행으로 대체).

### J26 — GAS(Gauge Auto Shift) + bottom shiftable

`PlayerConfig.java:163-167` 5모드, `PlayerConfig.java:907-908` 클램프, 적용은 `BMSPlayer.java:638-670`.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GaugeAutoShift {
    #[default]
    None = 0,          // gauge 0 -> FAILED
    Continue = 1,      // keep playing at 0
    SurvivalToGroove = 2, // on 0: non-course gauge -> Normal
    BestClear = 3,
    SelectToUnder = 4,
}
```

BestClear / SelectToUnder 는 **매 프레임** 실행된다(0 도달 대기가 아니다, `BMSPlayer.java:639-651`):

```
len  = BestClear      ? (type >= Class ? ExHardClass+1 : Hazard+1)
     : SelectToUnder  ? (course ? min(max(config_gauge, Normal) + (Class - Normal), ExHardClass) + 1
                                : config_gauge + 1)
type = course ? Class
     : (current_type < bottom_shiftable ? current_type : bottom_shiftable)
for i in type..len { if gauge[i].value > 0 && gauge[i].is_qualified() { type = i } }
set_type(type)
```

None/Continue/SurvivalToGroove 는 선택 게이지가 0 이 된 시점에만 분기한다(`BMSPlayer.java:650-670`).
`bottom_shiftable_gauge` 는 **AssistEasy..=Normal(0..=2)** 로 클램프(`PlayerConfig.java:908`).

Rust 배치: `GrooveGauge` 에 `pub fn auto_shift(&mut self, mode: GaugeAutoShift, selected: GaugeKind, bottom: GaugeKind)` 를 두고, 매 프레임 호출은 `crates/rbms-play/src/lib.rs` 의 업데이트 루프에서 한다.

### 클리어 램프 매핑 (`ClearType.java:10-20, 51-60`)

| ClearType | id | 대응 gauge index |
|---|---|---|
| NoPlay | 0 | (없음) |
| Failed | 1 | (없음) |
| AssistEasy | 2 | (없음 — assist 강등 전용) |
| **LightAssistEasy** | 3 | 0 (ASSISTEASY) |
| Easy | 4 | 1 |
| Normal | 5 | 2, 6 (CLASS) |
| Hard | 6 | 3, 7 (EXCLASS) |
| ExHard | 7 | 4, 8 (EXHARDCLASS) |
| FullCombo | 8 | 5 (HAZARD) |
| Perfect | 9 | (없음) |
| Max | 10 | (없음) |

현 `gauge.rs:14-25` 의 `ClearType` 에 `LightAssistEasy` 를 **id 3 위치(AssistEasy 다음)** 로 삽입하고, `clear_lamp` 의 `GaugeKind::AssistEasy => ClearType::AssistEasy` 를 `LightAssistEasy` 로 고친다. HAZARD 클리어가 `FullCombo` 로 매핑되는 것도 그대로 반영한다(현재는 `ExHard`).
`ClearType` 의 직렬화(스코어 파일)는 이름 문자열이므로 **`rule_version` 갱신 + 구 기록 호환 처리**가 필요하다(결정 2 A안, `apps/rbms-player/src/scores.rs`).

### 테스트
- `gauge_table_pins_all_45_elements`: `GAUGE_TABLE` 을 `(modifier tag, min, max, init, border, value[6], guts)` 튜플 45개 리터럴 배열과 `assert_eq!` — 위 5개 표를 그대로 옮겨 적는다. 이 테스트가 byte 고정 역할을 한다.
- `guts_tables_pin`: 3개 guts 배열 값 고정.
- `modify_damage_matches_reference_samples`: `(total, notes)` = (240,1000) (200,500) (130,100) (60,20) (300,2000) 5조합에 대해 계산값을 상수로 고정. 값은 위 알고리즘을 **레퍼런스와 동일한 순서로** 계산한 결과여야 한다(구현 후 실측해 상수화, 임의 기대값 금지).
- `nine_gauges_advance_together`: 판정 1회 후 9개 값이 각자 스펙대로 변한 것을 확인.
- `hazard_clear_maps_to_full_combo` / `assist_easy_gauge_maps_to_light_assist_easy`.
- `gas_best_clear_picks_highest_qualified`, `gas_select_to_under_respects_bottom_shiftable`, `gas_survival_to_groove_only_on_zero`, `gas_none_fails`.
- 회귀: 기존 `gauge_tests`(`gauge.rs:165-` )는 SevenKeys 세트 기준으로 전부 통과해야 한다.

---

## 3. J23 + J12 2단계 — JudgeWindowRule (fixjudge · fixmin/fixmax)

### 현재
`windows.rs:178-182` `scaled()` 는 judgerank 를 PG/GR/GD/BD 에 균일 적용하고 MS 만 고정한다. PMS 의 per-index `fixjudge` 도, `fixmin/fixmax` 클램프도 없다.
`with_window_rate`(`windows.rs:188-208`)는 JUDGE WIDTH 3티어 + BAD/이전티어 클램프까지 이미 레퍼런스 `JudgeProperty.java:262-273` 와 동치다.

### 레퍼런스 (`JudgeProperty.java:209-211, 222-276`)

```rust
/// JudgeProperty.java:209-211. Rows = JUDGERANK level (VERYHARD, HARD, NORMAL, EASY, VERYEASY);
/// columns = judge index (PG, GR, GD, BD, MS).
pub const WINDOW_RULE_NORMAL_JUDGERANK: [[i32; 5]; 5] = [
    [ 25,  25,  25,  25, 100],
    [ 50,  50,  50,  50, 100],
    [ 75,  75,  75,  75, 100],
    [100, 100, 100, 100, 100],
    [125, 125, 125, 125, 100],
];
pub const WINDOW_RULE_NORMAL_FIXJUDGE: [bool; 5] = [false, false, false, false, true];

pub const WINDOW_RULE_PMS_JUDGERANK: [[i32; 5]; 5] = [
    [100,  33,  33, 100, 100],
    [100,  50,  50, 100, 100],
    [100,  70,  70, 100, 100],
    [100, 100, 100, 100, 100],
    [100, 133, 133, 100, 100],
];
pub const WINDOW_RULE_PMS_FIXJUDGE: [bool; 5] = [true, false, false, true, true];
```

`getJudgeRank(judgerank)` (`JudgeProperty.java:222-224`): 인덱스 i 가 `fixjudge[i]` 면 100, 아니면 인자 judgerank. → PMS 는 PG/BD/MS 가 judgerank 에 **불변**이고 GR/GD 만 스케일된다.

`create(org, judgerank[], judgeWindowRate[])` (`JudgeProperty.java:230-276`) 3단계:
1. 각 인덱스 i, 각 bound j: `judge[i*2+j] = fixjudge[i] ? org[i*2+j] : org[i*2+j] * judgerank[i] / 100`
2. **fixmin/fixmax 클램프** (`:238-260`) — `min(org.len(), 4)` 범위 순회. `fixmin` = 지금까지 만난 마지막 fixjudge 인덱스, `fixmax` = i 다음의 첫 fixjudge 인덱스(4 미만). 각 bound 에 대해
   - `fixmin != -1 && |judge[i]| < |judge[fixmin]|` → `judge[i] = judge[fixmin]`
   - `fixmax != -1 && |judge[i]| > |judge[fixmax]|` → `judge[i] = judge[fixmax]`
3. JUDGE WIDTH rate (`:263-273`) — 이미 `with_window_rate` 로 구현된 로직.

PMS 의 실제 효과: fixjudge = [T,F,F,T,T] → fixmin=0(PG), fixmax=3(BD). 즉 GR/GD 는 **PG 보다 좁아질 수 없고 BAD 보다 넓어질 수 없다**.

### 변경 (Rust)

`windows.rs` 에 규칙 타입 추가하고 `JudgeProperty` 에 필드 1개 추가:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JudgeWindowRule { Normal, Pms }

impl JudgeWindowRule {
    pub fn judgerank_table(self) -> [[i32; 5]; 5];
    pub fn fixjudge(self) -> [bool; 5];
    /// JudgeProperty.java:222-224.
    pub fn judgerank_per_index(self, judgerank: i32) -> [i32; 5];
    /// JudgeProperty.java:230-276, steps 1 and 2 only. `with_window_rate` remains step 3.
    pub fn create(self, org: &JudgeWindows, judgerank: [i32; 5]) -> JudgeWindows;
}

pub struct JudgeProperty { /* existing fields */ pub window_rule: JudgeWindowRule }
```

- FIVEKEYS/SEVENKEYS/KEYBOARD → `Normal`, PMS → `Pms` (`JudgeProperty.java:21,32,43,54`).
- `JudgeWindows::scaled(judgerank)` 는 **`JudgeWindowRule::Normal` 고정 하위호환 래퍼**로 남기고(deprecated 아님, 테스트 다수가 씀), 실사용 경로(`matcher.rs:231-240` `apply_mode`, `crates/rbms-play/src/lib.rs:132-141`)를 `rule.create(...)` 로 교체한다.
- LN 종단 테이블은 원소가 4쌍(8개)이라 `org.len()/2 == 4` → 인덱스 4(MS) 는 없다. 루프 상한 `min(org.len(), 4)` 를 그대로 지켜 패닉을 막는다.

### 3-B. J23 나머지 절반 — `judge_vanish` · `MissCondition` 엔진 배선

계획 §1 J23 은 "PMS 판정 규칙(MissCondition · JudgeWindowRule.PMS fixjudge) — Phase A 는 데이터화만, **엔진 배선은 Phase D**" 다. `fixjudge`/`fixmin`/`fixmax` 만으로는 J23 이 닫히지 않는다.

**실측**: `crates/rbms-judge/src/windows.rs:46-49` 에 `combo` / `judge_vanish` / `miss_condition` 필드와 4행 데이터(`:70-71, 83-84, 98-99, 112-113`)가 이미 있으나, **이 세 필드를 읽는 코드는 같은 파일의 테스트(`:372-374`) 외에 크레이트·앱 어디에도 없다**(순수 미사용 데이터). 배선하지 않으면 Phase D 종료 후에도 J23 은 미해소로 남는다.

레퍼런스 값(`JudgeProperty.java:14-55`, 인덱스 = PG,GR,GD,BD,PR,MS):

| 행 | `combo` | `judge_vanish` | `miss` | `longnoteMargin` |
|---|---|---|---|---|
| FIVEKEYS | T,T,T,F,F,F | T,T,T,T,T,F | ALWAYS | 0 |
| SEVENKEYS | T,T,T,F,F,T | T,T,T,T,T,F | ALWAYS | 0 |
| PMS | T,T,T,F,F,F | **T,T,T,F,T,F** | **ONE** | **200000** |
| KEYBOARD | T,T,T,F,F,T | T,T,T,T,T,F | ALWAYS | 0 |

**(a) `judge_vanish[judge]` = "이 판정이 노트를 소비하는가".** `JudgeManager.java:428, 444, 452, 458` 이 매 판정에 전달하고, `updateMicro`(`:639-646`)가 true 일 때만 `setState(judge+1)` + `passnotes++` 를 한다. PMS 는 BD=false 이므로 **BAD 를 맞아도 노트가 살아남아** 나중에 見逃し 대상이 되고, 그 사이 재타격 후보가 된다. LN/CN 헤드도 `judgeVanish[judge]` 가 true 일 때만 `processing`(=`holding`) 으로 넘어간다(`:428, :444`).
- rbms 대응: `press` 에서 `note.judged = true` / `note.holding = true` 를 세우는 지점(`matcher.rs:372, 386`)을 `w_prop.judge_vanish[judge as usize]` 로 게이팅한다. 카운트(`apply`)는 vanish 여부와 무관하게 수행한다.

**(b) `MissCondition::One`** (`JudgeProperty.java:201-203`) 은 두 곳에서만 쓰인다.
1. 후보 필터(`JudgeManager.java:397-399`): `miss == ONE` 이면 (i) 이미 판정된 노트는 후보에서 제외, (ii) 미판정이라도 `playTime != 0`(= 이미 한 번 판정을 받아 살아남은 노트)이면 **GOOD 밴드 안**(`getTime(type,2,early/late)`)일 때만 후보. → §1 의사코드에 이미 반영됨.
2. 이중 카운트 억제(`JudgeManager.java:648`): `miss == ONE && judge == 4(見逃し) && playTime != 0` 이면 `updateMicro` 가 **소비(vanish) 처리 뒤 즉시 return** 하여 판정 카운트·콤보·타이밍 기록을 하지 않는다.
- rbms 대응: `Note` 에 `play_time_us: Option<i64>`(레퍼런스 `getPlayTime`) 추가. `apply`/`record_timing` 앞에 위 조건을 둔다.

**(c) `combo[judge]`** 는 이미 데이터가 있으나 콤보 갱신 경로가 이를 읽는지 D3 착수 시 확인한다(레퍼런스 `combocond[judge] && judge < 5`, `JudgeManager.java:660`).

### J12 2단계 — 키/스크래치 분리
`crates/rbms-play/src/lib.rs:132-141` `set_judge_window_rates(key: [i32;3], scratch: [i32;3])` 는 **이미 분리되어 있다**. Phase D 가 할 일은 앱이 6개 값을 실제로 전달하는 것뿐(§7). `set_judge_rate`(`lib.rs:125-127`)은 6값을 같은 값으로 채우는 편의 함수로 유지한다.

### 테스트
- `window_rule_tables_pin`: 위 4개 상수 배열을 리터럴로 재기입해 대조.
- `pms_judgerank_fixes_pg_bd_ms`: `judgerank_per_index(75)` == `[100, 75, 75, 100, 100]`.
- `pms_create_clamps_great_to_pgreat_floor`: judgerank 33 에서 GR/GD 가 PG 바운드보다 좁아지지 않음.
- `pms_create_clamps_good_to_bad_ceiling`: judgerank 133 에서 GD 가 BD 를 넘지 않음.
- `normal_rule_matches_legacy_scaled`: `JudgeWindowRule::Normal.create(w, [r;5])` == 기존 `w.scaled(r)` (7K/5K/KB 3행 × judgerank 25/50/75/100/125 전수).
- `ln_end_rule_does_not_panic_on_four_pairs`.
- **`judge_property_tables_pin`** (계획 §2 "레퍼런스 `JudgeProperty` **전 배열** 대조" 이행): FIVEKEYS/SEVENKEYS/PMS/KEYBOARD 4행 × (`note`, `scratch`, `ln_end`, `ln_scratch_end`) 4테이블 + `longnote_margin` + `combo` + `judge_vanish` + `miss_condition` 을 리터럴로 재기입해 `assert_eq!`. 레퍼런스 원문은 `JudgeProperty.java:13-55`.
  - **주의(오타 아님)**: FIVEKEYS `ln_scratch_end` = `{-130000,130000, -160000,160000, -110000,110000, -260000,260000}` 로 3번째 쌍이 2번째보다 **좁다**(비단조). `JudgeProperty.java:16` 원문 그대로이므로 **고치지 말 것**. 이 pin 테스트가 "충실 이식"임을 고정한다.
  - PMS `scratch` / `ln_scratch_end` 는 레퍼런스에서 **빈 배열**(`new long[]{}`)이다. rbms 가 무엇을 넣었는지 확인해 동치 처리(빈 테이블 = 스크래치 없음)를 pin 한다.
- `pms_bad_does_not_consume_note` / `pms_miss_condition_one_counts_poor_once` / `sevenkeys_bad_consumes_note` — 3-B 배선 회귀.

---

## 4. J9 / A10 — 스크래치 윈도우 · 정역 2키 · BSS/MSS

### 현재
- 스크래치 윈도우 테이블 자체는 이식 완료(`windows.rs:64,77,92,106`), 레인별 선택도 있다(`matcher.rs:246-256`).
- **미구현**: 레인당 2키(정/역), BSS(Back Spin Scratch) 종단, MSS(Multi Spin Scratch).
- 키 바인딩은 레인당 1키(`apps/rbms-player/src/keyconfig.rs:193-211` `lane_keys`).

### 레퍼런스
`JudgeManager.java` 는 `sckey[sc]` 배열로 "지금 이 스크래치 레인을 잡고 있는 물리 키"를 기억한다.
- **BSS 종단** (`JudgeManager.java:358-372`): CN/HCN 을 처리 중인 스크래치 레인에서 **다른 방향 키가 눌리면** 그것이 종단 판정이다. `dmtime = processing.time - pmtime`, `window.getJudge(LONGSCRATCH_END, dmtime)` 로 판정하고 `processing = null`, **`releasetime = Long.MIN_VALUE`, `lnendJudge = Integer.MIN_VALUE`**, `sckey[sc] = 0` 를 **모두** 리셋한다(`:368-370`). §5 의 deferral 상태가 남지 않도록 이 3개를 함께 비우는 것이 필수다.
- **같은 방향 키를 다시 누르면** 종단이 아니라 "누름 재개"(`releasetime = MIN`) (`:373-375`).
- **스크래치 레인 점유(`sckey[sc] = key`)는 LN/CN 헤드를 잡는 순간 설정된다** (`JudgeManager.java:435`(LN) · `:449`(CN/HCN)). 단 **`judgeVanish[judge]` 가 true 일 때만** — 즉 헤드 판정이 노트를 소비했을 때만 점유가 성립한다. 같은 자리에서 `releasetime = MIN`, `lnendJudge = MIN` 도 함께 초기화된다.
- **BSS 도중 릴리스** (`JudgeManager.java:501-508`) — 조건이 둘이다:
  ```java
  if (judge != 4 || key != sckey[sc]) { release = false; } else { sckey[sc] = 0; }
  ```
  `judge` 는 `getJudge(LONGSCRATCH_END, dmtime)` 결과이고, LN 종단 테이블은 4쌍이므로 **`judge == 4` 는 "어떤 밴드에도 안 듦 = 종단 윈도우 완전 밖"** 을 뜻한다. 즉 스크래치의 중간 릴리스는 **(i) 종단 윈도우 밖이고 (ii) 릴리스한 키가 점유 키일 때만** 성립하고, 그 외에는 **릴리스 자체를 무시**한다(`release = false`). 종단 윈도우 안에서 뗀 스크래치는 릴리스로 확정되지 않는다.
  → 성립한 경우에도 즉시 확정이 아니라 §5 의 deferral 조건(`judge >= 3 && dmtime > 0`)을 거친다. `judge == 4` 이므로 스크래치 CN 의 중간 릴리스는 `dmtime > 0`(이르게 뗌)이면 **항상 보류**된다.
- **plain LN(비 CN) 릴리스**는 조건이 하나뿐이다(`JudgeManager.java:525`): `key != sckey[sc]` 면 무시. `judge != 4` 절이 **없다**.
- **MSS**: 같은 레인에서 CN 종단 직후 다시 반대 방향으로 이어지는 연속 스핀. 위 두 규칙(다른 키 = 종단, `sckey` 리셋)이 조합되어 자연히 성립한다 — 별도 분기가 아니다.
- 見逃し POOR 로 CN 종단이 소실될 때도 `sckey[state.sckey] = 0` 으로 되돌린다(`JudgeManager.java:617-622`).

### 변경 (Rust)

`matcher.rs`:

```rust
/// Physical direction of a scratch input. Key lanes always use `Forward`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScratchDir { Forward, Backward }

impl JudgeEngine {
    pub fn press_dir(&mut self, lane: usize, dir: ScratchDir, press_us: i64) -> Option<JudgeResult>;
    pub fn release_dir(&mut self, lane: usize, dir: ScratchDir, release_us: i64) -> Option<JudgeResult>;
}
```

- 기존 `press`/`release` 는 `press_dir(lane, ScratchDir::Forward, t)` 로 위임해 **호출부 무변경**.
- `Lane` 에 `sc_dir: Option<ScratchDir>` (레퍼런스 `sckey[sc]` 대응, `None` = 미점유) 추가.
- `sc_dir` 은 **헤드 판정이 노트를 소비(§3-B `judge_vanish`)했을 때** `Some(dir)` 로 설정한다(레퍼런스 `:435`/`:449`). 그 전에는 `None` 이므로 첫 스핀은 언제나 정상 시작한다.
- press 시 `holding_cn && lane.sc_dir.is_some() && Some(dir) != lane.sc_dir` → BSS 종단 경로. 종단에서 `sc_dir = None`, `release_us = None`, `end_judge = None` 을 함께 비운다.
- press 시 같은 방향이면 "누름 재개": `release_us = None` 만 (§5 deferral 취소).
- release 시:
  - CN/HCN 스크래치: **`judge_code == 4`(종단 윈도우 밖) 이고 `Some(dir) == lane.sc_dir`** 일 때만 릴리스 처리 + `sc_dir = None`. 그 외에는 **아무 것도 하지 않는다**.
  - plain LN 스크래치: `Some(dir) == lane.sc_dir` 만 검사.
  - 키 레인(`sc_dir` 개념 없음): 기존대로 무조건 처리.

`apps/rbms-player/src/keyconfig.rs`: 스크래치 레인에 **정/역 2개 토큰**을 허용한다. 기존 RON 은 레인당 문자열 1개이므로 후방호환을 위해 `"UP|DOWN"` 같은 구분자 확장이 아니라, 스크래치 레인 전용 **보조 맵**을 추가한다:

```rust
pub struct KeyConfig {
    lanes: HashMap<String, Vec<String>>,          // 기존
    scratch_reverse: HashMap<String, Vec<String>>, // 신규: mode key -> lane index -> token
}
```
`lane_keys` 는 그대로 두고 `pub fn scratch_reverse_keys(&self, mode: Mode) -> Vec<(KeyCode, usize)>` 를 신설한다(기본값: 7K 스크래치 역회전 = 기존 키의 짝, 미설정이면 빈 목록 → 현행 1키 동작 유지).

### 테스트
- `bss_end_on_opposite_direction`: CN 스크래치 유지 중 반대 키 press → `LONGSCRATCH_END` 윈도우로 판정되고 노트가 소비됨.
- `bss_same_direction_repress_is_not_end`.
- `bss_release_of_other_key_is_ignored`.
- `bss_release_inside_end_window_is_ignored`: 종단 윈도우 **안**에서 점유 키를 떼도 (judge != 4) 릴리스가 무시되는지 — `JudgeManager.java:503` 의 `judge != 4` 절 회귀.
- `plain_ln_scratch_release_ignores_judge_code`: plain LN 은 `judge != 4` 절이 없음(`:525`).
- `scratch_owner_set_only_when_head_vanished`: `judge_vanish` 가 false 인 헤드 판정에서는 `sc_dir` 이 점유되지 않음.
- `mss_two_consecutive_spins`: 종단 후 `sc_dir` 이 비워져 다음 스핀이 정상 시작.
- `cn_end_missed_resets_scratch_owner`.
- `forward_only_input_matches_pre_phase_d_behaviour`: 기존 스크래치 테스트 전수 회귀.

---

## 5. J24 — CN deferral + HCN 연속 게이지

### 현재
`matcher.rs:369-381`: CN/HCN 은 press 에서 헤드 판정을 즉시 확정하고, `release`(`matcher.rs:399-424`)에서 종단을 판정한다. **deferral 없음**, HCN 홀드 중 게이지 변화 없음.

### 레퍼런스

**(a) CN/HCN 릴리스 deferral** (`JudgeManager.java:492-521`):
```
judge = window.getJudge(sc>=0 ? LONGSCRATCH_END : LONGNOTE_END, dmtime)
if judge >= 3 && dmtime > 0 {     // BAD 이하이면서 '이르게' 뗀 경우
    state.releasetime = mtime; state.lnendJudge = judge;   // 확정 보류
} else {
    updateMicro(...) 즉시 확정
}
```
보류된 것은 프레임 루프에서 회수한다(`JudgeManager.java:579-589`): `releasetime + releasemargin <= mtime` 이면 `lnendJudge` 로 확정. `releasemargin` = 키 레인은 `nreleasemargin`(= `longnoteMargin` × 사용자 LN 마진 rate), 스크래치는 `sreleasemargin`.
즉 **너무 일찍 뗐어도 마진 안에 다시 잡으면 살아난다**. 되잡는 경로는 press 쪽 `releasetime = Long.MIN_VALUE`(`JudgeManager.java:374, 379`).

**(a-2) plain LN 의 확정 규칙** (`JudgeManager.java:563-577`, 이번에 통독해 확정):
```
if releasetime != MIN && releasetime + releasemargin <= mtime:
    updateMicro(pair, lnendJudge, processing.time - releasetime)      # 마진 만료 → 보류 판정으로 확정
elif processing.time < mtime:
    updateMicro(pair, lnstartJudge, lnstartDuration)                  # 종단 시각 통과 → 헤드 판정을 종단에도 부여
```
즉 plain LN 은 **헤드 판정 1개가 종단까지 그대로 이어지는** 모델이고(`lnstartJudge`/`lnstartDuration` 을 헤드에서 저장, `:429-430`), 이르게 뗐을 때만 `lnendJudge` 로 덮인다. `releasemargin` 은 CN 과 동일하게 키/스크 분리(`:559`).
현 rbms `release`(`matcher.rs:399-420`)는 릴리스 시점에 단일 판정을 내리므로, **"종단 시각을 지나면 헤드 판정으로 확정"** 경로가 없다 → `update` 회수 루프에 이 분기를 함께 추가한다. `Lane` 에 `start_judge: Option<Judge>` / `start_delta_us: Option<i64>` 를 둔다(레퍼런스 `lnstartJudge`/`lnstartDuration`).

**(b) HCN 연속 게이지** (`JudgeManager.java:81, 310-336`):
```rust
/// JudgeManager.java:81.
const HCN_GAUGE_TICK_US: i64 = 200_000;
```
- 홀드 중(`inclease == true`): `mpassingcount += dt`; `mpassingcount > HCN_GAUGE_TICK_US` 마다 `gauge.update(judge=1 /* GREAT */, rate=0.5)` 후 `mpassingcount -= HCN_GAUGE_TICK_US`.
- 놓친 중(`inclease == false`): `mpassingcount -= dt`; `< -HCN_GAUGE_TICK_US` 마다 `gauge.update(judge=3 /* BAD */, rate=0.5)` 후 `mpassingcount += HCN_GAUGE_TICK_US`.
- 즉 200 ms 마다 GREAT 회복량의 절반 / BAD 감소량의 절반. 판정 카운트·콤보·EX 스코어에는 **영향 없다**(게이지만).
- 키음 볼륨도 함께 토글되지만 이는 오디오 영역(Phase D 범위 밖, 앱 통합 시 참고).

### 변경 (Rust)

`crates/rbms-judge/src/matcher.rs` (또는 신규 `ln.rs` 로 분리):

```rust
struct LaneHold {
    /// JudgeManager LaneState.releasetime; `None` = not deferred.
    release_us: Option<i64>,
    /// JudgeManager LaneState.lnendJudge.
    end_judge: Option<Judge>,
    /// JudgeManager LaneState.mpassingcount (HCN only).
    passing_us: i64,
    /// JudgeManager LaneState.inclease.
    increasing: bool,
}
```

- `release_dir` 에서 CN/HCN 이고 `judge >= Judge::Bad && dm > 0` 이면 확정 대신 `release_us = Some(t)`, `end_judge = Some(judge)`.
- `press_dir` 에서 같은 레인 재점유 시 `release_us = None`.
- `update(now_us)`(`matcher.rs:425-489`)에 회수 루프 추가: `release_us + ln_margin(lane) <= now_us` → `end_judge` 확정.
- `update` 에 HCN 틱 처리 추가: `prev_now_us` 를 보관해 `dt` 를 만들고 위 알고리즘 그대로. 게이지 호출은 `GrooveGauge::update(Judge::Great, 0.5)` / `(Judge::Bad, 0.5)`.
- `ln_margin`(`matcher.rs:261-267`)은 이미 키/스크 분리 + 사용자 rate 반영 상태 → 그대로 재사용.

### 테스트
- `cn_early_release_within_margin_is_recovered`: 마진 내 재점유 → 종단이 정상 판정.
- `cn_early_release_past_margin_commits_deferred_judge`: 마진 초과 → 보류했던 judge 로 확정.
- `cn_late_or_good_release_commits_immediately`: `judge < Bad` 또는 `dm <= 0` 이면 즉시.
- `hcn_hold_ticks_gauge_every_200ms`: 1,000,000 µs 홀드 → 정확히 5틱, 게이지 증가량 == `5 * great_delta * 0.5`.
- `hcn_release_ticks_damage_every_200ms`.
- `hcn_ticks_do_not_change_counts_or_combo`.
- `hcn_partial_tick_carries_remainder`: 250 ms + 100 ms 두 프레임 → 총 1틱 + 잔여 150 ms.
- `plain_ln_past_end_time_commits_head_judge`: plain LN 을 계속 누른 채 종단 시각을 지나면 헤드 판정으로 확정(`JudgeManager.java:572-574`).
- `plain_ln_early_release_past_margin_commits_end_judge`.
- `cn_end_missed_resets_release_state`: 見逃し로 CN 종단이 소실될 때 `release_us`/`end_judge`/`sc_dir` 3개가 함께 비워짐(`JudgeManager.java:617-625`).

---

## 6. J25 / J6 — lnmode 강제 · 24K 모드

### J25 lnmode (`PlayerConfig.java:109, 328-333, 925`)
`lnmode` 0..2 클램프. 의미(`JudgeManager.java:235-236, 262-275` 의 `lntype` 사용):
| 값 | 의미 |
|---|---|
| 0 | LN (`BMSModel.LNTYPE_LONGNOTE`) |
| 1 | CN (charge note) |
| 2 | HCN (hell charge note) |

`TYPE_UNDEFINED` 인 노트는 이 `lntype` 을 따라간다. 명시적으로 `TYPE_LONGNOTE` / `TYPE_CHARGENOTE` / `TYPE_HELLCHARGENOTE` 인 노트는 **차트 지정이 우선**한다.

> **주의 — 스케일이 둘이다(이번에 확인).** BMS 차트 헤더 `#LNMODE` 는 `0=미지정 / 1=LN / 2=CN / 3=HCN` 이고, 플레이어 설정 `PlayerConfig.lnmode` 는 `MathUtils.clamp(lnmode, 0, 2)`(`PlayerConfig.java:925`)로 `0=LN / 1=CN / 2=HCN` 이다. 계획 §1 J25 의 "0~3" 은 **차트 헤더 쪽 스케일**이므로 그 자체가 오류는 아니다. 두 스케일을 문서에서 구분해 적는다.

**적용 지점 — 판정 크레이트가 아니라 차트 변환부다(이번에 확인해 정정).** 레퍼런스는 플레이어 설정 lnmode 를 **디코더 생성자 인자**로 넘겨(`new BMSDecoder(lntype)`, `ChartInformation(source, lntype, ...)`) `model.getLntype()` 에 담고, `JudgeManager` 는 `lntype = model.getLntype()`(`JudgeManager.java:142`)를 읽어 **노트 타입이 `TYPE_UNDEFINED` 인 경우에만** 적용한다(`:262, :273, :356, :495, :615`). 즉 "차트 지정 우선, 미지정은 설정값" 이며 적용 자체는 모델 레벨이다.

**rbms 실측 — 현재 `LnKind` 에 "미지정"이 없다.** `crates/rbms-model/src/lib.rs:31-35` 는 `Ln | Cn | Hcn` 3값뿐이고, `crates/rbms-chart/src/lib.rs:99-107` `to_model` 이 `#LNMODE` 를 읽으며 **`0`(미지정)을 그 자리에서 `LnKind::Ln` 으로 접어버린다**. 그래서 판정 크레이트에는 "미지정" 정보가 도달하지 않는다.

**필요한 변경(2군데 + 1군데):**
1. `crates/rbms-model/src/lib.rs`: `LnKind` 에 `Undefined` 변형 추가. (기존 사용처는 `matches!` 라 비-exhaustive 안전: `matcher.rs:25`, `rbms-chart/src/lib.rs:337`. 컴파일 확인 필수)
2. `crates/rbms-chart/src/lib.rs:99-107`: `0 => LnKind::Undefined` 로 바꾸고, `to_model` 에 플레이어 lnmode 인자를 추가해 `Undefined` 를 `Ln/Cn/Hcn` 으로 해소한다(레퍼런스의 디코더 인자와 동일 위치).
3. 판정 크레이트는 해소된 값을 그대로 쓰므로 `matcher` 변경은 원칙적으로 불필요하다. 다만 `to_model` 을 거치지 않는 경로(테스트 픽스처 등)를 위해 아래 방어용 API 를 둔다.

rbms: `matcher.rs:128-186` `from_triples` 가 `LnKind` 를 노트에서 직접 읽는다. 보조 API 는
```rust
/// PlayerConfig.java:109. 0 = LN, 1 = CN, 2 = HCN. Applies to notes whose chart type is undefined.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LnMode { #[default] LongNote, ChargeNote, HellChargeNote }

impl JudgeEngine { pub fn set_ln_mode(&mut self, m: LnMode); }
```
`from_model*` 이 `LnKind::Undefined` 인 노트에 대해 `ln_mode` 를 적용한다(정본 적용 지점은 위 2번 `to_model`).

### J6 24K (KEYBOARD)
- 윈도우 행 `JudgeProperty::KEYBOARD` 는 이미 있다(`windows.rs:104-114`) — 도달 경로만 없다.
- 게이지 세트 `GaugeSet::Keyboard` 는 §2 에서 신설.
- 필요한 것: `rbms_model::Mode` 에 24K 모드 추가 + `JudgeProperty::for_mode` 분기 + `GaugeSet::for_mode` 분기 + 기본 키 바인딩.
- **파일 위치 정정**: `Mode` 는 `crates/rbms-model/src/lib.rs` 가 아니라 **`crates/rbms-model/src/mode.rs`** 에 있다(`lib.rs:1` `pub mod mode;`, `:3` `pub use mode::Mode;`). 현 상수는 `BEAT_7K(key 8, scratch [7])` · `BEAT_5K(6, [5])` · `BEAT_10K(12, [5,11])` · `BEAT_14K(16, [7,15])` · `POPN_9K(9, [])` 5개이고 `Mode::ALL` 도 같은 파일이다.
- `Mode` 는 `channel_assign: &[i8; 18]`(raw 18채널 → 논리 레인)을 요구한다. 24K 는 채널 폭이 18을 넘으므로 **`channel_assign` 배열 폭 자체를 확장해야 할 가능성이 크다** — D4 착수 시 `Mode`/`lane_of_raw`(`mode.rs:32-38`) 와 파서의 채널 스캔을 먼저 통독해 결정한다(§11-1).
- `JudgeProperty::for_mode`(`windows.rs:117-123`)는 `mode.name` 문자열 매칭이다. **KEYBOARD 라우팅 분기는 D2 가 아니라 D4 에서** 넣는다 — D2 시점에는 대응 `Mode` 상수가 없어 사문(dead) 분기가 되고, 문자열 리터럴을 확정할 근거도 없다.
- Phase D 범위에서는 **판정/게이지 데이터 경로만** 연결하고, 렌더 레인 레이아웃·차트 스캔 필터는 Phase G 로 미룬다.

### 테스트
- `lnmode_forces_undefined_ln_to_cn` / `_to_hcn`, `lnmode_does_not_override_explicit_chart_type`.
- `keyboard_mode_selects_keyboard_windows_and_gauge_set`.

---

## 6.5. DJ 랭크 산출식 대조 (계획 §5 문서 정정 항목)

계획 §5 는 "`clear_lamp` ↔ `ClearType.getClearTypeByGauge`, **DJ 랭크 산출식은 미대조(Phase D에서 대조)**" 를 남겼다. 이번에 양쪽을 직접 통독해 대조했다.

**레퍼런스** (`ScoreDataProperty.java:35, 94, 101-102` + `BooleanPropertyFactory.java:280-288`):
```java
rate = totalnotes == 0 ? 1.0f : ((float) exscore) / (totalnotes * 2);
for (int i = 0; i < 27; i++) rank[i] = totalnotes != 0 && rate >= 1f * i / 27;   // rank.length == 27
// 밴드 경계 인덱스: {0, 6, 9, 12, 15, 18, 21, 24, 28} = F, E, D, C, B, A, AA, AAA
```
→ 경계는 `6/27, 9/27, 12/27, 15/27, 18/27, 21/27, 24/27` = **`2/9, 3/9, 4/9, 5/9, 6/9, 7/9, 8/9`**.

**rbms** (`crates/rbms-render/src/result.rs:77-100`): `RANK_BANDS` 8밴드 `F,E,D,C,B,A,AA,AAA` + `RANK_BOUNDS = [0, 2/9, 3/9, 4/9, 5/9, 6/9, 7/9, 8/9, 1]`, `dj_rank(ex, max_ex)` 가 `rate = ex/max_ex` 로 `rposition(rate >= b)`.

**결론: 밴드 경계·개수·순서 전부 일치.** 남은 차이는 2건뿐이고 둘 다 사소하다.

| # | 차이 | 처리 |
|---|---|---|
| 1 | `max_ex == 0` 일 때 레퍼런스는 `rate = 1.0` (→ AAA), rbms `dj_rank` 는 밴드 0(F) 반환 | 노트 0개 차트는 실사용 경로가 아니다. **rbms 현행 유지**하고 `reference-divergences.md` 에 1줄 기록 |
| 2 | 레퍼런스는 `1f * i / 27`, rbms 는 `2.0/9.0` 등 f32 리터럴 — 경계 정확히 위인 EX 에서 마지막 비트가 갈릴 수 있다 | pin 테스트로 고정 |

**테스트** (`crates/rbms-render/src/result.rs` 테스트 모듈, D5 소유):
- `dj_rank_boundaries_match_reference`: `max_ex = 1000*2` 에서 `exscore` 를 각 경계값 `ceil(i/27 * max_ex)` 와 그 −1 로 잡아 8밴드 전이를 전수 고정(레퍼런스 `rank[i] = rate >= i/27` 을 그대로 재현한 기대표).
- 기존 `dj_rank_bands_match_ninths`(`result.rs:321-`)는 유지.

**Phase D 작업량**: 산출식이 이미 일치하므로 **코드 변경 없음 + pin 테스트 1개 + 계획 §5 행을 "대조 완료"로 갱신**이 전부다.

---

## 7. JUDGE 설정 탭 — 노출 행과 결정 12 연결

현 설정 화면은 단일 리스트(`app_select.rs` `setting_line` / `adjust_setting`, index 0..23)다. Phase D 는 **JUDGE 서브 화면**을 추가한다(기존 `11 => ("KEY CONFIG", ">")` 와 같은 진입 방식).

| # | 행 | 값 범위 / 스텝 | 기본 | 저장 필드(`settings.rs`) | 비고 |
|---|---|---|---|---|---|
| 1 | JUDGE ALGORITHM | COMBO / DURATION / LOWEST / SCORE (순환) | COMBO | `judge_algorithm: String` | `JudgeAlgorithm::ALL` |
| 2 | JUDGE WIDTH KEY PG | 50~200 %, 스텝 5 | 100 | `judge_rate_key: [i32; 3]` [0] | |
| 3 | JUDGE WIDTH KEY GR | 50~200 %, 스텝 5 | 100 | 〃 [1] | |
| 4 | JUDGE WIDTH KEY GD | 50~200 %, 스텝 5 | 100 | 〃 [2] | |
| 5 | JUDGE WIDTH SCR PG | 50~200 %, 스텝 5 | 100 | `judge_rate_scratch: [i32; 3]` [0] | |
| 6 | JUDGE WIDTH SCR GR | 50~200 %, 스텝 5 | 100 | 〃 [1] | |
| 7 | JUDGE WIDTH SCR GD | 50~200 %, 스텝 5 | 100 | 〃 [2] | |
| 8 | LN MARGIN | 50~200 %, 스텝 5 | 100 | `longnote_margin_rate: i32` | `set_longnote_margin_rate` |
| 9 | LN MODE | LN / CN / HCN | LN | `ln_mode: String` | J25 |
| 10 | GAUGE SET | AUTO / LR2 | AUTO | `gauge_set: String` | AUTO = 모드에서 유도 |
| 11 | GAUGE AUTO SHIFT | NONE / CONTINUE / SURVIVAL TO GROOVE / BEST CLEAR / SELECT TO UNDER | NONE | `gauge_auto_shift: String` | J26 |
| 12 | BOTTOM SHIFTABLE | ASSIST EASY / EASY / NORMAL | ASSIST EASY | `bottom_shiftable_gauge: String` | 0..=2 클램프 |
| 13 | TARGET | MAX / RATE_A~AAA / RANK NEXT / LOCAL BEST / IR BEST / RIVAL | LOCAL BEST | `target: String` | 계산은 Phase F, 여기선 값 저장만 |

기존 인덱스 12 `JUDGE WIDTH` 행은 **`("JUDGE", ">")` 진입 행으로 대체**하고, 구 `judge_rate` 는 마이그레이션(§9)한다.

### 결정 12 연결 (assist / score 플래그)

`apps/rbms-player/src/main.rs` 의 `ir_submission_block_reason` / `updates_score` 와 `apps/rbms-player/src/ir_map.rs` 의 `assist_flags` 를 다음 규칙으로 확장한다(현재는 단일 `judge_rate > 100` 만 본다).

```rust
/// Reference behaviour: BMSPlayer.java:207-214, 862-882 / PlayDataAccessor.java:427-451 / MusicResult.java:81-97.
/// assist 2 = "custom judge": no IR submit, no replay save, no EX/BP/combo update; lamp and play count still update.
fn assist_level(judge_rate_key: [i32; 3], judge_rate_scratch: [i32; 3], ln_margin_rate: i32, scratch_auto: bool) -> u8 {
    let widened = judge_rate_key.iter().chain(judge_rate_scratch.iter()).any(|r| *r > 100) || ln_margin_rate > 100;
    if widened { 2 } else if scratch_auto { 1 } else { 0 }
}
```

- `assist >= 2` → `updates_score == false`, IR 제출 차단, 리플레이 저장 차단, 램프·플레이카운트는 갱신.
- `assist > 0` → 결과 램프에서 **FullCombo / Perfect / Max 를 생략**하고, 램프를 `LightAssistEasy`(gauge 0 선택 시) 또는 `AssistEasy`(그 외)로 강등한다. 강등 지점은 `crates/rbms-judge/src/gauge.rs` 의 `clear_lamp` 가 아니라 **앱 측 결과 산출**(`app_play.rs` 결과 생성부)에 둔다 — 판정 크레이트는 assist 개념을 모른다.
- 판정을 **좁히는 쪽(rate <= 100)** 은 정상 기록. 현행 `judge_rate > JUDGE_RATE_UNMODIFIED` 게이트와 동일 방향.
- autoplay / replay / practice 는 기존대로 기록·제출 제외.

### 테스트
- `assist_level_matrix`: (키/스크 6값 × LN 마진 × scratch_auto) 조합 표를 리터럴로 고정.
- `assist_two_blocks_submit_but_updates_lamp`.
- `assist_one_downgrades_lamp_and_drops_fc`.
- `narrowed_judge_still_records`.
- `settings_roundtrip_judge_rows`: RON 왕복 + 결측 필드 기본값.

---

## 8. 브랜치 분할 (파일 소유권 — 교집합 0)

### 8-0. 왜 선행 브랜치 D0 이 필요한가

`crates/rbms-judge/src/lib.rs` 는 현재 `pub mod` 3줄 + `pub use` 3줄 + `Judge` enum + **대형 `#[cfg(test)] mod tests`**(23행부터, 매처 통합 테스트가 전부 여기 있다)로 구성된다. 신규 모듈(`algorithm.rs` · `gauge_tables.rs` · `ln.rs`)은 전부 이 파일에 `pub mod` 선언이 필요하므로 D1·D2·D3 가 모두 lib.rs 를 건드리게 된다 → 병렬 창에서 충돌한다.

**해소책 2가지를 D0 에서 한 번에 처리한다.**
1. **모듈 선언·타입 선행 커밋**: `pub mod algorithm; pub mod gauge_tables; pub mod ln;` 추가 + 세 파일을 `//!` 한 줄짜리 빈 스텁으로 생성. 동시에 `rbms_model::LnKind` 에 `Undefined` 변형 추가(§6).
2. **테스트 모듈 이관**: lib.rs 의 `#[cfg(test)] mod tests { ... }` 본문을 신규 `crates/rbms-judge/src/tests.rs` 로 옮기고 lib.rs 에는 `#[cfg(test)] mod tests;` 한 줄만 남긴다. 이후 lib.rs 는 **아무도 건드리지 않는다.**
3. **신규 타입은 재export 하지 않는다.** D1/D2/D3 가 만드는 `GrooveGauge` · `GaugeSet` · `GaugeElement` · `JudgeAlgorithm` · `JudgeWindowRule` 등은 `pub use` 를 추가하지 않고 **모듈 경로로 직접 사용**한다(`rbms_judge::gauge::GrooveGauge`, `rbms_judge::algorithm::JudgeAlgorithm`). 재export 를 늘리면 다시 lib.rs 충돌이 생긴다. 재export 정리가 필요하면 §9-6 문서 브랜치에서 일괄 처리한다.

D0 은 D1/D2 분기 **이전에 단독으로 병합**한다. D0 이 만드는 4개 파일(스텁 3 + `tests.rs`)의 **내용 소유자는 아래 표의 소유 브랜치**이며, D0 은 생성만 한다.

### 8-1. 소유 파일 표

| 브랜치 | 소유 파일 (전부 명시) | 담당 항목 | 선행 |
|---|---|---|---|
| **D0 judge-decl** | `crates/rbms-judge/src/lib.rs`, `crates/rbms-model/src/lib.rs` | 모듈 선언 3개 + 스텁 파일 생성, 테스트 모듈 `tests.rs` 이관, `LnKind::Undefined` 추가 | 없음 (최초) |
| **D1 judge-gauge** | `crates/rbms-judge/src/gauge.rs`, `crates/rbms-judge/src/gauge_tables.rs` | J20 9게이지 병렬, J21 5세트 45원소, J22 MODIFY_DAMAGE(생성 시 1회 적용), J26 GAS + bottom shiftable, ClearType `LightAssistEasy` | D0 |
| **D2 judge-window** | `crates/rbms-judge/src/windows.rs`, `crates/rbms-judge/src/algorithm.rs` | J17 `JudgeAlgorithm` 4종, J23 `JudgeWindowRule`(fixjudge + fixmin/fixmax), `judge_code`/`in_good_band`/`in_ms_band` 헬퍼, `judge_property_tables_pin` | D0 (**D1 과 병렬**) |
| **D3 judge-matcher** | `crates/rbms-judge/src/matcher.rs`, `crates/rbms-judge/src/ln.rs`, `crates/rbms-judge/src/tests.rs` | 알고리즘 2단계 폴드(§1), J23 `judge_vanish`/`MissCondition` 배선(§3-B), J9/A10 BSS·MSS(§4), J24 deferral·plain LN 확정·HCN 틱(§5), 9게이지 배선 | D1 **and** D2 |
| **D4 play-integration** | `crates/rbms-play/src/lib.rs`, `crates/rbms-model/src/mode.rs`, `crates/rbms-chart/src/lib.rs` | 24K `Mode` 신설 + `for_mode` KEYBOARD 분기, GAS 프레임 훅, `set_judge_window_rates` 를 `JudgeWindowRule` 경로로, 게이지 세트 선택, J25 lnmode 를 `to_model` 에 적용 | D3 |
| **D5 app-judge-tab** | `apps/rbms-player/src/app_select.rs`, `settings.rs`, `keyconfig.rs`, `ir_map.rs`, `main.rs`, `app_play.rs`, `scores.rs`, `replay.rs`, `app_input.rs`, `crates/rbms-render/src/result.rs` | JUDGE 탭 13행, 결정 12 assist/score 플래그 + 램프 강등, 스크래치 역회전 바인딩(`app_input.rs` 디스패치 포함), 리플레이에 알고리즘·판정폭 기록(§10 R1), `rule_version` bump + 스코어 마이그레이션, DJ 랭크 pin 테스트(§6.5) | D4 **and Phase I** |

### 8-2. 소유권 검증

- D0 = {`rbms-judge/src/lib.rs`, `rbms-model/src/lib.rs`}
- D1 = {`rbms-judge/src/gauge.rs`, `rbms-judge/src/gauge_tables.rs`}
- D2 = {`rbms-judge/src/windows.rs`, `rbms-judge/src/algorithm.rs`}
- D3 = {`rbms-judge/src/matcher.rs`, `rbms-judge/src/ln.rs`, `rbms-judge/src/tests.rs`}
- D4 = {`rbms-play/src/lib.rs`, `rbms-model/src/mode.rs`, `rbms-chart/src/lib.rs`}
- D5 = {`rbms-player/src/app_select.rs`, `settings.rs`, `keyconfig.rs`, `ir_map.rs`, `main.rs`, `app_play.rs`, `scores.rs`, `replay.rs`, `app_input.rs`, `rbms-render/src/result.rs`}

**소유권 검증: 위 6개 집합은 모든 쌍에 대해 교집합이 공집합이다(D0∩D1 = D0∩D2 = … = D4∩D5 = ∅). 본 문서가 수정을 지시하는 모든 파일은 정확히 한 브랜치에만 속한다.** 특히 (a) `rbms-judge/src/lib.rs` 는 D0 단독이고 D1·D2·D3 는 절대 열지 않는다, (b) `rbms-model` 은 `lib.rs`(D0) 와 `mode.rs`(D4) 로 파일이 갈려 겹치지 않는다, (c) 유일한 병렬 창인 **D1 ∥ D2** 의 파일 집합 교집합은 ∅ 이다.

병렬 창: **D0 → (D1 ∥ D2) → D3 → D4 → D5**.
D5 는 Phase I 와 같은 파일(`main.rs`, `app_select.rs`, `app_play.rs`, `ir_map.rs`, `replay.rs`)을 만지므로 **반드시 Phase I 커밋 이후에 착수**하고, 라인 번호가 아니라 함수명(`setting_line`, `adjust_setting`, `ir_submission_block_reason`, `updates_score`, `assist_flags`)으로 앵커한다. 이 문서의 앱 측 앵커는 전부 **Phase I 이후 재확인** 대상이다.

문서 갱신(`docs/acknowledge/reference-divergences.md`, `docs/PROCESS.md`, 계획 §1/§5 표)은 **D5 완료 후 단독 브랜치**로 처리해 충돌을 피한다.

---

## 9. 순서와 게이트

0. **D0** 모듈 선언 3개 + 스텁 파일 생성 + `tests.rs` 이관 + `LnKind::Undefined`. 게이트: `cargo test -p rbms-judge` · `cargo test -p rbms-model` 무변경 통과(순수 이동/선언이므로 테스트 수·결과가 동일해야 한다).
1. **D1** 게이지 테이블 45원소 + MODIFY_DAMAGE + GrooveGauge 9개 + GAS + ClearType 확장. 게이트: `cargo test -p rbms-judge` 전 통과, `gauge_table_pins_all_45_elements` 포함.
2. **D2** JudgeWindowRule + JudgeAlgorithm 타입. 게이트: `normal_rule_matches_legacy_scaled` 로 **기존 동작 무변경** 증명.
3. **D3** matcher 통합(알고리즘 폴드 → BSS/MSS → CN deferral → HCN 틱 → lnmode). 각 소단계마다 `cargo test -p rbms-judge`. 기본 알고리즘을 `Combo` 로 바꾸는 커밋은 **마지막**에 단독으로 두고, 그 커밋에서만 기존 기대값이 바뀌는 테스트를 정리한다.
4. **D4** play 배선 + 24K 모드 + J25 lnmode(`to_model`). 게이트: `cargo test -p rbms-play` · `-p rbms-chart` · `-p rbms-model`, 리플레이 재시뮬 회귀(기존 리플레이 픽스처로 EX/BP/램프가 동일한지 — 단 알고리즘 기본값 변경분은 예외로 문서화).
5. **D5** 앱 설정 탭 + assist 정책 + 스코어 마이그레이션. 게이트: `cargo test --workspace`, `cargo clippy --workspace`, 실제 실행 1회(JUDGE 탭 진입·값 변경·저장·재기동 반영 확인).
6. 문서: `reference-divergences.md` 의 J17/J20~J26/J9/J12/J6 항목을 "해소"로 갱신(신규 발산 3건 추가: LR2 세트를 설정으로 노출, `dj_rank` 의 `max_ex == 0` 처리, KEYBOARD 세트 init 20/30 혼재를 원문 유지), 계획 §5 의 "DJ 랭크 미대조" 행을 §6.5 결과로 "대조 완료(일치)" 갱신, 계획 §1 표의 해당 행 상태 갱신, `docs/history/` 에 세션 기록.

`rule_version` 은 **D5 에서 1 증가**시킨다(램프 의미 변경 + LightAssistEasy 신설 + 기본 알고리즘 변경으로 EX/램프가 달라질 수 있으므로).

---

## 10. 리스크

| # | 리스크 | 완화 |
|---|---|---|
| R1 | 기본 알고리즘 `Duration → Combo` 변경으로 **기존 로컬 기록과 리플레이 재생 결과가 달라진다** | 알고리즘 전환을 단독 커밋으로 분리, `rule_version` 증가, **`apps/rbms-player/src/replay.rs` 의 `Replay` 에 `judge_algorithm` + 판정폭 6값 + `ln_margin_rate` 필드를 추가**해 재생 시 그 값으로 재현(결측 = 구 리플레이는 `Duration` + 100% 로 해석). 이 파일은 D5 소유(§8-1) |
| R2 | `ClearType` 에 `LightAssistEasy` 삽입 시 RON 직렬화 호환 깨짐 | 이름 기반 직렬화 유지 + 미지 값은 `NoPlay` 로 폴백, 구 기록은 `rule_version` 으로 구분 표시(결정 2 A안) |
| R3 | `MODIFY_DAMAGE` 의 `note > n \|\| note > 1` 루프를 "최적화"하면 값이 달라짐 | 레퍼런스 구조를 그대로 옮기고 샘플 5조합 값을 테스트로 고정 |
| R4 | HAZARD 게이지 클리어가 `FullCombo` 램프로 매핑되는 것이 직관과 어긋나 "버그로 오해"되어 되돌려질 수 있음 | 표에 명시 + 테스트 이름에 근거 파일:라인 기재 |
| R5 | D5 가 Phase I 와 같은 파일을 만져 충돌 | 순서 강제(Phase I 완료 후), 함수명 앵커, D5 착수 시 해당 함수 재통독 |
| R6 | 9게이지 병렬 갱신으로 판정당 연산이 6→9배 | `Gauge` 는 f32 8필드 구조체이고 판정당 9회 분기 없는 산술 → 무시 가능. 그래도 Phase B 소크 하네스에서 fps 회귀 확인 |
| R7 | 24K 모드 추가가 렌더/스캔/키설정까지 파급 | Phase D 는 **판정·게이지 경로만** 연결, UI 노출은 Phase G |
| R9 | `Mode.channel_assign` 이 `[i8; 18]` 고정폭이라 24K 가 안 들어갈 수 있다(`mode.rs:5-11`) | D4 착수 시 배열 폭 확장 여부를 먼저 결정하고, 확장하면 `lane_of_raw` 전수 테스트(`mode.rs` 테스트 모듈)를 함께 갱신 |
| R10 | `LnKind::Undefined` 추가가 하위 크레이트의 `match` 를 깨뜨린다 | D0 에서 단독 추가 후 `cargo test --workspace` 로 확인. 현 사용처는 `matcher.rs:25` · `rbms-chart/src/lib.rs:337` 둘 다 `matches!` 라 비-exhaustive 안전 |
| R8 | PMS `fixmin/fixmax` 클램프 도입으로 기존 PMS 기록의 판정이 바뀜 | R1 과 동일하게 `rule_version` 로 흡수, PMS 전용 회귀 테스트 추가 |

---

## 11. 미확인 사항

> 2026-09-09 비평 반영 시 **해소된 항목은 §11-A 로 옮겼다.** 아래는 여전히 미확인인 것만 남긴다.

1. **24K(KEYBOARD) `Mode` 의 레인 수·스크래치 유무·기본 키 배치.** `crates/rbms-model/src/mode.rs` 에 대응 상수가 없고, 레퍼런스의 24K 모드 정의 파일을 이번 세션에서 찾지 못했다(`Mode.java` 미존재 — 모드 상수가 `PlayModeConfig`/`BMSPlayerMode` 계열에 흩어져 있다). D4 착수 시 레퍼런스에서 24K 모드 정의를 직접 특정해 확정한다. 본 문서에 있던 **"key = 26 추정"은 근거 없는 추정이라 삭제**했다. 함께 확정할 것: `channel_assign` 배열 폭(현 `[i8; 18]`, §10 R9).
2. **`MusicResult.java:81-97` · `PlayDataAccessor.java:427-451` · `BMSPlayer.java:207-214, 862-882`** (결정 12 근거)는 acknowledge 문서 인용을 옮긴 것이고 직접 통독하지 않았다. **D5 착수 전 재확인 필수.**
3. **J26 GAS 의 `len`/`type` 계산(`BMSPlayer.java:638-670`), GAS 5모드 정의(`PlayerConfig.java:163-167`), `bottom_shiftable` 클램프(`PlayerConfig.java:907-908`)** 도 직접 통독하지 않았다(§2 의 의사코드는 인용 기반). **D1 착수 전 재확인 필수.** 확인된 것은 `PlayerConfig.java:925` 의 `lnmode` 클램프 `0..2` 뿐이다.
4. **LR2 게이지 세트를 사용자 설정 행으로 노출하는 것은 본 설계의 결정**이며 레퍼런스(코스 제약으로만 선택, `GrooveGauge.java:142-144`)와 발산한다. `reference-divergences.md` 기록 필요.
5. **TARGET 행의 값 집합**은 계획 §2 Phase F 문구에서 옮긴 것이다. 참고로 레퍼런스 `PlayerConfig.java:74` 의 `targetlist` 는 `RATE_A-, RATE_A, RATE_A+, RATE_AA-, RATE_AA, RATE_AA+, RATE_AAA-, RATE_AAA, RATE_AAA+, RATE_MAX-, MAX ...` 형태로 **본 문서 표(§7 13행)의 값 집합과 다르다.** 실제 계산·값 목록 확정은 Phase F 소관이며, Phase D 는 **문자열 저장만** 한다.
6. **HCN 200 ms 틱이 판정 카운트·EX 스코어에 반영되지 않는다**는 것은 `updateMicro` 를 거치지 않고 `gauge.update` 를 직접 호출한다는 사실로부터의 추론이며 반례를 확인하지 않았다.
7. **`cargo test --workspace` baseline** 을 이번 세션에서 실행하지 않았다. PROCESS 기재값(Phase A 이후 1,060 통과)을 인용했을 뿐이다. **D0 착수 시 최초로 실측해 기록**한다.
8. **`combo[judge]` 를 rbms 콤보 경로가 이미 읽는지** 확인하지 못했다(§3-B (c)). D3 착수 시 확인.

### 11-A. 해소된 항목 (2026-09-09, 코드/레퍼런스 직접 확인)

| 구 항목 | 결론 |
|---|---|
| `LnKind` 가 "차트 미지정(TYPE_UNDEFINED)" 을 표현하는가 | **표현하지 않는다.** `crates/rbms-model/src/lib.rs:31-35` 는 `Ln/Cn/Hcn` 3값뿐이고, `crates/rbms-chart/src/lib.rs:99-107` 이 `#LNMODE 0` 을 그 자리에서 `LnKind::Ln` 으로 접는다. → J25 적용 지점이 파서(`to_model`)로 올라간다. §6 에 설계 반영, D4 소유. |
| plain LN 의 레퍼런스 확정 규칙(`JudgeManager.java:563-577`)과 rbms `release` 의 동치 여부 | **비동치.** 레퍼런스는 (i) 마진 만료 시 `lnendJudge` 확정, (ii) **종단 시각 통과 시 `lnstartJudge`(헤드 판정)로 확정** 2경로다. rbms 는 (ii) 가 없다. §5 (a-2) 에 설계·테스트 추가, D3 소유. |
| MSS 전용 분기가 레퍼런스에 따로 없는가 | **없다(확인).** `sckey` 는 `JudgeManager.java` 안에서 `:435`·`:449`(점유), `:370`·`:507`·`:529`·`:624`(해제) 6곳에서만 쓰이며 MSS 전용 코드는 존재하지 않는다. 다만 그 규칙들이 스펙에 정확히 옮겨지지 않았던 부분(`judge != 4`, 점유 시점)을 §4 에서 정정했다. |
| 계획 §1 J25 의 "lnmode 0~3" 이 레퍼런스 `0..2` 클램프와 어긋나는가 | **어긋나지 않는다(스케일이 둘).** 차트 헤더 `#LNMODE` 는 0~3(0=미지정), 플레이어 설정 `PlayerConfig.lnmode` 는 0~2. 계획 문서의 "0~3" 은 헤더 쪽이므로 **정정 대상이 아니다**. §6 에 두 스케일을 구분해 명기. |
| DJ 랭크 산출식 대조 (계획 §5) | **완료 — 일치.** §6.5 신설. rbms `dj_rank`(`crates/rbms-render/src/result.rs:77-100`)의 9분위 경계가 레퍼런스(`ScoreDataProperty.java:101-102` + `BooleanPropertyFactory.java:280-288`)와 동일. 차이는 `max_ex == 0` 처리 1건뿐. |
| PMS `judge_vanish`/`MissCondition` 배선 (계획 §1 J23 "엔진 배선은 Phase D") | **미배선 확인 → 설계 추가.** `windows.rs` 의 세 필드는 같은 파일 테스트(`:372-374`) 외에 읽는 코드가 전무했다. §3-B 신설, D3 소유. |
| `windows.rs:67` FIVEKEYS `ln_scratch_end` 3번째 쌍 `(-110000, 110000)` 이 오타인가 | **오타 아님.** `JudgeProperty.java:16` 원문이 `{-130000,130000,-160000,160000,-110000,110000,-260000,260000}` 로 동일한 비단조 값이다. **수정 금지**, §3 pin 테스트로 고정. |

---

## 12. 비평 반영 (2026-09-09)

완결성 비평(verdict: 수치 이식은 정확, 설계·소유권 4건이 착수 전 수정 필요)에 대해 **인용된 코드와 레퍼런스를 전부 직접 열어 검증**한 뒤 아래를 반영했다.

### 수정 (비평이 옳았던 것)

1. **§4 BSS 중간 릴리스 규칙 — 레퍼런스 조건 누락 복구.** 원문은 `if (judge != 4 || key != sckey[sc]) { release = false; }`(`JudgeManager.java:503`)인데 스펙은 `judge != 4`(= 종단 윈도우 완전 밖일 때만 릴리스 성립)를 통째로 빠뜨렸었다. 본문·`release_dir` 설계·테스트를 정정하고, plain LN 은 이 절이 **없다**는 것(`:525`)도 함께 명시했다.
2. **§1 후보 선택 폴드 — 판정완료 노트 고정 결함 수정.** `None => chosen = Some(i)` 무조건 첫-후보 채택은, `judged`/`holding` 노트까지 순회하는 rbms 후보 루프(`matcher.rs:335-354`)에서 판정완료 노트를 `t1` 로 고정시켜 `Lowest` 가 그것을 재판정하게 만든다. 레퍼런스의 **2단계 구조**(stage 1 `tnote == null || tnote.getState() != 0 || compare(...)` = `:396`, stage 2 judge code 4/5/6 분기 = `:407-413`)를 그대로 옮긴 의사코드로 교체했다.
3. **§3-B 신설 — J23 "엔진 배선" 누락 복구.** `judge_vanish` · `MissCondition::One` · `combo` 는 `windows.rs` 에 데이터만 있고 읽는 코드가 없었다(테스트 제외). 레퍼런스 사용처 3곳(`:428/444/452/458` 소비 게이팅, `:397-399` 후보 필터, `:648` 이중 카운트 억제)을 표·설계·테스트로 명세했다.
4. **§3 `judge_property_tables_pin` 추가 — 계획 §2 "전 배열 대조" 이행.** `JudgeProperty` 4행 × 4테이블 + margin/combo/judge_vanish/miss_condition 을 pin 한다. FIVEKEYS `ln_scratch_end` 의 비단조 값은 원문 확인 결과 **오타가 아님**을 명시하고 수정 금지로 못박았다.
5. **§6.5 신설 — DJ 랭크 산출식 대조.** 계획 §5 가 요구한 항목이 통째로 빠져 있었다. 양쪽을 통독해 **경계 일치**를 확인하고 pin 테스트 + 차이 2건을 표로 남겼다.
6. **§8 전면 재작성 — 파일 소유권 충돌 해소.** `crates/rbms-judge/src/lib.rs` 가 D1·D2·D3 모두에게 필요했던 문제를 **선행 브랜치 D0**(모듈 선언 + 테스트 모듈 `tests.rs` 이관 + 신규 타입 재export 금지)으로 끊었다. `replay.rs`·`app_input.rs` 를 D5 에 편입하고, `crates/rbms-render/src/result.rs`(§6.5), `crates/rbms-chart/src/lib.rs`(§6 J25) 도 소유자를 배정했다. **§8-2 에 6개 집합을 명시하고 쌍별 교집합이 ∅ 임을 기재**했다.
7. **§6 순서 모순 해소.** KEYBOARD 라우팅 분기를 D2 → **D4** 로 옮겼다(D2 시점에는 대응 `Mode` 상수가 없어 사문 분기가 된다). 아울러 `Mode` 의 실제 파일이 `crates/rbms-model/src/lib.rs` 가 아니라 **`crates/rbms-model/src/mode.rs`** 임을 정정하고 현 5개 모드 상수와 `channel_assign: [i8; 18]` 제약(§10 R9)을 기재했다.
8. **§2 `modify_damage` 적용 시점 명시** — 판정마다가 아니라 **게이지 생성 시 1회**(`GrooveGauge.java:260-291`).
9. **§4 `sc_dir` 점유 시점 명시** — 헤드 판정이 노트를 소비했을 때(`judgeVanish` true) `sckey[sc] = key`(`:435`/`:449`), BSS 종단 시 `releasetime`·`lnendJudge`·`sckey` 3개를 함께 리셋(`:368-370`).
10. **§5 plain LN 확정 규칙 추가** — 레퍼런스 `:563-577` 을 통독해 "종단 시각 통과 시 헤드 판정으로 확정" 경로가 rbms 에 없음을 확인하고 설계·테스트를 추가했다.
11. **앵커 드리프트 정정** — `matcher.rs:325-352` → `335-354`(후보 루프), `342-346` → `343-347`(미판정 최소 |dm|) / `348-351`(재타격 수집) / `392-397`(`empty_poor_hit`), `matcher.rs:399-424` → `399`(`release`, 본문 399-420; `update` 는 425-), `rbms-play/src/lib.rs:132-141` → `133-140`, `windows.rs:188-208` → `188`.
12. **§10 R1 구체화** — 리플레이 재현 필드를 `apps/rbms-player/src/replay.rs` 의 `Replay` 확장으로 명시(소유자 D5). R9·R10 신설.
13. **§11 정리** — 해소된 6건을 §11-A 표로 이동, 미확인은 8건으로 재정렬. GAS(`BMSPlayer.java:638-670` 등) 미통독을 **새 미확인 항목 3번**으로 추가했다.

### 반려 (비평이 틀렸던 것)

1. **"rbms 에 `dj_rank`/`DJ_RANK`/`dj_level` 류 심볼 자체가 없다"** — 사실이 아니다. `crates/rbms-render/src/result.rs:77-100` 에 `RANK_BANDS`(8밴드) · `RANK_BOUNDS`(9분위) · `pub fn dj_rank(ex, max_ex)` 가 있고 `crates/rbms-render/src/lib.rs:14` 에서 재export 되어 `select.rs:452, 466` 과 `result.rs:149` 가 사용한다. 기존 테스트 `dj_rank_bands_match_ninths`(`result.rs:321`)도 있다. 다만 **"DJ 랭크 대조가 스펙에 빠졌다"는 지적 자체는 옳아** §6.5 를 신설했다(대조 결과: 일치).
2. **"계획 §1 J25 의 lnmode 0~3 은 레퍼런스 클램프 0..2 와 어긋나 계획 문서 정정 대상"** — 반려. 두 값은 **다른 스케일**이다. BMS 차트 헤더 `#LNMODE` 는 `0=미지정/1=LN/2=CN/3=HCN` 이고(rbms 도 `crates/rbms-chart/src/lib.rs:99-107` 에서 이 스케일로 읽는다), `PlayerConfig.lnmode` 만 `0..2`(`PlayerConfig.java:925`)다. 계획 문서의 "0~3" 은 헤더 쪽이므로 **정정하지 않는다.** 대신 §6 에 두 스케일 구분을 추가했다.
3. **"`windows.rs:67` FIVEKEYS `ln_scratch_end` 3번째 쌍이 비단조라 오타인지 확인이 필요하다"** — 확인 결과 **레퍼런스 원문과 문자 단위로 동일**하다(`JudgeProperty.java:16`). 오타가 아니라 원본의 성질이므로 스펙의 수치는 그대로 두고, pin 테스트에 "수정 금지" 주석만 덧붙였다.
4. **"`matcher.rs:399-424` 가 `release` 라는 표기는 틀렸고 421-424 는 `update` 의 doc 주석"** — 실질적으로 옳아 앵커는 정정했으나, 비평이 함께 주장한 **"§5 의 `matcher.rs:425-489`(update) 는 425 시작이 정확"** 만 채택하고 범위 끝값은 검증하지 않았으므로 시작 라인 앵커(`matcher.rs:425`)로 통일했다. (라인 범위 끝은 Phase I 이후 드리프트 가능성이 있어 시작 라인 + 함수명 앵커가 더 안전하다.)
