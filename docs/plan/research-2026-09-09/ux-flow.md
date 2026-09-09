# rbms 화면 흐름 전수 UX 감사 + 게임 시작 전 옵션 설계 + 타깃/그래프

조사일 2026-09-09. 읽기 전용 조사(파일 수정 없음). 근거는 rbms / beatoraja 양쪽 file:line.
시간 상한 15분 기준으로 코드 근거 확보에 집중했고, 못 본 범위는 §5에 명시.

---

## 1. 흐름 전수 — 메인 스레드 블로킹 지점

winit 이벤트 루프(단일 스레드)에서 직접 실행되는 무거운 작업 전수. "스레드" 열은 실제 실행 위치.

| # | 지점 | 코드 근거 | 스레드 | 사용자 체감 |
|---|---|---|---|---|
| B1 | 난이도표 URL/파일 추가 → HTTP fetch | `app_select.rs:740-747`(`add_table_source`→`load_and_match`), `tablesrc.rs:47-55`, `crates/rbms-table/src/lib.rs:140-149`(`reqwest::blocking`, `.timeout(15s)`) | **메인** | Enter 후 최대 15초 완전 프리징. 로딩 표시 없음 |
| B2 | 곡선택 포커스 이동 시 차트 상세 파싱 + 커버 디코드 | `app_select.rs:265-278`(`refresh_focused_detail` → `compute_chart_detail`, `decode_bga_256`) | **메인** | 행을 이동할 때마다 파싱 1회 + 이미지 디코드 1회. 스크롤 히칭 |
| B3 | 차트 로드(파싱·셔플·스킨) | `app_play.rs:9-60`, 호출 `app_select.rs:849-858`(`finish_loading`) | **메인** | LOADING 화면을 1프레임 먼저 그린 뒤 실행(`begin_loading` `app_select.rs:840-845`) → 정지 화면. 진행률 없음 |
| B4 | BGA 이미지 전량 디코드 | `app_play.rs:105-115`(`for … decode_bga_256`) | **메인** | 키음은 스레드인데(아래) BGA는 동기. 키음 진행바가 100%인데도 멈춰 보임 |
| B5 | rfd 네이티브 다이얼로그 3종 | `app_select.rs:676`(폴더), `app_select.rs:748`(표 json), `app_input.rs:147`(폰트) | **메인**(모달) | 다이얼로그 동안 창 리페인트 중단 |
| B6 | 결과 진입 시 파일 3종 저장 | `app_play.rs:334`(리플레이), `:359`(scores.ron 전체 재직렬화), `:367`(auto-cal 시 settings) | **메인** | 기록이 쌓일수록 결과 진입 1프레임 지연 증가 |
| B7 | 설정 저장 | `app_input.rs:135-137`(`save_settings`), 호출부 `main.rs:1036,1051` | **메인** | 소규모, 실질 무해 |

정상적으로 백그라운드로 뺀 것(대조군):
- 폴더 스캔: `app_select.rs:695-710`(`rescan_all_folders` → `thread::spawn`), 진행 카운터 `scan_count`, 화면 `app_play.rs:501`
- 키음 디코드: `app_play.rs:73-83`(`spawn_keysound_decode`), 진행바 `app_play.rs:487`
- 미리듣기 autoplay 준비: `app_select.rs:474-510`(파싱·스케줄·디코드 전부 워커, 취소 플래그)
- 스코어 제출: `app_play.rs:312-315`(`thread::spawn`)
- 서버 health 폴링 5초: `main.rs:181-186`

→ **패턴이 이미 존재한다(스캔·키음·미리듣기).** B1~B4는 같은 패턴을 안 쓴 누락이지 구조적 한계가 아니다.

## 2. 로딩 표시 / 에러 피드백

- LOADING 화면은 폴더 스캔(발견 곡 수)·키음 디코드(진행바) 두 가지만 표시: `app_play.rs:487`(`ks_progress`/`ks_total`), `:501`(`scan_count`). B1·B2·B4는 표시 없음.
- **GUI 상 에러 피드백 수단이 하나도 없다.** 실패는 전부 `eprintln!`/`println!`로 터미널에만 나간다:
  - 차트 파일 없음 `app_play.rs:16-18` (그리고 조용히 Select 복귀 `app_select.rs:857`)
  - 표 로드 실패 `tablesrc.rs:84`("table load failed") → 화면에는 레벨 0개 폴더가 사라질 뿐
  - 폰트 로드 실패 `app_input.rs:155-157`
  - 미리듣기 실패 6분기 `app_select.rs:403,412,421,428,436` (게다가 `config.debug`일 때만 출력)
  - 스코어 제출 실패 `app_play.rs:314`
  - 설정/스코어/표 파일 쓰기 실패 `settings.rs:97`, `scores.rs:58`, `tables.rs:43`
  - 리플레이 md5 불일치 경고 `app_play.rs:33`
  GUI 빌드(창만 띄우는 실행)에서는 사용자가 이 메시지를 볼 수단이 없다. **토스트/상태줄 1종이 최우선 UX 부채.**
