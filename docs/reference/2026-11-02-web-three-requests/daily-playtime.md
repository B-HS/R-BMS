> 최종 갱신 2026-11-02 · 대응 커밋 `28085a7`·`8af1a39`·`f737c4d`(조사 시점 `837d652`, 구현은 이 작업) · 조사 wf_db20f1ba-f79

# 일별 플레이 시간 그래프·표 추가를 위한 읽기 전용 조사 보고서 (2026-11-02)

표기: **[확인]** 은 파일을 직접 읽어 본 사실, **[추측]** 은 코드나 문서 근거로 짐작한 것이고, 미확인 항목은 따로 적었습니다. 줄 번호는 `/Users/hyunseokbyun/development/R-BMS` 기준입니다. 경로 앞의 `web/` 는 `web/` 폴더를 뜻하고, `crates/`·`apps/`·`contract/`·`docs/` 는 저장소 루트의 폴더입니다. 파일 수정, git, 네트워크, DB 접속, `.env*` 열람은 하지 않았습니다.

## 구현 결과 (2026-11-02, 커밋 `8af1a39`)

7절의 결정은 전부 추천안으로 구현됐다(플레이어 페이지 통계 패널의 모드 전환, 판 간격 추정, KST 자정, 최근 30일, 모든 판). 결정 기록: [일별 플레이 시간 ADR](../../acknowledge/2026-11-02-adr-daily-play-time.md). 판 길이 실측(클라이언트 `extra`)은 보류했다.

- 채택한 규칙(`web/src/server/service/domain/fe/player-play-days.ts`): 판 끝 시각 오름차순, 앞 판 끝과 간격 10분 이하(`PLAY_SESSION_GAP_MS`)면 그 간격, 아니면·첫 판이면 2분(`PLAY_DEFAULT_MS`)을 판 끝 시각의 KST 날에 더함, 창 30일(`PLAY_DAYS_WINDOW`), 창 바로 앞의 한 판은 간격 계산에만 씀. 차트 라이브러리 없이 CSS 막대.
- API: `GET /api/fe/players/[playerId]/play-days`(`withPublicRead`, 쿼리 없음) → `{ days:[{ date, plays, play_ms }] 30개, total_plays, total_ms }`. 라우트 `app/api/fe/players/[playerId]/play-days/route.ts`, `route/fe.ts` 의 `createFePlayerPlayDaysRoute`, `public-reads.ts` 의 `readPlayerPlayDays`, `compose/fe.ts` 의 `listPlayerPlayedAts`(Drizzle).
- 캐시: `read-cache.ts` 의 `getPlayerPlayDaysCached`(`'use cache'`, `cacheLife('minutes')`, 태그 `player-play-days:{loginId}`)이고 캐시 키에 KST 오늘 0시 `todayStart` 가 들어가 자정에 창이 넘어간다. `revalidateOnScoreSubmit` 이 무효화한다.
- 화면: `widgets/player/player-stats-panel.tsx`(`ToggleGroup` 모드 전환), `widgets/player/player-play-days.tsx`(`PlayerPlayDays`, 모드를 고른 뒤에만 마운트해 조회), `features/stats/play-day-chart.tsx`, `features/stats/play-day-table.tsx`, 문구 `web/messages/*/browse.json` 의 `browse.player.playDays`, 포맷 `shared/lib/format.ts` 의 `formatHoursMinutes`, 쿼리 `entities/player.query.ts` 의 `playerPlayDaysQueryOptions`·`QUERY_KEY.PLAYER.PLAY_DAYS`.
- **코스 판 미확인 항목(2절 140행, 7절 4번, 9절)은 확인됐다**: 코스의 스테이지는 곡별 `score` 행으로 따로 올라온다(근거 `apps/rbms-player/src/app_result.rs:158-196`, `web/src/server/route/course.ts:114-125`). 그래서 `course_score` 는 읽지 않아도 코스 도중이 공백이 되지 않고, 읽으면 두 번 센다. 코스 제출(`revalidateOnCourseSubmit`)은 이 태그를 무효화하지 않는다.
- 검증(실제 결과): 웹 `bun test` 2,004 통과. 실행하지 못한 것: play-days Drizzle 쿼리의 창 경계 DB 통합 시험(테스트 부채 3.142), 브라우저 실화면(QA 3.26 T3~T5), 일본어 문구 감수(3.143), 추정 상수의 체감 검증(3.144). 새 컴포넌트 시험은 `fetch` 를 바꾸지 않고 쿼리 캐시를 미리 채운다(`globalThis.fetch` 를 바꾸는 방식은 다른 DOM 시험 파일과 함께 돌 때 실패했다).

## 0. 핵심 요약

1. **서버에는 판 길이나 시작 시각이 저장돼 있지 않습니다.** [확인]
   - 클라이언트가 보내는 점수 본문에는 `played_at` 한 개(판이 끝난 시각, ms)만 있습니다. 판 길이, 시작 시각, 곡 길이 필드는 없습니다.
   - `chart` 표에도 곡 길이 열이 없습니다.
   - 따라서 서버만 고치는 방식으로는 정확한 플레이 시간을 낼 수 없고, 판과 판 사이 간격으로 추정하는 방식이 기본안이 됩니다.
2. **곡 길이를 부분적으로 아는 경로가 세 가지 있습니다.** [확인]
   - 모두 일부 판만 덮습니다.
   - 곡별 패턴이 업로드된 곡은 `chart_pattern.features.length_ms` 가 있습니다.
   - 본인이 판별 trace 를 올린 판은 trace 의 `play_time_ms` 와 분석 요약의 구간 끝이 있습니다.
   - 리플레이가 있는 판은 `replay.duration_us` 가 있습니다. 이 값은 곡 길이가 아니라 키 입력 이벤트의 `max(t_us) - min(t_us)` 입니다.
