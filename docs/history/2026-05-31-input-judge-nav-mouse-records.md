# 2026-05-31 (세션 B) — 입력판정 근본수정·폴더 네비·마우스·로컬 기록

사용자 요청 4건 + 추가 2건. 모두 레퍼런스 구현 원본 대조 후 구현, `cargo test --workspace` 99 통과.

## 1. 입력 판정 버그 근본수정 — 空POOR(empty poor)

### 증상
"키가 많이 눌리면 하나가 씹힌다 / 분명 제대로 눌렀는데 POOR로 뜬다."

### 원인 (원본 대조)
`crates/rbms-judge/src/matcher.rs` `press()`가 **이른 빈 POOR**(MS 윈도우에만 걸치고 BAD 바깥, 7K NOTE 기준 +220~500ms 빠른 누름)를 일반 `Judge::Poor`로 처리하여:
- `note.judged = true` 로 **노트를 소실**시키고,
- `apply(Poor)` 로 **콤보를 끊었다**.

the reference implementation `play/JudgeManager.java` + `play/JudgeProperty.java`(`SEVENKEYS`):
- `judgeVanish = {PG,GR,GD,BD,PR,MS} = {t,t,t,t,t,false}` → 空POOR(judge 5 = MS)는 **노트 미소실**.
- `combo = {t,t,t,f,f,true}` → 空POOR는 **콤보 미차단**.
- score MS 카운트 +1, 게이지 MS 페널티만 적용.

즉 잭/약간 빠른 누름에서 레퍼런스 구현는 빈 POOR 플래시만 띄우고 노트는 살아있어 재타 가능 → 우리 포팅은 노트를 먹고 콤보를 끊어 "씹힘/억울한 POOR"이 발생.

### 수정
`press()`에서 분류 결과가 `Judge::Poor`(=空POOR)일 때:
- 노트 미소실(`judged` 유지)·콤보 미차단,
- `gauge.update(Judge::Miss)`(MS 페널티, Normal −2% < 기존 Poor −6%),
- `empty_poor` 카운터 +1, `last_judge = Poor`(POOR 플래시),
- 노트는 그대로 남아 추후 정타 시 정상 판정.

`JudgeEngine`에 `pub empty_poor: u32` 추가. 결과 로그·기록 모달에 노출.

테스트(`crates/rbms-judge/src/lib.rs`): `early_empty_poor_keeps_note_hittable_and_combo`·`empty_poor_does_not_block_full_combo`·`empty_poor_drains_gauge` + 기존 회귀.

## 2. 폴더 네비 ←→ (`apps/rbms-player/src/main.rs`)
Select: `ArrowLeft = select_back`(상위 폴더), `ArrowRight = select_enter`(폴더 진입/곡 시작). 기존 ↑↓·Enter·Esc·Tab·O·T 유지.

## 3. 마우스 이벤트
- `WindowEvent::CursorMoved` → physical 좌표를 고정 논리 1280×720으로 스케일(`inner_size` 비율).
- `WindowEvent::MouseInput(Left,Pressed)` → `handle_click`.
- 즉시모드 `hot: Vec<(Rect, Hot)>`: 매 프레임 렌더 시 클릭영역 적재, topmost-first 히트테스트.
- Select 행: **1클릭 선택, 재클릭(포커스된 행) 열기**(사용자 선택). 우측 기록 행 클릭 → 상세 모달.
- Settings: 탭 클릭 → 전환, 값 행 클릭 → `adjust_setting(+1)`(KEY CONFIG은 진입).

## 4. 로컬 기록(scores) 영속 + 우측 인라인 리스트 + 상세 모달
- `apps/rbms-player/src/scores.rs`: `ScoreRecord`/`ScoreBook`(RON, `~/.config/rbms/scores.ron`). `for_md5`(최신순). counts는 `[u32;6]`라 **RON 직렬화 시 튜플 `(...)`**(주의: 손으로 픽스처 작성 시 `[...]` 불가).
- `enter_result`: 실인터랙티브 플레이마다 기록 저장(**서버 유무 무관**, autoplay/replay 제외). 리플레이 저장 시 파일명 연결.
- 우측 패널(곡 포커스): best 램프 요약(램프색=레퍼런스 구현) + EX + 최근 기록 리스트(날짜/램프/EX/BP, 클릭 가능).
- 상세 모달(`record_modal`): 기록 클릭 또는 `R`. 판정내역·EX·콤보·BP·EMPTY POOR·게이지·날짜·옵션. ↑↓ 이전/다음, Enter/PLAY REPLAY 버튼으로 해당 리플레이 재생, Esc/CLOSE/바깥클릭 닫기.
- 리플레이 재생 안전: `load()`의 player autoplay 플래그를 `self.autoplay && self.replay.is_none()`로 바꿔 리플레이 중 이중 입력(autoplay+feed_replay) 방지.

## 5. 추가
- **클리어 램프 색상 = 레퍼런스 구현 공식**(`select/SkinDistributionGraph.LAMP`, ARGB→RGB). `clear_label_color` 교체 + `clear_type_id`/`clear_type_from_id`.
- **AUTO REPLAY 토글**(설정 PLAY 탭, 기본 ON). `PlaySettings.auto_replay`. 리플레이 자동저장 게이트.

## 6. 편의기능 (후속 요청)

### 6.1 Play에서 Esc → 즉시 result (남은 노트 없을 때)
`window_event` Play `Esc`: `player.judge.total_judged() >= total_notes()`(모든 노트가 hit/miss로 해결됨 = 더이상 칠 노트 없음)이면 `enter_result()`로 직행. 아니면 기존대로 `to_select_or_exit()`(중도 포기 → 셀렉트). 곡 종료 후 아웃트로(자동 result 트리거 `last_timeline+2s`) 대기를 스킵한다. 빈 POOR는 `empty_poor`로 분리돼 `total_judged`를 부풀리지 않으므로 판정 기준이 정확.

### 6.2 DEBUG MODE (설정 DISPLAY 탭)
- `PlaySettings.debug`(기본 false)+`PlayerConfig.debug`, setting idx 17, DISPLAY 탭. `#[serde(default)]`라 구버전 settings.ron도 무리없이 로드.
- 의존성: `memory-stats = "1.2"`(크로스플랫폼 RSS, 추측 FFI 회피).
- `App` 메트릭 필드: `last_frame:Instant`·`fps:f32`(EMA)·`frame_count:u64`·`ram_mb:f32`. `frame()` 진입 시 dt로 FPS 갱신, RAM은 15프레임마다 `memory_stats()` 갱신.
- 좌상단 반투명 오버레이(디버그 ON): `FPS(프레임ms)`·`RAM MB`·`QUADS`(인스턴스 수, `Gpu::quad_count`)·`STAGE`. Play 시 `TIME/last`·`NOTES judged/total`·`COMBO/MAX`·`EX/GAUGE`·`FAST/SLOW/EPOOR`·`HISPEED/OFFSET`·`AUDIO클럭/ANCHOR`. Select 시 `SEL`·`SCORES/SONGS`·`CURSOR`.

## 검증
`cargo test --workspace` 99/0. debug·release 빌드 클린(경고 0). 합성 `scores.ron` 실로더 파싱 OK·앱 기동 무패닉. DEBUG MODE ON 상태 기동 시 `memory_stats()` 런타임 호출·오버레이 렌더 무패닉 확인. GUI 스크린샷은 macOS 접근성/화면녹화 권한으로 자동화 불가 → 사용자 환경 육안 확인 필요.
