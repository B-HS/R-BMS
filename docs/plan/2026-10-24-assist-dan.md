# 설계: 코스 어시스트 클리어를 A-단위로 기록·표시

> 작성 2026-10-24 · 기준 커밋 `3ee88ee`(dev) · 상태: 설계(코드 변경 없음) · 작성: 설계 담당 에이전트
> 요청(2026-10-24 사용자 원문): "추가 기능으로, 코스모드에서 어시스트 클리어를하면 그것도 웹에서 볼수있게해줘 A-단위 이런식으로".
> 확정 결정(사용자 "전부 추천대로"): ① A-단위 = 정식 단위 조건에서 "어시스트 제외" 하나만 푼 판(SCRATCH AUTO, ASSIST·LIGHT ASSIST 램프(id 2·3), 어시스트 옵션 플래그. 게이지가 남은 채 끝냄, 미러·LN 제약 준수) ② 정식과 따로 기록, 같은 체계 안에서 정식이 같거나 높으면 정식만, A-단위가 더 높으면 둘 다 표시 ③ 마이페이지 단위 탭에서 A-단위도 표시 단위로 고를 수 있음 ④ 스테이지 정보가 있는 판(0012 이후)은 소급(`dan-recompute` 와 표 갱신 소급 모두).
> 관련: [단위 ADR](../acknowledge/2026-10-23-adr-dan-auto-award.md), [architecture §15-3](../web/architecture.md), [api-spec §3·§4·§6·§12](../backend/api-spec.md), [HANDOFF 5절](../HANDOFF.md).

## 0. 결론 요약

1. 데이터: `user_dan` 에 `assisted boolean NOT NULL DEFAULT false` 를 더하고 기본 키를 `(user_id, system, label, assisted)` 로 넓힌다. `user_dan_display` 에 `assisted boolean NOT NULL DEFAULT false` 를 더한다. 마이그레이션 `0013_assisted_dan`. 기존 행은 모두 정식(`false`)이 된다. 0002(`scratch_auto` 를 PK 에 넣은 선례)와 같은 모양이다.
2. 판정: `judgeDanEligibility` 가 `{ eligible: true, kind: 'formal' | 'assisted', grant }` 또는 `{ eligible: false, reason }` 를 낸다. 한 판은 둘 중 하나만 준다.
3. "순위 조건"은 "어시스트만 빼면 ranked 였을 판"으로 바꾼다. 새 판은 제출 때 그 사실(`ranked_without_assist`)을 `course_score.extra` 에 남기고, 옛 판은 플래그로 추정한다(아래 3-3).
4. 표시: 서버의 단일 함수 `shownDansOf` 가 체계마다 보일 단위(0~2개)를 정하고, 모든 공개 응답(`extra.dans`, 순위 행 `dans`, 내 단위 `shown`)이 `{ system, name, label, assisted }` 를 받는다. `label` 은 A-단위면 서버가 `"A-"` 를 붙인 글자(`A-十段`, `A-st3`)다.
5. 클라이언트(R-BMS 플레이어)는 바꾸지 않는다. 옛 클라이언트는 모르는 `assisted` 를 무시하고 `label` 의 `A-十段` 을 그대로 그린다.
6. 운영 순서: 마이그레이션 파일만 커밋·푸시 → 사용자 `!` 로 `web-db migrate` → 코드 푸시 → `dan-recompute-count` → 사용자 `!` 로 `dan-recompute`.

## 1. 현재 흐름 지도 (3ee88ee)

### 1-1. 제출 → 판정 → 저장

| 단계 | 위치 | 하는 일 |
| --- | --- | --- |
| 클라이언트 코스 램프 | `apps/rbms-player/src/app_result.rs:773-777`, `apps/rbms-player/src/lib.rs:386-394`, `crates/rbms-judge/src/gauge.rs:157-168` | 코스 전체 어시스트(`CourseAssist`)로 램프를 내린다: SCRATCH AUTO 만 → LIGHT ASSIST(id 3), 넓힌 판정·자기 TOTAL 이 있으면 → ASSIST(id 2). FAILED 는 그대로 |
| 클라이언트 본문 | `apps/rbms-player/src/course_ir.rs:98-137` | `scratch_auto`, `assist`(태그 `AUTO_SCRATCH`, `CUSTOM_JUDGE`, `crates/rbms-ir/src/mapping.rs:126,129,154-163`), `gauge`, `course_gauge`, `stages` |
| 서버 ranked | `web/src/server/service/domain/course/course.service.ts:283-285` | `flags = assistFlags(input) + buildTrustFlags`, `ranked = rankBlockingFlags(flags) 없음 && 빌드가 막지 않음` |
| 플래그 규칙 | `web/src/server/service/domain/score/ranked-policy.ts:27,31-38` | `AUTO_SCRATCH` → `SCRATCH_AUTO`(순위 중립), 다른 태그(`CUSTOM_JUDGE`) → `ASSIST`(순위 막음), `UNKNOWN_BUILD` 는 중립이지만 `isBuildBlockingRank`(같은 파일 58-59)가 막을 수 있음 |
| 행 저장 | `course.service.ts:201-245`(`buildCourseScoreRow`, 242 줄 `extra` 에 `flags` 저장) | 플래그가 있으면 `extra = { ...input.extra, flags, scratch_auto, assist }` |
| 판정 호출 | `web/src/server/route/course.ts:115-120` | `danService.awardFromRun(result.scoreRow)`, 새 단위면 `revalidate.onDanAward`, 던져도 제출 성공 |
| 판정 함수 | `web/src/server/service/domain/dan/dan-eligibility.ts:60-80` | 순서: 단위 코스(63) → 게스트(64) → `ranked`(65) → 램프 EASY 이상·게이지>0(66) → SCRATCH AUTO(67) → ASSIST 플래그(68) → 스테이지 사실(69) → 코스 게이지(70-71) → 스테이지 수(72) → 차트(73) → RANDOM(74-75) → LN(76-78) |
| 저장 | `dan.service.ts:99-106`, `compose/dan.ts:85-88` | `INSERT IGNORE` 로 (user, system, label) 마다 첫 판만 |

