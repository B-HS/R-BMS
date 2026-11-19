# O1 테스트 웨이브 보고 (wf_3df278ff-bb7)

> 테스트 웨이브의 에이전트가 끝낼 때 낸 보고 원문이다. 메인이 병합하면서 바꾼 것은 계획서 9절에 있다.

- 상태: done

## 바꾼 파일

- apps/rbms-player/src/app_play.rs
- crates/rbms-audio/src/engine.rs
- crates/rbms-audio/src/mixer.rs

## 요약

U1 반례 사냥 결과, 결함 하나를 시험으로 증명하고 고쳤습니다. 보정이 켜진 판은 시작 후 L 동안 곡 위치가 0에 머뭅니다. 그런데 예약 위치를 그 멈춘 위치에서 계산해서(`song + lookahead + poll`, lookahead 안에 L 포함), 첫 L 동안 엔진이 요구하는 것보다 최대 L만큼 앞까지 소리를 미리 예약했습니다. 믹서는 같은 키의 새 재생을 받는 순간 그 키를 멈추고, 아직 제 프레임을 기다리던 앞 소리는 페이드할 것이 없어 끝내 울리지 않습니다. 그래서 판 시작이나 연습 구간 시작 직후 약 200 ms 안에 같은 음원(드럼 등)이 두 번 나오면 첫 번째가 사라졌습니다. 연습 구간은 곡 중간에서 시작하므로 거의 매번 이 조건에 듭니다. OFF 에서는 나오지 않는, U1 이 만든 회귀입니다. 수리: `app_play.rs` 에 순수 함수 `song_and_booking_lead_us` 를 두었습니다. 리드는 엔진 리드에서 clamp 가 곡 위치를 가청 축 위로 올려 둔 만큼을 뺀 값이고, 예약 위치는 '가청 위치 - 기준점 + lookahead' 입니다. `song_and_lookahead_us` 가 이 리드를 돌려주므로 `tick_run` 은 바꾸지 않았습니다. 판정·그리기·리플레이가 쓰는 곡 위치는 그대로입니다. OFF 에서는 clamp 가 마이크로초 단위로 걸릴 때만 예약 위치가 엔진의 scheduled_us 와 같아집니다. 시험 세 가지: (1) 플레이어 시험 `a_run_waiting_out_the_device_delay_books_as_far_ahead_as_a_run_on_a_device_without_one`. 고치기 전에는 160 ms 장치 시작 순간의 예약 위치가 170687 us 로 나와 기대값 10687 us 와 달라 실패했고, 고친 뒤 통과합니다. (2) 결과가 실제로 소리를 잃는다는 것을 믹서 시험 `a_play_of_a_key_drops_an_earlier_play_of_it_still_waiting_for_its_frame` 로 고정했습니다. (3) 엔진 시뮬레이션 `late_onsets_of_a_run` 이 고치기 전 플레이어와 같은 방식으로 예약하고 있어서, 고친 방식으로 바꾸고 문서를 맞췄습니다.

## 기준별 판정

