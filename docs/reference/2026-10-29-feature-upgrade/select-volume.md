# 선곡 화면 음량 조절이 "안 먹는 느낌" — 원인 조사

최종 갱신 2026-10-29 · 대응 작업 U4 (조사 기준 커밋 c1dc988)

사용자 보고: "사운드 조절(곡선택화면)이 제대로 먹히긴 하나? 안 먹는 느낌". 0~5절은 수정 전에 한 읽기 전용 조사이고, 6절이 U4 에서 고친 결과다. 0~5절의 표와 줄 번호는 수정 전 코드 기준이라 6절 이후에는 맞지 않는 곳이 있다.

## 0. 결론 요약

슬라이더 → 설정 → 믹서로 가는 경로는 **고장 나지 않았다.** 드래그한 값은 그 프레임이 끝날 때 오디오 스트림에 바로 들어가고(기존 시험 11개 통과, 4절), 손을 떼면 파일에 저장된다. "안 먹는 느낌"의 근본 원인은 **세 슬라이더가 움직이는 버스와, 선곡 화면에서 실제로 들리는 소리가 타는 버스가 어긋난 것**이다. 확신도는 높음(코드 경로와 시험 근거, 기본 효과음 길이 실측).

| 기본 스킨 슬라이더(표시 이름) | 실제로 바꾸는 값 | 선곡 화면에서 그 값의 영향을 받는 소리 | 체감 |
| --- | --- | --- | --- |
| MASTER / 마스터 (rate 17) | `audio.system`(시스템 버스). 진짜 마스터인 `audio.master` 가 **아님** | 짧은 시스템 효과음(o-change, scratch, f-open 등)과 길이 3초 이상인 `select` 음악. 기본 효과음 묶음의 `select.wav` 는 0.5초라 선곡 음악은 **없다** | 미리듣기가 나오는 동안 끝까지 내려도 미리듣기는 그대로. "안 먹는다" |
| KEY / 키음 (rate 18) | `audio.key`(키 버스) | **없음.** 자동 재생 미리듣기의 키음도 BGM 버스로 울린다 | 선곡 화면에서는 전혀 안 들림 |
| BGM (rate 19) | `audio.bg`(BGM 버스) | 미리듣기 전체(`#PREVIEW` 파일, 자동 재생 미리듣기의 키음과 BGM) | 바로 반영됨. 셋 중 이것 하나만 들린다 |

곁들이는 원인: (a) 스킨의 "MASTER" 숫자는 시스템 음량(기본 50%)을 보이는데, SETTINGS 의 MASTER VOL(기본 100%)은 다른 값이라 둘이 따로 논다. (b) 음량이 선형 이득이라 50%→100% 가 +6 dB 뿐이어서 위쪽 절반에서 변화가 작게 들린다(보조 요인).

## 1. 현재 구조

### 1.1 음량 설정의 종류와 저장 위치

| 값 | 필드 | 기본값 | 근거 |
| --- | --- | --- | --- |
| 마스터 | `config.audio.master` | 1.0 | `crates/rbms-config/src/audio.rs:5`, `:55`, `:81` |
| 키음 버스 | `config.audio.key` | 0.5 | `audio.rs:8`, `:57`, `:82` |
| BGM 버스 | `config.audio.bg` | 0.5 | `audio.rs:59`, `:83` |
| 시스템 버스 | `config.audio.system` | 0.5 | `audio.rs:61`, `:84` |
| 미리듣기 | `config.library.preview_volume` | 0.85 | `crates/rbms-config/src/schema.rs:154`, `:886`, `:916` |
| 미리듣기 페이드 | `config.library.preview_fade_ms` | 200 ms | `schema.rs:166`, `:888` |

- `AudioOptions` 는 `#[serde(default)]` (`audio.rs:43-45`). 범위 0..=1, `clamp_volume`(`audio.rs:151-153`)이 NaN 을 기본값으로, 나머지를 0..1 로 자른다.
- 이득은 선형이다. 최종 출력 = 소리 하나의 gain × 버스 이득 × 차트 이득 × 마스터 (`crates/rbms-audio/src/mixer.rs:692-696`, `:729-733`).

### 1.2 바꾸는 곳(UI)

