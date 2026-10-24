# ADR: GENOSIDE 2018 단위 코스 44개의 데이터 파일, 생성기, 출처, 표시 위치

> 최종 갱신 2026-10-23 · 대응 커밋 5a0eacf
> 상태: 유효(결정 18 은 사용자 확인 2026-10-23: "DP도 포함". 데이터 파일 구조·제약과 트로피의 원천·DP 값·표시 위치는 메인 결정이고 사용자 확인 2026-10-24(미확인 16 지금대로)) · 결정일 2026-10-23 · 결정자 사용자(범위), 세부 메인(사용자 "나머지도 추천대로") · 커밋 f174f2b(데이터 파일·생성기·`bundled`·`table`), ea5829d(DP 차트 제목 88개), a52d9a3(COURSE 탭), 0247561(발광 표 안 COURSE 폴더), c79f53f(웹 등록 서비스와 스크립트), 608eae7(출처 표기 화면), df52b3c(web-db 액션)
> 관계: 단위 판정은 [단위 자동 판정](2026-10-23-adr-dan-auto-award.md). 발광 표 페이지의 다른 사이트 흔적 제거([2026-10-14 ADR](2026-10-14-adr-table-page-no-source-notice.md))는 그대로이고, 코스의 출처는 코스 목록 화면에 따로 표기한다(결정 14).

## 배경

- 사용자 요구 8(2026-10-23): 발광 GENOSIDE 단위 표로 들어가 모든 단위를 정확하게 가져와 서버에 등록하고, 출처도 등록하며, 단위를 얻으면 내 정보에 넣고, 다른 코스들도 만든다.
- 결정 18: GENOSIDE 2018 SP 22개 + DP 22개 + Stella·Satellite 표 코스. 사용자는 DP 를 포함하라고 답했다.
- 3c0cd22 시점에는 번들 코스가 없었다. 코스 파일은 설정 폴더의 `courses/*.json` 뿐이었고, 표 header 의 `course` 는 보관만 하고 읽지 않았다.

## 결정

### 1) 원천: 제작자의 공식 코스 파일

- 공식 코스 파일 `2018_course.zip`(4,084 바이트, SHA-256 `c426719dcf13c279fe003fcd80258fd625c3b796e7c5adda21abec1655df65ca`)의 항목 `test.lr2crs`(12,042 바이트, SHA-256 `9d7981aeab8259d96349cc32237164ba1798344e6372cb6c7ee3bb83b417ffb7`). Shift_JIS XML, 코스 44개: SP(`<line>7`) 22 + DP(`<line>14`) 22. 공식 순서는 어려운 단부터이고 데이터는 쉬운 단부터 뒤집어 쓴다(初段 … 十段, 発狂初段 … 発狂皆伝, Overjoy).
- `<hash>` 는 접두사 32자(`00000000002000000000000000005190`, 44개 동일) + md5 32자 x 4 이다. `<type>` 은 44개 모두 2 다. 이 두 값의 의미는 확인하지 못해(beatoraja 에는 `.lr2crs` 를 읽는 코드가 없다) 쓰지 않고 출처 기록에만 남겼다.

### 2) 데이터 파일과 출처 필드

- `web/data/courses/genoside-2018/courses.json`(schema 1). 클라이언트가 `include_str!` 로 내장하고(`rbms_course::genoside_2018()`) 웹 등록 스크립트가 같은 파일을 읽는다.
- `source`: `maker`, `page_url`, `file_url`, `retrieved_on`(2026-10-23), `file_bytes`, `file_sha256`, `entry_name`, `entry_sha256`, `entry_hash_prefix`, `entry_type`, `constraint_sources`(제약·트로피를 읽은 표 header 둘의 주소, SHA-256, 받은 날), `title_source`.
- 묶음 `genoside-2018-sp`(keys 7, 22코스)와 `genoside-2018-dp`(keys 14, 22코스). 코스마다 `course_hash`, `name`(공식 `<title>` 그대로), `charts`(md5 와 표시용 `title`, sha256 은 비움), `constraint`, `trophy`, `release: true`.
- 제약·트로피: SP 22개는 beatoraja 용 표 header 두 개(通常 ☆, 発狂 ★)에서 md5 열이 같은 코스를 찾아 그 값을 쓴다(이름이 아니라 md5 열로 22/22 일치, 못 찾으면 생성 실패). 제약은 전부 `grade_mirror, gauge_lr2, ln`. 트로피는 ☆ 10개(初段~十段) bronze(7.5, 0.0)·silver(5.0, 70.0)·gold(2.5, 85.0), ★ 12개(発狂 10단·皆伝·Overjoy) silver·gold. DP 22개는 원천이 없어 같은 단 SP 코스의 제약·트로피를 쓴다.
- 차트 제목(표시용, id 에 들어가지 않음): SP 는 bms-ir 코스 페이지의 곡 제목, DP 88개(22코스 x 4곡)도 같은 출처에서 받아 넣었다(ea5829d). 데이터 파일의 모든 차트에 제목이 있다.