현재 어시스트 판이 실제로 걸리는 사유: SCRATCH AUTO 클리어는 램프가 LIGHT ASSIST(3) 라 `not_cleared`(66 줄이 67 줄보다 먼저), 넓힌 판정·TOTAL 판은 `ASSIST` 플래그 때문에 `ranked=false` 라 `unranked`(65 줄). 즉 `scratch_auto`·`assisted` 사유는 지금 클라이언트의 판에서는 거의 나오지 않는다.

### 1-2. 소급

- `dan.service.ts:115-135` `awardForCourses`: `stages` 가 있는 판을 `playedAt, id` 오름차순 500개씩(`compose/dan.ts:50-83`) 읽어 판정, `{ scanned, eligible, awarded }`.
- `web/scripts/dan-recompute.ts:22-34`: 모든 단위 코스에 대해 위를 부르고 한 줄 출력(`plan|applied danCourses N, scanned N, eligible N, awarded N`).
- `.github/workflows/web-db.yml:128-136`: `dan-recompute-count`(count), `dan-recompute`(count → apply → count).
- 표 갱신: `table-refresh.service.ts:264-274` 가 새 단위 코스(`newDanHashes`)만 `awardForCourses` 하고 `awarded` 를 보고. `read-cache.ts:239-245` 가 `awarded > 0` 이면 프로필·순위 캐시 무효화.

### 1-3. 표시

| 소비처 | 위치 | 모양 |
| --- | --- | --- |
| 단일 계산 | `dan.service.ts:69-85`(`displayOf`, `shownLabelOf`, `systemViewOf`), `147-163`(`listDisplayed`) | 체계마다 label 하나(auto = rank 최고, label = 고른 것, hidden = 없음) |
| 프로필 `extra.dans` | `compose/index.ts:146`, `player.service.ts:30-38` | `[{ system, name, label }]` |
| 차트 순위 행 `extra.dans`(IR) | `compose/index.ts:186`, `score.service.ts:513-516`, `dto/score.ts:122` | 같은 모양 |
| FE 플레이어 순위 `dans` | `compose/index.ts:404`, `fe.service.ts:80-85,110` | 같은 모양 |
| 내 단위 | `dan.service.ts:88-96`, `dto/dan.ts:21-40`, `route/dan.ts:29-50` | `systems[]: { system, name, display: { mode, label }, shown: string \| null, acquired[] }` |
| 웹 배지 | `web/src/features/dan/dan-badge.tsx:5-9`, `dan-badge-list.tsx:6-20`(key = `dan.system`) | `{name} {label}` 의 `secondary` 배지 |
| 웹 소비 | `widgets/player/player-detail-panel.tsx:54`(`readDisplayedDans`, `entities/dan.type.ts:14-17` 의 `.catch([])`), `widgets/leaderboard/player-leaderboard-panel.tsx:52`, `widgets/settings/dan-system-card.tsx`, `widgets/settings/dan-panel.tsx:14,40-44` | |
| 클라이언트 | `crates/rbms-ir/src/dto/score.rs:220-226`(`DisplayedDan { system, name, label }`, `deny_unknown_fields` 없음), `241-249`(항목별 `deserialize().ok()`), `apps/rbms-player/src/ir_ranking_view.rs:83`(`MAX_DAN_BADGES = 2`), `183`(`label` 만 씀), `409-428`(배지 그리기) | |

## 2. 데이터 모델

### 2-1. 두 안 비교

| 기준 | A안(추천): `user_dan.assisted` + PK 확장 | B안: 별도 표 `user_assisted_dan` |
| --- | --- | --- |
| PK·유일 | `(user_id, system, label, assisted)`: 정식 十段 과 A-十段 이 한 사용자에 따로 한 줄씩 | 표마다 `(user_id, system, label)` |
| "처음 얻은 판만 남김" | `INSERT IGNORE` 그대로. 종류별로 첫 판이 남는다 | 표마다 같은 규칙, 삽입 함수가 둘 |
| 읽기 | `listUserDans` 한 번에 두 종류(지금 쿼리에 열 하나) | 두 표를 읽어 합치기 |
| 기존 코드와 공존(마이그레이션 먼저 적용 기간) | 옛 코드의 `INSERT` 는 `assisted` 를 안 보내 기본값 `false` → 정식 행으로 정상 동작. `hasUserDan` 은 정식 행만 있으므로 결과 동일 | 영향 없음 |
| 마이그레이션 위험 | PK 교체: `DROP PRIMARY KEY, ADD PRIMARY KEY(...)` 를 **한 문장**으로 써야 한다(FK `user_dan_user_id_user_id_fk` 가 PK 의 첫 열 `user_id` 색인을 쓰므로 따로 떨어뜨리면 1553 오류가 날 수 있고, `sql_require_primary_key` 환경에서도 한 문장이면 안전). 0002 가 같은 형태로 운영 적용됨(`web/drizzle/0002_scratch_auto_bests.sql:4-5`) | 새 표 만들기만 |
| 코드 패턴 | `chart_best`·`course_best` 의 `scratch_auto` PK(`schema.ts:160`)와 같다 | 같은 열 묶음을 두 번 정의 |

