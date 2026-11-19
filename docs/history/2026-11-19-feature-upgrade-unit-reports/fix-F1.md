# F1 수리 웨이브 보고 (wf_08935ad2-8ca)

> 수리 웨이브의 에이전트가 끝낼 때 낸 보고 원문이다.

- 상태: done

## 바꾼 파일

- apps/rbms-player/src/app_play.rs
- crates/rbms-audio/src/engine.rs
- crates/rbms-audio/src/host.rs
- crates/rbms-audio/src/latency.rs
- crates/rbms-audio/src/lib.rs

## 요약

U1 출력 지연 보정의 열린 발견 1~8번을 처리했습니다. 7건은 고쳤고, 8번 가운데 같은 이름 장치 둘인 경우의 이름 지정 경로만 deferred 입니다.

(1) 기본 출력 장치 따라가기: cpal 0.17.3 은 스트림을 열 때 장치가 그 순간의 시스템 기본이면 CoreAudio DefaultOutput AU 로 엽니다(device.rs:206-211). coreaudio-rs 문서에 이 AU 는 "user's selected default device in Sound Preferences 에 연결" 된다고 되어 있습니다. 또 기본 장치일 때만 끊김 오류 콜백을 더미로 둡니다(device.rs:916-924). 그래서 기본 장치로 연 스트림은 기본 출력이 바뀌면 따라간다고 판단했습니다. 다만 사용자의 기본 출력을 바꿀 수 없어서 실제로 따라가는지는 재현하지 않았습니다. 엔진은 연 순간 그 장치가 시스템 기본이었는지를 cpal DeviceId 로 비교해 둡니다(`opened_on_system_default`, 순수 함수). 지연 조회 대상은 `LatencyTarget::of_stream` 으로 정합니다. 기본 위의 스트림은 `get_default_device_id(false)`(지금의 기본 장치)에게 묻고, 그 밖의 스트림은 이름으로 묻습니다. 과제 문구와 다른 점이 하나 있습니다. 이름으로 골랐는데 그 장치가 마침 기본이었던 경우에도 cpal 이 기준을 '장치 id 가 기본과 같은가' 로 잡아 DefaultOutput 으로 열기 때문에, 이 경우도 '기본을 따라감' 으로 분류했습니다. 그대로 이름으로 물으면 소리가 옮겨 간 뒤 옛 장치의 지연을 읽게 됩니다.

(2) 프레임을 시간으로 바꿀 때 장치의 샘플레이트를 씁니다. 출력 AU 의 출력 범위(kAudioUnitProperty_SampleRate, Scope::Output, Element::Output)를 `get_property` 로 읽고, 쓸 수 없는 값이면 스트림 rate 를 씁니다(`OutputLatency::of_device_frames`). unsafe 와 새 의존성은 없습니다. 이 Mac 에서 스트림을 44.1 kHz 로 두고 물었더니 MacBook Pro Speakers 108 frames → 2.25 ms 가 나와 48 kHz 로 환산했음을 확인했습니다. 사전 탐사에서 같은 AU 의 입력 범위(클라이언트 쪽)는 44100, 출력 범위와 nsrt 는 48000 이었습니다.

(3) 알림 규칙: 보정이 30 ms 이상이고 (지연을 읽은 장치 이름, 보정 ms)가 마지막으로 알린 것과 다를 때만 알립니다(`output_latency_notice_to_give`, 순수 함수). 스트림을 열 때와 판 시작 전 재조회 양쪽에서 이 규칙을 씁니다. 알림의 장치 이름은 지연을 읽은 장치의 이름입니다. 문구는 그대로 두었고, 88자 시험(기존 notices.rs 시험)도 그대로 통과합니다.

(4) 디버그 줄 끝에 지연을 읽은 장치를 붙입니다. 형태는 `FROM [DEFAULT ]<이름>` 이고, 응답이 없으면 `ASKED ...` 입니다. 이름은 DEVICE 줄과 같은 20자에서 자릅니다.

