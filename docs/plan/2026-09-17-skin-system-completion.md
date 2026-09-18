# 스킨 시스템 완성과 기본 스킨 전면 제작 사양

상태: K2 확정(2026-09-17), K3~K5 구현·검증 완료(2026-09-17, 미커밋). 최종 갱신 2026-09-18 — 구현 결과와 다르던 §3·§7·§9.1·§9.6·§11 을 코드에 맞춰 정정했다. 실제 진행은 `docs/history/2026-09-17-skin-system-completion.md`. 이 문서는 `docs/plan/2026-09-17-skin-object-spec.md`(개별 객체 계약)와 `docs/plan/2026-09-17-skin-visual-correction.md`(화면 보정)를 흡수하는 상위 사양이며, 구현은 이 문서의 파일 소유권과 완료 조건을 따른다.

입력: 사용자가 제공한 외부 스킨 묶음(구조 분석 입력, 저장소 복사 금지)과 화면 캡처 11장, 조사 보고서 R1(엔진 격차)·R2(앱 배선)·R3(외부 묶음 레이아웃). 레퍼런스 구현의 JSON 스킨 로더 원본은 의미 확인에만 쓰고 이름은 저장소에 적지 않는다.

## 1. 목표와 완료 조건

1. 기본 스킨이 화면당 그림 두 장이 아니라 **요소별 객체 문서**로 구성되고, 캡처 1·2번의 결함(장식선이 글자·노트를 가로지름, 옵션 패널이 상세를 가림)이 재현되지 않는다.
2. 문서 모델에 선언돼 있으나 그리지 못하던 객체(note·gauge·judge·songlist·hiddenCover·liftCover·gaugegraph·judgegraph·bpmgraph·timingdistributiongraph·timingvisualizer·hiterrorvisualizer)가 실제로 컴파일·렌더된다.
3. 문서가 네이티브 출력 묶음을 화면별로 대체 선언할 수 있고, 대체가 불완전하면 네이티브가 남는다(기존 결과 화면 계약을 select·play로 확장).
4. 커스터마이즈(옵션·파일·오프셋)가 번들 단위로 공유되고 세대 이동 시 유실되지 않는다. 시스템 사운드 세트를 번들이 제공한다.
5. 5/7/9/10/14키·선택·옵션·결정·결과의 헤드리스 캡처와 실제 창 확인으로 배치·가독성·입력 일치를 검증하고 `cargo fmt --all --check`·`cargo clippy --workspace --all-targets --all-features -- -D warnings`·관련 테스트를 통과한다.

완료로 판정하지 않는 것: 곡 선택 BGM 루프, 캐릭터(pmchara)·연습(practice)·스킨 미리보기 객체, 문서 객체의 클릭 이벤트 디스패치(`click`/`act`), 외부 CSV 스킨 형식 직접 로드, 비디오 BGA. §12에 후속으로 기록한다.

## 2. 결정

| 번호 | 결정 | 근거 |
| --- | --- | --- |
| D1 | 새 기본 번들은 **3세대 `steel-neon-v3`**, 프리셋 라벨은 `STEEL NEON` 유지 | 설치 코드가 세대 기반이고 사용자에게 보이는 라벨 변경·설정 마이그레이션을 피한다. 세대 처리는 v1/v2 하드코딩을 세대 목록으로 일반화한다(R2 §2a) |
| D2 | 반복 객체(note·judge·songlist)의 중첩 `Vec<Destination>`은 **로더가 로드 시 트랙으로 조립**해 `LoadedSkin`에 보관한다 | 렌더 크레이트가 `build_track`을 재호출하지 않고, Lua 컴파일·경고가 한 곳에 남는다(R1 §Q3 (a) 선결 과제 1안) |
| D3 | 행·시계열 상태는 `SkinStateSource`를 바꾸지 않고 **`SkinFrame.extra`**(화면별 타입 참조)로 넘긴다 | 스칼라 트레이트를 유지해 기존 문서·테스트를 깨지 않는다(R1 차단 요소 2) |
| D4 | 타이머는 계속 `TimerState`가 원천이며, **레인별 KEYON/KEYOFF/BOMB/HOLD와 결과 타이머 드라이버**를 추가한다. `SkinStateSource::timer`는 `None` 유지 | 레퍼런스도 타이머 테이블을 별도로 굴린다. 드라이버 추가만으로 문서 애니메이션이 동작한다 |
| D5 | 플레이 문서에 `note` 객체가 있으면 **레인 사각형의 단일 원천은 문서의 `note.dst`**이고 네이티브 `Skin`의 x/w/top_y/judge_y가 그 값으로 재구성된다. 없으면 종전대로 RON | Phase E 사양 §(753행) "레인 좌표는 NoteSet.dst가 대체"와 일치. 1P/2P 사이드 전환이 문서 분기만으로 끝난다 |
| D6 | 네이티브 대체 선언을 **최상위 `replace: [..]`**로 일반화하고 `result.replace`는 하위 호환으로 유지 | 결과 전용 하드코딩(R1 §2.4)을 화면 공통 표로 바꾼다 |
| D7 | 히트 사각형은 songlist 행 기하와 **`hotspot` 선언**(객체 id → 네이티브 동작)에서 산출한다 | 문서가 커서만 받고 클릭을 만들지 못하는 제약(R2 §1.1)을 최소 변경으로 푼다 |
| D8 | 커스터마이즈 공유는 **번들 스코프 저장**(R2 안 A). `property`/`filepath`/`offset`에 `scope: 'bundle'`을 허용하고 SKIN 탭이 번들 행을 화면과 무관하게 노출한다 | 값 공유가 목표이며 로더·렌더 무변경. 세대 이동 시 `custom` 키 이동도 같은 변경으로 고친다(R2 §3.3-5) |
| D9 | 시스템 사운드는 `audio.sound_folder`가 비어 있고 프리셋이 `STEEL NEON`이면 활성 번들의 `sound/`를 읽는다. 스템 22개는 **절차 생성 WAV**로 제공 | 외부 음원 복사 금지. 폴더 규약은 기존 `syssound` 그대로(R2 §2c) |
| D10 | 선택 화면 옵션 패널은 select 문서가 `replace: ['options']`로 대체한다. 행 값은 사설 문자열 id(20101~)로 노출 | 참고 화면 9~11의 옵션 패널이 선택 화면의 일부이며 레퍼런스 property `OPTION_PANEL1`(21)로 열림을 표현한다 |

