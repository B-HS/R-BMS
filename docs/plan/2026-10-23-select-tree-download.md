# 설계: 선곡 폴더 트리·Stella 기본 표·없는 곡 받기 (결정 10·11·12·13·15·16)

> 최종 갱신 2026-10-23 · 기준 커밋 1169efb(dev) · 상태: 설계(구현 전, 코드 변경 없음)
> 근거 문서: [PROCESS.md](../PROCESS.md) "Stella·코스·단위와 클라이언트 보완 (2026-10-23)" 의 결정 1~21, [조사 보고](../reference/2026-10-23-stella-course-dan-survey.md), [HANDOFF.md](../HANDOFF.md) 5절. 같은 날 다른 설계자가 쓰는 [코스·게이지·코스 원천 설계](2026-10-23-course-gauge-data.md)(이하 "코스 묶음 설계")와 경계를 3.2절에 적었다.
> 원본 근거: beatoraja `select/BarManager.java`, `select/bar/{SongBar,HashBar,TableBar,ContainerBar}.java`, `select/BarRenderer.java`, `select/BarSorter.java`, `select/MusicSelector.java`, `skin/property/EventFactory.java`, `TableDataAccessor.java`, `Config.java`(경로는 모두 `/Users/hyunseokbyun/development/beatoraja/src/bms/player/beatoraja/` 아래).

## 1. 범위와 결정 번호 대응

| 결정 | 사용자 결정 요지(PROCESS 원문 요약) | 이 문서의 절 | 구현 단위 |
| --- | --- | --- | --- |
| 10 | 폴더 스택(원본 BarManager)으로 커서 복원 | 3.1 | U1 |
| 11 | 임의 깊이 폴더 = 표 묶음 > 표 > 레벨, 표 코스 폴더 | 3.1, 3.2 | U1, U5 |
| 12 | Stella st 와 Satellite sl 둘 다 기본 표, 원본 주소에서 직접 받음 | 3.3 | U2 |
| 13(클라이언트 쪽) | 저장본이 하루 지나면 시작 때 백그라운드로 다시 받음 | 3.4 | U3 |
| 16 | 표 레벨 안 흐린 없는 곡 막대 + 설정의 전용 다운로드 폴더(곡 폴더 목록에 자동 포함) | 3.5, 3.6 | U4, U6 |
| 15 | 직접 zip 링크만 앱이 받아 풀고(새 의존성 `zip` 하나 승인), rar·7z·HTML 은 브라우저, 모달에 "차분만, 음원은 곡 URL" 안내 | 3.7~3.10 | D1, D2, D3 |

하지 않는 것(다른 묶음 또는 결정 밖):

1. 웹 쪽 13(Vercel Cron·운영 DB 저장)과 14(출처 표기 웹 화면), 12 의 웹 표 페이지 — F6 묶음.
2. 표 header 의 `course` 를 `rbms_course::Course` 로 바꾸는 변환, 코스 게이지·제약·제출 — 코스 묶음 설계(F5). 이 문서는 소비자로서 가정 인터페이스만 적는다(3.2).
3. MY FOLDERS 안의 없는 곡 막대(결정 16 은 "표 레벨 안"만 말함). MY FOLDERS 는 지금처럼 가진 곡만 막대로 보인다.
4. rar·7z·lzh 해제, 단일 `.bms` 파일 저장(결정 15: 직접 zip 만), 곡 미러(결정 17).
5. 표 관리 화면에서 묶음(group)을 편집하는 UI. 묶음은 기본 표 등록이 쓰고 설정 파일로만 바꿀 수 있다.
6. 판·기록 저장/전송 조건. 이 묶음은 판 제외 조건을 새로 만들지 않고 저장·전송 경로를 건드리지 않는다.

## 2. 현재 코드 사실(1169efb 에서 다시 열어 확인)

### 2.1 선곡 위치 모델

1. `SelectView`(apps/rbms-player/src/lib.rs:458-466)는 `Clone, Copy, PartialEq` 인 평평한 enum `{Root, AllSongs, TableLevels(usize), TableLevel(usize, usize), UserFolders, UserFolder(usize)}` 이다. 부모도 커서 기억도 없다. 문서 주석은 452-457행.
2. `SelectItem`(lib.rs:470-473)은 `Song(usize)` 와 `Folder { label: String, target: SelectView }` 둘뿐이다.
3. `AppShared` 의 목록 상태: `table_names`(lib.rs:706), `table_levels: Vec<TableLevels>`(707), `select_view`(714), `select_items`(715), `sel`(721), `select_gen`(723). 초기값 `SelectView::Root`(1070).
4. 목록 생성은 `AppShared::rebuild_select_items_with`(apps/rbms-player/src/stage/select/list.rs:276-336) 한 곳이다. Root(279-294): `ALL SONGS (n)`, 레벨이 있는 표마다 `"{name} ({total})"`, `MY FOLDERS (n)`. TableLevels(297-310): 레벨마다 `"LV {level} ({n})"`. 끝에서 `sel` 을 목록 길이로 자르고(333-335) `select_gen` 을 올린다(336).
5. 막대 변환: `select_bars`(list.rs:599), `chart_bar`(612, 라이브러리에 없는 인덱스는 이미 `BarKind::Song { exists: false }` 로 만든다: 613-615), `folder_kind`(576-581: Root·AllSongs → `Folder`, 나머지 → `Table`), `folder_bar`(657), `course_bars`(702, 코스 탭 전용).
6. `TableLevels` 형(apps/rbms-player/src/tablesrc.rs:42)은 `Vec<(String, Vec<usize>)>` 로, 가진 곡의 라이브러리 인덱스만 담는다. 표 항목(title·artist·url·url_diff)은 선곡까지 오지 않는다.

### 2.2 들어가기·나오기

1. 들어가기: `SelectState::enter_row`(apps/rbms-player/src/stage/select/mod.rs:421-443)가 `Folder` 이면 `select_view = target; sel = 0;` 후 rebuild(434-440).
2. 나오기의 모든 길이 `select_back`(mod.rs:866-895) 하나로 모인다. Esc·왼쪽: `handle_key` 의 1175행 → `select_escape`(851-861) → `select_back`. 컨트롤러 검은 건반: 1237행 → `close_folder`(382-387) → `select_back`. 스킨 키 FolderClose(apps/rbms-player/src/stage/select/skinned.rs:313-315)와 마우스 CloseBar(skinned.rs:397-398) → `close_bar`(349-357) → `select_back`(354). 검색이 켜져 있으면 폴더보다 검색을 먼저 나간다(mod.rs:867-871).
3. `select_back` 의 세 갈래가 모두 `shared.sel = 0`(880, 885, 890)이다. 조사 보고의 원인 진단이 맞다.
4. 루트에서: Esc 는 두 번 눌러 종료(851-861), 검은 건반은 아무것도 안 함(383), 스킨 close 는 정렬 이벤트(skinned.rs:355-356, 원본 `BarManager.close()` 490-494 와 같음).

### 2.3 위치 복원 선례와 다른 호출처

1. `FocusedFolder`(tablesrc.rs:990-1022)와 `ListPlace`(1041-1097), `show_the_list_again`(1099-1103). 표가 도착했을 때(`apply_finished_fetches` 893-944, 908행에서 `ListPlace::of`, 937행에서 복원)와 검색 이탈(`exit_search`, apps/rbms-player/src/stage/select/search.rs:205-215)에 쓰인다. MY FOLDERS 도착도 같은 것을 쓴다(apps/rbms-player/src/user_folders.rs:35 import).
2. 검색 진입은 `search_resume = Some(ListPlace::of)` 후 `select_view = AllSongs`(search.rs:147-156), 다른 화면이 루트로 돌린 검색은 `drop_a_search_left_behind`(249-259)가 버린다.
3. 루트로 강제로 돌리는 곳: 스캔 적용 `apply_scan`(apps/rbms-player/src/stage/loading.rs:441-453, 448-450), 로딩 취소 `cancel`(531-543, 539-541), 표 관리 화면 Esc(apps/rbms-player/src/stage/tables.rs:415-419).
4. 플레이 뒤 복귀 `leave_play`(apps/rbms-player/src/app_play.rs:779-787)는 `select_view` 와 `sel` 을 그대로 두고 rebuild 만 한다. 결과 화면의 다음 곡 `following_song`(apps/rbms-player/src/app_result.rs:752-759)은 `SelectItem::Song` 만 따라간다.
5. 스킨 브리지 `open_folders`(apps/rbms-player/src/skin_host/select/bridge.rs:117-140)가 `STRING_DIRECTORY`(1000, apps/rbms-player/src/skin_host/select.rs:431, 구분자 `" > "` shown.rs:46·243-245)를 만든다. 레벨은 `"LV {level}"`(bridge.rs:27·125-132). 결정 화면의 `table_of_the_open_folder`(apps/rbms-player/src/stage/decide.rs:310-317)는 `TableLevel` 일 때만 (표 이름, 레벨 원문)을 준다. 기본 화면 머리글은 scene.rs:155-170.
6. 스킨 불린: `OPTION_FOLDERBAR`(1)·`OPTION_SONGBAR`(2)·`OPTION_GRADEBAR`(3)·`OPTION_PLAYABLEBAR`(5) 가 막대 종류로 정해진다(skin_host/select.rs:320-327). `OPTION_PLAYABLEBAR` 는 `Song { exists: true }` 일 때만 참이다.

### 2.4 표 받기·저장·기본 표

1. 기본 표 규칙: `DefaultTable { id, location, same_as }`(crates/rbms-config/src/tables.rs:17-28), id·location 은 릴리스 뒤 바꾸지 않는다(19-24). `DEFAULT_TABLES` 는 발광 한 건(36-37). `seed_default_tables`(79-93)는 한 번만 넣고, 넣은 표는 목록 맨 앞(91)에 둔다. 주소 비교 `table_address_key`(48-59)는 `/index.html`·`/header.json`·`/data.json`(41)만 같은 표로 본다. `table.html`·`score.json` 은 같은 표로 보지 않는다.
2. 내장 스냅샷: `SNAPSHOTS: [Snapshot; 1]`(apps/rbms-player/src/tablesrc/seed.rs:31-35). `carried` 가 없으면 경고 알림을 낸다(55-58). 시험 `every_table_given_is_carried_by_the_binary`(95-103)는 모든 기본 표가 내장되어 있기를 요구한다.
3. `TableSource { name, location }`(crates/rbms-config/src/schema.rs:436-440), `LibraryOptions`(850-876: `folders` 855, `tables` 871, `seeded_default_tables` 874). 스키마 버전은 6(schema.rs:20).
4. 받기: `fetch_with`(crates/rbms-table/src/fetch.rs:78-95)가 페이지 meta → header → `data_url`(상대 주소 합침, 106-107)로 간다. Stella 의 `data_url: "score.json"` 이 그대로 된다. 상한 `LOAD_TIMEOUT` 30초(27행), `MAX_RESPONSE_BYTES` 64MB(37행), 무응답 15초(22행).
5. 헤더 `course` 는 해석하지 않고 `Option<serde_json::Value>` 로 보관한다(crates/rbms-table/src/dto.rs:51-54, lib.rs:28-30, saved.rs:26-27·42-43).
6. 레벨 순서: `level_order` 가 없으면 숫자 순(crates/rbms-table/src/lib.rs:194-207). symbol 이 없는 헤더나 헤더 없는 표의 symbol 은 `"*"`(lib.rs:107, 시험 446). 플레이어는 지금 symbol 을 전혀 쓰지 않는다(`.symbol` 소비처는 rbms-table 시험뿐).
7. 저장본 위치 `tables/<주소해시>.json`(tablesrc.rs:58·238-260). 자동으로 다시 받지 않는다(apps/rbms-player/src/stage/tables.rs:15-17, `COPY_OLD_AFTER_DAYS = 30` 은 표시 강조뿐 tablesrc.rs:66-68). 첫 받기는 `start_first_fetches`(840-858), 결과 적용 `apply_finished_fetches` 는 선곡(mod.rs:1051-1053)과 표 관리(tables.rs:380-381)에서만 부른다. 플레이 중에는 적용하지 않는다.
8. 표 이름은 라틴 문자를 대문자로 바꾼다(`pick_table_name`, tablesrc.rs:402-404, 시험 1393: `"Satellite"` → `"SATELLITE"`). Stella 헤더 이름 "Stella" 는 "STELLA" 로 보인다.

### 2.5 막대 종류와 스킨

1. 렌더러는 이미 없는 곡 막대를 원본처럼 그린다: `BarKind::Song { exists: false }` → 그림 4번(`IMAGE_MISSING`, crates/rbms-render/src/skin_render/songlist/bars.rs:50·142-153), 레벨 숫자는 있는 곡만(`level_index` 281-286), 기능 라벨도 있는 곡만(`drawn_features` 247-252). `ContainerBar`·`CommandBar` 는 `BarKind::Command` → 그림 5번(bars.rs:133-134·149). 원본 `BarRenderer.java:150-165` 의 `ba.value` 와 같다.
2. 기본 스킨은 이미 없는 곡 막대 그림과 흐린 글자를 갖고 있다: `BAR_SPRITES` 의 `"bar_missing"`(assets/skins/rbms-default/lua/select/list.lua:13), `{ id = "txt.bar.missing", color = theme.color.muted }`(31행).
3. ModernChic 도 7종 막대 그림을 모두 정의한다(`bar-song`·`bar-command`·`bar-grade`·`bar-nosong`·`bar-table`·`bar-folder`·`bar-search`, /Users/hyunseokbyun/Downloads/ModernChic/Select/lua/songlist.lua:195-201).
4. 기본(스킨 없는) 행은 `row_of`(apps/rbms-player/src/stage/select/scene.rs:102-140)가 `SelectRow`(crates/rbms-render/src/select.rs:23-37)로 만든다. 흐림 표시 필드는 없다. 세부 패널 `SelectDetail { Song, Folder, Empty }`(select.rs:100-104).

### 2.6 곡 폴더·스캔·노출 경계

