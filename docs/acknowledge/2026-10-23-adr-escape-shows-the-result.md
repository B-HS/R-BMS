# ADR: Esc 로 나간 판은 결과 화면을 거친다 (코스 도중이면 코스 결과까지)

> 최종 갱신 2026-10-23 · 대응 커밋 5a0eacf
> 상태: 유효(결정 9 는 사용자 결정, 세부 M9-1~M9-5 는 메인 결정이라 사용자 미확인) · 결정일 2026-10-23 · 결정자 사용자(결정 9) + 메인(세부) · 커밋 `9e39262`(제목은 마이그레이션 0012 이지만 웨이브 1 의 클라이언트 변경 전체가 들어 있다. 경위는 [커밋 사고 기록](../feedback/2026-10-23-stage-check-before-commit.md))
> 관계: [Esc 포기 판 ADR](2026-10-14-adr-abandoned-run-saved.md) 결정 1 의 "결과 화면 없이"와 기각 대안 "결과 화면을 거친다", [Esc 판 ADR](2026-10-15-adr-escape-keeps-every-run.md) 결정 1·2 의 "선곡으로 나감"과 기각 대안 "코스 Esc 때 코스 결과 화면"을 대체한다. 무엇을 저장·전송하는지(노트를 하나라도 쳤으면 모두, 하나도 치지 않았으면 아무것도)는 그대로다.

## 배경

- 사용자 요구 2(2026-10-23 원문, 아래): 노트를 하나 이상 치고 서버로 정보를 보내는 상황이면 Esc 로 대기화면이 아니라 결과 화면을 띄운다.
- 결정 전의 동작: Esc 는 `leave_run` 에서 결과 화면 없이 `record_left_run`(이력·리플레이 저장 + 그 판 하나만의 기록 동기화)을 하고 바로 선곡으로 나갔다. 코스 도중이면 `leave_course` 가 스테이지와 FAILED 코스를 남기고 선곡으로 나갔다. 두 경우 모두 알림 한 줄(`run left mid-song kept as FAILED`, `course left part-way kept as FAILED`)이 결과 화면을 대신했다.
- 앞선 두 ADR 은 "Esc 는 바로 나가는 키"라는 이유로 결과 화면을 기각 대안에 적었다. 이번 결정은 사용자가 그 판단을 뒤집은 것이다.
- 원본은 Esc 를 `stopPlay` 로 처리한다. 남은 노트가 있으면 `STATE_FAILED` 로 가고, 점수 데이터가 있으면 RESULT 로 간다(`BMSPlayer.java:693-742`). 점수 데이터는 코스가 아니고 PG·GR·GD·BD 합이 0 이면 만들지 않는다(`:858-861`). FAILED 로 닫히는 판은 게이지 로그의 꼬리를 0 으로 채운다(`:722-727`).

## 결정