추천: **A안**. 표시·선택·정렬이 두 종류를 함께 다루고(결정 2·3), 기존 `scratch_auto` 패턴과 같으며, 공존 기간에도 옛 코드가 깨지지 않는다.

### 2-2. 표시 선택 `user_dan_display`

- PK 는 `(user_id, system)` 그대로(체계마다 선택 하나). `assisted boolean NOT NULL DEFAULT false` 를 더한다.
- 의미: 행 없음 = 자동, `label` NULL = 숨김(`assisted` 무시, `false` 로 씀), `label` 값 + `assisted` = 그 단위.

### 2-3. 마이그레이션 `0013_assisted_dan` (초안)

`web/drizzle/0013_assisted_dan.sql`:

```sql
ALTER TABLE `user_dan` ADD `assisted` boolean DEFAULT false NOT NULL;--> statement-breakpoint
ALTER TABLE `user_dan` DROP PRIMARY KEY, ADD PRIMARY KEY(`user_id`,`system`,`label`,`assisted`);--> statement-breakpoint
ALTER TABLE `user_dan_display` ADD `assisted` boolean DEFAULT false NOT NULL;
```

- 만드는 법: `web/src/server/db/schema.ts:377-410` 를 아래처럼 고친 뒤 `cd web && bun run db:generate` → 나온 파일 이름과 `meta/_journal.json` 의 `tag` 를 `0013_assisted_dan` 으로 맞추고(0012 와 같은 방식), drizzle-kit 이 PK 교체를 두 문장으로 내면 위처럼 한 문장으로 합친다. `meta/0013_snapshot.json` 은 생성된 그대로 둔다.
- `schema.ts` 변경: `userDan` 에 `assisted: boolean('assisted').default(false).notNull()`, PK `primaryKey({ columns: [table.userId, table.system, table.label, table.assisted] })`. `userDanDisplay` 에 `assisted: boolean('assisted').default(false).notNull()`.
- 기존 행: 모두 `assisted = 0`(정식). 데이터 변경 문장 없음. 운영 `user_dan` 은 `dan-recompute` 를 돌린 적이 없어(ADR "확인하지 못한 사실") 행이 적거나 없을 것으로 보이며 표 재구성 비용은 작다(확인 못 함).
- 되돌리기: `assisted = 1` 행이 생긴 뒤에는 PK 를 되돌릴 수 없으므로(중복) 되돌림 마이그레이션은 만들지 않는다.

## 3. 판정 함수 변경 (`dan-eligibility.ts`)

### 3-1. 결과 모양

```ts
export const DAN_AWARD_KIND = { FORMAL: 'formal', ASSISTED: 'assisted' } as const
export type DanAwardKind = (typeof DAN_AWARD_KIND)[keyof typeof DAN_AWARD_KIND]

export const DAN_ASSIST_SOURCE = { SCRATCH_AUTO: 'scratch_auto', ASSIST_LAMP: 'assist_lamp', ASSIST_FLAG: 'assist_flag' } as const

// 성공: { eligible: true, kind, assistedBy: DanAssistSource[], grant: { userId, dan, courseGauge, gauge } }
// 실패: { eligible: false, reason }
```

- `DAN_INELIGIBLE_REASON` 에서 `SCRATCH_AUTO`·`ASSISTED` 를 뺀다(더 이상 거절 사유가 아니라 `kind`·`assistedBy` 로 나온다). 나머지 열 개는 그대로.
- `DanRunFacts`(= `SubmittedCourseRun` 의 일부)에 `rankedWithoutAssist: boolean | null` 을 더한다(3-3).

### 3-2. 순서와 조건 (위에서 처음 걸리는 것)

1. `not_a_dan_course`: `dan === null`.
2. `guest`: `userId === null`.
3. `unranked`: `!(run.ranked || (hasAssistFlag && rankedWithoutAssistOf(run)))`. `hasAssistFlag = flags.includes('ASSIST')`. 어시스트 플래그가 없는데 `ranked=false` 면(신뢰하지 않는 빌드 등) 그대로 거절.
4. `not_cleared`: `run.clear < clearLampToId('AssistEasy')`(NO PLAY·FAILED) 또는 `gaugeValue <= 0`. ASSIST(2)·LIGHT ASSIST(3) 램프는 여기서 통과한다.
5. `no_stage_facts` → `not_a_course_gauge` → `stage_count` → `chart_mismatch` → `random_not_allowed` → `ln_mode_mismatch`: 지금 69-78 줄 그대로(정식·A-단위 공통. 결정 1 의 "미러·LN 제약은 지킨 판").
6. 종류 결정: `assistedBy` = [`scratch_auto`: `run.scratchAuto || flags.includes('SCRATCH_AUTO')`, `assist_lamp`: `run.clear < clearLampToId('Easy')`(= 2·3), `assist_flag`: `hasAssistFlag`] 중 참인 것. 비었으면 `kind: 'formal'`, 아니면 `kind: 'assisted'`.

