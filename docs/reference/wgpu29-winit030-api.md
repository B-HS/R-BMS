# wgpu 29.0.3 / winit 0.30.13 API (레지스트리 소스 확인, 2026-05)

> 실시간 윈도우(rbms-player) 구현용. 추측 금지 — 아래는 실제 시그니처.

## winit 0.30
- `trait ApplicationHandler`: `fn resumed(&mut self, &ActiveEventLoop)`, `fn window_event(&mut self, &ActiveEventLoop, WindowId, WindowEvent)`.
- `EventLoop::new()? ; el.run_app(&mut app)?` (app: &mut impl ApplicationHandler).
- 윈도우 생성은 `resumed` 안에서만: `event_loop.create_window(Window::default_attributes().with_title(..).with_inner_size(..))? -> Window`. `Arc<Window>`로 감싸 wgpu surface lifetime 회피.
- `WindowEvent`: `Resized(PhysicalSize<u32>)`, `CloseRequested`, `RedrawRequested`, `KeyboardInput { event: KeyEvent, .. }`.
- `KeyEvent { physical_key: keyboard::PhysicalKey, state: ElementState, .. }`. `ElementState` in `winit::event`. `winit::keyboard::{PhysicalKey, KeyCode}` (PhysicalKey::Code(KeyCode)).
- 매 프레임 렌더: `el.set_control_flow(ControlFlow::Poll)`, `RedrawRequested`에서 그리고 `window.request_redraw()`.

## wgpu 29 (핵심: 다수 타입이 개명됨)
- `Instance::new(desc: InstanceDescriptor) -> Self` (**값으로**). `InstanceDescriptor::default()`.
- `instance.create_surface(target) -> Result<Surface, _>` (Arc<Window> 가능).
- `instance.request_adapter(&RequestAdapterOptions{ power_preference, compatible_surface: Some(&surface), force_fallback_adapter:false }) -> impl Future<Output=Result<Adapter, RequestAdapterError>>` (**async + Result**). `pollster::block_on`.
- `adapter.request_device(&DeviceDescriptor) -> impl Future<Output=Result<(Device, Queue), _>>` (**async + Result, trace 인자 없음**).
- `surface.get_capabilities(&adapter) -> SurfaceCapabilities { formats, present_modes, alpha_modes }`.
- `surface.configure(&device, &SurfaceConfiguration{ usage: TextureUsages::RENDER_ATTACHMENT, format, width, height, present_mode, alpha_mode, view_formats, desired_maximum_frame_latency })`.
- `surface.get_current_texture() -> CurrentSurfaceTexture` (**enum, Result 아님**): `Success(SurfaceTexture) | Suboptimal(SurfaceTexture) | Timeout | Occluded | Outdated | Lost | Validation`. `SurfaceTexture { texture: Texture }` + `.present()`.
- 텍스처 업로드/카피 타입은 wgpu 24+에서 **개명**: `ImageCopyTexture→TexelCopyTextureInfo`, `ImageDataLayout→TexelCopyBufferLayout` (구현 시 wgpu-types에서 정확명 재확인).

## rbms-player 계획 (텍스처 블릿 방식)
검증된 CpuCanvas playfield를 매 프레임 RGBA로 그려 wgpu 텍스처에 `write_texture` → 풀스크린 삼각형(big-triangle) WGSL 셰이더로 surface에 샘플링 표시. (네이티브 인스턴스드 쿼드는 추후 최적화.)
- 마스터 클럭 = `AudioEngine::clock_us()` (키음 로드 후 anchor 빼서 song time). `Player::update(song_us, |e| audio.play(e.wav,1.0,0.0,1.0, e.at_us + anchor))`.
- 입력: KeyCode→lane 바인딩 → `Player::press(lane, song_us, ..)`.
- 윈도우 480×640 고정(텍스처=캔버스 1:1). resize 시 surface reconfigure.
