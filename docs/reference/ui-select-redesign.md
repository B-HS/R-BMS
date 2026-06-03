# SONG SELECT 전면 재설계 스펙 (beatoraja modern chic 지향)

> 1차 타깃: `docs/reference/ui/provided/06-beatoraja-select-target.png`. 전체 UI 스펙은 `ui-design.md`(이 문서는 곡선택만 정밀화).
> 렌더 프리미티브: `clear` / `fill_rect(Rect)` / `draw_text(_centered/_right)` (scale 1.0≈14px) + **단일 BGA 텍스처 1장**(커버 전용). 좌표공간 1280×720 고정.

## 0. 핵심 결정

1. **렌더링을 `rbms-render::render_select`로 추출**한다. `playfield/hud/result`와 동일하게 `pub fn render_select<R: Renderer>(r: &mut R, v: &SelectView) -> Vec<(Rect, SelectHot)>`. 이유: (a) `CpuCanvas`로 헤드리스 PNG 검증 가능, (b) 데이터주도 스킨화 정렬, (c) main.rs 비대화 해소. main.rs는 `SelectView` 조립 + 반환 hot→`Hot` 매핑 + 커버 BGA 텍스처 set만 담당.
2. **레이아웃은 list-left / detail-right 유지**(ROADMAP "우측 상세"·"현재 골격 유지 가능"). 06의 좌우는 미러지만 목표는 **정보밀도·비주얼 품질**이지 픽셀 미러가 아니다.
3. **커버 이미지**는 단일 BGA 슬롯 사용: `set_bga(cover_rgba, cover_rect)`는 모든 쿼드 **뒤**에 그려지므로, `render_select`는 커버 영역에 **불투명 패널을 깔지 않고** 테두리+placeholder만 그린다(텍스처가 비쳐 보이게). 커버 없으면 `clear_bga` + "NO IMAGE".
4. **밀도 그래프**는 beatoraja `SongInformation` 알고리즘을 정밀 포팅(§3).
5. **`#PREVIEW` 재생은 본 라운드 분리**(select 중 `audio=None` → 엔진 라이프사이클 별도). 파서/데이터는 준비, 재생은 후속. → **갱신(2026-06-03)**: select 전용 `AudioEngine`로 재생 배선·`config.debug` 계측·`samples/preview-demo` 픽스처 완료(가청 확인만 수동 잔여, `docs/bug/2026-06-03-preview-playback.md`).

## 1. 영역 좌표 (1280×720)

| 영역 | x | y | w | h |
|---|---|---|---|---|
| 배경 | 0 | 0 | 1280 | 720 | `clear(8,8,14)` + 헤더띠 |
| 헤더 | 0 | 0 | 1280 | 52 | "MUSIC SELECT" 좌(s2.6), 우측 N/M·정렬 |
| **곡 리스트** | 32 | 60 | 584 | 600 | 행 리스트(§2) |
| **상세 패널** | 632 | 60 | 616 | 600 | 상세(§4) |
| 하단 가이드 | 0 | 694 | 1280 | 26 | 조작 안내(s1.1 GRAY) |

상수: `LIST_X=32, LIST_W=584, DETAIL_X=632, DETAIL_W=616, TOP=60`.

## 2. 곡 바 리스트 (행 재설계)

- 행 높이 `ROW=36`, 간격 `4`. 포커스 행만 `44`. 가시 행 `~15`. 포커스가 중앙 근처 오도록 스크롤(`start = sel - rows/2` clamp).
- 행 내부(행 y기준):
  - **램프 좌측 바**: `x=LIST_X, w=6, h=ROW`, 클리어 램프색(미플레이 `(44,44,54)`).
  - **행 배경**: 포커스 `(50,56,82)` + 테두리 2px `(120,200,240)`(시안), 비포커스 짝/홀 `(22,24,34)`/`(27,29,42)`, 폴더행 `(40,44,30)`.
  - **KEY 배지**: `x=LIST_X+12, y=중앙-12, w=46, h=24`, bg=`mode_color` 어둡게(×0.5)+테두리 mode_color, 텍스트 `mode_short`(7K/5K/9K/10K/14K) s1.2 흰, 중앙.
  - **레벨 배지**: `x=+62, w=40, h=24`, bg=difficulty 색 어둡게, 텍스트 `#PLAYLEVEL` s1.4 흰 중앙.
  - **타이틀**: `x=LIST_X+170, y=중앙`, s1.5(포커스 s1.7), 색 `(240,232,150)`(옅은 노랑, chic) / 비포커스 `(190,186,150)`. `fit_text`로 우측 LED 전까지.
  - **클리어 LED 바**(우측 끝, 06식 세로 막대): `x=LIST_X+LIST_W-12, y, w=8, h=ROW`, 램프색(미플레이 어둡게).
  - 폴더행: KEY/레벨 배지 대신 `▶` + 폴더명, 우측 곡 수.
