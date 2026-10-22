# 클리어 램프 조사 (요청 3: ASSIST 램프 세분화)

> 최종 갱신 2026-10-22 · 대응 커밋 47a7f9c

조사로 시작해 구현을 따라 고친 살아 있는 문서다. 0절(결론), 1~4절 중 현재 상태를 말하는 부분, 8절 결정은 구현 뒤의 코드 기준이고, 5절(원본·LR2·IIDX)과 6절(설계 대안 비교)은 조사 당시 그대로다. 문서와 코드가 다르면 코드를 믿었다. 근거는 저장소 루트 기준 `파일:줄` 이다. 결정은 사용자 지시 "끝까지 다 구현해놔" 에 따라 메인이 추천안(6.4절 안 c)으로 정했고 사용자는 아직 확인하지 않았다. ADR [어시스트 램프 등급](../acknowledge/2026-10-22-adr-assist-lamp-grade.md).

구현(2026-10-22): `4d57c2a`(클라이언트 등급, 저장, 병합, 소급, 표시), `8d85b32`(웹 마이그레이션 0011, 운영 적용 web-db run 39693988962), `64c6120`(웹 등급 표시·순위·AI), `0c450db`(기본 스킨 등급 표시), `242397f`(코스 스테이지를 실제로 끝난 CLASS 계열 게이지로 저장·전송), `47a7f9c`(웹 AI 기록 질문 정렬·비교 합계, 등급 백필 액션). 운영 확인: 낡은 등급 행 0(`assist-grade-backfill-count`, run 39733088817), 등급이 매겨진(0 보다 큰) `chart_best` 행 398.

조사 때 읽은 것: `docs/HANDOFF.md`, `docs/play-options.md`(10절 전체), `docs/acknowledge` 의 SCRATCH AUTO·TOTAL·웹 클리어 ADR, `docs/reference/ai-features`, `docs/reference/skin-compat/README.md`, beatoraja 소스(`/Users/hyunseokbyun/development/beatoraja`), ModernChic 스킨(읽기만, 내용을 복사하지 않았다). 읽지 않은 것: `.env*`, 실제 HOME 의 `settings.ron`·`scoredb.sqlite`, 운영 DB 의 행 내용. GUI 는 실행하지 않았다.

---

## 0. 결론 (먼저 읽을 것)

1. 램프는 beatoraja `ClearType` 과 같은 11값 선형 id 하나다(0~10). 게이지 종류가 램프를 정하고(6종 선택 게이지, 9종 계산 게이지), 풀콤 조건이 되면 8~10 으로 덮고, 어시스트가 있으면 게이지·풀콤과 무관하게 3 또는 2 로 강등한다(1.3, 1.4절). 구현은 이 체계를 바꾸지 않았다.
2. SCRATCH AUTO 를 켜고 HARD 게이지로 클리어하면 램프는 여전히 LIGHT ASSIST(id 3, 와이어 `LightAssistEasy`)다. 풀콤이어도 같다. 이제 그 위에 어시스트 등급 ASSIST HARD(3)가 따라붙는다. 등급은 램프 id·와이어 이름·강등 규칙을 바꾸지 않는 파생 값이다(1.8절).
3. 등급의 재료는 이미 저장돼 있었다(`scorelog.gauge`·`assist`, 서버 `score.gauge`). 클라이언트 `score` 에 열 `assist_grade` 를, 서버 `chart_best`·`course_best` 에 열 `assist_grade` 를 더했고(마이그레이션 0011), 기존 기록은 클라이언트가 DB 를 열 때 `scorelog` 에서 다시 접고(`refold_assist_grades`) 서버가 마이그레이션의 백필 UPDATE 로 소급했다.
4. 램프 id 는 단순한 이름이 아니라 정렬 키다. 클라이언트는 최고 램프를 `(clear, 등급)` 쌍으로 접고(`RecordLamp`, `score.rs:132`), 웹은 순위를 `(clear, assist_grade, ex_score, minbp)` 순으로 매긴다(`best-scope.ts:26-31`). 등급은 id 3 안에서만 갈리므로 어시스트 판은 EASY(4) 이상의 정상 판을 넘지 못한다.
5. 와이어는 문자열 enum 이고 양쪽이 모르는 이름을 거절한다. 새 램프 이름을 와이어에 싣는 안은 이 위험을 지므로 쓰지 않았다. 대신 서버 응답에 필드 `assist_grade`(0~4, 기본 0)를 additive 로 실었다. 제출 본문은 바뀌지 않았다(3.1, 3.7절).
6. 외부 Lua 스킨은 11칸을 전제로 한다(4.3절). 외부 스킨에는 계속 0~10(어시스트 클리어는 id 3)만 주고, 기본 스킨만 확장 불린 21110~21134 로 등급을 읽는다(4.2절).
7. 이름은 하나로 통일했다: id 2 ASSIST, id 3 LIGHT ASSIST, 등급 1~4 는 ASSIST EASY / ASSIST CLEAR / ASSIST HARD / ASSIST EXHARD. 이름 `ASSIST EASY` 는 EASY 게이지 클리어를 가리키고, ASSIST EASY 게이지로 클리어한 SCRATCH AUTO 판은 등급 없음(LIGHT ASSIST)이다(1.8절).
8. 남은 것: `course_best.assist_grade` 는 항상 0 이다(코스는 게이지를 저장하지 않는다). 사용자 확인 대기 항목은 8절 각 결정의 "결정됨" 줄과 10절.

---

## 1. 클라이언트의 램프 정의와 결정

### 1.1 램프 값과 이름 (한 표로)

정의: `crates/rbms-judge/src/gauge.rs:107-119`(`ClearType`), id 변환 `gauge.rs:584-616`(`clear_type_id`, `clear_type_from_id`, 모르는 id 는 NoPlay). id 는 단조 증가로 고정돼 있고 테스트가 지킨다(`gauge.rs:655-680` 의 `clear_type_ids_are_the_reference_values`, `clear_type_ids_are_strictly_monotonic`).

| id | Rust 변형 | 와이어 문자열 | 앱 화면 라벨·색(`format.rs:116-129`) | 기본 스킨 이름·색(`theme.lua:139-169`) | 웹 라벨·색(`clear-lamp.ts:21-47`, `globals.css:57-67`) | 스킨 옵션 id | beatoraja gaugetype |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 0 | NoPlay | NoPlay | NO PLAY (64,64,64) | NO PLAY, `lamp_off` (68,68,68), 두 번째 테마 (58,69,86) | NO PLAY #444444 | 100 | 없음 |
| 1 | Failed | Failed | FAILED (0,0,128) | FAILED (255,0,0) | FAILED #ff0000 | 101 | 없음 |
| 2 | AssistEasy | AssistEasy | ASSIST (128,0,128) | ASSIST (255,99,253) | ASSIST #ff63fd | 1100 | 없음 |
| 3 | LightAssistEasy | LightAssistEasy | LIGHT ASSIST (255,0,255), 등급 있으면 ASSIST EASY~EXHARD | LIGHT ASSIST (101,99,255), 등급 있으면 ASSIST EASY~EXHARD | LIGHT ASSIST #6563ff, 등급 있으면 ASSIST EASY~EXHARD | 1101 | 0 (ASSIST EASY 게이지) |
| 4 | Easy | Easy | EASY (64,255,64) | EASY CLEAR (0,255,36) | EASY #00ff24 | 102 | 1 |
| 5 | Normal | Normal | CLEAR (0,192,240) | CLEAR (255,187,0) | CLEAR #ffbb00 | 103 | 2, 6 |
| 6 | Hard | Hard | HARD (255,255,255) | HARD CLEAR (223,223,223) | HARD #dfdfdf | 104 | 3, 7 |
| 7 | ExHard | ExHard | EX-HARD (136,255,255) | EX HARD CLEAR (255,110,110) | EX HARD #ff6e6e | 1102 | 4, 8 |
| 8 | FullCombo | FullCombo | FULL COMBO (255,255,136) | FULL COMBO (0,223,255) | FULL COMBO #00dfff | 105 | 5 |
| 9 | Perfect | Perfect | PERFECT (136,136,255) | PERFECT (244,153,242) | PERFECT #f499f2 | 1103 | 없음 |
| 10 | Max | Max | MAX (0,0,255) | MAX (234,243,0) | MAX #eaf300 | 1104 | 없음 |

- 이름은 하나로 통일했다(2026-10-22, 조사 때는 앱 `format.rs`, 기본 스킨 `theme.lua`, 웹 `clear-lamp.ts` 가 id 2·3·6·7 에서 서로 달랐다). 앱 `format.rs:114-166`, 기본 스킨 `theme.lua:163-186`, 웹 `clear-lamp.ts`(`lampLabel`)가 같은 이름을 쓴다. id 6·7 의 표기(앱 `HARD`/`EX-HARD`, 스킨 `HARD CLEAR`/`EX HARD CLEAR`, 웹 `HARD`/`EX HARD`)는 그대로 다르다. 앱 설정 설명 문구 `stage/settings/catalog.rs:147,169` 도 `the ASSIST lamp at best` 로 고쳤다(7절 1).
- 색은 둘이다. 앱 화면은 beatoraja 공식 팔레트(`format.rs:114` 주석), 기본 스킨과 웹은 같은 자체 팔레트다(`theme.lua:139-150` 값이 `globals.css:57-67` 과 같다). 등급 네 색은 셋이 같은 값이다(1.8절).
- 선택 가능한 게이지의 클리어 판정: 그루브 계열(ASSIST EASY, EASY, NORMAL)은 끝 값이 border 이상이어야 한다(7K 기준 border 60, 80, 80, `gauge.ron:271,280,289`). 서바이벌 계열(HARD, EXHARD, HAZARD)은 border 0 이라 값이 0 보다 크면 산다(`gauge.ron:298` 이하). 공통 판정은 `Gauge::is_cleared` = `value > 0 && value >= border`(`gauge.rs:316-318`).

### 1.2 게이지 → 램프 (`lamp_for_gauge`, `gauge.rs:146-155`)

| 게이지 | 선택 가능(`GaugeKind`, `gauge.rs:9`) | 인덱스(`GaugeIndex`, `gauge.rs:23`, DB·웹 `gauge` id 와 같다) | 클리어 램프 |
| --- | --- | --- | --- |
| ASSIST EASY | 예 | 0 | LightAssistEasy (3) |
| EASY | 예 | 1 | Easy (4) |
| NORMAL | 예 | 2 | Normal (5) |
| HARD | 예 | 3 | Hard (6) |
| EX-HARD | 예 | 4 | ExHard (7) |
| HAZARD | 예 | 5 | FullCombo (8) (한 번이라도 BAD·POOR 면 끝나므로 클리어는 풀콤) |
| CLASS | 아니오 | 6 | Normal (5) |
| EXCLASS | 아니오 | 7 | Hard (6) |
| EXHARDCLASS | 아니오 | 8 | ExHard (7) |

- 아홉 게이지는 모든 판정에 동시에 움직이고, 선택된 하나가 클리어를 정한다(`gauge.rs:393`, `GrooveGauge::update_with_rate` 가 아홉 개를 모두 갱신). 선택 게이지가 판 도중 바뀔 수 있다(GAUGE AUTO SHIFT, `gauge.rs:500-560`). 기록하는 게이지는 끝난 게이지다(`session.rs:863` `finished_gauge`, 앱은 `app_result.rs:240` 의 `record.finished_gauge` 를 쓴다. 코스 스테이지는 CLASS 계열 게이지로 달리고 그 값 그대로 기록한다).
- 게이지 이름 `ASSIST EASY`(0번 게이지)와 어시스트 등급 이름 `ASSIST EASY`(1번 등급)는 다른 것이다. ASSIST EASY 게이지로 어시스트 없이 클리어하면 램프는 id 3(LIGHT ASSIST)이고, ASSIST EASY 게이지로 SCRATCH AUTO 클리어를 해도 등급은 없다. 등급 ASSIST EASY 는 EASY 게이지로 SCRATCH AUTO 클리어한 판이다. 램프 id 2 의 라벨은 이제 `ASSIST` 다.

### 1.3 램프가 정해지는 순서 (한 판)

1. `clear_lamp(gauge, counts, combo, pass_notes, total_notes)`(`gauge.rs:563-582`, 엔진 쪽 호출 `matcher.rs:790`)
   - 노트가 0 이면 NoPlay
   - 선택 게이지가 클리어가 아니면 Failed
   - `pass_notes > 0` 이고 끝에 남은 콤보 `combo == pass_notes` 이면 풀콤 계열: GREAT 와 GOOD 가 0 이면 Max, GOOD 가 0 이면 Perfect, 아니면 FullCombo. (`counts` 인덱스는 PG, GR, GD, BD, PR, MS 이므로 코드의 `counts[1]`·`counts[2]` 는 GREAT, GOOD 다.)
   - 아니면 선택 게이지의 램프(`lamp_for_gauge`)
2. 도중 포기·종료한 판은 요약에서 램프가 Failed 로 고정된다(`session.rs:585` `left_unfinished`).
3. 어시스트 강등 `assisted_lamp(lamp, assist)`(`apps/rbms-player/src/lib.rs:375-382` → `gauge.rs:160-171` `assist_downgrade`)
   - 램프가 NoPlay 또는 Failed 면 그대로
   - 어시스트 0 이면 그대로
   - 어시스트 1(`LIGHT_ASSIST`)이면 LightAssistEasy (3)
   - 어시스트 2 이상이면 AssistEasy (2)
4. 호출 위치 `app_result.rs:232-240`: `custom_judge = judged_wider(...)`, `scratch_auto = scratch_auto_played(play)`(`app_result.rs:653`, 실제로 턴테이블 레인에서 노트가 빠졌을 때만 참), `assist = assist_level(scratch_auto, custom_judge)`(`mapping.rs:111-120`: 넓힌 판정이 있으면 2, 아니면 SCRATCH AUTO 가 1, 아니면 0), `lamp = assisted_lamp(summary.clear_lamp, assist)`.
5. 이 `lamp` 가 `scorelog.clear`(`scoredb_store.rs:197`), 제출 본문 `clear`(`app_result.rs:251`, `mapping.rs:29-43` `ir_clear`), 결과 화면 라벨에 같은 값으로 간다. 최고 기록 병합은 1.7절.

