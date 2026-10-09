# R3 — R-BMS 앱(apps/rbms-player) 스킨 배선·화면 구성·입력 현황과 격차

> 최종 갱신 2026-10-09 · 대응 단계: L1 조사(구현 전) · 기준 커밋 `9ce92bb` · 색인과 갱신 규칙은 [README.md](README.md)

- 조사일: 2026-10-09, 대상 브랜치 `dev`(HEAD `9ce92bb`), 읽기 전용 조사
- 표기: `R:` = `/Users/hyunseokbyun/development/R-BMS/` 기준 경로, `B:` = `/Users/hyunseokbyun/development/beatoraja/src/bms/player/beatoraja/` 기준 경로, `M:` = `/Users/hyunseokbyun/Downloads/ModernChic/` 기준 경로
- "미확인" 은 이번 조사에서 원문을 열어 확인하지 못한 항목이다. 읽지 못한 파일 목록은 문서 끝(§14)에 있다.
- 저장소 규칙 주의: `R:docs/HANDOFF.md:50` 은 레퍼런스 엔진 이름과 외부 스킨 묶음 이름을 저장소(코드·문서·커밋·파일명)에 쓰지 말라고 적고 있다. 이 보고서는 저장소 밖 스크래치에만 있으므로 이름을 그대로 쓴다. 구현 사양·커밋·폴더명에서 이 규칙을 유지할지는 사용자 결정 사항이다(§13 미해결 질문 1).

---

## 0. 한눈에 보는 결론

| 영역 | 현재 R-BMS | ModernChic(레퍼런스 계약)이 요구하는 것 | 격차 크기 |
| --- | --- | --- | --- |
| 화면 수 | Stage 10개, 그중 스킨 문서가 붙는 곳 5종(play/select/result/loading=decide/keyconfig) | 7개 상태 전부가 스킨 단독 소유(select, decide, play, result, course result, key config, skin config) | L |
| 합성 방식 | 내장 화면 + 문서 혼합(`replace`/`layered`/`overlay`), 문서 없으면 내장 화면 | 스킨이 화면 전체를 그림. 내장 레이아웃 없음 | L |
| 장면 시계 | 앱 전체 단일 시계, 화면 전환 시 타이머 미초기화 | 상태 전환마다 모든 타이머 OFF + 장면 시계 0 재시작 | M (정확도에 치명적) |
| 장면 수명 | 키 입력 즉시 전환 | `input`/`scene`/`fadeout`/`loadstart`/`loadend`/`playstart`/`close`/`finishMargin` 로 스킨이 전환 시점을 정함 | L |
| 문서 탐색 | `.json`/`.json5` 만, 화면 타입 5·6·7·8·play | `.luaskin` 10개(타입 0,1,2,3,5,6,7,8,9,15) | M |
| 마우스 | 좌클릭 press 한 종류, 핫스팟 8종(select 전용) | 객체 click 이벤트, 슬라이더 드래그, 휠, 텍스트 입력 포커스 | L |
| 논리 해상도 | 1280x720 고정(`CW`/`CH`), 창 크기만 늘림 | 1920x1080 저작 | M |
| 시스템 사운드 | 22 stem, 1회 재생만, 루프·BGM 없음 | select/decide BGM 루프, 스킨 자체 효과음 | M |
| 동영상 | 미지원(이미지 BGA만) | mp4 8개(193MB): select/decide 배경, play 기본 BGA | L |
| 기본 스킨 배포 | 1.8MB 를 `include_bytes!` 로 바이너리에 내장 후 최초 실행 시 설치 | 원본 478MB (png 212MB, mp4 193MB, ttf 55MB) | 배포 방식 재설계 필요 |

---

## 1. 화면(Stage) 목록과 전이

### 1.1 R-BMS Stage 정의

- `Stage` 열거 10종: `Select, Settings, KeyConfig, Tables, Folders, Loading, Play, Result, CourseResult, Practice` (`R:apps/rbms-player/src/stage/mod.rs:65-76`), 대응 `StageId` (`stage/mod.rs:82-93`).
- 전이 타입 `Transition::{Stay, Open(Stage), To(Stage), Back, Quit}` (`stage/mod.rs:99-110`). `Open` 은 현재 화면을 `App.suspended` 스택에 넣고, `Back` 은 꺼낸다. 스택이 비면 새 `SelectState` 를 만든다(`R:apps/rbms-player/src/lib.rs:867-884`, 특히 872).
- 전이 실행은 한 곳: `App::switch` 가 `on_exit` → 교체 → `on_enter` 순서(`lib.rs:878-883`). 옵션 오버레이는 select 가 아닌 화면으로 갈 때 닫힌다(`lib.rs:875-877`).
- 화면 계약 `StageHandler`: `update / draw / handle_key / handle_mouse / handle_pad / on_enter / on_exit / debug_lines / holds_keys` (`stage/mod.rs:136-167`).

### 1.2 R-BMS 전이 그래프(코드 근거)

| 출발 | 조건 | 전이 | 근거 |
| --- | --- | --- | --- |
| (시작) | 라이브러리 폴더 있음 | `Loading(scan)` | `lib.rs:714-716` |
| (시작) | 폴더 없음 / 차트·리플레이 직접 실행 | `Select` (직접 실행은 `resumed` 에서 `load()` 후 `To`) | `lib.rs:708-713`, `lib.rs:1007-1018` |
| Loading(scan/table) | 완료 | `Back` (스택 비어 있으면 새 Select) | `stage/loading.rs:269-279, 300-304, 331` |
| Select | Enter/→/행 재클릭 (곡) | `Open(Loading::song)` + `SystemSound::Select` | `stage/select/mod.rs:142-149, 731, 747-750` |
| Select | Enter (폴더) | 같은 화면에서 목록 교체 + `FolderOpen` | `stage/select/mod.rs:150-157` |
| Select | Enter (코스 탭) | `start_course` → `To(Loading::song)` | `stage/select/mod.rs:164-175`, `lib.rs:233-255` |
| Select | Tab / O / T | `Open(Settings)` / `Open(Folders)` / `Open(Tables)` | `stage/select/mod.rs:714-716` |
| Select | F4 | `practice_requested=true` + `Open(Loading::song)` | `stage/select/mod.rs:189-196, 708` |
| Select | Esc(루트에서 1초 내 2회) | `Quit` | `stage/select/mod.rs:540-550`, `lib.rs:168` |
| Settings | KEY CONFIG 행 | `Open(KeyConfig)` | `stage/settings.rs:160-164, 544` |
| Settings / KeyConfig / Folders / Tables | Esc | `Back` | `stage/settings.rs:527-530`, `stage/keyconfig.rs:153-156` |
| Loading(song) | 한 프레임 그린 뒤 `load()` | `To(Loading::assets)` 또는 바로 `To(Play)` | `stage/loading.rs:340-349, 444-447`, `app_play.rs:441-451` |
| Loading(assets) | 키음·BGA 디코드 완료 | `SystemSound::Decide` + `start_play()` + `To(Play)`, 연습 요청이면 `To(Practice)` | `stage/loading.rs:453-465` |
| Loading | Esc | 취소 후 `Back` | `stage/loading.rs:355-368, 468-470` |
| Play | 곡 종료 또는 게이지 0(분석 재생 제외) | `enter_result` → `To(Result)` / 코스면 다음 스테이지 `To(Loading::song)` 또는 `To(CourseResult)` / 연습이면 `Back` | `stage/play/mod.rs:431-436, 732-734`, `app_result.rs:301-309, 383-437` |
| Play | Esc(즉시/홀드/더블) | `leave_run` → `Back`(연습), `end_course`(코스), `leave_play` | `stage/play/mod.rs:448-498` |
| Result | Esc/Enter | `leave_play` → `Back` (라이브러리 없으면 `Quit`) | `stage/result.rs:106`, `app_play.rs:564-573` |
| Result | R / N | 재시도 / 다음 곡 → `To(Loading::song)` | `stage/result.rs:107-108`, `app_result.rs:442-466` |
| CourseResult | Esc/Enter | `end_course` → `Back` | `stage/course_result.rs:78-86`, `lib.rs:259-267` |
| Practice | Enter | `Open(Play)` (슬라이스), 종료 시 `Back` 으로 패널 복귀 | `stage/practice.rs:65-77, 96-97`, `app_result.rs:301-303` |

### 1.3 레퍼런스 상태와 전이

- 상태 7종: `MUSICSELECT, DECIDE, PLAY, RESULT, COURSERESULT, CONFIG, SKINCONFIG` (`B:MainState.java:174-182`), 스위치 `B:MainController.java:248-268`.
- 상태 전환 시 공통 절차(`B:MainController.java:270-284`): 이전 상태 `shutdown()` + `setSkin(null)` → 새 상태 `create()`(여기서 `loadSkin`) → `skin.prepare()` → `timer.setMainState()` → `prepare()`.
- `timer.setMainState` 는 모든 타이머를 OFF(`Long.MIN_VALUE`)로 채우고 장면 시작 시각을 현재로 다시 잡는다(`B:TimerManager.java:100-106`). 즉 "타이머 없는 목적지"의 시간축은 항상 "이 상태에 들어온 뒤 경과 ms" 이다.

| 출발 | 조건 | 도착 | 근거 |
| --- | --- | --- | --- |
| (시작) | bms 파일 지정 / 미지정 | PLAY / MUSICSELECT | `B:MainController.java:341-351` |
| MUSICSELECT | 곡 결정(`readChart`) | DECIDE | `B:select/MusicSelector.java:426` |
| MUSICSELECT | 코스 결정(`_readCourse`) | DECIDE | `B:select/MusicSelector.java:527` |
| MUSICSELECT | NUM6 | CONFIG | `B:select/MusicSelector.java:296-297` |
| MUSICSELECT | `OPEN_SKIN_CONFIGURATION` | SKINCONFIG | `B:select/MusicSelector.java:298-299` |
| DECIDE | `nowtime > skin.scene` 또는 키 → `TIMER_FADEOUT` → `fadeout` 경과 | PLAY (취소면 MUSICSELECT) | `B:decide/MusicDecide.java:37-63` |
| PLAY | FINISHED + `finishMargin` → FADEOUT → `fadeout` 경과 | RESULT / 연습 복귀 / 다음 코스 PLAY / 다음 곡 DECIDE / MUSICSELECT | `B:play/BMSPlayer.java:745-784` |
| PLAY | FAILED + `skin.close` 경과 | RESULT / MUSICSELECT, 빠른 재시작은 PLAY | `B:play/BMSPlayer.java:693-742` |
| RESULT | 입력 또는 `scene` 경과 → FADEOUT → `fadeout` 경과 | MUSICSELECT / PLAY(리플레이·다음 코스 스테이지) / COURSERESULT | `B:result/MusicResult.java:172-258` |
| COURSERESULT | 종료 | MUSICSELECT | `B:result/CourseResult.java:182` |
| CONFIG | 종료 | MUSICSELECT | `B:config/KeyConfiguration.java:244` |
| SKINCONFIG | Esc | `saveConfig` 후 MUSICSELECT | `B:config/SkinConfiguration.java:61-67` |

### 1.4 화면 대응표

| 레퍼런스 상태 (SkinType id) | ModernChic 진입 파일 | R-BMS 대응 Stage | 현재 스킨 배선 | 차이·주의 |
| --- | --- | --- | --- | --- |
| MUSICSELECT (5) | `M:musicselect.luaskin` | `Select` | `prepare_skin(SKIN_TYPE_MUSIC_SELECT)` (`stage/select/mod.rs:786`) | 탭(곡/코스), 기록 모달, 랭킹 패널, 필터 패널, 옵션 오버레이가 전부 내장 UI |
| DECIDE (6) | `M:decide.luaskin` | `Loading` (곡 로드일 때만 의미 있음) | `prepare_skin(SKIN_TYPE_DECIDE)` (`stage/loading.rs:478`) | 레퍼런스의 decide 는 "시간 연출 장면"이고 로딩은 PLAY 의 PRELOAD 단계에서 한다. R-BMS 는 로딩 화면을 decide 로 그린다. 스캔·테이블 받기 중에도 같은 문서를 그린다(`stage/loading.rs:475-485` 는 task 종류를 가리지 않음) |
| PLAY (0,1,2,3,4,16...) | `M:play5_hw / play7_hw / play10_hw / play14_hw.luaskin` | `Play` | `mode_skin_type(mode)` → `prepare_skin` (`stage/play/mod.rs:797-804`) | PRELOAD/READY/FAILED/FINISHED 단계 없음. 9키·24키용 ModernChic 문서 없음 |
| RESULT (7) | `M:result.luaskin` | `Result` | `prepare_skin(SKIN_TYPE_RESULT)` (`stage/result.rs:115`) | 입력 잠금·fadeout·랭킹 스크롤·게이지 종류 전환 없음 |
| COURSERESULT (15) | `M:course.luaskin` | `CourseResult` | 스킨 호출 없음. 내장 텍스트만(`stage/course_result.rs:99-118`) | 타입 15 는 로더가 지원 목록에 없음(`R:crates/rbms-skin/src/loader.rs:221, 240-242`) |
| CONFIG (8) | `M:keyconfig.luaskin` | `KeyConfig` | `draw_keyconfig_skin(keys)` (`stage/keyconfig.rs:185-189`) | 문서에 넘기는 상태가 레인 키 이름 목록뿐(`skin_screen.rs:895-906`). 입력 처리는 내장 행 리스트 기준 |
| SKINCONFIG (9) | `M:skinselect.luaskin` | 없음(가장 가까운 것은 `Settings` 의 SKIN 탭) | 없음 | 타입 9 도 로더 미지원. 전용 Stage 신설 필요 |
| — | — | `Settings` | 없음 (`render_settings`, `stage/settings.rs:658-671`) | R-BMS 전용. 레퍼런스는 런처(별도 창)에서 설정 |
| — | — | `Tables`, `Folders` | 없음 | R-BMS 전용 |
| — | — | `Practice` | 없음(`stage/practice.rs:138-156`) | 레퍼런스는 PLAY 상태 안의 `STATE_PRACTICE`(`B:play/BMSPlayer.java:524-595`)로 play 스킨 위에서 처리 |
| — | — | `Loading`(scan/table/song) | decide 문서 또는 내장 진행 막대 | R-BMS 전용 단계 |

