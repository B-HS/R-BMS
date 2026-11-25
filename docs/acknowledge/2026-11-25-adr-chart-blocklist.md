# ADR: 차트 차단 목록 — 서버가 등록하지 않는 차트(IIDX 본가 곡)

> 최종 갱신 2026-11-25 · 대응 커밋 `f1c5f0a`(기능·첫 목록)와 추가 스크립트 정리 커밋
> 상태: 유효 · 결정일 2026-11-25 · 결정자 사용자(요구·기존 기록 삭제), 메인(저장 방식·응답 코드·거절 지점)

선례: [SABOTHER 삭제 ADR](2026-10-23-adr-chart-removed-at-owner-request.md), [PopCoRian -EX- ADR](2026-11-02-adr-popcorian-ex-removed.md). 그 두 곡은 기록만 지웠고 "다시 치면 판은 다시 서버로 올라간다(서버가 차트를 거르지 않는다)" 가 남은 것이었다. 이번 장치가 그 빈자리를 막는다.

## 배경

- 사용자 요청(2026-11-25 원문): "서버에 등록안되게하는 기능이필요해 예를들어서 iidx 본가의 곡들은 등록이 되지않아야해. 파일을 입수했는데 내가 계속 제공은할건데 지금은 18, 19파일밖에 가져오지못했어 ... 먼저 웹랭킹서버에 등록안되게하는 기능을 추가하고 이 두개폴더에서 모든 곡들을 분석해서 서버에 블랙리스트 곡으로 등록하게해줘".
- 진행 방식: 메인 단독(workflow 안 씀). 이미 운영 DB 에 있는 기록: 사용자 선택 "개수 확인 뒤 삭제"("1번이야. 근데 아마없을거야 플레이한기록은 없거든").

## 결정

1. **목록은 저장소 파일**: `web/data/chart-blocklist/<id>.json` 하나가 한 출처(게임 버전, 묶음)다. 형식 `{ id, label, charts: [{ md5, sha256, path }] }`(Zod `web/src/server/dto/chart-blocklist.ts`). 서버는 `chart-blocklist.data.ts` 가 가져오는 파일들을 처음 쓸 때 한 번 읽어 sha256·md5 집합으로 둔다. DB 표를 두지 않은 까닭: 마이그레이션과 관리자 업로드 API 가 필요 없고, 사용자가 폴더를 줄 때마다 메인이 생성 → 커밋 → 푸시(= 웹 배포)로 끝나며, 무엇이 차단됐는지 git 이력에 남는다. 대가는 목록 변경마다 배포가 필요하다는 것이다.
2. **해시는 클라이언트와 같은 방식**: 차트 파일 원본 바이트의 md5·sha256(`rbms-parser::parse_with`, bmson 도 같음). 대상 확장자는 클라이언트가 읽는 `.bms .bme .bml .pms .bmson`(대소문자 무시), 숨김 파일(`.DS_Store`, `._x.bms`)은 뺀다. 바이트가 같은 파일은 path 순으로 앞의 것 하나만 남긴다. `path` 는 확인용(폴더 기준 상대 경로, `/`, NFC)이다.
3. **판정**: sha256 이나 md5 중 하나라도 목록에 있으면 차단이다.
4. **거절 지점과 응답**: 다섯 쓰기 경로가 차트 행을 만들거나 기록을 저장하기 전에 `IR_CHART_BLOCKED`(422, "랭킹에 등록하지 않는 차트입니다.")로 거절한다.
   - `POST /api/scores`: 검증 뒤, 차트 행을 만들기(`ensureFromSubmission`) 전. 거절은 `submission_audit` 에 남는다(사유 `chart-blocked`).
   - `POST /api/charts`(차트 메타 upsert), `POST /api/charts/[hash]/replays`, `POST /api/charts/[hash]/traces`, `PUT /api/admin/chart-patterns/[md5]`.
   - 422 를 고른 까닭: 403 은 클라이언트 `IrError::is_auth_failure` 가 로그인 실패로 보는 경로가 있다(`user_folders.rs`). 422 는 `IrError::Server(422)` → 동기화 `Failure::Refused` 로 "거절" 집계되고, 거절된 판은 클라이언트나 서버 빌드가 바뀔 때(`refused_under`) 또는 손으로 동기화할 때만 다시 보낸다. 클라이언트는 바꾸지 않았다(모든 판은 그대로 로컬 저장·전송하고 서버가 거른다).
   - 코스 제출은 거르지 않는다. 코스는 서버에 등록된 코스만 받으며 그 정의는 관리자가 넣는다.
5. **기존 기록 정리**: `web/scripts/chart-blocklist-purge.ts <count|delete>`(Drizzle, 한 트랜잭션). 대상과 순서는 `chart-purge.delete.sql` 과 같다: `play_analysis`·`play_trace`·`replay`·`chart_best`·`submission_audit`·`score`·`chart_pattern`·`table_chart`·`user_folder_item`·`chart`(코스가 쓰는 차트 행은 남김, `course_chart_kept` 로 셈). `ai_result` 본문 검색은 하지 않는다(목록이 커지면 본문 LIKE 가 수천 개가 된다. 필요해지면 따로). 실행은 `web-db.yml` 의 `chart-blocklist-purge-count` → 사용자 확인 → 사용자 `!` `chart-blocklist-purge-delete`. 출력은 표별 숫자뿐이다(Actions 로그 공개).

