# 2026-05-31 — 곡선택(SONG SELECT) 전면 재설계 (레퍼런스 구현 modern chic)

## 대상
- `crates/rbms-parser/src/lib.rs` — `#BANNER`/`#PREVIEW` 헤더.
- `crates/rbms-chart/src/lib.rs` — `note_density`/`NoteDensity`(밀도 포팅).
- `crates/rbms-render/src/select.rs` (신규) — `SelectView`/`SelectHot`/`render_select`(재설계 렌더).
- `crates/rbms-render/examples/render_select.rs` (신규) — 헤드리스 PNG 검증.
- `apps/rbms-player/src/main.rs` — `SongEntry`(커버/배너/프리뷰), `ChartDetail`(밀도), `compute_chart_detail`, `refresh_focused_detail`(커버 디코드), `build_select_view`+캐시, Select 렌더 arm 교체, `difficulty_color`.
- 스펙: `docs/reference/ui-select-redesign.md`. 타깃: `docs/reference/ui/provided/06-reference-select-target.png`.

## 리포트
곡선택을 06 레퍼런스(레퍼런스 구현 modern chic) 수준으로 끌어올렸다. 1차 재설계(플레이/결과 IIDX)에서 곡선택은 내용만 고도화·비주얼은 MVP였다.

**근본 결정 — 렌더링을 `rbms-render`로 추출.** 기존 곡선택은 `main.rs`에 인라인이라 `CpuCanvas` 헤드리스 검증이 불가능했다. `playfield/hud/result`처럼 `render_select<R: Renderer>(r, &SelectView) -> Vec<(Rect, SelectHot)>`로 추출 → (1) 헤드리스 PNG로 레이아웃 시각검증, (2) 데이터주도 스킨화 정렬, (3) `main.rs` 비대화 해소. `main.rs`는 `SelectView` 조립 + hot 매핑 + 커버 BGA 업로드만 담당.

**핵심 제약 — 단일 BGA 텍스처 슬롯.** GPU 백엔드는 256² 텍스처 1장을 모든 쿼드 **뒤**에 그린다. 커버(`#STAGEFILE`→`#BANNER`)는 `set_bga(rgba, cover_rect())`로 업로드하고, `render_select`는 커버 영역에 불투명 패널을 깔지 않아(테두리/placeholder만) 텍스처가 비친다. 디코드는 기존 `decode_bga_256` 재사용, 포커스 이동당 1회.

**밀도 그래프 — 레퍼런스 구현 `SongInformation` 정밀 포팅.** 1초 빈, LN 바디는 점유 초마다 카운트, `peak`=최대 빈, `avg`=`total/bins/4` 임계 초과 빈 평균, `end`=게이지 border 이후 5초창 최대평균. 상세 패널 우하에 히스토그램+PEAK/AVG/END notes/sec.

**시각.** 행: KEY/레벨 배지·클리어램프 LED(좌바+우측 세로바)·포커스(시안 테두리+노랑 타이틀)·중앙 포커스 스크롤. 상세: 커버+제목블록(2줄 래핑)·배지행·2열 스탯그리드·밀도 히스토그램·기록(베스트바·DJ랭크바·최근행 EX+랭크+추세)·기록 모달. `#PREVIEW`는 데이터만 준비(재생은 후속 — select 중 `audio=None`).

## 검증
- `cargo test --workspace` **130 통과·0 경고**(파서 `#BANNER`/`#PREVIEW` 1 + `note_density` 4: 일반/LN바디/유령빈/댕글링헤드).
- 헤드리스: `cargo run --example render_select -p rbms-render -- /tmp/f.ppm [modal|long]` → PNG. 곡상세·모달·900빈(15분, 오버플로 없음) 확인.
- 라이브 커버(BGA 텍스처)는 GPU 전용 → `./start.sh` 확인.

