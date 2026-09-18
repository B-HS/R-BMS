# 스킨 시스템 완성과 기본 스킨 전면 제작 (2026-09-17)

사양: `docs/plan/2026-09-17-skin-system-completion.md`. 결정: `docs/acknowledge/2026-09-17-skin-system-decisions.md`. 검증 체크리스트: `docs/quality-assurance/2026-09-17-skin-system/checklist.md`. 이 문서는 K1~K6 의 실제 진행과 실측 결과를 기록한다(최종 갱신 2026-09-18).

## K1 조사 (Workflow `wf_32fd88cf-707`, Opus 3갈래, 9.6분)

- R1 엔진 격차: 그려지는 객체 8종(image·imageset·value·floatvalue·text·slider·graph·bga), 파싱만 되는 객체 15종. 차단 요소 3건 — 로더가 최상위 destination 만 조립, 상태 소스가 스칼라만 답함, 타이머가 상태 소스로 흐르지 않고 결과·결정 화면에 드라이버 없음. 네이티브 대체는 결과 화면 전용(`result.replace` 4단위).
- R2 앱 배선: 문서 화면 5종 중 합성 가능 3종(select·play·result). 커스터마이즈가 문서 절대경로 키라 5개 플레이 문서가 값을 공유하지 못하고 세대 이동 시 `custom` 이 유실됨. 시스템 사운드는 `audio.sound_folder` 단일 폴더, BGM 루프 없음.
- R3 외부 묶음: 1280×720 환산 좌표 17표(레인 폭 SC 60·백건 33·흑건 27, 곡 바 533×40·행 간격 40·중앙 index 10, BGA 3분기, DP 2필드+중앙 게이지, 결과 판정 행 36), 커스터마이즈 표면 옵션 55·파일 65·오프셋 28, 현재 rbms 캡처 결함 13건.

## K2 사양·결정

- 사양 §1~§12 확정. 결정 D1~D12(3세대 번들 `steel-neon-v3`·라벨 유지, 로더 중첩 트랙, `FrameExtra`, 타이머 드라이버, 문서 `note.dst` 가 레인 단일 원천, 최상위 `replace`, `hotspot`, 번들 스코프 저장, 절차 생성 사운드, 옵션 패널 대체, 문구는 `text`·숫자는 7세그먼트, `densitygraph`).
- 리뷰 반영 정정 2건: 숫자 시트는 렌더러 11셀 규약(0~9+공백, 소수는 11×2 에 소수점), SUDDEN+ 커버는 `offset: 4` 이미지(`hiddenCover` = HIDDEN+, `liftCover` = 리프트 밴드).

## K3 엔진 (Workflow `wf_b03ec12f-3f9`, 14 에이전트, 87분)

- W0 스캐폴드: 모델 `replace`/`hotspot`/`scope`/`densitygraph`, `NestedTracks`, `FrameExtra`·`PlayObjectState`·`SelectListState`·`ResultSeriesState`·`OptionsRows`, `SkinObjectKind`/`Body` 13종, 새 모듈 7개.
- W1-a 로더: 중첩 트랙 조립(note group/bpm/stop/time, judge images/numbers, songlist 9배열+graph), `replace_names()`, `HOTSPOT_ACTIONS` 검증, `CustomFile.scope`. 테스트 14건·픽스처 4개.
- W1-b 플레이 객체: note(네이티브와 동일 y, LN 3분할, 지뢰, hidden 채널, 소절선 트랙), gauge(4/8/12/36 노드 인덱스맵·parts 양자화·종류별 열), judge(중첩 트랙 id 매칭·콤보 x 보정·`shift`·text 이미지 허용·2P 대역), covers(HIDDEN+/리프트).
- W1-c 목록·그래프: songlist(슬롯=rows[sel-center+i], imageset 바, 제목 말줄임, 게이트·클립 반영 히트 사각형), graphs 7종(gauge/judge/bpm/timing dist/timing vis/hit error/density), `parse_hex_color`·`modulate`.
- W1-d 설정·설치·사운드: `SkinOptions.shared`·`bundle_key`·병합 `user_config`, `BUNDLE_GENERATIONS`(v1·v2·v3), 세대 이동 시 `custom`·`shared` 동반 이동, `bundle_is_complete`/`bundle_can_draw` 분리, 번들 `sound/` 해석·프리셋 변경 시 재로드. v3 파일 59개 등록.
- W1-e 자산: `palette.json`, `generate-assets.py`(PNG 21장: notes·digits 4·ui·covers·배경 8·프레임 4), `generate-sounds.py`(22 스템, 572K), 자리표시 문서 10개.
- 적대 리뷰 3갈래 31건(critical/major 19) → 수정 3갈래 반영. 남긴 항목은 K4 브리프로 이관(대체 게이트 앱 배선, 등급 옵션 응답, 레인커버 오프셋, RON 값, 문서 본문).
- 게이트(실측): `cargo fmt --all --check` 통과, `cargo clippy --workspace --all-targets --all-features -- -D warnings` 통과, `cargo test --workspace` 3,118 통과·0 실패·3 ignored, `git diff --check` 통과, 금지 명칭 grep 0건.

