# G2 (스코어DB) 배선 지시

> 대상: W3 통합 갈래(G8). 이 문서만 보고 배선한다. 근거는 스펙 `docs/plan/2026-09-09-phase-g-spec.md` §4.
> G2 가 소유·완성한 것: `crates/rbms-store/src/scoredb.rs` + `scoredb/{schema.rs,schema.sql,migrate.rs,replay_gc.rs,tests.rs}`,
> `apps/rbms-player/src/scoredb_store.rs` + `scoredb_store/tests.rs`. 이 파일들은 G8 이 고칠 필요가 없다(§9 의 allow 제거만 예외).

## 0. 배선의 형태 — `ScoreBook` 은 남고 저장소만 바뀐다

스펙 §4.4-7 그대로다. `ScoreBook` 은 **삭제하지 않고 SQLite 의 읽기 캐시**로 남긴다.

- **쓰기**: 결과 화면이 `ScoreDb::record_play` 로 직접 쓴다. `scores.ron` 재직렬화는 사라진다.
- **읽기**: 기동 시 `scorelog` 전량을 읽어 `ScoreBook::from_records` 로 만든다.
  → `for_md5` / `best_ex_for_md5` / `best_clear_for_md5(_in_ln_mode)` 를 쓰는 **곡선택·플레이·결과 화면 20여 곳은 한 줄도 안 고친다.**
- 책이 표현하지 못하는 것(플레이 1회 로그, 일자 통계, early/late 분리, 리플레이 보존)은 `ScoreDb` 에 직접 묻는다.

아래 패치는 **1·2번이 필수**, 3~5번은 선택(각 항목에 표시).

---

## 1. [필수] `apps/rbms-player/src/lib.rs` — `AppShared` 에 DB 를 얹는다

### 1-1. 필드 추가

**위치**: `struct AppShared` 의 `scores: ScoreBook,` / `scores_path: PathBuf,` 두 줄 **바로 아래**.

```rust
    /// The durable store the book above is a read-through cache of. `None` when the database could
    /// not be opened, in which case this run browses and plays but records nothing.
    scoredb: Option<rbms_store::scoredb::ScoreDb>,
    /// Where saved replays live, and what the retention policy prunes.
    replay_dir: PathBuf,
```

### 1-2. 생성부 교체

**위치**: `AppShared` 를 만드는 함수 안, 아래 두 줄(현재 `favorites_path` 계산 바로 위).

교체 전:

```rust
        let scores_path = settings_path.parent().map(|d| d.join("scores.ron")).unwrap_or_else(|| PathBuf::from("scores.ron"));
        let scores = ScoreBook::load(&scores_path);
```

교체 후:

```rust
        let config_dir = settings_path.parent().map(Path::to_path_buf).unwrap_or_else(|| PathBuf::from("."));
        let (scoredb_path, scores_path, replay_dir) = scoredb_store::score_paths(&config_dir);
        let mut scoredb = scoredb_store::open_score_db(&scoredb_path, &scores_path);
        let scores = match &scoredb {
            Some(db) => scoredb_store::book_from_db(db),
            None => ScoreBook::load(&scores_path),
        };
        if let Some(db) = scoredb.as_mut() {
            scoredb_store::run_replay_gc(db, &replay_dir, &rbms_store::scoredb::ReplayPolicy::default());
        }
```

### 1-3. 구조체 리터럴에 필드 추가

**위치**: 같은 함수의 `AppShared { … }` 리터럴, `scores,` / `scores_path,` 줄 **바로 아래**.

```rust
                scoredb,
                replay_dir,
```

### 근거

- 스펙 §4.4-1·6: `scoredb.sqlite` 가 없고 `scores.ron` 이 있으면 1회 임포트, 실패 시 SQLite 삭제·RON 유지.
  `open_score_db` 가 그 전부를 하고, 실패하면 `None` 을 돌려 **RON 폴백**으로 떨어진다(플레이어는 계속 동작).
- 스펙 §4.3 리플레이 보존: 기동 1회 적용. 매 결과마다 돌리면 파일 stat 이 결과 화면에 얹힌다.
- 이 패치가 없으면 DB 가 열리지 않아 §4 전체가 죽는다.

---

## 2. [필수] `apps/rbms-player/src/app_result.rs` — 결과 기록 경로