## 3. 번들 구조

```
assets/skins/steel-neon-v3/
  select.json5  decide.json5  result.json5
  play-5k.json5 play-7k.json5 play-9k.json5 play-10k.json5 play-14k.json5 play-24k.json5(준비)
  shared/objects-play.json5      # 단일 플레이 문서(5/7/9키)가 include 로 공유하는 객체. PLAY SIDE × GRAPH POSITION 4조합을 op 로 분기
  shared/judge-sp.json5          # 단일 플레이 판정 팝업(1P/2P 분기)
  play.ron  play-dual.ron  theme.ron
  palette.json
  images/*.png                   # 생성 스크립트 산출 27장(스프라이트 시트·숫자·배경·프레임 7장)
  images/{notes,covers,select,decide}/*.png   # filepath 슬롯 후보
  sound/*.wav                    # 22 스템, 생성 스크립트 산출
  tools/generate-assets.py  tools/generate-sounds.py
```

- 문서는 1280×720 기준. 이미지는 `palette.json`과 스크립트로 재생성 가능해야 하며 PNG 직접 교체도 허용한다.
- `include`는 로더가 이미 지원한다(`branch::transform`의 include 콜백). 공유 객체 파일은 `type`이 없는 부분 문서다.
- `shared/`·`sound/`·`tools/`·`images/`는 배포 목록(`assets.rs` 번들 파일 목록)에 전부 포함한다.

## 4. 문서 계약 확장 (`crates/rbms-skin`)

### 4.1 중첩 트랙 조립 (D2)

- `LoadedSkin`에 `nested: NestedTracks` 추가. `NestedTracks { note_group: Vec<NamedTrack>, note_bpm, note_stop, note_time, judge: BTreeMap<String /*judge id*/, JudgeTracks { images: Vec<NamedTrack>, numbers: Vec<NamedTrack> }>, songlist: Option<SongListTracks { listoff, liston, text, level, lamp, playerlamp, rivallamp, trophy, label: Vec<NamedTrack>, graph: Option<NamedTrack> }> }`.
- `load_skin`이 최상위 destination 조립 직후 같은 `build_track(…, relative)`으로 채운다. 판정 numbers는 레퍼런스가 `relative` 숫자로 다루므로 `relative=true`. 실패는 경고로 누적하고 해당 항목만 비운다.
- `NoteSet.dst`(레인별 `Animation`)는 트랙이 아니라 사각형이므로 그대로 둔다. `NoteSet.size`·`expansionrate`·`dst2`는 모델에 이미 있다.

### 4.2 대체·핫스팟·스코프 (D6·D7·D8)

- `SkinDef.replace: Vec<String>` 추가(기본 빈 배열). `result.replace`가 있으면 합집합으로 해석한다.
- `SkinDef.hotspot: Vec<HotspotDef { id: String, action: String }>` 추가. `action` 값: `search`·`sort`·`folders`·`tables`·`records`·`settings`·`modal-replay`·`modal-close`. 알 수 없는 값은 경고 후 무시.
- `PropertyDef`·`Filepath`·`OffsetDef`에 `scope: Option<String>` 추가. `'bundle'`이면 번들 공유, 그 외·없음은 문서 전용. 헤더(`load_header`)에도 노출한다.
- 모르는 필드는 종전대로 무시하므로 외부 문서 호환은 유지된다.

### 4.3 사설 속성 id

레퍼런스 id 공간과 겹치지 않는 20000 대역을 `rbms-render/src/skin_render/state.rs`에 모은다(기존 `RESULT_TEXT_*` 20001~20011 유지).

| id | 종류 | 의미 |
| --- | --- | --- |
| 20101~20111 | STRING | 옵션 행 0~10의 라벨 |
| 20121~20131 | STRING | 옵션 행 0~10의 현재 값 |
| 20141~20151 | OPTION | 옵션 행 0~10이 포커스 |
| 20201 | STRING | 플레이 목표 이름 |
| 20202 | STRING | 목표 대비 EX 차이 문자열(부호 포함) |
| 20203 | STRING | 선택 화면 조작 힌트 / 20204 결과 힌트 / 20205 결과 화면 IR 상태(여러 줄을 한 줄로) |
| 20301~20306 | STRING | 선택 상세 통계 셀 0~5의 `라벨 값`(NOTES/TOTAL/TIME/PLAY/JUDGE/LN 등 `DetailView.stats` 순서) |
| 20311~20315 | STRING | 선택 곡의 기록: `EX best / max`, `BP n`, `PLAYS n CLEARS n`, 램프 이름, 기록 시각 |

옵션 패널 열림은 레퍼런스 `OPTION_PANEL1`(21)·타이머 `PANEL1_ON`(21)/`PANEL1_OFF`(31)로 답한다.

## 5. 렌더러 객체 (`crates/rbms-render/src/skin_render`)

새 파일로 분리해 병렬 구현 충돌을 막는다. `object.rs`는 `Body` 변형·`SkinObjectKind`·`build_body` 분기만 늘리고 본체 컴파일·그리기는 각 파일이 맡는다. `draw.rs`의 `draw_object` match도 분기만 추가한다.