## 첫 목록(2026-11-25)

| 파일 | label | 차트 | 원본 폴더(사용자 제공) |
| --- | --- | --- | --- |
| `iidx-18-resort-anthem.json` | beatmania IIDX 18 Resort Anthem | 391(전부 `.bml`) | `~/Downloads/Beatmania IIDX18 ~Resort Anthem~` (64곡 폴더) |
| `iidx-19-lincle.json` | beatmania IIDX 19 Lincle | 353(`.bme` 342, `.bms` 5, `.bml` 4, `.pms` 2) | `~/Downloads/Beatmania IIDX19 ~Lincle~` (60곡 폴더, `.dtx` 5개는 클라이언트가 읽지 않아 뺌) |

두 목록 사이에 겹치는 해시는 없다(합 744, 시험이 고정).

## 다음 묶음을 더하는 절차

사용자 요청(2026-11-25 원문): "추후에 블랙리스트 곡을 올리기위한 스크립트도 제대로 잘 정리해놔 수집하는대로 바로 등록해버리게". 그래서 폴더만 주면 끝나게 했다.

```sh
cd web
bun run blocklist:add "<폴더>" ["<폴더>" ...]            # 목록 파일과 서버 모듈만 고친다
bun run blocklist:add "<폴더>" ["<폴더>" ...] --publish  # 위 + 시험 + 커밋·푸시(=웹 배포) + 운영 count 실행
```

`blocklist:add`(`web/scripts/chart-blocklist.ts add`)가 하는 일:

1. 폴더 이름에서 id·label 을 만든다: `Beatmania IIDX20 ~tricoro~` → `iidx-20-tricoro`·`beatmania IIDX 20 tricoro`, `beatmania IIDX 9th style` → `iidx-9-style`. IIDX 가 아닌 이름은 이름을 kebab-case 로. 만들 수 없는 이름(영문·숫자가 없음)이나 다르게 부르고 싶을 때는 폴더 하나에 `--id <id> --label "<이름>"`.
2. 차트 파일을 해시해, sha256 이나 md5 가 이미 **다른** 목록에 있는 차트는 뺀다(같은 곡이 여러 버전 폴더에 있어도 한 번만 남는다).
3. 같은 id 의 목록이 있으면 합친다(지우지 않음, 이름은 처음 것). 나중에 같은 버전의 빠진 곡을 받아도 같은 명령이면 된다.
4. `web/src/server/service/domain/chart-blocklist/chart-blocklist.data.ts` 를 폴더의 파일 목록으로 다시 쓴다(손으로 고치지 않는다). `bun run blocklist:check` 와 시험이 둘이 어긋나면 실패한다.
5. 목록마다 "찾은 수, 더한 수, 이미 다른 목록에 있던 수, 목록 크기" 한 줄을 찍는다. 새로 더한 것이 없으면 아무 파일도 바꾸지 않는다.

`--publish` 가 더 하는 일(사용자가 직접 돌려도 되고 메인이 돌려도 된다):

1. 이미 스테이징된 다른 변경이 있으면 멈춘다.
2. 차단 목록 시험(`tests/service/chart-blocklist.test.ts`, `tests/scripts/chart-blocklist.test.ts`)을 돌린다.
3. 바뀐 목록 파일과 모듈만 스테이징해 `feat(web): 차트 차단 목록에 <id> 추가(<수>개)` 로 커밋하고 지금 브랜치에 푸시한다. `dev` 푸시가 웹 배포다.
4. `gh workflow run web-db.yml -f action=chart-blocklist-purge-count` 를 띄운다. 이 워크플로는 저장소를 받아 그 목록으로 세므로 배포를 기다리지 않아도 된다. 행이 있으면 `gh workflow run web-db.yml --ref dev -f action=chart-blocklist-purge-delete`(운영 쓰기라 사용자 `!`).

메인이 돌릴 때는 `--publish` 대신 평소처럼 게이트·커밋·CI 확인을 하고 운영 count 를 돌린 뒤 결과를 보고한다.

## 남는 것

- 같은 곡이라도 바이트가 다른 다른 판(다른 배포본, 수정본)은 막지 못한다. 해시가 다르면 다른 차트다. 그런 파일을 받으면 같은 방식으로 목록에 더한다.
- 목록 변경은 배포되어야 효력이 있다. 배포 전에 들어온 판은 정리 스크립트로 지운다.
- 차단 목록은 공개 저장소에 있다(해시와 파일 경로). 차트 본문이나 음원은 커밋하지 않는다.
- 사용자 PC 의 로컬 기록은 건드리지 않는다.

## 날짜와 사용자 원문

2026-11-25. 위 배경의 요청 원문, workflow 질문에 "메인 단독 (Recommended)", 기존 기록 질문에 "1번이야. 근데 아마없을거야 플레이한기록은 없거든".