### 1.4 무엇이 어떤 램프를 만드는가

| 상황 | 어시스트 단계 | 클리어한 판의 램프 | 최고 기록(EX·판정·콤보·BP) | 서버 ranked |
| --- | --- | --- | --- | --- |
| 어시스트 없음, 풀콤 아님 | 0 | 게이지 램프: ASSIST EASY 3, EASY 4, NORMAL 5, HARD 6, EX-HARD 7, HAZARD 8 | 갱신 | ranked |
| 어시스트 없음, 끝 콤보가 지나간 노트 수와 같음 | 0 | MAX 10 (GREAT·GOOD 0), PERFECT 9 (GOOD 0), FULL COMBO 8 | 갱신 | ranked |
| SCRATCH AUTO, 그 판의 턴테이블 레인에 노트가 있음 | 1 | LIGHT ASSIST 3 (게이지·풀콤 무관), 등급은 끝난 게이지로(1.8절) | 갱신(규칙 세대 3 이상, `sets_best` `score.rs:90-96`) | ranked, SCRATCH AUTO 최고 기록 칸(`ranked-policy.ts:27,61-72`) |
| SCRATCH AUTO 인데 턴테이블 노트가 없는 차트·턴테이블 없는 모드(9키) | 0 | 어시스트 없음과 같다 | 갱신 | ranked |
| JUDGE WIDTH 또는 LN MARGIN 이 100% 초과(`judge_setup.rs:19-22`: 키 3개·스크래치 3개, 모두 여섯 판정 폭 중 하나라도, 또는 LN 여유) | 2 | ASSIST EASY 2 | 갱신 안 함(`updates_score` `lib.rs:345-347`) | unranked(`ASSIST` 표지) |
| TOTAL 을 직접 지정(`total_override > 0`, `lib.rs:358,367`) | 2 | ASSIST EASY 2 | 갱신 안 함 | unranked(`TOTAL_OVERRIDE`) |
| 위 둘과 SCRATCH AUTO 의 조합 | 2 (큰 쪽) | ASSIST EASY 2. SCRATCH AUTO 라는 사실은 단계에서 사라진다 | 갱신 안 함 | unranked |
| LEGACY NOTE (`schema.rs:557`) | 0 | 영향 없음. 그리기만 바꾼다("판정은 그대로", `playfield.rs:26-28`) | - | - |
| LANE OPTION FLIP·BATTLE·BATTLE AUTO-SC | 0 | 영향 없음. 설정 값은 있으나 판에 적용되지 않고 메뉴에서도 숨긴다(`settings.rs:1681-1682` 테스트) | - | - |
| RANDOM 의 H-RANDOM, ALL-SCRATCH 등 | 0 | 영향 없음. 원본은 이 옵션들을 LIGHT_ASSIST 로 센다(5절). rbms 는 세지 않는다 | - | - |
| GAUGE AUTO SHIFT 로 게이지가 바뀐 판 | 위와 같다 | 끝난 게이지 기준 | - | - |
| AUTOPLAY | 0 | 결과 화면에는 정상 램프(완주면 MAX)가 뜨지만 저장·전송하지 않는다(`ir_submission_block_reason` `lib.rs:330-339`, `contract/client-requests/score-autoplay.json` 은 `Max`) | 안 함 | 보내지 않음 |
| 리플레이 감상 | 0 | 기록하지 않는다 | 안 함 | 보내지 않음 |
| 연습 구간 | - | 결과 화면 없이 돌아간다(`app_result.rs:160`) | 안 함 | 보내지 않음 |
| Esc·앱 종료 | 위와 같다 | Failed, 단 노트를 하나도 치지 않았으면 기록 자체가 없다 | EX 가 더 높을 때만 | 보냄 |

- 어시스트 단계는 와이어에서 `options.scratch_auto`(bool)와 `options.assist`(태그 배열)로 나뉜다. 태그는 `AUTO_SCRATCH`(`mapping.rs:97`), `CUSTOM_JUDGE`(`mapping.rs:100`)이고 TOTAL 지정도 `CUSTOM_JUDGE` 로 간다(`assist_flags` `mapping.rs:125-134`).
- 제출되는 `options.judge_rate` 는 키 레인 PGREAT 폭 하나뿐이다(`app_result.rs:43,293`). 다른 폭만 넓혀도 `assist` 태그가 있어 서버는 `ASSIST` 표지로 unranked 로 만든다(`ranked-policy.ts:33-36,61-72`).

### 1.5 확정: SCRATCH AUTO + HARD 게이지 클리어

경로: 게이지 HARD(`finished_gauge`)가 클리어 → `clear_lamp()` 는 Hard(풀콤이면 8~10) → `assist_level(true, false) = 1` → `assist_downgrade(lamp, Light) = LightAssistEasy` → `scorelog.clear = 3`, `gauge = 3`(HARD), `assist = 1`, 제출 `clear: "LightAssistEasy"`, `options.gauge: "Hard"`, `options.scratch_auto: true`, `options.assist: ["AUTO_SCRATCH"]`. 이 판의 어시스트 등급은 ASSIST HARD(3)다(`assist_grade(LightAssistEasy, Hard)`, 새 골든 `contract/client-requests/score-scratch-auto-hard.json`).

테스트 근거(실행해서 통과 확인): `main_tests.rs:317-328` 의 `assisted_lamp(Hard, LIGHT_ASSIST) == LightAssistEasy` 와 `an_assisted_run_is_demoted_and_cannot_reach_a_full_combo_lamp`(`main_tests.rs:332-352`: 어시스트 1 에서 Easy~Max 전부 LightAssistEasy, 어시스트 2 에서 전부 AssistEasy, Failed·NoPlay 는 그대로). 엔진 쪽 `gauge.rs:747-753` 의 `assist_downgrade_replaces_every_cleared_lamp`. 계약 골든 `contract/client-requests/score-scratch-auto.json` 도 `clear: LightAssistEasy` 이다(그 골든의 `options.gauge` 는 `Normal`).

### 1.6 코스 램프

- 진행 중 접기: `PlaySession::course_lamp(notes_before, counts_before)`(`session.rs:933-938`)가 지금까지의 판정 합계와 코스의 최장 연속 콤보로 `clear_lamp` 를 다시 부르고, 앱이 어시스트로 강등한다(`app_result.rs:774-778`). 코스 어시스트는 스테이지들의 OR 이다(`course_ir.rs:31-46` `CourseAssist`).
- 결과 스킨 쪽 접기: `skin_host/result/course.rs:164-181` — 실패한 스테이지가 있으면 Failed, 이미 AssistEasy·LightAssistEasy 면 그대로, 코스 최장 콤보가 코스 노트 전부면 (GOOD, GREAT) 로 Max·Perfect·FullCombo, 아니면 마지막 스테이지의 끝난 게이지의 램프.
- 코스 제출 본문의 `clear` 는 `clear_type_from_id(run.clear)` 의 `ir_clear`(`course_ir.rs:79`). 서버는 코스 램프를 검사하지 않는다(0.1.0 이 마지막 스테이지 램프를 보내서 꺼 둠, `course.service.ts:143-144`, `HANDOFF.md:89` 3번).
- 코스 게이지는 CLASS 계열 셋(`GaugeIndex` 의 6~8번, `gauge.rs:23-33`)이고 선택 가능한 게이지 목록(`GaugeKind`)에는 없다. 조사 때는 선택 게이지가 코스 게이지이면 `finished_gauge` 가 `None` 이라 기록하는 게이지가 설정 게이지로 대체됐고(`reported_gauge`), 그 탓에 코스 스테이지의 SCRATCH AUTO 클리어가 설정 게이지의 등급으로 저장·전송됐다(리뷰 lamps-1). 2026-10-22 수리(`242397f`)로 `reported_gauge` 를 없애고 `record.finished_gauge`(`GaugeIndex`)를 저장(`scoredb_store.rs:220`)하고 `rbms_ir::mapping::ir_gauge_of_index`(`mapping.rs:62`)로 제출한다. 코스 스테이지는 CLASS(6), EXCLASS(7), EXHARDCLASS(8)로 기록되고 등급은 CLASS=ASSIST CLEAR, EXCLASS=ASSIST HARD, EXHARDCLASS=ASSIST EXHARD 다. 이미 저장된 행은 DB 를 열 때 `put_course_stages_on_class`(`scoredb/courses.rs:126`, 호출 `scoredb_store.rs:117`)가 남아 있는 코스의 스테이지 행(gauge < CLASS)을 CLASS 로 옮기고 등급을 다시 접는다. 코스를 지운 뒤의 스테이지 행은 식별할 수 없어 그대로다.

### 1.7 최고 램프의 병합과 규칙 세대

- 한 차트의 `score` 행(차트×모드×LN 종류): 램프를 `(clear, assist_grade)` 쌍(`RecordLamp`, `score.rs:132`)으로 비교해 엄격히 높아질 때만 `clear`, `assist_grade`, `option`, `seed`, `gauge`, `assist` 를 그 판의 값으로 바꾼다(`merge_best`, `scoredb.rs:653-`). 같은 id 3 안에서 등급이 오른 판도 gauge·assist 를 따라간다. `lamp_counts = updates_best || !is_stale_rule_version(...)`(`scoredb.rs:654`)라서 최고 기록을 못 세우는 판(넓힌 판정)도 규칙 세대 2 이상이면 램프는 올린다. 클리어 횟수는 `clear > 1` 인 판마다 +1.
- 메모리 기록 책(`ScoreBook`): 선곡의 막대 램프는 `best_lamp_for_md5`(`score.rs:392`)의 clear, 정렬·상세는 `(clear, 등급)` 쌍이다. `best_clear_for_md5` 는 시그니처(u8)가 그대로이고 그 쌍의 clear 를 돌려준다. 어시스트이면서 규칙 세대 2 미만인 행은 제외한다(그 세대는 강등 전 램프를 저장했다, `reference-divergences.md` D-D9).
- SQL 쪽 접기: `ScoreDb::best` 는 `ORDER BY clear DESC, assist_grade DESC, EX DESC, minbp ASC`(`scoredb.rs:482`), `chart_best_many` 는 `MAX(clear * 5 + assist_grade)` 로 접고 clear 와 등급으로 나눈다(`scoredb.rs:500-511`).
- 규칙 세대 `SCORE_RULE_VERSION = 4`(`score.rs:34`). 3 은 SCRATCH AUTO 가 턴테이블을 판정에서 뺀 세대(`score.rs:74`), 4 는 원본 기록 정의(`score.rs:64`). `sets_best(assist, rule_version)` 가 최고 기록 자격을 정한다(`score.rs:90-96`: 어시스트 0 이면 참, 1 이면 세대 3 이상, 그 외 거짓). 제출 본문에도 `extra.rule_version` 으로 간다(`app_result.rs:317`). 서버는 이 값을 AI 분석의 "기준 규칙" 필터(`>= 4`)에만 쓴다(`score-feature.constants.ts:50`, `judge-habit.service.ts:129`, `weekly-summary.service.ts:175`).

### 1.8 어시스트 등급 (구현, 2026-10-22)

램프 id(3), 와이어 이름, 강등 규칙, `SCORE_RULE_VERSION`(4), 제출 본문은 그대로다. 등급은 램프 3(`LightAssistEasy`) 안에서만 갈린다.

| 등급 id | `AssistGrade` | 이름 | 되는 판 (강등 전 게이지가 주는 램프) | 앱 색 / 기본 스킨·웹 색 |
| --- | --- | --- | --- | --- |
| 0 | `None` | LIGHT ASSIST | 램프가 3 이 아닌 모든 판, ASSIST EASY 게이지의 클리어, 게이지를 모르는 판, 규칙 세대 2 미만 행, `assist = -1` 행 | (255,0,255) / `#6563ff` |
| 1 | `Easy` | ASSIST EASY | EASY | `#3ad19f` |
| 2 | `Clear` | ASSIST CLEAR | NORMAL, CLASS | `#c4a796` |
| 3 | `Hard` | ASSIST HARD | HARD, EXCLASS | `#afb8ef` |
| 4 | `ExHard` | ASSIST EXHARD | EX-HARD, HAZARD, EXHARDCLASS | `#ca76a9` |

