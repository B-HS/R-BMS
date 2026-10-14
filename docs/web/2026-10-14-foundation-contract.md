# 웹 기반 계약 (웨이브 17B-1 통합, 17B-2 반영)

> 최종 갱신 2026-10-14 · 대응 커밋 7095ef7 (웨이브 20 의 SCRATCH AUTO 순위는 6.6절에 반영돼 있고, 12절·14절의 테스트 수와 측정값은 각 웨이브 종료 시점의 기록이라 지금 값과 다를 수 있다)

> 현재 상태: 토큰·골격·기존 화면(17B-1 U1)과 서버·폴더·표 제공(17B-1 U2) 위에 17B-2 가 발광 표 화면(U3)과 폴더 화면(U4)을 올렸고, 리뷰 발견을 고친 뒤 게이트와 화면 확인을 다시 통과했습니다. 17B-2 에서 바뀐 계약은 해당 절을 고쳤고, 새 화면·컴포넌트·쿼리 키·검증은 13절과 14절에 모았습니다.
> 작성일 2026-10-14. 근거는 실제 파일과 로컬 실행(포트 3103, 임시 MySQL 8.4)의 응답이며, 두 단위의 보고와 달랐던 곳은 11절에 적었습니다. 문서와 파일이 다르면 파일이 맞습니다.
> 결정의 정본은 `docs/acknowledge/2026-10-14-web-decisions.md` 입니다. Q2 는 발광 표 기본 보기가 카드 격자(A), Q3 은 폴더 v1 전부 비공개(A)로 정해졌습니다.

## 0. 먼저 알아 둘 것

1. 모든 화면은 `AppShell`(루트 `layout.tsx`) 안에서 `ShellPage` 한 개를 반환합니다. 머리(`h1`)·스크롤·모바일 메뉴 버튼은 `ShellPage` 가 가집니다(3절).
2. 새 보호 경로(`/folders/...`)는 `src/proxy.ts` 의 matcher 와 `PROTECTED_PATH_PREFIXES` 에 이미 들어 있습니다. 서버 확인은 해당 경로의 `layout.tsx` 에서 `SessionGate` 로 한 번 더 합니다(3.4절).
3. 서버 API(6절)와 화면이 모두 있습니다. `/tables/insane1`(정적 페이지가 동적 `[tableId]` 보다 우선), `/folders`, `/folders/[folderId]` 의 구성은 13절입니다.
4. 발광 표(`insane1`)는 DB 가 아니라 저장소의 스냅샷(`web/data/tables/insane1/`)에서 나옵니다. 화면은 DB 의 `/api/tables`(IR 표) 가 아니라 `/api/fe/tables`(스냅샷)를 읽어야 합니다. 두 갈래의 쿼리 키·타입이 따로 있어야 합니다(7절).
5. 웹 로그인과 가입은 세션 전용 경로(6.1절)입니다. 게임용 `/api/auth/login`·`/register` 는 토큰을 발급하며 웹이 부르지 않습니다.

## 1. 경로 지도

| 구분 | 경로 | 비고 |
|---|---|---|
| 토큰·유틸 클래스 | `web/src/app/globals.css` | `--rbms-*`, `.rbms-*` |
| 골격 | `web/src/widgets/shell/*` | `AppShell`, `ShellPage`, `ShellPageHeader`, `ShellPageLoading`, `ShellNavMenu`, `ShellAccountControl`, `ShellThemeControl`, `ShellSidebarPortal` |
| 공통 상태·패널 | `web/src/features/{state,dashboard,dialog,filter,lamp,score,shell,table}/*` | 4절 |
| shadcn | `web/src/shared/ui/*` | 생성물은 그대로 둡니다 |
| 훅·상수·함수 | `web/src/shared/{hooks,constants,lib,providers,utils}/*` | 5절 |
| 인증 | `web/src/entities/auth.{type,query}.ts`, `web/src/widgets/auth/*`, `web/src/proxy.ts` | 3.4, 6.1절 |
| 서버 | `web/src/server/{route,service/domain,compose,dto,lib}/*`, `web/src/app/api/**` | 6절 |
| 표 스냅샷 | `web/data/tables/insane1/{header.json,data.json,source.json}` | 서버만 읽습니다 |
| 마이그레이션 | `web/drizzle/{0000_smooth_cerebro,0001_user_folders,0002_scratch_auto_bests}.sql`, `web/drizzle/meta/*` | 커밋 대상. 셋 다 운영 DB 에 적용됨(2026-10-14) |

## 2. 토큰

값은 `globals.css` 에 있습니다. `:root` 가 라이트 값, `.dark` / `[data-theme='dark']` 가 다크 값이고 `next-themes` 의 기본 테마가 다크(`defaultTheme='dark'`, 시스템 설정 무시, 저장 키 `rbms-theme`)입니다. 바탕은 앱 무채색 단계(다크 0 / 0.098 / 0.162 / 0.212 계열)이고 색은 램프·판정·난이도 표시에만 씁니다.

| 분류 | 이름 | 값 |
|---|---|---|
| 램프 | `--rbms-lamp-noplay` `failed` `assist` `light-assist` `easy` `normal` `hard` `exhard` `fullcombo` `perfect` `max` | `#444444` `#ff0000` `#ff63fd` `#6563ff` `#00ff24` `#ffbb00` `#dfdfdf` `#ff6e6e` `#00dfff` `#f499f2` `#eaf300` |
| 판정 | `--rbms-judge-pgreat` `great` `good` `bad` `poor` | `#6ba6f5` `#f6ab22` `#f9b21f` `#fb8116` `#f25200` |
| 타이밍 | `--rbms-timing-fast` `slow` | `#03a9ff` `#ff4503` |
| 난이도 | `--rbms-difficulty-unknown` `beginner` `normal` `hyper` `another` `insane` | `#c3c3c3` `#06ff00` `#12d2d7` `#ffc000` `#ff0000` `#a840aa` |
| 치수 | `--sidebar-width` `--sidebar-width-icon` `--sidebar-width-mobile` `--panel-inset` `--panel-gutter` `--radius` | `16rem` `3rem` `18rem` `12px` `1px` `0rem` |
| 글자 | Tailwind `text-2xs` | `0.6875rem`(11px) |
| 층 | `--z-rail` `--z-raised` `--z-level-header` `--z-scroll-indicator` `--z-overlay` | `10` `20` `30` `40` `50` |

- Tailwind 색 별칭은 `bg-lamp-noplay`, `text-judge-*`, `text-timing-*` 뿐입니다. 나머지 램프·난이도 색은 `style={{ background: 'var(--rbms-lamp-hard)' }}` 로 씁니다. 램프 이름에서 값으로 가는 표는 `LAMP_BACKGROUND`(`@shared/constants/clear-lamp`) 입니다.
- 클래스: `.rbms-page`(페이지 틀), `.rbms-page-header`(48px 머리), `.rbms-level-header`(붙는 묶음 머리. 모바일은 `top: var(--rail-chrome-height)` 에 z 10(`--z-rail`)이라 밀려 올라갈 때 모바일 페이지 머리(z 20) 아래로 들어가고, `md` 이상은 `top: 0` 에 z 30. 발광 표의 `TableLevelSection` 이 씀), `.rbms-list-scroll`(아래 9rem 여백), `.rbms-micro-label`(11px 대문자 라벨), `.rbms-empty`, `.rbms-lamp`(램프 띠 기본 배경). `html` 에 `scroll-padding-block-start: calc(var(--rail-chrome-height) * 2)`(96px, 붙는 머리 두 개) 가 있고 `--rail-chrome-height` 는 `3rem`, `--list-bottom-padding` 은 `9rem` 입니다.
- 라이트 `--muted-foreground` 는 `oklch(0.48 0 0)`(`--palette-neutral-480`)입니다. 17B-2 리뷰에서 전의 `0.556` 이 사이드바 위 11px 라벨 3.69:1, `bg-muted` 위 12px 숫자 4.35:1 로 4.5:1 에 못 미쳐 낮췄고, 고친 뒤 같은 두 곳이 5.13:1·6.04:1 입니다(다크는 그대로, 8.13:1·5.86:1).
- 라이트 테마에서 `hard`·`max`·`easy` 램프는 흰 바탕 대비가 낮습니다. 색만으로 뜻을 전하지 말고 견본 옆에 이름을 함께 씁니다(`LampLabel` 이 그렇게 합니다). 견본은 40% 전경 테두리를 가집니다.
- Normal 클리어의 표시 이름은 `CLEAR`(`CLEAR_LAMP_LABEL.Normal`)입니다. 값 이름(`'Normal'`)은 API·DB 와 같습니다.

## 3. 화면을 얹는 방법

### 3.1 한 페이지의 모양

```tsx
import { Suspense } from 'react'
import { buildPageMetadata } from '@shared/lib/metadata'
import { PanelSkeleton } from '@features/state/panel-skeleton'
import { ShellPage } from '@widgets/shell/shell-page'
import { FolderListContent } from '@widgets/folder/folder-list-content'

export const metadata = buildPageMetadata({ title: '내 폴더', description: '내가 만든 곡 폴더.', index: false })

const FoldersPage = () => (
    <ShellPage title='내 폴더' description='R-BMS 선곡 화면이 읽는 폴더'>
        <Suspense fallback={<PanelSkeleton height='page' label='폴더 불러오는 중' />}>
            <FolderListContent />
        </Suspense>
    </ShellPage>
)

export default FoldersPage
```

