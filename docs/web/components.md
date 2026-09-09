# rbms web — 컴포넌트 / 디자인 토큰 매핑 (W0)

> 근거: `docs/acknowledge/design.md` §1-2(두 Surface) · §8(레이아웃) · §10(컴포넌트 카탈로그) · §11(페이지 패턴) · §16(Tailwind v4 `@theme` 딜리버리) · §17(에이전트 프롬프트).
> 이 문서는 **`bunx shadcn add` 의 정본 목록**이자, design.md 아키타입을 BMS IR 화면으로 옮긴 매핑표다.

---

## 1. Surface 배치

design.md §1-2 는 두 Surface 가 스타일시트를 공유하지 않는다고 못박는다. 이를 라우트 그룹으로 옮긴다.

| Surface | 라우트 그룹 | 화면 | radius | 그림자 | 배경 티어 | `--warning` |
|---|---|---|---|---|---|---|
| **A — 앱** | `(app)` | 홈·검색·차트·플레이어·리더보드·표·코스·리플레이·설정·관리 | `0rem` | none(오버레이만) | 3(sidebar/body/card) | 있음 |
| **B — 공개** | `(public)` | 로그인·가입·가이드 | `0.375rem` | `0 0 7px` 글로우 | 1(`oklch(1 0 0)`) | 없음 |

### 1-1. 토큰 스코프 — `data-surface` 속성 방식

**결정: CSS 파일 분리가 아니라 루트 `data-surface` 속성으로 스코프한다.**

- 이유: Next App Router 는 하나의 `<html>` 문서에서 라우트 그룹만 바꾸므로, 별도 스타일시트 두 벌을 로드/언로드할 수 없다. 반면 design.md 의 두 토큰 세트는 **같은 이름의 변수에 다른 값**을 넣는 관계라 CSS 변수 스코프로 완전히 재현된다.
- 구조:
  - `src/app/globals.css` 에 `@import 'tailwindcss'` + `@custom-variant dark (&:is(.dark *, [data-theme='dark'] *))` + design.md **§16-1 블록 전체**를 `:root` 에 그대로 이식(Surface A 가 기본).
  - 이어서 design.md **§16-2 의 델타만** `[data-surface='public']` 선택자로 재정의(`--radius`, `--shadow-*`, 배경 1티어, `--warning` 제거는 값 미사용으로 대체).
  - `(app)/layout.tsx` 는 `data-surface="app"`, `(public)/layout.tsx` 는 `data-surface="public"` 를 각 그룹 최상위 `div` 에 부여.
- `@theme` 의 `--color-*` 는 Tailwind 유틸리티를 생성하고, 실제 값은 `:root` / `[data-surface]` 의 CSS 변수를 따르므로 유틸리티 클래스(`bg-card` 등)는 두 Surface 에서 그대로 동작한다.
- **§16-1 의 radius 규칙 필수**: 네 파생 radius 를 전부 `var(--radius)` 에 핀한다. `calc(var(--radius) - 4px)` 는 `--radius: 0rem` 에서 `-4px` 가 되어 CSS 가 깨진다(design.md §6-3).

### 1-2. 다크모드

- 메커니즘: 루트에 `.dark` 클래스. `[data-theme='dark']` 는 interop 별칭(§16-1 `@custom-variant`).
- **pre-paint 스크립트(§16-3)를 `app/layout.tsx` 의 `<head>` 최상단, 스타일시트 링크보다 앞, `defer` 없이** 인라인한다. design.md 는 Surface A 가 이걸 빠뜨려 플래시가 있다고 기록했으므로, 우리는 **양쪽 Surface 모두에 적용**한다.
- 저장 키: `rbms-theme`(원본 `flunti-otel-theme` 를 제품명으로 치환).
- **`next-themes` 는 이미 `web/package.json` 에 설치되어 있다.** 이를 쓰되, design.md §4-7 이 기록한 결함(`Toaster` 가 `useTheme()` 를 호출하는데 provider 가 마운트되지 않아 sonner 만 OS 테마를 따라가는 불일치)을 재현하지 않도록 **`ThemeProvider` 를 반드시 마운트**한다(`attribute="class"`, `storageKey="rbms-theme"`, `defaultTheme="system"`). pre-paint 스크립트는 next-themes 가 자체 주입하므로 §16-3 스크립트를 수기로 넣지 않는다(중복 주입 시 키가 어긋나면 플래시가 남는다 — 실제 렌더로 확인할 것).

