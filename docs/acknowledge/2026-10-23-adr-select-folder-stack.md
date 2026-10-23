# ADR: 선곡 폴더 스택 (임의 깊이 폴더, 나올 때 커서 복원, 표 묶음, 표 코스 폴더, 레벨 제목)

> 최종 갱신 2026-10-23 · 대응 커밋 5a0eacf
> 상태: 유효(결정 10·11 은 사용자 결정, 레벨 제목 J-14 와 설계서 F3 의 세부는 메인 결정이라 사용자 미확인) · 결정일 2026-10-23 · 결정자 사용자(결정 10·11) + 메인(세부) · 커밋 `fc23232`(폴더 스택·커서 복원·표 묶음·레벨 제목), `0247561`(표 코스 폴더·발광 아래 GENOSIDE 2018·코스 파일 다시 읽기), `2787264`(기본 표의 묶음 값)
> 관계: 표 폴더와 MY FOLDERS 의 구조를 바꾼다. [사용자 폴더 ADR](2026-10-14-adr-user-folders.md) 의 MY FOLDERS 구조(MY FOLDERS > 폴더)는 그대로이고 표 묶음과는 별개다. 기본 표가 어떻게 들어오는지는 [Stella 기본 표 ADR](2026-10-23-adr-stella-default-tables.md), 없는 곡 막대는 [없는 곡·다운로드 ADR](2026-10-23-adr-missing-charts-and-download.md).

## 배경

- 요구 3(아래 원문): 발광 3 폴더에서 Esc 를 누르면 그 폴더가 포커스되어야 하는데 폴더 목록의 맨 처음으로 간다. 요구 4: Stella 난이도표를 넣을 것이고 폴더 안의 폴더로 들어가는 재귀적 폴더가 필요하다.
- 결정 전의 위치는 스택이 아니라 고정 깊이 enum `SelectView { Root, AllSongs, TableLevels, TableLevel, UserFolders, UserFolder }` 였다. 이탈은 `select_back` 한 곳이었고 세 갈래 모두 `sel = 0` 을 넣었다. 그래서 ★3 레벨 폴더에서 Esc 하면 레벨 목록의 첫 줄로, 표 폴더에서 Esc 하면 루트의 첫 줄(ALL SONGS)로 갔다.
- 원본은 `BarManager` 에 폴더 스택(`dir`)과 진입 직전 커서의 막대를 쌓는 큐(`sourcebars`)를 두고, 나올 때 그 막대를 새 목록에서 찾아 커서로 삼는다(`BarManager.java:73, 273, 298-302, 375-377, 424-447`).

## 결정

1. **폴더 스택.** `AppShared.select_view` 를 지우고 `select_path: Vec<OpenedFolder>`(비면 루트)를 둔다. 폴더는 목록의 몇 번째가 아니라 무엇인지로 부른다(`SelectFolder`): `AllSongs`, `TableGroup(이름)`, `Table`, `TableLevel(표, 레벨 이름)`, `TableCourses(표)`, `UserFolders`, `UserFolder(id)`. 표(`TableRef`)는 등록 주소로 같음을 가르고, 주소가 비어 있는 시험·캡처용 목록만 자리로 가른다. `OpenedFolder` 는 폴더와 열 때 커서가 있던 막대(`came_from`)를 함께 든다. 막대는 가진 곡·없는 곡이면 md5(소문자), 폴더를 여는 행이면 그 폴더, 코스이면 코스 해시로 알아본다(`BarIdentity`).
2. **들어가기.** 열 때 커서가 있던 막대를 `came_from` 으로 쌓고 커서를 첫 줄에 둔다. 마우스로 커서 밖의 폴더를 눌러 열어도 `came_from` 은 커서가 있던 막대다(원본 `BarRenderer` 와 같음).
3. **나오기.** 스택을 하나 꺼내 목록을 다시 만들고 커서를 `came_from` 의 막대에 둔다. 루트로 나갈 때는 방금 나온 폴더의 막대에 둔다. 막대를 못 찾으면 첫 줄이다. 키보드 Esc·왼쪽 화살표, 컨트롤러 검은 건반, 스킨의 `FolderClose`·마우스 `CloseBar` 가 모두 같은 `select_back` 을 거친다. 검색 중이면 검색을 나가고(결정 4), 아니면 `leave_folder` 다.
4. **밑에서 목록이 바뀌어도 위치를 지킨다(M16).** 전체 재스캔, 로딩 취소, 표 관리 화면에서 Esc, 표 도착, MY FOLDERS 도착, 받은 곡의 부분 스캔이 모두 루트로 돌아가지 않고 `ListPlace`/`show_the_list_again` 으로 열린 경로(아직 있는 가장 긴 앞부분)와 커서의 막대를 다시 찾는다. 검색은 시작할 때 위치를 저장하고 `[AllSongs]` 로 들어가며 끝날 때 그 위치를 돌려준다.
5. **표 묶음.** `TableSource.group` 이 같은 표들은 루트에서 묶음 막대 하나(`STELLA (n)`)로 보이고, 묶음 안에 표 막대가 들어간다. 묶음은 묶음에 처음 나온 표의 자리에 한 번 나오고, 레벨이 있는 표가 하나도 없으면 막대가 없다. 막대 종류는 `Command`(원본 `ContainerBar`, 스킨 막대 그림 5번)이고 표·레벨·코스 폴더·MY FOLDERS 는 `Table`, ALL SONGS 는 `Folder` 다. 경로는 표 묶음 > 표 > 레벨이다. 묶음 편집 UI 는 없다(M18). 기본 표 등록이 값을 넣고 설정 파일로만 바꾼다.
6. **레벨 폴더 제목(J-14, 사용자 미확인).** 표 헤더의 symbol 과 레벨을 붙인다(원본 `tag + level`, `TableDataAccessor.java:185`): 발광은 `★12`(헤더 symbol `★`), Stella 는 `st3`, Satellite 는 `sl5`. symbol 이 없거나 대체값 `*` 이면 지금처럼 `LV 12`다. 레벨 막대의 `(n)` 과 루트·묶음 합계는 가진 곡 수 그대로다(M8).
   - 이 제목은 기본 화면 머리글(열린 폴더 제목을 ` / ` 로 이음), 스킨의 `STRING_DIRECTORY`(열린 폴더 제목 목록), 곡 결정 화면의 표·레벨 문자열(`STRING_TABLE_LEVEL` 등)이 같이 쓴다. 발광 표의 화면 글자가 `LV 12` 에서 `★12` 로 바뀐다.
