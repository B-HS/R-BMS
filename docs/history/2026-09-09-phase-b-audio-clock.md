# Phase B — 오디오 클럭 재설계 (2026-09-09)

> SSOT: `docs/plan/2026-09-09-phase-b-spec.md`. 결정: `docs/acknowledge/2026-09-09-enhancement-decisions.md`. BASELINE: `f98899b`(`cargo rev-parse --short HEAD`, Wave 0 착수 직전).

## 1. 목적

계획 `docs/plan/2026-09-09-enhancement-plan.md` §2 Phase B 1~6항, 갭 A1·A2·A4·A5·A6·A8·A9·A13, P7·P10, U11 을 해소한다.

- 게임 시계가 오디오 콜백 주기(512프레임@48kHz ≈ 10.7ms)로 계단화되지 않게 한다.
- BGM/autoplay/프리뷰 온셋이 버퍼 경계로 반올림돼 `delay=0`으로 붕괴하지 않게 한다.
- press 키음 지연은 유지한다.
- 앱 수명 동안 cpal 출력 스트림을 1개로 통일한다.
- 볼륨을 system/key/bg 3버스 + `#VOLWAV` + 마스터로 분리하고 클릭 없는 보이스 스틸을 만든다.
- AUDIO 설정 탭과 판정/언더런/보이스 계측을 추가한다.

A13(채널 키에 피치 포함)은 Phase A-audio 에서 이미 해소되어 있어 Phase B 작업 대상에서 제외했다.

## 2. Wave 별 구현 요약

### Wave 0 — 선행 스텁 (직렬, Fable)
`crates/rbms-audio/src/mixer.rs` 에 §3.0 동결 계약(`Bus`, `Bus::ALL`, `Command::Play{..,bus}`, `Command::StopRange/BusGain/ChartGain`, `MixStats`, `Mixer::stats()`) 선언만 넣고 렌더 동작은 무변경으로 유지했다. `bus`는 `apply`에서 폐기, 게인 필드는 수용 후 무시, `stats()`는 `MixStats::default()`. `StopRange`만 신규 경로라 실제 구현(반열림 `[lo_key, hi_key)` 즉시 정지)으로 넣었다.

경계 예외 2건: `Command::Play`에 필수 필드 `bus`가 생기면 리터럴을 만드는 모든 지점이 컴파일 에러가 나므로, 소유 밖 `engine.rs:171`(`Bus::Key`)·`decode.rs:237`(테스트 리터럴) 각 1줄을 최소 적응시켰다. `decode.rs`는 이후 §6 정정대로 "디코딩 경로 무변경, Command 시그니처 적응만 허용, 소유는 B-audio-mixer"로 명문화됐다.

### Wave 1 — 4갈래 병렬

**B-audio-clock (`engine.rs`, `lib.rs`) — B1/B2/B5.**
- seqlock을 5필드(`frames_start`/`frames_end`/`callback_nanos`/`playback_ahead_nanos`/`buffer_frames`)로 확장하고 `ClockSnapshot` + `audible_us`/`scheduled_us`/`lookahead_us`를 추가했다.
- audible 기준은 §3.1 대로 `frames_at_callback_start`(콜백 종료 후 값을 쓰면 버퍼 1개분 앞서는 스큐가 남는다는 성질을 테스트로 고정).
- lookahead = 실측 `buffer_frames` + `LOOKAHEAD_EXTRA_FRAMES`(초판 1). 512@48k → 10,687µs(명세 값 일치).
- 콜백이 `info.timestamp()`의 `playback.duration_since(&callback)`을 쓰고, `None`이거나 `MAX_PLAYBACK_AHEAD_MS(200)` 초과면 버퍼 기반 폴백 + `timestamp_fallbacks` 증가. 콜백 진입 간격이 직전 버퍼 주기의 `UNDERRUN_GAP_RATIO(1.5)`를 넘으면 `underruns` 증가(추정치).
- `MixStats`를 별도 seqlock으로 원자 발행하는 `mix_stats()`.
- `AudioOptions`/`AudioOpenReport`/`AudioEngine::open` 사다리 0~4단 + 장치 없음(§3.5). `pick_config`/`pick_notes`를 디바이스 비의존 순수 함수로 분리해 전수 테스트.
- `clear_namespace(IdNamespace)` = 뱅크 retain + `Command::StopRange`. `PLAY`/`PREVIEW` 구간이 서로소.

