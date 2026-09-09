# Phase B 설계 명세 — 오디오 클럭 재설계 (2026-09-09)

> 대상: 계획 `docs/plan/2026-09-09-enhancement-plan.md` §2 Phase B 1~6항. 관련 갭 행 §1.2 A1·A2·A4·A5·A6·A8·A9·A13, §1.5 P7·P10, §1.7 U11.
> **A13(채널 키에 피치 포함)은 이미 해소됐다** — `crates/rbms-audio/src/mixer.rs:40-48` `channel_key(id, pitch) = id*256 + semitone + 128` (Phase A-audio 완료분). **Phase B 작업 없음.**
> 이 문서는 **설계 명세**다. 구현은 하지 않았다. 구현 에이전트는 §7 파일 소유권 분할과 §3 동결 계약을 그대로 따른다.
> 선행 조건: **Phase I(`crates/rbms-ir`·`apps/rbms-player`) 머지 완료 후 착수.** 이 문서의 `apps/rbms-player` 라인 번호는 Phase I 이전 스냅샷이므로 **함수명으로 앵커**하고 착수 시 재확인한다(각 항목에 "Phase I 이후 재확인" 표기).

---

## 1. 현재 상태 (실측 앵커)

### 1.1 rbms 코드

| 위치 | 현재 동작 |
|---|---|
| `crates/rbms-audio/src/engine.rs:111-117` | `clock_frames()` = 마지막 콜백이 기록한 누적 프레임, `clock_us()` = `frames * 1e6 / out_rate`. **콜백 사이에는 정지값**(A1) |
| `engine.rs:119-125` | `clock_snapshot() -> (u64, Instant)` — Phase A 에서 추가된 (frames, 콜백 벽시각) 페어. **현재 호출부 0** |
| `engine.rs:223-256` | seqlock 3함수 `write_clock_pair` / `try_read_clock_pair` / `read_clock_pair`, `CLOCK_SNAPSHOT_ATTEMPTS = 4`, `CLOCK_SEQ_STEP = 1`. Phase B 는 이 프로토콜을 **그대로 확장 재사용** |
| `engine.rs:285-302` | 출력 콜백. 2번째 인자 `_`(= `&OutputCallbackInfo`)를 **버림** → cpal 타임스탬프 미사용 |
| `engine.rs:174-180` | `play(id, gain, pan, pitch, at_us)` — `at_frame = at_us * rate / 1e6`(절대 프레임) |
| `crates/rbms-audio/src/mixer.rs:144-160` | `Mixer::play` — `delay = at_frame.saturating_sub(self.clock)`. `apply` 는 `mix` 앞에서 실행되므로 `self.clock` = **이번 콜백 시작 프레임**. 스케줄 자체는 이미 정상 |
| `mixer.rs:180-192` | `alloc_slot` — 라운드로빈. 풀이 꽉 차면 커서 위치 보이스를 **가청 여부 무시하고 강탈**, 페이드 없음(A6) |
| `mixer.rs:10,250-254` | `DEFAULT_MASTER_GAIN = 0.5`(레퍼런스 구현 `keyvolume`/`bgvolume` 0.5 를 마스터 1회로 근사), `soft_limit` 적용 완료. **버스 분리·`#VOLWAV` 없음**(A5) |
| `mixer.rs:40-48` | `channel_key(id, pitch) = id*256 + semitone + 128`, `channel_sample_id(key) = key/256` |
| `apps/rbms-player/src/app_play.rs` `song_us()` (스냅샷 197-213) | `audio.clock_us() - anchor_us`, 스트림 사망 시 `resumed_clock_us` 벽시계 폴백. **보간 없음** |
| `app_play.rs` `start_play()` (스냅샷 161-167) | `anchor_us = audio.clock_us()` |
| `app_play.rs` `frame()` 내 `player.update` (스냅샷 457-458) | `audio.play(wav, 1.0, 0.0, 1.0, e.at_us + anchor)` — BGM/autoplay. `player.update(song, play)` 가 `at <= now` 만 방출(`crates/rbms-play/src/lib.rs` `update`, 스냅샷 186-192)이라 항상 `at_frame <= clock` → **delay 0**(A2) |
| `apps/rbms-player/src/main.rs` press 경로 (스냅샷 1264-1272) | `raw = song_us()`, `judge_t = judge_time_us(raw, offset_ms)`, `sound_t = keysound_time_us(raw, anchor_us)`(`main.rs:189-191` = `raw + anchor`). Phase A-app 에서 판정/키음 시각 분리 완료 |
| `apps/rbms-player/src/app_select.rs` `start_preview` / `start_autoplay_preview` (스냅샷 436·471) | 포커스 정착마다 **두 번째 `AudioEngine::new()`** → 별도 cpal 스트림·별도 클럭(A8). `stop_preview` 로 파기 |
| `app_select.rs` `update_preview` (스냅샷 300) | `eng.clock_us()` 기준 자체 앵커·루프 |
| `apps/rbms-player/src/main.rs:707-714` | `SETTING_TABS` = PLAY/GAUGE/JUDGE/DISPLAY/INPUT/NETWORK, 전역 인덱스 0~23. **AUDIO 탭 없음**(A9) |
| `apps/rbms-player/src/app_select.rs` `setting_line`/`adjust_setting` (스냅샷 935·966) | 인덱스 → (라벨, 값) / 증감 |
| `apps/rbms-player/src/settings.rs:10-39` | `PlaySettings` — 오디오 필드 0개 (`pub struct` 10행, 닫는 `}` 39행 — 실측 확인) |
| `apps/rbms-player/src/main.rs:61` | `const PREVIEW_GAIN: f32 = 0.85` — 프리뷰 보이스 게인. 호출부 `app_select.rs:370,386,455` |
| `apps/rbms-player/src/app_input.rs:238` | 리플레이 재생 경로 `audio.play(.., 1.0, 0.0, 1.0, sound_t)` |
| `crates/rbms-play/src/lib.rs:5-9` | `pub struct PlayEvent { pub wav: i32, pub at_us: i64 }` — **버스 판별자 없음**. bg 채널과 `AutoAction::Press` 키음이 같은 타입으로 나온다 |
| `crates/rbms-play/src/lib.rs:184-229` `Player::update(now_us, play)` | **스케줄러가 아니다.** 같은 `now_us` 로 (a) `bg` 커서 방출(185-189), (b) `actions` 커서 = 오토플레이 `judge.press`/`judge.release`(190-220), (c) `AUTO_BEAM_US`(80ms) 빔 소등(221-227), (d) `self.judge.update(now_us)` = **미스 스윕**(228) 을 전부 수행 |
| `crates/rbms-play/src/lib.rs:231-245` `Player::press` | `nearest_head_wav` → `play(PlayEvent{ wav, at_us: now_us })`. 판정 시각과 발음 시각이 같은 인자 |
| `apps/rbms-player/src/app_select.rs:909-929` `to_select_or_exit` | Play·Result 이탈의 **단일 깔때기**. `:916` `self.audio = None` 으로 엔진을 파기한다 |
| `apps/rbms-player/src/main.rs:1234` | Loading 취소 경로. 여기서도 `self.audio = None` |
| `crates/rbms-chart/src/lib.rs:96` `to_model(src, mode)` | `Headers -> ModelMeta` 변환의 **유일한 지점**. `ModelMeta` 리터럴은 `:189-201`, `total: src.headers.total.unwrap_or(0.0)` 는 `:199` |
| `crates/rbms-parser/src/lib.rs:51-71, 236-262` | `Headers` 에 **`volwav` 필드 없음**, 헤더 디스패치에 `"VOLWAV"` 분기 없음 |
| `crates/rbms-model/src/lib.rs:95-110` | `ModelMeta` 에 `volwav` 없음 |
| `app_play.rs` 디버그 오버레이 (스냅샷 770-790) | FPS/RAM/QUADS/TIME/NOTES/COMBO/EX/FAST·SLOW/HISPEED/AUDIO US/ANCHOR/STREAM·DROP·REALLOC/FONT. **보이스 수·언더런·판정 오차 없음**(U11) |

### 1.2 cpal 0.17.3 실제 시그니처 (`~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cpal-0.17.3`)

```rust
// src/lib.rs:528-534, 655-662
pub struct OutputCallbackInfo { timestamp: OutputStreamTimestamp }
impl OutputCallbackInfo {
    pub fn timestamp(&self) -> OutputStreamTimestamp;   // Copy 반환
}

// src/lib.rs:510-520
pub struct OutputStreamTimestamp {
    pub callback: StreamInstant,   // 콜백이 호출된 시각
    pub playback: StreamInstant,   // 이번에 쓴 데이터가 DAC 로 나갈 예측 시각
}

// src/lib.rs:492-495, 576, 591, 602
pub struct StreamInstant { secs: i64, nanos: u32 }   // 백엔드 시계(macOS mach_absolute_time / Windows QPC)
impl StreamInstant {
    pub fn duration_since(&self, earlier: &Self) -> Option<Duration>;  // earlier > self 면 None
    pub fn add(&self, duration: Duration) -> Option<Self>;
    pub fn sub(&self, duration: Duration) -> Option<Self>;
}

// src/lib.rs:395-401, 405-416, 418-426
pub struct StreamConfig { pub channels: ChannelCount, pub sample_rate: SampleRate, pub buffer_size: BufferSize }
pub enum SupportedBufferSize { Range { min: FrameCount, max: FrameCount }, Unknown }
pub struct SupportedStreamConfigRange { /* private */ }

// src/lib.rs:778-786, 810, 825
impl SupportedStreamConfigRange {
    pub fn min_sample_rate(&self) -> SampleRate;
    pub fn max_sample_rate(&self) -> SampleRate;
    pub fn buffer_size(&self) -> &SupportedBufferSize;
    pub fn try_with_sample_rate(self, sample_rate: SampleRate) -> Option<SupportedStreamConfig>;
    pub fn with_max_sample_rate(self) -> SupportedStreamConfig;
}

// src/traits.rs:110, 152, 160, 210 / :50, :83
trait DeviceTrait {
    fn name(&self) -> Result<String, DeviceNameError>;
    fn supported_output_configs(&self) -> Result<Self::SupportedOutputConfigs, SupportedStreamConfigsError>;
    fn default_output_config(&self) -> Result<SupportedStreamConfig, DefaultStreamConfigError>;
    fn build_output_stream<T, D, E>(&self, config, data_callback: D, error_callback: E, timeout: Option<Duration>) -> Result<Self::Stream, BuildStreamError>;
}
trait HostTrait { fn devices(&self) -> Result<Self::Devices, DevicesError>;
                  fn output_devices(&self) -> Result<OutputDevices<Self::Devices>, DevicesError>; }
```

**중요:** `StreamInstant` 는 `std::time::Instant` 와 **다른 시계**다. 상호 변환 API 가 없다. 그래서 §3.1 은 `playback.duration_since(&callback)` 로 **차분(Duration)만** 뽑아 rbms 의 `Instant` 축에 얹는다.

### 1.3 레퍼런스 구현 패리티 (읽기만, 파일명 인용)

| 항목 | 근거 | 값 |
|---|---|---|
| 볼륨 3분리 | `AudioConfig.java:43-54` | `systemvolume = 0.5f`, `keyvolume = 0.5f`, `bgvolume = 0.5f` |
| 버퍼·동시발음·샘플레이트 | `AudioConfig.java:21-32` | `deviceBufferSize = 384`(**:24**), `deviceSimultaneousSources = 256`(**:28**), `sampleRate = 0`(=지정 없음, **:32**). 21·23·26·30 은 주석 |
| 드라이버 | `AudioConfig.java:12-20` | `DriverType.OpenAL` 기본, `driverName` 선택 |
| `#VOLWAV` | `AbstractAudioDriver.java:355-358` | `if (0 < volwav < 200) volume = volwav/100f; else volume = 1.0f` — **차트 로드 시 1회**, 범위 밖이면 1.0 |
| `#VOLWAV` 적용 지점 | `AbstractAudioDriver.java:486-491` | `play(Note n, float volume, int pitch)` → `play0(n, this.volume * volume, pitch)`. 레이어드 노트도 같은 곱 |
| 채널 키 | `AbstractAudioDriver.java:507-509` | `channel(id, pitch) = id * 256 + pitch + 128` (rbms `mixer.rs:40-43` 동일) |
| 판정음 채널 | `AbstractAudioDriver.java:493-505` | `channel = (65536 + judge) * 256` — 판정음은 **id 65536+ 네임스페이스**. §3.3 네임스페이스 분리의 선례 |
| 판정음에도 `#VOLWAV` 적용 | `AbstractAudioDriver.java:502` | `play(sound, channel, volume, 1.0f)` — `volume` 은 `#VOLWAV` 유래 필드. **판정음도 차트 게인을 곱한다** → §3.4 `chart_gain` 은 전 버스 적용(패리티) |
| 램프 | 없음 | 레퍼런스 구현은 `stop` 즉시 컷(OpenAL). **rbms 의 start/stop 램프는 의도적 이탈**(§3.4) |

