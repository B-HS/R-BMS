# S4 테스트 웨이브 보고 (wf_3df278ff-bb7)

> 테스트 웨이브의 에이전트가 끝낼 때 낸 보고 원문이다. 메인이 병합하면서 바꾼 것은 계획서 9절에 있다.

- 상태: done

## 바꾼 파일

- apps/rbms-player/src/text/menus/key_editor.rs
- apps/rbms-player/src/text/menus/layout_pick.rs
- apps/rbms-player/src/text/notices.rs
- crates/rbms-render/src/text/result.rs
- crates/rbms-render/tests/golden_result.rs

## 요약

이번 작업(`cac3795..origin/dev`)이 만든 사용자 표시 문구 전부를 세 언어와 동작 기준으로 대조했습니다. 결함 5건을 고치고, 시험 12개를 더했고, 골든 한 줄을 맞췄습니다. 앱·렌더 시험은 모두 통과했고(player lib 3,279, render 851 + 골든 8), 스킨 Lua는 건드리지 않아 스킨 덤프는 돌리지 않았습니다. 스킨 메뉴 도움말 폭은 이 Mac의 실제 글꼴로 따로 쟀고(한국어 최대 710px, 일본어 최대 745px, 칸 800px), 시험으로 고정하지 않았습니다.

고친 것:
1. 내장 결과 화면 안내 줄(`HintSelectRetryNext`)에 capture 키 안내를 세 언어로 더했습니다(`P  CAPTURE LAYOUT` / `P  배치 저장` / `P  配置を保存`). 영어 문구를 고정한 시험과 `GOLDEN_RESULT_PACED` 골든(0x6c9a_a722_9ff8_67c7)을 맞췄습니다.
2. OUTPUT LATENCY 알림 둘이 토스트의 88자 절단(`TOAST_TEXT_CHARS`)에 걸려 "판정 오프셋을 되돌리라"는 핵심 안내가 잘렸습니다. 영어는 약 150자였고 JUDGE OFFSET 행 이름이 105번째 글자 근처에서 시작했습니다. 장치 이름을 문장 끝으로 옮기고 문장을 줄여 세 언어 모두 두 행 이름(출력 지연, 판정 오프셋)이 절단 앞에 들어옵니다.
3. FULLSCREEN 폴백 알림의 영어 문구가 97자라 "BORDERLE…"에서 잘렸습니다. 86자로 줄였습니다.
4. LAYOUT PICK 안내 줄이 동작과 어긋났습니다. 둘째 줄이 CAPTURED 목록에 키가 가 있을 때도 "UP DOWN MEASURE"를 보였지만, 그때 위·아래는 목록 커서를 움직입니다. 이 문구를 첫째 줄(HintTyping)로 옮겨, 둘째 줄이 두 상태 모두에서 참이 되게 했습니다.
5. 새 문구의 문체를 고쳤습니다. 한국어 조사 띄어쓰기를 붙였고(`AUTOPLAY 가` → `AUTOPLAY가`, `OFF 로` → `OFF로`, `ENTER 로` → `ENTER로`), 일본어 `KEY 4 を2回で削除`를 `KEY 4 を2回押して削除`로 바꿨습니다.

더한 시험:
- 내장 결과 안내 줄의 폭(세 언어)
- 알림이 토스트 절단 앞에 두 행 이름을 담는지, FULLSCREEN 알림이 88자 이내인지
- LAYOUT PICK 안내 줄 세 개의 폭, 둘째 줄이 목록 키(UP DOWN)를 말하지 않는지
- 키 편집기 캡처 힌트 5개와 조합 알림 3개의 폭(가장 긴 이름 기준)
- 결과 스킨 바닥 안내의 R·N·P가 `Shortcut::default_key`와 같은지

문구 대 동작은 다음 코드를 직접 읽어 확인했습니다. 안내 줄의 키(Space, Page Up/Down, Home, End, Tab, Delete, Right/Left, SELECT, KEY 1~4)는 `layout_pick.rs`의 `handle_key`·`list_lane`과 같고, 기존 `layout_pick/tests.rs`가 동작을 시험합니다. 스킨 결과 안내의 R·N·P는 기본 단축키와 같고, 선곡 바닥의 H는 `GUIDE_KEY`입니다.

## 기준별 판정

