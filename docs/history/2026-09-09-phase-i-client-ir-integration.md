# Phase I — 클라이언트 IR 완전 통합 (2026-09-09)

> `docs/PROCESS.md`의 "멈추라 할 때까지 멈추지 말고 계획대로 전부 구현" 지시 아래 진행된 작업. GUI(플레이어)에서 IR 로그인·설정 동기화·랭킹 조회·리플레이 다운로드·제출까지 전부 가능하도록, `rbms-ir`를 배포된 IR 서버 계약의 완전한 클라이언트로 확장하고 `rbms-player`에 그 GUI 흐름을 붙였다. 웹 쪽은 `/guide`와 README를 CLI 기준에서 GUI 절차 기준으로 재작성했다.

## 목적

- `rbms-ir`가 서버 계약(웹 `dto/*.ts` + `route/*.ts` + 서비스 계층)의 부분 클라이언트에 머물러 있어, 계정·라이벌·리플레이·랭킹 페이지네이션 등 다수 엔드포인트가 미구현이었다.
- 플레이어 GUI에는 IR 진입점이 전혀 없어(CLI 플래그로만 서버/플레이어 지정 가능), 로그인·랭킹 확인·리플레이 재생이 불가능했다.
- 웹 `/guide`가 CLI 절차만 안내해 실제 GUI 사용자와 어긋나 있었다.

## 구현

### rbms-ir

- `SubmitResponse`를 서버 슈퍼셋으로 확장: `ranked`/`flags`/`is_new_best`/`score_id`/`extra`를 전부 `#[serde(default)]`로 추가해 구버전 서버 응답도 그대로 디코드. 서버 `SCORE_FLAG` 문자열을 미러링한 `ScoreFlag` 타입 + `unranked_reasons()`/`unknown_flags()`/`unranked_summary()` 접근자, `UNKNOWN_BUILD`만 랭킹을 막지 않는 서버 정책 반영.
- 미구현 엔드포인트 구현 + 신규 트레이트 메서드: `whoami`(GET /auth/me), `put_rivals`(PUT /players/{id}/rivals), `player_scores`, `chart_replays`, `version`, `chart_ranking_page`/`course_ranking_page`(page·lnmode·rival_of). settings PUT은 204 성공 + 409 낙관적 잠금 충돌을 전용 변형 `IrError::SettingsConflict`(서버 사본 포함)로 표면화.
- 토큰: `with_token(Option<String>)`(consuming) + `token()`/`base_url()` 접근자. 토큰이 있으면 모든 호출에 Bearer 첨부.
- 에러 매핑: 401/403/404/409/413/429 전용 변형 + 그 외 `Server(status, body)`. 서버 실패 봉투를 파싱해 사람이 읽을 수 있는 Display로 렌더(200자 절단). `status()`/`detail()`/`error_code()`/`is_auth_failure()`/`is_retryable()`.
- 게스트 신원 상수화: `GUEST_PLAYER_ID = "guest"`(단일 출처), `ir_session::submission_player_id(session, configured)`로 제출 신원을 세션에서 파생시켜 토큰 없는 런은 항상 guest로 감.
- `ScoreSubmission`에 `passnotes`(레퍼런스 구현의 미타격 노트 수와 정확히 대응) 추가, `clamp_to_server_bounds()`로 `played_at`/`gauge_value`를 서버 zod 범위에 맞춰 클램프(`CourseSubmission` 동일).
- `ReplayMeta.url`이 배포 오리진 기준(`/api` 포함)임을 문서화 + `download_id()` 헬퍼로 이중 프리픽스 함정 방지.
- worker: `spawn_submit(server, SubmitJob{submission, replay})` — 제출 성공 + 랭킹 차단 flag 없음일 때만 리플레이 업로드, `SubmitOutcome{submit, replay}`을 채널로 전달. 제출 성공/업로드 실패가 서로를 가리지 않음.
- 파일 분할: `dto.rs`를 `dto/{chart,score,account,system,replay,settings,query}.rs`로(공개 경로 불변), `IrError`를 `error.rs`로, mock HTTP 하네스를 `mock_http.rs`로, 신설 테스트가 커진 `dto/score.rs`(607줄)는 `dto/score/tests.rs`로 재분리해 269줄로.

