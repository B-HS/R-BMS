# rbms — 레퍼런스 구현 core PLAY 모듈 Rust 포팅 (PROCESS / 단일 출처)

> 새 세션은 **이 문서부터** 읽는다. 현재 상태·아키텍처·실행법·할 일의 SSOT. (ai-process.md 원칙 1·14)
> 베이스 룰: `~/.claude/CLAUDE.md` + convention. Rust 프로젝트 → TS 전용 규칙(arrow 등) 비적용, **공통 원칙**(주석 금지·설명은 docs/·정확 네이밍·근본 해결·공식문서 우선·검증 후 진행)은 그대로.
> 위치: `/Users/gkn/R-BMS`. 빌드 `cargo build`, 테스트 `cargo test --workspace`(**Phase F 최종 검증 실측 2232 통과 · 0 실패 · 2 ignored**). lint 는 이제 게이트다 — `cargo fmt --all --check` 와 `cargo clippy --workspace --all-targets --all-features -- -D warnings` 가 CI 필수 통과 조건이고 툴체인은 `rust-toolchain.toml` 로 `1.95.0` 고정. 실행은 §5(`./start.sh`).
> git: **dev(작업)/prod(배포) 브랜치 모델**(CI는 dev, 릴리스는 prod → `docs/ci-release.md`). **커밋 메시지에 co-author(Claude) 넣지 않음**(사용자 명시 지시), 작성자 `Hyunseok Byun <gumyoincirno@gmail.com>`. `target`·`Cargo.lock`·라이브러리 차트 커밋 금지(.gitignore).
> 다음 할 일(로드맵)은 **`ROADMAP.md`**, 백엔드 설계는 **`docs/backend/`**, 배포/CI는 **`docs/ci-release.md`**.

---

## 현재 작업 — 전면 고도화 감사·계획 수립 (2026-09-09~)

> 사용자 지시: 프로젝트를 정확히 읽고 고도화 계획 수립. 관점 = (1) 스킨 완전 커스터마이징 (2) IIDX(~34) 기능 전수 확인 (3) 레퍼런스 구현 판정 동일성 + 설정 노출 (4) crate 분리·역할 (5) 파일 길이·Rust 관용성 (6) 메모리 누수 (7) UI/UX(프리징·미리듣기·디버그·시작 전 옵션·target 그래프) (8) 성능 (9) 시작 전 설정 vs 인게임 옵션.
> 운용: 위임은 **Workflow만**(Agent 단독 호출 금지, 사용자 지시 2026-09-09). 리서치·구현·적대 검증 = Opus, 기계적 수정(clippy/문서 경로/테스트 분리) = Sonnet, 검토·종합·핵심 판단 = Fable 직접. 병렬 가능한 것은 항상 병렬. 산출 = 계획 문서 `docs/plan/2026-09-09-enhancement-plan.md`.

- [x] a. 인라인 정찰 — PROCESS.md·크레이트 맵·LOC 핫스팟·레퍼런스 구현 소스 배치(play/skin/json·lr2·lua 로더)·현 SkinConfig 스키마 확인
- [x] b. 리서치 워크플로 실행(`wf_1911d201-312`) — 9관점 보고서 + 관점별 검증 9편 완료(반박 2건: 디버그 오버레이 부재·LN unwrap 패닉), IIDX 30~34 웹 조사 완료. 완전성 비평 대기
- [x] c. Fable 검토 — 전 보고서·검증 통독, 직접 코드 확인 6건(見逃し POOR 슬롯·윈도우 표·오디오 클럭·FLOATING·그린넘버 LIFT·assist 플래그). 검토 노트 `scratchpad/review-notes.md`
- [x] d. 계획 문서 확정 — `docs/plan/2026-09-09-enhancement-plan.md`(관점별 갭 표·Phase A~G·결정 12건·비평 반영·문서 정정 목록·한계). 근거 보고서 24편 `docs/plan/research-2026-09-09/`. 후속 조사 4건(IR·커스텀 판정 정책·Renderer diff·cpal 실측) 병합, `cargo test` baseline 889/0
- [x] e. 사용자 결정 12건 전부 추천안 채택 → `docs/acknowledge/2026-09-09-enhancement-decisions.md`

### Phase A — 정확성 핫픽스 (2026-09-09 착수, Workflow `rbms-phase-a`)

> 파일 소유권 분리로 병렬: 1차 core(model/parser/chart/judge/play)·audio·ir·render 4갈래 동시 → 2차 app 통합 → 갈래별 적대 리뷰 5개 병렬 → 수정 → 문서 정정(Sonnet) → Fable 최종 검증(`cargo test --workspace` + clippy). 항목 정의는 계획 §2 Phase A.

- [x] A-core: J1+J2+J3, J4·J5·J7·J8, J10·J11, J12-1, J13·J15·J16·J18·J19, J14 `#DEFEXRANK`, C7 NaN 소절 거부, `#SWITCH/#CASE/#SKIP/#DEF`(결정 10) — 리뷰 반영까지 전부 완료. 보류 2건: J17/J20/J21/J23 상세·J24(계획대로 Phase D 범위), 지뢰 damage 스케일(1차 출처 미확인, 현 값 유지)
- [x] A-audio: A5 기본 gain 하향(0.5, 레퍼런스 구현 정확 일치), A7 스트림 사망 감지, P7 카운터·scratch 선할당, A13 채널 키 피치, 클럭 페어 seqlock화 — 전부 완료. 보류 1건: A6 start/stop 램프는 계획대로 Phase B 범위
- [x] A-ir: `http.rs` 타임아웃 강등 제거, HTTP mock 테스트, degraded 경로·8엔드포인트 URL 검증 — 완료. 보류 1건: URL percent-encoding은 `url` 크레이트 신규 의존 필요해 문서화만(코드 미변경)
- [x] A-render: P1 폰트 캐시 상한, K9 결과 팔레트 스킨화(후방호환 API), 골든 하네스 격자·감도 강화 — 완료. 보류 1건: RSS 상한 계측 하네스는 계획 문서(소유 밖) 반영만 남음
- [x] A-app: A3 키음/판정 시각 분리, IR 제출 게이트(autoplay/replay/judge rate>100), assist 플래그, U3 Root Esc, U4 조작 저장, P6 cancel, 원자적 저장 + `rule_version`, U6 URL 중복, P3 디바운스, 스트림 사망 폴백, 그린넘버 매직상수 통합 — 완료. 보류 1건: 어시스트 램프 강등(`LightAssistEasy`)은 `rbms_judge::ClearType` 변형 신설이 필요해 Phase D 이월(대신 `best_clear_for_md5`가 어시스트 기록을 보수적으로 제외)
- [x] 적대 리뷰 5갈래 → 수정 (core/audio/ir/render/app 전 갈래 반영 완료, `scratchpad/phase-a/review-*.md`·`fix-*.md`)
- [x] 문서 정정(계획 §4) + divergences.md J1~J26 섹션
- [x] 커밋(2026-09-09, 사용자 지시 "갈래별로 co-author 없이"): `b9e25f6` parser · `c8f6b1b` judge · `cfc41ee` audio · `18eefc3` ir · `3179b2d` render · `e730209` player · `2c7adef` docs (author 단독, 트레일러 0, Conventional Commits — 커밋 훅이 요구)
- [x] Fable 최종 검증(2026-09-09 실측): `cargo test --workspace` **1,060 통과 · 0 실패 · 2 ignored**(착수 전 889), `cargo clippy --workspace` 경고 51(기준 동일), `--all-targets` 94(기준 101). 에이전트가 추가한 `//` 주석 약 210줄은 Sonnet 워크플로로 제거·history 문서로 이동. 실기 가청 1회(사용자)는 미실시 — history §5 절차 참조

### Phase W — web/ Next.js 단독 서버 + 웹 FE (2026-09-09 착수, Phase A 와 병렬)

> 사용자 지시: 백엔드를 Next.js 단독 서버로 이 레포 `web/` 에, Vercel `bms.hyuns.uk`. better-auth + MySQL(.env 추후) + Drizzle, 전부 최신, TanStack prefetch + Next 캐시/revalidate, design.md 토큰 + shadcn 목록화. 결정 → `docs/acknowledge/2026-09-09-enhancement-decisions.md` §웹.
> 워크플로: W0 설계(`docs/web/architecture.md`·`tasks.md`·`components.md`) ∥ 스캐폴드(web/ 생성·토큰·providers·drizzle·better-auth·health) → Fable 검토 → W1 api-core ∥ web-ui(mock) → W2 api-ext ∥ ui 통합 → 리뷰·수정 → 검증(typecheck·lint·test·build) → 문서.

