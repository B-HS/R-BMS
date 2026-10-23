# 2026-10-23 설계 교차 검토 — F2·F3/F4·F5·F6·F7 다섯 설계서

> 최종 갱신 2026-10-23 · 대응 커밋 1169efb(dev) · 상태: 검토(구현 전). PROCESS "Stella·코스·단위와 클라이언트 보완 (2026-10-23)" F1 의 마지막 단계.
> 검토 대상(이하 약칭): [client-small](2026-10-23-client-small.md)(F2), [select-tree-download](2026-10-23-select-tree-download.md)(F3·F4), [course-gauge-data](2026-10-23-course-gauge-data.md)(F5), [web-course-dan-tables](2026-10-23-web-course-dan-tables.md)(F6), [song-mirror-prep](2026-10-23-song-mirror-prep.md)(F7).
> 이 문서는 설계서를 고치지 않는다. 고쳐야 할 곳은 "정정 지시" 로 적었고, 메인이 구현 지시에 반영하거나 해당 설계서에 합친다.
> 실제 코드로 다시 확인한 것은 9절에 모았다. 확인에 쓴 명령은 읽기만 했다(grep·sed·crates.io API 조회). 코드·빌드·git 상태는 바꾸지 않았다.

---

## 0. 요약

1. 결정 1~21 중 **위반은 없다.** 누락은 둘: GENOSIDE 2018 을 선곡 트리에 두는 자리(F5 추천 7 을 F3 가 구현하지 않음), 결정 질문 원문(추천안 문구)이 저장소에 없어 "추천대로" 의 내용을 문서끼리 대조할 수 없는 점(PROCESS F0 미완).
2. 인터페이스 불일치 9건. 컴파일·CI 를 깨는 것은 셋: 표 코스 변환 함수 이름·반환형(F3 ↔ F5), `TableSource` 리터럴 46곳이 F3 U1 소유 밖에 있음, `settings-blob.json` 을 세 묶음이 따로 재생성함(한 줄짜리 JSON 문자열이라 git 이 합치지 못한다).
3. 웨이브 안은 7웨이브(3절). 병렬 단위는 모두 `rbms-player` 를 건드리므로 **병렬 단위마다 worktree 가 필요**하다. 웹은 Rust 와 독립이고, 메인이 지켜야 할 직렬 제약은 "F5 C4 의 계약 골든 푸시는 F6 W2 의 DTO 배포와 같거나 뒤", "클라이언트 릴리스는 웹 배포 뒤" 둘이다.
4. 판정·점수 의미가 지금 R-BMS 와 달라지는 항목 중 사용자가 직접 정하지 않은 것은 F5 D4(코스 시작 슬롯이 GAUGE 설정을 따름)와 D8(LN 제약 강제)이다. 조사 추천안의 "이미 그렇다" 는 전제가 틀렸으므로(F5 2.9절) **사용자 확인을 권한다**(4절).
5. 메인 결정 필요 항목은 다섯 문서 합계 89개(F2 17·F3 20·F5 16·F6 26·F7 10)와 F7 의 사용자 질문 8개다(6절). 검토자가 추천안을 바꾸자고 하는 것은 7개다(6.1절).

---

## 1. 사용자 결정 1~21 대응

"다루는 곳" 은 설계서 약칭과 절. "판정" 은 검토 결과.

| 결정 | 요지(PROCESS 요약) | 다루는 곳 | 판정 |
| --- | --- | --- | --- |
| 1 | 패널 키 배치·확장 id 지금대로 | 변경 없음. F2 3.3 은 확장 표시가 있는 스킨(기본 스킨)에서 지금 표를 그대로 둔다 | 지킴 |
| 2 | 9키 행 이름 공유 지금대로 | 변경 없음(어느 설계도 손대지 않음) | 지킴 |
| 3 | 등급 이름·색·ASSIST EASY 게이지 = 등급 0·웹 순위 키 지금대로 | F6 3.3 이 `deriveAssistGrade`·`ASSIST_GRADE_OF_GAUGE` 를 그대로 재사용. F5 3.1 의 IIDX 세트는 슬롯 0~5 를 7키 값으로 채워 등급 표를 건드리지 않음 | 지킴 |
| 4 | Inter Bold 유지 | 변경 없음 | 지킴. 단 F3 M2 의 `★12` 레벨 제목이 Inter 에 없는 글리프일 수 있음(7절 위험 T-9) |
| 5 | 스킨 헤더의 확장 지원 표시가 있을 때만 재배정 키 | F2 3.3(U-C) | 지킴 |
| 6 | 코스 제출에 게이지 종류 → `course_best.assist_grade` | F5 3.4(C4), F6 3.3(W2) | 지킴. 와이어 이름·타입 일치(2절 표 6행) |
| 7 | v0.1.5 draft 그대로 | 해당 없음 | 지킴(어느 설계도 릴리스를 건드리지 않음) |
| 8 | SCRATCH AUTO 색 = PLAY 전역 RGBA, 외부 스킨 포함, 없으면 `#30E050` | F2 3.1(U-A) | 지킴. 저장값 없을 때 화소 동일을 비트 단위로 보인다(F2 3.1.2) |
| 9 | 친 판 Esc → 결과 화면, 코스 Esc → FAILED 코스 결과, 판정 수 = 저장값 | F2 3.2(U-B) | 지킴. 판정 분포 그래프는 입력 기준이라 남은 노트를 넣지 않는다(F2 9절 3, 기존 의미) |
| 10 | 폴더 스택(BarManager)으로 커서 복원 | F3 3.1(U1) | 지킴 |
| 11 | 임의 깊이 = 표 묶음 > 표 > 레벨, 표 코스 폴더 | F3 3.1·3.2(U1·U5) | 지킴. **누락 하나**: GENOSIDE 2018(번들 코스)의 트리 자리. 발광 표 header 에 `course` 가 없어(9절 확인) F3 U5 의 표 코스 폴더로는 나오지 않는다. F5 8절 추천 7 은 "발광 표 아래 코스 폴더(F3)" 인데 F3 에 구현 단위가 없다 → 6.1절 R-3 |
| 12 | Stella st·Satellite sl 둘 다 | F3 3.3(U2), F6 3.7(W7), F5 3.6(표 코스 변환) | 지킴 |
| 13 | Vercel Cron UTC 15:00 → 운영 DB, 클라이언트도 원본 직접·하루 지나면 다시 | F6 3.6(W5, `0 15 * * *`), F3 3.4(U3) | 지킴 |
| 14 | Stella·단위는 출처 표기, insane1 흔적 제거 유지 | F6 3.4·3.7(W3·W7·W8), F3 3.3-7(클라이언트 표 관리 화면은 등록 주소) | 지킴. insane1 `source: null` 유지(F6 3.6 표 읽기 7) |
| 15 | 직접 zip 만 앱이 받아 풀기(`zip` 승인), rar·7z·HTML 은 브라우저, 모달 안내 | F3 3.7~3.10(D1~D3), F7 5.5·5.8(같은 크레이트, 모달 자리) | 지킴. F7 은 같은 크레이트를 묶음 **쓰기**에도 쓴다 → 승인 범위 확인 필요(F7 D-1) |
| 16 | 표 레벨 안 흐린 없는 곡 막대 + 전용 다운로드 폴더(곡 폴더 목록 자동 포함) | F3 3.5·3.6(U4·U6) | 지킴 |
| 17 | 곡 미러는 준비만, 조사 기록은 맨 마지막에 보고 | F7 전부(U1 만 구현, M1~M4 설계만, 13절 보고 초안) | 지킴. 메인은 G 단계 마지막 응답에 F7 13절을 붙여야 한다 |
| 18 | GENOSIDE 2018 SP 22 + DP 22 + Stella·Satellite 표 코스 | F5 3.5·3.6·3.8(C5·C6), F6 3.4(W3), F3 3.2(U5) | 지킴. DP 제약·트로피는 원천 없음 → 같은 단 SP 값(F5 결정 5, 메인 결정) |
| 19 | 서버 자동 판정 → `user_dan`, 마이페이지에서 표시 단위 선택 | F6 3.5·3.7(W4·W8) | 지킴 |
| 20 | 단위 = 게이지 남은 채 끝냄, 미러·LN 제약 준수, 어시스트·SCRATCH AUTO 제외 | F6 3.5 판정 12단계(W4), F5 3.3(2)-3(LN 제약을 실제로 적용, C3)·3.4(스테이지 사실 전송, C4) | 지킴. F6 이 근거로 든 "결정 20 '모두 인정하되 기록'"(게이지 모드 무관 인정)은 PROCESS 요약에 없는 문구다. 질문 원문이 저장소에 없어 확인 못함 → 메인이 대화 원문으로 확인(6.1절 R-7) |
| 21 | 코스 게이지 LR2(기본)/BEATORAJA/IIDX, 설정이 `gauge_lr2` 보다 우선, IIDX 실패선 0% | F5 3.1~3.3(C1~C3) | 지킴. "나머지도 추천대로" 로 채택된 시작 슬롯 대응(D4)은 조사 보고의 틀린 전제("이미 있음") 위에 있었다(4절) |