- 같은 판은 한 종류만 준다(6 단계가 배타적). 정식 조건을 다 갖춘 판은 A-단위를 주지 않는다.
- 정식을 나중에 얻어도 A-단위 행은 지우지 않는다(행은 독립, 보이는 것은 4절 규칙). 반대로 A-단위를 가진 뒤 같은 단위의 정식을 얻으면 정식 행이 새로 생긴다(`awarded` 1).
- 게이지 모드(LR2·BEATORAJA·IIDX)는 지금처럼 묻지 않고 `course_gauge` 에 기록.
- 주의(결정 1 의 결과): 코스 본문에는 판정 폭·TOTAL 이 따로 없고 클라이언트가 넓힌 판정과 자기 TOTAL 을 `CUSTOM_JUDGE` 태그 → 서버 `ASSIST` 플래그 → ASSIST 램프로 보낸다(1-1 표). 그래서 **넓힌 판정·자기 TOTAL 로 깬 코스도 A-단위가 된다**. 결정 1("ASSIST 램프, 어시스트 옵션 플래그")과 일치하지만 ADR 의 "`unranked` = 넓힌 판정·TOTAL 덮어쓰기" 서술은 이 판들에 대해 바뀐다(문서 단위가 고친다).

### 3-3. "어시스트만 빼면 ranked" 사실 (`rankedWithoutAssist`)

- 문제: 어시스트 판은 `ranked=false` 로 저장되어, 빌드가 막았는지(신뢰하지 않는 빌드, `REQUIRE_BUILD_HASH=true` 의 미등록 빌드) 저장된 행만으로 알 수 없다. 둘 다 `UNKNOWN_BUILD` 플래그만 남기고(`ranked-policy.ts:56`), 릴리스 빌드는 허용 목록에 자동 등록되지 않아(`.github/workflows/release.yml:259` 주석) 대부분의 판이 `UNKNOWN_BUILD` 를 갖는다. ADR 은 "`ranked` 를 무시하고 어시스트 플래그만 봄" 을 기각했다.
- 새 판(제출 시): `course.service.ts` `submit` 이 `rankedWithoutAssist = rankBlockingFlags(flags.filter((f) => f !== 'ASSIST')).length === 0 && !isBuildBlockingRank(buildTrust, requireTrustedBuild)` 를 계산해 `SubmittedCourseRun` 에 싣고, `flags` 에 `ASSIST` 가 있으면 `buildCourseScoreRow` 가 `extra.ranked_without_assist` 를 **항상 서버 값으로 덮어** 쓴다(클라이언트 `extra` 의 같은 키는 덮인다). `ASSIST` 가 없으면 이 사실은 판정에 쓰이지 않는다.
- 저장된 판(중복 제출 경로 `course.service.ts:326-339`, 소급 `compose/dan.ts:82`): 새 함수 `storedRankedWithoutAssistOf(extra): boolean | null`(`storedFlagsOf` 옆, 171-172 줄)로 읽는다. 키가 없으면 `null`.
- `null`(0013 배포 전에 저장된 0012 이후 판)의 추정: `rankBlockingFlags(flags.filter((f) => f !== 'ASSIST')).length === 0` 이면 참. 코스 플래그는 `ASSIST`·`SCRATCH_AUTO`·`UNKNOWN_BUILD` 뿐이라 사실상 "빌드가 막지 않았다고 본다" 이다. 기본 배포(`REQUIRE_BUILD_HASH=false`)에서 `trusted=false` 로 등록된 빌드가 없으면 정확하다. 이 경로로 A-단위를 받은 판 수를 `assistedLegacy` 로 센다(6절). 운영 `REQUIRE_BUILD_HASH` 값은 확인하지 못했다(11절).

### 3-4. 서비스·compose 변경

| 위치 | 변경 |
| --- | --- |
| `dan.service.ts:20` `UserDanRow` | `$inferSelect` 에서 유도되므로 `assisted` 가 자동으로 들어온다 |
| `dan.service.ts:45-58` | `DanAwardResult = { awarded: CatalogDan \| null; kind: DanAwardKind \| null; reason }`, `toUserDanRow` 에 `assisted: verdict.kind === 'assisted'` |
| `dan.service.ts:99-106` `awardFromRun` | 종류와 함께 삽입, `awarded` 는 새 행이면 단위(종류 무관) |
| `dan.service.ts:115-135` `awardForCourses` | 합계 `{ scanned, eligible, awarded, eligibleAssisted, awardedAssisted, assistedLegacy }`(`eligible`·`awarded` 는 정식만) |
| `dan.service.ts:36` `hasUserDan` | 인자에 `assisted` |
| `compose/dan.ts:50-83` | 행에서 `rankedWithoutAssist: storedRankedWithoutAssistOf(extra)` 를 더 만든다 |
| `compose/dan.ts:90-97` | `eq(userDan.assisted, assisted)` 조건 추가 |
| `compose/dan.ts:99-119` | `assisted` 열 선택, 정렬 `rank asc, assisted asc` |
| `compose/dan.ts:121-135` | `listDisplays` 에 `assisted`, `upsertDisplay` 의 `set` 에 `assisted` |
| `course.service.ts:92-95` | `SubmittedCourseRun` 에 `rankedWithoutAssist: boolean \| null` |
| `course.service.ts:201-245,283-285,326-339,377-390` | 3-3 의 계산·저장·읽기 |
| `table-refresh.service.ts:65-67,270-273` | `TableDanAward` 결과에 `awardedAssisted`, 보고의 `awarded = awarded + awardedAssisted`(캐시 무효화 기준 `read-cache.ts:244` 를 그대로 쓰기 위함) |
| `route/course.ts:116-117` | 변경 없음(`awarded` 가 종류 무관이므로 그대로 무효화) |

