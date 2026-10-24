# ADR: 코스 어시스트 클리어를 A-단위로 따로 기록하고 웹에서 보인다

> 최종 갱신 2026-10-24 · 대응 커밋 `35f77ae`(마이그레이션 0013 파일. 서버·웹 화면·문서 커밋은 이 문서를 쓸 때 아직 없다. 커밋 뒤 이 줄을 해시로 고친다)
> 상태: 유효(결정 1~5 는 사용자 확인 2026-10-24 "전부 추천대로 그리고 workflow도 사용". 아래 "메인 결정"은 사용자 미확인이고 미확인 목록 28·29번에 모았다) · 결정일 2026-10-24 · 결정자 사용자(요청과 결정 5개), 세부 메인 · 커밋 `35f77ae`(마이그레이션 0013 `0013_assisted_dan`). 운영 DB 에는 아직 적용하지 않았다(적용 순서: 설계서 10절)
> 관계: [단위 자동 인정 ADR](2026-10-23-adr-dan-auto-award.md)의 인정 조건 일부와 표시 규칙, `user_dan` 의 기본 키를 **대체**한다(그 파일에 표시함). 설계서는 [assist-dan](../plan/2026-10-24-assist-dan.md)이고 이 ADR 이 정본이다. 램프 의미는 [어시스트 등급](2026-10-22-adr-assist-lamp-grade.md), 어시스트 코스 제출은 [어시스트 코스 제출](2026-10-14-adr-assisted-course-submission.md).

## 배경

- 사용자 요청(2026-10-24): "추가 기능으로, 코스모드에서 어시스트 클리어를하면 그것도 웹에서 볼수있게해줘 A-단위 이런식으로".
- 2026-10-23 의 단위 판정은 어시스트 판을 모두 거절했다. SCRATCH AUTO 클리어는 램프가 LIGHT ASSIST 라 `not_cleared`, 넓힌 판정·자기 TOTAL 판은 `ASSIST` 플래그 때문에 `ranked = false` 라 `unranked` 였다(설계서 1-1절). 즉 어시스트로 코스를 깬 사람의 기록은 순위에는 있어도 단위에는 흔적이 없었다.
- 어시스트 판은 `ranked = false` 로 저장되는데, 같은 `false` 가 "빌드가 막아서"인 판과 "어시스트라서"인 판을 구별하지 못한다(둘 다 `UNKNOWN_BUILD` 만 남기고, 릴리스 빌드는 허용 목록에 자동 등록되지 않아 대부분의 판이 `UNKNOWN_BUILD` 를 갖는다). 그래서 "순위 조건"을 어떻게 읽을지가 이 기능의 핵심 결정이 된다.

## 결정

### 사용자 결정 5개

PROCESS "어시스트 단위 A-단위" 체크리스트의 요약이다. 질문 원문은 저장소에 없고 답은 "전부 추천대로 그리고 workflow도 사용" 하나다(앞 단위 ADR 과 같은 사정).

| 번호 | 결정 |
| --- | --- |
| 1 | workflow 사용 |
| 2 | 어시스트 범위 = 지금 단위 인정에서 빠지는 판 전부(SCRATCH AUTO, ASSIST·LIGHT ASSIST 램프, 어시스트 옵션). 게이지가 남은 채 끝냈고 미러·LN 제약은 지킨 판 |
| 3 | 정식 단위와 따로 기록. 같은 체계 안에서 정식이 같거나 높으면 정식만, A-단위가 더 높으면 둘 다 보임 |
| 4 | 마이페이지 단위 탭에서 A-단위도 표시 단위로 고를 수 있음 |
| 5 | 스테이지 정보가 있는 판(v0.1.8 이후)은 소급 |

### 1) 데이터 모델 (마이그레이션 0013)

