# 리플레이의 배치로 직접 플레이하기(LAYOUT PLAY) 조사 보고

> 작성 2026-10-24 · 대응 커밋 `9fed3c0`(작업 트리에 이 문서만 추가) · 상태: 조사만 했고 코드는 바꾸지 않았다.
> 범위: 요구 2 "리플레이를 볼 때 해당 리플레이의 배치를 활용해 나도 플레이하는 모드(battle 모드와 비슷한 것)".
> 읽은 곳: R-BMS 저장소 전체(아래 절마다 파일:줄), beatoraja 소스(`/Users/hyunseokbyun/development/beatoraja/src`, 읽기만), 외부 문서(URL 표기, 못 읽은 것은 "확인 못 함").
> 관련 기존 문서: [record-target-pace.md](record-target-pace.md)(리플레이를 다시 판정해 고스트로 쓰는 기능), [../play-options.md](../play-options.md) 10.7절(같은 배치로 다시 하기), [../acknowledge/2026-10-14-adr-same-layout-retry.md](../acknowledge/2026-10-14-adr-same-layout-retry.md).

## 0. 결론 먼저

1. 사용자가 말한 "battle 모드와 비슷한 것"에 가장 가까운 것은 beatoraja 의 BATTLE 이 아니라 LR2 의 G-BATTLE(ghost battle)과, beatoraja 가 리플레이를 시작할 때 특정 건반을 누르고 있으면 그 리플레이의 배치·옵션으로 직접 치게 하는 "리플레이 재현 모드 : 譜面"(`BMSPlayer.java:140-149`)이다. beatoraja 의 BATTLE 은 SP 채보를 DP 양쪽에 복사해 두 사람이 치는 모드라 리플레이와 무관하다(3.5절).
2. R-BMS 는 이미 배치를 재현하는 재료를 전부 갖고 있다. 리플레이 파일과 서버 리플레이 모두 `random`(옵션)과 `seed`(배치 시드)를 싣고, 같은 값이면 같은 배치가 나온다(`app_play.rs:142-156`, `shuffle.rs:115-159`). 부족한 것은 "그 값을 빌려 새 판을 시작하는 길"뿐이다. 서버·DB·와이어·설정 변경 없이 클라이언트만으로 1단계를 만들 수 있다.
3. 추천: 1단계는 "배치 빌리기"(옵션+시드만 복사, 게이지·SCRATCH AUTO·판정은 플레이어 자기 설정), 진입은 기록 모달·IR RANKING 패널·리플레이 보는 중 단축키. 2단계(선택)는 이 판을 그 리플레이의 노트별 점수와 겨루는 고스트 페이스(기존 `ghost.rs` 재사용, 로컬 리플레이만). 서버 리플레이 고스트는 와이어·DB 를 늘려야 해서 3단계로 미룬다.
4. 기록 의미는 바뀌지 않는다: 빌린 배치로 친 판은 원본과 같이 일반 PLAY 로 저장·전송되고, 옵션과 시드가 그대로 적힌다. 판정·점수 의미를 바꾸는 항목은 없어서 그 종류의 사용자 확인은 필요 없다. 정할 것은 5절의 7개다.

## 1. 요구 해석

원문: "리플레이를 볼때 해당 리플레이의 배치를 활용하여 나도 플레이하는 모드(battle모드였나? 그거랑비슷한거야)".

| 해석 | 내용 | 판단 |
| --- | --- | --- |
| A. 배치 빌리기 | 리플레이의 RANDOM 계열 옵션과 시드를 그대로 써서 내가 친다 | 핵심. beatoraja "譜面" 재현, LR2 G-BATTLE 의 배치 부분 |
| B. 고스트 대결 | A 에 더해 그 리플레이의 점수를 노트별 고스트로 보며 친다 | LR2 G-BATTLE 의 "ghost". 이미 있는 기록 목표 고스트(record-target-pace.md)를 재사용할 수 있다 |
| C. 문자 그대로 동시 | 리플레이가 도는 화면과 내가 치는 화면을 한 판에 나란히 | 본 질문의 "battle 모드와 비슷"일 수 있으나 R-BMS 에는 DP BATTLE 도 없다(`reference-divergences.md:177`: LANE OPTION 의 FLIP/BATTLE 은 런에 반영되지 않아 설정 화면에서도 숨김). 크다 |

추천은 A 를 1단계, B 를 2단계로 두는 것이다. C 는 대안으로만 적는다(4.4절). "배치"는 레인 배치(RANDOM, S-RANDOM 등 옵션이 만든 노트 위치)이고 BMS 의 `#RANDOM` 분기 선택과 구별한다. R-BMS 는 `#RANDOM` 분기를 고정 시드 0 으로 푼다(`app_play.rs:207` 의 `parse_with(bytes, Default::default())`, `rbms-parser/src/lib.rs:127-129`)라서 분기는 매번 같고 리플레이에 따로 적을 필요가 없다(beatoraja 는 `ReplayData.rand` 로 적는다, 3.1절).

## 2. 현재 R-BMS 상태

### 2.1 리플레이에 무엇이 저장되는가

`crates/rbms-store/src/replay.rs:152-179`(`Replay`), `:91-128`(`ReplayJudge`):

| 필드 | 뜻 | 배치 재현에 필요한가 |
| --- | --- | --- |
| `md5` | 차트 파일 해시 | 필요(같은 차트 확인) |
| `mode` | 모드 이름 | 확인용 |
| `random` | 레인 옵션 라벨 문자열(`OFF`, `MIRROR`, `RANDOM`, `S-RANDOM`, `R-RANDOM`, `ROTATE`, `H-RANDOM`, `ALL-SCRATCH`, `NoteOption::from_str`) | 필요 |
| `seed` | 배치 시드(u64, 새 시드는 2^53 미만이고 0 이 아니다, `app_play.rs:16-27`) | 필요 |
| `scratch_auto`, `gauge` | 그 판의 SCRATCH AUTO 와 게이지 | 아니오(보는 동안만 쓴다) |
| `judge` | 판정 폭 6칸, LN MARGIN, `ln_mode`, 게이지 표, `clock`, TOTAL 직접값 등 | 아니오. 단 `ln_mode` 는 4.3절 |
| `events` | 입력 로그(`t`, `lane`, `press`, `backward`) | 아니오 |
| `chart_path` | 기록한 기기의 차트 경로 | 아니오(옛 경로일 수 있다, 6절) |

