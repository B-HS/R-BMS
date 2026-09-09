# 레퍼런스 구현 판정 외 기능 인벤토리 vs rbms (2026-09-09)

조사 시간 상한 15분. 판정/게이지 수치 대조는 범위 밖(다른 에이전트).
근거는 `파일:라인` 으로 양쪽 인용. 코드로 확인 못 한 것은 `[미확인]`.

- rbms: `/Users/gkn/R-BMS` (총 20,879 LOC Rust)
- 레퍼런스 구현: `<reference>/src/bms/player/레퍼런스 구현` (조사 대상 패키지 합계 33,377 LOC Java)

---

## 1. MainState 별 기능 인벤토리

### 1.1 select/ (MusicSelector)

| 레퍼런스 구현 기능 | 근거 | rbms 상태 | 근거 |
|---|---|---|---|
| Bar 타입 13종 (SongBar/FolderBar/DirectoryBar/TableBar/GradeBar/HashBar/CommandBar/ContainerBar/RandomCourseBar/SameFolderBar/SearchWordBar/ExecutableBar/SelectableBar) | `select/bar/` 디렉토리 목록 | **부분** — 4종만 (`Root`/`AllSongs`/`TableLevels`/`TableLevel`) | main.rs:527-541 `enum SelectView`, `enum SelectItem` |
| 정렬 12종 (TITLE/ARTIST/BPM/LENGTH/LEVEL/CLEAR/SCORE/MISSCOUNT/DURATION/LASTUPDATE/RIVALCOMPARE_CLEAR/RIVALCOMPARE_SCORE) | select/BarSorter.java:14-236, 260 `defaultSorter` 8종 | **부분** — 5종 (DEFAULT/TITLE/ARTIST/LEVEL/CLEAR) | main.rs:543-573 `SortMode` |
| 난이도 필터 `DifficultyFilter`, 모드 필터 `ModeFilter` | select/DifficultyFilter.java, select/ModeFilter.java; PlayerConfig.java:96,100 | **미구현** — 필터 UI/상태 없음 | app_select.rs 전체에 filter 상태 없음(검색만) |
| 검색 (SearchWordBar, `getSongDatasByText`) | select/bar/SearchWordBar.java, song/SQLiteSongDatabaseAccessor.java:385 | **구현(단순)** — `/` 로 전체 라이브러리 문자열 검색 | app_select.rs:24-45 |
| 즐겨찾기 (ADD_FAVORITE_SONG / ADD_FAVORITE_CHART) | input/KeyCommand.java:13-14 | **미구현** | 리포지토리 전체에 favorite 심볼 없음 |
| 랜덤 선택 / 랜덤 코스 (`isRandomSelect`, RandomCourseData/RandomStageData) | PlayerConfig.java:199, select/RandomCourseData.java, select/RandomStageData.java | **미구현** | 없음 |
| 코스(GradeBar, CourseData) | CourseData.java, CourseDataAccessor.java, select/bar/GradeBar.java | **미구현(클라)** — IR DTO 에만 `CourseSubmission`/`course_ranking` 존재, 플레이 경로 없음 | rbms-ir/src/lib.rs:54,60; crates/rbms-play 에 Course 심볼 0건 |
| IR 랭킹 표시(5초 갱신, 10분 재로드, RankingDataCache) | select/MusicSelector.java:76-77, 223-250, 371-374 | **미구현** — 곡선택에 랭킹 패널 없음 | rbms-render/src/select.rs 의 `DetailView` 에 랭킹 필드 없음(select.rs:80-93) |
| 곡 미리듣기 (PreviewMusicProcessor, `SongPreview` 설정) | select/PreviewMusicProcessor.java, MusicSelector.java:173-208 | **구현(상위)** — `#PREVIEW` 파일 + 없으면 autoplay 하이브리드 미리듣기 | main.rs:56-69, app_select.rs:286-540 |
| 커스텀 폴더 (FolderEditorView) | launcher/FolderEditorView.java | **미구현** | tables.rs/folders.rs 는 "라이브러리 루트 목록"·"난이도표 목록"만 |
| 리플레이 슬롯 전환 (RESET/NEXT/PREV_REPLAY) | select/MusicSelectCommand.java:25-48 | **부분** — 슬롯 개념 없음, 로컬 기록 모달에서 1건씩 재생 | app_select.rs:542-587 |
| 같은 폴더 곡 보기 SHOW_SONGS_ON_SAME_FOLDER, IPFS 다운로드 | select/MusicSelectCommand.java:94,129 | **미구현** | — |
| 밀도 그래프(SkinDistributionGraph) | select/SkinDistributionGraph.java | **구현** — `note_density` 포팅 + DensityView | rbms-render/src/select.rs:38-41, main.rs:466 |