(5) `load()` 에서 스트림이 이미 열려 있으면 재조회만 하고, 막 여는 경우에는 열기만 합니다(`prepare_audio_for_run`). 같은 조회를 두 번 하지 않습니다.

(6) PRACTICE 를 `stage_owns_chart_audio` 에 넣었습니다. 고치기 전 시험이 'stage Practice' 단언으로 실패하는 것을 확인했고, 고친 뒤 통과합니다.

(7) 믹서는 늦게 받은 소리를 건너뛰지 않고 받은 프레임에서 첫 샘플부터 통째로 틀어, 소리 전체가 늦은 만큼 밀립니다(mixer.rs `begin` 에서 delay 0 → start_in_place pos 0). 그래서 판 기준점을 예약 축으로 옮겼습니다. `AudioClocks::run_anchor_us(poll)` = scheduled_us + 폴링 간격이고, 선곡 미리듣기의 `preview_sched_us` 와 같은 축이며 지연 L 과 무관합니다. 연습 구간 재시작도 `start_play` 를 거치므로 같이 바뀝니다. 선곡 미리듣기는 이미 이 축이라 고칠 것이 없었습니다. 시뮬레이션 `late_onsets_of_a_run` 은 이제 플레이어처럼 기준점 다음 프레임부터 예약하고, 새 기준점에서는 0·160·400 ms 모두 늦은 소리가 0개, 최대 늦음 0 us 입니다. 옛 기준점(가청+L)에서는 모든 지연에서 286개 중 5개가 늦고 최대 28.687 ms 늦었습니다. 제때 예약된 소리가 들리는 순간의 곡 위치와 차트 시간의 차이는 두 기준점 모두 13 us 로 1프레임(21 us) 안이라 관계가 바뀌지 않음을 시험으로 고정했습니다.

지금 R-BMS 대비 바뀐 점(48 kHz·512 버퍼·60 Hz 기준 시뮬레이션 수치): 판 시작 직후 곡 위치가 0에 머무는 시간이 38.04 ms 늘어납니다. 백엔드 앞섬 10.67 + 버퍼 10.67 + 2프레임 0.04 + 폴링 16.67 ms 입니다. 그래서 0 ms 장치는 0 → 38.0 ms, 160 ms 장치는 160 → 198.0 ms, 400 ms 장치는 400 → 438.0 ms 입니다. 차트 첫 약 28 ms 안의 소리(차트 시간 0에 시작하는 긴 BGM 포함)는 판마다 0~28.7 ms 늦게 시작하던 것이 이제 제자리(늦음 0)에 옵니다.

(8) 조회 시한: 호스트 스레드 질문에 1초 시한(`ask_within`, `LATENCY_QUESTION_TIMEOUT`)을 두고, 시한을 넘기면 직전 값을 유지하게 했습니다. 같은 이름 장치는 기본을 따르는 경로에서는 이름을 쓰지 않게 되어 해소됐습니다. 이름으로 고른 경로는 deferred 입니다.

이 Mac 에서 스크래치 HOME 과 `--server http://127.0.0.1:9` 로 rbms-player 를 띄워 stdout 에 `audio: MacBook Pro Speakers adds 108 frames (2.2 ms), compensating 2.2 ms` 가 찍히는 것을 확인했습니다(판은 하지 않음). `pwd` 는 /Users/hyunseokbyun/development/R-BMS/.claude/worktrees/wf_08935ad2-8ca-1 이고, `git status --short` 는 M apps/rbms-player/src/app_play.rs, M crates/rbms-audio/src/{engine.rs,host.rs,latency.rs,lib.rs} 입니다.

## 항목별 결과

