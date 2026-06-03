# rbms — beatoraja core PLAY 모듈 Rust 포팅 (PROCESS / 단일 출처)

> 새 세션은 **이 문서부터** 읽는다. 현재 상태·아키텍처·실행법·할 일의 SSOT. (ai-process.md 원칙 1·14)
> 베이스 룰: `~/.claude/CLAUDE.md` + convention. Rust 프로젝트 → TS 전용 규칙(arrow 등) 비적용, **공통 원칙**(주석 금지·설명은 docs/·정확 네이밍·근본 해결·공식문서 우선·검증 후 진행)은 그대로.
> 위치: `/Users/hyunseokbyun/rbms`. 빌드 `cargo build`, 테스트 `cargo test --workspace`(현재 **886 통과·0 실패·0 경고**). 실행은 §5(`./start.sh`).
> git: **dev(작업)/prod(배포) 브랜치 모델**(CI는 dev, 릴리스는 prod → `docs/ci-release.md`). **커밋 메시지에 co-author(Claude) 넣지 않음**(사용자 명시 지시), 작성자 `Hyunseok Byun <gumyoincirno@gmail.com>`. `target`·`Cargo.lock`·라이브러리 차트 커밋 금지(.gitignore).
> 다음 할 일(로드맵)은 **`ROADMAP.md`**, 백엔드 설계는 **`docs/backend/`**, 배포/CI는 **`docs/ci-release.md`**.

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
- [x] ⭐ `LnKind` 판정 전파 + CN/HCN 2-판정 모델(head@press + end@release, 이른릴리스=end 판정·미히트=2 Miss, 분모 CN=2) + 합성 픽스처 5종. `Cn`/`Hcn` 게이트 → LN/Normal byte 불변. 886 통과. (HCN 연속 게이지·CN deferral·BSS = Phase 7; → `docs/reference/cn-hcn-judgment.md`)

**Phase 2 — 첫 릴리스(인프라)**
- [ ] prod 브랜치 생성·푸시 + 브랜치 보호 전략 결정(필수, `docs/acknowledge` 기록)
- [ ] 첫 GitHub Release (v0.1.0 수동 태그, macOS 유니버설+Windows+sha256)

**Phase 3 — 클라 마지막 기능**
- [x] NETWORK 설정 탭 — SERVER URL/PLAYER ID 행(22/23), text_input 인플레이스 편집(Enter 커밋·Esc 취소·버퍼 표시), `PlaySettings` 영속(라운드트립 테스트), `build_server` 추출로 ScoreServer 재구성. 빈 URL=오프라인·빈 ID=guest.

**Phase 4 — 백엔드 MVP + 클라 연동 (별 MIT 레포, 서버 임계경로)**
- [ ] IR 계약 동결(게이트): /api 프리픽스·settings 경로·필드명 reconcile → `docs/acknowledge`
- [ ] 백엔드 부트스트랩 + compose 스켈레톤 (P0/M0) — MySQL·better-auth/Drizzle 소유권 결정 필요
- [ ] 백엔드 M1: health+auth+chart upsert+score submit+ranking/best (네이티브 RAW JSON)
- [ ] 백엔드 M1.5: ranked 정책 + build allowlist + audit
- [ ] 클라 `SubmitResponse` DTO 확장 (ranked/flags/is_new_best/score_id)
- [ ] 클라 M2a: /api 정합·로그인 UI/토큰(token=None 제거)·랭킹 패널 → MVP 루프 완성

**Phase 5 — 서버 P1 + FE 토대 (병렬)**
- [ ] FE 별 MIT 레포 스캐폴드 (Next.js App Router + FSD + TanStack Query v5 + shadcn)
- [ ] FE entities 데이터 레이어 + api-spec envelope 타입 클라 + QUERY_KEY
- [ ] FE mock/stub 백엔드(MSW, envelope)
- [ ] 백엔드 M2: replays(µs)+settings sync+rivals+players/scores
- [ ] 클라 M2-client: 동기화 토글·리플레이 업/다운로드·로그인 영속·rival UI (ir-api.md 갱신)
- [ ] (선택) 설정 화면 마우스 스테퍼 UX

