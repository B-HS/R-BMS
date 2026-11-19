# ADR: 출력 장치가 보고하는 지연을 오디오 시계에 보정한다(macOS 자동, OUTPUT LATENCY AUTO/OFF)

> 최종 갱신 2026-11-19 · 대응 커밋 `b47398a`
> 상태: 유효(코드는 `dev` 에 있음. 사용자가 정한 것은 요구 한 줄뿐이고 아래 M1~M5 와 "구현에서 정한 것"은 모두 메인 또는 구현 단위의 결정이며 사용자 확인 전. 실기(AirPods 로 체감) 확인 전) · 결정일 2026-11-19 · 결정자 사용자(요구)와 메인(M1~M5), 구현 단위 U1(구현에서 정한 것) · 커밋 `b47398a`
> 관계: 요구 1 의 결정이다. 같은 묶음의 [FULLSCREEN](2026-11-19-adr-exclusive-fullscreen.md)이 Game Mode 의 전체 화면 경로와 만난다. 판정 타이밍 보정(JUDGE OFFSET)·자동 보정([play-options.md](../play-options.md) 3절)은 그대로 있고 이 보정과 별개의 값이다. 계획 [plan/2026-11-19-feature-upgrade-12.md](../plan/2026-11-19-feature-upgrade-12.md) U1, 조사 [reference/2026-11-19-feature-upgrade-12/README.md](../reference/2026-11-19-feature-upgrade-12/README.md) 1절, 구현 단위 보고 [history/2026-11-19-feature-upgrade-unit-reports/U1.md](../history/2026-11-19-feature-upgrade-unit-reports/U1.md).

## 배경

- 사용자 요청(2026-11-19) 요지: 블루투스(에어팟 등) 오디오 최적화. 요구 12건의 원문 요지는 [PROCESS.md](../PROCESS.md) 의 "기능 고도화 12건" 체크리스트에 있다.
- 플레이 시계는 오디오 시계다. `audible_us` 는 cpal 이 콜백마다 주는 `playback_ahead`(콜백 시각에서 DAC 까지의 앞섬)로 보간한다. cpal 0.17.3 의 CoreAudio·WASAPI 구현은 이 값에 스트림 버퍼 길이만 넣고 장치가 따로 더하는 지연(`kAudioDevicePropertyLatency`, 안전 오프셋, 블루투스 구간)은 넣지 않는다. 그래서 블루투스 출력에서는 시계가 가리키는 위치가 실제로 들리는 위치보다 장치 지연만큼 앞선다. 사용자가 할 수 있던 보정은 JUDGE OFFSET(±200 ms)뿐이었다.
- 이 Mac 에서 잰 값(2026-11-19): AirPods Pro 2 가 7,680 프레임(48 kHz 에서 160 ms, 안전 오프셋 0, 전송 종류 블루투스), 내장 스피커가 108 프레임(지연 60 + 안전 오프셋 48, 2.25 ms). 장치를 열어 같은 값을 `unsafe` 없이 읽을 수 있는 것을 확인했다.
- 입력 장치(마이크)는 열지 않으므로 블루투스가 통화 프로필(HFP)로 떨어지는 원인은 이 앱에 없다.

## 결정

### 1) 사용자가 정한 것

| 번호 | 결정 |
| --- | --- |
| D1 | 블루투스(에어팟 등) 오디오를 최적화한다. 방법과 범위는 메인에 맡겼다 |

### 2) 메인 결정(사용자 미확인)

