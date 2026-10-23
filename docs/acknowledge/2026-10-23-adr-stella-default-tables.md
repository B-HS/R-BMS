# ADR: Stella·Satellite 를 기본 표로 (원본 주소에서 직접, 내장 사본 없음, 하루 지난 사본은 다시 받음)

> 최종 갱신 2026-10-23 · 대응 커밋 5a0eacf
> 상태: 유효(결정 12·13 의 클라이언트 쪽은 사용자 결정, 넣는 자리·내장 여부·다시 받기 범위와 알림은 메인 결정이라 사용자 미확인) · 결정일 2026-10-23 · 결정자 사용자(결정 12·13) + 메인(세부) · 커밋 `2787264`(기본 표 등록·묶음), `519d7d6`(하루 지난 사본 다시 받기), `fc23232`(레벨 제목의 symbol)
> 관계: [발광 기본 표 ADR](2026-10-14-adr-default-insane-table.md) 의 "한 번만 등록(`seeded_default_tables`)" 방식을 그대로 쓰되, 그 ADR 이 정한 "스냅샷을 실행 파일에 내장"은 이 표들에 적용하지 않는다. 웹 쪽(Vercel Cron 이 원본을 받아 운영 DB 에 저장, 표 페이지, 출처 표기)은 이 ADR 의 범위 밖이다. 폴더가 어떻게 보이는지는 [폴더 스택 ADR](2026-10-23-adr-select-folder-stack.md).

## 배경

- 요구 4·6(아래 원문): Stella 난이도표를 넣을 것이고, Stella 는 내 서버가 아니라 출처가 명확하므로 Stella 에서 항상 가져온다. 이 표는 계속 갱신되므로 최신화한다.
- 사용자 결정 12: Stella(`st`)와 Satellite(`sl`) 둘 다. 결정 13: 웹은 Vercel Cron(UTC 15:00)이 원본을 받아 저장하고, 클라이언트도 원본을 직접 받고 하루가 지나면 다시 받는다.
- 이 앱은 표를 한 번 받아 사본(`TableStore`)으로 두고 이후 실행은 그 사본을 읽을 뿐 네트워크를 묻지 않았다. 발광 표는 실행 파일에 내장한 스냅샷을 첫 실행에 심었다.
- 원본은 정식 표를 2주 간격으로 반영한다고 밝힌다(조사 보고 기준). 응답에 ETag·Last-Modified 가 없어 조건부 요청은 쓸 수 없다.

## 결정