- 정의(엔진): `rbms_judge::AssistGrade`(`gauge.rs:180-`, `COUNT` 5, `id()`, `from_id()`)와 `rbms_judge::assist_grade(lamp, finished_gauge)`(`gauge.rs:229`). 램프가 `LightAssistEasy` 이고 게이지를 알 때만 `lamp_for_gauge` 로 접는다: Easy→Easy, Normal→Clear, Hard→Hard, ExHard·FullCombo(HAZARD)·Perfect·Max→ExHard, ASSIST EASY 게이지→None. 풀콤 계열이 섞여 보이는 것은 HAZARD 를 EXHARD 로 접으려는 것이다(어시스트 판은 풀콤 계열이 못 된다).
- 정의(저장): `rbms-store` 는 `rbms-judge` 에 의존하지 않아(`scoredb.rs` 의 `CLEAR_FAILED_ID` 문서) 정수 거울을 따로 둔다. `ASSIST_GRADE_OF_GAUGE: [u8; 9]`(`score.rs:103-113`)와 `assist_grade_of(clear, gauge, assist, rule_version)`(`score.rs:122`)는 clear == 3, assist != -1(`ASSIST_UNRECORDED`), 규칙 세대 2 이상일 때만 표 값을 준다. 엔진 함수와 같은 답인지는 `format::tests::the_score_store_grades_every_stored_run_as_the_engine_does`(`format.rs:426`)가 램프 11 × 게이지 9 전부로 확인한다.
- 열: 클라이언트 `score.assist_grade INTEGER NOT NULL DEFAULT 0`(`schema.sql:40`, 기존 이관 방식 `ADDED_COLUMNS` `schema.rs:68`, `SCORE_DB_SCHEMA_VERSION` 1 그대로). `scorelog` 에는 열이 없다(`clear`, `gauge`, `assist`, `rule_version` 에서 파생). `ScoreRecord.assist: i32` 를 serde default(-1)로 더했다(`score.rs:199`, `record_of` 가 scorelog 값으로 채움, 옛 `scores.ron` 이관은 기록된 assist 가 있으면 그 값, 없으면 예전 규칙 assisted → -1, 아니면 0).
- 병합과 정렬: 1.7절.
- 소급: `refold_assist_grades`(`scoredb.rs:774-`)가 `schema::migrate`(`schema.rs:50`)에서 `refold_reference_bests` 다음에 DB 를 열 때마다 돈다. 각 `score` 행의 clear 가 3 이면 같은 램프의 `scorelog` 행 가운데 최고 등급을, 동률이면 id 가 가장 작은 첫 판을 고르고, 등급이 다를 때만 쓰며 등급이 0 보다 크면 그 판의 gauge·assist 도 쓴다. clear != 3 인 행은 등급 0 으로 정리한다. 내부 스캔은 `WHERE l.clear = ?1`(LIGHT ASSIST)로 좁혀 있다(리뷰 lamps-2). 멱등이다(테스트 `opening_the_database_folds_the_assist_grade_from_the_plays` `scoredb/tests.rs:999`가 두 번째 실행의 `total_changes` 증가 0 을 확인). 구 빌드가 같은 DB 를 쓰며 행을 다시 써서 등급이 0 이 돼도 다음 실행에서 복구된다.
- 코스 스테이지: 1.6절 마지막 항목.
- 앱 표시: 결과 화면 라벨 `lamp_label_color(lamp, grade)`(`format.rs:151`, 호출 `app_result.rs:91`), 등급은 `rbms_judge::assist_grade(lamp, Some(record.finished_gauge))`(`app_result.rs:241`). 라벨 함수 둘: `clear_label_color`(등급 없는 램프, `format.rs:118`)와 `stored_lamp_label_color(RecordLamp)`(저장된 램프, `format.rs:164`). 선곡 상세의 최고 기록 라벨, 기록 모달, 기록 목록 행은 등급 이름(`scene.rs:245`, `app_input.rs:615`). 내장 선곡 목록의 막대 LED 는 등급 색(`SelectBar.assist_grade` `list.rs:633`, `scene.rs:135`, 리뷰 lamps-3). 선곡 정렬 Clear 는 `(clear, 등급)` 순(`list.rs:155`). 선곡 필터 CLEARED/NOT CLEARED 는 `ClearType::is_cleared()` 로 NoPlay·Failed 가 아니면 클리어(`list.rs:202-203`, 웹 기준과 같다). 문구: 결과 화면 `SCRATCH AUTO: TURNTABLE NOT COUNTED, ASSIST LAMP`(`stage/result.rs:830`), PLAY 메뉴 `WHILE ON: TURNTABLE NOT COUNTED, ASSIST LAMP BY GAUGE (E.G. ASSIST HARD)`(`play_menu.rs:122`).
- 서버 순위 패널: 응답의 `assist_grade` 를 `RankingRow.record_lamp` 로 받고 램프가 LIGHT ASSIST 일 때만 등급을 붙인다(`ir_ranking.rs:180-186`). 램프 분포는 라벨 문자열이 아니라 `record_lamp.clear` id 로 센다(`skin_host/ir.rs:179`). 서버가 0~4 밖의 값(음수, 255 초과)을 보내면 그 순위 응답 전체의 역직렬화가 실패한다(`assist_grade: u8`). 서버 계약(0~4, 없으면 0)에 의존한다.
- 바뀌지 않은 것: 선곡 막대 램프 목록(11개), 폴더 분포 11종(`bars.rs:21,32`), 결과의 370/371, 폴더 NUMBER 320~330, IR NUMBER 202~240. 외부 스킨은 계속 0~10 만 본다.
- 등급 색 네 개는 각 게이지 램프색(easy, normal, hard, exhard)을 light-assist(`#6563ff`)와 OKLab 에서 35% 섞은 값이고, 기존 11색과 OKLab 거리가 12 이상이다(웹 W2c 보고). 앱 `format.rs:136-139`, 기본 스킨 `theme.grade_lamp`(`theme.lua:152-157`), 웹 `globals.css` 토큰 `--rbms-lamp-assist-easy/-clear/-hard/-exhard` 가 같은 hex 다. 같은지는 `format.rs` 테스트와 웹 `clear-lamp` 테스트가 확인한다.

---

## 2. 저장

### 2.1 열 (클라이언트 `scoredb.sqlite`)

정의 `crates/rbms-store/src/scoredb/schema.sql`, 구조체 `scoredb.rs:193`(`PlayLog`), `scoredb.rs:257`(`BestScore`).

| 열 | `scorelog`(판마다) | `score`(차트별 최고) | 램프 세분과의 관계 |
| --- | --- | --- | --- |
| `clear` INTEGER | 강등 후 램프 id | 최고 램프 id | 정렬 키. 값 변경은 모든 소비처에 번진다(3, 4절) |
| `gauge` INTEGER | 끝난 게이지 id 0~8(`schema.rs:117-131`) | 램프를 올린 판의 값 | 세분 등급의 입력 |
| `gauge_value` REAL | 끝난 게이지 % | 없음 | 클리어 확인용 |
| `assist` INTEGER | 0, 1, 2. 옛 `scores.ron` 에서 이관된 어시스트 행은 -1(`ASSIST_UNRECORDED`, `migrate.rs:18,142`) | 램프를 올린 판의 값 | 세분 등급의 입력 |
| `assist_grade` INTEGER NOT NULL DEFAULT 0 | 없음(파생) | 최고 램프의 어시스트 등급(0~4) | 구현됨(1.8절). 마이그레이션 `ADDED_COLUMNS` 와 `schema.sql` 양쪽에 있다 |
| `rule_version`, `ln_mode`, `state`, `ir_submitted`, `replay_file`, `title` | 있음 | 일부 | 소급 대상 선별 |
| 기타 | 판정 12칸, `combo`, `minbp`, `avgjudge`, `seed`, `random`, `playtime`, `date` | 판정 12칸, `playcount`, `clearcount` 등 | - |

부수 테이블: `play_submission(log_id, body JSON, played_as)` 는 판이 끝날 때 만든 제출 본문 전체를 담는다. 따라서 그 판의 `options.assist` 태그, `options.scratch_auto`, `options.gauge` 가 원문으로 남아 있다(0.1.4 이후의 판). `play_trace(log_id, body)` 는 trace 본문이다(2.4절). `ir_sync`, `course_submission`, `course_sync` 는 전송 상태다. 스키마 세대는 `SCORE_DB_SCHEMA_VERSION = 1`(`schema.rs:12`)이고 열 추가는 `ADDED_COLUMNS`(`schema.rs:60-68`)에 `ALTER TABLE ADD COLUMN` 으로 하는 선례가 있고 `assist_grade` 가 이 길로 들어갔다(`schema.rs:68`). 열 값을 바꾸는 `refold_reference_bests` 는 DB 를 열 때마다 돈다(`schema.rs:49`, `scoredb.rs:691`).

### 2.2 소급·재계산이 되는가

| 알고 싶은 것 | 어디서 | 가능 여부 |
| --- | --- | --- |
| 어시스트 판이 어떤 게이지로 클리어했나 | `scorelog.gauge`, 서버 `score.gauge` | 가능. 단 GAUGE AUTO SHIFT 로 중간에 바뀐 판은 끝난 게이지만 남는다. trace 가 있으면 아홉 게이지 곡선이 있다 |
| 어시스트 단계가 1 인가 2 인가 | `scorelog.assist`, 서버 본문 태그 | 가능 |
| 단계 2 의 원인이 넓힌 판정인가 TOTAL 인가, SCRATCH AUTO 도 겸했나 | 클라이언트 `play_submission.body`(태그 `CUSTOM_JUDGE`, `AUTO_SCRATCH`), 서버 `score.flags`·`total_override`·`judge_rate` | 판별 기록 열만으로는 불가(`assist=2` 하나로 합쳐진다). 본문이나 서버 행으로는 가능 |
| 어시스트 판의 강등 전 램프(풀콤 여부 등) | - | 안 남는다. 어시스트 판은 풀콤 계열이 될 수 없으므로 세분에는 필요 없다 |
| 램프를 올린 판이 아닌 판의 게이지(같은 램프 id 안에서 더 강한 게이지) | `scorelog` 전체 | 가능. `score` 행은 갱신 규칙 때문에 못 따라온다(1.7절). 소급하려면 `scorelog` 에서 다시 접어야 한다 |
| 규칙 세대 2 미만의 어시스트 행 | `scorelog.clear` | 강등 전 램프가 저장돼 있다(D-D9). 선곡 LED 에서 이미 제외된다. 소급 대상에서도 빼는 것이 안전하다 |
| 이관 행(`assist = -1`) | - | 어시스트 종류를 알 수 없다 |
| 메모리 기록 책 `ScoreRecord` | `ScoreRecord.gauge: String`(끝난 게이지 토큰, `gauge_token_of`), `assisted: bool` 뿐(`score.rs:102-141`, `scoredb_store.rs:242-267`) | `assisted` 만으로는 불가. `assisted = !sets_best(assist, rule)` 이므로 SCRATCH AUTO 판(세대 3 이상)은 `assisted=false` 로 읽힌다. 다만 종합 검토에서 확인한 대로 `clear == 3` 이고 `gauge` 가 ASSIST EASY 가 아니면 그 판은 강등된 어시스트 판일 수밖에 없다(`lamp_for_gauge` 가 3 을 주는 게이지는 ASSIST EASY 하나, `gauge.rs:146-155`). 따라서 게이지가 ASSIST EASY 인 판(어차피 가장 낮은 등급)과 세대 2 미만 행을 빼면 기존 필드로도 등급을 계산할 수 있다. 판을 정확히 가르려면 `assist` 필드 추가가 여전히 낫다 |

구현 결과(2026-10-22): 위 표의 어려움 가운데 메모리 기록 책의 `assist` 필드 부족은 `ScoreRecord.assist` 추가로, 램프를 올린 판이 아닌 판의 게이지는 `scorelog` 에서 다시 접는 소급(1.8절)으로 풀었다. 단계 2 의 원인 구분(D4)은 하지 않았다.

### 2.3 리플레이에 옵션이 남는가

`Replay`(`replay.rs:156`)에는 `scratch_auto`(설정 값. 실제로 노트가 빠졌는지가 아니다), `gauge`(설정 게이지 토큰. 끝난 게이지가 아니다), `random`, `seed`, `judge`(`ReplayJudge` `replay.rs:93`: 판정 알고리즘, 키·스크래치 폭 각 3개, LN 여유, LN 방식, 게이지 세트, 게이지 자동 전환과 하한, 오프셋 단계, 시계, `total_override`)가 남는다. 이 정보로 판을 다시 판정해 램프·어시스트 단계·끝난 게이지를 복원하는 코드가 이미 있다: `trace_backfill.rs:428-439` 의 `stored_as` 가 `assist_level(session.has_auto_played_notes(), custom_judge)` 와 `assisted_lamp` 로 행을 다시 만들고, `differs`(`trace_backfill.rs:482-518`)가 `("lamp", judged.clear, stored.clear)` 등을 비교한다. 즉 램프 정의를 바꾸면 소급 trace 검증이 옛 행과 어긋나 거절한다(`Refusal::Never`). 리플레이 파일은 AUTO REPLAY 가 켜진 사람이 친 판에만 있고 보관 정책으로 지워질 수 있다. 소급 검증 `stored_as` 는 이제 `record.finished_gauge` 로 행을 다시 만든다(`trace_backfill.rs:430-440`). 코스 스테이지 행의 저장 gauge 는 CLASS 계열인데 리플레이 헤더의 gauge 는 시작 게이지(설정 게이지)다(`app_result.rs:336`, `ir_replay.rs:119`). 스테이지 리플레이는 CLASS 게이지와 이어받은 게이지 값을 보존하지 않아 충실한 재생이 아니다(조사 이전부터).

### 2.4 trace

판별 trace 는 아홉 게이지 전부의 이력과 경계(`TraceGauge`, `TraceGaugeBounds`)를 담고 선택 게이지 인덱스를 적는다(`dto/trace.rs:56-66,112-130`). 서버 점검(`trace-check.ts:120-132`)은 차트 해시·판정 수·실행 조건만 보고 램프는 비교하지 않는다. 서버 AI(`gauge-advice`)는 "다른 게이지였다면 클리어했을까"를 이 곡선으로 센다(`gauge-advice.service.ts:49`).

### 2.5 기록 동기화가 만드는 본문

로컬 행에서 제출 본문을 다시 만드는 `rebuilt_submission`(`record_sync.rs:898-925`)은 `clear`(행 값), `options.gauge`(행의 `gauge`), `options.scratch_auto`·`options.assist`(행의 `assist`: 0 이면 없음, 1 이면 `AUTO_SCRATCH`, 2 이면 `CUSTOM_JUDGE`, 그 외는 미상 태그)만으로 옵션을 채운다. 세분 정보가 이 경로로도 서버에 도달한다.

---

## 3. 전송과 서버

### 3.1 와이어

- `ClearLamp`(`crates/rbms-ir/src/dto/chart.rs:15-29`)는 변형 이름 그대로의 문자열 enum 이다(대소문자 구분). 점수 제출(`score.rs:55`), 점수 기록 응답(`score.rs:181`), 코스 제출(`score.rs:346`)이 쓴다. `GaugeType` 은 아홉 값(`chart.rs:31-42`).
- 모르는 이름은 디코드 실패다(`chart.rs:219-222`). `ScoreRecord` 등 DTO 에 `deny_unknown_fields` 는 없어(크레이트 안에 없음) 모르는 필드는 무시된다. 따라서 필드 추가는 하위호환이고 enum 값 추가는 아니다. 등급은 필드로 추가했다: `ScoreRecord.assist_grade: u8`(`crates/rbms-ir/src/dto/score.rs:203`, `#[serde(default)]`, 없으면 0).
- 계약 골든 33개가 `contract/client-requests/` 에 있고(`contract_requests.rs:35-68`), Rust 테스트가 실제 요청과 같은지, 웹 테스트가 서버 검사를 통과하는지 본다(`web/tests/dto/client-requests.test.ts`). 램프와 직접 관련된 골든: `score-scratch-auto.json`(`LightAssistEasy`), `score-scratch-auto-hard.json`(신규, `LightAssistEasy` + `options.gauge: Hard`, 서버가 ASSIST HARD 로 파생), `course-scratch-auto.json`(`LightAssistEasy`), `score-widened-judge.json`(AssistEasy 계열), `score-autoplay.json`(`Max`), 나머지는 일반 램프. 갱신은 `RBMS_UPDATE_CONTRACT=1`(`contract_requests.rs:28-31`).
- 다중 IR 프로필: 제출 본문은 클램프 후 한 번 만들어 모든 활성 프로필에 같은 바이트로 보낸다(`ir-multi.md` 2절 7항). 프로필의 서버가 rbms 서버가 아닐 수 있다.
- `api_version` 은 1 이고(`crates/rbms-ir/src/lib.rs:35`) 서버는 다른 값을 400 으로 거절한다(`ir-api.md:12`).

