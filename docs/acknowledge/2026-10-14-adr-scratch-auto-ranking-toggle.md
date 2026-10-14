# ADR: SCRATCH AUTO 판도 순위에 올리고 목록에서는 토글로 보인다

> 최종 갱신 2026-10-14 · 대응 커밋 7095ef7
> 상태: 유효 · 결정일 2026-10-14 · 결정자 사용자(선택지 B 의 변형)

## 배경

SCRATCH AUTO 판을 서버로 똑같이 보내기로 한 뒤([2026-10-14-adr-scratch-auto-rule.md](2026-10-14-adr-scratch-auto-rule.md)), 서버는 여전히 `SCRATCH_AUTO` 표지를 순위를 막는 표지로 다뤘다. 순위에서 뺄지(A) 올릴지(B) 사용자에게 물었다.

## 결정

1. 서버의 차트·코스 최고 기록을 SCRATCH AUTO 여부로 나눈다. 한 사람이 일반 판 최고와 SCRATCH AUTO 판 최고를 따로 가진다. 마이그레이션 `web/drizzle/0002_scratch_auto_bests.sql`: `chart_best`·`course_best`·`course_score` 에 `scratch_auto boolean DEFAULT false NOT NULL`, 두 best 의 기본 키를 `(…, user_id, scratch_auto)` 로, `chart_best_rank_idx` 를 `(chart_sha256, scratch_auto, clear, ex_score)` 로.
2. `SCRATCH_AUTO` 는 순위를 막지 않는 표지다(`ranked-policy.ts` 의 `RANK_NEUTRAL_FLAGS = [SCRATCH_AUTO, UNKNOWN_BUILD]`, 클라이언트 `ScoreFlag::blocks_ranking`). 제출 응답 메시지는 `saved (scratch auto)`.
3. 순위 목록과 리더보드는 기본으로 일반 판만 보인다. 질의 인자 `include_scratch_auto` 를 주면 둘 중 높은 기록으로 SCRATCH AUTO 판을 포함한다. SCRATCH AUTO 판의 순위 숫자는 포함 목록에서의 자리다.
4. 웹: 차트 순위, 코스 순위, 리더보드에 "SCRATCH AUTO 포함" 스위치를 두고 상태를 주소에 남긴다. SCRATCH AUTO 기록에는 `A-SCR` 표지(목록, 내 기록, 리플레이). 문구는 `web/src/shared/constants/scratch-auto.ts`.
5. 클라이언트 결과 화면은 `IR: RANK #n (SCRATCH AUTO)`(`apps/rbms-player/src/ir_outcome.rs` 의 `SCRATCH_AUTO_RANK_MARK`). 그 판의 리플레이도 자동 업로드 대상이다.
6. 넓힌 판정 판은 지금처럼 순위에서 빠진다(사용자 미언급, 메인이 유지).

## 이유

사용자 원문: B 로 하되 "기본적으로 리스트에서는 안보이고 어딘가 토글을 누르면 보일수있도록". SCRATCH AUTO 판의 EX 는 턴테이블을 뺀 노트로만 계산되어 일반 판과 크기가 다르다. 같은 칸에 두면 서로를 덮어쓰므로 최고 기록을 나누고, 기본 목록은 일반 판끼리 비교하게 한다.

## 기각된 대안

| 대안 | 기각 이유 |
| --- | --- |
| A. SCRATCH AUTO 판은 순위에서 뺀다(unranked) | 사용자 B 선택 |
| B 원안: 일반 판과 한 목록에 섞어 보인다 | 사용자가 기본 숨김 + 토글로 변형 |
| 최고 기록을 나누지 않고 한 칸에 둔다 | 턴테이블을 뺀 EX 와 일반 EX 가 서로를 덮는다 |

## 영향과 관련 파일

- 서버: `web/drizzle/0002_scratch_auto_bests.sql`, `web/drizzle/meta/0002_snapshot.json`, `web/src/server/db/schema.ts`, `web/src/server/service/domain/score/ranked-policy.ts`, `score.service.ts`, `course.service.ts`, `web/src/server/dto/score.ts`, `web/src/server/dto/fe.ts`, `web/src/server/route/chart.ts`, `course.ts`, `fe.ts`, `web/src/server/compose/fe.ts`.
- 웹 화면: `web/src/widgets/chart/chart-leaderboard-panel.tsx`, `web/src/widgets/course/course-detail-panel.tsx`, `web/src/widgets/leaderboard/player-leaderboard-panel.tsx` 등.
- 클라이언트: `crates/rbms-ir/src/dto/score.rs`, `apps/rbms-player/src/ir_outcome.rs`.
- 운영 DB 에 0002 적용됨(2026-10-14, web-db 워크플로 run 38548459519).
- 커밋: `cf40490`(클라이언트), `0b04629`(서버·웹·문서), 결정 기록 `a524787`. 웨이브 20(`wf_8b68e818-5c2`).
- 남은 일(미착수): 게임 안 IR 순위 패널의 SCRATCH AUTO 포함 토글.

## 날짜와 사용자 원문

2026-10-14. "B 로가긴하는데 B로 구현하고 기본적적으로 리스트에서는 안보이고 어딘가 토글을 누르면 보일수있도록하는게 좋지않을까"
