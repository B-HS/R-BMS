# ADR: Esc 로 나간 판은 노트를 하나라도 쳤으면, 코스 도중이면 코스까지 저장·전송

> 최종 갱신 2026-10-19 · 대응 커밋 ea698a9(결정 7 은 이 커밋 뒤)
> 상태: 유효 · 결정일 2026-10-15 · 결정자 사용자(감사 결정 9) · 커밋 `f81d499`(결정 1~4), `4cc72d0`(결정 5·6), `1676c57`(종료·코스 보완). 결정 7 은 2026-10-19 사용자 지시로 결정 1·2·5 에서 노트를 하나도 치지 않은 판을 뺀다

## 배경

[Esc 포기 판 ADR](2026-10-14-adr-abandoned-run-saved.md) 은 메인 판단으로 두 예외를 두었다. 판정이 하나도 없는 판은 남기지 않고, 코스 도중 Esc 는 기록 없이 코스를 끝낸다. 메인이 이 둘을 다시 추천안으로 냈고, 사용자가 거부했다(피드백 [기록 전송에서 제외 조건을 스스로 만들지 않는다](../feedback/2026-10-15-do-not-exclude-runs.md)).

2026-10-19 에 사용자가 직접, 시작하고 노트를 아무것도 치지 않은 채 Esc 를 누른 판이 저장되는 것을 고치라고 했다(결정 7). 메인의 제외 제안을 사용자가 거부한 10-15 의 결정과 달리 이번 제외는 사용자의 지시다.

## 결정

1. **단일 곡 Esc.** 남은 노트가 있으면 노트를 하나라도 친 판을 FAILED 로 저장·전송한다. 남은 노트는 놓친 노트(늦은 POOR)다. 노트를 하나도 치지 않은 판은 예외로 남기지 않는다(결정 7). 남은 노트가 없거나 닫히는 중이면 정상 판정으로 결과 화면에 간다. 판정할 노트가 처음부터 없는 차트(BGM 만 있는 차트, SCRATCH AUTO 로 턴테이블 노트만 빠진 차트)도 남은 노트가 없는 판이다(원본 `stopPlay` 의 `getPastNotes() == getNotes()`, `BMSPlayer.java:953-958`). 램프는 NO PLAY 다.
2. **코스 도중 Esc.**
   - 그 스테이지에 남은 노트가 없거나 닫히는 중이면 정상 판정이다. 결과로 가서 코스가 이어지거나 끝난다(마지막 스테이지면 코스 결과 화면).
   - 남은 노트가 있으면(그 스테이지에서 노트를 하나라도 쳤을 때. 하나도 치지 않았으면 결정 7) 다음을 모두 저장·전송하고 선곡으로 나간다. 끝낸 곡의 곡 기록은 이미 저장되어 있다. 포기한 곡은 FAILED 곡 기록이다. 코스 기록은 FAILED 이고, 끝낸 스테이지와 포기한 스테이지가 합쳐진다. 코스 기록은 로컬 `course_submission` 에도 남아 서버가 받을 때까지 다시 보낸다.
3. 제외는 사람이 치지 않은 판(오토플레이, 리플레이 보기, 연습)과 옛 세대 2 SCRATCH AUTO 판뿐이다.
4. 서버는 이미 FAILED 코스를 받는다. 순위·최고 기록 정책은 단일 곡 실패 판과 같다(ranked, 램프 → EX → BP 순서의 최고 기록 경쟁). 서버 코드는 바꾸지 않았고 테스트로 확인했다.