3. **정확한 값을 새 판부터 얻으려면 클라이언트가 `extra` 에 값을 실어 보내는 방법이 있습니다.** [확인]
   - 이미 있는 선례 `mean_offset_us` 와 `rule_version`(`crates/rbms-ir/src/dto/score.rs:96-160`)을 따르면 계약 파일 갱신과 Rust 시험이 함께 필요합니다.
   - 이미 올라간 판은 소급할 수 없습니다. 중복 제출 분기가 기존 행을 갱신하지 않기 때문입니다(`web/src/server/service/domain/score/score.service.ts:354-387`).
4. **"분석 그래프"에 해당하는 화면이 하나로 정해지지 않습니다.** 후보가 둘로 갈립니다.
   - 플레이어 페이지 통계 패널(공개)과 AI 허브 주간 요약 탭(본인 전용)입니다.
   - 그래프를 그리는 화면 중 모드 전환 UI 가 이미 있는 곳은 없습니다. 사용자 결정이 필요합니다(7절).
5. **날짜 경계 규칙이 서로 다릅니다.** [확인]
   - 화면 시각은 `Asia/Seoul` 고정입니다.
   - 주간 요약은 UTC 월~일 주입니다.
   - 사용자별 시간대 설정은 없습니다.
6. **차트 라이브러리는 없습니다.** [확인]
   - `web/package.json` 의존성에 차트 계열이 없습니다.
   - 기존 그래프는 CSS 막대와 인라인 SVG 로 그립니다.
   - 새 의존성 없이 구현할 수 있습니다.

## 1. 후보 화면 전수와 순위

### 1-1. 후보별 사실

| 후보 | 위치 | 보여 주는 것 | 모드·탭·토글 | 공개/본인 |
|---|---|---|---|---|
| 플레이어 페이지 통계 | `web/src/widgets/player/player-detail-panel.tsx:62-63`(통계·기록 패널을 순서대로 배치), `player-stats-panel.tsx:44-65` | StatTile 3개(총 제출·클리어 차트·클리어 비율)와 램프 분포 누적 막대 `LampDistribution` → `lamp-stack-bar.tsx:16-34` | 없음 | 공개 읽기(`docs/web/architecture.md:50`). 본인일 때만 판마다 "판 코치" 링크가 붙음(`player-recent-panel.tsx:36,72`) |
| 플레이어 플레이 기록 | `player-recent-panel.tsx` | 판 목록과 페이저(그래프 아님) | 없음 | 공개 |
| AI 허브 주간 요약 | `web/src/widgets/ai/ai-weekly-panel.tsx:41-142` | 이 주·앞 주 비교 표(판 수, 친 곡, **친 날**, 처음 친 곡, 램프 갱신, 첫 클리어, EX 갱신, FAST 비율). 그래프는 없고 표만 있음(`:79-112`) | 이전·다음 주 버튼(`:62-78`, 상태는 `useState`). 허브 자체는 Radix Tabs(`ai-hub-tabs.tsx:30`) | 세션 필수(`app/(app)/ai/layout.tsx` `SessionGate`), 본인 전용 |
| AI 허브 판정 습관 | `ai-judge-habit-panel.tsx` | FAST/SLOW, 오프셋, GREEN 수치(텍스트·목록) | 없음 | 본인 전용 |
| AI 허브 플레이어 리포트 | `ai-report-panel.tsx` | 태그 잔차, 레인별 EX 격자, 텍스트 | 없음 | 본인 전용 |
| 판 코치(`/ai/plays/[scoreId]`) | `web/src/features/ai/ai-segment-chart.tsx:23-64`, 쓰는 곳 `ai-coach-panel.tsx:55` | 구간 32개의 BP 막대(버튼)와 게이지 선(SVG `polyline`) | 막대를 누르면 구간 선택 | 본인 전용 |
| 리플레이 뷰어 | `web/src/widgets/replay/replay-waterfall.tsx:25-` | 레인×시간 워터폴(인라인 SVG, `var(--color-*)` 직접 참조) | 없음 | 공개 |
| 표 진행 요약 | `web/src/features/table/table-progress-panel.tsx:1-40` | 램프 분포와 레벨별 막대. 접기·펼치기(`isDetailOpen`) 있음 | 접힘만 있음 | 표 화면. 내 기록은 세션 의존 |
| 홈 | `web/src/widgets/home/*` | 서버 통계 수치와 최근 활동 목록(그래프 없음) | 없음 | 공개 |
| `BarCountList` | `web/src/features/stats/bar-count-list.tsx:11-38` | 라벨·막대·수 목록 | 없음 | `src` 안에 사용처가 없음(검색 확인). 설계 문서도 "고치거나 없앨 것"으로 적음(`docs/web/2026-10-14-design-direction.md:667`) |

보조 사실 [확인]:

- 모드 전환에 쓸 수 있는 공용 UI 는 둘입니다.
  - `web/src/shared/ui/toggle-group.tsx`: 쓰는 곳은 `widgets/search/chart-search-filter.tsx:87-103` 의 단일 선택 토글 그룹입니다.
  - `web/src/shared/ui/tabs.tsx`: 쓰는 곳은 AI 허브와 설정 탭(`widgets/settings/settings-tabs.tsx:27-58`)입니다.
