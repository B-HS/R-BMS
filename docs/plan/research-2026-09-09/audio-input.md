# R-BMS 오디오/입력 경로 정확성 조사 (vs beatoraja)

조사 시각: 2026-09-09 / 조사 상한 15분 / 읽기 전용 조사(파일 변경 없음)

대상
- rbms: `crates/rbms-audio/src/{engine.rs,mixer.rs,lib.rs}`, `apps/rbms-player/src/{main.rs,app_play.rs,app_input.rs,app_select.rs,keyconfig.rs}`, `crates/rbms-play/src/lib.rs`
- beatoraja: `src/bms/player/beatoraja/audio/{AbstractAudioDriver,PortAudioDriver}.java`, `AudioConfig.java`, `PlayModeConfig.java`, `input/{BMSPlayerInputProcessor,KeyBoardInputProcesseor,BMControllerInputProcessor,MidiInputProcessor}.java`

---

## 0. 결론 요약

가장 큰 문제는 리샘플러 품질이 아니라 **시간축(클럭) 설계** 하나다.

`song_us()`가 "믹서가 지금까지 믹싱한 프레임 수"를 그대로 쓰기 때문에

1. 판정 타임스탬프가 **오디오 콜백 주기 단위로 계단(quantize)** 되고,
2. 그 클럭은 **실제로 귀에 들리는 위치보다 출력 레이턴시만큼 앞서** 있으며,
3. 그 결과 `Player::update(song)`이 방출하는 키음/BGM 이벤트는 **항상 `at_frame <= clock`** 이 되어
   믹서의 샘플 정밀 스케줄링(`delay`) 경로가 **한 번도 동작하지 않는다**.

즉 rbms가 "beatoraja 대비 타이밍 소스 개선"이라고 주석에 적어 둔 부분(`engine.rs:16-19`)이,
현재 사용 방식에서는 오히려 **버퍼 주기만큼의 계단 + 상수 지연**을 만들고 있다.
`song_us()`를 믹서 클럭보다 한 버퍼 뒤로 물리고 벽시계로 보간하는 **한 가지 수정**이
판정 정밀도·오디오 온셋 정밀도·시청각 동기 세 가지를 동시에 해결한다.

---

## 1. 오디오 경로

### 1.1 리샘플링 — 품질은 오히려 beatoraja보다 낫다

| 항목 | rbms | beatoraja |
|---|---|---|
| 방식 | 보이스별 `f64` 위치 + **선형 보간** (`mixer.rs:145-160`) | 로드시 PCM 레이트 변환 + 재생시 **정수 증분(최근접)** (`PortAudioDriver.java:628-634`) |
| 피치 | stride에 곱해 무료로 얻음 (`mixer.rs:85`) | `input.posf += gpitch * input.pitch` 후 `(int)` 절삭 |
| 레이트 불일치 | `sample.rate / out_rate` 를 stride 에 반영 (`mixer.rs:85`) | 스트림 오픈 시 디바이스 레이트 후보를 시도 (`PortAudioDriver.java:228-239`) |

beatoraja PortAudio 믹서 루프(`PortAudioDriver.java:602-660`)는 보간이 없다(`sample[input.pos]` 직접 인덱싱, `posf` 는 정수 증분 계산에만 사용). 따라서 **선형 보간을 쓰는 rbms 쪽이 재생 품질에서 앞선다.**

남는 문제는 **다운샘플 시 안티에일리어싱 부재**뿐이다. 44.1 kHz 키음을 48 kHz 디바이스에서 재생하면 stride < 1 (업샘플)이라 무해하고, 48 kHz 소스를 44.1 kHz 디바이스에서 재생할 때만 stride > 1 로 에일리어싱이 난다. 실사용 빈도가 낮아 우선순위는 낮다. 반면 beatoraja는 `AudioConfig.sampleRate`(`AudioConfig.java:32`)로 디바이스 레이트를 지정할 수 있어 회피 수단이 있다.

### 1.2 클럭 — 계단 + 상수 선행

