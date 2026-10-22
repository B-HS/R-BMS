# ADR: SCRATCH AUTO 클리어의 어시스트 등급 (ASSIST EASY / CLEAR / HARD / EXHARD)

> 최종 갱신 2026-10-22 · 대응 커밋 47a7f9c
> 상태: 유효(사용자 미확인) · 결정일 2026-10-22 · 결정자 메인(사용자 지시 "끝까지 다 구현해놔"에 따라 추천안 채택, 사용자 미확인. 운영 DB 마이그레이션 적용만 사용자가 승인) · 커밋 4d57c2a(클라이언트 코어), 8d85b32(웹 마이그레이션 0011), 64c6120(웹 등급), 242397f(코스 스테이지), 47a7f9c(웹 AI 정렬·백필 액션)
> 관계: [2026-10-14-adr-scratch-auto-rule.md](2026-10-14-adr-scratch-auto-rule.md) 결정 4(SCRATCH AUTO 클리어의 램프는 LIGHT ASSIST)를 대체하지 않고 확장한다. 램프 id 3 과 그 강등 규칙은 그대로이고, 그 안을 등급으로 나눈다.

## 배경

- 사용자 지시(2026-10-22, 요지): 클리어 램프를 세분화한다(ASSIST HARD, ASSIST EXHARD 등). 원문이 아니라 `docs/PROCESS.md` 의 요지다.
- 그때까지 SCRATCH AUTO 로 클리어하면 게이지와 상관없이 LIGHT ASSIST(id 3, 와이어 `LightAssistEasy`) 하나였다. HARD 로 깼다는 사실은 `scorelog.gauge`·`assist` 열에만 남았다([clear-lamps.md](../reference/clear-lamps.md) 1.5절).
- 램프 id 는 정렬 키다. 클라이언트는 최고 램프를 id 의 최댓값으로 접고, 웹은 순위를 `clear desc, ex_score desc, minbp asc` 로 매겼다. 와이어는 문자열 enum 이라 모르는 이름이 한 줄이라도 있으면 구 클라이언트가 응답 전체를 못 읽고 구 서버는 400 으로 막는다. 외부 Lua 스킨은 램프 11칸(0~10)을 전제한다(같은 문서 0절 4~6항).
- 램프 이름이 세 벌이었다. 앱 화면은 id 2 "ASSIST EASY"·id 3 "LIGHT ASSIST", 기본 스킨은 "ASSIST CLEAR"·"LIGHT ASSIST CLEAR", 웹은 "ASSIST"·"L-ASSIST". [HANDOFF.md](../HANDOFF.md) 6절 8번이 앱과 스킨의 두 벌을 미해결로 적고 있었다.

## 결정

### 1) 구조: 호환 id 를 유지하고 LIGHT ASSIST 안에 등급을 덧붙인다

- 램프 id 0~10, 와이어 이름, 강등 규칙, `SCORE_RULE_VERSION`(4), 점수 제출 본문은 바뀌지 않는다.
- 램프 id 3(LIGHT ASSIST)을 가진 판에만 어시스트 등급 0~4 가 있다. 최고 기록과 순위의 비교 키는 `(램프, 등급, EX, BP)` 다. 등급은 id 3 안에서만 갈려서 어떤 등급도 EASY(id 4) 위로 올라가지 못한다.
- 정의는 한 곳이다: 엔진 `rbms_judge::assist_grade`(`crates/rbms-judge/src/gauge.rs:229-243`), 저장 쪽 정수 거울 `ASSIST_GRADE_OF_GAUGE`(`crates/rbms-store/src/score.rs:103`), 서버 `deriveAssistGrade`(`web/src/server/service/domain/score/assist-grade.ts`). 엔진과 저장 거울이 모든 램프 x 게이지에서 같은 답을 내는지는 `format::tests::the_score_store_grades_every_stored_run_as_the_engine_does`(`apps/rbms-player/src/format.rs:426`)가, 서버 표와 마이그레이션 SQL 의 CASE 가 같은지는 `web/tests/service/assist-grade.test.ts` 가 지킨다.