### 3.2 서버의 검증

- 본문 스키마(`web/src/server/dto/score.ts:50`, `dto/common.ts:25`): `clear` 는 `z.enum(CLEAR_LAMPS)`, `options` 는 `z.object` 라서 모르는 키는 버려지고(strict 아님) 모르는 enum 값은 400. `extra` 는 자유 레코드.
- 일관성 검사 `findSubmissionIntegrityFailure`(`score.service.ts:293-305`): EX, 판정 수 ≤ 노트 수, max_ex, minbp, 램프(`isClearLampConsistent` `score.service.ts:262-277`: 풀콤·퍼펙트·맥스만 판정 수로 확인하고 나머지 램프는 확인하지 않음), 파생값의 INT 범위. 판정 수는 `effectiveJudgeCounts`(`best-policy.ts:18-25`: 각 칸을 early+late 로 읽고 0 이면 합계 필드)로 센다. 램프와 `options.gauge`·`gauge_value`의 일치는 검사하지 않는다. 즉 서버는 어시스트 램프와 게이지가 맞는지 모른다.
- 순위 가능 정책 `rankedPolicy`(`ranked-policy.ts:61-72`): 플래그 `AUTOPLAY`, `SCRATCH_AUTO`, `ASSIST`(태그 중 `AUTO_SCRATCH` 가 아닌 것이 있을 때), `JUDGE_WIDTH`(`judge_rate > 100`), `TOTAL_OVERRIDE`(`total_override > 0`), `UNKNOWN_BUILD`, `GUEST`. `SCRATCH_AUTO` 와 `UNKNOWN_BUILD` 는 순위를 막지 않는다(`ranked-policy.ts:27,38`). SCRATCH AUTO 는 `scratchAuto=true` 로 별도 최고 기록 칸에 들어간다.
- 최고 기록 갱신 판정 `isBetterBest`(`best-policy.ts:40-46`): `clear` 가 다르면 큰 쪽, 같으면 `assistGrade`(2026-10-22 부터), 같으면 EX, 같으면 BP 가 작은 쪽.

### 3.3 서버 DB 열

`web/src/server/db/schema.ts`. `clear` 는 모두 `tinyint`(램프 id, 배열 인덱스와 같다)다.

| 테이블 | 램프 관련 열 | 인덱스 |
| --- | --- | --- |
| `score`(판마다, 67~130행) | `clear`(67), `gauge` tinyint(98, 0~8), `gauge_value`(97), `scratch_auto`, `assist` int(110, **태그 개수**이고 클라이언트의 0/1/2 단계가 아니다. `score.service.ts:184`), `judge_rate`, `total_override`, `autoplay`, `ranked`(123), `flags` JSON(124), `extra` JSON | `score_ranking_idx(chart_sha256, ranked, clear, ex_score)`(133) |
| `chart_best`(차트×사용자×`scratch_auto`, `schema.ts:141-`) | `clear`, `assist_grade` tinyint(148, 0~4, 신규), `ex_score`, `minbp`, `max_combo`, `scratch_auto`, `score_id` | PK(`chart_sha256`, `user_id`, `scratch_auto`), `chart_best_rank_idx(chart_sha256, scratch_auto, clear, ex_score)`, `chart_best_rank_grade_idx(chart_sha256, scratch_auto, clear, assist_grade, ex_score)`(161, 신규. 옛 색인은 additive 규칙에 따라 그대로 둬 앞부분이 겹친다) |
| `replay` | `gauge` tinyint(180) | - |
| `course_score`(261~) | `clear`(261), `scratch_auto`, `ranked` | `course_score_rank_idx(course_hash, ranked, clear, ex_score)`(298) |
| `course_best`(`schema.ts:305-`) | `clear`, `assist_grade` tinyint(312, 신규, **항상 0**: 코스 제출과 `course_score` 에 게이지가 없어 파생할 수 없다. 공유 비교 코드를 균일하게 하려고 열만 있다), `scratch_auto` | PK(`course_hash`, `user_id`, `scratch_auto`) |
| `ai_goal.params`, `ai_result` | 램프 이름 문자열(JSON) | - |

`chart_best` 에는 `gauge` 가 없다. `score_id` 로 `score.gauge` 를 조인해야 한다. 등급 열은 이 조인을 피하려고 최고 기록 행에 저장한다. 마이그레이션은 `web/drizzle/0000~0011`(0011 = `0011_assist_grade.sql`, 저널 `when` 1792606768379), 데이터 UPDATE 선례는 `0006_admin_seed.sql:1`. 운영 DB 작업은 `web-db.yml` 워크플로로 메인이 한다(`2026-10-14-adr-production-db-operations.md`).

마이그레이션 0011(`drizzle-kit generate` 로 만들고 이름만 `0011_assist_grade` 로 바꿨다): ① `chart_best` 에 `assist_grade tinyint DEFAULT 0 NOT NULL`, ② `course_best` 에 같은 열, ③ 색인 `chart_best_rank_grade_idx`, ④ 마지막 문장은 수동 SQL 로 `chart_best` 를 `score` 와 조인해 `assist_grade = CASE WHEN score.clear = 3 THEN CASE score.gauge WHEN 1 THEN 1 WHEN 2 THEN 2 WHEN 3 THEN 3 WHEN 4 THEN 4 WHEN 5 THEN 4 WHEN 6 THEN 2 WHEN 7 THEN 3 WHEN 8 THEN 4 ELSE 0 END ELSE 0 END` 로 채운다(`updated_at` 은 바뀌지 않고, WHERE 가 없어 여러 번 돌려도 같다). SQL 의 CASE 와 TS 표 `ASSIST_GRADE_OF_GAUGE`(`assist-grade.ts:8-18`)가 같은지는 단위 테스트가 파일을 읽어 비교한다. 운영 적용은 마이그레이션 먼저(`8d85b32`, run 39693988962), 웹 코드는 그 뒤(`64c6120`)였다([web/architecture.md](../web/architecture.md) 3-2절).

배포 사이 창: 마이그레이션과 코드 푸시 사이에 옛 코드가 `chart_best` 행을 바꾸면 `assist_grade` 가 낡은 채 남는다(옛 코드는 이 열을 쓰지 않는다). 보정 액션 `web-db.yml` 의 `assist-grade-backfill-count`(세기만: 낡은 행 수, 0 초과 등급 행 수, 점수 행이 사라진 행 수)와 `assist-grade-backfill`(세기 → 보정 → 세기)가 있다(`web/scripts/assist-grade.{count,backfill}.sql`, 보정은 바뀔 행에만 `WHERE` 가 걸려 멱등). 운영에서 낡은 행 0 을 확인했다(run 39733088817).

### 3.4 순위·비교 질의

`web/src/server/compose/best-scope.ts` 전부가 `(clear, assist_grade, ex_score, minbp)` 순서를 쓴다. 키 순서는 표 하나 `BEST_ORDER`(`best-scope.ts:26-31`)이고 비교 SQL 은 `isRankedAbove`(`:39`)가 만든다. 등급은 id 3 안에서만 갈리므로 어시스트 판은 더 높은 램프 위로 올라가지 못한다.

- 두 칸(`scratch_auto` true/false)을 합칠 때의 `outranks`(`:51`): 키 순서로 앞서면 이기고 모두 같으면 일반 쪽이 이긴다.
- `isBeatenBy`(`:122`), `isRankedAheadOf`(`:127`), `rankInList`(`:133`, `rank() over (order by clear desc, assist_grade desc, ex_score desc, minbp asc)`), `listOrder`(`:140`). 최고 기록 갱신은 `isBetterBest`(`best-policy.ts:40-46`)가 같은 순서로 비교한다.
- 호출 위치: 차트 순위·내 순위 `compose/index.ts`, 코스 `compose/course.ts`, 점수 제출 응답의 순위(`score.service.ts`). 코스는 `COURSE_ASSIST_GRADE = NO_ASSIST_GRADE`(`course.service.ts:147`)라 등급이 항상 0 이다.
- SCRATCH AUTO 칸 안의 순위가 조사 때는 사실상 EX 로만 갈렸으나(어시스트 클리어가 3 으로 뭉침) 이제 등급으로 먼저 갈린다. 기존 순위가 바뀌는 변경이다(8절 D5).
- 서버 행 파생: 순위·활동·목록 행의 `assist_grade` 는 `score.gauge` 로 즉석 파생하고(`deriveAssistGrade`, `assist-grade.ts:30-31`), 비교·집계(`chart_best`)는 저장된 열을 쓴다. 둘은 같은 표라 일치한다. `deriveAssistGrade({clear, gauge})` 는 clear 가 3 이면 게이지별 등급(`ASSIST_GRADE_OF_GAUGE`), 아니면 0 이고 모르는 게이지 id 도 0 이다. 규칙 세대와 `scratch_auto` 는 입력이 아니다(세대 2 미만 행은 강등 전 램프를 저장했으므로 서버에 "LIGHT ASSIST 인데 게이지가 ASSIST EASY 가 아닌" 옛 행이 생길 수 없다).

### 3.5 집계·표시 소비처 (서버·웹, 램프 id 를 숫자로 비교하는 곳)

| 위치 | 하는 일 | 의존 |
| --- | --- | --- |
| `dto/common.ts:3-56` | `CLEAR_LAMPS` 배열, `clearLampToId`(= 인덱스), `clearLampFromId`(범위 밖은 NoPlay), `MIN_CLEAR_LAMP_ID = clearLampToId('AssistEasy')` (= 2), `ASSIST_GRADES`, `AssistGrade`, `assistGradeSchema`, `ASSIST_GRADE_NAMES`, `NO_ASSIST_GRADE`(44-56) | 순서 |
| `shared/constants/clear-lamp.ts` | 화면용 같은 배열·`MIN_CLEAR_LAMP`·라벨·색 토큰, 등급 표시 계약(`ASSIST_GRADE_LABEL`, `lampLabel(lamp, grade)`, `lampBackground`, `DISPLAY_LAMPS`(11종을 15칸으로 펼친 목록), `isSameDisplayLamp`, `displayLampRank`). 라벨은 id 2 `ASSIST`, id 3 `LIGHT ASSIST`(`L-ASSIST` 약칭 폐기). 두 배열의 일치는 `tests/dto/clear-lamps.test.ts` 가 지킨다 | 순서, 이름 |
| `compose/fe.ts:75`, `fe.service.ts:102` | 클리어 차트 수·클리어 수 = `clear >= MIN_CLEAR_LAMP_ID` (리더보드 `cleared_charts`, 플레이어 통계 `cleared`) | `>=` |
| `fe.service.ts:67-76`, `compose/fe.ts:183-190` | 램프 분포 = `chart_best` 를 `clear`·`assist_grade`·`scratch_auto` 로 묶음(`toLampDistribution`이 `scratch_auto` 를 합침) | 이름 |
| `table-snapshot-filter.ts:11-22,57` | 발광 표 램프 필터(`all/noplay/failed/unclear/cleared/easy/clear/hard/exhard/fullcombo`), 램프 순서로 정렬·분포, 이름 문턱 | 순서, 이름 |
| `ai-goal-form.tsx:10` | 목표 램프 후보 = `CLEAR_LAMPS.slice(indexOf(MIN_CLEAR_LAMP))` | 순서 |
| `chart-leaderboard-panel.tsx:136`, `player-stats-panel.tsx:33`, `lamp-stack-bar.tsx`, `lamp-distribution.tsx` | 최고 램프(인덱스 비교), 분포 막대 | 순서, 이름 |
| AI | 아래 3.6절 | 순서, 이름 |
| `globals.css:57-67,289` | 램프 색 토큰 11개와 등급 토큰 4개(`--rbms-lamp-assist-easy`, `-clear`, `-hard`, `-exhard`). 테스트가 15개 표시 색이 서로 다른 값인지 확인한다 | 이름 |

등급이 닿은 곳(웹 W2c, `64c6120`): 차트 순위·코스 순위·플레이어 점수 목록의 `ScoreRecord`(`score.service.ts:217`), 활동 항목(`fe.service.ts:56`), 플레이어 통계 `lamps[]` 의 `{clear, assist_grade, count}`(`fe.service.ts:76`, `entities/player.type.ts:22`), 발광 표 기록 `TableRecord.assist_grade`(`table-snapshot.service.ts:132`). 컴포넌트 `LampLabel`·`LampStrip`·`LampSwatch`·`LampStackBar`·`LampDistribution`·`ScoreRow` 는 `assistGrade?`(기본 0)를 받는다. 이름이 길어진 표 칸은 폭을 늘렸고(차트 순위·코스 순위 128 → 144, 기록 질문 112 → 144) 약칭은 만들지 않고 `truncate` 를 유지했다. 클리어 집계(`MIN_CLEAR_LAMP_ID` = 2, `cleared_charts`, 표 필터 '클리어 (ASSIST 이상)')는 그대로다.

### 3.6 AI 기능이 램프를 쓰는 곳

모두 `web/src/server`.

