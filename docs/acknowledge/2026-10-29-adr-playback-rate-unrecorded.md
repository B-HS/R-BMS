# ADR: 플레이 배속(PLAYBACK RATE)은 재생 속도 자체를 바꾸고, 100% 가 아닌 판은 저장·전송하지 않는다

> 최종 갱신 2026-10-29 · 대응 커밋 `9f1b484`
> 상태: 유효(D2·D3 은 사용자 확인 2026-10-29. M5~M9 와 "세부" 절은 메인 추천 또는 구현 단위의 결정, 사용자 미확인. 연습 경로 수리 M8 이 연습의 표시를 바꾼 것은 사용자 확인이 필요하다) · 결정일 2026-10-29 · 결정자 사용자(요청과 D2·D3)와 메인(M5~M9), 구현 단위 U2(세부) · 커밋 `9f1b484`
> 부분 변경하는 결정: [사람이 친 판은 모두 저장·전송](2026-10-14-adr-save-and-send-every-played-run.md)의 예외 하나를 더한다(바뀌는 점은 아래 1절). 나머지 저장·전송 규칙은 그대로다.
> 관계: 연습의 FREQUENCY 와 시계는 [play-options.md](../play-options.md) 9절, 판의 종류별 기록은 같은 문서 10.2절. 설계 [plan/2026-10-29-feature-upgrade.md](../plan/2026-10-29-feature-upgrade.md), 조사 [reference/2026-10-29-feature-upgrade/play-rate.md](../reference/2026-10-29-feature-upgrade/play-rate.md).

## 배경

- 사용자 요청(2026-10-29): "플레이 부가기능으로 플레이 배속기능 (100퍼가 기본이고 1퍼씩 조절해서 줄이거나 높이거나 하는 기능)도 만들어줘".
- beatoraja 에는 일반 플레이의 배속이 없고 연습에만 FREQUENCY 가 있다. 이 플레이어의 연습 FREQUENCY(50~200%)는 오디오 피치, 오디오 시계와 차트 시계의 변환, 판정 창의 실시간 길이(`PlaySession::set_playback_rate`), 판정 오프셋의 실시간 환산을 이미 갖고 있다. 새로 필요한 것은 연습이 아닐 때도 배율을 갖는 시계, 설정과 UI, 기록 차단이다.
- 배속 판의 점수는 같은 차트를 100% 로 친 점수와 같은 잣대가 아니다.

## 결정

### 1) 사용자가 정한 것

| 번호 | 결정 |
| --- | --- |
| D2 | 배속이 100% 가 아닌 판은 기록을 저장하지 않는다. 로컬 DB·리플레이·trace·서버 전송·기록 동기화 전부다. "사람이 친 판은 전부 저장·전송"(2026-10-14) 정책의 사용자 명시 예외다 |
| D3 | 배속은 재생 속도 자체를 바꿔 피치도 함께 변한다. 범위 50~200%, 1% 단위, 기본 100% |

### 2) 메인 추천(사용자 미확인)

| 번호 | 결정 |
| --- | --- |
| M5 | 배속 값은 설정(`PlayOptions.playback_percent`)에 저장한다. PLAY 메뉴 CHART 묶음과 F1 오버레이에서 바꾼다. 플레이 중 키는 없다 |
| M6 | 코스와 리플레이 감상은 100% 로 강제하고 코스 시작 때 알린다. 연습은 자기 FREQUENCY 를 쓰고 전역 배율을 무시한다 |
| M7 | 배속 판의 판정 창은 실시간 길이다. 녹색 숫자·KEEP GREEN NUMBER 기준 템포·BPM 표시는 실시간 기준(BPM x 배율)이다. AUTO ADJUST 는 그대로 동작한다 |
| M8 | 연습 경로의 기존 결함 후보 둘(판정 그래프 커서의 이중 배율, 리듬 타이머가 배율을 모름)은 같은 경로이므로 함께 고친다. 둘 다 실제 결함이었다 |
| M9 | 배속 판의 결과 화면은 NEW RECORD 를 보이지 않고 "저장 안 됨" 줄과 IR SKIPPED 사유를 보인다 |