### 2) 등급의 정의: 강등되기 전에 그 게이지가 줬을 램프

| 등급 id | 이름 | 끝난 게이지 |
| --- | --- | --- |
| 0 | LIGHT ASSIST | ASSIST EASY 게이지, 게이지를 모르는 판. 램프가 id 3 이 아닌 모든 판도 0 |
| 1 | ASSIST EASY | EASY |
| 2 | ASSIST CLEAR | NORMAL, CLASS(코스 게이지) |
| 3 | ASSIST HARD | HARD, EXCLASS |
| 4 | ASSIST EXHARD | EX-HARD, HAZARD, EXHARDCLASS |

- 풀콤 계열 덮어쓰기(FULL COMBO·PERFECT·MAX)는 어시스트에서 나오지 않으므로 무시한다. HAZARD 는 EX-HARD 에 접는다.
- 등급 1 의 이름 "ASSIST EASY"는 EASY 게이지에서 강등된 클리어다. ASSIST EASY 게이지로 클리어한 판은 등급 0 이고 LIGHT ASSIST 로 보인다. 그 게이지는 SCRATCH AUTO 없이 깨도 램프가 id 3 이라 "강등된 클리어"와 구분되지 않기 때문이다(`gauge.rs:219-228` 문서).
- 조사 문서 8절 D2 A 는 ASSIST EASY 게이지를 EASY 등급에 합친다고 적었다. 구현은 합치지 않고 0 으로 둔다. 코드가 정본이다(`gauge.rs` 테스트 `GRADE_OF_GAUGE`, 별도 결정 기록은 없다).
- 램프 id 2(넓힌 판정·TOTAL 덮어쓰기)는 등급이 없다. 항상 unranked 이고 최고 기록도 못 세워 나눌 실익이 없다(조사 8절 D4).

### 3) 이름 통일 (HANDOFF 6절 8번 해소)

- 앱(`apps/rbms-player/src/format.rs`), 기본 스킨(`assets/skins/rbms-default/lua/theme.lua:166-184`), 웹(`web/src/shared/constants/clear-lamp.ts`)이 같은 이름을 쓴다: id 2 = `ASSIST`, id 3 등급 0 = `LIGHT ASSIST`, 등급 1~4 = `ASSIST EASY`·`ASSIST CLEAR`·`ASSIST HARD`·`ASSIST EXHARD`. 웹의 약칭 `L-ASSIST` 는 없앴다. 좁은 칸은 약칭을 만들지 않고 전체 이름을 `truncate` 로 둔다.
- 따라서 HANDOFF 6절 8번("램프 이름 두 가지: 통일할지")은 이번에 해소됐다. 통일 자체는 메인이 정했고 사용자는 이름 안을 보지 못했다(미확인).
- 등급 색 4개는 앱 `format.rs:138-141` 의 상수와 웹 `globals.css` 가 같은 hex 다: `#3ad19f`(EASY), `#c4a796`(CLEAR), `#afb8ef`(HARD), `#ca76a9`(EXHARD). 등급 0 은 기존 light-assist 색을 쓴다. 각 게이지 램프색을 light-assist(`#6563ff`)와 OKLab 에서 35% 섞어 정했고 기존 11색과의 OKLab 거리가 모두 12 이상임을 스크립트로 확인했다. 앱 상수가 웹 hex 와 같은지 보는 테스트가 있다. 눈으로 대조하지는 않았다.

### 4) 클라이언트의 저장·비교·소급

