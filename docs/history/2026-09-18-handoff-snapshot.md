# HANDOFF — 2026-09-18 세션 스냅샷

> 새 세션의 단일 진입점. 대응 코드: `dev` 브랜치, 기준 커밋 `4537ad6`(`fix(skin): 기본 화면 배치와 합성 수정`) 위의 **미커밋 작업 트리**(코드·자산 변경 48 + 신규 22, 문서 다수). 이전 스냅샷(2026-09-10)은 `docs/history/2026-09-10-handoff-snapshot.md`. 이 문서는 항상 덮어쓴다.

## 1. 프로젝트 한 줄 정의

R-BMS: 레퍼런스 구현(이름은 저장소에 적지 않음)의 PLAY 코어를 클린룸 포팅한 Rust BMS 리듬게임 플레이어(워크스페이스 크레이트 + `apps/rbms-player`·`rbms-cli`) + `web/` Next.js IR 서버·사이트. 이번 세션은 플레이어의 **스킨 시스템**만 다뤘고 `web/`은 손대지 않았다.

## 2. 현재 목표

- **최종 목표(사용자 원문 요지, 2026-09-17)**: "현재 프로젝트를 확인하고, 외부 스킨 묶음과 첨부 화면 11장을 참고해, 완벽하게 현재 프로젝트의 default 스킨과 스킨을 위한 구조, 그리고 커스터마이징 가능하게, docs/에 추가기능이 필요하면 추가기능 넣고 잘 구현하도록." 하위 목표 4개: (a) 기본 스킨 전면 제작 (b) 스킨 구조(문서 계약·렌더 객체) 완성 (c) 커스터마이징(번들 단위 옵션·파일·오프셋) (d) 필요한 기능은 `docs/`에 사양을 두고 구현.
- **현재 마일스톤**: `docs/PROCESS.md` "스킨 시스템 완성과 기본 스킨 전면 제작" K1~K6 중 K1~K5 완료, **K6(커밋) 대기**.
- **직전 작업**: 사용자의 `/prepare-new` 지시로 이 핸드오프 문서와 문서 정합성 정정(§9 문서 지도)을 수행했다. 그 직전에는 최종 보고(구현 요약, 게이트 실측, 남은 항목, 커밋 단위 5건 제안, personal-llm 갱신 여부 질문)를 보냈고 사용자 답은 아직 없다.

## 3. 완료 / 진행 중 / 미착수

### 3.1 완료 (전부 미커밋, 게이트 실측 통과)

게이트(2026-09-17 K5 실측): `cargo fmt --all --check` 통과, `cargo clippy --workspace --all-targets --all-features -- -D warnings` 통과, `cargo test --workspace` 3,181 통과·0 실패·ignored 3, `git diff --check` 통과, 금지 명칭 grep 0건(2026-09-18 재확인: diff-check·금지 명칭 동일).

