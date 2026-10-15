# ADR: 발광 표 페이지에서 출처·삭제 요청 문구를 뺀다

> 최종 갱신 2026-10-15 · 대응 커밋 ea698a9
> 상태: 유효 · 결정일 2026-10-14(결정), 2026-10-15(추가 결정 1·2, 정정) · 결정자 사용자 · 커밋 `4a0872d`(결정), `ea34002`(추가 결정), `4fda17c`(추가 결정 2), `dd5518e`(정정: 다운로드 링크 복구)
> 이 파일 안에서 대체된 것: "결정"의 "정보 패널의 출처 항목 … 그대로 둔다"는 "추가 결정"이, "추가 결정 2"의 다운로드 링크(`url`·`url_diff`·`name_diff`, 곡 상세의 "받는 곳") 삭제는 "정정"이 대체한다. 지금 유효한 상태는 맨 아래 "현재 상태" 절이다.

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

## 추가 결정(2026-10-15)

사용자 원문: "출저 - 보관본 / 출처 / 보관본 / darksabun.club(외부 사이트, 새 탭) 이딴식으로 적혀있는데 제발 다른사이트의 흔적좀 지워놔".

- 오른쪽 정보 패널의 "출처" 묶음(보관본 링크, 사본 수집일)을 지우고 "표 정보"(갱신일, 곡·레벨 수, 표 주소 복사)만 남긴다. "원본 갱신"은 "갱신"으로.
- 표 카드의 "출처 darksabun.club", 페이지 설명의 "보관본 사본"을 지운다.
- 공개 응답에서도 지운다: 표준 `header.json` 의 `original_url`·`mirror_of`, 화면용 표 요약 API 의 `original_url`·`source`. `web/data/tables/insane1/source.json`(스냅샷을 다시 받는 스크립트의 입력)은 서빙하지 않으므로 저장소에 둔다.
- 위 "결정"의 "오른쪽 정보 패널의 출처 항목 … 그대로 둔다"를 이 절이 대체한다.

## 추가 결정 2(2026-10-15): 곡별 데이터의 외부 주소도 지운다

사용자 원문: "흠.. 결국 해당 페이지가 원천이 되는거니까 B인가 ?", "B가 맞겠다 어차피 해당 난이도표는 거의 9년동안 갱신되지않았고 갱신될 예정도 없어".

- 스냅샷 스크립트(`web/scripts/snapshot-table.ts`)가 정규화 단계에서 `header.json` 의 `original_url` 을 빼고, 곡마다 `url`·`url_diff`·`name_diff`·`comment` 를 빈 문자열로 두고, 다른 글자 필드(artist 9건)에서 웹 주소를 뗀다. 다시 받아도(`--check`) 같은 결과다.
- 바뀐 값: url 1,033, url_diff 825, name_diff 782, comment 792, artist 9. md5·level·title·lr2_bmsid 는 그대로.
- 화면: 곡 상세의 "받는 곳"(본편·차분 링크)과 표의 코멘트를 지운다. 표 항목 API 에서도 그 필드를 뺀다. 표준 `data.json` 은 키를 남기되 값은 비어 있다(BMS 표 형식 호환). (다운로드 링크 부분은 아래 "정정"이 되돌렸다.)
- R-BMS 클라이언트는 이 파일을 실행 파일에 내장하므로(`apps/rbms-player/src/tablesrc/seed.rs`) 다음 빌드부터 같은 데이터다.
- 기각: 곡 제작자 사이트 링크만 남기기(D) — 사용자가 이 페이지를 원천으로 삼기로 함.

## 정정(2026-10-15): 곡 다운로드 링크는 남긴다

사용자 원문: "어라 잠시만 외부주소 말한게 곡 다운로드 받는 그곳말한거야 ? 그건 지우면안되는데". 위 "추가 결정 2" 의 다운로드 링크 삭제를 되돌린다(커밋 4fda17c 의 일부를 되돌림).

- 남긴다(원본 그대로): 곡마다 `url`·`url_diff`·`name_diff`, 곡 상세의 "받는 곳"(본편·차분 링크).
- 지운 채로 둔다: `comment`(원본 표 운영자의 메모, 792건), artist 의 웹 주소(9건), header 의 `original_url`, 화면의 출처 표기.
- 스냅샷 정규화가 이 규칙을 적용하고 `--check` 가 통과한다.

## 현재 상태(2026-10-15, `dd5518e` 기준)

- 페이지·카드·정보 패널에 다른 사이트의 출처 표기가 없다. 정보 패널은 "표 정보"(갱신일, 곡·레벨 수, 표 주소 복사)만 보인다.
- 표준 `header.json` 에 `original_url`·`mirror_of` 가 없다. 화면용 표 요약 API 에 `original_url`·`source` 가 없다.
- 곡 데이터: `comment` 는 빈 문자열(792건), artist 의 웹 주소는 뗐다(9건). `url`·`url_diff`·`name_diff` 는 원본 그대로이고 곡 상세의 "받는 곳"과 표 항목 API(`web/src/server/dto/table-snapshot.ts`, `table-snapshot.service.ts`)에 있다(`web/src/features/table/table-external-link.tsx`, `web/src/widgets/table/table-chart-details.tsx`).
- 규칙의 정본은 스냅샷 스크립트의 상수다: `web/scripts/snapshot-table.ts` 의 `HEADER_FIELDS_DROPPED`(`original_url`), `ENTRY_FIELDS_EMPTIED`(`comment`), `ENTRY_FIELDS_KEPT`(`url`, `url_diff`, `name_diff`), `WEB_ADDRESS_PATTERN`. `--check` 로 커밋된 스냅샷과 대조한다.
- 저장소에 남은 흔적(서빙하지 않음): `web/data/tables/insane1/source.json`(원본 주소와 HTTP 기록, 스냅샷 재현용), `snapshot-table.ts` 의 사용 예, `crates/rbms-config/src/tables.rs` 의 `same_as: ["darksabun.club/table/archive/insane1"]`(원본 주소를 이미 등록한 사용자에게 사본을 중복 등록하지 않기 위한 값)와 그 테스트, `docs` 의 조사 문서. 지울지는 사용자 확인 대기(색인의 "확인받지 않은 메인의 결정").

## 기각된 대안(추가 결정과 정정)

| 대안 | 기각 이유 |
| --- | --- |
| 정보 패널의 출처 항목은 남긴다(첫 결정의 범위) | 사용자 "제발 다른사이트의 흔적좀 지워놔" |
| 곡 제작자 사이트 링크만 남긴다(D) | 사용자가 이 페이지를 원천으로 삼기로 함(B) |
| 곡 다운로드 링크까지 지운다(`4fda17c`) | 사용자 정정 "그건 지우면안되는데". `dd5518e` 로 되돌림 |
