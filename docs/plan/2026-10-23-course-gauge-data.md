# 2026-10-23 설계: 코스 게이지 모드(LR2·BEATORAJA·IIDX)와 코스 원천(GENOSIDE 2018·표 코스)

> 최종 갱신 2026-10-23 · 대응 커밋 1169efb(dev) · 상태: 설계(구현 전). 묶음 F5(클라이언트 코스 쪽).
> 근거: [PROCESS.md](../PROCESS.md) "Stella·코스·단위와 클라이언트 보완 (2026-10-23)" 의 사용자 결정 1~21, [조사 보고](../reference/2026-10-23-stella-course-dan-survey.md), [HANDOFF.md](../HANDOFF.md) 5절. 이 문서의 파일:줄은 모두 1169efb 의 실제 코드를 다시 열어 확인한 값이다. 조사 보고와 다른 곳은 2.9절에 정정으로 모았다.
> 함께 읽을 설계: 선곡 폴더·다운로드 `docs/plan/2026-10-23-select-tree-download.md`(F3·F4, 이 문서의 3.6절 함수를 부른다), 웹 묶음 F6(이 문서 부록 A 의 와이어 계약을 받는다), 작은 묶음 F2(결정 8·9, 같은 파일을 만진다).

---

## 1. 범위와 결정 번호 대응

| 결정 | 원문 요지 | 이 문서에서 다루는 것 | 다루지 않는 것(다른 묶음) |
| --- | --- | --- | --- |
| 21 | "3개 다 iidx까지 구현 하고 나머지도 추천대로 (0%)" — 코스 게이지 설정 LR2(기본)/BEATORAJA/IIDX, 설정이 코스 제약 `gauge_lr2` 등보다 우선, IIDX 실패선 0% | IIDX 게이지 표 신설, 설정 행, 게이지 표 선택, 사용자 게이지 → 코스 게이지 대응(조사 당시 추천안 "beatoraja 식"), 곡 사이 이월, 모드별 적용 | — |
| 6 | 코스 제출에 게이지 종류를 더해 `course_best.assist_grade` 를 채움 | 클라이언트 제출 본문의 `gauge`·`course_gauge`·`stages`, 구 서버 호환, 계약 골든, 재전송 경로 | 서버 DTO·`course_best.assist_grade` 파생(F6) |
| 20 | 단위 인정 = 게이지가 남은 채 끝낸 판, 미러·LN 제약 준수, 어시스트·SCRATCH AUTO 제외 | 판정에 필요한 사실(스테이지별 RANDOM·LN 종류·게이지)을 본문에 싣기, `ln` 제약을 실제로 적용(지금 미적용, 2.5절) | 인정 판정 자체(F6) |
| 18 | GENOSIDE 2018 SP 22 + DP 22 + Stella·Satellite 표 코스 | (a) 공식 코스 파일에서 뽑은 데이터 파일과 생성기(`--check`), 출처 필드, (b) 표 header `course` → `rbms_course::Course` 변환 함수(이름·시그니처 확정), 코스 id 규칙, `.lr2crs` `<type>2`, DP(14키) 지원 확인, 클라이언트 표시 위치 | 선곡 트리의 표 코스 폴더 그리기(F3), 서버 코스 등록(F6) |
| 19 | 서버가 코스 기록으로 단위 판정 | 클라이언트가 보낼 사실만(부록 A) | 판정·`user_dan`(F6) |
| 12, 11 | Stella st·sl 둘 다, 표 코스 폴더 | 변환 함수가 st·sl header 의 `course` 를 읽음(3.6절) | 폴더 구조(F3) |

이 문서의 "원본" 은 beatoraja(`/Users/hyunseokbyun/development/beatoraja`) 이고, 원본이 침묵하는 곳은 LR2·IIDX(iidx.org 원문 사본) 순으로 근거를 찾았다.

---

## 2. 현재 코드 사실(1169efb, 다시 확인)

### 2.1 게이지 표와 슬롯

1. `GaugeIndex` 9슬롯: `AssistEasy..Hazard`(0~5), `Class`=6, `ExClass`=7, `ExHardClass`=8 (`crates/rbms-judge/src/gauge.rs:23-33`, `index()` 55-67, `is_course()` 98-100: `self >= Class`).
2. 램프: `Class`→NORMAL, `ExClass`→HARD, `ExHardClass`→EX-HARD (`gauge.rs:146-155 lamp_for_gauge`). 등급: 같은 표로 Class=2, ExClass=3, ExHardClass=4 (`gauge.rs:229-243 assist_grade`, 서버 `web/src/server/service/domain/score/assist-grade.ts:11-21 ASSIST_GRADE_OF_GAUGE`).
3. 게이지 표 5세트: `GaugeSetId { FiveKeys, SevenKeys(기본), Pms, Keyboard, Lr2 }`, `COUNT = 5` (`crates/rbms-judge/src/gauge_tables.rs:26-33, 49`). `GAUGE_TABLE: [[GaugeElement; 9]; 5]` (130-185). 7키 CLASS `[0.15, 0.12, 0.06, -1.5, -3.0, -1.5]` + `GUTS_CLASS` (149), EXCLASS `[0.15,0.12,0.03,-3,-6,-3]` (150), EXHARDCLASS `[0.15,0.06,0,-5,-10,-5]` (151). LR2 CLASS `[0.1,0.1,0.05,-2,-3,-2]` + `GUTS_LR2 = [(30.0, 0.6)]` (182, 16), EXCLASS `[0.1,0.1,0.05,-6,-10,-2]` + `GUTS_LR2` (183), EXHARDCLASS `[0.1,0.1,0.05,-12,-20,-2]` 완충 없음 (184). beatoraja `GaugeProperty.java:93-95, 123-125` 와 한 칸씩 대조해 같았다(LR2 = `CLASS_LR2`/`EXCLASS_LR2`/`EXHARDCLASS_LR2`).
4. 번들 데이터 `crates/rbms-judge/data/gauge.ron` 의 키는 `BEAT_10K, BEAT_14K, BEAT_5K, BEAT_7K, KEYBOARD_24K, LR2, POPN_9K` (테스트 `bundled_gauge_data_has_exactly_the_expected_keys`, `gauge_tables.rs` 테스트 모듈). 컴파일 표와의 일치는 `bundled_gauge_data_matches_the_compiled_in_table` 이 지킨다. 행 형식은 `GaugeParams { modifier, min, max, init, border, deltas, guts }` (`crates/rbms-judge/src/data.rs:89-97`).
5. 완충(guts) 비교는 **엄격 미만**이다: `if self.value < g.0` (`gauge.rs:340`), 원본 `GrooveGauge.java:233` `if(value < gut[0])` 와 같다. "이하" 비교는 지금 표현할 수 없다.
6. 모든 세트의 CLASS 계열 3행은 `init 100, min 0, max 100, border 0`, modifier `None` (TOTAL 보정 없음).

### 2.2 코스가 게이지를 고르는 길

1. 코스 스테이지는 **언제나 `Class` 슬롯으로 시작**한다: `initial_gauge: course_state.map(|(gauge_value, _)| (GaugeIndex::Class, gauge_value))` (`apps/rbms-player/src/app_play.rs:578`). 사용자 GAUGE 설정(HARD 등)은 시작 슬롯에 영향이 없다.
2. 게이지 표: `judge_setup.gauge_set = overrides.gauge_set_for(judge_setup.gauge_set)` (`app_play.rs:458-462`). `judge_setup.gauge_set` 의 원래 값은 설정 GAUGE SET(AUTO=None 또는 LR2, `crates/rbms-config/src/judge.rs:28-31`). 제약 `gauge_lr2/5k/7k/9k/24k` 가 있으면 그 표(`apps/rbms-player/src/course_ui.rs:301-305`), 없으면 설정, 그것도 None 이면 차트 모드의 표(`GaugeSetId::for_mode`, `gauge_tables.rs:85-92`).
3. 코스 시작 값: `course_start_gauge(set)` 가 그 표 Class 행의 `init` 을 읽는다(`apps/rbms-player/src/lib.rs:259-261`). 호출: `start_course` (`lib.rs:270-283`, `set = overrides.gauge_set_for(config.judge.gauge_set).unwrap_or_default()` — 차트 모드를 모르므로 Beatoraja 쪽은 7키 표로 읽는다).
4. 원본은 사용자 게이지로 시작 슬롯을 정한다: `id = type <= 2 ? 6 : (type == 3 ? 7 : 8)` (`GrooveGauge.java:164`). R-BMS 에서 이 대응은 GAUGE AUTO SHIFT 의 `SelectToUnder` 상한 계산에만 있다(`gauge.rs:608-611`). **조사 보고 C 절 결정 지점의 "R-BMS 가 이미 이 규칙을 가짐" 은 틀렸다**(2.9절).
5. 이월: `CourseRun.carry_gauge: f32` 한 값(`crates/rbms-course/src/run.rs:132`), 스테이지가 끝날 때 `stage.gauge_value`(선택 게이지의 끝 값)로 바뀐다(`run.rs:178`). 다음 스테이지는 이 값으로 `Class` 를 세팅한다(`crates/rbms-play/src/lib.rs:421-426 set_initial_state`). 원본은 9슬롯 전부를 이월한다(`GrooveGauge.java:150-155`, 직전 곡 게이지 기록 각 슬롯의 마지막 값).
6. 코스 중 GAS: `BestClear` 는 코스면 `Class..=ExHardClass` 를 훑고(`gauge.rs:596-623`), `SurvivalToGroove` 는 코스 게이지를 옮기지 않는다(584-589). 원본 `BMSPlayer.java:639-667` 과 같다.
7. 실패: 선택 게이지가 0 이 되면 GAS None 일 때 판이 끝난다(`gauge.rs:580-583`, 원본 `BMSPlayer.java:653-655`). 클리어는 `value > 0 && value >= border` (`gauge.rs:390-392`).

### 2.3 코스 제약 적용

1. `CourseOverrides::of` (`course_ui.rs:285-313`): grade·grade_mirror·grade_random → `RandomLock`, no_speed → 속도 잠금, gauge_* → `gauge_set`, ln/cn/hcn → `ln_mode`, no_good/no_great → `unapplied` 에 배지만(300).
2. `RandomLock::Mirror` 는 MIRROR 만 남기고 나머지를 OFF 로(`course_ui.rs:246-252`) — 원본 `MusicSelector.java:483-491`(random==1 이면 유지, 아니면 0) 과 같다. 원본은 이때 `random2 = 1`, `doubleoption = 1`(FLIP)도 세팅한다. R-BMS 는 DP 에서도 한 `NoteOption` 을 두 사이드에 쓰고(`crates/rbms-chart/src/shuffle.rs:115-144`), `LaneOption`(FLIP 등)은 기록에만 쓰고 플레이에 적용하지 않는다(`grep lane_option`: `skin_host/options.rs:189`, `play_menu/rows.rs:395` 뿐).
3. **`ln`·`cn`·`hcn` 제약은 적용되지 않는다.** `ln_mode_for` 는 `#[cfg(test)]` 이고(`course_ui.rs:315-318`), 앱 어디에서도 코스의 `ln_mode` 를 판정 설정에 넣지 않는다(`app_play.rs` 의 `judge_setup.ln_mode` 는 설정 `config.judge.ln_mode` 그대로, `apps/rbms-player/src/judge_setup.rs:31`). 원본은 `config.setLnmode(0)` (`MusicSelector.java:501`). 조사 보고는 "제약 적용은 CourseOverrides" 라고만 적었다(2.9절 정정).
4. NO GOOD/NO GREAT: 원본은 판정 폭 비율을 0 으로 만든다(`JudgeManager.java:175-183`: NO_GREAT 는 GREAT·GOOD, NO_GOOD 는 GOOD). R-BMS 는 미적용. 2026-10-23 표 header 의 코스(발광 ★ 49개·☆ 31개·Stella 26개·Satellite 13개)와 GENOSIDE 2018 공식 44개 어느 것도 이 제약을 쓰지 않는다(제약 집합은 모두 `grade_mirror, gauge_lr2, ln`, 3.6절의 실측).
5. 제약 묶음 규칙: 같은 그룹은 처음 것만 남긴다(`crates/rbms-course/src/model.rs:325-336 collapse_constraints`, 원본 `CourseData.java:118-124`).
6. `course_ui.rs:14-16` 머리 주석 "The module is complete but not yet called ... the allow below goes away" 는 낡았다. 실제로 `lib.rs:115` 에서 쓰이고, 파일에 `#[allow]` 는 없다.

### 2.4 코스 제출과 재전송

