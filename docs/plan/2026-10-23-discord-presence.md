# Discord 상태 표시(Rich Presence) 설계서

> 최종 갱신 2026-10-23 · 대응 커밋 92a0caa(dev) · 작성 2026-10-23 · 상태: 설계(구현 전). PROCESS 체크리스트의 D 항목 몫이다.
> 사용자 요청(2026-10-23): "discord에 게임으로 인식되게하는 기능", 답 "좋아 1번대로 하도록하고 Application 만들어서 아이디는 1563156237539999865 / 아이콘이나 기타 꼼꼼하게 잘 할수있지 ?".
> 메인이 정한 것(PROCESS D): Rich Presence(로컬 IPC, 새 의존성 없음), v0.1.8 draft 전에 넣음, 표시는 곡 제목·난이도·모드·코스 이름·경과 시간, 설정 DISCORD STATUS 기본 켬, Application ID `1563156237539999865` 는 비밀이 아니므로 코드 상수.
> 이 문서는 결정을 바꾸지 않는다. 결정이 다루지 않는 세부는 9절 "메인 결정 필요" 에 추천안과 대안을 적었다.

## 1. 범위

| 들어감 | 빠짐 |
| --- | --- |
| Discord 데스크톱 앱이 켜진 기기에서 R-BMS 를 "R-BMS 하는 중" 으로 보이기, 화면별 문구, 경과 시간, 큰 그림(아이콘) | Discord Social SDK·GameSDK(네이티브 라이브러리, 새 의존성), OAuth 인증, 친구 초대·참가(Join/Spectate), 파티, 버튼(9절 M7) |
| 설정 DISCORD STATUS(기본 켬)와 옛 설정 파일 호환 | 웹·서버·DB·마이그레이션 변경(없음) |
| macOS·Linux 유닉스 소켓, Windows 이름 있는 파이프 | Discord 가 없는 기기에서 알림·토스트(조용히 넘어간다) |
| 끊김·Discord 없음에서 간격을 두고 다시 붙기 | 판정·점수·기록 저장 규칙(건드리지 않는다) |

새 의존성 없음: 소켓은 `std::os::unix::net::UnixStream`, Windows 파이프는 `std::fs::OpenOptions`, JSON 은 이미 쓰는 `serde_json`(작업공간 의존성, `rbms-player` 에는 아직 직접 의존이 없으므로 `Cargo.toml` 에 `serde_json = { workspace = true }` 한 줄을 더한다. 새 크레이트나 새 판은 들어오지 않지만 `Cargo.lock` 의 `rbms-player` 의존 목록에 `serde_json` 한 줄이 생기므로, 그 한 줄을 반영한 잠금 파일을 같은 커밋에 넣어야 `--locked` 가 통과한다. 메인이 diff 가 그 한 줄뿐인지 확인한다). `discord-rpc` 코드는 옮기지 않고 프로토콜만 따르므로 THIRD-PARTY-NOTICES 변경 없음.

## 2. 공식 문서 확인(2026-10-23)

### 2.1 출처

| 기호 | 출처 | 확인한 것 |
| --- | --- | --- |
| S1 | https://docs.discord.com/developers/topics/rpc (옛 주소 `discord.com/developers/docs/topics/rpc` 는 이리로 301) | IPC 전송, opcode, 핸드셰이크, SET_ACTIVITY 인자, activity 의 type 제한, 소켓 경로 |
| S2 | https://docs.discord.com/developers/events/gateway-events (Activity Object) | activity 필드, Activity Asset Image(외부 URL), 버튼 길이, 상태 갱신 5회/20초, timestamps 단위(ms) |
| S3 | https://docs.discord.com/developers/discord-social-sdk/development-guides/setting-rich-presence | Art Assets 1024x1024 권장, 키 소문자화, 자산 300개, timestamps "Unix timestamps in seconds", 데스크톱 앱이 켜져 있어야 함 |
| S4 | https://discord.com/developers/docs/social-sdk/classdiscordpp_1_1Activity.html, `..._1_1ActivityAssets.html`, `..._1_1ActivityTimestamps.html` | details·state 2~128자, LargeImage·SmallImage 1~300자(키 또는 외부 URL), LargeText·SmallText 2~128자, Timestamps 는 ms(SDK 가 작은 값을 초로 보고 바꿈) |
| S5 | https://docs.discord.com/developers/rich-presence/best-practices | 이미지 1024x1024 이상, details·state 는 문장 아닌 짧은 조각, 한 줄 유지 |
| S6 | https://docs.discord.com/developers/topics/opcodes-and-status-codes | RPC Close 코드 4000~4005, RPC Error 코드 |
| S7 | https://github.com/discord/discord-rpc (master, 마지막 푸시 2024-06-13, README 에 "deprecated in favor of GameSDK") `src/connection_unix.cpp`, `src/connection_win.cpp`, `src/rpc_connection.{h,cpp}`, `src/serialization.cpp`, `src/discord_rpc.cpp`, `include/discord_rpc.h`, `documentation/hard-mode.md` | 프레이밍, 핸드셰이크·READY, PING→PONG, CLOSE, 경로 탐색 순서, 재연결 0.5초~60초, 필드 바이트 상한, 프레임 64 KiB |
| S8 | discord-api-docs 의 옛 `docs/rich_presence/How_To.md`(커밋 `3746b0d74de7855df098f0aac523fcf9ed705216`, 지금은 지워짐) | `Discord_UpdatePresence` 는 15초에 1번, 그 사이 것은 줄 세워 가장 새 것만 보냄, timestamps 는 epoch 초, 자산 키 자동 소문자 |

### 2.2 확인한 사실

