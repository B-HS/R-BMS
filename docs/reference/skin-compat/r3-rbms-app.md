# R3 — R-BMS 앱(apps/rbms-player) 스킨 배선·화면 구성·입력 현황과 격차

> 최종 갱신 2026-10-11 · 대응 단계: 웨이브 7B(동영상 source) 반영 · 본문은 작성 시점 서술이며, 아래 "웨이브 … 반영 사항"이 최신 것부터 우선한다 · 색인과 갱신 규칙은 [README.md](README.md)

## 웨이브 7B 반영 사항 (2026-10-11)

`crates/rbms-video`(OpenH264 + `re_mp4`)와 스킨 동영상 source 재생을 넣은 뒤의 상태다.

- (W7-2) r3-rbms-app.md: 상단에 '웨이브 7B 반영 사항' 신설 — assets.rs 의 SkinAssetKind::Movie·SkinAsset::{Movie, Unplayable}(워커가 헤더만 읽어 엶), skin_screen/movies.rs 의 MoviePlayers(PLAYING_MOVIES_LIMIT 4, prepare 와 draw 사이에서 show, let_go 때 정지·해제, 실패 시 경고 1회 후 hide), PreparedDocument 가 재생기를 들고 draw 직전에 갱신, 테스트용 scripted_movie 와 wait_for_movie_frames


## 웨이브 7A 반영 사항 (2026-10-11)

비트맵 폰트(.fnt) 로더와 표준·distance field 그리기를 넣은 뒤의 상태다.

- (W7-3) r3-rbms-app.md(스킨 화면 수명 절): SkinScreens 에 font_pages(FontPageDecode) 추가, finish_frame 이 sync_font_pages 를 먼저 부름. 디코드 실패 페이지는 워커 종료 시 '없음'으로 정산해 무한 대기를 막음. 페이지 대기 중에는 scene hold 를 걸지 않음
- (W7-4) r3-rbms-app.md 캡처 하니스 절: capture.rs 에 추가된 `draw_until_text_settles`(프레임마다 finish_skin_frame, 폰트 페이지가 다 들어온 프레임까지), `choose_in_skin`(스킨 옵션 행 선택), 자작 DF 팩 캡처 테스트, ModernChic 결정 화면 비트맵 폰트 양 백엔드 캡처 테스트, `DOCUMENT_LOAD_WAIT`(프레임 예산의 4초 하한)를 기록
- (리뷰 수정) r3-rbms-app.md 캡처 하니스 절(stage/capture.rs 의 draw_until_compiled): 240프레임 예산을 없애고 '컴파일됨 또는 더는 로딩 중이 아님'을 기다리며 60초는 멈춤 방지용이라고 적어야 합니다. 근거(GPU 대상에서 240프레임이 약 8ms, 3배 부하에서 86번째 프레임 도착, 실제 실패는 재현 못 함)도 함께 남겨야 합니다


## 웨이브 6 반영 사항 (2026-10-11)

note·judge·커버·bga·비주얼라이저 재작성, 플레이 상태기계(PRELOAD → READY → PLAY → FAILED/FINISHED), 플레이 타이머 드라이버와 오프셋 1~5, 호스트 군집 C·D, 스킨 경로의 내장 레이아웃 의존 제거와 구 어댑터(`state.rs`)·구 드라이버(`screen.rs`) 삭제를 한 뒤의 상태다.

- (W6-5) r3-rbms-app.md §6.2: 플레이 화면 상태 공급을 갱신합니다. 구 PlayViewState 어댑터 대신 PlayShown::of(&PlaySession, &PlayLive)와 ScreenHost::show_play가 군집 B·C·D·M에 싣는 구조입니다. Stage 연결은 W6-3 대기, 구 어댑터는 폴백만 담당한다고 적습니다.
- (W6-5) r3-rbms-app.md §8.6 테스트 표: skin_host/play/tests.rs 33건, play/shown/tests.rs 9건, score/tests.rs 3건, score/standing/tests.rs 3건을 추가합니다.
- (W6-4) r3-rbms-app.md §6.2: 타이머 표 아래에 '새 드라이버 apps/rbms-player/src/skin_host/play_timers.rs(PlayTimerDriver, SceneEvent 18종, PlaySetup, Reached, PlayOffsets/FieldOffsets) 추가, Stage 연결은 W6-3, 구 PlayTimers 삭제는 W6-8' 을 추가. 주의점의 '레인 → 타이머 키 번호는 내장 Skin.x 를 x 좌표순으로 정렬해 매긴다' 는 구 드라이버 한정이며 새 드라이버는 lane_slots(mode)(LaneProperty 표)로 매긴다고 적음. '공급하지 못하는 것' 표의 TIMER_STARTINPUT, TIMER_RHYTHM, MUSIC_END/ENDOFNOTE/FADEOUT, SCORE_A~TARGET, PM_CHARA 행을 '드라이버 구현(W6-4), 사건 공급 대기(W6-3)' 로 변경
- (W6-4) r3-rbms-app.md §6.1 또는 §6.2: ScreenHost 에 play_offsets: Option<&PlayOffsets> 필드가 생겼고 오프셋 질의 순서가 play_offsets → 군집 → fallback → 사용자 nudge 라는 점 추가
- (W6-2) r3-rbms-app.md §6.2(플레이 프레임 조립): 'PlayViewState 의 judged_side·last_judge 로 판정 표시를 구동'하던 서술을 '스킨 판정 표시는 FrameData.judge(영역별 JudgeHit)와 타이머 46/47/247 로 구동. 앱이 판정마다 영역 = lane / (레인 수 / judgeregion) 로 채워야 하며 현재 미연결'로 갱신. skin_host/play_timers.rs 의 judge_regions 와 rbms-render 의 JudgeFrame::region_count 중복을 기록
- (W6-1) r3-rbms-app.md §6.2: 플레이 화면 스킨 그리기 절에 'draw_play_skin 은 창 픽셀 직접 그리기. W6-3 이 FrameData.notes 를 채우기 전까지 skin_screen.rs 의 lane_notes_of / held_long_notes 가 기존 NoteField 에서 임시 변환'을 추가.
- (W6-1) r3-rbms-app.md §11-4: 스킨 경로의 내장 Skin 의존 목록에서 note 를 빼고, 남은 것은 구 어댑터 state.rs 의 OFFSET_LIFT·커버 오프셋과 게이지 임계값이라고 갱신.
- (W6-7) r3-rbms-app.md §6.2: FrameSeries.recent_hits 의 계약 변경. RecentHits{hits, recorded, judge_area}. hits 는 (오차 ms, 판정), 빠름이 양수(beatoraja mfast/1000), 최근 100개를 읽고 judge >= 4 는 기록하지 않음. judge_area 는 JudgeProperty.getNoteJudge ms 5쌍으로 앱이 채워야 함. 앱의 RECENT_HITS_KEPT(64)는 100 이상이어야 하고 recorded 를 넘겨야 함.
- (W6-6) r3-rbms-app.md §10.2 와 상단 반영 사항: BGA 로드 조건이 내장 Skin.bga 가 아니라 '문서에 bga 객체가 있는가'(wants_bga_pictures)와 display.bga 로 바뀜, Canvas::bga_frame/release_bga_textures 가 단일 background_texture 슬롯을 대체함. 미스 레이어 데이터는 rbms-chart/rbms-model 변경 전까지 없음
- (W6-3) r3-rbms-app.md §6.2: 'Play 는 진입 즉시 곡 시작, PlayTimers.start/update/fail 로 타이머 구동, PlayViewState 어댑터'를 '플레이 스킨이 있으면 on_enter 에서 PlayScene(stage/play/scene.rs) 시작: PRELOAD → READY(40) → PLAY(41, start_play 로 곡 시계 시작) → FAILED(3, close) / FINISHED(908 → finishmargin → 2 → fadeout). 사건은 SkinFeed(stage/play/skin.rs)가 PlayTimerDriver 에 보내고, 프레임 데이터는 LaneNotes·JudgeFrame·GaugeFrame::playing·BgaPlayhead·NoteDistribution(초 단위 판정표)·RecentHits, 속성은 PlayShown. 스킨이 없으면 scene = None 으로 기존 흐름'으로 교체
- (W6-3) r3-rbms-app.md §6.5: 'Decide 는 로드가 끝났으면 Play, 아니면 Loading' 에 '플레이 스킨이 있으면(ChartAssets::preloads_on_the_play_screen) 로드 여부와 무관하게 Play(PRELOAD)로 직행하고 디코드는 ChartLoads 로 넘어간다. 연습 요청은 기존 Loading 경로. LoadingState::Song 은 플레이 스킨이 받을 차트면 한 프레임을 검정으로 그린다' 추가
- (W6-3) r3-rbms-app.md §11: 격차 목록에서 'PRELOAD/READY/폐점 연출 없음', '플레이 문서만 배율 래퍼', '차트 미리보기 141 없음', '폐점 즉시 재시작 없음' 항목을 해결로 표시하고, 남은 격차로 'CONSTANT·LEGACY NOTE 의 스킨 대응 없음, 엔진의 LN 보유 상태 미공개, BACKBMP 미공급' 추가
- (W6-3) r3-rbms-app.md §7.1: 플레이 입력 분배에 '스킨 장면에서는 PLAY 상태에서만 건반을 세션에 전달하고(그 전 눌림은 키빔만), stage/play/controls.rs 가 START/SELECT 조합(하이스피드, 커버, 더블탭, duration, 리프트/히든 전환, 휠)을 매 프레임 읽는다' 추가
- (W6-3) r3-rbms-app.md §1.2: 전이 그래프에 'Decide → Play(스킨 있음)', 'Loading(Song) → Play(PRELOAD)', 'Play(FAILED) → Loading(즉시 재시작)' 간선 추가
- (W6-8) r3-rbms-app.md §6.1: with_skin_frame 에서 older 인자와 ScreenHost.fallback 이 없어졌고, 군집이 모르는 id 는 모든 화면에서 값 없음이라고 수정. FrameInputs 에 keyconfig: KeyConfigState 와 settings: Option<SettingsView> 추가
- (W6-8) r3-rbms-app.md §6.2(Play): AppShared.skin_play_timers·skin_select_timers 와 SkinScene 의 play·select 필드 삭제. 플레이 타이머는 skin_host/play_timers.rs 의 드라이버만 씀
- (W6-8) r3-rbms-app.md §6.3·§6.4: SelectDraw.view, ResultDraw.view·extras·cleared, CourseSkinRun.view·extras·summary_view 삭제. 선곡 노트 수(350~353)와 코스 결과 TOTAL(368)은 이전에 구 어댑터의 0 이 그려졌고 지금은 빈칸이라고 기록
- (W6-8) r3-rbms-app.md §6.5(Decide, KeyConfig): 키 이름은 skin_host/keyconfig.rs 군집 K 가 답함(40+i → i번째 레인, 240+i → 10+i번째). 결정·키 설정 화면은 options 군집에 SettingsView::of_config 를 실음
- (W6-8) r3-rbms-app.md §11-4: 해소로 표시. BGA 디코드 조건은 문서의 bga 유무, 레인·판정 측·봄은 play_timers 와 모드 표, 결과 팔레트는 내장 화면 전용
- (W6-8) r3-rbms-app.md §11(상단 웨이브 5 '리뷰 수정' 항목): SelectTimers 타입 삭제 완료로 표시
- (W6-8) r3-rbms-app.md §8.6 테스트 표: 삭제된 테스트(render_tests.rs 의 타이머 드라이버 기억 2건, skin_render/tests.rs 의 구 드라이버·어댑터 테스트, tests/skin_render.rs 의 게이트·어댑터 4건)와 추가된 테스트(whole.rs 6건, tests/skin_render.rs 3건, keyconfig.rs 3건, skin_screen/tests.rs 의 SKIN 탭 OFFSET_ALL 1건과 팩 선택 테스트 1건) 반영
- (리뷰 수정) r3-rbms-app.md §6.2: 플레이 화면 절에 반영 — RunTrace 는 노트 상태 비교가 아니라 엔진 판정 로그를 소비(빈 POOR 레인 추정 삭제), 풀콤보는 스테이지 콤보(이월 콤보 제외), 폐점·종료 중 뗌 입력 미전달, 자동 레인은 세션 빔 상태 OR, 노트 필드 시각은 한 번 절삭, 스킨 실패 시 PRELOAD 는 내장 LOADING 표시
- (리뷰 수정) r3-rbms-app.md §11-4(스킨 경로의 RON 의존 제거 절): SceneEvent::PracticeRestarted 와 PlayPhase::Practice/PracticeFinished 삭제 반영. 옵션 1080(OPTION_STATE_PRACTICE)은 항상 false


## 웨이브 5 반영 사항 (2026-10-10)

songlist 재작성, 선곡 상태 모델, 선곡 입력 키 표와 장면 수명, 패널 1~3 과 옵션 이벤트, 호스트 군집 F·H(선곡)·E, 슬라이더 쓰기와 편집 텍스트, 선곡 사운드, 스킨 위 시스템 오버레이를 넣은 뒤의 상태다.

- (W5-6) r3-rbms-app.md §10.1 '재생' 줄과 상단 '웨이브 4 반영 사항'의 루프 API 설명: play_loop가 SystemSoundSet::play_loop와 AppShared::play_system_sound_loop, play_result_sound, drive_select_bgm(SelectBgm 상태기계)으로 돌아왔다고 갱신합니다. AudioEngine::set_effect_level(Command::EffectLevel)이 추가됐습니다. 믹서 보이스에 level/level_target/level_step 필드가 생겼습니다.
- (W5-1) r3-rbms-app.md §6.3: skin_screen.rs 의 select_list 가 SelectRow 를 그대로 넘기던 서술을 'select_bars 임시 변환기(SelectRow → SongBar, 폴더 여부·제목·레벨만)로 교체됨, W5-2 가 선곡 모델에서 SongBar 를 공급하도록 대체 예정' 으로 갱신. 새 액션 SelectBar/CloseBar 가 run_skin_actions 에서 무동작이라는 점 추가
- (W5-5) r3-rbms-app.md §7.5 '텍스트 입력' 행을 구현 완료로 바꾸고 §2.1 SkinScreens.input 의 SkinInput 필드에 text: Option<TextSession>, text_writes, settings_dirty 를 추가. §7 입력 분배에 Stage::handle_key 가 skin_text_key 를 가장 먼저 부른다는 내용과 lib.rs 의 WindowEvent::Ime → skin_text_ime, App::sync_ime → AppWindow::sync_ime 를 적는다. textedit.rs 의 ImeEdit(TextEdit 는 Stage 크기 예산 때문에 늘리지 않음)와 skin_host/writers.rs 의 TextSession 을 추가
- (W5-5) r3-rbms-app.md 상단 웨이브 4 반영 사항의 '편집 텍스트 포커스 … 뒤 웨이브다'와 W4-1 의 '텍스트 입력은 FocusText 판정만' 문장을 구현 완료로 교체. skin_screen/tests.rs 의 슬라이더 테스트가 큐 대신 config.audio 를 확인하도록 바뀜
- (W5-2) r3-rbms-app.md 상단: '웨이브 5 반영 사항(W5-2)' 절 추가 — 선곡 목록 모델 SelectBar(lib.rs), BarList·row_of·wheel_bar(stage/select/scene.rs), ChartFacts·select_bars·course_bars(list.rs), 참조 이미지 워커 SongImages(images.rs), draw_select_skin(canvas, &SelectDraw{view, data})
- (W5-2) r3-rbms-app.md §6.3: '공급 지점' 문단을 'SelectState::draw 가 FrameData{bars, images, bga} 를 만들어 draw_select_skin(SelectDraw) 로 넘긴다'로 교체. '행 SelectRow … 램프가 색 값이지 램프 id 가 아니다' 문장은 '행은 SelectBar 모델에서 row_of 로 내려 만들고, 모델의 lamp 는 원본 ClearType id(Option<u8>)'로 교체
- (W5-2) r3-rbms-app.md §6.3 '공급하지 못하는 것' 표: '바 종류 구분' 행 → '모델에 BarKind 로 구분됨(Song·Folder·Table·Course). Executable·RandomCourse·Command·Search·곡 없음은 원천 없음'. '코스 바' 행 → '코스 탭이 Course{complete} 바로 공급됨(구성 곡 특성 포함). 코스 램프·트로피는 로컬 코스 기록이 없어 미공급'. '배너와 스테이지파일을 별도 이미지로' 행 → '스킨 장면에서 -100·-102 를 커서 곡 기준으로 따로 공급, -101 은 마지막 로드 차트의 BACKBMP(원본과 같은 규칙). 내장 화면은 기존대로 1장'. '라이벌' 행에 'SongBars.rival 은 항상 false' 추가
- (W5-2) r3-rbms-app.md §6.3 타이머 표 아래: 'SONGBAR_CHANGE(11)가 장면 시작 시 켜지지 않아 ModernChic 의 스테이지 파일·배너·제목이 첫 커서 이동 전에는 숨겨진다(W5-3a)' 추가
- (W5-2) r3-rbms-app.md §8.6 테스트 표: stage/select/bars_tests.rs 14건과 capture.rs 의 the_browser_of_a_skin_pack_named_by_the_environment_is_captured_over_a_scanned_library 추가
- (W5-3a) r3-rbms-app.md §6.3 타이머 표: '매 프레임 skin_select_timers.update → SONGBAR_MOVE/MOVE_UP/MOVE_DOWN/SONGBAR_CHANGE' 행을 '스킨 장면에서 stage/select/skinned.rs 의 run_scene_timers 가 STARTINPUT(1, input 경과 뒤), SONGBAR_CHANGE(11, 첫 입력 프레임 on, 커서 이동·목록 재구성·스크롤바 쓰기·LN 모드 변경 시 재시작), IR_CONNECT 172~174(ranking_cache 상태)를 켠다. 10/12/13/14 는 켜지 않는다. SelectTimers 는 호출부가 없다(타입 삭제는 W6-8)'로 교체
- (W5-3a) r3-rbms-app.md §6.3 '공급하지 못하는 것' 표: TIMER_STARTINPUT 행을 '구현(W5-3a)'으로 바꾸고 괄호의 'skin.input 이후 입력 허용'을 '타이머만 켠다. 원본 선곡은 입력을 막지 않는다(B:MainController.java:495-498)'로 정정. IR 상태 타이머는 '구현(패널이 채운 캐시 기준, 자동 조회 없음)', 모드 필터·LN 모드·정렬은 '키 1/2/3 과 곡 바 우클릭(루트)에서 구현, id 공급은 W5-4'로
- (W5-3a) r3-rbms-app.md §7.1 분배 표와 §7.4 끝 문단: 'R-BMS select 는 레인 키와 START/SELECT 개념이 없다'를 '스킨 장면에서는 원본 키 표(stage/select/keys.rs)를 프레임당 1회 상태로 읽는다. 키보드 레인 키와 패드 모두 key_index_pressed 로 읽고, 프레임 사이의 탭은 handle_key/handle_pad 가 래치한다. 휠은 handle_scroll → BarScroller, 곡 바 클릭은 AppShared::take_skin_bar_presses 로 Stage 가 실행한다. 스킨 없는 선곡은 기존 이벤트 방식 그대로'로
- (W5-3a) r3-rbms-app.md 상단 웨이브 3B 반영 사항의 '스킨을 기다리는 동안 Select 는 내장 화면을 그렸다가 스킨으로 바뀐다'와 사양 웨이브 3 구현 메모의 같은 문장: 'Select 는 스킨을 기다리는 동안 검정을 그리고 입력을 받지 않는다. 10초 한도 초과나 읽기 실패 시 내장 화면. 스킨 없이 프레임을 그린 뒤 스킨이 처음 그려지는 프레임에 장면 시계 0'으로
- (W5-4) r3-rbms-app.md §6.3(선곡 어댑터): 구 SelectViewState 의 답은 군집이 모르는 id 의 폴백일 뿐이며, 선곡 프레임은 AppShared::select_shown(skin_host/select/bridge.rs)→SelectShown::of(순수)→ScreenHost::show_select 로 군집 F·E·H 에 전달된다고 갱신. FrameInputs.select 필드와 선곡에서도 IR 이름(1020·1021)을 호스트에 싣는 점 추가.
- (W5-3b) r3-rbms-app.md 상단: '웨이브 5 반영 사항(W5-3b)' 추가 — stage/select/panel.rs(PanelState, show_panel, panel_keys, press_act)와 stage/select/events.rs(EVENTS 표, step_setting, carry_out_skin_events, run_event) 신설. SelectState 에 panel, options_dirty, autoplay_put_back 필드. SelectDraw 에 panel: u8 과 mode_filter: Option<Mode>. 갱신 순서는 skin_input_frame → carry_out_skin_events → settle_option_changes
- (W5-3b) r3-rbms-app.md §6.3 타이머 표: '옵션 오버레이 열고 닫을 때 PANEL1_ON, PANEL1_OFF (app_options.rs:207-211)' 행 삭제. 'START/SELECT/숫자 5 홀드로 패널 n 이 바뀔 때 PANELn_ON(21~23)과 PANELn_OFF(31~33)를 setPanelState 대로 전환(stage/select/panel.rs show_panel)'으로 교체
- (W5-3b) r3-rbms-app.md §6.3 '공급하지 못하는 것' 표: '패널 2·3' 행과 '패널 내 개별 옵션 값' 행을 구현됨으로 변경(옵션 21~23 은 SelectShown.panel, 값은 skin_host/options.rs SettingsView, 쓰기는 events.rs). 남는 것은 duration, 커스텀 판정 토글, 판정 영역, 마크 노트, BPM 가이드, 지뢰 모드, 자동 저장 조건, HS 자동 조정, constant
- (W5-3b) r3-rbms-app.md §6.3 '리플레이 슬롯 4개와 선택된 슬롯' 행: '슬롯 n = 그 곡의 리플레이 있는 기록 중 n 번째 최신(events.rs replay_in_slot, bridge.rs replays_stored). 선택된 슬롯 모델은 없음(항상 0)'으로 갱신
- (W5-3b) r3-rbms-app.md §6.3 옵션 패널 행 서술: F1 오버레이 11행은 시스템 오버레이로 유지되고 스킨 패널 타이머와 무관함을 명시(app_options.rs 모듈 문서)
- (W5-3b) r3-rbms-app.md §7.4(레퍼런스 select 키 계약): 패널 1~3 키 표가 구현됨. 패널 1 타깃 스크롤은 스크래치·휠·방향키로 동작하고 패널 3 DURATION 홀드는 미구현(대응 설정 없음). 숫자 6 → 키 설정, F12 → 설정 SKIN 탭 추가. 건반 5(AUTO)는 1회 오토플레이
- (W5-3b) r3-rbms-app.md §7.5 또는 §8.6 테스트 표: stage/select/panel.rs 내부 9건, panel_tests.rs 15건, events_tests.rs 21건 추가. skinned_tests.rs 의 key_five_and_key_six_are_taken_and_start_nothing 은 key_five_plays_the_chart_by_itself_once 와 key_six_is_taken_and_starts_nothing 으로 나뉨. capture.rs 선곡 캡처에 select-panel0~3, select-panel1-stepped, select-panels-gone 추가
- (W5-7) r3-rbms-app.md §6.3: 스킨 선곡 위 시스템 오버레이를 추가한다. SelectState::draw 가 draw_select_skin 직후 overlay.rs::draw_skin_overlays 를 불러 빈 목록 안내 카드, 검색 상자, 코스 탭 표시, 필터 패널, 랭킹 패널, 단축키 안내(guide.rs, H), 기록 모달을 1280x720 래퍼로 그린다고 적는다. 공급하지 못하는 것 표의 'IR 랭킹 … 내장 랭킹 패널로만 존재'와 '검색 텍스트 입력 … 표시용 문자열만' 행을 오버레이로 표시됨으로 바꾸고, 호스트가 search 텍스트에 빈 문자열을 답한다는 점을 명시
- (W5-7) r3-rbms-app.md §7.2: Hot 열거가 14종에서 16종(OverlayPanel, GuideClose)이 되었고, 스킨 화면의 앱 오버레이는 hot 영역을 등록해 skin_pointer 가 물러난다는 서술로 교체. '문서가 그리는 화면은 hot 영역을 등록하지 않음'은 낡음
- (W5-7) r3-rbms-app.md §7.4와 §7.5: R-BMS 선곡 단축키 표에 H(안내)를 추가하고 단축키 안내 항목을 구현 완료로 갱신. 키 선택은 H(충돌 없음 확인), F5~F7 은 노트북 Fn 때문에, F8·F10·F11 은 beatoraja 키라서 제외
- (W5-7) r3-rbms-app.md §8.5 표: 옵션 오버레이, 기록 모달, IR 랭킹 패널, 필터 패널, 검색 상자, 코스 탭, 토스트·연결 점·디버그 패널 행의 선택지를 '시스템 오버레이 유지'로 확정하고 구현 위치(stage/select/overlay.rs, guide.rs, lib.rs draw_overlays)를 적는다. §8.6 테스트 표에 overlay_tests.rs 17건과 capture.rs 의 오버레이 캡처 테스트를 추가
- (W5-7) r3-rbms-app.md 토스트·연결 점·디버그 패널 절(lib.rs draw_overlays): 선곡이 스킨 문서로 그려질 때 점은 (4,8), 디버그 패널은 (6,144) 로 옮긴다는 규칙. 내장 화면은 (CW-22,10)과 (6,6) 그대로. 토스트는 우하단 유지(하단 인셋 44)
- (리뷰 수정) r3-rbms-app.md §6.3: 선곡 프레임의 상태 공급을 고칩니다 — 군집 A 는 SelectDraw.chart 로 받고(곡 바 = 라이브러리 엔트리 + songdb ChartFact + ChartDetail, 폴더·코스 = 빈 슬롯), 군집 F 는 BrowserLent(패널, 모드 필터, 코스, 선택 리플레이 슬롯, 커서 곡 최고 기록과 리플레이 수)로 받습니다. 최고 기록은 프레임마다가 아니라 커서·기록 수가 바뀔 때 Stage 가 읽습니다(stage/select/lent.rs)
- (리뷰 수정) r3-rbms-app.md §7.4: 스킨 선곡의 키 계약을 고칩니다 — 화살표 위·아래는 턴테이블과 같은 유지 키 경로(BarScroller, 300ms 뒤 50ms), 숫자 4 와 건반 6 은 NEXT_REPLAY, 건반 7 은 선택 슬롯의 리플레이(없으면 플레이), 패널 키가 눌린 동안 레인 키는 내장 단축키로 내려가지 않습니다
- (리뷰 수정) r3-rbms-app.md §11: SelectTimers(skin_select_timers)가 더는 구동되지 않는 상태로 남아 있고 W6-8 에서 지운다는 것을 적습니다


