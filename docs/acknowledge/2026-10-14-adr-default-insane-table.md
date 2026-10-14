# ADR: 기본 발광 난이도표 — 스냅샷 커밋, 사본 주소, 실행 파일 내장, 한 번 등록

> 최종 갱신 2026-10-14 · 대응 커밋 7095ef7
> 상태: 유효 · 결정일 2026-10-14 · 결정자 사용자(U2, U3, Q1, Q2) + 메인(M1, M2, M5)
> 세부 결정 표의 정본: [2026-10-14-web-decisions.md](2026-10-14-web-decisions.md). 이 ADR 은 그 가운데 기본 표에 관한 것을 한 결정으로 묶고 현재 코드와 대조한 것이다.

## 배경

사용자 지시(2026-10-14): https://darksabun.club/table/archive/insane1/ 의 표 정보를 가져와 저장하고, 웹페이지에서 발광 난이도 페이지를 제공하며, R-BMS 에 기본으로 등록되게 한다. 조사(`docs/web/2026-10-14-insane-table.md`): 1,035곡, 레벨 `1`~`25` 와 `???`, 전 곡 md5 만(sha256 없음), 원본에 재배포 금지 문구도 허락 문구도 없고 페이지는 NOINDEX.

## 결정

1. 표를 저장소에 스냅샷(JSON)으로 커밋한다: `web/data/tables/insane1/{header,data,source}.json`. 다시 받는 스크립트 `web/scripts/snapshot-table.ts`(`--check` 로 일치 확인).
2. 웹이 표준 BMS 표 형식(`/tables/insane1/header.json`, `/tables/insane1/data.json`)과 페이지(`/tables/insane1`)로 제공한다. DB(`table_chart`)에 넣지 않고 스냅샷 파일을 직접 읽는다. 정본 주소는 `https://bms.hyuns.uk/tables/insane1`(끝 슬래시 없음, `bmstable` 메타는 루트 상대 경로). 사본 페이지는 `noindex`, 출처 세 곳과 삭제 요청을 받는다는 문구를 넣는다. 기본 보기는 레벨별 카드 격자 + 상세 대화상자.
3. R-BMS 의 기본 등록 주소는 원본이 아니라 사본 `https://bms.hyuns.uk/tables/insane1` 이다(`crates/rbms-config/src/tables.rs` 의 `DEFAULT_TABLES`, id `insane1`, `same_as: ["darksabun.club/table/archive/insane1"]`).
4. 스냅샷 두 파일을 실행 파일에 넣어(`include_bytes!`, `apps/rbms-player/src/tablesrc/seed.rs`) 저장본까지 심는다. 오프라인 첫 실행에도 표가 보인다.
5. 한 번만 등록한다. 설정 `library.seeded_default_tables` 에 id 를 적고, 지운 표는 다시 넣지 않는다. 같은 표의 다른 주소(원본 주소, `http://`, 끝 슬래시, `/index.html`·`/header.json`·`/data.json` 붙은 꼴)가 이미 있으면 넣지 않고 id 만 적는다(`LibraryOptions::seed_default_tables`, `table_address_key`). 설정 스키마 6(`CURRENT_SCHEMA_VERSION = 6`).
6. 스냅샷만 바뀐 웹 커밋에서도 Rust CI 를 돈다(`.github/workflows/ci.yml` 의 `web/data/tables/**` 예외, 커밋 `38153fc`).

## 이유

- 사본: 원본은 더 이상 바뀌지 않는 보관판이고, 원본 사이트가 내려가도 플레이어가 표를 잃지 않게 한다(`tables.rs` 의 문서 주석).
- DB 가 아니라 파일: 기존 `table_chart` 는 sha256 이 NOT NULL 이라 md5 만 있는 이 표를 담지 못한다(M2).
- 실행 파일 내장 + 한 번 등록: 두 설계 문서의 추천이 갈린 곳에서 user-folders 쪽(M5)을 택했다. 첫 실행이 네트워크에 기대지 않고, 지운 사용자의 선택을 존중한다.
- 출처·삭제 요청 문구: 원본에 재배포 조건이 없어 Q1 의 A(문구를 넣고 진행, 연락은 사용자 판단).

## 기각된 대안

| 대안 | 기각 이유 |
| --- | --- |
| 표를 DB(`table_chart`)에 넣기 | sha256 NOT NULL 이라 md5 만 있는 표를 못 담음 |
| 원본 주소를 기본 등록(Q1 의 B) | 원본 사이트에 기댄다. 사용자 A 선택(U3) |
| 첫 실행에 네트워크로 받기만 하기(웹 설계 문서 쪽 추천) | 오프라인 첫 실행에 표가 없다(M5) |
| 한 곡 한 행 표 보기를 기본으로(Q2 의 B) | 사용자 A 선택. 2차 작업으로 남김 |

## 영향과 관련 파일

- 웹: `web/data/tables/insane1/*`, `web/scripts/snapshot-table.ts`, `web/src/app/tables/[tableId]/header.json/route.ts`, `web/src/app/tables/[tableId]/data.json/route.ts`, `web/src/app/(app)/tables`, `web/src/app/api/tables`, `web/src/server/route/table-snapshot.ts`.
- 클라이언트: `crates/rbms-config/src/tables.rs`, `crates/rbms-config/src/schema.rs`(`seeded_default_tables`), `apps/rbms-player/src/tablesrc/seed.rs`, 테스트 `crates/rbms-table/src/fetch/tests.rs`.
- 커밋: `1e56fb4`(스냅샷), `d38b6fb`(표 제공), `711b518`(표 페이지), `8b11a17`(클라이언트 내장·등록), `38153fc`(CI).
- 운영 확인(2026-10-14, 커밋 `d81eafc` 배포): `header.json` 200, `data_url` 이 `https://bms.hyuns.uk/tables/insane1/data.json`, `data.json` 1,035곡, head 에 `bmstable` 메타와 noindex.
- 남은 일(2차): 한 곡 한 행 보기, 카드 높이 선택, 표 RSC 자료 축소(`level`·`url_diff` 중복).

## 날짜와 사용자 원문

2026-10-14. 지시 요지(`docs/PROCESS.md` 현재 작업의 (가)): "https://darksabun.club/table/archive/insane1/ 의 표 정보를 가져와 저장한다. 웹페이지에서 발광 난이도 페이지를 제공하고 R-BMS 에 기본으로 등록된다." 결정 답: U2·U3(사용자 결정), Q1·Q2 "여섯 개 모두 A(추천안)".