- AI 허브 탭은 URL `?tab=` 을 마운트 때 한 번만 읽고 `defaultValue` 로 씁니다(`ai-hub-tabs.tsx:18,27,30`). 탭을 눌러도 URL 이 바뀌지 않습니다.
- 허브 탭 라벨 `AI_HUB_TAB_LABEL`(`web/src/entities/ai.type.ts:209-217`)은 한국어 하드코딩입니다. 메시지 카탈로그(i18n)를 거치지 않으므로 새 탭을 만들면 같은 방식이 됩니다.
- AI 원칙 안내 문구는 "숫자, 표, 그래프는 서버가 계산한 결과"라고 합니다(`web/messages/ko/pages.json:272`). 허브가 "분석" 성격이라는 근거입니다.

### 1-2. 순위 [추측: 사용자가 가리킨 화면이 확정되지 않았습니다]

1. **플레이어 페이지 통계 패널(`PlayerStatsPanel`)에 모드 전환 추가**
   - 이유: 기록의 "그래프"(램프 분포 막대)가 있는 화면이고 공개 읽기 캐시 패턴이 있습니다. 같은 패널 안 모드 전환(램프 분포 ↔ 일별 시간)이 사용자 문장 "모드로 변경해서"와 가장 잘 맞습니다.
   - 주의: 공개 화면이라 남도 일별 시간을 보게 됩니다. 이미 `/players/[id]` 의 플레이 기록이 모든 판의 `played_at` 을 공개하고 있어(`docs/web/architecture.md:131`) 새 노출은 "추정 시간"뿐입니다.
2. **AI 허브 주간 요약 탭에 모드 추가**
   - 이유: "분석" 맥락이고 이미 "친 날"(UTC 일 수)을 셉니다. 단 주 단위(7일)라 "하루하루"에는 좁고, 본인 전용입니다.
3. **표 진행 요약(`TableProgressPanel`)**: 표 단위 진척이라 시간 개념과 맞지 않습니다.

## 2. 플레이 시간을 계산할 데이터

### 2-1. `score` 표 (`web/src/server/db/schema.ts:59-140`) [확인]

- `playedAt`: `bigint` ms(`:129`). 클라이언트가 보낸 값을 그대로 저장합니다(`score.service.ts:213`).
- **기준 시각은 판이 끝난 시각입니다.**
  - 클라이언트는 결과 화면을 만들 때 `SystemTime::now()` 를 `played_at` 으로 씁니다(`apps/rbms-player/src/app_result.rs:308,368`).
  - 리플레이 파일명(`{md5}-{played_ms}.ron`)에도 같은 값이 쓰입니다(`app_result.rs:384,391`).
  - 지연 재전송 판은 `rebuilt_played_at` 이 리플레이 파일명의 ms 를 되살리고, 없으면 초 단위로 내립니다(`apps/rbms-player/src/record_sync.rs:889-896`).
- 판 규모 대리값(길이 아님):
  - `notes`(`:96`): 전체 노트 수.
  - `passnotes`(`:97`): 판정된 노트 수. Esc·실패로 일찍 끝나면 `notes` 보다 작습니다.
  - 점수 본문 `total_notes`·`passnotes` 는 `web/src/server/dto/score.ts:57-58`.
- 판별용 열:
  - `autoplay`(`:117`), `ranked`(`:125`), `replayId`(`:128`), `mode`(`:67`), `userId`(`:63`, 사용자 삭제 시 `set null`).
  - 사용자 조회에 쓸 수 있는 색인은 `score_player_idx(user_id, played_at)`(`:136`)입니다.
- `extra`(`:131`)와 `flags`(`:126`) JSON:
  - `extra` 는 클라이언트가 보낸 객체를 그대로 저장합니다(`score.service.ts:214`). 현재 클라이언트가 채우는 키는 `mean_offset_us`, `rule_version` 둘뿐입니다(`contract/client-requests/score-normal.json`의 `"extra": { "mean_offset_us": -14655, "rule_version": 4 }`).
  - `flags` 는 서버 규칙이 정한 값(SCRATCH_AUTO, ASSIST, UNKNOWN_BUILD 등)이고 시간 정보가 없습니다.
- **판 길이나 종료 방식 열은 없습니다.**
  - 단, 서버가 만드는 `play_analysis.summary.run_end`(`finished|gauge_empty|left`)는 trace 가 있는 판에만 있습니다(2-3).

### 2-2. 곡 길이를 줄 수 있는 곳 [확인]

- `chart` 표(`schema.ts:23-57`)에 길이 열이 없습니다. `total`(`:34`)은 `#TOTAL` 게이지 값, `notes`(`:40`)는 노트 수, `extra`(`:49`)는 클라이언트가 보낸 메타 JSON 입니다.
  - 점수 제출에 딸려 오는 차트 메타는 `chart.title/artist/level` 과 해시뿐입니다(`crates/rbms-ir/src/mapping.rs:175-178` `submitted_chart`).
- `chart_pattern.features.length_ms`
  - 위치: `schema.ts:618-638`(`features` 는 `:626`), 계산은 `web/src/server/service/domain/analysis/chart-features.ts:109`(마지막 노트 시각).
  - 한계: 관리자 스크립트가 올린 곡만 있습니다(`apps/rbms-cli`, 관리자 Bearer, `docs/web/architecture.md:1-4 §11`).
  - 실제 DB 적재율은 읽을 수 없어 미확인입니다.
- `play_trace`/`play_analysis`
  - 본인이 trace 를 올린 판에만 있고, `play_time_ms` 는 trace 본문(gzip `mediumblob`)에 들어 있습니다(`schema.ts:685-723`, `web/src/server/dto/trace.ts:169`).
  - 분석 요약에는 BEAT_7K 판에 한해 `metrics.segments` 32개(`SEGMENT_COUNT = 32`, `analysis-constants.ts:72`)가 있습니다. 마지막 구간의 `to_ms` 가 `play_time_ms` 입니다(`play-metrics.ts:155-156,148`, 호출 `trace-analysis.ts:163`).
  - `play_trace` 에는 별도 길이 열이 없고 `playedAt`·`noteCount`·`byteSize` 만 있습니다.