- 비교: `merge_best` 는 `best.lamp() < log.lamp()`(`RecordLamp` = 램프 id, 등급 순서)일 때 `clear`·`assist_grade`·`option`·`seed`·`gauge`·`assist` 를 그 판의 값으로 바꾼다(`crates/rbms-store/src/scoredb.rs:653-662`). 같은 id 3 안에서 등급이 오른 판도 따라간다. `ScoreDb::best` 는 `clear DESC, assist_grade DESC, EX DESC, minbp ASC`, `chart_best_many` 는 `MAX(clear * 5 + assist_grade)` 로 접는다(`scoredb.rs:482, 500`).
- 저장: `score` 테이블에 `assist_grade INTEGER NOT NULL DEFAULT 0` 열(`schema.sql:40`, 기존 열 추가 방식 `ADDED_COLUMNS` `schema.rs:68`). `SCORE_DB_SCHEMA_VERSION` 은 1 그대로, 규칙 세대 `SCORE_RULE_VERSION` 도 4 그대로다. `ScoreRecord` 에 `assist`(serde 기본값 -1 = 기록 안 됨)를 더했다.
- 소급: `refold_assist_grades` 가 DB 를 열 때마다(`schema::migrate`, `schema.rs:50`) 돈다. 램프 id 3 의 `scorelog` 판 가운데 같은 차트·모드·LN 방식의 최고 등급을 고르고(동률이면 먼저 친 판), 등급이 다를 때만 `score.assist_grade` 를 쓰며 등급이 0 보다 크면 그 판의 `gauge`·`assist` 도 쓴다. 규칙 세대 2 미만 행과 `assist = -1` 행은 등급이 없다. id 3 이 아닌 행은 0 으로 정리한다. 멱등이며 구 빌드가 행을 다시 써서 0 이 되어도 다음 실행에서 복구된다. 내부 스캔은 `WHERE l.clear = ?1`(LIGHT ASSIST 행)로 좁혔다(`scoredb.rs:783`).
- 표시: 결과 화면·선곡 상세·기록 목록·기록 모달·내장 선곡 막대 LED 가 등급 이름과 색을 쓴다. 선곡 정렬 Clear 는 `(clear, 등급)` 순이다. 선곡 `CLEARED` 필터는 NO PLAY·FAILED 가 아닌 모든 램프(`ClearType::is_cleared`, id 2 이상)로 맞췄다. 웹의 2026-10-19 기준([웹 ADR](2026-10-19-adr-web-clear-and-play-history.md))과 같아졌다.
- IR 순위 패널은 서버 응답의 `assist_grade` 를 읽어 `RankingRow.record_lamp` 에 담고 램프 분포를 라벨 문자열이 아닌 램프 id 로 센다(`apps/rbms-player/src/ir_ranking.rs`, `skin_host/ir.rs`).

### 5) 코스 스테이지는 실제로 끝난 CLASS 계열 게이지로 저장·전송한다

- 코스 스테이지는 언제나 CLASS 계열 게이지로 달린다(`apps/rbms-player/src/app_play.rs:578`, `GaugeIndex::Class`). 결과 화면의 등급은 `record.finished_gauge` 로 계산했지만 저장·전송 `gauge` 는 설정의 게이지(`reported_gauge`)였다. 설정이 EXHARD 이고 SCRATCH AUTO 로 코스를 치면 결과 화면은 ASSIST CLEAR 인데 DB 와 서버는 ASSIST EXHARD 를 기록했다(통합 리뷰 lamps-1).
- 지금은 `scorelog.gauge` 에 `GaugeIndex` 번호를 그대로 쓴다(0~8, `GAUGE_IDS` 순서). 제출 `options.gauge` 는 `rbms_ir::mapping::ir_gauge_of_index` 로 Class·ExClass·ExHardClass 를 보낸다. GAUGE AUTO SHIFT 로 바뀐 판은 바뀐 뒤의 게이지다. 기록 동기화의 재전송 본문(`record_sync::rebuilt_submission`)과 소급 trace(`trace_backfill`)도 같은 규칙이다.
- 이미 저장된 스테이지 행: DB 를 열 때 `put_course_stages_on_class`(`crates/rbms-store/src/scoredb/courses.rs:126`)가 보관된 코스의 스테이지 행(`trace_backfill::course_stages`)을 CLASS(6)로 옮기고 등급을 다시 접는다. 6 미만인 행만 옮기므로 반복해도 쓰지 않는다. 보관되지 않은 코스의 스테이지는 알 수 없어 그대로 남는다.
- 리플레이 헤더의 게이지는 설정 게이지 그대로 둔다. 재생이 시작하는 게이지이고 `gauge_from_name` 에 코스 게이지가 없다.