- 페이지 머리·`h1` 은 `ShellPage` 만 만듭니다. 본문 안에서 `h1` 을 또 쓰지 않습니다.
- 제목이 데이터에 달린 화면(차트 상세)은 `ShellPage` 를 클라이언트 패널 안에서 데이터가 온 뒤에 그립니다(`widgets/chart/chart-leaderboard-panel.tsx`). 그 전에는 `ShellPageLoading` 을 `Suspense` 의 fallback 으로 씁니다.
- Cache Components(`cacheComponents: true`) 규칙: 요청 값(`searchParams`, `cookies()`, `headers()`)을 읽는 서버 컴포넌트는 `Suspense` 안에 둡니다. 안 그러면 빌드가 실패합니다. 서버에서 데이터를 미리 채우는 `*-content.tsx` 는 `getQueryClient()` → `prefetchQuery` → `HydrationBoundary` 의 틀을 따릅니다(`widgets/leaderboard/player-leaderboard-content.tsx`).
- 없는 대상은 서버에서 `fetchQueryOrNotFound(queryClient, options)`(`@shared/lib/fetch-query-or-not-found`)로 `notFound()` 를 냅니다. API 404 만 `notFound()` 가 되고 그 밖의 실패는 `null` 을 돌려 클라이언트가 재시도하게 둡니다. 동적 하위 경로의 not-found 는 스트리밍이라 HTTP 200 으로 나가고 `noindex` 가 붙습니다(문서에 적힌 동작).
- 메인 영역 배치: `ShellPage` 의 본문은 `flex flex-col gap-px pb-12 *:shrink-0` 이고 패널은 `bg-card` 를 가져 1px 틈으로 갈라집니다. 본문이 스크롤하므로 직계 자식은 줄어들지 않고 제 높이로 흐릅니다(17B-2 리뷰: 데스크톱 `/players/<id>` 에서 `DashboardPanel` 이 눌려 범례와 기록이 잘리고 스크롤되지 않던 원인). 안쪽 스크롤 칸이 필요한 화면(폴더 편집)은 `ShellPage` 대신 `section.rbms-page` + `ShellPageHeader` 를 직접 씁니다. 본문 클래스를 바꾸려면 `contentClassName` 을 넘깁니다. 홈 같은 패널 격자는 부모에 `@container/dashboard` 를 달고 `DASHBOARD_GRID_CLASS_NAME` 을 씁니다.
- 접힘·붙는 머리: 묶음 머리는 `className='rbms-level-header ...'` 를 달면 모바일은 문서 스크롤 위쪽 48px 아래에, 데스크톱은 안쪽 스크롤 맨 위에 붙습니다. 데스크톱의 안쪽 스크롤은 `ScrollContainer`(`ShellPage` 가 감쌈)입니다. 모바일은 문서가 스크롤합니다.

### 3.2 레이아웃과 메뉴

- `app/layout.tsx` → `Providers`(QueryClient, 테마, 토스트) → `AppShell`. 라우트 그룹 `(app)`·`(public)` 의 `layout.tsx` 는 지웠습니다. 새 화면은 그룹을 가리지 않고 `src/app/(app)/<경로>/page.tsx` 에 둡니다(`/folders`, `/tables/insane1`).
- 메뉴 항목은 `NAV_ITEMS`(`@shared/constants/navigation`): 홈 `/`, 차트 검색 `/search`, 순위 `/leaderboards`, 가이드 `/guide`, 내 폴더 `/folders`, 난이도표 `/tables`. 활성 표시는 `ShellNavMenu` 가 경로 접두사로 계산합니다.
- 계정 메뉴(바닥): 내 페이지 `/players/<login_id>`, 내 폴더, 설정, (관리자) 빌드 관리, 로그아웃. 비로그인이면 `로그인` 링크(`/login?next=`).

### 3.3 사이드바 슬롯

`ShellSidebarPortal` 안에 요소를 두면 메뉴 아래 슬롯으로 올라갑니다(발광 표의 출처·표 주소 복사, 폴더의 "R-BMS 에서 보기"). 데스크톱에서만 사이드바에 보이고 모바일은 시트 안에 들어갑니다. 서버 렌더에서는 비어 있다가 하이드레이션 뒤에 채워집니다(`useIsHydrated`, `@shared/hooks/use-is-hydrated`).

```tsx
<ShellSidebarPortal>
    <nav aria-label='레벨 이동'>...</nav>
</ShellSidebarPortal>
```

### 3.4 보호 경로

- `src/proxy.ts`(Next 16 의 proxy): `/settings`, `/admin`, `/folders` 아래에서 세션 쿠키(`better-auth.session_token`)가 없으면 `307 /login?next=<경로>` 로 보냅니다. 쿠키 존재만 보는 낙관적 확인입니다.
- 서버 확인은 해당 경로 `layout.tsx` 의 `<Suspense fallback={<ShellPageLoading title='...' />}><SessionGate next='/folders'>{children}</SessionGate></Suspense>` 입니다(`/settings/layout.tsx` 가 본). 관리자 전용은 `requireAdmin` 을 더합니다. `readSessionUser()`(서버 전용)는 `SessionUser | null` 을 돌려줍니다.
- 새 보호 경로가 `/settings` `/admin` `/folders` 밖이면 `proxy.ts` 의 `matcher` 와 `PROTECTED_PATH_PREFIXES` 둘 다에 더합니다.
- 서버의 모든 API 에서 인가는 다시 확인됩니다. 화면의 가림은 편의입니다.

### 3.5 메타데이터

`export const metadata = buildPageMetadata({ title, description?, path?, index? })`(`@shared/lib/metadata`). 제목은 `%s | R-BMS` 템플릿에 들어갑니다. 비공개·사본 화면은 `index: false`(noindex, nofollow) 를 줍니다. 발광 표 사본 페이지는 `index: false` 이고 사이트맵(`SITEMAP_STATIC_PATHS`)에 넣지 않습니다. 사이트맵·robots 의 목록은 `@shared/constants/site` 에 있습니다.

표준 BMS 표 자동 발견용 메타는 페이지 `metadata` 의 `other` 로 냅니다. 루트 상대 경로여야 합니다(운영이 끝 슬래시를 308 로 떼므로 상대 해석이 흔들리지 않게).

```ts
export const metadata = {
    ...buildPageMetadata({ title: '発狂BMS難易度表', path: '/tables/insane1', index: false }),
    other: { bmstable: '/tables/insane1/header.json' },
}
```

## 4. 공통 컴포넌트

모두 named export, `FC<Props>`, 1파일 1컴포넌트입니다. 경로는 `src/` 기준입니다.

### 4.1 골격

| 이름 | 경로 | props |
|---|---|---|
| `ShellPage` | `widgets/shell/shell-page.tsx` | `{ title: string; description?: string; actions?: ReactNode; contentClassName?: string; children }` |
| `ShellPageLoading` | `widgets/shell/shell-page-loading.tsx` | `{ title: string; description? }` |
| `ShellSidebarPortal` | `widgets/shell/shell-sidebar-portal.tsx` | `{ children }` |

### 4.2 패널과 상태

| 이름 | 경로 | props |
|---|---|---|
| `DashboardPanel` | `features/dashboard/dashboard-panel.tsx` | `{ title; badge?: ReactNode; actions?: ReactNode; className?; bodyClassName?; children }` (36px 머리 + 본문, `section` + `aria-labelledby`) |
| `DashboardPanelEmpty` | `features/dashboard/dashboard-panel-empty.tsx` | `{ message: string; children? }` |
| `DashboardPanelError` | `features/dashboard/dashboard-panel-error.tsx` | `{ message: string; isRetrying?: boolean; onRetry?: () => void }` |
| `DashboardPanelLoading` | `features/dashboard/dashboard-panel-loading.tsx` | `{ label: string; className? }` |
| `EmptyState` | `features/state/empty-state.tsx` | `{ title; description?; eyebrow?; action?: ReactNode; className? }` |
| `LoadErrorState` | `features/state/load-error-state.tsx` | `{ title; description?; isRetrying?; onRetry?; className? }` |
| `NotFoundNotice` | `features/state/not-found-notice.tsx` | `{ title?; description?; actionHref?; actionLabel? }` (기본 문구: 404 안내 + 홈으로) |
| `RowListSkeleton` | `features/state/row-list-skeleton.tsx` | `{ rows?: number(3); label?: string }` |
| `PanelSkeleton` | `features/state/panel-skeleton.tsx` | `{ height?: 'panel' \| 'table' \| 'page'; label?: string }` |
| `TableSkeleton` | `features/state/table-skeleton.tsx` | `{ rows?: number(6); label?: string }` |
| `PanelCard` / `StatTile` / `PanelFootnote` | `features/shell/*` | `{ title?; action?; contentClassName?; className? }` / `{ label; value; hint?; accent?: 'warning' \| 'danger' }` / children |
| `SettingsSection` | `features/shell/settings-section.tsx` | `{ title; description?; action?: ReactNode; children }` |

패널의 상태 순서는 대기 → 전송 오류 → 빈 → 결과입니다. 자료가 바뀌는 동안 "결과 없음"이 깜빡이지 않게 `isLoading` 을 먼저 봅니다(`widgets/leaderboard/player-leaderboard-panel.tsx` 가 본). 패널 단위 오류는 `DashboardPanelError`, 화면 단위는 `LoadErrorState`.

