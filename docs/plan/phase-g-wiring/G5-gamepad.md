# G5 컨트롤러 — 배선 지시 (G8 용)

> README 색인 표의 `g5.md` 행이 이 파일이다.
> 갈래 소유 파일: `apps/rbms-player/src/gamepad.rs`, `apps/rbms-player/src/gamepad/{analog,tests}.rs`,
> `apps/rbms-player/src/keyconfig.rs`, `apps/rbms-player/src/keyconfig_tests.rs`.
> 근거 스펙: `docs/plan/2026-09-09-phase-g-spec.md` §7, §11-10.

---

## 0. 이 갈래가 이미 끝낸 것 (배선 불필요)

- `PadConfig` 는 **`keyconfig.ron` 안에** 산다(`KeyConfig.pad`, 컨테이너 `#[serde(default)]`).
  → `crates/rbms-config` 의 `schema.rs`·`settings.rs` 는 **한 줄도 건드릴 필요가 없다.** 설정 탭에 PAD 행을 만들지 않는다.
- 구파일(pad 필드 없는 `keyconfig.ron`) 로드 + 범위 클램프는 `KeyConfig::load` 안에서 끝난다.
- 디바운스·2키 스크래치·아날로그 V1/V2·충돌 검출은 `PadMapper`/`AnalogScratch` 안에서 끝난다.

배선은 아래 네 가지뿐이다: **① 소유 ② 프레임 폴링 ③ 이벤트 소비(플레이) ④ 키 컨피그 PAD 열**.

---

## 1. `apps/rbms-player/src/lib.rs` — 컨트롤러를 `AppShared` 가 소유한다

### 1-1. import

**위치**: `use keyconfig::{ControlAction, KeyConfig, key_from_name, key_name};` 바로 다음 줄.

```rust
use gamepad::{PadBinding, PadEvent, PadState};
```

### 1-2. 필드

**위치**: `struct AppShared` 의 `active_reverse_keys: Vec<(KeyCode, usize)>,` 바로 다음.

```rust
    /// The controller, when this machine has one to open. `None` is the shipped state and the
    /// state of a machine with no gilrs backend: the keyboard path is untouched either way.
    pad: Option<PadState>,
```

### 1-3. 생성

**위치**: `App::new` 안, `let keyconfig = KeyConfig::load(&keyconfig_path);` 바로 다음.

```rust
        let pad = PadState::new(&keyconfig.pad);
```

**위치**: 같은 함수의 `AppShared { .. }` 리터럴에서 `active_reverse_keys: keyconfig.scratch_reverse_keys(MODE),` 다음.

```rust
                pad,
```

### 1-4. 프레임 폴링

**위치**: `App::frame`, `let transition = self.stage.update(&mut FrameCtx { shared: &mut self.shared, now, dt });` **바로 앞**.

```rust
        for event in self.shared.poll_pad() {
            let transition = self.stage.handle_pad(&mut FrameCtx { shared: &mut self.shared, now, dt: 0.0 }, event);
            self.apply(transition, event_loop);
        }
```

**근거**: §7.2. 컨트롤러 입력은 winit 이벤트가 아니라 폴링이라 프레임 루프에서만 읽힌다. `stage.update` **앞**에 두는 이유는 키보드 입력(`window_event`)이 프레임보다 먼저 도착하는 것과 순서를 맞추기 위함이다.

---

## 2. `apps/rbms-player/src/app_input.rs` — 폴링 헬퍼

**위치**: `impl AppShared` 블록 안, `pub(crate) fn control_for(&self, code: KeyCode) -> Option<ControlAction>` 바로 다음.

```rust
    /// One frame of controller input, or nothing when there is no controller.
    ///
    /// The two fields are taken apart rather than reached through `self`, because the poll needs
    /// the controller mutably and its bindings immutably at the same time.
    pub(crate) fn poll_pad(&mut self) -> Vec<PadEvent> {
        let AppShared { pad, keyconfig, mode, .. } = self;
        let Some(pad) = pad.as_mut() else {
            return Vec::new();
        };
        pad.poll_now(&keyconfig.pad, *mode)
    }
```

