# Phase G 구현 스펙 — 데이터 스케일·롱테일

> 대상: 계획 `docs/plan/2026-09-09-enhancement-plan.md` §2 Phase G. 결정 5(rusqlite 곡DB)·결정 7(gilrs 먼저, MIDI 후순위)·결정 9(수천 곡 가정)·결정 12(어시스트/커스텀 판정 기록 정책) 전제.
> 이 문서는 설계만 한다. 구현은 아래 §9 의 갈래별 파일 소유권을 그대로 따른다(두 갈래가 같은 파일을 건드리지 않는다).
> 레퍼런스 인용은 파일명:라인만 쓴다(레퍼런스 구현 = 원본 Java 플레이어). rbms 앵커는 커밋 `72bffa5`(= 이 문서 작성 시점 HEAD) 기준 실측. 레퍼런스 인용은 2026-09-09 재검증 완료.
> **주의**: `crates/rbms-ir`, `apps/rbms-player` 는 Phase I 워크플로가 동시 수정 중이다. 이 두 트리의 라인번호는 신뢰하지 말고 **함수명으로 앵커**하며, §7·§8 은 **Phase I 이후 재확인** 표시가 붙은 항목을 반드시 다시 읽고 착수한다.

---

## 1. 현재 상태 (실측 앵커)

| 영역 | 현 구현 | 앵커 |
|---|---|---|
| 라이브러리 스캔 | 매 실행 전량 재파싱. DFS 스택 + `std::fs::read` + `rbms_parser::parse` 를 **단일 스레드**로 돌고 결과를 `Vec<SongEntry>` 로 메모리에만 보관 | `apps/rbms-player/src/main.rs:506` `scan_folders`, `:514` `scan_folder` |
| 스캔 대상 확장자 | `bms|bme|bml|pms` 4종 (bmson 미지원) | `main.rs:500` `is_chart` |
| 곡 메타 | `SongEntry` 18필드(경로·제목·부제·아티스트·장르·maker·level·difficulty·init_bpm·rank·total·mode·md5·stagefile·banner·preview). **sha256·length·notes·minbpm/maxbpm·favorite·adddate 없음** | `main.rs:466` |
| 상세 | `ChartDetail`(notes/long_notes/duration/bpm min·max/density) — 포커스된 1곡만 지연 계산, 캐시 없음 | `main.rs:488` 정의, `main.rs:558` `compute_chart_detail`, `app_select.rs` `refresh_focused_detail` |
| 스캔 취소 | 없음. 백그라운드 스레드는 `scan_rx` 채널로 결과만 보냄. 취소는 `scan_rx = None` 으로 **수신만 포기**(스레드는 끝까지 돈다) | `main.rs:919` spawn, `main.rs:611` `ScanOutcome`, `main.rs:1226` |
| 진행률 | `AtomicUsize` 차트 수 카운터 1개 | `main.rs:904` `scan_count` |
| 스코어 | `scores.ron` 전체를 `Vec<ScoreRecord>` 로 로드/전량 재직렬화. 인덱스 없음, `for_md5` 는 선형 스캔 | `apps/rbms-player/src/scores.rs:23` `ScoreRecord`, `:55` `ScoreBook`, `:60` `load`, `:72` `save`, `:88` `for_md5`, `:101` `best_ex_for_md5`, `:110` `best_clear_for_md5`, `:7` `SCORE_RULE_VERSION = 1` |
| 리플레이 | 파일당 1 RON, GC·상한 없음 | `apps/rbms-player/src/replay.rs:17` `Replay`, `:32` `load`, `:37` `save` |
| 폴더 목록 | `folders.ron` 문자열 배열 | `apps/rbms-player/src/folders.rs` `FolderList` |
| 입력 | 키보드 전용. 게임패드·MIDI·마우스 스크래치 전부 없음 | `apps/rbms-player/src/keyconfig.rs:167` `KeyConfig`, `:117` `ControlBinds`, `:83` `ControlAction`, `:193` `lane_keys`, `:51` `default_keys_for_mode`; `app_input.rs` `lane_for`/`control_for` |
| 코스·연습·시스템 사운드 | 전부 없음 (`rbms-ir` 에 `CourseSubmission` DTO 만 존재) | `crates/rbms-ir/src/dto.rs` `CourseSubmission`, `crates/rbms-ir/src/null.rs` `submit_course` |
| 저장 원자성 | `write_atomic` + `.bak` 백업은 있음. **스키마 버전 필드는 `scores.ron` 의 `rule_version` 뿐**(파일 자체 버전 없음 — Phase C-2 `schema_version` 과 맞물림) | `main.rs:160` `write_atomic` |

---

## 2. 목표와 비목표

**목표(G 범위)**: ① SQLite 곡DB + 증분 스캔 + 병렬 파싱 + 취소 ② SQLite 스코어DB(ScoreLog·플레이어 프로필/통계) + `scores.ron` 마이그레이션 + 리플레이 보존 정책 ③ 코스/단위(Course) 데이터 모델·UI·IR 제출 연동 ④ 연습 모드 ⑤ gilrs 게임패드(스크래치 2키/아날로그, 디바운스) ⑥ 시스템 사운드 ⑦ 라이벌·IR 랭킹 패널·복수 IR 의 **확장 지점**(Phase I 산출물 위에 얹음).

**비목표(이번 Phase 밖)**: MIDI(결정 7 후순위 — §7.4 인터페이스만), 마우스 스크래치(계획 §2 Phase G 나열 항목이나 이번 스펙에서 **범위 축소** — §7.3 확장 지점만), 랜덤 코스(§13), BGA 비디오, 스크린샷/Discord, 스킨 바인딩(Phase E), 판정 알고리즘 확장(Phase D).

**보류(사용자 재확인 필요)**: **bmson 파서**. `docs/acknowledge/2026-09-09-enhancement-decisions.md` 결정 10 채택안은 "`#SWITCH/#CASE/#SKIP/#DEF`는 Phase A-core에 포함, **bmson은 Phase G 별도 항목**"이다. 즉 결정 10 은 bmson 을 **G 범위 안**으로 두었다. 이 스펙은 §9 갈래를 데이터·입력·코스 축으로만 짰으므로 bmson 을 넣으려면 조건부 갈래 **G9**(§9 표 마지막 행)를 활성화해야 한다. 착수 전 사용자 확인 없이 배제하지 않는다(§13-1).

---

## 3. 곡DB (결정 5 — rusqlite bundled)

### 3.1 레퍼런스 스키마 대조

레퍼런스 구현 `SQLiteSongDatabaseAccessor.java:95-105` = `folder` 테이블, `:107-136` = `song` 테이블.

`song`: `md5 TEXT`(unique 1), `sha256 TEXT`(unique 1), `title`, `subtitle`, `genre`, `artist`, `subartist`, `tag`, `path TEXT`(pk), `folder`, `stagefile`, `banner`, `backbmp`, `preview`, `parent`, `level INTEGER`, `difficulty`, `maxbpm`, `minbpm`, `length`, `mode`, `judge`, `feature`, `content`, `date`, `favorite`, `adddate`, `notes`, `charthash TEXT`.
`folder`: `title`, `subtitle`, `command`, `path`(pk), `banner`, `parent`, `type INTEGER`, `date`, `adddate`, `max`.

### 3.2 rbms 스키마 DDL (`songdb.sqlite`, `config_dir()` 하위)

레퍼런스 컬럼명을 그대로 유지해 **향후 레퍼런스 DB 임포트가 가능**하도록 한다. rbms 고유 컬럼은 `rbms_` 접두사로 구분한다.

```sql
PRAGMA journal_mode = WAL;
PRAGMA synchronous  = NORMAL;
PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS meta (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
-- meta: schema_version, parser_version, created_at

CREATE TABLE IF NOT EXISTS folder (
    path     TEXT PRIMARY KEY,
    title    TEXT NOT NULL DEFAULT '',
    subtitle TEXT NOT NULL DEFAULT '',
    command  TEXT NOT NULL DEFAULT '',
    banner   TEXT NOT NULL DEFAULT '',
    parent   TEXT NOT NULL DEFAULT '',
    type     INTEGER NOT NULL DEFAULT 0,
    date     INTEGER NOT NULL DEFAULT 0,
    adddate  INTEGER NOT NULL DEFAULT 0,
    max      INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS song (
    path       TEXT PRIMARY KEY,
    md5        TEXT NOT NULL DEFAULT '',
    sha256     TEXT NOT NULL DEFAULT '',
    title      TEXT NOT NULL DEFAULT '',
    subtitle   TEXT NOT NULL DEFAULT '',
    genre      TEXT NOT NULL DEFAULT '',
    artist     TEXT NOT NULL DEFAULT '',
    subartist  TEXT NOT NULL DEFAULT '',
    tag        TEXT NOT NULL DEFAULT '',
    folder     TEXT NOT NULL DEFAULT '',
    parent     TEXT NOT NULL DEFAULT '',
    stagefile  TEXT NOT NULL DEFAULT '',
    banner     TEXT NOT NULL DEFAULT '',
    backbmp    TEXT NOT NULL DEFAULT '',
    preview    TEXT NOT NULL DEFAULT '',
    level      INTEGER NOT NULL DEFAULT 0,
    difficulty INTEGER NOT NULL DEFAULT 0,
    maxbpm     INTEGER NOT NULL DEFAULT 0,
    minbpm     INTEGER NOT NULL DEFAULT 0,
    length     INTEGER NOT NULL DEFAULT 0,   -- ms
    mode       INTEGER NOT NULL DEFAULT 0,
    judge      INTEGER NOT NULL DEFAULT 0,   -- #RANK
    feature    INTEGER NOT NULL DEFAULT 0,   -- bitflags: LN/CN/HCN/mine/random/stop/scroll
    content    INTEGER NOT NULL DEFAULT 0,   -- bitflags: bga/keysound/preview
    date       INTEGER NOT NULL DEFAULT 0,   -- 파일 mtime(초)
    favorite   INTEGER NOT NULL DEFAULT 0,
    adddate    INTEGER NOT NULL DEFAULT 0,
    notes      INTEGER NOT NULL DEFAULT 0,
    charthash  TEXT NOT NULL DEFAULT '',
    rbms_size       INTEGER NOT NULL DEFAULT 0,  -- 증분 스캔 키
    rbms_total      REAL    NOT NULL DEFAULT 0,
    rbms_init_bpm   REAL    NOT NULL DEFAULT 0,
    rbms_long_notes INTEGER NOT NULL DEFAULT 0,
    rbms_maker      TEXT    NOT NULL DEFAULT '',
    rbms_scanned_at INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_song_md5    ON song(md5);
CREATE INDEX IF NOT EXISTS idx_song_sha256 ON song(sha256);
CREATE INDEX IF NOT EXISTS idx_song_folder ON song(folder);
CREATE INDEX IF NOT EXISTS idx_song_title  ON song(title COLLATE NOCASE);
CREATE INDEX IF NOT EXISTS idx_song_level  ON song(mode, level);

-- 상세 패널(ChartDetail) 캐시. song 삭제 시 함께 사라진다.
CREATE TABLE IF NOT EXISTS song_detail (
    path         TEXT PRIMARY KEY REFERENCES song(path) ON DELETE CASCADE,
    duration_us  INTEGER NOT NULL,
    peak_density REAL NOT NULL,
    avg_density  REAL NOT NULL,
    end_density  REAL NOT NULL,
    density_bins BLOB NOT NULL   -- u32 LE 배열
);
```

