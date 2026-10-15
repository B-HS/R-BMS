# rbms web — 컴포넌트 / 디자인 토큰 매핑 (W0)

> 최종 갱신 2026-10-15 · 대응 커밋 ea698a9

> **구현 상태:** 이 문서는 W0 설계 기준입니다. W1/W2 UI 통합은 완료됐고, 현재 컴포넌트와 경로는 `web/src/{shared,entities,features,widgets,app}`가 정본입니다. 네이티브 `theme.ron` 편집·동기화는 웹 기능이 아니며 [../theme.md](../theme.md)의 경계를 따릅니다.

> 근거: `docs/acknowledge/design.md` §1-2(두 Surface) · §8(레이아웃) · §10(컴포넌트 카탈로그) · §11(페이지 패턴) · §16(Tailwind v4 `@theme` 딜리버리) · §17(에이전트 프롬프트).
> 이 문서는 **`bunx shadcn add` 의 정본 목록**이자, design.md 아키타입을 BMS IR 화면으로 옮긴 매핑표다.

---

## 1. Surface 배치

design.md §1-2 대로 Surface 는 하나다(2026-10-14, T1). 모든 화면이 루트 `app/layout.tsx` 의 `AppShell` 안에 있고, `data-surface` 속성과 `[data-surface='public']` 델타는 지웠다. 라우트 그룹 `(app)`·`(public)` 은 폴더 정리용으로만 남아 있고 레이아웃 파일은 없다.

| 화면 | radius | 그림자 | 배경 티어 |
|---|---|---|---|
| 전부(홈·검색·순위·차트·플레이어·표·코스·리플레이·설정·관리·로그인·가입·가이드·404) | `0rem` | none(오버레이만) | 3(sidebar/body/card) |

### 1-2. 다크모드

- 메커니즘: 루트에 `.dark` 클래스. `[data-theme='dark']` 는 interop 별칭(§16-1 `@custom-variant`).
- 테마 적용은 `Providers`의 `next-themes` `ThemeProvider`가 담당합니다. 설정은 `attribute={['class', 'data-theme']}`, `defaultTheme='dark'`, `enableSystem={false}`, `disableTransitionOnChange`, `storageKey='rbms-theme'`입니다. 기본은 다크이고 사이드바 바닥의 버튼으로 라이트를 고릅니다(T2).
- next-themes가 자체 주입하는 pre-paint 처리를 사용합니다. `app/layout.tsx`에 수기 인라인 스크립트를 추가하지 않습니다.
- `AppToaster`(`shared/providers/app-toaster.tsx`)와 `TooltipProvider`는 같은 `ThemeProvider` 하위에 있어 모든 Surface가 같은 테마 상태를 사용합니다.

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
| §10-1 Panel Card | `features/shell/panel-card` | 모든 블록의 유일한 표면 | border 0 · radius 0 · 인셋 12px 양축. 제목은 **`h2`** 로 렌더(§7-4 구현 기준) |
| §10-2 Stat Tile | `features/shell/stat-tile` | KPI 셀(총 제출·플레이어 수·내 순위 등) | `border-0` 필수(shadcn `Item` 기본 border 제거, 268px 베이스라인 유지) |
| §10-3 Data Table | `features/table/data-table` | 랭킹·검색결과·최근기록·토큰·빌드 | 컬럼 디스크립터 `{ key, label, width, align, cell }`. 우측 정렬 컬럼은 자동 `tabular-nums`(EX·BP·콤보·순위 전부 해당) |
| §10-4 Series Chart | `widgets/chart/series-chart`(**만들지 않았다** — 제출 추이 차트는 홈에 없다. 홈 재구성은 웹 2차 후보) | 제출 추이·EX 추이 | 높이 224px 고정. **애니메이션 금지**. 시리즈 색은 `--color-chart-1..5`, 다크 값 재바인딩(§4-4) |
| §10-5 Status Badge | `features/lamp/lamp-label`·`lamp-swatch`·`lamp-strip`, `features/badge/meta-badge`, `features/score/scratch-auto-badge`(A-SCR) 등(옛 `lamp-badge` 는 d38b6fb 에서 대체됨) | 램프(11종)·ranked/unranked·flags·표 레벨 | radius 9999px(유일한 라운드). **램프 11종 → 시맨틱 색 매핑표는 §5** |
| §10-6 Bar-Count List | `features/stats/bar-count-list`, `features/lamp/lamp-distribution`·`lamp-stack-bar` | 플레이어 램프 분포·레벨 분포 | 바 폭 전이 0.24s |
| §10-7 Trace Waterfall | `widgets/replay/replay-waterfall` | µs 리플레이 타임라인 | 절대 픽셀 상수. 자체 오버플로 |
| §10-8 Attribute Tree | `features/inspect/attribute-tree`(**만들지 않았다** — 스코어 상세 인스펙터 화면이 없다) | 스코어 상세 `options`/`extra` 원시 JSON 인스펙터 | 재귀 key/value |
| §10-9 Form Dialog | `widgets/*/…-form-dialog` | 토큰 발급·빌드 등록·라이벌 추가 | 최대폭 672px(`sm:max-w-2xl`) |
| §10-10 State Triad | `features/state/{empty,load-error,query-error}` | 빈/전송오류/조회오류 3상태 | **인라인 가드**, early return 금지. 순서: pending → 전송오류 → 조회오류 → empty → 결과 |
| §10-11 Navigation Rail | `widgets/shell/shell-nav-menu`(+ `app-shell`·`shell-account-control`·`shell-theme-control`) | 좌측 256px 레일 | 항목 높이 36px, 트리거 32px, **우측 border 제거는 `group-data-[side=left]:border-r-0` 로**(변형 특이성 함정, §6-4) |
| §10-12 Context Filter Panel | `features/filter/filter-bar`·`filter-field`·`scratch-auto-toggle`, 표는 `widgets/table/table-snapshot-sidebar`(우측 패널이 아니라 사이드바 슬롯 `shell-sidebar-portal`) | 우측 320px 필터(모드·레벨·램프·기간·검색어·플레이어) | 전부 URL 직렬화 |
| §10-13 Pager | `features/table/pager` | offset 페이저(page/total 표기) | 비활성 = `opacity .5` + `pointer-events: none` |
| §10-13 Confirm Action | `features/dialog/confirm-action` | 토큰 폐기·라이벌 삭제 | destructive 확인, 취소는 좌측 ghost |
| §10-13 Column Toggle | `features/table/column-toggle` | 랭킹 표시 컬럼 선택 | `cols=` URL 파라미터 |
| §10-13 Panel Footnote | `features/shell/panel-footnote` | "unranked 기록은 랭킹에서 제외됩니다" 등 | `--text-xs` muted |