1. **기본 표 둘.** `DEFAULT_TABLES` 에 `stella-st`(`https://stellabms.xyz/st/table.html`)와 `stella-sl`(`https://stellabms.xyz/sl/table.html`)을 더한다. 우리 서버의 사본이 아니라 원본 주소다. 이 id 와 주소는 첫 릴리스 뒤에 바꾸지 않는다(사본 이름이 주소를 따른다). `same_as`(`stellabms.xyz/st`·`/st/score.json`, `/sl`·`/sl/score.json`)로 이미 같은 표를 다른 주소로 등록한 사용자에게는 두 번째 사본을 주지 않고 id 만 기록한다.
2. **묶음.** 두 표는 `TableSource.group = "STELLA"` 로 등록되어 루트에서 `STELLA (n)` 막대 하나 안에 `STELLA`·`SATELLITE`(표가 스스로 부르는 이름의 대문자) 둘로 들어간다. 경로는 `STELLA > STELLA > st3` 이다. 레벨 폴더는 헤더 symbol 을 붙여 `st0`~`st12`, `sl0`~`sl12` 로 부르고, 레벨 순서는 `level_order` 가 없어 숫자 순이다.
3. **넣는 자리(M3).** 새 기본 표는 `DEFAULT_TABLES` 순서상 바로 앞 기본 표 뒤에 들어가고, 앞 기본 표가 목록에 없으면 맨 앞에 들어간다. 새 사용자는 `[발광, st, sl]` 이고, 발광을 이미 받은 사용자도 `[발광, st, sl, 사용자 표…]` 가 된다. 지금까지의 "맨 앞" 규칙이면 올린 사용자는 `[st, sl, 발광…]` 이 되어 사용자마다 순서가 달라진다. 한 번 등록하면 `seeded_default_tables` 에 id 가 남아 사용자가 지운 표는 다시 들어오지 않는다.
4. **내장 스냅샷은 두지 않는다(M4).** `SNAPSHOTS` 는 발광 하나 그대로다. 첫 실행에 오프라인이면 Stella 폴더는 첫 받기가 끝날 때까지 없고 표 관리 화면에는 아직 받지 않음으로 보인다. 내장하지 않는 표는 경고를 내지 않는다(`Carried::Absent` 와, 내장했는데 읽히지 않는 `Carried::Unreadable` 만 경고).
5. **하루 지난 사본은 다시 받는다.** 실행마다 한 번, 라이브러리를 읽은 뒤 선곡이 처음 서는 때(`start_first_fetches`), 주소로 등록된 표 가운데 사본이 하루(86,400초) 이상 됐거나 받은 시각을 모르는 사본(실행 파일에 내장한 사본이 그렇다)을 백그라운드로 다시 받는다(`FetchWhy::Stale`). 파일로 등록한 표는 사본이 없어 대상이 아니다. 시계가 되돌려져 받은 시각이 지금보다 뒤인 사본은 낡은 것으로 보지 않는다. 새 사본이 도착하면 열린 폴더와 커서를 지킨 채 목록이 바뀐다. 받는 일은 워커 스레드에서 하고 적용은 선곡과 표 관리 화면에서만 해서 플레이 중 프레임 스레드에는 I/O 와 네트워크가 없다([렉 기준 ADR](2026-10-14-adr-play-hitch-criteria.md) 1항).
6. **알림 없음(M6).** 다시 받기는 성공도 실패도 알림을 내지 않는다. 실패하면 사본이 그대로 쓰이고 이유는 표 관리 화면의 해당 표 옆(`stale`)에만 보인다. 첫 받기·사용자가 요청한 다시 받기의 알림은 그대로다.
7. **범위(M5).** 주소로 등록된 표 전부이고 기본 표만이 아니다.

### 메인 결정(설계서 select-tree-download 8절, 사용자 미확인)

| id | 결정 | 이유 |
| --- | --- | --- |
| M3 | 새 기본 표는 앞선 기본 표 바로 뒤 | 새·기존 사용자의 순서가 같아진다 |
| M4 | Stella 내장 스냅샷을 두지 않음 | 결정 12 "항상 가져오고", 2주마다 바뀌는 데이터를 실행 파일에 박으면 낡는다, 저장소에 Stella 데이터의 사본을 두지 않는다 |
| M5 | 다시 받기는 주소 표 전부(파일 표 제외) | 사용자가 등록한 표도 하루 한 번 최신으로 |
| M6 | 다시 받기 알림 없음 | 저장본이 그대로 쓰이므로 매일 알림은 소음이다 |
| M15 | 묶음 이름 `STELLA` | 과제 문구 |

## 이유

- 사용자 결정 12·13 과 요구 6 의 "항상 원본에서". 내장 사본을 두면 첫 실행에 원본보다 낡은 데이터가 보이고 저장소에 Stella 의 데이터가 사본으로 남는다.
- 사본이 하루 지나면 다시 받으면 원본의 반영(2주 간격)이 며칠 안에 클라이언트에 닿고, 사용자당 하루 한 번 표 둘(약 0.8MB + 0.9MB, 조사 보고의 크기)로 서버 부담이 작다. 오프라인이거나 원본이 점검 중이면 사본으로 계속 쓴다.
- 앞선 기본 표 뒤에 넣으면 사용자 사이 순서가 같다.

## 기각된 대안

| 대안 | 기각 이유 |
| --- | --- |
| Stella 스냅샷을 실행 파일에 내장 | 오프라인 첫 실행에도 폴더가 보이는 이점이 있으나 위 M4 의 이유 셋. 실행 파일이 약 1.7MB 커진다(설계서의 추정) |
| 우리 서버의 사본을 기본 주소로 | 결정 12 와 요구 6 이 원본 주소를 요구했다 |
| 맨 앞에 넣는 기존 규칙 | 올린 사용자는 STELLA 가 발광보다 앞에 온다(M3) |
| 기본 표만 다시 받음 / 알림은 실패만 경고 | 사용자 표도 최신이 낫고, 매일 경고는 소음이다(M5·M6) |
| 정식 표 외에 추천·서브 표도 기본으로 | 요구는 정식 표 `st`·`sl` 이다. 서브(3,247곡)는 후속 |