---

## 2. 페이지 아키타입 매핑

design.md §11 의 7개 아키타입(A~G) + H(공개)를 이 제품 화면에 배정한다.

| 화면 | 아키타입 | 구성 | 근거 |
|---|---|---|---|
| `/` 홈 | **11-C Metric Overview** | 스탯 타일 4~5(차트 수·플레이어 수·제출 수·오늘 제출) → 제출 추이 차트 패널(224px) → 최근 활동 테이블 패널 | §11-C |
| `/search` 차트 검색 | **11-F Query Console** | 컨트롤 패널(모드 탭 · 레벨 칩 · 정렬 탭 · 검색어) → 결과 테이블 패널 + 페이저. 모든 컨트롤이 URL 에 직렬화 | §11-F |
| `/charts/[hash]` 리더보드 | **11-B Stacked Detail** | 아이덴티티 패널(제목/부제/아티스트 + 배지행(mode·level·lntype·notes) + muted 메타(BPM·TOTAL·해시)) → 타일 그리드(최고 EX·최고 램프·플레이어 수·내 순위) → 랭킹 테이블 패널 → 리플레이 목록 패널 | §11-B |
| `/players/[id]` 플레이어 | **11-B + 11-A** | 아이덴티티 패널(이름·rank 배지·총 플레이) → 램프 분포 Bar-Count 패널 → 최근 기록 테이블 패널 → 라이벌 패널 | §11-B, §10-6 |
| `/leaderboards` | **11-A Signal List** | 툴바 패널(정렬 칩) → 결과 테이블 패널 + offset 페이저 | §11-A |
| `/tables`·`/tables/[id]` | **11-A** | 표 목록 → 폴더별 차트 테이블(폴더 = Collapsible 섹션) | §11-A, §10-13 |
| `/courses/[hash]` | **11-B** | 코스 아이덴티티(구성 차트 목록 · constraint 배지) → 랭킹 테이블 | §11-B |
| `/replays/[id]` 리플레이 뷰어 | **11-G Full-bleed Visualization** | µs 이벤트를 레인×시간 워터폴로. **자체 오버플로 소유, 반응형 무시, 스크롤**. 라이브러리 미도입, 인라인 SVG + 절대 픽셀 상수 | §11-G, §10-7, §10-14 |
| `/settings` | **11-D Tabbed Console → 11-E Admin CRUD** | 탭 바 패널(프로필 / API 토큰 / 동기화 blob / 라이벌) → 각 탭 본문이 11-E(툴바 + 결과 테이블 + 액션 클러스터) | §11-D, §11-E |
| `/admin/builds` | **11-E Admin CRUD** | 툴바(빌드 등록) + 테이블(sha256·version·platform·channel·trusted) | §11-E |
| `/login`·`/signup`·`/guide` | **11-H Public** | 384px 카드(24px 패딩·16px 갭·실제 border·6px radius·shadow-sm). 가이드는 48px sticky 헤더 + 768px 중앙 컬럼 | §11-H |

### 2-1. 이 제품에서 쓰지 않는 아키타입

- **11-G 의 트레이스 워터폴 원형**은 리플레이 뷰어로만 재사용한다.
- Service Map Graph(§10-14)는 대응 화면 없음 → 미사용. 단 "SVG 는 유틸리티 클래스 대신 `var(--color-*)` 직접 참조" 규칙만 리플레이 뷰어에 적용한다.

---

## 3. design.md 아키타입 → 우리 컴포넌트

design.md §10 의 13개 주 아키타입 중 채택분. 전부 `features/`(순수 UI) 또는 `widgets/`(데이터 결합)에 자체 구현하며, 아래 §4 의 shadcn 프리미티브를 조합한다.

