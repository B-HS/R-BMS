# CI / 릴리스 (GitHub Actions)

최종 갱신 2026-10-24 · 대응 커밋 76e026a(web-db 새 동작 여덟 개와 표 Cron 추가, 2026-10-23 운영 실행 기록. v0.1.7 행은 d438294. 2026-10-24 에 6.1·6.2절의 `dan-recompute` 출력 숫자 설명과 0013 기록만 더했다)

> 이 문서는 워크플로 구현, 릴리스 절차와 이력, 현재 배포 경계를 설명합니다. 워크플로가 존재한다는 사실은 산출물이 지금 사용 가능하다는 보증이 아닙니다. 게시된 릴리스는 `v0.1.4` 하나입니다(`gh release view v0.1.4` 의 `publishedAt` 이 2026-10-15T07:27:28Z, 대응 커밋 ea698a9 의 3분 뒤). `v0.1.0`·`v0.1.3` 은 게시하지 않은 draft 로 남아 있습니다(2.3절).
>
> 저장소는 PUBLIC 이므로 Actions 로그도 공개입니다. 외부 키, 인증서, 공증 자격 증명, DB 주소의 값은 이 저장소나 문서에 넣지 않으며 워크플로 로그에도 출력하지 않습니다.

이 문서의 "왜" 는 결정 기록에 있습니다: [릴리스는 태그와 draft 까지 자동, 게시는 사용자, 자격 증명은 빌드 전에](acknowledge/2026-10-14-adr-release-policy.md), [macOS 는 공증 티켓을 붙인 앱 번들 + dmg](acknowledge/2026-10-14-adr-macos-app-bundle-dmg.md), [릴리스 툴체인 1.95.0 고정](acknowledge/2026-10-14-adr-release-toolchain-pin.md), [운영 DB 작업은 web-db 워크플로](acknowledge/2026-10-14-adr-production-db-operations.md), [Vercel Git 연결과 운영 브랜치 dev](acknowledge/2026-10-14-adr-vercel-git-deploy.md), [배치 시드 범위](acknowledge/2026-10-14-adr-layout-seed-range.md), [v0.1.3 draft 는 두고 v0.1.4 로 낸다](acknowledge/2026-10-14-adr-v0-1-4-release.md), [앱 아이콘과 Windows 리소스](acknowledge/2026-10-15-adr-app-icon.md). 테스트 공백과 그 실행 시점은 [테스트 부채](quality-assurance/2026-10-14-test-debt.md)에 있습니다.

## 0. 워크플로 한눈에

| 워크플로 | 파일 | 시작 | 하는 일 | 상세 |
| --- | --- | --- | --- | --- |
| ci | `.github/workflows/ci.yml` | `dev`·`prod` 푸시와 PR(`web/` 만 바뀐 변경 제외. 단 `web/data/tables/**` 는 포함) | Rust 세 OS 빌드와 테스트(Windows 는 아이콘 리소스를 먼저 컴파일), fmt + clippy | 1절 |
| web | `.github/workflows/web.yml` | `web/**`, `contract/**` 또는 이 워크플로 파일이 바뀐 `dev`·`prod` 푸시와 PR | 웹 typecheck, lint, test, format, build(`verify` 잡)와 임시 MySQL 위의 통합 테스트(`db` 잡) | 6절 |
| release | `.github/workflows/release.yml` | 수동 실행(실제로 쓰는 경로), `prod` 푸시, `v*.*.*` 태그 푸시 | 버전·태그, macOS 앱 번들과 dmg(서명·공증), Windows zip, draft Release | 2절 |
| signing-check | `.github/workflows/signing-check.yml` | 수동 | 서명·공증 시크릿의 꼴과 로그인 점검(빌드 없음) | 3.2절 |
| web-db | `.github/workflows/web-db.yml` | 수동 | 운영 DB 의 상태 확인, 마이그레이션, 시험 계정 정리, 차트 노트 수 점검·복구·되돌리기·백업 삭제, 어시스트 등급 백필, 삭제 요청 차트의 행 지우기, GENOSIDE 코스 등록, Stella·Satellite 표 점검·강제 저장, 단위 다시 판정(동작 열아홉 개) | 6.1절 |

## 1. CI (`.github/workflows/ci.yml`)

- 트리거: `dev`·`prod`의 push와 두 브랜치를 대상으로 한 pull request. 경로 필터는 `paths: ["**", "!web/**", "web/data/tables/**"]` 입니다. 즉 `web/` 만 바뀐 변경은 이 워크플로를 건너뛰고(`web.yml` 이 맡음), 단 `web/data/tables/**` 는 플레이어가 실행 파일에 넣고(`apps/rbms-player/src/tablesrc/seed.rs`) 테스트가 읽으므로 이 워크플로도 돕니다. 같은 ref 의 이전 실행은 취소됩니다(`concurrency: ci-${{ github.ref }}`, `cancel-in-progress`). 권한은 `contents: read` 입니다.
- `test` 잡: Ubuntu, macOS, Windows 매트릭스(`fail-fast: false`)에서 `cargo build --workspace --locked`, `cargo test --workspace --no-fail-fast --locked`를 실행합니다. `--locked` 는 `Cargo.lock` 이 `Cargo.toml` 들과 어긋난 푸시를 CI 에서 실패로 드러냅니다. 테스트 수는 고정 계약이 아니며(ea698a9 의 Ubuntu 실행 38698550145 에서 모든 테스트 바이너리의 `test result` 합이 6,024개 통과·0 실패·6개 ignored), 숫자 대신 CI 결과로 판단합니다.
- Windows 아이콘 리소스: Windows 매트릭스에서만 빌드 앞에 `scripts/windows-icon-resource.ps1` 를 돌립니다(`Compile the icon resource`). 릴리스가 `rbms-player.exe` 에 링크하는 아이콘 리소스가 깨졌을 때 릴리스 실행이 아니라 푸시마다 드러나게 하려는 것입니다. 스크립트가 `RBMS_WINDOWS_RESOURCE` 를 `GITHUB_ENV` 에 적으므로 뒤의 `cargo build --workspace` 가 `apps/rbms-player/build.rs` 를 거쳐 실제로 링크합니다(2절 Windows).
- Linux 의존성: `libasound2-dev`, `libgtk-3-dev`, `libudev-dev`, `libxkbcommon-dev`, `libwayland-dev`, `libx11-dev`, `libxrandr-dev`, `libxi-dev`, `libxcursor-dev`를 설치합니다. 이는 cpal(ALSA), gilrs(udev), winit(X11/Wayland), rfd(GTK3) 링크 요구를 덮습니다.
- `lint` 잡: Ubuntu에서 Rust 1.95.0과 `rustfmt`, `clippy`를 설치한 뒤 `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`를 실행합니다. 둘 다 실패 시 CI를 실패시키는 게이트입니다.
- 캐시: 모든 잡은 `Swatinem/rust-cache@v2`를 사용합니다. CI 툴체인은 `rust-toolchain.toml`과 같은 1.95.0으로 고정됩니다.
- 제한 시간(`timeout-minutes`): `test` 30분, `lint` 15분. 가장 최근 통과 실행(ea698a9, run `38698550145`)에서 `test` 잡은 Ubuntu 9.3분, macOS 10.6분, Windows 14.1분(가장 느린 쪽), `lint` 잡은 1.5분이었습니다. 앞선 실행은 run `38557704381`(Ubuntu 9.1분, macOS 7.0분, Windows 13.3분, lint 1.4분)과 run `38562057725`(Ubuntu 8.7분, macOS 8.2분, Windows 12.8분, lint 1.2분)입니다. 제한은 가장 느린 실측의 약 2.1배이며, 캐시가 빈 첫 실행에서 모든 의존성을 다시 컴파일하는 시간과 멈춘 잡을 기본 6시간까지 두지 않는 것 사이의 값입니다. 2026-10-10 의 실행에서는 `test` 가 2.8~8.3분이었고 이후 테스트가 늘어 Windows 가 13~14분대가 되었습니다. 빈 캐시 실행의 소요는 측정하지 못했으므로, Windows 가 제한의 3분의 2(20분)에 닿으면 올립니다.
- OpenH264(동영상): `openh264-sys2` 가 C 소스를 `cc` 로 컴파일하므로 세 OS 모두 추가 설치 단계가 없습니다. x86_64 에서는 `nasm` 이 있으면 어셈블리를 쓰고 없거나 실패하면 어셈블리 없이 C 로만 빌드합니다(빌드는 실패하지 않음).
- 봇이 만드는 `release: vX.Y.Z` 커밋은 `GITHUB_TOKEN` 으로 푸시되므로 CI 를 시작하지 않습니다(GitHub 의 동작. 실제로 `release: v0.1.1`·`v0.1.2`·`v0.1.3`(fb29d24)·`v0.1.4`(87eacd0) 커밋에는 실행이 없습니다. `gh run list --commit <해시>` 로 확인). 그 커밋이 바꾸는 것은 `Cargo.toml` 과 `Cargo.lock` 의 버전뿐이고, 다음 푸시의 CI 가 같은 `--locked` 빌드로 덮습니다.

마지막으로 확인한 통과 CI 실행은 `38698550145`(ea698a9, v0.1.4 점검 기록)이며 Ubuntu·macOS·Windows test와 fmt/clippy 잡이 모두 성공했습니다. 웹 CI(`web.yml`)의 마지막 통과는 `38694945226`(51f3a1e)입니다. 푸시 뒤에는 `gh run list --branch dev` 와 `gh run view` 로 세 OS 의 결과를 직접 확인합니다.

## 2. 릴리스 워크플로 (`.github/workflows/release.yml`)

릴리스 워크플로는 다음 세 경로를 정의합니다.