| 항목 | 값 | 출처 |
| --- | --- | --- |
| 프레임 | `[opcode u32 LE][length u32 LE][JSON UTF-8 length 바이트]`. 한 번의 쓰기로 보내야 한다(나눠 쓰면 파이프가 깨진다) | S7 hard-mode "Notes", `rpc_connection.h` |
| opcode | 0 HANDSHAKE, 1 FRAME, 2 CLOSE, 3 PING, 4 PONG | S1, S7 |
| 핸드셰이크 | opcode 0 에 `{"v":1,"client_id":"<id>"}`. 성공하면 FRAME 으로 `{"cmd":"DISPATCH","evt":"READY",...}` | S1, S7 `rpc_connection.cpp` |
| 활동 설정 | FRAME 에 `{"cmd":"SET_ACTIVITY","args":{"pid":<프로세스 id>,"activity":{...}},"nonce":"<문자열>"}`. 지우기는 `activity` 를 빼고 보낸다(`Discord_ClearPresence` = 활동 없는 SET_ACTIVITY) | S1, S7 `serialization.cpp`, `discord_rpc.cpp` |
| 응답 | 보낸 명령마다 같은 nonce 로 응답이 온다. 실패면 `evt":"ERROR"` 와 `data.code`·`data.message` | S7 hard-mode, `discord_rpc.cpp` |
| PING | 받으면 같은 본문을 opcode 4 로 돌려준다 | S7 `rpc_connection.cpp` |
| CLOSE | 받으면 본문의 `code`·`message` 를 읽고 연결을 닫는다. Close 코드 4000 Invalid client ID, 4001 Invalid origin, 4002 Rate limited, 4003 Token revoked, 4004 Invalid version, 4005 Invalid encoding | S6, S7 |
| 인증 | S1 은 "RPC 명령은 인증해야 한다(4006)" 라고 쓰지만, 공식 discord-rpc 는 IPC 에서 AUTHORIZE·AUTHENTICATE 없이 HANDSHAKE 뒤 바로 SET_ACTIVITY 를 보낸다. S3 도 "no-auth RPC method on desktop" 이라 한다. 그래서 인증하지 않는다 | S1, S3, S7 |
| 유닉스 경로 | `XDG_RUNTIME_DIR` → `TMPDIR` → `TMP` → `TEMP` → `/tmp` 중 처음 정의된 하나 아래 `discord-ipc-0` ~ `discord-ipc-9`, 처음 붙는 것을 쓴다 | S1, S7 `connection_unix.cpp` |
| Windows 경로 | `\\?\pipe\discord-ipc-0` ~ `-9`(S1·S7). hard-mode 는 `\\.\pipe\` 로도 쓴다. 둘은 같은 파이프다. `ERROR_FILE_NOT_FOUND` 면 다음 번호, `ERROR_PIPE_BUSY` 면 기다렸다 다시 | S1, S7 `connection_win.cpp` |
| snap·flatpak | 공식 문서·discord-rpc 에 없음(확인 못함). 커뮤니티 구현들이 `$XDG_RUNTIME_DIR/app/com.discordapp.Discord/` 와 `$XDG_RUNTIME_DIR/snap.discord/` 를 더 찾는다는 것은 알려져 있으나 공식 근거는 없다 | - |
| activity.type | IPC SET_ACTIVITY 의 type 은 Playing 0, Listening 2, Watching 3, Competing 5 만 | S1 |
| name 바꾸기 | Social SDK 는 `SetName` 으로 표시 이름을 덮을 수 있다고 한다. IPC SET_ACTIVITY 의 `name` 이 받아지는지는 확인 못함. 이름은 Portal 의 앱 이름(R-BMS)으로 정하므로 보내지 않는다 | S3 |
| 문자열 길이 | details·state 2~128자, large_text·small_text 2~128자(S4). discord-rpc 헤더는 state·details·largeImageText 128 바이트, 이미지 키 32 바이트(S7·S8). 1자짜리는 거절된다는 것이 S4 의 하한(2) 이다 | S4, S7, S8 |
| 이미지 | `large_image` 는 Art Asset 키 또는 외부 URL(미디어 프록시로 바뀜, S2·S4). URL 은 "알맞은 크기" 여야 한다는 말만 있고 수치는 없다(S3). Art Asset 은 PNG·JPEG·WebP, 1024x1024 권장, 키는 소문자로 바뀐다, 300개까지 | S2, S3, S4 |
| 기본 아이콘 | 그림 값을 주지 않으면 앱의 아이콘과 이름을 쓴다(Social SDK 문서). IPC 에서 그대로인지는 확인 못함 | S4 |
| timestamps | 옛 IPC 문서와 Social SDK 안내는 초(S3·S8), 게이트웨이 Activity Object 와 SDK 클래스 문서는 ms(S2·S4, SDK 는 작은 값을 초로 보고 바꾼다). `start` 만 주면 경과 시간, `end` 면 남은 시간 | S2, S3, S4, S8 |
| 갱신 속도 | 옛 SDK 문서: 15초에 1번, 그 사이 갱신은 줄 세워 가장 새 것만(S8). 게이트웨이: 상태 갱신 20초에 5번(S2). discord-rpc 코드 자체는 속도를 막지 않고 바뀔 때마다 보낸다(S7) → 제한은 Discord 앱 쪽에서 건다고 본다. Close 코드 4002 Rate limited 가 있다 | S2, S6, S7, S8 |
| 버튼 | 2개까지, label 1~32자, url 1~512자, 본인에게는 안 보인다 | S2, S3 |
| 데스크톱 앱 | Discord 데스크톱 앱이 켜져 있어야 한다 | S3 |
| 재연결 | discord-rpc: 0.5초에서 60초까지 늘어나는 간격(무작위 섞음), 성공하면 처음으로 | S7 `discord_rpc.cpp` `Backoff ReconnectTimeMs(500, 60 * 1000)` |
| 프레임 크기 | discord-rpc 버퍼 64 KiB | S7 `MaxRpcFrameSize` |
| Portal 앱 아이콘 최소 크기 | 공식 문서에서 찾지 못함(확인 못함). 비공식 글들이 512x512 를 말한다. 1024x1024 를 올리면 어느 쪽이든 넉넉하다 | - |

## 3. 현재 코드 사실(HEAD 92a0caa, 직접 열어 확인)

| 사실 | 위치 |
| --- | --- |
| 화면은 `Stage` 열거형 13가지(Select, Settings, SkinMenu, PlayMenu, KeyConfig, Tables, Folders, Decide, Loading, Play, Result, CourseResult, Practice), 정체만은 `StageId` | `apps/rbms-player/src/stage/mod.rs:70-104` |
| 모든 화면 전환은 `App::switch` 한 곳: 나가는 `on_exit` → 교체 → 들어오는 `on_enter`. `Open` 은 아래 화면을 `suspended` 에 쌓고 `Back` 이 되살린다 | `apps/rbms-player/src/lib.rs:1292-1321` |
| 프레임은 `App::frame`: 네트워크·IR·오디오 poll 뒤 화면 update·draw | `lib.rs:1346-1400` |
| 앱 종료: `ApplicationHandler::exiting` → `keep_what_quitting_cuts_short` → `finish_saves`. macOS Quit 은 루프 안에서 프로세스를 끝내므로 `Drop`·`run` 에 기댈 수 없다 | `lib.rs:1595-1629` |
| 앱이 뜬 시각 `AppShared::booted: Instant` | `lib.rs:923`, `:1199` |
| 백그라운드 스레드 본보기: `SaveWorker`(`rbms-save`, `mpsc` 채널, `for_this_build()` 가 단위 테스트에서는 스레드 없이 그 자리에서 실행, 종료 때 `flush`), 이름 상수 `GHOST_THREAD_NAME = "rbms-ghost"`, `BACKFILL_THREAD_NAME = "rbms-trace-backfill"`, `PRESENT_THREAD = "rbms-present"`(종료 때 "patience" 안에 안 끝나면 경고만) | `save_worker.rs:95-140`, `ghost.rs:46`, `trace_backfill.rs:78`, `gpu/present.rs:52`, `:288` |
| 플레이 화면: `PlayState.session.model()` 의 `Model { mode, meta: ModelMeta { title, subtitle, play_level, difficulty, .. }, md5 }`, 연습 판이면 `PlayState.practice: Option<PracticeSession>` | `stage/play/mod.rs:219-263`, `crates/rbms-play/src/session.rs:1012`, `crates/rbms-model/src/lib.rs:135-164` |
| 오토플레이: `AppShared::run_plays_itself()`(AUTOPLAY 설정 또는 이번 한 판 `autoplay_once`). 리플레이 보기: `AppShared::replay: Option<Replay>` | `app_play.rs:701-703`, `lib.rs` `AppShared.replay` |
| 일시 정지: 손으로 치는 판에는 없다. 리플레이 분석 재생만 `analysis_paused()` 가 있다 | `stage/play/mod.rs:750-772`, `:1283` |
| 코스: `AppShared::course_run: Option<CourseRun>`, `CourseRun { course: Course { name, charts, .. }, index, failed_at, .. }`, `stage_number()`(1부터), `course.stage_count()`. 스테이지마다 Loading→Play 로 `switch` 를 지난다 | `crates/rbms-course/src/run.rs:133-171`, `model.rs:245` |
| 결과: `ResultState.view: Box<ResultView>` 의 `title`, `mode_label`, `clear_label` | `stage/result.rs:789-801`, `crates/rbms-render/src/result.rs:16-34` |
| 코스 결과: `CourseResultState { cleared, course_name, .. }` | `stage/course_result.rs:90-102` |
| 로딩: `LoadingTask::{Song(usize), Scan{..}, Assets(Box<ChartAssets>)}` — 스캔과 곡 로딩이 같은 화면 | `stage/loading.rs:369-383` |
| 결정 화면: `DecideState { chart: ChartOverview, course: Option<String>, table_name, table_level, .. }`, 표 레벨은 `table_of_the_open_folder`(비공개)가 `innermost_level` + `level_title` 로 만든다(예: `sl5`, 기호 없으면 `LV 5`) | `stage/decide.rs:174-193`, `:308-318`, `select_tree.rs:123-130`, `:246` |
| 이름표: `format::difficulty_name(1..=5)` = BEGINNER·NORMAL·HYPER·ANOTHER·INSANE(그 밖 `—`), `format::mode_short` = 5K·7K·9K·10K·14K(그 밖 `?`) | `apps/rbms-player/src/format.rs:62-72`, `:100-109` |
| 설정 표: `rbms_config::SettingId`(96개, `SETTING_COUNT`), `SettingTab`, `row(id, tab, label, kind, help)`. 화면의 묶음·설명은 `stage/settings/catalog.rs`(`NETWORK_SECTIONS` = SERVER AND ACCOUNT, SCORES) | `crates/rbms-config/src/settings.rs:174-375`, `:933`, `catalog.rs:60-81`, `:249-253` |
| 설정 스키마: `NetworkOptions` 는 `#[serde(default)]`, 토글 본보기 `auto_upload_trace: bool`(기본 true) | `crates/rbms-config/src/schema.rs:796-860` |
| 설정 업로드 계약 `contract/client-requests/settings-blob.json`(RON 본문 통째로), 갱신은 `RBMS_UPDATE_CONTRACT=1 cargo test -p rbms-player --locked contract_requests` | `app_result/record_tests/contract_requests.rs:13-30` |
| 렉 기준: 플레이 중 프레임 스레드에 디스크 I/O·큰 할당·잠금 대기·동기 네트워크 없음 | `docs/acknowledge/2026-10-14-adr-play-hitch-criteria.md` |

