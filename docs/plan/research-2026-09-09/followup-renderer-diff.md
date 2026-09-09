# X-01 — Renderer 텍스처 프리미티브 추가 effort 재산정

전량 통독 대상: `crates/rbms-render/src/{lib.rs,cpu.rs,font.rs,playfield.rs,hud.rs,select.rs,result.rs}`(합 2,404줄), `apps/rbms-player/src/gpu.rs`(354줄). 보조 확인: `apps/rbms-player/src/app_play.rs:460-545`, `crates/rbms-render/examples/*.rs`.

## 0. 현행 사실 (근거)

| 항목 | 사실 | 근거 |
|---|---|---|
| Renderer trait | 메서드 3개(`size`/`clear`/`fill_rect`)뿐. 텍스처·클립 개념 없음 | `lib.rs:61-65` |
| 디스패치 | 전 호출부가 `<R: Renderer>` 제네릭. `dyn Renderer` 사용처 없음 | `playfield.rs:18`, `hud.rs:55`, `select.rs:214`, `result.rs:86`, `font.rs:171` |
| GPU 쿼드 | `Instance{rect:[f32;4], color:[f32;4]}` 인스턴스 1개 = fill_rect 1회, 전체 1 draw call | `gpu.rs:54-59, 61, 317-322, 348-353` |
| GPU 텍스처 | BGA 전용 파이프라인 1개. 텍스처 **1슬롯 고정 256×256**, uniform에 screen+rect만, `blend: None`, uv=쿼드 코너 고정 | `gpu.rs:32-52, 178-225, 219, 247-259` |
| 그리기 순서 | BGA는 항상 쿼드보다 **먼저** 무조건 1회 → 텍스처는 영구 최하단 레이어 | `gpu.rs:312-322` |
| 커버(선곡) | BGA 슬롯을 재사용. 그래서 select는 커버 사각형을 일부러 **비워 둠**(텍스처가 비쳐 보이게) | `app_play.rs:511`, `select.rs:134, 140-151` |
| 텍스트 | 글리프를 런 병합해 **1px 높이 `fill_rect`** 로 방출. 텍스처 미사용 | `font.rs:101-131`(특히 `:128`), `merge_runs:139-157` |
| 클리핑 | 전무. `fit_text`/`wrap_two`/수동 `max()` 로 오버플로 회피 | `playfield.rs:95-100`, `select.rs:164-177`, `font.rs:186-205` |
| CpuCanvas | 캔버스 경계 클램프만, 저장 알파는 항상 255로 강제(테스트가 이 규약을 검증) | `cpu.rs:39-61, 58`, `cpu.rs:102-110` |
| 테스트 | crate 내 `#[test]` **106개**. 전부 `pixel_at` 프로브/순수함수 assert. **PNG 골든 비교 하네스 없음** | `cpu.rs`(13), `font.rs`(19), `skin.rs`(27), `playfield.rs`(29), `result.rs`(14), `theme.rs`(4) |
| 예제 출력 | PPM(P6)을 손으로 write. 이미지 크레이트/골든 비교 없음 | `examples/render_frame.rs:54`, `text_render.rs:31`, `render_result.rs:28`, `render_select.rs:16` |

**검증자 상충의 원인**: gpu.rs 에 "이미 동작하는 텍스처 파이프라인"이 있다는 사실만 보면 S, 그 파이프라인이 *단일 슬롯·고정 크기·고정 순서·틴트/클립/회전 없음* 이라 사실상 그 90여 줄 구간을 재작성해야 한다는 점을 보면 L 로 보인다. 실측 규모는 **글리프 아틀라스·회전을 빼면 M**이다.

## 1. 최소 변경안 — 항목별 구체 변경과 예상 LOC

전제: trait 신규 메서드에 **기본 구현(default impl)** 을 준다. 그러면 기존 두 impl(`CpuCanvas`, `Gpu`)과 전 호출부가 무수정 컴파일된다.