1. 곡 폴더 목록 `config.library.folders`, 시작 스캔(lib.rs:1016-1019), 다시 스캔 `LoadingState::rescan`(loading.rs:415-418), 곡 폴더 화면은 나갈 때 전체 다시 스캔(apps/rbms-player/src/stage/folders.rs:88-90).
2. `scan_into`(crates/rbms-library/src/scan.rs:122-151)는 `roots` 아래만 지우고(`delete_missing`, songdb.rs:258-278) 끝나면 무조건 `set_parser_version(PARSER_VERSION)`(scan.rs:149)을 찍는다. 즉 일부 폴더만 훑는 스캔이 이 줄을 지나면, 다른 폴더에 아직 필요한 전체 재파싱이 "끝난 것"으로 기록된다.
3. 스캔 뒤 라이브러리는 `stored_library`(apps/rbms-player/src/library.rs:92-100) = DB 의 모든 곡을 제목 순 정렬(84-88). 곡 하나가 들어오면 그 뒤 곡들의 인덱스가 밀린다.
4. 곡 DB 연결에 `busy_timeout` 설정이 없다(grep 0건). 두 연결이 동시에 쓰면 SQLITE_BUSY 가 난다.
5. 숨김 폴더도 훑는다(`enumerate`, scan.rs:165-189 는 이름으로 거르지 않고 심볼릭 링크만 건너뜀).
6. 기기 고유 경로는 설정 동기화에서 빠진다(`keep_local`, apps/rbms-player/src/ir_sync.rs:55-70: folders·tables·seeded_default_tables 등).
7. 외부 프로세스 실행 `Command::new` 는 저장소에 0건, 압축 해제 코드 0건.
8. 의존성: `reqwest 0.12`(blocking·rustls, Cargo.toml workspace), Cargo.lock 의 `flate2 1.1.10` 은 `crc32fast`·`miniz_oxide 0.9.1`·`zlib-rs 0.6.8` 에 의존한다(Cargo.lock:957-965). `indexmap 2.14.2`·`memchr 2.8.3`·`nix 0.31.3` 도 이미 있다. 라이선스 목록은 THIRD-PARTY-NOTICES.md 5절.

### 2.7 조사 보고 정정

| 조사 보고 문구 | 확인 결과 |
| --- | --- |
| 3번 "list.rs:334-336 clamp" | 333-335행이다. |
| 4번 "user_folders.rs:1-5, 44"(MY FOLDERS 2단 근거) | `USER_FOLDERS_LABEL` 은 39행, 44행은 `COPY_EXTENSION` 이다. 근거 문장은 1-9행 모듈 주석. |
| 5번 "목록 모델, 스킨 쪽 막대 종류(list.rs:578-579 BarKind) ... 모두 바뀝니다. 외부 Lua 스킨이 그리는 막대와의 호환 영향이 큽니다" | 렌더러가 이미 `Song { exists: false }` 를 원본대로 그리고(2.5-1), 기본 스킨·ModernChic 이 없는 곡 막대 그림을 갖고 있다(2.5-2·3). 바뀌는 것은 목록 모델과 기본(스킨 없는) 행이고, 외부 스킨 쪽 새 그림·새 id 는 필요 없다. |
| 5번 "flate2 ... png 의 하위 의존으로만" | Cargo.lock 의 flate2 는 `zlib-rs` 에도 의존한다(Cargo.lock:961-965). `zip` 의 `deflate-flate2-zlib-rs` 기능을 쓰면 새 압축 백엔드 크레이트가 생기지 않는다(3.8). |
| 3번 "SelectView ... lib.rs:458-466" | 맞다(문서 주석 452-457). |
| 3번 이탈 경로·`select_back` 866-895·`sel = 0` 세 곳 | 맞다(2.2). |
| 4번 "표 레벨은 level_order 가 없으면 숫자 순" | 맞다. 덧붙여 원본은 레벨 폴더 제목을 `tag + lv`(header symbol + 레벨)로 짓는다(`TableDataAccessor.java:185`). R-BMS 는 `"LV {level}"` 이다. |

## 3. 설계

### 3.1 폴더 스택과 커서 복원(U1, 결정 10·11)

#### 3.1.1 새 타입(새 파일 `apps/rbms-player/src/select_tree.rs`, lib.rs 에 `mod select_tree;`)

```rust
/// A folder the browser can open (`DirectoryBar`), named by what it is rather than by where it sits
/// in a list, so the same folder is found again after the tables or the library under it changed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SelectFolder {
    AllSongs,
    /// Several tables shown under one row of the root (no counterpart in the reference).
    TableGroup(String),
    Table(TableRef),
    /// A level of a table, by the level's own name (`HashBar`).
    TableLevel(TableRef, String),
    /// The courses a table's header lists (U5).
    TableCourses(TableRef),
    UserFolders,
    /// One of the player's own folders, by its id.
    UserFolder(String),
}

/// A difficulty table of the player's list: its address, and the place it was at when this was
/// made, which is only a hint once the list has changed.
#[derive(Clone, Debug, Eq)]
pub(crate) struct TableRef {
    pub(crate) at: usize,
    pub(crate) location: String,
}

/// One open folder and the bar the cursor was on when it was opened (`BarManager.dir`, `sourcebars`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct OpenedFolder {
    pub(crate) folder: SelectFolder,
    pub(crate) came_from: Option<BarIdentity>,
}

/// What a bar is, for finding it again in a list built afresh.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum BarIdentity {
    /// A chart, owned or missing, by its md5 in lower case.
    Chart(String),
    /// A row that opens a folder.
    Opens(SelectFolder),
    /// A course, by its name (U5).
    Course(String),
}
```

규칙:

1. `TableRef` 의 `PartialEq` 는 직접 구현한다: 두 `location` 이 모두 비어 있지 않으면 `location` 만 비교하고, 하나라도 비면 `at` 을 비교한다. 설정에 표 행이 없는 시험·캡처(`table_names`·`table_levels` 만 직접 채움, capture.rs:1771-1772)도 같은 규칙으로 동작한다.
2. `TableRef::resolve(&self, shared: &AppShared) -> Option<usize>`: `config.library.tables.get(at)` 의 location 이 같으면 `at`, 아니면 location 이 비지 않았을 때 처음 같은 location 의 자리, location 이 비었으면 `at < table_levels.len()` 일 때 `at`. 그 밖은 `None`.
3. `AppShared` 에서 `select_view: SelectView` 를 지우고 `select_path: Vec<OpenedFolder>` 를 둔다(비어 있으면 루트). `SelectView` enum 은 삭제한다(죽은 코드 남기지 않음).
4. `SelectItem` 은 `Folder { label: String, target: SelectFolder }` 로 바뀐다(U4 가 `Missing`, U5 가 `Course` 를 더한다).
5. `select_tree.rs` 의 함수:
   - `impl SelectFolder { pub(crate) fn title(&self, shared: &AppShared) -> String }`: 막대 제목에서 수 `(n)` 를 뺀 바탕 제목. AllSongs `"ALL SONGS"`, TableGroup 이름 그대로, Table 은 `table_names[resolve]`, TableLevel 은 3.1.4 의 레벨 제목, TableCourses `"COURSE"`(M14), UserFolders `USER_FOLDERS_LABEL`, UserFolder 은 폴더 이름.
   - `pub(crate) fn is_there(&self, shared: &AppShared) -> bool`: AllSongs 항상; Table 은 resolve 되고 레벨이 1개 이상; TableLevel 은 그 표에 같은 이름의 레벨; TableGroup 은 그 묶음의 표 하나라도 레벨이 있음; TableCourses 는 코스가 1개 이상(U5 전에는 항상 false); UserFolders 는 `user_folders.shown` 이 비지 않음; UserFolder 는 그 id 가 있음.
   - `pub(crate) fn bar_kind(&self) -> BarKind`: AllSongs → `Folder`; TableGroup → `Command`(원본 `ContainerBar`, M1); Table·TableLevel·TableCourses·UserFolders·UserFolder → `Table`(원본 TableBar·HashBar, 루트 코스 폴더도 TableBar). list.rs 의 `folder_kind`(576-581)를 이것으로 바꾼다.
   - `impl AppShared { pub(crate) fn identity_of(&self, item: &SelectItem) -> Option<BarIdentity>; pub(crate) fn is_at_root(&self) -> bool; pub(crate) fn open_titles(&self) -> Vec<String>; pub(crate) fn innermost_level(&self) -> Option<(usize, String)> }`. `identity_of(Song(i))` 은 `library.songs()[i].md5` 소문자.
6. `ListPlace` 와 `show_the_list_again` 은 tablesrc.rs 에서 select_tree.rs 로 옮기고 `FocusedFolder`·`shown_level`·`shown_user_folder`(tablesrc.rs:970-1022)는 지운다. 새 정의:

```rust
pub(crate) struct ListPlace {
    path: Vec<OpenedFolder>,
    focused: Option<BarIdentity>,
    sel: usize,
}
impl ListPlace {
    pub(crate) fn of(shared: &AppShared) -> ListPlace;
    /// The path as the tables and folders stand now: the longest prefix whose every folder is still there.
    pub(crate) fn path(&self, shared: &AppShared) -> Vec<OpenedFolder>;
    /// The row of the bar the cursor was on, found by what it is; the row it was on when it is gone.
    pub(crate) fn row(&self, shared: &AppShared) -> usize;
}
pub(crate) fn show_the_list_again(shared: &mut AppShared, place: &ListPlace);
```

`path` 는 앞에서부터 `is_there` 가 거짓인 첫 프레임에서 자른다(지금의 "레벨이 사라지면 표의 레벨 목록, 표가 사라지면 루트" 규칙 tablesrc.rs:1068-1084 를 임의 깊이로 일반화). 자른 프레임 안의 `TableRef.at` 은 `resolve` 결과로 고쳐 쓴다. `row` 는 지금처럼 찾으면 그 자리, 못 찾으면 이전 `sel` 을 목록 길이로 자른다(tablesrc.rs:1090-1096 과 같음). user_folders.rs 와 search.rs 의 import 를 새 경로로 고친다.

#### 3.1.2 목록 만들기

`AppShared::rebuild_select_items_with`(list.rs:276)의 `match self.select_view` 를 `self.rows_of(self.select_path.last().map(|open| &open.folder), filter)` 로 바꾼다. `fn rows_of(&self, folder: Option<&SelectFolder>, filter: SelectFilter) -> Vec<SelectItem>`:

1. 루트(`None`): `ALL SONGS (n)` → 설정 표 순서대로 걷되, 묶음이 빈 표(`group` 이 빈 문자열)는 그 자리에 표 막대, 묶음이 있는 표는 그 묶음이 처음 나온 자리에 묶음 막대 한 번(`"{group} ({합})"`, 레벨이 있는 표가 하나도 없는 묶음은 막대 없음) → `MY FOLDERS (n)`. 표 막대 수 계산은 지금 식(list.rs:284) 그대로.
2. `TableGroup(name)`: 그 묶음의 표 가운데 레벨이 있는 것을 설정 순서로, 막대 모양은 루트의 표 막대와 같다.
3. `Table(t)`: 레벨마다 `"{레벨 제목} ({가진 곡 수})"` → (U5) 코스가 있으면 끝에 `"COURSE ({k})"`.
4. `TableLevel(t, level)`: 지금의 `TableLevel` 갈래(list.rs:311-316)와 같다(U4 가 없는 곡을 더함).
5. `AllSongs`·`UserFolders`·`UserFolder(id)`: 지금 갈래와 같고, `UserFolder` 은 id 로 찾는다(못 찾으면 빈 목록).
6. 마지막 처리(333-336)는 그대로.

#### 3.1.3 들어가기·나오기(원본 BarManager 대응)

`SelectState` 에 두 함수를 둔다(mod.rs):

```rust
fn enter_folder(&mut self, shared: &mut AppShared, folder: SelectFolder);
fn leave_folder(&mut self, shared: &mut AppShared);
```

1. `enter_folder`: `came_from = shared.select_items.get(shared.sel).and_then(|item| shared.identity_of(item))`(원본 `prevbar = currentsongs[selectedindex]`, BarManager.java:273, 쌓는 곳 375-377) → `select_path.push(OpenedFolder { folder, came_from })` → `sel = 0` → `self.rebuild(shared)`. 마우스로 커서 밖 폴더를 눌러 열어도(skinned.rs:388) `came_from` 은 커서 막대다(원본 `BarRenderer.mousePressed` 93-94 → `select.select(sd)` → `updateBar`, 같은 동작).
2. `leave_folder`: `let Some(left) = select_path.pop()` → rebuild → 찾을 막대 = 경로가 비었으면 `BarIdentity::Opens(left.folder)`(원본 루트로 갈 때 `prevbar = dir.first()`, 284-286), 아니면 `left.came_from`(원본 `sourcebar = sourcebars.removeLast()`, 298-302) → `sel = select_items 에서 identity_of 가 같은 첫 행, 없으면 0`(원본 424-447).
3. `enter_row` 의 `Folder` 갈래(434-440)는 `enter_folder(target.clone())` 와 FolderOpen 소리. `select_back` 의 두 번째 match(876-893)는 `SystemSound::FolderClose` 후 `leave_folder` 한 줄로 바뀐다. 루트 판정 `select_view == SelectView::Root`(383, 852, 873, 877, skinned.rs:353)는 `shared.is_at_root()`.
4. 검색: `enter_search`(search.rs:147-156)는 `search_resume = Some(ListPlace::of(shared))` 후 `select_path = vec![OpenedFolder { folder: SelectFolder::AllSongs, came_from: None }]`. `exit_search`(205-215)는 `select_path = place.path(shared)` → rebuild → `sel = place.row(shared)`. `drop_a_search_left_behind`(249-259)의 `select_view == AllSongs` 는 `select_path == [AllSongs 하나]`.
5. 표 도착(`apply_finished_fetches`), MY FOLDERS 도착(user_folders.rs 의 `show_the_list_again` 호출)은 새 `ListPlace` 로 그대로 동작한다.
6. 스캔 적용(loading.rs:448-450)과 로딩 취소(539-541), 표 관리 화면 Esc(tables.rs:416-418)는 루트로 돌리지 않고 `let place = ListPlace::of(shared)` 를 바뀌기 전에 잡아 바뀐 뒤 `show_the_list_again` 한다(M16, 원본 `updateBar()` 가 현재 dir 을 유지하는 것과 같음). 스캔 적용은 라이브러리를 바꾸기 전에 place 를 잡는다(place 는 인덱스가 아니라 md5·이름·주소로 기억하므로 새 라이브러리에서도 찾는다).
7. 플레이 뒤 복귀 `leave_play` 는 바꾸지 않는다(경로와 `sel` 이 그대로 남는다). 문서 주석만 새 이름으로.

#### 3.1.4 제목·머리글·브리지