1. **선곡 화면 사이드 메뉴 VOLUME 탭(기본 스킨)**: 스피커 아이콘(`assets/skins/rbms-default/lua/select/side.lua:16`)으로 열고, 슬라이더 세 개를 그린다(`side.lua:51-56`, `:124-144`).
   - 라벨: `side_master` = MASTER/마스터/マスター, `side_key_sound` = KEY/키음/キー音, 그리고 `"BGM"` 문자열 그대로 (`assets/skins/rbms-default/lua/i18n/select_other.lua:7-8`, `side.lua:55`).
   - id: rate 17/18/19 (`assets/skins/rbms-default/lua/props.lua:400-402`), 숫자 57/58/59 (`props.lua:232-234`).
   - `kit.slider` 는 `changeable = true` 로 만든다(`assets/skins/rbms-default/lua/select/kit.lua:30-32`). 읽기 전용으로 묶인 것이 **아니다**.
2. **SETTINGS → AUDIO 탭**: MASTER VOL / KEY VOL / BGM VOL / SYSTEM VOL (`crates/rbms-config/src/settings.rs:854-881`, 라벨 `apps/rbms-player/src/text/settings/labels.rs:101-104`). SELECT 탭의 PREVIEW VOLUME (`settings.rs:908-914`).
3. **스킨 Lua**: `main_state.volume_sys/key/bg` 와 `set_volume_*` (`crates/rbms-skin/src/lua/main_state.rs:358`, `:425-430`). 시스템/키/BGM 셋뿐이고 마스터는 없다(`crates/rbms-skin/src/property/host.rs:53-60` 의 `VolumeBus`).
4. 선곡 화면의 옵션 패널(QUICK SETTINGS, 패널 1~3)에는 음량 행이 없다.

### 1.3 슬라이더 값이 출력까지 가는 경로(전부 즉시)

1. 포인터가 슬라이더 트랙을 누르거나 끌면 `SkinInputMap` 이 `SkinAction::Write { writer: Rate(17|18|19), value }` 를 만든다. 쓰기 대상은 `changeable` 이고 레퍼런스가 쓸 수 있는 rate 일 때만 정해진다(`crates/rbms-render/src/skin_render/input.rs:196-209`, `:316-330`).
2. 프레임 안에서 `host.write_rate(id, value)` 로 기록된다(`apps/rbms-player/src/skin_screen.rs:671`).
3. 프레임 끝 `finish_skin_frame` → `carry_out_skin_calls` → `PlayerRequests::cluster` 가 먼저 `writers::carry_out` 을 부른다(`skin_screen.rs:1306-1315`, `:686-691`).
4. `carry_out` 이 `rate_bus(id)` 로 버스를 고른다. **rate 17 → `VolumeBus::System`**, 18 → Key, 19 → Background (`apps/rbms-player/src/skin_host/writers.rs:57-63`). `set_volume` 이 `config.audio.system/key/bg` 를 바꾸고(`writers.rs:71-80`), 값이 움직였으면 `apply_audio_gains()` 와 `note_skin_settings_changed()` (`writers.rs:89-101`).
5. `apply_audio_gains` 는 마스터와 세 버스 이득을 모두 엔진에 넣는다(`apps/rbms-player/src/settings_ui.rs:380-387`). 엔진은 명령을 믹서로 보내고(`crates/rbms-audio/src/engine.rs:524-531`), 믹서는 슬루를 걸어 바로 적용한다(`mixer.rs:459-466`). 이미 울리는 소리에도 적용된다(시험 `bus_gain_takes_effect_while_a_voice_is_already_sounding`, `mixer.rs:1589`).
6. 저장: 마우스를 떼면 `save_settings()` (`skin_screen.rs:1312-1314`). 드래그 중에는 쓰지 않는다.
7. 스킨이 보이는 숫자/채움은 `SystemState.volumes` 에서 읽는다: NUMBER 57 / RATE 17 = system (`apps/rbms-player/src/skin_host/system.rs:147-148`, `:191-202`, 값 공급 `skin_screen.rs:1654`).

SETTINGS 화면도 같은 `apply_audio_gains` 로 즉시 반영한다(`apps/rbms-player/src/stage/settings.rs:337-345`, `:563`). 스트림을 새로 열 때도 다시 넣는다(`apps/rbms-player/src/app_play.rs:886`).

### 1.4 선곡 화면에서 실제로 울리는 소리와 버스