---

## 4. shadcn 컴포넌트 목록 (`bunx shadcn add` 정본)

현재 `components.json` 설정: style `radix-nova`, base color `neutral`, CSS variables 활성화, icon library `lucide`, `aliases.ui = @shared/ui`, `aliases.utils = @shared/lib/cn`.

### 4-1. W1 완료 항목

| 컴포넌트 | 사용 화면 | design.md 근거 | 커스텀 |
|---|---|---|---|
| `sidebar` | 셸 좌측 사이드바 | §8-1, §10-11 | 적용(2026-10-14, iidx-rank 수정본을 옮김: 안쪽 스크롤 `ScrollContainer`, 문구 한국어) — 우측 border 제거, 항목 h-9, 트리거 size-8 |
| `card` | 모든 Panel Card | §10-1 | 적용 — border 0 · radius 0 · py-3/px-3, 제목 `h2` |
| `table` | 랭킹·검색·최근기록·토큰 | §10-3 | 소폭(우측정렬 `tabular-nums`) |
| `button` | 전역 | §14 | 없음 |
| `badge` | 램프·ranked·flags·레벨 | §10-5 | 시맨틱 색 매핑만 추가 |
| `input` | 검색·폼 | §11-H | 없음 |
| `label` · `form` | 로그인·가입·토큰 발급 | §10-9 | 없음(RHF+zodResolver 전제) |
| `skeleton` | 모든 로딩 | §10-10, §17-1 | 없음. **스피너 금지**(로딩은 스켈레톤) |
| `separator` | 레일·패널 내부 구분 | §6-4 | 없음 |
| `scroll-area` | 콘텐츠 컬럼·필터 패널 | §8-1 | 없음 |
| `tooltip` | 판정 약어(EPG/LGR…) 설명 | §10-13 | 없음. **native `title=` 금지** |
| `sonner` | 제출·토큰 발급 토스트 | §13, §14 | 적용 — shadcn `sonner.tsx` 는 지우고 `shared/providers/app-toaster.tsx` 가 오른쪽 아래·반경 0·왼쪽 4px 선으로 대체 |
| `sheet` | 모바일 사이드바 | §8-1 | 없음(288px) |
| `select` | 정렬·모드 선택 | §11-F | 없음 |
| `tabs` | `/settings`, 신호/메트릭 탭 | §11-D | 없음 |
| `empty` | State Triad 의 empty | §10-10 | 없음 |
| `item` | Stat Tile 기반 | §10-2 | 적용 — `border-0` |

