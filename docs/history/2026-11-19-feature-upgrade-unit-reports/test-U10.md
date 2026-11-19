# U10 테스트 웨이브 보고 (wf_3df278ff-bb7)

> 테스트 웨이브의 에이전트가 끝낼 때 낸 보고 원문이다. 메인이 병합하면서 바꾼 것은 계획서 9절에 있다.

- 상태: done

## 바꾼 파일

- apps/rbms-cli/scenarios/select.json
- apps/rbms-cli/src/skin_dump/scenario.rs
- apps/rbms-player/src/app_result/record_tests.rs
- apps/rbms-player/src/lib.rs
- apps/rbms-player/src/skin_host/extension.rs
- apps/rbms-player/src/skin_host/mod.rs
- apps/rbms-player/src/skin_host/select.rs
- apps/rbms-player/src/skin_host/select/shown.rs
- apps/rbms-player/src/skin_host/select/tests.rs
- apps/rbms-player/src/skin_host/tests.rs
- apps/rbms-player/src/stage/select/bars_tests.rs
- apps/rbms-player/src/stage/select/browse_facts/tests.rs
- apps/rbms-player/src/stage/select/list.rs
- apps/rbms-player/src/stage/select/recorded_tests.rs
- apps/rbms-player/src/stage/select/scene.rs
- assets/skins/rbms-default/lua/props.lua
- assets/skins/rbms-default/lua/select/info.lua
- crates/rbms-render/examples/render_select.rs
- crates/rbms-render/src/select.rs
- crates/rbms-render/src/skin_render/songlist.rs
- crates/rbms-render/src/skin_render/songlist/bars.rs
- crates/rbms-render/src/skin_render/songlist/tests.rs
- crates/rbms-render/tests/golden_select.rs
- crates/rbms-store/src/lib.rs
- crates/rbms-store/src/score.rs
- crates/rbms-store/src/scoredb.rs
- crates/rbms-store/src/scoredb/rows.rs
- crates/rbms-store/src/scoredb/schema.rs
- crates/rbms-store/src/scoredb/schema.sql
- crates/rbms-store/src/scoredb/tests.rs

## 요약

U10 램프 반 쪼개기를 구현했습니다. U4 수용 기준은 모두 충족이며 결함은 찾지 못했습니다. U4 보고가 남긴 skin-dump 시나리오 공백은 메웠습니다.

(1) 저장(rbms-store)
- `score` 행에 `best_assist_grade INTEGER NOT NULL DEFAULT 0` 열을 더했습니다(schema.sql, `ADDED_COLUMNS`, BEST_COLUMNS·write_best).
- 스키마 버전, `SCORE_RULE_VERSION`, 서버 전송·기록 동기화 본문은 바꾸지 않았습니다.
- 갱신 경로:
  - `merge_best`: 램프와 상관없이 판의 등급 최댓값을 유지합니다.
  - `refold_best_assist_grades`: `refold_assist_grades` 끝에서 부르므로 DB 열기와 코스 스테이지 CLASS 이동 모두에서 돕니다. `assist_grade_sql` 을 그대로 써서 규칙 세대 2 미만 행과 assist=-1 행은 등급이 없습니다. 바뀔 행만 쓰므로 멱등이고, LIGHT ASSIST 판이 없는 행의 잘못된 값은 0 으로 되돌립니다.
  - `ScoreDb::best` 는 같은 차트·모드의 모든 LN 방식 행에서 최댓값을 접고, `chart_best_many` 는 MAX 로 접습니다.
- 규칙(M20)은 순수 함수 `rbms_store::split_assist_grade(best: RecordLamp, best_assist_grade) -> u8` 하나입니다. 램프 id 4~6 이고, 등급이 1~4 이며, 3+등급 > id 일 때만 쪼갭니다.
- `ScoreBook::best_assist_grade_for_md5` 는 기록들에서 접어 냅니다.

(2) 렌더(rbms-render)
- `SongBar.split_assist_grade` 를 더했습니다. 쪼갤 때는 이미지를 통째로 그리고 칸 중앙선에서 클립합니다(push_clip). 위 절반은 램프, 아래 절반은 `lamp_graded` 그림입니다.
- 그래서 그림이 찌그러지지 않고, stretch·각도·애니메이션 프레임이 그대로입니다. 두 클립이 같은 선을 공유하므로 홀수 높이·배율에서도 빈틈과 겹침이 없습니다. 클립의 바깥 세 변은 칸 밖까지 넓혀서, 바깥 가장자리 반올림으로 줄이 잘리지 않게 했습니다.
- `lamp_graded` 가 없는 스킨, 라이벌 모드, 범위를 넘는 등급은 통짜로 그립니다.
- 내장 선곡 화면은 `SelectRow.split_lamp: Option<Color>` 를 받아 LED 두 개를 위아래 반으로 칠합니다.

