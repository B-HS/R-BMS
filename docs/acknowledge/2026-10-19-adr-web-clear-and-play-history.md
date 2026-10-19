# ADR: 웹의 클리어 기준, SCRATCH AUTO 포함 기본 켬, 플레이 기록 페이지

> 최종 갱신 2026-10-19 · 대응 커밋 e2c829f 이후 작업 트리
> 상태: 유효 · 결정일 2026-10-19 · 결정자 사용자(지시 3건). 이 ADR 의 2) 는 [2026-10-14-adr-scratch-auto-ranking-toggle.md](2026-10-14-adr-scratch-auto-ranking-toggle.md) 의 "화면 기본은 끔"을 바꾼다.

## 배경

웹을 쓰던 사용자가 세 가지를 짚었다. (1) L-ASSIST 램프가 클리어로 세어지지 않는다. 클리어 수는 `MIN_CLEAR_LAMP_ID = clearLampToId('Easy')` 를 기준으로 리더보드 SQL 과 플레이어 통계가 세고 있었다. (2) SCRATCH AUTO 포함 토글이 꺼진 채로 시작한다. (3) 플레이어 페이지의 "최근 기록"이 20판까지만 나오고 그 이상은 볼 방법이 없다.

## 결정

### 1) 클리어는 NO PLAY 와 FAILED 가 아닌 모든 램프

- 램프 순서(`NoPlay`, `Failed`, `AssistEasy`, `LightAssistEasy`, `Easy`, ...) 에서 `AssistEasy` 이상이 클리어다. 서버 정의는 `web/src/server/dto/common.ts` 의 `MIN_CLEAR_LAMP_ID = clearLampToId('AssistEasy')` 하나이고, 화면 쪽 정의는 `web/src/shared/constants/clear-lamp.ts` 의 `MIN_CLEAR_LAMP = 'AssistEasy'` 하나다. 서버는 화면 상수를 import 하지 않으므로(두 램프 목록은 원래 따로 있다) `web/tests/dto/clear-lamps.test.ts` 가 램프 목록과 기준의 일치를 지킨다.
- 이 상수를 쓰는 곳은 리더보드 `cleared_charts` 집계(`web/src/server/compose/fe.ts`, `count(distinct case when chart_best.clear >= ...)`)와 플레이어 통계 `cleared`(`web/src/server/service/domain/fe/fe.service.ts`)이고, 둘 다 상수만 바꿔 고쳤다. 플레이어 통계 화면의 "클리어 차트"·"클리어 비율"과 리더보드의 "클리어" 열이 따라온다.
- 발광 표의 램프 필터(`web/src/widgets/table/table-snapshot-filter.ts`)도 같은 기준으로 맞췄다: `failed` 는 FAILED 만, `unclear` 는 NO PLAY·FAILED(옛 "EASY 미만"), 새 `cleared` 는 ASSIST 이상("클리어 (ASSIST 이상)"). `easy`("EASY 이상")·`clear`("CLEAR 이상"= 램프 이름 Normal)·`hard` 이상은 램프 이름 문턱이라 그대로다. 주소 파라미터 `lamp` 의 값 `cleared` 가 늘었다.
- 마이그레이션은 필요 없다. 클리어 수를 미리 계산해 둔 열은 없고(`user.rank_points`·`total_plays` 는 이 집계와 무관한 옛 열), 질의 때 `chart_best.clear` 에서 센다.
- 순위(ranked) 정책(`web/src/server/service/domain/score/ranked-policy.ts`)은 바꾸지 않았다. 겹치는 점: L-ASSIST 램프는 클라이언트가 SCRATCH AUTO 판에 주는 램프(`crates/rbms-ir/src/mapping.rs` 의 `LIGHT_ASSIST`)라 2) 의 기본 켬과 함께 리더보드의 클리어에 기본으로 센다. AssistEasy 램프는 판정폭을 넓힌 판(`CUSTOM_JUDGE_ASSIST`)에서 나오는데 그런 판은 `ASSIST` 표지로 unranked 라 `chart_best` 에 올라가지 않으므로 이 집계에는 실제로 거의 들어오지 않는다(옛 클라이언트나 직접 제출의 예외만 가능).

### 2) SCRATCH AUTO 포함의 화면 기본은 켬, 서버 API 기본은 끔 그대로