DP 는 옵션 하나(`random`)를 양쪽에 적용하고 시드에서 쪽마다 값을 만든다(`shuffle.rs:115-126` 의 `seed.wrapping_add(side)`). 2P 옵션을 따로 두는 필드는 없다(`random_p2: None`, `app_result.rs:339`).

저장 조건: 손으로 친 모든 판, AUTO REPLAY 가 켜진 때(기본 켬, `schema.rs:627`), 입력이 하나라도 있을 때(`app_result.rs:288-290`). 파일명은 `replays/<md5 앞 8자>-<ms>.ron`(`docs/play-options.md` 10.1절). 서버 업로드는 AUTO UPLOAD REPLAY(기본 켬, `schema.rs:858`)이고 여섯 판정 폭이 한 값이며 LN MARGIN 이 기본이고 TOTAL 직접값이 없을 때만 올린다(`ir_replay.rs:98-103`).

### 2.2 어디서 고르는가

| 길 | 코드 | 비고 |
| --- | --- | --- |
| 선곡의 리플레이 슬롯 | `stage/select/events.rs:295-296`(`replay_in_slot`: 그 차트 기록 중 `replay_file` 이 있는 것의 N번째, 최신순 4개), `:487-498`(`play_replay_slot`), `skinned.rs:326-330`(REPLAY 키), `lent.rs:92-101`(NEXT_REPLAY) | 슬롯은 LN 종류·어시스트를 가리지 않는다 |
| 기록 모달의 PLAY REPLAY | `stage/select/mod.rs:732-762`(`play_record_replay`, `play_replay_file`), `:974-983`(모달 키), `crates/rbms-render/src/select.rs:690-696`(버튼과 `SelectHot::ModalReplay`) | 이 기기의 기록만 |
| IR RANKING 패널의 행 | `mod.rs:854-874`(`play_ranking_replay`), `:900-937`(`poll_replay_download`), `ir_ranking_view.rs:30-39`(`PanelAction::PlayReplay`, ENTER) | 행당 가장 새 리플레이 하나(`ir_ranking.rs:178-190`, 테스트 `:461`). 리플레이가 없는 행은 `no replay for that row` |
| 명령줄 | `command_line.rs:51,146`(`--replay FILE.ron`) | 선곡을 건너뛴다 |

### 2.3 서버의 리플레이

- 다운로드는 인증이 필요 없다: `web/src/server/route/replay.ts:63-67`(`GET /api/replays/{id}`)가 `readReplay` 를 그대로 돌려준다. 목록은 `GET /api/charts/{hash}/replays`(`:51-61`, 메타만).
- 돌려주는 본문(`replay.service.ts:83-105` `toReplayData`): `chart{md5,sha256}`, `mode`, `random`, `random_p2`, `seed`, `lntype`, `offset_ms`, `judge_rate`(한 값), `scratch_auto`, `gauge`, `events`. 목록 메타(`:65-81`)에는 `seed` 와 `lntype` 은 있고 `random` 은 없다. 업로드 때 `random`, `seed` 를 저장한다(`:141-143`).
- 서버가 `seed` 를 검증하지 않는다: 안전한 정수 밖의 시드는 0 으로 읽고(`web/src/server/dto/common.ts:142`), 순위 정책(`ranked-policy.ts:60-72`)은 `random`·`seed` 를 보지 않는다. 재시뮬 검증(FR-12)은 후속으로만 적혀 있다(`docs/backend/api-spec.md:426`).
- 랭킹 행(`ScoreRecord`)에는 시드·옵션 문자열이 없고 비트마스크 `option` 만 있다(`crates/rbms-ir/src/dto/score.rs:190-192`, `api-spec.md:182`). 시드는 `GET /api/scores/{id}` 상세에서만 본다(클라이언트에는 그 호출이 없다).
- 클라이언트의 변환 손실(`apps/rbms-player/src/ir_replay.rs:110-127`): `from_ir_replay` 는 `random`, `seed`, `gauge`, `scratch_auto`, `judge_rate`, 입력만 복원하고, 나머지 판정 설정은 `ReplayJudge::default()`(알고리즘 Duration, LN, 시계 FRAME)로 채운다. 스크래치 방향(`backward`)도 없다. `note_option_from_ir` 는 `Converge` 를 조용히 OFF 로 바꾼다(`:131-136`). 배치 복사에는 `random`·`seed`만 쓰므로 판정 손실은 영향이 없지만 Converge 는 거절해야 한다(4.3절).
- 같은 이유로 서버 리플레이는 지금 고스트로 다시 판정할 수 없다(`ghost.rs:264-266` 이 FRAME 시계를 거절, 테스트 부채 3.79 `docs/quality-assurance/2026-10-14-test-debt.md:230`).

### 2.4 재생 경로와 이미 있는 "같은 배치" 재료

- 재생 시작: `play_replay_file`(`mod.rs:744-762`)이 `shared.chart_path = rp.chart_path` 와 `shared.replay = Some(rp)` 를 두고 `shared.load()` 를 부른다. `load()`(`app_play.rs:447-606`)는 `play_as_recorded()`(`:66-74`)로 설정의 SCRATCH AUTO·게이지·RANDOM 을 리플레이 값으로 덮고(`OwnPlayOptions` 에 플레이어 자기 값을 보관, 끝나면 `end_replay()` `:85-88` 이 되돌린다) `(옵션, 시드)`를 받아 `lay_out_notes(model, random, seed, ...)`(`:142-156`)로 배치한다. 차트 md5 가 다르면 경고만 하고 계속한다(`:464-469`).
- 기본 설정(`replay_analysis` = 켬, `schema.rs:774`)이면 리플레이는 스스로 끝나지 않고 플레이어가 Esc 로 나간다(`stage/play/mod.rs:633-639`, `:1361-1368`). 그래서 "보다가" 넘어가는 길은 결과 화면이 아니라 재생 중 단축키여야 한다. 재생 중 키는 `analysis_key`(`mod.rs:754-782`)와 `Shortcut::ReplayPause` 등 다섯 개(`keyconfig.rs:750-754`, 기본 Space, =, -, PageUp, PageDown `:934-938`)다.
- 같은 배치로 다시 하기(이미 있음): 결과 화면에서 파란 건반+START(`Leave::RunSameLayout`, `stage/result.rs:215-225`, `:1019-1031`), R/7번 키(`RunAgain`), 실패 직후 SELECT(`stage/play/mod.rs:1449-1456`)가 시드를 `AppShared.retry_seed`(`lib.rs:802`)에 두고 다음 `load()` 가 한 번 쓴다(`app_play.rs:448`, `:480`). 옵션은 "설정에 있는 그대로"라서 시드만 넘긴다. 리플레이를 본 뒤에는 `offers_retry` 가 거짓이다(`app_result.rs:897-899`).
- 판의 기록: 저장되는 `random` 은 그 시점의 `config.play.random`(`app_result.rs:338`, `scoredb_store.rs:225`), 시드는 `play.seed()`(`app_result.rs:279`)다. 리플레이 재생은 기록을 남기지 않는다(`app_result.rs:288-290`, play-options.md 10.2절).

