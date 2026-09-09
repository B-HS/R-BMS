# Phase B 적대 리뷰 수정 (2026-09-09)

> 대상 명세: `docs/plan/2026-09-09-phase-b-spec.md`. 리뷰에서 검증된 지적 23건(critical 5 · major 5 · minor 13)을 반영했다.
> 명세 본문에는 항목별로 "정정 (2026-09-09 적대 리뷰 반영)" 블록을 넣었다. 이 문서는 무엇을 왜 바꿨는지의 요약이다.

## 1. 판정 축 — 보간 클럭이 계단형으로 붕괴하고 있었다 (critical)

`audible_us_from` 이 경과 시간을 `saturating_duration_since` 로 구했다. 모든 백엔드가 `playback_ahead` 를
**최소 버퍼 1주기**로 보고하므로(CoreAudio `host/coreaudio/macos/device.rs:899-908` 는 정확히 1주기, 폴백 경로도
정확히 1주기) 콜백 구간 전체에서 경과가 0 으로 잘렸고, 값이 콜백당 1회만 튀는 구 클럭을 1버퍼 평행이동한 것에
불과했다. 실측 잔차 표준편차 3,080µs = 양자화 이론값 `10667/sqrt(12)`.

**부호 있는 차분**으로 바꿨다. 음수 구간(이번 버퍼가 아직 DAC 에 안 나간 구간)이 살아나고, 콜백 경계에서 값이
정확히 연속이라 단조 클램프도 그대로 성립한다. 외삽 상한의 기준점도 명세 §3.1 본문대로 `callback_at` 으로 맞췄다.

**실기 실측(MacBook Pro Speakers, 48 kHz, 버퍼 512 = 10,666µs 주기): 잔차 sd 12.5µs.** 같은 장비에서 계단형이면
3,079µs 다(§2.1 합격선 1,000µs). 재현: `cargo test -p rbms-audio --lib the_interpolated_clock_is_smooth -- --ignored --nocapture`.

이 클럭은 판정(`main.rs` press 경로) · 미스 스윕(`update_judge`) · 노트 렌더 · 리플레이 기록이 전부 읽는다.
고치기 전 판정 분해능은 10.7ms(같은 콜백 구간의 모든 입력이 동일 시각으로 판정)였다.

## 2. 룩어헤드가 최소 1버퍼 + 1프레임 부족했다 (critical, 1번과 함께 고쳐야 함)

명세의 유도가 `at_frame` 이 **audible 축이 아니라 mixer clock 축**과 비교된다는 점을 놓쳤다. 소비 시점의 mixer
clock 은 audible 위치보다 `playback_ahead + 1버퍼` 앞서 있고, 게임 스레드는 프레임당 1회만 예약한다.

- 엔진: `lookahead_us = playback_ahead + (buffer_frames + 2) / rate`. `+2` 는 µs↔프레임 절삭이 엔진과 믹서에서
  각각 한 번씩 일어나 1프레임 여유를 그대로 먹기 때문이다.
- 앱: 자기 폴링 간격(최근 프레임 시간의 피크 홀드 + 감쇠, 1~50ms 클램프)을 그 위에 더한다.

1번만 고치면 우연히 얹혀 있던 +1버퍼 바이어스가 사라져 이 문제는 오히려 악화된다 — 두 건은 함께 고쳤다.
60Hz 프레임 시뮬레이션 회귀에서 구 공식은 223건이 `delay == 0` 으로 붕괴하고, 새 공식은 0건이다.

**알려진 하한:** 곡 시작 직후 `playback_ahead + 1버퍼`(실측 약 21ms) 구간의 온셋은 물리적으로 예약할 수 없다.
그 프레임들은 첫 명령이 큐잉되기 전에 이미 장치로 넘어갔다.

## 3. 오디오 콜백이 샘플 PCM 을 free 하고 있었다 (critical, RT 위반)

`clear_namespace` 는 게임 스레드에서 뱅크 참조를 먼저 버리고 `StopRange` 만 큐잉하므로, 아직 울리고 있는 보이스가
마지막 소유자가 되고 릴리스 종료 시 콜백 안에서 수 MB PCM 이 해제됐다. 전역 계수 할당자 실측: 콜백 경로 8회에
`deallocations 2, freed 192,056 bytes`. 트리거는 Play 진입 · Play 이탈 · 프리뷰 교체(곡선택 커서 이동마다)로 전부
상시 경로다.

`Mixer` 가 은퇴한 `Arc<SampleData>` 를 rtrb 링으로 게임 스레드에 돌려보내고, 게임 스레드가
`AudioEngine::collect_retired` 에서 드롭한다(프레임마다 1회 + 모든 명령 push 경로). 링이 가득 찬 경우에만 콜백이
직접 드롭하고 `retire_overflows` 를 올린다 — 오버레이 STREAM 줄 `RETIRE-OF` 와 소크 CSV 신규 컬럼으로 관측한다.
회귀는 `crates/rbms-audio/tests/rt_safety.rs`(전역 계수 할당자).

## 4. xrun 1회로 스트림을 사망 처리하고 있었다 (critical)

