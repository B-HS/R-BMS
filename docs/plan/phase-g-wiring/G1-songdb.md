# G1 곡DB — G8 배선 지시

> 대상 스펙: `docs/plan/2026-09-09-phase-g-spec.md` §3(곡DB·증분 스캔), §11 순서 2·3.
> 소유 파일(G1이 이미 작성 완료): `crates/rbms-library/src/songdb.rs` + `songdb/**`,
> `crates/rbms-library/src/scan.rs` + `scan/**`, `apps/rbms-player/src/library.rs` + `library/**`.
> 아래 패치는 전부 **G8 소유 파일**에 대한 지시다. G1은 이 파일들을 한 줄도 건드리지 않았다.

## 0. 새로 쓸 수 있는 것

| 심볼 | 경로 | 용도 |
|---|---|---|
| `SongDb` (`open`/`open_in_memory`/`migrate`/`all_songs`/`songs_in_folders`/`by_md5`/`song`/`song_count`/`detail`/`put_detail`/`set_favorite`/`stamp_index`/`upsert_batch`/`upsert_batch_with_details`/`delete_missing`/`needs_full_rescan`/`parser_version`/`set_parser_version`/`columns_of`) | `rbms_library::songdb` | 곡DB 접근자 |
| `SongRow`, `DetailRow`, `SongDbError`, `SCHEMA_VERSION`, `PARSER_VERSION`, `mode_id`, `mode_from_id`, `FEATURE_*`, `CONTENT_*` | `rbms_library::songdb` | 행·상수 |
| `scan_into`, `ScanRequest`, `ScanProgress`, `ScanCounts`, `ScanOutcomeG`, `normalize` | `rbms_library::scan` | 증분 스캔 |
| `SONGDB_FILE`, `open_song_db`, `stored_library`, `library_of`, `entry_of`, `chart_detail`, `detail_of`, `detail_row_of`, `spawn_scan`, `scan_now`, `LibraryScan`, `ScanReport` | `crate::library` (플레이어) | 플레이어 배선용 얇은 층 |

`crate::library` 는 현재 `#![allow(dead_code)]` 이다. **아래 배선이 끝나 모든 항목이 호출되면 그 줄을 지운다.**

---

## 1. 기동 — 곡DB 열기와 `AppShared.song_db`

**파일**: `apps/rbms-player/src/lib.rs`

### 1-1. `AppShared` 필드 추가

**위치**: `struct AppShared` 의 `library: Library,` 필드 **바로 아래**.

```rust
    /// The song database the library is read back from. `None` when it could not be opened, in
    /// which case the run browses whatever a scan hands it and stores nothing.
    song_db: Option<rbms_library::songdb::SongDb>,
```

### 1-2. 기동 시 열기

**위치**: `impl App` 의 `fn new(input: String, mut config: Config, launch: LaunchOptions, settings_path: PathBuf) -> Self` 안, `let library = Library::default();` 문장을 **교체**한다(`let favorites = Favorites::load(&favorites_path);` 아래, 스테이지 결정 `let (stage, chart_path, launch_chart) = ...` 위). 1-3·1-4 도 같은 함수다.

```rust
    let songdb_path = settings_path.parent().map(|d| d.join(crate::library::SONGDB_FILE)).unwrap_or_else(|| PathBuf::from(crate::library::SONGDB_FILE));
    let song_db = crate::library::open_song_db(&songdb_path);
    let library = song_db.as_ref().map(crate::library::stored_library).unwrap_or_default();
    let full_rescan = song_db.as_ref().is_none_or(|db| db.needs_full_rescan().unwrap_or(true));
```

### 1-3. 스테이지 결정에 DB 경로·전량 여부 전달

**위치**: 같은 함수의 `let stage = Stage::Loading(LoadingState::scan(config.library.folders.clone(), config.library.tables.clone()));`

```rust
            let stage = Stage::Loading(LoadingState::scan(config.library.folders.clone(), config.library.tables.clone(), songdb_path.clone(), full_rescan));
```

### 1-4. `AppShared { ... }` 리터럴

