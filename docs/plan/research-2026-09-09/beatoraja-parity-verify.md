# beatoraja 비판정 기능 패리티 보고서 — 적대적 검증

대상 보고서: `scratchpad/research/feature-inventory.md` (findings 01~19)
검증 방식: 인용된 rbms/beatoraja 파일을 직접 통독. 수치(정렬 종류·enum 개수 등)는 원본에서 재계수.
검증 소요: 약 10분. 미조사 범위는 문서 하단에 명시.

## 총평

19건 중 **12건 확인(confirmed)**, **6건 부분정정(partially)**, **1건 uncertain**, refuted 0건.
반박 가능한 "이미 구현되어 있는데 미구현이라고 했다" 유형의 오류는 없었다. 정정은 대부분
**범위 과장(전부 없다 → 일부는 있다)** 과 **수치 오차**다. 심각도 자체를 뒤집을 오류는 없다.

## 판정 표

| id | 판정 | 요지 |
|---|---|---|
| 01 | confirmed | 곡DB 캐시 없음·전량 재파싱·md5+sha256 매번. 단 "단일 스레드"는 맞고 "부팅을 프리징"은 아님(백그라운드 스레드) |
| 02 | confirmed | select_key에 sel 포함·행 전량 clone·행마다 O(N) 스코어 스캔 모두 사실 |
| 03 | confirmed | gamepad/MIDI 의존성 0 |
| 04 | confirmed | fs::write 직접, 원자 교체 아님 |
| 05 | confirmed | Course 심볼 클라이언트 0건 |
| 06 | partially | 정렬 5/12는 정확. "필터 0"은 부정확 — 텍스트 검색 필터는 있음 |
| 07 | confirmed | master_gain 1개, 재생 gain 상수 1.0 |
| 08 | confirmed | chart_ranking 호출부 없음 |
| 09 | confirmed | practice 심볼 0건 |
| 10 | partially | Esc 즉시 이탈 사실. 단 이는 PROCESS §7이 기록한 **의도된 사양** |
| 11 | partially | "자동 저장 1건만"은 오류 — 플레이마다 별도 파일 저장·모달에서 전체 재생 가능 |
| 12 | confirmed | 패턴 옵션 축이 random 1개뿐 |
| 13 | partially | "추가"는 UI 스레드 동기 fetch 맞음. "갱신(rescan)"은 이미 백그라운드 스레드 |
| 14 | confirmed | ResultView에 게이지 시계열·target 없음 |
| 15 | confirmed | 이미지 BGA만 |
| 16 | partially | 사실. 단 beatoraja KeyCommand는 11종이 아니라 **13종** |
| 17 | confirmed | Stage에 Decide 없음 |
| 18 | uncertain | rbms 측 부재는 확인. beatoraja `external/`·`stream/` 디렉토리는 시간상 미확인 |
| 19 | confirmed | PROCESS.md 경로 stale·커버리지 격차 미기재 |

## 항목별 검증

### 01 곡 라이브러리 캐시 부재 — confirmed (정정 1건)

`apps/rbms-player/src/main.rs:406-443` 의 `scan_folder` 는 스택 기반 단일 스레드 DFS로
`fs::read` → `rbms_parser::parse` → `detect_mode` 를 전 파일에 대해 수행한다. 캐시 조회 분기는 없다.
`crates/rbms-parser/src/lib.rs:128-129` 가 `Md5::digest` 와 `Sha256::digest` 를 **항상 둘 다** 계산하므로
"2중 해시 매번" 도 사실이다. beatoraja 측 `song/SQLiteSongDatabaseAccessor.java:461 updateSongDatas(..., updateAll, ...)`,
`:1052-1056 BMSFolder(Path, String[], long lastModifiedTime)` 로 mtime 기반 증분 갱신 인용도 원문과 일치한다.

**정정**: 보고서가 "부팅 때마다 전량 스캔"을 문제 삼으면서 프리징을 암시하지만,
`main.rs:795-804` 는 스캔을 `std::thread::spawn` 으로 배경 실행하고 LOADING 화면을 계속 그린다
(`main.rs:777-780` 주석이 "a big library took seconds and froze startup before any UI" 라며 이미 수정된 이력임을 명시).
따라서 증상은 UI 프리징이 아니라 **부팅 후 곡목록 사용 가능까지의 지연 + 디스크/CPU 낭비**다. severity high가 타당하며 critical은 과대다.