---

## 2. 문서 사이 인터페이스 불일치

| # | 대상 | 불일치 | 영향 | 정정 지시(추천) |
| --- | --- | --- | --- | --- |
| I-1 | 표 header `course` → `Course` 변환 | F3 3.2-1 은 `rbms_course::table::courses_of_table(&Value) -> Vec<Course>` 를 가정. F5 3.6 은 `courses_from_table_header(&Value) -> TableCourses { courses, skipped }` 로 확정 | U5 컴파일 실패 | **F5 이름을 정본으로**. U5 는 `courses_from_table_header(v).courses` 를 쓰고 `skipped` 는 표 관리 화면 알림 없이 버린다. F3 의 `SelectFolder::TableCourses` 변종과 F5 의 `struct TableCourses` 이름이 같으므로 U5 는 `use rbms_course::table::TableCourses as HeaderCourses` 로 들인다(이름 바꾸기는 F5 소유라 하지 않음) |
| I-2 | 번들 코스와 표 코스의 같은 곡 열 | F5 3.6-5·결정 8 은 호출 쪽(F3)에 `bundled_twin` 사용을 권함. F3 는 언급 없음 | id 가 갈려 순위가 둘로(사용자가 miraiscarlet 표를 등록한 경우) | U5 의 `CourseEntry` 생성 직전에 `bundled_twin(&course).cloned().unwrap_or(course)` 한 줄(6.1절 R-4) |
| I-3 | 코스 막대 정체(커서 복원 열쇠) | F3 `BarIdentity::Course(String)` = 코스 이름. F5·F6 의 코스 정체는 `Course::hash()` | 같은 이름의 코스(번들 SP 와 표 코스 이름이 같은 경우 등)에서 커서가 엉뚱한 막대로 | `BarIdentity::Course(course_hash)` 로(6.1절 R-5) |
| I-4 | `zip` 크레이트 의존 트리 | F3 3.8-1 은 새 크레이트 `zip`·`typed-path 0.12.4`. F5 3.5 는 `typed-path(0.13.x)`·"경우에 따라 `libz-rs-sys`". F7 은 버전 미기재 | 문서 기록 오류(동작 영향 없음) | crates.io 재확인(9절): zip 9.0.5 의 요구는 `typed-path ^0.12` → **0.12.4**(0.13.0 은 호환 범위 밖). Cargo.lock 의 flate2 1.1.10 은 `zlib-rs` 를 직접 쓰고 `libz-rs-sys` 는 생기지 않는다. typed-path 0.12.4 는 의존이 없다. F5 3.5 의 그 문장을 F3 값으로 정정 |
| I-5 | `zip` 워크스페이스 줄을 누가 만드나 | F3 D1 이 소유(루트 `Cargo.toml`·`Cargo.lock`), F5 C5 는 "먼저인 쪽", F7 U1 은 "F4 줄 그대로" | 동시 수정 시 `Cargo.lock` 충돌 | **F3 D1 이 만든다**. C5·F7 U1 은 D1 병합 뒤 `{ workspace = true }` 만(3절 웨이브 1·2) |
| I-6 | 코스 제출 필드 | F5 `gauge: Option<GaugeType>`("Class" 등), `course_gauge: Option<CourseGaugeType>`(`rename_all = "UPPERCASE"` → `"LR2"`·`"BEATORAJA"`·`"IIDX"`), `stages: Vec<{gauge, random, lntype: i32}>`. F6 `gaugeTypeSchema.nullish().default(null)`, `z.enum(['LR2','BEATORAJA','IIDX'])`, `stages max(COURSE_CHARTS_MAX)`, `lntype z.number().int()` | **일치**. `stages[i].random` 은 코스 클램프 뒤 값이다(`apply_to` 가 `config.play.random` 을 좁힌 상태에서 `ir_random(shared.config.play.random)`, 9절) | 없음. 테스트 벡터는 F5 부록 A·F6 2.7 의 세 해시를 양쪽 테스트가 같은 값으로 쓴다 |
| I-7 | 코스 id 규칙 | F5 3.7 = F6 2.7·3.4 `courseHashOf`(이름 원문 + sha256 있으면 sha256, 없으면 md5) | 일치. 위험은 TS 쪽 이름 정규화(trim 등) | F6 W3 `courseHashOf` 는 이름을 **그대로**(trim·NFC 없이) 쓴다고 명시하고, `bundledCatalog` 의 재계산 대조(던짐)를 유지 |
| I-8 | 설정 스키마 직렬화 방식 | F2 M8-5 "rbms-config 에 `skip_serializing_if` 선례 없음 → 쓰지 않음". F3 3.3·3.6 은 `TableSource.group`·`download_folder` 에 `skip_serializing_if` 를 새로 씀. F5 `course_gauge` 는 없음 | 같은 크레이트에 두 방식 | **선례대로 `skip_serializing_if` 를 쓰지 않는다**(`songs_folder: None` 이 이미 그렇게 적힌다, 9절). 6.1절 R-1 |
| I-9 | `settings-blob.json` 영향 | F2 U-A(`scratch_auto_color: None,`), F5 C2(`course_gauge: "LR2",`)는 재생성을 계획. **F3 6.3-1 은 "영향 없음" 이라 적었으나 틀렸다**: 업로드 묶음은 `library` 절을 통째로 싣고(`sanitise_for_upload` = `keep_local(&Config::default(), …)`, 9절) `show_missing_charts: true,` 가 생긴다(I-8 을 따르면 `download_folder: None,` 도) | `a_settings_upload` 계약 테스트 실패 | U6 소유 파일에 `contract/client-requests/settings-blob.json` 을 더한다. 이 파일은 `content` 가 한 줄 문자열이라 **어느 단위도 병합하지 않고, 각 웨이브 통합 때 메인이 한 번 `RBMS_UPDATE_CONTRACT=1 cargo test --locked -p rbms-player --lib a_settings_upload` 로 다시 만든다**. 웹 `client-requests.test.ts` 는 `content` 를 문자열로만 다뤄 영향 없음 |
| I-10 | `SelectItem` 변종 | F3 U1 `Folder{label, target: SelectFolder}`, U4 `Missing(MissingRef)`, U5 `Course(CourseRef{table, course})`. 다른 묶음은 `SelectItem` 을 만들지 않음 | 일치 | 없음 |
| I-11 | 다운로드 폴더 | F3 `library.download_folder: Option<String>`(기기 고유, `keep_local` 제외), F7 M3 는 "결정 16 의 다운로드 폴더" 를 그대로 씀 | 일치 | 없음 |
| I-12 | 모달·받기 작업 계약(F7 U2) | F7 5.8 C1(선택지는 순서 있는 목록), C2(미러 선택지는 지금 넣지 않음), C3(받기 입력에 기대 바이트·sha256·최상위 폴더), C4(해제 상한 ≥ 묶음 상한). F3 3.7 `ModalButton` 고정 enum + `buttons() -> Vec<(ModalButton, bool)>`(C1 충족), `start_download(shared, entry: &TableEntry)`(C3 미충족) | M3 때 시그니처 변경 필요 | C1·C2·C4 는 지금 F4 에 반영, **C3 은 M3 에서**(지금 넣으면 늘 `None` 인 필드 = 죽은 코드, F7 C2 의 금지와 같은 이유). 6.1절 R-6 |
| I-13 | 해제기 위치·상한(F7 D-3) | F7 은 `crates/rbms-library` 의 `archive` 모듈을 추천. F3 는 새 `crates/rbms-download` 에 둔다 | rbms-cli 가 앱 크레이트에 기대지 않는다는 조건은 `rbms-download` 로도 충족 | **F3 위치(`rbms-download`)를 정본으로**, F7 U1 은 `rbms-download` 를 의존해 `EXTRACT_MAX_*` 상수와 `extract_zip` 으로 왕복 시험. 또 F3 의 **항목당 1 GiB 상한·확장자 허용 목록**을 F7 이 반영하지 않았다: 1 GiB 넘는 단일 파일, 허용 목록 밖 확장자(`m4a`·`wma`·`ogv`·`flv`·`tga`·`webp` 등)는 클라이언트가 풀지 않는다 → U1 이 `Refusal::EntryTooLarge` 와 매니페스트 `skipped_by_client` 수를 낸다(7절 T-6) |

