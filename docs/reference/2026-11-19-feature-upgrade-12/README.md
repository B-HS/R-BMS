# 조사: 기능 고도화 12건 (2026-11-19)

최종 갱신 2026-11-19 · 대응 커밋 `439a93b`(조사 시점의 `dev`), 각 절 끝의 "구현 결과"는 `b47398a`·`e62240d`·`c4e75c2`·`df80201`·`80ca8d5`·`1eb4768`·`3c546d3`·`37e8ddd`·`51f4884`(구현)와 `fe1f427` 기준. 조사자: 메인(Fable). 계획은 [plan/2026-11-19-feature-upgrade-12.md](../../plan/2026-11-19-feature-upgrade-12.md).

이 문서는 12개 요구의 현재 코드 근거를 적는다. 구현으로 대상 코드가 바뀌면 해당 절을 고친다.

조사 때의 서술(각 절의 본문)은 조사 시점(`439a93b`)의 것이라 고치지 않았다. 구현으로 달라진 점은 각 절 끝의 "구현 결과(2026-11-19)"에 적었고 줄 번호는 `fe1f427` 의 것이다. 요구 13(램프 반 쪼개기, 계획 U10)은 이 조사 범위 밖이고 구현 중이라 여기에 없다. 결정은 [acknowledge/](../../acknowledge/README.md) 의 2026-11-19 ADR 에 있다.

## 1. 블루투스 오디오

### 1.1 지금 구조

- 출력은 cpal 0.17.3(`crates/rbms-audio/Cargo.toml`). 엔진은 `crates/rbms-audio/src/engine.rs`, 장치 열거는 `host.rs`(호스트 스레드 한 곳에서 `ask` 로 묻는다).
- 플레이 시계는 오디오 시계다. `AppShared::audio_clocks_at`(`apps/rbms-player/src/app_play.rs:1012`)가 `AudioEngine::clocks(now)` 의 `audible_us`·`lookahead_us` 를 쓴다. `audible_us` 는 콜백이 낸 `playback_ahead`(콜백 시각에서 DAC 까지의 앞섬)로 보간한다(`engine.rs:710` `audible_us_from`).
- `playback_ahead` 는 cpal 의 `OutputStreamTimestamp { callback, playback }` 차이다. 200 ms 를 넘으면 백엔드 오류로 보고 버퍼 길이로 대신한다(`MAX_PLAYBACK_AHEAD_MS`, `engine.rs:61`, `ahead_or_fallback` `engine.rs:739`).
- cpal 0.17.3 의 `playback` 은 버퍼 길이만 더한다. CoreAudio: `cpal-0.17.3/src/host/coreaudio/macos/device.rs:898-908`(`delay = frames_to_duration(latency_frames = device_buffer_frames)`), WASAPI: `src/host/wasapi/stream.rs:586-599`(TODO 주석으로 추정치라고 적혀 있다). 장치 지연(`kAudioDevicePropertyLatency`, 안전 오프셋, 블루투스 구간)은 들어 있지 않다.
- 따라서 블루투스 출력에서는 `audible_us` 가 실제로 들리는 위치보다 장치 지연만큼 앞선다. 지금 사용자가 할 수 있는 보정은 JUDGE TIMING(`config.judge.offset_ms`, 범위 ±200 ms `crates/rbms-config/src/schema.rs:201-204`, 자동 조정 `judge.auto_offset`)뿐이다.
- 입력 장치(마이크)는 열지 않는다(`rbms-audio` 에 입력 스트림 코드 없음). 블루투스가 통화 프로필(HFP)로 떨어지는 원인은 없다.

### 1.2 이 Mac 에서 잰 값 (2026-11-19, 임시 프로그램)

| 장치 | transport | `kAudioDevicePropertyLatency` | 안전 오프셋 | 버퍼 |
| --- | --- | --- | --- | --- |
| AirPods Pro 2 | `blue` | 7,680 프레임(48 kHz 면 160 ms) | 0 | 512 |
| MacBook Pro 스피커 | `bltn` | 60 프레임 | 48 | 512 |

- 같은 값을 `unsafe` 없이 읽을 수 있다: `coreaudio::audio_unit::macos_helpers::audio_unit_from_device_id(id, false)` 로 만든 AUHAL 에 `AudioUnit::get_property::<u32>(selector, Scope::Output, Element::Output)`. 직접 `AudioObjectGetPropertyData` 로 읽은 값과 같았다(지연 7680, 안전 오프셋 0, 버퍼 512).
- 장치 id 는 `get_default_device_id(false)` 또는 `get_device_id_from_name(name, false)`, 전송 종류는 `get_device_transport_type(id)`(모두 안전한 함수, `coreaudio-rs-0.14.2/src/audio_unit/macos_helpers.rs:43,76,679`).
- 선택자 값: `kAudioDevicePropertyLatency = 0x6c746e63`('ltnc'), `kAudioDevicePropertySafetyOffset = 0x73616674`('saft'), 전송 종류 블루투스 `0x626c7565`('blue')·블루투스 LE `0x626c6561`('blea')(`objc2-core-audio-0.3.2/src/generated/AudioHardware.rs:347-396`).
- `coreaudio-rs 0.14.2` 는 cpal 의 전이 의존성으로 이미 `Cargo.lock` 에 있다. `rbms-audio` 는 `#![forbid(unsafe_code)]`, 워크스페이스 린트는 `unsafe_code = "deny"`.
- 스트림 지연(`kAudioStreamPropertyLatency`)은 재지 않았다. 스트림 id 열거에 안전한 함수가 없다.

