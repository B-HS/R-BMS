# 플레이 배속(재생 속도) 조사

> 최종 갱신 2026-10-29 · 대응 커밋 c1dc988
> 대상: 요구사항 5 "플레이 배속(100% 기본, 1% 단위로 줄이거나 높이기)". 사용자 결정: 50%~200%, 1% 단위, 재생 속도 자체를 바꿔 피치도 함께 변함(beatoraja `FrequencyType.FREQUENCY`), 새 의존성 없음, **100% 가 아닌 판은 로컬 저장·서버 전송·리플레이 저장을 모두 하지 않는다**(기존 "사람이 친 판은 전부 저장·전송" 정책, [ADR](../../acknowledge/2026-10-14-adr-save-and-send-every-played-run.md)의 명시적 예외).
> 읽기 전용 조사다. 소스는 고치지 않았다. 실행한 검증: `cargo test -p rbms-judge --locked --lib a_slice_played_at_another_frequency` 1건 통과(아래 1.4).

## 0. 결론 먼저

1. **배속의 엔진은 이미 있다.** 연습 패널의 FREQUENCY(50~200%)가 쓰는 길이 그대로 일반 판에 쓸 수 있다: 오디오 피치(`PracticeClock::pitch`), 오디오 시계 ↔ 차트 시계 변환(`PracticeClock::chart_time_us`/`engine_time_us`), 판정 창을 실시간 길이로 유지하는 `PlaySession::set_playback_rate`, 판정 오프셋의 실시간 환산(`field_lead_us`). 새로 만들 것은 "연습이 아닐 때도 배율을 갖는 시계"와 설정 값·UI·기록 차단이다.
2. 오디오는 **미리 디코딩된 PCM 을 목소리마다 소수 보폭(stride)으로 선형 보간해 읽는 구조**라, 배율은 목소리를 시작할 때 `pitch` 에 곱하는 것으로 끝난다(지금 연습이 그렇게 한다). 믹서·디코더는 바꿀 필요가 없다.
3. 판정 창은 **실시간 기준**(beatoraja 와 같음)이 이미 구현·시험돼 있다. 입력 시각은 오디오 시계로 받아 차트 시각으로 바꾼다.
4. 기록 차단은 `app_result.rs` 의 세 입구(`show_result`→`record_run`, `keep_run_cut_short_by_quitting`, `record_retried_run`)가 모두 이미 `state.practice.is_some()` 으로 연습을 막고 있어, 같은 자리에 "배속 판" 조건을 더하는 것이 가장 작은 변경이다. 결과 화면을 보여 줄지(연습은 안 보여 줌)만 다르다.
5. 연습 경로에서 발견한 기존 결함 후보 2건(판정 그래프 커서의 이중 배율, 리듬 타이머가 배율을 모름)은 배속을 일반 판에 넓히면 그대로 드러나므로 함께 고치는 것을 권한다(4절).

## 1. 현재 구조

### 1.1 오디오(`crates/rbms-audio`)

| 사실 | 근거 |
| --- | --- |
| 키음·BGM 은 로드 때 전부 디코딩된 PCM(`Arc<[f32]>`, 원래 샘플레이트)이다. 스트리밍이 아니다 | `crates/rbms-audio/src/mixer.rs:134-144`(`SampleData { pcm, channels, rate }`), 로드 `engine.rs:446-460`(`load`, `insert_decoded`), 앱의 디코드 작업 `apps/rbms-player/src/app_play.rs:556-566`(`spawn_keysound_decode`) |
| 장치 샘플레이트로의 리샘플과 피치는 같은 한 값, 목소리의 읽기 보폭이다: `stride = sample.rate / out_rate * pitch` | `mixer.rs:296-300`(`start_in_place`) |
| 보간은 선형(두 프레임 사이 `frac`), 매 출력 프레임 `pos += stride` | `mixer.rs:609-706`(`mix`, 673-678 의 `i`·`frac`, 706) |
| 피치 하한 `MIN_PITCH_RATIO = 0.0001` | `mixer.rs:37-39` |
| 같은 id 의 재생은 채널 키 `id * 256 + 반음 오프셋 + 128` 로 겹침·끊음이 갈린다(원본 `AbstractAudioDriver.java:507-509`). 반음 오프셋은 `round(12 * log2(pitch))` | `mixer.rs:21-26, 89-113` |
| 엔진의 재생 API 는 `play_on(bus, id, gain, pan, pitch, at_us)` | `engine.rs:482-487` |
| 차트 소리는 `PlayAudioSink` 한 곳에서 엔진으로 간다. 시각은 `anchor + clock.engine_time_us(song_us)`, 피치는 `clock.pitch(sound.pitch)` | `apps/rbms-player/src/play_sink.rs:30-58` |

배속을 넣을 지점: **이미 `play_sink.rs:55` 에서 `PracticeClock::pitch` 로 곱하고 있다.** 일반 판에 배율을 주려면 시계(`PracticeClock`)가 연습이 아닐 때도 배율을 갖게 하면 된다. 믹서는 손대지 않는다.

음질·성능 영향(코드 읽기로 판단):