- [x] W0 설계 문서(`docs/web/{architecture,components,tasks}.md`) + 스캐폴드(`web/`, Next 16.3.4·React 19.2.8·Tailwind 4.3.3·shadcn 4.21·Drizzle 0.45·better-auth 1.7.3·TanStack 5.102, typecheck/lint/test/build 통과). 미결 6건 결정 → decisions 문서
- [x] W1 API 코어 + 웹 UI(MSW) — 리뷰·수정 완료, typecheck/lint 0·테스트 155 통과·빌드 19 라우트. `drizzle-kit migrate` 로 사용자 DB에 21테이블 적용(2026-09-09). **실 DB 스모크(Fable)**: `/api/health`·`/api/version` 200, register 201+토큰, guest 제출 201(unranked: guest), 토큰 제출 rank 1, 더 나쁜 재제출 시 베스트 유지, autoplay 제출 unranked, ranking raw 배열·best raw 객체·chart 메타 정상, `/api/auth/me` Bearer 정상. 계약 보정 4건 도출 → W2a: 미등록 차트 ranking `[]`/best `null`(404 대신), played_at ±7일 → 미래 5분만 거부, `.env.example` 키 보강; 후속: 빈 sha256 허용(md5 단독 클라 지원)
- [x] W2 API 확장 ∥ UI part1 → UI part2 → 리뷰 3·수정 완료. Fable 최종 게이트: typecheck 0·lint 0·테스트 196 통과(2 skip)·빌드 35 페이지. 실 DB 스모크 전 항목 정상(→ `docs/history/2026-09-09-web-nextjs-ir-server.md`)
- [x] 커밋(web 갈래별) + push origin dev (2026-09-09)
- [x] Vercel 배포(2026-09-09): 프로젝트 `rbms-web`(b-hs) 링크, Production env 6종(DATABASE_URL·BETTER_AUTH_SECRET·BETTER_AUTH_URL·NEXT_PUBLIC_APP_URL·ALLOW_GUEST·REPLAY_MAX_BYTES) 등록, `vercel --prod` 성공. 프로덕션 별칭 `https://rbms-web.vercel.app` 에서 health/version/랭킹/FE 통계/페이지 7종 200 확인(실 DB). 배포 해시 URL은 배포 보호(SSO)로 302 — 정상
- [x] 도메인 `bms.hyuns.uk`: 사용자가 Cloudflare 레코드를 Vercel CNAME(`*.vercel-dns-016.com`, DNS only)으로 교체(2026-09-09). `https://bms.hyuns.uk/api/health` 200 확인(Vercel IP 216.150.1.65 경유)
- [x] CI(dev) 골든 실패 수정 완료: 원인은 곡선택 폴더 행 마커 U+25B8 가 임베드 Inter 에 없어 OS 시스템 폰트로 폴백된 것. 골든 테스트를 임베드 폰트 전용 결정적 렌더(`TextEngine::embedded_only`, `use_embedded_fonts_only`)로 수정 — 커밋 `61b9a97`, CI run 34307010505 에서 ubuntu/macos/windows + fmt/clippy 전부 통과
- 후속: md5 단독 제출(LR2IR 어댑터), Vercel Blob, 라이트/다크 스크린샷 확인, 슈퍼셋 서버의 누락 필드 `.default()`

### 사용자 지시 (2026-09-09): "멈추라 할 때까지 멈추지 말고 계획대로 전부 구현" — 클라 GUI 로그인 등 IR 연동은 CLI 가 아니라 GUI 에서 전부 가능해야 함

> 실행 순서(파일 충돌 회피): Phase I(클라 GUI 연동) → B(오디오) → C(구조) → D ∥ E1~E2 ∥ F → E3~E6 ∥ G → R(릴리스). 각 Phase = Workflow 1개(구현 Opus max, 파일 소유권 분할 → 적대 리뷰 Opus → 수정 → Fable 게이트 `cargo fmt/test/clippy` + web `bun run verify`) → 갈래별 Conventional Commit → push → CI 확인 → PROCESS/history 갱신.

### Phase I — 클라 ↔ IR 서버 GUI 연동 (Workflow `rbms-phase-i`)
- [x] I-ir(커밋 `a1987ba`): `rbms-ir` DTO 확장(`SubmitResponse` ranked/flags/is_new_best/score_id) + `HttpScoreServer` register/login/me/replay download/settings get·put(낙관적 잠금)/rivals put/course ranking + 에러 변형(401/409/413/429) + mock HTTP 테스트 + `docs/reference/ir-api.md`
- [x] I-app(커밋 `6226f0c`): NETWORK 탭 GUI — 계정 상태·EMAIL·PASSWORD(마스킹, 비영속)·LOGIN/REGISTER/LOGOUT(백그라운드, 토큰 영속) · `build_server` 토큰 전달 · 결과 화면 IR 결과(rank/new best/unranked 사유) · 곡선택 IR 랭킹 패널(비동기 캐시·라이벌 행) · 리플레이 자동 업로드 + 랭킹 행에서 다운로드 재생 · 설정 동기화(업로드/다운로드/충돌) · 라이벌 관리
- [x] I-web(커밋 `a91f73f`): `/guide` 를 GUI 절차 기준으로 재작성(CLI 제거) + README Web 절 정정. 워크스페이스 `rustfmt.toml`(max_width 160) 추가 + 포맷 커밋 `0f12ae8`, CI 3-OS 통과
- [x] 적대 리뷰 28건(critical 2: 비밀번호가 SERVER URL 로 유출·랭킹 패널 LOADING 고착) → Rust 측 전건 수정 → 검증(테스트 1,277 통과). **GUI 실기 검증(2026-09-09, TAIDE 접근성 권한 후 키 자동화)**: 곡선택 → SETTINGS/NETWORK 13행 렌더 → SERVER URL 입력 → REGISTER 로 일회용 계정 `gui-e2e-871795` 생성(ACCOUNT "logged in as", PASSWORD 소거, rivals 갱신) → 서버 `/api/players/{id}` 조회 확인 → IR RANKING 패널 상태 표시 확인. 사용자 settings.ron 은 백업 후 복원
- [x] 서버 측 계약 결함 수정(커밋 `7fa8106` web · `beb2210` ir · `a1edc71` player · `f98899b` docs): submit 응답에 ranked/flags/is_new_best/score_id, 리플레이 다운로드에 gauge·chart.md5, settings PUT 200 `{updated_at}`(204 하위호환), guest 상수 확인. 로컬 dev 서버 e2e(`crates/rbms-ir/tests/e2e_local.rs`, 계정 `e2e-phase-i-18d391944f88`) 통과. `vercel --prod` 재배포 후 `https://bms.hyuns.uk` health/version/guide/홈 200, Chrome 실렌더 `/guide` 라이트·다크 양쪽 확인(2026-09-09). history: `docs/history/2026-09-09-phase-i-client-ir-integration.md`

### Phase B~G + 릴리스 (계획 `docs/plan/2026-09-09-enhancement-plan.md` §2)
- [x] B 오디오 클럭 재설계(2026-09-09, 가청 확인만 사용자 몫) — 보간 클럭·룩어헤드 스케줄·단일 AudioEngine·볼륨 3분리/#VOLWAV/보이스 스틸/램프·오디오 설정 노출·판정 오차 하네스/언더런 카운터/소크
  - [x] 구현 (명세 `docs/plan/2026-09-09-phase-b-spec.md`)
  - [x] 적대 리뷰 23건 반영 — 보간 클럭 계단형 붕괴·룩어헤드 부족·콜백 내 PCM free·xrun 오탐 사망·스틸 클릭·검증 지표 상수화 등. → `docs/history/2026-09-09-phase-b-audio-review-fixes.md`, 명세에 항목별 정정 블록
  - [x] §4.3 소크 13분(6세그먼트, HOME 격리, 수정 반영 후): underruns/drops/steals/hard_steals/late/ts_fallbacks 0, interp_sd 최대 9.17µs(계단형이면 3,079µs), RSS 기울기 −1.99MB/분, 로그 panic/error 0. 실기 보간 잔차 sd 3,073µs → 12.5µs
  - [x] 게이트(Fable 실측): fmt 통과, `cargo test --workspace` 1,475 통과·0 실패·3 ignored(착수 전 1,284), clippy 신규 경고 0. 커밋 `ec4d9d2` audio · `b53f99b` play · `b70fc56` parser/#VOLWAV · `b683e88` player
  - [ ] 실기 가청 확인 1회(키음 즉시성·BGM 온셋·클릭 없음·프리뷰 전환) — 사용자 몫