## 웨이브 4 반영 사항 (2026-10-10)

스킨 입력 디스패치와 이벤트 실행기, gauge·gaugegraph·timingdistributiongraph·judgegraph type 1·2, 플레이 엔진 기록 확장, Result·CourseResult Stage 의 장면 수명, 호스트 군집 B·G·H·E 일부, 스킨 사운드 버스를 넣은 뒤의 상태다.

- (W4-3) r3-rbms-app.md §6.4 '공급하지 못하는 것' 표의 '게이지 종류 전환 … 게이지 로그도 1종만(app_result.rs:124)' 행: '기록은 생겼다 — AppShared.run_records 의 PlayRecord.gauge_log 가 9종 500ms 이력과 폐점 0 채움을 갖는다. 종류 전환 입력과 그래프 연결은 W4-4·W4-5. ResultView.gauge_series(1종 1초)는 내장 화면용으로 유지'로 교체
- (W4-3) r3-rbms-app.md §6.4 본문: 'enter_result 가 play.record() 를 keep_run_record 로 AppShared.run_records 에 둔다(단곡은 교체, 코스는 스테이지별 누적, 연습은 제외)'를 추가하고, 상단에 '웨이브 4 반영 사항(W4-3)' 절을 만들어 같은 내용과 AppShared 필드 표의 run_records 를 적음
- (W4-3) r3-rbms-app.md §8.6 테스트 표: rbms-play record/tests.rs 17건, rbms-judge tests.rs 4건, app_result.rs 2건(앱 2건은 통합 뒤 실행 확인 필요) 추가
- (W4-1) r3-rbms-app.md §7.1: 마우스 분배 순서를 '옵션 패널 열림 → AppShared::skin_pointer(스킨 객체가 소비) → Stage::handle_mouse/handle_mouse_drag/handle_scroll' 로 고칩니다(pointer.rs route_pointer). release 와 휠은 스킨이 받지 않습니다
- (W4-1) r3-rbms-app.md §7.2: '문서가 그리는 화면은 hot 영역을 등록하지 않음' 뒤에 추가합니다 — 스킨 화면의 클릭 판정은 직전 프레임의 SkinInputMap(SkinScreens.input)으로 하고, 앱이 등록한 hot 영역이 커서 아래 있으면 스킨보다 앱이 먼저 받습니다
- (W4-1) r3-rbms-app.md §7.5 표: '객체 클릭(skin_click 신설)' 행을 구현 완료로 바꿉니다 — AppShared::skin_pointer(at, PointerInput) -> bool, 실행은 다음 프레임 바인딩 안의 run_skin_actions, 결과 요청은 ctx.shared.skin_requests().take(Cluster). 슬라이더 드래그·호버도 구현, 텍스트 입력은 FocusText 판정만, 곡 바 클릭은 웨이브 5
- (W4-1) r3-rbms-app.md §2.1 '컴파일 캐시' 행: SkinScreens 필드에 input: RefCell<SkinInput>{map, fresh, actions, calls} 와 requests: RequestQueue 를 추가합니다
- (W4-1) r3-rbms-app.md §6.1: with_skin_frame 이 host.take_calls() 를 거두고(스킨 읽기 프레임 포함) finish_skin_frame 이 dispatch_calls 로 분배한다는 내용을 추가합니다. 'Nothing carries those out yet' 에 해당하는 서술은 삭제합니다
- (W4-1) r3-rbms-app.md 상단: '웨이브 4 반영 사항'에 W4-1 을 추가합니다 — 키 질의는 app_input.rs 의 GDX_KEYS(90개)·SkinKeys·AppShared::skin_keys, ScreenHost.keys: Option<&dyn HeldKeyQuery>
- (W4-7) r3-rbms-app.md §10.1 '재생' 줄: '루프·정지 API 가 없다' 를 삭제하고, SystemSoundSet::play_loop/stop/stop_loops/looping_sounds 와 AppShared::play_system_sound_loop/stop_system_sound/stop_system_sound_loops 가 생겼으며 루프는 AudioEngine::play_effect(Bus::System, id, gain, looped=true) 로 재생한다고 적는다. 반복 중인 시스템 사운드는 begin_skin_scene 에서 stop_system_sound_loops 로 멈춘다.
- (W4-7) r3-rbms-app.md §10.1 'ModernChic 쪽 실측' 마지막 줄: '스킨 발 재생을 받을 버스·id 네임스페이스·로더가 없다' 를 갱신한다. 이제 skin_host/audio.rs 의 SkinSounds 가 System 버스, SKIN_SOUND_NAMESPACE(0x00A0_0000, 길이 0x1000), 워커 스레드 디코드로 처리한다. 상단 '웨이브 4 반영 사항' 에 AppShared.skin_sounds 필드와 begin_skin_scene → end_scene_sounds, carry_out_skin_calls → settle_skin_sounds 연결을 추가한다.
- (W4-7) r3-rbms-app.md §9 표 E1(사운드 확장): 루프·정지·스킨 발 버스는 완료, 스킨 팩 사운드 폴더와 select BGM 호출은 웨이브 5 로 갱신한다.
- (W4-4) r3-rbms-app.md §6.4: 공급 지점을 ResultState::draw → draw_result_skin(canvas, &ResultDraw { view, extras, cleared, chart: Option<&ChartMeta>, scene: skin_host::ResultScene, data: FrameData })로 교체. ResultSeriesState 서술 삭제. ResultRun(stage/result.rs)이 PlayRecord 에서 9종 게이지 이력·경계, 301칸 타이밍 분포 + 판정 폭(JudgeWindowSet::for_mode 의 note, ms), 초별 판정표를, ChartOverview(플레이된 모델)에서 노트 종류 분포·BPM 변화·스테이지 파일을 FrameData.gauge(GaugeFrame::finished)·series·images 로 싣는다고 기록
- (W4-4) r3-rbms-app.md §6.4 '공급하지 못하는 것' 표: STARTINPUT·입력 잠금·scene 자동 종료·FADEOUT 후 전환, RESULTGRAPH_BEGIN/END 동시 on, RESULT_UPDATESCORE(ranktime 0 고정), 게이지 종류 전환, IR_CONNECT 타이머 172~174, 리플레이 슬롯 상태(1번 슬롯 = 자동 저장 결과), RESULT_CLOSE 를 fadeout 시작에 재생 행을 '구현(W4-4)'으로 변경. 남는 행: 이전 스코어 전체·IR 순위 값(W4-5 연결 대기), 결과 BGM 루프(설정 없음), 코스 결과
- (W4-4) r3-rbms-app.md §6.4 타이머 문단: 'on_enter 에서 skin_result_timers.enter, 1000ms 뒤 END' 를 'on_enter 와 스킨 첫 프레임, 이후 매 update 에서 150·151·152 를 switch(true). ResultTimers(skin_result_timers)는 Result Stage 가 더는 쓰지 않음' 으로 교체
- (W4-4) r3-rbms-app.md §1.2 전이표 Result 행: 스킨 없음 = 키 즉시(기존). 스킨 있음 = input 뒤 확인(키 인덱스 0~3·4·6, Enter, Esc) 또는 R/N → FADEOUT → fadeout 초과 시 Back/Quit(leave_play), To(Loading::song)(재도전·다음 곡). 스킨 대기 중 Esc 는 즉시 leave_play
- (W4-4) r3-rbms-app.md 상단 웨이브 3B 반영 사항의 '스킨을 기다리는 동안 Select·Play·Result 는 내장 화면을 그렸다가 스킨으로 바뀐다': Result 는 스킨이 오는 동안 검정을 그리고 Esc 만 받는다로 수정(스킨 실패·10초 초과 시 내장 폴백)
- (W4-4) r3-rbms-app.md §6.6: scene_life 사용처에 Result 추가(SceneTimes::of_skin, advance, takes_input, begin_fadeout)
- (W4-4) r3-rbms-app.md §8.6 테스트 표: stage/result/tests.rs 31건(자작 픽스처 stage/result/fixture/result.luaskin) 추가. stage/capture.rs 는 PACK_SCREENS 3개(result 제외)와 결과 장면 전용 캡처 테스트(클리어·실패 x 0/1000/3000ms/menu2/fade) 추가. rbms-player lib 테스트 1227건
- (W4-4) r3-rbms-app.md §7(입력) 또는 §11: AppShared.retry_seed(결과 화면의 같은 배치 재도전이 다음 load() 한 번에 넘기는 시드. 리플레이 로드에서는 무시)와 skin_host::ResultScene / ScreenHost.result_scene(결과 화면이 스스로 가진 상태: gauge_type, replay[4], ranking_offset, ranking_total) 추가를 기록
- (W4-5) r3-rbms-app.md §6.4 Result: 결과 화면 상태 공급이 구 ResultViewState(skin_render/state.rs)에서 ScreenHost 군집(B·G·H·E) + ResultSnapshot 으로 옮겨감을 기술하고, 구 어댑터는 폴백만 담당(W6-8 삭제 대상)임을 명시. IrStatus 는 문자열만 갖고 숫자 순위는 없다는 점, IR 타이머 172~174 의 phase 매핑(IrLink::timers).
- (W4-6) r3-rbms-app.md §1.4 화면 대응표 COURSERESULT(15) 행: '스킨 호출 없음, 내장 텍스트만'과 '타입 15 는 로더가 지원 목록에 없음'이 낡았다. 이제 stage/course_result.rs 가 skin_screen::draw_course_result_skin 으로 type 15 문서를 그리고, rbms-skin loader.rs SCREEN_SKIN_TYPES 에 SKIN_TYPE_COURSE_RESULT 가 들어갔다. 스킨이 없으면 내장 행 목록 화면.
- (W4-6) r3-rbms-app.md §1.1·§8.6: CourseResultState::of 시그니처가 of(run, &AppShared)로 바뀜(스킨이 선택돼 있을 때만 CourseSkinRun 을 만든다). stage/course_result/{skin_run.rs, tests.rs, fixture/course.luaskin} 추가, 테스트 17건. skin_host/result/course/tests.rs 10건. app_play.rs 에 chart_model.
- (W4-6) r3-rbms-app.md skin_screen.rs 절: FrameInputs 에 result: Option<&ResultSnapshot> 추가, with_skin_frame 이 스냅샷이 있으면 host.show_result(snapshot, scene)을 부른다. CourseResultDraw 와 draw_course_result_skin 신설, draw_result_skin 은 draw_result_document(screen, …) 공통 헬퍼로.
- (W4-6) r3-rbms-app.md §6.4(결과 화면 상태 공급): 코스 결과는 ChartMeta.notes 를 코스 전체 노트 합, heading 을 코스 이름으로, sheet 를 코스 합계 ScoreSheet, course_titles·course_clear 로 같은 ResultSnapshot::of 에서 만든다. 단일 결과는 아직 스냅샷을 FrameInputs 로 넘기지 않음.
- (리뷰 수정) r3-rbms-app.md §6.4: 'on_enter 에서 skin_result_timers.enter(view, now) … 매 프레임 update(now) 1000ms 뒤 RESULTGRAPH_END' 서술 삭제. ResultTimers 와 AppShared.skin_result_timers 는 없어졌고, 결과·코스 결과는 stage/result.rs 의 begin_scene_with_skin(스킨 첫 프레임에 장면 시작, 시계 0, 150~152 on)과 run_scene_frame(IR 타이머, 장면 step, 페이드 시작 감지)을 공유한다고 교체
- (리뷰 수정) r3-rbms-app.md §6.4: 단곡 결과의 상태 공급 경로 추가 — enter_result 가 scores.push 이전에 PreviousScore::of_book 을 읽고 ResultRun::of(session, shared, RunStanding{replay_saved, lamp, previous, target}) 가 ResultSnapshot 을 만들어 ResultDraw.run 으로 넘긴다. draw_result_skin(canvas, screen, &ResultDraw) 하나가 type 7·15 를 그리고 CourseResultDraw·draw_course_result_skin 은 없다
- (리뷰 수정) r3-rbms-app.md §6.4: IR 공급 추가 — with_skin_frame 이 결과·코스 결과에서 host.ir.link = IrLink::new(has_primary_ir_server, IrPhase::from(&ir_status)) 와 service_name(주 프로필 라벨)·user_name(submission_player_id)을 매 프레임 채운다(Lua 로드 전). 타이머 172~174 는 IrLink::timers() 한 곳에서 두 화면이 쓴다. 코스 결과는 마지막 스테이지의 ir_status 를 쓴다는 한계 명시. 상단 '결과 문서에 IR 상태는 공급되지 않음'(웨이브 1A 반영 사항 W1-1b) 문장은 낡음
- (리뷰 수정) r3-rbms-app.md §2.1(234행 표): 타이머 행의 `skin_result_timers` 삭제, '화면별 드라이버 3개' 를 2개(PlayTimers, SelectTimers)로
- (리뷰 수정) r3-rbms-app.md 웨이브 1B 반영 사항(W1-5, 77행): '드라이버 기억(PlayTimers/SelectTimers/ResultTimers) 초기화' 에서 ResultTimers 삭제
- (리뷰 수정) r3-rbms-app.md §10.1(668행)과 00-synthesis.md S11·T9: 시스템 사운드는 1회 재생 + 정지(SystemSoundSet::stop, AppShared::stop_system_sound)까지 있고 루프 API 는 없다. play_loop 는 호출부가 없어 웨이브 4 리뷰에서 삭제, 선택 BGM·결과 루프 설정은 웨이브 5 에서 호출부와 함께 추가한다고 고침


## 웨이브 3B 반영 사항 (2026-10-10)

스킨 텍스처 관리자(참조 source 만 로드, 화면 이탈 해제, 예산)와 Decide Stage(장면 수명 헬퍼, 백그라운드 차트 로드)를 넣은 뒤의 상태다.

- (W3-7) r3-rbms-app.md §1.1: Stage 열거가 11종(Decide 추가, Box<DecideState>), StageId::Decide 와 라벨 'Decide', StageId::ALL 11개
- (W3-7) r3-rbms-app.md §1.2 전이표: Select Enter/흰 건반 곡 행은 결정 스킨이 있으면 Open(Decide), 없으면 Open(Loading::song)(AppShared::decided_song_stage). F4 연습도 같은 규칙. 기록·IR 리플레이 재생은 load() 뒤 decided_chart_stage(스킨 없으면 기존 enter_loaded_chart). 코스는 load_course_stage 가 run.index == 0 일 때만 Decide
- (W3-7) r3-rbms-app.md §1.2 전이표에 Decide 행 추가: FADEOUT 경과 > fadeout 이고 로드 완료면 To(Play) 또는 To(Practice), 미완료면 To(Loading::waiting_on). 취소면 Back(코스는 end_course). 스킨 로드 실패면 To(Loading). Loading(assets) 행은 'Decide 가 이미 울렸으면 Decide 큐 생략(ChartAssets.announced)' 으로
- (W3-7) r3-rbms-app.md §1.4 화면 대응표 DECIDE(6) 행: 대응 Stage 를 Loading 에서 Decide(stage/decide.rs)로. '스캔·테이블 받기 중에도 같은 문서를 그린다'는 서술 삭제. Loading 은 항상 내장 화면
- (W3-7) r3-rbms-app.md §6.5: draw_decide_skin(canvas, &DecideDraw { chart: &ChartMeta, progress, data: FrameData }) 로 교체. 곡 상태는 skin_host/overview.rs 의 ChartOverview(셔플 전 모델에서 load() 가 계산해 LoadedChart.overview 로 전달)가 군집 A 와 FrameSeries(notes.kinds, bpm of_chart)로 공급. 스테이지 파일은 진입 시 동기 디코드해 Canvas::background_texture 로 올려 ReferenceImages.stagefile 에 실음. 로딩 군집은 LoadingScreen::Elsewhere(옵션 80/81 둘 다 off). 타이머 1·2 사용. DECIDE 사운드는 '로딩 완료 시'가 아니라 '스킨 첫 프레임(장면 시작)'
- (W3-7) r3-rbms-app.md §6.6: '앱은 input/scene/fadeout 을 읽는 곳이 없다'를 'stage/scene_life.rs 의 SceneTimes::of_skin 이 LoadedSkin.def.input/scene/fadeout 을 읽고 advance/takes_input/begin_fadeout 이 STARTINPUT(1)·FADEOUT(2)을 켠다. 현재 사용처는 Decide 뿐, 결과·선곡·플레이는 뒤 웨이브'로
- (W3-7) r3-rbms-app.md §8.6 테스트 표: stage/decide/tests.rs 19건, stage/scene_life.rs 6건, skin_host/overview/tests.rs 6건 추가. render_tests_document.rs 는 'score and decide screens' 로 이름이 바뀌고 Loading 이 decide 문서를 그리지 않는다는 테스트 1건 추가. capture.rs 는 PACK_SCREENS 4개(decide 제외)와 결정 장면 전용 캡처 테스트(실차트 7시점 + 자작 차트 2시점) 추가. rbms-player lib 테스트 1084건
- (W3-7) r3-rbms-app.md 상단: '웨이브 3B 반영 사항' 절 추가(Decide Stage, scene_life, 로딩 분리, ChartOverview, stage_owns_chart_audio 에 Decide 포함, app_play::loaded_chart_for_tests)
- (W3-4) r3-rbms-app.md 상단: '웨이브 3B 반영 사항'에 W3-4 추가 — skin_screen.rs 에 AppShared::skin_is_loading, finish_skin_frame, debug_skin_texture_line 추가. skin_now_us 는 문서가 오는 동안 멈춘다. lib.rs frame() 이 stage.draw 뒤에 finish_skin_frame 을 부른다
- (W3-4) r3-rbms-app.md §2.1 표 '컴파일 캐시' 행: SkinScreens 필드가 { built, pending, textures: SkinTexturePool, stamps, prepared, last_prepared, parked, scene_moved, hold, hold_spent } 로. SkinScene 에 parked: Arc<()> 추가
- (W3-4) r3-rbms-app.md §2.2 5번: PendingScreen::start 가 document.sources 전부가 아니라 referenced_source_files 만 요청하고, 올라와 있는 파일은 pin 한 뒤 stat 요청(SkinAsset::Unchanged)으로 보낸다. assets.rs 의 타입이 SkinAssetRequest/SkinAssetRead/FileStamp 로 늘었다
- (W3-4) r3-rbms-app.md §2.2 6번: 컴파일은 SkinScreen::build_shared(풀 사용). 7번 '그 사이의 화면': 내장 레이아웃이 그려지는 것은 그대로이나 그동안 장면 시계가 멈추고(SCENE_HOLD_LIMIT 10초), Decide 는 W3-7 이 검정을 그린다
- (W3-4) r3-rbms-app.md §2.2 8번 '캐시 수명': '타입별로 앱 수명 동안 유지' 서술과 상단 W2-8 의 관련 항목을 '화면을 떠나면 해제, Open 중 유지, 같은 파일은 재디코드 없이 재사용, 파일이 바뀌면(수정 시각·길이) 다음 읽기에서 재디코드'로 교체
- (W3-4) r3-rbms-app.md §8.6: 테스트 추가 — skin_screen/texture_tests.rs 10건(선택 실행 1건 포함), assets.rs 1건, rbms-render textures/tests.rs 8건. rbms-player lib 테스트 1085건
- (W3-4) r3-rbms-app.md §11(위험): '1920x1080 스킨 10종을 전부 상주시키면 GPU 메모리 사용이 크다' 항목을 해소로 표시
- (리뷰 수정) r3-rbms-app.md §1.1·§1.2(및 상단 W1-5 반영 사항): 'Open 은 아래 화면을 주차하고 Back 이 복원한다'에 '주차된 화면의 컴파일 결과와 텍스처는 그 위에서 To 전이로 새 장면이 시작되면(begin_skin_scene → SkinScreens::unpark) 다음 프레임 끝에 해제된다. 선곡 → Open(Decide) → To(Play) 가 대표 경로이고, 주차된 장면(타이머·경과 시간)은 남아 Back 때 복원되며 스킨은 그때 다시 읽고 컴파일한다. To 없이 Back 하는 Open(설정, 결정 취소)은 유지' 추가
- (리뷰 수정) r3-rbms-app.md §2.2 8번(캐시 수명, W2-8 반영 사항): 'begin_skin_scene 은 expire_scripted 에 더해 주차 목록을 비운다. 스킨 종류(Lua·JSON)와 무관하게 주차 화면이 해제된다' 추가
- (리뷰 수정) r3-rbms-app.md §6.5 Decide: '스킨을 SCENE_HOLD_LIMIT(10초) 넘게 기다리면 AppShared::skin_wait_is_spent 로 Gone 처리해 Loading 으로 넘긴다. Reading 중 Esc 또는 START+SELECT 는 페이드 없이 즉시 abandon(Back, 코스면 end_course)' 추가. 'Reading 상태에서는 무조건 Stay' 서술이 있으면 삭제


