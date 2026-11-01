> 최종 갱신 2026-11-02 · 대응 커밋 `28085a7`·`8af1a39`·`f737c4d`(조사 시점 `837d652`, 구현은 이 작업) · 조사 wf_db20f1ba-f79


# 홈 "최근 활동"에 어시스트 활동 넣기 조사 보고서

기준은 작업 트리 파일이다. 줄 번호는 현재 파일 기준이고, 경로는 `web/` 아래다(`docs/`, `crates/`, `apps/` 제외). 확인한 사실에는 파일:줄을 붙였고, 추측은 "추측"으로 표시했다.

## 구현 결과 (2026-11-02, 커밋 `f737c4d`)

5절의 **안 A** 로 구현됐다(사용자 선택: "SCRATCH AUTO 판 포함 (추천)"). 결정 기록: [홈 최근 활동 ADR](../../acknowledge/2026-11-02-adr-home-activity-scratch-auto.md).

- `web/src/server/compose/fe.ts` 의 `listRecentActivity` 가 `where(eq(score.ranked, true))` 만 둔다(SCRATCH AUTO 판 포함, `ranked=false` 판은 계속 숨김). `ActivityRow` 모양·캐시 태그·`QUERY_KEY` 는 불변.
- `web/src/widgets/home/recent-activity-panel.tsx` 가 `ScoreRow` 에 `isScratchAuto={row.scratch_auto}` 를 넘겨 A-SCR 표지를 붙인다. `browse.home.recent.note` 를 세 언어 모두 "순위에 반영되는 제출만 표시됩니다. A-SCR 은 SCRATCH AUTO 로 친 기록입니다." 취지로 바꿨다.
- 시험: `tests/db/scratch-auto-ranking.test.ts` 의 기대값 갱신(홈 목록에 SCRATCH AUTO 판 포함), 새 `tests/components/home-recent-activity-panel.test.tsx`. DB 시험은 임시 MySQL 이 없어 **실행하지 못했다**(테스트 부채 3.141).
- 문서: `docs/backend/api-spec.md` §12 와 `docs/web/architecture.md` §1-3 의 활동 피드 서술을 고쳤다. 4-1절이 지적한 `docs/web/architecture.md` 406행("코스 제출이 `activity-recent` 를 무효화")은 **코드(`read-cache.ts` 의 `revalidateOnCourseSubmit`: `course-ranking`·`course-best`·`player-stats` 만)대로 고쳤다**. 코드는 바꾸지 않았다. `docs/acknowledge/2026-10-19-adr-web-clear-and-play-history.md` 2절의 "홈은 SCRATCH AUTO 를 뺀다" 에 대체됨 표시를 더했다.
- 7절 결정: 1 안 A, 2 코스·A-단위는 제외, 3 문구는 "A-SCR 표지와 함께 표시", 4 플레이어 기록의 `ranked=false` 표지는 범위 밖. 4-2절이 말한 홈 패널 렌더 시험은 없었으나 이번에 더했다.

## 0. 요약

1. 홈 최근 활동은 `score.ranked = true AND score.scratchAuto = false` 인 판만 고른다(`src/server/compose/fe.ts:148`). 그래서 SCRATCH AUTO 판은 늘 빠진다.
2. `ASSIST`(id 2)나 `LIGHT ASSIST`(id 3) 램프라는 사실만으로 빠지지는 않는다. 어시스트 게이지 클리어(램프 id 3, `assistGrade` 0)처럼 `ranked=true` 이고 `scratchAuto=false` 인 판은 지금도 나온다.
3. `ranked=false` 판(넓힌 판정, 자기 TOTAL, 어시스트 옵션, 오토플레이, 게스트, 신뢰 안 하는 빌드)은 빠진다. 플레이어 기록 `listPlayerRecent`(`fe.ts:154-167`)는 필터가 없어 전부 나온다.
4. 활동 응답과 `ScoreRow` 에는 이미 `scratch_auto`, `assist_grade` 가 있다. 홈 패널이 `isScratchAuto` 를 안 넘기는 것만 다르다(`src/widgets/home/recent-activity-panel.tsx:37-48`). `ranked`/`flags` 필드는 활동 응답에 없다.
5. 공식 클라이언트는 오토플레이 판을 서버로 보내지 않는다(`apps/rbms-player/src/lib.rs:344-357`). 다만 서버는 API 토큰으로 직접 온 오토플레이 판도 거절하지 않고 `ranked=false` 로 저장한다.
6. 가장 작은 안은 `scratchAuto=false` 조건을 빼고 홈 패널에 A-SCR 표지를 붙이는 것이다. 안전은 하지만 "ranked=false 어시스트 판"은 계속 빠진다.

