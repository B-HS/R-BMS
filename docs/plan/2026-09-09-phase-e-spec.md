# Phase E 상세 설계 — 스킨 완전 커스터마이징 (E1~E6)

> 상위 계획: `docs/plan/2026-09-09-enhancement-plan.md` §2 Phase E. 결정: `docs/acknowledge/2026-09-09-enhancement-decisions.md` 결정 1 — **레퍼런스 구현 JSON 스킨 호환 + json5 관대 파서 + Lua 식 평가(mlua, 샌드박스·화이트리스트)**. LR2 CSV 는 후순위.
> 이 문서는 Phase E 구현 에이전트의 단일 사양이다. 앵커는 **현재 rbms 코드의 file:line**, 패리티 근거는 레퍼런스 구현의 **파일명:라인**으로 표기한다.
> 주의: `crates/rbms-ir`, `apps/rbms-player` 는 Phase I 워크플로가 동시 수정 중이다. 이 두 트리는 **라인 번호를 신뢰하지 말고 함수명으로 앵커**하며, 착수 시 "Phase I 이후 재확인" 항목을 먼저 검증한다.

---

## 0. 현 상태 요약 (착수 기준선)

| 항목 | 현재 | 위치 |
|---|---|---|
| Renderer trait | `size` / `clear` / `fill_rect` **3개뿐**. 텍스처·클립·회전 없음 | `crates/rbms-render/src/lib.rs:61-65` |
| CPU 백엔드 | `CpuCanvas` (RGBA8), 골든은 `block_signature(cols,rows)` + `signature_hash` (FNV-1a). **PNG 입출력 없음** | `cpu.rs:15`(struct) / `:21-77`(`impl CpuCanvas`) / `:78`(`impl Renderer for CpuCanvas`) |
| GPU 백엔드 | wgpu 인스턴스드 쿼드. `fill_rect` 1개 = 인스턴스 1개, 단일 파이프라인 | `apps/rbms-player/src/gpu.rs:63-`, `impl Renderer for Gpu` @ `gpu.rs:338` |
| BGA | **특수 경로**: 전용 파이프라인·유니폼·256x256 텍스처 (`BGA_DIM = 256` @ `gpu.rs:32`, 텍스처 생성 @ `gpu.rs:180`) | `set_bga` @ `gpu.rs:247`, `clear_bga` @ `gpu.rs:261` |
| BGA 호출부 | `apps/rbms-player/src/app_play.rs` 내 **10곳** (`set_bga` 2 / `clear_bga` 8) — 함수명 기준 재확인 필요(Phase I) | `app_play.rs` 489/533/534/559/560/618/646/693/728/754 |
| 논리 해상도 | **1280x720 확정**. `const CW: u32 = 1280;` / `const CH: u32 = 720;` (crate 루트 비공개 const, `gpu.rs:10` 이 `use crate::{CH, CW};` 로 사용). `Renderer::size()` 가 논리 크기를 반환 | `apps/rbms-player/src/main.rs:53-54` |
| 스킨 | `SkinConfig`(RON, serde `Deserialize`, `#[serde(default)]`) **pub 필드 40개**(실측) + 파생 `Skin` | `crates/rbms-render/src/skin.rs:12-64,127-` |
| 화면 렌더러 | `playfield.rs`(649) `select.rs`(538) `result.rs`(495) `hud.rs`(129) — 전부 `fill_rect` + `font::draw_text` 하드코딩. 그 외 `theme.rs`(색 팔레트, Phase E 미변경) | `crates/rbms-render/src/` |
| 폰트 | `font.rs`(700), 런/레이아웃 캐시 상한 존재(`RUN_CACHE_LIMIT`/`LAYOUT_CACHE_LIMIT`), **글리프 아틀라스 없음** | `crates/rbms-render/src/font.rs` |
| 골든 테스트 | `crates/rbms-render/tests/golden.rs` (기존 파일, 신규 아님) | `crates/rbms-render/tests/golden.rs` |

핵심 결론: 현 렌더러는 **색칠된 사각형 + CPU 래스터 텍스트**만 그릴 수 있다. 스킨 호환의 전제는 E1(텍스처 프리미티브)이며, E2~E4 는 E1 없이도 **순수 데이터 계층으로 선행 가능**하다(새 크레이트 `rbms-skin`). 이것이 병렬 분할의 근거다.

---

## 1. 신규 크레이트 배치

```
crates/rbms-skin/          (신규, no_gpu — 순수 데이터/평가 계층)
  src/lib.rs               재수출, SkinError(thiserror)
  src/timer.rs             E2: TimerId 레지스트리 + TimerState
  src/dst.rs               E2: Destination/Animation 보간기 (prepareRegion 시맨틱)
  src/property/mod.rs      E3: PropertyRegistry (Boolean/Integer/Float/String/Timer)
  src/property/generated.rs E3: SkinProperty 기계 추출 상수표 (커밋됨, 재생성 스크립트 별도)
  src/model.rs             E4: JsonSkin 미러 (serde)
  src/loader.rs            E4: 파일 해석(와일드카드/filemap/customfile/offset/property)
  src/lua.rs               E4: mlua 샌드박스 (feature = "lua", 기본 on)
crates/rbms-render/        E1: 프리미티브 + E5 화면 이식
apps/rbms-player/          E1(gpu.rs) + E5/E6 화면·UI 배선
```

`rbms-skin` 은 `rbms-render` 에 **의존하지 않는다**(역방향: `rbms-render` 가 `rbms-skin` 을 쓴다). 좌표·색은 `rbms-skin` 자체 POD 타입(`SkinRect`, `SkinColor`)으로 두고, `rbms-render` 쪽에 `From` 변환을 둔다. 이렇게 해야 E2~E4 가 E1 완료 전에 테스트까지 끝난다.

---

## 2. E1 — 렌더 프리미티브

### 2.1 trait 확장 (정확 시그니처)

`crates/rbms-render/src/lib.rs` 의 `Renderer` 를 다음으로 확장한다. **기존 3 메서드는 시그니처 불변**(하위호환), 신규는 전부 기본 구현을 주지 않는다(두 백엔드 모두 구현 강제).

```rust
/// 레퍼런스 `Skin.java:636-645`(preDraw 의 blend switch) / `:659-663`(postDraw 복원) 실측 기준.
/// 열거되지 않은 정수값(0, 1, 그리고 5~8 등 미정의 값)은 **전부 `Alpha`** 다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BlendMode {
    /// GL(SRC_ALPHA, ONE_MINUS_SRC_ALPHA). skin blend 0 / 1 / 그 외 전부.
    #[default]
    Alpha,
    /// GL(SRC_ALPHA, ONE). skin blend 2, **그리고 3**(아래 주석 참조).
    Add,
    /// GL(ZERO, SRC_COLOR). skin blend 4.
    Multiply,
    /// GL(ONE_MINUS_DST_COLOR, ZERO). skin blend 9.
    InvertDst,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureFilter { Nearest, Linear }

/// 0.0..=1.0 정규화 UV. (u0,v0)=좌상단, (u1,v1)=우하단. divx/divy 셀은 로더가 UV 로 환산한다.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UvRect { pub u0: f32, pub v0: f32, pub u1: f32, pub v1: f32 }

/// 텍스처 레지스트리 핸들. 백엔드별 실제 자원은 핸들 뒤에 숨는다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TextureId(pub u32);

#[derive(Debug, Clone, Copy)]
pub struct QuadParams {
    pub dst: Rect,
    pub src: UvRect,
    pub tint: Color,          // 곱셈 틴트. dst 의 a/r/g/b 를 그대로 넣는다.
    pub blend: BlendMode,
    pub filter: TextureFilter,
    /// 도(degree), **화면상 시계방향 양수**. 0.0 이면 회전 파이프라인을 타지 않는다.
    /// 레퍼런스 JSON `angle` 은 libGDX y-up 기준 반시계 양수이므로 로더가 **부호를 뒤집어** 넣는다(§2.1.2).
    pub angle_deg: f32,
    /// 회전 중심(픽셀, **dst 좌상단 기준 상대, y 아래로 증가**). skin `center` 0..9 는 로더가 §2.1.2 표로 환산.
    pub center: (f32, f32),
}

pub trait Renderer {
    fn size(&self) -> (u32, u32);
    fn clear(&mut self, color: Color);
    fn fill_rect(&mut self, rect: Rect, color: Color);

    // --- E1 신규 ---
    /// RGBA8 (premultiplied 아님) 픽셀을 등록하고 핸들을 돌려준다. 같은 `key` 재등록은 기존 핸들 반환.
    fn register_texture(&mut self, key: &str, rgba: &[u8], width: u32, height: u32) -> TextureId;
    /// 등록된 텍스처를 해제한다. 미등록 핸들은 무시(패닉 금지).
    fn release_texture(&mut self, tex: TextureId);
    fn texture_size(&self, tex: TextureId) -> Option<(u32, u32)>;

    fn draw_textured_quad(&mut self, tex: TextureId, params: QuadParams);

    /// 논리 좌표계 시저 사각형을 push. 중첩 시 **현재 클립과 교집합**을 적용한다.
    /// 빈 교집합이면 이후 draw 는 전부 버려진다(스택 균형은 유지).
    fn push_clip(&mut self, rect: Rect);
    /// push 와 반드시 1:1. 빈 스택 pop 은 debug_assert + 무시.
    fn pop_clip(&mut self);
}
```

#### 2.1.1 skin `blend` 정수 → `BlendMode` 환산표 (레퍼런스 실측)

| skin blend | 레퍼런스 GL 설정 (`Skin.java:636-645`) | rbms `BlendMode` |
|---|---|---|
| 0 | (설정 없음 → postDraw 기본 유지) SRC_ALPHA, ONE_MINUS_SRC_ALPHA | `Alpha` |
| 1 | 동상 | `Alpha` |
| 2 | `setBlendFunction(SRC_ALPHA, ONE)` | `Add` |
| 3 | `glBlendEquation(FUNC_SUBTRACT)` → `setBlendFunction(SRC_ALPHA, ONE)` → **즉시 `glBlendEquation(FUNC_ADD)` 로 복귀** | `Add` |
| 4 | `setBlendFunction(ZERO, SRC_COLOR)` | `Multiply` |
| 9 | `setBlendFunction(ONE_MINUS_DST_COLOR, ZERO)` | `InvertDst` |
| 그 외 | 미설정 | `Alpha` |

**blend 3 은 감산이 아니다.** 레퍼런스는 `FUNC_SUBTRACT` 를 걸었다가 draw 이전에 `FUNC_ADD` 로 되돌리므로 실제 화면 결과는 blend 2 와 동일하다(주석에도 "減算描画は難しいか？" 로 남아 있음). rbms 도 **`Add` 로 매핑**해 화면 패리티를 맞춘다. 이 사실은 `docs/acknowledge/reference-divergences.md` 에 "의도적 동형 매핑"으로 기록한다.
`postDraw`(`Skin.java:659-663`)는 `blend >= 2` 일 때만 알파로 복원하므로, 오브젝트 사이에 blend 상태가 새지 않는다 → rbms 는 **draw 호출마다 blend 를 명시**한다(상태 누수 없음).

#### 2.1.2 skin `center` 0~9 → `QuadParams.center` 환산 (레퍼런스 실측)

레퍼런스 `SkinObject.java:80-81`:
`CENTERX = {0.5, 0, 0.5, 1, 0, 0.5, 1, 0, 0.5, 1}`, `CENTERY = {0.5, 0, 0, 0, 0.5, 0.5, 0.5, 1, 1, 1}` — **정규화 비율(0..1)** 이며, `Skin.java:70-75` 의 `sprite.draw(image, x, y, cx*w, cy*h, w, h, 1, 1, angle)` 로 픽셀 오프셋이 된다. 좌표계는 **libGDX y-up(0 = 아래쪽)**. 즉 LR2 넘패드 배치(1=좌하 … 9=우상, 0=중앙)다.

rbms 는 y-down 이므로 `center_px = (cx * w, (1.0 - cy) * h)`:

