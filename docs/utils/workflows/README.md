# Workflow 스크립트 보관 (2026-09-09 ~ 09-10 고도화 세션)

Claude Code 의 `Workflow` 도구로 실행한 Phase 별 오케스트레이션 스크립트 보관본이다. 각 스크립트는 당시 `docs/plan/2026-09-09-phase-*-spec.md`를 정본으로 읽고, 파일 소유권을 나눠 병렬 구현 → 적대 리뷰 → 수정 → 검증 → 문서 순으로 실행했다. 현재 상태와 다음 작업은 [PROCESS](../../PROCESS.md)와 [roadmap](../../roadmap.md)가 정본이며, 이 스크립트를 재개해 현재 상태를 추론하지 않는다.

| 파일 | Phase | 상태 |
|---|---|---|
| `wf-phase-b.js` | B 오디오 클럭 | 완료 |
| `wf-phase-c.js` | C 구조 개편 | 완료 |
| `wf-phase-d.js` | D 판정 패리티 | 완료 |
| `wf-phase-f.js` | F UX 고도화 | 완료 |
| `wf-phase-e.js` | E 스킨 | 완료 |
| `wf-phase-g.js` | G 데이터 스케일 | 완료 — [구현 이력](../../history/2026-09-16-phase-g-data-long-tail.md) |
| `wf-phase-h.js` | H 경미 후속 일괄 | 완료 — [이력](../../history/2026-09-16-phase-h-minor-followups.md), [수동 점검 절차](../../quality-assurance/2026-09-16-phase-h-manual-checks.md) |

스크립트 안의 `<reference-root>` 는 당시 레퍼런스 구현 소스 루트의 자리표시자다. 재현 조사에만 이 보관본을 사용하며, 기능 재개·배포는 현재 정본과 사용자 결정으로 새 작업을 수립한다. 남은 Phase R은 실제 키가 필요한 사용자 동반 릴리스 작업이다.