7. **표 코스 폴더.** 표를 열면 레벨들 뒤에 `COURSE (k)` 폴더가 있다(이름은 원본 루트의 코스 폴더 `BarManager.java:207` 에서). 안에는 표 헤더의 `course` 가 싣는 코스가 헤더 순서대로 코스 막대로 보이고, 고르면 코스 탭과 같은 규칙으로 시작한다(빠진 스테이지가 있으면 알림만, 없으면 `start_course`). 기본 발광 표의 이 폴더에는 헤더에 코스가 없어도 빌드가 싣는 GENOSIDE 2018 단위 코스가 SP 다음 DP 순서로 들어간다. 헤더가 같은 차트로 부르는 코스는 번들 코스로 바꿔 한 번만 보인다(`bundled_twin`). 코스 탭(로컬 코스 파일)은 그대로이고, 선곡에 들어올 때마다 코스 파일을 다시 읽어 파일을 넣고 뺀 것이 바로 반영된다(전에는 한 번 읽은 목록을 라이브러리에 맞춰 다시 풀기만 했다).

### 메인 결정(설계서 select-tree-download 8절, 사용자 미확인)

| id | 결정 | 이유 |
| --- | --- | --- |
| M1 | 표 묶음 막대 종류는 `Command`(그림 5) | 원본에서 다른 막대를 담는 폴더는 `ContainerBar`(그림 5)다 |
| M2 = J-14 | 레벨 폴더 제목은 헤더 symbol + 레벨(모든 표) | 원본과 같고 Stella 의 `st0` 형이 요구된다. 발광도 `★12` 로 바뀌는 화면 변화를 ADR 과 QA 에 적는다 |
| M8 | 레벨 폴더 수 `(n)` 은 가진 곡 수 | 지금 화소·시험 유지 |
| M14 | 표 코스 폴더 이름은 `COURSE` | 원본 루트 코스 폴더 이름 |
| M15 | 묶음 이름은 `STELLA` | 안의 표 이름도 대문자화로 `STELLA` 라 경로가 `STELLA > STELLA > st3` 이다 |
| M16 | 재스캔·표 관리 Esc·로딩 취소 뒤에도 경로를 보존 | 원본 `updateBar()` 가 dir 를 유지한다 |
| M17 | 막대가 0개인 폴더에도 빈 목록으로 들어감 | 기존 동작 유지(원본은 들어가지 않는다) |
| M18 | 표 묶음 편집은 설정 파일만 | 범위 최소 |

## 이유

- 사용자 결정 10·11: 요구 3 과 4 를 따로 고치면 4번에서 스택으로 다시 갈아엎게 되므로 처음부터 원본 방식의 스택으로 한 번에 푼다. 조사 보고의 결정 지점(폴더 스택 장기안, 루트로 나갈 때도 같은 규칙)에 사용자가 "1-11 추천대로"로 답했다.
- 폴더를 이름·주소·id 로 부르면 표가 갱신되거나 스캔이 곡 번호를 밀어도 같은 폴더를 다시 찾는다.
- 발광 `★12` 로의 변경은 한 표만 예외로 하면 규칙이 둘이 되어(M2 대안) 원본과 같은 한 규칙으로 했다.

## 기각된 대안

