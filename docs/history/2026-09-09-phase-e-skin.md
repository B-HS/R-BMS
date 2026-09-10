# Phase E — 스킨 완전 커스터마이징 (완료, 2026-09-10)

> 명세 `docs/plan/2026-09-09-phase-e-spec.md`, 결정 `docs/acknowledge/2026-09-09-enhancement-decisions.md`(결정 1). 이탈은 전부 `docs/acknowledge/reference-divergences.md` §Phase E 세 표(착수 전 5건 E-D1~E-D5, 웨이브 3 통합 시점 9건 E-D6~E-D14, 적대 리뷰 반영 7건 E-D15~E-D21)에 등록되어 있다. 이 문서는 웨이브 0~3 진행 경과와 최종 게이트 실측, 그리고 스킨을 실제로 쓰는 사람을 위한 안내를 담는다.

## 1. 무엇이 들어왔나

JSON(및 관대한 json5 방언) 스킨 문서를 읽어 PLAY/SELECT/RESULT/DECIDE/KEYCONFIG 5화면을 그대로 그릴 수 있게 되었다. 신규 크레이트 `crates/rbms-skin`(문서 모델·로더·타이머·프로퍼티 레지스트리·`mlua` 샌드박스·경로 해석)과 `crates/rbms-render/src/skin_render/`(문서를 프리미티브로 그리는 렌더러), 그리고 `apps/rbms-player`의 SKIN 탭·문서 탐색/선택 UI로 구성된다.

## 2. 웨이브별 경과

- **웨이브 0(E-scaffold)**: `crates/rbms-skin` 모듈 트리를 명세 §1 그대로 빈 스텁으로 생성.
- **웨이브 1(E-prim, 명세 §2)**: `Renderer` trait에 텍스처 등록/해제·크기 조회·textured quad·클립 push/pop 6개 메서드 추가, `BlendMode`/`TextureFilter`/`UvRect` 도입. 배칭 규칙은 제출 순서를 유지하며 `(TextureId, BlendMode, TextureFilter, 회전 유무, 클립)`이 연속으로 같을 때만 병합(E-D1). 클리핑·회전 골든 PNG 3장 추가.
- **웨이브 1(E2, 타이머/DST)**: 레퍼런스 `SkinProperty.java`에서 `TIMER_*` 151개를 체크인된 생성기(`tools/gen-skin-timer.rs`)로 기계 추출해 `crates/rbms-skin/src/timer/generated.rs`에 커밋. DST(목적지 키프레임) 보간·`center`/`op`/`blend`/`filter` 계산.
- **웨이브 2(E-prop, 명세 §4)**: `SkinProperty.java`의 `public static final int` 선언 968개를 같은 방식으로 생성기(`tools/gen-skin-property.rs`)로 추출. 프로퍼티 레지스트리와 boolean/integer/float/text/misc 상태 원천 라우팅 표, 미구현 id 카운터 구축.
- **웨이브 2(E-load, 명세 §5)**: 모델·로더(JSON5 관대 파싱, branch/stretch/track 하위 로더)·Lua 샌드박스(`mlua`, io/os/require/load 없음, 인스트럭션·메모리 제한)·경로 해석(스킨 루트 이탈 거부, E-D5). 로더 통합 테스트를 새로 작성하는 과정에서 결함 2건(최소 픽스처를 아예 못 읽던 문제 등)을 발견해 수정.
- **웨이브 3(E-screen, 명세 §6)**: `crates/rbms-render/src/skin_render/` 신규 — 문서 좌표계(y-up) → 화면 좌표계(y-down) 변환, 타이머·프로퍼티를 살아있는 화면 상태에 결선. BGA 전용 특수 경로 제거(문서가 자기 오브젝트로 BGA도 그린다).
- **웨이브 3(E-ui, 명세 §7)**: SKIN 탭을 설정 descriptor 표에 추가, 스킨 문서 탐색·선택·행별 커스터마이즈 UI, `rbms-config`에 `SkinOptions` 영속화(§5.5).

## 3. 적대 리뷰 28건 전건 반영

웨이브 3 통합 직후 스킨 파이프라인 전체를 적대 리뷰해 critical 4 / major 8+8 / minor 9+3 = 28건을 확인했고, 전건에 테스트를 붙여 수정했다. 대표 항목:

- **critical**: 커스터마이즈 op가 런타임에 항상 false로 평가되어 대상 오브젝트가 영구히 안 그려지던 문제, 배열 안 조건부 아이템을 오브젝트 분기로 오인해 배열이 단일 값으로 붕괴하던 문제, 정수 value 오브젝트의 zeropadding 필드 오독, 알파 0 오브젝트가 blend 4/9에서 조기 반환 없이 계속 그려지던 문제(레퍼런스 `SkinObject.java:624-626` 패리티).
- **major**: 11/22칸 floatvalue 스트립의 소수점 미표시, `isSignvisible`을 모든 float 레이아웃에 오적용, 스킨 선택 시 BGA가 전혀 안 그려지던 문제, Lua 프레임당 호출 예산이 value/text/number 식에는 미적용, include 확장 총량 무제한(수 KB 문서가 렌더 스레드를 무한정 붙잡음), BGA/커버가 원본 해상도 그대로 Nearest 축소되어 화질 회귀, 배경 텍스처를 변경 없이도 매 프레임 전량 재업로드, 글리프 아틀라스 UV가 페이지 성장을 못 견뎌 지연 백엔드에서 텍스트 깨짐.
- **minor**: 정수 value의 MIN/MAX 센티널 미처리, dstfilter 판정 기준(문서 좌표 vs 화면 픽셀), FLOAT_* 프로퍼티 클램프·gain 순서 반전, register_texture 계약 위반, clear()의 클립 스택 처리 백엔드 불일치, 프레임당 힙 할당, 성능/백엔드 대조 커버리지 부재, 커밋된 골든 PNG가 결정 E2의 "3장 이하"를 초과(→ 예산 기준 200 KB 이하로 정정, E-D10).

레퍼런스와의 일치 복구(알파 0 조기 반환, 커스터마이즈 op, ArraySerializer 의미, 11/22칸 소수점, padding 필드, isSignvisible 26칸 한정, dstfilter/NO_RESIZE 화면 픽셀 기준)는 `docs/acknowledge/reference-divergences.md`에 별도로 적지 않았다 — 그것들은 이탈이 아니라 일치이기 때문이다. rbms 고유의 새 결정만 E-D15~E-D21로 등록했다(Lua 패턴 함수 제거, 청크 텍스트 모드 고정, 벽시계 예산 2종, include 총량 상한, 숫자 자릿수 상한 16, 스킨 파일 워커 풀 디코드, 글리프 아틀라스 세대별 키).

## 4. 최종 게이트 실측 (2026-09-10, 이 세션)

- `cargo fmt --all` — 변경 없음(이미 정렬됨)
- `cargo build --workspace` — 통과
- `cargo test --workspace` — **2633 통과 · 0 실패**
- `cargo clippy --workspace --all-targets -- -D warnings` — **0 경고**
- 레퍼런스 구현 이름 금지어 grep(`crates apps tools docs`) — **0건**
- 이 세션에서 새로 추가된 파일 중 실제 Rust 주석(`//`, `/// ` 제외, `//!` 모듈 문서 제외)은 **0건** — 유일하게 매칭된 `apps/rbms-player/src/skin_select/fixtures.rs:41`의 `// the seven-key play screen`은 JSON5 테스트 픽스처를 담은 raw 문자열 리터럴 내부 텍스트이며 Rust 주석이 아니다.
- 커밋된 골든 PNG(`crates/rbms-render/tests/golden/`) 4장, 합계 약 23 KB — 200 KB 예산 이내(E-D10). 이번 세션에서 mtime 변경 없음(코드·상수 무변경으로 그대로 통과).
- 헤드리스 스크린샷: 기본(내장) SELECT 화면과 픽스처 JSON 스킨 SELECT 화면을 각각 PNG로 저장 —
  - `/private/tmp/claude-501/-Users-gkn-R-BMS/847c1230-e223-4d0c-adb0-2d6113bbb669/scratchpad/phase-e-shots/default-select.png` (`cargo run --example render_select -p rbms-render`, 1280x720, 문서 미선택 시의 내장 레이아웃)
  - `/private/tmp/claude-501/-Users-gkn-R-BMS/847c1230-e223-4d0c-adb0-2d6113bbb669/scratchpad/phase-e-shots/fixture-skin-select.png` (커밋된 골든 `crates/rbms-render/tests/golden/skin_fixture.png`과 동일 — 이 골든 자체가 `crates/rbms-render/tests/skin/skin.json` 픽스처 문서를 헤드리스로 그린 결과다, 512x288)

## 5. 알려진 미구현 / 후속 (E-D9, E-D14)

