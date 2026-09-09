# ROADMAP 클라이언트 기능 9종 구현 (2026-05-31)

> `ROADMAP.md` 의 클라이언트 섹션 전체. 탐색→웨이브별 구현→각 웨이브 적대적 멀티에이전트 리뷰→검증. 테스트 **123 통과·0 경고**.

## 진행 방식 (사용자 확정)
- 웨이브 순서대로 끝까지 자동 진행, 각 웨이브 후 `cargo test --workspace` 검증.
- 랭크 밴드 = **레퍼런스 구현/IIDX 9분법**. 그래프/점수비교는 결과+곡선택 **둘 다 + 설정 토글**.
- Wave 3 큰 결정 확인: F5=패널 위치까지 RON(실 구현은 HUD 표면), F4=비주얼 일시정지+시크(확장성), F6=P3만(의존성無), F9=둘 다 레퍼런스 구현 패리티.

## Wave 1 — 무결정·저위험
- **F1 폴더 로딩화면** — `apps/rbms-player/src/main.rs`. `pending_song:Option<usize>` → `enum Loading{Song(usize),Folder(PathBuf)}`. `open_folder_dialog`는 picker 후 `begin_loading(Loading::Folder)`만 하고, 실제 스캔+테이블 fetch(블로킹)는 한 프레임 뒤 `finish_loading`에서 → SCANNING 프레임 선노출. 폴더 descent(in-memory)·뒤로가기는 제외.
- **F9-b 그린넘버 HUD** — `crates/rbms-render/src/hud.rs`(`HudView.green_number`)·`scroll.rs`(`green_number`, lane-cover 반영)·main.rs. 현재 BPM 세그먼트(`binary_search`)로 매 프레임 계산. CONSTANT 모드는 `2000/hispeed*(1-cover)`.
- **F9-c CN/HCN** — `crates/rbms-parser/src/lib.rs`(`#LNMODE` 파싱, 0=undef→LN/1=LN/2=CN/3=HCN), `crates/rbms-chart/src/lib.rs`(`to_model`에서 `LnKind` 매핑, 기존 하드코딩 `LnKind::Ln` 제거). 판정 윈도우는 MVP 동일(타입만 모델 기록).

## Wave 2 — 그래프/점수 (스키마 안전)
- **F2 랭크 그래프** — `crates/rbms-render/src/result.rs`: `dj_rank`(ex/maxex→8밴드)·`RANK_BANDS`·`RANK_BOUNDS`(밴드별 rate폭)·`draw_rank_bar`(통과밴드 bright+마커). 결과화면 RANK 라벨+바, 셀렉트 RECORDS 베스트 랭크 바+행별 랭크. `ResultView`에 `prev_best_ex/prev_ex/show_graph` 확장.
- **F3 점수 비교** — `ex_delta_label`(녹/적/회). 결과: `+Δ BEST`/`+Δ PREV`/`NEW BEST`. 셀렉트: 행별 직전대비 Δ. 모달: 랭크. `enter_result`가 push 전 history로 prev/best 산출.
- **설정 토글** — `SCORE GRAPH`(DISPLAY 탭, idx19, 기본 on). `PlaySettings.score_graph` serde default.

## Wave 3 — 설계 무거움
- **F7-pre early/late** — `crates/rbms-judge/src/matcher.rs`: `early/late:[u32;6]`(`early[i]+late[i]==counts[i]`), `avg_judge_us`. press/release는 dm 부호로 분류, update sweep은 late. 空POOR 제외.
- **F7 IR 슈퍼셋** — `crates/rbms-ir/src/dto.rs`: JudgeBreakdown +epg…lms·avgjudge·empty_poor, PlayOptions +전체옵션, ScoreSubmission +seed/judge_algorithm/rule/skin/client_build_sha256/client_platform, ReplayData→`ReplayEvent{t_us,lane,press}`, 신규 SettingsBlob/AuthRequest/AuthResponse. `lib.rs` trait에 course_ranking/download_replay/get·put_settings/register/login(default=Unsupported), `http.rs` 구현(PUT 헬퍼). 전부 serde default 후방호환(미충족 payload 디코드 테스트 포함).
- **F8 빌드해시** — main.rs `compute_build_hash`(`current_exe`→sha2, 1회 캐시)·`client_platform`(OS-ARCH). `enter_result`가 제출에 채움. `sha2` 의존성 추가.
- **F9-a ALL-SCRATCH/H-RANDOM** — `crates/rbms-chart/src/shuffle.rs`: `NoteOption` +HRandom/AllScratch, `apply_time_based`(per-row 시간기반, anti-jack: 스크래치 40ms·키 125ms 임계, LN 핀). 레퍼런스 구현 `Randomizer.java` 대조(`SRandomizer`/`AllScratchRandomizer`).
- **F9-d 듀얼필드 14K** — `crates/rbms-render/src/skin.rs`: `Skin.fields:Vec<(x0,w)>`, `dual_field`/`dual_gap`(serde default). `player>=2`면 P1좌·P2우 분리, 스크래치 바깥 가장자리(IIDX DP). `playfield.rs` judge라인/구분선/외곽선/lane-cover 필드별, `hud.rs` 게이지는 전체 extent(`max(x+w)`) 1개.
- **F5 데이터 주도 HUD 스킨** — `skin.rs` SkinConfig+Skin에 judge_colors/labels/labels_short·gauge 임계·색·height·combo/judge/fastslow 오프셋(serde default=기존값). `hud.rs`가 const 대신 skin 필드 사용. 기본 스킨 HUD 렌더 동일(회귀 PNG 확인). *(결과/메뉴 위치 RON화는 후속)*
- **F6 폰트** — `font.rs`: P3b 캐시를 `HashMap<u32,HashMap<String,Laid>>`(hit 시 `&str` 무할당), P3c `fit_text`(말줄임, 셀렉트 곡제목 적용). **P3a run-length 병합은 프로파일 결과 ~11%(AA 텍스트 픽셀별 alpha 상이)로 보류** — 본 해법은 글리프 텍스처 아틀라스(P3 범위 밖). P4 웹폰트 스킵.

## 적대적 리뷰 (멀티에이전트)
- Wave 1+2 리뷰(9 에이전트): 실 버그 0, `#LNMODE 1` 명시 매핑 보강. 
- Wave 3 리뷰(45 에이전트, 42 raw→37 confirmed, 33 정상확인): **F4 상태관리 3건 수정** — ① `to_select_or_exit`에서 analysis 상태 미초기화(stale) → 전부 리셋, ② `seek_replay` replay-None 가드, ③ REPLAY ANALYSIS 토글 off 시 진행 중 분석 즉시 해제. ALL-SCRATCH 테스트 문구 정확화 + 고밀도/DP/CJK 테스트 보강.

## 검증
- `cargo test --workspace` 123 통과·0 실패·0 경고.
- 렌더 PNG: 그린넘버 HUD, 결과 랭크 바+델타, 14K 듀얼필드, F5 기본 HUD 회귀.
- F4: 실제 리플레이로 GUI 6초 구동(패닉 없음). 오버레이 시각 확인은 적대적 리뷰로 대체.

## 알려진 후속 (이번 범위 밖)
- F5 결과/메뉴 패널 위치 RON화, F6 P3a 글리프 아틀라스·P4 웹폰트, F9-c CN/HCN 판정 차별화, IR `PlayOptions.lntype`에 실제 LN모드 전파(현재 모델에 lnmode 미보유), 서버 구현 시 IR 신규 엔드포인트 E2E.
