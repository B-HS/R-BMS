# 계획: 기능 고도화 12건 (2026-11-19)

최종 갱신 2026-11-19 · 대응 커밋 `439a93b`(계획 시점). 계획·검토: 메인(Fable). 조사 근거: [reference/2026-11-19-feature-upgrade-12/README.md](../reference/2026-11-19-feature-upgrade-12/README.md). 체크리스트: [PROCESS.md](../PROCESS.md) 맨 위.

## 0. 진행 방식 (사용자 지시)

"모든 계획 및 검토는 Fable 로 하고 계획 -> 검토 -> 구현(opus + workflow) -> 테스트(sonnet + opus, Fable 이 workflow 에 필요한 모델 선정)", "workflow 는 사용해도 subagent 는 사용하지 말도록".

- 계획·검토·병합·게이트·커밋은 메인이 한다. Agent 도구는 쓰지 않는다.
- 구현은 Workflow 의 Opus(effort high), 테스트는 Workflow 의 Sonnet(xhigh)·Opus(high). Fable·Haiku 는 workflow 에 배정하지 않는다(HANDOFF 5절).
- 웨이브마다 worktree 격리로 병렬 실행하고, 메인이 `git apply` 로 합친 뒤 게이트·커밋·푸시·CI 확인을 한다. worktree 는 `origin/dev` 에서 만들어지므로 다음 웨이브 전에 푸시한다.
- 범위는 클라이언트(Rust·기본 스킨)다. 웹·DB·릴리스는 하지 않는다.

## 1. 메인 결정

요구가 정하지 않은 부분을 메인이 정했다. 사용자 확인 전이다.

| 번호 | 항목 | 결정 | 버린 대안과 이유 |
| --- | --- | --- | --- |
| M1 | 1 블루투스 | 장치가 보고하는 출력 지연을 오디오 시계에 더해 BGM·노트·판정을 들리는 소리에 맞춘다(macOS 자동). AUDIO 탭 OUTPUT LATENCY 행 AUTO(기본)/OFF | 수동 값만 두기: JUDGE TIMING ±200 ms 가 이미 그 역할을 한다 |
| M2 | 1 블루투스 | macOS Game Mode 자격을 번들에 넣는다(`LSSupportsGameMode`·`GCSupportsGameMode`). 전체 화면에서 OS 가 AirPods 지연을 줄인다 | 없음 |
| M3 | 1 블루투스 | 지연 조회는 `coreaudio-rs`(이미 `Cargo.lock` 에 있는 cpal 의 전이 의존성)의 안전한 함수로 한다. `rbms-audio` 에 macOS 한정 직접 의존성 한 줄이 늘고, `Cargo.lock` 은 `rbms-audio` 의 의존 목록에 한 줄만 늘며 새 크레이트는 없다(cpal 과 같은 feature `core_audio`·`audio_toolbox`, `default-features = false`). `unsafe` 는 쓰지 않는다 | `unsafe` FFI 직접 호출: 프로젝트 규칙 위반 |
| M4 | 1 블루투스 | Windows·Linux 는 자동 조회가 없다(cpal 이 API 를 주지 않고 안전한 경로가 없다). AUTO 는 0 ms 로 동작하고 JUDGE TIMING 으로 맞춘다 | `windows` 크레이트 직접 호출: `unsafe` 필요 |
| M5 | 1 블루투스 | 지연값은 스트림을 열 때와 판을 시작하기 전에 다시 읽는다. 판 도중에는 바꾸지 않는다(시계가 끊기지 않게) | 주기 조회로 판 도중 반영: 노트가 튄다 |
| M6 | 2 폴더 | 플레이 머리 둘째 줄 오른쪽에 출발 폴더를 강조색으로 쓴다. 값은 선곡 경로의 마지막 두 마디(`発狂BMS / ★8`), 루트면 빈 값 | 난이도 이름 앞: 좁은 머리(DP)에서 난이도·레벨을 밀어낸다 |
| M7 | 3 옵션 | 필드 아래(HI-SPEED·GREEN 밑 줄)에 그 판의 게이지·배치·어시스트 옵션을 쓴다. 값은 설정이 아니라 판의 실제 값(빌린 배치, 코스 게이지, GAUGE AUTO SHIFT 반영) | 설정 인덱스(40·42·43)로 스킨이 그리기: 빌린 배치·코스에서 틀린다 |
| M8 | 4 램프 | 선곡 목록 막대 램프, MY RECORDS 줄, 랭킹 줄이 기존 등급 색 4개를 쓴다. 폴더 램프 분포와 스크롤 분포는 11칸 그대로 | 새 색: 웹·결과 화면과 어긋난다 |
| M9 | 5 창 | WINDOW MODE 에 FULLSCREEN(독점) 추가. 해상도는 RESOLUTION 행에 가장 가까운 모니터 모드, 주사율은 가장 높은 것. 독점이 안 되는 환경은 BORDERLESS 로 떨어진다 | 단축키(Alt+Enter) 추가: 요청 밖 |
| M10 | 6 Esc | 새 조작 `ESC`(키 편집기의 조작 목록 끝). 키보드 기본 키 없음(Esc 고정 키는 그대로), 패드 기본 없음. 묶인 키·버튼의 누름·뗌을 Esc 키 입력으로 화면에 넘긴다(HOLD·DOUBLE TAP 포함 모든 화면 동일) | 화면마다 패드 처리 추가: 빠지는 화면이 생긴다 |
| M11 | 7 조합키 | 조작·단축키에 키 2개 조합을 묶을 수 있다(키보드·패드). 둘 다 눌린 순간(순서 무관) 실행, 누르고 있는 동안 반복하는 조작은 둘 다 눌린 동안 반복. 레인·역회전 키는 단일 키만 | 3키 이상: 편집기·충돌 규칙이 커지고 요청은 "2개" |
| M12 | 7 조합키 | 조합이 완성된 누름에서는 그 키의 단일 조작·단축키를 실행하지 않는다. 레인 입력은 항상 판정에 간다. 조합의 첫 키에 단일 조작이 묶여 있으면 그 조작이 먼저 실행된다는 안내를 편집기가 낸다(거절하지 않음) | 단일 실행을 뗄 때까지 미루기: 입력 지연 |
| M13 | 8 미리보기 | LAYOUT PICK 미리보기가 실제 시간으로 흐른다: 사용자의 HI-SPEED·GREEN·COVER·LIFT 반영, Space 재생/정지, Page Up/Down 탐색, Home 처음으로, 끝나면 처음부터. 소리는 없다 | 소리 포함: 선곡 미리듣기 내부를 뜯어야 해 위험이 크다. 후속 후보로 기록 |
| M14 | 9 capture | LAYOUT PICK 에 통합한다. 결과 화면 `P`(바꿀 수 있는 단축키. 처음 계획은 `C` 였으나 `C` 는 기본 배열의 5번째 레인 키라 결과 화면에서 "새 배치로 다시 플레이"로도 읽힌다)로 저장, LAYOUT PICK 의 CAPTURED 목록에서 골라 바로 플레이. 저장 대상은 레인 순열로 표현되는 배치(RANDOM·R-RANDOM·MIRROR) | 별도 모달: 미리보기·시작 흐름이 두 벌이 된다 |
| M15 | 9 capture | 저장 파일은 설정 폴더의 `captured_layouts.ron`. 항목: md5, 제목, 모드, 배치 옵션 이름, 시드, 레인 순서 표기, 저장 시각, 그 판의 EX·램프 | 점수 DB 표: 스키마 변경이 필요하고 기록과 무관한 데이터다 |
| M16 | 10 판정 위치 | 여러 해상도로 그려 좌표를 재고 원인을 고친다. 차이가 재현되지 않으면 측정값을 보고하고 사용자에게 화면을 묻는다 | 추측 수정 |
| M17 | 11 NPS | 판정 위 줄의 FAST/SLOW 오른쪽에 현재 NPS(직전 1초 동안 판정선을 지난 노트 수). 스킨 옵션 `NPS Display` On(기본)/Off | 기본 Off: 요청한 기능이 숨는다 |
| M18 | 12 선곡 UI | 머리 한 줄 통합(경로를 머리로), 제목 축소, 바닥 한 줄, 정보창 구획 통일. 스킨만 바꾼다 | 앱 쪽 값 추가: 범위가 커진다 |
| M19 | 10 판정 위치 | (웨이브 1 뒤 추가) 사용자 오프셋(JUDGE POSITION, Judge offset, Judge Detail offset 등 스킨 메뉴의 모든 오프셋)을 스킨 문서 단위로 읽는다. 지금은 화면 물리 화소로 읽어 해상도·Retina 배율마다 이동량이 달랐다. 저장값은 그대로 두고 뜻만 바뀐다(1920x1080 물리 화소로 맞춘 값은 그대로, 다른 크기에서 맞춘 값은 한 번 다시 맞춰야 한다) | 기본 스킨 오프셋만 바꾸기: 규칙이 둘이 된다. 지금대로 두고 문서만 고치기: 사용자가 보고한 문제가 남는다 |
| M20 | 13 램프 반 쪼개기 | (2026-11-19 개정: 사용자 보고로 조건을 "둘 다 있으면 쪼갬"으로 넓혔다. 처음 조건으로는 사용자 기록의 68개 중 3개만 쪼개졌다. 11절) (추가 요청 2026-11-19) 한 차트에 어시스트 없는 클리어 램프(EASY 이상)와 어시스트 램프(SCRATCH AUTO 로 깬 등급 1~4)가 함께 있고 어시스트 쪽이 더 높은 게이지일 때, 선곡 목록 막대의 램프를 위아래 반으로 나눠 위에 어시스트 없는 램프, 아래에 어시스트 등급 색을 그린다. 어시스트 없는 램프가 같거나 높으면 지금처럼 그 램프만 그린다(A-단위 결정과 같은 규칙). 범위는 클라이언트 | 둘 다 있으면 항상 쪼개기: EX HARD CLEAR 옆에 ASSIST EASY 가 붙는 식의 정보 없는 표시가 생긴다. 웹까지: 서버 DB 마이그레이션이 필요해 범위가 커진다 |

