# AI 기능 10개를 위한 클라이언트 데이터 조사 (2026-10-19, 조사 전용)

> 최종 갱신 2026-10-19 · 대응 커밋 1eadbb5
> 이 문서는 조사 보고서 전문이다(줄이지 않았다). 끝에 조사자가 함께 낸 "제약"과 "확인하지 못한 것" 목록을 원문 그대로 붙였다. 기획은 [../../plan/2026-10-19-web-ai-features.md](../../plan/2026-10-19-web-ai-features.md), 색인은 [README.md](README.md).
> 4절의 평균 노트 수(1,500/3,000)는 이 조사자의 가정이다. 발광 표 1,035곡의 실제 노트 수 분포(중앙값 2,114, 최대 5,000)는 웹 서버 조사([server.md](server.md) 3.3절)가 기존 `rbms-cli` 바이너리로 측정했다.

범위: 코드·Git 상태·운영 사이트·.env·settings.ron·scoredb.sqlite 를 건드리지 않고 읽기만 했다. 차트 라이브러리 경로는 `docs/memory/test-library.md` 만 읽었고 실제 추출은 하지 않았다. 라이브러리의 실제 노트 수는 한 번도 재지 않았다(4절의 크기는 가정이 들어간 추정이다). 아래 `파일:줄` 은 모두 `/Users/hyunseokbyun/development/R-BMS` 기준 2026-10-19 작업 트리다.

핵심 결론 5줄
1. 판이 끝날 때 클라이언트는 사용자가 원하는 거의 모든 것을 이미 메모리에 갖고 있다. 단 노트별 판정(`JudgeEngine::note_marks`, `judged`)과 게이지 이력(`PlayRecord`)은 결과 화면용으로만 쓰이고 어디에도 저장·전송되지 않는다(`app_result.rs:635-641`). 하이스피드·커버·리프트·판정 오프셋은 판 도중 키로 바뀌는데 시작값과 변경 이력은 기록하지 않고, 제출 본문은 끝값만 담는다.
2. 새 필드를 점수 본문에 얹는 것보다 새 엔드포인트(리플레이 업로드와 같은 모양, `score_id` 로 묶음)가 낫다. 이유는 2절. 계약 교차 테스트에는 요청 종류 하나(`trace`)를 더하는 일이 되고 기존 15개 파일은 바뀌지 않는다.
3. 곡별 추출은 `rbms-cli` 하위 명령으로 가능하다. 기존 경로(`rbms_parser::parse` → `detect_mode` → `to_model`)는 소리·BGA 파일을 읽지 않고 이름만 들고 있으며, md5·sha256 은 플레이어·라이브러리 스캔과 같은 함수가 만든 같은 값이다. 발광 표 `data.json` 에는 노트 수·sha256 이 없고 md5 만 있다.
4. 한 판의 노트별 데이터는 촘촘한 이진 약 4~5.5 B/노트, JSON 배열 약 20 B/노트(가정)다. 5,000 노트 판은 JSON 약 100 KB 라서 점수 본문 한도(256 KiB)에 가깝고, 5만 노트 차트는 넘는다. 별도 엔드포인트의 한도가 필요하다. Vercel 함수 요청 본문은 4.5 MB 가 절대 상한이다.
5. 패턴 특징은 서버(TypeScript)에서 원자료(배치+판정)로부터 계산하는 쪽을 권한다. 클라이언트 릴리스 주기가 느리고 분류 정의는 자주 바뀔 것이기 때문이다. 단 RANDOM·S-RANDOM 등이 인접성·연타(잭) 계열 특징을 바꾸므로 사용자별 분석은 반드시 "실제로 보인 배치"에서 계산해야 한다.

---

## 1. 판이 끝날 때 클라이언트가 아는 것

### 1.1 옵션 적용 뒤 실제 배치

흐름은 `AppShared::load`(`apps/rbms-player/src/app_play.rs:422-585`)에 있다.
- 차트 읽기 `decode_chart`(`:431`, 정의 `:181-193`) → 롱노트 종류 확정 `rbms_chart::resolve_long_note_flavour(&mut model, judge_setup.ln_mode.resolve())`(`:472`) → `lay_out_notes(&mut model, random, seed, bmson, &mode)`(`:486`, 정의 `:135-149`) → TOTAL 덮어쓰기(`:490-493`) → `PlaySession::new(model, options)`(`:582`).
- `lay_out_notes` 는 `rbms_chart::shuffle::apply(model, random, seed)`(`:142`)와 TOTAL 정리(`:143-148`)만 한다. 이 시점의 `Model.timelines` 가 "옵션 적용 뒤 배치"다.
- SCRATCH AUTO 레인 마스크는 `scratch_auto_lanes(mode, scratch_auto) -> Vec<bool>`(`:153-158`, 호출 `:564`). 판정 엔진이 보는 차트에서는 그 레인의 노트가 빠지고(`take_auto_lanes`, `crates/rbms-play/src/lib.rs:94-125`), 그리는 쪽에는 남는다.

자료형과 접근 경로
- `Model { mode, meta, init_bpm, timelines: Vec<TimeLine>, md5, sha256 }`(`crates/rbms-model/src/lib.rs:154-164`). `TimeLine { time_us: i64, section: f64, notes: Vec<Option<Note>>, hidden, bgnotes, bpm: f64, stop_us: i64, scroll: f64, ... }`(`:101-114`). 레인 하나에 타임라인당 노트 하나(`Vec<Option<Note>>`, 인덱스가 논리 레인).
- `Note { kind: NoteKind, wav: i32, time_us: i64, section: f64, ... }`(`:84-93`), `NoteKind::{Normal, LongStart{ln}, LongEnd{ln}, Mine{damage}}`(`:76-82`), `LnKind::{Ln, Cn, Hcn, Undefined}`(`:68-74`).
- 판이 끝난 뒤에는 `PlaySession::field_timelines() -> &[TimeLine]`(`crates/rbms-play/src/session.rs:997`)이 "그려지는 차트"(자동 레인 노트 포함), `PlaySession::model()`(`session.rs:991`)이 "판정되는 차트"(자동 레인 노트 제외)다(`rbms-play/src/lib.rs:388-396`).
- 배치를 정하는 값: `PlaySession::seed() -> u64`(`session.rs:1034`), 옵션 8종 `NoteOption`(`crates/rbms-chart/src/shuffle.rs:6-15`). 난수는 손으로 만든 xorshift64(`shuffle.rs:78-97`)라 같은 차트·옵션·시드는 같은 배치다.
- 제출 본문에 이미 있는 것과 없는 것: `options.random` 은 `shared.config.play.random`(`app_result.rs:281`), `seed` 는 `play.seed()`(`:303`). `random_p2` 는 항상 `None`(`:282`), `option` 은 0(`:287`), `input_device` 는 `"keyboard"` 고정(`:285`). 즉 DP 두 번째 쪽 옵션과 컨트롤러 입력은 구분되지 않는다.
- 레인 배치로 표현되는 정도: MIRROR·RANDOM·R-RANDOM·ROTATE 는 한 판 전체의 레인 순열 하나(`lane_permutation`, `shuffle.rs:115-144`, `perm[old]=new`, 스크래치는 항등, DP 는 쪽별로 따로). S-RANDOM(`:185-232`)은 타임라인 행마다 새 순열(진행 중 롱노트는 고정), H-RANDOM·ALL-SCRATCH(`apply_time_based`, `:235-`)는 행마다 새 배정이라 하나의 순열로 표현되지 않는다. 그래서 서버가 시드로 재현하려면 시프트·피셔-예이츠·행별 로직을 TS 로 옮겨야 한다. 사용자 결정 (a)(실제 보인 배치를 올림)가 이 이식을 피한다.
- 보태서 필요한 값: 롱노트 종류(`PlayState.lntype`, `ln_mode_key`, `stage/play/mod.rs:224-228`), 자동 레인(`session.auto_lanes()`), 차트가 `#LNMODE` 를 말했는지(`rbms_chart::contains_undefined_long_note`, `crates/rbms-chart/src/lib.rs:398`).