| center | cx | cy(y-up) | rbms `center` (px, y-down) | 위치 |
|---|---|---|---|---|
| 0 | 0.5 | 0.5 | `(0.5w, 0.5h)` | 중앙 |
| 1 | 0 | 0 | `(0, h)` | 좌하 |
| 2 | 0.5 | 0 | `(0.5w, h)` | 중앙하 |
| 3 | 1 | 0 | `(w, h)` | 우하 |
| 4 | 0 | 0.5 | `(0, 0.5h)` | 좌중 |
| 5 | 0.5 | 0.5 | `(0.5w, 0.5h)` | 중앙 |
| 6 | 1 | 0.5 | `(w, 0.5h)` | 우중 |
| 7 | 0 | 1 | `(0, 0)` | 좌상 |
| 8 | 0.5 | 1 | `(0.5w, 0)` | 중앙상 |
| 9 | 1 | 1 | `(w, 0)` | 우상 |

범위 밖 값은 `0`(중앙)으로 클램프한다(레퍼런스는 배열 인덱스라 예외가 나므로 rbms 가 더 관대한 쪽으로 diverge — `reference-divergences.md` 기록).
**각도 부호**: libGDX `Sprite` 회전은 y-up 기준 반시계 양수 = 화면상 반시계. rbms `angle_deg` 는 화면상 시계 양수이므로 **`angle_deg = -(skin angle)`**. 단계 2 테스트에 "skin angle +90 → 좌상 앵커 기준 화면 반시계 90도" 픽셀 assert 를 반드시 포함한다(부호 역전은 조용히 틀리기 쉬운 지점).

부수 규칙:

- `fill_rect` 는 내부적으로 "1x1 흰 텍스처 + `BlendMode::Alpha`" 로 강등하지 **않는다**. 기존 인스턴스 경로를 유지해 회귀 위험을 0으로 둔다(별도 파이프라인 유지).
- BGA 특수 경로(`set_bga`/`clear_bga`/`bga_pipeline`/`BGA_DIM`)는 **삭제**하고, BGA 프레임을 매 프레임 `register_texture("__bga", ...)` 로 갱신 후 `draw_textured_quad` 로 그린다. 256x256 고정 제약이 사라진다. 호출부 치환은 `apps/rbms-player/src/app_play.rs` 10곳(§0 표) — Phase I 이후 함수명 기준 재확인.
- `wgpu::Queue::write_texture` 는 `COPY_BYTES_PER_ROW_ALIGNMENT = 256` 을 요구한다. 현재 BGA 는 256x256x4 = 1024 로 **우연히** 통과 중이다. 임의 폭 텍스처는 `bytes_per_row` 를 256 배수로 패딩한 스테이징 버퍼를 거쳐야 한다. 이 패딩 헬퍼가 E1 의 가장 흔한 실패 지점이다.

### 2.2 배치(batching) 순서 규칙 — **엄수**

스킨은 그리는 순서가 곧 z-order다. 따라서:

1. draw 호출은 **제출 순서(submission order)를 절대 재배열하지 않는다.**
2. 배치 병합은 **연속 구간(run)** 에 대해서만 한다: 직전 draw 와 `(TextureId, BlendMode, TextureFilter, 회전 유무, 현재 클립)` 이 **모두 같을 때만** 같은 draw call 에 인스턴스를 추가한다. 하나라도 다르면 **flush 후 새 배치**.
3. `push_clip`/`pop_clip`/`clear` 는 무조건 flush 경계다.
4. 회전(`angle_deg != 0`)은 인스턴스 데이터에 회전을 실어 같은 파이프라인에서 처리하되, 회전 파이프라인과 비회전 파이프라인을 분리했다면 그 전환도 flush 경계다.

즉 "텍스처별로 모아 그리기"는 **금지**다. 성능 최적화는 스킨 저작 측이 같은 텍스처를 인접 배치하도록 유도하는 것으로 얻는다(아틀라스가 이를 크게 돕는다).

> **계획 문서와의 상충 — 기록 필요**: 상위 계획 `docs/plan/2026-09-09-enhancement-plan.md` §2 E1 은 "per-texture draw call 배치 병합(순서 보존)" 이라고 적혀 있으나, 텍스처별 전역 수집은 순서 보존과 양립하지 않는다(반투명 오브젝트가 섞이면 z-order 가 깨진다). **이 사양의 "연속 run 병합만 허용" 이 정본**이며, 계획 문구는 이 절로 대체된다. 착수 전 이 결정을 `docs/acknowledge/2026-09-09-enhancement-decisions.md` 에 한 줄로 추가한다(브랜치: E-prim, 그 브랜치의 첫 커밋에 포함).

### 2.3 클립 좌표 변환

`Renderer::size()` 는 **논리 크기**(`CW`/`CH`)를 반환하는데 wgpu 시저는 **물리 서피스 픽셀**을 받는다. 따라서:

```
sx = round(rect.x * config.width  as f32 / CW as f32)
sy = round(rect.y * config.height as f32 / CH as f32)
sw = round(rect.w * config.width  as f32 / CW as f32)
sh = round(rect.h * config.height as f32 / CH as f32)
```
`set_scissor_rect` 는 서피스 밖 좌표에서 검증 오류를 낸다 → **서피스 사각형과 교집합 + `w/h == 0` 이면 draw 자체를 스킵**. CpuCanvas 는 논리=물리이므로 변환 없이 픽셀 클리핑한다.

### 2.4 CpuCanvas 참조 구현

`draw_textured_quad` 를 CPU 에서 정확히 구현한다(골든의 진실 소스).
- 샘플링: `TextureFilter::Nearest` 는 최근접, `Linear` 는 bilinear.
- 틴트: `out = src * tint / 255` 채널별.
- 블렌드(§2.1.1 의 GL factor 를 채널별로 그대로 옮긴다. `s` = 틴트 적용 후 소스 채널, `a` = 틴트 적용 후 소스 알파 0..1, `d` = 대상 채널, 전부 0..255 클램프):
  - `Alpha`  : `d' = s*a + d*(1-a)`
  - `Add`    : `d' = min(255, s*a + d)`
  - `Multiply`: `d' = d * s / 255`  (src factor ZERO 이므로 소스 항이 사라진다 — 알파를 곱하지 않는다)
  - `InvertDst`: `d' = s * (255 - d) / 255`  (dst factor ZERO)
  - **`Subtract` 는 존재하지 않는다**(§2.1.1). 감산식을 구현하면 레퍼런스와 화면이 달라진다.
- 회전: 목적 사각형의 AABB 를 순회하며 역회전 매핑으로 소스 UV 를 구한다(포워드 스캔 금지 — 구멍 생김).

### 2.5 PNG 골든 하네스

현행 `block_signature`/`signature_hash` 는 **유지**(빠른 회귀 감지)하고, 그 위에 PNG 를 얹는다.

```rust
// crates/rbms-render/src/golden.rs (신규)
pub struct GoldenOptions { pub cols: u32, pub rows: u32, pub max_channel_delta: u8, pub max_diff_ratio: f32 }
impl Default for GoldenOptions { /* cols:16, rows:9, max_channel_delta:2, max_diff_ratio:0.002 */ }

/// `tests/golden/{name}.png` 와 비교한다.
/// - 파일이 없고 `RBMS_GOLDEN_UPDATE=1` 이면 생성 후 통과.
/// - 파일이 없고 환경변수도 없으면 실패(조용한 통과 금지).
/// - 불일치 시 `target/golden-out/{name}.actual.png` 와 `{name}.diff.png` 를 쓰고 실패.
pub fn assert_golden_png(canvas: &CpuCanvas, name: &str, opts: GoldenOptions) -> Result<(), String>;
```
- PNG 인코딩/디코딩은 `png` 크레이트(순수 Rust, 무의존 확장 없음)를 dev-dependency 로 추가. 런타임 의존이 아니다.
- **폰트 결정성**: 기존 CI 골든 실패 원인(임베드 폰트 폴백)을 반복하지 않도록, PNG 골든 테스트도 `TextEngine::embedded_only` / `use_embedded_fonts_only` 경로를 강제한다(PROCESS "CI(dev) 골든 실패 수정" 항목 참조).
- 텍스처가 필요한 골든은 **코드 생성 텍스처**(체커보드·그라디언트 8x8/16x16)를 쓴다. 바이너리 에셋을 새로 커밋하지 않는다.

### 2.6 글리프 아틀라스 (K7)

`font.rs` 의 래스터 결과를 `TextureId` 1장(2048x2048, shelf packing)에 적재하고 텍스트를 텍스처 쿼드 배치로 전환한다. **E1 의 마지막 단계**이며, 아틀라스 없이도 E5 는 진행 가능(기존 CPU 텍스트 경로 유지). 아틀라스 도입 시 골든 해시는 전부 재생성 대상이다 → 별도 커밋으로 분리한다.

---

## 3. E2 — 타이머 레지스트리와 dst 보간기

### 3.1 TIMER 레지스트리 (레퍼런스 `SkinProperty.java` 의 `TIMER_*` 상수)

> **이 표는 전수가 아니라 "직접 확인한 대표 앵커"다.** `PM_CHARA` 는 900/901 이후로도 계속되고, `HOLD_*`/`HCN_*`/`KEYON_*`/`KEYOFF_*` 계열은 `71..`, `251..` 처럼 생략돼 있다. **TIMER 도 §4.1 기계 추출 대상**이며, `timer_id` 모듈은 손번역이 아니라 생성 산출물에서 재수출한다. 아래 표는 생성 결과를 사람이 검증하기 위한 스팟체크 목록으로만 쓴다.

| ID | 이름 | 의미 |
|---|---|---|
| 1 | STARTINPUT | 입력 수용 시작 |
| 2 | FADEOUT | 화면 페이드아웃 시작 |
| 3 | FAILED | 실패 연출 시작 |
| 10 | SONGBAR_MOVE | 곡 바 이동 |
| 11 | SONGBAR_CHANGE | 곡 바 변경 |
| 12 / 13 | SONGBAR_MOVE_UP / _DOWN | 위/아래 이동 |
| 14 | SONGBAR_STOP | 바 정지 |
| 15 / 16 | README_BEGIN / _END | README 표시 시작/종료 |
| 21~26 | PANEL1_ON ~ PANEL6_ON | 패널 열림 |
| 31~36 | PANEL1_OFF ~ PANEL6_OFF | 패널 닫힘 |
| 40 | READY | READY 연출 |
| 41 | PLAY | 플레이 시작(모든 노트 타이밍의 기준) |
| 42 / 43 | GAUGE_INCLEASE_1P / _2P | 게이지 증가 |
| 44 / 45 | GAUGE_MAX_1P / _2P | 게이지 MAX |
| 46 / 47 / 247 | JUDGE_1P / _2P / _3P | 판정 표시 갱신 |
| 446 / 447 / 448 | COMBO_1P / _2P / _3P | 콤보 표시 갱신 |
| 48 / 49 | FULLCOMBO_1P / _2P | 풀콤보 연출 |
| 348 / 349 / 350 | SCORE_A / _AA / _AAA | 랭크 도달 |
| 351 / 352 | SCORE_BEST / _TARGET | 베스트/타깃 상회 |
| 50~59 | BOMB_1P_SCRATCH, BOMB_1P_KEY1..KEY9 | 1P 키 봄 |
| 60~69 | BOMB_2P_SCRATCH, BOMB_2P_KEY1..KEY9 | 2P 키 봄 |
| 70 / 71.. | HOLD_1P_SCRATCH / HOLD_1P_KEY1.. | 1P LN 홀드 중 |
| 80 / 81.. | HOLD_2P_SCRATCH / HOLD_2P_KEY1.. | 2P LN 홀드 중 |
| 250 / 251.. | HCN_ACTIVE_1P_SCRATCH / _KEY1.. | HCN 활성 |
| 270 / 271.. | HCN_DAMAGE_1P_SCRATCH / _KEY1.. | HCN 데미지 |
| 100~109 | KEYON_1P_SCRATCH, KEYON_1P_KEY1..KEY9 | 1P 키 눌림 |
| 110~119 | KEYON_2P_* | 2P 키 눌림 |
| 120~129 | KEYOFF_1P_SCRATCH, KEYOFF_1P_KEY1..KEY9 | 1P 키 뗌 |
| 130~139 | KEYOFF_2P_* | 2P 키 뗌 |
| 140 | RHYTHM | BPM 동기 리듬 타이머 |
| 143 / 144 | ENDOFNOTE_1P / _2P | 마지막 노트 통과 |
| 150 / 151 | RESULTGRAPH_BEGIN / _END | 리절트 그래프 애니 |
| 152 | RESULT_UPDATESCORE | 스코어 갱신 연출 |
| 172 / 173 / 174 | IR_CONNECT_BEGIN / _SUCCESS / _FAIL | IR 통신 상태 (Phase I 의 `HttpScoreServer` 상태와 연결 — Phase I 이후 재확인) |
| 900 / 901 / 902 / 903 … | PM_CHARA_1P_NEUTRAL / _FEVER / _GREAT / _GOOD … (이후 계속) | 캐릭터 연출 — **기계 추출로 전수 확보** |