- [x] C 구조 개편(2026-09-09, 명세 `docs/plan/2026-09-09-phase-c-spec.md`, 커밋 `3813075`..`a243ce5`; 게이트 실측 fmt 통과·clippy `-D warnings` 0·`cargo test --workspace` 1,656 통과·0 실패·3 ignored(착수 전 1,474)·리뷰 30건 반영·에이전트 `//` 주석 693줄 제거·history `docs/history/2026-09-09-phase-c-structure.md`) — Stage enum·PlaySession→rbms-play·rbms-config·설정 descriptor 테이블·판정/게이지 데이터화·rbms-store/rbms-library 이관·CI lint 게이트/thiserror/forbid(unsafe)
  - [x] Wave 0 (P1~P6 병렬): judge 데이터화(`data/judge.ron` 6모드·`data/gauge.ron` BEAT_7K + 패리티 가드) + `JudgeAlgorithm` 4종 + `clear_type_id` 수신 · `rbms-table` `match_levels`/`TableError` · render `SkinError`·전역 인자 주입 · parser/chart/model clippy 0 · 루트 `[workspace.lints]`/`[workspace.dependencies]` · rbms-play clippy 0 + `Player::judge()` 접근자
  - [x] Wave 1 (S1→S4 직렬): `rbms-store`(scores/replay/write_atomic, md5 인덱스로 O(N) 조회) · `rbms-library`(취소 가능 스캔·`Library`) · `[lib]` 타깃 + `run() -> ExitCode` + `rbms-cli scan`/`scores` · `ir_map`→`rbms-ir` feature `mapping` · `rbms-config`(단일 `Config`+`schema_version`+마이그레이션, 픽스처 7종) · `App{shared,stage}` 분해 + `StageHandler`/`Transition`(`self.stage = Stage::` **0곳**) · `PlaySession`+`SoundSink` · 설정 descriptor 테이블(정수 인덱스 상수·`SETTING_TABS` 삭제)
  - [x] Wave 2 H0: 앱 잔여 clippy 0, `keyconfig.rs` 인라인 테스트 → `keyconfig_tests.rs` 분리
  - [x] Wave 2 H1 게이트 플립(실측 2026-09-09): 전 14크레이트 `[lints] workspace = true`, 전 크레이트 루트 `#![forbid(unsafe_code)]`(플레이어 포함 — `bytemuck` derive 와 충돌 없음이 실측으로 확인돼 §11 미확인 1 해소), CI `continue-on-error` 제거 + `cargo fmt --all --check` 게이트 + `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `rust-toolchain.toml` `channel = "1.95.0"` 고정(CI 도 동일 버전 설치). 검증: clippy **0 경고**, fmt 통과, `cargo test --workspace` **1,634 통과 · 0 실패 · 3 ignored**(착수 전 1,475)
  - [x] 문서(§9 step 14): `docs/crates.md`(신규 3크레이트 + judge `data`/`algorithm` + `match_levels` + `PlaySession` + 플레이어 절 재작성), `docs/architecture.md`(크레이트 맵·Stage/PlaySession·데이터화·lint 게이트), `docs/acknowledge/reference-divergences.md` §Phase C(C-D1 후보선택 기본값 `Duration`·C-D2 trait→enum·C-D3 게이지 6종)
  - **예외 1건**: `crates/rbms-audio/tests/rt_safety.rs` 는 계수 global allocator 로 콜백 무할당을 증명하는 테스트라 `unsafe` 가 필수 → 그 테스트 크레이트 루트만 `#![allow(unsafe_code)]`. 나머지 전 타깃은 forbid/deny 유지
  - [x] 적대 리뷰 25건 반영(2026-09-09) — major 9 · minor 16. 결함 수정: RESULT 화면에서 오디오 재오픈 가드가 풀려 키음 뱅크가 버려지던 것(`stage_owns_chart_audio` 신설) · 상위 스키마 동기화 blob 을 v1 로 강등 파싱해 계정 설정을 파괴하던 것 · 마이그레이션이 구 `settings.ron` 을 백업 없이 덮어써 롤백 불가였던 것(`settings.ron.v{from}.bak`) · 오디오 장치가 없을 때 press 가 리플레이에 안 남아 press 없는 release 만 담기던 것 · IR 제출의 `judge_algorithm` "Combo" 하드코딩(실제 정책 `Duration` 과 모순) · 기동 실패 `unwrap`(`Gpu::new -> Result` + `ExitCode::FAILURE`). 배선 누락: `rbms-cli config` 신설 · `judge.ron`/`gauge.ron` 런타임 소비 전환(내장 const 는 폴백 겸 패리티 기준) · `AppShared.songs` → `rbms_library::Library`(md5 인덱스) · NETWORK 탭 정수 행 인덱스 13개 + `NETWORK_SETTING_ROWS` 삭제(전부 `SettingId` 라우팅). 위생: `ConfigError::Read` 실사용 · `AudioOptions` `PartialEq` 에서 `reopen_pending` 제외 · 도달 불가 `AUDIO_PENDING_STATUS` 제거 · `Player::judge` 비공개 · `rbms-ir` 글롭 re-export → 명시 목록 + 워크스페이스 dep 통합 · `render_playfield` shim/`#[allow(too_many_arguments)]` 제거 · `tablesrc` `Result<_, String>` → 타입 에러 + `rbms-table` shim 2개 삭제 · `rbms-store` 의 UI 문자열 → 앱 `format.rs`(`is_stale_rule_version` 만 노출), `ScoreBook.records` 비공개화 · 매직넘버 2건 상수화 · 신규 파일 `//` 주석 제거 · `stage/select.rs` → `select/{mod,preview,scene}.rs`, `lib.rs` → `assets.rs` 분리
  - [x] 리뷰 후 게이트(Fable 실측): fmt 통과, `cargo test --workspace` **1,656 통과 · 0 실패 · 3 ignored**(리뷰 전 1,634), `cargo clippy --workspace --all-targets --all-features -- -D warnings` 0 경고, `cargo run -p rbms-cli -- config <v0 settings.ron>` 스모크 통과
  - [x] 스펙·문서 정정: 스펙 §4(`rbms-config → rbms-store`)·§4.1(백업/`ConfigError::Read`/blob 상한)·§4.7·§6.1(소비 전환), `docs/acknowledge/reference-divergences.md` §Phase C 스펙 대비 의도적 이탈 C-S1~C-S4(목록 상태 `AppShared` 유지 · `dyn StageHandler` 단일 match · `rbms-config → rbms-store` · `App`/`AppShared` 크레이트 루트 유지)
  - 미실시: 실기 GUI 확인(H1 은 게이트·문서 변경이라 렌더 경로 무변경)
- [x] D 판정 패리티 완성 + JUDGE 탭 노출(2026-09-10, 명세 `docs/plan/2026-09-09-phase-d-spec.md`; D0~D5 순차 구현 + 적대 리뷰 23건 반영, history `docs/history/2026-09-09-phase-d-judge-parity.md`, 발산 `docs/acknowledge/reference-divergences.md` §Phase D) — J17 알고리즘 4종(기본값 `Combo` 전환)·J20 9게이지+J26 GAS·J21~J23(5세트×9원소 게이지+PMS fixjudge)·J9/A10 정역 2키(BSS/MSS 앱 배선)·J24 CN/HCN 2단계 폴드·J25 LN MODE(모델+리플레이+스코어 키 전부 동일 축)·J6 24K Mode 신설·어시스트 램프 강등(`rule_version` 게이팅). 게이트(Fable 실측): fmt 통과, `cargo test --workspace` **1876 통과 · 0 실패 · 3 ignored**(D 착수 전 1,656), `cargo clippy --workspace --all-targets -- -D warnings` 0 경고, 금지어 grep 0, 신규 `//` 주석 0. HEAD `0c20fa9` 위 워킹트리 변경(미커밋)
- [ ] E1~E6 스킨 완전 커스터마이징 — 프리미티브(PNG 골든·textured quad·클립·아틀라스)·타이머/키프레임·프로퍼티 바인딩·JSON+Lua 로더·화면 이식·스킨 선택 UI
- [x] F UX·기능 고도화(2026-09-09~10, 명세 `docs/plan/2026-09-09-phase-f-spec.md`; F0~F4 5갈래 병렬 구현 → 통합(`docs/history/2026-09-10-phase-f-integration.md`) → 적대 리뷰 21건(critical 1·major 다수) 반영 19건·발산 등록 2건 → 최종 검증, history `docs/history/2026-09-09-phase-f-ux.md`, 발산 `docs/acknowledge/reference-divergences.md` §Phase F) — 옵션 오버레이(결정 3, 11행)·토스트/상태줄·백그라운드 로딩(폴더스캔/표fetch/키음·BGA 디코드 워커화)·정렬 12종(레퍼런스 `BarSorter` 패리티, 오름차순+무기록 마지막)·필터 패널(레벨/모드/클리어/즐겨찾기)·타깃 8종(고정레이트 11종 전체선택+RANK NEXT)/PACEMAKER 실시간·결과 3분할 그래프(GAUGE/TIMING/JUDGE)·27분위 랭크바. 최종 게이트(Fable 실측, 수정 없이 재검증): fmt 통과(no-op), clippy `-D warnings` 0 경고, `cargo test --workspace` **2232 통과 · 0 실패 · 2 ignored**(테스트 바이너리 38개 전부 ok), 금지어·금지주석 0건. 옵션 오버레이/결과+그래프/필터+토스트 3화면 헤드리스 PNG 육안 확인(임시 하네스, 확인 후 원복). 미구현 2건(BPM 정렬=시작BPM vs 레퍼런스 최고BPM · 차트파싱 미워커화)과 LANE OPTION FLIP/BATTLE 미배선은 발산 문서에 사유·해소조건 등록 후속(경미): 토스트가 곡선택 우하단 힌트 줄과 겹침 — 토스트를 힌트 위로 올리거나 표시 중 힌트를 숨길 것
- [ ] G 데이터 스케일·롱테일 — 곡DB(rusqlite)·스코어DB·코스·연습 모드·gilrs/MIDI·시스템 사운드·복수 IR
- [ ] H 경미 후속 일괄(사용자 결정 2026-09-10: G 다음에 모아서 처리) — ① 토스트가 곡선택 우하단 힌트 줄과 겹침 ② Phase B 실기 가청 확인(사용자 몫) ③ 이후 발생하는 경미 항목은 여기에 누적
- [ ] R 릴리스(prod 브랜치·브랜치 보호·v0.1.0 태그·서명) — **사용자가 직접 수행**(자택 보관 키 사용, 2026-09-10 결정). 에이전트는 착수하지 않음