- `replay.durationUs`
  - 위치: `schema.ts:187`(NOT NULL, 기본 0).
  - 서버 계산: 클라이언트가 값을 안 보내면 이벤트의 `max(t_us) - min(t_us)` 입니다(`replay.service.ts:54-63,152`). 마지막 입력 이벤트까지의 길이라 곡 길이나 판 길이와 같지 않습니다.
  - 계약 예: `contract/client-requests/replay-long.json` 의 `duration_us: 89450000`.

### 2-3. 클라이언트가 서버로 보내는 것 [확인]

- 점수 본문 키(`contract/client-requests/score-normal.json`): `api_version, chart, clear, client, client_build_sha256, client_platform, ex_score, extra, gauge_value, judge, judge_algorithm, max_combo, max_ex_score, minbp, mode, options, passnotes, played_at, player, replay_id, rule, seed, skin, total_notes`. **길이나 시작 시각 키는 없습니다.**
- 코스 본문(`contract/client-requests/course-normal.json`): `played_at`, `stages[]`(게이지·lntype·random)뿐이며 길이는 없습니다.
- trace(`contract/client-requests/trace-normal.json`, Rust 정의 `crates/rbms-ir/src/dto/trace.rs:50-75`):
  - `play_time_ms: 12800`: 문서 주석상 "차트가 플레이 상태에 머무는 시간"입니다.
  - `failed_at_ms`: 게이지가 비어 끝난 판에만 값이 있습니다.
- `play_time_ms` 의 정의: 마지막 노트 시각 + 5,000 ms 여유입니다(`crates/rbms-play/src/record.rs:119-125`, `PLAY_TIME_MARGIN_MS` 는 `:25`).
  - Esc·중도 포기여도 곡의 전체 길이 기준 값입니다. 실제로 친 시간이 아닙니다. [확인]
- **클라이언트 로컬 DB 는 판마다 `playtime` 을 저장하지만 서버로 보내지 않습니다.**
  - 로컬 기록: `apps/rbms-player/src/app_result.rs:442` 의 `playtime_ms: playtime_ms(last_time_us)`.
  - 일 단위 집계 `player_stat` 도 로컬에만 있습니다(`crates/rbms-store/src/scoredb.rs:548-556`, `scoredb/rows.rs:179-181`).
  - 이 값도 곡 마지막 줄의 시각(`crates/rbms-play/src/lib.rs:589-591`, `timelines.last().time_us`)이어서 중도 포기해도 전체 길이로 잡힙니다. [확인]
  - 코드 주석(`scoredb_store.rs:154-160`)은 "실제로 친 마지막 노트"라고 쓰지만 구현은 곡 끝 줄 시각입니다.
- trace 업로드 조건: 로그인 상태, 설정 `auto_upload_trace`(기본 true, `crates/rbms-config/src/schema.rs:841,882`), 서버 능력 플래그 `capabilities.traces`(`docs/web/architecture.md:700` 13-1), 과거 판은 backfill(`apps/rbms-player/src/trace_backfill.rs`) 대상입니다.
- 리플레이 업로드 조건: 로그인, `auto_upload_replay`(기본 true, `schema.rs:837`), 이벤트 존재, `wire_carries_judge` 통과(`apps/rbms-player/src/app_ranking.rs:248-255`). 리플레이 저장은 `play.auto_replay`(기본 true)가 켜져 있고 자동·리플레이 관전이 아닐 때만입니다(`app_result.rs:300`).
- 오토플레이·리플레이 관전·연습·재생 속도 변경 판은 제출되지 않습니다(`ir_submission_block_reason`, `app_result.rs:376-377` 근처). 서버가 보는 `autoplay` 열은 사실상 항상 false 입니다. [추측: 호출부 확인은 일부]

### 2-4. 중도 포기·Esc 판 [확인/추측]

- 문서 결정: 노트를 1개 이상 친 판의 Esc 는 결과 화면이 나오고 판정 수는 저장값과 같습니다(`docs/PROCESS.md:71`의 결정 9). [확인: 문서] **0개 친 Esc 는 기록이 남지 않을 가능성이 있습니다.** [추측]
- 서버에서 일찍 끝난 판을 아는 방법:
  - `passnotes < notes` 이면 일찍 끝난 판입니다. 시간 길이는 모릅니다.
  - trace 가 있으면 `run_end`, `failed_at_ms`, 각 노트의 `state` 로 도달 시각을 알 수 있습니다(`web/src/server/service/domain/trace/trace-analysis.ts:104-108`). Esc 로 끝난 판의 도달 시각은 trace 의 마지막 판정 노트 시각으로 계산해야 합니다. 이 계산은 구현된 곳이 없습니다.
- 간격 기반 추정은 포기한 판도 "다음 판 시작까지의 시간"으로 들어가므로 별도 처리가 필요 없습니다.

### 2-5. 판 사이 간격(세션)으로 추정하는 방식 [추측]