**B-audio-mixer (`mixer.rs`) — B3.**
- 신호 체인 `sample × voice_gain × env × pan_gain × bus_gain[bus]` → 합산 → `× chart_gain × master_gain` → `soft_limit()`.
- `DEFAULT_MASTER_GAIN` 0.5 → **1.0**, `DEFAULT_BUS_GAIN = 0.5`(System/Key/Bg), `DEFAULT_CHART_GAIN = 1.0`. 최종 진폭은 이전과 동일(0.5×1.0 = 1.0×0.5).
- `ATTACK_MS = 1.0` / `RELEASE_MS = 3.0` 램프, `Voice { env, phase }`. `stop`/`stop_id`/`stop_range`/재트리거는 즉시 컷 대신 Release.
- 보이스 스틸 3단계(비활성 → Release 중 env 최소 → gain×env 최소, 동률은 최고령). 라운드로빈 커서 제거.
- `MixStats`: `active_voices`/`steals`/`hard_steals`/`late_schedules`.

**B-volwav (`rbms-parser`/`rbms-model`/`rbms-chart`) — B4.**
- `ModelMeta.volwav: i32`, `Headers.volwav`(기본 `VOLWAV_DEFAULT_PERCENT=100`), 헤더 디스패치 `"VOLWAV"` 추가.
- 순수 함수 `chart_gain(volwav_percent: i32) -> f32` — `0 < v < 200`만 `v/100`, 그 외(0·200·음수·범위초과)는 1.0(레퍼런스 구현과 경계까지 동일).
- `to_model()` 리터럴에 `volwav: src.headers.volwav,` 1줄.

**B-play-api (`rbms-play/src/lib.rs`) — B0.**
- `PlaySource { Bgm, Key }` + `PlayEvent.source`. 방출 지점: bg 커서 → `Bgm`, `AutoAction::Press` → `Key`, `Player::press` 히트음 → `Key`.
- `Player::update`를 `update_schedule(sched_us, play)`(bg/action 커서 전진, 판정 미접촉)와 `update_judge(audible_us)`(신규 `judge_cursor`로 재판정, 빔, 미스 스윕)로 분리. 하위호환 `update`는 두 함수의 합성으로 유지.
- `judge_cursor <= action_cursor` 불변식은 강제하지 않고 호출 순서(스케줄 먼저)로 보장 — 강제 클램프는 판정 정체라는 더 나쁜 실패를 만들기 때문.

### Wave 1.5 — 순수 이동 (직렬, Fable)
`apps/rbms-player/src/settings_ui.rs` 신설. `SETTING_TABS`/`SETTING_KEYCONFIG`/`SETTING_FONT`, `setting_line`, `adjust_setting`을 `main.rs`/`app_select.rs`에서 그대로 이동(동작 변경 0, 바이트 대조로 검증). Wave 2 두 갈래의 파일 소유가 서로소가 되도록 하는 필수 선행 단계.

### Wave 2 — 2갈래 병렬

**B-app-settings (`settings.rs`, `settings_ui.rs`, `app_input.rs`) — B6.**
- `SETTING_TABS`에 AUDIO 탭 추가. Phase I 최종 인덱스(`ir_panel::SETTING_RIVALS=34`) 다음인 35~42로 재할당(§3.5 잠정 24~31은 폐기).
- `PlaySettings`에 오디오 8필드(`audio_device`/`audio_buffer_frames`/`audio_sample_rate`/`audio_polyphony`/`vol_master`/`vol_key`/`vol_bg`/`vol_system`) 전부 `#[serde(default)]`.
- 값 의미론(볼륨 정수 퍼센트 격자, 사다리 순환, clamp)을 순수 함수로 분리해 단위 테스트.
- 동결 인터페이스 `audio_options`/`audio_reopen_pending`/`clear_audio_reopen_pending` 구현. 재오픈 플래그는 값이 실제로 움직였을 때만 세운다.
- `app_input.rs` 리플레이 재생을 `audio.play_on(Bus::Key, ..)`로 전환(게인 1.0 유지).