`song.path` 는 **정규화된 절대경로 문자열**(`std::path::absolute` + 구분자 `/` 통일). 대소문자 구분 파일시스템 차이는 정규화하지 않는다(macOS 대소문자 무시 FS 에서 중복 행이 생길 수 있음 → §13-7).

`feature`/`content` 비트값은 **레퍼런스 값을 그대로 채택**한다(`SongData.java:24-36` 실측, 2026-09-09 확인).

| 컬럼 | 비트 | 상수 | 산출 근거(레퍼런스) |
|---|---|---|---|
| feature | 1 | UNDEFINEDLN | `SongData.java:192` |
| feature | 2 | MINENOTE | `:206` |
| feature | 4 | RANDOM | `:215` (`model.getRandom()` 비어있지 않음) |
| feature | 8 | LONGNOTE | `:195` |
| feature | 16 | CHARGENOTE | `:198` |
| feature | 32 | HELLCHARGENOTE | `:201` |
| feature | 64 | STOPSEQUENCE | `:182` |
| feature | 128 | SCROLL | `:185` |
| content | 1 | TEXT (동봉 txt 존재) | `:137` |
| content | 2 | BGA | `:216` (`getBgaList().length > 0`) |
| content | 4 | PREVIEW | `:35` 상수(세팅은 프리뷰 파일 존재 시) |
| content | 128 | NOKEYSOUND | `:217` (`length >= 30000 && wavList.len <= length/50000 + 3`) |

`charthash` 는 레퍼런스 `SongData.java:220-222` 기준 **`SHA-256(model.toChartString() 바이트)` 의 hex 소문자**다. rbms 의 `toChartString()` 대응물이 아직 없으므로 G 에서는 빈 문자열로 두고, Phase A 파서 정비 후 동일 직렬화를 정의해 채운다(§13).

### 3.3 크레이트 API (`crates/rbms-songdb`)

```rust
pub struct SongDb { conn: rusqlite::Connection }

pub struct SongRow {
    pub path: String, pub md5: String, pub sha256: String,
    pub title: String, pub subtitle: String, pub artist: String, pub subartist: String,
    pub genre: String, pub maker: String, pub level: String, pub difficulty: i32,
    pub mode: i32, pub judge: i32, pub total: f64, pub init_bpm: f64,
    pub min_bpm: i32, pub max_bpm: i32, pub length_ms: i64, pub notes: i32, pub long_notes: i32,
    pub stagefile: String, pub banner: String, pub backbmp: String, pub preview: String,
    pub folder: String, pub favorite: i32, pub date: i64, pub adddate: i64, pub size: i64,
}

pub struct DetailRow { pub duration_us: i64, pub peak_density: f64, pub avg_density: f64, pub end_density: f64, pub density: Vec<u32> }

impl SongDb {
    pub fn open(path: &std::path::Path) -> Result<SongDb, SongDbError>;
    pub fn open_in_memory() -> Result<SongDb, SongDbError>;      // 테스트 전용
    pub fn migrate(&mut self) -> Result<u32, SongDbError>;       // schema_version 반환
    pub fn all_songs(&self) -> Result<Vec<SongRow>, SongDbError>;
    pub fn songs_in_folders(&self, roots: &[String]) -> Result<Vec<SongRow>, SongDbError>;
    pub fn by_md5(&self, md5: &str) -> Result<Vec<SongRow>, SongDbError>;
    pub fn detail(&self, path: &str) -> Result<Option<DetailRow>, SongDbError>;
    pub fn put_detail(&self, path: &str, d: &DetailRow) -> Result<(), SongDbError>;
    pub fn set_favorite(&self, path: &str, favorite: i32) -> Result<(), SongDbError>;
    /// 증분 스캔 키: path -> (date=mtime_secs, size)
    pub fn stamp_index(&self) -> Result<std::collections::HashMap<String, (i64, i64)>, SongDbError>;
    pub fn upsert_batch(&mut self, rows: &[SongRow]) -> Result<(), SongDbError>;   // 단일 트랜잭션
    pub fn delete_missing(&mut self, alive: &std::collections::HashSet<String>, roots: &[String]) -> Result<usize, SongDbError>;
}

#[derive(Debug)] pub enum SongDbError { Sqlite(rusqlite::Error), Schema(String) }
```

`rusqlite` 는 `features = ["bundled"]`(시스템 SQLite 비의존, Windows 배포 포함 결정 5의 근거). 버전은 착수 시점 최신을 확인해 정확히 고정한다(§13-2).

### 3.4 증분 스캔 + 병렬 파싱 + 취소

```rust
pub struct ScanProgress {
    pub found: std::sync::atomic::AtomicUsize,    // 워크에 들어간 차트 파일 수
    pub parsed: std::sync::atomic::AtomicUsize,   // 실제 파싱한 수(스킵 제외)
    pub skipped: std::sync::atomic::AtomicUsize,  // mtime+size 일치로 재사용한 수
    pub removed: std::sync::atomic::AtomicUsize,
}

pub struct ScanRequest {
    pub roots: Vec<String>,
    pub full: bool,                                // true = mtime/size 무시하고 전량 재파싱
    pub cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
    pub progress: std::sync::Arc<ScanProgress>,
}

pub enum ScanOutcomeG { Completed { upserted: usize, removed: usize }, Cancelled }

pub fn scan_into(db: &mut SongDb, req: &ScanRequest) -> Result<ScanOutcomeG, SongDbError>;
```

알고리즘:

1. **열거(단일 스레드)** — 현 `scan_folder`(`main.rs:514`)의 DFS 를 그대로 옮기되 파싱은 하지 않고 `(PathBuf, mtime_secs, size)` 만 모은다. 심볼릭 링크는 따라가지 않는다(무한 루프 방지). 열거 루프 매 디렉터리마다 `cancel` 확인.
2. **차분** — `db.stamp_index()` 와 대조. `full == false` 이고 `(mtime, size)` 가 동일하면 **파싱 스킵**. DB 에 있으나 열거에 없고 경로가 `roots` 하위인 행은 삭제 후보.
3. **병렬 파싱** — 워커 수 = `std::thread::available_parallelism().min(8).max(1)`(현 `spawn_keysound_decode`, `main.rs:436` 와 동일 규칙). 각 워커가 `read → rbms_parser::parse → detect_mode → md5/sha256 → to_model` 을 수행해 `SongRow` + `DetailRow` 를 채널로 보낸다. **1차 구현은 `std::thread` + `mpsc`** (rayon 신규 의존을 늘리지 않는다 — 컨벤션 "의존성 다이어트", 이미 같은 패턴이 레포에 존재). 워커는 청크마다 `cancel` 을 확인하고 즉시 반환한다.
4. **커밋** — 수집 스레드가 512행 단위로 `upsert_batch`(단일 트랜잭션)를 호출. 배치 사이에서 `cancel` 이면 지금까지의 배치는 유지한 채 `Cancelled` 반환(증분 스캔이라 다음 실행에서 이어짐).
5. **삭제** — 취소되지 않은 경우에만 `delete_missing`.

`to_model` 은 비용이 크므로 **스캔 시에는 상세(`song_detail`)까지 계산**한다(포커스 지연 계산을 없애 곡선택 반응성을 확보). 파싱 실패 파일은 로그만 남기고 건너뛴다(현 동작과 동일).

### 3.5 RON → SQLite 마이그레이션

- `folders.ron` 은 **그대로 유지**한다(라이브러리 루트 목록은 사용자 설정이지 캐시가 아님).
- 곡 목록 자체는 RON 파일이 없다(현재 메모리 전용) → 마이그레이션 대상 없음. 최초 실행 시 `songdb.sqlite` 가 없으면 `full = true` 스캔 1회.
- `meta.schema_version` 불일치 시: 상위 버전 DB면 열지 않고 사용자에게 알림, 하위 버전이면 `migrate()` 가 `ALTER TABLE` 로 올린다. **파싱 규칙이 바뀐 릴리스**(파서/모델 변경)는 `meta.parser_version` 을 올려 전량 재스캔을 강제한다.

---

## 4. 스코어DB

### 4.1 레퍼런스 대조

- `ScoreDatabaseAccessor.java:31-34` = `info(id, name, rank)`
- `:36-53` = `player(date PK, playcount, clear, epg, lpg, egr, lgr, egd, lgd, ebd, lbd, epr, lpr, ems, lms, playtime, maxcombo)` — **일자별 누적 통계 행**
- `:55-84` = `score(sha256, mode) PK` + 판정 10필드 + `notes, combo, minbp, avgjudge, playcount, clearcount, trophy, ghost, option, seed, random, date, state, scorehash`
- `ScoreDataLogDatabaseAccessor.java:22` `TABLE_NAME = "scoredatalog"`, `:23-52` 컬럼 정의 — `score` 와 같은 컬럼셋에 **플레이 1회당 1행**. rbms 는 테이블명을 `scorelog` 로 쓴다(자체 명명, 레퍼런스 DB 임포트 시 `scoredatalog` → `scorelog` 로 매핑).