| 영역 | 내용 | 파일 |
| --- | --- | --- |
| 스킨 문서 모델·로더 (`crates/rbms-skin`) | `SkinDef.replace`, `hotspot`(`HotspotDef`, `HOTSPOT_ACTIONS` 8종 검증), `scope: 'bundle'`(`CustomFile.scope`), `densitygraph` 객체, 중첩 트랙 조립 `NestedTracks`/`JudgeTracks`/`SongListTracks`, `LoadedSkin::replace_names()`, 조건부 원소·include 분기 | `src/model.rs`, `src/model/objects.rs`, `src/loader.rs`, `src/resolve.rs`, `tests/skin_loader.rs`, `tests/skin_resolve.rs`, 신규 `tests/skin_nested.rs`, `tests/fixtures/nested/**` |
| 렌더러 (`crates/rbms-render`) | 객체 13종 추가(Note·Gauge·Judge·SongList·HiddenCover·LiftCover·GaugeGraph·JudgeGraph·BpmGraph·TimingDistribution·TimingVisualizer·HitError·Density, 기존 8종과 합쳐 21종), `FrameExtra` 상태, 사설 id 20001~20315, `PlayViewState` 확장(BPM·시간·FAST/SLOW·목표·레벨·아티스트·게이지 종류 옵션·NOW 랭크), `SelectViewState::new`(통계 6셀·기록 5줄), `ResultViewState.with_hint/with_ir_status`, `DecideChart`, `content.rs`(`PlayContent`/`SelectContent`/`ScreenContent`), `render_hud_with_content`, `render_select_on_background_with_content`, `ResultExtras`/`ResultContent`/`ResultView.artist`, `Skin::with_document_lanes`, 이중 필드 좌측 BGA 판별(`DUAL_LEFT_FRACTION`·`DUAL_MARGIN`·`DUAL_LEFT_BGA_LIMIT`) | `src/skin_render/{mod,object,draw,state,screen,tests}.rs`, 신규 `src/skin_render/{notes,gauge,judge,songlist,covers,graphs,color,tests_play_objects,tests_list_graphs}.rs`, 신규 `src/content.rs`, `src/{hud,result,select,skin,lib,ctx}.rs`, `examples/render_result.rs`, `tests/{golden_result,skin_render}.rs` |
| 설정 (`crates/rbms-config`) | `SkinOptions.shared`(번들 키별 `SkinCustomisation`), `bundle_key`, `shared_customisation/shared_customise`, `move_shared`, `user_config(path)` 병합 | `src/schema.rs`, `src/tests.rs` |
| 앱 배선 (`apps/rbms-player`) | `SkinScreen`: `screen_content(screen)` 컴파일 시 캐시, `replacement(screen, name)` 대체 표(play 8·select 4·result 9), `MergedOffsets`, `skin_hotspots`, `skin_draws_background`; 화면별 상태 조립(`PlayObjectState`·`SelectListState`·`ResultSeriesState`·`OptionsRows`), `PlayTimers`(KEYON/KEYOFF/HOLD/BOMB, 1P/2P, `judged_side`)·`ResultTimers`(150/151/152)·PANEL1; 플레이 문서가 사용자 오프셋을 읽음(회귀 테스트 `a_play_document_reads_the_nudges_the_player_stored_for_it`); 문서가 `bga`를 그리면 네이티브 BGA 사각형 생략; 곡 목록·핫스팟 클릭 → `Hot`; SKIN 탭 `BUNDLE` 그룹 행·`shared_customise` 저장·RESET; `BUNDLE_GENERATIONS`(v1·v2·v3) 설치·세대 이동 시 `custom`·`shared` 이동·`bundle_is_complete`/`bundle_can_draw`; 번들 `sound/`를 시스템 사운드 폴더로(`sound_folder_path`); 결과 `ChartRun.artist` | `src/{skin_screen.rs, skin_screen/tests.rs, app_options.rs, assets.rs, syssound.rs, skin_select.rs, skin_select/{fixtures,tests}.rs, settings_ui.rs, lib.rs, app_play.rs, app_input.rs, app_result.rs}`, `src/stage/{mod.rs, loading.rs, result.rs, settings.rs, settings/skin_tests.rs, render_tests.rs, render_tests_skin.rs, play/mod.rs, play/tests.rs, select/mod.rs, select/tests.rs}`, 신규 `src/stage/render_tests_skin_v3_{select,decide_result,play_sp,play_dp}.rs` |
| 기본 번들 (`assets/skins/steel-neon-v3/`, 66 파일 전부 `assets.rs` 등록) | `select/decide/result.json5`, `play-{5,7,9,10,14,24}k.json5`(24k는 준비 파일), `shared/objects-play.json5`(PLAY SIDE × GRAPH POSITION 4조합)·`shared/judge-sp.json5`, `play.ron`·`play-dual.ron`·`theme.ron`, `palette.json`, `tools/generate-assets.py`·`tools/generate-sounds.py`(PEP 723, `uv run`), `images/` 27장(notes·digits-s/m/l 11셀·digits-f 11×2·ui·covers·배경 8·프레임 7 + 슬롯 폴더 notes/covers/select/decide), `sound/` 22 WAV | `assets/skins/steel-neon-v3/**` |
| 문서 | 사양·결정(D1~D23)·가이드·QA 체크리스트+캡처 12장·이력·PROCESS·docs 지도·README 스킨 문단, 그리고 이번 핸드오프의 정합성 정정(§9) | `docs/**`, `README.md` |

