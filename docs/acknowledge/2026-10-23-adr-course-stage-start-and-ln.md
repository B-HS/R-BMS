# ADR: 코스 스테이지의 시작 게이지 슬롯, 9슬롯 이월, LN 제약 적용

> 최종 갱신 2026-10-23 · 대응 커밋 5a0eacf
> 상태: 유효(코드 반영). **사용자 미확인**: 시작 슬롯(J-4), LN 제약 플레이 적용(J-7), 9슬롯 이월(J-6). 사용자는 결정 21 에서 "나머지도 추천대로" 만 말했고 이 세 가지의 내용을 직접 승인하지 않았다. 세 가지 모두 지금 R-BMS 의 동작을 바꾼다 · 결정일 2026-10-23 · 결정자 메인(조사 보고의 추천안 채택) · 커밋 c4fcdfa(코스 진행 통합: 시작 슬롯 적용, 9슬롯 이월, LN 제약), 9e39262(`GaugeSetId::course_gauge_for`·`course_ceiling`. 웨이브 1 전체가 섞인 커밋), 47e8f70(코스 시작 때의 규칙 고정)
> 관계: 표 선택과 IIDX 표는 [코스 게이지 설정](2026-10-23-adr-course-gauge-modes.md). 2026-10-22 [어시스트 등급 ADR](2026-10-22-adr-assist-lamp-grade.md) 5절의 "코스 스테이지는 언제나 CLASS 계열 게이지" 서술이 이 결정으로 좁아진다(저장·전송이 실제 끝난 게이지를 따르는 규칙은 그대로).

## 배경

- 3c0cd22 시점의 코드: 코스 스테이지는 사용자 GAUGE 설정과 무관하게 항상 `Class` 슬롯으로 시작했다(`app_play.rs:578` `GaugeIndex::Class`). 이월은 `CourseRun.carry_gauge` 한 값(선택 게이지의 끝 값)이었고, 코스의 `ln`·`cn`·`hcn` 제약은 `CourseOverrides::ln_mode_for`(`#[cfg(test)]`)에만 있어 판정 설정에 들어가지 않았다.
- 원본 beatoraja: 시작 슬롯은 사용자 게이지로 정하고(`GrooveGauge.java:164` `id = type <= 2 ? 6 : (type == 3 ? 7 : 8)`), 새 곡은 직전 곡 게이지 기록의 마지막 값으로 9슬롯 모두를 세팅하며(`GrooveGauge.java:150-155`), LN 제약 코스는 `config.setLnmode(0)` 으로 LN 을 강제한다(`MusicSelector.java:501`).
- 조사 보고 C 절의 추천은 "beatoraja 식(사용자 게이지 → 코스 슬롯), R-BMS 가 이미 이 규칙을 가짐" 이었다. 이 전제는 틀렸다. 대응식은 GAS `SelectToUnder` 의 상한 계산에만 있었다(설계서 `docs/plan/2026-10-23-course-gauge-data.md` 2.9절 정정). 결정 21 의 "나머지도 추천대로" 가 이 추천을 채택하면서 새로 만든 규칙이다.
- 결정 20 의 "LN 제약 준수" 는 단위 인정 조건(서버 판정)으로 기록돼 있고, 플레이에서 LN 제약을 강제하라는 말까지 담았는지는 질문 원문이 저장소에 없어 확인하지 못했다(교차 검토서 J-7).

## 결정

### 1) 시작 슬롯은 사용자 GAUGE 설정을 따른다 (J-4)

- ASSIST EASY·EASY·NORMAL → CLASS(DAN), HARD → EXCLASS, EXHARD·HAZARD → EXHARDCLASS. 고른 표의 `course_ceiling` 으로 자른다(IIDX 표는 EXCLASS 까지). `GaugeSetId::course_gauge_for`(`gauge_tables.rs`), 호출은 `course_ui::course_stage_gauge`.
- 코스를 시작할 때의 첫 값은 그 슬롯의 `init`(모든 표의 코스 슬롯이 100)을 읽는다(`lib.rs` `start_course`, 첫 차트 모드를 모르므로 7키 표로 읽는다).

### 2) 곡 사이에 9슬롯 모두를 이월한다 (J-6)

- 스테이지가 끝나면 아홉 게이지의 판 끝 값(`PlaySession::gauge_values`)을 `CourseRun.carry_gauges` 에 담는다(`app_result.rs` `fold_course_stage`, `rbms_course::run::GAUGE_SLOTS = 9` 와 `GaugeIndex::COUNT` 의 일치는 컴파일 시점 단언). 다음 스테이지는 `SessionOptions.carried_gauges` 로 아홉 슬롯 전부를 그 값으로 시작하고(`Player::carry_gauges`, 분석 seek 뒤에도 유지), 시작 값은 `carried[시작 슬롯]` 이다. 0 이 된 슬롯은 0 으로 남는다.
- 원본은 게이지 기록 표본의 마지막 값을 쓰고 여기서는 판 끝의 정확한 값을 쓴다(표본 간격 안의 차이. 기존 `carry_gauge` 가 `summary.gauge_value` 를 쓰는 것과 같은 선택).
- `carry_gauge`(한 값)는 코스 결과의 GAUGE 행과 제출 `gauge_value` 용으로 남는다.

### 3) 코스의 ln·cn·hcn 제약을 플레이에 적용한다 (J-7)