---

## 3. 파일 소유권 충돌과 실행 웨이브 안

### 3.1 두 단위 이상이 고치는 파일

"순서" 는 같은 웨이브 안 병합 순서 또는 웨이브 사이 순서다. 소유가 겹쳐도 고치는 줄이 떨어져 있으면 worktree 병렬 + 순서대로 병합으로 둔다.

| 파일 | 고치는 단위 | 처리 |
| --- | --- | --- |
| `apps/rbms-player/src/stage/play/mod.rs` | F2 U-A(`:1186-1196`·`:1698-1707`), F2 U-B(`:690-728`) | 같은 웨이브, U-A → U-B 병합 |
| `apps/rbms-player/src/app_result.rs` | F2 U-B(`show_result`, `leave_course`·`record_left_run` 삭제), F3 U1(시험 리터럴), F3 U4(`following_song`), F5 C3(`fold_course_stage` 이월), F5 C4(`fold_course_stage` 시그니처·`keep_finished_course`) | **U-B → C3 → C4** 직렬(F2 5.4·F5 R8 이 같은 순서). F5 2.4-3 의 "세 호출처" 는 U-B 뒤 둘이 된다(`leave_course` 삭제) → C3·C4 지시에 반영. U1·U4 는 시험·한 갈래라 병합만 |
| `apps/rbms-player/src/app_result/record_tests.rs`, `…/record_tests/contract_requests.rs` | F2 U-B, F5 C3(`mod` 한 줄), F5 C4 | U-B → C3 → C4 |
| `apps/rbms-player/src/lib.rs` | F3 U1·U4·U5·U6(`:1016-1019`)·D2·D3, F5 C3(`start_course`·`course_start_gauge`·`course_facts`), F5 C4(`course_facts` 시작·끝) | F3 단위는 서로 직렬(F3 5절). C3·C4 는 F3 단위와 다른 함수라 병렬 worktree + 병합 |
| `apps/rbms-player/src/app_play.rs` | F2 U-A(`scratch_auto_color`), F3 U1·U4(`print_selection`), F5 C3(`load()`) | 줄이 떨어짐. 병합 순서만 |
| `apps/rbms-player/src/course_ui.rs`(+ `course_ui/tests.rs`) | F5 C3, F5 C6(`CourseList::load`), F3 U5(`CourseEntry::resolved`) | **C3 → C6 → U5** 직렬 |
| `apps/rbms-player/src/stage/select/{skinned.rs, skinned_tests.rs, panel_tests.rs, overlay_tests.rs}` | F2 U-C, F3 U1(그리고 U4·D2 가 skinned.rs) | **U-C 를 U1 보다 한 웨이브 먼저**(F2 5.3·F3 5절이 같은 판단) |
| `apps/rbms-player/src/stage/select/{mod.rs, list.rs, scene.rs, events.rs, overlay.rs}` | F3 U1·U4·U5·D2·D3 | F3 안에서 직렬(U1 → U4 → U5 → D2 → D3) |
| `apps/rbms-player/src/tablesrc.rs` | F3 U1·U3·U4·U5 | 직렬(U1 → U3 → U4 → U5) |
| `apps/rbms-player/src/stage/loading.rs` | F3 U1·U6·D3 | 직렬 |
| `crates/rbms-config/src/schema.rs` | F2 U-A(`PlayOptions`), F3 U1(`TableSource.group`), F3 U6(`LibraryOptions`), F5 C2(`JudgeOptions`) | 구조체가 다르다. 웨이브를 나눠 U-A(1) → C2·U1(2) → U6(3) |
| `crates/rbms-config/src/{settings.rs, lib.rs, tests.rs}`, `apps/rbms-player/src/stage/settings/catalog.rs` | F5 C2, F3 U6(그리고 U2 가 `lib.rs`, F2 U-A 가 `tests.rs`) | C2(웨이브 2) → U6(웨이브 3). F2 5.1 이 적은 "F5 가 PLAY 메뉴에 행을 더하면 겹친다" 는 해당 없음(F5 는 설정 화면 JUDGE 탭에 행을 더하고 `play_menu/*` 는 건드리지 않는다) |
| `TableSource { … }` 리터럴 46곳 / 14파일(9절) | F3 U1 이 필드를 더하면 모두 컴파일 실패. 그중 `crates/rbms-config/src/{tables.rs, tests.rs, tests/library.rs, tests/persistence.rs}`, `apps/rbms-player/src/ir_sync.rs` 는 **U1 소유 목록에 없다**(tables.rs 는 U2, ir_sync.rs 는 U6 소유) | U1 소유에 이 5파일을 "리터럴 한 칸만" 으로 더한다. `TableSource` 에는 `Default` 가 없고 리터럴은 `..` 를 쓰지 않는다 |
| `contract/client-requests/settings-blob.json` | F2 U-A, F5 C2, F3 U6 | 단위는 재생성하지 않고(또는 하더라도 병합하지 않고) 메인이 웨이브마다 한 번 재생성(I-9) |
| `contract/client-requests/course-{normal,scratch-auto}.json` | F5 C4 | 푸시 시점 제약(5절) |
| `crates/rbms-play/src/session.rs` | F2 U-B(`left_record`), F5 C3(`carried_gauges`·`gauge_values`) | 웨이브가 다름(U-B 1, C3 3) |
| 루트 `Cargo.toml`·`Cargo.lock` | F3 D1(members·`rbms-download`·`zip`), F5 C5(rbms-cli 의존), F7 U1(rbms-cli 의존) | D1(1) → C5(2) → F7 U1(3) |
| `apps/rbms-cli/{Cargo.toml, src/main.rs}` | F5 C5(`course-snapshot`), F7 U1(`mirror-pack`) | C5(2) → F7 U1(3) |
| `web/src/server/{service/domain/course/course.service.ts, compose/course.ts}` | F6 W2, W3 | W2 → W3(F6 5.1 과 같음) |
| `web/tests/db/assist-grade.test.ts` | F6 W2, W5 | 같은 웨이브면 W2 → W5 병합 |
| `web/src/server/service/read-cache.ts` | F6 W4, W8 | W4 → W8 |
| `web/src/server/{db/schema.ts, lib/env.ts, compose/index.ts}` | F6 W1·W5·W6, F7 M2(승인 뒤) | F7 M2 는 이번 범위 밖(0013) |
| 문서(`play-options.md`, `reference-divergences.md`, `skin.md`, `crates.md`, `architecture.md`, test-debt, QA 점검표, `HANDOFF.md`, `PROCESS.md`, `acknowledge/README.md`) | 다섯 묶음 전부 | 단위는 문서를 고치지 않고 메인이 G 단계에서 합친다(각 설계서 7절 목록을 그대로 모음). 새 ADR 파일 이름은 서로 겹치지 않는다(13건) |

### 3.2 실행 웨이브