1. 레벨 제목(`SelectFolder::TableLevel` 의 `title`): 그 표의 `named_symbol()`(아래)이 있으면 `"{symbol}{level}"`(원본 `td.getTag() + lv`), 없으면 지금처럼 `"LV {level}"`(M2). rbms-table 에 `pub fn named_symbol(&self) -> Option<&str>` 를 더한다: symbol 이 비었거나 헤더가 없을 때 넣는 대체값이면 `None`. 대체값 `"*"` 는 `pub(crate) const ABSENT_SYMBOL: &str = "*";` 로 lib.rs:107 과 함께 쓴다(같은 크레이트가 자기 대체값을 판정하는 것이므로 매직 문자열 비교가 아니다).
2. 플레이어가 symbol 을 알도록 tablesrc.rs 에 표마다의 덧붙임 정보를 둔다:

```rust
/// What the browser keeps of a table beside its levels: aligned with `table_names` and `table_levels`.
#[derive(Clone, Debug, Default)]
pub(crate) struct TableExtras {
    /// The symbol the header names, without the stand-in for none.
    pub(crate) symbol: Option<String>,
}
/// A table read and matched against the library.
pub(crate) struct MatchedTable {
    pub(crate) name: String,
    pub(crate) levels: TableLevels,
    pub(crate) extras: TableExtras,
}
```

   `read_and_match`(437)·`read_and_match_all`(466)·`match_and_record`(414)은 `MatchedTable`/`Vec<MatchedTable>` 를 돌려준다. `AppShared` 에 `table_extras: Vec<TableExtras>` 를 더하고 `set_table_slot`(957-968)이 세 벡터를 같이 채운다. 표 관리 화면의 `remove_table_source`(tables.rs:304-317)는 새 `AppShared::remove_table_slot(at)` 으로 세 벡터를 같이 지운다. 시험이 `table_names`·`table_levels` 만 채우면 `table_extras.get(i)` 이 `None` 이고 기본값으로 읽는다. `ScanOutcome`(loading.rs:75-79)은 `tables: Vec<MatchedTable>` 로 바꾼다(491행 호출처).
3. 기본 화면 머리글(scene.rs:155-170): 루트는 지금과 같고, 그 밖에는 `open_titles().join(" / ")`. 지금 화면(`"{table} / LV {l}"`, `"MY FOLDERS / {name}"`)과 같은 글자가 나온다(symbol 없는 표).
4. 스킨 `STRING_DIRECTORY`(bridge.rs `open_folders`, 117-140): 검색이 켜져 있으면 지금처럼 `Search : '{word}'` 하나, 아니면 `open_titles()`. 원본 `dirString`(BarManager.java:457-461, 각 제목 뒤 `" > "`)과 같은 모양이 된다: `STELLA > STELLA > st3 > `.
5. 결정 화면(decide.rs:310-317): `innermost_level()` 이 주는 (표 자리, 레벨 이름)으로 `(표 이름, 레벨 제목)` 을 만든다. 레벨 제목이 symbol 을 붙인 값이 되어 원본 `setTablelevel(HashBar.getTitle())`(MusicSelector.java:355-366)와 같아진다. `STRING_TABLE_NAME`(1001)·`STRING_TABLE_LEVEL`(1002)·`table_full`(skin_host/chart.rs:187-189)이 이 값을 읽는다.
6. 디버그 출력 `print_selection`(app_play.rs:1126-)은 `open_titles()` 를 찍는다.
7. 새 스킨 id 는 없다. 확장 대역(이벤트 21000~21099·불린 21100~21199)도 쓰지 않는다.

#### 3.1.5 외부 Lua 스킨 막대 종류 호환

| 행 | 막대 종류 | 원본 클래스 | 원본 그림(`ba.value`) |
| --- | --- | --- | --- |
| ALL SONGS | `Folder` | FolderBar | 1 |
| 표 묶음(STELLA) | `Command` | ContainerBar | 5 |
| 표 | `Table` | TableBar | 2 |
| 레벨 | `Table` | HashBar | 2 |
| 표 코스 폴더 | `Table` | TableBar(원본 루트 "COURSE") | 2 |
| MY FOLDERS·폴더 | `Table` | (지금 규칙 유지) | 2 |
| 가진 곡 | `Song { exists: true }` | SongBar | 0 |
| 없는 곡(U4) | `Song { exists: false }` | SongBar(existsSong false) | 4 |
| 표 코스(U5) | `Course { complete }` | GradeBar | 3 / 4 |

`OPTION_FOLDERBAR` 는 `is_directory`(bars.rs:157-159)로 정해지므로 Command 인 묶음도 폴더로 읽힌다. 그림 5번이 없는 스킨에서는 묶음 막대가 그려지지 않는다(원본과 같음, 9절 위험).

### 3.2 표 코스 폴더(U5, 결정 11)와 코스 묶음과의 경계

1. 가정 인터페이스(코스 묶음 설계가 정한다, 이름이 다르면 U5 는 호출 한 줄만 바꾼다):

```rust
// crates/rbms-course/src/table.rs (F5 소유)
/// The courses a difficulty table's header lists (`header.course`: a list of courses, or a list of
/// lists of them), flattened in the header's order. An item that is not a course is left out.
pub fn courses_of_table(course: &serde_json::Value) -> Vec<rbms_course::Course>;
```

   계약: 입력은 `DifficultyTable.course` 원문(dto.rs:51-54), 출력 순서는 헤더 순서, 실패해도 패닉하지 않고 해당 항목만 버린다. 모르는 제약은 원본처럼 버린다(TableDataAccessor.java:199-200). Stella 헤더는 `course: [[{name, constraint, trophy, md5: [...]}, ...]]` 모양이다(조사 산출물 scratchpad/web/header.json 확인: 그룹 1개, 코스 26개, 차트는 `md5` 배열). 코스의 해시·IR 제출·게이지는 F5 몫이다.
2. 이 묶음이 하는 것: `TableExtras` 에 `courses: Vec<CourseEntry>` 를 더한다. 매칭 워커(`match_and_record`)가 `courses_of_table` 을 불러 `Vec<Course>` 를 만들고, 프레임 스레드의 `set_table_slot` 이 `CourseEntry::resolved(course, &library)`(새 생성자, apps/rbms-player/src/course_ui.rs, `CourseList::resolve` 와 같은 규칙)로 빠진 곡을 계산한다. 라이브러리가 바뀌면(스캔·부분 스캔) 다시 계산한다.
3. 목록: `Table(t)` 끝에 `SelectItem::Folder { label: "COURSE ({k})", target: TableCourses(t) }`. `TableCourses(t)` 의 행은 `SelectItem::Course(CourseRef { table, course })`. 막대는 `course_bars`(list.rs:702-729)의 한 건 계산을 `fn course_bar(entry: &CourseEntry, library: &Library, facts: &ChartFacts) -> SelectBar` 로 빼서 두 곳(코스 탭, 표 코스 폴더)이 같이 쓴다.
4. 고르기: `enter_row` 의 `Course` 갈래는 지금의 `course_enter`(mod.rs:446-458)와 같은 규칙: 빠진 곡이 있으면 알림만, 아니면 `crate::start_course(shared, &entry)`. 코스 탭(`SelectTab::Courses`, 로컬 코스 파일)은 그대로 둔다.
5. 정체: `BarIdentity::Course(name)`(원본 GradeBar 는 클래스+제목).
6. 판정·점수: 표 코스의 `gauge_lr2`·`grade_mirror`·`ln` 제약이 게이지·옵션을 바꾸는 것은 F5 의 코스 게이지 설계(결정 21: 설정이 `gauge_lr2` 보다 우선)대로 `CourseOverrides` 가 처리한다. 이 묶음은 바꾸지 않는다.

### 3.3 Stella·Satellite 기본 표(U2, 결정 12)

1. 설정 스키마: `TableSource` 에 묶음을 더한다(crates/rbms-config/src/schema.rs:436-440). 이 필드는 U1 이 더한다(목록이 읽음).

```rust
pub struct TableSource {
    pub name: String,
    pub location: String,
    /// The row of the song list's root the table is shown under together with the others of the
    /// same name, or empty for a row of its own.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub group: String,
}
```

   빈 묶음은 쓰지 않으므로 지금 설정 파일과 바이트가 같다. 스키마 버전은 6 그대로(덧붙인 필드뿐, 옛 빌드는 모르는 필드를 무시).
2. 기본 표(crates/rbms-config/src/tables.rs): `DefaultTable` 에 `pub group: &'static str` 를 더하고 `source()` 가 넣는다.

```rust
pub const STELLA_TABLE_ID: &str = "stella-st";
pub const SATELLITE_TABLE_ID: &str = "stella-sl";
/// The row the two tables of stellabms.xyz are shown under.
pub const STELLA_GROUP: &str = "STELLA";
pub const DEFAULT_TABLES: &[DefaultTable] = &[
    DefaultTable { id: INSANE_TABLE_ID, location: "https://bms.hyuns.uk/tables/insane1", same_as: &["darksabun.club/table/archive/insane1"], group: "" },
    DefaultTable { id: STELLA_TABLE_ID, location: "https://stellabms.xyz/st/table.html", same_as: &["stellabms.xyz/st", "stellabms.xyz/st/score.json"], group: STELLA_GROUP },
    DefaultTable { id: SATELLITE_TABLE_ID, location: "https://stellabms.xyz/sl/table.html", same_as: &["stellabms.xyz/sl", "stellabms.xyz/sl/score.json"], group: STELLA_GROUP },
];
```

   `same_as` 는 `table_address_key` 결과로 적는다: `/st/`·`/st/header.json`·`/st/index.html` 은 `stellabms.xyz/st` 로, `score.json` 은 그대로 남으므로 따로 적는다. 사용자 지시("stella bms 발광표에서 항상 가져오고")대로 우리 서버 사본이 아니라 원본 주소다. 이 두 id·location 은 첫 릴리스 뒤 바꾸지 않는다(19-24행 규칙).
3. 넣는 순서(M3 추천): 이미 받은 기본 표가 목록에 있으면 새 기본 표를 `DEFAULT_TABLES` 순서상 바로 앞 기본 표 뒤에 넣고, 앞 기본 표가 목록에 없으면 맨 앞에 넣는다. 새 사용자는 `[insane, st, sl]`, 발광을 이미 받은 사용자도 `[insane, st, sl, 사용자 표...]` 가 된다. 지금의 "맨 앞" 규칙(91행)을 그대로 두면 올린 사용자는 `[st, sl, insane, ...]` 가 된다.
4. 내장 스냅샷(M4 추천: 두지 않음): `SNAPSHOTS` 는 발광 한 건 그대로. 근거: 결정 12 의 "항상 가져오고", Stella 정식 표는 2주마다 바뀌므로 실행 파일에 박으면 낡음, Stella 데이터를 저장소에 사본으로 두지 않음. 첫 실행 오프라인이면 Stella 폴더는 받을 때까지 없다(`NotFetched`, 표 관리 화면에 그렇게 보임). seed.rs 의 `seed_default_tables`(50-61)는 `carried` 가 없을 때 경고를 내지 않도록 "내장하지 않는 표"와 "내장했는데 읽히지 않는 표"를 나눈다: `fn carried(table) -> Carried` 가 `Carried::Snapshot(DifficultyTable) | Carried::NotCarried | Carried::Unreadable` 를 돌려주고 마지막만 경고한다.
5. 받기 경로: 주소가 `table.html` 이라 `fetch_with` 가 meta → `header.json` → `score.json` 으로 간다(2.4-4). `level_order` 가 없으니 `"0"`~`"12"` 숫자 순, symbol `st`/`sl` → 레벨 제목 `st0`~`st12`, `sl0`~`sl12`(3.1.4). 첫 실행에 `read_and_match` 가 `NotFetched` 를 적고(tablesrc.rs:440-443) 선곡의 `start_first_fetches` 가 받는다.
6. 화면: 루트 `ALL SONGS` → `発狂BMS難易度表` → `STELLA` → `MY FOLDERS`. `STELLA` 안은 `STELLA (n)`·`SATELLITE (n)`(표 이름 대문자화, 2.4-8). 경로는 `STELLA > STELLA > st3 > `(M15).
7. 출처: 표 관리 화면이 지금처럼 등록 주소(stellabms.xyz)를 보인다(결정 14 의 클라이언트 쪽). 웹 표기는 F6.

### 3.4 하루 지난 저장본 다시 받기(U3, 결정 13 의 클라이언트 쪽)

1. 상수(tablesrc.rs): `const COPY_STALE_AFTER_SECS: u64 = SECONDS_PER_DAY;`(이미 있는 `SECONDS_PER_DAY` 72행 재사용).
2. `FetchWhy` 에 `Stale` 를 더한다(504-511). `failure()` 는 `"table refresh failed"`, `tally()` 는 성공·실패 모두 `None`(M6: 조용히, 실패 이유는 표 관리 화면의 `stale` 로 보임).
3. `TableFetches` 에 `stale_checked: bool` 을 더하고, `start_first_fetches`(840-858) 끝에서 한 번만(실행마다 한 번, "시작 때") 다음 원본을 `FetchWhy::Stale` 로 시작한다: 주소 표(`is_address`)이고 `table_status` 가 `Loaded { origin: TableOrigin::Address { fetched_at }, .. }` 이며 `fetched_at` 이 `None`(내장 사본)이거나 `unix_now() - t >= COPY_STALE_AFTER_SECS`. 파일 표는 대상이 아니다. 범위는 주소 표 전부(M5).
4. 실패 처리: `apply_finished_fetches` 의 `Kept` 갈래(921-930)에서 `why == FetchWhy::Stale` 면 알림을 내지 않고 상태만 `stale: Some(reason)` 으로 적는다. 저장본과 폴더는 그대로.
5. 프레임 스레드: 받기·파싱은 지금 워커(`TableFetches::start`, 698-730)에서, 적용은 선곡·표 관리 화면에서만(2.4-7). 플레이 중에는 아무것도 적용하지 않으므로 [렉 기준 ADR](../acknowledge/2026-10-14-adr-play-hitch-criteria.md) 1항(플레이 중 프레임 스레드에 I/O·네트워크 없음)을 지킨다. 도착하면 `show_the_list_again` 이 경로와 커서를 지킨다(3.1.1).
6. 서버 부담: 사용자당 하루 한 번 표 2개(약 0.8MB + 0.9MB, 조사 A-1·A-2). ETag·Last-Modified 가 없어 조건부 요청은 쓰지 않는다.

### 3.5 없는 곡 막대(U4, 결정 16)

1. rbms-table(crates/rbms-table/src/lib.rs)에 더한다:

