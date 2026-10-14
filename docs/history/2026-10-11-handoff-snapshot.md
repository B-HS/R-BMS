# HANDOFF — 2026-10-11 세션 스냅샷

최종 갱신 2026-10-11 · 대응 커밋: d4cd23f 이후 웨이브 8C(작업 트리 기준, 커밋 전). 이전 스냅샷은 `docs/history/2026-10-09-handoff-snapshot.md`(구현 직전 상태), 그 앞은 `docs/history/2026-09-18-handoff-snapshot.md`. 이 문서는 항상 덮어쓴다.

새 세션의 진입점이다. 읽는 순서: 이 문서 → `docs/PROCESS.md` 상단 "현재 작업" 절 → 사양 `docs/plan/2026-10-09-lua-skin-compat.md` 의 해당 웨이브 표.

## 1. 프로젝트 한 줄 정의

R-BMS 는 레퍼런스 구현(beatoraja)의 PLAY 코어를 Rust 로 옮긴 BMS 리듬 게임 플레이어다(Cargo 워크스페이스 크레이트 15개 + 앱 2개, edition 2024, Rust 1.95.0, GPL-3.0-or-later, wgpu 렌더, 샘플 클럭 오디오).

## 2. 현재 목표

| 단계 | 내용 |
| --- | --- |
| 최종 목표 | beatoraja 용 풀 Lua 스킨(ModernChic)을 수정 없이 읽어 같은 배치·타이밍으로 그리고, 첫 실행부터 자작 기본 스킨으로 선곡 → 결정 → 플레이 → 결과 전 화면이 그려진다(사양 §1) |
| 현재 마일스톤 | L5 기본 스킨, 웨이브 8C(W8-1 설정 행 정리·기기 로컬 경로·레터박스, W8-3 내장·설치·릴리스·고지, W8-4 폴백, W8-5 테스트 재편, W8-6 문서) |
| 직전 작업 | 8C 구현 단위 A(내장·설치·기본 선택·폴백), B(설정 행 표지·동기화·레터박스), C(제3자 고지·릴리스·CI 시간), D(게이트·일반화), 수정 단위(리뷰 13건 중 10건)가 작업 트리에 끝났고, 이 문서를 포함한 문서 단위(W8-6)가 끝났다. 커밋·푸시는 아직 하지 않았다 |

## 3. 완료 / 진행 중 / 미착수

### 3.1 완료 (커밋됨)

| 범위 | 결과 | 주요 경로 |
| --- | --- | --- |
| L1~L2 조사·사양·결정 | 보고서 16개, 사양, 결정 D1~D10 | `docs/reference/skin-compat/`, `docs/plan/2026-10-09-lua-skin-compat.md`, `docs/acknowledge/2026-10-09-lua-skin-compat-decisions.md` |
| 웨이브 1 철거와 기반 | 구 번들·혼합 합성 삭제, Lua 5.2, 타이머 µs, 장면 시계, GPU 논리 크기, 해상도 설정, START/SELECT, 캡처 하니스 | `crates/rbms-skin/`, `crates/rbms-render/src/skin_render/`, `apps/rbms-player/src/stage/capture.rs`, `scene_clock.rs` |
| 웨이브 2 Lua 런타임과 로더 | 샌드박스 런타임, 2패스 로더, `SkinHost`, `skin-dump`, 스킨 팩 폴더 | `crates/rbms-skin/src/lua/`, `apps/rbms-cli/src/skin_dump/`, `apps/rbms-player/src/skin_select.rs` |
| 웨이브 3~6 화면 | 결정(Decide Stage), 결과·코스 결과, 선곡, 플레이 5/7/10/14키 | `apps/rbms-player/src/stage/{decide,result,course_result}.rs`, `stage/select/`, `stage/play/`, `skin_host/` |
| 웨이브 7 동영상·비트맵 폰트 | OpenH264 + `re_mp4` 디코더, `.fnt`, distance field | `crates/rbms-video/`, `crates/rbms-render/src/bitmap_font.rs` |
| CI 복구 | 세 OS 통과(run `38077937158`), 이후 통과 run `38084893496` | `.github/workflows/ci.yml` |
| 웨이브 8A | NEXT_REPLAY 제거(D8), 기본 스킨 설계서·골격·결정 화면 | `docs/plan/2026-10-11-default-skin-design.md`, `assets/skins/rbms-default/` |
| 웨이브 8B | 기본 스킨 선곡·플레이 5/7/9/10/14키·결과·코스 결과, 엔진 수정 2건 | `assets/skins/rbms-default/`, `apps/rbms-player/src/stage/result.rs`, `stage/select/list.rs` |