- 서버 연결 상태만은 시각화됨: `app_play.rs:744`(`server_connected`).

## 3. 곡선택 미리듣기 UX

| 항목 | 현재 | 근거 |
|---|---|---|
| 디바운스 | **프레임 수 기준 20프레임** — 프레임레이트에 따라 실제 대기시간이 변함(144Hz≈139ms, 60Hz≈333ms, 30fps≈667ms) | `main.rs:59`(`PREVIEW_DEBOUNCE_FRAMES: u64 = 20`), `app_select.rs:303` |
| 페이드 | 없음(즉시 재생/즉시 정지) | `app_select.rs:439-441`(`eng.play(PREVIEW_ID, PREVIEW_GAIN, …)`), `reset_preview_playback` `:518-540` |
| 볼륨 | 상수 `PREVIEW_GAIN` 고정, 설정 항목 없음 | `main.rs:56-60` 부근 상수 블록, 설정 목록 `app_select.rs:885-` |
| 정지 타이밍 | 플레이 진입 시 `stop_preview()`로 cpal 스트림 파기(스트림 공존 방지) | `app_play.rs:12` |
| 온오프 | DISPLAY 탭 PREVIEW 토글 | `PlaySettings.preview`, `app_input.rs:130` |
| 루프 | `#PREVIEW`는 길이 기반 재발화, autoplay 미리듣기는 스케줄 끝+tail | `app_select.rs:440-443`, `:500-503` |

문제: 디바운스 단위(프레임)와 상세 파싱(B2, 디바운스 없음)의 기준이 서로 다르다. 상세 파싱은 **디바운스가 아예 없어** 빠른 스크롤에서 매 행 파싱한다.

## 4. 조작 일관성 / 그 밖의 UX

- Tab 키 과부하: Select에서 Tab=설정 진입(`main.rs:1003`), Settings에서 Tab=탭 순환(`main.rs:1039`). 같은 키가 "화면 이동"과 "탭 이동" 두 의미.
- 뒤로가기 키가 화면마다 다름: Select Esc/←=상위(`main.rs:999`), Result Esc/Enter=Select(`main.rs:1086-1088`), Loading Esc=취소 후 **Root로 리셋**(`main.rs:1091-1106`, 선택 위치 유실), Folders Esc=전체 재스캔(`app_select.rs:717`).
- **빈 라이브러리에서 Esc=앱 종료**: `app_select.rs:867-869`(`to_select_or_exit`가 `songs.is_empty()`면 `event_loop.exit()`). 첫 실행에서 폴더 추가를 취소하면 그대로 앱이 꺼진다.
- 텍스트 입력(SERVER URL / PLAYER ID / 표 URL): 커서 없음, 붙여넣기 없음, 커서 이동/선택 없음, 긴 문자열 스크롤 없음. 처리 분기는 Enter/Esc/Backspace/문자만 (`app_input.rs:170-200`, 표 URL은 `app_select.rs:769-782`).
- 정렬: F3이 메뉴 없이 순환(`main.rs:1002`, `app_select.rs:47-50`). `SortMode::Level` 비교자가 매 비교마다 `parse::<i64>()` (`app_input.rs:70-73`) — 라이브러리가 커지면 정렬 비용이 눈에 띈다.
- 검색: `/`가 현재 폴더 컨텍스트를 버리고 AllSongs로 점프(`app_select.rs:25-33`), 매 타이핑마다 전체 리스트 재구성+정렬(`apply_search` `:40-44` → `rebuild_select_items` → `arrange_songs`). 결과 개수 표시 없음.
- 창 리사이즈: `WindowEvent::Resized`가 `gpu.resize`만 호출(`main.rs:935-938`, `gpu.rs:329`). 1280×720 레퍼런스 좌표 고정이므로 레이아웃 리플로우 없음(스케일만). PROCESS.md Phase 7 "리사이즈 리플로우 (선택)"와 일치.
- 디버그 기능: `debug`는 **bool 1개**이고 실제 사용처는 미리듣기 eprintln 뿐(`main.rs:94,127,153`, `app_select.rs`의 `dbg` 분기). 온스크린 디버그 오버레이·판정 타이밍 히스토그램은 없다. 판정 타이밍 시각화는 리플레이 분석 모드의 노트별 ms-off(`main.rs` analysis 계열, PROCESS.md §4)뿐 — 라이브 플레이 중에는 FAST/SLOW 카운터만.
- 설정 즉시 반영: 대체로 즉시(`adjust_setting` → config 직접 변경, lift는 `rebuild_skin` 동반 `app_input.rs:98-105`). 다만 **플레이 중 바꾼 hispeed/cover/lift는 저장되지 않는다** — `apply_control`(`app_input.rs:88-106`)이 `save_settings()`를 부르지 않고, 저장은 Settings 화면 Esc/Enter(`main.rs:1036,1051`)·폰트·네트워크 편집·auto-cal(`app_play.rs:367`) 시점뿐. 곡을 몇 번 돌며 맞춘 하이스피드가 재실행 시 사라진다.
- 결과 화면 정보량: `ResultView`(`app_play.rs:218-233`) = 판정 6종 카운트·EX·최대콤보·총노트·fast/slow·게이지·클리어라벨·직전 EX·베스트 EX·그래프 토글. **없는 것**: 타깃 대비 비교, 시간축 게이지/스코어 추이 그래프, 판정 오차 분포, 코스 결과.