1. 수동 dispatch(실제로 쓰는 경로): Actions의 `release`에서 patch, minor, major를 선택해 실행합니다(`gh workflow run release.yml --ref dev -f bump=patch`). `version` 잡이 가장 최근 `v*.*.*` 태그에서 해당 자리를 올립니다. 태그가 없으면 `v0.0.0`에서 계산합니다. 루트 `Cargo.toml`의 첫 workspace 버전만 바꾸고, 추적되는 `Cargo.lock` 의 워크스페이스 크레이트 버전도 `cargo update --workspace` 로 함께 맞춘 뒤(외부 의존 버전은 바뀌지 않습니다. 이 단계가 없으면 아래 `--locked` 빌드가 실패합니다) `release: vX.Y.Z` 커밋과 주석 태그(`git tag -a`)를 만들어 실행한 ref(dev)와 그 태그를 이름으로 push합니다(`--follow-tags` 는 주석 태그만 보내므로 태그를 직접 지정합니다. 뒤의 빌드 잡이 이 태그를 체크아웃합니다).
2. `prod` push: 같은 버전·빌드 흐름이 patch 로 돕니다. `prod` 브랜치는 만든 적이 없고(원격 브랜치는 `dev` 와 `feat/skin-followups`) 이 경로는 쓴 적이 없습니다.
3. 태그 push: `v*.*.*` 태그 push는 version 잡을 건너뛰고 그 태그 ref로 빌드합니다. 첫 릴리스 `v0.1.0` 이 이 경로였습니다.

`release.yml` 은 최상위 `concurrency: { group: release, cancel-in-progress: false }` 라서 릴리스 실행은 한 번에 하나만 돕니다. 실행 중에 또 시작하면 앞 실행이 끝날 때까지 기다리고, 기다리는 실행이 둘이 되면 더 오래된 대기 실행이 취소됩니다(GitHub 동작). 실행 중에는 `dev` 에 푸시하지 않습니다. `version` 잡은 `git push origin "HEAD:${GITHUB_REF_NAME}"`(수동 실행에서는 `dev`)로 올리므로 그 사이 `dev` 가 앞서면 비빨리감기라 거절됩니다(강제 푸시는 쓰지 않습니다). 같은 이유로 실행이 끝나면 로컬에서 `git pull` 과 `git fetch --tags` 를 먼저 합니다.

빌드와 게시 단계는 다음과 같습니다.

- macOS: 빌드보다 먼저 자격 증명을 확인합니다(사용자 지시 2026-10-13: 빌드 뒤에 확인하면 틀린 시크릿 하나에 10분 넘게 걸린다). 첫 두 단계가 인증서를 임시 키체인에 넣어 서명 신원을 정하고(`Prepare the signing keychain`), 공증 계정으로 로그인해 봅니다(`Check the notarisation account`). 어느 쪽이든 틀리면 체크아웃과 빌드 전에 잡이 멈춥니다. 이어서 `aarch64-apple-darwin`과 `x86_64-apple-darwin` release binary를 `cargo build --release --locked -p rbms-player` 로 빌드하고 `lipo`로 합친 뒤, `scripts/macos-app.sh` 로 앱 번들 `R-BMS.app`(번들 ID `uk.hyuns.rbms`, 실행 파일 `Contents/MacOS/rbms-player`, 최소 macOS 11.0, 아이콘은 `assets/icon/R-BMS.icns` 를 `Contents/Resources/R-BMS.icns` 로 복사하고 `Info.plist` 의 `CFBundleIconFile` 이 `R-BMS` 를 가리킴. 아이콘 파일이 없으면 스크립트가 종료 코드 1 로 멈추므로 아이콘 없는 앱이 만들어지지 않습니다)을 만듭니다. 시크릿이 있으면 앱을 Developer ID 로 서명(hardened runtime, 보안 타임스탬프)하고 공증해 티켓을 붙인 뒤(`scripts/macos-notarize.sh`: `notarytool submit --wait`, 로그의 `status: Accepted` 줄로 통과를 판정, `stapler staple`, `stapler validate`), 앱과 `Applications` 링크와 세 문서를 담은 `R-BMS-macos-universal.dmg` 를 만들어 서명·공증·티켓 부착을 한 번 더 하고, `spctl` 로 Gatekeeper 판정을 확인합니다(앱은 `--type execute`, dmg 는 `--type open --context context:primary-signature`). 시크릿이 없으면 ad-hoc 서명이고 공증은 없습니다. 결과물은 dmg 와 SHA-256 파일입니다.
- 앱 번들로 바꾼 이유(2026-10-14, v0.1.1): 단독 실행 파일에는 공증 티켓을 붙일 수 없어 Gatekeeper 가 온라인 조회에 기댑니다. 공증이 통과한(`spctl` 의 온라인 판정 `accepted, source=Notarized Developer ID`) v0.1.1 의 실행 파일도 Chrome 으로 받은 압축을 푼 사용자 환경에서 "Apple could not verify" 로 막혔고, `syspolicy_check distribution` 은 "Notary Ticket Missing" 을 냈습니다. 앱과 dmg 에는 티켓을 붙일 수 있어 오프라인에서도 통과합니다. 따라서 워크플로의 `spctl` 판정은 온라인에서의 판정이라 이것만으로 충분하지 않고, 오프라인 증거는 `stapler validate` 입니다(2.2절의 점검에 둘 다 있습니다). Finder 로 앱을 처음 열 때 macOS 가 붙일 수 있는 `-psn_...` 인자는 곡 경로로 읽지 않습니다(`apps/rbms-player/src/lib.rs` 의 `is_system_launch_arg`).
- Windows: 빌드 앞의 `Compile the icon resource` 단계가 `scripts/windows-icon-resource.ps1` 를 돌립니다. 스크립트는 러너의 Windows SDK(`%ProgramFiles(x86)%\Windows Kits\10\bin\*\x64\rc.exe` 중 버전이 가장 높은 것, 없으면 예외)로 `assets/icon/rbms.rc`(내용은 `1 ICON "rbms.ico"` 한 줄)를 `target/rbms-icon.res` 로 컴파일하고 그 경로를 `RBMS_WINDOWS_RESOURCE` 로 `GITHUB_ENV` 에 적습니다. `apps/rbms-player/build.rs` 의 `link_windows_resource` 가 그 `.res` 를 `rbms-player.exe` 에 링크합니다(탐색기와 작업 표시줄의 앱 아이콘). 변수가 없는 빌드는 아이콘 없이 링크하고(로컬 개발 빌드가 SDK 도구에 기대지 않게), 변수가 있는데 Windows MSVC 타깃이 아니거나 가리키는 파일이 없으면 빌드가 멈춥니다(아이콘 없는 실행 파일이 배포되지 않게). `winresource` 같은 크레이트 대신 SDK 도구를 쓴 것은 새 의존성을 들이지 않으려는 사용자 결정(2026-10-15, 옵션 Q2 A)입니다([앱 아이콘 ADR](acknowledge/2026-10-15-adr-app-icon.md), 기각된 대안 포함). `ci.yml` 의 Windows 잡도 같은 스크립트로 컴파일·링크해 깨진 리소스를 푸시마다 잡습니다(1절). `cargo build --release --locked -p rbms-player` 로 빌드한 `rbms-player.exe`를 `rbms-windows-x64.zip`으로 묶고 SHA-256 파일을 만듭니다(PowerShell `Get-FileHash` 의 출력이라 파일 이름 없이 대문자 해시 한 줄입니다. macOS 쪽은 `shasum` 형식이라 `shasum -a 256 -c` 로 검사할 수 있지만 이 파일은 안 됩니다). 코드 서명은 하지 않습니다.
- 초안 릴리스: 두 플랫폼 아카이브와 체크섬을 `softprops/action-gh-release@v2`로 **초안(draft)** GitHub Release 에 첨부합니다(`draft: true`, `generate_release_notes: true`, `fail_on_unmatched_files: true`, 사용자 결정 2026-10-12). 어느 경로로 돌든 자동은 태그와 초안까지이고, 게시(publish)는 유지보수자가 GitHub 의 Releases 화면에서 릴리스 노트를 확인한 뒤 직접 합니다. 자동 생성 노트는 커밋 목록뿐이라 메인이 사실을 확인해 쓴 노트로 바꿉니다(2.2절).
- 동영상: 두 빌드 모두 `rbms-player` 의 기본 피처 `video` 를 켠 채(`--no-default-features` 없이) 빌드하므로 OpenH264 가 바이너리에 정적으로 들어갑니다(결정 D10). 소스 빌드라 Cisco 가 배포하는 바이너리를 내려받지 않습니다.
- 툴체인: 릴리스 빌드 잡도 CI 와 같이 `dtolnay/rust-toolchain@master` 에 `toolchain: "1.95.0"` 을 줍니다(`rust-toolchain.toml` 과 같은 값. 올릴 때 `rust-toolchain.toml`, `ci.yml`, `release.yml` 의 `1.95.0` 과 루트 `Cargo.toml` 의 `rust-version` 을 함께 올립니다). 첫 릴리스 실행(2026-10-12, 태그 `v0.1.0`, run 38222704409)은 `@stable` 에 macOS 두 타깃을 설치했는데, cargo 는 `rust-toolchain.toml` 의 1.95.0 으로 빌드하므로 `x86_64-apple-darwin` 의 `core` 를 찾지 못해 실패했습니다(`error[E0463]: can't find crate for core`). 빌드에 쓰이는 툴체인에 타깃을 설치해야 합니다.
- x86_64 슬라이스: 유니버설 빌드의 `x86_64-apple-darwin` 쪽 OpenH264 는 Apple Silicon 러너에서 교차 컴파일됩니다. 릴리스 실행 다섯 번(v0.1.0~v0.1.4)에서 빌드는 통과했고 내려받은 바이너리가 `x86_64 arm64` 였습니다. Intel Mac 에서 실제로 실행해 본 기록은 없고, `nasm` 이 없는 러너에서는 어셈블리 없이 C 로 빌드되어 Intel Mac 의 동영상 디코드가 느릴 수 있습니다(`docs/quality-assurance/2026-10-14-test-debt.md` 3.5).

### 2.1 배포 묶음과 제3자 고지

두 배포물은 같은 세 문서와 실행 파일을 담습니다.

| 파일 | macOS (`R-BMS-macos-universal.dmg`) | Windows (`rbms-windows-x64.zip`) |
| --- | --- | --- |
| 실행 파일 | `R-BMS.app`(안의 `Contents/MacOS/rbms-player`) | `rbms-player.exe` |
| `LICENSE` | O | O |
| `README.md` | O | O |
| `THIRD-PARTY-NOTICES.md` | O | O |