- `user_dan.assisted boolean NOT NULL DEFAULT false` 를 더하고 기본 키를 `(user_id, system, label, assisted)` 로 넓힌다. 한 사용자에게 정식 十段 과 A-十段 이 따로 한 줄씩 있다. `INSERT IGNORE` 는 종류별로 첫 판만 남긴다(전과 같은 규칙이 종류마다 성립).
- `user_dan_display.assisted boolean NOT NULL DEFAULT false` 를 더한다. 기본 키는 `(user_id, system)` 그대로다(체계마다 선택 하나). 행 없음 = 자동, `label` NULL = 숨김(`assisted` 는 `false`), `label` 값 + `assisted` = 그 단위.
- 기존 행은 모두 `assisted = 0`(정식)이 된다. 데이터를 바꾸는 문장은 없다.
- 파일 `web/drizzle/0013_assisted_dan.sql`(`35f77ae`):

```sql
ALTER TABLE `user_dan` ADD `assisted` boolean DEFAULT false NOT NULL;--> statement-breakpoint
ALTER TABLE `user_dan` DROP PRIMARY KEY, ADD PRIMARY KEY(`user_id`,`system`,`label`,`assisted`);--> statement-breakpoint
ALTER TABLE `user_dan_display` ADD `assisted` boolean DEFAULT false NOT NULL;
```

- 기본 키 교체는 **한 문장**이어야 한다. `drizzle-kit generate` 는 `DROP PRIMARY KEY;` 와 `ADD PRIMARY KEY(...)` 를 두 문장으로 내는데, 로컬 MySQL 8.4.11 에서 첫 문장이 `ERROR 1553 Cannot drop index 'PRIMARY': needed in a foreign key constraint` 로 실패했다(`user_dan_user_id_user_id_fk` 가 기본 키의 첫 열 `user_id` 색인을 쓴다). 한 문장이면 이 오류와 `sql_require_primary_key` 환경을 모두 피한다. 0002(`scratch_auto` 를 기본 키에 넣은 선례)와 같은 모양이다.
- 되돌림 마이그레이션은 만들지 않는다. `assisted = 1` 행이 생기면 기본 키를 되돌릴 수 없다(중복).
- 공존 기간(마이그레이션 적용 뒤, 새 코드 배포 전): 옛 코드의 `INSERT`/`INSERT IGNORE` 는 `assisted` 를 보내지 않아 기본값 `false` 인 정식 행으로 정상 동작한다(M0 가 실제 DB 로 확인).

### 2) 판정 (`judgeDanEligibility`)

위에서 아래로 처음 걸리는 것을 낸다. 성공은 `{ eligible: true, kind: 'formal' | 'assisted', assistedBy, grant }`, 실패는 `{ eligible: false, reason }`이다. **한 판은 둘 중 하나만 준다.**

1. `not_a_dan_course`, `guest`: 전과 같다.
2. `unranked`: 판이 `ranked` 이거나, `ASSIST` 플래그가 있고 **어시스트만 빼면 ranked 였을 판**(`rankedWithoutAssist`)이면 통과한다. 어시스트 플래그가 없는데 `ranked = false` 인 판(신뢰하지 않는 빌드 등)은 그대로 거절한다.
3. `not_cleared`: 램프가 NO PLAY·FAILED 이거나 게이지가 0 이하일 때만 거절한다. ASSIST(id 2)·LIGHT ASSIST(id 3) 램프는 여기서 통과한다(하한을 EASY 에서 ASSIST 로 내림).
4. `no_stage_facts`, `not_a_course_gauge`, `stage_count`, `chart_mismatch`, `random_not_allowed`, `ln_mode_mismatch`: 전과 같고 정식·A-단위가 공통으로 지킨다(결정 2 의 "미러·LN 제약은 지킨 판").
5. 종류: `assistedBy` 가 `scratch_auto`(`run.scratchAuto` 또는 `SCRATCH_AUTO` 플래그), `assist_lamp`(램프가 EASY 보다 낮음), `assist_flag`(`ASSIST` 플래그) 중 참인 것이다. 비면 `formal`, 아니면 `assisted`.

