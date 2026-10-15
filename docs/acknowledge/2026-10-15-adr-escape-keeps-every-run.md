# ADR: Esc 로 나간 판은 판정 수와 상관없이, 코스 도중이면 코스까지 저장·전송

> 최종 갱신 2026-10-15 · 대응 커밋 ea698a9
> 상태: 유효 · 결정일 2026-10-15 · 결정자 사용자(감사 결정 9) · 커밋 `f81d499`(결정 1~4), `4cc72d0`(결정 5·6), `1676c57`(종료·코스 보완)

## 배경

[Esc 포기 판 ADR](2026-10-14-adr-abandoned-run-saved.md) 은 메인 판단으로 두 예외를 두었다. 판정이 하나도 없는 판은 남기지 않고, 코스 도중 Esc 는 기록 없이 코스를 끝낸다. 메인이 이 둘을 다시 추천안으로 냈고, 사용자가 거부했다(피드백 [기록 전송에서 제외 조건을 스스로 만들지 않는다](../feedback/2026-10-15-do-not-exclude-runs.md)).

## 결정

1. **단일 곡 Esc.** 남은 노트가 있으면 판정 수와 상관없이 FAILED 로 저장·전송한다. 남은 노트는 놓친 노트(늦은 POOR)다. 판정이 0 인 판도 같다. 남은 노트가 없거나 닫히는 중이면 정상 판정으로 결과 화면에 간다. 판정할 노트가 처음부터 없는 차트(BGM 만 있는 차트, SCRATCH AUTO 로 턴테이블 노트만 빠진 차트)도 남은 노트가 없는 판이다(원본 `stopPlay` 의 `getPastNotes() == getNotes()`, `BMSPlayer.java:953-958`). 램프는 NO PLAY 다.
2. **코스 도중 Esc.**
   - 그 스테이지에 남은 노트가 없거나 닫히는 중이면 정상 판정이다. 결과로 가서 코스가 이어지거나 끝난다(마지막 스테이지면 코스 결과 화면).
   - 남은 노트가 있으면 다음을 모두 저장·전송하고 선곡으로 나간다. 끝낸 곡의 곡 기록은 이미 저장되어 있다. 포기한 곡은 FAILED 곡 기록이다. 코스 기록은 FAILED 이고, 끝낸 스테이지와 포기한 스테이지가 합쳐진다. 코스 기록은 로컬 `course_submission` 에도 남아 서버가 받을 때까지 다시 보낸다.
3. 제외는 사람이 치지 않은 판(오토플레이, 리플레이 보기, 연습)과 옛 세대 2 SCRATCH AUTO 판뿐이다.
4. 서버는 이미 FAILED 코스를 받는다. 순위·최고 기록 정책은 단일 곡 실패 판과 같다(ranked, 램프 → EX → BP 순서의 최고 기록 경쟁). 서버 코드는 바꾸지 않았고 테스트로 확인했다.

5. **앱 종료로 끝난 판(S19 2차 REC2-2, 2026-10-15 추가).** 플레이 중 창 닫기·Cmd+Q·메뉴 Quit·Dock 종료는 Esc 와 같은 규칙으로 판과 코스를 남긴다. 사용자 정책 "사람이 친 판은 실패·포기·종료 포함 전부 저장·전송"에 따른다. 끄는 중에는 네트워크로 보내지 않고, 다음 실행의 기록 동기화가 보낸다(끝나는 프로세스와 함께 끊긴 요청이 같은 판을 두 번 보내게 하지 않는다). 코스의 스테이지 사이(LOADING·결정 화면)에서 꺼도 친 스테이지까지의 FAILED 코스가 남는다. 강제 종료(kill -9·정전·크래시)는 남길 수 없다.
6. **코스 스테이지 사이에서 나감(S19 2차 REC2-1).** 다음 스테이지의 LOADING 에서 Esc, 결정 화면에서 나감, 스테이지 차트를 못 읽거나 라이브러리에 없으면 코스가 그 자리에서 끝난다. 친 스테이지가 있으면 FAILED 코스로 남겨 보낸다. 서 있던 스테이지는 치지 않았으므로 아무것도 더하지 않는다.

## 이유

- 사용자 원칙 "서버로 저장되지 않는 기록따윈 없어"([모든 판 저장·전송 ADR](2026-10-14-adr-save-and-send-every-played-run.md)).
- 남은 노트가 없는 판을 포기로 치면 끝까지 친 판이 FAILED 가 된다.

## 기각된 대안

| 대안 | 기각 이유 |
| --- | --- |
| 판정이 없는 판은 남기지 않는다(옛 메인 판단) | 사용자 거부 |
| 코스 도중 Esc 는 기록하지 않는다(옛 메인 판단) | 사용자 거부 |
| 코스 Esc 때 코스 결과 화면을 보인다 | Esc 는 바로 나가는 키라는 기존 결정과 어긋난다 |

## 영향과 관련 파일

- `crates/rbms-play/src/session.rs`(`PlaySession::all_notes_resolved`: 판정한 노트 수 ≥ 노트 수, 노트 0 개면 참), `apps/rbms-player/src/stage/play/mod.rs`(`leave_run` 순서: 연습 → 남은 노트 없음·닫히는 중이면 결과 → 코스면 `leave_course` → 단일 곡이면 `record_left_run`, `keeps_a_left_run`), `app_result.rs`(`leave_course`, `record_left_run`, `fold_course_stage`, `keep_finished_course`).
- 코스 기록의 BP 에는 포기한 스테이지의 남은 노트(POOR 로 셈)가 들어가지만, 아직 치지 않은 스테이지의 노트는 들어가지 않는다(원본 `MusicResult.java:430-436` 은 더한다). 테스트 부채로 둔다.
- 서버: `web/tests/service/course.test.ts`(FAILED 코스 저장·순위·최고 기록 경쟁).
- 이 ADR 은 [Esc 포기 판 ADR](2026-10-14-adr-abandoned-run-saved.md) 의 결정 2·3 을 대체한다.
- 테스트: `app_result/record_tests.rs` `leaving_before_any_note_is_judged_keeps_a_failed_run`, `leaving_mid_course_keeps_the_stage_and_a_failed_course`, `leaving_a_course_stage_with_no_note_left_ends_it_as_played`.
- 결정 5·6: `apps/rbms-player/src/lib.rs`(`App::keep_what_quitting_cuts_short`, `exiting`), `app_result.rs`(`keep_run_cut_short_by_quitting`, `abandon_course`, `Sending`), `stage/loading.rs`(`give_up_on_the_chart`), `stage/decide.rs`(`abandon`), 테스트 `app_result/record_tests/left_behind.rs`.

## 날짜와 사용자 원문

2026-10-15. "9번은 ESC 눌렀을때 남은 노트가 없다면 그냥 판단해서, 또한 실패기록도 다 서버로 보내야지 뭘 안보내려고 발악을 쳐하는거야".