## 2. 단위와 소유 파일

파일 소유가 겹치지 않게 나눴다. 겹칠 수 있는 곳은 "병합 주의"에 적었다. 모든 단위 공통 규칙은 4절.

### 웨이브 1 (병렬 5)

#### U1 블루투스 출력 지연 (요구 1)

- 소유: `crates/rbms-audio/**`, `crates/rbms-config/src/audio.rs`, `scripts/macos-app.sh`, `apps/rbms-player/src/app_play.rs` 의 오디오 열기·다시 열기 부분, `apps/rbms-player/src/settings_ui.rs` 의 오디오 부분.
- 병합 주의(U2 와 같은 파일): `crates/rbms-config/src/{settings.rs,schema.rs,lib.rs,tests.rs}`, `apps/rbms-player/src/stage/settings/catalog.rs`, `apps/rbms-player/src/text/settings/{labels,descriptions}.rs`, `apps/rbms-player/src/text/notices.rs`. 자기 행·문구만 더한다.
- 할 일
    1. `rbms-audio` 에 장치 지연 조회 모듈. macOS: 열린 출력 장치의 `kAudioDevicePropertyLatency + kAudioDevicePropertySafetyOffset`(프레임)과 전송 종류를 조사 1.2 의 안전한 함수로 읽는다. 호스트 스레드(`host.rs` 의 `ask`)에서만 부른다. 그 밖의 OS: 값 없음.
    2. 엔진이 장치 지연을 받아 콜백이 내는 `playback_ahead` 에 더한다. 백엔드가 준 앞섬의 200 ms 검사(`MAX_PLAYBACK_AHEAD_MS`)는 백엔드 값에만 건다. 장치 지연의 상한은 따로 둔다(1,000 ms). `audible_us`·`lookahead_us`·예약 경로가 0·160·400 ms 지연에서 맞는지 시험한다.
    3. 지연 적용 시점: 스트림을 열 때, 그리고 판 시작 전(M5). 판 도중 값이 바뀌지 않는다.
    4. 설정: `AudioOptions.compensate_output_latency: bool`(기본 true), AUDIO 탭 OUTPUT LATENCY 행(AUTO/OFF), 다시 열지 않고 바로 반영. 설명 문구 세 언어.
    5. 알림: 보정값이 30 ms 이상이면 스트림을 연 뒤 한 번 알린다(장치 이름, ms, 블루투스 여부, "이 장치용으로 JUDGE TIMING 을 옮겨 뒀다면 되돌리라"). 세 언어.
    6. 디버그 오버레이에 보정값 한 줄.
    7. `scripts/macos-app.sh` Info.plist 에 `LSSupportsGameMode`·`GCSupportsGameMode` true.
