# 조사: 기능 고도화 12건 (2026-11-19)

최종 갱신 2026-11-19 · 대응 커밋 `439a93b`(조사 시점의 `dev`). 조사자: 메인(Fable). 계획은 [plan/2026-11-19-feature-upgrade-12.md](../../plan/2026-11-19-feature-upgrade-12.md).

이 문서는 12개 요구의 현재 코드 근거를 적는다. 구현으로 대상 코드가 바뀌면 해당 절을 고친다.

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

## 2. 플레이 화면의 출발 폴더

- 선곡 경로 문자열: `STRING_DIRECTORY`(1000) → `shown.directory_path()`(`apps/rbms-player/src/skin_host/select.rs:452`). 선곡 화면 전용(`Cluster::Select`).
- 표 이름·레벨: `STRING_TABLE_NAME`·`_LEVEL`·`_FULL`(1001~1003, `skin_host/chart.rs:389-391`, `Cluster::Chart`).
- 플레이 중에도 열린 폴더는 알 수 있다: Discord 표시가 `open_table_level(shared)`(`apps/rbms-player/src/discord_presence.rs:212`)로 읽는다. 폴더 스택은 `select_tree.rs`.
- 기본 스킨 플레이 머리: `assets/skins/rbms-default/lua/play/info.lua:143` `head`(1줄 제목, 2줄 난이도 이름·레벨·아티스트, 오른쪽 스테이지·배속 표지).

## 3. 플레이 화면의 옵션 표시

- 지금 플레이 화면에 보이는 옵션: HI-SPEED, GREEN, COVER·LIFT(바뀌는 동안), AUTOPLAY/REPLAY, 배속 표지뿐(`play/info.lua:379-419`). 게이지 종류·배치 옵션·어시스트 옵션 이름은 없다.
- 스킨이 읽을 수 있는 설정 인덱스: `props.index.GAUGE_1P = 40`, `RANDOM_1P = 42`, `RANDOM_2P = 43`. 설정값이지 그 판의 실제 값이 아니다. 빌린 배치(LAYOUT PICK·리플레이 배치)는 설정과 달리 RANDOM + 시드로 달리고(`borrowed_layout.rs`), 코스는 CLASS 계열 게이지, GAUGE AUTO SHIFT 는 판 도중 게이지가 바뀐다.
- 확장 불린: `SCRATCH_AUTO_ON = 21100` 등(`lua/props.lua:167-181`).

## 4. 어시스트 등급 램프 색

- 등급 정의와 색은 이미 있다: [ADR 2026-10-22](../../acknowledge/2026-10-22-adr-assist-lamp-grade.md). 색 `#3ad19f`(ASSIST EASY)·`#c4a796`(ASSIST CLEAR)·`#afb8ef`(ASSIST HARD)·`#ca76a9`(ASSIST EXHARD), 스킨 `theme.grade_lamp`(`lua/theme.lua:154-159`), 이름 `theme.assist_grade`(`theme.lua:181-187`).
- 등급 색이 쓰이는 곳: 선곡 정보창 MY SCORE 의 램프 이름(`select/info.lua:459-467`), 결과 화면 램프 이름(`result/score.lua:69,111`).
- 등급 색이 안 쓰이는 곳: 선곡 목록 막대의 램프(`select/list.lua:147-159` `lamps`). `songlist` 의 램프는 11칸(`crates/rbms-render/src/skin_render/songlist/bars.rs:24` `BAR_LAMPS = 11`)이고 `SongBar.lamp: i32`(`bars.rs:223`)에 등급이 없다. 그래서 SCRATCH AUTO 로 깬 곡은 게이지와 상관없이 모두 LIGHT ASSIST 색(`{101, 99, 255}`)이다. 사용자가 본 "다 같은 색"이 이것이다.
- 폴더 램프 분포(`BarDistribution.lamps[11]`), 스크롤 막대 분포(`list.lua:232-252`), MY RECORDS 줄 램프(`props.extension_number.MY_RECORD_LAMP_FIRST = 21066`)도 11칸 기준이다.

## 5. 창 모드