- Note/Judge/Gauge/그래프 계열 오브젝트(`Note`/`Judge`/`Gauge`/`gaugegraph`/`judgegraph`/`bpmgraph`/`hiterrorvisualizer`/`timingvisualizer`/`pmchara`/`songlist`)는 아직 그리지 않는다(경고 1줄을 남기고 드롭). 이들은 프로퍼티 id 밖의 플레이 세션 데이터(레인별 노트 목록, 판정 팝업, 게이지 노드)를 원천으로 하며, 그 배선은 Phase E 범위 밖이다.
- 레인별 `BOMB`/`KEYON`/`HOLD` 타이머는 아직 켜지지 않는다(같은 원천 필요).
- trimmed 계열·`FitWidth`/`FitHeight`/`NoExpanding` stretch 모드는 1회 경고 후 `STRETCH`로 근사한다(E-D8).
- `TYPE_BILINEAR` 전용 셰이더는 하드웨어 Linear로 근사한다(E-D4) — 확대·축소 시 보간 품질만 미세하게 다르다.

## 6. 사용자 가이드 — 스킨은 어디에 두고, 어떻게 고르고, 어떻게 손보는가

### 6.1 스킨 폴더 위치

설정 파일(`settings.ron`) 옆의 `skin/` 폴더가 기본 위치다(`rbms_config::DEFAULT_SKIN_FOLDER`). SKIN 탭에서 다른 폴더를 지정하면 그 경로를 대신 쓴다(`SkinOptions::folder`, `None`이면 기본값). 한 문서는 `<스킨폴더>/<스킨이름>/<문서>.json`(또는 `.json5`) 형태로 두고, 그 문서가 참조하는 이미지·폰트·부분 문서(include)는 전부 같은 스킨 루트 아래에 있어야 한다 — 루트를 벗어난 경로는 로더가 거부한다(E-D5, `SkinError::PathEscape`).

### 6.2 화면별로 스킨을 고르는 법

설정의 SKIN 탭에서 화면(PLAY 7KEYS, SELECT, RESULT, DECIDE, KEY CONFIG 등 `SkinType` 14종)을 하나 고르면, 그 폴더 아래에서 찾은 문서 후보 목록이 뜬다. 화면마다 독립적으로 문서를 고르거나(`SkinOptions.selected`, 화면 id → 문서 경로), 비워 두면 그 화면은 내장(built-in) 레이아웃으로 그려진다 — 새로 설치한 상태의 기본값이 이것이다. 문서를 고른 화면은 그 프레임 전체를 문서가 그린다(E-D11) — 내장 레이아웃과 섞이지 않는다.

### 6.3 문서 안에서 값을 손보는 법(커스터마이즈)

문서가 선언한 커스터마이즈 가능 행(속성 선택지·파일 슬롯·오프셋)은 SKIN 탭 안에서 화면별로 값을 바꿀 수 있고, 그 선택은 문서 경로를 키로 `SkinOptions.custom`(`SkinCustomisation`)에 저장된다. 문서가 버전업하며 행이 늘거나 줄어도 이름이 같은 행은 선택이 그대로 유지된다. 오브젝트를 화면에서 드래그해 옮긴 오프셋도 같은 방식으로 저장된다.

### 6.4 문서를 새로 로드할 때

문서가 선언한 이미지·폰트는 백그라운드 워커 풀에서 디코드된다(E-D20) — 그 화면으로 전환한 직후 몇 프레임은 내장 레이아웃이 보이다가, 전부 도착하면 문서가 이어받는다. 문서가 아예 없거나 로드에 실패하면 그 화면은 계속 내장 레이아웃으로 그려진다.

### 6.5 스킨을 만들 때 알아 두면 좋은 제한

- Lua 표현식은 표준 `string` 라이브러리 중 패턴 함수(`find`/`match`/`gmatch`/`gsub`)와 `string.dump`를 쓸 수 없다(E-D15, 보안). `format`/`sub`/`rep`/`upper`/`lower` 등은 그대로 쓸 수 있다.
- 표현식 하나에 20ms, 프레임 전체에 4ms의 시간 예산이 있다(E-D17). 넘긴 오브젝트는 기본값으로 떨어질 뿐 스킨 전체가 죽지는 않는다.
- 숫자(`value`) 오브젝트의 자릿수는 최대 16자리로 제한된다(E-D19).
- include로 문서를 나눌 때, 한 번 로드에서 펼쳐지는 include 총량은 1024개를 넘을 수 없다(E-D18, 같은 대상은 캐시되어 1회만 읽는다).