```
engine.rs:126  mixer.mix(&mut scratch);
engine.rs:133  clock.store(mixer.clock_frames(), Ordering::Release);
engine.rs:60   pub fn clock_us(&self) -> i64 { clock_frames * 1e6 / out_rate }
app_play.rs:191-195  song_us() = audio.clock_us() - anchor_us
main.rs:1130   let raw = self.song_us();   // 키 입력 판정 시각
```

- `clock`은 콜백이 끝날 때만 갱신되므로, 콜백 사이(수 ms)에는 **완전히 정지한 값**이다. cpal는 `default_output_config().config()`(`engine.rs:31-33`)로 `BufferSize::Default`를 그대로 쓰므로 버퍼 크기를 앱이 알지도, 고르지도 못한다. macOS CoreAudio 기본은 통상 512프레임(48 kHz 기준 **10.7 ms**).
- 따라서 같은 콜백 구간에 들어온 두 입력은 **동일 타임스탬프**를 받고, 판정 오차가 0~버퍼주기(균등분포, 평균 ≈ 5 ms) 만큼 실린다.
- 동시에 `clock`은 "믹싱 완료 프레임 수"라 **실제 가청 위치보다 (버퍼 + 디바이스 레이턴시)만큼 앞선다.** 화면 노트는 이 앞선 클럭으로 그려지므로 **시각이 청각보다 그만큼 빠르다.** `offset_ms`/`auto_offset`(`app_input.rs:216-218`, `main.rs:1143-1150`)은 판정만 보정할 뿐 이 시청각 어긋남은 보정하지 못한다.

beatoraja 대조: 키보드 입력은 렌더 스레드에서 `poll(microtime)` 으로 처리되고(`KeyBoardInputProcesseor.java:99-110`), microtime 은 `System.nanoTime()/1000 - starttime`(`BMSPlayerInputProcessor.java:501`)이다. **beatoraja도 프레임 단위로 양자화되지만 값 자체는 연속 시계**라, 프레임률을 올리면 정밀도가 그대로 올라간다. rbms는 프레임률을 올려도 오디오 콜백 주기 아래로는 내려가지 않는다.

### 1.3 샘플 정밀 스케줄링이 사실상 죽어 있음

```
rbms-play/src/lib.rs:174-179   while bg[cursor].0 <= now_us { play(PlayEvent{ at_us: at }) }
app_play.rs:436               audio.play(..., e.at_us + anchor)
engine.rs:94                  at_frame = at_us * out_rate / 1e6
mixer.rs:93                   delay: at_frame.saturating_sub(self.clock)
```

`update(now_us)`는 `at <= now_us` 인 이벤트만 방출하고, `now_us`는 이미 믹서 클럭에서 온 값이며, 명령이 믹서에 도달할 때 믹서 클럭은 그보다 더 진행돼 있다. 결과적으로 **`delay`는 항상 0**이다. 즉 `mixer.rs`의 `delay` 로직과 그 테스트(`delay_is_sample_accurate` 등)는 실사용에서 절대 타지 않는 경로다.

실질 영향: 모든 BGM/키음 온셋이 **다음 콜백 경계로 반올림**되어 최대 한 버퍼(≈10 ms)의 지터를 갖는다. 음절(音切り, `#WAV` slice) 계열 차트나 촘촘한 BGM에서 청감상 흔들림으로 나타난다.

### 1.4 볼륨 — 체계 자체가 없음

- `AudioEngine::set_master_gain`(`engine.rs:100`)을 **호출하는 코드가 워크스페이스에 하나도 없다**(grep 결과 `crates/rbms-audio` 밖에서 0건).
- 모든 재생이 `gain = 1.0`으로 나간다(`app_play.rs:237`, `app_play.rs:436`, `main.rs:1138`).
- 믹서 최종단은 `(*s * g).clamp(-1.0, 1.0)`(`mixer.rs:181`) — 즉 **하드 클리핑**. 동시발음이 몰리는 구간에서 왜곡된다.