### 2.5 기록 목표·고스트(연결점)

`ghost.rs:237-278` 의 `session_of_replay` + `rejudge` 가 저장된 리플레이를 소리·화면 없이 다시 판정해 노트별 EX 누적(`GhostPace`)을 만든다. 플레이 화면은 목표가 LOCAL BEST·IR BEST 일 때 그 페이스를 쓰고(`stage/play/mod.rs:488-509` `attach_ghosts`), 노트 수·최종 EX 가 맞아야 붙는다. 대상은 이 기기의 최고 기록과 IR BEST 로 한정되고(`ghost.rs:139-143`), 서버 리플레이는 위 2.3절 이유로 비례 페이스다. 지금 `ScoreTarget::Rival` 은 점수만 알고 노트별 페이스는 없다(`target.rs:46-50`, `:158-170`).

### 2.6 관련 코드 지도

| 영역 | 파일 |
| --- | --- |
| 리플레이 형식 | `crates/rbms-store/src/replay.rs` |
| 배치 계산 | `crates/rbms-chart/src/shuffle.rs`(`lane_permutation` `:115`, `apply` `:148`) |
| 로드와 배치 적용, 보관 옵션 | `apps/rbms-player/src/app_play.rs`(`play_as_recorded`, `OwnPlayOptions`, `lay_out_notes`, `load`) |
| 공유 상태 | `apps/rbms-player/src/lib.rs`(`retry_seed` `:802`, `own_play_options` `:809`, `replay_download_*` `:884-886`) |
| 선곡 진입 | `stage/select/{mod,events,skinned,lent}.rs`, `ir_ranking_view.rs`, `crates/rbms-render/src/select.rs`, `stage/select/overlay.rs` |
| 재생 중 | `stage/play/mod.rs`, `keyconfig.rs` |
| 결과와 다시 하기 | `stage/result.rs`, `app_result.rs` |
| 고스트 | `ghost.rs`, `target.rs`, `skin_host/score/standing.rs` |
| 서버 | `web/src/server/{route/replay.ts,service/domain/replay/replay.service.ts,dto/replay.ts,dto/common.ts}` |

## 3. 원본(beatoraja)·LR2·IIDX 기준

### 3.1 beatoraja 가 리플레이에 싣는 것