- [고침] 1. 기본 출력 장치 따라가기: cpal 0.17.3 device.rs:206-211 은 연 장치가 그 순간 기본이면 IOType::DefaultOutput 으로 열고, device.rs:916-924 는 기본 장치 스트림에만 더미 오류 콜백을 둡니다. coreaudio-rs types.rs:444 문서는 DefaultOutput 을 '사용자가 고른 기본 장치에 연결' 로 설명합니다. 따라간다고 판단했습니다(사용자 기본 출력을 바꿀 수 없어 실기 재현은 안 함). 엔진은 open 시작 때 host.default_output_device().id() 를 읽고, 각 rung 에서 장치 id 와 비교합니다(opened_on_system_default). 조회 대상은 LatencyTarget::of_stream(SystemDefault 면 get_default_device_id(false), 아니면 Named) 입니다. 과제와 다른 점: 이름으로 골랐지만 마침 기본이던 장치도 cpal 이 DefaultOutput 으로 열기 때문에 SystemDefault 로 분류했습니다. (시험: cargo test -p rbms-audio --locked --lib on_the_system_default (engine::a_stream_is_on_the_system_default_when_its_device_was_the_default_as_it_opened, latency::a_stream_on_the_system_default_asks_the_default_of_now_and_any_other_asks_its_device) 통과)
- [고침] 2. 프레임 → 시간 환산에 장치 샘플레이트 사용: latency.rs macos::read 가 AudioUnit::get_property::<f64>(kAudioUnitProperty_SampleRate=2, Scope::Output, Element::Output) 로 장치 rate 를 읽습니다. 쓸 수 없는 값(None·0·0.5·음수·NaN·무한대·u32 초과)이면 스트림 rate 를 씁니다(OutputLatency::of_device_frames). 실측: 스트림 44.1 kHz 로 물었을 때 SystemDefault 와 Named 모두 MacBook Pro Speakers 108 frames, 2.25 ms 로, 48 kHz 로 환산됐습니다. 사전 탐사: AU 출력 범위 48000, 입력 범위 44100, nsrt 48000. unsafe 와 새 의존성은 없습니다. (시험: cargo test -p rbms-audio --locked --lib every_output_device_answers_or_is_skipped -- --nocapture (출력 확인), a_device_s_frames_are_counted_at_the_rate_the_device_runs_at, a_device_that_gives_no_rate_to_count_at_is_counted_at_the_stream_rate)
- [고침] 3. 알림 규칙(30 ms 이상이고 (장치, ms)가 마지막 알림과 다를 때만): app_play.rs 에 순수 함수 output_latency_notice_to_give(last, reading, compensated) 를 두었습니다. 마지막 알림은 thread_local LAST_OUTPUT_LATENCY_NOTICE 에 둡니다(AppShared 필드가 더 자연스럽지만 lib.rs 가 범위 밖). 스트림을 열 때(open_audio_with)와 판 시작 전 재조회(refresh_output_latency) 양쪽에서 이 규칙을 씁니다. 알림의 장치 이름은 DeviceLatency.device_name 입니다. 문구는 바꾸지 않았고, 88자 시험은 기존 notices.rs 시험으로 통과합니다. (시험: cargo test -p rbms-player --locked --lib the_player_is_told_of_a_device_delay_again_only_when_the_device_or_the_figure_changed 통과)
- [고침] 4. 디버그 오버레이에 지연을 읽은 장치 표시: output_latency_line(reading, target, compensated) 가 '... COMPENSATED 160.0 MS  FROM [DEFAULT ]<이름>' 을 냅니다. 응답이 없으면 '... ASKED [DEFAULT ]<이름>' 입니다. 이름은 DEVICE_NAME_OVERLAY_CHARS(20)에서 자릅니다. (시험: the_debug_overlay_shows_what_the_device_reported_and_what_is_compensated, the_debug_overlay_names_the_device_the_delay_was_read_from 통과)
- [고침] 5. load() 에서 막 연 스트림을 두 번 조회하지 않음: load() 의 ensure_audio()+refresh_output_latency() 를 prepare_audio_for_run() 으로 바꿨습니다. 스트림이 열려 있으면 재조회만 하고, 없으면 ensure_audio 만 합니다. 여는 길에 open_first_rung 이 이미 조회합니다. (시험: 전용 시험 없음(실장치가 필요한 분기). cargo test -p rbms-player --locked --lib 전체 3301 passed 로 회귀 없음만 확인)
- [고침] 6. Practice 화면이 차트 뱅크 소유 목록에 없음: 고치기 전 새 시험 the_practice_screen_owns_the_chart_bank_so_a_reopen_waits_for_it 와 표 시험 reopening_the_stream_waits_while_a_chart_owns_it 이 'assertion left == right failed: stage Practice' 로 실패했습니다. stage_owns_chart_audio 에 StageId::Practice 를 넣은 뒤 통과합니다. (시험: cargo test -p rbms-player --locked --lib -- practice_screen_owns reopening_the_stream_waits result_screen_still_owns)
- [고침] 7. 판 시작 직후 첫 소리가 지난 프레임으로 예약됨: 믹서는 늦은 소리를 건너뛰지 않고 받은 프레임에서 첫 샘플부터 통째로 틉니다(mixer.rs begin: delay=at_frame-clock 포화 → start_in_place pos 0). 그래서 소리 전체가 밀립니다. AudioClocks::run_anchor_us(poll) = scheduled_us + poll 로 바꾸고, start_play 가 schedule_poll_us 를 넘깁니다. 미리듣기는 원래 같은 축(preview_sched_us)이라 바꾸지 않았고, 연습 구간은 start_play 를 거칩니다. 시뮬레이션(48 kHz·512·60 Hz, 다음 프레임부터 예약): 새 기준점은 0/160/400 ms 모두 late 0, most_late 0 이고, 옛 기준점은 late 5/286, most_late 28687 us. 제때 예약된 소리의 들리는 곡 위치와 차트 시간의 차이는 두 기준점 모두 13 us(1프레임 21 us 미만)입니다. 0에 머무는 시간은 38040 us 늘었습니다. (시험: cargo test -p rbms-audio --locked --lib -- a_run_anchored_on_the_booking_position_books_its_opening_in_time_on_any_device a_run_anchored_on_the_audible_position_starts_its_first_sounds_late_on_any_device a_sound_booked_in_time_is_heard_at_its_chart_time_whichever_anchor_the_run_took the_output_latency_holds_the_audible_position_back --nocapture; 플레이어 a_run_waiting_out_the_device_delay_books_as_far_ahead_as_a_run_on_a_device_without_one 는 새 기준점으로 갱신)
- [고침] 8a. 조회 시한 없음: host.rs 에 ask_within(timeout) 을 더했습니다. output_device_latency 는 1초(LATENCY_QUESTION_TIMEOUT) 안에 답이 없으면 None 을 돌려주고, 엔진 refresh 는 그때 직전 값을 유지합니다. 블루투스 연결 중 실제로 오래 멈추는 경우는 재현하지 못했고, 시한 동작만 결정적 시험으로 고정했습니다(질문을 asker 가 포기한 뒤에 풀어 주는 방식이라 실제 시계 속도에 기대지 않음). (시험: cargo test -p rbms-audio --locked --lib a_question_that_is_not_answered_in_time_leaves_the_asker_without_an_answer 통과)
- [미룸] 8b. 같은 이름의 출력 장치가 둘인 경우: 기본을 따르는 경로(설정에 장치 이름 없음·이름 못 찾아 기본으로 떨어짐·이름 붙은 장치가 기본)는 이제 이름이 아니라 get_default_device_id 로 물어 해소됐습니다. 기본이 아닌 장치를 이름으로 고른 경우는 남습니다. cpal 의 find_output_device 와 coreaudio-rs 의 get_device_id_from_name 이 둘 다 장치 목록에서 이름이 처음 맞는 것을 고르므로 보통 같은 장치이지만 보장되지는 않습니다. cpal Device::id() 의 UID 로 장치를 찾는 안전한 함수가 coreaudio-rs 0.14.2 에 없고 unsafe 는 금지라 고치지 않았습니다. 같은 모델 장치 두 대가 없어 재현 수단도 없습니다. (시험: 없음(실기: 같은 모델 USB DAC 두 대))