모든 병렬 단위는 Workflow 기본처럼 단위마다 worktree(`.claude/worktrees/wf_*`)에서 돈다. 이유: 거의 모든 Rust 단위가 `rbms-player` 를 고치므로 한 작업 트리에서 동시에 돌면 다른 단위의 반쯤 고친 코드가 서로의 `cargo test` 를 깬다. 웹 단위는 `web/` 만 고치지만 같은 웨이브의 웹 단위끼리도 worktree 로 나눈다(타입 검사가 트리 전체를 본다). 각 웨이브 끝에 메인이 순서대로 병합 → `settings-blob.json` 재생성(필요할 때) → 통합 게이트 한 번 → 논리 단위 커밋.

| 웨이브 | 병렬 단위(worktree 각각) | 병합 순서 | 직렬 의존(앞 웨이브) | 웨이브 끝 |
| --- | --- | --- | --- | --- |
| 1 | F2 U-C, F2 U-A, F2 U-B, F5 C1, F3 D1, F6 W1 | U-C → C1 → D1 → U-A → U-B / W1 | 없음 | Rust 게이트, 커밋. **웹 커밋 A(마이그레이션 파일 3개만) 푸시 → `status` → 사용자 승인 `migrate` → `status`**(5절 1~3) |
| 2 | F3 U1, F5 C2, F5 C5, F6 W2, F6 W5 | C2 → C5 → U1 / W2 → W5 | U1 ← U-C(선택 화면 시험 픽스처), C2 ← C1·U-A(schema.rs), C5 ← D1(zip 줄), W2·W5 ← W1 | Rust 게이트, 커밋. 0012 적용이 끝났으면 **웹 커밋 B1(`schema.ts` + W2) 푸시**(5절 4). W5 는 B2 로 보류(`vercel.json` crons 는 `CRON_SECRET` 등록 뒤) |
| 3 | F5 C3, F3 U2, F3 U3, F3 U6, F7 U1, F6 W3 | U2 → U3 → U6 → C3 → F7 U1 / W3 | C3 ← C1·C2·U-B, U2·U3·U6 ← U1, F7 U1 ← D1·C5(rbms-cli 파일), W3 ← W2·C5(데이터 파일) | Rust 게이트, 커밋 |
| 4 | F5 C4, F5 C6, F3 U4, F6 W4 | C6 → U4 → C4 / W4 | C4 ← C3, C6 ← C3·C5, U4 ← U1·U6, W4 ← W3 | Rust 게이트, 커밋. **C4 는 B1(W2)이 이미 푸시됐을 때만 푸시**, 아니면 로컬 커밋으로 보류(5절 4) |
| 5 | F3 U5, F6 W6 | U5 / W6 | U5 ← U4·C5·C6, W6 ← W3·W4·W5 | Rust 게이트, 커밋 |
| 6 | F3 D2, F6 W7, F6 W8, F6 W9 | D2 / W7 → W8 → W9 | D2 ← U4·D1, W7·W8 ← W6, W9 ← W3·W4·W5 | Rust 게이트, 웹 `bun run verify`·`build`, 커밋. **사용자가 Vercel 에 `CRON_SECRET` 등록 → 웹 커밋 B2(W3~W9) 푸시** → 등록·표 갱신 운영 단계(5절 5~8) |
| 7 | F3 D3 | D3 | D3 ← D2·U6 | 전체 게이트(F8 통합 리뷰·수리 → G) |

메모:

1. 웨이브 1 의 U-A·U-B 는 같은 `stage/play/mod.rs` 를 고치지만 헝크가 수백 줄 떨어져 있다(F2 5.4). 병합 충돌이 나면 U-B 쪽 worktree 에서 다시 맞춘다.
2. F3 U1 은 호출처 수십 곳의 기계적 치환이 있어 한 작업자에게 맡긴다(F3 5절). 웨이브 2 의 가장 긴 단위다. C2·C5 와 파일이 겹치는 곳은 `schema.rs`(다른 구조체)뿐이다.
3. F5 C7·F6 W10·각 묶음 문서 갱신은 G 에서 메인이 한 번에 한다(3.1 마지막 행).
4. F7 M1~M4 는 사용자 답(Q-R1~R5, Q-C1~C3) 뒤의 별도 작업이라 웨이브에 넣지 않는다.

---

## 4. 판정·점수 의미 변경 목록

"기준" 열: 원본(beatoraja → LR2 → IIDX) 대비인지, 지금 R-BMS 대비인지. "확인" 열: 사용자 결정이 직접 다루는지, 확인이 필요한지(HANDOFF 5절 "판정·점수 의미가 원본과 달라지는 수정은 고치기 전에 사용자 확인").

| id | 출처 | 변경 | 기준 | 확인 |
| --- | --- | --- | --- | --- |
| J-1 | F5 D1 | 코스 게이지 표는 COURSE GAUGE 설정이 정하고 `gauge_lr2`·`gauge_5k` 등 제약은 배지만. BEATORAJA 모드에서 발광·Stella 코스가 7키 CLASS 로 돈다 | 원본과 다름 | 결정 21 이 직접 정함(확인됨) |
| J-2 | F5 D2 | 제약 없는 코스(사용자 파일)도 기본 LR2 표 | 원본과 다름, 지금 R-BMS 와도 다름 | 결정 21 "LR2(기본)"(확인됨) |
| J-3 | F5 D3 | IIDX DAN/EX-DAN 수치, 30% **이하**에서 BAD·POOR·빈 POOR 절반, 0% 실패 | IIDX 대비 내부 단위·반올림 미상 | 0% 는 결정 21. "이하"·빈 POOR 절반은 iidx.org 원문(F5 2.7-8)이고 결정 21 요약에 포함된 것으로 F5 가 판단 → 메인이 질문 원문으로 확인 |
| J-4 | F5 D4 | 코스 시작 슬롯이 GAUGE 설정을 따른다(NORMAL 이하 CLASS, HARD EXCLASS, EXHARD 이상 EXHARDCLASS). HARD 설정 사용자의 코스 스테이지가 HARD 램프·등급 3 으로 남는다 | 원본에 맞춰짐, **지금 R-BMS 대비 바뀜** | **사용자 확인 권장**: "추천대로" 로 채택됐으나 추천의 전제("R-BMS 가 이미 이 규칙을 가짐")가 틀렸다(F5 2.9) |
| J-5 | F5 D5·결정 1 | IIDX 모드에서 EXHARD 이상도 EX-DAN(ExClass), 상한 ExClass | 설계 선택 | 메인 결정 |
| J-6 | F5 D6 | 곡 사이 이월을 9슬롯 모두(지금은 선택 게이지 한 값) | 원본에 맞춰짐, GAS 를 켠 코스만 바뀜 | 메인 결정(F5 결정 3) |
| J-7 | F5 D8 | `ln`·`cn`·`hcn` 제약을 실제로 적용(지금 미적용). CN·HCN 설정 사용자가 발광·Stella·GENOSIDE 코스에서 LN 으로 친다 | 원본에 맞춰짐, **지금 R-BMS 대비 바뀜** | 결정 20 "LN 제약 준수" 가 단위 인정 조건을 말할 뿐 플레이 강제까지 정했는지는 원문 확인 필요 → **사용자 확인 권장**(J-4 와 한 질문으로) |
| J-8 | F5 D9·D10 | DP MIRROR 코스에 FLIP 없음, NO GOOD/NO GREAT 미적용 | 원본과 다름 | 기존 차이 유지(대상 코스 0개, 테스트 부채) |
| J-9 | F2 4절 | Esc 판의 남은 노트를 늦은 POOR 로 판정 수에 넣음, 코스 Esc 때 남은 곡 노트를 코스 BP 에 더하지 않음 | 원본과 다름 | 기존 결정 유지(2026-10-14 판정 수 ADR). 새 변경 아님 |
| J-10 | F2 M9-5 | Esc 판 게이지 그래프 꼬리 0 채움 | 원본과 같아짐 | 표시만, 메인 결정 |
| J-11 | F6 E1 | 단위 인정에 게이지 모드 제한 없음(LR2·BEATORAJA·IIDX 모두 같은 단위) | 원본(발광 코스는 `gauge_lr2` 로만 돎)과 다름 | F6 은 "결정 20 '모두 인정하되 기록'" 이라 하나 PROCESS 요약에 없는 문구 → **메인이 원문 확인**(R-7) |
| J-12 | F6 E3 | `course_best.assist_grade` 가 채워져 SCRATCH AUTO 코스 클리어끼리 순서가 게이지 등급을 따름 | 지금 R-BMS 대비 바뀜 | 결정 6 의 직접 결과(확인됨) |
| J-13 | F6 E4·E5·E10 | 단위는 `ranked=true` 판만, ASSIST·LIGHT ASSIST 램프 제외, 0012 이전 판 소급 없음 | R-BMS 규칙 | 메인 결정(F6 결정 8·9·11) |
| J-14 | F3 M2 | 레벨 폴더 제목이 `symbol + level`. **발광 표도 `LV 12` → `★12`**(`web/data/tables/insane1/header.json` 의 symbol 이 `★`, 9절) | 원본과 같아짐, 지금 화면 바뀜 | 판정 무관. 화면 변화라 ADR·QA 에 적는다 |