1. 본문 `rbms_ir::dto::CourseSubmission` (`crates/rbms-ir/src/dto/score.rs:346-384`): api_version, course_hash, player, clear, ex_score, judge, max_combo, gauge_value, charts, played_at, lntype, max_ex_score, minbp, trophy, scratch_auto(없으면 생략), assist(비면 생략), extra. **게이지 종류·RANDOM·코스 게이지 모드가 없다.**
2. 만드는 곳 `build_course_submission(run, player, assist)` (`apps/rbms-player/src/course_ir.rs:71-109`). `lntype` 은 코스 제약에서만 읽는다(`course_lntype`, 121-129). 문서 주석(24, 67)의 "the caller overwrites it with what the run used" 는 사실이 아니다(덮어쓰는 곳이 없다).
3. 스테이지를 코스에 접는 곳은 세 군데이고 모두 그 스테이지의 `RecordedRun`(점수 본문 `sub` 포함)을 갖고 있다: 결과 진입(`apps/rbms-player/src/app_result.rs:151-160`), 앱 종료로 끊김(`app_result.rs:545-575`), 코스 도중 Esc(`app_result.rs:595-613`). 접는 함수 `fold_course_stage` (786-812).
4. 스테이지 점수 본문은 이미 `options.gauge = ir_gauge_of_index(finished_gauge)`(코스 게이지 이름), `options.random = ir_random(config.play.random)`, `options.lntype = state.lntype` 를 싣는다(`app_result.rs:285-289`).
5. 와이어 이름: `GaugeType` 은 serde 기본(PascalCase) `"Class"`·`"ExClass"`·`"ExHardClass"` (`crates/rbms-ir/src/dto/chart.rs:31-42`; 골든 `contract/client-requests/score-normal.json:52` `"gauge": "Normal"`). `RandomOption` 도 `"Off"`·`"Mirror"`… (chart.rs:44-55).
6. 보관과 재전송: 끝난 코스 본문은 JSON 그대로 `course_submission` 에 남고(`crates/rbms-store/src/scoredb/courses.rs:114` `kept_courses`, 151 `course_candidates`), 기록 동기화가 `CourseSubmission::from_json` 으로 읽어 `player.id` 만 바꿔 다시 보낸다(`apps/rbms-player/src/record_sync.rs:466-477`). trace 소급은 본문의 `charts` 만 읽는다(`apps/rbms-player/src/trace_backfill.rs:347-354`). 따라서 새 필드는 serde 기본값만 갖추면 옛 본문도 읽히고, 옛 본문은 옛 모양 그대로 다시 나간다.
7. 서버 DTO `courseSubmissionSchema` 는 `z.object(...)` 이고 `.strict()`·`strictObject` 가 없다(`web/src/server/dto/course.ts:44-66`, `grep strict web/src/server` 0건). Zod 4(`web/package.json:40` `^4.5.4`)의 `z.object` 는 모르는 키를 버린다. **구 서버는 새 최상위 키를 버리고 400 을 내지 않는다.** `chartIdSchema` 의 md5·sha256 은 빈 문자열 기본값이라 sha256 이 빈 차트도 통과한다(`web/src/server/dto/common.ts:63-64, 97-100`).
8. 계약 골든: `contract/client-requests/course-normal.json`, `course-scratch-auto.json` (테스트 `apps/rbms-player/src/app_result/record_tests/contract_requests.rs:861-871`, 갱신 명령 같은 파일 31-32행), 설정 묶음 `settings-blob.json`(같은 파일 940행). 웹 쪽 교차 검사 `web/tests/dto/client-requests.test.ts`, `web/tests/db/submission-integrity.test.ts:59`.

### 2.5 코스 모델·id·표시

1. `Course { name, charts: Vec<CourseChart{md5, sha256, title}>, constraints, trophies, release }`, 별칭 `hash`/`song`, `constraint`, `trophy` (`model.rs:243-254`). 표 header 식 `"md5": ["…"]` 문자열 배열은 이 모델로 읽히지 않는다(`md5` 키가 없어 charts 가 비고 `validate` 가 거절).
2. 코스 id: `Course::hash()` = SHA-256( 이름 UTF-8 + `\n` + 차트마다 (sha256 이 있으면 sha256, 없으면 md5) + `\n` ) 의 소문자 hex (`model.rs:304-313`, `CourseChart::hash_key` 229-231). 원본의 랭킹 캐시 키는 이름을 넣지 않는다(`RankingDataCache.java:84-96`, 주석 299-303).
3. 코스 파일은 `<설정 폴더>/courses/*.json` 에서만 읽는다(`course_ui.rs:70-72`, `stage/select/mod.rs:463-469 refresh_courses`, `crates/rbms-course/src/load.rs:47-58`). 번들 코스·표 코스는 없다.
4. 표 header 의 `course` 는 `Option<serde_json::Value>` 로 보관만 한다(`crates/rbms-table/src/dto.rs:50-54`, `lib.rs:28-29`). rbms-table 은 rbms-course 에 기대지 않는다(`crates/rbms-table/Cargo.toml`).
5. 14키(DP) 지원: `BEAT_14K` 키 배치(`apps/rbms-player/src/keyconfig.rs:304`), 모드 이름(347), 2P 턴테이블 역방향 키(`keyconfig_tests.rs:482`), 스킨 모드 띠 14K(`docs/play-options.md:290`), 계약 골든 `score-14k-2p.json` 이 있다. DP 코스는 곡이 있으면 지금 코스 경로로 그대로 친다.
6. 코스 결과 행은 `result_rows(run)` (`course_ui.rs:376-391`), 스킨 코스 결과는 `CourseSkinRun` 이 `shared.run_records` 의 스테이지별 `PlayRecord`(9슬롯 `gauge_log`, `finished_gauge`)를 읽는다(`apps/rbms-player/src/stage/course_result/skin_run.rs:39-55`, `crates/rbms-play/src/record.rs:134-159, 426-461`).

### 2.6 설정

1. `JudgeOptions` 는 `#[serde(default)]` (`crates/rbms-config/src/schema.rs:630-631`), `gauge_set` 은 토큰 직렬화(652-653, 956-967). 스키마 세대 6(`schema.rs:20`). 새 필드는 세대를 올리지 않고 더할 수 있다.
2. JUDGE 탭 GAUGE 구역: `GaugeSet, GaugeAutoShift, BottomShiftableGauge, Total` (`apps/rbms-player/src/stage/settings/catalog.rs:44`). 행 정의 `crates/rbms-config/src/settings.rs:688-694`(GAUGE SET), 값 표시 1074, 걸음 1260-1263.
3. 연습 패널은 게이지 표를 `GaugeSetId::ALL` 로 돌고 토큰 표가 `[_; GaugeSetId::COUNT]` 다(`apps/rbms-player/src/practice.rs:169-180, 372-378`). `COUNT` 를 바꾸면 이 배열이 컴파일 오류로 같이 바뀌어야 한다.

### 2.7 외부 원천(2026-10-23 사본, 다시 읽음)

1. 공식 코스 파일 `/private/tmp/claude-501/-Users-hyunseokbyun-development-R-BMS/93f31835-8882-49ef-96ed-d71e20e979cb/scratchpad/crs_official/2018_course.zip`: 4,084 바이트, SHA-256 `c426719dcf13c279fe003fcd80258fd625c3b796e7c5adda21abec1655df65ca`, 항목 하나 `test.lr2crs`(12,042 바이트, 2018-04-15 16:45, SHA-256 `9d7981aeab8259d96349cc32237164ba1798344e6372cb6c7ee3bb83b417ffb7`). XML, `encoding="shift_jis"`, CRLF, `<courselist>` 안에 `<course><title/><line/><hash/><type/></course>` 44개.
2. 순서: SP(`<line>7</line>`) Overjoy → 発狂皆伝 → 発狂十段 … 発狂初段 → 十段 … 初段, 이어 DP(`<line>14</line>`) 같은 순서. 제목은 `GENOSIDE 2018 段位認定 {단}` / `GENOSIDE 2018 DP段位認定 {단}`.
3. `<hash>` 는 160자 = 접두사 `00000000002000000000000000005190`(44개 모두 같음) + md5 32자 x 4. `<type>` 은 44개 모두 `2`.
4. beatoraja 기본 표 header 사본(`.../scratchpad/web/mirai_header.json` = 発狂 ★, SHA-256 `0b98df7687b1d64f07f8282ee3f02bbe43dfa235c6be2126581d3807fd27adaf`, Last-Modified 2025-08-05; `mirai_normal_header.json` = 通常 ☆, SHA-256 `0206d0358854f5133ec52720efa54824e55167db492f933c5483f4c0933096fd`). 두 파일 모두 **UTF-8 BOM 으로 시작**한다. `course` 는 배열의 배열(★ 4묶음 13/12/12/12, ☆ 3묶음 11/10/10).
5. 공식 SP 22개의 md5 열은 header 코스와 **md5 열로** 22/22 일치한다(이름이 아니라). 이름은 다르다: ☆ 쪽은 `GENOSIDE 2018 初段`(段位認定 없음), ★ 쪽은 `GENOSIDE 2018 段位認定 発狂初段`, Overjoy 는 `GENOSIDE 2018 段位認定 overjoy`(소문자). 공식 DP 22개는 header 에 하나도 없다(0/22).
6. 제약·트로피(GENOSIDE 2018 SP 22개): 제약은 모두 `["grade_mirror","gauge_lr2","ln"]`. 트로피 ☆ 10개: bronzemedal(7.5, 0.0), silvermedal(5.0, 70.0), goldmedal(2.5, 85.0). ★ 12개(皆伝·overjoy 포함): silvermedal(5.0, 70.0), goldmedal(2.5, 85.0).
7. Stella header(`.../scratchpad/web/header.json`, symbol st): `course` 는 1묶음 26개, 키는 `name, constraint, trophy, md5`, md5 개수 4 또는 5, 제약 모두 `grade_mirror, gauge_lr2, ln`. Satellite(`sl_header.json`, sl): 1묶음 13개, md5 4개. **조사 보고 4절의 "코스 약 78개" 는 틀렸다**(2.9절).
8. iidx.org 원문(사본 `.../scratchpad/web/iidxorg_compendium_gauges_and_timing.txt`, `iidxorg_beginner_gauge.txt`, `iidxorg_misc_iidx_lr2_beatoraja_diff.txt`):
   - compendium 표: `DAN 0.16 0.16 0.04 -1.5 -2.5 -1.5 Yes`, `EX-DAN 0.16 0.16 0.04 -3.0 -5.0 -3.0 No`. 각주: "Low Life Adjustment: when the gauge is **at or below 30%**, the rate of decrease caused by BAD and POORs are halved." 생존 게이지는 "you start at 100%, and get a stage fail if you drop to 0%".
   - beginner: DAN 은 "You start at 100%, and fail out if it falls below 2%. The gauge amount carries over from one stage to the next. It is like hard mode in that there is a halved decrease rate at 30%". HARD 절: "**At or below 30%** gauge, the rate of decrease for BAD / POORs will halve."
   - misc: "In IIDX and LR2, the gauge decreases at a lower rate when the gauge is **at or below 30%**", DAN 표의 IIDX 칸 `30+ : -1.5 -2.5 -1.5`, `30 : -0.75 -1.25 -0.75`(빈 POOR 칸도 절반). "These are for 7key and 14key modes".
   - 결론: 경계는 **이하(≤ 30)** 로 확정. 빈 POOR 도 절반(misc 표, 사용자 결정 21 요약과 같다). 실패선은 사용자 결정대로 0%(compendium 과 같고 beginner 의 2% 는 쓰지 않는다).

### 2.8 `.lr2crs` 의 `<type>2`

확인 못함. beatoraja 소스에는 `.lr2crs` 를 읽는 코드가 없고(`grep -rn lr2crs src` 0건), LR2 는 소스가 비공개이며, 조사 산출물에도 형식 명세가 없다. 관찰값만 남긴다: 공식 44개·bms-ir 판 모두 `<type>2`, `<hash>` 접두사 32자가 모두 같다. 이 설계는 `<type>` 과 접두사를 쓰지 않고 데이터 파일의 출처 기록에만 남긴다(3.5절).

### 2.9 조사 보고 정정

| 조사 보고 위치 | 보고 내용 | 실제(1169efb) |
| --- | --- | --- |
| C 절 결정 지점 "사용자 게이지 설정과 코스 게이지 선택의 관계" | beatoraja 식이 추천이고 "R-BMS 가 이미 이 규칙을 가짐" | 시작 슬롯은 항상 `Class` (`app_play.rs:578`). 대응식은 GAS `SelectToUnder` 의 상한에만 있다(`gauge.rs:608-611`). 결정 21 이 이 추천을 채택했으므로 이번에 새로 만든다 |
| 8절 "제약 적용은 CourseOverrides(course_ui.rs:285-312)" | 제약이 적용된다는 서술 | `ln`/`cn`/`hcn` 은 미적용(`course_ui.rs:315` `#[cfg(test)]`). RANDOM·NO SPEED·게이지 표만 적용 |
| 4절 외부 관찰 | Stella 코스 약 78개 | st 26개(1묶음), sl 13개(1묶음) |
| `docs/play-options.md` 10.13 4번 | "코스 게이지(CLASS, 코스 제약에 따라 EXCLASS·EXHARDCLASS)", `options.gauge` 가 `class`·`exclass`·`exhardclass` | 제약은 EXCLASS 계열을 고르지 않는다(GAS 만 옮긴다). 와이어 이름은 `"Class"`·`"ExClass"`·`"ExHardClass"`(소문자는 점수 DB 행의 토큰, `trace_backfill.rs:104`) |
| 8절 "course_ui.rs 머리 주석 14-17행" | 낡은 주석 | 14-16행. "the allow below" 가 가리키는 `#[allow]` 도 없다 |
| 8절 "코스 진행 중에는 항상 Class 로 시작(app_play.rs:578)", "lib.rs:259-261", "app_play.rs:459-462" | — | 줄 번호 모두 맞다 |
| `course_ir.rs` 문서 주석 24·67행 | lntype 을 호출자가 덮어쓴다 | 덮어쓰는 곳 없음 |

---

## 3. 설계