## K4 앱 배선·기본 문서 (Workflow `wf_1a582c5d-df7`)

- W2-0: `screen_content(screen)` 게이트 일반화(§7 표를 `replacement(screen, name)` 한 곳에), `MergedOffsets`(번들 공유 → 문서 우선), 핫스팟 표를 `SkinScreen`으로 이동·화면 좌표 변환, `options_rows`·PANEL1 타이머, v3 문서 테스트 파일 4개.
- W2-a play: `PlayObjectState` 조립(keys_down·recent_hits 64·gauge_kind), 문서 `note.dst`로 `Skin` 레인 재구성(D5), `render_hud_with_content`·필드/커버/키봄 생략, `OFFSET_LIFT/LANECOVER/HIDDEN_COVER` 응답, 판정 사이드별 1P/2P 옵션·타이머, 레인별 KEYON/KEYOFF/HOLD/BOMB.
- W2-b select: `render_select_on_background_with_content`(상단 바·목록·상세 생략), 문서 핫스팟→`Hot` 매핑, 옵션 패널 대체 시 네이티브 패널 생략.
- W2-c result: `ResultSeriesState`(gauge/timing/judge/bpm_points 512 상한), `grade/graphs/title` 생략, `ResultTimers`(150/151/152), 등급 옵션 300~307·340~347 응답.
- W2-d SKIN 탭: `BUNDLE > 이름` 행(선택 문서 헤더 합집합), `shared_customise` 저장, 번들 전 화면 stale, RESET은 문서 값만.
- 메인 직접 수정: W2-a가 보고한 "플레이 문서가 사용자 오프셋을 읽지 못함"을 `draw_document`가 `skin_offsets(skin_type)`를 `PlayViewState.offsets`로 넘기도록 고치고 회귀 테스트 `a_play_document_reads_the_nudges_the_player_stored_for_it`를 추가했다(수정 되돌리면 실패, 적용하면 통과 확인).
- 메인 직접 보강 2: 중간 캡처에서 결과 화면의 네이티브 IR 상태 줄과 하단 힌트가 문서 표 위에 겹치는 것을 확인해 대체 단위 `hint`(`result-hint`)·`ir`(`result-ir`)를 추가하고 사설 id 20205(IR 상태 한 줄)·20204(힌트)·20203(선택 힌트)을 상태가 답하게 했다. `draw_result_skin*`는 `ResultExtras`를 받는다.
- 메인 직접 보강 3(문서 갈래 보고에서 드러난 상태 공백): 플레이 상태가 BPM 최소·최대·주(90~92), 경과·남은 시간(161~164), FAST/SLOW 합(423/424), 목표 EX(121)·차이(153)·비율(115/135), 최고 기록 비율(113), 레벨(96), 아티스트(14/16), 목표 이름·차이 문자열(20201/20202)을 답한다. 선택 상태는 통계 6셀(20301~)·기록 5줄(20311~)·최고 기록 비율(113)·힌트(20203)를 답하고 `SelectViewState::new`로 조립한다. 결정 상태는 라이브러리 항목의 부제·아티스트·장르·레벨을 답한다(`DecideChart`). 문서가 `bga` 객체를 그리면 네이티브 BGA 사각형을 생략한다. 이중 필드 RON 폴백이 좌측 BGA 옆에서 40px로 붕괴하던 `Skin::build`를 좌/우 BGA 판별로 고치고 설치 테스트의 "BGA는 오른쪽" 가정을 교차 검사로 바꿨다. 헤드리스 캡처 테스트의 옵션 프로브를 문서 패널 안(300,300)으로 옮기고 F1 뒤 한 프레임을 다시 그린다.
- W3 문서 4갈래: select.json5(songlist 15슬롯·상세·densitygraph·통계·기록·하단 버튼 hotspot 6·옵션 패널 11행, theme.ron 갱신), decide.json5(배경 슬롯·텍스트·진행 바·DECIDE EFFECT), result.json5(9개 대체 단위 전부·등급별 배경·3그래프·그래프 열), play-5k/7k/9k.json5 + shared/objects-play.json5(note·gauge·judge·covers·SUDDEN+ offset 4·키 위젯·HUD·그래프 열, 숫자 시트 11셀 재생성), play-10k/14k.json5 + play-dual.ron(양측 필드·중앙 게이지·좌 BGA 2슬롯·판정 2개).
- 적대 리뷰 2갈래 23건(major 9) → 수정: 게이지 종류 옵션(42/43/1046 등, 레퍼런스 BooleanPropertyFactory 매핑) 응답, `RESULT_UPDATESCORE`는 신기록 시 on, 키봄은 항상 네이티브, `screen_content` 컴파일 시 1회 캐시, `render_decide_screen`이 호출자 상태 유지, 선택 기록 패널(20311~) 배치·통계 셀 1열 재배치·10K 필드 판 폭·숫자 셀 크기 정합·digits-s 획 두께.
- 게이트(실측): `cargo fmt --all --check` 통과, `cargo clippy --workspace --all-targets --all-features -- -D warnings` 통과, `cargo test --workspace` 0 실패(최대 스위트 905), 캡처 테스트 2종(1+30) 통과, 금지 명칭 0건, `images/select/Default.png` 번들 목록 누락 1건 정정.