## 4. 아이콘

| 파일 | 크기 | 비고 |
| --- | --- | --- |
| `assets/icon/png/icon-1024.png` | 1024x1024 RGBA, 32,896 B | 투명 바탕에 둥근 사각형(가장자리 100 px 여백, `BODY_INSET`) |
| `assets/icon/png/icon-512.png` = `web/src/app/icon.png` | 512x512 RGBA | 같은 그림, 웹에서 `https://bms.hyuns.uk/icon.png` 로 200 응답(2026-10-23 확인) |
| `web/src/app/apple-icon.png` | 180x180 RGB | 여백 없이 꽉 찬 배경(`master(full_bleed=True)`) |

Discord 는 앱 아이콘을 둥글게(프로필·목록에서는 원 또는 둥근 사각형으로) 잘라 보이고, Rich Presence 큰 그림도 둥근 사각형으로 잘린다. 지금의 1024 그림은 투명 여백이 사방 약 10% 라 잘린 틀 안에서 작게, 모서리가 이중으로 보인다. 그래서 **꽉 찬 1024x1024 RGB 를 하나 더 만드는 것을 추천한다**(9절 M1).

만드는 법(지금 만들지 않는다): `assets/icon/tools/generate-icon.py` 에 상수 `DISCORD_ICON_SIZE = 1024` 와 `outputs()` 의 한 줄
`files['png/discord-1024.png'] = png_bytes(master(full_bleed=True).convert('RGB'))` 를 더하고 `uv run assets/icon/tools/generate-icon.py` 로 만든 뒤 `uv run assets/icon/tools/generate-icon.py --check` 로 확인한다(`master` 는 이미 `CANVAS` 1024 로 그리므로 크기 조정이 필요 없다). `assets/icon/README.md` 의 표에 한 줄 더한다.

같은 파일 하나를 Portal 앱 아이콘과 Art Asset 둘 다에 올린다(7절).

## 5. 설계

### 5.1 모듈과 파일

| 파일 | 내용 |
| --- | --- |
| `apps/rbms-player/src/discord_presence.rs` | `DiscordPresence`(프레임 스레드가 쥐는 손잡이), 명령 `Command`, 스레드 띄우기, `AppShared` 연결 메서드 |
| `apps/rbms-player/src/discord_presence/activity.rs` | `Activity` 구조체(직렬화), 화면 → 활동 대응(순수 함수 `activity_for(&PresenceView) -> Option<Activity>`), 문자열 맞추기 `fit_text` |
| `apps/rbms-player/src/discord_presence/ipc.rs` | 프레임 인코딩·디코딩, 핸드셰이크·SET_ACTIVITY·CLEAR JSON, `IpcStream` 트레이트, `Connect` 트레이트, 실제 연결기(유닉스·Windows), 소켓 후보 목록 |
| `apps/rbms-player/src/discord_presence/link.rs` | 스레드 안의 상태 기계 `Link`(연결·재연결 간격·속도 제한·중복 제거), 시계를 인자로 받는 동기 함수 |
| `apps/rbms-player/src/discord_presence/tests.rs`(필요하면 `tests/` 하위로 나눔) | 9절 목록의 시험 |
| `crates/rbms-config/src/schema.rs`, `settings.rs`, `tests.rs` | `NetworkOptions.discord_status`, `SettingId::DiscordStatus` 행 |
| `apps/rbms-player/src/stage/settings/catalog.rs` | NETWORK 의 새 묶음 `DISCORD`, 행 설명 |
| `apps/rbms-player/src/lib.rs` | `AppShared.discord` 필드, `App::switch` 끝·`App::frame`·`App::exiting` 의 세 줄 |
| `apps/rbms-player/Cargo.toml` | `serde_json = { workspace = true }` |