레퍼런스에만 있고 R-BMS 에 없는 것: 독립 DECIDE 장면, SKINCONFIG 상태, PLAY 내부 6단계 상태기계(`STATE_PRELOAD/PRACTICE/PRACTICE_FINISHED/READY/PLAY/FAILED/FINISHED`, `B:play/BMSPlayer.java:69-75`), RESULT 의 IR 처리 상태(`STATE_OFFLINE/IR_PROCESSING/IR_FINISHED`, `B:result/AbstractResult.java:22-30`), 코스 스테이지 사이 RESULT → PLAY 직행(`B:result/MusicResult.java:200-208`; R-BMS 는 스테이지마다 Loading 을 거친다 `lib.rs:245-255`).

---

## 2. 스킨 선택·로드·캐시·리로드 흐름

### 2.1 데이터 구조

| 계층 | 타입 | 위치 | 역할 |
| --- | --- | --- | --- |
| 설정 | `SkinOptions { folder, screen, selected: BTreeMap<i32,String>, custom, shared, default_skin_installed }` | `R:crates/rbms-config/src/schema.rs:247-264` | 화면 타입 id → 문서 절대경로, 문서경로 → 커스터마이즈, 번들키 → 공유 커스터마이즈 |
| 디스크 목록 | `SkinLibrary { root, documents: Vec<SkinHeader>, loaded: BTreeMap<i32, LoadedDocument>, errors, stale, next_build }` | `R:apps/rbms-player/src/skin_select.rs:189-208` | 헤더 스캔 결과와 화면별 "전체 로드된 문서" |
| 컴파일 캐시 | `SkinScreens { built: BTreeMap<i32, BuiltScreen>, pending: BTreeMap<i32, PendingScreen> }` | `R:apps/rbms-player/src/skin_screen.rs:462-465` | 화면 타입별 `rbms_render::SkinScreen`(텍스처 보유)과 디코드 중인 작업 |
| 타이머 | `skin_timers: TimerState`, `skin_play_timers`, `skin_select_timers`, `skin_result_timers` | `lib.rs:547-550` | 전 화면 공용 타이머 표 1개 + 화면별 드라이버 3개 |

### 2.2 흐름

1. **루트 결정**: `skin_root()` = `config.skin.folder` 가 있으면 그 경로, 없으면 `<settings.ron 이 있는 폴더>/skin` (`skin_select.rs:716-721`, 기본 폴더명 `DEFAULT_SKIN_FOLDER = "skin"` `schema.rs:211`). 즉 기본은 `~/.config/rbms/skin/`.
2. **스캔**: Settings 화면 `on_enter` 와 RELOAD 행에서 `rescan_skins()` (`stage/settings.rs:582-589, 203-208`, `settings_ui.rs:161-164`). `scan()` 은 깊이 3(`SCAN_MAX_DEPTH`), 최대 512개(`SCAN_MAX_DOCUMENTS`), 확장자 `json`/`json5` 만(`DOCUMENT_EXTENSIONS`) 보고 `load_header` 가 성공한 파일만 모은다(`skin_select.rs:32-39, 727-762`). 헤더 로드 시 모드는 항상 `crate::MODE`(7키)로 넘긴다(`skin_select.rs:749`).
3. **선택**: SKIN 탭의 SCREEN 행이 `config.skin.screen`(화면 타입 id)을 바꾸고, SKIN 행이 그 타입의 후보를 순환한다. 순환 대상은 "내장 화면(None)" + 같은 `skin_type` 의 헤더들(`skin_select.rs:254-256, 620-634`). 선택은 `config.skin.select(screen, path)` 로 저장(`schema.rs:311-316`).
4. **전체 로드**: 화면이 그려질 때마다 `prepare_skin(canvas, screen)` 이 `needs_reload_for` 를 보고 `reload_for` 를 호출한다(`skin_screen.rs:573-583`). `reload_for` 는 `load_skin(path, SkinLoadOptions::new(&root, &user, mode))` 를 **프레임 루프에서 동기 실행**한다(`skin_select.rs:682-705`). 모드는 `skin_type_mode(screen).unwrap_or(MODE)` (`skin_select.rs:693`). 성공 시 `next_build` 를 증가시켜 빌드 번호를 준다(`skin_select.rs:697-699`). 실패 사유는 `errors[screen]` 에 남고 자동 재시도하지 않는다(`skin_select.rs:658-672, 701-703`).
5. **자산 디코드(비동기)**: `SkinScreens::sync` 가 빌드 번호 변화를 보고 이전 화면 텍스처를 `release` 하고 `PendingScreen::start` 로 `document.sources`(이미지)와 `document.fonts`(폰트 바이트)를 워커 풀에 건다(`skin_screen.rs:483-497, 427-434`). 워커 수는 코어 수(최대 8, `assets.rs:21, 523`).
6. **컴파일**: 풀이 끝난 프레임에 `SkinScreen::build(r, text, document, &mut assets)` (`skin_screen.rs:499-525`). 이미지/폰트는 이미 디코드된 것만 넘긴다(`PlayerSkinAssets`, `skin_screen.rs:48-84`). 표현식은 문서의 Lua 샌드박스에 컴파일(`skin_screen.rs:81-83`). 경고 첫 줄을 토스트로 알린다(`skin_screen.rs:513-517`).
7. **그 사이의 화면**: 컴파일 전에는 `SkinScreens::get` 이 `None` 이라 **내장 레이아웃이 그려진다**(`skin_screen.rs:473-479, 687`). 스킨 단독 소유로 바꾸면 이 구간에 그릴 것이 없어지므로 "로드 중 화면" 정책을 새로 정해야 한다(§8.4).
8. **캐시 수명**: 컴파일된 화면은 타입별로 앱 수명 동안 유지된다(`skin_screen.rs:456-460`). 레퍼런스는 상태를 떠날 때마다 스킨을 dispose 한다(`B:MainController.java:272-273`, `B:MainState.java:116-120`). 1920x1080 스킨 10종을 전부 상주시키면 GPU 메모리 사용이 크다(§11 위험).
9. **리로드 트리거**: (a) 선택 경로 변경(`needs_reload_for` 의 `entry.path != path`, `skin_select.rs:671`), (b) property/file 행 변경 → `stale` (`skin_select.rs:533, 547`), (c) 번들 공유 행 변경 → 같은 번들의 모든 화면 stale(`skin_select.rs:569-573`), (d) RELOAD 행(`stage/settings.rs:203-208`), (e) RESET 행(`settings_ui.rs:155-158`). offset 행은 리로드 없이 매 프레임 `MergedOffsets` 로 읽는다(`skin_screen.rs:378-387, 658-666`).
10. **주의**: `AppShared::reload_skin()` 은 "SKIN 탭이 지금 보고 있는 화면 타입" 하나만 다시 읽는다(`settings_ui.rs:167-169` → `skin_select.rs:677-679`). 다른 화면은 다음에 그 화면이 그려질 때 `prepare_skin` 이 읽는다.

### 2.3 문서 활성화 게이트(프리셋 결합)

`skin_document_is_enabled(settings_path, config, screen)` (`R:apps/rbms-player/src/assets.rs:305-318`):

- 선택이 없으면 false.
- 선택 경로가 내장 번들 세대(`steel-neon`, `steel-neon-v2`, `steel-neon-v3`)의 문서 경로가 아니면 **항상 true**.
- 내장 번들 문서면 `config.display.skin == "STEEL NEON"` 이고 활성 세대 폴더의 경로일 때만 true.

즉 DISPLAY 의 `SKIN` 프리셋 행(`NORMAL`/`WIDE`/`STEEL NEON`, `schema.rs:176-177`)이 내장 번들 문서의 on/off 스위치 역할을 한다. 이 프리셋은 동시에 (a) 플레이 필드 RON(`play.ron`/`play-dual.ron`, `assets.rs:211-214`, `app_play.rs:355-367`), (b) UI 테마 `theme.ron`(`assets.rs:216-221, 455-470`), (c) 번들 `sound/` 폴더(`assets.rs:321-327`)까지 고른다. 스킨 단독 소유로 가면 이 결합 전체가 제거 대상이다.

---

## 3. 기본 번들 설치와 바이너리 영향

### 3.1 현재 방식

- `BundledFile { path, bytes: &'static [u8] }` 를 `include_bytes!` 로 채운다(`assets.rs:41-44, 58-175`). v1 20개, v2 24개, v3 66개 파일이 **세 세대 모두** 바이너리에 들어 있다(`BUNDLE_GENERATIONS`, `assets.rs:188-192`). 별도로 `normal.ron`/`wide.ron` 이 `include_str!`(`assets.rs:26-27`).
- 소스 크기: `assets/skins/steel-neon` 148KB, `steel-neon-v2` 392KB, `steel-neon-v3` 1.3MB(du 실측). 로컬 release 바이너리는 22,220,464 바이트(`target/release/rbms-player`, 2026-10-09 빌드).
- 설치 시점: `run()` 초입에서 `needs_default_skin_install` → `install_default_skin` (`lib.rs:1125-1134`). 조건은 `!config.skin.default_skin_installed` 이거나 현 세대 파일이 하나라도 없을 때(`assets.rs:223-229`).
- 설치 규칙: 파일이 이미 있으면 **절대 덮어쓰지 않는다**(`create_new(true)`, `assets.rs:399-422`). 사용자가 고친 파일 보존이 목적. 이전 세대 폴더는 "이미 있는 경우에만" 보수한다(`assets.rs:235-238`).
- 세대 이동: 선택이 이전 세대의 "출하본과 바이트가 같은" 문서를 가리키면 현 세대 경로로 옮기고 `custom`/`shared` 선택도 옮긴다(SHA-256 비교, `assets.rs:262-303, 380-388`).
- 최초 설치 시 선택 채우기: 프리셋이 기본값(`NORMAL`)이면 `STEEL NEON` 으로 바꾸고 화면 타입 8종(`STEEL_NEON_DOCUMENTS`: 5, 6, 0, 1, 2, 3, 4, 7)에 문서를 지정한다(`assets.rs:177-186, 243-257, 367-376`). key config(8)는 지정하지 않는다.
- 테마: `~/.config/rbms/theme.ron` 템플릿도 같이 쓴다(`assets.rs:240-241, 427-453`).

### 3.2 바이너리와 엮인 부수 효과

- `compute_build_hash()` 가 시작 시 **실행 파일 전체를 메모리로 읽어** SHA-256 을 낸다(`lib.rs:339-345`, `lib.rs:835`). IR 제출의 `client_build_sha256` 에 쓰인다(`app_result.rs:199`). 바이너리가 수백 MB 가 되면 시작 지연·메모리 급증이 직접 생긴다.
- 릴리스 산출물은 "단일 바이너리 + LICENSE + README" 를 tar/zip 한 것이다(`R:.github/workflows/release.yml:75-92, 111-120`). 리소스 폴더를 같이 싣는 단계가 없다. macOS 는 `lipo` 로 두 아키텍처를 합치므로 내장 자산은 두 번 들어간다(`release.yml:78-83`).
- 설치는 "복사"이므로 내장 크기만큼 디스크를 한 번 더 쓴다.
- 덮어쓰지 않는 규칙 때문에 기본 스킨을 고치려면 세대(폴더명)를 올려야 한다. 대형 스킨에서는 세대마다 전체 복사본이 쌓인다.

### 3.3 대형 스킨을 기본으로 둘 때의 배포 방식 비교

ModernChic 원본 실측(`find`/`stat`): 총 478MB. png 284개 212.3MB, mp4 8개 192.8MB, ttf 11개 54.7MB, fnt 10개 12.8MB, ogg 73개 2.6MB, lua 126개 1.1MB. 폴더별: Play 214MB, Select 115MB, Result 69MB, Decide 66MB, SkinSelect 11MB, Root 2.9MB, Sound 1.2MB. 최대 단일 파일은 `Play/parts/common/BGA/movie/NOSTALGIC.mp4` 48.4MB.