rbms 는 **md5 를 1차 키로 쓰는 기존 기록**(`scores.rs` `ScoreRecord.md5`)이 있으므로 `sha256` 단독 PK 를 쓰지 못한다. `chart_key = sha256 이 있으면 sha256, 없으면 md5` 로 두고 두 해시를 모두 컬럼으로 보관한다.

### 4.2 DDL (`scoredb.sqlite`)

```sql
PRAGMA journal_mode = WAL;

CREATE TABLE IF NOT EXISTS meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);

-- 차트별 베스트 1행 (레퍼런스 score 대응)
CREATE TABLE IF NOT EXISTS score (
    chart_key TEXT NOT NULL,
    mode      INTEGER NOT NULL,
    md5       TEXT NOT NULL DEFAULT '',
    sha256    TEXT NOT NULL DEFAULT '',
    clear     INTEGER NOT NULL DEFAULT 0,
    epg INTEGER NOT NULL DEFAULT 0, lpg INTEGER NOT NULL DEFAULT 0,
    egr INTEGER NOT NULL DEFAULT 0, lgr INTEGER NOT NULL DEFAULT 0,
    egd INTEGER NOT NULL DEFAULT 0, lgd INTEGER NOT NULL DEFAULT 0,
    ebd INTEGER NOT NULL DEFAULT 0, lbd INTEGER NOT NULL DEFAULT 0,
    epr INTEGER NOT NULL DEFAULT 0, lpr INTEGER NOT NULL DEFAULT 0,
    ems INTEGER NOT NULL DEFAULT 0, lms INTEGER NOT NULL DEFAULT 0,
    notes INTEGER NOT NULL DEFAULT 0,
    combo INTEGER NOT NULL DEFAULT 0,
    minbp INTEGER NOT NULL DEFAULT 2147483647,
    avgjudge INTEGER NOT NULL DEFAULT 2147483647,
    playcount  INTEGER NOT NULL DEFAULT 0,
    clearcount INTEGER NOT NULL DEFAULT 0,
    option INTEGER NOT NULL DEFAULT 0,
    seed   INTEGER NOT NULL DEFAULT -1,
    random INTEGER NOT NULL DEFAULT 0,
    date   INTEGER NOT NULL DEFAULT 0,
    state  INTEGER NOT NULL DEFAULT 0,
    gauge        INTEGER NOT NULL DEFAULT 0,
    assist       INTEGER NOT NULL DEFAULT 0,
    rule_version INTEGER NOT NULL DEFAULT 0,
    replay_file  TEXT,
    PRIMARY KEY (chart_key, mode)
);

-- 플레이 1회당 1행 (레퍼런스 scorelog 대응). 그래프·이력·리플레이 GC 의 근거.
CREATE TABLE IF NOT EXISTS scorelog (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    chart_key TEXT NOT NULL, mode INTEGER NOT NULL,
    md5 TEXT NOT NULL DEFAULT '', sha256 TEXT NOT NULL DEFAULT '',
    clear INTEGER NOT NULL DEFAULT 0,
    epg INTEGER, lpg INTEGER, egr INTEGER, lgr INTEGER, egd INTEGER, lgd INTEGER,
    ebd INTEGER, lbd INTEGER, epr INTEGER, lpr INTEGER, ems INTEGER, lms INTEGER,
    notes INTEGER, combo INTEGER, minbp INTEGER, avgjudge INTEGER,
    gauge INTEGER, gauge_value REAL, assist INTEGER,
    option INTEGER, seed INTEGER, random INTEGER,
    date INTEGER NOT NULL, state INTEGER NOT NULL DEFAULT 0,
    rule_version INTEGER NOT NULL DEFAULT 0,
    ir_submitted INTEGER NOT NULL DEFAULT 0,
    replay_file TEXT,
    title TEXT NOT NULL DEFAULT ''
);
CREATE INDEX IF NOT EXISTS idx_log_chart ON scorelog(chart_key, date DESC);
CREATE INDEX IF NOT EXISTS idx_log_date  ON scorelog(date DESC);
CREATE INDEX IF NOT EXISTS idx_log_replay ON scorelog(replay_file);

-- 일자별 누적 통계 (레퍼런스 player 대응)
CREATE TABLE IF NOT EXISTS player_stat (
    date INTEGER PRIMARY KEY,      -- UTC 일 시작 epoch(초)
    playcount INTEGER NOT NULL DEFAULT 0,
    clear     INTEGER NOT NULL DEFAULT 0,
    epg INTEGER, lpg INTEGER, egr INTEGER, lgr INTEGER, egd INTEGER, lgd INTEGER,
    ebd INTEGER, lbd INTEGER, epr INTEGER, lpr INTEGER, ems INTEGER, lms INTEGER,
    playtime INTEGER NOT NULL DEFAULT 0,   -- ms
    maxcombo INTEGER NOT NULL DEFAULT 0
);

-- 프로필 (레퍼런스 info 대응)
CREATE TABLE IF NOT EXISTS profile (
    id   TEXT PRIMARY KEY,
    name TEXT NOT NULL DEFAULT '',
    rank TEXT NOT NULL DEFAULT ''
);
```

### 4.3 크레이트 API (`crates/rbms-scoredb`)

```rust
pub struct ScoreDb { conn: rusqlite::Connection }

pub struct PlayLog { /* scorelog 1행 = 결과 화면이 만든 값 */ }
pub struct BestScore { /* score 1행 */ }
pub struct DayStat { /* player_stat 1행 */ }

impl ScoreDb {
    pub fn open(path: &std::path::Path) -> Result<ScoreDb, ScoreDbError>;
    pub fn open_in_memory() -> Result<ScoreDb, ScoreDbError>;
    pub fn migrate(&mut self) -> Result<u32, ScoreDbError>;

    /// 플레이 1회 기록: scorelog insert + score 갱신 + player_stat 누적을 한 트랜잭션으로.
    /// `updates_best == false`(결정 12: 어시스트/커스텀 판정)면 scorelog 만 쓰고
    /// score 의 EX/BP/콤보는 건드리지 않되 playcount 와 램프는 갱신한다.
    pub fn record_play(&mut self, log: &PlayLog, updates_best: bool) -> Result<i64, ScoreDbError>;

    pub fn best(&self, chart_key: &str, mode: i32) -> Result<Option<BestScore>, ScoreDbError>;
    pub fn best_many(&self, keys: &[String]) -> Result<std::collections::HashMap<String, BestScore>, ScoreDbError>;
    pub fn history(&self, chart_key: &str, limit: usize) -> Result<Vec<PlayLog>, ScoreDbError>;
    pub fn day_stats(&self, from: i64, to: i64) -> Result<Vec<DayStat>, ScoreDbError>;
    pub fn profile(&self) -> Result<Option<(String, String, String)>, ScoreDbError>;

    /// 리플레이 보존 정책 적용. 남길 파일 목록을 반환(호출부가 파일 삭제 담당).
    pub fn replay_gc_plan(&self, policy: &ReplayPolicy) -> Result<ReplayGcPlan, ScoreDbError>;
}

pub struct ReplayPolicy {
    pub keep_best_per_chart: bool,   // 기본 true
    pub keep_recent_per_chart: usize,// 기본 3
    pub max_total_bytes: u64,        // 기본 2 GiB, 0 = 무제한
}
pub struct ReplayGcPlan { pub delete: Vec<String>, pub kept: usize, pub freed_bytes: u64 }
```

### 4.4 `scores.ron` → SQLite 마이그레이션

1. 앱 시작 시 `scoredb.sqlite` 가 없고 `scores.ron` 이 있으면 1회 실행.
2. `ScoreBook::load`(`scores.rs:60`)로 읽어 **`played_at` 오름차순**으로 `record_play` 를 재생한다. 각 레코드의 `assisted`(`scores.rs`)를 `updates_best = !assisted` 로 매핑해 `best_clear_for_md5`(`scores.rs:110`)의 기존 정책(어시스트는 베스트가 되지 않음)을 SQLite 에서도 그대로 재현한다.
3. `mode` 문자열(`ScoreRecord.mode`)은 `rbms_model::Mode` 로 파싱해 정수화. 실패 시 0.
4. `counts[6]`(`scores.rs:30`) → `epg..lms` 매핑은 **fast/slow 분리가 없던 기록**이므로 late 쪽(`lpg/lgr/lgd/lbd/lpr/lms`)에 전량 넣고 early 는 0. `scorelog.state = 1`(=마이그레이션 유래) 로 표시해 통계에서 구분한다.
5. `rule_version` 은 그대로 옮긴다(0 = 버저닝 이전, `scores.rs:10` `rule_version_mark` 표기 규칙 유지).
6. 성공하면 `scores.ron` 을 `scores.ron.migrated` 로 **이름만 바꾸고 지우지 않는다**. 실패하면 SQLite 파일을 지우고 RON 경로로 폴백(앱은 계속 동작).
7. 마이그레이션 후에도 `ScoreBook` 타입은 남기되 저장소만 SQLite 로 바꾼다(§9 G8 통합에서 호출부 교체).

---

## 5. 코스 / 단위 (Course)

### 5.1 레퍼런스 모델

`CourseData.java:18-34`: `name`, `hash: SongData[]`(순서 있는 차트 목록), `constraint: CourseDataConstraint[]`, `trophy: TrophyData[]`, `release: boolean`.
`CourseData.java:146-204` 제약 enum(괄호 안은 배타 그룹 id):

