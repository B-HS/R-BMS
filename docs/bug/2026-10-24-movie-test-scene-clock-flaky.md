# Windows CI 에서 가끔 실패하던 GPU 동영상 시험 (장면 시계가 실제 시계)

최종 갱신 2026-10-24 · 대응 커밋 ab6f7a4

## 대상 파일

- `apps/rbms-player/src/skin_screen/movie_tests.rs`(`Pack::new`, `Pack::compile`, `Pack::start`, `a_movie_plays_on_the_gpu_backend_frame_after_frame`, `a_movie_stands_still_while_the_scene_clock_does`)
- `apps/rbms-player/src/stage/select/skinned_tests.rs`(`the_input_timer_goes_on_after_the_input_time_and_holds_no_input_back`)
- `apps/rbms-player/src/stage/render_tests.rs`(`aged_app`)
- `apps/rbms-player/src/stage/mod.rs`, `apps/rbms-player/src/stage/capture.rs`(시험 전용 `MOVIE_FRAME_PATIENCE` 를 함께 쓰려고 `pub(crate)`, 둘 다 `#[cfg(test)]` 안)

## 증상

문서만 바꾼 커밋 `80cac02` 의 ci(run 40085339231)에서 `test (windows-latest)` 만 실패했다: `pass 1, frame 4: the GPU drew [0, 255, 0, 255], wanted [96, 159, 4, 255]`. 같은 코드의 앞 커밋 `0621e2f` 는 3 OS 모두 통과했다. 가끔 실패하는 시험이었다.

## 원인

- 그려진 `[0, 255, 0]` 은 시험 동영상의 0번 프레임(`assets/scripted_movie.rs` 의 `pixel_of(0)`)이고, 기대한 것은 4번 프레임이다. 0번 프레임이 나오려면 장면 시각이 기대보다 1초(반 프레임) 넘게 앞서야 한다.
- 시험의 `Pack::set_clock` 은 `put_skin_scene_at` 으로 장면 시각을 두지만, 그 시계는 실제 시계와 함께 흐른다(`SceneClock::at`, `scene_instant()` 이 `Instant::now()`). 시계를 둔 뒤 프레임이 시각을 읽기까지 걸린 기계 시간이 장면 시각에 더해진다.
- 로드가 걸린 Windows 러너(WARP 소프트웨어 어댑터)에서 그 사이가 1초를 넘으면 다음 회차의 0번 프레임이 정당하게 "제때 프레임" 이 된다. 원래 설계는 프레임을 2초로 길게 잡아 실제 시계가 더하는 시간을 견디는 시간 여유 방식이었다.
- 재생기(`crates/rbms-video/src/player.rs`)의 경쟁은 아니었다. 디코더는 시각 순서대로만 프레임을 넣고, 낡은 프레임은 세대 번호로 버린다.

## 해결

- `Pack::new` 에서 `stop_scene_clock()` 으로 장면 시계를 멈춰, 프레임이 시험이 둔 시각을 정확히 읽게 했다.
- GPU 시험의 컴파일 대기 루프가 그리기까지 해서 동영상을 장면 시각에서 미리 시작시키고, 뒤로 가는 시계의 재시작 경로에 우연히 기대던 결합을 없앴다. 헤드리스와 같은 `compile()` → `start()` 경로를 쓴다.
- `a_movie_stands_still_while_the_scene_clock_does` 의 30 ms 잠을 `stop_scene_clock_at(parked_at + PARKED_FOR)` 로 바꿨다. 제품을 일부러 망가뜨려(멈춘 순간부터 기계 시간을 세게) 새 시험이 잡고, 옛 잠 버전은 놓치는 것을 확인했다.
- 컴파일 대기 단언은 아직 컴파일되지 않았을 때만 하고, 프레임 기다림 한도는 캡처와 같은 `MOVIE_FRAME_PATIENCE`(120초)를 쓴다.
- 같은 부류: 선곡 입력 타이머 시험과 `render_tests.rs` 의 `aged_app` 도 장면 시계를 멈췄다(700 ms 잠 주입으로 옛 입력 타이머 시험의 실패를 재현). `skin_writes/tests.rs`, `play/screen_tests.rs`, `skin_screen/tests.rs`, `texture_tests.rs` 는 점검했고 실제 시계로 문턱 직전을 단언하지 않아 그대로 뒀다.
- 제품 코드는 바꾸지 않았다.

## 검증

- macOS 에서 고치기 전 1.05초 지연 주입 재현(헤드리스·GPU 둘 다 CI 와 같은 값으로 실패) → 고친 뒤 통과.
- `skin_screen::movie_tests` 20회 연속, 입력 타이머 시험 20회 연속 모두 통과. `cargo test -p rbms-player --locked` 3,154 통과·0 실패, workspace clippy·fmt 통과.
- `ab6f7a4` 의 ci run 40094404863: ubuntu·macOS·Windows 와 fmt + clippy 모두 통과.

## 다음에 적용할 것

장면 시계(`put_skin_scene_at`, `age_skin_scene`, `start_skin_scene_clock`)를 쓰는 시험은 `stop_scene_clock()` 으로 시계를 멈추고 시각을 시험이 세게 한다. 실제 시계가 더하는 시간을 "프레임을 길게" 같은 여유로 견디게 짜지 않는다.