### 6) 서버(웹)

- `deriveAssistGrade({ clear, gauge })` 가 단일 정의다. 규칙 세대와 `scratch_auto` 는 입력으로 받지 않는다. 세대 2 미만 행은 강등 전 램프를 저장했으므로 "LIGHT ASSIST 인데 게이지가 ASSIST EASY 가 아닌" 옛 행이 서버에 생길 수 없다. 모르는 게이지 id 는 0 이다.
- 마이그레이션 `web/drizzle/0011_assist_grade.sql`: `chart_best`·`course_best` 에 `assist_grade tinyint DEFAULT 0 NOT NULL`, 인덱스 `chart_best_rank_grade_idx (chart_sha256, scratch_auto, clear, assist_grade, ex_score)`, 마지막 문장은 `chart_best` 를 가리키는 `score` 의 램프·게이지로 등급을 채우는 백필 UPDATE 다(`updated_at` 불변). 앞 세 줄은 drizzle 이 만든 그대로이고 UPDATE 한 문장만 수동 SQL 이다. 기존 인덱스 `chart_best_rank_idx` 는 지우지 않았다(additive).
- 순위·최고 기록 키는 `(clear, assist_grade, EX, BP)` 다. `web/src/server/compose/best-scope.ts` 의 `BEST_ORDER` 표 하나로 `outranks`·`isBeatenBy`·`isRankedAheadOf`·`rankInList`·`listOrder` 와 `best-policy.ts` 의 `isBetterBest` 를 만든다. SCRATCH AUTO 칸의 기존 순위가 이 키로 다시 매겨진다.
- 응답에 `assist_grade`(0~4, 최상위 정수)를 더했다: 차트·코스 순위 행, 플레이어 점수 목록, 활동 항목, 플레이어 통계 `lamps[]`, 표 기록, AI 응답. 제출 본문과 `SubmitResponse` 는 바뀌지 않는다. Rust 쪽 `ScoreRecord` 는 `#[serde(default)] assist_grade: u8` 이고 `deny_unknown_fields` 가 없어 구 클라이언트는 무시한다.
- AI: 추천·목표·연습·주간·기록 질문의 프롬프트 버전 5개를 1 에서 2 로 올렸다(캐시된 결과는 한 번 다시 생성된다). 기록 질문의 lamp 정렬은 `(clear, assist_grade)` 순이고(점수 source 는 `score.gauge` 에서 파생하는 SQL CASE), 비교 합계(`judgeTotalsSchema`)도 등급을 싣는다.
- `course_best.assist_grade` 는 항상 0 이다. 코스 제출 본문과 코스 DB 에 게이지가 없어 파생할 수 없다. 공유 비교 코드가 균일해지는 이점으로 열은 추가했지만 지금은 쓰이지 않는 열이다.

### 7) 운영 적용

- 마이그레이션 0011 은 사용자 승인("마이그레이션도 니가하고", 2026-10-22) 뒤 `web-db.yml` 로 적용했다(run 39693988962, 성공). 웹 코드 `64c6120`, 이어서 `47a7f9c` 를 배포했다.
- 0011 적용과 새 코드 배포 사이에 옛 코드가 `chart_best` 행을 바꾸면 등급이 낡은 채 남을 수 있어, `web-db.yml` 에 `assist-grade-backfill-count`(세기만)와 `assist-grade-backfill`(세기, 보정, 세기)을 더했다(`web/scripts/assist-grade.{count,backfill}.sql`). 보정은 `WHERE` 로 바뀔 행만 건드려 멱등이다.
- 배포 뒤 `assist-grade-backfill-count` 를 돌린 결과(run 39733088817, 2026-10-22 06:18 UTC): 낡은 `chart_best` 행 0, 등급이 0 보다 큰 `chart_best` 398행, 점수 행이 사라져 보정이 닿지 않는 행 0.

### 8) 외부 Lua 스킨에는 0~10 만 준다