### 3.2 진행 중: 웨이브 8C (작업 트리, 커밋 전)

| 단위 | 내용 | 경로 |
| --- | --- | --- |
| A 내장·설치 | `build.rs` 가 기본 팩(65개 파일, 1,720,337 B)을 표로 내장, `install_default_pack` 가 `<설정 폴더>/skin/rbms-default/` 에 `install-manifest.txt` 기준으로 설치·갱신, 기본 선택(`RBMS_SKIN_PACK` > `skin.pack` > 기본 팩), PACK FOLDER 행의 `DEFAULT (<경로>)` 표시, 실패 문구 `skin <파일> did not load, drawing the built-in screen: <이유>`, `BUNDLED_FONT` 를 `pub static` 으로 | `apps/rbms-player/build.rs`, `src/assets/default_pack.rs`, `src/skin_select.rs`, `src/skin_screen.rs`, `crates/rbms-render/src/font.rs` |
| B 설정 | 스킨이 못 읽는 설정 행에 표지(`BUILT-IN SCREENS ONLY`, `SKIN OPTION COMES FIRST`), `skin` 그룹 전체 기기 로컬(계정 동기화 제외), `DEFAULT_LETTERBOX` = true | `crates/rbms-config/src/{settings,schema}.rs`, `apps/rbms-player/src/{settings_ui,settings_view,ir_sync}.rs`, `stage/settings.rs` |
| C 고지·릴리스 | `THIRD-PARTY-NOTICES.md`(OpenH264 BSD-2, Inter OFL, Lua, SQLite, Rust 크레이트 목록과 전문), 릴리스 압축에 고지 동봉(없으면 실패), CI 제한 시간 test 90→30분 lint 20→15분 | `THIRD-PARTY-NOTICES.md`, `.github/workflows/{ci,release}.yml`, `docs/ci-release.md` |
| D 게이트 | 환경 변수 없이 도는 기본 스킨 게이트(엄격 덤프, 9종 화면 그리기, 금지 API 스캔, 시트 대조), ModernChic 전용 테스트 일반화, `notify.rs` 테스트 보호 | `apps/rbms-cli/src/skin_dump/tests.rs`, `apps/rbms-player/src/stage/capture/gate.rs`, `crates/rbms-skin/tests/default_pack.rs`, `crates/rbms-render/tests/skin_external.rs` |
| 수정 | 리뷰 13건 중 10건: 조용한 건너뛰기, 대소문자만 바뀐 이름, 팩 안 링크 폴더, `*.rbms-partial` 청소, 알림 2회, `--skin` 규칙, SCORE GRAPH 도움말, 표지 판정, 고지 7절, 금지 API 스캐너 | 위 경로들 |
| 문서(W8-6) | `docs/skin.md` 재작성, `architecture.md`·`crates.md`·`development.md`·`README.md` 갱신, 이 문서, 구 문서 표지, 수동 확인 목록 | `docs/` |

게이트 기록(수정 단위 시점): `cargo fmt --all --check` 통과, `cargo clippy --workspace --all-targets --all-features -- -D warnings` 통과, `cargo test --workspace --no-fail-fast` 4,878 통과·0 실패·5 ignored, 동영상 끈 구성 `cargo test -p rbms-player --no-default-features` 통과, 기본 팩과 ModernChic 을 `RBMS_SKIN_PACK` 으로 준 전체 테스트 통과, `skin-dump assets/skins/rbms-default --strict --frames 120` 통과. 이 기록은 각 단위 보고서의 값이며 메인이 통합 때 현재 트리에서 다시 확인한다.

### 3.3 미착수

