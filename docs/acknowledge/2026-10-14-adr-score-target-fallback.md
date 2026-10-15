# ADR: 목표 점수의 fallback 과 RATE CUSTOM

> 최종 갱신 2026-10-15 · 대응 커밋 ea698a9
> 상태: 유효 · 결정일 2026-10-14 · 결정자 사용자(요청) + 메인(설계)

## 배경

사용자 보고(2026-10-14): 판정·콤보 위 숫자와 FAST/SLOW 그래프의 목표가 LOCAL BEST 로 해도 AAA 를 목표로 계산된다. 웨이브 16 의 조사로 "AAA" 는 코드로 재현되지 않았다. 실제로는 LOCAL BEST 가 없을 때(어시스트로만 친 차트) 목표가 조용히 MAX 로 물러났고, 기본 스킨 범례가 고정 글자 TARGET 이라 무엇을 쓰는지 알 수 없었다.

## 결정

1. 설정 `judge.target_fallback`(행 이름 TARGET FALLBACK, 기본 RATE AAA)을 더한다. 목표가 가리키는 점수가 없을 때(기록 없음, IR 점수 없음, 라이벌 점수 없음) 이것으로 물러난다. 고를 수 있는 값은 차트만으로 계산되는 13개(`ScoreTarget::FALLBACKS`, `TARGET_FALLBACK_LABELS`: MAX, RATE A- ~ RATE MAX-, RANK NEXT, RATE CUSTOM).
2. RATE CUSTOM 목표를 더한다. 차트 최대치의 몇 %를 목표로 할지 `judge.target_rate_percent` 로 정한다(원본의 `RATE_<number>` 목표, `TargetProperty.java:136-146`).
3. 물러나는 순서: LOCAL BEST → fallback, IR BEST → 로컬 → fallback, RIVAL → fallback.
4. 그래프 범례, 선곡, PLAY 메뉴가 실제로 쓰는 목표를 말한다(없을 때는 `NO RECORD`·`NO IR SCORE`·`NO RIVAL SCORE` 와 물러난 목표).
5. 결과 화면과 선곡이 플레이와 같은 로컬 최고(어시스트 제외, 같은 LN 종류)를 쓴다.

## 이유

- 사용자 요청 원문의 요지: fallback score 도 따로 설정할 수 있게.
- 기본값 RATE AAA: 사용자가 "AAA 로 계산된다"고 느낀 것이 가장 흔한 기대값이고, 이전의 MAX 물러남은 사용자가 알 수 없는 동작이었다.
- 목표가 무엇인지 화면이 말하지 않은 것이 혼란의 근본 원인이므로 범례·메뉴 문구를 함께 고쳤다.

## 기각된 대안

| 대안 | 기각 이유 |
| --- | --- |
| 이전 동작(조용히 MAX 로 물러남) 유지 | 보고된 혼란의 원인 |
| fallback 으로 LOCAL BEST·IR BEST·RIVAL 도 허용 | 그 값도 없을 수 있어 다시 물러나야 한다. 차트만으로 계산되는 목표로 제한했다 |

## 영향과 관련 파일

- 설정: `crates/rbms-config/src/judge.rs`(`ScoreTarget`, `TARGET_FALLBACK_COUNT = 13`, `TARGET_FALLBACK_LABELS`, `target_fallback_from_token`), `crates/rbms-config/src/schema.rs`(`JudgeOptions::target_fallback`, `target_rate_percent`).
- 계산과 표시: `apps/rbms-player/src/app_result.rs`(`target::resolve`), `apps/rbms-player/src/skin_host/select/bridge.rs`(`target::settle`), `apps/rbms-player/src/stage/play_menu.rs`, `apps/rbms-player/src/stage/play_menu/rows.rs`, `apps/rbms-player/src/stage/play/skin_preview.rs`.
- 커밋: `4bed711`, 문서 `d7a0034`. 웨이브 16(`wf_d2cb5aad-038`). 사용자 문서 `docs/play-options.md`.

## 날짜와 사용자 원문

2026-10-14. 요청 요지(`docs/history/2026-10-15-process-archive.md` 의 "현재 작업" 절, 사용자 지시 (바)): "판정·콤보 위의 숫자와 FAST/SLOW 의 target score 가 LOCAL BEST 로 해도 AAA 를 목표로 계산된다. fallback score 도 따로 설정할 수 있게 고도화한다." 원문 그대로는 보존되지 않았다.
