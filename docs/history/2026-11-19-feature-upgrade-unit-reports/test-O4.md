# O4 테스트 웨이브 보고 (wf_3df278ff-bb7)

> 테스트 웨이브의 에이전트가 끝낼 때 낸 보고 원문이다. 메인이 병합하면서 바꾼 것은 계획서 9절에 있다.

- 상태: done

## 바꾼 파일

- apps/rbms-player/src/stage/play/run_setup.rs
- apps/rbms-player/src/stage/play/run_setup/tests.rs
- apps/rbms-player/src/stage/play/skin_preview.rs

## 요약

U6 반례 사냥 결과입니다. 결함 2건을 실패하는 시험으로 증명했습니다.

1. (범위 안, 고침) 메뉴 미리보기 PlaySample(SKIN MENU, PLAY OPTIONS)의 어시스트 줄이 SCRATCH AUTO, LN MODE, EX JUDGE 를 한 번도 쓰지 않았습니다. 원인은 이 셋을 샘플 세션에서 읽은 것입니다. 샘플 세션은 auto_lanes 가 비어 있고 판정 설정이 기본값이라 사용자 설정과 상관없이 늘 빈 값이 나옵니다. 반면 CONSTANT, LEGACY NOTE, 배치, 옵션 패널 인덱스(ASSIST_EXJUDGE, LNMODE)는 설정에서 읽으므로 같은 화면 안에서 서로 맞지 않았습니다. U6 보고의 "SAMPLE_LN_MODE_DECIDES=true 로 설정값을 보입니다"도 실제로는 성립하지 않았습니다.
   - 재현: 설정 SCRATCH AUTO on, LN MODE CN, 판정 폭 2배로 미리보기를 그리면 어시스트 줄이 기대값 "SCRATCH AUTO · CN · EX JUDGE" 대신 ""(빈 값)이었습니다.
   - 수리: run_setup.rs 에 `Assisted`(세션에서 읽는 of_session / 설정에서 읽는 of_settings)와 공통 `setup_of` 를 두고, `settings_setup(shared, mode, ln_mode_decides)` 를 더했습니다. 설정에서 읽을 때 SCRATCH AUTO 는 `scratch_auto_lanes` 로 턴테이블이 있는 모드에서만 켭니다(9키는 없음).
   - skin_preview.rs 에는 `PlaySample::run_setup` 메서드를 두고, draw 는 그 메서드를 부릅니다.
   - 실제 판(`run_setup`)의 동작은 바뀌지 않았습니다.
   - 새 시험: `the_sample_run_of_the_menus_names_the_arrangement_and_the_assists_the_settings_hold`(7키, 14키 양쪽 배치, 9키 SCRATCH AUTO 없음, 설정을 바꾸면 따라감). 고치기 전 실패를 확인했고 고친 뒤 통과했습니다.

2. (범위 밖, 고치지 않음) 빌린 배치로 하다 실패한 판을 START(새 배치)로 재시도하면 그 판의 기록·리플레이·제출이 빌린 배치가 아니라 사용자 자신의 배치로 남습니다.
   - 원인: `PlayState::retry`(stage/play/mod.rs)가 `app_result::retry` 를 먼저 부릅니다. 이 함수의 `hand_borrowed_layout_on → let_go_of_borrowed_layout → put_back_own_play_options` 가 설정의 random 을 사용자 값으로 되돌립니다. 그 뒤에 `record_retried_run` 이 `shared.config.play.random` 을 읽습니다.
   - 재현 시험(임시, 지금은 지움): 사용자 MIRROR, 빌린 R-RANDOM, play_through 후 `state.retry(&mut shared, false)` 를 부르면 DB 행의 random 이 1(MIRROR)이었습니다. 기대값은 4(R-RANDOM)입니다.
   - 리플레이 파일도 빌린 시드에 사용자 random 이 붙어 저장되므로 그 리플레이는 원래 배치로 재생되지 않습니다.
   - SELECT(같은 배치) 재시도는 시드가 맞아 빌린 배치가 유지되므로 정상입니다.
   - 고칠 방향: retry 를 시작하기 전에 기록하거나, 기록 경로가 판이 실제로 놓인 배치를 판 쪽에 들고 있게 합니다.

