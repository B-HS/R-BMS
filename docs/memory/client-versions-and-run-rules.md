# 클라이언트의 버전 상수와 실행 규칙

최종 갱신 2026-10-24 · 대응 커밋 66b9e63

다음 세션이 다시 찾지 않도록 모아 둔 사실이다. 값은 코드가 정본이고, 바뀌면 이 문서를 같이 고친다.

## 버전 상수

| 이름 | 값 | 어디서 | 뜻 |
| --- | --- | --- | --- |
| 기록 규칙 세대 `SCORE_RULE_VERSION` | 4 | `crates/rbms-store/src/score.rs` | 세대 4 부터 원본 구현의 기록 정의를 따른다(`avgjudge`, 게이지로 끝난 판의 BP, 코스 곡별 콤보, 풀콤, TOTAL 어시스트). 판정은 입력 이벤트 시각, 롱노트는 원본이 정리하는 대로 정리한 차트 위에서 한다 |
| 곡 파서 세대 `PARSER_VERSION` | 5 | `crates/rbms-library/src/songdb/schema.rs` | 세대 5 는 낱개 `\r` 로 줄을 끊고 `#TOTAL` 을 원본처럼 자른다. 저장된 열이 바뀌므로 올린 뒤 첫 실행은 곡 DB 를 다시 읽는다 |
| 설정 스키마 `CURRENT_SCHEMA_VERSION` | 6 | `crates/rbms-config/src/schema.rs` | 올라가면 첫 실행이 `settings.ron.v<이전>.bak` 을 남긴다 |
| KEY DEBOUNCE | 기본 16 ms, 0~100 | 설정 키 `play.key_debounce_ms` (`crates/rbms-config/src/schema.rs`) | 키보드의 LR2 형 디바운스 |
| PAD DEBOUNCE | 기본 16 ms, 0~100 | 키 설정 파일 `keyconfig.ron` 의 `pad.debounce_ms` | 컨트롤러의 LR2 형 디바운스. 설정 화면의 INPUT 탭에서 바꾸지만 값은 `settings.ron` 이 아니라 `keyconfig.ron` 에 있다 |
| 설정 행 수 `SETTING_COUNT` | 99 | `crates/rbms-config/src/settings.rs` | 2026-10-23 에 COURSE GAUGE, MISSING CHARTS, DOWNLOAD FOLDER, DISCORD STATUS 가 더해져 97, 2026-10-24 에 LANGUAGE(DISPLAY 탭 맨 위, 98)와 RANKING KEYS(SELECT 탭 SONG LIST 구역, 99)가 더해졌다. 길이 고정 배열(예 `apps/rbms-player/src/settings_ui.rs` 시험 고정표 `SHIPPED_ROWS`)이 함께 바뀌고, DISPLAY 탭이 한 줄 늘어 스크롤이 생기는 것을 `stage/settings/layout_tests.rs`·`skin_tests.rs` 가 단언한다 |
| 2026-10-24 설정 필드 | 스키마 6 그대로(`#[serde(default)]`, `skip_serializing_if` 없음) | `crates/rbms-config/src/{schema,language}.rs` | `display.language`(`AUTO`·`ENGLISH`·`KOREAN`·`JAPANESE`, 없거나 모르는 토큰은 AUTO, 기본은 AUTO), `library.ranking_keys`(켬). 계정 설정 묶음과 함께 동기화된다(`ir_sync.rs` 의 `keep_local` 에 넣지 않았다). `contract/client-requests/settings-blob.json` 에 `language: "AUTO"`, `ranking_keys: true` 가 있다 |
| NOTES RADAR 정의 세대 `RADAR_VERSION` | 1 | `crates/rbms-chart/src/radar.rs` | 지표 정의가 바뀔 때만 올린다(표시 스케일 `RADAR_SOFLAN_SCALE`·`RADAR_CHARGE_SCALE`·`RADAR_CHORD_SCALE` 은 표시 단계라 올리지 않는다). 곡 DB 표 `song_radar`(`crates/rbms-library/src/songdb/schema.sql`)의 `version` 열이 이 값이고 `SCHEMA_VERSION` 1·`PARSER_VERSION` 5 는 그대로다. 현재 버전이 아닌 행은 선곡에서 커서가 머물 때 작업자가 다시 재서 저장한다 |
| 확장 스킨 id 공간 | 이벤트 21000~21099, 불린 21100~21199, 정수·Float·Rate·텍스트·참조 이미지 21000~21199 | `crates/rbms-skin/src/property/extension.rs`(대역과 `implements`), `apps/rbms-player/src/skin_host/extension.rs`(이름) | 2026-10-24 부터 불린·이벤트 외 공간도 연다. Float 와 Rate 는 한 공간처럼 번호를 나눈다. 정본 배정표는 [ADR](../acknowledge/2026-10-24-adr-rbms-extension-id-spaces.md) 3절. 언어 불린은 21150~21152(`21150 + Language::id()`), 레이더 정수·Float 21020~21025, 참조 이미지 `"-21020"`, 랭킹 모드 이벤트 21002·불린 21105. 새 id 를 더할 때는 상수·라우트·`props.lua` 표와 `skin_host/tests.rs` 의 대역 시험을 함께 고친다 |
| 단축키 수 `Shortcut::ALL` | 32 | `apps/rbms-player/src/keyconfig.rs` | 2026-10-24 에 `ReplayPlayLayout`(기본 `J`, 그룹 Replay, `config_key` `replay_play_layout`)이 더해져 31 에서 32. 옛 키 파일에서 다른 Replay 키가 `J` 를 쓰면 이 단축키는 키 없이 남는다 |
| UI 언어 | `Language` 셋(영어·한국어·일본어), 환경변수 `RBMS_LANGUAGE` | `crates/rbms-model/src/language.rs`, `apps/rbms-player/src/app_language.rs` | AUTO 는 `RBMS_LANGUAGE` → OS 로캘 → 영어. 스킨은 불린 21150~21152, 카탈로그는 `translations!`. 시험은 영어를 명시한다([UI 다국어 ADR](../acknowledge/2026-10-24-adr-ui-language.md)) |
| 랭킹 받기 상수 | 쪽 100줄, 만료 10분, 실패 재시도 30초, 캐시 64곡, 자동 받기 5초, 길게 누름 300 ms | `apps/rbms-player/src/ir_ranking.rs`(`RANKING_PAGE_LIMIT`, `RANKING_READY_LIFETIME`, `RANKING_FAILED_RETRY_AFTER`, `RANKING_CACHE_CAPACITY`), `stage/select/ranking_mode.rs`(`RANKING_AUTO_FETCH_AFTER`, `SCRATCH_REPEAT_DELAY`) | [랭킹 모드 ADR](../acknowledge/2026-10-24-adr-ranking-mode.md) |
| 2026-10-23 설정 필드 | 스키마 6 그대로(`#[serde(default)]`, `skip_serializing_if` 없음) | `crates/rbms-config/src/schema.rs` | `play.scratch_auto_color`, `judge.course_gauge`(LR2 기본), `library.download_folder`(기기 고유, 업로드에서 뺌), `library.show_missing_charts`(켬), `network.discord_status`(켬), `library.tables[].group`(빈 값도 기록). 하나라도 바뀌면 `contract/client-requests/settings-blob.json` 을 `RBMS_UPDATE_CONTRACT=1 cargo test --locked -p rbms-player --lib a_settings_upload` 로 다시 만든다 |
| Discord Application ID | `1563156237539999865` | `apps/rbms-player/src/discord_presence.rs` 의 `DISCORD_APPLICATION_ID` | 비밀이 아니다. Art Asset 키 `rbms`(Developer Portal) |

