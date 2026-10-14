# ADR: 사람이 친 판은 모두 저장·전송한다

> 최종 갱신 2026-10-14 · 대응 커밋 7095ef7
> 상태: 유효 · 결정일 2026-10-14 · 결정자 사용자(원칙) + 메인(넓힌 판정 판의 최고 기록·순위 제외, 사용자 미확인)
> 확장: Esc 중도 포기 판도 포함 → [2026-10-14-adr-abandoned-run-saved.md](2026-10-14-adr-abandoned-run-saved.md). 넓힌 판정 판의 최고 기록·순위 제외는 2026-10-14 사용자 확인(추천안 A).
> 대체하는 결정: `2026-09-09-enhancement-decisions.md` 결정 12 의 "판정폭 rate>100 또는 LN 마진 rate>100 → IR 제출·리플레이 저장 차단"

## 배경

결정 12 는 원본 정책을 이식해 넓힌 판정(판정폭 또는 LN 마진 rate>100) 판의 IR 제출과 리플레이 저장을 막았고, SCRATCH AUTO 판의 최고 기록 갱신도 막았다. 사용자는 SCRATCH AUTO 규칙을 다시 정리하면서 "서버로 저장되지 않는 기록따윈 없어 무조건 저장될거야. 또한 리플레이도 잘 저장되어야하고"라고 했다.

## 결정

1. 사람이 친 판(오토플레이, 리플레이 보기, 연습이 아닌 판)은 어시스트 여부와 상관없이 전부 로컬 이력에 남기고, 리플레이를 저장하고, 점수 서버로 보낸다. 전송을 막는 이유는 오토플레이·리플레이 재생·연습 셋뿐이다(`apps/rbms-player/src/lib.rs` 의 `ir_submission_block_reason`, 원본 `MusicResult.java:82` 와 같은 범위).
2. 어시스트는 전송 본문에 실어 보내고 판단은 서버가 한다(`assist_flags`, `options.scratch_auto`).
3. 넓힌 판정 판은 저장·전송하지만 로컬 최고 기록(EX·판정 수·콤보·BP)은 바꾸지 않는다(`updates_score`, `crates/rbms-store/src/score.rs` 의 `sets_best` 가 assist 2 이상에 false). 서버도 그 판을 `ranked=false` 로 저장하고 `chart_best` 를 바꾸지 않는다(`web/src/server/service/domain/score/ranked-policy.ts` 의 `JUDGE_WIDTH`·`ASSIST` 는 순위를 막는 표지, `score.service.ts` 의 `isNewBest` 는 `ranked` 일 때만).
4. SCRATCH AUTO 판은 일반 판처럼 최고 기록을 갱신한다([2026-10-14-adr-scratch-auto-rule.md](2026-10-14-adr-scratch-auto-rule.md)).
5. 실제로 보내지 않은 판은 보낸 것으로 적지 않는다(커밋 `8b11a17`).
6. 리플레이 저장은 설정 AUTO REPLAY(`config.play.auto_replay`, 기본 켬)를 따른다. 끄면 그 판의 리플레이 파일은 만들지 않는다(`apps/rbms-player/src/app_result.rs` 98행).

## 이유

- 사용자 원문: 저장되지 않는 기록은 없어야 하고 리플레이도 저장돼야 한다.
- 넓힌 판정 판은 같은 차트를 더 쉬운 조건에서 친 것이므로 최고 기록과 순위에 섞으면 비교가 깨진다. 원본도 그 판의 `score` 플래그를 내려 EX·BP·콤보 갱신을 막는다(`ScoreData.java:548,566,572,578`). 이 부분은 메인이 원본을 따라 정했고 사용자는 언급하지 않았다(미해결 질문).

## 기각된 대안

| 대안 | 기각 이유 |
| --- | --- |
| 결정 12 그대로(넓힌 판정 판의 전송·리플레이 차단) | 사용자 "서버로 저장되지 않는 기록따윈 없어" |
| 넓힌 판정 판도 최고 기록·순위에 넣기 | 쉬운 조건의 판이 일반 판과 경쟁한다. 원본과도 다르다 |

## 영향과 관련 파일

- 클라이언트: `apps/rbms-player/src/lib.rs`(`ir_submission_block_reason`, `updates_score`, `assisted_lamp`), `apps/rbms-player/src/app_result.rs`, `crates/rbms-store/src/score.rs`, `crates/rbms-ir/src/mapping.rs`.
- 서버: `web/src/server/service/domain/score/ranked-policy.ts`, `web/src/server/service/domain/score/score.service.ts`.
- 커밋: `8b11a17`. 코스는 [2026-10-14-adr-assisted-course-submission.md](2026-10-14-adr-assisted-course-submission.md).
- KNOWN ISSUE: `apps/rbms-player/src/app_result.rs` 245~246행이 `rp.save(...)`(실패해도 `eprintln!` 만 하는 `Replay::save`, `crates/rbms-store/src/replay.rs` 168행) 뒤에 저장 성공 여부와 상관없이 `replay_file = Some(name)` 을 둔다. 디스크 쓰기가 실패해도 로컬 기록이 없는 리플레이 파일을 가리키고 결과 화면이 리플레이 저장됨(`replay_saved`)으로 보인다. 고치는 방법: `try_save` 의 결과가 `Ok` 일 때만 `replay_file` 을 채우고, 실패하면 `notify(Level::Error, ..)` 로 알린다(업로드할 리플레이는 메모리의 것을 그대로 쓸 수 있다).

## 날짜와 사용자 원문

2026-10-14. "서버로 저장되지 않는 기록따윈 없어 무조건 저장될거야. 또한 리플레이도 잘 저장되어야하고" (SCRATCH AUTO 재정리 원문의 일부, 전문은 [2026-10-14-adr-scratch-auto-rule.md](2026-10-14-adr-scratch-auto-rule.md)).