| 객체 | 파일 | 원천·의미(레퍼런스 로더 기준) | 완료 조건 |
| --- | --- | --- | --- |
| `note` | `notes.rs` | 레인별 이미지 배열(note/lnstart/lnend/lnbody/lnbody_active/mine/hidden/processed)을 `image_sprite`로 해석. 레인 사각형은 `note.dst[i]`, 노트 높이는 `size[i]` 또는 스프라이트 셀 높이. y는 `rbms_chart::scroll::{visible_offsets,constant_offsets}`를 네이티브와 같은 인자로 호출해 계산(`playfield.rs:76-80`과 동일 값). LN은 head/body/tail 3분할, 활성 LN은 `lnbody_active`. `group/bpm/stop/time` 트랙은 소절선·BPM/STOP 라인 | 7키 문서에서 노트·LN·지뢰가 네이티브 필드와 같은 프레임에 같은 y로 그려지고 `hidden` 이미지가 처리된 노트에 쓰임 |
| `gauge` | `gauge.rs` | `nodes` 이미지 id(4·8·12·36개)와 레퍼런스 인덱스 맵(4→{0,4,6,10…}, 8, 12)으로 36종 셀 표 구성, `parts`(기본 50)로 나눠 현재값·`range` 깜빡임·`cycle` 애니메이션. 게이지 종류 인덱스는 `PlayObjectState.gauge_kind` | 게이지 값 74%에서 37/50칸이 켜지고 종류(ASSIST/EASY/NORMAL/HARD/EXHARD/HAZARD)별 셀 열이 바뀜 |
| `judge` | `judge.rs` | `index`별 팝업: `images[k]`(판정 k 이미지, 레퍼런스는 k=0 PG…5 MISS 순)와 `numbers[k]`(콤보, `relative` 숫자, x를 `w*digit/2`만큼 왼쪽으로 보정). `shift`가 true면 콤보 자릿수만큼 이미지를 옮긴다. 표시 게이트는 `JUDGE_1P` 타이머와 `last_judge`. rbms 확장: `images[k]`의 id가 `text` 객체를 가리켜도 된다(문구는 비트맵이 아니라 글꼴로) | 판정 변화마다 해당 이미지·콤보가 `judgetimer` 동안 표시 |
| `songlist` | `songlist.rs` | `listoff/liston[i]`는 슬롯 i의 바 배경(`imageset`을 레퍼런스처럼 허용), `center`가 선택 곡 슬롯, `text/level/lamp/playerlamp/rivallamp/trophy/label[i]`는 슬롯 i의 부속 객체. 슬롯 i의 곡 = `rows[sel - center + i]`. `clickable` 슬롯의 배치 사각형을 히트 사각형으로 반환 | 5행 목록에서 선택 행이 `center` 슬롯에 오고, 마우스 클릭이 슬롯 사각형과 일치 |
| `hiddenCover`/`liftCover` | `covers.rs` | `image_sprite` 재사용. 레퍼런스 의미 그대로 `hiddenCover` = HIDDEN+ 하단 밴드(`LaneShade.hidden`), `liftCover` = 리프트가 판정선 아래 남긴 밴드. **SUDDEN+ 상단 커버는 전용 객체가 없고**, 레퍼런스처럼 `offset: 4`(`OFFSET_LANECOVER`)를 단 일반 `image`로 그린다 — `PlayViewState`가 `OFFSET_LIFT`(3)/`OFFSET_LANECOVER`(4)/`OFFSET_HIDDEN_COVER`(5)를 `LaneShade`에서 답한다(K3 리뷰 반영) | HIDDEN+ 30%에서 필드 하단 30%가 덮이고, SUDDEN+ 30%에서 `offset: 4` 이미지가 상단 30%를 덮음 |
| `gaugegraph` | `graphs.rs` | `ResultSeriesState.gauge_series`를 시간축으로 폴리라인(세로 1px 막대열). 색은 14종 문자열 → `parse_hex_color` | 결과 문서에서 네이티브 추이와 같은 형상 |
| `judgegraph` | `graphs.rs` | `judge_dist`를 노트 진행 구간별 누적 막대(레퍼런스 type 0/1/2 중 0·1 구현, 2는 0으로 폴백·경고) | 판정 6종 막대가 팔레트 색으로 표시 |
| `bpmgraph` | `graphs.rs` | 차트 타임라인의 BPM/STOP 변화를 선분으로. `PlayObjectState.timelines` 또는 `ResultSeriesState.bpm_points` | BPM 변화 차트에서 단계가 보임 |
| `timingdistributiongraph` | `graphs.rs` | `timing_hist` 히스토그램 + 평균·편차선 | 결과 타이밍 분포가 표시 |
| `densitygraph`(rbms 확장) | `graphs.rs` | 선택 상세의 노트 밀도(`DetailView.density: DensityView { bins, peak, avg, end }`)를 막대 히스토그램으로. 필드: `id`, `barColor`, `peakColor`, `lineWidth` | 5행 목록 선택 곡의 밀도가 표시되고 `bins`가 비면 그리지 않음 |
| `timingvisualizer`·`hiterrorvisualizer` | `graphs.rs` | 최근 판정 오차 목록(`PlayObjectState.recent_hits: &[(i64 /*ms 오차*/, u8 /*judge*/)]`, 최대 `windowLength`)을 눈금 위 점으로. 판정 폭 자 눈금은 현재 판정 창 | 플레이 중 FAST/SLOW 점이 좌우로 찍힘 |

공통: `parse_hex_color(&str) -> Option<Color>`(`RRGGBB`/`RRGGBBAA`)는 `skin_render/color.rs`에 둔다. 선은 `fill_rect`로 그린다(렌더러에 선 프리미티브가 없다. R1 확인). 새 객체는 `SkinObjectKind`에 각각 변형을 추가해 `count_of`가 세게 한다.

## 6. 상태 공급과 타이머

### 6.1 `SkinFrame.extra` (D3)

```rust
pub enum FrameExtra<'a> { None, Play(&'a PlayObjectState<'a>), Select(&'a SelectListState<'a>), Result(&'a ResultSeriesState<'a>) }
pub struct PlayObjectState<'a> { pub field: &'a Skin, pub playfield: &'a PlayfieldView<'a>, pub shade: LaneShade, pub gauge_kind: usize, pub bomb: &'a [(i64, u8)], pub keys_down: &'a [bool], pub recent_hits: &'a [(i64, u8)] }
pub struct SelectListState<'a> { pub rows: &'a [SelectRow], pub sel: usize, pub detail: &'a SelectDetail, pub options: Option<&'a OptionsRows<'a>> }
pub struct ResultSeriesState<'a> { pub gauge_series: &'a [f32], pub timing_hist: &'a [u32], pub judge_dist: &'a [u32; 6], pub bpm_points: &'a [(f32 /*0..1 진행*/, f64 /*bpm*/)] }
```