- 성능: 목소리마다 보폭만 달라지고 연산량은 같다. 200% 에서 같은 실제 시간에 노트가 두 배 몰리지만 각 소리의 실제 길이가 절반이라 동시 발음 수는 대략 같다(50% 는 반대). 폴리포니(설정 `audio.polyphony` 기본 512, `engine.rs:14` 의 기본값 주석)에 새 압박은 없다.
- 음질: 선형 보간에 저역 통과 필터가 없어, 보폭이 1 을 넘는 경우(예: 44.1 kHz 샘플·48 kHz 장치에서 200% 는 보폭 약 1.84) 고역 에일리어싱이 생긴다. 지금 연습 FREQUENCY 200% 와 같은 수준이다. beatoraja FREQUENCY(OpenAL/libGDX 의 `setGlobalPitch`)의 보간 품질은 미확인.
- 채널 키 양자화: 101~105% 는 반음 +1 로, 95~97% 는 -1 로 양자화된 키에 들어간다(보폭은 실제 배율 그대로). 한 판의 모든 소리가 같은 배율이므로 "같은 id·같은 키면 끊음" 관계는 100% 와 같다. 앱이 `stop_pitched` 를 부르는 곳은 없다(`grep` 결과 정의만 있음, `engine.rs:520`). 판을 끊는 `stop_chart_sounds` 는 id 전체를 멈춘다(`app_play.rs:685-694`).

### 1.2 시계(오디오 시각 → 차트 시각)

| 단계 | 내용 | 근거 |
| --- | --- | --- |
| 판 시작 | 오디오 시계의 현재 값을 `anchor_us` 로 잡는다 | `app_play.rs:954-960`(`start_play`) |
| 프레임마다 | `song_and_lookahead_us` = 들리는 오디오 시각 - anchor(단조 클램프) | `app_play.rs:1054-1059` |
| 입력 | `song_us_at(at)` = 입력이 들어온 순간의 오디오 시각 - anchor | `app_play.rs:1068-1072` |
| 차트 시각으로 | `PracticeClock::chart_time_us(engine_elapsed) = start_us + elapsed * freq / 100` | `apps/rbms-player/src/practice.rs:548-550` |
| 소리를 예약할 때 | `engine_time_us(chart) = (chart - start_us) * 100 / freq` | `practice.rs:552-554`, `play_sink.rs:45-50` |
| 프레임 진행 | `tick_run` 이 위 변환으로 `song`(들리는 차트 시각)과 `scheduled_us`(예약 선행)를 만든다 | `apps/rbms-player/src/stage/play/mod.rs:1329-1353` |
| 입력 판정 | `lane_input` 이 `clock.chart_time_us(shared.song_us_at(at))` 로 바꿔 세션에 넘긴다 | `stage/play/mod.rs:867-890`(875-876) |
| 시계 선택 | 연습이면 그 구간의 시계, 아니면 `PracticeClock::normal()`(시작 0, 100%) | `stage/play/mod.rs:417-419`(`practice_clock`) |

`PracticeClock` 의 상수는 `FREQ_MIN = 50`, `FREQ_MAX = 200`, `FREQ_UNMODIFIED = 100`(`practice.rs:106-108`)이고, `new` 가 범위로 자른다(`practice.rs:540-542`). 사용자 결정의 범위(50~200)와 같다.

`apps/rbms-player/src/timing.rs` 는 이 작업과 관계없다(파일 목록의 이름만 확인, 내용 미열람. 시계 관련 함수는 모두 `app_play.rs` 에 있음을 grep 으로 확인). `scene_clock.rs` 는 스킨 장면 시계(실시간)이고 차트 시각을 모른다(`apps/rbms-player/src/scene_clock.rs:14-44`).

### 1.3 스킨 타이머와 화면이 읽는 차트 시각

| 사실 | 근거 |
| --- | --- |
| 스킨의 PLAY 타이머는 매 프레임 `now - chart_us` 로 다시 놓인다. 그래서 `now - PLAY` 는 **차트 시각**(배율이 이미 곱해진 값)이다 | `apps/rbms-player/src/skin_host/play_timers.rs:708-713` |
| 시작 순간 `timers.set_on(PLAY, now - start_offset_us)` | `play_timers.rs:656-660`, `start_offset_us = practice_clock().chart_time_us(0)`(`stage/play/mod.rs:1417`) |
| 노트 필드·BGA 가 읽는 시각 `played_chart_us` 는 `now - PLAY` | `stage/play/mod.rs:1021-1032`, 필드 `drawn_chart_us` 1042-1044, BGA `background_frame` 1156-1162 |
| 판정 오프셋은 실시간 길이로 환산돼 필드에만 더해진다 | `stage/play/mod.rs:623-626`(`field_lead_us`), 문서 `docs/play-options.md` 3.1절 5번 |
| 리듬 타이머: `slowed = 100 - bpm * play_speed / 60`, `delta_us` 는 장면 시계(실시간), `bpm` 은 차트 BPM, `play_speed` 는 분석 모드가 아니면 늘 100 | `play_timers.rs:714-716`, `stage/play/mod.rs:1372-1377`(`play_speed_percent`) |
| 마디선 시각에 `100 / practice_freq` 를 곱하는 칸이 있지만 앱은 이 값을 100 으로만 둔다(차트 시각과 비교하므로 맞다) | `play_timers.rs:315-319, 727-733`. `practice_freq` 설정처는 테스트뿐(grep) |
| 판정 그래프의 커서: `played = now - PLAY`(차트 시각)에 `PlayCursor.speed` 를 다시 곱한다 | `crates/rbms-render/src/skin_render/graphs/notes_dist.rs:407-422, 507-508`, `speed` 는 연습 때 `freq / 100`(`stage/play/mod.rs:1108-1118`) |
| 스킨이 읽는 BPM·이동 시간·녹색 숫자는 차트 BPM 으로 계산된다(`Scroll { now_bpm, main_bpm, min_bpm, max_bpm, region_ms }`) | `stage/play/mod.rs:1088-1094`, `skin_host/play.rs:60-100`(`duration_at`), 내장 필드의 녹색 숫자 `stage/play/mod.rs:1690-1700` |
| KEEP GREEN NUMBER 가 잡는 기준 템포도 차트 BPM | `stage/play/mod.rs:536-561`(`held_tempo`, `hold_green_number`), `app_input.rs:59-72` |
| 남은 시간(number 163·164)은 `play_time_ms - played_ms`, 곧 차트 시간 | `skin_host/play.rs:189, 265-266` |
| 곡의 BGA 동영상은 그리지 않는다. `rbms-video` 는 스킨의 동영상 source 전용이고 스킨 장면 시계(실시간)를 따른다 | `docs/skin.md` 2.6절 "한계", `apps/rbms-player/src/skin_screen/movies.rs:103-150`(`into_movie_us = now_us - started_us`) |