- 외부 스킨이 받는 램프 값(이미지 인덱스 370·371, 막대 램프, 폴더 분포 11칸, 옵션 1100~1104)은 그대로다. 등급이 있는 클리어도 id 3 이다. `SelectBar.lamp` 는 `Option<u8>` 그대로이고 등급은 `SelectBar.assist_grade` 에 따로 있다.
- 기본 스킨만 확장 옵션 21110~21134 로 등급을 읽는다([확장 id ADR](2026-10-22-adr-rbms-extension-ids.md)). 이 id 를 모르는 스킨은 아무 영향이 없고 새 번역 계층도 필요 없다.

## 이유

- id·와이어 이름·강등 규칙을 그대로 두면 이미 배포된 클라이언트(v0.1.2~v0.1.6), 구 서버, 11칸을 전제한 외부 스킨, beatoraja 의 강등 규칙과 모두 호환된다. 제출 본문이 안 바뀌므로 계약 골든은 새 파일 하나(`contract/client-requests/score-scratch-auto-hard.json`)만 늘었다.
- 필요한 입력(끝난 게이지, 어시스트 단계, 강등된 램프)이 `scorelog` 와 서버 `score` 에 이미 판마다 저장돼 있어 새 열 하나와 다시 접기로 기존 기록까지 소급된다.
- 등급이 id 3 안에만 있어 "어시스트 판은 정상 판을 넘지 못한다"는 성질과 정수 최댓값 접기가 그대로 안전하다.
- 코스는 결과 화면이 이미 `finished_gauge` 로 등급을 보여 주므로 저장과 전송을 같은 값에 맞춰 화면·DB·서버가 일치한다.

## 기각된 대안

조사 문서 [clear-lamps.md](../reference/clear-lamps.md) 6절의 대안이다.

| 대안 | 기각 이유 |
| --- | --- |
| 선형 id 끼워 넣기(a1): id 3 과 4 사이에 4개를 넣고 4~10 을 4 씩 민다 | 와이어에 새 이름이 생겨 구 클라이언트의 응답 디코드 실패와 구 서버의 400 을 진다. 양쪽 DB 일괄 UPDATE 와 외부 스킨용 0~10 번역 계층이 필요하다 |
| 11 이상으로 덧붙이기(a2) | id 11 이 MAX(10)보다 커서 `max(clear)` 와 `ORDER BY clear` 가 ASSIST HARD 를 최고로 만든다. 모든 비교를 순위표로 바꿔야 해 사실상 별도 순위 열이 필요하다 |
| 램프와 어시스트를 직교 필드로 분리(b): 램프는 비강등, 어시스트는 별도 축 | 3 으로 강등되어 저장된 판의 풀콤 정보가 없어 소급이 불완전하다. beatoraja 강등 규칙과 어긋나 IR 의 id 의미가 바뀌고, 합산 순위에서 SCRATCH AUTO HARD 가 일반 NORMAL 을 이긴다 |
| 표시 전용(d) | 선곡 최고 램프와 웹이 계속 LIGHT ASSIST 라 "내 최고는 ASSIST HARD"가 어디에도 안 나온다. 요청의 절반만 충족한다 |
| 넓힌 판정·TOTAL(id 2)도 세분(D4 B) | 항상 unranked 이고 최고 기록을 못 세운다. 원인 구분이 판별 기록 열만으로는 안 된다 |
| HAZARD 를 별도 등급(D2 C) | 어시스트 상황에서 HAZARD 를 구별할 근거가 없다고 조사 문서가 적었다 |
| 원본의 다른 어시스트 원인(H-RANDOM, ALL-SCRATCH 등)도 LIGHT ASSIST 로 셈(D11) | 요청 범위 밖이고, 세면 기존 기록과 서버 ranked 정책이 함께 바뀐다. 이번에는 하지 않았다 |

## 영향과 관련 파일