## 그 밖에 찾은 것

- [medium, 고침] 판 기준점이 가청 축이라 판 시작 직후 첫 약 28 ms 의 소리가 판마다 다른 양만큼 늦게 시작함: 시뮬레이션: 옛 기준점은 286개 중 5개가 늦고 최대 28.687 ms 늦었습니다(지연 0 에서도). 믹서는 늦은 소리를 통째로 밉니다. run_anchor_us 를 예약 축(scheduled_us + 폴링)으로 옮겨 고쳤습니다.
- [medium, 고침] 기본 출력으로 연 스트림의 지연 조회가 연 순간의 장치 이름에 고정됨: cpal 은 그 순간 기본인 장치를 DefaultOutput AU 로 엽니다(device.rs:206-211, 916-924). 조회를 LatencyTarget::SystemDefault(get_default_device_id)로 바꿔 고쳤습니다. 실기 확인(실행 중 AirPods 연결이나 해제)은 아직 안 했습니다.
- [low, 고침] Practice 화면에서 미뤄 둔 스트림 다시 열기가 차트 뱅크를 버릴 수 있음: stage_owns_chart_audio 에 Practice 가 없었습니다. 시험으로 재현한 뒤 고쳤습니다.
- [low, 안 고침] 마지막 알림 상태를 AppShared 가 아니라 thread_local 에 둠: AppShared 구조체가 lib.rs(범위 밖)에 있어 app_play.rs 의 thread_local LAST_OUTPUT_LATENCY_NOTICE 에 두었습니다. 프레임 스레드 하나가 오디오를 여는 구조에서는 동작이 같습니다. 메인이 원하면 AppShared 필드로 옮길 수 있습니다.
- [low, 안 고침] CourseResult·LayoutPick 이 차트 뱅크를 쥐는지 확인하지 않음: stage_owns_chart_audio 목록에 없는 다른 화면입니다. 이번 과제 범위는 Practice 뿐이라 확인하지 않았습니다(의심 아님, 미조사).

