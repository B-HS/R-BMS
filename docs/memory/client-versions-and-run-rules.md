# 클라이언트의 버전 상수와 실행 규칙

최종 갱신 2026-10-24 · 대응 커밋 ec48d8d

다음 세션이 다시 찾지 않도록 모아 둔 사실이다. 값은 코드가 정본이고, 바뀌면 이 문서를 같이 고친다.

## 버전 상수

| 이름 | 값 | 어디서 | 뜻 |
| --- | --- | --- | --- |
| 기록 규칙 세대 `SCORE_RULE_VERSION` | 4 | `crates/rbms-store/src/score.rs` | 세대 4 부터 원본 구현의 기록 정의를 따른다(`avgjudge`, 게이지로 끝난 판의 BP, 코스 곡별 콤보, 풀콤, TOTAL 어시스트). 판정은 입력 이벤트 시각, 롱노트는 원본이 정리하는 대로 정리한 차트 위에서 한다 |
| 곡 파서 세대 `PARSER_VERSION` | 5 | `crates/rbms-library/src/songdb/schema.rs` | 세대 5 는 낱개 `\r` 로 줄을 끊고 `#TOTAL` 을 원본처럼 자른다. 저장된 열이 바뀌므로 올린 뒤 첫 실행은 곡 DB 를 다시 읽는다 |
| 설정 스키마 `CURRENT_SCHEMA_VERSION` | 6 | `crates/rbms-config/src/schema.rs` | 올라가면 첫 실행이 `settings.ron.v<이전>.bak` 을 남긴다 |
| KEY DEBOUNCE | 기본 16 ms, 0~100 | 설정 키 `play.key_debounce_ms` (`crates/rbms-config/src/schema.rs`) | 키보드의 LR2 형 디바운스 |
| PAD DEBOUNCE | 기본 16 ms, 0~100 | 키 설정 파일 `keyconfig.ron` 의 `pad.debounce_ms` | 컨트롤러의 LR2 형 디바운스. 설정 화면의 INPUT 탭에서 바꾸지만 값은 `settings.ron` 이 아니라 `keyconfig.ron` 에 있다 |
| 설정 행 수 `SETTING_COUNT` | 97 | `crates/rbms-config/src/settings.rs` | 2026-10-23 에 COURSE GAUGE, MISSING CHARTS, DOWNLOAD FOLDER, DISCORD STATUS 가 더해졌다. 길이 고정 배열(예 `apps/rbms-player/src/settings_ui.rs` 시험 고정표)이 함께 바뀐다 |
| 2026-10-23 설정 필드 | 스키마 6 그대로(`#[serde(default)]`, `skip_serializing_if` 없음) | `crates/rbms-config/src/schema.rs` | `play.scratch_auto_color`, `judge.course_gauge`(LR2 기본), `library.download_folder`(기기 고유, 업로드에서 뺌), `library.show_missing_charts`(켬), `network.discord_status`(켬), `library.tables[].group`(빈 값도 기록). 하나라도 바뀌면 `contract/client-requests/settings-blob.json` 을 `RBMS_UPDATE_CONTRACT=1 cargo test --locked -p rbms-player --lib a_settings_upload` 로 다시 만든다 |
| Discord Application ID | `1563156237539999865` | `apps/rbms-player/src/discord_presence.rs` 의 `DISCORD_APPLICATION_ID` | 비밀이 아니다. Art Asset 키 `rbms`(Developer Portal) |

## 실행 규칙

- 에이전트는 `rbms-player` 를 `HOME=$(mktemp -d)` 와 `--server http://127.0.0.1:9` 로만 실행한다. 사용자의 설정과 기록 DB 를 열고 운영 서버로 기록을 보내기 때문이다(2026-10-15 사고, [피드백](../feedback/2026-10-15-run-the-player-with-a-scratch-home.md)). 부작용 없이 끝나는 실행은 `--version`·`--help` 뿐이다.
- 시험 라이브러리의 경로와 내용은 [test-library.md](test-library.md).
- 운영 서버에는 GET 만 보낸다. 운영 DB 작업은 `web-db` 워크플로로만 한다([ci-release.md](../ci-release.md) 6.1절).

## 확인하지 못한 것

- S19 감사 중 `airTKkill`·`airTkill` 두 차트는 `#TOTAL` 이 1.1·1 이라 오토플레이 만점도 FAILED 이고 원본도 같다고 보았다(결함 아님). 이번 문서 점검에서는 라이브러리 안의 차트를 다시 읽지 않았다.
- S19 에서 라이브러리 전체의 오토플레이 만점을 센 도구(세션 한정 scratch 의 `sem`/`probe` 와 차트 목록)는 저장소에 남기지 않았다. 다시 하려면 새로 만들거나 저장소에 넣을지 정해야 한다([테스트 부채](../quality-assurance/2026-10-14-test-debt.md) 3.18).
