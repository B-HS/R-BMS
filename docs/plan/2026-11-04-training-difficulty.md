# 훈련곡 난이도·밀도 흐름 강화 — 설계

> 최종 갱신 2026-11-04 · 대응: 조사 [04 원인 진단](../reference/2026-11-04-ai-training-dashboard/04-training-difficulty-diagnosis.md), [05 실제 곡 밀도](../reference/2026-11-04-ai-training-dashboard/05-real-chart-density.md), 앞 설계 [2026-11-04-ai-training-dashboard.md](2026-11-04-ai-training-dashboard.md) 3절

## 1. 요청과 결정

- 사용자 요청(2026-11-04): "생성된 훈련곡이 너무쉬운데 그리고 실제 밀도처럼 나오지도않는듯하고말이야 AI 곡 생성부분을 조금 더강화해봐".
- 사용자 결정: 기본 강도는 내 밀도 한계 L 의 100~110%, 최고 강도는 L 을 넘는 115~130%. "실제 밀도처럼 안 나온다" 는 표시 NPS 대비 체감이 낮은 것과, 실제 곡처럼 오르내리는 흐름이 없는 것 둘 다.
- 메인 추천으로 정한 해석(ADR 로 알림):
  - **M1 기준 통계**: 강도 범위는 "곡 본문(카운트인·도입 제외)의 2초 국소 밀도 nps2 의 p90" 에 건다. 한계 L 이 프로필에서 2초 창 밀도로 정의되므로 같은 통계를 쓴다. 실제 곡에서 p50 : p90 : 최댓값 = 0.83 : 1.00 : 1.18(05 3.7절)이라, p90 이 범위 안이면 가장 진한 순간은 L 의 1.2~1.5배가 된다. 곡 본문 평균 μ 는 p90 / 1.31 근처가 된다(05 9.2절).
  - **M2 강도 3단계 유지, L 기준으로 다시 정의**(04 6.1 B): build `[0.90, 1.00] L`, **push(기본) `[1.00, 1.10] L`**, **over(최고) `[1.15, 1.30] L`**. 하한도 L 에 묶어 어떤 LLM 출력으로도 범위 아래로 내려가지 않는다.
  - **M3 LLM 은 구성, 서버는 수치**(04 6.2 B+C): LLM 출력에서 구간별 `target_nps` 를 없애고 `level ∈ [0, 1]`(범위 안 위치 하나)와 `shape`(흐름 원형 A 평탄·B 점증·C 중반 골·D 앞 높음-골), 구간별 종류·레인 힌트·의도·무게만 받는다. 서버가 범위·원형·시드로 시간별 목표 밀도를 계산한다.
  - **M4 실제 곡 같은 흐름**(04 6.3 A, 05 8·9절): 60초를 5초 12점 원형 곡선(05 9.4절 값 그대로)으로 깔고, 시드 고정 잡음(표준편차 0.2, 인접 점 상관 0.5)을 더한 뒤 0.5초 단위로 선형 보간한 밀도 곡선 `densityAt(t)` 를 만든다. 합성기는 구간 안에서도 이 곡선을 따라 칸마다 놓을 확률(share)을 바꾼다. 받아들이는 조건: 본문 nps2 max/p50 ≥ 1.25, 5초 12구간 변동계수 0.15~0.35.
  - **M5 도입·끝**: 카운트인 1마디는 그대로 두고, 워밍업은 최대 1구간·곡 길이의 8% 이하, 쿨다운은 없앤다(실제 곡은 끝이 가장 진하다, 05 4.1절). 원형 B 는 자체 도입이 있다.
  - **M6 종류별 손실 보정**(04 6.6): 연속 종류의 런 시작을 오차 확산으로(ceil 손실 제거), stair 는 실제로 놓이는 행 수로 주기 계산, scratch_solo·ln_release 의 노트 비율을 실제 배치에 맞춤, ln_hold 의 LN 머리를 행 계산에 포함. 목표 대비 실제 nps 오차 ±7% 이내.
  - **M7 격자 상한**(04 6.5 A): 목표 최고 밀도가 `trainingMaxNpsOf` 를 넘으면 기준 BPM 을 4의 배수로 필요한 만큼(최대 220) 올리고 `corrections` 에 남긴다. `TRAINING_NPS_CEIL` 40 → 60(05 9.6절: 실제 Stella 상위 최댓값 52).
  - **M8 한계 추정 보간**(04 6.7 A): 프로필의 `limit_nps_ex`·`limit_nps_bp` 를 2-NPS 구간 아래 끝이 아니라 기준선을 지나는 점의 선형 보간으로. `PROFILE_VERSION` 1 → 2(지문이 버전을 포함하는지 확인하고 아니면 포함).
  - **M9 표시**: PLAYLEVEL = 본문 nps2 p90 반올림(25 상한 제거, 1~60). 구간 `actual_nps` 는 그대로 노트 수 기준이고 상세 화면에 행/초를 함께 보인다(서버가 `actual_rps` 를 spec 에 더함).
  - **M10 재현성·진단**: `spec` 에 `bounds`(L, 범위, μ, 원형, 잡음 시드)를 남긴다(04 7.1절: 지금은 복원 불가). 같은 사양·시드면 같은 바이트.
  - **M11 버전**: `GENERATOR_VERSION`, `TRAINING_PLAN_PROMPT_VERSION` 올림, 골든 3벌 재생성(TS 다음 Rust). 이미 만든 곡은 저장된 바이트 그대로(지우고 다시 만들면 새 방식).