**근거**: §7.2. `PadState::poll_now` 는 컨트롤러 자신의 epoch 을 쓴다 — `AppShared::clock` 은 런마다 리셋되므로(`app_play.rs` `start_run`) 디바운스 시각으로 쓰면 리셋 직후 한 창(window) 동안 모든 입력이 막힌다. **`clock` 을 넘기지 말 것.**

---

## 3. `apps/rbms-player/src/stage/mod.rs` — `handle_pad` 갈래

### 3-1. 트레이트 기본 구현

**위치**: `trait StageHandler` 안, `fn handle_mouse(..)` 기본 구현 바로 다음.

```rust
    /// One controller event, already debounced. A screen with no use for a controller is unchanged.
    fn handle_pad(&mut self, ctx: &mut FrameCtx<'_>, event: crate::gamepad::PadEvent) -> Transition {
        let _ = (ctx, event);
        Transition::Stay
    }
```

### 3-2. 디스패처

**위치**: `impl Stage` 안, `pub(crate) fn handle_mouse(..)` 바로 다음.

```rust
    /// Route one controller event to the screen that is up. The option overlay does not take these:
    /// it is opened and moved with keys, and a lane held on a controller must reach the run under it.
    pub(crate) fn handle_pad(&mut self, ctx: &mut FrameCtx<'_>, event: crate::gamepad::PadEvent) -> Transition {
        self.handler().handle_pad(ctx, event)
    }
```

**근거**: §7.2. `Stage` 의 다른 입력 메서드와 동일하게 `handler()` 한 번으로 디스패치된다 — 화면 8종을 각각 손대지 않는다.

---

## 4. `apps/rbms-player/src/stage/play/mod.rs` — 레인 입력 소비

### 4-1. `lane_key` 를 (lane, dir) 진입점으로 쪼갠다

**위치**: `impl PlayState` 의 `fn lane_key(&mut self, shared: &mut AppShared, key: &KeyInput<'_>)`.
**교체**: 함수 본문을 아래 두 함수로 바꾼다(문서 주석은 기존 것을 `lane_input` 쪽으로 옮긴다).

```rust
    fn lane_key(&mut self, shared: &mut AppShared, key: &KeyInput<'_>) {
        let Some((lane, dir)) = shared.lane_input_for(key.code) else {
            return;
        };
        if key.pressed {
            self.lane_input(shared, lane, dir, true);
        } else if key.released {
            self.lane_input(shared, lane, dir, false);
        }
    }

    /// The lane press/release path: judged against the live song clock, with the keysound played on
    /// the raw input time so a judge offset never moves the sound.
    ///
    /// Without an output stream the press still goes through the session, into a sink that drops
    /// it. The session is what records the replay, so skipping the call would leave a run with
    /// releases but no presses — a replay that cannot be played back.
    fn lane_input(&mut self, shared: &mut AppShared, lane: usize, dir: ScratchDir, press: bool) {
        if shared.config.play.autoplay || shared.replay.is_some() {
            return;
        }
        let raw = shared.song_us();
        self.sync_judge_settings(shared);
        if press {
            let anchor = shared.anchor_us;
            let hit = match shared.audio.as_mut() {
                Some(audio) => self.session.press_dir(lane, dir, raw, &mut PlayAudioSink::new(audio, anchor)),
                None => self.session.press_dir(lane, dir, raw, &mut NullSink),
            };
            shared.push_timing_sample(raw, hit.map(|r| r.delta_us));
        } else {
            self.session.release_dir(lane, dir, raw);
        }
    }
```

**주의**: `autoplay`/`replay` 가드가 `lane_key` 에서 `lane_input` 으로 내려갔다. 두 진입점이 같은 가드를 공유해야 하므로 **`lane_key` 쪽에 가드를 남기지 말 것**(남기면 중복이고, 옮기지 않으면 패드 입력이 오토플레이 중에도 판정된다).