---

## 2. 목표와 비목표

**목표(Phase B 완료 정의)**
1. 게임 시계가 콜백 주기로 계단화되지 않는다(보간 오차 표준편차 < 1ms, §6.1 하네스로 실측).
2. BGM/autoplay/프리뷰 온셋이 버퍼 경계로 반올림되지 않는다(`delay > 0` 스케줄이 실제로 사용된다).
3. press 키음 지연은 늘어나지 않는다(룩어헤드 예외).
4. 앱 수명 동안 cpal 출력 스트림이 **1개**다.
5. 볼륨이 system/key/bg 3버스 + `#VOLWAV` + 마스터로 분리되고, 보이스 스틸/정지에 클릭이 없다.
6. AUDIO 설정 탭에서 디바이스·버퍼·샘플레이트·동시발음을 바꿀 수 있고, 실패해도 앱이 죽지 않는다.
7. 판정 오차 분포·언더런·보이스 수를 화면에서 볼 수 있고, 10~15분 소크 로그가 남는다.

**비목표(Phase B 범위 밖)**
- 컨트롤러/MIDI/아날로그 스크래치(A10·A11 → Phase D/F), 시스템 사운드·판정음(A12 → Phase F), 다운샘플 안티에일리어싱(A14 → 후순위), `#WAV` slice(bmson 경로, 범위 밖), TimeStretch/FREQUENCY 옵션(레퍼런스 구현 `TimeStretchProcessor.java` — Phase F 이후).

---

## 3. 설계

### 3.0 동결 계약 (병렬 갈래가 서로를 기다리지 않기 위한 사전 합의)

아래 타입·시그니처는 **문자 그대로** 구현한다. 갈래가 달라도 이 텍스트를 기준으로 각자 작성하면 머지 시 컴파일된다.

```rust
// crates/rbms-audio/src/mixer.rs  (소유: B-audio-mixer)
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Bus { System, Key, Bg }

impl Bus { pub const ALL: [Bus; 3] = [Bus::System, Bus::Key, Bus::Bg]; }

#[non_exhaustive]
pub enum Command {
    Play { sample: Arc<SampleData>, gain: f32, pan: f32, pitch: f32, key: u32, at_frame: u64, bus: Bus },
    Stop { key: u32 },
    StopId { id: u32 },
    StopRange { lo_key: u32, hi_key: u32 },   // [lo_key, hi_key) 반열림
    MasterGain(f32),
    BusGain { bus: Bus, gain: f32 },
    ChartGain(f32),                            // #VOLWAV/100, 차트 전환 시 1회
}

pub struct MixStats { pub active_voices: u32, pub steals: u64, pub hard_steals: u64, pub late_schedules: u64 }
impl Mixer { pub fn stats(&self) -> MixStats; }
```

```rust
// crates/rbms-audio/src/engine.rs  (소유: B-audio-clock)
pub struct AudioOptions {
    pub device_name: Option<String>,
    pub sample_rate: Option<u32>,
    pub buffer_frames: Option<u32>,
    pub max_voices: usize,
}
impl Default for AudioOptions { /* device_name: None, sample_rate: None, buffer_frames: None, max_voices: DEFAULT_MAX_VOICES */ }

pub struct AudioOpenReport {
    pub device_name: String,
    pub sample_rate: u32,
    pub channels: u16,
    pub buffer: cpal::BufferSize,
    pub fallback_step: u8,        // 0 = 요청 그대로, 1.. = §3.5 사다리 단계
    pub notes: Vec<String>,       // 사용자에게 보여줄 강등 사유(각 1줄)
}

/// 오디오 콜백이 마지막으로 발행한 시계 상태. seqlock 1회 읽기로 원자적으로 얻는다.
#[derive(Clone, Copy, Debug)]
pub struct ClockSnapshot {
    pub frames_at_callback_start: u64,   // 이번 콜백이 렌더를 시작한 시점의 누적 프레임
    pub frames_rendered: u64,            // 콜백 종료 후 누적 프레임 (= 기존 clock_frames)
    pub callback_at: Instant,            // 콜백 진입 벽시각 (clock_epoch + nanos)
    pub playback_ahead: Duration,        // playback - callback (cpal), 못 구하면 buffer_frames/rate
    pub buffer_frames: u32,              // 이번 콜백의 프레임 수
}

impl AudioEngine {
    pub fn open(opts: &AudioOptions) -> Result<(AudioEngine, AudioOpenReport), AudioError>;
    pub fn new() -> Result<Self, AudioError>;                // 기존 유지 = open(&Default) 의 .0
    pub fn out_rate(&self) -> u32;                           // 기존
    pub fn clock_frames(&self) -> u64;                       // 기존
    pub fn clock_us(&self) -> i64;                            // 기존(계단형, 디버그·회귀용으로 유지)
    pub fn snapshot(&self) -> ClockSnapshot;
    /// 지금(now) DAC 에서 실제로 들리고 있는 위치(µs). 계단 없음.
    pub fn audible_us(&self, now: Instant) -> i64;
    /// 스케줄러가 써야 하는 시각 = audible_us + lookahead_us.
    pub fn scheduled_us(&self, now: Instant) -> i64;
    pub fn lookahead_us(&self) -> i64;
    pub fn is_alive(&self) -> bool;                          // 기존
    pub fn dropped_commands(&self) -> u64;                   // 기존
    pub fn scratch_reallocations(&self) -> u64;              // 기존
    pub fn underruns(&self) -> u64;
    pub fn mix_stats(&self) -> MixStats;                     // 콜백이 원자로 발행한 최신 값
    pub fn load(&mut self, id: u32, bytes: Vec<u8>, ext: Option<&str>) -> Result<(), AudioError>;   // 기존
    pub fn insert_decoded(&mut self, id: u32, audio: DecodedAudio);                                  // 기존
    pub fn loaded(&self) -> usize;                            // 기존
    pub fn has_sample(&self, id: u32) -> bool;                // 기존
    pub fn sample_duration_us(&self, id: u32) -> Option<i64>; // 기존
    pub fn play(&mut self, id: u32, gain: f32, pan: f32, pitch: f32, at_us: i64);       // 기존 시그니처 유지 = play_on(Bus::Key, ..)
    pub fn play_on(&mut self, bus: Bus, id: u32, gain: f32, pan: f32, pitch: f32, at_us: i64);
    pub fn stop(&mut self, id: u32);                          // 기존
    pub fn stop_pitched(&mut self, id: u32, pitch: f32);      // 기존
    pub fn set_master_gain(&mut self, g: f32);                // 기존
    pub fn set_bus_gain(&mut self, bus: Bus, g: f32);
    pub fn set_chart_gain(&mut self, g: f32);                 // #VOLWAV/100
    /// 네임스페이스의 뱅크 엔트리를 제거하고 해당 채널 키 구간의 보이스를 정지.
    pub fn clear_namespace(&mut self, ns: IdNamespace);
}

/// 샘플 id 구간. 채널 키 구간은 [base*256, (base+len)*256).
#[derive(Clone, Copy, Debug)]
pub struct IdNamespace { pub base: u32, pub len: u32 }
impl IdNamespace {
    pub const PLAY: IdNamespace = IdNamespace { base: 0, len: 0x0001_0000 };
    pub const PREVIEW: IdNamespace = IdNamespace { base: 0x0080_0000, len: 0x0001_0000 };
}
```

```rust
// crates/rbms-play/src/lib.rs  (소유: B-play-api)

/// 어느 출력 버스로 보낼 소리인지. rbms-play 는 rbms-audio 에 의존하지 않으므로
/// `Bus` 를 import 하지 않고 자체 판별자를 쓴다. 호출부가 1:1로 매핑한다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaySource {
    /// BGM 채널(자동 반주). -> Bus::Bg
    Bgm,
    /// 노트 키음(오토플레이 `AutoAction::Press`, 또는 `Player::press` 의 히트음). -> Bus::Key
    Key,
}

pub struct PlayEvent {
    pub wav: i32,
    pub at_us: i64,
    pub source: PlaySource,   // 신규 필드. 기존 2필드는 이름·타입 그대로 유지
}

impl Player {
    /// 스케줄 축(= scheduled_us). bg/autoplay 커서를 전진시키며 `play` 를 호출한다.
    /// **판정·빔·미스 스윕을 하지 않는다.**
    pub fn update_schedule<F: FnMut(PlayEvent)>(&mut self, sched_us: i64, play: F);
    /// 판정 축(= audible_us). 오토플레이 press/release 판정, 빔 타이머, 미스 스윕을 수행한다.
    /// `play` 를 호출하지 않는다(발음은 update_schedule 이 이미 예약했다).
    pub fn update_judge(&mut self, audible_us: i64);
    /// 하위호환 유지용. `update_schedule(now_us, play); update_judge(now_us);` 와 동치.
    /// 기존 테스트·분석(가상 시계) 경로 전용이며, 실시간 재생 경로는 쓰지 않는다.
    pub fn update<F: FnMut(PlayEvent)>(&mut self, now_us: i64, play: F);
}
```

```rust
// apps/rbms-player 신규 App 필드 (소유: B-app-clock, main.rs)
// B-app-clock 은 이 필드가 존재한다고 가정하고 app_play.rs / app_select.rs 를 작성한다.
audio: Option<AudioEngine>,          // 기존. 수명이 앱 전체로 확장됨(더 이상 Play 진입 시 생성하지 않음)
audio_report: Option<AudioOpenReport>,
// preview_audio: Option<AudioEngine>  → 삭제
timing: TimingProbe,                 // apps/rbms-player/src/timing.rs (소유: B-app-clock)
song_us_last: std::cell::Cell<i64>,  // 보간 클럭 단조 보장
```

### 3.1 보간 클럭 (계획 §2 Phase B-1)

**원리.** 콜백은 자기 진입 시각(`Instant`)과 그 시점의 누적 프레임, 그리고 cpal 이 알려주는 "이 데이터가 DAC 에 도달할 예측 시각까지의 리드타임"을 한 묶음으로 발행한다. 게임 스레드는 그 묶음에 벽시계 경과를 더해 **가청 위치**를 연속값으로 얻는다.

```
audible_us(now)
  = (snap.frames_at_callback_start as i128 * 1_000_000 / out_rate)
  + signed_micros_between(snap.callback_at + snap.playback_ahead, min(now, snap.callback_at + MAX_EXTRAPOLATION))
```

> **정정 (2026-09-09 적대 리뷰 반영).** 원안은 두 번째 항을 `saturating_duration_since` 로 적었다. 그것으로는 이 절이 없애려던 계단이 그대로 남는다: 모든 백엔드가 `playback_ahead` 를 **최소 버퍼 1주기**로 보고하므로(CoreAudio `host/coreaudio/macos/device.rs:899-908` 는 정확히 1주기, ALSA·WASAPI 는 그 이상, 폴백 경로도 정확히 1주기) 콜백 구간 `[t_N, t_N + 주기)` 전체에서 `now - (callback_at + ahead) <= 0` 이 되어 값이 `frames_at_callback_start` 에 고정된다. 실측 잔차 표준편차 3,080µs = 이론 양자화값 `10667/sqrt(12)`, 즉 구 클럭을 1버퍼 평행이동한 것에 불과했다.
> **부호 있는 차분**을 쓰면 음수 구간(이번 버퍼가 아직 DAC 에 안 나간 구간)이 살아나고, 콜백 경계에서 `F_N - ahead == F_{N-1} + 주기 - ahead` 로 값이 정확히 연속이므로 단조 클램프도 그대로 성립한다. 외삽 상한의 기준점도 §3.1 본문대로 `callback_at` 으로 맞췄다(원 구현은 `callback_at + ahead` 기준이었다).
> 회귀: `engine.rs` `the_interpolated_clock_stays_on_its_line_across_a_device_timeline`(잔차 sd < 1ms, 계단형이면 3.08ms 로 실패), `audible_position_advances_inside_one_callback_period`, `audible_position_is_continuous_across_a_callback_boundary`, 실기 `the_interpolated_clock_is_smooth_on_a_real_device`(`#[ignore]`).
> **실기 실측(MacBook Pro Speakers, 48kHz, 버퍼 512 = 10,666µs): 잔차 sd 12.5µs** — 같은 장비에서 계단형이면 3,079µs. §2.1 합격선(1ms) 통과.

- `frames_at_callback_start` 를 쓰는 이유: `playback` 은 "**이번에 쓰는** 데이터가 나갈 시각"이므로, 그 시각에 DAC 가 재생하는 프레임은 이번 콜백의 **첫 프레임**이다. `frames_rendered`(콜백 종료 후)를 쓰면 버퍼 1개분(실측 10.667ms) 앞서게 되어 A4 스큐가 그대로 남는다.
- `now < callback_at + playback_ahead` 인 구간(= 이번 버퍼가 아직 안 나감)에서는 `saturating_duration_since` 가 0 을 주어 값이 잠깐 평평해진다. 이는 정상이며, 이전 스냅샷과의 연속성은 아래 단조 클램프가 보장한다.
- **외삽 상한**: `now - snap.callback_at > MAX_EXTRAPOLATION` 이면 그 이상 늘리지 않는다. 오디오 스레드가 멈춘 상태에서 시계만 달리는 것을 막는다. 스트림 사망은 기존 `is_alive()` + `resumed_clock_us` 경로가 처리한다.
  ```rust
  const MAX_EXTRAPOLATION_BUFFERS: u32 = 3;   // = 3 * buffer_frames / rate. 실측 512@48k => 32ms
  ```
