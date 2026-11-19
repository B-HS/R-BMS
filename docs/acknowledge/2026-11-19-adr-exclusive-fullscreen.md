# ADR: WINDOW MODE 에 독점 전체 화면 FULLSCREEN 을 더한다(일반 창으로 열고, BORDERLESS 에서도 일반 창을 거쳐 들어간다)

> 최종 갱신 2026-11-19 · 대응 커밋 `e62240d`
> 상태: 유효(코드는 `dev` 에 있음. 사용자가 정한 것은 요구 한 줄뿐이고 M9 와 "구현에서 정한 것"은 메인·구현 단위의 결정이며 사용자 확인 전. macOS 에서만 실행해 확인했고 Windows·Linux 는 확인하지 못함) · 결정일 2026-11-19 · 결정자 사용자(요구)와 메인(M9), 구현 단위 U2(구현에서 정한 것) · 커밋 `e62240d`
> 관계: [출력 지연 보정](2026-11-19-adr-output-latency-compensation.md)의 Game Mode 설명이 BORDERLESS 를 전체 화면 경로로 적는다. DISPLAY 탭 행의 서술은 [skin.md](../skin.md) 1.3절 E6 과 [option-panels.md](../reference/option-panels.md) 에 있다. 계획 U2, 조사 5절, 보고 [U2.md](../history/2026-11-19-feature-upgrade-unit-reports/U2.md).

## 배경

- 사용자 요청 요지(2026-11-19): 창 모드를 window·borderless·fullscreen 으로. 지금까지 `WindowMode` 는 `Windowed`·`Borderless` 둘이었고(`WINDOW_MODE_COUNT = 2`), `Borderless` 는 winit 의 `Fullscreen::Borderless(None)` 이었다. macOS 에서 이것은 OS 의 전체 화면(Space)이다. 독점 전체 화면(`Fullscreen::Exclusive`)은 없었다.
- winit 0.30.13 에 `Fullscreen::Exclusive(VideoModeHandle)` 와 `MonitorHandle::video_modes()` 가 있다.

## 결정

### 1) 사용자가 정한 것

| 번호 | 결정 |
| --- | --- |
| D1 | 창 모드에 WINDOWED·BORDERLESS 와 함께 FULLSCREEN 을 둔다 |

### 2) 메인 결정(사용자 미확인)

| 번호 | 결정 |
| --- | --- |
| M9 | WINDOW MODE 에 FULLSCREEN(독점)을 더한다. 해상도는 RESOLUTION 행에 가장 가까운 모니터 모드, 주사율은 가장 높은 것. 독점이 안 되는 환경은 BORDERLESS 로 떨어진다. 단축키(Alt+Enter)는 요청 밖이라 더하지 않는다 |

### 3) 구현에서 정한 것(구현 단위가 정함, 사용자 미확인)