(3) 플레이어
- `SelectBar.split_assist_grade` 를 더했습니다. 채우는 곳은 list.rs `chart_bar` 이고, scene.rs 가 `wheel_bar` 와 내장 row 의 등급 색에 넘깁니다.
- `BarRecord.split_assist_grade` 를 더하고, 확장 불린 21160~21163(`RBMS_OPTION_SELECTED_BEST_SPLIT_GRADE_FIRST/LAST`, 등급 1~4)을 라우트·props.lua 와 함께 더했습니다.
- info.lua MY SCORE 램프 줄은 쪼개진 차트에서 왼쪽 절반에 램프 이름, 오른쪽 절반에 등급 이름을 등급 색으로 보입니다. 쪼개지지 않으면 기존 한 줄 그대로입니다. 새 번역 문구는 없습니다(게임 용어).
- 폴더 분포·스크롤 분포·정렬·필터는 바꾸지 않았습니다.

(4) 캡처
- 새 시험 `the_browser_splits_the_lamp_of_a_chart_cleared_without_assists_and_with_scratch_auto_on_a_stronger_gauge` 를 더했습니다. 손 플레이 NORMAL 클리어 다음에 SCRATCH AUTO HARD 판을 칩니다.
- 이 시험이 만든 `/tmp/rbms-u10-capture/select-recorded-split-lamp.png` 를 Read 로 열어 확인했습니다. 막대 램프는 위 노랑(CLEAR)·아래 라벤더(ASSIST HARD)이고, MY SCORE 는 "CLEAR  ASSIST HARD", MY RECORDS 의 ASSIST HARD 판 줄은 등급색입니다.

계획과 달라진 점
- `ScoreRecord`(scores.ron)에 필드를 더하지 않았습니다. ScoreRecord 는 판 하나라서 등급이 이미 clear·gauge·assist·rule_version 에서 유도됩니다(`assist_grade()`). 판마다 "지금까지의 최고" 값을 두면 출처가 없어 어긋납니다. 대신 `ScoreBook::best_assist_grade_for_md5` 로 접습니다. 선곡 막대는 DB 경로에서도 ScoreBook(모든 판)을 읽으므로 대체 저장소와 같은 함수를 씁니다.
- 반쪽을 그리는 방식은 원본 영역을 반으로 자르는 대신 클립입니다. Stretch 에서는 결과가 같고, fit 계열 stretch·각도에서도 정확합니다.
- `app_result/record_tests.rs` 의 `played_on` 을 `pub(crate)` 로 열었습니다(시험 재사용용 한 단어).

## 기준별 판정