- 거절 사유에서 `scratch_auto`·`assisted` 를 뺐다. 더 이상 거절이 아니라 `kind` 와 `assistedBy` 로 나온다. 나머지 열 개는 그대로다.
- **넓힌 판정·자기 TOTAL 로 깬 코스도 A-단위가 된다.** 코스 본문에는 판정 폭·TOTAL 이 따로 없고 클라이언트가 넓힌 판정과 자기 TOTAL 을 `CUSTOM_JUDGE` 태그 → 서버 `ASSIST` 플래그 → ASSIST 램프로 보내기 때문이다. 결정 2 의 "ASSIST 램프, 어시스트 옵션"과 일치하지만, 앞 ADR 의 "`unranked` = 넓힌 판정·TOTAL 덮어쓰기" 서술은 이 판들에 대해 바뀐다.
- 정식을 나중에 얻어도 A-단위 행은 지우지 않는다(행은 독립이고, 보이는 것은 3절 규칙이 정한다). 반대로 A-단위를 가진 뒤 같은 단위의 정식을 얻으면 정식 행이 새로 생긴다.
- 게이지 모드(LR2·BEATORAJA·IIDX)는 전과 같이 묻지 않고 `course_gauge` 에 기록만 한다.

**`rankedWithoutAssist`(어시스트만 빼면 ranked 였던가).**

- 새 판(제출 때): `courseService.submit` 이 `rankBlockingFlags(flags 에서 ASSIST 를 뺀 것)` 이 비고 빌드가 막지 않으면 참으로 계산해 `SubmittedCourseRun` 에 싣는다. `ASSIST` 플래그가 있으면 `buildCourseScoreRow` 가 `course_score.extra.ranked_without_assist` 를 **항상 서버 값으로 덮어** 쓴다(클라이언트 `extra` 의 같은 키는 버려진다). `ASSIST` 가 없으면 이 사실은 쓰지도 쓰이지도 않는다.
- 저장된 판(재전송 경로와 소급): `storedRankedWithoutAssistOf(extra)` 가 불리언일 때만 그 값을 읽고 아니면 `null` 이다.
- **`null`(0013 배포 전에 저장된 0012 이후 판)의 추정**: `ASSIST` 를 뺀 플래그에 순위를 막는 것이 없으면 참으로 본다. 코스 플래그는 `ASSIST`·`SCRATCH_AUTO`·`UNKNOWN_BUILD` 뿐이고 앞 둘은 순위를 막지 않거나(`SCRATCH_AUTO`) 뺀 것이라, 사실상 "빌드가 막지 않았다고 본다"이다.

### 3) 표시 (서버 단일 함수 `shownDansOf`)

모든 공개 응답(프로필 `extra.dans`, 차트 순위 행 `extra.dans`, FE 플레이어 순위 `dans`, 내 단위 `shown`)이 이 함수 하나로 체계마다 보일 단위를 정한다(체계마다 따로 부른다).

| 표시 모드 | 보이는 것 |
| --- | --- |
| `hidden` | 없음 |
| `label` | 고른 (label, assisted) 한 줄. 갖고 있지 않으면 없음 |
| `auto`, 정식 최고 rank ≥ A-단위 최고 rank | 정식만 |
| `auto`, A-단위가 더 높음 | 정식(있으면) 다음에 A-단위 |
| `auto`, A-단위만 있음 | A-단위 하나 |

- 응답 원소는 `{ system, name, label, assisted }`다. **`label` 은 A-단위면 서버가 `A-` 를 붙인 글자**(`A-十段`, `A-発狂初段`, `A-st3`)이고 `assisted` 는 덧붙은 필드다. 접두는 `ASSISTED_DAN_LABEL_PREFIX = 'A-'`, 글자는 `danDisplayLabel` 이 만든다(서버와 웹 선택 목록·획득 표가 같은 함수). `A-` 는 세 언어에서 같은 꼴이다(label 자체가 언어와 무관한 글자). 설명 문구와 용어는 번역한다: 한국어 "A-단위", 영어 "A-Dan", 일본어 "A-段位"(용어집에 추가).
- 출력 순서는 체계 순(`DAN_SYSTEMS`), 체계 안에서 정식 다음 A-단위다. 획득 목록은 rank 순, 같은 rank 는 정식이 먼저다.
- `GET /api/fe/me/dans` 의 `shown` 이 문자열에서 **배열**로 바뀌고(`null` 이 아니라 빈 배열이 "없음"), `display` 에 `assisted` 가 더해지며, `acquired[]` 에 `assisted` 가 더해진다(`label` 은 원래 글자). `PUT /api/fe/me/dans/display` 의 label 모드는 `assisted?: boolean`(기본 `false`)을 받고, 갖고 있지 않은 (label, assisted) 조합은 400 `VALIDATION_ERROR`(`details.field = 'label'`)다. `displayedDanSchema` 의 `assisted` 는 `.default(false)` 라 캐시에 남은 옛 모양도 정식으로 읽힌다.
- 웹 화면(설계서 7절, 구현은 W1 단위): A-단위 배지는 정식과 구별되는 점선 테두리 윤곽 배지와 설명 `title`, 한 체계 두 배지를 위한 `key = system:assisted`, 마이페이지 선택 목록에 정식·A-단위 모두, 규칙 목록에 A-단위 항목. 메시지 키는 `pages.dan.assisted`, `settings.danSystemCard.displayAuto`, `settings.danPanel.rules.{cleared,exclusions,assisted}` 다.