F3·F7 은 판정·점수 의미를 바꾸지 않는다(두 문서 4절·6절, 검토로도 확인).

---

## 5. 운영 반영 순서와 호환

### 5.1 순서

1. 웨이브 1 끝: 웹 커밋 A = `web/drizzle/0012_course_dan_tables.sql`, `meta/0012_snapshot.json`, `meta/_journal.json` 만. `schema.ts` 는 넣지 않는다(F6 2.2-4: `findCourse` 가 `select().from(course)` 로 모든 열을 읽어 migrate 전 배포면 500). 푸시 → Vercel 은 옛 코드 그대로.
2. `gh workflow run web-db.yml --ref dev -f action=status`(읽기) → 0012 `would_run=yes`.
3. 메인이 "새 열 5(course_score 3, course_chart 2)·새 표 7, 기존 행 변경 없음" 과 명령을 보이고 **사용자 승인** 또는 사용자가 `! gh workflow run web-db.yml --ref dev -f action=migrate` → `status` 로 최신 확인(HANDOFF 5절: 자동 모드가 거부).
4. 웨이브 2 끝: 웹 커밋 B1 = `schema.ts` + W2(DTO·제출·등급). 푸시 → web CI·Vercel. **이 뒤에만 F5 C4 의 `contract/client-requests/course-*.json` 을 푸시한다**: 웹 `client-requests.test.ts:252-255` 가 서버가 버린 키를 실패로 본다(F6 2.10 의 F5 정정, 9절 확인). C4 가 먼저 가면 web CI 가 깨진다.
5. 사용자가 Vercel 프로젝트에 `CRON_SECRET`(16자 이상)을 등록한다(값은 대시보드 또는 `vercel env add` 프롬프트로, 대화에 남기지 않음).
6. 웨이브 6 끝: 웹 커밋 B2 = W3~W9(`vercel.json` crons, `web-db.yml` action 6개, 화면). 푸시 → CI·배포.
7. GENOSIDE 등록: `-f action=courses-seed-count`(읽기, 기대: SP 22·DP 22 inserted, danCourses 44) → 숫자를 보이고 **사용자 승인** → `-f action=courses-seed`. 데이터 파일(F5 C5)은 웨이브 2 에 커밋돼 있어야 한다.
8. 표 첫 갱신: `-f action=tables-refresh-count`(러너가 원본을 받아 검증만) → 대시보드에서 Cron 한 번 실행 또는 다음 KST 0시. 첫 로그에서 Cloudflare 차단(F6 R2) 여부를 본다.
9. 클라이언트 릴리스(v0.1.8): 모든 Rust 웨이브와 F8·G 게이트 뒤, **6 의 웹 배포 뒤**. `release.yml` 은 사용자가 `!` 로 실행, 메인이 산출물 점검, 게시는 사용자.

### 5.2 호환 표

| 조합 | 결과 | 근거 |
| --- | --- | --- |
| 새 클라이언트 + 옛 서버(0012 전) | 코스 제출의 세 키를 서버가 버리고 200. 등급 0, `stages` 영영 없음 → 그 판은 단위 소급 불가 | F5 2.4-7(`z.object` 는 모르는 키 버림), F6 R1. 그래서 5.1-9 순서 |
| 옛 클라이언트(v0.1.7 이하) + 새 서버 | 세 키 없음 → `gauge NULL`·등급 0·`no_stage_facts`. 나머지 그대로 | F6 3.3 |
| 옛 클라이언트 보관 코스 본문의 재전송 | 세 키 없이 다시 나감 | F5 3.4-6 |
| Esc 결과 화면(F2 U-B) | 같은 `POST /scores` 본문, 기록 동기화 대신 결과 화면 전송. 계약 `score-abandoned.json`·`trace-abandoned.json` 불변이어야 함 | F2 5.2 |
| 설정 파일·계정 설정 묶음 | 새 필드는 모두 `#[serde(default)]` 구조체 안. rbms-config 에 `deny_unknown_fields` 가 없어 옛 빌드가 새 묶음을 내려받아도 모르는 필드를 버림(9절). 옛 빌드가 다시 올리면 새 필드는 기본값으로 돌아감 | F2 9절 8 |
| `gauge.ron` 사용자 덮어쓰기 | `guts_edge` 없는 옛 파일은 `Below`, 데이터 세대 1 유지 | F5 3.1 |
| 표준 표 JSON | insane1 바이트 동일, st·sl 새로 | F6 3.6 |
| IR 프로필 `extra.dans` | Rust `PlayerProfile.extra: HashMap` 이라 옛 클라이언트도 읽음 | F6 2.6-4 |
| 웹 `ignoreCommand` | `web/data/courses/…`(C5)는 `web/` 아래라 그 커밋도 웹 배포를 부른다. 무해(정적 파일) | `web/vercel.json` |

---

## 6. 메인 결정 필요 — 다섯 문서 통합

"검토" 열: 동의 = 그 문서 추천안대로, 수정 = 6.1절의 검토자 수정안.

### 6.1 검토자가 추천안을 바꾸자는 것

| id | 대상 | 검토자 추천 | 근거 |
| --- | --- | --- | --- |
| R-1 | F2 M8-5 와 F3 3.3·3.6 의 `skip_serializing_if` | 쓰지 않는다(F2 방식으로 통일). `group: ""`·`download_folder: None` 이 설정 파일에 적힌다 | 선례(`songs_folder: None`), 한 크레이트 한 방식. 계약은 어차피 웨이브마다 재생성(I-9) |
| R-2 | F3 6.3-1 "계약 영향 없음" | 영향 있음으로 고치고 U6 가 `settings-blob.json` 을 소유 | I-9 |
| R-3 | F5 결정 7 vs F3(GENOSIDE 의 트리 자리) | **COURSE 탭(C6) + 발광 표 안 `COURSE` 폴더에 번들 44개(SP 22 → DP 22)**. F3 U5 의 `rows_of(TableCourses(t))` 가 header 코스 뒤에, `t` 가 기본 발광 표(`INSANE_TABLE_ID` 의 주소 열쇠와 같음)일 때 `rbms_course::bundled::genoside_2018()` 의 코스를 붙인다. 대안: COURSE 탭만(트리 변경 없음) | 결정 11 의 "표 코스 폴더" 와 결정 18 의 GENOSIDE 가 만나는 자리가 비어 있다. GENOSIDE 는 발광 표의 단위 코스다(miraiscarlet 발광 header 가 같은 md5 열을 담음, F5 2.7-5) |
| R-4 | F5 결정 8 의 소비자 | F3 U5 가 `bundled_twin` 을 부른다(I-2) | 같은 단위의 순위가 갈리지 않게 |
| R-5 | F3 `BarIdentity::Course(name)` | `BarIdentity::Course(course_hash)` | I-3 |
| R-6 | F7 U2 의 C3 | M3 로 미룬다(지금 F4 는 C1·C2·C4 만) | 죽은 필드 금지 |
| R-7 | F6 결정 15·E1(게이지 모드 무관 인정) | 추천은 그대로 두되 근거를 "결정 20 원문" 이 아니라 "메인 판단(사용자 미확인)" 으로 ADR 에 적고, 질문 원문을 확인해 다르면 고친다 | 질문 원문이 저장소에 없다(PROCESS F0 미완) |