### 1.2 decide/ (MusicDecide)

| 기능 | 근거 | rbms |
|---|---|---|
| 곡 결정 연출 스테이지(스킨·사운드 포함 독립 MainState) | decide/MusicDecide.java:16-67 | **미구현** — LOADING 스테이지만 존재 | main.rs:479-501 `Stage`(Select/Settings/KeyConfig/Tables/Folders/Loading/Play/Result) |

### 1.3 play/ (BMSPlayer)

| 기능 | 근거 | rbms 상태 | 근거 |
|---|---|---|---|
| BGA 이미지 처리(BGImageProcessor) | play/bga/BGImageProcessor.java (148L) | **구현(이미지만)** | app_play.rs:108-123 |
| BGA 비디오(mpg/FFmpeg/GdxVideo) | video/FFmpegProcessor.java, video/GdxVideoProcessor.java, play/bga/BGAProcessor.java(405L) | **미구현** | PROCESS.md §7 명시, 코드에 video decode 없음 |
| PracticeConfiguration(연습 모드: 구간 반복·속도·게이지 실시간 변경) | play/PracticeConfiguration.java(전체 클래스), play/SkinPractice.java | **미구현** | rbms 전체에 practice 심볼 0건 |
| hidden/sudden/lift 커버 | play/SkinHidden.java, ControlInputProcessor.java:231-271 | **부분** — LANE COVER(sudden)·LIFT 있음, HIDDEN 별도 없음 | app_select.rs:893-894 (5=LIFT, 6=LANE COVER) |
| 인게임 배속/커버/리프트 조작(ControlInputProcessor) | play/ControlInputProcessor.java:64-121 (모드별 키 배열·아날로그 틱·turbo) | **부분** — 6개 액션(HiSpeed±/Cover±/Lift±) 키 바인딩, 아날로그/turbo 없음 | keyconfig.rs:83-109, app_input.rs:90-105 |
| pomyu chara | play/PomyuCharaProcessor.java | **미구현** | 없음 |
| 커스텀 타이머(RhythmTimerProcessor / TimerManager) | play/RhythmTimerProcessor.java, TimerManager.java | **미구현** | 없음 |
| 리플레이 저장/자동재생 | ReplayData.java, PlayDataAccessor.java, PlayerConfig.java:169 `autosavereplay[]`(슬롯 배열) | **구현(1슬롯 자동)** — AUTO REPLAY 저장 + 재생, 슬롯 개념 없음 | app_play.rs:318-338, replay.rs:17-37 |
| 일시정지/빠른 리트라이/exitPressDuration | PlayerConfig.java:184 `exitPressDuration`, BMSPlayer.java `[미확인 — 시간상 본문 미독]` | **미구현** — Esc 는 즉시 이탈(전 노트 판정 완료면 결과로) | main.rs:1107-1117 |
| 리플레이 분석(재생속도·시크·일시정지) | 레퍼런스 구현 대응 기능 `[미확인]` | **rbms 고유 확장** — Space/+/-/PageUp·Down | app_input.rs:294-317 |
| autoplay | pattern/AutoplayModifier.java | **구현** | main.rs:1127 `if !self.autoplay` |

### 1.4 result/

| 기능 | 근거 | rbms 상태 | 근거 |
|---|---|---|---|
| MusicResult: 상세 판정 분포·게이지 그래프·IR 전송·랭킹 | result/MusicResult.java, result/SkinGaugeGraphObject.java | **부분** — 카운트 6종·EX·MAX콤보·fast/slow·랭크바·이전 베스트 비교, 게이지 그래프/랭킹 패널 없음 | rbms-render/src/result.rs:5-28 |
| 리플레이 4슬롯 저장 UI | PlayerConfig.java:169 `autosavereplay[]` 배열 | **미구현** — 단일 자동 저장 | app_play.rs:318-338 |
| target 비교(TargetProperty, `targetid="MAX"`) | play/TargetProperty.java, PlayerConfig.java:72 | **부분** — 이전 베스트 EX 델타만 | result.rs:18-20 |
| CourseResult | result/CourseResult.java, CourseResultSkin.java | **미구현** | 코스 자체 미구현 |