### 3.1 게이지 표: IIDX 세트와 "이하" 완충

**타입 변경(crates/rbms-judge)**

```rust
/// Where a softening band starts: below its threshold, as the reference's `value < gut[0]` has it,
/// or at it, as IIDX's "at or below 30%" has it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum GutsEdge {
    #[default]
    Below,
    AtOrBelow,
}
```

1. `data.rs` `GaugeParams` 에 `#[serde(default, skip_serializing_if = "GutsEdge::is_below")] pub guts_edge: GutsEdge` 를 더한다. 기존 `gauge.ron` 행과 사용자가 둔 옛 덮어쓰기 파일은 필드가 없어 `Below` 로 읽힌다. **`CURRENT_GAUGE_DATA_VERSION` 은 1 그대로**(읽기 호환, 쓰기도 Below 는 생략돼 옛 모양).
2. `gauge_tables.rs` `GaugeElement` 에 `pub guts_edge: GutsEdge` 를 더하고 `element()` 는 `Below` 를 넣는다. IIDX DAN 행만 `const fn softened_at_or_below(e: GaugeElement) -> GaugeElement` 로 감싼다. `to_params` 가 옮긴다.
3. `gauge.rs` `Gauge` 에 `guts_edge` 필드, `at_index` 가 옮기고, `update_with_rate`(333-345)의 비교를 `let softened = match self.guts_edge { GutsEdge::Below => self.value < g.0, GutsEdge::AtOrBelow => self.value <= g.0 };` 로 바꾼다. 다른 모든 행은 `Below` 라 **기존 판정은 화소·수치 모두 그대로**다.
4. 새 상수: `pub const GUTS_IIDX: &[(f32, f32)] = &[(IIDX_LOW_LIFE_THRESHOLD, IIDX_LOW_LIFE_DAMAGE_RATE)];` 와 `const IIDX_LOW_LIFE_THRESHOLD: f32 = 30.0;`, `const IIDX_LOW_LIFE_DAMAGE_RATE: f32 = 0.5;` (근거 2.7절 8, 주석이 아니라 `///` 영어 문서로 iidx.org 를 적는다).
5. `GaugeSetId` 에 `Iidx` 를 더한다(rbms 확장, 원본 5세트 뒤 6번째). `COUNT = 6`, `ALL` 6개, `index()` Iidx=5, `data_keys()` = `["IIDX"]`, `pub const IIDX_GAUGE_KEY: &str = "IIDX";`. `for_mode` 는 Iidx 를 고르지 않는다(Lr2 와 같다). 문서 주석 "the reference implementation's five gauge tables" 를 "the reference's five tables and the IIDX dan table this engine adds" 로 고친다.
6. `GAUGE_TABLE` 6번째 행(IIDX):

| 슬롯 | 값 | 근거 |
| --- | --- | --- |
| 0~5 (AssistEasy~Hazard) | 7키 세트(`SevenKeys`) 행을 그대로 복사 | 코스에서 선택되지 않는 자리다(코스는 Class 이상에서 시작하고 GAS 도 코스 게이지 밖으로 옮기지 않는다, 2.2절 6). 9슬롯 구조를 유지하려고 채운다. IIDX 비코스 게이지(a 값)는 원문이 차트별 함수라 구현하지 않는다 |
| 6 Class (DAN) | `None, min 0, max 100, init 100, border 0, [0.16, 0.16, 0.04, -1.5, -2.5, -1.5], GUTS_IIDX, AtOrBelow` | compendium 표, 결정 21 |
| 7 ExClass (EX-DAN) | `None, 0, 100, 100, 0, [0.16, 0.16, 0.04, -3.0, -5.0, -3.0], GUTS_NONE` | compendium 표("No") |
| 8 ExHardClass | 7번과 같은 값(EX-DAN 복사) | IIDX 에는 셋째 단위 게이지가 없다. 코스에서는 상한(아래 7)으로 닿지 않는다. 연습 패널에서 고르면 EX-DAN 수치에 EX-HARD 램프 이름이 붙는다(9절 위험) |

7. 코스 상한: `impl GaugeSetId { pub fn course_ceiling(self) -> GaugeIndex }` — `Iidx` 는 `ExClass`, 나머지는 `ExHardClass`. `GrooveGauge::shift_to_best`(596-623)의 코스 상한 `len` 두 갈래에 `.min(self.set.course_ceiling().index() + 1)` 를 건다. 원본 5세트는 상한이 ExHardClass 라 동작이 같다.
8. 시작 슬롯: `impl GaugeSetId { pub fn course_gauge_for(self, chosen: GaugeKind) -> GaugeIndex }` — `AssistEasy | Easy | Normal → Class`, `Hard → ExClass`, `ExHard | Hazard → ExHardClass`, 그 뒤 `course_ceiling()` 로 자른다(원본 `GrooveGauge.java:160-166`). IIDX 는 HARD 이상이 모두 ExClass(EX-DAN)가 된다(8절 결정 1).
9. `gauge.ron` 에 `"IIDX"` 키 9행을 더한다(Class 행만 `guts_edge: AtOrBelow`). `data.rs` 의 `GaugeSet` 문서 주석 "never selects one, having no course mode" 를 고친다.

**코스 게이지 모드 타입(같은 파일 `gauge_tables.rs`)**

```rust
/// Which gauge table a course is played on, chosen in the settings (the player's decision of
/// 2026-10-23): it outranks a course's own gauge constraint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CourseGaugeMode {
    #[default]
    Lr2,
    Beatoraja,
    Iidx,
}

impl CourseGaugeMode {
    pub const ALL: [CourseGaugeMode; 3] = [CourseGaugeMode::Lr2, CourseGaugeMode::Beatoraja, CourseGaugeMode::Iidx];
    /// The table a stage of a chart of `mode` plays on.
    pub fn gauge_set(self, mode: &Mode) -> GaugeSetId {
        match self {
            CourseGaugeMode::Lr2 => GaugeSetId::Lr2,
            CourseGaugeMode::Beatoraja => GaugeSetId::for_mode(mode),
            CourseGaugeMode::Iidx => GaugeSetId::Iidx,
        }
    }
}
```

`rbms-judge/src/lib.rs` 의 재수출은 하지 않는다(경로 `rbms_judge::gauge_tables::CourseGaugeMode` 로 쓴다, `GaugeSetId` 와 같다).

**모드별 적용(결정 21 이 다루지 않는 세부, 추천안)**

| 차트 모드 | LR2 | BEATORAJA | IIDX |
| --- | --- | --- | --- |
| 7키·14키 | LR2 세트 | 7키 세트(`BMSPlayerRule.java:13-17`) | IIDX 세트 |
| 5키·10키 | LR2 세트(원본도 `gauge_lr2` 코스면 모드와 무관하게 LR2, `GrooveGauge.java:142-144`) | 5키 세트 | IIDX 세트 그대로(원문은 7·14키 값뿐, 8절 결정 2) |
| 9키(PMS) | LR2 세트 | PMS 세트 | IIDX 세트 그대로 |
| 24키 | (모드 미연결, `data.rs:16-19`) | KEYBOARD 세트 | IIDX 세트 그대로 |

### 3.2 설정 COURSE GAUGE(결정 21)

1. `crates/rbms-config/src/judge.rs`: `pub const COURSE_GAUGE_LABELS: &[&str] = &["LR2", "BEATORAJA", "IIDX"];`(순서 = `CourseGaugeMode::ALL`), `pub fn course_gauge_token(mode: CourseGaugeMode) -> &'static str`(라벨과 같은 대문자 토큰), `pub fn course_gauge_from_token(token: &str) -> CourseGaugeMode`(대소문자 무시, 모르면 `Lr2`).
2. `schema.rs` `JudgeOptions` 에 `#[serde(with = "course_gauge_token")] pub course_gauge: CourseGaugeMode` (기본 `Lr2`). `gauge_set_token` 모듈과 같은 꼴의 `mod course_gauge_token`. 스키마 세대는 6 그대로(필드 없는 파일은 기본값).
3. `settings.rs`: `SettingId::CourseGauge` 를 `GaugeSet` 바로 뒤에 넣고, 행 `row(SettingId::CourseGauge, SettingTab::Judge, "COURSE GAUGE", SettingKind::Cycle { values: COURSE_GAUGE_LABELS }, "Which gauge table a course is played on, ahead of the course's own gauge constraint")`, `display_value` 와 `step`(1074·1260 옆)에 갈래 추가. `lib.rs` 재수출에 새 이름 셋.
4. `apps/rbms-player/src/stage/settings/catalog.rs:44` GAUGE 구역: `[GaugeSet, CourseGauge, GaugeAutoShift, BottomShiftableGauge, Total]`.
5. 설정 GAUGE SET(AUTO/LR2)은 단일 곡 전용이 된다: 코스 중에는 COURSE GAUGE 만 표를 정한다(3.3절). GAUGE SET 행 설명은 그대로 두고, 문서(play-options)에 "코스에서는 COURSE GAUGE" 를 적는다.
6. 계정 동기화 묶음(`settings-blob.json`)에 `course_gauge: "LR2"` 한 줄이 생긴다(6.3절).

### 3.3 코스 진행 통합

> 구현과 다름: (2)의 스테이지마다 설정값을 읽는 방식 대신, 코스를 시작할 때 `config.judge.course_gauge` 를 한 번 읽어 `CourseFacts.gauge_rule` 에 담고 이어지는 스테이지는 그 값으로 표를 고른다(`stage_judge_setup`). 코스 도중 설정을 바꿔도 시작 때의 규칙이다([ADR](../acknowledge/2026-10-23-adr-course-gauge-modes.md)).

**(1) 스테이지 게이지 결정 함수(apps/rbms-player/src/course_ui.rs)**

```rust
/// How a course stage's gauge starts: the table, the slot, its value, and every slot's value the
/// stage before left (`GrooveGauge.java:123-158`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct CourseStageGauge {
    pub(crate) set: GaugeSetId,
    pub(crate) start: GaugeIndex,
    pub(crate) value: f32,
    pub(crate) carried: Option<[f32; GaugeIndex::COUNT]>,
}

pub(crate) fn course_stage_gauge(rule: CourseGaugeMode, chosen: GaugeKind, mode: &rbms_model::Mode, run: &CourseRun) -> CourseStageGauge
```

- `set = rule.gauge_set(mode)`, `start = set.course_gauge_for(chosen)`.
- 첫 스테이지(`run.carry_gauges == None`): `value = run.carry_gauge`(코스 시작 값), `carried = None`.
- 다음 스테이지: `carried = run.carry_gauges`, `value = carried[start]`.
- `app_play.rs` `load()` 와 계약 테스트 도우미 `contract_requests.rs:236-250 loaded()` 가 **같은 함수**를 부른다(테스트가 앱과 다른 길로 세션을 만들지 않게).

**(2) `app_play.rs` `load()` 변경(457-470, 569-581)**

1. `course_state` 를 `self.course_run.as_ref().map(|run| course_stage_gauge(self.config.judge.course_gauge, self.config.play.gauge, &mode, run))` 로 바꾼다(리플레이면 None, 지금과 같다).
2. `course_state` 가 있으면 `judge_setup.gauge_set = Some(stage.set)` (설정 GAUGE SET 과 제약 `gauge_*` 를 모두 무시, 결정 21).
3. 같은 자리에서 `judge_setup.ln_mode = overrides.ln_mode_for(judge_setup.ln_mode)` — `ln_mode_for` 의 `#[cfg(test)]` 를 지운다. 반드시 `run_lntype`·`ln_mode_key`·`resolve_long_note_flavour`(463-472) **앞**이어야 한다(그 셋이 LN 종류로 점수 키와 차트를 정한다).
4. `SessionOptions` 에 `initial_gauge: course_state.map(|s| (s.start, s.value))`, `carried_gauges: course_state.and_then(|s| s.carried)`, `initial_combo` 는 지금처럼.

**(3) 9슬롯 이월(8절 결정 3·4, 추천: 원본처럼 9슬롯)**

1. `crates/rbms-play/src/session.rs`: `SessionOptions` 와 `Setup` 에 `pub carried_gauges: Option<[f32; GaugeIndex::COUNT]>`(기본 None). `apply_setup`(1197-1213)에서 `set_initial_state` 다음에 `if let Some(values) = setup.carried_gauges { player.carry_gauges(&values); }`. 분석 seek 가 `Setup` 으로 다시 만들므로 seek 뒤에도 같다.
2. `crates/rbms-play/src/lib.rs`: `pub fn carry_gauges(&mut self, values: &[f32; GaugeIndex::COUNT])` — 각 슬롯 `self.judge.gauge.set_value_at(index, values[i])`. 선택 슬롯은 `set_initial_state` 와 같은 값이라 결과가 같다. 0 인 슬롯은 0 으로 남는다(`add_value` 가 0 이하에서 멈추는 것은 원본 `setValue` 와 같다).
3. `PlaySession::gauge_values(&self) -> [f32; GaugeIndex::COUNT]` 를 더한다(판 끝의 정확한 값, `value_at` 9번).
4. `crates/rbms-course/src/run.rs`: `pub const GAUGE_SLOTS: usize = 9;`(문서: "the reference's nine gauge slots"), `CourseRun` 에 `pub carry_gauges: Option<[f32; GAUGE_SLOTS]>`(new 에서 None). `StageResult` 는 바꾸지 않는다(리터럴 22곳의 수정을 피한다).
5. `app_result.rs` `fold_course_stage` 가 `run.advance(&stage)` 뒤에 `run.carry_gauges = Some(gauges)` 를 넣는다. 세 호출처가 `state.session.gauge_values()` 를 넘긴다. 앱 쪽에 `const _: () = assert!(rbms_course::GAUGE_SLOTS == GaugeIndex::COUNT);` 를 둔다(매직넘버 대신 컴파일 시점 일치).
6. 원본은 직전 곡 게이지 **기록 표본**의 마지막 값을 쓴다. 여기서는 판 끝의 정확한 값을 쓴다(지금 `carry_gauge` 가 `summary.gauge_value` 를 쓰는 것과 같은 선택). 차이 표 4절 D7.