### 6.2 통합 표

| 문서·번호 | 결정 지점 | 추천안 | 검토 |
| --- | --- | --- | --- |
| F2 M8-1 | 저장 필드 | `play.scratch_auto_color: Option<SkinColor>`, None = 출하 초록 | 동의 |
| F2 M8-2 | 채널 행 | 색 행 뒤 접이 `SCRATCH AUTO CHANNELS`(닫힘) | 동의 |
| F2 M8-3 | 0 키 | 색 행 = 지움, 채널 = 그 채널만 출하값 | 동의 |
| F2 M8-4 | PLAY 메뉴 미리보기 | SCRATCH AUTO 를 그리지 않음(견본만) | 동의 |
| F2 M8-5 | 계약 | `settings-blob.json` 재생성, `skip_serializing_if` 안 씀 | 동의(R-1 로 F3 도 이 방식) |
| F2 M8-6 | 몸통 알파 | 사용자 알파 × 0xA0/255 | 동의 |
| F2 M8-7 | SCRATCH AUTO 꺼짐일 때 색 행 | 늘 보임 | 동의 |
| F2 M8-8 | Scratch Note 색 행이 AUTO 노트에 닿나 | 닿지 않음 | 동의 |
| F2 M9-1 | 오토·리플레이 보기 Esc | 지금처럼 선곡 | 동의 |
| F2 M9-2 | 코스에서 친 노트 0 스테이지 Esc | 지금처럼 앞 스테이지까지 FAILED 코스, 화면 없음 | 동의 |
| F2 M9-3 | Esc 결과 화면의 다시 하기 | 허용 | 동의 |
| F2 M9-4 | 알림 `run left mid-song kept…` | 지움 | 동의 |
| F2 M9-5 | Esc 판 게이지 그래프 | 꼬리 0 채움 | 동의 |
| F2 M5-1 | 헤더 표시 형식 | 최상위 정수 `rbms_extensions` | 동의 |
| F2 M5-2 | 표시 없는 스킨의 패널 1 키 4 | 원본 역할(DP OPTION 다음), 키 6 RANDOM 이전 | 동의 |
| F2 M5-3 | 표시를 넣을 기본 스킨 문서 | `select.luaskin` 만 | 동의 |
| F2 M5-4 | 선곡 문서 읽는 중 | 원본 표 | 동의 |
| F3 M1 | 표 묶음 막대 | `Command`(그림 5) | 동의 |
| F3 M2 | 레벨 폴더 제목 | 모든 표 symbol + 레벨, 없으면 `LV n` | 동의. 발광도 `★12` 로 바뀜을 ADR·QA 에(J-14), 글리프 확인(T-9) |
| F3 M3 | 새 기본 표 넣는 자리 | 앞선 기본 표 바로 뒤 | 동의 |
| F3 M4 | Stella 내장 스냅샷 | 두지 않음 | 동의 |
| F3 M5 | 하루 지난 표 다시 받기 범위 | 주소 표 전부 | 동의 |
| F3 M6 | 다시 받기 알림 | 없음 | 동의 |
| F3 M7 | 없는 곡 막대 설정 | 행 추가, 기본 켬 | 동의(결정 16 이 기능을 요구하므로 "저장값 없으면 화소 불변" 의 예외로 ADR 에 적음) |
| F3 M8 | 레벨 폴더 수 | 가진 곡 수 | 동의 |
| F3 M9 | 다운로드 폴더 기본값 | 없음, 첫 받기 때 고르기 | 동의 |
| F3 M10 | Windows 브라우저 열기 | `rundll32 url.dll,FileProtocolHandler` | 동의 |
| F3 M11 | 단일 `.bms` 링크 | 브라우저로 | 동의 |
| F3 M12 | 모달 문구 언어 | 영어 | 동의 |
| F3 M13 | 받기·풀기 상한 | 512 MiB·30초·15분, 항목 1만·항목 1 GiB·합계 2 GiB | 동의. F7 U1 이 같은 상수를 쓴다(I-13) |
| F3 M14 | 표 코스 폴더 이름 | `COURSE` | 동의 |
| F3 M15 | 묶음 이름 | `STELLA` | 동의 |
| F3 M16 | 재스캔·표 관리 Esc·로딩 취소 뒤 위치 | 경로 보존 | 동의 |
| F3 M17 | 막대 0개 폴더 진입 | 지금처럼 들어감 | 동의 |
| F3 M18 | 표 묶음 편집 | 설정 파일만 | 동의 |
| F3 M19 | 받은 압축에 그 md5 없음 | 폴더 남기고 알림 | 동의 |
| F3 M20 | 압축에 차트 없음 | 버림 | 동의 |
| F5 1 | IIDX 에서 EXHARD 이상 | EX-DAN(ExClass), 상한 ExClass | 동의 |
| F5 2 | 5·9·24키의 IIDX 표 | 수치 그대로 | 동의 |
| F5 3 | 이월 범위 | 9슬롯(원본) | 동의(J-6) |
| F5 4 | 이월 값 출처 | 판 끝 정확한 값 | 동의 |
| F5 5 | DP 22 제약·트로피 | 같은 단 SP 값 | 동의(출처 표기에 "DP 값은 SP 값" 안내, F6 3.7) |
| F5 6 | 차트 제목 | bms-ir 코스 페이지 제목(DP 22쪽은 1초 간격 한 번) | 동의. 구현자가 받는 22쪽은 외부 요청이므로 메인이 실행 주체를 정한다(에이전트 실행 허용 여부) |
| F5 7 | 번들 코스 표시 위치 | COURSE 탭 + 발광 표 아래 코스 폴더 | **수정 R-3**(F3 쪽 구현을 명시) |
| F5 8 | 같은 곡 열의 표 코스 | 번들 코스로 바꿈(`bundled_twin`) | 동의 + R-4 |
| F5 9 | 코스 id 규칙 | 유지 | 동의 |
| F5 10 | NO GOOD/NO GREAT | 범위 밖(부채 T1) | 동의 |
| F5 11 | 연습 패널 IIDX 표 | 고를 수 있게 | 동의. 단일 곡 GAUGE SET 행(`GAUGE_SET_CYCLE`)은 AUTO/LR2 그대로임을 시험으로 고정(7절 T-3) |
| F5 12 | 스테이지 사실 | `stages` 배열 | 동의 |
| F5 13 | 코스 결과 GAUGE RULE 행 | 넣음 | 동의 |
| F5 14 | 코스 막대 배지 | 선언된 제약 그대로 | 동의 |
| F5 15 | 생성기 위치 | `rbms-cli course-snapshot` | 동의 |
| F5 16 | 웹 순위 모드 분리 | F6 이 정함(→ F6 15) | 동의 |
| F6 1 | 웹 표 id | `stella-st`·`stella-sl` | 동의(F3 기본 표 id 와 같음) |
| F6 2 | 표 저장 방식 | 표마다 한 행, 받은 바이트 gzip mediumblob | 동의 |
| F6 3 | 검증 상수 | 첫 500곡, 급감 10%, 변경 25%, 1 MiB·8 MiB·20초 | 동의 |
| F6 4 | 큰 변경 수용 | `web-db.yml` count → 승인 → force | 동의 |
| F6 5 | 표 코스를 같은 갱신에서 | 예(머리 바뀜·첫 저장) | 동의 |
| F6 6 | 표에서 사라진 코스 | `active=false`, 단위 계속 | 동의 |
| F6 7 | 단위 체계 | GENOSIDE SP·DP, Stella Skill Simulator, Satellite Skill Analyzer, 技能訓練 제외 | 동의 |
| F6 8 | ranked 조건 | `ranked=true` 만 | 동의 |
| F6 9 | 램프 조건 | NO PLAY·FAILED·ASSIST·LIGHT ASSIST 제외 + 게이지 > 0 | 동의 |
| F6 10 | 스테이지 수·곡 일치 | 필수 | 동의 |
| F6 11 | 옛 판 소급 | `stages` 없는 판은 안 함 | 동의 |
| F6 12 | `user.rank` | 쓰지 않음 | 동의 |
| F6 13 | 표시·공개 | 기본 자동, 공개는 표시 단위만 | 동의 |
| F6 14 | 순위 행 배지 | 넣음 | 동의 |
| F6 15 | 게이지 모드별 순위 | 나누지 않고 기록만 | 동의 + R-7(근거 표기) |
| F6 16 | md5 만 있는 코스 차트 | `course_chart.md5` 열 + `chart_sha256 = ''` | 동의 |
| F6 17 | GENOSIDE 등록 수단 | TS 스크립트 + `web-db.yml` | 동의 |
| F6 18 | 마이그레이션 개수 | 0012 하나 | 동의 |
| F6 19 | 코스 목록 화면 | `/courses` + 내비게이션 | 동의 |
| F6 20 | 출처 표기 내용 | 이름·제작 주체·링크·받은 날(KST)·해시 12자 | 동의 |
| F6 21 | 코스 차트 제목 | `course_chart.title` | 동의 |
| F6 22 | IR 프로필 | `extra.dans` 추가 | 동의 |
| F6 23 | `CRON_SECRET` 없음 | 503 `SERVICE_NOT_CONFIGURED` | 동의 |
| F6 24 | 판정 실패와 제출 | 제출 성공, 판정은 후처리 | 동의 |
| F6 25 | 원본 요청 방식 | 차례로, 전용 UA, 리다이렉트 안 따름 | 동의 |
| F6 26 | 원격 표 읽기 캐시 | 인스턴스 메모리 캐시 | 동의 |
| F7 D-1 | `zip` 을 묶음 쓰기에도 | 쓴다 | 동의. 결정 15 승인 문구가 "해제용" 이라 **사용자 확인 항목**으로 마지막 보고(F7 13절)에 넣는다 |
| F7 D-2 | 묶음 도구 위치 | `rbms-cli mirror-pack` | 동의 |
| F7 D-3 | 해제기 위치·상한 | `rbms-library` 의 `archive` | **수정**: F3 의 `rbms-download` 를 정본으로, U1 이 그 크레이트를 의존(I-13) |
| F7 D-4 | zip 버전 | F4 줄(9.0.5, deflate-flate2-zlib-rs) | 동의 |
| F7 D-5 | 범위 기본값 | `--table` 또는 `--all` 필수 | 동의 |
| F7 D-6 | 재배포 낱말 찾기 | 넣음 | 동의 |
| F7 D-7 | NFD 이름 | 세기만 | 동의 |
| F7 D-8 | 업로드 도구 | Bun `S3Client` 스크립트(M1, 승인 뒤) | 동의 |
| F7 D-9 | 서버 서명 | Node 런타임 + `node:crypto` SigV4 | 동의 |
| F7 D-10 | 객체 키 | `packs/<sha256>.zip` | 동의 |
| F7 Q-R1~R5, Q-C1~C3 | 권리 근거·범위·접근·삭제 창구·사전 연락, 계정·버킷·결과 폴더 | F7 3.3·11절 추천 | 사용자 질문(결정 17 대로 맨 마지막 보고에 묶음). 메인이 정하지 않는다 |

