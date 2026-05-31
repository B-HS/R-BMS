# rbms

beatoraja(Java/libGDX BMS 구동기)의 **코어 PLAY 모듈**을 Rust로 새로 포팅한 BMS 플레이어. macOS/Windows 크로스플랫폼(wgpu/Metal, cpal/CoreAudio).

> **작업 연속성 / 현재 상태 / 다음 할 일은 [`docs/PROCESS.md`](docs/PROCESS.md)가 단일 출처(SSOT).** 새 세션은 거기부터.

## 현재 상태

완전 플레이 가능 + beatoraja/IIDX 확장 다수 완성. 워크스페이스 **100 테스트·0 실패**. 실제 발광 ★1 라이브러리 727곡 파싱 MD5 byte-exact, autoplay 풀콤보 검증.

기능: BMS 파싱 · 마디→µs 타이밍 · 샘플정확 오디오 믹서 · 스크롤/하이스피드(+CONSTANT 그린넘버) · 판정(beatoraja 윈도우 일치·콤보·EX·게이지·클리어램프(beatoraja 색)·LN·**空POOR 정확처리**) · autoplay/입력 · 모드 자동감지(7K/5K/14K/PMS/10K) · 1280×720 GPU 인스턴스 렌더 · 데이터주도 스킨(일반/와이드 RON) · 키 빔 · 레인 외곽선/구분선 · 가로 게이지(IIDX식) · BGA(이미지, on/off) · **다국어 폰트**(cosmic-text+Inter+시스템폴백 → 일/한/중/태/아랍 등 모든 언어, 교체 가능) · 플레이 HUD · 노트옵션(랜덤/미러/S-랜덤 등) · 판정 오프셋 + 오토 캘리브레이션 · JUDGE WIDTH/TOTAL 조정 · 통합 키 설정(파일+인앱 에디터) · 설정 영속(탭 UI) · 리플레이 저장/재생 · GUI 곡선택(폴더·난이도표 네비, **마우스 클릭**) · **로컬 플레이 기록**(scores.ron·곡선택 인라인 리스트·상세 모달) · 난이도표 인앱 관리(다중) · DEBUG 오버레이(FPS/RAM) · IR 슈퍼셋 스코어 인터페이스(**백엔드 전체설계=`docs/backend/`**).

## 빌드 & 실행

```bash
./start.sh                                    # release 빌드 후 기본 라이브러리+발광1 표로 열기
./start.sh "<폴더|차트>" [옵션...]             # 인자 그대로 전달 (env RBMS_SONGS / RBMS_TABLE)
# 또는 직접:
cargo build --release -p rbms-player
BIN=./target/release/rbms-player
$BIN "<곡 폴더>"                  # GUI 곡선택 (↑↓, Enter 열기, O 폴더, T 난이도표, Tab 설정, Esc)
$BIN "<차트.bme>" [--interactive] # 단일 차트 (기본 autoplay)
$BIN --replay <file.ron>          # 리플레이 재생
```

기본 레인 키(beatoraja Z열): 7K=`Z S X D C F V`+LShift(스크), 5K/9K/14K 등 모드별 프리셋(차트 자동감지). 조작: ↑↓ 속도·→← 커버·][ lift. **모든 키는 설정→KEY CONFIG에서 재바인딩**, 설정·키·기록(scores.ron)·리플레이는 `~/.config/rbms/`에 저장.
곡선택: ↑↓·←뒤로·→/Enter 열기·**마우스 클릭**·`R` 기록 모달. 설정: Tab 탭전환·←→ 값·클릭. DISPLAY 탭의 **FONT**로 UI 폰트 교체.
옵션: `--interactive|--auto --sc-left --sc-auto --lift F --hispeed F --gauge X --keys ... --skin file.ron --font file.ttf --table URL --keyconfig path.ron --replay file.ron --server URL --player ID`.

## 아키텍처 (의존 위→아래)

```
apps/rbms-player → rbms-play → {rbms-render, rbms-audio, rbms-ir, rbms-judge, rbms-chart, rbms-table} → rbms-parser → rbms-model
```

| 크레이트 | 역할 |
|---|---|
| rbms-model | 순수 타입(Mode·Note·TimeLine·Model) |
| rbms-parser | BMS lexical(base36/62·Shift-JIS·#RANDOM·MD5/SHA-256) |
| rbms-chart | detect_mode·타이밍 적분·채널→레인·scroll·shuffle(노트옵션) |
| rbms-judge | 판정 윈도우·매칭·콤보·EX·게이지·클리어램프·LN |
| rbms-audio | symphonia 디코드 + cpal RT 믹서(샘플 클럭) |
| rbms-render | Renderer trait + CPU백엔드 + skin·playfield·hud·result·**다국어 font(cosmic-text)** |
| rbms-ir | IR 슈퍼셋 DTO·ScoreServer trait·HTTP/Null 클라(클라 계약=docs/reference/ir-api.md, 서버 설계=docs/backend) |
| rbms-table | 난이도표(BMS table) fetch·md5 매칭·level 그룹핑 |
| rbms-play | 통합 드라이버(스케줄러·autoplay·입력·판정·키빔 와이어링) |

앱 모듈(`apps/rbms-player/src/`): `keyconfig`(키 설정)·`settings`(플레이옵션 영속)·`replay`(리플레이)·`tables`(난이도표 목록)·`scores`(로컬 기록).

## 릴리스 / 배포 (GitHub Actions)

`.github/workflows/`: `ci.yml`(dev/prod·PR → 전 플랫폼 빌드·테스트) + `release.yml`(자동 버전 + **macOS 유니버설·Windows x64** 빌드 → GitHub Release). 폰트·스킨이 바이너리에 임베드되어 산출물은 단일 실행파일. 상세·코드서명/공증 주의는 [`docs/ci-release.md`](docs/ci-release.md).

## 문서

- `docs/PROCESS.md` — **현재 상태·실행·다음 할 일(SSOT)**, `ROADMAP.md` — 앞으로 할 일
- `docs/backend/` — **백엔드 IR-슈퍼셋 서버 설계**(PRD·전 엔드포인트·Drizzle 스키마·LR2IR/beatoraja 매핑)
- `docs/ci-release.md` — GitHub Actions CI/CD·서명·라이선스, `docs/font-cjk-support.md` — 다국어 폰트
- `docs/reference/` — beatoraja 메커닉·Rust 스택·아키텍처·wgpu/winit API·IR 계약
- `docs/history/`(시간순) · `docs/acknowledge/`(확정 결정·보류 차이) · `docs/memory/`(테스트 라이브러리 실측)

## 라이선스

**GPL-3.0-or-later** (beatoraja 포팅 파생물 — copyleft 고정). 별 저장소가 될 백엔드 서버·웹 FE는 HTTP로만 통신하는 분리 작품이라 MIT 등 가능. 번들 폰트 Inter는 SIL OFL 1.1(`assets/fonts/Inter-OFL.txt`).