### 4) 소급

- `awardForCourses` 가 2절 판정을 그대로 쓰므로 `dan-recompute` 와 표 갱신의 새 단위 코스 소급이 A-단위도 준다. 같은 판이 두 종류를 주지 않고 `INSERT IGNORE` 라 두 번 돌려도 두 번째는 0 이다.
- 합계가 `{ scanned, eligible, awarded, eligibleAssisted, awardedAssisted, assistedLegacy }` 로 늘었다. `eligible`·`awarded` 는 정식만이다. `assistedLegacy` 는 `ranked = false` 이고 `rankedWithoutAssist = null` 이어서(위의 추정 경로) A-단위 대상이 된 판 수이고 dry run 에서도 센다.
- `dan-recompute` 출력(숫자만, Actions 로그가 공개라서): `plan|applied danCourses N, scanned N, eligible N, awarded N, eligibleAssisted N, awardedAssisted N, assistedLegacy N`. `web-db.yml` 은 바꾸지 않는다.
- 표 갱신 보고의 `awarded` 는 정식과 A-단위를 합한 값이다(캐시 무효화 기준 `awarded > 0` 을 그대로 쓰려고).
- 소급은 Vercel 밖에서 돌아 캐시를 무효화하지 않는다. 프로필·순위의 A-단위는 캐시 수명 뒤에 보인다(전과 같음).
- 운영에서 `dan-recompute` 를 아직 돌리지 않았다면(앞 ADR 의 "확인하지 못한 사실") 이번 실행이 정식 단위의 첫 소급도 겸한다. 숫자를 볼 때 이 점을 같이 본다.

### 5) 클라이언트(R-BMS 플레이어)는 바꾸지 않는다

- 서버가 `label` 에 `A-` 를 넣어 주고 `assisted` 는 추가 필드다. `DisplayedDan`(`crates/rbms-ir/src/dto/score.rs`)은 `deny_unknown_fields` 가 없어 `assisted` 를 무시하고, 목록은 항목마다 `deserialize().ok()` 라 한 항목이 틀려도 나머지를 읽는다. 그래서 게시된 v0.1.8, 아직 draft 인 v0.1.9, 그 뒤의 빌드 모두 순위 오버레이에 `A-十段` 을 그대로 그린다. 와이어 버전·`API_VERSION` 변경이 없다.
- 남는 차이: 오버레이는 줄마다 배지를 최대 2개(`MAX_DAN_BADGES = 2`)만 그려서, 한 체계에서 정식 + A-단위 둘을 보이는 플레이어가 다른 체계의 단위도 보이면 세 번째부터 잘린다(서버 순서: 체계 순, 체계 안에서 정식 → A-단위). A-단위 배지의 색 구분도 없다. 후속으로 `DisplayedDan` 에 `assisted` 를 읽어 색을 바꾸는 클라이언트 단위를 둘 수 있다(이번 범위 밖, 테스트 부채 3.129).

### 메인 결정 (사용자 미확인)