### 3.2 진행 중 — K6 커밋 (2026-09-18 사용자 지시로 실행)

- 커밋 완료: `1535b0a` feat(skin) · `4f1a338` feat(config) · `3704b8c` feat(render) · `3cd72f4` feat(player)+v3 자산 · docs(skin)(이 문서를 포함하는 다음 커밋). 각 커밋 author 단독·트레일러 0건 확인. 이어서 `origin/dev` 푸시와 남은 기능 구현(§8 TODO 3·4)을 같은 지시로 시작한다.

### 3.3 미착수

- 실제 GPU 창 확인(사용자 몫): 화면 녹화 권한 부여 후 `docs/quality-assurance/2026-09-17-skin-system/checklist.md` §3 절차. 결과 캡처는 같은 폴더 `captures/live-*.png`.
- 기본 번들 미세 결함 4건과 스킨 후속 기능(사양 §12) — §8 TODO 3·4.
- `~/personal-llm` 갱신 여부(사용자 답 대기) — §6.

## 4. 의사결정 요약 (상세: `docs/acknowledge/2026-09-17-skin-system-decisions.md`)

채택(각 항목의 기각 대안은 결정 문서에): D1 번들 3세대·라벨 유지 / D2 중첩 트랙은 로더 조립 / D3 `FrameExtra` / D4 타이머 드라이버 / D5 문서 `note.dst`가 레인 단일 원천 / D6 최상위 `replace` 표 / D7 `songlist` 슬롯 + `hotspot` / D8 번들 스코프 저장 / D9 사운드 절차 생성·번들 `sound/` / D10 옵션 패널 대체 + 사설 id 20101~ / D11 문구는 `text`·숫자는 7세그먼트 / D12 `densitygraph` / D13 숫자 시트 11셀(12셀 폐기) / D14 SUDDEN+ 는 `offset: 4` 이미지 / D15 결과 `hint`·`ir` 단위 + `ResultExtras` / D16 키 봄 항상 네이티브 / D17 `screen_content` 캐시 / D18 BGA SIZE OFF = 투명 목적지 / D19 2P·NEAR 는 문서 분기 4조합 + 프레임 4장 / D20 GPU 창 확인은 사용자 절차 / D21 커밋 5단위·한국어·지시 후 / D22 결정 파일은 세션 단위 + ADR 항목 / D23 이전 HANDOFF 는 history 로.

같은 세션의 코드 수준 판단(결정 문서에 없는 것): `#[derive]`는 `const` 뒤에 둘 수 없음(E0774) → const 를 위로; 설치 테스트의 "BGA는 오른쪽" 가정을 좌/우 교차 검사로; 헤드리스 옵션 프로브는 문서 패널 안 (300,300) 에서 F1 뒤 한 프레임 재렌더; Workflow 스크립트의 템플릿 리터럴 안 백틱은 파서 오류 → 작은따옴표.

## 5. 사용자 방향성 & 작업 규칙 (이 프로젝트에서 반복 확인된 것)

