# ADR: 플레이어 페이지의 일별 플레이 시간 — 판 간격으로 추정한 최근 30일

> 최종 갱신 2026-11-02 · 대응 커밋 `28085a7`·`8af1a39`·`f737c4d`
> 상태: 유효 · 결정일 2026-11-02 · 결정자 사용자(범위와 규칙) · 커밋 `8af1a39`

## 배경

- 사용자 지시(2026-11-02 원문): "웹에서, 또 다른 분석 그래프로 하루하루 몇시간했는지 볼 수 있는 그래프 및 표도 보이게 해줘 (모드로 변경해서 보이게하면 되겠지)".
- 서버에는 판 길이가 없다. 점수 제출 본문에는 판이 끝난 시각 `played_at` 한 개뿐이다(길이·시작 시각·곡 길이 없음). 곡 길이를 아는 경로(패턴 `length_ms`, trace `play_time_ms`, 리플레이 `duration_us`)는 일부 판만 덮는다. 조사: [reference/2026-11-02-web-three-requests/daily-playtime.md](../reference/2026-11-02-web-three-requests/daily-playtime.md).

## 결정

1. 화면: 플레이어 페이지 통계 패널(`PlayerStatsPanel`)의 모드 전환(램프 분포 / 일별 플레이 시간). 일별 모드는 높이 비율 막대 30개(`PlayDayChart`)와 날짜별 표(`PlayDayTable`)를 보이고 "판과 판 사이 간격으로 추정한 시간입니다 (한국 시간 기준)." 를 적는다. 모드 전환은 새 라우트가 아니라 같은 패널 안이며, 일별 모드를 고른 뒤에야 조회한다.
2. 시간은 **판 간격으로 추정**한다: 판 끝 시각 오름차순으로, 앞 판 끝과의 간격이 10분(`PLAY_SESSION_GAP_MS`) 이하면 그 간격을, 아니면 그리고 첫 판이면 2분(`PLAY_DEFAULT_MS`)을 그 판의 시간으로 삼아 **판 끝 시각이 속한 KST 날**에 더한다.
3. 날짜 경계는 한국 시간 자정(화면 시각 `DISPLAY_TIME_ZONE` 과 같음. 주간 요약의 UTC 주와는 다르다), 창은 최근 30일(`PLAY_DAYS_WINDOW`)이다. 창 바로 앞의 한 판은 창 첫 판의 간격 계산에만 쓴다.
4. 대상은 그 플레이어의 `score` 모든 판(`ranked` 여부·SCRATCH AUTO 무관, "시간은 시간이다"). 코스 판은 `course_score` 를 읽지 않는다: 코스의 스테이지는 곡별 `score` 행으로 따로 올라오므로(`apps/rbms-player/src/app_result.rs:158-196`, `web/src/server/route/course.ts:114-125`) 이미 들어 있고, 읽으면 두 번 센다.
5. API: `GET /api/fe/players/{playerId}/play-days`(무인증 공개 읽기, 쿼리 없음) → `{ days:[{ date, plays, play_ms }] 30개, total_plays, total_ms }`. 서버 캐시는 `'use cache'` + `cacheLife('minutes')`, 태그 `player-play-days:{loginId}`, 캐시 키에 KST 오늘 0시(`todayStart`)를 넣어 자정에 창이 넘어간다. `POST /api/scores` 의 `revalidateOnScoreSubmit` 이 무효화하고 코스 제출은 무효화하지 않는다(위 4).
6. 차트 라이브러리를 들이지 않는다(기존 그래프와 같은 CSS 막대). 새 컴포넌트: `features/stats/play-day-chart.tsx`, `features/stats/play-day-table.tsx`, `widgets/player/player-play-days.tsx`. 문구는 `web/messages/*/browse.json` 의 `browse.player.playDays`.

## 기각된 대안 (보류)

- **클라이언트가 판 길이를 실측해 `extra` 로 보내기**: 정확하지만 계약 파일과 Rust 시험 갱신이 필요하고 이미 올라간 판은 소급되지 않는다(중복 제출은 기존 행을 갱신하지 않는다). 사용자가 추정을 골랐으므로 이번에는 하지 않고 보류한다. 추정 결과가 실제와 크게 어긋나면 그때 다시 연다.
- 곡 길이(패턴·trace·리플레이)를 섞어 쓰기: 일부 판만 덮어 값이 판마다 다른 방식이 된다.
- AI 허브 주간 요약 탭에 넣기: 본인 전용이고 UTC 주 기준이라 공개 통계와 맞지 않는다.

## 남는 것

- 추정이라 실제 플레이 시간과 어긋난다(10분 이하 간격은 곡 사이 휴식까지 시간으로 센다. 첫 판과 휴식 뒤 판은 2분 고정). 두 상수는 체감 검증 전의 값이다([테스트 부채](../quality-assurance/2026-10-14-test-debt.md) 3.144).
- 창 경계(자정 전후, 창 바로 앞 한 판)를 실제 DB 쿼리로 확인하는 통합 시험은 없다(테스트 부채 3.142).
- 일본어 문구는 원어민 감수 전이다(테스트 부채 3.143).

## 날짜와 사용자 원문

2026-11-02. "웹에서, 또 다른 분석 그래프로 하루하루 몇시간했는지 볼 수 있는 그래프 및 표도 보이게 해줘 (모드로 변경해서 보이게하면 되겠지)". 결정 선택(플레이어 페이지, 판 간격 추정 10분/2분, KST 자정, 최근 30일, 모든 판) 은 2026-11-02 사용자 답.