1. **0013 이전 판의 플래그 추정을 받아들인다.** `ranked_without_assist` 를 모르는 0012 이후 판은 "빌드가 막지 않았다"고 본다. **위험**: 운영의 `REQUIRE_BUILD_HASH` 가 켜져 있거나 `client_build` 에 `trusted = false` 로 등록된 빌드가 있으면, 빌드 해시로 막혔던 어시스트 판이 섞여 A-단위를 받을 수 있다. 둘 다 운영에서 확인하지 못했다(설계서 11절 1번, 감사 문서 `docs/web/2026-10-14-audit.md`). **결정**: 받아들이고, 소급을 적용하기 전에 `dan-recompute-count` 의 `assistedLegacy` 숫자를 사용자에게 보여 준 뒤 사용자가 `dan-recompute` 를 실행하게 한다. 이 숫자가 섞일 수 있는 판 수의 상한이다. 새 판은 이 위험이 없다(서버가 사실을 저장한다). 배경: 이전에 클라이언트가 `extra.ranked_without_assist` 를 보냈다면 옛 코드는 클라이언트 `extra` 를 펼쳐 저장해서 그 값이 남아 있을 수 있는데, 추정과 같은 방향(참)이라 차이는 위와 같은 경우뿐이다.
2. **넓힌 판정·자기 TOTAL 코스도 A-단위로 한다.** 위 2절 마지막 주의. 사용자가 "어시스트"를 SCRATCH AUTO 와 램프만으로 생각했을 가능성은 확인하지 못했다.
3. **`label` 모드로 A-단위를 고른 뒤 같은 단위의 정식을 얻어도 선택은 A-단위로 남는다.** 사용자가 고른 것을 서버가 바꾸지 않는다.
4. **클라이언트가 `extra.flags` 로 자기 판을 A-단위로 낮출 수 있다.** 서버 플래그가 비어 있는 판(신뢰 빌드, 어시스트 없음)은 클라이언트 `extra` 를 그대로 저장하므로, 클라이언트가 `extra.flags` 에 `ASSIST` 를 넣으면 소급 때 같은 label 의 A-단위 행이 하나 더 생길 수 있다. 자기 판의 종류를 스스로 낮추는 것뿐이고(표시에는 정식만 보인다) 서버 플래그가 있는 판은 서버 값이 덮으므로 고치지 않았다(테스트 부채 3.128).

## 이유

- 정식과 같은 표에 두면서 `assisted` 를 키에 넣는 방식(A안)은 표시·선택·정렬이 두 종류를 함께 다루는 결정 3·4 에 맞고, 기존 `scratch_auto` 기본 키(0002)와 같은 모양이며, 공존 기간에도 옛 코드가 깨지지 않는다.
- 어시스트만 빼면 ranked 였던 사실을 새 판에 저장하는 것이, 저장된 `ranked = false` 하나로는 가를 수 없는 "빌드가 막음"과 "어시스트라서"를 구별하는 가장 작은 방법이다. 클라이언트 값을 믿지 않고 서버 값으로 덮는다.
- 한 판이 한 종류만 주게 하면 정식 조건을 다 갖춘 판이 A-단위를 따로 주는 일이 없고, 소급이 멱등이다.
- 서버가 `label` 에 `A-` 를 넣으면 이미 배포된 클라이언트도 A-단위를 구별해서 본다.

## 기각된 대안

| 대안 | 기각 이유 |
| --- | --- |
| 별도 표 `user_assisted_dan`(B안) | 삽입 함수가 둘이고 읽기에서 두 표를 합쳐야 하며, 같은 열 묶음을 두 번 정의한다. 기본 키 교체 위험이 없다는 이점뿐 |
| `label` 은 원래 글자로 두고 클라이언트가 `assisted` 를 읽어 `A-` 를 붙임 | 이미 게시된 v0.1.8 과 draft 인 v0.1.9 가 A-단위를 정식처럼 보이게 된다 |
| `ranked` 를 무시하고 어시스트 플래그만 봄 | 신뢰하지 않는 빌드와 `REQUIRE_BUILD_HASH` 의 막음까지 풀린다(앞 ADR 에서도 기각, `rankedWithoutAssist` 가 이 기각을 지키는 방법이다) |
| 한 판이 정식과 A-단위를 모두 줌 | 정식 조건을 다 갖춘 판이 A-단위를 따로 얻는 중복이고 표시 규칙이 모호해진다 |
| 정식을 얻으면 A-단위 행을 지움, 또는 선택을 자동으로 정식으로 바꿈 | 기록과 사용자의 선택을 서버가 바꾼다. 행은 독립으로 두고 표시 규칙이 가린다 |
| 되돌림 마이그레이션 | `assisted = 1` 행이 생기면 기본 키 복구가 중복으로 실패한다 |
| 클라이언트가 `ranked_without_assist` 를 보냄 | 위조 가능. 서버가 계산한다 |