- [pass] 같은 결함(가청 위치 기준점으로 첫 L 동안 늦은 예약)이 다른 기준점에 남아 있는지: 선곡 미리듣기(자동 재생·#PREVIEW 시작·반복): preview.rs 는 기준점과 반복 경계를 모두 `audible + lookahead + poll`(엔진 scheduled 축)로 잡습니다. 이 축은 L 과 무관하다는 것을 기존 엔진 시험 the_output_latency_holds_the_audible_position_back_and_leaves_the_booking_position_where_it_was 가 증명합니다(scheduled_us 불변). 늦은 예약도, 토글할 때 튀는 일도 없습니다.
- [pass] 연습 구간 재시작·재시도·코스 다음 스테이지·플레이 배속 100% 아님 경로의 기준점: 세 경로 모두 start_play(run_anchor_us)를 거칩니다(start_practice_slice 869행, Loading 368행, 플레이 장면 Started 1490행, enter_loaded_chart 783행). 배속은 practice_clock 이 엔진 축에서 선형 변환하므로 엔진 축 예약 위치가 같습니다. 대신 이 경로들에서 위 예약 앞섬 결함이 나왔고, 고쳤습니다.
- [pass] 리플레이 분석의 탐색·정지·재개·배속: session.rs 의 toggle_analysis_pause·adjust_analysis_rate·seek 는 모두 manual=true 로 바꾸고 되돌리는 길이 없습니다. tick_run 은 manual 이면 가상 시계와 NullSink 를 써서 오디오 기준점을 다시 잡지 않습니다. 정지 뒤에 남는 예약 소리는 렌더 위치 기준으로 buf+poll 이라 U1 전과 같고, L 부분은 이미 장치 파이프라인에 들어간 소리입니다.
- [pass] 곡 위치가 L 동안 0 에 머무는 구간의 판정·그리기·BGA·타이머·음수 위치: song_position_us 의 clamp(song_us_last 는 start_play 에서 0)로 곡 위치·입력 판정 위치가 음수가 되지 않습니다. 기존 시험 an_audio_clock_behind_the_anchor_is_clamped_to_the_start 와 새 시험이 확인합니다. 스킨 PLAY 타이머가 차트 시간보다 L 앞서는 점, 그 사이 입력이 0 으로 판정되는 점은 U1 보고에 있는 절충이고 결함으로 보지 않았습니다. 예약 쪽은 고쳤습니다.
- [pass] 스트림이 죽었을 때 벽시계 대체·다시 열기 직후·판 밖에서 OUTPUT LATENCY 토글: audio_clocks_at 의 대체는 죽은 순간의 audible_us(L 포함)에서 이어 가므로 연속이고, 이때 lookahead 는 0 입니다. 토글은 L 만큼 audible 과 lookahead 를 반대로 움직여 scheduled 가 그대로라 미리듣기가 겹치거나 끊기지 않습니다. 재열기 때는 새 엔진에 보정을 먼저 넣고 저장합니다(open_audio_with 945행). 실제 장치로 토글해 보지는 않았습니다.
- [pass] macOS 가 아닌 OS 컴파일과 장치 없는 CI: gh run view 44391494818(4cbb323, U1 포함)과 44398612164: test(macos·ubuntu·windows)와 ubuntu 의 fmt+clippy(--workspace --all-features -D warnings)가 모두 success 입니다. 비 macOS 의 read_output_latency 는 매개변수가 `_` 접두라 미사용 경고가 없습니다.
- [pass] OFF 일 때 이전 시계와 완전히 같은지: OFF 에서는 compensated_output_latency 가 0 이고 snapshot_of 에 0 이 더해지며 run_anchor_us 는 audible_us 와 같습니다. 엔진 시험 the_device_delay_is_compensated_only_when_asked_and_only_up_to_its_limit 가 확인합니다. 이번 수리 뒤 OFF 의 곡 위치는 그대로이고, 예약 위치는 clamp 가 걸릴 때만(마이크로초) 엔진 scheduled_us 와 같아집니다.
- [pass] 장치 이름으로 CoreAudio 장치를 다시 찾는 방식(이름 표기 차이): cpal 0.17.3 의 Device::description 은 coreaudio-rs 의 같은 get_device_name(kAudioDevicePropertyDeviceNameCFString)을 씁니다(cpal device.rs:49, 391). 두 이름 문자열은 같습니다. 같은 이름의 장치 둘과 기본 장치 추종은 findings 에 의심으로 적었습니다.
- [pass] 로딩 중 재조회가 호스트 스레드를 막는 시간: 이 Mac(스피커만 연결)에서 every_output_device_answers_or_is_skipped 가 장치 열거와 조회를 합쳐 0.09 s 에 끝났습니다. 블루투스 장치가 바뀌는 중일 때의 지연은 재지 못했습니다(findings).
- [fail] 30 ms 알림이 스트림을 열 때마다 반복되는지: open_audio_with 가 열 때마다 announce_output_latency 를 부르므로 BUFFER·SAMPLE RATE·POLYPHONY·DEVICE 를 바꿀 때마다(reopen_audio) 같은 알림이 다시 뜹니다. OFF 에서 AUTO 로 토글할 때는 알림이 없습니다. 계획의 '스트림을 연 뒤 한 번'에 문자 그대로는 맞으므로 정책 판단으로 넘기고 고치지 않았습니다.

## 찾은 것

- [high, 고침] 보정 판의 첫 L 동안 예약이 멈춘 곡 위치에서 계산돼 L 만큼 더 앞서 예약되고, 같은 키의 앞 소리가 울리지 않음: tick_run 의 schedule_position_us(engine_song_us, lookahead_us, ...)에서 engine_song_us 는 clamp 된 0 이고 lookahead 에 L 이 들어 있어, 시작 순간 예약 위치가 L+ahead+buf+poll 이었습니다(OFF 는 ahead+buf+poll). Mixer::start_voice 가 같은 키를 stop 하고, 지연 중인 보이스는 env=0 이라 release 에서 바로 꺼집니다. 고치기 전 새 시험 실패 출력: 'left: 170687, right: 10687'. song_and_booking_lead_us 로 '가청-기준점+lookahead' 에서 예약하도록 고쳤습니다. (시험: cargo test -p rbms-player --locked --lib a_run_waiting_out_the_device_delay_books_as_far_ahead_as_a_run_on_a_device_without_one ; cargo test -p rbms-audio --locked --lib a_play_of_a_key_drops_an_earlier_play_of_it_still_waiting_for_its_frame)
- [medium, 안 고침] (의심) 시스템 기본 출력으로 연 스트림이 기본 장치를 따라가면, 재조회가 처음 연 장치 이름으로 엉뚱한 장치의 지연을 읽음: cpal 은 기본 장치 스트림을 IOType::DefaultOutput AU 로 만들고 오류 콜백도 dummy 로 둡니다(cpal device.rs:206-241, 905-913). 기본 장치가 바뀌면 소리가 새 장치로 옮겨 간다는 뜻으로 읽힙니다. 그런데 AudioEngine.device_name 은 연 순간의 이름에 고정돼 있어 refresh_output_latency 가 옛 장치를 묻습니다. 예: 스피커로 연 뒤 AirPods 연결이면 보정 2 ms(160 ms 미보정). AirPods 로 연 뒤 메뉴 막대에서 스피커로 바꾸면 스피커 소리에 160 ms 오보정. 시스템 기본 출력을 바꿔야 확인할 수 있어(사용자 환경 변경) 재현하지 않았습니다. 실기 점검 항목이 필요합니다. (시험: 실기: OUTPUT DEVICE 를 시스템 기본으로 두고 실행 중 AirPods 연결이나 해제 뒤 차트를 열어 디버그 줄 OUTPUT LATENCY 를 확인)
- [low, 안 고침] (기존, U1 이전부터) 차트 시간 0~약 ahead+buf 의 소리가 지난 프레임으로 예약돼 늦게 시작함: 기준점이 가청 위치(OFF)이거나 가청+L(AUTO)이어서, 그 순간 렌더 위치보다 ahead+buf-δ 만큼 뒤에 있습니다. engine.rs 시험 late_onsets_of_a_run 의 late_without 가 0 이 아니어도 같다는 것만 단언합니다. 기준점을 scheduled 축으로 옮기면 OFF 시계가 바뀌어 U1 완료 기준('OFF 에서 지금과 같은 시계')과 어긋나므로 고치지 않았습니다. (시험: engine.rs late_onsets_of_a_run(Duration::ZERO, AudioClocks::run_anchor_us).0 이 0 보다 큼)
- [low, 안 고침] 출력 지연 알림이 같은 장치에서 버퍼·샘플레이트·폴리포니를 바꿀 때마다 다시 뜸: app_play.rs open_audio_with 가 열 때마다 announce_output_latency 를 부릅니다. reopen_audio 는 설정 화면의 BUFFER 등을 바꿀 때마다 돕니다. OFF 에서 AUTO 로 토글할 때는 알림이 없습니다. '같은 장치·같은 ms 면 다시 알리지 않음' 같은 규칙을 정하는 것은 정책 결정이라 메인에 넘깁니다. (시험: 없음(정책 결정 대기))
- [low, 안 고침] (의심) 같은 이름의 출력 장치가 둘이면 엉뚱한 장치의 지연을 읽을 수 있음: latency.rs 는 get_device_id_from_name 으로 이름이 처음 맞는 출력 장치를 고릅니다. 기본 장치가 같은 이름의 두 번째 장치라면 cpal 은 기본 장치로 열고 조회는 첫 번째 장치를 묻습니다. cpal 의 UID(Device::id)로 찾는 안전한 함수가 coreaudio-rs 0.14.2 에 없습니다. 장치가 없어 재현하지 않았습니다. (시험: 실기: 같은 모델 USB DAC 두 대)
- [low, 안 고침] (의심) 장치의 실제 샘플레이트가 스트림과 다르면 프레임→시간 환산이 틀림: of_frames 는 스트림 rate 로 나눕니다. cpal 이 열 때 장치 nominal rate 를 맞추지만(cpal set_sample_rate), 다른 앱이 나중에 rate 를 바꾸면(예: AirPods 가 HFP 로 떨어짐) 장치 프레임 단위가 달라집니다. 7680 프레임 기준 44.1 kHz 와 48 kHz 사이 오차는 약 14 ms 입니다. AU 출력 범위의 kAudioUnitProperty_SampleRate 로 장치 rate 를 읽는 안전한 길이 있습니다. 재현하지 않았습니다. (시험: 실기)
- [low, 안 고침] (의심) 장치 조회에 시한이 없어 CoreAudio 가 오래 걸리면 로딩 중 게임 스레드가 멈춤: host::ask 는 receive.recv() 로 무기한 기다립니다. load() 에서 refresh_output_latency 가 AU 를 만들고 초기화합니다. load() 바로 앞에서 ensure_audio 가 스트림을 새로 열었으면 같은 조회가 두 번 돕니다. 이 Mac 의 측정은 0.09 s 이하였고, 블루투스 장치가 연결·해제되는 중일 때는 재지 못했습니다. (시험: 실기: AirPods 연결 중 차트 로딩)
- [low, 안 고침] (기존, U1 밖) Practice 화면이 audio_reopen_blocked 에 없어, 미뤄 둔 재열기가 연습 화면에서 일어나 차트 뱅크를 버릴 수 있음: stage_owns_chart_audio 는 Decide·Play·Loading·Result 뿐입니다. Practice 는 구간 사이에 같은 엔진 뱅크를 다시 씁니다(start_practice_slice). 재현 시험은 만들지 않았습니다. 보정값 토글 반영(poll_audio_reopen)도 같은 조건을 따르지만 연습 화면에서는 설정을 바꿀 수 없어 영향이 없습니다. (시험: 없음)

## 남은 위험

- 실제 AirPods 로 판을 플레이해 첫 L 동안의 같은 키 연타가 들리는지는 확인하지 못했습니다(장치 미연결). 결함과 수리는 순수 함수 시험과 믹서 시험으로만 증명했습니다.
- 수리 뒤 예약 위치가 clamp 가 걸린 순간 마이크로초만큼 뒤로 갈 수 있습니다. update_schedule 의 커서는 앞으로만 가므로 다시 예약되지는 않습니다.
- 기본 장치 추종·같은 이름 장치·장치 rate 변경·조회 시한은 실기 점검 전까지 의심으로 남습니다.

## 갱신이 필요한 문서

- docs/reference/2026-11-19-feature-upgrade-12/README.md 1절 또는 공통: 보정 판의 예약 리드는 clamp 된 곡 위치가 아니라 가청 축 기준(song_and_booking_lead_us)이라는 점
- docs/quality-assurance: AirPods 실기 항목에 '판·연습 구간 시작 200 ms 안의 같은 음원 연타가 모두 들림', '시스템 기본 출력 전환 뒤 디버그 OUTPUT LATENCY 값이 실제 장치와 맞음' 추가
- docs/bug: U1 예약 앞섬으로 같은 키의 첫 소리가 사라지던 결함(증상·원인·해결)
- docs/plan/2026-11-19-feature-upgrade-12.md 7절 U1 행 또는 후속: 알림 반복 정책 결정 대기

## 돌린 검사

- `cargo test -p rbms-player --locked --lib a_run_waiting_out_the_device_delay (수리 전)` → FAILED: booking position left 170687 right 10687 (160000 us device, 0 us in)
- `cargo test -p rbms-audio --locked --lib` → 216 passed, 0 failed, 3 ignored
- `cargo test -p rbms-player --locked --lib` → 3273 passed, 0 failed, 2 ignored (77 s)
- `cargo clippy -p rbms-audio -p rbms-player --all-targets --locked -- -D warnings` → 통과(경고 0)
- `cargo fmt --all --check` → exit 0
- `cargo test -p rbms-audio --locked --lib every_output_device_answers_or_is_skipped -- --nocapture` → MacBook Pro Speakers 108 frames 2.25 ms, 0.09 s(AirPods 미연결이라 블루투스 값은 못 봄)
- `gh run view 44391494818 / 44398612164 --json jobs` → test macos·ubuntu·windows, fmt+clippy(ubuntu) 모두 success. 비 macOS 컴파일·장치 없는 CI 확인