5. **앱 종료로 끝난 판(S19 2차 REC2-2, 2026-10-15 추가).** 플레이 중 창 닫기·Cmd+Q·메뉴 Quit·Dock 종료는 Esc 와 같은 규칙으로 판과 코스를 남긴다. 사용자 정책 "사람이 친 판은 실패·포기·종료 포함 전부 저장·전송"에 따른다. 끄는 중에는 네트워크로 보내지 않고, 다음 실행의 기록 동기화가 보낸다(끝나는 프로세스와 함께 끊긴 요청이 같은 판을 두 번 보내게 하지 않는다). 코스의 스테이지 사이(LOADING·결정 화면)에서 꺼도 친 스테이지까지의 FAILED 코스가 남는다. 강제 종료(kill -9·정전·크래시)는 남길 수 없다.
6. **코스 스테이지 사이에서 나감(S19 2차 REC2-1).** 다음 스테이지의 LOADING 에서 Esc, 결정 화면에서 나감, 스테이지 차트를 못 읽거나 라이브러리에 없으면 코스가 그 자리에서 끝난다. 친 스테이지가 있으면 FAILED 코스로 남겨 보낸다. 서 있던 스테이지는 치지 않았으므로 아무것도 더하지 않는다.
7. **노트를 하나도 치지 않은 판(2026-10-19 추가, 결정 1·2·5 의 예외).** 시작하고 노트를 하나도 치지 않은 채 Esc 로 나가면 아무것도 저장·전송하지 않는다. 곡 기록·히스토리·리플레이·제출이 모두 없고, 선곡으로 바로 나간다.
   - "친 게 0" 은 PG·GR·GD·BD 가 모두 0 이고 머리를 잡고 쥔 롱노트가 없는 것이다(`PlayState::has_judged_nothing`, 판정 엔진 `counts` 의 앞 네 칸과 `JudgeEngine::long_note(lane).processing`). 노트가 지나가 생긴 POOR(見逃し)와 노트 없는 곳을 누른 빈 POOR 는 친 것이 아니므로 그것만 있는 판도 남기지 않는다. 일반 LN 의 머리는 뗄 때 판정되므로 쥔 채로 나가면 판정은 0 이지만 친 판이라 결정 1 대로 남긴다.
   - `leave_run` 의 순서는 연습 → 남은 노트가 없거나 닫히는 중이면 결과 → 친 노트가 없으면 버림 → 코스 또는 단일 곡의 기존 처리다. 남은 노트가 없는 판(노트가 처음부터 없는 차트)은 친 노트가 없어도 결과 화면으로 가서 정상 판정으로 남는다.
   - 코스 도중 스테이지에서 노트를 하나도 치지 않고 Esc 하면 결정 6 과 같다. 그 스테이지는 FAILED 곡 기록으로도 더하지 않고 코스가 그 자리에서 끝난다. 친 스테이지가 있으면 그 스테이지들로 FAILED 코스를 남겨 보내고, 첫 스테이지였으면 코스 기록도 없다(`app_result::leave_course` 가 `abandon_course` 로 넘긴다).
   - 앱 종료는 결정 5 가 Esc 와 같은 규칙으로 묶어 두었으므로 같은 규칙을 따른다. 친 노트가 없고 남은 노트가 있는 판은 남기지 않고, 코스 도중이면 그 스테이지 앞까지의 FAILED 코스를 남긴다(`app_result::keep_run_cut_short_by_quitting`). 끝났거나 닫히는 판은 친 노트가 없어도 결과 화면이 남겼을 그대로 남는다.

## 이유

- 사용자 원칙 "서버로 저장되지 않는 기록따윈 없어"([모든 판 저장·전송 ADR](2026-10-14-adr-save-and-send-every-played-run.md)).
- 남은 노트가 없는 판을 포기로 치면 끝까지 친 판이 FAILED 가 된다.
- 결정 7: 사용자는 "친게 0이니까" 저장되지 않아야 한다고 했고, 이어서 "빈 poor도 사실상 안친거니까 그것도 저장안하는걸로", 지나간 노트의 POOR 도 "말그대로 아무것도 안쳤을때 ... 그때 poor가 뜰텐데 그것도 너가 판단해봐"라고 했다. 메인 판단: 친 것은 노트를 맞힌 판정(PG·GR·GD·BD)과 쥔 롱노트다. POOR 는 맞히지 못한 노트, 빈 POOR 는 노트를 맞히지 못한 누름이라 둘 다 친 것이 아니다.

## 기각된 대안