### 4-2. W2 완료 항목

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
| `textarea` | 설정 blob 편집(읽기 전용 뷰 포함) | §11-D |
| `breadcrumb` | 표 > 폴더 > 차트 | §10-13 |
| `kbd` | 가이드의 CLI 예시 키 표기 | §10-13 |
| `field` · `button-group` | 폼 구성 보조 | §10-9 |
| `chart` | Series Chart 래퍼(Recharts) | §10-4 |

### 4-3. 설치하지 않음

`accordion`(Collapsible 로 충분) · `command` · `resizable` · `spinner`(로딩은 스켈레톤) · `avatar`(아바타 없음) · `calendar`(기간은 프리셋 + 입력)은 현재 구성에 포함하지 않습니다.

---

## 5. 도메인 시맨틱 매핑

design.md §10-5 는 상태 배지의 **시맨틱 색 시스템**을 규정한다. BMS 도메인 값을 여기에 배정한다.

### 5-1. ClearLamp(11종) → 스킨 색 토큰 (2026-10-14 변경, T3)

램프는 배지가 아니라 "견본 + 이름"(`features/lamp/lamp-label.tsx`), 행 왼쪽 5px 띠(`features/lamp/lamp-strip.tsx`), 누적 막대(`features/lamp/lamp-stack-bar.tsx`)로 그린다. 색은 기본 스킨 `theme.lua` 값이며 `shared/constants/clear-lamp.ts` 의 `LAMP_BACKGROUND` 가 키 → `var(--rbms-lamp-…)` 를 준다. 글자는 항상 `text-foreground`.

| 램프 | 표시 이름 | 토큰 | 값 |
|---|---|---|---|
| `NoPlay` | NO PLAY | `--rbms-lamp-noplay` | `#444444` |
| `Failed` | FAILED | `--rbms-lamp-failed` | `#ff0000` |
| `AssistEasy` | ASSIST | `--rbms-lamp-assist` | `#ff63fd` |
| `LightAssistEasy` | L-ASSIST | `--rbms-lamp-light-assist` | `#6563ff` |
| `Easy` | EASY | `--rbms-lamp-easy` | `#00ff24` |
| `Normal` | CLEAR | `--rbms-lamp-normal` | `#ffbb00` |
| `Hard` | HARD | `--rbms-lamp-hard` | `#dfdfdf` |
| `ExHard` | EX HARD | `--rbms-lamp-exhard` | `#ff6e6e` |
| `FullCombo` | FULL COMBO | `--rbms-lamp-fullcombo` | `#00dfff` |
| `Perfect` | PERFECT | `--rbms-lamp-perfect` | `#f499f2` |
| `Max` | MAX | `--rbms-lamp-max` | `#eaf300` |

판정(`--rbms-judge-pgreat|great|good|bad|poor`), FAST/SLOW(`--rbms-timing-fast|slow`), 난이도 0~5(`--rbms-difficulty-unknown|beginner|normal|hyper|another|insane`)도 같은 출처의 값으로 `:root` 에 있다. 랭크는 `shared/lib/dj-rank.ts` 의 `getDjRank(ex, maxEx)`(앱 27단계와 같은 이름)와 `getDjRankClassName`(색 없이 굵기·명도)으로 쓴다.

### 5-2. ranked / flags

| 값 | 표현 |
|---|---|
| `ranked: true` | 배지 없음(기본) |
| `ranked: false` | outline 배지 `unranked` + flags 를 muted 배지로 나열(`AUTOPLAY` `ASSIST` `JUDGE_WIDTH` `TOTAL_OVERRIDE` `UNKNOWN_BUILD` `GUEST`) |
| `verified: true` | `--color-chart-2` 배지 `verified` |

`unranked` 행은 랭킹 테이블에 나타나지 않는다(서버가 제외). 플레이어 기록 목록에서는 보이며 **행 전체를 muted 처리**한다.

### 5-3. 숫자 표기