**B-app-clock (`main.rs`, `app_play.rs`, `app_select.rs`, `timing.rs`) — B7/B8.**
- 엔진을 앱 수명 1개로 통일(`ensure_audio`). `preview_audio` 필드와 `AudioEngine::new()` 중복 생성 2곳 제거.
- 엔진 파기 2지점(`to_select_or_exit`, Loading 취소)을 `clear_namespace(PLAY/PREVIEW)`로 교체.
- `#VOLWAV`: `load()`에서 `set_chart_gain`, Play 이탈 시 중립(`VOLWAV_DEFAULT_PERCENT`)으로 복귀.
- `audio_clock_us()` 공용 헬퍼(보간 audible + `is_alive()` 폴백 + `resumed_clock_us`)를 `song_us()`와 `preview_sched_us()` 양쪽이 공유(R10 해소).
- 프레임 루프: `player.update_schedule(song+lookahead)` → `player.update_judge(song)`, `PlaySource → Bus` 1:1 매핑.
- press 키음은 `audio.play_on(Bus::Key, .., IMMEDIATE_KEYSOUND_AT_US)` 즉시 발음(§3.2 예외).
- 재오픈 디바운스(`poll_audio_reopen`, 400ms, Play 중이 아닐 때만).
- `timing.rs` 신설: `TimingProbe`/`TimingStats`/per-sample CSV/`SoakLogger`(60초마다 §4.3 15컬럼 append). 디버그 오버레이에 AUDIO/STREAM/VOICES/DEVICE/JUDGE 줄 추가.

### Wave 3 — 통합·리뷰·소크
게이트, 적대 리뷰 3갈래, 수정, 소크 12분(실제 780초×6세그먼트), 문서화. 아래 §4~§7 참조.

## 3. 동결 계약 최종형 (public APIs)

`docs/plan/2026-09-09-phase-b-spec.md` §3.0 문면 그대로 구현됐다. 주요 항목만 재기술한다(전문은 스펙 참조).

```rust
// crates/rbms-audio/src/mixer.rs
pub enum Bus { System, Key, Bg }
impl Bus { pub const ALL: [Bus; 3]; }
pub enum Command {
    Play { sample: Arc<SampleData>, gain: f32, pan: f32, pitch: f32, key: u32, at_frame: u64, bus: Bus },
    Stop { key: u32 }, StopId { id: u32 }, StopRange { lo_key: u32, hi_key: u32 },
    MasterGain(f32), BusGain { bus: Bus, gain: f32 }, ChartGain(f32),
}
pub struct MixStats { pub active_voices: u32, pub steals: u64, pub hard_steals: u64, pub late_schedules: u64 }

// crates/rbms-audio/src/engine.rs
pub struct AudioOptions { device_name, sample_rate, buffer_frames, max_voices }
pub struct AudioOpenReport { device_name, sample_rate, channels, buffer, fallback_step, notes }
pub struct ClockSnapshot { frames_at_callback_start, frames_rendered, callback_at, playback_ahead, buffer_frames }
impl AudioEngine {
    pub fn open(opts: &AudioOptions) -> Result<(AudioEngine, AudioOpenReport), AudioError>;
    pub fn snapshot(&self) -> ClockSnapshot;   // 최종형: clock_snapshot() -> (u64, Instant) 대체
    pub fn audible_us(&self, now: Instant) -> i64;
    pub fn scheduled_us(&self, now: Instant) -> i64;
    pub fn lookahead_us(&self) -> i64;
    pub fn underruns(&self) -> u64;
    pub fn mix_stats(&self) -> MixStats;
    pub fn play_on(&mut self, bus: Bus, id: u32, gain: f32, pan: f32, pitch: f32, at_us: i64);
    pub fn set_bus_gain(&mut self, bus: Bus, g: f32);
    pub fn set_chart_gain(&mut self, g: f32);
    pub fn clear_namespace(&mut self, ns: IdNamespace);
}
pub struct IdNamespace { pub base: u32, pub len: u32 }
impl IdNamespace { pub const PLAY; pub const PREVIEW; }

// crates/rbms-play/src/lib.rs
pub enum PlaySource { Bgm, Key }
pub struct PlayEvent { pub wav: i32, pub at_us: i64, pub source: PlaySource }
impl Player {
    pub fn update_schedule<F: FnMut(PlayEvent)>(&mut self, sched_us: i64, play: F);
    pub fn update_judge(&mut self, audible_us: i64);
    pub fn update<F: FnMut(PlayEvent)>(&mut self, now_us: i64, play: F);  // 하위호환
}
```