추가로 `customTimers`(스킨 정의 타이머, `JsonSkin.CustomTimer`)를 **ID 공간 상단(예: 100000+)** 에 동적 할당한다.

```rust
// crates/rbms-skin/src/timer.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TimerId(pub i32);

pub mod timer_id { pub const PLAY: super::TimerId = super::TimerId(41); /* ... 위 표 전수 ... */ }

/// 타이머는 "켜진 시각(ms)" 또는 OFF 이다. 레퍼런스의 `Long.MIN_VALUE` 센티널을 `None` 으로 표현한다.
#[derive(Debug, Default, Clone)]
pub struct TimerState { on: std::collections::HashMap<TimerId, i64> }

impl TimerState {
    pub fn set_on(&mut self, id: TimerId, now_ms: i64);
    pub fn set_off(&mut self, id: TimerId);
    pub fn is_off(&self, id: TimerId) -> bool;
    /// 켜진 시각(ms). OFF 면 None.
    pub fn get(&self, id: TimerId) -> Option<i64>;
    /// 경과 ms. OFF 면 None.
    pub fn elapsed(&self, id: TimerId, now_ms: i64) -> Option<i64>;
}
```

### 3.2 dst 보간기 — `SkinObject.prepareRegion` 시맨틱 (레퍼런스 `SkinObject.java:348-433`, `getRate` @ `:545-575`). **그리기 여부 판정은 §3.3 이 담당한다 — 이 함수는 타이머·키프레임만 본다.**

정확한 알고리즘(**순서와 분기를 그대로 옮긴다**):

1. `clip = false`.
2. 타이머가 지정되어 있으면: `timer.isOff(state)` → **`draw = false` 후 즉시 반환**. 아니면 `time -= timer.get(state)`.
3. `lasttime = endtime`(= 마지막 키프레임 time).
4. **loop == -1 경로**: `if time > endtime { time = -1 }`. (뒤의 `starttime > time` 검사에서 대부분 `draw=false` 로 떨어진다 — 즉 "1회 재생 후 소멸".)
5. **loop >= 0 경로**: `if lasttime > 0 && time > dstloop` → `if lasttime == dstloop { time = dstloop } else { time = (time - dstloop) % (lasttime - dstloop) + dstloop }`.
6. `if starttime > time { draw = false; return }`.
7. `nowtime = time`, `rate = -1`, `index = -1`, 각 offset 값 조회.
8. 고정 사각형(`fixr`, 키프레임 1개뿐이라 보간 불필요)이면 그 값 사용, 아니면 `getRate()` 후 보간.
9. **offset 적용**: `relative == false` 일 때만 `x += off.x - off.w/2; y += off.y - off.h/2`. `w += off.w; h += off.h` 는 relative 무관하게 항상.

`getRate()`:

```
if rate != -1 { return }                       // 캐시
time2 = dst[last].time
if nowtime == time2 { rate = 0; index = last; return }
for i in (0..=last-1).rev() {
    time1 = dst[i].time
    if time1 <= nowtime && time2 > nowtime {
        r = (nowtime - time1) as f32 / (time2 - time1) as f32
        r = match acc { 1 => r*r, 2 => 1.0 - (r-1.0)*(r-1.0), _ => r }
        rate = r; index = i; return
    }
    time2 = time1
}
rate = 0; index = 0
```

**acc 종류**:

| acc | 이름 | 식 |
|---|---|---|
| 0 | Linear | `r` |
| 1 | Accelerate (ease-in) | `r * r` |
| 2 | Decelerate (ease-out) | `1 - (r-1)^2` |
| 3 | Step (보간 없음) | region/clip 모두 `dst[index]` 값을 그대로 사용. **`getRate` 는 여전히 acc 0 처럼 rate 를 계산하지만 region 계산에서 rate 를 무시한다** (`SkinObject.java:388-393`, clip 은 `:445`) |

`rate == 0` 이거나 `acc == 3` 이면 `dst[index]` 를 그대로, 그 외에는 `dst[index]` ↔ `dst[index+1]` 을 `rate` 로 선형 보간(x/y/w/h 각각). 클립도 동일 규칙이되 `dst[index].clip` 이 `None` 이면 `clip = false`, `dst[index+1].clip` 이 `None` 이면 `clip1` 을 그대로 쓴다(`SkinObject.java:440-457`).
색(a/r/g/b)과 각도(angle)는 `prepareColor`/`prepareAngle` 이 같은 index/rate 로 보간한다 — 동일 보간기를 재사용한다.
클립 최종 유효성: `clip = clipRegion.width > 0 && clipRegion.height > 0` (`SkinObject.java:472`).

```rust
// crates/rbms-skin/src/dst.rs
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct SkinRect { pub x: f32, pub y: f32, pub w: f32, pub h: f32 }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SkinColor { pub r: u8, pub g: u8, pub b: u8, pub a: u8 }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Acc { Linear, Accelerate, Decelerate, Step }
impl Acc { pub fn from_id(v: i32) -> Acc; pub fn apply(self, r: f32) -> f32; }

#[derive(Debug, Clone, Copy)]
pub struct Keyframe {
    pub time_ms: i64,
    pub rect: SkinRect,
    pub clip: Option<SkinRect>,
    pub acc: Acc,
    pub color: SkinColor,
    pub angle_deg: f32,
}

#[derive(Debug, Clone)]
pub struct DestinationTrack {
    pub timer: Option<TimerId>,
    /// -1 = 1회 재생 후 소멸, >=0 = 그 시각부터 마지막 키프레임까지 반복
    pub loop_ms: i64,
    pub blend: i32,
    pub filter: i32,
    pub center: i32,
    pub offsets: Vec<i32>,
    /// JSON 필드가 아니다. play 화면 로더가 판정 카운트 오브젝트에만 true 로 세팅한다(§5.7).
    pub relative: bool,
    pub frames: Vec<Keyframe>,   // time 오름차순 정렬 보장 (로더 책임)
    /// §3.3. 정수 op 와 Lua draw 식이 합류한 그리기 조건. 비어 있으면 항상 그린다.
    pub draw_conditions: Vec<DrawCondition>,
    /// §3.3. `region` 상대 좌표. 마우스가 밖이면 그리지 않는다.
    pub mouse_rect: Option<MouseRect>,
    /// §5.6. JSON `stretch`(-1 = 미지정 → STRETCH).
    pub stretch: i32,
}

#[derive(Debug, Clone, Copy)]
pub struct Resolved { pub rect: SkinRect, pub clip: Option<SkinRect>, pub color: SkinColor, pub angle_deg: f32 }

/// `None` = 이 프레임에 그리지 않음(draw = false).
pub fn resolve(track: &DestinationTrack, now_ms: i64, timers: &TimerState, offsets: &dyn OffsetSource) -> Option<Resolved>;
```

### 3.3 op / draw 게이팅 — 그리기 여부 판정 (레퍼런스 `SkinObject.prepare` @ `SkinObject.java:590-610`)

`resolve()`(§3.2)는 **타이머·키프레임만** 다룬다. 오브젝트를 그릴지 말지는 그 **앞뒤**에서 별도로 판정한다. 레퍼런스 `prepare(time, state, offsetX, offsetY)` 의 실제 순서:

1. `dstdraw`(BooleanProperty 배열)를 순회해 **하나라도 false 면 `draw = false` 후 즉시 반환**(prepareRegion 조차 호출하지 않음).
2. `prepareRegion` (= §3.2 `resolve`).
3. `region.x += offsetX; region.y += offsetY`, `prepareClip(offsetX, offsetY)`.
4. `mouseRect` 가 있으면 마우스 좌표가 `region` 상대 사각형 안인지 검사 → 밖이면 `draw = false` 후 반환.
5. `prepareColor()`, `prepareAngle()`.

`op`(정수 배열)과 `draw`(BooleanProperty)의 관계: 로더의 `setDrawCondition(int[] dstop)`(`SkinObject.java:282-298`)가 정수 op 배열을 받아 **BooleanProperty 로 변환해 `dstdraw` 에 합류**시킨다. 즉 런타임에는 `dstdraw` 하나만 평가하면 된다. 음수 op ID 는 §4.3 규약대로 `abs(id)` 조회 후 부정이다. op 값 `0` 은 "조건 없음"이므로 변환 단계에서 **버린다**.

rbms API(`crates/rbms-skin/src/dst.rs`, **E-timer 소유**):

```rust
/// 로더가 op 정수 배열과 Lua BooleanProperty 를 합쳐 만든 그리기 조건.
#[derive(Debug, Clone)]
pub enum DrawCondition {
    /// 정수 프로퍼티 ID. 음수면 부정.
    Option(i32),
    /// Lua 식 핸들(§5.3). 평가 실패 시 false + 1회 warn.
    Lua(LuaExprId),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MouseRect { pub x: f32, pub y: f32, pub w: f32, pub h: f32 }

/// `prepare` 전체. `None` = 이 프레임에 그리지 않음.
/// 평가 순서는 위 1~5 를 그대로 따른다.
pub fn prepare(
    track: &DestinationTrack,
    now_ms: i64,
    timers: &TimerState,
    state: &dyn SkinStateSource,
    lua: Option<&LuaSandbox>,
    offset_xy: (f32, f32),
    mouse: Option<(f32, f32)>,
) -> Option<Resolved>;
```

관련 필드(`draw_conditions`, `mouse_rect`)는 §3.2 의 `DestinationTrack` 정의에 이미 포함돼 있다.

**책임 경계**: `DrawCondition` 열거·`prepare` 는 **E-timer** 가 소유하고(`dst.rs`), `LuaExprId` 는 E-timer 가 정의하는 **불투명 `u32` 뉴타입**이다. E-load 가 Lua 식을 컴파일해 이 ID 를 채우고, `LuaSandbox` 가 ID→함수 를 해석한다. 이렇게 해야 E-timer 가 E-load 완료 전에 (Lua 없이, `lua: None`) 단독 테스트를 마칠 수 있다. `lua: None` 인데 `DrawCondition::Lua` 를 만나면 **false 취급 + 1회 warn**.

**테스트(E2-2)**: 조건 전부 true → `Some`, 하나 false → `None`(그리고 `resolve` 가 호출되지 않았음을 스파이로 확인), 음수 op 부정, op `0` 무시, `mouseRect` 안/밖 2케이스, `lua: None` + `DrawCondition::Lua` → `None`.

**테스트(E2-1)**: acc 4종 × (loop=-1 / loop=0 / loop=중간값 / lasttime==loop) 매트릭스, 타이머 OFF → `None`, `starttime > time` → `None`, 키프레임 1개(fixr) 경로, `nowtime == last.time` 경계, offset relative on/off, 클립 `None` 전파 3케이스. 각 케이스에 기대값을 손으로 계산해 상수로 박는다.

---

## 4. E3 — 프로퍼티 레지스트리

정수 ID 하나가 곧 게임 상태 조회다. 레지스트리는 5종:

| 종류 | 레퍼런스 팩토리 | 규모 | 반환 |
|---|---|---|---|
| Boolean(OPTION) | `BooleanPropertyFactory.java` (758줄) | 수백 | `bool` (음수 ID = 논리 부정) |
| Integer(NUMBER) | `IntegerPropertyFactory.java` (1509줄) | 최대 | `i32` |
| Float(SLIDER/BARGRAPH) | `FloatPropertyFactory.java` (596줄) | 수십~백 | `f32` 0.0..=1.0 |
| String(TEXT) | `StringPropertyFactory.java` (421줄) | 수십 | `String` |
| Timer | `TimerPropertyFactory.java` (42줄) | §3.1 표 | `Option<i64>` |

상수 정의는 전부 `SkinProperty.java` 한 파일 — **`public static final int` 실측 968개**(`grep -c`). **손으로 옮기지 않는다.**

### 4.1 상수표 생성 절차 (E3-1, 필수 선행)