- 완료 기준: AirPods 를 연결한 이 Mac 에서 조회값이 7,680 프레임 근처로 읽히는 것을 `rbms-cli` 나 시험 출력으로 보인다(장치가 없으면 건너뛰는 시험). 엔진 시계 시험 통과. OFF 에서 지금과 같은 시계.

#### U2 창 모드 FULLSCREEN (요구 5)

- 소유: `crates/rbms-config/src/window.rs`, `apps/rbms-player/src/window_mode.rs`.
- 병합 주의: U1 과 같은 설정 파일들, `apps/rbms-player/src/lib.rs`(창 생성·동기화 호출부만).
- 할 일
    1. `WindowMode::Fullscreen`(토큰·라벨 `FULLSCREEN`), `WINDOW_MODE_COUNT = 3`, 행 순서 WINDOWED → BORDERLESS → FULLSCREEN.
    2. 독점 모드 고르기: 창이 있는 모니터의 `video_modes()` 에서 RESOLUTION 크기와 같은 것, 없으면 가장 가까운 것, 같은 크기 안에서는 주사율이 가장 높은 것. 고르는 함수는 순수 함수로 두고 시험한다.
    3. 전환표: 세 모드 사이의 모든 전환과 FULLSCREEN 중 RESOLUTION 변경. 독점 모드를 못 얻으면 BORDERLESS 로 떨어지고 알린다.
    4. FULLSCREEN 도 화면 비율을 지킨다(`fits_screen_shape`). LETTERBOX 설명 문구를 세 언어로 맞춘다.
    5. WINDOW MODE 설명에 "macOS 의 BORDERLESS 는 OS 전체 화면이고 Game Mode 대상" 을 넣는다.
- 완료 기준: 전환표·모드 고르기 시험 통과. 이 Mac 에서 세 모드를 실제로 전환해 본 결과(스크래치 HOME, `--server http://127.0.0.1:9`)를 보고한다. 실행하지 못하면 그렇게 적는다.

#### U3 입력: 패드 Esc 와 조합키 (요구 6·7)

- 소유: `apps/rbms-player/src/{keyconfig.rs,keyconfig_tests.rs,gamepad.rs,gamepad/**,app_input.rs,input_debounce.rs}`, `apps/rbms-player/src/stage/{keyconfig.rs,keyconfig/**}`, `apps/rbms-player/src/text/menus/key_editor.rs`, `apps/rbms-player/src/stage/select/guide.rs`, `apps/rbms-player/src/stage/mod.rs` 의 입력 분배 부분.
- 할 일
    1. `ControlAction::Escape`(화면 이름 `ESC`, 설정 키 `ESCAPE`), 키보드·패드 기본 없음. 묶인 키·버튼의 누름과 뗌이 `Stage::handle_key(KeyInput { code: Escape, .. })` 로 간다. 키보드는 공통 래퍼 `Stage::handle_key`(`stage/mod.rs:326`, `note_key` 뒤)에서 키 코드를 Esc 로 바꾸고, 패드는 `Stage::handle_pad`(`stage/mod.rs:374`)에서 Esc 키 입력으로 바꿔 같은 래퍼에 넣는다. 패드 대상은 누름·뗌을 다 낸다.
    2. 조합(M11·M12): 키보드 토큰 `A+B`(기존 단일 토큰 파일은 그대로 읽힌다), 패드는 `PadConfig` 에 둘째 요소 표(`#[serde(default)]`, `skip_serializing_if` 금지). 조작·단축키만, 레인·역회전은 단일.
    3. 눌림 판정(`control_held`·`control_repeating`·`shortcut_held`·`shortcut_for`·패드 `held`)이 조합을 이해한다. 조합이 완성된 누름은 그 키의 단일 조작·단축키를 누른다. 레인은 건드리지 않는다.
    4. 키 편집기: 조작·단축키 칸에서 첫 키를 누른 채 둘째 키를 누르면 조합, 한 키를 눌렀다 떼면 단일. 표기 `A + B`. 충돌: 같은 화면의 같은 조합은 거절, 조합의 첫 키에 단일 조작이 있으면 안내.
    5. 키 가이드(H)와 스킨 바닥 표기가 조합을 `A + B` 로 보인다.
    6. 문구 세 언어.
- 완료 기준: 조합 상태 기계 시험(순서 무관, 반복, 단일 억제, 레인 통과, 포커스 잃음), 옛 키 설정 파일 읽기 시험, Esc 의 IMMEDIATE·HOLD·DOUBLE TAP 이 패드 버튼으로 같은 결과를 내는 시험.

#### U4 어시스트 등급 램프 색 (요구 4)

- 소유: `crates/rbms-render/src/skin_render/songlist/**`, `crates/rbms-render/src/skin_render/songlist.rs`, `crates/rbms-skin` 의 `songlist` 정의와 로더(`loader/branch.rs:102-110` 의 키 목록, `loader/from_lua.rs:507-517`), `assets/skins/rbms-default/lua/select/{list.lua,records.lua}`, 플레이어의 막대 채우기(`apps/rbms-player/src/stage/select/scene.rs:90` 가 `RecordLamp` 를 `i32` 로 바꾸며 등급을 버린다, `stage/select/list.rs:50-65,780`).
- 할 일
    1. `SongBar` 에 어시스트 등급(0~4). 플레이어가 최고 기록의 등급으로 채운다.
    2. `songlist` 확장 키 `lamp_graded`(등급 1~4 의 그림 4개). 램프가 3 이고 등급이 1 이상이면 그 그림을 그린다. 키가 없는 스킨은 지금과 같다.
    3. 기본 스킨 목록 막대가 `theme.grade_lamp` 색으로 그린다. MY RECORDS 줄과 랭킹 줄의 램프 색도 등급 색(값이 이미 오면 스킨만, 없으면 확장 숫자를 더하고 보고).
