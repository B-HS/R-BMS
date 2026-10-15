# ADR: 키보드 입력은 화면 대기와 떨어진 이벤트 루프가 받은 순간으로 판정하고, 레인 키에 디바운스를 둔다

> 최종 갱신 2026-10-15 · 대응 커밋 ea698a9
> 상태: 유효(기본값 16 ms 와 컨트롤러 LR2 형은 사용자 확인 끝, 아래 "확인받은 것") · 결정일 2026-10-15 · 결정자 사용자(S19 결정 2·3), 디바운스 방식은 결정 2 의 "LR2 등에서 쓰이는 방법으로"에 따라 LR2 형 · 커밋 `794361b`(결정 1~3), `1676c57`(컨트롤러 LR2 형·PAD DEBOUNCE, 남은 한계 4), `c81eda1`(rbms-cli 의 PAD DEBOUNCE 표시)

## 배경

- [입력 시각 판정 ADR](2026-10-15-adr-event-time-judging.md)의 "남은 한계"(감사 PERF-2, 테스트 부채 3.53). 키보드는 `window_event` 의 `KeyboardInput` 이 `Instant::now()` 를 판정 시각으로 넘긴다. 그런데 메인 스레드 하나가 이벤트 수신·갱신·그리기·present 대기를 모두 했고, macOS 의 winit 은 `RedrawRequested` 처리(그 안의 `get_current_texture`, Metal `nextDrawable` 대기 포함)가 끝난 뒤에야 다음 키를 넘겨준다. 같은 구조의 하네스(M5 Max 내장 120 Hz, Metal, winit 0.30.13, wgpu 29.0.4)에서 벽시계의 98.6~99.5 % 가 그 처리 안이었고, 그동안 들어온 이벤트는 평균 4.5~5.6 ms, p95 8.0~8.3 ms 늦게 받혔다(0~1 프레임 균등).
- 사용자 원문(결정 2): "2번은 beatmania iidx, lunatic rave 2(lr2)등에서 쓰이는방법으로 제대로 정확하게 구현해". 결정 3: "키보드에도 짧은 디바운스(설정 가능)로 채터를 막는다".
- 조사(S19 선행 사례 조사, 원본 소스 사본은 세션 스크래치):
  - **LR2**(디컴파일 재작성본 OpenLR2 기준. 원본 바이너리와 같다는 보장은 없다): DxLib 로 키보드·패드를 폴링하고 폴링한 순간의 QPC 를 시각으로 쓴다(`En_input.cpp:171-345, 290-329`). 입력 폴링과 판정을 하는 `ProcGame` 은 최대 500 Hz 이고(`POLLGAP` 2 ms, `Scene04_Play.cpp:1229-1235`), vsync 가 켜져 있으면 그리기를 마친 뒤 다음 vblank 직전까지 이 루프를 돈다(`Main.cpp:2400-2408`). 스레드 옵션을 켜면 별도 스레드에서 돈다. 그리는 동안에는 폴링하지 않는다.
  - **LR2 의 디바운스**(`inputinterval`, 기본 16 ms, `LR2_configsave.cpp:970`, `En_input.cpp:276-292, 361`): 키마다 상태 0(뗌)·1(이번 폴링에 누름)·2(누르고 있음)·3(이번 폴링에 뗌)과 누른 시각을 둔다. 뗀 상태(0·3 아님)에서 누르면 그 폴링에서 바로 1 이 되고 시각을 기록한다. 즉 **누름은 절대 늦추지 않는다.** 1·2 에서 떼었는데 누른 지 interval 이 지나지 않았으면 2(누르고 있음)로 남겨 두고, 지난 첫 폴링에서 3 이 된다. 즉 **누른 뒤 창 안의 뗌만 창 끝까지 미룬다.** 3 의 다음 폴링에 눌려 있으면 1 이 아니라 2 가 된다. 즉 **뗀 직후 한 폴링 안의 누름은 새 누름이 아니다.** 키보드·패드·MIDI 모두 같다.
  - **beatoraja 의 디바운스**(`KeyBoardInputProcesseor.java:99-146`)는 누름·뗌 어느 쪽이든 마지막 변화 뒤 창 동안 막는다. 그래서 뗀 직후 창 안에 다시 누르면 그 누름이 창 끝까지 늦는다.
  - **beatoraja**: 1 kHz 폴링 스레드(`MainController.java:355-370`)와 약 0.5 ms 주기의 판정 스레드(`KeyInputProccessor.java:140-205`, `JudgeManager.update` 의 유일한 호출부). 시각은 폴링한 순간의 `System.nanoTime`. 다만 키보드 상태는 LWJGL2 의 키 버퍼가 렌더 루프의 `Display.processMessages` 에서만 갱신되어 렌더 루프 주기(기본 vsync 꺼짐·240 fps, 약 4.2 ms)로 양자화된다(LWJGL2 소스와 libGDX 1.9.8 소스의 조합으로 본 추정).
  - **IIDX**: 공식 자료가 없다. 커뮤니티 자료로는 고정 주사율(60/120 Hz) 전용 하드웨어에서 프레임 단위로 진행하는 엔진으로 보인다(추정). I/O 폴링 주기는 확인하지 못했다. PC 의 가변 프레임 환경에 그대로 옮길 모델은 아니다.
  - **Etterna**: OS 별 입력 스레드가 받은 순간의 `steady_clock` 으로 곡 시각을 되돌려 판정한다(`Player.cpp:1881-1895`). **osu!framework**: 메인(창) 스레드는 입력만 받고 Update·Draw 는 다른 스레드다.
  - 공통 원리: **입력을 받는 코드가 그리기·present(vsync) 대기에 막히지 않게 하고, 받은 순간의 단조 고해상도 시계로 시각을 찍어 그 시각을 곡 시계로 바꿔 판정한다.** OS 이벤트 자체의 시각을 쓰는 곳은 Etterna 의 X11 경로뿐이다.