### 02 곡선택 O(N×M) — confirmed

`app_select.rs:78-80` `select_key()` = `(select_gen, self.sel, record_modal, scores.records.len(), score_graph)`.
`app_select.rs:86-93` `refresh_select_cache` 가 이 키 변화 시 `build_select_view()` 전체 재실행.
`app_select.rs:99-127` 의 map 은 모든 `select_items` 에 대해 `label.clone()` / `title.clone()` / `level.clone()` 을 수행하고
행마다 `scores.best_clear_for_md5` 를 호출한다. 그 함수는 `scores.rs:85-86` 에서
`records.iter().filter(|r| r.md5.eq_ignore_ascii_case(md5)).map(...).max()` — 전체 기록 선형 스캔이다.
커서 1칸 이동 = 행 수 × 기록 수 회의 대소문자 무시 문자열 비교 + 행 수 × 3회 String 할당. 주장 그대로다.

### 03 컨트롤러/MIDI 없음 — confirmed

`grep -niE "gilrs|gamepad|joystick|midi"` 를 `--include='*.rs' --include='*.toml'` 로 재실행한 결과
실질 히트는 `crates/rbms-ir/src/dto.rs:493,501` 의 테스트 JSON 리터럴 `"input_device": "midi"` 뿐이다.
beatoraja 측 `input/` 디렉토리에 `BMControllerInputProcessor.java`, `MidiInputProcessor.java`,
`MouseScratchInput.java`, `BMSPlayerInputDevice.java` 실재를 `ls` 로 확인했다. 주장 그대로다.

### 04 비원자 저장 — confirmed

`scores.rs:53-63` `ron::ser::to_string_pretty` → `std::fs::write(path, &s)`. tmp+rename 없음, fsync 없음, 락 없음.
`scores.rs:39-44` 의 `.ron.bak` 은 **load 시 파싱 실패 경로**에서만 `rename` 하므로 이미 손상된 파일을 백업한다는 지적도 정확하다.
`folders.rs:28-42` 도 동일 패턴. `replay.rs:36-44` 역시 같은 비원자 write 이며 보고서가 언급하지 않았다(→ missed).

### 05 코스 미구현 — confirmed

`grep "Course|course"` 를 `crates/rbms-table/src`, `crates/rbms-play/src`, `apps/rbms-player/src` 에 걸었을 때
유일한 히트는 `apps/rbms-player/src/ir_map.rs:207` 의 주석("dan-course gauges rbms never plays")이다.
IR 쪽에만 `crates/rbms-ir/src/lib.rs:54 submit_course`, `:60 course_ranking` 이 존재한다.
beatoraja 측 `CourseData.java`·`CourseDataAccessor.java`(루트), `result/CourseResult.java`·`CourseResultSkin.java` 실재 확인.

### 06 필터·정렬·즐겨찾기 — partially

정렬 수치는 양쪽 다 정확하다. rbms `main.rs:543-566` `SortMode` 는 Default/Title/Artist/Level/Clear 5종,
beatoraja `select/BarSorter.java` enum 상수를 재계수하면 TITLE(:19)·ARTIST(:47)·BPM(:65)·LENGTH(:83)·LEVEL(:101)·
CLEAR(:126)·SCORE(:144)·MISSCOUNT(:167)·DURATION(:185)·LASTUPDATE(:206)·RIVALCOMPARE_CLEAR(:221)·RIVALCOMPARE_SCORE(:236)
= **정확히 12종**.

**정정**: "필터 0" 은 틀렸다. `app_select.rs:33` `close_search`("Close the search box (Esc) and clear the filter"),
`:41` "Re-filter after the query changed (every keystroke)" 로 **제목 검색 필터가 구현되어 있다**
(`Hot::NavSearch`, `main.rs:583`). 없는 것은 **난이도 필터·모드 필터·즐겨찾기**다.
beatoraja 측 `select/DifficultyFilter.java`·`ModeFilter.java` 실재는 `ls select/` 로 확인.
severity medium 유지, 다만 "필터 0 → 검색만 있고 난이도/모드 필터 없음" 으로 문구 수정 필요.

