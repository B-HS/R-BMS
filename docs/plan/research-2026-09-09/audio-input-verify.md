# audio-input 보고서 반박 검증

검증 시각: 2026-09-09 / 상한 10분 / 읽기 전용
대상 보고서: `scratchpad/research/audio-input.md` (findings 13건)
검증 방식: 인용된 file:line 을 rbms/레퍼런스 구현 양쪽에서 직접 열어 대조. 인용 위치가 틀렸어도 주장 자체가 다른 위치의 코드로 성립하면 주장은 유지하고 인용 오류만 정정.

---

## 0. 총평

| 판정 | 건수 | id |
|---|---|---|
| confirmed | 8 | 01, 05, 07, 08, 09, 10, 11, 12 |
| partially | 5 | 02, 03, 04, 06, 13 |
| refuted | 0 | - |
| uncertain | 0 | - |

주장의 방향은 대체로 코드로 확인된다. 다만 **finding 02 의 "delay 는 언제나 0" 은 반례가 있고**(키 press 경로), 그 반례가 보고서가 놓친 별개 버그(키음 온셋에 판정 offset 이 섞임)를 드러낸다. finding 03 의 크기(magnitude)와 finding 04 의 "모든 재생 gain 1.0" 은 과장이다.

인용 라인 오차(주장 무관, 위치만 어긋남): `engine.rs:100→103`, `engine.rs:121→122`, `engine.rs:130→135`, `app_play.rs:237→app_input.rs:237`, `audio/AudioConfig.java→<reference>/AudioConfig.java`, `apps/rbms-player/src/Cargo.toml→apps/rbms-player/Cargo.toml`.

---

## 1. finding 별 판정

### audio-input-01 — confirmed

| 인용 | 실제 | 결과 |
|---|---|---|
| engine.rs:133 clock.store 1회 | `clock.store(mixer.clock_frames(), Ordering::Release);` — mix 이후 콜백 말미 1회 | 일치 |
| engine.rs:56-61 clock_us | 실제 56-62 (`clock_frames` 56-58, `clock_us` 60-62) | 사실상 일치 |
| app_play.rs:191-195 song_us | `Some(a) => a.clock_us() - self.anchor_us` | 정확 |
| main.rs:1130 raw | `let raw = self.song_us();` | 정확 |
| engine.rs:31-33 BufferSize 미지정 | `default_output_config()` → `supported.config()`, BufferSize 지정 없음 | 정확 |
| engine.rs:121 콜백 info 버림 | 실제 **122**: `move |data: &mut [T], _|` | 위치만 -1 |
| KeyBoardInputProcesseor.java:99-110 | `public void poll(final long microtime)` = 99, keyChanged = 107 | 정확 |
| BMSPlayerInputProcessor.java:501 | `final long now = System.nanoTime() / 1000 - starttime;` | 정확 |

주장(오디오 콜백 주기 양자화) 성립. 단 "512프레임/10.7ms" 는 보고서 본문 §4 가 스스로 미실측이라 밝혔고 finding 요약문에는 그 단서가 빠졌다 — 수치는 추정으로 읽어야 한다.

### audio-input-02 — partially

성립하는 부분(BGM/오토플레이/프리뷰):
- `rbms-play/src/lib.rs:174-178` `while ... self.bg[self.bg_cursor].0 <= now_us` → 방출 이벤트의 `at_us <= now_us` 확정.
- `app_play.rs:436` `audio.play(..., e.at_us + anchor)`, `engine.rs:94` at_frame 환산, `mixer.rs:93` `delay: at_frame.saturating_sub(self.clock)` → delay = 0.
- 프리뷰도 동일: `app_select.rs:354-356`(`<= song` 후 `at + anchor`), `app_select.rs:371-372`(`while clock_us() >= next` 후 `play(..., next)`).

