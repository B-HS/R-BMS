# ADR: 앱 아이콘은 스크립트로 만든 무채색 레인 그림, Windows 는 rc.exe 로 새 의존성 없이

> 최종 갱신 2026-10-15 · 대응 커밋 ea698a9
> 상태: 유효 · 결정일 2026-10-15 · 결정자 사용자(아이콘을 만들라는 지시, Windows 방식 Q2 의 A) + 메인(그림) · 커밋 `cc500ef`(앱), `089c737`(웹 파비콘)

## 배경

R-BMS 실행 파일·앱 번들·창에 아이콘이 없었다(macOS 는 기본 앱 그림, Windows 는 기본 실행 파일 그림). 웹 사이트의 파비콘도 앱과 달랐다(`web/src/app/icon.svg`). 사용자가 아이콘을 만들라고 했다.

## 결정

1. **그림.** 기본 스킨과 같은 무채색으로, 검은 둥근 사각형 안에 스크래치 레인 하나와 건반 레인 일곱 줄, 판정선, 노트 몇 개를 그린다. 모든 파일은 `assets/icon/tools/generate-icon.py` 가 만든다(손으로 고치지 않는다, `--check` 로 커밋된 파일과 대조). 파일 목록과 쓰는 곳의 정본은 `assets/icon/README.md` 다.
2. **macOS.** `scripts/macos-app.sh` 가 `assets/icon/R-BMS.icns` 를 `R-BMS.app/Contents/Resources` 에 넣고 `Info.plist` 의 `CFBundleIconFile` 을 `R-BMS` 로 둔다.
3. **Windows(Q2 의 A).** 릴리스 워크플로가 Windows SDK 의 `rc.exe` 로 `assets/icon/rbms.rc`(`rbms.ico`)를 컴파일하고(`scripts/windows-icon-resource.ps1`), 그 `.res` 경로를 `RBMS_WINDOWS_RESOURCE` 로 넘기면 `apps/rbms-player/build.rs` 의 `link_windows_resource` 가 링크 인자로 붙인다. 변수가 없는 빌드(로컬, 다른 OS)는 아무것도 링크하지 않아 SDK 도구에 기대지 않는다. 변수가 가리키는 파일이 없거나 대상이 Windows MSVC 가 아니면 빌드를 멈춘다. CI 의 Windows 잡도 같은 단계를 돈다(`.github/workflows/ci.yml`, `release.yml`).
4. **창.** Windows·Linux 의 창 제목 표시줄·작업 표시줄 아이콘은 실행 파일에 넣은 `assets/icon/png/icon-256.png` 를 winit `Icon` 으로 준다(`apps/rbms-player/src/window_mode.rs` 의 `WINDOW_ICON_PNG`, `window_icon`). 읽지 못하면 플랫폼 기본 아이콘으로 창은 그대로 연다. macOS 는 번들의 아이콘을 쓴다.
5. **웹.** 같은 스크립트가 `web/src/app/favicon.ico`(16·32·48), `icon.png`(512), `apple-icon.png`(180)를 만들고 옛 `icon.svg` 는 지운다(Next.js 의 app 아이콘 파일 규칙).

## 이유

- 새 의존성을 들이지 않는다(프로젝트 규칙). Windows 리소스는 Windows SDK 에 이미 있는 `rc.exe` 와 Cargo 의 링크 인자만으로 된다.
- 그림을 스크립트로 만들면 크기별 파일과 웹 파일이 한 원본에서 나오고 다시 만들 수 있다.
- 무채색은 앱 UI 무채색 결정(`2026-10-09-lua-skin-compat-decisions.md`)과 맞춘다.

## 기각된 대안

| 대안 | 기각 이유 |
| --- | --- |
| Windows 리소스를 `winresource` 크레이트(build 의존성)로 넣는다(Q2 의 B) | 새 의존성. 사용자가 A 를 택함 |
| 그림 파일을 손으로 그려 커밋한다 | 크기별·웹 파일이 서로 어긋날 수 있고 다시 만들 수 없다 |

## 영향과 관련 파일

- `assets/icon/`(`R-BMS.icns`, `rbms.ico`, `rbms.rc`, `png/icon-*.png`, `README.md`, `tools/generate-icon.py`), `apps/rbms-player/build.rs`, `apps/rbms-player/src/window_mode.rs`, `scripts/macos-app.sh`, `scripts/windows-icon-resource.ps1`, `.github/workflows/ci.yml`, `.github/workflows/release.yml`, `web/src/app/{favicon.ico,icon.png,apple-icon.png}`.
- v0.1.4 산출물에서 확인: macOS 앱의 `CFBundleIconFile` `R-BMS`, Windows `rbms-player.exe` 의 `RT_ICON` 7개·`RT_GROUP_ICON` 1개([v0.1.4 릴리스 ADR](2026-10-14-adr-v0-1-4-release.md)).

## 날짜와 사용자 원문

2026-10-15. 아이콘은 "적당히 만들어서"(세션 인벤토리의 요지. 원문 전체는 저장소에 남아 있지 않다). 이 작업은 Workflow 없이(B) 메인이 했고, Windows 아이콘 방식은 Q2 에 A(`rc.exe` + `build.rs`, 새 의존성 없음).