`ReplayData.java:35-86`: `sha256`, `mode`(LN 모드), `keylog`, `gauge`, `pattern`(옛 호환 `PatternModifyLog[]`), `laneShufflePattern`, `rand`(#RANDOM 분기), `randomoption`/`randomoptionseed`(1P), `randomoption2`/`randomoption2seed`(2P), `doubleoption`(DP 옵션), `sevenToNinePattern`, `config`(하이스피드 등 `PlayConfig`). 저장 때 채우는 곳은 `BMSPlayer.java:890-906`. 판 점수에도 `option = random + (DP면 random2*10 + double*100)`, `seed = (DP면 seed2*65536*256) + seed1` 가 적힌다(`BMSPlayer.java:885-887`). 리플레이 파일은 sha256 + (정의 안 된 LN 이 있으면 LN 모드) + 슬롯 번호로 이름이 정해져(`PlayDataAccessor.java:663-666`) 다른 차트의 리플레이는 애초에 읽히지 않는다.

### 3.2 리플레이를 시작할 때 키를 누르고 있으면 직접 치는 기능

`BMSPlayer` 생성자(`BMSPlayer.java:110-175`, 리플레이 모드 1곡):

| 누르고 있는 건반(`getKeyState(i)`, 7키 키보드 기준 Z S X D C F V = 0~6) | 동작 | 줄 |
| --- | --- | --- |
| 인덱스 1(둘째 건반) | "리플레이 재현 모드 : 譜面". `randomoption`, `randomoptionseed`, `randomoption2`, `randomoption2seed`, `doubleoption`, `rand` 를 리플레이에서 복사 | `:140-149` |
| 인덱스 2(셋째 건반) | "リプレイ再現モード : オプション". 옵션만 복사(시드는 새로) | `:150-157` |
| 인덱스 4(다섯째 건반) | "하이스피드". 저장된 `PlayConfig` 를 현재 모드 설정에 덮어씀 | `:158-163`, `:346-349` |
| 위 셋 중 하나라도 | 리플레이 데이터를 버리고 모드를 `PLAY` 로 바꿔 직접 친다 | `:164-168` |
| 인덱스 3·5 | 보는 리플레이의 게이지를 +2/+1 단계 올림(직접 치는 모드와 무관한 보기용) | `:352-358` |

키를 읽는 시점은 `BMSPlayer` 가 만들어질 때다. `MainController.changeState(PLAY)` 가 DECIDE 뒤에 생성자를 부르므로(`MainController.java:257-262`) 선곡에서 REPLAY 키(7키는 V, `MusicSelectKeyProperty.java:15`)를 누르고 결정 화면이 끝날 때까지 쥐고 있어야 한다. 이는 코드로 읽은 것이고 실제 기기로는 확인하지 못했다.

복사한 판은 일반 PLAY 다: `resource.setPlayMode(PLAY)`(`:166`), 점수 저장은 어시스트 옵션이 아닌 한 그대로(`:363`), 옵션·시드는 복사값으로 점수에 적힌다(`:885-887`). "복사했다"는 표시는 없다.

### 3.3 ChartReplicationMode(RIVALCHART)

`PlayerConfig.java:67` 기본값이 `"RIVALCHART"` 이고, `MusicSelector.readChart`(`MusicSelector.java:377-423`)가 라이벌의 점수에서 `option`, `seed` 를 풀어(`rival.getOption() % 10` 등) 일반 PLAY 의 배치로 쓴다. 선택 가능한 값은 `NONE, RIVALCHART, RIVALOPTION` 뿐이다(`:699`; `REPLAYCHART`/`REPLAYOPTION` 은 열거형에만 있다). R-BMS 는 이 모드(이벤트 344)를 만들지 않았다(`stage/select/events.rs:24-26`). 즉 원본은 리플레이가 없어도 라이벌 점수의 옵션·시드만으로 배치를 빌릴 수 있다. R-BMS 서버는 `score.seed`, `score.random` 을 저장하지만(`web/drizzle/0000_smooth_cerebro.sql:170-215`) 랭킹 행으로 내보내지 않는다(2.3절).

### 3.4 결과 화면의 REPLAY_SAME / REPLAY_DIFFERENT

7키에서 5번째 건반 REPLAY_DIFFERENT, 7번째 REPLAY_SAME(`ResultKeyProperty.java:10`). PLAY 모드에서만 동작하고(`MusicResult.java:226-241`), DIFFERENT 는 `randomoptionseed = -1` 로 옵션은 두고 시드만 새로, SAME 은 같은 시드로 다시 하되 어시스트 판은 막는다(`:234-241`). R-BMS 의 결과 화면은 이미 이를 따른다(`stage/result.rs:511-512`, play-options.md 10.7절). R-BMS 는 어시스트 판도 막지 않는다(ADR 2026-10-14 결정 1).

### 3.5 beatoraja 의 BATTLE 과 FLIP

`Random.BATTLE`/`FLIP`(`pattern/Random.java:28-29`, `OPTION_SINGLE = {IDENTITY, BATTLE}`, `OPTION_DOUBLE = {IDENTITY, FLIP}`). BATTLE 은 SP 모드(5K, 7K, 24K)를 DP 모드로 올리고 채보를 양쪽에 복사한다(`BMSPlayer.java:238-258`, `LaneShuffleModifier.java:186-204` `PlayerBattleModifier`). 한 대에서 두 사람이 같은 곡을 치는 모드이고, 어시스트 수준 ASSIST 라 점수를 저장하지 않는다(`:251-252`). FLIP 은 DP 양쪽을 바꿔 보여 주는 옵션(`:175-183`, 어시스트 없음). 둘 다 리플레이와 무관하다.

### 3.6 LR2 G-BATTLE (ghost battle)

- 외부 글(`https://toiblog.com/bms/19419/`)에 따르면: OPTION 의 BATTLE 항목을 "G-BATTLE" 로 바꾸고 선곡에서 4번 건반을 길게 누르면 랭킹 상위 999명이 표시되고, 고른 플레이어가 친 채보와 같은 채보를 칠 수 있으며 "(オプションやゲージも同一となる模様)" 라고 한다. 글쓴이는 "らしい"로 전언임을 밝힌다. 라이벌 폴더에서 같은 방식으로 라이벌의 자기 베스트 채보를 칠 수 있다고 한다.
- 검색 도구가 낸 ECHOES wiki 요약(`https://w.atwiki.jp/echoes140/pages/220.html`, 이 서버가 WebFetch 를 403 으로 막아 본문을 직접 읽지 못했다): IR 에 접속해 점수를 보낸 채보에서 랭킹을 열고 특정 플레이어를 고르면 그 사람이 자기 베스트를 냈을 때의 채보와 게이지로 칠 수 있고, 점수·클리어 상태는 자기 것으로 저장된다. 같은 요약은 S-RANDOM·H-RANDOM 은 점수 저장이 되지 않는 것으로 적혀 있다고 한다. 본문 대조는 확인 못 함.
- LR2 계열 구현의 코드 주석(`https://github.com/rian-eimu/lr2oraja-endlessdream/pull/33`)은 노트 수가 바뀌면 ghost 가 무효화된다고 적는다: "ノーツ数が変わるとゴーストが無効化され (ScoreDataProperty の長さ比較)". beatoraja 도 같은 검사를 한다(`ScoreDataProperty.java:171-172`, record-target-pace.md 2절).
- LR2 의 "리트라이"(플레이 직후 같은 랜덤 배치로 다시)와 OpenLR2 의 START+SELECT 조합은 검색 요약으로만 확인했다. R-BMS 가 이미 가진 "같은 배치로 다시 하기"에 해당한다.

결론: 요구 문장 "리플레이 배치로 나도 플레이 + battle" 의 가장 가까운 원형은 LR2 G-BATTLE 이다(상대의 채보 조건을 빌려 친다 + ghost). beatoraja 는 같은 효과를 리플레이 시작 때의 건반 조합으로 주고 ghost 는 별개로 가진다.

### 3.7 IIDX

- RemyWiki(`https://remywiki.com/What_is_beatmania_IIDX`): BATTLE 은 "If on Double Play, this uses the Single Play notecharts instead on both sides.", Premium Area 의 "Music retry" 는 "Allows one replay of a song with the exact same settings and RANDOM pattern if a STYLE modifier is used."(일본 기기 한정), Pacemaker/Ghost 표시는 "a display of the difference between the current score and a previous high score."
- 랜덤 배치를 고정하는 공식 수단은 유료 "ランダムレーンチケット"다(`https://github.com/monadnadnad/iidx-rlt` 설명: "チケットを消費することで、記載の配置に固定できる課金アイテム"). 이 글은 아케이드와 INFINITAS 중 어느 것인지 적지 않는다.
- IIDX 에 남의 리플레이 배치를 복사해 치는 무료 기능이 있는지는 위 자료로 확인하지 못했다(없다고 단정하지 않는다).

### 3.8 비교

| 항목 | beatoraja | LR2 | IIDX | R-BMS 지금 |
| --- | --- | --- | --- | --- |
| 내 직전 판의 배치로 재도전 | 결과 화면 키(REPLAY_SAME) | 리트라이 | Music retry, 한정 | 있음(파란 건반+START, R) |
| 남/옛 리플레이의 배치로 치기 | 리플레이 시작 때 건반 쥐기 | G-BATTLE(채보·옵션·게이지) | 확인 못 함 | 없음 |
| 라이벌 점수의 배치로 치기 | ChartReplicationMode | 라이벌 폴더+G-BATTLE | 확인 못 함 | 없음 |
| ghost | 기록 판 노트별 고스트 | ghost battle | Ghost 표시 | 있음(내 기록만) |
| 두 사람 동시(BATTLE) | SP→DP 복사 | BATTLE | DP BATTLE | 없음(숨김) |

## 4. 설계안

### 4.1 추천안: LAYOUT PLAY 를 두 단계로

**1단계, 배치 빌리기(필수)**

- 개념: `AppShared` 에 `borrowed_layout: Option<BorrowedLayout>`(`random: NoteOption`, `seed: u64`) 한 칸. 다음 `load()` 에서 `play_as_recorded()` 분기와 `retry_seed` 보다 먼저 읽어 `(random, seed)` 로 쓴다(`app_play.rs:470-480`). 실행 동안 `config.play.random` 만 빌린 값으로 바꾸고 플레이어 자기 값을 `own_play_options` 에 보관한다(게이지·SCRATCH AUTO 는 자기 값 그대로). 이미 있는 보관·복원(`app_play.rs:29-57`, `settings_to_save` `:100-112`)을 재사용하므로 판 도중 설정이 저장돼도 플레이어의 RANDOM 이 파일에 남는다.
- 시작 함수: `AppShared::borrow_layout_of(&Replay, 라이브러리 인덱스) -> Result<(), 거절 이유>`. 검사(4.3절): 차트 md5 일치, `seed != 0`(옵션이 OFF 가 아닐 때), 옵션이 R-BMS 가 가진 8개 중 하나, 코스·연습·오토플레이 중이 아님. 통과하면 `shared.end_replay()` 후 라이브러리 인덱스로 `start_song` 과 같은 길(`app_result.rs:868` `start_song`, `:877` `retry`, 인덱스는 `:737` `played_song_index` 가 `shared.chart_path` 로 찾는다)로 LOADING 에 들어간다. 새 판의 차트 경로는 리플레이의 `chart_path` 가 아니라 선곡에서 포커스된 라이브러리 항목의 것을 쓴다(6절).
- 진입 길(추천 순):
  1. 기록 모달(`mod.rs:974-983`, `select.rs:690-696`): 버튼 "PLAY THIS LAYOUT"(`SelectHot::ModalLayout` 추가)과 키 하나.
  2. IR RANKING 패널(`ir_ranking_view.rs:14-39`): `PanelAction::PlayLayout` 추가. 내려받기(`poll_replay_download` `mod.rs:900-937`)가 끝나면 `Replay` 를 재생하는 대신 `borrow_layout_of` 를 부르게 `replay_download_target` 에 "배치만" 표시를 더한다. 힌트 줄(`:207-208`)에 키를 적는다.
  3. 리플레이를 보는 중: 새 단축키 `Shortcut::ReplayPlayLayout`(`keyconfig.rs:745-795` 의 `ALL` 이 31 → 32, 기본 키는 구현 때 `Control` 키와 겹치지 않는 것으로 정한다). `analysis_shortcut`(`stage/play/mod.rs:763-782`)이 처리한다. 기존 키 파일에는 항목이 없어도 안전하다(`keyconfig.rs:1369-1375`: 없으면 출하 키, 이미 쓰인 키면 비워 둠).
  4. (선택) beatoraja 식: REPLAY 키(슬롯 재생)를 파란 건반 하나를 쥔 채 누르면 슬롯의 리플레이로 `borrow_layout_of`(`skinned.rs:326-330` 에서 `shared.key_index_pressed(1)`, 결과 화면이 쓰는 파란 건반 규칙과 같은 인덱스 `stage/result.rs:161` `blue_keys`). 숨은 조작이라 모달 버튼이 주 경로다.
- 재시도 규칙: 빌린 배치는 같은 배치 재시도(파란 건반+START, R·7번 키, 실패 직후 SELECT)에서는 유지하고, 선곡으로 나가거나 다른 곡(N)으로 가거나 "다른 배치로"(5번 키, START 로 다시)를 고르면 버리고 플레이어 자기 옵션으로 돌아간다. 이유: `load()` 는 리플레이가 없으면 `put_back_own_play_options()` 를 먼저 부른다(`app_play.rs:473`)라서 빌린 옵션이 재시도 직전에 사라지지 않게 따로 붙들어야 한다. 손대는 곳: `stage/result.rs:1019-1031`, `stage/play/mod.rs:1449-1456`, `app_play.rs:824-833`(`leave_play`).
- 기록: 일반 PLAY 로 저장·전송(규칙대로 노트를 하나라도 친 판은 전부). 로컬 `scorelog.random`/`seed`, 제출 본문 `options.random`/`seed`, AUTO REPLAY 리플레이의 `random`/`seed` 는 모두 빌린 값이다. 표지·열을 추가하지 않는다. 서버는 `random`·`seed` 를 보지 않으므로 순위는 달라지지 않는다(2.3절).
- 화면 알림(작게): 로딩 화면의 단계 글자 또는 콘솔에 `layout of <리플레이 출처> (<옵션>)` 한 줄. 새 스킨 id 는 만들지 않는다.

**2단계, 고스트 대결(선택)**

- 빌린 배치로 친 판을 그 리플레이의 노트별 EX 와 견준다. 기존 `GhostPace` 를 그대로 쓰고(`ghost.rs`), 로컬 리플레이만 대상으로 한다. 이유: 서버 리플레이는 재판정에 필요한 판정 설정이 와이어에 없다(2.3절).
- 구현 방향: `GhostSources` 에 "이번 판이 빌린 리플레이" 칸을 더하고(`ghost.rs:120-130`, `:139-143`), `attach_ghosts`(`stage/play/mod.rs:488-509`)가 이 판의 TARGET 을 그 칸으로 바꿔 쓰며(목표 이름 `REPLAY <표시 이름>`), 결과 화면 TARGET 도 같은 값을 읽는다(`app_result.rs:695` `run_target`). 설정의 TARGET 행은 건드리지 않고 한 판 한정이다(ADR 2026-10-14 "한 판의 목표는 시작할 때 한 번"과 맞다).
- 검사는 기존 `rejudge` 와 같다(md5, EVENT 시계, 노트 수, PG·GR 수, 최종 EX). 어긋나면 비례로 물러나고 콘솔에 이유를 쓴다(record-target-pace.md 6절).

**3단계(보류)**: 서버 리플레이 고스트와 리플레이 없는 랭킹 행의 배치 빌리기. 필요한 것은 와이어·DB 변경이라 5절 6번에서 묻는다. 3.3절의 ChartReplicationMode 를 R-BMS 식으로 옮기는 길(랭킹 행에 `seed`/`random` 을 `extra` 로 싣기)도 이 단계다.

### 4.2 바뀌는 파일 목록(1단계)

| 파일 | 변경 |
| --- | --- |
| `apps/rbms-player/src/lib.rs` | `borrowed_layout` 필드 추가와 초기화(`:1147` 근처) |
| `apps/rbms-player/src/app_play.rs` | `BorrowedLayout`, `borrow_layout_of`, `load()` 분기(`:470-480`), `OwnPlayOptions` 보관·복원 재사용, `leave_play` 에서 정리(`:824-833`) |
| `apps/rbms-player/src/stage/select/mod.rs` | 기록 모달 키·버튼 동작, 랭킹 패널 `PlayLayout`, `poll_replay_download` 의 "배치만" 갈래 |
| `apps/rbms-player/src/stage/select/{events,skinned,overlay}.rs` | (선택) 파란 건반 쥐고 REPLAY, `Hot::ModalLayout` 연결 |
| `apps/rbms-player/src/ir_ranking_view.rs` | `PanelAction::PlayLayout`, 힌트 문구, 닫힘 시험(`:348` 근처) |
| `crates/rbms-render/src/select.rs` | 모달 버튼과 `SelectHot::ModalLayout` |
| `apps/rbms-player/src/keyconfig.rs` | `Shortcut::ReplayPlayLayout`(라벨, `config_key`, 기본 키, 그룹, `ALL` 길이) |
| `apps/rbms-player/src/stage/play/mod.rs` | `analysis_shortcut` 의 새 단축키, 실패 직후 재시도의 빌린 배치 유지(`:1449-1456`) |
| `apps/rbms-player/src/stage/result.rs`, `app_result.rs` | 같은 배치 재시도에서 빌린 배치 유지, 나갈 때 정리 |
| `crates/rbms-chart/src/shuffle.rs` | (권장) 옵션별 순열의 골든 시험(6절 크로스 버전 위험) |
| `docs/` | play-options.md 새 절, 선곡 키 안내(`docs/play-options.md` 12절), `acknowledge` ADR, `reference-divergences.md`, QA 점검표 |

### 4.3 거절·주의 규칙

| 상황 | 처리 | 근거 |
| --- | --- | --- |
| 리플레이의 md5 ≠ 포커스된 차트 md5(대소문자 무시) | 거절. 메시지에 두 해시 앞부분. 서버 리플레이는 `data.chart.md5/sha256` 도 대조(지금 `poll_replay_download` 는 대조하지 않는다, `mod.rs:906-909`) | 요구 "해시 다른 차트 거절". 지금 재생 경로는 경고만 한다(`app_play.rs:464-469`) |
| `seed == 0` 이고 옵션이 OFF 가 아님 | 거절("배치를 알 수 없음") | R-BMS 가 만드는 시드는 0 이 아니다(`app_play.rs:22-27`). 0 은 서버가 안전한 정수 밖의 옛 시드를 바꾼 값이다(`dto/common.ts:142`) |
| 서버 `random` 이 `Converge` | 거절 | R-BMS 에 같은 옵션이 없다. 재생 경로는 조용히 OFF 로 둔다(`ir_replay.rs:131-136`)지만 배치 복사에서는 다른 배치가 되므로 거절 |
| 옵션이 OFF | 그대로 허용(OFF 로 친다) | 옵션 OFF 도 한 배치다 |
| 코스·연습·오토플레이 | 진입 길을 막음(코스 제약이 RANDOM 을 덮는다: 원본 `MusicSelector.java:475-506`, R-BMS `CourseOverrides`, play-options.md 10.14절) | 원본도 연습은 이 갈래를 타지 않는다(`BMSPlayer.java:269`) |
| 노트 기반 옵션(S-RANDOM, H-RANDOM, ALL-SCRATCH)이고 차트에 정의 안 된 LN 이 있는데 리플레이의 `judge.ln_mode` ≠ 지금 LN MODE | 배치가 다를 수 있다는 알림만 하고 진행(거절·강제 변경 안 함). 서버 리플레이는 `lntype` 만 있다 | 배치 계산이 LN 해석 뒤에 도는 순서(`app_play.rs:492,506`)와 S-RANDOM 이 장기 노트를 고정하는 방식(`shuffle.rs:185` `apply_srandom`). 같은 배치가 되는지는 실측하지 못했다(6절) |
| DP 리플레이 | 허용. 쪽마다 시드에서 파생하므로 `random` 하나로 충분(`shuffle.rs:115-126`) | 2P 옵션이 따로 없다 |

### 4.4 대안

- 대안 1, 시작 때 건반 쥐기만(beatoraja 그대로): 구현은 가장 작지만 숨은 조작이고 DECIDE 가 끝날 때까지 쥐고 있어야 한다(3.2절). 모달 버튼을 두는 안이 발견성이 낫다.
- 대안 2, 결과 화면에 단축키: 기본 설정의 리플레이는 스스로 끝나지 않아(2.4절) 결과 화면에 닿지 않는다. 리플레이 분석을 끈 사용자에게만 닿으므로 보조 길이다(`stage/result.rs:511-512` 의 REPLAY_SAME 을 리플레이 결과에서도 허용하는 방식).
- 대안 3, 문자 그대로 동시 화면(C): 플레이 필드 옆에 리플레이 필드를 그리려면 DP 모양의 필드와 스킨 계약(14K 플레이 스킨), 두 `PlaySession` 이 필요하다. `LaneOption::Battle` 이 아직 런에 반영되지 않아 SP→DP 승격이 없다(`reference-divergences.md:177`). 크고 위험해서 추천하지 않는다.
- 대안 4, 점수 행에서 빌리기(ChartReplicationMode 식): 리플레이를 올리지 않은 플레이어의 배치도 빌릴 수 있으나 와이어·서버 변경이 필요하다(3단계).

### 4.5 새 스킨 id·설정 필드·DB/API 변경 여부

| 항목 | 1단계 | 2단계 | 3단계 |
| --- | --- | --- | --- |
| 스킨 id | 필요 없음. 선택: 이벤트 21002~21005(리플레이 슬롯 1~4 배치 플레이)를 외부 스킨 버튼용으로 예약 가능 | 없음 | 없음 |
| 설정 필드 | 없음(키는 `keyconfig` 의 `shortcuts` 맵, `#[serde(default)]` 이미 있음 `keyconfig.rs:1154`). 설정 스키마 6, `SETTING_COUNT` 97 유지 | 없음 | 없음 |
| 로컬 DB | 없음 | 없음 | 없음 |
| 서버 DB·API | 없음 | 없음 | 있음: 리플레이에 판정 설정(알고리즘, 폭, LN, 시계 등)을 싣는 새 형식 또는 열, 마이그레이션 0013, DTO·계약 골든 |
| 와이어 | 없음 | 없음 | `ReplayData` 확장 또는 `extra`(지금 서버는 `extra` 를 저장하지 않는다, `ir_replay.rs:42-46`) |

스킨 id 현황(조사 결과, `apps/rbms-player/src/skin_host/extension.rs:38-85`, `docs/skin.md` 1.4절): 이벤트는 21000(SCRATCH AUTO), 21001(KEEP GREEN NUMBER)만 쓰고 21002~21099 가 남는다. 불린은 21100~21104, 21110~21114, 21120~21124, 21130~21134, 21140~21148 을 쓰고 남은 번호는 21105~21109, 21115~21119, 21125~21129, 21135~21139, 21149~21199 다. 램프 id 0~10·와이어 램프 이름은 건드리지 않는다.

### 4.6 테스트 방법

모두 입력과 결과만 비교하는 헤드리스 시험이다(창·소리 없음).

1. 배치 동일성: 같은 차트에 `lay_out_notes(옵션, 시드)`를 두 번 적용한 모델과 `session_of_replay`(`ghost.rs:237-254`)의 모델이 레인별·타임라인별로 같다. 7K, 14K(DP), 9K 와 8개 옵션 전부.
2. `load()` 분기: `borrowed_layout` 이 있으면 `retry_seed` 와 새 시드보다 우선하고 `config.play.random` 이 판 동안 빌린 값, `settings_to_save` 는 플레이어 값(`app_result/record_tests` 방식).
3. 기록: 그 판의 `ScoreRecord.random`, 제출 본문 `options.random`/`seed`, 저장 리플레이의 `random`/`seed` 가 빌린 값(`record_tests/contract_requests.rs` 방식, 계약 골든은 바뀌지 않아야 한다).
4. 거절: md5 불일치, `seed == 0`+옵션 있음, `Converge`, 코스·연습 중.
5. 재시도: 파란 건반+START·R·실패 직후 SELECT 는 유지, 5번 키·선곡·N 은 정리하고 플레이어 옵션이 돌아온다.
6. 키 설정: `Shortcut::ALL` 길이, 기본 키가 `Control` 키와 겹치지 않음, 옛 키 파일 읽기.
7. UI: 기록 모달 버튼의 핫 영역, `panel_action` 의 닫힌 매핑 시험(`ir_ranking_view.rs:348`).
8. 크로스 버전 배치 골든: 옵션·시드·모드별 순열을 값으로 고정한다(지금은 같은 실행 안의 결정성만 시험한다, `shuffle.rs:370,424,639,771`).
9. 실기 점검(QA 항목으로 추가): S-RANDOM 차트를 치고 리플레이를 만든 뒤, 그 리플레이를 보면서 단축키로 배치 플레이를 시작해 노트 배열이 같은지 눈으로 확인.

게이트는 변경 위험에 맞춰 `cargo test -p rbms-player --locked` 의 관련 모듈과 `cargo test -p rbms-chart --locked`(골든) 한 번씩이면 충분하다. 웹은 1단계에서 바뀌지 않는다.

## 5. 사용자 결정이 필요한 질문

```markdown
## 결정 필요

1. 범위 — **A. 배치 빌리기(1단계) 먼저, 고스트는 2단계(추천: 원본·LR2 모두 배치 복사가 핵심이고 고스트는 이미 있는 기능을 재사용)** / B. 둘을 한 번에 / C. 동시 화면(DP 모양)
2. 진입 길 — **A. 기록 모달 버튼 + IR 패널 키 + 리플레이 보는 중 단축키(추천: 발견하기 쉽고 "보다가 바로"를 만족)** / B. 위 + 건반 쥐고 REPLAY(beatoraja 식) / C. 건반 쥐기만
3. 무엇을 복사 — **A. 옵션+시드만, 게이지·SCRATCH AUTO 는 내 설정(추천: 원본의 "譜面" 재현과 같고 판정·점수 의미가 안 바뀜)** / B. 게이지도 / C. 게이지·SCRATCH AUTO·LN MODE 전부(점수 의미가 달라질 수 있어 비추천)
4. 빌린 배치의 수명 — **A. 같은 배치 재시도에서만 유지, 선곡·다음 곡·다른 배치로는 버림(추천: 설정이 영구히 바뀌지 않음)** / B. 선곡으로 나갈 때까지 유지
5. 기록 표시 — **A. 일반 판과 같이 저장·전송, 표지 없음(추천: 원본과 같고 DB·API 변경 없음)** / B. 저장 행에 "배치 빌림" 표지 열 추가
6. 서버 리플레이 고스트·점수 행 배치 — **A. 이번에는 안 함(추천: 와이어·DB 변경이 필요하고 테스트 부채 3.79 와 한 묶음)** / B. 3단계로 진행(리플레이 판정 설정을 서버에 저장, 마이그레이션 0013 포함)
7. 배치를 알 수 없는 리플레이(시드 0, Converge) — **A. 거절하고 이유 표시(추천: 다른 배치로 치고 같은 배치라고 믿는 것을 막음)** / B. 옵션만 복사해 새 시드로 진행
```

사용자 확인이 필요한 "판정·점수 의미가 원본과 달라지는 변경"은 이 조사에서 발견하지 못했다. 위 7개는 범위·UX·저장 형태의 선택이다. 6번만 서버 변경(마이그레이션)을 포함한다.

## 6. 위험과 확인 못 한 것

1. 실기 확인 못 함: 이 조사는 코드와 문서 읽기뿐이며 앱을 실행하지 않았다. "DECIDE 가 끝날 때까지 건반을 쥐어야 한다"(3.2절)는 소스로 읽은 것이다.
2. 크로스 버전 배치 안정성: `shuffle.rs` 는 2026-06-01 이후 바뀌지 않았지만(`git log`), 시험은 같은 실행 안의 결정성만 본다. 알고리즘이 바뀌면 옛 리플레이·서버 리플레이의 배치가 조용히 달라진다. 골든 시험을 권장하는 이유다(4.6절 8번).
3. 노트 기반 옵션과 LN 해석: S-RANDOM 등은 롱노트 처리 뒤의 모델에 대해 도는데(`app_play.rs:492`, `:506`), 정의 안 된 LN 이 있는 차트에서 LN MODE 가 다르면 모델이 다를 수 있다. 같은 배치가 되는지 시험하지 못했다. 4.3절은 알림으로 처리하자고 정했고, 거절·강제 중 무엇이 맞는지는 실측 뒤 결정할 일이다.
4. 오래된 `chart_path`: 로컬 리플레이 재생은 기록 당시 경로를 쓴다(`mod.rs:748`). 라이브러리를 옮긴 사용자는 "chart not found" 가 난다(기존 문제, 범위 밖이라 보고만). 새 길은 포커스된 라이브러리 항목의 경로를 쓰게 해 이를 피한다.
5. 서버 리플레이의 손실 필드: 스크래치 방향 없음, 판정 설정 기본값(2.3절). 배치 복사는 영향을 받지 않지만 2단계 고스트는 서버 리플레이에 쓸 수 없다.
6. 24키·5키·10키 모드의 배치 시험은 하지 않았다. 코드는 모드 일반(`side_key_lanes`)이라 같은 길로 보이나 확인 못 함.
7. LR2 G-BATTLE 의 정확한 동작(ghost 표시 방식, 점수 저장 규칙)은 못 읽은 위키(atwiki 403)와 전언에 기대었다(3.6절). 용어 "ghost battle" 의 LR2 공식 설명은 확인 못 함.
8. IIDX 에서 남의 리플레이 배치를 복사하는 무료 기능의 유무는 확인 못 함(3.7절).
9. 서버 리플레이 다운로드는 인증 없이 누구나 받는다(`route/replay.ts:63-67`). 배치 복사가 공개 데이터에 기대므로 새 노출은 없지만, 비공개 리플레이 개념이 나중에 생기면 이 기능도 같은 규칙을 따라야 한다.
10. 기록 목표 설계 선택(record-target-pace.md 9절)의 미응답 질문 3번(서버 리플레이 재판정)과 이 보고 5절 6번이 같은 결정이다. 사용자가 한 번에 답하면 된다.

## 7. 구현 단위 분해

worktree 병렬 가능 단위, 파일 소유권(겹치지 않게), 의존 순서, 예상 변경 규모(코드+시험, 내 추정이며 측정하지 않았다).

| 단위 | 내용 | 소유 파일 | 의존 | 규모 |
| --- | --- | --- | --- | --- |
| L0 계약 | `BorrowedLayout` 타입과 `AppShared::borrow_layout_of` 시그니처, 거절 이유 열거형만 먼저 확정 | 없음(메인이 문서로 고정) | 없음 | 10줄 |
| L1 코어 | 필드, 검사, `load()` 분기, 보관·복원, 정리, 재시도 유지, 시험 1·2·3·4·5 | `apps/rbms-player/src/{lib,app_play,app_result}.rs`, `stage/result.rs` | L0 | 코드 160 + 시험 220 |
| L2 선곡 진입 | 기록 모달 버튼·키, IR 패널 `PlayLayout`, 다운로드 갈래, (선택) 파란 건반 쥐고 REPLAY, 시험 7 | `stage/select/{mod,events,skinned,overlay}.rs`, `ir_ranking_view.rs`, `crates/rbms-render/src/select.rs` | L0(시그니처), 합칠 때 L1 | 코드 200 + 시험 150 |
| L3 재생 중 단축키 | `Shortcut::ReplayPlayLayout`, `analysis_shortcut`, 실패 직후 재시도 유지(`:1449-1456`), 시험 6 | `keyconfig.rs`, `stage/play/mod.rs` | L0, 합칠 때 L1 | 코드 90 + 시험 90 |
| L4 골든·문서 | 배치 순열 골든(시험 8), play-options 새 절, ADR, reference-divergences, QA 점검표, 선곡 키 안내 | `crates/rbms-chart/src/shuffle.rs`(시험 모듈만), `docs/**` | L1~L3 이 끝난 뒤 문서, 골든은 독립 | 시험 60 + 문서 200 |
| G1 고스트(2단계, 선택) | 이 판의 TARGET 을 빌린 리플레이로, 결과 화면 TARGET, 시험 | `ghost.rs`, `stage/play/mod.rs`(attach), `app_result.rs`(`run_target`), `target.rs`, `skin_host/score/standing.rs` | L1 완료 | 코드 250 + 시험 200 |
| S1 서버 판정 설정(3단계, 보류) | 리플레이에 판정 설정 저장, `from_ir_replay` 복원, 마이그레이션 0013, DTO, 계약 골든, 문서 | `crates/rbms-ir`, `apps/rbms-player/src/ir_replay.rs`, `web/**` | 5절 6번 결정 | 600 이상 |

순서: L0 → (L1, L2, L3, L4 골든 병렬) → 병합 후 L4 문서 → (G1, S1 은 사용자 결정 뒤). 파일 충돌은 `stage/play/mod.rs` 가 L3 에만 있게 하고(L1 이 건드리지 않는다), `AppShared` 필드는 L1 만 추가해 막았다. 단위마다 변경 위험을 직접 덮는 시험 한 번이면 되고 전체 게이트는 병합 뒤 한 번이다.

## 8. 출처

R-BMS(저장소 안): 본문 각 절의 파일:줄.
beatoraja(읽기만, `/Users/hyunseokbyun/development/beatoraja/src/bms/player/beatoraja/`): `ReplayData.java`, `play/BMSPlayer.java`, `select/MusicSelector.java`, `select/MusicSelectKeyProperty.java`, `select/MusicSelectInputProcessor.java`, `result/MusicResult.java`, `result/ResultKeyProperty.java`, `pattern/{Random,LaneShuffleModifier,PatternModifier}.java`, `PlayDataAccessor.java`, `PlayerConfig.java`, `MainController.java`.
외부:
- https://toiblog.com/bms/19419/ (LR2 G-BATTLE 설명, 전언)
- https://w.atwiki.jp/echoes140/pages/220.html (ECHOES wiki, 검색 요약으로만 확인, 본문 확인 못 함)
- https://github.com/rian-eimu/lr2oraja-endlessdream/pull/33 (ghost 가 노트 수 불일치로 무효화된다는 코드 주석)
- https://remywiki.com/What_is_beatmania_IIDX (IIDX BATTLE, Music retry, Ghost 표시)
- https://github.com/monadnadnad/iidx-rlt (ランダムレーンチケット 설명)