- 제약 `ln`·`cn`·`hcn` 이 있으면 그 코스의 모든 스테이지가 LN MODE 를 LN·CN·HCN 으로 강제해 판정한다(`CourseOverrides::ln_mode_for`, `#[cfg(test)]` 를 지움). 적용 자리는 `app_play.rs` `stage_judge_setup` 이고, 스테이지의 LN 종류와 기록 키(`run_lntype`·`ln_mode_key`)를 읽기 전이다.
- 설정 파일의 LN MODE 는 바꾸지 않는다. 원본은 `config.setLnmode` 로 설정 자체를 바꾸지만 R-BMS 는 RANDOM·속도 제약과 같이 복사본에만 적용한다(코스가 끝나면 플레이어의 값 그대로).
- 코스 제출 최상위 `lntype`: LN 제약이 있으면 그 값(0 LN, 1 CN, 2 HCN), 없으면 첫 스테이지가 실제로 친 값.

## 바뀌는 점 (지금 R-BMS 대비)

| 대상 | 전 | 후 |
| --- | --- | --- |
| HARD 설정의 사용자 | 코스 스테이지가 CLASS 로 달림 | EXCLASS 로 달리고 HARD 램프. SCRATCH AUTO 판이면 어시스트 등급 3 (서버는 `ExClass` → 3) |
| EXHARD·HAZARD 설정의 사용자 | CLASS | EXHARDCLASS (IIDX 모드는 EXCLASS) |
| GAS(BEST CLEAR·SELECT TO UNDER)를 켠 코스 | 다음 스테이지에 선택 게이지의 끝 값 하나만 CLASS 로 이월 | 아홉 게이지가 각자의 끝 값에서 시작 |
| CN·HCN 설정의 사용자가 발광·Stella·GENOSIDE 코스(모두 `ln` 제약)를 침 | 설정의 CN·HCN 으로 침 | LN 으로 침 |

## 이유

- 원본(beatoraja)과 같아진다. 사용자가 "게이지를 정확하게" 구현하라고 했고 단위 인정(결정 20)이 LN·미러 제약 준수를 요구한다. 플레이가 LN 제약을 강제하지 않으면 CN·HCN 설정 사용자는 같은 코스에서 서버가 인정하지 않는 `lntype` 으로 판을 내게 된다.
- 시작 슬롯이 설정을 따르지 않으면 HARD 로 치는 사람의 코스 판이 늘 NORMAL 램프라 사용자 게이지 설정이 코스에서 의미가 없다.

## 기각된 대안

| 대안 | 기각 이유 |
| --- | --- |
| 시작 슬롯을 항상 CLASS 로 유지 | 원본과 다르고 GAUGE 설정이 코스에서 무의미하다 |
| 이월을 선택 게이지 한 값으로 유지 | 원본은 아홉 슬롯 모두. GAS 가 슬롯을 옮기는 코스에서 시작 값이 달라진다 |
| 이월을 게이지 기록 표본의 마지막 값으로 | 판 끝의 정확한 값이 이미 있다 |
| LN 제약을 서버의 단위 인정에서만 확인하고 플레이에는 적용하지 않음 | CN·HCN 설정 사용자가 제약을 어긴 판만 내게 되어 인정 조건이 막힌다(메인 판단) |
| 원본처럼 설정 파일의 LN MODE 자체를 바꿈 | 코스가 끝난 뒤 설정이 남는다. 다른 제약과 같이 복사본에 적용한다 |

## 영향과 관련 파일

- 사용자 확인이 필요한 질문(최종 보고에 "지금 R-BMS 대비 바뀜" 으로 알린다): HARD 이상 설정이 코스에서 EXCLASS·EXHARDCLASS 로 달려도 되는가(J-4), CN·HCN 설정이 코스에서 LN 으로 강제돼도 되는가(J-7).
- 남은 위험: 차트 자체의 `#LNMODE` 가 LN 종류를 고정한 곡이 LN 제약 코스에 있으면 `lntype` 이 0 이 아니어서 서버가 단위를 인정하지 않을 수 있다(설계서 web 9절 R3). GENOSIDE·Stella 곡의 `#LNMODE` 사용 여부는 확인하지 못했다.
- 범위 밖으로 둔 것: NO GOOD·NO GREAT(대상 코스 0개, `CourseOverrides.unapplied` 에 배지만), DP MIRROR 코스의 FLIP(R-BMS 가 `LaneOption` 을 플레이에 적용하지 않는다). 둘 다 테스트 부채다.
- 코드: `apps/rbms-player/src/{course_ui,app_play,app_result,lib}.rs`, `crates/rbms-judge/src/gauge_tables.rs`, `crates/rbms-play/src/{lib,session}.rs`, `crates/rbms-course/src/run.rs`.
- 시험: `course_ui::tests::{a_hard_player_starts_a_course_on_exclass, an_exhard_player_starts_an_iidx_course_on_exclass, a_later_stage_starts_on_every_gauge_the_stage_before_left, the_long_note_constraints_each_force_their_flavour}`, `record_tests::course_gauge::{an_ln_course_plays_long_notes_whatever_the_ln_mode_setting, the_next_stage_starts_on_every_gauge_the_last_one_left, a_hard_player_records_course_stages_on_exclass}`, `rbms_play::session::tests::{a_course_stage_starts_with_every_carried_gauge, a_seek_keeps_the_carried_gauges}`.

## 날짜와 사용자 원문

2026-10-23.

- 결정 21: "좋아 너가말하나 3개 다 iidx까지 구현 하고 나머지도 추천대로 (0%)" (`docs/PROCESS.md` 에 기록된 원문). 시작 슬롯·9슬롯 이월·LN 강제는 이 말의 "나머지" 에 포함시켜 메인이 조사 추천안을 채택한 것이다. 사용자가 이 세 항목의 내용을 직접 확인한 기록은 없다.
- 결정 20: "추천대로" (단위 인정 조건 "게이지가 남은 채 끝낸 판, 미러·LN 제약 준수, 어시스트·SCRATCH AUTO 제외" 의 요약이 `docs/PROCESS.md` 에 있다).