### 4.3 입력·대화상자

| 이름 | 경로 | props |
|---|---|---|
| `FilterBar` | `features/filter/filter-bar.tsx` | `{ label: string; summary?: ReactNode; className?; children }` (머리 아래 줄, `role='group'`) |
| `FilterField` | `features/filter/filter-field.tsx` | `{ label; htmlFor?; children }` |
| `FormDialog` | `features/dialog/form-dialog.tsx` | `{ open; onOpenChange; title; description?: ReactNode; submitLabel; submitDisabled?; onSubmit: () => void; size?: 'md' \| 'lg'('lg'); returnFocusRef?; children }` |
| `ConfirmAction` | `features/dialog/confirm-action.tsx` | `{ open; onOpenChange; title; description: ReactNode; confirmLabel; confirmDisabled?; cancelLabel?; returnFocusRef?; onConfirm }` (파괴 동작용, 확인 버튼 destructive) |
| `CopyButton` | `shared/ui/copy-button.tsx` | Button 의 props(`onClick`·`children` 제외) + `{ value: string; label: string; copiedMessage?; showLabel? }` |
| `SortSelect` | `features/table/sort-select.tsx` | `{ label; value; options: { value; label }[]; onValueChange }` |
| `Pager` | `features/table/pager.tsx` | `{ page; totalPages; onPageChange }` (이전·다음 + "n / N") |
| `DataTable` | `features/table/data-table.tsx` | `{ columns: DataTableColumn<T>[]; rows: T[]; rowKey; rowMuted?; caption? }`, 열은 `{ key; label; width?; align?; flex?; mono?; hideBelow?: 'sm' \| 'md' \| 'lg'; cell(row, index) }` |

`FormDialog`·`ConfirmAction` 은 상태로 열립니다(트리거 없이 `open` 으로). 닫을 때 `useDialogFocusReturn`(`features/dialog/use-dialog-focus-return.ts`)이 `returnFocusRef` 를 주면 그 요소로, 없으면 연 순간의 포커스 요소가 아직 DOM 에 있을 때 그 요소로 포커스를 돌려줍니다. 메뉴 항목처럼 닫히면 사라지는 요소에서 열면 메뉴 트리거의 ref 를 `returnFocusRef` 로 넘깁니다. 기존 호출부는 고치지 않아도 됩니다.

### 4.4 램프·점수

| 이름 | 경로 | props |
|---|---|---|
| `LampSwatch` | `features/lamp/lamp-swatch.tsx` | `{ lamp: ClearLamp; className? }` 10px 견본 |
| `LampLabel` | `features/lamp/lamp-label.tsx` | `{ lamp; className? }` 견본 + 이름 |
| `LampStrip` | `features/lamp/lamp-strip.tsx` | `{ lamp; className? }` 5px 세로 띠 (`self-stretch`) |
| `LampStackBar` | `features/lamp/lamp-stack-bar.tsx` | `{ lamps: LampCount[]; className? }`, `LampCount = { lamp: ClearLamp; count: number }`. `role='img'` + 개수 읽는 `aria-label`. 높이 12px. 카드 격자의 레벨 묶음 머리가 이것을 씁니다 |
| `LampDistribution` | `features/lamp/lamp-distribution.tsx` | `{ lamps: LampCount[]; played: number; total: number; playedLabel?; rateLabel? }` |
| `ScoreRow` | `features/score/score-row.tsx` | `{ href; title; subtitle?; subtitleHref?; lamp; exScore; minBp; playedAt }` (`<li>`, 부모는 `<ul>`) |

`ClearLamp` 는 11값(`NoPlay` … `Max`)이고 서버의 `CLEAR_LAMPS`(`@server/dto/common`)와 같은 순서·철자입니다(`LampCount` 의 순서를 `CLEAR_LAMPS` 순으로 맞추면 막대가 램프 순으로 쌓입니다).

### 4.5 shadcn

새로 들어온 것: `sidebar`, `collapsible`(세 파일: `collapsible`, `collapsible-trigger`, `collapsible-content`), `toggle`, `toggle-group`, `checkbox`, `popover`, `progress`, `pagination`(한국어 레이블), `alert`, `textarea`, `scroll-container`, `scroll-indicator`. 원래 있던 것: `dialog`, `alert-dialog`, `dropdown-menu`, `select`, `tabs`, `table`, `tooltip`, `sheet`, `input`, `field`, `button`(터치 영역 확대), `badge`, `skeleton`, `empty`, `item`, `card`, `separator`, `switch`, `label`, `scroll-area`. 새 패키지는 들어오지 않았습니다(`radix-ui` 묶음 하나).

## 5. 공유 함수·상수·훅

| 이름 | 경로 | 하는 일 |
|---|---|---|
| `getDjRank(ex, maxEx)`, `getRateStep`, `getDjRankClassName` | `@shared/lib/dj-rank` | 27단계 DJ 랭크(`AAA` … `F`), 달성률 단계, 굵기 클래스 |
| `getMaxExScore(totalNotes)`, `formatChartTitle({title, sha256})`, `formatCount`, `formatHash`, `formatDateTime(ms)`, `formatPercent` | `@shared/lib/format` | 표시용. 제목이 비면 `제목 미등록 <sha 앞 8자…>`. 시각은 `Asia/Seoul` 고정 |
| `readParam`, `readNumberParam`, `readListParam`, `serializeSearchParams`, `toggleListValue` | `@shared/lib/search-params` | 쿼리 문자열 |
| `buildApiUrl(path, params?)`, `fetchEnvelope<T>`, `fetchEnvelopePage<T>`, `fetchRaw<T>`, `fetchNoContent`, `ApiError` | `@shared/lib/fetch` | 아래 참고 |
| `forwardedCookieInit()` | `@shared/lib/server-request-init` | 서버 프리페치에서 세션 쿠키 전달 |
| `getQueryClient()`, `createQueryClient()` | `@shared/utils/get-query-client` | staleTime 60초, 변경 실패는 토스트(`meta: { suppressErrorToast: true }` 로 끔) |
| `useIsMobile`, `useScrollThumb`, `useShellSidebarSlot`, `useUnsavedChangesWarning(isEnabled)` | `@shared/hooks/*` | `beforeunload` 경고는 폴더 편집의 미저장 상태에 씁니다 |
| `LOGIN_PATH`, `NEXT_PARAM`, `DEFAULT_NEXT_PATH`, `PROTECTED_PATH_PREFIXES`, `buildLoginHref`, `isSameOriginPath` | `@shared/constants/auth-route` | 로그인 경로 |
| `CLEAR_LAMPS`, `CLEAR_LAMP_LABEL`, `LAMP_BACKGROUND` | `@shared/constants/clear-lamp` | 램프 표 |
| `NAV_ITEMS`, `SITE_*`, `SITEMAP_STATIC_PATHS`, `ROBOTS_DISALLOW_PATHS`, `PLAYER_SORT_OPTIONS`, `DEFAULT_PLAYER_SORT` | `@shared/constants/*` | 메뉴, 사이트, 순위 정렬 |

`fetch.ts` 의 약속:

- `fetchEnvelope`: `/api/fe/*` 용. `{ success: true, data }` 를 풀어 `data` 만 돌려주고 `success: false` 는 `ApiError(code, message, status, details?)` 를 던집니다.
- `fetchEnvelopePage`: 같은데 `{ data, pagination }` 을 돌려줍니다(`pagination = { page, limit, total, totalPages }`).
- `fetchRaw`: 네이티브 IR 경로(봉투 없음). `fetchNoContent`: 204 를 돌려주는 DELETE.
- 클라이언트에서는 상대 경로(`/api/...`)로 가고 서버에서는 `NEXT_PUBLIC_APP_URL` → `VERCEL_URL` → `http://localhost:<PORT>` 의 자기 URL 로 갑니다(9절의 위험).
- 모든 요청에 `credentials: 'include'` 가 붙습니다. 세션 쿠키로 인증하는 FE 경로는 그대로 됩니다.

## 6. 서버 API

모든 FE 경로는 `{ success, data, pagination? }` 봉투와 snake_case 입니다. 에러는 `{ success: false, error: { code, message, details? } }` 이고 `details` 는 프로덕션에서 나가지 않습니다(그에 기대는 화면을 만들지 않습니다).

### 6.1 인증

| 메서드 경로 | 인증 | 설명 |
|---|---|---|
| `POST /api/fe/auth/login` | 없음 | 본문 `{ id, password }`. 200 + `Set-Cookie`(better-auth 세션) + `{ success: true, data: { user: { id, login_id, name, role } } }`. 틀린 자격은 401 `UNAUTHORIZED`(아이디가 없는 경우와 같은 응답). API 토큰을 만들지 않습니다 |
| `POST /api/fe/auth/register` | 없음 | 본문 `{ id, email, password, name? }`. 201 + 같은 꼴의 `user`. 아이디 형식 `^[a-zA-Z0-9_-]+$` 1~64자, `guest` 거부, 비밀번호 8자 이상(400 `VALIDATION_ERROR`), 중복 409 `IR_ACCOUNT_EXISTS` |
| `GET /api/auth/get-session` | 쿠키 | better-auth 클라이언트(`authClient.useSession`)가 부릅니다 |
| `POST /api/auth/sign-out` | 쿠키 | `authClient.signOut()` |
| `POST /api/auth/login`, `/api/auth/register` | 없음 | 게임용. 토큰을 발급합니다. 웹이 부르지 않습니다 |