- 제약: unsafe 금지(워크스페이스 `unsafe_code = "deny"`), 새 의존성 금지. winit 0.30.13 의 `KeyEvent` 에는 시각이 없고 macOS 는 `DeviceEvent::Key` 도 내지 않는다. macOS 에서 이벤트 루프는 메인 스레드에서만 만들 수 있다. `SkinLibrary`(mlua `Rc`)가 `Send` 가 아니고 글꼴·테마가 `thread_local!` 이라, 갱신·그리기를 통째로 다른 스레드로 옮길 수 없다.

## 결정

1. **화면 대기(present)를 전용 스레드로 뗀다.** `gpu/present.rs` 의 `Presenter` 가 `rbms-present` 스레드에서 다음 이미지를 기다리고(`Surface::get_current_texture`), 받으면 `EventLoopProxy` 로 이벤트 루프를 깨운다(`FrameReady`). 이벤트 루프는 그 이미지를 가져가(`Gpu::frame_ready`) 갱신·그리기·제출하고, 이미지를 다시 그 스레드에 넘겨 present 하게 한 뒤 다음 이미지를 요청한다(`Gpu::render`). 이벤트 루프는 `ControlFlow::Wait` 로, 프레임과 프레임 사이에 창과 이 깨움 말고는 아무것도 기다리지 않는다. 그래서 키는 OS 가 창에 넘긴 즉시 `window_event` 에 들어오고, 그 순간의 `Instant::now()` 가 판정 시각이다(지금까지와 같은 경로, `AppShared::song_us_at`). 이것이 LR2 의 "vblank 를 기다리는 동안 입력을 받는다"와 osu!framework 의 "창 스레드는 입력만, 그리기 대기는 다른 곳"을 R-BMS 의 제약 안에서 옮긴 것이다.
2. **화면 대기 외의 일은 그대로 메인 스레드에 둔다. 메인 스레드는 화면을 기다리지 않는다.** 갱신·그리기 목록 작성·인코딩·제출, 창 조작(`pre_present_notify` 포함), 표면 재설정(`configure`)은 메인 스레드다. 전용 스레드는 `Window` 메서드를 부르지 않는다(macOS 에서 메인으로 동기 디스패치되어 교착할 수 있다).
   - **표면 재설정.** 나가 있는 이미지가 없어야 하므로(wgpu 의 `configure` 계약), 크기 변경은 받아 둔 이미지를 버린다. 요청 중인 이미지가 없으면 바로 재설정하고, 있으면 "재설정 대기"만 기록한다(`Gpu::resize`). 그 이미지가 오면 `Gpu::frame_ready` 가 그것을 버리고 재설정한 뒤 다음 이미지를 요청한다. 메인이 요청 중인 이미지를 기다리지 않으므로, DX12 에서 Present 중 창 스레드로 오는 동기 메시지, macOS 의 창 끌어 크기 바꾸기, Wayland 에서 보이지 않는 표면의 획득에 메인이 막히지 않는다. 이미지를 얻지 못한 경우(가려짐·Outdated 등)는 예전처럼 프레임을 돌리고 표면을 재설정한다.
   - **불변식: 이미지를 요청해 둔 동안 메인은 queue 에 제출하지 않고, device 를 poll 하지 않고, 표면을 재설정하지 않는다.** wgpu-core 29.0.4 의 획득은 대기 내내 device 의 fence 를 읽기로 쥐고(`present.rs:165-175`), `Queue::submit` 은 같은 fence 를 쓰기로 잡는다(`device/queue.rs:1202`). 그래서 이 셋 가운데 하나라도 메인에 생기면 메인이 다시 vsync 까지 막혀 이번 결정이 조용히 무너진다. 지금 유일한 제출은 이미지를 받은 뒤의 `Gpu::render` 다(`gpu/present.rs` 모듈 문서에도 적었다).
   - **전용 스레드의 실패.** 전용 스레드가 끝날 때(패닉으로 풀릴 때 포함) 이벤트 루프를 깨운다. 그러면 메인의 다음 `take`·`request` 가 "rbms-present thread stopped" 로 패닉한다. 예전에 같은 오류(wgpu 검증 오류 등)가 메인에서 바로 크래시했던 것과 같은 결과이고, 창이 조용히 멈추지 않는다.
   - **종료.** 이벤트 루프가 끝나면 메인이 전용 스레드를 멈추고 최대 1 초(wgpu 의 획득 시한 `FRAME_TIMEOUT_MS`) 기다린다(`Gpu::stop_presenting`, `Presenter::stop`). 그래서 표면과 창의 마지막 참조는 메인에서 놓인다(macOS 에서 `Window::drop` 이 다른 스레드면 메인으로 동기 디스패치한다). 시한 안에 끝나지 않으면 경고를 남기고 그대로 종료한다.