| 방식 | 구현 지점 | 장점 | 영향·제약 |
| --- | --- | --- | --- |
| A. 지금처럼 `include_bytes!` 내장 + 최초 실행 복사 | `assets.rs` 표 확장 | 단일 파일 배포 유지, 오프라인 최초 실행 가능 | 바이너리 +283MB(mp4 제외) ~ +478MB, macOS 유니버설은 약 2배. 컴파일 시간·링커 메모리 증가. `compute_build_hash` 가 전체를 읽음. 디스크 2배. 파일 표를 손으로 관리(현재 110줄, 원본은 500여 파일). 세대 올릴 때마다 전체 재복사. 사실상 부적합 |
| B. 저장소 폴더 + 실행 시 직접 참조(복사 없음) | `skin_root` 후보에 "실행 파일 옆 `skins/`" 와 개발용 `CARGO_MANIFEST_DIR` 상대 경로 추가, `release.yml` 에 폴더 동봉 | 바이너리 크기 불변, 복사 없음, 스킨 수정이 바로 반영 | 배포물이 "바이너리 + 폴더" 가 됨(tar/zip 구조 변경). 읽기 전용 위치일 수 있으므로 스킨이 쓰는 파일(`M:io/**`, `M:History/**`)의 쓰기 경로를 사용자 폴더로 돌려야 함. git 저장소가 수백 MB 증가(현재 `.git` 15MB). GitHub 단일 파일 100MB 제한에는 걸리지 않음(최대 48.4MB) |
| C. 저장소 폴더 + 최초 실행 시 사용자 폴더로 복사 | B + 디렉터리 복사 루틴(현 `install_bundle` 을 파일 표 대신 디렉터리 순회로) | 사용자가 고칠 수 있는 사본 확보, 스킨의 쓰기 경로 문제 없음 | 디스크 2배, 최초 실행 시 수 초 복사, 갱신 정책(덮어쓰기 금지 규칙과 충돌) 재설계 필요 |
| D. 앱 번들 리소스(macOS `.app/Contents/Resources`, Windows 설치 폴더) | 릴리스 파이프라인에 번들 생성 추가 | OS 관례에 맞음 | 현재 `.app` 번들을 만들지 않음(`release.yml` 은 맨 바이너리). 서명·공증 범위가 늘어남. 번들 내부는 읽기 전용 취급 |
| E. 별도 다운로드(최초 실행 또는 설정 화면에서) | 다운로드·검증·압축 해제 루틴 신설, 호스팅 필요 | 바이너리·저장소 모두 가벼움 | 오프라인 최초 실행에 스킨이 없음 → 최소 폴백 화면이 반드시 필요. 호스팅·체크섬·실패 처리 필요 |

판단 재료:

- 사용자 지시는 "제공한 스킨을 조금 더 심플하게 해서 기본 스킨으로". 단순화 판에서 mp4 를 빼면 285MB, 폰트·대형 PNG 를 더 줄이면 수십 MB 까지 내려갈 여지가 있다(실제 감량 폭은 미확인, 스킨 본체 조사 담당의 몫).
- 어느 방식이든 "스킨이 하나도 없을 때의 화면"을 정해야 한다. 지금은 내장 네이티브 화면이 그 역할을 하지만, 단독 소유 전환 뒤에는 (a) 네이티브 화면을 폴백으로 남길지 (b) 아주 작은 내장 스킨을 따로 둘지 결정해야 한다.
- 재배포 권리: ModernChic 폴더에는 폰트 라이선스(`M:Decide/SIL_Open_Font_License_1.1.txt` 등)와 `readme.txt` 가 폴더마다 있고 `M:Sound/使用させていただいた楽曲.txt` 가 있다. 이미지·동영상·음원의 재배포 조건은 이번 범위에서 읽지 않았다(미확인). GPL-3.0 저장소에 넣기 전에 확인이 필요하다.

---

## 4. SKIN 탭 커스터마이즈 저장 구조

- 고정 행 5개는 `rbms_config::SETTINGS` 에 있고(SCREEN / SKIN / LOADED / RELOAD / RESET — `SettingId::SkinScreen, SkinDocument, SkinInfo, SkinReload, SkinReset`, 기본 표시값은 `settings_ui.rs:248-252`), 문서가 선언한 행은 `SkinReload` 행 바로 위에 끼워 넣는다(`SKIN_CUSTOM_ANCHOR`, `settings_ui.rs:19, 115-126`).
- 문서 선언 행 종류 `SkinRow`: `BundleProperty / BundleFile / BundleOffset(axis)` + `Property / File / Offset(axis)` (`skin_select.rs:158-171`). 축은 `X, Y, W, H, R, A` (`skin_select.rs:79-87`), 한 스텝 1.0, 한계 ±2000(위치·크기) / ±360(각도) / ±255(알파) (`skin_select.rs:42-53`).
- 저장 형태 `SkinCustomisation { properties: BTreeMap<String,i32>, filepaths: BTreeMap<String,String>, offsets: BTreeMap<i32,SkinOffset> }` (`schema.rs:223-232`). property 는 "행 이름 → 선택된 op id", file 은 "슬롯 이름 → 후보 파일명", offset 은 "offset id → 6축 값".
- 두 스코프: `custom[문서 절대경로]`(문서 단위)와 `shared[번들키]`(번들 단위). 번들키는 "스킨 폴더 바로 아래 디렉터리 이름"(`bundle_key`, `schema.rs:339-343`). 문서가 스킨 폴더 바로 아래에 있으면(디렉터리 1단 없음) 번들키가 없다(`first_directory_name`, `schema.rs:289-294`).
- 로더에 넘기는 값: `user_config(path)` = 번들 공유값 위에 문서 값을 덮은 `SkinUserConfig` (`schema.rs:373-387`).
- 번들 공유 행은 문서 헤더에서 `scope: 'bundle'` 로 선언된 것만이다(`shared_scope`, `skin_select.rs:766-768`). 이것은 R-BMS 자체 확장이며 레퍼런스 스킨에는 없다.
- 레퍼런스 대응: 화면 타입마다 `SkinConfig { path, properties { option[], file[], offset[] } }` 하나, 스킨을 바꾸면 이전 설정을 `skinHistory` 에 보관(`B:config/SkinConfiguration.java:151-197, 205-215, 236-244`). 옵션은 이름→값, 파일은 이름→경로, 오프셋은 id→(x,y,w,h,r,a) (`B:config/SkinConfiguration.java:250-378`). 저장 모델 자체는 R-BMS 의 `custom[path]` 와 1:1 로 맞는다.
- 차이: ModernChic 은 같은 이름의 옵션을 여러 `.luaskin` 이 각자 선언한다(스킨 본체 조사 담당 확인 사항). 레퍼런스는 화면 타입별로 따로 저장하므로 화면마다 값이 독립이다. R-BMS 의 번들 공유 스코프는 `.luaskin` 에는 적용되지 않는다(선언 문법이 없음).

---

## 5. "스킨 폴더 하나를 가리켜 10개 .luaskin 을 화면별로 쓰기" 위해 바꿀 곳

| # | 바꿀 곳 | 현재 | 필요한 변경 |
| --- | --- | --- | --- |
| 1 | `skin_select.rs:39` `DOCUMENT_EXTENSIONS` | `["json","json5"]` | `luaskin` 추가. 레퍼런스는 `.json`, `.luaskin`, 그 외(LR2)만 본다(`B:config/SkinConfiguration.java:489-507`) |
| 2 | `skin_select.rs:727-755` `scan` → `load_header` | JSON 헤더 파서만 | `.luaskin` 은 `skin_config == nil` 로 실행해 `t.header` 를 받는 경로 필요(`M:play7_hw.luaskin:2-5`). 헤더 126개 Lua 를 스캔마다 실행하면 느리므로 캐시 고려 |
| 3 | `R:crates/rbms-skin/src/loader.rs:221, 240-242` `is_supported_skin_type` | 5, 6, 7, 8 + play 모드 | 9(SKIN SELECT), 15(COURSE RESULT) 추가. 라벨 표에는 이미 있음(`schema.rs:185-205`) |
| 4 | `skin_select.rs:682-705` `reload_for` | 프레임 루프에서 동기 `load_skin` | 2.5만 줄 Lua 를 실행하므로 워커 스레드로 이동 필요(샌드박스 `Send` 여부는 미확인) |
| 5 | `skin_select.rs:694-695` 샌드박스 루트 | 스킨 루트 폴더 전체 | 문서별 스킨 폴더 기준 `require`/파일 접근 규칙, 그리고 스킨이 쓰는 `io/`·`History/` 쓰기 허용 범위 결정 |
| 6 | `assets.rs:305-318` `skin_document_is_enabled` | 프리셋과 결합 | 프리셋 결합 제거. "선택돼 있으면 사용" 한 줄로 |
| 7 | `assets.rs:177-186, 367-376` 기본 선택 | 타입 8종에 JSON 문서 지정 | "스킨 팩 폴더를 고르면 그 안의 모든 타입을 한 번에 지정" 하는 함수로 교체. 현재 SKIN 탭은 화면 타입마다 따로 골라야 한다(`skin_select.rs:620-634`) |
| 8 | `schema.rs:339-343` `bundle_key` | 스킨 루트 아래 1단 디렉터리 | `config.skin.folder` 를 스킨 팩 폴더 자체로 지정한 경우(문서가 루트 직하) 번들키가 없어짐. 팩 단위 식별자를 "문서의 부모 디렉터리"로 재정의하거나, 폴더 지정은 "팩들의 부모" 로 고정 |
| 9 | `stage/course_result.rs:99-118` | 스킨 호출 없음 | `prepare_skin(15)` + 상태 공급 + 타이머 |
| 10 | 신규 Stage(SkinSelect) | 없음 | 타입 9 문서를 그리고 `BUTTON_CHANGE_SKIN`, 커스터마이즈 버튼, 슬라이더(`M:skinselect.lua:122` `SKINSELECT_POSITION`), 스킨 타입 선택 버튼을 처리(`B:config/SkinConfiguration.java:114-149`) |
| 11 | `stage/loading.rs` | decide 문서를 로딩 화면으로 사용 | decide 를 독립 장면으로 분리하고 로딩은 play 스킨의 PRELOAD 단계로 이동(§6.5) |
| 12 | `stage/play/mod.rs:797` `mode_skin_type` | 모드 → 타입 0/1/2/3/4/16 | ModernChic 에 9키·24키 문서가 없음. 문서가 없는 모드의 폴백 정책 필요 |
| 13 | `app_input.rs:282-329` `rebuild_skin`/`document_field` | 문서의 `note.dst` 로 내장 필드 좌표를 덮음 | 스킨 단독 소유면 내장 `Skin`(RON 필드) 자체가 필요 없어짐. 키 봄·레인 판정 측(`field_side`, `stage/play/mod.rs:243-248`)이 내장 `Skin.x/fields` 에 의존하므로 대체 좌표원이 필요 |
| 14 | `lib.rs:218-223` `sound_folder_path` / `assets.rs:321-327` | 프리셋이 STEEL NEON 일 때만 번들 `sound/` | 선택된 스킨 팩의 사운드 폴더 규칙으로 교체(§10.1) |

---

## 6. 화면별 상태 공급과 타이머

### 6.1 공통 뼈대

- 시계: `skin_now_ms()` = `self.clock.elapsed().as_millis()` (`skin_screen.rs:567-569`). `clock` 은 `App::new`(`lib.rs:810`)와 `start_play()`(`app_play.rs:673-679`)에서만 다시 잡힌다.
- 한 프레임 조립: `skin_frame()` 이 `SkinDraw { screen, timers: &self.skin_timers, now_ms, lua, mouse, background, offsets, extra }` 를 만든다(`skin_screen.rs:676-699`). `mouse` 는 논리 커서를 문서 좌표(y 상향, 저작 크기)로 바꾼 값(`document_cursor`, `skin_screen.rs:556-563`).
- 화면별 상태는 `SkinStateSource` 구현체(`PlayViewState`, `SelectViewState`, `ResultViewState`, `DecideViewState`, `KeyConfigViewState` — 모두 `rbms_render::skin_render::state`, `skin_screen.rs:22`)와, 프로퍼티 id 로 담기 어려운 것을 싣는 `FrameExtra::{None, Play(&PlayObjectState), Select(&SelectListState), Result(&ResultSeriesState)}` 로 넘긴다.
- 타이머 표는 화면 전환 때 지워지지 않는다(앱 코드에 `skin_timers.clear()` 호출 없음, grep 결과 0건). 레퍼런스는 전환마다 전부 OFF(`B:TimerManager.java:100-106`).

### 6.2 Play

공급 지점: `PlayState::draw` → `draw_document` (`stage/play/mod.rs:593-647`).

| 넘기는 것 | 필드 | 근거 |
| --- | --- | --- |
| `HudView` | `mode_label, combo, last_judge, last_fast, fast[2], slow[2], counts[6], ex_score, gauge, green_number, white_number, judge_text_y, max_ex, best_ex, pace{name, delta}` | `stage/play/mod.rs:841-860` |
| `PlayViewState` | `title, song_ms, duration_ms, bpm, hispeed, autoplay, now_ms, offsets, field, shade, judged_side, gauge_kind, artist, level, bpm_min, bpm_max, bpm_main, target_ex, target_delta` | `stage/play/mod.rs:611-632` |
| `PlayObjectState` (FrameExtra) | `field(&Skin), playfield{timelines, microtime, hispeed, beam_on, beam_off, constant, legacy_note}, shade{cover, hidden}, gauge_kind(0..5), bomb, keys_down, recent_hits(최근 64개)` | `stage/play/mod.rs:633-642, 861-870, 42` |
| 배경 텍스처 | 현재 BGA 프레임 1장(`self.bga.get(&play.bga_frame())`) | `stage/play/mod.rs:809-822` |

타이머:

| 시점 | 호출 | 켜지는 타이머 | 근거 |
| --- | --- | --- | --- |
| `on_enter` | `skin_play_timers.start` | `PLAY` on, `READY` off (즉시) | `stage/play/mod.rs:694-698`, `R:crates/rbms-render/src/skin_render/screen.rs:262-265` |
| 매 프레임(문서가 있을 때만) | `skin_play_timers.update(hud, total_notes, now, lanes)` | `JUDGE_1P/2P, COMBO_1P/2P, FULLCOMBO_1P, GAUGE_MAX_1P, GAUGE_INCLEASE_1P, KEYON/KEYOFF/HOLD/BOMB_{1P,2P}_{SCRATCH,KEY1..KEY10}` | `stage/play/mod.rs:603-606`, `screen.rs:277-` (타이머 id 는 grep 집계) |
| 실패 감지 프레임 | `skin_play_timers.fail` | `FAILED` | `stage/play/mod.rs:600-602` |