## 4. 표시 규칙 (서버 단일 함수)

### 4-1. 함수

`dan.service.ts` 의 `shownLabelOf`(75-79)를 바꾼다.

```ts
type HeldDan = Pick<AcquiredDanRow, 'label' | 'rank' | 'assisted'>

const highestOf = (held: readonly HeldDan[], assisted: boolean) =>
    held.filter((row) => row.assisted === assisted).toSorted((left, right) => right.rank - left.rank)[0]

/** The dans of one system others see: none when hidden, the chosen one, or in auto the highest regular dan and also the highest A-dan when it ranks above it. */
const shownDansOf = (held: readonly HeldDan[], display: DanDisplayOutput) => {
    if (display.mode === 'hidden') return []
    if (display.mode === 'label') return held.filter((row) => row.label === display.label && row.assisted === display.assisted).slice(0, 1)
    const regular = highestOf(held, false)
    const assisted = highestOf(held, true)
    if (!assisted || (regular && regular.rank >= assisted.rank)) return regular ? [regular] : []
    return regular ? [regular, assisted] : [assisted]
}
```

- `auto`: 정식 최고 rank 가 A-단위 최고 rank 이상이면 정식만, A-단위가 더 높으면 정식(있으면) + A-단위 순서. A-단위만 있으면 A-단위 하나. 비교는 같은 체계 안에서만(체계마다 이 함수를 따로 부른다).
- `label`: 고른 단위 하나만(결정 3). 고른 행이 없으면 빈 목록(지금도 `null`).
- `hidden`: 빈 목록.
- `systemViewOf`(81-85)는 `shown: shownDansOf(...)` 를 돌려주고, `listMine`·`listDisplayed` 둘 다 이것만 쓴다. 출력 순서는 `DAN_SYSTEMS` 순, 체계 안에서 정식 → A-단위.

### 4-2. 표시 이름

- `dto/dan.ts` 에 `export const ASSISTED_DAN_LABEL_PREFIX = 'A-'` 와 `export const danDisplayLabel = (dan: { label: string; assisted: boolean }) => (dan.assisted ? \`${ASSISTED_DAN_LABEL_PREFIX}${dan.label}\` : dan.label)`. 서버(공개 응답)와 웹(선택 목록·획득 표)이 같은 함수를 쓴다.
- 세 언어 모두 같은 꼴(`A-十段`, `A-発狂初段`, `A-st3`, `A-sl0`). label 자체가 언어와 무관한 글자(일본어 단위 이름, `st<n>`)라 접두도 번역하지 않는다. 메시지는 설명 문구에만 쓴다(7-2).
- 용어: 한국어 "A-단위", 영어 "A-Dan", 일본어 "A-段位"(용어집 `docs/reference/2026-10-24-i18n-glossary.md:64` 의 DAN 줄 옆에 추가).

### 4-3. 응답 모양 변경

| 응답 | 지금 | 바뀐 뒤 |
| --- | --- | --- |
| `displayedDanSchema`(`dto/dan.ts:19`) | `{ system, name, label }` | `{ system, name, label, assisted: z.boolean().default(false) }`. `label` 은 `danDisplayLabel` 결과(A-단위면 `A-` 포함). `.default(false)` 는 캐시에 남은 옛 모양을 웹이 읽을 때(`entities/dan.type.ts:14-17` 의 `.catch([])` 가 전부 버리지 않게) 필요 |
| 프로필 `extra.dans`, 차트 순위 행 `extra.dans`, FE 순위 `dans` | 체계마다 최대 1개 | 체계마다 0~2개, 같은 원소 모양 |
| `danDisplayOutputSchema`(21) | `{ mode, label }` | `{ mode, label, assisted: boolean }`(auto·hidden 은 `false`) |
| `acquiredDanOutputSchema`(23-30) | `{ label, rank, course_hash, course_name, course_gauge, acquired_at }` | `assisted: boolean` 추가(`label` 은 원래 label, 표시는 웹이 `danDisplayLabel`) |
| `danSystemOutputSchema.shown`(36) | `string \| null` | `z.array(displayedDanSchema)`(웹만 쓰는 세션 API 라 같은 배포에서 바꾼다) |

- 옛 클라이언트 호환: `DisplayedDan`(`crates/rbms-ir/src/dto/score.rs:220-226`)은 `deny_unknown_fields` 가 없어 `assisted` 를 무시하고, 목록은 항목마다 `deserialize().ok()`(241-249)라 한 항목이 틀려도 나머지를 읽는다. `label` 에 `A-` 가 들어 있으므로 플레이어 순위 오버레이에 `A-十段` 이 그대로 보인다.

## 5. 표시 선택 API

- `danDisplayInputSchema`(`dto/dan.ts:12-16`)의 `label` 가지에 `assisted: z.boolean().default(false)` 를 더한다. 옛 요청(`assisted` 없음)은 정식 선택으로 해석된다. 본문 상한 1 KiB(`DAN_DISPLAY_BODY_MAX_BYTES`)는 그대로.
- `setDisplay`(`dan.service.ts:139-145`): `label` 모드면 `hasUserDan({ userId, system, label, assisted })` 가 거짓일 때 `VALIDATION_ERROR { field: 'label' }`. `upsertDisplay({ userId, system, label, assisted })`, `hidden` 은 `label: null, assisted: false`.
- 예: `{ "system": "genoside2018_sp", "mode": "label", "label": "十段", "assisted": true }` → A-十段 을 보인다.
- 무효화는 지금처럼 `revalidateOnDanAward`(`route/dan.ts:47`).