- 화면: 차트 순위·코스 순위·플레이어 리더보드의 "SCRATCH AUTO 포함" 스위치는 켠 채로 시작한다. 주소 파라미터 `include_scratch_auto` 는 없으면 켬, `false` 면 끔, `true` 는 켬이다. 스위치를 끄면 `include_scratch_auto=false` 를 주소에 쓰고, 켜면 파라미터를 지운다(`web/src/shared/lib/search-params.ts` 의 `readBooleanParam(params, key, fallback)`·`writeBooleanParam(value, fallback)`, `web/src/shared/lib/scratch-auto-param.ts`, 기본값 상수 `DEFAULT_INCLUDE_SCRATCH_AUTO` 는 `web/src/shared/constants/scratch-auto.ts`).
- 서버 프리페치와 클라이언트 쿼리 키가 같은 캐시를 가리키도록 하드코딩된 `include_scratch_auto: false` 를 기본값 상수로 바꿨다: 발광 표 곡 상세 창의 "차트 순위" 버튼(`web/src/widgets/table/table-chart-details.tsx`)과 차트 페이지 메타데이터(`web/src/app/(app)/charts/[hash]/page.tsx`).
- 서버 API(`includeScratchAutoSchema`, `/api/charts/{hash}/ranking`·`/best`, `/api/courses/{hash}/ranking`·`/best`, `/api/fe/...`)의 기본값은 끔 그대로다. 근거: Rust 클라이언트(`crates/rbms-ir/src/http.rs` 의 `/charts/{md5}/ranking?limit=..`, `/charts/{md5}/best?player=..`, `/courses/{hash}/ranking?limit=..`)는 `include_scratch_auto` 를 싣지 않고 서버의 기본 목록에 기댄다([reference/ir-api.md](../reference/ir-api.md) 는 "클라이언트는 싣지 않는다"고 적는다). API 기본을 켬으로 바꾸면 곡 선택의 IR RANKING 패널·결과 화면의 순위·내 최고 기록이 배포된 클라이언트에서 몰래 바뀐다. `web/tests/components/scratch-auto-param.test.ts` 가 API 기본이 끔인 것을 못 박는다.
- 홈의 "최근 활동"은 SCRATCH AUTO 판을 계속 뺀다(이 지시의 범위 밖, 문구만 "차트·코스 순위에서 볼 수 있습니다" 로 고쳤다). 가이드(`web/src/widgets/guide/guide-sections.ts`)와 순위 각주 문구도 새 기본에 맞췄다.

### 3) 플레이 기록: 모든 판을 페이지로

- 패널 제목·로딩·오류·aria-label 의 "최근 기록"을 "플레이 기록"으로 바꿨다(`web/src/widgets/player/player-recent-panel.tsx`). 홈의 "최근 활동"·검색의 "최근 등록 순" 같은 다른 "최근"은 그대로다.
- `GET /api/fe/players/{playerId}/recent` 가 `page`(기본 1)·`limit`(기본 20, 최대 100)을 받고 그 플레이어의 모든 판을 `played_at` 내림차순 + `score.id` 내림차순(동률 안정 정렬)으로 준다. 응답은 다른 목록처럼 `{ success, data, pagination: { page, limit, total, totalPages } }`(`pageEnvelope`, `toPagination`). 라우트 경로·`QUERY_KEY.PLAYER.RECENT`·`usePlayerRecent`·`PlayerRecentPanel` 이름은 바꾸지 않았다(응답에 `pagination` 이 붙고 `data` 는 그대로라 호환).
- 페이지 상태는 기존 목록(순위·검색)처럼 주소다: `/players/{id}?page=N`. 서버 컴포넌트(`player-detail-content.tsx`)가 `searchParams` 로 읽어 같은 키로 프리페치하고, 패널의 `Pager` 가 `router.push`(`scroll: false`)로 옮긴다. 1페이지는 파라미터를 쓰지 않는다. 범위를 벗어난 페이지는 "이 페이지에는 기록이 없습니다" 와 "마지막 페이지로" 버튼을 보인다.
- 캐시: `getPlayerRecentCached` 의 태그 `player-recent:{loginId}` 는 인자(page, limit)와 무관하게 하나라서 판 제출·리플레이 업로드의 `revalidateTag` 가 모든 페이지를 한 번에 무효화한다(`web/src/server/service/read-cache.ts`).
- 인덱스: `score_player_idx (user_id, played_at)` 가 있고 질의(`WHERE user_id = ? ORDER BY played_at DESC, id DESC LIMIT/OFFSET`)와 개수(`count WHERE user_id = ?`)가 쓴다. 임시 MySQL 8.4.11 에서 EXPLAIN 으로 확인했고 플레이어 한 명의 판 1만 개에서 OFFSET 8,000 이 약 3.5ms 였다. 새 마이그레이션은 없다.