결함이 아니었던 가설(무엇을 어떻게 확인했는지)
- 배치는 판의 실제 값입니다. 코드로 확인: load 의 `layout_to_load` 가 빌린 배치·리플레이 배치를 설정에 넣고, 내려놓는 시점은 다음 load, end_replay, 결과 화면뿐입니다. 플레이 중 random·constant·legacy·scratch_auto 를 쓰는 곳은 없습니다(grep). 기록 경로도 같은 설정값을 씁니다. 판이 시작된 뒤 설정이 바뀌어도 표시가 고정된다는 것은 기존 시험이 덮습니다.
- 첫 그림 전에 폴더 스택이나 설정이 바뀌는 경로: PlayState 는 load 뒤에 만들어지고, 로딩 중에는 선곡 화면이 동작하지 않습니다. 연습 반복, 재시도, 다음 곡, 코스 스테이지는 모두 새 PlayState 를 만들어 setup 이 다시 None 에서 시작합니다(app_play.rs:165, 867).
- 출발 폴더의 각 경우: 코스와 리플레이 보기는 빈 값이고, LAYOUT PLAY 는 `borrow_layout` 이 `end_replay` 를 불러 replay 가 None 이 되므로 폴더가 보입니다. 검색은 선곡 화면 경로와 같은 `open_folders` 를 씁니다. 랭킹 모드와 기록 모달 배치 플레이는 선곡 경로를 그대로 따릅니다. `end_course` 가 course_run 을 지웁니다.
- 게이지: 이름과 색이 모두 같은 `shown.gauge.kind` 에서 오므로 GAUGE AUTO SHIFT 와 코스 게이지를 함께 따라갑니다. Lua `gauge_lamps` 는 `lamp_for_gauge` 와 같습니다(HAZARD 는 FULL COMBO).
- DP: lane_option 은 load 에서 적용되지 않으므로(lay_out_notes) 두 사이드를 같은 이름으로 쓰는 것이 맞습니다. 9키 SCRATCH AUTO 는 턴테이블이 없어 빠집니다(캡처로 확인).
- NPS
  - 이분 탐색(partition_point 2회)과 창 안의 노트만 훑으므로 프레임 비용은 노트 수에 비례하지 않습니다.
  - 경계: 정확히 1,000 ms 전은 제외, 판정선 위는 포함, 동시치기는 모두 셈, 첫 노트 전과 마지막 노트 뒤는 0. 기존 시험이 덮습니다.
  - 시간 기준: 플레이 타이머 경과가 차트 시간입니다. 연습은 구간 시작에서 출발하고(PracticeClock::chart_time_us), 리플레이 분석은 장면 시계를 따라가므로 뒤로 탐색해도 순수 함수라 맞습니다. 이 부분은 코드 읽기로만 확인했습니다.
  - 숨김 노트는 `hidden` 에 따로 있어 세지 않고, 지뢰와 롱노트 끝도 제외합니다. SCRATCH AUTO 레인은 셉니다.
- 선곡·결과 화면에서 21010~21014 와 21085 를 물으면: Play 클러스터에 frame 이 없어 None 이 되고 TEXT_ABSENT(빈 문자열)·INTEGER_ABSENT 가 옵니다. `show_play` 를 부르는 곳은 skin_screen.rs:1394 하나뿐입니다.
- PlayLive 복제: 프레임마다 Arc 증가 4회만 있고 힙 할당은 없습니다(setup.clone 과 PlayShown::of 의 clone).
- Lua: text value 함수는 그려지는 객체에서만 불립니다(rbms-render ValueSource 문서). number_box 는 크기만 재고 아무것도 놓지 않으므로, NPS Display Off 이면 NPS 만 빠집니다.
- EX JUDGE 정의: TOTAL 덮어쓰기는 이름에 넣지 않습니다. 선곡 옵션 패널의 `custom_judge` 와 같은 `is_custom_judge` 를 쓰므로 일관됩니다.

캡처: capture.rs 와 play_rows.rs 에 임시 프로브를 넣어 가장 긴 조합을 그렸고, 지금은 git checkout 으로 되돌렸습니다. 조합은 EX HARD CLASS + R-RANDOM + SCRATCH AUTO·HCN·EX JUDGE·CONSTANT·LEGACY NOTE + 긴 CJK 출발 폴더 + AUTOPLAY, 그리고 RATE 150% 입니다. 모드는 5·7·9·10·14키, 넓은 레인, 필드·턴테이블 오른쪽, 스코어 그래프 끔, 코스 FINAL STAGE 에서 PNG 30장을 Read 로 열어 보았습니다. 겹침과 넘침은 없었습니다. 다만 5키·7키 기본 너비에서 AUTOPLAY 와 함께일 때는 옵션 줄 나머지가 약 110~220px 로 SHRINK 되어 읽을 수 없습니다(아래 findings).

## 기준별 판정