| 항목 | 내용 |
| --- | --- |
| 값과 행 | `WindowMode::Fullscreen`(설정 토큰·라벨 `FULLSCREEN`), `WINDOW_MODE_COUNT = 3`, 행 순서 WINDOWED → BORDERLESS → FULLSCREEN, `WINDOW_MODE_LABELS` 도 셋. 설정 행 수는 그대로다. `WindowMode::covers_the_monitor()`(Windowed 가 아님)를 더했고 `DisplayOptions::fits_screen_shape()` 가 이것을 쓴다 |
| 모드 고르기 | 순수 함수 `nearest_video_mode(&[(w, h, 밀리헤르츠, 색 깊이)], wanted) -> Option<usize>`. 가로·세로 차이의 합이 가장 작은 모드(RESOLUTION 과 크기가 같으면 0), 같은 거리면 면적이 큰 쪽, 같은 크기 안에서는 주사율이 높은 쪽, 그다음 색 깊이가 큰 쪽. 화면 비율은 보지 않는다(비율은 LETTERBOX 가 맞춘다). 고르는 모니터는 창이 있는 모니터이고 없으면 주 모니터다. 모드 크기는 RESOLUTION 값을 물리 화소로 그대로 쓴다(macOS winit 의 `video_modes()` 는 HiDPI 가 아닌 화소 크기를 준다) |
| 일반 창으로 열기 | FULLSCREEN 은 창을 BORDERLESS 가 아니라 일반 창으로 열고(`opening_rows` 가 Fullscreen 을 Windowed 로 바꿈) 첫 `AppWindow::sync` 에서 독점 모드로 들어간다. 이유는 아래 "BORDERLESS 를 거치지 않는 이유" |
| 전환표 | `changes(applied, wanted)` 가 아홉 쌍을 모두 다룬다. 일반 → FULLSCREEN: `EnterExclusive`. BORDERLESS → FULLSCREEN: `[LeaveFullScreen, EnterExclusive]` 두 단계(일반 창을 거침). FULLSCREEN → 일반: `[LeaveFullScreen, Resize]`. FULLSCREEN → BORDERLESS: `EnterBorderless`. FULLSCREEN 중 RESOLUTION 변경: 그 크기의 모드로 바로 `EnterExclusive`. 모든 OS 에 같은 표를 쓴다(Windows·X11 에서는 BORDERLESS → FULLSCREEN 에서 한 번 깜빡일 수 있다) |
| `WindowChange` 이름 | `LeaveBorderless` 를 `LeaveFullScreen` 으로 바꿨다(두 전체 화면에서 나가는 단계를 함께 뜻함) |
| 폴백 | 독점 모드를 얻을 수 없으면 BORDERLESS 로 떨어진다: Wayland(`RawWindowHandle::Wayland` 로 판별, winit 이 모드 요청을 처리하지 않음), 창이 모니터에 없음, 모니터가 모드를 나열하지 않음. 이때 알림 `NoticeText::FullscreenFellBackToBorderless`(세 언어, 자리표시자 없음)를 내고 원인은 `window: exclusive full screen not taken: <영어 원인>` 으로 표준 출력에 남긴다. 설정값은 FULLSCREEN 으로 남고, 행이 바뀔 때만 다시 시도한다(매 프레임 시도하지 않음) |
| 비율 | FULLSCREEN 도 BORDERLESS 처럼 항상 화면 비율을 지킨다(`fits_screen_shape`). LETTERBOX 설명 문구를 세 언어로 맞췄다 |
| 문구 | WINDOW MODE 설명에 FULLSCREEN 과 "macOS 의 BORDERLESS 는 시스템 전체 화면이고 Game Mode 가 동작함" 을 넣었다. RESOLUTION 설명은 FULLSCREEN 이 요청하는 모드라는 내용을, LETTERBOX 는 BORDERLESS·FULLSCREEN 이 항상 비율을 지킨다는 내용을 갖는다. `rbms-config` 의 영어 행 설명 세 개도 같이 고쳤다 |

### 4) BORDERLESS 를 거치지 않는 이유

계획은 "깜빡임을 줄이려면 BORDERLESS 로 열고 전환"이었다. 이 Mac(내장 Liquid Retina XDR, 기본 2624x1696@120)에서 BORDERLESS 로 연 창을 `Exclusive` 로 바꾸면 화면 모드는 1920x1200 으로 바뀌지만 창이 옛 모드의 틀에 남아 y=-496 에 걸리고 화면 아래 절반이 검게 남았다. winit macOS 의 (Borderless, Exclusive) 분기가 창 레벨만 바꾸기 때문이다. 일반 창에서 들어가면 0,0 에 1920x1200 으로 정확히 놓였다. 그래서 열 때도 전환 때도 일반 창을 거친다.

## 이유

- 독점 전체 화면은 OS 합성기를 건너뛰고 모니터를 고정 모드로 쓰므로 일부 환경에서 지연·프레임 안정성이 낫다. 사용자가 요청한 세 모드를 모두 갖춘다.
- 가장 가까운 모드를 고르는 규칙은 순수 함수라 시험으로 고정할 수 있고, RESOLUTION 이 모니터가 나열하지 않는 크기여도 항상 어떤 모드든 얻는다.
- 폴백이 있어 독점이 안 되는 환경에서도 전체 화면 창은 보장된다.

## 기각된 대안