---

## 현재 작업 — 전체 로드맵 실행 (2026-06-03~)

> 사용자 지시: "계획을 docs에 반영 후 Phase 0→7까지 멈추지 말고 진행." 확정 순서(9-에이전트 적대적 분석·비평으로 도출, 비평 정정 반영). 상세 forward plan은 로컬 `ROADMAP.md`, 결정 대기 항목은 `docs/acknowledge/`.
> 효력: S<1일 · M=1~3일 · L≈1주 · XL=수주. `(선택)`/`(BLOCKED)` 표기.

**Phase 0 — 위생 & 사실 정정 (전부 S, 즉시) ✅ 완료(2026-06-03)**
- [x] 문서 사실 정정 1패스 — 테스트 수 `132/92/100 → 880`(PROCESS·CLAUDE·ci-release), CN/HCN 판정 미차별 명확화(`divergences.md` 추가)
- [x] `ci-release.md` stale 정정 — 리모트·dev 존재 반영, prod 브랜치+첫 태그만 잔여
- [x] 루트 `LICENSE`(GPL-3.0 전문) 추가
- [x] `ROADMAP.md`·README 배포 섹션 dev/prod·Actions 상태 정정 (역할: ROADMAP=로컬 워킹, docs/roadmap=공개 ledger)
- [x] 미커밋 `docs/` + `LICENSE`를 dev 커밋(`1d3fb13`, 작성자 Hyunseok Byun, co-author 없음; gitignore 항목 제외)
- [x] dev → origin 푸시 (CI 3-OS 트리거됨)

**Phase 1 — 클라 정확성(핵심 패리티) + 마무리**
- [x] `PlayOptions.lntype`를 헤더에서 유도 (app_play.rs 하드코딩 제거 → `ir_map::ir_lntype`, 0=LN/1=CN/2=HCN, 테스트)
- [x] #PREVIEW: `config.debug` 6분기 계측 + `samples/preview-demo` 픽스처 + dead_code 제거 (가청 확인은 수동 1회 — `docs/bug/2026-06-03-preview-playback.md`)
- [x] (2026-06-07) **곡선택 하이브리드 미리듣기** — `#PREVIEW` 있으면 파일, 없으면 곡 **autoplay 미리듣기**(백그라운드 코디네이터 스레드: 파싱→스케줄→키음 디코드, 취소가능, 위상연속 루프). `load()`와 디코드 헬퍼 공통화. 2R 적대적 리뷰 반영. → `docs/history/2026-06-07-select-autoplay-preview.md` (가청 1회는 사용자 몫)
- [x] ⭐ `LnKind` 판정 전파 + CN/HCN 2-판정 모델(head@press + end@release, 이른릴리스=end 판정·미히트=2 Miss, 분모 CN=2) + 합성 픽스처 5종. `Cn`/`Hcn` 게이트 → LN/Normal byte 불변. (HCN 연속 게이지·CN deferral·BSS = Phase 7; → `docs/reference/cn-hcn-judgment.md`)

**Phase 2 — 첫 릴리스(인프라)**
- [ ] prod 브랜치 생성·푸시 + 브랜치 보호 전략 결정(필수, `docs/acknowledge` 기록)
- [ ] 첫 GitHub Release (v0.1.0 수동 태그, macOS 유니버설+Windows+sha256)

**Phase 3 — 클라 마지막 기능**
- [x] NETWORK 설정 탭 — SERVER URL/PLAYER ID 행(22/23), text_input 인플레이스 편집(Enter 커밋·Esc 취소·버퍼 표시), `PlaySettings` 영속(라운드트립 테스트), `build_server` 추출로 ScoreServer 재구성. 빈 URL=오프라인·빈 ID=guest.

**Phase 4~6 — 백엔드 + 웹 FE (2026-09-09 Phase W 로 대체·완료, 별 레포 대신 이 레포 `web/`)**
- [x] IR 계약 동결(/api 프리픽스·settings 경로·필드명) → decisions W7
- [x] 백엔드 M1·M1.5·M2·M3 전부 `web/` Next.js 단독 서버로 구현(38 라우트: health/version/auth/scores/charts/players/replays/settings/rivals/courses/tables/admin/fe) + ranked 정책·build allowlist·audit + Vercel 프로덕션 배포
- [x] 웹 FE: 검색·차트·리더보드·플레이어·인증(로그인/가입)·홈·tables·courses·리플레이 뷰어·settings(토큰/동기화/라이벌)·admin/builds
- [ ] 클라 `SubmitResponse` DTO 확장 — 현 `rbms-ir` DTO 는 accepted/rank/previous_best/message 뿐, 서버가 주는 ranked/flags/is_new_best/score_id 미수신
- [ ] 클라 로그인 UI/토큰 — `apps/rbms-player/src/main.rs` `build_server` 가 `HttpScoreServer::try_new(url, None)` 으로 token=None → 플레이어 제출은 전부 guest(unranked). NETWORK 탭에 로그인(register/login → 토큰 저장) 필요. `rbms-ir` 는 Bearer 지원 완료
- [ ] 클라 랭킹 패널·리플레이 업/다운로드·설정 동기화 토글·rival UI — 서버 엔드포인트는 전부 준비됨, 클라 호출부 없음
- [ ] (선택) 설정 화면 마우스 스테퍼 UX

**Phase 7 — 롱테일 / 선택 폴리시 & 하드닝**
- [ ] HCN 연속 게이지(L, 정확성·HCN 희소) · (선택) 스크래치 BSS · (선택) 게이지 5K/PMS
- [ ] (선택/BLOCKED) 결과·메뉴 레이아웃 RON화 · (선택) 테마 chrome 색 완성
- [ ] (선택) 백엔드 M4 LR2IR READ 어댑터 · (선택/BLOCKED) FE 리플레이 뷰어 · (선택) 백엔드 M5 안티치트
- [ ] (선택) macOS 서명/공증·Windows Authenticode · (선택) .app/.dmg+무인자 picker
- [ ] (선택) 코스메틱: 리사이즈 리플로우·BGA 비디오(mpg)·글리프 아틀라스(P3a)·웹폰트(P4) — 판정/점수 영향 0

> 결정 대기(요약): CN/HCN 테스트 코퍼스 범위 · prod 보호+첫 버전 · 서명 비용. (IR 계약·백엔드 소유권·리플레이 뷰어는 Phase W 로 해소.) 다음 착수 후보: ① 클라 로그인/토큰 + DTO 확장(MVP 루프 완성) ② 고도화 계획 Phase B~G(`docs/plan/2026-09-09-enhancement-plan.md` §2) ③ prod 브랜치·첫 릴리스.

---

## 0. 목표 / 현재 상태

레퍼런스 구현(Java/libGDX) **코어 PLAY**를 Rust로 재구현. **완전 플레이 가능 + 레퍼런스 구현/IIDX 확장 다수 완성.**
흐름: BMS 로드 → (노트옵션 셔플) → 로딩화면 → 하이스피드 스크롤 → 키음 샘플정확 재생 → 입력/판정/게이지 → HUD → 결과 → (옵션)스코어 서버 전송. GUI 곡선택(폴더·난이도표 네비)·설정(탭)·키설정 에디터·리플레이 포함.

판정/게이지/TOTAL은 **레퍼런스 구현과 byte/수치 단위로 대조 검증됨**(§6 검증 참고). 미구현은 §7 한계.

## 1. 확정 결정