## 웨이브 3A 반영 사항 (2026-10-10)

prepare/draw 2단계 파이프라인과 `SkinHost` 직접 그리기, 그리기 조건 의미론, 참조 이미지·음수 크기·이미지 인덱스·숫자·슬라이더·그래프 정합, TTF 텍스트, judgegraph·bpmgraph, Lua 함수 값 프레임 평가, 앱 호스트 군집 A·I·M 을 넣은 뒤의 상태다.

- (W3-0) r3-rbms-app.md §6.1: 'SkinDraw { …, background, offsets, extra }' 와 FrameExtra 서술을 'SkinDraw { …, offsets, data }', 'FrameInputs { offsets, data: FrameData }' 로
- (W3-0) r3-rbms-app.md §6.2 표 'PlayObjectState (FrameExtra)' 행과 '배경 텍스처' 행: NoteField{field, playfield, shade, bomb, keys_down} + GaugeFrame{kind, clear_threshold} + series.recent_hits + BgaFrame 으로 나뉘어 한 FrameData 로 전달. §6.3 배경 텍스처는 BgaFrame::of(background), 행 목록은 SongBars. §6.4 ResultSeriesState 는 FrameSeries(gauge_history, timing, bpm, notes)
- (W3-0) r3-rbms-app.md §12 와 상단 반영 사항: apps/rbms-player/src/skin_host/ 추가(ScreenHost, ClusterState, ROUTES, 군집 파일 14개, 아직 미연결, lib.rs 에 pub mod skin_host)
- (W3-1a) r3-rbms-app.md: skin_screen.rs 절에 with_skin_frame 의 새 흐름(ScreenHost 조립: offsets, static_screen, window, fallback -> 바인딩 1회 안 prepare -> 바인딩 밖 draw, 호스트에 기록된 명령은 아직 버림)과 skin_host/mod.rs 의 ScreenHost.fallback(군집이 모르는 id 를 기존 어댑터가 답함, 타이머는 TimerState 직접) 추가
- (W3-6) r3-rbms-app.md §6(화면별 상태 공급, 22행 리뷰 수정 항목의 결정 화면 호스트 서술): 결정 화면의 150~155·96·문자열·80/81·진행률은 이제 ScreenHost 의 chart·loading 군집이 답하고 구 DecideViewState 는 군집이 모르는 id 의 폴백으로만 남음. skin_screen.rs 절: FrameInputs 가 chart·loading 을 싣고 FrameInputs::new 로 만들며, with_skin_frame 이 모든 화면에 SystemState(skin_system_state)를 채움. AppShared.booted 추가
- (W3-6) r3-rbms-app.md §2 AppShared 필드 표: booted: Instant(앱 시작 시각, 스킨의 부팅 시간 숫자 27~29 용) 추가
- (W3-2) r3-rbms-app.md(호스트 절): image_index 를 답하지 않는 id 는 이미지가 숨는다는 점과, 앱이 FrameData.images 를 채워야 한다는 점을 W3-7·W4-5·W5 의 선행 조건으로 추가
- (리뷰 수정) r3-rbms-app.md 의 skin_host 절(system 군집): WallClock·of_unix_seconds 와 날짜 변환 상수가 삭제되고 SystemState.clock 이 rbms_skin::lua::LocalTime(로컬 시각)이 됐다고 고칩니다. 날짜 변환 구현은 format.rs fmt_datetime(UTC, 기록 목록용) 한 곳만 남습니다


## 웨이브 2B 반영 사항 (2026-10-10)

스킨 덤프 CLI, 앱의 스킨 팩 폴더 지정과 `.luaskin` 로드, 오버레이 총 크기 상한, 외부 스킨 첫 정지 프레임을 넣은 뒤의 상태다.

- (W2-8) r3-rbms-app.md §2.1: SkinOptions 는 { folder, pack, screen, selected, custom }. SkinLibrary 필드가 settings_path, root, forced_pack, pack, documents, pack_documents, pack_paths, headers(HeaderCache), loaded, errors, stale, waiting, arrived(RefCell), unannounced, seed, next_build 로 바뀜
- (W2-8) r3-rbms-app.md §2.2 1~2번: 스캔 확장자는 luaskin/json/json5. 스킨 폴더(깊이 3)와 스킨 팩(깊이 1) 두 곳을 훑고, 헤더는 경로 + 수정 시각 + 크기 키로 캐시(HeaderCache)되어 재스캔 때 Lua 헤더 패스를 다시 돌리지 않음. RELOAD 행(rescan_skins_afresh)만 캐시를 비움. 팩 헤더는 App::new 에서도 읽음
- (W2-8) r3-rbms-app.md §2.2 3번: 화면의 문서는 SkinLibrary::document_path = 수동 선택(config.skin.selected) 우선, 없으면 팩의 헤더 type 매핑(같은 타입은 파일명순 첫 문서). SKIN 행은 팩 문서를 'PACK: 이름' 으로 표시
- (W2-8) r3-rbms-app.md §2.2 4번: reload_for 가 프레임 루프에서 load_skin 을 부른다는 서술 교체. JSON 은 request_for 가 즉시 DefaultState 로 읽고, .luaskin 은 waiting 에 올린 뒤 with_skin_frame(화면 상태 어댑터가 호스트)에서 read_waiting 으로 읽고 다음 프레임 prepare_skin 의 adopt 가 반영. 시드는 SkinRead { host, seed } 인자(운영 None, 테스트·캡처 pin_seed). 쓰기 오버레이는 assets::skin_overlay_folder = <설정 폴더>/skin-data/<폴더명-sha256 8바이트>/
- (W2-8) r3-rbms-app.md §2.2 4·6번: 로드 실패는 errors 에 남고 take_failure 로 첫 줄을 1회 알림('skin: … - drawing the built-in screen'), 자동 재시도 없음. JSON 실패도 같은 알림을 탐
- (W2-8) r3-rbms-app.md §2.2 8번: 캐시 수명에 'begin_skin_scene(To, 스택 빈 Back)이 expire_scripted 로 성공한 Lua 스킨을 stale 로 만들어 다음 진입에서 다시 읽는다. suspend_skin_scene(Open)은 만료하지 않는다' 추가
- (W2-8) r3-rbms-app.md §2.3: 게이트는 skin_screen.rs 의 skin_document_is_enabled = skins.document_path(config, screen).is_some(). has_skin_document 는 대기 중인 Lua 스킨도 있는 것으로 봄
- (W2-8) r3-rbms-app.md §4: SKIN 탭 고정 행이 6개(PACK FOLDER / SCREEN / SKIN / LOADED / RELOAD / RESET). SettingId::SkinPack 은 host_row, SETTING_COUNT 85. LOADED 행에 'READ WHEN ITS SCREEN OPENS' 상태 추가. 커스터마이즈 저장은 여전히 custom[문서 경로]이고 팩 문서도 같은 키를 씀
- (W2-8) r3-rbms-app.md §5 표: 1행(확장자)·2행(.luaskin 헤더 경로와 캐시) 완료. 4행(워커 스레드 이동)은 '동기 로드 유지, 화면 첫 프레임에서 호스트와 함께 실행'으로 교체. 5행(쓰기 허용 범위)은 오버레이로 확정. 7행('팩 폴더를 고르면 모든 타입 지정')은 config.skin.pack + RBMS_SKIN_PACK 으로 완료. 8행(팩 식별자)은 '문서의 부모 폴더 경로에서 skin_pack_identifier 로 생성'
- (W2-8) r3-rbms-app.md §8.6: stage/capture.rs 의 팩 테스트는 config.skin.pack 으로 팩을 지정하고 화면별로 로드를 시도해 실패를 출력만 함(테스트 실패 아님), 팩 폴더 무변경 단언 포함. skin_select/tests.rs 6건, skin_screen/tests.rs 2건, stage/settings/skin_tests.rs 1건, assets.rs 1건, rbms-config tests.rs 1건 추가. rbms-player lib 테스트 982건
- (W2-8) r3-rbms-app.md §7 dialog 서술: 네이티브 대화상자가 5종(pick_skin_pack_folder 추가)
- (리뷰 수정) r3-rbms-app.md 상단에 '웨이브 2B 반영 사항' 추가, §2(스킨 선택·로드·캐시·리로드 흐름): SkinLibrary 의 실패 기록은 화면이 아니라 (화면, 실패한 경로)로 남고 현재 경로와 같을 때만 재시도를 막는다. drop_moved 가 팩 교체·해제·선택 변경 시 더 이상 그 문서로 그려지지 않는 화면의 loaded·waiting·errors 를 비운다. prepare_skin 은 매 프레임 drop_moved 와 SkinScreens::release_dropped 를 호출해 보이지 않는 화면의 컴파일 결과(텍스처, 디코드 워커)도 해제한다
- (리뷰 수정) r3-rbms-app.md §5(스킨 폴더 하나를 가리켜 화면별로 쓰기): 팩 폴더 지정(설정 skin.pack, 환경 변수 RBMS_SKIN_PACK)은 구현됨. skin.pack 은 계정 설정 동기화에서 기기 로컬 값(ir_sync.rs keep_local)이라 업로드되지 않고 다운로드로 덮이지 않는다
- (리뷰 수정) r3-rbms-app.md §6(화면별 상태 공급과 타이머): 결정(로딩) 화면 호스트가 OPTION_DIFFICULTY0~5(150~155)를 곡의 difficulty 로 답한다(1~5 는 해당 옵션, 그 밖과 곡 없음은 150)


## 웨이브 2A 반영 사항 (2026-10-10)

Lua 5.2 런타임(`crates/rbms-skin/src/lua/`), `SkinHost`, Lua 값 변환기, 2패스 `.luaskin` 로더를 넣고 구 샌드박스(`skin.*`)를 삭제한 뒤의 상태다.

- (W2-0) r3-rbms-app.md 274행 부근(화면별 상태 구현체 서술): `SkinStateSource` 를 `SkinHost` 로.
- (W2-5) r3-rbms-app.md 의 skin_screen.rs 절: `PlayerSkinAssets` 가 샌드박스를 들지 않음, `SkinSandboxFrame` 삭제, `skin_frame` → `with_skin_frame`(스킨 인터프리터에 프레임당 1회 호스트 바인딩, 클로저 안에서 그림). SKIN 탭 절: 플레이 문서에 자동 오프셋 행 15개가 생김, `uses_lua` 판정 기준 변경


## 웨이브 1B 반영 사항 (2026-10-10)

타이머 µs, stretch 11종, 장면 시계, 마우스 이벤트, GPU 논리 크기 런타임화와 색 공간, START/SELECT, 해상도 설정, 캡처 하니스를 넣은 뒤의 상태다.

- (W1-4) r3-rbms-app.md §7(245-246행): `skin_now_ms()` = `clock.elapsed().as_millis()` 를 `skin_now_us()` = `clock.elapsed().as_micros()` 로, `SkinDraw { …, now_ms, … }` 를 `now_us` 로. 257·333·354·356행의 `PlayViewState`/`ResultViewState::new`/`DecideViewState`/`KeyConfigViewState` 필드 `now_ms` 를 `now_us` 로
- (W1-4) r3-rbms-app.md §12(610행)과 A1 행(629행): `skin_now_ms` 표기를 `skin_now_us` 로. 장면 시계 미도입 문제 자체는 그대로(W1-5)
- (W1-5) r3-rbms-app.md §6.1: '시계: skin_now_ms() = clock.elapsed()...' 와 '타이머 표는 화면 전환 때 지워지지 않는다(skin_timers.clear() 호출 없음)' 는 낡았습니다. skin_now_us() 는 AppShared.scene_started 기준 장면 경과 µs 이고(skin_screen.rs), App::switch(lib.rs) 가 To/Back(스택 비어 있음)에서 begin_skin_scene() 으로 타이머 전부 OFF + 드라이버 기억(PlayTimers/SelectTimers/ResultTimers) 초기화 + 시계 0 을 수행한다. clock 은 오디오 폴백과 타이밍 계측 전용으로 남고 start_play() 는 스킨 시계를 건드리지 않는다.
- (W1-5) r3-rbms-app.md §1.1·§1.2: 'App.suspended 스택' 은 이제 Vec<Suspended { stage, scene: SkinScene }> 이다. Open 은 suspend_skin_scene() 으로 아래 화면의 타이머·드라이버 기억·경과 시간을 보관하고 열린 화면은 새 장면으로 시작하며, Back 은 resume_skin_scene() 으로 복원한다(시계는 열려 있던 동안 정지). 전이 실행은 여전히 App::switch 한 곳이다. 스택 비어 있는 Back 은 새 Select + 새 장면이다.
- (W1-5) r3-rbms-app.md §1.1 화면 계약: StageHandler 에 handle_mouse(ctx, at, button, pressed), handle_mouse_drag(ctx, at), handle_scroll(ctx, lines) 가 있다(handle_mouse 의 시그니처가 (at) 에서 바뀜). Stage::handle_mouse 는 Stage::handle_pointer(ctx, at, PointerInput) 로 바뀌었다.
- (W1-5) r3-rbms-app.md §7.1 표: '마우스 이동 - 상태만 저장, 이벤트 전달 없음', '마우스 클릭 - Pressed, Left 만', '마우스 release / 우클릭 / 가운데 / 휠 - 처리 없음' 은 낡았다. 이제 CursorMoved 는 버튼이 눌려 있으면 handle_mouse_drag 로, MouseInput 은 좌·우·가운데의 press/release 모두 handle_mouse(at, button, pressed) 로, MouseWheel 은 handle_scroll(lines) 로 전달된다(pointer.rs: PointerInput, HeldButtons, route_pointer). 옵션 패널이 열려 있으면 모두 삼킨다. 뒤로/앞으로 버튼은 전달하지 않는다. Focused(false) 는 눌린 버튼 기록을 비운다. 휠 lines 는 beatoraja amountY 부호(사용자 쪽으로 굴리면 양수)다.
- (W1-5) r3-rbms-app.md §7.2: 'handle_mouse 구현 화면은 Select 와 Settings 둘뿐' 은 유지되지만 둘 다 is_left_press 가드로 좌클릭 press 에만 반응한다.
- (W1-5) r3-rbms-app.md §7.5 표: 슬라이더 드래그(버튼 종류·release·drag 의 AppShared.mouse_held 필요) 와 휠(handle_scroll) 행은 '붙일 곳'이 아니라 배선 완료로 고친다. 객체 클릭 행(skin_click)과 호버, 텍스트 입력, 레인 키 select 조작 행은 그대로 남는다.
- (W1-5) r3-rbms-app.md §12: A1(장면 시계·타이머 수명) 과 A3(입력 이벤트 확장, IME 제외)을 완료로 표시한다. A1 은 '곡 시계와 분리' 까지 끝났다. A3 의 IME 는 미완이다.
- (W1-7) r3-rbms-app.md 의 Canvas 와 마우스 좌표 서술(§7 부근, 'gpu.logical_from_physical', 'Canvas::Window 가 1280x720', 'gpu.window'): position_in_space(x, y, UI_SIZE), Canvas = UI_SIZE 배율 래퍼 + Canvas::native(), gpu.request_redraw() 로 바뀜. Gpu::new 시그니처는 (window, shape).
- (W1-10) r3-rbms-app.md §7.1 표 '게임패드' 행: `Stage::handle_pad` 가 `PlayState` 만 구현한다는 서술을 'PlayState 와 SelectState(스크래치 이동, 흰 건반 결정, 검은 건반 폴더 상위, 홀드 반복)'로 바꿔야 합니다. 키보드 행에는 `Stage::handle_key` 가 가장 먼저 `AppShared::note_key` 로 눌림 집합(`KeyConfig.held`)을 갱신한다는 점을 추가합니다.
- (W1-10) r3-rbms-app.md §7.4 마지막 문단 ('레인 키와 START/SELECT 개념이 없다'): 이제 `ControlAction::{Start, Select}`(기본 키보드 A·W, 패드 STANDARD START/SELECT)와 `AppShared::{start_pressed, select_pressed, key_index_pressed}` 가 있습니다. 선곡의 레인 키 입력은 패드 이벤트만 연결됐고 키보드 레인 키는 아직 목록을 움직이지 않는다고 고칩니다.
- (W1-10) r3-rbms-app.md §7.5 마지막 행('게임패드/레인 키로 select 조작')과 §14.2 미확인 항목: 'START/SELECT 바인딩이 keyconfig 에 있는지는 미확인' 을 '없었고 W1-10 에서 추가됨(keyconfig.rs `ControlAction`, `ControlBinds.start/select`, gamepad.rs `StandardButton`)'으로 바꿉니다. `SelectState::handle_pad` 는 구현 완료(패널 닫힘 기본 조작만, 패널 1~3 은 W5).
- (W1-8) r3-rbms-app.md §9.1: '설정 스키마에 해상도·풀스크린 항목이 없다'는 서술이 낡았습니다. DisplayOptions 에 window_resolution(1280x720 기본, 1600x900, 1920x1080, 2560x1440, 3840x2160)과 window_mode(WINDOWED/BORDERLESS)가 생겼습니다. 창 생성은 lib.rs resumed 에서 window_mode::window_attributes(&config.display)를 쓰고(창 크기는 LogicalSize), 창 생성 줄 번호(lib.rs:997)는 바뀌었습니다.
- (W1-8) r3-rbms-app.md §9.1·§9.4: '표시 모드·풀스크린 지정 없음, Fullscreen/set_fullscreen grep 0건'은 낡았습니다. apps/rbms-player/src/window_mode.rs 가 Fullscreen::Borderless(None) 와 request_inner_size 를 씁니다. 적용은 App::sync_window 가 frame() 첫 줄에서 두 행의 변경을 감지해 수행합니다. 독점 전체 화면(Exclusive)과 vsync 행은 아직 없습니다.
- (W1-8) r3-rbms-app.md §9.1 레터박스 문단: 레터박스 적용은 draw_overlays 에서 config.display.fits_screen_shape() = letterbox || window_mode==Borderless 값으로 합니다. 즉 BORDERLESS 이면 LETTERBOX 행과 무관하게 항상 맞춰 그립니다(E6 구현).
- (W1-8) r3-rbms-app.md §9.4 '설정에 해상도·창 모드·vsync 항목 추가': 해상도와 창 모드는 완료(RESOLUTION, WINDOW MODE 행), vsync 만 남음. SETTING_COUNT 는 82 에서 84 로 바뀌었고 테스트 표 SHIPPED_ROWS 도 같이 늘었습니다.
- (W1-8) r3-rbms-app.md §4 또는 설정 저장 절: 이 변경은 스키마 버전을 올리지 않았습니다. 새 DisplayOptions 필드가 serde(default)로 기본값을 채우기 때문이고, 현재 스키마는 3 그대로입니다.
- (W1-9) r3-rbms-app.md §8.6(영향받는 테스트 표, 505~513행): stage/capture.rs 행 추가 필요(캡처 하니스, 테스트 6건: UI 크기 캡처가 render_tests 프레임과 동일, 큰 타깃 비율 채움, GPU 일치, 장면 시각, PNG 저장, 팩 선택 테스트). render_tests_document.rs 행은 자체 헬퍼(settings_of, 대기 루프)를 하니스 것으로 바꿨고 GPU 문서 캡처 테스트 1건이 늘어 6건이 됐다고 고쳐야 함.
- (리뷰 수정) r3-rbms-app.md §7.5·§12: 문서 그리기 진입점 서술에 'skin_frame 은 캔버스를 받지 않고, 커서는 UI_SIZE 공간 값(`AppShared.cursor`)을 그대로 document_cursor 에 넘긴다'를 반영해야 합니다. 입력 절에는 '`WindowEvent::Focused(false)` 에서 `AppShared::release_held_inputs()` 가 키보드 보유 상태(`KeyConfig.held`)와 마우스 버튼을 함께 비운다'를 추가해야 합니다.
- (리뷰 수정) r3-rbms-app.md §9.4: LETTERBOX 행 도움말이 'Keep the screen's aspect ratio inside the window; a borderless window always does' 로 바뀌었고, `Gpu::new(window, shape, letterbox)` 가 초기 맞춤 여부를 받아 BORDERLESS 로 시작해도 첫 프레임부터 맞춰 그린다는 점을 적어야 합니다. help 문자열은 화면에 그려지지 않는다는 사실도 함께 적는 편이 좋습니다.


## 웨이브 1A 반영 사항 (2026-10-09)

혼합 합성과 구 번들을 삭제한 뒤의 상태다. 본문의 해당 절은 아래 내용으로 읽는다.