**반례 — 키 press 경로에서는 delay 가 0 이 아닐 수 있다.**
- `main.rs:1131` `let judge_t = raw + self.offset_us();`
- `main.rs:1138` `player.press(lane, judge_t, |e| audio.play(..., e.at_us + anchor))`
- `crates/rbms-play/src/lib.rs:229` `play(PlayEvent { wav, at_us: now_us })` — 즉 `at_us = judge_t`.
- 따라서 `at_us + anchor = clock_us + offset_us`. `offset_ms > 0` 이면 **at_frame > clock → delay > 0** 이 실제로 걸린다.
- 리플레이 경로도 동일: `app_input.rs:237` `p.press(ev.lane, ev.t + off, play)`.

즉 "delay 로직은 실사용에서 절대 타지 않는 죽은 코드" 는 **부정확**하다. 살아 있고, 살아 있는 방식이 오히려 버그다(아래 missed-01).

**권고 자체의 결함**: lookahead 도입 시 press 키음(`at_us = judge_t = song + offset`)이 lookahead 만큼 **늦게** 발음된다. 키음 반응 지연은 리듬게임에서 가장 체감이 큰 항목이므로, press 경로는 `at_us = 0`(즉시) 로 예외 처리해야 한다는 조건이 권고에 반드시 붙어야 한다.

레퍼런스 구현 대조(`PortAudioDriver.java:537-550` put)는 정확 — 스케줄 개념 없음.

### audio-input-03 — partially

방향은 성립: `mixer.rs:183` `self.clock += frames`(믹싱 완료 프레임), `engine.rs:133` 콜백 말미 store → 클럭은 DAC 출력보다 앞선다. `app_play.rs:191-195` 의 같은 값을 렌더(`app_play.rs:421-433`)와 판정(`main.rs:1130-1131`)이 공용하는 것도 확인.

**과장 지점 — 크기**: "버퍼 + 디바이스 레이턴시만큼 시청각이 어긋난다" 는 절대량이지, 레퍼런스 구현 대비 **차이**가 아니다. 레퍼런스 구현 도 클럭은 `System.nanoTime()`(now)이고 오디오 출력은 디바이스 레이턴시만큼 늦으므로 동일한 방향의 어긋남을 갖는다. rbms 의 **추가** 스큐는 "믹싱 완료 vs 재생 완료" 차이 ≈ 콜백 버퍼 1개분이다. 심각도 high 는 유지 가능하나 근거 문장은 이 구분을 하지 않았다.

인용 오류: `audio/AudioConfig.java:24` → 실제 경로는 `bms/player/<reference>/AudioConfig.java:24` (`deviceBufferSize = 384` 값은 정확). `PortAudioDriver.java:271-277` getSuggestedLatency 정확(270행 선언, 271 bufferLatency).

### audio-input-04 — partially

성립:
- `set_master_gain` 은 `engine.rs:103`(인용 100 은 어긋남)에 정의되고 **크레이트 밖 호출처 0건**(grep 결과 mixer/engine 내부 + 테스트만).
- `mixer.rs:181` `*s = (*s * g).clamp(-1.0, 1.0)` 하드 클리핑, `mixer.rs:61` master_gain 기본 1.0.
- `#VOLWAV`: rbms 전체 grep 0건 → 미반영 확정.
- 레퍼런스 구현 `AudioConfig.java:46,50,54` = 0.5/0.5/0.5 정확, `AbstractAudioDriver.java:355-356` volwav 반영 정확, `:486-489` `play0(n, this.volume * volume, pitch)` 정확.

**부정확**:
- "모든 재생이 gain 1.0" 은 틀렸다. 프리뷰는 `PREVIEW_GAIN = 0.85`(`main.rs:60`)를 쓴다(`app_select.rs:356, 372, 441`). Play 경로 3곳만 1.0 리터럴이다.
- 인용 `app_play.rs:237` 은 gain 리터럴이 아니라 score submission 코드. 실제 위치는 `app_input.rs:237`.
- 권고 누락: `engine.rs:103` 에 API 가 이미 있으므로 "설정 UI 이전에 master_gain 기본값을 0.5~0.7 로 한 줄 적용"이라는 즉시 완화책이 가능하다.