## 1. 어시스트 관련 활동별 현재 노출 여부

| 항목 | 홈 최근 활동 | 플레이어 기록(listPlayerRecent) | 근거 |
| --- | --- | --- | --- |
| (a) SCRATCH AUTO 판 (`scratchAuto=true`, 서버는 `ranked=true` 로 저장) | 빠짐 | 나옴, A-SCR 표지 있음 | `fe.ts:148`, `fe.ts:154-167`, `player-recent-panel.tsx:71` |
| (b-1) LIGHT ASSIST 램프(id 3) + `ranked=true` + `scratchAuto=false` | 나옴 | 나옴 | `fe.ts:148` |
| (b-2) ASSIST 램프(id 2, 넓힌 판정·자기 TOTAL) | 빠짐 (`ranked=false`) | 나옴 | 아래 1-1 |
| (c) `score.assist` 비트 | 아래 참고 | | |
| (d) `ranked=false` 판 | 빠짐 | 나옴 | 3절 |
| (e-1) 코스 판 | 안 나옴 | 안 나옴 | `fe.ts:142-167` 은 `score` 표만 본다 |
| (e-2) A-단위 획득 | 안 나옴 | 안 나옴 | 아래 1-2 |

### 1-1. 램프와 `assistGrade`

- 램프 배열은 `CLEAR_LAMPS = NoPlay, Failed, AssistEasy(2), LightAssistEasy(3), Easy, ...` 이다(`src/server/dto/common.ts:3-15`). `MIN_CLEAR_LAMP_ID` 는 `AssistEasy` 라서 둘 다 클리어로 센다(`common.ts:33-34`).
- `assistGrade` 는 저장 열이 아니라 `deriveAssistGrade({clear, gauge})` 로 만든다(`src/server/service/domain/score/assist-grade.ts:32-33`). `activitySelection` 에 `gauge` 를 넣어 `toActivityRow` 가 계산한다(`fe.ts:19`, `fe.ts:32-56`).
- 규칙은 다음과 같다(`assist-grade.ts:24-32`).
  - `LightAssistEasy` 램프이고 게이지가 `AssistEasy` 가 아니면 게이지 등급 1~4(ASSIST EASY / CLEAR / HARD / EXHARD)다.
  - 그 밖의 램프는 모두 0이다. ASSIST 램프(넓힌 판정·자기 TOTAL)와 `AssistEasy` 게이지 클리어(LIGHT ASSIST, 등급 0)도 0이다.
- 클라이언트는 SCRATCH AUTO 클리어를 LIGHT ASSIST 로, 넓힌 판정·자기 TOTAL 클리어를 ASSIST 로 내린다(`docs/plan/2026-10-24-assist-dan.md` 1-1절 첫 행. 확인한 사실은 문서 서술이고 클라이언트 코드는 `crates/rbms-ir/src/mapping.rs:123-145` 의 `assist_level`).
- 그래서 등급 1~4(ASSIST EASY/CLEAR/HARD/EXHARD)는 사실상 SCRATCH AUTO 클리어에서만 나온다. 홈 필터가 SCRATCH AUTO 를 빼므로 홈에서는 등급 1~4 가 한 번도 안 보인다(추론).