### 4-2. `handle_pad`

**위치**: `impl StageHandler for PlayState` 안, `fn handle_key(..)` 바로 다음.

```rust
    fn handle_pad(&mut self, ctx: &mut FrameCtx<'_>, event: crate::gamepad::PadEvent) -> Transition {
        match event {
            crate::gamepad::PadEvent::Lane { lane, dir, press } => self.lane_input(ctx.shared, lane, dir, press),
            crate::gamepad::PadEvent::Control(action) => self.in_play_control(ctx.shared, action),
        }
        Transition::Stay
    }
```

**근거**: §7.2. `PadEvent::Lane` 은 이미 `(lane, dir)` 이므로 `lane_input_for` 를 거치지 않는다 — 패드는 `KeyCode` 를 만들 수 없다. `PadEvent::Control` 은 **누를 때만** 발행되므로(release 는 이벤트가 없다) `key.pressed` 게이트가 필요 없다.

---

## 5. `apps/rbms-player/src/lib.rs` + `stage/keyconfig.rs` — 키 컨피그 PAD 열

### 5-1. `KcRow` 에 PAD 행 3종

**파일**: `apps/rbms-player/src/lib.rs`
**위치**: `enum KcRow` 정의.

```rust
enum KcRow {
    ModeSelect,
    Control(ControlAction),
    Lane(usize),
    /// The second key a scratch lane may be spun backwards with.
    ScratchReverse(usize),
    /// The controller device the pad rows bind against, and the turntable algorithm they read it with.
    PadDevice,
    PadAnalogMode,
    PadControl(ControlAction),
    PadLane(usize),
    PadScratchReverse(usize),
}
```

**위치**: `fn kc_rows(edit_mode: Mode) -> Vec<KcRow>`, `rows` 를 반환하기 직전.

```rust
    rows.push(KcRow::PadDevice);
    rows.push(KcRow::PadAnalogMode);
    rows.extend(ControlAction::ALL.into_iter().map(KcRow::PadControl));
    rows.extend((0..edit_mode.key).map(KcRow::PadLane));
    rows.extend((0..edit_mode.key).filter(|&lane| edit_mode.is_scratch(lane)).map(KcRow::PadScratchReverse));
```

**근거**: §7.2 "기존 `Stage::KeyConfig` 에 PAD 열을 추가한다". 별도 화면을 만들지 않는다. 키보드 행이 전부 나온 뒤에 PAD 행이 이어지므로 기존 행의 인덱스는 움직이지 않는다(`KeyConfigState::sel` 의 기존 동작 보존).

### 5-2. 캡처 경로

**파일**: `apps/rbms-player/src/stage/keyconfig.rs`

먼저 import 를 더한다(이 파일은 `use crate::*;` 를 쓰지만 `gamepad` 의 타입은 크레이트 루트에 재노출돼 있지 않다):

```rust
use crate::gamepad::{AnalogMode, PadBinding};
```

`KeyConfigState::capture(&mut self, shared, code: KeyCode)` 의 `match row` 에 새 팔을 더한다. **PAD 행은 키보드 키로 바인딩되지 않으므로 키 캡처에서는 전부 무시한다.**

```rust
                KcRow::ModeSelect
                | KcRow::PadDevice
                | KcRow::PadAnalogMode
                | KcRow::PadControl(_)
                | KcRow::PadLane(_)
                | KcRow::PadScratchReverse(_) => {}
```

`AppShared::binding_collides`(`app_input.rs`)의 `match row` 에도 같은 팔을 `=> false` 로 더한다(키보드 충돌 검사 대상이 아니다).

PAD 행이 포커스된 상태에서 ENTER 를 누르면 키가 아니라 **패드 입력**을 기다린다. `KeyConfigState::update` 에 아래를 넣는다(패드 캡처는 프레임 폴링에서만 도착한다).