새 크레이트로 빼지 않는다(9절 M10). 이름 규칙: 스레드 `rbms-discord`(`DISCORD_THREAD_NAME`), Application ID 상수 `DISCORD_APPLICATION_ID: &str = "1563156237539999865"`, Art Asset 키 상수 `LARGE_IMAGE_KEY: &str = "rbms"`.

### 5.2 스레드·채널

```
프레임 스레드                                   rbms-discord 스레드
App::switch 끝 ─ PresenceView 만들기 ─┐
App::frame (설정 켬/끔 bool 비교만) ─────┤  mpsc::Sender<Command>  ──>  Link::run
App::exiting ─ Stop ─────────────────────┘                          (연결·핸드셰이크·SET_ACTIVITY·응답 읽기·재연결)
```

- 프레임 스레드는 화면이 바뀔 때(`App::switch` 끝, `on_enter` 뒤)와 설정 켬/끔이 바뀐 프레임에만 `Sender::send` 한다. 같은 활동이면 보내지 않는다(프레임 스레드 쪽 `last: Option<Activity>` 와 비교). 플레이 도중에는 화면이 바뀌지 않으므로 아무것도 보내지 않는다. 매 프레임 하는 일은 `config.network.discord_status` 와 마지막으로 알린 값의 bool 비교 하나다(할당·I/O 없음). 렉 기준 ADR 을 지킨다.
- 활동 문자열 조립은 화면 전환 때 프레임 스레드에서 한다(곡 한 개 분량의 짧은 `String` 몇 개, 로딩 화면 사이라 플레이 프레임에 걸리지 않는다). JSON 직렬화와 모든 소켓 I/O 는 스레드에서.
- 스레드 쪽은 받은 활동 중 가장 새 것만 기억한다(latest-wins). 채널은 무제한 `mpsc::channel`(보내는 쪽이 막히지 않음).
- `Command`: `Show(Option<Activity>)`(None = 지우기), `Enable(bool)`(설정 켬/끔), `Stop`(종료).
- 단위 테스트 빌드에서는 `DiscordPresence::for_this_build()` 가 스레드를 띄우지 않고 받은 명령을 `Vec<Command>` 에 적기만 한다(`SaveWorker::for_this_build` 와 같은 꼴). 그래서 수백 개의 앱 테스트·헤드리스 캡처가 실제 Discord 에 붙지 않는다. 스레드 시험은 `DiscordPresence::spawn_with(connector, clock)` 로 가짜 연결기를 주입한다.
- 스레드를 띄우지 못하면(`Builder::spawn` 실패) 조용히 비활성으로 둔다(알림 없음).

### 5.3 스레드 안의 상태 기계(`Link`)

상태: `Off`(설정 끔 또는 보일 것 없음), `Waiting { retry_at }`, `Connected { stream, sent: Option<Activity> }`.

1. `recv_timeout(다음 할 일까지)` 로 명령을 기다린다. 다음 할 일 = 재연결 시각, 속도 제한이 풀리는 시각 중 이른 것. 할 일이 없으면 `recv()`.
2. `Show(a)` → `wanted = a`. `Enable(false)` → 연결돼 있으면 활동 없는 SET_ACTIVITY(지우기)를 한 번 보내고 닫는다, `Off`. `Enable(true)` → `Waiting { retry_at: now }`. `Stop` → 연결돼 있으면 지우기 한 번, 닫고, 종료 신호를 보내고 끝.
3. `Waiting` 이고 `now >= retry_at` 이고 켜져 있고 `wanted` 가 있으면 연결기로 후보를 차례로 시도 → 붙으면 HANDSHAKE 를 쓰고 READY 를 읽는다(읽기 제한 `HANDSHAKE_TIMEOUT` 5초, 유닉스만). 성공하면 `Connected`, 간격을 처음으로. 실패면 `retry_at = now + backoff`, 간격 두 배(`RECONNECT_FIRST` 0.5초 → `RECONNECT_MAX` 60초, S7 과 같은 범위). 무작위 섞음은 넣지 않는다(새 의존성 없이, 시험 결정성).
4. `Connected` 이고 `wanted != sent` 이고 속도 제한이 허락하면 SET_ACTIVITY(또는 지우기)를 쓰고 같은 nonce 의 응답이 올 때까지 읽는다(lock-step, S7 hard-mode). 읽는 동안 PING 은 PONG 으로 답하고, CLOSE 는 연결 끊김으로 처리한다. 응답이 `evt: ERROR` 면 연결은 두고 `sent = wanted` 로 적어 같은 활동을 되풀이해 보내지 않는다(문자열 길이 같은 우리 잘못이면 다시 보내도 같다).
5. 쓰기·읽기 오류, 0 바이트 읽기(끊김), 64 KiB 를 넘는 길이, 알 수 없는 opcode → 닫고 `Waiting`(다음 간격).
6. CLOSE 코드 4000(Invalid client ID)·4004(Invalid version) → `Waiting` 에 최대 간격(60초)을 바로 준다(고칠 수 없는 오류로 소켓을 두드리지 않기). 4002(Rate limited) → 최대 간격.
7. 다시 붙으면 `sent = None` 이라 가장 새 활동을 다시 보낸다.

속도 제한(9절 M5 추천안): 20초 창에 5번(S2), 넘으면 창에서 가장 오래된 것이 빠질 때까지 미루고 그때 가장 새 활동만 보낸다(S8 의 "가장 새 것만" 과 같은 결과). 지우기도 한 번으로 센다.

nonce: 연결마다 1부터 오르는 정수의 문자열. pid: `std::process::id()`.

### 5.4 IPC 연결기

- `trait IpcStream: Read + Write + Send` 와 `fn set_read_timeout(&self, Option<Duration>) -> io::Result<()>`(기본 구현은 아무것도 안 함).
- `trait Connect: Send { fn connect(&mut self) -> io::Result<Box<dyn IpcStream>>; }`. 실제 연결기는 후보를 차례로 열고 처음 열린 것을 돌려준다. 시험은 TCP 루프백 연결기·유닉스 소켓 연결기를 주입한다.
- 프레임 쓰기는 머리 8 바이트와 본문을 한 `Vec<u8>` 에 담아 `write_all` 한 번(S7 hard-mode).
- 유닉스(macOS·Linux, `#[cfg(unix)]`): 후보 = 첫 번째로 정의된 기준 폴더(`XDG_RUNTIME_DIR`, `TMPDIR`, `TMP`, `TEMP`, 없으면 `/tmp`) 아래 `discord-ipc-0..=9`. 9절 M9 추천안이면 `XDG_RUNTIME_DIR` 이 있을 때 그 아래 `app/com.discordapp.Discord/discord-ipc-N` 과 `snap.discord/discord-ipc-N` 을 뒤에 더한다. 후보 목록 함수는 환경 읽기를 인자로 받는다(`fn unix_candidates(var: impl Fn(&str) -> Option<OsString>) -> Vec<PathBuf>`) 그래야 시험이 실제 환경을 읽지 않는다. `UnixStream` 의 `set_read_timeout` 을 쓴다.
- Windows(`#[cfg(windows)]`): `OpenOptions::new().read(true).write(true).open(r"\\?\pipe\discord-ipc-N")`, N = 0..=9, `NotFound` 면 다음, 그 밖의 오류(파이프 바쁨 `raw_os_error() == Some(231)` 포함)도 다음 번호로 넘어가고 모두 실패하면 재연결 간격을 둔다(`WaitNamedPipeW` 는 표준 라이브러리에 없어 쓰지 않는다). 파일 핸들에는 읽기 제한이 없어 Discord 가 답하지 않으면 스레드가 기다린다. 프레임 스레드는 영향이 없고, 종료는 5.6 처럼 기다리지 않으므로 막히지 않는다. 후보 이름 함수 `windows_pipe_names()` 는 cfg 없이 두어 모든 OS 에서 문자열 시험을 한다.
- unsafe·새 `#[allow]`·`todo!()` 없음.