### 1-2. 항목 (c), (e)

- (c) `score.assist` 는 비트가 아니라 `input.options.assist.length`, 즉 어시스트 태그 개수를 저장한다(`src/server/db/schema.ts:112`, `src/server/service/domain/score/score.service.ts:196`). 판정에 쓰이지 않고 `compose/trace.ts:54` 에서만 읽힌다. 어시스트 여부를 가를 근거로는 `scratchAuto`(`schema.ts:105`), `flags` JSON(`schema.ts:126`), `judgeRate`(`schema.ts:114`), `totalOverride`(`schema.ts:117`), `autoplay`(`schema.ts:118`)가 낫다.
- (e-1) 코스 판은 `course_score` 표에 따로 있고(`schema.ts:261-309`) 최근 활동 쿼리는 읽지 않는다. 같은 피드에 섞으려면 새 쿼리나 UNION 이 필요하다. 코스 제출 후 `activity-recent` 태그 무효화도 없다(4절).
- (e-2) A-단위는 `user_dan.assisted` 와 `acquiredAt`(`schema.ts:377-395`)에 저장된다. 활동 피드에는 연결돼 있지 않고 프로필 `extra.dans` 와 순위 행 `dans` 배지로만 나온다.

## 2. `ScoreRow` 와 활동 응답 타입

- `ScoreRow`(`src/features/score/score-row.tsx`)
  - props 에 `assistGrade?: AssistGrade`(14줄)와 `isScratchAuto?: boolean`(18줄)가 있다.
  - 램프 띠는 `<LampStrip lamp assistGrade/>`(36줄), 표지는 `isScratchAuto && <ScratchAutoBadge/>`(53줄), 램프 글자는 `lampLabel(lamp, assistGrade)`(54줄)로 그린다.
  - `ScratchAutoBadge`(`src/features/score/scratch-auto-badge.tsx`)는 `A-SCR` 글자와 툴팁 `pages.scratchAuto.badgeTitle` 을 쓴다. 라벨 상수는 `src/shared/constants/scratch-auto.ts:5`(`SCRATCH_AUTO_BADGE_LABEL = 'A-SCR'`)다.
  - `lampLabel` 은 `LightAssistEasy` 램프에서 `ASSIST_GRADE_LABEL`(LIGHT ASSIST / ASSIST EASY / ASSIST CLEAR / ASSIST HARD / ASSIST EXHARD)을 쓴다(`src/shared/constants/clear-lamp.ts:78-80`, `:64-70`). `AssistEasy` 램프는 `ASSIST` 다(`:25`).
  - 즉 `ScoreRow` 는 어시스트 표현을 램프 라벨·색과 A-SCR 표지로 이미 지원한다. ranked 여부 표지는 없다.
- 사용처는 두 곳이다.
  - 홈 `recent-activity-panel.tsx:37-48`: `assistGrade={row.assist_grade}` 는 넘기고 `isScratchAuto` 는 안 넘긴다.
  - 플레이어 기록 `player-recent-panel.tsx:61-73`: `isScratchAuto={row.scratch_auto}` 를 넘긴다.
- 활동 응답 타입 `ActivityRow`(`src/entities/stats.type.ts:20-32`)에는 `assist_grade`(25줄)와 `scratch_auto`(30줄)가 있고 `ranked` 와 `flags` 는 없다. 서버 쪽 `ActivityRow`(`src/server/service/domain/fe/fe.service.ts:7-23`)와 `buildActivityItem`(`fe.service.ts:52-64`)도 같다.
- `activitySelection`(`fe.ts:14-30`)은 `scoreId, loginId, playerName, clear, gauge, exScore, minbp, maxCombo, playedAt, scratchAuto, chartSha256, chartMd5, title, artist, level` 이다. `ranked` 와 `flags` 를 읽으려면 선택에 추가하고 `toActivityRow`/`ActivityRow`/`buildActivityItem`/`ActivityRow`(entities) 4곳을 함께 고쳐야 한다.
- `listRecentActivity` 는 `innerJoin(user, eq(user.id, score.userId))`(`fe.ts:146`)라서 `userId` 가 null 인 게스트 판은 어떤 경우에도 안 나온다. 정렬은 `desc(score.playedAt)` 이고 `score_recent_idx` 가 `playedAt` 인덱스다(`schema.ts:137`).