1. **노트를 하나라도 친 판의 Esc 는 로그인과 상관없이 결과 화면으로 간다.** 닫힘 연출 없이 곧장 간다(조사 보고의 결정 지점 "진입 연출: 즉시 결과(추천: 변경 최소)"를 사용자가 "1-11 추천대로"로 받았다). PlayStop 소리는 Esc 를 누른 자리에서 울리고 곡 소리는 `stop_chart_sounds` 로 끊는다. 서버가 없거나 로그인하지 않았으면 결과 화면의 전송이 건너뜀(`IrStatus::Off`)이 되고 DB 에는 늘 남는다.
2. **결과 화면이 보이는 값은 저장되는 값과 같다.** 남은 노트는 늦은 POOR 로 센 FAILED 요약(`PlaySummary::left_unfinished`)을 화면의 판정 수, 이력 행, 전송 본문, 스킨 결과 화면이 읽는 기록(`run_records.last()`)이 모두 쓴다. 결과 화면 그래프의 게이지 이력은 나간 자리부터 끝까지 0 으로 채운다(`PlaySession::left_record`, 원본 `:722-727` 과 같음). 게이지 사망으로 끝난 판은 지금처럼 사망 시각부터 채운다.
3. **이력·리플레이·전송은 결과 화면 경로 하나다.** `record_left_run`·`send_without_a_screen` 호출(이중 전송 경로)을 이 갈래에서 없앴고, 알림 두 개와 `leave_course` 를 지웠다. 판정 trace 는 끝까지 친 판과 같이 서버가 점수를 받은 뒤 기록 동기화가 보낸다.
4. **코스 도중 Esc 는 그 스테이지를 FAILED 로 접어 코스 결과(불합격)까지 간다.** 스테이지 기록은 결과 화면 전송이, 코스 기록은 `finish_course` 가 한 번만 남기고 보낸다(`keep_finished_course(Sending::Now)`).
5. **바뀌지 않는 것.** 오토플레이·리플레이 보기·연습의 Esc(결과 화면 없음), 노트를 하나도 치지 않은 판([Esc 판 ADR](2026-10-15-adr-escape-keeps-every-run.md) 결정 7: 선곡 또는 `abandon_course`), 코스의 스테이지 사이에서 나감, 앱 종료는 지금대로 화면 없이 남긴다. 닫히는 중·끝난 판의 Esc 는 원래부터 결과 화면이다.

### 메인 결정(설계서 client-small 8절, 사용자 미확인)

| id | 결정 | 이유 |
| --- | --- | --- |
| M9-1 | 오토플레이·리플레이 보기 판의 도중 Esc 는 지금처럼 선곡 | 결정 9 의 대상은 사람이 친 판이고 이 판들은 기록이 없다 |
| M9-2 | 친 노트 0 인 코스 스테이지의 Esc 는 지금처럼 앞 스테이지까지의 FAILED 코스를 남기고 선곡(코스 결과 화면 없음) | 결정 9 "노트를 하나도 치지 않은 판은 지금처럼" |
| M9-3 | Esc 로 온 결과 화면의 다시 하기(R, N, 파란 건반 + START)를 허용 | 원본 결과 화면도 받는다. `offers_retry` 를 바꾸지 않았다 |
| M9-4 | 알림 두 개를 지움 | 결과 화면이 판이 남았음을 보인다 |
| M9-5 | Esc 판의 게이지 그래프를 나간 자리부터 0 으로 채움 | 원본이 FAILED 로 닫는 모든 판에 0 을 채운다 |

## 이유

- 사용자 요구와 결정 9. 저장·전송하는 판의 결과를 사용자가 보지 못하는 상태를 없앤다. 조사 보고([stella-course-dan-survey](../reference/2026-10-23-stella-course-dan-survey.md) 2번)의 결정 지점 중 결과 화면 조건(로그인 무관), 코스 도중 Esc(FAILED 코스 결과까지), 진입 연출(즉시), 판정 수(저장값과 같게)는 모두 사용자가 "추천대로"로 정했다.
- 원본이 Esc 판에 결과 화면을 주므로 같아진다.
- 화면의 판정 수가 저장값과 다르면 결과 화면과 기록 목록이 어긋난다. 그래서 요약을 한 곳에서 접어 모든 소비자가 같은 값을 읽는다.
- 전송 경로를 하나로 하면 이중 전송과 "화면 없는 전송"의 별도 보고 경로가 사라진다. 계약 본문 `score-abandoned.json` 은 바뀌지 않았다.

## 기각된 대안

| 대안 | 기각 이유 |
| --- | --- |
| 지금처럼 바로 나가고 알림만 (2026-10-14·15 의 판단) | 사용자가 결과 화면을 요구했다 |
| 원본식 FAILED 닫힘 연출을 거친 뒤 결과 화면 | 조사 보고의 추천이 즉시 결과(변경 최소)였고 사용자가 추천대로 정했다. 플레이 장면에는 남은 노트가 있을 때 Esc 로 Failed 로 닫는 길이 없어 연출을 넣으려면 장면을 고쳐야 한다 |
| 결과 화면 전송과 기록 동기화를 모두 시작 | 같은 판이 두 번 간다 |
| 오토플레이·리플레이도 결과 화면(원본 동작) | 기록이 없는 판이라 M9-1 로 제외 |
| 게이지 그래프를 나간 자리에서 끝냄 | 게이지 사망 판과 모양이 달라 어디서 멈췄는지 안 보인다(M9-5) |

