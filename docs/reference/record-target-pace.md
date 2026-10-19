# 기록 목표의 진행 중 값: 기록 판의 노트별 판정을 따라간다

> 최종 갱신 2026-10-19 · 대응 커밋 e2c829f 이후 작업 트리
> 범위: 플레이 중 LOCAL BEST·IR BEST 목표와 BEST 칸(최고 기록)의 "지금까지" 값. 결과 화면과 고정 비율 목표는 바뀌지 않는다.
> 결정 기록: [ADR 2026-10-19](../acknowledge/2026-10-19-adr-record-target-follows-its-run.md). 사용자 문서: [play-options.md 11.7](../play-options.md)

이 문서는 원본(beatoraja) 재조사 없이 이 기능을 고치거나 넓힐 수 있게 조사 결과와 실제 구현을 함께 적는다. 코드가 바뀌면 해당 절을 고친다.

## 1. 사용자 요청(2026-10-19 원문)

> 지금은 목표점수가 퍼센트를 기준으로 target을하게되어있는데 이게 AA, A 등 특정 퍼센트에서는 맞는말이지만 my best, localbest등에서는 퍼센트가 아니라 실제 best기록의 동시간의 기준으로 노트하나하나당의, 그러니까 리플레이 플레이 그대로를 기준으로 되어야해. 퍼센트로 하는게 아니라는 뜻이지

결정(사용자): 그 판의 노트별 판정을 얻을 수 없으면 지금처럼 선형 비례로 물러난다. RATE·MAX·RANK NEXT·RATE CUSTOM 목표는 선형 비례 그대로.

## 2. 원본의 의미(읽기만 함, `/Users/hyunseokbyun/development/beatoraja`)

| 무엇 | 원본 위치 | 내용 |
| --- | --- | --- |
| 판마다 남기는 것 | `JudgeManager.java:137-140` | `ghostJudge = new int[totalNotes]`, 전부 4(POOR)로 채운다 |
| 기록 순서 | `JudgeManager.java:639-646` | `updateMicro` 에서 `judgeVanish` 이면 `ghostJudge[passnotes] = judge` 후 `passnotes + 1` |
| 세지 않는 POOR | `JudgeManager.java:647-649` | `MissCondition.ONE` 의 이른 return 보다 앞에서 적으므로, 세지 않는 소비 POOR 도 자리를 차지한다 |
| 저장 | `BMSPlayer.java:888`, `ScoreData.java:507-523` | `encodeGhost` 로 gzip+base64, 점수 DB 의 ghost 열 |
| 최고 행의 ghost | `ScoreData.java:550-564` | EX 가 엄격히 클 때만 바뀐다. 같은 EX 면 처음 낸 판의 ghost 가 남는다 |
| 진행 중 값 설정 | `BMSPlayer.java:440-454` | 연습은 `setTargetScore(0, null, 0, null, notes)`. 그 밖(손, 오토, 리플레이, 코스)은 BEST 칸에 로컬 최고의 ghost, 목표(rival) 칸에는 언제나 `null` |
| 길이 검사 | `ScoreDataProperty.java:155-174` | `useBestGhost = bestGhost != null && bestGhost.length == totalnotes`(랜덤 분기로 노트 수가 다를 수 있다는 주석 `:171`) |
| 갱신 | `ScoreDataProperty.java:64-134` | ghost 가 있으면 `previousNotes..notes` 의 ghost 를 PG 2, GR 1, 그 밖 0(`:137-144`)으로 누적한 값이 `nowbestscore`(`:114-118`), 없으면 `bestscore * notes / totalnotes` 내림(`:119-121`). rival 도 같은 꼴(`:123-131`)이지만 원본은 rivalGhost 가 늘 null 이라 비례다 |
| `notes` | `BMSPlayer.java:1031`, `:1019-1037` | `judgem.getPastNotes()` = passnotes. 판정이 하나 셀 때마다 불린다(`JudgeManager.java:693`) |
| 쓰는 곳 | `IntegerPropertyFactory.java:702-710`, `FloatPropertyFactory.java:299-302`, `MainStatePropertyLuaApiExporter.java:56-59` | 152/172 = nowEX - nowBest, 108/128·153 = nowEX - nowRival, rate 112·114, Lua `rate_best`. 결과 화면은 최종 값만(`MusicResult.java:343`, `CourseResult.java:268`) |
| 목표 목록 | `TargetProperty.java:61-76` | 원본에는 LOCAL BEST 가 목표로 없다 |