## 5. 게임 시작 전 옵션 — 3방식 비교와 권고

### 현황 근거
- rbms: Select에서 Tab → **전체 화면 설정(Stage::Settings)** 으로 전환(`main.rs:1003`). 곡 정보/상세 패널이 통째로 사라진다. 탭 5+1종(PLAY/GAUGE/JUDGE/DISPLAY/INPUT/NETWORK), 값 조정은 ←→(`main.rs:1057-1066`). 플레이 중 조작은 hispeed/cover/lift 3종뿐(`app_input.rs:88-106`, `ControlAction::ALL`).
- beatoraja: 곡선택 리스트를 **떠나지 않고** START 홀드 중에만 옵션 패널이 뜬다 — `select/MusicSelectInputProcessor.java:114-151`(`input.startPressed()` → `select.setPanelState(1)` + OPTION_OPEN 사운드, 이후 OPTION1_UP/DOWN·GAUGE_UP/DOWN·OPTIONDP·OPTION2·HSFIX 를 홀드 중에만 해석). SELECT 홀드는 어시스트 패널(`:206-241`), 다른 홀드는 상세 패널(`:243-290`). 손을 떼면 패널 닫힘(`:100-107`). 플레이 중에는 `play/ControlInputProcessor.java:130-166`(START+↑↓ 하이스피드, 휠 레인커버, START 더블프레스로 커버 on/off), `:205-216`(ESCAPE, NUM1~4 = 리트라이/연습 등 분기).
- IIDX: 곡선택에서 START 홀드 옵션 패널, 플레이 중 START+턴테이블/키로 하이스피드·SUD+ 조정. (외부 지식, 코드 근거 없음 — **미확인** 표기)

### 비교

| 방식 | 장점 | 단점 | rbms 적용 비용 |
|---|---|---|---|
| A. 전체화면 설정(현행) | 항목이 많아도 다 보인다. 탭으로 분류 가능. 키설정·폰트·네트워크 같은 "환경설정"과 같은 자리 | 곡 컨텍스트 상실(이 곡에 무슨 램프/베스트가 있는지 안 보임). 곡마다 옵션을 바꾸는 리듬게임 루프와 상충. 왕복이 잦다 | 0 (이미 있음) |
| B. 곡선택 오버레이 패널(beatoraja) | 곡 리스트·상세·기록을 보면서 랜덤/게이지/하이스피드를 바꾼다. 홀드-조작-릴리스가 1동작. 플레이 직전 옵션이 곧 제출 옵션 | 홀드 키 개념을 키설정에 추가해야 함. 항목 수 제한(패널 크기). 키보드 홀드+방향키 동시 입력 처리 필요 | M — 오버레이 렌더 1종 + 홀드 상태 머신. `SelectScene`에 패널 필드 추가로 흡수 가능 |
| C. decide/플레이 중 전환 | 로딩 대기시간을 옵션 조정에 활용. 실제 노트가 흐르는 걸 보며 하이스피드 확정 | 옵션이 판정/제출에 영향 → 확정 시점이 모호. 랜덤/게이지는 로드 후 변경 불가(셔플이 `load()`에서 적용 `app_play.rs:51`) | S(hispeed/cover/lift는 이미 있음) / 랜덤·게이지는 불가 |