| 항목 | 내용 | 경로 |
| --- | --- | --- |
| 8C 통합 | 메인의 게이트 재확인, 논리 단위 커밋·푸시, 푸시 뒤 세 OS CI 확인 | §8 |
| L6 최종 검증 | 덤프 pcall 실패 0, 화면별 캡처를 `m2`~`m5` 레이아웃과 대조, 실제 GPU 창 확인 | `docs/PROCESS.md` L6, `docs/quality-assurance/2026-10-11-default-skin-manual-check.md` |
| L7 마감 | 조사 문서·PROCESS·history 정리 | `docs/PROCESS.md` L7, `docs/reference/skin-compat/` |
| 웨이브 9 | customTimers/customEvents, IR 랭킹 목록 연결, 24키 문서, type 9 스킨 설정 Stage, skinpreview, BGA 동영상, 종류별 노트 수·밀도·주 BPM, `filter != 0` 셰이더 | 사양 §6 웨이브 9 |
| Phase R | `prod` 생성, 첫 릴리스, 서명. 실제 키가 필요해 사용자와 함께 한다. 이번 작업과 무관 | `docs/roadmap.md`, `docs/ci-release.md` |
| `web/` IR 서버 | 이번 작업과 무관. 상태는 `docs/roadmap.md` 그대로 | `web/`, `docs/web/` |

## 4. 의사결정 요약

상세와 원문은 `docs/acknowledge/2026-10-09-lua-skin-compat-decisions.md`(D1~D10)와 사양 §2(E1~E10).

### 4.1 사용자 결정

| # | 결정 | 기각한 대안 |
| --- | --- | --- |
| D1 | 기본 스킨은 자작. ModernChic 의 Lua·이미지·폰트·사운드를 저장소에 넣거나 개변하지 않고 배치만 참고 | ModernChic 개변판 포함(원작자 표기와 제3자 자산 정리 필요) |
| D2 | beatoraja·ModernChic 이름 금지 규칙 폐지 | |
| D3 | `steel-neon` 3세대와 혼합 합성 삭제, 스키마 버전으로 구 선택값 초기화 | 혼합 합성 유지 |
| D4 | 기본 스킨은 영어 문구, 선곡 사이드 메뉴는 설정·볼륨 2패널, 캐릭터·음성·IR 메뉴·이력·인프레션·자산 변종·로테이션·버전 확인 제외 | |
| D5 | mp4 동영상 포함, 디코더 의존성 승인(OpenH264 소스 빌드 + `re_mp4`) | FFmpeg 정적·시스템 링크, 사이드카, GStreamer, OS 기본 디코더, 순수 Rust H.264, symphonia |
| D6 | 비교 기준은 캡처 없이 좌표 표 대조 | |
| D7 | 웨이브가 게이트를 통과할 때마다 논리 단위로 커밋·푸시(이 작업에 한해) | |
| D8 | 선곡의 건반 6(F)은 즐겨찾기만, NEXT_REPLAY 제거 | 둘 다 실행 |
| D9 | 선곡의 종류별 노트 수·밀도·주 BPM 은 빈칸, 별도 작업 | 지금 songdb 열 추가와 전체 재스캔 |
| D10 | 릴리스 바이너리에도 동영상 포함, OpenH264 BSD-2 고지 동봉 | |

### 4.2 메인이 정하고 고지한 기본값

| # | 요지 |
| --- | --- |
| E1 | Lua 격리: `require`·`dofile`·io 읽기는 스킨 루트 안만, 쓰기는 `<설정 폴더>/skin-data/<식별자>/` 오버레이, 네트워크는 항상 실패 |
| E2 | Settings·Tables·Folders·Practice·스캔 대기는 내장. IR 랭킹, 기록 모달, 필터, 옵션 오버레이(F1), 안내, 토스트, 디버그, 리플레이 분석 띠는 스킨 위 시스템 오버레이 |
| E3 | 기존 단축키 유지 + START(A)/SELECT(W) 신설. Q 는 IR 랭킹이 쓴다 |
| E4 | 내장 play/select/result 는 폴백으로 유지(스킨 로드 실패, 24키) |
| E5 | 스킨 폰트에 없는 글자는 앱 다국어 폰트로 |
| E6 | 기본 창 1280x720, 해상도·창 모드 설정, 16:9 가 아니면 레터박스(8C 에서 일반 창 기본값을 켬) |
| E7 | 그리기·타이밍은 beatoraja HEAD `8320241d` 그대로, 입력 결함은 원작 의도로 교정 |
| E8 | IR 타이머 172~174 와 옵션은 실제 값, 랭킹 목록 연결은 후속 |
| E9 | 기본 스킨에 9키 포함, ModernChic 의 9키는 내장 폴백 |
| E10 | 색 공간은 바이트 통과(비 sRGB 표면). 실제 창의 내장 화면이 전보다 어둡다 |