**위치**: `shared: AppShared {` 리터럴의 `library,` 바로 아래.

```rust
                song_db,
```

**근거**: 스펙 §3.5(최초 실행 시 `songdb.sqlite` 가 없으면 전량 스캔 1회), §3.3. 이 배선이 없으면 스캔 결과가 어디에도 영속되지 않아 다음 실행이 다시 전량 재파싱한다.
**주의**: `needs_full_rescan()` 은 `meta.parser_version` 이 이 빌드의 `PARSER_VERSION` 과 다를 때 `true` 다. 파서·모델이 바뀐 릴리스는 `rbms_library::songdb::PARSER_VERSION` 을 올리기만 하면 다음 실행이 전량 재스캔한다.

---

## 2. 스캔 진입점 교체 — LOADING 화면

**파일**: `apps/rbms-player/src/stage/loading.rs`

### 2-1. `ScanProgress` 를 라이브러리 카운터 위에 얹는다

**위치**: `pub(crate) struct ScanProgress` 정의 전체를 교체.

```rust
/// How far a background folder scan has got. Read by the screen while the worker fills it in.
#[derive(Default)]
pub(crate) struct ScanProgress {
    /// The scan's own counters, shared with the worker driving [`rbms_library::scan::scan_into`].
    pub(crate) charts: Arc<rbms_library::scan::ScanProgress>,
    /// Difficulty tables matched against the library so far.
    pub(crate) tables: AtomicUsize,
    /// Set once the folder walk is over and the difficulty tables are being matched, which is the
    /// step the screen names and the point its bar stops sweeping and starts filling.
    pub(crate) matching: AtomicBool,
    /// Set when the screen is left, so the scan stops instead of finishing work nobody wants.
    pub(crate) cancel: Arc<AtomicBool>,
}
```

`cancel` 이 `Arc<AtomicBool>` 로 바뀌지만 `progress.cancel.store(true, Ordering::Relaxed)` 호출부(`LoadingState::cancel` 의 `LoadingTask::Scan` 갈래, 테스트)는 **그대로 컴파일된다**(Deref).

### 2-2. `LoadingState::scan`

**위치**: `impl LoadingState` 의 `pub(crate) fn scan(...)` 전체를 교체.

```rust
    /// Rescan every library folder off-thread into the song database (then re-match tables),
    /// landing back on Select. Used at startup and after the folder list changes.
    pub(crate) fn scan(dirs: Vec<String>, sources: Vec<TableSource>, db_path: PathBuf, full: bool) -> LoadingState {
        let progress = Arc::new(ScanProgress::default());
        let worker = progress.clone();
        let request = rbms_library::scan::ScanRequest {
            roots: dirs,
            full,
            cancel: Arc::clone(&progress.cancel),
            progress: Arc::clone(&progress.charts),
        };
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let report = crate::library::scan_now(&db_path, &request);
            if let Some(failure) = &report.failure {
                notify(Level::Error, format!("library scan failed: {failure}"));
            }
            let library = report.library;
            worker.matching.store(true, Ordering::Relaxed);
            let (names, levels) = fetch_and_match(&sources, &library, &worker.tables);
            let _ = tx.send(ScanOutcome { library, names, levels });
        });
        LoadingState { task: LoadingTask::Scan { rx, progress }, drawn: false }
    }
```

### 2-3. `LoadingState::rescan`

**위치**: 바로 아래의 `pub(crate) fn rescan(shared: &AppShared) -> LoadingState`.

```rust
    /// Rescan the folder list as it stands now.
    pub(crate) fn rescan(shared: &AppShared) -> LoadingState {
        let db_path = shared.settings_path.parent().map(|d| d.join(crate::library::SONGDB_FILE)).unwrap_or_else(|| PathBuf::from(crate::library::SONGDB_FILE));
        let full = shared.song_db.as_ref().is_none_or(|db| db.needs_full_rescan().unwrap_or(false));
        LoadingState::scan(shared.config.library.folders.clone(), shared.config.library.tables.clone(), db_path, full)
    }
```

### 2-4. 진행 표시 두 곳

