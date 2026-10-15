# 보조 도구 보관 (docs/utils)

최종 갱신 2026-10-15 · 대응 커밋 ea698a9

작업에 쓴 보조 스크립트를 보관한다. 제품 코드가 아니며 CI 가 돌리지 않는다. 현재 상태와 다음 작업은 [PROCESS](../PROCESS.md)와 [roadmap](../roadmap.md)가 정본이다.

| 경로 | 무엇 | 쓰는 법 |
| --- | --- | --- |
| `class-disassembler.py` | JVM 클래스 파일의 각 메서드 `Code` 속성을 바이트코드 명령 목록으로 출력한다(표준 라이브러리 `struct` 만 씀). 원본 구현의 BMS 디코더 `jbms-parser`(`beatoraja/lib/jbms-parser.jar`)는 소스가 없어 이것으로 롱노트·`#LNOBJ` 정리 규칙을 바이트코드까지 읽어 확정했다([롱노트 몸통 안 노트 ADR](../acknowledge/2026-10-15-adr-long-note-body-notes.md), [원본과 다른 점](../acknowledge/reference-divergences.md) "롱노트 정리") | `python3 -I docs/utils/class-disassembler.py <파일.class> [메서드 이름]`. 메서드 이름을 주면 그 메서드만 출력한다. 인자를 주지 않으면 `IndexError` 로 멈춘다 |
| [`workflows/`](workflows/README.md) | 2026-09-09~10 고도화 세션의 Phase B~H 오케스트레이션 스크립트 보관본 | 재현 조사에만 쓴다. 기능 재개는 현재 정본으로 새 작업을 세운다 |

원본 구현의 소스나 `.class` 파일은 저장소에 두지 않는다. 필요할 때 사용자가 가진 사본에서 꺼낸다.