**리뷰로 초판에서 바뀐 지점**:
- `clock_snapshot() -> (u64, Instant)` 제거, `snapshot() -> ClockSnapshot`으로 대체(외부 호출부 0이라 무해).
- `Mixer::play_on`(8인자)은 신설하지 않음 — `clippy::too_many_arguments` 억제 전례를 만들지 않기 위해 엔진이 `Command::Play{..,bus}`로 버스를 전달.
- `Mixer::play`(6인자, `Bus::Key` 고정)는 `#[cfg(test)]` 전용으로 격하, 실제 경로는 `start_voice(PlayRequest)`.

## 4. 축 분리 결정

레퍼런스 없는 신규 문제(§3.6): `Player::update`가 하나의 시각으로 (a) bg/오토플레이 발음 예약, (b) 오토플레이 판정, (c) 빔 타이머, (d) 미스 스윕을 동시에 수행하고 있었다. 룩어헤드를 도입한 시각(`scheduled_us`)을 그대로 넣으면 판정·미스·빔이 1버퍼(약 10.7ms) 앞당겨져 화면(`audible_us`)과 어긋난다.

**결정**: 발음 예약(`update_schedule`, scheduled 축)과 판정 상태(`update_judge`, audible 축)를 분리하고, 판정 축 전용 `judge_cursor`를 신설해 `actions`를 다시 훑는다. 오토플레이 판정 시각 자체(커서의 절대 `at`)는 축 분리로 값이 바뀌지 않는다 — 어느 축에서 `at <= now` 판별을 하느냐만 바뀐다. 호출 순서는 반드시 `update_schedule` → `update_judge`(발음 예약이 판정보다 먼저 서야 `judge_cursor <= action_cursor` 불변식이 성립).

## 5. 볼륨 모델 재배치

`master 0.5, bus 없음` → `master 1.0, bus 0.5(System/Key/Bg 공통), chart_gain 1.0` 로 재배치했다. 진폭 등가성(0.5×1.0 = 1.0×0.5)을 회귀 테스트로 고정. `#VOLWAV`는 `chart_gain`으로 전 버스(향후 Phase F `Bus::System` 포함)에 곱한다 — 레퍼런스 구현이 판정음에도 같은 `volume`을 곱하는 것과 일치.

## 6. 폴백 사다리

`AudioEngine::open`: 요청 디바이스+요청 rate+요청 버퍼 → 요청 rate+기본 버퍼 → 디바이스 기본 config → 시스템 기본 디바이스 → 전 출력 디바이스 순회 → `NoDevice`. 각 단계 강등 사유를 `AudioOpenReport.notes`에 1줄씩 기록하고, 리뷰 반영으로 AUDIO 탭 status 줄에 실제 표시하게 했다(초판은 stdout에만 나가 화면에 없었다).

## 7. 검증 수치 (baseline vs after)

| 지표 | baseline(f98899b) | Phase B 완료 후 |
|---|---|---|
| `cargo test --workspace` | 1284 통과·0 실패·2 ignored | 1447 통과·0 실패·2 ignored |
| `cargo clippy --workspace --all-targets` 경고 | 70 | 43 (rbms-audio 0, rbms-play 15, rbms-parser 17, rbms-model 1, rbms-chart 12, rbms-player 16 — 전 크레이트 baseline 이하) |
| `cargo fmt --all -- --check` | 통과 | 통과 |
| 보간 클럭 잔차 sd(실기, 512@48k) | — | 계단형이면 3,079µs(불합격) → 수정 후 **12.5µs**(합격선 1,000µs) |
| 룩어헤드 late 스케줄(60/120Hz 시뮬레이션) | — | 구 공식 223건 late → 새 공식 **0건** |
| 보이스 스틸 경계 스텝 | — | 구현 0.3248 풀스케일(클릭) → 수정 후 < 0.01 |
| RT 콜백 dealloc(전역 계수 할당자) | — | 링 제거 시 54건 dealloc(위반) → 링 도입 후 **0건** |

## 8. 리뷰 findings와 수정

Wave 3 적대 리뷰(오디오 RT 안전성 / 클럭 정확성 / 앱 통합·UX 3갈래) 총 23건(critical 5·major 5·minor 13) 확정, 전부 반영. 틀렸다고 판단해 기각한 항목은 없음.

