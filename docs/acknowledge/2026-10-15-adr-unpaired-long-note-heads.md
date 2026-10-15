# ADR: 짝 없는 롱노트 머리는 원본처럼 지우고 노트 수 정의를 한 곳에 둔다

> 최종 갱신 2026-10-15 · 대응 커밋 ea698a9
> 상태: 유효 · 결정일 2026-10-15 · 결정자 사용자(감사 결정 8, 추천안) · 커밋 `f81d499`

## 배경

감사 PLAY-12. LN 채널 객체가 홀수 개라 끝나지 않는 머리를 rbms 는 모델에 남겼다. `count_playable_notes`(곡 DB 노트 수, 기본 TOTAL, 코스 미플레이 곡 노트 수)는 그 머리를 1로 세고 판정 엔진은 꼬리가 없어 노트를 만들지 않았다. 감사 때 코퍼스 18개 차트의 두 수가 어긋났다(예: 1476 대 1470, 표본 cqc.bms 6, ts5.bms 2). 선곡 화면·곡 DB 의 노트 수가 결과 화면의 `total_notes` 와 다르고, 기본 TOTAL 이 판정하지 않는 노트까지 세어 게이지 증가량이 달라졌다.

원본은 디코드가 끝날 때 아직 열려 있는 머리를 경고하고 레인에서 지운다(`BMSDecoder.decode` 의 정리 루프, `setNote(lane, null)`). 노트 수는 `TimeLine.getTotalNotes(lntype)` 가 타임라인에 남은 노트에서 센다: 일반 노트 1, LN 머리 1, 꼬리는 CN·HCN 일 때만(미정의 LN 은 LN MODE 가 LN 이 아닐 때).

## 결정

1. 끝나지 않은 머리는 디코더가 모델에서 지운다. 그 뒤의 노트는 그대로 친다(꼬리가 없으니 몸통도 없다).
2. 머리·꼬리가 어긋나는 나머지 경우(꼬리가 다른 객체에 덮인 칸 등)도 레인마다 마지막에 `long_note::repair` 가 짝 없는 쪽을 지운다. 모델은 항상 "머리, 꼬리, 사이에 아무것도 없음"의 모양이다.
3. 노트 수의 정의는 `rbms_model::NoteKind::counts_as_playable_note` 한 곳이다. `rbms_model::count_playable_notes`(`rbms_chart` 가 다시 내보낸다)와 BMSON 변환기가 그것을 쓴다. 판정 엔진의 `total_notes` 는 정리된 모델에서 같은 값이 나온다.
4. 곡 DB 는 `PARSER_VERSION` 3 으로 전체 재스캔한다(지금은 5, [몸통 노트 ADR](2026-10-15-adr-long-note-body-notes.md) 결정 5). 점수 기록은 몸통 안 일반 노트 ADR 의 6항과 같이 다룬다(규칙 세대 4 의 정의에 포함, 세대 3 이하 행은 그대로 비교).

## 기각된 대안

| 대안 | 기각 이유 |
|---|---|
| 짝 없는 머리를 일반 노트로 강등한다 | 원본은 지운다. 노트가 하나 더 생겨 원본과 노트 수가 다르다 |
| 판정 엔진이 짝 없는 머리를 세도록 고친다 | 칠 수 없는 노트가 생긴다. 모델 쪽이 틀린 것이다 |
| `count_playable_notes` 를 엔진의 노트 수로 바꿔 치환한다 | 엔진이 세는 값은 모델이 틀려서 생긴 부산물이다. 근본(모델 정리)이 남는다 |

## 영향과 관련 파일

`crates/rbms-model/src/lib.rs`(`NoteKind::counts_as_playable_note`, `count_playable_notes`), `crates/rbms-chart/src/long_note.rs`, `crates/rbms-parser/src/bmson/convert.rs`, `crates/rbms-library/src/songdb/schema.rs`. 테스트 `crates/rbms-chart/src/tests.rs`(`an_unclosed_long_note_head_is_removed` 등과 `repair` 모듈), `crates/rbms-play/tests/note_count.rs`(`the_decoder_and_the_judge_engine_count_the_same_notes`). 원본과 아직 다른 점은 `reference-divergences.md` 의 "롱노트 정리" 절.

## 날짜와 사용자 원문

2026-10-15. 결정 묶음에 "전부 추천안대로"(8번: "짝 없는 LN 머리는 원본 처리로 한 곳에서 정리").
