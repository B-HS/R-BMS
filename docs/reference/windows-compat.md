# Windows / 크로스플랫폼 호환 — 현황 & 잔여

> rbms는 크로스플랫폼 크레이트(winit/wgpu·cpal·rfd·cosmic-text)로 작성돼 **플랫폼별 분기 코드가 없다**(`cfg(windows)` 0건). 따라서 호환성은 "코드"보다 "**실제 런타임 검증**"이 관건이다. 이 문서가 그 단일 출처.

## CI가 보장하는 것
- `.github/workflows/ci.yml`이 **ubuntu·macos·windows 매트릭스**로 `cargo build --workspace` + `cargo test --workspace`(887) 수행 → **Windows에서 컴파일·단위테스트(로직)는 green**.
- `release.yml`이 `x86_64-pc-windows-msvc`로 `rbms-player.exe` 빌드 → zip + sha256.
- **한계:** CI 러너는 디스플레이·오디오 장치가 없어 **GUI/오디오/입력 런타임은 실행하지 않는다**. 즉 "빌드되고 로직 테스트는 통과"까지만 보장.

## 수정됨 (2026-06-03)
- **설정 경로 USERPROFILE 폴백** — 기존 `var_os("HOME")`만 사용해 Windows(HOME 미설정)에서 모든 config/scores/replays가 **cwd `./.config/rbms/`**로 떨어지던 버그. `config_dir()`가 `HOME → USERPROFILE → "."` 순으로 해석(순수 `config_dir_from` 단위테스트). 이제 Windows에서 `%USERPROFILE%\.config\rbms`에 저장(ci-release.md 주장과 일치).

## 실제 Windows 머신 검증 필요 (런타임 — macOS에서 불가)
- [ ] **GUI 렌더(wgpu→DX12/Vulkan)** — 1280×720 창·노트/HUD/셀렉트 정상 표시.
- [ ] **오디오(cpal→WASAPI)** — 키음 샘플정확 재생·마스터 클럭(재생 샘플 수) 동작·지연.
- [ ] **입력(winit `KeyCode`)** — 레인/조작 키, 키 설정 캡처(물리 키코드라 레이아웃 무관 기대).
- [ ] **파일 다이얼로그(rfd)** — 폴더 선택(O)·폰트/난이도표 파일 선택이 Windows 네이티브 다이얼로그로 동작.
- [ ] **CJK 폰트 폴백(cosmic-text fontdb)** — 시스템 폰트에서 일/한/중 글리프 발견(Windows: Meiryo/Malgun/MS Gothic 등). 없으면 두부(□). 번들 Inter는 라틴만.
- [ ] **HiDPI/창 스케일** — Windows 배율(125/150%)에서 좌표/클릭 정합(논리 1280×720 고정).
- [ ] **`#PREVIEW`/키음 경로 해석** — `resolve_file`가 Windows 경로(역슬래시·대소문자)·상대경로 정상 처리(차트는 보통 `/` 사용).
- [ ] **무인자 실행** — 더블클릭(인자 없음) 시 현재 usage 종료 → 폴더 picker 진입 보강 필요(배포 P3 항목과 동일).

## 알려진 크로스플랫폼 한계
- **창 리사이즈 UI 리플로우 없음** — 논리 1280×720 고정(Windows/macOS 공통). → Phase 7 cosmetic.
- **미서명 exe → SmartScreen 경고**(하드 차단 아님). 서명은 배포 P3(Authenticode). → `docs/ci-release.md`.
- **BGA 비디오(mpg) 미지원**(전 플랫폼 공통).

## 결론
컴파일·로직은 3-OS green, **Windows 설정 경로 버그는 수정**. 남은 것은 대부분 **실제 Windows 환경에서의 GUI/오디오/폰트 런타임 1회 검증**(코드 변경이 아니라 확인)이며, 발견 시 크로스플랫폼 크레이트 설정·폰트 폴백 조정으로 대응한다.
