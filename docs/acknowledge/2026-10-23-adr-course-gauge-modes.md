# ADR: 코스 게이지 설정 COURSE GAUGE (LR2 기본 · BEATORAJA · IIDX)

> 최종 갱신 2026-10-23 · 대응 커밋 5a0eacf
> 상태: 유효(결정 21 은 사용자 확인 2026-10-23. IIDX 수치의 세부와 모드별 대응은 메인 판단이며 사용자 미확인) · 결정일 2026-10-23 · 결정자 사용자(결정 21), 세부 메인(사용자 "나머지도 추천대로") · 커밋 9e39262(IIDX 게이지 표·`GutsEdge`·`CourseGaugeMode`. 웨이브 1 전체가 섞여 들어간 커밋이다: [feedback](../feedback/2026-10-23-stage-check-before-commit.md)), 9f8dfbc(설정 행), c4fcdfa(코스 진행 통합), 47e8f70(코스 시작 때의 규칙 고정)
> 관계: 시작 슬롯·이월·LN 제약은 [코스 시작 슬롯과 LN 제약](2026-10-23-adr-course-stage-start-and-ln.md), 제출 본문의 게이지 필드는 [코스 제출 게이지](2026-10-23-adr-course-submission-gauge.md).

## 배경

- 사용자 요구 8(2026-10-23)의 게이지 부분: 코스 게이지를 iidx·비토라자·lr2 모드로 정확히 구현하고, lr2 를 기준으로 하되 설정에서 바꿀 수 있어야 한다.
- 3c0cd22 시점의 코스: 표는 코스 제약(`gauge_lr2`·`gauge_5k`·`gauge_7k`·`gauge_9k`·`gauge_24k`)이 있으면 그 표, 없으면 설정 GAUGE SET(AUTO·LR2), 그것도 AUTO 면 차트 모드의 표였다. IIDX 표는 없었다. 원본 beatoraja 도 제약이 표를 정한다(`GrooveGauge.java:128-147`).
- 조사 결과: 발광(GENOSIDE)·Stella·Satellite 코스는 모두 제약이 `grade_mirror, gauge_lr2, ln` 이라 beatoraja 에서도 LR2 단위 게이지로 돈다. 사용자의 "비토라자와 LR2 코스 게이지가 비슷할 것" 은 수치로는 같지 않다(LR2 DAN 은 PG +0.10, BAD -2 / 7키 CLASS 는 PG +0.15, BAD -1.5). 근거 요약은 [조사 보고](../reference/2026-10-23-stella-course-dan-survey.md) 의 C 절.

## 결정

### 1) 설정

- 설정 `judge.course_gauge`: `LR2`(기본) / `BEATORAJA` / `IIDX`. JUDGE 탭 GAUGE 구역의 GAUGE SET 바로 뒤 행 COURSE GAUGE(`catalog.rs:46`). 저장 토큰은 행 값과 같은 대문자, 없거나 모르는 값은 LR2, 설정 스키마 세대는 6 그대로.
- 계정 설정 묶음 계약 `contract/client-requests/settings-blob.json` 에 `course_gauge` 한 줄이 생겼다.

### 2) 코스는 이 설정의 표로만 돈다

| COURSE GAUGE | 코스 스테이지가 쓰는 게이지 표 |
| --- | --- |
| LR2 | LR2 세트(차트 모드와 무관) |
| BEATORAJA | 차트 모드의 표: 5키·10키 FIVEKEYS, 9키 PMS, 24키 KEYBOARD, 그 밖 SEVENKEYS(`BMSPlayerRule.java:13-17`) |
| IIDX | IIDX 세트(차트 모드와 무관, 5·9·24키에도 같은 수치) |

- 코스가 선언한 게이지 제약은 막대 배지에만 남고 표를 정하지 않는다. 설정이 제약보다 우선한다. 제약이 없는 사용자 코스 파일도 기본은 LR2 표다.
- 설정 GAUGE SET(AUTO·LR2)은 단일 곡 전용이 되었다. 코스 중에는 COURSE GAUGE 만 표를 정한다.
- 코스 결과 화면의 행 GAUGE 바로 뒤에 `GAUGE RULE` 행을 보인다(`course_ui.rs` `result_rows`).