| 번호 | 결정 |
| --- | --- |
| M1 | 장치가 보고하는 출력 지연을 오디오 시계에 더해 BGM·노트·판정을 들리는 소리에 맞춘다. AUDIO 탭 OUTPUT LATENCY 행 AUTO(기본)/OFF. OFF 는 이전과 같은 시계다 |
| M2 | macOS 번들 `Info.plist` 에 `LSSupportsGameMode`·`GCSupportsGameMode` 를 true 로 넣는다(`scripts/macos-app.sh`). Game Mode 는 전체 화면에서 OS 가 블루투스 샘플링 속도를 올려 AirPods 지연을 줄이는 기능이고, 켜질지는 OS 가 정한다 |
| M3 | 지연 조회는 `coreaudio-rs`(cpal 의 전이 의존성으로 이미 `Cargo.lock` 에 있던 0.14.2)의 안전한 함수로 한다. `rbms-audio` 에 macOS 한정 직접 의존성 한 줄이 늘고(`[target.'cfg(target_os = "macos")'.dependencies]`, feature `core_audio`·`audio_toolbox`, `default-features = false`), `Cargo.lock` 은 `rbms-audio` 의 의존 목록에 한 줄만 늘었다(새 크레이트 없음). `unsafe` 는 쓰지 않는다. 계획의 "새 의존성 금지" 규칙에 대한 유일한 예외다 |
| M4 | Windows·Linux 는 자동 조회가 없다(cpal 이 API 를 주지 않고 안전한 경로가 없다). AUTO 는 0 ms 로 동작하고 JUDGE OFFSET 으로 맞춘다 |
| M5 | 지연값은 스트림을 열 때와 판을 시작하기 전(`AppShared::load`)에 다시 읽고 판 도중에는 바꾸지 않는다(시계가 끊기지 않게) |

계획서 M1·M4 와 조사 문서는 JUDGE OFFSET 을 "JUDGE TIMING" 이라고 적었다. 같은 값을 스킨 선곡 옵션 패널이 그렇게 부르고(`lua/i18n/select_other.lua`), 설정 화면·PLAY 메뉴의 행 이름은 JUDGE OFFSET 이다. 이 문서와 알림 문구는 행 이름을 쓴다.

### 3) 구현에서 정한 것(구현 단위가 정함, 사용자 미확인)

| 항목 | 내용 |
| --- | --- |
| 더하는 자리 | 계획은 "콜백이 내는 `playback_ahead` 에 더한다"였으나 콜백이 아니라 게임 스레드가 스냅샷을 읽는 쪽(`snapshot_of`)에서 더한다. 로딩 직후 곧바로 `start_play` 하는 경로에서도 콜백 한 번을 기다리지 않고 새 값이 반영되고, 이전 콜백 기록과 섞여 판 도중 가청 위치가 튀지 않는다. `ClockSnapshot.playback_ahead` 에 지연이 포함되는 공개 결과는 계획과 같다 |
| 상한 | 백엔드가 준 앞섬의 200 ms 검사(`MAX_PLAYBACK_AHEAD_MS`)는 백엔드 값에만 건다. 장치 지연은 따로 `MAX_OUTPUT_LATENCY_MS` = 1,000 ms 로 자른다(블루투스 헤드셋은 약 150 ms 를 보고하고 그 몇 배는 장치의 오보고라고 본다) |
| 판 기준점 | `AudioClocks::run_anchor_us` = 가청 위치 + 보정값. `start_play` 가 이것을 판의 곡 위치 0 으로 잡는다. 가청 위치에 그대로 고정하면 차트 첫 160 ms 의 BGM·키음이 이미 렌더된 프레임으로 예약되어 늦게 시작된다. 기준점을 지연만큼 늦추면 곡 위치는 지연 동안 0 에 머문 뒤 진행하고 예약은 제때 걸린다. OFF 에서는 보정값이 0 이라 이전과 같다 |
| 예약 경로 | 예약 위치(프레임 축 `scheduled_us`)는 지연과 무관해 그대로이고 예약 앞섬(`lookahead_us`)만 지연만큼 커진다. 0·160·400 ms 지연에서 `audible_us`·`lookahead_us`·`scheduled_us`·기준점·콜백 경계 연속성을 시험으로 확인했다 |
| 조회 위치 | `rbms_audio::output_device_latency` 는 호스트 스레드(`host.rs` 의 `ask`)에서만 부른다. 장치에 AUHAL 유닛을 열어 읽는 데 수 ms 가 걸려 콜백에서는 부르지 않는다. 읽는 값은 `kAudioDevicePropertyLatency`('ltnc') + `kAudioDevicePropertySafetyOffset`('saft')의 프레임 합과 전송 종류(블루투스 'blue', 블루투스 LE 'blea')이고, 프레임을 시간으로 바꿀 때는 스트림 샘플레이트를 쓴다 |
| 선택자 | `u32::from_be_bytes(*b"ltnc")` 꼴의 이름 있는 상수로 두고(objc2-core-audio 직접 의존을 피함) 조사의 16진 값과 같은지 시험으로 확인한다 |
| 설정 | `AudioOptions.compensate_output_latency: bool`(기본 true, `#[serde(default)]`). `SettingId::OutputLatency` 를 열거 끝에 더해 `SETTING_COUNT` 가 101 이 됐다(행 인덱스가 밀리지 않게). AUDIO 탭 OUTPUT 구획의 POLYPHONY 다음 행. 스트림을 다시 열지 않고, 차트가 스트림을 쥔 화면(Decide·Loading·Play·Result) 밖에서는 `poll_audio_reopen` 이 다음 프레임에 바로 반영한다 |
| 알림 | 스트림을 연 뒤 보정값이 30 ms 이상이면(`OUTPUT_LATENCY_NOTICE_MIN`) 한 번 알린다. 블루투스 장치면 문구가 다르다(`OutputLatencyCompensatedBluetooth`). 장치 이름과 ms 를 넣고 "이 장치 때문에 JUDGE OFFSET 을 옮겨 두었다면 되돌려 달라"고 적는다. 세 언어 |
| 디버그 | SETTINGS DISPLAY 의 DEBUG MODE 를 켜면 `OUTPUT LATENCY 7680 FR 160.0 MS BLUETOOTH  COMPENSATED 160.0 MS` 꼴의 한 줄이 audio 줄 사이에 나온다 |