요점: 기준 인덱스는 시각이 아니라 "지금까지 판정(소비)된 노트 수"(passnotes)다. 비교하는 값은 "기록 판이 처음 k 개 노트를 판정했을 때의 EX" 이며, 어느 레인의 어느 노트인지가 아니라 판정된 순서다.

## 3. R-BMS 가 따르는 의미

- 진행 중 값 = 고스트의 `ex_after[total_judged]`. `ex_after` 는 길이 `total_notes + 1` 의 누적합이다(`ex_after[0] = 0`, PG 2, GR 1, 그 밖 0). 끝까지 가지 못한 노트(게이지로 끝난 판, 도중에 떠난 판)는 원본처럼 POOR(EX 0)다. 인덱스가 끝을 넘으면 마지막 값.
- 인덱스는 R-BMS 의 `JudgeEngine::total_judged`(원본 passnotes 와 같은 정의, `crates/rbms-judge/src/matcher.rs` `total_judged`).
- BEST 칸(`best_ghost`): 로컬 최고 기록의 고스트. 원본과 같다(지금까지 R-BMS 는 비례였다).
- 목표 칸(`target_ghost`): 실제로 쓰는 목표(`ResolvedTarget::used`)가 LOCAL BEST 면 같은 고스트, IR BEST 면 이 기기에서 친 같은 판의 고스트. 원본은 목표 칸이 늘 비례이므로 이것은 R-BMS 확장이다(사용자 지시).
- 고스트가 없거나 맞지 않으면 그 칸은 지금처럼 비례(`pace_ex_at`, `ScoreStanding::of` 의 `pace`).
- 한 판의 목표와 고스트는 시작 전에 한 번 붙고 끝까지 그대로다(ADR 2026-10-14 의 "한 판의 목표는 시작할 때 한 번").

## 4. 데이터 출처: 리플레이를 다시 판정한다

- 점수 DB(score·scorelog)에는 노트별 판정 열이 없다(`crates/rbms-store/src/scoredb/schema.sql`). DB 마이그레이션은 하지 않는다.
- 리플레이 파일(`replays/{md5 앞 8자}-{played_at_ms}.ron`)에는 입력만 있다(`crates/rbms-store/src/replay.rs` `Replay`). 그래서 리플레이를 화면·소리 없이 다시 판정해 소비 판정 목록을 얻는다.
- 재판정은 프레임 간격과 무관하다(`PlaySession::tick` 이 입력마다 시계를 그 시각까지 정산). 그래서 마지막 노트 + 꼬리 시각으로 한 번 tick 하면 된다(`PlaySession::play_out`).
- 같은 판정을 얻으려면 load 와 똑같이 모델을 배치해야 한다: 리플레이의 LN 종류 해석, random + seed 배치, bmson/기본 TOTAL, TOTAL override, SCRATCH AUTO 레인, 게이지, JUDGE 행 전부(`replay_judge_setup`). 배치는 `lay_out_notes`, SCRATCH AUTO 레인은 `scratch_auto_lanes` 로 load 와 공유한다.

### 4.1 어느 판을 쓰나

| 칸 | 찾는 법 | 함수 |
| --- | --- | --- |
| BEST | 같은 md5·같은 LN 종류 키·어시스트 아님, EX == 최고 EX, `replay_file` 있음, 가장 오래된 것(그 EX 를 처음 낸 판, 그 판에 리플레이가 없으면 다음 같은 EX 판) | `ScoreBook::best_record_with_replay_in_ln_mode` |
| IR BEST | TARGET 이 IR BEST 일 때만. 순위표 캐시의 내 줄(`RankingRow.played_at` ms, `ex_score`)과 같은 md5·LN 종류 키·같은 초(`played_at / 1000`)·같은 EX 의 로컬 기록, `replay_file` 있음 | `ghost::account_best_played_here` |