3. **키보드 레인 키에 디바운스(KEY DEBOUNCE)를 둔다. 방식은 LR2 의 `inputinterval` 이다.** 설정 `play.key_debounce_ms`(SETTINGS 의 INPUT 탭 CONTROLS 묶음, 0~100 ms, 1 ms 단위), **기본 16 ms**. LR2 의 상태 기계(배경 절)를 폴링 대신 창이 알린 변화의 시각으로 옮겼다. 상태 기계는 `apps/rbms-player/src/input_debounce.rs` 의 `Lr2Debounce`(뗀 직후의 튐 구간 `LR2_POLL_GAP` 2 ms)이고, 키보드 레인 키는 `stage/play/key_gate.rs` 의 `KeyGates` 가 그것을 쓴다(`1676c57` 에서 컨트롤러와 함께 쓰도록 옮김).
   - **누름은 들어온 순간 그대로 넘긴다.** 디바운스가 누름을 늦추는 경우는 없다.
   - **누른 뒤 창(KEY DEBOUNCE) 안에 들어온 뗌은 창 끝까지 미룬다.** 창 끝에 그때도 떼어져 있으면 창 끝의 시각으로 넘기고, 그 전에 다시 눌렸으면 버린다(누르고 있는 것으로 남는다). 그래서 누를 때 접점이 튀는 채터는 한 번의 누름이 되고, 창보다 짧은 탭의 뗌은 창 끝에 판정된다. 창이 지난 뒤의 뗌은 들어온 순간 그대로다.
   - **넘겨진 뗌 뒤 LR2 의 한 폴링 간격(`POLLGAP` 2 ms, 창이 그보다 짧으면 창) 안에 들어온 누름은 새 누름이 아니다**(LR2 의 3→2). 뗄 때 접점이 튀는 채터다. 판에는 넘기지 않고, 판에서 그 키는 뗀 채로 남는다. 다음 누름은 그 키가 다시 떼어진 뒤에 받는다.
   - 미룬 뗌은 다음 키·컨트롤러 이벤트·프레임이 먼저 확인하므로 판의 입력은 시각 순서로 들어간다(`PlayState::take_opened_keys`). 판의 레인 입력에만 건다. 메뉴·키 설정 화면·글자 입력·IME 에는 걸지 않는다. 0 ms 는 모든 변화를 그대로 넘긴다(뗀 직후의 튐 포함).
   - LR2 와 다른 점: LR2 는 누름 시각이 폴링 시각(최소 2 ms 양자화)이고 미룬 뗌도 창이 지난 첫 폴링에 넘긴다. 여기서는 창이 알린 시각과 창 끝의 시각을 그대로 쓴다. 뗀 직후의 튐을 흡수하는 구간도 LR2 에서는 실제 폴링 간격(그리는 동안은 폴링하지 않으므로 더 길 수 있다)이고, 여기서는 그 최소값 2 ms 로 고정이다.

