# 스킨 제작 가이드

최종 갱신 2026-09-18 · 대응 코드: `dev` 작업 트리(기준 커밋 `4537ad6` + 미커밋 스킨 시스템 변경, `docs/HANDOFF.md` 참조).

rbms 는 화면마다 JSON5 스킨 문서를 읽어 그린다. 문서가 없는 화면은 내장 화면으로 그려진다. 이 문서는 기본 번들 `steel-neon-v3` 를 기준으로 폴더 구조, 문서 계약, 커스터마이즈, 검증 방법을 설명한다. 엔진 계약의 정본은 `docs/plan/2026-09-17-skin-system-completion.md` 이고, 색 테마(`theme.ron`)는 `docs/theme.md` 를 본다.

## 1. 위치와 폴더

- 문서 검색 루트: 설정 파일 옆 `skin/` (SKIN 탭의 폴더 설정으로 변경 가능).
- 번들 한 벌은 루트 아래 한 디렉터리다. 기본 번들은 첫 실행 때 `skin/steel-neon-v3/` 로 설치되며 이미 있는 파일은 덮어쓰지 않는다.

```
skin/steel-neon-v3/
  select.json5  decide.json5  result.json5
  play-5k.json5 play-7k.json5 play-9k.json5 play-10k.json5 play-14k.json5 play-24k.json5
  shared/objects-play.json5     단일 플레이 문서가 include 로 공유하는 객체(PLAY SIDE × GRAPH POSITION 4조합)
  shared/judge-sp.json5         단일 플레이 판정 팝업(1P/2P 분기)
  play.ron  play-dual.ron       문서에 note 객체가 없을 때 쓰는 네이티브 필드 배치
  theme.ron                     UI 색·선택 화면 열 배치
  palette.json  tools/          이미지·사운드 생성 스크립트
  images/                       PNG 27장: 시트(notes·digits-s/m/l/f·ui·covers)·배경 8장·프레임 7장(sp·sp-2p·sp-near·sp-2p-near·dp·select·result)
  images/{notes,covers,select,decide}/   파일 슬롯 후보(SKIN 탭 파일 행이 고르는 PNG)
  sound/                        22개 시스템 사운드
```

## 2. 문서 구조

문서는 1280×720 을 기준으로 y-up(아래가 0) 좌표로 쓴다. 상단 필드는 `type`(화면 번호), `composition`(`replace`/`overlay`/`layered`), `source`(이미지), `font`, 객체 목록(`image`, `imageset`, `value`, `floatvalue`, `text`, `slider`, `graph`, `note`, `gauge`, `judge`, `songlist`, `hiddenCover`, `liftCover`, `gaugegraph`, `judgegraph`, `bpmgraph`, `timingdistributiongraph`, `timingvisualizer`, `hiterrorvisualizer`, `densitygraph`, `bga`), 그리고 `destination`(객체 배치와 애니메이션)이다. 객체 필드의 이름과 기본값은 레퍼런스 구현의 JSON 스킨과 같으므로 그 형식의 문서를 그대로 읽을 수 있다.

### 2.1 합성 모드와 레이어

- `replace`: 문서가 화면 전체를 그린다.
- `overlay`: 내장 화면 위에 문서를 얹는다.
- `layered`: `destination.layer` 가 `background` 인 객체 → 내장 화면 → `foreground` 객체 순서로 그린다. 기본 번들은 이 모드를 쓴다.

### 2.2 내장 화면 대체 (`replace`)

`layered` 문서는 최상위 `replace: [...]` 로 내장 출력 묶음을 끌 수 있다. 묶음에 필요한 객체 id 가 하나라도 없으면 경고를 남기고 내장 출력을 유지한다.

| 화면 | 이름 | 필요한 객체 id |
| --- | --- | --- |
| play | `field` | `note` 객체 |
| play | `gauge` | `gauge` 객체, `play-gauge-value` |
| play | `judge` | `judge` 객체 |
| play | `score` | `play-ex`, `play-best`, `play-green` |
| play | `counts` | `play-count-pg`…`play-count-ms`, `play-fast`, `play-slow` |
| play | `graph` | `play-graph-ex`, `play-graph-best`, `play-graph-target` |
| play | `cover` | `hiddenCover` 또는 `liftCover` |
| play | `frame` | 없음(내장 외곽선만 끔) |
| select | `list` | `songlist` 객체 |
| select | `detail` | `select-title`, `select-artist`, `select-genre`, `select-level`, `select-stat-0`…`5`, `select-density`, `select-record` |
| select | `topbar` | `hotspot` 의 `search`·`sort`·`folders`·`tables`·`records`·`settings` 6종 전부 |
| select | `options` | `options-panel`, `option-row-{0..10}-label`, `option-row-{0..10}-value` |
| result | `score` | `result-score-label`, `result-score`, `result-combo-label`, `result-combo`, `result-notes-label`, `result-notes` |
| result | `clear` | `result-clear` |
| result | `judgment` | `result-judge-perfect`, `result-judge-great`, `result-judge-good`, `result-judge-bad`, `result-judge-poor`, `result-judge-miss` |
| result | `target` | `result-target` |
| result | `grade` | `result-rank`, `result-rate`, `result-rankbar` |
| result | `graphs` | `gaugegraph`, `judgegraph`, `timingdistributiongraph` |
| result | `title` | `result-title` |
| result | `hint` | `result-hint` |
| result | `ir` | `result-ir` |