| 소리 | 버스 | 소리 하나의 gain | 근거 |
| --- | --- | --- | --- |
| `#PREVIEW` 파일 미리듣기 | **Bg** | `library.preview_volume` | `apps/rbms-player/src/stage/select/preview.rs:394-400`, `:306-309` |
| 자동 재생 미리듣기(키음과 BGM 모두) | **Bg** | `library.preview_volume` | `preview.rs:288-296`. 일정 생성 때 `ev.source`(Key/Bgm)를 버린다(`preview.rs:453-461`) |
| 미리듣기 페이드 | Bg 버스 이득을 `bg × level` 로 매 프레임 덮어씀 | - | `preview.rs:176-191`, 끝나면 `release_bus` 로 `bg` 복귀 `preview.rs:195-200` |
| 시스템 효과음 | System | 1.0 | `apps/rbms-player/src/syssound.rs:72-74`, `:437-441` |
| 선곡 음악(`select` 가 3초 이상일 때만) | System(루프 effect) | 1.0, 미리듣기 중 level 0 | `syssound.rs:81`, `:390`, `:484-499` |
| 스킨이 트는 소리 | System | 스킨이 준 값 | `apps/rbms-player/src/skin_host/audio.rs:124` |

- 기본 효과음 묶음의 `select.wav` 는 `afinfo` 실측 0.5초(`assets/skins/rbms-default/sound/select.wav`). 그래서 기본 설치에서는 선곡 음악이 돌지 않는다(`syssound.rs:30-35`).
- 플레이 화면에서는 키음이 Key, BGM 이 Bg 로 간다(`apps/rbms-player/src/play_sink.rs:18-19`). 선곡 미리듣기만 전부 Bg 다.

### 1.5 레퍼런스(beatoraja) 대조

- rate 17/18/19 는 각각 `systemvolume / keyvolume / bgvolume` 을 읽고 쓴다(`~/development/beatoraja/src/bms/player/beatoraja/skin/property/FloatPropertyFactory.java:261-274`). 레퍼런스에는 별도 마스터가 없다. rbms 의 rate 17 → System 대응은 레퍼런스를 그대로 옮긴 것이다(`docs/reference/skin-compat/b5-properties.md:871`).
- 레퍼런스는 **미리듣기와 선곡 음악을 `systemvolume` 으로 튼다**. 값이 바뀌면 50 ms 안에 재생 중인 미리듣기의 음량도 바꾼다(`PreviewMusicProcessor.java:83-109`). 그래서 레퍼런스에서는 "MASTER" 슬라이더가 선곡 화면의 미리듣기 음량 조절 역할을 한다.
- rbms 는 미리듣기를 Bg 버스와 별도의 `preview_volume` 으로 옮겼다. 그러면서 rate 17 은 레퍼런스 뜻(system)으로 남아, **레퍼런스에서 미리듣기를 조절하던 슬라이더가 rbms 에서는 미리듣기와 끊겼다.** 이것이 차이의 핵심이다.

### 1.6 문서와 코드의 차이

- `docs/quality-assurance/2026-10-11-default-skin-manual-check.md:42`, `:290` 은 "볼륨 슬라이더를 드래그하면 … 실제 소리 크기가 바뀜"을 기대값으로 둔다. 어느 소리인지는 적지 않았다. 코드상 MASTER 와 KEY 는 선곡 화면에서 기대대로 들리지 않는다.
- `docs/skin.md:527` 은 "VOLUME 3개"라고만 쓰고 버스 대응은 적지 않았다. `writers.rs:20` 의 모듈 문서는 "system, key and background"라고 정확히 쓴다.
- 설정 설명 문구 `MasterVolume` = "The volume of everything the player sounds"(`apps/rbms-player/src/text/settings/descriptions.rs:289-290`). 스킨 라벨 MASTER 는 이 값이 아니다.

## 2. 원인 후보별 수정안

### 원인 1 (주 원인): "MASTER" 슬라이더가 시스템 버스라 미리듣기에 닿지 않음

대안 A — **rate 17·NUMBER 57 을 진짜 마스터(`audio.master`)로 다시 묶는다** (추천)
- 바꿀 곳:
  - `apps/rbms-player/src/skin_host/writers.rs` `rate_bus` / `set_volume`: 17 → master. `VolumeBus` 는 rbms-skin 공개 타입이고 Lua `volume_sys` 와 묶여 있으므로(`main_state.rs:358`), 둘 중 하나를 고른다. ① 앱 쪽 전용 열거형(예: `VolumeTarget { Master, Bus(VolumeBus) }`)을 `writers.rs` 에 둔다(Lua API 는 그대로). ② `VolumeBus::Master` 를 추가하되 Lua 표에는 넣지 않는다.
  - `apps/rbms-player/src/skin_host/system.rs` `Volumes` 에 `master` 필드를 두고 NUMBER 57 / RATE 17 이 그것을 읽게 한다(`system.rs:80-93`, `:191-202`). 공급처 `skin_screen.rs:1654`, `skin_screen/preview.rs:176`.
