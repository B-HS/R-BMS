# CI / 릴리스 (GitHub Actions)

> 이 문서는 워크플로 구현과 현재 배포 경계를 설명합니다. 워크플로가 존재한다는 사실은 외부 릴리스 산출물·서명·배포 엔드포인트가 현재 사용 가능하다는 보증이 아닙니다.
>
> 2026-09-16 현재 작업 브랜치는 `dev`이고 원격 추적 브랜치도 `origin/dev`만 있습니다. `prod` 생성, 첫 릴리스/태그, 실제 서명 또는 배포 자격 증명 적용은 유지보수자가 실제 키를 준비한 뒤 함께 수행할 Phase R이며, 이 라운드에서는 시작하지 않습니다.

## 1. CI (`.github/workflows/ci.yml`)

- 트리거: `dev`·`prod`의 push와 두 브랜치를 대상으로 한 pull request.
- `test` 잡: Ubuntu, macOS, Windows 매트릭스에서 `cargo build --workspace --locked`, `cargo test --workspace --no-fail-fast --locked`를 실행합니다. `--locked` 는 `Cargo.lock` 이 `Cargo.toml` 들과 어긋난 푸시를 CI 에서 실패로 드러냅니다. 테스트 수는 고정 계약이 아니며(웨이브 8B 게이트 기준 4,815개 통과), 숫자 대신 CI 결과로 판단합니다.
- Linux 의존성: `libasound2-dev`, `libgtk-3-dev`, `libudev-dev`, `libxkbcommon-dev`, `libwayland-dev`, `libx11-dev`, `libxrandr-dev`, `libxi-dev`, `libxcursor-dev`를 설치합니다. 이는 cpal(ALSA), gilrs(udev), winit(X11/Wayland), rfd(GTK3) 링크 요구를 덮습니다.
- `lint` 잡: Ubuntu에서 Rust 1.95.0과 `rustfmt`, `clippy`를 설치한 뒤 `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`를 실행합니다. 둘 다 실패 시 CI를 실패시키는 게이트입니다.
- 캐시: 모든 잡은 `Swatinem/rust-cache@v2`를 사용합니다. CI 툴체인은 `rust-toolchain.toml`과 같은 1.95.0으로 고정됩니다.
- 제한 시간(`timeout-minutes`): `test` 30분, `lint` 15분. 2026-10-10 dev 의 통과 실행 4건(`38084893496`, `38078567755`, `38077937158`, `38091651597`)에서 `test` 잡은 2.8~8.3분(가장 느린 쪽은 Windows), `lint` 잡은 1.1~1.5분이었습니다. 제한은 가장 느린 실측의 약 3.6배이며, 캐시가 빈 첫 실행에서 모든 의존성을 다시 컴파일하는 시간과 멈춘 잡을 기본 6시간까지 두지 않는 것 사이의 값입니다. 빈 캐시 실행의 소요는 측정하지 못했으므로, 그런 실행이 30분에 가까워지면 올립니다.
- OpenH264(동영상): `openh264-sys2` 가 C 소스를 `cc` 로 컴파일하므로 세 OS 모두 추가 설치 단계가 없습니다. x86_64 에서는 `nasm` 이 있으면 어셈블리를 쓰고 없거나 실패하면 어셈블리 없이 C 로만 빌드합니다(빌드는 실패하지 않음).

마지막으로 확인한 통과 CI 실행은 `38084893496`(웨이브 8A 결과 커밋)이며 Ubuntu·macOS·Windows test와 fmt/clippy 잡이 모두 성공했습니다. 푸시 뒤에는 `gh run list --branch dev` 와 `gh run view` 로 세 OS 의 결과를 직접 확인합니다.

## 2. 릴리스 워크플로 (`.github/workflows/release.yml`)

릴리스 워크플로는 다음 세 경로를 정의합니다.

1. `prod` push: `version` 잡이 가장 최근 `v*.*.*` 태그에서 patch 버전을 올립니다. 태그가 없으면 `v0.0.0`에서 계산합니다. 루트 `Cargo.toml`의 첫 workspace 버전만 바꾸고, 추적되는 `Cargo.lock` 의 워크스페이스 크레이트 버전도 `cargo update --workspace` 로 함께 맞춘 뒤(외부 의존 버전은 바뀌지 않습니다. 이 단계가 없으면 아래 `--locked` 빌드가 실패합니다) `release: vX.Y.Z` 커밋과 주석 태그(`git tag -a`)를 만들어 같은 `prod` ref 와 그 태그를 이름으로 push합니다(`--follow-tags` 는 주석 태그만 보내므로 태그를 직접 지정합니다. 뒤의 빌드 잡이 이 태그를 체크아웃합니다).
2. 수동 dispatch: Actions의 `release`에서 patch, minor, major를 선택해 같은 버전·빌드 흐름을 실행합니다.
3. 태그 push: `v*.*.*` 태그 push는 version 잡을 건너뛰고 그 태그 ref로 빌드합니다.