| 대안 | 기각 이유 |
| --- | --- |
| 단축키(Alt+Enter) 추가 | 요청 밖 |
| FULLSCREEN 을 BORDERLESS 로 열고 독점으로 전환 | macOS 에서 창이 옛 틀에 남아 화면 절반이 가려지는 것을 실측으로 확인 |
| 화면 비율 기준으로 모드 고르기 | LETTERBOX 가 비율을 맞추므로 중복이고 모드가 더 적게 후보에 든다 |
| 폴백 없이 오류 | Wayland 등에서 전체 화면 창조차 못 얻는다 |

## 이 Mac 에서 실행해 본 것

내장 디스플레이(가장 작은 모드 1920x1200)에서 스크래치 HOME 과 `--server http://127.0.0.1:9` 로 실행하고, 화면 모드는 CoreGraphics 로, 창 위치·크기는 System Events 로 읽었다.

- 시작: WINDOWED 는 1280x752 창(제목 막대 포함), BORDERLESS 는 0,49 의 2624x1647 창, FULLSCREEN 은 RESOLUTION 1920x1080 에서 1920x1200@120 모드로 열렸고 창은 0,0 의 1920x1200, 16:9 레터박스로 그려졌다.
- 실행 중 전환: W→B, B→F(1920x1200@120, 창 0,0), FULLSCREEN 중 RESOLUTION 2560x1440(2560x1600@120), F→B(원래 모드 2624x1696 복귀), B→W(2560x1472 창), W→F 직접, F→W, W→B→F 를 0.4초 간격으로 연속. 모두 창이 화면에 맞게 놓였고 패닉은 없었다.
- FULLSCREEN 에서 프로세스를 강제로 끝내도 원래 모드 2624x1696@120 으로 돌아왔다.
- 독점 모드를 얻지 못해 BORDERLESS 로 떨어지는 경로는 이 Mac 에서 재현할 수 없어 빈 모드 목록의 순수 시험으로만 확인했다.

## 영향과 관련 파일

- 코드: `crates/rbms-config/src/{window,schema,settings,tests}.rs`, `apps/rbms-player/src/window_mode.rs`(시험: `every_change_between_the_three_modes_asks_for_its_own_steps`, `exclusive_full_screen_opens_as_a_window_and_takes_its_video_mode_on_the_first_sync`, 모드 고르기 `the_video_mode_of_the_chosen_size_is_taken_at_its_highest_refresh_rate`·`a_size_the_monitor_lacks_takes_the_nearest_one`·`a_monitor_that_lists_no_video_modes_has_none_to_take` 등), `text/settings/descriptions.rs`, `text/notices.rs`. `lib.rs` 의 창 생성·동기화 호출부는 그대로 맞아 고치지 않았다. 설정 계약 파일은 기본값이 그대로라 바뀌지 않았다.
- 알려진 위험(테스트 부채 3.166·3.167, 실기 QA 3.29 G3):
  - Windows·X11 의 독점 전환, Wayland·모드 없음의 BORDERLESS 폴백은 실기로 확인하지 못했다. Windows 에서는 FULLSCREEN 으로 시작할 때 일반 창이 한 프레임 보였다가 전환될 수 있다.
  - winit 0.30.13 macOS 는 `video_modes()` 에서 모르는 픽셀 인코딩을 만나면 `unimplemented!()` 를 내고 `CGDisplayCapture`·`CGDisplaySetDisplayMode` 가 실패하면 assert 로 패닉한다. 이 Mac 에서는 일어나지 않았지만 다른 디스플레이에서는 코드로 막을 수 없다.
  - macOS 독점 전체 화면에서 Game Mode 가 켜지는지는 확인하지 못했다.
  - 사용자의 디스플레이 모드를 시험 중 잠시 바꿨고 끝난 뒤 원래 모드로 돌아온 것을 확인했다.
  - 알림과 설정 설명의 한국어·일본어 문구는 새로 썼다. 용어집에 없던 '화면 모드/画面モード', '주사율/リフレッシュレート' 는 용어집 6.10절에 올렸다.
- 문서: [play-options.md](../play-options.md) 20절, [skin.md](../skin.md) 1.3절 E6, [crates.md](../crates.md) `window_mode.rs`.

## 날짜와 사용자 원문

2026-11-19.

- 사용자 요청 요지(PROCESS 체크리스트): "5 창 모드 window·borderless·fullscreen". 원문 전문은 이 저장소에 없다.