- `played_at`(끝 시각)을 사용자별로 오름차순 정렬합니다.
- 연속한 두 판의 간격 `g = t_i - t_{i-1}` 이 `SESSION_GAP` 이하면 같은 세션으로 보고, 세션 시간을 `마지막 - 처음 + 첫 판의 추정 길이`로 계산합니다.
- 장점: 모든 과거 판에 소급 적용되고 서버 변경만으로 가능합니다. 단점: 곡 선택·대기·휴식 시간이 섞입니다.
- 필요한 상수: 세션 경계 간격, 첫 판 길이 가정, 하루 상한. 매직넘버 금지 규칙에 따라 모두 상수화하고 사용자 승인이 필요합니다(7절).
- 보강 가능: 길이를 아는 판(2-2)은 길이로 간격 상한을 둘 수 있습니다. 이는 trace 조회가 필요해 본인 전용 경로에서만 가능합니다.
- 코스 판은 `course_score` 표(`schema.ts:261-309`, `playedAt` 은 `:298`)에 따로 있고 `/players/[id]/recent`(`web/src/server/compose/fe.ts:154-167`)에는 나오지 않습니다. 시간 선에 합치지 않으면 코스 도중이 긴 공백처럼 보입니다. 코스 판의 곡별 점수 행이 따로 제출되는지는 조사 시점에는 확인하지 못했으나 구현 때 확인됐다(위 "구현 결과": 곡별 `score` 행이 따로 올라온다).

### 2-6. 데이터 접근 제약 [확인]

- `play_trace`·`play_analysis` 표는 `compose/trace.ts` 만 읽는다고 문서가 고정하고 시험이 지킵니다(`docs/web/architecture.md:709` 13-1 "읽기 경로 없음", `tests/route/trace.test.ts`). trace 기반 길이를 쓰려면 `TraceService` 에 메서드를 추가하는 방식이어야 합니다. [확인: 문서 기술. 시험 본문은 열어 보지 않음]
- trace 는 본인만 볼 수 있고(ADR Q3), 공개 화면에서는 쓸 수 없습니다.

## 3. 날짜 경계

- **화면 표시 시간대는 `Asia/Seoul` 고정입니다.**
  - `web/src/shared/lib/format.ts:7`(`DISPLAY_TIME_ZONE`), `:9-24`(`formatDateTime`).
  - next-intl 요청 설정도 같은 값을 씁니다(`web/src/shared/i18n/request.ts:14`).
  - 사용자별 시간대·로케일 설정 필드는 없습니다(`timezone` 계열 검색에서 `format.ts`·`request.ts` 외에는 없음).
- **주간 요약은 UTC 월~일입니다.**
  - 주 시작 계산: `weekStartOf`(`web/src/server/service/domain/ai/weekly-summary.service.ts:108-113`), `resolveWeekStart`(`:119-125`).
  - "친 날" = `Math.floor(playedAt / DAY_MS)` 의 개수입니다(`:215`, UTC 일).
  - 화면도 `"{start} ~ {end} (UTC)"` 로 표기합니다(`web/src/widgets/ai/ai-weekly-panel.tsx:67`, 문서 `architecture.md:674`).
- 서버 저장·교환은 UTC 이고 표시는 한국 시각이라는 불일치가 이미 존재합니다. KST 새벽 0~9시 플레이는 UTC 로 전날이 됩니다.
- `formatCount` 는 `ko-KR` 로 고정입니다(`format.ts:3`). 영어·일본어에서도 숫자 형식이 한국식입니다.
- 새 일별 집계를 만들 때 필요한 선택: KST 일(권장 후보) 또는 UTC 일(7절).

## 4. 기존 집계 API 구조와 따를 패턴

### 4-1. 공개·플레이어별 읽기 (`/api/fe/players/[playerId]/stats` 가 가장 가까운 선례) [확인]

1. 라우트 어댑터: `web/src/app/api/fe/players/[playerId]/stats/route.ts`(`export const GET = createFePlayerStatsRoute()`)
2. 핸들러: `web/src/server/route/fe.ts:37-38`(`withPublicRead` = `withErrorHandling(withRateLimit('fe-read', RATE_LIMIT.PUBLIC_READ)(...))`), `:68-72`(`createFePlayerStatsRoute`), 봉투 `envelope`(`:30`), 페이지형 `pageEnvelope`(`:32`)
3. 공개 읽기: `web/src/server/service/public-reads.ts:41-47`(`parseQueryParams`), `:49-53`(`requireUserId`, 없으면 `IR_PLAYER_NOT_FOUND`), `:101`(`readPlayerStats`)
4. 캐시: `web/src/server/service/read-cache.ts` 의 `getPlayerStatsCached`(`:289-295`: `'use cache'`, `cacheLife('minutes')`, `cacheTag(CACHE_TAG.playerStats(loginId))`), 태그 사전 `CACHE_TAG`(`:8-21`), 무효화 `revalidateOnScoreSubmit`(`:75-91`, 새 점수 제출 때 `player-stats`·`player-recent` 등 무효화), 코스 제출 시 `revalidateOnCourseSubmit`(`:220-224`)
5. 서비스: `web/src/server/service/domain/fe/fe.service.ts:94-138`(`createFeService`), `playerStats` 는 `:114-122`, DB 계약 `FeServiceDb`(`:41-50`)
6. DB: `web/src/server/compose/fe.ts:194-201`(`countLampsByUser`, Drizzle), `countPlayerScores`(`:302-305`)
7. DTO: `web/src/server/dto/fe.ts`(`playerRecentQuerySchema` `:34-36`, 쿼리는 `z.coerce`)
8. 클라이언트: `web/src/shared/constants/query-key.ts:15-22`(`QUERY_KEY.PLAYER.*`), `web/src/entities/player.type.ts`, `web/src/entities/player.query.ts:18-28`(`playerStatsQueryOptions` + `usePlayerStats`)
9. 서버 프리페치: `web/src/widgets/player/player-detail-content.tsx:21-28`. `queryFn` 을 `read*` 로 바꿔 끼워야 하며 `tests/lib/ssr-no-self-fetch.test.ts` 가 이를 검사합니다.
10. 화면: `web/src/widgets/player/player-stats-panel.tsx`