### 5.5 설정

| 항목 | 값 |
| --- | --- |
| 필드 | `NetworkOptions.discord_status: bool`, 기본 `true`(`Default for NetworkOptions`). 구조체가 `#[serde(default)]` 라 이 필드가 없는 옛 `settings.ron` 은 켬으로 읽힌다. `skip_serializing_if` 쓰지 않음, 스키마 버전 그대로, 마이그레이션 없음 |
| 행 | `SettingId::DiscordStatus`(목록 끝, `DownloadFolder` 다음), `SETTING_COUNT` 97, `row(SettingId::DiscordStatus, SettingTab::Network, "DISCORD STATUS", SettingKind::Toggle, "Show what you play on your Discord profile")`, `display_value` 는 `on_off`, `adjust` 는 `toggle` |
| 화면 자리(9절 M3 추천) | NETWORK 탭 끝에 새 묶음 `Section { heading: "DISCORD", rows: &[SettingId::DiscordStatus] }`, 묶음 요약 `"THE SCORE SERVER, YOUR ACCOUNT ON IT AND DISCORD"` |
| 설명(catalog `description`) | "Shows R-BMS as the game you are playing on your Discord profile, with the song, its mode and difficulty, the course and how long you have played. Anyone who can see your profile sees it. Needs the Discord desktop app on this computer." |
| 계정 설정 동기화 | 설정 본문 전체가 올라가므로 이 값도 같이 오간다. `settings-blob.json` 이 `discord_status: true,` 한 줄만큼 바뀐다(메인이 `RBMS_UPDATE_CONTRACT=1` 로 재생성). 웹은 문자열로만 다루므로 웹 변경 없음 |
| 반영 | 설정 화면·계정 내려받기 어느 쪽으로 바뀌든 다음 프레임의 bool 비교가 `Enable` 을 보낸다. 끄면 몇 초 안에 Discord 프로필에서 사라진다 |

### 5.6 앱 연결

- `AppShared.discord: discord_presence::DiscordPresence`, `App::new` 에서 `DiscordPresence::for_this_build(config.network.discord_status)`.
- `App::switch` 맨 끝(`on_enter` 뒤): `self.shared.discord.show(discord_presence::view_of(&self.stage, &self.shared))`. `view_of` 는 화면과 공유 상태에서 `PresenceView`(빌린 문자열의 작은 구조체)를 만들고, `show` 가 `activity_for` 로 활동을 짓고 직전과 다를 때만 보낸다.
- `App::frame` 의 poll 들 옆: `self.shared.discord.follow_setting(self.shared.config.network.discord_status)`(bool 비교, 달라졌을 때만 `Enable`).
- `App::exiting` 의 `finish_saves` 뒤: `self.shared.discord.stop()` — `Stop` 을 보내고 스레드의 종료 신호를 `EXIT_PATIENCE`(250 ms) 만 기다린다. 못 받아도 그냥 끝낸다(프로세스가 끝나면 소켓이 닫히고 Discord 가 그 pid 의 활동을 지운다). join 하지 않는다(Windows 에서 읽기가 막혀 있을 수 있음).
- 경과 시간의 기준 시각(유닉스 초)은 `DiscordPresence` 가 쥔다: 앱 시작(`SystemTime::now()` 를 생성 때 한 번), 플레이 시작(Play 화면에 들어온 `switch`), 코스 시작(`course_run` 이 `None` → `Some` 이 된 첫 `switch`). 시각 읽기는 시스템 호출이지 I/O 가 아니다.

### 5.7 화면 → 활동 대응 표

모든 문구는 영어. `{title}` = `title` 과 `subtitle` 을 공백 하나로 이은 것(서브타이틀이 비면 제목만). `{chart}` = `{mode} {difficulty} {level}` 을 공백으로 이은 것에서 빈 조각을 뺀 것(`mode_short` 가 `?`, `difficulty_name` 이 `—`, `play_level` 이 빈 경우 그 조각 생략), 표 폴더에서 고른 곡이면 끝에 ` · {table level}`(예: `7K ANOTHER 12 · sl5`). `{version}` = `env!("CARGO_PKG_VERSION")`. 큰 그림은 언제나 `large_image = "rbms"`(9절 M2).

| 화면·상태 | details | state | large_text | timestamps.start |
| --- | --- | --- | --- | --- |
| Select, Folders, Tables | `Choosing a song` | (없음) | `R-BMS {version}` | 앱 시작 |
| Settings, KeyConfig, SkinMenu, PlayMenu | `In the menus` | (없음) | `R-BMS {version}` | 앱 시작 |
| Loading(Scan) | `Scanning the song folders` | (없음) | `R-BMS {version}` | 앱 시작 |
| Decide, Loading(Song·Assets), 곡 하나 | `{title}` | `Loading · {chart}` | `R-BMS {version}` | 앱 시작 |
| Play, 손으로 치는 판 | `{title}` | `{chart}` | `R-BMS {version}` | 이 판 시작 |
| Play, 오토플레이(`run_plays_itself`) | `{title}` | `Autoplay · {chart}` | 같음 | 이 판 시작 |
| Play, 리플레이 보기(`replay.is_some()`) | `{title}` | `Watching a replay · {chart}` | 같음 | 이 판 시작 |
| Practice 화면, 연습 판(`practice.is_some()`) | `{title}` | `Practice · {chart}` | 같음 | 연습 판이면 그 판 시작, 패널이면 앱 시작 |
| 코스 중 Decide·Loading·Play | `{course name} · {n}/{m}` | `{title}` | `{chart}` | 코스 시작 |
| Result(곡 하나) | `{title}` | `Result · {clear_label}`(예 `Result · HARD CLEAR`) | `{mode_label}` 또는 `R-BMS {version}` | 앱 시작 |
| 코스 중 스테이지 Result | `{course name} · {n}/{m}` | `{title}` | `{chart}` | 코스 시작 |
| CourseResult | `{course name}` | `Course cleared` / `Course failed` | `R-BMS {version}` | 앱 시작 |
| 설정 끔 | 활동 지우기(Command `Enable(false)`) | | | |