### 3) 코스 시작 때의 규칙으로 모든 스테이지를 판정한다

- 코스를 시작할 때 `config.judge.course_gauge` 를 한 번 읽어 `CourseFacts.gauge_rule` 에 담고(`lib.rs` `start_course`), 이후 스테이지는 설정이 아니라 이 값으로 표를 고른다(`app_play.rs` `stage_judge_setup`). 코스 도중 설정을 바꿔도 이어지는 스테이지는 시작 때의 규칙이다.
- 이유: 코스 제출이 `course_gauge` 한 값으로 규칙을 말하므로(제출 ADR) 모든 스테이지가 그 규칙으로 판정돼야 본문과 판정이 일치한다.
- 구현과 다름: 설계서 `docs/plan/2026-10-23-course-gauge-data.md` 3.3절 (2)는 스테이지마다 설정값을 읽는다고 적었다. 47e8f70 에서 메인이 시작 때의 값으로 고쳤다.

### 4) IIDX 게이지 표

- 출처는 iidx.org 한 곳이다(gauges_and_timing 의 표, beginner 의 gauge, misc 의 IIDX·LR2·beatoraja 비교). 다른 공식 자료는 열리지 않아 확인하지 못했다.

| 슬롯 | 값 |
| --- | --- |
| Class (DAN) | 시작 100, 0% 에서 실패(클리어는 0 초과). PG +0.16, GR +0.16, GD +0.04, BD -1.5, POOR -2.5, 빈 POOR -1.5. 게이지가 30% 이하이면 BAD·POOR·빈 POOR 감소량을 절반(-0.75, -1.25, -0.75). TOTAL 보정 없음 |
| ExClass (EX-DAN) | 같은 증가량, BD -3.0, POOR -5.0, 빈 POOR -3.0. 30% 보정 없음 |
| ExHardClass | EX-DAN 값의 복사. IIDX 에는 셋째 단위 게이지가 없고 코스 상한이 ExClass 라 코스에서는 닿지 않는다 |
| AssistEasy ~ Hazard | 7키 세트의 복사. 코스에서 선택되지 않는 자리이고 9슬롯 구조를 유지하려고 채웠다 |

- 실패선은 사용자가 0% 로 정했다. iidx.org 의 gauge compendium 은 0%, beginner 는 "2% 미만" 으로 서로 다르며 사용자 결정으로 compendium 쪽을 따른다.
- "30% 이하" 경계: iidx.org 가 "at or below 30%" 로 적었다. 원본의 완충 비교는 `value < gut[0]`(엄격 미만, `GrooveGauge.java:233`)이라 새 열거형 `GutsEdge { Below, AtOrBelow }` 를 `GaugeParams` 와 `Gauge` 에 두었다. 기존 모든 행은 `Below`(생략 가능, 데이터 세대 1 유지, 옛 사용자 덮어쓰기 파일도 `Below` 로 읽힘)이고 IIDX Class 행만 `AtOrBelow` 다. 기존 판정은 수치·화소 모두 그대로다(54칸 고정 시험).
- 사용자 GAUGE 설정 → 코스 게이지: HARD 이상은 IIDX 에서 EX-DAN(ExClass) 까지만 간다(`GaugeSetId::course_ceiling`). `BEST CLEAR`·`SELECT TO UNDER` 자동 이동도 이 상한을 넘지 않는다.
- 연습 패널의 게이지 표 순환에 IIDX 가 들어 6세트가 되었다(`practice.rs`).

## 이유

- 사용자가 모드 선택을 설정으로 요구했고 "lr2 기준" 을 말했다. 발광·Stella 코스가 모두 `gauge_lr2` 이고 R-BMS 에 LR2 세트와 완충이 이미 있어 LR2 를 기본으로 두면 beatoraja 에서 돌던 결과와 같다.
- 제약을 배지로만 두어야 BEATORAJA·IIDX 모드가 의미가 있다. 제약이 표를 정하면 발광·Stella 코스는 모드와 무관하게 늘 LR2 다.
- `GutsEdge` 는 원본의 엄격 미만 비교를 고치지 않고 IIDX 의 "이하" 만 표현하려는 최소 변경이다.