### rbms-player

- IR 관련 로직 전부를 신규 모듈로 분리(`app_network`·`app_ranking`·`app_library`·`ir_map`·`ir_outcome`·`ir_panel`·`ir_ranking`·`ir_ranking_view`·`ir_replay`·`ir_session`·`ir_sync`·`settings_view`·`main_tests`, 최대 477줄). `main.rs`는 상태 필드·라우팅만 증가.
- 모든 네트워크 호출은 백그라운드 스레드(`rbms_ir::spawn_query`/`spawn_submit`)에서 실행하고, 매 프레임 최상단 `App::poll_network()` + `App::poll_ir_jobs()`가 채널을 배수(drain)한다. 렌더 스레드에서 블로킹 IR 호출은 없음.
- 비밀번호는 `App::password`(휘발성 String)에만 존재, LOGIN/REGISTER가 소비 즉시 비움. 저장·로그·렌더 어디에도 평문 노출 없음(행 표시는 최대 12개 `*`).
- 설정 동기화는 업로드 전 `ir_token`/`ir_login_id`/`ir_email`/`player_id`/`server_url`/`songs_folder`/`font_path`를 지운 사본을 올리고 다운로드 시 로컬 값을 보존 — 토큰이 서버 블롭에 들어가지 않고, 다운로드된 사본이 세션을 하이재킹하거나 로컬 경로를 외부로 돌릴 수 없음.
- 설정 화면이 스크롤됨(10행 노출), NETWORK 탭 13행. 렌더는 `settings_view::render_settings`로 분리(헤드리스 테스트 가능).
- 랭킹 패널 토글 키 `I`(기존 미사용 키), 포커스 시 Up/Down/Enter/Esc/**ArrowRight**(리뷰 반영)를 패널이 가져감.

### web · README

- `/guide`를 CLI 기준에서 GUI 절차 기준으로 재작성: 서버 연결 → 계정 → 랭킹 반영 조건(결정 12: SKIPPED/UNRANKED 구분) → 곡선택 IR RANKING 패널 → 리플레이 → 설정 동기화 → 라이벌, 이후 고급 절(웹 `/settings` API 토큰, `--server`/`--player`는 1회성 오버라이드).
- 콘텐츠는 `web/src/widgets/guide/guide-sections.ts`, 렌더는 기존 `guide-article.tsx` 디자인 토큰 그대로.
- README Features의 IR 항목을 GUI 기능 요약으로 교체, "## Web"에 GUI 연결 절차 문단 추가, CLI 플래그는 오버라이드라는 문장으로 강등.

## UI 흐름 (ui_spec)

### 1. 설정 화면 (곡선택에서 Tab)

"SETTINGS" 타이틀, 힌트 "TAB SWITCH   UP DOWN MOVE   LEFT RIGHT CHANGE   ENTER OPEN   ESC SAVE/BACK". 탭 순서 PLAY | GAUGE | JUDGE | DISPLAY | INPUT | NETWORK.

행 목록이 스크롤됨(10행 노출, 선택 중앙 유지, 10행 초과 탭에는 "N / total" 카운터). 패널 좌하단에 상태 라인(강조색, 최근 네트워크 결과/오류).

### 2. NETWORK 탭 (13행)

1. **SERVER URL** — 값: URL 또는 "(none)". Enter/Right로 인라인 에디터, Enter 커밋(빈 값=오프라인), Esc 취소. 커밋 시 스코어 서버 재생성.
2. **PLAYER ID** — 값: id(기본 "guest"). Enter/Right 편집, 빈 값→"guest".
3. **ACCOUNT** — "guest" / "logging in..." / "registering..." / "logged in as \<id\>" / "login failed: \<err\>" / "register failed: \<err\>" / "session expired: \<err\>". Enter로 상태 라인에 복사.
4. **EMAIL** — 값: 주소 또는 "(none)". REGISTER 전용.
5. **PASSWORD** — "(not set)" 또는 "*"×길이(최대 12). 마스킹 에디터(입력 문자는 항상 "*"+caret "_"), Enter 커밋(메모리만), Esc 폐기. 저장·출력·렌더 어디에도 평문 없음, LOGIN/REGISTER 소비 즉시 비움.
6. **LOGIN** — 성공 시 토큰·로그인id 저장, PLAYER ID를 서버 반환 id로, 스코어 서버 토큰 재생성, 라이벌 목록 서버 갱신. 실패 시 기존 세션 유지.
7. **REGISTER** — PLAYER ID를 login id로, EMAIL/PASSWORD/표시명(PLAYER ID)로 가입. 성공 시 LOGIN과 동일 후속.
8. **LOGOUT** — 토큰·로그인id·비밀번호 클리어, 무자격 서버 재생성, 상태 "logged out". PLAYER ID는 guest로 복귀(재로그인 시 계정 id 재입력 필요 — 제출 안전성은 `ir_session::submission_player_id`가 별도 보장).
9. **SYNC SETTINGS** — ON/OFF(기본 OFF).
10. **UPLOAD SETTINGS NOW** — 플레이 설정+키설정을 RON 블롭 "player"로 낙관적 잠금과 함께 업로드. 서버 URL+로그인 필요.
11. **DOWNLOAD SETTINGS NOW** — 같은 블롭을 받아 적용, 토큰/로그인id/이메일/PLAYER ID/SERVER URL/songs폴더/font경로는 로컬 유지. 키설정 파일 재작성 + 라이브 바인딩 리로드.
12. **AUTO UPLOAD REPLAY** — ON/OFF(기본 ON).
13. **RIVALS** — 값: 라이벌 수. Enter/Right로 인라인 목록 오픈.

### 3. 인라인 라이벌 목록

"RIVALS", 힌트 "UP DOWN MOVE   ENTER ADD   D REMOVE   ESC SAVE/CLOSE". 마지막 행 "+ ADD RIVAL (PLAYER ID)"(녹색). 9행 노출·중앙 스크롤. D로 제거(추가행 제외), Esc로 저장+닫기+로그인 시 PUT /players/{id}/rivals. LOGIN/REGISTER 성공 후, 그리고 저장된 토큰이 있으면 시작 시 1회 서버에서 갱신.

### 4. 곡선택 IR RANKING 패널

토글 키 "I"(우측 상세 컬럼 위 오버레이, x 632..1248, y 60..660). 타이틀 "IR RANKING". 포커스 힌트 "UP DOWN MOVE   ENTER REPLAY   I CLOSE".

포커스 차트가 150ms(Phase A `FOCUS_DETAIL_DEBOUNCE`) 안정되면 chart_ranking(limit 10)+player_best(자신)+player_best(라이벌, 최대 8)+chart_replays(limit 10)를 한 번에 가져와 md5별 64엔트리 LRU에 캐시. 세대 불일치 결과는 폐기.

메시지: "IR OFF - SET SERVER URL IN SETTINGS" / "SELECT A CHART" / "LOADING..." / "ERROR: \<err\>" / "NO SCORES YET".

행 구성: 순위, 플레이어명(빈 값은 id로 폴백), EX(우측정렬), 클리어램프(색상 라벨), 꼬리 "F\<fast\>/S\<slow\>"(있으면)+"REP"(리플레이 존재 시). 이후 "YOU" 행, 라이벌별 "RIVAL \<id\>" 행. 17행 노출·중앙 스크롤. "REP" 행에서 Enter/ArrowRight/클릭 시 리플레이 다운로드 후 재생(패널 닫힘). 리플레이 없는 행은 "no replay for that row".

### 5. 상태 텍스트

가드: "set SERVER URL first" / "log in first" / "set PLAYER ID to your account id first" / "set PASSWORD first" / "set EMAIL first".
인증: "logging in..." / "registering..." / "logged in as \<id\>" / "login failed: \<err\>" / "register failed: \<err\>" / "session expired: \<err\>" / "logged out".
동기화: "uploading settings..." / "settings uploaded" / "downloading settings..." / "settings downloaded" / "conflict: server newer, download first" / "settings blob unreadable: \<err\>" / "settings upload worker stopped" / "settings download worker stopped".
라이벌: "saving rivals..." / "rivals: \<count\>" / "rivals: \<err\>". 랭킹: "ranking: \<err\>". 리플레이: "downloading replay \<id\>..." / "replay downloaded" / "replay download: \<err\>" / "downloaded replay has no inputs" / "no replay for that row".
모든 에러 텍스트는 공백 정규화 후 56자에서 "..."로 절단.

### 6. 결과 화면 IR 블록

좌측 컬럼 x 44, y 470부터 30px 간격, 스케일 1.8. "IR: OFF" / "IR: SKIPPED (\<reason\>)"(autoplay/replay playback/judge window widened/scratch assist) / "IR: SENDING..." / "IR ERROR: \<err\>"(빨강) / "IR: REJECTED"(+메시지, 빨강) / "UNRANKED: \<reasons\>"(노랑, 서버 차단 flag들을 ", "로 조인) / "IR: RANK #\<n\>" / "IR: SENT". 신규 베스트 시 "NEW BEST"(녹색). 리플레이 업로드 시도 시 "REPLAY UPLOADED"(녹색) 또는 "REPLAY ERROR: \<err\>"(빨강).

### 7. CLI

`--server`/`--player`는 저장된 설정에 덮어쓰는 1회성 오버라이드. 저장된 베어러 토큰은 매 실행 시 스코어 서버에 전달되고 첫 프레임 whoami 프로브로 검증(만료 시 폐기 + "session expired: ...").

## 계약 변경

- `SubmitResponse` 슈퍼셋 필드(`ranked`/`flags`/`is_new_best`/`score_id`)는 클라이언트가 `#[serde(default)]`로 수용 준비를 마쳤으나, 서버(`route/score.ts`)가 아직 4개 필드를 보내지 않아 랭킹 제외 사유 표시·리플레이-스코어 연결·베스트 판정 등 3개 기능이 서버 대응 전까지 무력이다(needs_from_others로 문서화, `docs/reference/ir-api.md` §5).
- 게스트 미인증 제출은 `player_id: "guest"` 리터럴을 요구하는 서버 규칙을 `rbms_ir::GUEST_PLAYER_ID`로 문서화 + 계약 테스트로 고정.
- settings PUT 409 충돌을 전용 `IrError::SettingsConflict` 변형(서버 사본 포함)으로 노출하도록 클라이언트 계약을 확장(서버 쪽 변경 없음, 기존 409 응답 파싱만 강화).

## 리뷰 결과와 수정

병렬 코드 리뷰에서 크리티컬 2건, major 다수, minor 다수가 나왔고 전부 처리(FIXES 로그 기준):

- **크리티컬 — 비밀번호가 SERVER URL로 유출**: 텍스트 편집 커밋이 "지금 편집 중인 행"을 기억하지 않고 인덱스 매핑에 `_ => SERVER URL` catch-all이 있었던 것이 원인. `App.text_edit_row: Option<usize>` 신설 + `network_text_field(index)`를 전역 전사(total) 함수로 교체, 포커스 이동 시 `cancel_text_edit()` 강제 호출로 근본 해결.
- **크리티컬 — 랭킹 패널 LOADING 영구 고착**: 이전 fetch를 버리는 동작 자체가 없었던 것이 원인. 단일 in-flight 가드 + `RankingCache::has_settled_answer()`(로딩 자리표시자는 캐시 히트로 안 침)로 해결.
- **major 8건**: `passnotes` 누락(레퍼런스 구현 `counts[..5].sum()`과 대응 확인 후 추가), 로그아웃 후 401 유실(`finish_logout`이 `player_id`를 guest로 복구 + `submission_player_id`로 이중 방어), guest id 미상수화(`GUEST_PLAYER_ID` 신설), 설정 PUT 204의 낙관적 잠금 갱신 누락(`refresh_sync_base()`), 409가 잠금을 전진시키는 문제(`SyncLock`+`SyncOutcome` 상태기), 리플레이 다운로드 하이재킹(`replay_download_is_applicable(&Stage)` 가드), 낡은 제출이 새 런의 IR 상태를 덮어씀(`IrStatus::accepts_report()`), 파일 길이 초과 3건(분할로 해소).
- **minor**: `//` 라인 주석 44건 제거, `ReplayMeta.url` 이중 프리픽스 문서화+테스트, `played_at`/`gauge_value` 서버 범위 클램프, 랭킹 패널 ArrowRight 미지원, 라이벌 목록 열림 중 탭 클릭 시 오버레이 잔류.
- 처리하지 않고 보고만 한 것: `ir_map.rs`의 기존 `//` 주석 11건(이번 변경 파일 아님, comments.md §1.2에 따라 유지), `http.rs` 628줄 초과(이번 변경은 1줄뿐이라 미분리), `cargo fmt` 미실행 판단(당시 근거는 rustfmt.toml 부재 — 본 검증 태스크에서 지시에 따라 `cargo fmt --all` 실행 및 재검사 통과로 갱신됨, 아래 참조).

E2E 검증(실 DB, `RBMS_IR_E2E_URL` 게이팅 신규 테스트): 계정 등록/로그인/whoami, 점수 제출·랭킹·베스트, 리플레이 업로드/다운로드 이벤트 바이트 동일 비교(gauge 필드는 서버 미반영 확인 — 위 계약 변경 항목과 동일 원인), 설정 PUT 204/409/충돌 사본 확인, 라이벌 PUT, 타 계정 설정 PUT 403, 과대 페이로드 413, 중복 제출 200, 계정 중복 409, 미등록 차트 조회 빈 배열/null, 게스트 미토큰 제출 201/타 id 401 — 전부 `docs/reference/ir-api.md` §3 문서와 일치.

## 검증 수치

이번 태스크(검증 전용, 코드 미수정)에서 실측:

1. `cargo fmt --all -- --check` — 최초 실패(레포에 `rustfmt.toml` 없음, 기본 폭 100 vs 실제 폭 ~160 스타일 불일치로 전 레포 규모 diff). 지시에 따라 `cargo fmt --all` 실행 후 재검사 **통과**(0 diff).
2. `cargo test --workspace` — 크레이트별:
   - rbms-audio: 99 passed, 0 failed, 1 ignored
   - rbms-chart: 149 passed, 0 failed, 0 ignored
   - rbms-cli: 0 passed(테스트 없음)
   - rbms-ir: 231 passed, 0 failed, 1 ignored (+ `tests/e2e_local.rs`: 1 passed, 게이트 미설정 시 스킵 경로)
   - rbms-judge: 131 passed, 0 failed, 0 ignored
   - rbms-model: 50 passed, 0 failed, 0 ignored
   - rbms-parser: 130 passed, 0 failed, 0 ignored
   - rbms-play: 49 passed, 0 failed, 0 ignored
   - rbms-player: 255 passed, 0 failed, 0 ignored (+ `tests/autoplay_preview.rs`: 1 passed)
   - rbms-render: 123 passed, 0 failed, 0 ignored (+ `tests/golden.rs`: 8 passed)
   - rbms-table: 50 passed, 0 failed, 0 ignored
   - 합계: **1,277 passed / 0 failed / 2 ignored**
3. `cargo clippy --workspace --all-targets` — 실제 경고(요약 라인 제외) **76개**. `crates/rbms-ir`: **0개**. `apps/rbms-player`: **22개**(중복 계산 없는 고유 위치), 전부 `main.rs`/`settings.rs`/`app_input.rs`/`app_library.rs`/`app_play.rs`/`app_select.rs`/`scores.rs`/`tablesrc.rs`의 collapsible-if·sort_by_key·to_* 관용구 스타일 lint이며 정확성 이슈는 없음.
4. `cd web && bun run verify` — typecheck 통과, lint(eslint) 통과, `bun test` **203 pass / 2 skip(Next 런타임 필요 라우트 통합 테스트, 의도된 게이팅) / 0 fail**, 432 expect() calls.
5. 레퍼런스 구현 명칭 grep(`crates/`, `apps/`, `web/src`, `README.md`, `docs/`) — **0건**.
6. `crates/rbms-ir`·`apps/rbms-player` 우선순위 diff의 신규 `"//"`(비-`///`) 주석 라인 — 기계적 grep 1건 검출됐으나 확인 결과 `apps/rbms-player/src/ir_map.rs:304`의 **기존 주석**이 `cargo fmt`로 공백만 재정렬된 것(내용 불변, 이번 세션에서 추가되지 않음). 실질 신규 추가 **0건**.
7. `git status --short` — `cargo fmt --all`이 워크스페이스 전역을 재포맷해 이전에는 수정 없던 파일들(rbms-audio/rbms-chart/rbms-judge/rbms-model/rbms-parser/rbms-play/rbms-render/rbms-table 각 소스·예제·테스트 30여 개)까지 `M`으로 나타난다. 기존 IR 통합 변경분(Modified 다수 + Untracked: `app_library.rs`/`app_network.rs`/`app_ranking.rs`/`ir_outcome.rs`/`ir_panel.rs`/`ir_ranking.rs`/`ir_ranking_view.rs`/`ir_replay.rs`/`ir_session.rs`/`ir_sync.rs`/`main_tests.rs`/`settings_view.rs`(apps/rbms-player), `contract_tests.rs`+`contract_tests/`+`dto/`+`error.rs`+`mock_http.rs`+`tests/`(crates/rbms-ir), `guide-sections.ts`+`guide-article.test.tsx`(web))는 그대로 유지. 상세 목록은 명령 실행 로그 참조.

## 알려진 제약

- 서버(`web/src/server/route/score.ts`)가 `SubmitResponse` 슈퍼셋 4필드를 아직 보내지 않아, 클라이언트가 이미 대비한 랭킹 제외 사유 표시·리플레이-스코어 연결·`is_replay_upload_warranted` 판정이 실서버 응답에서는 항상 기본값으로 나온다.
- `web/src/server/service/domain/replay/replay.service.ts`가 다운로드 응답에서 `gauge`를 누락하고 `chart.md5`를 빈 문자열로 반환해, 다운로드한 고스트 리플레이가 원래 런을 완전히 재현하지 못한다(E2E에서 이벤트 바이트는 동일함을 확인, gauge/md5만 미반영).
- `web/src/server/route/setting.ts`의 성공 PUT이 204(본문 없음)라 클라이언트가 새 잠금 기준을 응답에서 받을 수 없다 — 현재는 PUT 직후 GET 재조회로 우회(`refresh_sync_base()`).
- `finish_logout`이 PLAYER ID를 guest로 되돌려 재로그인 시 계정 id 재입력이 필요하다(제출 안전성 자체는 세션 파생 신원으로 보장, UX 트레이드오프로 남김).
- `crates/rbms-ir/src/http.rs`(628줄)와 `apps/rbms-player/src/main.rs`는 파일 길이 상한을 이미 넘긴 기존 상태이며, 이번 변경으로 추가된 줄 수가 적어 분리하지 않았다.
- `apps/rbms-player`의 clippy 22건(collapsible-if 등 스타일 lint)은 기존 코드 이동으로 귀속 파일만 바뀌었을 뿐 이번 세션이 새로 만든 경고는 아니며, 우선순위가 낮아 미수정 상태로 남겨졌다.
- `cargo fmt`는 이번 검증 태스크에서 처음으로 워크스페이스 전체에 적용됐다(직전까지는 "rustfmt.toml 부재로 인한 전 레포 재포맷 방지" 판단에 따라 미실행 상태였음) — 이후 세션은 이 포맷 기준을 새 SSOT로 삼아야 한다.

## 계약 수정 (리뷰 후속)

리뷰에서 지적된 서버측 계약 결함 세 건을 전부 엔드투엔드로 수정하고, 지적 4·5는 이미 충족되어 있음을 재확인했으며, `bun run dev`로 띄운 실서버를 상대로 실제 HTTP 라운드트립을 실행해 검증했다.

**수정 1 — `POST /api/scores` 및 코스 제출 2개 라우트가 슈퍼셋 응답 필드를 누락.** 원인은 라우트가 아니라 서비스였다: `submit()`이 `ranked`/`flags`/`scoreId`/`isNewBest`를 계산해 두고도 4필드짜리 `response` 리터럴만 만들어 `irRaw`가 그대로 흘려보냈다. `submitResponseSchema`에 `ranked`·`flags`·`is_new_best`·`score_id`를 추가하고 두 서비스의 응답 객체를 `SubmitResponseOutput`으로 타이핑해, 앞으로 필드가 어긋나면 런타임 누락이 아니라 컴파일 에러가 되도록 고쳤다. `score.service.ts`는 랭킹 정책·베스트 비교·삽입된 id에서 값을 채우고(중복 제출 경로는 기존 id + `is_new_best: false`), `course.service.ts`도 동일 패턴(`RANKED_COURSE_SUBMISSION`, 빈 flags, `isNewBest` 호이스팅)을 따른다. 라우트 자체는 변경이 필요 없었다 — 둘 다 서비스 결과를 그대로 직렬화한다.

**수정 2 — 리플레이 다운로드가 원래 런을 재현하지 못함.** `toReplayData`가 저장된 `gauge` 컬럼을 전혀 읽지 않았고 `chart.md5: ''`를 하드코딩했다. 이제 `gauge: gaugeTypeFromId(row.gauge)`와 `md5: row.chartMd5 ?? ''`를 반환하며, `ReplayRow`에 `chartMd5`를 추가하고 두 리플레이 쿼리에 `leftJoin(chart, chart.sha256 = replay.chartSha256)`을 넣어 채운다. FE `ReplayData` 타입에 `gauge`를 추가했다.

**수정 3 — `PUT settings`가 204를 응답해 클라이언트가 저장된 잠금 기준을 알 수 없었음.** 라우트가 이제 200 `{ updated_at }`(신규 `settingPutResponseSchema`)을 응답하고, 더 이상 쓰이지 않는 `irNoContent`는 제거했다. `rbms-ir`에서는 `put_settings`가 `Result<SettingsPutResult, IrError>`(`{ updated_at, from_server }`)를 반환하도록 바꿔 `SettingsBlob`은 그대로 두면서 가장 작은 일관된 변경만 적용했다 — 새 `SettingsPutResponse` 바디를 디코딩하고, 구버전 서버의 204 응답에는 보낸 스탬프로 폴백하며 `from_server: false`를 표시한다. 플레이어는 새로 추가한 순수 함수 `ir_sync::upload_outcome`을 통해 서버 스탬프를 받아들인다: `Some(Uploaded(stamp))`는 라운드트립 없이 잠금을 이동시키고, `None`(204)이면 여전히 `refresh_sync_base`를 트리거한다. `SyncOutcome::Uploaded`는 이제 스탬프를 담으며, `Conflict`는 여전히 기준을 전진시키지 않는다는 점을 기존 유닛 테스트와 새 e2e 어서션이 함께 고정한다.

**지적 4 — `GUEST_PLAYER_ID`**: 이미 `crates/rbms-ir/src/lib.rs`에 `pub const`로 존재하고, 플레이어가 제출 id가 필요한 모든 곳(`ir_session::submission_player_id`, `app_network`, `main.rs`, `settings.rs`)에서 쓰이며 `ir-api.md` §1에도 문서화되어 있다. player id로 남은 `"guest"` 리터럴은 없다(남은 두 건은 ACCOUNT 상태 라벨과 문서 산문일 뿐). 변경 불요.

**지적 5 — 이미 충족 확인, 변경 불요**: `ScoreSubmission.passnotes`는 존재하고 플레이어가 채우며(`app_play.rs:293 passnotes: j.total_judged()`) `contract_tests::submit_score_sends_passnotes_alongside_total_notes`로 고정되어 있다. `ReplayMeta.url`은 API base와 조합해 쓰이지 않는다 — 플레이어는 `meta.id` + `download_replay(id)`를 쓰며, `download_id()`와 이중 접두사 위험을 고정하는 테스트가 있다. `played_at`/`gauge_value` 범위 좁히기는 `dto/score/tests.rs`와 `worker.rs`의 `clamp_to_server_bounds` 테스트로 커버된다.

**드러난 사양 편차**: PUT 응답의 `updated_at`은 **ISO 문자열이 아니라 epoch-ms 정수**다. 계약의 다른 모든 곳(`settingBlobResponseSchema`, 409 conflict 바디, 키 목록, `rbms-ir::SettingsBlob.updated_at: i64`)이 이 타입이므로, ISO 문자열로 했다면 클라이언트의 잠금 기준 타입을 깨고 GET과도 어긋났을 것이다. 반환값은 저장된 값 그대로다.

**`docs/reference/ir-api.md` §5에 기록된 남은 제약(이번 작업이 닫은 두 격차를 대체)**: `replay` 테이블에 `extra` 컬럼이 없어 업로드된 `extra`가 버려진다(마이그레이션이 필요해 이번 변경 범위 밖). `gauge` 없이 업로드된 리플레이는 컬럼이 `NOT NULL DEFAULT 0`이라 `AssistEasy`로 돌아온다 — 플레이어는 항상 `gauge`를 보내므로 실제 경로에는 영향 없다.

**소유 범위 밖이라 손대지 않은 stale 문서(후속 필요)**: `docs/crates.md:243`는 여전히 "`put_settings uses put_no_content`"라 적혀 있고, `docs/web/tasks.md:135,147`는 여전히 settings PUT을 204로 설명한다.

### 실제 HTTP 라운드트립 (E2E)

백그라운드에서 `cd web && bun run dev`를 실행(Ready in 251ms, `GET /api/health 200`)한 뒤, 그 살아있는 서버를 상대로 게이트된 테스트를 돌렸다. 이 테스트는 `#[ignore]`가 아니라 `RBMS_IR_E2E_URL`이 없으면 스스로 스킵하므로, 환경변수를 설정한 일반 `cargo test`로 실행했다. 종료 후 3000번 포트가 비었고 `next dev`/`next-server` 프로세스가 남지 않았음을 확인했다.

명령: `RBMS_IR_E2E_URL=http://localhost:3000/api cargo test -p rbms-ir --test e2e_local -- --nocapture`

출력 그대로:

```
running 1 test
created throwaway account e2e-phase-i-18d391944f88
submit -> accepted=true rank=Some(1) ranked=true flags=["UNKNOWN_BUILD"] is_new_best=true score_id=Some("sc_63331ebe443846b0a8ed24f8ddae66")
test phase_i_round_trip_against_a_live_server ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.77s
```

생성된 유일한 계정은 `e2e-phase-i-18d391944f88`.

네 가지 수정에 대해 e2e에 추가한 어서션(실서버 대상 전부 통과):

- 슈퍼셋 필드: `score_id`가 비어 있지 않게 존재(이후 리플레이의 `score_id`로 재사용), `ranked`가 true, `has_flag(UnknownBuild)`, `unranked_reasons()`가 비어 있음, `previous_best: None`과 함께 `is_new_best`가 true. `played_at + 1`에 더 나쁜 두 번째 제출을 보내면 `is_new_best: false`, 다른 `score_id`, `previous_best: Some(EX_SCORE)`를 받음 — new-best·not-new-best·flags 경로를 전부 실제 응답으로 커버.
- 리플레이 충실도: 업로드 → 다운로드를 `serde_json::to_value`로 전체 비교하고(서버가 부여하는 `id`만 기대값에 주입), `gauge`가 유지되는지와 `chart.md5`·`chart.sha256`이 함께 돌아오는지 명시적으로 확인.
- Settings PUT: `put.from_server`가 true, `put.updated_at == get_settings().updated_at`, `put.updated_at != blob.updated_at`(서버가 자신의 시계로 스탬프를 찍음)를 확인. 이어지는 조건부 쓰기는 `put.updated_at`을 기준으로 성공하고 자신의 저장 스탬프를 보고한다.
- Conflict가 기준을 전진시키지 않음: 오래된 기준을 두 번 보내 두 번 다 지고, 409 바디의 서버측 사본이 매번 그대로임을 확인.

실행 중 dev 서버 로그에는 예상대로 `PUT .../settings/keyconfig 200`(이전엔 204), 의도된 `409` 두 건과 의도된 `403` 한 건이 찍혔고 예상 밖의 4xx/5xx는 없었다.