---

## 7. 빠진 테스트와 위험

### 7.1 빠진 테스트(설계서에 추가할 것)

| id | 단위 | 테스트 | 단언 |
| --- | --- | --- | --- |
| T-1 | F3 U6 | `a_settings_upload` 계약 재생성 | diff 가 `show_missing_charts: true,`(R-1 이면 `download_folder: None,` 도)만 |
| T-2 | F3 U1 | 리터럴 수정 확인 | `cargo test --locked -p rbms-config` 와 `ir_sync` 시험 통과(소유 추가 5파일) |
| T-3 | F5 C1 | `the_single_song_gauge_set_cycle_stays_auto_and_lr2` | `GAUGE_SET_CYCLE` 2칸 그대로, `gauge_set_from_token("IIDX") == None`(단일 곡 GAUGE SET 은 IIDX 를 읽지 않음. `gauge_set_token` 이 `data_key()` 를 쓰므로 Iidx 추가가 토큰에 새지 않는지) |
| T-4 | F5 C3 + F2 U-B | `leaving_mid_course_ends_on_a_failed_course_result` 를 C4 뒤에도 | 코스 Esc 본문에 `stages` 가 친 스테이지 수만큼, 마지막 `gauge` 가 그 스테이지의 Class 계열 |
| T-5 | F5 C4 + F6 W2 | 계약 교차 | 새 `course-normal.json` 으로 웹 `client-requests.test.ts` "버림 없음" 통과(같은 푸시에서 확인) |
| T-6 | F7 U1 | `a_pack_is_fully_unpacked_by_the_client_extractor` | `rbms_download::extract_zip` 이 `skipped == 0` 이고 묶음의 모든 파일을 쓴다. 1 GiB 초과 항목은 `Refusal::EntryTooLarge`, 허용 목록 밖 확장자는 매니페스트 `skipped_by_client` 에 셈 |
| T-7 | F3 U5 | `the_insane_table_course_folder_lists_the_bundled_genoside_courses`(R-3 채택 시) | 발광 표 `COURSE (44)`, 첫 줄 `GENOSIDE 2018 段位認定 初段`, 23번째 `… DP段位認定 初段`, 정체가 `course_hash` |
| T-8 | F6 W3 + F5 C5 | 이름 원문 보존 | `courseHashOf` 가 앞뒤 공백이 있는 이름을 그대로 해시(F5 3.5 규칙 1) |
| T-9 | F3 U1 | 글리프 | 기본 스킨·내장 화면에서 `★12` 제목이 대체 글꼴로라도 그려진다(`.notdef` 상자 0). 캡처 `folders-nested-level` 에 발광 레벨 하나 |
| T-10 | F2 U-B | 소리 | QA 실기(헤드리스로 못 봄, F2 9절 4) |

### 7.2 위험

1. **푸시 순서(치명, CI)**: F5 C4 골든이 F6 W2 보다 먼저 푸시되면 web CI 실패(5.1-4). F5 6.3-4 의 "구 DTO 는 통과해야 한다" 는 틀렸다.
2. **릴리스 순서(치명, 데이터)**: 웹 B1 배포 전에 나간 클라이언트의 코스 판은 `stages` 가 영영 없다(5.2 첫 행).
3. **`settings-blob.json` 병합**: 한 줄 문자열이라 세 묶음의 재생성이 텍스트로 충돌한다. 단위가 각자 커밋한 값을 합치지 말고 메인이 웨이브마다 재생성(I-9).
4. **`TableSource` 리터럴**: F3 U1 소유 밖 5파일에서 컴파일 실패(3.1).
5. **J-4·J-7 의 체감 변화**: HARD 설정 사용자의 코스 램프, CN 설정 사용자의 코스 LN. 사용자 확인 전 구현하면 확인 결과에 따라 C3 일부를 되돌려야 한다 → C3 를 웨이브 3 에 두는 동안 확인을 받는다.
6. **결정 질문 원문 부재**: "추천대로" 가 무엇을 채택했는지 다섯 문서가 각자 해석했다(F2 "조사의 추천", F6 "결정 20 '모두 인정하되 기록'"). PROCESS F0 에서 질문 원문을 저장하면 G 단계 ADR 근거가 맞춰진다.
7. **Stella Cloudflare 차단**(F6 R2): Vercel 서버 요청이 막히면 웹 표만 낡는다(클라이언트는 원본 직접이라 영향 없음).
8. **zip 9.0.5 의 나이**: 게시 6일, 9.0.0 이후 패치 5번(9절). 세 묶음이 같은 줄을 쓰므로 문제가 보이면 한 번에 내린다.
9. **F3 U1 크기**: 웨이브 2 의 임계 경로. 지연되면 U2·U3·U6(웨이브 3), U4(4), U5(5), D2·D3(6·7)이 모두 밀린다. 대안: D1 과 F5 C1·C2·C5 를 먼저 끝내 두는 지금 안이 이미 U1 을 혼자 두는 꼴이다.
10. **DP 22 코스 제목 받기**(F5 결정 6): bms-ir 에 22번 요청. 외부 사이트 요청이므로 실행 주체(에이전트/사용자)를 메인이 정한다.

---

## 8. 치명적 문제 목록