- 행 전체에 `SelectHot::Row(idx)` hot 등록.

난이도 색(`difficulty_name` 슬롯): BEGINNER `(80,220,120)` / NORMAL `(90,180,240)` / HYPER `(240,200,70)` / ANOTHER `(240,90,90)` / INSANE `(200,120,230)` / 미지정 `GRAY`.

## 3. 밀도 데이터 (`ChartDetail` 확장 — beatoraja 포팅)

beatoraja `SongInformation(BMSModel)` 정밀 포팅. 모델 타임라인에서:

- `bins = last_time_ms/1000 + 2` 개의 1초 빈. 각 빈 7카테고리 `[s_lnhead,s_lnbody,s_normal,k_lnhead,k_lnbody,k_normal,mine]`.
  - LN head(스타트): head초 `[s_lnhead/k_lnhead]++`, 그리고 head초..pair초 모든 빈 `[s_lnbody/k_lnbody]++` 후 head초만 body `--`(이중카운트 보정). (beatoraja와 동일 순서)
  - normal: `[s_normal/k_normal]++`. mine: `[6]++`.
  - `#LNMODE==1` 또는 (LNMODE==0 & LNTYPE==LN)면 LN end는 카운트 제외.
- **per-sec total** `bin_sum = sum(cat0..5)`(mine 제외) → 히스토그램 막대값.
- **peak_density** = `max(bin_sum)`.
- **avg_density** = `bd=total_notes/bins/4` 이상인 빈만 평균(`sum/count`).
- **end_density**: `border = total_notes*(1 - 100/total_value)` 누적 도달 빈 `borderpos`부터, `d=min(5, bins-borderpos-1)` 슬라이딩창의 `max(window_sum/d)`. `total_value`는 `#TOTAL`(없으면 모델 total).
- 저장: `ChartDetail { …기존, density: Vec<u32>(bin_sum), peak_density: f64, avg_density: f64, end_density: f64 }`.
- `count_playable_notes`와 별개로, `total_notes`(=normal+LN head, scratch 포함)는 위 카운팅에서 같이 산출.

검증: 발광1 라이브러리 한 차트에서 peak/avg/end가 양수·합리적 범위(예: peak 10~30 notes/sec)인지 테스트.

## 4. 상세 패널 (632,60 / 616×600)

상단→하단 흐름. 패널 배경은 커버 영역을 피해 섹션별 `fill_rect`.

1. **상단 액센트 바**: `(DETAIL_X,60,616,6)` mode_color.
2. **헤더(커버+제목)** y 74..250:
   - **커버 정사각** `160×160` at `(DETAIL_X+16, 78)`. `render_select`는 테두리 `(90,96,120)` 2px + 내부 placeholder(없음="NO IMAGE" / 있음=빈칸, BGA가 비침). main.rs가 `set_bga(cover, (DETAIL_X+16,78,160,160))`.
   - 제목블록(커버 우측 `x=DETAIL_X+192`): 타이틀 s2.0(y84, fit), 부제 s1.2(y116), 아티스트 s1.3 시안(y140), 장르·제작자 s1.1 GRAY(y162).
   - **배지 행**(y 196): KEY 배지(mode_color, mode_short, s1.4) + 레벨 배지(difficulty색, "Lv {level}") + 난이도명(difficulty_name, s1.2). 우측 `draw_text_right` PLAYS 수.
3. **스탯 그리드** y 262..356, 2열×3행, 행높이 32: BPM(범위 `a–b`)·NOTES(`n (mLN)`)·LENGTH(`m:ss`)·JUDGE(`#RANK명 %`)·TOTAL(`AUTO`/값)·DIFFICULTY명. 라벨 GRAY s1.0 + 값 흰 s1.5. 상단 구분선.
4. **밀도 그래프** y 366..470:
   - 라벨 "DENSITY" s1.3(y366) 좌, 우측 `draw_text_right` `PEAK {peak} / AVG {avg} / END {end}` (notes/sec, 정수).
   - 그래프 박스 `(DETAIL_X+16, 384, 584, 80)` 배경 `(12,12,18)`. 밀도 `Vec<u32>`를 막대로: 막대폭 `= box_w/bins`(최소 1px), 높이 `= 80*(bin_sum/peak_scale)`, `peak_scale=max(peak,1)`. 색=강도(저 `(60,140,200)`→고 `(240,90,90)` 보간) 또는 키/스크 2색. 10초 그리드 세로선 GRAY 약하게.