## 남은 위험

- 기본 장치를 따라가는지는 cpal·coreaudio-rs 소스와 문서로만 판단했습니다. 실기(OUTPUT DEVICE 를 시스템 기본으로 두고 실행 중 AirPods 연결이나 해제 뒤 차트를 열어 디버그 OUTPUT LATENCY 줄의 FROM DEFAULT <장치> 확인)는 하지 않았습니다.
- 판 시작 직후 곡 위치가 0에 머무는 시간이 버퍼·백엔드 앞섬·폴링만큼(48 kHz·512·60 Hz 에서 약 38 ms) 늘어납니다. 스킨 PLAY 타이머 기준 애니메이션은 그만큼 더 앞서고, 디버그 'CLOCK quantised gap' 값도 그만큼 커집니다. 판정과 노트 그리기가 들리는 소리와 맺는 관계는 그대로입니다(시험으로 고정).
- 첫 예약은 기준점을 잡은 다음 프레임에 일어납니다. 그 프레임 간격이 폴링 간격(직전 프레임 시간의 감쇠 최대값)보다 길면 첫 소리가 그 차이만큼 늦을 수 있습니다(예: 첫 PLAY 프레임에서 큰 텍스처를 올리는 경우).
- 조회 시한 1초를 넘기면 직전 값을 씁니다. 장치가 바뀌는 중이었다면 그 판은 옛 장치의 보정으로 시작합니다. 시한을 넘긴 질문은 호스트 스레드에 남아 뒤의 질문(장치 목록 등)을 기다리게 합니다(기존과 같음).
- macOS 가 아닌 OS 컴파일(cfg(not(macos)) 경로의 LatencyTarget 인자, nothing_is_read_off_macos 시험)은 로컬에서 확인하지 못했습니다.
- 기본을 따르는 스트림이 다른 장치로 옮겨 가도 overlay 의 DEVICE 줄(AudioOpenReport.device_name)은 연 순간의 이름을 보입니다. 실제 장치는 OUTPUT LATENCY 줄의 FROM DEFAULT <이름> 이 보여 줍니다.

## 갱신이 필요한 문서

