# 기능 고도화 12건: 구현 단위 보고 원문 (2026-11-19)

계획 [plan/2026-11-19-feature-upgrade-12.md](../../plan/2026-11-19-feature-upgrade-12.md) 의 구현 단위(U1~U9), 테스트 웨이브(`test-*`), 수리 웨이브(`fix-*`)가 낸 보고를 그대로 옮겼다. 결정의 정본은 계획서와 ADR 이고, 이 폴더는 구현 단위가 본 것과 남긴 위험의 기록이다.

| 단위 | 내용 | 파일 |
| --- | --- | --- |
| U1 | 블루투스 출력 지연 | [U1.md](U1.md) |
| U2 | 창 모드 FULLSCREEN | [U2.md](U2.md) |
| U3 | 패드 ESC 와 조합키 | [U3.md](U3.md) |
| U4 | 어시스트 등급 램프 색 | [U4.md](U4.md) |
| U5 | 판정 표시 위치(원인 조사) | [U5.md](U5.md) |
| U6 | 플레이 화면: 출발 폴더·옵션·NPS | [U6.md](U6.md) |
| U7 | LAYOUT PICK 미리보기와 capture | [U7.md](U7.md) |
| U8 | 선곡 화면 개편 | [U8.md](U8.md) |
| U9 | 사용자 오프셋 단위 | [U9.md](U9.md) |

## 테스트 웨이브 (wf_3df278ff-bb7)

| 담당 | 내용 | 파일 |
| --- | --- | --- |
| S1 | 창 모드·오프셋 단위 검증 (Sonnet) | [test-S1.md](test-S1.md) |
| U10 | 램프 반 쪼개기 구현과 등급 램프 검증 (Opus) | [test-U10.md](test-U10.md) |
| S3 | 선곡 화면 검증 (Sonnet) | [test-S3.md](test-S3.md) |
| S4 | 문구 검증 (Sonnet) | [test-S4.md](test-S4.md) |
| O1 | 출력 지연 반례 찾기 (Opus) | [test-O1.md](test-O1.md) |
| O2 | 조합키·ESC 반례 찾기 (Opus) | [test-O2.md](test-O2.md) |
| O3 | LAYOUT PICK·capture 반례 찾기 (Opus) | [test-O3.md](test-O3.md) |
| O4 | 플레이 화면 표시 반례 찾기 (Opus) | [test-O4.md](test-O4.md) |
| DOC | 문서 (Sonnet) | [test-DOC.md](test-DOC.md) |

## 수리 웨이브 (wf_08935ad2-8ca)

| 담당 | 내용 | 파일 |
| --- | --- | --- |
| F1 | 출력 지연 후속: 기본 장치 따라가기, 판 기준점 (Opus) | [fix-F1.md](fix-F1.md) |
| F2 | 조합키·결과 화면·LAYOUT PICK 후속 (Opus) | [fix-F2.md](fix-F2.md) |
| F3 | 빌린 배치 재시도 기록 (Opus) | [fix-F3.md](fix-F3.md) |
| F4 | 스킨·문구 다듬기 (Sonnet) | [fix-F4.md](fix-F4.md) |
