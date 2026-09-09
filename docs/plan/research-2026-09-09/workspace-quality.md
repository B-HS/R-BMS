# 워크스페이스 구조 · Rust 코드 품질 감사

조사 일시: 2026-09-09 / 대상 커밋: `c6f0885` (dev, clean)
도구: `cargo clippy --workspace --all-targets` (rust 1.95.0), `cargo build --workspace`, 파일 통독

---

## 0. 요약

| 항목 | 실측 |
|---|---|
| 워크스페이스 멤버 | 9 crate + 2 app (`Cargo.toml:3-15`) |
| 총 Rust LOC | 20,879 |
| `cargo build --workspace` 경고 | **0** |
| `cargo clippy --workspace --all-targets` 경고 | **51** (lib/bin 기준: parser 2, chart 4, render 7, judge 5, play 4, table 1, player 28) — 전부 스타일 계열, correctness 계열 0 |
| `#[test]` 개수 | 889 |
| `#[allow(...)]` 개수 | **0** (검사기 무력화 없음) |
| `unsafe` | **0** (전 크레이트) |
| `todo!`/`unimplemented!` | **0** |
| `Mutex`/`RwLock` | **0** (핫패스 락 경합 없음 — 오디오는 `rtrb` SPSC + `AtomicU64` 클럭) |

전반적으로 **매우 깨끗하다.** 문제는 "관용성 결함"이 아니라 **구조**다: 게임 상태·설정 스키마·IR 매핑·스코어북이 전부 `apps/rbms-player` 안에 있고, 렌더 프리미티브가 `fill_rect` 하나뿐이라 스킨 고도화의 물리적 상한이 걸려 있다.

---

## 1. 크레이트 경계·의존 그래프

### 1.1 실측 의존 (각 `Cargo.toml`)

```
rbms-model    ← (없음)
rbms-parser   ← model, md-5, sha2, encoding_rs
rbms-chart    ← model, parser
rbms-judge    ← model
rbms-play     ← model, chart, judge          (dev: parser)
rbms-render   ← model, chart, serde, ron, cosmic-text   (dev: parser)
rbms-audio    ← cpal, rtrb, symphonia        (rbms 의존 0)
rbms-ir       ← reqwest, serde, serde_json   (rbms 의존 0)
rbms-table    ← serde, serde_json, reqwest   (rbms 의존 0)
apps/rbms-cli    ← model, parser, chart
apps/rbms-player ← 전부 + winit, wgpu, pollster, bytemuck, image, serde, ron, rfd, memory-stats, sha2
```

- **순환 없음, 방향 정상.** DAG 이고 model 이 최하층이다. `docs/PROCESS.md:103-119` 의 크레이트 맵과 실제 코드가 일치한다(stale 아님).
- **불필요 의존 후보 1건**: `rbms-render → rbms-chart`. render 는 `playfield.rs` 가 `TimeLine`/`NoteKind` 뷰만 쓰는데 chart(마디→µs 변환·shuffle·scroll 전체)를 통째로 끌어온다. 스크롤 오프셋 계산 결과만 넘겨받으면 끊을 수 있다. (효력 S, 리스크 낮음)
- **워크스페이스 dep 미등록으로 인한 버전 드리프트**: `serde` 가 ir/render/player 는 `1.0.228`, table 은 `1` (`crates/rbms-table/Cargo.toml:8`). `serde_json`(ir `1.0.150` / table `1.0.150`), `ron`(render `0.12.1` / player `0.12.1`), `reqwest`(ir·table 각각 선언)도 `[workspace.dependencies]`(`Cargo.toml:23-30`)에 없다. md-5/sha2/encoding_rs 만 등록되어 있다.

### 1.2 앱에 있으나 라이브러리로 내려야 할 것

