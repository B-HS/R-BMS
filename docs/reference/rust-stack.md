## rbms — Rust Stack Cheat-Sheet (2026-05 기준 검증 버전)

> macOS(CoreAudio) + Windows(WASAPI), Apple Silicon 개발기, Rust 1.95. **핵심 설계 결정: 재생 위치(=판정 시간축)는 vsync 프레임 클럭이 아니라 오디오 디바이스가 실제 재생한 샘플 수에서 유도한다.** 이것이 레퍼런스 구현 대비 가장 중요한 수정이다.

---

### 1. 크레이트 + 정확한 버전 (전부 crates.io 현행)

| 역할 | 크레이트 | 버전 | 비고 |
|---|---|---|---|
| 오디오 출력 스트림 | `cpal` | **0.17.3** | macOS CoreAudio / Win WASAPI. `SampleRate(pub u32)` 튜플 → `.0` |
| 오디오 디코드 | `symphonia` | **0.5.5** | WAV/OGG-Vorbis/FLAC/MP3 → interleaved f32. `copy_to_vec_interleaved` |
| 리샘플 | `rubato` | **3.0.0** | load 시 디바이스 레이트로 1회. planar 입출력 |
| 명령 링버퍼 (game→callback) | `rtrb` | **0.3.3** | wait-free SPSC, 고정 용량 |
| 텔레메트리 (callback→game) | `triple_buffer` | **8.x** | 구조체 스냅샷; 단순 클럭은 `AtomicU64` |
| RT-safe 지연 drop | `basedrop` | latest | 디코드 버퍼를 오디오 스레드 밖에서 해제 |
| 피치/타임스트레치(선택) | `signalsmith-stretch`/`ssstretch` | latest | 오프라인 프리렌더 권장 |
| 그래픽 (주) | `wgpu` | **29.0.3** | single-int major |
| 윈도잉 | `winit` | **0.30.13** | `ApplicationHandler`; 0.31은 beta, 회피 |
| 텍스트 | `glyphon` | **0.11.0** | wgpu^29, cosmic-text. 대안 `wgpu_text 29.0.3` |
| 이미지 | `image` | **0.25.10** | `default-features=false, features=["png"]` |
| async 블록 | `pollster` | 0.4 | `resumed()` 내 wgpu init |
| POD 캐스팅 | `bytemuck` | 1.x | vertex/instance `#[derive(Pod)]` |
| (프로토타입 대안) | `macroquad` | **0.4.15** | miniquad 백엔드(=wgpu 아님). 버림용 |

```toml
[dependencies]
cpal = "0.17"; symphonia = { version = "0.5", features = ["wav","pcm","ogg","vorbis","flac","mp3","isomp4"] }
rubato = "3"; rtrb = "0.3"; triple_buffer = "8"; basedrop = "0.1"
wgpu = "29"; winit = "0.30"; glyphon = "0.11"
image = { version = "0.25", default-features = false, features = ["png"] }
pollster = "0.4"; bytemuck = { version = "1", features = ["derive"] }
```
> `cpal` 0.16→0.17: `config.sample_rate.0` (튜플). `glyphon 0.11`·`wgpu_text 29`는 둘 다 `wgpu ^29` → `wgpu="29"` 핀하면 정렬.

---

### 2. 마스터 클럭 설계 (= 레퍼런스 구현 대비 핵심 수정)

3 역할을 깔끔히 분리하되 **클럭만 고친다**:

- **오디오 스레드 = 클럭 소스.** 콜백 끝에서 렌더한 프레임 수를 누적. `play_position_us = SAMPLES_PLAYED / sample_rate * 1e6`. 이것이 판정·BGM·화면이 공유하는 단일 시간축 → A/V/judge 드리프트 제거.
- **입력 스레드/이벤트루프 = OS 타임스탬프로 엣지를 play-time µs에 스탬프.** `is_key_pressed` 1ms 폴링 금지. winit/gilrs 디바이스 이벤트(또는 RawInput/evdev)의 하드웨어 타임스탬프 사용 → ~1ms 샘플링 지터 제거.
- **judge = 순수 함수 `judge(note_us, press_us)`** (mechanicsRef §6-7 윈도우 그대로 포팅). 엣지 도착 즉시 입력 스레드에서 실행 가능(최저 레이턴시).
- **render/프레임 클럭 = 표시 전용.** 절대 프레임 시간에서 판정 시간을 끌어오지 않는다.