**critical**
1. 보간 클럭 계단형 붕괴(`saturating_duration_since`) → 부호 있는 차분으로 교체. 잔차 sd 3,073µs → 12.5µs(실기).
2. 룩어헤드 최소 1버퍼+1게임프레임 부족(BGM/오토플레이 온셋 100% delay 0) → `playback_ahead + (buffer_frames+2)/rate` + 앱 소유 `schedule_poll_interval_us`(프레임 시간 피크 홀드). `LOOKAHEAD_EXTRA_FRAMES`를 1→2로.
3. 오디오 콜백의 PCM free(`clear_namespace` 상시 유발) → rtrb 링으로 게임 스레드에 은퇴 샘플 반환, `collect_retired`가 드롭. 링 포화 시만 콜백이 직접 드롭(`retire_overflows`).
4. 복구 가능한 xrun(`BufferUnderrun`)을 스트림 사망으로 처리 → `stream_error_is_fatal` 4변형 분기, `BufferUnderrun`은 카운트만.
5. (추가 발견, mixer.rs) 보이스 스틸이 페이드 없이 슬롯을 제자리 덮어써 클릭 발생 → "피해 보이스의 페이드 뒤에 예약"(`Voice.pending`, 샘플 정확 시작)으로 재설계.

**major**
- RT·클릭 회귀 테스트 부재 → `rt_safety.rs` + 스틸 경계 스텝 테스트 + 게인 슬루 경계 스텝 테스트 신설.
- 검증 지표(`interp_sd_us`)가 구조적으로 상수(같은 레코드 필드 차분이라 보간 품질과 무관) → "벽시계 대비 보간 클럭의 선형 잔차 sd"(`ClockFit`, Welford 온라인 공분산)로 재정의. Play 프레임마다 샘플링(입력이 아니라).
- 재오픈 가드가 `Stage::Play`만 막아 Loading 중 재오픈 시 키음 뱅크·`#VOLWAV` 게인 유실 → 차트가 스트림을 쥔 전 구간(Play/Loading/player 보유/키음 디코드 중)에서 보류(`audio_reopen_blocked`).
- 재오픈 디바운스가 첫 변경에만 타이머를 걸어 갱신 안 됨 → 대기 중 매 프레임 데드라인 갱신(`audio_reopen_deadline`).
- AUDIO 탭이 오픈 폴백 결과를 표시하지 않음 → `settings_ui::audio_status_text`로 실제 오픈값·강등 사유·보류 안내 표시.

**minor**
- 에러 콜백 `eprintln!`이 오디오 스레드에서 stderr 락 점유 → 제거, 원자 카운터만.
- seqlock 재시도 소진 후 찢어진 레코드 가능 → 마지막 온전한 레코드로 폴백.
- 디바이스 폴백 시 `fallback_step`이 승격되지 않음 → `reported_step_floor`로 하한 강제.
- 게인 변경이 스텝(즉시 반영) → `GAIN_SLEW_MS=3ms` 선형 슬루(변화량 무관 등시간).
- `decode.rs` 수정 관련 §6 "미변경" 문구가 실측과 어긋남 → "Command 시그니처 적응만 허용, 소유는 B-audio-mixer"로 정정(코드 변경 불필요, 문서 정정으로 해소).
- 리플레이 키음이 과거 절대 시각을 예약해 `late_schedules` 오염 → 즉시 발음 센티널로 통일.
- AUDIO DEVICE 행이 키 입력마다 호스트를 열거 → 설정 진입 1회 캐시로 전환.
- 실기 lookahead 항등식 단언이 스냅샷 3회 독립 읽기라 플레이키 → 1회 스냅샷(`clocks()`)으로 통일.
- 800줄 초과 파일 4개(`main.rs`/`app_play.rs`/`app_select.rs`/`keyconfig.rs`) → Phase C 이월로 명시, 분리 대상(`app_play.rs`의 오디오 수명·재오픈·소크/오버레이 블록)을 문서에 기재.

부분 해소 1건: `scheduled_us` 실사용 0(동결 계약 API라 제거하지 않고 `clocks()` 위임 + doc으로 프레임 단위 예약 호스트용임을 명시). 문서 전용 2건: `decode.rs`(위 minor), 800줄 초과 4파일(Phase C 이월).

## 9. 소크 결과 표

