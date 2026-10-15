# 앱 아이콘

최종 갱신 2026-10-15 · 대응 커밋 ea698a9

R-BMS 의 앱 아이콘. 기본 스킨과 같은 무채색으로, 검은 둥근 사각형 안에 스크래치 레인 하나와 건반 레인 일곱 줄, 판정선, 노트 몇 개를 그린다.

| 파일 | 쓰는 곳 |
| --- | --- |
| `R-BMS.icns` | macOS `R-BMS.app` 의 `Contents/Resources`(`scripts/macos-app.sh` 가 복사하고 없으면 멈춘다, `Info.plist` 의 `CFBundleIconFile` = `R-BMS`) |
| `rbms.ico`(16·24·32·48·64·128·256 일곱 크기), `rbms.rc`(`1 ICON "rbms.ico"`) | Windows `rbms-player.exe` 의 리소스(`scripts/windows-icon-resource.ps1` 이 `rc.exe` 로 컴파일, `apps/rbms-player/build.rs` 가 링크. 순서와 이유는 [ci-release.md](../../docs/ci-release.md) 2절 Windows) |
| `png/icon-256.png` | 플레이어 창의 아이콘. 바이너리에 내장되어(`apps/rbms-player/src/window_mode.rs` 의 `WINDOW_ICON_PNG`) Windows·Linux 의 제목 줄과 작업 표시줄에 쓰인다. macOS 는 번들의 `R-BMS.icns` 를 쓰고 창 아이콘은 쓰지 않는다 |
| `web/src/app/favicon.ico`(16·32·48), `icon.png`(512), `apple-icon.png`(180, 둥근 테두리 없이 꽉 찬 배경의 RGB) | 웹 사이트의 파비콘과 홈 화면 아이콘(Next.js 의 app 아이콘 파일 규칙) |
| `png/icon-<크기>.png` | 16·32·48·64·128·256·512·1024 px 원본 크기별 그림(위의 `icon-256.png` 말고는 다른 곳에 아이콘이 필요할 때) |

모든 파일은 `tools/generate-icon.py` 가 만든다(손으로 고치지 않는다). 모양을 바꾸면 스크립트의 상수를 고치고 다시 만든다. 스크립트는 PEP 723 인라인 메타로 `pillow==12.3.0` 에 기대므로 `uv run` 으로만 돌린다. 웹의 세 파일도 같은 스크립트가 쓰고 `--check` 가 함께 비교한다.

```bash
uv run assets/icon/tools/generate-icon.py          # 다시 만들기
uv run assets/icon/tools/generate-icon.py --check  # 커밋된 파일이 스크립트와 같은지(쓰지 않음, 다르면 종료 코드 1)
```

이 아이콘과 Windows 에 `winresource` 같은 크레이트 대신 SDK 의 `rc.exe` 를 쓴 이유(새 의존성을 들이지 않으려는 사용자 결정 2026-10-15)는 [앱 아이콘 ADR](../../docs/acknowledge/2026-10-15-adr-app-icon.md)에, 릴리스·CI 의 단계는 위 ci-release.md 에 있고 여기에 다시 적지 않는다.