```rust
    fn update(&mut self, ctx: &mut FrameCtx<'_>) -> Transition {
        if !self.capturing {
            return Transition::Stay;
        }
        let rows = kc_rows(ctx.shared.kc_edit_mode);
        let Some(row) = rows.get(self.sel).filter(|row| is_pad_row(row)) else {
            return Transition::Stay;
        };
        let AppShared { pad, keyconfig, kc_edit_mode, .. } = ctx.shared;
        let Some(state) = pad.as_mut() else {
            return Transition::Stay;
        };
        let Some(binding) = state.take_capture(&keyconfig.pad) else {
            return Transition::Stay;
        };
        let binding = match row {
            KcRow::PadLane(lane) if kc_edit_mode.is_scratch(*lane) && keyconfig.pad.analog_mode != AnalogMode::Off => {
                binding.as_analog_scratch().unwrap_or(binding)
            }
            _ => binding,
        };
        match row {
            KcRow::PadControl(action) => keyconfig.pad.set_control(*action, Some(binding)),
            KcRow::PadLane(lane) => keyconfig.pad.set_lane(*kc_edit_mode, *lane, Some(binding)),
            KcRow::PadScratchReverse(lane) => keyconfig.pad.set_scratch_reverse(*kc_edit_mode, *lane, Some(binding)),
            _ => {}
        }
        self.capturing = false;
        Transition::Stay
    }
```

`is_pad_row` 는 `stage/keyconfig.rs` 의 파일 지역 헬퍼로 둔다.

ENTER 로 캡처를 시작할 때(현재 `KeyCode::Enter` 팔) PAD 행이면 **직전 입력을 먼저 버린다**: `if let Some(state) = ctx.shared.pad.as_mut() { state.drain(&ctx.shared.keyconfig.pad); }` — 그대로 두면 화면에 들어오기 전에 눌린 버튼이 곧바로 바인딩된다. (두 필드 동시 차용이므로 여기서도 `let AppShared { pad, keyconfig, .. } = ctx.shared;` 로 쪼갠다.)

### 5-3. 표시

**위치**: `StageHandler for KeyConfigState` 의 `draw`, `let (label, raw) = match &rows[ridx]` .

```rust
                KcRow::PadDevice => ("PAD DEVICE".to_string(), ctx.shared.keyconfig.pad.device_name.clone().unwrap_or_else(|| "ANY".to_string())),
                KcRow::PadAnalogMode => ("PAD ANALOG".to_string(), ctx.shared.keyconfig.pad.analog_mode.label().to_string()),
                KcRow::PadControl(a) => (format!("PAD {}", a.label()), pad_token(ctx.shared.keyconfig.pad.control_binding(*a))),
                KcRow::PadLane(lane) => {
                    let name = if edit_mode.is_scratch(*lane) { format!("PAD SCRATCH {}", lane + 1) } else { format!("PAD LANE {}", lane + 1) };
                    (name, pad_token(ctx.shared.keyconfig.pad.lane_binding(edit_mode, *lane)))
                }
                KcRow::PadScratchReverse(lane) => (format!("PAD SCRATCH {} REVERSE", lane + 1), pad_token(ctx.shared.keyconfig.pad.scratch_reverse_binding(edit_mode, *lane))),
```

`fn pad_token(binding: Option<PadBinding>) -> String { binding.map(PadBinding::label).unwrap_or_default() }` 를 같은 파일에 둔다 — 빈 문자열이면 기존 코드가 이미 `-` 로 그린다.

중복 표시(`is_dup`)는 키보드 전용 `dups` 대신 `ctx.shared.keyconfig.pad.collisions(edit_mode)` 를 함께 본다.

`KcRow::PadAnalogMode` 는 LEFT/RIGHT 로 `AnalogMode::ALL` 을 순환시킨다(`ModeSelect` 팔과 같은 자리). `KcRow::PadDevice` 는 `PadState::device_names()` 를 순환시키고, 목록의 처음(`ANY`)은 `device_name = None` 이다.