## 6. 소급

- `awardForCourses` 가 3절 판정을 그대로 쓰므로 `dan-recompute` 와 표 갱신 소급이 A-단위도 준다. 같은 판이 두 종류를 주지 않고, `INSERT IGNORE` 라 두 번 돌려도 두 번째는 0.
- 출력(`dan-recompute.ts:30-32`): `plan danCourses N, scanned N, eligible N, awarded N, eligibleAssisted N, awardedAssisted N, assistedLegacy N`(`eligible`·`awarded` = 정식, `*Assisted` = A-단위, `assistedLegacy` = 3-3 의 추정으로 A-단위가 된 판). 숫자만 찍는 원칙(공개 로그) 유지.
- `web-db.yml` 은 바꾸지 않는다(같은 스크립트·같은 action). 워크플로 설명 문구만 "dans (A-dans included)" 정도로 고칠지는 선택.
- 운영에서 `dan-recompute` 를 아직 돌리지 않았다면(ADR 기록) 이번 실행이 정식 단위의 첫 소급도 겸한다. 숫자를 볼 때 이 점을 같이 본다.
- 소급 스크립트는 Vercel 밖에서 돌아 캐시를 무효화하지 않는다(지금과 같음). 프로필·순위의 단위는 캐시 수명(`player-profiles`·`leaderboard-players`·`chart-ranking`) 뒤에 보인다.

## 7. 웹 화면

### 7-1. 변경 목록

| 파일 | 변경 |
| --- | --- |
| `web/src/features/dan/dan-badge.tsx` | `dan.assisted` 면 `variant='outline'` + `border-dashed`(테마 토큰 `border-border`·`text-foreground` 라 다크·라이트 모두 대비 유지), `title={t('assisted')}`(`pages.dan`). 정식은 지금 `secondary` 그대로. 글자는 `{dan.name} {dan.label}`(label 에 이미 `A-`) |
| `web/src/features/dan/dan-badge-list.tsx:14` | `key` 를 `` `${dan.system}:${dan.assisted}` `` 로(한 체계 두 배지) |
| `web/src/widgets/settings/dan-system-card.tsx` | 선택 값 인코딩 `label:<label>` / `assisted:<label>`(`selectValueOf`·`toDisplayRequest`, 16-28), 선택 목록에 정식·A-단위 모두(`danDisplayLabel`), `key`·`rowKey` 를 `` `${assisted}:${label}` ``, `shownToOthers` 줄은 `system.shown` 배열을 `DanBadgeList` 로(지금 80-84 의 단일 배지 대신), 획득 표의 단위 열은 `danDisplayLabel(row)` |
| `web/src/widgets/settings/dan-panel.tsx:14` | `ELIGIBILITY_RULES` 에 `'assisted'` 추가 |
| `web/src/entities/dan.type.ts` | 타입은 DTO 유도라 자동. `readDisplayedDans` 그대로(스키마 `.default(false)` 덕에 옛 캐시도 읽힘) |
| `player-detail-panel.tsx`, `player-leaderboard-panel.tsx` | 변경 없음(`DanBadgeList` 가 처리) |
| 코스 페이지(`/courses`, `/courses/[hash]`) | 변경 없음(코스가 주는 단위 표시는 사용자 단위와 무관) |

### 7-2. 메시지 (세 언어 같은 키)

| 키 | ko | en | ja |
| --- | --- | --- | --- |
| `pages.dan.assisted`(새) | 어시스트 판으로 얻은 단위(A-단위) | Dan earned with assist (A-Dan) | アシストありで獲得した段位（A-段位） |
| `settings.danSystemCard.displayAuto`(고침) | 자동 (가장 높은 단위, A-단위가 더 높으면 함께) | Automatic (the highest Dan, plus the A-Dan when it is higher) | 自動（最も高い段位。A-段位の方が高ければ併記） |
| `settings.danPanel.rules.cleared`(고침) | 게이지가 남은 채 코스를 끝낸 판입니다(FAILED 는 제외. ASSIST·LIGHT ASSIST 램프는 A-단위가 됩니다). | Plays that finished the course with gauge remaining (FAILED is excluded; ASSIST and LIGHT ASSIST lamps give an A-Dan). | ゲージを残してコースを終えたプレーです（FAILED は除外。ASSIST・LIGHT ASSIST ランプは A-段位になります）。 |
| `settings.danPanel.rules.exclusions`(고침) | 신뢰하지 않는 빌드처럼 어시스트가 아닌 이유로 순위에 오르지 않는 판은 제외합니다. | Plays that do not enter the ranking for a reason other than assist, such as an untrusted build, are excluded. | 信頼されていないビルドなど、アシスト以外の理由でランキングに載らないプレーは除外します。 |
| `settings.danPanel.rules.assisted`(새) | SCRATCH AUTO·ASSIST·LIGHT ASSIST 나 넓힌 판정으로 깬 판은 A-단위(예: A-十段)로 따로 기록합니다. 자동 표시에서는 정식 단위가 같거나 높으면 정식 단위만, A-단위가 더 높으면 둘 다 보입니다. | Plays cleared with SCRATCH AUTO, ASSIST, LIGHT ASSIST or a widened judge are recorded separately as an A-Dan (for example A-十段). In automatic display only the regular Dan is shown when it is the same or higher, and both are shown when the A-Dan is higher. | SCRATCH AUTO・ASSIST・LIGHT ASSIST や判定幅の拡大でクリアしたプレーは A-段位（例: A-十段）として別に記録します。自動表示では通常の段位が同じか高ければ通常の段位だけ、A-段位の方が高ければ両方を表示します。 |