## K4-b 후속 (Workflow `wf_136680dc-fb5`)

- 단일 플레이 문서(5/7/9키)에 `bga` 객체와 번들 옵션 PLAY SIDE(900/901)·BGA SIZE(910/911/912)·GRAPH POSITION(920/921)·BGA 오프셋 40을 추가했다. 로더의 조건부 원소·include 지원으로 `shared/objects-play.json5`(4조합)와 `shared/judge-sp.json5`로 분기한다. 2P/NEAR 프레임 3장(`frame-sp-2p`, `frame-sp-near`, `frame-sp-2p-near`)을 생성기에 추가했다.
- 7세그먼트 '1'·'0' 글리프를 끊김 없는 세로 막대로 고쳐 LEVEL 11·BPM 120이 판독된다. 선택 통계 2열×3행, 기록 5줄, 결정 레벨 숫자, DP BGA SIZE 사각형 구분, 게이지 종류 라벨(42/43/1046)을 반영했다.
- 게이트(실측): `cargo test --workspace` 3,181 통과·0 실패, fmt·clippy 통과, 캡처 8장, 금지 명칭 0건.
- 메인 직접: 결과 화면 `ResultView.artist`와 `STRING_ARTIST` 응답.

## K5 검증

- 자동 검사와 헤드리스 캡처는 `docs/quality-assurance/2026-09-17-skin-system/checklist.md`와 `captures/`에 기록했다. 메인이 캡처 12장을 직접 열어 장식선 관통·겹침·가림이 없음을 확인했다.
- 실제 GPU 창 확인은 이 세션의 터미널에 화면 녹화 권한이 없어 캡처하지 못했다(앱은 격리 HOME으로 정상 기동·5차트 스캔). 절차는 체크리스트 §3에 적었다.

## K6 문서·핸드오프 (2026-09-18)

- 2026-09-17: `docs/skin.md` 신설, `docs/README.md` 표·`README.md` 스킨 문단 갱신, 이 이력과 QA 체크리스트 작성. 커밋 단위 5건 계획(skin 로더 / render 객체 / config 스코프 / player 배선+v3 자산 / docs)은 사용자 지시 대기.
- 2026-09-18 `/prepare-new`: 코드↔문서 대조에서 결과 `score` 대체 단위 id 6개 미기재, 사양 §9.6 대체 목록에 `hint`·`ir` 누락, §3·§9.1 자산 목록 누락(`shared/judge-sp.json5`·프레임 4종·슬롯 폴더), §11 실제 창 저장 경로 오기, architecture/crates/development/roadmap 의 스킨 서술 구식, PROCESS 테스트 수(3,040)·V7-C/V8 상태 미갱신을 찾아 정정했다. 이전 HANDOFF 본문은 `docs/history/2026-09-10-handoff-snapshot.md`로 옮기고 `docs/HANDOFF.md`를 이번 세션 스냅샷으로 다시 썼다. 결정 D13~D23 을 `docs/acknowledge/2026-09-17-skin-system-decisions.md`에 추가했다. 이 시점의 작업 트리는 변경 48·신규 22(코드·자산) + 문서이며 전부 미커밋, v3 번들 파일 66개가 `assets.rs`에 등록돼 있다.

## 남은 범위

- `frame-dp.png`가 14키 좌표라 10키에서는 프레임이 필드보다 36px 넓다(10키 전용 프레임 생성 필요).
- 선택 화면 SCORE DATA의 판정 6행은 브라우저 상태에 판정별 카운트 id가 없어 생략했다.
- BGA SIZE OFF는 투명 목적지로 소유를 유지한다(엔진이 `bga` 선언 자체로 게이트하면 불필요).
- 플레이 상단 NOTES 수치가 패널 경계에 물리는 미세 배치, `shared/objects-play.json5`는 4조합을 펼친 생성 산출물이라 수정 시 조합별 동기화가 필요하다.
- 곡 선택 BGM 루프·pmchara·practice·객체 `click` 이벤트·외부 CSV 스킨 직접 로드·비디오 BGA는 사양 §12대로 후속이다.
