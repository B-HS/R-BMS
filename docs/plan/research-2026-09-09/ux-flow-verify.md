# ux-flow 감사 보고서 검증 (회의적 재검토)

대상: `scratchpad/research/ux-flow.md` findings 21건. 각 항목을 rbms/레퍼런스 구현 코드로 직접 반박 시도.
검증 시각 기준 브랜치: dev (c6f0885).

## 요약

| 판정 | 건수 | id |
|---|---|---|
| confirmed | 13 | 01, 02, 03, 04, 05, 06, 07, 08, 13, 14, 18, 19, 21 |
| partially | 7 | 09, 10, 11, 12, 15, 16, 20 |
| refuted | 1 | 17 |
| uncertain | 0 | — |

가장 큰 오류 2건:
1. **ux-flow-17 은 사실이 아니다.** DEBUG 온스크린 오버레이가 이미 구현돼 있고 FPS/프레임ms/RAM/QUADS/STAGE + 플레이 중 오디오 클럭·앵커까지 표시한다 (`apps/rbms-player/src/app_play.rs:747-778`). 보고서의 "fps는 계산만 되고 표시 경로 미확인", "DEBUG 실효 범위가 미리듣기 로그뿐"은 코드와 정면 충돌한다.
2. **ux-flow-12 의 코드 경로가 틀렸다.** Select 의 Esc 는 `to_select_or_exit` 가 아니라 `select_back` 을 부르며(`main.rs:999`), `select_back` 은 `SelectView::Root` 에서 **라이브러리가 비어 있든 아니든 무조건 `event_loop.exit()`** 한다 (`app_select.rs:824-827`). 즉 문제는 보고서가 쓴 것보다 넓다(빈 라이브러리 한정이 아님).

---

## 항목별 검증

### ux-flow-01 — confirmed (심각도는 critical→high 로 하향 권고)

근거 재확인:
- `app_play.rs:17` chart not found, `:58` skin load failed, `:314` score submit 실패 → 전부 `eprintln!`.
- `app_select.rs:390,402,408,416,426,435,445,461,472` 미리듣기 9분기(보고서는 "6분기"라 했으나 실제 `config.debug` 게이트 eprintln 은 9곳), `:580` replay load failed(이건 debug 게이트 **없음**).
- `scores.rs:52,57,62`, `tables.rs:38,44,48`, `settings.rs` 저장 실패 전부 eprintln.
- `crates/rbms-render/src/` 전 파일(cpu/font/hud/lib/playfield/result/select/skin/theme) 에 toast/status/banner 없음 — 확인.

정정 2건:
- 보고서가 인용한 `app_select.rs:403,412,421,436` 은 실제 라인과 어긋난다(정확히는 402,408,416,426...). 주장 자체는 성립.
- **GUI 피드백이 "전무"하지는 않다**: 서버 연결 인디케이터(`app_play.rs:743-746`, 녹/적 사각형)와 빈 리스트 온보딩 힌트(`app_select.rs:241-249` → `crates/rbms-render/src/select.rs:275`)가 이미 있다. "실패 이벤트를 알리는 수단이 없다"로 좁히면 정확. severity critical 은 과대, high 가 타당.

### ux-flow-02 — confirmed

`app_select.rs:740-745` `add_table_source` → `load_and_match`(`tablesrc.rs:75-89`) → `load_table_source`(`tablesrc.rs:47-55`) → `rbms_table::DifficultyTable::fetch_or_cache` → `crates/rbms-table/src/lib.rs:140-146` `reqwest::blocking::Client::builder().timeout(Duration::from_secs(15))`. 호출부 `app_select.rs:748`(파일), `:769-772`(URL Enter 커밋) 모두 winit 키 핸들러 안. 대조군 `rescan_all_folders`(`app_select.rs:694-709`)가 `thread::spawn`+mpsc 로 같은 `fetch_and_match` 를 도는 것도 사실. 수치·경로 전부 일치.

### ux-flow-03 — confirmed (라인 인용만 정정)