| 값 | 문자열 | 그룹 | 의미 |
|---|---|---|---|
| CLASS | `grade` | 0 | 단위(옵션 금지) |
| MIRROR | `grade_mirror` | 0 | 단위(미러 허용) |
| RANDOM | `grade_random` | 0 | 단위(랜덤 허용) |
| NO_SPEED | `no_speed` | 1 | 하이스피드 변경 금지 |
| NO_GOOD | `no_good` | 2 | GOOD 판정 없음 |
| NO_GREAT | `no_great` | 2 | GREAT 판정 없음 |
| GAUGE_LR2 / GAUGE_5KEYS / GAUGE_7KEYS / GAUGE_9KEYS / GAUGE_24KEYS | `gauge_lr2` / `gauge_5k` / `gauge_7k` / `gauge_9k` / `gauge_24k` | 3 | 게이지 세트 강제 |
| LN / CN / HCN | `ln` / `cn` / `hcn` | 4 | lnmode 강제 |

`CourseData.java:229-273` `TrophyData { name, missrate, scorerate }`.
`CourseDataAccessor.java:32-64`: 코스 디렉터리의 `*.json` 을 읽어 `CourseData[]` 또는 단일 `CourseData` 로 역직렬화(알 수 없는 필드 무시, `validate()` 통과분만 채택).

**`validate()` 실제 동작** (`CourseData.java:99-131` 전문 통독, 2026-09-09):

| 상황 | 레퍼런스 동작 |
|---|---|
| `hash` 가 null 이거나 길이 0 | **`false` 반환**(유일한 코스 레벨 실패 조건) |
| `name` 이 null 이거나 빈 문자열 | 실패가 **아니라** `"No Course Title"` 로 자동 채움 |
| 차트 원소가 null | **`false` 반환** |
| 차트 제목이 비었음 | `"course {i+1}"` 로 자동 채움 |
| `sd.validate()` 실패 | **`false` 반환** |
| `constraint` 가 null | `CourseDataConstraint.EMPTY` 로 대체 |
| 같은 그룹(`type`)의 제약이 2개 이상 | 실패가 **아니라** 그룹당 **첫 원소만 남기고 제거**(`cdc[5]` 버킷 + `removeInvalidElements`) 후 `true` |
| `trophy` 가 null | `TrophyData.EMPTY` 로 대체 |

즉 **정규화(normalize)가 기본이고 거부는 예외**다. rbms 가 "그룹 중복 = 거부"로 만들면 레퍼런스 코스 JSON 이 통째로 탈락하므로 이 동작을 그대로 따른다.

### 5.2 rbms 데이터 모델 (`crates/rbms-course`)

```rust
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub enum CourseConstraint {
    Class, Mirror, Random,
    NoSpeed, NoGood, NoGreat,
    GaugeLr2, Gauge5Keys, Gauge7Keys, Gauge9Keys, Gauge24Keys,
    Ln, Cn, Hcn,
}
impl CourseConstraint {
    pub fn token(self) -> &'static str;             // "grade" ...
    pub fn from_token(s: &str) -> Option<Self>;
    pub fn group(self) -> u8;                       // 0..=4 (배타 그룹)
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct TrophyRule { pub name: String, pub missrate: f32, pub scorerate: f32 }

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct CourseChart { pub md5: String, pub sha256: String, pub title: String }

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Course {
    pub name: String,
    pub charts: Vec<CourseChart>,
    #[serde(default)] pub constraints: Vec<CourseConstraint>,
    #[serde(default)] pub trophies: Vec<TrophyRule>,
    #[serde(default = "default_true")] pub release: bool,
}

impl Course {
    /// 레퍼런스 `CourseData.validate()` 이식. **정규화 후 채택 여부만 반환**한다.
    /// - `charts` 가 비면 `false`(유일한 코스 레벨 실패).
    /// - 차트 원소의 md5·sha256 이 **둘 다** 비면 `false`(레퍼런스 `sd.validate()` 실패 대응).
    /// - `name` 이 비면 `"No Course Title"` 로 채운다(실패 아님).
    /// - 차트 `title` 이 비면 `"course {i+1}"` 로 채운다(실패 아님).
    /// - 같은 `group()` 의 제약이 2개 이상이면 **선언 순서상 첫 원소만 남기고 제거**한다(실패 아님).
    pub fn validate(&mut self) -> bool;
    pub fn is_class(&self) -> bool;                 // group 0 제약 보유
    pub fn hash(&self) -> String;                   // IR course_hash: sha256(name + '\n' + 차트 해시 순서 결합)
}

/// `*.json`, 배열/단일 모두 허용, unknown field 무시. 각 원소에 `validate()` 를 적용해
/// **정규화한 뒤 true 인 것만** 반환한다(레퍼런스 `CourseDataAccessor.java:32-64` 와 동일).
pub fn load_dir(dir: &std::path::Path) -> Vec<Course>;
pub fn save(dir: &std::path::Path, c: &Course) -> std::io::Result<()>;
```

레퍼런스 JSON 과 호환되도록 serde 별칭을 단다: `#[serde(alias = "hash")]` for `charts`, `#[serde(alias = "constraint")]`, `#[serde(alias = "trophy")]`, `#[serde(alias = "song")]`.

### 5.3 코스 진행 상태

```rust
pub struct CourseRun {
    pub course: Course,
    pub index: usize,                 // 현재 스테이지
    pub carry_gauge: f32,             // 스테이지 간 이월 게이지
    pub totals: CourseTotals,         // ex/notes/판정/미스 누적
    pub failed_at: Option<usize>,
}
pub struct CourseTotals { pub ex: u32, pub max_ex: u32, pub notes: u32, pub counts: [u32; 6], pub empty_poor: u32, pub max_combo: u32, pub combo_carry: u32 }

impl CourseRun {
    pub fn advance(&mut self, stage: &StageResult) -> CourseStep;   // Next | Cleared | Failed
    pub fn trophy(&self) -> Option<&TrophyRule>;                    // missrate/scorerate 로 최고 등급 선택
}
```

게이지·콤보는 스테이지 경계에서 **이월**한다(레퍼런스 단위 인정 동작). 게이지가 0 이 되면 즉시 `Failed`, 남은 스테이지는 플레이하지 않는다.

### 5.4 UI 흐름

- 곡선택에 **COURSE 탭**을 추가한다(정렬 축이 아니라 별도 리스트). `Stage::Select` 하위 뷰로 `SelectTab { Songs, Courses }`.
- 코스 행 표시: 이름 · 스테이지 수 · 제약 배지 · 각 차트의 라이브러리 존재 여부(전부 있어야 시작 가능 — 레퍼런스 `BarRenderer.java:154-155` 의 `existsAllSongs` 대응).
- 시작 → `Stage::Loading`(스테이지 1 로드) → `Stage::Play` → 스테이지별 결과는 요약만 표시하고 **다음 스테이지로 자동 진행** → 마지막 스테이지 후 `Stage::CourseResult`.
- 제약 적용: `NoSpeed` 는 하이스피드 조작 키(`ControlAction::HispeedUp/Down`)를 무시, `Class` 는 random/mirror 옵션을 `Off` 로 강제(그룹 0 의 Mirror/Random 은 각각 허용), 게이지·lnmode 제약은 시작 시 설정값을 덮어쓴다.
- 코스 중 Esc: 기존 2회 누르기 규칙(`main.rs:237` `esc_confirms_quit`)을 그대로 쓰되 코스 전체를 중단하고 Select 로 돌아간다(기록 없음).

### 5.5 IR 코스 제출 연동

`crates/rbms-ir/src/dto.rs` 의 `CourseSubmission`(이미 존재, `course_hash` + charts)을 그대로 쓴다. 제출 게이트는 단일 곡과 동일한 규칙을 재사용한다: `main.rs` `ir_submission_block_reason`(autoplay / replay / judge_rate>100 / scratch_auto)이 **어느 스테이지에서든 한 번이라도 참이면 코스 전체가 제출 불가**. `updates_score`(`main.rs`)도 동일하게 코스 단위 AND 로 판정한다.
**IR 트레이트 현황**(2026-09-09 실측, Phase I 가 동시 수정 중이므로 착수 전 재확인):

| 항목 | 위치 | 상태 |
|---|---|---|
| `fn submit_course(&self, sub: &CourseSubmission) -> Result<SubmitResponse, IrError>` | `crates/rbms-ir/src/lib.rs:38` (`trait ScoreServer` 본문) | **필수 메서드**(기본 구현 없음) |
| 같은 메서드의 offline 스텁 | `crates/rbms-ir/src/null.rs:27` | `NullScoreServer` 구현 |
| `fn course_ranking(&self, _course_hash: &str, _limit: u32) -> Result<Vec<ScoreRecord>, IrError>` | `crates/rbms-ir/src/lib.rs:44` | **이미 기본 구현 존재**(`Err(IrError::Unsupported)`) |

따라서 코스 랭킹은 **새로 정의하지 말고 기존 `course_ranking` 을 호출**한다(§10). `submit_course` 의 `HttpScoreServer` 구현은 Phase I 소유 파일을 건드리므로 G 범위 밖이다.

**`crates/rbms-ir` 는 G 가 한 줄도 수정하지 않는다.** Rust 는 신규 모듈 파일에 `lib.rs` 의 `pub mod ...;` 선언이 필요한데 `crates/rbms-ir/src/lib.rs:1-5` 는 Phase I 소유이므로, 초판 스펙의 `crates/rbms-ir/src/course_ext.rs` 안은 **폐기**한다. 코스 전용 헬퍼는 플레이어 쪽 신규 파일 **`apps/rbms-player/src/course_ir.rs`**(G3 소유, `mod` 선언은 G0 이 미리 넣음)에 두고, 내용은 다음으로 한정한다.

```rust
// apps/rbms-player/src/course_ir.rs  — rbms-ir 의 공개 타입만 소비한다(트레이트 구현 없음)
/// CourseRun 결과를 rbms_ir::dto::CourseSubmission 으로 변환.
pub fn build_course_submission(run: &rbms_course::CourseRun, player: &str) -> rbms_ir::dto::CourseSubmission;
/// 스테이지별 차단 사유를 코스 단위로 AND 접기. 하나라도 차단이면 Some(사유).
pub fn course_block_reason(stage_reasons: &[Option<String>]) -> Option<String>;
/// primary 프로필 서버에만 제출. Unsupported 는 조용히 무시(구버전 IR 호환).
pub fn submit(server: &dyn rbms_ir::ScoreServer, sub: &rbms_ir::dto::CourseSubmission)
    -> Result<rbms_ir::SubmitResponse, rbms_ir::IrError>;
```