beatoraja: `systemvolume/keyvolume/bgvolume` 기본값 각각 **0.5**(`AudioConfig.java:46,50,54`), `#VOLWAV`도 반영(`AbstractAudioDriver.java:355-359`). rbms는 `#VOLWAV`도 미반영(코드에서 확인 불가 → 미확인이 아니라, `play()` 호출부 gain 이 전부 리터럴 1.0 이므로 미반영 확정).

### 1.5 폴리포니/보이스 스틸/재발화

| 항목 | rbms | beatoraja |
|---|---|---|
| 동시발음 | `with_max_voices(512)` 하드코딩 (`engine.rs:25`), 설정 불가 | `deviceSimultaneousSources` 기본 256, 설정 가능 (`AudioConfig.java:28`) |
| 슬롯 없음 | `alloc_cursor` 위치의 **활성 보이스를 강탈** (`mixer.rs:108-119`) | 빈 입력이 없으면 **드롭** (`PortAudioDriver.java:537-545`) |
| 동일 키 재발화 | `play()` 진입 시 `stop(key)` (`mixer.rs:82`) | `stop(wav, channel); play(wav, channel, ...)` (`AbstractAudioDriver.java:511-531`) |
| 채널 정의 | `key = wav id` | `channel = id*256 + pitch + 128` (`AbstractAudioDriver.java:507-509`) |

동일 키음 재발화 시 이전 발음을 끊는 정책 자체는 **양쪽이 동일**하다. 다만 beatoraja는 **피치가 다르면 다른 채널**로 취급해 서로 끊지 않는데, rbms는 `key = id` 뿐이라 피치가 다른 동일 wav 도 서로 끊는다(Practice/FREQUENCY 계열 기능을 넣을 때 문제가 된다).

보이스 스틸 정책은 rbms가 더 나쁘다. 라운드로빈 커서가 가리키는 보이스를 무조건 죽이므로, 길게 울리는 BGM이 짧은 키음에 잘려 나갈 수 있다(가장 오래된/가장 조용한 보이스 선택이 아님).

### 1.6 디바이스/스트림 견고성

```
engine.rs:130   |e| eprintln!("audio stream error: {e}"),
```
- 에러 콜백이 **출력만 하고 아무 복구도 하지 않는다.** 재생 중 출력 장치를 뽑거나 기본 장치가 바뀌면 스트림이 죽고, 그 시점부터 `clock`이 정지하므로 **`song_us()`가 멈춰 게임 자체가 정지**한다(`app_play.rs:191-195`).
- 디바이스 선택/드라이버 선택/샘플레이트 지정/버퍼 크기 지정 모두 없음.

beatoraja: 드라이버 종류(`DriverType`, `AudioConfig.java:15`), 장치명(`driverName`, `:20`), 버퍼(`deviceBufferSize=384`, `:24`), 샘플레이트(`:32`)를 설정으로 노출하고, 장치 열기 실패 시 **후보 장치 × 후보 레이트로 폴백**한다(`PortAudioDriver.java:162-191`, `242-268`).

### 1.7 프리뷰(곡선택) — 동작하되 구조가 비싸다

`docs/bug/2026-06-03-preview-playback.md`의 "무음" 이슈는 **해소됐다**(문서 하단 2026-06-07 항목). 코드도 그와 일치한다: `start_preview`가 `#PREVIEW` 유무로 분기해 파일 프리뷰 / autoplay 프리뷰를 태우고(`app_select.rs:382-397`), 각 실패 분기에 debug 로그가 붙어 있다(`app_select.rs:389-427`).

남은 구조적 문제:
- 프리뷰가 **곡 포커스마다 두 번째 `AudioEngine::new()`(= 두 번째 cpal 출력 스트림)를 새로 연다**(`app_select.rs:422`). 목록을 빠르게 훑으면 스트림 open/close 가 반복되고, 일부 백엔드(WASAPI 배타, 일부 ASIO)에서는 두 번째 스트림 자체가 열리지 않는다.
- Play 진입 시 `stop_preview()` 로 프리뷰 엔진을 먼저 버려 충돌을 피하는 방식(`app_play.rs:10-13`)이라, 두 스트림 공존 전제가 이미 불안정하다는 것을 코드가 인정하고 있다.
- 권장: 앱 수명 동안 **엔진 하나만 유지**하고 프리뷰/플레이가 같은 믹서를 공유(키 id 네임스페이스 분리)한다.