### 2.3 입력 영역

- 곡 목록 클릭 영역은 `songlist.clickable` 에 적은 슬롯의 배치 사각형이다.
- 버튼은 `hotspot: [{ id: 'btn-search', action: 'search' }]` 로 선언한다. `action` 은 `search`, `sort`, `folders`, `tables`, `records`, `settings`, `modal-replay`, `modal-close`.

### 2.4 rbms 전용 속성 id

레퍼런스 id 와 겹치지 않는 20000 대역을 쓴다. 결과 텍스트 20001~20011, 옵션 행 라벨 20101~20111 · 값 20121~20131 · 포커스 20141~20151, 플레이 목표 이름 20201 · 차이 20202, 힌트 20203(선택)/20204(결과), 결과 IR 상태 20205, 선택 통계 셀 20301~20306, 선택 기록 20311~20315(EX/BP/플레이·클리어 수/램프/시각). 플레이 문서는 레퍼런스 id로 BPM 최소·최대·주(90~92), 경과·남은 시간(161~164), FAST/SLOW 합(423/424), 목표 EX(121)·차이(153), 레벨(96), 아티스트(14)를 읽는다.

### 2.5 rbms 확장

- `judge.images[k]` 는 `image` 대신 `text` 객체 id 를 가리킬 수 있다.
- `densitygraph` 는 선택 곡의 노트 밀도를 그린다(`barColor`, `peakColor`, `lineWidth`).
- 플레이 문서에 `note` 객체가 있으면 레인 사각형은 문서가 정하고 내장 HUD·커버도 그 좌표를 따른다.

### 2.6 기본 번들의 옵션

번들 스코프로 PLAY SIDE(1P/2P: 필드·그래프 열·BGA 미러), BGA SIZE(LARGE/STANDARD/OFF), GRAPH POSITION(FAR/NEAR), JUDGE TIMING(OFF/FAST-SLOW/MILLISECONDS), 배경·노트·레인 커버 파일 슬롯, BGA(40)·FRAME BRIGHTNESS(47)·LANE BRIGHTNESS(48)·SCORE GRAPH(46) 오프셋을 선언한다. 단일 플레이 문서는 `note.dst`를 1P/2P로 분기하므로 사이드를 바꾸면 노트 필드가 통째로 옮겨진다.

### 2.7 알려진 제약

- 선택 화면의 기록은 최고 EX·BP·플레이/클리어 수·램프·시각 5줄이다. 판정별 카운트와 최근 기록 행은 브라우저 상태에 id가 없다.
- `frame-dp.png`는 14키 좌표로 그려져 10키에서는 프레임이 필드보다 넓다.
- BGA SIZE OFF는 투명 목적지로 BGA 소유를 유지한다(목적지를 전부 없애면 내장 사각형이 다시 그려진다).
- `shared/objects-play.json5`는 옵션 4조합을 펼친 생성 산출물이다. 좌표를 고칠 때 조합별로 함께 고친다.

## 3. 커스터마이즈

문서 상단의 `property`(선택 옵션), `filepath`(파일 슬롯), `offset`(위치·크기·각도·알파 보정)은 설정의 SKIN 탭에 행으로 나타난다. `scope: 'bundle'` 을 적은 항목은 번들 안의 모든 문서가 값을 공유하며 화면과 무관하게 `BUNDLE` 그룹으로 보인다. 선택값은 설정 파일에 저장되고, 번들 세대가 바뀌어도 같은 이름의 행은 값을 유지한다.

## 4. 사운드

`audio.sound_folder` 를 지정하지 않았고 프리셋이 `STEEL NEON` 이면 번들의 `sound/` 에서 22개 스템(`scratch`, `f-open`, `f-close`, `o-open`, `o-change`, `o-close`, `playready`, `playstop`, `clear`, `fail`, `resultclose`, `course_clear`, `course_fail`, `course_close`, `guide-pg`…`guide-ms`, `select`, `decide`)을 읽는다. 파일은 `wav`/`ogg`/`flac`/`mp3` 순으로 찾는다.

## 5. 편집과 반영

- JSON5 를 고쳤으면 SKIN 탭의 RELOAD, RON 을 고쳤으면 앱 재시작.
- 이미지는 `palette.json` 을 고친 뒤 `uv run tools/generate-assets.py`, 사운드는 `uv run tools/generate-sounds.py` 로 다시 만든다. PNG/WAV 를 직접 바꿔도 된다.
- 헤드리스 캡처: `RBMS_SKIN_CAPTURE_DIR=<폴더> cargo test -p rbms-player the_current_default_bundle_keeps_information_and_chart_art_visible --lib`(select·options·decide·play 5종·result), `RBMS_SKIN_CAPTURE_DIR=<폴더> cargo test -p rbms-player render_tests_skin_v3 --lib`(2P·NEAR·기록 패널 등 v3 장면).
- 실제 GPU 창 확인 절차는 `docs/quality-assurance/2026-09-17-skin-system/checklist.md` §3.