| 현재 위치 | 내용 | 내려야 할 곳 | 근거 |
|---|---|---|---|
| `apps/rbms-player/src/ir_map.rs` (267줄) | `GaugeKind`↔`GaugeType`, `NoteOption`↔`RandomOption`, `ClearType`→`ClearLamp`, `lnmode`→`lntype` 매핑 + 테스트 | `rbms-ir` (또는 신설 `rbms-session`) | 순수 타입 매핑에 winit 의존 없음. IR 와이어 포맷 지식이 앱에 새 있음. `apps/rbms-player/src/main.rs:44` 에서 import |
| `apps/rbms-player/src/format.rs:105-135` (`clear_type_id`/`clear_type_from_id`) | 레퍼런스 구현 램프 ID 인코딩 | `rbms-judge` 또는 `rbms-ir` | 테스트가 `clear_type_ids_are_the_레퍼런스 구현_values`(`format.rs:230`)라고 명시 — 표현이 아니라 **프로토콜 값**이다 |
| `apps/rbms-player/src/scores.rs` (`ScoreBook`, 327줄 중 코드 91줄) | 로컬 기록 영속·베스트 조회 | 신설 `rbms-store` | 레퍼런스 구현 는 `ScoreDatabaseAccessor.java`/`PlayDataAccessor.java` 로 분리 |
| `apps/rbms-player/src/replay.rs` | 리플레이 포맷 · load/save | `rbms-play` 또는 `rbms-store` | 레퍼런스 구현 `ReplayData.java` 는 core 패키지 |
| `apps/rbms-player/src/settings.rs` + `main.rs:71-137`(`PlayerConfig`) | 플레이 옵션 스키마 | 신설 `rbms-config` | 아래 1.3 |
| `apps/rbms-player/src/main.rs:392-477` (`is_chart`/`scan_folders`/`scan_folder`/`compute_chart_detail`) | 라이브러리 폴더 스캔 | 신설 `rbms-library` | 레퍼런스 구현 `song/` 패키지 대응 |
| `apps/rbms-player/src/tablesrc.rs:9-33` (`compute_table_levels`) | 난이도표 ↔ 라이브러리 md5 매칭 | `rbms-table` | 이미 `rbms-table` 이 `by_level` 을 제공하므로 자연스러운 자리 |

반대(크레이트에 있으나 앱 전용)는 **발견되지 않았다.** `rbms-render::select`/`result` 가 앱 UI 전용처럼 보이지만 헤드리스 예제(`crates/rbms-render/examples/render_select.rs`)로 테스트되고 CPU 백엔드와 공유하므로 현 위치가 맞다.

### 1.3 설정 스키마 이중화 (가장 큰 단일 구조 결함)

- `PlayerConfig` (`main.rs:71-100`, 28 필드) 와 `PlaySettings` (`settings.rs:10-40`, 24 필드)가 **손으로 쓴 양방향 매핑**으로 연결된다: `apply_settings`(`main.rs:138-167`) 와 `current_settings`(`app_input.rs:107-135`).
- 필드를 추가하면 5곳(두 구조체·두 매핑·`SETTING_TABS` 인덱스)을 동시에 고쳐야 하고, 컴파일러가 누락을 잡아주지 않는다.
- 실제로 `PlayerConfig` 에만 있는 필드(`keys_override`, `skin_path`, `table_url`, `keyconfig_path`, `replay_path`)와 `PlaySettings` 에만 있는 필드가 섞여 있어 어느 쪽이 정본인지 코드로 판별되지 않는다.

### 1.4 설정 UI의 인덱스 결합

`main.rs:592-607`:
```rust
const SETTING_KEYCONFIG: usize = 11;
const SETTING_FONT: usize = 18;
const SETTING_SERVER_URL: usize = 22;
const SETTING_PLAYER_ID: usize = 23;
const SETTING_TABS: &[(&str, &[usize])] = &[("PLAY", &[0,1,2,3,16]), ("GAUGE", &[4,13]), ...];
```
설정 행이 **정수 인덱스**로만 식별된다. `setting_line`(`app_select.rs:888`)·`adjust_setting`(`app_select.rs:919`)이 이 숫자로 분기한다. 행 하나 추가 = 상수 4개 + 탭 배열 6개를 손으로 재계산. "판정 설정 노출" 같은 후속 작업이 여기서 바로 막힌다.