**주의**: `analog_mode` 나 `device_name` 을 바꾼 뒤에는 `KeyConfig::save` 가 그대로 저장한다. `PadState` 는 다시 만들 필요가 없다 — `PadMapper` 가 알고리즘 변경을 스스로 감지해 진행 중이던 회전을 버린다(`PadMapper::retune`).

---

## 6. 하지 말 것

- `crates/rbms-config` 에 PAD 설정 행을 만들지 말 것. `PadConfig` 는 `keyconfig.ron` 이 정본이다.
- `AppShared::clock` 을 `PadState::poll` 에 넘기지 말 것(§2 근거).
- `PadEvent::Control` 에 press/release 게이트를 걸지 말 것. release 는 애초에 발행되지 않는다.
- `PadState::new` 가 `None` 을 돌려주는 것은 오류가 아니다(컨트롤러 없는 기계, 또는 `pad.enabled: false`). 토스트를 올리지 말 것 — 모듈이 이미 필요한 경우에만 `notify` 한다.
- 키 컨피그에 **PAD ENABLED 행을 만들지 말 것**. `enabled` 는 `PadState::new` 가 한 번만 읽는 파일 레벨 스위치라(끄면 gilrs 를 아예 열지 않는다) 화면에서 켜도 재시작 전에는 장치가 열리지 않는다.

---

## 7. 레퍼런스 이탈 (G8 이 `docs/reference-divergences.md` 에 등재)

| # | 이탈 | 내용 |
|---|---|---|
| G5-D1 | 스크래치 표현 | 레퍼런스는 스크래치 두 방향을 **서로 다른 두 레인 배정**(`AXISn_PLUS`/`AXISn_MINUS`)으로 둔다. rbms 는 한 레인 + `ScratchDir` 이므로 `PadEvent::Lane` 이 `dir` 을 함께 싣는다. 스펙 §7.2 의 `PadEvent::Lane { lane, press }` 는 `scratch_reverse`(Phase C~F 산출물) 이전에 쓰인 시그니처다 |
| G5-D2 | 아날로그 기계의 인덱스 | 레퍼런스는 `AnalogScratchAlgorithm[8]`(`AXIS_LENGTH`) 고정 배열이다. rbms 는 축 코드(`u32`)로 키잉한 맵이다 — gilrs 의 축은 플랫폼 원시 코드이지 0..8 인덱스가 아니다. 축 하나당 기계 하나라는 성질은 동일하다 |
| G5-D3 | 2키 스크래치 임계 | 레퍼런스는 0.9 고정 비교(`BMControllerInputProcessor.java:232-237`). rbms 는 `axis_deadzone`(기본 0.2, 스펙 §7.2)으로 설정화했다 — 스틱을 2키로 바인딩하면 0.9 에 닿지 않는다 |
| G5-D4 | 한 요소가 두 곳에 바인딩된 경우 | 레퍼런스는 레인 루프가 `buttonchanged` 를 먼저 소비해 start/select 가 같은 버튼이면 뒤쪽이 죽는다. rbms 는 모든 대상에 발행하고, 겹침은 `PadConfig::collisions` 로 편집기에서 표시한다 |
| G5-D5 | gilrs 필터 | `GilrsBuilder::with_default_filters(false)` — gilrs 기본 deadzone/jitter 필터는 0.009 틱과 0 부근 위치를 삼켜 아날로그 알고리즘이 볼 입력을 없앤다 |

## 8. 미구현 (스펙이 이번 Phase 밖으로 둔 것)

- **마우스 스크래치**(§7.3): 확장 지점 `ScratchSource { Keyboard, Pad, Mouse }` 만 있다.
- **MIDI**(§7.4): `trait ExternalInput { fn poll(&mut self, now_us: i64) -> Vec<PadEvent>; }` 만 있다. `PadState` 는 이 트레이트를 구현하지 않는다 — 바인딩과 모드가 필요해 시그니처가 맞지 않는다. `midir` 백엔드는 이 트레이트로 들어온다.