8C 에서 정한 기본값: 기본 스킨은 `skin.folder` 와 무관하게 `<설정 폴더>/skin/rbms-default/` 에 설치하고 팩을 지정하지 않으면 이 폴더를 쓴다. `skin.*` 전체는 기기 로컬. 일반 창 레터박스 기본 켬. 테마 색 재조정은 범위 밖.

## 5. 작업 규칙

| 구분 | 규칙 |
| --- | --- |
| 위임 | Workflow 전용(Agent 단독 호출과 general-purpose 금지). 모델은 Opus effort high 이하, Sonnet effort xhigh 이하. 모든 `agent()` 에 `model` 과 `effort` 를 명시한다. 사양·판정·Git 은 메인이 직접 한다 |
| workflow 선택 | 새 세션, resume, clear, compact, 이 문서 기반 재개에서는 workflow 사용 여부를 다시 묻는다. 이전 선택을 승계하지 않는다 |
| 커밋·푸시 | 웨이브가 게이트를 통과하면 메인이 되돌릴 수 있는 논리 단위로 커밋하고 `origin/dev` 에 일반 push. Conventional Commits, 설명은 한국어, author 단독, AI 트레일러(Co-Authored-By 등) 금지, `git add -A` 금지(선별 스테이징), force push 금지. 하위 에이전트는 커밋하지 않는다 |
| CI | 푸시 뒤 `gh run list --branch dev --limit 3` 으로 세 OS 결과를 확인하고, 실패하면 `gh run view <id> --log-failed` 로 원인을 찾아 다음 작업 전에 고친다. 로컬 통과는 CI 통과의 근거가 아니다 |
| 조사 결과 | `docs/` 에 상세히 저장하고, 코드가 바뀌면 같은 단위에서 해당 절을 고친다(재탐색으로 토큰을 쓰지 않기 위함). 색인은 `docs/reference/skin-compat/README.md` |
| 읽기 전용 | `/Users/hyunseokbyun/Downloads/ModernChic` 과 `/Users/hyunseokbyun/development/beatoraja` 는 읽기 전용이다. ModernChic 의 파일을 저장소에 넣지 않는다 |
| UI | 실제 렌더(캡처)를 열어 확인한 뒤에만 됐다고 보고한다. 실제 GPU 창 확인은 사용자 절차다 |
| Rust | `//` 주석 금지(`//!`·`///` 영어 문서 주석만), 매직넘버 상수화, `#[allow]`·`unsafe` 금지, `rustfmt.toml` 의 `max_width = 160` |
| 보안 | `.env` 계열은 읽지도 쓰지도 않는다 |
| 응답 | 한국어 존댓말로 간결하게. 첫 줄에 행동이나 답, 마지막에 다음 행동 하나 |

## 6. 미해결 · 사용자 확인 필요

| # | 내용 | 출처 |
| --- | --- | --- |
| 1 | 실제 GPU 창에서 첫 실행 → 선곡 → 결정 → 플레이 → 결과, 마우스 조작, 소리, 1280x720 창, 한글·일본어 제목 확인 | `docs/quality-assurance/2026-10-11-default-skin-manual-check.md` |
| 2 | 내장 화면의 밝기 변화(E10)에 따른 테마 색 재조정 여부. 사양은 범위 밖으로 두었고 실창 확인 뒤 별도 작업으로 정한다 | 사양 E10, 웨이브 1B 기록 |
| 3 | 기존 사용자의 `letterbox: false` 를 한 번 켜는 스키마 4 마이그레이션 여부. 사양의 "명시적 값은 유지"와 충돌해 하지 않았다 | 8C 단위 B 위험 |
| 4 | `Cargo.lock` 이 `.gitignore` 로 추적되지 않아 릴리스 빌드가 고지 파일의 버전과 다른 의존을 해석할 수 있다. 추적하고 `--locked` 를 쓸지 | 8C 단위 C 위험 |
| 5 | 더는 싣지 않는 설치 파일을 지우는 동작(단위 A 의 지시 밖 판단)을 유지할지. 빼려면 `install_pack` 의 첫 번째 `for` 루프만 지운다 | 8C 단위 A·수정 |
| 6 | `--skin` 규칙이 넓어졌다(팩과 손으로 고른 플레이 문서도 제치고 내장 필드). 원치 않으면 `SkinLibrary::plays_built_in` 에 "팩을 지정하지 않았을 때만" 조건을 넣는다. 내장 화면으로 가는 별도 설정 행은 없다 | 8C 수정 |
| 7 | 고지 7.3절의 41개 크레이트는 패키지에 라이선스 파일이 없어 저작권 표시를 싣지 못했다. `cargo-about` 도입 여부, Apache-2.0 선택 문구 확인 | 8C 수정 |
| 8 | macOS 러너에 `nasm` 이 없으면 x86_64 OpenH264 가 C 로만 빌드되어 느릴 수 있다(`brew install nasm` 단계). `x86_64-apple-darwin` 의 OpenH264 빌드는 첫 실제 릴리스에서야 확인된다 | 8C 단위 C |
| 9 | 스킨 옵션을 기기 간에 옮기려면 팩 이름 기준 키로 바꾸는 별도 작업이 필요하다. 계정에 이미 올라간 다른 기기의 스킨 경로는 다음 업로드에서 빈 값으로 덮인다 | 8C 단위 B |
| 10 | Windows·Linux 에서 확인하지 못한 것: `build.rs` 의 `include_bytes!` 경로, `note_tests.rs`(기본 팩을 읽음), 링크 폴더 테스트(`cfg(unix)`), 예산 초과 테스트의 5초 | 푸시 뒤 CI |
| 11 | `skin_external` 은 ModernChic 에서 2분, 기본 팩에서 36초. 동영상 디코드 테스트는 디버그에서 10분 이상(release 92초). 모두 환경 변수 테스트라 CI 에는 영향이 없다 | 8C 단위 D |
| 12 | 선곡 위 앱 오버레이와 스킨의 정합(검색 상자, 랭킹 패널 폭, 옵션 오버레이 비침, 토스트), customTimers, 선곡의 표 이름·레벨 | `docs/skin.md` §6, 설계서 §7 |