- `/api/auth/[...all]` 은 `GET /get-session` 과 `POST /sign-out` 만 통과시키고 나머지(`/sign-in/email`, `/sign-up/email` 등)는 404 `NOT_FOUND` 입니다. 웹에서 `authClient.signIn` 을 부르면 안 됩니다.
- 로그인·가입의 IP 당 한도는 5/분이고 로그인 두 경로가 한 통을, 가입 두 경로가 한 통을 나눠 씁니다(429 `RATE_LIMITED`). 인스턴스 로컬 메모리라 서버리스에서는 인스턴스마다 따로입니다.
- 클라이언트 훅(`@entities/auth.query`): `useSessionLogin()`(변수 `{ id, password }`, 결과 `{ id, login_id, name, role }`, 401 은 "아이디 또는 비밀번호가 올바르지 않습니다."), `useRegister()`(변수 `{ id, password, email, name }`), `useSessionLogout()`, `useSessionUser()` → `{ user: SessionUser | null, isPending }`. 로그인·가입 성공 시 `authClient.$store.notify('$sessionSignal')` 로 계정 메뉴를 즉시 갱신하고 `removeUserScopedQueries(queryClient)` 로 사용자 묶음 캐시(7절)를 지웁니다. 로그아웃 성공도 같은 캐시를 지웁니다. 다른 경로로 세션을 바꾸는 코드를 쓰면 같은 신호를 보냅니다.
- `useSessionUser` 는 하이드레이션이 끝나기 전에는 항상 `{ user: null, isPending: true }` 를 돌려줍니다. better-auth 의 `useStore`(`node_modules/better-auth/dist/client/react/react-store.mjs`)가 `useSyncExternalStore(subscribe, get, get)` 으로 서버 스냅샷 자리에도 클라이언트의 현재 값을 넘기므로, 세션 조회가 하이드레이션보다 먼저 끝나면 서버가 그린 대기 상태와 다른 마크업이 그려져 React #418(하이드레이션 불일치)이 났습니다(17B-2 리뷰: 프로덕션 빌드의 `/tables/insane1`·차트 페이지에서 6/6, dev 서버에서는 재현되지 않음). 소비처마다 고치지 말고 이 훅을 씁니다. 회귀 테스트는 `tests/components/session-user.test.tsx` 입니다.
- 로그인 성공 뒤 이동은 `window.location.assign(next)` 의 전체 이동입니다. 비로그인 때 사이드바의 "내 폴더" 링크가 `/folders` 를 prefetch 하면 proxy 의 307 결과(로그인 화면)가 라우터 캐시에 남아, 전의 `router.replace('/folders')` 가 그 캐시를 써서 로그인 화면에 머물렀습니다. 세션이 바뀌면 모든 서버 렌더가 달라지므로 라우터 캐시를 믿지 않습니다. 보호 경로에서 로그아웃할 때도 같은 이유로 `window.location.assign('/')` 를 쓰고(전에는 `router.refresh()` 가 방금 나온 보호 경로를 다시 요청해 콘솔 404 를 남김), 그 밖의 경로에서는 `router.refresh()` 만 합니다. 가입은 `/settings` 로 `router.replace` + `refresh` 그대로입니다(`/settings` 는 비로그인 때 prefetch 되는 링크가 없고 리뷰에서 정상 확인).

### 6.2 난이도표(번들된 스냅샷)

표 id 는 `insane1` 하나이고 목록에서 읽습니다. `/api/fe/tables` 아래 세 읽기는 공개 캐시(`public, max-age=300, s-maxage=3600, stale-while-revalidate=86400`)이고 IP 120/분(`fe-read`) 한도를 받습니다. `records` 와 표준 형식 두 주소는 아래에 따로 적었습니다. 없는 id 와 모양이 틀린 id(`^[a-z0-9][a-z0-9-]{0,63}$` 아님)는 404 `IR_TABLE_NOT_FOUND` 입니다.

`GET /api/fe/tables` → 표 요약 배열(1,129B).

```json
{"success":true,"data":[{"id":"insane1","name":"発狂BMS難易度表","symbol":"★","entry_count":1035,"level_count":26,"last_update":"2017.02.05",
"original_url":"http://nekokan.dyndns.info/~lobsak/genocide/insane.html",
"source":{"page_url":"https://darksabun.club/table/archive/insane1/","fetched_at":"2026-10-14T02:41:43.771Z"},
"levels":[{"level":"1","count":53},{"level":"2","count":56}],
"standard_paths":{"page":"/tables/insane1","header":"/tables/insane1/header.json","data":"/tables/insane1/data.json"}}]}
```

`GET /api/fe/tables/insane1` → `{ table: <요약>, levels: [{ level: string, entries: Entry[] }] }`. `levels` 는 `level_order`("1" … "25", "???") 순서이고 380,844B 입니다. `Entry = { md5, level, title, artist, url, url_diff, name_diff, comment }`(전부 문자열, 없으면 빈 문자열). `sha256` 이 없고 곡 식별자는 `md5` 입니다. `url`·`url_diff` 는 외부 자료라 `http://` 또는 `https://` 로 시작할 때만 링크로 만들고 `target='_blank' rel='noopener noreferrer'` 를 답니다.

`GET /api/fe/tables/insane1/entries?q=&level=&page=&limit=` → `data: Entry[]` + `pagination`. `q` 는 제목·아티스트의 부분 일치(NFKC 소문자)와 md5 앞 6자 이상. `limit` 최대 100(기본 50).

`GET /api/fe/tables/insane1/records`(세션 필요, `Cache-Control: private, no-store`, 사용자당 30/분) → 내 기록. 기록이 있는 곡만 들어가고 없는 md5 는 키가 없습니다.

```json
{"success":true,"data":{"table_id":"insane1","records":{
  "0077fe17f69a9db8922c16ac00df960f":{"clear":"Hard","ex_score":1550,"minbp":12,"max_combo":500,"total_notes":1200,"played_at":1791949770000,"scratch_auto":false}}}}
```

- `clear` 는 `ClearLamp` 문자열입니다. 랭크는 `getDjRank(ex_score, total_notes * 2)` 로 계산합니다. 1,035곡 전부에 기록이 있어도 200KB 안쪽이고 한 번의 호출입니다.
- 세션이 없으면 401 이므로 비로그인에서는 부르지 않습니다(`useSessionUser().user` 가 `null` 이면 `enabled: false`).
- `scratch_auto`(웨이브 20): 내 기록은 일반 판 최고와 SCRATCH AUTO 판 최고 가운데 더 높은 것이고, `true` 면 SCRATCH AUTO 판입니다. 표의 카드와 곡 창은 그때 A-SCR 을 붙입니다.
- 내 기록은 `chart.md5` 로 맞춥니다(`chart_best` 기준). md5 하나에 대응하는 차트 행이 여럿(sha256 이 다름)이면 그중 한 건만 담기며 어느 것인지는 정해져 있지 않습니다.

표준 형식(봉투 없음, `Access-Control-Allow-Origin: *`, `Cache-Control: public, max-age=3600, s-maxage=3600, stale-while-revalidate=86400`):

- `GET /tables/insane1/header.json` → `{ name, symbol, data_url, level_order, last_update, original_url, mirror_of }`. `data_url` 은 요청의 origin 으로 만든 절대 주소(`https://bms.hyuns.uk/tables/insane1/data.json`)입니다.
- `GET /tables/insane1/data.json` → 원본 배열 그대로(399,616B, 1,035건, 곡마다 `lr2_bmsid` 포함). `ETag` 를 주고 `If-None-Match` 가 같으면 304.
- 이 두 주소는 `src/app/tables/[tableId]/{header,data}.json/route.ts` 가 처리합니다. 화면 페이지(`(app)/tables/[tableId]`)와 같은 `[tableId]` 이름을 써야 라우트 충돌이 나지 않습니다. 정적 `(app)/tables/insane1/page.tsx` 는 동적보다 우선합니다.

### 6.3 사용자 폴더

한도(서버 상수 `FOLDER_LIMIT`, `src/server/dto/folder.ts`): 폴더 50개, 폴더당 2,000곡, 사용자 전체 20,000곡, 한 번에 넣기 500, 이름 64자, 설명 255자, 곡 메모 100자, 가져오기 레벨 100개. 화면은 이 값을 `FOLDER_LIMIT`(`@entities/folder.type`, 서버 값과 같음을 테스트로 고정)로 두고 입력에서 막습니다. 서버가 다시 검사합니다.

편집은 **세션만** 받습니다(Bearer 는 401). 쓰기 한도 60/분, 가져오기 10/분, 읽기 120/분(사용자별). 응답 `Cache-Control: private, no-store`. 폴더 id 는 `^uf_[0-9a-f]{30}$` 이고 모양이 틀리면 400, 없거나 남의 폴더면 404 `IR_FOLDER_NOT_FOUND`(남의 폴더인지 알려 주지 않음).