## 이유

- 보정 없이는 블루투스에서 노트와 판정이 소리보다 160 ms 앞서 간다. 사용자가 JUDGE OFFSET 으로 같은 값을 따로 맞춰야 했고, 장치를 바꿀 때마다 다시 맞춰야 한다.
- 장치가 스스로 알려 주는 값을 쓰면 사용자 입력 없이 맞는다. 값이 틀리거나 장치가 거짓말하는 경우를 위해 OFF 와 JUDGE OFFSET 이 남아 있다.
- 게임 스레드가 읽을 때 더하는 방식은 콜백을 건드리지 않아 실시간 안전성(할당·잠금 없음)이 그대로다.
- 판 도중에 바꾸면 곡 위치가 점프하므로 시작 전에만 읽는다. Game Mode 는 들어가고 나올 때 지연값을 바꾸므로(Apple 개발자 포럼 764070) 판을 시작하기 전에 다시 읽는다.

## 기각된 대안

| 대안 | 기각 이유 |
| --- | --- |
| 수동 값만 두기 | JUDGE OFFSET ±200 ms 가 이미 그 역할을 한다. 장치마다 사용자가 다시 맞춰야 한다 |
| `unsafe` FFI 로 `AudioObjectGetPropertyData` 직접 호출 | 프로젝트 규칙(`unsafe_code = "deny"`, 크레이트 루트 `forbid`) 위반. 안전한 `coreaudio-rs` 가 같은 값을 준다 |
| `windows` 크레이트 직접 호출(Windows) | `unsafe` 가 필요하다. cpal 이 API 를 주지 않아 안전한 경로가 없다 |
| 주기 조회로 판 도중 반영 | 곡 위치가 튀어 노트가 튄다 |
| 콜백 안에서 지연을 더하기 | 콜백이 새로 읽는 값이 없어야 하고, 로딩 직후 시작하는 판이 콜백 한 번을 기다려야 한다 |
| 가청 위치를 판 기준점으로 유지 | 160 ms 장치에서 차트 첫 160 ms 의 소리가 이미 지난 프레임으로 예약된다(엔진 시험이 이 결함과 해결을 함께 보인다) |