### 07 볼륨 3분리 없음 — confirmed

`crates/rbms-audio/src/mixer.rs:50,61,77,179` 에 `master_gain` 만 존재하고 채널 구분이 없다.
`engine.rs:103 set_master_gain` 이 유일한 노출점. `settings.rs:10-38` `PlaySettings` 필드 전량을 통독했고
volume 계열 필드는 0개다. 호출부 `main.rs:1137` `audio.play(..., 1.0, 0.0, 1.0, ...)` 로 gain 상수 1.0 확인.
beatoraja `AudioConfig.java:46 systemvolume`, `:50 keyvolume`, `:54 bgvolume`, `:24 deviceBufferSize`,
`:32 sampleRate`, `:37 freqOption` 전부 인용 라인과 일치.

### 08 IR 랭킹 표시 없음 — confirmed

`crates/rbms-ir/src/lib.rs:50` 에 `chart_ranking` 트레잇 메서드는 있으나 `apps/` 전체에서 호출부가 없다
(grep 결과 히트는 lib/null/dto 및 테스트뿐). `crates/rbms-render/src/select.rs` 의 DetailView 에도 랭킹 필드 없음.
beatoraja `select/MusicSelector.java:76-77` 의 `rankingDuration = 5000`, `rankingReloadDuration = 10*60*1000`,
`:223-250` 의 `main.getRankingDataCache().get(song, ...)` + `TIMER_IR_CONNECT_*` 스위칭까지 원문 그대로 확인.

### 09 연습 모드 없음 — confirmed

`grep -i practice` 의 rbms 히트는 `scores.rs:29` 주석("Append-only in practice") 1건뿐.
`main.rs:478-487` `Stage` enum = Select/Settings/KeyConfig/Tables/Folders/Loading/Play/Result — Practice 없음.
beatoraja `play/PracticeConfiguration.java`·`play/SkinPractice.java`·`pattern/PracticeModifier.java` 실재 확인.

### 10 일시정지·리트라이 없음 — partially

`main.rs:1107-1117` 코드는 주장대로다(전 노트 판정 완료 시 `enter_result`, 아니면 `to_select_or_exit`).
**정정**: 이는 버그나 누락이 아니라 **의도적으로 도입된 사양**이다 —
`docs/PROCESS.md` §7 하단 세션 이력에 "Play-Esc-즉시결과"가 완료 기능으로 명시되어 있다.
따라서 "Esc 즉시 이탈을 길게 누르기로 바꾼다"는 권고는 기존 결정을 되돌리는 것이므로
사용자 확인 없이 적용하면 안 된다. 진짜 갭은 **Pause/Retry의 부재**이며 그 부분은 confirmed.

### 11 리플레이 슬롯 — partially

**정정**: "자동 저장 1건만"은 사실이 아니다. `app_play.rs:319-336` 은 파일명을
`format!("{stem}-{played_ms}.ron")`(md5 앞 8자 + 밀리초 타임스탬프)로 만들어 **플레이마다 새 파일**을 남기고,
각 `ScoreRecord.replay_file` 이 자기 파일을 가리킨다. `app_select.rs:561-583 play_record_replay` 는
기록 모달에서 선택한 **임의의 과거 기록**을 재생할 수 있다.
따라서 실제 갭은 (a) 슬롯 정책 부재(무한 누적 → 디스크 증가, 정리 수단 없음),
(b) beatoraja `PlayerConfig.java:169 autosavereplay[]` 같은 클리어타입별 슬롯 및 `select/MusicSelectCommand.java` 의
슬롯 전환 커맨드 부재다. severity low 유지, 문구는 "슬롯/보존정책 부재 + 무제한 누적"으로 정정.

### 12 패턴 옵션 절반 미구현 — confirmed

`settings.rs:10-38` 에 패턴 관련 필드는 `random: String` 하나뿐(및 `scratch_auto`).
`main.rs:918-921` 의 옵션 순회도 `NoteOption::ALL` 단일 축.
beatoraja `pattern/` 디렉토리 `ls` 결과 `ExtraNoteModifier`·`MineNoteModifier`·`LongNoteModifier`·
`ScrollSpeedModifier`·`ModeModifier`·`PracticeModifier` 실재, `PlayerConfig.java:113 scrollMode`,
`:119 longnoteMode`, `:136 mineMode`, `:142 extranoteType` 라인 일치.