### 4-2. 본인 전용 읽기 (`/api/fe/me/ai/weekly` 가 가장 가까운 선례) [확인]

- 라우트 파일: `web/src/app/api/fe/me/ai/weekly/route.ts`
- 핸들러: `web/src/server/route/ai.ts:333-340`(`withLimited`: 세션 사용자 + `enforceUserRateLimit`, `RATE_LIMIT.TABLE_RECORDS` = 사용자당 30/분), `:418-421`(`weeklyView`)
- 응답은 `private, no-store`(`aiPrivateJson`)이고 서버 데이터 캐시를 쓰지 않습니다.
- 서비스: `weekly-summary.service.ts`. DB 어댑터는 `web/src/server/compose/ai-score-features.ts:111-169`(`createAiWeeklySummaryDb.listScoresBetween`). 서비스는 순수 함수(`summarizeWeek`)에 가깝고 `ServiceDb` 를 인라인 객체로 대체해 시험합니다.
- 클라이언트: `web/src/entities/ai.query.ts:242-246`(`aiWeeklyQueryOptions`), 경로 상수 `:203-209`(`AI_FEATURE_PATH`), 키 `QUERY_KEY.AI.DIGEST(feature, params)`(`query-key.ts:86-88`)
- 더 단순한 본인 전용 예: `web/src/server/route/trace.ts:103`(`mine: withUser(...)`, `privateJson`)

### 4-3. 메시지 카탈로그 [확인]

- 위치: `web/messages/{ko,en,ja}/{common,ai,settings,browse,pages,server}.json`(네임스페이스 여섯 개 고정, `docs/web/architecture.md:806-812` 16-1).
- 플레이어 화면 문구는 `browse.json` 의 `player.{detail,recent,stats}`, AI 주간 요약 문구는 `ai.json` 의 `weekly`(`:177`)입니다.
- 한국어가 원본이고 en·ja 는 키가 빠지면 `tsc` 가 막으며, `tests/lib/i18n-messages.test.ts` 가 세 언어의 키 집합과 ICU 자리표 일치를 검사합니다.
- 서버 컴포넌트는 `getTranslations`, 클라이언트는 `useTranslations`(예: `player-stats-panel.tsx` 의 `useTranslations('browse.player.stats')`)를 씁니다.

### 4-4. 시험 위치 [확인]

- 서비스 순수 시험: `tests/service/fe-player-history.test.ts`(`createFeService` 에 인라인 `db`)
- 공개 읽기·쿼리 스키마: `tests/lib/public-reads.test.ts`
- 엔티티 URL: `tests/components/entity-query-wiring.test.ts`
- 컴포넌트: `tests/components/player-recent-panel.test.tsx`, `tests/components/browse-locales.test.tsx`, `tests/components/ai-score-features.test.tsx:160`(주간 패널)
- 라우트: `tests/route/ai-score-features.test.ts`
- DB 통합: `tests/db/*`(임시 MySQL 필요, `RBMS_DB_E2E=1` 없으면 건너뜀, 예 `tests/db/ai-score-features.test.ts:26,205`)

## 5. 차트 라이브러리와 기존 그리기 방식

- **차트 라이브러리 없음** [확인]: `web/package.json` 의존성에 recharts·d3·chart.js 등이 없습니다. 설계 문서도 "라이브러리 미도입, 인라인 SVG"(`docs/web/components.md:42`)입니다.
- 기존 방식:
  - 폭 비율 CSS 막대(`lamp-stack-bar.tsx:16-34`, `bar-count-list.tsx:20-27`, `table-progress-panel.tsx` 의 레벨 막대)
  - 높이 비율 막대 + SVG 선(`ai-segment-chart.tsx:35-62`, `viewBox 0 0 100 100`, `preserveAspectRatio='none'`, `vectorEffect='non-scaling-stroke'`)
  - 픽셀 상수 기반 SVG(`replay-waterfall.tsx:25-`)
- 공용 상수: `PERCENT_SCALE`(`web/src/shared/constants/ui.ts:11`), 색은 `bg-primary`/`bg-muted-foreground/40` 같은 토큰이나 `var(--color-*)` 입니다.
- 새 의존성 없이 가능합니다. 일별 막대 N개 + 표(`web/src/shared/ui/table.tsx` 활용)는 `ai-segment-chart.tsx` 방식(높이 비율 막대)을 따르면 됩니다.
- `docs/web/components.md` 의 막대 목록에 새 컴포넌트를 추가해야 합니다(`:65`, `:234` 형식).

## 6. 구현 시 수정할 파일 목록

### 6-A. 권장안: 플레이어 페이지, 추정 시간, 공개 읽기, KST 일 경계

신규 (경로는 모두 `web/` 기준):

| 파일 | 요지 |
|---|---|
| `src/app/api/fe/players/[playerId]/play-days/route.ts` | `GET = createFePlayerPlayDaysRoute()` 한 줄 어댑터 |
| `src/features/stats/play-day-chart.tsx` | 일별 시간 막대(순수 props, 비즈니스 로직 없음, `ai-segment-chart.tsx` 방식) |
| `src/features/stats/play-day-table.tsx` | 일·판 수·추정 시간 표(`shared/ui/table`) |
| `src/widgets/player/player-play-days-panel.tsx` | 쿼리 호출 + 로딩·오류·빈 상태(`DashboardPanelLoading/Error/Empty` 패턴, `player-recent-panel.tsx` 참고) |
| `tests/service/…play-days….test.ts` | 세션·경계·상한 순수 시험 |
| `tests/components/…play-day….test.tsx` | 차트·표 렌더, 모드 전환 시험 |