- `SkinDraw`가 `extra`를 받아 `SkinFrame`에 넣는다. 기존 호출자는 `FrameExtra::None`.
- `PlayViewState`는 `gauge_kind`·`target`(`HudPace`)·`keys_down`을 추가로 답한다(`OPTION_1P_*` 게이지 종류 id 42/43/1046 등 레퍼런스 id에 매핑, `NUMBER_TARGET_SCORE`·`NUMBER_DIFF_TARGETSCORE`).
- 옵션 행 값(`OptionsRows { labels: [&str; 11], values: [String; 11], focused: usize, open: bool }`)은 앱의 `app_options`가 descriptor에서 만들어 `SelectListState.options`로 넘긴다. 문자열 id 20101~는 `SelectViewState`가 이 값으로 답한다.
- 상태가 답하는 레퍼런스 id(구현 결과): 플레이 — 판정 수·콤보·EX·게이지·하이스피드·BPM(현재/최소/최대/주)·그린/화이트 넘버·경과/남은 시간·FAST/SLOW 합·목표 EX/차이/비율·최고 기록 비율·레벨·제목/아티스트·1P/2P 판정·EARLY/LATE·NOW 등급(340~347)·레인커버 오프셋(3/4/5). 선택 — 제목/부제/아티스트/장르·레벨·폴더/검색어·통계 6셀·기록 5줄·최고 기록 비율·힌트·옵션 행. 결과 — 점수/판정/목표/게이지·등급(300~307)·제목/아티스트·결과 텍스트 20001~20011·힌트·IR. 결정 — 진행·제목·부제/아티스트/장르·레벨.

### 6.2 타이머 드라이버 (D4)

- `PlayTimers::update`에 레인별 `KEYON_1P_KEY{n}`/`KEYOFF`(스크래치 포함), `BOMB_1P_*`(`session.bomb()` 이벤트), `HOLD_1P_*`(LN 활성) 스위치를 추가한다. 이중 필드(10/14키)는 2P 대역(`KEYON_2P_*` 등)을 오른쪽 필드 레인에 매핑한다.
- `ResultTimers` 신설: 결과 진입 시 `RESULTGRAPH_BEGIN`(150) on, 1초 뒤 `RESULTGRAPH_END`(151), 진입 시 최고 기록을 넘긴 런이면 `RESULT_UPDATESCORE`(152) on(구현 결정: rbms에는 등급 연출이 없어 레퍼런스의 "점수 표시 시점" 직역이 항상 on이 되므로 신기록 의미로 둔다).
- 옵션 패널 열림/닫힘 시 `PANEL1_ON`(21)/`PANEL1_OFF`(31) 토글은 `app_options`가 켠다.

## 7. 네이티브 대체 단위 (D6)

`replace` 이름 → 필요한 객체 id(전부 있어야 대체). 하나라도 없으면 경고 1회 + 네이티브 유지(기존 계약).

| 화면 | 이름 | 필요한 객체 id | 숨기는 네이티브 |
| --- | --- | --- | --- |
| play | `field` | `note`(문서 `note.id`) | 레인 배경·노트·판정선·키빔·소절선(`render_playfield_*`) |
| play | `gauge` | `gauge` id + `play-gauge-value` | 게이지 바·수치 |
| play | `judge` | `judge` 객체 1개 이상 | 콤보·판정 문자·FAST/SLOW 문자 |
| play | `score` | `play-ex`, `play-best`, `play-green` | 좌상단 EX/BEST/GREEN/WHITE 텍스트 |
| play | `counts` | `play-count-{pg,gr,gd,bd,pr,ms}`, `play-fast`, `play-slow` | 판정 카운터 패널 |
| play | `graph` | `play-graph-ex`, `play-graph-best`, `play-graph-target` | 페이스메이커 그래프 |
| play | `cover` | `hiddenCover` 또는 `liftCover` 1개 이상 | `render_lane_cover` |
| play | `frame` | (없음) | `layout_frame` 외곽선·BGA 테두리 |
| select | `list` | `songlist` id | 곡 행 목록(히트 사각형은 songlist가 대신 제공) |
| select | `detail` | `select-title`, `select-artist`, `select-genre`, `select-level`, `select-stat-0`~`select-stat-5`, `select-density`, `select-record` | 상세 패널 전체 |
| select | `topbar` | `hotspot`에 6개 action 전부 | 상단 바·버튼(핫스팟이 대신 제공) |
| select | `options` | `option-row-{0..10}-label`, `option-row-{0..10}-value`, `options-panel` | 옵션 오버레이 패널 |
| result | `score` | `result-score-label`, `result-score`, `result-combo-label`, `result-combo`, `result-notes-label`, `result-notes` | 점수·콤보·노트 수 표 |
| result | `clear` | `result-clear` | 클리어 문자 |
| result | `judgment` | `result-judge-{perfect,great,good,bad,poor,miss}` | 판정 6행 |
| result | `target` | `result-target` | 목표 행 |
| result | `grade` | `result-rank`, `result-rate`, `result-rankbar` | 등급 문자·비율·등급 바·vs BEST/PREV |
| result | `graphs` | `gaugegraph`·`judgegraph`·`timingdistributiongraph` 각 1개 | 하단 추이 3패널 |
| result | `title` | `result-title` | 상단 바 제목·모드 |
| result | `hint` | `result-hint` | 하단 조작 힌트 |
| result | `ir` | `result-ir` | IR 상태 줄(앱이 그리던 것) |

- `hot` 산출: select layered 경로에서 `songlist` 슬롯 사각형과 `hotspot` 객체의 해석 사각형을 `Vec<(Rect, SelectHot)>`에 합친다. 네이티브 목록이 대체되면 네이티브 행 사각형은 만들지 않는다.
- play `field` 대체 시 D5에 따라 `Skin` 레인 기하가 문서에서 오므로 HUD·커버·키봄도 같은 좌표를 쓴다.

