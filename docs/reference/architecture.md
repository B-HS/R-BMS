## rbms — Proposed Architecture & First-Milestone Plan

> 레퍼런스 구현 PLAY 코어의 from-scratch Rust 포팅. **스킨 제외**(추후 Rust-native 추상화). 렌더러는 trait 뒤에 숨겨 wgpu/macroquad 교체 가능. 시간 단위 전역 µs(`i64`). 마스터 클럭 = 오디오 디바이스 샘플(stackRef §2).

---

### 1. 워크스페이스 레이아웃 (Cargo workspace, 크레이트 = 경계)

단일 crate가 아니라 **workspace**를 권장한다 — `bms-parser`/`chart`/`judge`는 순수 로직(no_std-friendly, 테스트 빠름)이고 `audio`/`render`/`input`은 무거운 외부 의존(cpal/wgpu/winit)을 끌어오므로, 경계를 crate로 강제해 컴파일·테스트를 분리한다.

```
rbms/                              # workspace root
├── Cargo.toml                     # [workspace] members
├── crates/
│   ├── rbms-model/                # 순수 데이터 타입 (의존성 0) — Model, TimeLine, Note, Mode
│   │   └── src/lib.rs
│   ├── rbms-parser/               # BMS/bmson → rbms-model. 의존: rbms-model, md5, sha2, encoding_rs, serde_json
│   │   ├── src/bms.rs             # #xxxCC 디코더, parseInt36, RANDOM/IF, hash
│   │   ├── src/bmson.rs           # serde 구조체 + pulse→µs
│   │   └── src/lib.rs
│   ├── rbms-chart/                # 타이밍 적분(section→µs), 채널→레인, LN 페어링. 의존: rbms-model
│   │   └── src/lib.rs             # mechanicsRef §2,§3 — PlayChart(렌더/판정용 가공본) 생성
│   ├── rbms-judge/               # 판정 윈도우·매칭·게이지·콤보·스코어 (순수). 의존: rbms-model
│   │   ├── src/window.rs          # JudgeProperty 테이블 + judgerank 스케일
│   │   ├── src/matcher.rs         # JudgeAlgorithm(Combo/Duration/Lowest) + 커서 매칭
│   │   ├── src/gauge.rs           # GrooveGauge + modifier
│   │   └── src/lib.rs             # judge(note_us, press_us) 순수 함수
│   ├── rbms-audio/                # 디코드+리샘플+RT 믹서. 의존: cpal, symphonia, rubato, rtrb, basedrop
│   │   ├── src/decode.rs          # symphonia → interleaved f32
│   │   ├── src/resample.rs        # rubato, 디바이스 레이트
│   │   ├── src/mixer.rs           # voice pool, RT 콜백, SAMPLES_PLAYED
│   │   └── src/clock.rs           # play_position_us (마스터 클럭)
│   ├── rbms-render/               # Renderer trait + wgpu 구현. 의존: wgpu, winit, glyphon, image, bytemuck
│   │   ├── src/lib.rs             # trait Renderer
│   │   ├── src/wgpu_backend.rs    # 인스턴스 배치, present-mode
│   │   └── src/macroquad_backend.rs  # (feature="macroquad") 프로토타입용
│   ├── rbms-input/                # OS 타임스탬프 입력 → play-time µs 엣지. 의존: winit/gilrs
│   │   └── src/lib.rs
│   └── rbms-play/                 # 게임 루프 와이어링(스레드 3역할 + scroll 계산). 의존: 전부
│       ├── src/scroll.rs          # mechanicsRef §5 — note_screen_y, hispeed, green number
│       ├── src/loop.rs            # render/input/judge 조립
│       └── src/lib.rs
└── apps/
    └── rbms-cli/                  # 바이너리: 차트 로드→플레이. 의존: rbms-play
```

의존 방향(위→아래로만): `play → {render,input,judge,audio,chart} → parser → model`. `model`은 누구나 import, 아무도 import 안 함. 이로써 순수 코어(`model/parser/chart/judge`)를 GUI/오디오 없이 단위테스트.