### 1.4 판정(`crates/rbms-judge`, `crates/rbms-play`)

| 사실 | 근거 |
| --- | --- |
| 엔진은 차트 시계로 구동되지만, `set_playback_rate(percent)` 를 주면 모든 창·LN 마진·후보 범위·HCN 틱을 **플레이어의 실시간 길이**로 늘려 읽고, 보고하는 타이밍(FAST/SLOW ms, 자동 오프셋, 통계)도 실시간이다 | `crates/rbms-judge/src/matcher.rs:48-66`(`PlaybackRate` 설명), 364-365, 729-742 |
| 하한 1%(`MIN_PLAYBACK_PERCENT`) | `matcher.rs:48-49` |
| 세션이 그 값을 기억하고 분석 `seek` 재구성에도 다시 건다 | `crates/rbms-play/src/session.rs:279-280, 913-922, 1227` |
| 지금 이 값을 거는 곳은 연습뿐이다 | `stage/play/mod.rs:405-415`(`set_practice` 의 `self.session.set_playback_rate(practice.freq_percent)`) |
| 시험: `a_slice_played_at_another_frequency_is_judged_in_the_players_own_time` | `crates/rbms-judge/src/tests.rs:1882-1940`. 2026-10-29 실행 통과 |
| 문서: 연습 FREQUENCY 의 판정은 실시간 기준. "전에는 50% 에서 실제 30 ms 늦은 입력이 PGREAT" 였던 것을 고친 기록 | `docs/play-options.md` 9.3절 "재생 속도와 판정" 행 |

beatoraja 의 방식(저장소 밖 원본 소스 `~/development/beatoraja` 에서 확인):

- 연습에서만 FREQ 가 있다. 시작할 때 `BMSModelUtils.changeFrequency(model, freq/100)` 로 **차트 자체를 다시 배치**하고, `FrequencyType.FREQUENCY` 면 `audio.setGlobalPitch(freqRate)`, `SPEED` 면 피치 그대로 `audio.setModel(model, freqRate)`(타임스트레치) — `src/bms/player/beatoraja/play/BMSPlayer.java:554-568`. 기본값은 `FREQUENCY`(`AudioConfig.java:37`).
- 그 뒤 `JudgeManager` 는 같은 창 길이로 판정하므로 창은 실시간 길이다(R-BMS 의 `matcher.rs:53-58` 설명과 일치). 연습 구간 시각도 `* 100 / freq` 로 환산(`BMSPlayer.java:567-568, 585-586`).
- 리듬 타이머는 `property.freq` 를 받아 마디선에 `100 / freq` 를 곱한다(`RhythmTimerProcessor.java:60-76`, `BMSPlayer.java:628`). 판정 그래프 커서는 `currenttime *= freq`(`SkinNoteDistributionGraph.java:217-218`). 둘 다 원본의 타이머가 실시간이기 때문이다.
- `changeFrequency` 가 BPM 을 `freq` 배로 바꾸는지(그래서 스크롤 속도·녹색 숫자가 실시간 기준이 되는지)는 `lib/jbms-parser.jar` 안에 있어 소스로 확인하지 못했다(미확인). 차트 시각을 나누기만 하고 BPM 을 그대로 두면 스크롤 위치가 어긋나므로 BPM 도 곱한다고 보는 것이 자연스럽다.
- 일반 플레이에는 배속이 없고, 오토·리플레이 감상의 재생 속도(`setPlaySpeed`, 25/50/200/300%)만 있다(`BMSPlayer.java:791-796`, `ControlInputProcessor.java:211-219`). 기록은 연습 판이라 남지 않는다.

### 1.5 기록 저장 경로

