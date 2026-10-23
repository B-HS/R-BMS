# ADR: 코스 제출에 게이지 종류·게이지 규칙·스테이지 사실을 싣고 course_best.assist_grade 를 채움

> 최종 갱신 2026-10-23 · 대응 커밋 5a0eacf
> 상태: 유효(결정 6 은 사용자 확인 2026-10-23: "1-11 추천대로". 와이어 모양·필드 이름·스테이지 항목의 구성은 메인 결정이며 사용자 미확인) · 결정일 2026-10-23 · 결정자 사용자(결정 6), 세부 메인 · 커밋 47e8f70(클라이언트 본문·계약 골든), db534d1(웹 DTO·저장·등급), 9e39262(마이그레이션 0012 의 `course_score` 열. 웨이브 1 전체가 섞인 커밋)
> 관계: [2026-10-22 어시스트 등급 ADR](2026-10-22-adr-assist-lamp-grade.md) 6절의 마지막 항목("`course_best.assist_grade` 는 항상 0")과 영향 절의 "쓰지 않는 열" 항목을 이 ADR 이 **대체**한다(그 파일에 표시함). 나머지 6절(순위 키, 응답 필드)은 그대로다.

## 배경

- 2026-10-22 ADR 6절: 코스 제출 본문과 코스 DB 에 게이지가 없어 `course_best.assist_grade` 를 파생할 수 없었고, 열은 있으나 쓰이지 않았다. 코스 최고 기록의 등급 정책은 따로 정해야 한다고 적었다.
- 단위 인정(결정 20)은 판이 어떤 스테이지를 어떤 RANDOM·LN 종류·게이지로 쳤는지를 알아야 한다. 코스 제출 본문에는 이 사실이 없었다(`CourseSubmission` 에는 게이지 종류·RANDOM·코스 게이지 규칙이 없었다).
- 결정 6: 코스 제출에 게이지 종류를 더해 `course_best.assist_grade` 를 채운다.

## 결정

### 1) 와이어: 코스 제출 본문의 세 필드 (모두 생략 가능)

```json
"course_gauge": "LR2",
"gauge": "Class",
"stages": [ { "gauge": "Class", "random": "Off", "lntype": 0 }, { "gauge": "Class", "random": "Off", "lntype": 0 } ]
```

| 필드 | 뜻 |
| --- | --- |
| `gauge` | 코스가 끝난 게이지: 마지막으로 친 스테이지의 값. 와이어 이름은 `Class`·`ExClass`·`ExHardClass` 등 `GaugeType` 그대로 |
| `course_gauge` | 코스를 친 COURSE GAUGE 규칙: `LR2`·`BEATORAJA`·`IIDX`([코스 게이지 ADR](2026-10-23-adr-course-gauge-modes.md)) |
| `stages` | 친 스테이지마다 하나, 친 순서대로. `gauge`(그 스테이지가 끝난 게이지), `random`(코스 제약이 좁힌 뒤의 실제 값), `lntype`(0 LN, 1 CN, 2 HCN). 비면 생략 |

- 값은 각 스테이지 자신의 점수 본문 `options` 에서 복사한다(`CourseStagePlay::of`). 스테이지를 접는 세 경로(결과 진입, 앱 종료로 끊김, 코스 도중 Esc) 모두 같다. 스테이지를 하나도 접지 못한 코스는 보내지 않는다.
- 최상위 `lntype`: LN 제약이 있으면 그 값, 없으면 첫 스테이지의 값(이제 LN 제약이 실제로 적용되므로 사실과 같다).
- 계약 골든: `contract/client-requests/course-normal.json`, `course-scratch-auto.json` 에 세 필드가 들어갔다.

### 2) 서버: DTO, 저장, 등급

- `courseSubmissionSchema` 에 `gauge`(없으면 null), `course_gauge`(`COURSE_GAUGE_RULES`), `stages`(최대 `COURSE_CHARTS_MAX` = 64). `stages[].lntype` 은 0~2 로 좁히지 않는다. 범위 밖 값이 판 전체를 400 으로 버리면 안 된다는 기록 원칙이다. 판정에서만 쓴다.
- `course_score` 에 `gauge tinyint NULL`, `course_gauge varchar(16) NULL`, `stages json NULL`(마이그레이션 0012, 추가만). 저장되는 `stages` 항목은 제출의 `charts[index]` 의 md5·sha256 을 붙인 `StoredCourseStage` 다.
- `courseAssistGrade({ clear, gauge })`: `gauge` 가 null 이면 등급 0, 아니면 `deriveAssistGrade` — 램프가 LIGHT ASSIST 일 때만 CLASS 2, EXCLASS 3, EXHARDCLASS 4. 이 값을 `course_best.assist_grade` 에 저장하고, 코스 순위·최고 기록 비교 키는 `(clear, assist_grade, EX, BP)` 다(`isBetterBest`·`isRankedAheadOf`는 이미 등급을 쓰고 있었다). 같은 판의 재전송은 처음 저장된 `gauge` 로 등급을 다시 만든다.
- IR 응답(`SubmitResponse`)은 바뀌지 않는다. 단위 판정이 쓰는 `scoreRow` 만 서비스가 더 돌려준다([단위 ADR](2026-10-23-adr-dan-auto-award.md)).