**위치**: `fn heading(...)` 의 `LoadingTask::Scan { progress, .. } => match progress.charts.load(Ordering::Relaxed) {` 갈래.

```rust
            LoadingTask::Scan { progress, .. } => match progress.charts.found.load(Ordering::Relaxed) {
                0 => ("SCANNING", format!("{} folder(s)", shared.config.library.folders.len())),
                found => ("SCANNING", format!("{found} charts found")),
            },
```

`stage/loading.rs` 의 기존 테스트 `progress.charts.store(42, Ordering::Relaxed)` 는
`progress.charts.found.store(42, Ordering::Relaxed)` 로 바꾼다(문구 `"42 charts found"` 는 그대로 통과한다).

**근거**: 스펙 §3.4. `scan_folders` 를 그대로 두면 증분·병렬·취소가 전부 죽은 코드가 된다.
**주의**: 취소는 `progress.cancel` 하나로 통일된다 — `scan_into` 가 열거·파싱·커밋 세 지점에서 같은 플래그를 본다. 취소된 스캔은 **삭제 패스를 건너뛰고** `parser_version` 도 찍지 않으므로, 다음 실행이 이어서 스캔한다.

---

## 3. 곡선택 상세 — DB 캐시 소비

**파일**: `apps/rbms-player/src/stage/select/mod.rs`

**위치**: `fn refresh_focused_detail(&mut self, shared: &AppShared, now: Instant)` 안의 `self.focused_detail = ...` 한 줄.

```rust
        self.focused_detail = si.and_then(|i| shared.library.songs().get(i)).and_then(|e| match &shared.song_db {
            Some(db) => crate::library::chart_detail(db, &e.path, e.mode),
            None => compute_chart_detail(&e.path, e.mode),
        });
```

**근거**: 스펙 §3.4 마지막 문단(스캔 시 상세까지 계산해 포커스 지연 계산을 없앤다). 스캔이 이미 `song_detail` 에 넣어둔 값을 쓰므로 커서를 굴릴 때 `to_model` 이 돌지 않는다. 캐시가 없는 행(구 DB·스캔 밖 차트)은 예전처럼 즉석 계산하고 **결과를 캐시에 넣는다**.

---

## 4. 구 스캔 래퍼 제거

**파일**: `apps/rbms-player/src/assets.rs`

**위치**: `pub(crate) fn scan_folders(folders: &[String], count: &AtomicUsize, cancel: &AtomicBool) -> Vec<SongEntry>` 전체를 삭제. 같은 파일의 `use` 에서 쓰이지 않게 된 `SongEntry`·`AtomicUsize`·`AtomicBool` 도 함께 정리한다.

**파일**: `apps/rbms-player/src/lib.rs`

**위치**: `pub(crate) use assets::{DecodedImage, bundled_skin, decode_bga_image, keysound_jobs, load_theme, resolve_file, scan_folders, spawn_keysound_decode};` 에서 `scan_folders,` 제거.

**근거**: 2번 배선 뒤 유일한 호출부가 사라진다. 남겨두면 `-D warnings` 에서 dead code 로 걸린다.

---

## 5. `rbms-library` 정리

**파일**: `crates/rbms-library/src/lib.rs`

1. **위치**: `pub fn scan_folders(...)` 와 `pub fn scan_folder(...)` — 6번(`rbms-cli`) 교체가 끝난 뒤 **둘 다 삭제**한다. `is_chart`·`compute_chart_detail`·`SongEntry`·`ChartDetail`·`Library` 는 그대로 둔다(전부 계속 쓰인다).
2. **위치**: `crates/rbms-library/src/tests.rs` 의 `scan_folder*` 테스트 5건(`scan_folder_reads_the_sample_charts_and_counts_them`, `scan_folder_sorts_by_lowercased_title`, `scan_folder_of_a_missing_root_is_empty`, `a_cancelled_scan_reads_nothing`, `scan_folders_merges_every_folder_into_one_list`, `scan_folders_of_an_empty_folder_list_is_empty`)도 함께 삭제한다. 같은 성질의 검증은 `scan::tests` 가 tempdir 기준으로 이미 덮는다.
3. **위치**: `pub fn is_chart(p: &Path) -> bool` — G9(bmson)의 배선 지시대로 확장자를 추가한다. `rbms_library::scan` 은 `crate::is_chart` 를 그대로 호출하므로, 이 한 곳만 고치면 스캔이 bmson 을 함께 걷는다. **`scan.rs` 는 수정할 필요가 없다.**

