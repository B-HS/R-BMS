# 앱 아이콘

R-BMS 의 앱 아이콘. 기본 스킨과 같은 무채색으로, 검은 둥근 사각형 안에 스크래치 레인 하나와 건반 레인 일곱 줄, 판정선, 노트 몇 개를 그린다.

| 파일 | 쓰는 곳 |
| --- | --- |
| `R-BMS.icns` | macOS `R-BMS.app` 의 `Contents/Resources`(`scripts/macos-app.sh`, `Info.plist` 의 `CFBundleIconFile`) |
| `rbms.ico`, `rbms.rc` | Windows `rbms-player.exe` 의 리소스(`scripts/windows-icon-resource.ps1` 이 `rc.exe` 로 컴파일, `apps/rbms-player/build.rs` 가 링크) |
| `png/icon-<크기>.png` | 16~1024 px 원본 크기별 그림(다른 곳에 아이콘이 필요할 때) |

모든 파일은 `tools/generate-icon.py` 가 만든다(손으로 고치지 않는다). 모양을 바꾸면 스크립트의 상수를 고치고 다시 만든다.

```bash
uv run assets/icon/tools/generate-icon.py          # 다시 만들기
uv run assets/icon/tools/generate-icon.py --check  # 커밋된 파일이 스크립트와 같은지
```
