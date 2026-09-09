# 확정 결정 — 空POOR 처리 · 로컬 기록 스키마

## 대상 파일
- `crates/rbms-judge/src/matcher.rs` (`JudgeEngine::press`, `empty_poor`)
- `apps/rbms-player/src/scores.rs` · `apps/rbms-player/src/main.rs` (`enter_result`)

## 1. 空POOR(empty poor) 동작 — 레퍼런스 구현 7K 준거

원본 `JudgeProperty.SEVENKEYS`: `judgeVanish[5]=false`, `combo[5]=true`. 따라서 누름이 MS 윈도우에만
걸치고 BAD 바깥(far early)이면:

- 노트 **미소실**(재타 가능), 콤보 **미차단**, MS 게이지 페널티 적용.

### 우리 구현의 의도적 차이 (divergence)
- 레퍼런스 구현는 `score.addJudgeCount(5, …)`로 **MISS 카운트**를 증가시킨다. 우리는 별도 `empty_poor`
  카운터로 분리해 `counts[5]`(실제 見逃しMISS)와 섞지 않는다.
  - **이유**: (a) `counts[]`가 노트당 1개라는 불변식 유지(FC 판정 `max_combo==total_notes`와 `counts`
    기반 `broke` 휴리스틱이 빈 POOR로 오염되지 않음), (b) 결과화면 MISS 수의 의미 보존, (c) 빈 POOR는
    "노트 소비 없는 이벤트"라 노트당 집계와 본질이 다름.
  - **영향**: 결과화면 MISS 수는 실제 見逃し만. 빈 POOR는 기록 모달 `EMPTY POOR`로 노출. 플레이 중
    HUD는 `last_judge=Poor`로 POOR 플래시(정직한 피드백).
- 게이지 페널티는 `Judge::Miss` 델타(MS) 사용 — 레퍼런스 구현 judge 5와 동일 인덱스. 기존(노트 소비 +
  Poor델타 −6%)보다 약함(−2%)이라 **순수 개선**(노트도 살아남음).

## 2. 로컬 기록(scores.ron) 결정
- 위치: `~/.config/rbms/scores.ron`. `ScoreBook { records: Vec<ScoreRecord> }`(append-only).
- **서버와 무관하게 항상 저장**(실인터랙티브 플레이). autoplay·replay 재생은 기록 생성 안 함(중복/허수 방지).
- 램프는 레퍼런스 구현 `ClearType.id`(0..10) 정수로 영속 → 색/라벨 왕복(`clear_type_from_id`). LightAssistEasy(3)
  는 rbms에 별도 램프 없어 AssistEasy로 흡수.
- `counts: [u32;6]`는 RON 직렬화 시 **튜플 `(...)`**(고정배열=튜플). 손으로 편집/픽스처 작성 시 `[...]`는
  파싱 실패(`Expected opening '('`). 코드의 `ron::ser`는 올바르게 기록.
- best 정의: 램프 우선, 동률 시 EX 우선. 리스트는 `played_at` 최신순.

## 3. 리플레이 재생 중 이중입력 방지
`load()`의 player autoplay 플래그를 `self.autoplay && self.replay.is_none()`로 결정. 리플레이가 로드되면
player는 autoplay하지 않고 `feed_replay`만 입력을 구동한다(기존엔 self.autoplay만 봐서, 모달에서 리플레이
재생 시 autoplay+리플레이 이중 타격 위험이 있었음).