- [pass] U10 할 일 1: score 행에 최고 어시스트 등급 열(ADDED_COLUMNS), 판 저장 시 갱신, DB 열 때 scorelog 에서 멱등 소급(세대 낡은 행·assist=-1 취급 동일), scores.ron 대체 저장소에도 같은 값: best_assist_grade 열과 merge_best·refold_best_assist_grades 를 더했습니다. 시험 the_best_row_keeps_the_highest_assist_grade_a_clear_without_assists_hides 와 opening_the_database_folds_the_highest_assist_grade_from_the_plays(EASY+ASSIST HARD DB, 구 빌드 0 리셋, 잘못된 값, 가져온 판, 낡은 세대, 코스 스테이지 CLASS 이동, 두 번째 refold 의 total_changes 불변)가 통과합니다. scores.ron 은 필드를 더하지 않고 ScoreBook::best_assist_grade_for_md5 로 접습니다(계획과 다름, 이유는 summary). 시험 the_score_book_folds_the_highest_assist_grade_beside_the_lamp 가 통과합니다.
- [pass] U10 할 일 2: 쪼개기 규칙(M20) 순수 함수와 표 시험: rbms_store::split_assist_grade 를 더했습니다. a_lamp_is_split_only_by_an_assist_grade_of_a_stronger_gauge 가 램프 0~10 x 등급 0~5 전체 표, LIGHT ASSIST 최고 램프, EX HARD 를 시험하고 통과합니다.
- [pass] U10 할 일 3: SongBar 에 쪼갤 등급, lamp_graded 있으면 위 절반 램프·아래 절반 등급 그림(찌그러짐 없음, 깜빡이는 램프, 홀수 높이), lamp_graded 없는 스킨·라이벌 모드는 화소 동일: 다음 렌더 시험이 통과합니다. a_split_lamp_draws_the_upper_half_of_its_lamp_over_the_lower_half_of_the_graded_lamp 는 행마다 텍셀 행·열이 정확히 일치하는지 봅니다. a_split_lamp_of_an_odd_height_drawn_larger_meets_in_the_middle_and_keeps_its_frame 는 높이 21·1.5배·2프레임 애니메이션에서 빈 줄 없이 위/아래 절반이 각자 소스 절반만 읽고 프레임을 따라가는지 봅니다. a_split_lamp_is_drawn_whole_where_the_wheel_has_nothing_to_split_it_with 는 lamp_graded 없음·라이벌·범위 밖 등급에서 화소가 완전히 같은지 봅니다.
- [pass] U10 할 일 4: 정보창 MY SCORE 램프 이름 줄에 쪼개진 등급 이름(확장 불린 21160~21163): extension.rs·mod.rs 라우트·props.lua·info.lua 를 고쳤습니다. 시험 the_grade_the_best_lamp_is_split_with_switches_on_its_own_option_only, a_split_grade_option_names_the_grades_from_assist_easy_to_assist_exhard, 라우트·props 이름 표 시험이 통과합니다. 캡처에서 'CLEAR  ASSIST HARD' 가 보이는 것을 확인했습니다.
- [pass] U10 할 일 5: 내장 선곡 막대 LED 도 위아래 반씩: SelectRow.split_lamp 와 fill_lamp 를 더했습니다. 시험 select::tests::a_split_lamp_lights_the_upper_half_of_each_led_in_its_lamp_and_the_lower_half_in_the_grade(좌우 LED 의 위·아래 화소)와 bars_tests::a_clear_without_assists_over_a_stronger_assisted_clear_splits_the_lamp_of_the_bar(scene row 의 split_lamp 색)가 통과합니다.
- [pass] U10 할 일 6: 서버 전송·기록 동기화·IR 본문 불변: rbms-ir, record_sync, submission 코드를 건드리지 않았습니다(git status 에 없음). 새 열은 로컬 score 테이블에만 있습니다.
- [pass] U10 완료 기준: 조건 표·저장·소급·렌더 화소 시험, 스킨 덤프, 캡처: 위 시험이 모두 통과합니다. skin-dump --strict --frames 120 은 문서 9개 load ok, fn fail 0, warn 0 입니다(select.luaskin dest 2190, fns 465). select-recorded-split-lamp.png 캡처를 확대해 보니 막대 위 노랑·아래 라벤더입니다.
- [pass] U4: 등급 0~4 가 목록 막대에서 서로 다른 그림·색: 기존 시험 every_assist_grade_of_a_light_assist_lamp_draws_a_lamp_of_its_own 를 다시 돌려 통과했습니다(다섯 색 서로 다름).
- [pass] U4: 램프 id 가 3 이 아닐 때 등급 값 무시: a_graded_lamp_stands_in_only_for_a_light_assist_lamp_it_has_an_image_for(lamp 5+grade 2 → Lamp(5))가 통과합니다. records.lua 의 그리기 조건에도 lamp==3 확인이 있습니다.
- [pass] U4: lamp_graded 키가 없는 스킨이 전과 같음: a_wheel_without_graded_lamps_draws_every_grade_as_the_light_assist_lamp(화소 동일)가 통과합니다. 새 split 시험도 같은 성질을 쪼갠 막대에 대해 봅니다.
- [pass] U4: MY RECORDS 6줄 등급 숫자(21087~21092)가 줄마다 올바른 판의 값, 빈 줄은 absent: 새 시험 browse_facts::tests::every_record_row_reads_the_assist_grade_of_the_play_it_names 를 더했습니다. best 줄은 best.assist_grade, 이후 줄은 각 판의 등급(EXHARD·0·EASY·0·HARD)이고, 다른 램프의 best 는 0 입니다. 기존 a_record_row_reads_its_five_figures... 는 빈 줄 INTEGER_ABSENT 를 확인합니다.
- [pass] U4: 랭킹 10칸 등급 숫자(21093~21102)가 칸마다 올바른 점수의 값, 결과 화면·구 서버 응답: ir.rs 는 NUMBER_RANKING1_CLEAR 와 같은 slot 함수로 assist_grade 를 답합니다. 시험 a_graded_light_assist_score_reads_its_grade_beside_its_lamp...(0,0,1,2,3,4,ABSENT)와 결과 화면 INTEGER_ABSENT 시험(ir/tests.rs:574)이 통과합니다. 구 서버는 assist_grade 를 serde 기본값 0 으로 받아 LIGHT ASSIST 색이 되고, LIGHT ASSIST 가 아닌 행은 ir_ranking 이 0 으로 막습니다(ADR 의 알려진 동작).
- [pass] U4: skin-dump 시나리오가 등급 블록·쪼개진 램프 경로를 지나고 --strict 통과: select.json 을 고쳤습니다. 기록 줄 21068=3 에 등급 21089=3, 랭킹 image index 393=3 에 등급 21096=4, 21110=true(최고 램프 등급 0), 21162=true(ASSIST HARD 로 쪼개짐)입니다. 그래서 records.lua 의 graded draw 함수가 lamp==3 다음의 number() 비교까지 호출됩니다. skin-dump --strict 는 fn fail 0 으로 통과했습니다. 막대(SongBar)는 MapHost 시나리오에 없어 덤프로는 렌더 경로를 지나지 못하므로 렌더 시험과 캡처로 덮었습니다.
- [untested] U4: 랭킹 줄 등급 색을 실제 캡처로 확인: 랭킹 모드를 연 캡처는 찍지 않았습니다. 수치 경로는 시험으로만 확인했고, MY RECORDS 줄의 등급색만 캡처로 확인했습니다.