- 세 문서 중 하나라도 저장소 루트에 없으면 패키징 단계가 실패합니다(예전에는 없으면 조용히 건너뛰었습니다). 고지 없는 아카이브가 게시되지 않게 하려는 의도입니다.
- 기본 스킨(`assets/skins/rbms-default`)은 바이너리에 내장되므로 아카이브에 스킨 폴더를 따로 넣지 않습니다.
- `THIRD-PARTY-NOTICES.md` 는 영어이며 다음을 담습니다. OpenH264(Cisco, BSD-2-Clause)의 저작권 표시와 라이선스 전문 및 "소스에서 빌드해 정적으로 링크했다"는 사실, `openh264` 바인딩과 `re_mp4` 의 라이선스, Inter 글꼴의 SIL OFL 1.1 전문, 바이너리에 컴파일되어 들어가는 Lua 5.2.4 와 SQLite, 링크되는 모든 Rust 크레이트의 이름·버전·라이선스(직접 의존 표와 라이선스별 전체 목록), 그리고 그 크레이트들의 라이선스 전문(그 파일의 7절). 7절은 Apache-2.0 을 선택지로 주는 크레이트를 Apache-2.0 으로 받아 전문 한 벌로 덮고, 나머지 크레이트는 패키지에 실린 라이선스 파일을 저작권 표시와 함께 싣습니다(같은 본문은 한 번). 패키지에 라이선스 파일이 없는 크레이트는 7.3 절에 이름·저자·저장소로 적습니다.
- 갱신 시점: 의존성을 추가·삭제하거나 버전을 올렸을 때, 번들 글꼴·네이티브 소스·제3자 자산이 바뀌었을 때, 그리고 릴리스 전. `THIRD-PARTY-NOTICES.md` 6절의 스크립트(`python3 -I gen-notices.py`. 파일은 저장소에 없고 그 절에 본문이 실려 있으며 내부에서 `cargo metadata --locked` 를 실행합니다)로 그 파일의 5절 표와 7절 전체를 다시 만듭니다. 스크립트는 로컬 레지스트리의 패키지에서 라이선스 파일을 읽으므로 먼저 `cargo fetch --locked` 가 되어 있어야 하고, 본문은 행 끝 공백만 지우고 그대로 싣습니다.
- 새 라이선스가 GPL-3.0-or-later 와 호환되는지 확인합니다. 현재 의존에서는 `OR` 선택지가 있는 크레이트(`self_cell` 의 Apache-2.0 OR GPL-2.0-only, `r-efi` 의 LGPL-2.1-or-later 선택지)를 허용 쪽 라이선스로 쓰며, `symphonia` 13개는 MPL-2.0 입니다(MPL-2.0 3.3 절로 결합 가능, 소스는 crates.io 에서 변경 없이 받을 수 있음).
- `Cargo.lock` 은 git 에 추적되고 CI·릴리스 빌드는 모두 `--locked` 입니다. 따라서 릴리스 바이너리의 의존 버전은 커밋된 `Cargo.lock` 과 같고, 고지 파일의 버전표도 그 `Cargo.lock` 에서 만듭니다. 의존성을 바꾸면 `Cargo.lock` 을 함께 커밋하고 `THIRD-PARTY-NOTICES.md` 의 표를 다시 만듭니다. 플랫폼 전용 의존(`cargo tree --target all`)도 `Cargo.lock` 에 모두 들어 있어 macOS 두 아키텍처와 Windows 가 같은 파일로 빌드됩니다.

### 2.2 릴리스 한 번의 절차와 확인

항상 태그와 draft 까지 자동이고 게시는 사용자입니다(사용자 결정 2026-10-12). 순서는 다음과 같습니다.

1. 릴리스할 커밋(`dev` 의 최신)이 푸시되어 있고 그 커밋의 Rust CI 세 OS 와 lint 가 통과했는지 확인합니다(`gh run list --branch dev --limit 5`, `gh run view <id>`). 웹 변경이 섞였으면 `web` 워크플로도 봅니다.
2. 시크릿을 바꿨다면 먼저 `gh workflow run signing-check.yml --ref dev` 로 값의 꼴과 로그인을 확인합니다(3.2절).
3. `gh workflow run release.yml --ref dev -f bump=patch` 를 실행하고 `gh run watch` 로 끝까지 봅니다. macOS 잡의 단계는 순서대로 서명 키체인 준비, 공증 계정 확인, 체크아웃, 툴체인, 유니버설 빌드, 앱 번들, 서명, 앱 공증, dmg 생성, dmg 서명·공증, Gatekeeper 판정, 체크섬, 키체인 삭제, 업로드입니다. 앞의 둘이 틀리면 체크아웃과 빌드 전에 멈춥니다.
4. 끝나면 로컬에서 `git pull`(봇이 `dev` 에 `release: vX.Y.Z` 커밋을 올렸습니다)과 `git fetch --tags` 를 합니다. 로컬 저장소에는 `v0.1.0` 만 있고 `v0.1.1`~`v0.1.4` 태그가 없을 수 있습니다(2026-10-15 확인: 원격에는 모두 있음).
5. 산출물을 내려받아(`gh release download vX.Y.Z --dir <빈 폴더>`) 메인이 직접 점검합니다. 값이 아니라 출력으로 판단합니다.
   - SHA-256: macOS 는 `shasum -a 256 -c R-BMS-macos-universal.sha256`(같은 폴더에서). Windows 는 `shasum -a 256 rbms-windows-x64.zip` 의 값을 `rbms-windows-x64.sha256` 의 대문자 해시와 대소문자 없이 비교합니다.
   - dmg: `xcrun stapler validate R-BMS-macos-universal.dmg`, `spctl --assess --type open --context context:primary-signature -vv R-BMS-macos-universal.dmg`.
   - 앱: `hdiutil attach R-BMS-macos-universal.dmg -nobrowse` 로 마운트한 뒤 `R-BMS.app` 에 `xcrun stapler validate`, `spctl --assess --type execute -vv`, `syspolicy_check distribution`("passed all pre-distribution checks"가 나와야 함), `codesign --verify --deep --strict --verbose=2`, `lipo -archs Contents/MacOS/rbms-player`(`x86_64 arm64`), `/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' Contents/Info.plist`(릴리스 버전과 같아야 함), `/usr/libexec/PlistBuddy -c 'Print :CFBundleIconFile' Contents/Info.plist`(`R-BMS`)와 `Contents/Resources/R-BMS.icns` 존재. 끝나면 `hdiutil detach`.
   - 실행 파일: `Contents/MacOS/rbms-player --version` 만 실행합니다. `--version`·`--help` 는 설정·기록 DB·서버를 건드리지 않고 끝납니다(`apps/rbms-player/src/command_line.rs`, 1676c57 부터). 다른 인자로 내려받은 앱을 실제 `HOME` 에서 실행하면 그 사용자의 설정·곡 DB·기록을 바꾸고 운영 서버로 기록을 보냅니다. 실행이 꼭 필요하면 `HOME=$(mktemp -d)` 와 `--server http://127.0.0.1:9` 를 줍니다(2026-10-15 사고: [피드백](feedback/2026-10-15-run-the-player-with-a-scratch-home.md)).
   - Windows zip: 풀어서 `rbms-player.exe` 의 리소스에 `RT_ICON` 과 `RT_GROUP_ICON` 이 있는지 봅니다(v0.1.4 는 `RT_ICON` 7개, `RT_GROUP_ICON` 1개. 7개는 `rbms.ico` 의 크기 7종).
   - 사람이 하는 확인: Chrome 으로 받은 dmg 를 열어 `R-BMS.app` 을 처음 실행합니다(v0.1.1 이 이 환경에서 막혔습니다. draft 의 파일은 GitHub 에 로그인한 브라우저로만 받을 수 있습니다).
6. 릴리스 노트를 `gh release edit vX.Y.Z --notes-file <파일> --draft=true` 로 바꿉니다. 본문은 코드와 문서로 사실을 확인해 씁니다: 내려받기 표(파일과 SHA-256), 바뀐 점, 업그레이드(설정 파일의 스키마가 오르면 `settings.ron.vN.bak` 이 생김), 알려진 한계. 노트 초안은 세션 scratchpad 에 있을 수 있어 새 세션에서는 `docs/HANDOFF.md` 의 요지로 다시 씁니다.
7. 게시는 사용자가 GitHub 의 Releases 화면에서 합니다. 에이전트는 draft 를 해제하지 않습니다.
8. 문제가 나와 draft 를 지우면(사용자 요청이 있을 때만 `gh release delete`) 태그는 남습니다. 다음 실행은 가장 최근 태그에서 올리므로 지운 버전 번호는 건너뜁니다(`v0.1.1`, `v0.1.2` 가 그렇게 건너뛰어졌습니다). 같은 번호를 다시 쓰려면 태그를 옮겨야 하며, `v0.1.0` 을 한 번 옮길 때 사용자 승인을 받았습니다.

### 2.3 릴리스 이력