## 이유

- 결정 1 은 조사한 모든 게임의 공통 원리(입력 수신이 화면 대기에 막히지 않음)를 그대로 따른다. 선택지 셋 가운데 가장 정확하면서 제약 안에서 되는 것이다.
  - 갱신·그리기·present 를 통째로 게임 스레드로 옮기는 안(B, LR2 스레드 옵션·Etterna 형)은 판정 시각이 같은 수준이지만, 게임 스레드가 present 를 기다리는 동안 입력 큐를 비우지 못해 키음·판정 표시가 최대 한 프레임 늦고, `SkinLibrary`(Lua, `Send` 아님)·`thread_local!` 글꼴·테마와 창 조작의 메인 왕복 때문에 지금 구조로는 옮길 수 없다(mlua `send` 기능은 의존성 변경이라 금지).
  - 단일 스레드에서 vsync 대신 OS 이벤트 대기로 프레임을 맞추는 안(C)은 wgpu 가 vblank 시각을 주지 않아 티어링이나 저더가 생기고, Metal 은 Immediate 에서도 드로어블 풀 때문에 평균 약 6 ms 막힌다(하네스 실측). present 모드만 바꾸는 것은 해결책이 아니다.
  - 조사가 권한 A 안(렌더 스레드가 표면 획득·제출·present)의 판정 시각 효과는 "획득·present 만 떼기"로 같다. 남는 오차는 메인의 프레임 CPU 시간으로 두 안이 같고, 제출까지 옮기면 오프스크린 텍스처와 전체 화면 복사 한 번이 더 든다. 그래서 대기하는 부분(획득, 그리고 DX12 Fifo 에서 막히는 present)만 뗐다.
  - 측정(같은 구조의 하네스, M5 Max 120 Hz, Metal, 6초씩): 이벤트가 들어와 받히기까지 **전 p50 4.14 ms / p95 8.01 ms → 후 p50 0.023 ms / p95 0.043 ms**. 프레임마다 CPU 일 4 ms 를 넣으면 전 p50 4.35 / p95 8.11 ms → 후 p50 0.009 / p95 3.70 ms(남는 것은 CPU 일 시간). 프레임 주기는 전후 모두 p50 8.33 ms 로 같다. 두 조건 모두 p99·최대값(수십~100 ms)은 창·표면을 만드는 시작 직후 표본이며 따로 떼지 않았다.
- 결정 3 의 방식은 결정 2 의 원문("LR2 등에서 쓰이는 방법으로 제대로 정확하게")에 따라 LR2 형으로 했다. 처음 구현은 원본(beatoraja)·컨트롤러와 같은 양방향 잠금이었는데, 리뷰에서 같은 레인 고밀도 연타의 누름이 늦게 판정되는 것이 드러났다. 예: 200 BPM 32분(37.5 ms 간격)을 30 ms 씩 누르면, 뗌(30 ms) 뒤 창(46 ms 까지) 안의 다음 누름(37.5 ms)이 46 ms 로 밀려 매 노트가 8.5 ms SLOW 가 된다. 누른 시간이 21.5 ms 를 넘으면 매번 `(누른 시간 − 21.5) ms` 만큼 늦는다. LR2 형은 누름을 늦추지 않으므로 이 지연이 없다(`input_time_tests::a_lane_hit_in_quick_succession_is_played_where_every_press_came`, `key_gate::tests::a_press_is_never_late_however_soon_it_follows_a_release` 가 고정한다).
- 결정 3 의 기본 16 ms 는 LR2 `inputinterval`(`LR2_configsave.cpp:970`), 원본 키보드(`PlayModeConfig.java:270`, 범위 0..100 `:118`), R-BMS 컨트롤러(`gamepad::DEFAULT_DEBOUNCE_MS` = `rbms_config::DEFAULT_PAD_DEBOUNCE_MS`)가 모두 16 ms 라서다. 이 값이 판정 시각을 늦추는 것은 누른 지 16 ms 안에 뗀 짧은 탭의 뗌뿐이다(창 끝으로 밀린다). 일반 롱노트·CN·HCN 의 끝은 누른 지 16 ms 보다 훨씬 뒤라 영향이 없다.

