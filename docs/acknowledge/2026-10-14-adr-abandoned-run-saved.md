# ADR: Esc 로 중도 포기한 판도 저장·전송한다

> 상태: 일부 대체됨(결정 2·3) · 결정일 2026-10-14 · 결정자 사용자(추천안 A) + 메인(판정 없는 판 제외, 코스 제외) · 커밋 `5f4a9a2`

## 배경

`leave_run`(Esc) 은 결과 화면·기록·전송 없이 나갔다. [모든 판 저장·전송 ADR](2026-10-14-adr-save-and-send-every-played-run.md) 의 "사람이 친 판은 전부"에 이 판이 들어가는지가 미결이었다(HANDOFF 6절 3).

## 결정

1. 사람이 친 판을 Esc 로 나가면 결과 화면 없이 바로 이력·리플레이에 남기고 서버로 보낸다. 램프 FAILED, 남은 노트는 늦은 POOR(놓침)로 센다(`PlaySummary::left_unfinished`).
2. 판정된 노트가 하나도 없는 판은 남기지 않는다(메인 판단: 시작하자마자 나간 판은 친 판이 아니다). 대체됨: [2026-10-15-adr-escape-keeps-every-run.md](2026-10-15-adr-escape-keeps-every-run.md)
3. 오토플레이·리플레이 보기·연습은 지금처럼 남기지 않는다. 코스 도중 Esc 는 지금처럼 기록 없이 코스를 끝낸다(메인 판단, 이번 범위 밖). 대체됨(코스 부분): [2026-10-15-adr-escape-keeps-every-run.md](2026-10-15-adr-escape-keeps-every-run.md)
4. 노트가 다 처리된 뒤나 닫히는 장면에서의 Esc 는 결과 화면으로 간다.

## 이유

- 사용자 결정(추천안 A). 원본도 포기를 FAILED 로 저장한다(`BMSPlayer.java` `stopPlay`). 원본은 남은 노트를 BP 에만 더하지만, 여기서는 판정 수에도 넣어 완주 판처럼 판정 합이 노트 수와 같게 한다.
- Esc 는 바로 나가는 키라 결과 화면을 강제하지 않는다.

## 기각된 대안

| 대안 | 기각 이유 |
| --- | --- |
| 지금처럼 남기지 않는다(B) | 사용자가 A 를 택함 |
| 결과 화면을 거친다 | 빨리 나가는 키의 체감이 바뀐다 |

## 영향과 관련 파일

`apps/rbms-player/src/stage/play/mod.rs`(`leave_run`, `keeps_a_left_run`), `apps/rbms-player/src/app_result.rs`(`record_left_run`), `crates/rbms-play/src/session.rs`, `docs/play-options.md` 10.2, 계약 파일 `contract/client-requests/score-abandoned.json`. 이 ADR 은 [모든 판 저장·전송 ADR](2026-10-14-adr-save-and-send-every-played-run.md) 의 범위를 넓힌다.

## 날짜와 사용자 원문

2026-10-14. 결정 묶음 5번(추천안 A: "저장과 전송에 포함, 램프는 FAILED, 남은 노트는 놓친 것") — "나머지는 다 추천안대로 가면 돼".