- [pass] 값이 판의 실제 값인지(빌린 배치, 리플레이, 오토플레이, 연습, 코스, GAUGE AUTO SHIFT, 재시도, 다음 곡, DP, 9키): 기존 run_setup 시험 10개 통과. 설정을 쓰는 곳을 grep 해 보니 플레이 중 random 을 바꾸는 경로는 없습니다. 9키는 캡처로 SCRATCH AUTO 가 빠지는 것을 확인했습니다. 연습과 리플레이는 코드 읽기로만 확인했습니다.
- [pass] 미리보기(PlaySample)에서도 값이 온다(계획 U6 5항): 고치기 전에는 어시스트 줄이 늘 빈 값이었습니다(실패 시험으로 확인). settings_setup 로 고친 뒤 the_sample_run_of_the_menus_names_the_arrangement_and_the_assists_the_settings_hold 가 통과합니다.
- [pass] 출발 폴더가 경우마다 맞고 이전 값이 남지 않음: 기존 시험(루트, 1개, 3개, 코스, 리플레이)이 통과합니다. 판마다 새 PlayState 의 setup 이 None 에서 시작하는 것을 확인했고, borrow_layout 이 end_replay 를 부르는 것도 확인했습니다.
- [pass] NPS 정의·경계·시간 기준·프레임 비용: partition_point 2회와 창 안 훑기만 합니다. shown/tests 의 경계 시험이 통과합니다. 연습의 차트 시간 기준은 코드 읽기로만 확인했습니다.
- [pass] 가장 긴 조합에서 겹침·넘침 없음(5·7·9·10·14키, 넓은 레인, 좌우 뒤집기, 그래프 끔, 코스, 배속 표지): 임시 프로브로 PNG 30장을 떠서 Read 로 확인했습니다. 겹침과 넘침은 없었습니다. 5키·7키 기본 너비에서는 SHRINK 때문에 읽기 어렵습니다(findings 에 적음).
- [pass] 선곡·결과 화면에서 새 id 가 빈 값: ROUTES 가 Play 클러스터로만 보냅니다. show_play 를 부르는 곳은 skin_screen.rs:1394 하나이고, 그 밖의 화면에서는 TEXT_ABSENT 가 옵니다. 기존 시험 'a cluster with no frame answers no text' 가 통과합니다.
- [pass] PlayLive 가 Clone 이 된 뒤 프레임마다 불필요한 할당 없음: PlayLive 를 만드는 곳은 두 곳뿐입니다. 프레임마다 Arc 클론 2회 × 2(live, PlayShown::of)만 있고 힙 할당은 없습니다.

## 찾은 것

- [low, 고침] 메뉴 미리보기의 어시스트 줄이 SCRATCH AUTO·LN MODE·EX JUDGE 를 쓰지 않음: skin_preview.rs 가 run_setup(shared, &self.session, ...) 을 불렀습니다. 샘플 세션(sample_session)은 auto_lanes 가 비어 있고 판정 설정이 기본값이라 세 어시스트가 늘 꺼진 것으로 읽혔습니다. 설정 SCRATCH AUTO on, LN MODE CN, 판정 폭 2배에서 기대값은 "SCRATCH AUTO · CN · EX JUDGE" 인데 실제는 "" 였습니다(고치기 전 실패 시험 출력). 같은 화면의 옵션 인덱스는 설정에서 읽으므로 서로 맞지 않았습니다. (시험: stage::play::run_setup::tests::the_sample_run_of_the_menus_names_the_arrangement_and_the_assists_the_settings_hold)
- [medium, 안 고침] 빌린 배치로 실패한 판을 START 로 재시도하면 사용자 자신의 배치로 기록됨(범위 밖): stage/play/mod.rs PlayState::retry 는 crate::app_result::retry(shared) 를 먼저 부릅니다. 이 함수의 hand_borrowed_layout_on 에서 시드가 다르면 let_go_of_borrowed_layout → put_back_own_play_options 가 실행되어 config.play.random 이 사용자 값으로 돌아갑니다. 그 뒤에 record_retried_run 이 shared.config.play.random 으로 DB 행, 리플레이(random 라벨), 제출을 씁니다. 임시 시험 결과: 사용자 MIRROR, 빌린 R-RANDOM, play_through 후 state.retry(&mut shared, false) 를 부르면 db.all_plays()[0].random == 1(MIRROR)로, 기대값 4(R-RANDOM)와 달랐습니다. 리플레이가 빌린 시드와 사용자 random 의 조합으로 저장되어 원래 배치로 재생되지 않습니다. U6 이전부터 있던 결함이고 U6 범위 밖이라 고치지 않았습니다. 재현 시험은 실패하는 시험을 남기지 않으려고 지웠습니다. (시험: 임시 재현(지움): a_library_of_one_chart → config.play.random=Mirror → borrow_layout(R-RANDOM) → next_run → play_through → state.retry(&mut shared, false) → db.all_plays()[0].random 을 RRandom 과 비교)
- [low, 안 고침] 가장 긴 옵션 조합에서 5키·7키 기본 너비의 옵션 줄이 SHRINK 로 읽을 수 없게 줄어듦: 캡처 rows-play5-play, rows-play7-play 에서 EX HARD CLASS + R-RANDOM + 다섯 어시스트 + AUTOPLAY 일 때 나머지 글자가 약 110px(5키)·220px(7키)로 줄었습니다. 겹침과 넘침은 없습니다. 오토플레이가 아니면 status_room 120px 이 풀려 조금 나아지고, 일반 조합(게이지·배치·어시스트 1~2개)은 들어갑니다. 두 줄로 나누거나 약칭을 쓰려면 레이아웃 결정이 필요해 고치지 않았습니다. (시험: 임시 캡처 프로브(되돌림): RBMS_U6_PROBE 로 세션·설정을 가장 긴 조합으로 두고 play_rows 의 장면을 캡처)