- `WindowMode` 는 `Windowed`·`Borderless` 둘(`crates/rbms-config/src/window.rs:68-74`, `WINDOW_MODE_COUNT = 2`). 독점 전체 화면은 없다.
- 적용: `apps/rbms-player/src/window_mode.rs`. `Borderless` 는 `Fullscreen::Borderless(None)`. macOS 에서 winit 의 Borderless 는 OS 전체 화면(Space)이다.
- 설정 행: DISPLAY 탭 WINDOW MODE(`crates/rbms-config/src/settings.rs`), LETTERBOX 설명에 "A borderless window always keeps it"(`settings.rs:781`), `display.fits_screen_shape()`(`apps/rbms-player/src/lib.rs:1445`).
- winit 0.30.13: `Fullscreen::Exclusive(VideoModeHandle)`, `MonitorHandle::video_modes()`.

## 6. 컨트롤러의 Esc

- Esc 는 키보드 고정 키다(`fixed_keys_of`, `apps/rbms-player/src/keyconfig.rs:982-992`). 컨트롤러에 묶을 수 있는 것은 레인·`ControlAction` 20개·`Shortcut` 33개(`Bindable`, `keyconfig.rs:999`)이고 Esc 는 없다.
- 패드 이벤트: `PadEvent::{Lane, Control, Shortcut}`(`gamepad.rs:420`). Control·Shortcut 은 누를 때만 난다(`PadTarget::event`, `gamepad.rs:404`). 분배는 `lib.rs:1412` → `Stage::handle_pad`(`stage/mod.rs:374`).
- Esc 는 누름과 뗌이 다 필요하다: PLAY ESCAPE 가 IMMEDIATE·HOLD(600 ms)·DOUBLE TAP(`crates/rbms-config/src/options.rs:133-147`).
- 키 입력 분배: `lib.rs:1603-1615` → `Stage::handle_key(KeyInput { code, pressed, released, text })`(`stage/mod.rs:138,326`).

## 7. 조합키

- 키보드 바인딩은 키 이름 문자열 하나(`ControlBinds` 의 `String`, `KeyConfig.shortcuts: BTreeMap<String, String>`, `keyconfig.rs:522,1177`). 패드는 `PadBinding` 하나(`gamepad.rs:112`, `PadConfig`).
- 눌림 상태: `HeldKeys`(`keyconfig.rs:1121`), `control_held`·`control_repeating`·`shortcut_held`(`app_input.rs:312-340`), 패드는 `pad_held()`.
- 충돌 규칙: `Bindable::read_with`·`clash_among`(`keyconfig.rs:1027,1073`). 키 편집기: `stage/keyconfig.rs`(501줄), 문구 `text/menus/key_editor.rs`.
- START·SELECT 는 눌린 상태로 읽고 화면이 레인 키와 조합한다(`ControlAction` 문서, `keyconfig.rs:358-363`).

## 8. LAYOUT PICK 미리보기

- `apps/rbms-player/src/stage/layout_pick.rs`(747줄). 미리보기는 내장 플레이필드(`render_playfield_view`)의 정지 화면이다: `PREVIEW_HISPEED = 0.25`(네 마디가 보임), Page Up/Down 으로 `MEASURES_PER_PAGE = 4` 씩 이동. 재생·사용자 속도 반영은 없다.
- 플레이 스킨으로 표본 차트를 돌리는 미리보기가 따로 있다: `stage/play/skin_preview.rs` 의 `PlaySample`(PLAY MENU·SKIN MENU 가 쓴다, 소리 없음, 차트는 `sample_model`).
- 차트를 소리와 함께 자동 재생하는 코드: 선곡 미리듣기 `stage/select/preview.rs`(`autoplay_preview_schedule`, PREVIEW id 공간). 선곡 화면을 벗어나면 정리된다.

## 9. 배치 저장(capture)

- 한 판의 배치는 `(배치 옵션 이름, 시드)`로 다시 만든다: `LayoutRequest { md5, random_label, seed, source }`(`borrowed_layout.rs:40`), `AppShared::borrow_layout`(`borrowed_layout.rs:205`). 리플레이 배치 플레이·기록 모달이 이미 쓴다.
- 레인 순서 표기: `rbms_chart::shuffle::{layout_of_permutation, layout_row_text, permutation_of_layout}`.
- 결과 화면 단축키: `Shortcut::Retry`(R)·`NextSong`(N), `ShortcutScreen::Result` 고정 키는 Esc·Enter(`keyconfig.rs:988`). 처리 `stage/result.rs:1083,1236-1257`.
- 설정 폴더의 작은 파일 저장 선례: `favorites.rs`(`favorites.ron`, `write_atomic`).