- 엔진·저장: `crates/rbms-judge/src/{gauge,lib}.rs`, `crates/rbms-store/src/{score,lib}.rs`, `crates/rbms-store/src/scoredb.rs`, `scoredb/{schema.rs,schema.sql,rows.rs,courses.rs,migrate.rs}`, `crates/rbms-ir/src/{mapping.rs,dto/score.rs}`.
- 앱: `apps/rbms-player/src/{format,app_result,scoredb_store,trace_backfill,record_sync,ir_ranking,app_input}.rs`, `stage/{result,play_menu}.rs`, `stage/select/{list,scene}.rs`, `skin_host/{ir,select,result}.rs`.
- 기본 스킨: `assets/skins/rbms-default/lua/{theme,parts}.lua`, `lua/select/info.lua`, `lua/result/score.lua`.
- 웹: `web/drizzle/0011_assist_grade.sql`, `web/src/server/{db/schema.ts,dto/common.ts,compose/best-scope.ts,service/domain/score/assist-grade.ts}`, `web/src/shared/constants/clear-lamp.ts`, `web/src/app/globals.css`, `web/src/features/lamp/*`, `web/scripts/assist-grade.{count,backfill}.sql`, `.github/workflows/web-db.yml`.
- 테스트: `gauge::assist_grade_tests`, `format::tests::the_score_store_grades_every_stored_run_as_the_engine_does`, `app_result/record_tests.rs` 의 `a_course_stage_is_stored_and_sent_on_the_course_gauge_it_ran_on`, `stage/select/bars_tests.rs` 의 `a_graded_light_assist_clear_lights_the_built_in_row_in_its_grade`, `web/tests/service/assist-grade.test.ts`, `web/tests/db/assist-grade.test.ts`, `web/tests/db/assist-grade-scripts.test.ts`.
- 사용자 확인이 필요한 것(미확인):
  - 웹 순위 키가 `(clear, 등급, EX, BP)` 로 바뀌어 SCRATCH AUTO 칸의 기존 순위가 달라진다. 조사 문서 8절 D5 는 이 변경에 사용자 확인이 필요하다고 적었다.
  - ASSIST EASY 게이지를 등급 0 에 두는 것(EASY 등급에 합치는 안과 다르다), HAZARD 를 EXHARD 로 접는 것, 이름 통일.
  - `course_best.assist_grade` 를 쓰지 않는 열로 두는 것. 코스 최고 기록의 등급 정책은 따로 정해야 한다.
- 남은 위험:
  - 서버 백필은 `chart_best` 가 가리키는 `score` 의 값만 채운다. 같은 차트에서 게이지가 다른 SCRATCH AUTO 판이 둘 이상이면 `(clear, 등급, EX, BP)` 가 더 높은 다른 판이 있을 수 있고, 그 사용자의 best 는 더 높은 판을 제출할 때 바뀐다. 최고 기록 재선정은 하지 않았다.
  - 이미 서버에 올라간 코스 스테이지 판은 설정 게이지로 남아 서버 쪽 등급이 높게 매겨질 수 있다. 서버 보정이나 재전송은 하지 않았다.
  - 구 빌드가 같은 DB 를 열면 `score` 행을 다시 쓰면서 `assist_grade` 가 0 이 되어 그 사이 구 빌드 화면에는 등급이 안 보인다. 다음 실행에서 복구된다.
  - `chart_best_rank_idx` 와 `chart_best_rank_grade_idx` 는 앞부분이 겹친다. 옛 코드와의 공존이 끝나면 후속 마이그레이션에서 옛 인덱스를 지울 후보다.
  - 등급 색은 계산으로 골랐고 웹은 브라우저에서, 앱은 실제 창에서 눈으로 확인하지 않았다(기본 스킨은 헤드리스 캡처로 확인).
- 문서: [clear-lamps.md](../reference/clear-lamps.md) 는 조사 시점(0323bd9)의 서술이라 구현 뒤의 값과 다른 곳이 있다(1.1절 이름 표, 1.6절 코스 게이지, 8절 D2).

## 날짜와 사용자 원문

2026-10-22.

- 지시 요지(`docs/PROCESS.md` 의 기록, 원문 아님): "(3) 클리어 램프 세분화(ASSIST HARD·ASSIST EXHARD 등)".
- 설계 확정 지시: "끝까지 다 구현해놔"
- 운영 마이그레이션 승인: "마이그레이션도 니가하고"
