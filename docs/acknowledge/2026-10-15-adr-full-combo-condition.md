# ADR: 풀콤 조건은 원본대로(끝에 남은 콤보 = 지나간 노트)

> 상태: 유효 · 결정일 2026-10-15 · 결정자 사용자(감사 결정 6, 추천안) · 커밋 미정(메인이 채운다)

## 배경

감사 PLAY-9. `clear_lamp` 는 `BAD+POOR == 0 && max_combo == total_notes` 로 풀콤을 정했다. 원본은 `judge.getPastNotes() == judge.getCombo()`, 즉 끝에 남은 콤보가 지나간 노트 수와 같은지를 본다(`BMSPlayer.java:868`). 5키·pop'n 은 空POOR 가 콤보를 끊으므로(`JudgeProperty.java:18,40`), 마지막 노트 뒤 빈 누름 하나가 있어도 rbms 는 MAX, 원본은 게이지 램프였다.

## 결정

1. 풀콤 계열(FC·PERFECT·MAX)은 끝에 남은 스테이지 콤보가 지나간 노트 수와 같을 때다(`clear_lamp(gauge, counts, combo, pass_notes, total_notes)`). MAX·PERFECT 는 지금처럼 GREAT·GOOD 수로 가른다. 지나간 노트가 0 이면 풀콤이 아니다.
2. `BAD+POOR` 검사는 원본에 없으므로 뺀다. 결과로 pop'n 에서 BAD(노트를 남김) 뒤 같은 노트를 다시 쳐 콤보가 끝까지 이어지면 원본처럼 풀콤이다.

## 이유

- 원본 정의를 따른다(사용자 결정). 7키는 空POOR 가 콤보를 끊지 않아 결과가 같다.

## 기각된 대안

| 대안 | 기각 이유 |
| --- | --- |
| `max_combo == total_notes` 유지 | 5K·PMS 에서 마지막 노트 뒤 空POOR 가 있어도 MAX 가 된다 |
| 원본 조건에 BAD 검사를 더 붙인다 | 원본과 다른 규칙을 새로 만든다 |

## 영향과 관련 파일

- `crates/rbms-judge/src/gauge.rs`(`clear_lamp`), `matcher.rs`(`JudgeEngine::clear_lamp` 가 스테이지 콤보와 `pass_notes` 를 넘긴다). 스테이지 콤보는 [코스 곡별 콤보 ADR](2026-10-15-adr-course-stage-combo.md).
- 테스트: `gauge.rs` `lamp_reads_the_combo_standing_at_the_end_against_the_notes_got_through`(그 밖의 `clear_lamp` 테스트는 새 인자로 고침), `session.rs` `five_key_empty_poor_after_the_last_note_breaks_the_full_combo`.

## 날짜와 사용자 원문

2026-10-15. 결정 묶음에 "전부 추천안대로"(6번: "풀콤 조건 원본대로(5K·PMS 의 마지막 노트 뒤 空POOR)").