## 10. 판정 표시 위치

- 판정 글자는 이미지이고 문서 좌표(1920x1080)로 놓인다: `lua/play/field.lua:250-288`(`judge_row_at`·`judge_of`), 렌더 `crates/rbms-render/src/skin_render/judge.rs:402-518`.
- 판정 위 줄(목표 차이, FAST/SLOW)은 숫자 스프라이트와 텍스트다(`field.lua:290-321`). UI 글자는 장치 화소에 맞춰 그린다([ADR 2026-10-22 ui-text-device-pixels](../../acknowledge/2026-10-22-adr-ui-text-device-pixels.md)).
- 오프셋: LIFT·Judge Position(스킨 오프셋 100)·`JUDGE_1P`. LIFT 는 문서의 첫 레인 사각형에서 계산한다(`stage/play/mod.rs:1227` `lane_offsets`).
- 창 비율: LETTERBOX 설정이 꺼져 있으면 창 크기로 늘린다(`gpu/mod.rs:535` `surface_viewport`). RESOLUTION 다섯 값은 모두 16:9.
- 원인은 읽기만으로 특정하지 못했다. 여러 해상도로 그려 좌표를 재야 한다.

## 11. NPS

- 선곡 화면의 밀도 값은 있다(`props.number.DENSITY_PEAK = 360` 등). 플레이 중 현재 NPS 값은 없다.
- 판정 위 줄 배치: `field.lua:290-321` `above_of`(목표 차이 숫자 뒤에 `ABOVE.gap` 을 두고 FAST/SLOW 또는 ms).
- 스킨 옵션 id: `layout.OPTION` 900~915, 930~936 사용 중(`lua/play/layout.lua:6-30`).

## 12. 선곡 화면

- 스킨: `assets/skins/rbms-default/lua/select/`(main·list·info·side·panel·controls·records·radar·sheet·words·inform·kit), 토큰 `lua/theme.lua`. 현재 모습은 `docs/assets/screenshot-select.jpg`.
- 메인이 캡처를 보고 적은 문제:
    1. 머리: 한 줄에 모양이 다른 7개 블록(`MUSIC SELECT` 맨글자, `SP`·`FREE PLAY` 채운 칩, 밑줄 검색칸, 시계 칩, 두 줄짜리 MODE·SORT·LN MODE 카드)이 섞여 있고, 아래 선이 오른쪽 절반에만 있다. 경로(`ALL SONGS`)는 머리와 목록 사이에 떠 있다.
    2. 제목: 제목 글자가 화면에서 가장 크고(디스플레이 크기), LEVEL·BPM 은 오른쪽 끝에 떨어져 있어 사이가 빈다. 블록 높이 200.
    3. 바닥: 두 줄. 둘째 줄은 대문자 문장("SEARCH, FILTER, ... SEE H FOR THE KEY OF EACH")이다. 왼쪽 키 표지와 오른쪽 버튼 높이가 다르다.
    4. 정보창: 구획 머리의 선 길이와 값 정렬이 구획마다 다르다. MY SCORE 의 등급 상자와 `MAX -27` 이 붙어 있다.
- 지켜야 할 결정: [select-iidx-layout](../../acknowledge/2026-10-24-adr-select-iidx-layout.md)(바탕 모노톤, 강조색은 등급·램프·메달·레이더만), [select-info-overflow](../../acknowledge/2026-10-29-adr-select-info-overflow.md).
- 캡처: `RBMS_SKIN_PACK=$PWD/assets/skins/rbms-default RBMS_SKIN_CAPTURE_DIR=<폴더> cargo test -p rbms-player --lib --locked the_browser_of_a_skin_pack_named_by_the_environment_is_captured_over_a_scanned_library`.

## 공통

- 확장 id 빈 자리(2026-11-19): 숫자 21085~, 텍스트 21006~, 불린 21107~21109·21160~, 이벤트 21004~21009(`crates/rbms-skin/src/property/extension.rs`, `lua/props.lua:167-215`).
- 설정 행은 100개(`SETTING_COUNT`, `crates/rbms-config/src/settings.rs:279`). 설정 문서는 서버에 통째로 올라간다(`contract/client-requests/settings-blob.json`, 서버는 필드를 검증하지 않는다). 문서 모양이 바뀌면 계약 파일을 다시 만든다.