- [pass] OUTPUT LATENCY 행·설명·알림 둘이 세 언어로 있고 뜻이 같으며 용어를 따른다: text/settings/labels.rs 행 이름 OUTPUT LATENCY/출력 지연/出力遅延, descriptions.rs 설명, notices.rs 알림 둘. 알림은 토스트 절단 문제로 다시 썼고 `the_output_latency_notice_names_both_rows_before_a_toast_cuts_it_short` 와 기존 `..._names_both_rows_as_the_settings_screen_does` 가 세 언어에서 통과합니다.
- [pass] 알림 길이 제한(TOAST_TEXT_CHARS=88)에 새 알림이 걸리는지: 고치기 전 영어 OUTPUT LATENCY 알림은 약 150자로 JUDGE OFFSET 행 이름이 절단 뒤에 있었고 FULLSCREEN 알림은 97자였습니다. 고친 뒤 시험 2개(`...before_a_toast_cuts_it_short`, `the_fullscreen_notice_is_not_cut_short_by_a_toast`)가 통과합니다.
- [pass] WINDOW MODE·RESOLUTION·LETTERBOX 설명과 FULLSCREEN 폴백 알림: descriptions.rs 세 설명의 영어·한국어·일본어가 같은 뜻이고 U2 구현(BORDERLESS 는 macOS 시스템 전체 화면, FULLSCREEN 은 가장 가까운 모드·최고 주사율, 못 얻으면 BORDERLESS)과 맞습니다. 설명 패널 맞춤 시험 `every_description_is_written_whole_on_the_panel` 와 `every_description_fits_the_panel_in_korean_and_japanese` 가 통과합니다.
- [pass] 키 편집기의 ESC 라벨·조합 힌트·충돌·안내 문구: text/menus/key_editor.rs. 조합 힌트 둘은 stage/keyconfig.rs 531~534행의 선택과 같고 충돌·안내 3개의 자리표시자 의미가 `overlap`/`give_keys` 코드와 맞습니다. 폭 시험 2개를 더했고 통과합니다.
- [pass] LAYOUT PICK 안내 줄이 말하는 키가 코드가 읽는 키와 같다: HintCaptured·HintMoving 의 Space·Page Up/Down·Home·End·Tab·Delete·Right/Left·SELECT·KEY 1~4 는 layout_pick.rs 의 handle_key/list_lane 과 일치합니다. 어긋난 둘째 줄의 UP DOWN 은 첫째 줄로 옮겨 고쳤고(`the_second_line_of_keys_names_no_key_the_list_of_captured_layouts_takes_for_itself`), 폭 시험도 통과합니다.
- [pass] capture 성공·중복·거절 알림이 세 언어로 있고 용어집을 따른다: CaptureText 의 12개 항목이 모두 세 언어이고 `catalog_problems` 가 통과합니다. 한국어 조사 띄어쓰기 3곳을 고쳤습니다.
- [pass] 스킨 메뉴 오프셋 도움말이 M19(스킨 문서 단위)와 맞고 폭이 맞는다: OffsetPlaceHelp·AutomaticPlaceHelp 세 언어 모두 '스킨 좌표의 픽셀/skin pixels'. 이 Mac 실제 글꼴 실측으로 한국어 최대 710px, 일본어 최대 745px 가 칸 800px 안입니다. 테스트 글꼴로는 CJK 를 잴 수 없어 시험으로 고정하지 않았습니다.
- [pass] 선곡 바닥 KEY GUIDE(키 안내)와 결과 스킨 바닥 안내(R·N·P): select_bar.lua `bar_hint_key_guide` 가 세 언어이고 용어가 GuideText::ThisGuide(이 안내/このガイド)와 맞고 칸 폭이 충분합니다. result.lua 안내의 R·N·P 는 `Shortcut::default_key` 와 같고 시험(`the_hint_of_the_default_result_skin_names_the_keys_the_result_shortcuts_ship_on_in_every_language`)이 고정합니다. 기존 default_pack 시험이 세 언어·자리표시자를 검사합니다.
- [pass] 내장 결과 화면 안내 줄에 capture 키(P) 안내를 세 언어로 더하고 골든·시험을 맞춘다: crates/rbms-render/src/text/result.rs 수정, 영어 고정 시험·키 포함 시험·폭 시험 갱신, golden_result.rs 의 GOLDEN_RESULT_PACED 를 재생성한 값으로 교체. `cargo test -p rbms-render --locked --test golden_result` 8 passed. 다른 두 골든은 run_again 이 false 라 그대로입니다.
- [pass] 키 가이드(H)가 ESC 와 capture 를 올바른 묶음·이름으로 보이고 조합은 `A + B`: stage/select/guide.rs 의 `escape_label` 은 'ESC / BACKSPACE', 조합은 `bind_label`·`label_with` 로 'A + B' 입니다(기존 시험 `a_chord_is_named_as_its_two_keys_and_the_key_given_to_esc_beside_escape` 통과). capture 는 R·N 과 같은 Result 묶음이라 선곡 가이드에는 나오지 않고 키 편집기의 RESULT 묶음에 'CAPTURE LAYOUT/배치 저장/配置を保存' 로 나옵니다.
- [pass] 폭 시험(설정 설명 패널, 도움말, 알림)이 세 언어에서 통과: `cargo test -p rbms-player --lib --locked` 3279 passed 0 failed 2 ignored(기준선 3272 + 새 시험 7). 설명 패널 시험 2개·스킨 메뉴 도움말 시험·새 폭 시험 모두 통과.
- [pass] 영어 문구를 바꿨으면 그 문구를 기대하는 시험을 모두 찾아 맞춘다: 바꾼 영어(결과 안내, OUTPUT LATENCY 알림 둘, FULLSCREEN 알림, LAYOUT PICK 안내 둘)를 기대하는 시험은 notices.rs·text/result.rs·golden_result.rs 뿐이었고 모두 맞췄습니다. 다른 소스·시험·docs 에서 그 문자열을 고정한 곳은 docs/play-options.md 550행 한 군데입니다.