---

### 2. 핵심 데이터 타입 (rbms-model)

mechanicsRef §1을 그대로. 추가로 가공본:

```rust
// rbms-model
pub struct Model { pub mode: Mode, pub wavmap: Vec<String>, pub bgamap: Vec<String>,
    pub lnobj: i32, pub lnmode: i32, pub init_bpm: f64,
    pub timelines: Vec<TimeLine>, pub md5: String, pub sha256: String }
pub struct TimeLine { pub time_us: i64, pub section: f64, pub notes: Vec<Option<Note>>,
    pub hidden: Vec<Option<Note>>, pub bgnotes: Vec<Note>, pub section_line: bool,
    pub bpm: f64, pub stop_us: i64, pub scroll: f64, pub bga: i32, pub layer: i32 }
pub enum NoteKind { Normal, Long { ln_type: LnType, is_end: bool, pair: usize }, Mine { damage: f64 } }
pub struct Note { pub kind: NoteKind, pub wav: i32, pub start_us: i64, pub duration_us: i64,
    pub time_us: i64, pub section: f64, pub layered: Vec<Note> }
pub struct Mode { pub key: usize, pub player: u8, pub scratch: &'static [usize] }

// rbms-chart 가공본(렌더/판정 hot path 최적화: 레인별 시간정렬 평탄 배열)
pub struct PlayChart {
    pub mode: Mode,
    pub lanes: Vec<Vec<PlayNote>>,        // [lane] → 시간순 노트
    pub bg_events: Vec<(i64, u32)>,       // (time_us, wav_id) autoplay
    pub segments: Vec<Segment>,           // scroll 적분용: (time_us, section, bpm, scroll, stop_us)
    pub total_notes: usize, pub main_bpm: f64, pub min_bpm: f64, pub max_bpm: f64,
}
pub struct PlayNote { pub time_us: i64, pub end_us: Option<i64>, pub wav: i32, pub kind: NoteKind, pub state: i32 }
```

**Renderer trait** (백엔드 교체점):
```rust
// rbms-render
pub trait Renderer {
    fn begin_frame(&mut self);
    fn draw_lane_bg(&mut self, lane_rect: Rect);
    fn draw_note(&mut self, lane: usize, y: f32, height: f32, kind: NoteKind);   // y = mechanicsRef §5 dsty
    fn draw_text(&mut self, s: &str, x: f32, y: f32);
    fn present(&mut self);
    fn lane_geometry(&self) -> LaneGeometry;   // hu, hl 제공 (스킨 대체)
}
pub struct LaneGeometry { pub hu: f32, pub hl: f32, pub lane_x: Vec<f32>, pub lane_w: Vec<f32> }
```
scroll 계산(`rbms-play::scroll`)은 `LaneGeometry`만 받아 백엔드 무관. wgpu/macroquad는 이 trait만 구현.

---

### 3. 첫 플레이 가능 모듈 — 마일스톤 (parse → scroll → keysound → autoplay)

목표: **차트를 읽어 hi-speed로 노트가 스크롤되고, autoplay로 키음이 정확히 울리는 최소 재생기.** 판정·게이지·입력은 다음 단계. 각 스텝마다 검증.

**M0 — model + parser (rbms-model, rbms-parser)**
- BMS 디코더: 라인 파싱, `parseInt36`(base-36 대소문자 동일), 헤더 last-wins, RANDOM/IF 단일패스, 채널 split.
- **해시**: raw 파일 바이트로 MD5(32 lowercase hex)+SHA-256(64) — 디코드 전, 바이트 그대로.
- bmson: serde_json 구조체 + `resolution`.
- *검증*: 알려진 차트의 MD5/SHA-256가 레퍼런스 구현 DB값과 일치(byte-exact). 헤더/노트 수 단위테스트. `parseInt36('A','a')`등 경계 테스트.