- **단조 클램프**: 호출부(`app_play.rs::song_us`)가 `song_us_last: Cell<i64>` 로 `value = value.max(last)` 를 적용하고 저장한다. 엔진은 순수 함수로 두어 테스트 가능성을 유지한다.

**원자 레이아웃(seqlock 확장).** `engine.rs:223-256` 의 프로토콜을 유지하되 페이로드를 5개로 늘린다. 시퀀스 카운터 1개는 그대로다.

```rust
struct ClockCell {
    seq: AtomicU64,                     // 짝수 = 안정, 홀수 = 쓰는 중 (기존 CLOCK_SEQ_STEP 규약 유지)
    frames_start: AtomicU64,
    frames_end: AtomicU64,              // 기존 `clock` 필드를 여기로 이동(clock_frames() 이 읽음)
    callback_nanos: AtomicU64,          // clock_epoch.elapsed().as_nanos() as u64
    playback_ahead_nanos: AtomicU64,
    buffer_frames: AtomicU32,
}
```

- 쓰기: `seq += 1` → `fence(Release)` → 5개 `store(Relaxed)` → `seq += 1` `store(Release)`. 기존 `write_clock_pair` 를 `write_clock_state` 로 확장(같은 파일, 같은 규약).
- 읽기: `try_read_clock_state` → 실패 시 `CLOCK_SNAPSHOT_ATTEMPTS`(4) 회 재시도 → 최후에는 각 필드 `Acquire` plain read. **기존 3함수의 테스트 4개**(`read_clock_pair_returns_the_last_completed_pair`, `try_read_clock_pair_rejects_a_half_written_pair`, `concurrent_clock_pair_reads_never_observe_a_torn_pair`, `telemetry_starts_alive_with_zero_counters`)는 필드 확장에 맞춰 갱신하고, **동시성 찢김 테스트는 5필드 전부의 불변식**(예: `frames_end - frames_start == buffer_frames`, `callback_nanos == frames_end * K`)으로 강화한다.
- `clock_frames()`/`clock_us()` 는 `frames_end` 를 읽는 기존 의미를 유지한다(회귀 비교·디버그용).

**`playback_ahead` 산출(콜백 안, 할당 없음).**
```rust
move |data: &mut [T], info: &cpal::OutputCallbackInfo| {
    let ts = info.timestamp();
    let ahead = ts.playback.duration_since(&ts.callback).unwrap_or(fallback_ahead);
    // fallback_ahead = Duration::from_nanos(buffer_frames as u64 * 1_000_000_000 / rate as u64)
    //                  (첫 콜백 전에는 config 의 요청 버퍼, 이후에는 직전 콜백의 실측 프레임 수)
}
```
- `duration_since` 가 `None`(백엔드가 playback < callback 을 보고) 또는 비정상적으로 큰 값(> `MAX_PLAYBACK_AHEAD` = 200ms)을 주면 폴백을 쓰고 `timestamp_fallbacks` 카운터를 증가시킨다. ALSA/WASAPI/CoreAudio 에서 값이 다르게 나오므로 **정상 동작의 일부**로 취급한다.

**언더런 검출.** 콜백 진입 간격이 예상 주기의 `UNDERRUN_GAP_RATIO` 배를 넘으면 카운트한다.
```rust
const UNDERRUN_GAP_RATIO: f64 = 1.5;   // 직전 콜백의 buffer_frames 기준 주기 대비
```

> **정정 (2026-09-09 적대 리뷰 반영).** §8-3 "cpal 이 언더런을 노출하는지 미확인" 은 **노출한다**로 확정됐다: `cpal::StreamError::BufferUnderrun` 을 ALSA(`host/alsa/mod.rs:807`·`:853`·`:1045`)와 JACK(`host/jack/stream.rs:468`)이 xrun 복구 후 올린다. CoreAudio 는 올리지 않는다.
> 따라서 ① **에러 콜백은 변형을 분기해야 한다** — `BufferUnderrun` 은 복구 가능하므로 카운트만 하고 `alive` 를 유지하고, `DeviceNotAvailable`/`StreamInvalidated`/`BackendSpecific` 만 스트림 사망으로 본다(`stream_error_is_fatal`). 분기 없이 전부 사망 처리하면 리눅스/JACK 에서 xrun 1회로 판정 축이 남은 세션 내내 벽시계로 넘어가고 룩어헤드가 0 이 되어 A2 가 재발한다.
> ② 간격 기반 추정은 신호를 주지 않는 백엔드(CoreAudio)를 위한 **보조 수단**으로 격하한다. 두 경로가 같은 카운터를 올리므로 오버레이는 계속 `UNDERRUN~` 로 표기한다.
> ③ 에러 콜백은 오디오 스레드에서 실행되므로 `eprintln!` 을 두지 않는다(stderr 락 + write 시스템콜). 원자 카운터/플래그만 갱신하고, 문구는 게임 스레드가 `is_alive()` 변화를 관측했을 때 1회 출력한다(`app_play.rs::audio_clocks`).
> 회귀: `engine.rs` `a_recoverable_underrun_does_not_kill_the_stream`(4변형 전부).

### 3.2 룩어헤드 (계획 §2 Phase B-2)

**유도.**
1. 게임 스레드가 벽시각 `t` 에 `Command::Play { at_frame }` 를 링에 넣는다.
2. 그 명령을 실제로 보는 것은 `t` **이후에 시작하는 첫 콜백**이다. 그 콜백의 시작 프레임을 `F_next` 라 하면 `F_next >= frames_end(현재)`.
3. `Mixer::play` 는 `delay = at_frame.saturating_sub(self.clock)` 이므로, `at_frame < F_next` 인 명령은 delay 0 으로 붕괴하여 **콜백 경계로 반올림**된다(= 현재 A2 상태).
4. 따라서 스케줄러는 최소 1버퍼 앞을 봐야 한다. 경계 동률(`at_frame == F_next`)에서도 여유를 두기 위해 1프레임을 더한다.

```rust
// engine.rs
const LOOKAHEAD_EXTRA_FRAMES: u32 = 2;
// lookahead_us = snapshot().playback_ahead
//              + (snapshot().buffer_frames + LOOKAHEAD_EXTRA_FRAMES) * 1_000_000 / out_rate
// 호스트는 여기에 자기 폴링 간격(프레임 주기)을 더한다 — apps/rbms-player `schedule_position_us`.
```
버퍼 크기는 **런타임 실측값**(콜백이 발행한 `buffer_frames`)을 쓴다. 고정 상수로 박지 않는다.

> **정정 (2026-09-09 적대 리뷰 반영).** 원안의 유도(위 1~4)는 `at_frame` 이 **audible 축**이 아니라 **mixer clock 축**과 비교된다는 점을 놓쳤다. 소비 시점의 mixer clock 은 audible 위치보다 `playback_ahead + 1버퍼` 앞서 있고, 게임 스레드는 프레임당 1회만 예약하므로 필요한 리드타임은 **`playback_ahead` + 1버퍼 + 프레임 주기**다. 원안의 `1버퍼 + 1프레임`으로는 60Hz 프레임 시뮬레이션에서 예약의 32%(두 결함이 함께 있으면 100%)가 `delay == 0` 으로 붕괴했다.
> `LOOKAHEAD_EXTRA_FRAMES` 를 2 로 올린 것은 µs↔프레임 절삭이 엔진에서 한 번, 믹서에서 한 번 일어나 1프레임 여유를 그대로 먹기 때문이다.
> 프레임 주기 항은 앱이 소유한다(`schedule_poll_interval_us`, 최근 프레임 시간의 피크 홀드 + 감쇠, 1~50ms 클램프). 엔진은 장치 축만 안다.
> 회귀: `engine.rs` `a_frame_paced_host_never_books_an_onset_the_mixer_has_already_passed`(60/120Hz 시뮬레이션에서 `late_schedules == 0`; 구 공식으로 되돌리면 223건 실패), `app_play.rs` `scheduling_leads_the_audible_axis_by_the_device_lead_and_the_frame_period`.
> **알려진 하한:** 곡 시작 직후 `playback_ahead + 1버퍼`(실측 약 21ms) 구간의 온셋은 물리적으로 예약 불가다 — 그 프레임들은 첫 명령이 큐잉되기 전에 이미 DAC 로 넘어갔다. 시뮬레이션은 그 창을 지나서 시작한다.

**적용 지점.**

| 소비자 | 사용할 시각 | 이유 |
|---|---|---|
| `player.update_schedule(sched, play)` (BGM·autoplay **발음 예약**) | `scheduled_us(now) - anchor_us` | 온셋을 정확한 프레임에 앉힌다 |
| `player.update_judge(audible)` (오토플레이 판정·빔·**미스 스윕**) | `audible_us(now) - anchor_us` | §3.6. 여기에 룩어헤드를 넣으면 전 판정이 10.7ms 앞당겨진다 |
| `update_schedule` 콜백의 `at_us` → `audio.play_on(bus, .., e.at_us + anchor)` | 변경 없음(이미 절대 시각) | `at_frame > clock` 이 되어 delay 가 살아난다. `bus` 는 `e.source` 에서 유도(§3.6) |
| 프리뷰 스케줄 방출(`update_preview`) | `scheduled_us(now) - preview_anchor` | 동일 |
| **판정 시각**(`judge_time_us` 입력 `raw`) | `audible_us(now)` | 사람이 들은 것과 비교해야 한다. 룩어헤드를 더하면 전 판정이 10.7ms 밀린다 |
| **press 키음**(`keysound_time_us` → `at_us`) | `audible_us(now) + anchor_us` (= 현행 계산에 보간만 적용) | **즉시 발음 예외**. `at_frame <= clock` 이 되어 delay 0 → 다음 콜백 경계에 즉시 발음. 최소 지연 |
| 노트 렌더 위치 | `audible_us(now)` | 화면은 들리는 것과 맞춘다 |
| 리플레이 기록(`ReplayEvent.t`) | `audible_us(now)` | 재생 시 동일 축 |
| `analysis_us`(수동 분석) | 변경 없음(가상 시계) | |

**즉시 발음 예외의 명시적 불변식:** press 키음 경로는 **절대 `scheduled_us` 를 쓰지 않는다.** `Mixer` 는 `delay == 0` 으로 붕괴한 스케줄을 `late_schedules` 로 센다(§3.0 `MixStats`) — press 키음은 정상적으로 여기에 집계되므로, 오버레이는 press 유래와 BGM 유래를 구분하기 위해 **BGM/autoplay 경로만** 세는 별도 카운터를 쓴다. 구현: `Command::Play` 에 별도 플래그를 추가하지 않고, `Bus::Key` 즉시 발음은 호출부에서 `at_us = 0` 을 넘겨 "지연 없음 의도"를 명시하고, 믹서는 `at_frame == 0` 을 late 집계에서 제외한다.

### 3.3 단일 엔진 + 키 네임스페이스 (계획 §2 Phase B-3)

- `AudioEngine` 을 `App` 이 **한 번만** 연다(`resumed` 최초 진입 또는 첫 사용 시 lazy 1회). `Stage::Play` 진입/이탈, 프리뷰 시작/정지에서 **생성·파기하지 않는다**.
- `App::preview_audio` 필드를 제거하고 `App::audio` 를 공유한다. `app_select.rs` 의 `start_preview` / `start_autoplay_preview` 안의 `AudioEngine::new()` 2곳(스냅샷 436·471)을 삭제한다. **Phase I 이후 재확인**(같은 파일을 Phase I 가 만진다).
- 네임스페이스(§3.0 `IdNamespace`):
  - `PLAY` = `[0, 0x1_0000)` — 차트 wav id. BMS 최대 `#WAVZZ`(base36) = 1295, base62 확장 고려해도 3843 이므로 충분.
  - `PREVIEW` = `[0x0080_0000, 0x0081_0000)` — 프리뷰. `PREVIEW_ID`(파일 프리뷰 단일 클립)는 `IdNamespace::PREVIEW.base`, autoplay 프리뷰 키음은 `PREVIEW.base + wav_id`.
  - 상한 검증: `channel_key` 는 `id * 256 + bias` 이므로 최대 키 = `0x0081_0000 * 256 = 0x8100_0000` < `u32::MAX`. 오버플로 없음(`wrapping_mul` 유지).
  - 레퍼런스 구현의 판정음 네임스페이스(`(65536 + judge) * 256`, `AbstractAudioDriver.java:493-503`)와 동일한 발상이며, 향후 시스템 사운드용으로 `SYSTEM = [0x0082_0000, 0x0083_0000)` 을 예약해 둔다(Phase F).