| design.md | 우리 컴포넌트(레이어) | 용도 | 커스텀 포인트 |
|---|---|---|---|
| §10-1 Panel Card | `features/shell/panel-card` | 모든 블록의 유일한 표면 | border 0 · radius 0 · 인셋 12px 양축. 제목은 **`h2`** 로 렌더(§7-4 지적사항 선반영) |
| §10-2 Stat Tile | `features/shell/stat-tile` | KPI 셀(총 제출·플레이어 수·내 순위 등) | `border-0` 필수(shadcn `Item` 기본 border 제거, 268px 베이스라인 유지) |
| §10-3 Data Table | `features/table/data-table` | 랭킹·검색결과·최근기록·토큰·빌드 | 컬럼 디스크립터 `{ key, label, width, align, cell }`. 우측 정렬 컬럼은 자동 `tabular-nums`(EX·BP·콤보·순위 전부 해당) |
| §10-4 Series Chart | `widgets/chart/series-chart` | 제출 추이·EX 추이 | 높이 224px 고정. **애니메이션 금지**. 시리즈 색은 `--color-chart-1..5`, 다크에서 hue 회전(§4-4) |
| §10-5 Status Badge | `features/badge/lamp-badge` 등 | 램프(11종)·ranked/unranked·flags·표 레벨 | radius 9999px(유일한 라운드). **램프 11종 → 시맨틱 색 매핑표는 §5** |
| §10-6 Bar-Count List | `features/stats/lamp-distribution` | 플레이어 램프 분포·레벨 분포 | 바 폭 전이 0.24s |
| §10-7 Trace Waterfall | `widgets/replay/replay-waterfall` | µs 리플레이 타임라인 | 절대 픽셀 상수. 자체 오버플로 |
| §10-8 Attribute Tree | `features/inspect/attribute-tree` | 스코어 상세 `options`/`extra` 원시 JSON 인스펙터 | 재귀 key/value |
| §10-9 Form Dialog | `widgets/*/…-form-dialog` | 토큰 발급·빌드 등록·라이벌 추가 | 최대폭 672px(`sm:max-w-2xl`) |
| §10-10 State Triad | `features/state/{empty,load-error,query-error}` | 빈/전송오류/조회오류 3상태 | **인라인 가드**, early return 금지. 순서: pending → 전송오류 → 조회오류 → empty → 결과 |
| §10-11 Navigation Rail | `widgets/shell/nav-rail` | 좌측 256px 레일 | 항목 높이 36px, 트리거 32px, **우측 border 제거는 `group-data-[side=left]:border-r-0` 로**(변형 특이성 함정, §6-4) |
| §10-12 Context Filter Panel | `widgets/shell/filter-panel` | 우측 320px 필터(모드·레벨·램프·기간·검색어·플레이어) | 전부 URL 직렬화 |
| §10-13 Pager | `features/table/pager` | offset 페이저(page/total 표기) | 비활성 = `opacity .5` + `pointer-events: none` |
| §10-13 Confirm Action | `features/dialog/confirm-action` | 토큰 폐기·라이벌 삭제 | destructive 확인, 취소는 좌측 ghost |
| §10-13 Column Toggle | `features/table/column-toggle` | 랭킹 표시 컬럼 선택 | `cols=` URL 파라미터 |
| §10-13 Panel Footnote | `features/shell/panel-footnote` | "unranked 기록은 랭킹에서 제외됩니다" 등 | `--text-xs` muted |

---

## 4. shadcn 컴포넌트 목록 (`bunx shadcn add` 정본)

초기화: `bunx shadcn@latest init` — style `new-york`, baseColor `neutral`, cssVariables `true`, icon library `lucide`. `components.json` 의 `aliases.ui = @shared/ui`, `aliases.utils = @shared/lib/cn`.

### 4-1. W1(즉시 필요)