수정:

| 파일 | 요지 |
|---|---|
| `src/server/dto/fe.ts` | `playerPlayDaysQuerySchema`(기간 일수 등, `z.coerce`) 와 타입 |
| `src/server/service/domain/fe/fe.service.ts` | `FeServiceDb.listPlayerPlayTimes` 계약 + 순수 집계 함수 + `playerPlayDays` 메서드 |
| `src/server/compose/fe.ts` | Drizzle 구현(`score.playedAt` 만 선택, `userId` + 기간 조건, 오름차순. 코스 포함 시 `courseScore` 도) |
| `src/server/service/read-cache.ts` | `CACHE_TAG.playerPlayDays`, `getPlayerPlayDaysCached`, `revalidateOnScoreSubmit`(+ 코스 포함 시 `revalidateOnCourseSubmit`)에 무효화 추가 |
| `src/server/service/public-reads.ts` | `readPlayerPlayDays` |
| `src/server/route/fe.ts` | `createFePlayerPlayDaysRoute`(`withPublicRead`) |
| `src/shared/constants/query-key.ts` | `QUERY_KEY.PLAYER.PLAY_DAYS(playerId, params)` |
| `src/entities/player.type.ts`, `src/entities/player.query.ts` | 타입, `playerPlayDaysQueryOptions`, `usePlayerPlayDays` |
| `src/widgets/player/player-stats-panel.tsx` (또는 `player-detail-panel.tsx:62`) | 모드 전환(`ToggleGroup`, 램프 분포 ↔ 일별 시간). 상태는 `useState`, URL 동기화는 선택 |
| `src/widgets/player/player-detail-content.tsx` | (선택) 서버 프리페치 추가. 추가하면 `queryFn` 을 `readPlayerPlayDays` 로 바꿔 끼워야 `ssr-no-self-fetch` 시험을 통과 |
| `messages/{ko,en,ja}/browse.json` | `player.playDays.*`(제목, 모드 라벨, 표 머리, "추정" 안내, 빈 상태, 오류) |
| `tests/components/entity-query-wiring.test.ts`, `tests/lib/public-reads.test.ts` | URL·스키마 검증 항목 추가 |
| `docs/web/architecture.md`(§1-3 표, §5-2·5-3 태그), `docs/web/components.md`, `docs/PROCESS.md` | 라우트·태그·컴포넌트 기록 |

### 6-B. 대안: AI 허브 주간 요약(본인 전용)

수정: `src/widgets/ai/ai-weekly-panel.tsx`(모드 토글: 요약 표 ↔ 일별 시간), `src/server/dto/ai.ts`(`aiWeekQuerySchema` 재사용 또는 확장), `src/server/service/domain/ai/weekly-summary.service.ts`(일별 항목 추가) 또는 새 서비스, `src/server/compose/ai-score-features.ts`(`listScoresBetween` 확장 또는 신규), `src/entities/ai.type.ts`(`AiWeeklyView` 확장), `messages/*/ai.json`, 관련 시험(`tests/route/ai-score-features.test.ts`, `tests/components/ai-score-features.test.tsx`, `tests/db/ai-score-features.test.ts`).
주의: `weekly` GET 은 AI 해석 캐시 키(`weeklySubjectKey`)와 얽혀 있으니 digest 가 바뀌지 않도록 일별 항목은 digest 밖에 둡니다. [추측]

### 6-C. 선택: 정확한 시간을 새 판부터 얻기 (클라이언트 변경)

- `crates/rbms-ir/src/dto/score.rs`: 새 `*_EXTRA_KEY` 와 `state_*` 및 getter(선례 `:96-160`).
- `apps/rbms-player/src/app_result.rs`: `state_mean_offset`(`:376`) 옆에 호출 추가. 값은 "실제로 친 시간"이어야 하는데 현재 로컬 `playtime_ms` 는 곡 끝 줄 시각이어서 정의를 새로 정해야 합니다.
- `apps/rbms-player/src/record_sync.rs:980`(재구성 제출 경로) 처리.
- 계약 파일 `contract/client-requests/*.json` 재생성: `RBMS_UPDATE_CONTRACT=1 cargo test -p rbms-player --lib contract_requests`(`docs/development.md:73`).
- 서버: `extra` 읽기 스키마 추가(`aiScoreTimingExtraSchema` 선례, `web/src/server/dto/ai.ts:599-602`). 소급하려면 중복 제출 분기(`score.service.ts:354-387`)가 기존 행을 갱신하도록 바꿔야 합니다. 이 경우 별도 결정이 필요합니다.

## 7. 사용자 결정이 필요한 지점

### 결정 필요

1. **어느 화면의 "분석 그래프"인가**
   - 선택지: A. 플레이어 페이지 통계 패널(공개, 램프 분포와 모드 전환), B. AI 허브 주간 요약 탭(본인 전용), C. 둘 다.
   - 추천: **A**. 모드 전환 대상이 될 "그래프"가 있는 화면이고 "하루하루"를 길게 볼 수 있으며 공개 읽기 패턴이 이미 있습니다.
   - 확신도는 낮습니다. 사용자가 AI 허브를 말한 것이라면 B 입니다.
2. **"몇 시간"의 정의**
   - 선택지: A. 판 간격 기반 추정(서버만, 전 과거 판 소급, 정확도 낮음, 화면에 "추정" 표기), B. 곡 길이 합(길이를 아는 판만, 본인 trace/패턴 의존), C. 클라이언트가 실제 시간을 보냄(새 판부터, 소급 불가, Rust 변경 필요).
   - 추천: **A 먼저**, 값이 부정확하다고 느껴지면 C 를 추가합니다.
   - A 를 택하면 상수 값이 필요합니다: 세션 경계 간격, 첫 판 길이 가정, 하루 상한. 코드에 임의로 정하지 않으므로 값 승인이 필요합니다.
