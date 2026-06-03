# 2026-05-31 — M3 스크롤 + 렌더 추상화 (CPU 백엔드)

## 한 일
- `rbms-chart::scroll`(순수): mechanicsRef §5 포팅.
  - `green_number(bpm,hispeed,scroll,cover)` = 240000/bpm/hispeed/scroll·(1-cover).
  - `closed_form_offset` (변화 없는 구간 닫힌형) + `visible_offsets`(세그먼트 walk: BPM/SCROLL/STOP 적분, STOP은 노트 freeze, 가시 윈도우까지).
  - 좌표: offset = 판정선 위 픽셀 거리. 노트는 time==microtime에서 offset 0.
- `rbms-render`: `Renderer` trait(`clear`/`fill_rect`/`size`) + `CpuCanvas`(RGBA8 소프트 래스터, `pixel_at`) + `playfield`(LaneLayout, note_color, `render_playfield`). 백엔드 무관.
- example `render_frame`: 실제 차트→모델→한 프레임 PPM, sips로 PNG.

## 검증
- 단위 테스트: scroll 5(walk=closed-form, hispeed 선형, 판정선 도달, green=traversal, STOP freeze) + render 3(노트 픽셀 위치·판정선·하이스피드 픽셀 이동). **워크스페이스 전체 44 통과.**
- 시각: FELYS 7key(47.5s, hispeed 1.5) 렌더 → 8레인·흰/파랑 건반·빨강 스크래치·하단 판정선·스크롤 배치 정상. 사용자에게 PNG 전달.

## 설계
- 렌더러는 trait 뒤. CPU 백엔드 = 결정적 테스트/헤드리스 검증용 레퍼런스. **wgpu 백엔드는 같은 trait 구현(다음 단계)** — headless render-to-texture 픽셀 리드백으로 CPU 백엔드와 대조 가능.
- 좌표: 화면 top-left, 노트 낙하(판정선 하단), screen_y = judge_y - offset - NOTE_H.

## 다음
- wgpu+winit 실시간 윈도우(인스턴스드 쿼드, present-mode) → M4 autoplay(오디오 클럭 → 매 프레임 playfield + 노트 time_us 도달 시 키음 Play).