## 8. 설정·설치·사운드

- `assets.rs`: 세대 목록 `BUNDLE_GENERATIONS: &[BundleGeneration { directory, files, documents }]`(v1·v2·v3). 설치는 현재 세대만 새로 만들고 이전 세대는 존재할 때만 재설치. 자동 이동 규칙(원본 해시 일치·프리셋 조합)은 기존과 같되 **`custom` 키를 새 경로로 함께 이동**한다.
- `rbms-config` `SkinOptions`: `shared: BTreeMap<String /*번들 디렉터리*/, SkinCustomisation>` 추가. `user_config(path)`는 번들 공유 → 문서 전용 순으로 병합. 번들 키는 스킨 폴더 아래 첫 디렉터리 이름.
- SKIN 탭: 번들 스코프 행을 `SCREEN` 행과 무관하게 `LOADED` 아래 `BUNDLE > 이름`으로 노출. 변경 시 해당 번들의 모든 선택 문서를 `stale`로 표시.
- 사운드: `sound_folder_path`가 `None`이면 활성 번들 `sound/`를 후보로. 프리셋·문서 변경 경로(`SettingId::Skin`/`SkinDocument`)에서 `reload_system_sounds()` 호출.
- 테마·RON 경로 결정은 종전과 같다(`active_theme_path`·`installed_play_skin_path`).

## 9. 기본 스킨 사양

작성 원칙: 1280×720(y-down 기준으로 아래 표기, 문서 작성 시 y-up 변환). 외부 묶음의 레인 폭 비율·행 간격·BGA 비율(R3)과 참고 화면의 정보 계층을 따르되 rbms가 가진 상태만 표시한다. 노트·판정·수치 위치를 배경 그림에 굽지 않는다. 장식선은 정보 사각형 안쪽을 지나지 않는다. 팔레트는 v2 `palette.json`을 계승한다(ink `#070912`, night `#11182D`, panel `#1D2340`, metal `#5A6684`, metalLight `#A9B7D4`, violet `#E15BD7`, cyan `#69F1E4`, amber `#FFD36A`, white `#F4FBFF`, blue `#6D96FF`, rose `#FF7C9A`).

### 9.1 자산 계약 (`tools/generate-assets.py` 산출, `images/`)

문서 작성자와 생성 스크립트가 같은 표를 본다. 셀 좌표는 PNG 안 픽셀(x,y,w,h).

| 파일 | 내용 | 셀 규약 |
| --- | --- | --- |
| `notes.png` | 노트 스프라이트. 행 = 레인 종류(0 백건 white, 1 흑건 blue, 2 스크래치 rose, 3 팝 노랑, 4 팝 초록, 5 팝 빨강, 6 지뢰 amber), 열 = 상태(0 일반, 1 LN 시작, 2 LN 본체, 3 LN 끝, 4 LN 활성 본체, 5 처리됨/hidden 반투명). 셀 64×24, 행 피치 24 | `x=col*64, y=row*24` |
| `digits-s.png` / `digits-m.png` / `digits-l.png` | 7세그먼트풍 숫자 0~9, 10번 셀 = 선행 빈자리용 공백(렌더러의 11셀 정수 규약: 11번째 셀이 대체 0). 흰색(문서에서 색 지정). 셀 14×20 / 20×28 / 32×44, 시트 154×20 / 220×28 / 352×44 | 가로 11셀 1행, `value.divx: 11` |
| `digits-f.png` | 소수용 11×2 셀 20×28(220×56): 행 0 = 0~9 + 10번 셀 소수점 `.`(양수), 행 1 = 같은 구성 rose 색(음수 반쪽). 렌더러의 22셀 소수 규약 | `floatvalue.divx: 11, divy: 2` |
| `ui.png` | 공용 UI 조각 256×256: (0,0,64,64) 패널 채움 night α224 · (64,0,64,64) 패널 밝은 panel · (128,0,64,32) 헤더 바 metal · (192,0,64,32) 헤더 바 violet · (0,64,64,32) 버튼 normal · (64,64,64,32) 버튼 active cyan · (128,64,32,32) 배지 채움 · (0,96,16,16)×10 램프 색(NOPLAY/FAILED/ASSIST/EASY/NORMAL/HARD/EXHARD/FC/PERFECT/MAX) · (0,112,64,8) 판정선 cyan · (64,112,64,8) 소절선 metal · (0,120,64,64) 키빔(세로 그라데이션 violet α) · (64,120,48,48) 턴테이블 원 · (112,120,24,40) 백건 위젯 off · (136,120,24,40) 백건 on · (160,120,20,40) 흑건 off · (180,120,20,40) 흑건 on · (0,184,32,16)×4 게이지 노드(0 켜짐 groove cyan, 1 켜짐 hard rose, 2 꺼짐 groove, 3 꺼짐 hard) · (128,184,64,64) 선택 곡 바 ON · (192,184,64,64) 곡 바 OFF · (0,200,128,8) 구분선 | 문서가 `image`로 잘라 쓴다 |
| `covers.png` | 레인커버 2종(SOLID 불투명 night, GRADIENT 위 불투명→아래 투명) 각 320×480 | `filepath` 슬롯 후보 `images/covers/*.png`로도 복사 |
| `select-bg.png`, `decide-bg.png`, `play-bg.png`, `result-bg-{aaa,aa,a,clear,failed}.png` | 1280×720 배경(그라데이션+별빛/도시 실루엣 추상). 정보 영역엔 장식 없음 | 배경 레이어 |
| `frame-sp.png`, `frame-sp-2p.png`, `frame-sp-near.png`, `frame-sp-2p-near.png`, `frame-dp.png`, `frame-select.png`, `frame-result.png` | 화면 외곽·패널 경계선만 있는 전경(투명 PNG). 사각형 안쪽은 비움. 단일 플레이는 PLAY SIDE × GRAPH POSITION 조합별 4장, `frame-dp.png` 는 14키 좌표(10키 전용은 후속) | 전경 레이어 |

