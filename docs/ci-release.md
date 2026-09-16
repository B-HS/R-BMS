# CI / 릴리스 (GitHub Actions)

> 이 문서는 워크플로 구현과 현재 배포 경계를 설명합니다. 워크플로가 존재한다는 사실은 외부 릴리스 산출물·서명·배포 엔드포인트가 현재 사용 가능하다는 보증이 아닙니다.
>
> 2026-09-16 현재 작업 브랜치는 `dev`이고 원격 추적 브랜치도 `origin/dev`만 있습니다. `prod` 생성, 첫 릴리스/태그, 실제 서명 또는 배포 자격 증명 적용은 유지보수자가 실제 키를 준비한 뒤 함께 수행할 Phase R이며, 이 라운드에서는 시작하지 않습니다.

## 1. CI (`.github/workflows/ci.yml`)

- 트리거: `dev`·`prod`의 push와 두 브랜치를 대상으로 한 pull request.
- `test` 잡: Ubuntu, macOS, Windows 매트릭스에서 `cargo build --workspace`, `cargo test --workspace`를 실행합니다. 테스트 수는 고정 계약이 아니지만, 마지막 로컬 Phase H 실행은 3,040개를 등록하고 exit 0이었습니다.
- Linux 의존성: `libasound2-dev`, `libgtk-3-dev`, `libudev-dev`, `libxkbcommon-dev`, `libwayland-dev`, `libx11-dev`, `libxrandr-dev`, `libxi-dev`, `libxcursor-dev`를 설치합니다. 이는 cpal(ALSA), gilrs(udev), winit(X11/Wayland), rfd(GTK3) 링크 요구를 덮습니다.
- `lint` 잡: Ubuntu에서 Rust 1.95.0과 `rustfmt`, `clippy`를 설치한 뒤 `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`를 실행합니다. 둘 다 실패 시 CI를 실패시키는 게이트입니다.
- 캐시: 모든 잡은 `Swatinem/rust-cache@v2`를 사용합니다. CI 툴체인은 `rust-toolchain.toml`과 같은 1.95.0으로 고정됩니다.

마지막 확인된 CI 실행은 `35055642686`이며 Ubuntu·macOS·Windows test와 fmt/clippy 잡이 모두 성공했습니다. 로컬에서 마지막으로 확인한 관련 명령은 `cargo test --workspace` exit 0(실제 오디오 장치 테스트 2건 ignored), `cargo build --release -p rbms-player` exit 0, fmt와 clippy 게이트 exit 0입니다.

## 2. 릴리스 워크플로 (`.github/workflows/release.yml`)

릴리스 워크플로는 다음 세 경로를 정의합니다.

1. `prod` push: `version` 잡이 가장 최근 `v*.*.*` 태그에서 patch 버전을 올립니다. 태그가 없으면 `v0.0.0`에서 계산합니다. 루트 `Cargo.toml`의 첫 workspace 버전만 바꾸고 `release: vX.Y.Z` 커밋과 태그를 같은 `prod` ref로 push합니다.
2. 수동 dispatch: Actions의 `release`에서 patch, minor, major를 선택해 같은 버전·빌드 흐름을 실행합니다.
3. 태그 push: `v*.*.*` 태그 push는 version 잡을 건너뛰고 그 태그 ref로 빌드합니다.

빌드와 게시 단계는 다음과 같습니다.

- macOS: `aarch64-apple-darwin`과 `x86_64-apple-darwin` release binary를 빌드하고 `lipo`로 합친 뒤 ad-hoc `codesign` 합니다. `rbms-macos-universal.tar.gz`와 SHA-256 파일을 만듭니다.
- Windows: `rbms-player.exe`를 `rbms-windows-x64.zip`으로 묶고 SHA-256 파일을 만듭니다.
- publish: 두 플랫폼 아카이브와 체크섬을 `softprops/action-gh-release@v2`로 GitHub Release에 첨부합니다. `LICENSE`와 `README.md`는 산출물에 존재할 때 함께 복사하며, 현재 루트 `LICENSE`는 존재합니다.

릴리스 빌드 잡은 현재 CI와 달리 `dtolnay/rust-toolchain@stable`을 사용합니다. Phase R에서 첫 실제 릴리스를 수행하기 전에 이 차이를 의도적으로 유지할지, CI와 같은 고정 버전으로 맞출지 확인합니다.

## 3. Phase R 전제와 보안 경계

- 워크플로의 `permissions: contents: write`와 기본 `GITHUB_TOKEN`은 버전 커밋·태그·GitHub Release 생성에 쓰입니다. 이 토큰만으로 macOS 공증이나 Windows Authenticode 서명은 되지 않습니다.
- 현재 macOS 산출물은 ad-hoc 서명만 하며 공증하지 않습니다. Windows 산출물은 Authenticode 서명 단계가 없습니다. 외부 키, 인증서, 공증 자격 증명은 이 저장소나 문서에 넣지 않습니다.
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