### 3) 호환

| 조합 | 결과 |
| --- | --- |
| 새 클라이언트 + 0012 이전의 옛 서버 | 서버 DTO 가 `z.object` 라 모르는 세 키를 버리고 400 을 내지 않는다. 등급 0, `stages` 없음 → 그 판은 단위 소급 불가 |
| 옛 클라이언트(v0.1.7 이하) + 새 서버 | 세 키가 없다 → `gauge` NULL, 등급 0, 단위 판정 `no_stage_facts`. 나머지는 그대로 |
| 옛 클라이언트가 보관한 코스 본문의 재전송 | serde 기본값으로 읽혀 세 필드 없이 다시 나간다 |
| 옛 행 | `gauge` NULL 그대로, 등급 0, 백필 없음(게이지 정보가 없어 채울 값이 없다) |

- 순서: 마이그레이션 0012(운영 적용 web-db run 39909099575) → 웹 DTO(`db534d1`) → 계약 골든. 웹 `client-requests.test.ts` 가 서버가 버린 키를 실패로 보기 때문에 골든을 바꾼 푸시는 DTO 와 같거나 뒤여야 한다. 클라이언트 릴리스는 웹 배포 뒤(`release.yml` 은 사용자 실행, 이 ADR 의 범위 밖).
- Rust `CourseSubmission` 은 `deny_unknown_fields` 가 없고 새 필드가 모두 `serde(default)` 라 옛 빌드가 새 본문을 읽어도 무시한다.

## 이유

- 판이 끝난 게이지를 모르면 SCRATCH AUTO 코스 클리어를 단일 곡과 같은 정책(어시스트 등급)으로 줄 세울 수 없다. 스테이지별 사실이 없으면 단위 인정의 제약 준수(결정 20)를 서버가 확인할 방법이 없다.
- 세 필드를 모두 생략 가능한 추가 필드로 두면 구 서버·구 클라이언트·보관 본문이 모두 그대로 동작한다.

## 기각된 대안

| 대안 | 기각 이유 |
| --- | --- |
| `course_best` 키에 게이지 규칙을 넣어 규칙별 순위 | 마이그레이션과 IR 순위 와이어 변경이 필요하다. `course_score.course_gauge` 에 기록만 한다(설계서 web 8절 결정 15) |
| 스테이지 `lntype` 을 0~2 로 검증 | 범위 밖 값 하나가 판 전체를 거절한다(기록 원칙) |
| 옛 행의 등급을 램프로 추정해 백필 | 게이지를 알 수 없다 |

## 영향과 관련 파일

- 순위 의미: SCRATCH AUTO 코스 클리어끼리의 순서가 게이지 등급을 따르게 된다(교차 검토서 J-12, 결정 6 의 직접 결과). 실제 순위 변화는 사용자 실기로만 확인된다.
- 확인하지 못한 사실: 운영 서버가 이 DTO 를 배포한 상태인지와 새 필드를 실은 코스 판이 운영에 도착했는지(이 작업에서 `/api/version`·운영 DB 를 보지 않았다).
- 코드: `crates/rbms-ir/src/{dto/score,mapping,lib}.rs`, `apps/rbms-player/src/{course_ir,app_result,lib}.rs`, `web/src/server/{dto/course,service/domain/course/course.service,compose/course,db/schema}.ts`, `web/drizzle/0012_course_dan_tables.sql`, `contract/client-requests/course-*.json`.
- 시험: `rbms_ir::dto::score::tests::{course_gauge_types_have_their_wire_names, a_course_stage_play_round_trips, a_course_body_without_the_gauge_fields_reads_back_and_writes_them_out_again_absent}`, `course_ir::tests::{a_course_submission_states_its_finished_gauge_and_gauge_rule, a_course_submission_lists_every_stage_played_in_order, a_course_without_an_ln_constraint_reports_the_lntype_its_first_stage_played}`, `record_sync::tests::a_course_kept_by_an_older_build_is_sent_without_the_gauge_fields`, `web/tests/service/course.test.ts`, `web/tests/dto/client-requests.test.ts`.

## 날짜와 사용자 원문

2026-10-23.

- 결정 6 은 "1-11 추천대로" 에 포함된다(`docs/PROCESS.md` 에 기록된 원문. 질문 원문은 저장소에 없고, 결정 6 의 내용은 PROCESS 의 요약 "코스 제출에 게이지 종류를 더해 `course_best.assist_grade` 를 채움" 이다).
- 운영 반영 승인: "3번은 승인 그냥 바로 진행해놔" (마이그레이션 0012 운영 적용과 Vercel 환경 변수 등록. 클라이언트 `release.yml` 은 이 승인에 들지 않는다).
