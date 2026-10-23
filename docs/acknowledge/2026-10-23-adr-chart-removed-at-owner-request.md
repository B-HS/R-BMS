# ADR: 架空ユリカゴ　 [SABOTHER] 를 웹에서 모두 지움

> 최종 갱신 2026-10-23 · 대응 커밋 a6e7ffe
> 상태: 유효 · 결정일 2026-10-23 · 결정자 사용자 · 커밋 a6e7ffe(표 사본·SQL·web-db 액션), 운영 삭제 web-db run 39927201095

## 배경

- 사용자 지시(2026-10-23 원문): "웹사이트에서 "架空ユリカゴ　 [SABOTHER]" 애대한 모든 정보를 다 지워버려". 범위 질문에 "웹의 모든 흔적" 을 골랐다.
- 대상: 발광 표 ★9, md5 `f8b3b8ca5b5c5f7be1352b7aae723bee`.

## 결정

1. 웹 표 사본(`web/data/tables/insane1/data.json`)에서 그 항목을 뺀다(1,035 → 1,034곡, 26레벨 그대로). `web/scripts/snapshot-table.ts` 의 `ENTRY_MD5_REMOVED` 가 이 md5 를 늘 빼므로 다시 만들어도 돌아오지 않고, `--check` 는 원본에서 그 곡만 빠진 것과 같다고 본다. `source.json` 의 정규화 설명에 한 줄을 더했다.
2. 운영 DB 에서 그 차트의 모든 행을 지운다: `chart` 1, `score` 63, `chart_best` 1, `replay` 57, `play_trace` 63, `play_analysis` 63, `submission_audit` 63, `chart_pattern` 1(모두 플레이어 1명). 표·코스·내 폴더 항목·AI 결과는 0행이었다. SQL 은 `web/scripts/chart-purge.{count,delete}.sql`(한 트랜잭션), 실행은 `web-db.yml` 의 `chart-purge-count`·`chart-purge-delete`(사용자 `!` 실행, run 39927201095, 삭제 뒤 모든 대상 0).
3. 코스에 들어 있는 차트라면 코스 정의가 깨지지 않게 `chart` 행을 남긴다(`course_chart_kept`, 이번에는 0).

## 남는 것

- 사용자 PC 의 로컬 기록은 지우지 않았다(요청 범위는 웹).
- v0.1.7 클라이언트에 내장된 발광 표에는 다음 릴리스 전까지 남는다. 내장 사본은 다음 클라이언트 빌드부터 1,034곡이다(`apps/rbms-player/src/tablesrc/seed.rs` 가 같은 파일을 읽는다).
- 그 곡을 다시 치면 판은 다시 서버로 올라간다(서버가 차트를 거르지 않는다).

## 날짜와 사용자 원문

2026-10-23. "하는 사이에 웹사이트에서 "架空ユリカゴ　 [SABOTHER]" 애대한 모든 정보를 다 지워버려 개쓰레기노래", 범위 선택 "웹의 모든 흔적 (추천)".