```rust
/// For each level of [`DifficultyTable::by_level`], the entries whose md5 the library does not hold,
/// in the table's own order, each md5 once. An entry that names a chart by sha256 alone is in
/// neither this nor [`DifficultyTable::match_levels_in`]: it cannot be told owned or not.
pub fn missing_levels_in(&self, library: &Md5Index) -> Vec<Vec<usize>>;
```

   반환은 `entries` 의 인덱스이고 `match_levels_in` 과 같은 레벨 순서·같은 길이다.
2. `TableExtras` 에 `pub(crate) missing: Vec<Vec<rbms_table::TableEntry>>`(레벨마다, `TableLevels` 와 같은 길이)를 더하고 매칭 워커가 채운다(항목 복제: Stella 4,847곡 기준 수 MB 미만).
3. `SelectItem::Missing(MissingRef)`, `pub(crate) struct MissingRef { pub(crate) table: usize, pub(crate) level: usize, pub(crate) entry: usize }`(lib.rs). `identity_of` 는 `BarIdentity::Chart(md5 소문자)` 로 가진 곡과 같은 열쇠를 쓴다. 그래서 받은 뒤 그 줄이 `Song` 이 되어도 커서가 머문다.
4. 설정 `library.show_missing_charts: bool`(기본 `true`, 원본 `Config.showNoSongExistingBar = true` Config.java:83, M7). 끄면 원본 BarManager.java:323-331 처럼 없는 곡 막대를 뺀다(설정 행은 U6).
5. `rows_of(TableLevel)`: 가진 곡은 지금처럼 `arrange_songs`, 없는 곡은 아래 규칙으로 거른 뒤 합친다.
   - 검색어: 선곡 검색은 늘 ALL SONGS(라이브러리)로 가므로(search.rs:147-156) 없는 곡은 검색에 나오지 않는다. 원본 SearchWordBar 도 곡 DB 만 찾는다(`SearchWordBar.java:24`).
   - 필터(`SelectFilter::keeps` list.rs:419-433 의 규칙을 "기록 없음, 레벨 모름, 모드 모름, 즐겨찾기 아님" 인 차트로 적용): `favorite_only` 면 빠짐, 모드가 정해졌으면 빠짐, 레벨 범위가 하나라도 있으면 빠짐(`UNKNOWN_LEVEL` 규칙과 같음), 클리어 필터 `Any`·`Unplayed`·`Unclear` 는 남고 `Cleared`·`FullCombo` 는 빠짐.
   - 정렬(원본 BarSorter.java 따름): `Default` 는 가진 곡 다음에 없는 곡을 표 순서로. `Title` 은 제목(소문자) 비교로 섞고 같으면 가진 곡이 먼저(원본 TITLE 17-39 은 없는 곡도 제목으로 비교). 그 밖의 정렬은 가진 곡을 정렬한 뒤 없는 곡을 제목 순으로 뒤에(원본 ARTIST·BPM·LEVEL 등은 없는 곡이 뒤 44-111, 기록 정렬은 기록 없는 쪽이 뒤).
6. 수: 레벨 막대의 `(n)` 과 루트·묶음 합계는 가진 곡 수 그대로(M8). 폴더 램프 분포(`distribution`)도 가진 곡만(원본 HashBar.updateFolderStatus 도 곡 DB 결과만).
7. 막대: `SelectItem::Missing` → `SelectBar { missing: Some(BarMissing { artist }), ..SelectBar::new(BarKind::Song { exists: false }, title) }`. `SelectBar` 에 `pub(crate) missing: Option<BarMissing>` 를 더한다(lib.rs:542-563). 제목이 비면 md5 를 쓴다. 램프·레벨·기능·이미지는 없다.
8. 기본(스킨 없는) 행: `SelectRow` 에 `pub missing: bool` 을 더하고(crates/rbms-render/src/select.rs:23-37), 그리는 쪽은 그 행의 글자를 `text_muted`, 램프 띠를 비우고 레벨 칸은 비운다. `SelectDetail::Missing { title: String, artist: String }` 을 더해 세부 패널에 제목·아티스트·`"NOT IN THE LIBRARY - ENTER FOR THE DOWNLOAD"` 를 보인다. 없는 곡이 없는 목록은 화소가 지금과 같다(골든 그대로).
9. 기본 스킨: 바꾸지 않는다(이미 `bar_missing`·`txt.bar.missing` 로 흐리게 그림, 2.5-2).
10. 스킨 속성: `OPTION_SONGBAR` 참, `OPTION_PLAYABLEBAR` 거짓(기존 규칙, 2.3-6). `text()`(skin_host/select.rs:420-433)에 갈래를 더해 커서가 없는 곡 막대일 때 `STRING_TITLE`·`STRING_FULLTITLE`(10·11)은 막대 제목, `STRING_ARTIST`(14)는 `BarMissing.artist` 를 준다(원본은 표 항목의 SongData 를 `resource.setSongdata` 로 보임). 그 밖의 곡 숫자·그림은 없음(원본도 경로 없는 SongData).
11. `SelectItem` 을 match 하는 곳에 갈래를 더한다: `focused_song_index`·`focused_md5`(apps/rbms-player/src/app_input.rs:596-610, `_` 로 이미 처리), `following_song`(app_result.rs:755-758, `None`), 세부 패널(scene.rs:173-240), `focused_bar`(skinned.rs:334-344: 새 `FocusedBar::Missing`), `print_selection`, seed.rs:135-140·tables.rs:867-872 의 시험 match.
12. `enter_row` 의 `Missing` 갈래: U4 에서는 알림 `"not in the library"` 만, D2 에서 모달로 바꾼다. 연습·자동·리플레이 키도 같은 길(원본 MusicSelector.java:253-262, 261행: `play != null` 이면 모드와 상관없이 `open_download_site`).

### 3.6 다운로드 폴더 설정(U6, 결정 16)

1. 스키마(`LibraryOptions`, schema.rs:850-876):

```rust
/// The folder the charts the song list downloads are unpacked into, scanned with the song folders.
#[serde(default, skip_serializing_if = "Option::is_none")]
pub download_folder: Option<String>,
/// Show the charts a difficulty table lists that the library does not have, dimmed
/// (`Config.showNoSongExistingBar`).
pub show_missing_charts: bool,
```

   `Default` 에 `None`·`true`. 스키마 버전 6 유지.
2. `impl LibraryOptions { pub fn scan_roots(&self) -> Vec<String> }`: `folders` 다음에 `download_folder` 를 붙이되, 이미 같은 경로(앞뒤 공백·끝 구분자 무시)가 `folders` 에 있으면 한 번만. 호출처: lib.rs:1016-1019(빈 판정도 `scan_roots().is_empty()`), loading.rs:417, loading.rs:564(폴더 수 표시).
3. 설정 동기화: `keep_local`(ir_sync.rs:55-70)에 `incoming.library.download_folder = local.library.download_folder.clone();` 를 더한다(기기 경로). `show_missing_charts` 는 기호이므로 동기화된다.
4. 설정 행(crates/rbms-config/src/settings.rs): `SettingId::DownloadFolder`(Select 탭, `SettingKind::Action`, `SoundFolder` 와 같은 경로 행: settings.rs:874·1135 의 방식), `SettingId::ShowMissingCharts`(Select 탭, Toggle). 화면: catalog.rs:87 의 `"SONG LIST"` 섹션에 `ShowMissingCharts`, 새 섹션 `"DOWNLOADS"` 에 `DownloadFolder`. 설명 문구(catalog.rs `description`): DownloadFolder `"The folder the song list unpacks a chart it downloads into. It is scanned with the song folders. With none chosen, the first download asks for one."`, ShowMissingCharts `"Lists the charts a difficulty table names that the library does not have, dimmed. Choosing one offers its download."`. 키 안내(info.rs:98 의 SoundFolder 줄과 같은 모양)와 `RowControl::Path`(settings_ui.rs:203), 고르기 창 `dialog::pick_download_folder()`(dialog.rs 에 제목 `"Select download folder"` 상수와 함께), 행 처리(stage/settings.rs:478·577 의 SoundFolder 와 같은 모양). settings.rs 의 전체 목록 시험(1974·2295 행 근처 목록)에도 넣는다.
5. 곡 폴더 화면(stage/folders.rs): `download_folder` 가 있으면 목록 끝(`+ ADD FOLDER` 앞)에 `"DOWNLOADS  {path}"` 행을 지울 수 없게 보인다. 그 행에서 D·Delete 는 `notify(Info, "the download folder is changed in SETTINGS > SELECT")`.
6. 첫 받기에 폴더가 없으면(M9 추천): D2 의 [GET CHART] 가 `pick_download_folder()` 를 열고, 고르면 저장한 뒤 받기를 시작한다. 닫으면 아무것도 하지 않는다.

### 3.7 없는 곡 모달(D2, 결정 15)

1. 새 파일 `apps/rbms-player/src/stage/select/download_modal.rs`:

```rust
pub(crate) struct DownloadModal {
    pub(crate) entry: rbms_table::TableEntry,
    pub(crate) table_title: String,
    pub(crate) level_title: String,
    pub(crate) chart_link: rbms_download::LinkKind,
    pub(crate) song_url: Option<String>,
    pub(crate) chart_url: Option<String>,
    pub(crate) focus: ModalButton,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ModalButton { GetChart, OpenSongUrl, OpenChartUrl, Close, Cancel }
impl DownloadModal {
    pub(crate) fn of(shared: &AppShared, missing: &MissingRef) -> Option<DownloadModal>;
    pub(crate) fn buttons(&self, job: Option<&crate::download::JobStatus>) -> Vec<(ModalButton, bool)>;
}
```

   `song_url`·`chart_url` 은 `rbms_download::browsable_url` 을 통과한 것만 `Some`.
2. `SelectState` 에 `download_modal: Option<DownloadModal>`. 열기: `enter_row` 의 `Missing` 갈래, 스킨의 재생·연습·자동·리플레이 이벤트가 `FocusedBar::Missing` 에서 온 경우. 레코드 모달과 같은 자리에서 모든 키를 먼저 받는다(mod.rs:897-906 `modal_key`, events.rs:311-315, skinned.rs `press_bars` 의 `record_modal` 처리와 나란히).
3. 화면(앱 UI 는 영어, M12): 제목, 아티스트, `"{table} {level}"`, 안내 두 줄 `"THE TABLE'S DL IS THE CHART FILE ONLY (A DIFFERENCE CHART)."`·`"THE SONG ITSELF - ITS SOUNDS AND IMAGES - HAS TO BE GOT FROM THE SONG URL AND PUT BESIDE THE CHART."`, 버튼 줄 `[GET CHART]`(직접 zip 일 때만 켜짐, 받는 중이면 `[CANCEL]` 로 바뀜) `[OPEN SONG URL]` `[OPEN CHART URL]` `[CLOSE]`, 진행 줄(3.10). 꺼진 버튼은 흐리게 그리고 커서가 건너뛴다.
4. 키: 왼쪽·오른쪽(그리고 턴테이블)으로 버튼, Enter·`FolderOpen` 키로 누름, Esc·`FolderClose` 키로 닫기. 마우스: 버튼마다 `Hot::DownloadButton(usize)`, 모달 바탕은 `Hot::OverlayPanel`(lib.rs:601-637 의 `Hot`). 스킨 위에서는 overlay.rs 가, 스킨 없는 화면에서는 기본 화면 그리기 뒤에 같은 함수 `render_download_modal(r: &mut Renderer, modal: &DownloadModal, job: Option<&JobStatus>, hot: &mut Vec<(Rect, Hot)>)` 를 부른다.
5. 모달을 열 때 디스크·네트워크를 읽지 않는다(표 항목은 메모리에 있음).

### 3.8 받기·풀기 크레이트(D1, 결정 15)

새 작업 공간 크레이트 `crates/rbms-download`(외부 의존 아님. 워크스페이스 members 와 `[workspace.dependencies]` 에 경로로 등록). 의존: `reqwest`(workspace), `encoding_rs`(workspace), `thiserror`(workspace), 그리고 승인된 새 외부 의존 하나:

```toml
zip = { version = "9.0.5", default-features = false, features = ["deflate-flate2-zlib-rs"] }
```

1. 버전 확인(crates.io API, 2026-10-23 조회): `zip` 최신 안정판 9.0.5(2026-10-17 게시), 라이선스 MIT, `rust-version` 1.88(작업 공간 1.95 이하). 9.0.5 의 필수 일반 의존은 `crc32fast ^1.5`·`indexmap ^2`·`memchr ^2.7`·`typed-path ^0.12`·`nix ^0.31.3`(fs 기능)이고, `flate2 ^1.1` 은 이 기능으로 켜진다. Cargo.lock 에 이미 있는 것: crc32fast 1.5.2, indexmap 2.14.2, memchr 2.8.3, nix 0.31.3, flate2 1.1.10, zlib-rs 0.6.8. 새로 들어오는 크레이트는 `zip` 과 `typed-path`(최신 0.12.x = 0.12.4, MIT OR Apache-2.0, 의존 없음) 둘이다. 기본 기능(aes·bzip2·lzma·zstd·ppmd·deflate64·zopfli·time)은 쓰지 않는다. `cargo tree` 는 이 설계 단계에서 실행 권한이 거부되어 돌리지 못했다. D1 첫 단계에서 `cargo tree -p rbms-download -e normal --target all --locked` 로 위 두 개만 새로 생기는지 확인하고, 다르면 멈추고 보고한다.
2. GPL-3.0-or-later 와 MIT·Apache 는 함께 배포할 수 있다. THIRD-PARTY-NOTICES.md 를 그 문서 6절 방식으로 다시 만든다(7절 문서 갱신).
3. 공개 API:

```rust
/// What a link from a table is, judged from the link alone.
pub enum LinkKind { DirectZip, Browser, Unusable }
pub fn classify_link(raw: &str) -> LinkKind;
/// `raw` as an address that may be handed to the system's browser: http or https with a host,
/// no white space or control character, at most `MAX_BROWSABLE_URL_CHARS`, normalised.
pub fn browsable_url(raw: &str) -> Option<String>;
/// A folder name made from a chart's title that every platform accepts.
pub fn safe_folder_name(title: &str, md5: &str) -> String;

pub struct DownloadLimits { pub max_bytes: u64, pub silence: Duration, pub total: Duration }
pub struct ExtractLimits { pub max_entries: usize, pub max_entry_bytes: u64, pub max_total_bytes: u64 }
pub struct Progress { pub received: AtomicU64, pub expected: AtomicU64, pub phase: AtomicU8 }
pub fn download_to(url: &str, file: &Path, limits: &DownloadLimits, progress: &Progress, cancel: &AtomicBool) -> Result<(), DownloadError>;
pub fn download_with(url: &str, file: &Path, limits: &DownloadLimits, progress: &Progress, cancel: &AtomicBool,
    get: &mut dyn FnMut(&str) -> Result<Answer, DownloadError>) -> Result<(), DownloadError>;
pub struct ExtractReport { pub written: usize, pub skipped: usize, pub charts: usize }
pub fn extract_zip(archive: &Path, into: &Path, limits: &ExtractLimits, cancel: &AtomicBool) -> Result<ExtractReport, DownloadError>;
/// Move `staging` to a new folder under `parent` named `name`, or `name (2)`, `name (3)` ... when
/// that is taken. Nothing that is there is ever overwritten.
pub fn settle(staging: &Path, parent: &Path, name: &str) -> Result<PathBuf, DownloadError>;
```

   `download_with` 는 시험이 네트워크 없이 돌도록 rbms-table `fetch_with` 와 같은 방식이다.