## 기각된 대안

| 대안 | 기각 이유 |
|---|---|
| OS 이벤트 시각(macOS `NSEvent.timestamp`, Windows `GetMessageTime`, Wayland input-timestamps) | winit 이 넘겨주지 않는다. 받으려면 objc2·windows-sys 직접 의존과 FFI(unsafe)가 필요하다(이번 제약 위반). Windows 는 받아도 ms 단위이고 같은 기준인 `GetTickCount` 의 분해능이 10~16 ms 다 |
| 키보드 전용 HID 스레드(IOHIDManager·Raw Input 직접 읽기, Etterna macOS 형) | 새 의존성·unsafe, macOS 입력 모니터링 권한 |
| 게임 스레드 분리(B) / 단일 스레드 + 이벤트 대기 페이싱(C) | "이유" 절 |
| 표시 모드를 Mailbox 로 바꾼다(S19 결정 2 의 선택지 B) | Metal 은 Immediate 에서도 드로어블 풀 때문에 평균 약 6 ms 막힌다(하네스 실측). present 모드만 바꾸면 메인이 여전히 획득을 기다린다(측정은 "이유" 절의 단일 스레드 안 C 에 있다) |
| 키보드는 처리 시각으로 두고 바꾸지 않는다(S19 결정 2 의 선택지 C) | 프레임 스레드가 present 를 기다린 시간(0~1 프레임)이 판정 시각에 남는다. 사용자가 "LR2 등에서 쓰이는 방법으로 제대로 정확하게"를 택함 |
| 디바운스: 양방향 잠금(마지막 변화 뒤 창 동안 누름·뗌 모두 막음, beatoraja `KeyBoardInputProcesseor.java:108`·R-BMS 컨트롤러 형) | 뗀 직후 창 안의 정상 재누름이 창 끝까지 늦게 판정된다(같은 레인 고밀도 연타에서 계통적인 SLOW, "이유" 절). LR2 는 누름을 늦추지 않는다 |
| 아날로그 턴테이블 방향도 LR2 형으로("둘다 A" 의 2번 B) | 턴테이블 알고리즘이 잠금이 통과시킨 읽기 횟수를 세므로 감도가 원본과 달라진다. 턴테이블은 튀는 접점이 아니라 이점이 없다(남은 한계 4). 사용자가 A(원본 잠금 유지)를 택함 |
| 디바운스: 누름은 즉시, 모든 뗌을 미룸(QMK `asym_eager_defer_pk` 형) | 누른 채 생기는 채터(뗌이 먼저 오는 채터)까지 막지만, 뗌을 (a) 원래 시각으로 판정하려면 그동안 다른 레인의 입력과 시간으로 진행되는 판정을 N ms 미뤄야 하고(키음·판정 표시 지연, 다른 레인 입력과의 순서), (b) 확정 시각으로 판정하면 CN·HCN 끝 판정이 N ms(기본 16 ms, PGREAT 창과 비슷한 크기) SLOW 로 밀린다. LR2 는 누른 뒤 창 안의 뗌만 미룬다 |

## 남은 한계