---

## 2. 파일 길이 — raw LOC vs 실제 코드 LOC

**중요 정정**: 지시서의 "600줄 초과 파일" 목록 중 대부분은 **인라인 테스트 모듈이 차지한 것**이다. `#[cfg(test)] mod tests` 시작 줄까지가 실제 코드다.

| 파일 | raw | tests 시작 | 실제 코드 |
|---|---:|---:|---:|
| `apps/rbms-player/src/main.rs` | 1456 | 1277 | **1277** |
| `apps/rbms-player/src/app_select.rs` | 967 | (없음) | **967** |
| `apps/rbms-player/src/app_play.rs` | 796 | (없음) | **796** |
| `apps/rbms-player/src/app_input.rs` | 402 | (없음) | 402 |
| `crates/rbms-chart/src/shuffle.rs` | 890 | 333 | 333 |
| `crates/rbms-ir/src/dto.rs` | 938 | 301 | 301 |
| `crates/rbms-play/src/lib.rs` | 902 | 271 | 271 |
| `crates/rbms-render/src/skin.rs` | 541 | 293 | 293 |
| `crates/rbms-audio/src/mixer.rs` | 785 | 188 | 188 |
| `crates/rbms-render/src/playfield.rs` | 649 | 196 | 196 |
| `crates/rbms-table/src/lib.rs` | 614 | 162 | 162 |
| `apps/rbms-player/src/keyconfig.rs` | 660 | 296 | 296 |
| `apps/rbms-player/src/format.rs` | 434 | 153 | 153 |
| `crates/rbms-judge/src/lib.rs` | 787 | 22 | **22** |

→ **진짜 분할 대상은 3개뿐**: `main.rs`(1277), `app_select.rs`(967), `app_play.rs`(796). 라이브러리 크레이트는 전부 350줄 이하이고 분할이 필요 없다. `rbms-parser`/`rbms-chart` 는 이미 `tests.rs` 별도 파일 패턴을 쓰므로(`crates/rbms-parser/src/tests.rs`, `crates/rbms-chart/src/tests.rs`), 나머지 크레이트도 같은 패턴으로 옮기면 raw LOC 이 일관되게 줄어든다.

### 2.1 `apps/rbms-player/src/main.rs` (1277줄 코드) 분할안

| 이동 대상 | 줄 | 새 모듈 |
|---|---|---|
| `PlayerConfig`, `Default`, `apply_settings`, `default_total`, `calibrated_offset` | 71-167, 209-221 | `config.rs` (→ 장기적으로 `rbms-config` 크레이트) |
| `build_server`, `compute_build_hash`, `client_platform` | 169-208 | `server.rs` |
| `SKIN_NORMAL`/`SKIN_WIDE`/`bundled_skin`/`THEME_TEMPLATE`/`load_theme` | 222-280 | `assets.rs` |
| `resolve_file`, `resolve_keysound`, `keysound_jobs`, `spawn_keysound_decode`, `decode_bga_256` | 281-357 | `loadres.rs` |
| `SongEntry`, `ChartDetail`, `is_chart`, `scan_folders`, `scan_folder`, `compute_chart_detail` | 358-478 | `library.rs` (→ `rbms-library` 크레이트) |
| `Stage`, `Loading`, `ScanOutcome`, `KcRow`, `kc_rows`, `SelectView`, `SelectItem`, `SortMode`, `Hot`, `GAUGE_CYCLE`, `SETTING_*` | 479-607 | `state.rs` |
| `struct App` + `App::new` | 609-909 | `app.rs` |
| `ApplicationHandler for App` 의 `window_event` 키보드 분기 (약 220줄) | 949-1170 | 각 stage 모듈의 `fn handle_key` 로 이관 (아래 2.4) |
| `config_dir`, `config_dir_from`, `main` | 1178-1276 | `main.rs` 잔류 |