**근거**: 스펙 §9.2 G9 행(확장자 필터 확장은 G8 위임) + §3.4.

---

## 6. `rbms-cli`

**파일**: `apps/rbms-cli/src/main.rs`

**위치**: `fn scan_command(dir: &Path) -> ExitCode` 의 `let songs = scan_folder(dir, &count, &cancel);` 앞뒤.

```rust
    let db_path = std::env::temp_dir().join(format!("rbms-cli-scan-{}.sqlite", std::process::id()));
    let _ = std::fs::remove_file(&db_path);
    let mut db = match rbms_library::songdb::SongDb::open(&db_path).and_then(|mut db| db.migrate().map(|_| db)) {
        Ok(db) => db,
        Err(e) => {
            eprintln!("scan database not opened: {e}");
            return ExitCode::FAILURE;
        }
    };
    let request = rbms_library::scan::ScanRequest::new(vec![dir.to_string_lossy().into_owned()], true);
    if let Err(e) = rbms_library::scan::scan_into(&mut db, &request) {
        eprintln!("scan failed: {e}");
        return ExitCode::FAILURE;
    }
    let songs: Vec<rbms_library::SongEntry> = db.all_songs().unwrap_or_default().iter().map(rbms_library::songdb::rows_entry).collect();
```

`rows_entry` 에 해당하는 변환은 플레이어의 `crate::library::entry_of` 와 같다. CLI 에서 쓰려면 **둘 중 하나**를 고른다.
- (권장) `scan_line`/`mode_histogram` 을 `SongRow` 기준으로 고쳐 변환 자체를 없앤다(`SongRow.judge`=`#RANK`, `SongRow.level` 은 원문 문자열).
- 또는 `entry_of` 를 `rbms-library` 로 올리고 플레이어가 그것을 쓰게 한다(플레이어 `library.rs` 의 `entry_of` 는 G8 시점에 이동해도 무방하다).

`apps/rbms-cli/Cargo.toml` 에는 이미 `rbms-library` 의존이 있다. 추가 의존은 필요 없다.

**근거**: 5번에서 `scan_folder` 를 지우면 CLI 가 깨진다. CLI 는 임시 DB 로 돌려 사용자 `~/.config/rbms/songdb.sqlite` 를 건드리지 않는다.

---

## 7. `favorites.ron` → `song.favorite` (선택, 소유권 표상 G8 판단)

`song.favorite` 컬럼은 이미 있고 **스캔이 절대 덮어쓰지 않는다**(`ON CONFLICT` 갱신 목록에서 `favorite`·`adddate` 제외 — `songdb/rows.rs` `UPSERT_SONG`). 이관을 택한다면:

**파일**: `apps/rbms-player/src/lib.rs`(`App::new`, 1-2 직후)

```rust
    if let Some(db) = &song_db {
        for md5 in favorites.md5s() {
            for row in db.by_md5(md5).unwrap_or_default() {
                let _ = db.set_favorite(&row.path, 1);
            }
        }
    }
```

`Favorites` 에 md5 목록을 내주는 접근자가 없으면(`favorites.rs` 는 `contains`만 공개) 그 파일에 반복자 하나를 추가한다 — `favorites.rs` 는 G8 소유다.
주의: 즐겨찾기 키는 md5(차트), 곡DB 키는 경로다. 같은 md5 가 여러 폴더에 있으면 전부 별이 붙는다(현행 md5 기준 동작과 동일).

---

## 8. 스캔 중 부분 결과 표시 (선택, 스펙 R6)