### 1.2 노트별 판정·시각·오차

`crates/rbms-judge/src/matcher.rs`
- `JudgeEngine::note_marks() -> impl Iterator<Item = NoteMark>`(`:765`). `NoteMark { lane: usize, time_us: i64(차트 시각), long_end: bool, charge: bool, state: u8, play_time_us: i64 }`(`:130-145`). `state` 는 0=아직 판정 없음(도달 못함), 판정 코드+1(1=PGREAT … 5=見逃し POOR). `play_time_us` 는 부호 있는 오차로 양수가 빠름(early)이고 플레이어 시간 기준(`update_micro` 의 `real_span_us`, `:1408`). 입력 시각은 `time_us - play_time_us` 로 복원된다. 순서는 레인별·차트 순서이고 롱노트는 머리·끝 두 개(`marks_of`, `:268-273`). 일반 LN 은 머리 객체에 한 번만 판정이 남고 끝은 state 0(`:123-128, 136-138`). CN/HCN 은 머리·끝 각각.
- `note_marks_at(NoteAt{lane,index})`(`:771`)는 레인 안 몇 번째 머리인지로 찾는다.
- `JudgeEngine::judged() -> &[JudgedNote]`(`:784`). `JudgedNote { lane, judge: Judge, delta_us: i64, combo: u32 }`(`:163-172`). 센 판정 전부를 센 순서대로 담고, 空POOR(`Judge::Miss`)도 들어 있다. 어느 노트였는지와 노트 시각은 없다.
- `vanished() -> &[Judge]`(`:793`, 쌓는 곳 `:1413`)는 노트를 소비한 판정만 소비 순서대로 담는다(원본 ghost). `total_judged()`(`:755`). 기록 목표 고스트가 이것을 쓴다(`apps/rbms-player/src/ghost.rs:238-260`).
- 빠름/늦음: `delta >= 0` 이 빠름(`matcher.rs:1461`, `session.rs:475`). 판정별 빠름/늦음 합계는 `JudgeEngine.early/late: [u32;6]`(`:371-372`) → `PlaySummary.early/late`(`session.rs:538-539`).
- 한 판의 판정 합계: `PlaySession::summary() -> PlaySummary`(`session.rs:843`, 구조체 `:525-557`).
- 結果 가공본: `PlaySession::record() -> PlayRecord`(`session.rs:872`, `crates/rbms-play/src/record.rs:421-446`)에 `timing: TimingDistribution`(1 ms 버킷, `:252`), `avg_duration_us`, `seconds: JudgeSeconds`(초별 판정, `:316`)가 있다.
- 지금 어디에도 저장되지 않는다: `keep_run_record`(`app_result.rs:635-641`)가 메모리 `shared.run_records` 에만 둔다. 화면용 `RunTrace`(`stage/play/trace.rs`)도 화면 상태다(새 구조체 이름은 이것과 겹치지 않게 해야 한다).
- 중단한 판: `PlaySummary::left_unfinished()`(`session.rs:577-588`)가 합계에 남은 노트를 POOR 로 더하지만 `note_marks` 의 `state` 는 0 그대로다. 서버가 "도달 못함"과 "놓침"을 구분하려면 마크 기준으로 보내야 한다.

### 1.3 게이지 추이

- `PlayRecord.gauge_log: GaugeLog`(`record.rs:134`): 9개 게이지 전부, 500 ms 간격(`GAUGE_LOG_INTERVAL_MS`, `record.rs:21`), `Vec<f32>`. 게이지가 빈 판은 실패 시점부터 0 으로 곡 끝까지 채운다(`closed()`, `record.rs:213-227`). 함께: `gauge_bounds`(min·max·border), `finished_gauge: GaugeIndex`, `failed_at_ms: Option<i64>`, `play_time_ms`.
- 내장 화면용 `PlayInstrumentation::gauge_series()`(`session.rs:429`)는 선택 게이지만 1초 간격, 최대 1,800개(`session.rs:372-377`).
- 지금 제출은 끝값 `gauge_value` 하나뿐(`app_result.rs:278`).

### 1.4 입력 이벤트

- `PlaySession::recorded_events() -> &[ReplayEvent]`(`session.rs:1044`). `ReplayEvent { t: i64(µs, 오프셋 없음), lane: usize, press: bool, backward: bool }`(`crates/rbms-store/src/replay.rs:17-22`). 기록은 `press_dir`/`release_dir`(`session.rs:716-742`).
- 리플레이 파일은 `save_replay` 일 때만 쓴다: AUTO REPLAY 켬 + 연습·리플레이 감상·오토플레이 아님 + 이벤트 있음(`app_result.rs:232-234`).
- 와이어 `rbms_ir::ReplayEvent { t_us, lane: u32, press }` 에는 `backward` 가 없다(`crates/rbms-ir/src/dto/replay.rs:9-14`, `ir_replay.rs:62-67`).

### 1.5 하이스피드·녹색 숫자·커버·리프트·판정 오프셋

- 값은 `shared.config.play.{hispeed, constant_speed, cover, lift, hidden, duration_ms}` 와 `config.judge.offset_ms` 에 있고 판 도중 키·휠·턴테이블로 바뀐다(`stage/play/controls.rs:373`, `:441-520`; 오프셋 `stage/play/mod.rs:602-604`, 자동 보정 `session.rs:778-788`).
- 제출 본문은 판이 끝날 때의 끝값만 복사한다(`app_result.rs:288-298`): `hispeed` f64, `lift`·`lane_cover` f32, `constant`, `offset_ms`, `auto_offset`, `scratch_left`, `green_number` f64. `hidden` 은 `PlayOptions`(`crates/rbms-ir/src/dto/chart.rs:113-145`)에 없다. 시작값과 변경 이력은 어디에도 없다. `PlayState.speed: Option<RunSpeed>`(`stage/play/mod.rs:240`)가 판 시작의 속도 상태를 갖지만 제출하지 않는다.
- 주의(조언 내용에 직접 영향): 제출의 `green_number` 는 `green_number_for(constant, init_bpm, hispeed, 1.0, cover)`(`apps/rbms-player/src/lib.rs:234-240`) 즉 초기 BPM 에서의 `currentduration`(레인 통과 시간 ms, `(240000/bpm/hispeed)/scroll*(1-cover)`, `crates/rbms-chart/src/scroll.rs:40-45`)이다. 스킨이 GREEN 으로 표시하는 값은 여기에 0.6 을 곱한 것이라(`scroll.rs:30-33`) 다르다. CONSTANT 는 rbms 고유 보정 `CONSTANT_GREEN_BASE_MS = 2000`(`scroll.rs:25, 50-55`)이다. 서버 컬럼 `score.green_number` 는 정수로 반올림해 저장한다(`web/src/server/service/domain/score/score.service.ts` `greenNumber`).
- 오프셋 의미: JUDGE OFFSET 은 노트를 그리는 시각만 옮기고 판정은 입력 시각 그대로다(`docs/play-options.md` 3.1). 그래서 노트별 `play_time_us` 의 평균(치우침)이 곧 체감 타이밍 습관이고, 오프셋 조언은 "그려지는 위치를 옮기는 값"으로 말해야 한다. 자동 보정을 켠 판은 오프셋이 입력마다 1~5 ms 움직이므로(3.2) 시작값과 끝값을 모두 알아야 습관을 분리한다.

### 1.6 BPM 변화