- 전환 규칙:
  - 프리뷰 시작 → `clear_namespace(IdNamespace::PREVIEW)` 후 로드.
  - `Stage::Play` 진입 → `clear_namespace(IdNamespace::PREVIEW)`(프리뷰 정지) + `clear_namespace(IdNamespace::PLAY)`(이전 차트 잔재 제거) 후 키음 디코드 삽입.
  - `Stage::Play` 이탈 → `clear_namespace(IdNamespace::PLAY)`. **뱅크를 비우므로 Phase A-render 의 메모리 회귀와 동일한 상한 성질을 유지**한다(엔진 수명이 길어져도 뱅크는 무제한 누적되지 않는다).
- `anchor_us`: 엔진이 앱 수명 동안 살아 있으므로 `start_play()` 의 `anchor_us = audio.clock_us()`(스냅샷 164)는 **`audible_us(Instant::now())` 로 바꾼다**. 프리뷰도 같은 축을 쓰므로 `preview_anchor` 계산이 동일해진다.

### 3.4 볼륨 모델 · `#VOLWAV` · 리미터 · 보이스 스틸 · 램프 (계획 §2 Phase B-4)

**신호 체인 (보이스 → 출력)**
```
sample * voice_gain * env * pan_gain          (보이스)
  * bus_gain[bus]                              (System / Key / Bg, 기본 0.5 — AudioConfig.java:46-54)
  * chart_gain                                 (#VOLWAV/100, 기본 1.0 — AbstractAudioDriver.java:355-358)
  * master_gain                                (사용자 마스터, 기본 1.0)
  -> soft_limit()                              (mixer.rs:54, 기존)
```

- **기존 `DEFAULT_MASTER_GAIN = 0.5` 는 제거하고 `master_gain` 기본값을 1.0 으로 올린다.** 0.5 는 원래 레퍼런스 구현의 per-voice `keyvolume`/`bgvolume` 를 마스터 1회로 근사한 값이었고(현 `mixer.rs:4-10` 주석), 이제 `bus_gain` 이 그 역할을 정확히 맡는다. **최종 진폭은 변하지 않는다**(0.5 × 1.0 = 1.0 × 0.5). 회귀 테스트로 이 등가성을 고정한다.
- `chart_gain` 은 차트 로드 시 1회 `set_chart_gain(volwav_factor)`. 값은 `if 0 < volwav < 200 { volwav as f32 / 100.0 } else { 1.0 }` — 레퍼런스 구현과 **경계 조건까지 동일**(0 과 200 은 제외).
- **`chart_gain` 적용 범위 = 전 버스(System/Key/Bg).** 레퍼런스 구현은 노트 발음(`AbstractAudioDriver.java:486-491` `this.volume * volume`)뿐 아니라 **판정음에도 같은 `volume` 을 곱한다**(`:502` `play(sound, channel, volume, 1.0f)`). 따라서 Phase F 의 `Bus::System`(시스템 사운드·판정음)도 `chart_gain` 을 받는다. 구현상 `chart_gain` 은 버스 합산 **이후**에 곱하는 단일 스칼라이므로 분기가 필요 없다.
- **호출부 게인 리터럴(A5) 처분** — `bus_gain` 도입 후에도 per-voice 게인은 "같은 버스 안에서의 상대 밸런스" 용도로 남긴다.
  | 호출부 | 현행 | Phase B 후 | 근거 |
  |---|---|---|---|
  | Play BGM·autoplay (`app_play.rs`) | `1.0` | `1.0` 유지 | 기준 레벨 |
  | press 키음 (`main.rs` 스냅샷 1272) | `1.0` | `1.0` 유지 | 기준 레벨 |
  | 리플레이 재생 (`app_input.rs:238`) | `1.0` | `1.0` 유지, 버스만 `Bus::Key` | press 와 동일 소리 |
  | 프리뷰 (`PREVIEW_GAIN`, `main.rs:61` = `0.85`) | `0.85` | **`0.85` 유지** | 곡선택 프리뷰를 의도적으로 살짝 낮춘 값이지 버스 볼륨이 아니다. `Bus::Bg` 로 가므로 최종 진폭은 `0.85 × bg 0.5 × master 1.0` = 현행 `0.85 × master 0.5` 와 동일 |
- `#VOLWAV` 파싱(신규):
  - `crates/rbms-parser/src/lib.rs` `Headers` 에 `pub volwav: i32`(`Default` = 100) 추가, 디스패치에 `"VOLWAV" => h.volwav = rest.parse().unwrap_or(100),` 추가(기존 `"TOTAL"` 행 `lib.rs:252` 인접).
  - `crates/rbms-model/src/lib.rs` `ModelMeta` 에 `pub volwav: i32` 추가(`Default` derive 는 0 을 주므로, chart 변환부에서 `headers.volwav` 를 그대로 넣고 **0 은 "미지정"이 아니라 레퍼런스 구현 규칙상 1.0** 으로 해석한다).
  - `crates/rbms-chart` 의 `Headers -> ModelMeta` 변환에 1줄 추가.
- **버스 배정**
  | 소리 | 버스 |
  |---|---|
  | press 키음, autoplay 노트 키음(`AutoAction::Press`) | `Bus::Key` |
  | BGM 채널(`bg` 스케줄) | `Bus::Bg` |
  | 프리뷰(파일·autoplay 양쪽) | `Bus::Bg` |
  | (Phase F) 시스템 사운드·판정음 | `Bus::System` |
- **램프 상수**
  ```rust
  // mixer.rs
  const ATTACK_MS: f32 = 1.0;    // 시작 클릭 제거. 최단 키음(수 ms)도 손상하지 않는 길이
  const RELEASE_MS: f32 = 3.0;   // stop / 재트리거 / 보이스 스틸 공통 페이드아웃
  ```
  - `Voice` 에 `env: f32`, `env_step: f32`, `phase: VoicePhase { Attack, Sustain, Release }` 추가. 프레임마다 `env` 를 갱신하고, `Release` 에서 `env <= 0` 이면 `active = false; sample = None`.
  - `Mixer::stop(key)` / `stop_id` / `stop_range` 는 즉시 컷 대신 `phase = Release` 로 바꾼다. `Mixer::play` 의 같은 키 재트리거(`mixer.rs:145` `self.stop(key)`)도 Release 로 바뀌므로 **꼬리가 겹친 채 새 보이스가 시작**된다 → 슬롯 1개를 더 쓴다(폴리포니 여유 512 로 충분).
  - `Release` 중인 보이스는 `stop_*` 재호출 시 무시(이미 감쇠 중).
- **보이스 스틸 정책** (`alloc_slot` 교체)
  1. 비활성 슬롯 → 즉시 시작.
  2. `Release` 단계 중 `env` 가 가장 작은 슬롯 → **그 슬롯에 예약**(`steals += 1`).
  3. 전부 `Attack`/`Sustain` → **현재 진폭 `gain * env` 가 가장 작은** 보이스, 동률이면 `start_frame` 이 가장 오래된 보이스에 **예약**(`hard_steals += 1`).
  4. 남은 슬롯이 전부 이미 예약을 물고 있으면 이번 Play 를 버린다(무음이 클릭보다 낫다).
  - 라운드로빈 커서(`alloc_cursor`)는 제거한다. 순회 비용은 O(voices)=512 로, 콜백당 명령 수는 수십 개 수준이라 무시 가능(소크에서 콜백 시간으로 검증, §6.3).

> **정정 (2026-09-09 적대 리뷰 반영).** 원안의 2·3 단은 슬롯을 **제자리 덮어쓰기**로 가져가므로 두 경우 모두 한 프레임 만에 다른 파형으로 튄다 — 실측 경계 스텝 **0.3248 풀스케일**(2단·3단 동일). §2 완료정의 5("보이스 스틸/정지에 클릭이 없다")와 §3.4 램프 도입 취지에 정면으로 어긋난다. 원안 주석의 "마지막 단만 클릭 가능"도 사실이 아니다: 2단은 `env` 가 가장 작은 것을 고를 뿐이고, 페이드 중인 보이스가 하나뿐이면 `env≈1.0` 이다.
> **해결: 스틸을 "즉시 교체"가 아니라 "피해 보이스의 페이드 뒤에 예약"으로 바꿨다.** `Voice.pending: Option<PlayRequest>` 에 새 요청을 얹고, 릴리스가 무음에 닿는 **그 프레임에 같은 슬롯에서 제자리 시작**한다(`start_in_place`, 샘플 정확). 아직 발음 전인 보이스를 스틸하면 슬롯이 즉시 비므로 지연 없이 시작한다. 예약을 물고 있는 슬롯은 다시 스틸 대상이 되지 않아 예약된 소리가 밀려나지 않는다. `stop`/`stop_id`/`stop_range` 는 같은 키 범위의 예약도 함께 취소한다(`clear_namespace` 의미 유지).
> 대가는 풀이 꽉 찬 경우에 한해 새 소리가 최대 `RELEASE_MS`(3ms) 늦게 시작한다는 것뿐이다.
> 회귀: `taking_a_slot_from_an_audible_voice_does_not_step_the_output`, `taking_a_slot_from_a_fading_voice_does_not_step_the_output`(둘 다 경계 스텝 < 0.01; 구 구현으로 되돌리면 0.3248 로 실패), `a_queued_start_still_sounds_after_the_victim_has_faded`, `a_slot_that_already_has_a_queued_start_is_not_taken_again`, `stopping_a_key_cancels_a_start_queued_on_it`.

- **오디오 콜백의 메모리 해제 금지 (신규, 적대 리뷰).** `Mixer` 는 보이스가 끝날 때 `Arc<SampleData>` 를 **드롭하지 않고** rtrb 링(`Mixer::set_retire`)으로 게임 스레드에 돌려보내고, 게임 스레드가 `AudioEngine::collect_retired`(모든 명령 push·`insert_decoded` 경로)에서 드롭한다. 링이 가득 찬 경우에만 콜백이 직접 드롭하고 `retire_overflows` 를 올린다(유계·희소).
  이유: `clear_namespace` 는 게임 스레드에서 `bank.retain(...)` 으로 뱅크 참조를 먼저 버리고 `StopRange` 만 큐잉하므로, 아직 울리고 있는 보이스가 **마지막 소유자**가 되고 `RELEASE_MS` 뒤 콜백 안에서 수 MB PCM 이 free 된다. 실측: 전역 계수 할당자로 콜백 경로 8회에 `deallocations 2, freed 192,056 bytes`. 트리거 지점은 Play 진입(`released_namespaces(true)`), Play 이탈, 프리뷰 교체(커서 이동마다)로 전부 상시 경로다.
  회귀: `crates/rbms-audio/tests/rt_safety.rs`(전역 계수 할당자로 `mix`/`apply` 경로 alloc·dealloc 0 고정 + 은퇴한 샘플 전량이 링으로 돌아오는지 확인; 링을 빼면 54건 dealloc 으로 실패).

- **게인 슬루 (신규, 적대 리뷰).** `MasterGain`/`BusGain`/`ChartGain` 은 즉시 대입이 아니라 `GAIN_SLEW_MS`(3ms) 동안 선형으로 이동한다. 이동 시간은 변화량과 무관하게 일정하다(변경 시점에 프레임당 증분을 계산). 정상상태 값은 그대로이므로 §3.4 진폭 등가성 회귀는 유지된다. 회귀: `a_gain_change_ramps_instead_of_stepping`, `a_gain_change_reaches_its_target_within_one_slew`.
- **리미터**: 기존 `soft_limit`(`SOFT_LIMIT_THRESHOLD = 0.8`, tanh 니) 유지. 변경 없음. 다만 `master_gain` 이 1.0 으로 오르므로 **리미터 진입 빈도가 실제로 늘어나는지**를 소크에서 관측한다(피크 홀드 카운터 `limiter_engaged_frames`).

### 3.5 오디오 설정 + 오픈 폴백 사다리 (계획 §2 Phase B-5)

**설정 행(신규 AUDIO 탭).** `SETTING_TABS`(`main.rs:707-714`)에 `("AUDIO", &[24, 25, 26, 27, 28, 29, 30, 31])` 를 **NETWORK 뒤에** 추가한다. Phase I 가 NETWORK 탭에 22·23 이후 인덱스를 추가할 예정이므로, **Phase B 는 24 부터가 아니라 "Phase I 병합 후 가장 큰 인덱스 + 1" 부터 할당**한다(착수 시 재확인, 아래 표의 숫자는 잠정).