`refresh_focused_detail` 은 `app_select.rs:266-278`(보고서 인용 265-278 은 사실상 일치). 호출은 `app_play.rs:408`(frame() 안, `Stage::Select` 인 동안 매 프레임 — 다만 `si == self.focused_detail_si` early-return 이라 실제 파싱은 행 이동당 1회). `compute_chart_detail` + `decode_bga_256` 둘 다 동기. 미리듣기만 디바운스(`app_select.rs:294-303`) + 백그라운드(`:447-` 코디네이터 스레드)를 쓰는 비대칭도 사실. 다만 "빠른 스크롤 시 매 행 히칭"은 **행 이동당 1회**로 정확히 표현해야 한다(매 프레임 아님).

### ux-flow-04 — confirmed

키음: `app_play.rs:75-82` `spawn_keysound_decode` + `ks_progress`/`ks_total`. BGA: `app_play.rs:109-118` `for (id,name) in model.bgamap.iter().enumerate() { ... decode_bga_256(&dir, name) }` 동기 루프. 진행바는 `app_play.rs:487-491` 에서 `ks_progress`/`ks_total` 만 참조. "진행률 100% 후 원인불명 정지"는 정확. 단 `self.config.bga && self.skin.bga.is_some()` 게이트(`app_play.rs:107`)가 있어 BGA OFF 스킨/설정에서는 발생하지 않는다 — 보고서에 이 조건이 빠져 있다.

### ux-flow-05 — confirmed

레퍼런스 구현 `play/TargetProperty.java:116-140` 고정레이트 11종(RATE_A- … MAX), `:142-151` 임의 RATE_<n>, 라이벌/랭크 계열도 같은 파일에 존재 — 인용 정확. rbms: `crates/rbms-render/src/hud.rs:22` `best_ex: Option<u32>` (주석 "Local best EX on this chart"), `:28` `draw_score_graph(..., best: Option<u32>)` 단일 타깃, `app_play.rs:232`(ResultView `show_graph`) — HUD 쪽 전달은 별도지만 단일 best 인 점은 맞다. 타깃 선택 UI 없음 확인.

### ux-flow-06 — confirmed

`crates/rbms-ir/src/lib.rs:49-52` 에 `chart_ranking`/`player_best`/`player_profile`/`rivals` 선언. 플레이어 전체 grep 결과 `server.` 호출은 `main.rs:183` `health()` 와 `app_play.rs:312` `submit_score()` 둘뿐 — 재현됨.

### ux-flow-07 — confirmed

`app_input.rs:89-105` `apply_control` 6분기 전부 save 없음(Lift 만 `rebuild_skin()`). 저장 지점은 `app_input.rs:136-138`(`save_settings`) 를 부르는 `main.rs`(Settings Esc/Enter), `app_input.rs:150`(폰트), `:191`(NETWORK), `app_play.rs:367`(auto-cal). 재실행 시 유실 성립.
정정: 레퍼런스 구현 대조로 든 `ControlInputProcessor.java:130-166` 은 레인커버/START-홀드 처리 구간이며 "PlayerConfig 에 반영"하는 코드는 이 범위에 없다(`setCoverValue` 는 렌더러 상태). 대조 근거로는 약하나 결론은 무관하게 성립.

### ux-flow-08 — confirmed

rbms `main.rs:1003` `KeyCode::Tab => self.stage = Stage::Settings` — 전체화면 전환 확인. 레퍼런스 구현 `select/MusicSelectInputProcessor.java:114-151` `input.startPressed()` 홀드 중 `setPanelState(1)` + OPTION1/GAUGE/OPTIONDP/OPTION2/HSFIX, `:99-107` 릴리스 시 `OPTION_CLOSE` — 인용 정확.

### ux-flow-09 — partially

rbms 쪽(ControlAction 6종, Play Esc = 결과 직행 또는 이탈: `main.rs:1107-1118`)은 정확. 그러나 **레퍼런스 구현 대조가 틀렸다**:
- `play/ControlInputProcessor.java:205-216` 의 `ControlKeys.NUM1~NUM4` 는 pause/retry 가 아니라 **autoplay/replay 전용 재생속도 변경**(`player.setPlaySpeed(25/50/200/300)`)이다. 바로 위 `if (autoplay.mode == AUTOPLAY || REPLAY)` 가드가 그 증거.
- 같은 파일 `:203-205` `ESCAPE → player.stopPlay()` 는 rbms 의 Esc 와 동일 동작(즉시 이탈). START+SELECT 장압 종료(`:194-200`)도 종료지 일시정지가 아니다.
- 레퍼런스 구현 에 인터랙티브 플레이 중 **일시정지는 없다**. `PracticeConfiguration.java` 는 별도 PRACTICE 모드의 사전 설정이고, "QuickRetry" 는 `BMSPlayer.java:184` 주석에만 등장한다(입력 경로 미확인).
→ "레퍼런스 구현 가 제공하는 일시정지·리트라이가 rbms 에 없다"는 **원본 대비 결손이 아니라 신규 기능 제안**이다. severity medium → low, 그리고 근거를 "원본 divergence"가 아닌 "UX 개선"으로 재분류해야 한다.