```rust
use std::sync::atomic::{AtomicU64, Ordering};
static SAMPLES_PLAYED: AtomicU64 = AtomicU64::new(0);
// 콜백 끝: SAMPLES_PLAYED.fetch_add(frames as u64, Ordering::Release);
// game/judge: let t_us = SAMPLES_PLAYED.load(Ordering::Acquire) as f64 / rate as f64 * 1e6;
```
콜백은 청크 단위 업데이트 → 화면 스크롤은 `Instant`로 콜백 사이 보간(부드럽게), **판정은 raw 오디오 클럭 + 입력 타임스탬프**로만.

---

### 3. RT-safe 믹서 아키텍처

```
chart scheduler thread            audio callback (CoreAudio/WASAPI render thread)
─────────────────────             ───────────────────────────────────────────────
SAMPLES_PLAYED 읽고 song time 유도   rtrb 명령 전부 drain → voice pool 활성화
Play{sample,gain,pan,pitch,at} push  프레임마다: 활성 voice 합산(fractional stride+lerp)
                                     per-voice gain/pan → master gain → interleaved write
load thread(symphonia+rubato)        SAMPLES_PLAYED += frames (Release)
 → Arc<[f32]> 샘플 테이블 빌드
basedrop Collector::collect() (game) 퇴역 버퍼 해제
```

**콜백 절대 금지**: malloc, Mutex lock, 파일 I/O, `println!`. 전부 사전 할당. 통신은 lock-free SPSC.

```rust
struct SampleData { pcm: Arc<[f32]>, channels: u16 }   // interleaved, 디바이스 레이트
enum AudioCmd { Play { sample: u32, gain: f32, pan: f32, at_sample: u64 }, Stop { sample: u32 }, SetMasterGain(f32) }

let (mut tx, mut rx) = rtrb::RingBuffer::<AudioCmd>::new(4096).split();  // 동시타 대비 크게

struct Voice { pcm: Arc<[f32]>, src_ch: u16, pos: f64, stride: f64, gain: f32, pan: f32, active: bool }
const MAX_VOICES: usize = 512;   // BMS 동일 키음 연타로 voice 다수 중첩
```

콜백 render 루프(핵심): out을 0으로 clear → 각 active voice에 대해 `i=pos.floor()`, `frac=pos-i`, 인접 프레임 선형보간 `s = lerp*gain`, equal-power pan으로 L/R 가산, `pos += stride`(=피치), `i+1>=frames`면 `active=false`(슬롯 재사용). 마지막에 master gain·리미터. 리트리거 클릭 방지용 수 샘플 attack/release 램프 필수.

**샘플 단위 정확 트리거**: `AudioCmd::Play.at_sample`(절대 디바이스 샘플)을 실어, 콜백 시작 `SAMPLES_PLAYED` 스냅샷 기준으로 `[start, start+frames)` 안에 들면 voice 초기 `pos`를 서브버퍼 오프셋만큼 당겨 정확히 그 위치에서 시작. 128~480 프레임 블록에서도 샘플 정확.

**키음 cutoff**: mechanicsRef §9의 `id*256+pitch+128` 채널 개념 = voice 식별자. 같은 (wav,pitch) 재생 시 이전 voice를 같은 키로 stop(또는 빠른 release 램프)하여 retrigger cutoff 재현.

---

### 4. 디코드 + 리샘플 (load time, RT 밖)

각 키음 1회: symphonia probe→format reader→decoder→packet loop, `decoded.copy_to_vec_interleaved(&mut Vec<f32>)`로 코덱 무관 interleaved f32. 그 후 `rubato` `SincFixedIn`(`sinc_len=256, f_cutoff=0.95, oversampling=256, BlackmanHarris2`)으로 `device_rate/src_rate` 비율 리샘플 → **모든 키음을 디바이스 레이트로 저장** → 믹서가 프레임마다 리샘플 안 함. rubato는 planar(채널별 Vec) 입출력 → de-interleave 후 process, re-interleave. 완료 후 `Arc<[f32]>`로 동결.

