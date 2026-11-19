# S1 테스트 웨이브 보고 (wf_3df278ff-bb7)

> 테스트 웨이브의 에이전트가 끝낼 때 낸 보고 원문이다. 메인이 병합하면서 바꾼 것은 계획서 9절에 있다.

- 상태: done

## 바꾼 파일

- apps/rbms-player/src/window_mode.rs
- apps/rbms-player/src/skin_screen/tests.rs
- crates/rbms-config/src/tests.rs

## 요약

U2(FULLSCREEN, M9)와 U9·U5(오프셋 단위, M16·M19)를 항목별로 코드와 시험으로 확인했습니다. 결함 하나를 찾아 고쳤습니다. 세로 모니터에서 독점 모드를 고르는 함수가 가장 가까운 모드를 잘못 골랐습니다(1080x1920 모니터에서 RESOLUTION 1920x1080 이 600x1024 로 감). 고치기 전 실패를 먼저 재현했습니다. 폴백 판정을 순수 함수 `exclusive_choice` 로 빼서 시험을 더했고, 옛·새 WINDOW MODE 토큰 읽기 시험과 오프셋이 문서 단위로 화면과 SKIN MENU 미리보기에 같은 비율로 나오는 화소 시험도 더했습니다. 새 미리보기 시험은 이전 규칙을 흉내 낸 임시 편집에서 실패하는 것을 확인했고 편집은 되돌렸습니다. judge_sizes 는 디버그 빌드에서 각각 20.2·18.4·25.9 초이고 셋을 함께 돌리면 25.7 초라 60 초를 넘는 시험이 없었습니다. 실제 창에서 세 모드를 다시 전환하는 실기는 사용자 디스플레이 모드를 바꾸므로 다시 하지 않았고 U2 보고의 실기 결과를 그대로 인용합니다.

## 기준별 판정