**위치**: `fn enter_result` 안, `if shared.replay.is_none() && !shared.config.play.autoplay {` 로 시작하는 블록 전체.

교체 전(현재):

```rust
    if shared.replay.is_none() && !shared.config.play.autoplay {
        let record = ScoreRecord {
            md5: chart.md5.clone(),
            …
            assisted: !scores_count,
        };
        shared.scores.push(record);
        shared.scores.save(&shared.scores_path);
    }
```

교체 후:

```rust
    if shared.replay.is_none() && !shared.config.play.autoplay {
        let finished = crate::scoredb_store::FinishedPlay {
            md5: &chart.md5,
            sha256: &chart.sha256,
            title: &chart.title,
            mode: shared.mode,
            ln_mode: &state.ln_mode_key,
            lamp,
            summary: &summary,
            gauge: finished_gauge,
            random: shared.config.play.random,
            seed: chart.seed,
            assist,
            played_at_ms: played_ms,
            playtime_ms: crate::scoredb_store::playtime_ms(last_time_us),
            ir_submitted: block_reason.is_none() && shared.config.network.server_url.is_some(),
            replay_file: replay_file.clone(),
        };
        let log = crate::scoredb_store::play_log(&finished);
        if let Some(db) = shared.scoredb.as_mut() {
            crate::scoredb_store::record_finished_play(db, &log, scores_count);
        }
        shared.scores.push(crate::scoredb_store::record_of(&log));
    }
```

`last_time_us` 는 `play` 가 아직 살아 있는 동안 미리 뽑아 둔다 — **`let assist = assist_level(…);` 줄 바로 아래**에 한 줄 추가:

```rust
    let last_time_us = play.last_time_us();
```

### 확인 사항

- `replay_file` 은 지금 `ScoreRecord` 로 **이동(move)** 되고 있다. 위 코드는 `clone()` 한다 —
  `replay_file` 은 이 블록 뒤에서 쓰이지 않지만, `FinishedPlay` 가 `Option<String>` 을 소유하므로 clone 이 맞다.
- `scores_count`(= `crate::updates_score(...)`)를 **그대로** 넘긴다. 결정 12 의 판단은 이미 그 함수가 내렸고,
  `record_play` 는 그 플래그만 받는다(재유도 금지 — IR 게이트와 DB 가 어긋나면 안 된다).
- `shared.scores.save(&shared.scores_path)` 는 **삭제**한다. RON 저장 경로는 이 패치로 사라진다.
- `ScoreRecord` 를 이 파일에서 더 이상 직접 만들지 않으므로, `use` 목록에서 `ScoreRecord`/`SCORE_RULE_VERSION`/
  `SCORE_LN_MODE_FROM_CHART` 가 미사용이 되면 `lib.rs` 의 `use rbms_store::{…}` 에서 정리한다
  (`ScoreBook` 은 계속 필요하다).

### 근거

- 스펙 §4.3 `record_play`: scorelog insert + score 갱신 + player_stat 누적이 **한 트랜잭션**.
- 스펙 §4.2 `scorelog`: 플레이 1회 = 1행. `shared.scores.push` 는 화면이 즉시 새 기록을 보게 하는 캐시 갱신일 뿐이다.
- 이 패치가 없으면 DB 에 아무것도 쌓이지 않아 history·통계·리플레이 GC 가 전부 빈 상태로 돈다.

---

## 3. [선택] `apps/rbms-player/src/lib.rs` — `scores_path` 정리

1-2 패치 뒤 `scores_path` 는 **RON 폴백 로드에만** 쓰인다. `AppShared.scores_path` 필드는 저장 경로가 사라졌으므로
지워도 되고(그 경우 1-3 의 `scores_path,` 도 함께 제거), 폴백 재로드를 위해 남겨도 된다. G2 는 **남기는 쪽**을 권한다 —
DB 가 열리지 않은 런에서 사용자가 무엇을 읽고 있는지 화면에 보여줄 근거가 된다.

---

## 4. [선택] 설정 행 — 리플레이 보존 정책