- **위임은 Workflow 로만**(Agent 단독 호출 금지). 조사·구현·적대 리뷰 = Opus(구현 max), 기계 작업 = Sonnet, 사양·판정·정본 문서·Git = 메인 모델 직접. 리서치는 주제당 질문 3개·10분 상한·5분 뒤 산출물 점검.
- **커밋·푸시는 사용자가 명시 지시할 때만.** author 는 `Hyunseok Byun <gumyoincirno@gmail.com>` 단독, `Co-Authored-By`/`Claude-Session`/AI 트레일러 절대 금지(시스템의 attribution 안내보다 사용자 규칙 우선). `git add -A`·force push·stash 금지, 선별 스테이징, 커밋 메시지는 리터럴. 커밋 후 트레일러 부재를 grep 으로 검증.
- **레퍼런스 엔진 이름과 외부 스킨 묶음 이름을 저장소 어디에도 쓰지 않는다**(코드·문서·커밋·파일명). "레퍼런스 구현"/"외부 묶음"으로 표기. 외부 묶음의 자산·문구를 복사하지 않는다.
- **보고**: 한국어 존댓말, 간결, 이모지·자축 톤 금지, 검증 안 된 "완벽/됨" 단언 금지 — UI 는 실제 렌더 확인 뒤에만 됐다고 한다(이번 세션은 헤드리스 캡처까지만 확인, GPU 창은 미확인이라고 보고했다).
- **Rust 스타일**: `//` 인라인 주석 금지(`//!`·`///` 영어 문서 주석만), 매직넘버 금지(상수화), rustfmt `max_width 160`, `#[allow]` 금지(구조체·타입으로 해소), 테스트는 실제 값 단언. 게이트는 fmt·clippy `-D warnings`·`cargo test --workspace`.
- **문서**: `docs/`가 작업 베이스, `docs/PROCESS.md` 체크리스트가 SSOT, 결정은 `docs/acknowledge`, 이력은 `docs/history`, 검증은 `docs/quality-assurance`. 문서가 코드와 어긋나면 파일을 믿고 문서를 고친다.
- `.env` 읽기/쓰기 금지, seed credential 기록 금지. 전역 규칙 "README.md 는 요청 없이 손대지 않음"은 이번 지시 범위(프로젝트 전체 완성)에서 스킨 문단 갱신으로 예외 적용됐다 — 추가 README 수정은 다시 묻는다.
- 전역 CLAUDE.md 는 세션 말미에 `~/personal-llm` 갱신 여부를 묻도록 하고, 2026-09-10 세션 기록은 "질문 생략"이라 적혀 있다. 이번 세션은 한 번 물었고 답이 없다(§6).

## 6. 미해결 질문 / 사용자 확인 필요

1. **커밋 승인** — §8 TODO 1 의 5단위로 커밋할지(푸시 포함 여부도). 지시 전까지 미커밋 유지.
2. **`~/personal-llm` 갱신** — 이번 세션의 규칙·결정(Workflow 전용 위임, 스킨 시스템 결정, 리서치 상한)을 개인 문서(Rust 도메인은 별도 파일)에 반영할지.
3. **화면 녹화 권한** — 실제 GPU 창 확인을 사용자가 직접 할지, 권한을 준 뒤 다음 세션이 할지.
4. **커밋 설명 언어** — 최근 커밋에 맞춰 한국어로 잡았고 사용자 확인은 없다(`2026-09-17-commit-language.md`).

## 7. 환경 & 전제