## 2. 받아들이는 조건(측정)

`docs/utils/2026-11-04-training-density/diag/measure.ts` 를 새 코드에 맞춰 고친 측정(대표 프로필 3 × 강도 3 × 종류 전부 × 시드 5)에서:

1. 본문 nps2 p90 / L 이 강도 범위 안(±5%): build 0.86~1.05, push 0.95~1.15, over 1.10~1.36.
2. 본문 nps2 max / p50 ≥ 1.25, 5초 12구간 변동계수 0.15~0.35(원형 A 는 0.12 이상).
3. 종류별 구간 actual_nps / 목표 0.93~1.07.
4. 곡 전체 평균 NPS / L: push 0.70 이상(이전 0.53~0.94).

## 3. 구현 단위

| 단위 | 모델 | 소유 | 완료 |
| --- | --- | --- | --- |
| V1 합성·계획 | opus high | `web/src/server/service/domain/training/*`(training.service.ts 는 spec 모양 변화가 닿는 곳만), `web/src/server/service/domain/ai/training-plan.service.ts`, `web/src/server/dto/training.ts`, `web/src/entities/ai.type.ts`·`web/src/entities/ai.query.ts` 의 훈련곡 화면 거울, `web/src/widgets/ai/ai-training-*`·`web/src/features/ai/ai-training-*`(강도 설명·행/초 표시만), `web/messages/{ko,en,ja}/{pages,ai}.json` 의 훈련곡 키, `contract/training-chart/*`, `apps/rbms-cli/src/chart_pattern/training_contract_tests.rs`, `crates/rbms-download/src/tests/extract.rs`, 관련 시험, `docs/utils/2026-11-04-training-density/diag/*` | 2절 조건 전부를 측정 결과로 보고, 웹 단위 시험·typecheck·lint, Rust `training_contract`·`rbms-download training` |
| V2 한계 보간 | sonnet xhigh | `web/src/server/service/domain/analysis/weakness-profile.ts`, `profile-stats.ts`(필요 시), `web/src/server/service/domain/ai/play-feature.constants.ts`, 관련 시험 | M8, 경계 시험(기준선이 두 구간 사이를 지나는 경우, 첫 구간, 떨어진 적 없음), 기존 시험 통과 |
| V3 문서 | 메인 | ADR, 조사 04·05 의 구현 결과 절, QA 3.28, 테스트 부채 | `git diff --check` |

V1·V2 는 파일이 겹치지 않아 worktree 병렬로 돈다(둘 다 `origin/dev` 기준이므로 이 문서를 먼저 푸시한다).