### 2.2 `apps/rbms-player/src/app_select.rs` (967줄) 분할안

이미 `impl App` 확장 블록이지만 4개 관심사가 한 파일에 있다.
- `build_select_view`(98-265, **168줄 단일 함수**) + `select_key`/`refresh_select_cache`/`record_row_view` → `select_view.rs`
- `update_preview`/`start_preview`/`start_autoplay_preview`/`reset_preview_playback`/`stop_preview`(286-541) → `preview.rs`
- `open_tables`/`add_table_source`/`add_table_file`/`remove_table_source`/`tables_input`/`open_folders`/`folders_*`/`rescan_all_folders`(658-834) → `library_screens.rs`
- `setting_line`/`adjust_setting`(888-967) → `settings_screen.rs`

### 2.3 `apps/rbms-player/src/app_play.rs` (796줄) 분할안

- `frame()`(373-783, **410줄**)이 FPS 갱신 → 로딩 폴링 → 프리뷰 → 클럭/analysis → 리플레이 피드 → player.update → stage별 렌더의 7단계를 한 함수에 담고 있다.
  - `fn tick_stats(&mut self, dt)` / `fn tick_loading(&mut self)` / `fn tick_clock(&mut self) -> (i64, bool)` / `fn tick_play(&mut self, song, anchor, manual)` 로 분리하고,
  - stage별 렌더는 `fn draw_loading(&mut self)` / `draw_select` / `draw_settings` / `draw_play` / `draw_result` / `draw_keyconfig` 로 쪼갠다(각 stage 파일로).
- `load()`(9-149)는 파싱·셔플·스킨·오디오·BGA·Player 생성 6책임 → `fn build_model(&mut self) -> Model` / `fn spawn_audio(&mut self, &Model, &Path)` / `fn load_bga(&mut self, &Model, &Path)` 로 분리.
- `enter_result()`(198-372, 174줄) → 결과 집계와 IR 제출을 분리.

### 2.4 stage 상태기계 분리 (구조 개편 핵심)

현재 `struct App`(`main.rs:609-733`)은 **약 100 필드**의 god object 다. 셀렉트 UI 상태(`sel`, `search`, `sort`, `hot`, `cached_select`, `preview_*` 12필드), 플레이 세션(`player`, `audio`, `bga_*`, `recording`, `replay*`, `analysis*` 6필드, `cal_*`), 설정 편집(`set_tab`, `set_sel`, `kc_*`), 로딩(`scan_rx`, `ks_*`)이 한 구조체에 평평하게 있다. 어느 필드가 어느 stage 에서 유효한지 타입으로 표현되지 않는다.

권고: `Stage` 를 데이터 보유 enum 으로 바꾼다.
```
enum Stage { Loading(LoadingState), Select(SelectState), Settings(SettingsState),
             KeyConfig(KeyConfigState), Tables(..), Folders(..), Play(PlaySession), Result(ResultState) }
```
`PlaySession`(player+audio+bga+replay+analysis+calibration)은 `rbms-play` 로 올린다 — 레퍼런스 구현 의 `PlayerResource.java`/`BMSPlayer.java` 대응. 각 state 에 `handle_key`/`draw` 를 붙이면 `window_event` 의 220줄 중첩 match 와 `frame()` 의 410줄이 자연히 해체된다.

---

## 3. Rust 관용성

### 3.1 clippy 결과 (전량, `--all-targets`)

lint 종류별 건수(중복 포함 전체 출력 기준):