### 3) 구현 방식(구현 단위가 정함, 사용자 미확인)

- 시계: 차트를 beatoraja 연습처럼 다시 배치하지 않고 시계를 늘린다. `PlayState.whole_run_clock`(연습이 아닐 때의 `PracticeClock`)이 연습의 구간 시계와 같은 일을 한다. `PlayState::set_playback_rate` 가 시계와 `session.set_playback_rate` 를 함께 걸고, 값은 `PendingChart.playback_percent` 로 실려 모든 진입 경로가 지나는 `into_play` 에서 첫 tick 전에 걸린다. 오디오 피치와 예약 시각, 판정 입력 변환, `field_lead_us` 는 이미 이 시계를 썼으므로 믹서는 바꾸지 않았다. 차트를 재배치하지 않으므로 리플레이·고스트·기록의 차트 시각이 그대로다.
- 값의 출처: `AppShared::run_playback_percent()` 가 리플레이 감상이나 코스 중이면 100, 아니면 설정값을 돌려준다. 연습은 `set_practice` 가 자기 FREQUENCY 로 덮어쓴다.
- 범위 상수 `PLAYBACK_RATE_MIN_PERCENT`(50)·`MAX`(200)·`STEP`(1)·`UNCHANGED`(100)를 `rbms-config` 에 하나만 두고 연습의 `FREQ_MIN/MAX/UNMODIFIED` 가 가리키게 해 중복을 없앴다. `sanitise` 가 50~200 으로 자른다. 이전 설정 파일은 100 으로 읽힌다.
- 행: `SettingId::PlaybackRate`(enum 과 `ALL` 맨 끝, `SETTING_COUNT` 100). 화면에서는 PLAY 메뉴 CHART 묶음의 LEGACY NOTE 바로 아래다(SCRATCH AUTO 아래에 두면 색 행과 AUTO REPLAY 사이가 끊겨 기존 시험이 깨진다). F1 오버레이는 `OPTION_ROWS` 의 마지막 행(12번째)이고 `rbms-render` 의 `OPTIONS_ROW_COUNT` 를 11 에서 12 로 늘렸다.
- 기록 차단: `PlayState::keeps_no_record()` 는 "연습이거나 배속 판"이고 `changed_playback_percent()` 는 연습이 아니면서 100 이 아닐 때의 배율이다. 세 입구(`record_run`, `keep_run_cut_short_by_quitting`, `record_retried_run`)가 모두 이 판정을 쓴다. `ir_submission_block_reason` 과 `updates_score` 에 넷째·다섯째 인자 `changed_rate` 를 더했고 사유는 `"playback rate"`(`practice::PLAYBACK_RATE_BLOCK_REASON`)다. 결과 화면 생략은 연습에만 남겼다.
- 결과 화면: `RunStanding.unsaved_playback_percent` → `ResultRun.updates_score = false`(스킨의 ENABLE_SAVE_SCORE 가 꺼지고 NEW RECORD 가 보이지 않는다), `playback_note`(세 언어)를 어시스트 줄 대신 노란 줄로, IR 줄은 `IR: SKIPPED (playback rate / 재생 속도 변경 / 再生速度変更)`.
- 코스 알림: `start_course` 에서 설정이 100 이 아니면 `NoticeText::CoursePlaysAtTheChartsOwnSpeed`(세 언어)를 띄운다.
- 표시: `PracticeClock` 에 `heard_bpm`·`heard_length` 를 더하고 KEEP GREEN NUMBER 기준 템포(`held_tempo`), 스킨 `Scroll` 의 네 BPM 과 `region_ms`, 내장 HUD 의 녹색·흰 숫자를 실시간 기준으로 바꿨다. 같은 HI-SPEED 에서 200% 면 노트가 실제로 두 배 빨리 내려오고, KEEP GREEN NUMBER 가 켜져 있으면 하이스피드가 절반이 되어 녹색 숫자가 유지된다. 이 계산은 연습 FREQUENCY 에도 같은 시계로 적용된다(연습의 동작 변화).
- 스킨 id: 불린 21106 `RBMS_OPTION_PLAYBACK_RATE_CHANGED`, 정수 21084 `RBMS_NUMBER_PLAYBACK_PERCENT`. 각 공간의 첫 빈 번호다([확장 id 공간 ADR](2026-10-24-adr-rbms-extension-id-spaces.md)의 배정표에 더했다). 결과 → 플레이 → 옵션 순으로 답한다: 결과 화면은 끝난 판의 배율, 플레이 화면은 진행 중인 판의 시계 배율(연습 FREQUENCY 포함), 선곡과 그 밖의 화면은 설정값이다. 스킨 미리보기는 꺼짐이다.
- 기본 스킨: 플레이 화면 `play/info.lua` 의 오른쪽 스테이지 칸에 노란 `RATE n%`(`play_playback_rate`), 결과 화면 `result/main.lua` 의 verdict 에서 배속 판이면 `SCORE NOT SAVED` 대신 노란 `NOT SAVED (RATE n%)`(`result_not_saved_at_rate`). 선곡 화면은 `select/info.lua` 의 CHART DATA 머리 줄(제목 오른쪽, 난이도 탭 왼쪽)에 같은 노란 `RATE n%` 를 그린다(차트 막대 위에서 21106 이 켜졌을 때. 선곡의 옵션 클러스터가 설정값을 답한다). 단위 U2 는 `lua/props.lua` 가 U3 소유라 플레이·결과 두 lua 파일에 id 숫자를 직접 적었고, 작업 트리에서는 그 숫자가 `props.extension.PLAYBACK_RATE_CHANGED`·`props.extension_number.PLAYBACK_PERCENT` 이름으로 옮겨져 있으며 선곡 표지도 붙어 있다.