- 완료 기준: 등급 0~4 막대가 서로 다른 색인 렌더 시험, `skin-dump --strict` 통과, 외부 스킨(키 없음) 회귀 없음.

#### U5 판정 표시 위치 (요구 10)

- 소유: 원인에 따라 `crates/rbms-render/src/skin_render/{judge.rs,draw.rs,text/**}` 또는 `assets/skins/rbms-default/lua/play/field.lua` 의 판정 부분. U4 의 `songlist` 파일은 건드리지 않는다.
- 할 일
    1. 재현: 플레이 문서를 1280x720·1920x1080·2560x1440·3840x2160 과 16:10 창(레터박스 켬·끔)으로 그려, 판정 글자·콤보 숫자·위 줄(목표 차이·FAST/SLOW)의 사각형을 문서 좌표로 환산해 비교한다. 판정선·레인 기준 상대 위치도 잰다.
    2. 원인을 찾아 고친다. 문서 좌표 기준으로 0.5 문서 화소 안에서 같아야 한다.
    3. 회귀 시험: 해상도별 환산 좌표가 같다는 시험.
- 완료 기준: 고치기 전·후 측정표. 재현되지 않으면 측정표와 의심 지점을 보고하고 코드는 바꾸지 않는다.

### 웨이브 2 (병렬 4, 웨이브 1 푸시 뒤)

#### U6 플레이 화면: 출발 폴더·옵션·NPS (요구 2·3·11)

- 소유: `apps/rbms-player/src/skin_host/**`, `apps/rbms-player/src/stage/play/**`(단 `skin_preview.rs` 는 U7 과 병합 주의), `crates/rbms-skin/src/property/extension.rs`, `assets/skins/rbms-default/lua/{props.lua,play/**,i18n/play.lua}`.
- 확장 id(이 단위만 배정): 텍스트 21010 `PLAY_ORIGIN`, 21011 `PLAY_GAUGE`, 21012 `PLAY_ARRANGE_1P`, 21013 `PLAY_ARRANGE_2P`, 21014 `PLAY_ASSIST`. 숫자 21085 `PLAY_NPS`, 21086 `PLAY_GAUGE_KIND`(게이지 색을 고를 때만). 스킨 옵션 916 `NPS_ON`, 917 `NPS_OFF`.
- 할 일
    1. 출발 폴더(M6): 판을 시작할 때 선곡 경로(`skin_host/select/shown.rs:253` 의 `directory` 제목 목록, 표 폴더는 `stage::decide::table_of_the_open_folder`)의 마지막 두 마디를 ` / ` 로 이어 판과 함께 둔다(재시도 유지). 선곡 화면 밖에서 시작한 판과 코스는 빈 값.
    2. 옵션(M7): 판의 실제 게이지 이름(판 도중 바뀌면 따라감), 사이드별 배치 이름, 어시스트 옵션 요약(SCRATCH AUTO, LN MODE 가 기본이 아닐 때, 판정 확장, CONSTANT, LEGACY NOTE 등 켜진 것만). 용어는 옵션 패널과 같은 이름.
    3. 스킨: 머리 둘째 줄 오른쪽에 출발 폴더(SHRINK, 머리 너비의 40% 까지), 필드 아래 줄에 `게이지 · 배치 · 어시스트`, AUTOPLAY/REPLAY 는 같은 줄 오른쪽. DP 는 사이드마다 자기 배치.
    4. NPS(M17): 직전 1,000 ms 동안 판정선을 지난 노트(일반 노트와 롱노트 시작, 지뢰·보이지 않는 노트 제외) 수. 판정 위 줄에서 FAST/SLOW(또는 ms) 자리 오른쪽. 스킨 옵션 `NPS Display`.
    5. `PlaySample`(미리보기)에서도 값이 온다.
- 완료 기준: 값 시험(빌린 배치, 코스, GAUGE AUTO SHIFT, DP), NPS 계산 시험, `skin-dump --strict --frames 120` 세 언어, 플레이 캡처.

#### U7 LAYOUT PICK: 실시간 미리보기와 capture (요구 8·9)

- 소유: `apps/rbms-player/src/stage/{layout_pick.rs,layout_pick/**}`, 새 `apps/rbms-player/src/captured_layouts.rs`, `apps/rbms-player/src/stage/result.rs` 와 `stage/result/**` 의 단축키 부분, `apps/rbms-player/src/borrowed_layout.rs`, `apps/rbms-player/src/text/menus/layout_pick.rs`, `assets/skins/rbms-default/lua/result/**`, `lua/i18n/result.lua`.
- 병합 주의: `keyconfig.rs`(`Shortcut::CaptureLayout` 한 개 추가, U3 병합본 위에서), `text/notices.rs`, `lib.rs`(`AppShared` 필드), `stage/play/skin_preview.rs`.
- 할 일
    1. 미리보기(M13): 차트 시간으로 흐르는 미리보기. 사용자의 속도·커버 설정으로 그린다. `PlaySample` 을 임의 차트로 일반화해 플레이 스킨으로 그리는 길을 먼저 검토하고, 화면 일부 영역에 그릴 수 없거나 구조 변경이 300줄을 넘으면 내장 플레이필드의 실시간 스크롤로 한다. 고른 길과 이유를 보고한다.
    2. 조작: Space 재생/정지, Page Up/Down 네 마디, Home 처음. 안내 줄 갱신.
    3. 저장소 `captured_layouts.ron`(M15): 읽기·쓰기(`write_atomic`), 같은 (md5, 옵션, 시드)는 한 번만.
    4. 결과 화면 `Shortcut::CaptureLayout`(기본 `C`, 패드 가능). 저장 대상이 아니면(S-RANDOM 등, 오토플레이, 코스) 이유를 알린다. 결과 스킨 바닥에 키 안내.
    5. LAYOUT PICK 의 CAPTURED 목록: 이 차트의 저장분 먼저, 전환 키로 전체 보기. 고르면 그 배치가 미리보기에 나오고 START 로 그 시드로 플레이(`LayoutRequest`). Delete 두 번으로 삭제. 다른 차트의 항목은 라이브러리에 있으면 그 차트로 옮겨 가고 없으면 알린다.
    6. 문구 세 언어.