| 태그 | 가리키는 커밋 | 실행 | 산출물 | 결과 |
| --- | --- | --- | --- | --- |
| `v0.1.0` | `00b91b1`(처음 `ef75bbc` 에 만들었다가 웨이브 15 이후 커밋으로 한 번 옮김, 사용자 승인) | 태그 push. run 38222704409 실패(툴체인에 타깃 없음, 2026-10-12) → run 38253353651 시도 1·2 실패(인증서 값에 `.` 이 섞임, base64 해독 실패) → 시도 3 통과(2026-10-13 18:31~18:50 UTC) | `rbms-macos-universal.tar.gz`(맨 실행 파일), `rbms-windows-x64.zip`, sha256 둘 | Developer ID 서명과 공증 `status: Accepted` 로 draft 생성. **게시된 적 없고 draft 가 남아 있음.** 산출물이 맨 실행 파일이라 v0.1.1 과 같은 Gatekeeper 차단이 예상된다. 지울지는 사용자 미결정 |
| `v0.1.1` | `cdd27a6` | 수동 실행 run 38550449246(2026-10-14) | tar.gz(맨 실행 파일), zip | 공증 통과. 사용자가 Chrome 으로 받은 실행 파일이 "Apple could not verify" 로 막힘(공증 티켓 없음). draft 삭제, 태그는 남음 |
| `v0.1.2` | `c3937f4` | 수동 실행 run 38559305840(2026-10-14) | `R-BMS-macos-universal.dmg`, zip, sha256 둘 | 앱 번들 + dmg 로 앱·dmg 모두 티켓 부착. dmg SHA-256 일치, `stapler validate` 통과(dmg 와 앱), Gatekeeper `accepted, source=Notarized Developer ID`, `syspolicy_check distribution` 통과, 버전 0.1.2, `x86_64 arm64`, zip SHA-256 일치. 그러나 실제 판의 점수 제출이 전부 400(큰 시드, `docs/bug/2026-10-14-score-submit-400-large-seed.md`) → draft 삭제, 태그는 남음 |
| `v0.1.3` | `fb29d24` | 수동 실행 run 38563811699(2026-10-14) | dmg, zip, sha256 둘 | 점검 통과, draft 작성. 그 뒤 空POOR 400·기록 손실이 발견되어 게시하지 않고 v0.1.4 로 넘어감(사용자 결정 2026-10-14: draft 는 지우지 않고 둠) |
| `v0.1.4` | `87eacd0`(release 워크플로의 버전 커밋, 부모 `51f3a1e`) | 수동 실행 run 38696475211(2026-10-15) | dmg, zip, sha256 둘 | 메인이 내려받아 확인: SHA-256 일치(둘), dmg·앱 `stapler validate`, Gatekeeper `accepted, source=Notarized Developer ID`(dmg·앱), `codesign --verify --deep --strict`, `syspolicy_check distribution` 통과, `CFBundleShortVersionString` 0.1.4, `CFBundleIconFile` R-BMS(`R-BMS.icns`), `x86_64 arm64`, `rbms-player --version` = 0.1.4, Windows exe 리소스 `RT_ICON` 7·`RT_GROUP_ICON` 1. draft 본문은 v0.1.0 대비 릴리스 노트(다른 사이트 이름 0). **게시됨**: `publishedAt` 2026-10-15T07:27:28Z. 에셋은 네 개(dmg 26,596,924바이트, zip 12,911,751바이트, 체크섬 둘) |
| `v0.1.5` | `9ee3481`(release 워크플로의 버전 커밋, 부모 `109061c`) | 수동 실행 run 39245774265(2026-10-19) | dmg, zip, sha256 둘 | 메인이 내려받아 확인: SHA-256 일치(둘), dmg·앱 `stapler validate`, Gatekeeper `accepted, source=Notarized Developer ID`(dmg·앱), `codesign --verify --deep --strict`, `syspolicy_check distribution` 통과, `CFBundleShortVersionString` 0.1.5, `CFBundleIconFile` R-BMS(`R-BMS.icns`), `x86_64 arm64`, `rbms-player --version` = 0.1.5, dmg 에 앱·`Applications`·문서 셋, Windows zip 에 exe·문서 셋, exe 리소스 `RT_ICON` 7·`RT_GROUP_ICON` 1. draft 본문은 v0.1.4 대비 노트(다른 사이트 이름 0). 에셋 dmg 27,136,494바이트, zip 12,928,385바이트. **draft**, 게시는 사용자 |
| `v0.1.6` | `2bb868b`(release 워크플로의 버전 커밋, 부모 `eb2f238`) | 수동 실행 run 39318721794(2026-10-20) | dmg, zip, sha256 둘 | 메인이 내려받아 확인: SHA-256 일치(둘), dmg·앱 `stapler validate`, Gatekeeper `accepted, source=Notarized Developer ID`(dmg·앱), `codesign --verify --deep --strict`, `syspolicy_check distribution` 통과, `CFBundleShortVersionString` 0.1.6, `CFBundleIconFile` R-BMS(`R-BMS.icns`), `x86_64 arm64`, `rbms-player --version` = 0.1.6, dmg 에 앱·`Applications`·문서 셋, Windows zip 에 exe·문서 셋, exe 리소스 `RT_ICON` 7·`RT_GROUP_ICON` 1. draft 본문은 v0.1.5 대비 노트(판별 trace·과거 판 채우기, 다른 사이트 이름 0). 에셋 dmg 27,351,327바이트, zip 13,032,424바이트. **draft**, 게시는 사용자 |
| `v0.1.7` | `d438294`(release 워크플로의 버전 커밋, 부모 `994def3`) | 수동 실행 run 39812316178(2026-10-23, 사용자가 직접 실행) | dmg, zip, sha256 둘 | 메인이 내려받아 확인: SHA-256 일치(둘), dmg·앱 `stapler validate`, Gatekeeper `accepted, source=Notarized Developer ID`(dmg·앱), `codesign --verify --deep --strict`, `syspolicy_check distribution` 통과, `CFBundleShortVersionString` 0.1.7, `CFBundleIconFile` R-BMS(`R-BMS.icns`), `x86_64 arm64`, `rbms-player --version` = 0.1.7, Windows zip 에 exe·문서 셋, exe 리소스 `RT_ICON` 7·`RT_GROUP_ICON` 1. draft 본문은 2026-10-22 고도화 노트(옵션 패널, 어시스트 등급, 색 행, 글자, 다른 사이트 이름 0). 에셋 dmg 27,923,586바이트, zip 13,307,121바이트. 사용자가 게시(2026-10-23 확인, Latest) |
| `v0.1.8` | `e92866a`(release 워크플로의 버전 커밋, 부모 `e9d2ed7`) | 수동 실행 run 39970364536(2026-10-23, 사용자가 직접 실행) | dmg, zip, sha256 둘 | 사용자 판단으로 내려받지 않고 실행 로그로 확인: 앱 `codesign` valid on disk·Designated Requirement, 앱·dmg 공증 `status: Accepted` 와 스테이플·검증 성공, Gatekeeper `accepted, source=Notarized Developer ID`(앱·dmg), `Cargo.toml` 0.1.8, 네 잡 success. 내려받아 하던 SHA-256 대조·`x86_64 arm64`·`--version`·아이콘·Windows `RT_ICON` 점검은 이번에 하지 않았다. draft 본문은 한글 노트(난이도표·선곡, 코스·단위, 플레이, Discord, 알려진 것, 설치). 에셋 dmg 28,040,939바이트, zip 13,619,927바이트. 사용자가 게시(2026-10-23 14:52 UTC, Latest) |
| `v0.1.9` | `49fe493`(release 워크플로의 버전 커밋, 부모 `3b8892e`) | 수동 실행 run 40055763447(2026-10-24, 사용자 지시 "드래프트까지 다만들어놔" 로 메인이 실행) | dmg, zip, sha256 둘 | 실행 로그로 확인: 앱 `codesign` valid on disk·Designated Requirement, 앱·dmg 공증 `status: Accepted` 와 스테이플·검증 성공, Gatekeeper `accepted, source=Notarized Developer ID`, 네 잡 success. 사용자가 draft 를 내려받아 확인하기로 함. draft 본문은 한글 노트(언어, 선곡 화면, 랭킹 보기, 리플레이 배치로 치기, 알려진 것, 설치). 에셋 dmg 29,037,777바이트, zip 13,848,668바이트. 게시 전(draft) |

`Cargo.toml` 의 workspace 버전은 가장 최근 릴리스 커밋의 값(지금 0.1.9)입니다. 원격 태그는 `v0.1.0`~`v0.1.9` 입니다(`git ls-remote --tags origin`). 게시 상태는 `v0.1.4`·`v0.1.6`·`v0.1.7`·`v0.1.8` 게시(v0.1.8 이 Latest, 2026-10-24 확인), `v0.1.0`·`v0.1.3`·`v0.1.5`·`v0.1.9`(2026-10-24 만듦) 는 draft, `v0.1.1`·`v0.1.2` 는 draft 를 지우고 태그만 남았습니다(`gh release list`, 2026-10-15 확인).

## 3. 서명·공증과 보안 경계

- 워크플로의 `permissions: contents: write`와 기본 `GITHUB_TOKEN`은 버전 커밋·태그·GitHub Release 생성에 쓰입니다. 이 토큰만으로 macOS 공증이나 Windows Authenticode 서명은 되지 않습니다.
- macOS 산출물의 서명과 공증은 아래 시크릿이 저장소에 등록돼 있을 때만 수행합니다(2026-10-12 추가). 등록돼 있지 않으면 ad-hoc 서명만 하고 공증하지 않습니다. 현재 저장소에는 여섯 개(`MACOS_CERTIFICATE_P12`, `MACOS_CERTIFICATE_PASSWORD`, `APPLE_ID`, `APPLE_APP_SPECIFIC_PASSWORD`, `APPLE_TEAM_ID`, 웹용 `WEB_DATABASE_URL`)가 등록돼 있습니다(`gh api repos/B-HS/R-BMS/actions/secrets` 로 이름만 확인). Windows 산출물은 Authenticode 서명 단계가 없습니다(사용자: 나중에). 외부 키, 인증서, 공증 자격 증명의 값은 이 저장소나 문서에 넣지 않습니다.

### 3.1 macOS 서명·공증 시크릿

등록 위치: GitHub 저장소 Settings → Secrets and variables → Actions → Repository secrets. 이름은 `release.yml` 의 `build-macos` 잡이 읽는 이름과 같아야 합니다. 유지보수자의 다른 macOS 배포(TAIDE)가 쓰는 이름·방식과 같게 맞췄으므로 같은 값을 그대로 등록하면 됩니다.

| 시크릿 이름 | 내용 | 쓰이는 단계 |
| --- | --- | --- |
| `MACOS_CERTIFICATE_P12` | Developer ID Application 인증서와 개인 키를 내보낸 `.p12` 파일의 base64 문자열(줄바꿈·공백이 섞여도 됨) | 서명 |
| `MACOS_CERTIFICATE_PASSWORD` | 그 `.p12` 를 내보낼 때 정한 비밀번호 | 서명 |
| `APPLE_ID` | 공증에 쓰는 Apple 계정 이메일 | 공증 |
| `APPLE_APP_SPECIFIC_PASSWORD` | 그 계정의 앱 전용 비밀번호(`xxxx-xxxx-xxxx-xxxx`) | 공증 |
| `APPLE_TEAM_ID` | 10자리 Team ID | 공증 |
| `APPLE_SIGNING_IDENTITY`(선택) | 서명 주체 이름을 직접 지정할 때만. 없으면 인증서에서 읽는다 | 서명 |