| 입구 | 지금의 연습 차단 | 근거 |
| --- | --- | --- |
| 결과 화면(끝까지, Esc 로 나간 판) `show_result` → `record_run` | `record_run` 안에서 `practice` 가 리플레이 저장(`!practice`), 최고 기록 갱신(`updates_score`), 서버 전송 사유(`ir_submission_block_reason`), DB 쓰기(`!practice`)를 모두 막는다. `show_result` 는 연습이면 결과 화면 없이 `Transition::Back` | `apps/rbms-player/src/app_result.rs:104-208`(115, 186-188), `record_run` 268-470(288, 290-291, 372-373, 418) |
| 플레이 중 앱 종료 `keep_run_cut_short_by_quitting` | 첫 줄 `state.practice.is_some() \|\| !keeps_a_left_run` 이면 아무것도 남기지 않음 | `app_result.rs:569-575` |
| 실패 화면에서 START·SELECT 로 곧장 재시작 `record_retried_run` | 첫 줄 `practice \|\| autoplay \|\| replay` 이면 return | `app_result.rs:610-618` |
| Esc 로 나가기 `leave_run` | 연습이면 패널로 돌아감. 그 밖은 결과 화면으로(`enter_result_of_a_left_run`) | `stage/play/mod.rs:723-743` |
| 서버 전송 사유 문자열 | `ir_submission_block_reason(autoplay, replay, practice)` → `practice_block_reason` → `"practice"` | `apps/rbms-player/src/lib.rs:343-352`, `practice.rs:55, 763-768` |
| 최고 기록 갱신 여부 | `updates_score = block_reason.is_none() && !custom_judge` | `lib.rs:358-360` |
| 결과 화면 IR 줄 `IR: SKIPPED (사유)`, 사유 번역 | `SkipReasonText`(Autoplay·ReplayPlayback·Practice·NotSignedIn) | `apps/rbms-player/src/ir_outcome.rs:72-79, 105-107`, `text/menus/ir_outcome.rs:55-64` |
| 결과 화면 오른쪽 위 어시스트 노란 줄 | `assist_note(custom_judge, own_total, scratch_auto)` | `apps/rbms-player/src/stage/result.rs:848-859, 708` |
| 스킨이 읽는 "점수 갱신 판"(`updates_score`) | `ResultRun::of` 가 연습 인자를 **`false` 로 고정**해 계산 | `stage/result.rs:672-712`(694) |
| 판별 trace·저장 스레드·보류 판 | 모두 `record_run` 의 DB 쓰기 분기 안에서만 생긴다 | `app_result.rs:418-460`(`keep_trace`, `keep_on_the_side`) |
| 리플레이 파일 | `record_run` 의 `save_replay` 조건 안 | `app_result.rs:290-291, 376-415` |
| 다시 하기(R, 파란 건반+START) | `offers_retry` 는 오토·리플레이·라이브러리 밖만 막는다 | `app_result.rs:900-902` |
| 결과 화면의 스킨 기록(`run_records`) | `keep_run_record` 는 저장과 무관한 화면용 | `app_result.rs:649-655` |

`save_worker.rs`·`play_trace.rs`·`ir_sync`·`record_sync` 는 `record_run` 이 행을 만든 뒤에만 움직인다(위 표 근거). 따라서 `record_run` 에 들어가기 전, 또는 그 안의 `practice` 와 같은 자리에서 막으면 저장·전송·리플레이·trace·동기화가 한꺼번에 막힌다.

### 1.6 연습 모드

`docs/play-options.md` 9.1절: FREQUENCY 행이 이미 있다(50~200, 한 칸 1, Shift 10, 0 키 100). 저장은 `practice.ron`(차트별), 연습은 기록·전송을 하지 않는다(9.3절). 앱의 문구 `Frequency => ("FREQUENCY", "재생 속도", "再生速度")`(`apps/rbms-player/src/text/menus/practice.rs:28`). 연습 구간 시작은 `app_play.rs:789-805`.

### 1.7 설정과 UI 의 자리

| 곳 | 사실 | 근거 |
| --- | --- | --- |
| 설정 구조체 | `PlayOptions` 는 `#[serde(default)]`. 같은 성격의 어시스트 값 `total_override` 가 여기 있다 | `crates/rbms-config/src/schema.rs:538-542, 595, 627`, 정리 `schema.rs:525` |
| 설정 행 | `SettingId` 열거와 행 표. `SettingTab::PlayChart` 에 넣으면 PLAY 메뉴 CHART 묶음에 자동으로 나온다 | `crates/rbms-config/src/settings.rs:175-255, 441, 637-680`(`TargetRate` 행이 정수·% 의 본보기), PLAY 메뉴 목록 `apps/rbms-player/src/stage/play_menu/rows.rs:230-257` |
| 행 이름 번역 | `RowLabelText`(예: `TargetRate => ("TARGET RATE", "타깃 비율", "ターゲット率")`) | `apps/rbms-player/src/text/settings/labels.rs:53, 160`, 설명 `text/settings/descriptions.rs` |
| F1 오버레이 | `OPTION_ROWS` 에 행 id 를 넣으면 나온다 | `apps/rbms-player/src/app_options.rs:39-49` |
| 선곡 패널 1~3 | 확장 표에서 빈 칸이 사실상 없다(패널 1 의 7번이 2번과 같은 RANDOM 다음으로 중복, 패널 2 의 1번이 읽기 전용 EX JUDGE) | `docs/play-options.md` 12.2절, `stage/select/keys.rs` |
| 플레이 중 키 | 배율은 세션 시작 전에만 걸 수 있다(`set_playback_rate` "Call it before the first tick") | `crates/rbms-play/src/session.rs:913-918` |
| 설정 파일의 계약 골든 | 설정 업로드 본문이 `contract/client-requests/settings-blob.json` 과 바이트 비교된다. `play` 에 필드가 생기면 갱신 필요 | `apps/rbms-player/src/app_result/record_tests/contract_requests.rs:30-31, 469-494, 943-951` |
| 스킨 확장 id | 정수·불린 대역 21000~21199, 배정표는 ADR 이 정본 | `docs/acknowledge/2026-10-24-adr-rbms-extension-id-spaces.md` 3)절 |

### 1.8 Discord