- macOS(Darwin 25.6, arm64), Rust `1.95.0`(`rust-toolchain.toml`, rustfmt·clippy 포함), edition 2024, `rustfmt.toml` max_width 160. `uv` 설치돼 있어 번들 생성 스크립트는 `uv run assets/skins/steel-neon-v3/tools/generate-assets.py`(PEP 723). CI `.github/workflows/ci.yml`(3-OS 테스트 + fmt/clippy 게이트), `release.yml`.
- 앱 설정 `~/.config/rbms/`(settings.ron·keyconfig.ron·skin/…). 실행 검증은 사용자 설정을 건드리지 않도록 격리 HOME: `mkdir -p /tmp/rbms-live && HOME=/tmp/rbms-live ./target/release/rbms-player "<곡 폴더>"`(`cargo build --release -p rbms-player` 선행). 첫 실행이 `skin/steel-neon-v3/`를 설치한다.
- 헤드리스 캡처: `RBMS_SKIN_CAPTURE_DIR=<폴더> cargo test -p rbms-player the_current_default_bundle_keeps_information_and_chart_art_visible --lib`(9장) 및 `... render_tests_skin_v3 --lib`(v3 장면). 캡처는 1280×720 `CpuCanvas` 픽셀이며 GPU 창과 동일성은 미검증.
- 좌표 전제: 문서는 1280×720 y-up, `composition: 'layered'`(background → 내장 → foreground), 프리셋 라벨 `STEEL NEON`, 사설 id 20001~20315, 핫스팟 8 action 중 상단 바는 6개, 옵션 패널 11행.
- 세션 스크래치(저장소 밖, 임시 디렉터리라 사라질 수 있음): `/private/tmp/claude-501/-Users-gkn-R-BMS/2073940e-b626-4827-b345-98972e29b738/scratchpad/` — `skin-research/r1-engine-gap.md`·`r2-app-wiring.md`·`r3-reference-layout.md`(조사 보고서, 외부 묶음 이름이 들어 있어 저장소에 넣지 않음; 핵심 수치는 사양 §9·이력 K1), `k4/{k4.js,k5.js,commit-plan.md,captures*}`. Workflow 실행 id: K1 `wf_32fd88cf-707`, K3 `wf_b03ec12f-3f9`, K4 `wf_1a582c5d-df7`, K4-b `wf_136680dc-fb5`.
- 입력 자료: 외부 스킨 묶음은 사용자의 다운로드 폴더(이름 기록 금지), 참고 화면 11장은 대화 첨부라 디스크 경로가 없다. 재사용이 필요하면 사용자에게 다시 요청한다.
- `web/`·IR 서버·릴리스(Phase R)는 이번 세션과 무관하며 상태는 `docs/PROCESS.md`·`docs/roadmap.md` 그대로.

## 8. 다음 세션 TODO (우선순위)

1. **K6 커밋(사용자 지시 후)** — 순서와 파일:
   1. `feat(skin): 문서 대체·핫스팟·번들 스코프·중첩 트랙 로더` — `crates/rbms-skin/src/{loader.rs,model.rs,model/objects.rs,resolve.rs}`, `crates/rbms-skin/tests/{skin_loader.rs,skin_resolve.rs,skin_nested.rs}`, `crates/rbms-skin/tests/fixtures/nested/`
   2. `feat(config): 스킨 커스터마이즈 번들 공유 스코프` — `crates/rbms-config/src/{schema.rs,tests.rs}`
   3. `feat(render): 스킨 객체 13종과 화면 상태 확장` — `crates/rbms-render/src/{content.rs,ctx.rs,hud.rs,lib.rs,result.rs,select.rs,skin.rs}`, `crates/rbms-render/src/skin_render/`(전체), `crates/rbms-render/examples/render_result.rs`, `crates/rbms-render/tests/{golden_result.rs,skin_render.rs}`
   4. `feat(player): 스킨 대체 게이트·상태·타이머·설치·사운드와 기본 번들 v3` — `apps/rbms-player/src/`(§3.1 표의 파일 전부, 신규 4 테스트 파일 포함), `assets/skins/steel-neon-v3/`
   5. `docs(skin): 스킨 시스템 사양·결정·가이드·QA·이력·핸드오프` — `docs/`(변경·신규 전부), `README.md`
   완료 조건: 5 커밋 모두 author 단독·트레일러 0건, 마지막 커밋 뒤 fmt·clippy·`cargo test --workspace` 통과. 중간 커밋 각각의 단독 빌드는 미검증이며 stash 는 금지이므로, 필요하면 `git worktree`로 중간 커밋을 검사한다. 푸시는 별도 지시.