### 권고안 (순서대로)

1. **B를 주 경로로 도입, A는 "환경설정"으로 축소.** 곡선택에서 옵션 홀드 키(기본 Shift 또는 키설정의 새 `ControlAction::OptionPanel`)를 누르는 동안 우측 상세 패널 위에 오버레이: RANDOM / GAUGE / HI-SPEED(+SPEED FIX) / LANE COVER / LIFT / SCRATCH SIDE·AUTO / AUTOPLAY. ↑↓로 항목, ←→로 값. 릴리스 시 닫고 **즉시 `save_settings()`**.
   - 이유: 이 7개가 "곡마다 바꾸는" 값이고, 나머지(키설정·폰트·서버·스킨·PREVIEW·SCORE GRAPH)는 세션당 한 번 바꾸는 값이다. 현재 `SETTING_TABS`는 둘을 섞어 놓았다.
2. **플레이 중 조작에 저장을 붙인다** — `apply_control`(`app_input.rs:88-106`) 끝에 `save_settings()` 또는 결과 진입 시 1회 저장. 지금은 유실된다.
3. **플레이 중 PAUSE / RETRY 추가** — beatoraja `ControlInputProcessor.java:205-216` 대응. 현재 Esc는 즉시 이탈(`main.rs:1107-1118`)뿐이라 오조작 복구 수단이 없다. `ControlAction`에 `Pause`/`Retry` 2종 추가가 최소 구현.
4. decide 화면(C)은 **도입하지 않는 것을 권고.** rbms는 이미 LOADING 단계에서 키음 진행바를 보여주고 있고, 옵션 확정 시점이 두 곳으로 갈라지면 제출 payload(`ScoreSubmission.options`, `app_play.rs:280-300`)의 진실 출처가 흐려진다.

## 6. 타깃 / 그래프

### beatoraja 타깃 전수 (`play/TargetProperty.java`)

| 종류 | id 형식 | 근거 |
|---|---|---|
| 고정 레이트 | `RATE_A-` `RATE_A` `RATE_A+` `RATE_AA-` `RATE_AA` `RATE_AA+` `RATE_AAA-` `RATE_AAA` `RATE_AAA+` `RATE_MAX-` `MAX` (각각 17/27~26/27, 100%) | `TargetProperty.java:113-141` |
| 임의 레이트 | `RATE_<0~100 실수>` → "SCORE RATE n%" | `:143-152` |
| 라이벌(개별) | `RIVAL_<n>` — n번째 라이벌의 그 차트 스코어 | `:279-287`, 해석 `:186-192` |
| 라이벌 랭크 | `RIVAL_RANK_<n>` — 라이벌+본인 스코어 정렬 후 n위 | `:269-277`, `:193-201` |
| 라이벌 넥스트 | `RIVAL_NEXT_<n>` — 본인 위 n번째 | `:259-267`, `:202-219` |
| 넥스트 랭크 | `RANK_NEXT` — 현재 EX 위쪽 첫 랭크 경계(max×i/27, i=15..26) | `:296-320` |
| IR 랭킹 | `InternetRankingTargetProperty` (getTargetProperty 체인 3번째) | `:62-68` |

타깃은 `ScoreData`(epg/lpg/egr/lgr/option)로 환원되어 스킨 그래프가 소비한다. 관련 스킨 요소: `play/SkinGauge.java`, `play/SkinJudge.java`, 분포 그래프는 `select/SkinDistributionGraph.java`.

### rbms 현황

| 항목 | 현재 | 근거 |
|---|---|---|
| 라이브 스코어 그래프 | 세로 패널 + A/AA/AAA(6/9·7/9·8/9) 밴드선 + NOW(시안) / BEST(초록) 2바 + "vs BEST ±n" | `crates/rbms-render/src/hud.rs:29-52` |
| 그래프의 "타깃" | **로컬 베스트 EX 단 하나.** `HudView.best_ex` 주석 "Local best EX on this chart" | `hud.rs:20-24`, 공급 `app_play.rs`의 HudView 조립 |
| 결과 랭크 그래프 | `dj_rank`(9분법) + `draw_rank_bar` | `crates/rbms-render/src/result.rs:47-58,97-104` |
| 결과 비교 | 직전 EX(`prev_ex`) / 로컬 베스트 EX(`prev_best_ex`) | `app_play.rs:214-232` |
| 시간축 추이 그래프 | **없음** (게이지·스코어 모두 현재값 바만) | `hud.rs` 전체 |
| 타깃 선택 UI | **없음** | `SETTING_TABS` 항목 목록 `app_select.rs:885-` |