5. **RECORDS** y 480..654:
   - "RECORDS" s1.4(y480) + 우측 `{n} PLAYS`. CLEAR/FC RATE는 (로컬엔 시도횟수만 → "CLEAR {n}/{plays}" 비율) 우측 보조.
   - 무기록: "NO PLAY YET".
   - 베스트 램프 바 `(DETAIL_X+16, 502, 584, 28)` 램프색 + 라벨(검정) + `EX {best}` 우. score_graph면 DJ랭크+`draw_rank_bar`(y 536).
   - 최근 기록 행(newest first) `rh=32`, 바닥(654)까지: 날짜·램프라벨·EX(+랭크)·BP(+Δ). 각 행 `SelectHot::Record(ri)`.
6. **폴더 포커스 시**: 액센트+「FOLDER」+폴더명 + 곡수만.
7. **기록 모달**: 기존 로직 유지(전체화면 dim+패널). `SelectHot::ModalReplay/ModalClose`.

## 5. `SelectView` / `SelectHot` (rbms-render)

```
pub enum SelectHot { Row(usize), Record(usize), ModalReplay, ModalClose }

pub struct SelectRow { pub folder: bool, pub title: String, pub mode_short: &'static str,
    pub mode_color: Color, pub level: String, pub difficulty_color: Color,
    pub lamp: Color, pub folder_count: Option<usize> }

pub struct DensityView { pub bins: Vec<u32>, pub peak: f64, pub avg: f64, pub end: f64 }

pub struct StatCell { pub label: &'static str, pub value: String }

pub struct RecordRowView { pub when: String, pub lamp: Color, pub lamp_label: &'static str,
    pub ex: u32, pub max_ex: u32, pub bp: u32, pub trend: Option<String> }

pub struct DetailView { // Song 포커스
    pub accent: Color, pub title: String, pub subtitle: String, pub artist: String,
    pub genre_maker: String, pub mode_short: &'static str, pub mode_color: Color,
    pub level: String, pub difficulty_color: Color, pub difficulty_name: &'static str,
    pub cover: CoverState, pub stats: Vec<StatCell>, pub density: Option<DensityView>,
    pub records: RecordsView, pub plays: usize }

pub enum CoverState { None, Present } // rect는 render_select 내부 상수
pub enum SelectDetail { Song(DetailView), Folder { label: String, count: usize }, Empty }

pub struct RecordsView { pub best: Option<RecordRowView>, pub rank_bar: Option<(u32,u32)>, pub recent: Vec<RecordRowView> }

pub struct SelectModal { /* 기록 모달 표시 데이터 */ }

pub struct SelectView { pub rows: Vec<SelectRow>, pub sel: usize, pub start: usize,
    pub header: String, pub n: usize, pub m: usize, pub guide: &'static str,
    pub detail: SelectDetail, pub modal: Option<SelectModal>, pub score_graph: bool }
```

- `render_select`는 hot을 `Vec<(Rect,SelectHot)>`로 반환. main.rs가 `Hot::SelectRow/RecordRow/ModalReplay/ModalClose`로 매핑(modal_open이면 행 hot 생략).
- `cover_rect()`는 상수 `Rect(DETAIL_X+16,78,160,160)` — main.rs가 동일 상수로 `set_bga`.

## 6. 색 팔레트 추가

- 타이틀 노랑 `(240,232,150)` / 비포커스 `(190,186,150)`.
- 포커스 테두리 시안 `(120,200,240)`.
- 밀도 저→고 `(60,140,200)`→`(240,90,90)`.
- 난이도 색 §2.

## 7. 검증

- `cargo test --workspace`(파서 #BANNER/#PREVIEW·밀도 peak/avg/end).
- 헤드리스: `examples/render_select.rs`(rbms-render) → PPM→PNG로 레이아웃 확인(커버는 placeholder).
- 라이브: `./start.sh`로 커버 BGA·포커스 연출·밀도그래프 확인.

## 8. 구현 체크리스트 (완료 2026-05-31)

- [x] a. 파서 `#BANNER`/`#PREVIEW` + 테스트
- [x] b. `SongEntry`에 stagefile/banner/preview + scan 반영
- [x] c. 밀도 포팅 → **`rbms_chart::note_density`**(peak/avg/end + bins, 테스트 4개) + `ChartDetail` 확장
- [x] d. `rbms-render::render_select` + `SelectView`/`SelectHot` + 예제(헤드리스 PNG)
- [x] e. main.rs: `build_select_view` 조립(+`SelectKey` 캐시) + hot 매핑 + 커버 디코드/`set_bga`
- [x] (리뷰) 2라운드 적대적 멀티에이전트 리뷰 — 1R 16건 반영·2R 0건
- [ ] f. (후속) `#PREVIEW` 재생(데이터 준비완료), KEY BOMB, 스킨 데이터화 결과/메뉴 → `ROADMAP.md`
</content>
</invoke>
