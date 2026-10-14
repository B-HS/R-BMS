# R-BMS 웹 디자인 기준 (웨이브 17A · 작업 A2)

> 최종 갱신 2026-10-14 · 대응 커밋 7095ef7

> 현재 상태: 4.1절의 1차(골격·토큰·난이도표·폴더)는 웨이브 17B-1·17B-2 에서 구현됐고(d38b6fb, 711b518), 6절의 결정 10건은 모두 A안으로 채택됐습니다([../acknowledge/2026-10-14-web-decisions.md](../acknowledge/2026-10-14-web-decisions.md) M7, 현재 디자인 정본은 [../acknowledge/design.md](../acknowledge/design.md)). 4.2절의 2차 가운데 메타(`openGraph` 속성, `robots.ts`, `sitemap.ts`, `icon.svg`)와 난이도표 상세의 "폴더에 추가"는 끝났고, 나머지는 미착수입니다: 홈 대시보드 재구성, 차트·플레이어 페이지 재구성, 설정 구역화, 검색 필터의 머리 + 대화상자, 난이도표 한 곡 한 행 보기·카드 높이 선택, 폴더 끌어 옮기기·가상화, OG 이미지, Inter 글꼴 검토([tasks.md](./tasks.md) "남은 일"). 2.4절·5절이 말하는 운영 사이트의 결함(시험 계정, 영문 404, 제목 `rbms IR`)은 해소됐습니다([2026-10-14-audit.md](./2026-10-14-audit.md) 해소 현황). 아래 본문은 설계 당시의 기록이며, `web:` 로 적은 경로 중 `features/badge/lamp-badge`·`widgets/shell/nav-rail`·`widgets/shell/filter-panel` 은 지금 없습니다([components.md](./components.md) 3절에 대응 경로).
> 이 문서는 다음 웨이브가 원본을 다시 읽지 않고 구현할 수 있도록 쓴 설계 기준이었습니다.
> 작성일 2026-10-14. 이 문서는 새 문서이며 `docs/acknowledge/design.md`, `docs/web/components.md` 를 고치지 않았습니다. 두 문서와 달라지는 곳은 3.1절 표에 모았고, 채택 여부는 6절의 결정으로 남깁니다.
> 근거 표기: `iidx:` 는 `/Users/hyunseokbyun/development/iidx-rank/src/` 아래, `web:` 은 `/Users/hyunseokbyun/development/R-BMS/web/src/` 아래 경로입니다. 뒤의 숫자는 줄 번호입니다.

## 0. 요약