- (리뷰 수정) §3 또는 §4(설정 저장·마이그레이션): 구 번들 선택값 폐기가 파일 로더(crates/rbms-config/src/io.rs migrate)뿐 아니라 계정 설정 다운로드(apps/rbms-player/src/ir_sync.rs parse_blob, 409 충돌 서버 사본 포함)에서도 스키마 2 이하 블롭에 적용된다는 점을 추가.
- (리뷰 수정) §8.2(418, 422, 426행): render_playfield_on_background, render_select_on_background_with_content, render_result_on_background_with_content 를 '제거 대상'으로 적은 행을 '제거 완료'로 갱신. on_background 진입점은 전부 없어졌고 내장 화면은 render_playfield_view, render_select(_ctx), render_result_with_palette(_ctx) 만 남았습니다.
- (리뷰 수정) §8.6(영향받는 테스트, 477~481행): render_tests_skin.rs 행의 '대부분 폐기 또는 재작성'을 실제 결과로 바꿔야 합니다. 5개 파일 삭제, 남는 동작 5건(문서 단독 그리기, 읽는 동안 내장 화면, 문서 없으면 내장 프레임 불변, 화면 간 격리, play bga 객체)은 apps/rbms-player/src/stage/render_tests_document.rs 로 옮겼고 결과·결정 화면 단독 그리기 1건을 추가(총 6건).
- (W1-1b) r3-rbms-app.md §8.2(앱 쪽 코드 표): skin_screen.rs·stage/play/mod.rs·stage/select/mod.rs·stage/result.rs·app_options.rs·app_input.rs 행 전부 처리 완료로 바꾸고 줄 번호 삭제. 남은 것은 assets.rs 게이트(아래), stage/loading.rs·stage/keyconfig.rs 의 '문서가 못 그리면 내장' 폴백 두 행뿐. skin_screen.rs 에 남은 진입점은 prepare_skin, has_skin_document, skin_offsets, skin_frame, draw_play_skin, draw_select_skin(canvas, view, background), draw_result_skin, draw_decide_skin, draw_keyconfig_skin.
- (W1-1b) r3-rbms-app.md §2.3(문서 활성화 게이트): skin_document_is_enabled 는 이제 '선택 없음 → false, 선택 경로가 번들 세대(steel-neon, -v2, -v3) 문서 경로 → 항상 false, 그 외 → true'. display.skin 프리셋은 문서 on/off 에 더 이상 관여하지 않음(필드 RON·theme.ron·sound 선택에만 남음).
- (W1-1b) r3-rbms-app.md §2.2 7번: '컴파일 전에는 내장 레이아웃' 은 유지. 6번의 '대체 미충족 경고 토스트'는 삭제됨(컴파일 경고 첫 줄 토스트만 남음).
- (W1-1b) r3-rbms-app.md §6.1~§6.4: FrameExtra::Select 의 SelectListState 는 {rows, sel, detail, options_open: bool}. OptionsRows(라벨·값 11쌍) 삭제. PlayViewState 에서 target_delta 삭제. ResultViewState 의 with_hint/with_ir_status 와 IR 상태 한 줄 문자열(IR_STATUS_SEPARATOR) 삭제 — 결과 문서에 IR 상태는 공급되지 않음. draw_document 의 layer 인자, draw_*_skin_layer 3종 삭제. §6.2 주의점의 '레인 → 타이머 키 번호는 내장 Skin.x' 는 그대로이고, Skin 이 더 이상 문서 note.dst 로 덮이지 않는다는 문장을 추가.
- (W1-1b) r3-rbms-app.md §7.2(클릭 판정 구조): 스킨 핫스팟 사상(select_hotspots → Hot)이 삭제됨. 문서가 그리는 화면은 hot 영역을 하나도 등록하지 않음. 내장 render_select 의 SelectHot → Hot 사상만 남음.
- (W1-1b) r3-rbms-app.md §8.5: 옵션 오버레이는 screen_content 게이트 없이 열려 있으면 항상 그림(문서 위 시스템 오버레이). 키 봄은 내장 경로에서만 그림(문서가 그리는 프레임에는 없음).
- (W1-1b) r3-rbms-app.md §8.6(영향받는 테스트 표): render_tests_skin*.rs 5개 삭제 완료. stage/select/tests.rs 의 write_blocks_document·blocks_app·테스트 4건 삭제(476행 이후 전부). stage/play/tests.rs 는 대체 표·레인 결합·키 봄 테스트 4건 삭제, 단독 문서 픽스처(play_document())로 키 타이머·넛지 2건 유지. skin_select/tests.rs 의 bundled_default_documents_compile_without_warnings 삭제. assets.rs 의 RESULT_REPLACEMENT_CONTRACT 삭제, 게이트 단언 2곳 반전. 전체 테스트 수 3,181 은 낡음(rbms-player lib 858, rbms-render lib 273).
- (W1-1a) r3-rbms-app.md §2.1 표 '설정' 행: SkinOptions 는 { folder, screen, selected, custom } 넷뿐(schema.rs:252). shared 와 default_skin_installed 삭제, '번들키 → 공유 커스터마이즈' 문구 삭제.
- (W1-1a) r3-rbms-app.md §2.2 항목 9 (c) '번들 공유 행 변경 → 같은 번들의 모든 화면 stale' 삭제(stale_bundle 없음). 같은 절의 skin_select.rs 줄 번호 전부 재기재 필요(파일 818 → 578줄, skin_root 는 483행).
- (W1-1a) r3-rbms-app.md §2.3 전체 교체: assets.rs 의 skin_document_is_enabled 는 삭제됐고 게이트는 skin_screen.rs:397 의 비공개 메서드 'config.skin.document(screen).is_some()' 한 줄. DISPLAY SKIN 프리셋은 NORMAL/WIDE 둘뿐이며 필드 RON 만 고름. theme.ron 은 항상 <설정 폴더>/theme.ron, 사운드 폴더는 프리셋과 무관.
- (W1-1a) r3-rbms-app.md §3.1 전체 교체: 내장분은 사운드 22개뿐(assets.rs:43 embedded_sounds! 매크로, :53 DEFAULT_SOUNDS). 설치는 run() 초입 lib.rs:1122 의 install_default_sounds(settings_path) 가 매 실행 <설정 폴더>/skin/rbms-default/sound/ 에 없는 파일만 create_new 로 씀. 세대, SHA-256 비교, 세대 이동, 최초 선택 채우기, 설치 플래그는 없음. theme.ron 템플릿은 load_theme(assets.rs:171)이 파일이 없을 때 씀. 소스 크기는 assets/skins/rbms-default 580 KB.
- (W1-1a) r3-rbms-app.md §3.2: '덮어쓰지 않는 규칙 때문에 세대를 올려야 한다' 항목은 세대 개념이 사라져 삭제. compute_build_hash 설명은 유지.
- (W1-1a) r3-rbms-app.md §4: SkinRow 는 Property / File / Offset(axis) 셋(skin_select.rs:144). '두 스코프', bundle_key, shared[번들키], scope: 'bundle' 공유 행 서술 삭제. user_config(path) 는 custom[path] 만 돌려줌(schema.rs:302).
- (W1-1a) r3-rbms-app.md §5 표의 6행(skin_document_is_enabled 프리셋 결합), 8행(bundle_key), 14행(sound_folder_path 의 STEEL NEON 조건): 현재 상태로 갱신. 6행은 완료, 8행은 대상 함수 삭제, 14행은 '미지정이면 설치된 기본 세트(assets.rs:106 default_sound_folder)'.
- (W1-1a) r3-rbms-app.md §8.2 표의 'assets.rs 305-318 skin_document_is_enabled 프리셋 게이트 → 단순화' 행: 완료 표시.
- (W1-1a) r3-rbms-app.md §8.6 표: 'assets.rs 내 mod tests' 행은 번들 테스트 7건 삭제와 사운드 설치 테스트 3건 추가로 갱신(mod tests 는 382행부터). 'skin_select/tests.rs, fixtures.rs' 행은 번들 스코프 테스트 4건, installed_default_documents 테스트, SHARED 픽스처 삭제 반영. 'stage/settings/skin_tests.rs' 행은 6건 → 5건(번들 공유 행 테스트 삭제).
- (W1-1a) r3-rbms-app.md §10.1 '폴더 해석 순서': (2)를 '프리셋과 무관하게 <설정 폴더>/skin/rbms-default/sound (첫 실행에 설치, 폴더가 없으면 무음)'으로 교체. 참조는 lib.rs:215 sound_folder_path, assets.rs:106.
- (W1-1a) r3-rbms-app.md §12 B3 행(schema.rs:247-399 범위)과 §14 의 줄 번호: schema.rs 가 줄어 범위 재기재 필요.
- (W1-2) r3 와 00-synthesis 의 매트릭스에서 rbms-skin 의 'hotspot/replace/scope/densitygraph/composition/layer' 열이 있다면 '삭제됨' 으로 갱신하세요(해당 열은 이 단위에서 직접 확인하지 못했습니다).


- 조사일: 2026-10-09, 대상 브랜치 `dev`(HEAD `9ce92bb`), 읽기 전용 조사
- 표기: `R:` = `/Users/hyunseokbyun/development/R-BMS/` 기준 경로, `B:` = `/Users/hyunseokbyun/development/beatoraja/src/bms/player/beatoraja/` 기준 경로, `M:` = `/Users/hyunseokbyun/Downloads/ModernChic/` 기준 경로
- "미확인" 은 이번 조사에서 원문을 열어 확인하지 못한 항목이다. 읽지 못한 파일 목록은 문서 끝(§14)에 있다.
- 저장소 규칙 주의: `R:docs/HANDOFF.md:50` 은 레퍼런스 엔진 이름과 외부 스킨 묶음 이름을 저장소(코드·문서·커밋·파일명)에 쓰지 말라고 적고 있다. 이 보고서는 저장소 밖 스크래치에만 있으므로 이름을 그대로 쓴다. 구현 사양·커밋·폴더명에서 이 규칙을 유지할지는 사용자 결정 사항이다(§13 미해결 질문 1).

---

## 0. 한눈에 보는 결론

| 영역 | 현재 R-BMS | ModernChic(레퍼런스 계약)이 요구하는 것 | 격차 크기 |
| --- | --- | --- | --- |
| 화면 수 | Stage 10개, 그중 스킨 문서가 붙는 곳 5종(play/select/result/loading=decide/keyconfig) | 7개 상태 전부가 스킨 단독 소유(select, decide, play, result, course result, key config, skin config) | L |
| 합성 방식 | 내장 화면 + 문서 혼합(`replace`/`layered`/`overlay`), 문서 없으면 내장 화면 | 스킨이 화면 전체를 그림. 내장 레이아웃 없음 | L |
| 장면 시계 | 앱 전체 단일 시계, 화면 전환 시 타이머 미초기화 | 상태 전환마다 모든 타이머 OFF + 장면 시계 0 재시작 | M (정확도에 치명적) |
| 장면 수명 | 키 입력 즉시 전환 | `input`/`scene`/`fadeout`/`loadstart`/`loadend`/`playstart`/`close`/`finishMargin` 로 스킨이 전환 시점을 정함 | L |
| 문서 탐색 | `.json`/`.json5` 만, 화면 타입 5·6·7·8·play | `.luaskin` 10개(타입 0,1,2,3,5,6,7,8,9,15) | M |
| 마우스 | 좌클릭 press 한 종류, 핫스팟 8종(select 전용) | 객체 click 이벤트, 슬라이더 드래그, 휠, 텍스트 입력 포커스 | L |
| 논리 해상도 | 1280x720 고정(`CW`/`CH`), 창 크기만 늘림 | 1920x1080 저작 | M |
| 시스템 사운드 | 22 stem, 1회 재생만, 루프·BGM 없음 | select/decide BGM 루프, 스킨 자체 효과음 | M |
| 동영상 | 미지원(이미지 BGA만) | mp4 8개(193MB): select/decide 배경, play 기본 BGA | L |
| 기본 스킨 배포 | 1.8MB 를 `include_bytes!` 로 바이너리에 내장 후 최초 실행 시 설치 | 원본 478MB (png 212MB, mp4 193MB, ttf 55MB) | 배포 방식 재설계 필요 |

---

## 1. 화면(Stage) 목록과 전이

### 1.1 R-BMS Stage 정의

- `Stage` 열거 10종: `Select, Settings, KeyConfig, Tables, Folders, Loading, Play, Result, CourseResult, Practice` (`R:apps/rbms-player/src/stage/mod.rs:65-76`), 대응 `StageId` (`stage/mod.rs:82-93`).
- 전이 타입 `Transition::{Stay, Open(Stage), To(Stage), Back, Quit}` (`stage/mod.rs:99-110`). `Open` 은 현재 화면을 `App.suspended` 스택에 넣고, `Back` 은 꺼낸다. 스택이 비면 새 `SelectState` 를 만든다(`R:apps/rbms-player/src/lib.rs:867-884`, 특히 872).
- 전이 실행은 한 곳: `App::switch` 가 `on_exit` → 교체 → `on_enter` 순서(`lib.rs:878-883`). 옵션 오버레이는 select 가 아닌 화면으로 갈 때 닫힌다(`lib.rs:875-877`).
- 화면 계약 `StageHandler`: `update / draw / handle_key / handle_mouse / handle_pad / on_enter / on_exit / debug_lines / holds_keys` (`stage/mod.rs:136-167`).

### 1.2 R-BMS 전이 그래프(코드 근거)

| 출발 | 조건 | 전이 | 근거 |
| --- | --- | --- | --- |
| (시작) | 라이브러리 폴더 있음 | `Loading(scan)` | `lib.rs:714-716` |
| (시작) | 폴더 없음 / 차트·리플레이 직접 실행 | `Select` (직접 실행은 `resumed` 에서 `load()` 후 `To`) | `lib.rs:708-713`, `lib.rs:1007-1018` |
| Loading(scan/table) | 완료 | `Back` (스택 비어 있으면 새 Select) | `stage/loading.rs:269-279, 300-304, 331` |
| Select | Enter/→/행 재클릭 (곡) | `Open(Loading::song)` + `SystemSound::Select` | `stage/select/mod.rs:142-149, 731, 747-750` |
| Select | Enter (폴더) | 같은 화면에서 목록 교체 + `FolderOpen` | `stage/select/mod.rs:150-157` |
| Select | Enter (코스 탭) | `start_course` → `To(Loading::song)` | `stage/select/mod.rs:164-175`, `lib.rs:233-255` |
| Select | Tab / O / T | `Open(Settings)` / `Open(Folders)` / `Open(Tables)` | `stage/select/mod.rs:714-716` |
| Select | F4 | `practice_requested=true` + `Open(Loading::song)` | `stage/select/mod.rs:189-196, 708` |
| Select | Esc(루트에서 1초 내 2회) | `Quit` | `stage/select/mod.rs:540-550`, `lib.rs:168` |
| Settings | KEY CONFIG 행 | `Open(KeyConfig)` | `stage/settings.rs:160-164, 544` |
| Settings / KeyConfig / Folders / Tables | Esc | `Back` | `stage/settings.rs:527-530`, `stage/keyconfig.rs:153-156` |
| Loading(song) | 한 프레임 그린 뒤 `load()` | `To(Loading::assets)` 또는 바로 `To(Play)` | `stage/loading.rs:340-349, 444-447`, `app_play.rs:441-451` |
| Loading(assets) | 키음·BGA 디코드 완료 | `SystemSound::Decide` + `start_play()` + `To(Play)`, 연습 요청이면 `To(Practice)` | `stage/loading.rs:453-465` |
| Loading | Esc | 취소 후 `Back` | `stage/loading.rs:355-368, 468-470` |
| Play | 곡 종료 또는 게이지 0(분석 재생 제외) | `enter_result` → `To(Result)` / 코스면 다음 스테이지 `To(Loading::song)` 또는 `To(CourseResult)` / 연습이면 `Back` | `stage/play/mod.rs:431-436, 732-734`, `app_result.rs:301-309, 383-437` |
| Play | Esc(즉시/홀드/더블) | `leave_run` → `Back`(연습), `end_course`(코스), `leave_play` | `stage/play/mod.rs:448-498` |
| Result | Esc/Enter | `leave_play` → `Back` (라이브러리 없으면 `Quit`) | `stage/result.rs:106`, `app_play.rs:564-573` |
| Result | R / N | 재시도 / 다음 곡 → `To(Loading::song)` | `stage/result.rs:107-108`, `app_result.rs:442-466` |
| CourseResult | Esc/Enter | `end_course` → `Back` | `stage/course_result.rs:78-86`, `lib.rs:259-267` |
| Practice | Enter | `Open(Play)` (슬라이스), 종료 시 `Back` 으로 패널 복귀 | `stage/practice.rs:65-77, 96-97`, `app_result.rs:301-303` |

### 1.3 레퍼런스 상태와 전이

- 상태 7종: `MUSICSELECT, DECIDE, PLAY, RESULT, COURSERESULT, CONFIG, SKINCONFIG` (`B:MainState.java:174-182`), 스위치 `B:MainController.java:248-268`.
- 상태 전환 시 공통 절차(`B:MainController.java:270-284`): 이전 상태 `shutdown()` + `setSkin(null)` → 새 상태 `create()`(여기서 `loadSkin`) → `skin.prepare()` → `timer.setMainState()` → `prepare()`.
- `timer.setMainState` 는 모든 타이머를 OFF(`Long.MIN_VALUE`)로 채우고 장면 시작 시각을 현재로 다시 잡는다(`B:TimerManager.java:100-106`). 즉 "타이머 없는 목적지"의 시간축은 항상 "이 상태에 들어온 뒤 경과 ms" 이다.

| 출발 | 조건 | 도착 | 근거 |
| --- | --- | --- | --- |
| (시작) | bms 파일 지정 / 미지정 | PLAY / MUSICSELECT | `B:MainController.java:341-351` |
| MUSICSELECT | 곡 결정(`readChart`) | DECIDE | `B:select/MusicSelector.java:426` |
| MUSICSELECT | 코스 결정(`_readCourse`) | DECIDE | `B:select/MusicSelector.java:527` |
| MUSICSELECT | NUM6 | CONFIG | `B:select/MusicSelector.java:296-297` |
| MUSICSELECT | `OPEN_SKIN_CONFIGURATION` | SKINCONFIG | `B:select/MusicSelector.java:298-299` |
| DECIDE | `nowtime > skin.scene` 또는 키 → `TIMER_FADEOUT` → `fadeout` 경과 | PLAY (취소면 MUSICSELECT) | `B:decide/MusicDecide.java:37-63` |
| PLAY | FINISHED + `finishMargin` → FADEOUT → `fadeout` 경과 | RESULT / 연습 복귀 / 다음 코스 PLAY / 다음 곡 DECIDE / MUSICSELECT | `B:play/BMSPlayer.java:745-784` |
| PLAY | FAILED + `skin.close` 경과 | RESULT / MUSICSELECT, 빠른 재시작은 PLAY | `B:play/BMSPlayer.java:693-742` |
| RESULT | 입력 또는 `scene` 경과 → FADEOUT → `fadeout` 경과 | MUSICSELECT / PLAY(리플레이·다음 코스 스테이지) / COURSERESULT | `B:result/MusicResult.java:172-258` |
| COURSERESULT | 종료 | MUSICSELECT | `B:result/CourseResult.java:182` |
| CONFIG | 종료 | MUSICSELECT | `B:config/KeyConfiguration.java:244` |
| SKINCONFIG | Esc | `saveConfig` 후 MUSICSELECT | `B:config/SkinConfiguration.java:61-67` |

### 1.4 화면 대응표

| 레퍼런스 상태 (SkinType id) | ModernChic 진입 파일 | R-BMS 대응 Stage | 현재 스킨 배선 | 차이·주의 |
| --- | --- | --- | --- | --- |
| MUSICSELECT (5) | `M:musicselect.luaskin` | `Select` | `prepare_skin(SKIN_TYPE_MUSIC_SELECT)` (`stage/select/mod.rs:786`) | 탭(곡/코스), 기록 모달, 랭킹 패널, 필터 패널, 옵션 오버레이가 전부 내장 UI |
| DECIDE (6) | `M:decide.luaskin` | `Loading` (곡 로드일 때만 의미 있음) | `prepare_skin(SKIN_TYPE_DECIDE)` (`stage/loading.rs:478`) | 레퍼런스의 decide 는 "시간 연출 장면"이고 로딩은 PLAY 의 PRELOAD 단계에서 한다. R-BMS 는 로딩 화면을 decide 로 그린다. 스캔·테이블 받기 중에도 같은 문서를 그린다(`stage/loading.rs:475-485` 는 task 종류를 가리지 않음) |
| PLAY (0,1,2,3,4,16...) | `M:play5_hw / play7_hw / play10_hw / play14_hw.luaskin` | `Play` | `mode_skin_type(mode)` → `prepare_skin` (`stage/play/mod.rs:797-804`) | PRELOAD/READY/FAILED/FINISHED 단계 없음. 9키·24키용 ModernChic 문서 없음 |
| RESULT (7) | `M:result.luaskin` | `Result` | `prepare_skin(SKIN_TYPE_RESULT)` (`stage/result.rs:115`) | 입력 잠금·fadeout·랭킹 스크롤·게이지 종류 전환 없음 |
| COURSERESULT (15) | `M:course.luaskin` | `CourseResult` | 스킨 호출 없음. 내장 텍스트만(`stage/course_result.rs:99-118`) | 타입 15 는 로더가 지원 목록에 없음(`R:crates/rbms-skin/src/loader.rs:221, 240-242`) |
| CONFIG (8) | `M:keyconfig.luaskin` | `KeyConfig` | `draw_keyconfig_skin(keys)` (`stage/keyconfig.rs:185-189`) | 문서에 넘기는 상태가 레인 키 이름 목록뿐(`skin_screen.rs:895-906`). 입력 처리는 내장 행 리스트 기준 |
| SKINCONFIG (9) | `M:skinselect.luaskin` | 없음(가장 가까운 것은 `Settings` 의 SKIN 탭) | 없음 | 타입 9 도 로더 미지원. 전용 Stage 신설 필요 |
| — | — | `Settings` | 없음 (`render_settings`, `stage/settings.rs:658-671`) | R-BMS 전용. 레퍼런스는 런처(별도 창)에서 설정 |
| — | — | `Tables`, `Folders` | 없음 | R-BMS 전용 |
| — | — | `Practice` | 없음(`stage/practice.rs:138-156`) | 레퍼런스는 PLAY 상태 안의 `STATE_PRACTICE`(`B:play/BMSPlayer.java:524-595`)로 play 스킨 위에서 처리 |
| — | — | `Loading`(scan/table/song) | decide 문서 또는 내장 진행 막대 | R-BMS 전용 단계 |

레퍼런스에만 있고 R-BMS 에 없는 것: 독립 DECIDE 장면, SKINCONFIG 상태, PLAY 내부 6단계 상태기계(`STATE_PRELOAD/PRACTICE/PRACTICE_FINISHED/READY/PLAY/FAILED/FINISHED`, `B:play/BMSPlayer.java:69-75`), RESULT 의 IR 처리 상태(`STATE_OFFLINE/IR_PROCESSING/IR_FINISHED`, `B:result/AbstractResult.java:22-30`), 코스 스테이지 사이 RESULT → PLAY 직행(`B:result/MusicResult.java:200-208`; R-BMS 는 스테이지마다 Loading 을 거친다 `lib.rs:245-255`).

---

## 2. 스킨 선택·로드·캐시·리로드 흐름

### 2.1 데이터 구조

| 계층 | 타입 | 위치 | 역할 |
| --- | --- | --- | --- |
| 설정 | `SkinOptions { folder, screen, selected: BTreeMap<i32,String>, custom, shared, default_skin_installed }` | `R:crates/rbms-config/src/schema.rs:247-264` | 화면 타입 id → 문서 절대경로, 문서경로 → 커스터마이즈, 번들키 → 공유 커스터마이즈 |
| 디스크 목록 | `SkinLibrary { root, documents: Vec<SkinHeader>, loaded: BTreeMap<i32, LoadedDocument>, errors, stale, next_build }` | `R:apps/rbms-player/src/skin_select.rs:189-208` | 헤더 스캔 결과와 화면별 "전체 로드된 문서" |
| 컴파일 캐시 | `SkinScreens { built: BTreeMap<i32, BuiltScreen>, pending: BTreeMap<i32, PendingScreen> }` | `R:apps/rbms-player/src/skin_screen.rs:462-465` | 화면 타입별 `rbms_render::SkinScreen`(텍스처 보유)과 디코드 중인 작업 |
| 타이머 | `skin_timers: TimerState`, `skin_play_timers`, `skin_select_timers`, `skin_result_timers` | `lib.rs:547-550` | 전 화면 공용 타이머 표 1개 + 화면별 드라이버 3개 |

