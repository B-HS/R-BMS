# ADR: Discord Rich Presence 로 R-BMS 를 게임으로 보이기

> 최종 갱신 2026-10-23 · 대응 커밋 3d21944
> 상태: 유효(2~4 와 설계서 M1~M11 은 사용자 미확인) · 결정일 2026-10-23 · 결정자 사용자(기능·workflow·Application ID), 메인(나머지) · 커밋 d7892ad(설계), 3d21944(구현)

## 배경

- 사용자 요청(2026-10-23): "draft올리기전에, 혹시 discord에 게임으로 인식되게하는 기능도 만들 수 있어 ?" 메인이 Rich Presence(로컬 IPC)를 추천하고 결정 네 가지를 물었다.
- 답: "좋아 1번대로 하도록하고 Application 만들어서 아이디는 1563156237539999865 / 아이콘이나 기타 꼼꼼하게 잘 할수있지 ?" 그 뒤 "portal에서 뭘해야하나 ; … 에서 너가 직접 미리 다 해놔".
- Discord 의 자동 감지(Detectable games)는 Discord 쪽 목록 등록이 필요해 코드로 할 수 없다.

## 결정

1. 진행은 workflow(사용자). 2~4 는 답이 없어 메인이 추천안으로 정했다(사용자 미확인): v0.1.8 전에 넣는다, 곡 제목·난이도·모드·코스·경과 시간을 보인다, 설정 NETWORK > DISCORD > DISCORD STATUS 를 두고 기본 켬.
2. 방식: Discord 앱과의 로컬 IPC(macOS·Linux 유닉스 소켓 `discord-ipc-0`~`9`, Windows named pipe). 새 크레이트 없이 std 와 이미 쓰는 `serde_json` 으로 한다(`apps/rbms-player/src/discord_presence/`). 프레이밍·핸드셰이크·SET_ACTIVITY 는 설계서 2절의 공식 문서와 `discord/discord-rpc` 를 따른다.
3. 스레드: 전용 스레드 `rbms-discord` 가 채널로 명령을 받는다. 프레임 스레드는 화면이 바뀔 때와 설정이 바뀐 프레임에만 명령을 보내고, 플레이 중에는 보내지 않는다(렉 기준 ADR).
4. 재연결: 0.5초에서 60초까지 두 배씩 늘리며 조용히 다시 붙는다. 간격은 첫 활동이 수락된 뒤에만 처음으로 되돌린다. Close 4000·4002·4004 는 바로 60초. 갱신은 20초에 5번까지, 그 사이 바뀐 것은 가장 새 것 하나만. 수락된 활동은 Discord 가 꺼졌다 켜진 것을 알아채려고 20초마다 다시 보내고, 거절(`evt: ERROR`)된 활동은 다시 보내지 않는다.
5. 표시: 앱 이름 R-BMS, 큰 그림은 Art Asset 키 `rbms`, 곡 제목, `7K ANOTHER 12` 꼴 줄, 코스는 `n/m · 코스 이름`(긴 이름에서도 진행이 잘리지 않게), 오토플레이·리플레이·연습 표시, 경과 시간(초 단위). 글자는 128바이트 안으로 자른다.
6. Developer Portal 은 메인이 Claude in Chrome 으로 설정했다(사용자 요청): 앱 아이콘과 Art Asset `rbms` 를 여백 없는 `assets/icon/png/discord-1024.png`(생성기 `assets/icon/tools/generate-icon.py` 가 만듦)로, 설명 한 문단, 태그 rhythm·bms·game.

## 이유

- IPC 는 Discord 데스크톱이 지원하는 유일한 무의존 경로다. 공식 Social SDK 는 네이티브 라이브러리라 새 의존성 금지와 맞지 않는다.
- 전용 스레드와 화면 전환 시점 전송으로 플레이 프레임에 영향이 없다.

## 기각된 대안

| 대안 | 이유 |
| --- | --- |
| Discord Social SDK | 네이티브 라이브러리 의존 |
| `discord-rich-presence` 등 크레이트 | 새 의존성 금지, 프로토콜이 단순해 직접 구현이 작다 |
| 큰 그림을 외부 URL 로 | 공식 문서로 IPC 지원을 확인하지 못했다. Art Asset 키가 확실하다 |
| 설정 행을 SELECT 탭에 | 성격이 맞지 않는다. NETWORK 탭이 20칸이 되어 스크롤되지만 설정 화면 설계(18칸 넘으면 스크롤)가 허용한다 |

## 영향과 관련 파일

- `apps/rbms-player/src/discord_presence.rs`, `discord_presence/{activity,ipc,link,tests}.rs`, `lib.rs`(App::switch, App::wind_down), `stage/settings/catalog.rs`, `app_network.rs`, `crates/rbms-config/src/{schema,settings}.rs`(`NetworkOptions.discord_status`, `#[serde(default)]`), `contract/client-requests/settings-blob.json`, `assets/icon/png/discord-1024.png`.
- 시험 29개는 가짜 TCP·유닉스 소켓 서버와 주입한 시각만 쓰고 실제 Discord 에 붙지 않는다.
- 확인하지 못한 것: Windows 파이프(이 Mac 에 Windows 타깃 없음), 경과 시간 단위가 초가 맞는지, flatpak·snap 경로, 실제 프로필 표시. 실기 점검표 3.22, 테스트 부채 3.111.

## 날짜와 사용자 원문

2026-10-23. 위 배경의 세 문장.