같은 파일이면 한 번만 판정한다(`GhostJob::judge_all`).

### 4.2 다른 판을 막는 검사(모두 통과해야 고스트)

1. 리플레이 파일을 읽을 수 있고 RON 이 맞다.
2. `Replay.md5` == 이번 차트 md5(대소문자 무시).
3. 리플레이의 시계가 이벤트 시계다(`Replay.judge.clock == REPLAY_EVENT_CLOCK`, `rejudge` 가 판정을 시작하기 전에 거른다). 옛 프레임 구동 빌드가 저장한 리플레이(`REPLAY_LEGACY_CLOCK` = `FRAME`)는 지나감·롱노트 놓음·지뢰·HCN 틱이 그 기기의 프레임에 놓인 채 기록되어, 입력을 다시 먹여도 같은 판정이 나온다는 보장이 없다. 재생은 되지만(테스트 부채 3.56) 노트별 판정의 근거로는 쓰지 않는다.
4. 다시 판정한 노트 수 == 기록의 `total_notes`.
5. 다시 판정한 PGREAT·GREAT 수 == 기록의 `counts[0]`·`counts[1]`(EX 일치를 함축하고, 도중에 떠난 판의 남은 노트 POOR 처리에 영향받지 않는다).
6. 플레이 화면에 붙일 때: 고스트 노트 수 == 이번 판 노트 수(원본 `ScoreDataProperty.java:172`), 고스트 최종 EX == 그 칸의 점수(로딩 뒤 기록·순위표가 바뀐 경우를 막는다).

허용 오차는 없다. 하나라도 어긋나면 그 칸은 비례다.

## 5. 구현(실제 코드)

| 곳 | 내용 |
| --- | --- |
| `crates/rbms-judge/src/matcher.rs` | `JudgeEngine.vanished: Vec<Judge>`(원본 ghostJudge). `update_micro` 의 `if vanish` 블록에서 push(MissCondition::One 이른 return 앞). 엔진을 만들 때 `count_objects` 만큼 미리 잡아 플레이 중 커지지 않는다. 접근자 `vanished()`. 판정 규칙은 바뀌지 않았다 |
| `crates/rbms-play/src/session.rs` | `PlaySession::play_out()`: `last_time_us() + PLAY_TAIL_US` 로 `NullSink` tick 한 번 |
| `crates/rbms-store/src/score.rs` | `ScoreBook::best_record_with_replay_in_ln_mode` |
| `apps/rbms-player/src/ghost.rs`(새 파일) | `GhostPace`(`of_judges`, `ex_after`, `total_notes`, `final_ex`), `GhostSource`, `GhostSources`, `ghost_sources`, `GhostJob`(`spawn`, `judge_all`), `rejudge`, `GhostLoad`, `RunGhosts` |
| `apps/rbms-player/src/app_play.rs` | `lay_out_notes`, `scratch_auto_lanes` 를 뽑아 load 와 고스트가 함께 쓴다. load 가 LN 해석 전 모델을 복제해(출처가 있을 때만) `GhostJob` 을 띄우고 `LoadedChart.ghost` 로 싣는다. 연습 요청이면 만들지 않는다. 파일이 없고 고스트만 기다리면 LOADING 화면으로 가되 결정음은 울리지 않는다(이전의 바로 시작과 같게) |
| `apps/rbms-player/src/stage/loading.rs` | `ChartLoads` 에 `ghost`/`ghosts`, `with_ghost`, `take_ghosts`. `poll` 이 받고, `is_done` 이 기다리고, `stop` 이 cancel 한다. 단계 글자 `record run` |
| `apps/rbms-player/src/stage/play/mod.rs` | `PlayState.ghosts/best_ghost/target_ghost`. `wait_for`·`poll_loads` 가 도착한 고스트를 받고, `ensure_pace_target` 과 `poll_loads` 끝의 `attach_ghosts` 가 붙인다. `paced_now`, `target_ex_now`(HUD 점수 차). 스킨 없는 판도 첫 tick 전에 목표를 정한다 |
| `apps/rbms-player/src/skin_host/score/standing.rs` | `PacedScores { best, rival }`, `TargetPace.now`. `ScoreStanding::of` 가 Some 이면 비례 대신 그 값을 쓴다 |
| `skin_host/score.rs`, `skin_host/play/shown.rs`, `stage/play/skin.rs` | `RunScore::of_session(.., now)`, `PlayLive.paced_now`, `Standing.paced_now` |
| `apps/rbms-player/src/ir_ranking.rs` | `RankingRow.played_at`(서버 기록의 played_at) |