## 못 하는 것

- 키음은 키를 누른 뒤에 예약하므로 블루투스 구간 지연만큼 늦게 들린다. 소프트웨어로 줄일 수 없다. 보정은 BGM·노트·판정을 들리는 소리에 맞추는 것까지다. OUTPUT LATENCY 설명 문구에 적었다.
- Windows·Linux 에서는 자동으로 읽을 수 없다. OUTPUT LATENCY 는 있어도 0 ms 로 동작하고 JUDGE OFFSET 으로 맞춘다.
- Game Mode 가 실제로 켜지는지는 OS 가 정하고, macOS 독점 전체 화면(FULLSCREEN)에서 켜지는지는 확인하지 못했다. 문서에는 BORDERLESS(macOS 에서는 시스템 전체 화면)를 Game Mode 경로로 적는다.

## 영향과 관련 파일

- 코드: `crates/rbms-audio/src/latency.rs`(새, `OutputLatency`·`read_output_latency`), `host.rs`(`output_device_latency`), `engine.rs`(`MAX_OUTPUT_LATENCY_MS`, `ClockSnapshot.output_latency`, `AudioClocks.output_latency_us`·`run_anchor_us`, `AudioEngine::{set_output_latency_compensation, refresh_output_latency, device_output_latency, output_latency}`, `AudioOpenReport.output_latency`), `lib.rs`, `Cargo.toml`, `crates/rbms-config/src/{audio,settings,schema}.rs`, `apps/rbms-player/src/{app_play,settings_ui}.rs`, `stage/settings/catalog.rs`, `text/settings/{labels,descriptions}.rs`, `text/notices.rs`, `scripts/macos-app.sh`, `contract/client-requests/settings-blob.json`(audio 에 `compensate_output_latency: true` 한 줄).
- 시험: `rbms-audio` 의 0·160·400 ms 시계, 콜백 경계 연속성, 200 ms 검사 순서, 상한·OFF, 기준점 예약 시뮬레이션(지연별 늦은 예약 수가 0 ms 와 같음), 선택자 값, `latency::tests::every_output_device_answers_or_is_skipped`(장치가 없으면 건너뜀), `rbms-config` 의 `output_latency_compensation_is_on_for_an_older_file_and_flips_without_a_reopen`, `app_play` 의 알림·디버그 줄 시험, `text/notices.rs` 의 `the_output_latency_notice_names_both_rows_as_the_settings_screen_does`.
- 이 Mac 에서 `rbms-player` 를 스크래치 HOME 으로 띄웠을 때 `audio: <AirPods> adds 7680 frames (160.0 ms) over Bluetooth, compensating 160.0 ms` 와 블루투스 알림이 출력됐다(판은 치지 않았다).
- 알려진 위험(테스트 부채 3.163~3.165, 실기 QA 3.29 G1·G2):
  - 실제 판에서 AirPods 로 BGM·노트·판정이 맞는지 체감 확인을 하지 않았다.
  - 기준점 이동으로 판 시작 직후 곡 위치가 보정값만큼(160 ms) 0 에 머문다. READY 다음의 정지 화면이 그만큼 길어지는 모양이고 스킨 PLAY 타이머 기준 애니메이션은 차트 시간보다 그만큼 앞선다.
  - 디버그 지표 `CLOCK quantised gap` 과 soak 기록의 interp-quantized 평균이 보정값만큼 커진다(지표의 뜻 변화, 판정 영향 없음).
  - 예약이 최대 지연만큼 더 앞서 걸려 대기 중인 보이스가 늘어난다(400 ms·초당 60건이면 약 24개, 상한 512 대비 작음).
  - 이전에 이 장치용으로 JUDGE OFFSET 을 옮겨 둔 사용자는 이중 보정이 된다(알림으로 안내).
  - 연습 구간 재시작(`start_practice_slice`)은 `load` 를 거치지 않아 지연을 다시 읽지 않는다.
  - Windows·Linux 컴파일은 로컬에서 확인하지 못했다(이 Mac 에 해당 타깃이 없다). 비 macOS 경로는 `None` 을 돌려주는 함수와 그 시험뿐이고 CI 3 OS 로 확인해야 한다.
  - 새 일본어·한국어 문구(행 이름·설명·알림)는 모델 초안이다.