빌드와 게시 단계는 다음과 같습니다.

- macOS: `aarch64-apple-darwin`과 `x86_64-apple-darwin` release binary를 `cargo build --release --locked` 로 빌드하고 `lipo`로 합친 뒤 ad-hoc `codesign` 합니다. `rbms-macos-universal.tar.gz`와 SHA-256 파일을 만듭니다.
- Windows: `cargo build --release --locked` 로 빌드한 `rbms-player.exe`를 `rbms-windows-x64.zip`으로 묶고 SHA-256 파일을 만듭니다.
- 초안 릴리스: 두 플랫폼 아카이브와 체크섬을 `softprops/action-gh-release@v2`로 **초안(draft)** GitHub Release 에 첨부합니다(`draft: true`, 사용자 결정 2026-10-12). 어느 경로로 돌든 자동은 태그와 초안까지이고, 게시(publish)는 유지보수자가 GitHub 의 Releases 화면에서 릴리스 노트를 확인한 뒤 직접 합니다.
- macOS 는 서명 시크릿이 등록돼 있으면 ad-hoc 대신 Developer ID 로 서명하고 공증합니다(3.1절).
- 동영상: 두 빌드 모두 `rbms-player` 의 기본 피처 `video` 를 켠 채(`--no-default-features` 없이) 빌드하므로 OpenH264 가 바이너리에 정적으로 들어갑니다(결정 D10). 소스 빌드라 Cisco 가 배포하는 바이너리를 내려받지 않습니다.

## 2.1 배포 묶음과 제3자 고지

두 아카이브의 내용은 같은 세 문서와 실행 파일입니다.

| 파일 | macOS (`rbms-macos-universal.tar.gz`) | Windows (`rbms-windows-x64.zip`) |
| --- | --- | --- |
| 실행 파일 | `rbms-player` | `rbms-player.exe` |
| `LICENSE` | O | O |
| `README.md` | O | O |
| `THIRD-PARTY-NOTICES.md` | O | O |

- 세 문서 중 하나라도 저장소 루트에 없으면 패키징 단계가 실패합니다(예전에는 없으면 조용히 건너뛰었습니다). 고지 없는 아카이브가 게시되지 않게 하려는 의도입니다.
- 기본 스킨(`assets/skins/rbms-default`)은 바이너리에 내장되므로 아카이브에 스킨 폴더를 따로 넣지 않습니다.
- `THIRD-PARTY-NOTICES.md` 는 영어이며 다음을 담습니다. OpenH264(Cisco, BSD-2-Clause)의 저작권 표시와 라이선스 전문 및 "소스에서 빌드해 정적으로 링크했다"는 사실, `openh264` 바인딩과 `re_mp4` 의 라이선스, Inter 글꼴의 SIL OFL 1.1 전문, 바이너리에 컴파일되어 들어가는 Lua 5.2.4 와 SQLite, 링크되는 모든 Rust 크레이트의 이름·버전·라이선스(직접 의존 표와 라이선스별 전체 목록), 그리고 그 크레이트들의 라이선스 전문(7절). 7절은 Apache-2.0 을 선택지로 주는 크레이트를 Apache-2.0 으로 받아 전문 한 벌로 덮고, 나머지 크레이트는 패키지에 실린 라이선스 파일을 저작권 표시와 함께 싣습니다(같은 본문은 한 번). 패키지에 라이선스 파일이 없는 크레이트는 7.3 절에 이름·저자·저장소로 적습니다.
- 갱신 시점: 의존성을 추가·삭제하거나 버전을 올렸을 때, 번들 글꼴·네이티브 소스·제3자 자산이 바뀌었을 때, 그리고 릴리스 전. 6절의 스크립트(`python3 -I gen-notices.py`, 내부에서 `cargo metadata --locked` 실행)로 5절의 표와 7절 전체를 다시 만듭니다. 스크립트는 로컬 레지스트리의 패키지에서 라이선스 파일을 읽으므로 먼저 `cargo fetch --locked` 가 되어 있어야 하고, 본문은 행 끝 공백만 지우고 그대로 싣습니다.
- 새 라이선스가 GPL-3.0-or-later 와 호환되는지 확인합니다. 현재 의존에서는 `OR` 선택지가 있는 크레이트(`self_cell` 의 Apache-2.0 OR GPL-2.0-only, `r-efi` 의 LGPL-2.1-or-later 선택지)를 허용 쪽 라이선스로 쓰며, `symphonia` 13개는 MPL-2.0 입니다(MPL-2.0 3.3 절로 결합 가능, 소스는 crates.io 에서 변경 없이 받을 수 있음).
- `Cargo.lock` 은 git 에 추적되고 CI·릴리스 빌드는 모두 `--locked` 입니다. 따라서 릴리스 바이너리의 의존 버전은 커밋된 `Cargo.lock` 과 같고, 고지 파일의 버전표도 그 `Cargo.lock` 에서 만듭니다. 의존성을 바꾸면 `Cargo.lock` 을 함께 커밋하고 `THIRD-PARTY-NOTICES.md` 의 표를 다시 만듭니다. 플랫폼 전용 의존(`cargo tree --target all`)도 `Cargo.lock` 에 모두 들어 있어 macOS 두 아키텍처와 Windows 가 같은 파일로 빌드됩니다.

