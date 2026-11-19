## R-BMS v0.1.13

v0.1.12 이후의 변경입니다.

### 고친 것
- **클리어 램프가 나뉘어 보이지 않던 문제.** v0.1.12 는 SCRATCH AUTO 로 깬 게이지가 어시스트 없이 깬 램프보다 높을 때만 목록 램프를 나눴습니다. 이제 어시스트 없이 깬 램프(EASY 이상)와 SCRATCH AUTO 로 깬 기록이 함께 있으면 어느 쪽이 높든 위아래로 나뉘어 둘 다 보입니다. 위 절반이 어시스트 없는 램프, 아래 절반이 어시스트 등급 색이고, 정보창 MY SCORE 에 두 이름이 나란히 나옵니다. 예: HARD CLEAR 와 ASSIST HARD, HARD CLEAR 와 ASSIST EASY.
- 이미 쳐 둔 기록에도 적용됩니다. 다시 칠 필요가 없습니다.
- 배포 파일에 든 README 의 기능·조작 설명을 v0.1.12 의 변경(출력 지연 보정, FULLSCREEN, 조합키와 컨트롤러 ESC, 배치 저장)에 맞췄습니다.

### 알려진 것
- SCRATCH AUTO 로만 깬 곡은 전처럼 등급 색 하나, 어시스트 없이만 깬 곡은 그 램프 하나로 보입니다.
- 웹의 순위·기록 화면은 바뀌지 않았습니다(램프 나누기는 플레이어의 선곡 화면에만 있습니다).
- Windows 는 아직 코드 서명이 없어 처음 실행할 때 SmartScreen 이 한 번 묻습니다.

### 설치
- macOS(Apple silicon·Intel): `R-BMS-macos-universal.dmg` 를 열어 R-BMS 를 응용 프로그램으로 끌어 놓습니다. 서명·공증판입니다.
- Windows: `rbms-windows-x64.zip` 을 풀고 `rbms-player.exe` 를 실행합니다.
- 각 파일의 SHA-256 은 함께 올린 `.sha256` 파일에 있습니다.

**Full Changelog**: https://github.com/B-HS/R-BMS/compare/v0.1.12...v0.1.13