### 5.1 스레드와 프레임 비용

- 재판정은 `rbms-ghost` 이름의 작업 스레드에서 한다. 먼저 저장 스레드를 기다린다(`SaveWorker::settled`): 방금 최고를 낸 판을 바로 다시 할 때 그 판의 리플레이가 아직 저장 중일 수 있다.
- 로딩 게이트(`ChartLoads`)가 고스트를 키음·BGA 와 함께 기다린다. 판정은 로딩이 끝난 뒤에만 시작하므로 판 도중 값이 바뀌지 않는다. 프레임 스레드는 `try_recv` 만 한다.
- 플레이 중 비용: 판정마다 배열 하나 읽기(`ex_after`). 할당 없음.

### 5.2 바뀌지 않는 곳

결과 화면의 목표·최종 값(`run_target`, `ResultSnapshot`), `TIMER_SCORE_BEST/TARGET`(최종 값 비교, 원본 `BMSPlayer.java:1036-1037` 과 같음), 내장 HUD 그래프의 BEST 선(최종), 스킨 미리보기(기록 없음), 선곡 화면.

## 6. 물러나는 조건(비례)

| 조건 | 칸 |
| --- | --- |
| 목표가 MAX, RATE A- ~ MAX-, RANK NEXT, RATE CUSTOM 이거나 그것으로 물러남 | 목표 |
| 목표가 RIVAL(원본도 비례, 서버 리플레이는 와이어에 판정 알고리즘·LN 종류·시계가 없어 재판정 불가) | 목표 |
| IR BEST 인데 이 기기에 같은 판(같은 초·EX·md5·LN 종류)이 없거나 리플레이가 없음, 다른 기기에서 친 IR BEST | 목표 |
| IR BEST 가 로컬 최고로 물러남(`used == LocalBest`) | 로컬 최고 고스트를 쓴다 |
| 로컬 최고 없음(처음 치는 차트, 넓힌 판정·옛 SCRATCH AUTO 판만) | 둘 다 |
| 같은 EX 의 최고 기록 어느 것에도 리플레이 없음(AUTO REPLAY 꺼짐, 저장 실패, GC) | 해당 칸 |
| 파일 읽기·RON 실패, md5 다름 | 해당 칸 |
| 노트 수 다름(SCRATCH AUTO 판과 아닌 판, LN 종류, 파서 세대로 노트 수가 바뀐 차트) | 해당 칸 |
| 옛 FRAME 시계 리플레이(`Replay.judge.clock != REPLAY_EVENT_CLOCK`). `rejudge` 가 판정하기 전에 거부한다 | 해당 칸 |
| PG/GR 다름(코스 스테이지 판의 클래스 게이지 초기값, 긴 노트를 쥔 채 Esc 로 끝난 판, 엔진 규칙이 바뀐 뒤의 옛 기록) | 해당 칸 |
| 연습(원본 `BMSPlayer.java:441-442`) | 둘 다 |
| 고스트 작업 취소(차트를 떠남), 스레드를 띄우지 못함 | 둘 다 |
| 로딩 중 순위표가 도착해 IR BEST 가 새로 생김(로딩 시점에 출처를 정하므로) | 목표 |

물러날 때 화면 표시는 하지 않는다(질문 4). 콘솔에 `record run <파일> is paced evenly: <이유>` 한 줄을 쓴다.

## 7. 테스트