- 오토플레이·리플레이·연습 판도 보인다(9절 M6). 판정·기록 규칙은 바꾸지 않는다(표시만).
- 일시 정지 개념이 손으로 치는 판에는 없으므로 표시도 없다. 리플레이 분석의 멈춤은 표시하지 않는다(화면이 바뀌지 않음).
- 활동에 `type` 은 넣지 않는다(빠지면 Playing, discord-rpc 와 같음, S1 이 허용하는 0 과 같다). `instance: false`. `name`·`buttons`·`party`·`secrets` 없음.
- timestamps 단위는 초(9절 M8 추천, S3·S8·discord-rpc 와 같음).

길이 처리 `fit_text(s) -> Option<String>`:
1. 앞뒤 공백을 자르고 비면 `None`(그 필드를 빼고 보낸다. details 가 없으면 state 도 Discord 가 아래로 당긴다).
2. 글자 수가 2 미만(`MIN_TEXT_CHARS` = 2)이면 따옴표로 감싼다(`A` → `"A"`, 3자). 공백 덧붙이기는 잘릴 수 있어 쓰지 않는다.
3. UTF-8 128 바이트(`MAX_TEXT_BYTES` = 128, S4 의 128자와 S7 의 128 바이트 둘 다 만족)를 넘으면 125 바이트 안의 마지막 글자 경계에서 자르고 `...`(ASCII 3 바이트)를 붙인다. 멀티바이트(일본어 제목) 글자를 쪼개지 않는다.
4. 큰 그림 키는 상수라 따로 자르지 않는다(4 바이트, 32 바이트 상한 안).

## 6. Windows·Linux·macOS 차이

| 항목 | macOS | Linux | Windows |
| --- | --- | --- | --- |
| 전송 | `UnixStream`, Discord 는 사용자별 `$TMPDIR`(`/var/folders/.../T/`) 아래 | `UnixStream`, 보통 `$XDG_RUNTIME_DIR`(`/run/user/<uid>`) 아래. flatpak·snap Discord 는 하위 폴더(9절 M9) | `\\?\pipe\discord-ipc-N` 를 `OpenOptions` 로 연 `File` |
| 읽기 제한 | 있음(`set_read_timeout`) | 있음 | 없음(스레드가 기다림, 5.4) |
| 여러 Discord(Stable·PTB·Canary) | 0~9 중 처음 붙는 것 | 같음 | 같음 |
| 시험 | 유닉스 소켓 시험 + 루프백 시험 | 같음 | 루프백 시험만(표준 라이브러리로 파이프 서버를 만들 수 없음), 파이프 이름 문자열 시험 |
| 유닉스 소켓 경로 길이 | 시험의 임시 폴더는 `std::env::temp_dir()` 아래 짧은 이름(macOS `sun_path` 104 바이트 한도) | 같음 | 해당 없음 |

## 7. 사용자가 Developer Portal 에서 할 일

앱: https://discord.com/developers/applications/1563156237539999865

1. **General Information** → NAME 을 `R-BMS` 로(프로필에 "Playing R-BMS" 로 보이는 이름. 활동에 name 을 보내지 않으므로 이 이름이 쓰인다).
2. **General Information** → APP ICON 에 `assets/icon/png/discord-1024.png`(4절 방법으로 만든 꽉 찬 1024x1024, M1 추천). 만들지 않기로 하면 `assets/icon/png/icon-1024.png`.
3. **Rich Presence → Art Assets** → 같은 파일을 올리고 키 이름을 `rbms` 로(Portal 이 소문자로 바꾸므로 소문자로 적는다, S3). 저장. 올린 뒤 Discord 에 보이기까지 시간이 걸릴 수 있다(시간은 문서에 없음, 확인 못함).
4. DESCRIPTION 은 선택(예: "Open-source BMS player"). 그 밖의 설정(OAuth2, Bot, Social SDK 신청)은 하지 않는다.
5. 확인: Discord 데스크톱 앱 → 사용자 설정 → 활동 개인정보 → "활동 상태 표시" 가 켜져 있어야 보인다.

## 8. 시험 목록

모든 시험은 실제 Discord 에 붙지 않는다(가짜 서버 또는 주입한 연결). 시간은 주입한 `now` 로 진행하고 잠들지 않는다. 가짜 서버 스레드와는 소켓 주고받기 순서로만 맞춘다. 읽기 제한은 실패 때 멈추지 않게 하는 안전장치(5초)일 뿐 성공 경로의 조건이 아니다(떠는 시험 금지).

### 8.1 순수 단위(모든 OS)

| # | 이름(뜻) | 확인 |
| --- | --- | --- |
| T1 | 프레임 인코딩 | opcode·길이 LE, 본문 바이트, 한 버퍼 |
| T2 | 프레임 디코딩 | 정상, 머리 일부만, 길이 > 64 KiB 거절, 알 수 없는 opcode |
| T3 | 핸드셰이크 JSON | 정확히 `{"v":1,"client_id":"1563156237539999865"}` |
| T4 | SET_ACTIVITY JSON | `cmd`, `args.pid`, 문자열 nonce 증가, 빈 필드 생략, `type` 없음, 지우기는 `activity` 키 없음 |
| T5 | `fit_text` | 빈 → None, 1자 → 따옴표, 128 바이트 경계의 일본어 제목이 글자 중간에서 안 잘림, 결과 ≤128 바이트, 2자 이상 |
| T6 | 대응 표 | 5.7 표의 각 줄을 `PresenceView` 로 만들어 details·state·large_text·start 단언(오토·리플레이·연습·코스 n/m·표 레벨 있음/없음·모드 `?`·난이도 0) |
| T7 | 재연결 간격 | 0.5, 1, 2, … , 60 에서 멈춤, 성공 뒤 처음으로, Close 4000·4002·4004 는 바로 60 |
| T8 | 속도 제한 | 20초 창 5번 통과, 6번째는 가장 오래된 것 + 20초까지 미룸, 그 사이 여러 번 바뀌면 가장 새 것 하나만, 같은 활동은 다시 안 보냄 |
| T9 | 유닉스 후보 | 환경 순서 XDG_RUNTIME_DIR > TMPDIR > TMP > TEMP > /tmp(주입한 환경), 0..9, flatpak·snap 하위는 XDG 일 때만(M9 A) |
| T10 | Windows 파이프 이름 | `\\?\pipe\discord-ipc-0` ~ `-9` |
| T11 | 프레임 스레드 쪽 중복 제거 | 같은 활동 두 번 `show` → 명령 하나, 설정 켬/끔 bool 이 같으면 명령 없음 |

### 8.2 가짜 서버(TCP 루프백, 모든 OS)