### ux-flow-10 — partially

`main.rs:59 const PREVIEW_DEBOUNCE_FRAMES: u64 = 20`, `app_select.rs:294-303` 프레임 카운트 비교 — 사실. 다만 **수치가 틀렸다**: 20프레임 / 144Hz = 약 **139ms**(보고서와 일치), 20/30fps = 약 **667ms**(일치). 그러나 실제 렌더 루프가 vsync 로 60Hz 고정인지 무제한인지는 확인하지 못했다(`gpu.rs` present mode 미조사) — "144Hz 에서 139ms"는 **가정에 의존**한다. 결론(프레임 기준 → 시간 기준 전환)은 타당하나 근거는 조건부. severity low 유지.

### ux-flow-11 — partially

`app_select.rs:439` `eng.play(PREVIEW_ID, PREVIEW_GAIN, 0.0, 1.0, now)` 페이드 인자 없음, `main.rs:60 PREVIEW_GAIN: f32 = 0.85` 상수 고정 — 사실. 설정에 볼륨 행 없음도 사실.
정정: 보고서 스스로 "레퍼런스 구현 `PreviewMusicProcessor.java` 미확인"이라 적어 원본 대비 결손인지 근거가 없다. 또한 이 항목은 `config.preview` 토글(`app_input.rs`, DISPLAY 탭)로 전체 on/off 는 되므로 "설정 항목이 없다"는 볼륨 한정으로 좁혀야 한다.

### ux-flow-12 — partially (기전 정정, 문제는 오히려 확대)

`app_select.rs:865-867` `to_select_or_exit` 의 `if self.songs.is_empty() { event_loop.exit() }` 는 사실. Loading 취소 경로가 이를 우회하며 그 이유가 `main.rs:1096-1100` 주석에 적혀 있는 것도 사실.
**그러나 보고서가 든 "Select back(main.rs:999)" 은 이 함수를 부르지 않는다.** `main.rs:999` 는 `self.select_back(event_loop)` 이고, `select_back`(`app_select.rs:824-838`)은 `SelectView::Root` 일 때 **songs 유무와 무관하게 무조건 `event_loop.exit()`** 한다. 즉 곡이 수천 곡 있어도 Root 에서 Esc 한 번이면 확인 없이 종료된다 — 보고서가 놓친 더 큰 문제다.
또한 빈 상태 온보딩 UI 는 이미 존재한다(`app_select.rs:241-249` WELCOME/NO CHARTS 힌트) → 권고의 "안내 화면으로 보내라"는 절반은 이미 구현돼 있고, 필요한 건 exit 경로 차단뿐이다.

### ux-flow-13 — confirmed

`app_select.rs:676` `.pick_folder()`, `:748` `.pick_file()`(표 json), `app_input.rs:147` `.pick_file()`(폰트) — 세 곳 모두 키 핸들러 안 동기 호출. 라인 정확.

### ux-flow-14 — confirmed

`crates/rbms-render/src/result.rs:5-23` ResultView 전 필드가 스칼라(시계열 없음), `:97-104` 랭크바+dj_rank 만. `app_play.rs:218-232` 조립부도 동일. 권고에 인용된 `result.rs:29-58`(RANK_BANDS/dj_rank) 재사용 가능성도 실재(`:30-40` RANK_BANDS, `:47-55` dj_rank).

### ux-flow-15 — partially