## 기각된 대안

| 대안 | 기각 이유 |
| --- | --- |
| 원본처럼 코스 제약이 표를 정함 | 사용자 결정 21 이 설정 우선을 정했다 |
| 기본값을 BEATORAJA | 사용자가 lr2 기준을 말했고 기존 코스 제약과 맞다 |
| IIDX 실패선 2% (beginner 쪽) | 사용자가 0% 로 정했다 |
| IIDX 경계를 엄격 미만으로 통일 | iidx.org 문구가 "이하" 다. 정확히 30.0 일 때만 갈린다 |
| 모든 행의 경계를 "이하" 로 통일 | 기존 LR2·beatoraja 판정이 바뀐다 |
| IIDX 에서 EXHARD 이상을 별도 셋째 게이지로 | IIDX 에 없다(설계서 F5 결정 1 의 추천 채택) |
| 단일 곡 GAUGE SET 행에도 BEATORAJA·IIDX 를 더함 | 단일 곡 행은 `GAUGE_SET_CYCLE` = [AUTO, LR2] 그대로이고 `crates/rbms-config/src/tests.rs` 가 고정한다 |

## 영향과 관련 파일

- 판정 의미가 바뀌는 곳: BEATORAJA 에서 발광·Stella 코스가 LR2 가 아니라 7키 CLASS 로 돈다. 제약 없는 사용자 코스도 LR2 표다(지금 R-BMS 에서도 바뀜). IIDX 의 내부 단위·반올림은 알 수 없어 실제 IIDX 와 다를 수 있다.
- 사용자 미확인(메인 판단): "30% 이하" 경계와 빈 POOR 절반, EXHARD 이상을 EX-DAN 으로 대응, 5·9·24키에도 같은 IIDX 수치, 연습 패널에 IIDX 노출, 결과 화면 GAUGE RULE 행, 막대 배지는 선언된 제약 그대로.
- 확인하지 못한 사실: LR2 본체의 시작값·실패선(iidx.org 는 LR2oraja 가 32% 시작·2% 실패라고 적었으나 다른 출처로 확인되지 않음), IIDX 의 내부 반올림.
- 코드: `crates/rbms-judge/src/{gauge_tables,gauge,data}.rs`, `crates/rbms-judge/data/gauge.ron`("IIDX" 키), `crates/rbms-config/src/{judge,schema,settings}.rs`, `apps/rbms-player/src/{course_ui,app_play,lib,practice}.rs`, `apps/rbms-player/src/stage/{settings/catalog,course_result}.rs`.
- 시험: `gauge_tables::tests::{the_iidx_dan_gauges_pin_their_published_values, only_the_iidx_dan_gauge_softens_at_or_below_its_band, the_iidx_set_tops_out_at_exclass, every_course_gauge_of_every_set_starts_full}`, `gauge::tests::an_iidx_dan_gauge_halves_damage_at_thirty_exactly`, `course_ui::tests::the_course_gauge_setting_outranks_a_gauge_constraint`, `the_gauge_constraints_are_shown_but_do_not_pick_the_table`, `settings::tests::the_course_gauge_row_cycles_lr2_beatoraja_iidx`.

## 날짜와 사용자 원문

2026-10-23.

- 요구 8 의 게이지 부분: "게이지도 정확하게 구현하고 (iidx모드, 비토라자모드, lr2모드 등이고 아마 비토라자랑 lr2의 코스 게이지가 비슷할텐데 lr2를 기준으로 게이지를 넣으면될거야, 이또한 설정에서 변경가능하게 만들어놔야하고.)"
- 결정 21: "좋아 너가말하나 3개 다 iidx까지 구현 하고 나머지도 추천대로 (0%)" (`docs/PROCESS.md` 에 기록된 원문. 질문 원문은 저장소에 없다.)