---

## 6. 연습 모드 (Practice)

### 6.1 레퍼런스 파라미터

`PracticeConfiguration.java:653-702` `PracticeProperty`:

| 필드 | 기본 | 의미 |
|---|---|---|
| `starttime` | 0 | 연습 시작 시각(ms) |
| `endtime` | 10000 | 종료 시각(ms) |
| `gaugecategory` | - | 게이지 카테고리(모드별 세트) |
| `gaugetype` | 2 | 게이지 종류 인덱스(0..8) |
| `startgauge` | 20 | 시작 게이지량 |
| `random` / `random2` / `doubleop` | 0 | 옵션 |
| `judgerank` | 100 | 판정폭 배율 |
| `freq` | 100 | 재생 속도 배율(%) |
| `total` | 0 | TOTAL 오버라이드 |
| `graphtype` | 0 | 그래프 종류 |

조작 단위(`PracticeConfiguration.java:531-640`, `roundDownTo` 는 `:520`). 스텝은 START/END TIME 모두 기본 100ms, turbo 시 2500ms(아날로그 입력이면 1000ms). 경계는 전량 실측 확인(2026-09-09):

| 요소 | 하한 | 상한 | 부수효과 |
|---|---|---|---|
| START TIME (`:531-543`) | `0` | `roundDownTo(마지막 타임라인 - 2000, 100)` | **증가 방향일 때만** `endtime = max(endtime, starttime + 1000)` (여기엔 내림 없음) |
| END TIME (`:545-557`) | `roundDownTo(starttime + 1000, 100)` | `roundDownTo(마지막 타임라인 + 1000, 100)` | 없음 |

`endtime` 기본값은 `:662` 의 `10000` 이지만, 차트 로드시 `:57` 이 `모델 마지막 시각 + 1000` 으로 덮어쓴다. `gaugetype` 은 `(v + (inc ? 1 : 8)) % 9`. PMS(POPN_5K/9K)에서 `gaugetype >= 3` 이고 `startgauge > 100` 이면 100 으로 클램프. GAUGE VALUE 는 turbo 10 / 기본 1 단위, `1..=max` 클램프. JUDGERANK 는 turbo 25 / 기본 1.

### 6.2 rbms 스펙

```rust
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct PracticeProperty {
    pub start_ms: i32, pub end_ms: i32,
    pub gauge_type: u8,      // 0..=8 (Phase D 의 9종 게이지와 정합. 현 6종이면 0..=5 로 클램프)
    pub start_gauge: i32,
    pub judge_rate: i32,     // = PlaySettings::judge_rate 와 같은 축
    pub freq: i32,           // 재생 속도 %, 50..=200
    pub total: f64,          // 0 = 차트 값 사용
    pub random: String,
}
pub fn clamp_start(p: &mut PracticeProperty, last_tl_ms: i32, turbo: bool, analog: bool, inc: bool);
pub fn clamp_end(p: &mut PracticeProperty, last_tl_ms: i32, turbo: bool, analog: bool, inc: bool);
```

- **범위 재생**: `start_ms` 이전 노트/키음을 건너뛰고, `end_ms` 도달 시 즉시 결과 없이 연습 패널로 복귀. 게이지는 `start_gauge` 로 고정 시작하며 **연습 중 게이지 사망은 곡을 끝내지 않는다(게이지 락)**.
- **속도(freq)**: 오디오 재생 속도 변경은 Phase B 의 클럭 재설계와 직접 얽힌다. G 에서는 **`freq` 를 UI·모델에만 넣고 실제 적용은 `100` 고정**으로 두거나, Phase B 의 보간 클럭이 이미 들어왔으면 `song_us` 스케일링으로 적용한다(**Phase B 이후 재확인**).
- **기록·IR**: 연습 플레이는 결정 12 대로 **기록·제출 대상이 아니다**. `ir_submission_block_reason` 에 `practice` 사유를 추가한다(G8 통합 갈래 소유).
- **진입**: 곡선택에서 별도 키(예: F2)로 `Stage::Practice` 진입 → 패널에서 값 조정 → 시작. 종료 시 패널로 복귀해 값이 유지된다(차트별로 `practice.ron` 에 마지막 값 저장, 키 = chart_key).

---

## 7. 컨트롤러 입력 (결정 7 — gilrs 먼저)

### 7.1 레퍼런스 상수

`BMControllerInputProcessor.java`: `:45` `AXIS_LENGTH = 8`, `:61` `duration = 16`(ms, 버튼 디바운스 — `:136` `microtime >= buttontime[button] + duration * 1000` 인 경우에만 상태 변경 수용), `:81` `TICK_MAX_SIZE = 0.009f`, `:218` `computeAnalogDiff`(±1 을 넘는 차이는 `2 + TICK_MAX_SIZE/2` 로 랩어라운드 보정 후 `TICK_MAX_SIZE` 로 나눠 tick 수 산출, 부호에 따라 ceil/floor), `:333` 아날로그 알고리즘 V1(방향 전환 감지 + 정지 카운터), `:414` V2(임계 이내 이동을 tick 카운터로 누적해 2회 이상이면 스크래치 인정).

### 7.2 rbms 매핑 모델 (`apps/rbms-player/src/gamepad.rs`)

```rust
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum PadBinding {
    Button(u32),                       // gilrs Button 코드 (Unknown 포함, 원시 코드로 저장)
    Axis { axis: u32, positive: bool },// 2키 스크래치: 축 방향 하나가 곧 1개 레인 입력
    AnalogScratch { axis: u32 },       // 아날로그 턴테이블: 회전 방향이 좌/우 2레인으로 분해
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct PadConfig {
    pub enabled: bool,
    pub device_name: Option<String>,           // 재연결 시 재바인딩용
    pub lanes: std::collections::BTreeMap<String, Vec<Option<PadBinding>>>, // mode_config_key(mode) -> lane 별
    pub controls: std::collections::BTreeMap<String, PadBinding>,           // ControlAction::label 기준
    pub debounce_ms: u32,                      // 기본 16 (레퍼런스 duration)
    pub analog_mode: AnalogMode,               // None | V1 | V2
    pub analog_threshold: u32,                 // 레퍼런스 analogScratchThreshold 대응, 기본 100 (1..=1000 클램프)
    pub axis_deadzone: f32,                    // 기본 0.2 (2키 스크래치 판정 임계)
}

#[derive(Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum AnalogMode { Off, V1, V2 }

pub const TICK_MAX_SIZE: f32 = 0.009;          // 레퍼런스 BMControllerInputProcessor.java:81
pub const DEFAULT_DEBOUNCE_MS: u32 = 16;       // 레퍼런스 :61
pub const DEFAULT_ANALOG_THRESHOLD: u32 = 100;   // 레퍼런스 PlayModeConfig.java:514
pub const ANALOG_THRESHOLD_RANGE: std::ops::RangeInclusive<u32> = 1..=1000; // 레퍼런스 :150 클램프

pub fn compute_analog_diff(old_value: f32, new_value: f32) -> i32;  // 레퍼런스 :218 이식

pub struct AnalogScratch { mode: AnalogMode, threshold: u32, counter: u64, tick_counter: i32, old_x: f32, active: bool, right: bool }
impl AnalogScratch {
    pub fn new(mode: AnalogMode, threshold: u32) -> Self;
    /// `plus` = 양방향 레인인지. 레퍼런스 analogScratchInput 과 동일한 상태 기계.
    pub fn input(&mut self, current_x: f32, plus: bool) -> bool;
}

pub struct PadState { /* gilrs Gilrs + 버튼별 마지막 변경 시각(us) + AnalogScratch[8] */ }
impl PadState {
    pub fn new(cfg: &PadConfig) -> Option<PadState>;    // gilrs 초기화 실패 시 None (키보드로 계속 동작)
    /// 프레임마다 호출. 디바운스를 통과한 (lane, press) 만 반환한다.
    pub fn poll(&mut self, now_us: i64, cfg: &PadConfig, mode: rbms_model::Mode) -> Vec<PadEvent>;
}
pub enum PadEvent { Lane { lane: usize, press: bool }, Control(crate::keyconfig::ControlAction) }
```

- **디바운스**: 버튼별 마지막 변경 `now_us < last_us + debounce_ms * 1000` 이면 상태 변경을 무시한다(레퍼런스 `:136` 과 동일 부등호).
- **2키 스크래치**: `Axis { positive }` 는 축 값이 `deadzone` 을 넘으면 press, 되돌아오면 release. 아날로그 모드가 켜져 있으면 이 경로 대신 `AnalogScratch` 를 쓴다.
- **아날로그 턴테이블**: 축 값 랩어라운드(-1↔+1)를 `compute_analog_diff` 로 보정. 상태 기계 전문은 아래 표대로 이식한다(레퍼런스 `:359-410`(V1), `:444-491`(V2) 전문 통독, 2026-09-09).

`counter` 는 시간이 아니라 **`input()` 호출 횟수**(프레임 카운터)다. 두 버전 모두 `old_x` 초기값이 `10`(> 1)이라 첫 호출은 값만 흡수하고 `false` 를 반환한다.