- 앞의 둘만 있으면 앱을 Developer ID 로 서명(hardened runtime, 보안 타임스탬프)하는 데까지만 가고, 앱 공증·dmg 서명과 공증·Gatekeeper 확인 단계는 건너뜁니다(디스크 이미지는 서명되지 않습니다). 계정 셋까지 다 있어야 그 단계들이 `notarytool` 로 돕니다.
- 인증서는 잡 안에서 만든 임시 키체인에만 들어가고 잡이 끝나면 키체인을 지웁니다. 키체인 비밀번호는 잡마다 새로 만들므로 시크릿이 아닙니다.
- 공증 티켓은 앱과 dmg 에 붙입니다(`xcrun stapler staple`, 이어서 `validate`). 단독 실행 파일일 때(v0.1.0, v0.1.1)는 티켓을 붙일 수 없어 Gatekeeper 가 처음 실행할 때 온라인으로 티켓을 확인했고, 그 때문에 v0.1.1 이 막혔습니다(2.3절).
- 시크릿이 없는 저장소의 ad-hoc 서명판은 처음 열 때 사용자가 격리 속성을 지워야 합니다(`xattr -dr com.apple.quarantine R-BMS.app`, macOS 의 일반 절차이며 이 저장소에서는 시험하지 않았습니다). 이 저장소의 릴리스는 서명·공증판이라 해당되지 않습니다(`release.yml` 의 ad-hoc 단계 주석이 이 문서를 가리킵니다).
- `prod` 브랜치는 없으므로 보호 규칙 문제도 없습니다. 릴리스는 `dev` 에서 수동으로 실행하고 봇이 `dev` 에 커밋을 올립니다. `dev` 에는 보호 규칙과 ruleset 이 없습니다(2026-10-14, GitHub API 로 확인). `prod` 경로를 쓰게 되면 보호 규칙이 자동 버전 커밋의 푸시를 막을 수 있으므로 그때 정책을 정합니다.

### 3.2 시크릿 점검 워크플로 (`.github/workflows/signing-check.yml`)

시크릿을 바꾼 뒤 빌드 없이 값의 꼴만 확인하는 수동 워크플로입니다(Actions 의 `signing-check` → Run workflow, 또는 `gh workflow run signing-check.yml --ref dev`). macOS 러너에서 1분 안에 끝나며 값은 출력하지 않습니다(`APPLE_SIGNING_IDENTITY` 는 점검 대상이 아닙니다).

1. `MACOS_CERTIFICATE_P12` 가 base64 인가: 공백을 뺀 바이트 수, base64 알파벳 밖 바이트 수, 디코드한 바이트 수를 출력합니다. 밖의 바이트가 4개 이하면 그 바이트(16진수)와 첫 위치도 냅니다.
2. 인증서가 `MACOS_CERTIFICATE_PASSWORD` 로 열리고 `Developer ID Application` 신원(개인 키 포함)이 들어 있는가: 개수만 출력합니다.
3. `APPLE_ID`, `APPLE_APP_SPECIFIC_PASSWORD`, `APPLE_TEAM_ID` 로 `notarytool` 이 로그인되는가(`notarytool history`).

만든 이유: 릴리스 실행은 빌드에 10분 넘게 걸린 뒤에야 서명 단계에 닿았습니다. 2026-10-12~13 의 run 38253353651 시도 1·2 가 `base64: stdin: (null): error decoding base64 input stream` 으로 실패했는데 그 문구로는 값의 어디가 틀렸는지 알 수 없었습니다. 같은 문구는 값에 따옴표가 붙었거나, `.p12` 의 원본 바이트나 파일 경로를 넣었을 때 납니다(로컬 `base64 --decode` 로 확인). 릴리스 워크플로가 지금은 같은 점검을 빌드 앞에서 하지만(2절), 시크릿을 바꾼 직후에는 빌드 없이 보는 이 워크플로가 더 빠릅니다.

시크릿을 넣는 명령(값이 화면에 나오지 않습니다): `base64 -i certificate.p12 | gh secret set MACOS_CERTIFICATE_P12 --repo B-HS/R-BMS`

### 3.3 로컬 선검증 스크립트 (`scripts/`)

시크릿을 등록하기 전에 로컬 macOS 에서 같은 절차를 돌려 보는 스크립트입니다. 사용자의 다른 저장소 TAIDE 의 것을 가져와(2026-10-13, 사용자 지시) 이 저장소의 단계 이름에 맞추고 `--set-secret` 을 더했습니다. TAIDE 도 2026-08-28 첫 릴리스에서 같은 `error decoding base64 input stream` 을 겪고 이 스크립트를 만들었습니다.

- `bash scripts/verify-signing-cert.sh <cert.p12 | base64.txt> [--set-secret]`: `.p12` 원본이면 임시 키체인에 넣어 암호와 서명 신원을 확인하고 secret 용 한 줄 base64 를 만듭니다. base64 텍스트 파일이면 먼저 형식을 엄격히 봅니다(base64 밖 문자 수, 길이가 4의 배수인가, 디코드 뒤 다시 인코드한 값이 같은가). 암호는 `CERT_PASSWORD` 환경 변수나 프롬프트로만 받습니다. 서명 신원은 CI 와 같이 `security find-identity -v` 로 찾으므로 만료됐거나 신뢰되지 않는 신원은 통과하지 못합니다. `--set-secret` 을 주면 검증을 통과한 값과 암호를 `gh secret set` 으로 이 저장소의 `MACOS_CERTIFICATE_P12`, `MACOS_CERTIFICATE_PASSWORD` 에 바로 넣습니다(클립보드와 웹 화면을 거치지 않습니다. 붙여 넣은 값의 한 글자가 `.` 으로 바뀌어 릴리스가 세 번 실패한 일이 있습니다). 주지 않으면 TAIDE 와 같이 클립보드에 복사합니다.
- `bash scripts/verify-notary-credentials.sh`: `APPLE_ID`, `APPLE_TEAM_ID`, 앱 암호(`NOTARY_PASSWORD`)를 환경 변수나 프롬프트로 받아(앱 암호 프롬프트는 입력이 화면에 나오지 않습니다) `xcrun notarytool history` 로 Apple 공증 서버에 실제로 로그인해 봅니다(제출 없음). 형식이 이상하면 경고하고, 실패하면 Apple 의 오류와 원인 후보를 냅니다.
- 같은 폴더의 `macos-app.sh`(앱 번들 만들기: `bash scripts/macos-app.sh <rbms-player 바이너리> <버전> <빈 출력 폴더>`)와 `macos-notarize.sh`(`bash scripts/macos-notarize.sh <R-BMS.app | 파일.dmg>`, `APPLE_ID`·`APPLE_APP_SPECIFIC_PASSWORD`·`APPLE_TEAM_ID` 환경 변수)는 릴리스 워크플로가 부르는 스크립트이고 로컬에서도 같은 인자로 돌릴 수 있습니다. 인자 없이 실행하면 사용법을 내고 종료 코드 2 로 끝납니다. `scripts/` 에는 그 밖에 Windows 러너 전용 `windows-icon-resource.ps1`(2절 Windows)이 있고, 로컬 macOS 에서는 돌리지 않습니다.

확인한 것(2026-10-13, 일회용 자체 서명 인증서와 가짜 `gh`·`pbcopy`): `.p12` 의 맞는 암호와 틀린 암호, 유효한 base64 파일, 한 글자가 `.` 으로 바뀐 파일(`base64 밖 문자 1`), 잘린 파일, `--set-secret` 이 넘기는 값이 클립보드 값과 같음, 사용법 출력. 실제 인증서와 실제 계정으로는 사용자가 돌립니다(에이전트는 키 파일을 읽지 않습니다). 2026-10-14 에 네 스크립트 모두 `bash -n` 구문 검사와 인자 없는 실행(사용법 출력)을 확인했고, 2026-10-15 에 다시 확인했습니다(`bash -n` 네 개 통과, `macos-app.sh`·`macos-notarize.sh`·`verify-signing-cert.sh` 는 인자 없이 종료 코드 2. `verify-notary-credentials.sh` 는 인자를 받지 않고 값을 프롬프트로 묻습니다). 같은 날 일회용 자체 서명 인증서로 `find-identity` 가 `-v` 없이는 신뢰되지 않는 신원(`CSSMERR_TP_NOT_TRUSTED`)을 내고 `-v` 를 주면 거르는 것과, `verify-signing-cert.sh` 가 그 인증서에서 `[3/3 실패]` 로 끝나는 것을 확인했습니다.

순서: 로컬 스크립트 둘 → `signing-check` 워크플로(3.2절, 등록된 값 확인) → 릴리스.

첫 통과 실행(태그 `v0.1.0`, run 38253353651 의 세 번째 시도, 2026-10-13 18:31~18:50 UTC = 2026-10-14 KST): Developer ID 서명과 공증(`status: Accepted`)이 통과해 draft 릴리스가 만들어졌습니다. 이 실행의 워크플로 정의는 태그가 가리키는 커밋의 것이라 빌드 전 자격 증명 점검(`287b80d`)이 아직 없던 순서입니다. 그 앞의 실패는 순서대로 툴체인에 타깃 없음(2절 툴체인 항목), 인증서 값의 한 글자(시도 1·2), Apple 개발자 계정의 계약 미동의(`signing-check` 가 `HTTP 403. A required agreement is missing or has expired` 로 알림. 동의 뒤 반영까지 몇 분)였습니다. 내려받은 macOS 실행 파일은 `x86_64 arm64`, 하드닝 런타임, Developer ID 체인으로 확인했습니다. 단독 실행 파일이라 `spctl --assess --type execute` 는 "does not seem to be an app" 으로 판정하지 않으며, 이는 실패가 아닙니다.

## 4. 사용자 실행 전제