## 영향과 관련 파일

- 응답 모양 변경(내 단위 `shown`, `display.assisted`, `acquired[].assisted`, 모든 공개 단위 원소의 `assisted`)은 [api-spec.md](../backend/api-spec.md) 3·4·6·12절, 데이터 모델은 [data-model.md](../backend/data-model.md), 구조는 [web/architecture.md](../web/architecture.md) §15-3 에 반영했다. `dan-recompute` 출력 설명은 [ci-release.md](../ci-release.md) 6.1·6.2절.
- 운영 순서(설계서 10절): 마이그레이션 파일만 커밋·푸시 → `web` 워크플로 통과 → `status` 로 0013 `would_run=yes` → 사용자 `!` 로 `migrate` → `status` 최신 → 서버·웹 코드 푸시 → `dan-recompute-count` 로 `assistedLegacy` 를 사용자에게 보임 → 사용자 `!` 로 `dan-recompute`. 0012 때 `schema.ts` 가 migrate 보다 먼저 배포된 사고([feedback](../feedback/2026-10-23-stage-check-before-commit.md))를 되풀이하지 않도록 마이그레이션 커밋에는 `schema.ts`·`src` 를 섞지 않았다.
- 코드: `web/src/server/db/schema.ts`, `dto/dan.ts`, `service/domain/dan/{dan-eligibility,dan.service}.ts`, `service/domain/course/course.service.ts`(`rankedWithoutAssist`, `storedRankedWithoutAssistOf`), `service/domain/table-refresh/table-refresh.service.ts`, `compose/dan.ts`, `web/scripts/dan-recompute.ts`; 화면 `web/src/features/dan/*`, `web/src/widgets/settings/{dan-system-card,dan-panel}.tsx`.
- 시험: `web/tests/service/{dan-eligibility,dan,player-dans,ranking-dans,course,table-refresh}.test.ts`, `web/tests/route/dan.test.ts`, `web/tests/scripts/dan-recompute.test.ts`, `web/tests/db/{dan-award,migration-0013}.test.ts`, `web/tests/components/{dan-panel,dan-display}.test.tsx`.
- 남은 위험·확인하지 못한 것: 운영 `REQUIRE_BUILD_HASH` 값과 `client_build` 의 `trusted = false` 행 수(위 메인 결정 1). 운영 `user_dan` 행 수(기본 키 교체의 잠금·재구성 비용, 0002 가 같은 형태로 적용됐고 표는 작다고 보지만 확인하지 못했다). 실제 R-BMS 플레이어로 어시스트 코스를 깨서 제출한 램프·플래그 조합(서술은 코드에서 읽은 것이다). DB 시험은 실제 MySQL 에서 메인이 돌린다. 외부 소비자가 `shown` 이 문자열이던 응답을 읽는지(웹 화면만 읽는 세션 전용 경로로 확인했다).
- 테스트 부채: [3.127(0013 이전 판 추정), 3.128(클라이언트 `extra.flags` 자기 신고), 3.129(플레이어 순위 배지 2개 제한)](../quality-assurance/2026-10-14-test-debt.md). 실기 점검: [QA 3.24](../quality-assurance/2026-10-11-default-skin-manual-check.md).

## 날짜와 사용자 원문

2026-10-24.

- 요청: "추가 기능으로, 코스모드에서 어시스트 클리어를하면 그것도 웹에서 볼수있게해줘 A-단위 이런식으로".
- 결정 답: "전부 추천대로 그리고 workflow도 사용"(위 결정 1~5 가 그 질문 묶음의 추천안이다).