### 1.3 macOS Game Mode

- Apple 지원 문서: Game Mode 는 블루투스 샘플링 속도를 두 배로 올려 AirPods 의 오디오 지연과 컨트롤러 입력 지연을 줄인다. Apple silicon, macOS 14 이상, 전체 화면에서만 켜지고 전체 화면을 벗어나면 꺼진다(https://support.apple.com/105118).
- 앱 쪽 조건: `LSApplicationCategoryType` 이 게임 범주, `LSSupportsGameMode`(macOS 26 이후)와 `GCSupportsGameMode`(macOS 14 이후, 함께 써도 됨)가 true. 켜질지는 OS 가 정한다(Apple 개발자 포럼 782343·764575, WWDC25 세션 209).
- 지금 번들: `scripts/macos-app.sh:58` 에 `LSApplicationCategoryType = public.app-category.music-games` 만 있다. Game Mode 키는 없다.
- 개발자 포럼 764070: Game Mode 에 들어가고 나올 때 블루투스 지연값이 바뀐다. 실행 중에 지연을 다시 읽어야 한다.

### 1.4 소프트웨어로 못 하는 것

키음은 누른 뒤에 예약하므로 블루투스 구간 지연만큼 늦게 들린다. 보정은 BGM·노트·판정을 들리는 소리에 맞추는 것까지다.

### 구현 결과(2026-11-19)

- 1.1 의 구조가 이렇게 달라졌다. 줄 번호: `AppShared::audio_clocks_at` `app_play.rs:1089`, `audible_us_from` `engine.rs:766`, `MAX_PLAYBACK_AHEAD_MS` `engine.rs:64`(200 ms. 이제 **백엔드가 준 앞섬에만** 건다), `ahead_or_fallback` `engine.rs:829`. 장치 지연은 콜백이 아니라 스냅샷을 읽는 쪽(`snapshot_of`)에서 `playback_ahead` 에 더하고(`AudioClocks.output_latency_us`, 상한 `MAX_OUTPUT_LATENCY_MS` = 1,000), 판 시작 기준점은 가청 위치가 아니라 `AudioClocks::run_anchor_us`(= 가청 위치 + 지연)다. 조사 때의 "`audible_us` 는 콜백이 낸 `playback_ahead` 로 보간한다"는 그대로이고 그 `playback_ahead` 에 지연이 포함된다. 이 보정이 없으면 160 ms 장치에서 차트 첫 160 ms 의 소리가 이미 지난 프레임으로 예약되는 결함을 구현 단위가 찾았다(조사에는 없던 점).
- 1.1 의 "JUDGE TIMING": 설정 화면과 PLAY 메뉴의 행 이름은 JUDGE OFFSET(`판정 오프셋`/`判定オフセット`)이고, 스킨 선곡 옵션 패널(`lua/i18n/select_other.lua` 의 `sheet_judge_timing`)은 같은 값을 JUDGE TIMING 이라 부른다. 범위 ±200 ms 는 `crates/rbms-config/src/schema.rs:201-204`(`JUDGE_OFFSET_MIN_MS`·`MAX_MS`)에 그대로 있다. 출력 지연 알림 문구는 JUDGE OFFSET 이라는 행 이름으로 안내한다.
- 1.2 의 읽기 방식을 그대로 구현했다(`crates/rbms-audio/src/latency.rs`: `get_device_id_from_name` → `audio_unit_from_device_id` → `get_property::<u32>`, 선택자 `ltnc`·`saft`, 전송 종류 `blue`·`blea`). 호스트 스레드(`host::output_device_latency`)에서만 부른다. 조사 때 재지 않았던 내장 스피커는 108 프레임(지연 60 + 안전 오프셋 48)으로 읽혔다. 스트림 지연(`kAudioStreamPropertyLatency`)은 여전히 읽지 않는다.
- 1.3: `scripts/macos-app.sh:58-63` 에 `GCSupportsGameMode`·`LSSupportsGameMode` true 를 넣었다. 이 앱에서 Game Mode 가 실제로 켜지는지는 확인하지 못했다.
- 1.4: 키음은 여전히 장치 지연만큼 늦다. OUTPUT LATENCY 설명 문구에 적었다.
- 설정: `audio.compensate_output_latency`(기본 true), AUDIO 탭 OUTPUT LATENCY 행(AUTO/OFF). 설정 행은 101개가 됐다.

## 2. 플레이 화면의 출발 폴더

- 선곡 경로 문자열: `STRING_DIRECTORY`(1000) → `shown.directory_path()`(`apps/rbms-player/src/skin_host/select.rs:452`). 선곡 화면 전용(`Cluster::Select`).
- 표 이름·레벨: `STRING_TABLE_NAME`·`_LEVEL`·`_FULL`(1001~1003, `skin_host/chart.rs:389-391`, `Cluster::Chart`).
- 플레이 중에도 열린 폴더는 알 수 있다: Discord 표시가 `open_table_level(shared)`(`apps/rbms-player/src/discord_presence.rs:212`)로 읽는다. 폴더 스택은 `select_tree.rs`.
- 기본 스킨 플레이 머리: `assets/skins/rbms-default/lua/play/info.lua:143` `head`(1줄 제목, 2줄 난이도 이름·레벨·아티스트, 오른쪽 스테이지·배속 표지).

### 구현 결과(2026-11-19)

- 값은 선곡 화면 클러스터의 `STRING_DIRECTORY`(`skin_host/select/shown.rs:253` 의 `directory_path`, 사용처 `skin_host/select.rs:452`)가 아니라 선곡 폴더 스택(`AppShared::open_folders`, `skin_host/select/bridge.rs:134`)에서 만든다. `stage/play/run_setup.rs` 의 `origin_of` 가 마지막 두 마디(`ORIGIN_DEPTH` = 2)를 ` / ` 로 잇는다. Play 클러스터의 확장 텍스트 21010 이 답한다. 코스·리플레이 보기·명령줄 시작은 빈 값이다. 조사의 `open_table_level`(`discord_presence.rs:212`)은 쓰지 않았다.
- 기본 스킨: `play/info.lua` 의 `head`(160행)가 머리 둘째 줄 오른쪽 끝에 강조색으로 쓴다(`origin_share` 0.4).

## 3. 플레이 화면의 옵션 표시

- 지금 플레이 화면에 보이는 옵션: HI-SPEED, GREEN, COVER·LIFT(바뀌는 동안), AUTOPLAY/REPLAY, 배속 표지뿐(`play/info.lua:379-419`). 게이지 종류·배치 옵션·어시스트 옵션 이름은 없다.
- 스킨이 읽을 수 있는 설정 인덱스: `props.index.GAUGE_1P = 40`, `RANDOM_1P = 42`, `RANDOM_2P = 43`. 설정값이지 그 판의 실제 값이 아니다. 빌린 배치(LAYOUT PICK·리플레이 배치)는 설정과 달리 RANDOM + 시드로 달리고(`borrowed_layout.rs`), 코스는 CLASS 계열 게이지, GAUGE AUTO SHIFT 는 판 도중 게이지가 바뀐다.
- 확장 불린: `SCRATCH_AUTO_ON = 21100` 등(`lua/props.lua:167-181`).

### 구현 결과(2026-11-19)

- 조사의 "스킨이 읽을 수 있는 설정 인덱스는 설정값일 뿐"이라는 판단대로 설정 인덱스(40·42·43)는 쓰지 않고 앱이 판의 실제 값을 정해 준다: 확장 텍스트 21011(게이지 이름)·21012·21013(사이드별 배치)·21014(어시스트), `stage/play/run_setup.rs` 의 `RunSetup`. 게이지 종류는 새 숫자(21086)를 만들지 않고 플레이 화면의 인덱스 40 이 이미 엔진의 실제 게이지를 답하는 것을 쓴다.
- 기본 스킨: `play/info.lua` 의 `foot_options`(490행)가 필드 아래 줄에 `게이지 · 배치 · 어시스트` 를 쓴다. 조사 때의 `AUTOPLAY/REPLAY` 줄(`play/info.lua:379-419`)은 같은 줄 오른쪽(`STATUSES`, 447행)으로 옮겼다. `lua/props.lua` 의 이름표는 `props.extension_text`(216행).

## 4. 어시스트 등급 램프 색

- 등급 정의와 색은 이미 있다: [ADR 2026-10-22](../../acknowledge/2026-10-22-adr-assist-lamp-grade.md). 색 `#3ad19f`(ASSIST EASY)·`#c4a796`(ASSIST CLEAR)·`#afb8ef`(ASSIST HARD)·`#ca76a9`(ASSIST EXHARD), 스킨 `theme.grade_lamp`(`lua/theme.lua:154-159`), 이름 `theme.assist_grade`(`theme.lua:181-187`).
- 등급 색이 쓰이는 곳: 선곡 정보창 MY SCORE 의 램프 이름(`select/info.lua:459-467`), 결과 화면 램프 이름(`result/score.lua:69,111`).
- 등급 색이 안 쓰이는 곳: 선곡 목록 막대의 램프(`select/list.lua:147-159` `lamps`). `songlist` 의 램프는 11칸(`crates/rbms-render/src/skin_render/songlist/bars.rs:24` `BAR_LAMPS = 11`)이고 `SongBar.lamp: i32`(`bars.rs:223`)에 등급이 없다. 그래서 SCRATCH AUTO 로 깬 곡은 게이지와 상관없이 모두 LIGHT ASSIST 색(`{101, 99, 255}`)이다. 사용자가 본 "다 같은 색"이 이것이다.
- 폴더 램프 분포(`BarDistribution.lamps[11]`), 스크롤 막대 분포(`list.lua:232-252`), MY RECORDS 줄 램프(`props.extension_number.MY_RECORD_LAMP_FIRST = 21066`)도 11칸 기준이다.

### 구현 결과(2026-11-19)

- 조사가 지적한 "등급 색이 안 쓰이는 곳"이 해소됐다. 목록 막대: `SongBar.assist_grade`(`crates/rbms-render/src/skin_render/songlist/bars.rs`, `SongBar` 220행, `lamp: i32` 는 234행 그대로, `BAR_LAMPS` 24행 11 그대로)와 `songlist` 확장 키 `lamp_graded`. 플레이어는 `stage/select/scene.rs:96` 에서 `bar.assist_grade` 를 넘긴다(조사의 `scene.rs:90` 의 버려지던 자리). 기본 스킨 `select/list.lua` 의 `graded_lamps`(162행)가 `theme.assist_grade` 색을 낸다(`theme.grade_lamp` 154행·`theme.assist_grade` 181행은 그대로).
- MY RECORDS 줄과 랭킹 줄은 값에 등급이 없어 확장 숫자를 더했다: 21087~21092(`MY_RECORD_ASSIST_GRADE_FIRST`)·21093~21102(`RANKING_ASSIST_GRADE_FIRST`, 계획의 21099 를 넘음). `MY_RECORD_LAMP_FIRST` 21066 은 `lua/props.lua:198` 에 그대로다.
- 폴더 램프 분포(`BarDistribution.lamps[11]`)와 스크롤 막대 분포는 11칸 그대로다. 내장(스킨 없는) 선곡 막대의 LED 는 이미 등급을 따랐다.

## 5. 창 모드

- `WindowMode` 는 `Windowed`·`Borderless` 둘(`crates/rbms-config/src/window.rs:68-74`, `WINDOW_MODE_COUNT = 2`). 독점 전체 화면은 없다.
- 적용: `apps/rbms-player/src/window_mode.rs`. `Borderless` 는 `Fullscreen::Borderless(None)`. macOS 에서 winit 의 Borderless 는 OS 전체 화면(Space)이다.
- 설정 행: DISPLAY 탭 WINDOW MODE(`crates/rbms-config/src/settings.rs`), LETTERBOX 설명에 "A borderless window always keeps it"(`settings.rs:781`), `display.fits_screen_shape()`(`apps/rbms-player/src/lib.rs:1445`).
- winit 0.30.13: `Fullscreen::Exclusive(VideoModeHandle)`, `MonitorHandle::video_modes()`.

### 구현 결과(2026-11-19)

- `WindowMode` 는 셋이다(`crates/rbms-config/src/window.rs:66-78`, `WINDOW_MODE_COUNT` = 3 은 12행, 토큰·라벨 `FULLSCREEN`). 조사 때 `lib.rs:1445` 라고 적은 `fits_screen_shape` 는 `crates/rbms-config/src/schema.rs:814` 에 있고 `covers_the_monitor()`(BORDERLESS·FULLSCREEN)를 쓴다. LETTERBOX 설명은 `crates/rbms-config/src/settings.rs:786`("A borderless or full screen window always keeps it").
- `apps/rbms-player/src/window_mode.rs`: `nearest_video_mode`, `changes`(아홉 쌍), `opening_rows`, `AppWindow::{sync, enter_exclusive}`. 조사의 winit 서술은 그대로 맞았다. 새로 확인한 것: macOS 에서 BORDERLESS 로 연 창을 `Exclusive` 로 바꾸면 창이 옛 모드의 틀에 남아 화면 아래 절반이 가려진다(winit 의 (Borderless, Exclusive) 분기가 창 레벨만 바꿈). 그래서 FULLSCREEN 은 일반 창으로 열고 BORDERLESS → FULLSCREEN 도 일반 창을 거친다.
- 이 Mac(기본 2624x1696@120)에서 세 모드와 전환을 실행해 확인했다. Windows·Linux 는 확인하지 못했다.

## 6. 컨트롤러의 Esc

- Esc 는 키보드 고정 키다(`fixed_keys_of`, `apps/rbms-player/src/keyconfig.rs:982-992`). 컨트롤러에 묶을 수 있는 것은 레인·`ControlAction` 20개·`Shortcut` 33개(`Bindable`, `keyconfig.rs:999`)이고 Esc 는 없다.
- 패드 이벤트: `PadEvent::{Lane, Control, Shortcut}`(`gamepad.rs:420`). Control·Shortcut 은 누를 때만 난다(`PadTarget::event`, `gamepad.rs:404`). 분배는 `lib.rs:1412` → `Stage::handle_pad`(`stage/mod.rs:374`).
- Esc 는 누름과 뗌이 다 필요하다: PLAY ESCAPE 가 IMMEDIATE·HOLD(600 ms)·DOUBLE TAP(`crates/rbms-config/src/options.rs:133-147`).
- 키 입력 분배: `lib.rs:1603-1615` → `Stage::handle_key(KeyInput { code, pressed, released, text })`(`stage/mod.rs:138,326`).

### 구현 결과(2026-11-19)

- `ControlAction::Escape`(화면 이름 `ESC`, 설정 키 `ESCAPE`)가 21번째 조작이다(`ControlAction::ALL` 은 21개). 조사의 "Esc 는 키보드 고정 키"는 그대로이고(`fixed_keys_of` 는 `keyconfig.rs:1005`, 조사의 982-992 에서 이동) 거기에 키·버튼 하나를 ESC 로 더 묶을 수 있다. 조사의 "조작 20개·`Shortcut` 33개"는 21개·34개가 됐다(`Shortcut::CaptureLayout`).
- 분배: 키보드는 공통 래퍼 `Stage::handle_key`(`stage/mod.rs:360`)가 ESC 에 준 키를 `KeyCode::Escape` 로 바꾼다. 패드는 조사 때의 `Stage::handle_pad`(`stage/mod.rs:374`)가 아니라 `Stage::handle_marked_pad`(`stage/mod.rs:450`)가 `PadMark { pressed, made_a_chord }` 와 함께 받아 Escape 키 입력으로 바꾼다. `PadTarget::event`(`gamepad.rs:502`)가 ESC 만 뗌도 낸다(조사 때는 누를 때만). `PadEvent` 는 `gamepad.rs:550`.

## 7. 조합키

- 키보드 바인딩은 키 이름 문자열 하나(`ControlBinds` 의 `String`, `KeyConfig.shortcuts: BTreeMap<String, String>`, `keyconfig.rs:522,1177`). 패드는 `PadBinding` 하나(`gamepad.rs:112`, `PadConfig`).
- 눌림 상태: `HeldKeys`(`keyconfig.rs:1121`), `control_held`·`control_repeating`·`shortcut_held`(`app_input.rs:312-340`), 패드는 `pad_held()`.
- 충돌 규칙: `Bindable::read_with`·`clash_among`(`keyconfig.rs:1027,1073`). 키 편집기: `stage/keyconfig.rs`(501줄), 문구 `text/menus/key_editor.rs`.
- START·SELECT 는 눌린 상태로 읽고 화면이 레인 키와 조합한다(`ControlAction` 문서, `keyconfig.rs:358-363`).

### 구현 결과(2026-11-19)

- 저장 형식: 키보드 조합은 같은 문자열 칸에 `A+B` 토큰(`keyconfig/chord.rs` 의 `KeyBind`), 패드는 `PadConfig`(`gamepad.rs:183`)에 둘째 요소 표 `control_chords`·`shortcut_chords` 를 더했다. 조사 때 줄 번호의 현재 위치: `Bindable` `keyconfig.rs:1022`(조사 999), `Bindable::read_with` 1051, `clash_among` 1107(조사 1027·1073), `HeldKeys` 1181(조사 1121), `KeyConfig` 1263 과 `shortcuts` 1268(조사 1177), `ControlBinds` 532(조사 522), `control_held` `app_input.rs:374`·`control_repeating` 381·`shortcut_held` 423(조사 312-340).
- 키 편집기 `stage/keyconfig.rs` 는 612줄이 됐다(조사 501줄). 조합 캡처(`ChordCapture`, 패드는 `PadCapture`), 충돌 규칙(같은 조합은 거절, 단일 키와 겹치면 안내), 새 시험 파일이 더해졌다.
- 조사의 "START·SELECT 는 눌린 상태로 읽고 화면이 레인 키와 조합한다"는 그대로이고, 명시적으로 묶은 조합이 그 내장 조합보다 우선한다. 레인과 스크래치 역방향은 단일 키만이다.

## 8. LAYOUT PICK 미리보기

- `apps/rbms-player/src/stage/layout_pick.rs`(747줄). 미리보기는 내장 플레이필드(`render_playfield_view`)의 정지 화면이다: `PREVIEW_HISPEED = 0.25`(네 마디가 보임), Page Up/Down 으로 `MEASURES_PER_PAGE = 4` 씩 이동. 재생·사용자 속도 반영은 없다.
- 플레이 스킨으로 표본 차트를 돌리는 미리보기가 따로 있다: `stage/play/skin_preview.rs` 의 `PlaySample`(PLAY MENU·SKIN MENU 가 쓴다, 소리 없음, 차트는 `sample_model`).
- 차트를 소리와 함께 자동 재생하는 코드: 선곡 미리듣기 `stage/select/preview.rs`(`autoplay_preview_schedule`, PREVIEW id 공간). 선곡 화면을 벗어나면 정리된다.

### 구현 결과(2026-11-19)

- `apps/rbms-player/src/stage/layout_pick.rs` 는 1051줄이 됐다(조사 747줄). 조사의 "정지 화면, `PREVIEW_HISPEED = 0.25`"는 달라졌다: `PREVIEW_HISPEED` 는 지웠고 미리보기는 `stage/layout_pick/clock.rs` 의 `PreviewClock` 이 프레임 시간으로 흘린다(사용자 HI-SPEED·커버·히든·리프트·CONSTANT 반영, 끝 + 1 초에 처음부터, 소리 없음). `MEASURES_PER_PAGE = 4`(122행)는 그대로이고 Space·Home·End 가 더해졌다.
- 조사가 검토하라던 `PlaySample` 일반화는 하지 않았다: 화면 전체를 사각형에 줄여 그려 레인 아래 번호를 쓸 수 없고, 앱 공용 스킨 타이머에 묶여 정지·탐색이 없으며, 일반화하면 PLAY 메뉴·SKIN 메뉴 미리보기를 건드려야 한다. 구조 변경이 300줄을 넘는지는 재지 않았다(코드를 읽고 판단함).

## 9. 배치 저장(capture)

- 한 판의 배치는 `(배치 옵션 이름, 시드)`로 다시 만든다: `LayoutRequest { md5, random_label, seed, source }`(`borrowed_layout.rs:40`), `AppShared::borrow_layout`(`borrowed_layout.rs:205`). 리플레이 배치 플레이·기록 모달이 이미 쓴다.
- 레인 순서 표기: `rbms_chart::shuffle::{layout_of_permutation, layout_row_text, permutation_of_layout}`.
- 결과 화면 단축키: `Shortcut::Retry`(R)·`NextSong`(N), `ShortcutScreen::Result` 고정 키는 Esc·Enter(`keyconfig.rs:988`). 처리 `stage/result.rs:1083,1236-1257`.
- 설정 폴더의 작은 파일 저장 선례: `favorites.rs`(`favorites.ron`, `write_atomic`).

### 구현 결과(2026-11-19)

- `LayoutRequest`(`borrowed_layout.rs:40`)와 `borrow_layout`(205행)은 그대로 쓴다. 저장소는 새 파일 `apps/rbms-player/src/captured_layouts.rs`(`captured_layouts.ron`, `write_atomic`, `favorites.rs` 와 같은 방식)다. 결과 화면 단축키는 `Shortcut::CaptureLayout`, **기본 키 `P`**: 계획의 `C` 는 모든 기본 배열의 5번째 레인 키이고 결과 화면에서 5번째 키는 "새 배치로 다시 플레이"라서 저장과 함께 화면을 떠난다(조사·검토의 "결과 화면에서 `C` 를 읽는 코드는 없다"는 틀렸다). 처리는 `stage/result.rs:1041` 의 `capture_layout`. 조사의 `ShortcutScreen::Result` 고정 키(Esc·Enter, `keyconfig.rs:1010`)는 그대로다.
- 저장 대상은 레인 순열인 RANDOM·R-RANDOM·MIRROR·ROTATE(ROTATE 는 계획에 없었음)이고 OFF 는 거절한다. LAYOUT PICK 에 CAPTURED 목록이 더해졌다.

## 10. 판정 표시 위치

- 판정 글자는 이미지이고 문서 좌표(1920x1080)로 놓인다: `lua/play/field.lua:250-288`(`judge_row_at`·`judge_of`), 렌더 `crates/rbms-render/src/skin_render/judge.rs:402-518`.
- 판정 위 줄(목표 차이, FAST/SLOW)은 숫자 스프라이트와 텍스트다(`field.lua:290-321`). UI 글자는 장치 화소에 맞춰 그린다([ADR 2026-10-22 ui-text-device-pixels](../../acknowledge/2026-10-22-adr-ui-text-device-pixels.md)).
- 오프셋: LIFT·Judge Position(스킨 오프셋 100)·`JUDGE_1P`. LIFT 는 문서의 첫 레인 사각형에서 계산한다(`stage/play/mod.rs:1227` `lane_offsets`).
- 창 비율: LETTERBOX 설정이 꺼져 있으면 창 크기로 늘린다(`gpu/mod.rs:535` `surface_viewport`). RESOLUTION 다섯 값은 모두 16:9.
- 원인은 읽기만으로 특정하지 못했다. 여러 해상도로 그려 좌표를 재야 한다.

### 구현 결과(2026-11-19)

- 조사의 "원인은 읽기만으로 특정하지 못했다"는 해소됐다. 측정(1280x720 ~ 3840x2160, 16:10 레터박스 켬·끔): 오프셋 0 에서 판정 글자·콤보·목표 차이·판정선 기준 상대 위치는 모든 크기에서 0.5 문서 화소 안(720p 는 반 장치 화소 0.75)이다. 달라지는 것은 **사용자 오프셋**이다: `JUDGE POSITION Y` 30 이 1280x720 에서 45, 2560x1440 에서 22.5, 3840x2160 에서 15 문서 화소를 움직였다(저장값을 화면 물리 화소로 읽고 `authored / target` 으로 환산한 `DocumentOffsets`). 지금은 스킨 문서 단위 그대로 읽어 모든 크기에서 30 이다([오프셋 단위 ADR](../../acknowledge/2026-11-19-adr-user-offsets-in-document-units.md)). 부원인 하나는 FAST/SLOW 글자의 장치 화소 맞춤(16:9 크기 사이 최대 1.5 문서 화소)이고 ADR 2026-10-22 의 절충으로 그대로 둔다.
- 줄 번호의 현재 위치: `lua/play/field.lua` 의 `judge_row_at` 260행·`judge_of` 279행·`above_of` 320행(조사 250-288·290-321), `gpu/mod.rs` 의 `surface_viewport` 1271행(조사 535). 조사의 `lane_offsets`(`stage/play/mod.rs:1227`)는 `apps/rbms-player` 가 아니라 `crates/rbms-render/src/skin_render/notes.rs:412`(`lane_offsets`, `first_lane_rect` 429행)에 있고 플레이어는 `stage/play/skin_preview.rs` 가 `rbms_render::skin_render::frame` 에서 가져다 쓴다. `crates/rbms-render/src/skin_render/judge.rs`·`text.rs` 의 줄 번호는 다시 확인하지 않았다.
- RESOLUTION 다섯 값이 모두 16:9 인 것은 그대로다. 회귀 시험 `stage/capture/play_parts/judge_sizes.rs` 가 해상도별 문서 좌표 일치(오프셋 0, LIFT·COVER, JUDGE POSITION 30)를 지킨다(시간 때문에 3840x2160 과 1600x900 은 목록에서 뺐다).

## 11. NPS

- 선곡 화면의 밀도 값은 있다(`props.number.DENSITY_PEAK = 360` 등). 플레이 중 현재 NPS 값은 없다.
- 판정 위 줄 배치: `field.lua:290-321` `above_of`(목표 차이 숫자 뒤에 `ABOVE.gap` 을 두고 FAST/SLOW 또는 ms).
- 스킨 옵션 id: `layout.OPTION` 900~915, 930~936 사용 중(`lua/play/layout.lua:6-30`).

### 구현 결과(2026-11-19)

- 조사의 "플레이 중 현재 NPS 값은 없다"가 해소됐다: 정수 21085(`props.extension_number.PLAY_NPS`), 계산은 `skin_host/play/shown.rs:188` 의 `notes_in_the_last_second`(직전 1,000 ms, 일반 노트와 롱노트 시작, 지뢰·보이지 않는 노트·롱노트 끝 제외). 스킨은 `lua/play/field.lua` 의 `nps`(352행)가 `above_of`(320행)의 FAST/SLOW 칸 오른쪽에 그린다. 스킨 옵션 `NPS Display` 는 916(On)·917(Off)이다(조사 때 900~915·930~936 사용 중이었다). DP 는 첫 사이드에만 그린다.

## 12. 선곡 화면

- 스킨: `assets/skins/rbms-default/lua/select/`(main·list·info·side·panel·controls·records·radar·sheet·words·inform·kit), 토큰 `lua/theme.lua`. 현재 모습은 `docs/assets/screenshot-select.jpg`.
- 메인이 캡처를 보고 적은 문제:
    1. 머리: 한 줄에 모양이 다른 7개 블록(`MUSIC SELECT` 맨글자, `SP`·`FREE PLAY` 채운 칩, 밑줄 검색칸, 시계 칩, 두 줄짜리 MODE·SORT·LN MODE 카드)이 섞여 있고, 아래 선이 오른쪽 절반에만 있다. 경로(`ALL SONGS`)는 머리와 목록 사이에 떠 있다.
    2. 제목: 제목 글자가 화면에서 가장 크고(디스플레이 크기), LEVEL·BPM 은 오른쪽 끝에 떨어져 있어 사이가 빈다. 블록 높이 200.
    3. 바닥: 두 줄. 둘째 줄은 대문자 문장("SEARCH, FILTER, ... SEE H FOR THE KEY OF EACH")이다. 왼쪽 키 표지와 오른쪽 버튼 높이가 다르다.
    4. 정보창: 구획 머리의 선 길이와 값 정렬이 구획마다 다르다. MY SCORE 의 등급 상자와 `MAX -27` 이 붙어 있다.
- 지켜야 할 결정: [select-iidx-layout](../../acknowledge/2026-10-24-adr-select-iidx-layout.md)(바탕 모노톤, 강조색은 등급·램프·메달·레이더만), [select-info-overflow](../../acknowledge/2026-10-29-adr-select-info-overflow.md).
- 캡처: `RBMS_SKIN_PACK=$PWD/assets/skins/rbms-default RBMS_SKIN_CAPTURE_DIR=<폴더> cargo test -p rbms-player --lib --locked the_browser_of_a_skin_pack_named_by_the_environment_is_captured_over_a_scanned_library`.

### 구현 결과(2026-11-19)

- 조사의 문제 1~4 는 스킨만 고쳐 해소했다(`lua/select/{controls,info,kit,list,main,radar,records}.lua`, `lua/theme.lua`, `lua/i18n/select_bar.lua`). 머리: 한 줄에 모양이 다른 블록 7개 → 기준선 y 77 을 공유하는 한 띠(정보는 맨글자, 조작은 칩과 칸. MODE·SORT·LN MODE 는 한 줄 칩). 경로는 머리 왼쪽으로 올렸다. 제목: 블록 높이 200 → 136, 제목 `title` 64 → `heading` 44, LEVEL·BPM 을 색 막대 옆 폭 104 칸으로. 바닥: 두 줄 → 한 줄(중심선 y 1040, 키 표지 높이 28·버튼 높이 40). 정보창: 구획 머리를 `kit.section` 한 부품으로 통일, MY SCORE 의 램프 이름과 `MAX -27` 을 등급 상자 아래 한 줄로.
- 문제 1 의 "아래 선이 오른쪽 절반에만 있다"는 선이 반만 있던 것이 아니었다. 선은 원래 64~1856 전체에 있었고, 오른쪽 필터 카드의 아랫변(y 96)이 선(y 104) 바로 위에 붙어 오른쪽 절반에만 이중선처럼 보인 것이었다. 칩이 88 에서 끝나게 해서 하나만 보인다.
- LN MODE 칩 값은 LN·CN·HCN 약어(한 줄 칩에서 전체 이름이 잘림). 검색 강조는 `STRING_DIRECTORY` 가 `Search : '` 로 시작하는지로 판단한다(스킨이 읽을 입력 중 값이 없음). 한국어·일본어와 실창은 확인하지 못했다. 캡처 하니스 환경 변수와 시험 이름은 조사의 것이 그대로 맞다. `docs/assets/screenshot-select.jpg` 는 개편 전 모습이다(메인이 교체한다).
- 지킬 결정: [select-iidx-layout](../../acknowledge/2026-10-24-adr-select-iidx-layout.md)과 [select-info-overflow](../../acknowledge/2026-10-29-adr-select-info-overflow.md)의 일부(머리 칩·경로 위치·바닥 두 줄, 곡 머리 LEVEL·BPM 위치와 크기, MY SCORE 칸 폭, 레이더 중심)는 [선곡 화면 개편 ADR](../../acknowledge/2026-11-19-adr-select-screen-refresh.md)로 대체됐다.

## 공통

- 확장 id 빈 자리(2026-11-19): 숫자 21085~, 텍스트 21006~, 불린 21107~21109·21160~, 이벤트 21004~21009(`crates/rbms-skin/src/property/extension.rs`, `lua/props.lua:167-215`).
- 설정 행은 100개(`SETTING_COUNT`, `crates/rbms-config/src/settings.rs:279`). 설정 문서는 서버에 통째로 올라간다(`contract/client-requests/settings-blob.json`, 서버는 필드를 검증하지 않는다). 문서 모양이 바뀌면 계약 파일을 다시 만든다.

### 구현 결과(2026-11-19)

- 확장 id 빈 자리(2026-11-19 갱신): 숫자 21085(`PLAY_NPS`)와 21087~21102(등급)를 썼고 21086 은 만들지 않아 비어 있다. 빈 숫자는 21086·21103~, 텍스트는 21006~21009·21015~(21010~21014 사용), 불린은 21107~21109·21160~, 이벤트는 21004~21009 그대로다. 램프 반 쪼개기가 계획한 불린 21160~21163 은 구현 전이라 코드에 없다.
- 설정 행은 101개다(`SETTING_COUNT`, `crates/rbms-config/src/settings.rs:283`. OUTPUT LATENCY 가 마지막 `SettingId`). 설정 계약 파일 `contract/client-requests/settings-blob.json` 은 audio 에 `compensate_output_latency` 와 keyconfig 에 `controls.escape`·`pad.control_chords`·`pad.shortcut_chords` 가 늘어 다시 만들었다.