- 파일: `web/messages/{ko,en,ja}/{pages,settings}.json`. 키 짝 맞춤은 `tests/lib/i18n-messages.test.ts` 가 본다. 일본어는 원어민 감수 전(용어집 상태와 같음).

## 8. 클라이언트(R-BMS 플레이어)

- **변경 불필요(추천)**. 서버가 `label` 에 `A-` 를 넣어 주고 `assisted` 는 추가 필드라 옛·새 클라이언트 모두 `A-十段` 을 그린다(4-3). 와이어 버전·`API_VERSION` 변경 없음.
- 기각한 안: `label` 은 원래 글자로 두고 클라이언트가 `assisted` 를 읽어 `A-` 를 붙임 → 이미 배포된 v0.1.8·v0.1.9 가 A-단위를 정식처럼 보이게 된다.
- 남는 차이: 오버레이는 최대 2개(`ir_ranking_view.rs:83`)만 그리므로, 한 체계에서 정식+A-단위 둘을 보이는 플레이어가 다른 체계의 단위도 보이면 세 번째부터 잘린다(서버 순서: 체계 순, 체계 안에서 정식 → A-단위). 배지 색 구분도 없다. 원하면 후속으로 `DisplayedDan` 에 `assisted` 를 읽어 배지 색을 바꾸는 클라이언트 단위를 둘 수 있다(이번 범위 밖).

## 9. 구현 단위 (worktree)

공통: 컨벤션(`web/AGENTS.md`, `~/.claude/convention/*`), 주석 금지(JSDoc 영어만), 매직넘버 금지, 하위 에이전트 커밋·푸시 금지. 웹 게이트 = `cd web && bun run verify && bun run format:check && bun run build`. DB 시험은 메인이 Docker MySQL 8.4 로 `bun run db:migrate` 뒤 `RBMS_DB_E2E=1 bun test tests/db`.

| 단위 | 소유 파일 | 순서 | 검증 | 엣지 케이스 |
| --- | --- | --- | --- | --- |
| **M0 마이그레이션(파일만)** | `web/drizzle/0013_assisted_dan.sql`, `web/drizzle/meta/0013_snapshot.json`, `web/drizzle/meta/_journal.json` (생성을 위해 `schema.ts` 를 2-3 대로 고치되 넘기지 않음. `schema.ts` 는 S1 소유) | 첫 웨이브, S1 과 병렬 | SQL 이 2-3 초안과 같은지(PK 교체 한 문장), 메인이 Docker 로 0000~0013 적용 → `bun run db:status` 최신. 통합 뒤 메인이 `bun run db:generate` 로 "변경 없음" 확인(M0 스냅샷 = S1 의 `schema.ts`) | 커밋에 `schema.ts`·코드가 섞이지 않을 것(Vercel Production 이 `dev` 라 마이그레이션 전 배포되면 새 열을 쓰는 코드가 깨진다) |
| **S1 서버** | `web/src/server/db/schema.ts`, `dto/dan.ts`, `service/domain/dan/{dan-eligibility,dan.service}.ts`, `compose/dan.ts`, `service/domain/course/course.service.ts`, `service/domain/table-refresh/table-refresh.service.ts`, `web/scripts/dan-recompute.ts`; 시험 `tests/service/{dan-eligibility,dan,player-dans,ranking-dans,course,table-refresh}.test.ts`, `tests/route/dan.test.ts`, `tests/scripts/dan-recompute.test.ts`, `tests/db/dan-award.test.ts`, 새 `tests/db/migration-0013.test.ts` | 첫 웨이브 | `bun run verify && bun run format:check`(빌드는 W1 뒤 메인 통합 게이트), 좁게 `bun test tests/service/dan-eligibility.test.ts tests/service/dan.test.ts tests/scripts/dan-recompute.test.ts`(현재 50 통과가 기준선). DB 시험은 메인 | SCRATCH AUTO 만(램프 3, ranked) → A; CUSTOM_JUDGE(램프 2, ASSIST, ranked=false, `ranked_without_assist` true/false/없음); 램프 EASY 이상 + `scratchAuto` → A; FAILED·게이지 0 → `not_cleared`; 빌드가 막은 비어시스트 판 → `unranked`; 클라이언트 `extra.ranked_without_assist` 위조가 덮이는지; 같은 사용자 정식·A 같은 label 공존; 두 번째 `apply` 0; `auto`/`label`/`hidden` × (정식만, A만, A>정식, A=정식, A<정식); 옛 요청 본문(`assisted` 없음) |
| **W1 웹 화면** | `web/src/features/dan/{dan-badge,dan-badge-list}.tsx`, `web/src/widgets/settings/{dan-system-card,dan-panel}.tsx`, `web/messages/{ko,en,ja}/{pages,settings}.json`; 시험 `tests/components/{dan-panel,dan-display}.test.tsx`, `tests/components/course-dan-entities.test.ts`(필요 시) | 둘째 웨이브(S1 의 DTO 병합 뒤) | `bun run verify && bun run format:check && bun run build` | 한 체계 두 배지 key 중복 없음; 옛 캐시 원소(`assisted` 없음); A-단위만 가진 체계의 선택 값 왕복; 다크·라이트 대비(점선 테두리) |
| **D1 문서** | 새 `docs/acknowledge/2026-10-24-adr-assisted-dan.md`, `docs/acknowledge/2026-10-23-adr-dan-auto-award.md`(관계·"넓힌 판정" 서술), `docs/web/architecture.md` §15-3·마이그레이션 목록(284-288 근처), `docs/backend/api-spec.md` §3(140)·§4(183)·§6(321)·§12(493-498), `docs/reference/2026-10-24-i18n-glossary.md`, `docs/backend/data-model.md`(단위 표 절이 있으면) | 둘째 웨이브(S1 결과 기준) | 메인 diff 검토(문서 검사 없음) | 코드 줄 번호는 병합 뒤 커밋 기준 |
| (선택) C1 클라이언트 | `crates/rbms-ir/src/dto/score.rs`, `apps/rbms-player/src/ir_ranking_view.rs` | 이번 범위 밖 | — | — |