4. 링크 판정(`classify_link`, 조사 산출물 score.json 2350곡에 적용하면 직접 zip 1,213곡: stellabms.xyz/upload 636 + 경로가 `.zip` 577):
   - 비었거나 http·https 가 아니거나 호스트가 없으면 `Unusable`(Stella 2곡이 http 아님).
   - 호스트 `stellabms.xyz` 이고 경로가 `/upload/` 로 시작하면 `DirectZip`(조사 A-3: content-type application/zip, attachment).
   - 경로(쿼리 제외)가 대소문자 무시 `.zip` 으로 끝나면 `DirectZip`.
   - 그 밖(`.rar`·`.7z`·`.lzh`·`.lha` 41곡, 단일 `.bms` 등 123곡, HTML·클라우드 693곡)은 `Browser`(M11).
5. 받기 상수(rbms-table 의 64MB·30초와 다른 전용 상한, M13): `DOWNLOAD_MAX_BYTES = 512 MiB`, `DOWNLOAD_SILENCE = 30 s`(무응답, 클라이언트 timeout), `DOWNLOAD_TOTAL = 15 min`(읽기 사이마다 남은 시간 확인, fetch.rs:141-161 의 `read_within` 과 같은 방식), 리다이렉트 10회(reqwest 기본), 최종 주소도 http·https 여야 함, User-Agent `"rbms-download"`. `Content-Length` 가 상한을 넘으면 읽기 전에 거절. 첫 4바이트가 `PK\x03\x04`(또는 빈 보관 `PK\x05\x06`)가 아니면 `NotAZip`(HTML 응답 포함). 본문은 메모리가 아니라 임시 파일로 흘린다(64KiB 조각마다 취소 확인).
6. 풀기 상수: `EXTRACT_MAX_ENTRIES = 10_000`, `EXTRACT_MAX_ENTRY_BYTES = 1 GiB`, `EXTRACT_MAX_TOTAL_BYTES = 2 GiB`. 크기는 머리에 적힌 값이 아니라 실제로 쓴 바이트(`Read::take(남은 한도 + 1)`)로 센다. 넘으면 그 자리에서 멈추고 임시 폴더를 지운다(압축 폭탄).
7. 이름과 경로(zip-slip 차단): 항목 이름은 `ZipFile::name_raw()` 의 바이트로 읽어, UTF-8 플래그가 있거나 유효한 UTF-8 이면 UTF-8, 아니면 Shift_JIS(`encoding_rs::SHIFT_JIS`, 오류 없을 때), 그래도 안 되면 손실 UTF-8 로 해석한다(일본 BMS 압축본 이름이 CP437 로 깨지면 `#WAV` 이름과 맞지 않아 키음이 사라진다). 구분자 `/`·`\` 로 나눈 각 조각에서 다음이면 그 항목을 거절한다: 빈 조각(절대 경로 포함), `.`·`..`, `:` 포함(드라이브·대체 스트림), 제어 문자, 끝 공백·점, Windows 예약 이름(`CON`·`PRN`·`AUX`·`NUL`·`COM1`~`COM9`·`LPT1`~`LPT9`, 확장자 무시·대소문자 무시). 심볼릭 링크 항목(`unix_mode()` 의 S_IFLNK)은 건너뛴다. 합친 경로가 임시 폴더 아래인지 마지막으로 확인한다.
8. 확장자 허용 목록(그 밖은 건너뛰고 `skipped` 에 셈): 차트 `bms`·`bme`·`bml`·`pms`·`bmson`, 소리 `wav`·`ogg`·`mp3`·`flac`, 그림 `bmp`·`png`·`jpg`·`jpeg`·`gif`, 영상 `mpg`·`mpeg`·`avi`·`mp4`·`wmv`·`m4v`·`webm`·`mkv`, 글 `txt`. 실행 파일·스크립트는 쓰지 않는다. 차트가 하나도 없으면 `NoChart` 로 실패하고 임시 폴더를 지운다(M20).
9. 덮어쓰기: 풀기는 늘 새 임시 폴더 `{download_folder}/.rbms-partial/{md5}/`(일을 시작할 때 `.rbms-partial` 아래를 비움, 동시에 한 일만 있음)에 하고, 끝나면 `settle` 로 `{download_folder}/{safe_folder_name}` 로 옮긴다. 같은 이름이 있으면 ` (2)`, ` (3)` … 를 붙인다. 이미 있는 파일은 절대 덮지 않는다.
10. `safe_folder_name`: 제목에서 허용하지 않는 문자(`<>:"/\|?*`·제어 문자)를 `_` 로, 앞뒤 공백·점 제거, 64자(문자 수) 자르기, 끝에 `" [{md5 앞 8자}]"`, 비거나 예약 이름이면 md5 앞 8자만.

### 3.9 브라우저 열기(D2, 결정 15)

새 파일 `apps/rbms-player/src/open_url.rs`:

```rust
/// The program and the arguments that hand `url` to this platform's browser. `url` has been through
/// [`rbms_download::browsable_url`]: it is the last argument, alone, and no shell reads it.
pub(crate) fn opener_command(url: &str) -> (&'static str, Vec<String>);
/// Hand `url` to the browser on a thread of its own, which waits for the opener so it leaves no
/// process behind. Nothing is done with an address `browsable_url` refuses.
pub(crate) fn open_in_browser(raw: &str);
```

1. macOS `("open", [url])`, Linux 등 그 밖의 유닉스 `("xdg-open", [url])`.
2. Windows(M10 추천): `("rundll32", ["url.dll,FileProtocolHandler", url])`. 과제 문구의 `start` 는 cmd 의 내장 명령이라 `cmd /C start "" <url>` 로만 부를 수 있는데, cmd 는 `&`·`|`·`^`·`%` 를 해석한다. Stella 곡 URL 2350개 가운데 1,492개가 `&` 나 `%` 를 담는다(조사 산출물에서 계산, 예: manbow `event.cgi?action=More_def&num=94&event=146`). rundll32 는 셸을 거치지 않는다. 메인이 `start` 를 고르면 `std::os::windows::process::CommandExt::raw_arg` 로 따옴표를 직접 쓰고 `"`·`%`·`^` 가 든 URL 은 퍼센트 인코딩된 형태만 넘기는 별도 규칙이 필요하다.
3. `Command::spawn` 은 프레임 스레드가 아니라 짧은 스레드에서 하고 `wait()` 로 거둔다. 실패는 `notify(Warn, "could not open the browser: {reason}")`.
4. 시험용: `opener_command` 는 순수 함수라 플랫폼별 인자를 시험한다. 실제 실행은 시험하지 않는다(9절 부채).

### 3.10 받기 일과 부분 재스캔(D3, 결정 15·16)

1. 새 파일 `apps/rbms-player/src/download.rs`:

```rust
pub(crate) struct Downloads { job: Option<DownloadJob>, refresh: Option<LibraryRefresh> }
pub(crate) struct JobStatus { pub(crate) phase: Phase, pub(crate) received: u64, pub(crate) expected: Option<u64>, pub(crate) md5: String }
pub(crate) enum Phase { Downloading, Extracting, Scanning, Done { found: bool, charts: usize }, Failed(String), Cancelled }
pub(crate) fn start_download(shared: &mut AppShared, entry: &rbms_table::TableEntry) -> Result<(), &'static str>;
pub(crate) fn cancel_download(shared: &mut AppShared);
pub(crate) fn status(shared: &AppShared) -> Option<JobStatus>;
/// Take what the download and the scan after it came back with. Called by the browser only.
pub(crate) fn apply_finished(shared: &mut AppShared) -> Option<LibraryChange>;
pub(crate) struct LibraryChange;
```

   `AppShared` 에 `downloads: download::Downloads` 와 `library_generation: u64` 를 더한다.
2. 순서: (1) 프레임 스레드는 링크가 `DirectZip` 이고 다운로드 폴더가 있고 다른 일이 없을 때만 워커를 띄운다(동시에 한 일, 다른 일이 있으면 버튼이 꺼지고 `"ANOTHER DOWNLOAD IS RUNNING"`). (2) 워커: `.rbms-partial` 비우기 → `download_to`(임시 zip 파일) → `extract_zip` → `settle` → 끝. (3) 선곡의 `update`(mod.rs:1046-1053, `apply_finished_fetches` 옆)가 `apply_finished` 로 일이 끝난 것을 받으면 `LibraryRefresh` 를 띄운다. (4) `LibraryRefresh` 워커: `LIBRARY_WRITE` 를 잡고 `scan_now(db, ScanRequest::partial(vec![final_dir]))` → `read_and_match_all(store, 표 원본 사본, &library)` → 결과(새 라이브러리, `Vec<MatchedTable>`, 시작할 때의 `library_generation`)를 보낸다. (5) 선곡이 결과를 받으면 세대가 같을 때만 적용하고, 다르면(그 사이 전체 스캔이 라이브러리를 바꿈) 같은 폴더로 `LibraryRefresh` 를 다시 띄운다.
3. 적용(프레임 스레드, 선곡에서만): `let place = ListPlace::of(shared)` → `shared.library` 교체 → 표 세 벡터 교체 → `user_folders::rematch(shared)` → `shared.trace_backfill_after_scan()` → 표 코스 재계산 → `library_generation += 1` → `show_the_list_again(shared, &place)`. `SelectState::library_replaced(&mut self, shared)` 가 인덱스로 기억하던 것을 비운다: `focused_detail`·`focused_detail_si`·`focus_settle_si`·`cover_image`·`cover_si`·`cached_scene`·`cached_key`·`loaded_backbmp`·`focused_records`·`selected_replay`·`course_shown`, `facts_generation` 올림(다음 프레임에 `ChartFacts` 다시 읽음), `refresh_courses`. 전체 스캔 적용(loading.rs `apply_scan`)도 `library_generation += 1`.
4. 결과 알림: 라이브러리에 그 md5 가 생기면 `"downloaded: {title} ({charts} chart(s))"`, 차트는 있었으나 그 md5 가 없으면 `"the archive had {charts} chart(s) but not this one"`(폴더는 남김, M19), 실패는 `"download failed: {reason}"`. 모달이 열려 있으면 진행 줄도 같은 말.
5. 진행 줄: `"DOWNLOADING {received} / {expected}"`(크기는 KB·MB 로, `expected` 모르면 받은 양만), `"EXTRACTING"`, `"SCANNING"`, 끝 메시지. 값은 워커의 원자값을 프레임마다 읽는 것뿐이다.
6. 동시 쓰기 차단: `apps/rbms-player/src/library.rs` 에 `pub(crate) static LIBRARY_WRITE: Mutex<()>` 를 두고 `scan_now`(library.rs:217-232)가 처음에 잡는다. 전체 스캔(로딩 화면)과 부분 스캔이 같은 경로를 지나므로 둘이 동시에 곡 DB 를 쓰지 않는다. 잡는 쪽은 늘 워커 스레드다(프레임 스레드는 기다리지 않음).
7. 부분 스캔의 파서 버전: `ScanRequest` 에 `pub stamps_parser_version: bool` 을 더하고(`new` 는 `true`), `ScanRequest::partial(roots)` 는 `false`. `scan_into` 는 이 값이 참일 때만 `set_parser_version`(scan.rs:149)을 부른다. 그래야 부분 스캔이 다른 폴더의 전체 재파싱 필요를 지우지 않는다(2.6-2).
8. 플레이 중에 끝난 일: 워커의 결과는 채널에 남고 선곡으로 돌아왔을 때 적용한다(플레이 중 프레임 스레드에서 라이브러리를 바꾸지 않음).
9. 앱 종료: 워커를 기다리지 않는다. 남은 `.rbms-partial` 은 다음 일이 시작할 때 비운다.

### 3.11 호환성·마이그레이션 요약