**Phase 6 — 웹 FE 읽기 기능 + FE 엔드포인트**
- [ ] 백엔드 M3: courses+tables+FE 쿼리 엔드포인트+envelope/CORS (FE-전용 4 엔드포인트 응답 스키마 먼저)
- [ ] FE 검색 → 리더보드 → 플레이어 페이지 → better-auth 세션/OAuth → 홈/대시보드

**Phase 7 — 롱테일 / 선택 폴리시 & 하드닝**
- [ ] HCN 연속 게이지(L, 정확성·HCN 희소) · (선택) 스크래치 BSS · (선택) 게이지 5K/PMS
- [ ] (선택/BLOCKED) 결과·메뉴 레이아웃 RON화 · (선택) 테마 chrome 색 완성
- [ ] (선택) 백엔드 M4 LR2IR READ 어댑터 · (선택/BLOCKED) FE 리플레이 뷰어 · (선택) 백엔드 M5 안티치트
- [ ] (선택) macOS 서명/공증·Windows Authenticode · (선택) .app/.dmg+무인자 picker
- [ ] (선택) 코스메틱: 리사이즈 리플로우·BGA 비디오(mpg)·글리프 아틀라스(P3a)·웹폰트(P4) — 판정/점수 영향 0

> 결정 대기(요약): CN/HCN 테스트 코퍼스 범위 · IR 계약 3종+envelope 정책 · 백엔드 MySQL/인증 소유권 · prod 보호+첫 버전 · 서명 비용 · 리플레이 뷰어 전략. 하드 블로커(외부 리소스/권한)는 기본값 문서화 후 우회·계속.

---

## 0. 목표 / 현재 상태

beatoraja(Java/libGDX) **코어 PLAY**를 Rust로 재구현. **완전 플레이 가능 + beatoraja/IIDX 확장 다수 완성.**
흐름: BMS 로드 → (노트옵션 셔플) → 로딩화면 → 하이스피드 스크롤 → 키음 샘플정확 재생 → 입력/판정/게이지 → HUD → 결과 → (옵션)스코어 서버 전송. GUI 곡선택(폴더·난이도표 네비)·설정(탭)·키설정 에디터·리플레이 포함.

판정/게이지/TOTAL은 **beatoraja와 byte/수치 단위로 대조 검증됨**(§6 검증 참고). 미구현은 §7 한계.

## 1. 확정 결정

- **그래픽**: `wgpu 29 + winit 0.30`, 네이티브 인스턴스드 쿼드(`fill_rect`=1인스턴스). 렌더는 `Renderer` trait 추상화(CPU 백엔드 `CpuCanvas`도 존재 → 테스트/예제). 레퍼런스 좌표공간 **1280×720(16:9)** 고정, GPU 유니폼이 NDC로 매핑.
- **오디오**: `cpal 0.17 + symphonia 0.5 + 커스텀 RT 믹서`. **마스터 클럭 = 재생된 샘플 수**(vsync 아님) — beatoraja 판정 vsync 양자화를 구조적 해결.
- **시간 전역 µs(i64)**. 차트 해시 = raw 바이트 MD5+SHA-256(byte-exact).
- **모드/스킨/키맵/설정은 데이터 주도**. 모드=`Mode` 구조체, 스킨=`SkinConfig`(RON), 키=`KeyConfig`(RON), 플레이옵션=`PlaySettings`(RON).
- **키맵 기본 = beatoraja Z열**(사용자 결정). 차트 모드 자동감지로 프리셋 선택, CLI/인앱 오버라이드.
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
  rbms-chart      ← detect_mode·to_model(마디→µs·채널→레인·LN/mine)·scroll(visible/constant_offsets·그린넘버)·shuffle(NoteOption)·count_playable_notes·**note_density**(beatoraja SongInformation 포팅: 1초빈 히스토그램+peak/avg/end).
  rbms-parser     ← BMS lexical: base36/62·Shift-JIS·#mmmCC·#RANDOM/#IF·MD5/SHA-256 → BmsSource.
  rbms-model      ← 순수 타입: Mode·Note·TimeLine·Model·LnKind·NoteKind.