- 완료 기준: 저장·중복·삭제·손상 파일 시험, capture → LAYOUT PICK → 시작이 같은 시드로 이어지는 시험, 미리보기 시계 시험(재생·정지·탐색·끝에서 처음).

#### U8 선곡 화면 개편 (요구 12)

- 소유: `assets/skins/rbms-default/lua/select/**`, `lua/i18n/select_*.lua`, `lua/theme.lua` 의 선곡 토큰, `assets/skins/rbms-default/tools/sections/select.py` 와 그 결과 시트.
- 병합 주의: `lua/theme.lua`·`lua/props.lua`(U6).
- 설계 지침(M18, 조사 12절의 문제 1~4)
    1. 머리: 한 띠로. 왼쪽 화면 이름과 경로(목록 위에 있던 것), 가운데 검색(비었을 때는 작게), 오른쪽 MODE·SORT·LN MODE 를 같은 모양의 칩으로, 끝에 시계. 띠 아래 선은 화면 전체 너비.
    2. 제목: 글자를 heading 크기로 줄이고(SHRINK 유지), LEVEL·BPM 을 제목 블록 안으로 붙인다. 줄어든 높이는 아래 정보창에 준다.
    3. 바닥: 한 줄. 키 표지 + 짧은 이름(ENTER, ESC, 이동, H). 문장형 둘째 줄은 없앤다. 오른쪽 버튼과 높이를 맞춘다.
    4. 정보창: 구획 머리(캡션 + 구획 너비 선)를 한 부품으로, 라벨 위·값 아래 칸 정렬 통일, 겹침 제거, 간격 24·바깥 여백 64 유지.
    5. 지킬 것: 모노톤 바탕과 강조색 규칙, 넘침 규칙, 세 언어, 외부 값 추가 없음.
- 완료 기준: 개편 전·후 캡처, `skin-dump --strict --frames 120` 세 언어, 시트 검사. 메인이 캡처를 보고 판정한다.

#### U9 사용자 오프셋 단위 (요구 10 의 원인 수리, M19)

- 소유: `apps/rbms-player/src/skin_screen.rs` 의 `DocumentOffsets`, `apps/rbms-player/src/stage/capture/play_parts/judge_sizes.rs`, 오프셋 단위를 말하는 스킨 메뉴·설정 문구.
- 할 일
    1. `DocumentOffsets::offset` 이 저장값을 문서 단위 그대로 돌려준다(`authored/target` 환산 제거). `OFFSET_ALL` 과 각도·알파는 지금대로.
    2. 화면 화소 이동을 고정한 기존 시험(`the_default_pack_moves_its_judgement_by_its_position_and_hides_it_when_told_to` 등)을 문서 단위 기준으로 고친다.
    3. `judge_sizes` 회귀 시험에 JUDGE POSITION 30 변형을 더한다(모든 크기에서 같은 문서 좌표).
    4. 외부 스킨(문서 크기가 1920x1080 이 아닌 것)에서도 같은 규칙인지 시험한다.
- 완료 기준: JUDGE POSITION 30 이 모든 해상도에서 30 문서 화소 이동. 스킨 미리보기(SKIN MENU)의 오프셋 조작이 같은 값으로 보인다.

### 테스트 웨이브와 함께 (추가 요청)

#### U10 램프 반 쪼개기 (요구 13, M20)

사용자 요청 원문(2026-11-19): "추가기능으로 램프에있어서, Assist와 assist가 없는 클리어램프가 2개 있는경우 클리어램프를 반으로 쪼개서 두 개 다 표시가능하도록해".

- 배경: 최고 램프는 `(램프 id, 등급)` 의 최댓값 하나다. 어시스트 없는 EASY CLEAR(id 4)가 생기면 LIGHT ASSIST(id 3) 안의 등급(ASSIST HARD 등)은 최고 기록 행에서 사라진다(`crates/rbms-store/src/scoredb.rs` 의 `merge_best`, `chart_best_many` 의 `MAX(clear * 5 + assist_grade)`).
- 소유: `crates/rbms-store/**`(최고 기록 행에 "지금까지의 최고 어시스트 등급" 열, 소급), 플레이어의 최고 램프 조회와 막대 채우기(`apps/rbms-player/src/stage/select/{list.rs,scene.rs}`, 점수 캐시), `crates/rbms-render/src/skin_render/songlist*`(반으로 나눠 그리기), `assets/skins/rbms-default/lua/select/{list.lua,info.lua}`, 내장 선곡 막대의 LED.
- 할 일
    1. 저장: `score` 행에 최고 어시스트 등급(0~4) 열을 더한다(기존 열 추가 방식 `ADDED_COLUMNS`, 스키마 버전·규칙 세대는 그대로). 판을 저장할 때 갱신하고, DB 를 열 때 `scorelog` 에서 소급한다(멱등, `refold_assist_grades` 와 같은 방식). `scores.ron` 대체 저장소에도 같은 값(`#[serde(default)]`).
    2. 규칙(M20): 쪼개기 조건은 "최고 램프가 EASY·CLEAR·HARD(id 4~6)이고 최고 어시스트 등급 + 3 이 그 id 보다 큼". 순수 함수 하나로 두고 표로 시험한다.
    3. 렌더: `SongBar` 에 쪼갤 어시스트 등급(0 이면 안 쪼갬). 값이 있고 `lamp_graded` 의 그 등급 그림이 있으면 램프 칸의 위 절반에 램프 그림의 위 절반, 아래 절반에 등급 그림의 아래 절반을 그린다. `lamp_graded` 가 없는 스킨은 한 화소도 달라지지 않는다.
    4. 정보창 MY SCORE 의 램프 이름 줄: 쪼개진 차트는 어시스트 등급 이름을 함께 보인다(확장 불린 21160~21163 = 쪼개진 등급 1~4).
    5. 내장(스킨 없는) 선곡 막대의 LED 도 같은 규칙으로 반씩 칠한다.
    6. 서버 전송·기록 동기화·IR 본문은 바꾸지 않는다.