| lint | 건수 |
|---|---:|
| `collapsible_if` | 24 |
| `unnecessary_get_then_check` | 13 |
| `needless_range_loop` | 13 |
| `field_reassign_with_default` | 9 |
| `unnecessary_sort_by` | 4 |
| `type_complexity` | 4 |
| `too_many_arguments` | 4 |
| `manual_is_multiple_of` | 3 |
| `collapsible_match` | 3 |
| `wrong_self_convention` / `unnecessary_min_or_max` / `should_implement_trait` / `manual_contains` / `manual_clamp` / `items_after_test_module` / `derivable_impls` | 각 1 |

- **correctness / suspicious / perf 계열 경고는 0건.** 전부 style·complexity 다.
- `cargo clippy --fix` 로 자동 수정 가능한 것이 다수(각 crate 요약이 "apply N suggestions" 로 안내).
- `too_many_arguments`/`type_complexity` 위치: `crates/rbms-render/src/playfield.rs:18`, `:50`, `crates/rbms-judge/src/matcher.rs:95`, `:139`, `:140`, `apps/rbms-player/src/main.rs:339`, `crates/rbms-chart/src/lib.rs:393`. → 렌더/판정 API가 인자 폭발 중이라는 신호(3.5 참조).
- `items_after_test_module` 1건 — 테스트 모듈 뒤에 코드가 있다(파일 구성 위생).

### 3.2 에러 처리

- `anyhow`/`thiserror` **미사용**(전 크레이트 grep 0건). 대신 `rbms-ir` 만 손으로 쓴 `IrError` enum + `Display` + `std::error::Error` 구현(`crates/rbms-ir/src/lib.rs:16-35`) — 이건 잘 되어 있다.
- 나머지는 `Result<T, String>` 8곳: `crates/rbms-table/src/lib.rs:41,50,65,140,148`, `crates/rbms-render/src/skin.rs:115`, `apps/rbms-player/src/tablesrc.rs:48`, `apps/rbms-player/src/replay.rs:32`. 호출부가 원인을 분기할 수 없고(`e.to_string()` 문자열 비교밖에 못 함), `?` 로 합성이 안 된다.
- 실패는 대부분 `eprintln!` + 계속 진행(`app_play.rs:14-17` 차트 없음, `:57` 스킨 로드 실패, `main.rs:1259-1264` 폰트) — GUI 앱인데 **사용자에게 보이는 오류 경로가 없다**(터미널 없이 실행되는 .dmg 케이스는 `main.rs:1244` 주석에 스스로 적어 놓았다).

### 3.3 panic 위험 지점 (런타임 입력 경로)

`unwrap`/`expect`/`panic!`/`unreachable!` 총계(테스트 포함): audio 14, chart 30, ir 57, judge 43, parser 31, play 29, render 17, table 21, player 53. **대부분 테스트 안**이다. 테스트 모듈 밖 실질 위험은 아래 6곳:

| 위치 | 코드 | 위험 |
|---|---|---|
| `crates/rbms-chart/src/lib.rs:156` | `events.sort_by(\|a,b\| a.section.partial_cmp(&b.section).unwrap())` | **f64 NaN 이면 패닉.** section 은 파싱된 BMS 소절 번호 유래 → 악성/깨진 차트가 트리거 가능. `f64::total_cmp` 로 교체하면 clippy `unnecessary_sort_by`(`lib.rs:393`)도 같이 해소 |
| `crates/rbms-judge/src/matcher.rs:264,293` | `note.end_us.unwrap()` | `LongStart` 인데 `end_us` 가 없는 차트(dangling LN)에서 패닉. `rbms-play` 는 dangling 을 명시적으로 처리하는 테스트가 있는데(`play/lib.rs:391 dangling_longstart_does_not_stick_beam`) judge 쪽은 unwrap |
| `crates/rbms-chart/src/lib.rs:248` | `timelines[si].notes[*lane].as_mut().unwrap()` | 같은 계열 |
| `crates/rbms-audio/src/mixer.rs:133` | `v.sample.as_ref().unwrap()` | **실시간 오디오 콜백 경로.** 여기서 패닉하면 오디오 스레드가 죽고 곡이 무음이 된다. `active` 플래그와 `sample` 의 일관성이 불변식으로만 보장됨 |
| `apps/rbms-player/src/main.rs:917,1270,1273`, `gpu.rs:96,102,103` | `create_window().unwrap()`, `EventLoop::new().unwrap()`, `expect("no graphics adapter")` | 기동 실패 시 스택트레이스만 남고 사용자 안내 없음. GUI 앱이라 터미널이 없을 수 있음 |
| `crates/rbms-parser/src/control.rs:93,101` | `let Frame::If{..} = self.stack[pos] else { unreachable!() }` | 직전 검사로 보호되지만, 불변식을 타입으로 못 세운 자리 |