| # | 확인 |
| --- | --- |
| L1 | 핸드셰이크 → READY → SET_ACTIVITY 가 서버에 기대한 JSON 으로 도착 → 같은 nonce 응답 → `sent` 갱신 |
| L2 | 서버가 응답 전에 PING 을 보내면 같은 본문의 PONG 이 오고 그 뒤 응답을 받아 끝난다 |
| L3 | 서버가 CLOSE `{code:4000}` → 연결 닫힘, 다음 시도 60초 뒤 |
| L4 | READY 뒤 서버가 끊음 → 새 활동이 오면 간격 뒤(주입 now) 다시 붙어 가장 새 활동을 보낸다 |
| L5 | 서버 없음(연결 거절) → 패닉 없음, 간격이 늘어남, 설정 끔이면 시도하지 않음 |
| L6 | `Enable(false)` → 활동 없는 SET_ACTIVITY 하나 뒤 닫힘, 그 뒤 연결 시도 없음 |
| L7 | 응답이 `evt: ERROR` → 연결 유지, 같은 활동을 다시 보내지 않음 |
| L8 | READY 대신 엉뚱한 FRAME → 닫고 간격 |
| L9 | `Stop` → 지우기 하나, 종료 신호, 스레드 끝남 |

### 8.3 유닉스 소켓(`#[cfg(unix)]`)

| # | 확인 |
| --- | --- |
| U1 | 임시 폴더에 `UnixListener` 로 `discord-ipc-0` 을 열고 주입 환경 `TMPDIR` 을 그 폴더로 → 실제 유닉스 연결기가 붙어 L1 과 같은 흐름 |
| U2 | `discord-ipc-0` 이 없고 `discord-ipc-1` 만 있을 때 1 에 붙음 |

### 8.4 앱·설정

| # | 확인 |
| --- | --- |
| A1 | 테스트 앱(`for_this_build` 기록기)에서 Select → Decide → Loading → Play 전환이 5.7 표의 활동을 차례로 남기고, 플레이 중 프레임은 명령을 남기지 않음 |
| A2 | 설정 DISCORD STATUS 를 끄면 다음 프레임에 `Enable(false)` 하나, 켜면 `Enable(true)` 와 현재 활동 |
| A3 | `App::exiting` 이 `Stop` 을 남김 |
| A4 | 단위 테스트 빌드의 `AppShared` 가 스레드를 띄우지 않음(`for_this_build` 가 기록기) |
| C1 | `rbms-config`: 필드 없는 옛 RON → `discord_status == true`, 기본값 true, 왕복 직렬화 |
| C2 | `rbms-config`: `tab_rows(SettingTab::Network)` 단언 끝에 `DiscordStatus`, `SETTING_COUNT` 97, `adjust` 토글 |
| C3 | `catalog`: NETWORK 의 `DISCORD` 묶음, 설명 문구 있음 |
| K1 | 계약 `settings-blob.json` 재생성 diff 가 `discord_status: true,` 한 줄(메인) |

### 8.5 실행 명령

```bash
cargo test -p rbms-config --locked
cargo test -p rbms-player --locked discord_presence
RBMS_UPDATE_CONTRACT=1 cargo test -p rbms-player --locked contract_requests   # 메인이
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
```

실기 점검(사용자, QA 문서에 추가): Discord 데스크톱 켜고 `rbms-player` 실행(개발자가 직접 띄울 때는 `HOME` 을 새 임시 폴더로, `--server http://127.0.0.1:9`), 프로필에 "Playing R-BMS"·아이콘·곡 제목·`7K ANOTHER 12` 꼴·경과 시간, 코스 `1/4 · {name}`, 설정 끄면 사라짐, Discord 를 끄고 켜면 60초 안에 다시 보임, 앱 종료하면 사라짐, Windows·Linux(가능하면 flatpak) 각각.

## 9. 메인 결정 필요

| # | 결정 | 추천 | 근거 | 대안 |
| --- | --- | --- | --- | --- |
| M1 | 아이콘 파일 | **A: 꽉 찬 1024x1024 RGB `assets/icon/png/discord-1024.png` 를 `generate-icon.py` 로 추가** | Discord 가 둥글게 자르므로 투명 여백 10% 가 그림을 작게 하고 테두리를 이중으로 보이게 한다. `master(full_bleed=True)` 가 이미 있어 한 줄 | B: 지금의 `icon-1024.png` 그대로 |
| M2 | 큰 그림 | **A: Art Asset 키 `rbms`** | 공식 권장 경로, 외부 사이트·미디어 프록시에 기대지 않음, 사용자가 Portal 에서 올림 | B: 외부 URL `https://bms.hyuns.uk/icon.png`(Portal 업로드 불필요하나 512 투명 그림이고 "알맞은 크기" 기준이 문서에 없음, IPC 에서 URL 이 받아지는지 실기 전엔 확인 못함) |
| M3 | 설정 행 자리 | **A: NETWORK 탭 끝 새 묶음 DISCORD, 탭 요약 문구에 Discord 추가** | 밖으로 정보를 내보내는 설정이 모인 곳 | B: DISPLAY 탭 |
| M4 | 경과 시간 | **A: 메뉴·선곡·결과는 앱 시작부터, 판은 그 판 시작부터, 코스는 코스 시작부터** | 사용자가 말한 "경과 시간" 을 플레이 중엔 곡 시간, 그 밖엔 세션 시간으로 | B: 판(코스) 중에만 보이고 메뉴에선 안 보임 |
| M5 | 갱신 속도 | **A: 20초 5번 창 + 가장 새 것만** | 공식 게이트웨이 수치(S2), 곡을 고르자마자 판 표시가 늦지 않음 | B: 15초에 1번(옛 SDK 문서 S8, 선곡→플레이 표시가 최대 15초 늦음) |
| M6 | 오토플레이·리플레이·연습 | **A: 보이되 앞에 `Autoplay`·`Watching a replay`·`Practice` 표시** | 숨기면 친 판처럼 보이는 오해가 없고, 보이는 쪽이 사실과 같음 | B: 선곡 중처럼 보임 |
| M7 | 버튼 | **A: 넣지 않음** | 범위 최소, 본인에게는 보이지 않아 실기 확인이 어려움 | B: `R-BMS` → `https://bms.hyuns.uk` 버튼 1개 |
| M8 | timestamps 단위 | **A: 유닉스 초** | IPC 를 쓰는 공식 구현(discord-rpc)·옛 IPC 문서·Social SDK 안내가 초, SDK 는 작은 값을 초로 알아봄 | B: ms(게이트웨이 객체·SDK 클래스 문서). 실기에서 경과 시간이 이상하면 바꾼다 |
| M9 | Linux flatpak·snap 경로 | **A: XDG_RUNTIME_DIR 이 있을 때 `app/com.discordapp.Discord/`, `snap.discord/` 하위도 찾음** | Linux 사용자 상당수가 flatpak Discord. 찾는 비용은 연결 실패 몇 번 | B: 공식 경로만(공식 근거 없는 것은 넣지 않음) |
| M10 | 모듈 자리 | **A: `rbms-player` 안 `discord_presence/`** | 쓰는 곳이 플레이어 하나, `record_sync`·`save_worker` 와 같은 꼴, 작업공간 멤버·문서 늘지 않음 | B: 새 크레이트 `rbms-discord` |
| M11 | 코스 표시 배치 | **A: details = `{course} · n/m`, state = 곡 제목, 곡 정보는 그림 hover** | 코스가 지금 하는 일의 단위, 두 줄에 코스·곡이 다 보임 | B: details = 곡 제목, state = `{course} · n/m` |