## 7. 환경과 전제

| 항목 | 내용 |
| --- | --- |
| 기기 | macOS arm64(Apple Silicon, Metal). JRE 없음(beatoraja 실행 불가) |
| 경로 | 저장소 `/Users/hyunseokbyun/development/R-BMS`, 외부 스킨 `/Users/hyunseokbyun/Downloads/ModernChic`(476 MiB, 저장소 밖), 레퍼런스 체크아웃 `/Users/hyunseokbyun/development/beatoraja`(HEAD `8320241d`) |
| 도구 | Rust 1.95.0(`rust-toolchain.toml`, rustfmt·clippy 포함), `uv`(이미지·사운드 생성), `gh`(CI 확인), `bun`(`web/`) |
| 실행 | `./start.sh [곡 폴더 또는 차트] [옵션]`(gitignore 된 로컬 스크립트) 또는 `cargo run --release -p rbms-player -- <인자>`. 격리 실행: `HOME=<임시 폴더> ./target/release/rbms-player samples/preview-demo` |
| 환경 변수 | `RBMS_SKIN_PACK`(이번 실행의 스킨 팩), `RBMS_SKIN_CAPTURE_DIR`(캡처 PNG 저장, 테스트), `RBMS_REQUIRE_GPU`(GPU 없으면 테스트 실패), `RBMS_SKIN_SEED`(무작위 고정), `RBMS_GOLDEN_UPDATE`(골든 재생성), `RBMS_SONGS`, `RBMS_TIMING_CSV`, `RBMS_SOAK_LOG`. 표는 `docs/development.md` |
| Cargo 피처 | `rbms-player` 의 `video`(기본 켬). 끄면 동영상 자리가 비고 코덱이 컴파일되지 않는다 |
| 설정 폴더 | `~/.config/rbms/`(Windows `%USERPROFILE%\.config\rbms\`): `settings.ron`, `skin/rbms-default/`, `skin-data/` 등 |

## 8. 다음 할 일 (우선순위 순)

1. **8C 통합 게이트와 커밋·푸시(메인).** `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --no-fail-fast`, `git diff --check` 를 한 번 확인한다(§3.2 의 기록이 현재 트리에 유효한지 `git status` 로 대조). 논리 단위로 선별 스테이징해 커밋한다. 후보 단위: 설정·동기화·레터박스(`crates/rbms-config/`, `settings_ui.rs`, `settings_view.rs`, `ir_sync.rs`), 기본 팩 내장·설치·폴백(`apps/rbms-player/build.rs`, `assets/default_pack.rs`, `skin_select.rs`, `skin_screen.rs`, `font.rs`), 테스트·게이트(`stage/capture/`, `crates/rbms-skin/tests/default_pack.rs`, `apps/rbms-cli/src/skin_dump/tests.rs`), 고지·릴리스·CI(`THIRD-PARTY-NOTICES.md`, `.github/workflows/`, `docs/ci-release.md`), 문서.
2. **푸시 뒤 `gh run list --branch dev --limit 3`.** 세 OS 의 test 와 fmt+clippy 를 확인한다. §6-10 의 항목이 Windows·Linux 에서 걸리는지 본다.
3. **조사 문서와 PROCESS 갱신(메인).** `docs/PROCESS.md` L5 웨이브 8C 체크, 사양 §6 웨이브 8C 완료 기록, 결정 문서에 §6 의 3~7번 결정, 조사 문서 `r3-rbms-app.md`(기본 팩 설치·선택·폴백), `r2-rbms-render.md`(`BUNDLED_FONT`), 색인의 갱신 이력.
4. **L6 최종 검증.** `cargo run -p rbms-cli -- skin-dump <ModernChic 폴더> --strict --allow-failure keyconfig.luaskin`(keyconfig 만 원본 결함), 기본 팩의 `--strict --frames 120`, 캡처를 `m2`~`m5` 와 대조하고 사용자가 `docs/quality-assurance/2026-10-11-default-skin-manual-check.md` 를 수행한다.
5. **L7 마감.** `docs/history/` 에 이번 작업 기록, 조사 문서 정리, 이 문서 갱신.
6. **웨이브 9 후보**(사양 §6): customTimers/customEvents(`crates/rbms-render/src/skin_render/`, `skin_host/`), IR 랭킹·라이벌 연결(`skin_host/ir.rs`), 종류별 노트 수·밀도·주 BPM(songdb 열 추가, `PARSER_VERSION` 상승, 전체 재스캔), 24키 문서, 스킨 설정 Stage(type 9), skinpreview, BGA 동영상(`rbms-video` 재사용), 선곡 위 오버레이와 스킨의 정합.

## 9. 문서 지도

| 문서 | 한 줄 |
| --- | --- |
| `docs/PROCESS.md` | 현재·누적 작업 체크리스트(상단 "현재 작업"이 이 작업) |
| `docs/skin.md` | 스킨 가이드: 지원 범위, 팩 쓰기, 기본 스킨, Lua 환경, 도구, 알려진 한계 |
| `docs/plan/2026-10-09-lua-skin-compat.md` | 정본 사양과 웨이브별 진행·완료 기록 |
| `docs/plan/2026-10-11-default-skin-design.md` | 기본 스킨 설계서(격자, 모듈 API, 화면별 좌표, 검증 기록) |
| `docs/acknowledge/2026-10-09-lua-skin-compat-decisions.md` | 사용자 결정 D1~D10, 디코더 선정, 기각한 대안 |
| `docs/reference/skin-compat/README.md` | 조사 보고서 색인(b1~b5 beatoraja 계약, m1~m6 외부 스킨, r1~r3 R-BMS 현황, v1~v2 실측) |
| `docs/quality-assurance/2026-10-11-default-skin-manual-check.md` | 사람이 실제 창에서 확인할 목록 |
| `docs/quality-assurance/2026-09-16-phase-h-manual-checks.md` | 오디오·연습·코스·다중 IR 수동 점검(아직 미수행) |
| `docs/architecture.md`, `docs/crates.md`, `docs/development.md` | 구조·크레이트 레퍼런스·빌드와 개발 안내(영어) |
| `docs/ci-release.md` | CI 매트릭스, 릴리스 워크플로, 배포 묶음과 제3자 고지, Phase R 경계 |
| `docs/theme.md` | 내장 화면의 색 테마 `theme.ron` |
| `docs/roadmap.md` | 남은 작업과 테스트가 고정한 의도적 동작 |
| `docs/history/` | 변경 불가 이력과 스냅샷(`2026-10-09-handoff-snapshot.md` 포함) |
| `docs/feedback/`, `docs/acknowledge/` | 지적 리포트(`2026-10-11-ci-not-checked-after-push.md` 등)와 결정·합의 |
| `docs/plan/2026-09-17-*.md`, `docs/acknowledge/2026-09-17-skin-system-decisions.md` | 2026-10-09 폐기된 구 스킨 체제의 사양·결정(이력으로만) |
| `THIRD-PARTY-NOTICES.md` (저장소 루트) | 제3자 고지(영어). 의존성이 바뀌면 갱신(`docs/ci-release.md` §2.1) |