- [pass] 전환표 아홉 쌍(W/B/F 서로)과 FULLSCREEN 중 RESOLUTION 변경: window_mode.rs `changes` 와 시험 `every_change_between_the_three_modes_asks_for_its_own_steps`(9쌍 x 같은 크기/다른 크기, 표 길이 단정 포함). (Fullscreen,Fullscreen) 크기 변경은 [EnterExclusive(새 크기)], Borderless 중 크기 변경은 아무 것도 안 함, B->F 는 [LeaveFullScreen, EnterExclusive]. cargo test window_mode 13 passed
- [pass] 저장값 FULLSCREEN 으로 시작하는 경로: `opening_rows` 가 FULLSCREEN 을 일반 창으로 열고 첫 `AppWindow::sync` 가 EnterExclusive 를 낸다(시험 `exclusive_full_screen_opens_as_a_window_and_takes_its_video_mode_on_the_first_sync`). lib.rs 의 resumed 에서 AppWindow::new 뒤 frame() 첫머리 sync_window 가 호출되는 것을 코드로 확인. 실창 시작은 U2 보고의 실기 결과에 의존(재실행 안 함)
- [pass] 독점 모드를 못 얻을 때의 BORDERLESS 폴백과 알림: 세 실패 사유(Wayland, 모니터 없음, 모드 목록 빔)를 새 순수 함수 `exclusive_choice` 로 빼 시험 `exclusive_full_screen_that_cannot_be_had_gives_the_reason_and_the_one_that_can_gives_its_mode` 로 고정. `enter_exclusive` 는 Err 에서 notify_text(FullscreenFellBackToBorderless) 후 Borderless 로 간다. 알림은 notices.rs:249 세 언어와 영어 골든(notices.rs:933)이 있다. 실제 Wayland·모드 없음 환경에서의 동작은 실행하지 못했다
- [pass] 모드 고르기 함수 경계: 빈 목록, 같은 거리, 주사율·색 깊이 순서, 세로 모니터: 빈 목록 None, 같은 거리는 큰 면적(1600x1100), 같은 크기는 주사율 높은 쪽 다음 색 깊이 큰 쪽(시험 the_video_mode_of_the_chosen_size...)은 기존 시험이 고정. 세로 모니터는 fail 이었고 수리함: 고치기 전 시험이 `left: Some(4) right: Some(0)` 로 실패(1080x1920 목록에서 1920x1080 이 600x1024 로 감), 모드와 같은 방향으로 돌려 비교하게 고친 뒤 `a_monitor_stood_on_its_side_takes_the_mode_of_the_chosen_size_turned` 통과. 가로 모드에서는 결과가 달라지지 않음(정렬 매칭이 L1 거리를 최소로 하므로)
- [pass] 옛 설정 파일의 WINDOW MODE 토큰(WINDOWED·BORDERLESS, 모르는 토큰)이 그대로 읽히고 새 토큰이 저장·복원: 새 시험 `every_stored_window_mode_token_reads_back_as_the_mode_it_names`: WINDOWED·BORDERLESS·FULLSCREEN·소문자·공백 변형이 스키마 버전 변경 없이 읽히고 옆 행(RESOLUTION, LETTERBOX)이 그대로이며, 세 모드 모두 ron 저장 뒤 읽기에서 복원. 모르는 토큰(EXCLUSIVE)은 기존 시험 the_window_choices_are_stored_under_their_tokens_and_unknown_ones_fall_back 이 WINDOWED 로 폴백함을 고정. cargo test -p rbms-config 187+15 passed
- [pass] LETTERBOX 와 fits_screen_shape 가 FULLSCREEN 에서 비율을 지킴: schema.rs:814 `letterbox || window_mode.covers_the_monitor()`, 시험 a_borderless_window_is_always_fitted_to_its_shape 가 LETTERBOX 꺼짐에서 BORDERLESS·FULLSCREEN 모두 true 를 고정. lib.rs 의 draw_overlays 가 매 프레임 apply_letterbox(fits_screen_shape()) 로 GPU 에 전달
- [pass] 오프셋 길이·각도·알파·OFFSET_ALL 각각이 저장값 그대로: skin_screen.rs:259-262 `DocumentOffsets::offset` 이 `stored.offset(id)` 를 그대로 돌려줌. 시험 a_nudge_is_answered_in_the_documents_own_units 가 x,y,w,h,r,a 전부와 OFFSET_ALL 을 저장값과 비교. 적용 쪽 `apply_offset`(rbms-skin dst.rs:531)은 문서 사각형에 더하므로 환산이 따로 없음
- [pass] 1920x1080 이 아닌 문서에서 같은 규칙: 기존 stage/play/tests.rs a_nudge_is_read_in_the_units_of_a_document_laid_out_at_another_size(1280x720 문서를 1920x1080 에 그림, x 축). 새 시험이 y 축을 더함(고치기 전 규칙을 흉내 낸 임시 편집에서 `the part is not raised by 30 units` 로 실패, 편집은 되돌림). w·h 는 DocumentOffsets 단위 시험과 dst.rs 가 문서 좌표에 더하는 것으로 확인했고 끝에서 끝 화소 시험은 하지 않음
- [pass] SKIN MENU 미리보기에서 같은 값으로 보임: skin_screen/preview.rs 가 같은 `skin_offsets(screen)` 를 쓰고 문서를 사각형에 축척. 새 시험 a_nudge_moves_a_part_by_the_same_share_of_the_screen_on_a_larger_target_and_in_the_preview: 1920x1080 화면은 JUDGE OFFSET Y 30 이 45 화소, 같은 캔버스 위 절반 크기 미리보기 사각형은 22.5 화소(30 문서 단위)로 나옴. 기존 시험은 체크섬이 달라졌는지만 봤다
- [pass] 오프셋을 말하는 문구가 세 언어에서 새 규칙과 맞음: text/menus/skin_menu.rs:86-88,121-123 OffsetPlaceHelp·AutomaticPlaceHelp 가 'skin pixels / 스킨 좌표의 픽셀 / スキン座標のピクセル' 로 고쳐져 있고 화면 전체는 퍼센트, 각도는 도, 알파는 -255~+255 문구는 맞음. 스킨 lua i18n 과 text/settings 에는 오프셋 단위 문구가 없음. 용어집 오프셋/オフセット 과 일치. cargo test text:: settings skin_menu 446 passed(도움말 폭 시험 포함)
- [pass] U9 완료 기준: JUDGE POSITION 30 이 모든 크기에서 30 문서 화소: judge_sizes the_default_pack_raises_its_judgement_by_the_same_document_pixels_at_every_size 가 1280x720·1920x1080·2560x1440·1728x972·1728x1117 에서 기준(1920x1080 오프셋 0 판 + 30)과 0.5 문서 화소(또는 반 장치 화소) 안에서 같음을 확인하고 판정선은 그대로임을 확인. 3840x2160·1600x900 은 시험 시간 때문에 메인이 뺀 상태
- [pass] judge_sizes 시험 실행 시간(디버그), 60초 초과 시 장수 줄이기: 단독 실행: draws_..._at_every_size 20.2 s(5장), lifts 18.4 s(4장), raises 25.9 s(6장). 셋을 함께 judge_sizes 필터로 25.7 s. 장당 1.7~5 s(2560x1440 이 가장 무거움, 첫 장은 6 s 의 콜드 스타트). 60 초를 넘는 시험이 없어 코드는 바꾸지 않음. 느린 러너(2~3배)에서는 raises 가 60 초 안팎이 될 수 있어 제안만 함
- [untested] 이 Mac 에서 세 모드를 실제로 전환(실기): 사용자의 디스플레이 모드를 바꾸는 실기라 이번 검증에서 다시 하지 않음. U2 보고(docs/history/2026-11-19-feature-upgrade-unit-reports/U2.md)에 WINDOWED·BORDERLESS·FULLSCREEN 시작과 W→B, B→F, F 중 해상도 변경, F→B, B→W, W→F, F→W, 빠른 연속 전환 성공, 강제 종료 뒤 원래 모드 복귀가 기록돼 있다