## 3. `ranked=false` 가 되는 조건과 공개 노출 위험

### 3-1. 서버 판정 (`src/server/service/domain/score/ranked-policy.ts`)

`rankedPolicy`(61-74)는 플래그를 쌓고 `ranked = rankBlockingFlags(flags).length === 0 && !isBuildBlockingRank(...)` 로 정한다. 순위를 막는 플래그와 조건은 다음과 같다.

| 조건 | 플래그 | 줄 |
| --- | --- | --- |
| `options.autoplay` | `AUTOPLAY` | 63 |
| `assist` 배열에 `AUTO_SCRATCH` 외 태그(예: `CUSTOM_JUDGE`) | `ASSIST` | 35, 64 |
| `judge_rate > 100` | `JUDGE_WIDTH` | 23, 65 |
| `total_override > 0` | `TOTAL_OVERRIDE` | 66 |
| 게스트 제출 | `GUEST` | 68 |
| 빌드가 `untrusted` | 플래그 아님. `isBuildBlockingRank` 가 직접 막는다 | 58-59, 70 |
| 빌드가 `unknown` 이고 `REQUIRE_BUILD_HASH=true` | 같음 | 58-59, 70 |

- 순위를 막지 않는 중립 플래그는 `SCRATCH_AUTO` 와 `UNKNOWN_BUILD` 이다(27, 38줄).
- 배속(HI-SPEED)은 `ranked` 판정에 없다. `hispeed` 는 저장만 한다(`score.service.ts:187`).
- `SCRATCH_AUTO` 는 `scratch_auto` 가 true 이거나 `assist` 에 `AUTO_SCRATCH` 가 있을 때다(31줄).
- 중요한 점이 있다. 신뢰 안 하는 빌드는 `flags` 에도 `UNKNOWN_BUILD`(중립)로만 남는다. `buildTrustFlags` 는 TRUSTED 가 아니면 `UNKNOWN_BUILD` 를 붙이기 때문이다(56줄). 그래서 `ranked=false` 판을 `flags` 로 걸러서는 "빌드 때문에 막힌 판"을 구별할 수 없다. 구별하려면 `score.clientBuildSha256`(`schema.ts:122`)로 `client_build.trusted`(`schema.ts:457`)를 조인해야 한다(추측: 허용 목록 로직은 `service/shared/integrity/build-allowlist.ts` 에 있고 읽지 않았다).
- 코스 판의 `ranked` 판정은 단일 곡과 같다(`docs/backend/api-spec.md:316`).

### 3-2. 오토플레이 판은 서버에 저장되는가

- 서버는 저장한다. 오토플레이 판 제출을 거절하는 분기가 없고(`src/server/route/score.ts:33-113`), `autoplay` 열(`schema.ts:118`)에 true 로 저장하며 `ranked=false`, `flags=['AUTOPLAY']` 가 된다. 문서의 예도 같다(`docs/backend/api-spec.md:244`). 시험은 `tests/service/ranked-policy.test.ts:25-34` 다.
- 공식 클라이언트는 보내지 않는다. `ir_submission_block_reason` 이 `autoplay`, 리플레이 보기, 연습, 재생 속도 변경을 막는다(`apps/rbms-player/src/lib.rs:344-357`). 근거 결정은 `docs/acknowledge/2026-10-14-adr-save-and-send-every-played-run.md` 결정 1이다. 오토플레이가 서버에 도달하는 길은 API 토큰으로 직접 보내는 경우뿐이다(추론).
- 반면 넓힌 판정·자기 TOTAL·어시스트 옵션 판은 공식 클라이언트가 보내고 서버가 `ranked=false` 로 저장한다(위 ADR 결정 3).