주의점:

- 타이머 갱신이 `update` 가 아니라 `draw_document` 안에 있고, 문서가 없으면 함수 초입에서 빠져나가므로(`stage/play/mod.rs:595-597`) 문서 로드 전 프레임의 이벤트는 타이머에 반영되지 않는다.
- 레인 → 타이머 키 번호는 내장 `Skin.x` 를 x 좌표순으로 정렬해 매긴다(`lane_timer_states`, `stage/play/mod.rs:265-286`).

공급하지 못하는 것(레퍼런스 근거 포함):

| 항목 | 레퍼런스 | R-BMS 상태 |
| --- | --- | --- |
| PRELOAD 단계(로딩 진행, `loadstart`/`loadend`) | `B:play/BMSPlayer.java:480-522` | Loading Stage 가 대신함. play 스킨의 로딩 연출이 재생되지 않음 |
| READY 단계(`TIMER_READY`, `playstart`, `PLAY_READY` 사운드 후 대기) | `B:play/BMSPlayer.java:513-515, 605-618` | `on_enter` 에서 사운드만 울리고 즉시 PLAY (`stage/play/mod.rs:695-697`) |
| `TIMER_STARTINPUT` | `B:play/BMSPlayer.java:470-472` | 없음 |
| `TIMER_RHYTHM` | `B:play/BMSPlayer.java:609-610, 628` | 없음(드라이버 목록에 없음) |
| FAILED 후 `close` 시간 동안의 폐점 연출 | `B:play/BMSPlayer.java:693-742` | 실패 즉시 결과로 전환(`stage/play/mod.rs:431-436, 732-734`) |
| `TIMER_MUSIC_END`, `TIMER_ENDOFNOTE_1P`, `finishMargin`, `TIMER_FADEOUT` | `B:play/BMSPlayer.java:679-690, 745-751` | 없음. 종료 즉시 전환 |
| `TIMER_SCORE_A/AA/AAA/BEST/TARGET` | `B:play/BMSPlayer.java:1033-1037` | 없음 |
| `TIMER_PM_CHARA_*` | `B:play/BMSPlayer.java:518-521, 676` | 없음 |
| 라이벌/타깃 스코어 데이터(고스트 포함) | `B:play/BMSPlayer.java:447-453` | `target_ex` 숫자와 pace delta 만 |
| BGA 레이어·POOR 레이어 분리 | (렌더 측, 미확인) | 프레임 1장만 넘김(`stage/play/mod.rs:809`) |
| 동영상 BGA | (렌더 측) | 이미지 확장자만 디코드(`assets.rs:649-651`) |
| 연습 모드 패널(play 스킨 위) | `B:play/BMSPlayer.java:524-595, 988` | 별도 내장 `Practice` Stage |
| 코스 중 게이지/콤보 승계 표시, 코스 제약 표시 | `B:play/BMSPlayer.java:419-424` | 엔진은 승계(`app_play.rs:311, 430-431`)하나 스킨 상태로는 미공급 |

### 6.3 Select

공급 지점: `SelectState::draw` (`stage/select/mod.rs:779-868`) → `draw_select_skin[_layer]` (`skin_screen.rs:777-821`).

- 원천은 `SelectScene`(캐시됨, 키는 `select_key` `stage/select/mod.rs:125-131`): `rows[], sel, header, guide, detail, modal, score_graph, search, sort, filter, empty_hint` (`stage/select/scene.rs:198-210`).
- 행 `SelectRow`: `folder, title, mode_short, mode_color, level, difficulty_color, lamp(색), folder_count, dj_level, favorite` (`scene.rs:35-62`). 램프가 **색 값**이지 램프 id 가 아니다(`scene.rs:49`).
- 상세 `DetailView`: `accent, title, subtitle, artist, genre_maker, mode_short, level, difficulty_name, cover(Present/None), stats[6]{BPM, DENSITY, NOTES, JUDGE, LENGTH, TOTAL}, density, records{plays, clears, best, rank_bar, recent[]}` (`scene.rs:114-136`). 통계 값은 이미 **문자열로 포맷된** 상태다(`scene.rs:83-98, 127-132`).
- 옵션 패널 행: `OptionsRows { labels[11], values[11], focused, open }` (`app_options.rs:218-225`), 행 구성 `Random, Gauge, HiSpeed, FixHiSpeed, LaneCover, Lift, Hidden, ScratchSide, ScratchAuto, Autoplay, Target` (`app_options.rs:29-41`).
- 배경 텍스처: 포커스 곡의 stagefile(없으면 banner) 1장(`stage/select/mod.rs:309-312, 791-804`).

타이머:

| 시점 | 타이머 | 근거 |
| --- | --- | --- |
| 매 프레임 `skin_select_timers.update(row, now)` | `SONGBAR_MOVE, SONGBAR_MOVE_UP, SONGBAR_MOVE_DOWN, SONGBAR_CHANGE` | `stage/select/mod.rs:805-807`, `screen.rs:351-` |
| 옵션 오버레이 열고 닫을 때 | `PANEL1_ON`, `PANEL1_OFF` | `app_options.rs:207-211` |

공급하지 못하는 것:

| 항목 | 레퍼런스 | R-BMS 상태 |
| --- | --- | --- |
| `TIMER_STARTINPUT` (`skin.input` 이후 입력 허용) | `B:select/MusicSelector.java:193-195` | 없음 |
| 패널 2·3 (`PANEL2/3_ON/OFF`): 어시스트 옵션, 상세 옵션 | `B:select/MusicSelector.java:553-565`, `B:select/MusicSelectInputProcessor.java:114-117` 및 그 이후 블록(§7.4) | 패널 1 하나뿐, 내용도 R-BMS 고유 11행 |
| 패널 내 개별 옵션 값(랜덤 1P/2P, DP 옵션, 게이지, HS fix, BGA, 게이지 자동 전환, 판정 타이밍, duration, 타깃 등)의 id 기반 공급 | `B:select/MusicSelectInputProcessor.java` 의 `executeEvent(EventType.*)` 호출들 | 라벨·값 문자열 11쌍만(`app_options.rs:218-225`) |
| IR 랭킹(`RankingData`, 상태 타이머 `IR_CONNECT_BEGIN/SUCCESS/FAIL`, 랭킹 오프셋·스크롤) | `B:select/MusicSelector.java:223-250, 672-694` | 내장 랭킹 패널로만 존재(`stage/select/mod.rs:386-400, 856-859`), 스킨 상태 미공급 |
| 라이벌(선택된 라이벌, 라이벌 스코어 캐시, 라이벌 램프) | `B:select/MusicSelector.java:135-154, 375-376` | 라이벌 id 목록만 설정에 있음(`schema.rs:675`). 라이벌 정렬은 제목순 폴백(`stage/select/list.rs:149`) |
| 코스 바(GradeBar: 구성곡, 제약, 트로피), 랜덤 코스 | `B:select/MusicSelector.java:236-245, 265-275, 571-587` | 코스 탭이 폴더 행으로 그려짐(`scene.rs:218-254`). 제약은 `level` 문자열 배지 |
| 바 종류 구분(곡/폴더/테이블/해시/검색결과/실행형/같은 폴더 등) | `B:select/MusicSelector.java` 의 `SongBar, DirectoryBar, TableBar, HashBar, SearchWordBar, ExecutableBar, SameFolderBar, GradeBar, RandomCourseBar` | `SelectItem::{Song, Folder}` 두 종류(`lib.rs:406-409`) |
| 검색 텍스트 입력(스킨의 편집 가능 텍스트 객체) | `B:skin/Skin.java:394-405`, `B:select/MusicSelector.java:311-321` | 내장 검색 상자(`/` 키, `stage/select/mod.rs:208-247, 587-628`). 스킨에는 표시용 문자열만(`scene.rs:206`) |
| 리플레이 슬롯 4개와 선택된 슬롯 | `B:select/MusicSelector.java:49, 334-340` | 기록별 리플레이 파일(`stage/select/mod.rs:336-361`). 슬롯 개념 없음 |
| 배너와 스테이지파일을 별도 이미지로 | `B:select/MusicSelector.java:636-644` | 둘 중 하나만 1장 |
| 노트 분포 그래프용 BMS 모델(선택 350ms 뒤 로드) | `B:select/MusicSelector.java:212-222` | `ChartDetail.density` 로 일부 대응(150ms 디바운스, `stage/select/mod.rs:295-313`, `lib.rs:165`) |
| 모드 필터 / LN 모드 / 정렬 id | `B:select/MusicSelectInputProcessor.java:86-96` | 내장 필터 패널(`stage/select/filter.rs`)과 `SortMode` 라벨 문자열(`scene.rs:207`) |
| 플레이어 통계(`PlayerData`) | `B:select/MusicSelector.java:161` | 미공급(보유 여부 미확인) |
| select BGM(`SELECT` 사운드를 프리뷰 기본음으로 루프) | `B:select/MusicSelector.java:173-174, 187-189` | 없음. `SystemSound::Select` 는 곡 결정 시 1회 효과음으로 사용(`stage/select/mod.rs:147`) |

### 6.4 Result

공급 지점: `ResultState::draw` (`stage/result.rs:113-143`) → `draw_result_skin[_layer]` (`skin_screen.rs:842-878`).

- `ResultView`: `title, artist, mode_label, counts[6], ex_score, max_score, max_combo, total_notes, fast[2], slow[2], gauge, clear_label, clear_color, prev_best_ex, prev_ex, show_graph, show_result_graphs, gauge_series, timing_hist, judge_dist` (`app_result.rs:106-127`).
- `ResultViewState::new(view, target, cleared, now_ms, offsets).with_hint(run_again).with_ir_status(text)` (`skin_screen.rs:844-846`). IR 상태는 여러 줄을 공백 3칸으로 이어 붙인 한 줄 문자열(`skin_screen.rs:375, 648-650`).
- `ResultSeriesState { gauge_series, timing_hist, judge_dist, bpm_points }` (`stage/result.rs:60-67`).

타이머: `on_enter` 에서 `skin_result_timers.enter(view, now)` — `RESULTGRAPH_BEGIN` on, 신기록이면 `RESULT_UPDATESCORE` on(`stage/result.rs:89-93`, 테스트 `stage/result.rs:273-306`). 매 프레임 `update(now)` — 1000ms 뒤 `RESULTGRAPH_END` on(`stage/result.rs:116-117, 283-286`).

공급하지 못하는 것:

| 항목 | 레퍼런스 | R-BMS 상태 |
| --- | --- | --- |
| `TIMER_STARTINPUT`, 입력 잠금, `scene` 자동 종료, `TIMER_FADEOUT` + `fadeout` 후 전환 | `B:result/MusicResult.java:168-173, 250-258, 267-298` | 없음. 키 즉시 전환(`stage/result.rs:101-111`) |
| `RESULT_UPDATESCORE` 의 2단계 동작(`rankTime != 0` 이면 첫 입력에 켜고 두 번째 입력에 종료) | `B:result/MusicResult.java:165-167, 286-298` | 진입 시 한 번 결정 |
| `RESULTGRAPH_BEGIN/END` 를 진입 즉시 둘 다 on | `B:result/MusicResult.java:162-163` | END 를 1000ms 뒤에 켬(동작 차이) |
| IR 처리 상태와 `IR_CONNECT_*` 타이머, 랭킹·순위·이전 순위·총 인원, 랭킹 스크롤 | `B:result/MusicResult.java:75-144`, `B:result/AbstractResult.java:217-270` | 상태 문자열 한 줄만 |
| 게이지 종류 전환(CHANGE_GRAPH 키로 게이지 그래프 종류 순환) | `B:result/MusicResult.java:271-276` | 없음. 게이지 로그도 1종만(`app_result.rs:124`) |
| 리플레이 저장 슬롯 4개 상태(`ReplayStatus`)와 NUM1~4 저장 | `B:result/AbstractResult.java:56-57, 71-72`, `B:result/MusicResult.java:300-308` | 자동 저장 1개(`app_result.rs:85-91, 208-238`) |
| 이전 스코어 전체(`oldscore`: 클리어, 콤보, minbp 등) | `B:result/AbstractResult.java:64, 239-241` | `prev_best_ex`, `prev_ex` 만 |
| 결과 BGM 루프, `RESULT_CLOSE` 를 fadeout 시작에 재생 | `B:result/MusicResult.java:150-158, 252-256` | 진입 1회 재생, 나갈 때 `ResultClose`(`stage/result.rs:89-97`) |
| 코스 결과 전용 상태 | `B:result/CourseResult.java` (본문 미열람) | 내장 행 목록(`stage/course_result.rs:43-56`) |

### 6.5 Decide(=Loading)와 KeyConfig

- Decide: `DecideViewState { progress, done, title, chart{subtitle, artist, genre, level}, now_ms, offsets }` (`skin_screen.rs:881-892`, `stage/loading.rs:393-417`). `chart` 는 `LoadingTask::Song` 일 때만 채워지고 `Assets` 단계에서는 기본값이다(`stage/loading.rs:393-403`). 타이머 없음.
- 레퍼런스 decide 는 로딩과 무관한 시간 장면이다: `input` 경과 후 `STARTINPUT`, `scene` 경과 또는 키 입력으로 `FADEOUT`, `fadeout` 경과 후 PLAY(`B:decide/MusicDecide.java:37-63`). 진입 시 `DECIDE` 사운드(`B:decide/MusicDecide.java:32-35`). R-BMS 는 로딩 **완료** 시 `Decide` 사운드를 울린다(`stage/loading.rs:463`).
- KeyConfig: `KeyConfigViewState { keys: &[String], now_ms, offsets }` (`skin_screen.rs:895-906`). 포커스 행, 캡처 중 여부, 편집 모드, 컨트롤러/MIDI 바인딩 등은 미공급. 입력은 내장 행 모델(`KcRow`, `lib.rs:361-386`) 기준으로만 동작(`stage/keyconfig.rs:135-180`).