`rbms_store::scoredb::ReplayPolicy` 는 세 값을 갖는다. 기본값은 스펙 §4.3 그대로
(`keep_best_per_chart = true`, `keep_recent_per_chart = 3`, `max_total_bytes = 2 GiB`).
설정으로 노출하려면 `crates/rbms-config/src/schema.rs` 의 `LibraryOptions`(또는 신설 `ScoreOptions`)에

```rust
    /// How many of each chart's most recent replays the retention policy keeps.
    pub replay_keep_recent: usize,
    /// Ceiling on the replay directory in bytes; zero is no ceiling.
    pub replay_max_bytes: u64,
```

를 `#[serde(default)]` 로 추가하고, `settings.rs` 의 descriptor 표에 두 행을 넣은 뒤
1-2 의 `ReplayPolicy::default()` 를 그 값으로 바꾼다. **스펙이 요구하는 항목은 아니다.**

---

## 5. [선택] `crates/rbms-store/src/lib.rs` — 재노출

`pub mod scoredb;` 는 G0 이 이미 넣었고 경로는 `rbms_store::scoredb::…` 로 동작한다. 호출부를 짧게 하고 싶으면

```rust
pub use scoredb::{PlayLog, ScoreDb, ScoreDbError};
```

를 기존 `pub use` 목록 옆에 추가한다. **필수 아님** — 위 1·2 패치는 전부 `rbms_store::scoredb::` 경로로 쓴다.

---

## 6. G2 가 노출한 공개 표면 (요약)

### `rbms_store::scoredb`

| 항목 | 뜻 |
|---|---|
| `ScoreDb::{open, open_in_memory, migrate}` | 열기·스키마 적용. 상위 세대 DB 는 `FutureSchema` 로 거부(덮어쓰지 않음) |
| `ScoreDb::record_play(&PlayLog, updates_best) -> i64` | 플레이 1회. scorelog+score+player_stat 한 트랜잭션, scorelog id 반환 |
| `ScoreDb::best(chart_key, mode)` | 램프 우선·동률 시 EX 로 고른 **한 행**(레퍼런스 `getScoreData` 선택 규칙) |
| `ScoreDb::best_in_ln_mode(chart_key, mode, ln_mode)` | 저장된 행 그대로 |
| `ScoreDb::chart_best(chart_key)` / `chart_best_many(&[String])` | 모드·LN 플레이버를 가로질러 접은 값(곡선택 한 줄 = 램프 하나) |
| `ScoreDb::history(chart_key, limit)` / `all_plays()` | 플레이 로그 |
| `ScoreDb::day_stats(from, to)` | 일자 통계 |
| `ScoreDb::profile()` / `set_profile(&Profile)` | 플레이어 식별 행(1행) |
| `ScoreDb::replay_gc_plan(dir, &ReplayPolicy)` / `clear_replay_refs(&[String])` | 보존 계획 / 삭제 후 참조 해제 |
| `ScoreDb::backup_to(path)` | `VACUUM INTO` 단일 파일 스냅샷(스펙 §12 R5) |
| `migrate_score_book`, `open_with_import`, `MigrationReport` | RON → SQLite |
| `chart_key`, `mode_id`/`mode_name`, `gauge_id`/`gauge_token`, `random_id`/`random_token`, `utc_day_start` | 토큰 ↔ 저장 정수 |
| `JudgeCounts`(`early`/`late` + `ex_score`/`total`/`combo_breaks`), `PlayLog`, `BestScore`, `ChartBest`, `DayStat`, `Profile`, `ReplayPolicy`, `ReplayGcPlan`, `ScoreDbError` | 값 타입 |

### `apps/rbms-player/src/scoredb_store.rs`

`SCOREDB_FILE`, `SCORES_RON_FILE`, `REPLAY_DIR`, `score_paths`, `open_score_db`, `playtime_ms`,
`FinishedPlay`, `play_log`, `record_finished_play`, `record_of`, `book_from_db`, `run_replay_gc`.

---

## 7. 스펙과 다르게 한 것 (G8 이 알아야 하는 이탈)

