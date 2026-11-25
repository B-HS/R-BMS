# ADR: 차트 차단 목록 — 서버가 등록하지 않는 차트(IIDX 본가 곡)

> 최종 갱신 2026-11-25 · 대응 커밋 `f1c5f0a`·`f5e5795`(첫 판: 저장소 파일), `6d29ed6`(마이그레이션 0015)와 DB 전환 커밋
> 상태: 유효 · 결정일 2026-11-25 · 결정자 사용자(요구, DB 저장, 기존 기록 삭제), 메인(응답 코드·거절 지점·표 설계)

선례: [SABOTHER 삭제 ADR](2026-10-23-adr-chart-removed-at-owner-request.md), [PopCoRian -EX- ADR](2026-11-02-adr-popcorian-ex-removed.md). 그 두 곡은 기록만 지웠고 "다시 치면 판은 다시 서버로 올라간다(서버가 차트를 거르지 않는다)" 가 남은 것이었다. 이번 장치가 그 빈자리를 막는다.

## 배경

- 사용자 요청(2026-11-25 원문): "서버에 등록안되게하는 기능이필요해 예를들어서 iidx 본가의 곡들은 등록이 되지않아야해. 파일을 입수했는데 내가 계속 제공은할건데 지금은 18, 19파일밖에 가져오지못했어 ... 먼저 웹랭킹서버에 등록안되게하는 기능을 추가하고 이 두개폴더에서 모든 곡들을 분석해서 서버에 블랙리스트 곡으로 등록하게해줘".
- 이어서: "추후에 블랙리스트 곡을 올리기위한 스크립트도 제대로 잘 정리해놔 수집하는대로 바로 등록해버리게".
- 진행 방식: 메인 단독(workflow 안 씀). 이미 운영 DB 에 있는 기록: "개수 확인 뒤 삭제"("1번이야. 근데 아마없을거야 플레이한기록은 없거든"), 남아 있던 패턴 1행도 "삭제".

## 결정

1. **목록은 운영 DB 의 `chart_blocklist` 표**(마이그레이션 0015): `sha256` 기본 키, `md5` 유일, `source_id`(묶음 id, 색인)·`source_label`·`path`(확인용 상대 경로)·`added_by`·`created_at`. 등록하면 배포 없이 다음 제출부터 막힌다.
   - 처음에는 메인이 사용자에게 묻지 않고 저장소 파일(`web/data/chart-blocklist/*.json`, `f1c5f0a`)로 만들었다. 사용자 지적 "chart-blocklist를 파일로했구나? 나는 DB에 넣는걸 원했는데 이러면 push할때마다 계속 갱신해야해서 불편해" 로 DB 로 옮겼고(설계 확인 "이대로 진행"), 파일과 생성 모듈은 지웠다. 파일판의 744개 목록은 `f1c5f0a` 에 남아 있다.
2. **등록은 관리자 API**: `PUT /api/admin/chart-blocklist`(관리자 역할의 API 토큰만, `withAdminBearer`, 사용자당 120회/분, 본문 2 MiB, 한 번에 1,000개). 본문 `{ source: { id, label }, charts: [{ md5, sha256, path }] }`, 답 `{ received, added, already_listed }`. sha256 이나 md5 가 이미 어느 묶음에든 있는 차트는 세기만 하고 넣지 않는다(`INSERT IGNORE` 로 경쟁에도 안전). `GET` 은 묶음별 차트 수.
3. **해시는 클라이언트와 같은 방식**: 차트 파일 원본 바이트의 md5·sha256(`rbms-parser::parse_with`, bmson 도 같음). 대상 확장자는 클라이언트가 읽는 `.bms .bme .bml .pms .bmson`(대소문자 무시), 숨김 파일(`.DS_Store`, `._x.bms`)은 뺀다. 바이트가 같은 파일은 path 순으로 앞의 것 하나만 보낸다.
4. **판정**: sha256 이나 md5 중 하나라도 표에 있으면 차단이다. 제출마다 색인 조회 한 번(`chartBlocklistService.isBlocked`). 해시 꼴이 아닌 값만 있으면 조회하지 않는다.
5. **거절 지점과 응답**: 다섯 쓰기 경로가 차트 행을 만들거나 기록을 저장하기 전에 `IR_CHART_BLOCKED`(422, "랭킹에 등록하지 않는 차트입니다.")로 거절한다.
   - `POST /api/scores`: 검증 뒤, 차트 행을 만들기(`ensureFromSubmission`) 전. 거절은 `submission_audit` 에 남는다(사유 `chart-blocked`).
   - `POST /api/charts`(차트 메타 upsert), `POST /api/charts/[hash]/replays`, `POST /api/charts/[hash]/traces`, `PUT /api/admin/chart-patterns/[md5]`.
   - 422 를 고른 까닭: 403 은 클라이언트 `IrError::is_auth_failure` 가 로그인 실패로 보는 경로가 있다(`user_folders.rs`). 422 는 `IrError::Server(422)` → 동기화 `Failure::Refused` 로 "거절" 집계되고, 거절된 판은 클라이언트나 서버 빌드가 바뀔 때(`refused_under`) 또는 손으로 동기화할 때만 다시 보낸다. 클라이언트는 바꾸지 않았다(모든 판은 그대로 로컬 저장·전송하고 서버가 거른다).
   - 코스 제출은 거르지 않는다. 코스는 서버에 등록된 코스만 받으며 그 정의는 관리자가 넣는다.