macOS 유니버설 빌드의 `x86_64-apple-darwin` 쪽 OpenH264 는 Apple Silicon 러너에서 교차 컴파일되며, 이 경로는 아직 확인되지 않았습니다(로컬에 해당 타깃이 없어 시도하지 않았고, 실제 릴리스 실행에서 처음 확인됩니다). `nasm` 이 없는 러너에서는 어셈블리 없이 C 로 빌드되어 Intel Mac 의 디코드가 느릴 수 있습니다.

릴리스 빌드 잡은 현재 CI와 달리 `dtolnay/rust-toolchain@stable`을 사용합니다. Phase R에서 첫 실제 릴리스를 수행하기 전에 이 차이를 의도적으로 유지할지, CI와 같은 고정 버전으로 맞출지 확인합니다.

## 3. Phase R 전제와 보안 경계

- 워크플로의 `permissions: contents: write`와 기본 `GITHUB_TOKEN`은 버전 커밋·태그·GitHub Release 생성에 쓰입니다. 이 토큰만으로 macOS 공증이나 Windows Authenticode 서명은 되지 않습니다.
- macOS 산출물의 서명과 공증은 아래 시크릿이 저장소에 등록돼 있을 때만 수행합니다(2026-10-12 추가). 등록돼 있지 않으면 이전처럼 ad-hoc 서명만 하고 공증하지 않습니다. Windows 산출물은 Authenticode 서명 단계가 없습니다. 외부 키, 인증서, 공증 자격 증명의 값은 이 저장소나 문서에 넣지 않습니다.

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

- 앞의 둘만 있으면 Developer ID 서명(hardened runtime, 보안 타임스탬프)까지 하고 공증은 건너뜁니다. 계정 셋까지 다 있으면 `notarytool` 로 공증합니다.
- 인증서는 잡 안에서 만든 임시 키체인에만 들어가고 잡이 끝나면 키체인을 지웁니다. 키체인 비밀번호는 잡마다 새로 만들므로 시크릿이 아닙니다.
- 배포물이 단독 실행 파일이라 공증 티켓을 붙일(staple) 수 없습니다. Gatekeeper 는 처음 실행할 때 온라인으로 티켓을 확인합니다.
- 이 단계는 실제 자격 증명으로 실행해 본 적이 없습니다. 첫 릴리스 실행에서 확인합니다.
- `prod` 보호 규칙이 자동 버전 커밋/태그 push를 막을 수 있습니다. Phase R에서 실제 보호 정책과 릴리스 주체를 확인한 뒤, 필요한 최소 예외 또는 수동 태그 경로를 선택합니다.
- 첫 릴리스 버전은 루트 `Cargo.toml`의 현재 `0.1.0`과 일치하는 태그 전략을 정한 뒤 진행합니다. 워크플로를 시험하기 위해 `prod`, 태그 또는 외부 배포를 임의로 만들지 않습니다.

## 4. 사용자 실행 전제

- macOS 아카이브는 압축 해제 후 `./rbms-player <곡폴더>`로 실행합니다. 공증되지 않은 배포물의 Gatekeeper 처리 방식은 실제 서명 상태와 배포 채널을 확인한 뒤 안내합니다.
- Windows 아카이브는 압축 해제 후 `rbms-player.exe <곡폴더>`로 실행합니다. SmartScreen 동작은 실제 서명·평판 상태에 따라 달라질 수 있습니다.
- 인자 없이 실행하면 플레이어는 기억한 `songs_folder`, `RBMS_SONGS`, 빈 GUI 순으로 진입합니다. 자세한 실행과 GUI 폴더 등록은 루트 [README.md](../README.md)를 정본으로 사용합니다.

## 5. Phase R 체크리스트

실제 키가 준비된 유지보수자 세션에서만 다음을 수행합니다.

1. `prod` 생성과 보호 정책을 확인하고, 자동 버전 커밋이 허용되는지 결정합니다.
2. 실제 macOS/Windows 서명 또는 공증 자격 증명을 안전한 CI secret으로 등록합니다.
3. 첫 버전·태그 전략을 확정하고 release workflow를 실행합니다.
4. 게시된 아카이브의 체크섬과 실제 설치·실행을 확인합니다.

이 체크리스트 전에는 `dev` CI만 배포 전 검증 근거입니다.