`run_kind` 가 연습·리플레이·오토·손으로를 가른다(`apps/rbms-player/src/discord_presence.rs:237-246`). 시간은 판 시작 시각부터의 경과(`mark`, 165-175)라 배율이 시간을 어긋나게 하지 않는다. 종료 예정 시각을 보내지 않는다.

## 2. 요구사항별 구현 지점과 설계안

### 2.1 시계: 연습 밖에서도 배율을 갖게

권장안 A(작은 변경): `PlayState` 에 `playback_percent: i32`(연습이 아닐 때의 배율)를 두고, `practice_clock()` 을 "연습이면 그 구간 시계, 아니면 `PracticeClock::new(0, playback_percent)`"로 바꾼다(`stage/play/mod.rs:417-419`). 그 한 곳으로 `tick_run`·`lane_input`·`field_lead_us`·`PlayAudioSink` 의 피치·예약·`push_timing_sample`·`start_offset_us` 가 모두 따라온다(1.2절 표의 근거들). 같은 값을 `self.session.set_playback_rate(percent)` 로 세션에도 건다(첫 tick 전에. `set_practice` 와 같은 자리, 또는 `app_play.rs:623-624` 의 `PlaySession::new` 직후).

- 이름: `PracticeClock` 이 일반 판에도 쓰이면 이름이 어긋난다. `RunClock`/`PlaybackClock` 으로 바꾸고 `practice.rs` 밖(예: `play_sink.rs` 옆)으로 옮기는 것이 읽기 쉽다. 단 rename 은 diff 를 키운다(최소 변경 원칙과 저울질). 범위 상수 `FREQ_MIN/MAX/UNMODIFIED` 는 설정 크레이트에 하나만 두고(예: `rbms_config::PLAYBACK_MIN_PERCENT` 등) 연습과 공유해 매직넘버 중복을 피한다.
- 대안 B: 차트 자체를 beatoraja 처럼 다시 배치(시각 / 배율, BPM x 배율). 판정·오디오·스킨 쪽은 손이 덜 가지만, 리플레이·trace·ghost·기록의 "차트 시각"이 모두 바뀐 차트 기준이 되고 연습이 이미 택한 방식과 갈린다. 비추천.

배율을 강제로 100 으로 두는 판(권장): 리플레이 감상(`shared.replay.is_some()`. 기록된 판은 모두 100% 이므로 다른 배율로 재생하면 판정이 달라진다), 연습(자기 FREQUENCY 를 쓴다. 겹쳐 곱하지 않는다), 코스(2.4절). 오토플레이는 배율을 따라도 된다(기록이 원래 없다).

### 2.2 판정 창의 기준

권장: **실시간 기준(지금 연습과 같음)**. beatoraja 연습이 그렇고(1.4절), 엔진·시험이 이미 있다. 입력 시각 변환은 지금 그대로다: 오디오 시각(실시간) → `chart_time_us` → 세션. `docs/play-options.md` 3.4절의 이벤트 시각 판정(키보드·패드 디바운스, 지연 릴리스, HCN 200 ms 틱)은 모두 실시간 길이를 유지한다(`matcher.rs:53-58`).

대안(차트 시각 기준 창)은 200% 에서 PGREAT 창이 실제 절반이 되어 같은 실력에도 훨씬 어려워진다. 기록이 남지 않으므로 공정성 문제는 없지만, 원본·연습과 다르고 엔진에 새 모드를 넣어야 한다. 비추천.

### 2.3 화면: 스크롤·녹색 숫자·BGA·타이머

| 항목 | 그대로 두면 | 권장 |
| --- | --- | --- |
| 노트 스크롤 | 필드는 차트 시각으로 그리므로, 같은 HI-SPEED 에서 200% 면 실제로 두 배 빨리 내려온다(beatoraja 연습의 FHS 꺼짐과 같은 체감) | 그대로 |
| KEEP GREEN NUMBER 켬 | 기준 템포가 차트 BPM 이라 녹색 숫자는 차트 시간 기준으로 유지되고, 실제 이동 시간은 녹색/배율이 된다(200% 면 체감 절반) | 기준 템포를 `bpm * 배율 / 100` 으로 넘겨 **실시간 녹색 숫자를 유지**(`held_tempo` `stage/play/mod.rs:536-541`, `hold_green_number` 554-561). beatoraja 연습의 FHS 가 `changeFrequency` 후 다시 계산하는 것과 같은 결과(BPM 이 곱해진다는 전제는 1.4절의 미확인) |
| 스킨·내장 화면의 녹색 숫자·이동 시간(313 등)·현재 BPM(160) | 차트 시간 기준 값이 보인다 | `Scroll` 의 네 BPM 에 배율을 곱해 넘기면 표시가 실시간 기준이 된다(`stage/play/mod.rs:1088-1094`, 내장 1690-1700). 현재 BPM 표시는 "들리는 템포"가 되어 원본 연습과 같아진다. 대안: 그대로 두고 배율만 따로 보인다(혼동 소지) |
| BGA 이미지 | `played_chart_us`(차트 시각)로 바뀐다 → 곡과 맞다 | 그대로 |
| 미스 레이어 지속 | 차트 ms 로 재면 실제 지속이 1/배율 | 영향 작음. 그대로(미확인: `BgaPlayhead` 의 `miss_duration_ms` 가 어느 시계로 비교되는지 미열람) |
| 곡 BGA 동영상 | 지원하지 않음(1.3절) | 해당 없음 |
| 스킨 동영상 | 장면 시계(실시간) | 그대로 |
| 리듬 타이머 | 차트 BPM x 100 으로 계산돼 배율이 있으면 박에서 어긋난다(마디선마다 다시 맞춤) | `PlayingFrame.play_speed` 에 배율을 넣는다(`play_speed_percent` `stage/play/mod.rs:1372-1377` 에서 분석 모드가 아니면 100 대신 배율). 원본 연습의 `bpm * freq` 와 같은 식이 된다 |
| 판정 그래프 커서 | 연습에서는 이미 차트 시각인 값에 배율을 한 번 더 곱한다(4절 기존 결함 후보) | 일반 배속 판에는 `PlayCursor.speed` 를 넣지 않는다(`None`). 연습도 고치는 것을 권함 |
| 남은 시간(163·164) | 차트 시간 기준(200% 면 실제보다 두 배 길게 보인다) | `left_ms` 에 `100/배율` 을 곱하면 실제 남은 시간. 선택 사항 |
| READY·FADEOUT·Esc 홀드 등 장면 시간 | 실시간 | 그대로 |
| 게이지 이력(0.5초 경계) | 차트 시간 경계(`docs/play-options.md` 3.4절 표) | 그대로(결과 그래프 가로축은 차트 진행) |