## 찾은 것

- [medium, 고침] OUTPUT LATENCY 알림의 핵심 안내가 토스트 88자 절단에 잘림: toast.rs `TOAST_TEXT_CHARS = 88`. 영어 원문은 장치 이름 16자 기준 약 153자였고 'JUDGE OFFSET' 이 105번째 글자 부근에서 끝나 화면에는 '(OUTPUT LATENCY AU…' 까지만 보였습니다. (시험: text::notices::tests::the_output_latency_notice_names_both_rows_before_a_toast_cuts_it_short)
- [low, 고침] FULLSCREEN 폴백 알림 영어가 97자라 단어 중간(BORDERLE…)에서 잘림: 원문 97자 > 88. 86자로 줄임. (시험: text::notices::tests::the_fullscreen_notice_is_not_cut_short_by_a_toast)
- [medium, 고침] 내장(스킨 없는) 결과 화면 안내 줄에 capture 키(P) 안내 없음: crates/rbms-render/src/text/result.rs HintSelectRetryNext 가 R·N 만 말했고 stage/result.rs 는 P 를 내장 화면에서도 처리합니다. (시험: rbms-render text::result::tests::the_keys_of_a_hint_are_the_same_in_every_language, the_hint_that_offers_a_retry_fits_between_the_margins_of_the_screen_in_every_language, tests/golden_result.rs result_screen_with_a_target_matches_its_golden_signature)
- [medium, 고침] LAYOUT PICK 둘째 안내 줄이 CAPTURED 목록 조작 중에도 'UP DOWN MEASURE'를 말함: layout_pick.rs draw 가 HintMoving 을 항상 그리고 handle_key 는 Focus::Captured 에서 ArrowUp/Down 을 step_list 로 보냅니다. 첫째 줄(HintCaptured)은 'UP DOWN CHOOSE' 라 같은 화면에서 모순이었습니다. (시험: text::menus::layout_pick::tests::the_second_line_of_keys_names_no_key_the_list_of_captured_layouts_takes_for_itself)
- [low, 고침] 새 한국어 문구의 조사 띄어쓰기('AUTOPLAY 가', 'OFF 로', 'ENTER 로')와 일본어 'KEY 4 を2回で削除': 같은 코드베이스의 설명 문구는 조사를 붙입니다(`FULLSCREEN이`, `BORDERLESS로`). 일본어는 어색한 'を2回で' 였습니다. (시험: text::menus::layout_pick::tests::every_entry_is_written_in_every_language)
- [low, 안 고침] CAPTURED 목록이 비었을 때 capture 키를 해제해 둔 경우 안내가 'PRESS  ON A RESULT'(키 이름 빈칸)로 나옴: stage/layout_pick.rs `draw_list` 가 `shortcut_bind(Shortcut::CaptureLayout).map(label).unwrap_or_default()` 로 빈 문자열을 `CapturedNone` 의 {0} 에 넣습니다. 소유 밖 파일이라 고치지 않았고 재현 시험도 만들지 않았습니다(코드 읽기 근거, 의심). (시험: 미작성: 단축키를 해제한 채 CAPTURED 목록을 비워 그린 화면의 문구를 확인하는 시험이 필요합니다.)
- [low, 안 고침] 플레이 화면의 게이지 이름 'EX HARD'가 설정·용어집의 'EX-HARD'와 다름: skin_host/play.rs `GAUGE_NAMES` 와 lua/theme.lua `gauge_name` 은 'EX HARD'(스킨 옵션 패널 이름과 맞춘 것), rbms-config `GAUGE_LABELS`·format.rs·용어집 1절은 'EX-HARD'. 이전부터 있던 스킨 대 앱 불일치이고 이번에 플레이 화면 바닥 줄이 그 표기를 새로 쓰게 됐습니다. 소유 밖이라 고치지 않았습니다. (시험: skin_host/play/tests.rs `every_gauge_name_is_the_one_the_default_skin_s_option_panel_shows` 가 스킨 표기를 고정하고 있어 어느 쪽으로 맞출지 결정 뒤에 함께 고쳐야 합니다.)
- [low, 안 고침] 일본어 알림이 자리표시자 앞뒤에 공백을 둠('{0} は', '{1} ms 遅れて'): 용어집 3절 5항은 원어 용어와 일본어 사이에 공백을 두지 않는다고 하지만, 코드에는 'リプレイ {0} をダウンロード中', 'RANDOM シード {0} でプレー' 같은 선례가 여럿 있어 위반으로 보지 않고 그대로 뒀습니다. 일본어 감수 때 확정할 사항입니다. (시험: 해당 없음(표기 관례 확인 사항))

