# Windows CI STATUS_ACCESS_VIOLATION — SKIN 탭 테스트

## 대상 파일

- `crates/rbms-audio/src/host.rs` (신규 — 오디오 호스트 전용 스레드)
- `crates/rbms-audio/src/lib.rs` (`mod host` + `output_device_names` 재노출)
- `apps/rbms-player/src/settings_ui.rs` (기존 `output_device_names` 제거)
- `apps/rbms-player/src/stage/settings.rs` (`SettingsState::on_enter` 호출 경로)
- 증상 최초 관측: CI `0f21ce7` / `dev`, job `test (windows-latest)`

## 증상

`cargo test --workspace` 가 windows-latest 에서만 죽었다. macOS·Linux 는 통과.

```
error: test failed, to rerun pass `-p rbms-player --lib`
Caused by:
  process didn't exit successfully: `...\rbms_player-....exe`
  (exit code: 0xc0000005, STATUS_ACCESS_VIOLATION)
```

패닉 메시지도 백트레이스도 없이 프로세스가 통째로 사라졌다. 620개 중 526개가 `ok` 를
보고한 뒤였고, 보고되지 않은 94개는 전부 `stage::settings::skin_tests` 이후 순번이었다.
즉 죽은 지점은 SKIN 탭 테스트 5개 구간이다.

- `choosing_a_document_adds_the_rows_it_declares_between_the_row_and_the_actions`
- `reset_drops_the_choices_and_the_file_no_longer_holds_them`
- `the_skin_tabs_own_rows_are_drawn_and_redraw_when_one_moves`
- `the_zero_key_puts_an_offset_row_back_to_the_authors_placement`
- `what_the_tab_edits_survives_the_settings_file`

## 원인

스킨(`rbms-skin` / json5 / Lua 샌드박스 / 이미지 워커 풀)과 무관하다. 스킨 문서를 통째로
읽고 그리는 `stage::render_tests_skin` 과 `skin_select::tests` 는 같은 실행에서 전부 통과했다.
죽은 5개에만 있는 것은 `SettingsState::on_enter` 이고, 그 첫 줄이 오디오 출력 장치 열거였다.

cpal 은 장치 열거에 쓰는 `IMMDeviceEnumerator` 를 **프로세스 전역 `OnceLock` 에 한 번만
만들어 두고 다시는 만들지 않는다**(`cpal-0.17.3/src/host/wasapi/device.rs`,
`static ENUMERATOR: OnceLock<Enumerator>` + `unsafe impl Send/Sync`). 그런데 그 객체를 만든
스레드의 COM 아파트먼트는 **그 스레드가 끝날 때 정리된다**: cpal 의 COM 초기화는
thread-local 이고, 드롭될 때 `CoUninitialize()` 를 부른다(`.../wasapi/com.rs`). `CoUninitialize`
는 문서 그대로 "closes the COM library on the current thread, **unloads all DLLs loaded by
the thread**, frees any other resources that the thread maintains" 이다. 전역 슬롯에는 이미
해제된 객체의 포인터만 남고, 다음 호출자가 그 vtable 을 읽는 순간 STATUS_ACCESS_VIOLATION 이다.

libtest 는 테스트 하나마다 스레드를 새로 띄우고 끝나면 보낸다(`--test-threads=1` 이어도
동일). 그래서 SKIN 탭 첫 테스트가 열거자를 자기 스레드 아파트먼트에 만들고, 그 스레드가
빠지면서 열거자를 데리고 나가고, 두 번째 테스트가 죽은 포인터를 밟았다. 앱 본체는
프레임 루프(메인 스레드) 한 곳에서만 열거하므로 실제 플레이에서는 드러나지 않는,
테스트 하네스가 먼저 밟은 잠복 결함이다. macOS·Linux 는 COM 이 없어 무관하다.

### CI 실험으로 확정 (`ci/windows-skin-crash`, run 34443502093)

| 실험 | 결과 |
|---|---|
| 스킨 코드 없이 짧은 스레드 4번에서 장치 열거만 | `round 0` 출력 직후 **0xc0000005** |
| 같은 루프를 호스트 스레드 경유로 | `round 0..3` 전부 정상, ok |
| SKIN 탭 5개, `--test-threads=1` | 첫 테스트 ok → 다음 테스트에서 **0xc0000005** |
| SKIN 탭 5개, `--test-threads=4` | 첫 테스트 ok → **0xc0000005** |
| SKIN 탭 5개, 각각 별도 프로세스 | 5개 전부 ok |

장치가 0개인 러너에서도 재현된다. 열거 결과가 비어 있어도 열거자 객체는 만들어지고,
스레드와 함께 사라지기 때문이다.

## 해결

오디오 호스트에 던지는 질문을 **한 번 만들고 절대 끝나지 않는 스레드 하나**로 모았다
(`crates/rbms-audio/src/host.rs`). 열거자를 소유한 아파트먼트가 프로세스 수명을 함께 가므로
누가 물어보든 살아 있는 객체를 읽는다. 테스트를 `#[ignore]` 하거나 스킵하지 않는다.

- `rbms_audio::output_device_names()` 가 호스트 스레드에 질문을 보내고 답을 기다린다.
- 질문이 패닉해도 `catch_unwind` 로 스레드 자체는 남는다. 아파트먼트가 살아 있는 것이
  전부이기 때문이다.
- 플레이어의 `output_device_names` 는 사라지고 `SettingsState::on_enter` 가 위 함수를 부른다.
  열거 비용은 여전히 호출자가 기다리므로 프레임 루프 관점의 비용은 그대로다.

`AudioEngine::open` 은 프레임 루프에서만 불리고 메인 스레드는 프로세스와 함께 살아 있으므로
그대로 두었다. 어느 쪽이 먼저 열거자를 만들든 그 스레드는 프로세스보다 오래 산다.

## 검증

- 회귀 테스트 2개 (`crates/rbms-audio/src/host.rs`)
  - `asking_from_threads_that_come_and_go_answers_the_same_list_every_time` — 크래시가
    났던 모양 그대로 짧은 스레드에서 반복 질문. 수정 전 Windows 에서 프로세스가 죽는다.
  - `every_question_is_answered_on_the_host_thread_whichever_thread_asks` — 어느 스레드가
    물어도 답은 호스트 스레드에서 나온다는 불변식. 이 불변식이 깨지면 COM 이 없는
    macOS·Linux 에서도 결정적으로 실패하므로, 같은 실수를 다시 하면 3종 전부에서 잡힌다.
- 로컬(macOS): `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test -p rbms-player` (796 passed, 0 failed).
- CI `ci/windows-skin-crash` 브랜치 run 34443930132 — `test (windows-latest)` 에서
  `rbms-player --lib` **620 passed / 0 failed** (직전 크래시 실행은 526 보고 후 프로세스 사망),
  SKIN 탭 5개 전부 ok. ubuntu·macOS·`fmt + clippy` 도 통과. 확인 후 브랜치 삭제.
