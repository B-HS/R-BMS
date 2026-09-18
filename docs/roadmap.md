# R-BMS 현재 로드맵

> 정본 상태(2026-09-16): Phase A~H는 구현·자동 검증까지 완료했습니다. 이 문서는 현재 남은 작업만 다룹니다. 과거 설계와 당시 TODO는 [계획 문서](plan/2026-09-09-enhancement-plan.md), 완료 근거는 [PROCESS](PROCESS.md)와 [Phase G 이력](history/2026-09-16-phase-g-data-long-tail.md)·[Phase H 이력](history/2026-09-16-phase-h-minor-followups.md)에 보존합니다.

## 완료된 단계

| 단계 | 완료 상태 | 근거 |
|---|---|---|
| Phase A | 정확성·파서·차트·기초 오디오/앱 보완 완료 | `docs/history/2026-09-09-phase-a-accuracy-hotfix.md` |
| Phase B | 오디오 클럭·스케줄·볼륨 경로 구현 완료 | `docs/history/2026-09-09-phase-b-audio-clock.md` |
| Phase C | 크레이트 구조·설정·저장소 개편 완료 | `docs/history/2026-09-09-phase-c-structure.md` |
| Phase D | 판정·게이지·CN/HCN 패리티 완료 | `docs/history/2026-09-09-phase-d-judge-parity.md` |
| Phase E | 스킨 로드·렌더·설정 UI 완료 | `docs/history/2026-09-09-phase-e-skin.md` |
| Phase F | 선택/설정/결과 UX 고도화 완료 | `docs/history/2026-09-10-phase-f-integration.md` |
| Phase G | SongDB·ScoreDB·Course·Practice·gamepad·system sound·multi-IR·bmson 완료 | `docs/history/2026-09-16-phase-g-data-long-tail.md` |
| Phase H | 경미 후속과 수동 점검 절차 문서화 완료 | `docs/history/2026-09-16-phase-h-minor-followups.md` |
| 스킨 시스템 완성 (2026-09-17) | 객체 21종 렌더·대체 단위·핫스팟·번들 스코프·기본 번들 v3 구현과 헤드리스 검증 완료, 커밋은 사용자 지시 대기 | `docs/history/2026-09-17-skin-system-completion.md`, `docs/HANDOFF.md` |

## 남은 작업

### Phase R — 사용자와 공동 릴리스

상태: 미착수. 현재 저장소는 dev-only 운용이며, `prod` 브랜치 생성·보호 규칙·태그·서명·공개 배포에는 사용자 보관 키와 최종 선택이 필요합니다. 키가 제공된 뒤 사용자가 함께 수행합니다. 세부 경계는 [릴리스 전략](acknowledge/release-branch-strategy.md)을 따릅니다.

### 실제 환경 수동 QA

자동 검증은 완료했지만, 다음은 실제 장치·서버 환경이 필요하여 아직 수행하지 않았습니다.

- 실제 오디오 장치에서 재생·가청·입력 지연
- Practice 구간·시작 게이지·50~200% 속도·BGA 반복 재시작
- Course 연속 게이지/콤보 carry와 결과
- 다중 IR 프로필 전환·제출·랭킹·리플레이

절차와 합격 기준은 [수동 점검 목록](quality-assurance/2026-09-16-phase-h-manual-checks.md)에 있습니다.

## 선택적 후속

- 추가 게이지·코스메틱은 제품 우선순위가 정해질 때 별도 단계로 계획합니다.
- 스킨 후속(사양 §12): 곡 선택·결정 BGM 루프, `pmchara`·`practice`·`skinpreview`·`customEvents`·`customTimers`, 객체 `click`/`act` 이벤트, 외부 CSV 스킨 직접 로드, 비디오 BGA, 24키 실제 활성화, 옵션 패널 마우스 조작. 기본 번들의 남은 미세 결함(10키 전용 `frame-dp`, 선택 화면 판정별 카운트 행, NOTES 수치 경계 물림)은 [이력](history/2026-09-17-skin-system-completion.md)의 남은 범위 절에 있습니다. 서명/공증은 릴리스 범위 밖이며 차이는 [레퍼런스 발산 기록](acknowledge/reference-divergences.md)에서 관리합니다.