**(4) 코스 시작(`lib.rs:259-283`)**

1. `course_start_gauge(set: GaugeSetId, start: GaugeIndex) -> f32` 로 바꿔 시작 슬롯의 `init` 을 읽는다. `start_course` 는 첫 차트 모드를 모르므로 `set = config.judge.course_gauge.gauge_set(&Mode::BEAT_7K)` 로 읽는다. 모든 세트의 코스 슬롯 init 이 100 이라 결과가 같다는 것을 테스트 `every_course_gauge_of_every_set_starts_full` 이 지킨다.
2. `CourseOverrides.gauge_set` 필드와 `gauge_set_for` 를 지운다(쓰는 곳이 사라진다). 배지(`constraint_badge`)는 코스가 선언한 제약을 그대로 보인다.
3. AppShared 에 `course_facts: CourseFacts`(3.4절)를 `course_assist` 와 같은 자리에서 시작·끝에 비운다(`lib.rs:278, 317`, `AppShared` 필드 741 옆, 생성 1082 옆).

**(5) 정리**

- `course_ui.rs:1-16` 머리 문서를 지금 상태로 고친다: "탭과 결과 행, 제약을 판에 적용하는 값, 코스 스테이지 게이지(`course_stage_gauge`)". 낡은 14-16행 삭제.
- `CourseOverrides.unapplied` 문서의 이유("needs the judge tables to be selectable per run")를 "NO GOOD·NO GREAT 는 대상 코스가 없어 아직 적용하지 않는다" 로 고친다(8절 결정 9).
- 결과 행: `result_rows(run, rule: CourseGaugeMode)` 에 `("GAUGE RULE", "LR2"|"BEATORAJA"|"IIDX")` 를 GAUGE 행 바로 뒤에 넣는다(8절 결정 12). 호출처 `stage/course_result.rs:116` 은 `shared.course_facts.gauge_rule` 을 넘긴다.

### 3.4 코스 제출 본문(결정 6·20)

**새 와이어 타입(crates/rbms-ir/src/dto/score.rs)**

```rust
/// The gauge table rule a course was played under (the COURSE GAUGE setting).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum CourseGaugeType {
    Lr2,
    Beatoraja,
    Iidx,
}

/// What one played stage of a course was played with, as that stage's own score submission states it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CourseStagePlay {
    pub gauge: GaugeType,
    pub random: RandomOption,
    pub lntype: i32,
}
```

`CourseSubmission` 에 세 필드(모두 생략 가능):

```rust
/// The course gauge the course finished on: its last played stage's.
#[serde(default, skip_serializing_if = "Option::is_none")]
pub gauge: Option<GaugeType>,
/// The COURSE GAUGE rule it was played under.
#[serde(default, skip_serializing_if = "Option::is_none")]
pub course_gauge: Option<CourseGaugeType>,
/// One entry per stage played, in order.
#[serde(default, skip_serializing_if = "Vec::is_empty")]
pub stages: Vec<CourseStagePlay>,
```

와이어 예(새 클라이언트, 2스테이지 계약 코스):

```json
"course_gauge": "LR2",
"gauge": "Class",
"stages": [ { "gauge": "Class", "random": "Off", "lntype": 0 }, { "gauge": "Class", "random": "Off", "lntype": 0 } ]
```

1. `mapping.rs`(feature `mapping`): `pub fn ir_course_gauge(mode: CourseGaugeMode) -> CourseGaugeType`.
2. 앱: `course_ir.rs` 에 `pub(crate) struct CourseFacts { pub(crate) gauge_rule: CourseGaugeMode, pub(crate) stages: Vec<rbms_ir::dto::CourseStagePlay> }`, `CourseFacts::starting(rule)`, `with_stage(&self, options: &rbms_ir::PlayOptions) -> CourseFacts`(`gauge`·`random`·`lntype` 를 그 스테이지 점수 본문에서 복사, 2.4절 4). `fold_course_stage` 가 `&RecordedRun` 을 받아(지금의 `block_reason` 인자를 대신한다) `shared.course_facts` 에 한 스테이지를 더한다. 세 호출처(2.4절 3) 모두 같다.
3. `build_course_submission(run, player, assist, facts: &CourseFacts)`: `gauge = facts.stages.last().map(|s| s.gauge)`, `course_gauge = Some(ir_course_gauge(facts.gauge_rule))`, `stages = facts.stages.clone()`. 스테이지를 하나도 접지 못한 코스는 이 함수에 오지 않는다(`abandon_course` 가 친 스테이지가 없으면 보내지 않는다).
4. 최상위 `lntype`: LN 제약이 있으면 그 값(지금과 같고, 이제 실제로 적용되므로 사실과 같다), 없으면 `stages[0].lntype`. `course_lntype` 문서의 낡은 문장을 지운다.
5. 결정 20 에 필요한 사실: RANDOM(스테이지별, 클램프 뒤의 실제 값), LN 종류(스테이지별 `lntype`), 끝난 게이지(스테이지별·코스), 어시스트(`scratch_auto`·`assist`, 이미 있음). 서버 판정은 F6.
6. **구 서버**: 2.4절 7 대로 모르는 키를 버린다. 400 없음. **옛 본문 재전송**: `Option`/`Vec` 기본값으로 읽혀 세 필드 없이 다시 나간다(서버는 게이지를 모르는 코스로 받는다 → 등급 0, 단위 판정 불가. 9절 위험 R4).
7. trace 소급(`trace_backfill::course_stages`)은 `charts` 만 읽어 영향 없음. 점수 DB 의 `course_submission` 스키마는 바뀌지 않는다(본문 JSON 그대로).

### 3.5 코스 원천 (a): GENOSIDE 2018 데이터 파일과 생성기(결정 18)

> 구현과 다름: `zip` 은 9.0.5 가 아니라 9.0.0 이다(워크스페이스 줄과 `Cargo.lock`)([ADR](../acknowledge/2026-10-23-adr-genoside-2018-course-data.md)).

**데이터 파일**: `web/data/courses/genoside-2018/courses.json`(새 파일). 클라이언트가 `include_str!` 로, 웹(F6)이 정적 import 로 같은 파일을 읽는다(선례: 클라이언트가 `web/data/tables/insane1` 을 `include_bytes!` 로 내장, `apps/rbms-player/src/tablesrc/seed.rs:31`).

```json
{
  "schema": 1,
  "source": {
    "maker": "GENOCIDE",
    "page_url": "https://nekokan.dyndns.info/~lobsak/genocide/grade.html",
    "file_url": "http://nekokan.dyndns.info/~lobsak/genoside/2018_course.zip",
    "retrieved_on": "2026-10-23",
    "file_bytes": 4084,
    "file_sha256": "c426719dcf13c279fe003fcd80258fd625c3b796e7c5adda21abec1655df65ca",
    "entry_name": "test.lr2crs",
    "entry_sha256": "9d7981aeab8259d96349cc32237164ba1798344e6372cb6c7ee3bb83b417ffb7",
    "entry_hash_prefix": "00000000002000000000000000005190",
    "entry_type": 2,
    "constraint_sources": [
      { "url": "https://miraiscarlet.github.io/bms/table/genocide_normal/header_normal.json", "sha256": "0206d0358854f5133ec52720efa54824e55167db492f933c5483f4c0933096fd", "retrieved_on": "2026-10-23" },
      { "url": "https://miraiscarlet.github.io/bms/table/genocide_insane/header_insane.json", "sha256": "0b98df7687b1d64f07f8282ee3f02bbe43dfa235c6be2126581d3807fd27adaf", "retrieved_on": "2026-10-23" }
    ],
    "title_source": "https://www.bms-ir.org/new/course?hash= (chart titles only, for display)"
  },
  "groups": [
    { "id": "genoside-2018-sp", "name": "GENOSIDE 2018 段位認定", "keys": 7, "courses": [
      { "course_hash": "ae8ddca1ddec7737a393eddbd3862d2be2e42bb7184365fbc4664efe280ed0d1",
        "name": "GENOSIDE 2018 段位認定 初段",
        "charts": [ { "md5": "ae11930223a8f24e91633f1a7864e1eb", "title": "Agito" }, "…" ],
        "constraint": ["grade_mirror", "gauge_lr2", "ln"],
        "trophy": [ { "name": "bronzemedal", "missrate": 7.5, "scorerate": 0.0 }, "…" ],
        "release": true } ] },
    { "id": "genoside-2018-dp", "name": "GENOSIDE 2018 DP段位認定", "keys": 14, "courses": [ "…" ] }
  ]
}
```

규칙:

1. 코스 이름 = 공식 `.lr2crs` `<title>` 그대로(Shift_JIS → UTF-8, 앞뒤 공백도 그대로). 차트 = `<hash>` 의 33번째 글자부터 32자씩 4개, 소문자, `sha256` 은 **비운다**(키 자체를 쓰지 않는다. `Course::hash` 가 md5 로 계산되게 — 3.7절).
2. 묶음 순서: `<line>7` → `genoside-2018-sp`, `<line>14` → `genoside-2018-dp`. 묶음 안은 공식 파일 순서를 **뒤집어** 初段 → … → 十段 → 発狂初段 → … → 発狂皆伝 → Overjoy(원본 표·bms-ir 의 오름차순 표시와 같다).
3. 제약·트로피(SP): beatoraja 기본 표 header 두 개에서 **md5 열이 같은 코스**를 찾아 그 `constraint`·`trophy` 를 그대로 쓴다(이름으로 찾지 않는다, 2.7절 5). 못 찾는 SP 코스가 하나라도 있으면 생성 실패.
4. 제약·트로피(DP): 원천이 없다. 같은 단의 SP 코스 값을 쓴다(☆ 단 → 3메달, ★·皆伝·Overjoy → 2메달, 제약 `grade_mirror, gauge_lr2, ln`). 8절 결정 4.
5. 차트 제목: bms-ir 코스 페이지의 곡 제목(표시용). SP 는 조사 산출물 `genocide_courses.json` 에 있고, DP 22개는 구현자가 `/new/course?hash={160자}` 22쪽을 1초 이상 간격으로 한 번 받아 `md5 → 제목` JSON 으로 만든다(robots.txt 가 막지 않는 경로, 조사 B-3). 제목은 id 에 들어가지 않는다. 8절 결정 5.
6. `course_hash` = `Course::hash()` 의 값을 미리 적어 둔다. 웹이 같은 규칙을 다시 구현하지 않고 이 값을 등록에 쓸 수 있고, Rust 테스트가 계산값과 같은지 지킨다.
7. `release: true` (원본 `CourseData.release` 기본값 true, `CourseData.java:34`).

**생성기**: `rbms-cli course-snapshot`(새 하위 명령, `apps/rbms-cli/src/course_snapshot.rs`). 선례 `chart-pattern`(`apps/rbms-cli/src/main.rs:69-71`)과 `web/scripts/snapshot-table.ts` 의 `--check` 종료 코드(같으면 0, 다르면 1, 실패 2).

```
rbms-cli course-snapshot \
  --zip <2018_course.zip> \
  --normal-header <header_normal.json> --insane-header <header_insane.json> \
  --titles <md5-title.json> --retrieved-on 2026-10-23 \
  --out web/data/courses/genoside-2018/courses.json [--check]
```

1. 네트워크를 쓰지 않는다. 입력 파일은 유지보수자가 받는다(URL 은 `source` 에 상수로 적는다). 같은 입력이면 바이트까지 같은 출력(`serde_json::to_string_pretty` + 끝 줄바꿈, 키 순서는 구조체 선언 순서).
2. ZIP 은 결정 15 가 승인한 `zip` 크레이트로 연다(F4 와 같은 크레이트·같은 명세를 쓴다, 아래). Shift_JIS 는 워크스페이스에 있는 `encoding_rs`(`Cargo.toml:47`). header JSON 앞의 BOM 을 벗긴다(2.7절 4).
3. `.lr2crs` 읽기는 이 모듈 안의 작은 파서(`<course>` 블록마다 `title`, `line`, `hash`, `type` 4태그만, 그 밖은 무시). 정규식 의존성 없이 `str::find` 로. `hash` 가 32 + 32 x n 글자가 아니거나 hex 가 아니면 실패.
4. `--check`: 메모리에서 만든 결과와 `--out` 파일 바이트를 비교.
5. rbms-cli `Cargo.toml` 에 `rbms-course`, `encoding_rs`, `zip` 을 더한다(워크스페이스 내부·기존 크레이트 + 승인 크레이트).

