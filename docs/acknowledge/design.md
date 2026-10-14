# R-BMS 웹 디자인 결정

> 이 문서는 현재 `web/` 구현의 디자인 정본입니다. 외부 프로젝트의 디자인 명세를 복구하지 않으며, 토큰과 화면 규칙은 `web/src/app/globals.css`, 레이아웃과 provider는 `web/src/app/{layout,providers}.tsx`, 조합 규칙은 `docs/web/components.md`를 우선 근거로 합니다.

## 1. 범위와 Surface

### 1-1. 정본과 범위

- 대상은 R-BMS IR 웹의 `(app)`과 `(public)` 라우트 그룹입니다.
- 데이터 요청, 인증 계약, 서버 DB는 이 문서의 범위가 아닙니다.
- 색상·간격·반응형·접근성의 구현 값은 CSS 변수와 Tailwind 유틸리티로만 유지합니다.

### 1-2. Surface 하나 (2026-10-14 변경, design-direction T1)

모든 화면(로그인·가입·가이드 포함)이 같은 셸(2열: 사이드바 + 본문)과 같은 토큰을 씁니다. 공개 Surface 델타(`[data-surface='public']`: 반경 6px·그림자·배경 한 단)는 지웠습니다. 근거는 `docs/web/2026-10-14-design-direction.md` 3.1.1 과 결정 `docs/acknowledge/2026-10-14-web-decisions.md` M7 입니다.

| 항목 | 값 |
|---|---|
| 라우트 | `(app)`, `(public)` 둘 다 루트 `AppShell` 안 |
| radius | `0rem` |
| 깊이 | sidebar / background / card 3단 |
| 그림자 | overlay(대화상자·팝오버·드롭다운)만 |

## 2. 시각 원칙

- 모든 화면은 정사각·평면·고밀도입니다. 블록은 1px gap과 배경 명도로 분리합니다. 인증 카드도 반경·그림자 없이 1px 테두리만 씁니다.
- 의미 없는 데이터는 muted 색을 사용하고, destructive·warning·chart 토큰은 상태를 나타낼 때만 사용합니다.
- 이모지 대신 lucide 아이콘과 텍스트 레이블을 사용합니다.

## 3. 색상과 타이포그래피

- `globals.css`의 `--palette-*`는 OKLCH 원색, `--background`·`--card`·`--foreground` 등은 의미 토큰입니다. 컴포넌트는 의미 토큰만 참조합니다.
- Surface A light는 sidebar `0.915`, background `0.955`, card `1`; dark는 앱 무채색 단계(`crates/rbms-render/src/theme.rs`, 스킨 `theme.mono`)에 맞춰 `0`(GROUND), `0.162`, `0.200`(SURFACE) 순입니다. dark 의 `--foreground` 는 `0.961`(INK), `--primary` 는 `1`(LIT, 흰 강조)입니다. `--sidebar-primary`·`--sidebar-ring` 은 뜻 없는 파랑 대신 `var(--primary)`·`var(--ring)` 입니다.
- 본문·숫자는 `--font-sans`, 해시·토큰·코드·식별자만 `--font-mono`를 사용합니다. 시각은 사람이 읽는 값이라 본문 글꼴 + `tabular-nums` 입니다(T8). 굵기는 400·500·600 셋뿐이고 패널·구역 제목은 600 입니다(T7). 숫자 열은 `tabular-nums`와 우측 정렬을 기본으로 합니다.

## 4. 테마와 모션

- `ThemeProvider`는 `attribute={['class', 'data-theme']}`, `defaultTheme='dark'`, `enableSystem={false}`, `disableTransitionOnChange`, `storageKey='rbms-theme'`로 루트 provider에 항상 마운트합니다.
- `.dark`와 `[data-theme='dark']`는 같은 dark token을 선택합니다. next-themes가 pre-paint 처리를 맡으므로 별도 수기 스크립트를 추가하지 않습니다.
- 기본 easing은 `cubic-bezier(0.4, 0, 0.2, 1)`, fade는 `0.18s`, bar 전이는 `0.24s`입니다. `prefers-reduced-motion`에서는 전이와 애니메이션을 사실상 제거합니다.

### 4-4. chart 색과 도메인 색

`--chart-1`부터 `--chart-5`는 light/dark에서 각각 팔레트를 다시 바인딩하는 시리즈 색이며 램프에는 쓰지 않습니다.