### 1.8 시스템 사운드 부재

beatoraja에는 `SystemSoundManager.java`(곡선택 스크롤/결정/결과 BGM)와 판정음(`AbstractAudioDriver.play(int judge, boolean fast)`, `AbstractAudioDriver.java:493-505`)이 있다. rbms에는 대응물이 없다(`AudioEngine` 사용처가 Play/프리뷰뿐).

---

## 2. 입력 경로

### 2.1 타임스탬프 정밀도

- winit `KeyEvent`에는 타임스탬프 필드가 없다. rbms는 `window_event`에서 즉시 `song_us()`를 읽는다(`main.rs:1130`). 이벤트 자체는 프레임에 배칭되지 않고 도착 즉시 처리되므로 **"프레임 양자화"는 아니지만, 클럭이 계단이라 결과적으로 오디오 콜백 주기로 양자화**된다(§1.2).
- 추정 정밀도: 버퍼 512프레임/48 kHz 가정 시 **0~10.7 ms 균등 오차(평균 5.3 ms, 표준편차 ≈ 3.1 ms)**. PGREAT 창이 ±20 ms 수준인 것을 감안하면 무시할 수 없다.
- beatoraja 키보드: 렌더 스레드 `poll(microtime)`, 연속 시계(`KeyBoardInputProcesseor.java:99-110` + `BMSPlayerInputProcessor.java:501`). 120 fps면 ≈8.3 ms 양자화. **MIDI는 수신 스레드에서 `System.nanoTime()/1000 - starttime`을 직접 찍어**(`MidiInputProcessor.java:160`) 프레임과 무관한 정밀도를 낸다 — rbms에는 대응이 없다.

### 2.2 디바이스 지원

| 장치 | rbms | beatoraja |
|---|---|---|
| 키보드 | O (`main.rs:949`, `keyconfig.rs`) | O (`KeyBoardInputProcesseor.java`) |
| 게임패드/전용 컨트롤러 | **X** — `apps/rbms-player/Cargo.toml`에 gilrs 등 입력 크레이트 없음 | O (`BMControllerInputProcessor.java`) |
| MIDI | **X** — midir 등 없음 | O (`MidiInputProcessor.java`) |
| 마우스 스크래치 | X | O (`MouseScratchInput.java`) |

### 2.3 스크래치

- rbms: `lane_keys()`가 **레인당 정확히 1개 키**를 반환한다(`keyconfig.rs:193-212`, `by_lane: Vec<Option<KeyCode>>`). 스크래치 레인도 단일 키다.
- beatoraja: 7K 키보드 기본 배정이 **9키/8레인** — `{Z,S,X,D,C,F,V, SHIFT_LEFT, CONTROL_LEFT}`(`PlayModeConfig.java:300`)로, 마지막 두 개가 스크래치의 **정/역 회전 2키**다. 컨트롤러는 `AXIS1_PLUS/AXIS2_MINUS`(`BMControllerInputProcessor.java:31-32`) + 아날로그 알고리즘 2종(`:98-114`, TICK 0.00787 등 `:81`).
- 영향: 연속 스크래치(회전 방향 교대로 채보가 이어지는 구간)에서 rbms는 **같은 키를 반복 연타**해야 한다. 판정 자체는 press 이벤트만 보므로 "칠 수는 있으나", 실기 감각과 채보 의도가 어긋나고, 아날로그 컨트롤러 사용자는 사실상 플레이가 불가능하다.
- 완화 장치로 `scratch_auto`(`app_input.rs:113`, `app_play.rs:267-270` → `Player::set_auto_lanes`)가 있다.