| 컴포넌트 | 사용 화면 | design.md 근거 | 커스텀 |
|---|---|---|---|
| `sidebar` | `(app)` 좌측 레일 | §8-1, §10-11 | **필요** — 우측 border 제거(변형 특이성), 항목 h-9, 트리거 size-8 |
| `card` | 모든 Panel Card | §10-1 | **필요** — border 0 · radius 0 · py-3/px-3, 제목 `h2` |
| `table` | 랭킹·검색·최근기록·토큰 | §10-3 | 소폭(우측정렬 `tabular-nums`) |
| `button` | 전역 | §14-5 | 없음 |
| `badge` | 램프·ranked·flags·레벨 | §10-5 | 시맨틱 색 매핑만 추가 |
| `input` | 검색·폼 | §11-H | 없음 |
| `label` · `form` | 로그인·가입·토큰 발급 | §10-9 | 없음(RHF+zodResolver 전제) |
| `skeleton` | 모든 로딩 | §10-10, §17-1 | 없음. **스피너 금지**(로딩은 스켈레톤) |
| `separator` | 레일·패널 내부 구분 | §6-4 | 없음 |
| `scroll-area` | 콘텐츠 컬럼·필터 패널 | §8-5 | 없음 |
| `tooltip` | 판정 약어(EPG/LGR…) 설명 | §10-13 | 없음. **native `title=` 금지** |
| `sonner` | 제출·토큰 발급 토스트 | §14-8 | **필요** — 테마를 앱 `localStorage` 에 연결(§1-2) |
| `sheet` | 모바일 사이드바 | §8-1 | 없음(288px) |
| `select` | 정렬·모드 선택 | §11-F | 없음 |
| `tabs` | `/settings`, 신호/메트릭 탭 | §11-D | 없음 |
| `empty` | State Triad 의 empty | §10-10 | 없음 |
| `item` | Stat Tile 기반 | §10-2 | **필요** — `border-0` |

### 4-2. W2(확장 시)

| 컴포넌트 | 사용 화면 | 근거 |
|---|---|---|
| `dialog` | 토큰 발급·빌드 등록 Form Dialog | §10-9 |
| `alert-dialog` | Confirm Action(토큰 폐기·삭제) | §10-13 |
| `dropdown-menu` | 테이블 행 액션·소스 선택 | §11-A |
| `popover` | 기간 선택·필터 보조 | §10-13 |
| `checkbox` · `switch` | 필터 토글·unranked 포함 여부·trusted 토글 | §11-E |
| `toggle` · `toggle-group` | Column Toggle 칩 그룹 | §10-13 |
| `pagination` | offset 페이저 | §10-13 |
| `collapsible` | 표 폴더·리플레이 프레임 접기 | §10-13 |
| `alert` | 가이드 경고·unranked 안내 | §11-H |
| `textarea` | 설정 blob 편집(읽기 전용 뷰 포함) | §9 settings |
| `breadcrumb` | 표 > 폴더 > 차트 | §10-13 |
| `kbd` | 가이드의 CLI 예시 키 표기 | §10-13 |
| `field` · `button-group` | 폼 구성 보조 | §10-9 |
| `chart` | Series Chart 래퍼(Recharts) | §10-4 |

### 4-3. 설치하지 않음

`accordion`(Collapsible 로 충분) · `command`(원본에서도 importer 0) · `resizable`(importer 0) · `spinner`(로딩은 스켈레톤) · `avatar`(아바타 없음) · `calendar`(기간은 프리셋 + 입력).

---

## 5. 도메인 시맨틱 매핑

design.md §10-5 는 상태 배지의 **시맨틱 색 시스템**을 규정한다. BMS 도메인 값을 여기에 배정한다.

### 5-1. ClearLamp(11종) → 배지 시맨틱

| 램프 | 의미 | 토큰 |
|---|---|---|
| `Max` · `Perfect` | 최상 | `--color-chart-1`(전용 강조) |
| `FullCombo` | 최상 | `--color-chart-2` |
| `ExHard` | 상 | `--color-chart-3` |
| `Hard` | 상 | `--color-chart-4` |
| `Normal` | 중 | `--color-chart-5` |
| `Easy` | 중 | `--color-muted-foreground` 계열 outline |
| `LightAssistEasy` · `AssistEasy` | 어시스트 | outline + muted (unranked 뉘앙스) |
| `Failed` | 실패 | `--color-destructive` |
| `NoPlay` | 없음 | muted outline |