- 문서: [play-options.md](../play-options.md) 19절, [crates.md](../crates.md) `rbms-audio`·`apps/rbms-player`, 사용자 문구는 [용어집](../reference/2026-10-24-i18n-glossary.md) 6.10절.

## 날짜와 사용자 원문

2026-11-19.

- 사용자 요청 요지(PROCESS 체크리스트): "1 블루투스(에어팟 등) 오디오 최적화". 요청 원문 전문은 이 저장소에 없고 요지만 있다.
- 사용자 지시(진행 방식): 계획·검토는 Fable, 구현은 Opus + workflow, 테스트는 Sonnet + Opus. 방법과 범위(M1~M5)는 사용자에게 따로 묻지 않았다.

## 후속(2026-11-19 테스트·수리 웨이브, 커밋 `d0bff5c`·`4d1ce8e`)

| 항목 | 내용 |
| --- | --- |
| 첫 구간 예약 | 보정 판의 곡 위치가 0 에 머무는 동안 그 멈춘 위치에서 예약하면 지연만큼 더 앞서 예약돼, 같은 키의 앞 소리가 뒤 소리 예약에 끊겼다. 예약 앞섬을 가청 축 기준으로 계산하게 고쳤다(`song_and_booking_lead_us`) |
| 기본 장치 따라가기 | 사용자 질문 "감지하는 로직을 짜야하나" 에 대한 답: 상시 감지는 만들지 않는다. cpal 은 연 장치가 그 순간 시스템 기본이면 CoreAudio DefaultOutput 으로 열어 기본 출력이 바뀌면 소리가 따라간다. 그래서 그런 스트림(`opened_on_system_default`)은 판 시작 전 재조회에서 지금의 기본 장치(`get_default_device_id`)에게 묻는다(`LatencyTarget`). 이름으로 고른 장치가 기본이 아니었으면 이름으로 묻는다 |
| 환산 | 지연 프레임은 장치의 샘플레이트(출력 AU 의 출력 범위)로 시간으로 바꾸고, 읽지 못하면 스트림 샘플레이트를 쓴다 |
| 알림(M21) | (지연을 읽은 장치, 보정 ms)가 마지막 알림과 다르고 30 ms 이상일 때만. 상태는 `AppShared::output_latency_noticed`. 알림 문구는 88자 제한 안에 핵심이 들게 줄였다 |
| 판 기준점 | 이전부터의 결함: 판 기준점이 가청 축이라 차트 첫 약 28 ms 의 소리가 지난 프레임으로 예약됐고, 믹서는 늦은 소리를 건너뛰지 않고 처음부터 틀어 소리 전체가 밀렸다(48 kHz·512·60 Hz 시뮬레이션: 286개 중 5개, 최대 28.7 ms). 기준점을 예약 축으로 옮겼다(`AudioClocks::run_anchor_us(poll)`). 지금은 0·160·400 ms 모두 늦은 소리 0. 판 시작 직후 정지가 약 38 ms 늘고, 제때 예약되던 소리와 차트 시간의 관계는 같다(차이 13 us, 시험으로 고정) |
| 그 밖 | 조회에 1초 시한(넘기면 직전 값), `load()` 가 스트림을 막 연 직후의 중복 조회 제거, PRACTICE 화면을 차트 소리 뱅크를 쥔 화면에 추가(이전부터의 결함), 디버그 줄에 `FROM [DEFAULT ]<장치>` |
| 확인하지 못한 것 | 기본 장치가 실제로 바뀌는 실기, macOS 가 아닌 OS 의 컴파일(CI 로 확인), 같은 이름의 장치 둘 |