| 잠정 idx | 라벨 | 값 표기 | `adjust_setting` 동작 |
|---|---|---|---|
| 24 | `MASTER VOL` | `{0..100}%` | ±5%, clamp 0~100 |
| 25 | `KEY VOL` | `{0..100}%` | ±5% |
| 26 | `BGM VOL` | `{0..100}%` | ±5% |
| 27 | `SYSTEM VOL` | `{0..100}%` | ±5% |
| 28 | `AUDIO DEVICE` | 디바이스명 or `DEFAULT` | `host.output_devices()` 목록 순환(+ `DEFAULT`) |
| 29 | `BUFFER SIZE` | `AUTO / 128 / 192 / 256 / 384 / 512 / 768 / 1024 / 2048` | 목록 순환. `384` 는 레퍼런스 구현 기본값(`AudioConfig.java:24`) |
| 30 | `SAMPLE RATE` | `AUTO / 44100 / 48000 / 88200 / 96000` | 목록 순환(`AudioConfig.java:32` 의 `0 = 지정 없음` = `AUTO`) |
| 31 | `POLYPHONY` | `{64..1024}` | ±64, clamp. 기본 512(현 `DEFAULT_MAX_VOICES`), 레퍼런스 구현 기본 256(`AudioConfig.java:28`) |

- 24~27 은 **즉시 반영**(`set_bus_gain`/`set_master_gain`, 스트림 재오픈 없음).
- 28~31 은 **스트림 재오픈 필요**. 값 변경 시 즉시 재오픈하지 않고 `AUDIO DEVICE` 행 아래에 `APPLY (RESTART AUDIO)` 를 별도 행으로 두지 않는다 — 대신 **행에서 벗어날 때(포커스 이동/탭 이동/Settings 이탈) 1회 재오픈**한다(디바운스 `AUDIO_REOPEN_DEBOUNCE = 400ms`). 재오픈 중에는 `Stage::Play` 가 아닐 때만 수행하고, Play 중이면 "다음 곡부터 적용" 을 표시한다.
- `PlaySettings`(`settings.rs:10-39`)에 추가:
  ```rust
  pub audio_device: Option<String>,   // None = 시스템 기본
  pub audio_buffer_frames: Option<u32>,
  pub audio_sample_rate: Option<u32>,
  pub audio_polyphony: usize,         // default 512
  pub vol_master: f32,                // default 1.0
  pub vol_key: f32,                   // default 0.5
  pub vol_bg: f32,                    // default 0.5
  pub vol_system: f32,                // default 0.5
  ```
  전부 `#[serde(default)]` 규약을 따라 기존 `settings.ron` 을 깨지 않는다.

**폴백 사다리(`AudioEngine::open`).** 각 단계 실패 시 다음으로 내려가며, `AudioOpenReport.fallback_step` 과 `notes` 에 1줄씩 기록한다.

| step | 시도 | 실패 시 사유 예 |
|---|---|---|
| 0 | 요청 디바이스 + 요청 샘플레이트 + `BufferSize::Fixed(요청)` | — |
| 1 | 요청 디바이스 + 요청 샘플레이트 + `BufferSize::Default` | 버퍼가 `SupportedBufferSize::Range{min,max}` 밖 |
| 2 | 요청 디바이스 + `default_output_config()` | `try_with_sample_rate()` 가 `None` |
| 3 | 시스템 기본 디바이스 + `default_output_config()` | 요청 디바이스명이 `output_devices()` 에 없음 / 열기 실패 |
| 4 | `host.output_devices()` 를 순서대로 각각 `default_output_config()` | 기본 디바이스가 죽어 있음 |
| 5 | `Err(AudioError::NoDevice)` | 출력 장치 0개 |

- step 5 는 기존 "visual only" 경로(`app_play.rs` 스냅샷 85-89 의 `Err(e) => println!("audio unavailable ...")`)를 그대로 탄다. 게임은 `song_us()` 벽시계 폴백으로 계속 진행한다.
- 요청 검증은 `supported_output_configs()` 를 순회해 `min_sample_rate()..=max_sample_rate()` 와 `buffer_size()` 를 확인한 뒤 `try_with_sample_rate()` 로 확정한다(§1.2 시그니처).
- 재오픈 실패 시 **직전에 성공한 설정으로 되돌려 다시 연다.** 그것마저 실패하면 step 3 부터 다시 내려간다. 어떤 경우에도 `App` 이 오디오 없이 패닉하지 않는다.
- 폴백이 일어났으면 AUDIO 탭 하단에 `notes` 를 표시한다(예: `requested 96000 Hz -> device max 48000 Hz`).

> **정정 (2026-09-09 적대 리뷰 반영).**
> - **`fallback_step` 하한**: 요청 디바이스명을 `output_devices()` 에서 못 찾아 시스템 기본으로 내려갔는데도 그 뒤 포맷이 요청대로 열리면 step 이 0~2 로 보고돼 위 표의 step 3 과 어긋났다. 디바이스 폴백이 실제로 일어났으면 보고 step 을 `STEP_DEFAULT_DEVICE` 이상으로 승격한다(`reported_step_floor`). 회귀: `losing_the_requested_device_is_reported_as_a_device_fallback`.
> - **AUDIO 탭 상태 줄**: `notes` 는 stdout 으로만 나가고 화면 어디에도 없었다. 설정 화면의 status 줄을 AUDIO 탭이 소유하게 해서 ① 실제로 열린 디바이스·레이트·채널, ② 강등 사유(`notes`), ③ 차트가 스트림을 쥐고 있어 재오픈이 보류 중이라는 안내를 표시한다(`settings_ui::audio_status_text`). 회귀 4건.
> - **재오픈 디바운스**: 원 구현은 첫 변경에만 데드라인을 걸고 갱신하지 않아 "마지막 변경 후 400ms" 가 아니라 "첫 변경 후 400ms" 에 발화했고, 한 행을 여러 번 밟으면 스트림이 그 횟수만큼 닫혔다 열렸다. 대기 중에는 매 프레임 데드라인을 밀어낸다(`audio_reopen_deadline`).
> - **재오픈 보류 범위**: `Stage::Play` 만 막으면 Loading 중 재오픈이 디코드된 키음 뱅크와 `#VOLWAV` 게인을 통째로 버린다. 차트가 스트림을 쥐고 있는 모든 구간(Play/Loading/`player` 보유/키음 디코드 중)에서 보류한다(`audio_reopen_blocked`). 재오픈 성공 후에는 로드된 차트의 `chart_gain` 을 다시 밀어 준다.
> - **AUDIO DEVICE 행 열거**: 좌우 스텝마다 호스트를 열거하던 것을 설정 화면 진입 시 1회 캐시로 바꿨다(`App::open_settings`). 동명 장치 구분(cpal `Device::id()`)은 Phase C 로 남긴다.

### 3.6 `rbms-play` 축 분리 (신규 — 룩어헤드가 판정을 오염시키지 않게)

**문제.** `Player::update(now_us, play)`(`crates/rbms-play/src/lib.rs:184-229`)는 이름과 달리 스케줄러가 아니다. 하나의 `now_us` 로 네 가지를 동시에 한다.

| 하는 일 | 라인 | 있어야 할 축 |
|---|---|---|
| `bg` 커서 방출 (BGM 발음 예약) | 185-189 | **scheduled** (룩어헤드 포함) |
| `actions` 커서 = 오토플레이 키음 발음 예약 | 195 | **scheduled** |
| 오토플레이 `judge.press` / `judge.release` | 196-215 | **audible** (사람이 듣는 축) |
| 빔 on/off 타이머(`AUTO_BEAM_US`) | 199-218, 221-227 | **audible** (화면) |
| `self.judge.update(now_us)` = **미스 스윕** | 228 | **audible** |

§3.2 표대로 `scheduled_us(now) - anchor_us` 를 그대로 넣으면 **미스 판정과 오토플레이 판정·빔이 1버퍼(실측 10.69ms) 앞당겨져** `audible_us` 로 그리는 노트·판정과 어긋난다. 룩어헤드가 판정 축으로 새는 것(R6)이 press 경로가 아니라 **여기서** 일어난다.

**해결.** `update` 를 두 함수로 쪼갠다(§3.0 동결 계약). 커서 전진(`bg_cursor`/`action_cursor`)은 `update_schedule` 이, 판정 상태(`judge`/`beam_*`/`ln_active`/`bomb`)는 `update_judge` 가 소유한다.

- 오토플레이 판정 시각은 **커서의 `at`(악보상 절대 시각)** 을 그대로 쓰므로(`judge.press(lane, at)`, 현행과 동일) 축 분리로 값이 바뀌지 않는다. 다만 `at <= now_us` 판별을 어느 축으로 하느냐가 바뀌므로, `update_judge` 는 **자체 커서**(`judge_cursor`)로 `actions` 를 audible 축에서 다시 훑는다. 즉 `actions` 에 커서가 2개(`action_cursor` = 발음 예약용, `judge_cursor` = 판정용)가 되고 `judge_cursor <= action_cursor` 가 불변식이다.
- `bomb`/`beam_on`/`beam_off`/`ln_active` 갱신은 전부 `update_judge` 로 이동한다.
- `Player::press`(231-245)는 축이 하나뿐인 즉시 경로라 시그니처를 바꾸지 않는다. 호출부가 판정 시각(`judge_t`)을 넘기고, 발음 시각은 `PlayEvent.at_us` 를 무시하고 §3.2 의 즉시 발음 규칙(`at_us = 0`)으로 덮어쓴다.
- 하위호환 `update` 는 남겨 두어 **기존 테스트(`autoplay_collect` 계열, `lib.rs:464`)와 수동 분석(가상 시계) 경로가 그대로 통과**하게 한다.

**버스 판별자.** `PlayEvent` 에 `source: PlaySource { Bgm, Key }` 를 추가한다. 이것이 없으면 §3.4 '버스 배정' 표를 호출부에서 구현할 수 없다(bg 채널과 오토플레이 키음이 동일 구조체로 나온다). 매핑은 호출부 1줄:

```rust
let bus = match e.source { PlaySource::Bgm => Bus::Bg, PlaySource::Key => Bus::Key };
```

방출 지점별 값: `lib.rs:187`(bg) → `Bgm`, `lib.rs:195`(`AutoAction::Press`) → `Key`, `lib.rs:240`(`Player::press` 히트음) → `Key`.

**호출부 변경(B-app-clock).**
```rust
// app_play.rs::frame() — 기존 player.update(song, play) 1회를 2회로
let sched = audio.scheduled_us(now) - self.anchor_us;
let audible = audio.audible_us(now) - self.anchor_us;
player.update_schedule(sched, |e: PlayEvent| {
    let bus = match e.source { PlaySource::Bgm => Bus::Bg, PlaySource::Key => Bus::Key };
    audio.play_on(bus, e.wav.max(0) as u32, 1.0, 0.0, 1.0, e.at_us + self.anchor_us);
});
player.update_judge(audible);
```
`update_schedule` 이 먼저다(발음 예약이 판정보다 앞서야 커서 불변식 `judge_cursor <= action_cursor` 가 유지된다).

---

## 4. 검증 설계 (계획 §2 Phase B-6)

### 4.1 판정 오차 실측 하네스

**신규 파일 `apps/rbms-player/src/timing.rs`.** 순수 로직 + 링버퍼로 두어 단위 테스트가 가능하게 한다.

```rust
pub const TIMING_SAMPLES: usize = 4096;

#[derive(Clone, Copy, Default)]
pub struct TimingSample {
    pub input_at_us: i64,        // 입력 처리 시점의 audible_us
    pub quantized_us: i64,       // 같은 시점의 계단형 clock_us (비교군)
    pub judge_delta_us: i64,     // JudgeResult.delta_us (있을 때만)
    pub frames_at_start: u64,
    pub buffer_frames: u32,
}

pub struct TimingProbe { /* 링버퍼 + 카운터 */ }
impl TimingProbe {
    pub fn push(&mut self, s: TimingSample);
    pub fn stats(&self) -> TimingStats;
    pub fn write_csv(&self, path: &std::path::Path) -> std::io::Result<()>;
}

#[derive(Clone, Copy, Default)]
pub struct TimingStats {
    pub n: usize,
    pub interp_minus_quantized_mean_us: f64,   // 보간이 계단형 대비 얼마나 앞/뒤인지
    pub interp_minus_quantized_sd_us: f64,
    pub interp_minus_quantized_max_us: i64,
    pub judge_delta_mean_us: f64,
    pub judge_delta_sd_us: f64,
    pub judge_delta_p95_abs_us: i64,
}
```

- 수집 지점: `main.rs` press 경로(스냅샷 1264-1272)에서 `raw`(보간) + `audio.clock_us() - anchor`(계단형) + `hit.delta_us` 를 함께 push. **Phase I 이후 재확인.**
- 결과 화면 진입 시 `--timing-csv <path>` 인자(또는 `debug` 모드)로 CSV 를 덤프한다.
- **합격 기준**: 보간 클럭이 벽시계에 대한 **최소자승 직선에서 벗어나는 잔차의 표준편차 < 1ms.**