- 장점: 라벨(MASTER/마스터)과 동작이 맞고, SETTINGS 의 MASTER VOL 과 같은 값을 보인다. 미리듣기, 효과음, 플레이까지 모두 즉시 들린다. 외부 beatoraja 스킨도 대개 "MASTER"라고 적어 두었으니 라벨과 맞는다.
- 단점: 레퍼런스 id 뜻(17 = systemvolume)과 달라진다. `b5-properties.md:871` 과 `docs/skin.md` 에 rbms 차이로 적어야 한다. 그 뒤 시스템 음량은 SETTINGS 와 Lua `set_volume_sys` 로만 바꿀 수 있다.

대안 B — 레퍼런스처럼 **미리듣기를 시스템 음량에 따르게** 한다
- 미리듣기를 System 버스로 옮기거나, 소리마다 `preview_volume × system` 을 준다.
- 장점: id 뜻이 레퍼런스와 같다.
- 단점: 페이드가 지금 Bg 버스 이득으로 구현돼 있다(`preview.rs:176-200`). System 버스로 옮기면 효과음까지 페이드된다. 소리마다 gain 을 매기면 이미 울리는 `#PREVIEW` 클립은 다음 반복 때까지 바뀌지 않는다(`play_on` gain 은 시작 때 고정). 또 BGM 슬라이더가 미리듣기에서 떨어져, 지금 유일하게 들리는 슬라이더를 잃는다. 비추천.

대안 C — 기본 스킨 라벨만 "SYSTEM / 시스템 / システム"으로 바꾸고, 마스터용 rbms 전용 rate 를 새로 만들어 네 번째 슬라이더를 단다
- 장점: 레퍼런스 뜻이 그대로이고 오해가 없다.
- 단점: 새 property id(생성 표 `crates/rbms-skin/src/property/generated/*.rs`, 체크섬 시험 `crates/rbms-skin/tests/property_checksum.rs`), `reference_writes` 판정(`input.rs:188-190`), 스킨 레이아웃 변경까지 범위가 넓다. 라벨만 바꾸면 "안 들린다"는 체감이 그대로 남는다.

### 원인 2: 자동 재생 미리듣기의 키음이 Bg 로 울려 KEY 슬라이더가 선곡에서 안 들림

수정안 (추천, 원인 1 과 함께):
- `apps/rbms-player/src/stage/select/preview.rs` 의 일정 타입을 `Vec<(i64, u32)>` 에서 출처를 담는 형태로 바꾼다(`preview.rs:68`, `:453-461`, `PreviewMsg::Schedule`). `ev.source` 는 이미 `PlayEvent` 에 있다(`crates/rbms-play/src/lib.rs:41-45`). 재생 때 `crate::play_sink::bus_for(source)` 로 버스를 고른다(`play_sink.rs:18-19`).
- `pump_fade` 와 `release_bus` 는 Bg 와 Key 두 버스에 같은 `level` 을 곱하고 되돌린다(`preview.rs:176-200`).
- `#PREVIEW` 파일 클립은 섞인 음원이므로 Bg 그대로 둔다.
- 장점: 미리듣기에서 KEY 와 BGM 슬라이더가 플레이와 같은 뜻으로 들린다.
- 단점: 클립 미리듣기에는 KEY 가 여전히 안 들린다(본질적 한계). 페이드가 두 버스를 다룬다.

### 원인 3 (보조): 슬라이더와 SETTINGS 의 "마스터" 숫자가 다름

원인 1 대안 A 로 해소된다. 대안 C 를 고르면 라벨 변경으로 해소된다.

### 원인 4 (보조): 선형 이득

- 50%→100% 가 +6 dB 뿐이다. 레퍼런스도 선형이고 저장값의 뜻이 바뀌므로 이번 범위에서는 **바꾸지 않기를 권한다.** 필요하면 별도 결정으로 다룬다(곡선 이득을 넣으면 `volume_percent`, `step_volume`, 설정 파일의 뜻이 바뀐다).

### 원인 5 (보조): 페이드 중 슬라이더를 쓰면 한 프레임 동안 Bg 가 전체 값으로 튐

- `carry_out` → `apply_audio_gains` 가 `bg` 를 그대로 넣고(`settings_ui.rs:385`), 다음 프레임 `pump_fade` 가 `bg × level` 로 되돌린다(`preview.rs:186-190`). 슬루가 있어 들리는 정도는 작다. 원인 2 수정 때 `apply_audio_gains` 가 페이드 중인 버스를 건드리지 않게 하거나, 선곡 상태가 페이드 레벨을 곱해 다시 넣게 할 수 있다(선택).