## 남은 위험

- 일본어·한국어 폭은 테스트 글꼴이 CJK 를 못 그려 '영문은 실제 폭, 한·일 글자는 전각 1em' 근사로 쟀습니다. 이 Mac 실제 글꼴 실측은 스킨 메뉴 도움말에만 했습니다(최대 한국어 710px/일본어 745px, 칸 800px). 키 편집기·LAYOUT PICK 안내 줄은 근사만 봤고 여유가 큽니다(LAYOUT PICK 최대 약 675px/1200px).
- OUTPUT LATENCY 알림 문구를 다시 썼습니다. 기존 '노트와 판정을 그만큼 늦춥니다' 문장은 '보정합니다'로 줄었습니다. 의미는 유지했지만 한국어·일본어는 여전히 모델 초안이라 감수가 필요합니다.
- LAYOUT PICK 안내 줄 문구 이동(UP DOWN 을 첫째 줄로)은 layout_pick.rs 를 건드리지 않고 문구만 옮긴 것입니다. 다른 에이전트가 draw 에서 안내 줄 선택을 바꾸면 이 가정이 깨집니다. 새 시험이 그 경우를 잡습니다.
- 새 폭 시험의 HINT_SCALE·margin 상수는 stage/layout_pick.rs, stage/keyconfig.rs, crates/rbms-render/src/result.rs 의 값을 시험 안에 베낀 것입니다. 원본 상수가 바뀌면 시험이 따라가지 못합니다(소유 밖이라 pub(crate) 로 노출하지 않았습니다).
- 실제 창에서 세 언어로 그린 화면(글꼴 모양·줄바꿈·잘림)은 보지 않았습니다. 실행하지 않았고, 시험은 문구와 근사 폭만 봅니다.
- U10(램프 반 쪼개기)의 새 문구(MY SCORE 램프 이름 줄 등)는 이번 범위에 아직 없어 검증하지 않았습니다.

## 갱신이 필요한 문서