### 6.6 공통으로 빠진 장면 속성

레퍼런스 스킨 객체가 제공하고 각 상태가 읽는 장면 시간: `getInput()`, `getScene()`, `getFadeout()` (select/decide/result 공통, `B:select/MusicSelector.java:193`, `B:decide/MusicDecide.java:39-47`, `B:result/MusicResult.java:168-173, 250`), play 전용 `getLoadstart()`, `getLoadend()`, `getPlaystart()`, `getClose()`, `getFinishMargin()` (`B:play/BMSPlayer.java:499, 606, 711, 748`), result 전용 `getRankTime()` (`B:result/MusicResult.java:165`). R-BMS 앱은 이 값들을 읽는 곳이 없다(앱 소스 grep 결과 0건). 로더가 값을 보존하는지는 렌더·로더 조사 담당 범위(미확인).

---

## 7. 입력

### 7.1 분배 구조

| 장치 | 진입 | 분배 | 근거 |
| --- | --- | --- | --- |
| 키보드 | `WindowEvent::KeyboardInput` → `KeyInput { code(물리 키), pressed(반복 제외), released, text }` | `Stage::handle_key`: 옵션 오버레이가 먼저(`app_options::options_key`), 남으면 화면 | `lib.rs:1045-1058`, `stage/mod.rs:123-128, 263-269` |
| 마우스 이동 | `CursorMoved` → `gpu.logical_from_physical` → `shared.cursor` (논리 1280x720) | 상태만 저장. 화면에 이벤트 전달 없음 | `lib.rs:1034-1038`, `R:apps/rbms-player/src/gpu/mod.rs:511-513, 624-627` |
| 마우스 클릭 | `MouseInput { Pressed, Left }` 만 | `Stage::handle_mouse(at)`: 옵션 오버레이가 열려 있으면 삼킴 | `lib.rs:1039-1044`, `stage/mod.rs:276-281` |
| 마우스 release / 우클릭 / 가운데 / 휠 | 처리 없음(`_ => {}`) | — | `lib.rs:1065` |
| 게임패드 | 프레임마다 `poll_pad()` → `PadEvent::{Lane{lane,dir,press}, Control(action)}` | `Stage::handle_pad` — **`PlayState` 만 구현** | `lib.rs:918-921`, `R:apps/rbms-player/src/gamepad.rs:302-305`, `stage/play/mod.rs:744-750`, `stage/mod.rs:145-148` |
| IME | 처리 없음(`WindowEvent::Ime` 매치 없음, grep 0건) | — | `lib.rs:1026-1067` |

### 7.2 클릭 판정 구조

- 즉시 모드: 매 프레임 `shared.hot: Vec<(Rect, Hot)>` 를 비우고(`lib.rs:932`) 화면이 그리면서 채운다. 클릭 시 `hit_test` 가 뒤에서부터(위에 그려진 것 우선) 찾는다(`app_play.rs:577-580`).
- `Hot` 은 닫힌 열거 14종: `SelectRow, RecordRow, SettingTab, SettingRow, RivalRow, RankingRow, ModalClose, ModalReplay, NavSearch, NavSort, NavFolders, NavTables, NavRecords, NavSettings` (`lib.rs:419-434`).
- 스킨 핫스팟: select 화면만 소비한다. `select_hotspots()` 가 문서의 핫스팟을 캔버스 좌표로 받아 `SkinHotAction` 9종을 `Hot` 으로 사상(`skin_screen.rs:829-839`, `stage/select/mod.rs:842-855`). 문서가 `list` 나 `topbar` 블록을 대체했을 때만 반환한다(`skin_screen.rs:830-833`).
- `handle_mouse` 구현 화면은 Select 와 Settings 둘뿐(`stage/select/mod.rs:739`, `stage/settings.rs:634`).

### 7.3 레퍼런스의 마우스·텍스트 계약

- 매 프레임(ms 단위 틱) `current.input()` 뒤에 `skin.mousePressed(state, button, x, y)` 와 `skin.mouseDragged(...)` 를 부른다(`B:MainController.java:495-508`).
- `Skin.mousePressed`: 객체 배열을 **뒤에서 앞으로**(위에 그려진 것 먼저) 돌며, 편집 가능한 `SkinText` 의 입력 영역이면 텍스트 입력 포커스, 아니면 `obj.draw && obj.mousePressed(...)` 가 true 인 첫 객체에서 멈춘다(`B:skin/Skin.java:394-411`). 텍스트 입력 중 영역 밖 클릭은 커밋(`B:skin/Skin.java:395-397`).
- `Skin.mouseDragged`: `SkinSlider` 객체에만 전달(`B:skin/Skin.java:413-419`).
- 휠: select 는 곡 바 스크롤(`B:select/MusicSelectInputProcessor.java:155-156`), skin config 는 커스텀 옵션 오프셋(`B:config/SkinConfiguration.java:69-76`), result 는 랭킹 오프셋(`B:result/AbstractResult.java:247-254`, `mov` 의 원천은 미확인).
- 플레이 중 5초간 마우스가 안 움직이면 커서 grab(`B:MainController.java:510-515`).
- 화면 모드 전환 키(`SWITCH_SCREEN_MODE`)로 풀스크린/창 전환(`B:MainController.java:522-531`).
- 스킨 이벤트 실행: `MainState.executeEvent(id, arg1, arg2)` — 커스텀 이벤트 id 면 `skin.executeCustomEvent`, 내장이면 `EventType.event.exec` (`B:MainState.java:82-106`). SkinConfiguration 은 이를 오버라이드해 `BUTTON_CHANGE_SKIN`, 커스터마이즈 버튼, 스킨 타입 선택 id 를 처리(`B:config/SkinConfiguration.java:114-149`).

### 7.4 레퍼런스 select 의 키 계약(스킨 패널과 직결)

`B:select/MusicSelectInputProcessor.java`:

- NUM0 검색 팝업(71-83), NUM1 모드 필터(86-88), NUM2 정렬(90-92), NUM3 LN 모드(94-96), NUM4 또는 NEXT_REPLAY 리플레이 슬롯 순환(109-113).
- START 만 누름 → `setPanelState(1)` + `OPTION_OPEN` 사운드, 레인 키로 `option1p / gauge1p / optiondp / option2p / hsfix / target` 이벤트(114-204 부근).
- SELECT 만 누름 → `setPanelState(2)`, 레인 키로 customJudge / constant / judgearea / legacynote / marknote / bpmguide / nomine 토글(205-240 부근).
- NUM5 또는 START+SELECT → `setPanelState(3)`, 레인 키로 bga / gaugeautoshift / notesdisplaytiming / duration(241-285 부근).
- 아무것도 안 누름 → `setPanelState(0)`, PLAY/PRACTICE/AUTO/REPLAY 키와 폴더 열기·닫기, NUM7 라이벌, NUM8 같은 폴더, NUM9 문서 열기(286-330 부근).
- 옵션 키를 떼면 `OPTION_CLOSE`(100-107).

R-BMS select 는 화살표·Enter·문자 단축키(`/`, F2, F3, F4, Tab, O, T, R, I, F)만 받고(`stage/select/mod.rs:702-733`), 레인 키와 START/SELECT 개념이 없다. 옵션 패널은 LeftShift 홀드 또는 F1 고정(`app_options.rs:89-92`).

### 7.5 붙일 지점 제안

| 기능 | 붙일 곳 | 내용 |
| --- | --- | --- |
| 객체 클릭(`act`/click 이벤트) | `lib.rs:1039-1044` 에서 버튼 종류를 넘기고, `Stage::handle_mouse` 에 `button` 인자 추가 → 공통 함수 `AppShared::skin_click(screen, state, extra, at, button) -> Option<(event_id, arg)>` 를 `skin_screen.rs` 에 신설 | `skin_hotspots()`(`skin_screen.rs:708-726`)와 같은 방식으로 프레임을 조립해 "그려진 객체를 역순으로" 판정. 결과는 화면별 `execute_event(id, arg1, arg2)` 로 전달. 현재 `Hot` 열거를 거치는 사상(`stage/select/mod.rs:843-853`)은 닫힌 집합이라 확장 불가 |
| 슬라이더 드래그 | `MouseInput { Released }` 와 "버튼이 눌린 상태의 `CursorMoved`" 를 `StageHandler::handle_mouse_drag(at)` 로 전달 | `AppShared` 에 `mouse_down: Option<MouseButton>` 추가. 레퍼런스처럼 슬라이더 객체에만 전달 |
| 휠 | `WindowEvent::MouseWheel` → `StageHandler::handle_scroll(delta)` | select 는 곡 바 이동, skin select 는 옵션 오프셋, result 는 랭킹 오프셋 |
| 호버 | 이미 `SkinDraw.mouse` 로 전달 중(`skin_screen.rs:694`) | 추가 작업 없음. 다만 논리 좌표가 1280x720 기준이므로 해상도 변경 시 `document_cursor` 인자만 따라가면 됨 |
| 텍스트 입력 | `KeyInput.text`(`lib.rs:1053`) + 기존 `TextEdit`(`R:apps/rbms-player/src/textedit.rs:24-147`: insert/backspace/delete/left/right/home/end/paste/window) 재사용 | 스킨의 편집 가능 텍스트 객체가 포커스를 잡으면 `holds_keys()` 가 true 를 돌려 옵션 오버레이 키를 막는 기존 계약(`stage/mod.rs:160-166`)을 그대로 사용. 일본어·한국어 검색이 필요하면 `window.set_ime_allowed(true)` 와 `WindowEvent::Ime` 처리 추가 필요 |
| 게임패드/레인 키로 select 조작 | `SelectState::handle_pad` 신설, 키보드 레인 키는 `shared.lane_for(code)`(`app_input.rs:99-101`) 재사용 | START/SELECT 바인딩이 `keyconfig` 에 있는지는 미확인(`keyconfig.rs` 미열람) |

---

## 8. 내장 UI 와 스킨 문서의 혼합 합성 — 걸린 코드 전부

### 8.1 개념

- 문서 헤더 `composition`: `replace`(문서가 화면 전체), `overlay`(내장 화면 위에 덧그림), `layered`(문서 background → 내장 → 문서 foreground). 기본 번들은 decide 만 `replace`, 나머지는 전부 `layered` + `replace: [...]` 블록 목록(`R:assets/skins/steel-neon-v3/*.json5:7-8`).
- `replace` 블록 목록은 "내장 출력 중 이 이름의 덩어리는 문서가 대신 그린다" 는 선언이고, 문서가 필요한 객체를 실제로 컴파일했을 때만 내장 쪽이 비켜선다(`skin_screen.rs:161-214, 322-367`).

### 8.2 앱 쪽 코드(제거·수정 대상)

| 파일 | 줄 | 내용 | 처리 |
| --- | --- | --- | --- |
| `apps/rbms-player/src/skin_screen.rs` | 130-140 | `BuiltScreen.content: ScreenContent` | 제거 |
| 〃 | 142-159 | `ScreenKind`, `screen_kind()` | 제거 |
| 〃 | 161-214 | `Replacement` 구조체와 `met_by` | 제거 |
| 〃 | 216-292 | `replacement(screen, name)` 대체 표(play 8, select 4, result 9) | 제거 |
| 〃 | 294-320 | `OPTION_PANEL_IDS` | 제거 |
| 〃 | 322-367 | `block_replaced`, `screen_content_of` | 제거 |
| 〃 | 518-524 | 컴파일 뒤 대체 미충족 경고와 `content` 저장 | 제거 |
| 〃 | 527-531 | `SkinScreens::content` | 제거 |
| 〃 | 595-626 | `skin_uses_overlay`, `skin_uses_native_layout`, `skin_uses_layered_layout`, `screen_content` | 제거 |
| 〃 | 654-656 | `skin_draws_background` | 제거(문서가 항상 배경 담당) |
| 〃 | 746-748, 790-792, 851-853 | overlay 가 아니면 `clear(BLACK)` 분기 | 무조건 clear 로 단순화 |
| 〃 | 753-771, 797-821, 857-878 | `draw_*_skin_layer` 3종 | 제거 |
| 〃 | 829-839 | `select_hotspots` 의 `content.list/topbar` 게이트 | 게이트 제거, 일반 클릭 판정으로 교체(§7.5) |
| `apps/rbms-player/src/stage/play/mod.rs` | 805-822 | `overlay/layered/native_layout/built_in_background` 계산과 BGA 슬롯 3분기 | 문서 텍스처 경로 하나로 |
| 〃 | 872-874 | `!native_layout && draw_document(...)` 조기 반환 | 유일 경로로 |
| 〃 | 875-904 | `content` 조회, layered background, 내장 `render_playfield_on_background`, `render_lane_cover`, `render_key_bomb`, `render_hud_with_content`, foreground/overlay 재그리기 | 제거(키 봄·분석 오버레이의 귀속은 §8.5) |
| 〃 | 593, 643-646 | `draw_document(..., layer: Option<SkinLayer>)` | `layer` 인자 제거 |
| `apps/rbms-player/src/stage/select/mod.rs` | 787-804 | `overlay/layered/native_layout` 과 커버 슬롯 3분기 | 단일 경로 |
| 〃 | 808-810 | `!native_layout && draw_select_skin` 조기 반환 | 유일 경로 |
| 〃 | 811-826 | `content`, layered 분기, `render_select_on_background_with_content`, `render_select` | 제거 |
| 〃 | 827-855 | 내장 hot 사상과 스킨 핫스팟 사상 | 일반 이벤트로 교체 |
| 〃 | 856-862 | 내장 랭킹 패널, 필터 패널 | §8.5 결정 필요 |
| 〃 | 863-867 | foreground/overlay 재그리기 | 제거 |
| `apps/rbms-player/src/stage/result.rs` | 119-142 | `overlay/layered/native_layout`, `content`, `render_result_on_background_with_content`, `render_result_with_palette`, IR 줄(`!content.ir`), foreground/overlay | 단일 경로 |
| `apps/rbms-player/src/app_options.rs` | 272-275 | `screen_content(...).select.options` 면 내장 패널 생략 | 내장 패널 자체의 존폐에 따라 정리. `draw` 전체(272-307)가 내장 UI |
| `apps/rbms-player/src/assets.rs` | 305-318 | `skin_document_is_enabled` 프리셋 게이트 | 단순화 |
| `apps/rbms-player/src/app_input.rs` | 282-329 | `rebuild_skin` 의 `with_document_lanes`, `document_field` | 내장 필드 의존 제거와 함께 재설계 |
| `apps/rbms-player/src/stage/loading.rs` | 482-509 | 문서가 못 그리면 내장 진행 막대 | 폴백 정책에 따라 |
| `apps/rbms-player/src/stage/keyconfig.rs` | 187-254 | 문서가 못 그리면 내장 행 목록 | 〃 |