## 남은 위험

- 재시도 기록 결함(범위 밖)이 그대로 남아 있습니다. 빌린 배치 판을 START 로 재시도하면 기록·리플레이·제출의 random 이 틀립니다.
- 연습 FREQUENCY 와 PLAYBACK RATE 에서 NPS 가 차트 시간 기준이라는 것은 코드 읽기로만 확인했습니다. 그 배속에서 NPS 값을 직접 단언하는 시험은 없습니다.
- settings_setup 은 미리보기에서 SCRATCH AUTO 를 설정값으로 이름만 쓰고, 샘플 판 자체는 여전히 SCRATCH AUTO 없이 그립니다(노트 색칠 없음). 이것은 기존 미리보기 설계를 따른 것입니다.
- 가장 긴 조합에서 5키·7키 옵션 줄의 가독성은 레이아웃 결정이 필요합니다.
- 워크스페이스 전체 시험과 캡처 게이트 전체(the_play_scene_of_the_default_pack_is_captured_with_its_rows_switched)는 돌리지 않았습니다. 메인이 돌릴 몫입니다.

## 갱신이 필요한 문서

- docs/reference/2026-11-19-feature-upgrade-12/README.md U6 절: 미리보기(PlaySample)의 어시스트·배치는 설정에서 읽는다(run_setup::settings_setup)
- docs/history/2026-11-19-feature-upgrade-unit-reports/U6.md 또는 계획서 8절: 'SAMPLE_LN_MODE_DECIDES 로 설정값을 보인다'는 보고가 수리 전에는 성립하지 않았다는 정정
- docs/bug: 빌린 배치 판의 START 재시도가 사용자 배치로 기록되는 결함(원인 PlayState::retry 의 호출 순서)
- docs/quality-assurance: 연습 FREQUENCY 에서 NPS 차트 시간 기준 시험 부채, 가장 긴 옵션 조합 5키·7키 가독성

## 돌린 검사

- `cargo test -p rbms-player --lib --locked the_sample_run_of_the_menus (수리 전)` → FAILED: left (["MIRROR", ""], "") right (["MIRROR", ""], "SCRATCH AUTO · CN · EX JUDGE") — 결함 재현
- `cargo test -p rbms-player --lib --locked run_setup (수리 후)` → ok. 11 passed; 0 failed
- `cargo test -p rbms-player --lib --locked probe_a_failed_run (임시 재현 시험, 이후 삭제)` → FAILED: left 1 right 4 — 재시도 기록 결함 재현
- `cargo test -p rbms-player --lib --locked stage::play` → ok. 302 passed; 0 failed
- `cargo test -p rbms-player --lib --locked skin_preview` → ok. 57 passed; 0 failed
- `cargo test -p rbms-player --lib --locked play_menu` → ok. 81 passed; 0 failed
- `cargo test -p rbms-player --lib --locked skin_host::play` → ok. 104 passed; 0 failed
- `RBMS_U6_PROBE=1 RBMS_SKIN_CAPTURE_DIR=/tmp/u6cap cargo test -p rbms-player --lib --locked probe_u6_longest_combination (임시 프로브, 되돌림)` → ok. PNG 15장. RBMS_U6_RATE=1 로 한 번 더 돌려 15장. Read 로 확인한 결과 겹침과 넘침은 없었습니다.
- `cargo fmt --all --check` → 통과
- `cargo clippy -p rbms-player --all-targets --locked -- -D warnings` → 경고 0
- `skin-dump --strict` → 실행하지 않음. 스킨 Lua 파일은 바꾸지 않았습니다.
