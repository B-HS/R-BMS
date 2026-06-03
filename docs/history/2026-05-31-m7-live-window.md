# 2026-05-31 — M7 실시간 윈도우 (apps/rbms-player)

## 한 일
- `apps/rbms-player` (winit 0.30.13 + wgpu 29.0.3): 실제 플레이 가능 바이너리.
- 렌더: **검증된 CpuCanvas playfield → wgpu 텍스처 `write_texture` → 풀스크린 삼각형(WGSL) 블릿**으로 surface 표시. 검증 코드 재사용, 네이티브 GPU 쿼드는 추후.
- 클럭: `AudioEngine::clock_us()` − anchor(키음 로드 후 캡처) = song time. 오디오 없으면 `Instant` 폴백.
- 루프: `RedrawRequested`마다 song_us 계산 → `Player::update(song, |e| audio.play(e.wav, ..., e.at_us+anchor))` → `render_playfield` → 블릿. `ControlFlow::Poll` + `request_redraw`.
- 입력: KeyCode(S D F Space J K L, A/Shift=스크래치)→레인 → `Player::press`. `--interactive` 플래그(기본 autoplay).
- 키음 해석: `#WAV`명이 `.wav`라도 실제 `.ogg`인 BMS 관례 → 확장자 폴백(ogg/wav/flac/mp3).

## wgpu 29 API 함정(수정)
- `InstanceDescriptor` Default 없음 → 필드 명시(`display: None` 포함).
- `PipelineLayoutDescriptor`: `bind_group_layouts: &[Option<&_>]`, `push_constant_ranges`→`immediate_size`.
- `RenderPipelineDescriptor`: `multiview`→`multiview_mask`.
- `RenderPassDescriptor`: `multiview_mask` 추가, `RenderPassColorAttachment.depth_slice` 추가.
- `get_current_texture()->CurrentSurfaceTexture`(enum, `Success`/`Suboptimal`). `write_texture`는 `TexelCopyTextureInfo`/`TexelCopyBufferLayout`.
- → docs/reference/wgpu29-winit030-api.md.

## 검증 (라이브 실행)
```
loaded 163 keysounds, device 44100 Hz
playing 'FELYS 7' (812 notes) — AUTOPLAY
```
패닉 없이 윈도우 오픈 + GPU 초기화 + 163 키음 디코드/로드 + autoplay 이벤트 루프 렌더(~8s 구동). FELYS 스크롤 플레이필드 + 동기 키음.

## 결과
**첫 플레이 가능 모듈 완성.** 사용자 요구(노트 출력·하이스피드·키음·입력·판정) 충족. 워크스페이스 56 테스트.