### 3-3. `ranked=false` 를 홈에 그대로 넣을 때의 문제

1. 홈은 로그인 없이 보는 공개 피드(`createFeActivityRecentRoute`, `src/server/route/fe.ts:51-54`)이고 `withPublicRead` 를 쓴다. 지금 필터는 "순위에 반영되는 제출만"이라는 약속이다. 문구도 이렇게 못박혀 있다(`messages/ko/browse.json:22`).
2. `ranked=false` 에는 오토플레이, 미신뢰 빌드, 판정폭 확대 같은 조작성·비교 불가 판이 섞인다. 서버는 여기에 "순위 제외" 표지를 활동 응답에 실어 주지 않는다(`ranked`/`flags` 필드 없음).
3. 표지 없이 섞으면 홈에서 "클리어 램프 + EX 점수"가 정상 판과 구별 안 된다. 다만 플레이어 기록(`listPlayerRecent`)은 이미 ranked 무관 전체를 보여 주고 `ranked` 표지가 없다(`player-recent-panel` 의 note 문구, `messages/ko/browse.json:74`). 이 점은 현재 동작의 비일관이다.
4. 판정폭 확대·TOTAL 변경 판은 `AssistEasy` 램프(id 2)로 내려가므로 램프로 구별되지만, SCRATCH AUTO 와 달리 기록 값은 비교 불가라는 설명이 필요하다.
5. 홈 쿼리는 `limit` 20이라(`recent-activity-panel.tsx:16`) 조작성 판이 몰리면 정상 판이 밀려난다. `ranked=false` 를 막을 필터가 없으면 도배에 취약하다(추정).

## 4. 캐시 태그와 무효화, 시험, 문서 위치

### 4-1. 캐시

- 서버 캐시 태그 `activityRecent: () => 'activity-recent'`(`src/server/service/read-cache.ts:19`).
- 읽기: `getRecentActivityCached`(`read-cache.ts:192-197`)가 `'use cache'`, `cacheLife('minutes')`, `cacheTag(CACHE_TAG.activityRecent())` 로 `compose().feService.recentActivity(limit)` 를 부른다. 공개 읽기 래퍼는 `readRecentActivity`(`src/server/service/public-reads.ts:76`)다.
- 무효화는 한 곳이다. `revalidateOnScoreSubmit`(`read-cache.ts:75-88`, 80줄)이 `revalidateTag(activityRecent, 'minutes')` 를 하고, 호출은 `src/server/route/score.ts:108` 이다.
- 코스 제출은 `activity-recent` 를 무효화하지 않는다(`revalidateOnCourseSubmit`, `read-cache.ts:220-224`: courseRanking, courseBest, playerStats 만). `docs/web/architecture.md:406` 은 코스 제출이 `activity-recent` 를 무효화한다고 썼는데 코드와 어긋났다(2026-11-02 에 코드대로 고쳤다). 코스를 피드에 넣는다면 무효화를 추가해야 한다(문서 오류이고 코드가 맞다).
- 클라이언트 캐시 키는 `QUERY_KEY.ACTIVITY.RECENT(params)`(`src/shared/constants/query-key.ts:63-66`), `activityRecentQueryOptions`(`src/entities/stats.query.ts:12-16`), `useActivityRecent`(26줄)이다. 홈 서버 프리패치는 `home-content.tsx:18-19` 에서 같은 옵션을 쓴다.
- 새 필드를 넣어도 키와 태그 이름은 안 바뀐다. 다만 응답이 바뀌므로 배포 후 최대 `minutes` 동안 옛 응답이 남을 수 있다(추측).

### 4-2. 시험 (web/tests)

