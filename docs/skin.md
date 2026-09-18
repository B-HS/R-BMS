# 스킨 제작 가이드

최종 갱신 2026-09-18 · 대응 코드: 브랜치 `feat/skin-followups`(스킨 후속 기능 L2 머지 시점, `docs/PROCESS.md` 최상단 참조).

rbms 는 화면마다 JSON5 스킨 문서를 읽어 그린다. 문서가 없는 화면은 내장 화면으로 그려진다. 이 문서는 기본 번들 `steel-neon-v3` 를 기준으로 폴더 구조, 문서 계약, 커스터마이즈, 검증 방법을 설명한다. 엔진 계약의 정본은 `docs/plan/2026-09-17-skin-system-completion.md` 이고, 색 테마(`theme.ron`)는 `docs/theme.md` 를 본다.

## 1. 위치와 폴더

- 문서 검색 루트: 설정 파일 옆 `skin/` (SKIN 탭의 폴더 설정으로 변경 가능).
- 번들 한 벌은 루트 아래 한 디렉터리다. 기본 번들은 첫 실행 때 `skin/steel-neon-v3/` 로 설치되며 이미 있는 파일은 덮어쓰지 않는다.

```
skin/steel-neon-v3/
  select.json5  decide.json5  result.json5
  play-5k.json5 play-7k.json5 play-9k.json5 play-10k.json5 play-14k.json5 play-24k.json5(26레인 실제 문서)
  shared/objects-play.json5     단일 플레이 문서가 include 로 공유하는 객체(PLAY SIDE × GRAPH POSITION 4조합)
  shared/judge-sp.json5         단일 플레이 판정 팝업(1P/2P 분기)
  play.ron  play-dual.ron       문서에 note 객체가 없을 때 쓰는 네이티브 필드 배치
  theme.ron                     UI 색·선택 화면 열 배치
  palette.json  tools/          이미지·사운드 생성 스크립트
  images/                       PNG 29장: 시트(notes·digits-s/m/l/f·ui·covers)·배경 8장·프레임 9장(sp·sp-2p·sp-near·sp-2p-near·dp·dp-10k·24k·select·result)
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
- 버튼은 `hotspot: [{ id: 'btn-search', action: 'search' }]` 로 선언한다. `action` 은 `search`, `sort`, `folders`, `tables`, `records`, `settings`, `modal-replay`, `modal-close`. 레퍼런스 호환 경로인 `act`/`click`(§2.8)과 함께 쓸 수 있다.
- 옵션 패널은 행마다 클릭 사각형을 낸다. 네이티브 패널은 강조 띠, `options` 대체 문서는 `option-row-{i}-value` 객체의 현재 목적지 사각형이다. 클릭은 그 행을 포커스하고 오른쪽 반쪽이면 +1, 왼쪽 반쪽이면 -1 로 값을 옮긴다(키 조작과 같은 소리·저장). 행 밖 클릭은 아래 화면으로 내려간다.

### 2.4 rbms 전용 속성 id

레퍼런스 id 와 겹치지 않는 20000 대역을 쓴다. 결과 텍스트 20001~20011, 옵션 행 라벨 20101~20111 · 값 20121~20131 · 포커스 20141~20151, 플레이 목표 이름 20201 · 차이 20202, 힌트 20203(선택)/20204(결과), 결과 IR 상태 20205, 선택 통계 셀 20301~20306, 선택 기록 20311~20315(EX/BP/플레이·클리어 수/램프/시각). 플레이 문서는 레퍼런스 id로 BPM 최소·최대·주(90~92), 경과·남은 시간(161~164), FAST/SLOW 합(423/424), 목표 EX(121)·차이(153), 레벨(96), 아티스트(14)를 읽는다. 선택 문서는 레퍼런스 id 로 포커스 차트 최고 기록의 판정별 카운트(NUMBER_PERFECT 110·GREAT 111·GOOD 112·BAD 113·POOR 114·MISS 420)와 최대 콤보(75)를 읽는다(기록이 없으면 0). 연습 행은 라벨 20401~20416·값 20421~20436·포커스 20441~20456, 연습 화면 열림 옵션 20461(§2.9). 문서 자체 타이머 id 대역은 10000~19999(§2.8).

### 2.5 rbms 확장

- `judge.images[k]` 는 `image` 대신 `text` 객체 id 를 가리킬 수 있다.
- `densitygraph` 는 선택 곡의 노트 밀도를 그린다(`barColor`, `peakColor`, `lineWidth`).
- 플레이 문서에 `note` 객체가 있으면 레인 사각형은 문서가 정하고 내장 HUD·커버도 그 좌표를 따른다.
- 플레이 문서가 `bga` 객체를 **선언**하기만 하면 내장 BGA 사각형은 그려지지 않는다(목적지가 모두 꺼져 있어도 같다). BGA SIZE OFF 는 목적지를 두지 않는 것으로 표현한다.
- `skinpreview` 객체는 모든 화면 문서에서 쓸 수 있다(§2.9).

### 2.6 기본 번들의 옵션

번들 스코프로 PLAY SIDE(1P/2P: 필드·그래프 열·BGA 미러), BGA SIZE(LARGE/STANDARD/OFF), GRAPH POSITION(FAR/NEAR), JUDGE TIMING(OFF/FAST-SLOW/MILLISECONDS), 배경·노트·레인 커버 파일 슬롯, BGA(40)·FRAME BRIGHTNESS(47)·LANE BRIGHTNESS(48)·SCORE GRAPH(46) 오프셋을 선언한다. 단일 플레이 문서는 `note.dst`를 1P/2P로 분기하므로 사이드를 바꾸면 노트 필드가 통째로 옮겨진다.

### 2.7 알려진 제약

- 선택 화면의 최근 기록 행(개별 플레이 목록)은 브라우저 상태에 id가 없다. 최고 기록의 판정별 카운트·최대 콤보는 §2.4 의 레퍼런스 id 로 읽는다.
- `shared/objects-play.json5`는 옵션 4조합을 펼친 생성 산출물이다. 좌표를 고칠 때 조합별로 함께 고친다. 정보 패널(NOW PLAYING 줄)의 내용 여백은 플레이트 안쪽 8px 이고 LV…NOTES 묶음은 그 여백에 우측 정렬된다.
- 선택 화면 RECORDS 패널(x 380..664, y 238..368)의 외곽선은 `frame-select.png` 에 구워져 있다. 패널 좌표를 바꾸면 생성기도 함께 고친다.
- 문서의 커스텀 타이머·이벤트는 곡 선택 화면에서만 매 프레임 갱신된다. 플레이·결과·결정·키 설정 문서도 선언은 읽히지만 아직 실행되지 않는다(후속).
- `composition: 'replace'` 인 선택 문서는 클릭 사각형을 받지 못한다. 클릭이 필요하면 `layered`/`overlay`.
- 24키 문서는 `shared/*.json5` 를 include 하지 않고 객체를 직접 가진다(공유 객체가 320px 7K 필드 기준이라서). 24키의 스크래치 두 레인은 같은 KEYON 타이머(100)를 낸다.
- 곡 선택 BGM 루프와 미리듣기는 동시에 난다(레퍼런스는 미리듣기 동안 BGM 을 멈춘다). 후속.

### 2.8 문서 이벤트: `customTimers` · `customEvents` · `act` · `click` · `mouseRect`

레퍼런스 JSON 스킨과 같은 의미로 동작한다.

- `customTimers: [{ id, timer }]` — `id` 는 10000~19999(내장 타이머 0~2999 와 겹치면 경고 후 무시). `timer` 는 타이머 id(그 타이머가 켜진 순간) 또는 Lua 식. **단위는 밀리초**(이 빌드의 모든 타이머와 같은 시계; 레퍼런스의 마이크로초와 다르므로 1000 을 곱하지 않는다). `nil`/`false` 는 꺼짐. `timer` 가 없는 항목은 수동 타이머로, 이벤트 action 만 켜고 끈다. 어떤 `dst` 든 `timer: <id>` 로 쓸 수 있다.
- `customEvents: [{ id, action, condition, minInterval }]` — `id` 가 레퍼런스 내장 이벤트 번호와 겹치면 경고 후 무시(내장이 우선). `condition`(옵션 id 또는 Lua 식)이 있으면 참인 프레임마다 발화하되 마지막 발화 후 `minInterval` ms 동안 억제(처음 참이 되는 프레임은 항상 발화). `condition` 이 없으면 클릭이나 다른 이벤트의 `action` 이 이름을 부를 때만 발화. `action` 은 숫자(내장 또는 문서 자체 이벤트) 또는 Lua 식. 조건 발화는 증분 0, 클릭은 ±1 을 나른다. 자체 이벤트 연쇄는 8단계까지.
- `action` Lua 식에서만 `skin.set_timer(id)` / `skin.clear_timer(id)` 로 문서 자체 타이머를 켜고 끌 수 있다. 값·조건·타이머 식에는 이 함수가 없다. 커스텀 옵션을 쓰는 API 는 없다.
- `image`·`imageset` 의 `act`(숫자 = 이벤트 id, 문자열 = Lua 식; 이벤트 이름 문자열은 아니다)와 `click`(0 전체 +1 · 1 전체 -1 · 2 좌 -1/우 +1 · 3 아래 -1/위 +1, 그 외 값은 경고 후 클릭 없음). 클릭 사각형은 그 프레임의 **현재 애니메이션 목적지**이며, 그려지지 않는 객체(조건 실패·투명·`mouseRect` 밖)는 클릭되지 않는다. 겹치면 위에 그린 객체가 먼저다.
- `mouseRect`(목적지 필드)는 커서가 그 사각형(객체 영역 기준 상대 좌표) 밖이면 객체를 그리지 않는 호버 게이트다.
- 이 빌드가 곡 선택 화면에서 실행하는 내장 이벤트: 12/312 정렬, 13 키 설정, 14 설정 SKIN 탭, 15 포커스 차트 시작, 315 연습, 19·316~318 기록 모달 리플레이, 89/90 즐겨찾기, 210 랭킹 패널, 211 폴더 재스캔. 설정 행을 클릭 증분만큼 옮기는 번호: 16 오토플레이, 40 게이지, 42 랜덤, 55 하이스피드 고정, 57 하이스피드, 72 BGA, 77 목표, 308 LN 모드, 330 레인커버, 331 리프트, 332 히든, 340 판정 알고리즘. 그 밖의 번호는 경고 1회 후 무시.

### 2.9 `skinpreview` 와 `practice`

- `skinpreview: { id }` + 같은 id 의 `destination`. 어느 화면 문서에서든 SKIN 탭이 보고 있는 화면(`config.skin.screen`)의 문서를 유휴 상태(모든 속성 미매핑·타이머 꺼짐·시계 0)로 256×144 에 오프스크린 렌더한 정지 화면을 그린다(문서를 다시 읽을 때 재렌더). 자기 화면을 미리보는 문서는 아무것도 그리지 않는다. 네이티브 SKIN 탭도 같은 정지 화면을 (1010,160) 256×144 에 PREVIEW 캡션으로 보여준다.
- `practice: { id, visibleItems }` 는 플레이 문서 전용. `visibleItems > 0` 이면 행 수만 정하고(0~16) 행은 §2.4 의 연습 id 에 묶인 일반 객체가 그린다. `visibleItems: 0` 이면 객체 사각형 안에 레거시 목록을 직접 그린다(라벨 x+w/8, 값 +150, 첫 행 y+h*7/8, 행 간격 22, 포커스 행만 전체 색). 연습 화면은 차트 모드의 플레이 문서가 `practice` 를 선언할 때 배경 → 전경 순으로 그 문서를 그리고, 없으면 네이티브 행을 유지한다. 플레이 크롬 숫자가 0 으로 뒤에 그려지므로 깨끗한 패널을 원하면 크롬 객체를 `op: [-20461]` 로 가린다. 기본 번들은 `play-7k.json5` 에 12행 패널을 `op: [20461]` 로 두었다. 로드 시점의 `{ if, values }` 그룹은 20461 같은 런타임 id 로 게이트할 수 없고, 목적지 `op` 만 런타임 게이트다.

### 2.10 24키

- `KEYBOARD_24K` 는 bmson `info.mode_hint: "keyboard-24k"` 로만 도달한다(BMS 채널 표에는 없다). 26레인: 키 0~23, 스크래치 24·25. 레인 i 는 아래 옥타브 C 에서 i 반음 위이며 i%12 ∈ {1,3,6,8,10} 이 검은건반(10개).
- 기본 키: 아래 옥타브 흰건반 Z X C V B N M(레인 0,2,4,5,7,9,11)·검은건반 S D G H J(1,3,6,8,10), 위 옥타브 Q W E R T Y U(12,14,16,17,19,21,23)·2 3 5 6 7(13,15,18,20,22), 스크래치 LSHIFT(24)·RSHIFT(25). `keyconfig.ron` 키는 `24K`.
- 내장 필드(`Skin::build`)는 레인별 폭 가중치(흰 1.0·검은 0.8·스크래치 1.75, 합이 레인 수)로 배치하고 다른 모드의 사각형은 그대로다. 문서 `play-24k.json5`(타입 16)는 x 34 부터 610px 필드(스크래치 42·흰 24·검은 19), 판정선 y 220, 전용 `frame-24k.png`, 대체 8단위 전부, NOTES·LANE COVER·BGA·FRAME/LANE BRIGHTNESS 행만 가진다(PLAY SIDE 등 번들 옵션 없음). 타입 17(DOUBLE)·18(BATTLE)은 이 빌드에 모드가 없다.
- 캡처: `RBMS_SKIN_CAPTURE_DIR=<폴더> cargo test -p rbms-player --lib render_tests_skin_v3` 가 `play-24k.png` 를 쓴다.

## 3. 커스터마이즈

문서 상단의 `property`(선택 옵션), `filepath`(파일 슬롯), `offset`(위치·크기·각도·알파 보정)은 설정의 SKIN 탭에 행으로 나타난다. `scope: 'bundle'` 을 적은 항목은 번들 안의 모든 문서가 값을 공유하며 화면과 무관하게 `BUNDLE` 그룹으로 보인다. 선택값은 설정 파일에 저장되고, 번들 세대가 바뀌어도 같은 이름의 행은 값을 유지한다.

## 4. 사운드

`audio.sound_folder` 를 지정하지 않았고 프리셋이 `STEEL NEON` 이면 번들의 `sound/` 에서 22개 스템(`scratch`, `f-open`, `f-close`, `o-open`, `o-change`, `o-close`, `playready`, `playstop`, `clear`, `fail`, `resultclose`, `course_clear`, `course_fail`, `course_close`, `guide-pg`…`guide-ms`, `select`, `decide`)을 읽는다. 파일은 `wav`/`ogg`/`flac`/`mp3` 순으로 찾는다.

`select` 는 곡 선택 화면의 배경음으로, 화면에 들어올 때 오디오 엔진의 루프 보이스(`Command::Play { looping }`, 샘플 끝에서 커서를 되감아 이음새 없이 반복)로 시작하고 화면을 떠날 때 정지한다(설정·결과에서 돌아오면 다시 시작, 곡 결정 시 정지 후 `decide` 1회). BGM 스템(`select`·`decide`)만 루프할 수 있고 효과음은 루프하지 않는다. `select` 파일이 없으면 조용할 뿐 오류가 아니다. 차트를 고를 때 나던 `select` 1회 재생은 레퍼런스처럼 없앴다.

## 5. 편집과 반영

- JSON5 를 고쳤으면 SKIN 탭의 RELOAD, RON 을 고쳤으면 앱 재시작.
- 이미지는 `palette.json` 을 고친 뒤 `uv run tools/generate-assets.py`, 사운드는 `uv run tools/generate-sounds.py` 로 다시 만든다. PNG/WAV 를 직접 바꿔도 된다. 이중 필드 프레임(`frame-dp.png`·`frame-dp-10k.png`)의 필드 열은 생성기 상수 `DUAL_FIELD_COLUMNS_14K`/`DUAL_FIELD_COLUMNS_10K` 가 정하므로 `play-10k/14k.json5` 의 레인 dst 를 바꾸면 함께 고친다. 24키 프레임은 `frame_keyboard_play`.
- 헤드리스 캡처: `RBMS_SKIN_CAPTURE_DIR=<폴더> cargo test -p rbms-player the_current_default_bundle_keeps_information_and_chart_art_visible --lib`(select·options·decide·play 5종·result), `RBMS_SKIN_CAPTURE_DIR=<폴더> cargo test -p rbms-player render_tests_skin_v3 --lib`(2P·NEAR·기록 패널 등 v3 장면).
- 실제 GPU 창 확인 절차는 `docs/quality-assurance/2026-09-17-skin-system/checklist.md` §3.