### 2.4 기록 차단과 결과 화면

권장 단일 지점: `PlayState` 에 "기록을 남기지 않는 판" 판정을 하나 둔다. 예: `fn keeps_no_record(&self) -> bool { self.practice.is_some() || self.playback_percent != 100 }`(이름은 구현자 판단). 그리고

1. `record_run`(`app_result.rs:288`)의 `let practice = state.practice.is_some()` 를 그 판정으로 바꾸면 리플레이 저장(290-291)·DB 쓰기(418)·`updates_score`(372)가 함께 막힌다.
2. 서버 전송 사유는 따로 둔다: `ir_submission_block_reason` 에 배속 사유를 더하거나(`lib.rs:343-352`) `record_run` 에서 연습 사유 대신 배속 사유를 고른다. 새 `SkipReasonText::PlaybackRate`(세 언어)를 `text/menus/ir_outcome.rs:58-63` 에 추가 → 결과 화면 `IR: SKIPPED (재생 속도 110%)` 같은 줄. 사유 문자열은 영어 원문으로 번역을 찾는 구조라(`ir_outcome.rs:105-107`) 숫자를 넣으려면 `{0}` 자리표시가 있는 별도 항목이 필요하다. 숫자 없이 고정 사유(`"playback rate"`)가 가장 단순하다.
3. `keep_run_cut_short_by_quitting`(569-575)·`record_retried_run`(610-618)의 첫 줄 조건도 같은 판정으로 바꾼다.
4. `show_result` 의 `if practice { return Transition::Back }`(186-188)는 **연습에만** 남긴다. 배속 판은 결과 화면을 보여 준다(`leave_run` 의 연습 분기 723-731 도 연습만).
5. 결과 화면 안내: `assist_note` 처럼 노란 줄 하나 `PLAYBACK 110%: NOT SAVED, NOT SENT`(세 언어, `IR_LINE_MAX_CHARS` 시험 `text/menus/ir_outcome.rs` 의 `an_assist_note_fits_a_line_…` 를 통과하는 길이). 배속이 어시스트보다 우선(저장되지 않으므로 어시스트 램프 설명은 뜻이 없다). 자리: `ResultRun`(`stage/result.rs:672-712`)에 배율을 실어 `assist_note` 선택에 넣는다.
6. `ResultRun::of` 의 `updates_score`(694)가 연습 인자를 `false` 로 고정하고 있으므로, 배속 판에는 `false` 가 되게 고쳐야 스킨이 NEW RECORD·점수 갱신 표시를 하지 않는다.
7. 결과 화면의 "이전 최고"와의 차이는 그대로 보여 줘도 된다(저장되지 않을 뿐).
8. 다시 하기(R, 파란 건반+START): 설정의 배율로 다시 시작되므로 같은 배속·같은 배치가 된다(`offers_retry` 는 바꾸지 않아도 됨, `app_result.rs:900-902`). 실패 화면 즉시 재시작(`may_retry`, `stage/play/mod.rs:1428`)도 같다.
9. 코스: 권장안은 **코스 중에는 배율을 100% 로 강제**(코스 기록·코스 게이지·스테이지 접기 `fold_course_stage` 를 건드리지 않는다). 선곡에서 코스를 시작할 때 배율이 100 이 아니면 알림 한 줄. 대안: 코스 전체를 기록 없이 진행(코스 결과·`abandon_course`·`course_ir` 전부에 분기 필요, 비추천). 사용자 확인 필요(5절).
10. 연습: 연습 패널의 FREQUENCY 가 그대로 담당. 전역 배율은 무시(겹쳐 곱하지 않음).

### 2.5 UI