### 1.5 pattern/ (노트 옵션)

| 레퍼런스 구현 Random 옵션 | 근거 | rbms |
|---|---|---|
| IDENTITY/MIRROR/RANDOM/ROTATE/S_RANDOM/SPIRAL/H_RANDOM/ALL_SCR + *_EX 4종 (총 12) | pattern/Random.java:6-17 | rbms `NoteOption` — PROCESS.md §7 에 "ALL-SCRATCH/H-RANDOM 해결" 기재. 정확한 세트는 rbms-chart/src/shuffle.rs `NoteOption::ALL` (main.rs:918-921 에서 순회) `[전수 대조 미실시]` |
| FLIP / BATTLE (2P 필드) | pattern/PatternModifier.java:118-121 | **미구현 추정** — 14K 듀얼필드는 있으나 FLIP/BATTLE 옵션 없음 `[미확인]` |
| ExtraNoteModifier / MineNoteModifier / LongNoteModifier / ScrollSpeedModifier / ModeModifier / PracticeModifier | pattern/*.java 파일 목록; PlayerConfig.java:113-144 (scrollMode/longnoteMode/mineMode/extranoteType) | **전부 미구현** | settings.rs:11-38 에 대응 필드 없음 |
| PatternModifyLog(옵션 적용 로그 → 리플레이 재현) | pattern/PatternModifyLog.java | **다른 방식** — seed 저장으로 재현 | replay.rs:21-22 (`random`, `seed`) |

### 1.6 input/

| 장치 | 근거(레퍼런스 구현) | rbms |
|---|---|---|
| 키보드 | input/KeyBoardInputProcesseor.java | **구현** | app_input.rs (winit KeyCode) |
| 컨트롤러/게임패드 | input/BMControllerInputProcessor.java, launcher/ControllerConfigViewModel.java | **미구현** — 워크스페이스에 gilrs/gamepad 의존성 0건 | `grep -rn "gilrs\|gamepad\|joystick" crates apps Cargo.toml` → 히트 0 |
| MIDI | input/MidiInputProcessor.java | **미구현** — 유일한 midi 문자열은 IR DTO 테스트 리터럴 | rbms-ir/src/dto.rs:493 |
| 아날로그 스크래치(마우스) | input/MouseScratchInput.java | **미구현** | — |
| KeyCommand 11종(FPS/폴더갱신/탐색기열기/전체화면/스크린샷/트위터/즐겨찾기2/폴더오토/IR열기/스킨설정) | input/KeyCommand.java:5-17 | **거의 미구현** — 대응 기능 없음 | main.rs:576-591 `Hot` 목록 |

### 1.7 audio/

| 기능 | 근거 | rbms |
|---|---|---|
| 드라이버 선택(OpenAL/PortAudio/GdxSound 등 DriverType + deviceBufferSize/simultaneousSources/sampleRate) | AudioConfig.java:15-32 | **미구현(고정)** — cpal 단일, 설정 노출 없음 | settings.rs:11-38 |
| 볼륨 3분리(system/key/bg) | AudioConfig.java:46-54 | **미구현** — 마스터 게인만, 사용자 설정 없음 | rbms-audio/src/mixer.rs:50,61,77 `master_gain`; settings.rs 에 volume 필드 없음 |
| freqOption / fastForward (pitch vs freq, TimeStretchProcessor) | AudioConfig.java:37-41, audio/TimeStretchProcessor.java | **미구현** | mixer 는 pitch 파라미터를 받으나(mixer.rs:42) 설정 노출 없음 |
| 결과음 루프 옵션 | AudioConfig.java:59-64 | **미구현** | SystemSoundManager 대응물 자체 없음 |

### 1.8 ir/

| 레퍼런스 구현 IRConnection 메서드 | 근거 | rbms ScoreServer |
|---|---|---|
| register/login/getRivals/getTableDatas/getPlayData/getCoursePlayData/sendPlayData/sendCoursePlayData/getSongURL/getCourseURL/getPlayerURL/getVersionInfo/getIllegalSongs | ir/IRConnection.java:17-128 | **DTO·trait 은 슈퍼셋으로 존재하나 서버 미구현** → `register/login/get_settings/put_settings/download_replay/course_ranking` 은 기본 `Unsupported` | rbms-ir/src/lib.rs:46-84 |
| RankingDataCache(곡선택 랭킹 캐시) | ir/RankingDataCache.java | **미구현** | — |
| IRConnectionManager(복수 IR 등록), IRWorker(별도 프로세스 격리) | ir/IRConnectionManager.java, IRWorkerMain.java, IsolatedIRConnection.java | **미구현** — 단일 서버 URL | settings.rs:36 `server_url: Option<String>` |

### 1.9 song/ · Score DB · Table

| 기능 | 근거 | rbms |
|---|---|---|
| SQLite 곡DB + 증분 갱신(디렉토리 lastModified 비교, 배치 업데이터, 텍스트 검색 SQL) | song/SQLiteSongDatabaseAccessor.java:461-701, 1052-1056(`BMSFolder(path, bmsroot, lastModifiedTime)`), :385 `getSongDatasByText` | **미구현** — DB 없음, 매 실행 전량 재귀 스캔 | main.rs:398-443 `scan_folders`/`scan_folder` |
| SongInformation(밀도·노트수 등 사전계산 캐시) | song/SongInformationAccessor.java | **부분** — 포커스 곡만 lazy 계산, 캐시 없음 | main.rs:450-470 `compute_chart_detail`, main.rs:380-390 주석 |
| ScoreDatabase / ScoreLog / Rival | ScoreDatabaseAccessor.java, ScoreDataLogDatabaseAccessor.java, RivalDataAccessor.java | **부분** — 단일 RON `scores.ron`, ScoreLog·Rival 없음 | scores.rs:29-63 |
| SongReview | song/SongReviewAccessor.java | **미구현** | — |
| TableData(난이도표 캐시·코스 정의) | TableData.java, TableDataAccessor.java | **부분** — 표 fetch/캐시·md5 매칭 구현, **코스 정의 미지원** | rbms-table/src/lib.rs:41-96 (Course 심볼 0건) |

### 1.10 launcher/ (설정 GUI)

레퍼런스 구현 는 JavaFX 런처에 9개 탭(Audio/Input/IR/MusicSelect/Play/Resource/Skin/Video/Stream) + 3개 에디터(Course/Folder/Table) + SongDataView 를 갖는다 (launcher/*.java 파일 목록).

rbms 는 인게임 설정 6탭(PLAY/GAUGE/JUDGE/DISPLAY/INPUT/NETWORK), 총 24행 (main.rs:598-606 `SETTING_TABS`, app_select.rs:888-916 `setting_line`).

| 레퍼런스 구현 탭 | rbms 대응 |
|---|---|
| AudioConfiguration | **없음** (드라이버·버퍼·볼륨 3분리·freq 전부 미노출) |
| InputConfiguration + ControllerConfig | **부분** — KEY CONFIG 화면(키보드 전용), 컨트롤러 없음 (keyconfig.rs) |
| SkinConfiguration + SkinPreview | **부분** — SKIN 행에서 RON 파일 선택만 (app_select.rs:903) |
| IRConfiguration | **부분** — SERVER URL / PLAYER ID 2행 (app_select.rs:911-912) |
| PlayConfiguration | **부분** — PLAY/GAUGE/JUDGE 탭 |
| ResourceConfiguration(BMS 루트 경로) | **구현** — FOLDERS 화면 (folders.rs, app_select.rs:658-733) |
| VideoConfiguration | **없음** |
| StreamConfiguration + stream/command | **없음** |
| CourseEditor / FolderEditor / TableEditor | **부분** — TABLES 추가/삭제만 (tables.rs) |
| SongDataView | **없음** |

### 1.11 stream/ · external/

| 기능 | 근거 | rbms |
|---|---|---|
| 스트리밍 연동(StreamController, StreamCommand/StreamRequestCommand, 리퀘스트 큐 `enableRequest`/`maxRequestCount`) | stream/, PlayerConfig.java:256-258 | **미구현** |
| Discord Rich Presence | external/discord/DiscordListener.java | **미구현** |
| 스크린샷 저장/트위터 업로드 | external/ScreenShotFileExporter.java, ScreenShotTwitterExporter.java; PlayerConfig.java:247-253 | **미구현** |
| BMS 검색(BMSSearchAccessor), 스코어 임포터(ScoreDataImporter) | external/*.java | **미구현** |

---

## 2. 데이터 계층 스케일 분석

### 2.1 곡 스캔

레퍼런스 구현: SQLite `songdata`/`folder` 테이블 + 폴더 `lastModifiedTime` 비교 증분 갱신(`updateSongDatas(path, bmsroot, updateAll, info)`, song/SQLiteSongDatabaseAccessor.java:461; `BMSFolder(Path, String[], long lastModifiedTime)` :1056). 검색은 SQL(`getSongDatasByText` :385).

rbms: 캐시 없음. 매 실행 시(그리고 폴더 목록 변경마다) 전 라이브러리를 재귀 스캔하며 **모든 차트 파일을 read + 전체 파싱**한다.

- main.rs:406-443 `scan_folder` — `std::fs::read(&p)` → `rbms_parser::parse(&bytes)` → `rbms_chart::detect_mode`. 단일 스레드 DFS(`stack`), 병렬화 없음.
- main.rs:398-404 `scan_folders` — 폴더별 순차 `extend`.
- 실행 지점: 최초 부팅 main.rs:797, 폴더 변경 시 app_select.rs:695-707.
- `rbms_parser::parse` 는 원시 바이트 MD5 + SHA-256 을 계산한다(PROCESS.md §1 "차트 해시 = raw 바이트 MD5+SHA-256"), 즉 곡당 해시 2회 + 렉싱 1회.

추정(코드 구조 기반, 실측 아님 `[미확인]`): 곡당 read+parse+2해시가 1~5 ms 라면 **10,000 차트 = 10~50 초 단일 스레드**, 30,000 차트면 30~150 초. 레퍼런스 구현 는 2회차부터 증분이므로 사실상 0초. 이것이 대규모 라이브러리에서의 가장 큰 체감 격차다.

메모리: `SongEntry`(main.rs:358-379)는 곡당 String 9개 + PathBuf. 10k 곡이면 수 MB~십수 MB 수준으로 문제 없음. 문제는 시간과 I/O.

### 2.2 곡선택 화면 O(n×m)

- `select_key` 에 `self.sel` 이 포함된다 → **커서를 한 칸 움직일 때마다 전체 장면 재구축**. app_select.rs:78-80.
- `build_select_view` 는 **현재 목록의 모든 행**에 대해 `title.clone()` + `level.clone()` + `self.scores.best_clear_for_md5(&e.md5)` 를 수행한다. app_select.rs:99-127.
- `best_clear_for_md5` 는 전체 레코드 선형 스캔 + `eq_ignore_ascii_case`. scores.rs:82-84.

→ ALL SONGS(또는 검색 결과)가 N 곡이고 로컬 기록이 M 건이면 **커서 이동 1회당 O(N×M) 문자열 비교 + N개 String 할당**. N=10,000 / M=10,000 이면 1억 회 비교가 키 반복 입력마다 발생 → 곡선택 프리징. 레퍼런스 구현 는 `ScoreDataCache`(select/ScoreDataCache.java)로 해시→스코어 맵을 캐싱한다.

### 2.3 스코어 저장 (`scores.ron`)

scores.rs:29-63.

| 위험 | 근거 |
|---|---|
| 전체 재직렬화 | 플레이 1회마다 전 레코드를 `to_string_pretty` 후 `fs::write` — O(M) 쓰기. 수만 건이면 수 MB 를 매 플레이마다 재작성 |
| **원자성 없음** | `std::fs::write` 직접 호출(temp+rename 아님) — 저장 중 크래시/전원차단 시 파일 절단 → 전체 기록 손실 (`.bak` 는 *다음 파싱 실패 시*에만 만들어지므로 이미 손상된 파일을 백업) scores.rs:56-63 |
| 동시성 없음 | 파일 락 없음. 인스턴스 2개 동시 실행 시 나중 저장이 앞 기록을 덮어씀 |
| 조회 선형 | `for_md5`/`best_ex_for_md5`/`best_clear_for_md5` 전부 O(M) 전수 스캔 (scores.rs:70-84) |
| ScoreLog 없음 | 레퍼런스 구현 는 별도 `ScoreDataLogDatabaseAccessor` 로 시계열 로그를 둔다 |

`folders.ron`/`tables.ron`/`settings.ron` 도 같은 비원자 write 패턴(folders.rs:28-42).

---

## 3. 우선순위 갭 (유저 체감 순)

| # | 갭 | 효력 | 근거 |
|---|---|---|---|
| 1 | 곡DB(SQLite 등) + 증분 스캔 — 대규모 라이브러리 시작 시간 | L | main.rs:398-443 vs SQLiteSongDatabaseAccessor.java:461-701 |
| 2 | 곡선택 O(N×M) 재구축 — 커서 이동 프리징 | M | app_select.rs:78-127, scores.rs:82 |
| 3 | 컨트롤러(게임패드) 입력 — IIDX 유저 필수 | L | 워크스페이스 gamepad 의존성 0건 vs input/BMControllerInputProcessor.java |
| 4 | scores.ron 원자적 저장(temp+rename) — 데이터 손실 방지 | S | scores.rs:56-63 |
| 5 | 코스(단급) 플레이 + CourseResult | L | CourseData.java, result/CourseResult.java vs 클라 Course 심볼 0건 |
| 6 | 즐겨찾기 / 난이도·모드 필터 / 정렬 12종 | M | KeyCommand.java:13-14, BarSorter.java:260 vs main.rs:543 |
| 7 | 볼륨 3분리(system/key/bg) + 오디오 설정 노출 | S | AudioConfig.java:46-54 vs settings.rs |
| 8 | 곡선택 IR 랭킹 패널 + 랭킹 캐시 | M | MusicSelector.java:223-250 |
| 9 | 연습 모드(PracticeConfiguration) | L | play/PracticeConfiguration.java |
| 10 | 일시정지 / 빠른 리트라이 | S | PlayerConfig.java:184 vs main.rs:1107 |
| 11 | 리플레이 다중 슬롯 | M | PlayerConfig.java:169 |
| 12 | 패턴 옵션 확장(ExtraNote/Mine/LongNote/Scroll/FLIP/BATTLE) | L | pattern/*.java vs settings.rs |
| 13 | BGA 비디오(mpg) | L | video/FFmpegProcessor.java, PROCESS.md §7 |
| 14 | 결과 게이지 그래프 / target 비교 | M | result/SkinGaugeGraphObject.java, play/TargetProperty.java |
| 15 | 스크린샷 / Discord / 스트림 연동 | M | external/, stream/ |
| 16 | MusicDecide 스테이지 | S | decide/MusicDecide.java |

---

## 4. 문서 vs 코드 (stale)

- PROCESS.md 머리말 위치가 `/Users/hyunseokbyun/rbms` — 실제는 `/Users/gkn/R-BMS`. (PROCESS.md 이미 §e 체크리스트에 정정 예정으로 기재)
- PROCESS.md §7 "난이도표 추가 fetch는 동기(1개씩)" — 코드와 일치(rbms-table/src/lib.rs:50 reqwest blocking).
- PROCESS.md §4/§7 은 "완전 플레이 가능"으로 기술하나, **select/result 의 레퍼런스 구현 기능 커버리지는 위 표대로 절반 이하**다. 문서에 인벤토리 격차가 명시돼 있지 않다.

---

## 5. 미조사 범위

- `play/BMSPlayer.java` 본문 미독 → 일시정지/리트라이/exitPressDuration 실제 동작, PomyuChara·RhythmTimer 세부.
- `rbms-chart/src/shuffle.rs` 의 `NoteOption::ALL` 전수 ↔ `pattern/Random.java` 12종 1:1 대조 미실시.
- skin/ 패키지(LR2/JSON/Lua 스킨 로더) — 다른 관점 에이전트 소관으로 판단해 제외.
- `select/bar/*.java` 13종 개별 본문 미독(파일 목록 기준 분류).
- 실측 벤치(스캔 시간·메모리) 미실시 — 본 보고서의 시간 추정치는 코드 구조 기반 추정.