### 3.4 전역 상태 / 동시성

- `unsafe` 0, `static mut` 0, `Mutex`/`RwLock` 0. 오디오는 `rtrb` SPSC 링 + `Arc<AtomicU64>` 클럭(`crates/rbms-audio/src/engine.rs:18,113`) — RT 스레드에 락 없음. **핫패스 설계는 좋다.**
- 다만 렌더에 **암묵적 thread_local 전역**이 2개 있다: `crates/rbms-render/src/theme.rs:164-166`(`THEME: RefCell<Theme>`)와 `crates/rbms-render/src/font.rs:159-161`(`ENGINE: RefCell<TextEngine>`). `set_theme`/`set_ui_family` 는 `main()`(`main.rs:1256,1267`)에서 한 번 호출된다.
  - 결과: 다른 스레드(헤드리스 예제·향후 워커 렌더·테스트)에서는 조용히 기본 테마로 그려진다. 테마를 렌더 함수 인자(`&Theme`)로 넘기면 사라지는 결합이다. 스킨 커스터마이징이 "스킨별 테마"까지 가면 이 전역이 곧 장애물이 된다.

### 3.5 trait 추상화

- **`Renderer`(`crates/rbms-render/src/lib.rs:61-65`)가 3 메서드뿐이다: `size`/`clear`/`fill_rect`.** 즉 전 UI가 **축정렬 단색 사각형**으로만 그려진다. `Gpu` 는 텍스처를 그릴 수 있지만(`apps/rbms-player/src/gpu.rs` 의 `set_bga`/`clear_bga`) 이건 **trait 밖 고유 메서드**라 `playfield`/`select`/`result` 렌더러가 못 쓴다 — BGA/커버 한 장을 위한 단일 슬롯이다.
  - 레퍼런스 구현 스킨은 `SkinImage`/`SkinNumber`/`SkinText`/`SkinSlider`/`SkinGraph`/`SkinNoteDistributionGraph` 등 20+ 오브젝트 타입에 타이머·프로퍼티 바인딩(`<reference>/skin/SkinObject.java`, `SkinProperty.java`, `SkinPropertyMapper.java`, `SkinLoader.java`)을 쓴다. LR2/json/lua 3종 로더도 있다(`skin/lr2`, `skin/json`, `skin/lua`).
  - **결론: 현 `Renderer` 로는 "스킨 완전 커스터마이징"이 원리적으로 불가능하다.** 최소 `draw_textured_quad(dst, tex, src_uv, tint, alpha)` + `push_clip/pop_clip` + z-order 가 필요하다.
- `SkinConfig`(`crates/rbms-render/src/skin.rs:12-64`)는 **약 40개 스칼라 필드의 평면 구조체**다. 위치 몇 개와 색만 바꿀 수 있고 "오브젝트를 추가/삭제/조건부 표시"할 수 없다. 레퍼런스 구현 의 `Vec<SkinObject>` 씬그래프 모델과 표현력 차이가 크다.
- `ScoreServer`(`crates/rbms-ir/src/lib.rs:44-91`)는 잘 설계돼 있다: 필수 8 + 기본 `Unsupported` 확장 8. `Send + Sync` 이고 `Arc<dyn ScoreServer>` 로 런타임 교체(`main.rs:169-193 build_server`) 가능.
  - 다만 **전부 blocking**이라 호출부가 스레드를 직접 띄워야 한다. 판정 직후 제출이 프레임을 막을 수 있는지는 `enter_result` 통독 범위 밖 — **미확인**.

