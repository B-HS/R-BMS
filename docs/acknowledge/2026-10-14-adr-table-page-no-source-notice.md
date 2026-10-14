# ADR: 발광 표 페이지에서 출처·삭제 요청 문구를 뺀다

## 배경

`/tables/insane1` 페이지는 위쪽 요약("이 페이지는 darksabun.club 가 보관한 発狂BMS難易度表의 사본입니다 … 요청이 있으면 내립니다: 삭제 요청")과 맨 아래 출처 문단(한국어·영어, 연락처 GitHub 이슈)을 보였다(`web/src/features/table/table-source-notice.tsx`). 근거는 Q1 의 A(`2026-10-14-web-decisions.md`)와 [기본 발광 표 ADR](2026-10-14-adr-default-insane-table.md) 의 "출처 세 곳과 삭제 요청 문구".

## 결정

페이지의 두 문구(요약, 맨 아래 출처 문단)와 그 컴포넌트를 지운다. 오른쪽 정보 패널의 출처 항목(보관 주소, 원본 갱신, 사본 수집일), 표 카드의 출처 표기, 표 데이터의 `source.json`, 표준 `header.json` 의 `mirror_of` 는 그대로 둔다.

## 이유

사용자 지시.

## 기각된 대안

- 문구를 그대로 둔다(Q1 의 A) — 사용자가 삭제를 지시했다.

## 영향과 관련 파일

- `web/src/widgets/table/table-snapshot-workspace.tsx`, `web/src/widgets/table/table-snapshot-content.tsx`(연락처 상수와 `contactUrl` 전달 제거), 삭제 `web/src/features/table/table-source-notice.tsx`.
- 대체하는 행: `2026-10-14-web-decisions.md` 의 M1·Q1 중 "출처·삭제 요청 문구" 부분, [기본 발광 표 ADR](2026-10-14-adr-default-insane-table.md) 결정 2의 같은 부분.

## 날짜와 사용자 원문

2026-10-14. "웹페이지에서 \"이 페이지는 darksabun.club(외부 사이트, 새 탭) 가 보관한 発狂BMS難易度表의 사본입니다(…). 요청이 있으면 내립니다: 삭제 요청(외부 사이트, 새 탭)\" 이런 문구는 삭제해"