1. `SkinProperty.java` 의 `public static final int <PREFIX>_<NAME> = <n>;` 를 정규식으로 전수 추출(줄 단위, 주석/한자 주석 무시).
2. `crates/rbms-skin/src/property/generated.rs` 로 출력 — `pub const OPTION_<NAME>: i32 = n;` 형태 + `pub const ALL_OPTION: &[(i32, &str)] = &[...]` 인덱스 배열.
3. 생성 스크립트는 `tools/gen-skin-property.rs` (또는 `xtask`)에 두고 **생성 결과를 커밋**한다. build.rs 로 레퍼런스 트리를 읽지 않는다(레퍼런스는 이 레포에 없다).
4. 산출물 검증: 추출 개수를 스크립트가 출력하고, 테스트가 `ALL_OPTION.len()` 등 하한을 assert 한다(무언의 0건 방지). **총 `public static final int` 개수는 968 이며, 접두사별 합계가 이 값과 일치하는지도 assert 한다**(누락 즉시 검출).
5. **TIMER 도 이 추출에 포함**한다. `timer_id` 모듈(§3.1)은 생성 상수의 얇은 재수출이어야 하며, §3.1 표의 ID 는 "생성 결과에 존재하는가"를 확인하는 스팟체크 테스트로만 쓴다.

### 4.2 알려진 ID 대역 (앵커, `SkinProperty.java`)

| 대역 | prefix | 예 |
|---|---|---|
| 1~20 | SLIDER_* | 1 MUSICSELECT_POSITION, 6 MUSIC_PROGRESS, 7 SKINSELECT_POSITION, 17 MASTER_VOLUME, 18 KEY_VOLUME, 19 BGM_VOLUME, 20 PRACTICE_POSITION |
| 101~ | BARGRAPH_* | 101 MUSIC_PROGRESS, 102 LOAD_PROGRESS, 103 LEVEL, 105~109 LEVEL_BEGINNER..INSANE, 110 SCORERATE, 111 SCORERATE_FINAL, 112/113 BESTSCORERATE_NOW/BESTSCORERATE, 114/115 TARGETSCORERATE_NOW/TARGETSCORERATE, 140~143 RATE_PGREAT/GREAT/GOOD/BAD |
| §3.1 표 | TIMER_* | 1~3, 10~16, 21~36, 40~49, 50~69, 70~81, 100~144, 150~152, 172~174, 247/250/270, 348~352, 446~448, 900~ (표는 스팟체크용, **확정은 §4.1 기계 추출**) |
| 대규모 | OPTION_* / NUMBER_* / STRING_* | E3-1 기계 추출로 확정 |

### 4.3 rbms 상태 원천 매핑

```rust
// crates/rbms-skin/src/property/mod.rs
/// 스킨이 조회하는 게임 상태의 유일한 창구. Phase C 의 PlaySession 등을 이 trait 뒤에 숨긴다.
pub trait SkinStateSource {
    fn boolean(&self, id: i32) -> bool;      // id < 0 이면 abs(id) 조회 후 부정
    fn integer(&self, id: i32) -> i32;
    fn float(&self, id: i32) -> f32;         // 0.0..=1.0 로 클램프
    fn string(&self, id: i32) -> &str;
    fn timer(&self, id: i32) -> Option<i64>;
    fn offset(&self, id: i32) -> Option<SkinOffset>;
}
```

구현부(`rbms-render` 또는 `apps/rbms-player`)에서 실제 상태를 꼬리표별로 잇는다:

| 프로퍼티군 | rbms 상태 원천 | 비고 |
|---|---|---|
| 플레이 판정/콤보/스코어/게이지(NUMBER, OPTION, BARGRAPH 110~143) | **Phase C 의 `PlaySession`** (`crates/rbms-play`) | **C1 선행 필수.** C 완료 전에는 테스트 더블(`FakeState`)로만 검증 |
| 판정 카운트·FAST/SLOW | `rbms_judge` 카운터 | Phase D 로 확장되는 판정 종류와 동기 |
| 노트/LN/HCN 상태 타이머(50~81, 250~271) | `PlaySession` 의 레인별 입력·LN 상태 | D(J24 CN/HCN) 이후 HCN 계열 실값 |
| 곡 선택 리스트/폴더/정렬(select 계열 OPTION/STRING/NUMBER) | `apps/rbms-player/src/{app_select,folders,tables}.rs` | Phase I 이후 함수명 재확인 |
| 리절트/베스트/타깃(348~352, 112~115, TIMER 150~152) | `apps/rbms-player/src/scores.rs`, `rbms-render/src/result.rs` | |
| IR(172~174 및 IR 관련 OPTION/STRING) | **Phase I 의 `rbms-ir` `HttpScoreServer` 상태** | Phase I 이후 재확인. 미로그인 시 전부 false/빈 문자열 |
| 볼륨 슬라이더(17~19) | Phase B 의 볼륨 3분리 | B 선행 시 실값, 미완이면 마스터만 |
| 미구현 ID | `false` / `0` / `0.0` / `""` / `None` | **패닉 금지.** 미구현 ID 는 debug 빌드에서 1회만 `tracing::debug!` 로 기록 |

**테스트(E3)**: 생성 상수표 개수 하한, 음수 ID 부정 규칙, 미구현 ID 기본값, Float 클램프, 대표 20개 ID 에 대한 `FakeState` 왕복.

---

## 5. E4 — 스킨 모델·로더

### 5.1 serde 모델 (`crates/rbms-skin/src/model.rs`)

`JsonSkin.java:6-529` 를 1:1 미러한다. 최상위 `Skin`:

```rust
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields = false)]
pub struct SkinDef {
    pub r#type: i32,          // -1
    pub name: String, pub author: String,
    pub w: i32,               // 1280
    pub h: i32,               // 720
    pub fadeout: i32, pub input: i32, pub scene: i32, pub close: i32,
    pub loadend: i32, pub playstart: i32,
    pub judgetimer: i32,      // 1
    pub finishmargin: i32,
    pub category: Vec<Category>, pub property: Vec<PropertyDef>,
    pub filepath: Vec<Filepath>, pub offset: Vec<OffsetDef>,
    pub source: Vec<Source>, pub font: Vec<FontDef>,
    pub image: Vec<ImageDef>, pub imageset: Vec<ImageSet>,
    pub value: Vec<ValueDef>, pub floatvalue: Vec<FloatValueDef>,
    pub text: Vec<TextDef>, pub slider: Vec<SliderDef>,
    pub graph: Vec<GraphDef>, pub gaugegraph: Vec<GaugeGraph>,
    pub judgegraph: Vec<JudgeGraph>, pub bpmgraph: Vec<BpmGraph>,
    pub hiterrorvisualizer: Vec<HitErrorVisualizer>,
    pub timingvisualizer: Vec<TimingVisualizer>,
    pub timingdistributiongraph: Vec<TimingDistributionGraph>,
    pub note: Option<NoteSet>, pub gauge: Option<GaugeDef>,
    #[serde(rename = "hiddenCover")] pub hidden_cover: Vec<HiddenCover>,
    #[serde(rename = "liftCover")] pub lift_cover: Vec<LiftCover>,
    pub bga: Option<BgaDef>, pub skinpreview: Option<SkinPreview>,
    pub practice: Option<Practice>, pub judge: Vec<JudgeDef>,
    pub songlist: Option<SongList>, pub pmchara: Vec<PmChara>,
    #[serde(rename = "skinSelect")] pub skin_select: Option<SkinConfigurationProperty>,
    #[serde(rename = "customEvents")] pub custom_events: Vec<CustomEvent>,
    #[serde(rename = "customTimers")] pub custom_timers: Vec<CustomTimer>,
    pub destination: Vec<Destination>,
}
```

`Destination`(`JsonSkin.java:453-481`) / `Animation`(`:507-529`) 은 특히 정확히:

```rust
#[derive(Debug, Clone, Deserialize)] #[serde(default)]
pub struct Destination {
    pub id: String, pub blend: i32, pub filter: i32,
    pub timer: i32,               // TimerProperty (정수 ID 또는 Lua 식)
    pub loop_: i32,               // #[serde(rename = "loop")]
    pub center: i32, pub offset: i32, pub offsets: Vec<i32>,
    pub stretch: i32,             // -1
    pub op: Vec<DestinationOption>,   // 정수 ID 또는 BooleanProperty(Lua)
    pub draw: Option<BooleanPropertyRef>,
    pub dst: Vec<Animation>,
    #[serde(rename = "mouseRect")] pub mouse_rect: Option<RectDef>,
}

/// 미지정 필드는 `Integer.MIN_VALUE` 센티널 → 직전 키프레임 값을 상속한다.
/// serde 에서는 `Option<i32>` 로 받고 로더가 앞 프레임에서 채운다.
#[derive(Debug, Clone, Deserialize)] #[serde(default)]
pub struct Animation {
    pub time: Option<i64>,
    pub x: Option<i32>, pub y: Option<i32>, pub w: Option<i32>, pub h: Option<i32>,
    pub clip_x: Option<i32>, pub clip_y: Option<i32>, pub clip_w: Option<i32>, pub clip_h: Option<i32>,
    pub acc: Option<i32>,
    pub a: Option<i32>, pub r: Option<i32>, pub g: Option<i32>, pub b: Option<i32>,
    pub angle: Option<i32>,
}
```

**상속 규칙(중요)**: `Integer.MIN_VALUE` = "미지정". 첫 키프레임의 미지정은 타입 기본값(x/y/w/h=0, a=255, r/g/b=255, acc=0, angle=0)으로, 두 번째 이후는 **직전 키프레임 값**으로 채운다. 이 채움을 로더에서 끝내면 `dst.rs` 는 완전한 키프레임만 본다.

기타 오브젝트: `Image`(`JsonSkin.java:112-127`: src/x/y/w/h/divx/divy/timer/cycle/len/ref/act/click), `ImageSet`(`:129-136`), `Value`(`:138-154`: align/digit/padding/zeropadding/space/ref/value/offset), `FloatValue`(`:156-175`: fketa/iketa/gain/isSignvisible), `Text`(`:177-200`: font/size/align/ref/value/event/constantText/editable/wrapping/overflow/outlineColor/outlineWidth/shadow*), `Slider`(`:203-223`: angle/range/type/changeable/value/event/isRefNum/min/max), `Graph`(`:225-242`), `NoteSet`(`:339-361`: note/lnstart/lnend/lnbody/lnbodyActive/lnactive/hcnstart/hcnend/hcnbody/hcnactive/hcnbodyActive/hcndamage/hcnbodyMiss/hcnreactive/hcnbodyReactive/mine/hidden/processed/dst/dst2/expansionrate), `Gauge`(`:369-378`: nodes/parts=50/type/range=3/cycle=33/starttime=0/endtime=500), `HiddenCover`/`LiftCover`(`:380-`), `BGA`(`:410-412`).

### 5.2 json5 파서 선정

- 크레이트: **`json5`**(순수 Rust, serde `Deserializer` 제공). 주석·트레일링 콤마·비따옴표 키·홑따옴표 문자열을 허용해 실제 배포 스킨의 관대함을 흡수한다.
- 전략: **먼저 `serde_json` 으로 시도 → 실패 시 `json5` 로 재시도.** 대다수 스킨은 정상 JSON 이라 빠른 경로가 살고, 깨진 스킨만 느린 경로를 탄다. 두 경로 모두 실패하면 라인/컬럼을 포함한 `SkinError::Parse`.
- **`json5` 는 f64 로 숫자를 다루므로** 큰 정수 정밀도에 주의(스킨 값은 전부 i32 범위라 실무상 안전 — 그래도 `i64` 초과 시 에러).

### 5.3 Lua 식 평가 (mlua 샌드박스)

`timer` / `op` / `draw` / `value` 필드는 정수 ID 대신 **Lua 식 문자열**이 올 수 있다(레퍼런스의 `.luaskin` 및 JSON 내 프로퍼티 식).