- EX·BP·콤보·노트수·순위·µs 값은 전부 **`tabular-nums`** + 우측 정렬(§10-3).
- 해시(md5/sha256)·빌드 sha256 은 **mono**(design.md §3). 목록에서는 앞 8자 + 말줄임, 툴팁에 전문.
- 시각은 서버가 epoch ms(UTC) 로 준다. 표시 계층에서만 dayjs 로 로컬 변환(§common.md §9).

---

## 6. 필수 제약 (design.md §17-1 마스터 프롬프트 요약 — 위반 시 리뷰 반려)

1. `border-radius: 0` (Surface A). 파생 radius 4단계 전부 `var(--radius)` 에 핀. `calc(var(--radius) - Npx)` 금지.
2. 표면에 border·shadow 없음. 블록 분리는 `--color-background` 위 **1px gap** 하나로만. gap 은 부모가 단독 생성(자식이 `pt-px` 를 더하면 1px 틀어짐).
3. 폰트 두께는 **400 / 500 / 600** 셋뿐. `font-bold`(700)·`font-extrabold`(800) 금지(T7).
4. 색은 `var(--color-*)` 와 도메인 색 `var(--rbms-*)` 만(T11). hex·rgb·hsl·Tailwind 팔레트 유틸리티(`neutral-800`, `black/50`) 금지.
5. 콘텐츠 인셋 12px 양축. 데이터 화면(목록·난이도표·검색·순위)은 카드 없이 본문에 가득 채우고, Panel Card·`DashboardPanel` 은 대시보드 패널과 상세 블록에만 쓴다(T5).
6. 모션: opacity 0.18s, 바 폭 0.24s, `cubic-bezier(0.4,0,0.2,1)`. 그 외 무애니메이션. **차트는 애니메이션하지 않는다.**
7. 로딩은 스켈레톤. 버튼의 "로딩"은 disabled.
8. `.dark` 와 `[data-theme='dark']` 둘 다 지원.
9. **이모지 금지.** 아이콘은 lucide 만.
10. 동적 className 은 `cn()` 합성(템플릿 리터럴 금지). 툴팁은 shadcn `Tooltip`(native `title` 금지).

### 6-1. 자체 검사 grep (컴포넌트 작성 후 매번)

```
/#[0-9a-fA-F]{3,8}|rgb\(|hsl\(/          → 0건 (globals.css 팔레트 주석 제외)
/rounded-(?!none)|shadow-(?!none)|font-bold|font-extrabold/ → 0건 (shadcn `shared/ui` 생성물 제외)
```

---

## 7. 접근성 — 구현된 기준과 경계

design.md §7-4의 랜드마크·텍스트 이중 인코딩은 현재 컴포넌트 계약입니다. 리플레이 이벤트의 세부 키보드 탐색만 알려진 경계로 남습니다.

- Panel Card 제목을 `div` 가 아니라 **`h2`** 로 렌더 → 30여 화면에 문서 개요 제공. 시각 결과는 동일.
- 콘텐츠 컬럼에 **`<main>`**, 좌측 레일에 **`<nav>`**, 우측 필터 패널에 `<aside>` 를 붙여 랜드마크를 채운다.
- 램프·ranked 는 색 + 텍스트 이중 인코딩(§5-1).
- 리플레이 워터폴은 `role="img"` + `aria-label` 요약. 세부 이벤트 키보드 내비게이션은 아직 지원하지 않습니다.

## 8. 1차 골격의 공통 컴포넌트 (2026-10-14, 웨이브 17B-1)

다음 웨이브의 난이도표·폴더 화면은 아래 계약 위에 올린다. 상세 props 는 각 파일의 타입이 정본이다.