> **정정 (2026-09-09 적대 리뷰 반영).** 원안의 "계단형 대비 보간의 잔차"는 지표가 될 수 없다. `input_at_us`(= `audible_us`)와 `quantized_us`(= `clock_us`)는 **같은 레코드**의 `frames_at_callback_start` 와 `frames_end` 에서 나오므로 둘의 차이는 보간 품질과 무관하게 항상 정확히 −1버퍼이고, 그 표준편차는 구조적으로 0 이다. 즉 §4.3 의 `interp_sd < 1000µs` 게이트는 실패 상태에서도 무조건 통과했다.
> **`interp_sd_us` 를 "벽시계 대비 보간 클럭의 선형 잔차 sd" 로 재정의한다**(`timing::ClockFit`, Welford 온라인 공분산이라 소크 전 구간을 메모리 없이 요약한다). 기울기가 장치 클럭과 시스템 클럭의 상수 배율 차를 흡수하므로 드리프트는 오차로 세지 않는다. 계단형 클럭이면 이 값이 `buffer_us / sqrt(12)` ≈ 3.08ms 로 나와 게이트에 걸린다.
> 표본은 **입력이 아니라 Play 프레임마다** 넣는다 — §4.3 의 소크는 autoplay 무인 실행이라 키 입력이 0건이다. `input_at_us − quantized_us` 는 진단용 `quantised gap` 으로 CSV·오버레이에 남긴다.
> 회귀: `timing.rs` `the_clock_fit_separates_an_interpolated_clock_from_a_quantised_one`, `the_clock_fit_absorbs_a_constant_rate_difference_between_the_two_clocks`.

### 4.2 언더런/드롭 카운터 오버레이

`app_play.rs` 디버그 오버레이(스냅샷 770-790)의 `Stage::Play` 블록에 3줄 추가:

```
AUDIO   {audible_us} US  ANCHOR {anchor}  LOOKAHEAD {lookahead_ms:.2} MS
STREAM  {ALIVE|DEAD}  DROP {dropped}  REALLOC {realloc}  UNDERRUN~{underruns}  RETIRE-OF {retire_overflows}
VOICES  {active}/{max}  STEAL {steals}/{hard}  LATE {late}  TS-FB {timestamp_fallbacks}
JUDGE   n={n}  d(mean) {mean:+.2}MS  sd {sd:.2}MS  p95 {p95:.2}MS
CLOCK   n={clock_n}  fit sd {sd:.3}MS  quantised gap {gap:+.2}MS
```

- `Stage::Select` 에서도 프리뷰가 같은 엔진을 쓰므로 `VOICES`/`UNDERRUN` 줄은 항상 표시한다.
- 오버레이는 `config.debug` 게이트 유지(기존과 동일).
- **U11(판정 ms-off 분포) 처분**: 화면에는 mean/sd/p95 3수치만 띄우고, **분포 원본은 `TimingProbe` 의 per-sample CSV(§4.1 `write_csv`)로 전량 덤프**한다(4096 샘플 링버퍼). 즉 스캐터/히스토그램은 오프라인에서 그릴 수 있다. **화면 내 히스토그램 위젯은 Phase F 로 미룬다**(스킨/오버레이 개편과 함께). 이 축소는 의도된 결정이며 계획 §1.7 U11 은 Phase B 에서 "수치 + 원본 데이터"까지만 충족한다.

### 4.3 10~15분 소크 절차

> 개인 규칙 guidelines §20("장시간 파이프라인은 리뷰 전에 소크")에 따라, **라이브 첫 동작 확인 직후·적대 리뷰 전에** 게이트로 실행한다.

절차:
1. `cargo build --release`.
2. `RBMS_SOAK_LOG=/tmp/rbms-soak.csv ./start.sh --autoplay <장곡>` 로 **autoplay 무한 반복**(곡 종료 시 결과 → 재시작). 최소 12분.
3. 병행 세션 2종을 각각 1회: (a) 곡선택 화면에서 5초 간격 커서 이동(프리뷰 반복 로드/해제 = 네임스페이스 누수 검출), (b) Play ↔ Select 왕복 20회(엔진 수명 유지 검증).
4. 앱이 60초 간격으로 아래 CSV 1행을 append 한다(신규, `timing.rs` 옆 `soak_log` 함수 또는 오버레이와 동일 소스).

```
ts_iso,stage,uptime_s,rss_mb,fps_avg,frame_ms_p95,active_voices,underruns,drops,steals,hard_steals,late,ts_fallbacks,retire_overflows,interp_sd_us,judge_sd_us
2026-09-09T12:00:00Z,Play,60,182.4,119.6,9.1,37,0,0,4,0,0,0,0,0.41,8.9
```

> `retire_overflows` 는 적대 리뷰에서 추가된 컬럼이다(§3.4 정정 — 오디오 콜백의 메모리 해제 금지). 오버레이 STREAM 줄의 `RETIRE-OF` 와 같은 값이다.

> **위 데이터 행은 컬럼 형식 예시이며 값은 전부 임의다.** 실측 baseline 이 아니다. (§1.2 의 512프레임/10.667ms 만 실측값이다.)

**합격 기준**
| 지표 | 기준 |
|---|---|
| RSS 추세 | 마지막 5분 선형회귀 기울기 < 0.5 MB/분, 최종값이 시작 대비 < +15% |
| fps | 평균이 목표 대비 -5% 이내, `frame_ms_p95` < 16.7ms(60Hz 기준) |
| underruns | 12분 누적 0 (1 이상이면 버퍼 사다리 재조정) |
| dropped_commands | 0 |
| scratch_reallocations | 첫 콜백 이후 0 |
| hard_steals | 0 |
| interp_sd | < 1000 µs (벽시계 대비 선형 잔차 sd — §4.1 정정 참조. 계단형이면 약 3,080µs) |
| retire_overflows | 0 (오디오 콜백이 샘플 PCM 을 직접 free 한 횟수) |

- 실패 시 **리뷰로 넘어가지 않고** 원인 수정 → 소크 재실행.
- 감시 스크립트를 쓸 경우 PID 는 `timeout`/래퍼가 아닌 실제 프로세스를 잡는다(guidelines §20).

---

## 5. 단계별 구현 순서 + 단계별 테스트

각 단계는 **`cargo test --workspace` 통과 상태로 끝난다.**

| # | 단계 | 산출 | 테스트 |
|---|---|---|---|
| B0 | `PlayEvent.source` 추가 + `update` → `update_schedule`/`update_judge` 분리(`judge_cursor` 도입) | `crates/rbms-play/src/lib.rs` | 기존 `autoplay_collect`(`lib.rs:464`) 계열 전부 통과(하위호환 `update` 경유); 신규: `update_schedule(t)` 는 판정을 진행시키지 않는다, `update_judge(t)` 는 `play` 를 호출하지 않는다, `judge_cursor <= action_cursor` 불변식, 두 축을 10ms 어긋나게 넣었을 때 미스 스윕 시각이 audible 축을 따른다 |
| B1 | seqlock 5필드 확장 + `ClockSnapshot` + `audible_us`/`scheduled_us`/`lookahead_us` | `engine.rs` | 기존 seqlock 테스트 3개를 5필드 불변식으로 확장; `audible_us` 순수 계산 테스트(경계: `now < callback+ahead` → 평평, 외삽 상한 클램프, `out_rate = 0` 방어); `lookahead_us`(512@48k = 10687µs ±1) |
| B2 | cpal 타임스탬프 수용(`info.timestamp()`), `playback_ahead` 폴백, 언더런 추정, `MixStats` 발행 | `engine.rs` | `duration_since` → `None` 폴백 단위 테스트(순수 함수로 분리: `fn ahead_or_fallback(Option<Duration>, u32, u32) -> Duration`); 언더런 판정 함수 테스트 |
| B3 | `Bus`/`StopRange`/`BusGain`/`ChartGain`, 램프, 새 `alloc_slot`, `MixStats` | `mixer.rs` | 진폭 등가성 회귀(`master 0.5 × bus 1.0` == `master 1.0 × bus 0.5`); Attack/Release 램프가 단조·경계에서 0/1; 스틸 우선순위 3단계 각각; `StopRange` 반열림 경계; `delay>0` 스케줄이 정확한 프레임에서 발음(기존 `plays_and_advances_clock` 계열 확장) |
| B4 | `#VOLWAV` 파싱·모델 전파 | `rbms-parser`, `rbms-model`, `rbms-chart` | `#VOLWAV 0/1/99/100/199/200/201/-5/abc` 파싱 + `chart_gain` 환산 경계(0 과 200 은 1.0) |
| B5 | `AudioOptions`/`open`/폴백 사다리/`AudioOpenReport`/`clear_namespace` | `engine.rs`, `lib.rs` | 사다리 선택 로직을 디바이스 비의존 순수 함수(`fn pick_config(supported: &[SupportedStreamConfigRange], req: &AudioOptions) -> Option<(u32, BufferSize)>`)로 분리해 테스트; `clear_namespace` 키 구간 산출 테스트. 실기 테스트는 기존 `#[ignore]` 관례 유지 |
| B6 | `PlaySettings` 오디오 필드 + AUDIO 탭 + `setting_line`/`adjust_setting` + 재오픈 디바운스 | `settings.rs`, `main.rs`, `app_input.rs` | 기존 `settings.ron` 라운드트립(신규 필드 default); 각 행 증감 클램프; 탭 인덱스 범위 |
| B7 | 단일 엔진 수명 + 프리뷰 네임스페이스 이전 + `song_us` 보간·룩어헤드 분리 | `app_play.rs`, `app_select.rs` | `song_us` 단조성 테스트(순수 함수 `fn monotonic(prev: i64, cur: i64) -> i64`); 프리뷰 ↔ Play 전환 시 `clear_namespace` 호출 순서(로직 함수로 분리) |
| B8 | `TimingProbe` + 오버레이 3줄 + 소크 로그 | `timing.rs`(신규), `app_play.rs` | `TimingStats` 통계 계산(평균/σ/p95) 고정 입력 테스트; CSV 포맷 테스트 |
| B9 | 소크 12분 실행 → 합격 확인 → 적대 리뷰 → 수정 → `cargo fmt`/`test`/`clippy` 게이트 | — | §4.3 |

---

## 6. 병렬 브랜치 분할 (파일 소유권)

> **규칙: 동시에 진행되는 두 브랜치가 같은 파일을 만지지 않는다.** 아래는 브랜치별 **파일 전체 목록**이며, 여기 없는 파일은 그 브랜치가 수정하지 않는다. 각 Wave 끝에 소유권 교집합이 공집합임을 명시한다.

### Phase B 가 건드리는 전체 파일 (기존 12개 + 신규 2개)

`crates/rbms-audio/src/engine.rs`, `crates/rbms-audio/src/lib.rs`, `crates/rbms-audio/src/mixer.rs`, `crates/rbms-play/src/lib.rs`, `crates/rbms-parser/src/lib.rs`, `crates/rbms-model/src/lib.rs`, `crates/rbms-chart/src/lib.rs`, `apps/rbms-player/src/main.rs`, `apps/rbms-player/src/app_play.rs`, `apps/rbms-player/src/app_select.rs`, `apps/rbms-player/src/app_input.rs`, `apps/rbms-player/src/settings.rs`, `apps/rbms-player/src/settings_ui.rs`(신규), `apps/rbms-player/src/timing.rs`(신규) + Wave 3 문서 3종.

**`decode.rs` 정정 (2026-09-09 적대 리뷰 반영):** 원안은 `crates/rbms-audio/src/decode.rs` 를 "어느 브랜치도 열지 않는다" 로 적었으나, §3.0 동결 계약이 `Command::Play` 에 `bus` 필드를 추가한 이상 이 파일의 테스트는 그것 없이 컴파일되지 않는다. 실제 변경은 2줄(`use` 1줄 + `Command::Play { .., bus: Bus::Bg }` 1줄)이다.
→ **정정: `decode.rs` 는 디코딩 경로 무변경. `Command` 시그니처 변경에 따른 테스트 적응만 허용하며, 소유는 B-audio-mixer.**

### Wave 1 (동시 착수 4갈래)

| 브랜치 | 소유 파일 (배타·전체 목록) | 담당 단계 |
|---|---|---|
| **B-audio-clock** | `crates/rbms-audio/src/engine.rs`<br>`crates/rbms-audio/src/lib.rs` | B1, B2, B5 |
| **B-audio-mixer** | `crates/rbms-audio/src/mixer.rs` | B3 |
| **B-volwav** | `crates/rbms-parser/src/lib.rs`<br>`crates/rbms-model/src/lib.rs`<br>`crates/rbms-chart/src/lib.rs` | B4 |
| **B-play-api** | `crates/rbms-play/src/lib.rs` | B0 |

**소유권 검증 (Wave 1):** 4갈래의 파일 집합은 서로 서로소다 — `{engine.rs, lib.rs(rbms-audio)}` ∩ `{mixer.rs}` ∩ `{parser/lib.rs, model/lib.rs, chart/lib.rs}` ∩ `{play/lib.rs}` = ∅. 교집합 없음.