- 영향 시험은 `tests/db/scratch-auto-ranking.test.ts:193-201`("홈 최근 활동은 SCRATCH AUTO 판을 빼고 플레이어 플레이 기록은 표시와 함께 넣는다")이다. 홈 목록에 SCRATCH AUTO 가 없음을 `['dave',false],['bob',false],['alice',false]` 로 단정한다. `RBMS_DB_E2E` 환경변수가 있어야 돈다(`:9-12`).
- `tests/service/fe-player-history.test.ts` 는 `activityRow` 모양(`:8-25`), `stubDb`(`:38-48`), `playerRecent` 시험을 가진다. `ActivityRow` 에 필드를 더하면 이 팩토리와 `tests/service/chart-meta.test.ts:5,165-166`(`toActivityItem`), `chart-meta.test.ts:176`, `tests/service/player-dans.test.ts:18` 의 stub 이 같이 영향을 받는다.
- `tests/components/browse-locales.test.tsx` 는 `PlayerRecentPanel` 을 렌더한다(`:19,99-117`). 홈 패널 렌더 시험은 확인되지 않았다.
- `tests/service/ranked-policy.test.ts`(`:19-146`)는 `rankedPolicy` 정책 시험이다. 이 정책을 안 건드리면 바꿀 필요 없다.
- 활동 라우트 시험(`tests/route/*`)에는 `fe/activity` 시험이 없다(grep 결과 없음).

### 4-3. 문서와 문구 위치

- `docs/backend/api-spec.md:491-493`: 493줄이 "`/api/fe/activity/recent` 는 SCRATCH AUTO 판을 늘 뺀다"고 못박는다. 492줄은 활동 항목의 `assist_grade` 를 설명한다.
- `docs/web/architecture.md:30`, `:129`: "전체 최근 제출 피드(ranked 이고 SCRATCH AUTO 가 아닌 판)". `:395,402,406,422` 가 캐시·무효화 표다.
- `docs/acknowledge/2026-10-19-adr-web-clear-and-play-history.md:25`: "홈의 최근 활동은 SCRATCH AUTO 판을 계속 뺀다(이 지시의 범위 밖)". 이번 변경이 이 결정을 뒤집는다.
- 문구: `messages/ko/browse.json:22`, `messages/en/browse.json:22`, `messages/ja/browse.json:22`(`browse.home.recent.note`). 플레이어 쪽 note 는 `messages/ko/browse.json:74` 이다.
- 가이드: `src/widgets/guide/guide-sections.ts:55` 의 `ranked.rows.scratchAuto` 설명(`messages/ko/browse.json:549-552`)이 순위 쪽 SCRATCH AUTO 설명이다. 홈에 대한 언급은 확인되지 않았다.

## 5. 추천 범위안 (사용자 결정용)

### 안 A (추천): ranked 인 SCRATCH AUTO 판을 홈에 넣고 A-SCR 표지를 붙인다

- 서버: `fe.ts:148` 의 `eq(score.scratchAuto, false)` 를 빼고 `eq(score.ranked, true)` 만 둔다.
- 화면: 홈 패널에 `isScratchAuto={row.scratch_auto}` 를 넘긴다. 램프 라벨은 이미 LIGHT ASSIST / ASSIST EASY~EXHARD 로 나온다.
- 문구 3개 언어의 note 에서 "SCRATCH AUTO 판은 이 목록에 나오지 않고" 부분을 고친다.
- 장점: 변경이 작고, 순위 반영 판만 나오므로 3-3의 위험이 없다. 응답 모양은 변하지 않는다.
- 한계: 넓힌 판정·자기 TOTAL·어시스트 옵션의 `ranked=false` 판(ASSIST 램프)은 계속 빠진다.

### 안 B: 안 A + 어시스트로 인해서만 unranked 가 된 판도 넣는다