- 타임라인마다 `bpm`, `stop_us`, `scroll`(`rbms-model/src/lib.rs:101-114`)이 이미 풀려 있고 `time_us` 는 `assign_times`가 BPM·STOP·SCROLL 을 적분해 정한다(`crates/rbms-chart/src/lib.rs:350-383`, `time = prev.time + stop + 240e6*Δsection/prev.bpm`). 결과 화면 BPM 그래프용 요약 `tempo_points`는 `app_result.rs:649-666` (BPM 바뀌는 지점만 `(진행률, bpm)`).
- 실제 속도감은 BPM·scroll 과 하이스피드의 곱이므로 구간별 녹색 숫자는 이 값들로 결정론적으로 계산된다. 원본(`#RANDOM`) 상태와는 무관하다.

### 1.7 그 밖의 확인

- 제출 본문 `judge`(22칸), `passnotes`, `minbp`, `gauge_value`, `judge_algorithm`, `skin`, `extra { mean_offset_us, rule_version }`(`app_result.rs:249-312`, `crates/rbms-ir/src/dto/score.rs:50-93, 98-106`). 판 전체 집계에는 충분하지만 노트·구간 단위 정보는 없다.
- 판 종류별 호출 지점은 모두 `record_run`(`app_result.rs:216`)을 지난다: 결과 화면(`enter_result :82`), Esc 로 나간 판(`record_left_run :461`), 앱 종료(`keep_run_cut_short_by_quitting :517`), 실패 뒤 곧장 재시작(`record_retried_run :596`). 새 데이터를 만드는 자리는 여기 한 곳이면 된다.
- 재판정 선례: 저장된 리플레이를 화면·소리 없이 다시 판정하는 코드가 이미 있다(`ghost.rs:238-260 rejudge`, `PlaySession::play_out` `session.rs:822-825`, `docs/reference/record-target-pace.md` 4절). 이것으로 과거 판의 노트별 판정을 소급 생성할 수 있지만 `lay_out_notes`·`replay_judge_setup` 이 `apps/rbms-player` 안에 있어(`ghost.rs:25-26`) CLI 에서 쓰려면 라이브러리 크레이트로 옮겨야 한다. 옛 FRAME 시계 리플레이는 판정 재현을 보장하지 않아 쓸 수 없다(`record-target-pace.md` 4.2 의 3). 소급 대상으로 알려진 것은 대기 중 리플레이 43개(`HANDOFF.md:87`) 정도다.

---

## 2. 지금 제출하는 것과 더하는 방법

### 2.1 현재 요청 4종과 본문 크기

계약 교차 테스트가 아는 종류는 `score`, `course`, `replay`, `settings`(`web/tests/dto/client-requests.test.ts:25`)이고 파일은 15개(`contract/client-requests/`: 점수 10, 코스 2, 리플레이 2, 설정 1; Rust 쪽 목록 `contract_requests.rs:35`).
- 점수 본문 `ScoreSubmission`(`crates/rbms-ir/src/dto/score.rs:50-93`)은 약 1.9 KB(`score-normal.json` 1,905 B). 서버 스키마 `scoreSubmissionSchema`(`web/src/server/dto/score.ts:48-72`). 본문 한도 `SUBMISSION_MAX_BYTES = 256 * 1024`(`web/src/server/route/score.ts:16`), 분당 60회(`web/src/server/lib/with-rate-limit.ts:6`). `extra` 는 `HashMap<String, Value>` 이고 서버는 깊이만 검사해 `score.extra` JSON 컬럼에 그대로 저장한다(`dto/common.ts:114-117`, `score.service.ts:202`).
- 리플레이 `POST /charts/{hash}/replays`(`crates/rbms-ir/src/http.rs:184-187`)는 입력 이벤트만 싣는다. 이벤트 한 개 minified JSON 이 평균 40.4 B(`replay-long.json` 1,596 이벤트에서 측정, 파일은 들여쓰기 포함 90,711 B). 본문 한도 `REPLAY_MAX_BYTES` 기본 4 MiB(`web/src/server/lib/env.ts:4, 17`), 사용자당 2,000개·512 MiB(`web/src/server/dto/replay.ts:13`), 분당 10회(`with-rate-limit.ts:7`), 저장은 `replay.data` mediumtext 의 JSON 문자열(`web/src/server/db/schema.ts` replay). 요청 스키마는 `extra` 를 받지만 서비스가 저장하지 않고 응답에서 `extra: {}` 로 돌려준다(`replay.service.ts:104`). 즉 `ReplayData.extra` 에 담아 보내도 쓸모없다.
- 리플레이는 점수가 수락돼 `score_id` 를 받은 뒤 같은 `score_id` 로 업로드한다(`crates/rbms-ir/src/worker.rs:91-111`, 기록 동기화는 `apps/rbms-player/src/record_sync.rs:559-606`, 상태는 `ir_sync.replay_state`, `crates/rbms-store/src/scoredb/schema.sql:130`). 또 `wire_carries_judge`가 거짓이면(균일하지 않은 판정 폭, LN MARGIN, TOTAL 직접 설정) 리플레이는 올리지 않는다(`ir_replay.rs:98-103`). 노트별 데이터는 이 조건에 묶이면 안 된다.
- 요청 제한 시간은 5초(`crates/rbms-ir/src/http.rs:14`), 폴더 요청만 30초(`:19`).

### 2.2 새 필드 vs 새 엔드포인트 (권고: 새 엔드포인트)

점수 본문에 얹을 때의 문제
- 점수 본문은 그대로 `play_submission.body`(`schema.sql:112-116`)에 저장되고 기록 동기화가 그 본문을 반복 전송한다. 수 KB~수백 KB 가 더해지면 같은 본문이 매번 오가고, 256 KiB 한도를 넘는 판(JSON 약 13,000 노트 이상, 4절)은 점수 자체가 413/400 으로 막힌다. 사용자 원칙(`HANDOFF.md:75`: 사람이 친 판은 실패·포기 포함 전부 저장·전송)에 반한다.
- 순위·상세 질의가 `score` 행을 읽는 곳이 많아 JSON 컬럼이 커지면 행 크기가 늘어난다(읽는 컬럼 선택 여부는 확인하지 못했다).
- 계약 테스트: 필드를 점수 스키마에 추가하면 `score-*.json` 10개와 `course-*` 와 무관하게 점수 파일이 전부 재생성되고, 서버 스키마에 없으면 "서버가 버림"으로 실패한다(`client-requests.test.ts:219-226`, `changedPaths :90-104`).

새 엔드포인트의 장점
- 점수 수락과 독립. 실패해도 점수는 이미 서버에 있다.
- 리플레이와 같은 순서(점수 → `score_id` → 부속)를 재사용한다. 기록 동기화가 이미 "수락된 판에 부속을 올리고 상태를 `ir_sync` 에 기록하고 거절은 빌드가 바뀔 때만 다시 보낸다"를 한다(`docs/play-options.md` 10.9). 부속 상태 열(`trace_state`, `trace_refused_under`)은 `ADDED_COLUMNS`(`crates/rbms-store/src/scoredb/schema.rs:60`)로 더하면 되고, 새 표(`play_trace`)를 더하는 일은 스키마 세대(`SCORE_DB_SCHEMA_VERSION = 1`, `schema.rs:12`)를 올리지 않는다(`play-options.md` 10.9 의 표·열 추가 주석).
- 옛 서버: 알 수 없는 경로는 404 → `IrError::NotFound` → `Failure::Refused`(`crates/rbms-ir/src/sync.rs:122-127`) → "거절" 로 기록되고 서버 빌드가 바뀌면 자동으로 다시 보낸다(10.9 "대상"). 옛 클라이언트: 아무것도 보내지 않으므로 서버는 부속을 선택 사항으로 취급해야 하고 AI 기능은 "데이터 없음"으로 처리한다.
- 별도 한도·속도 제한·보존 정책·동의(옵트인) 설정을 둘 수 있다.