## 영향과 관련 파일

- `apps/rbms-player/src/stage/play/mod.rs`(`leave_run`: 연습 → 끝났거나 닫히는 판은 `enter_result` → 남기지 않는 판 → 친 노트 0 → `enter_result_of_a_left_run`), `apps/rbms-player/src/app_result.rs`(`RunEnd`, `show_result`, `enter_result_of_a_left_run`; `record_left_run`·`leave_course`·`LEFT_RUN_KEPT`·`LEFT_COURSE_KEPT` 삭제), `crates/rbms-play/src/{session,record}.rs`(`PlaySession::left_record`, `GaugeLogRecorder::closed_left`). 장면 경로(`scene_escape`)와 길게 누르기·두 번 누르기 경로는 모두 `leave_run` 을 거쳐 따로 고친 곳이 없다.
- 테스트: `app_result/record_tests.rs`(`leaving_mid_song_shows_a_failed_result_with_the_counts_it_keeps`, `leaving_mid_song_signed_in_sends_the_run_once_from_its_result`, `leaving_mid_song_signed_out_still_shows_its_result`, `leaving_mid_course_ends_on_a_failed_course_result`), `stage/play/screen_tests/run_tests.rs`(`escape_with_notes_left_on_a_skinned_run_goes_straight_to_its_result`), `crates/rbms-play/src/record/tests.rs`(`a_history_closed_as_left_falls_to_zero_from_where_it_stopped`, `a_record_left_part_way_counts_the_rest_as_poor`), 계약 `app_result/record_tests/contract_requests.rs`(`a_run_left_mid_song`).
- 설계서와 다른 점(구현과 다름, 설계서 `docs/plan/2026-10-23-client-small.md` 3.2.2절): 설계는 `record_run` 도 `left_record` 를 쓴다고 적었으나 구현은 이력 행과 전송 본문에 샘플된 그대로의 게이지 이력을 쓰고(`record_run` 에는 `play.record()`), 0 으로 채운 이력은 결과 화면용 `keep_run_record` 에만 쓴다. 판정 trace 도 샘플된 그대로다. trace 가 실패 시각 없음과 닿지 못한 노트로 도중에 나간 판임을 이미 말한다.
- 알려진 차이: 결과 화면의 판정 분포 그래프와 스킨의 초별 판정 표는 "판정된 입력" 기준이라 남은 노트의 POOR 를 넣지 않는다. 끝까지 친 판에서도 같은 기존 의미라 고치지 않았다. 판정 수 칸·DB·전송은 접은 값이다.
- 확인하지 못한 것: Esc 직후 곡 소리가 실제로 멈추는지와 PlayStop 이 한 번만 나는지(헤드리스 테스트에 오디오 장치가 없다. QA 실기 항목), 실제 창에서 본 Esc 체감.
- 이 ADR 이 대체한 앞선 두 ADR 의 상태 줄에 "일부 대체됨"을 적었다. 색인(`README.md`)의 상태 갱신은 이 ADR 을 쓴 작업의 범위 밖이라 하지 않았다.

## 날짜와 사용자 원문

2026-10-23.

- 요구 2(원문): "플레이 도중에 (노트를 하나 이상 친 상황)에 esc 를 누르면 그냥 바로 대기화면으로 넘어가버리는데 하나라도 쳤고, 서버로 정보를 보내는 상황이면 result 화면을 띄워야해"
- 결정 회신(원문): "1-11 추천대로". 결정 9 의 요지(`docs/PROCESS.md` 의 요약, 원문 아님): 노트를 1개 이상 친 판의 Esc 는 로그인과 상관없이 결과 화면, 코스 도중 Esc 는 FAILED 코스 결과까지, 결과 판정 수는 저장값과 같게.