- 조건은 `ranked=false` 이면서 막은 사유가 `ASSIST`/`JUDGE_WIDTH`/`TOTAL_OVERRIDE` 이고 오토플레이·게스트가 아닌 판으로 한다.
- 구현은 `ranked=true OR (autoplay=false AND flags 에 AUTOPLAY 없음 AND (judgeRate>100 OR totalOverride>0 OR assist>0))` 같은 조건이다. `flags` 는 JSON 이라 SQL 에서 `JSON_CONTAINS` 가 필요하다.
- 문제: 미신뢰 빌드가 같이 걸린 판은 3-1처럼 `flags` 만으로 구별되지 않아 `client_build` 조인이 추가로 필요하다.
- 문제: 판 위에 "순위 제외" 표지가 있어야 하므로 `ActivityRow` 4곳(서버 선택, `toActivityRow`, 서버 `ActivityRow`/`buildActivityItem`, 클라이언트 `stats.type.ts`)과 `ScoreRow` 표지(새 prop), 메시지 3개 언어, 시험 stub 들을 고쳐야 한다.
- 이 경우 홈이 "순위에 반영되는 제출만" 이라는 의미를 잃으므로 note 를 다시 써야 한다.

### 안 C: 안 A 로 시작하고 별도 "어시스트 활동" 소스는 나중에

- 안 A 를 먼저 내고, 코스 어시스트 클리어·A-단위 획득을 피드에 넣는 일은 별도 작업으로 둔다. 데이터가 다른 표(`course_score`, `user_dan`)에 있어 UNION 이나 별도 쿼리가 필요하고, 코스 제출 때 `activity-recent` 무효화가 없어 이 부분이 먼저 필요하다(4-1).

추천 근거: 안 A는 "순위 반영만 노출"이라는 공개 피드 약속을 지키면서 사용자가 요청한 "어시스트 관련 활동"(SCRATCH AUTO 와 그 등급 램프)을 모두 보이게 한다. 안 B 는 사용자가 원하는 범위가 "넓힌 판정 판"까지인지 확인이 필요하다.

## 6. 구현 시 수정할 파일 목록 (안 A 기준, 안 B 는 괄호)

| 파일 | 변경 요지 |
| --- | --- |
| `src/server/compose/fe.ts` | 148줄 `where(and(eq(score.ranked, true), eq(score.scratchAuto, false)))` 를 `where(eq(score.ranked, true))` 로. (안 B: `activitySelection` 에 `ranked`, `flags` 추가와 조건 확장, `toActivityRow` 입력 타입 수정) |
| `src/widgets/home/recent-activity-panel.tsx` | `ScoreRow` 에 `isScratchAuto={row.scratch_auto}` 추가 |
| `messages/ko/browse.json`, `messages/en/browse.json`, `messages/ja/browse.json` | `browse.home.recent.note`(22줄)에서 SCRATCH AUTO 제외 문구를 "A-SCR 표지가 붙은 SCRATCH AUTO 판도 표시" 취지로 수정 |
| `tests/db/scratch-auto-ranking.test.ts` | 193-201줄 홈 최근 활동 시험: SCRATCH AUTO 판이 `scratch_auto: true` 로 포함됨을 기대하도록 갱신(DB 시험이라 `RBMS_DB_E2E` 환경이 필요) |
| `docs/backend/api-spec.md` | 493줄 "SCRATCH AUTO 판을 늘 뺀다" 수정, 갱신일 헤더(3줄) 갱신 |
| `docs/web/architecture.md` | 30줄, 129줄 서술 수정. 406줄은 코드와 어긋나므로 같이 바로잡거나 코스 무효화를 추가 |
| `docs/acknowledge/` | 새 결정 기록(2026-10-19 ADR 25줄의 "계속 뺀다" 결정을 뒤집음). 사용자 결정 원문 기록 |
| (안 B 추가) `src/server/service/domain/fe/fe.service.ts`, `src/entities/stats.type.ts`, `src/features/score/score-row.tsx`, `tests/service/fe-player-history.test.ts`, `tests/service/chart-meta.test.ts`, `tests/service/player-dans.test.ts` | `ranked` 표지 필드와 표지 렌더링, stub 팩토리 갱신 |