- macOS: `R-BMS-macos-universal.dmg` 를 열어 `R-BMS.app` 을 Applications 로 끌어 놓고 실행합니다. 서명·공증판이고 티켓이 붙어 있습니다. 터미널에서 곡 폴더를 넘기려면 `/Applications/R-BMS.app/Contents/MacOS/rbms-player /path/to/songs` 입니다(루트 [README.md](../README.md)가 정본). Finder 로 열 때 붙는 `-psn_...` 인자는 무시합니다. `rbms-player --version` 과 `--help`(`-h`)는 창을 띄우지 않고 설정·기록·서버를 건드리지 않은 채 끝납니다.
- Windows: `rbms-windows-x64.zip` 압축 해제 후 `rbms-player.exe <곡폴더>`로 실행합니다. 코드 서명이 없어 SmartScreen 이 한 번 묻습니다(평판이 쌓이면 달라질 수 있음). 실제 Windows 에서 열어 본 기록은 없습니다(`docs/reference/windows-compat.md` 의 런타임 항목, 테스트 부채 3.3).
- 인자 없이 실행하면 플레이어는 기억한 `songs_folder`, `RBMS_SONGS`, 빈 GUI 순으로 진입합니다. 자세한 실행과 GUI 폴더 등록은 루트 [README.md](../README.md)를 정본으로 사용합니다.

## 5. 릴리스 체크리스트와 남은 일

릴리스마다 2.2절의 순서를 따릅니다(CI 확인 → 필요하면 `signing-check` → 수동 실행 → `git pull`·`git fetch --tags` → 산출물 점검 → 노트 → 사용자 게시). 남은 일은 다음과 같습니다.

1. (해소 2026-10-15) v0.1.4 게시: 2026-10-15T07:27:28Z 에 게시됨. 게시된 사본의 실기 확인(`IR: RANK`, SYNC RECORDS NOW, 키 입력, JUDGE OFFSET 재조정, PAD DEBOUNCE)은 [실제 창 QA](quality-assurance/2026-10-11-default-skin-manual-check.md) 0.1절에 남아 있습니다.
2. `v0.1.0`·`v0.1.3` draft 는 사용자 결정으로 그대로 둔다(게시하지 않음).
3. Windows 코드 서명(사용자: 나중에)과 Windows 실제 실행 확인.
4. (해소 2026-10-15) 앱 아이콘: [`assets/icon/`](../assets/icon/README.md)(생성 스크립트 `assets/icon/tools/generate-icon.py`, `--check`), macOS `R-BMS.icns`, Windows `rbms.ico`(리소스로 링크), 웹 파비콘.

같은 400 류 사고를 막는 교차 테스트는 끝났다(`docs/quality-assurance/2026-10-14-test-debt.md` 2절, 6절의 `web.yml`).

## 6. 웹 (`web/`) CI 와 배포

- `.github/workflows/web.yml`: `web/**`, `contract/**`(클라이언트 요청 계약 파일. 웹 테스트 `web/tests/dto/client-requests.test.ts` 가 읽는다) 또는 이 파일이 바뀐 `dev`·`prod` 푸시와 PR 에서 Ubuntu 잡 둘이 함께 돈다. 같은 ref 의 이전 실행은 취소된다(`web-${{ github.ref }}`). Bun 은 `web/package.json` 의 `packageManager`(`bun@1.4.0`)를 따른다.
  - `verify`: `bun install --frozen-lockfile` → `bun run typecheck`(`next typegen && tsc --noEmit`) → `bun run lint` → `bun run test` → `bun run format:check` → `bun run build`. **빌드에는 환경 변수를 주지 않는다**(DB·시크릿 없이 성공해야 한다. `NEXT_TELEMETRY_DISABLED` 만 준다). 이 잡의 `bun run test` 에서는 `web/tests/db/` 가 `RBMS_DB_E2E` 가 없어 건너뛴다. 제한 시간 15분(최근 실행 run 38694945226 에서 0.9분).
  - `db`: MySQL 서비스 컨테이너(`mysql:8.4.11`, `mysqladmin ping` 헬스체크가 통과한 뒤 시작)에 `bun install --frozen-lockfile` → `bun run db:migrate` → `bun run db:status`(최신이 아니면 실패) → `bun test tests/db` 를 돈다. 잡 환경 변수는 `DATABASE_URL=mysql://rbms:rbms@127.0.0.1:3306/rbms` 와 `RBMS_DB_E2E=1` 이다. 접속 값은 잡마다 새로 뜨고 사라지는 컨테이너의 것이라 시크릿이 아니다. 제한 시간 15분. 가장 최근 실행(run 38694945226, 51f3a1e)은 0.7분에 마이그레이션 0000~0004 적용, `db:status` 최신, `bun test tests/db` 8개 파일 57개 테스트 통과였다. 처음 이 잡을 만든 2026-10-14 에는 같은 이미지·같은 순서를 로컬 Docker 로 돌려 마이그레이션 0000~0002 적용, `bun test tests/db` 8개 통과를 확인했다. 그 전의 테스트는 판 일곱 개를 1~7밀리초 간격으로 넣어, 같은 초를 같은 판으로 보는 멱등 규칙에 걸려 7개가 실패했다(1 통과 7 실패). 판 사이를 1초로 벌려 고쳤다. `web/tests/db/` 의 파일 여덟 개는 모두 `RBMS_DB_E2E` 가 없으면 건너뛴다.
- `ci.yml`(Rust 3 OS 빌드)의 두 트리거(push, pull_request)에는 `paths: ["**", "!web/**", "web/data/tables/**"]` 가 있어, `web/` 만 바뀐 푸시와 PR 에서는 Rust 빌드를 건너뛰고 이 워크플로만 돈다(웨이브 17B-1 통합에서 추가. 변경이 `web/` 밖의 파일을 하나라도 포함하면, 또는 `web/data/tables/` 를 건드리면 `ci.yml` 도 돈다). GitHub API 로 읽어 보니 `dev` 브랜치에 보호 규칙과 ruleset 이 없어, 필수 체크가 대기 상태로 남는 문제는 지금 없다. 나중에 브랜치 보호에서 `test` 나 `verify` 를 필수 체크로 삼으면, 경로 필터 때문에 해당 경로를 건드리지 않은 PR 에서 그 체크가 실행되지 않아 대기 상태로 남는다(GitHub 의 알려진 동작).
- 배포는 Vercel 이 한다(프로젝트 `rbms-web`, Production Branch `dev`, Root Directory `web`, Git 연결 `B-HS/R-BMS`). `web/vercel.json` 의 `ignoreCommand`(`if [ "$VERCEL_ENV" != "production" ]; then exit 0; fi; git diff --quiet "$VERCEL_GIT_PREVIOUS_SHA" HEAD -- . || exit 1`)가 프로덕션이 아닌 빌드와 `web/` 이 바뀌지 않은 푸시를 건너뛴다. Vercel 위에서 이 명령이 실제로 어떻게 도는지는 아직 푸시로 확인하지 못했다(테스트 부채 3.7). 운영 DB 마이그레이션은 **배포보다 먼저** 적용한다(6.1절의 `web-db` 워크플로로 메인이 한다). 설정값, 환경 변수, 순서, 기준선 절차는 `docs/web/architecture.md` 3-2 와 6-2 가 정본이다.
- 웹 CI 는 릴리스(`release.yml`)와 무관하다. 앱 릴리스 태그·`prod` 푸시가 웹을 배포하지 않는다(웹은 `dev` 푸시 기준).

### 6.1 운영 DB 워크플로 (`.github/workflows/web-db.yml`)

운영 DB(`https://bms.hyuns.uk` 의 MySQL)에 대한 수동 작업입니다. 운영 `DATABASE_URL` 은 Vercel 에 Secret 으로 있어 `vercel env pull`·`vercel env run` 으로 받을 수 없으므로("2 Secret values cannot be pulled" 확인) 저장소 시크릿 `WEB_DATABASE_URL` 에 따로 두었습니다(사용자 등록). 값은 출력하지 않습니다. 실행: Actions 의 `web-db` → Run workflow, 또는 `gh workflow run web-db.yml --ref dev -f action=<동작>`. 잡은 `web/` 에서 도는 Ubuntu 한 개(제한 10분, `concurrency: web-db` 에 취소 없음)이고 첫 단계가 시크릿이 비었으면 실패합니다. 이후 `bun install --frozen-lockfile` 을 합니다.