- 표 진행 `service/domain/ai/table-progress.ts:51-52,64,74-83`: `FAILED_LAMP_ID`, `HARD_LAMP_ID`, `isClearLampId(lampId) = lampId >= MIN_CLEAR_LAMP_ID`, 기록이 없으면 `CLEAR_LAMPS[0]`, 곡마다 `lampId`.
- 목표 `goal-plan.service.ts:125-126,148`: `lamp_count_at_level` 은 `chart.lampId >= targetLampId`, 목표 램프는 클리어여야 한다. 저장된 목표의 `params.lamp` 는 이름 문자열(`dto/ai.ts:423`).
- 추천 `recommend.service.ts:115`(`lampId > 0` 로 플레이 여부), `practice-folder.service.ts:45,137`(`lampId < FULL_COMBO_LAMP_ID` 인 곡을 연습 후보로), `score-feature.constants.ts:20`(실패 3, 미플레이 2, 램프 업 1 가중치).
- 주간 요약 `weekly-summary.service.ts:145-170,205`: 지난 최고 램프와 비교해 `best.clear > before.clear && isClearLampId(best.clear)` 로 램프 업을 세고 `from_lamp`·`to_lamp` 이름을 낸다.
- 기록 질의 `record-query.compiler.ts:49,99-100`(`NO_PLAY_LAMP_ID`, 이름 → id), `compose/ai-record-query.ts:39`(`compareNumber(columns.clear, op, values)`로 SQL 비교), `record-query.service.ts:148,221`(램프 정렬은 `CLEAR_LAMPS.indexOf`, 모델에게 주는 digest 의 `lamps: CLEAR_LAMPS`). `gte`·`lte` 가 id 순서에 의존한다.
- 게이지 조언 `gauge-advice.service.ts:153`(`isClearLampId` 로 게이지별 클리어 수), 리플레이 코치·플레이 비교(`play-compare.service.ts:92,195,216,266,322`, `replay-coach.service.ts:86,150`)가 램프 이름을 출력한다.
- 출력 스키마 `dto/ai.ts:223,309,423,482-483`, `dto/ai-play.ts:141,174,189` 가 `clearLampSchema`(이름 enum)로 검증한다. 저장된 `ai_result` 에 옛 이름이 있어도 이름이 유지되면 깨지지 않는다.
- 문서: `docs/reference/ai-features/server.md:52,164,174`(`MIN_CLEAR_LAMP_ID`, `clear` 열).

등급이 닿은 AI(웹 W2c, `47a7f9c`): 이름 등급을 더하고 필터 의미(램프 id 비교)는 그대로 둔다.

- 응답·digest 필드 `assist_grade`: 추천·연습·목표 후보(`dto/ai.ts:224`), 기록 질문 행(`ai.ts:311`), 판 코치 play(`ai-play.ts:142`), 주간 요약의 `from_assist_grade`/`to_assist_grade`(`ai.ts:485-487`), 비교 합계 `judgeTotalsSchema`(`ai-play.ts:176`, `47a7f9c`).
- 기록 질문 digest 에 `assist_grades`(이름 배열, `record-query.service.ts:234`)와 `lamp` 필드 설명에 등급 안내를 더했다. `lamp` 필터는 등급을 못 가른다(`VALUE_FORMAT.lamp`에 적혀 있다). 정렬은 등급을 가른다(`SORT_VALUE.lamp = CLEAR_LAMPS.indexOf(row.lamp) * ASSIST_GRADES.length + row.assist_grade`, 점수 source 는 `score.gauge` 에서 파생한 SQL CASE, `compose/ai-record-query.ts`, 리뷰 web-1).
- 추천·목표·연습·주간 프롬프트에 `ASSIST_GRADE_RULE`(`ai-digest.ts:25`) 문장을 넣었고 프롬프트 버전을 1 에서 2 로 올렸다: `AI_GOAL_PROMPT_VERSION`, `AI_PRACTICE_PROMPT_VERSION`, `AI_RECOMMEND_PROMPT_VERSION`, `AI_WEEKLY_PROMPT_VERSION`, `AI_RECORD_QUERY_PROMPT_VERSION`(각 `service/domain/ai/*.service.ts`). 캐시된 AI 결과가 사용자가 다시 요청할 때 한 번 다시 만들어진다. 비교 합계(web-3)는 모델 digest 가 totals 를 담지 않아 프롬프트 버전을 올리지 않았다.
- 주간 요약은 같은 id 안에서 등급이 오르면 램프 상승으로 센다(`lamp_ups`, `new_clears` 정의가 달라졌다). 목표의 남은 곡 정렬은 id 가 같으면 등급 높은 쪽이 앞이다. `listPriorBests` 는 `(차트, scratch_auto, clear, gauge)` 별 max(EX) 행을 돌려주고 서비스가 접는다.
- 제한: `ASSIST_GRADE_RULE` 문장이 영어이고 모델 출력 품질은 실제 공급자로 확인하지 못했다(가짜 공급자 테스트만). 문장은 "그 등급이 이름하는 게이지의 SCRATCH AUTO 클리어" 라고 적는데 등급 1 은 ASSIST EASY 게이지가 아니라 EASY 게이지다(리뷰 web-2는 기각: 이름 규약과 일치).

### 3.7 호환 위험 정리

| 조합 | 결과 |
| --- | --- |
| 새 서버가 응답에 새 램프 이름을 실음 + 이미 설치된 클라이언트(v0.1.6 이하) | 순위·내 최고 기록 응답을 읽지 못함. 서버는 클라이언트 버전을 모르고 응답을 맞출 수 없다 |
| 새 클라이언트가 새 램프 이름을 제출 + 배포 전 서버 | 400 `VALIDATION_ERROR`. 기록 동기화는 거절된 판을 같은 빌드에서는 다시 보내지 않는다(`HANDOFF.md:91` 5번) |
| 새 클라이언트가 기존 이름만 제출(새 필드는 추가) + 구 서버 | 안전. zod 가 모르는 키를 버린다 |
| 서버가 추가 필드를 응답에 실음 + 구 클라이언트 | 안전(무시) |
| 램프 id 일괄 변경 + 서버·클라이언트 순서 | 와이어는 이름이라 id 변경은 보이지 않지만, DB 일괄 UPDATE 와 앱 DB 이관이 한 번씩 필요 |
| 서버가 응답에 `assist_grade` 를 실음 + 구 클라이언트(v0.1.6 이하) | 안전. Rust DTO 에 `deny_unknown_fields` 가 없어 무시된다. 구 클라이언트는 등급 없이 `LIGHT ASSIST` 로 보인다 |
| 새 클라이언트 + `assist_grade` 를 아직 안 싣는 서버 | 안전. `#[serde(default)]` 로 0, 이름은 `LIGHT ASSIST` |
| 마이그레이션 0011 과 코드 푸시 사이 | 옛 코드가 `chart_best` 를 바꾸면 낡은 등급이 남는다. 보정 액션 `assist-grade-backfill`(3.3절)로 바로잡는다 |

---

## 4. 표시

### 4.1 앱이 직접 그리는 화면

- 라벨·색은 `format.rs:114-166` 한 곳이다: `clear_label_color`(등급 없는 램프), `lamp_label_color(램프, 등급)`, `stored_lamp_label_color(RecordLamp)`. 호출: 결과 화면(`app_result.rs:91`), 선곡 막대 LED·상세·기록 모달(`scene.rs:135,245`), 기록 목록 행(`app_input.rs:615`), 서버 순위 패널(`ir_ranking.rs:151-186`, `clear_type_from_lamp` 가 와이어 → 엔진 변환).
- 어시스트 안내 문구: 결과 화면 `stage/result.rs:830` — `SCRATCH AUTO: TURNTABLE NOT COUNTED, ASSIST LAMP`, 그 아래 `WIDENED JUDGE: SENT AND KEPT, BUT NO BEST SCORE`, `OWN TOTAL: SENT AND KEPT, BUT NO BEST SCORE`. PLAY 메뉴 `play_menu.rs:122`. 문구에서 램프 이름 `LIGHT ASSIST` 를 뺐다.
- 서버 순위 패널의 램프 분포는 `record_lamp.clear` id 로 센다(`skin_host/ir.rs:179`). 조사 때는 라벨 문자열이 같은 id 를 찾는 방식이라 등급 라벨 행이 분포에서 빠질 위험이 있었다(`ir_ranking.rs` 의 옛 구현). 클리어 합계는 id 2~10 범위.
- 선곡 필터 `ClearFilter`(`list.rs:165-206`): NOT CLEARED 는 `!lamp.is_cleared()`, CLEARED 는 `lamp.is_cleared()`(= NoPlay·Failed 가 아니면 클리어), FULL COMBO 는 `>= FullCombo`. 정렬 `SortMode::Clear` 는 `(clear, 등급)` 순(`list.rs:155`). 폴더 분포 배열은 `LAMP_KINDS = 11`, 폴더 대표 램프는 분포의 가장 낮은 칸. 곡 상세의 클리어 횟수는 `clear >= 2`(`scene.rs:202`), 최고 기록은 `(clear, 등급)` 큰 순 → EX 큰 순. 조사 때 서로 달랐던 클리어 기준 셋 가운데 필터는 웹 기준으로 맞췄다(7절 2).

### 4.2 기본 스킨 (`assets/skins/rbms-default/lua`)

- `theme.lua:139-186`: `theme.lamp` 색, 등급 색 표 `theme.grade_lamp`(easy, clear, hard, exhard), `theme.clear[0..10]`(키, 이름, 색. id 2 `ASSIST`, id 3 `LIGHT ASSIST`), `theme.graded_clear = 3`, `theme.assist_grade[0..4]`(이름, 색), `theme.assist_grade_last = 4`. 등급 색을 `theme.lamp` 에 넣지 않은 이유: `tools/sprites.py` 가 `theme.lamp` 를 순서대로 읽어 `lamp_strip` 을 만들어서 11칸이 15칸이 되고 `generate-images --check` 가 select 이미지 차이를 낸다.
- 선곡 막대: `select/list.lua:15,123-137` — 램프 id 0~10 마다 막대 안쪽 이미지 하나(`lamp_<key>` 스프라이트가 있으면 도는 프레임, 없으면 `lamp.color` 단색 블록). 어시스트 클리어는 계속 id 3 의 LIGHT ASSIST 색이다(등급 없음).
- 폴더 분포 그래프: `select/list.lua:45,163-164` — `type = -1`(램프별), 시트 `divx=11`. 그대로.
- 선곡 정보창: `select/info.lua:288-`: 선택 곡의 최고 램프가 `theme.graded_clear`(3)이면 `parts.lamp_names`(`parts.lua:180-199`)가 확장 불린 21110~21114(`SELECTED_BEST_ASSIST_GRADE_FIRST`)로 등급 이름·색을 그린다. 등급 0 은 1~4 가 모두 꺼졌을 때 그려 호스트가 답하지 않아도 이름이 비지 않는다(`parts.assist_grade_drawn`).
- 결과: `result/score.lua:68-69`(이번 판, 21120~21124), `110-111`(이번 판 이전의 최고, 21130~21134)가 같은 `parts.lamp_names` 를 쓴다. 이전 최고 램프 글자색을 `sub` 에서 램프 색으로 바꿨다(등급 색을 보이려고 모든 램프에 일관 적용한 작은 시각 변화). 게이지 그래프 선 색은 `result/graphs.lua:25-37` 그대로.
- 호스트가 스킨에 램프를 주는 통로: 이미지 인덱스 370(`INDEX_CLEAR`)과 371(`INDEX_TARGET_CLEAR`), 선택된 막대의 램프 옵션 100~105·1100~1104, 폴더의 램프별 곡 수 NUMBER 320~330, IR 순위의 램프 분포 NUMBER 202~240 의 11칸, 결과의 `NUMBER_CLEAR`(370)·`NUMBER_TARGET_CLEAR`(371)는 그대로 0~10 이다. 등급은 확장 불린 21110~21134 로만 간다(`option-panels.md` 3.4절 표).

### 4.3 외부 Lua 스킨이 기대하는 것 (beatoraja 0~10)

| 항목 | 값 | 근거 |
| --- | --- | --- |
| 선곡 막대 램프 목록 길이 | 11 (`lamp`, `playerlamp`, `rivallamp` 각각) | `bars.rs:21`, `build.rs:240-242` |
| 폴더 분포 종류 | 11 | `bars.rs:32` |
| 분포 그래프 시트 | 열이 종류 수 이상이어야 그림, 모자라면 경고 후 안 그림 | `build.rs:175-182` |
| 막대의 램프 이미지 선택 | 램프 id 로 목록을 직접 인덱싱 | `songlist.rs:391-393`, `scene.rs:92` `lamp: i32::from(bar.lamp)` |
| 결과의 CLEAR 이미지 인덱스 | 370/371 이 램프 id 그대로 | `skin_host/mod.rs:83-84` |
| 폴더 NUMBER 320~330 | 11개 | `skin_host/select.rs:334-339` |
| IR NUMBER 202~240 | 11개씩 | `ir.rs:282-340` |

ModernChic 의 선곡 스킨은 `songlist.lamp`, `playerlamp`, `rivallamp` 각각에 `noplay` 한 개와 `failed, assist, laassist, easy, normal, hard, exhard, fullcombo, perfect, max` 열 개를 이 순서로 넣고(`Select/lua/songlist.lua:128-141,144-157,159-172`), 분포 그래프는 `divx=11`(`docs/reference/skin-compat/m4-select.md:241,308`)이다. 새 id 가 생기면 이런 스킨에서는 이미지가 없어 램프가 사라지거나(목록 밖), 그래프가 사라진다.

LR2 형식 스킨의 램프 번호는 beatoraja 로더 주석이 정의한다: `0:NO PLAY, 1:FAILED, 2:EASY, 3:NORMAL, 4:HARD, 5:EXH, 6:FC, 7:PERFECT, 8:MAX, 9:ASSIST, 10:L-ASSIST` 이고, 이미지가 없는 램프는 `lampg = {{0},{1},{4,2,3},{5},{6,7},{7},{8,9,10},{9},{10},{2},{3}}` 로 다른 램프의 이미지를 공유한다(`LR2SelectSkinLoader.java:37-39`). 즉 원본은 "램프 종류를 늘릴 때 모르는 스킨에는 가까운 램프의 그림을 빌려 준다"는 선례를 이미 갖고 있다(예: LR2 의 EASY 그림 하나가 EASY, ASSIST, L-ASSIST 를 모두 그린다).