### 13 표 fetch 동기 — partially

**정정**: 갱신 경로는 이미 비동기다. `main.rs:795-801`(부팅)과 `app_select.rs:700-706`(rescan) 모두
`std::thread::spawn` 안에서 `fetch_and_match` 를 호출한다.
동기 블로킹이 실제로 남아 있는 곳은 **표 추가 경로**다: `app_select.rs:739-745 add_table_source` 가
UI 스레드에서 `load_and_match` → `tablesrc.rs:51 DifficultyTable::fetch_or_cache` → `rbms-table/src/lib.rs:50 fetch`(reqwest blocking)
를 직접 호출한다. `add_table_file`(`:747-753`)·`tables_input`(`:771-780`) 도 같은 경로.
"코스 정의 미파싱"(`crates/rbms-table/src/lib.rs` 에 course 없음)은 confirmed.
PROCESS.md §7 의 "난이도표 추가 fetch는 동기(1개씩)" 기술과도 일치한다(문서 stale 아님).

### 14 결과 게이지 그래프·target 없음 — confirmed

`crates/rbms-render/src/result.rs:5-22` `ResultView` 필드 전량을 확인했고 시계열/target 필드는 없다
(`prev_best_ex`, `prev_ex`, `show_graph` 까지가 전부).
beatoraja `result/SkinGaugeGraphObject.java`·`play/TargetProperty.java` 실재, `PlayerConfig.java:72 targetid = "MAX"` 라인 일치.

### 15 BGA 비디오 미지원 — confirmed

`app_play.rs:108-123` 이 `decode_bga_256`(image 크레이트, `main.rs:351`) 결과만 `bga_images: HashMap<i32, Vec<u8>>` 에 담고,
`app_play.rs:536` 렌더도 이 맵만 조회한다. 비디오 디코더 의존성/코드 0건.
PROCESS.md §7 에도 "BGA 비디오(mpg) 미지원"이 이미 기재되어 있다.

### 16 KeyCommand 계층 없음 — partially (수치 정정)

rbms 측은 맞다: `main.rs:576-591` `Hot` 은 마우스 히트영역 전용이고,
`grep -i "fullscreen|screenshot"` 는 리포지토리 전체에서 히트 0건이다.
**정정**: beatoraja `input/KeyCommand.java:5-17` 을 직접 세면 SHOW_FPS, UPDATE_FOLDER, OPEN_EXPLORER,
**COPY_SONG_MD5_HASH, COPY_SONG_SHA256_HASH**, SWITCH_SCREEN_MODE, SAVE_SCREENSHOT, POST_TWITTER,
ADD_FAVORITE_SONG, ADD_FAVORITE_CHART, AUTOPLAY_FOLDER, OPEN_IR, OPEN_SKIN_CONFIGURATION = **13종**이다.
보고서는 11종이라 적고 COPY_*_HASH 2종을 누락했다(해시 복사는 IR/표 문의에 실사용도가 높아 누락이 아쉬운 항목).

### 17 MusicDecide 없음 — confirmed

`main.rs:478-487` Stage enum + `:490-497` `Loading{Song, Scan}` 로 확인. Decide 스테이지 없음.

### 18 외부 연동 없음 — uncertain

rbms 측 부재는 확인했다(discord/stream/screenshot 코드·의존성 0건, 위 16의 grep과 동일 근거).
다만 beatoraja `external/`·`stream/` 디렉토리와 `PlayerConfig.java:247-258` 은 **시간 제약으로 직접 열지 못했다**.
근거의 절반이 미검증이므로 uncertain. 결론(제품 방향 결정 사항, 우선순위 후순위)에는 영향 없다.

### 19 PROCESS.md stale — confirmed