- 완료 기준: 조건 표 시험, 저장·소급 시험(EASY 정식 + ASSIST HARD 판이 있는 DB), 렌더 화소 시험(위·아래 색), 스킨 덤프, 캡처.

## 3. 웨이브와 게이트

1. 웨이브 1: U1·U2·U3·U4·U5 → 메인 병합 → 게이트 → 메인 검토 → 커밋·푸시 → CI.
2. 웨이브 2: U6·U7·U8 → 같은 순서.
3. 테스트 웨이브: 단위별 수용 기준 검증(Sonnet xhigh)과 위험 단위의 반례 찾기·수리(Opus high) → 메인 병합·게이트.
4. 문서: ADR, play-options·skin·crates, QA 실기 점검표, 테스트 부채, 조사 문서 갱신, HANDOFF.

게이트(HANDOFF 7절): `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`, `cargo test --workspace --no-fail-fast --locked`, 스킨 덤프, 시트 검사, 설정 계약(`RBMS_UPDATE_CONTRACT=1 cargo test --locked -p rbms-player --lib a_settings_upload` 뒤 `cd web && bun test tests/dto/client-requests.test.ts`).

## 4. 모든 단위 공통 규칙

- 커밋·푸시·브랜치 조작 금지. 변경은 작업 폴더에 둔다.
- 소유 파일 밖은 고치지 않는다. 필요하면 고치지 말고 보고한다. "병합 주의" 파일은 자기 줄만 더한다.
- Rust: `//` 주석 금지(`///`·`//!` 영어), 매직넘버·새 `#[allow]`·`unsafe`·`todo!()`·새 의존성 금지(U1 의 `coreaudio-rs` 만 예외), rustfmt 160, `--locked`. 설정 구조체는 `#[serde(default)]`, `skip_serializing_if` 금지.
- 새 사용자 표시 문구는 세 언어(앱 `translations!`, 스킨 `lua/i18n/*.lua`), 용어집 `docs/reference/2026-10-24-i18n-glossary.md`.
- 설정 계약 파일(`contract/**`)과 README 캡처는 만들지 않는다. 메인이 병합 뒤에 만든다.
- `rbms-player` 를 실행하면 스크래치 HOME 과 `--server http://127.0.0.1:9`.
- 끝나면 자기 단위의 좁은 검사(`cargo test -p <크레이트> <필터>`, `cargo clippy -p <크레이트>`, `cargo fmt --check`)를 돌리고 실제 출력을 보고한다. 돌리지 못한 검사는 그렇게 적는다.
- 보고: 바꾼 파일, 결정한 것, 계획과 달라진 점, 돌린 검사와 결과, 남은 위험, 갱신이 필요한 문서 절.

## 5. 위험

- U1: 장치 지연을 더하면 예약 앞섬(`lookahead_us`)이 커진다. BGM 예약 창이 그 앞섬을 감당하는지 시험으로 확인한다. 지연을 보정하던 사용자의 JUDGE TIMING 이 이중 보정이 된다(알림으로 안내).
- U2: macOS 독점 전체 화면에서 Game Mode 가 켜지는지는 확인하지 못했다. 문서에는 BORDERLESS 를 Game Mode 경로로 적는다.
- U3: 조합과 기존 START·SELECT + 레인 조합의 우선순위. 명시적으로 묶은 조합이 우선한다.
- U5: 원인 미상.
- U7: `PlaySample` 일반화가 PLAY MENU·SKIN MENU 미리보기를 깨뜨릴 수 있다. 그 시험들이 게이트다.
- 병합: 설정 파일들(U1·U2), `keyconfig.rs`(U3 → U7), `lua/props.lua`·`theme.lua`(U6·U8).

## 6. 검토 기록 (2026-11-19, 메인)

계획을 코드와 대조해 고친 점.

1. M3: `--locked` 게이트 때문에 `Cargo.lock` 이 한 줄 바뀐다는 점을 적었다(새 크레이트 없음).
2. U3: 키보드 쪽 Esc 치환 위치를 공통 래퍼 `Stage::handle_key` 로 정했다. 화면마다 `control_for` 를 보는 방식이면 빠지는 화면이 생긴다.
3. U4: 등급은 플레이어에 이미 있다(`RecordLamp`). 버려지는 곳이 `stage/select/scene.rs:90` 한 줄이라 소유 파일을 그쪽으로 고쳤다.
4. U6: 경로는 제목 목록(`directory`)이라 "마지막 두 마디"를 문자열 자르기 없이 만들 수 있다. 코스는 스테이지 표지가 이미 있어 빈 값으로 둔다.
5. U7: 결과 화면에서 `C` 를 읽는 코드는 없다(시험 상수 하나뿐). 기본 키 `C` 유지. (정정 2026-11-19: 틀린 검토였다. 결과 화면은 레인 키를 키 표대로 읽고 `C` 는 5번째 레인 키다. 그 시험 상수가 바로 그 동작을 고정한 것이었다. 구현 단위가 찾아 `P` 로 바꿨다.)
6. U1 방향 확인: `audible_us_from`(`engine.rs:710`)은 `callback_at + playback_ahead` 를 기준으로 삼으므로 `playback_ahead` 가 커지면 같은 순간의 가청 위치가 그만큼 앞당겨진다(늦게 들리는 것과 같은 방향). 노트 예약 위치(프레임 축)는 그대로이고 예약 앞섬만 커진다.
7. 디스크: 여유 516 GB, 기존 `target` 173 GB. worktree 5개 병렬 빌드 가능.
8. 남긴 위험: U5 원인 미상, U7 의 `PlaySample` 일반화 범위, macOS 독점 전체 화면에서의 Game Mode.

## 7. 웨이브 1 결과 (2026-11-19, wf_70a42bf1-788)