- docs/reference/2026-11-19-feature-upgrade-12/README.md 1.1·공통: 판 기준점은 이제 AudioClocks::run_anchor_us(poll) = scheduled_us + 폴링(예약 축, 지연과 무관)이고, 옛 '가청 위치 + 지연' 설명을 고쳐야 합니다. 지연 조회 대상은 LatencyTarget(SystemDefault 는 지금의 기본 장치)이고, 프레임 환산은 장치 rate(AU 출력 범위 SampleRate)로 합니다.
- docs/acknowledge ADR: '판 기준점을 예약 축으로 옮김(시작 정지 시간 약 38 ms 증가, 첫 소리 최대 28.7 ms 늦음 해소)', '기본 장치 위 스트림은 지금의 기본 장치에게 지연을 물음(이름으로 골랐어도 연 순간 기본이면 포함)', '알림은 30 ms 이상이고 (장치, ms)가 바뀔 때만', '지연 조회 시한 1초, 넘기면 직전 값 유지'
- docs/plan/2026-11-19-feature-upgrade-12.md 7절 U1 행 또는 후속: 알림 반복 정책 결정이 반영됨
- docs/bug: Practice 화면 재열기로 차트 뱅크가 버려지는 결함, 판 시작 첫 소리 늦은 예약 결함(증상·원인·해결)
- docs/quality-assurance: 실기 항목 'OUTPUT DEVICE 시스템 기본에서 실행 중 AirPods 연결이나 해제 → 다음 판 로딩 때 알림 1회와 디버그 OUTPUT LATENCY FROM DEFAULT <장치>', '판 시작 직후 차트 0 ms BGM 이 제때 들림', '같은 장치에서 BUFFER·SAMPLE RATE 를 바꿀 때 알림이 다시 뜨지 않음'. 테스트 부채: 같은 이름 장치 둘(이름 지정 경로), 블루투스 연결 중 조회 지연
- docs/crates.md: rbms-audio 의 LatencyTarget·DeviceLatency·OutputLatency::of_device_frames, output_device_latency 시그니처 변경(Option<Option<DeviceLatency>>), AudioEngine::output_latency_reading·output_latency_target, run_anchor_us(poll)

## 돌린 검사

- `cargo test -p rbms-player --locked --lib -- practice_screen_owns reopening_the_stream_waits result_screen_still_owns (수리 전)` → FAILED 2개: 'assertion left == right failed: stage Practice'(재현)
- `cargo test -p rbms-audio --locked` → lib 222 passed, 0 failed, 3 ignored; 추가 테스트 대상 1 passed(그 뒤 새 시험 하나를 더 넣고 해당 2개 시험을 따로 통과)
- `cargo test -p rbms-audio --locked --lib -- run_ anchored audible_position_starts heard_at_its_chart_time output_latency every_output_device question_that_is_not frames latency --nocapture` → 22 passed. 옛 기준점 late 5/286, most_late 28687 us; 새 기준점 hold 38040/198040/438040 us; SystemDefault 와 Named 모두 MacBook Pro Speakers 108 frames 2.25 ms(스트림 44.1 kHz)
- `cargo test -p rbms-audio --locked --lib on_the_system_default` → 2 passed
- `cargo test -p rbms-player --locked --lib` → 3301 passed, 0 failed, 2 ignored (75 s)
- `cargo clippy -p rbms-audio -p rbms-player --all-targets --locked -- -D warnings` → exit 0(경고 0). 마지막 engine 시험 추가 뒤 rbms-audio 를 다시 돌려 통과
- `cargo fmt --all --check` → exit 0
- `HOME=<스크래치> ./target/debug/rbms-player --server http://127.0.0.1:9 (stdout 확인 뒤 종료)` → audio: MacBook Pro Speakers — 48000 Hz, 2 ch / audio: MacBook Pro Speakers adds 108 frames (2.2 ms), compensating 2.2 ms
- `Windows·Linux 컴파일` → 실행하지 못함: aarch64/x86_64-apple-darwin 타깃만 설치돼 있어 CI 3 OS 확인이 필요합니다
