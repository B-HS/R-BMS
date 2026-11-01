# ADR: 홈 최근 활동에 SCRATCH AUTO 판도 A-SCR 표지와 함께 표시

> 최종 갱신 2026-11-02 · 대응 커밋 `28085a7`·`8af1a39`·`f737c4d`
> 상태: 유효 · 결정일 2026-11-02 · 결정자 사용자 · 커밋 `f737c4d`. [2026-10-19 웹 ADR](2026-10-19-adr-web-clear-and-play-history.md) 2절의 "홈의 최근 활동은 SCRATCH AUTO 판을 계속 뺀다" 를 뒤집는다.

## 배경

- 사용자 지시(2026-11-02 원문): "최근활동에도 어시스트 관련 활동이 보이게해놔".
- 홈 최근 활동(`listRecentActivity`)은 `ranked = true AND scratch_auto = false` 인 판만 골랐다. 그래서 SCRATCH AUTO 판(서버는 `ranked=true` 로 저장)은 늘 빠졌고, 어시스트 게이지 등급 1~4(ASSIST EASY~EXHARD)는 SCRATCH AUTO 클리어에서만 나오므로 홈에서는 한 번도 보이지 않았다. 조사: [reference/2026-11-02-web-three-requests/recent-activity-assist.md](../reference/2026-11-02-web-three-requests/recent-activity-assist.md).

## 결정

1. 사용자가 "SCRATCH AUTO 판 포함 (추천)" 을 골랐다(조사의 안 A). `listRecentActivity` 가 `ranked=true` 인 판을 SCRATCH AUTO 판까지 모두 싣고, 홈 패널(`RecentActivityPanel`)이 `ScoreRow` 에 `isScratchAuto={row.scratch_auto}` 를 넘겨 A-SCR 표지를 붙인다. `ActivityRow` 모양과 캐시 태그(`activity-recent`)·`QUERY_KEY` 는 바뀌지 않는다.
2. `ranked=false` 판(넓힌 판정, 자기 TOTAL, 어시스트 옵션, 오토플레이, 게스트, 신뢰 안 하는 빌드)은 계속 숨긴다. 홈은 로그인 없이 보는 공개 피드이고 "순위에 반영되는 제출만" 이라는 약속을 유지한다. 코스 판과 A-단위 획득은 이번 범위 밖이다(다른 표에 있어 별도 쿼리가 필요).
3. 홈 안내 문구(`browse.home.recent.note`)를 세 언어 모두 "순위에 반영되는 제출만 표시됩니다. A-SCR 은 SCRATCH AUTO 로 친 기록입니다." 취지로 바꿨다.

## 남는 것

- `ranked=false` 어시스트 판(안 B)은 보이지 않는다. 필요하면 활동 응답에 `ranked`·`flags` 를 싣고 "순위 제외" 표지를 다는 별도 작업이다.
- 바뀐 DB 시험 기대값(`web/tests/db/scratch-auto-ranking.test.ts`: 홈 최근 활동에 SCRATCH AUTO 판이 `scratch_auto: true` 로 포함)은 임시 MySQL 이 없어 실행하지 못했다(테스트 부채 3.141).
- SCRATCH AUTO 판이 많이 올라오는 날에는 홈 20줄에서 일반 판이 밀려날 수 있다(필터가 없다).
- 일본어 문구는 원어민 감수 전이다(테스트 부채 3.143).

## 날짜와 사용자 원문

2026-11-02. "최근활동에도 어시스트 관련 활동이 보이게해놔", 범위 선택 "SCRATCH AUTO 판 포함 (추천)".