부호가 있는 차이 값(목표 대비 EX 등)은 사설 문자열 id(20202 등)의 `text`로 그린다. 판정 문구(PGREAT/GREAT/GOOD/BAD/POOR/MISS·FAST/SLOW)와 라벨은 비트맵이 아니라 `text` 객체로 그린다. rbms 확장: `judge.images[k]`는 `image` 또는 `text` id를 가리킬 수 있다(§5 judge 구현 시 반영).

### 9.2 곡 선택 (`select.json5`, `replace: ['list','detail','topbar','options']`)

| 영역 | 사각형 | 객체 |
| --- | --- | --- |
| 상단 바 | (0,0,1280,40) | `select-logo` 텍스트 "MUSIC SELECT"(x 24,y 8,h 24), 모드 배지(x 200), 폴더 경로 `STRING_DIRECTORY`(x 300, 가운데 정렬 x 640), SORT `select-sort`(x 900), 곡 수 |
| 곡 목록 | (700,60,556,600) | `songlist` id `song-list`: 슬롯 15개, 행 y=60+40i, h 40, `center: 7`, 선택 슬롯만 x 680 w 576. 슬롯당 `listoff/liston`(ui 곡 바), `lamp`(x+0,w 8), `level`(x+14,w 40 숫자 digits-s), `label`(모드 칩 x+58), `text`(제목 x+110,h 22), `playerlamp`(x+520 램프). `clickable: 0..14` |
| 상세 제목 | (24,52,640,150) | 난이도표·레벨 줄(y 56), `select-title`(y 80,h 44 `STRING_TITLE`), `select-subtitle`(y 128,h 20), `select-artist`(y 150,h 20), `select-genre`(y 172,h 18) |
| 커버 | (470,56,190,142) | `bga` 객체(선택 커버 텍스처) |
| 차트 데이터 | (24,212,640,124) | 헤더 "BMS DATA" · `select-density`(x 40,y 236,w 360,h 88 밀도 히스토그램: rbms 확장 `graph` 대신 전용 `densitygraph` 객체 — §5에 `graphs.rs` 추가) · 통계 셀 6개 `select-stat-0..5`(x 420~, 2열×3행, h 18) |
| 점수 데이터 | (24,352,336,130) | 헤더 "SCORE DATA" · 판정 6행(`select-judge-{pg..ms}` 라벨+값, 행 피치 18) · `select-record`(EX/MAX COMBO/PLAY COUNT/CLEAR 요약 4줄) · 랭크 바 `select-rankbar`(y 470,h 8, `graph`로 `RATE_BESTSCORE`) |
| 기록/IR | (380,352,284,130) | 헤더 "RECORDS" · 최근 기록 3행(`select-recent-{0..2}`) · IR 순위 요약 |
| 하단 바 | (0,660,1280,60) | 버튼 6개 `btn-search`…`btn-settings`(x 24+108i, y 676, 100×28, ui 버튼) + `hotspot` 6개, 우측 힌트 `select-hint`(20203) |
| 옵션 패널 | (24,200,656,460), `op: [21]` | `options-panel`(ui 패널 α255), 제목 "OPTIONS"(y 212), 행 11개 `option-row-{i}-label`(x 44, y 252+34i, h 20, 색: 포커스 amber `op:[20141+i]`) / `option-row-{i}-value`(우측 정렬 x 660), 포커스 바 `option-row-{i}-focus`(ui 버튼 active, `op:[20141+i]`), 하단 힌트 y 632. 목록·제목은 계속 보인다 |

### 9.3 결정 (`decide.json5`, `composition: replace`)

| 영역 | 사각형 | 객체 |
| --- | --- | --- |
| 배경 | (0,0,1280,720) | `decide-bg.png`(filepath 슬롯 `BACKGROUND`, 후보 `images/decide/*.png`), 좌측 페이드는 배경에 포함 |
| 곡 정보 | 좌측 x 100 | 장르(y 296,h 22) · 제목(y 322,h 56 `STRING_TITLE`) · 부제(y 382,h 22) · 아티스트(y 406,h 22) · 레벨 숫자 digits-l(x 130,y 448) + 난이도명(x 200,y 470) |
| 띠 | (0,486,1280,40) | ui 헤더 바 violet α160, 우측 "NOW LOADING"/"READY" (`op:[OPTION_NOW_LOADING]`/`[OPTION_LOADED]`) |
| 진행 | (100,620,1080,6) | `graph`(`RATE_LOAD_PROGRESS`, cyan) + 배경 metal α80 |
| 텍스트 진입 | | x 1300→100, 300ms `acc: 2` |

### 9.4 단일 재생 5/7/9키 (`play-{5,7,9}k.json5`, `replace: ['field','gauge','judge','score','counts','graph','cover','frame']`)

공통 사각형(1P 사이드, `op:[900]`; 2P `op:[901]`는 x를 1280-x-w로 미러):

