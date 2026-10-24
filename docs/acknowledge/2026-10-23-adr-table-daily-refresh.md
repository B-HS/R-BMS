# ADR: Stella·Satellite 표를 Vercel Cron 으로 매일 받아 운영 DB 에 저장 (웹 쪽)

> 최종 갱신 2026-10-23 · 대응 커밋 5a0eacf
> 상태: 유효(코드 반영. 결정 13 은 사용자 확인 2026-10-23: "13 - 16 추천대로". 검증 상수·저장 방식·큰 변경 수용 경로는 메인 결정이고 사용자 확인 2026-10-24(미확인 16 지금대로). 운영 완료 전: `CRON_SECRET` 등록과 첫 저장이 남았다) · 결정일 2026-10-23 · 결정자 사용자(요구 6, 결정 13), 세부 메인(사용자 "나머지도 추천대로") · 커밋 942ed71(Cron 라우트·받기·검증·저장소·조립), 9e39262(마이그레이션 0012 의 `table_snapshot`. 웨이브 1 전체가 섞인 커밋), f2d8ce5(표 페이지와 출처 표기), 608eae7(갱신 뒤 캐시 무효화), df52b3c(web-db 액션)
> 관계: 클라이언트 쪽(원본에서 직접 받기, 하루 지난 저장본을 시작할 때 백그라운드로 다시 받기)은 [Stella·Satellite 기본 표 ADR](2026-10-23-adr-stella-default-tables.md)이고 커밋은 2787264·519d7d6 이다(`COPY_STALE_AFTER_SECS` = 86,400초). 표 코스 등록과 단위 소급은 [단위 자동 판정](2026-10-23-adr-dan-auto-award.md)과 [GENOSIDE 데이터](2026-10-23-adr-genoside-2018-course-data.md).

## 배경

- 사용자 요구 6(2026-10-23): Stella 난이도표는 내 서버가 아니라 출전이 명확하므로 항상 원본에서 가져오고, 업데이트가 계속되는 표이니 한국시간 매일 새벽 0시에 Vercel 의 workflow 등으로 최신화한다.
- 3c0cd22 시점의 웹은 발광 표(insane1)만 저장소에 커밋된 스냅샷으로 가졌고(`web/data/tables/insane1`), 배포 단위로만 바뀌었다. 표 header 의 `course` 는 웹 스키마가 버렸다. 원본 표는 `cache-control public, max-age=60`, CORS 헤더 없음, 응답 서버는 Cloudflare 다. 정식 표는 2주 간격으로 반영된다고 원본이 안내한다(조사 보고 A 절).
- 결정 13: Vercel Cron(UTC 15:00)이 원본을 받아 운영 DB 에 저장하고, 클라이언트도 원본에서 직접·하루가 지나면 다시 받는다.

## 결정

### 1) 일정과 보호

- `web/vercel.json`: `crons: [{ "path": "/api/cron/tables", "schedule": "0 15 * * *" }]`. Vercel Cron 은 UTC 고정이라 15:00 UTC 는 한국시간 0시다. 공식 문서에 따르면 Hobby 요금제에서는 정밀도가 시간 단위(±59분)라 실제로는 KST 0:00~0:59 에 실행된다. 요금제는 저장소에서 확인하지 못했다. 설정은 시험(`web/tests/lib/vercel-config.test.ts`)이 고정한다.
- 라우트 `GET /api/cron/tables`(`maxDuration = 60`): `withCronSecret` 이 `Authorization: Bearer <CRON_SECRET>` 를 상수 시간으로 비교한다. 비밀이 없으면 503 `SERVICE_NOT_CONFIGURED`(이유 `cron_secret_missing`), 틀리면 401. 공식 예제의 401 이 아니라 503 인 이유는 설정 누락을 로그에서 구별하려는 것이다. 응답은 표마다 결과를 숫자와 고정 사유 코드로만 담고 `Cache-Control: private, no-store` 다.
- `CRON_SECRET` 은 선택 환경 변수이고 16자 이상이다(`CRON_SECRET_MIN_LENGTH`). 값이 있는데 16자 미만이면 환경 검증 전체(`getEnv`)가 던진다.

### 2) 받기

- 대상 두 표: `stella-st`(Stella, 머리 `https://stellabms.xyz/st/header.json`)와 `stella-sl`(Satellite, `/sl/header.json`). 표 id 는 클라이언트 기본 표 id 와 같다. header 의 `data_url` 은 `https:` 이고 호스트가 `stellabms.xyz` 일 때만 따른다(`data_url_off_site`).
- 표를 차례로 하나씩(하루 4요청), 요청 헤더 `User-Agent: R-BMS table sync (+https://bms.hyuns.uk)`, `redirect: 'manual'`(리다이렉트를 따르지 않음), 머리 최대 1 MiB, 본문 최대 8 MiB, 20초 제한(`fetchLimited`). 한 표가 실패해도 다음 표를 계속 받는다.