| 대상 | 변경 | 호환 |
| --- | --- | --- |
| 설정 파일(settings.ron) | `TableSource.group`(빈 값 미기록), `library.download_folder`(None 미기록), `library.show_missing_charts`(기본 true), `seeded_default_tables` 에 `stella-st`·`stella-sl` 추가 | 스키마 버전 6 유지, 옛 빌드는 모르는 필드 무시, 마이그레이션 없음 |
| 표 저장본(tables/*.json) | 없음 | 그대로 |
| 곡 DB(songdb.sqlite) | 스키마 없음. 부분 스캔이 행을 더할 뿐 | 그대로 |
| 점수 DB·리플레이·IR 와이어 | 없음 | `contract/client-requests` 영향 없음 |
| 스킨 속성 id | 새 id 없음, 확장 대역 사용 없음 | 기존 스킨 그대로 |
| 설정 동기화 blob | `download_folder` 는 기기 고유로 제외, `show_missing_charts` 는 동기화 | 옛 빌드 blob 은 필드 없음 → 기본값 |
| 의존성 | `zip` 9.0.5(승인), `typed-path` 0.12.4(그 의존), 새 작업 공간 크레이트 `rbms-download` | `--locked`, Cargo.lock 커밋 |
| 판·기록 저장/전송 | 없음 | 그대로 |

## 4. 원본 대비 차이 표

"판정·점수" 열이 "없음" 이 아닌 행은 판정·점수 의미에 닿는다. 이 묶음에는 그런 행이 없고, 표 코스의 게이지·제약은 F5 경계다.

| 항목 | 원본(beatoraja, 그다음 LR2·IIDX) | R-BMS 설계 | 근거 | 판정·점수 |
| --- | --- | --- | --- | --- |
| 표 코스 위치 | TableBar 안 레벨 뒤에 GradeBar 를 나열(TableBar.java:80) | 표 안의 `COURSE` 폴더 | 결정 11 | 없음(코스 판정은 F5) |
| 표 묶음 폴더 | 없음 | `STELLA` 묶음(ContainerBar 처럼 Command) | 결정 11·12 | 없음 |
| 커서 복원 열쇠 | 있는 곡은 sha256, 그 밖은 같은 클래스+같은 제목(BarManager.java:430-446) | 곡·없는 곡 모두 md5, 폴더는 폴더 정체(표 주소·레벨 이름·폴더 id), 코스는 이름 | 라이브러리가 md5 색인(2.1-6). 같은 이름의 표 두 개를 구분 | 없음 |
| 없는 곡 열쇠 | 같은 클래스+제목(있는 곡과 섞일 수 있음) | md5(받은 뒤에도 커서 유지) | 3.5-3 | 없음 |
| 이탈 뒤 못 찾음 | 0 | 0(같음). 표 갱신·검색 복원은 지금 R-BMS 규칙(이전 줄 위치) 유지 | 결정 10 / 기존 동작 | 없음 |
| 빈 폴더 진입 | 막대가 0개면 들어가지 않음(BarManager.java:468-474) | 지금처럼 빈 목록으로 들어감(M17) | 기존 R-BMS | 없음 |
| 전체 스캔 뒤 위치 | 현재 dir 유지(`updateBar()`) | 경로 보존(M16). 지금은 루트 | 원본 따름 | 없음 |
| 레벨 폴더 제목 | `symbol + level`(TableDataAccessor.java:185) | 같음, symbol 없으면 `LV n`(M2). 수 `(n)` 덧붙임은 기존 R-BMS | 원본 따름 | 없음 |
| 표 이름 대문자화 | 그대로 | 라틴 문자 대문자(기존, tablesrc.rs:402-404) | 기존 R-BMS | 없음 |
| 없는 곡 막대 기본 | 보임(`showNoSongExistingBar = true`) | 보임, 설정으로 끔(M7) | 원본 따름 | 없음 |
| 없는 곡 고르기 | `open_download_site`: url 과 appendurl 을 둘 다 브라우저로(EventFactory.java:457-484) | 모달: zip 받기 / 곡 URL / 차분 URL / 닫기 | 결정 15 | 없음 |
| 없는 곡 필터 | 모드·난이도 필터로 빠짐(BarManager.java:333-371) | `SelectFilter` 규칙(즐겨찾기·모드·레벨 범위·Cleared·FullCombo 면 빠짐) | 3.5-5 | 없음 |
| 없는 곡 정렬 | TITLE 은 섞음, 그 밖은 뒤(BarSorter.java) | 같음, Default(정렬 없음)는 가진 곡 뒤 | 원본 따름 | 없음 |
| 검색 | 곡 DB 만 찾음 | 같음(없는 곡 안 나옴) | 원본과 같음 | 없음 |
| 표 매칭 | md5 또는 sha256(SongBar.java:123-126) | md5 만(기존). sha256 만 있는 항목은 가진 곡도 없는 곡도 아님 | 기존 R-BMS, Stella 는 모두 md5 있음 | 없음 |
| 자동 표 다시 받기 | 없음(사용자가 갱신) | 시작 때 하루 지난 주소 표 | 결정 13 | 없음 |
| 기본 표 | 없음 | 발광·Stella·Satellite | 결정 12 | 없음 |
| 레벨 폴더 수 | 없음 | 가진 곡 수 `(n)`(M8, 기존) | 기존 R-BMS | 없음 |

## 5. 구현 단위 목록

공통 규칙: 모든 단위는 HANDOFF 5절(Rust `//` 주석 금지·`///` 영어, 매직넘버·새 `#[allow]`·`unsafe`·`todo!()` 금지, rustfmt 160, `--locked`)을 따르고, 소유하지 않은 파일은 읽기만 한다. 커밋·푸시는 메인이 한다. 표에 적힌 "겹침"은 다른 묶음(F2 클라이언트 작은 묶음, F5 코스 게이지·코스 원천)도 고칠 가능성이 높은 파일이다.

### U1. 폴더 스택·커서 복원·표 묶음·레벨 제목(결정 10·11)

- 소유(수정): apps/rbms-player/src/select_tree.rs(새), apps/rbms-player/src/lib.rs(`SelectView` 삭제, `SelectItem`, `AppShared` 필드 `select_path`·`table_extras`, `App::new` 초기화, `remove_table_slot`), apps/rbms-player/src/tablesrc.rs(`MatchedTable`·`TableExtras`, `read_and_match*`, `set_table_slot`, `ListPlace` 이동, `FocusedFolder` 삭제), apps/rbms-player/src/stage/select/{mod.rs, list.rs, scene.rs, search.rs, skinned.rs, events.rs}, apps/rbms-player/src/stage/select/tree_tests.rs(새), apps/rbms-player/src/stage/select/{bars_tests.rs, events_tests.rs, lent_tests.rs, overlay_tests.rs, panel_tests.rs, recorded_tests.rs, search_tests.rs, skinned_tests.rs, table_tests.rs, tests.rs, user_folder_tests.rs}, apps/rbms-player/src/stage/select/skin_writes/tests.rs, apps/rbms-player/src/skin_host/select/bridge.rs, apps/rbms-player/src/skin_host/select/tests.rs, apps/rbms-player/src/stage/decide.rs, apps/rbms-player/src/stage/decide/tests.rs, apps/rbms-player/src/stage/loading.rs, apps/rbms-player/src/stage/tables.rs, apps/rbms-player/src/user_folders.rs, apps/rbms-player/src/user_folders/tests.rs, apps/rbms-player/src/app_play.rs(`print_selection`, 문서 주석), apps/rbms-player/src/app_result.rs(시험의 `SelectItem::Folder`), apps/rbms-player/src/app_options.rs(시험), apps/rbms-player/src/pointer/tests.rs, apps/rbms-player/src/stage/render_tests_select.rs, apps/rbms-player/src/stage/capture.rs, apps/rbms-player/src/stage/capture/{gate.rs, search.rs, panels.rs}, apps/rbms-player/src/tablesrc/seed.rs(시험의 `SelectItem` match 와 `read_and_match_all` 반환형만), crates/rbms-config/src/schema.rs(`TableSource.group` 만), crates/rbms-table/src/lib.rs(`named_symbol`, `ABSENT_SYMBOL`).
- 읽기 전용: crates/rbms-render/src/skin_render/songlist/bars.rs, apps/rbms-player/src/skin_host/select.rs, apps/rbms-player/src/skin_host/select/shown.rs, apps/rbms-player/src/course_ui.rs, beatoraja BarManager.java.
- 선행: 없음. D1 과 병렬.
- 겹침: stage/select/mod.rs·skinned.rs·events.rs(F2 의 Esc 결과 화면·패널 키 opt-in 이 select 키 처리를 고칠 수 있음), lib.rs, schema.rs(F2 의 SCRATCH AUTO 색, F5 의 코스 게이지 설정), stage/loading.rs. 메인은 U1 을 F2 의 select 쪽 변경과 같은 웨이브에 두지 않는다.
- 크기: 큼(호출처 수십 곳의 기계적 치환 포함). 한 작업자에게 맡긴다.

### U2. Stella·Satellite 기본 표(결정 12)

- 소유: crates/rbms-config/src/tables.rs(`DefaultTable.group`, 상수, `DEFAULT_TABLES`, 넣는 순서 규칙, 시험), apps/rbms-player/src/tablesrc/seed.rs(`Carried`, 경고 규칙, 시험), crates/rbms-config/src/lib.rs(새 상수 다시 내보내기가 필요하면), crates/rbms-table/src/fetch/tests.rs(Stella 모양 합성 고정물 시험).
- 읽기 전용: tablesrc.rs, schema.rs.
- 선행: U1(`TableSource.group`).
- 겹침: 없음.
- 병렬: U3·U6 과 병렬 가능(파일이 겹치지 않음).

### U3. 하루 지난 표 다시 받기(결정 13 클라이언트)

- 소유: apps/rbms-player/src/tablesrc.rs(받기 부분: `FetchWhy::Stale`, `stale_checked`, `start_first_fetches`, `apply_finished_fetches` 의 `Kept` 갈래, 시험).
- 읽기 전용: apps/rbms-player/src/stage/tables.rs, stage/select/mod.rs.
- 선행: U1(tablesrc.rs 의 같은 파일을 U1 이 크게 고침).
- 겹침: 없음.
- 병렬: U2·U6 과 병렬.

### U6. 다운로드 폴더·없는 곡 보이기 설정(결정 16)

- 소유: crates/rbms-config/src/schema.rs(`LibraryOptions` 두 필드, `scan_roots`), crates/rbms-config/src/settings.rs(`SettingId` 두 개, 행, 값 읽기·쓰기, 목록 시험), crates/rbms-config/src/tests.rs, apps/rbms-player/src/settings_ui.rs, apps/rbms-player/src/stage/settings.rs, apps/rbms-player/src/stage/settings/{catalog.rs, info.rs}과 그 시험 파일(tests.rs, view_tests.rs, layout_tests.rs 중 행 수를 단언하는 것), apps/rbms-player/src/dialog.rs(`pick_download_folder`), apps/rbms-player/src/ir_sync.rs, apps/rbms-player/src/stage/folders.rs, apps/rbms-player/src/stage/loading.rs(`rescan`·진행 문구), apps/rbms-player/src/lib.rs(시작 스캔 1016-1019 만).
- 읽기 전용: tablesrc.rs, stage/select/list.rs.
- 선행: U1(loading.rs·lib.rs 같은 파일).
- 겹침: schema.rs·settings.rs·catalog.rs·info.rs(F2 의 SCRATCH AUTO 색 설정, F5 의 코스 게이지 설정 행). 메인은 F2·F5 의 설정 행 단위와 같은 웨이브에 두지 않거나, 한 작업자가 차례로 하게 한다.
- 병렬: U2·U3 과 병렬.

### U4. 없는 곡 막대(결정 16)

- 소유: crates/rbms-table/src/lib.rs(`missing_levels_in`, 시험), apps/rbms-player/src/tablesrc.rs(`TableExtras.missing`), apps/rbms-player/src/lib.rs(`SelectItem::Missing`, `MissingRef`, `SelectBar.missing`, `BarMissing`), apps/rbms-player/src/select_tree.rs(`identity_of` 갈래), apps/rbms-player/src/stage/select/{list.rs, scene.rs, mod.rs(`enter_row` 갈래, 알림만), skinned.rs(`FocusedBar::Missing`)}, apps/rbms-player/src/stage/select/missing_tests.rs(새), apps/rbms-player/src/skin_host/select.rs(`text()` 갈래), apps/rbms-player/src/skin_host/select/tests.rs, apps/rbms-player/src/app_result.rs(`following_song` 갈래), apps/rbms-player/src/app_play.rs(`print_selection` 갈래), apps/rbms-player/src/tablesrc/seed.rs·apps/rbms-player/src/stage/tables.rs(시험 match 갈래만), crates/rbms-render/src/select.rs(`SelectRow.missing`, `SelectDetail::Missing`, 그리기), crates/rbms-render/tests/golden_select.rs, crates/rbms-render/examples/render_select.rs, crates/rbms-render/tests/golden/(새 골든 그림 1장).
- 읽기 전용: crates/rbms-config/src/schema.rs(`show_missing_charts`), bars.rs, assets/skins/rbms-default/lua/select/list.lua.
- 선행: U1, U6(`show_missing_charts`).
- 겹침: rbms-render/src/select.rs(F2 가 기본 화면을 고치면).
- 병렬: 없음(선곡 파일을 크게 고침).

### U5. 표 코스 폴더(결정 11)

- 소유: apps/rbms-player/src/tablesrc.rs(`TableExtras.courses`, 매칭 워커의 `courses_of_table` 호출), apps/rbms-player/src/lib.rs(`SelectItem::Course`, `CourseRef`), apps/rbms-player/src/select_tree.rs(`TableCourses` 의 `is_there`·`title`, `BarIdentity::Course`), apps/rbms-player/src/stage/select/{list.rs(`course_bar` 추출, 목록), mod.rs(`enter_row` 갈래)}, apps/rbms-player/src/course_ui.rs(`CourseEntry::resolved` 생성자만), apps/rbms-player/src/stage/select/tree_tests.rs(코스 시험 추가).
- 읽기 전용: crates/rbms-course/src/*(F5 소유), docs/plan/2026-10-23-course-gauge-data.md.
- 선행: U4(같은 파일), 코스 묶음 설계의 `courses_of_table` 구현 단위.
- 겹침: course_ui.rs(F5 가 코스 게이지·제출을 고침). `CourseEntry::resolved` 는 F5 단위가 먼저 만들었으면 그것을 쓴다.
- 병렬: D2·D3 과는 직렬(mod.rs·lib.rs 같은 파일). F5 가 늦으면 D2·D3 다음에 한다.

### D1. 받기·풀기 크레이트(결정 15)

- 소유: crates/rbms-download/(새: Cargo.toml, src/lib.rs, src/link.rs, src/fetch.rs, src/extract.rs, src/name.rs, src/tests/*.rs), Cargo.toml(작업 공간 `members` 와 `[workspace.dependencies]` 에 `rbms-download`·`zip`), Cargo.lock.
- 읽기 전용: crates/rbms-table/src/fetch.rs(같은 읽기 방식 참고), THIRD-PARTY-NOTICES.md.
- 선행: 없음. U1 과 병렬.
- 겹침: Cargo.toml·Cargo.lock(다른 단위가 의존성을 바꾸지 않으므로 사실상 없음).
- 첫 단계: `cargo tree` 로 새 크레이트가 `zip`·`typed-path` 둘뿐인지 확인(3.8-1).

### D2. 모달·브라우저 열기(결정 15)

- 소유: apps/rbms-player/src/stage/select/download_modal.rs(새), apps/rbms-player/src/stage/select/download_modal_tests.rs(새), apps/rbms-player/src/open_url.rs(새), apps/rbms-player/src/lib.rs(`Hot::DownloadButton`, `mod open_url;`), apps/rbms-player/src/stage/select/{mod.rs(모달 열기·키), events.rs, skinned.rs, overlay.rs, scene.rs(기본 화면 위 그리기)}, apps/rbms-player/Cargo.toml(`rbms-download` 경로 의존).
- 읽기 전용: crates/rbms-download, download.rs(D3 전에는 `job` 없이 `None` 으로 그림).
- 선행: U4(없는 곡 항목), D1.
- 겹침: overlay.rs·events.rs(F2 의 패널 키 opt-in).
- 병렬: 없음.

### D3. 받기 일·부분 재스캔(결정 15·16)

- 소유: apps/rbms-player/src/download.rs(새), apps/rbms-player/src/download/tests.rs(새), apps/rbms-player/src/library.rs(`LIBRARY_WRITE`), crates/rbms-library/src/scan.rs(`stamps_parser_version`, `ScanRequest::partial`, 시험), apps/rbms-player/src/lib.rs(`AppShared.downloads`·`library_generation`), apps/rbms-player/src/stage/loading.rs(`apply_scan` 의 세대 증가), apps/rbms-player/src/stage/select/{mod.rs(`apply_finished` 호출, `library_replaced`), download_modal.rs(시작·취소·진행 줄)}.
- 읽기 전용: crates/rbms-download, tablesrc.rs, user_folders.rs.
- 선행: D2, U6(다운로드 폴더).
- 겹침: loading.rs.
- 병렬: 없음.

권장 웨이브: 1) U1 ∥ D1 → 2) U2 ∥ U3 ∥ U6 → 3) U4 → 4) D2 → 5) D3 → 6) U5(F5 의 변환이 들어온 뒤, 더 일찍 들어오면 3) 과 4) 사이).

## 6. 테스트 계획

### 6.1 새 시험(이름과 단언)

U1(apps/rbms-player/src/stage/select/tree_tests.rs, 주로 기존 `browser`·`tap` 도구 재사용):

1. `leaving_a_level_puts_the_cursor_on_that_level`: 표 하나(레벨 1·2·3) → 표 → 세 번째 레벨 → Esc. 경로가 `[Table]`, `sel == 2`, `identity_of(select_items[sel]) == Opens(TableLevel(t, "3"))`.
2. `leaving_a_table_puts_the_cursor_on_that_table_at_the_root`: 표 두 개, 두 번째 표 들어갔다 Esc → 루트, `sel == 2`(ALL SONGS, 표1, 표2).
3. `leaving_my_folders_and_a_folder_of_them_puts_the_cursor_on_each`: MY FOLDERS > 두 번째 폴더 → Esc 두 번, 각각 `sel == 1`, 루트에서 `sel == MY FOLDERS 행`.
4. `every_way_out_of_a_folder_puts_the_cursor_on_the_same_bar`: Escape, ArrowLeft, 검은 건반(`BLACK_CLOSE_LANES`), 스킨 FolderClose 키, `SkinAction::CloseBar` 다섯 길 모두 같은 `sel`·경로.
5. `a_folder_whose_bar_is_gone_when_left_puts_the_cursor_first`: 레벨 안에서 표를 다시 받아 그 레벨이 사라진 표로 바꾼 뒤(경로는 `show_the_list_again` 이 자름) 남은 프레임에서 나가면 `sel == 0`.
6. `a_group_holds_its_tables_where_its_first_table_is`: 설정 표 `[A, S1(STELLA), B, S2(STELLA)]` → 루트 행 `["ALL SONGS (n)", "A (n)", "STELLA (n)", "B (n)"]`, STELLA 안 `["S1 (n)", "S2 (n)"]`.
7. `a_group_none_of_whose_tables_has_levels_is_no_row`.
8. `a_level_is_titled_with_the_symbol_its_table_names`: symbol `st` → `"st3 (n)"`, 헤더 없는 표 → `"LV 3 (n)"`.
9. `the_directory_string_is_the_titles_of_the_open_folders`: STELLA > S1 > st3 에서 `STRING_DIRECTORY == "STELLA > S1 > st3 > "`(skin_host/select/tests.rs 에 둬도 됨).
10. `a_chart_played_inside_a_nested_level_comes_back_to_it`: 레벨 안 두 번째 곡에서 플레이 → `leave_play` → 경로·`sel` 같음.
11. `a_search_made_in_a_nested_level_is_left_for_that_level`: 경로 3단 → `/` → 단어 → Esc → 경로·`sel` 복원.
12. `a_table_arriving_keeps_a_nested_path_and_its_cursor`: 묶음 안 표의 레벨 안에서 다른 표 도착(`apply_finished_fetches`) → 경로·`sel` 같음.
13. `a_table_removed_from_the_list_cuts_the_path_back_to_what_is_there`: 표 관리 화면에서 열린 표를 지우고 Esc → 루트, 다른 표 안이었다면 그 표에 남음.
14. `a_rescan_keeps_the_open_folder_and_the_chart_under_the_cursor`: `apply_scan` 뒤 경로 같음, 커서 곡 md5 같음.
15. `two_tables_of_the_same_name_are_told_apart_by_their_address`: 같은 이름 두 표 중 두 번째에 들어갔다 나오면 `sel == 2`.
16. `the_decide_screen_names_the_innermost_table_and_level`: 묶음 > 표 > `st3` 에서 고른 곡의 `(table_name, table_level) == ("S1", "st3")`.
17. crates/rbms-table: `named_symbol_is_none_without_a_header_symbol`(`"*"` 대체값·빈 값은 `None`, `"★"` 는 `Some`).

U2:

1. crates/rbms-config/src/tables.rs `stella_and_satellite_are_known_at_their_page_header_and_body`: `/st/table.html`·`/st/`·`/st/header.json`·`/st/score.json`·`http://` 형태가 st 로, `/sl/table.html` 은 st 가 아님.
2. `the_default_ids_and_addresses_are_the_released_ones`: 세 id·location·group 문자열을 그대로 단언(릴리스 뒤 바뀌면 실패).
3. `a_new_player_is_given_every_default_table_once_in_order`: `tables == [insane, st(STELLA), sl(STELLA)]`, 두 번째 호출은 빈 목록.
4. `a_player_given_the_insane_table_before_is_given_stella_after_it`: `seeded == ["insane1"]`, `tables == [insane, mine]` → `[insane, st, sl, mine]`(M3).
5. `a_player_who_registered_stella_by_hand_is_not_given_a_second_copy`: `tables == ["https://stellabms.xyz/st/"]` → st 는 기록만, sl 은 추가.
6. seed.rs `stella_is_not_carried_and_waits_for_its_first_fetch`: 첫 실행 seed 후 st·sl 저장본 없음, 알림 없음, `read_and_match_all` 뒤 `table_status == NotFetched`.
7. seed.rs `every_carried_snapshot_reads`(아래 뒤집히는 시험을 대신함): `SNAPSHOTS` 마다 읽히고 비지 않음, 발광 1,035곡·26레벨.
8. crates/rbms-table/src/fetch/tests.rs `a_stella_shaped_table_is_read_through_its_page`: 합성 page(meta `header.json`) → header(`data_url: "score.json"`, level_order 없음, course 배열의 배열) → body(level `"0"`·`"12"`·`"3"`) 에서 `level_order == ["0","3","12"]`, `symbol == "st"`, `course.is_some()`.
9. schema `a_table_source_with_no_group_is_written_as_before`: RON 에 `group` 이 없음, 있는 group 은 왕복.

U3(tablesrc.rs 안 시험):

1. `a_copy_older_than_a_day_is_fetched_again_once_a_run`: `fetched_at = now - 2일` → `start_first_fetches` 두 번 불러도 시작 한 번, `FetchWhy::Stale`.
2. `a_copy_younger_than_a_day_is_left_alone`.
3. `a_carried_copy_that_has_no_date_is_fetched_again`.
4. `a_refresh_that_fails_keeps_the_copy_and_says_nothing`: 거절하는 fetcher → 알림 0, `TableStatus::Loaded { stale: Some(_) }`, 폴더 같음.
5. `a_table_file_is_never_fetched_again_for_its_age`.
6. `a_refresh_that_lands_keeps_the_browser_where_it_is`(U1 12번과 같은 단언, `FetchWhy::Stale` 로).

U6:

1. crates/rbms-config `the_download_folder_is_scanned_after_the_song_folders`, `a_download_folder_listed_already_is_scanned_once`, `an_unset_download_folder_is_not_written`, `missing_charts_are_shown_by_default`.
2. ir_sync.rs `the_download_folder_stays_on_this_machine`(업로드 blob 에 없음, 내려받은 값이 로컬을 덮지 않음).
3. stage/settings 시험: `the_download_folder_row_opens_a_picker_and_zero_clears_it`, `the_missing_charts_row_toggles`.
4. stage/folders.rs `the_download_folder_is_listed_and_not_removed_here`.
5. stage/loading.rs `a_rescan_walks_the_download_folder_too`(`rescan` 의 roots 단언).

U4:

1. crates/rbms-table `missing_levels_list_what_the_library_lacks_in_level_order`, `an_entry_named_by_sha256_alone_is_neither_owned_nor_missing`, `a_missing_md5_listed_twice_in_a_level_is_one_bar`.
2. missing_tests.rs: `a_level_lists_its_missing_charts_after_the_owned_ones`(Default), `title_order_mixes_missing_and_owned_charts`, `other_orders_put_missing_charts_last`(Level·Clear·Artist), `favourites_a_mode_or_a_level_range_hide_missing_charts`, `not_cleared_keeps_missing_charts_and_cleared_hides_them`, `the_level_count_counts_owned_charts_only`, `hiding_missing_charts_in_the_settings_takes_their_bars_away`, `a_missing_chart_is_a_song_bar_not_on_disk`(`BarKind::Song { exists: false }`, `level_index == None`, 램프 없음), `the_cursor_on_a_missing_chart_stays_when_it_becomes_owned`(라이브러리에 그 md5 를 넣고 `show_the_list_again` → 같은 행이 `Song`), `entering_a_missing_chart_starts_nothing`(U4 단계: `Transition::Stay`, 알림 한 줄), `the_next_song_after_a_run_skips_missing_charts`.
3. skin_host/select/tests.rs: `a_missing_bar_is_a_song_bar_that_cannot_be_played`(`OPTION_SONGBAR` 참, `OPTION_PLAYABLEBAR` 거짓), `a_missing_bar_names_its_title_and_artist`(10·11·14).
4. crates/rbms-render/tests/golden_select.rs `built_in_list_with_missing_charts`(새 골든, `RBMS_GOLDEN_UPDATE=1` 로 한 번 만들고 메인이 그림을 본 뒤 커밋). 기존 골든은 그대로 통과해야 한다.

U5(tree_tests.rs):

1. `a_table_that_lists_courses_has_a_course_folder_after_its_levels`(`"COURSE (26)"` 는 고정물 수로).
2. `a_course_whose_charts_are_all_owned_starts_and_one_missing_a_chart_says_so`.
3. `leaving_the_course_folder_puts_the_cursor_on_it`.
4. `a_table_course_folder_bar_is_a_table_bar_and_its_courses_are_course_bars`.

D1(crates/rbms-download/src/tests/):

1. 링크: `stella_uploads_and_zip_paths_are_direct_archives`, `rar_7z_lzh_single_charts_pages_and_cloud_links_go_to_the_browser`, `a_query_does_not_make_a_page_a_zip`(`page.html?f=a.zip` 은 Browser), `a_link_that_is_not_http_is_unusable`.
2. 주소: `only_http_and_https_with_a_host_are_browsable`, `control_characters_white_space_and_overlong_urls_are_refused`, `javascript_file_and_data_urls_are_refused`, `a_browsable_url_never_starts_with_a_dash`.
3. 받기(`download_with`): `a_download_past_the_cap_is_stopped_and_leaves_no_file`, `a_content_length_past_the_cap_is_refused_before_reading`, `a_server_that_goes_silent_runs_out_of_time`(남은 시간 함수 주입), `an_html_answer_is_not_a_zip`, `a_cancelled_download_leaves_nothing`, `a_redirect_to_another_scheme_is_refused`.
4. 풀기: `entries_that_climb_out_are_refused`(`../a.bms`, `/abs.bms`, `C:x.bms`, `a\..\..\b.bms`), `link_entries_are_skipped`, `shift_jis_names_are_read_as_shift_jis`, `utf8_names_are_kept`, `files_of_other_kinds_are_skipped_and_counted`, `an_archive_that_unpacks_past_the_cap_is_stopped_and_removed`, `too_many_entries_are_refused`, `windows_device_names_are_refused`, `an_archive_with_no_chart_is_discarded`, `a_folder_that_is_there_is_never_overwritten`(` (2)`).
5. 이름: `a_title_becomes_a_folder_name_every_platform_takes`(예약 이름, 금지 문자, 64자 자름, 빈 제목 → md5).
6. 고정물: 시험 안에서 `zip::ZipWriter`(Stored·Deflated)로 만든다. 선택한 기능에서 쓰기가 안 되면 `crates/rbms-download/tests/fixtures/*.zip`(수 KB, 직접 만든 무해한 파일)로 바꾸고 보고한다.

D2:

1. download_modal_tests.rs `entering_a_missing_chart_opens_the_modal`(Enter, 기본 키 Play, 연습, 자동 각각), `get_chart_is_offered_only_for_a_direct_zip`, `the_modal_takes_every_key_and_esc_closes_it`, `a_press_outside_the_buttons_does_not_reach_the_list`, `a_url_the_check_refuses_is_no_button`.
2. skinned: `a_press_on_a_missing_bar_opens_the_modal_over_the_skin`.
3. open_url.rs `the_opener_is_handed_the_url_alone`(cfg 별: macOS `["open", url]`, 유닉스 `["xdg-open", url]`, Windows `["rundll32", ["url.dll,FileProtocolHandler", url]]`), `an_address_that_is_not_http_is_never_handed_on`.

D3(download/tests.rs, 네트워크 없는 fetcher 주입):

1. `a_finished_download_scans_only_its_folder_and_brings_the_bar_back`(받은 zip 의 차트 md5 가 표 항목과 같음 → 적용 뒤 같은 행이 `Song`, 커서 같음).
2. `an_archive_without_this_chart_says_so_and_keeps_the_bar_missing`.
3. `a_second_download_is_refused_while_one_runs`.
4. `a_refresh_whose_library_changed_meanwhile_is_run_again`(세대 불일치 → 다시 시작).
5. `a_download_that_ends_during_play_is_taken_on_the_browser`(플레이 스테이지에서는 적용 안 됨).
6. crates/rbms-library `a_partial_scan_leaves_the_parser_version_alone`, `a_partial_scan_removes_only_under_its_folder`.
7. library.rs `two_scans_never_write_the_song_database_at_once`(`LIBRARY_WRITE` 를 잡은 동안 두 번째 `scan_now` 가 기다림, 짧은 시간 기준이 아니라 순서 기록으로 단언. 떠는 시험 금지).

### 6.2 뒤집히는 기존 시험

1. `select_view == SelectView::...` 로 단언하는 모든 시험(2.1 의 파일 목록: user_folder_tests.rs:118·125·128, stage/select/tests.rs:480·487·489, table_tests.rs:62·172·210·222·246·351·469·487, search_tests.rs, skinned_tests.rs, stage/tables.rs:586-1295 안 `folders(&mut app, SelectView::...)`·1275-1295, user_folders/tests.rs, capture 파일들)은 경로 단언(`shared.open_titles()` 또는 `select_path` 비교)으로 바꾼다. 의미가 같은 치환이고, 루트로 돌아간다고 단언하던 스캔·표 관리 화면·로딩 취소 시험은 M16 에 따라 "경로 보존" 으로 뒤집힌다.
2. 레벨 폴더 이름: symbol 이 있는 헤더 고정물로 `"LV n (k)"` 를 단언하는 시험은 `"{symbol}n (k)"` 로 뒤집힌다(구현자가 `"LV ` 로 grep 해 전수 확인). 헤더 없는 고정물(대체값 `*`)은 그대로.
3. crates/rbms-config/src/tables.rs: `a_new_player_is_given_the_table_once_in_front_of_the_list`(`added`·`tables` 가 세 건), `a_player_who_has_the_original_is_not_given_the_copy`(`tables.len()` 3), `a_table_shipped_later_is_given_without_touching_the_ones_given_before`(넣는 순서 규칙 M3 로 단언이 바뀔 수 있음).
4. seed.rs: `every_table_given_is_carried_by_the_binary`(6.1 U2-7 로 대체), `a_first_run_has_the_table_and_its_folders_without_the_network`(`config.library.tables` 세 건, 폴더는 발광 하나), `a_player_with_the_original_address_is_not_given_the_copy`(`tables` 에 st·sl 이 붙음). `a_first_run_without_a_song_folder_lists_the_table_it_was_given` 의 행 단언은 그대로(Stella 는 받기 전이라 폴더가 아님).
5. stage/tables.rs:890 `"an unowned level opens onto an empty list"`: U4 뒤에는 없는 곡 막대가 보여 뒤집힌다(`show_missing_charts = false` 로 두는 원래 단언과, 없는 곡 막대 수 단언 두 개로 나눈다).
6. tablesrc.rs 의 `read_and_match_all` 반환형을 구조분해하는 시험(1312-1520 근처)은 `MatchedTable` 로 기계적 치환.

### 6.3 골든·계약·캡처

1. `contract/client-requests/*.json`: 영향 없음(요청 본문 변화 없음). 계약 시험이 그대로 통과해야 한다.
2. rbms-render 골든: 기존 그림 변화 없음, 새 그림 1장(U4).
3. 스킨 캡처(`RBMS_SKIN_CAPTURE_DIR`): table_tests·user_folder_tests 의 캡처 이름은 그대로 두고, U1 에서 `folders-group`·`folders-nested-level`, U4 에서 `level-with-missing`, D2 에서 `download-modal` 을 더한다. 메인이 기본 스킨·ModernChic 위 그림을 눈으로 본다. 표가 없는 캡처는 화소가 같아야 한다(capture.rs 의 `select-root`·`select-table` 은 표 행을 직접 넣으므로 TableRef location 이 비고, symbol 이 없어 `LV` 그대로).

### 6.4 실행 명령

1. 단위별: `cargo test --locked -p rbms-player --lib stage::select`, `cargo test --locked -p rbms-player --lib tablesrc`, `cargo test --locked -p rbms-config`, `cargo test --locked -p rbms-table`, `cargo test --locked -p rbms-library`, `cargo test --locked -p rbms-download`, `cargo test --locked -p rbms-render --test golden_select`.
2. 통합 게이트(docs/development.md:16-18·173): `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`, `cargo test --workspace --no-fail-fast --locked`, `git diff --check`. 떠는 시험 0.
3. 에이전트가 `rbms-player` 를 실행해야 하면 scratch HOME 과 `--server http://127.0.0.1:9`(HANDOFF 5절). 실제 Stella 받기·zip 받기·브라우저 열기는 사용자 실기(QA)로 한다.

## 7. 문서 갱신 목록

1. 새 ADR(docs/acknowledge/, 날짜 2026-10-23): `adr-select-folder-stack.md`(결정 10·11, M1·M2·M16·M17), `adr-stella-default-tables.md`(결정 12, M3·M4·M15, 2026-10-14 기본 표 ADR 의 부분 변경), `adr-table-copy-refresh.md`(결정 13 클라이언트, M5·M6), `adr-missing-chart-bars.md`(결정 16, M7·M8·M9), `adr-chart-download.md`(결정 15, M10~M13·M19·M20, 새 의존성 승인 기록). docs/acknowledge/README.md 색인.
2. 갱신: docs/acknowledge/2026-10-14-adr-default-insane-table.md(Stella 기본 표와 내장하지 않는 이유 한 줄), docs/acknowledge/2026-10-14-adr-user-folders.md(폴더 안 묶음 없음은 MY FOLDERS 에 그대로, 표 묶음은 별개), docs/architecture.md(선곡 경로 모델, 받기 워커, `LIBRARY_WRITE`), docs/crates.md(rbms-download), docs/skin.md(막대 종류 표 3.1.5, `STRING_DIRECTORY` 모양, 없는 곡 막대의 문자열 속성), docs/development.md(새 크레이트 시험, 실행 주의), THIRD-PARTY-NOTICES.md(6절 절차로 재생성: zip 9.0.5 MIT, typed-path 0.12.4 MIT OR Apache-2.0).
3. 조사 보고 docs/reference/2026-10-23-stella-course-dan-survey.md: 2.7 의 정정 6건을 해당 절에 반영(커밋은 메인).
4. docs/quality-assurance/: 수동 점검표에 실기 항목(Stella 첫 받기, st3 에서 Esc 커서, 묶음 경로, 없는 곡 흐림, 모달 버튼, zip 받기와 막대 살아남, rar 링크 브라우저, Windows·Linux 브라우저 열기, 하루 지난 표 갱신) 추가, 테스트 부채는 9절 후보.
5. PROCESS.md 체크리스트 F3·F4 와 HANDOFF 는 메인이 갱신한다.

## 8. 메인 결정 필요 표

| 번호 | 결정 지점 | 추천안 | 근거 | 대안 |
| --- | --- | --- | --- | --- |
| M1 | 표 묶음 막대 종류 | `Command`(그림 5) | 원본에서 다른 막대를 담는 폴더는 ContainerBar(그림 5, BarRenderer.java:162-163). ModernChic 에 `bar-command` 있음 | `Table`(그림 2): 표처럼 보이나 원본 클래스와 다름 |
| M2 | 레벨 폴더 제목 | 헤더 symbol + 레벨(모든 표), symbol 없으면 `LV n` | 원본 `td.getTag() + lv`(TableDataAccessor.java:185), Stella `st0` 요구 | Stella 만 symbol(발광은 `LV n` 유지) / 모두 `LV n` |
| M3 | 이미 기본 표를 받은 사용자에게 새 기본 표를 넣는 자리 | 앞선 기본 표 바로 뒤(새·기존 사용자 모두 `発狂, STELLA` 순) | 사용자 사이 순서가 같아짐 | 지금 규칙(맨 앞): 올린 사용자는 STELLA 가 발광보다 앞 |
| M4 | Stella 내장 스냅샷 | 두지 않음 | 결정 12 "항상 가져오고", 2주마다 바뀜, 사본 비보관 | 둠(오프라인 첫 실행에도 폴더, 실행 파일 약 1.7MB 증가, 저장소에 Stella 데이터) |
| M5 | 하루 지난 표 다시 받기 범위 | 주소로 등록된 표 전부(파일 표 제외) | 사용자가 등록한 표도 최신, 하루 한 번 | 기본 표만 |
| M6 | 다시 받기 알림 | 성공·실패 모두 알림 없음(실패 이유는 표 관리 화면) | 저장본이 그대로 쓰이므로 매일 알림은 소음 | 실패만 경고 한 줄 |
| M7 | 없는 곡 막대 보이기 설정 | 행 추가, 기본 켬 | 원본 `showNoSongExistingBar = true`, 결정 16 | 행 없이 늘 보임 / 기본 끔("저장값 없으면 화소 불변" 원칙을 엄격히 적용하면 이쪽. 그러면 결정 16 기능이 기본으로 안 보임) |
| M8 | 레벨 폴더 수 `(n)` | 가진 곡 수 유지 | 지금 화소·시험 유지 | `(가진/전체)`(MY FOLDERS 방식) |
| M9 | 다운로드 폴더 기본값 | 없음, 첫 받기 때 폴더 고르기 | 사용자 디스크 위치를 앱이 정하지 않음, 요구 5 "폴더 지정과 함께" | 설정 폴더 옆 `downloads` 자동 생성 |
| M10 | Windows 브라우저 열기 | `rundll32 url.dll,FileProtocolHandler <url>` | 셸 해석 없음. Stella 곡 URL 1,492/2,350 이 `&`·`%` 포함 | 과제 문구의 `cmd /C start "" <url>`(raw_arg·추가 이스케이프 규칙 필요) |
| M11 | 단일 차트 링크(.bms 등 123곡) | 브라우저로 | 결정 15 "직접 zip 링크만" | 앱이 그 파일을 폴더에 저장 |
| M12 | 모달 문구 언어 | 영어 | 앱 UI 전체가 영어 | 한국어(CJK 글꼴 경로는 있음) |
| M13 | 받기·풀기 상한 | 받기 512 MiB·무응답 30초·전체 15분, 풀기 항목 10,000·항목 1 GiB·합계 2 GiB | 차분 zip 은 수십 KB, 직접 zip 링크 일부는 곡 패키지 | 다른 값 |
| M14 | 표 코스 폴더 이름 | `COURSE` | 원본 루트 코스 TableBar 이름(BarManager.java:207) | `COURSES` |
| M15 | 묶음 이름 | `STELLA` | 과제 문구. 안의 표도 대문자화로 `STELLA` 라 경로가 `STELLA > STELLA > st3` | `STELLABMS` |
| M16 | 전체 재스캔·표 관리 화면 Esc·로딩 취소 뒤 위치 | 경로 보존 | 원본 `updateBar()` 가 dir 유지 | 지금처럼 루트·첫 줄 |
| M17 | 막대가 0개인 폴더 진입 | 지금처럼 빈 목록으로 들어감 | 기존 R-BMS(빈 목록 안내 있음) | 원본처럼 들어가지 않음(BarManager.java:468-474) |
| M18 | 표 묶음 편집 | 설정 파일만(기본 표가 넣음) | 범위 최소 | 표 관리 화면에 묶음 편집 추가 |
| M19 | 받은 압축에 그 md5 차트가 없을 때 | 폴더 남기고 알림 | 같은 곡의 다른 판일 수 있음 | 폴더 삭제 |
| M20 | 압축 안에 차트가 하나도 없을 때 | 버림(임시 폴더 삭제) | 쓸모없는 파일 방지 | 폴더로 남김 |

## 9. 남은 위험과 테스트 부채 후보

1. 차분만 받은 곡은 음원이 없어 키음 없이 재생될 수 있다. 막대는 살아나지만 플레이 품질은 사용자가 곡 URL 에서 받은 파일을 같은 폴더에 넣어야 맞다. 모달 문구로만 막는다. 후속 후보: 같은 제목의 라이브러리 곡 폴더에 풀기(원본 `open_with_explorer` 가 비슷한 탐색을 함, EventFactory.java:419-446).
2. 부분 스캔 중 프레임 스레드의 곡 DB 읽기(`ChartFacts`, 즐겨찾기)가 SQLITE_BUSY 를 받을 수 있다(`busy_timeout` 없음, 2.6-4). 읽기 실패는 기존처럼 기본값으로 떨어지지만 사실 정보가 한 번 비어 보일 수 있다. 부채: 재현 조건 "큰 압축 풀기 직후 스캔 중 커서 이동", 필요 시 `busy_timeout`(워커 연결만) 검토.
3. `.rbms-partial` 은 다운로드 폴더 안이라, 풀기 중에 사용자가 곡 폴더 화면을 나가 전체 스캔이 돌면 반쯤 푼 차트가 잠시 DB 에 들어갈 수 있다. 다음 스캔에서 지워진다. 부채로 기록.
4. `zip` 9.0.5 는 게시 6일, `typed-path` 0.12.4 는 3일 된 판이다(2026-10-23 기준). 컨벤션의 "최신" 을 따르되 Cargo.lock 으로 고정한다. 보안 공지가 나오면 갱신.
5. `cargo tree` 를 설계 단계에서 돌리지 못했다(실행 권한 거부). D1 첫 단계에서 확인한다.
6. Windows `rundll32` 와 Linux `xdg-open` 실제 동작은 CI 에서 확인할 수 없다(브라우저 없음). 실기 점검표로 한다.
7. reqwest blocking 의 `timeout` 이 "기다림마다" 라는 기존 가정(fetch.rs:19-22 주석)을 받기에도 쓴다. 15분 전체 상한은 조각마다 직접 확인하므로 가정이 틀려도 무한 대기는 없다.
8. 설정 파일에 같은 주소의 표가 두 행이면(손으로 쓴 파일) 두 막대가 같은 정체를 가진다. 나올 때 첫 막대로 간다. 부채.
9. 그림 5번(Command)을 정의하지 않은 외부 스킨에서는 STELLA 묶음 막대가 보이지 않는다(원본과 같은 동작). 그런 스킨이 보고되면 M1 을 다시 본다.
10. Stella 응답에 ETag·Last-Modified 가 없어 매일 전체를 받는다(사용자당 약 1.7MB/일). 사이트 점검 시간(조사 A-2)과 겹치면 그날은 실패하고 다음 실행에 다시 시도한다.
11. sha256 만 있는 표 항목은 가진 곡도 없는 곡도 아니라 보이지 않는다(기존 한계). Stella·Satellite 는 영향 없음.
12. 표 코스 폴더(U5)는 F5 의 `courses_of_table` 이 있어야 한다. F5 가 다른 이름·모양을 고르면 U5 의 호출 한 줄과 3.2 를 고친다.
13. 프레임 스레드에서 표 도착·부분 스캔 적용 때 `Md5Index` 를 라이브러리 전체로 만든다(기존 동작, 선곡에서만). 아주 큰 라이브러리에서 선곡 프레임이 한 번 길어질 수 있다. 플레이 중에는 일어나지 않는다. frame-cost 계측에 선곡 적용 프레임 항목을 더하는 것을 부채 후보로 둔다.