| 곳 | 제안 | 비고 |
| --- | --- | --- |
| PLAY 메뉴 CHART 묶음 | 새 `SettingId::PlaybackRate`, `SettingKind::IntRange { min: 50, max: 200, step: 1, unit: "%" }`, Shift 10, 0 키 100. 표시 `100%` | `TargetRate` 행과 같은 모양(`settings.rs:660-666`, 처리 1118·1328·1559·1602·1645·1896 의 각 `match`). 정보 패널 설명에 "100% 가 아니면 기록·전송·리플레이가 남지 않음" |
| F1 오버레이 | `OPTION_ROWS` 에 추가(`app_options.rs:39-49`) | 내장 선곡에서 바꾸는 유일한 길 |
| 선곡 옵션 패널 | 확장 표에 빈 ± 쌍이 없다. 넣으려면 기존 칸을 옮겨야 하고 `CHANGED_SLOTS` 시험·기본 스킨 패널 그림이 바뀐다 | 1차 범위에서 제외 권장(5절 질문) |
| 선곡 정보·플레이 화면 표시 | 100% 가 아닐 때만 `RATE 110%` 같은 표지. 기본 스킨에 보이려면 새 확장 id(정수: 배율, 불린: 100 이 아님)를 ADR 배정표에 더하고 `lua/i18n/*.lua` 세 언어 문구 | 다른 작업(선곡 UI·밀도)이 같은 대역에 번호를 배정할 수 있어 메인이 번호를 정해야 한다 |
| 플레이 중 키 | 두지 않는다(세션 시작 전에만 걸 수 있다, 1.7절) | |
| 표시 형식 | 정수 `%`. 배수(`x1.10`)는 쓰지 않음(1% 단위와 맞춤) | |
| 문구(초안) | 행 이름 `PLAYBACK RATE` / `재생 속도` / `再生速度`(연습의 `Frequency` 와 한·일 용어 통일). 결과 줄 `PLAYBACK {0}%: NOT SAVED, NOT SENT` / `재생 속도 {0}%: 저장·전송 안 됨` / `再生速度{0}%: 保存・送信なし`. IR 사유 `playback rate` / `재생 속도 변경` / `再生速度変更` | 세 언어 카탈로그 `catalog_problems` 시험이 지킨다 |
| 설정 저장 | `PlayOptions` 에 `playback_percent: i32`(기본 100, `sanitise` 에서 50..=200 으로 자름). 저장 여부는 5절 질문 | 저장하면 settings-blob 계약 골든 갱신 |

### 2.6 Discord·IR·리플레이

- Discord: 필수 변경 없음. 원하면 `RunKind`(`discord_presence/activity.rs:82-87`)에 배속을 더해 상태 줄에 `110%` 를 붙일 수 있다(선택).
- IR: 배속 판은 전송하지 않으므로 서버(`web/`) 변경 없음. `ScoreSubmission.options` 에 배율 필드를 더할 필요도 없다.
- 리플레이: 저장하지 않으므로 `Replay`·`ReplayJudge` 형식 변경 없음. 리플레이 감상은 100% 강제(2.1절). "리플레이의 배치로 치기"(`docs/play-options.md` 10.15절)는 손으로 치는 판이라 설정 배율을 따른다(100 이 아니면 기록 없음).

## 3. 영향받는 기존 기능·테스트·골든

| 영역 | 내용 |
| --- | --- |
| 시계 일반화 | `practice.rs`(`PracticeClock` 와 상수), `play_sink.rs`(타입 이름), `app_play.rs:391, 1076`(`timing_quantized_us`, `push_timing_sample` 의 인자 타입), `stage/play/mod.rs` 의 `practice_clock` 사용처 8곳(417, 624, 875, 1030, 1331-1349, 1417, 1548). 시험: `play_sink.rs:84-104`, `app_play.rs:1564-1573`, `practice/tests.rs:147, 709`, `stage/play/timing_tests.rs:193-206` |
| 판정 | 엔진 변경 없음. 기존 시험 `rbms-judge/src/tests.rs:1882-1940`, `rbms-play/src/session.rs:1946` |
| 스킨 타이머 | `play_speed_percent` 변경 시 `skin_host/play_timers/tests.rs` 의 리듬 타이머 시험(433-450 근처 마디선 시험 포함)을 확인 |
| 그래프 커서 | 연습 커서를 고치면 `notes_dist.rs` 관련 시험·연습 캡처가 바뀔 수 있다(미확인: 커서를 단언하는 시험 존재 여부 미조사) |
| 기록 | `app_result/record_tests.rs`(새 시험: 배속 판은 행·리플레이·보류 판·trace 없음, IR 줄 SKIPPED, 앱 종료·즉시 재시작도 없음), `main_tests.rs:249-273`(`ir_submission_block_reason`·`updates_score` 인자가 늘면 고침) |
| 결과 화면 | `stage/result.rs` 의 `assist_note` 시험 상수(836-845), `text/menus/ir_outcome.rs` 의 카탈로그·길이 시험 |
| 설정 | `crates/rbms-config/src/settings.rs` 의 행 목록 시험(2325, 2446 근처의 행 열거·범위 표), `crates/rbms-config/src/tests.rs`(새 필드 왕복·정리), `settings_ui.rs:452` 근처의 표시 표, `text/settings/labels.rs` 의 `SettingId` 매핑 |
| 계약 골든 | `contract/client-requests/settings-blob.json`(필드 추가 시). 갱신 명령은 `RBMS_UPDATE_CONTRACT=1 cargo test -p rbms-player --locked contract_requests` 후 `web/` 에서 `bun test tests/dto/client-requests.test.ts`(`contract_requests.rs:30-31`) |
| PLAY 메뉴 | 행이 하나 늘어 CHART 묶음 스크롤·캡처가 바뀔 수 있다(`stage/play_menu` 시험, 미확인: 행 수를 단언하는 시험) |
| F1 오버레이 | `app_options.rs:456-463` 의 행 위치 시험 |
| 문서 | `docs/play-options.md` 2절 표·7.2절·10.2절 표(새 행 "100% 아닌 판")·9.3절(연습과의 관계), 새 ADR(저장 정책의 예외), `docs/crates.md`(시계 이동 시) |
| 렌더 골든 | `crates/rbms-render/tests/golden` 4장은 필드·프리미티브라 영향 없음(파일 목록 확인) |