### 2.4 키 리피트 / 채터링

- rbms: OS 자동반복은 `!event.repeat`로 배제한다(`main.rs:951`, `main.rs:1134`). 별도 디바운스는 없다.
- beatoraja: 상태 변경 후 `duration`(기본 16 ms) 동안 재변경을 무시한다(`KeyBoardInputProcesseor.java:106`, 컨트롤러는 `BMControllerInputProcessor.java:61`). 키보드에서는 OS가 이미 처리하므로 큰 차이는 아니지만, 컨트롤러 도입 시 반드시 필요하다.
- 동시 입력: rbms는 이벤트마다 독립 처리라 동시 입력 제한이 없다. beatoraja는 폴링이라 같은 프레임 입력이 동일 타임스탬프를 받는다. rbms 쪽이 이 점에서는 유리하다.

---

## 3. 개선 우선순위

| 순위 | 항목 | 효과 | 난이도 |
|---|---|---|---|
| 1 | **송 클럭 보간 + 룩어헤드** — `song_us()`를 `콜백시각 기준 프레임 + 벽시계 경과`로 보간하고, 게임 시계를 믹서 클럭보다 한 버퍼 뒤로 물린다 | 판정 계단(≈10 ms) 제거 + `delay` 스케줄 활성화(온셋 지터 제거) + 시청각 어긋남 해소. **세 문제를 한 번에** | M |
| 2 | cpal `OutputCallbackInfo.timestamp().playback` 사용해 실제 가청 시각 기준 클럭 산출 | 1의 정확도를 기기별로 맞춤(레이턴시 추정 대신 실측) | M |
| 3 | 볼륨 체계(master/key/bg) + `#VOLWAV` + 소프트 리미터 | 클리핑 왜곡 제거, 설정 항목으로 노출 | S |
| 4 | 오디오 설정 노출(버퍼 크기·디바이스·샘플레이트·동시발음) | 저지연 환경 구성 가능, 기기 호환 폴백 | M |
| 5 | 스트림 에러/디바이스 변경 복구 (에러 콜백 → 재오픈, 실패 시 벽시계 폴백) | 재생 중 무음·게임 정지 방지 | M |
| 6 | 프리뷰를 단일 엔진 공유로 통합 | 스트림 반복 오픈 제거, 백엔드 호환성 | M |
| 7 | 스크래치 2키(정/역) 바인딩 | 채보 의도대로 플레이 가능 | M |
| 8 | 게임패드(gilrs) 지원 + 디바운스 | 실기 컨트롤러 사용자 확보 | L |
| 9 | 보이스 스틸 정책(가장 오래된/가장 조용한 우선), 피치를 키에 포함 | 긴 BGM 절단 방지 | S |
| 10 | MIDI 입력(수신 스레드 타임스탬프) | 최고 정밀도 입력 경로 | L |
| 11 | 시스템 사운드/결과 BGM/판정음 | 체감 완성도 | M |
| 12 | 다운샘플 안티에일리어싱 | 드문 조합에서의 음질 | M |

---

## 4. 미조사 범위

- `crates/rbms-audio/src/decode.rs`(250줄) 전체 통독 안 함 — 디코드 시 채널 다운믹스/레이트 보존 정책 미검증.
- cpal 0.17의 macOS 기본 버퍼 프레임 수를 **런타임으로 실측하지 않았다**. 본문의 512프레임/10.7 ms는 CoreAudio 통상값에 근거한 추정치다(수치는 미확인, 계단 발생 사실 자체는 코드로 확정).
- `AbstractAudioDriver.setModel`의 `#WAV` 슬라이스(音切り) 처리와 rbms 대응 여부 미대조.
- beatoraja `GdxSoundDriver`/`GdxAudioDeviceDriver`(OpenAL 경로) 미조사 — PortAudio 경로만 대조.
- 실제 판정 오차 ms 측정(실행 계측) 미수행 — 헤드리스로는 오디오 콜백 타이밍 재현이 불가.