램프 11종·판정 5색과 FAST/SLOW·난이도 6색은 기본 스킨 `assets/skins/rbms-default/lua/theme.lua` 의 sRGB 값을 그대로 `globals.css` 의 `:root` 에 `--rbms-lamp-*`, `--rbms-judge-*`, `--rbms-timing-*`, `--rbms-difficulty-*` 로 둡니다(T3, T11). 테마와 무관한 값이며, 띠·견본·막대의 배경으로만 칠하고 글자는 항상 `text-foreground` 에 이름을 함께 적습니다. 흰 계열 램프가 라이트에서 사라지지 않도록 견본은 `border-foreground/40`, 띠는 `.rbms-lamp` 의 오른쪽 구분선을 씁니다. `Normal` 의 표시 이름은 앱과 같은 CLEAR 입니다. 랭크(AAA … F)는 색 없이 굵기와 명도로만 구분합니다(`shared/lib/dj-rank.ts`).

### 4-7. provider 일관성

Toaster와 모든 theme-aware 컴포넌트는 같은 `ThemeProvider` 하위에 둡니다. provider 없이 OS 테마를 따르는 개별 오버레이를 만들지 않습니다.

## 5. 토큰 사용

- 색: `bg-background`, `text-foreground`, `bg-card`, `border-border`, `text-muted-foreground` 등 CSS variable 기반 유틸리티만 사용합니다.
- 간격: 기본 spacing은 `0.25rem`, 앱 패널 inset은 `12px`, 패널 사이 gap은 `1px`입니다.
- z-index는 rail `10`, raised `20`, overlay `50`을 기준으로 합니다.

## 6. 표면 규칙

### 6-3. radius 안전성

모든 화면은 `--radius: 0rem`입니다. 따라서 `--radius-step-sm`부터 `--radius-step-xl`은 모두 `var(--radius)`로 고정합니다. `calc(var(--radius) - Npx)`는 음수 radius를 만들 수 있어 앱 Surface에서 쓰지 않습니다.

### 6-4. 경계와 그림자

앱의 panel·격자는 border·shadow보다 background 위 `1px` gap으로 분리합니다. 목록 행과 페이지 머리는 `border-b` 로 나눕니다(T6). navigation rail의 불필요한 우측 border를 추가하지 않으며, dialog·popover 같은 overlay에서만 shadow 토큰을 사용합니다.

## 7. 접근성

### 7-4. 기본 보장과 알려진 경계

- panel 제목은 `h2`, 주 콘텐츠는 `main`, 탐색은 `nav`, 필터는 `aside`로 표시합니다.
- 램프, ranked, 경고는 색과 텍스트를 함께 표시합니다.
- Tooltip은 native `title` 대신 shadcn Tooltip을 사용합니다.
- 리플레이 워터폴은 `role='img'`와 요약 `aria-label`을 제공하지만, 세부 이벤트 키보드 탐색은 아직 지원하지 않습니다.

## 8. 레이아웃

### 8-1. 앱 2열 (T4, T9)

- 모든 화면은 shadcn `Sidebar`(`collapsible='icon'`, 폭 `16rem`, 접으면 `3rem`, 모바일 시트 `18rem`) + 본문의 2열입니다. 우측 context panel 은 폴더 편집 화면에서만 씁니다. 사이드바 머리·바닥과 페이지 머리는 `3rem`, 메뉴 항목은 `2.25rem` 입니다.
- 페이지마다 48px 머리(`ShellPageHeader`: `h1` + 설명 + 동작)가 하나 있습니다.
- `md` 이상은 화면 고정에 본문만 내부 스크롤(3px 스크롤 표시 막대), `md` 미만은 문서 스크롤이고 머리가 위에 붙습니다.
- 인증 카드는 셸 본문 가운데 `max-w-sm`, 문서(가이드)는 `max-w-3xl` 왼쪽 붙임입니다.

## 9. 상태 표현

- 데이터 로딩은 skeleton을 우선하며, 버튼은 진행 중 disabled 상태를 사용합니다.
- 빈 결과, 전송 실패, 조회 실패는 서로 다른 메시지와 재시도 동작을 갖습니다.
- destructive action은 confirm dialog를 거칩니다.

## 10. 컴포넌트 카탈로그

`docs/web/components.md`의 매핑을 구현 계약으로 사용합니다.