## 실행 규칙

- 한국어·일본어 화면을 찍거나 확인하려면 `RBMS_LANGUAGE=ko`(또는 `ja`)를 주고 scratch HOME 으로 띄운다. 캡처 하니스는 번들 Inter 만 써서 한글·일본어 글자를 그리지 못한다(빈 네모). 모양은 실창으로만 볼 수 있다.
- 에이전트는 `rbms-player` 를 `HOME=$(mktemp -d)` 와 `--server http://127.0.0.1:9` 로만 실행한다. 사용자의 설정과 기록 DB 를 열고 운영 서버로 기록을 보내기 때문이다(2026-10-15 사고, [피드백](../feedback/2026-10-15-run-the-player-with-a-scratch-home.md)). 부작용 없이 끝나는 실행은 `--version`·`--help` 뿐이다.
- 시험 라이브러리의 경로와 내용은 [test-library.md](test-library.md).
- 운영 서버에는 GET 만 보낸다. 운영 DB 작업은 `web-db` 워크플로로만 한다([ci-release.md](../ci-release.md) 6.1절).

## 확인하지 못한 것

- S19 감사 중 `airTKkill`·`airTkill` 두 차트는 `#TOTAL` 이 1.1·1 이라 오토플레이 만점도 FAILED 이고 원본도 같다고 보았다(결함 아님). 이번 문서 점검에서는 라이브러리 안의 차트를 다시 읽지 않았다.
- S19 에서 라이브러리 전체의 오토플레이 만점을 센 도구(세션 한정 scratch 의 `sem`/`probe` 와 차트 목록)는 저장소에 남기지 않았다. 다시 하려면 새로 만들거나 저장소에 넣을지 정해야 한다([테스트 부채](../quality-assurance/2026-10-14-test-debt.md) 3.18).