`docs/PROCESS.md` 머리말 4행에 "위치: `/Users/hyunseokbyun/rbms`" 가 그대로 남아 있고(실제 `/Users/gkn/R-BMS`),
§0(:83)이 "완전 플레이 가능 + beatoraja/IIDX 확장 다수 완성" 이라 기술한다.
§7(:169-178) 한계 목록을 통독한 결과 BGA 비디오·표 fetch 동기는 기재되어 있으나
**코스·컨트롤러·연습모드·곡DB 캐시·난이도/모드 필터·즐겨찾기·볼륨 분리는 전부 미기재**다. 주장 그대로.
단 문서 §"현재 작업" 체크리스트 e 항목에 경로 정정이 예정으로 이미 잡혀 있다(보고서도 이를 언급함).

## 보고서가 놓친 항목 (missed)

| # | 항목 | 근거 |
|---|---|---|
| M1 | **스킨 시스템 패리티** — beatoraja는 LR2/JSON/Lua 3종 스킨 로더 + 스킨 설정 화면(OPEN_SKIN_CONFIGURATION)을 가지지만 rbms는 자체 RON 테마 1종 | beatoraja `skin/SkinLoader.java`, `skin/{json,lr2,lua}/`(디렉토리 실재), `SkinConfig.java`(루트) / rbms `crates/rbms-render/src/{skin.rs,theme.rs}` 만 존재, PROCESS.md:177 "남은 건 F5 결과/메뉴 패널 위치 RON화" |
| M2 | **시스템 사운드 전무** — 곡선택 커서/결정/클리어 효과음 계층이 없다 | beatoraja `SystemSoundManager.java`(루트) / rbms `grep -i "SystemSound"` 히트 0건 |
| M3 | **라이벌 기능 미배선** — IR 트레잇에 `rivals`/`player_profile` 이 이미 있는데 클라이언트에서 호출·표시하지 않는다(정렬 RIVALCOMPARE_* 2종의 전제) | rbms `crates/rbms-ir/src/http.rs:85,89-90` 정의 존재 / `apps/` 호출부 0건, beatoraja `RivalDataAccessor.java`(루트) |
| M4 | **replay.ron 도 비원자 저장** — finding-04가 scores/folders만 지목했으나 리플레이도 같은 결함 | rbms `apps/rbms-player/src/replay.rs:36-44` `fs::write` 직접 |
| M5 | **리플레이 파일 무제한 누적 + GC 없음** — 플레이마다 파일 1개, 삭제 경로 없음 | rbms `app_play.rs:319-336`(매 플레이 `{md5[..8]}-{ms}.ron` 저장), 삭제 코드 grep 0건 |
| M6 | **플레이어 프로필/통계 계층 부재** — beatoraja는 PlayerData/PlayerInformation/PlayDataAccessor로 총 플레이수·램프 분포·표별 달성률을 관리하지만 rbms는 평면 `Vec<ScoreRecord>` 뿐 | beatoraja `PlayerData.java`·`PlayerInformation.java`·`PlayDataAccessor.java`(루트) / rbms `apps/rbms-player/src/scores.rs:30-32` |
| M7 | **런처(설정 GUI) 계층 부재** — beatoraja는 JavaFX 런처로 오디오/입력/IR/폴더/코스 에디터를 제공(코스 에디터 포함) | beatoraja `launcher/{AudioConfigurationView,InputConfigurationView,IRConfigurationView,CourseEditorView}.java` / rbms는 인게임 SETTING_TABS(`main.rs:598~`)만 |
| M8 | **랜덤 코스/랜덤 스테이지 미구현** — finding-05가 고정 코스만 다루고 랜덤 코스는 언급 없음 | beatoraja `select/RandomCourseData.java`·`select/RandomStageData.java` / rbms Course 심볼 0건 |

## 미조사 범위

- beatoraja `external/`·`stream/` 디렉토리 내용(finding-18의 절반) — 파일 미열람.
- beatoraja `play/BMSPlayer.java` 본문(finding-10의 일시정지 처리 실제 구현) — 보고서도 [미확인]으로 표기, 본 검증에서도 미열람.
- rbms `crates/rbms-render/src/select.rs` 전문 — DetailView 필드 존재 여부만 확인, 렌더 경로 전량 통독 아님.
- 성능 수치(스캔 시간·프레임 시간)는 실측하지 않았다. finding-01·02의 severity는 코드 구조 근거만으로 판단했다.