- **그래픽**: `wgpu 29 + winit 0.30`, 네이티브 인스턴스드 쿼드(`fill_rect`=1인스턴스). 렌더는 `Renderer` trait 추상화(CPU 백엔드 `CpuCanvas`도 존재 → 테스트/예제). 레퍼런스 좌표공간 **1280×720(16:9)** 고정, GPU 유니폼이 NDC로 매핑.
- **오디오**: `cpal 0.17 + symphonia 0.5 + 커스텀 RT 믹서`. **마스터 클럭 = 재생된 샘플 수**(vsync 아님) — 레퍼런스 구현의 vsync 양자화 자체는 피했으나, 클럭이 **오디오 콜백 주기(실측 512프레임/10.667ms 고정)로 양자화**되는 점은 남아 있다(판정 타임스탬프 오차 평균 5.33ms·최대 10.67ms). 근본 해결은 Phase B에서 마지막 콜백 시각 기준 연속 보간으로 처리 예정 — `docs/plan/2026-09-09-enhancement-plan.md` §1.2 A1.
- **시간 전역 µs(i64)**. 차트 해시 = raw 바이트 MD5+SHA-256(byte-exact).
- **모드/스킨/키맵/설정은 데이터 주도**. 모드=`Mode` 구조체, 스킨=`SkinConfig`(RON), 키=`KeyConfig`(RON), 플레이옵션=`PlaySettings`(RON).
- **키맵 기본 = 레퍼런스 구현 Z열**(사용자 결정). 차트 모드 자동감지로 프리셋 선택, CLI/인앱 오버라이드.
- **사용자 설정 파일**(`~/.config/rbms/`): `settings.ron`(플레이옵션) · `keyconfig.ron`(키) · `tables.ron`(난이도표 목록) · `scores.ron`(로컬 플레이 기록) · `replays/`(리플레이). 모두 없으면 기본값 자동생성, 파싱실패 시 `.bak` 백업.
- **테스트 데이터**: `/Users/hyunseokbyun/Documents/personally/1/`(발광 ★1, 727차트). **커밋 금지(저작권)**. 발광1 난이도표 = `https://darksabun.club/table/archive/insane1/data.json`(로컬 86곡/18레벨 매칭 실측).

## 2. 모드 프로파일 (확장성)

`rbms_model::Mode`(name/key/player/scratch/channel_assign) + 상수 `BEAT_7K/5K/10K/14K/POPN_9K`. `detect_mode(src, filename)` 자동판별. 새 키모드 = 데이터 추가(엔진 분기 없음).
**14K 레인(주의)**: BEAT7 table 기준 P1 키=lane0-6/스크=7, **P2 키=lane8-14/스크=15**.

## 3. 크레이트 맵 (의존: 위→아래)

```
apps/rbms-player  ← winit+wgpu 윈도우·게임루프·GUI(셀렉트/설정탭/키설정/난이도표·로컬기록모달·마우스·디버그). 모든 크레이트 사용.
                    main.rs + 모듈: keyconfig.rs(키설정) · settings.rs(플레이옵션 영속) · replay.rs(리플레이) · tables.rs(난이도표목록) · scores.rs(로컬 기록 ScoreBook).
apps/rbms-cli     ← 차트 정보 출력(파싱 검증 도구).
crates/
  rbms-play       ← 통합 드라이버: 키음 스케줄러·autoplay·press/release·auto_lanes·키빔상태·**키봄상태(bomb: 노트히트 판정 기록)**·set_judge_rate·simulate_autoplay.
  rbms-render     ← Renderer trait + CpuCanvas + skin(SkinConfig/Skin) + playfield(노트·키빔·레인장식·커버) + hud(콤보/판정/가로게이지) + result + **select**(SelectView/SelectHot/render_select: 곡선택 행리스트·상세·밀도그래프·기록·모달, 헤드리스 예제) + **render_key_bomb**(노트히트 폭발, SkinConfig 데이터주도) + font(**cosmic-text 다국어**: FontSystem+SwashCache, fill_rect 알파, load_font/set_ui_family).
  rbms-table      ← 난이도표(BMS table): dto(TableEntry/Header)·DifficultyTable(fetch/fetch_or_cache·by_level)·md5 매칭. reqwest blocking.
  rbms-audio      ← decode(symphonia)·mixer(RT voice pool,stride 리샘플)·engine(cpal, SAMPLES_PLAYED 클럭, **sample_duration_us**=프리뷰 루프용).
  rbms-ir         ← IR-슈퍼셋 클라이언트: dto(serde)·ScoreServer trait·HttpScoreServer·NullScoreServer. (클라 계약=docs/reference/ir-api.md, **서버 전체설계=docs/backend/**)
  rbms-judge      ← windows(µs 윈도우·rank·scaled)·matcher(JudgeEngine: 매칭·콤보·EX·미스·LN·fast/slow·**空POOR empty_poor**·set_windows)·gauge(GrooveGauge·ClearType).
  rbms-chart      ← detect_mode·to_model(마디→µs·채널→레인·LN/mine)·scroll(visible/constant_offsets·그린넘버)·shuffle(NoteOption)·count_playable_notes·**note_density**(레퍼런스 구현 SongInformation 포팅: 1초빈 히스토그램+peak/avg/end).
  rbms-parser     ← BMS lexical: base36/62·Shift-JIS·#mmmCC·#RANDOM/#IF·MD5/SHA-256 → BmsSource.
  rbms-model      ← 순수 타입: Mode·Note·TimeLine·Model·LnKind·NoteKind.
```

## 4. 완료 기능 (전부 검증됨)