1. **프레임 CPU 시간.** 이벤트 루프가 한 프레임을 갱신하고 그리는 동안 들어온 키는 그 일이 끝난 순간으로 판정된다(LR2 도 그리는 동안 폴링하지 않는다). 실제 플레이어에서 프레임당 CPU 시간은 재지 않았다. 무거운 스킨·BGA·Lua 일수록 길다.
2. **뗌이 먼저 오는 채터.** 누른 지 KEY DEBOUNCE 가 지난 뒤 접점이 잠깐 떨어지는 채터는 뗌이 바로 판정되어 일반 롱노트(마진 0)가 끊긴다. 다시 붙는 누름이 2 ms 안이면 LR2 처럼 새 누름으로 받지 않는다. LR2·원본·컨트롤러 디바운스도 이 뗌은 막지 못한다(원본에서 드러나지 않았던 것은 키보드 상태가 렌더 루프 주기로 양자화되어서다, 조사의 추정). `input_time_tests::a_key_held_past_the_window_that_bounces_open_is_let_go_of_at_once` 가 이 동작을 고정한다.
3. **뗄 때 2 ms 넘게 튀는 채터.** 뗀 뒤 2 ms 가 지나 다시 붙는 튐은 새 누름으로 판정된다(빈 POOR 나 다음 노트의 이른 판정이 될 수 있다). LR2 도 폴링 간격보다 긴 튐은 같다. 양방향 잠금이면 막히지만 정상 연타의 누름을 늦추므로 택하지 않았다.
4. **(해소, 2026-10-15) 컨트롤러 디바운스도 LR2 형이다.** 사용자 결정("둘 다 추천대로")으로 컨트롤러의 버튼과 두 키 축 방향도 키보드와 같은 상태 기계를 쓴다. 상태 기계는 `apps/rbms-player/src/input_debounce.rs`(`Lr2Debounce`) 하나이고, 키보드(`stage/play/key_gate.rs`)와 컨트롤러(`gamepad.rs` 의 `PadMapper::read_keys`·`take_opened`)가 같이 쓴다. 시각만 다르다(키보드는 `Instant`, 컨트롤러는 컨트롤러 시계의 µs). 같은 버튼 200 BPM 32분 연타의 누름은 노트 위 그대로 판정된다(`input_time_tests::a_controller_button_hit_in_quick_succession_is_played_where_every_press_came`, `gamepad::tests::a_button_hit_again_and_again_is_played_where_every_press_came`). 값은 **PAD DEBOUNCE**(SETTINGS 의 INPUT 탭 CONTROLS 묶음, KEY DEBOUNCE 아래, 0~100 ms, 1 ms 단위, 기본 16 ms)로 바꾼다. 저장은 지금처럼 키 설정 파일(`keyconfig.ron`)의 `pad.debounce_ms` 이다. 설정 표(`rbms-config`)는 이 행을 프로그램이 값을 가진 행(`host_row`)으로 두고 범위만 정하며, 앱이 키 설정에서 읽고 바꾸면 곧바로 키 설정 파일을 쓴다(`settings_ui.rs` 의 `AppShared::set_pad_debounce`. 슬라이더는 끌기를 놓을 때 한 번 쓴다). **남는 차이:** 아날로그 턴테이블 방향은 원본의 양방향 잠금(`PadMapper::read_turntables`: 받아들인 변화 뒤 창 동안 읽지 않음)을 그대로 둔다. 상태는 버튼과 같은 `Lr2Debounce` 에 두므로, 알고리즘을 바꿔 축이 턴테이블과 두 키 사이를 오가도 쥐고 있던 레인을 놓는다(`gamepad::tests::a_turntable_switched_to_two_keys_lets_go_of_the_lane_it_held`). 턴테이블 알고리즘(`analog.rs`)이 잠금이 통과시킨 읽기 횟수를 세므로(멈춤 판정의 idle 카운터), 잠금을 바꾸면 턴테이블의 감도가 원본과 달라진다. 턴테이블은 접점이 튀는 스위치가 아니어서 LR2 형의 이점(누를 때 튐 흡수)이 없다. **범위:** 키보드의 KEY DEBOUNCE 는 판의 레인 입력에만 걸지만, 컨트롤러는 한 곳(`PadMapper`)에서 읽으므로 PAD DEBOUNCE 는 선곡·설정·결과 화면의 메뉴 입력에도 걸린다. 전의 양방향 잠금은 뗌 뒤 16 ms 안의 튐도 막았지만 LR2 형은 2 ms 안만 막아, 뗌 뒤 2~16 ms 튀는 버튼은 메뉴에서 두 번 눌린 것이 될 수 있다(S19 리뷰 6번, 테스트 부채 3.71). 메뉴에 옛 잠금을 둘지는 사용자 결정으로 남겼다. 설정 화면의 PAD DEBOUNCE 설명은 이 범위에 맞췄다.
5. **장치를 섞어 칠 때의 순서.** 키보드는 오는 즉시, 컨트롤러는 프레임 첫머리에서 처리된다. 곡 시계가 뒤로 가지 않으므로 이미 처리한 키보다 이른 컨트롤러 입력은 그 키의 시각으로 판정된다(테스트 부채 3.55). 어긋남은 "키보다 이른 컨트롤러 입력과 그 키 사이"로 예전("프레임 끝에 처리한 키"까지)보다 작다. 키 이벤트마다 컨트롤러를 폴링하면 풀리지만, 아날로그 턴테이블 상태 기계가 폴링 횟수를 세므로(감도가 바뀐다) 하지 않았다.
6. **확인하지 못한 플랫폼 동작.** macOS 에서 wgpu Metal 의 `get_current_texture` 가 occlusion 검사로 `NSView.window`·`NSWindow.occlusionState` 를 메인 밖에서 읽는다(wgpu-hal 29.0.4 `metal/surface.rs:121-156`, 30.0.1 에도 같은 코드). 2차 감사(THR2-1)가 Xcode Main Thread Checker 로 위반을 재현했다(호출 지점마다 한 번 보고, 코드 경로는 이미지마다 실행). 크래시는 재현되지 않았다. 알려진 위험(정의되지 않은 동작, 읽기만 함): 메인이 같은 순간 창을 바꾸는 드문 경우(전체 화면 전환, 창 닫힘)에만 의미가 있다. 사용자 결정(2026-10-15, A): 구조를 유지하고 상류 수정을 기다린다(https://github.com/gfx-rs/wgpu/issues/10627, `bug` 라벨). wgpu 를 올릴 때 이 이슈가 고쳐졌는지 확인한다. 결정과 기각된 대안은 [wgpu Metal 메인 스레드 ADR](2026-10-15-adr-wgpu-metal-main-thread.md). Windows(DX12·Vulkan)·Linux 에서 다른 스레드의 획득·present 는 wgpu 타입상 허용되지만 실기에서 돌려 보지 않았다. 크기 변경 중 메인이 획득을 기다리지 않게 바꾼 것(결정 2)은 코드로만 확인했고, DXGI 의 Present 중 메시지·Wayland 의 숨긴 창 크기 변경을 실기에서 재현하지 않았다. 60 Hz 화면, 창 크기를 끌어 바꾸는 동안의 프레임, 전체 화면 전환, 가려진 창은 실기 확인 목록(`docs/quality-assurance/2026-10-11-default-skin-manual-check.md`)에 있다.
7. **JUDGE OFFSET.** 키보드 판정 시각이 예전보다 평균 약 반 프레임, 최대 한 프레임 이르다. 예전 빌드에서 JUDGE OFFSET·자동 보정을 맞춘 키보드 사용자는 판정이 FAST 쪽으로 밀린 채 시작하므로 다시 맞춘다(릴리스 노트 "업그레이드"에 컨트롤러와 함께 안내).

## 확인받은 것

- 기본값 16 ms: 추천대로 16 ms(키보드·컨트롤러 모두).
- 컨트롤러 디바운스도 LR2 형으로 맞출지(남은 한계 4): 추천대로 LR2 형. 디바운스 값은 설정 화면에서 바꿀 수 있게 했다(PAD DEBOUNCE).
- 사용자 원문(2026-10-15): "둘 다 추천대로", "다만 debounce 값은 나중에 오버조이가면 더 바뀔수도있으니 settings에서 바꿀수있게끔넣어두면좋을듯한데".
- 아날로그 턴테이블 방향은 원본의 양방향 잠금을 유지한다(남은 한계 4 의 "남는 차이"): 사용자 원문(2026-10-15) "둘다 A" 의 2번(1번은 [BGA 상주 상한 ADR](2026-10-15-adr-bga-resident-texture-cap.md)).
- 아직 확인받지 않은 것: 메뉴 입력의 PAD DEBOUNCE 를 옛 양방향 잠금으로 둘지(남은 한계 4, 테스트 부채 3.71).

## 영향과 관련 파일

- **판정 의미.** 바뀌지 않는다. 같은 키가 더 정확한 시각(OS 가 넘긴 순간)으로 판정될 뿐이다. 리플레이 형식·규칙 세대·점수 비교는 그대로다(리플레이는 판정된 시각을 담고, 디바운스를 거친 뒤의 입력만 담는다). 키음도 키가 오는 즉시 예약된다.
- **코드.** `apps/rbms-player/src/gpu/present.rs`(`Presenter`, `Swapchain`), `apps/rbms-player/src/gpu/mod.rs`(`Target::Window` 의 `presenter`·`acquired`·`reconfigure`, `Gpu::new` 의 `wake`, `frame_ready`, `render`, `resize`, `stop_presenting`), `apps/rbms-player/src/lib.rs`(`FrameReady`, `App::frame_waker`, `frame_if_ready`, `user_event`, `ControlFlow::Wait`, `run` 끝의 `stop_presenting`), `apps/rbms-player/src/stage/play/key_gate.rs`(`KeyGates`), `apps/rbms-player/src/stage/play/mod.rs`(`lane_key`, `take_opened_keys`, `update`·`handle_pad`·`handle_key` 첫머리), `crates/rbms-config/src/schema.rs`(`PlayOptions::key_debounce_ms`, `KEY_DEBOUNCE_*`), `crates/rbms-config/src/settings.rs`(`SettingId::KeyDebounce`, `SettingId::PadDebounce`), `crates/rbms-config/src/schema.rs`(`PAD_DEBOUNCE_*`), `apps/rbms-player/src/stage/settings/catalog.rs`, `apps/rbms-player/src/input_debounce.rs`(`Lr2Debounce`, 키보드·컨트롤러 공용), `apps/rbms-player/src/gamepad.rs`(`PadMapper::read_keys`·`read_turntables`·`take_opened`), `apps/rbms-player/src/settings_ui.rs`(`set_pad_debounce`), `apps/rbms-player/src/stage/settings.rs`(`applies_when_released`).
- **테스트.** `gpu/present.rs`(화면이 이미지를 붙잡고 있어도 요청이 바로 돌아온다, 이미지는 하나씩 순서대로, 요청 중인 동안만 `waiting`, 종료 시 전용 스레드가 표면을 먼저 놓는다, 응답 없는 화면이 종료를 시한 넘게 막지 않음, 전용 스레드가 패닉하면 이벤트 루프를 깨우고 다음 요청이 패닉), `stage/play/key_gate.rs`(누름은 뗌 직후에도 늦지 않음, 창 안 뗌은 창 끝, 한 폴링 안의 재누름은 튐), `stage/play/input_time_tests.rs`(누를 때 채터에도 롱노트 유지, 막힌 뗌은 창 끝 시각으로 다음 프레임이 넘김, 0 ms 는 모두 넘김, 누른 뒤 창이 지난 채터는 끊김, 같은 레인 32분 연타의 누름은 노트 위 그대로), `stage/play/tests.rs`(창 안에서 포커스를 잃으면 합성 뗌이 창 끝에 판정), `rbms-config` 의 설정 표·범위 테스트. 컨트롤러: `gamepad/tests.rs`(같은 버튼 32분 연타의 누름은 그 자리, 누른 채 튀는 접점은 뗌만 넘기고 재누름은 튐, 누를 때 튀는 접점은 한 번의 누름), `input_time_tests::a_controller_button_hit_in_quick_succession_is_played_where_every_press_came`, `input_debounce.rs`, `settings_ui.rs`(PAD DEBOUNCE 가 키 설정 파일에 쓰이고 다음 시작에 다시 읽힌다, 범위 끝으로 잡힌다), `stage/settings/number_tests.rs`(INPUT 탭의 두 숫자 행). `Gpu::frame_ready`·`resize`·`render` 의 재설정 상태 기계는 창과 GPU 가 있어야 해서 자동 테스트가 없다(실기 확인 목록).
- **문서.** `docs/play-options.md` 3.4, `docs/architecture.md`, `docs/crates.md`, `reference-divergences.md` "입력 시각 판정", 테스트 부채 3.53·3.55.

## 날짜와 사용자 원문

2026-10-15. 사용자: "1, 3은 추천대로 / 2번은 beatmania iidx, lunatic rave 2(lr2)등에서 쓰이는방법으로 제대로 정확하게 구현해". 결정 3 의 추천안: "키보드에도 짧은 디바운스(설정 가능)로 채터를 막는다".
