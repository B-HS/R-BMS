# S-05 cpal 출력 버퍼·콜백 주기 런타임 계측 (macOS)

측정 기기: macOS 26.6.2 (25G83), MacBook Pro Speakers, CoreAudio host, rustc 1.95.0
측정 프로젝트: `/private/tmp/claude-501/-Users-gkn-R-BMS/847c1230-e223-4d0c-adb0-2d6113bbb669/scratchpad/cpal-measure/` (cpal `=0.17.3`, `/Users/gkn/R-BMS/Cargo.lock:cpal 0.17.3` 과 동일)
스트림 구성: `default_output_device()` → `default_output_config()` → `.config()` → `build_output_stream(config, ...)` (rbms `crates/rbms-audio/src/engine.rs:29-47`, `:108-139` 와 동일 경로). 출력은 전부 0.0 무음.
원시 출력: `cpal-measure/run1.txt`, `run2.txt`. 하드웨어 레이턴시 조회: `cpal-measure/lat.c`(CoreAudio 속성 직접 조회).

## 0. 디바이스 기본값

| 항목 | 값 |
|---|---|
| `default_output_config()` | ch=2, rate=48000, `buffer_size: Range { min: 15, max: 4096 }`, fmt=F32 |
| `.config()` 결과 | `StreamConfig { channels: 2, sample_rate: 48000, buffer_size: Default }` |
| 지원 config 목록 | ch=2, 44100..96000, buf Range 15..4096, F32 (1개) |

## 1. 실측 (각 3초, 2회 반복 — run1/run2 값 동일 범위)

| 스트림 | 콜백 수/3s | frames/cb (min/med/max) | 콜백 간격 µs (min/med/max/mean) | playback−callback µs |
|---|---|---|---|---|
| `BufferSize::Default` (= 현 rbms) | 281 | 512 / 512 / 512 | 10578.6 / 10666.7 / 10752.8 / 10666.4 | 10666.7 (전 콜백 동일) |
| `BufferSize::Fixed(128)` | 1126~1127 | 128 / 128 / 128 | 2617.0 / 2666.5 / 2695.2 / 2666.7 | 2666.7 (동일) |
| `BufferSize::Fixed(256)` | 563 | 256 / 256 / 256 | 5297.1 / 5333.2 / 5371.8 / 5333.4 | 5333.3 (동일) |

- **프레임 수는 완전히 고정**(min=med=max). Default 에서 가변 블록은 한 번도 관측되지 않음.
- 콜백 간격 지터는 Default 기준 −88 / +86 µs (peak-to-peak 174 µs). 매우 안정적.
- **`OutputCallbackInfo.timestamp()` 의 playback−callback 은 "실측 레이턴시"가 아니다.** cpal macOS 백엔드가 `playback = callback + device_buffer_frames/rate` 로 **합성**한 값이다 (`~/.cargo/registry/src/index.crates.io-*/cpal-0.17.3/src/host/coreaudio/macos/device.rs:890-908`, `:796-801`). 그래서 정확히 버퍼 1주기와 같게 나온다. 하드웨어 경로 지연은 포함되어 있지 않다.

### 실제 출력 레이턴시 (CoreAudio 속성 직접 조회, rate=48000)

| 속성 | frames | ms |
|---|---:|---:|
| `kAudioDevicePropertySafetyOffset` (Output) | 48 | 1.000 |
| `kAudioDevicePropertyLatency` (Output) | 60 | 1.250 |
| `kAudioStreamPropertyLatency` (stream 0) | 690 | 14.375 |
| 합계 `L_hw` | **798** | **16.625** |
| + 버퍼 1주기(512) → 콜백 시각 기준 최대 총 지연 | 1310 | **27.29** |

즉 **콜백에서 쓴 블록의 마지막 샘플이 스피커에서 들리기까지 약 27.3 ms**, 첫 샘플은 약 16.6 ms.

## 2. 현 rbms 판정 타임스탬프 양자화 오차·시청각 스큐

경로: `crates/rbms-audio/src/engine.rs:133` 이 **콜백 1회당 1번** `clock.store(mixer.clock_frames())` → `engine.rs:60-62 clock_us()` → `apps/rbms-player/src/app_play.rs:191-195 song_us()` → `apps/rbms-player/src/main.rs:1133-1138` 에서 키 이벤트 시각으로 `player.press(lane, judge_t)`. 즉 판정 시각은 **오디오 콜백 주기로 계단 양자화**되고, 값은 "이미 렌더한 프레임 수(블록 끝)"이라 실제 가청 위치보다 앞선다.

`rate=48000`, `T_buf` = 버퍼 1주기, `L_hw` = 798 frames = 16.625 ms 기준:

| 항목 | 수식 | Default(512) | Fixed(256) | Fixed(128) |
|---|---|---:|---:|---:|
| 클럭 갱신 간격 = 양자화 스텝 `T_buf` | N/48000 | 10.667 ms | 5.333 ms | 2.667 ms |
| 양자화 오차(균등분포) 평균 | `T_buf/2` | **5.333 ms** | 2.667 ms | 1.333 ms |
| 양자화 오차 최대 | `T_buf` | **10.667 ms** | 5.333 ms | 2.667 ms |
| 양자화 지터 표준편차 | `T_buf/√12` | **3.079 ms** | 1.540 ms | 0.770 ms |
| 보고 클럭이 가청 위치보다 앞선 양 (범위) | `[L_hw, L_hw+T_buf]` | 16.63~27.29 ms | 16.63~21.96 ms | 16.63~19.29 ms |
| 같은 값 평균 (= 상수 바이어스) | `L_hw+T_buf/2` | **21.96 ms** | 19.29 ms | 17.96 ms |

해석
- **상수 바이어스 ≈ 22 ms**: 플레이어가 "들리는 소리"에 정확히 맞춰 치면 판정은 약 22 ms **늦게(SLOW)** 기록된다. 이건 `apps/rbms-player/src/app_input.rs:216-218 offset_us()`(`config.offset_ms`)와 `main.rs:1141-1145` 의 auto-calibration 이 흡수하도록 되어 있는 성분이다(오프셋 0이면 그대로 SLOW 편향).
- **오프셋으로 못 없애는 성분은 양자화 지터**: Default 에서 peak-to-peak 10.667 ms, σ=3.08 ms. PGREAT 창이 ±20 ms(`crates/rbms-judge/src/windows.rs:20`, 폭 40 ms)이므로 **지터 폭이 PG 창의 26.7%**를 차지한다. 즉 동일한 입력 정확도에서도 PG/GREAT 경계가 최대 10.7 ms 흔들린다.
- **시청각 스큐**: 노트 렌더도 같은 `song_us()`를 쓰므로(`app_play.rs:191`) 화면 역시 가청 위치보다 16.6~27.3 ms 앞선다. 디스플레이 출력 지연 `L_disp` 를 빼면 순 스큐 ≈ `21.96 − L_disp` ms(오디오가 뒤늦게 들림). 120 Hz 1프레임(8.3 ms) 가정 시 **≈ +13.6 ms**, 60 Hz 1프레임(16.7 ms) 가정 시 **≈ +5.3 ms**. `L_disp` 는 **미실측**(이번 과제 범위 밖).

## 3. `BufferSize::Fixed(128/256)` 실적용 여부

| 질문 | 결과 |
|---|---|
| 빌드/스트림 생성 성공? | 128·256 모두 **성공** (에러 없음) |
| 실제 적용? | **적용됨.** 콜백 frames/cb 가 정확히 128·256, 간격도 정확히 2666.7 / 5333.3 µs 로 균일(버스트 없음) → IO 주기 자체가 바뀐 것으로 판단 |
| 지원 범위 | `SupportedBufferSize::Range { min: 15, max: 4096 }` (48 kHz 기준 0.31 ms ~ 85 ms) |
| 설정 경로 | cpal 이 AUHAL 에 `kAudioDevicePropertyBufferFrameSize` 를 Global/Output 으로 set (`cpal-0.17.3/.../macos/device.rs:969-984`) |

주의(미해결): 별도 프로세스에서 `kAudioDevicePropertyBufferFrameSize`(Output scope)를 Fixed(128)/Fixed(256) 스트림 가동 중에 읽으면 **계속 512** 로 나왔다(`lat` t=4.5s/7.5s 샘플). 콜백 간격이 완전히 균일했으므로 이 IOProc 의 실제 주기는 128/256 이 맞다고 보지만, 디바이스 전역 속성과의 불일치 원인은 **미확인**. 또한 `L_hw`(798 frames)가 Fixed 적용 시에도 동일한지는 위 조회가 항상 798 을 돌려준 것 외에 **추가 검증 미실시**.

## 4. 함의 (요약)

- Default 512 프레임은 이 기기에서 **판정 클럭 해상도를 10.67 ms 로 고정**한다. 오프셋 보정으로도 남는 σ≈3.1 ms 지터가 PG 창 대비 무시할 수 없다.
- `BufferSize::Fixed(256)` 은 무비용으로 지터를 절반(σ 1.54 ms), `Fixed(128)` 은 1/4(σ 0.77 ms)로 줄인다. 콜백 부하는 각각 2배·4배(초당 187→375→750회)이며, 이번 무음 측정에서는 드롭·에러 콜백 0.
- 근본 개선은 버퍼 축소가 아니라 **입력 시각을 콜백 계단값이 아닌 연속 시각으로 보간**하는 것(마지막 콜백 `Instant` + 경과시간으로 프레임 보간)이다. 그러면 `L_hw` 상수 바이어스만 남고 양자화 지터는 제거된다. — 설계 제안, 이번 과제에서는 미구현.