**코어:** 파서(727곡 MD5 byte-exact) · 타이밍모델 · 오디오(실디바이스) · 스크롤/렌더 · 판정엔진 · autoplay · 입력 · 라이브 윈도우.
**판정/게이지:** 판정 윈도우(레퍼런스 구현 `JudgeProperty.SEVENKEYS` 일치) · `#RANK→judgerank`(`[25,50,75,100,125]%`, NORMAL=#RANK2=75%) · 게이지 6종+모디파이어 · 클리어램프(색=레퍼런스 구현 `LAMP`) · LN 풀판정(head hold→release) · **空POOR(이른 빈 POOR)=노트 미소실·콤보 유지·MS페널티**(레퍼런스 구현 `judgeVanish[5]=false`·`combo[5]=true`, `empty_poor` 카운터).
**게임옵션:** 모드 자동감지 · **노트옵션**(OFF/MIRROR/RANDOM/S-RANDOM/R-RANDOM/ROTATE, 시드 결정적·DP 사이드분리) · **하이스피드 고정**(FLOATING/CONSTANT 그린넘버) · **판정 오프셋**(±200ms) · **오토 캘리브레이션**(정확판정 평균오차로 결과 시 offset 보정, per-run 수렴) · **JUDGE WIDTH**(judge_rate 50~200% 윈도우 스케일) · **TOTAL**(override/노트수 기본) · lift · lane cover(sudden) · scratch side/auto.
**렌더/UI:** 1280×720 랜드스케이프(필드 center-left+우측 BGA) · 키 빔 · 레인 outline+구분선(opacity/onoff) · **가로 게이지**(레인 아래 0~100, IIDX식) · **다국어 폰트**(cosmic-text+Inter+시스템폴백 → 일/한/중/태/아랍 등 모든 언어 AA 렌더, `font_path`/`--font`로 교체) · 플레이 HUD(콤보·직전판정·FAST/SLOW·판정카운트·EX·게이지) · 결과화면 · 일반/와이드 스킨(RON, 임베드+`--skin`) · BGA(이미지) · BGA on/off.
**셀렉트/설정/키:** GUI 곡선택(재귀 스캔·폴더 네비 Root→ALL SONGS/난이도표→레벨→차트, ↑↓·←상위/→진입·Enter·**마우스 클릭**) · 우측 정보패널+**로컬 기록 인라인 리스트·상세 모달**(`R`/클릭, 리플레이 재생) · 폴더 선택(rfd) · **난이도표 인앱 관리**(다중표 tables.ron·URL텍스트입력/파일rfd 추가·제거) · **설정 탭**(PLAY/GAUGE/JUDGE/DISPLAY/INPUT, 탭/값 **마우스 클릭**) · **통합 키 설정**(파일+인앱 에디터, 레인+조작키 전부 재바인딩·충돌검출) · **설정 영속**(settings.ron, AUTO REPLAY 토글 포함) · **리플레이 저장·재생**(`--replay`/모달, 시드/옵션 복원해 동일 재현) · **로컬 스코어 영속**(scores.ron, 서버 무관).
**IR:** IR 슈퍼셋 클라이언트(스코어 제출, `--server`·`--player`) — DTO 슈퍼셋 확장(JudgeBreakdown **early/late(epg…lms)·avgjudge·empty_poor**, PlayOptions **전체옵션**, ScoreSubmission **seed/algo/rule/skin/`client_build_sha256`(자기-빌드 sha2)·`client_platform`**, ReplayData **µs구조**(`ReplayEvent`), settings/replay-dl/auth/course 메서드 stub). 전부 serde default 후방호환. **백엔드 서버는 설계 완료·구현 후속**.
**ROADMAP 클라이언트 9종(2026-05-31 완료):** 폴더 SCANNING 로딩·**스코어 랭크 그래프(IIDX 9분법 `dj_rank`+랭크바, 결과+셀렉트)**·**점수 ΔEX 비교(직전/베스트)**·**리플레이 분석모드(재생바·일시정지·배속·재시뮬 시크·노트별 ms-off)**·**데이터 주도 HUD 스킨**(판정 팔레트/라벨/게이지 임계·색/요소위치 RON화)·**폰트 P3**(무할당 중첩 캐시·말줄임)·**노트옵션 ALL-SCRATCH/H-RANDOM**(시간임계 40/125ms)·**그린넘버 표시**·**CN/HCN(`#LNMODE`)**·**듀얼필드 14K(P1좌·P2우)**. SCORE GRAPH·REPLAY ANALYSIS 설정 토글. → `docs/history/2026-05-31-roadmap-client-features.md`.
**UI(IIDX/LR2 지향 재설계, 2026-05-31):** 폴더 영속(`songs_folder`)·폴더 스캔 백그라운드 스레드+애니메이션 LOADING·노트 상단 클리핑(`[top_y,judge_y]`, 프레임 상단서 흘러나옴)·플레이 IIDX 레이아웃(좌 정보, 중앙 **라이브 스코어 그래프**, 우 판정카운트/BGA)·결과 IIDX 레이아웃(거대 DJ LEVEL+스코어 리포트, PGREAT 핫핑크)·곡선택 행별 클리어램프 LED·**곡선택 상세 메타 고도화**(부제/아티스트/장르·제작자 + 2열 스탯그리드: BPM범위·DIFFICULTY명·NOTES(+LN)·JUDGE(#RANK명+%)·LENGTH·TOTAL; 포커스 곡만 lazy `to_model`로 노트수/길이/BPM범위 산출, `#MAKER` 파싱, bms-rs 메타 모델 참조). DP(14K)는 BGA 비키도록 좌측 앵커. 레퍼런스 `docs/reference/ui/`·스펙 `docs/reference/ui-design.md`. → `docs/history/2026-05-31-ui-redesign-iidx.md`.
**곡선택 전면 재설계(레퍼런스 구현 modern chic, 2026-05-31):** 렌더링을 **`rbms-render::render_select`로 추출**(`SelectView`/`SelectHot`, `CpuCanvas` 헤드리스 PNG 검증 가능, `main.rs` 인라인 제거) — 행 KEY/레벨 배지·클리어램프 LED(좌바+우세로바)·포커스 연출(시안테두리+노랑타이틀)·중앙포커스 스크롤, 상세 **커버(`#STAGEFILE`→`#BANNER`, 단일 BGA슬롯 쿼드뒤·포커스당1회 디코드)**+제목블록(2줄 래핑)+2열 스탯그리드+**노트 밀도 히스토그램**(`note_density`, 서브픽셀·긴곡 오버플로없음·PEAK/AVG/END notes/sec)+기록(베스트바·DJ랭크바·최근행 EX+랭크+추세)+기록모달. `main.rs` `build_select_view` 조립 + `SelectKey` 캐시(매프레임 재할당 방지)+hot 매핑. 파서 `#BANNER`/`#PREVIEW` 추가(`#PREVIEW`는 데이터만, 재생 후속). **2라운드 적대적 멀티에이전트 리뷰**(1R 16건 반영·2R 0건). → `docs/history/2026-05-31-select-redesign.md`, 스펙 `docs/reference/ui-select-redesign.md`, 타깃 `…/ui/provided/06-reference-select-target.png`.
**곡선택 후속(2026-05-31): 스탯그리드 잘림 수정·#PREVIEW·KEY BOMB.** (1) 상세 스탯그리드 **2열×3행→3열×2행** 압축(하단 LENGTH/TOTAL이 DENSITY 구분선에 잘리던 것 해결). (2) **`#PREVIEW` 프리뷰 재생 (TODO — 실재생 미동작, 2026-06-07 해소)** — 포커스 settle(디바운스 20프레임) 시 `#PREVIEW` 디코드+루프(경계 재트리거, mixer `play(key)`가 동일키 먼저 stop하므로 선스케줄 대신 자연종료 후 재발화) 로직·select 전용 `AudioEngine`(플레이 엔진과 분리, `load()` 진입부·select 이탈 시 정지)·`AudioEngine::sample_duration_us`·**DISPLAY 탭 PREVIEW 토글**(`PlaySettings.preview`)까지 배선했으나 **포커스해도 소리 안 남**(추후 디버깅, 코드 유지). (3) **KEY BOMB** — 노트 히트(judge≤3, 空POOR/miss 제외) 시 판정선 판정색 확장+소멸 버스트. `Player.bomb`(press/release/autoplay 전 경로 기록)·`rbms_render::render_key_bomb`·**`SkinConfig` bomb_enabled/height/duration_ms 데이터주도**(기존 RON serde default 호환). 적대적 리뷰 1건(LOW: 리플레이 직접실행 시 cpal 스트림 1프레임 공존) 근본수정. → `docs/history/2026-05-31-select-redesign.md`.
**CI/배포:** GitHub Actions(`.github/workflows/ci.yml`·`release.yml`) — 자동 버전·macOS 유니버설·Windows 빌드·릴리스(→`docs/ci-release.md`). 실제 동작은 GitHub 원격 push 시.

**2026-06-03 세션:** IR `lntype` 차트 유도(`ir_map::ir_lntype`, 0=LN/1=CN/2=HCN; 기존 하드코딩 제거) · **`#PREVIEW` 계측**(6 silent 분기 `config.debug` 로그)+`samples/preview-demo` 픽스처+dead_code 제거(가청 확인만 수동 잔여) · **CN/HCN 2-판정**(head@press + end@release, `Cn`/`Hcn` 게이트, 분모 2, 픽스처 5종; LN/Normal byte 불변) · **NETWORK 설정 탭**(SERVER URL/PLAYER ID 인앱 편집·`PlaySettings` 영속·`build_server` 재구성) · **Windows 설정 경로**(`config_dir` HOME→USERPROFILE). + Phase 0 위생(LICENSE·테스트수 정정·docs 커밋·dev 푸시) · 릴리스/백엔드 결정 기록(보류). → `docs/history/2026-06-03-session.md`.

**2026-06-07 세션:** **곡선택 하이브리드 미리듣기**. 사용자 의도 정정 — 원하던 건 `#PREVIEW` 파일만이 아니라 "포커스한 곡이 들리는 미리듣기". 원본 레퍼런스 구현도 곡 autoplay 미리듣기는 없음(=#PREVIEW 파일 or 메뉴 BGM)이라, **하이브리드 신규 구현**: `#PREVIEW` 있으면 파일, 없으면 **백그라운드 코디네이터 스레드**가 차트 파싱→throwaway `Player`로 autoplay 키음 스케줄 추출→전체 키음 병렬 디코드(취소가능)→메인 스레드가 `PreviewMsg` 채널 드레인 후 클럭 앵커·dispatch-먼저-루프(+2초 tail, 위상연속 재-앵커). 키음 디코드 fan-out/job빌드는 `load()`와 공통 헬퍼(`keysound_jobs`/`spawn_keysound_decode`)로 통합. 헤드리스 통합테스트(`tests/autoplay_preview.rs`)+디코드→믹스 테스트 추가(889통과). **2라운드 적대적 리뷰**(1R 8건·2R 1건 전부 반영). → `docs/history/2026-06-07-select-autoplay-preview.md`. (실기 가청 1회는 사용자 몫)

**2026-06-07 세션 (2) — 첫 실행/온보딩(.dmg 대비):** `.dmg` 더블클릭(터미널 없음) 대비. (1) **무인자 진입** — 인자·기억폴더 없을 때 `usage`+`exit` 대신 GUI 진입. (2) **초기 스캔 백그라운드화** — `App::new` 동기 블로킹(창도 안 뜸) → 백그라운드 스레드+`scan_rx`/`apply_scan`, SCANNING 화면에 **곡 수 카운트** 표시. (3) **첫 실행 빈 곡선택 + 온보딩 CTA**(`SelectView.empty_hint`: WELCOME/add-folder, 헤드리스 렌더 시각확인). (4) **Esc-중-스캔** 종료 회귀 수정(취소 복귀). 집중 적대 리뷰 1건 반영. → `docs/history/2026-06-07-first-launch-onboarding.md`. (`.app`/`.dmg` 패키징·공증은 release.yml 측 후속)

상세 이력 → `docs/history/2026-05-31-*.md`·`2026-06-03-session.md` (§8 docs맵). 다음 할 일 → `ROADMAP.md`.

## 5. 실행

```bash
./start.sh                                    # release 빌드 후 기본 라이브러리+발광1 표로 열기
./start.sh "<폴더|차트>" [옵션...]             # 인자 그대로 전달 (env RBMS_SONGS / RBMS_TABLE 로 기본값 변경)
cargo build --release -p rbms-player          # 또는 직접 (BIN=./target/release/rbms-player)
$BIN "<폴더>"                                  # GUI 곡선택
$BIN "<차트.bme>" [--interactive]              # 단일 차트 (기본 autoplay; --interactive=직접)
$BIN --replay ~/.config/rbms/replays/<f>.ron  # 리플레이 재생
```
- **곡선택**: ↑↓ 이동 · →/Enter 열기/플레이 · ←/Esc 뒤로/상위 · **Tab 설정** · **O 폴더선택(rfd)** · **T 난이도표 관리** · **R 기록 모달** · **마우스**(행 1클릭 선택·재클릭 열기, 우측 기록 클릭→모달). 모달: ↑↓ 이전/다음·Enter 리플레이·Esc 닫기.
- **설정(Tab)**: **Tab으로 탭 전환**(PLAY/GAUGE/JUDGE/DISPLAY/INPUT) · ↑↓ 이동 · ←→ 값변경 · Enter(KEY CONFIG 진입) · Esc 저장후복귀.
  - PLAY: AUTOPLAY·HI-SPEED·SPEED FIX(FLOATING/CONSTANT)·RANDOM·**AUTO REPLAY**. GAUGE: GAUGE·TOTAL. JUDGE: JUDGE OFFSET·JUDGE WIDTH·AUTO CAL. DISPLAY: SKIN(NORMAL/WIDE)·**FONT**(DEFAULT/CUSTOM, Enter/우/클릭=파일선택 라이브 적용·좌=기본)·LIFT·LANE COVER·BGA·**DEBUG MODE**. INPUT: SCRATCH SIDE·SCRATCH AUTO·KEY CONFIG. (탭/값 마우스 클릭 가능)
- **플레이 중**: Esc=뒤로(중도 포기) — 단 **남은 노트가 없으면 Esc로 곧장 결과화면**(아웃트로 대기 스킵). **DEBUG MODE** 시 좌상단 FPS/RAM/프레임시간/노트·콤보·게이지 등 수치 오버레이.
- **키 설정**: 설정→`KEY CONFIG`→`Stage::KeyConfig`(EDIT MODE 전환·Enter 캡처 재바인딩·충돌 빨강·Esc 저장) 또는 `~/.config/rbms/keyconfig.ron` 직접 편집.
- **기본 레인 키(레퍼런스 구현 Z열)**: 7K=`Z S X D C F V`+LShift(스크) · 5K=`Z S X D C`+LShift · 9K/PMS=`Z S X D C F V G B`(스크없음) · 14K=좌손 P1(`Z S X D C F V`+LShift)·우손 P2(`M K , L . ; /`+RShift).
- **기본 조작 키(플레이 중, 재바인딩 가능)**: ↑/↓=hi-speed · →/←=lane cover · ]/[=lift. Esc=뒤로.
- **CLI 옵션**: `--interactive|--auto --sc-left --sc-auto --lift F --hispeed F --gauge ... --keys Z,S,X,... --skin file.ron --font file.ttf --table URL --keyconfig path.ron --replay file.ron --server URL --player ID`.

## 6. 검증 메모 (판정/게이지 — 사용자 "빡세다" 검증)

- 판정 윈도우 = 레퍼런스 구현 `JudgeProperty.SEVENKEYS`(PG±20/GR±60/GD±150/BD-280~220/MS-150~500 µs @judgerank100) **정확 일치**.
- `#RANK 0~4 → judgerank [25,50,75,100,125]%` = `JudgeWindowRule.NORMAL`과 일치. **#RANK 2(NORMAL)=75%=PGREAT ±15ms**는 레퍼런스 구현과 동일.
- 게이지/TOTAL = `deltas * total/notes`(그루브) 일치. 자동미스는 BAD 늦은경계 이후. **예외(2026-09-09 감사, Phase A에서 수정 완료)**: 見逃し POOR(스윕 미스)가 게이지 MS 슬롯(idx5)으로 잘못 들어가던 결함(J1)이 있었다 — 코드 4(PR)가 아니라 코드 5(MS)로 처리돼 게이지 페널티가 실제보다 컸다. `crates/rbms-judge/src/matcher.rs`에서 슬롯을 정정했다 → `docs/acknowledge/reference-divergences.md` "판정 윈도우·게이지 발산" 섹션.
- 결론: "빡센" 건 NORMAL 윈도우가 원래 타이트 + 입력/표시 지연 미보정. → **JUDGE OFFSET** 수동 또는 **AUTO CAL**(한 곡 플레이로 자동 보정).

## 7. 알려진 한계 / 다음 후보

- ~~CJK 폰트 없음~~ **해결**(cosmic-text). ~~캐시키 할당~~/~~말줄임~~ **P3 해결**(무할당 중첩 캐시·`fit_text`). **P3a run-length 병합 보류**(프로파일 ROI~11%, AA 텍스트 픽셀별 alpha 상이 → 본 해법은 글리프 텍스처 아틀라스, 후속). **P4 웹폰트(URL)** 후속.
- ~~14K 단일필드~~ **듀얼필드 해결**(`dual_field`, P1좌·P2우·바깥 스크래치, 필드별 judge라인/구분선/게이지 1개). 비활성화(`dual_field:false`) 시 레거시 단일필드.
- ~~ALL-SCRATCH/H-RANDOM 미구현~~ **해결**(시간임계 40/125ms). ~~green-number 미반영~~ **표시·반영**(HUD). ~~CN/HCN 미구분~~ **`#LNMODE`→`LnKind` 구분 + 종단 2-판정 차별화 적용**(HCN 연속게이지·CN deferral·스크래치 BSS는 Phase 7). 스크래치 회전(2키 교대) 단순화는 잔존.
- 윈도우 리사이즈 UI 리플로우 없음(논리 1280×720 고정). BGA 비디오(mpg) 미지원. 게이지 5K/PMS 변종·judgerank 커스텀 일부 미반영. 난이도표 추가 fetch는 동기(1개씩).
- **미구현(2026-09-09 감사, `docs/plan/2026-09-09-enhancement-plan.md` 참조)**: 곡DB(매 실행 전량 스캔·증분 없음) · 컨트롤러/MIDI/마우스 스크래치(의존성 0) · 코스/단위 · 연습 모드 · 곡선택 난이도·모드 필터/즐겨찾기/랜덤 선택 · 볼륨 3분리(system/key/bg, `#VOLWAV` 미반영, 현재 마스터 게인 1계통뿐).
- **IR 백엔드 = 전체 설계 완료(`docs/backend/`, 문서 단계)·구현 후속**. ~~클라 슈퍼셋 확장 필요~~ **클라 DTO 슈퍼셋 확장 완료**(§4 IR). 서버 미구현이라 신규 메서드(settings/replay-dl/auth/course)는 호출 시 `Unsupported`. `PlayOptions.lntype`에 실제 LN모드 전파는 후속(모델에 lnmode 미보유).
- ~~UI 곡선택 재설계~~·~~KEY BOMB~~ **완료**(§4). ~~#PREVIEW 파일만~~ **곡선택 하이브리드 미리듣기 완료**(2026-06-07): `#PREVIEW` 있으면 파일, 없으면 곡 **autoplay 미리듣기**(백그라운드 코디네이터·취소가능·위상연속 루프). 실기 가청 1회만 사용자 몫 → `docs/history/2026-06-07-select-autoplay-preview.md`. 남은 UI 후속: **스킨 데이터화**(결과/메뉴 패널까지 — 곡선택 추출·KEY BOMB 데이터화로 진척). ~~`.dmg` 첫 실행 UI~~ **첫 실행/온보딩 런타임 UI 완료**(2026-06-07: 무인자 진입·백그라운드 스캔+곡수·온보딩 CTA·Esc 취소 → `docs/history/2026-06-07-first-launch-onboarding.md`); 남은 건 **`.app`/`.dmg` 패키징·무인자 진입·macOS 공증**(release.yml 측).
- ~~Windows 설정 경로~~ **해결**(`config_dir` HOME→USERPROFILE, → `docs/reference/windows-compat.md`). ~~CN/HCN 판정 차별화~~ **종단 2-판정 적용**(HCN 연속게이지·CN deferral·BSS는 Phase 7). 남은: **F5 결과/메뉴 패널 위치 RON화**(현재 HUD 표면만 데이터화), **P3a 글리프 아틀라스·P4 웹폰트**, **FE 프로젝트**(별 저장소·MIT) → `ROADMAP.md`. 적대적 리뷰 보류 차이 → `docs/acknowledge/reference-divergences.md`.

> **세션 이력(완료)**: 입력판정 근본수정(空POOR)·폴더 ←→ 네비·마우스·로컬기록 모달·레퍼런스 구현 램프색·AUTO REPLAY·Play-Esc-즉시결과·DEBUG MODE → `docs/history/2026-05-31-input-judge-nav-mouse-records.md`. 다국어 폰트(cosmic-text) → `…-multilingual-font.md`. 백엔드 IR-슈퍼셋 설계 + CI/CD → `…-backend-ir-ci.md`. (모두 §4 완료기능·§8 docs맵에 반영됨)

## 8. docs 맵

- `docs/reference/` — mechanics(레퍼런스 구현 메커닉)·rust-stack·architecture·wgpu29-winit030-api·ir-api(서버계약)·_appendix-raw·**cn-hcn-judgment**(CN/HCN 판정 레퍼런스 구현 대조 구현 스펙·후속)·**windows-compat**(Windows/크로스플랫폼 현황: CI 보장·USERPROFILE 수정·런타임 검증 잔여)·**ui-design**(IIDX/LR2/레퍼런스 구현 UI 레이아웃 스펙)·**ui-select-redesign**(곡선택 정밀 스펙: render_select 추출·밀도 포팅·레이아웃)·**ui/**(레퍼런스 스크린샷: provided 6 + fetched 18 + README).
- `docs/plan/` — **2026-09-09 전면 고도화 계획**: `2026-09-09-enhancement-plan.md`(관점별 갭 표·판정 발산 J1~J26·Phase A~G 실행계획·결정 12건·문서 정정 목록, 이 문서의 정본) + `research-2026-09-09/`(근거 보고서·검증·후속조사·비평 24편, 원 리서치 산출물 보존용).
- `docs/acknowledge/` — decisions(확정결정)·reference-divergences(보류차이)·empty-poor-local-scores(空POOR처리·로컬기록 스키마).
- `docs/font-cjk-support.md` — 다국어 폰트 지원(cosmic-text 결정·레퍼런스 구현 폰트 파악·P1/P2 완료·P3/P4 후속).
- `docs/backend/` — **백엔드 IR-슈퍼셋 서버** 설계: README·PRD·api-spec(전 엔드포인트)·data-model(Drizzle)·endpoint-tasks·compatibility(LR2IR/레퍼런스 구현 매핑+출처)·**contract-freeze**(Phase 4 게이트: 클라 경로 대조·계약 동결·M0 단계). (Hono/Bun/Drizzle)
- `docs/ci-release.md` — GitHub Actions(ci/release): 자동 버전·macOS 유니버설·Windows 빌드·릴리스. 주의(원격/브랜치보호/LICENSE/서명).
- `docs/bug/` — 버그/진단 기록(예: `2026-06-03-preview-playback` #PREVIEW 계측·검증절차).
- `docs/memory/` — test-library(라이브러리 실측).
- `docs/history/` (시간순):
  - `m0-m1-parser-chart` · `m2-audio` · `m3-render-scroll` · `m6-judge-m4-autoplay` · `m7-live-window` — 코어 마일스톤.
  - `features-mode-gauge-ln-result-gpu-config-select` · `skin-bga-ir` · `font-hud-gui` — 1차 확장.
  - `beam-keymap-720p-table` — 키빔·모드별키맵·720p·재귀선택·난이도표.
  - `options-settings-folders` — 노트옵션·설정영속·게이지·BGA·하이스피드고정·오프셋·폴더선택·리플레이·다중표.
  - `judge-verify-tuning-skins` — 판정검증·JUDGE WIDTH/TOTAL·설정탭·일반/와이드스킨·레인장식·가로게이지·오토캘.
  - `input-judge-nav-mouse-records` — 空POOR 판정 근본수정·폴더 ←→ 네비·마우스 입력·로컬 기록(scores)+상세 모달·레퍼런스 구현 램프색·AUTO REPLAY·Play-Esc-즉시결과·DEBUG MODE.
  - `multilingual-font` — 다국어 폰트(cosmic-text+Inter+시스템폴백, 모든 언어 AA)·`font_path`/`--font` 교체·레퍼런스 구현 폰트 파악.
  - `backend-ir-ci` — 백엔드 IR-슈퍼셋 전체 설계(LR2IR+레퍼런스 구현 IR+설정동기화·µs리플레이·빌드해시무결성·FE) + GitHub Actions(자동버전·macOS유니버설·Windows)·라이선스/서명 정리.
  - `roadmap-client-features` — ROADMAP 클라이언트 9종: 폴더 SCANNING 로딩·스코어 랭크 그래프(IIDX 9분법)·점수 ΔEX 비교·리플레이 분석모드·데이터 주도 HUD 스킨·폰트 P3(캐시·말줄임)·IR 슈퍼셋 DTO·자기-빌드 SHA-256·노트옵션(ALL-SCR/H-RAN·green-number·CN/HCN·듀얼필드14K). 웨이브별 적대적 멀티에이전트 리뷰.
  - `ui-redesign-iidx` — IIDX/LR2 지향 UI 재설계: 폴더 영속·백그라운드 스캔(애니 로딩)·노트 상단 클리핑(근본 수정, 베젤 hack 제거)·플레이 IIDX 레이아웃+라이브 스코어 그래프·결과 IIDX 레이아웃·곡선택 클리어램프 LED+상세 메타 고도화(bms-rs 참조, `#MAKER`). DP 레이아웃 BGA 비킴. 적대적 리뷰로 DP충돌·stale폴더·perf 수정. 레퍼런스 수집·디자인 스펙.
  - `select-redesign` — 곡선택 전면 재설계(레퍼런스 구현 modern chic): `rbms-render::render_select` 추출(헤드리스 검증)·`note_density`(SongInformation 포팅)·커버(단일 BGA슬롯)·KEY/레벨 배지·밀도 히스토그램·타이틀 2줄·`build_select_view` 캐시·파서 `#BANNER`/`#PREVIEW`. 2라운드 적대적 리뷰(16건→0건).
  - `2026-06-03-session` — 위생(LICENSE·테스트수 정정·docs 커밋·dev 푸시)·IR lntype 차트 유도·#PREVIEW 계측+픽스처·CN/HCN 2-판정·NETWORK 설정 탭·Windows 설정 경로(HOME→USERPROFILE). 릴리스/백엔드 결정 기록(보류).
  - `2026-06-07-select-autoplay-preview` — 곡선택 **하이브리드 미리듣기**(#PREVIEW 파일 or 곡 autoplay): 백그라운드 코디네이터 스레드(파싱→autoplay 키음 스케줄→전체 키음 병렬 디코드, 취소가능)·위상연속 루프(+2초 tail)·`load()`와 디코드 헬퍼 공통화. 2R 적대적 리뷰(1R 8건·2R 1건 반영). 헤드리스 통합테스트 추가(889통과).
  - `2026-06-07-first-launch-onboarding` — `.dmg` 대비 **첫 실행/온보딩**: 무인자 GUI 진입(usage+exit 제거)·초기 스캔 백그라운드화(SCANNING+곡수 카운트, 창 즉시 표시)·첫 실행 빈 곡선택 온보딩 CTA(`empty_hint`)·Esc-중-스캔 종료 회귀 수정. 집중 적대 리뷰 1건 반영.
  - `2026-09-09-phase-a-accuracy-hotfix` — Phase A 정확성 핫픽스: 판정 윈도우·게이지 발산 J1~J19+J14(見逃し POOR 슬롯·5K/PMS 윈도우·LN 마진 등, 상세는 `reference-divergences.md`)·오디오 마스터 게인 재조정+seqlock 클럭·IR 타임아웃 강등 제거·폰트 캐시 LRU+골든 하네스 강화·`#SWITCH`계열 결함·앱 통합(IR 제출 게이트·assist 플래그·원자적 저장 등). 갈래별 병렬 구현→적대적 리뷰 1라운드→반영.
  - `2026-09-09-phase-d-judge-parity` — Phase D 판정 패리티 완성: `JudgeAlgorithm` 4종·9게이지+GAS·BSS/MSS·CN/HCN 2단계 폴드·LN MODE 전축 통일·24K Mode 신설·어시스트 램프 강등. 적대 리뷰 23건 반영.
  - `2026-09-10-phase-f-integration` — Phase F 갈래 합류: F0~F4 교차 배선(결과 fast/slow 스크래치 분리·모드 라벨·LETTERBOX 매프레임 반영·텍스트에디터 통일·PACEMAKER HUD·re-export·정렬 방향 확인). `stage/play.rs` 800행 초과 분리.
  - `2026-09-09-phase-f-ux` — Phase F 최종 검증(코드 수정 없음, `cargo fmt --all`만 적용): fmt/clippy(`-D warnings`)/`cargo test --workspace`(2232통과·0실패) 전부 green, 금지어·금지주석 0건, 옵션 오버레이·결과 그래프·필터+토스트 3화면 헤드리스 PNG 육안 확인.
- **새 세션 진입점 = 이 PROCESS.md**(CLAUDE.md가 지정). 별도 글로벌 하네스 메모리는 사용 안 함 — SSOT는 docs/.

## 9. 작업 규칙(요약)

각 스텝 검증 후 진행(테스트/렌더 PNG). 모호·범위변경 시 멈추고 질의. 큰 변경 후 적대적 멀티에이전트 리뷰. 커밋은 사용자 요청 시에만, **co-author 미포함**. `target`/`Cargo.lock`/라이브러리 차트 커밋 금지(.gitignore).