변경하지 않는 것: 캐시 태그 이름, `QUERY_KEY.ACTIVITY.RECENT`, `revalidateOnScoreSubmit`(이미 `activity-recent` 무효화함), `rankedPolicy`, DB 스키마(마이그레이션 불필요).

## 7. 사용자 결정이 필요한 지점

1. 범위: **안 A (추천: 순위 반영 판만 노출하는 약속을 지키고 변경이 작음)** / 안 B (넓힌 판정·TOTAL 변경 판도 포함, 표지·조인 추가 필요) / 안 C (안 A 먼저, 코스·A-단위는 후속).
2. 코스 어시스트 클리어와 A-단위 획득을 최근 활동에 넣을지: **지금은 제외(추천: 별도 표 조회가 필요한 큰 작업)** / 포함.
3. 홈 문구 방향: **"SCRATCH AUTO 판은 A-SCR 표지와 함께 표시" (추천)** / 문구에서 SCRATCH AUTO 언급 삭제.
4. 플레이어 기록(`listPlayerRecent`)에 `ranked=false` 표지를 달지: **이번 범위 밖(추천)** / 포함. 현재는 표지 없이 전체를 보여 줘 홈과 의미가 달라진다.

## 8. 최소 검증 명령

`package.json` 스크립트를 확인했다(`web/package.json`: `typecheck` = `next typegen && tsc --noEmit`, `lint` = `eslint`, `test` = `bun test`, `verify` = 셋 모두). cargo 는 서버 변경만이라 해당 없음.

- 코드·문구 변경(안 A): 작업 디렉터리 `web/` 에서 `bun test tests/service/fe-player-history.test.ts tests/service/chart-meta.test.ts tests/components/browse-locales.test.tsx` 로 `ActivityRow`/문구 계약을 확인하고, 이어 `bun run typecheck` 한 번이면 충분하다.
- SQL 조건 변경은 DB 시험이 직접 덮는다. 마이그레이션을 마친 임시 MySQL 이 있을 때만 `RBMS_DB_E2E=1 DATABASE_URL=... bun test tests/db/scratch-auto-ranking.test.ts` 로 돌린다. 환경이 없으면 실행하지 못한 위험으로 보고하고, 테스트 부채는 `docs/quality-assurance` 에 기록한다(`tests/db/scratch-auto-ranking.test.ts:9-12` 의 skip 조건).
- 안 B 를 고르면 위에 더해 `bun test tests/service/ranked-policy.test.ts` 를 포함한다.
- 같은 입력의 성공 결과는 커밋 전까지 재사용하고, 실패 수정 뒤에만 관련 검사를 1회 다시 돌린다.

## 9. 확인하지 못한 것

- 홈 패널을 직접 렌더하는 컴포넌트 시험 유무는 grep 결과로 없다고 판단했지만 모든 시험 파일을 열어 보지는 않았다.
- `service/shared/integrity/build-allowlist.ts` 의 신뢰 판정 세부와 `client_build.trusted` 를 안 읽었다. 안 B 의 빌드 구별 구현은 이 파일 확인이 선행돼야 한다.
- `ja`/`en` 문구는 22줄 위치만 확인했고 전체 맥락(주변 키)은 보지 않았다.

관련 파일 경로: `/Users/hyunseokbyun/development/R-BMS/web/src/server/compose/fe.ts`, `/Users/hyunseokbyun/development/R-BMS/web/src/widgets/home/recent-activity-panel.tsx`, `/Users/hyunseokbyun/development/R-BMS/web/src/server/service/domain/score/ranked-policy.ts`, `/Users/hyunseokbyun/development/R-BMS/web/src/server/service/read-cache.ts`, `/Users/hyunseokbyun/development/R-BMS/web/tests/db/scratch-auto-ranking.test.ts`.