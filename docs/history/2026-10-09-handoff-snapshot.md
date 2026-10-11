# HANDOFF — 2026-10-09 세션 스냅샷

> 새 세션의 진입점. 대응 코드: `dev` 브랜치, 코드 기준 커밋 `9ce92bb`(이번 세션은 문서만 추가). 이전 스냅샷은 `docs/history/2026-09-18-handoff-snapshot.md`. 이 문서는 항상 덮어쓴다.

## 1. 지금 무엇을 하는가

beatoraja 용 풀 Lua 스킨 ModernChic 을 R-BMS 가 수정 없이 읽어 똑같이 그리게 하고, 그 스킨을 참조해 단순화한 기본 스킨을 자작한다. 구 스킨(`steel-neon` 3세대와 내장 화면 혼합 합성)은 삭제한다.

- 정본 사양: `docs/plan/2026-10-09-lua-skin-compat.md`(목표, 결정, 목표 구조, 삭제 대상, 웨이브 1~9, 검증, 게이트, 자동 시작 절차)
- 결정: `docs/acknowledge/2026-10-09-lua-skin-compat-decisions.md`(D1~D7, 기각 대안, 이전 결정과의 관계)
- 조사 전문과 색인: `docs/reference/skin-compat/README.md`
- 체크리스트: `docs/PROCESS.md` 최상단 L1~L7

## 2. 상태

| 단계 | 상태 |
| --- | --- |
| L1 조사, L1-b 조사 결과 저장, L2 사양·결정 | 완료 |
| L3 엔진 구현(웨이브 1~2) | 미착수. 2026-10-09 21:54 KST 예약 실행으로 웨이브 1 부터 시작 |
| L4 화면 전환(웨이브 3~7), L5 기본 스킨(웨이브 8), L6, L7 | 미착수 |

예약은 세션 한정이라 세션이 닫히면 사라진다. 사라졌으면 새 세션에서 사양 §10 절차대로 시작한다.

## 3. 다음 세션이 할 일

1. `docs/PROCESS.md` L 체크리스트의 첫 미완료 항목을 확인한다.
2. 사양 §6 의 해당 웨이브 표와, 각 단위 "근거" 열의 조사 보고서 절만 읽는다. 레퍼런스 소스·외부 스킨·R-BMS 코드를 통독하지 않는다.
3. Workflow 로 실행하고(골격 → 구현 → 적대 리뷰 → 수정), 메인이 게이트와 캡처를 확인한 뒤 조사 문서·PROCESS 갱신, 커밋·푸시.

## 4. 작업 규칙 (이번 작업에서 확정)

- 위임은 Workflow 전용. Agent 단독 호출과 general-purpose 금지. Opus 는 effort high 이하, Sonnet 은 xhigh 이하. 모든 `agent()` 에 `model` 과 `effort` 를 명시한다.
- 커밋은 웨이브가 게이트를 통과할 때마다 메인이 커밋·푸시한다. author 단독, AI 트레일러 금지, 선별 스테이징, force push 금지, 설명은 한국어.
- 이름 금지 규칙은 폐지됐다. beatoraja·ModernChic 이름을 문서와 코드에 쓸 수 있다. 다만 ModernChic 의 파일(Lua·이미지·폰트·사운드)을 저장소에 넣지 않는다.
- 조사 결과는 `docs/` 에 상세히 저장하고, 코드가 바뀌면 조사 문서의 해당 절을 같은 단위에서 고친다.
- UI 는 실제 렌더(캡처)를 확인한 뒤에만 됐다고 보고한다.
- Rust: `//` 주석 금지, 매직넘버 상수화, `#[allow]`·`unsafe` 금지, rustfmt `max_width 160`.
- `.env` 계열은 읽지도 쓰지도 않는다.

## 5. 환경

- macOS arm64, Rust 1.95.0(`rust-toolchain.toml`), `uv`·`bun` 설치됨. JRE 없음(beatoraja 실행 불가).
- 실행: `./start.sh [곡 폴더|차트] [옵션]`(gitignore 된 로컬 스크립트) 또는 `cargo run --release -p rbms-player -- <인자>`. 릴리스 빌드는 2026-10-09 에 1분 걸려 성공했다.
- 경로: 레퍼런스 체크아웃 `/Users/hyunseokbyun/development/beatoraja`(HEAD `8320241d`), 외부 스킨 `/Users/hyunseokbyun/Downloads/ModernChic`(476 MiB, 저장소 밖 유지).
- 격리 실행: `HOME=<임시 폴더> ./target/release/rbms-player samples/preview-demo` 로 사용자 설정을 건드리지 않는다.
- 조사 Workflow 실행 id: `wf_43ad1f8d-42b`(스크립트는 세션 디렉터리에만 있음).

## 6. 미해결

- 색 공간(E10)과 Lua 예산 수치는 웨이브 1·2 의 실측으로 정한다.
- 동영상 디코더 크레이트는 웨이브 7 의 W7-1 에서 선정해 결정 문서에 추가한다.
- GPU 창 확인은 사용자 절차다. beatoraja 캡처가 없어 글자 위치의 픽셀 단위 일치와 색감은 보장 범위 밖이다(D6).
- `web/`·IR 서버·릴리스(Phase R)는 이번 작업과 무관하며 상태는 `docs/roadmap.md` 그대로다.