## 4. 위험·엣지 케이스

1. **기존 결함 후보 A — 판정 그래프 커서 이중 배율**: 연습 FREQUENCY 에서 커서 위치 = (이미 차트 시각인 `now - PLAY`) x `freq/100`(`notes_dist.rs:420, 507`, `stage/play/mod.rs:1116`, `play_timers.rs:712`). 원본은 타이머가 실시간이라 곱하는 것이 맞지만 여기서는 PLAY 타이머가 차트 시각에 맞춰져 있다. 코드 읽기로만 확인했고 실행으로 보지 않았다.
2. **기존 결함 후보 B — 리듬 타이머**: 배율이 있으면 박에 맞춰 움직이는 스킨 애니메이션이 마디 안에서 어긋난다(1.3절). 연습 FREQUENCY 에도 같은 문제가 있을 것이다(실행 확인 안 함).
3. 오프셋 자동 보정(AUTO ADJUST): 배속 판은 기록되지 않지만 자동 보정은 설정의 `judge.offset_ms` 를 그 자리에서 바꾼다(`keep_adjusted_offset`, `stage/play/mod.rs:610-612`). 실시간 기준이라 값의 뜻은 같지만, "기록 안 되는 판이 설정을 바꾸는 것"을 막을지 결정이 필요할 수 있다(연습도 지금 바꾼다, `docs/play-options.md` 3.2절 1번).
4. 설정에 배율이 남으면 다음 실행에서 모르고 친 판이 저장되지 않는다. 선곡·플레이·결과 세 곳의 표시가 이 위험을 줄인다(5절 질문).
5. 계정 설정 동기화(SYNC SETTINGS)로 다른 기기의 배율이 내려올 수 있다(필드를 저장하는 경우).
6. 장치 샘플레이트: 보폭이 샘플레이트 비와 배율의 곱이라 어떤 장치에서도 같은 식이다. 96 kHz 장치·50% 에서 보폭이 작아져도 하한(0.0001)과 거리가 멀다.
7. 키음 동시 발음: 1.1절대로 대략 불변. 단 50% 에서 긴 BGM 샘플이 두 배 오래 남아 목소리 점유 시간이 늘어난다(폴리포니 512 대비 위험 낮음, 실측 안 함).
8. LN 끝 판정: 일반 LN 마진 0 인 모드의 지연 릴리스·끝 확정은 엔진이 실시간 길이로 처리한다(`matcher.rs:53-58`). CN·HCN 틱 200 ms 도 실시간. 연습 시험이 이를 덮는다(1.4절).
9. 예약 선행: `scheduled_us = chart_time_us(engine + lookahead)` 라 선행이 차트 시간으로 배율만큼 늘거나 준다. 50% 에서 차트로 보면 선행이 절반이지만 실제 시간은 같다. 정수 나눗셈 절사로 µs 단위 오차만 있다(`practice.rs:561-564`).
10. 곡 끝 판정: 소리 길이로 정하는 플레이 시간은 차트 시간 기준이라 배율에 무관(재생 길이/배율 x 배율). 근거는 계산이며 `play_time_ms` 의 구현은 미열람.
11. 오토플레이 + 배속: 기록이 원래 없으므로 문제 없음. 분석 모드(리플레이 감상)의 재생 속도(`analysis_rate`)와는 별개 값이고, 리플레이 감상은 배율 100 강제라 겹치지 않는다.
12. ghost·목표 페이스: 차트 시각 기준이라 배속 판에서도 맞다. 연습만 ghost 를 끈다(`app_play.rs:510`).
13. 코스 속도 잠금(`course_locks_speed`)과 KEEP GREEN NUMBER 의 상호작용: 코스를 100% 강제로 하면 영향 없음.
14. 스킨 미리보기(PLAY 메뉴의 미리보기, `stage/play/skin_preview.rs`)는 자체 시계(`FULL_SPEED_PERCENT`, 304·668)라 배율을 보이지 않는다. 미리보기에도 배율을 보일지는 선택.

## 5. 미확인

1. beatoraja `BMSModelUtils.changeFrequency` 가 BPM 을 배율만큼 곱하는지(jar 안, 소스 없음). 녹색 숫자를 실시간 기준으로 둘 근거로 썼다.
2. beatoraja `setGlobalPitch` 의 보간·필터 품질.
3. `BgaPlayhead` 미스 레이어 지속이 비교되는 시계(`crates/rbms-render/src/skin_render/bga/playhead.rs` 의 구조체만 봄).
4. 기존 결함 후보 A·B 의 실제 화면 증상(실행·캡처하지 않음).
5. PLAY 메뉴 행 수·판정 그래프 커서를 단언하는 시험이 있는지.
6. `PlaySession::play_time_ms` 의 계산식(곡 끝과 배율의 관계를 계산으로만 판단).
7. `apps/rbms-player/src/timing.rs` 의 내용(시계 관련 코드가 `app_play.rs` 에 있음을 grep 으로만 확인).
8. 앱(`rbms-player`) 범위의 연습 FREQUENCY 시험(`timing_tests.rs`, `practice/tests.rs`)은 이번에 돌리지 않았다.