권고하는 모양(기획 수준)
- `POST /charts/{hash}/traces` 에 `score_id`(리플레이의 `resolveReplayChartSha256`과 같은 차트 해시 해석, `web/src/server/route/replay.ts:23-31`), 같은 `score_id` 에 한 번만 저장(`findLinkedReplay` 선례, `replay.service.ts`).
- 본문(자기완결형): 형식 태그·버전, 차트 md5·sha256, `mode`, `lntype`, `random`·`seed`, 자동 레인 마스크, 노트 배열(차트 시각 Δ, 실제 레인, 종류), 판정 배열(`state`, 오차 ms), 선택 게이지의 0.5초 열(필요하면 9개 전부), `failed_at_ms`, 옵션 시작값·끝값·변경 이벤트(하이스피드·커버·리프트·히든·오프셋), 타임라인 BPM·STOP·SCROLL 요약. 자기완결형으로 하면 (b) 차트 패턴이 없는 차트(발광 표 밖)에서도 분석이 된다.
- 로컬에는 `play_submission` 처럼 판과 한 트랜잭션으로 보관해야 한다(`ScoreDb::record_play_kept`). DB 를 못 열 때의 `pending-runs/*.ron` 폴백(`PendingRun`, `app_result.rs:383`)에도 같은 부속을 실어야 한다.

### 2.3 계약 교차 테스트에 주는 영향

새 엔드포인트(권고안)
- 웹: `CONTRACT_KINDS`(`client-requests.test.ts:25`)에 `trace` 추가 → `REQUEST_SCHEMA`(`:26-31`), `FLOAT_FIELDS`(`:32`), `NULLISH_FIELDS`(`:38`), `DROPPED_FIELDS`(`:44`)가 `Record<ContractKind, ...>` 라 typecheck 가 빠짐없이 채우게 강제한다. "네 종류의 요청이 모두 하나 이상 있다"(`:182-184`)가 다섯 종류로 바뀐다. 서비스 단계 `describe`(리플레이 `:323-355` 모양)를 추가한다.
- Rust: `CONTRACT_FILES: [&str; 15]`(`contract_requests.rs:35`)를 늘리고, `Kind` enum(`:258`)의 `of`·`nullable`·`fix_the_moment_and_the_machine`에 가지를 더하며, 테스트 `the_contract_folder_holds_every_request_and_nothing_else`(`:392`)가 목록과 폴더를 맞춘다. 캡처 서버(`wire.rs`)가 새 경로 `POST /charts/{hash}/traces` 에 응답해야 한다. 파일은 `trace-*.json` 새로 생성(`RBMS_UPDATE_CONTRACT=1`).
- 기존 15개 파일은 그대로다. 웹 CI 가 `contract/**` 변경에도 돈다(`HANDOFF.md:104`).

점수 본문에 필드를 더하는 안
- 점수 파일 10개와 `score-sync-resend`(재구성 본문)가 모두 바뀐다. 재구성 경로(`record_sync::rebuilt_submission`, `record_sync.rs:726`)는 노트별 데이터를 만들 수 없어 그 판은 부속이 없다.

### 2.4 배포 순서와 옛 클라이언트 호환

- 새 웹 코드가 새 마이그레이션에 기대면 마이그레이션 파일을 먼저 커밋·적용하고 코드를 나중에 푸시한다(`HANDOFF.md:81`, 0004 선례). 새 표는 마이그레이션 0005 가 된다(현재 0000~0004, `docs/architecture.md:117`).
- 순서: 마이그레이션 → 웹 배포 → 클라이언트 릴리스. 서버는 부속이 없는 판을 계속 받는다.

---

## 3. 곡별 패턴 추출 (`apps/rbms-cli`)

### 3.1 CLI 구조와 하위 명령 추가 방법

- `apps/rbms-cli/src/main.rs`: 사용법 상수 `USAGE`(`:16-24`), `run()`이 첫 인자로 분기(`:38-65`; `scan`, `config`, `scores`, `skin-dump`, 그 밖은 차트 경로), `usage_error`(`:67-70`, 종료 코드 2). 하위 명령 하나 = `match` 가지 하나 + `*_command` 함수 + 필요하면 `parse_*_args` + 같은 파일 하단 테스트 모듈(`:298-`). 큰 명령은 `skin_dump.rs` 처럼 모듈로 분리(`parse_args`, `run`). 테스트 선례: 임시 폴더에 차트를 쓰고 명령을 호출(`scan_rows_reads_song_rows_from_the_temporary_database`, `chart_command_reads_a_bmson_document`).
- 의존: rbms-model, parser, chart, config, library, store, skin, serde, serde_json(`apps/rbms-cli/Cargo.toml`). 판정 엔진·IR·표는 아직 의존에 없다. 이 앱은 "플레이어에 의존하지 않고 창·오디오를 끌어오지 않는다"는 원칙이다(`docs/crates.md:565-566`). 노트 배치만 뽑는 데는 지금 의존만으로 충분하다. 발광 표 대조에 `rbms-table`(워크스페이스 크레이트, 외부 의존 없음)을 더하는 것은 새 외부 의존이 아니지만 `docs/architecture.md:29` 의 표를 고쳐야 한다.
- HANDOFF 8절 4번이 "오토플레이 만점 검사를 저장소 도구로 남긴다면 `apps/rbms-cli` 하위 명령이 자연스럽다"고 이미 적어 둔다(`HANDOFF.md:120`).
- 네트워크: CLI 에는 HTTP 클라이언트가 없다(reqwest 는 `rbms-ir` 만). 추출 결과를 파일로 쓰고 별도 업로드(웹 쪽 스크립트나 관리자 임포트 경로)로 올리는 편이 가볍다. 새 서버 경로의 인증은 API 토큰 사용자에게 `role` 이 있어(`web/src/server/service/domain/auth/auth.service.ts:81-83`) 관리자 확인이 가능하다. 기존 관리자 라우트는 세션 쿠키 방식이다(`web/src/server/lib/with-session.ts:24`).

### 3.2 차트 파일 → 노트 배치로 가는 경로

플레이어(`decode_chart`, `app_play.rs:181-193`)·라이브러리 스캔(`parse_chart`, `crates/rbms-library/src/scan.rs:301-352`)·CLI 의 단일 차트 명령(`main.rs:260-296`)이 모두 같은 호출을 한다.
1. `rbms_parser::parse(&bytes)`(`crates/rbms-parser/src/lib.rs:131`, 플레이어는 `parse_with(bytes, Default::default())`로 같다).
2. `rbms_chart::detect_mode(&src, path)`(`crates/rbms-chart/src/lib.rs:35-60`): `.pms` 는 POPN_9K, P2 채널이 있으면 BEAT_14K, 6·7키 채널이 있으면 BEAT_7K, 아니면 BEAT_5K. 파일 확장자 목록은 `rbms_library::is_chart`(`crates/rbms-library/src/lib.rs:61`)의 `bms|bme|bml|pms|bmson`.
3. `rbms_chart::to_model(&src, mode)`(`lib.rs:132`).
4. bmson 은 `rbms_parser::bmson::parse(bytes)`(`bmson.rs:329`) → `chart.mode()`, `chart.to_model()`(`:317-319`). `info.ln_type`·노트 `t` 가 롱노트 종류를 말한다(`bmson/convert.rs:245`).

추출 대상은 모두 `Model` 에 이미 있다.
- 노트: `tl.notes[lane]` 의 `kind`·`time_us`(위 적분 결과)·`section`(마디 단위 위치, 박자 분할 판정에 쓸 수 있음). 롱노트는 `LongStart`/`LongEnd` 짝(`long_note::repair`, `lib.rs:235`로 원본처럼 정리됨: 몸통 안 일반 노트는 배경음, 닫히지 않은 머리는 제거). 지뢰는 `Mine{damage}`. 히든 노트(`tl.hidden`)는 치는 노트가 아니라 제외.
- BPM·STOP·SCROLL: 타임라인 `bpm`, `stop_us`, `scroll`. 헤더: `meta.total`, `rank`, `init_bpm`, `play_level`.
- 판정에 쓰는 노트 수: `rbms_model::count_playable_notes`(`rbms-model/src/lib.rs:189`).