### 3.6 판정 설정 노출 (후속 고도화 관점)

- `JudgeWindows`(`crates/rbms-judge/src/windows.rs:9-72`)는 **`const` 연관 상수 4개**로 하드코딩돼 있고(`SEVENKEY_NOTE`, `SEVENKEY_LN_END`, `POPN_NOTE`, `POPN_LN_END`), 노출된 조절 손잡이는 `scaled(judgerank_percent)`(`:77`) 하나뿐이다. 앱은 이걸 `judge_rate`(50~200 클램프, `main.rs:150`) 로만 노출한다.
- 레퍼런스 구현 는 `play/JudgeProperty.java` 에 모드별 판정 테이블 + `PlayConfig.java` 의 `judgewindowrate` 등을 둔다. rbms 가 "판정 설정 노출"을 하려면 `JudgeWindows` 를 **`Deserialize` 가능한 데이터**로 만들고 모드별 테이블을 RON 으로 빼야 한다. 현재는 `Deserialize` 도 없다(`windows.rs` 에 serde 없음, `rbms-judge/Cargo.toml` 에 serde 의존 없음).
- 판정 알고리즘 선택(레퍼런스 구현 `JudgeAlgorithm.java`: Combo/Duration/LowestNote/SlowestNote)에 해당하는 추상화도 없다 — `matcher.rs` 단일 구현.

### 3.7 가시성 · 캡슐화 · 기타

- 앱 모듈은 `pub(crate)` 위생이 좋다(`app_select.rs`/`app_play.rs`/`app_input.rs` 의 `pub fn` 0건, 전부 `pub(crate)`).
- `rbms-play::Player::judge` 가 **public 필드**(`crates/rbms-play/src/lib.rs:87`)라 외부가 `JudgeEngine` 내부를 직접 만진다(`app_play.rs:443` `player.judge.total_notes()`). 접근자로 감싸는 편이 낫다.
- `crates/rbms-ir/src/lib.rs:7` `pub use dto::*` — DTO 938줄 전체를 글롭 재수출. 이름 충돌·API 표면 팽창 위험.
- 매직넘버: `main.rs:52-53` `CW=1280`/`CH=720` 고정(스킨이 좌표를 1280×720 픽셀로 authoring 하도록 강제 — `skin.rs:5-8` 주석에 명시), `main.rs:592-595` 설정 인덱스 상수, `app_play.rs:445` `song > last_time_us + 2_000_000`(결과 화면 전환 지연) 등.
- 주석 품질은 높다. dead code·`#[allow]` 0건.
- 테스트 배치: 인라인 `mod tests` 가 대다수, 별도 `tests.rs` 는 parser/chart 2곳, 통합 테스트는 `apps/rbms-player/tests/autoplay_preview.rs` 1개. 라이브러리 크레이트에 **공개 API 대상 통합 테스트(`tests/`) 가 없다** — 리팩터링 시 내부 접근에 의존한 테스트가 같이 깨진다.

---

## 4. 개선 우선순위