## 영향과 관련 파일

- `crates/rbms-config/src/tables.rs`(`DEFAULT_TABLES`, `STELLA_TABLE_ID`, `SATELLITE_TABLE_ID`, `STELLA_GROUP`, `DefaultTable.group`, `seed_default_tables`의 자리 규칙), `crates/rbms-config/src/schema.rs`(`TableSource.group`), `apps/rbms-player/src/tablesrc/seed.rs`(`Carried`), `apps/rbms-player/src/tablesrc.rs`(`COPY_STALE_AFTER_SECS`, `is_copy_stale`, `start_stale_fetches`, `FetchWhy::Stale`).
- 테스트: `seed.rs`(`stella_is_not_carried_and_waits_for_its_first_fetch`), `tablesrc.rs`(`a_copy_older_than_a_day_is_fetched_again_once_a_run`, `a_copy_younger_than_a_day_is_left_alone`, `a_carried_copy_that_has_no_date_is_fetched_again`, `a_refresh_that_fails_keeps_the_copy_and_says_nothing`, `a_table_file_is_never_fetched_again_for_its_age`, `a_refresh_that_lands_keeps_the_browser_where_it_is`), `crates/rbms-config/src/tables.rs` 의 자리 규칙 시험.
- 설정 파일: 스키마 버전 6 은 그대로이고 `seeded_default_tables` 에 두 id 가 더해진다. 옛 빌드는 모르는 필드를 무시한다. `group` 은 빈 값도 `group:""` 로 쓴다(설계서는 빈 값 미기록이라 적었다. 구현과 다름).
- 이어지는 결과: 내장한 발광 사본은 받은 시각이 없어, 첫 실행 뒤 선곡이 처음 설 때 한 번 주소(R-BMS 사이트의 사본)에서 다시 받는다.
- 설계서와 다른 점은 위 설정 파일 줄 하나다. 자리 규칙·내장 여부·다시 받기는 `docs/plan/2026-10-23-select-tree-download.md` 3.3·3.4절과 같다.
- 들어감: 수리 커밋 `45d09fd` 가 표 받기·적용의 경쟁을 고쳤다. 표 사본이 놓이거나 빠지거나 바뀔 때마다 표 세대(`AppShared.table_generation`)가 오르고(`tablesrc.rs` 의 `set_table_slot`·`set_tables`·표 제거), 그 전에 시작한 부분 스캔이 맞춘 옛 표 사본은 선곡 목록을 덮어쓰지 않고 그 스캔을 다시 돌린다. 이 ADR 이 적은 자리 규칙·내장 여부·다시 받기는 바뀌지 않았다.
- 확인하지 못한 것: 원본 서버의 실제 응답을 이 작업에서 다시 받아 보지 않았다(크기·ETag 없음·2주 간격은 조사 보고의 관찰값). 사이트 점검 시간과 겹치는 날은 다시 받기가 실패하고 다음 실행에 다시 시도한다. 실제 네트워크로 Stella 를 받아 본 실기는 하지 않았다.

## 날짜와 사용자 원문

2026-10-23.

- 요구 4(원문): "추가적인 난이도표를 넣을거야 https://stellabms.xyz/st/table.html 이건데 stella 난이도표야. 그러면 폴더의 폴더를 들어가는 재귀적 폴더가 필요하겠지"
- 요구 6(원문): "stellabms 는 내서버가 아니라 출전가 명확하므로 stella bms 발광표에서 항상 가져오고. 이건 업데이트가 진행되는 난이도표니 한국시간 매일 새벽 0시에 업데이트 하도록 cron 도 vercel 에 workflow 나 다른것들을 이용해서 최신화 하게 만들어"
- 결정 회신(원문): "12. st, sl 둘 다 구현", "13 - 16 추천대로". 결정 13 의 요지(`docs/PROCESS.md` 의 요약, 원문 아님): Vercel Cron(UTC 15:00)이 원본을 받아 운영 DB 에 저장, 클라이언트도 원본 직접·하루 지나면 다시 받음.