| 단계 | V1 (`:359-410`) | V2 (`:444-491`) |
|---|---|---|
| 첫 호출(`old_x > 1`) | `old_x = cur; active = false; return false` | 동일 |
| 방향 판정 | 두 방향 이동량을 직접 비교해 짧은 쪽을 채택(랩어라운드 고려) | `ticks = compute_analog_diff(old_x, cur)`, `right = ticks >= 0` |
| 이동 발생 + 이미 active + 방향 반전 | `right = now_right` 만 갱신(계속 active) | `right = now_right; active = false; tick_counter = 0` (**재축적 필요**) |
| 이동 발생 + 비active | `active = true; right = now_right` (**즉시 인정**) | `tick_counter == 0 \|\| counter <= threshold` 일 때만 `tick_counter += ticks.abs()`; `tick_counter >= 2` 면 `active = true; right = now_right` |
| 이동 발생 공통 | `counter = 0; old_x = cur` | 동일 |
| 만료 검사(이동 여부 무관, 매 호출) | `counter > threshold && active` → `active = false; counter = 0` | `counter > threshold * 2` → `active = false; tick_counter = 0; counter = 0` (**active 여부 무관**) |
| 오버플로 가드 | `counter == u64::MAX` 면 0 으로 리셋 | 없음 |
| 마지막 | `counter += 1`; `plus ? active && right : active && !right` | 동일 |

Rust 이식 시 `counter: u64`, `tick_counter: i32`, `old_x: f32 = 10.0` 로 초기화하고 위 순서를 그대로 지킨다(순서를 바꾸면 만료 프레임이 1 어긋난다).
- **키 컨피그 UI**: 기존 `Stage::KeyConfig`(`keyconfig.rs`, `app_input.rs` `keyconfig_input`)에 **PAD 열**을 추가한다. 바인딩 캡처는 "다음 입력 대기" 모드로 gilrs 이벤트를 하나 받아 `PadBinding` 으로 확정. 저장은 `keyconfig.ron` 안의 신규 필드(`#[serde(default)] pad: PadConfig`)로 두어 기존 파일과 후방호환.

### 7.3 마우스 스크래치

레퍼런스 `MouseScratchInput.java` 대응. G 에서는 **인터페이스만** 둔다: `PadBinding::MouseWheel { up: bool }` 를 추가하지 않고, `apps/rbms-player/src/gamepad.rs` 에 `pub enum ScratchSource { Keyboard, Pad, Mouse }` 만 정의해 확장 지점을 남긴다(구현은 후속).

### 7.4 MIDI (후순위)

결정 7 대로 이번 Phase 에서 구현하지 않는다. `PadEvent` 와 동일한 이벤트 타입을 반환하는 `trait ExternalInput { fn poll(&mut self, now_us: i64) -> Vec<PadEvent>; }` 를 두어 `midir` 백엔드를 나중에 같은 자리에 끼울 수 있게 한다.

---

## 8. 시스템 사운드

레퍼런스 `SystemSoundManager.java:129-151` `SoundType` 전체:

| 상수 | 파일명 | 재생 계기 |
|---|---|---|
| SCRATCH | `scratch.wav` | 곡선택 커서 이동 |
| FOLDER_OPEN / FOLDER_CLOSE | `f-open.wav` / `f-close.wav` | 폴더 진입 / 이탈 |
| OPTION_CHANGE / OPTION_OPEN / OPTION_CLOSE | `o-change.wav` / `o-open.wav` / `o-close.wav` | 옵션 값 변경 / 패널 열기 / 닫기 |
| PLAY_READY / PLAY_STOP | `playready.wav` / `playstop.wav` | 플레이 시작 / 중단 |
| RESULT_CLEAR / RESULT_FAIL / RESULT_CLOSE | `clear.wav` / `fail.wav` / `resultclose.wav` | 결과 클리어 / 실패 / 닫기 |
| COURSE_CLEAR / COURSE_FAIL / COURSE_CLOSE | `course_clear.wav` / `course_fail.wav` / `course_close.wav` | 코스 결과 |
| GUIDESE_PG/GR/GD/BD/PR/MS | `guide-pg.wav` … `guide-ms.wav` | 판정별 가이드음(옵션) |
| SELECT | `select.wav` | 선택 확정(디렉터리 세트) |
| DECIDE | `decide.wav` | 결정(디렉터리 세트) |

파일 해석: 레퍼런스는 BGM 폴더를 `select.wav` 기준으로, 사운드 폴더를 `clear.wav` 기준으로 스캔해 "세트"를 찾는다(`SystemSoundManager.java:46,49`).

rbms 스펙 (`apps/rbms-player/src/syssound.rs`):

```rust
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum SystemSound { Scratch, FolderOpen, FolderClose, OptionChange, OptionOpen, OptionClose,
    PlayReady, PlayStop, ResultClear, ResultFail, ResultClose,
    CourseClear, CourseFail, CourseClose,
    GuidePg, GuideGr, GuideGd, GuideBd, GuidePr, GuideMs, Select, Decide }

impl SystemSound { pub fn file_stem(self) -> &'static str; }   // "scratch", "f-open", ...

pub struct SystemSoundSet { /* SystemSound -> 디코딩된 PCM */ }
impl SystemSoundSet {
    /// `dir` 하위에서 stem + [wav, ogg, flac, mp3] 순으로 해석. 없으면 그 슬롯은 무음.
    pub fn load(dir: &std::path::Path) -> SystemSoundSet;
    pub fn play(&self, engine: &rbms_audio::Engine, s: SystemSound, gain: f32);
}
```

해석 규칙은 기존 `resolve_file`(`main.rs:389`)의 확장자 폴백과 같은 방식을 쓴다. 사운드 폴더는 설정(`PlaySettings` 신규 필드 `sound_folder: Option<String>`)으로 지정하며, 미지정이면 전부 무음(현 동작과 동일). 가이드음은 별도 토글(`guide_se: bool`)로 기본 off.

볼륨은 Phase B 의 3분리(마스터/BGM/키음) 중 **시스템 사운드 채널**을 쓰되, Phase B 미착수 상태면 마스터 게인에 직결한다(**Phase B 이후 재확인**).

---

## 9. 갈래 분할 (파일 소유권 — 겹침 없음)

> **G0 은 직렬 선행, G8 은 직렬 후행.** 그 사이 G1~G7(+조건부 G9)만 병렬로 돈다.
> 아래는 **파일 단위 배타 소유**다. 한 갈래가 소유하지 않은 파일은 읽기만 하고 절대 수정하지 않는다.

### 9.1 실행 창(window)

| 창 | 동시 실행 갈래 | 규칙 |
|---|---|---|
| W1 (직렬) | G0 | 혼자 돈다. 끝나야 W2 시작 |
| W2 (병렬) | G1, G2, G3, G4, G5, G6, G7, (G9) | 서로 파일을 공유하지 않는다 |
| W3 (직렬) | G8 | 혼자 돈다. W2 전부 완료 후 시작 |

`apps/rbms-player/src/main.rs` 는 **W1 의 G0 과 W3 의 G8 이 서로 다른 시점에** 소유한다(동시 소유 아님). W2 의 어느 갈래도 이 파일을 열지 않는다.

### 9.2 갈래별 소유 파일 (전량 명시)

| 갈래 | 소유 파일 (이것만, 전부) | 내용 |
|---|---|---|
| **G0** (W1) | `Cargo.toml`<br>`apps/rbms-player/Cargo.toml`<br>`apps/rbms-player/src/main.rs`<br>`crates/rbms-songdb/Cargo.toml`<br>`crates/rbms-scoredb/Cargo.toml`<br>`crates/rbms-course/Cargo.toml` | 신규 크레이트 3개 골격 + 워크스페이스 members·deps 등록. `main.rs` 에는 **`mod library; mod scoredb_store; mod course_ui; mod course_ir; mod practice; mod gamepad; mod syssound; mod ir_ext;` 선언과 컴파일되는 빈 스텁만** 넣는다(로직 0). 각 신규 크레이트의 `src/lib.rs` 는 **해당 갈래가 만든다**(G0 은 `Cargo.toml` 만) — 단 `cargo build` 를 통과시켜야 하므로 G0 이 `src/lib.rs` 를 빈 파일로 생성한 뒤 즉시 소유권을 넘긴다(생성 커밋 이후 G0 은 재수정 금지) |
| **G1 곡DB** | `crates/rbms-songdb/src/**`<br>`apps/rbms-player/src/library.rs` | §3 전부(DDL·API·증분 스캔·병렬·취소) |
| **G2 스코어DB** | `crates/rbms-scoredb/src/**`<br>`apps/rbms-player/src/scoredb_store.rs` | §4 전부(DDL·API·RON 마이그레이션·리플레이 GC 계획) |
| **G3 코스** | `crates/rbms-course/src/**`<br>`apps/rbms-player/src/course_ui.rs`<br>`apps/rbms-player/src/course_ir.rs` | §5. **`crates/rbms-ir` 는 한 줄도 수정하지 않는다**(Phase I 소유). 코스 IR 헬퍼는 `course_ir.rs` 에서 `rbms_ir` 의 공개 타입만 소비한다 |
| **G4 연습** | `apps/rbms-player/src/practice.rs` | §6 |
| **G5 컨트롤러** | `apps/rbms-player/src/gamepad.rs`<br>`apps/rbms-player/src/keyconfig.rs` | §7. `keyconfig.rs` 에 `#[serde(default)] pad: PadConfig` 필드 추가·PAD 열 데이터 모델 |
| **G6 시스템 사운드** | `apps/rbms-player/src/syssound.rs` | §8 |
| **G7 IR 확장 지점** | `apps/rbms-player/src/ir_ext.rs`<br>`docs/reference/ir-multi.md` | §10. 데이터 모델과 확장 지점만. Phase I 산출물을 읽고 시작 |
| **G9 bmson** (조건부 — §2 보류, 결정 10 재확인 후에만 활성화) | `crates/rbms-parser/Cargo.toml`<br>`crates/rbms-parser/src/bmson.rs`<br>`crates/rbms-parser/src/lib.rs` | bmson 파서 + 모델 매핑(judgerank/TOTAL 퍼센트 변환). 스캔 확장자 필터(`is_chart`) 확장은 **G8 에 배선 지시로 위임** |
| **G8** (W3) | `apps/rbms-player/src/main.rs`<br>`apps/rbms-player/src/app_select.rs`<br>`apps/rbms-player/src/app_play.rs`<br>`apps/rbms-player/src/app_input.rs`<br>`apps/rbms-player/src/settings.rs`<br>`apps/rbms-player/src/scores.rs`<br>`apps/rbms-player/src/replay.rs`<br>`apps/rbms-player/src/folders.rs` | 전 갈래의 호출부 배선: 스캔→SongDb 교체, ScoreBook→ScoreDb 교체, Stage 확장(Course/CourseResult/Practice), 입력 폴링에 `PadState::poll` 추가, 시스템 사운드 트리거, `ir_submission_block_reason` 에 `practice` 사유 추가, `settings.rs` 에 `ir_profiles`·`sound_folder`·`guide_se` 필드 추가 |