## 찾은 것


## 남은 위험

- ScoreDb::best 가 best_assist_grade 를 같은 차트·모드의 모든 LN 방식 행에서 접습니다. best_in_ln_mode 와 best 의 같은 필드가 뜻이 다르므로(행 값 대 차트 값) 문서에 적어 둘 필요가 있습니다.
- 구 빌드가 같은 DB 를 다시 쓰면 best_assist_grade 가 0 이 됩니다. 다음 실행에서 소급됩니다(assist_grade 와 같은 성질).
- GPU 백엔드의 scissor 로 반쪽을 자르는 것은 CPU 캔버스 시험과 헤드리스 캡처로만 확인했습니다. GPU 캡처(.gpu.png)와 실제 창에서는 보지 않았습니다. CPU clip_bounds 주석은 GPU 와 같은 반올림이라고 적고 있습니다.
- info.lua 의 쪼개진 줄은 왼쪽·오른쪽 반칸에 이름을 나눠 씁니다. 긴 번역 램프 이름은 SHRINK_CUT 으로 줄어듭니다. 세 언어 캡처는 하지 않았습니다(램프 이름은 번역되지 않는 게임 용어).
- 외부 스킨용 SKIN MENU 미리보기의 샘플 곡에는 쪼개진 막대가 없습니다(범위 밖).

## 갱신이 필요한 문서

- docs/acknowledge/2026-10-22-adr-assist-lamp-grade.md 4절: score.best_assist_grade 열(merge_best·refold_best_assist_grades, ScoreDb::best 의 LN 방식 접기), 8절: 쪼개진 램프(songlist split, 확장 불린 21160~21163)
- docs/acknowledge/2026-10-24-adr-rbms-extension-id-spaces.md: 불린 21160~21163 SELECTED_BEST_SPLIT_GRADE (빈 자리 21164~)
- docs/plan/2026-11-19-feature-upgrade-12.md 2절 U10: scores.ron 필드 대신 ScoreBook 접기, 클립 방식, played_on pub(crate)
- docs/reference/2026-11-19-feature-upgrade-12/README.md 4절: 램프 반 쪼개기와 최고 등급 열
- 스킨 문서(rbms 확장 키 설명): songlist 의 split_assist_grade 와 lamp_graded 로 반쪽 그리기(라이벌·lamp_graded 없음은 통짜)
- docs/HANDOFF.md 7절 확장 id 공간 요약
- docs/quality-assurance: 랭킹 모드 등급색 캡처와 GPU 백엔드 반쪽 램프 실기 확인을 테스트 부채로

## 돌린 검사

- `cargo test -p rbms-render -p rbms-store -p rbms-cli --locked` → EXIT=0. render lib 854, store 145, cli 포함 모든 test result ok, 실패 0
- `cargo test -p rbms-player --lib --locked select` → 603 passed, 0 failed, 1 ignored
- `cargo test -p rbms-player --lib --locked skin_host` → 403 passed, 0 failed
- `cargo test -p rbms-player --lib --locked default` → 113 passed, 0 failed
- `RBMS_SKIN_CAPTURE_DIR=/tmp/rbms-u10-capture cargo test -p rbms-player --lib --locked recorded_tests` → 새 split 시험 포함 4 passed. 처음 실행에서 새 시험이 실패했습니다(summary.clear_lamp 는 강등 전 Hard). 가정을 고친 뒤 통과했고, PNG 4장을 Read 로 확인했습니다.
- `cargo run -q -p rbms-cli --locked -- skin-dump assets/skins/rbms-default --strict --frames 120` → 문서 9개 load ok, warn 0, fn fail 0, select.luaskin dest 2190/fns 465, 팩 파일 88→88 불변
- `cargo clippy -p rbms-store -p rbms-render -p rbms-player -p rbms-cli --all-targets --locked -- -D warnings` → EXIT=0, 경고 0
- `cargo fmt --all --check` → cargo fmt --all 적용 뒤 통과(바뀐 곳은 내 파일의 줄바꿈뿐)