**피치**: BMS는 거의 불필요. 필요 시 (a) voice의 fractional stride(`2^(semitones/12)`) + 선형보간(피치+길이 동시 변화 = 샘플러 동작, RT-safe), (b) 피치 독립이 꼭 필요하면 signalsmith로 오프라인 프리렌더. 권장 = (a).

**디바이스/버퍼**: f32, 네이티브 레이트(44.1k/48k), 스테레오. macOS `BufferSize::Fixed(256)`(≈5.3ms) 존중됨. Win **shared WASAPI는 Fixed 무시**(~10ms engine period) → `BufferSize::Default`, sub-5ms는 exclusive 모드 opt-in(시스템 믹서 점유 주의). 항상 마스터 클럭은 렌더된 샘플에서.

---

### 5. winit 0.30 + wgpu 29 스켈레톤

```rust
use std::sync::Arc;
use winit::{application::ApplicationHandler, event::WindowEvent,
            event_loop::{ActiveEventLoop, ControlFlow, EventLoop}, window::{Window, WindowId}};

struct State { window: Arc<Window>, surface: wgpu::Surface<'static>,
               device: wgpu::Device, queue: wgpu::Queue, config: wgpu::SurfaceConfiguration }
struct App { state: Option<State> }

impl ApplicationHandler for App {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        let window = Arc::new(el.create_window(Window::default_attributes()).unwrap()); // Arc → Surface 'static
        self.state = Some(pollster::block_on(State::new(window)));                      // async init 블록
    }
    fn window_event(&mut self, el: &ActiveEventLoop, _id: WindowId, ev: WindowEvent) {
        let Some(s) = self.state.as_mut() else { return };
        match ev {
            WindowEvent::CloseRequested => el.exit(),
            WindowEvent::Resized(sz) => s.resize(sz),
            WindowEvent::RedrawRequested => { s.render(); s.window.request_redraw(); }
            _ => {}
        }
    }
}
fn main() {
    let el = EventLoop::new().unwrap();
    el.set_control_flow(ControlFlow::Poll);     // 매 프레임 렌더 → Poll (Wait 아님)
    el.run_app(&mut App { state: None }).unwrap();
}
```
주의: window 생성은 `resumed()` 안에서만. `Arc<Window>`로 surface 자기참조 lifetime 회피. wgpu init은 async → `pollster::block_on`.

**Present mode(리듬게임 핵심)**: `surface.get_capabilities(&adapter).present_modes`로 지원 조회. 우선 `Mailbox`(저레이턴시·무티어링) → 없으면 `Fifo`(전 플랫폼 보장). 플레이어 토글로 `Immediate`(최저 input-to-photon, 티어링 허용) 제공. `desired_maximum_frame_latency: 1`로 큐 깊이 1프레임 절감.

**스프라이트 배치(노트 필드)**: 단일 unit-quad 정점버퍼 + per-frame instance 버퍼(`{pos, size, uv_rect, color}` `#[repr(C)] Pod`) → `queue.write_buffer` → **단일 `draw_indexed(0..6, 0, 0..N)`**. atlas 텍스처+sampler+ortho uniform 하나의 bind_group. 정점셰이더가 `@builtin(instance_index)`로 노트 필드 전체를 1 draw call로.

**텍스트**: `glyphon`(점수/콤보/판정/메뉴, 기존 render pass에 미들웨어로). 이미지: `image::load_from_memory(..).to_rgba8()` → `queue.write_texture`.

> **macroquad는 프로토타입 전용**(miniquad 백엔드=wgpu 아님, present-mode/frame-latency/instancing 제어 약함, FPS cap 없음). gameplay feel 검증 후 버리고 wgpu+winit로 이전.

---

### 6. 레퍼런스 구현에서 유지 vs 수정 요약

- **유지**: µs(`i64`) 타임스탬프 전역; per-key debounce 게이트; `setStartTime` 리베이스(입력 타임스탬프와 노트 시각 동일 축); keylog `(press_us−margin, keycode, pressed)`(리플레이); 리플레이 주입 = 키 엣지 타임스탬프 덮어쓰기.
- **수정**: ① poll-tick 타임스탬프 → OS 이벤트 타임스탬프; ② nanoTime 재생 클럭 → 오디오 재생 클럭(judge·BGM·화면 동일 디바이스 타임라인) → A/V/judge 드리프트 제거.