| 동작 | 실행되는 것 |
| --- | --- |
| `status` | `bun run db:status`(`web/scripts/db-migration-status.ts`, 읽기 전용). 종료 코드 0(최신)은 성공, 1(할 일 있음: 대기 마이그레이션이나 기준선 필요)은 성공으로 끝내되 `::warning::` 주석을 달고, 2(접속 실패)는 잡을 실패시킵니다. 1 의 원인은 로그의 `Result:` 줄에 있습니다 |
| `migrate` | 상태 → `bun run db:migrate`(`drizzle-kit migrate`) → 상태. 적용 전 상태는 종료 코드 1(대기 중이 정상)을 허용하고 2(접속 실패)에서 멈춥니다. 적용 뒤 상태가 최신이 아니면(종료 코드 1 이나 2) 잡이 실패합니다 |
| `cleanup-count` | `bun scripts/run-sql.ts cleanup-test-accounts.count.sql`: 시험 계정 다섯 개와 그 자료의 개수를 셉니다 |
| `cleanup-delete` | 세기 → `cleanup-test-accounts.delete.sql`(한 트랜잭션) → 다시 세기 |
| `chart-notes-count` | `chart-notes.count.sql`: 받아들인 판과 다른 `chart.notes`, 복구가 바꿀 행, 백업 표 유무 등을 셉니다 |
| `chart-notes-repair` | 세기 → `chart-notes.repair.sql`(바꿀 행을 `chart_notes_backup` 에 담고 바꿈. 표가 있으면 멈춤) → 다시 세기 |
| `chart-notes-restore` | 세기 → `chart-notes.restore.sql`(복구 뒤 다시 바뀌지 않은 행만 옛 값으로) → 다시 세기 |
| `chart-notes-drop-backup` | 세기 → `chart-notes.drop-backup.sql`(`chart_notes_backup` 삭제. 복구를 다시 돌리려면 먼저) → 다시 세기 |
| `admin-seed-count` | `admin-seed.count.sql`: 마이그레이션 0006 의 관리자 지정이 바꿀 행을 셉니다(읽기 전용) |
| `assist-grade-backfill-count` | `assist-grade.count.sql`: 낡은 `chart_best.assist_grade` 행을 셉니다 |
| `assist-grade-backfill` | 세기 → `assist-grade.backfill.sql`(낡은 등급을 그 판의 값으로 다시 계산. 마이그레이션 0011 이 한 것과 같고 되풀이해도 안전) → 다시 세기 |
| `chart-purge-count` | `bun scripts/run-sql.ts chart-purge.count.sql`: 사이트 주인의 요청으로 지운 차트 하나(md5 가 SQL 에 고정)의 행을 표별로 셉니다([ADR](acknowledge/2026-10-23-adr-chart-removed-at-owner-request.md)) |
| `chart-purge-delete` | 세기 → `chart-purge.delete.sql`(한 트랜잭션, 그 차트의 점수·최고 기록·리플레이·trace·패턴·표 행·폴더 항목·AI 결과를 지우고 `chart` 행은 코스 정의가 쓰지 않을 때만 지움) → 다시 세기 |
| `courses-seed-count` | `bun scripts/seed-courses.ts count`: `web/data/courses/genoside-2018/courses.json` 을 읽어 코스 등록 계획을 묶음별 숫자(inserted·changed·unchanged·deactivated·danCourses·newDanCourses)로만 냅니다. 쓰지 않습니다 |
| `courses-seed` | 세기 → `seed-courses.ts apply`(코스 정의·출처·묶음·단위 코스를 한 묶음씩 트랜잭션으로 등록) → 남은 계획을 다시 세어 바뀔 것이 0 이 아니면 실패 |
| `tables-refresh-count` | `bun scripts/refresh-tables.ts count`: 러너가 Stella·Satellite 의 `header.json`·`score.json` 을 받아 검증하고 저장된 사본과 비교만 합니다(쓰지 않음). 표마다 outcome·reason·`entries`·`added`·`removed`·`releveled` 한 줄 |
| `tables-refresh-force` | 위 확인 → `refresh-tables.ts apply-large`(하루치 검증 중 급감 10%·변경 25% 한도를 풀고 저장) → 다시 확인. 거절된 표가 있으면 실패 |
| `dan-recompute-count` | `bun scripts/dan-recompute.ts count`: 모든 단위 코스(`dan_course`)의 저장된 판 중 `stages` 가 있는 것을 다시 판정해 숫자만 냅니다. 쓰지 않습니다. 출력 한 줄: `plan danCourses N, scanned N, eligible N, awarded N, eligibleAssisted N, awardedAssisted N, assistedLegacy N`(2026-10-24 부터). `eligible`·`awarded` 는 정식 단위만 센 값이고 `*Assisted` 는 A-단위(어시스트 판)다. `awarded`·`awardedAssisted` 는 count 에서 항상 0 이고(쓰지 않으므로), 이미 얻은 판도 `eligible*` 에 센다. `assistedLegacy` 는 `ranked_without_assist` 를 모르는 0013 이전 판이 "빌드가 막지 않았다"는 추정으로 A-단위 대상이 된 판 수로, 빌드 해시가 막았던 판이 섞일 수 있는 수의 **상한**이다. 소급을 적용하기 전에 이 숫자를 사용자에게 보인다([A-단위 ADR](acknowledge/2026-10-24-adr-assisted-dan.md)) |
| `dan-recompute` | 세기 → `dan-recompute.ts apply`(종류별로 처음 획득만 남기므로 되풀이해도 두 번째 `applied` 줄은 `awarded 0, awardedAssisted 0`) → 다시 세기. 로그는 `plan`, `applied`, `plan` 세 줄이고 새로 준 수는 `applied` 줄의 `awarded`·`awardedAssisted` 다(`plan` 줄의 `awarded*` 는 쓰지 않으므로 늘 0). 운영에서 한 번도 돌리지 않았다면 이번 실행이 정식 단위의 첫 소급도 겸하므로 `awarded` 가 A-단위와 함께 늘어난다 |

- 실행 이력(2026-10-14): `status` run 38548385742(0000 만 기록, 0001·0002 대기) → `migrate` 38548459519(`__drizzle_migrations` 3행, 최신) → `cleanup-count` 38548532538(계정 5, score 7, chart_best 3, replay 5, submission_audit 13, rival 1, setting_blob 3, api_token 11, session 10, account 5, 다른 사용자의 score 3) → `cleanup-delete` 38549827633(사용자 승인, 대상 전부 0, 다른 사용자의 score 3 그대로). 같은 날 저녁 `status` run 38627383852(0003 대기, 종료 코드 1 이라 `::warning::` 만) → `migrate` run 38627474716(`0003_replay_user_quota`, `replay.user_id` 색인 `replay_user_idx`). 2026-10-15 `migrate` run 38673724720(`0004_course_score_judge_totals`, `course_score` 에 판정 수·`max_ex_score` 열 추가, `__drizzle_migrations` 5행) → `chart-notes-count` run 38673771025(받아들인 판과 `chart.notes` 가 다른 행 0, 복구가 바꿀 행 0, 점수가 하나도 없는 차트 행 3, 누적 거절 제출 39, `chart_notes_backup` 표 0. `chart-notes-repair` 는 돌리지 않았다). 2026-10-15 시점의 운영 DB 는 마이그레이션 `0000_smooth_cerebro`, `0001_user_folders`, `0002_scratch_auto_bests`, `0003_replay_user_quota`, `0004_course_score_judge_totals` 가 적용돼 있었습니다(그 뒤 0012 까지의 적용은 아래 2026-10-22~23 줄. `web/` 를 바꾼 마지막 커밋은 `9e2cc0e` 이다. 이후 `51f3a1e`·`ea698a9` 는 `web/` 를 바꾸지 않아 Vercel 의 `ignoreCommand` 가 건너뛴다. 배포 상태를 Vercel 에서 직접 확인하지는 않았다. 테스트 부채 3.7).
- 실행 이력(2026-10-22~23): `migrate` run 39693988962(`0011`, 어시스트 등급) → `assist-grade-backfill-count` run 39733088817(낡은 행 0). 2026-10-23 `migrate` run 39909099575(`0012_course_dan_tables`, 앞 `status` 가 12행·`would_run=yes`, 적용 뒤 최신. 사용자가 `!` 로 직접 실행) → `chart-purge-count` run 39926928236 → `chart-purge-delete` run 39927201095(사용자 요청의 차트 하나, 지운 뒤 다시 센 값이 전부 0) → `courses-seed-count` run 39938376095(`genoside-2018-sp`·`genoside-2018-dp` 각 inserted 22, danCourses 22, 이미 등록된 것 0) → `tables-refresh-count` run 39938388900(`stella-st` 2,350곡·`stella-sl` 2,497곡이 saved 로 저장 가능, 둘 다 첫 저장이라 added 가 곧 항목 수). `courses-seed` run 39946701885(커밋 76e026a, 앞 세기 `genoside-2018-sp`·`genoside-2018-dp` 각 inserted 22·danCourses 22·newDanCourses 22, 등록 뒤 남은 계획과 다시 센 값은 두 묶음 모두 inserted 0·unchanged 22·newDanCourses 0, 곧 SP 22개·DP 22개 등록) → `tables-refresh-force` run 39946847395(같은 커밋, `stella-st` 2,350곡·`stella-sl` 2,497곡이 saved, 둘 다 첫 저장이라 added 가 항목 수와 같고 removed·releveled 0). 이 두 run 은 `web-db` 로 돌렸고 `dan-recompute-count`·`dan-recompute` 는 2026-10-23 기준 돌리지 않았습니다. 2026-10-24 에 마이그레이션 `0013_assisted_dan` 파일을 커밋(`35f77ae`)했고 운영에는 아직 적용하지 않았습니다(적용 뒤 `dan-recompute-count` 로 `assistedLegacy` 를 보인 다음 사용자가 `dan-recompute` 를 실행하는 순서: [설계서](plan/2026-10-24-assist-dan.md) 10절). 첫 Cron 실행(`CRON_SECRET` 등록과 응답)은 저장소에서 확인되지 않았습니다. 2026-10-24: `status` run 40073142588(0013 대기) → 사용자 `!` `migrate` run 40074383695(`0013_assisted_dan`) → `status` run 40074421795(최신) → `dan-recompute-count` run 40074592117(danCourses 70, scanned 2, eligible 0, eligibleAssisted 1, assistedLegacy 0) → 사용자 `!` `dan-recompute` run 40074842429(awardedAssisted 1, 정식 awarded 0).
- Vercel Cron(2026-10-23 추가): `web/vercel.json` 의 `crons` 가 매일 UTC 15:00(KST 0시)에 `GET /api/cron/tables` 를 부릅니다. 이 경로는 `Authorization: Bearer <CRON_SECRET>` 을 상수 시간 비교로 확인하고, `CRON_SECRET`(16자 이상, Vercel 환경 변수. 값은 저장소·대화·로그에 두지 않습니다)이 없으면 `SERVICE_NOT_CONFIGURED`(503), 맞지 않으면 401 을 냅니다. 이 경로가 `table_snapshot` 과 표 코스 등록·단위 판정을 운영 DB 에 자동으로 쓰므로 "운영 DB 쓰기는 `web-db` 만" 이라는 [운영 DB ADR](acknowledge/2026-10-14-adr-production-db-operations.md) 의 범위가 이 한 곳만큼 넓어졌습니다. 같은 일을 사람이 숫자를 보고 하는 길이 `tables-refresh-count`/`tables-refresh-force` 입니다(러너가 원본으로 나가므로 Vercel 에서 막혀도 사본을 채울 수 있습니다). Hobby 요금제면 실행 시각은 KST 0:00~0:59 사이로 흩어집니다(요금제는 저장소에서 확인되지 않았습니다).
- 순서: 새 마이그레이션이 있는 변경은 `migrate` 를 **먼저** 돌리고 그 다음 웹을 배포합니다. 새 테이블을 쓰는 코드가 먼저 나가면 해당 경로가 500 을 냅니다.
- 이 저장소는 PUBLIC 이라 이 워크플로의 로그도 공개입니다. `run-sql.ts` 는 SELECT 결과의 행 내용을 로그에 찍지 않고 행 수와 열 이름만 냅니다. 예외는 `target`(문자열)·`row_count`(숫자) 두 열만 가진 개수 표이고, 이것만 `console.table` 로 그대로 찍습니다. 그래도 개수 표의 `target` 에 사용자 정보를 담는 SQL 은 `web/scripts/` 에 넣어 돌리지 않습니다. count 쿼리의 첫 SELECT(시험 계정의 `id`·`login_id`·`name`·`created_at`)는 이제 행 수와 열 이름만 로그에 남고, 개수 표(두 번째 SELECT)가 결과입니다.
- 시험 계정 목록은 SQL 파일에 고정된 다섯 아이디(`e2e-phase-i-18d38e33cabb`, `e2e-phase-i-18d391944f88`, `gui-e2e-871795`, `probe1`, `w2b2probe`)입니다. 이미 지웠으므로 다시 돌려도 0 개를 지울 뿐입니다. `crates/rbms-ir/tests/e2e_local.rs` 는 `RBMS_IR_E2E_URL` 을 주면 `e2e-phase-i-<무작위>` 계정을 새로 만드므로, 운영 주소로 돌리면 이 목록에 없는 계정이 남습니다(운영에는 돌리지 않습니다).