| 이름 | 경로 | 용도 |
|---|---|---|
| `AppShell` | `widgets/shell/app-shell.tsx` | 루트 레이아웃이 감싸는 2열 셸, 사이드바 슬롯 제공 |
| `ShellPage` | `widgets/shell/shell-page.tsx` | 페이지 틀: `section.rbms-page` + `ShellPageHeader` + `ScrollContainer` |
| `ShellPageHeader` | `widgets/shell/shell-page-header.tsx` | 48px 머리(`title`, `description?`, `actions?`) |
| `ShellPageLoading` | `widgets/shell/shell-page-loading.tsx` | 제목이 정해진 로딩 틀 |
| `ShellSidebarPortal` | `widgets/shell/shell-sidebar-portal.tsx` | 사이드바 메뉴 아래 슬롯에 패널을 꽂음 |
| `ScrollContainer` | `shared/ui/scroll-container.tsx` | 데스크톱 내부 스크롤 + 3px 표시 막대 |
| `DashboardPanel` 과 `-Empty`·`-Error`·`-Loading` | `features/dashboard/` | 36px 머리 패널과 패널 안 상태 |
| `EmptyState`, `LoadErrorState`, `NotFoundNotice`, `RowListSkeleton`, `PanelSkeleton`, `TableSkeleton` | `features/state/` | 화면·구역 단위 빈·오류·404·로딩 |
| `FilterBar`, `FilterField` | `features/filter/` | 머리 아래 필터 줄 |
| `FormDialog`, `ConfirmAction` | `features/dialog/` | 폼 대화상자(`size: 'md' \| 'lg'`), 확인 대화상자 |
| `LampSwatch`, `LampLabel`, `LampStrip`, `LampStackBar`, `LampDistribution` | `features/lamp/` | 램프 표현 |
| `ScoreRow` | `features/score/score-row.tsx` | 램프 띠가 있는 2줄 기록 행 |
| `SettingsSection` | `features/shell/settings-section.tsx` | 설정 구역 틀 |
| `CopyButton` | `shared/ui/copy-button.tsx` | 복사 + 토스트 |
| `buildPageMetadata` | `shared/lib/metadata.ts` | 페이지 metadata(`index: false` 로 noindex) |
| `fetchQueryOrNotFound` | `shared/lib/fetch-query-or-not-found.ts` | 서버 프리페치 404 → `notFound()` |


## 9. 발광 표·폴더 화면의 컴포넌트 (2026-10-14, 웨이브 17B-2)

목록과 props 계약의 정본은 `docs/web/2026-10-14-foundation-contract.md` 13.2절이다. 여기에는 다른 화면이 다시 쓸 만한 것과 디자인 규칙만 적는다.

| 이름 | 경로 | 용도 |
|---|---|---|
| `AddToFolderMenu` | `widgets/folder/add-to-folder-menu.tsx` | 곡 하나를 내 폴더에 넣고 빼는 메뉴(차트 머리, 표 상세). `level` 은 기호를 붙여(`★16`) 넘김 |
| `TableProgressPanel` | `features/table/table-progress-panel.tsx` | 접히는 진행 요약(기본 접힘). 펼치면 `LampDistribution` + 레벨별 막대(링크 아님) |
| `TableLevelSection` | `features/table/table-level-section.tsx` | `.rbms-level-header` 를 쓰는 접히는 묶음 |
| `TableChartGrid` / `TableChartCard` | `features/table/*` | 1px 틈 격자, 5px 램프 띠 카드. 묶음 안에서는 레벨을 카드에 다시 쓰지 않음 |
| `TableLevelJump` | `features/table/table-level-jump.tsx` | 레벨 칩 한 줄(모바일 가로 스크롤) |
| `useIsHydrated` | `shared/hooks/use-is-hydrated.ts` | 서버 HTML 과 다른 값을 하이드레이션 뒤에만 쓰게 함(사이드바 슬롯, 세션) |
| `useDialogFocusReturn` | `features/dialog/use-dialog-focus-return.ts` | 상태로 여는 대화상자의 포커스 복귀 |

- 같은 정보를 한 화면에 두 번 두지 않는다(17B-2 리뷰: 발광 표의 "내 진행" `h2` 와 "표 주소 복사" 가 둘씩이었음). 데스크톱 사이드바는 "표 정보"(갱신·곡·레벨 수)와 복사, 본문은 진행, 모바일 머리의 복사는 `md:hidden`. 출처 표기는 2026-10-15 에 지웠다(ea34002).
- 라이트 테마의 작은 회색 글자는 `text-muted-foreground` 로 4.5:1 을 넘는다(토큰 `oklch(0.48 0 0)`). 새 회색 단계를 만들지 않는다.
- 행 전체를 누르게 할 때는 제목 링크에 `after:absolute after:inset-0` 과 행의 `relative` 를 쓴다(`ScoreRow`, `FolderListItem`, `TableSnapshotCard`). 홈 "최근 활동" 행도 이미 이 방식이라 행 오른쪽 끝을 눌러도 링크가 받는다(17B-2 에서 `elementFromPoint` 로 확인).