### 3) 코스 id 규칙은 그대로 유지한다

- `Course::hash()` = SHA-256( 이름 UTF-8 + `\n` + 차트마다 (sha256 이 있으면 sha256, 없으면 md5 소문자) + `\n` ). 웹 `courseHashOf` 가 같은 규칙이다. 이름이 들어가므로 LR2IR(접두사 + md5)·beatoraja(차트 + 제약)와 달리 이름이 다르면 다른 코스다.
- 라이브러리에서 찾은 sha256 을 코스 정의에 써 넣지 않는다(id 가 바뀐다). 데이터 파일은 `course_hash` 를 미리 적어 두고, Rust 시험이 계산값과 같은지, 웹 등록 때 `bundledCatalog` 가 이름·차트로 다시 계산한 값과 같은지 확인한다. 다르면 등록이 거절된다.
- 예: `GENOSIDE 2018 段位認定 初段` + 4 md5 = `ae8ddca1ddec7737a393eddbd3862d2be2e42bb7184365fbc4664efe280ed0d1`.
- 표 header 의 같은 md5 열 코스는 이름이 달라도(☆ 쪽 `GENOSIDE 2018 初段`) `bundled_twin` 으로 번들 코스로 바꿔 한 id 로 모은다(클라이언트 표 코스 읽기: `tablesrc.rs` `courses_listed_by`).

### 4) 생성기

- `rbms-cli course-snapshot --zip <파일> --normal-header <파일> --insane-header <파일> --titles <md5→제목 JSON> --retrieved-on YYYY-MM-DD --out <파일> [--check]`. 네트워크를 쓰지 않고 유지보수자가 받은 입력으로 같은 바이트를 만든다. `--check` 는 커밋된 파일이 입력이 주는 값과 같은지 본다(같으면 0, 다르면 1, 입력 오류 2). `.lr2crs` 는 `<course>` 블록마다 `title`, `line`, `hash`, `type` 네 태그만 읽는 작은 파서이고 Shift_JIS 는 `encoding_rs`, 표 header 앞의 BOM 은 벗긴다.
- ZIP 읽기에 결정 15 가 승인한 `zip` 크레이트를 쓴다. 워크스페이스 줄은 `zip = { version = "9.0.0", default-features = false, features = ["deflate-flate2-zlib-rs"] }` 이고 `Cargo.lock` 은 9.0.0 이다. 구현과 다름: 설계서는 9.0.5 로 고정한다고 적었다.

### 5) 클라이언트 표시 위치

- COURSE 탭: 번들 44개(SP 22 → DP 22)가 사용자 코스 파일 앞에 보인다. 같은 `hash()` 가 두 번 나오면 번들만 남는다. 곡이 없으면 `N STAGES - M MISSING` 으로 보이고 시작할 수 없다.
- 선곡 트리: 기본 발광 표(`INSANE_TABLE_ID`)의 폴더 안 `COURSE` 폴더에 header 의 코스 뒤로 번들 44개가 붙는다. 다른 표에는 붙지 않는다. 막대의 제약 배지는 선언된 제약 그대로다(`LR2 GAUGE` 로 보여도 실제 표는 COURSE GAUGE 설정).
- DP(14키) 코스는 곡이 있으면 지금 코스 경로로 그대로 친다. DP MIRROR 에는 FLIP 이 없다([코스 시작 슬롯 ADR](2026-10-23-adr-course-stage-start-and-ln.md)).

### 6) 웹 등록과 출처 표기 (결정 14)

- 마이그레이션 0012(운영 적용 web-db run 39909099575)의 `course_source`(kind `bundled`, id `genoside-2018`), `course_group`, `course_group_member`, `course_chart`(md5·제목 열 추가), `dan_course`. 등록은 `web-db.yml` 의 `courses-seed-count` → `courses-seed`(`web/scripts/seed-courses.ts`, 세기 먼저, 적용 뒤 계획을 다시 세어 남은 변경이 0 이어야 성공).
- 코스 목록 화면 `/courses`(`CourseSourceNotice`): 출처 이름, 제작 주체, 공식 코스 파일 이름, 받은 날, SHA-256 앞 12자, 공식 페이지 링크, 제약·트로피를 읽은 표 header 링크와 받은 날, DP 코스에는 "제약·트로피는 같은 단의 SP 값입니다(DP 값의 원천이 없습니다)" 안내.
- 운영 상태(2026-10-23 확인 시점): `courses-seed-count` run 39938376095 가 SP 22·DP 22 inserted 로 끝났고, `courses-seed` 는 아직 실행하지 않았다(사용자 `!` 실행 대기).