| 대안 | 기각 이유 |
| --- | --- |
| 판정 수가 전부 0 인 판만 남기지 않는다(빈 POOR·놓친 POOR 가 있으면 남김, 첫 구현) | 사용자가 빈 POOR 도 친 것이 아니라고 정했다(2026-10-19) |
| 판정 수(POOR 포함)가 0 인지만 본다(쥔 롱노트 무시) | 일반 LN 머리를 쥔 채 나간 판은 친 판인데 버려진다(리뷰 지적) |
| 판정 0 인 판은 코스 기록까지 남기지 않는다(결정 7) | 친 스테이지가 있는 코스는 결정 6 대로 FAILED 코스로 남긴다. 그 스테이지만 더하지 않는다 |
| 코스 도중 Esc 는 기록하지 않는다(옛 메인 판단) | 사용자 거부 |
| 코스 Esc 때 코스 결과 화면을 보인다 | Esc 는 바로 나가는 키라는 기존 결정과 어긋난다 |

## 영향과 관련 파일

- `crates/rbms-play/src/session.rs`(`PlaySession::all_notes_resolved`: 판정한 노트 수 ≥ 노트 수, 노트 0 개면 참), `apps/rbms-player/src/stage/play/mod.rs`(`leave_run` 순서: 연습 → 남은 노트 없음·닫히는 중이면 결과 → 코스면 `leave_course` → 단일 곡이면 `record_left_run`, `keeps_a_left_run`. 결정 7: `has_judged_nothing` 이 참이면 단일 곡은 `record_left_run` 을 건너뛰고 코스는 `leave_course` 가 `abandon_course` 로 넘긴다), `app_result.rs`(`leave_course`, `record_left_run`, `fold_course_stage`, `keep_finished_course`).
- 코스 기록의 BP 에는 포기한 스테이지의 남은 노트(POOR 로 셈)가 들어가지만, 아직 치지 않은 스테이지의 노트는 들어가지 않는다(원본 `MusicResult.java:430-436` 은 더한다). 테스트 부채로 둔다.
- 서버: `web/tests/service/course.test.ts`(FAILED 코스 저장·순위·최고 기록 경쟁).
- 이 ADR 은 [Esc 포기 판 ADR](2026-10-14-adr-abandoned-run-saved.md) 의 결정 2·3 을 대체한다.
- 테스트: `app_result/record_tests.rs` `leaving_mid_course_keeps_the_stage_and_a_failed_course`, `leaving_a_course_stage_with_no_note_left_ends_it_as_played`. 결정 7: `leaving_before_any_note_is_judged_keeps_nothing`(2026-10-15 의 `leaving_before_any_note_is_judged_keeps_a_failed_run` 을 바꿈), `leaving_after_only_an_empty_poor_keeps_nothing`, `leaving_after_a_note_went_by_unhit_keeps_nothing`, `leaving_while_holding_the_first_long_note_keeps_a_failed_run`.
- 결정 5·6: `apps/rbms-player/src/lib.rs`(`App::keep_what_quitting_cuts_short`, `exiting`), `app_result.rs`(`keep_run_cut_short_by_quitting`, `abandon_course`, `Sending`), `stage/loading.rs`(`give_up_on_the_chart`), `stage/decide.rs`(`abandon`), 테스트 `app_result/record_tests/left_behind.rs`.
- 결정 7 의 코스·종료 테스트: `app_result/record_tests/left_behind.rs` `escape_on_a_course_stage_that_judged_nothing_ends_the_course_without_the_stage`, `escape_on_the_first_course_stage_that_judged_nothing_keeps_nothing`, `quitting_before_any_note_is_judged_keeps_nothing`, `quitting_in_a_course_stage_that_judged_nothing_keeps_the_course_without_the_stage`.

## 날짜와 사용자 원문

2026-10-15. "9번은 ESC 눌렀을때 남은 노트가 없다면 그냥 판단해서, 또한 실패기록도 다 서버로 보내야지 뭘 안보내려고 발악을 쳐하는거야".

2026-10-19(결정 7). "시작하고 노트를 아무것도 치지 않고 ESC를 누르면 저장되지않아야하는데 (친게 0이니까) 그냥 저장되어버리네, 이것도 수정해놔". 이어서 "아 그냥 빈 poor도 사실상 안친거니까 그것도 저장안하는걸로 가야겠네 맞어", 놓친 노트의 POOR 를 묻자 "말그대로 아무것도 안쳤을때 저장하는거야 그떄 poor가 뜰텐데 그것도 너가 판단해봐".