3. **날짜 경계**
   - 선택지: A. KST(`DISPLAY_TIME_ZONE`, 화면 시각과 같음), B. UTC(주간 요약과 같음).
   - 추천: **A**. 사용자가 보는 시각과 일치합니다. 주간 요약만 UTC 로 남는 불일치는 그대로 두되 화면에 시간대를 표기합니다.
4. **코스 판을 시간 선에 포함할지**
   - 선택지: A. 포함(`course_score.playedAt` 합산), B. 제외.
   - 추천: **A**. 제외하면 코스 도중이 긴 공백으로 읽혀 세션이 끊깁니다. 다만 코스 판의 곡별 행이 따로 올라오는지 미확인이라 구현 전 확인이 필요합니다. (구현 때 확인: 따로 올라오므로 `course_score` 는 읽지 않는다.)
5. **어떤 판을 포함할지**(ranked 아님, SCRATCH AUTO, 어시스트 판)
   - 선택지: A. 서버에 저장된 모든 판, B. ranked 만.
   - 추천: **A**. 시간은 시간입니다. 최근 활동처럼 ranked 만 보이게 하면 시간이 줄어듭니다.
6. **공개 범위**
   - 선택지: A. 플레이어 페이지 공개(누구나 봄), B. 본인에게만 노출.
   - 추천: **A**(기록 목록이 이미 공개). 비공개를 원하면 B 로 하고 `isOwnRecord`(`player-recent-panel.tsx:36`) 패턴을 씁니다.
7. **기간과 단위**
   - 선택지: A. 최근 30일 고정, B. 30·90일 선택, C. 월 이동.
   - 추천: **A**(최소 구현).

## 8. 최소 검증 명령 (`web/package.json` 스크립트 확인 후)

기본 (모든 변경 공통):

```
cd /Users/hyunseokbyun/development/R-BMS/web
bun run typecheck
bun run lint
bun test tests/service/<신규 시험>.test.ts tests/lib/public-reads.test.ts tests/components/entity-query-wiring.test.ts tests/lib/i18n-messages.test.ts tests/lib/ssr-no-self-fetch.test.ts
bun run format:check
```

- 마무리 게이트: `bun run verify`(= `typecheck` → `lint` → `test`, `package.json` 의 `verify` 스크립트).
- `'use cache'` 와 라우트를 추가하므로 `bun run build` 가 필요합니다(프리렌더 중 DB 접속 금지 규칙은 `docs/web/architecture.md:437` 5-4 의 "빌드 시 DB 미접속 보장"으로 검증, `docs/web/architecture.md:565` 8장 최소 기계 검증 안내도 같음). 환경 변수는 필요하지 않은 것으로 문서에 적혀 있습니다.
- DB 질의 시험(선택): `RBMS_DB_E2E=1 DATABASE_URL=<임시 MySQL> bun test tests/db/<신규>.test.ts`(마이그레이션을 마친 임시 MySQL 필요. 이번 변경은 마이그레이션 없음).
- 클라이언트 변경(6-C)을 할 때만:
  - `cargo test -p rbms-ir`
  - `RBMS_UPDATE_CONTRACT=1 cargo test -p rbms-player --lib contract_requests` 후 같은 명령을 `RBMS_UPDATE_CONTRACT` 없이 한 번 더 실행
  - 웹: `bun test tests/dto/client-requests.test.ts`
  - 전체 CI 동등 명령은 `cargo test --workspace --no-fail-fast --locked`(`docs/development.md:16`).

## 9. 미확인 항목과 부수 발견

미확인:

- 운영 DB 의 `chart_pattern`·`play_trace` 적재율(DB 접속 금지).
- 코스 도중의 곡별 점수 행이 따로 제출되는지. (구현 때 확인됨: 따로 제출된다. 위 "구현 결과".)
- 0노트 Esc 판이 서버로 가는지(문서 결정만 확인).
- `tests/route/trace.test.ts` 가 표 접근 고정을 실제로 어떻게 검사하는지(문서 기술만 확인).
- 간격 추정이 실제 플레이 시간과 얼마나 어긋나는지(실데이터 없음).

부수 발견 (요청 3번 "최근 활동에 어시스트 활동" 관련, 이번 조사 범위 밖):

- 홈 최근 활동은 `ranked = true AND scratch_auto = false` 인 판만 조회합니다(`web/src/server/compose/fe.ts:148`). 화면 안내도 같은 말입니다(`web/messages/ko/browse.json` 의 `home.recent.note`).
- 어시스트 옵션 판은 순위 제외 플래그(`rankBlockingFlags`, `web/src/server/service/domain/score/ranked-policy.ts:38`)로 `ranked = false` 가 되어 목록에서 빠질 수 있고, SCRATCH AUTO 판은 `scratch_auto = false` 조건 때문에 빠집니다. [확인: 조건 위치, 추측: 사용자가 말한 "어시스트 활동"이 이 두 경우인지]
- 활동 항목 자체는 이미 `assist_grade` 를 싣고 있습니다(`fe.service.ts:52-64`, `web/src/entities/stats.type.ts:20-32`).

보고서를 `docs/reference` 에 저장할 때의 참고 경로: `/Users/hyunseokbyun/development/R-BMS/docs/reference/`(기존 `2026-10-29-feature-upgrade/` 폴더가 같은 용도의 선례입니다).