6. **기존 기록 정리**: `web/scripts/chart-blocklist-purge.ts <count|delete>`(Drizzle, 한 트랜잭션, 목록은 `chart_blocklist` 에서 읽음). 대상과 순서는 `chart-purge.delete.sql` 과 같다: `play_analysis`·`play_trace`·`replay`·`chart_best`·`submission_audit`·`score`·`chart_pattern`·`table_chart`·`user_folder_item`·`chart`(코스가 쓰는 차트 행은 남김, `course_chart_kept` 로 셈). `ai_result` 본문 검색은 하지 않는다. 실행은 `web-db.yml` 의 `chart-blocklist-purge-count` → 사용자 확인 → 사용자 `!` `chart-blocklist-purge-delete`. 출력은 표별 숫자뿐이다(Actions 로그 공개). 등록이 기록을 지우지는 않는다(되돌릴 수 없는 삭제는 count 를 본 뒤 따로).

## 첫 목록(2026-11-25)

| 묶음 id | label | 차트 | 원본 폴더(사용자 제공) |
| --- | --- | --- | --- |
| `iidx-18-resort-anthem` | beatmania IIDX 18 Resort Anthem | 391(전부 `.bml`) | `~/Downloads/Beatmania IIDX18 ~Resort Anthem~` (64곡 폴더) |
| `iidx-19-lincle` | beatmania IIDX 19 Lincle | 353(`.bme` 342, `.bms` 5, `.bml` 4, `.pms` 2) | `~/Downloads/Beatmania IIDX19 ~Lincle~` (60곡 폴더, `.dtx` 5개는 클라이언트가 읽지 않아 뺌) |
| `iidx-20-tricoro` | beatmania IIDX 20 Tricoro | 586 | `/Volumes/SSD/iidxbms` 의 `IIDX20 ~Tricoro~ - *` 97곡 폴더(같은 폴더의 18·19 곡은 이미 등록돼 0개 추가) |

2026-11-26 추가(사용자 "곡 더 추가했어 스크립트 돌려봐", 같은 폴더 `/Volumes/SSD/iidxbms`): `bun run blocklist:add /Volumes/SSD/iidxbms --id iidx-unsorted --label "beatmania IIDX (unsorted)"`. 버전 폴더의 IIDX 1st~16·21·22·Substream·무번호(`iidx`) 6,816개와, 버전 접두어가 없는 곡 폴더의 1,103개(`iidx-unsorted`, 이름은 사용자 선택 "beatmania IIDX (unsorted)")를 새로 넣었다. 18·19·20 은 0개 추가. 서버 24 lists, 8,022 charts. 운영 count run 45720578043: 모든 대상 0행. 운영 `/tables/blocked` 가 24레벨·8,022항목으로 바로 반영됨.

세 묶음 사이에 겹치는 해시는 없다(합 1,330). IIDX20 등록 뒤 운영 count(run 45601926550, 목록 1,330): `chart_pattern` 1행, 나머지 0행(IIDX20 차트의 기록 없음). 사용자 `!` `chart-blocklist-purge-delete` run 45602670750 으로 그 1행을 지웠고, 지운 뒤 모든 대상이 0이다.

운영 반영(2026-11-25): 사용자 `!` migrate run 45599298074(0015), 웹 `c24ccd1` 배포, `bun run blocklist:add` 로 391·353개 등록(서버 2 lists 744 charts, 다시 돌리면 0 added). 운영 count(DB 목록, run 45599854472. 파일판 run 45595998606 과 같음): `chart_pattern` 1행, 나머지 대상은 0행. 패턴 행은 관리자 패턴 업로드로 들어온 것이고 플레이 기록은 없다.

## 다음 묶음을 더하는 절차

폴더만 주면 끝난다. 커밋·푸시·배포는 없다.

```sh
cd web
bun run blocklist:add "<폴더>" ["<폴더>" ...]   # 해시 → 서버 등록 → 서버의 묶음별 수 출력
bun run blocklist:add "<폴더>" --dry-run        # 보내지 않고 찾은 수만
bun run blocklist:list                          # 서버의 묶음별 수
```