## 10. 구현 단위와 소유 파일

| 단위 | 소유 파일(수정 가능) | 읽기 전용 | 선행 |
| --- | --- | --- | --- |
| P1 설정 | `crates/rbms-config/src/schema.rs`, `crates/rbms-config/src/settings.rs`, `crates/rbms-config/src/tests.rs`(또는 `tests/` 하위 해당 파일), `apps/rbms-player/src/stage/settings/catalog.rs` | `apps/rbms-player/src/settings_ui.rs`, `settings_view.rs` | 없음 |
| P2 IPC·링크 | 새 `apps/rbms-player/src/discord_presence/{ipc.rs,link.rs}` 와 그 시험, `apps/rbms-player/Cargo.toml`(serde_json 한 줄), `Cargo.lock`(`rbms-player` 의존 목록의 그 한 줄만) | `save_worker.rs`, `gpu/present.rs`(종료 기다림 본보기) | 없음(P1 과 병렬) |
| P3 활동·앱 연결 | 새 `apps/rbms-player/src/discord_presence.rs`, `discord_presence/activity.rs` 와 그 시험, `apps/rbms-player/src/lib.rs`(`mod` 한 줄, `AppShared` 필드와 생성, `switch`·`frame`·`exiting` 세 줄), `apps/rbms-player/src/stage/decide.rs`(`table_of_the_open_folder` 를 `pub(crate)` 로, 두 곳에서 쓰므로), `apps/rbms-player/src/stage/loading.rs`(스캔인지 묻는 접근자 하나) | `format.rs`, `select_tree.rs`, `stage/result.rs`, `stage/course_result.rs`(필드 접근자가 없으면 `pub(crate)` 접근자 하나씩 추가는 허용) | P1, P2 |
| P4 아이콘(M1 A 일 때) | `assets/icon/tools/generate-icon.py`, 새 `assets/icon/png/discord-1024.png`, `assets/icon/README.md` | - | 없음(병렬) |
| 메인 | `contract/client-requests/settings-blob.json` 재생성, 문서(11절), ADR, 커밋 | - | P1~P3 |

같은 파일을 두 단위가 고치지 않는다. P3 의 `lib.rs` 는 다른 진행 중 작업과 겹치면 메인이 순서를 정한다.

## 11. 문서 갱신 목록

| 문서 | 바꿀 것 |
| --- | --- |
| 새 ADR `docs/acknowledge/2026-10-23-adr-discord-presence.md` | 결정(IPC·의존성 없음·기본 켬·Application ID 공개 상수·표시 항목·M1~M11 결과), 사용자 원문 |
| `docs/architecture.md` | 백그라운드 스레드 목록에 `rbms-discord`(화면 전환 때만 보냄, 프레임 스레드 I/O 없음, 종료 때 기다리지 않음) |
| `docs/development.md` | 모듈 표에 `discord_presence/`, 시험이 Discord 에 붙지 않는 이유(`for_this_build`, 주입 연결기) |
| `docs/crates.md` | `rbms-config` 의 `NetworkOptions.discord_status`, `SETTING_COUNT` 97 과 DISCORD STATUS 행, 플레이어 모듈 |
| `docs/play-options.md` | 설정 행 설명(NETWORK 의 DISCORD 묶음), 보이는 문구 표(5.7 요약) |
| `README.md` | 기능 목록에 Discord 상태 표시와 끄는 법 |
| `assets/icon/README.md` | `png/discord-1024.png` 줄(Portal 앱 아이콘·Art Asset `rbms` 용) |
| `docs/quality-assurance/` 실기 점검표 | 8.5 의 실기 항목 |
| `docs/quality-assurance/2026-10-14-test-debt.md` | 12절 부채 |
| `docs/PROCESS.md`, `docs/HANDOFF.md` | D 항목 진행, 사용자 Portal 할 일 |

## 12. 위험과 테스트 부채 후보

| 위험 | 영향 | 대응 |
| --- | --- | --- |
| IPC Rich Presence 는 공식적으로 "옛 방식"(discord-rpc deprecated, 새 프로젝트는 Social SDK 권장) | Discord 가 언젠가 IPC SET_ACTIVITY 를 막으면 표시가 안 됨(앱 동작에는 영향 없음, 조용히 실패) | 받아들임. Social SDK 는 네이티브 라이브러리라 새 의존성 금지와 충돌 |
| 공개 노출 | 프로필을 볼 수 있는 사람은 곡 제목을 본다 | 설명 문구에 적고 설정으로 끌 수 있음. 기본 켬은 사용자 결정 |
| timestamps 단위 | 초·ms 문서가 엇갈림 | M8, 실기에서 확인 |
| Windows 파이프 읽기 막힘 | Discord 가 답하지 않으면 스레드가 멈춤(표시만 멈춤) | 프레임 스레드와 분리, 종료 때 기다리지 않음 |
| Art Asset 미등록 | 키 `rbms` 가 없으면 큰 그림이 비거나 앱 아이콘으로 대신 보임(확인 못함) | 7절을 사용자 할 일로 보고 |
| flatpak·snap 경로 | 공식 근거 없음 | M9, 부채로 실기 |
| 테스트 부채 | 실제 Discord(Stable·PTB·Canary 동시), Windows 파이프 바쁨, Linux flatpak·snap, 15초/5회 제한의 실제 동작, 큰 그림 URL 방식 | `docs/quality-assurance/2026-10-14-test-debt.md` 에 재현 조건·생략 이유·실행 시점과 함께 |

---

## 메인 결정 (2026-10-23, 메인)

M1~M11 은 모두 이 문서의 추천안을 채택한다. 사용자 결정(PROCESS 의 D 항목: workflow, v0.1.8 전, 곡·난이도·모드·코스·경과 시간, DISCORD STATUS 기본 켬)과 같은 방향이다.

- Portal 은 메인이 2026-10-23 사용자 요청("너가 직접 미리 다 해놔")으로 Claude in Chrome 으로 설정했다: 앱 이름 R-BMS(이미 그 이름), 앱 아이콘 `assets/icon/png/icon-1024.png`, 설명 한 문단, 태그 rhythm·bms·game, Rich Presence Art Asset 키 `rbms`(같은 1024 그림). 7절의 사용자 할 일은 이것으로 끝났다.
- M1 에 따라 여백 없는 `discord-1024.png` 를 만들면, 메인이 Portal 의 앱 아이콘과 Art Asset `rbms` 를 그 파일로 다시 올린다(키 이름은 그대로 `rbms`).