빌드: `cargo build --release -p rbms-player` 정상, 경고 0. 사전 100초 프로브로 정상 기동 확인 후, 본 실행은 `RBMS_SOAK_LOG` + `--timing-csv`로 780초를 6세그먼트(세그먼트 종료 판정 = `settings.ron` mtime 갱신, 하드캡 210초)로 구동. HOME을 scratchpad로 격리해 사용자 `~/.config/rbms/`는 읽기만 했다. RSS는 `ps`로 15초 간격 52샘플 독립 수집.

| 지표 | 실측 | 합격 기준 | 판정 |
|---|---|---|---|
| underruns / drops / steals / hard_steals / late / ts_fallbacks / retire_overflows | 전부 0 | 0 (steals는 기준 없음, 참고 관측) | 합격 |
| interp_sd_us 최대 | 9.17 | < 1000 | 합격 |
| RSS 추세(마지막 5분) | -1.99 MB/분 | < 0.5 MB/분 | 합격 |
| RSS 추세(세그먼트 종료) | -0.66 MB/분, 시작 대비 -3.2% | < +15% | 합격 |
| frame_ms_p95 | 0.03ms | < 16.7ms | 합격 |
| fps | 32,784~38,538 | (화면 잠금으로 vsync 미작동, 자유 구동 — 대표성 없음, 미검증으로 별도 기재) | 미검증 |

로그 무결성: 6세그먼트 전부 동일 차트(736 keysounds, 1647 notes)로 재기동 확인, `panic|error|fail|unavailable|dead|stopped|warn` grep 0건. 종료 후 프로세스 잔존 0. 소스 미변경이므로 소크 자체는 `fmt`/`test`/`clippy` 재실행 대상이 아니다.

미검증으로 남긴 것: fps 평균의 실사용 대표성(화면 잠금으로 자유 구동), `scratch_reallocations`(소크 CSV에 컬럼 없음, 로그로도 미확인), §4.3 절차 3번 병행 세션(프리뷰 커서 이동, Play↔Select 왕복 — 키 입력 필요해 미실행).

## 10. 실기 가청 확인

**사용자 몫으로 명시.** Wave 3 계획(§5)의 "라이브 1회 가청 확인(키음 즉시성·BGM 온셋·클릭 없음·프리뷰 전환)"은 이 세션에서 실시하지 않았다. 리스크 R3(마스터 게인 0.5→1.0 재배치 실수 시 음량 2배)에 대비해 **처음 실행 시 볼륨을 낮춘 상태로 시작**할 것을 권장한다. 확인 항목: 키음이 지연 없이 즉시 나는지, BGM/autoplay 온셋이 버퍼 경계로 밀리지 않는지, 보이스 스틸/정지 시 클릭이 없는지, 곡선택 프리뷰 전환이 매끄러운지, AUDIO 탭에서 디바이스/버퍼/샘플레이트/폴리포니 변경이 실제로 반영되는지.

## 11. 미확인 사항

`docs/plan/2026-09-09-phase-b-spec.md` §8 원문 기준, Phase B 종료 시점에도 남아 있는 항목:

1. 레퍼런스 구현의 `PCM`/`AudioDriver` 세부(`PCM.java`/`FloatPCM.java`/`ShortPCM.java`/`GdxSoundDriver.java`/`PortAudioDriver.java`)는 열지 않았다. 보이스 스틸 정책·페이드 유무·리샘플 방식은 미대조 — §3.4의 스틸/램프는 **rbms 독자 설계이며 레퍼런스 패리티 주장이 아니다**(§12 divergences 참조).
2. `deviceSimultaneousSources=256`이 OpenAL 소스 수인지 rbms 소프트 보이스와 1:1 대응하는지 미확인. `POLYPHONY` 기본 512는 현행 `DEFAULT_MAX_VOICES` 유지 결정이지 패리티가 아니다.
3. 디스플레이 지연(시청각 스큐 총합)은 Phase B로도 확정되지 않는다 — 오디오 축만 정확해졌다.
4. `master_gain` 1.0 상향 후 리미터 진입 빈도가 이론상 동일 진폭이어도 버스 합산 지점 차이로 실제 다른지는 소크에서 `limiter_engaged_frames`로 별도 측정하지 않았다(오버레이/CSV에 카운터 자체는 §3.4 정정으로 언급됐으나 이번 소크 실행에서 값 확인은 생략).
5. fps 평균의 실사용 대표성, `scratch_reallocations`, §4.3 병행 세션 절차(위 §9 소크 표 하단 참조).
6. 실기 가청 확인(§10) — 사용자 몫.