2. **실제 GPU 창 확인** — 체크리스트 §3(사용자 권한 필요). 발견 결함은 `docs/bug/`에 기록 후 문서·자산 수정.
3. **기본 번들 미세 결함** — (a) 10키 전용 `frame-dp` PNG(`tools/generate-assets.py`, `play-10k.json5`), (b) 플레이 상단 NOTES 수치 1~2px 경계 물림(`shared/objects-play.json5` 4조합 동시 수정), (c) 선택 화면 판정별 카운트 행(브라우저 상태에 id 없음 → 사설 id 추가 필요), (d) BGA SIZE OFF 를 `bga` 선언 게이트로 바꿀지(D18).
4. **스킨 후속 기능(사양 §12)** — BGM 루프, `pmchara`·`practice`·`skinpreview`·`customEvents`·`customTimers`, 객체 `click`/`act`, 외부 CSV 스킨 로드, 비디오 BGA, 24키 활성화, 옵션 패널 마우스.
5. **personal-llm 갱신**(사용자 승인 시) — Rust 도메인 파일에 이번 결정·규칙 요약.

## 9. 문서 지도

- `docs/HANDOFF.md` — 이 문서(진입점). `docs/README.md` — 문서 표.
- `docs/PROCESS.md` — 체크리스트 SSOT. 최상단 "스킨 시스템 완성" K1~K6, 그 아래 같은 날의 V/G/S 작업(K 에 흡수), 이전 Phase.
- `docs/plan/2026-09-17-skin-system-completion.md` — 엔진·번들 사양 §1~§12(2026-09-18 정정 반영). 이전 사양 `2026-09-17-skin-object-spec.md`·`skin-visual-correction.md`·`default-skin.md`는 이 사양이 흡수.
- `docs/acknowledge/2026-09-17-skin-system-decisions.md` — D1~D23. `2026-09-17-commit-language.md` — 커밋 한국어. `2026-09-09-enhancement-decisions.md`·`reference-divergences.md` — 이전 결정·발산 대장.
- `docs/skin.md` — 스킨 제작 가이드(폴더·문서 계약·대체 표·핫스팟·사설 id·번들 옵션·제약·사운드·편집). `docs/theme.md` — `theme.ron`.
- `docs/history/2026-09-17-skin-system-completion.md` — K1~K6 실측 기록과 남은 범위. `2026-09-17-default-skin.md`·`2026-09-17-skin-visual-correction.md` — v1/v2 세대 기록. `2026-09-10-handoff-snapshot.md`·`2026-09-10-session-wrap-up.md` — 과거 스냅샷.
- `docs/quality-assurance/2026-09-17-skin-system/checklist.md` + `captures/`(12 PNG) — 자동 검사 완료, §3 GPU 창 미실행. `2026-09-17-default-skin/` — v2 옛 캡처.
- `docs/architecture.md`·`docs/crates.md`·`docs/development.md` — 구조·크레이트 API·개발 절차(스킨 절 2026-09-18 갱신). `docs/roadmap.md` — 남은 작업(스킨 후속 포함).
- `docs/feedback/2026-09-17-skin-visual-verification.md` — "파싱·설치 검사만으로 시각 완료를 판단하지 말 것" 교정.
- `docs/utils/workflows/` — 과거 Phase 워크플로 스크립트(이번 세션 스크립트는 스크래치에만 있음).

## 10. 복기 신뢰도

- 이 세션은 도중에 컨텍스트 요약(compaction)이 한 번 있었다. 요약 이전 구간(조사·사양 확정·K3 엔진 구현·K4 배선 초반)은 요약본과 `docs/history`·`docs/plan`·git 작업 트리로 재구성했다. 결정·파일·수치는 2026-09-18 에 실제 파일을 다시 열어 확인했고, 대화 문구 자체의 정확도는 요약본에 의존한다.
- 게이트 수치(3,181 통과)는 2026-09-17 K5 실측이며 2026-09-18 에는 `git diff --check`·금지 명칭 grep 만 재실행했다(코드는 그 사이 바뀌지 않았다).