### 3.3 소리·BGA 를 읽지 않고 파싱만 하는 방법

- 위 경로는 차트 파일 바이트만 읽는다. `#WAVxx`·`#BMPxx` 는 이름 문자열로 `Model.wavmap`/`bgamap` 에만 들어간다(`rbms-chart/src/lib.rs:239-240`). 오디오 디코드(`keysound_jobs`/`spawn_keysound_decode`)와 BGA 디코드는 `AppShared::load`에만 있다(`app_play.rs:512-562`). 따라서 CLI 가 `rbms-parser`·`rbms-chart` 만 쓰면 미디어를 열 일이 없다. `rbms-cli scan` 도 같은 방식이다(`main.rs:131-148`).
- 기존 읽기용 함수 `rbms_library::compute_chart_detail(path, mode)`(`lib.rs:72-110`)가 같은 파싱으로 노트 수·롱노트 수·길이·BPM 범위·밀도를 계산한다. 새 명령은 이것과 같은 모양에서 노트 배열까지 내놓는 확장이다.

### 3.4 md5·sha256

- 둘 다 `parse_with` 안에서 파일 원본 바이트의 `Md5::digest`·`Sha256::digest` 소문자 16진(`rbms-parser/src/lib.rs:136-137`, `hex` `:357-364`). bmson 도 BOM 포함 원본 바이트의 해시다(`bmson.rs:327-350`). 라이브러리 스캔은 이 `source.md5`·`source.sha256` 을 그대로 저장한다(`scan.rs:319-320`, bmson `:369-370`). 플레이어의 점수 제출도 `model.md5/sha256`(`rbms-ir/src/mapping.rs:146-149`). 즉 같은 해시다.
- 측정: `web/data/tables/insane1/data.json` 의 md5 1,035개는 전부 소문자 32자 16진이고 중복이 없다. 같은 md5 가 라이브러리 여러 경로에 있을 수 있으므로 md5 로 중복 제거해야 한다(`rbms-table/src/lib.rs:32-48`의 `Md5Index.by_md5: Vec<usize>`).

### 3.5 발광 표 `data.json` 형식과 1:1 매칭

- 형식(2026-10-19 측정): JSON 배열 1,035개, 340,700 B. 키는 `artist, comment, level, lr2_bmsid, md5, name_diff, title, url, url_diff`. `level` 은 문자열 "1"~"25"와 "???"(5곡). 레벨별 곡 수는 1: 53, 2: 56, ... 24: 11, 25: 2. **노트 수·sha256·모드가 없다.** `header.json` 은 `data_url, last_update, level_order, name, symbol`.
- 매칭 도구: `rbms_table::DifficultyTable::parse_body`(`rbms-table/src/lib.rs:121`)와 `Md5Index`(`:32-48`, 소문자화), `match_levels`(`:167`). 서버는 표 항목을 md5 로 찾고(`web/src/server/service/domain/table-snapshot/table-snapshot.data.ts` `findSnapshotTextByMd5`, `compose/table-snapshot.ts`의 `chart.md5` 조인), `chart` 표의 기본키는 sha256 이고 md5 는 널 허용 유니크(`schema.ts` chart, `chart_md5_uniq`). 그래서 추출 결과에는 md5 와 sha256 을 둘 다 담아야 서버가 표 항목(md5)을 `chart` 행(sha256)과 이을 수 있다.
- 서버 `chart.notes` 는 첫 제출자가 정한 값이라 신뢰할 수 없다고 이미 기록돼 있다(`docs/quality-assurance/2026-10-14-test-debt.md` 3.36). 패턴 업로드가 노트 수의 첫 신뢰할 수 있는 출처가 될 수 있다.

### 3.6 LN 방식·`#RANDOM`·bmson 처리 주의

- LN 방식: 차트가 `#LNMODE` 를 말하지 않으면 모델의 롱노트 종류는 `Undefined` 이고 플레이어의 LN MODE 가 정한다(`chart lib.rs:120-127, 398, 413`). CN/HCN 이면 끝도 판정 객체라 노트 수가 달라진다(`rbms-model/src/lib.rs:166-180`). `compute_chart_detail`·곡 DB `notes` 는 확정 전(LN 머리만 셈) 값이다. 추출물은 "차트가 선언한 종류"와 "미확정 롱노트 존재 여부"를 필드로 가져야 한다.
- `#RANDOM`/`#SWITCH`: 파서가 시드 0 으로 한 가지 분기를 고정 해석한다(`rbms-parser/src/lib.rs:163`, `control.rs` `Control::new(seed)`). 플레이어와 스캔이 같은 분기를 쓰므로 추출 결과는 R-BMS 가 실제로 치는 패턴이지만 원본(beatoraja)이 뽑는 임의 분기와 다를 수 있다. 제어문 사용 여부를 판단하는 `uses_control_flow`는 `scan.rs:477-482`에 비공개로 있다(공개로 바꾸거나 같은 판단을 따로 해야 함).
- bmson: 모드는 `mode_hint`(`bmson.rs:70-77`, `keyboard-24k` 포함, 알 수 없으면 BEAT_7K), TOTAL 은 백분율이라 노트 수가 정해진 뒤 확정된다(`lay_out_notes :143-145`). 추출 시 `to_model()` 의 `meta.total` 을 그대로 쓸지 정해야 한다.
- 파서 세대: `PARSER_VERSION = 5`(`HANDOFF.md:109`). 파서가 바뀌면 같은 파일의 노트 배열이 달라질 수 있으므로 추출물에 파서 세대를 담는다.
- 사용자 라이브러리 경로(`docs/memory/test-library.md`): `/Volumes/SSD/bms`(11,336 차트), `/Volumes/SSD/2026-07-31-backup/Documents/personally/1`(727 차트). 외장 SSD 라 자주 빠진다. 저작권 때문에 저장소에 커밋 금지라고 적혀 있으니 추출물은 저장소가 아니라 scratch·DB 에만 둔다.

---

## 4. 크기 추정

전제: `data.json` 에 노트 수가 없고 서버 `chart.notes` 는 읽지 않았으며 라이브러리도 읽지 않았다. 아래 평균 노트 수는 측정이 아니라 가정이다.

### 4.1 한 노트의 자료형 근거와 인코딩 안

- 코드 자료형: 시각 `i64` µs(`Micros`), 레인 `usize`(최대 26: `KEYBOARD_24K`, `rbms-model/src/mode.rs`), 종류 4가지, 판정 코드 0~5(+미판정), 오차 `i64` µs. 실제 오차는 판정 창 안팎의 수백 ms 이내(空POOR 대역·BAD 늦은 한계, `matcher.rs:1360-1396`)라 ms 단위 i16 으로 충분하다. 차트 시각 상한은 86,400초(`MAX_TABLED_SECONDS`, `rbms-chart/src/lib.rs:23`)라 ms 로 u32 에 든다. 5K~14K 는 레인 8~16개.
- 안 A 촘촘한 이진: 시각 Δ(ms) varint 1~2 B + 레인·종류 1 B + 판정·방향·오차(zigzag varint) 1.5~2.5 B = 약 4~5.5 B/노트(가정: 평균 Δ 가 128 ms 안팎 이하이면 1~2 B).
- 안 B 고정 폭 이진: 시각 u32 ms 4 + 레인 1 + 종류 1 + 판정 1 + 오차 i16 2 = 9 B/노트.
- 안 C JSON 병렬 정수 배열: 노트당 약 20 B(가정: 시각 8자리+콤마, 레인, 종류, 판정, 오차 5자리). 현재 와이어는 JSON 이다(리플레이 이벤트가 객체 형식으로 평균 40.4 B, 측정).

롱노트 끝·지뢰는 노트 레코드가 하나 더 붙는다(레코드 수 ≈ 판정 노트 + LN 끝 + 지뢰).

