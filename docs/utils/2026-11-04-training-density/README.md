# 훈련곡 밀도 측정 스크립트

2026-11-04 조사에서 쓴 측정 스크립트다. 결과 해석은 [04 원인 진단](../../reference/2026-11-04-ai-training-dashboard/04-training-difficulty-diagnosis.md) 부록과 [05 실제 곡 밀도](../../reference/2026-11-04-ai-training-dashboard/05-real-chart-density.md) 10절에 있다.

- `diag/`: 합성기를 대표 프로필 3 × 강도 3 × 종류 전부 × 시드로 돌려 구간 목표 대비 실제 NPS, 2초 국소 밀도 분포, 한계 대비 비율을 잰다. `web` 디렉터리를 작업 폴더로 두고 `bun <경로>/measure.ts` 로 실행한다(저장소 모듈을 import 한다).
- `real-density/`: 로컬 곡 라이브러리에서 `rbms-cli chart-pattern` 으로 뽑은 패턴 문서를 읽어 레벨 대역별 밀도 흐름을 집계한다. 산출물 폴더 경로가 스크립트 안에 세션 임시 경로로 적혀 있으니 다시 돌릴 때 바꾼다. 패턴 문서와 산출물(곡별 배치)은 저장소에 두지 않는다.