| # | 파일 | 변경 내용 | 예상 LOC |
|---|---|---|---|
| 1 | `lib.rs:61-65` | `TextureId(u32)`, `Blend{Alpha,Opaque,Add}` 타입 추가 + trait 에 `register_texture/update_texture/free_texture/draw_textured_quad/push_clip/pop_clip` 6개를 default impl(텍스처=`fill_rect(dst,tint)` 폴백, 클립=no-op)로 추가 | +45 ~ +60 |
| 2 | `gpu.rs:54-61` | `Instance` 확장: `rect[4] + color[4] + uv[4] + xform[4](angle, cx, cy, flags)`. 32B→64B, `INSTANCE_ATTRS` 2→4 어트리뷰트 | +10 / -3 |
| 3 | `gpu.rs:12-30` (WGSL) | 단일 셰이더로 통합: uv 보간 출력, `textureSample * color` (무텍스처는 **1×1 흰색 더미 텍스처** 바인딩으로 같은 경로 통과), 회전 행렬(center 기준) 적용 | +25 ~ +35 |
| 4 | `gpu.rs` 신규 | 텍스처 레지스트리: `Vec<Option<TexEntry{texture,view,bind_group,w,h}>>` + freelist, 샘플러 1개 공유, 등록 시 bind group 즉시 생성 | +50 ~ +60 |
| 5 | `gpu.rs:247-259` | `write_texture` 일반화. **`COPY_BYTES_PER_ROW_ALIGNMENT`(256B) 패딩 필요** — 현재 코드는 `256*4=1024`가 우연히 정렬돼 동작. 임의 폭은 스테이징 행 패딩 경로 필요 | +20 ~ +25 |
| 6 | `gpu.rs:270-327` | **배치 전략(권고: per-texture draw call)**: `quads: Vec<Instance>` 를 순서 유지 단일 스트림으로 두고, `batches: Vec<{start,count,tex,blend,scissor}>` 를 병행 기록. 연속 동일 키는 한 draw 로 병합 → 전부 무텍스처면 **현재와 동일하게 1 draw**. 정렬/페인터 순서는 인스턴스 인덱스로 완전 보존 | +55 ~ +75 |
| 7 | `gpu.rs:32-52,178-225,247-263,312-316` | BGA 특수 경로 **삭제**(전용 셰이더/파이프라인/유니폼/bind group/`set_bga`/`clear_bga`/선행 draw). BGA·커버는 앱이 `register_texture`+`draw_textured_quad` 로 발행 | -55 ~ -70 |
| 8 | `app_play.rs:467,511-512,537-538,601,629,676,711,737` | `set_bga/clear_bga` 호출 9곳을 등록/발행으로 치환. BGA 텍스처는 `bga_images` 키별 1회 등록 캐시 | +35 / -12 |
| 9 | `gpu.rs` 클립 | `push_clip/pop_clip` → 스택(교집합) + 배치 경계 + `rp.set_scissor_rect`. **주의 2가지**: (a) 셰이더 좌표는 논리 `CW/CH`(`gpu.rs:125,339-341`)인데 시저는 물리 픽셀 → `config.width/height` 스케일 변환 필수, (b) 화면 밖 시저는 wgpu 패닉 → 클램프 | +20 ~ +30 |
| 10 | `cpu.rs` 참조 구현 | 텍스처 저장(`Vec<(w,h,Vec<u8>)>`), 최근접 샘플 + 틴트 곱 + source-over(기존 알파=255 강제 규약 유지), 클립 사각형 교집합을 `fill_rect`/`draw_textured_quad` 양쪽에 적용 | +60 ~ +75 |
| 11 | `cpu.rs` 회전 | 회전 지원 시 목적지 바운딩박스 순회 + 역변환 샘플 경로 추가 (미지원이면 0) | +35 ~ +50 |
| 12 | 테스트 | 신규: 텍스처 등록/샘플/틴트/클립 교집합/해제 후 재사용, gpu 배치 병합 카운트(순수 함수로 분리 시) | +80 ~ +120 |

**합계(회전 제외)**: 신규/수정 약 **+400 ~ +500 LOC, 삭제 약 -70 ~ -85 LOC**, 파일 4개(`lib.rs`, `cpu.rs`, `gpu.rs`, `app_play.rs`) + 테스트.

### bind group 전략 비교 (권고 근거)

| 전략 | 장점 | 단점 | 판정 |
|---|---|---|---|
| **per-texture draw call + 배치 병합** | 순서 완전 보존, 셰이더 단순, 전 백엔드(GL 포함) 동작, 무텍스처 프레임은 현행과 동일한 1 draw | 텍스처 종류 수만큼 draw(현실적으로 BGA 1 + 커버 1 + 글리프 아틀라스 1 = 3) | **권고** |
| 텍스처 배열(binding_array) | draw 1회 유지 | non-uniform indexing feature 요구 → 백엔드 가용성 리스크, 크기 이질(256² vs 160²) 처리 필요 | 비권고 |
| 단일 아틀라스 | draw 1회, 글리프에 최적 | 패커+퇴출 정책 필요, 대형 BGA(256²)·가변 커버를 함께 넣으면 용량·프래그멘테이션 문제 | **글리프 전용**으로만 |