1. 차트마다 묶음을 정한다: 준 폴더나 경로의 곡 폴더 중 처음으로 IIDX 버전을 말하는 이름에서 id·label 을 만든다. 버전은 폴더 이름의 첫 ` - ` 앞부분에서만 읽는다(곡명의 `~DIRTY MIX~`·`IIDX9` 를 버전으로 오인하지 않게, 2026-11-26 수정). 예: `Beatmania IIDX18 ~Resort Anthem~`·`IIDX18 ~Resort Anthem~ - ALBIDA` → `iidx-18-resort-anthem`, `IIDX 9th Style - x` → `iidx-9-style`(label `beatmania IIDX 9th Style`), 숫자 없는 `IIDX Substream - x` → `iidx-substream`, `Beatmania IIDX - ~OMAKE~ - x` → `iidx`. 그래서 여러 버전을 한 폴더에 모아 줘도 버전별로 나뉜다. 버전을 말하지 않는 차트는 `--id`·`--label` 로 준 묶음(폴더 하나일 때), 없으면 준 폴더 이름의 묶음으로 간다.
2. 토큰은 macOS 키체인(계정 `rbms-admin`, 서비스 `rbms-pattern-upload`, 패턴 업로드와 같은 관리자 API 토큰)이나 `--token-env <환경변수>`. 출력하거나 저장하지 않는다. 서버는 기본 `https://bms.hyuns.uk`, `--server` 는 https 나 이 컴퓨터의 http 만.
3. 1,000개씩 나눠 보내고, 묶음마다 "찾은 수, 더한 수, 이미 있던 수" 를 찍는다. 같은 폴더를 다시 돌려도 더해지는 것은 없다. 같은 버전의 빠진 곡을 나중에 받아도 같은 명령이면 같은 묶음에 더해진다.
4. 끝나면 서버의 묶음별 수와 `chart-blocklist-purge-count` 명령을 알린다. 등록 전에 이미 들어온 기록이 있을지 모르면 count → 행이 있으면 사용자 `!` delete.

## 차단 곡을 폴더로 보기(난이도표 `/tables/blocked`)

사용자 요청(2026-11-25 원문): "blocked 곡은 따로 폴더로 모아서 볼수있게해줘". 보이는 곳은 "난이도표로", 클라이언트 기본 표에는 넣지 않고 "웹만, URL 은 직접 추가", 메인 단독(모두 추천안).

- 웹이 `chart_blocklist` 를 표 하나(id `blocked`, 이름 `R-BMS Blocked Charts`, 기호 빈 문자열)로 내보낸다: 표 목록(`/tables`), 페이지 `/tables/blocked`(`bmstable` 메타), 표준 `/tables/blocked/header.json`·`data.json`. 레벨은 묶음마다 하나(묶음 이름, 예 `beatmania IIDX 18 Resort Anthem`), 묶음 id 의 숫자를 수로 비교한 순서, 항목 제목은 차트 파일 이름(확장자 뺌, 표에 제목을 따로 두지 않으므로). 목록이 비면 표가 없다.
- 기호를 비운 까닭: 클라이언트는 폴더 이름을 `기호 + 레벨` 로 붙인다(`select_tree.rs` `level_title`). 기호가 있으면 `Xbeatmania IIDX 18 …` 이 된다.
- 등록하면 배포 없이 다음 읽기부터 표에 나온다. 서버 인스턴스는 목록의 (개수, 마지막 등록 시각)이 바뀔 때만 표를 다시 만든다(`createBlockedTableCache`). 표준 파일 응답은 CDN 이 최대 1시간 캐시한다(기존 표와 같음).
- 클라이언트에서 보기: 표 관리자에 `https://bms.hyuns.uk/tables/blocked` 를 더하면 선곡 화면에 표 폴더로, 묶음별 하위 폴더로 보인다. 라이브러리에 없는 차트는 기존 표처럼 흐리게 나온다.
- 코드: `web/src/server/service/domain/table-snapshot/blocked-table.ts`, `compose/table-snapshot.ts`(`createTableLoader` 의 셋째 원천), `src/app/(app)/tables/blocked/page.tsx`.

## 남는 것

- 같은 곡이라도 바이트가 다른 다른 판(다른 배포본, 수정본)은 막지 못한다. 해시가 다르면 다른 차트다. 그런 파일을 받으면 같은 명령으로 더한다.
- 목록에서 빼는 API 는 없다(요청 범위 밖). 잘못 넣으면 운영 DB 에서 그 `source_id` 행을 지워야 한다.
- 사용자 PC 의 로컬 기록은 건드리지 않는다.

## 날짜와 사용자 원문

2026-11-25. 위 배경의 요청 원문, workflow 질문에 "메인 단독 (Recommended)", 기존 기록 질문에 "1번이야. 근데 아마없을거야 플레이한기록은 없거든", DB 전환 설계에 "이대로 진행 (Recommended)", 패턴 1행에 "삭제 (Recommended)".