## 찾은 것

- [medium, 고침] 세로 모니터에서 독점 모드 고르기가 가장 가까운 모드를 잘못 고름: `nearest_video_mode` 가 RESOLUTION(항상 가로) 과 모드 크기를 방향 구분 없이 가로+세로 차이의 합으로 비교했다. 1080x1920 모니터의 목록 [1080x1920, 900x1600, 768x1366, 720x1280, 600x1024] 에서 1920x1080 을 요청하면 600x1024(인덱스 4)를 골랐다(수리 전 시험 출력 `left: Some(4) right: Some(0)`). 수리: 모드가 세로(높이>너비)이면 요청 크기를 돌려서 비교. 가로 모드의 결과는 같다(정렬된 짝짓기가 L1 거리를 최소로 함). (시험: apps/rbms-player/src/window_mode.rs window_mode::tests::a_monitor_stood_on_its_side_takes_the_mode_of_the_chosen_size_turned (수리 전 실패, 후 통과). 기존 a_size_the_monitor_lacks_takes_the_nearest_one 등 13개 통과)
- [low, 고침] 폴백 결정 분기가 순수 함수가 아니라 시험으로 고정되지 않았음: `exclusive_video_mode` 가 Window 를 받아 Wayland·모니터 없음·빈 목록 세 실패 분기를 시험할 수 없었다(빈 목록은 nearest_video_mode 로만 확인됨). 순수 함수 `exclusive_choice(on_wayland, modes, size)` 로 빼서 네 분기를 시험하게 했다. 동작 변경 없음. (시험: window_mode::tests::exclusive_full_screen_that_cannot_be_had_gives_the_reason_and_the_one_that_can_gives_its_mode)
- [low, 안 고침] stage/skin_menu.rs 의 문서 주석이 오프셋 단위를 아직 '화면 화소'로 적음: apps/rbms-player/src/stage/skin_menu.rs:795 `A place and a size are counted in pixels of the screen, except for the one offset that moves a whole play screen` 는 M19 이전 규칙이다. 지금은 스킨 문서 단위이고 OFFSET_ALL 만 화면의 비율이다. 소유 파일 밖이라 고치지 않았다. 제안 문구: 'A place and a size are counted in pixels of the skin's own document, ...' (시험: 해당 없음(주석). 같은 파일의 skin_row_help 문구 시험은 통과)
- [low, 안 고침] judge_sizes 시험이 느린 러너에서 60초 근처가 될 수 있음(현재 60초 초과 없음): 이 Mac 디버그 빌드: 20.2/18.4/25.9 s, 합쳐서 25.7 s. 장당 1.7~5 s(콜드 스타트 6 s 포함, 2560x1440 이 가장 무거움). 15장 중 1920x1080 오프셋 0 기준 판이 세 시험에서 3번 그려지고 1728x972 는 헤드리스에서 16:9 크기 하나일 뿐이다. 제안: (1) 기준 판을 OnceLock 으로 세 시험이 공유(2장, 약 10% 절감, 적용 범위 손실 없음), (2) 더 줄여야 하면 raises 시험에서 1728x972 를 뺀다(비균등 배율은 720p·1440p 가 이미 덮음. 다만 레터박스 켬 16:10 표지가 사라진다). 60초를 넘는 시험이 없어 적용하지 않음. (시험: cargo test -p rbms-player --locked --lib judge_sizes (3 passed, 25.67 s))
- [low, 안 고침] 옛 규칙 문서 서술(skin.md 474행, play-options.md 386행)이 아직 '1920x1080 기준·화면 픽셀': docs/skin.md:474 `그 픽셀만큼 ... 좌표는 스킨의 1920x1080 기준이다`, docs/play-options.md:386 `스킨 좌표(1920x1080 기준)다`. M19 뒤에는 스킨 문서 단위(1280x720 문서는 그 단위)이고 해상도·배율과 무관하다. (시험: 해당 없음(문서))

## 남은 위험