### 9.3 소유권 검증

- **W2 병렬 집합의 교집합은 공집합이다.** G1∩G2∩G3∩G4∩G5∩G6∩G7∩G9 = ∅ — 어느 두 갈래를 골라도 공통 파일이 없음을 위 목록에서 직접 대조해 확인했다. 크레이트 트리는 갈래마다 서로 다른 크레이트(`rbms-songdb` / `rbms-scoredb` / `rbms-course` / `rbms-parser`)이고, 플레이어 쪽은 갈래마다 서로 다른 단일 파일(`library.rs` / `scoredb_store.rs` / `course_ui.rs`+`course_ir.rs` / `practice.rs` / `gamepad.rs`+`keyconfig.rs` / `syssound.rs` / `ir_ext.rs`)이다.
- **G0 ∩ W2 = ∅**, **G8 ∩ W2 = ∅**. G0 ∩ G8 = { `apps/rbms-player/src/main.rs` } 이지만 두 갈래는 **동시에 돌지 않는다**(W1 / W3).
- **이 스펙이 건드리는 모든 파일은 소유자가 정확히 1개다.** 소유자 없는 파일은 없고, 소유자가 2개인 파일도 없다(main.rs 는 시점 분리).
- **`crates/rbms-ir/**` 와 `apps/rbms-player` 의 NETWORK 탭 관련 코드는 어느 G 갈래도 소유하지 않는다**(Phase I 소유). 필요한 변경은 전부 배선 문서로만 남긴다.

**금지**: W2 의 어느 갈래도 `main.rs` / `app_*.rs` / `settings.rs` / `scores.rs` / `replay.rs` / `folders.rs` / `crates/rbms-ir/**` 를 수정하지 않는다. 필요한 호출부 변경은 각 갈래가 **`docs/plan/phase-g-wiring/<갈래>.md` 에 패치 지시로 적어** G8 이 반영한다(파일 경로·함수명·삽입 위치·정확한 코드 블록을 포함할 것 — G8 은 이 문서만 보고 배선한다).
**Phase I 충돌**: G3·G7 은 Phase I 커밋 이후 `crates/rbms-ir/src/lib.rs`(트레이트 시그니처)와 플레이어 IR 호출부를 다시 읽고 착수한다. 라인번호 앵커 금지, 함수명 앵커.

---

## 10. 라이벌 / IR 랭킹 패널 / 복수 IR (Phase I 위의 확장)

Phase I 가 이미 넣는 것(계획 §Phase I): 곡선택 IR 랭킹 패널(비동기 캐시·라이벌 행), 라이벌 관리 UI, 리플레이 업/다운로드, 설정 동기화. **G 는 그 위에 다음만 얹는다.**

```rust
// apps/rbms-player/src/ir_ext.rs
pub struct IrProfile { pub name: String, pub base_url: String, pub token: Option<String>, pub enabled: bool }
pub struct MultiIr { pub profiles: Vec<IrProfile>, pub primary: usize }
impl MultiIr {
    /// 제출은 enabled 프로필 전부에 대해 병렬로, 실패는 프로필별로 격리한다(하나 실패해도 나머지 진행).
    pub fn submit_all(&self, servers: &[std::sync::Arc<dyn rbms_ir::ScoreServer>], sub: &rbms_ir::ScoreSubmission) -> Vec<(String, Result<rbms_ir::SubmitResponse, rbms_ir::IrError>)>;
    /// 랭킹 패널은 primary 프로필만 조회한다(패널 탭으로 전환 가능).
    pub fn primary_server(&self) -> Option<usize>;
}
```

- 저장: `settings.rs` 의 단일 `server_url`/`player_id` 를 유지하면서 `#[serde(default)] ir_profiles: Vec<IrProfile>` 을 추가(빈 벡터면 기존 단일 서버 동작). **필드 추가는 G8 소유**.
- 랭킹 패널·라이벌 UI 자체는 Phase I 산출물을 재사용하고, G 는 **프로필 선택 축만** 추가한다.
- 코스 랭킹은 §5.5 의 `course_ir.rs` 헬퍼를 통해 primary 프로필에만 제출·조회한다. **`ScoreServer::course_ranking` 은 `crates/rbms-ir/src/lib.rs:44` 에 이미 기본 구현(`Err(IrError::Unsupported)`)이 있으므로 새 메서드를 정의하지 말고 그대로 호출한다.** `Unsupported` 는 오류가 아니라 "이 서버는 코스 랭킹 없음"으로 처리해 패널을 숨긴다.

---

## 11. 순서 · 단계별 테스트

| 순서 | 단계 | 검증(테스트) |
|---|---|---|
| 1 | G0 스캐폴드 | `cargo build --workspace` 통과, `cargo test --workspace` 기존 수치 유지(1,060 통과 기준) |
| 2 | G1 스키마·API | `open_in_memory` + `migrate` 후 `PRAGMA table_info` 로 컬럼 전수 확인 / `upsert_batch` → `all_songs` 왕복 / `stamp_index` 가 `(date,size)` 를 정확히 반환 |
| 3 | G1 증분·병렬·취소 | tempdir 에 더미 차트 N개 생성 → 1차 스캔 `parsed == N` → 2차 스캔 `skipped == N && parsed == 0` → 1개 touch 후 `parsed == 1` / 파일 삭제 후 `removed == 1` / `cancel` 을 즉시 세우면 `Cancelled` 반환하고 프로세스가 남지 않음 |
| 4 | G2 스키마·record_play | `record_play(updates_best=true)` 후 `best()` 갱신, `updates_best=false` 면 EX 미갱신·playcount 갱신(결정 12) / `player_stat` 일자 누적 / `scorelog` 행 증가 |
| 5 | G2 마이그레이션 | 픽스처 `scores.ron`(어시스트 1건 포함) → 마이그레이션 후 `best_clear` 가 `scores.rs:110` 기존 동작과 동일 / 실패 주입 시 SQLite 삭제·RON 유지 |
| 6 | G2 리플레이 GC | 차트당 5개 리플레이 → `keep_recent_per_chart=3` + best 유지 시 삭제 목록이 정확히 2개 |
| 7 | G3 코스 모델 | 레퍼런스 JSON 픽스처(배열/단일, unknown 필드 포함) 로드 / 제약 토큰 14종 `token`↔`from_token` 왕복 / **이름이 빈 코스는 `validate()==true` 이고 `name == "No Course Title"`** / **그룹 중복 제약(`grade`+`grade_mirror`)은 `validate()==true` 이고 제약이 첫 원소 1개만 남음** / 제목이 빈 차트는 `"course 1"` 로 채워짐 / **`charts` 가 비면 `validate()==false`** / `hash()` 가 차트 순서에 민감 |
| 8 | G3 진행 | 3스테이지 시뮬: 게이지 이월, 2스테이지에서 0 → `Failed` 이고 3스테이지 미실행 / trophy 선택이 missrate·scorerate 경계에서 정확 |
| 9 | G4 연습 | `clamp_start` 상한 `round_down(last-2000,100)`·하한 0, **증가 시 `end = max(end, start+1000)`(내림 없음)** / `clamp_end` 하한 `round_down(start+1000,100)`·상한 `round_down(last+1000,100)` / turbo·analog 스텝 2500/1000/100 / PMS gaugetype>=3 에서 startgauge 100 클램프 |
| 10 | G5 입력 | `compute_analog_diff` 를 레퍼런스 랩어라운드 케이스(0.995→-0.995 등)로 대조 / 디바운스: 15ms 뒤 입력 무시·17ms 뒤 수용 / **§7.2 표의 V1·V2 상태 전이 전수**: 첫 호출 `false`, V1 은 1회 이동으로 즉시 인정·V2 는 `tick_counter>=2` 필요, V1 만료 `counter > threshold`·V2 만료 `counter > threshold*2`, V2 방향 반전 시 `active=false`+`tick_counter=0` / `analog_threshold` 기본 100·클램프 1..=1000 / `keyconfig.ron` 구파일(pad 필드 없음) 로드 시 기본값 |
| 11 | G6 사운드 | 22종 stem 이 전부 유일하고 비어있지 않음 / 폴더에 파일이 없으면 `play` 가 패닉 없이 무음 |
| 12 | G7 다중 IR | 프로필 3개 중 1개가 에러를 반환해도 나머지 2개 결과가 그대로 반환됨 |
| 13 | G8 통합 | 앱 기동 → 최초 스캔 → 곡선택 표시 → 플레이 → 결과 기록이 SQLite 에 남는지 실제 실행으로 확인. `cargo fmt --check`, `cargo clippy --workspace --all-targets`(경고 기준선 초과 금지), `cargo test --workspace` |

---

## 12. 리스크