## 3. 영향받는 기존 기능, 시험, 골든

대안 A(rate 17 → master)를 택하면 다음이 바뀐다.
- `apps/rbms-player/src/skin_host/writers/tests.rs:33-41` (`only_the_three_volume_rates_set_a_volume`: 17 → System 단언), `:43-63` (`set_volume` 의 System 경로), `:66-` (`carry_out` 에 RATE_MASTERVOLUME).
- `apps/rbms-player/src/skin_host/system/tests.rs:137-145` (NUMBER 57 / RATE 17 = system 50%).
- `apps/rbms-player/src/skin_host/tests.rs:294` (`NUMBER_MASTER_VOLUME == 50` 은 system 0.5 를 전제. 마스터 기본 1.0 이면 100), `:407`, `:425` (rate 쓰기 기록. 대응 클러스터만 확인하면 그대로일 수 있다).
- `apps/rbms-player/src/skin_screen/tests.rs:709-725` (`a_slider_is_set_by_a_press_and_by_a_drag_and_writes_where_its_document_says`: `config.audio.system` 이 0.5 가 된다고 단언. `VOLUME_RATE = RATE_MASTERVOLUME` `:547`).
- `apps/rbms-cli/src/skin_dump/scenario.rs:73-75` (`volume_system` 등 덤프 키. 숫자 57 출처가 바뀌면 덤프 출력이 달라질 수 있다. 미확인).
- 문서: `docs/reference/skin-compat/b5-properties.md:271`, `:871`(rbms 차이 표기), `docs/skin.md:527`, `docs/quality-assurance/2026-10-11-default-skin-manual-check.md:290` 기대값을 구체화.

원인 2(미리듣기 버스 분리)를 택하면:
- `preview.rs` 안의 시험(`:520-580`, 페이드 레벨과 길이)은 레벨 계산만 보므로 그대로일 가능성이 높다. 일정 타입이 바뀌므로 그 타입을 만드는 시험이 있으면 고친다(미확인: `stage/select/tests.rs` 에서 `sched` 직접 생성 여부).
- 선곡 화면 렌더 골든(`crates/rbms-render/tests/golden_select.rs`)은 소리와 무관하므로 영향이 없다.

기본 스킨 라벨을 바꾸면(대안 C, 또는 A 에서 라벨을 유지할 경우 변경 없음):
- `assets/skins/rbms-default/lua/i18n/select_other.lua:7` 세 언어. 기본 팩 캡처(`docs/skin.md` 5.2 캡처 하니스)의 사이드 메뉴 장면이 달라진다.

## 4. 재현과 확인

- 실행: `cargo test --locked -p rbms-player --lib -- skin_host::writers::tests a_slider_is_set_by_a_press_and_by_a_drag` → **11 passed, 0 failed** (2026-10-29, c1dc988 작업 트리).
  - 확인된 사실: rate 17/18/19 쓰기가 그 프레임 끝에 `config.audio.system/key/bg` 로 들어간다. 대기열에 남지 않는다. 범위가 잘린다.
- 믹서 쪽 즉시 반영은 기존 시험 `bus_gain_scales_only_voices_on_that_bus`, `bus_gain_takes_effect_while_a_voice_is_already_sounding` (`mixer.rs:1575`, `:1589`)이 보증한다(이번에 실행하지는 않음).
- 실기 재현은 하지 않았다(출력 장치 필요). "MASTER/KEY 가 미리듣기에 안 들린다"는 결론은 1.4절의 버스 표에서 나온 코드 경로 근거이며, 확신도는 높음.

## 5. 미확인

- 사용자가 실제로 움직인 슬라이더가 MASTER 인지 KEY 인지. 보고 문구만으로는 알 수 없다. 사용자가 기본 스킨이 아닌 beatoraja 스킨을 쓰는지도 모른다. 외부 스킨도 rate 17~19 를 쓰므로 결론은 같다.
- 사용자가 SOUND FOLDER 를 3초 이상인 `select` 가 든 묶음으로 바꿨는지. 바꿨다면 MASTER 는 선곡 음악에는 들리지만, 미리듣기 중에는 선곡 음악이 0 으로 내려가 있다(`preview.rs:116-119`, `syssound.rs:498`).
- `apps/rbms-cli/src/skin_dump/scenario.rs` 덤프의 골든 파일이 있는지와 숫자 57 변경의 영향.
- IR 설정 동기화(`apps/rbms-player/src/ir_sync.rs:60-76`)는 음량을 기기 로컬로 두지 않는다. 다른 기기의 설정을 내려받으면 음량이 덮어써질 수 있는데, 이것이 체감에 영향을 줬는지는 확인하지 않았다(받는 시점이 선곡 중인지 미확인).