**`zip` 크레이트(결정 15 승인, 2026-10-23 crates.io 확인)**: 최신 안정 `9.0.5`(2026-10-17 게시, 라이선스 MIT, MSRV 1.88 — 워크스페이스 `rust-version = "1.95"` 안). 기본 기능은 aes·bzip2·deflate64·lzma·ppmd·zstd·xz·time 까지 끌어오므로 `zip = { version = "9.0.5", default-features = false, features = ["deflate-flate2-zlib-rs"] }` 로 워크스페이스에 둔다. 이 구성의 필수 의존: `crc32fast`·`indexmap`·`memchr`·`nix`(macOS·BSD 만)·`typed-path` + `flate2`(zlib-rs 백엔드). 1169efb 의 `Cargo.lock` 에 이미 있는 것: flate2 1.1.10, crc32fast 1.5.2, indexmap 2.14.2, memchr 2.8.3, nix 0.31.3, zlib-rs 0.6.8. 새로 들어오는 것: `zip`, `typed-path`(0.13.x, MIT OR Apache-2.0), 경우에 따라 `libz-rs-sys`. 9.0.5 가 게시된 지 6일이라 F4 와 같이 들어갈 때 `cargo update -p zip --precise 9.0.5` 로 고정하고 `--locked` 로 검증한다. F4 가 먼저 들어오면 그 항목을 그대로 쓰고, 이 묶음이 먼저면 이 묶음이 워크스페이스 항목을 만든다(같은 명세여야 한다, 5절 겹침).

**클라이언트 내장 모듈**: `crates/rbms-course/src/bundled.rs`(새 파일)

```rust
pub struct BundledCourses { pub schema: u32, pub source: BundledSource, pub groups: Vec<BundledGroup> }
pub struct BundledSource { pub maker: String, pub page_url: String, pub file_url: String, pub retrieved_on: String,
    pub file_bytes: u64, pub file_sha256: String, pub entry_name: String, pub entry_sha256: String,
    pub entry_hash_prefix: String, pub entry_type: u32, pub constraint_sources: Vec<SourceFile>, pub title_source: String }
pub struct SourceFile { pub url: String, pub sha256: String, pub retrieved_on: String }
pub struct BundledGroup { pub id: String, pub name: String, pub keys: u32, pub courses: Vec<BundledCourse> }
pub struct BundledCourse { pub course_hash: String, #[serde(flatten)] pub course: Course }
/// The GENOSIDE 2018 grade courses, SP then DP, parsed once from the file the build carries.
pub fn genoside_2018() -> &'static BundledCourses
```

- `include_str!("../../../web/data/courses/genoside-2018/courses.json")`, `OnceLock` 로 한 번 파싱, 파싱 실패는 `expect("bundled GENOSIDE 2018 courses parse")`(선례 `builtin_gauge_tables`, `crates/rbms-judge/src/data.rs:211-214`; 테스트가 파싱을 지키므로 실패는 빌드 결함). 파싱 뒤 각 `course.validate()`.
- `lib.rs` 에 `pub mod bundled; pub mod table;` 과 재수출.

### 3.6 코스 원천 (b): 표 header `course` → `Course` 변환(선곡 폴더 설계가 부르는 함수)

**확정 이름·시그니처**(`crates/rbms-course/src/table.rs`, 새 파일):

```rust
/// What a difficulty table header's `course` field lists, read as courses.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TableCourses {
    /// Every course that survives [`Course::validate`], groups flattened, in the order written.
    pub courses: Vec<Course>,
    /// Entries left out: not an object, or a course with no chart or a chart with neither hash.
    pub skipped: usize,
}

/// Read a table header's `course` field (`TableDataAccessor.java:192-213`).
pub fn courses_from_table_header(course: &serde_json::Value) -> TableCourses;

/// Whether two courses play the same charts in the same order, by MD5 (case-insensitive), whatever
/// their names.
pub fn same_charts(a: &Course, b: &Course) -> bool;

/// The bundled course that plays the same charts as `course`, if one does.
pub fn bundled_twin(course: &Course) -> Option<&'static Course>;
```

동작:

1. 입력 모양: 배열의 배열(Stella·Satellite·miraiscarlet), 코스 객체의 평평한 배열, 코스 객체 하나 — 모두 읽는다. 묶음은 원본처럼 평평하게 편다(`Stream.of(dt.getCourse()).flatMap(...)`).
2. 각 코스: 객체를 `Course` 로 역직렬화해 `name`·`constraint`(모르는 토큰 버림, `model.rs:178-181`)·`trophy` 를 얻고, `charts` 가 비었으면 `md5`(문자열 배열 또는 문자열 하나)·`sha256`(같은 꼴)을 차례로 맞춰 `CourseChart` 를 만든다(md5·sha256 소문자, 원본 `toSongData` 의 `toLowerCase`). `charts` 객체 배열이 있으면 그대로 쓴다. 숫자로 적힌 값은 글자로(`rbms_table::dto` 의 lenient 와 같은 취지, 다만 이 크레이트 안에 작게 다시 둔다 — rbms-table 은 rbms-course 를 모르고 반대도 그렇다).
3. `release = true`, `validate()` 를 거친다. 실패는 `skipped` 에 센다.
4. jbmstable-parser(원본이 쓰는 표 파서)는 소스 트리에 없어(`find ... DifficultyTableParser` 0건) 그쪽의 정확한 허용 범위는 확인 못함. 위 세 꼴을 다 읽는 것은 그 대신이다.
5. 호출 쪽(F3): 표의 저장본 `DifficultyTable.course`(`crates/rbms-table/src/lib.rs:28-29`)를 넘긴다. 같은 곡 열의 묶음 코스가 있으면 `bundled_twin` 으로 묶음 코스를 쓰라고 권한다(8절 결정 7: id 가 하나로 모인다).

### 3.7 코스 id 규칙(서버 등록과 맞추기)

1. **규칙은 바꾸지 않는다**: `Course::hash()`(2.5절 2). 이름은 원천 그대로, 차트는 sha256 이 있으면 sha256, 없으면 md5(소문자), 각 뒤에 `\n`. 이전 없음(마이그레이션 없음).
2. GENOSIDE 2018: 이름은 공식 파일, 차트는 md5 만 → 예: `GENOSIDE 2018 段位認定 初段` + 4 md5 = `ae8ddca1ddec7737a393eddbd3862d2be2e42bb7184365fbc4664efe280ed0d1`(이 문서 작성 중 같은 규칙으로 계산, Rust 테스트가 고정).
3. 표 코스(Stella 등): header 의 이름·md5. header 가 sha256 도 주면 sha256 으로 계산된다(같은 header 면 언제나 같은 id).
4. **채우지 말 것**: 라이브러리에서 찾은 sha256 을 코스 정의의 `sha256` 에 써 넣으면 id 가 바뀐다. 제출 본문의 `charts` 도 정의 그대로 보낸다(지금과 같다, `course_ir.rs:100`).
5. 웹 등록(F6)은 데이터 파일의 `course_hash` 를 쓰거나 같은 규칙을 TS 로 구현하고 위 예를 테스트 벡터로 쓴다(부록 A).
6. 원본(LR2IR 해시 = 접두사+md5, beatoraja 캐시 키 = 차트+제약)과 달리 이름이 들어간다. 8절 결정 8.

### 3.8 클라이언트 표시 위치

1. **COURSE 탭(이 묶음)**: `CourseList::load(dir, library)`(`course_ui.rs:100-104`)가 사용자 파일 앞에 번들 코스를 넣는다: `genoside-2018-sp` 22개 → `genoside-2018-dp` 22개 → 사용자 파일(파일 이름 순). 같은 `hash()` 가 두 번 나오면 먼저 것(번들)만 남긴다. 곡이 없으면 지금처럼 `N STAGES - M MISSING` 으로 보이고 시작할 수 없다.
2. **선곡 트리(F3)**: 이 문서는 함수만 준다. 추천은 발광(insane1) 표 폴더 아래 `GENOSIDE 2018 段位認定` 코스 폴더(SP·DP 하위 폴더)와, Stella·Satellite 표 폴더 아래 그 표의 코스 폴더(`courses_from_table_header`). 8절 결정 6.
3. 코스 막대의 제약 배지는 선언된 제약 그대로(`LR2 GAUGE` 가 보여도 실제 표는 COURSE GAUGE 설정). 실제 규칙은 결과 화면의 GAUGE RULE 행이 보인다. 8절 결정 13.

### 3.9 호환성·마이그레이션 요약

| 대상 | 변경 | 호환 | 마이그레이션 |
| --- | --- | --- | --- |
| `settings.ron` | `judge.course_gauge` 추가 | 없으면 LR2. 옛 빌드는 모르는 필드를 무시(serde 기본) | 없음, 스키마 세대 6 유지 |
| `gauge.ron`(번들·사용자 덮어쓰기) | `"IIDX"` 키, 행의 `guts_edge`(생략 가능) | 옛 파일은 Below 로 읽힘 | 없음, 데이터 세대 1 유지 |
| 점수 DB(`scoredb.sqlite`) | 없음(코스 스테이지는 지금처럼 게이지 번호 6~8) | — | 없음 |
| 코스 제출 와이어 | `gauge`·`course_gauge`·`stages` 추가 | 구 서버는 버림(2.4절 7), 옛 보관 본문은 세 필드 없이 재전송 | 없음 |
| 설정 묶음 와이어 | `course_gauge` 한 줄 | 서버는 blob 을 내용으로만 저장 | 없음 |
| 코스 id | 규칙 그대로 | — | 없음 |
| 서버 DB | (F6) | — | F6 이 정한다 |

---

## 4. 원본 대비 차이 표

"의미" 열: **판정·점수 의미가 달라지는 것**은 굵게.

| id | 항목 | 이 설계 | 원본(beatoraja) / IIDX / LR2 | 의미 | 근거·상태 |
| --- | --- | --- | --- | --- | --- |
| D1 | 게이지 제약 무시 | 코스는 COURSE GAUGE 설정의 표로만 돈다. `gauge_lr2`·`gauge_5k/7k/9k/24k` 는 배지일 뿐 | 제약이 표를 정한다(`GrooveGauge.java:128-147`) | **판정 의미 다름**: BEATORAJA 모드에서 `gauge_lr2` 코스(발광·Stella 전부)가 7키 CLASS 로 돈다 | 사용자 결정 21 |
| D2 | 기본 LR2 | 제약 없는 코스(사용자 파일 등)도 LR2 표 | 제약 없으면 모드 표 | **판정 의미 다름(지금 R-BMS 동작에서도 바뀜)** | 결정 21 |
| D3 | IIDX 표 | DAN/EX-DAN 수치, 30 **이하**에서 절반, 0% 실패, 이월 | IIDX AC: 같은 수치. 내부 표현(정수 단위인지)과 실패선(beginner 는 2% 미만)은 원문이 다르게 말하거나 밝히지 않는다 | **IIDX 와 다를 수 있음**: 내부 단위·반올림, 실패선은 결정대로 0% | 결정 21, 2.7절 8 |
| D4 | 시작 슬롯 | NORMAL 이하=CLASS, HARD=EXCLASS, EXHARD 이상=EXHARDCLASS(IIDX 는 HARD 이상=EXCLASS) | 원본 `GrooveGauge.java:164` | **지금 R-BMS 대비 판정 의미 바뀜**(원본에 맞춰짐): HARD 설정으로 친 코스 스테이지가 EXCLASS(HARD 램프·ASSIST HARD 등급)로 남는다 | 결정 21 "나머지도 추천대로"(조사 추천안) |
| D5 | IIDX 셋째 게이지 | EXHARD 이상도 EX-DAN | IIDX 에 EXHARD DAN 없음 | 설계 선택 | 8절 결정 1 |
| D6 | 9슬롯 이월 | 원본처럼 9슬롯 | 원본 `GrooveGauge.java:150-155` | GAS(BestClear·SelectToUnder) 켠 코스에서 지금 R-BMS 대비 바뀜(원본에 맞춰짐) | 8절 결정 2 |
| D7 | 이월 값 | 판 끝의 정확한 값 | 원본은 게이지 기록 표본의 마지막 값 | 표본 간격 안의 차이(원본이 미세하게 낡은 값) | 8절 결정 3 |
| D8 | LN 제약 적용 | `ln/cn/hcn` 이 그 코스 동안 LN MODE 를 정한다(설정 파일은 그대로) | 원본은 `config.setLnmode` 로 설정 자체를 바꾼다(`MusicSelector.java:501-503`) | **지금 R-BMS 대비 판정 의미 바뀜**(원본에 맞춰짐): CN·HCN 설정 사용자가 발광 코스에서 LN 으로 친다. 설정을 되돌리는 것은 기존 R-BMS 결정(RANDOM 과 같은 취급) | 결정 20 |
| D9 | DP 미러 단위 | 두 사이드 MIRROR, FLIP 없음 | MIRROR 코스에서 `random2 = 1`, `doubleoption = 1`(FLIP) | **판정 의미 다름**(DP 배치) | R-BMS 가 LaneOption 을 적용하지 않는 기존 한계(2.3절 2), 9절 T3 |
| D10 | NO GOOD / NO GREAT | 미적용(배지만) | 판정 폭 비율 0(`JudgeManager.java:175-183`) | **판정 의미 다름**, 대상 코스 0개 | 8절 결정 9, 9절 T1 |
| D11 | 코스 id | 이름+차트 | LR2IR: 접두사+md5, beatoraja 캐시: 차트+제약 | 순위 묶음 단위(이름이 다르면 다른 코스) | 기존, 8절 결정 8 |
| D12 | LR2 표 자체 | beatoraja 의 LR2 이식(30 미만 0.6배, 0% 실패) | LR2oraja 는 32% 시작·2% 실패라고 함(iidx.org misc, 미확인) | 기존 | 바꾸지 않음 |
| D13 | DP 제약·트로피 | SP 같은 단의 값 | 원천 없음 | 설계 선택 | 8절 결정 4 |
| D14 | 연습 패널 | IIDX 표도 고를 수 있음 | 원본 연습은 5세트 | 확장 | 8절 결정 10 |