### 8.3 라이브러리 쪽 연관 심볼(다른 조사 범위, 영향 파악용)

패턴 `SkinComposition|ScreenContent|PlayContent|SelectContent|ResultContent|replace_names|draw_layer|SkinLayer|_with_content|SkinHotAction|SkinHotspot|hotspot` 의 파일별 출현 수(grep 집계):

| 파일 | 출현 | 전체 줄 |
| --- | --- | --- |
| `crates/rbms-render/src/content.rs` | 7 | 57 |
| `crates/rbms-render/src/hud.rs` | 14 | 466 |
| `crates/rbms-render/src/result.rs` | 16 | 899 |
| `crates/rbms-render/src/select.rs` | 8 | 704 |
| `crates/rbms-render/src/lib.rs` | 3 | 354 |
| `crates/rbms-render/src/skin_render/mod.rs` | 22 | 425 |
| `crates/rbms-render/src/skin_render/state.rs` | 17 | 924 |
| `crates/rbms-render/src/skin_render/screen.rs` | 3 | 417 |
| `crates/rbms-render/src/skin_render/songlist.rs` | 5 | 374 |
| `crates/rbms-render/src/skin_render/object.rs` | 2 | 850 |
| `crates/rbms-skin/src/loader.rs` | 10 | 693 |
| `crates/rbms-skin/src/model.rs` | 8 | 522 |

### 8.4 단독 소유 전환 시 반드시 정해야 하는 것

1. **문서가 아직 컴파일되지 않은 프레임**: 지금은 내장 화면이 그려진다(`skin_screen.rs:473-479`). 전환 뒤 선택지: 검은 화면 / 이전 화면 유지 / 화면 전환 자체를 컴파일 완료까지 지연(레퍼런스는 `create()` 에서 동기 로드, `B:MainController.java:275-278`).
2. **문서 로드 실패**: 지금은 내장 화면 + LOADED 행에 사유(`skin_select.rs:701-703`). 전환 뒤 폴백 화면이 없으면 조작 불능이 된다.
3. **스킨이 그리지 않는 R-BMS 고유 UI**: 아래 §8.5.

### 8.5 스킨 계약에 대응물이 없는 R-BMS 고유 UI

| UI | 위치 | 선택지 |
| --- | --- | --- |
| 옵션 오버레이(11행) | `app_options.rs:272-307` | 스킨의 패널 1~3 으로 대체하고 폐지 / 스킨 위 시스템 오버레이로 유지 |
| 기록 모달(R 키) | `stage/select/mod.rs:316-361`, 렌더는 `render_select` 내부 | 폐지(스킨의 리플레이 슬롯으로 대체) / 시스템 오버레이 |
| IR 랭킹 패널(I 키) | `stage/select/mod.rs:363-447, 856-859` | 스킨의 랭킹 객체로 대체 / 시스템 오버레이 |
| 필터 패널(F2) | `stage/select/filter.rs`, `stage/select/mod.rs:860-862` | 스킨의 모드 필터 등으로 일부 대체 / 시스템 오버레이 |
| 검색 상자(`/`) | `stage/select/mod.rs:208-247` | 스킨의 편집 텍스트로 대체 / 시스템 오버레이 |
| 코스 탭(Shift+Tab) | `stage/select/mod.rs:709-712`, `scene.rs:218-254` | 레퍼런스처럼 곡 목록 안의 코스 폴더/바 로 통합 |
| 리플레이 분석 오버레이 | `stage/play/mod.rs:661-688, 897-899` | 시스템 오버레이로 유지(스킨 계약에 없음) |
| 키 봄 | `render_key_bomb`, `stage/play/mod.rs:895` (결정 D16 "항상 네이티브", `docs/HANDOFF.md:42`) | 스킨의 bomb 타이머 객체로 대체하고 폐지 |
| 토스트, 서버 연결 점, 디버그 패널 | `lib.rs:948-976` (좌표가 `CW` 기준, 954) | 시스템 오버레이로 유지. 좌표계만 새 논리 해상도에 맞춤 |
| Settings / Tables / Folders / Practice / Loading(scan·table) 화면 | 각 `stage/*.rs` | 스킨과 무관한 내장 화면으로 유지할지, 시각적으로 스킨과 어울리게 다시 그릴지 결정 필요(§12) |

### 8.6 영향받는 테스트

| 파일 | 줄 수 | 테스트 수(이름 기준) | 성격 | 예상 처리 |
| --- | --- | --- | --- | --- |
| `apps/rbms-player/src/stage/render_tests_skin.rs` | 412 | 8 | overlay 가 브라우저와 hot 을 유지, 빈 overlay 는 픽셀 불변, 문서 읽는 동안 내장 화면, 기본 번들 캡처 9장 | 대부분 폐기 또는 재작성 |
| `.../render_tests_skin_v3_select.rs` | 316 | 10 | v3 select 문서 무경고 컴파일, "모든 블록 대체", 휠·클릭, 옵션 패널 대체, 테마 배치 일치 | 번들 교체와 함께 폐기 |
| `.../render_tests_skin_v3_decide_result.rs` | 212 | 7 | v3 decide/result 문서 | 폐기 |
| `.../render_tests_skin_v3_play_sp.rs` | 418 | 12 | v3 5/7/9키 문서, PLAY SIDE·GRAPH POSITION·BGA SIZE 행 | 폐기 |
| `.../render_tests_skin_v3_play_dp.rs` | 281 | 6 | v3 10/14키 문서 | 폐기 |
| `.../stage/select/tests.rs` | 657 | (일부) | `write_blocks_document`(476-517), `blocks_app`(536-), 옵션 대체 유무 비교(644) | 해당 부분 폐기 |
| `.../stage/play/tests.rs` | 792 | (패턴 4회) | 플레이 문서와 내장 필드 연동 | 해당 부분 수정 |
| `.../stage/render_tests.rs` | 341 | 약 16 | "모든 Stage 가 프레임을 그린다", "두 Stage 가 같은 프레임이 아니다", hot 영역, 배경 슬롯 | 내장 화면 폴백 유지 여부에 따라 |
| `.../stage/render_tests_play.rs`, `render_tests_result.rs`, `render_tests_select.rs`, `render_tests_shell.rs` | 157 / 74 / 85 / 193 | — | 내장 화면 스냅샷 | 내장 화면 존폐에 따라 |
| `.../skin_screen/tests.rs` | 139 | 5 | 커서 좌표 변환, Lua 예산, 디스크 미접근, 오프셋 병합 | 유지(좌표 상수만 수정) |
| `.../skin_select/tests.rs`, `fixtures.rs` | 444 / 101 | — | SKIN 탭 행·번들 스코프 | `.luaskin` 추가에 맞춰 보강 |
| `.../stage/settings/skin_tests.rs` | 246 | 6 | SKIN 탭 행 추가·저장·RESET | 유지·보강 |
| `apps/rbms-player/src/assets.rs` 내 `mod tests` | 666-1096 | 약 12 | 세대 설치·이동·보존, `RESULT_REPLACEMENT_CONTRACT`(1013-1018) | 설치 방식 교체와 함께 대부분 재작성 |
| `apps/rbms-player/src/app_options.rs` 내 tests | 309-629 | 약 20 | 오버레이 키·그리기 | 오버레이 존폐에 따라 |
| `crates/rbms-render/tests/golden_*.rs`, `skin_render.rs`, `crates/rbms-skin/tests/skin_nested.rs`, `skin_model.rs` | — | — | 내장 화면 골든, 문서 대체 계약 | 다른 조사 범위. 영향 큼 |

참고: 저장소 문서 기준 전체 테스트는 3,181개(`docs/HANDOFF.md:19`). 헤드리스 캔버스 크기는 테스트 전반에서 `CW`/`CH` 를 쓴다(§9.2 표).

---

## 9. 창 크기·해상도·풀스크린

### 9.1 현재 동작

- 논리 화면 상수 `CW = 1280`, `CH = 720` (`lib.rs:134-135`).
- 창 생성: `Window::default_attributes().with_title("rbms").with_inner_size(LogicalSize::new(CW, CH))` (`lib.rs:997`). 크기 조절 가능 여부·최소 크기·풀스크린 지정 없음(`with_resizable`/`Fullscreen`/`set_fullscreen` grep 0건).
- 렌더러의 `size()` 는 항상 `(CW, CH)` 를 돌려준다(`gpu/mod.rs:524-527`). 셰이더 유니폼에도 `[CW, CH]` 를 한 번 써 넣는다(`gpu/mod.rs:198`). 서피스는 실제 창 픽셀 크기로 구성되고(`gpu/mod.rs:180-190`), 리사이즈 시 재구성(`gpu/mod.rs:515-521`, `lib.rs:1029-1033`).
- 따라서 모든 그리기는 1280x720 논리 좌표로 하고, GPU 가 창 전체로 늘린다. LETTERBOX 설정(`DisplayOptions.letterbox`, `schema.rs:630-631`)이 켜지면 16:9 최대 사각형에 맞추고 여백을 둔다(`surface_viewport`, `gpu/mod.rs:612-620`). 적용은 매 프레임 오버레이 단계(`lib.rs:949`).
- 커서는 같은 변환의 역으로 논리 좌표가 된다(`gpu/mod.rs:624-627`).
- 표시 모드: `PresentMode::AutoVsync` 고정(`gpu/mod.rs:185`), 이벤트 루프는 `ControlFlow::Poll`(`lib.rs:1214`).
- 설정 스키마에 해상도·풀스크린·vsync·FPS 상한 항목이 없다(`DisplayOptions`, `schema.rs:614-635`).
- 스킨 문서는 저작 크기에서 캔버스 크기로 `SkinViewport` 변환을 거친다(`app_input.rs:308` 에서 `(CW, CH)` 로 생성). 즉 1920x1080 문서는 지금도 1280x720 논리 좌표로 줄여 그린 뒤 창 크기로 다시 늘린다. 텍스처 쿼드는 GPU 가 최종 해상도에서 샘플링하므로 선명도 손실이 작지만, 텍스트 래스터가 어느 해상도에서 만들어지는지는 렌더 조사 범위(미확인). `docs/architecture.md:52` 는 텍스트도 `fill_rect` 로 내려간다고 적고 있어 1280x720 격자 기준일 가능성이 있다.

### 9.2 1280x720 전제가 박힌 위치

| 파일 | `CW`/`CH` 출현 | 용도 |
| --- | --- | --- |
| `apps/rbms-player/src/lib.rs` | 5 | 상수 정의, 기본 `Skin::default_for(MODE, CW, CH)`(776), 서버 연결 점 좌표(954), 창 크기(997) |
| `apps/rbms-player/src/gpu/mod.rs` | 21 | 유니폼, `size()`, 뷰포트·커서 변환, 시저(432) |
| `apps/rbms-player/src/app_input.rs` | 2 | `Skin::build(&cfg, mode, CW, CH)`(288), `SkinViewport`(308) |
| `apps/rbms-player/src/app_options.rs` | 7 | 내장 옵션 패널 위치(279) 외 테스트 |
| `apps/rbms-player/src/stage/play/mod.rs` | 6 | 분석 오버레이 좌표(666-685) |
| `apps/rbms-player/src/stage/loading.rs` | 2 | 내장 진행 막대 중심(487-488) |
| `apps/rbms-player/src/stage/{keyconfig, course_result, practice, tables, folders, result}.rs` | 각 1 | 내장 패널 가로 중앙 정렬 |
| `apps/rbms-player/src/app_play.rs`, `settings_ui.rs` | 각 1 | (용도 미확인, grep 집계만) |
| 테스트 파일 9개 | 3~11 | 헤드리스 캔버스 크기 |
| `apps/rbms-player/src/gpu/batch.rs` | — | 테스트 상수 `LOGICAL = (1280, 720)`(233) |
| `apps/rbms-player/src/settings_view.rs` | 0 | 자체 픽셀 상수로 배치(`ROW_PITCH 50.0` 등, 19-41). 암묵적으로 1280x720 전제 |

### 9.3 레퍼런스

- 해상도는 설정값 `config.getResolution()` 이고(`B:MainController.java:427` 등), 스킨은 저작 크기에서 그 해상도로 변환한다(`B:skin/Skin.java:386-391` 의 transform 설정, 세부는 미확인).
- 풀스크린/창 전환은 키 명령으로(`B:MainController.java:522-531`).