```rust
// crates/rbms-skin/src/lua.rs
pub struct LuaSandbox { lua: mlua::Lua, root: std::path::PathBuf, budget: Budget }

#[derive(Debug, Clone, Copy)]
pub struct Budget { pub max_instructions: u32, pub max_memory_bytes: usize, pub max_calls_per_frame: u32 }
impl Default for Budget { /* 200_000 / 8 MiB / 4_096 */ }

impl LuaSandbox {
    pub fn new(root: &std::path::Path, budget: Budget) -> Result<Self, SkinError>;
    pub fn eval_bool(&self, src: &str, state: &dyn SkinStateSource) -> Result<bool, SkinError>;
    pub fn eval_int(&self, src: &str, state: &dyn SkinStateSource) -> Result<i32, SkinError>;
    pub fn eval_float(&self, src: &str, state: &dyn SkinStateSource) -> Result<f32, SkinError>;
    pub fn eval_string(&self, src: &str, state: &dyn SkinStateSource) -> Result<String, SkinError>;
}
```

**샌드박스 규칙(엄수)**:

| 항목 | 정책 |
|---|---|
| 표준 라이브러리 | `mlua::StdLib::MATH \| STRING \| TABLE` **만** 로드. `io`/`os`/`package`/`debug`/`ffi` 전면 배제 |
| 제거할 전역 | `dofile`, `loadfile`, `load`, `loadstring`, `require`, `collectgarbage`, `rawset`(전역 오염 방지), `setmetatable`(선택적 유지) |
| 노출 API 화이트리스트 | `skin.boolean(id)`, `skin.number(id)`, `skin.float(id)`, `skin.text(id)`, `skin.timer(id)`, `skin.time()` — 이상 **6개뿐**. 전부 `SkinStateSource` 위임 |
| 파일 접근 | 없음. Lua 는 파일을 열 수 없다. 스킨 루트(`root`)는 로더가 Rust 측에서만 사용 |
| 메모리 | `Lua::set_memory_limit(budget.max_memory_bytes)` |
| 실행 시간 | `Lua::set_hook`(instruction count)로 예산 초과 시 중단 → `SkinError::LuaBudget` |
| 실패 처리 | 식 평가 실패는 **스킨 로드 실패가 아니다.** 해당 오브젝트만 그리지 않고 1회 warn. 프레임 루프에서 반복 로깅 금지 |
| 결정성 | `math.random` 은 로드 시 고정 시드로 재정의하거나 제거(골든 테스트 결정성) |

성능: 프레임마다 문자열을 파싱하지 않는다. **로드 시 `Lua::load(src).into_function()` 으로 컴파일해 `RegistryKey` 로 보관**하고 프레임에서는 호출만 한다.

### 5.4 파일 해석 규칙 (레퍼런스 `SkinLoader.getPath(imagepath, filemap)` 그대로 — 현 트리 기준 `SkinLoader.java:95-129`, 라인은 버전 차가 있으니 **함수명으로 앵커**)

`resolve_path(pattern, filemap) -> PathBuf` 순서:

1. **filemap 접두사 치환**: `filemap` 의 어떤 key 가 `pattern` 의 접두사이면 → `pattern[..pattern.rfind('*')] + filemap[key] + pattern[key.len()..]` 로 치환하고 **와일드카드 단계는 건너뛴다**(레퍼런스가 `imagepath = ""` 로 만들어 다음 분기를 무력화).
2. **와일드카드**: 남은 문자열에 `*` 가 있으면 확장자 `ext = pattern[rfind('*')+1..]`. `|` 가 있으면 `ext = pattern[rfind('*')+1 .. find('|')] + pattern[rfind('|')+1..]`(단, `|` 가 마지막 문자면 뒤 절만 생략).
3. `pattern[..rfind('/')]` 디렉터리를 열어 **소문자 비교로 `ext` 로 끝나는 파일**을 모으고, 그중 **무작위 1개**를 고른다.
4. 후보가 0개면 원래 경로를 그대로 반환(존재하지 않을 수 있음 → 로드 시 경고 후 스킵).

**rbms 추가 규칙(레퍼런스에 없음, 보안)**: 해석 결과 경로를 `canonicalize` 하여 **스킨 루트 밖으로 나가면 거부**(`..` traversal, 심볼릭 링크 탈출). 이 divergence 는 `docs/acknowledge/reference-divergences.md` 에 기록한다.

**무작위 결정성**: 골든/테스트에서는 `RBMS_SKIN_SEED` 환경변수 또는 `SkinLoadOptions::rng_seed: Option<u64>` 로 고정한다. 기본은 스킨 로드 1회당 1번 뽑아 캐시(같은 패턴은 같은 파일).

#### 5.4.1 customfile — filemap 의 **생성** 경로 (계획 §2 E4 항목)

§5.4 는 filemap 을 *소비*하는 쪽이다. filemap 이 어디서 오는지가 customfile 이다. 레퍼런스 실측 경로:

1. **후보 선언**: 스킨 JSON 최상위 `filepath[]`(§5.1 `SkinDef::filepath`)의 각 항목 `{ name, path, def }` 가 그대로 헤더의 CustomFile 이 된다. `path` 는 **스킨 파일이 있는 디렉터리 기준으로 앞에 부모 경로가 붙은** 절대 패턴이다(`JSONSkinLoader.java:145-157`: `new CustomFile(pr.name, p.getParent() + "/" + pr.path, pr.def)`).
2. **후보 열거 + 선택 확정**: `SkinHeader.java:174-203` 이 사용자 설정(`SkinConfig.FilePath{name, path}`)과 이름으로 매칭한다.
   - 사용자 값이 `"Random"` **이 아니면** 그 값을 그대로 `filename` 으로 확정.
   - 사용자 값이 `"Random"` 이면 §5.4 2~3 단계와 **동일한 ext 파싱(`*`/`|`)** 으로 디렉터리를 스캔해 후보를 모으고 **무작위 1개의 파일명**(경로가 아니라 `getName()`)을 확정.
3. **filemap 생성**: `JSONSkinLoader.java:223-225` 가 확정된 항목만 `filemap.put(customFile.path, customFile.getSelectedFilename())` 로 넣는다. **key 는 패턴 문자열 전체**(`.../gauge/*.png` 같은), value 는 파일명. §5.4 1단계의 "key 가 pattern 의 접두사" 판정이 이 형태를 전제한다.

rbms 매핑:

```rust
/// 스킨 헤더에서 뽑은 사용자 선택 대상 파일 슬롯. §7 E6 UI 의 "Filepath 행" 데이터 원천.
#[derive(Debug, Clone)]
pub struct CustomFile {
    pub name: String,
    /// 스킨 루트 기준으로 정규화된 와일드카드 패턴 (= filemap 의 key).
    pub pattern: String,
    /// JSON `def`. 사용자 설정이 없을 때의 초기값.
    pub default: Option<String>,
    /// 패턴을 확장한 후보 파일명 목록. UI 순환 선택의 항목이며 맨 앞에 `"Random"` 을 넣는다.
    pub candidates: Vec<String>,
}

/// 스킨 파일을 파싱한 직후, 텍스처 로드 이전에 호출한다.
pub fn enumerate_custom_files(def: &SkinDef, root: &Path) -> Vec<CustomFile>;
/// 사용자 설정 + 후보 목록으로 filemap 을 만든다. `"Random"` 은 `rng_seed` 로 결정적으로 뽑는다.
pub fn build_filemap(files: &[CustomFile], user: &SkinUserConfig, rng_seed: Option<u64>) -> BTreeMap<String, String>;
```

- `SkinUserConfig::filepaths`(§5.5)는 **`CustomFile.name` → 선택 파일명(또는 `"Random"`)** 을 저장한다(경로 전체가 아니다).
- 후보 열거 결과도 `canonicalize` 로 스킨 루트 밖을 거부한다(§5.4 보안 규칙 동일 적용).
- `enumerate_custom_files` 는 **로드 시 1회**만 디렉터리를 스캔하고 결과를 `LoadedSkin` 에 보관한다. E6 UI 가 매 프레임 디렉터리를 읽지 않게 하기 위함이다.

**테스트(E4-2)**: `filepath[]` → CustomFile 변환(부모 경로 결합), `"Random"` 시드 재현성, 사용자 지정값 우선, 후보 0개일 때 `def` 폴백, 생성된 filemap key 가 §5.4 접두사 치환과 실제로 맞물리는 왕복 테스트 1건.

### 5.5 사용자 설정 영속화

`property`(선택지) / `filepath`(파일 선택) / `offset`(위치 미세조정) 은 스킨별 사용자 설정이다.

```rust
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SkinUserConfig {
    pub path: String,                              // 스킨 정의 파일 경로(식별자)
    pub properties: std::collections::BTreeMap<String, i32>,   // Property.name -> 선택 op
    pub filepaths: std::collections::BTreeMap<String, String>, // Filepath.name -> 선택 경로(= filemap)
    pub offsets: std::collections::BTreeMap<i32, SkinOffset>,  // Offset.id -> x/y/w/h/r/a
}
```
저장 위치는 기존 설정 파일 체계를 따른다(`apps/rbms-player/src/settings.rs` — Phase I 이후 함수명 재확인). 원자적 저장(임시 파일 → rename)은 Phase A 에서 확립된 방식을 그대로 쓴다.

`SkinLoadOptions` 로 로더에 주입:

```rust
pub struct SkinLoadOptions<'a> {
    pub root: &'a std::path::Path,
    pub user: &'a SkinUserConfig,
    pub rng_seed: Option<u64>,
    pub lua_budget: Budget,
    pub mode: rbms_model::Mode,
}
pub fn load_skin(path: &std::path::Path, opts: SkinLoadOptions<'_>) -> Result<LoadedSkin, SkinError>;
```

**테스트(E4-1)**: json5 폴백(주석·트레일링 콤마·홑따옴표 픽스처), Animation 센티널 상속, filemap 치환 4케이스, 와일드카드 `|` 확장자 2케이스, traversal 거부, Lua 화이트리스트(금지 전역이 `nil` 인지 6종 assert), Lua 예산 초과 중단, 결정 시드 재현성.

### 5.6 stretch / dstfilter — 확정 규칙 (레퍼런스 실측)

**stretch (`StretchType.java` enum, 값 0~10)** — `SkinObject.draw` 가 `stretch.stretchRect(tmpRect, tmpImage, image)` 로 **dst 사각형과 소스 리전을 동시에 조정**한다(`SkinObject.java:628-631`). JSON `Destination.stretch` 기본값은 `-1`(= 미지정 → `STRETCH`).

| 값 | 이름 | 동작 |
|---|---|---|
| -1 / 0 | STRETCH | dst 에 그대로 늘린다(기본) |
| 1 | KEEP_ASPECT_RATIO_FIT_INNER | 비율 유지, dst 안쪽에 맞춤(letterbox). 작은 축 스케일 채택 |
| 2 | KEEP_ASPECT_RATIO_FIT_OUTER | 비율 유지, dst 전체를 덮음(넘침). 큰 축 스케일 채택 |
| 3 | KEEP_ASPECT_RATIO_FIT_OUTER_TRIMMED | 2 와 같되 넘치는 부분을 **소스 리전에서 잘라낸다**(dst 크기 불변) |
| 4 / 5 | KEEP_ASPECT_RATIO_FIT_WIDTH / _TRIMMED | dst 폭 기준, 높이를 비율로. TRIMMED 는 소스에서 자름 |
| 6 / 7 | KEEP_ASPECT_RATIO_FIT_HEIGHT / _TRIMMED | dst 높이 기준, 폭을 비율로 |
| 8 | KEEP_ASPECT_RATIO_NO_EXPANDING | 비율 유지 + **확대 금지**(축소만) |
| 9 / 10 | NO_RESIZE / NO_RESIZE_TRIMMED | 원본 픽셀 크기 그대로. TRIMMED 는 dst 밖을 소스에서 자름 |

`fit*` 계열은 dst 사각형을 **중심 유지로 축소/확대**한다(`fitWidth`/`fitHeight` 헬퍼). `*_TRIMMED` 계열은 dst 를 건드리지 않고 `trimmedImage` 의 UV 를 좁힌다 → rbms 에서는 `QuadParams.src`(UvRect) 조정으로 표현된다. **E5 에서 최소 0/1/2/9 를 먼저 구현**하고 나머지는 미지원 시 `STRETCH` 폴백 + 1회 warn(스킨이 통째로 깨지지 않게).

**dstfilter → `TextureFilter`** — `SkinObject.java:634-636` 실측:

```
setType(dstfilter != 0 && imageType == TYPE_NORMAL
        ? (dstRect.w == srcRegion.w && dstRect.h == srcRegion.h ? TYPE_NORMAL : TYPE_BILINEAR)
        : imageType)
```
`Skin.java:518-523` 의 타입 상수: `TYPE_NORMAL=0`, `TYPE_LINEAR=1`, `TYPE_BILINEAR=2`, `TYPE_FFMPEG=3`, `TYPE_DISTANCE_FIELD=5`. 그리고 `Skin.java:82-86` 의 `setFilter` 는 **`TYPE_LINEAR`/`TYPE_FFMPEG`/`TYPE_DISTANCE_FIELD` 일 때만 하드웨어 `TextureFilter.Linear`** 를 건다 — `TYPE_BILINEAR` 는 하드웨어 필터가 아니라 **전용 셰이더**다.

rbms 확정 규칙(divergence 포함):

| 조건 | rbms `TextureFilter` |
|---|---|
| `dstfilter == 0` | `Nearest` |
| `dstfilter != 0` 이고 dst 크기 == 소스 리전 크기(1:1) | `Nearest` |
| `dstfilter != 0` 이고 크기가 다름 | `Linear` |

즉 레퍼런스의 셰이더 바이리니어를 **하드웨어 Linear 로 근사**한다. 셰이더 재현은 하지 않는다 — `docs/acknowledge/reference-divergences.md` 에 "TYPE_BILINEAR 셰이더 → 하드웨어 Linear 근사"로 기록한다. `TYPE_FFMPEG`/`TYPE_DISTANCE_FIELD` 는 Phase E 범위 밖(동영상·SDF 폰트)이며 만나면 `Linear` + 1회 warn.

### 5.7 `relative` 플래그의 출처 (확정)

JSON 필드가 아니다. **로더가 특정 오브젝트에 코드로 켠다.** 레퍼런스 전체에서 `setRelative(true)` 호출은 두 군데뿐이다: `JsonPlaySkinObjectLoader.java:260`(플레이 화면의 판정 카운트 `numbers[i]`)과 `LR2PlaySkinLoader.java:778`(LR2 CSV 의 같은 대상). 따라서 rbms 는 `Destination` 모델에 `relative` 를 두지 않고, **play 화면 로더가 판정 카운트 오브젝트를 만들 때만 `DestinationTrack.relative = true` 로 세팅**한다. §3.2-9 의 offset 분기는 그대로 유효하다.

---

## 6. E5 — 화면 이식 순서와 SkinConfig 호환

이식 순서(레퍼런스 화면 수 = 스킨 타입): **play → select → result → decide → keyconfig**. 각 화면마다 다음 4단계를 반복한다.

1. 해당 화면의 `SkinStateSource` 구현(§4.3 매핑)을 채운다.
2. 해당 화면의 오브젝트 렌더러(Image/Value/Text/Slider/Graph/Note/Judge/Gauge/BGA)를 `draw_textured_quad` 로 구현한다.
3. **기존 하드코딩 렌더러를 삭제하지 않는다.** 스킨이 없거나 로드 실패면 기존 경로로 폴백한다.
4. 골든 추가: 스킨 로드 성공 경로 1장 + 폴백 경로 1장.

**SkinConfig(RON) 의 위치 — 결정**:

- `crates/rbms-render/src/skin.rs` 의 `SkinConfig`/`Skin` 은 **삭제하지 않는다.** "기본(내장) 스킨의 파라미터"로 남긴다.
- JSON 스킨이 선택되면 JSON 이 전권을 갖고 `SkinConfig` 는 참조되지 않는다. JSON 스킨이 없으면 현재와 100% 동일 동작.
- 즉 `SkinConfig` 는 **JSON 스킨의 하위 집합을 표현하는 별개 축**이며, 둘을 병합하지 않는다(병합은 우선순위 지옥을 만든다). 이는 계획 §2 의 "M8 제약 하 호환 유지"의 구체안이다.
- 레인 좌표(`field_x`/`field_width`/`judge_y`/`note_height`)는 JSON 스킨의 `NoteSet.dst`/`Destination` 이 대체한다.

**Phase C 의존**: play 화면의 프로퍼티 대부분이 `PlaySession` 을 원천으로 한다 → **E5 의 play 단계는 C1 완료 후 착수**. select/result 는 C1 없이도 진행 가능.

---

## 7. E6 — 스킨 선택·커스터마이즈 UI

기존 설정 화면(탭 구조)에 **SKIN 탭**을 추가한다. 행 구성:

| 행 | 종류 | 동작 |
|---|---|---|
| 화면 타입 선택 | 순환 선택 | §7.1 의 rbms 지원 타입만 |
| 스킨 목록 | 순환 선택 | 스킨 디렉터리를 스캔해 `name (author)` 표시. `(기본 내장)` 항목 항상 첫째 |
| 미리보기 | 정보 행 | 해상도 `w x h`, 파서(json/json5), Lua 사용 여부, 로드 경고 수 |
| Property 행 (스킨의 `property[]` 개수만큼 동적) | 순환 선택 | `Category > name` 라벨, `item[]` 의 이름 순환 → `SkinUserConfig.properties` |
| Filepath 행 (`filepath[]` 만큼) | 순환 선택 | 후보 파일 목록(와일드카드 확장 결과) 순환 → `filepaths`(= filemap) |
| Offset 행 (`offset[]` 만큼) | 수치 조절 | 정의가 허용한 축(x/y/w/h/r/a)만 좌우키로 증감, `0` 으로 리셋 |
| RELOAD | 액션 | 현재 설정으로 스킨 재로드(앱 재시작 없이) |
| RESET | 액션 | 해당 스킨의 사용자 설정 전부 삭제 |

#### 7.1 스킨 타입 목록 (레퍼런스 `SkinType.java:12-30` 실측)

스킨은 **타입별로 따로 선택**되므로 이 목록이 곧 SKIN 탭의 첫 행 항목이다. 레퍼런스 전체 열거:

| id | 이름 | 대응 Mode | Phase E 범위 |
|---|---|---|---|
| 0 | PLAY_7KEYS | BEAT_7K | O |
| 1 | PLAY_5KEYS | BEAT_5K | O |
| 2 | PLAY_14KEYS | BEAT_14K | O |
| 3 | PLAY_10KEYS | BEAT_10K | O |
| 4 | PLAY_9KEYS | POPN_9K | O |
| 5 | MUSIC_SELECT | - | O |
| 6 | DECIDE | - | O |
| 7 | RESULT | - | O |
| 8 | KEY_CONFIG | - | O |
| 9 | SKIN_SELECT | - | X (Phase E 의 SKIN 탭 자체가 이 화면을 대체) |
| 10 | SOUND_SET | - | X |
| 11 | THEME | - | X |
| 12 / 13 / 14 | PLAY_7KEYS_BATTLE / _5KEYS_BATTLE / _9KEYS_BATTLE | 동상 + battle | X (rbms 에 배틀 모드 없음) |
| 15 | COURSE_RESULT | - | X (코스 미구현) |
| 16 / 17 | PLAY_24KEYS / _24KEYS_DOUBLE | KEYBOARD_24K / _DOUBLE | rbms 가 해당 Mode 를 지원할 때만 |
| 18 | PLAY_24KEYS_BATTLE | - | X |

- **범위 밖 타입도 열거는 한다**(스킨 폴더에 존재할 수 있으므로) — 단 선택하면 "미지원" 경고 행만 띄우고 기본 스킨을 쓴다. 목록에서 감추지 않는 이유는, 사용자가 왜 자기 스킨이 안 보이는지 알 수 있게 하기 위해서다.
- 지원 타입 집합은 `rbms_model::Mode` 에 실제로 존재하는 값으로 **코드에서 유도**한다(손으로 5종을 박지 않는다).
- `SkinDef::type`(§5.1)의 기본값 `-1` 은 "타입 미지정"이며, 로더는 이때 파일 경로/헤더로 추론하지 않고 **로드 거부 + 경고**한다.

- 조작 규약은 기존 설정 화면과 동일(상하 이동, 좌우 값 변경, Esc 복귀 — Phase A 의 U3/U4 결정 준수).
- 로드 실패 시 **모달이 아니라 행 옆 경고 텍스트 + 기본 스킨 폴백**. 게임 진입을 막지 않는다.
- 스킨 재로드는 반드시 이전 스킨의 `TextureId` 를 `release_texture` 로 해제한다(누수 관점 P1 과 동일 기준).

---

## 8. 병렬 분할 (파일 소유권 — 웨이브별 교집합 공집합)

**규칙**: 파일 소유권은 **웨이브 단위**로 판정한다. 같은 웨이브에서 동시에 도는 브랜치끼리 파일이 겹치면 안 되고, 이 사양이 건드리는 **모든 파일에 소유자가 정확히 하나** 있어야 한다. 앞 웨이브가 머지된 뒤 다음 웨이브가 같은 파일을 이어받는 **순차 인계는 허용**한다(동시 편집이 아니므로).

### 웨이브 0 — E-scaffold (단독 머지, 병렬 없음)

크레이트가 존재하지 않으면 웨이브 1 의 E-timer 가 빌드조차 못 한다. 그래서 **스캐폴드를 먼저 단독으로 머지**한다. 이 브랜치는 **선언·설정만** 만들고 로직은 쓰지 않는다.

| 소유 파일 | 내용 |
|---|---|
| `Cargo.toml` (루트) | `members` 에 `"crates/rbms-skin"` 추가. **Phase E 전체에서 이 파일의 유일한 소유자** |
| `crates/rbms-skin/Cargo.toml` (신규) | 패키지 선언 + `serde`/`thiserror`. `json5`·`mlua` 는 optional feature 로 자리만 선언(§11-12 확인 후 버전 확정) |
| `crates/rbms-skin/src/lib.rs` (신규) | `pub mod timer; pub mod dst; pub mod property; pub mod model; pub mod loader; pub mod lua; pub mod resolve;` **전부 선언** + `SkinError`(thiserror). **Phase E 전체에서 이 파일의 유일한 소유자** |
| `crates/rbms-skin/src/{timer,dst,model,loader,lua,resolve}.rs`, `src/property/{mod,generated}.rs` (신규) | **빈 스텁**(파일만 생성). 이후 웨이브가 내용을 채운다 |
| `crates/rbms-render/Cargo.toml` | `png` **dev-dependency** 추가(E-prim 용) + `rbms-skin` dependency 추가(E-screen 용). **한 번에 둘 다** |
| `apps/rbms-player/Cargo.toml` | `rbms-skin` dependency 추가(E-ui 용) |
| `docs/acknowledge/reference-divergences.md` | 이 사양이 확정한 divergence 를 **선반영**: ① 배칭 정책(§2.2) ② blend 3 → Add 동형 매핑(§2.1.1) ③ center 범위 밖 클램프(§2.1.2) ④ TYPE_BILINEAR 셰이더 → 하드웨어 Linear 근사(§5.6) ⑤ 경로 traversal 거부(§5.4) |
| `docs/acknowledge/2026-09-09-enhancement-decisions.md` | 계획 §2 E1 의 "per-texture 배칭" 문구를 §2.2 로 대체한다는 결정 1줄 |

완료 판정: `cargo build --workspace` 통과(빈 크레이트가 workspace 에 들어감), `cargo test --workspace` 기준선 유지.

### 웨이브 1 (동시): E-prim ∥ E-timer

| 브랜치 | 담당 | **배타 소유 파일 (전수)** | 선행 |
|---|---|---|---|
| **E-prim** | E1 프리미티브·골든·아틀라스 | `crates/rbms-render/src/lib.rs`<br>`crates/rbms-render/src/cpu.rs`<br>`crates/rbms-render/src/golden.rs` (신규)<br>`crates/rbms-render/src/font.rs`<br>`crates/rbms-render/tests/golden.rs`<br>`crates/rbms-render/tests/primitives.rs` (신규)<br>`apps/rbms-player/src/gpu.rs` | 웨이브 0 |
| **E-timer** | E2 타이머·보간기·게이팅 | `crates/rbms-skin/src/timer.rs`<br>`crates/rbms-skin/src/dst.rs` | 웨이브 0 |

**소유권 검증(웨이브 1)**: `{rbms-render/src/lib.rs, cpu.rs, golden.rs, font.rs, tests/golden.rs, tests/primitives.rs, rbms-player/src/gpu.rs} ∩ {rbms-skin/src/timer.rs, rbms-skin/src/dst.rs} = ∅`. 두 브랜치는 서로 다른 크레이트만 만진다. `crates/rbms-skin/src/lib.rs` 는 웨이브 0 이 이미 완성했으므로 **E-timer 는 mod 선언을 추가하지 않는다**.