### 3) 검증과 저장

| 검사 | 규칙 |
| --- | --- |
| 응답 | 200 이 아니면 `http_status`, 크기 초과 `too_large`, 못 받으면 `fetch_failed`, UTF-8 JSON 이 아니면 `not_json` |
| 머리 | `name`(Stella·Satellite)·`symbol`(st·sl) 일치. 아니면 `name_mismatch`·`symbol_mismatch` |
| 항목 | 모든 항목이 md5 32자 hex 등 스키마에 맞아야 하고 md5 가 겹치면 `duplicate_md5` |
| 첫 저장 | 항목 500곡 이상(`TABLE_FIRST_SNAPSHOT_MIN_ENTRIES`), 아니면 `too_few_entries` |
| 급감 | 새 수가 이전의 90% 미만이면 `shrank`(`TABLE_MIN_KEPT_PERCENT`) |
| 변경량 | 추가 + 삭제 + 레벨 이동이 이전 수의 25% 를 넘으면 `changed_too_much`(`TABLE_MAX_CHANGE_PERCENT`) |

- 받은 바이트 그대로 gzip 해 `table_snapshot`(표마다 한 행; `header_gz`·`data_gz` mediumblob, 두 SHA-256, 항목·레벨 수, `fetched_at`, `checked_at`, `last_outcome`)에 저장한다. 해시가 이전과 같으면 `checked_at` 과 결과만 갱신한다. 이전 `data_sha256` 이 같을 때만 쓰는 조건부 갱신이라 겹친 실행이 서로 덮지 않는다.
- 같은 갱신에서 header 의 `course` 를 코스 카탈로그에 등록한다(표 하나 = 코스 묶음 하나, 사라진 코스는 `active = false` 로 기록·순위·단위를 유지). 새 단위 코스가 생기면 그 코스의 저장된 판을 소급 판정한다. 코스 동기화가 실패해도 표 저장은 되돌리지 않고 `course_sync_failed` 로 보고한다.
- 구현과 다름: 설계서의 `TABLE_MIN_KEPT_RATIO`(0.9)·`TABLE_MAX_CHANGE_RATIO`(0.25)는 코드에서 정수 퍼센트(90·25)이고 값은 같다. 사유 코드에 `course_sync_failed`·`unexpected_error` 가 더해졌다.

### 4) 큰 변경을 받아들이는 길

- `web-db.yml` 의 `tables-refresh-count`(받아서 검증만, 아무것도 쓰지 않음) → 숫자 확인(사용자) → `tables-refresh-force`(`apply-large`: 급감·변경량 검사만 건너뛰고 형식 검사는 그대로). 새 HTTP 관리 라우트는 만들지 않았다. Actions 로그는 공개라 표 id·결과·사유 코드·개수만 출력한다.

### 5) 읽기와 출처 표기 (결정 14)

- 표 목록·상세·표준 JSON 응답이 번들 표(insane1)와 DB 의 원격 표를 함께 낸다. 원격 표는 인스턴스 메모리에 두고, 작은 버전 행의 두 해시가 바뀐 때만 blob 을 풀어 다시 해석한다. 갱신 뒤에는 코스 카탈로그와, 단위를 받은 사용자의 프로필·플레이어 순위 캐시를 무효화한다(`revalidateOnTableRefresh`).
- Stella·Satellite 표 화면에는 출처 이름, 제작 주체(`stellabms.xyz`), 원본 링크, 내용 받은 날, 마지막 확인, 데이터 SHA-256 앞 12자를 보인다. 발광 표 페이지의 흔적 제거는 그대로다.

## 이유

- 원본 표가 바뀌는 대로 따라가야 하고 원본은 CORS 가 없어 브라우저에서 직접 읽을 수 없다. 서버가 받아 두면 표 페이지·코스 등록·단위 소급이 한 번의 갱신에서 맞는다.
- 받은 바이트와 해시를 그대로 보관하면 표준 `data.json` 을 원본과 같은 바이트로 내고 변경 감지가 해시 한 번이다.
- 빈 표·잘린 응답·다른 표로 바뀐 응답은 막되 정상 반영(수십 곡 규모로 관찰됨, 관찰 1회분)은 막지 않는 값이다. 큰 개편은 사람이 숫자를 보고 승인한다(운영 DB 작업은 `web-db.yml`: [운영 DB ADR](2026-10-14-adr-production-db-operations.md)).