**착수 중 빌드 가능성(중요).** Wave 1 은 "머지 시 정합"은 §3.0 동결 계약이 보장하지만 **착수 중 각자 빌드되지는 않는다.** `B-audio-clock` 의 B2(`MixStats` 발행)·`play_on` 은 `mixer.rs` 소유인 `Bus`/`Command::Play{bus}`/`MixStats` 정의를 필요로 한다. 그래서 Wave 1 은 **선행 0단계**를 둔다.

> **Wave 0 (직렬, Fable, 30분 이내):** `mixer.rs` 에 §3.0 의 `Bus`·`Command` 확장·`MixStats`·`Mixer::stats()` **선언만** 넣어(구현은 기존 동작 그대로 유지하는 최소 스텁 — `bus` 무시, `stats()` 는 0 반환) `cargo build --workspace` 를 통과시킨 뒤 통합 브랜치에 머지한다. Wave 1 의 4갈래는 이 커밋에서 분기한다. `B-audio-mixer` 가 B3 에서 그 스텁을 실제 구현으로 채운다.

Wave 1 종료 시 통합 컴파일 1회: `cargo build -p rbms-audio -p rbms-chart -p rbms-play`.

### Wave 1.5 (직렬, Fable — 순수 이동 커밋, 동작 변경 0)

Wave 2 의 두 갈래가 `app_select.rs`/`main.rs` 를 공유하지 않도록, **설정 UI 를 먼저 파일로 분리**한다. 이 커밋은 리팩터링 전용이며 로직을 바꾸지 않는다(`cargo test --workspace` 그린 유지).

- `apps/rbms-player/src/settings_ui.rs` 신규 생성.
- `app_select.rs` 의 `setting_line`(스냅샷 935)·`adjust_setting`(스냅샷 966) 을 그대로 이동.
- `main.rs` 의 `SETTING_TABS`(스냅샷 707-714)와 `SETTING_KEYCONFIG`/`SETTING_FONT`/`SETTING_SERVER_URL`/`SETTING_PLAYER_ID` 상수(스냅샷 700-703)를 그대로 이동하고 `pub(crate)` 로 재노출.
- `main.rs` 에 `mod settings_ui;` 추가.

이동 후에는 `main.rs`/`app_select.rs` 에 설정 UI 코드가 남지 않으므로 Wave 2 의 소유권이 깨끗하게 갈린다.

### Wave 2 (Wave 1 + 1.5 머지 후 동시 착수 2갈래)

| 브랜치 | 소유 파일 (배타·전체 목록) | 담당 단계 |
|---|---|---|
| **B-app-settings** | `apps/rbms-player/src/settings.rs`<br>`apps/rbms-player/src/settings_ui.rs`<br>`apps/rbms-player/src/app_input.rs` | B6 |
| **B-app-clock** | `apps/rbms-player/src/main.rs`<br>`apps/rbms-player/src/app_play.rs`<br>`apps/rbms-player/src/app_select.rs`<br>`apps/rbms-player/src/timing.rs`(신규) | B7, B8 + §3.0 의 신규 `App` 필드 **선언** |

**소유권 검증 (Wave 2):** `{settings.rs, settings_ui.rs, app_input.rs}` ∩ `{main.rs, app_play.rs, app_select.rs, timing.rs}` = ∅. 교집합 없음.

**경계 규칙(원안에서 뒤집힘).**
- `App` 구조체 필드 추가·삭제(`audio_report`, `timing`, `song_us_last`, `preview_audio` 제거)는 **B-app-clock 단독** 권한이다. `App` 은 `main.rs` 에 있고, `main.rs` 의 press 경로·Loading 취소 경로가 전부 B-app-clock 의 작업이기 때문이다. B-app-settings 는 그 필드가 이미 있다고 가정하고 **읽기·호출만** 한다.
- 설정 행 정의(`SETTING_TABS`·`setting_line`·`adjust_setting`)와 `PlaySettings` 필드는 **B-app-settings 단독** 권한이다(`settings_ui.rs`/`settings.rs`).
- 두 갈래의 접점은 **함수 시그니처 2개뿐**이며 §3.0 동결 계약에 준해 아래 텍스트로 고정한다.
  ```rust
  // settings_ui.rs (B-app-settings 가 작성) — B-app-clock 이 호출만 한다
  impl App {
      /// 설정에서 파생한 오디오 오픈 파라미터. B-app-clock 이 엔진 오픈/재오픈 시 호출한다.
      pub(crate) fn audio_options(&self) -> rbms_audio::AudioOptions;
      /// 24~31 행 중 스트림 재오픈이 필요한 항목이 바뀌었는지. B-app-clock 이 디바운스 타이머를 건다.
      pub(crate) fn audio_reopen_pending(&self) -> bool;
      pub(crate) fn clear_audio_reopen_pending(&mut self);
  }
  ```

**B-app-clock 의 `main.rs` 작업 목록(원안 누락분 — 실측 앵커).**

| 지점 | 현행 | 변경 |
|---|---|---|
| `main.rs` 스냅샷 1263 | `let raw = self.song_us();` | 보간 클럭(`audible_us` 기반, §3.1) 값으로 |
| `main.rs` 스냅샷 1269 | `self.recording.push(ReplayEvent { t: raw, .. })` | `raw` 가 audible 축이 되므로 값 자체는 그대로, **축 주석 갱신**(§3.2 표) |
| `main.rs` 스냅샷 1272 | `audio.play(e.wav.., sound_t)` | `audio.play_on(Bus::Key, e.wav.., 0)` — §3.2 즉시 발음 예외(`at_us = 0`) |
| `main.rs` 스냅샷 1264-1272 | — | §4.1 `TimingSample` push 추가 |
| `main.rs:1234` | `self.audio = None`(Loading 취소) | `self.audio` 유지 + `clear_namespace(PLAY)` + `clear_namespace(PREVIEW)` |
| `main.rs` 스냅샷 707-714 | `SETTING_TABS` | Wave 1.5 에서 이미 `settings_ui.rs` 로 이동 — **B-app-clock 은 손대지 않는다** |

**B-app-clock 의 `app_select.rs` 작업 목록.** `update_preview`(300)·`start_preview`(396)·`start_autoplay_preview`(467)·`stop_preview`(556)·`to_select_or_exit`(909-929, 특히 `:916` `self.audio = None` → `clear_namespace(PLAY)`). Wave 1.5 이후 이 파일에 설정 UI 는 없다.

**B-app-settings 의 `app_input.rs` 작업.** `:238` 리플레이 재생 `audio.play(..)` → `audio.play_on(Bus::Key, ..)` 1줄. (게인 `1.0` 유지 — §3.4 표)

### Wave 3 (직렬, Fable)

| 단계 | 내용 |
|---|---|
| 통합 | `cargo fmt --check`, `cargo test --workspace`, `cargo clippy --workspace --all-targets` — **baseline 은 Wave 0 착수 직전 커밋에서 직접 측정해 기록한다.** (참고: 계획 §2 G-04 는 커밋 `c6f0885` 기준 테스트 889 통과로 적혀 있다. 이 명세 초안의 "1,060 / clippy 51 / 94" 수치는 출처가 불명해 폐기했다.) 기준은 절대 수치가 아니라 **측정한 baseline 대비 악화 0** 이다 |
| 라이브 | 실기 1회 가청 확인(키음 즉시성·BGM 온셋·클릭 없음·프리뷰 전환) |
| 소크 | §4.3, 12분 |
| 리뷰 | 적대 리뷰(오디오 RT 안전성 / 클럭 정확성 / 앱 통합·UX) 3갈래 → 수정 |
| 문서 | `docs/acknowledge/reference-divergences.md` 에 B 절 추가(램프·마스터 게인 재배치·룩어헤드·축 분리), `docs/PROCESS.md` Phase B 체크 갱신, `docs/history/2026-09-09-phase-b-audio-clock.md` |

**소유권 검증 (Wave 3):** 소스 파일 소유 없음(문서 3종 단독). 다른 Wave 와 시간적으로 겹치지 않는다.

### 파일 길이 (Phase C 로 이월)

적대 리뷰 시점 실측: `main.rs` 1,562 · `app_play.rs` 1,236 · `app_select.rs` 847 · `keyconfig.rs` 829 로 800줄 기준을 넘는 파일이 4개다(신규 `timing.rs` 634 · `settings_ui.rs` 448 은 기준 이하). Wave 1.5 는 설정 UI 만 분리하도록 정의했고 `main.rs`/`app_play.rs` 분해는 Phase C 범위다. **Phase C 분리 목록에 `app_play.rs` 의 오디오 수명·재오픈·소크/오버레이 블록**(`ensure_audio`/`open_audio_with`/`reopen_audio`/`poll_audio_reopen`/`release_play_audio`/`debug_audio_lines`/`write_soak_row`/`push_clock_sample`)을 명시해 둔다 — 성격상 `timing.rs` 쪽이다.

---

## 7. 리스크

| # | 리스크 | 영향 | 완화 |
|---|---|---|---|
| R1 | `StreamInstant` 가 `Instant` 와 다른 시계라, `playback - callback` 차분만으로는 백엔드 편차를 다 못 잡는다 | 가청 위치 상수 오차 | 상수분은 기존 `JUDGE OFFSET`/AUTO CAL 이 흡수. σ(지터)만 목표로 삼는다. `TS-FB` 카운터로 폴백 빈도 관측 |
| R2 | ALSA/WASAPI 에서 `duration_since` 가 `None` 또는 비현실적 값 | 룩어헤드 오산 | `MAX_PLAYBACK_AHEAD = 200ms` 상한 + 버퍼 기반 폴백. 3 OS CI 에서 `#[ignore]` 실기 테스트로 수동 확인 |
| R3 | 마스터 게인 0.5 → 1.0 재배치에서 실수하면 **전체 음량 2배** | 청각 피해 | 진폭 등가성 회귀 테스트(B3)를 게이트로. 라이브 확인은 볼륨 낮춘 상태에서 시작 |
| R4 | 램프 도입으로 보이스 점유 시간이 늘어 스틸 빈도 증가 | 음 끊김 | `RELEASE_MS = 3.0` 으로 짧게. `steals`/`hard_steals` 를 소크 지표에 포함 |
| R5 | 엔진 수명 연장으로 뱅크가 누적 | RSS 증가 | `clear_namespace` 를 Stage 전환마다 호출. 소크 (a)(b) 세션이 이걸 정확히 겨냥 |
| R6 | 룩어헤드가 판정 축까지 새면 전 판정 +10.7ms | 정확성 회귀 | `audible_us` / `scheduled_us` 를 **이름으로 분리**하고, press 경로에 `scheduled_us` 사용을 금지하는 주석 대신 **테스트**(리플레이 재시뮬 회귀)를 둔다 |
| R7 | Phase I 가 `main.rs`·`app_select.rs` 를 동시 수정 중 | 충돌 | Phase B 는 Phase I 머지 후 착수(PROCESS 실행 순서 I → B). 착수 시 **함수명** 기준 재확인 — 이 문서의 `apps/rbms-player` 라인 번호는 전부 "스냅샷" 이다. `crates/` 라인 번호는 Phase I 범위 밖이라 그대로 유효 |
| R8 | Wave 1.5(`settings_ui.rs` 분리)를 건너뛰면 Wave 2 두 갈래가 `app_select.rs`·`main.rs` 를 공유 | 머지 충돌 | **Wave 1.5 는 선택이 아니라 필수 선행 단계**(§6). 순수 이동 커밋이라 리스크가 낮다. 건너뛸 경우 Wave 2 를 직렬화한다 |
| R9 | `alloc_slot` O(voices) 선형 탐색을 콜백에서 수행 | 콜백 시간 증가 | 512 × 명령 수십 개. 소크의 `frame_ms_p95` 와 `underruns` 로 검증. 초과 시 진폭 최소 힙 도입 |
| R10 | **엔진 수명을 앱 전체로 늘리면 A7(스트림 사망) 노출면이 Select 화면까지 넓어진다.** 현재 벽시계 폴백은 `app_play.rs:197-212` `song_us()` 에만 있고 `update_preview`(`app_select.rs:300`)에는 없다 | 스트림이 죽으면 **프리뷰가 정지**하고 복구되지 않는다 | B7 에서 `update_preview` 의 시계 읽기를 `song_us()` 와 **동일한 폴백 헬퍼**(보간 + `is_alive()` 체크 + `resumed_clock_us`)로 통일한다. 헬퍼는 `app_play.rs` 에 `pub(crate)` 로 두고 `app_select.rs` 가 호출(둘 다 B-app-clock 소유라 충돌 없음). 소크 세션 (a) 로 검증 |
| R11 | `rbms-play` API 분리(B0)가 `apps/rbms-player` 와 기존 테스트로 파급 | 컴파일 파손·판정 회귀 | 하위호환 `update` 를 남겨 기존 호출부·테스트를 그대로 통과시키고, 실시간 경로만 Wave 2 에서 2함수로 전환. `judge_cursor <= action_cursor` 불변식을 단위 테스트로 고정 |
| R12 | `judge_cursor` 도입으로 `actions` 를 두 번 훑게 되어 상태 이중 소유 실수 가능 | 오토플레이 판정 누락/중복 | `update_schedule` 은 `judge`/`beam_*`/`bomb`/`ln_active` 를 **읽지도 쓰지도 않는다**(컴파일 경계로 강제할 수 없으므로 리뷰 체크 항목 + 두 축을 어긋나게 넣는 테스트로 검출) |