---

## 5. 원본과 다른 체계

### 5.1 beatoraja (`/Users/hyunseokbyun/development/beatoraja/src/bms/player/beatoraja`)

- `ClearType.java:10-20`: NoPlay 0, Failed 1, AssistEasy 2, LightAssistEasy 3, Easy 4, Normal 5, Hard 6, ExHard 7, FullCombo 8, Perfect 9, Max 10. 각 항목의 `gaugetype`: LightAssistEasy {0}, Easy {1}, Normal {2, 6}, Hard {3, 7}, ExHard {4, 8}, FullCombo {5}. `getClearTypeByGauge`(`:51-60`)가 게이지 → 램프를 뽑는다. README 도 "11 types of clear lamp"와 "assist options: legacy note, expand judge, bpm guide, and no mine"를 적는다(`README.md:9,11`).
- 판 램프 결정 `play/BMSPlayer.java:863-880`: Failed 로 시작해, 상태가 FAILED 가 아니고 게이지가 클리어이면 `assist > 0` 인 경우 (코스가 아니면) `assist == 1 ? LightAssistEasy : AssistEasy`. 어시스트가 없으면 `passnotes == combo` 일 때 GOOD 0 이면 (GREAT 0 이면 Max 아니면 Perfect) 아니면 FullCombo, 풀콤이 아니면 `gauge.getClearType()`. rbms 는 이 로직을 그대로 옮겼다(`gauge.rs:563-582`, `lib.rs:375-382`). 같은 점: 어시스트 강등은 게이지와 풀콤을 덮는다.
- 어시스트 단계가 오르는 곳(`BMSPlayer.java:200-256,320-334`): BPM 가이드(1), 사용자 정의 판정 100% 초과(2), 패턴 변경자(`LongNoteModifier` REMOVE → 2 이고 ADD 는 LN 이 아닌 종류를 넣을 때 2, `MineNoteModifier` REMOVE → 1, `ScrollSpeedModifier` REMOVE → BPM·스크롤·STOP 변화가 있으면 1, `ExtraNoteModifier` → 노트를 옮기면 2, 7키→9키 `ModeModifier` → 1, 랜덤 옵션 중 H-RANDOM·S-RANDOM-EX·SPIRAL·ALL-SCRATCH·스크래치 포함 레인 셔플 → 1, `PlayerBattleModifier`(DP BATTLE) → 2 이지만 호출부가 `assist = max(assist, 1)` 로 덮음), 모두 `score = false` 가 된다. "legacy note" 라는 단어는 소스에 없다. 같은 일을 하는 것은 `LongNoteModifier` 의 REMOVE 모드(긴 노트 → 일반 노트, 단계 2)로 보이나 소스에 그 이름이 없어 추정이다.
- `AutoplayModifier` 는 beatoraja 에서 DP BATTLE AUTO-SC 에만 쓰이고(`BMSPlayer.java:238-254`) 거기서는 modifier 의 ASSIST(2)를 버리고 `assist = max(assist, 1)` 만 반영한다(로그 문구 "BATTLE (L-ASSIST)"). rbms 의 SP SCRATCH AUTO 가 단계 1 인 것은 이 경로를 따른 rbms 의 선택이다(`mapping.rs:86-88` 주석은 `BMSPlayer.java:248-252` 를 인용한다).
- 최고 기록 병합 `ScoreData.update`(`ScoreData.java:535-585`): `clear < newscore.clear` 이면 램프만 갱신(점수와 별개), 나머지는 `updateScore` 가 참일 때. 로그 검증은 `clear <= ClearType.Max.id`(`:590`, `ScoreLogDatabaseAccessor.java:279`). 즉 원본도 id 정수 비교다.
- 코스(`MusicResult.java:396-428`): 어시스트가 있으면 `LightAssistEasy`(단, 이미 AssistEasy 면 AssistEasy 유지) 또는 AssistEasy, 마지막 스테이지에서만 풀콤 계열 판정, 아니면 게이지 램프.
- 스킨 쪽 상수: 폴더 램프 `lamps = new int[11]`(`DirectoryBar.java:21`), IR 순위 `lamps = new int[11]`(`RankingData.java:40`), `IR_CLEAR_COUNT` 등 11칸 id 표(`IntegerPropertyFactory.java:987-998`), `FOLDER_CLEAR_COUNT` 320~330(`:996`), 막대 램프 옵션 `select_bar_*`(`BooleanPropertyFactory.java:460-469`).
- 원본의 `ScoreData.assist`(`:590` 검증 항목)가 있으나 BMSPlayer 에서 채우는 곳은 이 조사에서 확인하지 못했다.

### 5.2 LR2·IIDX

- LR2: 로컬 소스가 없다. 근거는 beatoraja 의 LR2 스킨 로더 주석(`LR2SelectSkinLoader.java:37-39`)뿐이며, LR2 스킨 번호에서 ASSIST(9)·L-ASSIST(10)는 beatoraja 가 덧붙인 칸으로 보인다(그 주석이 0~8 까지를 LR2 의 NO PLAY ~ MAX 로 나열하기 때문). LR2 의 게이지·램프 동작 자체는 이 조사의 근거 밖이다.
- IIDX: 로컬 원본이 없다. `docs/plan/research-2026-09-09/iidx-feature-gap.md:35-51` 은 IIDX 어시스트 옵션(AUTO SCRATCH, LEGACY NOTE, 5KEYS, A-SCRATCH)과 클리어램프 이름만 나열하고 "ASSIST CLEAR 가 게이지와 결합하는지"는 적지 않는다. 그 규칙(어시스트 옵션 사용 시 램프, ASSIST EASY 게이지와의 관계)은 이 저장소의 근거로는 확인되지 않았으므로 추측으로 쓰지 않았다. 필요하면 별도 웹 조사 항목이다.
- 따라서 "ASSIST HARD 같은 세분 램프"는 beatoraja·LR2 어느 쪽에도 선례가 없는 rbms 확장이다. 기존 선례는 두 가지뿐이다: ASSIST EASY 게이지가 별도 램프(3)를 갖는 것, 어시스트 단계 둘(1, 2)이 별도 램프(3, 2)를 갖는 것.

---

## 6. 세분화 설계 대안

공통 전제: 세분화의 입력은 "어시스트가 있었는가(단계)"와 "클리어를 정한 게이지"이고 둘 다 이미 저장·전송된다(2, 3절). 새로 정해야 하는 것은 이 입력으로 만든 값을 어디에, 어떤 순서로 두느냐다.

### 6.1 안 (a1) 선형 id 에 새 값을 끼워 넣기

예: LightAssistEasy 뒤에 4개(예: ASSIST EASY, ASSIST CLEAR, ASSIST HARD, ASSIST EXHARD 자리)를 넣고 Easy~Max 를 +4 로 민다(4~10 → 8~14).

- 순서: 어시스트 < EASY 를 지키려면 끼워 넣을 위치가 3 과 4 사이뿐이다. 그래서 id 4~10 이 밀리고 정수 비교 코드는 상수만 바뀐다.
- 최고 램프 병합: 코드 구조 그대로(정수 최댓값). 단 로컬 DB 의 `score.clear`, `scorelog.clear` 일괄 UPDATE 와 옛 `scores.ron` 이관 경로(`ScoreRecord.clear`)를 고쳐야 한다. 상수 `CLEAR_FAILED_ID`(1)는 안전, `clear >= 2`(`scene.rs:200`)는 어시스트 4개가 생겨도 그대로 맞고 `ClearFilter` 의 `Easy` 문턱은 `clear_type_id(Easy)` 라서 자동이다.
- 웹: 네 테이블(`score`, `chart_best`, `course_score`, `course_best`) `clear` 일괄 UPDATE 마이그레이션(운영 DB 는 `web-db.yml` 로 적용), `CLEAR_LAMPS` 두 배열, 라벨·색 토큰 4개, 테스트 고정값(`lamp-label.test.tsx:23-24` 의 11), `MIN_CLEAR_LAMP_ID` 는 위치가 안 변해 그대로. 순위 질의·인덱스는 변경 없음.
- 외부 스킨: 호스트가 스킨에 주는 모든 램프 값(370/371, 옵션 100~105·1100~1104, NUMBER 320~330, IR 202~240, 막대 램프 목록, 분포)을 0~10 으로 번역하는 계층이 필수다. 새 4개는 스킨에게 id 3 으로 접어 보여야 한다. `LAMP_KINDS`(11)를 키우면 외부 스킨의 분포 그래프가 사라진다(`build.rs:177`)이므로 키우지 못하고 번역해야 한다.
- 소급: 가능. `(clear ∈ {2,3}, assist, gauge)` 로 새 id 를 계산해 UPDATE. 단 `score` 최고 기록 행은 램프를 올린 판의 값만 들어 있어 같은 램프에서 더 강한 게이지의 판을 따라가지 못하므로 `scorelog` 에서 다시 접어야 한다.
- 호환: 와이어에 새 이름이 생겨 3.7절의 두 위험(구 클라이언트의 응답 디코드 실패, 구 서버의 400)을 그대로 진다. 서버를 먼저 배포해도 구 클라이언트는 새 이름이 든 응답을 못 읽는다. 완화하려면 응답에서 새 이름을 기존 이름으로 접는 변환이 필요한데 서버는 호출자의 버전을 모른다.
- 계약 테스트: 이름 목록 고정 테스트 3곳(`gauge.rs` id 테스트, `mapping.rs:169-270`, `format.rs:136-147`), 웹 `clear-lamps.test.ts`, `lamp-label.test.tsx`, 골든 3개 갱신과 새 골든.
- 작업량: 가장 크다(양쪽 DB 이관 + 스킨 번역 계층 + 와이어 호환 장치).

### 6.2 안 (a2) 선형 id 에 11 이상으로 덧붙이기

- 순서가 깨진다: id 11 이 MAX(10)보다 커서 `max(clear)` 와 `ORDER BY clear` 가 ASSIST HARD 를 최고 램프로 만든다. 그래서 정수 비교를 전부 "램프 순위표"로 바꾸거나 별도 순위 열을 둬야 한다. 클라이언트 비교 지점 최소 8곳(`scoredb.rs:449,466,615`, `score.rs:299-311`, `list.rs:153,199-201,681`, `scene.rs:200-201`)과 웹의 `best-scope.ts` 전부, `compose/fe.ts:75` 등 SQL `>=`, AI 의 `>=`·`gte` 비교가 영향을 받는다. `score_ranking_idx`·`chart_best_rank_idx` 가 `clear` 정렬에 의존하므로 순위표 변환을 SQL 에 넣으면 인덱스가 무력해진다 → 사실상 별도 순위 열이 필요하고 그러면 6.4 안과 같아진다.
- 데이터 이동은 없지만(기존 id 불변) 소급 UPDATE 는 같다. 스킨 번역 계층과 와이어 호환 위험은 a1 과 같다.

### 6.3 안 (b) 램프와 어시스트를 직교 필드로 분리하고 표시만 결합

램프 id 는 "게이지 기준"(어시스트로 강등하지 않음), 어시스트는 별도 축, 표시는 둘을 합친다. SCRATCH AUTO + HARD 클리어는 HARD(6) + 어시스트 1.

- 장점: 개념이 가장 깨끗하다. 새 정보가 램프에 이미 있으므로 id 추가가 없다. 외부 스킨은 0~10 그대로(어시스트 판이 HARD 로 보이는 점만 다르다).
- 병합·순위: 지금은 강등이 "어시스트 판이 정상 판을 램프로 못 넘게" 막는다. 강등을 없애면 이 일을 비교 키가 맡아야 한다: 클라이언트 `merge_best`·`best_clear_for_md5`·정렬에서 어시스트 단계를 따로 비교해야 하고, 웹은 `chart_best` 가 `scratch_auto` 로 칸이 갈려 있으나 "SCRATCH AUTO 포함" 합산 모드의 `outranks`(`best-scope.ts:15-25`)에서 SCRATCH AUTO HARD 가 일반 NORMAL 을 이기게 돼 순위가 바뀐다. 정책 재결정이 필요하다.
- 소급: 어렵다. 이미 3 으로 강등 저장된 SCRATCH AUTO 판의 풀콤 정보가 없어(어시스트 판은 풀콤 계열이 못 됐다) 복원값은 게이지 램프까지만 가능하다. 서버에는 끝 콤보가 없다(`max_combo` 만 있음).
- 호환: 원본 beatoraja 의 강등 규칙과 어긋나 IR 의 id 의미가 바뀐다. 구 클라이언트·구 서버가 같은 `clear` 이름을 다른 뜻으로 받는다(구 서버는 SCRATCH AUTO 판을 3 으로 가정하는 집계가 있다: 웹 ADR `2026-10-19-adr-web-clear-and-play-history.md` 1절).
- 계약 테스트: `score-scratch-auto.json`, `course-scratch-auto.json` 의 `clear` 값이 바뀐다.

### 6.4 안 (c) 호환 id 0~10 유지 + 어시스트 등급을 덧붙임 (채택, 구현됨)

구현 결과는 1.8절이고 아래는 조사 당시 설계다. 설계와 다르게 구현된 곳: 클라이언트 DB 에 열을 더하지 않는다고 했으나 `score.assist_grade` 열을 더했고(최고 기록 SQL 정렬과 소급 비교를 위해), 웹은 `chart_best`·`course_best` 에 열과 색인을 더했다.

램프 id·와이어 이름·강등 규칙은 그대로 둔다. 어시스트로 강등된 클리어(램프 2, 3)에 대해서만 게이지 기준 등급을 파생한다.