### 9.4 필요한 변경 요지

- 논리 해상도를 상수에서 "선택된 스킨의 저작 크기 또는 설정 해상도" 로 바꾸거나, 최소한 16:9 1920x1080 으로 올린다. `Renderer::size()` 가 `(CW, CH)` 를 돌려주는 구조라 `Gpu` 에 가변 논리 크기를 넣고 유니폼을 갱신하는 식이 최소 변경이다.
- 설정에 해상도·창 모드(창/테두리 없는 전체/독점 전체)·vsync 항목 추가(`schema.rs` `DisplayOptions` + `rbms_config::SETTINGS` 표).
- 내장 화면을 남긴다면 그 좌표 상수들이 전부 1280x720 기준이므로 스케일 계수를 두거나 내장 화면 전용 뷰포트를 둔다.

---

## 10. 시스템 사운드·BGA·폰트

### 10.1 시스템 사운드

- 22 stem, 레퍼런스 `SoundType` 순서와 파일명을 따른다(`R:apps/rbms-player/src/syssound.rs:57-138`): `scratch, f-open, f-close, o-change, o-open, o-close, playready, playstop, clear, fail, resultclose, course_clear, course_fail, course_close, guide-pg, guide-gr, guide-gd, guide-bd, guide-pr, guide-ms, select, decide`.
- 폴더 해석 순서: (1) `config.audio.sound_folder`(공백 제거 후 비어 있지 않으면) (2) 프리셋이 `STEEL NEON` 일 때 활성 번들의 `sound/` (3) 없음 = 무음(`lib.rs:218-223`, `assets.rs:321-327`, `R:crates/rbms-config/src/audio.rs:63, 121`).
- 파일 해석: `resolve_file(dir, stem, ["wav","ogg","flac","mp3"])` — stem 그대로의 파일 또는 stem + 확장자(`syssound.rs:33, 212-214`, `assets.rs:472-487`). `f-close2.ogg`, `scratch2.ogg` 같은 변형 파일은 읽지 않는다.
- 로드: 시작 시 동기 디코드(`lib.rs:724-728`, `syssound.rs:244-263`). 스트림 재개방 시 뱅크에 다시 설치(`app_play.rs:607-612`).
- 재생: `play_on(Bus::System, id, gain, pan 0, pitch 1, at 0)` 1회(`syssound.rs:321-326`). **루프·정지 API 가 없다.** 샘플 id 네임스페이스 `0x0090_0000` 길이 `0x100`(`syssound.rs:26`).
- 가이드음 6종은 `guide_se` 설정이 켜져 있을 때만(`syssound.rs:307-318`).
- 호출 지점: select 결정(`stage/select/mod.rs:147, 173`), 폴더 열기/닫기(152, 556), 옵션 열기/변경/닫기(`app_options.rs:198, 239, 251`), play ready(`stage/play/mod.rs:695`), play stop(489), 판정 가이드(556), decide(`stage/loading.rs:463`), result clear/fail/close(`stage/result.rs:90, 96`), course clear/fail/close(`stage/course_result.rs:91, 96`).

레퍼런스와의 차이:

- `SELECT`/`DECIDE` 는 BGM 계열이다(R-BMS 주석도 인정, `syssound.rs:173-179`). select 는 프리뷰 기본음으로 계속 재생(`B:select/MusicSelector.java:173-174, 187-189`), decide 는 장면 진입 시(`B:decide/MusicDecide.java:32-35`). R-BMS 는 둘 다 전환 순간의 1회 효과음으로 쓴다.
- result 는 설정에 따라 루프(`B:result/MusicResult.java:150-151`), fadeout 시작에 clear/fail 을 멈추고 `RESULT_CLOSE` 재생(`B:result/MusicResult.java:252-256`), 상태를 떠날 때 전부 정지(154-158). `MainState.play(sound, loop)` / `stop(sound)` (`B:MainState.java:162-172`).
- select 진입 시 `getSoundManager().shuffle()` (`B:select/MusicSelector.java:157`). 변형 파일 중 무작위 선택으로 보이나 `SystemSoundManager` 본문은 미확인.
- 스크롤 시 `SCRATCH` (`B:select/MusicSelectInputProcessor.java` 스크롤 루프). R-BMS select 는 커서 이동 시 사운드 없음(`stage/select/mod.rs:721-730`).

ModernChic 쪽 실측:

- `M:Sound/` 13개 ogg: `clear, f-close, f-close2, f-open, fail, o-change, o-close, o-open, playready, playstop, scratch, scratch2, screenshot`. select/decide BGM, guide, resultclose, course 계열은 없다.
- 스킨 자체 음원: `M:Root/sounds/`(change, click, close, enter, favorite, fullcombo, open, section, section-win, section-lose, `vo/` 등), `M:Select/sounds/`(open, close), 그리고 `M:Root/customsound.lua`. 스킨이 Lua 에서 직접 소리를 내는 계약이 있다는 뜻이며(세부는 스킨 본체 조사 범위), 앱 쪽에는 "스킨 발 재생"을 받을 버스·id 네임스페이스·로더가 없다.

### 10.2 BGA 와 동영상

- 차트 BGA: `bga_jobs` 가 `#BMP` 이름을 `["png","bmp","jpg","jpeg"]` 로만 해석(`assets.rs:633-651`), `image` 크레이트로 디코드(`assets.rs:654-659`). 앱 의존성의 `image` 기능도 `png, bmp, jpeg` 뿐(`R:apps/rbms-player/Cargo.toml:35`).
- 로드 조건: `config.display.bga && self.skin.bga.is_some()` (`app_play.rs:410`) — **내장 RON 필드에 BGA 사각형이 있어야** 디코드를 시작한다. 스킨 단독 소유 뒤에는 "문서가 bga 객체를 가졌는가" 로 바꿔야 한다.
- 표시: 현재 프레임 1장을 `background_texture` 로 올려 문서의 배경/bga 객체에 넘김(`stage/play/mod.rs:809-822`, `stage/canvas.rs:152-163`). 세대 번호로 재업로드를 피한다(`assets.rs:563-582`).
- 동영상: 워크스페이스 어디에도 영상 디코더 의존성이 없다(`Cargo.toml` 전체 grep 에서 `symphonia` 의 `isomp4`(오디오 컨테이너) 한 건뿐, `R:crates/rbms-audio/Cargo.toml:11`). 저장소 문서도 "비디오 BGA" 를 후속 과제로 적는다(`docs/HANDOFF.md:85`).
- ModernChic mp4 8개: `M:Select/bg/movie/BGmovie01.mp4`(21MB), `BGmovie02.mp4`(25MB), `M:Decide/bg/movie/sample.mp4`(6MB), `sample2.mp4`(13MB), `M:Play/parts/common/BGA/movie/#default.mp4`(20MB), `cyber.mp4`(45MB), `NOSTALGIC.mp4`(48MB), `travel.mp4`(24MB). 이들이 필수 요소인지 옵션(정지 이미지 대안 존재)인지는 스킨 본체 조사 범위(미확인). "똑같이" 가 목표면 스킨 소스 동영상 재생이 필요하고, "조금 더 심플한 기본 스킨" 에서는 빼는 것이 배포 크기상 가장 큰 절감이다.

### 10.3 폰트

- UI 폰트: 실행 인자 `--font` 또는 설정 `display.font_path` 를 시작 시 읽어 `rbms_render::load_font(bytes)` → `set_ui_family`(`lib.rs:1161, 1192-1203`), 설정 화면에서 파일 선택(`rfd`)으로 교체·초기화(`app_input.rs:247-267`, `stage/settings.rs:165-171, 271-284`). 저장소 내장 폰트는 `assets/fonts/Inter-Regular.ttf`(877KB) 하나(내장 방식은 렌더 크레이트 쪽, 미확인).
- 스킨 폰트: 문서의 `fonts` 경로를 워커가 **바이트로만** 읽어(`SkinAssetKind::Font`, `assets.rs:618`) `SkinScreen::build` 에 넘긴다(`skin_screen.rs:74-79, 429`). 해석은 렌더 쪽(cosmic-text).
- 비트맵 폰트 `.fnt`: 앱·크레이트 소스에 `fnt` 처리 코드가 없다(grep 0건). ModernChic 은 `.fnt` 10개(12.8MB)와 ttf 11개(54.7MB)를 쓴다. `.fnt` 지원은 렌더·로더 쪽 신규 작업이고, 앱 쪽은 워커 디코드 작업 종류에 "fnt 본문 + 페이지 이미지" 를 추가해야 한다.
- 텍스트 캐시 통계가 디버그 패널에 노출됨(`lib.rs:968-969`).

---

## 11. 그 밖에 앱 배선에서 발견한 구조적 격차

1. **장면 시계가 화면 전환에 묶여 있지 않다.** `skin_now_ms` 는 앱 시계이고(`skin_screen.rs:567-569`) `start_play()` 가 그 시계를 0 으로 되돌린다(`app_play.rs:674`). select 에서 켜 둔 타이머(예: `SONGBAR_CHANGE`, `PANEL1_OFF`)는 play 진입 뒤 "미래 시각" 이 되고, play 에서 켠 타이머는 result/select 로 넘어가도 남는다. 레퍼런스는 전환마다 타이머 전부 OFF + 시계 0(`B:TimerManager.java:100-106`). ModernChic 의 모든 진입 애니메이션(타이머 없는 dst)이 이 전제에 의존한다.
2. **컴파일된 화면이 영구 상주한다**(§2.2-8). 레퍼런스식 "상태 진입 시 로드, 이탈 시 해제" 로 바꾸면 메모리는 줄지만 매 전환마다 로드 비용이 생기므로 비동기 로드 + 전환 지연 설계가 필요하다.
3. **문서 로드(`load_skin`)가 프레임 루프에서 동기 실행된다**(`skin_select.rs:695`). JSON 한 장은 견디지만 Lua 수천 줄 실행은 프레임 정지를 만든다.
4. **`Skin`(내장 RON 필드)이 플레이 로직에 박혀 있다.** BGA 로드 조건(`app_play.rs:410`), 레인→타이머 키 번호(`stage/play/mod.rs:265-286`), 판정 측(`field_side`, 243-248), 키 봄 지속 시간(`skin.bomb_us`, 280), 결과 팔레트(`ResultPalette::from_skin`, `app_input.rs:287`)가 모두 내장 `Skin`/`SkinConfig` 를 읽는다. 스킨 단독 소유 뒤에도 "레인 순서·1P/2P 측" 은 모드 데이터에서 직접 계산해야 한다.
5. **옵션 변경이 스킨 재빌드를 부른다.** 옵션 오버레이를 닫을 때 `rebuild_skin()`(`app_options.rs:255-258`), 리프트 조절 때 `rebuild_skin()`(`app_input.rs:187-195`). 내장 필드 전제의 잔재다.
6. **IR 제출에 스킨 이름이 실린다**: `skin: shared.config.display.skin.clone()` (`app_result.rs:198`). 프리셋 행을 없애면 이 값의 출처를 바꿔야 한다.
7. **스킨이 디스크에 쓴다.** ModernChic 은 `M:io/Play/sp/log/*.txt`, `M:io/Play/**/lanecover/*.txt`, `M:History/recent.txt` 등을 보유한다. R-BMS 샌드박스는 `io` 를 제거한 상태(작업 지시서의 확인 사실). 쓰기 허용 범위와 위치(읽기 전용 설치 위치일 때의 리다이렉트)를 정해야 한다.
8. **decide 문서가 스캔·테이블 받기 화면에도 그려진다**(`stage/loading.rs:475-485`). ModernChic decide 는 곡 정보 연출이므로 곡이 없는 대기 화면에 쓰면 빈 연출이 나온다.
9. **코스**: R-BMS 는 스테이지마다 Loading 을 거치고(`lib.rs:245-255`), 코스 결과는 내장 화면이다. 레퍼런스는 스테이지 사이에 RESULT → PLAY 로 직행하고(`B:result/MusicResult.java:200-208`) 마지막에 COURSERESULT 스킨을 쓴다.
10. **`MODE` 고정 인자**: 헤더 스캔과 비플레이 화면 로드에 7키를 넘긴다(`skin_select.rs:693, 749`). play 문서가 모드에 따라 헤더가 달라지는 경우의 영향은 로더 조사 범위(미확인).

---

## 12. 권고: 변경 단위(파일별, 크기, 의존 순서)

크기 기준: S = 한 파일 100줄 이내, M = 여러 파일 수백 줄, L = 구조 변경 또는 1,000줄 이상(테스트 포함).