> **가정**: chart-1~5 는 원래 시리즈 색이지 상태 색이 아니다. 램프가 11단계라 semantic 5종(success/warning/destructive/info/muted)으로는 부족해 시리즈 팔레트를 상태 인코딩에 전용한다. 다크에서는 §4-4 의 hue 회전이 그대로 적용되므로 명도 대비는 유지된다. 색만으로 구분하지 않도록 **배지 텍스트에 램프명을 항상 표기**한다(§7-4 접근성).

### 5-2. ranked / flags

| 값 | 표현 |
|---|---|
| `ranked: true` | 배지 없음(기본) |
| `ranked: false` | outline 배지 `unranked` + flags 를 muted 배지로 나열(`AUTOPLAY` `ASSIST` `JUDGE_WIDTH` `TOTAL_OVERRIDE` `UNKNOWN_BUILD` `GUEST`) |
| `verified: true` | `--color-chart-2` 배지 `verified` |

`unranked` 행은 랭킹 테이블에 나타나지 않는다(서버가 제외). 플레이어 기록 목록에서는 보이며 **행 전체를 muted 처리**한다.

### 5-3. 숫자 표기

- EX·BP·콤보·노트수·순위·µs 값은 전부 **`tabular-nums`** + 우측 정렬(§10-3).
- 해시(md5/sha256)·빌드 sha256 은 **mono**(§5-3 "mono 는 액센트가 아니라 1급 역할"). 목록에서는 앞 8자 + 말줄임, 툴팁에 전문.
- 시각은 서버가 epoch ms(UTC) 로 준다. 표시 계층에서만 dayjs 로 로컬 변환(§common.md §9).

---

## 6. 필수 제약 (design.md §17-1 마스터 프롬프트 요약 — 위반 시 리뷰 반려)

1. `border-radius: 0` (Surface A). 파생 radius 4단계 전부 `var(--radius)` 에 핀. `calc(var(--radius) - Npx)` 금지.
2. 표면에 border·shadow 없음. 블록 분리는 `--color-background` 위 **1px gap** 하나로만. gap 은 부모가 단독 생성(자식이 `pt-px` 를 더하면 1px 틀어짐).
3. 폰트 두께는 **400 / 500 / 600 / 800** 넷뿐. `font-bold`(700) 금지.
4. 색은 `var(--color-*)` 만. hex·rgb·hsl·Tailwind 팔레트 유틸리티(`neutral-800`, `black/50`) 금지.
5. 콘텐츠 인셋 12px 양축. 모든 페이지 블록은 Panel Card.
6. 모션: opacity 0.18s, 바 폭 0.24s, `cubic-bezier(0.4,0,0.2,1)`. 그 외 무애니메이션. **차트는 애니메이션하지 않는다.**
7. 로딩은 스켈레톤. 버튼의 "로딩"은 disabled.
8. `.dark` 와 `[data-theme='dark']` 둘 다 지원.
9. **이모지 금지.** 아이콘은 lucide 만.
10. 동적 className 은 `cn()` 합성(템플릿 리터럴 금지). 툴팁은 shadcn `Tooltip`(native `title` 금지).

### 6-1. 자체 검사 grep (컴포넌트 작성 후 매번)

```
/#[0-9a-fA-F]{3,8}|rgb\(|hsl\(/          → 0건 (globals.css 팔레트 주석 제외)
/rounded-(?!none)|shadow-(?!none)|font-bold/ → 0건 (Surface B 카드 제외)
```

---

## 7. 접근성 — 원본 결함 선반영

design.md §7-4 는 Surface A 의 두 결함을 "알려진 이월"로 기록했다. 신규 구축이므로 **고쳐서 시작한다**(§7-4 가 권장하는 방향).

- Panel Card 제목을 `div` 가 아니라 **`h2`** 로 렌더 → 30여 화면에 문서 개요 제공. 시각 결과는 동일.
- 콘텐츠 컬럼에 **`<main>`**, 좌측 레일에 **`<nav>`**, 우측 필터 패널에 `<aside>` 를 붙여 랜드마크를 채운다.
- 램프·ranked 는 색 + 텍스트 이중 인코딩(§5-1).
- 리플레이 워터폴은 `role="img"` + `aria-label` 요약. 키보드 내비게이션은 v1 미지원(원본과 동일한 알려진 한계).