### 4) M8 의 두 결함(기존 연습 경로)

| 결함 | 원인 | 고침 |
| --- | --- | --- |
| 판정 그래프 커서의 이중 배율 | PLAY 타이머는 `now - chart_us` 로 이미 차트 시각이고 시작·끝 표지도 차트 ms 인데 배율을 다시 곱해 커서가 배율만큼 어긋났다 | `PlayCursor.speed` 필드를 지우고 위치를 그대로 쓴다 |
| 리듬 타이머가 배율을 모름 | `slowed = 100 - bpm*play_speed/60` 에서 bpm 은 차트 BPM 이고 delta 는 실시간이라 배율이 있으면 박이 어긋났다 | `play_speed_percent()` 가 분석 모드가 아니면 시계 배율을 돌려줘 원본의 `bpm * freq` 와 같게 한다 |

## 이유

- 점수는 같은 곡을 같은 속도로 쳐야 비교된다. 배속 판을 저장하면 최고 기록·순위·IR 이 같은 잣대가 아니게 된다. 사용자가 저장하지 않기로 정했다(D2).
- 시계를 늘리는 방식이 연습이 이미 쓰는 방식이고 믹서·디코더·판정 엔진을 바꾸지 않는다. 판정 창을 실시간 길이로 두면 연습·원본(beatoraja 연습)과 같고, 200% 에서 창이 절반이 되어 같은 실력에도 어려워지는 일이 없다.
- 코스는 스테이지마다 기록을 남기고 리플레이는 100% 로 기록된 판이라, 다른 배율로 재생하면 판정이 달라진다. 그래서 100% 로 강제한다.

## 기각된 대안

| 대안 | 기각 이유 |
| --- | --- |
| 차트를 beatoraja 처럼 다시 배치(시각 / 배율, BPM x 배율) | 리플레이·trace·고스트·기록의 차트 시각이 모두 바뀐 차트 기준이 되고 연습이 이미 택한 방식과 갈린다 |
| 판정 창을 차트 시각 기준으로 | 200% 에서 PGREAT 창이 실제 절반이 된다. 원본·연습과 다르고 엔진에 새 모드가 필요하다 |
| 코스에서도 배속 적용 | 코스는 스테이지마다 저장하는 판이다. 일부만 배속이면 코스 기록의 의미가 흐려진다 |
| 플레이 중 배속 키 | 사용자 요청이 설정 조절이다. 키 설정 이름과 충돌 검사를 더하지 않았다 |
| 배속 판을 저장하되 표지만 붙임 | 서버·로컬 스키마 변경이 필요하고 D2 와 다르다 |
| `PracticeClock` 이름 바꾸기 | diff 가 커진다(최소 변경). 일반 판에도 쓰이지만 이름은 그대로 두었다 |