| 단위 | 결과 | 계획과 달라진 점 |
| --- | --- | --- |
| U1 | 완료. 이 Mac 실측 AirPods Pro 2 7,680 프레임(160 ms)·스피커 108 프레임 | 지연은 콜백이 아니라 시계 기록을 읽는 쪽(`snapshot_of`)에서 더한다. 판 시작 기준점을 `가청 위치 + 보정값`(`AudioClocks::run_anchor_us`)으로 옮겼다. 그대로 두면 차트 첫 160 ms 의 소리가 이미 지난 프레임으로 예약된다 |
| U2 | 완료. 이 Mac 에서 세 모드와 모든 전환 확인 | FULLSCREEN 은 일반 창으로 열고 첫 동기화에서 들어간다. BORDERLESS → FULLSCREEN 도 일반 창을 거친다. macOS winit 이 BORDERLESS 에서 바로 독점으로 가면 창이 옛 모드 틀에 남는다 |
| U3 | 완료 | 레인은 PLAY 화면에서만 조합 완성 누름에도 판정으로 가고, 다른 화면에서는 조합의 것이다(선곡에서 레인 키가 다른 뜻으로 새지 않게). 메인 수정: 패드 표식을 큐가 아니라 이벤트와 함께 넘긴다(`Stage::handle_marked_pad`), 설정 화면·옵션 오버레이의 키 안내가 조합을 표기한다 |
| U4 | 완료 | MY RECORDS·랭킹 줄에 등급 값이 없어 확장 숫자 21087~21092(기록 6줄)·21093~21102(랭킹 10칸)를 더했다 |
| U5 | 원인 확인, 수리는 U9 로 | 판정 글자·콤보·목표 차이는 해상도와 무관했다(0.5 문서 화소 안). 달라지는 것은 사용자 오프셋(M19)이고, FAST/SLOW 글자는 장치 화소 맞춤으로 1.5 문서 화소까지 흔들린다(ADR 2026-10-22 가 받아들인 절충, 그대로 둔다). 메인 수정: 회귀 시험에서 3840x2160 과 1600x900 을 뺐다(시험 시간) |

계획서 U3 3항의 "단일 조작·단축키를 누른다"는 오타이고 M12 대로 "실행하지 않는다"가 맞다.

## 8. 웨이브 2 결과 (2026-11-19, wf_805372b8-c60)

| 단위 | 결과 | 계획과 달라진 점 |
| --- | --- | --- |
| U6 | 완료. 캡처로 7키·넓은 레인·9키·14키·코스 확인 | 숫자 21086(게이지 종류)은 만들지 않았다. 플레이 화면의 인덱스 40 이 이미 판의 실제 게이지를 답한다. NPS 는 DP 에서 첫 사이드에만 그린다. LN MODE 는 그 모드가 차트의 롱노트를 정할 때만 어시스트 줄에 쓴다. 새 번역 문구 없음(게임 용어) |
| U7 | 완료 | 기본 키 `P`(M14 정정). 미리보기는 내장 플레이필드의 실시간 스크롤이다. `PlaySample` 은 화면 전체를 사각형에 줄여 그려 레인이 좁고, 공용 스킨 타이머에 묶여 정지·탐색이 없다. 저장 대상에 ROTATE 추가(레인 순열), OFF 는 거절. 목록 조작: Right/Left 로 입력란과 목록 이동, Tab 범위 전환, Delete 두 번 삭제, SELECT 는 1P → 2P → 목록 순환 |
| U8 | 완료. 메인이 캡처로 확인 | 머리 아래 선은 원래 전체 너비였고, 오른쪽 카드의 아랫변이 겹쳐 이중선으로 보인 것이 원인이었다. LN MODE 칩 값은 LN·CN·HCN 약어. 검색 강조는 경로가 `Search : '` 로 시작하는지로 판단한다(스킨이 읽을 입력 중 값이 없다) |
| U9 | 완료. JUDGE POSITION 30 이 모든 크기에서 30 문서 화소(고치기 전 1280x720 45, 2560x1440 22.5) | `DocumentOffsets::drawn_at` 을 `new` 로 바꾸고 `skin_offsets` 의 대상 크기 인자를 없앴다 |

병합: U6 과 U9 가 `judge_sizes.rs` 한 줄에서 겹쳐 메인이 손으로 합쳤다.

## 9. 테스트 웨이브 결과와 사용자 결정 (2026-11-19, wf_3df278ff-bb7)

첫 실행(wf_45702168-567)은 에이전트 9개가 모두 5시간 사용량 한도로 시작 직후 끊겼다. 추가 요청 U10 을 더해 다시 실행했다. 보고 원문은 [history/2026-11-19-feature-upgrade-unit-reports/](../history/2026-11-19-feature-upgrade-unit-reports/README.md) 의 `test-*.md`.

| 담당 | 모델 | 고친 것 |
| --- | --- | --- |
| O1 오디오 | Opus | 보정 판의 첫 구간에서 멈춘 곡 위치로 예약해 같은 키의 앞 소리가 끊기던 문제(high) |
| O2 입력 | Opus | 조합이 풀린 뒤 남은 키의 재눌림, ESC 에 Enter·방향키·H·다른 모드의 레인 키를 줄 수 있던 것, 글자 입력 중 조합, 패드 분리 중 포착, 떠는 시험 하나 |
| O3 배치 | Opus | 목록에서 레인 키 두 번 삭제, 시드만 다른 같은 배치의 중복 저장(MIRROR·ROTATE·R-RANDOM), 읽지 못한 저장 파일을 덮어쓰던 것 |
| O4 플레이 화면 | Opus | 메뉴 미리보기의 어시스트 줄 누락 |
| U10 | Opus | 램프 반 쪼개기 구현, U4 기준 검증과 스킨 덤프 시나리오 공백 메움 |
| S1 창·오프셋 | Sonnet | 세로 모니터의 독점 모드 고르기, 폴백 판정 순수 함수화 |
| S3 선곡 | Sonnet | 제목·부제 간격, 긴 경로의 현재 폴더, MISS 다섯 자리, MAX COMBO 라벨 |
| S4 문구 | Sonnet | 88자에 잘리던 알림 둘, 내장 결과 화면의 capture 안내, LAYOUT PICK 목록 안내, 조사 띄어쓰기 |
| DOC | Sonnet | ADR 9건과 문서 |