1. 두 사이트의 기본 토큰은 같습니다. `iidx:app/globals.css` 7-118 과 `web:app/globals.css` 7-137 의 팔레트·의미 토큰·반경 0·그림자·모션·치수 값이 같은 원본(`iidx-rank/docs/DESIGN.md`, flunti-otel 디자인 명세)에서 왔습니다. 그러므로 R-BMS 웹이 부족해 보이는 까닭은 토큰이 아니라 그 위에 쌓은 것입니다: 골격, 도메인 표현, 상태 마감, 모바일 스크롤 모델.
2. iidx-rank 가 좋아 보이는 핵심은 다섯 가지입니다. 콘텐츠가 가장자리까지 차는 2열 골격(여백 0, 48px 머리 하나), 치수 몇 개의 반복(12px, 48·36·24px, 1px 틈), 크기 대신 굵기와 명도로 만든 위계(11·12·14px), 색을 램프·난이도에만 쓰는 절제, 화면마다 빠짐없는 로딩·빈·오류 상태.
3. R-BMS 웹은 1차에서 (a) 골격을 iidx-rank 의 `AppShell` 방식으로 바꾸고, (b) 램프·판정·난이도 색을 앱 스킨(`assets/skins/rbms-default/lua/theme.lua`) 값으로 맞추고, (c) 발광 난이도표를 레벨별 카드 격자로, (d) 내 폴더를 목록과 2단 편집 화면으로 만듭니다. 나머지 화면은 1차에서 새 머리만 얹고 2차에서 다시 짭니다.
4. 운영 사이트(https://bms.hyuns.uk)는 지금 시험 계정과 제목 없는 차트가 그대로 보이고, 모든 페이지 제목이 `rbms IR` 이며, 404 가 Next 기본 영문 화면입니다(2.4절). 디자인보다 먼저 눈에 띄는 결함입니다.

## 1. iidx-rank 읽기

### 1.1 기술 스택과 구조

| 항목 | 값 | 근거 |
|---|---|---|
| 프레임워크 | Next.js 16.3.8 App Router, `cacheComponents: true`, `reactCompiler: true`, `experimental.globalNotFound: true` | `iidx-rank/package.json`, `next.config.ts` |
| UI | React 19.2.8, Tailwind CSS 4, shadcn `radix-nova` 스타일(neutral, CSS 변수, lucide), `radix-ui` 1.6.7, `cmdk`, `sonner`, `next-themes` | `package.json`, `components.json` |
| 데이터 | TanStack Query 5, Drizzle + libSQL(Turso), better-auth | `package.json` |
| 국제화 | next-intl 4(ko·ja·en), 경로 `app/[locale]/(shell)/…` | `iidx:app/[locale]/layout.tsx` |
| 레이어 | `app` → `widgets` → `features` → `entities` → `shared`. 슬라이스마다 폴더(`features/chart-card-grid/chart-card.tsx`), 1파일 1컴포넌트 | `iidx:` 트리 |
| 디자인 문서 | `docs/DESIGN.md`(3,155줄, 원본 명세), 결정은 `docs/acknowledge/`, 지적은 `docs/feedback/`, 감사는 `docs/quality-assurance/2026-10-06-ui-ux-audit.md` | |

R-BMS 웹(`web/package.json`)은 Next 16.3.4, 같은 React·Tailwind·shadcn 스타일·TanStack Query·better-auth 이고 DB 만 MySQL 입니다. `cmdk`, `next-intl`, `@tailwindcss/typography` 는 없습니다. 구조가 같아 코드 조각을 거의 그대로 옮길 수 있습니다(3.5절).

### 1.2 전역 토큰

`iidx:app/globals.css` 에서 읽은 값입니다. R-BMS 웹과 같은 값은 "같음"으로 적었습니다.

| 분류 | 토큰 | 값 | R-BMS 웹 |
|---|---|---|---|
| 깊이(라이트) | `--sidebar` / `--background` / `--card` | `oklch(0.915 0 0)` / `0.955` / `1` (49-51) | 같음 |
| 깊이(다크) | 같은 세 토큰 | `0.098` / `0.162` / `0.212` (122-124) | 같음 |
| 글자 | `--foreground`, `--muted-foreground` | 라이트 `0.145`, `0.556` · 다크 `0.985`, `0.708` | 같음 |
| 선 | `--border` | 라이트 `0.922` · 다크 흰색 10% (67, 140) | 같음 |
| 강조 | `--primary` | 라이트 `0.205` · 다크 `0.922` | 같음 |
| 상태 | `--destructive`, `--warning` | 빨강 `oklch(0.577 0.245 27.325)` / 다크 `0.704 0.191 22.216`, 황색 `0.705 0.153 70` / 다크 `0.79 0.145 75` | 같음 |
| 글꼴 | `--font-sans`, `--font-mono` | 시스템 글꼴 묶음(83-85). 웹 폰트 없음 | 같음 |
| 반경 | `--radius` 와 파생 4단 | 전부 `0rem` (86, 198-201) | 앱 Surface 는 같음, 공개 Surface 만 `0.375rem` |
| 그림자 | `--shadow-2xs`…`--shadow` | `none`. `md` 이상만 값(88-95). 오버레이 전용 | 앱 Surface 는 같음 |
| 간격 | `--spacing` | `0.25rem`. 패널 안쪽 `--panel-inset: 12px`, 틈 `--panel-gutter: 1px` (87, 106-107) | 같음 |
| 모션 | ease / fade / bar | `cubic-bezier(0.4, 0, 0.2, 1)` / `0.18s` / `0.24s` (96-98) | 같음 |
| 치수 | 사이드바 / 접힘 / 모바일 / 머리 / 메뉴 항목 | `16rem` / `3rem` / `18rem` / `3rem` / `2.25rem` (99-105) | 같음 |
| 글자 크기 | `--text-2xs` | `0.6875rem`(11px) (202) | 같음 |
| z-index | rail / raised / rank-header / scroll-indicator / overlay | 10 / 20 / 30 / 40 / 50 (113-117) | 10 / 20 / 50 만 있음 |

iidx-rank 에만 있는 것은 도메인 색과 그것을 쓰는 클래스입니다.

| 토큰·클래스 | 값 | 줄 |
|---|---|---|
| `--iidx-hyper` / `--iidx-another` / `--iidx-leggendaria` | `#ffff00` / `#ff0000` / `#b000ff` | 292-294 |
| `--iidx-easy` / `--iidx-clear` / `--iidx-hard` / `--iidx-ex-hard` | `#00ff00` / `#ffff00` / `#ffffff` / `#ff0000` | 295-298 |
| `--iidx-lamp-off` / `--iidx-lamp-dark` | `#737373` / `#000000` | 299-300 |
| `.checker-chart-card` | 사방 1px `--border`, 왼쪽만 2px 난이도 색 | 304-308 |
| `.checker-lamp` | 꺼진 색 배경, 오른쪽에 `1px` 구분선 `color-mix(in oklch, var(--foreground) 45%, transparent)`. 값이 있으면 `steps(1, end)` 로 두 색 점멸 | 321-362 |
| 모션 감소 시 램프 | 점멸을 끄고 두 색을 위아래 반씩 나눈 정적 표시 | 363-370 |
| `.checker-micro-label` | 11px, 굵기 500, 자간 `0.025em`, 대문자, muted 색 | 255-261 |
| `.checker-toolbar`, `.shell-page-header` | `sticky top-0 z-(--z-raised) flex h-12 … border-b bg-background px-3 md:static` | 267-270 |
| `.checker-rank-header` | `sticky top-(--rail-chrome-height) … md:top-0` | 272-274 |
| `.checker-list-scroll` | 모바일은 문서 스크롤, `md` 이상만 내부 스크롤. 아래 여백 `9rem` | 276-281 |

게임 원색은 테마와 무관한 16진수로 한 곳(`:root`)에만 두고, 글자에는 쓰지 않으며, 띠와 견본에만 칠합니다. 흰 램프가 흰 카드에서 사라지지 않도록 구분선(323)과 견본 테두리(`iidx:features/lamp-swatch/lamp-swatch.tsx` 14, `border border-foreground/40`)를 둡니다.

### 1.3 골격

`iidx:widgets/app-shell/app-shell.tsx` 212-278 이 전부입니다.

| 부분 | 구성 | 값 |
|---|---|---|
| 뼈대 | shadcn `Sidebar collapsible='icon'` + `SidebarInset`. 2열. 우측 패널 없음 | 폭 256px, 접으면 48px, 모바일 시트 288px (`iidx:shared/constants/ui.ts` 2-4) |
| 사이드바 머리 | 접기 버튼(`size-8`, 왼쪽 여백 8px) + 사이트 이름(`text-sm font-semibold tracking-tight`) | 높이 48px (229-236) |
| 메뉴 | 항목 높이 36px, 반경 0, 좌우 12px, 항목 사이 틈 0. 켜진 항목이 사이드바 폭을 가득 채움 | `iidx:widgets/app-shell/shell-nav-menu.tsx` 8-9, 13 |
| 페이지 슬롯 | 메뉴 바로 아래 빈 `div`. 페이지가 포털로 자기 패널을 꽂음. 비면 숨김 | 242-245, `shell-sidebar-portal.tsx` |
| 보조 링크 | 본문 맨 아래 11px 링크(개인정보처리방침) | 246-250 |
| 사이드바 바닥 | 한 줄 48px. 테마 버튼(48px 정사각) · 언어 · 계정을 `grid-cols-[auto_auto_minmax(0,1fr)]` 로 | 252-265 |
| 본문 | `bg-background` 위 세로 flex. 바깥 여백 0 | 267-271 |
| 페이지 머리 | 높이 48px, 아래 선 1px, 좌우 12px. `h1`(`text-sm font-semibold`, 한 줄 말줄임) + 설명(`text-xs` muted, `sm` 미만에서 숨김) + 오른쪽 동작 묶음(`gap-1`) | `iidx:widgets/app-shell/shell-page-header.tsx` 8-17 |
| 스크롤 | `md` 이상은 화면 고정(`md:h-dvh md:overflow-hidden`)에 본문만 내부 스크롤. 모바일은 문서 스크롤 | `iidx:app/[locale]/layout.tsx` 35, `iidx:shared/ui/scroll-container.tsx` 7-10 |
| 스크롤 표시 | 네이티브 스크롤바는 전역으로 숨기고(`globals.css` 222-228), 스크롤 중에만 오른쪽에 3px 막대가 나타났다가 1초 뒤 사라짐 | `iidx:shared/ui/scroll-indicator.tsx` 21-27, `ui.ts` 16-18 |

컨테이너 폭 제한은 데이터 화면에 없습니다(폭 전체 사용). 문서형 화면만 `max-w-3xl`(48rem, `iidx:widgets/privacy-policy/privacy-policy.tsx` 78), 폼은 `max-w-xl`(36rem, `iidx:widgets/profile-settings/profile-settings-form.tsx` 100)이고 가운데가 아니라 왼쪽에 붙습니다.

사용자가 직접 고친 이력이 이 골격의 근거입니다. 처음 구현은 3열과 12px 바깥 여백, 한 곡 한 행이었고, 사용자가 "좌측 사이드바와 콘텐츠 패딩을 제거하고, 우측 패널을 좌측으로 통합하며, 한 곡 한 행의 표를 모바일 최소 3열 카드로" 바꾸게 했습니다(`iidx-rank/docs/feedback/2026-10-05-checker-density.md`). 또 "사이드바는 이동 수단이고 목록 같은 콘텐츠는 페이지에 둔다", "특정 메뉴에 딸린 보조 패널이 있으면 그 메뉴를 패널 바로 위(메뉴의 마지막)에 둔다"는 규칙이 지적으로 남아 있습니다(`docs/feedback/2026-10-06-sidebar-user-list.md`).

### 1.4 타이포그래피 단계

실제로 쓰는 단계는 적습니다.

| 역할 | 클래스 | 근거 |
|---|---|---|
| 페이지 제목(`h1`) | `text-sm font-semibold`(14px, 600) | `shell-page-header.tsx` 12 |
| 패널·구역 제목(`h2`) | `text-sm font-semibold` | `iidx:features/dashboard-panel/dashboard-panel.tsx` 19, `chart-rank-section.tsx` 45 |
| 작은 머리말(`h3`) | `.checker-micro-label`(11px, 500, 대문자) | `bar-count-list.tsx` 16 |
| 목록 행의 주 글자 | `text-sm font-medium`, 보조 `text-xs` muted | `iidx:widgets/user-list/user-list.tsx` 49-52 |
| 본문·설명 | `text-xs`(12px) | 전반 |
| 카드 곡명 | `text-2xs leading-tight font-medium`, 2줄 말줄임 | `chart-card.tsx` 151 |
| 큰 숫자 | `text-2xl leading-8 font-semibold tracking-tight tabular-nums`, 버금 `text-xl leading-7` | `catalog-source-summary.tsx` 24, `lamp-distribution.tsx` 25 |
| 프로필 이름 | `text-lg leading-tight font-semibold` | `profile-header.tsx` 23 |
| 문서 본문 | `text-sm leading-6 break-keep`, 절 제목 `text-base font-semibold tracking-tight` | `privacy-policy.tsx` 78, `privacy-section.tsx` 10 |

굵기는 400·500·600 만 씁니다(`DESIGN.md` 5-4 는 700 을 금지). 숫자는 항상 `tabular-nums`, 값이 0 이면 muted 색으로 내립니다(`bar-count-list.tsx` 31, `lamp-distribution.tsx` 53).

### 1.5 패턴 목록

| 패턴 | 만든 방식 | 근거 |
|---|---|---|
| 목록 | `ul` 에 행마다 `border-b border-border px-3 py-2 hover:bg-muted/50`. 제목 링크에 `after:absolute after:inset-0` 을 줘 행 전체가 눌림. 행 안의 다른 버튼은 `pointer-events` 로 살림 | `user-list.tsx` 44-49, `board-post-list-item.tsx` 20-36 |
| 카드 격자 | `grid gap-0 bg-card`, 카드마다 1px 테두리. 마지막 행 빈칸을 같은 높이의 빈 카드로 채워 격자가 끊기지 않게 함 | `chart-card-grid.tsx` 15-35, `checker.ts` 3, 13-19 |
| 대시보드 패널 | `section` + 36px 머리(제목·배지·동작) + 본문(`px-3 pb-3`). 패널 사이는 부모의 `gap-px bg-border` | `dashboard-panel.tsx` 16-26, `dashboard.ts` 27-28 |
| 배지 | shadcn `Badge variant='secondary'` 에 `text-2xs`. 개수 표기에만 씀 | `chart-rank-section.tsx` 49 |
| 필터 | 머리의 아이콘 버튼 → 대화상자(`sm:max-w-md`). 적용 개수를 버튼 모서리 14px 사각 배지로 표시. 라벨은 micro-label, 입력은 `Select`·`Checkbox`. 아래에 "n / 전체" 와 초기화 버튼 | `checker-workspace.tsx` 401-417, 471-481, `checker-filters.tsx` 53-150 |
| 검색 | 필터 대화상자 안의 `Input type='search'`. 입력 즉시 클라이언트에서 거름 | `checker-filters.tsx` 59-65, `checker-workspace.tsx` 94-122 |
| 전환 버튼 | `ToggleGroup type='single' variant='outline' size='sm' spacing={0}` | `checker-mode-toggle.tsx` 19-28 |
| 탭 | shadcn `Tabs`, 탭 줄은 `overflow-x-auto border-b px-3 py-2`, 선택은 주소 `?tab=` | `iidx:widgets/user-page/user-page.tsx` 91-95 |
| 대화상자 | `DialogContent` 에 `flex max-h-[calc(100dvh-2rem)] flex-col gap-0 overflow-hidden p-0 sm:max-w-md`, 안쪽은 `ScrollContainer variant='always'` 에 `p-4 gap-4` | `checker-workspace.tsx` 472-473 |
| 토스트 | 오른쪽 아래. 반경 0, 그림자 없음, 1px 테두리, 종류별 왼쪽 4px 선(성공 primary, 오류 destructive, 경고 warning, 안내 ring). 테마는 `resolvedTheme` | `iidx:shared/providers/app-toaster.tsx` 8-32 |
| 빈 상태 | shadcn `Empty`(`min-h-64 border-0`) + 제목 + 설명 + `outline sm` 버튼 하나. 패널 안에서는 12px 한 줄 문구(`min-h-16`) | `checker-workspace.tsx` 337-348, `dashboard-panel-empty.tsx` 8-9 |
| 오류 | 패널: 경고 아이콘(유일하게 색을 쓰는 곳) + 문구(`role='alert'`) + 재시도 `outline xs`. 화면: 셸 안 `error.tsx` 에 `Empty` + 재시도 | `dashboard-panel-error.tsx` 16-23, `iidx:app/[locale]/(shell)/error.tsx` |
| 로딩 | 스피너 없음. 내용과 같은 모양의 스켈레톤. 난이도표는 랭크별 실제 곡 수만큼 카드 틀을 그림. `role='status'` + `aria-busy` + 숨김 문구 | `checker-loading.tsx` 19, 58-69, `chart-rank-skeleton.tsx` |
| 자리 고정 | 값이 늦게 와도 줄이 밀리지 않게 높이를 박아 둠(`h-8`, `h-4`, `h-20`) | `checker-workspace.tsx` 237, 241, 254, 278 |
| 진행률 | 문구 + 백분율 + `Progress`(`aria-valuenow`) | `checker-workspace.tsx` 283-291 |
| 넘치는 목록 | 패널 높이에 들어가는 행만 보이고 넘친 행은 `inert` | `fitted-row-list.tsx` 24-38 |

### 1.6 난이도표를 보여 주는 방식

`iidx:widgets/checker-workspace/checker-workspace.tsx` 와 `iidx:features/chart-rank-section`, `chart-card-grid` 입니다. `iidx-rank/docs/assets/screenshot.png`(1440×900, 라이트)로 모양을 확인했습니다.

| 항목 | 값 |
|---|---|
| 묶음 | 랭크(S+ … F)별 `section`. 접을 수 있음(shadcn `Collapsible`, 기본 펼침). 묶음 안에서 다시 "지력 / 개인차" 소묶음 |
| 묶음 머리 | 높이 48px, `bg-muted`, 왼쪽에 화살표(접히면 -90도 회전)와 랭크 이름(`text-sm font-semibold`), 오른쪽에 곡 수 배지. 머리 전체가 버튼. 스크롤하면 위에 붙음(모바일 `top: 48px`, 데스크톱 `top: 0`) |
| 소묶음 머리 | 높이 24px, 11px muted, 이름 + 개수 |
| 열 수 | 3 / `sm` 4 / `lg` 5 / `xl` 6 / `2xl` 7 (`checker.ts` 3). 모바일에서도 최소 3열 |
| 카드 | 높이 40px(로고 모드) 또는 56px(이름 모드). 왼쪽 2px 난이도 색 선, 그 안쪽에 5px 램프 띠, 곡명 11px 2줄, 오른쪽 아래 DJ LEVEL(`font-semibold tabular-nums`). 배경에 시리즈 로고를 불투명도 10% 로 |
| 정렬 | 묶음 안에서 곡명 순(`localeCompare(…, 'ja')`, 122). 정렬 선택지는 없음 |
| 색 부호화 | 램프(띠), 난이도(왼쪽 선) 두 가지만. 나머지는 무채색 |
| 조작 | 누르면 램프가 한 단계 오름. 길게 누르기(550ms)·오른쪽 클릭·Shift+Enter 는 상세 대화상자. 터치 기기에는 한 줄 안내를 한 번 보여 줌 |
| 머리 동작 | 게이지 전환, 새로 고침(로그인), 결과 수(`sm` 이상), 필터, 표시 설정 |
| 사이드바 패널 | "원본 목록"(상태 점, 게시일, 수집 시각, 곡 수, 원본 보기 버튼) + "내 진행"(기록 수, 백분율, 진행 막대) |
| 모바일 | 같은 구조. 문서 스크롤, 머리 두 단이 차례로 붙음, 결과 수는 머리 아래 32px 줄로 내려감(431-437), 사이드바 패널은 시트 안 |
| 접근성 | 묶음은 `section aria-label`, 격자는 `ul aria-label`, 카드 버튼의 `aria-label` 에 곡명·난이도·램프·등급 |

### 1.7 상호작용·테마·반응형·접근성·데이터

| 주제 | 값 | 근거 |
|---|---|---|
| 호버 | 목록 행 `hover:bg-muted/50`, 링크 `hover:underline`. 크기·그림자 변화 없음 | `user-list.tsx` 44 |
| 포커스 | `outline-none` 은 반드시 고리와 함께. 일반 `focus-visible:ring-[3px] ring-ring/50`, 가득 찬 칸은 `ring-2 ring-inset ring-ring` | `chart-card.tsx` 112, `chart-rank-section.tsx` 43 |
| 터치 | 28px 아이콘 버튼에 `pointer-coarse:after:-inset-2` 로 눌리는 영역만 44px 로 넓힘 | `iidx:shared/ui/button.tsx` 7, 28-30 |
| 키보드 | 사이드바 접기 Ctrl/Cmd+B, 묶음 접기 Enter·Space, 카드 Shift+Enter(`aria-keyshortcuts`) | `iidx:shared/ui/sidebar.tsx` 21, 73 |
| 전환 | 화살표 회전(`transition-transform`), 스크롤 막대 투명도 200ms. 경로 전환·차트 애니메이션 없음 | |
| 모션 감소 | 전역으로 애니메이션·전환을 0.01ms 로 | `globals.css` 230-239 |
| 테마 | `next-themes`, `attribute='class'`, 기본 라이트, 시스템 추종 끔, `disableTransitionOnChange`. 버튼은 `dark:` 클래스로 해와 달을 바꿔 하이드레이션 불일치 없음 | `iidx:shared/providers/app-providers.tsx` 25, `app-shell.tsx` 70-71 |
| 기준점 | `md`(768px)에서 사이드바가 시트로, 스크롤 모델이 바뀜. 격자는 `sm` 640 · `lg` 1024 · `xl` 1280 · `2xl` 1536. 대시보드는 컨테이너 쿼리 45rem(2열) · 62rem(3열) · 96rem(4열) | `use-mobile.ts` 5, `dashboard.ts` 27-28 |
| 랜드마크 | `nav aria-label`, `SidebarInset` 이 `main`, 페이지마다 `h1` 하나, 패널은 `section aria-labelledby` | `app-shell.tsx` 239, `dashboard-panel.tsx` 16 |
| 색+글자 | 램프는 띠와 함께 `aria-label` 과 범례 글자. 막대 그래프는 `role='img'` + 값 나열 | `lamp-distribution.tsx` 17, 34 |
| 데이터 | 서버 컴포넌트가 `queryOptions` 로 프리페치 → `HydrationBoundary`. 공개 목록은 정적 셸, 세션·기록은 `Suspense` 안. 같은 스켈레톤을 `loading.tsx` 와 `Suspense fallback` 이 함께 씀 | `iidx:app/[locale]/(shell)/table/page.tsx` 30-79 |
| 메타 | 페이지별 제목(`title.template` 으로 "페이지 이름 + 사이트 이름"), canonical, openGraph, JSON-LD, `robots.ts`, `sitemap.ts`, 실제 404 상태의 전역 not-found | `iidx:app/[locale]/layout.tsx` 21-27, `iidx:app/global-not-found.tsx` |

### 1.8 좋아 보이게 하는 원리

1. 한 화면에 틀은 하나입니다. 48px 머리 하나와 사이드바 말고는 전부 콘텐츠이고, 콘텐츠는 가장자리까지 찹니다.
2. 치수가 몇 개뿐이고 계속 되풀이됩니다. 안쪽 여백 12px, 높이 48·36·24px, 선 1px. 눈이 격자를 한 번 배우면 끝입니다.
3. 위계는 크기가 아니라 굵기와 명도로 만듭니다. 11·12·14px 세 크기에 500·600 굵기, 보조 정보는 muted.
4. 선은 그리지 않고 틈으로 냅니다(`gap-px` 위 `bg-border`). 반경과 그림자가 0 이라 카드 격자가 표처럼 읽힙니다.
5. 색은 뜻이 있을 때만 나옵니다. 램프와 난이도는 게임의 원색, 그 밖은 무채색, 0 은 흐리게.
6. 숫자는 자릿수가 맞습니다(`tabular-nums`, 오른쪽 정렬, 단위와 라벨은 muted).
7. 도메인의 물건을 도메인의 방식으로 그립니다. 카드 왼쪽의 램프 띠는 게임 선곡 화면의 램프 그대로입니다.
8. 상태가 빠지지 않습니다. 패널마다 로딩(같은 모양)·빈·오류(재시도)가 따로 있고, 값이 늦게 와도 자리가 밀리지 않습니다.
9. 모바일은 축소판이 아닙니다. 스크롤 모델을 바꾸고(문서 스크롤 + 붙는 머리), 열 수를 지키고, 눌리는 영역을 넓힙니다.
10. 마감까지 사이트의 모양입니다. 토스트, 404, 오류 화면, 문서 제목, 스크롤 막대가 모두 같은 규칙을 따릅니다.

## 2. R-BMS 웹의 지금

### 2.1 토큰과 합의

`web:app/globals.css` 는 iidx-rank 와 같은 토큰에 공개 Surface 델타(183-209: 반경 `0.375rem`, `0 0 7px` 그림자, 배경 한 단)를 더한 것입니다. `docs/acknowledge/design.md` 는 이것을 "두 Surface, 두 토큰 세트"(1-2), "3열"(8-1), "panel 은 1px gap 으로 분리"(6-4), "하드코드 색 금지"(17-1)로 합의했습니다. 램프 색은 `docs/web/components.md` 5-1 이 `--chart-1`…`--chart-5` 에 배정했고, 스스로 "가정"이라고 적었습니다.

도메인 색 토큰은 없습니다. `web:features/badge/lamp-badge.tsx` 6-18 은 램프 11종을 차트 계열색 5개와 muted 로 칠하는데, 이 색은 앱의 램프 색과 무관합니다(라이트에서 MAX 가 주황, FULL COMBO 가 청록).

### 2.2 골격과 공통 컴포넌트

| 항목 | 지금 | 근거 |
|---|---|---|
| 뼈대 | 직접 만든 3열. `h-dvh` 고정에 `main` 이 내부 스크롤. 모바일도 같음 | `web:app/(app)/layout.tsx` 7-16 |
| 사이드바 | 직접 만든 `nav`(256px 고정, 접기 없음). 사이트 이름은 고정폭 글꼴 `rbms IR`. 바닥에 "로그인" 글자 링크와 테마 버튼 | `web:widgets/shell/nav-rail.tsx` 13-25 |
| shadcn `sidebar` | `web:shared/ui` 에 없음. git 이력에도 없음. `components.md` 4-1 은 "적용"이라고 적혀 있어 문서와 다름 | `git log -- web/src/shared/ui/sidebar.tsx` 결과 없음 |
| 메뉴 | 대시보드 · 차트 검색 · 리더보드 · 난이도표 · 설정 · 연동 가이드 | `web:shared/constants/navigation.ts` |
| 계정 표시 | 로그인해도 사이드바는 "로그인" 링크 그대로(세션을 읽지 않음) | `nav-rail.tsx` 21-23 |
| 페이지 머리 | 없음. 일부 화면만 `PageHeading`(`h1 text-lg`, 좌우 16px)을 본문 안에 둠. 홈·검색·순위에는 `h1` 이 없음 | `web:features/shell/page-heading.tsx` 4 |
| 본문 블록 | 전부 `PanelCard`(shadcn Card, 안쪽 12px). 화면은 카드의 세로 쌓기 | `web:features/shell/panel-card.tsx` |
| 표 | `DataTable`(12px, 칸 8px, 숫자 오른쪽 정렬). 모바일에서 열을 줄이는 수단 없음(가로 스크롤) | `web:features/table/data-table.tsx` |
| 상태 | `EmptyState`(아이콘·제목·설명, 동작 없음), `LoadErrorState`(재시도 버튼 없음), `PanelSkeleton`(높이 24·64·96 의 회색 덩어리 하나) | `web:features/state/*.tsx` |
| 토스트 | 오른쪽 위, shadcn 기본 모양 | `web:app/providers.tsx` 16 |
| 스크롤바 | 전역으로 숨김(287-296). 대신 보여 주는 표시가 없어 스크롤 위치를 알 수 없음 | `web:app/globals.css` |
| 테마 | `defaultTheme='system'`, `attribute={['class','data-theme']}` | `web:app/providers.tsx` 12 |
| 특수 파일 | `not-found.tsx`, `error.tsx`, `loading.tsx`, `robots.ts`, `sitemap.ts`, 아이콘 파일 모두 없음. `public/` 에는 Next 기본 svg 5개뿐 | `find web/src/app`, `ls web/public` |
| 문서 제목 | 모든 페이지가 `rbms IR` | `web:app/layout.tsx` 5-8 |
| 공개 화면 | 로그인·가입·가이드는 셸 밖. 둥근 카드와 그림자, 800 굵기 제목. 가이드에는 `font-bold` 가 한 곳 있음(자체 규칙 위반) | `web:app/(public)/login/page.tsx` 6, `web:widgets/guide/guide-article.tsx` 41 |

### 2.3 페이지 구성

| 경로 | 지금 | 부족한 점 |
|---|---|---|
| `/` | 숫자 타일 3개 + "최근 활동" 표 | 처음 온 사람에게 무엇을 하는 곳인지, 클라이언트를 어디서 받는지가 없음 |
| `/tables` | 표 하나(이름·폴더 수·차트 수·코스 수) | 운영은 "등록된 표가 없습니다" |
| `/tables/[tableId]` | 폴더마다 카드 하나, 안에 `sha256`·`md5` 앞 8자만 나오는 표 | 곡명·아티스트·레벨·링크·내 기록이 없음. 난이도표로 쓸 수 없음 (`web:widgets/table/table-detail-panel.tsx` 16-29) |
| `/charts/[hash]` | 제목 블록 + 타일 4개 + 랭킹 표 | 구성은 맞음. 머리·램프 색·내 기록 강조가 없음 |
| `/leaderboards`, `/search`, `/players/[id]` | 카드 + 표 | 구성은 맞음. 머리와 모바일 열 처리 없음 |
| `/settings` | 탭 4개 | 탭이 주소에 없음(`defaultValue='profile'`, `web:widgets/settings/settings-tabs.tsx` 15) |
| 내 폴더 | 없음 | 새로 만들어야 함 |

### 2.4 운영 사이트에서 본 것

2026-10-14 에 GET 으로 9개 경로와 스타일시트 1개를 한 번씩 받았습니다(모두 차례로, 총 10건).

| 경로 | 상태 | 관찰 |
|---|---|---|
| `/` | 200 | 타일 값 "등록 차트 5 · 플레이어 5 · 누적 제출 10". 최근 활동의 플레이어가 `e2e-phase-i-18d391944f88`, `probe1` 같은 시험 계정. 차트 칸의 링크에 글자가 없음(제목이 빈 차트). 레벨은 전부 "—" |
| `/tables` | 200 | "등록된 표가 없습니다" |
| `/leaderboards` | 200 | 1위가 `e2e-phase-i-18d38e33cabb`, 랭크 포인트 0.00 |
| `/settings` | 200 | 본문 대신 `/login?next=%2Fsettings` 로 보내는 스트림 리다이렉트(메타 refresh 1초 포함) |
| `/login` | 200 | 서버 HTML 에는 빈 카드 틀만 있음(폼은 `Suspense fallback={null}` 뒤 클라이언트에서 나타남) |
| `/guide` | 200 | 셸 없이 단독 문서 |
| `/no-such-page` | 404 | Next 기본 화면 "404: This page could not be found."(영문, 셸 없음) |
| 공통 | | `<title>` 이 전부 `rbms IR`, openGraph 태그 0개, 아이콘은 Next 기본 `favicon.ico` |

마크업은 코드와 같았습니다: `data-surface='app'`, `nav.w-64`, `main.overflow-y-auto`, 스켈레톤은 `h-96` 한 덩어리.

### 2.5 iidx-rank 와 견준 격차

| 관점 | iidx-rank | R-BMS 웹 | 메울 방법 |
|---|---|---|---|
| 위계 | 페이지마다 48px 머리에 `h1`·설명·동작 | 머리 없음. `h1` 이 없는 화면이 있고 있는 곳은 여백이 16px 로 다름 | `ShellPageHeader` 를 모든 페이지에 |
| 밀도 | 본문 여백 0, 카드 격자, 11px 곡명 | 카드 쌓기 + 표. 난이도표는 해시 8자 | 난이도표를 카드 격자로, 목록 화면은 가득 찬 행 목록으로 |
| 도메인 표현 | 램프 띠·난이도 선이 게임 색 | 램프가 무관한 계열색 배지 | 스킨 색 토큰 + 램프 띠·견본 |
| 일관성 | 한 Surface. 404·오류·토스트까지 같은 규칙 | 두 Surface(둥근 카드·그림자·800 굵기가 섞임), 기본 404, 기본 토스트 | Surface 통합, 특수 파일 추가 |
| 빈·오류 | 동작 버튼과 재시도가 항상 있음 | 문구만 있음 | `EmptyState` 에 동작, `LoadErrorState` 에 재시도 |
| 로딩 | 내용과 같은 모양 | 회색 덩어리 하나 | 화면별 스켈레톤 |
| 모바일 | 문서 스크롤 + 붙는 머리, 터치 영역 확대, 열 수 유지 | 모바일도 `h-dvh` 내부 스크롤(iidx-rank 가 Safari 문제로 고친 바로 그 구조, `iidx-rank/docs/history/2026-10-06-native-mobile-scroll-sticky-ranks.md`), 표는 가로 스크롤 | 스크롤 모델 교체, 열 숨김 |
| 스크롤 표시 | 3px 막대 | 없음(스크롤바만 숨김) | `ScrollContainer`·`ScrollIndicator` 이식 |
| 계정 | 사이드바 바닥에 이름·메뉴·로그아웃 | 항상 "로그인" 링크 | 계정 컨트롤 |
| 메타 | 페이지별 제목, OG, sitemap | 전부 같은 제목 | `title.template`, 페이지별 `metadata` |
| 내용 | 실제 데이터 | 시험 계정과 빈 제목이 그대로 노출 | 운영 데이터 정리(6절 결정 8) |

## 3. 설계

### 3.1 토큰

기조는 그대로 둡니다: 무채색 바탕, 반경 0, 그림자는 오버레이만, 1px 틈. 바꾸는 것은 네 가지입니다.

#### 3.1.1 기존 합의와 달라지는 곳

| 번호 | 지금(`docs/acknowledge/design.md`·`components.md`) | 제안 | 까닭 |
|---|---|---|---|
| T1 | 두 Surface, 공개 쪽은 반경 6px·그림자·배경 한 단 (1-2, 16-2) | Surface 하나. `[data-surface='public']` 델타(`globals.css` 183-209) 삭제, 로그인·가입·가이드를 셸 안으로 | 둥근 카드와 그림자는 앱의 평면·사각 정체성과 어긋나고, iidx-rank 는 Surface 하나로 일관됨. 공개 화면에서도 메뉴가 유지됨 |
| T2 | 기본 테마 `system` (4) | 기본 `dark`, 시스템 추종 끔, 토글로 라이트 선택, `disableTransitionOnChange` | 앱과 기본 스킨이 검정 바탕. 첫 화면이 앱과 이어져야 함. 저장 키 `rbms-theme` 는 유지 |
| T3 | 램프를 `--chart-1`…`--chart-5` 로 (components.md 5-1, "가정") | 램프·판정·난이도 색을 스킨 값 그대로 `--rbms-*` 토큰으로 | 웹의 램프 색이 게임의 램프 색과 같아야 "색이 뜻을 나른다". 지금 배정은 게임과 무관 |
| T4 | 3열(좌 256 / 본문 / 우 320), 머리 없음 (8-1) | 2열 + 페이지마다 48px 머리. 우측 패널은 폴더 편집 화면에서만 | iidx-rank 에서 사용자가 3열을 2열로 바꾸게 한 전례. 본문 폭 확보 |
| T5 | "모든 페이지 블록은 Panel Card" (components.md 6 의 5) | 데이터 화면(난이도표·목록)은 카드 없이 본문에 가득. `PanelCard` 는 대시보드 패널과 상세 블록에만 | 카드 쌓기는 여백과 제목 줄을 되풀이해 밀도를 깎음 |
| T6 | 목록도 1px gap (6-4) | 목록 행과 머리는 `border-b border-border`, 격자와 패널 사이는 gap | iidx-rank 가 실제로 쓰는 구분. 행 목록에 gap 을 쓰면 행마다 배경을 따로 칠해야 함 |
| T7 | 굵기 400·500·600·800 (800 은 공개 화면) | 400·500·600 만. 패널 제목은 500 에서 600 으로 | Surface 통합으로 800 을 쓸 곳이 없음. iidx-rank 의 제목은 600 |
| T8 | 시각은 고정폭 글꼴 (`recent-activity-panel.tsx` 19) | 시각은 본문 글꼴 + `tabular-nums`. 고정폭은 해시·토큰·코드만 | "기계 값만 고정폭" 원칙. 시각은 사람이 읽는 값 |
| T9 | 모바일도 화면 고정 + 내부 스크롤 | `md` 미만은 문서 스크롤 | Safari 의 주소창·상태 표시줄 동작. iidx-rank 가 같은 문제를 고친 기록 |
| T10 | 토스트 오른쪽 위, 기본 모양 | 오른쪽 아래, 반경 0·그림자 없음·종류별 왼쪽 4px 선 | 사이트 규칙과 맞춤 |
| T11 | "하드코드 hex 금지" (17-1 의 2) | 유지. 단 도메인 색 토큰은 `globals.css` 의 `:root` 에 16진수로 둠(컴포넌트에는 여전히 금지) | 스킨의 sRGB 값을 그대로 옮겨야 앱과 같은 색이 됨 |

#### 3.1.2 다크 바탕을 앱 값에 맞추기

앱의 무채색 단계는 `crates/rbms-render/src/theme.rs` 164-204 에 있고, 기본 스킨의 `theme.mono`(`assets/skins/rbms-default/lua/theme.lua` 8-18)와 같습니다. OKLCH 명도로 바꾸면 지금 웹의 다크 값과 거의 겹칩니다(환산은 표준 sRGB → OKLab 식으로 계산).

| 앱 상수 | sRGB | OKLCH 명도 | 웹 토큰(다크) | 지금 값 | 제안 |
|---|---|---|---|---|---|
| `GROUND` (바탕) | 0 `#000000` | 0.000 | `--sidebar` | 0.098 (`#030303`) | 0 (`--palette-neutral-000: oklch(0 0 0)` 추가) |
| (앱에 없음) | — | — | `--background` | 0.162 (`#0e0e0e`) | 유지. 카드 사이 틈으로 보이는 색 |
| `SURFACE` (패널·행) | 22 `#161616` | 0.200 | `--card` | 0.212 (`#191919`) | 0.200 (`--palette-neutral-200` 추가) |
| `RAISED` (올린 면) | 38 `#262626` | 0.269 | `--muted`, `--secondary`, `--accent` | 0.269 | 유지(이미 같음) |
| `LINE` (가는 선) | 60 `#3c3c3c` | 0.356 | `--border` | 흰색 10%(카드 위에서 약 `#2d2d2d`) | 유지. 카드 1,000장의 격자에서는 옅은 선이 낫다고 판단 |
| `INK_MUTED` | 144 `#909090` | 0.653 | `--muted-foreground` | 0.708 (`#a1a1a1`) | 유지. 11px 글자의 대비 7.0:1 을 지킴(앱 값은 5.7:1) |
| `INK` (먼저 읽는 글자) | 242 `#f2f2f2` | 0.961 | `--foreground` | 0.985 | 0.961 (`--palette-neutral-961` 추가) |
| `LIT` (커서·강조) | 255 `#ffffff` | 1.000 | `--primary` | 0.922 (`#e5e5e5`) | 1 (`--palette-neutral-1000`). 주 버튼과 켜진 표시가 순백 = "흰 강조" |
| `CAUTION` | (230,180,60) | `oklch(0.795 0.143 85)` | `--warning` | `oklch(0.79 0.145 75)` | 유지(거의 같음) |

함께 고칠 것: 다크의 `--sidebar-primary`(`--palette-blue-488`)와 양 테마의 `--sidebar-ring`(`--palette-blue-623`)은 뜻 없는 파랑입니다. 각각 `var(--primary)`, `var(--ring)` 으로 바꿉니다. 라이트 값은 그대로 둡니다(iidx-rank 와 같음).

#### 3.1.3 의미색

앱에는 램프 색이 두 벌 있습니다. 기본 스킨(`theme.lua` 139-150, 선곡·결과 화면)과 네이티브 화면(`apps/rbms-player/src/format.rs` 116-130, IR 순위·결과의 글자 색으로 쓰이며 주석에 "reference implementation's official lamp palette")입니다. 웹은 스킨 값을 따르는 것을 권합니다. 스킨이 사용자가 보는 얼굴이고, `format.rs` 값은 FAILED `#000080` 과 MAX `#0000ff` 가 다크 카드(`#161616`) 위에서 대비 1.1:1, 2.1:1 이라 웹 다크에 쓸 수 없습니다(6절 결정 3).

램프 11종 (`web:shared/constants/clear-lamp.ts` 의 키 → `theme.lua` 키):

| 웹 키 | 스킨 키 | 색 | `#161616` 대비 | 흰색 대비 | 토큰 | 표시 이름(제안) |
|---|---|---|---|---|---|---|
| `NoPlay` | `lamp_off` | `#444444` | 1.9 | 9.7 | `--rbms-lamp-noplay` | NO PLAY |
| `Failed` | `failed` | `#ff0000` | 4.5 | 4.0 | `--rbms-lamp-failed` | FAILED |
| `AssistEasy` | `assist` | `#ff63fd` | 7.3 | 2.5 | `--rbms-lamp-assist` | ASSIST |
| `LightAssistEasy` | `light_assist` | `#6563ff` | 4.1 | 4.4 | `--rbms-lamp-light-assist` | L-ASSIST |
| `Easy` | `easy` | `#00ff24` | 13.2 | 1.4 | `--rbms-lamp-easy` | EASY |
| `Normal` | `normal` | `#ffbb00` | 10.7 | 1.7 | `--rbms-lamp-normal` | CLEAR |
| `Hard` | `hard` | `#dfdfdf` | 13.6 | 1.3 | `--rbms-lamp-hard` | HARD |
| `ExHard` | `exhard` | `#ff6e6e` | 6.6 | 2.7 | `--rbms-lamp-exhard` | EX HARD |
| `FullCombo` | `fullcombo` | `#00dfff` | 11.2 | 1.6 | `--rbms-lamp-fullcombo` | FULL COMBO |
| `Perfect` | `perfect` | `#f499f2` | 9.2 | 2.0 | `--rbms-lamp-perfect` | PERFECT |
| `Max` | `max` | `#eaf300` | 14.9 | 1.2 | `--rbms-lamp-max` | MAX |

- 표시 이름: 지금 웹은 `Normal` 을 "NORMAL" 로 쓰지만 앱은 두 곳 모두 "CLEAR" 입니다(`theme.lua` 162, `format.rs` 123). 앱에 맞춥니다.
- 쓰는 법: 램프 색은 띠와 견본의 배경으로만 씁니다. 글자는 항상 `text-foreground` 이고 이름을 함께 적습니다. 라이트에서 대비가 1.2~2.7 인 색이 많으므로 견본에는 `border border-foreground/40`, 띠에는 오른쪽 구분선 `box-shadow: 1px 0 0 0 color-mix(in oklch, var(--foreground) 45%, transparent)` 를 둡니다(iidx-rank 와 같은 처리).
- 점멸은 넣지 않습니다. R-BMS 의 램프는 한 색이고, 카드 1,000장이 깜빡이면 소음입니다.
- `LampBadge`(색 배지)는 없애고 "견본 + 이름"(`LampSwatch` + 글자)으로 바꿉니다.

판정 5색과 타이밍 (`theme.lua` 121-137):

| 뜻 | 색 | 토큰 | 쓰는 곳 |
|---|---|---|---|
| PGREAT | `#6ba6f5` | `--rbms-judge-pgreat` | 점수 상세의 판정 막대, 리플레이 |
| GREAT | `#f6ab22` | `--rbms-judge-great` | 〃 |
| GOOD | `#f9b21f` | `--rbms-judge-good` | 〃 |
| BAD | `#fb8116` | `--rbms-judge-bad` | 〃 |
| POOR | `#f25200` | `--rbms-judge-poor` | 〃 |
| FAST | `#03a9ff` | `--rbms-timing-fast` | FAST/SLOW 수 |
| SLOW | `#ff4503` | `--rbms-timing-slow` | 〃 |

GREAT 과 GOOD 은 색이 거의 같으므로(스킨 값 그대로) 반드시 이름을 함께 적습니다. 숫자는 `text-foreground`, 색은 막대와 6px 사각 표식에만 씁니다.

난이도(`#DIFFICULTY` 1~5, `theme.lua` 103-110):

| 값 | 이름 | 색 | 토큰 |
|---|---|---|---|
| 0 | UNKNOWN | `#c3c3c3` | `--rbms-difficulty-unknown` |
| 1 | BEGINNER | `#06ff00` | `--rbms-difficulty-beginner` |
| 2 | NORMAL | `#12d2d7` | `--rbms-difficulty-normal` |
| 3 | HYPER | `#ffc000` | `--rbms-difficulty-hyper` |
| 4 | ANOTHER | `#ff0000` | `--rbms-difficulty-another` |
| 5 | INSANE | `#a840aa` | `--rbms-difficulty-insane` |

차트 페이지와 검색 결과에서 제목 블록·행의 왼쪽 2px 선으로 씁니다(iidx-rank 의 `.checker-chart-card` 방식). 난이도표 카드에는 쓰지 않습니다. 표 자료에 `#DIFFICULTY` 가 없고, 레벨은 묶음 머리가 이미 말합니다. (`format.rs` 73-81 에는 다른 난이도 색이 있습니다. 램프와 같은 까닭으로 스킨 값을 따릅니다.)

랭크(AAA … F)는 색을 쓰지 않습니다. 앱에도 랭크 색이 없습니다(`theme.rank_name` 만 있음). 굵기와 명도로 구분합니다: `MAX`·`MAX-`·`AAA` 계열은 `font-semibold text-foreground`, `AA`·`A` 계열은 `font-medium text-foreground`, `B` 이하는 `text-muted-foreground`. 랭크 계산은 앱의 27단계를 그대로 옮깁니다(`crates/rbms-render/src/result/grade.rs` 14 `RATE_STEPS = 27`, 52 `rate_27(ex, max_ex)`, 115 의 이름 표 `F-` … `MAX`). 웹에는 `shared/lib/dj-rank.ts` 로 한 번 만듭니다.

난이도표 레벨(★1 … ★25, ★???)에도 색을 주지 않습니다.

#### 3.1.4 글꼴·간격·반경·모션

| 분류 | 값 | 비고 |
|---|---|---|
| 글꼴 | 시스템 묶음 유지(`--font-sans`, `--font-mono`) | 스킨은 Inter 를 씁니다(`theme.lua` 235-238). 웹에서 Inter 를 쓰는 안은 2차 검토(곡명 대부분이 일본어라 어차피 시스템 글꼴과 섞임) |
| 크기 | 11px(`text-2xs`) · 12px(`text-xs`) · 14px(`text-sm`) 이 기본. 16px 문서 절 제목, 18px 프로필 이름, 20·24px 큰 숫자 | 1.4절 표 그대로 |
| 굵기 | 400 · 500 · 600 | `font-bold`, `font-extrabold` 금지 |
| 안쪽 여백 | 12px(`p-3`, `px-3`) 한 가지. 행 세로 8px(`py-2`), 대화상자 본문 16px(`p-4`) | |
| 높이 | 48px 페이지 머리·묶음 머리·사이드바 머리와 바닥, 36px 메뉴 항목·패널 머리, 32px 보조 줄, 24px 소묶음 머리, 56px 난이도표 카드, 40px 목록 행(폴더 편집) | |
| 버튼 | `sm` 28px 이 기본, 패널 안 `xs` 24px, 아이콘 `icon-sm` 28px. 주 동작만 채움(`default`), 나머지 `outline`·`ghost` | `web:shared/ui/button.tsx` 22-29 에 이미 있음 |
| 선 | 1px. 격자와 패널 사이는 `gap-px` + `bg-border`, 행과 머리는 `border-b` | |
| 반경 | 0 | 견본·띠·배지 모두 사각 |
| 그림자 | 대화상자·팝오버·드롭다운만 `shadow-md` 이상 | |
| 모션 | 투명도 0.18s, 막대 폭 0.24s, `cubic-bezier(0.4, 0, 0.2, 1)`. 화살표 회전과 스크롤 막대(200ms) 말고는 없음 | 스킨의 `theme.time.fade = 300` 은 게임 화면 전환용이라 옮기지 않음 |
| z-index | rail 10 · raised 20 · level-header 30 · scroll-indicator 40 · overlay 50 | 30 과 40 을 추가 |

`globals.css` 에 넣을 클래스(이름은 `rbms-` 접두): `.rbms-micro-label`(지금 `filter-panel.tsx` 52·62·81, `bar-count-list.tsx` 13 에 같은 클래스 묶음이 네 번 되풀이됨), `.rbms-page-header`, `.rbms-level-header`, `.rbms-list-scroll`, `.rbms-lamp`, `.rbms-empty`. 내용은 1.2절의 `checker-*` 와 같습니다.

### 3.2 골격

#### 3.2.1 뼈대

iidx-rank 의 `AppShell` 을 옮깁니다. 모든 경로가 이 셸 안에 들어갑니다.

| 부분 | 내용 |
|---|---|
| 사이드바 머리(48px) | 접기 버튼 + 사이트 이름 `R-BMS`(`text-sm font-semibold tracking-tight`, 홈 링크) |
| 메뉴(항목 36px) | 홈 `/` · 차트 검색 `/search` · 순위 `/leaderboards` · 가이드 `/guide` · 내 폴더 `/folders` · 난이도표 `/tables`. 보조 패널이 있는 두 메뉴를 마지막에 둬 패널이 메뉴 바로 아래에 펼쳐지게 함(1.3절의 지적 규칙) |
| 페이지 슬롯 | 난이도표 페이지는 "출처"와 "내 진행", 폴더 화면은 "R-BMS 에서 보는 법", 가이드는 목차를 꽂음 |
| 보조 링크 | 맨 아래 11px 한 줄: 버전(예: `v0.1.0`)과 저장소 링크. 접힌 사이드바에서는 숨김 |
| 바닥(48px) | 테마 버튼(48px 정사각) + 계정. 비로그인은 "로그인" 버튼, 로그인하면 이름과 드롭다운(내 페이지 `/players/<id>`, 내 폴더, 설정, 로그아웃) |
| 본문 | `main`. 바깥 여백 0. 페이지는 `section.flex.min-w-0.flex-1.flex-col.md:h-full.md:min-h-0.md:overflow-hidden` 안에 머리와 `ScrollContainer` 를 둠 |

설정은 메뉴에서 빼고 계정 드롭다운으로 옮깁니다(iidx-rank 와 같음. 로그인해야 쓸 수 있는 화면이 공개 메뉴에 있으면 눌렀을 때 로그인 화면으로 튕깁니다). 관리(`/admin/builds`)는 관리자에게만 드롭다운에 보입니다.

푸터는 따로 두지 않습니다. 데이터 화면은 끝없이 스크롤되고, 푸터에 둘 내용(버전, 저장소, 출처)은 사이드바 보조 링크와 각 페이지의 출처 줄이 맡습니다.

#### 3.2.2 공통 페이지 머리

`ShellPageHeader` 한 가지입니다.

| 요소 | 규칙 |
|---|---|
| 높이·여백 | 48px, 좌우 12px, 아래 1px 선, `bg-background` |
| 왼쪽 | (모바일만) 사이드바 열기 버튼 → `h1`(`text-sm font-semibold`, 한 줄 말줄임) → 설명(`text-xs` muted, 한 줄 말줄임, `sm` 미만 숨김) |
| 오른쪽 | 동작 묶음 `gap-1`. 순서는 전환(ToggleGroup) → 개수(`text-xs tabular-nums` muted, 폭 고정) → 아이콘 버튼들 → 주 버튼 |
| 붙음 | 모바일은 `sticky top-0 z-(--z-raised)`, `md` 이상은 본문이 내부 스크롤이라 고정 위치 |
| 제목 내용 | 목록 화면은 메뉴 이름("난이도표"), 상세 화면은 대상의 이름(표 이름, 곡명, 플레이어 이름, 폴더 이름) |

문서 제목은 `app/layout.tsx` 에 `title: { template: '%s | R-BMS', default: 'R-BMS' }` 를 두고 페이지마다 `metadata` 또는 `generateMetadata` 로 채웁니다.

#### 3.2.3 폭

| 화면 종류 | 폭 |
|---|---|
| 데이터(난이도표, 목록, 순위, 검색, 대시보드, 폴더 편집) | 본문 폭 전체 |
| 문서(가이드) | `max-w-3xl`(48rem), 왼쪽 붙임, `p-3 pb-12` |
| 폼(설정, 폴더 이름·설명) | 구역은 폭 전체 + `border-b p-3`, 필드 묶음만 `max-w-xl`(36rem) |
| 인증 카드(로그인·가입) | `max-w-sm`(24rem), 본문 가운데 |

#### 3.2.4 다음 웨이브가 병렬로 일할 때의 계약

골격 담당이 먼저 만들 것이고, 난이도표·폴더 담당은 아래 시그니처에 맞춰 씁니다.

| 이름 | 경로(제안) | 시그니처 |
|---|---|---|
| `ShellPageHeader` | `widgets/shell/shell-page-header.tsx` | `{ title: string; description?: string; actions?: ReactNode }` |
| `ShellSidebarPortal` | `widgets/shell/shell-sidebar-portal.tsx` | `PropsWithChildren`. 사이드바 슬롯에 자식을 꽂음 |
| `ScrollContainer` | `shared/ui/scroll-container.tsx` | `ComponentProps<'div'> & { variant?: 'always' \| 'desktop'; containerClassName?: string; indicatorClassName?: string }`. 기본 `desktop` |
| `LampSwatch` | `features/lamp/lamp-swatch.tsx` | `{ lamp: ClearLamp; className?: string }` |
| `LAMP_BACKGROUND` | `shared/constants/clear-lamp.ts` | `Record<ClearLamp, string>`, 값은 `'var(--rbms-lamp-…)'` |
| `getDjRank` | `shared/lib/dj-rank.ts` | `(exScore: number, maxExScore: number) => string` (27단계 이름) |
| 페이지 틀 | 각 위젯 | `<section className='flex min-w-0 flex-1 flex-col md:h-full md:min-h-0 md:overflow-hidden'>` 안에 머리 + `ScrollContainer` |

### 3.3 화면별 설계

데이터 열은 "화면이 필요로 하는 것"만 적었습니다. API 경로와 필드 이름은 표·폴더 담당 문서가 정합니다.

#### 3.3.1 홈 `/`

| 항목 | 내용 |
|---|---|
| 구성 | 머리("홈") + 패널 격자. 패널은 `DashboardPanel`(36px 머리 + 본문), 격자는 `gap-px bg-border` |
| 배치 | 본문 폭의 컨테이너 쿼리로 1열(45rem 미만) · 2열 · 3열(62rem) · 4열(96rem). iidx-rank 의 `DASHBOARD_GRID_CLASS_NAME` 을 그대로 씀 |
| 비로그인 패널 | (1) 소개: 한 문단 + 할 수 있는 일 3줄 + 버튼 [클라이언트 받기 → 가이드] [난이도표 보기] [로그인] (2) 서버 현황: 큰 숫자(등록 차트) + 플레이어·누적 제출 줄 (3) 난이도표: 표마다 한 줄(이름, 차트 수), 전체 보기 (4) 최근 활동: 행 목록 |
| 로그인 패널 | (1) 내 정보: 이름, id, 플레이 수, 최근 제출 시각, [내 페이지] (2) 램프 현황: 발광 표 기준 `LampDistribution` (3) 내 폴더: 최근 5개 + [새 폴더] (4) 최근 플레이: 행 목록 |
| 최근 활동 행 | 높이 40px. 왼쪽 5px 램프 띠, 곡명(12px 500) + 플레이어 이름(11px muted), 오른쪽에 램프 이름·랭크(12px) + EX·시각(11px muted). iidx-rank `RecentRecordRow` 모양 |
| 데이터 | 서버 통계, 최근 제출(이미 있음: `/api/fe/stats/summary`, `/api/fe/activity/recent`), 표 목록, 로그인 시 내 통계·최근·폴더 |
| 로딩 | 패널 틀은 정적, 본문만 스켈레톤(`DashboardPanelLoading`) |
| 빈 | 패널 안 한 줄 문구 + 버튼(예: "아직 기록이 없습니다" + [가이드 보기]) |
| 오류 | 패널마다 아이콘 + 문구 + 재시도 |
| 모바일 | 1열, 문서 스크롤. 목록 패널은 높이 제한(`max-h-100`) |

1차에서는 지금 있는 자료로 되는 네 패널(소개 또는 내 정보, 서버 현황, 난이도표, 최근 활동)만 만듭니다.

#### 3.3.2 난이도표 목록 `/tables`

| 항목 | 내용 |
|---|---|
| 구성 | 머리("난이도표", 설명 "R-BMS 에 기본 등록된 표와 서버가 제공하는 표") + 설명 한 줄(`border-b px-3 py-2 text-xs` muted) + 행 목록 |
| 행 | `border-b px-3 py-2 hover:bg-muted/50`, 행 전체 링크. 왼쪽 32px 사각 칸에 표 기호(예: ★, `bg-muted font-semibold`), 이름(`text-sm font-medium`) + 아랫줄 "차트 1,035 · 레벨 26 · 출처 darksabun.club"(`text-xs` muted). 오른쪽: 로그인 시 얇은 램프 누적 막대(폭 96px)와 "312 / 1,035"(`tabular-nums`) |
| 데이터 | 표마다 이름, 기호, 차트 수, 레벨 수, 출처 이름. 로그인 시 내 램프 분포 |
| 로딩 | 행 모양 스켈레톤 3줄 |
| 빈 | `Empty` "등록된 표가 없습니다" |
| 오류 | `Empty` + 재시도 |
| 모바일 | 오른쪽 막대를 아랫줄로 내림(`flex-col sm:flex-row`) |

#### 3.3.3 발광 난이도표 페이지 `/tables/[tableId]`

자료의 크기(같은 웨이브의 표 담당이 받은 `header.json`·`data.json` 을 읽어 셈): 차트 1,035개, 레벨 26단계(`level_order` 가 "1" … "25", "???"), 레벨당 2~64곡(★10 이 64, ★25 가 2, ★??? 가 5). 필드는 `title`, `artist`, `level`, `md5`, `url`, `url_diff`, `name_diff`, `comment`, `lr2_bmsid` 이고 `sha256` 은 없습니다. 곡명 길이는 중앙값 21자, 90% 가 32자 이하, 최대 66자. `url` 은 1,033건, `url_diff` 는 825건에 값이 있습니다. 머리에는 `name: 発狂BMS難易度表`, `symbol: ★`, `last_update: 2017.02.05`, `original_url` 이 있습니다.

보기는 레벨별 카드 격자를 권합니다(6절 결정 4). 1,035곡을 한 곡 한 행(32px)으로 놓으면 묶음 머리를 합쳐 약 34,000px 이고, 5열 카드(56px)면 약 13,000px 입니다. iidx-rank 에서 사용자가 행을 카드로 바꾸게 한 것도 같은 까닭입니다.

| 항목 | 내용 |
|---|---|
| 머리 | `h1` 표 이름(発狂BMS難易度表). 설명 "★1 ~ ★25 · 차트 1,035개 · 출처 darksabun.club". 동작: 검색 입력(`md` 이상, 폭 224px, 높이 28px, 돋보기 아이콘, `/` 키로 포커스) → 결과 수 "1,035곡" → 모두 접기·펼치기(아이콘) → 레벨 이동(아이콘, 드롭다운에 레벨 칩 6열) → 필터(아이콘 + 적용 개수 배지) |
| 레벨 묶음 | `section id='lv-12' aria-label='★12'`. 접을 수 있음(기본 펼침) |
| 묶음 머리(48px, 붙음) | `bg-muted`. 화살표 + "★12"(`text-sm font-semibold tabular-nums`) + (로그인) 램프 누적 막대(높이 12px, `flex-1`, 최대 폭 256px, 구간 사이 1px, 바깥 `bg-foreground/40 p-px`) + 오른쪽 "12 / 58"(플레이한 곡 / 전체, 비로그인은 "58곡") |
| 통계 머리 | 따로 표를 두지 않습니다. "모두 접기"를 누르면 묶음 머리 26줄이 그대로 레벨별 램프 표가 됩니다. 전체 합계는 사이드바 패널에 둡니다 |
| 격자 | 2열 / `sm` 3 / `lg` 4 / `xl` 5 / `2xl` 6. 마지막 행 빈칸은 빈 카드로 채움 |
| 카드(56px) | `button`. 왼쪽 5px 램프 띠(내 램프 색, 기록 없으면 `--rbms-lamp-noplay`, 비로그인도 같은 회색). 곡명 `text-2xs leading-tight font-medium` 2줄 말줄임. 아랫줄 왼쪽 아티스트(`text-2xs` muted 한 줄 말줄임), 오른쪽 내 랭크(`AAA` 등, 3.1.3 의 굵기 규칙). `hover:bg-muted/50`, 포커스는 안쪽 2px 고리. `aria-label` 은 "곡명 · 아티스트 · ★12 · HARD · AA" |
| 카드 누르기 | 상세 대화상자를 엽니다. 웹에서는 램프를 고치지 않습니다(램프는 R-BMS 가 제출한 기록에서 옴) |
| 상세 대화상자(`sm:max-w-md`) | 제목(곡명, 줄바꿈 허용) · 설명(아티스트) · 레벨 배지 · 링크 줄 [본체] (`url`) [차분] (`url_diff`, 이름은 `name_diff` 가 주소 모양이 아니면 그것을, 아니면 "차분") · 코멘트(있으면 `text-xs` muted) · md5(고정폭, 복사 버튼) · "내 기록" 상자(1px 테두리, 안쪽 8px): 램프 견본과 이름, 랭크, EX 와 달성률, BP, 최대 콤보, 기록 시각 · 버튼 [리더보드 보기 → `/charts/<md5>`]. 비로그인이면 상자 자리에 "로그인하면 내 램프와 점수가 표시됩니다" + [로그인] |
| 링크 안전 | `url`·`url_diff` 는 외부 자료입니다. `http://` 또는 `https://` 로 시작할 때만 링크로 만들고 `target='_blank' rel='noopener noreferrer'` 를 답니다(iidx-rank `checker-workspace.tsx` 255 와 같은 검사). 자료의 주소는 대부분 `http://` 입니다 |
| 검색 | 곡명과 아티스트를 클라이언트에서 거릅니다(소문자 비교). 맞는 곡이 없는 레벨은 숨깁니다 |
| 필터 대화상자 | 검색어(모바일은 여기서 입력) · 레벨(Select: 전체, ★1 … ★???) · (로그인) 램프(Select: 전체, 미플레이, FAILED 이하, EASY 이상 …) · 정렬(곡명 순 / 원본 순) · 아래에 "n / 1,035" 와 [초기화] |
| 주소 반영 | `?q=`, `?level=`, `?lamp=` 를 `router.replace` 로 반영합니다. "★12 만" 같은 주소를 공유할 수 있게 |
| 레벨 점프 | (1) 머리의 레벨 이동 드롭다운(칩을 누르면 `#lv-12` 로 이동) (2) 모두 접기 뒤 머리를 눌러 펼침. `html { scroll-padding-block-start }` 를 머리 두 단 높이(96px)로 둬 붙은 머리에 가리지 않게 함 |
| 사이드바 패널 | "출처": 표 이름, 원본(`original_url` 의 호스트), 보관본(darksabun.club archive), 원본 갱신일 `2017.02.05`, 차트 수, 레벨 수, 버튼 [원본 보기] (새 탭) [표 주소 복사]. "내 진행": 플레이한 곡 / 전체(큰 숫자), 백분율, 램프 누적 막대, 램프별 개수 2열(`LampDistribution`). 비로그인은 안내 문구 |
| 출처 표기 | 세 곳: 머리 설명, 사이드바 "출처", 목록 맨 아래 한 줄("출처: 発狂BMS難易度表 · darksabun.club archive 의 사본 · 원본 갱신 2017.02.05", `border-t px-3 py-2 text-xs` muted). 모바일에서는 사이드바가 시트 안이라 맨 아래 줄이 항상 보이는 표기입니다 |
| 표준 표 형식 | 이 페이지의 `head` 에 `bmstable` 메타가 들어갑니다(주소는 표 담당 문서). 화면에는 "표 주소 복사" 버튼으로만 드러냅니다 |
| 데이터 | 표 머리와 항목(정적 스냅샷). 로그인 시 이 표의 md5 전체에 대한 내 최고 기록을 한 번에: md5 → 램프, EX, 노트 수(랭크 계산용), BP, 최대 콤보, 시각. 차트별로 1,035번 요청하지 않게 일괄 조회가 필요합니다 |
| 로딩 | 표 자료는 정적이므로 서버가 곡명까지 그린 격자를 바로 내보냅니다(검색 엔진도 읽음). 내 기록이 오기 전에는 띠가 회색, 랭크 자리에 `Skeleton h-3 w-8`, 묶음 머리의 막대 자리에 스켈레톤 |
| 빈 | 검색·필터 결과 0: `Empty` "조건에 맞는 차트가 없습니다" + [필터 초기화] |
| 오류 | 내 기록만 실패: 목록 위 `Alert`(문구 + 재시도), 표는 그대로 보임. 없는 표: 셸 안 404 |
| 모바일 | 2열(폭 390px 에서 카드 195px). 문서 스크롤, 머리(48px)와 묶음 머리(48px)가 차례로 붙음. 결과 수는 머리 아래 32px 줄로. 검색은 필터 대화상자 안. 묶음 머리의 막대는 유지(폭이 줄어듦) |

카드 폭 검산: 본문 폭이 1,024px(화면 1,280 - 사이드바 256)일 때 5열이면 카드 205px, 안쪽 약 190px 에 11px 글자가 라틴 기준 한 줄 30자쯤 들어가 2줄이면 중앙값(21자)과 90%(32자)를 담습니다. 일본어 곡명은 한 줄 17자쯤이라 긴 것은 잘리고, 전체는 상세 대화상자에서 봅니다.

#### 3.3.4 차트 페이지 `/charts/[hash]`

| 항목 | 내용 |
|---|---|
| 머리 | `h1` 곡명, 설명 아티스트. 동작: [표에서 보기] (이 차트가 든 표로) |
| 제목 블록 | `border-b p-3`, 왼쪽 2px 난이도 색 선. 곡명(`text-lg font-semibold`, 줄바꿈), 부제, 아티스트. 메타 줄(`text-xs`): 모드 · 레벨 · 노트 · BPM · TOTAL 을 "라벨(muted) 값(`tabular-nums`)" 쌍으로. 표 소속 태그("発狂 ★12", 누르면 표의 그 레벨로). md5·sha256 은 고정폭 앞 8자 + 복사 버튼 |
| 요약 줄 | 4칸 격자(`gap-px`, 모바일 2칸): 플레이어 수, 최고 EX(달성률), 클리어한 사람 수, 내 순위(로그인) |
| 랭킹 | 가득 찬 표. 열: 순위 · (5px 램프 띠) 플레이어 · 램프 이름 · 랭크 · EX(달성률) · BP · 콤보 · 시각. 내 행은 `bg-muted` + 왼쪽 2px `--foreground` 선 |
| 데이터 | 지금 있는 리더보드 응답 + 표 소속 |
| 로딩 | 제목 블록 스켈레톤(3줄) + 표 행 스켈레톤 8줄 |
| 빈 | 기록 없음: 표 자리에 `Empty` "등록된 기록이 없습니다". 미등록 차트: 화면 전체 `Empty` + [차트 검색] |
| 오류 | `Empty` + 재시도 |
| 모바일 | 표 열을 순위·플레이어·EX·BP 로 줄임(나머지 `hidden sm:table-cell`). 램프는 띠로 남음 |

#### 3.3.5 순위 `/leaderboards`

| 항목 | 내용 |
|---|---|
| 머리 | "순위". 동작: 정렬 전환(ToggleGroup: 랭크 포인트 / 플레이 수) + "플레이어 n명" |
| 본문 | 가득 찬 표: 순위 · 플레이어(이름 500 + id 11px muted) · 랭크 포인트 · 플레이 수 · 클리어 수. 1~3위는 순위 숫자만 `font-semibold text-foreground`(메달·색 없음), 나머지 muted |
| 페이지 | 번호 페이지네이션, 주소 `?page=` |
| 상태 | 로딩 행 스켈레톤 10줄 · 빈 "아직 순위에 오른 플레이어가 없습니다" · 오류 재시도 |
| 모바일 | 순위·플레이어·랭크 포인트만 |

#### 3.3.6 플레이어 페이지 `/players/[playerId]`

| 항목 | 내용 |
|---|---|
| 머리 | `h1` 플레이어 이름 |
| 프로필 블록 | `border-b p-3`. 이름(`text-lg font-semibold`) + `@id`(고정폭 12px muted) + 한 줄 수치(종합 순위, 플레이 수, 가입일). 오른쪽 동작: (로그인, 남의 페이지) [라이벌 추가], (내 페이지) [설정] |
| 탭 | 개요 / 기록. 주소 `?tab=`. 탭 줄 `overflow-x-auto border-b px-3 py-2` |
| 개요 | 램프 분포(`LampDistribution`), 발광 표의 레벨별 램프(묶음 머리와 같은 막대를 26줄 목록으로), 라이벌 |
| 기록 | 행 목록(최근 활동 행과 같은 모양) + 페이지네이션 |
| 상태 | 프로필 스켈레톤 · 없는 플레이어는 `Empty` + [순위로] · 탭별 재시도 |
| 모바일 | 프로필 블록의 동작이 아랫줄로 내려감 |

#### 3.3.7 내 폴더 목록 `/folders`

세션이 필요합니다. 없으면 지금의 `SessionGate` 가 `/login?next=%2Ffolders` 로 보냅니다.

| 항목 | 내용 |
|---|---|
| 머리 | "내 폴더", 설명 "R-BMS 선곡 화면에 표처럼 나타나는 곡 묶음". 동작: [새 폴더] (채움 버튼, 더하기 아이콘) |
| 본문 | 행 목록. 행: 이름(`text-sm font-medium`) + 설명(`text-xs` muted 한 줄 말줄임). 오른쪽: "32곡"(`tabular-nums`) · 수정 시각(`text-xs` muted) · 점 세 개 메뉴(이름·설명 수정, 삭제). 행 전체가 편집 화면 링크이고 메뉴 버튼만 따로 눌림 |
| 새 폴더 | 대화상자(`FormDialog`): 이름(필수) · 설명(선택, 2줄). 만들면 편집 화면으로 이동 |
| 삭제 | 확인 대화상자(`ConfirmAction`): "폴더 '이름' 을 삭제합니다. R-BMS 선곡 화면에서도 사라집니다." 취소는 왼쪽 ghost, 삭제는 destructive |
| 사이드바 패널 | "R-BMS 에서 보기": 두 줄 안내("R-BMS 에 로그인하면 이 폴더들이 선곡 화면에 나타납니다. R-BMS 는 읽기만 합니다.") + [연결 방법 → 가이드] |
| 데이터 | 폴더마다 id, 이름, 설명, 곡 수, 수정 시각 |
| 로딩 | 행 스켈레톤 3줄 |
| 빈 | `Empty` "아직 폴더가 없습니다" + 설명 + [첫 폴더 만들기] |
| 오류 | `Empty` + 재시도 |
| 모바일 | 곡 수와 시각이 아랫줄로 |

#### 3.3.8 폴더 편집 `/folders/[folderId]`

2단 구성을 권합니다. 왼쪽이 폴더의 곡 목록, 오른쪽이 늘 열려 있는 검색 패널입니다. 찾고 → 넣고 → 목록에 붙는 것을 보는 일을 대화상자를 여닫지 않고 되풀이할 수 있습니다. 우측 패널 폭은 이미 있는 `--context-panel-width: 20rem` 을 씁니다.

| 항목 | 내용 |
|---|---|
| 머리 | `h1` 폴더 이름, 설명 "32곡 · 저장되지 않은 변경 3건"(변경이 없으면 수정 시각). 동작: [표에서 가져오기] (outline) · (`xl` 미만) [곡 추가] (outline) · [저장] (채움, 변경이 있을 때만 켜짐) · 점 세 개 메뉴(이름·설명 수정, 삭제) |
| 왼쪽: 곡 목록 | `ol`. 행 40px, `border-b`. 순번(`w-8` 오른쪽 정렬 `tabular-nums` muted) · 곡명(`text-xs font-medium` 한 줄) + 아티스트(`text-2xs` muted 한 줄) · (`lg` 이상) md5 앞 8자 고정폭 · 동작: 위로 · 아래로 · 삭제(각 `icon-xs` 24px, 터치 영역 확대) |
| 순서 바꾸기 | 1차는 위·아래 버튼과 행 메뉴의 "맨 위로 / 맨 아래로". 키보드와 터치에서 그대로 됩니다. 끌어 옮기기는 2차(라이브러리가 필요해 따로 결정) |
| 오른쪽: 검색 패널(`xl` 이상, 320px, `bg-sidebar`) | micro-label "곡 추가" · 입력(자동 포커스, "곡명 · 아티스트 · md5") · 출처 전환(ToggleGroup: IR 차트 / 난이도표) · 결과 행(40px): 곡명 + 아티스트, 오른쪽에 [추가] (더하기 아이콘) 또는 이미 든 곡은 muted "추가됨". 입력 후 300ms 뒤 조회(상수 `SEARCH_DEBOUNCE_MS`) |
| `xl` 미만 | 검색 패널을 대화상자로(머리의 [곡 추가]) |
| 표에서 가져오기 | 대화상자: 표(Select) → 레벨(칩 여러 개 선택, "전체 선택") → 요약 "58곡 추가(이미 있는 3곡 제외)" → [가져오기]. 작업 목록 끝에 붙고 저장 전까지는 미저장 |
| 이름·설명 | 대화상자(목록 화면과 같은 폼) |
| 저장 | 명시 저장을 권합니다(6절 결정 5). 순서와 곡 목록 전체를 한 번에 보냅니다. 미저장 상태로 탭을 닫으면 `beforeunload` 경고(iidx-rank `use-unsaved-changes-warning.ts`) |
| 삭제 | 확인 대화상자, 삭제 뒤 목록으로 |
| 데이터 | 폴더(이름, 설명, 수정 시각, 순서 있는 항목: md5 와 표시용 곡명·아티스트). 검색 결과(md5, 곡명, 아티스트, 레벨) |
| 로딩 | 머리 + 행 스켈레톤 8줄 + 패널 스켈레톤 |
| 빈 | 목록 자리에 `Empty` "곡이 없습니다" + "오른쪽에서 곡을 찾아 넣거나 표에서 가져오세요" + [표에서 가져오기]. 검색 패널: 입력 전 "두 글자 이상 입력하세요", 결과 0 "조건에 맞는 차트가 없습니다" |
| 오류 | 저장 실패: 토스트 + 머리 아래 `Alert`(다른 곳에서 먼저 고쳐 충돌했을 때 "다시 불러오기"). 조회 실패: `Empty` + 재시도. 남의 폴더·없는 폴더: 셸 안 404 |
| 모바일 | 1단. 행의 md5 숨김, 동작 3개 유지. 저장 버튼은 머리에 남음 |

검색 패널은 의존성 없이 `Input` 과 결과 `ul` 로 만듭니다. `cmdk`(shadcn `command`)는 명령 팔레트용이고 여기서는 늘 열린 패널이라 필요하지 않습니다(6절 결정 6).

#### 3.3.9 설정 `/settings`

| 항목 | 내용 |
|---|---|
| 머리 | "설정" |
| 탭 | 프로필 / API 토큰 / 동기화 / 라이벌. 주소 `?tab=` |
| 구역 | `SettingsSection`: `border-b p-3`, `h2 text-sm font-semibold` + 설명 `text-xs` muted, 필드 묶음 `max-w-xl` |
| 토큰 탭 | 구역 1 "토큰 발급": 설명 + [토큰 발급] → 대화상자(이름). 발급 직후 한 번만 보이는 상자: 1px 테두리, 고정폭, `break-all`, [복사] 버튼(누르면 토스트 "복사했습니다"), 아래에 `text-warning` 한 줄 "이 창을 닫으면 다시 볼 수 없습니다". 구역 2 "발급한 토큰": 행 목록(이름, 마지막 사용, 발급일, [폐기] → 확인 대화상자) |
| 상태 | 로딩 행 스켈레톤 · 빈 "발급한 토큰이 없습니다" + [토큰 발급] · 오류 재시도 |
| 모바일 | 탭 줄 가로 스크롤 |

#### 3.3.10 로그인 `/login` · 가입 `/signup`

셸 안에서 본문 가운데에 카드 하나입니다.

| 항목 | 내용 |
|---|---|
| 머리 | "로그인" / "가입" |
| 카드 | `mx-auto w-full max-w-sm border border-border bg-card p-6`(반경·그림자 없음). `h2 text-base font-semibold` + 설명 한 줄. 필드 간격 16px, 라벨과 입력 간격 6px. 제출 버튼 폭 전체(채움), 그 아래 전환 버튼 폭 전체(ghost): "계정이 없으신가요? 가입" |
| 오류 | 필드 아래 `role='alert'` 문구, 첫 오류 필드로 포커스. 서버 오류는 제출 버튼 위 한 줄. 인라인 오류가 있으면 토스트를 띄우지 않음 |
| 서버 HTML | 지금은 폼이 `Suspense fallback={null}` 뒤에 있어 첫 HTML 이 빈 카드입니다. fallback 을 폼 모양 스켈레톤으로 |
| 모바일 | 좌우 12px 여백, 카드 폭 전체 |

2차에서 사이드바 바닥의 "로그인"을 대화상자(같은 폼 위젯)로 열어 보던 화면을 떠나지 않게 합니다.

#### 3.3.11 가이드 `/guide`

| 항목 | 내용 |
|---|---|
| 머리 | "가이드", 설명 "클라이언트를 받고 계정에 연결하기" |
| 본문 | `article max-w-3xl p-3 pb-12 text-sm leading-6 break-keep`, 절 간격 32px. 절 제목 `h2 text-base font-semibold tracking-tight` |
| 절 1 내려받기 | 버전과 날짜 한 줄 + 운영체제별 버튼 3개(outline, 내려받기 아이콘) + "서명·공증된 빌드" 한 줄. 내려받기 주소는 릴리스 담당이 줍니다 |
| 절 2 연결 | 번호 단계(`ol`, 번호는 고정폭 muted): 가입 → 토큰 발급 또는 로그인 → R-BMS 설정에 입력. 코드 상자 `bg-muted p-3 font-mono text-xs overflow-x-auto` + 복사 버튼 |
| 절 3 난이도표와 폴더 | "発狂BMS難易度表 는 R-BMS 에 기본 등록되어 있습니다" + 다른 클라이언트용 표 주소(복사) + 내 폴더가 선곡 화면에 나타나는 방식 |
| 절 4 순위 규칙 | 설정 표: 1px 테두리, `table-fixed text-xs leading-5`, 머리 줄 `bg-muted/50`(iidx-rank `PrivacyTable` 모양) |
| 사이드바 패널 | 목차(절 제목 링크) |
| 모바일 | 표는 칸을 줄바꿈, 코드 상자는 가로 스크롤 |

#### 3.3.12 404 · 오류 · 빈 상태

| 화면 | 내용 |
|---|---|
| 404 (`app/not-found.tsx`) | 셸 안. 머리 "찾을 수 없음". `Empty`(`min-h-64 border-0`): micro-label "404" · 제목 "페이지를 찾을 수 없습니다" · 설명 "주소가 바뀌었거나 없는 페이지입니다." · [홈으로] (outline sm). 문서 제목은 "404" 에 사이트 이름을 붙인 것, `robots: { index: false }` |
| 오류 (`app/error.tsx`, 클라이언트) | 셸 안. `Empty`: 제목 "문제가 생겼습니다" · 설명 "잠시 후 다시 시도해 주세요." · [다시 시도] (`reset()` 호출) |
| 빈 상태 | `EmptyState` 를 `{ title, description?, action? }` 로 넓힘. 동작 버튼은 항상 하나 이하, `outline sm` |
| 조회 오류 | `LoadErrorState` 에 `onRetry` 와 `isRetrying` 을 더함. 색은 아이콘에만 |
| 순서 | 대기 → 전송 오류 → 빈 → 결과. 자료가 바뀌는 동안 "결과 없음"이 깜빡이지 않게 대기를 먼저 검사(지금 규칙 그대로) |

### 3.4 컴포넌트 목록

레이어 규칙: `web/AGENTS.md` 에는 Next 16 안내("코드를 쓰기 전에 `node_modules/next/dist/docs/` 를 읽을 것")만 있고, 레이어 규칙은 사용자 컨벤션(`fsd.md`)과 `docs/web/architecture.md` 를 따릅니다. `features` 는 props 만 받는 순수 컴포넌트, `widgets` 부터 조회·변경, `entities` 는 `<도메인>.type.ts`·`<도메인>.query.ts`, 배럴 파일 없음. 지금 `web/src/shared` 에는 `hooks`, `providers` 폴더가 없어 새로 만듭니다(`components.json` 의 `hooks` 별칭은 이미 `@shared/hooks`).

shadcn 에서 가져올 것(`bunx shadcn add`):

| 컴포넌트 | 쓰는 곳 | 비고 |
|---|---|---|
| `sidebar` | 셸 | 지금 없음. iidx-rank 본은 안쪽 스크롤을 `ScrollContainer` 로 바꾸고 문구를 next-intl 로 뺀 수정본이라, shadcn 원본을 넣고 같은 수정을 하거나 iidx-rank 파일을 옮겨 문구만 한국어로 바꿈 |
| `collapsible` | 레벨 묶음 | iidx-rank 는 1파일 1컴포넌트로 세 파일(`collapsible.tsx`, `collapsible-trigger.tsx`, `collapsible-content.tsx`) |
| `toggle`, `toggle-group` | 정렬·출처 전환 | |
| `checkbox` | 필터, 표에서 가져오기 | |
| `popover` | 레벨 이동(드롭다운으로 대신해도 됨) | |
| `progress` | 내 진행 | `value` 를 Root 에 넘겨 `aria-valuenow` 가 나오게(iidx-rank 감사 U5) |
| `pagination` | 순위, 기록 | |
| `alert` | 내 기록 조회 실패, 저장 충돌 | |
| `textarea` | 폴더 설명 | |

새로 만들 것:

| 이름 | 경로 | 레이어 | 하는 일 | 차수 |
|---|---|---|---|---|
| `AppShell` | `widgets/shell/app-shell.tsx` | widgets | 사이드바 + 본문 뼈대, 슬롯 컨텍스트 | 1 |
| `ShellNavMenu` | `widgets/shell/shell-nav-menu.tsx` | widgets | 메뉴(켜진 항목 계산) | 1 |
| `ShellThemeControl`, `ShellAccountControl` | `widgets/shell/` | widgets | 바닥의 테마 버튼, 계정 버튼과 메뉴 | 1 |
| `ShellPageHeader` | `widgets/shell/shell-page-header.tsx` | widgets | 페이지 머리 | 1 |
| `ShellSidebarPortal` | `widgets/shell/shell-sidebar-portal.tsx` | widgets | 슬롯에 꽂기 | 1 |
| `ShellSidebarSlotContext`, `useShellSidebarSlot` | `shared/providers/`, `shared/hooks/` | shared | 슬롯 요소 전달 | 1 |
| `ScrollContainer`, `ScrollIndicator` | `shared/ui/` | shared | 내부 스크롤 + 3px 막대 | 1 |
| `useScrollThumb`, `calculateScrollThumb` | `shared/hooks/`, `shared/lib/scroll-thumb.ts` | shared | 막대 위치 계산 | 1 |
| `AppToaster` | `shared/providers/app-toaster.tsx` | shared | 사이트 모양 토스트 | 1 |
| `CopyButton` | `shared/ui/copy-button.tsx` | shared | 복사 + 토스트(토큰, 표 주소, md5 세 곳) | 1 |
| `LampSwatch` | `features/lamp/lamp-swatch.tsx` | features | 10px 사각 견본 | 1 |
| `LampStackBar` | `features/lamp/lamp-stack-bar.tsx` | features | 램프 누적 막대(`role='img'`). 묶음 머리·목록·분포가 함께 씀 | 1 |
| `LampDistribution` | `features/lamp/lamp-distribution.tsx` | features | 큰 숫자 + 막대 + 램프별 개수 | 1 |
| `TableLevelSection` | `features/table/table-level-section.tsx` | features | 접히는 레벨 묶음과 붙는 머리 | 1 |
| `TableChartGrid`, `TableChartCard` | `features/table/` | features | 카드 격자와 빈칸 채움, 카드 | 1 |
| `TableChartDetails` | `features/table/table-chart-details.tsx` | features | 상세 대화상자 | 1 |
| `TableFilters` | `features/table/table-filters.tsx` | features | 필터 폼 | 1 |
| `TableLevelSkeleton` | `features/table/table-level-skeleton.tsx` | features | 묶음 모양 스켈레톤 | 1 |
| `TableWorkspace`, `TableWorkspaceLoading` | `widgets/table/` | widgets | 조회·필터 상태·대화상자·사이드바 패널 조립 | 1 |
| `TableList` | `widgets/table/table-list.tsx` | widgets | 표 목록(지금의 `table-list-panel` 대체) | 1 |
| `FolderListItem` | `features/folder/folder-list-item.tsx` | features | 폴더 행 | 1 |
| `FolderChartRow` | `features/folder/folder-chart-row.tsx` | features | 편집 화면의 곡 행(위·아래·삭제) | 1 |
| `ChartSearchResultRow` | `features/folder/chart-search-result-row.tsx` | features | 검색 결과 행 | 1 |
| `FolderList`, `FolderEditor`, `FolderChartSearchPanel`, `FolderTableImportDialog`, `FolderFormDialog` | `widgets/folder/` | widgets | 조회·변경·미저장 상태 | 1 |
| `useUnsavedChangesWarning` | `shared/hooks/` | shared | `beforeunload` 경고 | 1 |
| `folder.type.ts`, `folder.query.ts` | `entities/` | entities | 폴더 타입·`queryOptions`·변경 훅 | 1 |
| `table.type.ts` 확장 | `entities/` | entities | 표 항목(곡명·아티스트·레벨·링크)과 내 기록 일괄 조회 | 1 |
| `getDjRank` | `shared/lib/dj-rank.ts` | shared | 27단계 랭크 | 1 |
| `NotFoundNotice` | `features/state/not-found-notice.tsx` | features | 404 본문 | 1 |
| `SettingsSection` | `features/shell/settings-section.tsx` | features | 설정 구역 틀 | 2 |
| `DashboardPanel` 과 빈·오류·로딩·링크 | `features/dashboard/` | features | 홈 패널 | 1(홈 최소판) |
| `RecentScoreRow`, `FittedRowList` | `features/dashboard/` | features | 최근 활동 행, 넘침 숨김 | 2 |
| `BoardPagination` 에 해당하는 `ListPagination` | `features/table/` 또는 `features/shell/` | features | 번호 페이지네이션 | 2 |

고치거나 없앨 것: `features/shell/page-heading.tsx`(삭제, 머리로 대체), `widgets/shell/{nav-rail,mobile-nav,theme-toggle}.tsx` 와 `features/shell/nav-list.tsx`(셸로 대체), `widgets/shell/filter-panel.tsx`(검색 화면의 필터를 머리 동작 + 대화상자로 옮긴 뒤 삭제, 2차), `features/badge/lamp-badge.tsx`(견본 + 이름으로 대체), `features/state/{empty-state,load-error-state}.tsx`(동작·재시도 추가), `features/stats/bar-count-list.tsx`(막대 높이가 1px 이라 읽기 어려움. iidx-rank 모양으로).

### 3.5 iidx-rank 에서 가져올 코드 조각

공통으로 고칠 점: (a) `@shared/lib/utils` 의 `cn` → `@shared/lib/cn` (b) `useTranslations`·`useLocale` 호출을 한국어 문구와 `'ko-KR'` 로(R-BMS 웹에는 next-intl 이 없음) (c) `@shared/i18n/navigation` 의 `Link`·`usePathname` → `next/link`·`next/navigation` (d) 클래스 이름 `checker-*` → `rbms-*` (e) 상수 파일 위치는 `web:shared/constants/` 에 새 파일로.

| 파일(`iidx:`) | 무엇을 | 고칠 점 |
|---|---|---|
| `app/globals.css` 242-289 | `.checker-empty`, `.checker-micro-label`, `.checker-workspace`, `.checker-toolbar`·`.shell-page-header`, `.checker-rank-header`, `.checker-list-scroll`, `html { scroll-padding-block-start }` | 이름 바꾸기. `--z-rank-header`, `--z-scroll-indicator`, `--checker-list-bottom-padding` 토큰도 함께 |
| `app/globals.css` 291-370 | 도메인 색 토큰과 `.checker-lamp` | 값은 3.1.3 의 스킨 색으로. 점멸 규칙(348-370)은 가져오지 않음. 구분선(323)만 |
| `app/globals.css` 214-240 | `@layer base` 안의 스크롤바 숨김과 모션 감소 | R-BMS 는 같은 규칙이 레이어 밖(287-307)에 있음. 그대로 둬도 동작은 같음 |
| `widgets/app-shell/app-shell.tsx` | 셸 전체 | 언어 컨트롤(76-119)과 개인정보 링크(246-250) 삭제. 계정 메뉴 항목을 내 페이지·내 폴더·설정·로그아웃으로. `authClient` 는 `web:shared/lib/auth-client.ts`. 한 파일에 컴포넌트가 넷이라 R-BMS 에서는 파일을 나눔. `useIdentityTransition`·`AuthDialogWidget` 은 1차에서 빼고 "로그인"은 `/login?next=` 링크로 |
| `widgets/app-shell/shell-nav-menu.tsx`, `shell-nav.ts` | 메뉴와 항목 표 | 항목 표는 이미 있는 `web:shared/constants/navigation.ts` 를 고쳐 씀(아이콘을 문자열 키 대신 컴포넌트로) |
| `widgets/app-shell/shell-page-header.tsx`, `shell-sidebar-portal.tsx` | 페이지 머리, 슬롯 포털 | 문구만 |
| `shared/providers/shell-sidebar-slot-context.ts`, `shared/hooks/use-shell-sidebar-slot.ts` | 슬롯 컨텍스트 | 그대로 |
| `shared/ui/sidebar.tsx` (525줄) | 수정된 shadcn 사이드바 | `useTranslations` 한 곳을 문구로. `ScrollContainer`, `use-mobile` 에 의존 |
| `shared/hooks/use-mobile.ts` | 768px 판정 | 그대로 |
| `shared/ui/scroll-container.tsx`, `scroll-indicator.tsx`, `shared/hooks/use-scroll-thumb.ts`, `shared/lib/scroll-thumb.ts`, `shared/lib/__tests__/scroll-thumb.test.ts` | 내부 스크롤과 3px 막대, 계산 함수와 테스트 | 상수 3개(`SCROLL_INDICATOR_*`)를 함께. 테스트 설명은 이미 한국어인지 확인 후 옮김 |
| `shared/constants/ui.ts` | 셸 치수 상수 | `THEME_STORAGE_KEY` 는 R-BMS 의 `rbms-theme` 유지 |
| `shared/providers/app-toaster.tsx` | 토스트 모양 | `containerAriaLabel` 문구만. `web:shared/ui/sonner.tsx` 를 대체 |
| `shared/providers/app-providers.tsx` 25 | `ThemeProvider` 설정 | `defaultTheme='dark'`, `attribute` 는 R-BMS 의 `['class','data-theme']` 유지 |
| `shared/ui/button.tsx` 7, 28-30 | 터치 영역 확대 클래스 | R-BMS `button.tsx` 26-29 의 `icon*` 크기에 같은 클래스 추가 |
| `shared/ui/collapsible*.tsx` (3파일) | 접기 | 그대로 |
| `features/chart-rank-section/chart-rank-section.tsx` | 접히는 묶음 + 붙는 머리 | "지력/개인차" 소묶음(57-77) 삭제. 머리에 `LampStackBar` 와 "n / 전체" 추가. 랭크 → 레벨 |
| `features/chart-card-grid/chart-card-grid.tsx`, `shared/constants/checker.ts` 3, 13-19 | 격자와 빈칸 채움 | 열 수를 2·3·4·5·6 으로, 빈칸 표도 같이 |
| `features/chart-card-grid/chart-card.tsx` | 카드 | 길게 누르기·오른쪽 클릭·램프 올리기(39-64, 115-150) 삭제, 누르면 상세 열기만. 로고 배경(70-79) 삭제. 난이도 선 삭제. 아랫줄을 아티스트 + 랭크로. 네이티브 `title`(92-102) 삭제(R-BMS 규칙) |
| `features/checker-skeleton/chart-rank-skeleton.tsx` | 묶음 모양 스켈레톤 | 소묶음 삭제 |
| `features/checker-filters/checker-filters.tsx` | 필터 폼 틀 | 필드를 검색·레벨·램프·정렬로 |
| `features/chart-details/chart-details.tsx` | 상세 대화상자의 틀(제목 줄바꿈 50, 값 상자 115-131) | 편집 폼을 읽기 전용 "내 기록"으로 |
| `features/lamp-swatch/lamp-swatch.tsx`, `shared/constants/dashboard.ts` 14-23 | 견본과 램프 → 배경 표 | 표를 3.1.3 의 11종으로. 그라데이션 두 줄은 필요 없음 |
| `features/dashboard-lamp/lamp-distribution.tsx` | 큰 숫자 + 누적 막대 + 개수 목록 | 막대 부분(34-45)을 `LampStackBar` 로 떼어 묶음 머리와 함께 씀 |
| `features/dashboard-lamp/rank-lamp-list.tsx` | 줄마다 이름 + 누적 막대 + "n/전체" | 플레이어 페이지의 레벨별 램프 목록에 |
| `features/bar-count-list/bar-count-list.tsx` | 이름 + 막대 + 값 | `web:features/stats/bar-count-list.tsx` 를 대체 |
| `features/dashboard-panel/*.tsx` (5파일), `shared/constants/dashboard.ts` 27-39 | 패널 틀과 상태, 격자 클래스 | 문구만 |
| `features/dashboard-activity/recent-record-row.tsx` | 램프 띠가 있는 2줄 행 | 난이도 선 삭제, 아랫줄을 플레이어 이름으로 |
| `features/fitted-row-list/fitted-row-list.tsx` | 들어가는 행만 보이기 | 그대로 |
| `widgets/user-list/user-list.tsx`, `widgets/board-list/board-post-list-item.tsx` | 행 목록과 행 전체 링크 | 표 목록·폴더 목록의 본 |
| `widgets/board-list/board-frame.tsx` | 페이지 틀(머리 + 스크롤) | 그대로 본으로 |
| `features/board-pagination/*.tsx`, `get-pagination-items.ts` 와 테스트 | 번호 페이지네이션 | 문구만 |
| `features/profile-header/profile-header.tsx`, `widgets/user-page/user-page.tsx` 91-102 | 프로필 블록, 주소에 반영되는 탭 | 아바타 삭제 |
| `features/profile-settings-section/profile-settings-section.tsx` | 설정 구역 틀 | 그대로 |
| `features/privacy-document/privacy-section.tsx`, `privacy-table.tsx`, `privacy-list.tsx` | 문서 절·표·목록 | 가이드에 |
| `shared/hooks/use-unsaved-changes-warning.ts` | 미저장 경고 | 그대로 |
| `features/not-found-notice/not-found-notice.tsx`, `app/[locale]/(shell)/error.tsx` | 404·오류 본문 | 3개 언어 병기를 한국어 하나로 |
| `app/[locale]/(shell)/table/page.tsx` 30-79 | 정적 셸 + 세션 경계 + 프리페치의 3단 `Suspense` | 표시 설정 경계는 필요 없음 |

가져오지 않는 것: 에디터(tiptap), 게시판, e-amusement 가져오기, 시리즈 로고, 램프 점멸, 언어 전환, JSON-LD(2차에서 검토).

## 4. 우선순위

### 4.1 1차 (한 웨이브: 골격·토큰·난이도표·폴더)

| 묶음 | 내용 | 먼저 끝나야 하는 것 |
|---|---|---|
| A 토큰 | `globals.css`: 다크 값 맞춤(3.1.2), `--rbms-*` 색(3.1.3), `rbms-*` 클래스, z-index 2개, 공개 Surface 델타 삭제. `providers.tsx`: 기본 다크, `AppToaster`. `clear-lamp.ts`: 표시 이름과 `LAMP_BACKGROUND`. `dj-rank.ts` | 없음 |
| B 골격 | shadcn `sidebar`·`collapsible` 등 추가, `AppShell`·머리·슬롯·`ScrollContainer`, 모바일 문서 스크롤, 메뉴 순서, 계정 컨트롤, `title.template`, `not-found.tsx`·`error.tsx`, 로그인·가입·가이드를 셸 안으로 | A 의 클래스와 토큰 |
| C 기존 화면 얹기 | 홈·검색·순위·차트·플레이어·설정에 `ShellPageHeader` 를 얹고 `PageHeading` 삭제, `LampBadge` 를 견본 + 이름으로, 빈·오류 상태에 동작·재시도. 재구성은 하지 않음 | B |
| D 난이도표 | `/tables` 목록, 발광 표 페이지(묶음·격자·카드·상세·필터·사이드바 패널·내 램프·스켈레톤·출처 표기) | 3.2.4 의 계약(B 와 병렬 가능), 표 자료와 내 기록 일괄 조회 API |
| E 폴더 | `/folders` 목록, 새 폴더, 편집(목록·검색 패널·위아래·삭제·표에서 가져오기·명시 저장·미저장 경고), 삭제 확인 | 3.2.4 의 계약, 폴더 API, 차트 검색 API |

파일이 겹치는 곳: `globals.css` 와 `app/layout.tsx`·`providers.tsx` 는 A·B 만, `shared/constants/navigation.ts` 는 B 만 고칩니다. D 와 E 는 `features/table`, `features/folder`, `widgets/table`, `widgets/folder`, `entities/{table,folder}.*`, `app/(app)/{tables,folders}` 로 나뉘어 겹치지 않습니다. `features/lamp/*` 는 A 또는 B 가 먼저 만들어 D·E·C 가 가져다 씁니다.

검증(1차 끝): `bun run verify`(typecheck·lint·test) 한 번, 그리고 브라우저에서 1280px·390px 두 폭으로 다크·라이트를 열어 가로 넘침 0, 붙는 머리 위치, 램프 색을 확인합니다.

진행(2026-10-14): A·B·C 는 웨이브 17B-1, D·E 는 웨이브 17B-2 에서 끝났습니다. 구현과 계약은 `docs/web/2026-10-14-foundation-contract.md` 13절, 검증은 14절입니다. 이 문서와 달라진 곳:

- D: 3.3.3 의 머리 동작(검색·모두 접기·레벨 이동·필터)은 머리가 아니라 머리 아래 필터 막대와 레벨 칩 한 줄에 두었습니다. 로그인한 사용자의 "내 진행"은 본문 위의 접히는 패널(기본 접힘)이고, 사이드바 슬롯에는 출처와 표 주소 복사만 둡니다(리뷰에서 진행과 복사 버튼이 사이드바와 본문에 중복되어 첫 카드가 900px 화면의 y 541 까지 밀렸고, 고친 뒤 y 289). 카드에는 레벨을 쓰지 않습니다(묶음 머리가 보임). 상세 대화상자의 "폴더에 추가"(4.2 의 5 일부)는 1차에서 넣었습니다.
- E: 끌어 옮기기 대신 위·아래 버튼과 메뉴의 맨 위·맨 아래입니다(4.2 의 6).

### 4.2 2차

1. 홈 대시보드 전체(로그인 패널 4종, 컨테이너 쿼리 배치, `FittedRowList`).
2. 차트 페이지 재구성(제목 블록, 요약 줄, 내 행 강조, 모바일 열).
3. 플레이어 페이지 탭과 레벨별 램프, 순위의 번호 페이지네이션.
4. 설정의 구역화와 탭 주소 반영, 검색 화면 필터를 머리 + 대화상자로(우측 `FilterPanel` 삭제).
5. 난이도표: 한 곡 한 행 보기 전환, 카드 높이 선택(아티스트 숨김 40px), 접힘 상태 기억, 모바일 레벨 칩 선택기, 카드 목록으로 건너뛰기. (상세 대화상자의 "폴더에 추가"는 17B-2 에서 끝남)
6. 폴더: 끌어 옮기기, 내 램프 표시, 복제, 2,000곡 폴더의 행 가상화, 저장 한 요청화.
7. 사이드바의 로그인 대화상자.
8. 메타: openGraph, `robots.ts`, `sitemap.ts`, 아이콘.
9. 검토: Inter 글꼴. (카드 1,035장의 렌더 비용은 17B-2 에서 재고 늦게 그리기로 줄임: HTML 1,498KB → 669KB, 모바일 4배 감속 가장 긴 작업 455ms → 204ms)

## 5. 다음 작업자가 알아 둘 사실

1. 기본 토큰은 두 사이트가 같습니다. 고칠 것은 3.1.2 의 다크 값 다섯 줄과 3.1.3 의 도메인 색입니다.
2. shadcn `sidebar` 는 `web/src/shared/ui` 에 없습니다(문서와 다름). 지금 사이드바는 직접 만든 `nav` 입니다.
3. 발광 표 자료에는 `sha256` 이 없고 `md5` 만 있습니다. 내 기록 조회와 차트 링크는 md5 기준이어야 합니다.
4. 표의 `name_diff` 에는 이름 대신 주소가 든 항목이 있습니다. 링크 주소는 대부분 `http://` 입니다.
5. 앱에는 램프 색이 두 벌(`theme.lua`, `format.rs`) 있고 값이 다릅니다. 이름도 다릅니다(웹 "NORMAL", 앱 "CLEAR").
6. 랭크는 앱이 27단계(`grade.rs`)로 계산합니다. 웹에 같은 계산이 아직 없습니다.
7. 운영 사이트에는 시험 계정과 제목 없는 차트가 보입니다. `/tables` 는 비어 있습니다.
8. 운영의 404 는 Next 기본 화면이고, 모든 페이지 제목이 `rbms IR` 입니다.
9. R-BMS 웹은 모바일에서도 화면 고정 + 내부 스크롤입니다. iidx-rank 가 Safari 때문에 문서 스크롤로 바꾼 구조입니다.
10. R-BMS 웹은 스크롤바를 숨기기만 하고 대신 보여 주는 표시가 없습니다.

## 6. 결정이 필요한 것

| 번호 | 결정 지점 | A안 | B안 |
|---|---|---|---|
| 1 | Surface 를 하나로 합칠지 | 합친다(추천: 앱의 평면·사각 정체성과 iidx-rank 의 일관성, 공개 화면에서도 메뉴 유지) | 두 Surface 유지 |
| 2 | 기본 테마 | 다크 기본 + 토글(추천: 앱·스킨이 검정 바탕) | 시스템 추종(지금) |
| 3 | 램프·난이도 색의 기준 | 기본 스킨 `theme.lua`(추천: 사용자가 보는 색, 다크에서 전부 읽힘) | `format.rs`(FAILED·MAX 가 다크 카드 위에서 안 보임) |
| 4 | 발광 표의 기본 보기 | 레벨별 카드 격자(추천: 1,035곡을 훑기 좋고 iidx-rank 에서 사용자가 행을 카드로 바꾼 전례) | 한 곡 한 행의 표(아티스트·링크·점수가 한눈에, 대신 약 2.6배 길어짐) |
| 5 | 폴더 편집의 저장 | 명시 저장 + 미저장 경고(추천: 순서 변경을 한 번에 보내고 충돌 처리가 단순) | 변경마다 자동 저장 |
| 6 | 폴더의 곡 검색 UI | 의존성 없이 입력 + 결과 목록(추천: 늘 열린 패널이라 명령 팔레트가 필요 없음) | shadcn `command`(`cmdk` 추가) |
| 7 | 사이트 이름 표기 | `R-BMS`(추천: 제품 이름과 같음) | `rbms IR`(지금) |
| 8 | 운영 DB 의 시험 계정·제목 없는 차트 | 지운 뒤 공개(추천. 운영 DB 쓰기라 메인·사용자가 실행) | 화면에서 숨기는 조건 추가 |
| 9 | 램프 표시 이름 | 앱과 같게 `Normal` → "CLEAR"(추천) | "NORMAL" 유지 |
| 10 | 설정 메뉴 위치 | 계정 드롭다운으로(추천: iidx-rank 와 같음) | 사이드바 메뉴 유지 |

## 7. 확인하지 못한 것

| 항목 | 까닭 |
|---|---|
| 운영 사이트와 iidx-rank 의 실제 렌더(다크·라이트, 모바일 폭) | HTML 과 CSS 를 글자로만 받았고 브라우저로 열지 않았습니다. iidx-rank 는 `docs/assets/screenshot.png`(라이트, 1440×900) 한 장만 봤습니다 |
| 로그인한 상태의 화면(설정, 토큰) | 쓰는 요청이 금지라 로그인하지 않았습니다. 코드로만 읽었습니다 |
| 카드 1,035장의 렌더·스크롤 성능 | 측정하지 않았습니다. iidx-rank 는 667장을 가상화 없이 그립니다 |
| 내려받기 주소와 저장소 공개 여부 | 릴리스 담당의 영역이고 확인하지 않았습니다 |
| 표·폴더·내 기록 일괄 조회의 API 모양 | 같은 웨이브의 다른 문서가 정합니다. 여기에는 화면이 필요로 하는 자료만 적었습니다 |
| `theme.lamp_cycle = 50` 의 뜻(스킨 램프의 점멸 주기인지) | 스킨 엔진을 읽지 않았습니다. 웹은 점멸을 쓰지 않으므로 영향 없음 |
| 운영 `/tables` 의 DB 표와 정적 스냅샷 표를 한 목록으로 합치는 방식 | 표 담당 문서의 결정 |
| iidx-rank `sidebar.tsx` 가 shadcn 원본에서 정확히 무엇이 다른지 | import 줄만 견줬습니다(`ScrollContainer`, `useTranslations` 가 추가됨). 전체 비교는 하지 않았습니다 |
| 색 대비 수치의 화면 검증 | sRGB 값으로 계산한 값입니다. 실제 화면에서 재지 않았습니다 |