---

## 5. 구현 단위

모든 단위 공통: Rust 규칙(`//` 주석 금지·`///` 영어만, 매직넘버 금지, 새 `#[allow]`·`unsafe`·`todo!()` 금지, rustfmt 160, `--locked`), 새 외부 크레이트는 C5 의 `zip` 하나뿐. 에이전트는 커밋·푸시하지 않는다. `rbms-player` 를 실행하지 않는다.

| 단위 | 내용 | 소유(수정 가능) 파일 | 읽기 전용 | 선행 | 다른 묶음과 겹치는 파일 | 병렬 |
| --- | --- | --- | --- | --- | --- | --- |
| C1 | 게이지 표: `GutsEdge`, IIDX 세트, `course_ceiling`, `course_gauge_for`, `CourseGaugeMode`, GAS 상한 | `crates/rbms-judge/src/gauge_tables.rs`, `crates/rbms-judge/src/gauge.rs`, `crates/rbms-judge/src/data.rs`, `crates/rbms-judge/data/gauge.ron`, `apps/rbms-player/src/practice.rs`(토큰 표 6칸 `"IIDX"`, 컴파일 유지만) | beatoraja `GaugeProperty.java`·`GrooveGauge.java`, iidx.org 사본 | 없음 | 없음 | C5 와 병렬 |
| C2 | 설정 COURSE GAUGE | `crates/rbms-config/src/judge.rs`, `crates/rbms-config/src/schema.rs`, `crates/rbms-config/src/settings.rs`, `crates/rbms-config/src/lib.rs`, `crates/rbms-config/src/tests.rs`, `apps/rbms-player/src/stage/settings/catalog.rs`, `contract/client-requests/settings-blob.json`(재생성) | `apps/rbms-player/src/app_result/record_tests/contract_requests.rs` | C1 | F2(결정 8 SCRATCH AUTO 색 설정)가 `schema.rs`·`settings.rs`·`settings-blob.json` 을 만진다 → **F2 와 직렬** | C5 와 병렬 |
| C3 | 코스 진행: `course_stage_gauge`, 표 선택, 시작 슬롯, 9슬롯 이월, LN 제약 적용, `CourseOverrides` 정리, 머리 주석, `result_rows` 의 GAUGE RULE | `apps/rbms-player/src/course_ui.rs`, `apps/rbms-player/src/course_ui/tests.rs`, `apps/rbms-player/src/app_play.rs`, `apps/rbms-player/src/lib.rs`(start_course·course_start_gauge·AppShared 의 `course_facts` 자리만), `apps/rbms-player/src/stage/course_result.rs`(116행 인자), `crates/rbms-play/src/session.rs`, `crates/rbms-play/src/lib.rs`, `crates/rbms-course/src/run.rs`, `crates/rbms-course/src/tests.rs`, `apps/rbms-player/src/app_result.rs`(`fold_course_stage` 의 이월 부분), 새 `apps/rbms-player/src/app_result/record_tests/course_gauge.rs` + `record_tests.rs` 의 `mod` 한 줄 | — | C1, C2 | F2(결정 9 코스 Esc → 코스 결과)가 `app_result.rs` 의 `leave_course`·`fold_course_stage`·`record_tests.rs` 를 만진다 → **F2 와 직렬**. F3 가 `lib.rs`(SelectView)를 만진다 → 같은 웨이브면 직렬 | 아니오 |
| C4 | 제출 본문: `CourseGaugeType`·`CourseStagePlay`·세 필드, `ir_course_gauge`, `CourseFacts`, `build_course_submission`, 최상위 lntype, 계약 골든 | `crates/rbms-ir/src/dto/score.rs`, `crates/rbms-ir/src/dto/score/tests.rs`, `crates/rbms-ir/src/mapping.rs`, `crates/rbms-ir/src/http.rs`·`null.rs`·`contract_tests.rs`·`worker.rs`(테스트의 `CourseSubmission` 리터럴만), `apps/rbms-player/src/course_ir.rs`, `apps/rbms-player/src/course_ir/tests.rs`, `apps/rbms-player/src/app_result.rs`(`fold_course_stage` 시그니처·세 호출처·`keep_finished_course`), `apps/rbms-player/src/lib.rs`(`course_facts` 시작·끝), `apps/rbms-player/src/app_result/record_tests/contract_requests.rs`, `contract/client-requests/course-normal.json`, `contract/client-requests/course-scratch-auto.json`, `apps/rbms-player/src/record_sync.rs`(테스트만, 있으면) | `web/src/server/dto/course.ts` | C3 | F6 이 `web/src/server/dto/course.ts` 를 바꾼다(부록 A 를 받는다). F2 와 `app_result.rs` | 아니오 |
| C5 | 코스 데이터: 생성기, 데이터 파일, `bundled`, `table` | `apps/rbms-cli/src/main.rs`, 새 `apps/rbms-cli/src/course_snapshot.rs`(+ `course_snapshot/tests.rs`), `apps/rbms-cli/Cargo.toml`, `Cargo.toml`(`[workspace.dependencies]` 의 `zip` 한 줄), `Cargo.lock`, 새 `web/data/courses/genoside-2018/courses.json`, 새 `crates/rbms-course/src/bundled.rs`, 새 `crates/rbms-course/src/table.rs`, 새 `crates/rbms-course/src/bundled/tests.rs`·`table/tests.rs`, `crates/rbms-course/src/lib.rs` | 조사 산출물(`crs_official`, `mirai_*header.json`, `genocide_courses.json`, `header.json`, `sl_header.json`) | 없음 | F4 가 `Cargo.toml`·`Cargo.lock` 에 같은 `zip` 을 넣는다 → **둘 중 먼저인 쪽이 항목을 만들고 다른 쪽은 쓰기만**. F6 이 `web/data/courses/` 를 읽는다(쓰지 않음) | C1·C2 와 병렬 |
| C6 | COURSE 탭에 번들 코스 | `apps/rbms-player/src/course_ui.rs`(`CourseList::load`), `apps/rbms-player/src/course_ui/tests.rs` | `crates/rbms-course/src/bundled.rs` | C3, C5 | F3 가 선곡 트리에서 같은 함수를 부른다(파일은 다름) | 아니오 |
| C7 | 문서(7절) | 7절 목록 | — | C1~C6 | F2·F3·F6 의 문서와 `play-options.md`·`reference-divergences.md`·`HANDOFF.md`·`PROCESS.md` 공유 → 메인이 합친다 | 아니오 |

C3 에서 `CourseRun` 에 필드를 더하면 `crates/rbms-course/src/tests.rs` 와 앱의 `CourseRun { .. }` 리터럴(있으면)이 영향을 받는다. `CourseRun::new` 를 통해서만 만들면 영향 없음(1169efb 에서 리터럴 생성은 `run.rs:146` 하나).

---

## 6. 테스트 계획

### 6.1 새 테스트(이름과 단언)

**C1 — `crates/rbms-judge/src/gauge_tables.rs` 테스트 모듈**

- `gauge_table_pins_all_54_elements`(기존 `..._45_elements` 를 대체): `PINNED` 에 IIDX 9행을 더해 6 x 9 칸을 모두 비교하고, `guts_edge` 도 비교한다.
- `the_iidx_set_copies_the_seven_key_rows_outside_the_course_gauges`: 슬롯 0~5 에서 `element_of(Iidx, i) == element_of(SevenKeys, i)`.
- `the_iidx_dan_gauges_pin_their_published_values`: Class deltas `[0.16, 0.16, 0.04, -1.5, -2.5, -1.5]`, guts `[(30.0, 0.5)]`, `AtOrBelow`; ExClass `[0.16, 0.16, 0.04, -3.0, -5.0, -3.0]`, guts 없음, `Below`; 두 행 `init 100, min 0, max 100, border 0, modifier None`.
- `only_the_iidx_dan_gauge_softens_at_or_below_its_band`: 54칸 중 `AtOrBelow` 는 `(Iidx, Class)` 하나.
- `guts_appear_exactly_where_the_reference_puts_them`(갱신): 목록 끝에 `(Iidx, Hard)`, `(Iidx, Class)`.
- `lr2_and_iidx_are_never_reached_from_a_mode`(기존 `lr2_is_never_reached_from_a_mode` 확장): `Mode::ALL` 어느 것도 Lr2·Iidx 를 고르지 않는다.
- `bundled_gauge_data_has_exactly_the_expected_keys`(갱신): `["BEAT_10K", "BEAT_14K", "BEAT_5K", "BEAT_7K", "IIDX", "KEYBOARD_24K", "LR2", "POPN_9K"]`(BTreeMap 순서).
- `every_course_gauge_of_every_set_starts_full`: 6세트 x 슬롯 6~8 의 `init == 100.0`.
- `a_course_starts_on_the_course_gauge_the_chosen_gauge_maps_to`: 원본 5세트에서 AssistEasy/Easy/Normal → Class, Hard → ExClass, ExHard/Hazard → ExHardClass; Iidx 에서 Hard/ExHard/Hazard → ExClass.
- `the_iidx_set_tops_out_at_exclass`: `course_ceiling()` Iidx=ExClass, 나머지=ExHardClass.
- `course_gauge_mode_picks_its_table`: Lr2 → 모든 모드 Lr2, Beatoraja → `for_mode`, Iidx → 모든 모드 Iidx.

**C1 — `crates/rbms-judge/src/gauge.rs` 테스트 모듈**

- `an_iidx_dan_gauge_halves_damage_at_thirty_exactly`: Iidx Class 를 30.0 으로 세팅, BAD → 29.25(−0.75), 다시 30.0 에서 POOR → 28.75(−1.25), 빈 POOR → 29.25.
- `an_iidx_dan_gauge_takes_full_damage_just_above_thirty`: 30.5 에서 BAD → 29.0.
- `an_lr2_class_gauge_still_softens_only_below_thirty`: LR2 Class 30.0 에서 BAD → 28.0, 29.0 에서 BAD → 27.8(−1.2). 기존 동작 보존 확인.
- `an_iidx_ex_dan_gauge_never_softens`: Iidx ExClass 10.0 에서 POOR → 5.0.
- `an_iidx_dan_gauge_empties_at_zero_and_stays`: 1.0 에서 POOR → 0.0, 이어 PG → 0.0, `is_cleared() == false`.
- `best_clear_on_an_iidx_course_never_picks_exhardclass`: Iidx 세트, Class 선택, BestClear 로 여러 프레임 → 선택은 Class 또는 ExClass 뿐.
- `select_to_under_on_an_iidx_course_stops_at_exclass`: 설정 ExHard, SelectToUnder → 상한 ExClass.

**C1 — `crates/rbms-judge/src/data.rs` 테스트**

- `a_gauge_file_without_guts_edge_reads_below`: `guts_edge` 없는 RON 한 행 → `Below`.
- `the_bundled_file_states_guts_edge_only_on_the_iidx_dan_row`: 번들 텍스트에서 `guts_edge` 가 정확히 한 번.

**C1 — `apps/rbms-player/src/practice.rs` 테스트(기존 모듈)**

- `the_gauge_set_cycle_reaches_iidx_and_comes_back`: 6번 돌면 처음 표로, 토큰 `"IIDX"` 가 왕복.

**C2 — `crates/rbms-config/src/tests.rs`, `settings.rs` 테스트**

- `the_course_gauge_defaults_to_lr2`, `course_gauge_tokens_round_trip`(3값), `an_unknown_course_gauge_token_reads_lr2`, `a_settings_file_without_a_course_gauge_reads_lr2`(스키마 6 조각), `the_course_gauge_row_cycles_lr2_beatoraja_iidx`(걸음 ±1, 끝에서 돈다), 그리고 행 목록을 단언하는 기존 테스트(`settings.rs` 2236행 부근의 JUDGE 행 목록)에 `CourseGauge` 를 더한다.

**C3 — `apps/rbms-player/src/course_ui/tests.rs`**

- `a_first_stage_starts_on_the_class_gauge_of_the_lr2_table_by_default`: 기본 설정, 7키 → `set == Lr2`, `start == Class`, `value == 100`, `carried == None`.
- `the_course_gauge_setting_outranks_a_gauge_constraint`: 제약 `gauge_7k` 코스, 설정 IIDX → `set == Iidx`.
- `beatoraja_rule_follows_the_chart_mode`: 5키 차트 → FiveKeys, 9키 → Pms.
- `a_hard_player_starts_a_course_on_exclass`, `an_exhard_player_starts_an_iidx_course_on_exclass`.
- `a_later_stage_starts_on_every_gauge_the_stage_before_left`: `carry_gauges = Some([..])` 이면 `value == carried[start]`.
- `the_long_note_constraints_each_force_their_flavour`(기존, `ln_mode_for` 가 이제 비테스트 함수).
- `the_gauge_constraints_each_select_their_table` → **삭제**하고 `the_gauge_constraints_are_shown_but_do_not_pick_the_table` 로 대체(배지는 남고 `CourseOverrides` 에 표가 없다).
- `the_result_rows_name_the_gauge_rule`: GAUGE 행 다음에 `("GAUGE RULE", "IIDX")`.