- 정의: `tier = f(clear ∈ {2,3}, 끝난 게이지)`. 등급 후보는 게이지 5 단계(ASSIST EASY, EASY, NORMAL, HARD, EXHARD)에서 HAZARD 를 EXHARD 로 접거나, 둘 다 두는 것까지 8절 D2 에서 정한다. 라벨은 "ASSIST " + 등급 이름처럼 합성한다(8절 D3).
- 저장: 클라이언트 DB 에 새 열이 필요 없다(`scorelog.gauge`·`assist` 로 파생). `score` 최고 기록 행의 갱신 규칙만 `clear` 같고 등급이 더 높으면 `gauge`·`assist` 를 갱신하도록 넓힌다(`scoredb.rs:615`). `ScoreRecord` 에 `assist`(과 필요하면 등급)를 추가해야 한다(`score.rs:102`, `scoredb_store.rs:242`, 새 필드는 `serde(default)`).
- 병합: 비교 키를 `(clear, tier, EX, BP)` 로 넓힌다. 클라이언트 `merge_best`, `best_clear_for_md5*`, `ScoreDb::best`·`chart_best_many` SQL 에 등급 키가 들어간다. 웹은 `chart_best`·`course_best` 에 등급 열을 추가하고(또는 `score` 조인) `best-scope.ts` 의 비교(`outranks`, `isBeatenBy`, `isRankedAheadOf`, `rankInList`, `listOrder`)와 `isBetterBest` 에 등급을 한 단계 끼운다.
- 웹 마이그레이션: `ADD COLUMN`(additive) + `score` 를 조인한 백필 UPDATE + SCRATCH AUTO 칸의 최고 기록 재선정 스크립트. 기존 id·인덱스는 그대로. 기존 `0006` 의 데이터 UPDATE 와 `chart-notes.repair.sql` 형태의 복구 스크립트 선례가 있다.
- 외부 스킨: 0~10 만 본다(어시스트 클리어는 계속 id 3). 기본 스킨만 읽을 rbms 확장 속성(이미지 인덱스·NUMBER)을 추가하면 기본 스킨에서 세분 표시를 한다. `LAMP_KINDS`(11) 그대로.
- 소급: 가능(2.2절). 규칙 세대 2 미만과 `assist = -1` 행은 제외한다.
- 호환: 제출 본문이 바뀌지 않는다. 구 서버는 그대로 동작한다(`clear` 와 `options.gauge`, `scratch_auto` 는 이미 보냄). 서버가 응답에 등급 필드를 추가하면 구 클라이언트는 무시한다.
- 계약 테스트: 기존 골든 불변. SCRATCH AUTO + HARD 판의 새 골든 하나를 더하면 서버 쪽 등급 파생이 고정된다. Rust 쪽에는 파생 함수 단위 테스트가 새로 필요하다.
- 단점: 라벨이 `(clear, 등급)` 합성이라 "램프 id 하나"가 곧 표시 이름이 아니다. 서버 응답에 등급을 싣는 설계(필드 이름·형태)와 웹의 모든 램프 표시 컴포넌트가 등급 인자를 받도록 바꿔야 한다. 클리어 판정 합산(`clear >= 2`)은 영향이 없다.

### 6.5 안 (d) 표시 전용

저장·병합·웹은 건드리지 않고 결과 화면과 판별 기록 목록에서만 `(clear, assist, gauge)` 로 라벨을 만든다.

- 가장 작다. 단 선곡의 최고 램프와 웹은 계속 LIGHT ASSIST 이고, 최고 기록 행이 갱신되지 않아 "내 최고는 ASSIST HARD"가 어디에도 나타나지 않는다. 사용자의 요청(램프 세분화)을 절반만 충족한다.
- 6.4 의 1단계로 쓰기 좋다(저장 변경 없이 먼저 보여 주고 병합·웹은 뒤에 얹는다).

### 6.6 비교표

| 항목 | a1 끼워 넣기 | a2 덧붙이기 | b 직교(비강등) | c 호환 id + 등급 | d 표시 전용 |
| --- | --- | --- | --- | --- | --- |
| 최고 램프 병합 | 정수 비교 유지, DB 값만 이관 | 모든 비교를 순위표로 | 비교 키를 새로 정의 | 키에 등급 한 단계 추가 | 변경 없음 |
| 웹 집계·순위 | 4개 테이블 UPDATE, 상수 | 순위 열 추가가 사실상 필수 | 합산 순위 정책 재결정 | 열 추가 + 비교 5곳 | 변경 없음 |
| 마이그레이션 | 양쪽 일괄 UPDATE | 없음(단 순위 열 백필) | 복원 불완전 | additive + 백필 | 없음 |
| 외부 스킨(0~10) | 번역 계층 필수 | 번역 계층 필수 | 거의 무관(어시스트 판이 정상 램프로 보임) | 무관(id 3 유지) | 무관 |
| 기존 기록 소급 | 가능 | 가능 | 일부만 | 가능 | 해당 없음 |
| 구 클라이언트·구 서버 | 새 이름이 와이어에 → 위험 | 새 이름이 와이어에 → 위험 | 의미 변경 위험 | 안전(필드 추가뿐) | 안전 |
| 계약 골든 | 이름 목록·골든 갱신 | 같음 | `clear` 값 갱신 | 골든 불변 + 신규 1 | 불변 |
| 램프 순서 성질 | 어시스트 < EASY 유지 | 깨짐 | 깨짐(비교 키로 복구) | 유지 | 유지 |
| 작업량 | 매우 큼 | 큼 | 큼 | 중간 | 작음 |

### 6.7 어시스트 종류별로 따로 생각할 점

- SCRATCH AUTO(id 3): 게이지 등급이 의미 있다. 랭크 칸이 따로 있어 SCRATCH AUTO 안의 순위가 지금은 EX 만으로 갈리는 문제도 등급 키로 개선된다(3.4절 끝).
- ASSIST EASY 게이지로 어시스트 없이 클리어(id 3): 이미 별도 램프다. 세분에서 가장 낮은 등급에 합칠지가 필요하다.
- 넓힌 판정·TOTAL(id 2): 항상 unranked 이고 최고 기록도 못 세운다. 세분 실익이 작고 원인 구분(2.2절)이 판별 기록 열만으로는 안 된다.
- 어시스트 판의 풀콤 계열: 원본 규칙상 없다. 세분 목록에도 넣지 않는 편이 규칙과 맞다.

---

## 7. 기존 문서와 코드가 다른 곳, 코드끼리 다른 곳

조사 때의 목록이고 구현 뒤 상태를 덧붙였다.

1. 이름 세 벌(1.1절): 앱·기본 스킨·웹을 한 이름으로 통일했다. `docs/HANDOFF.md` 미해결 8번은 이 구현으로 해소. 앱 설정 설명 문구 `stage/settings/catalog.rs:147,169` 의 옛 이름 `ASSIST EASY at best` 도 `the ASSIST lamp at best` 로 고쳤다(2026-10-22).
2. "클리어"의 기준: 선곡 필터를 웹 기준(NO PLAY·FAILED 가 아니면 클리어)으로 맞췄다(`list.rs:202-203`). 곡 상세의 클리어 횟수 `clear >= 2` 와 웹 `MIN_CLEAR_LAMP_ID` = 2 는 같다.
3. 원본에서 자동 연주 레인을 만드는 `AutoplayModifier` 는 DP BATTLE AUTO-SC 에서만 쓰이고, 거기서 modifier 가 올리는 ASSIST(2)는 버려지고 호출부가 단계 1 만 올린다(`BMSPlayer.java:238-254`). SP 에서 턴테이블을 자동으로 치는 옵션은 원본에 없고 rbms 의 SCRATCH AUTO 가 그 경로의 단계 1 을 재사용한 것이다. `mapping.rs:86-88` 의 주석은 이 1 을 정확히 인용하지만 같은 파일의 `assist_flags` 주석(`mapping.rs:121-124`)은 단계 2 처럼 읽힌다. 동작은 1 이다. 변화 없음.
4. 원본이 어시스트로 세는 RANDOM 계열(H-RANDOM, ALL-SCRATCH, SPIRAL, S-RANDOM-EX 등)과 BPM 가이드·지뢰 제거 등을 rbms 는 어시스트로 세지 않거나 옵션 자체가 없다(1.4절, 5.1절). 변화 없음(D11).
5. `docs/plan/research-2026-09-09/iidx-feature-gap.md:38` 은 LEGACY NOTE 를 "미구현"으로 적지만 지금은 표시 옵션으로 있다. 변화 없음.
6. `ScoreRecord`(메모리 책)는 `assisted` 하나뿐이어서 선곡·기록 목록 쪽에 어시스트 단계가 없었다: `ScoreRecord.assist` 를 더해 해소(1.8절).
7. 서버 `score.assist`(태그 개수)와 클라이언트 `scorelog.assist`(0/1/2 단계)는 같은 이름의 다른 값이다(3.3절). 변화 없음. 서버 등급은 `assist` 가 아니라 `clear` 와 `gauge` 로 파생한다.
8. 서버는 어시스트 램프와 `options.gauge` 의 일치를 검사하지 않는다(3.2절). 등급 파생이 `score.gauge` 에 의존하므로 일치하지 않는 본문은 일치하지 않는 등급이 된다. 현재 클라이언트는 항상 일치시켜 보낸다.
9. 코스 스테이지의 저장 gauge 와 리플레이 헤더 gauge 가 다르다(2.3절).

---

## 8. 결정 지점

각 항목에 선택지, 메인 추천, 이유, 영향 범위를 적는다. D1 을 정하면 나머지 대부분이 따라온다.

### D1. 세분 구조

- 선택지: A) 호환 id 유지 + 어시스트 등급 덧붙임(6.4, c) / B) 선형 끼워 넣기(a1) / C) 직교 필드로 램프 비강등(b) / D) 표시 전용(d) / E) 덧붙이기(a2)
- 추천: A. 단계는 D → A 로 나눠 진행.
- 이유: id·와이어 이름·강등 규칙이 그대로라 구 클라이언트·구 서버·외부 스킨(11칸 고정)·원본 beatoraja 호환을 모두 지키고, 제출 본문이 안 바뀌어 계약 골든도 거의 그대로다. 필요한 데이터는 이미 저장돼 있어 소급도 된다. 어시스트 < EASY 순서 성질이 유지된다.
- 영향: 클라이언트 `rbms-judge`(등급 파생 함수), `rbms-store`(병합 규칙, `ScoreRecord` 필드), 앱 표시 4곳(결과, 선곡, 기록 목록, 서버 순위 패널), 기본 스킨, 웹(열 추가, 비교 5곳, 표시 컴포넌트).
- 결정됨(2026-10-22, 메인이 정함, 사용자 미확인): A. 안 c(호환 id 유지 + 어시스트 등급). 단계 D → A 로 나누지 않고 한 번에 구현했다. ADR [어시스트 램프 등급](../acknowledge/2026-10-22-adr-assist-lamp-grade.md).

### D2. 세분 등급의 목록과 순서

- 선택지: A) 4등급 EASY·NORMAL·HARD·EXHARD (ASSIST EASY 게이지는 EASY 에 합침, HAZARD 는 EXHARD 에 합침) / B) 5등급 ASSIST EASY·EASY·NORMAL·HARD·EXHARD (HAZARD 는 EXHARD) / C) 6등급 (HAZARD 따로) / D) 등급 없이 현재처럼 하나
- 추천: A. 순서는 낮은 쪽부터 EASY → NORMAL → HARD → EXHARD, 모두 EASY 램프(4)보다 아래.
- 이유: 사용자가 든 예(ASSIST HARD, ASSIST EXHARD)가 정상 램프(EASY, CLEAR, HARD, EX HARD)의 대응이다. ASSIST EASY 게이지와 EASY 게이지는 둘 다 쉬운 그루브 게이지라 갈라도 실익이 작다. HAZARD 는 어시스트 상황에서 의미를 줄 근거가 없다. 어시스트 클리어가 풀콤 계열이 못 되는 것은 규칙 그대로 둔다.
- 영향: 등급 열거형 크기, 스킨의 새 색 4개, 기본 스킨 막대·정보창 행 수, 웹 라벨·색 토큰.
- 결정됨(2026-10-22, 메인이 정함, 사용자 미확인): 추천 A(4등급 EASY·NORMAL·HARD·EXHARD, HAZARD 는 EXHARD 에 합침)를 따르되 ASSIST EASY 게이지의 클리어는 EASY 등급에 합치지 않고 등급 없음(LIGHT ASSIST)으로 두었다. 등급 이름 ASSIST EASY 는 EASY 게이지 클리어를 가리킨다(1.8절). 사용자가 이 이름 규약(ASSIST EASY 게이지와 ASSIST EASY 등급이 다른 것)을 확인해야 한다.

### D3. 표시 이름

- 선택지: A) "ASSIST " + 등급(ASSIST EASY, ASSIST CLEAR, ASSIST HARD, ASSIST EXHARD)이고, 등급을 알 수 없는 기존 id 3 은 "LIGHT ASSIST" 로 유지, id 2 는 "ASSIST" 로 이름만 짧게 / B) "L-ASSIST " + 등급 / C) 이름은 그대로 두고 색만 등급별
- 추천: A.
- 이유: 사용자 표현과 같다. id 2 의 이름이 새 등급 이름 "ASSIST EASY" 와 충돌하므로 id 2 를 "ASSIST" 로 줄여야 한다(웹은 이미 `ASSIST`, 앱은 `ASSIST EASY`, 기본 스킨은 `ASSIST CLEAR`). 이 기회에 `docs/HANDOFF.md:94` 의 이름 불일치를 앱·기본 스킨·웹 세 곳에서 한 번에 통일한다.
- 영향: `format.rs`, `theme.lua`, `clear-lamp.ts`, 결과 화면 안내 문구(`result.rs:829`, `play_menu.rs:113`), 테스트 문자열, 가이드 문서. 이름 통일 자체는 D1 과 독립으로 먼저 할 수 있다.
- 결정됨(2026-10-22, 메인이 정함, 사용자 미확인): A. 'ASSIST ' + 등급, 등급 없음은 LIGHT ASSIST, id 2 는 ASSIST. 앱·기본 스킨·웹 세 곳을 한 번에 통일했다(1.1절). 남은 옛 이름: 앱 설정 설명 문구(7절 1).

### D4. 어시스트 종류별 적용 범위