### 2.2 흐름

1. **루트 결정**: `skin_root()` = `config.skin.folder` 가 있으면 그 경로, 없으면 `<settings.ron 이 있는 폴더>/skin` (`skin_select.rs:716-721`, 기본 폴더명 `DEFAULT_SKIN_FOLDER = "skin"` `schema.rs:211`). 즉 기본은 `~/.config/rbms/skin/`.
2. **스캔**: Settings 화면 `on_enter` 와 RELOAD 행에서 `rescan_skins()` (`stage/settings.rs:582-589, 203-208`, `settings_ui.rs:161-164`). `scan()` 은 깊이 3(`SCAN_MAX_DEPTH`), 최대 512개(`SCAN_MAX_DOCUMENTS`), 확장자 `json`/`json5` 만(`DOCUMENT_EXTENSIONS`) 보고 `load_header` 가 성공한 파일만 모은다(`skin_select.rs:32-39, 727-762`). 헤더 로드 시 모드는 항상 `crate::MODE`(7키)로 넘긴다(`skin_select.rs:749`).
3. **선택**: SKIN 탭의 SCREEN 행이 `config.skin.screen`(화면 타입 id)을 바꾸고, SKIN 행이 그 타입의 후보를 순환한다. 순환 대상은 "내장 화면(None)" + 같은 `skin_type` 의 헤더들(`skin_select.rs:254-256, 620-634`). 선택은 `config.skin.select(screen, path)` 로 저장(`schema.rs:311-316`).
4. **전체 로드**: 화면이 그려질 때마다 `prepare_skin(canvas, screen)` 이 `needs_reload_for` 를 보고 `reload_for` 를 호출한다(`skin_screen.rs:573-583`). `reload_for` 는 `load_skin(path, SkinLoadOptions::new(&root, &user, mode))` 를 **프레임 루프에서 동기 실행**한다(`skin_select.rs:682-705`). 모드는 `skin_type_mode(screen).unwrap_or(MODE)` (`skin_select.rs:693`). 성공 시 `next_build` 를 증가시켜 빌드 번호를 준다(`skin_select.rs:697-699`). 실패 사유는 `errors[screen]` 에 남고 자동 재시도하지 않는다(`skin_select.rs:658-672, 701-703`).
5. **자산 디코드(비동기)**: `SkinScreens::sync` 가 빌드 번호 변화를 보고 이전 화면 텍스처를 `release` 하고 `PendingScreen::start` 로 `document.sources`(이미지)와 `document.fonts`(폰트 바이트)를 워커 풀에 건다(`skin_screen.rs:483-497, 427-434`). 워커 수는 코어 수(최대 8, `assets.rs:21, 523`).
6. **컴파일**: 풀이 끝난 프레임에 `SkinScreen::build(r, text, document, &mut assets)` (`skin_screen.rs:499-525`). 이미지/폰트는 이미 디코드된 것만 넘긴다(`PlayerSkinAssets`, `skin_screen.rs:48-84`). 표현식은 문서의 Lua 샌드박스에 컴파일(`skin_screen.rs:81-83`). 경고 첫 줄을 토스트로 알린다(`skin_screen.rs:513-517`).
7. **그 사이의 화면**: 컴파일 전에는 `SkinScreens::get` 이 `None` 이라 **내장 레이아웃이 그려진다**(`skin_screen.rs:473-479, 687`). 스킨 단독 소유로 바꾸면 이 구간에 그릴 것이 없어지므로 "로드 중 화면" 정책을 새로 정해야 한다(§8.4).
8. **캐시 수명**: 컴파일된 화면은 타입별로 앱 수명 동안 유지된다(`skin_screen.rs:456-460`). 레퍼런스는 상태를 떠날 때마다 스킨을 dispose 한다(`B:MainController.java:272-273`, `B:MainState.java:116-120`). 1920x1080 스킨 10종을 전부 상주시키면 GPU 메모리 사용이 크다(§11 위험).
9. **리로드 트리거**: (a) 선택 경로 변경(`needs_reload_for` 의 `entry.path != path`, `skin_select.rs:671`), (b) property/file 행 변경 → `stale` (`skin_select.rs:533, 547`), (c) 번들 공유 행 변경 → 같은 번들의 모든 화면 stale(`skin_select.rs:569-573`), (d) RELOAD 행(`stage/settings.rs:203-208`), (e) RESET 행(`settings_ui.rs:155-158`). offset 행은 리로드 없이 매 프레임 `MergedOffsets` 로 읽는다(`skin_screen.rs:378-387, 658-666`).
10. **주의**: `AppShared::reload_skin()` 은 "SKIN 탭이 지금 보고 있는 화면 타입" 하나만 다시 읽는다(`settings_ui.rs:167-169` → `skin_select.rs:677-679`). 다른 화면은 다음에 그 화면이 그려질 때 `prepare_skin` 이 읽는다.

### 2.3 문서 활성화 게이트(프리셋 결합)

`skin_document_is_enabled(settings_path, config, screen)` (`R:apps/rbms-player/src/assets.rs:305-318`):

- 선택이 없으면 false.
- 선택 경로가 내장 번들 세대(`steel-neon`, `steel-neon-v2`, `steel-neon-v3`)의 문서 경로가 아니면 **항상 true**.
- 내장 번들 문서면 `config.display.skin == "STEEL NEON"` 이고 활성 세대 폴더의 경로일 때만 true.

즉 DISPLAY 의 `SKIN` 프리셋 행(`NORMAL`/`WIDE`/`STEEL NEON`, `schema.rs:176-177`)이 내장 번들 문서의 on/off 스위치 역할을 한다. 이 프리셋은 동시에 (a) 플레이 필드 RON(`play.ron`/`play-dual.ron`, `assets.rs:211-214`, `app_play.rs:355-367`), (b) UI 테마 `theme.ron`(`assets.rs:216-221, 455-470`), (c) 번들 `sound/` 폴더(`assets.rs:321-327`)까지 고른다. 스킨 단독 소유로 가면 이 결합 전체가 제거 대상이다.

---

## 3. 기본 번들 설치와 바이너리 영향

### 3.1 현재 방식

- `BundledFile { path, bytes: &'static [u8] }` 를 `include_bytes!` 로 채운다(`assets.rs:41-44, 58-175`). v1 20개, v2 24개, v3 66개 파일이 **세 세대 모두** 바이너리에 들어 있다(`BUNDLE_GENERATIONS`, `assets.rs:188-192`). 별도로 `normal.ron`/`wide.ron` 이 `include_str!`(`assets.rs:26-27`).
- 소스 크기: `assets/skins/steel-neon` 148KB, `steel-neon-v2` 392KB, `steel-neon-v3` 1.3MB(du 실측). 로컬 release 바이너리는 22,220,464 바이트(`target/release/rbms-player`, 2026-10-09 빌드).
- 설치 시점: `run()` 초입에서 `needs_default_skin_install` → `install_default_skin` (`lib.rs:1125-1134`). 조건은 `!config.skin.default_skin_installed` 이거나 현 세대 파일이 하나라도 없을 때(`assets.rs:223-229`).
- 설치 규칙: 파일이 이미 있으면 **절대 덮어쓰지 않는다**(`create_new(true)`, `assets.rs:399-422`). 사용자가 고친 파일 보존이 목적. 이전 세대 폴더는 "이미 있는 경우에만" 보수한다(`assets.rs:235-238`).
- 세대 이동: 선택이 이전 세대의 "출하본과 바이트가 같은" 문서를 가리키면 현 세대 경로로 옮기고 `custom`/`shared` 선택도 옮긴다(SHA-256 비교, `assets.rs:262-303, 380-388`).
- 최초 설치 시 선택 채우기: 프리셋이 기본값(`NORMAL`)이면 `STEEL NEON` 으로 바꾸고 화면 타입 8종(`STEEL_NEON_DOCUMENTS`: 5, 6, 0, 1, 2, 3, 4, 7)에 문서를 지정한다(`assets.rs:177-186, 243-257, 367-376`). key config(8)는 지정하지 않는다.
- 테마: `~/.config/rbms/theme.ron` 템플릿도 같이 쓴다(`assets.rs:240-241, 427-453`).

### 3.2 바이너리와 엮인 부수 효과

- `compute_build_hash()` 가 시작 시 **실행 파일 전체를 메모리로 읽어** SHA-256 을 낸다(`lib.rs:339-345`, `lib.rs:835`). IR 제출의 `client_build_sha256` 에 쓰인다(`app_result.rs:199`). 바이너리가 수백 MB 가 되면 시작 지연·메모리 급증이 직접 생긴다.
- 릴리스 산출물은 "단일 바이너리 + LICENSE + README" 를 tar/zip 한 것이다(`R:.github/workflows/release.yml:75-92, 111-120`). 리소스 폴더를 같이 싣는 단계가 없다. macOS 는 `lipo` 로 두 아키텍처를 합치므로 내장 자산은 두 번 들어간다(`release.yml:78-83`).
- 설치는 "복사"이므로 내장 크기만큼 디스크를 한 번 더 쓴다.
- 덮어쓰지 않는 규칙 때문에 기본 스킨을 고치려면 세대(폴더명)를 올려야 한다. 대형 스킨에서는 세대마다 전체 복사본이 쌓인다.

### 3.3 대형 스킨을 기본으로 둘 때의 배포 방식 비교

ModernChic 원본 실측(`find`/`stat`): 총 478MB. png 284개 212.3MB, mp4 8개 192.8MB, ttf 11개 54.7MB, fnt 10개 12.8MB, ogg 73개 2.6MB, lua 126개 1.1MB. 폴더별: Play 214MB, Select 115MB, Result 69MB, Decide 66MB, SkinSelect 11MB, Root 2.9MB, Sound 1.2MB. 최대 단일 파일은 `Play/parts/common/BGA/movie/NOSTALGIC.mp4` 48.4MB.

| 방식 | 구현 지점 | 장점 | 영향·제약 |
| --- | --- | --- | --- |
| A. 지금처럼 `include_bytes!` 내장 + 최초 실행 복사 | `assets.rs` 표 확장 | 단일 파일 배포 유지, 오프라인 최초 실행 가능 | 바이너리 +283MB(mp4 제외) ~ +478MB, macOS 유니버설은 약 2배. 컴파일 시간·링커 메모리 증가. `compute_build_hash` 가 전체를 읽음. 디스크 2배. 파일 표를 손으로 관리(현재 110줄, 원본은 500여 파일). 세대 올릴 때마다 전체 재복사. 사실상 부적합 |
| B. 저장소 폴더 + 실행 시 직접 참조(복사 없음) | `skin_root` 후보에 "실행 파일 옆 `skins/`" 와 개발용 `CARGO_MANIFEST_DIR` 상대 경로 추가, `release.yml` 에 폴더 동봉 | 바이너리 크기 불변, 복사 없음, 스킨 수정이 바로 반영 | 배포물이 "바이너리 + 폴더" 가 됨(tar/zip 구조 변경). 읽기 전용 위치일 수 있으므로 스킨이 쓰는 파일(`M:io/**`, `M:History/**`)의 쓰기 경로를 사용자 폴더로 돌려야 함. git 저장소가 수백 MB 증가(현재 `.git` 15MB). GitHub 단일 파일 100MB 제한에는 걸리지 않음(최대 48.4MB) |
| C. 저장소 폴더 + 최초 실행 시 사용자 폴더로 복사 | B + 디렉터리 복사 루틴(현 `install_bundle` 을 파일 표 대신 디렉터리 순회로) | 사용자가 고칠 수 있는 사본 확보, 스킨의 쓰기 경로 문제 없음 | 디스크 2배, 최초 실행 시 수 초 복사, 갱신 정책(덮어쓰기 금지 규칙과 충돌) 재설계 필요 |
| D. 앱 번들 리소스(macOS `.app/Contents/Resources`, Windows 설치 폴더) | 릴리스 파이프라인에 번들 생성 추가 | OS 관례에 맞음 | 현재 `.app` 번들을 만들지 않음(`release.yml` 은 맨 바이너리). 서명·공증 범위가 늘어남. 번들 내부는 읽기 전용 취급 |
| E. 별도 다운로드(최초 실행 또는 설정 화면에서) | 다운로드·검증·압축 해제 루틴 신설, 호스팅 필요 | 바이너리·저장소 모두 가벼움 | 오프라인 최초 실행에 스킨이 없음 → 최소 폴백 화면이 반드시 필요. 호스팅·체크섬·실패 처리 필요 |

판단 재료:

- 사용자 지시는 "제공한 스킨을 조금 더 심플하게 해서 기본 스킨으로". 단순화 판에서 mp4 를 빼면 285MB, 폰트·대형 PNG 를 더 줄이면 수십 MB 까지 내려갈 여지가 있다(실제 감량 폭은 미확인, 스킨 본체 조사 담당의 몫).
- 어느 방식이든 "스킨이 하나도 없을 때의 화면"을 정해야 한다. 지금은 내장 네이티브 화면이 그 역할을 하지만, 단독 소유 전환 뒤에는 (a) 네이티브 화면을 폴백으로 남길지 (b) 아주 작은 내장 스킨을 따로 둘지 결정해야 한다.
- 재배포 권리: ModernChic 폴더에는 폰트 라이선스(`M:Decide/SIL_Open_Font_License_1.1.txt` 등)와 `readme.txt` 가 폴더마다 있고 `M:Sound/使用させていただいた楽曲.txt` 가 있다. 이미지·동영상·음원의 재배포 조건은 이번 범위에서 읽지 않았다(미확인). GPL-3.0 저장소에 넣기 전에 확인이 필요하다.

---

## 4. SKIN 탭 커스터마이즈 저장 구조

- 고정 행 5개는 `rbms_config::SETTINGS` 에 있고(SCREEN / SKIN / LOADED / RELOAD / RESET — `SettingId::SkinScreen, SkinDocument, SkinInfo, SkinReload, SkinReset`, 기본 표시값은 `settings_ui.rs:248-252`), 문서가 선언한 행은 `SkinReload` 행 바로 위에 끼워 넣는다(`SKIN_CUSTOM_ANCHOR`, `settings_ui.rs:19, 115-126`).
- 문서 선언 행 종류 `SkinRow`: `BundleProperty / BundleFile / BundleOffset(axis)` + `Property / File / Offset(axis)` (`skin_select.rs:158-171`). 축은 `X, Y, W, H, R, A` (`skin_select.rs:79-87`), 한 스텝 1.0, 한계 ±2000(위치·크기) / ±360(각도) / ±255(알파) (`skin_select.rs:42-53`).
- 저장 형태 `SkinCustomisation { properties: BTreeMap<String,i32>, filepaths: BTreeMap<String,String>, offsets: BTreeMap<i32,SkinOffset> }` (`schema.rs:223-232`). property 는 "행 이름 → 선택된 op id", file 은 "슬롯 이름 → 후보 파일명", offset 은 "offset id → 6축 값".
- 두 스코프: `custom[문서 절대경로]`(문서 단위)와 `shared[번들키]`(번들 단위). 번들키는 "스킨 폴더 바로 아래 디렉터리 이름"(`bundle_key`, `schema.rs:339-343`). 문서가 스킨 폴더 바로 아래에 있으면(디렉터리 1단 없음) 번들키가 없다(`first_directory_name`, `schema.rs:289-294`).
- 로더에 넘기는 값: `user_config(path)` = 번들 공유값 위에 문서 값을 덮은 `SkinUserConfig` (`schema.rs:373-387`).
- 번들 공유 행은 문서 헤더에서 `scope: 'bundle'` 로 선언된 것만이다(`shared_scope`, `skin_select.rs:766-768`). 이것은 R-BMS 자체 확장이며 레퍼런스 스킨에는 없다.
- 레퍼런스 대응: 화면 타입마다 `SkinConfig { path, properties { option[], file[], offset[] } }` 하나, 스킨을 바꾸면 이전 설정을 `skinHistory` 에 보관(`B:config/SkinConfiguration.java:151-197, 205-215, 236-244`). 옵션은 이름→값, 파일은 이름→경로, 오프셋은 id→(x,y,w,h,r,a) (`B:config/SkinConfiguration.java:250-378`). 저장 모델 자체는 R-BMS 의 `custom[path]` 와 1:1 로 맞는다.
- 차이: ModernChic 은 같은 이름의 옵션을 여러 `.luaskin` 이 각자 선언한다(스킨 본체 조사 담당 확인 사항). 레퍼런스는 화면 타입별로 따로 저장하므로 화면마다 값이 독립이다. R-BMS 의 번들 공유 스코프는 `.luaskin` 에는 적용되지 않는다(선언 문법이 없음).

---

## 5. "스킨 폴더 하나를 가리켜 10개 .luaskin 을 화면별로 쓰기" 위해 바꿀 곳

| # | 바꿀 곳 | 현재 | 필요한 변경 |
| --- | --- | --- | --- |
| 1 | `skin_select.rs:39` `DOCUMENT_EXTENSIONS` | `["json","json5"]` | `luaskin` 추가. 레퍼런스는 `.json`, `.luaskin`, 그 외(LR2)만 본다(`B:config/SkinConfiguration.java:489-507`) |
| 2 | `skin_select.rs:727-755` `scan` → `load_header` | JSON 헤더 파서만 | `.luaskin` 은 `skin_config == nil` 로 실행해 `t.header` 를 받는 경로 필요(`M:play7_hw.luaskin:2-5`). 헤더 126개 Lua 를 스캔마다 실행하면 느리므로 캐시 고려 |
| 3 | `R:crates/rbms-skin/src/loader.rs:221, 240-242` `is_supported_skin_type` | 5, 6, 7, 8 + play 모드 | 9(SKIN SELECT), 15(COURSE RESULT) 추가. 라벨 표에는 이미 있음(`schema.rs:185-205`) |
| 4 | `skin_select.rs:682-705` `reload_for` | 프레임 루프에서 동기 `load_skin` | 2.5만 줄 Lua 를 실행하므로 워커 스레드로 이동 필요(샌드박스 `Send` 여부는 미확인) |
| 5 | `skin_select.rs:694-695` 샌드박스 루트 | 스킨 루트 폴더 전체 | 문서별 스킨 폴더 기준 `require`/파일 접근 규칙, 그리고 스킨이 쓰는 `io/`·`History/` 쓰기 허용 범위 결정 |
| 6 | `assets.rs:305-318` `skin_document_is_enabled` | 프리셋과 결합 | 프리셋 결합 제거. "선택돼 있으면 사용" 한 줄로 |
| 7 | `assets.rs:177-186, 367-376` 기본 선택 | 타입 8종에 JSON 문서 지정 | "스킨 팩 폴더를 고르면 그 안의 모든 타입을 한 번에 지정" 하는 함수로 교체. 현재 SKIN 탭은 화면 타입마다 따로 골라야 한다(`skin_select.rs:620-634`) |
| 8 | `schema.rs:339-343` `bundle_key` | 스킨 루트 아래 1단 디렉터리 | `config.skin.folder` 를 스킨 팩 폴더 자체로 지정한 경우(문서가 루트 직하) 번들키가 없어짐. 팩 단위 식별자를 "문서의 부모 디렉터리"로 재정의하거나, 폴더 지정은 "팩들의 부모" 로 고정 |
| 9 | `stage/course_result.rs:99-118` | 스킨 호출 없음 | `prepare_skin(15)` + 상태 공급 + 타이머 |
| 10 | 신규 Stage(SkinSelect) | 없음 | 타입 9 문서를 그리고 `BUTTON_CHANGE_SKIN`, 커스터마이즈 버튼, 슬라이더(`M:skinselect.lua:122` `SKINSELECT_POSITION`), 스킨 타입 선택 버튼을 처리(`B:config/SkinConfiguration.java:114-149`) |
| 11 | `stage/loading.rs` | decide 문서를 로딩 화면으로 사용 | decide 를 독립 장면으로 분리하고 로딩은 play 스킨의 PRELOAD 단계로 이동(§6.5) |
| 12 | `stage/play/mod.rs:797` `mode_skin_type` | 모드 → 타입 0/1/2/3/4/16 | ModernChic 에 9키·24키 문서가 없음. 문서가 없는 모드의 폴백 정책 필요 |
| 13 | `app_input.rs:282-329` `rebuild_skin`/`document_field` | 문서의 `note.dst` 로 내장 필드 좌표를 덮음 | 스킨 단독 소유면 내장 `Skin`(RON 필드) 자체가 필요 없어짐. 키 봄·레인 판정 측(`field_side`, `stage/play/mod.rs:243-248`)이 내장 `Skin.x/fields` 에 의존하므로 대체 좌표원이 필요 |
| 14 | `lib.rs:218-223` `sound_folder_path` / `assets.rs:321-327` | 프리셋이 STEEL NEON 일 때만 번들 `sound/` | 선택된 스킨 팩의 사운드 폴더 규칙으로 교체(§10.1) |

---

## 6. 화면별 상태 공급과 타이머

### 6.1 공통 뼈대

- 시계: `skin_now_ms()` = `self.clock.elapsed().as_millis()` (`skin_screen.rs:567-569`). `clock` 은 `App::new`(`lib.rs:810`)와 `start_play()`(`app_play.rs:673-679`)에서만 다시 잡힌다.
- 한 프레임 조립: `skin_frame()` 이 `SkinDraw { screen, timers: &self.skin_timers, now_ms, lua, mouse, background, offsets, extra }` 를 만든다(`skin_screen.rs:676-699`). `mouse` 는 논리 커서를 문서 좌표(y 상향, 저작 크기)로 바꾼 값(`document_cursor`, `skin_screen.rs:556-563`).
- 화면별 상태는 `SkinStateSource` 구현체(`PlayViewState`, `SelectViewState`, `ResultViewState`, `DecideViewState`, `KeyConfigViewState` — 모두 `rbms_render::skin_render::state`, `skin_screen.rs:22`)와, 프로퍼티 id 로 담기 어려운 것을 싣는 `FrameExtra::{None, Play(&PlayObjectState), Select(&SelectListState), Result(&ResultSeriesState)}` 로 넘긴다.
- 타이머 표는 화면 전환 때 지워지지 않는다(앱 코드에 `skin_timers.clear()` 호출 없음, grep 결과 0건). 레퍼런스는 전환마다 전부 OFF(`B:TimerManager.java:100-106`).

### 6.2 Play

공급 지점: `PlayState::draw` → `draw_document` (`stage/play/mod.rs:593-647`).