`app_input.rs:180-203` `settings_text_input` 은 Enter/Escape/Backspace/typed 4분기 — 사실. `app_select.rs:766-786` 표 URL 입력도 동일 구조 — 사실. 클립보드 크레이트 없음도 사실.
정정: `Cargo.toml` 의존성 자체는 확인하지 못했고 grep 기반이다(보고서와 동일 한계). 또한 커서 이동이 없다는 점 때문에 "오타 수정은 끝에서부터만" 은 맞지만, NETWORK 편집은 `begin_net_edit`(`app_input.rs:168-176`)가 기존 값을 프리필하므로 "URL 을 손으로 전부 타이핑해야 한다"는 **재편집 시에는 거짓**이다(최초 입력 때만 참).

### ux-flow-16 — partially

`main.rs:1003`(Select Tab → Settings), `:1036-1039`(Settings Tab → 탭 순환) — 중복 사실. Loading 취소가 Root 리셋으로 선택 위치를 잃는 것(`main.rs:1099-1103`)도 사실.
정정: "뒤로가기 키가 화면마다 다르다"의 예로 든 `main.rs:999`(Select Esc)는 앞서 본 대로 `select_back` 이고, `app_select.rs:717`(Folders Esc = 전체 재스캔)은 실제로는 `folders_input` 의 `KeyCode::Escape => self.rescan_all_folders()`(`app_select.rs:714`). 라인이 어긋나지만 주장은 성립. 심각도 low 타당.

### ux-flow-17 — refuted

**핵심 주장이 코드와 어긋난다.** `app_play.rs:747-778` 에 `if self.config.debug` 게이트의 온스크린 오버레이가 이미 있고, 표시 내용은:
- `:751` `FPS {:.0} ({:.1} MS)` — 프레임타임 표시 **존재**(보고서: "표시 경로가 확인되지 않음" → 거짓)
- `:752` `RAM {:.1} MB`, `:753` `QUADS {} STAGE {:?}`
- Play 중(`:754-766`): TIME/NOTES/COMBO/EX/GAUGE/FAST/SLOW/EPOOR/HISPEED/OFFSET, 그리고 `:765-766` `AUDIO {} US ANCHOR {}` — **오디오 클럭 진단이 이미 있다**(보고서: "오디오 클럭 드리프트 없음" → 사실상 존재)
- Select 중(`:768-770`): SEL/SCORES/SONGS/CURSOR

따라서 "DEBUG 의 실효 범위가 미리듣기 로그뿐", "온스크린 진단 수단 없음"은 모두 거짓. 남는 실제 갭은 **판정 ms-off 스캐터**와 **활성 보이스 수** 2가지뿐이며 severity 는 low→info 로 하향해야 한다.

### ux-flow-18 — confirmed

`app_select.rs:25-31` `start_search` 가 `select_view = SelectView::AllSongs` 강제 — 사실. `:41-45` `apply_search` → `rebuild_select_items`, `main.rs:983-991` 매 키 입력마다 `apply_search()` — 사실. `app_input.rs:51-77` `arrange_songs` 가 매 호출 filter+sort 이고 `SortMode::Level` 이 비교자 안에서 `parse::<i64>()` 반복(`app_input.rs:68`) — 사실. 결과 개수 미표시도 사실(`SelectScene.search` 는 쿼리 문자열만: `app_select.rs:258`).

### ux-flow-19 — confirmed (severity info 가 타당)

`app_play.rs:334` `rp.save`, `:359` `self.scores.save`, `:367` auto-cal `save_settings` — 결과 진입 프레임 동기 3연속 저장 사실. `scores.rs:49-63` 전체 `ron::ser::to_string_pretty` 후 `fs::write` — 사실. 보고서 스스로 "현재 규모에서는 실질 무해"라 적었으므로 low 보다 info 가 맞다.

### ux-flow-20 — partially (미확인 항목을 확정 가능)

`main.rs:935-938` `Resized => gpu.resize(...)`, `gpu.rs:329-336` 은 `config.width/height` 갱신 + `surface.configure` 만 — 사실. `gpu.rs:338-340` `Renderer::size()` 가 상수 `(CW, CH)` 를 반환 → UI 는 논리 1280x720 고정.
**보고서가 "확인 필요"로 남긴 부분에 답이 있다**: `main.rs:942-943` 주석 "the surface stretches that space across the whole window" — 즉 **레터박스가 아니라 늘어난다(비-16:9 에서 왜곡)**. 권고의 "레터박스로 고정" 은 유효한 개선안이며 미확인이 아니다.