에러 콜백이 `StreamError` 변형을 구분하지 않고 무조건 `alive = false` 로 만들었다. cpal 은 복구 가능한 xrun 에
대해 `BufferUnderrun` 을 올린다(ALSA `host/alsa/mod.rs:807`·`:853`·`:1045`, JACK `host/jack/stream.rs:468`).
리눅스/JACK 에서 xrun 1회면 판정 축이 남은 세션 내내 벽시계 폴백으로 넘어가고(장치/시스템 클럭 100ppm 이면 3분
곡에서 약 18ms 드리프트), 룩어헤드가 0 이 되어 온셋 붕괴가 재발했다.

`stream_error_is_fatal` 로 분기했다. 아울러 에러 콜백은 오디오 스레드에서 도는 만큼 `eprintln!` 을 제거하고
원자 카운터만 갱신한다.

## 5. 보이스 스틸이 페이드 없이 슬롯을 덮어썼다 (major)

2단(releasing)·3단(audible) 모두 제자리 덮어쓰기라 한 프레임 만에 다른 파형으로 튀었다. 실측 경계 스텝
**0.3248 풀스케일**(양쪽 동일). 스틸을 "즉시 교체"에서 **"피해 보이스의 페이드 뒤에 예약"**으로 바꿨다:
`Voice.pending` 에 요청을 얹고 릴리스가 무음에 닿는 그 프레임에 같은 슬롯에서 제자리 시작한다(샘플 정확).
대가는 풀이 꽉 찬 경우 새 소리가 최대 3ms 늦게 시작한다는 것뿐이다.

## 6. 검증 지표가 구조적으로 상수였다 (major)

`interp_sd_us` 가 `audible_us − clock_us` 의 표준편차였는데, 둘은 같은 레코드의 `frames_at_callback_start` 와
`frames_end` 에서 나오므로 차이가 항상 정확히 −1버퍼이고 sd 는 구조적으로 0 이다. 즉 소크 게이트
`interp_sd < 1000µs` 는 실패 상태에서도 무조건 통과했다.

**벽시계 대비 보간 클럭의 선형 잔차 sd** 로 재정의했다(`timing::ClockFit`, Welford 온라인 공분산). 기울기가 장치
클럭과 시스템 클럭의 상수 배율 차를 흡수하므로 드리프트는 오차로 세지 않는다. 표본은 autoplay 무인 소크에도
남도록 Play 프레임마다 넣는다.

## 7. 앱 통합 (major/minor)

- **Loading 중 재오픈**이 디코드된 키음 뱅크와 `#VOLWAV` 게인을 통째로 버렸다 → 차트가 스트림을 쥔 모든 구간에서
  보류(`audio_reopen_blocked`), 재오픈 성공 후 `chart_gain` 재적용.
- **디바운스가 디바운스가 아니었다**(첫 변경에만 타이머) → 대기 중 매 프레임 데드라인 갱신(`audio_reopen_deadline`).
- **AUDIO 탭이 오픈 폴백 결과를 전혀 보여주지 않았다** → 설정 status 줄을 AUDIO 탭이 소유하고 실제 오픈값·강등
  사유·보류 안내를 표시(`settings_ui::audio_status_text`).
- **리플레이 키음**이 과거 절대 시각을 예약해 `late_schedules` 를 오염시켰다 → 라이브 press 와 같은 즉시 발음 규칙.
  이로써 `keysound_time_us` 는 호출부가 없어져 삭제했다.
- **AUDIO DEVICE 행**이 키 입력마다 호스트를 열거했다 → 설정 화면 진입 시 1회 캐시.

## 8. 나머지 정정

- 폴백 사다리: 요청 디바이스를 못 찾았으면 보고 step 을 `STEP_DEFAULT_DEVICE` 이상으로 승격.
- seqlock 재시도 소진 후 필드별 plain read 대신 **마지막으로 온전히 읽은 레코드**를 반환(찢김 방지).
- `out_rate` 또는 `buffer_frames` 가 0 이면 룩어헤드 0(구: 1초).
- `audible/scheduled/lookahead` 를 한 스냅샷에서 함께 얻는 `AudioEngine::clocks` 추가 — 실기 테스트의 항등식
  단언이 스냅샷 3회 독립 읽기라 플레이키했다. 앱도 이 경로를 쓴다.
- `decode.rs` 는 `Command::Play` 의 `bus` 필드 추가에 따른 테스트 적응 2줄만 변경 — 명세 §6 의 "미변경" 문구를
  정정했다.
- 800줄 초과 4파일(`main.rs`·`app_play.rs`·`app_select.rs`·`keyconfig.rs`)은 Phase C 이월. `app_play.rs` 의 오디오
  수명·재오픈·소크/오버레이 블록을 분리 목록에 명시.

## 검증

- `cargo fmt --all`
- `cargo test --workspace`: 통과 1,479 · 실패 0 · ignored 2 (수정 착수 시점 1,447 통과)
- `cargo clippy --workspace --all-targets`: 경고 개별 위치 55개 — 착수 시점과 동일(신규 0). `rbms-audio` 는 0.
- 회귀가 실제로 결함을 잡는지 각각 역검증했다: 구 공식/구 구현으로 되돌리면
  잔차 sd 3,080µs · late 223건 · dealloc 54건 · 경계 스텝 0.3248 로 실패한다.

**미실시:** 실기 가청 확인과 §4.3 의 12분 소크. 소크 게이트(`interp_sd`)는 이번 수정으로 비로소 의미를 갖게 됐으므로,
소크는 이 수정 이후에 돌려야 한다.