## 이유

사용자 원문 세 줄이 근거다(아래). 클리어는 "FAILED 가 아닌 건 다"라는 정의라 램프 이름 목록을 더 늘리지 않고 문턱 하나로 둔다. SCRATCH AUTO 는 사용자가 쓰는 판의 대부분이 SCRATCH AUTO 라 기본이 꺼져 있으면 자기 기록이 순위·통계에서 사라져 보였다(endpoint-tasks 의 관찰 기록 [backend/endpoint-tasks.md](../backend/endpoint-tasks.md): 운영에서 `bhs` 의 판은 모두 SCRATCH AUTO). API 기본을 그대로 두는 것은 클라이언트 동작을 몰래 바꾸지 않으려는 것이다.

## 기각된 대안

| 대안 | 기각 이유 |
| --- | --- |
| API 의 `include_scratch_auto` 기본값도 켬으로 | Rust 클라이언트가 파라미터를 싣지 않고 기본에 기대 곡 선택 패널·내 최고 기록이 배포 클라이언트에서 몰래 바뀐다 |
| 플레이 기록을 무한 스크롤로 | 순위·검색이 이미 `Pager` + 주소 `page` 를 쓴다. 기존 패턴을 따랐다 |
| 새 라우트(`/history`)를 만들고 `/recent` 는 20판 그대로 | 응답에 `pagination` 만 더해도 호환되고 소비자는 이 패널 하나다 |
| 클리어 수를 미리 계산해 열로 저장 | 마이그레이션이 필요하고 집계가 `chart_best` 에서 이미 한 번에 나온다 |
| 표 필터의 `unclear` 를 "EASY 미만"으로 유지 | L-ASSIST 를 클리어로 센다는 결정과 어긋난다 |

## 영향과 관련 파일

- 서버: `web/src/server/dto/common.ts`, `dto/fe.ts`(`playerRecentQuerySchema`), `route/fe.ts`, `service/public-reads.ts`, `service/read-cache.ts`, `service/domain/fe/fe.service.ts`, `compose/fe.ts`.
- 화면: `web/src/shared/constants/{clear-lamp,scratch-auto}.ts`, `shared/lib/{search-params,scratch-auto-param}.ts`, `entities/player.{query,type}.ts`, `widgets/player/{player-detail-content,player-detail-panel,player-recent-panel}.tsx`, `widgets/{chart,course,leaderboard}/*`, `widgets/table/{table-snapshot-filter.ts,table-chart-details.tsx}`, `widgets/guide/guide-sections.ts`, `widgets/home/recent-activity-panel.tsx`, `app/(app)/players/[playerId]/page.tsx`, `app/(app)/charts/[hash]/page.tsx`.
- 테스트: `web/tests/dto/clear-lamps.test.ts`, `tests/service/fe-player-history.test.ts`, `tests/components/{scratch-auto-param,player-recent-panel,search-params,table-snapshot,entity-query-wiring,guide-article}.test.ts(x)`, `tests/lib/public-reads.test.ts`, `tests/db/clear-lamps-and-history.test.ts`(임시 MySQL), `tests/db/scratch-auto-ranking.test.ts`.
- 문서: `docs/web/{architecture,tasks,2026-10-14-foundation-contract}.md`.
- 남은 위험: `page` 가 0 이하이거나 소수인 주소(`?page=0`)는 순위·검색과 똑같이 서버 읽기가 검증 오류로 끝나 패널의 오류 상태("플레이 기록을 불러오지 못했습니다")가 된다. 깊은 OFFSET 은 플레이어의 판 수만큼 훑는다(위 측정치, 현재 규모에서 문제 없음).

## 날짜와 사용자 원문

2026-10-19.

1. "L-ASSIST 는 클리어로 안치던데 FAILED 가 아닌건 다 클리어 처리해놔"
2. "SCRATCH AUTO 포함 off가 default인데 on이 default로되게 해놔"
3. "사용자의 최신기록이 20개인가 ? 까지밖에안나오는데 paging으로 플레이 전체 기록 보이게하고 \"최근 기록\" 을 \"플레이 기록\" 으로 바꾸면 되겠지"