```

## 4. 완료 기능 (전부 검증됨)

**코어:** 파서(727곡 MD5 byte-exact) · 타이밍모델 · 오디오(실디바이스) · 스크롤/렌더 · 판정엔진 · autoplay · 입력 · 라이브 윈도우.
**판정/게이지:** 판정 윈도우(beatoraja `JudgeProperty.SEVENKEYS` 일치) · `#RANK→judgerank`(`[25,50,75,100,125]%`, NORMAL=#RANK2=75%) · 게이지 6종+모디파이어 · 클리어램프(색=beatoraja `LAMP`) · LN 풀판정(head hold→release) · **空POOR(이른 빈 POOR)=노트 미소실·콤보 유지·MS페널티**(beatoraja `judgeVanish[5]=false`·`combo[5]=true`, `empty_poor` 카운터).
**게임옵션:** 모드 자동감지 · **노트옵션**(OFF/MIRROR/RANDOM/S-RANDOM/R-RANDOM/ROTATE, 시드 결정적·DP 사이드분리) · **하이스피드 고정**(FLOATING/CONSTANT 그린넘버) · **판정 오프셋**(±200ms) · **오토 캘리브레이션**(정확판정 평균오차로 결과 시 offset 보정, per-run 수렴) · **JUDGE WIDTH**(judge_rate 50~200% 윈도우 스케일) · **TOTAL**(override/노트수 기본) · lift · lane cover(sudden) · scratch side/auto.
**렌더/UI:** 1280×720 랜드스케이프(필드 center-left+우측 BGA) · 키 빔 · 레인 outline+구분선(opacity/onoff) · **가로 게이지**(레인 아래 0~100, IIDX식) · **다국어 폰트**(cosmic-text+Inter+시스템폴백 → 일/한/중/태/아랍 등 모든 언어 AA 렌더, `font_path`/`--font`로 교체) · 플레이 HUD(콤보·직전판정·FAST/SLOW·판정카운트·EX·게이지) · 결과화면 · 일반/와이드 스킨(RON, 임베드+`--skin`) · BGA(이미지) · BGA on/off.
**셀렉트/설정/키:** GUI 곡선택(재귀 스캔·폴더 네비 Root→ALL SONGS/난이도표→레벨→차트, ↑↓·←상위/→진입·Enter·**마우스 클릭**) · 우측 정보패널+**로컬 기록 인라인 리스트·상세 모달**(`R`/클릭, 리플레이 재생) · 폴더 선택(rfd) · **난이도표 인앱 관리**(다중표 tables.ron·URL텍스트입력/파일rfd 추가·제거) · **설정 탭**(PLAY/GAUGE/JUDGE/DISPLAY/INPUT, 탭/값 **마우스 클릭**) · **통합 키 설정**(파일+인앱 에디터, 레인+조작키 전부 재바인딩·충돌검출) · **설정 영속**(settings.ron, AUTO REPLAY 토글 포함) · **리플레이 저장·재생**(`--replay`/모달, 시드/옵션 복원해 동일 재현) · **로컬 스코어 영속**(scores.ron, 서버 무관).
**IR:** IR 슈퍼셋 클라이언트(스코어 제출, `--server`·`--player`) — DTO 슈퍼셋 확장(JudgeBreakdown **early/late(epg…lms)·avgjudge·empty_poor**, PlayOptions **전체옵션**, ScoreSubmission **seed/algo/rule/skin/`client_build_sha256`(자기-빌드 sha2)·`client_platform`**, ReplayData **µs구조**(`ReplayEvent`), settings/replay-dl/auth/course 메서드 stub). 전부 serde default 후방호환. **백엔드 서버는 설계 완료·구현 후속**.
**ROADMAP 클라이언트 9종(2026-05-31 완료):** 폴더 SCANNING 로딩·**스코어 랭크 그래프(IIDX 9분법 `dj_rank`+랭크바, 결과+셀렉트)**·**점수 ΔEX 비교(직전/베스트)**·**리플레이 분석모드(재생바·일시정지·배속·재시뮬 시크·노트별 ms-off)**·**데이터 주도 HUD 스킨**(판정 팔레트/라벨/게이지 임계·색/요소위치 RON화)·**폰트 P3**(무할당 중첩 캐시·말줄임)·**노트옵션 ALL-SCRATCH/H-RANDOM**(시간임계 40/125ms)·**그린넘버 표시**·**CN/HCN(`#LNMODE`)**·**듀얼필드 14K(P1좌·P2우)**. SCORE GRAPH·REPLAY ANALYSIS 설정 토글. → `docs/history/2026-05-31-roadmap-client-features.md`.
**UI(IIDX/LR2 지향 재설계, 2026-05-31):** 폴더 영속(`songs_folder`)·폴더 스캔 백그라운드 스레드+애니메이션 LOADING·노트 상단 클리핑(`[top_y,judge_y]`, 프레임 상단서 흘러나옴)·플레이 IIDX 레이아웃(좌 정보, 중앙 **라이브 스코어 그래프**, 우 판정카운트/BGA)·결과 IIDX 레이아웃(거대 DJ LEVEL+스코어 리포트, PGREAT 핫핑크)·곡선택 행별 클리어램프 LED·**곡선택 상세 메타 고도화**(부제/아티스트/장르·제작자 + 2열 스탯그리드: BPM범위·DIFFICULTY명·NOTES(+LN)·JUDGE(#RANK명+%)·LENGTH·TOTAL; 포커스 곡만 lazy `to_model`로 노트수/길이/BPM범위 산출, `#MAKER` 파싱, bms-rs 메타 모델 참조). DP(14K)는 BGA 비키도록 좌측 앵커. 레퍼런스 `docs/reference/ui/`·스펙 `docs/reference/ui-design.md`. → `docs/history/2026-05-31-ui-redesign-iidx.md`.
**곡선택 전면 재설계(beatoraja modern chic, 2026-05-31):** 렌더링을 **`rbms-render::render_select`로 추출**(`SelectView`/`SelectHot`, `CpuCanvas` 헤드리스 PNG 검증 가능, `main.rs` 인라인 제거) — 행 KEY/레벨 배지·클리어램프 LED(좌바+우세로바)·포커스 연출(시안테두리+노랑타이틀)·중앙포커스 스크롤, 상세 **커버(`#STAGEFILE`→`#BANNER`, 단일 BGA슬롯 쿼드뒤·포커스당1회 디코드)**+제목블록(2줄 래핑)+2열 스탯그리드+**노트 밀도 히스토그램**(`note_density`, 서브픽셀·긴곡 오버플로없음·PEAK/AVG/END notes/sec)+기록(베스트바·DJ랭크바·최근행 EX+랭크+추세)+기록모달. `main.rs` `build_select_view` 조립 + `SelectKey` 캐시(매프레임 재할당 방지)+hot 매핑. 파서 `#BANNER`/`#PREVIEW` 추가(`#PREVIEW`는 데이터만, 재생 후속). **2라운드 적대적 멀티에이전트 리뷰**(1R 16건 반영·2R 0건). → `docs/history/2026-05-31-select-redesign.md`, 스펙 `docs/reference/ui-select-redesign.md`, 타깃 `…/ui/provided/06-beatoraja-select-target.png`.
**곡선택 후속(2026-05-31): 스탯그리드 잘림 수정·#PREVIEW·KEY BOMB.** (1) 상세 스탯그리드 **2열×3행→3열×2행** 압축(하단 LENGTH/TOTAL이 DENSITY 구분선에 잘리던 것 해결). (2) **`#PREVIEW` 프리뷰 재생 (TODO — 실재생 미동작)** — 포커스 settle(디바운스 20프레임) 시 `#PREVIEW` 디코드+루프(경계 재트리거, mixer `play(key)`가 동일키 먼저 stop하므로 선스케줄 대신 자연종료 후 재발화) 로직·select 전용 `AudioEngine`(플레이 엔진과 분리, `load()` 진입부·select 이탈 시 정지)·`AudioEngine::sample_duration_us`·**DISPLAY 탭 PREVIEW 토글**(`PlaySettings.preview`)까지 배선했으나 **포커스해도 소리 안 남**(추후 디버깅, 코드 유지). (3) **KEY BOMB** — 노트 히트(judge≤3, 空POOR/miss 제외) 시 판정선 판정색 확장+소멸 버스트. `Player.bomb`(press/release/autoplay 전 경로 기록)·`rbms_render::render_key_bomb`·**`SkinConfig` bomb_enabled/height/duration_ms 데이터주도**(기존 RON serde default 호환). 적대적 리뷰 1건(LOW: 리플레이 직접실행 시 cpal 스트림 1프레임 공존) 근본수정. → `docs/history/2026-05-31-select-redesign.md`.
**CI/배포:** GitHub Actions(`.github/workflows/ci.yml`·`release.yml`) — 자동 버전·macOS 유니버설·Windows 빌드·릴리스(→`docs/ci-release.md`). 실제 동작은 GitHub 원격 push 시.

상세 이력 → `docs/history/2026-05-31-*.md` (§8 docs맵). 다음 할 일 → `ROADMAP.md`.

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
- **기본 레인 키(beatoraja Z열)**: 7K=`Z S X D C F V`+LShift(스크) · 5K=`Z S X D C`+LShift · 9K/PMS=`Z S X D C F V G B`(스크없음) · 14K=좌손 P1(`Z S X D C F V`+LShift)·우손 P2(`M K , L . ; /`+RShift).
- **기본 조작 키(플레이 중, 재바인딩 가능)**: ↑/↓=hi-speed · →/←=lane cover · ]/[=lift. Esc=뒤로.
- **CLI 옵션**: `--interactive|--auto --sc-left --sc-auto --lift F --hispeed F --gauge ... --keys Z,S,X,... --skin file.ron --font file.ttf --table URL --keyconfig path.ron --replay file.ron --server URL --player ID`.

## 6. 검증 메모 (판정/게이지 — 사용자 "빡세다" 검증)

- 판정 윈도우 = beatoraja `JudgeProperty.SEVENKEYS`(PG±20/GR±60/GD±150/BD-280~220/MS-150~500 µs @judgerank100) **정확 일치**.
- `#RANK 0~4 → judgerank [25,50,75,100,125]%` = `JudgeWindowRule.NORMAL`과 일치. **#RANK 2(NORMAL)=75%=PGREAT ±15ms**는 beatoraja와 동일.
- 게이지/TOTAL = `deltas * total/notes`(그루브) 일치. 자동미스는 BAD 늦은경계 이후.
- 결론: "빡센" 건 NORMAL 윈도우가 원래 타이트 + 입력/표시 지연 미보정. → **JUDGE OFFSET** 수동 또는 **AUTO CAL**(한 곡 플레이로 자동 보정).

## 7. 알려진 한계 / 다음 후보

- ~~CJK 폰트 없음~~ **해결**(cosmic-text). ~~캐시키 할당~~/~~말줄임~~ **P3 해결**(무할당 중첩 캐시·`fit_text`). **P3a run-length 병합 보류**(프로파일 ROI~11%, AA 텍스트 픽셀별 alpha 상이 → 본 해법은 글리프 텍스처 아틀라스, 후속). **P4 웹폰트(URL)** 후속.
- ~~14K 단일필드~~ **듀얼필드 해결**(`dual_field`, P1좌·P2우·바깥 스크래치, 필드별 judge라인/구분선/게이지 1개). 비활성화(`dual_field:false`) 시 레거시 단일필드.
- ~~ALL-SCRATCH/H-RANDOM 미구현~~ **해결**(시간임계 40/125ms). ~~green-number 미반영~~ **표시·반영**(HUD). ~~CN/HCN 미구분~~ **`#LNMODE`→`LnKind` 구분**(판정 차별화는 후속). 스크래치 회전(2키 교대) 단순화는 잔존.
- 윈도우 리사이즈 UI 리플로우 없음(논리 1280×720 고정). BGA 비디오(mpg) 미지원. 게이지 5K/PMS 변종·judgerank 커스텀 일부 미반영. 난이도표 추가 fetch는 동기(1개씩).
- **IR 백엔드 = 전체 설계 완료(`docs/backend/`, 문서 단계)·구현 후속**. ~~클라 슈퍼셋 확장 필요~~ **클라 DTO 슈퍼셋 확장 완료**(§4 IR). 서버 미구현이라 신규 메서드(settings/replay-dl/auth/course)는 호출 시 `Unsupported`. `PlayOptions.lntype`에 실제 LN모드 전파는 후속(모델에 lnmode 미보유).
- ~~UI 곡선택 재설계~~·~~KEY BOMB~~ **완료**(§4). **#PREVIEW = TODO**(배선·토글·로직 구현했으나 포커스 시 실제 재생 미동작 — 차트 대부분 `#PREVIEW` 미정의 가능성/두 번째 cpal 스트림/디바운스 미도달. 코드 유지, 추후 디버깅 → `ROADMAP.md`). 남은 UI 후속: **스킨 데이터화**(결과/메뉴 패널까지 — 곡선택 추출·KEY BOMB 데이터화로 진척).
- **F5 결과/메뉴 패널 위치 RON화**(현재 HUD 표면만 데이터화), **P3a 글리프 아틀라스·P4 웹폰트**, **CN/HCN 판정 차별화**, **FE 프로젝트**(별 저장소·MIT) 후속 → `ROADMAP.md`. 적대적 리뷰 보류 차이 → `docs/acknowledge/beatoraja-divergences.md`.

> **세션 이력(완료)**: 입력판정 근본수정(空POOR)·폴더 ←→ 네비·마우스·로컬기록 모달·beatoraja 램프색·AUTO REPLAY·Play-Esc-즉시결과·DEBUG MODE → `docs/history/2026-05-31-input-judge-nav-mouse-records.md`. 다국어 폰트(cosmic-text) → `…-multilingual-font.md`. 백엔드 IR-슈퍼셋 설계 + CI/CD → `…-backend-ir-ci.md`. (모두 §4 완료기능·§8 docs맵에 반영됨)

## 8. docs 맵

- `docs/reference/` — mechanics(beatoraja 메커닉)·rust-stack·architecture·wgpu29-winit030-api·ir-api(서버계약)·_appendix-raw·**cn-hcn-judgment**(CN/HCN 판정 beatoraja 대조 구현 스펙·후속)·**ui-design**(IIDX/LR2/beatoraja UI 레이아웃 스펙)·**ui-select-redesign**(곡선택 정밀 스펙: render_select 추출·밀도 포팅·레이아웃)·**ui/**(레퍼런스 스크린샷: provided 6 + fetched 18 + README).
- `docs/acknowledge/` — decisions(확정결정)·beatoraja-divergences(보류차이)·empty-poor-local-scores(空POOR처리·로컬기록 스키마).
- `docs/font-cjk-support.md` — 다국어 폰트 지원(cosmic-text 결정·beatoraja 폰트 파악·P1/P2 완료·P3/P4 후속).
- `docs/backend/` — **백엔드 IR-슈퍼셋 서버** 설계: README·PRD·api-spec(전 엔드포인트)·data-model(Drizzle)·endpoint-tasks·compatibility(LR2IR/beatoraja 매핑+출처)·**contract-freeze**(Phase 4 게이트: 클라 경로 대조·계약 동결·M0 단계). (Hono/Bun/Drizzle)
- `docs/ci-release.md` — GitHub Actions(ci/release): 자동 버전·macOS 유니버설·Windows 빌드·릴리스. 주의(원격/브랜치보호/LICENSE/서명).
- `docs/bug/` — 버그/진단 기록(예: `2026-06-03-preview-playback` #PREVIEW 계측·검증절차).
- `docs/memory/` — test-library(라이브러리 실측).
- `docs/history/` (시간순):
  - `m0-m1-parser-chart` · `m2-audio` · `m3-render-scroll` · `m6-judge-m4-autoplay` · `m7-live-window` — 코어 마일스톤.
  - `features-mode-gauge-ln-result-gpu-config-select` · `skin-bga-ir` · `font-hud-gui` — 1차 확장.
  - `beam-keymap-720p-table` — 키빔·모드별키맵·720p·재귀선택·난이도표.
  - `options-settings-folders` — 노트옵션·설정영속·게이지·BGA·하이스피드고정·오프셋·폴더선택·리플레이·다중표.
  - `judge-verify-tuning-skins` — 판정검증·JUDGE WIDTH/TOTAL·설정탭·일반/와이드스킨·레인장식·가로게이지·오토캘.
  - `input-judge-nav-mouse-records` — 空POOR 판정 근본수정·폴더 ←→ 네비·마우스 입력·로컬 기록(scores)+상세 모달·beatoraja 램프색·AUTO REPLAY·Play-Esc-즉시결과·DEBUG MODE.
  - `multilingual-font` — 다국어 폰트(cosmic-text+Inter+시스템폴백, 모든 언어 AA)·`font_path`/`--font` 교체·beatoraja 폰트 파악.
  - `backend-ir-ci` — 백엔드 IR-슈퍼셋 전체 설계(LR2IR+beatoraja IR+설정동기화·µs리플레이·빌드해시무결성·FE) + GitHub Actions(자동버전·macOS유니버설·Windows)·라이선스/서명 정리.
  - `roadmap-client-features` — ROADMAP 클라이언트 9종: 폴더 SCANNING 로딩·스코어 랭크 그래프(IIDX 9분법)·점수 ΔEX 비교·리플레이 분석모드·데이터 주도 HUD 스킨·폰트 P3(캐시·말줄임)·IR 슈퍼셋 DTO·자기-빌드 SHA-256·노트옵션(ALL-SCR/H-RAN·green-number·CN/HCN·듀얼필드14K). 웨이브별 적대적 멀티에이전트 리뷰.
  - `ui-redesign-iidx` — IIDX/LR2 지향 UI 재설계: 폴더 영속·백그라운드 스캔(애니 로딩)·노트 상단 클리핑(근본 수정, 베젤 hack 제거)·플레이 IIDX 레이아웃+라이브 스코어 그래프·결과 IIDX 레이아웃·곡선택 클리어램프 LED+상세 메타 고도화(bms-rs 참조, `#MAKER`). DP 레이아웃 BGA 비킴. 적대적 리뷰로 DP충돌·stale폴더·perf 수정. 레퍼런스 수집·디자인 스펙.
  - `select-redesign` — 곡선택 전면 재설계(beatoraja modern chic): `rbms-render::render_select` 추출(헤드리스 검증)·`note_density`(SongInformation 포팅)·커버(단일 BGA슬롯)·KEY/레벨 배지·밀도 히스토그램·타이틀 2줄·`build_select_view` 캐시·파서 `#BANNER`/`#PREVIEW`. 2라운드 적대적 리뷰(16건→0건).
- **새 세션 진입점 = 이 PROCESS.md**(CLAUDE.md가 지정). 별도 글로벌 하네스 메모리는 사용 안 함 — SSOT는 docs/.

## 9. 작업 규칙(요약)

각 스텝 검증 후 진행(테스트/렌더 PNG). 모호·범위변경 시 멈추고 질의. 큰 변경 후 적대적 멀티에이전트 리뷰. 커밋은 사용자 요청 시에만, **co-author 미포함**. `target`/`Cargo.lock`/라이브러리 차트 커밋 금지(.gitignore).