| # | 리스크 | 완화 |
|---|---|---|
| R1 | `main.rs` 는 1,740줄 단일 파일이라 G8 통합이 병목이자 충돌 지점 | G0 에서 mod 선언을 미리 전부 넣고, G1~G7 은 배선 지시를 문서로만 넘긴다. Phase C(구조 개편)가 먼저 끝나면 `Stage` enum·`PlaySession` 분리 결과 위에 얹는다 |
| R2 | Phase I 가 `crates/rbms-ir`·`apps/rbms-player` 를 동시 수정 | G3·G7 은 신규 파일만 소유하고 Phase I 커밋 이후 착수. 라인번호 앵커 금지, 함수명 앵커 |
| R3 | Phase B(오디오 클럭)·Phase D(9종 게이지)가 연습 모드의 `freq`·`gauge_type` 전제 | `freq` 는 100 고정으로 시작, `gauge_type` 은 현재 게이지 수로 클램프. 두 Phase 완료 후 확장 |
| R4 | rusqlite `bundled` 로 빌드 시간·바이너리 크기 증가, 크로스 컴파일(Windows/macOS CI) 영향 | G0 직후 3 플랫폼 CI 를 한 번 돌려 확인. 실패 시 `libsqlite3-sys` 링크 옵션 조정 |
| R5 | WAL 모드 DB 파일이 `.sqlite-wal`/`.shm` 을 만들어 기존 `.bak` 백업 관례와 다름 | 백업은 `VACUUM INTO` 로 단일 파일 스냅샷 |
| R6 | 대형 라이브러리 첫 스캔이 여전히 수십 초 | 배치 커밋으로 부분 결과를 즉시 조회 가능하게 하고, 스캔 중에도 이미 들어온 곡으로 곡선택을 그린다 |
| R7 | 코스 제약(NoGood/NoGreat)은 판정 엔진 개입이 필요 | Phase D 의 판정 데이터화 이후 배선. G 에서는 제약을 저장·표시하고 미적용 배지를 띄운다 |
| R8 | 병렬 파싱이 디스크 I/O 바운드면 스레드 증가가 역효과 | 워커 수를 설정으로 노출(기본 `min(cpu, 8)`), 스캔 시간 로그를 남겨 실측 후 조정 |

---

## 13. 미확인 사항 (2026-09-09 재검증 후 잔여)

> 아래는 **아직 확인되지 않은 것만** 남긴다. 이번 검증에서 해소된 항목은 §14 에 답과 함께 옮겼다.

1. **bmson 범위 — 결정 10 과 상충(사용자 재확인 필요, 착수 차단)**. 결정 10 채택안은 "bmson 은 Phase G 별도 항목"인데 초판 스펙은 이를 비목표로 뒀다. **A안**: G9(조건부 갈래, §9.2)를 활성화해 이번 Phase 에 포함 / **B안**: 별도 Phase 로 분리하고 결정 10 을 개정. 사용자 확인 전에는 어느 쪽도 착수하지 않는다.
2. **rusqlite / gilrs 정확 버전** — 이 환경에서 crates.io 를 조회하지 않았다. 착수 시 최신 버전을 확인해 정확히 고정할 것. `rusqlite` 는 `features = ["bundled"]` 필요.
3. **레퍼런스 `score.state` / `scorelog.state` 값의 의미** — `ScoreData.java:377` 에 `setState` 만 있고 의미를 규정하는 상수가 없다(값 도메인 미확인). rbms 는 자체 의미(0=정상, 1=마이그레이션 유래)로 쓰고, 레퍼런스 DB 임포트 시에는 이 컬럼을 무시한다.
4. **`charthash` 를 실제로 채울 시점** — 산출식은 확인됐다(§3.2: `SHA-256(model.toChartString())`). 다만 rbms 에는 `toChartString()` 에 대응하는 정규 직렬화가 아직 없다. G 에서는 빈 문자열로 두고, Phase A 파서 정비 때 동일 바이트열을 정의해 채운다.
5. **RandomCourseData 규칙** — `select/BarManager.java:567,583`, `select/RandomCourseData.java` 에 랜덤 코스가 존재함만 확인했고 선곡 규칙(레벨 범위·중복 배제·시드)은 미확인. 이번 스펙은 랜덤 코스를 **범위에서 제외**했다(§2 비목표).
6. **시스템 사운드 세트 디렉터리 규약** — 레퍼런스가 BGM/사운드 폴더를 분리(`SystemSoundManager.java:46,49`)하는 반면 rbms 는 단일 폴더로 설계했다. 사용자 폴더 호환이 필요하면 재확인.
7. **macOS 대소문자 무시 파일시스템에서의 경로 중복** — `song.path` 정규화 정책을 대소문자까지 확장할지 미결.
8. **Phase C 와의 크레이트 배치 충돌** — 계획 §2 Phase C 가 `rbms-store`/`rbms-library` 를 신설한다. Phase C 가 먼저 끝나면 `rbms-songdb`/`rbms-scoredb` 를 신규 크레이트로 두지 말고 그 두 크레이트 안으로 넣어야 한다. 착수 시 Phase C 상태 확인 필요.
9. **`freq`(재생 속도) 적용 가능 여부** — Phase B 의 보간 클럭·룩어헤드 스케줄이 들어온 뒤에야 판단 가능(§6.2).
10. **Phase I 의 `crates/rbms-ir` · `apps/rbms-player` 최종 형태** — 동시 수정 중이다. §5.5 의 트레이트 표(lib.rs:38/44, null.rs:27)는 2026-09-09 실측이나 **G3·G7 착수 직전 반드시 재확인**한다.

---

## 14. 비평 반영 (2026-09-09)

완전성 비평의 지적을 전부 코드·레퍼런스 원문으로 대조해 아래와 같이 처리했다.

### 14.1 수정한 것

| # | 지적 | 확인한 근거 | 조치 |
|---|---|---|---|
| 1 | 결정 10 위반 — bmson 을 "결정 10 근거"로 비목표 처리 | `docs/acknowledge/2026-09-09-enhancement-decisions.md:16` = "bmson은 Phase G 별도 항목" (계획 `:261` A안과 동일) | §2 에서 잘못된 인용 제거 → **보류(사용자 재확인)** 로 승격, §13-1 등재, §9.2 에 조건부 갈래 **G9** 신설 |
| 2 | `CourseData.validate()` 조건이 반대인데 테스트로 고정 | `CourseData.java:99-131` 전문: 이름 빈 값은 `"No Course Title"` 자동 채움, 그룹 중복은 **첫 원소만 남기고 제거 후 `true`**, 실패는 `hash` 빈/원소 null/`sd.validate()` 실패뿐 | §5.1 에 실동작 표 추가, §5.2 `validate()` 를 `&mut self` **정규화 함수**로 재정의, `load_dir` 계약 명시, §11 테스트 7 을 정규화 기대값으로 교체 |
| 3 | 파일 소유권 구멍 — `crates/rbms-ir/src/lib.rs` mod 선언이 무주공산 | `crates/rbms-ir/src/lib.rs:1-5` 가 mod 선언부이며 Phase I 소유 | `crates/rbms-ir/src/course_ext.rs` 안 **폐기**. 헬퍼를 **`apps/rbms-player/src/course_ir.rs`**(G3 소유, mod 선언은 G0)로 이동 → G 는 `rbms-ir` 를 한 줄도 수정하지 않음 |
| 4 | 레퍼런스 테이블명 오기 | `ScoreDataLogDatabaseAccessor.java:22` `TABLE_NAME = "scoredatalog"` | §4.1 정정 + rbms 의 `scorelog` 는 자체 명명임을 명시(임포트 매핑 규칙 추가) |
| 5 | submit_course 위치 오기 + `course_ranking` 누락 | 트레이트 정의 `lib.rs:38`, null 구현 `null.rs:27`, `course_ranking` 기본 구현 `lib.rs:44` | §5.5 에 IR 트레이트 현황 표 추가, §10 에 "새로 정의 말고 기존 `course_ranking` 호출" 명시(중복 정의 위험 제거) |
| 6 | 마우스 스크래치 범위 축소가 비목표에 미기재 | 계획 `:237` 이 Phase G 에 마우스 스크래치를 나열 | §2 비목표에 **명시적 범위 축소**로 등재 |
| 7 | 앵커 기준 커밋 표기 불일치 | `git rev-parse --short HEAD` = `72bffa5` | 헤더 갱신 |

### 14.2 반려한 것 (비평이 틀림)

| 지적 | 반려 근거 |
|---|---|
| "END TIME 하한에 100 단위 내림이 없다 (`Math.max(endtime, starttime+1000)` 뿐)" | 비평이 인용한 `PracticeConfiguration.java:539` 는 **STARTTIME 증가 시의 부수효과**이고, ENDTIME 요소의 하한은 `:549` `minEndTime = roundDownTo(property.starttime + 1000, 100)` 로 **내림이 실재**한다. 스펙의 원 서술이 맞다. 다만 상한도 `:548` `roundDownTo(마지막+1000, 100)` 으로 내림이 걸려 있어(원 스펙은 내림을 누락) 이 부분만 §6.1 표로 정밀화했다. |

### 14.3 해소한 "미확인 사항"

| 초판 §13 | 답 (2026-09-09 실측) |
|---|---|
| `feature`/`content` 비트 정의 | `SongData.java:24-36` 전량 확인 → §3.2 표로 확정. rbms 도 **레퍼런스 값 그대로 채택**(자체 정의 폐기) |
| `charthash` 산출식 | `SongData.java:220-222` = `SHA-256(model.toChartString() 바이트)` hex. 잔여는 rbms 쪽 직렬화 부재뿐(§13-4) |
| `avgjudge` 단위 | `BMSPlayer.java:922-930` 이 `getMicroPlayTime()` 절댓값을 누적하고 미판정 노트를 `1000000` 으로 계산 → **마이크로초**. 필드는 `long`(`ScoreData.java:87`, 기본 `Long.MAX_VALUE` — 초판의 `Integer.MAX_VALUE` 는 오기). §4.2 DDL 의 `avgjudge` 기본값은 SQLite 정수 상한 이슈를 피하려 `2147483647` 을 유지하되 **단위는 us** 로 확정 |
| `analogScratchThreshold` 기본값 | `PlayModeConfig.java:514` = **100**, `:150` 에서 `1..1000` 클램프 → §7.2 기본값을 8→100 으로 정정하고 클램프 상수 추가 |
| AnalogScratch V1/V2 `counter` 만료 조건 | `BMControllerInputProcessor.java:359-410`(V1)·`:444-491`(V2) 전문 통독 → §7.2 상태 전이 표로 확정. `counter` 는 **호출 횟수 카운터**, V1 만료 `counter > threshold && active`, V2 만료 `counter > threshold*2`(active 무관, `tick_counter` 도 리셋) |
| `CourseData.validate()` 정확 조건 | 위 14.1-2 참조 |