### audio-input-05 — confirmed

`engine.rs:29-33`(default_output_device + default_output_config, BufferSize 미지정), `engine.rs:25` `Self::with_max_voices(512)` 모두 정확. 외부 호출처는 `app_play.rs:66` 외에 `app_select.rs:422`, `app_select.rs:457` 도 있으므로 "외부 호출처는 app_play.rs:66 뿐"은 틀렸으나(3곳 모두 `AudioEngine::new()`), 셋 다 512 하드코딩 경로라 주장은 그대로 성립.
레퍼런스 구현: `AudioConfig.java:15,20,24,28,32` 전부 값·행 일치, `PortAudioDriver.java:69`(framesPerBuffer), `:84`(inputs 크기) 정확.

### audio-input-06 — confirmed (단, 레퍼런스 구현 대조는 절반만 유효)

rbms 측 정확: 에러 콜백은 `engine.rs:135`(인용 130 어긋남) `|e| eprintln!("audio stream error: {e}")` 뿐. `song_us()`의 벽시계 폴백(`app_play.rs:191-195`)은 `self.audio` 가 **None 일 때만** 동작하므로, 스트림이 죽어도 엔진 객체는 살아 있어 폴백이 안 걸린다 — 주장대로다.

**정정**: 레퍼런스 구현 도 런타임 복구는 없다. `PortAudioDriver.java:644-657` 은 write 실패 시 로그 후 `stop = true; break;` 로 믹서 스레드를 종료할 뿐 재오픈하지 않는다. 레퍼런스 구현 의 우위는 **오픈 시점** 폴백(`:162-191` 장치×레이트, `:242-267` 고레이턴시/버퍼 미지정 3단 폴백)에 한정된다. finding 본문 evidence 는 이를 정확히 적었으나 요약 문장은 "복구" 일반의 우위처럼 읽힌다.

### audio-input-07 — confirmed