| 대안 | 기각 이유 |
| --- | --- |
| enum 위에서 `ListPlace` 로 "떠난 폴더 항목"만 찾는 단기 수리 | 재귀 폴더를 넣을 때 다시 갈아엎게 된다 |
| 표 묶음을 `Table` 막대로(그림 2) | 표처럼 보이나 원본 클래스와 다르다(M1) |
| Stella 만 symbol 제목, 발광은 `LV n` 유지 / 모두 `LV n` | 규칙이 둘이 되거나 Stella 의 `st0` 형을 못 낸다(M2) |
| 레벨 폴더 수를 `(가진/전체)` 로 | 지금 화소·시험이 바뀐다(M8) |
| 재스캔 뒤 루트·첫 줄로(지금까지) | 원본과 다르고 요구 3 의 취지에 어긋난다(M16) |
| 막대가 0개인 폴더에는 들어가지 않음(원본) | 빈 목록 안내가 있는 기존 동작을 바꾼다(M17) |
| 표 관리 화면에 묶음 편집 추가 | 범위 밖(M18) |

## 영향과 관련 파일

- 새 파일 `apps/rbms-player/src/select_tree.rs`(`SelectFolder`, `TableRef`, `OpenedFolder`, `BarIdentity`, `ListPlace`, `show_the_list_again`, `level_title`). 지운 것: `SelectView`(앱의 enum), `FocusedFolder`·`shown_level`·`shown_user_folder`.
- `apps/rbms-player/src/stage/select/{mod,list,scene,search,skinned}.rs`(`enter_folder`, `leave_folder`, `rows_of`), `lib.rs`(`SelectItem::Folder { target }`, `Course`, `CourseRef`), `tablesrc.rs`(`TableExtras { symbol, missing, courses }`, `MatchedTable`, `table_courses`), `stage/loading.rs`·`stage/tables.rs`(경로 보존), `skin_host/select/bridge.rs`(`STRING_DIRECTORY`), `stage/decide.rs`(표·레벨 문자열), `course_ui.rs`(`CourseEntry::resolved`).
- `crates/rbms-table/src/lib.rs`(`named_symbol`, `ABSENT_SYMBOL`), `crates/rbms-config/src/schema.rs`(`TableSource.group`).
- 호환: 설정 파일의 스키마 버전 6 은 그대로이고 옛 파일의 `group` 은 빈 값으로 읽힌다. 새 스킨 id 는 없다.
- 설계서와 다른 점(구현과 다름, 설계서 `docs/plan/2026-10-23-select-tree-download.md` 3.1·3.3절): `TableSource.group` 은 빈 값도 `group:""` 로 쓴다(설계는 빈 값 미기록 `skip_serializing_if`). 코스 막대의 정체는 코스 이름이 아니라 코스 해시(`Course::hash`)다. 표의 덧붙임 정보 `TableExtras` 에 symbol 말고 없는 곡 목록(`missing`)과 코스(`courses`)도 든다.
- 테스트: `stage/select/tree_tests.rs`(`leaving_a_level_puts_the_cursor_on_that_level`, `every_way_out_of_a_folder_puts_the_cursor_on_the_same_bar`, `a_table_arriving_keeps_a_nested_path_and_its_cursor`, `a_level_is_titled_with_the_symbol_its_table_names`, `the_insane_table_course_folder_lists_the_bundled_genoside_courses`, `the_course_files_are_read_again_each_time_the_browser_is_arrived_on` 외), `stage/loading.rs`(`a_rescan_keeps_the_open_folder_and_the_chart_under_the_cursor`), `crates/rbms-table`(`named_symbol_is_none_without_a_header_symbol`).
- 확인한 사실: `★`(U+2605)는 번들 글꼴 Inter Regular 와 Inter Bold 의 문자 표(cmap)에 있다(두 TTF 파일을 직접 읽어 확인. 화면에 그려 본 것은 아니다). 설계 교차 검토의 위험 T-9("★12" 글리프)는 글꼴 쪽에서는 해소됐다.
- 확인하지 못한 것: 그림 5번(Command)을 정의하지 않은 외부 스킨에서는 STELLA 묶음 막대가 그려지지 않는다(원본과 같은 동작, 보고가 있으면 M1 을 다시 본다). 같은 주소의 표가 설정에 두 행이면 두 막대의 정체가 같다(손으로 쓴 파일만, 테스트 부채). 실제 창에서 본 Esc 복원과 `★12` 표시.

## 날짜와 사용자 원문

2026-10-23.

- 요구 3(원문): "폴더에서, 예를들어서 발광 3폴더에서 esc 를 누르면 발광 3 폴더에 focus 되어있어야하는데 폴더 목록의 가장 첫번째로 가게되네, 이것도 현재 폴더로 focus 되게해놔"
- 요구 4(원문): "추가적인 난이도표를 넣을거야 https://stellabms.xyz/st/table.html 이건데 stella 난이도표야. 그러면 폴더의 폴더를 들어가는 재귀적 폴더가 필요하겠지"
- 결정 회신(원문): "1-11 추천대로". 결정 10·11 의 요지(`docs/PROCESS.md` 의 요약, 원문 아님): 폴더 스택(원본 BarManager)으로 커서 복원 / 임의 깊이 폴더 = 표 묶음 > 표 > 레벨, 표 코스 폴더.
- 레벨 제목 J-14 는 사용자가 묻지 않은 메인 결정이다(`docs/plan/2026-10-23-design-review.md` 10절: "미확인(화면 변화, ADR·QA 에 적는다)").