### ux-flow-21 — confirmed

`docs/PROCESS.md:132` "(2) **`#PREVIEW` 프리뷰 재생 (TODO — 실재생 미동작)** … 포커스해도 소리 안 남(추후 디버깅, 코드 유지)" vs 같은 문서 `:40` "(2026-06-07) **곡선택 하이브리드 미리듣기**" 완료 체크, `:176` "**곡선택 하이브리드 미리듣기 완료**(2026-06-07)". 코드는 `app_select.rs:380-445`(#PREVIEW 경로) + `:447-` (autoplay 폴백)로 구현 완료. 문서 내부 모순 확정.

---

## 보고서가 놓친 항목 (missed)

| id | 내용 | 근거 |
|---|---|---|
| M1 | **Root 에서 Esc 가 확인 없이 앱을 종료한다 — 라이브러리가 가득 차 있어도** | `app_select.rs:824-827` `SelectView::Root => event_loop.exit()`, 호출 `main.rs:999`. ux-flow-12 가 빈 라이브러리 한정으로 오인한 더 큰 문제 |
| M2 | **Play 중 Esc 가 즉시·무확인으로 런을 폐기한다** | `main.rs:1108-1117`: 미완주 상태면 `to_select_or_exit` 로 곧장 이탈, 진행 중 기록 전부 소실. 확인 다이얼로그 없음. 레퍼런스 구현 는 START+SELECT 장압(`ControlInputProcessor.java:194-200`, `exitPressDuration`)으로 오조작을 막는다 |
| M3 | **표 로드 실패가 조용히 빈 레벨로 흡수된다** | `tablesrc.rs:84-87`: 실패 시 `fallback_source_name` + `Vec::new()` 반환 → UI 에는 이름만 있고 항목 0개인 표가 정상처럼 보인다. 실패/미로드 구분이 UI 에 없음 |
| M4 | **`replay load failed` 는 debug 게이트조차 없는 무성 실패** | `app_select.rs:580` `eprintln!("replay load failed: {e}")` 후 진행 — 기록 모달에서 리플레이를 골라도 아무 일도 안 일어난 것처럼 보인다 |
| M5 | **검색 중 Esc 가 쿼리를 지우는 동시에 뷰(AllSongs)는 복원하지 않는다** | `app_select.rs:33-39` `exit_search` 가 `select_view` 를 되돌리지 않음 + `start_search`(`:25-31`)가 원래 뷰를 저장하지 않음 → 표/레벨 폴더에서 검색하면 그 폴더로 못 돌아온다 |
| M6 | **결과 화면에 리트라이/다음곡 동선이 없다** | `main.rs:1085-1089`: Result 는 Esc/Enter 모두 `to_select_or_exit` 하나뿐. 같은 곡 재도전이 select 왕복 + 전체 재로딩(키음 재디코드)을 강제 |
| M7 | **로딩 화면에 BGA/스캔 단계 표시가 없어 취소 가능 여부를 알 수 없다** | 진행바는 키음만(`app_play.rs:487-491`), 스캔은 `scan_count`(`app_select.rs:696`) 별도. Esc 취소는 `main.rs:1091-1105` 에만 존재하고 화면 안내 문구 미확인 |
| M8 | **`add_table_source` 는 중복 URL 을 걸러내지 않는다** | `app_select.rs:740-745`: 폴더 추가(`:677-681`)에는 `!self.folders.iter().any(...)` 중복 체크가 있으나 표 소스에는 없음 → 같은 표를 여러 번 추가하면 15초 fetch 가 반복되고 리스트가 중복된다 |

---

## 미조사 범위

- `gpu.rs` present mode / vsync 설정 (ux-flow-10 의 실제 프레임레이트 전제)
- `Cargo.toml` 의존성 목록 (클립보드 크레이트 부재는 grep 기반)
- 레퍼런스 구현 `select/PreviewMusicProcessor.java` (ux-flow-11 원본 대조)
- rbms 설정 화면 렌더(`crates/rbms-render/src/select.rs` 의 Settings 경로) 및 `SETTING_TABS` 전 항목
- 실제 실행 계측(프리징 체감 시간, 표 fetch 실측)