| 항목 | 역할 |
|---|---|
| 10-1 Panel Card | 앱 Surface의 기본 정보 블록 |
| 10-2 Stat Tile | 작은 지표 요약 |
| 10-3 Data Table | 랭킹·검색·기록의 숫자 표 |
| 10-4 Series Chart | 시간·추이 시각화 |
| 10-5 Status Badge | 램프·ranked·flag 상태 |
| 10-6 Bar-Count List | 램프·분포 막대 |
| 10-7 Trace Waterfall | µs 리플레이 시간축 |
| 10-8 Attribute Tree | options·extra 검사 |
| 10-9 Form Dialog | 토큰·빌드·라이벌 입력 |
| 10-10 State Triad | pending·empty·error |
| 10-11 Navigation Rail | 앱 탐색 |
| 10-12 Context Filter Panel | URL 기반 필터 |
| 10-13 보조 제어 | pager, confirm, column toggle, footnote |
| 10-14 SVG 규칙 | 리플레이 등 SVG는 `var(--color-*)`를 직접 참조 |

## 11. 페이지 아키타입

### 11-A. Signal List

리더보드·표 목록처럼 필터와 표가 중심인 화면입니다.

### 11-B. Stacked Detail

차트·플레이어·코스처럼 identity, 지표, 상세 목록이 순서대로 쌓이는 화면입니다.

### 11-C. Metric Overview

홈의 서버 통계와 최근 활동입니다.

### 11-D. Tabbed Console

settings의 탭 기반 관리 화면입니다.

### 11-E. Admin CRUD

builds처럼 목록, 작성 동작, 관리 액션을 결합한 화면입니다.

### 11-F. Query Console

`/search`의 URL 직렬화 필터 화면입니다.

### 11-G. Full-bleed Visualization

`/replays/[replayId]`의 자체 overflow 워터폴입니다.

### 11-H. Public

로그인·가입·가이드도 셸 안의 화면입니다. 메뉴가 유지됩니다.

## 12. 반응형

- 사이드바는 `md` 미만에서 sheet 로 이동하고, 페이지 머리의 메뉴 버튼으로 엽니다.
- table은 정보 우선순위에 따라 열을 줄이고, 리플레이 워터폴은 자체 스크롤을 유지합니다.

## 13. 폼과 피드백

- 폼은 label, 오류 메시지, disabled submit을 갖습니다.
- 성공·실패 mutation은 sonner toast를 사용하되, 서버 오류 원문·시크릿을 표시하지 않습니다. 토스트는 오른쪽 아래, 반경 0·그림자 없음·종류별 왼쪽 4px 선입니다(T10, `shared/providers/app-toaster.tsx`). 폼 안에 인라인 오류가 있으면 토스트를 띄우지 않습니다.

## 14. shadcn 경계

- `components.json`은 `radix-nova`, neutral base color, CSS variables, lucide icon library를 사용합니다.
- shadcn primitive는 `@shared/ui`에 두고, 도메인 조합은 features/widgets에 둡니다.

## 15. 검증 범위

- 정적 검증은 `bun run typecheck`, `bun run lint`, `bun run test`, `bun run build`입니다.
- 시각 회귀와 light/dark 실렌더는 운영 환경을 포함한 수동 QA 범위이며, 실제 배포 확인은 Phase R에서 수행합니다.

## 16. Tailwind v4 전달

### 16-1. 앱 기본 토큰

`globals.css`는 `:root`와 `.dark, [data-theme='dark']`에 palette·semantic·spacing·radius·shadow·motion 변수를 정의하고, `@theme inline`으로 `--color-*`, `--font-*`, `--radius-*`, `--shadow-*` utility를 연결합니다.

### 16-2. 공개 Surface 델타 (삭제)

2026-10-14 에 지웠습니다(1-2 절). 도메인 색은 `:root` 의 `--rbms-*` 입니다(4-4 절).

### 16-3. 테마 적용

루트 `Providers`의 next-themes가 hydration 이전 테마 적용을 담당합니다. 수기 pre-paint 스크립트는 추가하지 않습니다.

## 17. 구현 제약

### 17-1. 마스터 제약

1. 앱 Surface의 radius·panel gap·semantic token 규칙을 우회하지 않습니다.
2. 하드코드 hex/rgb/hsl과 임의 Tailwind 팔레트 색은 컴포넌트에 쓰지 않습니다. 스킨에서 옮긴 도메인 색만 `globals.css` 의 `:root` 에 16진수로 두고, 컴포넌트는 `var(--rbms-*)`(또는 `LAMP_BACKGROUND`)로만 참조합니다(T11).
3. 색상만으로 상태를 전달하지 않습니다.
4. 로딩에는 skeleton, 아이콘에는 lucide, 도움말에는 Tooltip을 사용합니다.
5. `useCallback`·`useMemo`로 스타일 상태를 최적화하지 않습니다.
