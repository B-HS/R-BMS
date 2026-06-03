# CI / 릴리스 (GitHub Actions)

> `.github/workflows/ci.yml`(검증) + `release.yml`(자동 버전 + macOS·Windows 빌드 + GitHub Release). **GitHub 원격에 push된 뒤** 동작한다(원격 `github.com/B-HS/R-BMS` 설정됨·`dev` 푸시됨; 잔여는 `prod` 브랜치·첫 태그 — §5).
> **브랜치 모델**: **`dev` = 작업(개발) 브랜치, `prod` = 배포 브랜치.** dev에서 작업·PR → CI 통과 → prod 머지 시 자동 릴리스.

## 1. ci.yml — 검증 (push/PR)
- 트리거: `dev`·`prod` push, 그 둘로의 PR.
- `test` 잡: **ubuntu·macos·windows 매트릭스**로 `cargo build --workspace` + `cargo test --workspace`(현재 880). Linux는 cpal(ALSA)·winit(X11/Wayland)·rfd(GTK3) 링크용 dev 라이브러리 설치.
- `lint` 잡: `cargo fmt --check` + `cargo clippy`(둘 다 **informational**, `continue-on-error` — 프로젝트가 fmt/clippy 게이트가 아니므로 CI 실패 안 시킴).
- 캐시: `Swatinem/rust-cache@v2`.

## 2. release.yml — 자동 버전 + 빌드 + 릴리스
세 가지 트리거:

### (1) prod 머지 (배포) — `prod`에 push
- `version` 잡: 최신 `v*.*.*` 태그에서 **patch 자동 bump** → `Cargo.toml [workspace.package] version` 교체(perl, 모든 크레이트가 `version.workspace`라 전체 동기화) → 커밋 `release: vX.Y.Z` → 태그 → push → 빌드 → 릴리스.

### (2) 수동 bump — Actions → "release" → Run workflow → bump(patch/minor/major)
- `version` 잡이 선택한 단계로 bump(동일 흐름).

### (3) 수동 태그 — `git tag v1.2.3 && git push --tags`
- `version` 잡 skip(`github.ref_type == 'tag'`), `build-*`가 그 태그로 빌드.

### 빌드 산출물 (+ SHA-256 체크섬)
- **macOS**: `aarch64`+`x86_64` 빌드 → `lipo` **유니버설** → **ad-hoc 서명**(`codesign -s -`, arm64 실행 필수) → `rbms-macos-universal.tar.gz` + `.sha256`.
- **Windows**: `x86_64-pc-windows-msvc` → `rbms-player.exe` → `rbms-windows-x64.zip` + `.sha256`.
- `release` 잡: `softprops/action-gh-release@v2`로 아카이브 + 체크섬을 **GitHub Release** 첨부(릴리스 노트 자동).
- 체크섬은 백엔드 `client_build` allowlist(변조 탐지, `docs/backend`)의 입력이 된다 — 릴리스 빌드 해시 ↔ 클라가 런타임에 보내는 `client_build_sha256` 대조.

> 폰트(`assets/fonts/Inter-Regular.ttf`)·스킨은 `include_bytes!/str!`로 바이너리에 임베드 → 산출물은 **단일 실행파일**(런타임 에셋 불필요). 설정은 첫 실행 시 `~/.config/rbms`(win: `%USERPROFILE%\.config\rbms`)에 생성.

## 3. 실행 (사용자) — 코드 서명/공증 현실
- **macOS**: `tar xzf rbms-macos-universal.tar.gz && ./rbms-player <곡폴더>`.
  - CI에서 **ad-hoc 서명**돼 Apple Silicon(arm64)에서 *실행 자체는 가능*(ad-hoc 없으면 `killed: 9`).
  - 단 **공증(notarize) 안 됨** → 다운로드 파일엔 `com.apple.quarantine` 부착 → Gatekeeper "확인되지 않은 개발자" **경고로 막힘(하드 차단 아님)**. 우회: 최초 1회 **우클릭→열기**, 또는 `xattr -dr com.apple.quarantine rbms-player`, 또는 설정→개인정보보호→"그래도 열기".
- **Windows**: zip 풀고 `rbms-player.exe <곡폴더>`. 미서명 → SmartScreen "추가 정보→실행"(하드 차단 아님).
- **무경고 배포**(P3, 선택): macOS = Apple Developer 인증서 서명 + `notarytool` 공증($99/년), Windows = Authenticode(EV 권장) 서명. 시크릿 추가 시 워크플로에 단계 삽입.

## 4. 권한 / 시크릿
- `release.yml`은 `permissions: contents: write`(태그 push·릴리스 생성). `GITHUB_TOKEN` 자동 제공 — 추가 시크릿 불필요.
- 자동 버전 커밋/태그 push는 기본 `GITHUB_TOKEN`으로 수행되어 **새 워크플로를 트리거하지 않음**(무한루프 방지).

## 5. 사전 준비 / 주의 (현재 미충족 항목)
- [x] **GitHub 원격**: `origin = github.com/B-HS/R-BMS` 설정됨. `dev`(작업) 푸시됨(`origin/dev`), `refactor/entire-base`도 원격 존재. 태그 없음.
- [ ] **`prod` 배포 브랜치 생성**: 아직 없음. `git branch prod && git push -u origin prod` → `prod` push가 release.yml(자동 버전·빌드·릴리스)을 트리거. 단 첫 버전은 v0.0.1 자동 bump 대신 **수동 태그 `v0.1.0`**(Cargo.toml 일치) 권장(§2-3).
- [ ] **브랜치 보호**: `prod` 보호 시 `version` 잡의 자동 커밋/태그 push가 막힐 수 있음 → bot 예외 허용하거나 자동버전 대신 수동 태그(§2-3) 사용.
- [ ] **LICENSE 파일**: `Cargo.toml`·README는 `GPL-3.0-or-later`(beatoraja 포팅)이나 루트 `LICENSE` 파일 없음 → **GPL-3.0 전문 추가 권장**(릴리스 동봉). 폰트 Inter는 OFL(`assets/fonts/Inter-OFL.txt`).
- [ ] **코드 서명/공증**(P3): §3 참조. 현재 macOS ad-hoc(실행 OK)·미공증, Windows 미서명.
- [ ] **.app/.dmg / 인스톨러**(P3): 현재 바이너리 아카이브. 더블클릭 `.app`은 무인자 실행 시 폴더 picker 진입하도록 클라 보강 필요(현재 무인자=usage 종료).

## 5.1 라이선스 (클라이언트 vs 백엔드/FE)
- **클라이언트(rbms) = GPL-3.0-or-later 고정**: beatoraja(GPL-3.0)의 코어 PLAY를 포팅한 **파생 저작물** → copyleft로 MIT 불가. (판정 윈도우 등 원본 byte 대조 — 클린룸 아님.)
- **백엔드 서버 / 향후 FE = MIT 등 자유**: HTTP로만 통신하는 **별 프로그램**(파생 아님, mere aggregation) → 별 저장소·MIT 가능. (법적 조언 아님 — 핵심: 클라 GPL, 서버/FE 분리작품.)
- 폰트 Inter(OFL)·cosmic-text 등 deps는 GPL/MIT 양쪽 호환.

## 6. 로컬 검증(완료)
- 두 워크플로 YAML 파싱 OK(ruby). 버전 bump perl이 워크스페이스 version만 교체(rust-version·외부 dep 불변) 확인. `cargo build --release -p rbms-player` 정상.