## 적대적 멀티에이전트 리뷰 (2라운드)
- **1라운드**(5차원×review→verify, 10에이전트): 16건 확정(3 high·1 med·12 low) — 전부 반영.
  - HIGH: 밀도 빈을 **마지막 노트** 기준으로(마디 패딩→유령 빈 제거, `getLastMilliTime` 미러) · 히스토그램 **오버플로**(긴 곡: 서브픽셀 step+폭 클램프) · `build_select_view` **매 프레임 전체리스트 할당 회귀**(→ `select_key` 메모이제이션, `select_gen`).
  - MED/LOW: 지뢰 `counted` 누락(borderpos) · 댕글링 LN 헤드 0밀도(→ head를 LongStart에서 계상) · `#TOTAL` 없을 때 기본총량(`7.605n/(0.01n+6.5)`) · 최근기록 DJ랭크 프리픽스 복원(dead field 해소) · 타이틀 2줄 래핑 · 중복 DIFFICULTY셀→DENSITY · 그래디언트 앰버화 · 스탯그리드 구분선 · 예제 픽스처(best=clear-then-EX) · import 병합.
- **2라운드**(밀도 재구조화·메모이제이션 무효화·렌더 회귀 타깃, 3에이전트): **0건** — 수정이 전부 정확하고 새 버그 없음으로 확정.

## 후속 (같은 날, 사용자 피드백) — 잘림 수정 · #PREVIEW · KEY BOMB

1. **스탯그리드 잘림 수정**: 상세 패널 2열×3행 → **3열×2행**(`render_song_detail`). 하단 LENGTH/TOTAL 값이 DENSITY 구분선에 잘리던 문제 해결. DENSITY(구분선 348/라벨 362/박스 380–454)·RECORDS(468–) 섹션 위로 정렬, RECORDS 가시 행 증가. 값은 `fit_text`로 컬럼폭 클램프.
2. **#PREVIEW 프리뷰 재생**: 포커스 settle(디바운스 `PREVIEW_DEBOUNCE_FRAMES=20`) 시 `#PREVIEW` 해석(`resolve_file`)·디코드·루프. mixer `play(key)`가 동일 key를 먼저 stop하므로 **선스케줄(룩어헤드)은 현재 클립을 끊음** → 자연 종료 후 경계 재트리거(`while clock_us >= preview_next_us`)로 변경(≈1프레임 갭). select 전용 `preview_audio: AudioEngine`(플레이 엔진과 분리, select 이탈·`load()` 진입부에서 drop해 cpal 스트림 1개 보장). `AudioEngine::sample_duration_us` 추가. **DISPLAY 탭 PREVIEW 토글**(`PlaySettings.preview`/`PlayerConfig.preview` 양방향 매핑·serde default·setting index 21).
3. **KEY BOMB**: `Player.bomb: Vec<(i64,u8)>` — press/release/autoplay-update 전 경로에서 노트 히트(judge≤3, 空POOR=Judge::Poor·miss 제외) 시 `(hit_us, judge_index)` 기록. `rbms_render::render_key_bomb`(판정선 중심 판정색 확장 버스트 + 흰 코어 수축, age/skin.bomb_us로 페이드). **`SkinConfig` bomb_enabled/bomb_height/bomb_duration_ms 데이터주도**(기존 RON serde default 호환). 테스트 2개(히트=발화·빈프레스=무발화). `render_frame` 예제로 PNG 시각검증.

- 적대적 리뷰(4차원): **1건(LOW)** — 리플레이 직접 실행(`play_record_replay`)이 Loading 단계를 건너뛰어 프리뷰 정지 없이 플레이 엔진 생성(1프레임 cpal 스트림 2개). 근본수정: `load()` 진입부에서 `stop_preview()`(모든 play-entry 경로 커버).

## 상세
- `note_density` 카테고리: `[scr LN-head, scr LN-body, scr normal, key LN-head, key LN-body, key normal, mine]`. `bin_sum`=cats0-5(지뢰 제외). 댕글링 LN 헤드도 head-time에 계상(rbms는 미종결 LN 보존).
- `SelectKey=(select_gen, sel, record_modal, scores_len, score_graph)`. `refresh_focused_detail`(focused_detail+cover 갱신) → `refresh_select_cache` 순으로 호출. 커버 텍스처는 캐시와 별개로 매 프레임 `set_bga`.
- KEY BOMB 데이터: `bomb[lane]=(hit_us, judge_idx)`, `i64::MIN`=없음. 렌더는 `microtime - hit_us` 전에 `i64::MIN` 가드(오버플로 방지). 색=`skin.judge_colors[judge]`.
- 프리뷰 루프: `play()`의 `stop(key)` 때문에 선스케줄 불가 → 경계 재트리거. 완전 gapless는 2-키 교대 필요(보류, ROADMAP).
</content>
