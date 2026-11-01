# ADR: PopCoRian -EX- 를 발광 표 사본과 운영 기록에서 지움

> 최종 갱신 2026-11-02 · 대응 커밋 `28085a7`·`8af1a39`·`f737c4d`
> 상태: 유효 · 결정일 2026-11-02 · 결정자 사용자 · 커밋 `28085a7`(표 사본·제외 목록·purge SQL·시험)

선례: [SABOTHER 삭제 ADR](2026-10-23-adr-chart-removed-at-owner-request.md). 같은 장치(제외 목록 `ENTRY_MD5_REMOVED`, `chart-purge` SQL, `web-db` 액션)를 두 번째 곡에 쓴다.

## 배경

- 사용자 지시(2026-11-02 원문): "웹에서, 발광표 10에 popcorian -EX- 의 난이도를 표에서 지워버려".
- 대상: 발광 표 ★10 "PopCoRian -EX-", md5 `4c2a85efe95c8ee2121effaa4627a0cf`.
- 조사 기록의 범위 질문(표 사본만 / 표 사본 + 운영 기록)에 사용자가 "표 + 운영 기록까지 삭제" 를 골랐다. 조사에서는 표 사본만 빼는 안을 추천했으나(점수 기록은 되돌릴 수 없다) 사용자가 SABOTHER 와 같은 범위를 택했다. 조사: [reference/2026-11-02-web-three-requests/insane-table-removal.md](../reference/2026-11-02-web-three-requests/insane-table-removal.md).

## 결정

1. 웹 표 사본(`web/data/tables/insane1/data.json`)에서 그 항목을 뺀다: 1,034 → 1,033곡, ★10 64 → 63곡, 레벨 26개 그대로, `header.json` 변화 없음. 새 `data.json` 의 sha256 은 `afdb520ce15343394848add244c47a78d9e2e08e9252c90a21a82456b75a31e9` 이고 `source.json` 의 `snapshot.data_file_sha256`(ETag 의 근거)·`entries`·`entries_with_valid_md5` 와 `method.normalization` 의 제외 문구를 맞췄다(`http.data[0].entries` 는 원본 응답의 수라 그대로).
2. `web/scripts/snapshot-table.ts` 의 `ENTRY_MD5_REMOVED` 가 두 md5(SABOTHER `f8b3b8ca5b5c5f7be1352b7aae723bee`, 이 곡)를 늘 뺀다. 표를 다시 만들어도 돌아오지 않고 `--check` 는 원본에서 그 곡들만 빠진 것과 같다고 본다. 제외 사유 문구는 기존 고정 문구("removed at the site owner's request")를 그대로 쓴다.
3. 운영 기록 삭제를 위해 `web/scripts/chart-purge.{count,delete}.sql` 을 두 md5 모두를 대상으로 일반화했다(`md5 IN (…)`, AI 결과 본문 검색에 이 곡의 md5 와 제목 추가). SABOTHER 는 이미 0행이라 다시 돌려도 무해하다. 실행은 `web-db.yml` 의 `chart-purge-count` → 사용자 확인 → 사용자 `!` `chart-purge-delete`(액션 설명문만 "chart" 에서 "charts" 로 고침).
4. 시험의 고정 수치를 같이 고쳤다: 웹 `ENTRY_COUNT` 1,033·`DOWNLOAD_LINK_COUNT` 1,031·★10 63곡, Rust `seed.rs`·`insane_snapshot.rs`·`fetch/tests.rs` 의 1,033. 두 쪽이 같은 `data.json` 을 `include_bytes!` 로 읽으므로 저장소 안에서는 어긋나지 않는다.

## 남는 것

- 운영 삭제 대상 행 수(count run 41332390599, 2026-11-02): `chart` 1, `score` 57, `chart_best` 1, `replay` 57, `play_trace` 57, `play_analysis` 57, `submission_audit` 57, `chart_pattern` 1(모두 플레이어 1명). 표·코스·내 폴더 항목·AI 결과는 0행. 사용자 `!` 삭제 run 41333488160(2026-11-02) 성공, 삭제 뒤 모든 대상 0.
- 사용자 PC 의 로컬 기록은 지우지 않았다(요청 범위는 웹).
- v0.1.10 클라이언트에 내장된 발광 표에는 다음 릴리스 전까지 남는다. 클라이언트는 웹에서 표를 다시 받아 갱신하므로 웹 배포 뒤 실행 때 사본이 바뀐다(정확한 시점은 `tablesrc.rs` 를 끝까지 추적하지 않았다). 내장 사본은 다음 클라이언트 빌드부터 1,033곡이다.
- 그 곡을 다시 치면 판은 다시 서버로 올라간다(서버가 차트를 거르지 않는다). 표에는 다시 나타나지 않는다.
- 제외 목록은 사유 구분 없이 md5 배열 하나다. 곡이 더 늘면 같은 배열에 더하되 사유별 구분이 필요해지면 그때 구조를 바꾼다.

## 날짜와 사용자 원문

2026-11-02. "웹에서, 발광표 10에 popcorian -EX- 의 난이도를 표에서 지워버려", 범위 선택 "표 + 운영 기록까지 삭제".