### 웨이브 2 (동시): E-prop ∥ E-load

| 브랜치 | 담당 | **배타 소유 파일 (전수)** | 선행 |
|---|---|---|---|
| **E-prop** | E3 프로퍼티 레지스트리 | `crates/rbms-skin/src/property/mod.rs`<br>`crates/rbms-skin/src/property/generated.rs`<br>`tools/gen-skin-property.rs` (신규) | E-timer(`TimerId` 타입) |
| **E-load** | E4 모델·로더·Lua·customfile | `crates/rbms-skin/src/model.rs`<br>`crates/rbms-skin/src/loader.rs`<br>`crates/rbms-skin/src/lua.rs`<br>`crates/rbms-skin/src/resolve.rs`<br>`crates/rbms-skin/tests/**` (신규) | E-timer(`DestinationTrack`·`LuaExprId`), E-prop(`SkinStateSource`) |

**소유권 검증(웨이브 2)**: `{property/mod.rs, property/generated.rs, tools/gen-skin-property.rs} ∩ {model.rs, loader.rs, lua.rs, resolve.rs, rbms-skin/tests/**} = ∅`.
E-load 가 `SkinStateSource` trait(§4.3, E-prop 소유 파일에 정의) 를 필요로 하므로 **E-prop 의 trait 정의 커밋을 먼저 머지**한 뒤 E-load 가 리베이스한다(타입만 선행, 구현은 병렬).

### 웨이브 3 (동시): E-screen ∥ E-ui

| 브랜치 | 담당 | **배타 소유 파일 (전수)** | 선행 |
|---|---|---|---|
| **E-screen** | E5 화면 이식 + BGA 호출부 정리 | `crates/rbms-render/src/skin.rs`<br>`crates/rbms-render/src/playfield.rs`<br>`crates/rbms-render/src/select.rs`<br>`crates/rbms-render/src/result.rs`<br>`crates/rbms-render/src/hud.rs`<br>`crates/rbms-render/src/skin_render.rs` (신규, 스킨 오브젝트 렌더러 + `SkinRect`/`SkinColor` → `Rect`/`Color` `From` 변환)<br>`crates/rbms-render/src/lib.rs` (**E-prim 에서 인계**: `pub mod skin_render;` 선언 추가)<br>`apps/rbms-player/src/app_play.rs`<br>`apps/rbms-player/src/gpu.rs` (**E-prim 에서 인계**: BGA shim 제거) | E-prim(머지 완료), E-load, **Phase C1 PlaySession**(play 화면만), **Phase F 머지 완료** |
| **E-ui** | E6 스킨 선택·커스터마이즈 UI | `apps/rbms-player/src/skin_select.rs` (신규)<br>`apps/rbms-player/src/settings.rs`<br>`apps/rbms-player/src/main.rs` (**`mod skin_select;` 한 줄 + SKIN 탭 라우팅**) | E-load, **Phase I 머지 완료** |

**소유권 검증(웨이브 3)**: `{rbms-render/src/{skin,playfield,select,result,hud,skin_render,lib}.rs, rbms-player/src/{app_play,gpu}.rs} ∩ {rbms-player/src/{skin_select,settings,main}.rs} = ∅`.
`main.rs` 는 Phase E 안에서 **E-ui 만** 소유하고, `gpu.rs`/`lib.rs` 는 웨이브 1 의 E-prim 이 이미 머지된 뒤 E-screen 이 이어받으므로 동시 편집이 아니다.

### BGA 경로 — 2단 분할 (충돌 제거의 핵심)

원안은 "BGA 특수경로 제거 + `app_play.rs` 호출부 10곳 치환"을 통째로 E-prim 에 두었는데, 그 10곳은 전부 `app_play.rs` 안이라 E-screen·Phase F 와 3자 충돌한다. 그래서 둘로 나눈다.

- **E-prim(웨이브 1)**: `gpu.rs` 안에서 텍스처 레지스트리를 만들고, `set_bga(&mut self, rgba: &[u8], rect: Rect)` / `clear_bga(&mut self)` 의 **시그니처를 유지한 채 내부만** `register_texture("__bga", …)` + `draw_textured_quad` 위의 얇은 shim 으로 교체한다. 전용 파이프라인·유니폼은 이때 삭제하되 `BGA_DIM` 상수는 shim 이 입력 길이를 검사하는 데 계속 쓰므로 **남긴다**. `app_play.rs` 는 **한 줄도 건드리지 않는다** → Phase F 와 충돌 없음.
- **E-screen(웨이브 3, F 머지 후)**: `app_play.rs` 의 호출부 10곳을 새 API 로 치환하고, 그다음 `gpu.rs` 에서 shim 과 `BGA_DIM` 을 삭제한다. 이때 비로소 256x256 제약이 사라진다.

호출부 실측(현 시점, Phase I 이후 함수명 기준 재확인): `app_play.rs` 489 / 533 / 534 / 559 / 560 / 618 / 646 / 693 / 728 / 754 — `set_bga` 2곳(533, 559), `clear_bga` 8곳. **계획 문서의 "9곳" 은 오기이며 10곳이 맞다.**

### 충돌 회피 주의

- `apps/rbms-player/src/{main,settings}.rs` 는 Phase I 가 NETWORK 탭으로 손대는 중 → E-ui 는 **Phase I 머지 후** 착수하고, `settings.rs`·`main.rs` 변경은 **탭 등록/모듈 선언 최소 라인**으로 제한한다.
- `app_play.rs` 는 Phase F 와 E-screen 이 둘 다 노린다 → **F 완료 후 E-screen**.
- 루트 `Cargo.toml` 과 `crates/rbms-skin/src/lib.rs` 는 **웨이브 0 이후 아무도 수정하지 않는다.** 새 의존성이나 새 모듈이 필요해지면 웨이브 0 브랜치에 추가 커밋을 요청하고 재머지한다(브랜치가 임의로 손대지 않는다).

---

## 9. 단계별 순서와 검증

| # | 단계 | 브랜치 | 완료 판정 |
|---|---|---|---|
| 0 | 크레이트 스캐폴드 + Cargo/divergence 문서 | E-scaffold | `cargo build --workspace` 통과, 기준선 테스트 유지 |
| 1 | PNG 골든 하네스 (**§9.1 결정 선행**) | E-prim | 기존 골든 전부 통과 + 신규 PNG 골든 3장 생성/비교, `RBMS_GOLDEN_UPDATE` 미설정 시 파일 부재는 실패 |
| 2 | trait 확장 + CpuCanvas 텍스처/클립/회전 | E-prim | 샘플링 2종·블렌드 **4종**(§2.1.1)·회전 부호(§2.1.2)·center 10종 환산 단위 테스트, 클립 교집합·빈 클립 테스트 |
| 3 | wgpu 텍스처 레지스트리 + 연속구간 배칭 + 시저 | E-prim | 같은 장면을 CpuCanvas/Gpu 로 그려 블록 시그니처가 허용오차 내 일치(수동 1회), 256 정렬 패딩 테스트(비정렬 폭 300px) |
| 4 | BGA shim 화 (`gpu.rs` 내부만, `app_play.rs` 불변) | E-prim | 기존 BGA 표시 회귀 없음(수동 1회), 전용 파이프라인 심볼 소멸, `app_play.rs` diff 0줄 |
| 5 | 글리프 아틀라스 | E-prim | 텍스트 골든 재생성(**별도 커밋**), 캐시 상한 유지 |
| 6 | TimerId 표 + TimerState | E-timer | §3.1 스팟체크 ID 상수 존재 assert (**전수 판정은 단계 8 의 생성 개수 assert 가 담당** — §3.1 표는 전수가 아니다) |
| 7 | dst 보간기 + op/draw 게이팅 | E-timer | §3.2 테스트 매트릭스 전건 + §3.3 테스트 전건 |
| 8 | 프로퍼티 상수 생성(TIMER 포함) + 레지스트리 | E-prop | 총 968개 대조, 접두사별 합계 일치, 음수 부정, 기본값, Float 클램프 |
| 8a | **1차 출하: play 대역**(`SkinProperty` 100~150 + §3.1 play 타이머) 을 `SkinStateSource` 에 실제 배선 | E-prop | 해당 대역 대표 ID 20개 `FakeState` 왕복 |
| 8b | **2차 출하: select/result 대역**(250~350) 배선 | E-prop | 동상 20개 |
| 8c | **3차 출하: 잔여 대역** 배선(미구현은 기본값 유지) | E-prop | 미구현 ID 가 패닉 없이 기본값을 돌려주는지 |
| 9 | serde 모델 + json5 폴백 | E-load | 실 배포 스킨 픽스처(직접 저작한 최소 스킨) 파싱, 센티널 상속 |
| 10 | 파일 해석 + customfile + traversal 거부 | E-load | §5.4 / §5.4.1 테스트 |
| 11 | Lua 샌드박스 | E-load | 화이트리스트/예산/결정성 테스트 |
| 12 | select 화면 이식 | E-screen | 스킨/폴백 골든 2장 |
| 13 | result → decide → keyconfig | E-screen | 화면당 골든 2장 |
| 14 | play 화면 이식 (**C1 이후**) + `app_play.rs` BGA 치환 + shim 삭제 | E-screen | 골든 2장 + 실기 1회, `BGA_DIM` 심볼 소멸 |
| 15 | SKIN 탭 UI | E-ui | 설정 저장/재로드 왕복 테스트, 텍스처 해제 확인 |

**E3 단계적 출하(8a/8b/8c)의 근거**: 상위 계획 §2 E3 의 "play 100~150 → select/result 250~350" 순서를 유지하기 위한 분할이다. 상수 **생성**(단계 8)은 일괄이 맞지만(손번역 금지), 게임 상태 **배선**은 대역별로 나눠 중간 출하 지점을 만든다. 8a 만 끝나도 play 화면 스킨 실험이 가능하다.

### 9.1 착수 전 필수 결정 — 골든 PNG 커밋 여부 (블로커)

단계 1 은 `tests/golden/*.png` 를 리포지토리에 두는 것을 전제한다. **이 결정이 나기 전에는 웨이브 1 의 E-prim 을 시작할 수 없다**(웨이브 1 전체가 여기서 막힌다). 두 선택지:

- **A안(권장)**: PNG 3장 이하를 커밋한다. 16x16~64x64 의 코드 생성 텍스처 장면이라 장당 수 KB 수준이고, 회귀 시 시각적 진단이 가능하다.
- **B안**: PNG 는 커밋하지 않고 `signature_hash` 만 커밋한다. `assert_golden_png` 는 해시 비교로 동작하고, 불일치 때만 `target/golden-out/` 에 실제/기대 PNG 를 쓴다(기대 PNG 는 그 자리에서 재생성). 바이너리 0.

B안이면 §2.5 의 "파일이 없고 `RBMS_GOLDEN_UPDATE=1` 이면 생성 후 통과" 규칙은 **해시 파일**에 적용된다. 어느 쪽이든 §2.5 API 시그니처는 그대로다. 이 결정은 `docs/acknowledge/2026-09-09-enhancement-decisions.md` 에 기록한다.

---

## 10. 리스크