**C3 — 새 `apps/rbms-player/src/app_result/record_tests/course_gauge.rs`(기존 코스 하네스 사용)**

- `an_ln_course_plays_long_notes_whatever_the_ln_mode_setting`: 설정 CN, `ln` 제약 코스, 미정의 LN 차트 → 세션 `ln_mode == LongNote`, 스테이지 점수 `lntype == 0`, 코스가 끝난 뒤 설정 파일의 `ln_mode == CN`.
- `a_stage_on_iidx_rule_plays_on_the_iidx_table`: 세션 `gauge_set() == Iidx`.
- `the_next_stage_starts_on_every_gauge_the_last_one_left`: GAS BestClear, 1스테이지에서 선택이 ExClass 로 옮겨 끝남 → 2스테이지 시작 시 `value_at(Class)` 와 `value_at(ExClass)` 가 1스테이지 끝 값과 같다.
- `a_hard_player_records_course_stages_on_exclass`: 스테이지 점수 본문 `options.gauge == ExClass`, 저장 행 게이지 번호 7.

**C3 — `crates/rbms-play/src/session.rs` 테스트**

- `a_course_stage_starts_with_every_carried_gauge`: `carried_gauges` 9값이 세션 시작 뒤 그대로.
- `a_seek_keeps_the_carried_gauges`: 분석 모드 seek 뒤에도 같은 9값에서 다시 시작.

**C4 — `crates/rbms-ir/src/dto/score/tests.rs`**

- `course_gauge_types_have_their_wire_names`: `"LR2"`, `"BEATORAJA"`, `"IIDX"` 왕복.
- `a_course_stage_play_round_trips`: `{"gauge":"ExClass","random":"Mirror","lntype":0}`.
- `a_course_body_without_the_gauge_fields_reads_back_and_writes_them_out_again_absent`: 1169efb 모양의 JSON → `gauge == None`, `stages` 비었음, `to_json` 에 세 키가 없다.

**C4 — `apps/rbms-player/src/course_ir/tests.rs`**

- `a_course_submission_states_its_finished_gauge_and_gauge_rule`: 마지막 스테이지 `ExClass` → `gauge == Some(ExClass)`, `course_gauge == Some(Iidx)`.
- `a_course_submission_lists_every_stage_played_in_order`: 3스테이지 중 2에서 실패 → `stages.len() == 2`.
- `a_course_without_an_ln_constraint_reports_the_lntype_its_first_stage_played`: 제약 없음, 첫 스테이지 lntype 1 → 최상위 1.

**C4 — 기록 동기화(`apps/rbms-player/src/record_sync` 테스트 모듈, 기존 코스 재전송 테스트 옆)**

- `a_course_kept_by_an_older_build_is_sent_without_the_gauge_fields`: 세 필드 없는 보관 본문을 보내면 요청 본문에도 세 키가 없다.

**C5 — `crates/rbms-course/src/bundled/tests.rs`**

- `the_bundled_genoside_2018_has_22_sp_and_22_dp_courses`, `every_bundled_course_has_four_lowercase_md5s_and_no_sha256`, `every_bundled_course_hash_is_the_one_its_file_states`(`Course::hash() == course_hash`), `the_first_sp_course_keeps_its_id`(`ae8ddca1…ed0d1` 고정), `every_bundled_course_plays_mirror_lr2_ln`, `bundled_trophies_follow_the_beatoraja_tables`(SP ☆ 10개 3메달, ★ 12개 2메달; DP 같은 단과 같다), `the_bundled_courses_climb_from_shodan_to_overjoy`(묶음마다 첫 이름 `… 初段`, 끝 `… Overjoy`), `the_bundled_source_names_genocide_and_its_file`(maker, `file_sha256 == c426…65ca`, `entry_sha256 == 9d79…ffb7`), `every_bundled_course_validates_unchanged`.

**C5 — `crates/rbms-course/src/table/tests.rs`**(작은 손 픽스처, 외부 파일 복사 없음)

- `a_header_course_of_groups_reads_as_courses_in_order`, `a_flat_course_list_reads_too`, `a_single_course_object_reads_too`, `md5_lists_are_lowercased_and_become_charts`, `a_charts_list_is_read_as_written`, `a_course_with_no_chart_is_skipped_and_counted`, `unknown_constraints_are_dropped`, `a_table_course_keeps_its_trophies`, `a_table_course_is_for_release`, `a_table_course_id_is_its_name_and_md5s`(이름 `Stella Skill Simulator 4th st0` + 4 md5 의 해시를 테스트 안에서 SHA-256 으로 다시 계산해 비교), `same_charts_ignores_name_and_case`, `bundled_twin_finds_the_genoside_course_a_table_lists_under_another_name`(이름 `GENOSIDE 2018 初段` + 같은 md5 → 번들 `GENOSIDE 2018 段位認定 初段`).

**C5 — `apps/rbms-cli/src/course_snapshot/tests.rs`**(합성 `.lr2crs`·header 문자열, 공식 파일은 저장소에 넣지 않는다)

- `lr2crs_courses_are_read_in_file_order`, `a_hash_splits_into_its_prefix_and_four_md5s`, `a_hash_of_the_wrong_length_fails`, `each_line_is_reversed_to_climb_from_shodan`, `an_sp_course_without_a_beatoraja_twin_fails_the_snapshot`, `a_header_with_a_bom_reads`, `the_same_inputs_write_the_same_bytes`, `check_reports_a_difference_with_exit_code_one`.

**C6 — `apps/rbms-player/src/course_ui/tests.rs`**

- `the_course_tab_lists_the_bundled_courses_before_the_players_own`: 빈 폴더 → 44줄, 첫 줄 `GENOSIDE 2018 段位認定 初段`, 23번째 `GENOSIDE 2018 DP段位認定 初段`.
- `a_players_copy_of_a_bundled_course_is_listed_once`.
- `a_bundled_course_whose_charts_are_missing_cannot_start`.

### 6.2 뒤집히는 기존 테스트

| 테스트 | 위치 | 왜 | 처리 |
| --- | --- | --- | --- |
| `gauge_table_pins_all_45_elements` | `gauge_tables.rs` | 세트가 6개 | 54칸으로 이름·내용 교체 |
| `guts_appear_exactly_where_the_reference_puts_them` | 같은 곳 | IIDX 행 2칸 추가 | 목록 갱신 |
| `bundled_gauge_data_has_exactly_the_expected_keys` | 같은 곳 | `"IIDX"` | 목록 갱신 |
| `lr2_is_never_reached_from_a_mode` | 같은 곳 | IIDX 도 | 확장·이름 변경 |
| `set_indices_are_dense_and_ordered` | 같은 곳 | 그대로 통과해야 함 | 변경 없음 |
| `the_gauge_constraints_each_select_their_table` | `course_ui/tests.rs:195` | 제약이 표를 정하지 않음(결정 21) | 삭제·대체 |
| `the_judge_constraints_are_recorded_as_not_yet_enforced` | `course_ui/tests.rs:225` | 그대로 | 변경 없음 |
| 설정 JUDGE 행 목록 단언 | `crates/rbms-config/src/settings.rs` 테스트(2236행 부근) | 행 추가 | 목록 갱신 |
| `a_cleared_course`, `a_cleared_course_with_scratch_auto` | `contract_requests.rs:863-871` | 본문에 세 필드, 기본 LR2 표 | 골든 재생성 |
| 설정 묶음 골든 테스트 | `contract_requests.rs:940` | `course_gauge` | 골든 재생성 |
| `CourseSubmission` 리터럴을 쓰는 테스트 | `crates/rbms-ir/src/http.rs:319`, `null.rs:105`, `dto/score/tests.rs:75,106,344,387`, `contract_tests.rs`, `worker.rs` | 필드 추가 | `..` 가 없으면 세 필드 기본값을 적는다 |
| `CourseOverrides::gauge_set_for` 를 쓰는 테스트 | `course_ui/tests.rs` | 함수 삭제 | 함께 삭제 |

### 6.3 골든·계약 영향

1. `contract/client-requests/course-normal.json`, `course-scratch-auto.json`: `"course_gauge": "LR2"`, `"gauge": "Class"`, `"stages": [두 항목]` 추가. 계약 코스는 제약이 없어 지금은 7키 CLASS 로 돌지만 이제 LR2 CLASS 로 돈다(전부 PGREAT 라 `gauge_value` 100 은 그대로일 것이다 — 재생성 결과로 확인).
2. `contract/client-requests/settings-blob.json`: `course_gauge: "LR2",` 한 줄.
3. 재생성 명령: `RBMS_UPDATE_CONTRACT=1 cargo test -p rbms-player --lib --locked contract_requests` (`docs/development.md:59`). 실행 뒤 `git diff contract/` 에서 **위 세 파일만** 바뀌었는지 본다(다른 파일이 바뀌면 원인을 찾는다).
4. 웹 교차 검사 `cd web && bun test tests/dto/client-requests.test.ts` 와 `tests/db/submission-integrity.test.ts`: 구 DTO 는 새 키를 버려 통과해야 한다. F6 이 DTO 를 바꾸면 그 쪽에서 새 키를 검증한다.
5. 렌더 골든(`crates/rbms-render/tests/golden_*`): 영향 없음(코스 결과의 내장 행은 렌더 골든에 없다 — 구현자가 `grep -rn "GAUGE RULE\|result_rows" crates/rbms-render/tests apps/rbms-player/src/stage/capture` 로 확인).

### 6.4 실행 명령(위험에 비례, 단위별 최소)

| 단위 | 명령 |
| --- | --- |
| C1 | `cargo test -p rbms-judge --locked`, `cargo test -p rbms-player --lib --locked practice` |
| C2 | `cargo test -p rbms-config --locked`, 골든 재생성 명령(설정 묶음), `cargo test -p rbms-player --lib --locked settings` |
| C3 | `cargo test -p rbms-play --locked`, `cargo test -p rbms-course --locked`, `cargo test -p rbms-player --lib --locked course` |
| C4 | `cargo test -p rbms-ir --locked --all-features`, 골든 재생성 명령, `cargo test -p rbms-player --lib --locked course_ir record_sync contract_requests`, `cd web && bun test tests/dto/client-requests.test.ts` |
| C5 | `cargo test -p rbms-course --locked`, `cargo test -p rbms-cli --locked`, 그리고 생성기 실제 실행(조사 산출물 입력, 아래) 뒤 `--check` 가 0 |
| 통합(메인) | `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`, `cargo test --workspace --no-fail-fast --locked`, `git diff --check` |

C5 생성기 실제 실행(데이터 파일을 만드는 한 번):

```
S=/private/tmp/claude-501/-Users-hyunseokbyun-development-R-BMS/93f31835-8882-49ef-96ed-d71e20e979cb/scratchpad
cargo run -p rbms-cli --locked -- course-snapshot --zip $S/crs_official/2018_course.zip \
  --normal-header $S/web/mirai_normal_header.json --insane-header $S/web/mirai_header.json \
  --titles <구현자가 만든 md5→제목 JSON> --retrieved-on 2026-10-23 \
  --out web/data/courses/genoside-2018/courses.json
```

---

## 7. 문서 갱신 목록

| 문서 | 갱신 |
| --- | --- |
| 새 `docs/acknowledge/2026-10-23-adr-course-gauge-mode.md` | 결정 21·20 의 클라이언트 쪽: 세 모드, 설정 우선, IIDX 표와 "이하", 시작 슬롯 대응(지금까지 항상 CLASS 였던 것의 변경), 9슬롯 이월, LN 제약 적용, 8절 결정의 메인 판단(사용자 미확인 표시) |
| 새 `docs/acknowledge/2026-10-23-adr-genoside-2018-course-data.md` | 결정 18: 원천(공식 파일), 제약·트로피 원천(beatoraja 기본 표), DP 값의 선택, 출처 필드, id 규칙 유지, 표시 위치 |
| `docs/acknowledge/README.md` | 위 두 ADR 색인 |
| `docs/play-options.md` | 새 10.14절 "코스 게이지(COURSE GAUGE)"(표·시작 슬롯·이월·모드별 적용·IIDX 수치), 10.13절 4번 정정(2.9절 표의 두 줄), 10.2절 코스 행에 GAUGE RULE·`stages`, 설정 JUDGE 탭 행 표에 COURSE GAUGE, GAUGE SET 은 단일 곡 전용 |
| `docs/acknowledge/reference-divergences.md` | D-D2 행(이제 코스로도 LR2 에 닿는다, 코스는 설정이 정한다), 4절 D1~D14 중 새 것 행 추가, 384행(코스 등급: 게이지를 이제 싣는다, 서버는 F6) |
| `docs/crates.md` | rbms-judge(IIDX 세트·`GutsEdge`·`CourseGaugeMode`), rbms-course(`bundled`·`table`, `GAUGE_SLOTS`), rbms-ir(`CourseGaugeType`·`CourseStagePlay`), rbms-cli(`course-snapshot`) |
| `docs/backend/api-spec.md`·`docs/backend/compatibility.md` | 코스 제출 본문의 세 필드와 구 서버 호환(F6 과 같은 커밋으로 메인이 합친다) |
| `docs/quality-assurance/2026-10-14-test-debt.md` | 3.93 갱신(클라이언트가 게이지를 싣는다), 새 항목: 9절 T1~T6 |
| `docs/quality-assurance/2026-10-11-default-skin-manual-check.md` | 실기: COURSE GAUGE 세 값으로 GENOSIDE 初段 을 쳐서 게이지 감소·결과 GAUGE RULE·스테이지 램프 확인, DP 코스 1개 |
| `docs/HANDOFF.md` | 6절 14번(코스 게이지를 싣는다, 서버 쪽은 F6), 4절 의사결정 요약에 ADR 두 건 |
| `docs/PROCESS.md` | F5 체크 상태 |
| `docs/reference/2026-10-23-stella-course-dan-survey.md` | 2.9절의 정정을 해당 줄에 반영(또는 머리에 정정 절) |
| 코드 문서 | `course_ui.rs` 머리 주석(3.3절 5), `course_ir.rs` 24·67행, `data.rs` `GaugeSet` 문서, `gauge_tables.rs` `GaugeSetId` 문서 |