| 넘기는 것 | 필드 | 근거 |
| --- | --- | --- |
| `HudView` | `mode_label, combo, last_judge, last_fast, fast[2], slow[2], counts[6], ex_score, gauge, green_number, white_number, judge_text_y, max_ex, best_ex, pace{name, delta}` | `stage/play/mod.rs:841-860` |
| `PlayViewState` | `title, song_ms, duration_ms, bpm, hispeed, autoplay, now_ms, offsets, field, shade, judged_side, gauge_kind, artist, level, bpm_min, bpm_max, bpm_main, target_ex, target_delta` | `stage/play/mod.rs:611-632` |
| `PlayObjectState` (FrameExtra) | `field(&Skin), playfield{timelines, microtime, hispeed, beam_on, beam_off, constant, legacy_note}, shade{cover, hidden}, gauge_kind(0..5), bomb, keys_down, recent_hits(최근 64개)` | `stage/play/mod.rs:633-642, 861-870, 42` |
| 배경 텍스처 | 현재 BGA 프레임 1장(`self.bga.get(&play.bga_frame())`) | `stage/play/mod.rs:809-822` |

타이머:

| 시점 | 호출 | 켜지는 타이머 | 근거 |
| --- | --- | --- | --- |
| `on_enter` | `skin_play_timers.start` | `PLAY` on, `READY` off (즉시) | `stage/play/mod.rs:694-698`, `R:crates/rbms-render/src/skin_render/screen.rs:262-265` |
| 매 프레임(문서가 있을 때만) | `skin_play_timers.update(hud, total_notes, now, lanes)` | `JUDGE_1P/2P, COMBO_1P/2P, FULLCOMBO_1P, GAUGE_MAX_1P, GAUGE_INCLEASE_1P, KEYON/KEYOFF/HOLD/BOMB_{1P,2P}_{SCRATCH,KEY1..KEY10}` | `stage/play/mod.rs:603-606`, `screen.rs:277-` (타이머 id 는 grep 집계) |
| 실패 감지 프레임 | `skin_play_timers.fail` | `FAILED` | `stage/play/mod.rs:600-602` |

주의점:

- 타이머 갱신이 `update` 가 아니라 `draw_document` 안에 있고, 문서가 없으면 함수 초입에서 빠져나가므로(`stage/play/mod.rs:595-597`) 문서 로드 전 프레임의 이벤트는 타이머에 반영되지 않는다.
- 레인 → 타이머 키 번호는 내장 `Skin.x` 를 x 좌표순으로 정렬해 매긴다(`lane_timer_states`, `stage/play/mod.rs:265-286`).

공급하지 못하는 것(레퍼런스 근거 포함):

| 항목 | 레퍼런스 | R-BMS 상태 |
| --- | --- | --- |
| PRELOAD 단계(로딩 진행, `loadstart`/`loadend`) | `B:play/BMSPlayer.java:480-522` | Loading Stage 가 대신함. play 스킨의 로딩 연출이 재생되지 않음 |
| READY 단계(`TIMER_READY`, `playstart`, `PLAY_READY` 사운드 후 대기) | `B:play/BMSPlayer.java:513-515, 605-618` | `on_enter` 에서 사운드만 울리고 즉시 PLAY (`stage/play/mod.rs:695-697`) |
| `TIMER_STARTINPUT` | `B:play/BMSPlayer.java:470-472` | 없음 |
| `TIMER_RHYTHM` | `B:play/BMSPlayer.java:609-610, 628` | 없음(드라이버 목록에 없음) |
| FAILED 후 `close` 시간 동안의 폐점 연출 | `B:play/BMSPlayer.java:693-742` | 실패 즉시 결과로 전환(`stage/play/mod.rs:431-436, 732-734`) |
| `TIMER_MUSIC_END`, `TIMER_ENDOFNOTE_1P`, `finishMargin`, `TIMER_FADEOUT` | `B:play/BMSPlayer.java:679-690, 745-751` | 없음. 종료 즉시 전환 |
| `TIMER_SCORE_A/AA/AAA/BEST/TARGET` | `B:play/BMSPlayer.java:1033-1037` | 없음 |
| `TIMER_PM_CHARA_*` | `B:play/BMSPlayer.java:518-521, 676` | 없음 |
| 라이벌/타깃 스코어 데이터(고스트 포함) | `B:play/BMSPlayer.java:447-453` | `target_ex` 숫자와 pace delta 만 |
| BGA 레이어·POOR 레이어 분리 | (렌더 측, 미확인) | 프레임 1장만 넘김(`stage/play/mod.rs:809`) |
| 동영상 BGA | (렌더 측) | 이미지 확장자만 디코드(`assets.rs:649-651`) |
| 연습 모드 패널(play 스킨 위) | `B:play/BMSPlayer.java:524-595, 988` | 별도 내장 `Practice` Stage |
| 코스 중 게이지/콤보 승계 표시, 코스 제약 표시 | `B:play/BMSPlayer.java:419-424` | 엔진은 승계(`app_play.rs:311, 430-431`)하나 스킨 상태로는 미공급 |

### 6.3 Select

공급 지점: `SelectState::draw` (`stage/select/mod.rs:779-868`) → `draw_select_skin[_layer]` (`skin_screen.rs:777-821`).

- 원천은 `SelectScene`(캐시됨, 키는 `select_key` `stage/select/mod.rs:125-131`): `rows[], sel, header, guide, detail, modal, score_graph, search, sort, filter, empty_hint` (`stage/select/scene.rs:198-210`).
- 행 `SelectRow`: `folder, title, mode_short, mode_color, level, difficulty_color, lamp(색), folder_count, dj_level, favorite` (`scene.rs:35-62`). 램프가 **색 값**이지 램프 id 가 아니다(`scene.rs:49`).
- 상세 `DetailView`: `accent, title, subtitle, artist, genre_maker, mode_short, level, difficulty_name, cover(Present/None), stats[6]{BPM, DENSITY, NOTES, JUDGE, LENGTH, TOTAL}, density, records{plays, clears, best, rank_bar, recent[]}` (`scene.rs:114-136`). 통계 값은 이미 **문자열로 포맷된** 상태다(`scene.rs:83-98, 127-132`).
- 옵션 패널 행: `OptionsRows { labels[11], values[11], focused, open }` (`app_options.rs:218-225`), 행 구성 `Random, Gauge, HiSpeed, FixHiSpeed, LaneCover, Lift, Hidden, ScratchSide, ScratchAuto, Autoplay, Target` (`app_options.rs:29-41`).
- 배경 텍스처: 포커스 곡의 stagefile(없으면 banner) 1장(`stage/select/mod.rs:309-312, 791-804`).

타이머:

| 시점 | 타이머 | 근거 |
| --- | --- | --- |
| 매 프레임 `skin_select_timers.update(row, now)` | `SONGBAR_MOVE, SONGBAR_MOVE_UP, SONGBAR_MOVE_DOWN, SONGBAR_CHANGE` | `stage/select/mod.rs:805-807`, `screen.rs:351-` |
| 옵션 오버레이 열고 닫을 때 | `PANEL1_ON`, `PANEL1_OFF` | `app_options.rs:207-211` |

공급하지 못하는 것:

| 항목 | 레퍼런스 | R-BMS 상태 |
| --- | --- | --- |
| `TIMER_STARTINPUT` (`skin.input` 이후 입력 허용) | `B:select/MusicSelector.java:193-195` | 없음 |
| 패널 2·3 (`PANEL2/3_ON/OFF`): 어시스트 옵션, 상세 옵션 | `B:select/MusicSelector.java:553-565`, `B:select/MusicSelectInputProcessor.java:114-117` 및 그 이후 블록(§7.4) | 패널 1 하나뿐, 내용도 R-BMS 고유 11행 |
| 패널 내 개별 옵션 값(랜덤 1P/2P, DP 옵션, 게이지, HS fix, BGA, 게이지 자동 전환, 판정 타이밍, duration, 타깃 등)의 id 기반 공급 | `B:select/MusicSelectInputProcessor.java` 의 `executeEvent(EventType.*)` 호출들 | 라벨·값 문자열 11쌍만(`app_options.rs:218-225`) |
| IR 랭킹(`RankingData`, 상태 타이머 `IR_CONNECT_BEGIN/SUCCESS/FAIL`, 랭킹 오프셋·스크롤) | `B:select/MusicSelector.java:223-250, 672-694` | 내장 랭킹 패널로만 존재(`stage/select/mod.rs:386-400, 856-859`), 스킨 상태 미공급 |
| 라이벌(선택된 라이벌, 라이벌 스코어 캐시, 라이벌 램프) | `B:select/MusicSelector.java:135-154, 375-376` | 라이벌 id 목록만 설정에 있음(`schema.rs:675`). 라이벌 정렬은 제목순 폴백(`stage/select/list.rs:149`) |
| 코스 바(GradeBar: 구성곡, 제약, 트로피), 랜덤 코스 | `B:select/MusicSelector.java:236-245, 265-275, 571-587` | 코스 탭이 폴더 행으로 그려짐(`scene.rs:218-254`). 제약은 `level` 문자열 배지 |
| 바 종류 구분(곡/폴더/테이블/해시/검색결과/실행형/같은 폴더 등) | `B:select/MusicSelector.java` 의 `SongBar, DirectoryBar, TableBar, HashBar, SearchWordBar, ExecutableBar, SameFolderBar, GradeBar, RandomCourseBar` | `SelectItem::{Song, Folder}` 두 종류(`lib.rs:406-409`) |
| 검색 텍스트 입력(스킨의 편집 가능 텍스트 객체) | `B:skin/Skin.java:394-405`, `B:select/MusicSelector.java:311-321` | 내장 검색 상자(`/` 키, `stage/select/mod.rs:208-247, 587-628`). 스킨에는 표시용 문자열만(`scene.rs:206`) |
| 리플레이 슬롯 4개와 선택된 슬롯 | `B:select/MusicSelector.java:49, 334-340` | 기록별 리플레이 파일(`stage/select/mod.rs:336-361`). 슬롯 개념 없음 |
| 배너와 스테이지파일을 별도 이미지로 | `B:select/MusicSelector.java:636-644` | 둘 중 하나만 1장 |
| 노트 분포 그래프용 BMS 모델(선택 350ms 뒤 로드) | `B:select/MusicSelector.java:212-222` | `ChartDetail.density` 로 일부 대응(150ms 디바운스, `stage/select/mod.rs:295-313`, `lib.rs:165`) |
| 모드 필터 / LN 모드 / 정렬 id | `B:select/MusicSelectInputProcessor.java:86-96` | 내장 필터 패널(`stage/select/filter.rs`)과 `SortMode` 라벨 문자열(`scene.rs:207`) |
| 플레이어 통계(`PlayerData`) | `B:select/MusicSelector.java:161` | 미공급(보유 여부 미확인) |
| select BGM(`SELECT` 사운드를 프리뷰 기본음으로 루프) | `B:select/MusicSelector.java:173-174, 187-189` | 없음. `SystemSound::Select` 는 곡 결정 시 1회 효과음으로 사용(`stage/select/mod.rs:147`) |

### 6.4 Result

공급 지점: `ResultState::draw` (`stage/result.rs:113-143`) → `draw_result_skin[_layer]` (`skin_screen.rs:842-878`).

- `ResultView`: `title, artist, mode_label, counts[6], ex_score, max_score, max_combo, total_notes, fast[2], slow[2], gauge, clear_label, clear_color, prev_best_ex, prev_ex, show_graph, show_result_graphs, gauge_series, timing_hist, judge_dist` (`app_result.rs:106-127`).
- `ResultViewState::new(view, target, cleared, now_ms, offsets).with_hint(run_again).with_ir_status(text)` (`skin_screen.rs:844-846`). IR 상태는 여러 줄을 공백 3칸으로 이어 붙인 한 줄 문자열(`skin_screen.rs:375, 648-650`).
- `ResultSeriesState { gauge_series, timing_hist, judge_dist, bpm_points }` (`stage/result.rs:60-67`).

타이머: `on_enter` 에서 `skin_result_timers.enter(view, now)` — `RESULTGRAPH_BEGIN` on, 신기록이면 `RESULT_UPDATESCORE` on(`stage/result.rs:89-93`, 테스트 `stage/result.rs:273-306`). 매 프레임 `update(now)` — 1000ms 뒤 `RESULTGRAPH_END` on(`stage/result.rs:116-117, 283-286`).

공급하지 못하는 것:

| 항목 | 레퍼런스 | R-BMS 상태 |
| --- | --- | --- |
| `TIMER_STARTINPUT`, 입력 잠금, `scene` 자동 종료, `TIMER_FADEOUT` + `fadeout` 후 전환 | `B:result/MusicResult.java:168-173, 250-258, 267-298` | 없음. 키 즉시 전환(`stage/result.rs:101-111`) |
| `RESULT_UPDATESCORE` 의 2단계 동작(`rankTime != 0` 이면 첫 입력에 켜고 두 번째 입력에 종료) | `B:result/MusicResult.java:165-167, 286-298` | 진입 시 한 번 결정 |
| `RESULTGRAPH_BEGIN/END` 를 진입 즉시 둘 다 on | `B:result/MusicResult.java:162-163` | END 를 1000ms 뒤에 켬(동작 차이) |
| IR 처리 상태와 `IR_CONNECT_*` 타이머, 랭킹·순위·이전 순위·총 인원, 랭킹 스크롤 | `B:result/MusicResult.java:75-144`, `B:result/AbstractResult.java:217-270` | 상태 문자열 한 줄만 |
| 게이지 종류 전환(CHANGE_GRAPH 키로 게이지 그래프 종류 순환) | `B:result/MusicResult.java:271-276` | 없음. 게이지 로그도 1종만(`app_result.rs:124`) |
| 리플레이 저장 슬롯 4개 상태(`ReplayStatus`)와 NUM1~4 저장 | `B:result/AbstractResult.java:56-57, 71-72`, `B:result/MusicResult.java:300-308` | 자동 저장 1개(`app_result.rs:85-91, 208-238`) |
| 이전 스코어 전체(`oldscore`: 클리어, 콤보, minbp 등) | `B:result/AbstractResult.java:64, 239-241` | `prev_best_ex`, `prev_ex` 만 |
| 결과 BGM 루프, `RESULT_CLOSE` 를 fadeout 시작에 재생 | `B:result/MusicResult.java:150-158, 252-256` | 진입 1회 재생, 나갈 때 `ResultClose`(`stage/result.rs:89-97`) |
| 코스 결과 전용 상태 | `B:result/CourseResult.java` (본문 미열람) | 내장 행 목록(`stage/course_result.rs:43-56`) |

### 6.5 Decide(=Loading)와 KeyConfig

- Decide: `DecideViewState { progress, done, title, chart{subtitle, artist, genre, level}, now_ms, offsets }` (`skin_screen.rs:881-892`, `stage/loading.rs:393-417`). `chart` 는 `LoadingTask::Song` 일 때만 채워지고 `Assets` 단계에서는 기본값이다(`stage/loading.rs:393-403`). 타이머 없음.
- 레퍼런스 decide 는 로딩과 무관한 시간 장면이다: `input` 경과 후 `STARTINPUT`, `scene` 경과 또는 키 입력으로 `FADEOUT`, `fadeout` 경과 후 PLAY(`B:decide/MusicDecide.java:37-63`). 진입 시 `DECIDE` 사운드(`B:decide/MusicDecide.java:32-35`). R-BMS 는 로딩 **완료** 시 `Decide` 사운드를 울린다(`stage/loading.rs:463`).
- KeyConfig: `KeyConfigViewState { keys: &[String], now_ms, offsets }` (`skin_screen.rs:895-906`). 포커스 행, 캡처 중 여부, 편집 모드, 컨트롤러/MIDI 바인딩 등은 미공급. 입력은 내장 행 모델(`KcRow`, `lib.rs:361-386`) 기준으로만 동작(`stage/keyconfig.rs:135-180`).

### 6.6 공통으로 빠진 장면 속성

레퍼런스 스킨 객체가 제공하고 각 상태가 읽는 장면 시간: `getInput()`, `getScene()`, `getFadeout()` (select/decide/result 공통, `B:select/MusicSelector.java:193`, `B:decide/MusicDecide.java:39-47`, `B:result/MusicResult.java:168-173, 250`), play 전용 `getLoadstart()`, `getLoadend()`, `getPlaystart()`, `getClose()`, `getFinishMargin()` (`B:play/BMSPlayer.java:499, 606, 711, 748`), result 전용 `getRankTime()` (`B:result/MusicResult.java:165`). R-BMS 앱은 이 값들을 읽는 곳이 없다(앱 소스 grep 결과 0건). 로더가 값을 보존하는지는 렌더·로더 조사 담당 범위(미확인).

---

## 7. 입력

### 7.1 분배 구조

| 장치 | 진입 | 분배 | 근거 |
| --- | --- | --- | --- |
| 키보드 | `WindowEvent::KeyboardInput` → `KeyInput { code(물리 키), pressed(반복 제외), released, text }` | `Stage::handle_key`: 옵션 오버레이가 먼저(`app_options::options_key`), 남으면 화면 | `lib.rs:1045-1058`, `stage/mod.rs:123-128, 263-269` |
| 마우스 이동 | `CursorMoved` → `gpu.logical_from_physical` → `shared.cursor` (논리 1280x720) | 상태만 저장. 화면에 이벤트 전달 없음 | `lib.rs:1034-1038`, `R:apps/rbms-player/src/gpu/mod.rs:511-513, 624-627` |
| 마우스 클릭 | `MouseInput { Pressed, Left }` 만 | `Stage::handle_mouse(at)`: 옵션 오버레이가 열려 있으면 삼킴 | `lib.rs:1039-1044`, `stage/mod.rs:276-281` |
| 마우스 release / 우클릭 / 가운데 / 휠 | 처리 없음(`_ => {}`) | — | `lib.rs:1065` |
| 게임패드 | 프레임마다 `poll_pad()` → `PadEvent::{Lane{lane,dir,press}, Control(action)}` | `Stage::handle_pad` — **`PlayState` 만 구현** | `lib.rs:918-921`, `R:apps/rbms-player/src/gamepad.rs:302-305`, `stage/play/mod.rs:744-750`, `stage/mod.rs:145-148` |
| IME | 처리 없음(`WindowEvent::Ime` 매치 없음, grep 0건) | — | `lib.rs:1026-1067` |

### 7.2 클릭 판정 구조

- 즉시 모드: 매 프레임 `shared.hot: Vec<(Rect, Hot)>` 를 비우고(`lib.rs:932`) 화면이 그리면서 채운다. 클릭 시 `hit_test` 가 뒤에서부터(위에 그려진 것 우선) 찾는다(`app_play.rs:577-580`).
- `Hot` 은 닫힌 열거 14종: `SelectRow, RecordRow, SettingTab, SettingRow, RivalRow, RankingRow, ModalClose, ModalReplay, NavSearch, NavSort, NavFolders, NavTables, NavRecords, NavSettings` (`lib.rs:419-434`).
- 스킨 핫스팟: select 화면만 소비한다. `select_hotspots()` 가 문서의 핫스팟을 캔버스 좌표로 받아 `SkinHotAction` 9종을 `Hot` 으로 사상(`skin_screen.rs:829-839`, `stage/select/mod.rs:842-855`). 문서가 `list` 나 `topbar` 블록을 대체했을 때만 반환한다(`skin_screen.rs:830-833`).
- `handle_mouse` 구현 화면은 Select 와 Settings 둘뿐(`stage/select/mod.rs:739`, `stage/settings.rs:634`).

### 7.3 레퍼런스의 마우스·텍스트 계약

- 매 프레임(ms 단위 틱) `current.input()` 뒤에 `skin.mousePressed(state, button, x, y)` 와 `skin.mouseDragged(...)` 를 부른다(`B:MainController.java:495-508`).
- `Skin.mousePressed`: 객체 배열을 **뒤에서 앞으로**(위에 그려진 것 먼저) 돌며, 편집 가능한 `SkinText` 의 입력 영역이면 텍스트 입력 포커스, 아니면 `obj.draw && obj.mousePressed(...)` 가 true 인 첫 객체에서 멈춘다(`B:skin/Skin.java:394-411`). 텍스트 입력 중 영역 밖 클릭은 커밋(`B:skin/Skin.java:395-397`).
- `Skin.mouseDragged`: `SkinSlider` 객체에만 전달(`B:skin/Skin.java:413-419`).
- 휠: select 는 곡 바 스크롤(`B:select/MusicSelectInputProcessor.java:155-156`), skin config 는 커스텀 옵션 오프셋(`B:config/SkinConfiguration.java:69-76`), result 는 랭킹 오프셋(`B:result/AbstractResult.java:247-254`, `mov` 의 원천은 미확인).
- 플레이 중 5초간 마우스가 안 움직이면 커서 grab(`B:MainController.java:510-515`).
- 화면 모드 전환 키(`SWITCH_SCREEN_MODE`)로 풀스크린/창 전환(`B:MainController.java:522-531`).
- 스킨 이벤트 실행: `MainState.executeEvent(id, arg1, arg2)` — 커스텀 이벤트 id 면 `skin.executeCustomEvent`, 내장이면 `EventType.event.exec` (`B:MainState.java:82-106`). SkinConfiguration 은 이를 오버라이드해 `BUTTON_CHANGE_SKIN`, 커스터마이즈 버튼, 스킨 타입 선택 id 를 처리(`B:config/SkinConfiguration.java:114-149`).

### 7.4 레퍼런스 select 의 키 계약(스킨 패널과 직결)

`B:select/MusicSelectInputProcessor.java`:

- NUM0 검색 팝업(71-83), NUM1 모드 필터(86-88), NUM2 정렬(90-92), NUM3 LN 모드(94-96), NUM4 또는 NEXT_REPLAY 리플레이 슬롯 순환(109-113).
- START 만 누름 → `setPanelState(1)` + `OPTION_OPEN` 사운드, 레인 키로 `option1p / gauge1p / optiondp / option2p / hsfix / target` 이벤트(114-204 부근).
- SELECT 만 누름 → `setPanelState(2)`, 레인 키로 customJudge / constant / judgearea / legacynote / marknote / bpmguide / nomine 토글(205-240 부근).
- NUM5 또는 START+SELECT → `setPanelState(3)`, 레인 키로 bga / gaugeautoshift / notesdisplaytiming / duration(241-285 부근).
- 아무것도 안 누름 → `setPanelState(0)`, PLAY/PRACTICE/AUTO/REPLAY 키와 폴더 열기·닫기, NUM7 라이벌, NUM8 같은 폴더, NUM9 문서 열기(286-330 부근).
- 옵션 키를 떼면 `OPTION_CLOSE`(100-107).

R-BMS select 는 화살표·Enter·문자 단축키(`/`, F2, F3, F4, Tab, O, T, R, I, F)만 받고(`stage/select/mod.rs:702-733`), 레인 키와 START/SELECT 개념이 없다. 옵션 패널은 LeftShift 홀드 또는 F1 고정(`app_options.rs:89-92`).

### 7.5 붙일 지점 제안