## 2. 기존 호출부·테스트 파급

| 대상 | 깨짐 | 근거/비고 |
|---|---|---|
| `render_playfield`/`render_lane_cover`/`render_key_bomb` | **그대로** | `clear`+`fill_rect` 만 사용(`playfield.rs:19,24,32,69,99,129,150-158,173,641-645`) |
| `render_hud` | **그대로** | 동일(`hud.rs:31-52,70-128`) |
| `render_select`/`render_result` | **그대로** | 동일. 단 select 커버는 "GPU가 밑에 깐다"는 전제(`select.rs:134,141`)가 남음 — 텍스처를 렌더러가 직접 그리게 하려면 `DetailView`에 `TextureId` 추가가 **추가 변경**(최소안 범위 밖) |
| `font.rs` 1px 런 방출 | **그대로**(아틀라스 제외 시) | `font.rs:128` 은 계속 `fill_rect`. 아틀라스 도입 시에만 `TextEngine::draw`(101-131) 교체 |
| `Gpu::quad_count` 디버그 지표 | 의미 변화 | `gpu.rs:266-268` — 텍스처 쿼드 포함으로 카운트 정의가 바뀜(표시 문구만 조정) |
| 106개 테스트 | **전부 통과 예상** | 전부 `CpuCanvas::pixel_at` 프로브·순수함수(`playfield.rs:225`, `result.rs:276`, `font.rs:428-464`, `cpu.rs` 13건). `fill_rect` 시맨틱만 안 건드리면 무영향 |
| PNG 골든 비교 | **존재하지 않음** | 예제는 PPM 수동 기록(`render_frame.rs:54` 등). 회귀 안전망이 없다는 뜻이기도 함 — 텍스처 도입 전 골든 하네스를 먼저 두는 편이 안전 |
| 잠재 회귀 지점 | (a) `cpu.rs:58` 알파 255 강제 규약을 텍스처 경로가 어기면 `cpu.rs:102-110` 테스트가 깨짐. (b) 나중에 composer가 `push_clip`을 쓰기 시작하면 `playfield.rs:96` 수동 상단 클램프·`fit_text` 절단이 중복이 되고 `playfield.rs:225` 류 픽셀 기대값이 흔들릴 수 있음 | |
| 동작 개선(부수) | BGA 슬롯 단일 공유 제약(`app_play.rs:511` vs `537`)이 해소되어 커버/BGA 공존 가능, 텍스처를 임의 z 순서에 배치 가능 | |

## 3. effort 등급과 권고 순서

| 범위 | 등급 | 근거 |
|---|---|---|
| trait + gpu 배치/레지스트리 + CpuCanvas 참조(회전 제외, 글리프 아틀라스 제외) | **M** | 약 400-500 LOC, 파일 4개, 기존 호출부 0 수정, 테스트 0 수정 |
| 위 + 회전 | **M(상단)** | CpuCanvas 역변환 샘플 +35-50 LOC 뿐이나 현재 필요 호출부 없음(키봄은 축정렬 사각형 `playfield.rs:641`) |
| 위 + 글리프 아틀라스(font.rs) | **L** | 아틀라스 할당/성장, `(CacheKey)`→region 캐시(색은 tint 로 이동), CPU 폴백 경로 유지, `merge_runs`+전용 테스트 6건(`font.rs:263-347`) 정리, 대신 텍스트 쿼드 수가 런 단위→글리프 단위로 급감 |
| 위 + 스킨 스프라이트(LR2/beatoraja 이미지 스킨) | **XL** | 이미지 디코드·스킨별 리소스 수명·추가 블렌드 모드까지 딸려 옴. 별건으로 분리 권고 |

### 권고 순서

1. `lib.rs` trait + default impl (S) — 이 커밋만으로는 동작 변화 0, 컴파일 안전.
2. `gpu.rs` 텍스처 레지스트리 + per-texture 배치 draw + BGA/커버를 그 위로 이관, BGA 특수경로 삭제 (M) — 여기서 순서 자유도와 다중 텍스처가 열림.
3. `push_clip/pop_clip`(시저) (S) — 배치 경계 로직이 2와 동일해 직후가 가장 싸다.
4. `CpuCanvas` 텍스처 샘플/클립 (S~M) — 테스트·예제 패리티 회복.
5. (선택) 골든 PPM/PNG 비교 하네스 도입 — 현재 회귀 안전망이 없으므로 6 이전에 두는 것이 안전.
6. `font.rs` 글리프 아틀라스 (M) — 성능 이득이 가장 크지만 1-4 이후.
7. 회전 (S) — 호출부 수요 생길 때.