`app_select.rs:422-430` (#PREVIEW 경로) 와 **`app_select.rs:457-465` (autoplay 프리뷰 경로)** 두 곳 모두 포커스 정착마다 `AudioEngine::new()` 로 새 cpal 출력 스트림을 연다(보고서는 422 만 인용). `main.rs:692` preview_audio 필드, `app_play.rs:10-13` 주석("so the two cpal output streams never coexist") 전부 확인.

### audio-input-08 — confirmed (레퍼런스 구현 매핑은 산술 추론)

rbms 측 정확: `keyconfig.rs:193-212` `by_lane: Vec<Option<KeyCode>>` → 레인당 1키, `app_input.rs:80-82` `lane_for` 첫 매칭 1건. 완화책 `app_play.rs:267-270` 확인.
레퍼런스 구현: `PlayModeConfig.java:300` BEAT_7K = 9키 배열 확인, `BMControllerInputProcessor.java:31-32` AXIS1_PLUS/AXIS2_MINUS, `:61` duration=16, `:81` TICK_MAX_SIZE=0.009f, `:98-114` AnalogScratchAlgorithm v1/v2 전부 일치.
단, "9키가 8레인 = 스크래치 정/역 2키" 라는 매핑 자체는 `Mode` 정의가 외부 bms-model 의존(소스 트리에 `bms/model/Mode.java` 없음)이라 직접 확인 불가. 5K=7키/6레인, 7K=9키/8레인, 10K=14키/12레인 의 **+1(측당) 패턴**으로 강하게 뒷받침되나 코드 직독은 아니다.

### audio-input-09 — confirmed

`apps/rbms-player/Cargo.toml`(인용 경로 `src/Cargo.toml` 은 오타) 의존성에 gilrs/midir/sdl 없음 확인(winit 0.30.13, wgpu, rfd 등만). `main.rs:949` `WindowEvent::KeyboardInput` 만 처리 확인. 레퍼런스 구현 측 `input/MidiInputProcessor.java`, `input/MouseScratchInput.java`, `input/BMControllerInputProcessor.java` 파일 존재 확인(MidiInputProcessor 내부 행 번호는 미검증).

### audio-input-10 — confirmed

`mixer.rs:108-120` `alloc_slot`: free 슬롯 탐색 실패 시 `let i = self.alloc_cursor; ... return i;` — 활성 보이스 강탈 정확. `mixer.rs:86-96` 그 슬롯을 무조건 덮어씀. 레퍼런스 구현 `PortAudioDriver.java:537-550` `put`: `input.pos == -1` 인 빈 입력만 사용, 없으면 드롭 — 정확.

### audio-input-11 — confirmed

`engine.rs:95` `key: id`, `mixer.rs:82` `self.stop(key)` 진입 시, `mixer.rs:99-106` key 일치 전부 정지 — 정확. 레퍼런스 구현 `AbstractAudioDriver.java:507-509` `channel(id,pitch) = id*256 + pitch + 128`, `:517`/`:524-525` 사용부 정확. "현재는 피치가 항상 1.0" 도 사실(호출부 전부 `1.0` 리터럴).

### audio-input-12 — confirmed

`SystemSoundManager.java` 존재, `AbstractAudioDriver.java:493-502` `play(int judge, boolean fast)` + `channel = (65536 + judge) * 256` 로 판정음 전용 채널 대역, `AudioConfig.java:59,64` 결과/코스결과 사운드 루프 플래그 확인. rbms 측 `AudioEngine` 사용처가 Play(`app_play.rs:66`)/프리뷰(`app_select.rs:422,457`) 뿐인 것도 확인.

### audio-input-13 — partially

검증된 부분: `mixer.rs:149-165` 선형 보간, `mixer.rs:85` `stride = sample.rate / out_rate * pitch`. 레퍼런스 구현 `PortAudioDriver.java:611-626` 직접 인덱싱(보간 없음), `:628-633` `posf` 누적 후 `(int)` 절삭 = 최근접. `AudioConfig.java:32` sampleRate, `PortAudioDriver.java:227-239` 레이트 후보.

미검증: "레퍼런스 구현 는 **로드시 PCM 레이트 변환**" 이라는 대조축. `PCM`/`FloatPCM`/`ShortPCM` 는 외부 bms-model 의존이라 이 소스 트리에서 확인 불가 — 이 부분은 미확인으로 표기해야 한다. 나머지 주장(rbms 가 보간 우위, 다운샘플 AA 부재)은 성립.

---

## 2. 보고서가 놓친 항목 (missed)

| id | 항목 | 근거 |
|---|---|---|
| missed-01 | **키음 발음 시각에 판정 offset 이 섞인다** — press 는 `judge_t = raw + offset_us` 를 판정 시각이자 오디오 스케줄 시각으로 함께 넘긴다. `offset_ms != 0` 이면 키음 온셋이 그만큼 이동하고, 양수면 mixer delay 가 실제로 걸려 키음이 늦게 난다. `auto_offset` 이 켜지면 2회차 런부터 offset 이 자동으로 0 이 아니게 되므로 상시 발생한다. | `main.rs:1131,1138`, `rbms-play/src/lib.rs:229`, `mixer.rs:93`, `app_play.rs:364-366`, `app_input.rs:237` |
| missed-02 | **lookahead 권고의 부작용** — finding 02/03 대로 song 을 뒤로 물리면 press 키음이 lookahead 만큼 지연된다. press 경로만 `at_us=0`(즉시 발음) 예외로 두어야 권고가 안전하다. | `main.rs:1138`, `rbms-play/src/lib.rs:228-230`, `mixer.rs:93`, `engine.rs:94` |
| missed-03 | **AutoVsync present 블로킹에 의한 입력 디스패치 지연** — 이벤트 루프가 `ControlFlow::Poll` 로 돌지만 렌더는 FIFO present 라 acquire/present 에서 블록될 수 있고, 그 동안 도착한 키 이벤트는 블록 해제 후 처리되며 그때 `song_us()` 를 읽는다. 오디오 콜백 양자화와 **별개의** 지연원인데 보고서 §2.1 은 "프레임에 배칭되지 않는다" 로만 정리했다. (실측 미수행) | `gpu.rs:112` `PresentMode::AutoVsync`, `main.rs:1271` `ControlFlow::Poll`, `main.rs:1168` request_redraw, `main.rs:1130` |
| missed-04 | **`#WAV` 슬라이스(音切り) 미대응** — 레퍼런스 구현 는 `starttime/duration` 으로 slicesound 분기를 타지만, rbms 재생 경로에는 slice 개념이 아예 없다(`rbms-play`/`rbms-audio` grep 0건). 보고서는 이를 finding 이 아니라 "미조사 범위" 로만 남겼다. | `AbstractAudioDriver.java:527-533`(slicesound 분기), `rbms-play/src/lib.rs:176,184`(wav id 단독), `engine.rs:92-97` |
| missed-05 | **보이스 시작/정지 페이드 부재로 클릭 노이즈** — `play` 는 pos=0 에서 즉시 full gain, `stop` 은 즉시 `active=false` 라 라운드로빈 스틸(finding 10) 과 결합되면 재생 중 파형이 0 으로 절단된다. 스틸 정책 수정 시 짧은 릴리즈 램프를 함께 넣어야 한다. | `mixer.rs:86-96`, `mixer.rs:99-106`, `mixer.rs:108-120` |
| missed-06 | **master_gain 즉시 완화책이 권고에서 빠졌다** — 볼륨 설정 UI 없이도 `set_master_gain` 은 이미 존재하므로 기본값 하향 한 줄로 클리핑을 크게 줄일 수 있다. finding 04 권고는 설정 추가부터 요구해 effort 를 실제보다 크게 보이게 한다. | `engine.rs:103-105`, `mixer.rs:61`, `mixer.rs:179-182` |
| missed-07 | **엔진 부재 시 벽시계 폴백은 이미 존재한다** — finding 06 이 "재오픈 실패 시 벽시계 폴백" 을 신규 권고로 제시하나, `AudioEngine::new()` 실패 케이스(`app_play.rs:66` Err 분기 → `self.audio = None`)에 대해서는 `song_us()` 가 이미 `self.clock.elapsed()` 로 폴백해 무음 플레이가 성립한다. 필요한 것은 "스트림 사망을 None 과 동등하게 취급하는 경로" 하나뿐이라 effort 는 M 보다 작다. | `app_play.rs:191-195`, `app_play.rs:66`, `engine.rs:135` |
| missed-08 | **프리뷰 엔진도 512 보이스 · 별도 클럭을 갖는다** — finding 05(동시발음 하드코딩)와 finding 07(중복 엔진)이 결합되면 Select 화면에서 512 보이스 믹서가 하나 더 상주하고, 프리뷰 anchor 도 그 엔진의 독립 클럭 기준이라 Play 진입 시 시간축이 단절된다. 두 finding 을 따로 고치면 통합 시 재작업이 생긴다. | `engine.rs:25`, `app_select.rs:342,353,422,457`, `app_play.rs:10-13,162` |

---

## 3. 검증하지 못한 범위

- `MidiInputProcessor.java` 내부 행 번호(14-22, 160) 미확인 — 파일 존재만 확인.
- 레퍼런스 구현 `Mode` 정의(레인 수 ↔ 키 배열 매핑)는 외부 bms-model 의존이라 소스 직독 불가.
- `PCM` 계열 클래스의 로드시 레이트 변환 여부 미확인(finding 13 대조축).
- cpal 실제 버퍼 프레임 수 미실측(보고서와 동일 한계).
- `crates/rbms-audio/src/decode.rs` 미통독.
- 실행 계측(판정 오차 ms, 입력 지연) 미수행.