## 6. U4 구현 결과

결정 M11 대로 대안 A(rate 17·NUMBER 57 을 진짜 마스터로 재결합)와 원인 2 의 수정안을 함께 반영했다.

| 슬라이더 | 수정 후 | 선곡에서 들리는 소리 |
| --- | --- | --- |
| MASTER (rate 17, 숫자 57) | `audio.master` | 미리듣기(클립·자동 재생 모두)와 시스템 효과음, 선곡 음악 전부 |
| KEY (rate 18, 숫자 58) | `audio.key` | 자동 재생 미리듣기의 키음. `#PREVIEW` 클립은 섞인 음원이라 해당 없음 |
| BGM (rate 19, 숫자 59) | `audio.bg` | `#PREVIEW` 클립과 자동 재생 미리듣기의 BGM |

- 쓰기: `skin_host/writers.rs` 의 `VolumeTarget { Master, Bus(VolumeBus) }`. `rate_volume(17) = Master`, 18·19 는 기존 버스. 값 범위 보정은 마스터가 `DEFAULT_MASTER_VOLUME`, 버스가 `DEFAULT_BUS_VOLUME` 을 NaN 대체값으로 쓴다. 움직였으면 기존대로 `apply_audio_gains()`(마스터 이득 포함)와 설정 저장 표시. `VolumeBus` 와 Lua `volume_sys`·`set_volume_sys` 는 그대로(시스템 버스).
- 읽기: `skin_host/system.rs` 의 `Volumes` 에 `master` 필드를 더하고 `Volumes::of_audio(&AudioOptions)` 로 두 공급처(`skin_screen.rs`, `skin_screen/preview.rs`)를 하나로 모았다. 숫자 57 과 rate 17 은 `master`, `main_state.volume_sys` 는 `system` 을 읽는다.
- 미리듣기: `stage/select/preview.rs` 의 일정을 `(at_us, wav, source)` 로 바꿔 `PlayEvent.source` 를 보존하고 `play_sink::bus_for` 로 버스를 고른다(키음 Key, BGM Bg). 페이드(`pump_fade`)와 `release_bus` 는 `preview_bus_gains` 한 곳에서 Key·Bg 두 버스에 같은 레벨을 곱하거나 설정값으로 되돌린다. 자동 재생 일정 추출은 `autoplay_preview_schedule` 로 분리했다.
- 기본 스킨 라벨(`select_other.lua` 의 MASTER/마스터/マスター, KEY/키음/キー音, BGM)은 새 동작과 맞아 바꾸지 않았다. SETTINGS 의 MASTER VOL 과 같은 값을 보인다.
- 선형 이득 곡선은 유지했다(원인 4).
- 시험: `writers/tests.rs`(rate 17 -> Master, 마스터 단독 보정, 슬라이더 쓰기가 `audio.master` 만 움직임), `system/tests.rs`·`skin_host/tests.rs`(숫자 57·rate 17 이 마스터, `volume_sys` 는 시스템), `skin_screen/tests.rs`·`typing_tests.rs`(슬라이더가 `audio.master` 에 쓰고 파일에 저장), `stage/select/preview.rs`(페이드가 두 버스에 걸림, 자동 재생 일정이 출처를 보존하고 키음은 Key, 마스터·KEY·BGM 슬라이더 각각이 실제 믹서에서 미리듣기를 줄임).
- 재현: `the_master_key_and_bgm_sliders_each_turn_a_preview_down` 은 믹서에 마스터·버스 이득을 넣어 미리듣기가 끝에서 얼마로 나오는지 잰다. rate 17 을 예전처럼 시스템 값에 쓰게 되돌리면 "MASTER 를 0 으로 해도 미리듣기가 0.25 로 그대로"라며 실패하는 것을 확인했다.
- 남은 것: 원인 5(페이드 중 슬라이더를 끌면 한 프레임 동안 두 버스가 전체 값으로 튐)는 고치지 않았다. 페이드가 도는 200 ms 안에서만 생긴다. 실제 소리 체감은 출력 장치가 있는 실기 확인이 필요하다.
