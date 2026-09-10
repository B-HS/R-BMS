# Workflow 스크립트 보관 (2026-09-09 ~ 09-10 고도화 세션)

Claude Code 의 `Workflow` 도구로 실행한 Phase 별 오케스트레이션 스크립트다. 각 스크립트는 `docs/plan/2026-09-09-phase-*-spec.md` 를 SSOT 로 읽고, 파일 소유권을 나눠 병렬 구현 → 적대 리뷰 → 수정 → 검증 → 문서 순으로 돈다.

| 파일 | Phase | 상태 |
|---|---|---|
| `wf-phase-b.js` | B 오디오 클럭 | 완료 |
| `wf-phase-c.js` | C 구조 개편 | 완료 |
| `wf-phase-d.js` | D 판정 패리티 | 완료 |
| `wf-phase-f.js` | F UX 고도화 | 완료 |
| `wf-phase-e.js` | E 스킨 | 완료 |
| `wf-phase-g.js` | G 데이터 스케일 | G8 배선부터 재개 필요 (`wip/phase-g`) |
| `wf-phase-h.js` | H 경미 후속 일괄 | 미착수 |

실행 전 치환: 스크립트 안의 `<reference-root>` 를 레퍼런스 구현 소스 루트 경로로 바꾼다(레포에는 그 경로·명칭을 적지 않는다). 실행: `Workflow({ scriptPath: '<복사한 경로>' })`. 중단된 실행은 `resumeFromRunId` 로 재개하면 완료 에이전트 결과가 캐시에서 재사용된다.