| 메서드 경로 | 본문 → 응답 `data` |
|---|---|
| `GET /api/fe/me/folders` | `FolderSummary[]` (`position` 순) |
| `POST /api/fe/me/folders` | `{ name, description? }` → 201 `FolderSummary`. 같은 이름 409 `IR_FOLDER_NAME_TAKEN`, 50개 초과 409 `IR_FOLDER_LIMIT_EXCEEDED` |
| `PUT /api/fe/me/folders/order` | `{ folder_ids }` (지금 폴더와 같은 집합이어야 함, 아니면 400) |
| `GET /api/fe/me/folders/{id}` | `FolderDetail` (`items` 포함) |
| `PATCH /api/fe/me/folders/{id}` | `{ name?, description? }` → `FolderSummary` |
| `DELETE /api/fe/me/folders/{id}` | 204, 남은 폴더의 `position` 이 다시 매겨짐 |
| `POST /api/fe/me/folders/{id}/items` | `{ items: [{ md5, sha256?, title?, artist?, note? }] }`(1~500) → 변경 결과. 이미 든 곡은 `skipped` |
| `PUT /api/fe/me/folders/{id}/items` | **명시 저장용 통째 바꾸기** `{ items: [...순서대로], base_updated_at? }` → 변경 결과. `base_updated_at` 이 서버의 `updated_at` 과 다르면 409 `IR_FOLDER_CONFLICT`(쓰지 않음) |
| `POST /api/fe/me/folders/{id}/items/remove` | `{ md5s }` → 변경 결과 |
| `PUT /api/fe/me/folders/{id}/items/order` | `{ md5s }` (같은 집합이어야 함) → 변경 결과 |
| `PATCH /api/fe/me/folders/{id}/items/{md5}` | `{ note }` → 그 곡 항목 |
| `POST /api/fe/me/folders/{id}/import` | `{ table_id, levels? }` → 변경 결과. 서버가 번들 표에서 곡을 읽어 넣음. 없는 표 404 `IR_TABLE_NOT_FOUND` |
| `GET /api/fe/me/folders/containing/{md5}` | 그 곡이 든 내 폴더 id 배열 |

```json
FolderSummary = { "id":"uf_623ec6e8ddc745d58514a68517281c","name":"연습 12단","description":"12단 연습용","is_public":false,"position":0,"item_count":3,"updated_at":1791949779614 }
FolderDetail  = FolderSummary + { "items":[{ "md5":"0077…","sha256":"…03e9","title":"Born [29Another]","artist":"Ninja Action Team / 29","note":"" }] }
변경 결과     = { "added":3,"removed":0,"skipped":0,"item_count":3,"updated_at":1791949779614 }
409 충돌      = { "success":false,"error":{ "code":"IR_FOLDER_CONFLICT","message":"다른 곳에서 먼저 고친 폴더입니다. 다시 불러와 주세요.","details":{ "server_updated_at":1791949780686 } } }
```

- `updated_at` 은 Unix ms 입니다. 편집 화면은 폴더를 불러올 때의 `updated_at` 을 `base_updated_at` 으로 들고 있다가 저장(`PUT .../items`)에 실어 보냅니다. 409 면 저장하지 말고 다시 불러오라고 안내합니다(`details` 는 프로덕션에 안 나가므로 값에 기대지 않습니다).
- 곡의 `title`·`artist`·`sha256` 은 서버가 비어 있으면 차트 행 → 번들 표 → 요청 값 순서로 채웁니다. 서버가 아는 곡이 아니면 `sha256` 이 빈 문자열일 수 있습니다. 요청의 `title`·`artist` 는 제어 문자를 지우고 200자(코드 포인트)로 잘라 저장합니다.
- `md5` 는 소문자 32자 16진수여야 합니다(대문자는 서버가 낮춥니다).
- 곡 검색은 두 길입니다. 서버가 아는 차트는 `GET /api/fe/charts/search?q=`(제목이 비어 있는 행은 번들 표 제목으로 채워져 옴, `ChartMeta.md5` 가 빈 결과는 담지 말 것). 번들 표 곡은 `GET /api/fe/tables/{id}/entries?q=`.
- 클라이언트(R-BMS)가 읽는 통로(봉투 없음, Bearer 또는 세션, 본인만, `no-store`): `GET /api/players/{playerId}/folders` → `{ folders: FolderSummary[] }`, `GET /api/players/{playerId}/folders/{folderId}` → `FolderDetail`. 다른 `playerId` 는 403 `FORBIDDEN`, 인증 없음 401. `GET /api/health` 의 `capabilities.folders` 가 `true` 입니다. 웹 화면이 이 통로를 부를 일은 없습니다.
- 폴더 응답의 최악 크기는 약 3.9MB(2,000곡, 4바이트 문자 꽉 참)로 Vercel 본문 한도 4.5MB 에 여유가 작습니다. 화면은 곡 제목·아티스트의 입력 길이를 200자로 막고 한 폴더를 크게 만들지 않게 안내합니다.

### 6.4 에러 코드

| 코드 | 상태 | 뜻 |
|---|---|---|
| `IR_FOLDER_NOT_FOUND` | 404 | 폴더 없음 또는 남의 폴더 |
| `IR_FOLDER_NAME_TAKEN` | 409 | 같은 이름의 폴더가 이미 있음 |
| `IR_FOLDER_LIMIT_EXCEEDED` | 409 | 폴더·곡 개수 한도 초과(`details` 는 비프로덕션만) |
| `IR_FOLDER_CONFLICT` | 409 | `base_updated_at` 불일치 |
| `IR_TABLE_NOT_FOUND` | 404 | 번들 표 id 없음 |
| `IR_ACCOUNT_EXISTS` | 409 | 가입 아이디 중복 |
| `UNAUTHORIZED` | 401 | 세션 없음 또는 로그인 실패 |
| `FORBIDDEN` | 403 | 다른 플레이어의 자원 |
| `VALIDATION_ERROR` | 400 | 본문·쿼리 검증 실패 |
| `RATE_LIMITED` | 429 | 한도 초과 |

세 파일(`error-code.ts`, `error-message.ts`, `error.ts`)에 같은 코드가 있고, 메시지는 한국어입니다. 화면은 `ApiError.code` 로 갈라 사용자 문구를 고르고, 모르는 코드는 `ApiError.message` 를 보여 줍니다. 변경 훅의 전역 토스트는 `onError` 가 따로 없으면 `message` 를 띄웁니다.

### 6.5 이 웨이브에서 바뀐 기존 API

- `GET /api/fe/leaderboards/players?sort=cleared|played|plays`(기본 `cleared`; 옛 `rank_points` 는 `cleared`, `total_plays` 는 `plays` 로 받음). 항목은 `PlayerProfile` + `cleared_charts` + `played_charts`. `rank_points` 는 항상 0 이므로 화면에서 쓰지 않습니다. 클리어는 램프 EASY 이상입니다. 화면(`player-leaderboard-panel.tsx`)은 이미 이 값으로 바꿨습니다.
- 차트 순위(`/api/fe/charts/{hash}/leaderboard` 의 `ranking[]`, `/api/charts/{hash}/ranking`)의 `extra.replay_id`(문자열, 리플레이가 있는 점수만). 차트 화면의 "리플레이" 열이 `/replays/{id}` 로 잇습니다.
- 차트 제목 보강: 제출이 만든 제목 없는 차트 행은 읽을 때 번들 표의 제목·아티스트로 채워집니다(`GET /api/charts/{hash}`, FE 리더보드, 검색, 활동 피드). 점수 제출 본문의 `chart` 에 `title`, `artist`, `level` 을 선택으로 실을 수 있고 제목 없는 행만 채워집니다.
- `POST /api/charts` 는 빈 값만 채우는 방식(fill-only)이고 `POST /api/courses/meta` 는 관리자 세션 전용입니다.

### 6.6 웨이브 20: SCRATCH AUTO 순위

사용자 결정(2026-10-14): SCRATCH AUTO 판도 순위에 올리되 순위 목록에서는 기본으로 보이지 않고 토글을 켜면 보입니다. 넓힌 판정 판은 그대로 unranked 입니다.

- 쿼리 `include_scratch_auto=true|false`(기본 `false`): `/api/fe/charts/{hash}/leaderboard`, `/api/charts/{hash}/ranking`·`/best`, `/api/courses/{hash}/ranking`·`/best`, `/api/fe/leaderboards/players`. 차트·코스 목록은 기본이 일반 판 최고, 켜면 플레이어마다 일반 판 최고와 SCRATCH AUTO 판 최고 가운데 더 높은 것입니다. 리더보드의 클리어·플레이 곡·제출 수는 기본이 일반 판만이고, 켜면 SCRATCH AUTO 판까지 셉니다.
- 목록 행 `ScoreRecord.extra.scratch_auto: true` 가 SCRATCH AUTO 판입니다(없으면 일반 판). 활동 행(`/api/fe/players/{id}/recent`)은 `scratch_auto: boolean`, 통계(`/api/fe/players/{id}/stats`)는 `scratch_auto_charts`(더 높은 쪽이 SCRATCH AUTO 판인 곡 수)가 더해졌습니다. 홈의 최근 활동은 SCRATCH AUTO 판을 뺍니다.
- 화면: 차트 순위(`chart-leaderboard-panel.tsx`, 순위 머리), 코스 순위(`course-detail-panel.tsx`, 순위 카드의 action), 리더보드(`player-leaderboard-panel.tsx`, 정렬 옆)에 `ScratchAutoToggle`(`@features/filter/scratch-auto-toggle`, shadcn `Switch`)이 있고, 상태는 주소의 `include_scratch_auto=true` 로 남습니다(`router.push`, 뒤로 가기로 돌아감). 서버 `*-content.tsx` 가 `readBooleanParam` 으로 읽어 같은 값으로 미리 채웁니다. SCRATCH AUTO 행에는 `ScratchAutoBadge`(`@features/score/scratch-auto-badge`, 무채색 outline "A-SCR")가 붙고 뜻은 각 목록의 각주(`SCRATCH_AUTO_LIST_NOTE`)로 알립니다. 문구 상수는 `@shared/constants/scratch-auto`.