| 기능 | 붙일 곳 | 내용 |
| --- | --- | --- |
| 객체 클릭(`act`/click 이벤트) | `lib.rs:1039-1044` 에서 버튼 종류를 넘기고, `Stage::handle_mouse` 에 `button` 인자 추가 → 공통 함수 `AppShared::skin_click(screen, state, extra, at, button) -> Option<(event_id, arg)>` 를 `skin_screen.rs` 에 신설 | `skin_hotspots()`(`skin_screen.rs:708-726`)와 같은 방식으로 프레임을 조립해 "그려진 객체를 역순으로" 판정. 결과는 화면별 `execute_event(id, arg1, arg2)` 로 전달. 현재 `Hot` 열거를 거치는 사상(`stage/select/mod.rs:843-853`)은 닫힌 집합이라 확장 불가 |
| 슬라이더 드래그 | `MouseInput { Released }` 와 "버튼이 눌린 상태의 `CursorMoved`" 를 `StageHandler::handle_mouse_drag(at)` 로 전달 | `AppShared` 에 `mouse_down: Option<MouseButton>` 추가. 레퍼런스처럼 슬라이더 객체에만 전달 |
| 휠 | `WindowEvent::MouseWheel` → `StageHandler::handle_scroll(delta)` | select 는 곡 바 이동, skin select 는 옵션 오프셋, result 는 랭킹 오프셋 |
| 호버 | 이미 `SkinDraw.mouse` 로 전달 중(`skin_screen.rs:694`) | 추가 작업 없음. 다만 논리 좌표가 1280x720 기준이므로 해상도 변경 시 `document_cursor` 인자만 따라가면 됨 |
| 텍스트 입력 | `KeyInput.text`(`lib.rs:1053`) + 기존 `TextEdit`(`R:apps/rbms-player/src/textedit.rs:24-147`: insert/backspace/delete/left/right/home/end/paste/window) 재사용 | 스킨의 편집 가능 텍스트 객체가 포커스를 잡으면 `holds_keys()` 가 true 를 돌려 옵션 오버레이 키를 막는 기존 계약(`stage/mod.rs:160-166`)을 그대로 사용. 일본어·한국어 검색이 필요하면 `window.set_ime_allowed(true)` 와 `WindowEvent::Ime` 처리 추가 필요 |
| 게임패드/레인 키로 select 조작 | `SelectState::handle_pad` 신설, 키보드 레인 키는 `shared.lane_for(code)`(`app_input.rs:99-101`) 재사용 | START/SELECT 바인딩이 `keyconfig` 에 있는지는 미확인(`keyconfig.rs` 미열람) |

---

## 8. 내장 UI 와 스킨 문서의 혼합 합성 — 걸린 코드 전부

### 8.1 개념

- 문서 헤더 `composition`: `replace`(문서가 화면 전체), `overlay`(내장 화면 위에 덧그림), `layered`(문서 background → 내장 → 문서 foreground). 기본 번들은 decide 만 `replace`, 나머지는 전부 `layered` + `replace: [...]` 블록 목록(`R:assets/skins/steel-neon-v3/*.json5:7-8`).
- `replace` 블록 목록은 "내장 출력 중 이 이름의 덩어리는 문서가 대신 그린다" 는 선언이고, 문서가 필요한 객체를 실제로 컴파일했을 때만 내장 쪽이 비켜선다(`skin_screen.rs:161-214, 322-367`).

### 8.2 앱 쪽 코드(제거·수정 대상)

| 파일 | 줄 | 내용 | 처리 |
| --- | --- | --- | --- |
| `apps/rbms-player/src/skin_screen.rs` | 130-140 | `BuiltScreen.content: ScreenContent` | 제거 |
| 〃 | 142-159 | `ScreenKind`, `screen_kind()` | 제거 |
| 〃 | 161-214 | `Replacement` 구조체와 `met_by` | 제거 |
| 〃 | 216-292 | `replacement(screen, name)` 대체 표(play 8, select 4, result 9) | 제거 |
| 〃 | 294-320 | `OPTION_PANEL_IDS` | 제거 |
| 〃 | 322-367 | `block_replaced`, `screen_content_of` | 제거 |
| 〃 | 518-524 | 컴파일 뒤 대체 미충족 경고와 `content` 저장 | 제거 |
| 〃 | 527-531 | `SkinScreens::content` | 제거 |
| 〃 | 595-626 | `skin_uses_overlay`, `skin_uses_native_layout`, `skin_uses_layered_layout`, `screen_content` | 제거 |
| 〃 | 654-656 | `skin_draws_background` | 제거(문서가 항상 배경 담당) |
| 〃 | 746-748, 790-792, 851-853 | overlay 가 아니면 `clear(BLACK)` 분기 | 무조건 clear 로 단순화 |
| 〃 | 753-771, 797-821, 857-878 | `draw_*_skin_layer` 3종 | 제거 |
| 〃 | 829-839 | `select_hotspots` 의 `content.list/topbar` 게이트 | 게이트 제거, 일반 클릭 판정으로 교체(§7.5) |
| `apps/rbms-player/src/stage/play/mod.rs` | 805-822 | `overlay/layered/native_layout/built_in_background` 계산과 BGA 슬롯 3분기 | 문서 텍스처 경로 하나로 |
| 〃 | 872-874 | `!native_layout && draw_document(...)` 조기 반환 | 유일 경로로 |
| 〃 | 875-904 | `content` 조회, layered background, 내장 `render_playfield_on_background`, `render_lane_cover`, `render_key_bomb`, `render_hud_with_content`, foreground/overlay 재그리기 | 제거(키 봄·분석 오버레이의 귀속은 §8.5) |
| 〃 | 593, 643-646 | `draw_document(..., layer: Option<SkinLayer>)` | `layer` 인자 제거 |
| `apps/rbms-player/src/stage/select/mod.rs` | 787-804 | `overlay/layered/native_layout` 과 커버 슬롯 3분기 | 단일 경로 |
| 〃 | 808-810 | `!native_layout && draw_select_skin` 조기 반환 | 유일 경로 |
| 〃 | 811-826 | `content`, layered 분기, `render_select_on_background_with_content`, `render_select` | 제거 |
| 〃 | 827-855 | 내장 hot 사상과 스킨 핫스팟 사상 | 일반 이벤트로 교체 |
| 〃 | 856-862 | 내장 랭킹 패널, 필터 패널 | §8.5 결정 필요 |
| 〃 | 863-867 | foreground/overlay 재그리기 | 제거 |
| `apps/rbms-player/src/stage/result.rs` | 119-142 | `overlay/layered/native_layout`, `content`, `render_result_on_background_with_content`, `render_result_with_palette`, IR 줄(`!content.ir`), foreground/overlay | 단일 경로 |
| `apps/rbms-player/src/app_options.rs` | 272-275 | `screen_content(...).select.options` 면 내장 패널 생략 | 내장 패널 자체의 존폐에 따라 정리. `draw` 전체(272-307)가 내장 UI |
| `apps/rbms-player/src/assets.rs` | 305-318 | `skin_document_is_enabled` 프리셋 게이트 | 단순화 |
| `apps/rbms-player/src/app_input.rs` | 282-329 | `rebuild_skin` 의 `with_document_lanes`, `document_field` | 내장 필드 의존 제거와 함께 재설계 |
| `apps/rbms-player/src/stage/loading.rs` | 482-509 | 문서가 못 그리면 내장 진행 막대 | 폴백 정책에 따라 |
| `apps/rbms-player/src/stage/keyconfig.rs` | 187-254 | 문서가 못 그리면 내장 행 목록 | 〃 |

### 8.3 라이브러리 쪽 연관 심볼(다른 조사 범위, 영향 파악용)

패턴 `SkinComposition|ScreenContent|PlayContent|SelectContent|ResultContent|replace_names|draw_layer|SkinLayer|_with_content|SkinHotAction|SkinHotspot|hotspot` 의 파일별 출현 수(grep 집계):

| 파일 | 출현 | 전체 줄 |
| --- | --- | --- |
| `crates/rbms-render/src/content.rs` | 7 | 57 |
| `crates/rbms-render/src/hud.rs` | 14 | 466 |
| `crates/rbms-render/src/result.rs` | 16 | 899 |
| `crates/rbms-render/src/select.rs` | 8 | 704 |
| `crates/rbms-render/src/lib.rs` | 3 | 354 |
| `crates/rbms-render/src/skin_render/mod.rs` | 22 | 425 |
| `crates/rbms-render/src/skin_render/state.rs` | 17 | 924 |
| `crates/rbms-render/src/skin_render/screen.rs` | 3 | 417 |
| `crates/rbms-render/src/skin_render/songlist.rs` | 5 | 374 |
| `crates/rbms-render/src/skin_render/object.rs` | 2 | 850 |
| `crates/rbms-skin/src/loader.rs` | 10 | 693 |
| `crates/rbms-skin/src/model.rs` | 8 | 522 |

### 8.4 단독 소유 전환 시 반드시 정해야 하는 것

1. **문서가 아직 컴파일되지 않은 프레임**: 지금은 내장 화면이 그려진다(`skin_screen.rs:473-479`). 전환 뒤 선택지: 검은 화면 / 이전 화면 유지 / 화면 전환 자체를 컴파일 완료까지 지연(레퍼런스는 `create()` 에서 동기 로드, `B:MainController.java:275-278`).
2. **문서 로드 실패**: 지금은 내장 화면 + LOADED 행에 사유(`skin_select.rs:701-703`). 전환 뒤 폴백 화면이 없으면 조작 불능이 된다.
3. **스킨이 그리지 않는 R-BMS 고유 UI**: 아래 §8.5.

### 8.5 스킨 계약에 대응물이 없는 R-BMS 고유 UI

| UI | 위치 | 선택지 |
| --- | --- | --- |
| 옵션 오버레이(11행) | `app_options.rs:272-307` | 스킨의 패널 1~3 으로 대체하고 폐지 / 스킨 위 시스템 오버레이로 유지 |
| 기록 모달(R 키) | `stage/select/mod.rs:316-361`, 렌더는 `render_select` 내부 | 폐지(스킨의 리플레이 슬롯으로 대체) / 시스템 오버레이 |
| IR 랭킹 패널(I 키) | `stage/select/mod.rs:363-447, 856-859` | 스킨의 랭킹 객체로 대체 / 시스템 오버레이 |
| 필터 패널(F2) | `stage/select/filter.rs`, `stage/select/mod.rs:860-862` | 스킨의 모드 필터 등으로 일부 대체 / 시스템 오버레이 |
| 검색 상자(`/`) | `stage/select/mod.rs:208-247` | 스킨의 편집 텍스트로 대체 / 시스템 오버레이 |
| 코스 탭(Shift+Tab) | `stage/select/mod.rs:709-712`, `scene.rs:218-254` | 레퍼런스처럼 곡 목록 안의 코스 폴더/바 로 통합 |
| 리플레이 분석 오버레이 | `stage/play/mod.rs:661-688, 897-899` | 시스템 오버레이로 유지(스킨 계약에 없음) |
| 키 봄 | `render_key_bomb`, `stage/play/mod.rs:895` (결정 D16 "항상 네이티브", `docs/HANDOFF.md:42`) | 스킨의 bomb 타이머 객체로 대체하고 폐지 |
| 토스트, 서버 연결 점, 디버그 패널 | `lib.rs:948-976` (좌표가 `CW` 기준, 954) | 시스템 오버레이로 유지. 좌표계만 새 논리 해상도에 맞춤 |
| Settings / Tables / Folders / Practice / Loading(scan·table) 화면 | 각 `stage/*.rs` | 스킨과 무관한 내장 화면으로 유지할지, 시각적으로 스킨과 어울리게 다시 그릴지 결정 필요(§12) |

### 8.6 영향받는 테스트

| 파일 | 줄 수 | 테스트 수(이름 기준) | 성격 | 예상 처리 |
| --- | --- | --- | --- | --- |
| `apps/rbms-player/src/stage/render_tests_skin.rs` | 412 | 8 | overlay 가 브라우저와 hot 을 유지, 빈 overlay 는 픽셀 불변, 문서 읽는 동안 내장 화면, 기본 번들 캡처 9장 | 대부분 폐기 또는 재작성 |
| `.../render_tests_skin_v3_select.rs` | 316 | 10 | v3 select 문서 무경고 컴파일, "모든 블록 대체", 휠·클릭, 옵션 패널 대체, 테마 배치 일치 | 번들 교체와 함께 폐기 |
| `.../render_tests_skin_v3_decide_result.rs` | 212 | 7 | v3 decide/result 문서 | 폐기 |
| `.../render_tests_skin_v3_play_sp.rs` | 418 | 12 | v3 5/7/9키 문서, PLAY SIDE·GRAPH POSITION·BGA SIZE 행 | 폐기 |
| `.../render_tests_skin_v3_play_dp.rs` | 281 | 6 | v3 10/14키 문서 | 폐기 |
| `.../stage/select/tests.rs` | 657 | (일부) | `write_blocks_document`(476-517), `blocks_app`(536-), 옵션 대체 유무 비교(644) | 해당 부분 폐기 |
| `.../stage/play/tests.rs` | 792 | (패턴 4회) | 플레이 문서와 내장 필드 연동 | 해당 부분 수정 |
| `.../stage/render_tests.rs` | 341 | 약 16 | "모든 Stage 가 프레임을 그린다", "두 Stage 가 같은 프레임이 아니다", hot 영역, 배경 슬롯 | 내장 화면 폴백 유지 여부에 따라 |
| `.../stage/render_tests_play.rs`, `render_tests_result.rs`, `render_tests_select.rs`, `render_tests_shell.rs` | 157 / 74 / 85 / 193 | — | 내장 화면 스냅샷 | 내장 화면 존폐에 따라 |
| `.../skin_screen/tests.rs` | 139 | 5 | 커서 좌표 변환, Lua 예산, 디스크 미접근, 오프셋 병합 | 유지(좌표 상수만 수정) |
| `.../skin_select/tests.rs`, `fixtures.rs` | 444 / 101 | — | SKIN 탭 행·번들 스코프 | `.luaskin` 추가에 맞춰 보강 |
| `.../stage/settings/skin_tests.rs` | 246 | 6 | SKIN 탭 행 추가·저장·RESET | 유지·보강 |
| `apps/rbms-player/src/assets.rs` 내 `mod tests` | 666-1096 | 약 12 | 세대 설치·이동·보존, `RESULT_REPLACEMENT_CONTRACT`(1013-1018) | 설치 방식 교체와 함께 대부분 재작성 |
| `apps/rbms-player/src/app_options.rs` 내 tests | 309-629 | 약 20 | 오버레이 키·그리기 | 오버레이 존폐에 따라 |
| `crates/rbms-render/tests/golden_*.rs`, `skin_render.rs`, `crates/rbms-skin/tests/skin_nested.rs`, `skin_model.rs` | — | — | 내장 화면 골든, 문서 대체 계약 | 다른 조사 범위. 영향 큼 |

참고: 저장소 문서 기준 전체 테스트는 3,181개(`docs/HANDOFF.md:19`). 헤드리스 캔버스 크기는 테스트 전반에서 `CW`/`CH` 를 쓴다(§9.2 표).

---

## 9. 창 크기·해상도·풀스크린

### 9.1 현재 동작

- 논리 화면 상수 `CW = 1280`, `CH = 720` (`lib.rs:134-135`).
- 창 생성: `Window::default_attributes().with_title("rbms").with_inner_size(LogicalSize::new(CW, CH))` (`lib.rs:997`). 크기 조절 가능 여부·최소 크기·풀스크린 지정 없음(`with_resizable`/`Fullscreen`/`set_fullscreen` grep 0건).
- 렌더러의 `size()` 는 항상 `(CW, CH)` 를 돌려준다(`gpu/mod.rs:524-527`). 셰이더 유니폼에도 `[CW, CH]` 를 한 번 써 넣는다(`gpu/mod.rs:198`). 서피스는 실제 창 픽셀 크기로 구성되고(`gpu/mod.rs:180-190`), 리사이즈 시 재구성(`gpu/mod.rs:515-521`, `lib.rs:1029-1033`).
- 따라서 모든 그리기는 1280x720 논리 좌표로 하고, GPU 가 창 전체로 늘린다. LETTERBOX 설정(`DisplayOptions.letterbox`, `schema.rs:630-631`)이 켜지면 16:9 최대 사각형에 맞추고 여백을 둔다(`surface_viewport`, `gpu/mod.rs:612-620`). 적용은 매 프레임 오버레이 단계(`lib.rs:949`).
- 커서는 같은 변환의 역으로 논리 좌표가 된다(`gpu/mod.rs:624-627`).
- 표시 모드: `PresentMode::AutoVsync` 고정(`gpu/mod.rs:185`), 이벤트 루프는 `ControlFlow::Poll`(`lib.rs:1214`).
- 설정 스키마에 해상도·풀스크린·vsync·FPS 상한 항목이 없다(`DisplayOptions`, `schema.rs:614-635`).
- 스킨 문서는 저작 크기에서 캔버스 크기로 `SkinViewport` 변환을 거친다(`app_input.rs:308` 에서 `(CW, CH)` 로 생성). 즉 1920x1080 문서는 지금도 1280x720 논리 좌표로 줄여 그린 뒤 창 크기로 다시 늘린다. 텍스처 쿼드는 GPU 가 최종 해상도에서 샘플링하므로 선명도 손실이 작지만, 텍스트 래스터가 어느 해상도에서 만들어지는지는 렌더 조사 범위(미확인). `docs/architecture.md:52` 는 텍스트도 `fill_rect` 로 내려간다고 적고 있어 1280x720 격자 기준일 가능성이 있다.

### 9.2 1280x720 전제가 박힌 위치

| 파일 | `CW`/`CH` 출현 | 용도 |
| --- | --- | --- |
| `apps/rbms-player/src/lib.rs` | 5 | 상수 정의, 기본 `Skin::default_for(MODE, CW, CH)`(776), 서버 연결 점 좌표(954), 창 크기(997) |
| `apps/rbms-player/src/gpu/mod.rs` | 21 | 유니폼, `size()`, 뷰포트·커서 변환, 시저(432) |
| `apps/rbms-player/src/app_input.rs` | 2 | `Skin::build(&cfg, mode, CW, CH)`(288), `SkinViewport`(308) |
| `apps/rbms-player/src/app_options.rs` | 7 | 내장 옵션 패널 위치(279) 외 테스트 |
| `apps/rbms-player/src/stage/play/mod.rs` | 6 | 분석 오버레이 좌표(666-685) |
| `apps/rbms-player/src/stage/loading.rs` | 2 | 내장 진행 막대 중심(487-488) |
| `apps/rbms-player/src/stage/{keyconfig, course_result, practice, tables, folders, result}.rs` | 각 1 | 내장 패널 가로 중앙 정렬 |
| `apps/rbms-player/src/app_play.rs`, `settings_ui.rs` | 각 1 | (용도 미확인, grep 집계만) |
| 테스트 파일 9개 | 3~11 | 헤드리스 캔버스 크기 |
| `apps/rbms-player/src/gpu/batch.rs` | — | 테스트 상수 `LOGICAL = (1280, 720)`(233) |
| `apps/rbms-player/src/settings_view.rs` | 0 | 자체 픽셀 상수로 배치(`ROW_PITCH 50.0` 등, 19-41). 암묵적으로 1280x720 전제 |

### 9.3 레퍼런스

- 해상도는 설정값 `config.getResolution()` 이고(`B:MainController.java:427` 등), 스킨은 저작 크기에서 그 해상도로 변환한다(`B:skin/Skin.java:386-391` 의 transform 설정, 세부는 미확인).
- 풀스크린/창 전환은 키 명령으로(`B:MainController.java:522-531`).

### 9.4 필요한 변경 요지

- 논리 해상도를 상수에서 "선택된 스킨의 저작 크기 또는 설정 해상도" 로 바꾸거나, 최소한 16:9 1920x1080 으로 올린다. `Renderer::size()` 가 `(CW, CH)` 를 돌려주는 구조라 `Gpu` 에 가변 논리 크기를 넣고 유니폼을 갱신하는 식이 최소 변경이다.
- 설정에 해상도·창 모드(창/테두리 없는 전체/독점 전체)·vsync 항목 추가(`schema.rs` `DisplayOptions` + `rbms_config::SETTINGS` 표).
- 내장 화면을 남긴다면 그 좌표 상수들이 전부 1280x720 기준이므로 스케일 계수를 두거나 내장 화면 전용 뷰포트를 둔다.

---

## 10. 시스템 사운드·BGA·폰트

### 10.1 시스템 사운드

- 22 stem, 레퍼런스 `SoundType` 순서와 파일명을 따른다(`R:apps/rbms-player/src/syssound.rs:57-138`): `scratch, f-open, f-close, o-change, o-open, o-close, playready, playstop, clear, fail, resultclose, course_clear, course_fail, course_close, guide-pg, guide-gr, guide-gd, guide-bd, guide-pr, guide-ms, select, decide`.
- 폴더 해석 순서: (1) `config.audio.sound_folder`(공백 제거 후 비어 있지 않으면) (2) 프리셋이 `STEEL NEON` 일 때 활성 번들의 `sound/` (3) 없음 = 무음(`lib.rs:218-223`, `assets.rs:321-327`, `R:crates/rbms-config/src/audio.rs:63, 121`).
- 파일 해석: `resolve_file(dir, stem, ["wav","ogg","flac","mp3"])` — stem 그대로의 파일 또는 stem + 확장자(`syssound.rs:33, 212-214`, `assets.rs:472-487`). `f-close2.ogg`, `scratch2.ogg` 같은 변형 파일은 읽지 않는다.
- 로드: 시작 시 동기 디코드(`lib.rs:724-728`, `syssound.rs:244-263`). 스트림 재개방 시 뱅크에 다시 설치(`app_play.rs:607-612`).
- 재생: `play_on(Bus::System, id, gain, pan 0, pitch 1, at 0)` 1회(`syssound.rs:321-326`). **루프·정지 API 가 없다.** 샘플 id 네임스페이스 `0x0090_0000` 길이 `0x100`(`syssound.rs:26`).
- 가이드음 6종은 `guide_se` 설정이 켜져 있을 때만(`syssound.rs:307-318`).
- 호출 지점: select 결정(`stage/select/mod.rs:147, 173`), 폴더 열기/닫기(152, 556), 옵션 열기/변경/닫기(`app_options.rs:198, 239, 251`), play ready(`stage/play/mod.rs:695`), play stop(489), 판정 가이드(556), decide(`stage/loading.rs:463`), result clear/fail/close(`stage/result.rs:90, 96`), course clear/fail/close(`stage/course_result.rs:91, 96`).

레퍼런스와의 차이:

- `SELECT`/`DECIDE` 는 BGM 계열이다(R-BMS 주석도 인정, `syssound.rs:173-179`). select 는 프리뷰 기본음으로 계속 재생(`B:select/MusicSelector.java:173-174, 187-189`), decide 는 장면 진입 시(`B:decide/MusicDecide.java:32-35`). R-BMS 는 둘 다 전환 순간의 1회 효과음으로 쓴다.
- result 는 설정에 따라 루프(`B:result/MusicResult.java:150-151`), fadeout 시작에 clear/fail 을 멈추고 `RESULT_CLOSE` 재생(`B:result/MusicResult.java:252-256`), 상태를 떠날 때 전부 정지(154-158). `MainState.play(sound, loop)` / `stop(sound)` (`B:MainState.java:162-172`).
- select 진입 시 `getSoundManager().shuffle()` (`B:select/MusicSelector.java:157`). 변형 파일 중 무작위 선택으로 보이나 `SystemSoundManager` 본문은 미확인.
- 스크롤 시 `SCRATCH` (`B:select/MusicSelectInputProcessor.java` 스크롤 루프). R-BMS select 는 커서 이동 시 사운드 없음(`stage/select/mod.rs:721-730`).