커밋(메인, 선별 스테이징): ① `feat(web-db): 어시스트 단위 열 마이그레이션 0013 추가`(M0 세 파일만) → 운영 적용 뒤 ② S1 ③ W1 ④ D1.

## 10. 운영 반영 순서

1. 메인: M0 세 파일만 커밋·푸시 → `gh run list --branch dev --limit 5` 로 `web` 워크플로(DB 잡이 0000~0013 적용 + `tests/db`) 통과 확인. 이 커밋은 `web/src` 를 바꾸지 않아 Production 배포가 동작을 바꾸지 않는다.
2. 메인(읽기): `gh workflow run web-db.yml --ref dev -f action=status` → 0013 `would_run=yes`.
3. 사용자 실행(자동 모드가 거부, 운영 DB 쓰기): `! gh workflow run web-db.yml --ref dev -f action=migrate`
4. 메인(읽기): `gh workflow run web-db.yml --ref dev -f action=status` → 최신. 결과는 `gh run list --workflow web-db.yml --limit 3` 와 `gh run view <id> --log`.
5. 메인: S1·W1·D1 커밋 푸시 → `gh run list` 로 `web`(및 문서만이면 해당) 통과 → Vercel 배포. 배포 확인(`/api/version`)은 운영 GET 이라 사용자에게 요청.
6. 메인(읽기): `gh workflow run web-db.yml --ref dev -f action=dan-recompute-count` → 로그의 `plan ... eligible, awarded, eligibleAssisted, awardedAssisted, assistedLegacy` 를 사용자에게 보인다.
7. 사용자 실행: `! gh workflow run web-db.yml --ref dev -f action=dan-recompute` → 로그에서 `applied ...` 와 마지막 `plan ... awarded 0, awardedAssisted 0` 확인.

## 11. 위험과 확인 못 한 것

1. 운영 `REQUIRE_BUILD_HASH` 값과 `client_build` 의 `trusted=false` 행 수는 확인하지 못했다(`docs/web/2026-10-14-audit.md:294`). 둘 중 하나라도 있으면 0013 배포 전 저장된 어시스트 판의 추정(3-3)이 빌드가 막은 판에 A-단위를 줄 수 있다. 영향 판 수는 `assistedLegacy` 로 보인다. 사용자에게 Vercel 의 값 확인을 한 줄로 묻는 것을 추천.
2. 0013 배포 전에 저장된 판은 클라이언트가 `extra.ranked_without_assist` 를 넣어 보냈다면 그 값이 남아 있을 수 있다(옛 코드는 클라이언트 `extra` 를 펼쳐 저장). 추정과 같은 방향(참)이라 실제 차이는 1번과 같은 경우뿐이다.
3. 넓힌 판정·자기 TOTAL 코스 판도 A-단위가 된다(3-2 주의). 결정 1 의 문구와 일치하지만 사용자가 "어시스트" 를 SCRATCH AUTO 와 램프만으로 생각했을 가능성은 확인하지 못했다.
4. PK 교체 문장이 운영 MySQL 에서 잠금·재구성을 한다. 0002 가 같은 형태로 적용됐고 `user_dan` 은 작다고 보지만 운영 행 수는 확인하지 못했다.
5. 플레이어 오버레이는 배지 2개까지라 일부 단위가 잘릴 수 있고 A-단위의 색 구분이 없다(8절).
6. `label` 모드로 A-단위를 고른 뒤 같은 단위의 정식을 얻어도 선택은 A-단위로 남는다(사용자가 고른 것을 서버가 바꾸지 않음). 자동으로 바꾸고 싶다면 별도 결정이 필요하다.
7. `/api/fe/me/dans` 의 `shown` 이 문자열에서 배열로 바뀐다. 이 경로는 세션 전용이고 웹 화면만 쓰는 것으로 확인했다(`grep` 결과 `entities/dan.query.ts` 뿐). 외부 소비자는 확인하지 못했다.
8. 소급은 캐시를 무효화하지 않아 프로필·순위의 A-단위는 캐시 수명 뒤에 보인다(지금 정식 소급과 같음).
9. 실기(R-BMS 플레이어로 실제 어시스트 코스를 깨서 제출)에서 램프·플래그 조합을 확인하지 못했다. 서술은 코드(1-1 표)에서 읽은 것이다.