## 7. 쿼리 키 규칙

`QUERY_KEY`(`@shared/constants/query-key`)는 도메인 → 동작 → 파라미터 계층의 배열입니다. 파라미터 없는 키는 정적 배열, 있는 키는 함수입니다. 기존 `TABLE`(`['table', ...]`)은 DB 의 IR 표(`/api/tables`)용이라 섞지 않습니다. 아래가 17B-2 에서 실제로 들어간 키입니다.

| 키 | 값 | 훅·옵션 |
|---|---|---|
| `CHART.RANKING(hash, params)` | `['chart', 'ranking', hash, { include_scratch_auto }]` | `chartLeaderboardQueryOptions(hash, params)`, `useChartLeaderboard(hash, params)`. 웨이브 20 에서 `params` 가 더해졌습니다 |
| `COURSE.RANKING(courseHash, params)` | `['course', 'ranking', courseHash, { limit, include_scratch_auto }]` | `courseRankingQueryOptions`, `useCourseRanking` |
| `LEADERBOARD.PLAYERS(params)` | `['leaderboard', 'players', { sort, page, limit, include_scratch_auto }]` | `playerLeaderboardQueryOptions`, `usePlayerLeaderboard` |
| `TABLE_SNAPSHOT.RECORDS_ALL` | `['table-snapshot', 'records']` | 사용자 묶음 정리용 접두 |
| `TABLE_SNAPSHOT.RECORDS(tableId, userId)` | `['table-snapshot', 'records', tableId, userId]` | `tableSnapshotRecordsQueryOptions(tableId, userId)`, `useGetTableSnapshotRecords(tableId, userId \| null)`(null 이면 꺼짐). `queryFn` 은 봉투를 벗긴 `records` 맵(md5 → 기록) |
| `FOLDER.ALL` | `['folder']` | 곡이 들어가고 빠지는 변경의 무효화 |
| `FOLDER.LIST` | `['folder', 'list']` | `folderListQueryOptions`, `useGetFolderList(enabled)` |
| `FOLDER.DETAIL(id)` | `['folder', 'detail', id]` | `folderDetailQueryOptions(id)`, `useGetFolderDetail` |
| `FOLDER.CONTAINING_ALL` | `['folder', 'containing']` | 폴더 삭제 때 무효화 |
| `FOLDER.CONTAINING(md5)` | `['folder', 'containing', md5]` | `folderContainingQueryOptions`, `useGetFolderContaining(md5, enabled)` |

- 17B-1 의 제안과 다른 점: `RECORDS` 에 `userId` 를 넣어 계정이 바뀌어도 남의 기록이 한순간도 보이지 않게 했습니다. 표 목록·상세·곡 색인은 서버 컴포넌트가 8절의 함수로 직접 읽어 props 로 내리므로 `TABLE_SNAPSHOT.ALL`·`LIST`·`DETAIL`·`ENTRIES` 키와 쿼리는 두지 않았습니다(리뷰 뒤 쓰는 곳이 없던 `ALL`·`LIST` 를 지움). 클라이언트에서 표를 다시 받는 화면이 생기면 그때 더합니다.
- 위치는 `entities/table-snapshot.{type,query}.ts`, `entities/folder.{type,query}.ts` 입니다. 조회는 `queryOptions` 팩토리 + `useQuery`, 변경은 `useMutation` 이고 `onSuccess` 에서 무효화합니다. 폴더 변경 훅: `useCreateFolder`, `useUpdateFolder`, `useDeleteFolder`(LIST·CONTAINING_ALL 무효화, DETAIL 제거), `useReorderFolders`, `useReplaceFolderItems`, `useAddFolderItems`, `useRemoveFolderItems`(FOLDER.ALL 무효화). 대부분 `meta: { suppressErrorToast: true }` 이고 사용자 문구는 `toFolderErrorMessage`(`@entities/folder.type`)로 고릅니다. 폴더 편집 화면은 캐시를 직접 고치지 않고(`setQueryData` 금지) 로컬 초안(`widgets/folder/folder-draft.ts`)을 두었다가 명시 저장 뒤 무효화합니다.
- 사용자 묶음 캐시: `removeUserScopedQueries(queryClient)`(`@entities/auth.query`)가 `AUTH.ALL`, `FOLDER.ALL`, `TABLE_SNAPSHOT.RECORDS_ALL`, `TOKEN.ALL`, `SETTING.ALL` 을 지웁니다. 로그인·가입·로그아웃 성공에서 부릅니다. 공개 자료(예 `STATS.SUMMARY`)는 남깁니다(`tests/components/folder-contract.test.ts`).
- 서버 미리 채우기: 발광 표는 `widgets/table/table-snapshot-session.tsx` 가 `connection()` 뒤 `readSessionUser` 로 사용자를 읽고, 로그인이면 `compose().tableSnapshotService.getRecords` 로 `RECORDS(tableId, userId)` 를 채워 `HydrationBoundary` 로 내립니다. 첫 HTML 부터 램프가 보입니다. 폴더 목록·편집도 `compose().folderService` 를 직접 불러 같은 키를 채웁니다.

## 8. 서버에서 표 자료를 읽는 함수

표 화면은 정적 자료이므로 자기 URL 로 HTTP fetch 하지 않고 서버 컴포넌트에서 직접 읽는 것이 안전합니다. 아래 함수는 DB 를 건드리지 않아 환경 변수 없이 빌드(정적 프리렌더)에서도 쓸 수 있습니다.

| 함수 | 파일 | 돌려주는 것 |
|---|---|---|
| `getSnapshotTables()` / `getSnapshotTable(id)` | `@server/service/domain/table-snapshot/table-snapshot.data` | 검증된 `SnapshotTable`(`header`, `entries`, `source`) 또는 `null` |
| `summariseTable(table)`, `groupTableByLevel(table)` | `@server/service/domain/table-snapshot/table-snapshot.service` | `/api/fe/tables`, `/api/fe/tables/{id}` 의 `data` 와 같은 값 |
| `searchTableEntries(table, query)`, `searchMd5sInTables(tables, q)` | 같은 파일 | 곡 검색 |
| `findSnapshotTextByMd5(md5)` | `table-snapshot.data` | `{ title, artist }` 또는 `null` |
| `compose().tableSnapshotService.{list,getDetail,searchEntries,getRecords}` | `@server/compose` | 같은 일을 하는 서비스. `getRecords` 는 DB 를 읽으므로 `compose()` 는 요청 시점(`connection()` 뒤)에만 부릅니다 |

- 한 가지 길로 통일하려면 화면이 서버에서 `groupTableByLevel(getSnapshotTable('insane1'))` 를 부르고 결과를 `HydrationBoundary` 로 내리거나 props 로 넘깁니다. 클라이언트에서 다시 받아야 하는 것(내 기록)만 `/api/fe/tables/{id}/records` 를 부릅니다.
- 위젯이 `@server/*` 를 import 하는 것은 FSD 의 예외입니다(서버 전용 파일, `import 'server-only'` 로 클라이언트 번들 진입을 막음). `SessionGate` 가 같은 방식으로 `@server/lib/auth` 를 동적으로 불러 씁니다.

## 9. 빌드·런타임 주의