게이트: fmt·clippy 통과, Rust 7,322 통과·0 실패·6 무시, 스킨 덤프·시트 검사 통과. 커밋 `4874930`~`fe9d387` 과 문서.

사용자 결정(2026-11-19, 메인이 열린 세 건을 보고한 뒤의 원문: "1. 감지하는 로직을 짜야하나 ? 2. 그럼 수리지 3. 이것도 그럼 지금 수정인가 ? 남은것도 다 잘 처리하고 다 잘 처리해놔"):

| 번호 | 열린 건 | 처리 |
| --- | --- | --- |
| 1 | 기본 출력 장치가 실행 중 바뀌면 처음 연 장치의 지연을 읽는다(의심) | 상시 감지는 만들지 않는다. 시스템 기본으로 연 스트림은 판을 시작할 때마다 지금의 기본 출력 장치에게 지연을 묻는다(메인 답, 수리 F1) |
| 2 | 스킨 결과 화면이 조합으로 묶은 RETRY·NEXT 를 읽지 못한다 | 수리(F2) |
| 3 | 빌린 배치로 친 판을 재시도하면 사용자 배치 옵션으로 기록된다(이번 작업 이전부터) | 지금 수리(F3) |

메인 결정 M21(사용자 확인 전): 출력 지연 알림은 (지연을 읽은 장치, 보정 ms)가 마지막으로 알린 것과 다르고 30 ms 이상일 때만 낸다. 같은 장치로 스트림을 다시 열 때는 다시 뜨지 않고, 판 시작 전 재조회에서 장치가 바뀌었으면 뜬다.

수리 웨이브(wf_08935ad2-8ca): F1 오디오 후속(기본 장치 따라가기, 장치 샘플레이트 환산, M21, Practice 화면의 스트림 다시 열기 차단, 판 시작 직후 소리의 늦은 예약), F2 입력·결과·배치 후속, F3 빌린 배치 재시도 기록, F4 스킨·문구 다듬기(Sonnet).

## 10. 수리 웨이브 결과 (2026-11-19, wf_08935ad2-8ca)

| 담당 | 모델 | 결과 |
| --- | --- | --- |
| F1 오디오 | Opus | 기본 출력 장치 따라가기, 장치 샘플레이트 환산, 알림 규칙(M21), 디버그 줄의 장치 이름, 중복 조회 제거, PRACTICE 화면의 스트림 다시 열기 차단, 판 기준점을 예약 축으로(늦은 소리 5/286 → 0, 판 시작 정지 +38 ms), 조회 1초 시한. 미룸: 이름으로 고른 같은 이름의 장치 둘 |
| F2 입력 | Opus | 스킨 결과 화면과 LAYOUT PICK 의 조합 인식, capture 쓰기 실패 알림, 빈 목록 안내, 턴테이블 축의 조합 거절, 설정 동기화 때 눌린 키 유지, 결과 화면 단축키와 레인 키 겹침 안내, 모든 조작 키가 레인에 양보(정책 변경) |
| F3 기록 | Opus | 빌린 배치 판의 재시도 기록 수리. 다른 기록 입구는 문제없음을 확인. 미룸: 이미 잘못 기록된 행 |
| F4 다듬기 | Sonnet | 게이지 표기 `EX-HARD`, 사이드 패널 격자, 쓰이지 않는 토큰, CLEAR COUNT 겹침, 옵션 줄 둘째 줄. 미룸: 검색 입력의 누르는 영역(엔진 제약), 일본어 공백 |

메인 수정: 알림 상태를 스레드 전역 변수에서 `AppShared::output_latency_noticed` 로 옮겼다.

게이트: fmt·clippy 통과, Rust 7,347 통과·0 실패·6 무시, 스킨 덤프·시트 검사 통과. 커밋 `864ef42`·`4d1ce8e`·`733ba62`·`c29c407`.

지금 R-BMS 대비 바뀐 점(사용자 확인 전, 미확인 49·50): 판 시작 직후 정지가 약 38 ms 길어지고 첫 소리가 제자리에 온다. 레인 키와 같은 키의 조작은 그 모드에서 듣지 않는다.

남은 것(테스트 부채 3.182~3.187, 실기 점검 QA 3.29): 기본 장치 따라가기 실기, 판 시작 체감, GPU 에서의 램프 반쪽, 같은 이름의 장치, 옛 잘못 기록된 행, 검색 입력의 누르는 영역, 웹의 `EX HARD` 표기.

## 11. 램프 반 쪼개기 조건 개정과 릴리스 (2026-11-19)

- v0.1.12 draft: release run 44529321802. 첫 시도는 macOS 잡이 공증 계정 확인에서 Apple `403`(약관 동의 필요)으로 실패했고, 사용자가 동의한 뒤 실패한 잡만 다시 실행해 만들었다. 메인이 산출물을 내려받아 점검하고 노트를 한글로 바꿨다([ci-release.md](../ci-release.md) 2.3절).
- 사용자 보고(원문): "흠 어시스트 / 일반플레이 램프가 나눠서 표시안되는느낌인데". 사용자 점수 DB 의 사본으로 확인: 저장·소급은 정상, 둘 다 있는 차트 68개 중 M20 의 처음 조건(어시스트 쪽이 더 높은 게이지)에 맞는 것이 3개였다.
- 수정: 조건을 "어시스트 없는 클리어(EASY 이상)와 등급 있는 어시스트 클리어가 함께 있으면 쪼갬"으로 넓혔다(`rbms_store::split_assist_grade`, 시험 셋 갱신). 메인이 직접 고쳤다(함수 하나). [ADR](../acknowledge/2026-11-19-adr-split-clear-lamp.md) 개정 절.
- v0.1.12 는 처음 조건으로 빌드된 채 사용자가 게시했다(2026-11-19 13:40 UTC). 넓힌 조건은 v0.1.13 draft 에 있다(release run 44547568815, 버전 커밋 `cd82edb`, 메인이 산출물 점검과 노트 교체 완료, 게시는 사용자).