---

## 8. 메인 결정 필요

| # | 결정 지점 | 추천안 | 근거 | 대안 |
| --- | --- | --- | --- | --- |
| 1 | IIDX 모드에서 EXHARD 이상 설정의 코스 게이지 | **EX-DAN(ExClass 슬롯), 상한 ExClass** — HARD 이상이 모두 EX-DAN, 램프 HARD·등급 3 | IIDX 단위는 DAN·EX-DAN 둘뿐(compendium). 원본 대응식(HARD=EXCLASS)과 IIDX 의 "더 어려운 쪽" 을 같은 자리에 둔다. 같은 수치에 다른 램프가 붙지 않는다 | ExHardClass 슬롯에 EX-DAN 수치를 넣고 EXHARD 설정이면 그 슬롯(램프 EX-HARD, 등급 4): 같은 수치로 HARD/EX-HARD 램프가 갈린다 |
| 2 | 5키·9키·24키에서 IIDX 표 | **수치 그대로 적용** | 원문은 7·14키 값만 준다. LR2 표도 원본에서 모드와 무관하게 적용된다 | 7·14키 외에는 IIDX 를 고를 수 없게(설정이 LR2 로 물러남) |
| 3 | 곡 사이 이월 범위 | **9슬롯 모두**(원본) | `GrooveGauge.java:150-155`. GAS 를 켠 코스에서만 차이가 난다 | 선택 게이지 한 값(지금): GAS 코스에서 원본과 다름, 테스트 부채로 |
| 4 | 이월 값의 출처 | **판 끝의 정확한 값** | 지금 `carry_gauge` 와 같은 선택, 표본 간격의 낡은 값을 흉내내지 않는다 | 원본처럼 게이지 기록 표본의 마지막 값(`PlayRecord.gauge_log.last`) |
| 5 | DP 22개의 제약·트로피 | **같은 단 SP 코스의 값**(제약 `grade_mirror, gauge_lr2, ln`, ☆ 3메달·★ 2메달) | 원천(beatoraja 기본 표)에 DP 코스가 없다. 단위 인정(결정 20)이 미러·LN 제약을 보려면 제약이 있어야 한다 | 제약만 넣고 트로피 없음 / 제약도 없음(단위 판정 기준이 비게 된다) |
| 6 | 차트 제목 | **bms-ir 코스 페이지의 제목**(표시용, id 무관), DP 22쪽은 1초 간격 한 번 받기 | 곡이 없을 때 무엇이 없는지 보여야 한다. robots.txt 가 막지 않는 경로 | 제목 비움(`course 1`…으로 보인다), 라이브러리에 있으면 그 제목 |
| 7 | 묶음 코스 표시 위치 | **COURSE 탭(C6) + 선곡 트리의 발광 표 아래 코스 폴더(F3)** | 지금 UI 로 바로 친다. 트리는 결정 11 의 표 코스 폴더와 같은 자리 | 트리만(COURSE 탭은 사용자 파일만) |
| 8 | 표 코스와 번들 코스의 같은 곡 열 | **번들 코스로 바꿔 같은 id**(`bundled_twin`) | miraiscarlet 표의 GENOSIDE 코스는 이름이 달라 id 가 갈린다(2.7절 5). 순위가 한 곳에 모인다 | 표 이름 그대로 별 id(같은 단위가 두 순위로 갈림) |
| 9 | 코스 id 규칙 | **지금 규칙 유지**(이름 포함, 이전 없음) | 운영 서버의 기존 코스 행·보관 본문의 id 를 그대로 둔다. 8번으로 같은 곡 열 문제를 푼다 | 이름을 뺀 규칙(원본 캐시 키·LR2IR 처럼): 기존 id 전부 이전 필요(클라이언트 DB + 운영 DB 마이그레이션) |
| 10 | NO GOOD / NO GREAT | **이번 범위 아님**(테스트 부채 T1) | 대상 코스(GENOSIDE 44, 표 코스 119) 어느 것도 쓰지 않는다 | 이번에 적용: 판정 폭 비율 0(원본 `JudgeManager.java:175-183`), `is_custom_judge` 는 좁히기를 어시스트로 보지 않아 순위에 영향 없음 |
| 11 | 연습 패널의 IIDX 표 | **고를 수 있게**(6번째 표) | 단위 게이지 연습에 쓸 수 있다. 토큰 표가 어차피 6칸이 된다 | 원본 5세트만(`GaugeSetId::REFERENCE` 를 따로 둔다) |
| 12 | 제출 본문의 스테이지 사실 | **`stages` 배열(스테이지마다 gauge·random·lntype)** | 서버가 단위 인정(결정 20)에서 스테이지마다 확인할 수 있다. 값이 코스 안에서 달라도 뜻이 분명하다 | 코스 전체 한 값(`random` 하나): 스테이지가 다를 때의 뜻을 새로 정해야 한다 |
| 13 | 코스 결과의 GAUGE RULE 행 | **넣는다** | 배지(`LR2 GAUGE`)와 실제 표가 다를 수 있어 결과에 실제 규칙을 보인다 | 넣지 않음 |
| 14 | 코스 막대 배지 | **선언된 제약 그대로** | 코스 정의를 바꾸지 않는다 | 설정과 다른 게이지 배지는 숨김 |
| 15 | 생성기 위치 | **rbms-cli `course-snapshot`(Rust, 오프라인 입력)** | Shift_JIS(`encoding_rs`)·ZIP(승인 `zip`)이 워크스페이스에 있다. 웹 bun 은 Shift_JIS·ZIP 지원이 불확실하다(확인 못함) | `web/scripts/snapshot-courses.ts`(bun) |
| 16 | 웹 순위를 게이지 모드로 나눌지 | **F6 이 정한다**(클라이언트는 `course_gauge` 를 싣기만) | 결정 20·19 는 모드를 제한하지 않는다 | 단위 인정을 LR2 모드로만 |

---

## 9. 남은 위험과 테스트 부채 후보

**위험**

- R1. 시작 슬롯 변경(D4)과 LN 제약 적용(D8)은 지금 사용자 체감이 바뀐다(HARD 설정 사용자의 코스가 EXCLASS 로, CN 설정 사용자의 발광 코스가 LN 으로). 결정 21 "나머지도 추천대로" 로 채택된 조사 추천안이 "이미 있다" 는 틀린 전제였다(2.9절). 메인이 ADR 에 "사용자 미확인" 으로 보고하고 실기 점검표에 넣는다.
- R2. IIDX 수치는 iidx.org 한 곳뿐이고, 내부 단위(정수인지)·반올림은 원문에 없다. f32 합은 30.0 경계에서 미세하게 엇갈릴 수 있다(예: 0.16 의 누적). 경계 테스트는 값을 직접 세팅해 피한다.
- R3. `zip` 9.0.5 는 게시 6일이다. F4 와 같은 고정 버전이어야 하고, 문제가 보이면 같은 메이저의 이전 패치로 함께 내린다.
- R4. 옛 빌드가 보관한 코스 본문(세 필드 없음)은 재전송돼도 서버가 게이지를 모른다 → 등급 0, 단위 판정 불가. 옛 본문을 고쳐 보내는 것은 정보가 없어 불가능하다(스테이지별 게이지·RANDOM 이 본문에 없다).
- R5. 차트 자체의 `#LNMODE`(2=CN, 3=HCN)는 `ln` 제약으로도 바뀌지 않는다(원본도 같다). 그런 차트가 LN 코스에 있으면 스테이지 `lntype` 이 0 이 아니어서 서버가 LN 제약 위반으로 볼 수 있다. GENOSIDE 2018·Stella 차트의 `#LNMODE` 사용 여부는 확인 못함(곡 파일이 없다). F6 에 "lntype 비교는 제약이 정한 값과 차트가 정한 값을 구분" 을 넘긴다.
- R6. 공식 코스 파일의 재배포 조건은 확인 못함(조사 B-3). 데이터에는 md5·이름·제약만 있고 곡 파일은 없다. 운영자 요청 시 지울 수 있게 데이터 파일 하나에 모았다.
- R7. 번들 44코스는 대부분 곡이 없는 사용자에게 COURSE 탭을 길게 만든다(F3 트리 폴더가 생기면 COURSE 탭의 번들 표시를 줄이는 것을 다시 볼 것).
- R8. 같은 웨이브에서 F2(결정 9 코스 Esc → 코스 결과)가 `fold_course_stage`·`leave_course` 를 바꾸면 C3·C4 와 충돌한다. 직렬 순서: F2 → C3 → C4.

**테스트 부채 후보(`docs/quality-assurance/2026-10-14-test-debt.md` 에 추가)**

- T1. NO GOOD / NO GREAT 미적용. 재현: `no_good` 코스에서 GOOD 이 나온다. 필요해지는 때: 그 제약을 쓰는 코스를 들일 때. 방안: 코스 동안 해당 판정 폭 비율 0.
- T2. IIDX 의 내부 표현(정수 게이지 단위)·표시(2% 칸) 미모형. 필요해지는 때: IIDX 실기와 수치 비교 보고가 올 때.
- T3. DP 미러 단위에서 FLIP 미적용(원본은 MIRROR 코스에서 FLIP). 필요해지는 때: LaneOption 을 플레이에 적용할 때 함께.
- T4. 코스 스테이지 리플레이는 코스 게이지 표·이월 9슬롯·콤보를 담지 않는다(기존 3.94 의 확장). 재생 게이지가 실제와 다르다.
- T5. 연습 패널에서 IIDX 표의 ExHardClass 를 고르면 EX-DAN 수치에 EX-HARD 이름(8절 결정 1 의 부산물).
- T6. 번들 데이터의 DP 차트 제목·md5 를 bms-ir DP 페이지와 하나씩 대조하지 않았다(조사도 해시 목록 대조만). 생성기가 공식 파일의 md5 를 그대로 쓰므로 md5 는 공식과 같다. 제목만 표시용으로 남는 위험.

---

## 부록 A. 웹 묶음(F6)에 넘기는 계약

1. 코스 제출 본문 추가 키(모두 생략 가능, 옛 클라이언트는 보내지 않는다):
   - `gauge`: `"Class" | "ExClass" | "ExHardClass"`(다른 `GaugeType` 이름도 형식상 올 수 있으나 클라이언트는 코스 게이지만 보낸다). 서버의 `deriveAssistGrade({ clear, gauge: gaugeTypeToId(gauge) })` 로 `course_best.assist_grade` 를 채울 수 있다.
   - `course_gauge`: `"LR2" | "BEATORAJA" | "IIDX"`.
   - `stages`: 최대 `COURSE_CHARTS_MAX`(64) 개, 각 `{ gauge: GaugeType, random: RandomOption, lntype: int }`. 순서는 친 순서, 길이는 친 스테이지 수(실패 스테이지 포함, 치지 않은 스테이지 제외).
   - 최상위 `lntype` 은 LN 제약이 있으면 그 값, 없으면 첫 스테이지의 값으로 바뀐다(뜻이 사실과 같아진다).
2. 코스 id 규칙과 테스트 벡터: 3.7절. `GENOSIDE 2018 段位認定 初段` + `ae11930223a8f24e91633f1a7864e1eb`, `cb661ee05c67990012bfd12703ba656c`, `328dbf235f3b6cb2791e71bfb62951f8`, `7c6ca04aa14d07b2c081f4dd35432844` → `ae8ddca1ddec7737a393eddbd3862d2be2e42bb7184365fbc4664efe280ed0d1`.
3. 코스 정의 원천: `web/data/courses/genoside-2018/courses.json`(3.5절 모양). 출처 필드는 `source` 그대로 서버에 저장하면 결정 14 의 "Stella·단위는 출처 표기" 를 채운다.
4. 단위 인정(결정 20)에 클라이언트가 보장하는 것: grade_mirror 코스는 RANDOM 이 클램프돼 `Off` 또는 `Mirror` 만 나온다(`RandomLock::Mirror`). `ln` 코스는 LN MODE 가 LN 으로 강제된다(차트 자체의 `#LNMODE` 는 예외, 9절 R5). 어시스트는 기존 `scratch_auto`·`assist`.

## 부록 B. 이 문서 작성 중 실행한 것

읽기만 했다: 코드·문서 열람, 조사 산출물의 Shift_JIS 해석과 대조(Python `-I`), crates.io API 조회(zip 9.0.5, typed-path, flate2 의존). 코드 변경·빌드·테스트·플레이어 실행·git 상태 변경은 하지 않았다.