| # | 리스크 | 영향 | 완화 |
|---|---|---|---|
| R1 | `write_texture` 256 바이트 정렬 미준수 | 임의 크기 텍스처가 런타임 패닉/깨짐 | 단계 3에서 패딩 헬퍼 + 비정렬 폭(예: 300px) 전용 테스트 |
| R2 | 배칭이 순서를 재배열 | z-order 붕괴(스킨이 통째로 깨져 보임) | §2.2 를 코드 주석이 아닌 테스트로 고정: 인터리브 장면의 픽셀 결과 비교 |
| R3 | 프로퍼티 ID 손번역 | 조용한 오작동(스킨이 엉뚱한 값 표시) | 기계 추출 강제(§4.1), 손번역 금지 |
| R4 | Lua 가 프레임마다 파싱 | 프레임 드랍 | 로드 시 컴파일 + RegistryKey 캐시 |
| R5 | Lua 샌드박스 탈출 | 임의 파일 접근 | StdLib 최소화 + 금지 전역 nil assert 테스트 |
| R6 | 와일드카드 무작위로 골든 불안정 | CI 간헐 실패 | `rng_seed` 고정 + 테스트에서 항상 시드 지정 |
| R7 | `app_play.rs` 를 F/I 와 동시 수정 | 머지 충돌 다발 | §8 순서 엄수(F·I 머지 후 E-screen) |
| R8 | 아틀라스 도입이 전 골든 무효화 | 리뷰 노이즈 | 단계 5를 독립 커밋으로, 해시 재생성 diff 만 포함 |
| R9 | 텍스처 누수(스킨 재로드) | RSS 증가 | `release_texture` 필수 + 재로드 100회 소크로 RSS 추세 확인 |
| R10 | E5 가 기존 화면을 대체해버림 | 스킨 없는 사용자 회귀 | 폴백 경로 삭제 금지(§6-3) |
| R11 | 골든 PNG 커밋 여부 미결 | **웨이브 1 전체가 착수 불가** | §9.1 을 웨이브 0 종료 전에 결정한다. 미결이면 B안(해시만)으로 진행하고 나중에 A안으로 승격 |
| R12 | blend/center/각도 부호를 통례로 추정 | 화면이 조용히 어긋남(감산·상하반전·역회전) | §2.1.1/§2.1.2 표는 레퍼런스 실측이다. 단계 2 에서 blend 4종·center 10종·각도 부호를 **픽셀 assert** 로 고정 |
| R13 | 크레이트 스캐폴드 없이 웨이브 1 착수 | E-timer 가 빌드 불가 → 루트 `Cargo.toml` 동시 수정 → 충돌 | 웨이브 0(§8)을 단독 머지 후에만 웨이브 1 시작 |

---

## 11. 미확인 사항 (추정 금지 — 구현 직전 확인 필요)

> 2026-09-09 비평 대응으로 아래 항목 대부분을 코드/레퍼런스 직접 확인으로 **해소**했다. 해소된 것은 §11.1 에 답과 함께 남기고(재조사 금지), 진짜 미확인만 §11.2 에 둔다.

### 11.1 해소됨 (확인 완료 — 다시 조사하지 말 것)

| # | 질문 | 답 | 근거 |
|---|---|---|---|
| 1 | `CW`/`CH` 의 정의 위치·값 | `apps/rbms-player/src/main.rs:53-54`, `const CW: u32 = 1280; const CH: u32 = 720;` (crate 루트 비공개 const). **Phase I 재확인 불필요** — 값 자체가 확정됐고 Phase I 는 이 상수를 바꾸지 않는다 | 실측 |
| 2 | `app_play.rs` BGA 호출부 개수 | **10곳**(`set_bga` 2 / `clear_bga` 8) @ 489/533/534/559/560/618/646/693/728/754. 계획 문서의 "9곳"이 오기 | 실측 |
| 4 | `Destination.center` 0~9 앵커 매핑 | §2.1.2 표로 확정(정규화 비율 + y 반전 환산 + LR2 넘패드 배치) | `SkinObject.java:80-81`, `Skin.java:70-75` |
| 5 | `stretch`(StretchType) 시맨틱 | §5.6 표로 확정(값 0~10, `-1`=미지정→STRETCH) | `StretchType.java:16-105` |
| 6 | `dstfilter` ↔ `imageType` 전환 규칙 | §5.6 로 확정. `TYPE_BILINEAR` 는 하드웨어 필터가 아니라 셰이더 → rbms 는 하드웨어 `Linear` 로 근사(divergence 기록) | `SkinObject.java:634-636`, `Skin.java:82-86,518-523` |
| 7 | blend 정수값 ↔ 블렌드 함수 | §2.1.1 표로 확정. 실존 값은 **2/4/9** 뿐이고 3 은 실질 Add. `XOr = blend 5` 는 **존재하지 않는 값이었다**(원안 오류, 삭제됨) | `Skin.java:636-645,659-663` |
| 8 | `relative` 플래그의 출처 | JSON 필드가 아니다. 로더가 **판정 카운트 오브젝트에만** 코드로 켠다 → §5.7 | `JsonPlaySkinObjectLoader.java:260` (전체에서 `setRelative(true)` 호출 2곳뿐) |
| - | `SkinProperty` 상수 총수 | `public static final int` **968개**(원안의 "708개 이상"은 하한만 맞음) | `grep -c` |
| - | `SkinConfig` 필드 수 | pub 필드 **40개**(원안 "43필드"는 오기) | `crates/rbms-render/src/skin.rs:12-64` |
| - | 스킨 타입 목록 | 19종 전체 확정 → §7.1 | `SkinType.java:12-30` |
| - | customfile → filemap 경로 | §5.4.1 로 확정 | `JSONSkinLoader.java:145-157,223-225`, `SkinHeader.java:174-203` |
| - | op/draw 를 누가 언제 평가하나 | `prepare()` 진입 즉시 `dstdraw` 전건 평가, 하나라도 false 면 `prepareRegion` 도 안 탄다. 정수 `op` 는 로더가 BooleanProperty 로 변환해 `dstdraw` 에 합류 → §3.3 | `SkinObject.java:282-298,590-610` |
| - | `SkinObjectRenderer` 파일 위치 | 독립 파일이 아니라 **`Skin.java` 내부 클래스**(약 500-700행). "미독"이 아니라 파일을 잘못 짚었던 것 | 실측 |

### 11.2 여전히 미확인 (구현 직전 확인 필요)

1. **`OPTION_*` / `NUMBER_*` / `STRING_*` 의 전체 ID 목록과 대역 경계**: §4.1 기계 추출(총 968개 대조)로 확정한다. §4.2 표는 직접 확인한 앵커만 담은 스팟체크용이다.
2. **`Skin` 최상위의 `input`/`scene`/`close`/`loadend`/`playstart`/`judgetimer`/`finishmargin` 의 정확한 의미**: 필드 존재만 확인. 화면 전이 타이밍에 직결되므로 **E5(단계 12) 착수 전** `MainState`/각 화면 클래스에서 소비처를 확인한다.
3. **Phase C1 `PlaySession` 의 최종 API**: 미구현이라 §4.3 매핑은 "상태 원천이 거기"까지만 확정. C1 확정 후 `SkinStateSource` 구현체를 작성한다(단계 8a 의 선행).
4. **Phase I 의 IR 상태 표면**: TIMER 172~174 및 IR 관련 프로퍼티가 `HttpScoreServer` 의 어떤 상태에 붙는지는 **Phase I 완료 후** 확정.
5. **`json5` / `mlua` 크레이트의 최신 버전·serde 호환**: 실제 추가 시 공식 문서로 확인한다. `mlua` 의 Lua 버전(5.4 vs LuaJIT), `vendored` feature 사용 여부, 라이선스가 레포의 GPL-3.0 과 호환되는지(mlua 는 MIT — 호환, 단 vendored Lua 는 MIT 로 별도 표기 필요)를 함께 확인한다.
6. **골든 PNG 커밋 여부**: §9.1 참조. **웨이브 1 의 블로커**이므로 웨이브 0 종료 전에 결정한다.
7. **`stretch` 8~10(NO_EXPANDING / NO_RESIZE 계열)의 `trimmedImage` 세부 산식**: enum 값과 의도는 §5.6 에서 확정했으나 `fitWidthTrimmed`/`fitHeightTrimmed` 헬퍼 본문은 미독. E5 에서 해당 값을 실제로 쓰는 스킨을 만나면 그때 확인한다(그 전까지는 `STRETCH` 폴백 + warn).

---

## 12. 비평 반영 (2026-09-09)

완전성 비평(검증자 1인)에 대해 **인용된 코드를 전부 다시 열어 확인**한 뒤 반영한 내역이다.

**반영(비평이 옳았음)**

1. **§2.1 BlendMode 표 전면 교체** — `Skin.java:636-645` 실측 결과 실존 값은 2/4/9 뿐이고, 원안의 `XOr = blend 5` 는 존재하지 않았다. blend 3 은 `FUNC_SUBTRACT` 를 걸었다 draw 전에 `FUNC_ADD` 로 되돌리므로 실질 Add. `Subtract` 변형과 §2.4 의 감산식을 삭제하고 §2.1.1 환산표·§2.4 GL factor 기반 식으로 교체했다.
2. **§2.1.2 center 환산표 신설** — `SkinObject.java:80-81` 의 `CENTERX/CENTERY` 는 정규화 비율(0..1)에 libGDX y-up 이다. `QuadParams.center` 를 "픽셀, y-down" 으로 유지하되 `(cx*w, (1-cy)*h)` 환산표 10행을 넣고, 각도 부호 역전(`angle_deg = -skin angle`)도 명시했다.
3. **§0 앵커 수정** — `set_bga`@247 / `clear_bga`@261, `cpu.rs` 15/21-77/78, `SkinConfig` 40필드, `CW/CH = 1280/720`@`main.rs:53-54`, `JsonSkin.Image`@112, `tests/golden.rs` 는 기존 파일. `SkinProperty` 상수 968개.
4. **§8 소유권 표 전면 재작성 + 웨이브 0 신설** — 무주인 파일 5건(`font.rs`, `rbms-render/Cargo.toml`, `rbms-player/Cargo.toml`, `main.rs`, `From` 변환 배치처)과 소유권 충돌을 전부 해소했다. `From` 변환은 신규 `skin_render.rs`(E-screen)로 못 박았다. 각 웨이브에 **소유권 검증** 줄(교집합 공집합)을 추가했다.
5. **BGA 2단 분할** — `app_play.rs` 를 E-prim 이 건드리지 않도록 웨이브 1 은 `gpu.rs` 내부 shim 까지만, 호출부 치환·shim 삭제는 웨이브 3 E-screen(F 머지 후)으로 옮겼다(단계 4 / 단계 14).
6. **§3.3 op/draw 게이팅 신설** — 계획 §2 E2 의 누락 항목. `prepare()` 5단계 순서, 정수 op → BooleanProperty 합류, `mouseRect`, `DrawCondition`/`LuaExprId` 소유 경계(E-timer)까지 규정했다.
7. **§5.4.1 customfile 신설** — 계획 §2 E4 의 누락 항목. `filepath[]` → CustomFile → 후보 열거 → filemap 생성 경로와 §7 E6 UI 데이터 원천을 잇고, `SkinUserConfig::filepaths` 가 파일명(경로 아님)을 담음을 명시했다.
8. **§9 8a/8b/8c 단계적 출하** — 계획 §2 E3 의 "play 100~150 → select/result 250~350" 순서를 상수 생성(일괄)과 상태 배선(대역별)으로 분리해 복원했다.
9. **§2.2 계획 상충 기록** — "per-texture 배칭" 계획 문구를 이 사양이 대체한다는 결정을 웨이브 0 이 `docs/acknowledge/` 에 기록하도록 명시했다.
10. **§7.1 스킨 타입 19종** — `SkinType.java:12-30` 실측. 원안 5종은 과소였다(COURSE_RESULT·SKIN_SELECT·SOUND_SET·THEME·배틀·24KEYS 누락).
11. **§9.1 골든 PNG 결정 게이트 + R11** — 미결 결정이 웨이브 1 을 막는다는 사실을 리스크·단계 표에 명시하고 A/B 선택지를 적었다.
12. **§3.1 전수 아님 명시 + TIMER 기계 추출 편입** — 단계 6 의 완료 판정을 "스팟체크"로 격하하고 전수 판정을 단계 8 로 옮겼다. PM_CHARA 902/903 등 생략분을 표기했다.
13. **§5.6 / §5.7 신설** — stretch 11종, dstfilter 규칙, `relative` 출처를 미확인에서 확정으로 옮겼다.

**반려(비평이 틀렸거나 과했음)** — 없음. 인용된 갭은 전부 코드에서 재현·확인됐다. 다만 다음 두 건은 **표현만** 조정했다.

- 비평은 §4 의 "708개 이상"을 "추정으로 보임"이라 했으나 `이상` 이므로 거짓 진술은 아니었다. 그럼에도 검증 하한이 느슨해지는 지적은 타당해 **실측 968** 로 교체하고 접두사별 합계 대조를 §4.1-4 에 추가했다.
- 비평의 §5.4 라인 인용 정정(94-127 → 95-129)은 맞다. 다만 라인 1~2줄 오차는 레퍼런스 버전 차이로도 발생하므로, 본문은 라인 대신 **함수명(`SkinLoader.getPath`)** 기준으로 바꿨다.