- 선택지: A) SCRATCH AUTO(id 3)만 세분, 넓힌 판정·TOTAL(id 2)은 한 램프 유지 / B) 둘 다 세분
- 추천: A.
- 이유: id 2 는 항상 unranked 이고 최고 기록도 못 세워 보여 줄 가치가 작다. 원인(넓힌 판정 vs TOTAL vs SCRATCH AUTO 겸용)이 판별 기록 열에서 구분되지 않아(2.2절) 소급 정확도도 떨어진다. 구조는 나중에 id 2 로 확장할 수 있게 둔다.
- 영향: 파생 함수의 조건, 소급 SQL 범위, 웹 표시.
- 결정됨(2026-10-22, 메인이 정함, 사용자 미확인): A. SCRATCH AUTO(id 3)만 세분한다. id 2(넓힌 판정·TOTAL)는 등급이 없다.

### D5. 비교·순위에 등급을 쓸 범위

- 선택지: A) 클라이언트의 최고 램프 접기와 웹 순위 키 모두 `(clear, 등급, EX, BP)` / B) 클라이언트만 / C) 표시만(비교는 지금처럼)
- 추천: A, 단 웹 순위 키는 2단계(표시 → 병합 → 순위 순)로.
- 이유: 등급이 순위에 안 들어가면 SCRATCH AUTO 칸의 순위는 계속 EX 만으로 갈려 "ASSIST HARD 가 ASSIST NORMAL 보다 높다"가 순위에 나타나지 않는다. 반대로 순위 키 변경은 SCRATCH AUTO 칸의 기존 순위를 바꾸므로 사용자 확인이 필요하다.
- 영향: SCRATCH AUTO 최고 기록 칸의 순위, 합산 모드(`outranks`)에서 일반 판과의 비교(등급이 EASY 아래라 일반 클리어를 못 넘는 성질은 유지), `best-scope.ts` 전부, 클라이언트 `merge_best`.
- 결정됨(2026-10-22, 메인이 정함, 사용자 미확인): A. 클라이언트 최고 램프 접기와 웹 순위 키 모두 `(clear, 등급, EX, BP)`. 추천은 웹 순위 키를 단계(표시 → 병합 → 순위)로 나누는 것이었으나 한 번에 구현했다. SCRATCH AUTO 칸의 기존 순위가 바뀐다(3.4절).

### D6. 기존 기록 처리

- 선택지: A) 소급 파생(판별 기록은 읽을 때 계산, 최고 기록 행은 DB 를 열 때 `scorelog` 에서 다시 접기, 서버는 백필 마이그레이션) / B) 소급 없이 새 판부터 / C) 판별 기록 표시만 소급
- 추천: A. 규칙 세대 2 미만과 `assist = -1` 행은 제외.
- 이유: 필요한 입력이 이미 저장돼 있고 `refold_reference_bests`(`scoredb.rs:691`)처럼 매번 안전하게 다시 접는 선례가 있다. 사용자의 기존 SCRATCH AUTO 기록 대부분이 이 대상이다(`play-options.md` 10.4절, 운영 사용자의 판은 대부분 SCRATCH AUTO, `2026-10-19-adr-web-clear-and-play-history.md`).
- 영향: 클라이언트 DB 열기 경로, 서버 마이그레이션 0011(additive)과 최고 기록 재선정(`web-db.yml` 로 운영 적용, 파괴적 작업이 아님), 소급 실패 시 폴백(등급 없음 → "LIGHT ASSIST").
- 결정됨(2026-10-22, 메인이 정함, 사용자 미확인): A. 클라이언트는 DB 를 열 때 `scorelog` 에서 다시 접고(규칙 세대 2 미만·`assist = -1` 제외), 서버는 마이그레이션 0011 의 백필 UPDATE 와 보정 액션으로 소급했다. 서버 SCRATCH AUTO 칸의 최고 기록 재선정은 하지 않았다: 백필은 `chart_best` 가 가리키는 `score` 의 값만 채운다. 같은 차트에서 게이지가 다른 SCRATCH AUTO 판이 둘 이상인 사용자는 EX 로 고른 최고 기록보다 등급이 높은 다른 판이 `score` 에 있을 수 있고, 그 최고 기록은 다음에 더 높은 `(clear, 등급, EX, BP)` 판을 제출할 때 바뀐다.

### D7. 웹 반영 범위

- 선택지: A) 1단계 표시(최근 플레이·차트 순위·발광 표에서 `score.gauge` 로 파생 라벨) → 2단계 `chart_best` 열·비교·순위 → 3단계 AI·집계 / B) 처음부터 전부 / C) 웹은 건드리지 않음
- 추천: A.
- 이유: 웹은 와이어 호환 위험(3.7절)이 가장 큰 곳이라 additive 변경부터 배포하고 순위 키는 별도 결정(D5)에 맞춘다. AI 기능은 id 순서·이름에 의존(3.6절)하므로 id 를 안 바꾸는 안에서는 별도 수정이 거의 없다.
- 영향: `dto/common.ts`·`clear-lamp.ts`(등급 상수), 표시 컴포넌트(`features/lamp/*`, `score-row.tsx` 등), `compose/fe.ts`(분포), 필요 시 응답 DTO 에 필드 추가, 웹 테스트 고정값.
- 결정됨(2026-10-22, 메인이 정함, 사용자 미확인): A 의 범위를 단계 없이 한 번에 구현했다(표시, `chart_best`·비교·순위, AI·집계). 배포는 마이그레이션 0011 → 웹 코드 순서(두 번 푸시).

### D8. 규칙 세대와 구 빌드 공존

- 선택지: A) `SCORE_RULE_VERSION` 유지(4)하고 DB 열 때 다시 접기로 정합 / B) 5 로 올림
- 추천: A.
- 이유: 병합 규칙 변경이 "저장된 값의 의미"를 바꾸지 않고 비교 방식만 넓히며, 서버 AI 의 기준 규칙 필터(`>= 4`)와 `sets_best` 가 이미 4 에 맞춰져 있다. 구 빌드가 같은 DB 를 열어 옛 규칙으로 접어도 다음 실행에서 바로잡힌다.
- 영향: `score.rs:34`, `scoredb.rs:613-652`, 문서 `play-options.md` 10.11절.
- 결정됨(2026-10-22, 메인이 정함, 사용자 미확인): A. `SCORE_RULE_VERSION` 4 유지(`score.rs:34`). 구 빌드가 같은 DB 를 열어도 다음 실행에서 바로잡힌다.

### D9. 외부 스킨에 주는 값

- 선택지: A) 외부 스킨에는 항상 0~10(어시스트는 id 3)을 주고, 기본 스킨에만 확장 속성으로 등급을 줌 / B) 외부 스킨에도 확장 값(규격 밖)
- 추천: A.
- 이유: 11칸 전제를 지키고(4.3절) 새 id 가 없으니 번역 계층도 필요 없다.
- 영향: `skin_host` 에 rbms 확장 속성 한 개, 기본 스킨 `theme.lua`·`list.lua`·`info.lua`·`score.lua`, 아틀라스 이미지(없으면 단색 블록으로 폴백).
- 결정됨(2026-10-22, 메인이 정함, 사용자 미확인): A. 외부 스킨에는 0~10, 기본 스킨만 확장 불린 21110~21134(4.2절).

### D10. 이름 정리와 클리어 기준 정리

- 선택지: A) D3 와 함께 한 번에, 선곡 필터 `CLEARED` 도 웹 기준(NO PLAY·FAILED 가 아닌 모두)으로 맞춤 / B) 이름만 / C) 하지 않음
- 추천: A.
- 이유: 7절 1·2항의 불일치가 이미 사용자에게 보이는 문제이고 세분을 넣으면 더 눈에 띈다.
- 영향: `list.rs:195-201`, `HANDOFF.md` 미해결 8번.
- 결정됨(2026-10-22, 메인이 정함, 사용자 미확인): A. 이름 통일과 선곡 필터 CLEARED 를 웹 기준으로 맞춤(7절 1·2).

### D11. 원본 어시스트 원인 확장 (이번 범위 밖)

- 선택지: A) 이번에는 하지 않고 기록만 해 둠 / B) H-RANDOM·ALL-SCRATCH 등을 LIGHT ASSIST 로 셈
- 추천: A.
- 이유: 요청 범위(램프 세분화)와 별개이고, 세면 기존 기록과 서버 순위 정책(`ranked`)이 함께 바뀐다.
- 영향: `assist_level` 입력, 규칙 세대, 서버 `assistFlags`.
- 결정됨(2026-10-22, 메인이 정함, 사용자 미확인): A. 하지 않았다.

---

## 9. 구현한 곳 요약 (추천안 기준 체크리스트, 모두 완료)

- 클라이언트 코어: `crates/rbms-judge/src/gauge.rs`(`AssistGrade`, `assist_grade`), `crates/rbms-store/src/score.rs`(`ASSIST_GRADE_*`, `assist_grade_of`, `RecordLamp`, `ScoreRecord.assist`), `scoredb.rs`(`merge_best`, `best`, `chart_best_many`, `refold_assist_grades`), `scoredb/schema.rs`·`schema.sql`(열), `scoredb/courses.rs`(`put_course_stages_on_class`), `apps/rbms-player/src/scoredb_store.rs`(`record_of`, 코스 스테이지 gauge).
- 클라이언트 표시: `format.rs`, `app_result.rs`, `stage/select/{scene,list}.rs`, `app_input.rs`, `ir_ranking.rs`, `skin_host/{extension,select,result,ir}.rs`, `stage/result.rs:830`·`play_menu.rs:122`(문구), `trace_backfill.rs`(`stored_as` 가 `record.finished_gauge`), `crates/rbms-ir/src/{dto/score.rs,mapping.rs}`(`assist_grade` 필드, `ir_gauge_of_index`).
- 기본 스킨: `lua/theme.lua`, `lua/parts.lua`, `lua/props.lua`, `lua/select/info.lua`, `lua/result/score.lua`.
- 웹: `web/drizzle/0011_assist_grade.sql` + `meta`, `db/schema.ts`, `compose/best-scope.ts`, `compose/{index,course,fe,table-snapshot,ai-score-features,ai-play-features,ai-record-query}.ts`, `service/domain/score/{assist-grade,best-policy,score.service}.ts`, `dto/{common,score,table-snapshot,ai,ai-play}.ts`, `shared/constants/clear-lamp.ts`, `globals.css`, 램프 표시 컴포넌트, AI 서비스 8개, `web/scripts/assist-grade.{count,backfill}.sql`, `.github/workflows/web-db.yml`.
- 계약: 기존 골든 불변, 새 골든 `score-scratch-auto-hard.json`(`contract_requests.rs:53`, `:588`).
- 테스트(새): 엔진 등급 단위 테스트 7개(`rbms-judge`), 저장 파생·병합·`chart_best`·refold 멱등, 앱 `the_score_store_grades_every_stored_run_as_the_engine_does`·코스 스테이지·내장 막대 LED, 웹 `assist-grade.test.ts`(마이그레이션 SQL 과 TS 표 일치), `best-scope-sql.test.ts`, DB 테스트 `tests/db/assist-grade.test.ts`·`assist-grade-scripts.test.ts`.
- 문서: 이 문서, [ADR](../acknowledge/2026-10-22-adr-assist-lamp-grade.md), `docs/reference/ir-api.md`, `docs/reference/ai-features/server.md`, `docs/backend/{data-model,api-spec}.md`, `docs/web/architecture.md` 3-2, `docs/HANDOFF.md` 미해결 8번(해소). `docs/play-options.md` 10.2·10.11절, `docs/theme.md`, `docs/skin.md` 는 이 구현 전 상태를 적고 있을 수 있다(이 문서 소유 밖).

---

## 10. 확인하지 못한 것

- 사용자의 실제 로컬 `scoredb.sqlite` 에 `assist`·`gauge` 가 어떻게 분포하는지, 운영 DB 의 `score.gauge` 분포(읽지 않았다). 소급 대상 규모는 이 두 곳을 읽기 전용으로 세면 알 수 있다(`play-options.md` 10.4절에 옛 사용자 DB 37판 SCRATCH AUTO 의 기록이 있으나 오래된 값이다).
- LR2·IIDX 본가의 어시스트와 램프 결합 규칙(5.2절).
- beatoraja `ScoreData.assist` 의 쓰임(5.1절 끝).
- 원본 beatoraja 에서 "legacy note" 로 불리는 옵션의 정확한 소스 대응(5.1절 추정).
- (구현 뒤 추가) 새 등급 색 4개를 실제 화면에서 눈으로 대조하지 못했다. 계산으로 고른 값이고 OKLab 거리 12 이상만 스크립트로 확인했다. 다크 테마의 대비도 보지 못했다(램프 토큰은 원래 테마 독립).
- 기본 스킨의 등급 표시 캡처는 헤드리스다. 결과 화면 등급 캡처는 autoplay + 스크래치 레인 자동이라는 앱이 실제로 만들지 않는 조합의 고정물이다(표시 경로 확인용).
- 외부 스킨(ModernChic)에서 어시스트 클리어가 LIGHT ASSIST(id 3) 그대로 보이는지 실제로 확인하지 않았다(설계상 0~10 만 준다).
- `ASSIST_GRADE_RULE` 영어 문장과 프롬프트 버전 2 의 모델 출력 품질을 실제 공급자로 확인하지 못했다(가짜 공급자 테스트만).
- 서버 SCRATCH AUTO 칸의 최고 기록 재선정은 하지 않았다(D6). 필요하면 `chart-notes.repair.sql` 형태의 별도 복구 스크립트로 만들 수 있다.
- `course_best.assist_grade` 는 항상 0 인 죽은 열이다. 코스에 등급을 줄지(코스 제출·`course_score` 에 게이지 추가) 열을 지울지는 사용자 결정이다.
- `chart_best_rank_idx` 와 `chart_best_rank_grade_idx` 는 앞부분이 겹치는 중복 색인이다. 옛 코드와의 공존이 끝나면 후속 마이그레이션에서 옛 것을 지우면 된다.
- 계약 테스트의 기존 결함: `replay-long`·`replay-random` 골든의 `score_id`(`sc_3`)가 캡처 서버 요청 수에 따라 달라져 `RBMS_UPDATE_CONTRACT=1` 실행 때 `sc_4` 로 바뀔 수 있다(수정 전부터 있던 시간 의존 동작이라 손대지 않았다).