배치 커밋(512행)이 끝날 때마다 다른 커넥션에서 바로 보인다. LOADING 화면이 스캔이 끝나기를 기다리는 대신 곡선택을 먼저 띄우려면, `AppShared.song_db` 로 프레임마다(또는 N프레임마다) `crate::library::stored_library(db)` 를 다시 읽어 `shared.library` 를 갈아끼우고 `rebuild_select_items()` 를 호출하면 된다. **G1은 이 정책을 정하지 않았다** — 현행(LOADING 에서 완료를 기다림)도 그대로 동작한다.

---

## 9. 스펙 대비 이탈·보류 (G8 이 문서 정본에 옮길 것)

| # | 항목 | 내용 |
|---|---|---|
| 1 | `rbms_level_text` 컬럼 추가 | 스펙 §3.2 DDL 의 `level` 은 레퍼런스대로 `INTEGER` 인데 §3.3 `SongRow.level` 은 `String`(`#PLAYLEVEL` 원문)이다. 원문을 잃지 않으려고 `rbms_level_text TEXT` 를 추가하고 `level` 에는 파싱된 정수(실패 시 0, 레퍼런스 `SongData.java:167-171` 과 동일)를 쓴다. `idx_song_level(mode, level)` 이 그 정수 위에 선다 |
| 2 | `SongRow` 에 `feature`·`content` 필드 추가 | 스펙 §3.3 필드 목록에는 없으나 §3.2 가 비트값을 확정했다. 행에 없으면 두 컬럼이 영원히 0 이 된다 |
| 3 | `charthash` 는 빈 문자열 | 스펙 §13-4 그대로. rbms 에 `toChartString()` 대응 직렬화가 없다 |
| 4 | `minbpm`/`maxbpm` 은 정수 | 레퍼런스 `(int)` 절삭과 동일. 상세 패널의 BPM 범위가 소수점을 잃는다(초기 BPM 은 `rbms_init_bpm REAL` 로 정밀도 유지) |
| 5 | `FEATURE_RANDOM` 판정 | 레퍼런스는 `model.getRandom()` 을 보지만 rbms 파서는 제어흐름을 읽는 즉시 해소해 모델에 남기지 않는다 → **원본 바이트에서 줄머리 `#RANDOM`/`#SETRANDOM`/`#SWITCH`/`#SETSWITCH` 를 찾는다**(`scan.rs` `uses_control_flow`) |
| 6 | `CONTENT_PREVIEW` 판정 | 레퍼런스는 프리뷰 파일 해석 결과를, rbms 는 `#PREVIEW` 헤더 유무를 본다 |
| 7 | `CONTENT_NO_KEYSOUND` 의 샘플 수 | 레퍼런스 `getWavList().length`(슬롯 수) 대신 **비어있지 않은 슬롯 수**를 쓴다(rbms 의 dense 리소스 맵은 정의가 하나도 없어도 길이가 1이라 그대로는 뜻이 어긋난다) |
| 8 | `song.length` | 마지막 타임라인 시각(=상세 패널의 `duration_us`)을 ms 로 저장한다. 레퍼런스 `getLastTime()` 과 미세하게 다를 수 있다 |
| 9 | `folder` 테이블 | DDL 만 만들고 아무도 쓰지 않는다(레퍼런스 DB 임포트 대비). 폴더 뷰를 붙일 때 채운다 |
| 10 | 워커 수 설정 미노출 | 스펙 R8 의 "워커 수를 설정으로 노출" 은 하지 않았다. 현재는 `min(코어, 8)` 고정(`scan.rs` `worker_count`). 설정 행이 필요하면 G8 이 `ScanRequest` 에 필드를 얹지 말고 `worker_count` 를 파라미터화할 것 |
| 11 | 경로 대소문자 | 스펙 §13-7 미결 그대로 — 정규화는 절대경로 + `/` 통일까지만. 대소문자 무시 파일시스템에서 두 철자는 두 행이 된다 |
| 12 | 스탬프 해상도 | `date` 는 초 단위(레퍼런스와 동일). 같은 초 안에 **크기가 같은** 내용으로 덮어쓴 차트는 다음 스캔이 스킵한다. 그런 경우를 강제로 재파싱하려면 `full = true` 스캔(설정의 강제 재스캔)을 쓴다 |
