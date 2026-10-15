# ADR: 어시스트 곡이 낀 코스도 코스로 보낸다

> 최종 갱신 2026-10-15 · 대응 커밋 ea698a9
> 상태: 유효(SCRATCH AUTO 만 낀 코스의 순위 처리는 [2026-10-14-adr-scratch-auto-ranking-toggle.md](2026-10-14-adr-scratch-auto-ranking-toggle.md) 가 이어서 바꿈) · 결정일 2026-10-14 · 결정자 사용자(선택지 B)

## 배경

웨이브 18(커밋 `8b11a17`)은 어시스트 스테이지가 든 코스를 코스로는 보내지 않고 스테이지만 단일 곡으로 보냈다. "모든 판 저장·전송" 원칙([2026-10-14-adr-save-and-send-every-played-run.md](2026-10-14-adr-save-and-send-every-played-run.md))과 어긋나 사용자에게 물었다.

## 결정

1. SCRATCH AUTO 나 넓힌 판정 스테이지가 낀 코스도 코스 기록으로 보낸다. 코스 제출 본문에 선택 필드 `scratch_auto`(어느 스테이지든 SCRATCH AUTO 였으면 true)와 `assist`(스테이지들이 보고한 어시스트 표지를 한 번씩)를 더한다. 어시스트가 없으면 두 필드를 빼서 본문이 전과 같다(`crates/rbms-ir/src/dto/score.rs` 의 `CourseSubmission`, `skip_serializing_if`).
2. 서버는 코스 어시스트를 단일 곡과 같은 정책으로 처리한다(`web/src/server/service/domain/course/course.service.ts` 가 `ranked-policy.ts` 의 `assistFlags`·`rankBlockingFlags` 를 같이 쓴다). 스키마 변경은 없었다(웨이브 19).
3. 로컬 코스 최고 기록은 단일 곡 규칙을 따른다: SCRATCH AUTO 만 낀 코스는 갱신, 넓힌 판정이 낀 코스는 제외(`apps/rbms-player/src/course_ir.rs` 의 `CourseAssist::sets_best`, `course_updates_score`).
4. 웨이브 20 이후의 현재 동작: SCRATCH AUTO 만 낀 코스는 순위에 오르고 `course_best` 의 SCRATCH AUTO 칸에 들어간다. 넓힌 판정이 낀 코스는 `ranked=false`, `flags` 로 저장되고 `course_best` 를 바꾸지 않는다.
5. 배포 순서: 서버(웹)를 먼저 올리고 클라이언트 릴리스를 나중에. 옛 서버는 새 필드를 모르고 어시스트 코스를 ranked 로 받기 때문이다.

## 이유

사용자 선택 B. 모든 판을 저장한다는 원칙을 코스에도 적용하고, 순위 판단을 서버 한 곳(`ranked-policy.ts`)에 모은다. 서버는 코스 전체로 판단하므로 어느 스테이지에 어떤 어시스트가 있었는지는 보내지 않는다.

## 기각된 대안

| 대안 | 기각 이유 |
| --- | --- |
| A. 어시스트 스테이지가 든 코스는 코스로 보내지 않고 스테이지만 단일 곡으로(웨이브 18 의 동작) | 코스 기록이 남지 않는다. 사용자 B 선택 |

## 영향과 관련 파일

- 클라이언트: `apps/rbms-player/src/course_ir.rs`, `apps/rbms-player/src/app_result.rs`, `crates/rbms-ir/src/dto/score.rs`, `crates/rbms-ir/src/http.rs`, `crates/rbms-ir/src/null.rs`, 계약 테스트 `crates/rbms-ir/src/contract_tests.rs`·`contract_tests/fixtures.rs`.
- 서버: `web/src/server/dto/course.ts`, `web/src/server/service/domain/course/course.service.ts`, `web/src/server/service/domain/score/ranked-policy.ts`, 테스트 `web/tests/service/course.test.ts`. 가이드 문구 `web/src/widgets/guide/guide-sections.ts`.
- 커밋: `ffdb7a3`(클라이언트), `12a756d`(서버), 문서 `2c7ac01`. 웨이브 19(`wf_4ac91063-e18`).

## 날짜와 사용자 원문

2026-10-14. 사용자 답은 선택지 "B"(어시스트 곡이 낀 코스도 코스로 보낸다). 질문 원문은 보존되지 않았다(`docs/history/2026-10-15-process-archive.md` 의 S11 아래 기록).