## 영향과 관련 파일

- 코드: `crates/rbms-config/src/{schema,settings,lib}.rs`, `contract/client-requests/settings-blob.json`(play 블록에 `playback_percent: 100` 한 줄, 계약 골든 갱신), `crates/rbms-render/src/{theme.rs, skin_render/graphs/notes_dist.rs}`, `apps/rbms-player/src/{practice,app_play,app_result,app_options,ir_outcome,lib,settings_ui}.rs`, `stage/play/{mod,skin_preview}.rs`, `stage/play_menu/rows.rs`, `stage/result.rs`, `skin_host/{extension,mod,options,play,play_timers,result}.rs`, `text/{notices,settings/*,menus/ir_outcome}.rs`, 기본 스킨 `lua/{play/info,result/main,i18n/play,i18n/result}.lua`.
- 시험: `app_result/record_tests/playback_rate.rs`(결과 화면, 도중 포기, 실패 뒤 다시 하기, 코스·리플레이 100%, 연습), `stage/play/timing_tests.rs`(200% 에서 PGREAT 창 실시간 ±20 ms), `hold_tests.rs`(녹색 숫자), `stage/play_menu`·`app_options` 행 시험, `stage/result/tests.rs`, `skin_host/{options,result}` 시험, `rbms-config` 의 `the_playback_rate_reads_from_older_files_and_stays_in_its_range`.
- 서버·웹: 형식 변경 없음. 계약 골든의 설정 묶음에 필드 하나가 늘었다.
- 알려진 위험(테스트 부채 3.133~3.135, 실기 QA 3.25 V6~V9):
  - 배속 소리의 피치와 음질을 실창에서 듣지 않았다(보폭이 1 을 넘으면 선형 보간 에일리어싱. 연습 200% 와 같은 수준).
  - 연습 FREQUENCY 판의 BPM·녹색 숫자·KEEP GREEN NUMBER 기준 템포가 실시간 기준으로 바뀌었고 리듬 타이머와 판정 그래프 커서의 동작이 바뀌었다. 사용자 확인이 필요하다.
  - 설정 동기화(SYNC SETTINGS)로 다른 기기의 배율이 내려올 수 있다. 선곡 표지, F1 오버레이, PLAY 메뉴, 플레이 화면 표지, 결과 화면으로 배율을 알 수 있다.
  - 배속 판의 R 다시 하기는 `updates_score` 가 false 라 배치 시드를 잇지 않는다(넓힌 판정과 같은 규칙). 파란 건반 + START 는 판의 종류를 가리지 않으므로 같은 배치로 다시 한다.
  - 남은 시간(163·164)은 차트 시간 그대로 표시한다. 곡 BGA 이미지는 차트 시각, 스킨 동영상과 장면 타이머는 실시간이다.
  - 녹색 숫자의 실시간 기준은 beatoraja `changeFrequency` 가 BPM 을 곱한다는 전제로 맞췄고 이 전제는 원본 소스로 확인하지 못했다.
- 문서: [play-options.md](../play-options.md) 2절·7.2절·9.3절·10.2절·18절, [skin.md](../skin.md) 1.4절.

## 날짜와 사용자 원문

2026-10-29.

- 사용자 요청: "5. 플레이 부가기능으로 플레이 배속기능 (100퍼가 기본이고 1퍼씩 조절해서 줄이거나 높이거나 하는 기능)도 만들어줘"
- 사용자 결정: 배속(100% 아님) 판은 기록을 저장하지 않음(사용자 명시 제외). 배속은 재생 속도 자체를 바꿔 피치도 함께 변함, 50~200%, 1% 단위.