## 기각된 대안

| 대안 | 기각 이유 |
| --- | --- |
| 저장소의 커밋된 스냅샷(insane1 방식) | 배포 단위로만 갱신된다. 사용자가 매일 최신화를 요구했다 |
| 표를 JSON 열로 저장 | 다시 직렬화돼 원본 바이트와 달라진다 |
| 항목을 정규화 표(2,350 + 2,497행)로 | 갱신이 무겁고 표준 JSON 을 다시 만들어야 한다 |
| 관리자 토큰 라우트로 큰 변경 수용 | HTTP 관리 경로를 늘린다. `web-db.yml` 흐름을 쓴다 |
| 표 코스를 수동 action 으로만 등록 | 표 판이 바뀌면 새 코스가 늦게 들어온다 |
| 표를 병렬로 받기 | 원본 부담. 차례로 받는다 |
| 비밀이 없을 때 401 | 설정 누락과 잘못된 비밀을 구별하려 503 |
| `'use cache'` 로 원격 표 캐시 | 약 1 MB 항목의 한계를 확인하지 못했다. 인스턴스 메모리 캐시를 쓴다 |

## 영향과 관련 파일

- 사본은 원본보다 최대 약 하루 늦다(Hobby 면 실행이 시간 안에서 흩어진다).
- 알려진 위험: 원본이 Cloudflare 뒤라 Vercel(icn1)에서 보낸 요청이 봇 검사에 걸릴 수 있다. 그러면 `failed:http_status` 가 매일 남는다. GitHub Actions 러너에서 받는 `tables-refresh-count` 는 성공했지만(run 39938388900: st 2,350곡·sl 2,497곡 저장 가능) Vercel 에서 받아 본 적은 없다. 막히면 GitHub Actions 일정 실행으로 옮기는 것을 사용자에게 다시 묻는다. Cron 전달은 best effort 라 누락·중복 실행이 있을 수 있어 해시 비교로 멱등하게 만들었고, 이틀 이상 `checked_at` 이 멈춰도 알림은 없다(표 화면의 "마지막 확인" 으로 눈으로 본다).
- 운영 상태(2026-10-23 확인 시점): 마이그레이션 0012 적용됨(run 39909099575). Vercel 프로젝트의 `CRON_SECRET` 등록은 사용자 몫이고 아직이다. 첫 저장은 첫 Cron 실행이나 `tables-refresh-force`(사용자 `!` 실행 대기) 로 일어난다. 마지막 `web-db` 실행은 `tables-refresh-count` 였다.
- 확인하지 못한 사실: Vercel 요금제, `maxDuration = 60` 의 허용 여부, 원본이 서버 요청을 받아 주는지(위), 정식 표 외 Sub 표의 반영 주기.
- 코드: `web/vercel.json`, `web/src/app/api/cron/tables/route.ts`, `web/src/server/{route/cron,lib/with-cron-secret,lib/env}.ts`, `web/src/server/service/domain/table-refresh/*`, `web/src/server/service/shared/http/fetch-limited.ts`, `web/src/server/compose/{table-refresh,table-snapshot}.ts`, `web/scripts/refresh-tables.ts`, `web/src/server/service/read-cache.ts`, `.github/workflows/web-db.yml`.
- 시험: `web/tests/route/cron.test.ts`, `web/tests/lib/vercel-config.test.ts`, `web/tests/service/{fetch-limited,remote-table-snapshot}.test.ts`, `web/tests/db/table-snapshot-store.test.ts`, `web/tests/scripts/refresh-tables.test.ts`, `web/tests/lib/table-refresh-revalidation.test.ts`.

## 날짜와 사용자 원문

2026-10-23.

- 요구 6: "stellabms 는 내서버가 아니라 출전가 명확하므로 stella bms 발광표에서 항상 가져오고. 이건 업데이트가 진행되는 난이도표니 한국시간 매일 새벽 0시에 업데이트 하도록 cron 도 vercel 에 workflow 나 다른것들을 이용해서 최신화 하게 만들어"
- 결정 13: "13 - 16 추천대로" (`docs/PROCESS.md` 에 기록된 원문. 13 의 내용 요약: "Vercel Cron(UTC 15:00)이 원본을 받아 운영 DB 에 저장, 클라이언트도 원본 직접·하루 지나면 다시 받음").
- 운영 반영 승인: "3번은 승인 그냥 바로 진행해놔" (마이그레이션 0012 운영 적용과 Vercel 환경 변수 등록. 시크릿 값은 대화·로그에 남기지 않는다).