| 영역 | 사각형 | 객체 |
| --- | --- | --- |
| 노트 필드(7K) | x 34~354, 레인 상단 y 0, 판정선 y 500 | `note.dst` 레인: SC(34,0,64,500)·1(98,40)·2(138,32)·3(170,40)·4(210,32)·5(242,40)·6(282,32)·7(314,40). `size: 22`. 레인 배경은 `image`(night α200) 레인별, 판정선 `judge-line`(34,498,320,4), 소절선은 `note.group` 트랙(ui 소절선) |
| 노트 필드(5K) | SC(70,64)+5키 폭 40/32/40/32/40 → x 70~318 | 같은 y |
| 노트 필드(9K) | 9레인 폭 34, x 42~348, 스크래치 없음 | 색 3/4/5 행 |
| 레인커버 | `hiddenCover` src covers.png SOLID, (34,0,320,500), `liftCover` 동일 | `filepath` 슬롯 `LANE COVER` |
| 키 위젯 | (34,508,320,56) | 턴테이블 원(34,508,48,48 `op:[KEYON]` 타이머로 밝기), 백건/흑건 위젯 각 레인 x, `timer: KEYON_1P_KEY{n}` 셀 전환 |
| 게이지 | (34,572,320,22) | `gauge`(nodes ui 4종, `parts: 50`) + `play-gauge-value`(digits-m, x 300 우측, `NUMBER_GROOVEGAUGE`) + 종류 라벨 텍스트 `op:[OPTION_GAUGE_*]` |
| 점수·속도 | (34,602,320,28) | "SCORE" 라벨 + `play-ex`(digits-m, `NUMBER_SCORE`) 좌, "HI-SPEED" + `floatvalue`(`hispeed`) 우 |
| 판정 팝업 | 필드 중앙 x 194 | `judge` id `judge-1p`, `index: 0`: `images[k]` = text 객체 6개(색 §palette judge), `numbers[k]` = digits-l 콤보. FAST/SLOW 텍스트 `play-fastslow`(`op:[OPTION_1P_EARLY]`/`[OPTION_1P_LATE]`, y 300), ±ms `play-timing-ms`(`op:[924/925]`) |
| 곡 정보 | (368,8,632,100) | 스테이지 배너 텍스트(y 8), `play-title`(y 28,h 30), `play-artist`(y 60,h 18), 레벨·난이도·게이지 종류·TOTAL 배지 행(y 84) |
| BGA | (368,112,632,474) | `bga` id `play-bga`; `op:[910]` LARGE, `[911]` STANDARD (404,140,560,420), `[912]` OFF(그리지 않음). 오프셋 id 40 |
| 판정 카운터 | (368,596,232,116) | `play-count-{pg,gr,gd,bd,pr,ms}` 2열, `play-fast`/`play-slow`(y 690), 라벨 텍스트 |
| BPM | (620,610,180,80) | MIN/BPM/MAX 3열 digits-m(`NUMBER_MINBPM`/`NUMBER_NOWBPM`/`NUMBER_MAXBPM`) |
| 시간·LN | (820,610,180,80) | TIME 경과/전체(`NUMBER_PLAYTIME_MINUTE`…), `LN MODE` 배지 |
| 그래프 열 | (1008,8,264,704) | 헤더, `play-graph-you`(digits-m `NUMBER_SCORE`), `play-graph-pace`(`NUMBER_TARGET_SCORE`), 그래프 배경(1008,90,264,490) + 기준선 AAA/AA/A(8/9·7/9·6/9 높이) + 막대 `play-graph-ex`(x 1120,w 44 `RATE_EXSCORE` 세로 graph)·`play-graph-target`(x 1180 `RATE_TARGETSCORE`)·`play-graph-best`(x 1232 `RATE_BESTSCORE`), 하단 `play-target-delta`(20202)·현재/최고 % 텍스트·LV 배지. `op:[920]` 기본, `[921]`은 열을 필드 옆 x 368로 옮기고 BGA를 x 640부터 |
| 전경 | frame-sp.png | 필드·BGA·그래프 열 외곽선만 |

### 9.5 이중 재생 10/14키 (`play-{10,14}k.json5`, 같은 `replace`)

| 영역 | 사각형 |
| --- | --- |
| 1P 필드(14K) | x 300~620, 레인 SC(300,64)·키 40/32 교차; 10K는 SC+5키 x 336~584 |
| 2P 필드(14K) | x 674~994(스크래치 오른쪽 끝 930~994); 10K x 710~958 |
| 판정선 | y 500 양측, 판정 팝업 각 필드 중앙, `judge` index 0/1 |
| 게이지 | 중앙 (460,572,360,22) + 수치 |
| 키 위젯 | 양측 (300,508)·(674,508) |
| BGA | 좌 (14,168,232,174) + 동일 `bga` id 두 번째 목적지 (14,350,232,174) |
| 좌 정보 | (14,8,232,150) 스테이지·제목·아티스트·레벨·TIME |
| 좌 하단 | (14,540,232,170) 판정 카운터·BPM |
| 그래프 열 | (1008,8,264,704) SP와 동일 |
| SCORE/HI-SPEED | (300,602)·(674,602) |

### 9.6 결과 (`result.json5`, `replace: ['score','clear','judgment','target','grade','graphs','title','hint','ir']`)

| 영역 | 사각형 | 객체 |
| --- | --- | --- |
| 배경 | 랭크별 `result-bg-*.png` `op:[OPTION_RESULT_AAA_1P…]`/CLEAR/FAILED, `op:[911]`이면 공통 | |
| 상단 | (0,0,1280,40) `result-title` 가운데 "STAGE RESULT", 모드 라벨 우측 | |
| 좌 상단 | (16,48,384,200) | `gaugegraph`(24,56,368,184) + `result-rank`(큰 텍스트 등급, 중앙 y 100) + `result-rate`(%) |
| 좌 표 | (16,256,384,250) | 행: CLEAR TYPE(`result-clear`) / DJ LEVEL / SCORE(`result-score`) / MAXCOMBO(`result-combo`) / MISS / NOTES(`result-notes`) — 라벨 x 32, 베스트 열 x 220, 금회 열 x 300, 차분 우측 x 392(`NUMBER_DIFF_HIGHSCORE`), 행 피치 36 |
| 좌 하단 | (16,512,384,200) | 판정 6행 `result-judge-*`(x 32, 피치 22) + FAST/SLOW(`NUMBER_EARLY_PERFECT`/`LATE_PERFECT`) + `result-rankbar`(y 690,h 10) |
| 중앙 | (440,56,380,640) | `judgegraph`(440,80,380,150) 제목 "JUDGE", `timingdistributiongraph`(440,260,380,150) "TIMING", `bpmgraph`(440,440,380,110) "BPM", 곡 제목/아티스트(y 570), 배지 행(y 640: 레벨·모드·노트수·게이지·LN) |
| 우 열 | (1008,8,264,704) | 플레이와 동일 그래프 열(YOU/PACEMAKER/막대/LV) + `result-target`(20011) |
| 힌트 | (0,700,1280,20) `result-hint`(20204) | |

### 9.7 커스터마이즈 헤더