- docs/reference/2026-10-24-i18n-glossary.md 에 더할 용어(영어 / 한국어 / 일본어 / 쓰이는 곳): OUTPUT LATENCY / 출력 지연 / 出力遅延 / AUDIO 탭 행 이름, 알림; FULLSCREEN / FULLSCREEN(원어) / FULLSCREEN(原語) / WINDOW MODE 값(1절 원어 묶음); Game Mode / Game Mode(원어) / Game Mode(原語) / macOS 기능 이름, WINDOW MODE 설명; video mode / 화면 모드 / 画面モード / WINDOW MODE·RESOLUTION 설명, FULLSCREEN 폴백 알림; refresh rate / 주사율 / リフレッシュレート / WINDOW MODE 설명; system's full screen / 시스템 전체 화면 / システムのフルスクリーン / BORDERLESS 설명; compensate / 보정(합니다) / 補正(します) / OUTPUT LATENCY 알림(자동 보정 6.5절과 같은 계열); JUDGE OFFSET(행 이름) / 판정 오프셋 / 判定オフセット / 설정 행·알림(스킨 문구 6.5절의 판정 타이밍/判定タイミング 과 따로 있다는 주석); COMBINATION / 조합 / 組み合わせ / 키 편집기 힌트·충돌 안내(레이더 축 CHORD 와 다른 말); CAPTURE LAYOUT / 배치 저장 / 配置を保存 / 단축키 이름, 결과 화면 안내; CAPTURED(저장한 배치) / 저장한 배치 / 保存した配置 / LAYOUT PICK 목록 제목·안내; STOPPED / 정지 / 停止中 / LAYOUT PICK 미리보기 상태; KEY GUIDE / 키 안내 / キーガイド / 선곡 스킨 바닥 안내(6.1절의 'ALL KEYS AS THEY ARE SET / 설정된 모든 키 / 設定中のすべてのキー' 항목은 select_bar.lua 에서 지워졌으므로 대체 표기); skin pixels / 스킨 좌표의 픽셀 / スキン座標のピクセル / 스킨 메뉴 오프셋 도움말(M19). 7절에 'EX HARD(스킨·플레이 화면) 대 EX-HARD(앱 설정·램프·용어집 1절)' 어긋남 한 줄 추가.
- docs/play-options.md 550행: '바닥 안내 글자(...R  RETRY    N  NEXT SONG)는 내장 화면의 골든을 바꾸지 않기 위해 고치지 않았다' 는 이제 틀립니다. 내장 결과 안내 줄에 `P  CAPTURE LAYOUT`(세 언어)이 있고 골든 GOLDEN_RESULT_PACED 를 바꿨다고 고치기. 같은 문서의 LAYOUT PICK 조작 절에 'UP DOWN 한 마디'가 목록에 키가 있을 때는 목록 커서라는 설명이 맞는지 확인.
- docs/history/2026-11-19-feature-upgrade-unit-reports/U1.md 와 docs/plan/2026-11-19-feature-upgrade-12.md 의 OUTPUT LATENCY 알림 문구 인용: 알림이 토스트 88자 절단 때문에 장치 이름을 문장 끝으로 옮긴 새 문구로 바뀌었다고 기록(원문 보고는 그대로 두고 계획서 결과 절에 한 줄).
- docs/quality-assurance 실기 점검표: 세 언어로 실제 창에서 (a) OUTPUT LATENCY 알림이 토스트에서 두 행 이름까지 보이는지, (b) 키 편집기 조합 힌트·충돌 안내가 잘리지 않는지, (c) LAYOUT PICK 안내 줄 둘과 CAPTURED 목록, (d) 결과 화면 내장 안내 줄의 P 안내를 확인하는 항목 추가. 테스트 부채: CAPTURED 목록이 비었는데 capture 키를 해제한 경우의 안내 문구('PRESS  ON A RESULT'), 한·일 글자 폭 시험이 전각 1em 근사라는 점.
- docs/skin.md: 결과 바닥 안내 문구 변경('R, N AND P AS SHIPPED')과 선곡 바닥 안내 'H KEY GUIDE' 로 바뀐 것(U7·U8 보고가 이미 요청한 항목과 같은 절).

## 돌린 검사

- `cargo test -p rbms-render --locked --no-fail-fast` → 통과: lib 851 passed, 통합 시험 포함 전부 0 failed. 골든 `golden_result` 8 passed(GOLDEN_RESULT_PACED 교체 뒤). 이후 text::result 5 passed 로 새 폭 시험을 따로 확인했습니다.
- `cargo test -p rbms-player --lib --locked --no-fail-fast` → 통과: 3279 passed, 0 failed, 2 ignored. 기준선 3272 + 새 시험 7개.
- `cargo clippy -p rbms-render -p rbms-player --all-targets --locked -- -D warnings` → 통과(경고 0)
- `cargo fmt --all --check` → 통과
- `git diff 의 `//` 주석·#[allow]·unsafe·todo!·eprintln! 검사` → 추가된 것 없음