### 4.2 한 판의 노트별 판정 데이터

| 노트 수 | 안 A | 안 B | 안 C(JSON) |
| --- | --- | --- | --- |
| 1,000 | 4~5.5 KB | 9 KB | 약 20 KB |
| 3,000 | 12~16.5 KB | 27 KB | 약 60 KB |
| 5,000 | 20~27.5 KB | 45 KB | 약 100 KB |
| 10,000 | 40~55 KB | 90 KB | 약 200 KB |
| 50,000(문서가 말하는 가장 큰 차트, `test-debt.md` 3.39) | 200~275 KB | 450 KB | 약 1 MB |

한도와 비교
- 점수 본문 256 KiB = 262,144 B(`route/score.ts:16`): 안 C 는 약 13,000 노트부터 초과, 안 A·B 는 5만 노트에서도 가능하지만 점수 본문에 얹지 않는 것을 권한다(2.2).
- 리플레이 업로드 한도 4 MiB = 4,194,304 B(`env.ts:4`). 지금 리플레이는 이벤트 40.4 B 이고 노트당 입력이 2개 안팎이라 약 81 B/노트다. 계약 테스트가 10만 이벤트가 4 MiB 안에 들어야 한다고 고정한다(`client-requests.test.ts` `REPLAY_EVENTS_THAT_MUST_FIT`). 즉 5만 노트 차트의 리플레이는 이미 한도 근처다.
- Vercel 함수 요청·응답 본문은 4.5 MB 가 상한이고 넘으면 413 `FUNCTION_PAYLOAD_TOO_LARGE`(https://vercel.com/docs/functions/limitations#request-body-size, 2026-08-24 갱신본). 환경변수로 4.5 MB 이상 올려도 소용없다.
- 클라이언트 요청 제한 5초(`http.rs:14`): 수백 KB 본문은 느린 회선에서 폴더 요청처럼 더 긴 제한이 필요할 수 있다(`FOLDER_REQUEST_TIMEOUT` 선례, 값은 정하지 않았다).

게이지·옵션·입력
- 선택 게이지 0.5초 열: 5분 곡 600칸. f32 이면 2.4 KB, 8비트 양자화면 0.6 KB. 9개 전부 f32 면 약 21.6 KB. `failed_at_ms` 이후는 0 패딩이라 열에서 뺄 수 있다.
- 옵션 시작값·변경 이벤트: 1 KB 미만.
- 입력 이벤트는 노트별 판정으로 거의 설명된다(오차, 空POOR 판정이 `judged()` 에 있음). 원입력까지 올리려면 40.4 B/이벤트(JSON) 이므로 리플레이와 중복이다. 권고: 올리지 않는다.

### 4.3 곡별(1,035곡) 노트 배치 저장

레코드 구성: 시각 Δ + 레인 + 종류. 안 A 약 2.5~3.5 B, 안 B 6 B(u32 ms + 레인 + 종류), JSON 약 14 B(가정). 곡마다 머리말(md5 32 + sha256 64 + 모드·LN 플래그·BPM 목록·파서 세대 등) 약 150~300 B.

| 평균 노트 수(가정) | 안 A 합계 | 안 B 합계 | JSON 합계 |
| --- | --- | --- | --- |
| 1,500 | 약 4.7 MB | 약 9.3 MB | 약 22 MB |
| 3,000 | 약 9.3 MB | 약 18.6 MB | 약 43 MB |

한 곡 최대(5만 노트): 안 A 약 150 KB, 안 B 300 KB, JSON 약 700 KB. 모두 4.5 MB 안이다.
저장 컬럼: 리플레이가 쓰는 `mediumtext`(최대 16,777,215 B)가 선례(`schema.ts` replay.data). 운영 DB 의 `max_allowed_packet` 은 확인하지 못했다.

---

## 5. 결정론적으로 계산할 수 있는 패턴 특징

### 5.1 후보 특징과 입력

노트 배치(시각·레인·종류·LN 끝·지뢰)와 타임라인(BPM·STOP·SCROLL)만으로 정해지는 것이라 모두 일반 코드다.

| 특징 | 정의 입력 | 비고 |
| --- | --- | --- |
| 밀도: 초당 노트, 최고·평균·후반(게이지 border 이후) | 노트 시각 | 기존 구현 `rbms_chart::note_density`(`rbms-chart/src/lib.rs:446-`)가 1초 칸 `bins`·`peak`·`avg`·`end` 를 이미 낸다. 곡 DB 에 저장 중 |
| 슬라이딩 창(0.5/1/4초) 최고 밀도, 버스트와 쉼 | 노트 시각 | 박자 상대 정의: 16분 = `measure_us(bpm)/16 = 15,000,000/bpm` µs(`rbms-model/src/lib.rs:10-15`) |
| 동시치기(chord) 크기 분포, 손별 분배 | 같은 시각의 노트 수·레인 | 타임라인당 레인 한 칸이라 구조적으로 쉽다 |
| 연타(잭): 같은 레인이 간격 X 안에 반복 | 레인별 연속 노트 간격 | X 는 국소 BPM 의 16분 대비 비율로 둔다 |
| 계단(stair)·트릴·지그재그: 인접 레인으로 단조 이동·교대 | 연속 노트의 레인 차이 부호·크기 | 레인 인접성에 의존 |
| 스크래치 연동: 스크래치 단독, 스크래치+건반 동시, 스크래치 롱(백스핀), 연속 | 스크래치 레인(`Mode.scratch`, `mode.rs`) 노트와 이웃 | DP 는 양쪽 스크래치 |
| 롱노트 섞임: LN 길이 분포, LN 중 다른 손 노트 수·동시 보유 개수, LN 끝 직후 노트 | `LongStart`/`LongEnd` 짝 | LN 방식(LN/CN/HCN)이 판정 객체 수를 바꾸므로 입력에 포함 |
| 지뢰: 개수, 레인, 근접한 노트 | `Mine{damage}` | 레인 의존 |
| 소프란·BPM 변화: 변화 횟수·최대/최소 비, STOP 횟수·길이, SCROLL 변화 | `tl.bpm/stop_us/scroll`(`rbms-model/src/lib.rs:101-114`) | 구간별 녹색 숫자 = 하이스피드와 국소 BPM·SCROLL 의 곱 |
| 최소 노트 간격, 박자 분할(16분/24분 등) | `section` 과 시각 | `Note.section` 이 마디 단위 위치라 분할 판정에 쓸 수 있다(`rbms-model/src/lib.rs:84-93`) |

판정과 합치면 만들어지는 개인 지표(서버 쪽 계산): 패턴 종류별 PG/GR/GD/BD/POOR 비율, 레인별 치우침, 구간별 BP 와 게이지 꺾임 위치, 빠름/늦음 치우침(오프셋 조언), 같은 패턴 종류 안에서의 시간 경과.

### 5.2 RANDOM·MIRROR·S-RANDOM 이 특징에 주는 영향

근거 `crates/rbms-chart/src/shuffle.rs`. 모든 옵션이 시각은 건드리지 않고 레인만 바꾼다(노트 수 보존). 스크래치 레인은 H-RANDOM·RANDOM 계열에서 고정이고 ALL-SCRATCH 만 건드린다.

| 옵션 | 변환 | 유지되는 특징 | 바뀌는 특징 |
| --- | --- | --- | --- |
| MIRROR | 쪽 안에서 건반 레인을 뒤집음(`:125-127`) | 밀도, 동시치기 크기, 잭(같은 레인), 인접 거리 | 좌우: 손 분배 좌우 교환, 계단·트릴의 진행 방향 반전 |
| RANDOM | 쪽마다 피셔-예이츠 순열(`:132-135`) | 밀도, 동시치기 크기, 잭(순열은 전단사라 "같은 레인" 관계 보존) | 인접성(계단·트릴·지그재그 파괴), 손 분배, 레인별 분포 |
| R-RANDOM | 무작위 크기 r 로 순환 이동(`:128-131`) | 밀도, 잭, 인접성(순환 경계 제외), 진행 방향 | 레인 위치, 순환 경계의 인접 관계 |
| ROTATE | 한 칸 순환 이동 | R-RANDOM(r=1)과 같다 | |
| S-RANDOM | 타임라인 행마다 새 순열, 진행 중 롱노트는 고정(`:185-232`) | 밀도, 동시치기 크기, 롱노트 머리·끝 정렬 | 잭·계단·트릴 구조 전부 파괴(우연한 같은 레인 반복만 남음), 손 분배 |
| H-RANDOM | 행별 재배정 + 125 ms 안 같은 레인 회피(`HRAN_THRESHOLD_US`, `:70`) | 밀도, 동시치기 크기 | 잭을 억제(원 차트보다 적어짐), 계단·트릴 파괴 |
| ALL-SCRATCH | 스크래치를 풀에 넣어 먼저 채움, 스크래치는 40 ms 창(`SCRATCH_THRESHOLD_US`, `:75`) | 밀도 | 스크래치 노트가 크게 늘고 건반 구조가 바뀜 |
| SCRATCH AUTO (옵션이 아니라 판정 설정) | 배치 뒤 스크래치 레인 노트를 판정에서 뺌(`take_auto_lanes`) | 건반 특징 | 스크래치 특징은 "자동으로 침"으로 표시해야 함. 제출의 `options.scratch_auto` 가 있다 |

- DP 는 쪽마다 따로 섞고 쪽을 넘나들지 않는다(`side_key_lanes :100-107`).
- 결론: 박자 기반 특징(밀도, 동시치기 크기, 쉼, BPM 계열)은 옵션과 무관하게 (b) 원본 패턴에서 계산해도 같다. 레인 기반 특징(잭 제외한 인접성 계열, 손 분배, 레인 치우침)은 옵션과 시드마다 다르므로 약점 분석은 (a) 실제 배치에서 계산해야 한다. 곡 추천(5번)처럼 안 쳐 본 곡의 레인 기반 난이도를 말할 때는 원본 배치(옵션 OFF)에서 계산한 값에 "옵션 무시" 표시를 둔다.
- 시드 재현은 서버가 하지 않는다. 같은 판의 시드·옵션은 이미 제출에 들어 있지만(`seed`, `options.random`), 위 표의 변환을 TS 로 이식하는 대신 (a)에서 실제 레인을 받는 방식이 단순하다.

### 5.3 계산 위치 권고

- 서버(TypeScript)에서 원자료로부터 계산하고 분류 정의에 `pattern_algo_version` 을 붙여 다시 계산 가능하게 둔다. 이유: 정의는 계속 바뀔 것이고 클라이언트 릴리스는 느리다(릴리스 draft 게시까지 사용자 손을 거침, `HANDOFF.md:14`). 서버는 원자료가 있으면 지난 판을 소급 재계산한다.
- 클라이언트는 계산하지 않고 정확한 원자료(실제 배치, 노트별 판정, 게이지, 옵션 이력)만 올린다. 그 원자료는 엔진이 낸 값이라 결정론적이고 서버가 따로 구현해야 하는 판정·게이지 규칙이 없다(게이지 표는 `rbms-judge/data/gauge.ron` 에 있어 서버에는 없다).

---

## 구현 시 밟을 접점 요약 (코드를 고치지 않았다)

| 일 | 자리 |
| --- | --- |
| 판 요약 데이터 만들기 | `apps/rbms-player/src/app_result.rs:216 record_run` 한 곳 (결과·Esc·재시작·종료가 전부 이 함수를 지난다) |
| 하이스피드·커버·리프트·오프셋 시작값/변경 이력 | `stage/play/mod.rs`, `stage/play/controls.rs` 에 이벤트 로그 필요(지금 없음), `PlaySession::judge_offset_ms`(`session.rs:1057`)로 끝값 |
| 로컬 보관 | `crates/rbms-store/src/scoredb/` 새 표 + `ir_sync` 열(`ADDED_COLUMNS`) |
| 전송·재전송 | `rbms-ir` 의 `ScoreServer` 선택 메서드(기본 `Unsupported`, `crates/rbms-ir/src/lib.rs:60-128` 패턴) + `record_sync.rs` 리플레이 단계 옆 |
| 서버 | 새 마이그레이션(표 2개: 판별 부속, 곡별 패턴), 새 라우트, 서비스 `compose` 분리(web 계층 규칙 `docs/architecture.md:116`) |
| 곡별 추출 | `apps/rbms-cli` 새 하위 명령 |
| 계약 교차 테스트 | 2.3 |

---

## 조사자가 낸 제약 목록 (원문)

1. 프로젝트 규칙 단일 출처 docs/HANDOFF.md 5절: 응답은 한국어 존댓말, 이모지·글자 그림 금지, 확인하지 못한 것은 확인하지 못했다고 말한다, 근본 원인을 고친다(우회 금지), 판정·점수 의미를 원본과 다르게 바꾸는 수정은 고치기 전에 사용자 확인.
2. Rust 규칙: // 주석 금지(//! 와 /// 는 영어만), 매직넘버 금지(상수화), 새 #[allow]·unsafe·todo!() 금지, rustfmt max_width 160, 새 외부 의존성 금지, --locked. 압축(flate2)·base64 는 Cargo.lock 에 전이 의존으로만 있고(Cargo.lock 334/340, 957행) 어느 워크스페이스 크레이트의 직접 의존도 아니다. 직접 쓰려면 새 의존성이므로 사용자 확인이 필요하다.
3. 웹 규칙(web/AGENTS.md 와 컨벤션): arrow function, any·enum·코드 주석 금지, FSD, TanStack Query queryOptions+QUERY_KEY, 경계에서 Zod 검증, useCallback·useMemo 금지. web/AGENTS.md 는 Next.js 16 이 학습 데이터와 다르다고 경고하므로 웹 코드를 쓰기 전에 node_modules/next/dist/docs/ 를 읽어야 한다.
4. 기록 원칙: 사람이 친 판은 실패·포기·종료 포함 전부 저장·전송하고, 제외는 사용자가 지시한 '노트를 하나도 치지 않은 판'뿐이다. 메인이 제외 조건을 추천하지 않는다. 노트별 부속 데이터도 이 규칙을 따라 점수와 같은 판 단위로 생성해야 한다(HANDOFF.md:75, play-options.md 10.2).
5. 점수 본문은 SUBMISSION_MAX_BYTES 256 KiB(web/src/server/route/score.ts:16), 리플레이 본문은 REPLAY_MAX_BYTES 기본 4 MiB(web/src/server/lib/env.ts:4), Vercel Functions 요청·응답 본문은 4.5 MB 절대 상한(https://vercel.com/docs/functions/limitations#request-body-size). 클라이언트 요청 제한은 기본 5초(crates/rbms-ir/src/http.rs:14).
6. 속도 제한: 점수 분당 60, 리플레이 분당 10(web/src/server/lib/with-rate-limit.ts:6-7). 기록 동기화 간격은 점수 1.5초·리플레이 7.5초(record_sync.rs SCORE_SEND_INTERVAL, REPLAY_SEND_INTERVAL). 새 부속 업로드의 한도·간격은 정해져 있지 않다.
7. 계약 교차 테스트는 양방향으로 고정돼 있다: Rust contract_requests.rs 의 CONTRACT_FILES 가 폴더와 정확히 일치해야 하고(테스트 the_contract_folder_holds_every_request_and_nothing_else), 웹 client-requests.test.ts 는 서버 스키마가 값을 조용히 바꾸거나 버리면 실패한다. 새 요청 종류는 두 쪽 목록·스키마·널 허용 목록을 함께 고쳐야 하고 RBMS_UPDATE_CONTRACT=1 로 파일을 만든다.
8. 배포 순서: 새 웹 코드가 새 마이그레이션에 기대면 마이그레이션 파일을 먼저 커밋·적용하고 코드를 나중에 푸시(HANDOFF.md:81). 운영 DB 는 web-db.yml 로만, 운영 사이트에는 읽기 요청만. 마이그레이션은 drizzle-kit generate/migrate 이며 push 금지.
9. 해시 일치: 발광 표 매칭 키는 소문자 md5(rbms-table Md5Index 가 소문자화). 서버 chart 표의 기본키는 sha256 이고 md5 는 널 허용 유니크라 추출물에 둘 다 필요하다. 라이브러리 스캔·플레이어·CLI 가 같은 rbms_parser 해시 함수를 쓴다.
10. 서버의 chart.notes 는 첫 제출자가 정한 신뢰할 수 없는 값이다(test-debt.md 3.36). 노트 수 상한이나 검증 기준으로 쓰면 안 된다.
11. 제출 본문의 options.green_number 는 레인 통과 시간(currentduration, ms)이고 스킨 GREEN 표시값(0.6 곱함)이 아니다. CONSTANT 는 rbms 고유 보정이다. AI 조언에서 '녹색 숫자'를 말하려면 환산 방식을 정해야 한다.
12. JUDGE OFFSET 은 노트를 그리는 시각만 옮기고 판정은 입력 시각 그대로다(play-options.md 3.1). 오프셋 조언은 판정 보정이 아니라 표시 위치 이동으로 설명해야 하고, AUTO ADJUST 를 켠 판은 오프셋이 판 중 움직이므로 시작값·끝값이 모두 필요하다.
13. 옛 FRAME 시계 리플레이(judge.clock != EVENT)는 노트별 재판정의 근거로 쓸 수 없다(record-target-pace.md 4.2). 서버가 받는 와이어 리플레이는 판정 알고리즘·LN 종류·시계를 싣지 않아 서버 쪽 재판정도 보장되지 않는다.
14. #RANDOM/#SWITCH 차트는 R-BMS 가 시드 0 의 한 분기로 고정 해석한다. 곡별 추출물은 그 분기이며 원본 플레이어가 뽑는 분기와 다를 수 있다.
15. 차트 파일은 저작물이라 저장소에 커밋하지 않는다(test-library.md). 추출한 노트 배치는 서버에 비공개로 저장하고 공개 API 로 내보내지 않는다(사용자 결정). 에이전트는 rbms-player 를 실행하지 않으며 .env·settings.ron·scoredb.sqlite 를 읽지 않는다.
16. AI 키(BYOK)는 서버에만 암호화 저장되며 클라이언트 코드·계약 파일·로그에 키가 들어가서는 안 된다. 이번 조사는 클라이언트 쪽 데이터만 다루었고 공급자 어댑터·암호화는 조사 범위 밖이다.

## 확인하지 못한 것 (원문)

1. 발광 표 1,035곡의 실제 노트 수 분포를 알지 못한다. data.json 에 노트 수가 없고, 서버 chart.notes 는 신뢰할 수 없어 읽지 않았으며(운영 DB 접근 금지), 사용자 라이브러리는 읽지 않았다. 4절의 평균 노트 수(1,500/3,000)는 가정이다.
2. 실제 판의 노트별 데이터가 얼마나 압축되는지 측정하지 못했다. 계약 리플레이(replay-long.json)는 규칙적으로 만든 합성 입력이라 압축률이 의미가 없어 인용하지 않았다. JSON 노트당 20 B 와 14 B 는 자릿수에서 낸 가정이다.
3. 사용자의 발광 전곡 중 라이브러리에 실제 있는 곡 수, 같은 md5 가 여러 경로에 있는 곡 수, 표 안에 #RANDOM/#SWITCH 를 쓰는 곡 수, bmson 곡 수를 알지 못한다(추출을 하지 않았다).
4. 운영 MySQL 의 호스팅·용량·max_allowed_packet 과 Vercel 요금제(Hobby/Pro)를 확인하지 못했다. 함수 실행 시간(Hobby 300초 기본·최대, Pro 최대 800초)과 메모리가 AI 호출·서버 쪽 집계에 영향을 준다. web/vercel.json 은 리전 icn1 만 말한다.
5. 점수 순위·상세 질의가 score 행의 extra JSON 컬럼을 읽는지(즉 크기가 늘면 읽기 비용이 느는지)는 compose 쿼리를 끝까지 읽지 않아 확인하지 못했다. 이 조사는 노트별 데이터를 score.extra 에 두지 않는 쪽을 권하므로 영향은 없다.
6. 노트별 부속 업로드를 기본 켬으로 할지 설정 옵트인으로 할지는 결정되지 않았다(설정 스키마 6, 새 행은 rbms-config 의 SETTINGS 표에 한 줄이다). 개인 정보와 서버 용량 측면의 결정이 필요하다.
7. 부속 업로드의 속도 제한·본문 한도·사용자당 보존량(리플레이는 2,000개·512 MiB)을 정하지 않았다. 부속을 점수와 같은 속도로 보낼지 리플레이 업로드처럼 더 느린 몫으로 보낼지 정해야 한다.
8. 곡별 패턴을 올리는 주체와 경로(CLI 가 파일을 쓰고 웹 스크립트가 올릴지, CLI 가 rbms-ir 를 의존해 직접 올릴지, API 토큰의 관리자 권한을 쓸지)를 정하지 않았다. 기존 관리자 라우트는 세션 쿠키 방식이다.
9. 과거 판의 소급 생성은 가능하지만(저장된 리플레이 재판정) 범위·방법을 정하지 않았다. rejudge 용 lay_out_notes·replay_judge_setup 이 rbms-player 앱 안에 있어 CLI 로 쓰려면 라이브러리 크레이트로 옮겨야 하고, 옛 FRAME 시계 리플레이는 쓸 수 없다. 사용자가 실제 몇 판을 로컬에 가졌는지 모른다.
10. 게이지 이력을 선택 게이지 하나만 올릴지 9개 전부 올릴지(약 21.6 KB 차이) 정하지 않았다. 게이지가 판정 규칙(gauge.ron)에서 나오므로 서버가 다른 게이지를 가정 계산할 수 없다. 게이지·옵션 조언(8번)이 다른 게이지를 가정하려면 9개 전부가 필요하다.
11. 원입력(리플레이 이벤트)을 노트별 부속에 중복해서 올릴 필요가 있는지는 기능 정의에 달렸다. 空POOR(빈 입력)가 정확한 시각까지 필요한 기능이 있으면 판정 목록(judged)의 Miss 항목으로 충분한지 확인이 필요하다.
12. LN 방식(LN/CN/HCN)이 곡별 패턴의 노트 수와 어떤 대응을 갖게 할지 정하지 않았다. 차트가 방식을 선언하지 않은 곡은 플레이어마다 방식이 다르고 사용자의 판 제출에는 lntype 이 있다.
13. 클라이언트 내부 이름 충돌을 피하는 새 구조체 이름, 노트 식별 방식(시각 기반 조인 vs 타임라인 번호 기반 조인)을 정하지 않았다. 같은 시각 동시 노트는 시각으로는 구별되지 않으나 실제 배치 분석에는 영향이 없다고 판단했다. 시각이 같은 서로 다른 타임라인이 있는 극단적 차트는 확인하지 못했다.
14. 5만 노트 차트에서 노트별 데이터를 만드는 데 드는 시간과 프레임 영향을 재지 않았다. 계산은 판이 끝난 뒤 한 번 note_marks 를 훑는 일이라 작을 것으로 보이지만 측정하지 않았다(test-debt.md 3.75 와 같은 종류의 부채가 된다).
15. 공급자 쪽(Anthropic·OpenAI·Ollama Cloud) API 사양, 암호화 저장 방식, 구조화 출력 스키마 공통화는 이번 조사 범위가 아니어서 확인하지 않았다.