번들 스코프(`scope: 'bundle'`) property: PLAY SIDE(1P 900/2P 901, 5/7/9K 문서), BGA SIZE(LARGE 910/STANDARD 911/OFF 912), GRAPH POSITION(FAR 920/NEAR 921), JUDGE TIMING(OFF 923/FAST-SLOW 924/±MS 925), RESULT BACKGROUND(BY RANK 910/COMMON 911, result 문서), DECIDE EFFECT(ON 990/OFF 991). 파일 슬롯: NOTES(`images/notes/*.png`, Default), LANE COVER(`images/covers/*.png`, Solid), SELECT BACKGROUND(`images/select/*.png`), DECIDE BACKGROUND(`images/decide/*.png`). 오프셋: BGA(40, x/y/w/h/a), FRAME BRIGHTNESS(47, a), LANE BRIGHTNESS(48, a), SCORE GRAPH(46, x/a). 카테고리: PLAY OPTION / DETAIL / CUSTOMIZE.

### 9.8 사운드 (`tools/generate-sounds.py`, `sound/`)

22 스템을 44.1kHz 16bit 모노 WAV로 합성한다(사인·삼각파 짧은 엔벌로프, 각 0.05~0.6초). `select`·`decide`·`clear`·`fail`은 짧은 아르페지오, `scratch`·`o-change`는 클릭, `f-open`/`f-close`·`o-open`/`o-close`는 상행/하행 스윕, `playready`/`playstop`은 2음, `guide-*`는 판정별 높이가 다른 단음, `resultclose`·`course_*`는 2음 하행. 총 크기 1MB 이하.

## 10. 구현 분할과 파일 소유권

| 웨이브 | 작업 | 소유 파일 | 병렬 |
| --- | --- | --- | --- |
| W0 | 스캐폴드: `Body`/`SkinObjectKind` 변형, `FrameExtra`, `NestedTracks`·`replace`·`hotspot`·`scope` 필드, 새 파일 빈 모듈, 사설 id 상수. 컴파일 통과 | `rbms-skin/src/{model.rs,model/objects.rs,loader.rs}`, `rbms-render/src/skin_render/{mod.rs,object.rs,draw.rs,state.rs}` + 빈 `notes.rs`·`gauge.rs`·`judge.rs`·`songlist.rs`·`covers.rs`·`graphs.rs`·`color.rs` | 직렬(1) |
| W1-a | 로더 중첩 트랙·헤더 스코프·hotspot 검증 + 테스트 | `rbms-skin/src/loader.rs`, `rbms-skin/tests/*` | 병렬 |
| W1-b | note·gauge·judge·covers 렌더 + 테스트 | `notes.rs`, `gauge.rs`, `judge.rs`, `covers.rs`, `skin_render/tests.rs`(자기 절만) | 병렬 |
| W1-c | songlist·graphs·color 렌더 + 테스트 | `songlist.rs`, `graphs.rs`, `color.rs`, `select.rs`(히트 사각형 헬퍼만) | 병렬 |
| W1-d | 설정 공유 스코프·세대 목록 설치·custom 이동·사운드 폴더 | `rbms-config/src/{schema.rs,settings.rs}`, `apps/rbms-player/src/{assets.rs,syssound.rs}` | 병렬 |
| W1-e | 자산 생성 스크립트·팔레트·사운드 생성 스크립트·PNG/WAV | `assets/skins/steel-neon-v3/{palette.json,tools/*,images/*,sound/*}` | 병렬 |
| W2 | 앱 배선: 상태 조립(`FrameExtra`), 타이머 드라이버, 대체 단위 게이트, 핫스팟·songlist 히트, `Skin` 레인 재구성, 옵션 행 노출, SKIN 탭 번들 행 | `apps/rbms-player/src/{skin_screen.rs,stage/play/mod.rs,stage/select/mod.rs,stage/result.rs,app_options.rs,skin_select.rs,settings_ui.rs}`, `rbms-render/src/skin_render/screen.rs`, `rbms-render/src/{hud.rs,result.rs,playfield.rs}`(대체 플래그) | 직렬(W1 후) |
| W3 | 기본 문서 작성(선택·옵션·결정·5/7/9/10/14키·결과, 공유 객체, 커스터마이즈 헤더) | `assets/skins/steel-neon-v3/*.json5`, `shared/*.json5`, `play*.ron`, `theme.ron` | W2 후, 화면별 병렬 |
| W4 | 적대 리뷰(렌더·앱·자산 3갈래) → 수정 → 게이트 | 전체 | 리뷰 병렬, 수정 직렬 |

하위 에이전트는 커밋·푸시하지 않는다. 각 웨이브 산출은 `cargo test -p <crate>` 최소 검증과 diff 보고로 넘긴다.

## 11. 검증

- 단위: 새 객체별 헤드리스 렌더 테스트(픽셀 검사), 로더 중첩 트랙·스코프·hotspot 파싱, 설정 병합·세대 이동·custom 이동, 사운드 폴더 결정.
- 장면 캡처: `RBMS_SKIN_CAPTURE_DIR`로 `select`·`options`·`decide`·`play-5k/7k/9k/10k/14k`·`result` PNG 생성. 캡처 검사는 (a) BGA 프로브 픽셀 보존, (b) 대체된 네이티브 글자가 남지 않음(대체 전후 픽셀 비교), (c) 곡 행 히트 사각형이 songlist 슬롯과 일치.
- 실제 창: 앱을 실행해 선택→옵션→결정→7키 플레이→결과를 화면 캡처로 확인하고 `docs/quality-assurance/2026-09-17-skin-system/captures/live-*.png`에 저장(절차는 같은 폴더 `checklist.md` §3, 2026-09-18 현재 미실행).
- 게이트: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace`, 금지 명칭 grep 0건, `git diff --check`.

## 12. 비목표와 후속

- 곡 선택·결정 BGM 루프(사운드 세트의 `select`/`decide` 스템은 1회 재생 유지).
- `pmchara`·`practice`·`skinpreview`·`customEvents`·`customTimers`·객체 `click`/`act` 이벤트.
- 외부 CSV 스킨 직접 로드·자동 변환, 비디오 BGA, 24키 실제 활성화.
- 옵션 패널 마우스 조작(현재 키 전용 유지).