- **요청 접근 순서**: 요청 시점 라우트나 서버 컴포넌트에서 `compose()` 보다 먼저 `connection()` 또는 `headers()` 를 읽어야 합니다. 안 그러면 환경 변수 없는 빌드의 정적 수집이 DB 설정 오류 로그를 냅니다(빌드 자체는 성공하지만 로그가 더러워집니다). 새 `app/api` 라우트는 `withErrorHandling` 으로 감싸고 본문에서 요청을 읽는 형태를 따릅니다.
- **자기 URL fetch**: 서버 컴포넌트의 프리페치(`fetchEnvelope` 등)는 `NEXT_PUBLIC_APP_URL` → `VERCEL_URL` 의 자기 공개 URL 로 HTTP 호출합니다. Preview 배포가 운영 자료를 읽을 수 있고 서버 렌더가 `/api/fe` 의 IP 한도(120/분)를 같이 쓸 수 있다는 위험이 `docs/web/architecture.md` 6-1 에 적혀 있습니다. 이번 웨이브의 새 화면(표·폴더)은 8절의 직접 호출을 씁니다. 기존 화면의 전환은 남아 있습니다.
- **`NEXT_PUBLIC_APP_URL`**: 빌드 시점에 서버·클라이언트 번들에 박힙니다. 값이 바뀌면 다시 빌드합니다. 로컬 개발에서는 필요하지 않습니다(클라이언트는 상대 경로).
- **CSP**: 프로덕션에서만 붙습니다(`next.config.ts`). `script-src`·`style-src` 에 `'unsafe-inline'` 이 있고 `connect-src 'self'`, `img-src 'self' data: blob:` 입니다. 외부 도메인의 이미지·글꼴·스크립트·API 호출을 새 화면에 넣으면 CSP 를 함께 고쳐야 합니다. 표의 `url`·`url_diff` 는 링크(`a`)로만 쓰고 `img`·`iframe` 으로 불러오지 않습니다.
- **라우트 prefetch 와 세션**: 비로그인 때 메뉴의 보호 경로 링크(`/folders`)를 prefetch 하면 proxy 의 307 결과가 라우터 캐시에 남습니다. 세션이 바뀌는 동작(로그인, 보호 경로에서 로그아웃) 뒤에는 클라이언트 이동 대신 전체 이동을 씁니다(6.1절).
- **dev 와 프로덕션의 차이**: 하이드레이션 불일치(#418)는 dev 서버에서 재현되지 않고 프로덕션 빌드(`next start`)에서만 났습니다. 세션·하이드레이션에 닿는 변경은 프로덕션 빌드로 확인합니다.
- **rate limit 의 IP**: `getClientIp`(`src/server/lib/with-rate-limit.ts`)는 `X-Forwarded-For` 의 첫 값을 씁니다. Vercel 은 이 헤더를 덮어쓰고 바깥 IP 를 넘기지 않으므로(Vercel 문서 "Request headers" 의 `x-forwarded-for` 절: "we currently overwrite the X-Forwarded-For header and do not forward external IPs. This restriction is in place to prevent IP spoofing.") 운영에서는 우회되지 않습니다. 다른 호스팅이나 Vercel 앞에 다른 프록시를 두면 헤더를 바꿔 한도를 피할 수 있습니다(로컬 재현: 틀린 비밀번호 5회 뒤 헤더 값을 바꾸면 429 대신 401 이 계속 남). 지속형 한도로 바꿀 때 신뢰하는 헤더를 하나로 고정합니다.
- **이전 경로의 잔상**: 통합 확인에서 클라이언트 이동 직후 이전 페이지의 `h1`(예: 로그인 → 설정 뒤의 `로그인`)이 DOM 에 함께 남아 있는 것을 보았습니다(전체 새로고침 뒤에는 한 개). Cache Components 가 이전 경로를 보존하는 동작으로 보이며 화면에는 나타나지 않았습니다. 화면 테스트에서 `h1` 개수를 세지 않습니다.

## 10. 아직 없는 것

| 항목 | 상태 | 맡을 곳 |
|---|---|---|
| 홈의 로그인 패널(내 정보·램프 현황·내 폴더)과 난이도표 패널 | 없음 | 2차 |
| 발광 표: 한 곡 한 행 보기, 카드 높이 선택, 접힘 상태 기억, 모바일 레벨 칩 선택기 | 없음(모바일 시트의 레벨 선택은 26개 한 목록) | 2차 |
| 발광 표: 늦게 그린 묶음의 브라우저 찾기(Ctrl+F) | 화면에 오기 전 묶음은 DOM 에 없어 찾지 못함. 페이지의 검색 상자로는 찾음 | 2차 |
| 발광 표 RSC 자료 줄이기 | 항목의 `level`(묶음에 이미 있음)과 `url_diff == url` 중복이 약 458KB(gzip 114KB) 안에 있음 | 2차(서버 출력 정리) |
| 키보드: 카드 목록으로 건너뛰기 | 진행 패널의 레벨 링크 26개는 없앴고(레벨 칩이 이동 담당), "본문으로 건너뛰기"는 본문 맨 위로 감 | 2차 |
| 폴더: 끌어 옮기기, 항목별 내 램프, 복제, 사용자 전체 20,000곡 한도의 사전 안내 | 없음(한도는 서버 오류 문구로만 알림) | 2차 |
| 폴더 편집의 큰 목록 | 2,000곡에서 이동·빼기 75ms, CPU 4배 감속 236~344ms. 행 가상화 없음 | 필요해지면 2차 |
| 폴더 저장의 원자성 | 곡(PUT items)과 이름·설명(PATCH)이 두 요청. PATCH 실패 시 화면이 알림 | 2차(서버 한 요청으로) |
| 브라우저 뒤로·앞으로의 미저장 경고 | 앱 안 링크와 탭 닫기만 막음 | 2차 |
| 설정 탭의 주소 반영(`?tab=`) | 없음 | 2차 |
| 사이드바 보조 링크(버전, 저장소) | 저장소 공개 여부 미확인으로 보류 | 사용자 확인 후 |
| OG 이미지, JSON-LD | 없음(텍스트 OG 만) | 2차 |
| 기존 서버 컴포넌트의 자기 URL fetch 를 서비스 직접 호출로 | 새 화면(표·폴더)은 직접 호출. 기존 화면은 위험만 문서화(9절) | 별도 작업 |
| 지속형 rate limit | 인스턴스 로컬(결정 M10). IP 헤더 전제는 9절 | 후속 |
| 없는 상세 주소의 HTTP 상태 | 차트·플레이어·코스·리플레이의 없는 주소가 404 화면이지만 HTTP 200 (Suspense 안에서 `notFound()` 를 부르는 구조, 운영 확인 2026-10-14) | [2026-10-14-audit.md](./2026-10-14-audit.md) KNOWN ISSUE |

## 11. 통합에서 바뀐 것(두 단위의 보고와 달랐던 곳)

- 웹 로그인이 막혀 있었습니다. U1 의 `LoginForm` 이 `authClient.signIn.email`(`/api/auth/sign-in/email`)을 불렀고 U2 가 그 경로를 404 로 막았습니다. 아이디 기반 세션 전용 경로 `POST /api/fe/auth/login` 을 만들고 폼을 `아이디` 로 되돌렸습니다. 가입도 같은 방식(`POST /api/fe/auth/register`)으로 바꿔 웹이 API 토큰을 몰래 만들던 문제(점검 8번)를 로그인·가입 모두에서 없앴습니다. 게임용 두 경로는 그대로입니다.
- 가입 직후 계정 메뉴가 갱신되지 않던 문제를 고쳤습니다(가입이 better-auth 클라이언트를 거치지 않아 세션 신호가 없었음). 로그인·가입이 성공하면 `$sessionSignal` 을 보냅니다.
- 순위 화면이 U2 가 바꾼 정렬값을 몰랐습니다. 정렬 선택과 열을 `cleared`/`played`/`plays` 와 `cleared_charts`/`played_charts` 로 맞추고, 항상 0 인 랭크 포인트 열을 순위와 라이벌 목록에서 뺐습니다.
- 차트 순위에 "리플레이" 열을 더해 `extra.replay_id` 를 `/replays/{id}` 로 이었습니다(고아였던 리플레이 페이지에 들어오는 길이 생김).
- `ci.yml` 의 두 트리거에 `paths-ignore: ["web/**"]` 를 더했습니다(`dev` 브랜치 보호 규칙이 없음을 GitHub API 로 읽어 확인).
- `package.json` 에 `db:status`(`bun scripts/db-migration-status.ts`)를 더했습니다.
- `authClient` 에서 막힌 경로를 부르게 되는 `signIn`·`signUp` 내보내기를 지웠습니다.
- 두 단위의 계약 중 틀린 것은 없었고 위 항목만 어긋나 있었습니다.

## 12. 검증 기록(17B-1)

`web/` 에서(환경 변수는 명령줄로만 줌, `.env` 는 읽지 않음):

- `bun install --frozen-lockfile`: 변경 없음.
- `bun run verify`: 타입 생성(`next typegen`) + `tsc` 오류 0, `eslint` 출력 없음, `bun test` 통과(통합 중 로그인·가입·순위 변경분 포함).
- `bun run format:check`: 통과.
- 환경 변수 없는 `bun run build`: 성공, 오류 로그 0줄. `bunx drizzle-kit generate`: "No schema changes".
- 임시 MySQL(Docker `mysql:8`, 포트 33103, 끝난 뒤 삭제)에 `bun run db:migrate` 후 `bun run db:status` 가 `up to date`.
- 포트 3103 개발 서버와 프로덕션 서버(`next start`, CSP 포함)에서 실제 흐름: 보호 경로 307 → 로그인 폼(틀린 비밀번호의 오류 문구, 맞는 비밀번호) → `next` 로 복귀 → 계정 메뉴 이름 → 홈 → 설정 → 순위 → 차트 → 로그아웃 → 다시 보호 경로 307. 가입 흐름과 계정 메뉴. 1440x900 과 390x844, 가로 넘침 0. 프로덕션 CSP 위반 0건.
- 같은 서버에서 표 목록·상세·검색·내 기록, 표준 `header.json`/`data.json`, 폴더의 만들기·곡 넣기·표에서 가져오기·통째 바꾸기·충돌 409·순서·메모·포함 폴더·클라이언트 읽기(Bearer, 타인 403)·인증 오류를 확인했습니다.

## 13. 웨이브 17B-2 의 화면과 컴포넌트

### 13.1 화면

| 경로 | 페이지 파일 | 내용 |
|---|---|---|
| `/tables` | `app/(app)/tables/page.tsx` → `widgets/table/table-catalog-content.tsx` | 번들 표 카드(`TableSnapshotCard`: 진행 막대, 표 주소 복사)를 위에, DB 의 IR 표(`TableListPanel`)를 아래에 |
| `/tables/insane1` | `app/(app)/tables/insane1/page.tsx` → `widgets/table/table-snapshot-content.tsx` | 정적 metadata(`bmstable`, noindex). 머리(표 이름, 범위, 곡 수, 모바일만 표 주소 복사), 출처 요약 줄, 로그인하면 "내 진행"(기본 접힘: 요약 + 램프 막대, "레벨별 보기"로 분포·범례와 레벨 26줄), 비로그인 안내 줄, 필터 막대(검색 `/` 단축키, 레벨 범위, 램프, 정렬. 모바일은 아래 시트), 레벨 칩 한 줄, 접히는 레벨 묶음(붙는 머리, 램프 막대, 플레이 수), 카드 격자(2~6열), 상세 대화상자, 바닥 출처 문안. 사이드바 슬롯: 출처와 표 주소 복사 |
| `/folders` | `app/(app)/folders/page.tsx` → `widgets/folder/folder-list-content.tsx` | 폴더 행(행 전체가 링크, 위·아래, 메뉴), 새 폴더 대화상자, 빈 상태. 사이드바 슬롯: "R-BMS 에서 보기" |
| `/folders/[folderId]` | `app/(app)/folders/[folderId]/page.tsx` → `widgets/folder/folder-editor-content.tsx` | 명시 저장 편집: 곡 목록(순번, ★레벨, 위·아래, 메뉴, 빼기), xl 이상 오른쪽 검색 패널(아래는 시트), 표에서 가져오기, 409 충돌 안내, 미저장 경고(앱 안 링크·탭 닫기), Ctrl/Cmd+S |
| `/charts/[hash]` | (기존) | 표에 있는 곡이면 "★16 発狂BMS難易度表" 배지(`/tables/insane1#lv-16`), 머리의 "내 폴더에 넣기" |

- 보호: `/folders` 아래는 `folders/layout.tsx` 의 `SessionGate`(3.4절). 편집 화면은 세션이 없으면 `buildLoginHref('/folders/<id>')` 로 보내 깊은 경로를 지킵니다.
- 표 필터 주소: `q`, `from`, `to`(레벨 값), `lamp`(`all|noplay|failed|unclear|easy|clear|hard|exhard|fullcombo`), `sort`(`table|title|lamp|rate`). 기본값은 주소에 쓰지 않고, `window.history.pushState/replaceState` 로 바꿔 서버 왕복이 없습니다. 레벨 앵커는 `levelAnchorId(level, index)`: `lv-12`, 영숫자가 아닌 레벨은 `lv-other-<순서>`(`???` → `lv-other-25`). 표는 마운트할 때 해시 위치로 스크롤합니다.
- 렌더 비용: 화면에서 먼 레벨 묶음은 `TableDeferredMount`(IntersectionObserver)로 늦게 그리고 그동안 정확한 높이의 자리만 둡니다. 비로그인 HTML 668,814B(gzip 135,873B).

### 13.2 컴포넌트

| 이름 | 경로 | 계약 |
|---|---|---|
| `AddToFolderMenu` | `widgets/folder/add-to-folder-menu.tsx` | `{ md5; sha256?; title?; artist?; level? }`. md5 가 32자 16진수가 아니면 그리지 않음. `level` 은 받은 문자열 그대로 메뉴 머리에 보이므로 `★16` 처럼 기호를 붙여 넘김. 비로그인이면 로그인 링크, 로그인이면 체크 메뉴(즉시 반영), 새 폴더를 만들어 넣기, 내 폴더 관리. `sm` 미만은 아이콘만 + `aria-label` |
| `TableProgressPanel` | `features/table/table-progress-panel.tsx` | `{ lamps; played; total; levels: { key; label; lamps; played; total }[] }`. 기본 접힘, 머리 배지에 "played / total · %", "레벨별 보기" 버튼이 `aria-expanded`·`aria-controls`. 레벨 줄은 링크가 아님(이동은 `TableLevelJump`) |
| `TableLevelJump` | `features/table/table-level-jump.tsx` | `{ items: { anchorId; label; isAvailable }[]; onJump; className? }`. 조건에 맞는 곡이 없는 레벨은 점선 `aria-disabled` |
| `TableLevelSection` | `features/table/table-level-section.tsx` | Collapsible 레벨 묶음. 머리는 `h2.rbms-level-header` 안의 트리거(Enter·Space 로 접기) |
| `TableChartGrid` / `TableChartCard` | `features/table/table-chart-{grid,card}.tsx` | 1px 틈 격자와 빈칸 채움. 카드는 5px 램프 띠, 제목 2줄, 아티스트·EX·랭크 한 줄(레벨은 묶음 머리가 보이므로 카드에 쓰지 않고 `aria-label` 에만 넣음) |
| `TableDeferredMount`, `TableLevelSkeleton` | `features/table/*` | 늦게 그리기, Suspense 대기 화면 |
| `TableFilterFields` | `features/table/table-filter-fields.tsx` | `layout: 'inline' \| 'stacked'` 의 레벨·램프·정렬 필드 |
| `TableSourceNotice` | `features/table/table-source-notice.tsx` | `variant: 'summary' \| 'full'` 의 출처 문안(보관본·원본·삭제 요청, 연락처 `https://github.com/B-HS/R-BMS/issues`) |
| `TableExternalLink`, `formatUrlHost` | `features/table/*` | http(s) 만 링크, 새 탭, `noopener noreferrer` |
| `TableSnapshotCard` | `features/table/table-snapshot-card.tsx` | `/tables` 의 번들 표 카드 |
| `TableChartDialog` / `TableChartDetails` | `widgets/table/*` | 상세 대화상자(외부 링크, 코멘트, 내 기록, MD5 복사, `AddToFolderMenu`, 서버에 차트가 있을 때만 차트 순위 링크). 닫으면 포커스가 카드로 |
| `TableSnapshotWorkspace` / `-Sidebar` / `-Session` / `-ListItem` | `widgets/table/*` | 표 화면의 조립, 사이드바 슬롯, 서버의 내 기록 미리 채우기, `/tables` 의 번들 표 행 |
| `FolderListItem`, `FolderChartRow`, `ChartSearchResultRow`, `FolderMetaDialog`, `FolderNoteDialog`, `FolderTableImportDialog`, `FolderConflictNotice`, `FolderGuidePanel` | `features/folder/*` | 폴더 화면의 순수 조각 |
| `FolderListContent` / `-Panel`, `FolderEditorContent` / `FolderEditor` / `-Workspace`, `FolderChartSearchPanel` | `widgets/folder/*` | 폴더 화면의 조립. 초안 계산은 `folder-draft.ts`, 이탈 막기는 `use-folder-leave-guard.ts`, 편집 화면용 표 색인은 `read-folder-table-index.ts`(서버, 약 110KB) |
| `useIsHydrated` | `shared/hooks/use-is-hydrated.ts` | 하이드레이션 중 false, 뒤에 true |
| `useDialogFocusReturn` | `features/dialog/use-dialog-focus-return.ts` | 4.3절 |

### 13.3 entities

- `@entities/table-snapshot.type`: 표 요약·상세·항목·기록 타입(`@server/dto/table-snapshot` 의 출력에서 유도).
- `@entities/table-snapshot.query`: 7절.
- `@entities/folder.type`: `FOLDER_LIMIT`(서버 값과 같음을 테스트로 고정), `isFolderId`, `isFolderMd5`, `toNewFolderItemInput`, `folderMetaFormSchema`, `toFolderErrorMessage`, 폴더 타입.
- `@entities/folder.query`: 7절.

## 14. 17B-2 검증 기록

`web/` 에서, 환경 변수는 명령줄로만 줌(`.env` 는 읽지 않음).

- 리뷰 발견의 재현과 수정 확인(프로덕션 빌드 `next start`, 임시 MySQL 8): `/guide` 를 연 뒤 이동하는 방식 6회씩, 하이드레이션 오류가 로그인 `/tables/insane1` 0/6, 차트 0/6, `/players/alice` 0/6, `/folders` 0/6, 비로그인 표·차트 0/6(리뷰 때 6/6). `next=/folders` 로그인이 501ms 에 `/folders` 로 이동(리뷰 때 12초 동안 머묾). `/settings` 에서 로그아웃 → `/`, 콘솔 오류·4xx 0. 1440x900 `/players/alice` 의 패널 높이 = 내용 높이(램프 분포 192/192, 최근 기록 979/979), 본문 스크롤 852/1405 에서 휠 뒤 scrollTop 553.
- 발광 표(1440, 로그인): 첫 카드 y 541 → 289, 보이는 `h2` 의 "내 진행" 1개, 보이는 "표 주소 복사" 1개, 레벨 칩 1줄, 펼친 레벨 줄 26개에 링크 0개. 모바일 390 첫 카드 y 330. 라이트 대비 5.13:1(사이드바 라벨)·6.04:1(레벨 머리 숫자)·6.58:1(카드 아티스트).
- 게이트: `bun install --frozen-lockfile` 변경 없음, `bun run verify` 395 pass / 2 skip / 0 fail, `bun run format:check` 통과, 환경 변수 없는 `bun run build` 오류·경고 0줄, `bunx drizzle-kit generate` "No schema changes", `package.json`·`bun.lock` 변경 없음.
- 시험 데이터 정리 SQL(`web/scripts/cleanup-test-accounts.{count,delete}.sql`)을 임시 DB 에 같은 아이디 다섯 개(제출 10, 토큰 5, 세션 6, 폴더 1·곡 53)를 만들어 실행: 지운 뒤 모든 대상 0, 다른 계정 3개와 제출 460건·폴더 3개는 그대로.