| # | 항목 | 효과 | 리스크 | 효력 |
|---|---|---|---|---|
| 1 | `Renderer` 프리미티브 확장(텍스처 쿼드·클립·z) + `SkinConfig` → 오브젝트 리스트 모델 | 스킨 커스터마이징의 **전제조건**. 없으면 이후 스킨 작업 전부 막힘 | 중(렌더 전면) | L~XL |
| 2 | `Stage` 를 데이터 보유 enum 으로 + `PlaySession` 을 `rbms-play` 로 이관 | `App` 100필드 해체, `frame()`/`window_event` 자연 분해, 이후 모든 기능 추가 비용 하락 | 중 | L |
| 3 | `JudgeWindows` 를 데이터화(serde + 모드별 RON 테이블) + `JudgeAlgorithm` trait | "판정 설정 노출" 전제조건. 레퍼런스 구현 패리티 | 낮 | M |
| 4 | `PlayerConfig`/`PlaySettings` 통합 → `rbms-config` 크레이트 | 설정 추가 비용 5곳→1곳, 드리프트 제거 | 낮 | M |
| 5 | 설정 UI 를 인덱스 상수 → descriptor 테이블(enum + 메타)로 | 3·4 를 UI 에 노출하는 실질 통로 | 낮 | M |
| 6 | `main.rs`/`app_select.rs`/`app_play.rs` 모듈 분할(2.1~2.3 안) | 가독성·리뷰 가능성. 2 의 준비 단계로 먼저 해도 됨 | 낮 | M |
| 7 | 런타임 패닉 6곳 제거(3.3 표) | 깨진 차트로 인한 크래시 방지 | 낮 | S |
| 8 | `Result<_, String>` 8곳 → `thiserror` enum + GUI 오류 표시 경로 | 진단 가능성, .dmg 실행 시 침묵 실패 해소 | 낮 | M |
| 9 | clippy 51건 정리(`--fix` 다수 자동) + CI 에 `-D warnings` 게이트 | 재발 방지 | 낮 | S |
| 10 | 워크스페이스 dep 통합(serde/serde_json/ron/reqwest) | 버전 드리프트 차단 | 없음 | S |
| 11 | 인라인 대형 테스트 모듈 → `src/*/tests.rs` 분리 + `tests/` 통합테스트 추가 | raw LOC 정상화, 리팩터 내성 | 낮 | S~M |
| 12 | `ScoreBook`/`Replay`/`ir_map`/`clear_type_id`/폴더스캔 크레이트 이관 | 앱↔라이브러리 경계 정리, 재사용(웹/서버) 가능 | 낮 | M |
| 13 | `theme`/`font` thread_local 전역 → 인자 주입 | 1 과 함께 하면 스킨별 테마가 가능해짐 | 낮 | S~M |
| 14 | `rbms-render → rbms-chart` 의존 제거 | 빌드 그래프 단순화 | 낮 | S |
| 15 | `Player::judge` 필드 비공개화 + 접근자 | 캡슐화 | 낮 | S |

---

## 5. 미조사 범위

- `crates/rbms-ir/src/http.rs`, `null.rs`, `dto.rs` 본문 — trait 정의와 매니페스트만 확인. **IR 제출이 렌더 스레드를 블록하는지 미확인**(`app_play.rs:198-372 enter_result` 본문 미통독).
- `crates/rbms-audio/src/mixer.rs`/`engine.rs` 본문(락·아토믹 선언만 grep). 메모리 누수·보이스 풀 정책 미검증.
- `crates/rbms-chart/src/shuffle.rs`, `scroll.rs`, `crates/rbms-judge/src/gauge.rs`, `matcher.rs` 본문 — panic 지점만 grep.
- `crates/rbms-render/src/playfield.rs`, `select.rs`, `result.rs`, `hud.rs` 본문 — `Renderer` trait 표면과 clippy 위치만 확인.
- `apps/rbms-player/src/main.rs:949-1176`(window_event 본문) 은 구조만 grep, 줄단위 미통독.
- 레퍼런스 구현 측은 **패키지/파일 배치만** 확인했고 `SkinLoader.java`·`JudgeProperty.java`·`PlayerResource.java` 본문은 읽지 않았다 — 4장 1·3번 항목의 상세 스키마 설계는 그 통독 후에 확정해야 한다.
- 성능·메모리 누수 관점은 이 감사 범위 밖(별도 관점).