ModernChic 쪽 실측:

- `M:Sound/` 13개 ogg: `clear, f-close, f-close2, f-open, fail, o-change, o-close, o-open, playready, playstop, scratch, scratch2, screenshot`. select/decide BGM, guide, resultclose, course 계열은 없다.
- 스킨 자체 음원: `M:Root/sounds/`(change, click, close, enter, favorite, fullcombo, open, section, section-win, section-lose, `vo/` 등), `M:Select/sounds/`(open, close), 그리고 `M:Root/customsound.lua`. 스킨이 Lua 에서 직접 소리를 내는 계약이 있다는 뜻이며(세부는 스킨 본체 조사 범위), 앱 쪽에는 "스킨 발 재생"을 받을 버스·id 네임스페이스·로더가 없다.

### 10.2 BGA 와 동영상

- 차트 BGA: `bga_jobs` 가 `#BMP` 이름을 `["png","bmp","jpg","jpeg"]` 로만 해석(`assets.rs:633-651`), `image` 크레이트로 디코드(`assets.rs:654-659`). 앱 의존성의 `image` 기능도 `png, bmp, jpeg` 뿐(`R:apps/rbms-player/Cargo.toml:35`).
- 로드 조건: `config.display.bga && self.skin.bga.is_some()` (`app_play.rs:410`) — **내장 RON 필드에 BGA 사각형이 있어야** 디코드를 시작한다. 스킨 단독 소유 뒤에는 "문서가 bga 객체를 가졌는가" 로 바꿔야 한다.
- 표시: 현재 프레임 1장을 `background_texture` 로 올려 문서의 배경/bga 객체에 넘김(`stage/play/mod.rs:809-822`, `stage/canvas.rs:152-163`). 세대 번호로 재업로드를 피한다(`assets.rs:563-582`).
- 동영상: 워크스페이스 어디에도 영상 디코더 의존성이 없다(`Cargo.toml` 전체 grep 에서 `symphonia` 의 `isomp4`(오디오 컨테이너) 한 건뿐, `R:crates/rbms-audio/Cargo.toml:11`). 저장소 문서도 "비디오 BGA" 를 후속 과제로 적는다(`docs/HANDOFF.md:85`).
- ModernChic mp4 8개: `M:Select/bg/movie/BGmovie01.mp4`(21MB), `BGmovie02.mp4`(25MB), `M:Decide/bg/movie/sample.mp4`(6MB), `sample2.mp4`(13MB), `M:Play/parts/common/BGA/movie/#default.mp4`(20MB), `cyber.mp4`(45MB), `NOSTALGIC.mp4`(48MB), `travel.mp4`(24MB). 이들이 필수 요소인지 옵션(정지 이미지 대안 존재)인지는 스킨 본체 조사 범위(미확인). "똑같이" 가 목표면 스킨 소스 동영상 재생이 필요하고, "조금 더 심플한 기본 스킨" 에서는 빼는 것이 배포 크기상 가장 큰 절감이다.

### 10.3 폰트

- UI 폰트: 실행 인자 `--font` 또는 설정 `display.font_path` 를 시작 시 읽어 `rbms_render::load_font(bytes)` → `set_ui_family`(`lib.rs:1161, 1192-1203`), 설정 화면에서 파일 선택(`rfd`)으로 교체·초기화(`app_input.rs:247-267`, `stage/settings.rs:165-171, 271-284`). 저장소 내장 폰트는 `assets/fonts/Inter-Regular.ttf`(877KB) 하나(내장 방식은 렌더 크레이트 쪽, 미확인).
- 스킨 폰트: 문서의 `fonts` 경로를 워커가 **바이트로만** 읽어(`SkinAssetKind::Font`, `assets.rs:618`) `SkinScreen::build` 에 넘긴다(`skin_screen.rs:74-79, 429`). 해석은 렌더 쪽(cosmic-text).
- 비트맵 폰트 `.fnt`: 앱·크레이트 소스에 `fnt` 처리 코드가 없다(grep 0건). ModernChic 은 `.fnt` 10개(12.8MB)와 ttf 11개(54.7MB)를 쓴다. `.fnt` 지원은 렌더·로더 쪽 신규 작업이고, 앱 쪽은 워커 디코드 작업 종류에 "fnt 본문 + 페이지 이미지" 를 추가해야 한다.
- 텍스트 캐시 통계가 디버그 패널에 노출됨(`lib.rs:968-969`).

---

## 11. 그 밖에 앱 배선에서 발견한 구조적 격차

1. **장면 시계가 화면 전환에 묶여 있지 않다.** `skin_now_ms` 는 앱 시계이고(`skin_screen.rs:567-569`) `start_play()` 가 그 시계를 0 으로 되돌린다(`app_play.rs:674`). select 에서 켜 둔 타이머(예: `SONGBAR_CHANGE`, `PANEL1_OFF`)는 play 진입 뒤 "미래 시각" 이 되고, play 에서 켠 타이머는 result/select 로 넘어가도 남는다. 레퍼런스는 전환마다 타이머 전부 OFF + 시계 0(`B:TimerManager.java:100-106`). ModernChic 의 모든 진입 애니메이션(타이머 없는 dst)이 이 전제에 의존한다.
2. **컴파일된 화면이 영구 상주한다**(§2.2-8). 레퍼런스식 "상태 진입 시 로드, 이탈 시 해제" 로 바꾸면 메모리는 줄지만 매 전환마다 로드 비용이 생기므로 비동기 로드 + 전환 지연 설계가 필요하다.
3. **문서 로드(`load_skin`)가 프레임 루프에서 동기 실행된다**(`skin_select.rs:695`). JSON 한 장은 견디지만 Lua 수천 줄 실행은 프레임 정지를 만든다.
4. **`Skin`(내장 RON 필드)이 플레이 로직에 박혀 있다.** BGA 로드 조건(`app_play.rs:410`), 레인→타이머 키 번호(`stage/play/mod.rs:265-286`), 판정 측(`field_side`, 243-248), 키 봄 지속 시간(`skin.bomb_us`, 280), 결과 팔레트(`ResultPalette::from_skin`, `app_input.rs:287`)가 모두 내장 `Skin`/`SkinConfig` 를 읽는다. 스킨 단독 소유 뒤에도 "레인 순서·1P/2P 측" 은 모드 데이터에서 직접 계산해야 한다.
5. **옵션 변경이 스킨 재빌드를 부른다.** 옵션 오버레이를 닫을 때 `rebuild_skin()`(`app_options.rs:255-258`), 리프트 조절 때 `rebuild_skin()`(`app_input.rs:187-195`). 내장 필드 전제의 잔재다.
6. **IR 제출에 스킨 이름이 실린다**: `skin: shared.config.display.skin.clone()` (`app_result.rs:198`). 프리셋 행을 없애면 이 값의 출처를 바꿔야 한다.
7. **스킨이 디스크에 쓴다.** ModernChic 은 `M:io/Play/sp/log/*.txt`, `M:io/Play/**/lanecover/*.txt`, `M:History/recent.txt` 등을 보유한다. R-BMS 샌드박스는 `io` 를 제거한 상태(작업 지시서의 확인 사실). 쓰기 허용 범위와 위치(읽기 전용 설치 위치일 때의 리다이렉트)를 정해야 한다.
8. **decide 문서가 스캔·테이블 받기 화면에도 그려진다**(`stage/loading.rs:475-485`). ModernChic decide 는 곡 정보 연출이므로 곡이 없는 대기 화면에 쓰면 빈 연출이 나온다.
9. **코스**: R-BMS 는 스테이지마다 Loading 을 거치고(`lib.rs:245-255`), 코스 결과는 내장 화면이다. 레퍼런스는 스테이지 사이에 RESULT → PLAY 로 직행하고(`B:result/MusicResult.java:200-208`) 마지막에 COURSERESULT 스킨을 쓴다.
10. **`MODE` 고정 인자**: 헤더 스캔과 비플레이 화면 로드에 7키를 넘긴다(`skin_select.rs:693, 749`). play 문서가 모드에 따라 헤더가 달라지는 경우의 영향은 로더 조사 범위(미확인).

---

## 12. 권고: 변경 단위(파일별, 크기, 의존 순서)

크기 기준: S = 한 파일 100줄 이내, M = 여러 파일 수백 줄, L = 구조 변경 또는 1,000줄 이상(테스트 포함).

| 순서 | 단위 | 주요 파일 | 크기 | 선행 | 내용 |
| --- | --- | --- | --- | --- | --- |
| A1 | 장면 시계·타이머 수명 | `lib.rs`(`App::switch`), `skin_screen.rs`(`skin_now_ms`), `app_play.rs:673-679` | S | 없음 | 전환마다 `skin_timers.clear()` + 장면 시작 시각 기록. `skin_now_ms` 를 "장면 진입 후 ms" 로. 곡 시계(`clock`)와 분리 |
| A2 | 논리 해상도 가변화 | `gpu/mod.rs`, `lib.rs:134-135, 997`, `stage/canvas.rs`, `schema.rs`(`DisplayOptions`), `rbms-config` 설정 표 | M | 없음 | `Gpu` 에 논리 크기 필드, 유니폼 갱신, 커서 변환. 해상도·창 모드·vsync 설정 행 추가 |
| A3 | 입력 이벤트 확장 | `lib.rs:1026-1067`, `stage/mod.rs`(`StageHandler`), `AppShared` | M | 없음 | 버튼 종류, release, drag, wheel, (선택) IME. 기본 구현은 no-op |
| B1 | `.luaskin` 탐색·헤더·타입 확장 | `skin_select.rs:39, 727-762`, `rbms-skin/loader.rs:221-242` | M | 로더의 Lua 지원(타 조사 범위) | 확장자, 타입 9·15, 헤더 캐시 |
| B2 | 문서 로드 비동기화와 수명 정책 | `skin_select.rs:682-705`, `skin_screen.rs:462-552` | M | B1 | 워커에서 `load_skin`, 상태 이탈 시 해제 여부 결정 |
| B3 | 스킨 팩 선택 모델 | `schema.rs:247-399`, `skin_select.rs`, `settings_ui.rs`, `stage/settings.rs` | M | B1 | 폴더 하나로 모든 타입 지정, 프리셋 행(`display.skin`) 제거, 설정 마이그레이션(스키마 버전 3) |
| B4 | 기본 스킨 배포·설치 교체 | `assets.rs:26-422` 전부, `lib.rs:1125-1134`, `.github/workflows/release.yml`, `app_play.rs:355-367`, `lib.rs:218-223` | L | B3, 배포 방식 결정 | `include_bytes!` 표 제거, 디렉터리 기반 탐색/복사, 폴백 정책, 이전 세대(steel-neon*) 선택의 마이그레이션 |
| C1 | 혼합 합성 제거(§8.2 표 전체) | `skin_screen.rs`, `stage/play/mod.rs`, `stage/select/mod.rs`, `stage/result.rs`, `app_options.rs`, `app_input.rs` + 렌더·스킨 크레이트 대응부 | L | A1, B2, 폴백 정책 결정 | `ScreenContent`/`Replacement`/layer 경로 삭제, 단일 그리기 경로 |
| C2 | 스킨 이벤트 실행기와 클릭 판정 | `skin_screen.rs`(신규 `skin_click`), 각 Stage 의 `execute_event` | M | A3, C1 | 객체 역순 판정, 이벤트 id → 화면 동작. `Hot` 사상 제거 |
| D1 | Decide Stage 분리 | 신규 `stage/decide.rs`, `stage/loading.rs`, `stage/select/mod.rs:142-149`, `app_play.rs:441-451` | M | A1, C1 | `input/scene/fadeout`, 키 스킵·취소, DECIDE 사운드. 로딩은 백그라운드로 병행 |
| D2 | Play 상태기계 | `stage/play/mod.rs`, `rbms-render` `PlayTimers` | L | A1, C1, D1 | PRELOAD(로딩 진행)/READY/PLAY/FAILED(close)/FINISHED(finishMargin, fadeout), `STARTINPUT/RHYTHM/MUSIC_END/ENDOFNOTE/SCORE_*` 타이머, 내장 `Skin` 의존 제거 |
| D3 | Result 장면 수명·상태 | `stage/result.rs`, `app_result.rs` | M | A1, C1 | 입력 잠금, fadeout, `UPDATESCORE` 2단계, 게이지 종류 전환, IR 상태 타이머, 랭킹 공급 |
| D4 | CourseResult 스킨화 | `stage/course_result.rs`, `course_ui.rs` | M | B1, D3 | 타입 15 문서, 코스 합산 상태 공급 |
| D5 | Select 상태 모델 확장 | `stage/select/{mod,scene,list}.rs`, `lib.rs`(`SelectItem`), `app_options.rs` | L | A1, C1, C2 | 바 종류, 램프 id, 패널 1~3 과 옵션 이벤트, 리플레이 슬롯, 배너/스테이지파일 분리, 랭킹·라이벌 공급, 검색 텍스트, 레인 키/START/SELECT 입력, select BGM |
| D6 | KeyConfig 스킨화 | `stage/keyconfig.rs`, `skin_screen.rs:895-906` | M | C1, C2 | 포커스·캡처 상태 공급, 스킨 이벤트로 조작 |
| D7 | SkinSelect Stage 신설 | 신규 `stage/skin_select.rs`, `skin_select.rs` | M | B3, C2, A3 | 타입 9 문서, 스킨 교체·커스터마이즈 버튼·슬라이더·휠, 미리보기 여부 결정 |
| E1 | 사운드 확장 | `syssound.rs`, `rbms-audio`(루프·정지), `lib.rs:218-223` | M | B3 | BGM 루프/정지, 변형 파일, 스킨 팩 사운드 폴더, 스킨 발 재생용 버스·네임스페이스 |
| E2 | 스킨 소스 동영상·`.fnt` 자산 파이프 | `assets.rs`(`SkinAssetKind`), `Cargo.toml` 의존성 | L | 렌더 지원 | 디코더 선택(의존성 추가는 사용자 확인 필요), 프레임 업로드 경로 |
| F1 | 테스트 재편(§8.6) | `stage/render_tests*.rs`, `assets.rs` tests, `select/tests.rs`, `play/tests.rs` | L | C1 이후 각 단위와 함께 | v3 번들 테스트 폐기, 새 기본 스킨 기준 스냅샷, 전이·타이머 단위 테스트 |
| F2 | 문서 갱신 | `docs/architecture.md:73-85`, `docs/development.md:83-86`, `docs/skin.md`, `docs/HANDOFF.md`, `docs/PROCESS.md` | M | 각 단위와 함께 | 혼합 합성·세대 설치 서술 교체 |

의존 요약: A1·A2·A3 은 서로 독립이며 먼저 병렬 가능. B1 → B2/B3 → B4. C1 은 A1·B2 뒤. D 계열은 C1 뒤에 화면별 병렬 가능(파일 소유가 겹치지 않음: decide/loading, play, result, course_result, select, keyconfig, skin_select). E 는 B3 뒤 병렬.

---

## 13. UI·레이아웃 관점에서 사용자에게 보일 변화

| 영역 | 지금 | 전환 뒤(권고대로일 때) |
| --- | --- | --- |
| 창 | 1280x720 창, 늘리면 그대로 확대 | 1920x1080 기준 화면, 해상도·전체 화면 설정 가능 |
| 화면 흐름 | 선택 → 로딩 막대 → 플레이 → 결과 | 선택 → 결정 연출(스킵 가능) → 플레이(로딩·READY 연출 포함) → 페이드 → 결과(입력 잠금 뒤 종료 페이드) |
| 선택 화면 | 왼쪽 목록 + 오른쪽 상세 + 아래 내비게이션 버튼(내장 레이아웃 위에 프레임 이미지) | 스킨의 곡 바 휠·배너·랭킹·옵션 패널이 화면 전체를 구성. 아래 버튼 줄(SEARCH/SORT/FOLDERS/TABLES/RECORDS/SETTINGS)은 스킨에 대응물이 없으면 사라짐 → 단축키 또는 시스템 메뉴로만 접근 |
| 옵션 변경 | Shift 홀드/F1 로 11행 패널 | 스킨의 3개 패널(플레이 옵션/어시스트/상세). 조작 키 체계가 레퍼런스식(START/SELECT + 레인 키)으로 바뀌거나 키보드 대응 키를 새로 정의 |
| 검색·필터·기록·랭킹 | 내장 상자·패널·모달 | 스킨 객체로 대체되는 것과 시스템 오버레이로 남는 것으로 나뉨(§8.5 결정 필요) |
| 플레이 화면 | 내장 필드(RON) + 문서 장식, 키 봄은 항상 내장 | 노트·판정·게이지·봄·커버 전부 스킨. PLAY SIDE/GRAPH POSITION 같은 R-BMS 번들 옵션은 사라지고 스킨이 선언한 옵션만 남음 |
| 결과 화면 | 즉시 키 입력으로 종료, IR 한 줄 | 스킨 연출 시간 뒤 입력 허용, 랭킹·이전 기록 비교·게이지 종류 전환 |
| 코스 결과·키 설정 | 내장 텍스트 목록 | 스킨 화면 |
| 스킨 설정 | SETTINGS 의 SKIN 탭(행 목록, 화면 타입별 개별 선택) | 스킨 팩 단위 선택 + 스킨의 skin select 화면(미리보기·옵션 목록·슬라이더). SKIN 탭은 팩 선택과 폴더 지정 정도로 축소 가능 |
| 설정·폴더·테이블·연습 | 내장 화면 | 스킨 계약 밖이므로 내장 화면 유지가 기본. 다만 1920x1080 기준 재배치와 기본 스킨에 맞춘 색·폰트 조정이 필요(시각적 이질감이 가장 크게 남는 부분) |
| 소리 | 전환 순간 효과음 | 선택 화면 BGM, 결과 BGM 루프, 스킨 자체 클릭음 |
| 첫 실행 | 1.8MB 번들이 `~/.config/rbms/skin/` 에 풀림 | 배포 방식에 따라: 동봉 폴더 직접 참조(복사 없음) 또는 수십~수백 MB 복사/다운로드 |

---

## 14. 위험, 미확인, 읽지 못한 파일

### 14.1 위험

1. 폴백 부재: 단독 소유 전환 뒤 스킨 로드 실패·미설치 시 조작 불능. 최소 폴백을 먼저 정해야 한다.
2. 테스트 붕괴 규모: 스킨 관련 앱 테스트 약 50개 + 내장 화면 스냅샷 + 렌더 골든이 한 번에 깨진다. 게이트(`fmt`/`clippy -D warnings`/`cargo test --workspace`)를 중간 커밋마다 유지하려면 단위 순서를 지켜야 한다.
3. 메모리: 1920x1080 텍스처 다수(png 212MB 원본)를 화면 타입별로 상주시키면 VRAM 압박. 수명 정책(B2) 필요.
4. 프레임 정지: 동기 `load_skin` + 대형 Lua.
5. 배포 크기와 `compute_build_hash` 의 전체 파일 읽기.
6. 재배포 권리 미확인(이미지·동영상·음원).
7. 저장소 명명 규칙(`docs/HANDOFF.md:50`)과 "ModernChic 을 기본 스킨으로" 지시의 충돌.
8. 스킨의 디스크 쓰기 요구와 샌드박스 정책의 충돌.
9. 키 체계 변경(START/SELECT + 레인 키)이 기존 사용자 조작과 충돌.
10. 영상 디코더 의존성 추가는 컨벤션상 사용자 확인 대상(의존성 추가 전 확인 규칙).

### 14.2 미확인 항목(본문에 표시한 것의 모음)

- 렌더 크레이트의 텍스트 래스터 해상도와 내장 폰트 로딩 방식.
- 로더가 `input/scene/fadeout/loadstart/loadend/playstart/close/finishMargin/rankTime` 을 보존하는지.
- Lua 샌드박스 객체의 스레드 이동 가능 여부.
- `B:SystemSoundManager` 의 변형 파일 선택·셔플 규칙, `B:result/CourseResult.java` 본문, `B:config/KeyConfiguration.java` 본문, `B:TimerManager` 외 타이머 세부.
- `B:result/AbstractResult.java:247-254` 의 `mov` 원천(휠인지 키인지).
- ModernChic 에서 mp4 가 필수인지 선택인지, 옵션 이름이 화면 간에 공유되는지, `Sound/` 폴더가 스킨에서 어떻게 참조되는지.
- `keyconfig.rs` 에 START/SELECT 에 해당하는 바인딩이 있는지.
- `app_play.rs`·`settings_ui.rs` 의 `CW`/`CH` 1회 출현 용도.

### 14.3 배정 범위 중 전부 읽지 못한 파일

| 파일 | 읽은 범위 | 읽지 못한 범위 |
| --- | --- | --- |
| `apps/rbms-player/src/app_play.rs` | 270-690 정독, 나머지는 함수 시그니처 grep | 1-269(차트 디코드·연습 슬라이스·시계 헬퍼), 690-1327(디버그 줄·테스트) |
| `apps/rbms-player/src/app_result.rs` | 40-340, 380-475 | 1-39, 340-379(`run_target` 등), 475-680(테스트) |
| `apps/rbms-player/src/settings_view.rs` | grep(함수·상수)만 | 본문 전체 |
| `apps/rbms-player/src/stage/select/list.rs` | 1-330 | 330-649(`arrange_songs`, 테스트) |
| `apps/rbms-player/src/stage/select/preview.rs` | 1-300 | 300-552(클립/오토플레이 프리뷰 로드, 정지) |
| `apps/rbms-player/src/stage/select/filter.rs` | 1-230 | 230-441(조정·그리기·테스트) |
| `apps/rbms-player/src/stage/select/tests.rs`, `stage/play/tests.rs` | 패턴 grep 만 | 본문 |
| `apps/rbms-player/src/stage/render_tests*.rs`, `skin_screen/tests.rs`, `skin_select/{tests,fixtures}.rs`, `stage/settings/skin_tests.rs`, `syssound/tests.rs` | 테스트 함수 이름만 | 본문 |
| `apps/rbms-player/src/gpu/mod.rs` | 108-207, 500-637 | 나머지(파이프라인 생성, `render`) — 배정 목록 밖이나 해상도 판단에 사용 |
| `B:MainController.java` | 230-570 | 1-229, 570-855 |
| `B:play/BMSPlayer.java` | 366-785 정독 + 전체 grep | 1-365, 786-1092 |
| `B:result/MusicResult.java` | 150-319 정독 + 전체 grep | 1-149, 320-498 |
| `B:config/SkinConfiguration.java` | 44-200, 485-520 + 전체 grep | 200-484, 520-669 |
| `B:select/MusicSelector.java` | 40-710 | 1-39(import) |
| `B:select/MusicSelectInputProcessor.java` | 62-330 (배정 목록 밖, 입력 계약 확인용) | 나머지 |

배정 목록에 있었으나 스킨과 무관해 본문을 열지 않은 것은 없다. `stage/tables.rs`, `stage/folders.rs` 는 배정 목록에 없어 열지 않았다.