### IR/커스텀 서버 기록을 타깃으로 쓰기 위한 배선

`crates/rbms-ir/src/lib.rs`에 필요한 메서드는 **이미 trait에 다 있다.** 문제는 **플레이어가 한 번도 호출하지 않는다**는 것: 플레이어에서 실제로 부르는 것은 `health()`(`main.rs:183`)와 `submit_score()`(`app_play.rs:312`) 둘뿐이다(전수 grep 확인).

| 배선 항목 | 사용할 기존 API | 상태 | 효력 |
|---|---|---|---|
| IR 랭킹 n위 타깃 | `ScoreServer::chart_ranking(&ChartId, limit)` `lib.rs:50` | trait 존재, **호출부 0** | 곡 포커스 settle 시 비동기 조회 + 캐시 필요(미리듣기 코디네이터와 같은 패턴) |
| 내 서버 베스트 타깃 | `player_best(&ChartId, &PlayerId)` `lib.rs:51` | trait 존재, 호출부 0 | 로컬 베스트와 별도 표시(기기 이동 시 유용) |
| 라이벌 타깃 | `rivals(&PlayerId)` `lib.rs:52` + 라이벌별 `player_best` | trait 존재, 호출부 0 | 라이벌 목록 저장소·UI 필요(설정 탭 신설) |
| NEXT RANK 타깃 | 서버 불필요 — `dj_rank`/`RANK_BANDS`(`result.rs:29-58`)로 로컬 계산 | **즉시 가능** | 다음 랭크 경계 EX를 목표선으로. 비용 S |
| 고정 레이트 타깃(A~MAX) | 서버 불필요 — `max_ex * i/27` | **즉시 가능** | beatoraja `TargetProperty.java:113-141` 그대로 이식. 비용 S |
| 그래프 다중 타깃선 | `hud.rs:29`의 `best: Option<u32>` → `targets: &[(label, ex, color)]` 로 일반화 | 시그니처 변경 1곳 | 3바(NOW/BEST/TARGET) 또는 목표선 오버레이 |
| 시간축 추이 | `Player` 판정 이벤트 스트림에서 (t, ex) 샘플링 버퍼 | 신규 | 결과 화면 게이지/스코어 추이 그래프의 전제 |

**권고 순서**: (1) 타깃 개념을 `TargetKind` enum + 설정 항목으로 도입하고 **서버 불필요한 것부터**(MAX / RATE_* / RANK_NEXT / LOCAL_BEST) 구현, (2) `draw_score_graph`를 다중 타깃 시그니처로 일반화, (3) 그 뒤에 `chart_ranking`/`player_best`를 곡 포커스 settle 비동기 조회로 붙여 IR 타깃을 추가. 서버 배선 없이도 타깃 기능의 대부분이 완성된다.

## 7. 문서 vs 코드 (stale 확인)

- `docs/PROCESS.md` §4의 "#PREVIEW … **실재생 미동작**" 서술은 **stale**. 코드에는 하이브리드 미리듣기(파일 + autoplay fallback)가 구현되어 있고(`app_select.rs:380-510`), 같은 문서 상단 Phase 1 항목에는 2026-06-07 완료로 적혀 있다. 같은 문서 안에서 모순.
- 그 외 §3 크레이트 맵·§4 완료 기능은 코드와 대체로 일치(검증한 범위 내).

## 8. 미조사 범위

- 마우스 조작 전수(어느 화면이 클릭을 받고 어느 화면이 안 받는지) — Select 행·Settings 값은 확인, Folders/Tables/KeyConfig/Result는 **미확인**.
- `keyconfig.rs`(660줄) 에디터 UX 전체.
- `crates/rbms-render/src/select.rs` 렌더 세부(밀도 그래프·모달 레이아웃).
- beatoraja `select/PreviewMusicProcessor.java`(미리듣기 페이드/볼륨 정책) 대조 — rbms 미리듣기 페이드 부재의 레퍼런스 근거를 확보하지 못함.
- beatoraja `select/MusicSelectSkin.java` 옵션 패널 표시 요소 목록.
- IIDX 조작 체계는 외부 지식이며 코드 근거 없음(**미확인**).
- 실제 실행/렌더 확인은 하지 않음(읽기 전용 조사, 헤드리스 예제 미실행).