- Wayland·모드 없음 폴백, Windows·X11 독점 전환, macOS 독점 모드에서의 Game Mode 는 실기로 확인하지 못했다. 폴백은 결정 함수와 알림 문구만 시험으로 고정돼 있고 `AppWindow::enter_exclusive` 자체는 창이 필요해 시험하지 못했다.
- winit 0.30 macOS 는 `CGDisplayCapture`·`CGDisplaySetDisplayMode` 실패나 모르는 픽셀 인코딩에서 패닉할 수 있고 코드로 막을 수 없다(U2 보고와 같음).
- 세로 모니터 수리는 순수 함수 시험으로만 확인했다. 실제 세로 모니터의 `video_modes()` 가 돌려진 크기(1080x1920)를 주는지는 OS·winit 에 달려 있고 확인하지 못했다.
- M19 로 저장값의 뜻이 바뀐다. 1920x1080 물리 화소가 아닌 화면에서 맞춘 오프셋은 한 번 다시 맞춰야 하고 옮기는 코드는 없다(계획대로).
- 끝에서 끝 화소 시험은 y 축 하나(와 기존 x 축)다. 문서 크기가 다른 경우의 w·h 는 단위 시험과 dst.rs 의 적용 방식으로만 확인했다.
- 변경 파일 중 crates/rbms-config/src/tests.rs 는 지정 파일 목록(window.rs)에 없지만 U1·U2 병합 주의 파일이고 창 토큰 시험 옆에 시험 하나만 더했다. 다른 에이전트가 같은 파일을 고쳤다면 병합 때 충돌할 수 있다.

## 갱신이 필요한 문서

- docs/skin.md 474행: Judge Position 을 '스킨 문서 좌표 단위(1280x720 문서는 그 단위), 해상도·배율과 무관'으로 고치고 '그 픽셀만큼', '1920x1080 기준' 서술을 정정
- docs/play-options.md 386행: 같은 내용으로 정정
- docs/skin.md E6 행과 docs/crates.md window_mode.rs 설명: WINDOW MODE 에 FULLSCREEN 추가, BORDERLESS·FULLSCREEN 모두 비율을 지킴, 독점 모드 고르기에 세로 모니터(모드 방향에 맞춰 비교) 규칙 추가, B→F 가 일반 창을 거침, 폴백 알림
- docs/reference/2026-10-24-i18n-glossary.md: 주사율/リフレッシュレート, 화면 모드/画面モード 용어 등록 검토(notices.rs·descriptions.rs 가 새로 씀)
- docs/reference/skin-color-customization.md 250·331·402행과 docs/acknowledge/2026-10-22-adr-skin-colour-rows.md 23행: units_per_pixel 환산 언급과 skin_screen.rs 줄 번호 갱신
- M19 ADR: 사용자 오프셋을 스킨 문서 단위로 읽는 결정과 레퍼런스와의 차이(레퍼런스는 출력 화소에 더함)
- docs/quality-assurance: 실제 창(Retina·4K)에서 JUDGE POSITION 30 이 같은 자리인지 보는 실기 점검, 세로 모니터 FULLSCREEN·Wayland 폴백 알림 실기 점검 항목, judge_sizes 시험 시간(기준 판 공유 제안)을 테스트 부채에 기록
- apps/rbms-player/src/stage/skin_menu.rs:795 의 문서 주석(화면 화소 서술)은 소유 단위가 정정

## 돌린 검사

- `cargo test -p rbms-config --locked` → 통과: 187 passed + 15 passed, 0 failed(새 시험 every_stored_window_mode_token_reads_back_as_the_mode_it_names 포함)
- `cargo test -p rbms-player --locked --lib window_mode` → 통과: 13 passed(새 시험 2개 포함). 세로 모니터 시험은 수리 전 코드에서 실패를 확인
- `cargo test -p rbms-player --locked --lib -- skin_screen:: nudge skin_menu skin_preview play_parts text:: settings` → 통과: 446 passed, 0 failed, 51.5 s
- `cargo test -p rbms-player --locked --lib a_nudge_moves_a_part` → 통과(5.5 s). DocumentOffsets 를 이전 규칙(길이 x 2/3)으로 임시 편집한 상태에서는 `the part is not raised by 30 units of its document` 로 실패, 편집은 /tmp 백업으로 되돌려 skin_screen.rs 는 바뀌지 않음
- `cargo test -p rbms-player --locked --lib -- skin_screen:: window_mode nudge notices a_settings_upload judge_sizes` → 통과: 95 passed, 0 failed, 26.3 s(최종 상태)
- `cargo test -p rbms-player --locked --lib <judge_sizes 시험 3개 각각 단독>` → 20.2 s / 18.4 s / 25.9 s, 모두 통과. 셋 함께 25.7 s
- `cargo clippy -p rbms-config -p rbms-player --all-targets --locked -- -D warnings` → 통과(경고 없음)
- `cargo fmt --all --check` → 통과(exit 0)
- `skin-dump` → 실행하지 않음: 스킨 파일을 고치지 않았다
- `rbms-player 실제 창에서 세 모드 전환` → 실행하지 않음: 사용자 디스플레이 모드를 바꾸는 실기. U2 보고의 실기 결과를 인용
