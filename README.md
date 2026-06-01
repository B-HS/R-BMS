# rbms 🚧 (WIP)

beatoraja(Java/libGDX)의 **코어 PLAY 모듈**을 Rust로 새로 포팅한 BMS 플레이어. macOS/Windows 크로스플랫폼(wgpu/Metal, cpal/CoreAudio).

> **Work in Progress** — 완전 플레이는 가능하나 계속 개발 중.

## 구현된 기능

- **파서** — BMS lexical(base36/62 · Shift-JIS · `#RANDOM`/`#IF` · `#mmmCC`), 차트 해시 MD5/SHA-256(byte-exact).
- **타이밍/차트** — 마디→µs 적분 · 채널→레인 · LN/지뢰 · 변속(BPM/STOP/SCROLL) · 모드 자동감지(7K/5K/9K/10K/14K).
- **오디오** — symphonia 디코드 + cpal RT 믹서, **마스터 클럭 = 재생 샘플 수**(vsync 비의존, 샘플정확 키음) · **키음 병렬 디코드**(코어 수만큼, 로딩 진행바).
- **판정/게이지** — beatoraja 윈도우 byte-단위 일치(**모드별**) · 콤보 · EX · 게이지 6종 + 모디파이어 · 클리어램프(beatoraja 색) · LN 풀판정(릴리스 윈도우 #RANK/JUDGE WIDTH 스케일) · **空POOR 정확 처리**.
- **렌더** — 1280×720 wgpu 인스턴스 · 노트 상단 클리핑 · **롱노트 몸통 막대** · 키 빔 · 레인 외곽선/구분선 · 가로 게이지(IIDX식) · BGA(이미지, on/off) · **KEY BOMB**(데이터주도) · **다국어 폰트**(cosmic-text+Inter+시스템폴백, 글리프 런 캐시) · **UI 테마**(`theme.ron`, → [docs/theme.md](docs/theme.md)).
- **게임 옵션** — 노트옵션(MIRROR/RANDOM/S-RANDOM/R-RANDOM/ROTATE/ALL-SCRATCH/H-RANDOM) · 하이스피드 고정(FLOATING/CONSTANT 그린넘버) · 판정 오프셋 + **오토 캘리브레이션** · JUDGE WIDTH · TOTAL · lift · lane cover · 스크래치 side/auto.
- **곡선택(GUI)** — **검색(`/` 제목·아티스트 부분일치) · 정렬(`F3` 제목/아티스트/레벨/클리어) · 클릭 가능한 하단 네비 버튼** · 다중 폴더 라이브러리(여러 폴더 병합) · 난이도표 네비(키보드+마우스) · 커버(`#STAGEFILE`/`#BANNER`) · KEY/레벨 배지 · 클리어램프 LED · **노트 밀도 그래프**(PEAK/AVG/END) · 로컬 기록 인라인 + 상세 모달.
- **데이터 주도** — 모드/스킨(RON)/키맵(RON)/플레이옵션(RON)/**UI 테마(RON)**/**라이브러리 폴더(RON)**/난이도표(RON). 설정 탭 UI · 통합 키 설정(파일+인앱 에디터) · 영속(`~/.config/rbms/`, 파싱 실패 시 `.ron.bak` 백업).
- **그 외** — autoplay · 리플레이 저장/재생(시드·옵션 복원) · 스코어 랭크 그래프(IIDX 9분법) · 리플레이 분석 모드 · 난이도표 인앱 관리(다중) · DEBUG 오버레이 · IR 슈퍼셋 스코어 인터페이스(클라).

## 남은 일

- [ ] **`#PREVIEW` 프리뷰 재생** — 배선·토글까지 구현했으나 **실제 재생 미동작(TODO·디버깅 필요)**.
- [ ] **데이터화 마무리** — 메뉴/결과 **색은 `theme.ron`으로 완료**, **패널 위치/레이아웃**의 데이터화는 남음. 설정 화면 NETWORK 탭(서버/플레이어 ID)·마우스 스테퍼 UX → [docs/roadmap.md](docs/roadmap.md).
- [ ] **백엔드 서버**(Bun + Hono + Drizzle, IR-슈퍼셋) → 라이벌 · 리더보드(타인 리플레이) · 설정 동기화 · ranked 무결성.
- [ ] **웹 FE** — 검색 · 리더보드 · 플레이어 페이지 · 리플레이 뷰어.
- [ ] **배포** — GitHub 원격/Actions · LICENSE 파일 동봉 · 코드 서명/공증(선택) · `.app`/`.dmg`/인스톨러(선택).
- [ ] **잔여 한계** — 윈도우 리사이즈 리플로우 없음 · BGA 비디오(mpg) 미지원 · CN/HCN 판정 차별화 · 게이지 5K/PMS 변종 · 폰트 글리프 아틀라스(P3a)/웹폰트(P4) · 스크래치 회전 단순화.

## 빌드 & 실행

```bash
cargo build --release -p rbms-player
BIN=./target/release/rbms-player

$BIN "<곡 폴더>"                  # GUI 곡선택 (↑↓ 이동, Enter 열기, / 검색, F3 정렬, O 폴더관리, T 난이도표, Tab 설정, Esc 뒤로 — 하단 버튼 클릭도 가능)
$BIN "<차트.bme>" [--interactive] # 단일 차트 (기본 autoplay)
$BIN --replay <file.ron>          # 리플레이 재생
```

- **레인 키**(기본 beatoraja Z열): 7K = `Z S X D C F V` + LShift(스크), 5K/9K/14K는 모드별 프리셋(차트 자동감지). 조작: ↑↓ 속도 · →← 커버 · `]` `[` lift. 모든 키는 **설정 → KEY CONFIG**에서 재바인딩.
- **CLI 옵션**: `--interactive|--auto --sc-left --sc-auto --lift F --hispeed F --gauge X --keys ... --skin file.ron --font file.ttf --table URL --keyconfig path.ron --replay file.ron --server URL --player ID`.
- **테마**: `~/.config/rbms/theme.ron`(첫 실행 시 자동 생성, 편집 가능) — UI 색 커스터마이즈. → [docs/theme.md](docs/theme.md).
- **테스트**: `cargo test --workspace` (~880개, 엣지케이스 중심). 설계 문서는 [docs/](docs/) (architecture · theme · roadmap).

## 아키텍처 (의존 위→아래)

```
apps/rbms-player → rbms-play → {rbms-render, rbms-audio, rbms-ir, rbms-judge, rbms-chart, rbms-table} → rbms-parser → rbms-model
```

| 크레이트 | 역할 |
|---|---|
| rbms-model | 순수 타입(Mode·Note·TimeLine·Model) |
| rbms-parser | BMS lexical(base36/62·Shift-JIS·`#RANDOM`·MD5/SHA-256) |
| rbms-chart | detect_mode·타이밍 적분·scroll·shuffle(노트옵션)·note_density |
| rbms-judge | 판정 윈도우·매칭·콤보·EX·게이지·클리어램프·LN |
| rbms-audio | symphonia 디코드 + cpal RT 믹서(샘플 클럭) |
| rbms-render | Renderer trait + CPU 백엔드 + skin·playfield·hud·result·select·key-bomb·다국어 font(cosmic-text) |
| rbms-ir | IR 슈퍼셋 DTO·ScoreServer trait·HTTP/Null 클라 |
| rbms-table | 난이도표(BMS table) fetch·md5 매칭·level 그룹핑 |
| rbms-play | 통합 드라이버(스케줄러·autoplay·입력·판정·키빔·키봄 와이어링) |

## 라이선스

**GPL-3.0-or-later** (beatoraja 포팅 파생물 — copyleft). 번들 폰트 Inter는 SIL OFL 1.1.
</content>