**M1 — 타이밍 적분 (rbms-chart)**
- section → 절대 µs (`prev.time_us + prev.stop_us + 240_000_000*(section-prev.section)/bpm`). measure-rate, BPM(03/08), STOP(09), SCROLL(SC) 병합. 채널→레인 매핑. LN 페어링(LNTYPE 1/2, LNOBJ).
- *검증*: 단순 차트(고정 BPM)의 마지막 노트 µs = 손계산. STOP 1개 → 정확히 `240_000_000*S/bpm` µs 시프트. BPM 변화 차트의 마디 경계 시각 대조. `total_notes`/`main_bpm` 검증.

**M2 — 오디오 디코드 + 리샘플 + 믹서 (rbms-audio)**
- symphonia로 모든 `#WAV` 디코드 → rubato로 디바이스 레이트 → `Arc<[f32]>` 테이블.
- cpal 출력 스트림 + RT voice pool + rtrb 명령 + `SAMPLES_PLAYED`(마스터 클럭).
- *검증*: 단일 키음을 `at_sample` 지정해 재생 → 오실로스코프/로그로 샘플 정확 트리거 확인. 동일 키음 연타 시 클릭 없음(램프). `play_position_us`가 벽시계와 ±1ms 내 동행하되 드리프트 없음(장시간).

**M3 — scroll 렌더 (rbms-render wgpu + rbms-play::scroll)**
- mechanicsRef §5 적분(`y=hl`부터 segment walk, STOP 고정/부분 분수, `rxhs=(hu-hl)*hispeed`). hi-speed·green number·lanecover·lift.
- wgpu 인스턴스 배치(노트 1 draw call), present-mode 조회(Mailbox→Fifo).
- *검증*: hispeed 2배 → 화면 노트 속도 2배·가시시간 1/2(스크린샷 비교). 노트가 `time_us` 시점에 정확히 `y=hl` 통과(오디오 클럭과 대조). STOP 구간 노트 정지. green number = `round(240000/bpm/hispeed*(1-cover))` 일치. **macroquad 백엔드로도 동일 그림** → trait 교체 검증.

**M4 — autoplay 통합 (rbms-play::loop)**
- 3역할 스레드: 오디오(클럭), 렌더(표시), 스케줄러(노트 `time_us`==`play_position_us` 도달 시 `Play` push). BGM(채널01)+노트 키음 자동 재생.
- *검증*: 곡을 끝까지 재생 — 들리는 키음과 화면 노트 판정선 통과가 동기(A/V sync). 드리프트 없음(곡 끝까지 오디오 클럭 기준 유지). 프레임률 변동(vsync on/off)이 키음 타이밍에 영향 없음.

> 이후 단계(이 마일스톤 밖): rbms-input(OS 타임스탬프 엣지) → rbms-judge(윈도우·매칭·게이지·콤보·EX-score) → 리플레이/IR. M0~M4 완료 시 "autoplay 재생기"가 동작하고, judge는 순수 함수로 격리돼 있어 입력만 붙이면 인터랙티브.

---

### 4. 설계 원칙 (포팅 시 불변)

1. **시간 = µs `i64` 전역.** f64 누적 금지(드리프트). 적분은 정수 또는 단조 보장 f64 후 즉시 i64.
2. **마스터 클럭 = 오디오 샘플** (stackRef §2). 프레임/벽시계에서 판정 시간 유도 금지.
3. **judge는 순수 함수** `judge(note_us, press_us)` — GUI/오디오 의존 0, 테이블 verbatim 포팅.
4. **렌더러는 trait 뒤에** — scroll 계산은 `LaneGeometry`만 의존, 백엔드 swap 가능.
5. **RT 콜백 무할당·무락** — 모든 통신 lock-free SPSC, 버퍼 사전할당, 해제는 basedrop.
6. **byte-exact 호환** — 해시·parseInt36·타이밍 상수(240_000_000)·판정 윈도우는 레퍼런스 구현과 1:1, 점수/리플레이 호환 위해.