### 6.2 웹 보조 스크립트 (`web/scripts/`)

| 파일 | 하는 일 | 쓰는 곳 |
| --- | --- | --- |
| `db-migration-status.ts` | 마이그레이션 파일(`drizzle/meta/_journal.json`)과 DB 의 `__drizzle_migrations` 를 비교해 파일별 적용 여부, 새 테이블 존재, 결과를 출력(접속 정보는 출력하지 않음). 종료 코드 0 = 최신, 1 = 할 일(대기 마이그레이션 또는 기준선 필요), 2 = 접속 실패. `--baseline-sql` 은 `push` 로 만든 DB 를 기준선으로 채택하는 SQL 을 출력 | `bun run db:status`, `web-db` |
| `run-sql.ts` | `web/scripts/` 아래의 `.sql` 파일 하나를 여러 문장으로 실행하고 SELECT 는 행 수와 열 이름(`target`·`row_count` 개수 표만 내용까지), 그 밖의 문장은 영향 행 수를 출력. 그 밖의 경로는 거절(종료 코드 2) | `web-db` 의 cleanup·chart-notes·admin-seed·assist-grade·chart-purge 동작 |
| `cleanup-test-accounts.count.sql`, `cleanup-test-accounts.delete.sql` | 시험 계정 다섯 개와 딸린 자료를 세고 지움. `session`·`account`·`api_token`·`user_folder`·`user_folder_item` 은 `user` 삭제 시 외래 키가 연쇄 삭제하고, `score.user_id` 는 `set null` 이라 SQL 이 `score` 를 직접 지움 | `web-db` |
| `chart-purge.count.sql`, `chart-purge.delete.sql` | 사이트 주인의 요청으로 발광 표 사본에서 뺀 차트 하나의 운영 행을 세고 지움(md5 가 SQL 에 고정, `chart_pattern` 으로 얻은 sha256 까지 따라가 점수·리플레이·trace·AI 결과를 지움). 지워진 뒤에는 개수 표가 전부 0 | `web-db` ([ADR](acknowledge/2026-10-23-adr-chart-removed-at-owner-request.md)) |
| `seed-courses.ts` | `bun scripts/seed-courses.ts <count\|apply>`: `web/data/courses/genoside-2018/courses.json` 을 Zod 로 읽고 `course_hash` 를 TS 규칙(`courseHashOf`)으로 다시 계산해 파일 값과 다르면 중단한 뒤 `composeCourseCatalog` 로 계획(`count`) 또는 등록(`apply`)한다. 출력은 묶음별 숫자뿐이다(Actions 로그가 공개). 종료 코드 0 = 성공, 1 = 실패 또는 등록 뒤 남은 변경, 2 = 사용법 오류 | `web-db` 의 `courses-seed*` |
| `refresh-tables.ts` | `bun scripts/refresh-tables.ts <count\|apply-large>`: Cron 이 하는 표 갱신(`tableRefreshService.refreshAll`)을 러너에서 한다. `count` 는 `dryRun`(쓰지 않음), `apply-large` 는 급감·변경량 검사만 건너뛰고 저장(형식 검사는 그대로). 출력은 표 id·outcome·reason 코드·수뿐이다. 받지 못한 표(`failed`)가 있으면 종료 코드 1, `apply-large` 는 거절(`rejected`)도 1 | `web-db` 의 `tables-refresh*` |
| `dan-recompute.ts` | `bun scripts/dan-recompute.ts <count\|apply>`: 모든 `dan_course` 의 저장된 판을 다시 판정(`awardForCourses`)해 `danCourses`·`scanned`·`eligible`·`awarded`(정식)와 `eligibleAssisted`·`awardedAssisted`(A-단위)·`assistedLegacy`(0013 이전 판의 추정으로 A-단위 대상이 된 판) 숫자를 낸다(6.1절) | `web-db` 의 `dan-recompute*` |
| `lib/script-db.ts` | 위 세 스크립트의 DB 접속: 환경 변수에서 접속 주소를 풀어 `mysql2` 풀과 drizzle 을 만들고 세션의 잠금 대기 시간을 정한다. `server-only` 를 가져오는 `@server/db`·`compose/index.ts` 를 쓰지 않으려고 따로 둔 파일이며 접속 값은 출력하지 않는다 | 스크립트 |
| `snapshot-table.ts` | `bun scripts/snapshot-table.ts <표 주소> <출력 폴더> [--check]`: 난이도표 원본에서 `header.json`·`data.json`·`source.json` 을 받아 저장하거나(발광 표는 `web/data/tables/insane1/`) `--check` 로 저장본과 일치를 봄(종료 코드 0 = 같음, 1 = 다름, 2 = 실패). 저장할 때 정규화한다: `header.json` 에서 `original_url` 을 빼고, `data.json` 에서 곡마다 `comment` 를 비우며 `url`·`url_diff`·`name_diff`(곡 내려받는 곳) 말고 다른 글자 칸의 웹 주소를 뗀다([표 페이지 ADR](acknowledge/2026-10-14-adr-table-page-no-source-notice.md), dd5518e). 사이트 주인의 요청으로 뺀 차트의 md5 한 개는 `data.json` 을 만들 때 항목에서 빠지고(`ENTRY_MD5_REMOVED`, 발광 표는 1,034곡) `source.json` 의 `method` 에 적힌다. 스냅샷을 다시 만들 수 있게 `source.json` 에는 원본 주소·HTTP 기록이 남는다(웹이 서빙하지 않음. 지울지는 사용자 결정 전) | 수동 |

## 7. KNOWN ISSUE (코드가 문서가 말하는 것과 다르게 동작하는 곳)

새 세션의 할 일입니다. 이전에 있던 워크플로·스크립트 결함 네 건(`web-db` 의 `status` 가 항상 성공, 공증 암호 프롬프트가 입력을 보여 줌, 서명 신원 검색에 `-v` 없음, `release.yml` 에 `concurrency` 없음)은 고쳤고 6.1절과 2절, 3절에 반영했습니다. 남은 다섯은 사용자 결정, 클라이언트 한계 또는 코드 수리 대상이라 이 문서의 수리 대상이 아닙니다(문서만 고치는 작업이라 코드는 건드리지 않았습니다).

1. KNOWN ISSUE: 게시되지 않은 draft `v0.1.0` 의 산출물은 맨 실행 파일의 tar.gz 입니다. 게시하면 사용자는 v0.1.1 과 같은 Gatekeeper 차단을 만납니다. draft `v0.1.3` 도 게시하지 않고 둔 것입니다. 서버 수리가 배포돼 그 빌드의 판도 이제 받아들여지지만, 거절된 판을 다시 보내는 기록 동기화가 없어(`5f4a9a2` 이전의 빌드) 이미 거절된 판은 서버에 올라가지 못합니다. 게시된 `v0.1.4` 가 대신합니다. 고치는 방법: 사용자 결정으로 두 draft 를 지우거나(태그는 남음) 게시하지 않고 둡니다.
2. KNOWN ISSUE: 서버가 2^53 이상의 시드를 0 으로 저장하므로, 그런 시드를 담은 옛 클라이언트의 RANDOM 리플레이는 다시 내려받아 재생하면 배치가 달라질 수 있습니다. 해당 빌드는 게시된 적이 없어 영향이 거의 없습니다(테스트 부채 3.8).
3. KNOWN ISSUE: 같은 판 본문(같은 사용자·차트·`played_at`)을 동시에 두 번 보내면 늦은 쪽 요청이 500 입니다. 멱등 키 조회와 INSERT 가 원자적이지 않아 `score_idempotent_uniq`(코스는 `course_score_idempotent_uniq`)에서 `ER_DUP_ENTRY` 를 받고 점수·코스 쓰기 경로에는 그것을 "이미 있음" 으로 바꾸는 처리가 없습니다(`web/src/server/lib/mysql-error.ts` 의 `DUPLICATE_ENTRY` 는 `compose/folder.ts` 만 씁니다). 저장은 한 번뿐이고 재전송은 중복 응답과 최고 기록 갱신으로 이어집니다(`web/tests/db/best-repair.test.ts`). 고치는 방법과 영향: 테스트 부채 3.72.
4. KNOWN ISSUE: `POST /api/tables` 에서 한 폴더의 `charts` 에 같은 sha256 을, `courses` 에 같은 `course_hash` 를 두 번 넣으면 400 이 아니라 기본 키 위반(500)으로 보이며 그 표의 폴더 저장이 통째로 되돌려집니다. `tableService.upsert` 가 중복을 걸러내지 않는다는 것은 코드와 스키마를 읽은 추정이고 실행해 재현하지는 않았습니다. 관리자 전용이고 운영 `GET /api/tables` 는 지금 `[]` 입니다. 테스트 부채 3.74.
5. KNOWN ISSUE: `rbms-cli config <settings.ron>` 은 읽기 명령인데 플레이어와 같은 `rbms_config::load` 를 써서 파일이 없으면 그 경로에 첫 실행 설정을 쓰고, 읽을 수 없는 파일은 `.ron.bak` 으로 옮깁니다(`crates/rbms-config/src/io.rs`). 점검용으로 안내하기 전에 쓰지 않는 읽기 함수가 필요합니다. 테스트 부채 3.73.
