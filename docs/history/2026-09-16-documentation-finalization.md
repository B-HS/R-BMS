# 문서 정합성·고도화 마무리 (2026-09-16)

## 범위

이번 감사는 루트 안내, 현재 상태 문서, 아키텍처·크레이트·개발·CI 문서, 계획·참조·QA·이력의 역할을 실제 코드·Git·검증 기록과 대조했다. 목표는 과거 실행 스냅샷을 보존하면서 새 세션이 현재 상태를 혼동 없이 읽게 하는 것이었다.

## 정본 우선순위

1. [PROCESS](../PROCESS.md) — 현재 상태, 진행 중 체크리스트, 즉시 다음 작업
2. [roadmap](../roadmap.md), [development](../development.md), [architecture](../architecture.md), [crates](../crates.md) — 남은 범위, 실행법, 구현 구조
3. [history](.), `HANDOFF.md`, `plan/`, `reference/` — 완료 근거와 당시 설계·조사·실행 스냅샷

보존 문서의 당시 TODO·브랜치·테스트 수는 작성 시점의 증거이며 현재 작업 지시가 아니다. 현재 Phase 상태는 첫 번째 우선순위 문서만 따른다.

## 주요 교정

- `PROCESS.md`의 베이스 룰과 로드맵 경로를 현재 프로젝트 작업 환경과 `docs/roadmap.md`로 정리했다.
- 2026-06 로드맵 실행 블록과 Workflow 스크립트를 보존 이력으로 표시해, 당시 G·H 미완료 지시가 현재 상태로 읽히지 않게 했다.
- 활성 문서의 실행 명령, 브랜치·CI 게이트, Phase A~H 완료와 Phase R 보류, 수동 QA 경계를 실제 기록으로 맞췄다.
- 문서 지도에 이번 감사와 Phase G·H 이력, 수동 점검 절차를 연결했다.

## 확인 근거와 경계

- Phase H 최종 `cargo test --workspace`는 3,040개 테스트를 등록하고 성공 종료했다. 실제 오디오 장치가 필요한 테스트 2건은 ignored 상태다.
- 최종 GitHub Actions CI run `35055642686`은 Linux·macOS·Windows와 fmt·clippy 게이트를 통과했다.
- 현재 개발 계통은 `dev`와 `origin/dev` 하나이며, Phase G 작업 브랜치와 이전 원격 작업 브랜치는 정리됐다.
- Phase R의 `prod` 생성, 첫 태그·릴리스, 서명 또는 배포는 실제 키가 필요하므로 착수하지 않았다. 사용자가 키를 준비한 뒤 함께 진행한다.
- AUDIO, Practice, Course, Multi-IR의 하드웨어·서버 수동 QA는 실행하지 않았다. 절차와 합격 기준은 [Phase H 수동 점검](../quality-assurance/2026-09-16-phase-h-manual-checks.md)에 보존한다.

## 검증 결과

- Markdown 127개 파일에서 후보 255개와 상대 링크 178개를 확인했고, 깨진 링크는 0개다.
- 백틱 `docs/` 실경로 후보 345개와 의도적 glob 8개를 구분했으며, 누락 경로는 0개다.
- design 섹션 11/11과 근거 파일 5/5를 확인했고, `git diff --check`는 exit 0이다.
- 독립 문서 검토 결과 blocker·important·minor는 모두 0건이며 커밋 가능 상태다.
- 이번 변경은 문서 전용이므로 코드 테스트는 재실행하지 않았다. 대신 직전 Phase H의 `cargo test --workspace` 3,040개 등록 성공과 CI run `35055642686` 성공 증거를 재사용했다.