구현 전에 메인이 지시에 반영해야 하는 것.

1. **F5 C4 골든 푸시 ≥ F6 W2 배포**: 어기면 web CI 실패(F5 6.3-4 정정). 웨이브 4 의 C4 커밋은 B1 푸시 전이면 보류.
2. **클라이언트 릴리스는 웹 B1 배포 뒤**: 어기면 그 사이 코스 판의 단위가 영구히 없다.
3. **표 코스 변환 함수 불일치**(I-1): F3 U5 는 `courses_from_table_header(&Value) -> TableCourses` 를 쓴다.
4. **`TableSource` 리터럴 46곳**(3.1): F3 U1 소유에 `crates/rbms-config/src/{tables.rs, tests.rs, tests/library.rs, tests/persistence.rs}`, `apps/rbms-player/src/ir_sync.rs` 를 더한다.
5. **`settings-blob.json`**(I-9): F3 의 "계약 영향 없음" 은 틀렸다. 세 묶음 모두 이 파일을 바꾸고, 메인이 웨이브마다 한 번 재생성한다.
6. **`schema.ts` 는 migrate 뒤**: 웹 커밋 A 에 넣지 않는다(F6 5.2 와 같음, 재확인).
7. **J-4·J-7 사용자 확인**: 조사 추천의 전제 오류 위에서 채택됐다.

---

## 9. 다시 확인한 코드 사실(1169efb)

| 확인 | 결과 |
| --- | --- |
| `app_result.rs:284-289` 스테이지 본문 `random: ir_random(shared.config.play.random)` | 코스 중에는 `CourseOverrides::apply_to`(`course_ui.rs:327-339`)가 `config.play.random` 을 클램프한 사본이라 실제 값이다. `random_p2: None`. I-6 의 "클램프 뒤" 는 맞다 |
| `course_ui.rs:315` `#[cfg(test)] ln_mode_for` | 맞다(F5 2.3-3) |
| `app_play.rs:578` 코스 시작 슬롯 `GaugeIndex::Class` 고정 | 맞다(F5 2.2-1) |
| `crates/rbms-library/src/scan.rs:148` `set_parser_version` | 맞다(F3 2.6-2 는 149 라 적었다, 한 줄 차이) |
| `lib.rs:458-466` `SelectView`, `:470-473` `SelectItem` | 맞다(F3 2.1) |
| `KeyLayout::of` 호출 `skinned.rs:268`, `guide.rs:242` | 맞다(F2 2.3) |
| `record_tests.rs:936·965·1184`, `contract_requests.rs:747`(F2 의 746 은 `#[test]` 줄), `:863·869`, `:934` | 맞다 |
| `app_result.rs` `enter_result :70`, `record_left_run :489`, `abandon_course :519`, `keep_run_cut_short_by_quitting :545`, `leave_course :595`, `fold_course_stage :786`, `finish_course :821` | 맞다 |
| rbms-config `deny_unknown_fields` | 0건. 옛 빌드가 새 필드를 버린다는 F2·F3·F5 판단은 맞다 |
| `contract/client-requests/settings-blob.json` 의 `library` 절 | `folders: []`, `songs_folder: None`, …, `tables: []`, `seeded_default_tables: []` 를 싣는다. `ir_sync.rs:55-70 keep_local`, `sanitise_for_upload` 가 기기 값만 기본값으로 비운다 → I-9 |
| `TableSource` 리터럴 | 14파일 46곳, `..` 사용 0, `Default` 파생 없음(`schema.rs:436`) |
| `GAUGE_SET_CYCLE`(`judge.rs:28`)·`gauge_set_token`(`:359`, `data_key()` 사용)·`gauge_set_from_token`(`:368`) | 단일 곡 GAUGE SET 은 순환 표에 없는 토큰을 AUTO 로 읽는다 → T-3 |
| `web/data/tables/insane1/header.json` | 키 `data_url, last_update, level_order, name, symbol`, `course` 없음, symbol `★` → R-3·J-14 |
| `web/src/server/service/domain/course/course.service.ts:228-243 upsertMeta` | sha256 빈 차트를 버린다(F6 2.2-3 맞음). `submit` 은 코스 정의를 만들지 않는다(정의 없는 코스의 판도 `course_score` 에 남음) |
| `web/tests/dto/client-requests.test.ts:252-256` | "파싱 결과가 입력과 같다" 가 버린 키를 실패로 본다(F6 2.3 맞음, F5 6.3-4 틀림) |
| `web/src/server/compose/{table-snapshot.ts:8-10, index.ts:366-416}` | `loadTables: getSnapshotTables` 는 `composeTableSnapshot` 안이라 W5 가 비동기로 바꿔도 `compose/index.ts` 는 바뀌지 않는다. AI 조립은 `getRecords`(이미 async)만 쓴다 → W5 가 W6 없이 타입 검사를 통과할 수 있다 |
| `Cargo.lock:957-965` flate2 1.1.10 | 의존 `crc32fast`·`miniz_oxide 0.9.1`·`zlib-rs` → `libz-rs-sys` 안 생김 |
| crates.io(2026-10-23 조회) zip | 최신 안정 9.0.5(2026-10-17, MIT, rust_version 1.88, 미철회), 9.0.1~9.0.5 가 10-10~10-17. 일반 의존 `crc32fast ^1.5`·`indexmap ^2`·`memchr ^2.7`·`typed-path ^0.12`·`nix ^0.31.3`(macOS·BSD). 기능 `deflate-flate2-zlib-rs = ["deflate-flate2", "flate2/zlib-rs"]` |
| crates.io typed-path | 최신 0.13.0(2026-10-20), 0.12.4(2026-10-20), 둘 다 MIT OR Apache-2.0, 0.12.4 의존 없음 → zip 9.0.5 는 0.12.4 를 쓴다(I-4) |

---

## 10. 메인 결정 (2026-10-23, 메인)

사용자 지시 "나머지도 추천대로"(2026-10-23)에 따라 다섯 설계서의 "메인 결정 필요" 항목은 모두 각 문서의 추천안을 채택하고, 겹치는 곳은 6.1절 검토자 수정안 R-1~R-7 을 채택한다. 2절 정정 지시 I-1~I-13 과 8절 치명적 문제의 처리도 그대로 따른다. 실행 순서는 3.2절 웨이브 1~7, 운영 순서는 5.1절이다(운영 반영은 사용자가 2026-10-23 "3번은 승인 그냥 바로 진행해놔" 로 미리 승인, `docs/PROCESS.md`).

| id | 결정 | 근거 | 사용자 확인 |
| --- | --- | --- | --- |
| J-4 | 코스 시작 슬롯은 GAUGE 설정을 따른다(NORMAL 이하 CLASS, HARD EXCLASS, EXHARD 이상 EXHARDCLASS) | 원본 `GrooveGauge.java:164` 와 같아진다. 질문 21 의 추천안이 이 규칙을 전제했다 | 미확인(최종 보고에 "지금 R-BMS 대비 바뀜" 으로 알린다) |
| J-7 | 코스의 `ln`·`cn`·`hcn` 제약을 플레이에 적용한다 | 원본 `MusicSelector.java:470-505` 와 같아지고, 결정 20 의 "LN 제약 준수" 판정이 의미를 가진다 | 미확인(같은 보고) |
| J-11 | 단위 인정에 게이지 모드 제한을 두지 않고 모드를 기록한다(LR2·BEATORAJA·IIDX 모두 인정) | 결정 20 원문 질문의 조건(게이지 남음, 미러·LN, 어시스트·SCRATCH AUTO 제외)에 모드 제한이 없고, 결정 21 이 세 모드를 설정으로 고르게 했다 | 미확인(메인 판단) |
| J-14 | 레벨 폴더 제목은 표 symbol + 레벨(발광 `LV 12` → `★12`) | 원본 beatoraja 와 같다(F3 M2) | 미확인(화면 변화, ADR·QA 에 적는다) |
| M7(F3) | 없는 곡 막대는 설정 행을 두고 기본 켬 | 요구 5 가 직접 요청했다. "저장값이 없을 때 화소 불변" 원칙의 예외로 ADR 에 적는다 | 요구로 확인 |