---

## 8. 미확인 사항

> 이 절의 항목은 **추측하지 않고 남겨 둔 것**이다. 구현 착수 전에 해당 근거를 직접 확인한다.
> 2026-09-09 비평 반영 시 3·4·8 을 코드/레포 확인으로 해소해 §8.1 로 옮겼다.

1. **레퍼런스 구현의 `PCM`/`AudioDriver` 세부** — `AudioDriver.java`·`PCM.java`·`FloatPCM.java`·`ShortPCM.java`·`GdxSoundDriver.java`·`PortAudioDriver.java` 는 **열지 않았다**(시간 상한). 확인한 것은 `AudioConfig.java`(볼륨 3분리·버퍼 384·동시발음 256·샘플레이트 0)와 `AbstractAudioDriver.java`(`#VOLWAV` 355-358, `play(Note,volume,pitch)` 486-491, 판정음 493-505·볼륨 곱 502, `channel()` 507-509)뿐이다. **보이스 스틸 정책·페이드 유무·리샘플 방식은 미대조** — §3.4 의 스틸/램프는 rbms 독자 설계이며 레퍼런스 패리티 주장이 아니다.
2. **`deviceSimultaneousSources = 256`(`AudioConfig.java:28`) 이 OpenAL 소스 수인지 rbms 의 소프트 보이스와 1:1 대응하는지** 미확인. §3.5 의 `POLYPHONY` 기본 512 는 현행 `DEFAULT_MAX_VOICES` 유지 결정이지 패리티가 아니다.
3. ~~**언더런의 정확한 측정 수단**~~ — **해소(2026-09-09 적대 리뷰).** cpal 0.17.3 은 `StreamError::BufferUnderrun` 을 노출한다(ALSA `host/alsa/mod.rs:807`·`:853`·`:1045`, JACK `host/jack/stream.rs:468`). CoreAudio 는 올리지 않는다. §3.1 정정 참조 — 에러 콜백이 변형을 분기하고, 간격 추정은 신호를 주지 않는 백엔드용 보조로 격하됐다.
4. **디스플레이 지연** — 계획 §5 한계에 이미 "미실측"으로 남아 있다. 시청각 스큐 총합은 Phase B 로도 확정되지 않는다(오디오 축만 정확해진다).
5. **`SETTING_TABS` 최종 인덱스** — Phase I 가 NETWORK 탭에 몇 개를 추가하는지 미확정. §3.5 표의 24~31 은 **잠정값**이며, 착수 시 "가장 큰 기존 인덱스 + 1" 부터 재할당한다. **Phase I 이후 재확인.** (현행 최대는 `SETTING_PLAYER_ID = 23`, `main.rs` 스냅샷 703.)
6. **`master_gain` 1.0 상향 후 리미터 진입 빈도** — 이론상 동일 진폭이지만, `bus_gain` 이 보이스 단에 곱해지므로 리미터 이전 합산 지점이 달라진다. 실제로 다른지 소크 `limiter_engaged_frames` 로 **측정 후 판단**한다(현재 미측정).
7. **`apps/rbms-player` 내부 라인 번호** — Phase I 가 동시 수정 중이라 불안정하다. 이 문서는 **함수명 앵커 + "스냅샷 NNN" 표기**를 쓴다. `crates/` 쪽 라인 번호는 Phase I 범위 밖이라 실측 그대로 신뢰할 수 있다.

### 8.1 해소된 항목 (2026-09-09 확인)

| 원 항목 | 확인 결과 |
|---|---|
| `#VOLWAV` 의 rbms-chart 변환 지점 | **`crates/rbms-chart/src/lib.rs:96` `pub fn to_model(src: &BmsSource, mode: Mode) -> Model` 단 하나.** `ModelMeta` 리터럴은 `:189-201` 이고 `total: src.headers.total.unwrap_or(0.0)` 가 `:199` 다. B4 는 그 리터럴에 `volwav: src.headers.volwav,` 1줄을 추가하면 된다. `ModelMeta` 를 만드는 다른 지점은 이 파일에 없다(`grep -n ModelMeta` = `:1` import, `:189` 리터럴). → **B-volwav 의 소유 파일 3개 목록이 이 확인으로 충분함이 증명됐다.** |
| `Stage::Play` 이탈 지점 전수 | **깔때기 1개 + 방출 3개로 전수 확인.** ① 곡 종료 → `app_play.rs:469` `enter_result()` → `app_play.rs:392` `stage = Stage::Result`. ② Play 중 Esc, 전 노트 판정 완료 → `main.rs:1247` `enter_result()`. ③ Play 중 Esc, 미완료 → `main.rs:1249` `to_select_or_exit()`. ④ Result 에서 Esc/Enter → `main.rs:1220` `to_select_or_exit()`. **엔진 파기는 `app_select.rs:916`(`to_select_or_exit` 내부 `self.audio = None`) 한 곳뿐**이며, 그 외 `self.audio = None` 은 `main.rs:1234`(Loading 취소, Play 미진입) 하나다. → B7 은 이 **2개 지점만** `clear_namespace(PLAY)` 로 바꾸면 되고, `Stage::Play` 를 거치지 않는 누수 경로는 없다. (`enter_result` 는 오디오를 건드리지 않으므로 Result 화면에서도 엔진이 살아 있고, 이는 리절트 BGM/시스템 사운드(Phase F)에 유리하다.) |
| `start.sh` 의 환경변수 전달 | **`start.sh` 는 이 레포에 존재하지 않는다** — `.gitignore` 가 `start.sh` 를 제외한다(내부 작업 파일). 따라서 §4.3 의 소크 로그 활성화는 스크립트에 의존할 수 없다. **결정: `RBMS_SOAK_LOG` 환경변수를 앱이 직접 읽는다**(`std::env::var("RBMS_SOAK_LOG")`, 값이 있으면 그 경로로 60초마다 append, 없으면 비활성). 실행은 `RBMS_SOAK_LOG=/tmp/rbms-soak.csv cargo run --release -p rbms-player -- --autoplay <곡>` 로 하고, 로컬 `start.sh` 사용 여부와 무관하게 동작한다. 구현은 B8(`timing.rs`, B-app-clock 소유). |

---

## 9. 비평 반영 (2026-09-09)

완성도 비평(외부 리뷰)에 대해 **인용된 코드를 전부 직접 열어 확인**한 뒤 반영했다. 반영/기각 내역은 아래와 같다.

### 9.1 반영 — 설계 누락 (치명)

1. **`Player::update` 축 오염** (신규 §3.6, §3.2 표 2행 교체, §5 에 B0 단계 추가, R11·R12 추가, §3.0 에 `rbms-play` 동결 계약 추가)
   `crates/rbms-play/src/lib.rs:184-229` 를 통독해 확인했다. 한 `now_us` 로 bg 커서(185-189) · 오토플레이 `judge.press/release`(196-215) · `AUTO_BEAM_US` 빔 소등(221-227) · `judge.update` **미스 스윕**(228) 을 모두 돌린다. 원안대로 `scheduled_us` 를 넣었다면 미스·오토 판정·빔이 1버퍼(10.69ms) 앞당겨졌을 것이다. `update_schedule`/`update_judge` 로 분리하고 `judge_cursor` 를 도입했다.
2. **`PlayEvent` 에 버스 판별자 없음** (§3.0 `PlaySource` 추가, §3.6 매핑 명시)
   `lib.rs:5-9` 확인. `wav`/`at_us` 2필드뿐이라 bg 채널(`:187`)과 오토플레이 키음(`:195`)이 구분 불가였고, §3.4 버스 배정표를 호출부에서 구현할 수 없었다. `source: PlaySource { Bgm, Key }` 를 추가했다.

### 9.2 반영 — 파일 소유권 (§6 전면 재작성)

3. **`crates/rbms-play/src/lib.rs` 무주공산** → Wave 1 에 **B-play-api** 갈래 신설(B0).
4. **`main.rs` 소유권 충돌** — 실측으로 확인했다. B-app-clock 의 작업 지점 5곳(press 경로 1263·1269·1272, `TimingSample` push, Loading 취소 `:1234`)이 전부 원안에서 B-app-settings 배타 파일 안에 있었다. **경계 규칙을 뒤집어** `main.rs` 를 B-app-clock 소유로 옮기고, 설정 UI 는 **Wave 1.5 순수 이동 커밋**으로 `settings_ui.rs` 에 먼저 분리했다(원안의 R8 "권장" 을 필수 선행 단계로 승격). B-app-clock 의 `main.rs` 작업 목록을 앵커와 함께 표로 명시했다.
5. **Wave 1 독립성 과장** → **Wave 0(직렬 스텁 커밋)** 신설. `mixer.rs` 에 `Bus`/`Command` 확장/`MixStats` **선언만** 먼저 머지해 4갈래가 각자 빌드 가능하게 했다.
6. **`decode.rs` 미배정** → "Phase B 미변경" 으로 명시(어느 브랜치도 열지 않음).
7. **소유권 검증 라인** — Wave 1/1.5/2/3 각각에 파일 전체 목록 + 교집합 = ∅ 명시.

### 9.3 반영 — 앵커 수정

| 항목 | 원안 | 정정 |
|---|---|---|
| `app_select.rs` `update_preview` | 스냅샷 355-390 | **300** (실측) |
| `AudioConfig.java` 버퍼 등 | `:22-32`, `deviceBufferSize` = `:23` | `:21-32`, `deviceBufferSize` = **`:24`** (21·23·26·30 은 주석) |
| `AbstractAudioDriver.java` `play(Note,volume,pitch)` | 486-490 | **486-491** |
| `AbstractAudioDriver.java` 판정음 | 493-503 | **493-505** |

### 9.4 반영 — 설계 공백 채움

8. **판정음의 `chart_gain`** — `AbstractAudioDriver.java:502` 가 `play(sound, channel, volume, 1.0f)` 로 `#VOLWAV` 유래 `volume` 을 판정음에도 곱함을 확인. §3.4 에 "**`chart_gain` 은 전 버스 적용**(Phase F `Bus::System` 포함)" 을 결정으로 명시.
9. **A5 호출부 게인 리터럴** — §3.4 에 처분표 추가. Play/press/리플레이 `1.0` 유지, **`PREVIEW_GAIN = 0.85` 유지**(최종 진폭이 현행과 동일함을 계산으로 제시).
10. **A13(채널 키 피치 포함)** — 문서 헤더에 "**A13 은 `mixer.rs:40-48` 에서 이미 해소(Phase A-audio). Phase B 작업 없음**" 표기 추가.
11. **U11 분포 시각화 축소** — §4.2 에 명시적 처분 추가. 화면은 mean/sd/p95, **분포 원본은 per-sample CSV 전량 덤프**, 화면 내 히스토그램은 Phase F.
12. **프리뷰 스트림 사망(신규 리스크)** — R10 추가. `update_preview` 에 벽시계 폴백이 없음을 확인하고, `song_us()` 와 동일 폴백 헬퍼로 통일하도록 B7 에 지시.
13. **Wave 3 baseline 수치 출처 불명** — "1,060 / 51 / 94" 를 폐기하고 **Wave 0 착수 직전 커밋에서 재측정**으로 교체(계획 §2 G-04 의 `c6f0885` 기준 889 통과를 참고값으로 병기).
14. **§4.3 CSV 예시 행** — "값은 전부 임의이며 실측 baseline 이 아님" 경고 추가.

### 9.5 기각 — 비평이 틀린 항목

| 비평 | 실측 근거 | 판정 |
|---|---|---|
| "§1.1 `settings.rs:10-39` → `pub struct PlaySettings` 는 11행 시작(10행은 `#[serde(default)]`), 38행 종료" | `sed -n '8,42p' apps/rbms-player/src/settings.rs` 실행 결과: **8 `#[derive(...)]` / 9 `#[serde(default)]` / 10 `pub struct PlaySettings {`** … 필드 `player_id` 38행, 닫는 `}` **39행**, 41행 `impl Default`. | **기각.** 원안 `settings.rs:10-39` 가 정확하다. 비평이 한 줄씩 밀려 읽었다. 스펙 미변경(확인 사실만 괄호로 병기). |

### 9.6 비평이 확인해 준 사항 (변경 없음)

- 레퍼런스 패리티 3건(볼륨 3분리 `AudioConfig.java:43-54`, `#VOLWAV` 경계 `AbstractAudioDriver.java:355-358`, `channel()` `:507-509`) 전건 정확 — 재검증에서도 동일.
- §1.2 cpal 0.17.3 시그니처 표 및 산술(`513/48000 = 10.6875ms`, `3×512/48000 = 32ms`) 전건 일치.