| 테스트 | 위치 |
| --- | --- |
| 소비 순서와 空POOR 제외, MissCondition::One 의 세지 않는 POOR, 목록이 커지지 않음 | `crates/rbms-judge/src/tests.rs`(`a_run_lists_every_note_it_consumed_in_the_order_it_did`, `a_consumed_poor_that_is_not_counted_still_takes_its_place`, 기존 용량 테스트 두 개에 `vanished` 추가) |
| 한 번에 play_out == 1ms·60Hz 스텝 재생(summary, judged, vanished) | `crates/rbms-play/tests/replay_reproduction.rs`(`playing_a_replay_out_in_one_step_judges_it_as_playing_it_frame_by_frame`) |
| 같은 EX 의 가장 오래된 리플레이 있는 판, 어시스트·다른 LN 건너뜀 | `crates/rbms-store/src/tests.rs`(`the_best_with_a_replay_is_the_oldest_run_of_the_best_score`) |
| 누적합, POOR 채움, 저장한 판 재판정 == 원래 판, 물러나는 조건 6가지, IR BEST 같은 초 | `apps/rbms-player/src/ghost/tests.rs` |
| 플레이 화면: LOCAL BEST 진행 값·HUD 차·스킨 숫자, 고정 목표는 비례, SCRATCH AUTO 최고는 고스트 아님, 연습은 고스트 없음 | `apps/rbms-player/src/stage/play/target_tests.rs` |
| 로딩 게이트(스킨 없음: LOADING 화면이 기다리고 결정음 없음, 스킨 있음: Preload 유지) | `apps/rbms-player/src/stage/play/screen_tests.rs`(`..._for_its_record_runs`, `..._until_its_record_runs_are_in`) |

모든 테스트는 시각 비교 없이 입력과 결과만 비교한다.

## 8. 남은 위험과 테스트 부채

- 긴 차트(5만 노트)의 RON 파싱·모델 복제·재판정 시간을 재지 않았다. 작업 스레드라 프레임은 멈추지 않지만 로딩이 그만큼 늦을 수 있다(테스트 부채 3.75).
- 고스트 스레드는 저장 스레드의 `settled` 를 상한 없이 기다린다. 저장 스레드가 멈추면 로딩이 끝나지 않는다(테스트 부채 3.76).
- `PlayState::attach_ghosts` 의 IR BEST(계정 고스트) 분기와 고스트의 최종 EX 가 그 칸의 점수와 다를 때 쓰지 않는 분기는 테스트가 없다(테스트 부채 3.77).
- BEST 칸(스킨 152/172, rate 112, Lua `rate_best`)이 목표와 무관하게 리플레이 기준으로 바뀐다. 원본과 같아지는 변화지만 보이는 값이 바뀐다.
- 옛 FRAME 시계 리플레이는 시계 검사에, 코스 스테이지 판과 쥔 LN 으로 Esc 한 판은 PG/GR 대조에 걸려 조용히 비례가 된다.
- 기준이 판정된 순서라서 거의 동시인 두 노트를 이번 판과 기록 판이 다른 순서로 판정하면 한두 노트 동안 값이 1~2 흔들린다(원본도 같다).
- 범위 밖 기존 문제(보고만): `from_ir_replay`(`apps/rbms-player/src/ir_replay.rs`)가 와이어에 없는 알고리즘·LN 종류·시계를 `ReplayJudge::default` 로 채운다. 그래서 서버 리플레이(RIVAL, 다른 기기에서 친 IR BEST)는 같은 판정을 보장할 수 없어 비례로 둔다(테스트 부채 3.79).

## 9. 사용자에게 남긴 질문(원본 의미로 구현한 쪽이 A)

1. 기준 인덱스: A. 판정된 노트 수(구현) / B. 같은 곡 시각
2. BEST 칸도 리플레이 기준: A. 바꾼다(구현) / B. 목표 칸만
3. RIVAL·다른 기기 IR BEST: A. 비례 유지(구현) / B. 서버 리플레이 재판정(와이어 보강이 먼저)
4. 비례로 물러날 때 범례 표시: A. 표시 / B. 표시 안 함(지금 구현, 사용자 확인 대기. 테스트 부채 3.78)
5. 같은 EX 여럿: A. 처음 낸 판, 없으면 다음 같은 EX 판(구현) / B. 처음 판만
6. 고스트가 늦을 때: A. 로딩이 기다린다(구현) / B. 비례로 시작