| 순서 | 단위 | 주요 파일 | 크기 | 선행 | 내용 |
| --- | --- | --- | --- | --- | --- |
| A1 | 장면 시계·타이머 수명 | `lib.rs`(`App::switch`), `skin_screen.rs`(`skin_now_ms`), `app_play.rs:673-679` | S | 없음 | 전환마다 `skin_timers.clear()` + 장면 시작 시각 기록. `skin_now_ms` 를 "장면 진입 후 ms" 로. 곡 시계(`clock`)와 분리 |
| A2 | 논리 해상도 가변화 | `gpu/mod.rs`, `lib.rs:134-135, 997`, `stage/canvas.rs`, `schema.rs`(`DisplayOptions`), `rbms-config` 설정 표 | M | 없음 | `Gpu` 에 논리 크기 필드, 유니폼 갱신, 커서 변환. 해상도·창 모드·vsync 설정 행 추가 |
| A3 | 입력 이벤트 확장 | `lib.rs:1026-1067`, `stage/mod.rs`(`StageHandler`), `AppShared` | M | 없음 | 버튼 종류, release, drag, wheel, (선택) IME. 기본 구현은 no-op |
| B1 | `.luaskin` 탐색·헤더·타입 확장 | `skin_select.rs:39, 727-762`, `rbms-skin/loader.rs:221-242` | M | 로더의 Lua 지원(타 조사 범위) | 확장자, 타입 9·15, 헤더 캐시 |
| B2 | 문서 로드 비동기화와 수명 정책 | `skin_select.rs:682-705`, `skin_screen.rs:462-552` | M | B1 | 워커에서 `load_skin`, 상태 이탈 시 해제 여부 결정 |
| B3 | 스킨 팩 선택 모델 | `schema.rs:247-399`, `skin_select.rs`, `settings_ui.rs`, `stage/settings.rs` | M | B1 | 폴더 하나로 모든 타입 지정, 프리셋 행(`display.skin`) 제거, 설정 마이그레이션(스키마 버전 3) |
| B4 | 기본 스킨 배포·설치 교체 | `assets.rs:26-422` 전부, `lib.rs:1125-1134`, `.github/workflows/release.yml`, `app_play.rs:355-367`, `lib.rs:218-223` | L | B3, 배포 방식 결정 | `include_bytes!` 표 제거, 디렉터리 기반 탐색/복사, 폴백 정책, 이전 세대(steel-neon*) 선택의 마이그레이션 |
| C1 | 혼합 합성 제거(§8.2 표 전체) | `skin_screen.rs`, `stage/play/mod.rs`, `stage/select/mod.rs`, `stage/result.rs`, `app_options.rs`, `app_input.rs` + 렌더·스킨 크레이트 대응부 | L | A1, B2, 폴백 정책 결정 | `ScreenContent`/`Replacement`/layer 경로 삭제, 단일 그리기 경로 |
| C2 | 스킨 이벤트 실행기와 클릭 판정 | `skin_screen.rs`(신규 `skin_click`), 각 Stage 의 `execute_event` | M | A3, C1 | 객체 역순 판정, 이벤트 id → 화면 동작. `Hot` 사상 제거 |
| D1 | Decide Stage 분리 | 신규 `stage/decide.rs`, `stage/loading.rs`, `stage/select/mod.rs:142-149`, `app_play.rs:441-451` | M | A1, C1 | `input/scene/fadeout`, 키 스킵·취소, DECIDE 사운드. 로딩은 백그라운드로 병행 |
| D2 | Play 상태기계 | `stage/play/mod.rs`, `rbms-render` `PlayTimers` | L | A1, C1, D1 | PRELOAD(로딩 진행)/READY/PLAY/FAILED(close)/FINISHED(finishMargin, fadeout), `STARTINPUT/RHYTHM/MUSIC_END/ENDOFNOTE/SCORE_*` 타이머, 내장 `Skin` 의존 제거 |
| D3 | Result 장면 수명·상태 | `stage/result.rs`, `app_result.rs` | M | A1, C1 | 입력 잠금, fadeout, `UPDATESCORE` 2단계, 게이지 종류 전환, IR 상태 타이머, 랭킹 공급 |
| D4 | CourseResult 스킨화 | `stage/course_result.rs`, `course_ui.rs` | M | B1, D3 | 타입 15 문서, 코스 합산 상태 공급 |
| D5 | Select 상태 모델 확장 | `stage/select/{mod,scene,list}.rs`, `lib.rs`(`SelectItem`), `app_options.rs` | L | A1, C1, C2 | 바 종류, 램프 id, 패널 1~3 과 옵션 이벤트, 리플레이 슬롯, 배너/스테이지파일 분리, 랭킹·라이벌 공급, 검색 텍스트, 레인 키/START/SELECT 입력, select BGM |
| D6 | KeyConfig 스킨화 | `stage/keyconfig.rs`, `skin_screen.rs:895-906` | M | C1, C2 | 포커스·캡처 상태 공급, 스킨 이벤트로 조작 |
| D7 | SkinSelect Stage 신설 | 신규 `stage/skin_select.rs`, `skin_select.rs` | M | B3, C2, A3 | 타입 9 문서, 스킨 교체·커스터마이즈 버튼·슬라이더·휠, 미리보기 여부 결정 |
| E1 | 사운드 확장 | `syssound.rs`, `rbms-audio`(루프·정지), `lib.rs:218-223` | M | B3 | BGM 루프/정지, 변형 파일, 스킨 팩 사운드 폴더, 스킨 발 재생용 버스·네임스페이스 |
| E2 | 스킨 소스 동영상·`.fnt` 자산 파이프 | `assets.rs`(`SkinAssetKind`), `Cargo.toml` 의존성 | L | 렌더 지원 | 디코더 선택(의존성 추가는 사용자 확인 필요), 프레임 업로드 경로 |
| F1 | 테스트 재편(§8.6) | `stage/render_tests*.rs`, `assets.rs` tests, `select/tests.rs`, `play/tests.rs` | L | C1 이후 각 단위와 함께 | v3 번들 테스트 폐기, 새 기본 스킨 기준 스냅샷, 전이·타이머 단위 테스트 |
| F2 | 문서 갱신 | `docs/architecture.md:73-85`, `docs/development.md:83-86`, `docs/skin.md`, `docs/HANDOFF.md`, `docs/PROCESS.md` | M | 각 단위와 함께 | 혼합 합성·세대 설치 서술 교체 |

의존 요약: A1·A2·A3 은 서로 독립이며 먼저 병렬 가능. B1 → B2/B3 → B4. C1 은 A1·B2 뒤. D 계열은 C1 뒤에 화면별 병렬 가능(파일 소유가 겹치지 않음: decide/loading, play, result, course_result, select, keyconfig, skin_select). E 는 B3 뒤 병렬.

---

## 13. UI·레이아웃 관점에서 사용자에게 보일 변화

| 영역 | 지금 | 전환 뒤(권고대로일 때) |
| --- | --- | --- |
| 창 | 1280x720 창, 늘리면 그대로 확대 | 1920x1080 기준 화면, 해상도·전체 화면 설정 가능 |
| 화면 흐름 | 선택 → 로딩 막대 → 플레이 → 결과 | 선택 → 결정 연출(스킵 가능) → 플레이(로딩·READY 연출 포함) → 페이드 → 결과(입력 잠금 뒤 종료 페이드) |
| 선택 화면 | 왼쪽 목록 + 오른쪽 상세 + 아래 내비게이션 버튼(내장 레이아웃 위에 프레임 이미지) | 스킨의 곡 바 휠·배너·랭킹·옵션 패널이 화면 전체를 구성. 아래 버튼 줄(SEARCH/SORT/FOLDERS/TABLES/RECORDS/SETTINGS)은 스킨에 대응물이 없으면 사라짐 → 단축키 또는 시스템 메뉴로만 접근 |
| 옵션 변경 | Shift 홀드/F1 로 11행 패널 | 스킨의 3개 패널(플레이 옵션/어시스트/상세). 조작 키 체계가 레퍼런스식(START/SELECT + 레인 키)으로 바뀌거나 키보드 대응 키를 새로 정의 |
| 검색·필터·기록·랭킹 | 내장 상자·패널·모달 | 스킨 객체로 대체되는 것과 시스템 오버레이로 남는 것으로 나뉨(§8.5 결정 필요) |
| 플레이 화면 | 내장 필드(RON) + 문서 장식, 키 봄은 항상 내장 | 노트·판정·게이지·봄·커버 전부 스킨. PLAY SIDE/GRAPH POSITION 같은 R-BMS 번들 옵션은 사라지고 스킨이 선언한 옵션만 남음 |
| 결과 화면 | 즉시 키 입력으로 종료, IR 한 줄 | 스킨 연출 시간 뒤 입력 허용, 랭킹·이전 기록 비교·게이지 종류 전환 |
| 코스 결과·키 설정 | 내장 텍스트 목록 | 스킨 화면 |
| 스킨 설정 | SETTINGS 의 SKIN 탭(행 목록, 화면 타입별 개별 선택) | 스킨 팩 단위 선택 + 스킨의 skin select 화면(미리보기·옵션 목록·슬라이더). SKIN 탭은 팩 선택과 폴더 지정 정도로 축소 가능 |
| 설정·폴더·테이블·연습 | 내장 화면 | 스킨 계약 밖이므로 내장 화면 유지가 기본. 다만 1920x1080 기준 재배치와 기본 스킨에 맞춘 색·폰트 조정이 필요(시각적 이질감이 가장 크게 남는 부분) |
| 소리 | 전환 순간 효과음 | 선택 화면 BGM, 결과 BGM 루프, 스킨 자체 클릭음 |
| 첫 실행 | 1.8MB 번들이 `~/.config/rbms/skin/` 에 풀림 | 배포 방식에 따라: 동봉 폴더 직접 참조(복사 없음) 또는 수십~수백 MB 복사/다운로드 |

---

## 14. 위험, 미확인, 읽지 못한 파일

### 14.1 위험

1. 폴백 부재: 단독 소유 전환 뒤 스킨 로드 실패·미설치 시 조작 불능. 최소 폴백을 먼저 정해야 한다.
2. 테스트 붕괴 규모: 스킨 관련 앱 테스트 약 50개 + 내장 화면 스냅샷 + 렌더 골든이 한 번에 깨진다. 게이트(`fmt`/`clippy -D warnings`/`cargo test --workspace`)를 중간 커밋마다 유지하려면 단위 순서를 지켜야 한다.
3. 메모리: 1920x1080 텍스처 다수(png 212MB 원본)를 화면 타입별로 상주시키면 VRAM 압박. 수명 정책(B2) 필요.
4. 프레임 정지: 동기 `load_skin` + 대형 Lua.
5. 배포 크기와 `compute_build_hash` 의 전체 파일 읽기.
6. 재배포 권리 미확인(이미지·동영상·음원).
7. 저장소 명명 규칙(`docs/HANDOFF.md:50`)과 "ModernChic 을 기본 스킨으로" 지시의 충돌.
8. 스킨의 디스크 쓰기 요구와 샌드박스 정책의 충돌.
9. 키 체계 변경(START/SELECT + 레인 키)이 기존 사용자 조작과 충돌.
10. 영상 디코더 의존성 추가는 컨벤션상 사용자 확인 대상(의존성 추가 전 확인 규칙).

### 14.2 미확인 항목(본문에 표시한 것의 모음)

- 렌더 크레이트의 텍스트 래스터 해상도와 내장 폰트 로딩 방식.
- 로더가 `input/scene/fadeout/loadstart/loadend/playstart/close/finishMargin/rankTime` 을 보존하는지.
- Lua 샌드박스 객체의 스레드 이동 가능 여부.
- `B:SystemSoundManager` 의 변형 파일 선택·셔플 규칙, `B:result/CourseResult.java` 본문, `B:config/KeyConfiguration.java` 본문, `B:TimerManager` 외 타이머 세부.
- `B:result/AbstractResult.java:247-254` 의 `mov` 원천(휠인지 키인지).
- ModernChic 에서 mp4 가 필수인지 선택인지, 옵션 이름이 화면 간에 공유되는지, `Sound/` 폴더가 스킨에서 어떻게 참조되는지.
- `keyconfig.rs` 에 START/SELECT 에 해당하는 바인딩이 있는지.
- `app_play.rs`·`settings_ui.rs` 의 `CW`/`CH` 1회 출현 용도.

### 14.3 배정 범위 중 전부 읽지 못한 파일

| 파일 | 읽은 범위 | 읽지 못한 범위 |
| --- | --- | --- |
| `apps/rbms-player/src/app_play.rs` | 270-690 정독, 나머지는 함수 시그니처 grep | 1-269(차트 디코드·연습 슬라이스·시계 헬퍼), 690-1327(디버그 줄·테스트) |
| `apps/rbms-player/src/app_result.rs` | 40-340, 380-475 | 1-39, 340-379(`run_target` 등), 475-680(테스트) |
| `apps/rbms-player/src/settings_view.rs` | grep(함수·상수)만 | 본문 전체 |
| `apps/rbms-player/src/stage/select/list.rs` | 1-330 | 330-649(`arrange_songs`, 테스트) |
| `apps/rbms-player/src/stage/select/preview.rs` | 1-300 | 300-552(클립/오토플레이 프리뷰 로드, 정지) |
| `apps/rbms-player/src/stage/select/filter.rs` | 1-230 | 230-441(조정·그리기·테스트) |
| `apps/rbms-player/src/stage/select/tests.rs`, `stage/play/tests.rs` | 패턴 grep 만 | 본문 |
| `apps/rbms-player/src/stage/render_tests*.rs`, `skin_screen/tests.rs`, `skin_select/{tests,fixtures}.rs`, `stage/settings/skin_tests.rs`, `syssound/tests.rs` | 테스트 함수 이름만 | 본문 |
| `apps/rbms-player/src/gpu/mod.rs` | 108-207, 500-637 | 나머지(파이프라인 생성, `render`) — 배정 목록 밖이나 해상도 판단에 사용 |
| `B:MainController.java` | 230-570 | 1-229, 570-855 |
| `B:play/BMSPlayer.java` | 366-785 정독 + 전체 grep | 1-365, 786-1092 |
| `B:result/MusicResult.java` | 150-319 정독 + 전체 grep | 1-149, 320-498 |
| `B:config/SkinConfiguration.java` | 44-200, 485-520 + 전체 grep | 200-484, 520-669 |
| `B:select/MusicSelector.java` | 40-710 | 1-39(import) |
| `B:select/MusicSelectInputProcessor.java` | 62-330 (배정 목록 밖, 입력 계약 확인용) | 나머지 |

배정 목록에 있었으나 스킨과 무관해 본문을 열지 않은 것은 없다. `stage/tables.rs`, `stage/folders.rs` 는 배정 목록에 없어 열지 않았다.
