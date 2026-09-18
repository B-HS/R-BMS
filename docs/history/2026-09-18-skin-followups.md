# 스킨 후속 기능 전면 구현 (2026-09-18)

사양: `docs/plan/2026-09-18-skin-followups.md`. 결정: `docs/acknowledge/2026-09-18-skin-followups-decisions.md`. 브랜치 `feat/skin-followups`. 이 문서는 L1~L4 의 실제 진행과 실측 결과를 기록한다(최종 갱신은 문서 끝의 날짜).

## L1 연구 (Workflow `wf_86282201-14e`, Opus 4갈래, 5분 11초)

- R-A 레퍼런스 의미: customTimers/customEvents/act/click/mouseRect 의 JSON 형태·클릭 사각형(현재 애니메이션 영역, 위부터 첫 히트)·내장 이벤트 id 표, SkinType 0~18(24키 = 16, PRACTICE 타입 없음, 헤더 `preview` 없음), skinpreview(스킨 선택 화면 전용 FBO 라이브 렌더), practice(행 수만 정함, 0 이면 레거시 목록), pmchara(`.chp` MS932 정의, 타입 0~15, 모션 타이머 900~908, 좌상단 기준 축별 배율).
- R-B CSV 로더: 헤더 7명령·본문 명령 표(SRC/DST 인자 위치), `#RESOLUTION` 기반 좌표 변환, `!` 부정, 타이머 ≤0 무시, `#IF` 비중첩 2플래그, `#INCLUDE` 인라인, MS932, 와일드카드 무작위 선택.
- R-C 비디오: 순수 Rust 조합(`mpeg-ps`+`mpeg-pes`+`oxideav-mpeg12video`, `re_mp4`+`rusty_h264-decoder`), `openh264` 는 C++ 빌드, `.wmv` 순수 Rust 디코더 없음. 레퍼런스 동기 모델(전진 디코드·오프셋·프레임 드롭·EOF 공백·플레이 BGA 무루프).
- R-D 코드 사실: 믹서 보이스 구조와 루프 삽입 지점, BGA 경로의 최소 접합점(프레임마다 새 generation), 연습 Stage·SKIN 탭 구조, 24키 차단 요소 5건, 옵션 오버레이 입력 경로.

## L2 구현 1차 (Workflow `wf_d454c9f2-047`, Opus 5갈래, 격리 워크트리, 44분)

| 단위 | 커밋 | 내용 | 실측 |
| --- | --- | --- | --- |
| A 번들 결함 | `50a8ef3` | `frame-dp-10k.png`(생성기 `DUAL_FIELD_COLUMNS_*`), NOTES 묶음 8px 여백 우측 정렬(4조합), 선택 판정 6행(`RecordRowView.counts/max_combo`, 레퍼런스 id 110~114/420/75), BGA 는 `bga` 선언만으로 소유(투명 목적지 삭제, 10K/14K OFF 회귀도 해결) | render skin_render 71·player skin 82·assets 14·select 115 통과, 캡처 픽셀 측정(프레임 x 334/335…958/959, NOTES 989<1000) |
| B BGM 루프 | `ff48ddc` | `Command::Play { looping }`·`wrap_loop_pos`·보간 파트너 랩, `play_looping_on`(`ScheduleRequest`), `cue_loop`/`play_loop`/`stop`, 곡 선택 on_enter 루프·on_exit 정지, 확인 1회음 제거 | rbms-audio 194 통과(루프 6건), player lib 914 통과(개별 target) |
| C 문서 이벤트 | `d64b867` | `TimerRequest`/`TimerState::apply`, Lua `eval_timer`·`run_action`(`skin.set_timer/clear_timer`), `skin_render/events.rs`(`DocumentEvents`, 조건·minInterval·클릭 사각형·내장 표), `skin_screen/events.rs` 디스패치(Hot/설정 조정), 브라우저 매 프레임 갱신 | rbms-skin 통과(신규 4), render 83, player skin 82·select 115 |
| D 미리보기·연습·마우스 | `5834221` | `SkinScreens::render_preview`(CPU 캔버스 오프스크린, 캐시), SKIN 탭 축소판, `SkinObjectKind::SkinPreview`, 연습 id 20401~/20421~/20441~/20461, `FrameExtra::Practice`, `SkinObjectKind::Practice`, 연습 Stage 문서 그리기, `Hot::OptionRow` + `options_mouse` | render 78, player lib 921, 캡처 practice-7k·settings-skin |
| E 24키 | `39a586e`+`75fa520` | `Mode::ALL` 편입(`has_bms_channels`), 26레인 기본 키, `lane_width_weights`, `play-24k.json5` 실제 문서 + `frame-24k.png`, `render_tests_skin_v3_play_24k.rs`, 모드 라벨/색 | model 66·judge 295·render skin 112·keyconfig 44·skin 85 통과, v3 캡처 39 |

- 충돌 3건(skin_screen.rs 필드/함수·import, generate-assets.py 프레임 함수)은 양쪽 추가분을 합쳐 해소. 자산 재생성 결정성 확인(재생성 후 변경 0).
- 공유 `CARGO_TARGET_DIR` 가 워크트리 간 stale rlib 을 섞는 문제가 4갈래에서 보고돼 L3 부터 갈래별 target 디렉터리를 쓴다.
- 남은 항목(L4 로): 플레이·결과·결정·키설정 문서의 커스텀 이벤트 매 프레임 갱신, 미리듣기 중 BGM 정지, 내장 24K 필드 `note_color` 건반 패턴, 24K 스크래치 타이머 분리, 800줄 초과 파일 분할(`skin_screen.rs` 1223·`state.rs` 1066·`mixer.rs` 2190·`stage/settings.rs` 847), 이 문서와 QA 갱신.

## L3 구현 2차 (Workflow `wf_9df10520-9ff`)

진행 중.

## L4 리뷰·게이트·머지

미착수.