| # | 스펙 | 실제 | 이유 |
|---|---|---|---|
| D1 | `score` PK `(chart_key, mode)` | **`(chart_key, mode, ln_mode)`** | 스펙 작성 후 Phase D 가 `ScoreRecord.ln_mode` 를 넣었다. CN 은 양끝을 판정해 노트 수·EX 상한이 LN 과 다르므로 한 행을 공유할 수 없다. `ln_mode` 를 빼면 `best_ex_for_md5_in_ln_mode` 가 재현되지 않는다. 레퍼런스도 같은 축을 쪼개며 그 컬럼을 `mode` 라 부른다(`PlayDataAccessor.java:200-205`) — rbms 는 그 이름을 이미 플레이 모드가 쓰고 있어 축을 따로 뒀다 |
| D2 | `chart_key = sha256 있으면 sha256` | **md5 우선, 없을 때만 sha256** | rbms 의 기존 기록·즐겨찾기·난이도표가 전부 md5 키다. sha256 으로 키를 잡으면 마이그레이션된 과거 기록과 새 기록이 서로 다른 행이 된다. 두 해시 모두 컬럼으로 있으므로 나중에 재키잉 가능 |
| D3 | `scorelog` 컬럼 목록 | **`ln_mode`·`playtime` 2개 추가** | `ln_mode` 는 D1 때문. `playtime` 은 `player_stat.playtime` 의 원천을 로그에도 남겨, 통계를 로그에서 재구성할 수 있게 한다(스펙은 `player_stat` 에만 두어 재구성이 불가능했다) |
| D4 | `replay_gc_plan(&self, policy)` | **`replay_gc_plan(&self, dir, policy)`** | `freed_bytes` 와 `max_total_bytes` 는 파일 크기를 알아야 계산된다. DB 는 크기를 모른다 |
| D5 | `profile() -> Option<(String,String,String)>` | **`Option<Profile>`** | 익명 3튜플은 호출부에서 순서를 틀리기 쉽다 |
| D6 | `best_many -> HashMap<String, BestScore>` | **`chart_best_many -> HashMap<String, ChartBest>`** | 곡선택은 "곡 한 줄 = 램프 하나"를 묻는다. 여러 LN 플레이버 행을 하나의 저장 행으로 합칠 수 없어 접은 뷰 타입을 따로 뒀다. 저장 행이 필요하면 `best`/`best_in_ln_mode` |
| D7 | 미기재 | `gauge`/`random`/`mode` 토큰 표가 `rbms-store` 안에 있다 | `crates/rbms-store/Cargo.toml` 은 G0 소유라 `rbms-judge`/`rbms-config` 의존을 추가할 수 없다. 표는 `PlayerConfig.java:413-430`(모드 id)·`GaugeIndex` 순서·`NoteOption::ALL` 순서를 그대로 옮겼고 왕복 테스트로 고정했다. **후속(H 단계)에서 의존을 넣고 한 곳으로 합칠 수 있다** |
| D8 | 미기재 | `assist = -1`(`ASSIST_UNRECORDED`) | `ScoreRecord.assisted` 는 불리언이라 어느 단계의 어시스트였는지 모른다. 없는 정보를 지어내지 않고 음수 표지로 남긴다 |

## 8. 알려진 제약 (스펙 §13 대응)

- `scorelog.state` 는 rbms 자체 의미다: `0` = 이 빌드가 기록, `1` = `scores.ron` 유래(early/late 분리와 `avgjudge` 없음). 스펙 §13-3 그대로.
- `avgjudge` 는 **마이크로초**(스펙 §14.3). 미기록은 `UNSET_MINIMUM`(=2,147,483,647)이며 최소값 비교라 베스트가 되지 않는다.
- `player_stat.date` 는 **UTC 일 시작 초**다(스펙 §4.2). 레퍼런스는 로컬 타임존을 쓴다(`ScoreDatabaseAccessor.java:277-289`) — 의도적 이탈.
- 리플레이 GC 는 `scorelog` 가 가리키는 파일만 다룬다. 디렉터리에 있지만 아무 행도 가리키지 않는 고아 파일은 **세지도 지우지도 않는다.**

## 9. 배선 후 정리

`apps/rbms-player/src/scoredb_store.rs` 최상단의 `#![allow(dead_code)]` 와 그 위 문단의 마지막 문장
("The module is complete but not yet called: …")을 **삭제**한다. 1·2번 패치가 들어가면 전 항목이 호출된다.
`run_replay_gc`·`playtime_ms`·`score_paths` 까지 전부 쓰이므로 남는 dead code 는 없다.