## 이유

- 공식 코스 파일이 같은 정의(이름·md5 열)를 제작자에게서 직접 주고, SP 22개의 md5 열이 beatoraja 용 header 코스와 22/22 일치해 정확성을 두 출처로 확인할 수 있다. bms-ir 은 제작자 파일이 아니고 약관이 있어 표시용 제목 보조 출처로만 썼다(메인 판단).
- id 규칙을 바꾸지 않으면 클라이언트 보관 본문·서버 행·재전송에 이전이 필요 없다.
- 데이터 파일을 한 곳에 두면 클라이언트 내장과 웹 등록이 같은 정의를 쓰고 해시로 서로 맞춰진다.

## 기각된 대안

| 대안 | 기각 이유 |
| --- | --- |
| SP 22개만 | 사용자가 DP 포함을 정했다 |
| bms-ir 단위 ZIP(`download.zip?table=genocide2018`)을 원천으로 | 제작자 파일이 아니다. 같은 정의를 공식 파일이 직접 준다(메인 판단) |
| 코스 id 규칙을 접두사 + md5 로 변경 | 이전이 필요하고 이름이 다른 코스의 순위 묶음이 바뀐다 |
| ☆ 쪽 이름으로 통일 | 이름 포함 id 규칙에서는 다른 코스가 되므로 `bundled_twin` 으로 처리 |
| COURSE 탭만 두고 선곡 트리는 변경 없음 | 표 안 코스 폴더(결정 11)와 GENOSIDE 가 만나는 자리가 비었다(교차 검토서 R-3). 둘 다 두었다 |
| 데이터 마이그레이션 SQL 로 등록 | 파일을 그대로 읽어 해시를 다시 확인하고 세기를 먼저 보이는 스크립트가 운영 DB ADR 의 흐름에 맞는다 |
| DP 제약·트로피를 비워 둠 | 같은 단 SP 값을 쓰고 화면에 안내한다(설계서 F5 결정 5 의 추천) |

## 영향과 관련 파일

- 확인하지 못한 사실: `<type>2` 와 해시 접두사의 의미. DP 22코스의 곡 md5 를 코스 페이지와 개별로 대조했는지(조사 시점에는 해시 목록 대조만 했다). DP 제목 88개를 받은 실행 주체와 요청 간격(설계서는 1초 이상 간격을 권했다). 코스 파일·표 JSON 재배포를 허락하거나 금지하는 문구는 찾지 못했다.
- 서버는 정의(md5·이름·제약·트로피·제목)와 출처 행만 보관하고 곡 파일은 없다. 요청이 오면 출처 행 단위로 지울 수 있게 모았다.
- 코드: `apps/rbms-cli/src/course_snapshot*`, `crates/rbms-course/src/{bundled,table}.rs`, `apps/rbms-player/src/{course_ui,tablesrc,select_tree}.rs`, `web/data/courses/genoside-2018/courses.json`, `web/src/server/service/domain/course-catalog/*`, `web/scripts/seed-courses.ts`, `web/src/features/course/course-source-notice.tsx`, `.github/workflows/web-db.yml`.
- 시험: `crates/rbms-course/src/bundled/tests.rs`, `table/tests.rs`, `apps/rbms-cli/src/course_snapshot/tests.rs`, `stage/select/tree_tests.rs` 의 `the_insane_table_course_folder_lists_the_bundled_genoside_courses`, `web/tests/service/{course-hash,course-catalog,dan-of-course}.test.ts`, `web/tests/scripts/seed-courses.test.ts`.

## 날짜와 사용자 원문

2026-10-23.

- 요구 8: "코스모드를 구현해야해. 발광단위인정 등을 구현하고싶거든. https://www.bms-ir.org/new/dan?table=genocide2018 여기에 발광 제노사이드 표가 있는데 이것들로 들어가서 정확하게 모든 단위를 가져오고 서버에 등록해놔, 이것도 출처등록을 해두면 좋을거고 해당 단위를 획득하면 내정보에 단위를 넣도록하고, 다른 코스들도 생성하게 만들고." (뒤 문장은 [코스 게이지 ADR](2026-10-23-adr-course-gauge-modes.md), [단위 ADR](2026-10-23-adr-dan-auto-award.md) 에 있다.)
- 결정 18: "DP도 포함" (`docs/PROCESS.md` 에 기록된 원문).
- 결정 14·15·16 및 메인 결정: "13 - 16 추천대로", "나머지도 추천대로" (14 는 "Stella·단위는 출처 표기, 발광 insane1 은 흔적 제거 유지" 로 요약돼 있다).
