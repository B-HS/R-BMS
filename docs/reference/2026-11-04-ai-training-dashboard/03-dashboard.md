# 03. 로그인 사용자 "/" 대시보드 조사 (iidx-rank 대시보드 분해와 R-BMS 웹 대응)

> 작성 2026-11-04 · 조사 시점 R-BMS `dev` `4bac699`(작업 트리 깨끗), iidx-rank `2cd9795`(읽기 전용, `.env*`·`.vercel`·`node_modules`·`.next` 는 읽지 않았다)
> 목적: 로그인한 사용자가 `/` 에서 대시보드를 보게 한다. 사용자 요청 "iidx-rank 처럼 dashboard 에서 보일 수 있는 내용을 최대한 보여봐", 결정 "기존 홈 내용은 유지하고 로그인 사용자에게만 `/` 를 대시보드로 교체".
> 이 문서는 코드를 바꾸지 않은 조사 보고서다. 같은 폴더의 01·02 문서(다른 조사)는 읽지 않았다.

표기: **[확인]** 이 세션에서 파일을 직접 읽어 확인한 사실, **[추정]** 코드에서 추론했지만 실행으로 확인하지 않은 것, **[제안]** 조사자의 설계안(결정 아님), **[미확인]** 확인하지 못한 것. 경로는 `web/src` 를 `W:`, iidx-rank 의 `src` 를 `I:` 로 줄여 쓴다.

---

## 0. 먼저 읽을 것

1. **[확인]** iidx-rank 의 홈 대시보드는 `I:app/[locale]/(shell)/page.tsx` 하나가 서버에서 데이터를 프리페치하고, 클라이언트 위젯 `HomeDashboard` 가 로그인 여부로 회원 6패널 / 손님 6패널을 가른다. 회원 6패널은 내 정보(+e-amusement 가져오기 요약), 노트레이더, 램프 현황(+DJ 랭크 분포), 랭크별 램프, 최근 플레이곡 20곡, 게시판 활동이다. 모든 집계는 "카탈로그 전체 + 내 기록 전체" 두 덩어리를 받아 **클라이언트의 순수 함수**(`I:entities/dashboard/dashboard-summary.ts`)로 계산한다. 서버 집계 쿼리는 없다.
2. **[확인]** 차트 라이브러리는 iidx-rank 도 R-BMS 도 쓰지 않는다. 막대는 CSS(`flex-grow`·`width %`), 레이더는 인라인 SVG `polygon` 이다. R-BMS 도 `PlayDayChart` 가 CSS 막대다. 문서 규칙: 차트는 애니메이션하지 않는다(`docs/web/components.md` 6절 6번).
3. **[확인]** R-BMS 웹에는 이미 iidx-rank 의 `DashboardPanel`·`DashboardPanelEmpty/Error/Loading`·`DASHBOARD_GRID_CLASS_NAME` 이식본이 있고(`W:features/dashboard/*`, `W:shared/constants/dashboard.ts`), 현재 `/` 는 손님용 3패널(소개, 서버 현황, 최근 활동)이다. 로그인 패널은 `docs/web/2026-10-14-design-direction.md:433` 와 `docs/web/tasks.md:378` 에 "2차"로 미뤄 둔 채 만들어지지 않았다.
4. **[확인]** 바로 재사용 가능한 R-BMS 데이터: 플레이어 통계(램프 분포·총 제출·클리어 수·SCRATCH AUTO 곡 수), 일별 플레이 시간 30일, 최근 판 목록, 내 단위(A-단위 포함), 내 폴더 목록, 라이벌 목록, 발광 표 3종의 내 기록(`/records`), AI 결정론 digest(주간 요약·추천·목표·판정 습관·리포트), AI 사용량. 모두 기존 훅이 있다.
5. **[확인]** 새 서버 집계가 필요한 것: 표 **레벨별** 램프 분포(지금은 표 상세 약 307KB + 기록 약 150KB 를 클라이언트가 합친다. 크기는 `docs/web/architecture.md:165` 기록이고 이 세션에서 재지 않았다), DJ 랭크 분포(`chart_best` 에 노트 수가 없어 `score` 조인 필요), 활동 달력(30일을 넘는 기간), 라이벌 최근 활동·비교, 내 코스 진행, 내 리플레이, 종합 순위에서 내 위치.
6. **[확인]** `/` 는 `proxy.ts` 의 matcher 밖이라 공개이고(`W:proxy.ts:37-48`), 이미 요청마다 렌더된다(`W:app/layout.tsx:22` `instant = false`, `W:widgets/home/home-content.tsx:13` `await connection()`). 서버에서 세션을 읽는 선례는 `readSessionUser()`(`W:widgets/auth/session-gate.tsx:9-22`)와 이를 try/catch 로 감싸 `initialUserId` 를 클라이언트에 넘기는 `TableSnapshotSession`(`W:widgets/table/table-snapshot-session.tsx:12-39`)이다. 같은 패턴을 `/` 에 쓰면 된다.
7. **[확인]** 비용: 회원 `/` 에 패널을 늘릴수록 요청마다 DB 질의가 는다. `'use cache'` 로 감싼 읽기(통계·최근·일별)는 캐시되지만, 세션 읽기·표 기록·폴더·단위·AI digest 는 `private, no-store` 라 매번 DB 를 친다. AI digest GET 은 사용자당 분당 30회 제한이다(`W:server/lib/with-rate-limit.ts:12`).
8. **[제안]** 1차는 새 쿼리 없이 기존 훅만으로 8패널, 2차에 새 집계 3~4개(표 진행도, DJ 랭크, 활동 달력, 라이벌 활동)를 더하는 두 단계를 권한다(8절).

---

## 1. 조사 방법과 한계

- 먼저 읽은 기존 저장 조사: `docs/reference/ai-features/README.md`(색인), `docs/HANDOFF.md` 5·7절, `docs/web/architecture.md`(1-1, 1-3, 2, 5, 16절), `docs/web/components.md`(6, 8, 9, 11절), `docs/web/2026-10-14-design-direction.md`(1.5, 3.3.1, 3.3.6), `docs/reference/ai-features/server.md` 일부(4.3, 5.6, 5.7, 부록 A). 이 문서들이 기술한 사실 중 아래는 원본 코드로 다시 확인했다: 홈 구성, 프록시 matcher, 세션 읽기, 통계·일별·최근 API, 캐시 태그, 표 기록 API, AI digest 라우트, 차트 라이브러리 유무.
- Next.js 판은 16.3.4 이고 `web/AGENTS.md:5` 가 "이 Next 는 알던 것과 다르니 `node_modules/next/dist/docs/` 를 읽으라"고 한다. 읽은 문서: `01-app/01-getting-started/08-caching.md`, `01-app/02-guides/authentication-with-cache-components.md`, `01-app/03-api-reference/03-file-conventions/02-route-segment-config/instant.md`, `01-app/03-api-reference/04-functions/cacheLife.md`.
- **[미확인]** 운영 사이트(bms.hyuns.uk)에서 실제 응답 시간·DB 질의 수·캐시 적중은 측정하지 않았다. 아래 "비용"은 코드를 읽고 센 값이다.
- **[미확인]** iidx-rank 화면의 실제 렌더 모습(스크린샷)은 보지 않았다. 레이아웃은 클래스 문자열에서 읽은 것이다.
- **[미확인]** `'use cache'` 의 저장소가 Vercel 서버리스에서 어디에 놓이는지(인스턴스 메모리인지 공유인지)는 확인하지 않았다. `next.config.ts:25-30` 의 설정 객체에 `cacheHandlers` 는 없다.

---

## 2. iidx-rank 의 대시보드

### 2.1 라우트와 파일 전수

`I:app` 의 페이지는 `[locale]/(shell)/{page(홈), board, board/new, board/[postId], board/[postId]/edit, import, privacy, settings, table, u/[handle], users}` 와 `error`·`not-found` 뿐이다(`find src/app` 결과, **[확인]**). "대시보드"에 해당하는 화면은 **홈 `/` 하나**이고 사용자 페이지 `u/[handle]` 의 개요 탭이 보조 요약이다. 별도의 `/dashboard` 라우트는 없다.

| 파일 | 줄 | 역할 |
|---|---|---|
| `I:app/[locale]/(shell)/page.tsx` | 22-34, 36-57, 59-68, 70-81 | 서버: 프리페치와 `HydrationBoundary`, 페이지 틀, 메타데이터 |
| `I:widgets/home-dashboard/home-dashboard.tsx` | 12-19 | 클라이언트: 손님/회원 분기 |
| `I:widgets/home-dashboard/home-dashboard-frame.tsx` | 6-14 | 머리 + `@container/dashboard` 스크롤 영역 |
| `I:widgets/home-dashboard/home-member-dashboard.tsx` | 14-23 | 회원 6패널 배치 |
| `I:widgets/home-dashboard/home-{profile,radar,lamp,rank,recent,board}-panel.tsx` | 각 파일 | 회원 패널 |
| `I:widgets/home-dashboard/home-records-panel.tsx` | 18-31 | 카탈로그+기록 로딩/오류/빈 상태 래퍼 |
| `I:widgets/home-dashboard/home-catalog-panel.tsx` | 15-34 | 카탈로그만 필요한 패널 래퍼(손님) |
| `I:widgets/home-dashboard/home-guest-*.tsx` | 각 파일 | 손님 6패널 |
| `I:widgets/home-dashboard/home-dashboard-skeleton.tsx` | 7-14, 16-35 | 6패널 스켈레톤 |
| `I:entities/dashboard/{dashboard-summary.ts, dashboard.query.ts, comments-seen.client.ts, comments-seen.dto.ts}` | 위 | 집계 순수 함수, 조회 훅, "새 댓글" 로컬 저장 |
| `I:features/dashboard-{panel,lamp,activity,profile}/*` | 위 | 패널 틀, 램프/등급 막대, 행, 정보 표 |
| `I:features/{notes-radar-chart,notes-radar-list,bar-count-list,fitted-row-list,lamp-swatch}/*` | 위 | SVG 레이더, 값 목록, 막대 목록, 높이에 맞춰 자르는 목록, 램프 견본 |
| `I:shared/constants/dashboard.ts` | 1-37 | 상수와 그리드 클래스 |

### 2.2 서버 프리페치와 손님/회원 분기

- 서버 `HomeDashboardDataBoundary`(`I:app/[locale]/(shell)/page.tsx:36-57`): `getBoardViewer()` 로 viewer 를 읽고(37), 새 `QueryClient`(staleTime `DEFAULT_QUERY_STALE_TIME_MS`=60,000, `I:shared/constants/cache.ts:1`)에 병렬 프리페치한다(40-50). 항상: 카탈로그. viewer 가 있으면: 내 기록(`checkerQueryOptions`), 가져오기 상태(`importStatusQueryOptions`), 게시판 활동(`myBoardActivityQueryOptions`), 내 프로필·프로필 통계(`prefetchOwnProfile`, 22-34, 실패는 삼킴). 없으면: 게시판 1쪽. `Suspense fallback={<HomeDashboardSkeleton />}`(63).
- 클라이언트 `HomeDashboard`(`home-dashboard.tsx:12-19`): `useViewerIdentity(initialUserId)`(`I:entities/auth/use-viewer-identity.ts:5-14`)가 서버가 본 viewer 와 클라이언트 세션을 맞춘다. `isAligned` 가 거짓이면 스켈레톤, `viewerId === null` 이면 손님, 아니면 회원. 서버와 클라이언트 세션이 어긋나는 순간(로그인·로그아웃 직후)의 깜빡임을 막는 장치다.
- 캐시 시간: 카탈로그 5분(`cache.ts:2`), 사용자 데이터 1분(`cache.ts:3`), gc 5분(`cache.ts:4`). 내 기록 쿼리는 응답의 `userId` 가 요청과 같은지 검사한다(`I:entities/checker/checker.query-options.ts:9-14`).

### 2.3 레이아웃

- 본문 틀(`home-dashboard-frame.tsx:6-14`): `ShellPageHeader`(제목 "홈") + `ScrollContainer className='@container/dashboard flex flex-col'`. 컨테이너 쿼리 기준이 뷰포트가 아니라 본문 폭이다.
- 격자(`I:shared/constants/dashboard.ts:25-26`, 패널 사이는 `gap-px bg-border` 1px 선): 기본 1열. 본문 폭 45rem 이상: 2열, 행 `15rem 15rem 1fr`(최소 높이 44rem). 62rem 이상: 3열, 행 `minmax(15rem,1fr) minmax(0,1.6fr)`(최소 높이 31rem). 96rem 이상: 4열. 넓은 패널(`DASHBOARD_WIDE_PANEL_CLASS_NAME`, 27행)은 96rem 이상에서만 2칸을 차지한다.
- 회원 패널 순서(`home-member-dashboard.tsx:16-21`): 프로필, 레이더, 램프, 랭크, 최근(넓음), 게시판(넓음). 4열일 때 첫 줄 네 패널 + 둘째 줄 넓은 둘, 3열일 때 3+3, 2열일 때 2+2+2.
- 패널 틀 `DashboardPanel`(`I:features/dashboard-panel/dashboard-panel.tsx:12-28`): `section[aria-labelledby]` + 36px 머리(`h2` 제목, `badge`, `actions`) + 본문 `px-3 pb-3`. 머리 동작은 `DashboardPanelLink`(`dashboard-panel-link.tsx:8-12`, 기본 `size='xs' variant='ghost'`).
- 패널별 상태: 오류(문구+재시도), 로딩(스켈레톤, `role='status' aria-busy`), 빈(문구+버튼). 쿼리마다 따로 분기한다(예: `home-profile-panel.tsx:53-97`).

### 2.4 회원 대시보드 6패널의 데이터·계산·시각화

#### 패널 1. 내 정보(`home-profile-panel.tsx:25-99`)

- 데이터: `useMyProfile`(내 프로필: name, handle, avatarUrl, isPublic), `useProfile(handle)`(followerCount, followingCount, playedCount), `useImportStatus`(`/api/import/status` 의 `latest`). `latest` 필드는 `player{djName, iidxId, danRank, djPoint, playCountSp, playCountDp}`, `importedAt`, `channel('extension'|'file')`, `receivedCount`, `matchedCount`, `changedCount`, `notesRadar`(`I:entities/eamusement/eamusement.dto.ts:31-48`, `dashboard-import-summary.tsx:6-8`).
- 표시: 아바타·이름·공개/비공개 배지·`@handle · 팔로워 · 팔로잉`(`dashboard-identity.tsx:12-40`), `dl` 2열 정보 표(DJ 이름, ID, 단위, DJ POINT 소수 2자리, 플레이 수 SP/DP 한 줄, `dashboard-player-info.tsx:20-46`), 가져오기 요약 4칸(경로, 받은 수, 맞춘 수, 바뀐 수)과 마지막 동기화 시각(`dashboard-import-summary.tsx:19-42`). 가져온 적이 없으면 빈 상태 + 확장 프로그램 설치 버튼 + 설정 링크(`home-profile-panel.tsx:78-89`). 머리 동작: 내 페이지, 프로필 설정.
- 시각화: 없음(텍스트 표).

#### 패널 2. 노트레이더(`home-radar-panel.tsx:14-38`)

- 데이터: `importStatus.latest.notesRadar` 6축 `NOTES, CHORD, PEAK, CHARGE, SCRATCH, SOF-LAN`(각 `number|null`, `eamusement.dto.ts:20,41-48`). 이 값은 e-amusement 에서 가져온 값이고 계산하지 않는다.
- 계산식(`I:features/notes-radar-chart/notes-radar-chart.tsx`): 축 비율 `min((값 ?? 0) / NOTES_RADAR_CHART_MAX_VALUE(200), 1)`(55행). 점 좌표 `각도 = 2π·i/6 − π/2`(위쪽에서 시계 방향), `x = 50 + 34·비율·cos`, `y = 50 + 34·비율·sin`(31-35행). viewBox 100, 반지름 34, 점 반지름 1.4, 안내 링 4개(`dashboard.ts:9-12`).
- 시각화: 인라인 SVG. 링 4개 `polygon`(마지막 링만 `fill-muted/50`), 축선 6개, 값 `polygon`(`fill-primary/20 stroke-primary`), 꼭짓점 `circle`. 축 이름은 SVG 밖의 절대 위치 `span`(22-29행). 오른쪽에 6값 목록(`notes-radar-list.tsx:11-29`, 소수 2자리, `null` 은 `-`). 접근성: `role='img'` + `aria-label` 에 값 요약(66행).
- 레이아웃: `[container-type:size]`, 차트 `size-[min(100cqh,56cqw)]`(`home-radar-panel.tsx:32-34`).

#### 패널 3. 램프 현황 + DJ 랭크 분포(`home-lamp-panel.tsx:15-31`)

- 데이터: 카탈로그의 모든 차트 + 내 기록. 둘을 `useDashboardRecords`(`I:entities/dashboard/dashboard.query.ts:8-24`)가 합친다.
- 램프 요약 `getLampSummary`(`dashboard-summary.ts:30-39`): 차트마다 기록의 램프, 없으면 `NO_PLAY`. `total = 차트 수`, `played = NO_PLAY 가 아닌 차트 수`, 램프별 개수. 카탈로그 밖 기록은 무시한다(JSDoc, 25-29행).
- 표시(`lamp-distribution.tsx:12-60`): "플레이한 보면 {played} / {total}", 플레이 비율 `round(played/total·100)%`(16행), 누적 막대(높이 12px, 램프별 `flex-grow = count`, 개수 0 은 제외, 최소 폭 2px, 1px 틈, 위쪽 램프가 왼쪽: `lamps.toReversed()`), 2열 범례(견본+이름+개수). 램프 색 `LAMP_STATIC_BACKGROUNDS`(`dashboard.ts:14-23`): EX_HARD 는 위 50% ex-hard + 아래 50% clear 그라데이션, FULL_COMBO 는 hard/easy 반반.
- DJ 랭크 분포 `getScoreGradeDistribution`(`dashboard-summary.ts:94-99`): 카탈로그에 있는 차트의 기록만, `scoreGrade`(F..AAA 8단계, `checker.dto.ts:6`)별 개수, AAA→F 순서. 시각화(`score-grade-chart.tsx:12-34`): 세로 막대 8개, 높이 `count/max·100%`, 아래에 개수와 등급 글자. 막대 색은 `bg-muted-foreground/70` 한 가지.

#### 패널 4. 랭크별 램프(`home-rank-panel.tsx:17-26`)

- 상태: `ToggleGroup`(노멀/하드) `useState('hard')`(18행). 머리 동작에 `CheckerModeToggle`.
- 계산 `getRankLampDistribution`(`dashboard-summary.ts:13-23, 47-54`): 각 차트에는 노멀 랭크·하드 랭크(`F,E,D,C,B,B+,A,A+,S,S+` 또는 `null`, `catalog.dto.ts:3,12-13`)가 있고, 랭크마다 그 랭크 차트의 램프 개수와 `total`, `played` 를 센다. 순서는 `S+` → `F` → `null`(`RANK_ORDER`, 9행). 화면은 `null`(무랭크) 줄을 뺀다(`rank-lamp-list.tsx:17`).
- 시각화(`rank-lamp-list.tsx:14-70`): 랭크마다 한 줄 = 랭크 이름(w-7) + 램프 누적 막대(위 패널과 같은 방식) + `played/total`. 머리 오른쪽에 "플레이 {합계 played}/{합계 total}". 줄 높이는 `grid-template-rows: repeat(n, minmax(1rem, 1fr))`, 최대 높이 `n·2.5rem`(`BAR_COUNT_ROW_MAX_HEIGHT_REM`). 아래 범례. 막대에 `role='img'` 와 `aria-label`(램프 이름+개수).

#### 패널 5. 최근 플레이곡 20곡(넓음, `home-recent-panel.tsx:22-62`)

- 계산 `getRecentRecords(charts, records, 20)`(`dashboard-summary.ts:63-87`): 기록 중 NO_PLAY 아닌 것을 카탈로그 차트와 조인(차트 없으면 제외) → 정렬 `updatedAt 내림차순` → 같으면 `LAMPS.indexOf` 큰 쪽 → 같으면 `chartId` 사전순 → 앞에서 20개(`DASHBOARD_RECENT_RECORDS_LIMIT`, `dashboard.ts:1`). **곡(차트)당 한 줄**이고 "그 차트의 현재 기록이 갱신된 시각" 기준이다.
- 행(`recent-record-row.tsx:13-55`, 높이 40px): 왼쪽 5px 램프 띠(`CHART_LAMP_WIDTH_PX`=5), 곡명(+난이도 표기, `formatChartTitle`), "난이도 · 버전", 오른쪽에 램프 이름 + DJ 랭크, EX 점수 + 압축 시각. 머리 동작 "전체 보기" → 내 프로필 기록 탭 또는 `/table`.
- 목록 틀 `FittedRowList`(`fitted-row-list.tsx:14-43`): `ResizeObserver` 로 첫 행 높이 대비 목록 높이를 재서 들어가는 행 수만 보이고 나머지는 `inert`. 그래서 20곡을 다 렌더하지만 보이는 건 패널 높이만큼이다.

#### 패널 6. 게시판 활동(넓음, `home-board-panel.tsx:29-125`)

- 데이터: `useMyBoardActivity` → `{ posts: PostSummary[], comments: MyPostComment[] }`(`GET /api/board/mine`). 두 구역: 내가 쓴 글(행 28px)과 내 글에 달린 댓글(행 40px, 비율 2:3).
- "새 댓글": 브라우저 `localStorage` 키 `iidx:dashboard-comments-seen:v1:{userId}`, 최대 64개 id(`dashboard.ts:3-5`, `comments-seen.client.ts:8-52`). 저장 실패 시 세션 메모리로 대체. 머리에 새 댓글 개수 배지, "모두 확인" 버튼, 글쓰기·목록 아이콘 링크.

### 2.5 손님 대시보드 6패널(`home-guest-dashboard.tsx:9-17`)

소개(로그인 버튼·기능 3줄·난이도표 링크), 난이도표 출처 요약(갱신일·차트 수·수록 버전 수), 보면 구성(난이도 H/A/L 별 개수 막대, 노멀/하드 개인 차트 수 막대), 랭크별 보면 수(노멀/하드 각 `S+..F` 개수 막대, 최댓값 대비 비율), 공지, 최근 글. 카탈로그만 필요하다. `BarCountList`(`I:features/bar-count-list/bar-count-list.tsx:13-36`): 줄마다 라벨 + 막대(`width = clamp(ratio,0,1)·100%`) + 값.

### 2.6 대시보드 밖의 요약(사용자 페이지 `u/[handle]` 개요 탭)

`I:widgets/user-page/user-page-overview-tab.tsx:11-39`: e-amusement 요약(`profile-eamusement-summary.tsx:12-41`: DJ 이름, 단위, 동기화 시각, 레이더 값 목록)과 램프별 개수 `Progress` 막대(`profile-lamp-summary.tsx:13-36`, `round(count/total·100)`; `total` 은 NO_PLAY 를 뺀 플레이한 기록 수). 홈과 달리 카탈로그 없이 기록만으로 센다.

### 2.7 시각화 방식 요약

| 요소 | 방식 | 근거 |
|---|---|---|
| 누적 램프 막대 | `div`(`flex`, 자식 `flex-grow = count`) | `lamp-distribution.tsx:34-45`, `rank-lamp-list.tsx:40-54` |
| 등급 분포 | 세로 막대 `div` 높이 % | `score-grade-chart.tsx:20-32` |
| 개수 막대 | `div` 너비 % | `bar-count-list.tsx:28-30` |
| 레이더 | 인라인 SVG `polygon`/`line`/`circle`, `vectorEffect='non-scaling-stroke'` | `notes-radar-chart.tsx:64-91` |
| 색 | 램프 `--iidx-*` CSS 변수 8개(`I:app/globals.css:293-299`), 나머지는 `muted`/`primary` 토큰 | `dashboard.ts:14-23` |
| 라이브러리 | 없음(`I:package.json` 의 의존성에 차트 패키지 없음) | **[확인]** |
| 애니메이션 | 없음 | **[확인]**(해당 코드 없음) |

### 2.8 iidx-rank 에 **없는** 것

**[확인]** `I:src` 를 `calendar|heatmap|trend|streak|recommend` 로 검색한 결과 테스트 외 파일에서 0건이다. 즉 레이팅/점수 추이 그래프, 활동 달력, 연속 플레이, 약점·추천, 난이도별 클리어율 추이는 iidx-rank 대시보드에도 없다. "iidx-rank 처럼 최대한"의 상한은 위 6패널이고, 그 이상은 R-BMS 가 새로 만드는 것이다.

---

## 3. R-BMS 웹의 현재 "/"

### 3.1 구성

- `W:app/(app)/page.tsx:21-44`: `ShellPage title='홈'`(`contentClassName='@container/dashboard gap-0'`) 안에 `DASHBOARD_GRID_CLASS_NAME` 격자. 첫 패널 `HomeIntroPanel`(정적), 그 뒤 `<Suspense>` 안의 `HomeContent`. Suspense 폴백은 서버 현황·최근 활동 로딩 패널 둘(29-38).
- `W:widgets/home/home-content.tsx:12-29`: `await connection()` 후 `statsSummaryQueryOptions()`·`activityRecentQueryOptions({limit:20})` 를 프리페치(`queryFn` 을 `readStatsSummary`·`readRecentActivity` 로 바꿔 서비스 직접 읽기)하고 `HydrationBoundary` 로 `StatsSummaryPanel`, `RecentActivityPanel` 을 감싼다.
- 패널: `HomeIntroPanel`(`widgets/home/home-intro-panel.tsx:10-34`: 설명 + 하이라이트 3줄 + 가이드/발광 표/검색 버튼), `StatsSummaryPanel`(`stats-summary-panel.tsx:15-50`: 등록 차트 수 큰 숫자 + 플레이어·누적 제출), `RecentActivityPanel`(`recent-activity-panel.tsx:18-55`: 전체 사용자의 최근 제출 20건 `ScoreRow` 목록, A-SCR 표지, 각주).
- 격자(`W:shared/constants/dashboard.ts:1-5`): 1/2/3/4열(45/62/96rem), 행 높이 지정 없음(내용 높이), `DASHBOARD_WIDE_PANEL_CLASS_NAME` 은 62rem 부터 2칸, `FILL` 은 96rem 부터 2칸, `FULL` 은 `col-span-full`. iidx-rank 와 달리 고정 행 높이·`min-h`·`FittedRowList` 가 없고 페이지가 `ScrollContainer` 로 스크롤한다(`W:widgets/shell/shell-page.tsx:13-18`).
- 메타데이터: `generateMetadata`(`page.tsx:12-19`)가 `absolute` 제목과 canonical `/`.
- 문구: `pages.route.home`(title "홈", serverStatus, recentActivity…, `W:messages/ko/pages.json` route.home)와 `browse.home.{intro,recent,stats}`(`W:messages/ko/browse.json`).
- 기존 시험: `W:tests/components/home-recent-activity-panel.test.tsx`(A-SCR 표지 한 건). 손님 홈 변경 시 유지해야 한다.

### 3.2 로그인 여부를 아는 현재 방법

| 위치 | 방법 | 근거 |
|---|---|---|
| 프록시 | 세션 **쿠키 존재만** 본다(DB 없음). `/settings`·`/admin`·`/folders`·`/ai`·`/charts/:hash`·`/players/:id`·`/courses/:hash`·`/replays/:id` 만 matcher 에 있다. **`/` 는 없다** | `W:proxy.ts:33,37-48` |
| 서버 컴포넌트 | `readSessionUser()`: `headers()` → `getAuth().api.getSession({headers})` → `{id, login_id, name, email, role}` 또는 `null`. 설정·관리자·폴더·표 화면이 쓴다 | `W:widgets/auth/session-gate.tsx:9-22` |
| 서버 게이트 | `SessionGate`: 없으면 `redirect(buildLoginHref(next))` | `session-gate.tsx:28-32`, `W:app/(app)/{settings,folders,ai,admin}/layout.tsx` |
| 서버에서 손님도 허용 | `TableSnapshotSession`: `readSessionUser()` 를 try/catch 로 감싸 실패는 `null`, 사용자가 있으면 `queryClient.prefetchQuery` 로 그 사용자 기록을 담아 `initialUserId` 로 클라이언트에 전달 | `W:widgets/table/table-snapshot-session.tsx:12-39` |
| 클라이언트 | `useSessionUser()`: better-auth `authClient.useSession()` 을 읽고 하이드레이션 전에는 `isPending: true`(서버 HTML 과 어긋남 방지). `login_id` 가 플레이어 id | `W:entities/auth.query.ts:81-87`, `W:shared/hooks/use-is-hydrated.ts:10` |
| 표 화면의 클라이언트 정렬 | `const userId = isSessionPending ? initialUserId : (user?.id ?? null)` | `W:widgets/table/table-snapshot-workspace.tsx:88` |
| 로그인 후 이동 | 전체 새로고침 `window.location.assign(next)`(기본 `/`), 가입은 `/settings` | `W:widgets/auth/login-form.tsx:30-31,45`, `signup-form.tsx:53-54`, `W:shared/constants/auth-route.ts:2` |
| 로그아웃 | 보호 경로가 아니면 `router.refresh()` (서버 컴포넌트가 다시 렌더) | `W:widgets/shell/shell-account-control.tsx:36-43` |
| 세션 설정 | 만료 30일, 갱신 1일, **쿠키 캐시 설정 없음**(서버 세션 조회는 DB) | `W:server/lib/auth.ts:9-10,22` **[확인]** / DB 조회 여부는 **[추정]**(better-auth 내부 동작은 읽지 않았다) |

### 3.3 렌더와 캐시 현황

- 루트 레이아웃이 `getLocale()`(쿠키·헤더)을 읽어 모든 페이지가 요청마다 렌더된다. `export const instant = false`(`W:app/layout.tsx:22`), 근거 `docs/web/architecture.md:817`(§16-2), `docs/acknowledge/2026-10-24-adr-web-i18n.md` 결정 4. 데이터 캐시(`'use cache'`)는 그대로다.
- `cacheLife('minutes')` 프로필: stale 5분, revalidate 1분, expire 1시간(`node_modules/next/dist/docs/01-app/03-api-reference/04-functions/cacheLife.md:143`).
- Next 문서(`authentication-with-cache-components.md:15,47-60`): 세션 읽기는 요청 시점이라 정적 셸에 못 들어가고 `<Suspense>` 뒤에서 스트리밍한다. 세션에서 뽑은 값(`userId`)을 **인자로 넘겨** `'use cache'` 함수에 넣는 방식으로 세션 파생 데이터를 서버 캐시할 수 있다(같은 문서 Step 4, 225-267행 제목). R-BMS 의 `getPlayerStatsCached({loginId, userId})`(`read-cache.ts:291-297`)가 이미 이 방식이다.

---

## 4. R-BMS 웹이 이미 가진 사용자 데이터와 쿼리

### 4.1 플레이어 단위 읽기(공개, `withPublicRead` = IP 당 분당 120회)

공통: `withPublicRead = withErrorHandling(withRateLimit('fe-read', PUBLIC_READ))`(`W:server/route/fe.ts:38-39`, `with-rate-limit.ts:8`). 서버 프리페치는 `public-reads.ts` 의 `read*` 로 `queryFn` 을 바꿔 끼운다(`ssr-no-self-fetch.test.ts` 가 강제, 7.2절).

| 데이터 | 훅(`entities`) | 경로 | 서버 캐시 | 응답 핵심 필드 | 계산·SQL |
|---|---|---|---|---|---|
| 통계 | `usePlayerStats(id)` `player.query.ts:18-22,34` | `GET /api/fe/players/{id}/stats` (`fe.ts:69-73`) | `'use cache'` minutes, 태그 `player-stats:{id}` (`read-cache.ts:291-297`) | `total_plays, cleared, lamps[{clear, assist_grade, count}], scratch_auto_charts` (`player.type.ts:19-24`) | `fe.service.ts:116-124`: `cleared` = `clear >= MIN_CLEAR_LAMP_ID` 합, 램프 분포는 `toLampDistribution`(73-80). SQL `compose/fe.ts:194-201`: `chart_best` 를 `user_id` 로 걸러 `(clear, assist_grade, scratch_auto)` 로 `GROUP BY`, 범위는 `chartBestScope(db,true)`(일반 판과 SCRATCH AUTO 판 중 더 높은 쪽, `best-scope.ts:94-101`). 총 제출은 별도 `countPlayerScores`(`compose/fe.ts:319`) |
| 일별 플레이 시간 | `usePlayerPlayDays(id)` `player.query.ts:24-28,36` | `GET /api/fe/players/{id}/play-days` (`fe.ts:75-79`) | minutes, 태그 `player-play-days:{id}`, **캐시 키에 KST 오늘 0시** (`public-reads.ts:108-111`) | `days[30]{date(KST), plays, play_ms}, total_plays, total_ms` (`player.type.ts:26-36`) | `player-play-days.ts:7-9,35-48`: 창 30일, 앞 판과 간격 10분 이하면 그 간격, 아니면 2분. SQL `compose/fe.ts:203-218`: `score.played_at` 만 두 번 조회(창 안 전부 + 창 직전 1건), 인덱스 `score_player_idx(user_id, played_at)` |
| 최근 판 | `usePlayerRecent(id,{page,limit})` `player.query.ts:12-16,32` | `GET /api/fe/players/{id}/recent?page&limit` (`fe.ts:62-67`) | minutes, 태그 `player-recent:{id}` (`read-cache.ts:284-289`) | `ActivityRow`: `score_id, player, player_name, clear, assist_grade, ex_score, minbp, max_combo, played_at, scratch_auto, chart{md5, sha256, title, artist, level}` (`stats.type.ts:20-32`). 모든 판(ranked 무관). `limit` 기본 20·최대 100(`server/dto/fe.ts:6-7`) | SQL `compose/fe.ts:154-167`: `score` ⨝ `user` ⨝ `chart`(왼쪽), `played_at, id` 내림차순, `count(*)` 따로. **곡당 1행이 아니라 판당 1행**. 노트 수·최대 EX 는 안 실린다 |
| 프로필 | `usePlayer(id)` `player.query.ts:6-10,30` | `GET /api/players/{id}`(네이티브 공용, 봉투 없음) | minutes, 태그 `player:{id}`, `player-profiles` (`read-cache.ts:62-67`) | `id, name, total_plays, rank_points, extra.dans[{system, name, label, assisted}]` (`player.type.ts:4-10`, `readDisplayedDans` `dan.type.ts:17`) | `player.service.ts:35-37` |
| 서버 현황 | `useStatsSummary()` `stats.query.ts:6-10,24` | `GET /api/fe/stats/summary` | minutes, 태그 `stats-summary` | `charts, players, scores` | `compose/fe.ts:220-225` 의 `count(*)` 세 번 |
| 종합 순위 | `usePlayerLeaderboard(params)` `stats.query.ts:18-22,28` | `GET /api/fe/leaderboards/players?sort&page&limit&include_scratch_auto` | minutes, 태그 `leaderboard-players` | `PlayerProfile + dans + cleared_charts + played_charts` (`stats.type.ts:41-45`). 정렬 `cleared`(기본)·`played`·`plays` | `compose/fe.ts:73-103,169-192`: `score` 와 `chart_best` 를 **전 사용자 `GROUP BY`** 한 뒤 조인(무거움). 내 위치를 주는 필드는 없다 |
| 라이벌 | `useRivalList(id)` `rival.query.ts:6-12` | `GET /api/players/{id}/rivals` | `'use cache'` minutes, 태그 `player-rivals:{id}` (`read-cache.ts:173-178`; `architecture.md` 5-2 표는 hours 로 적었으나 코드는 minutes) | `PlayerProfile[]` | `rival(user_id, rival_id)` `schema.ts:216-224` |

### 4.2 내 데이터(세션 필요, 서버 캐시 없음, `private, no-store`)

| 데이터 | 훅 | 경로 | 응답 | 비고 |
|---|---|---|---|---|
| 내 단위 | `useMyDans()` `dan.query.ts:6-12` | `GET /api/fe/me/dans` | `systems[{system, name, display{mode,label,assisted}, shown[], acquired[{label, assisted, rank, course_hash, course_name, course_gauge, acquired_at}]}]` (`server/dto/dan.ts:50-58`) | 배지 `DanBadge`/`DanBadgeList`(`W:features/dan/*`), A-단위는 점선 테두리(`dan-badge.tsx:6-17`) |
| 내 폴더 | `useGetFolderList(enabled)` `folder.query.ts:12-16,30` | `GET /api/fe/me/folders` | `FolderSummary{id, name, description, is_public, position, item_count, updated_at}` (`folder.type.ts:21`), 한도 50개 | 폴더 한도 상수 `FOLDER_LIMIT`(`folder.type.ts:3-12`) |
| 발광 표 내 기록 | `useGetTableSnapshotRecords(tableId, userId)` `table-snapshot.query.ts:6-14` | `GET /api/fe/tables/{id}/records` | `{table_id, records: { [md5]: {clear, assist_grade, ex_score, minbp, max_combo, total_notes, played_at, scratch_auto} }}` (`server/dto/table-snapshot.ts:87-101`). 표의 모든 md5(발광 1,033곡)를 `IN` 한 번, 약 150KB | 사용자당 분당 30회(`table-snapshot.ts:83`, `TABLE_RECORDS`). SQL `compose/table-snapshot.ts:114-133`: `chart_best` ⨝ `chart` ⨝ `score` |
| 표 목록·상세 | (`fe/tables`, `fe/tables/{id}`) | 공개, 정적 번들 + DB 사본 | `TableSummary{id, name, symbol, entry_count, level_count, last_update, levels[{level,count}], standard_paths, source}` (`server/dto/table-snapshot.ts:70-79`) | 표 3종: `insane1`(번들), `stella-st`·`stella-sl`(DB 사본). 상세 약 307KB |
| AI 사용량·키 | `useAiUsageSummary()`, `useAiCredentialList()`, `useAiUserSetting()` `ai.query.ts:40-80` | `GET /api/fe/me/ai/{usage,credentials,settings}` | `usage{day, month, today, this_month, limits{daily_calls, concurrent_runs, monthly_token_limit}}` (`server/dto/ai.ts:126-136`) | `AiUsageMeter`(`W:features/ai/ai-usage-meter.tsx:15-29`) |

### 4.3 AI 결정론 digest(GET 은 공급자를 부르지 않는다)

`docs/web/architecture.md` §12-1(`652행~`): GET 은 digest 와 마지막 저장 결과를 주고 **공급자를 호출하지 않는다**. AI 해석은 POST 와 버튼으로만. 라우트는 `aiPrivateJson`(`Cache-Control: private, no-store`, `W:server/route/ai.ts:71,87`)이고 GET 은 `withLimited` → `enforceUserRateLimit(scope, user.id, TABLE_RECORDS)` = **사용자당 분당 30회**(`server/route/ai.ts:326-345`, `with-rate-limit.ts:12`). 캐시 없음, 요청마다 DB.

| digest | 훅 `ai.query.ts` | 경로 | 응답 핵심(대시보드에 쓸 부분) |
|---|---|---|---|
| 주간 요약 | `useAiWeekly(week\|null)` 244-247,298 | `/fe/me/ai/weekly` | `week{start,end,in_progress}`, `totals{plays, charts, active_days, first_plays, lamp_ups, new_clears, ex_ups, fast, slow, fast_share, mean_offset_ms}`, `previous`(앞 주), `lamp_changes[]`, `ex_changes[]`, `levels[]` (`server/dto/ai.ts:507-516`). 주는 UTC 월~일 |
| 다음 도전 곡 | `useAiRecommend(tableId\|null)` 218-222 | `/fe/me/ai/recommend` | `table`, `frontier`(경계 레벨), `levels[{key, level, charts, played, cleared, hard_or_better, clear_rate}]`, `candidates[]`(kind failed/unplayed/lamp_up, score) (`server/dto/ai.ts:319-345`). `levels` 가 **레벨별 클리어율**을 이미 서버에서 계산해 준다 |
| 목표 | `useAiGoalList()` 230-234,303 | `/fe/me/ai/goals` | `AiGoal{id, params, created_at, progress{label, target, achieved, done, remaining, recent_updates}\|null}` (`server/dto/ai.ts:439-453`). 활성 최대 10개 |
| 판정 습관 | `useAiJudgeHabit()` 248-252,300 | `/fe/me/ai/judge-habit` | `fast_slow{fast, slow, fast_share, grade, leaning}`, `offset{suggested_shift_ms, direction, ...}`, `green{latest, median}` (`server/dto/ai.ts:537-592`) |
| 플레이 리포트 | `useAiReport()` 254-258,284 | `/fe/me/ai/report` | `plays{analyzed, measured, ...}`, `tags[{tag, residual, grade, leaning}]`(패턴 강약), `lanes`, `timing`, `options` (`server/dto/ai-play.ts:66-103`). **판별 trace 를 올린 판이 있어야** 채워진다 |
| 게이지 조언 | `useAiGaugeAdvice()` 272-276,295 | `/fe/me/ai/gauge-advice` | `gauges[]`, `rescues[]`, `options[]` (`ai-play.ts:232-259`) |
| 연습 폴더 | `useAiPracticeFolder(source)` 224-228,301 | `/fe/me/ai/practice-folder` | `candidates[]`, `folders{used, limit, can_create}` (`server/dto/ai.ts:382-391`) |

### 4.4 R-BMS 에 **없는** 데이터

- 팔로우, 공개/비공개 프로필, 아바타, 게시판: 없다(`user` 에 해당 열 없음, `W:server/lib/auth.ts:23-32` 의 추가 필드는 `loginId, rank, rankPoints, totalPlays, role, allowGuestMerge`). `rank`·`rankPoints` 는 있으나 순위 화면은 쓰지 않는다(`docs/web/architecture.md:165`).
- e-amusement 가져오기(DJ POINT, SP/DP 플레이 수, 단위 문자열, 노트레이더 값): 없다. 웹 코드에 `radar` 가 한 줄도 없다(`grep -rli radar web/src` 0건 **[확인]**). 노트레이더는 클라이언트 곡 DB(`song_radar`)의 곡 단위 값이고 웹에 올라오지 않는다(`docs/HANDOFF.md:33`).
- 곡당 DJ 랭크: `chart_best` 에 `ex_score` 는 있고 노트 수·최대 EX 는 없다(`schema.ts:142-165`). 노트 수는 `score.notes`(`schema.ts` score 표). 변환은 `getDjRank(ex, maxEx)` 27단계(`W:shared/lib/dj-rank.ts:38-47`).
- 활동 달력·레이팅 추이: 없다(일별 플레이 시간 30일만 있다).
- 내 코스 진행: `course_best`(`schema.ts:311`)는 있으나 "내 코스 기록 목록" 엔드포인트가 없다(코스별 순위·내 최고만).
- 내 리플레이 목록: `replay.user_id` 인덱스(`schema.ts:167-198`)만 있고 사용자별 목록 엔드포인트가 없다(차트별 목록만, `entities/replay.query.ts:12-16`).

### 4.5 재사용 가능한 UI

| 컴포넌트 | 경로:줄 | 용도 | 재사용 시 주의 |
|---|---|---|---|
| `DashboardPanel`, `-Empty/-Error/-Loading` | `features/dashboard/*` | 패널 틀 | 그대로. iidx-rank 와 동일 계약 |
| `LampDistribution` | `features/lamp/lamp-distribution.tsx:18-51` | 플레이/비율 + 누적 막대 + 2열 범례 | props `lamps: LampCount[]`, `played`, `total`. 라벨 `pages.lamp.distribution` |
| `LampStackBar` | `features/lamp/lamp-stack-bar.tsx:17-32` | 누적 램프 막대(높이 12px) | 개수 0 제외 |
| `LampSwatch`, `LampLabel`, `LampStrip` | `features/lamp/*` | 견본/이름/행 띠 | 어시스트 등급 5색 지원(`clear-lamp.ts:52-86`) |
| `PlayerStatsPanel` | `widgets/player/player-stats-panel.tsx:22-102` | 타일 3개 + 램프/일별 전환 | `playerId` 하나로 동작. 대시보드에 그대로 넣을 수 있으나 `/players/{id}` 와 내용이 겹친다 |
| `PlayerPlayDays`, `PlayDayChart`, `PlayDayTable` | `widgets/player/player-play-days.tsx:13-43`, `features/stats/*` | 30일 막대 + 표 | 막대 `h-28`, 최소 2%, 색 `bg-primary` |
| `PlayerRecentPanel` | `widgets/player/player-recent-panel.tsx:27-85` | 최근 판 + 페이지 이동 + "코치" 링크 | `useRouter`·`Pager` 가 들어 있어 대시보드용은 목록만 떼야 한다(`ScoreRow` 직접 사용) |
| `ScoreRow` | `features/score/score-row.tsx:22-67` | 램프 띠 2줄 행(EX, BP, 시각, A-SCR) | DJ 랭크 표기 없음 |
| `StatTile` | `features/shell/stat-tile.tsx:17-25` | 라벨/큰 값/힌트 | |
| `TableProgressPanel` | `features/table/table-progress-panel.tsx:29-75` | 접히는 표 진행(전체 막대 + 레벨별 막대) | props `lamps, played, total, levels[]` 를 부모가 계산해 넘긴다(`table-snapshot-workspace.tsx:180-195`) |
| `AiTagResidualList` | `features/ai/ai-tag-residual-list.tsx:15-41` | 0 기준 좌우 막대(패턴 잔차) | 레이더 대안 |
| `AiUsageMeter` | `features/ai/ai-usage-meter.tsx:15-29` | 사용량/한도 `Progress` | |
| `DanBadgeList` | `features/dan/dan-badge-list.tsx` | 단위 배지 | |
| `BarCountList` | `features/stats/bar-count-list.tsx:11-38` | 라벨+막대+개수 목록 | **[확인]** 정의만 있고 어디서도 쓰지 않는다(`grep` 0건). 대시보드가 첫 사용처가 될 수 있다 |
| `FolderListItem` | `features/folder/folder-list-item.tsx` | 폴더 행(수정/삭제 메뉴 포함) | 대시보드용은 읽기 전용 행이 따로 필요 |

R-BMS 에 없는 iidx-rank 부품: `FittedRowList`(보이는 높이만큼만 행 표시), SVG 레이더, `CommentsSeen`.

---

## 5. iidx-rank 섹션별 R-BMS 대응표

판단 근거는 4절이다. "데이터"는 R-BMS 서버가 이미 그 값을 줄 수 있는가이다.

| iidx-rank 섹션 | R-BMS 데이터 | 필요한 새 쿼리 | 재사용 컴포넌트 |
|---|---|---|---|
| 내 정보: 이름·핸들·공개·팔로워 | 일부: 이름·`login_id`·총 플레이·대표 단위(`usePlayer`). 팔로워·공개 여부·아바타는 없음 | 없음 | `DanBadgeList`, `DashboardPanel` |
| 내 정보: e-amusement 플레이어 정보와 가져오기 요약 | 없음(해당 개념이 없다) | 대체: 마지막 제출 시각은 `recent` 1건, 서버 순위는 4.1절 종합 순위 | `StatTile` |
| 노트레이더(6축 SVG) | 없음(웹에 레이더 없음) | 대체 후보 A: AI 리포트 `tags[].residual` 강약(trace 업로드 필요), 후보 B: 판정 습관 `fast_share`. 둘 다 기존 GET | `AiTagResidualList`, 새 레이더 SVG 는 만들 수 있으나 입력축 정의가 먼저(**미결**) |
| 램프 현황(전체 차트 대비 비율 + 누적 막대 + 범례) | 있음: `usePlayerStats().lamps`. 단 분모가 "카탈로그 전체"가 아니라 **내가 플레이한 차트 수**(`lamps` 합)다. NO PLAY 는 없다 | 선택: 표 기준 분모(발광 1,033곡)로 보려면 표 기록 필요 | `LampDistribution`, `LampStackBar`, `PlayerStatsPanel`(`player-stats-panel.tsx:42-48`) |
| DJ 랭크 분포(8단계) | 없음(분포 쿼리 없음). R-BMS 등급은 27단계 | `chart_best` ⨝ `score(notes)` 로 `getRateStep` 별 개수(서버 집계), 또는 표 기록에서 클라이언트 계산(`records[md5].ex_score`·`total_notes`, 표 곡 한정) | 새 세로 막대(CSS), `getRateStep`(`dj-rank.ts:38-41`) |
| 랭크별 램프(노멀/하드 × 10랭크) | 대응 개념 = **표 레벨별 램프**(발광 표 레벨, Stella st/sl). 표 기록은 있으나 레벨별 집계는 클라이언트가 표 상세와 합쳐야 한다. `ai recommend.levels` 는 레벨별 클리어율을 주지만 램프 분포는 아니다 | 권장: 서버 집계 `표 id → 레벨별 램프 개수`. 토글은 노멀/하드가 아니라 표 선택(3종) | `TableProgressPanel`, `LampStackBar`, `countTableLamps`(`table-snapshot-filter.ts:181-187`) |
| 최근 플레이곡 20곡(곡당 1행, 갱신 시각) | 있음, 단 **판당 1행**(같은 곡 반복 가능). `usePlayerRecent(login_id,{page:1,limit:20})` | 선택: 곡당 최신으로 중복 제거하려면 쿼리 추가. DJ 랭크를 보이려면 `activitySelection`(`compose/fe.ts:14-30`)에 `score.notes` 추가 | `ScoreRow` |
| 게시판 활동 | 없음(게시판 없음) | 대체: 6절 후보(폴더, 라이벌 활동, 리플레이, AI) | - |
| 손님 6패널 | 이미 R-BMS 손님 홈이 별도 구성 | 결정에 따라 유지 | `HomeIntroPanel` 등 |

---

## 6. R-BMS 에만 있는 데이터: 대시보드 후보

"쿼리" 열의 **기존** 은 새 서버 코드 없이 쓸 수 있다는 뜻이다.

| 후보 | 보여줄 것 | 데이터·쿼리 | 비용·주의 | 컴포넌트 |
|---|---|---|---|---|
| 일별 플레이 시간 | 30일 막대 + 합계 | **기존** `usePlayerPlayDays` | 캐시됨(minutes). 추정 시간임을 각주로(`browse.player.playDays.note`) | `PlayDayChart` |
| 활동 달력 | 최근 N주 일별 판 수 격자, 연속일 | **새**: `play-days` 창을 90~365일로 확장, 일 단위 `GROUP BY`(KST) | `score_player_idx` 가 `played_at` 을 덮어 인덱스만 읽을 수 있다 **[추정]**. 365일×사용자 판 수에 비례 | 새 CSS 격자 |
| 발광 표 진행도 | 표 3종 전체 + 레벨별 램프 막대, 클리어 n/전체 | **기존**(표 상세 + `/records`, 클라이언트 합산) 또는 **새**(서버 레벨별 집계 권장) | `/records` 약 150KB·사용자당 분당 30회, 표 상세 약 307KB. 표 3종이면 6개 요청 | `TableProgressPanel` |
| 다음 도전 곡 | 경계 레벨, 클리어율, 상위 후보 3~5곡 | **기존** `useAiRecommend(tableId)` | AI 라우트 분당 30회, 요청마다 표 기록 + 곡별 패턴 읽기. 4.3절 | `AiChartCandidateList`(`features/ai/ai-chart-candidate-list.tsx`) |
| 이번 주 | 판 수, 활동일, 램프 갱신, 첫 클리어, 앞 주 대비 | **기존** `useAiWeekly(null)` | 같음. 주는 UTC 월요일 시작(화면은 KST 도 쓰므로 혼재) | 표 + 증감 |
| 목표 진행 | 목표별 `achieved/target` 막대, 남은 곡, 최근 28일 갱신 | **기존** `useAiGoalList()` | 목표 없으면 빈 상태 + `/ai?tab=goals` 링크 | `Progress`(`shared/ui/progress`) |
| 판정 습관 | FAST/SLOW 비율, 오프셋 제안 | **기존** `useAiJudgeHabit()` | 최근 100판, 증거 등급 표시 필수 | 텍스트 + 작은 막대 |
| 플레이 성향 | 패턴별 강약 | **기존** `useAiReport()` | trace 업로드가 없으면 `notice` 만 온다 | `AiTagResidualList` |
| 단위·A-단위 | 체계별 보유 단위, 남에게 보일 단위 | **기존** `useMyDans()` | 세션, no-store | `DanBadge` |
| 어시스트(SCRATCH AUTO) | 어시스트 램프 개수, A-SCR 곡 수 | **기존** `stats.lamps`(어시스트 등급 5종) + `scratch_auto_charts` | 램프 분포에 이미 섞여 있다 | `LampDistribution`, `ScratchAutoBadge` |
| 내 폴더 | 최근 수정 폴더 N개, 곡 수, 새 폴더 | **기존** `useGetFolderList` | 세션, 50개 한도. AI 연습 폴더 이름은 `AI 연습 · …` 접두(`server/dto/ai.ts:353`) | 새 읽기 전용 행 |
| AI 훈련 폴더 | 훈련 폴더 상태, 만들 수 있는 폴더 수(`can_create`), 후보 | **기존** `useAiPracticeFolder` | 훈련곡 생성 쪽은 01·02 문서의 범위(**미확인**) | - |
| 라이벌 | 라이벌 목록, 라이벌 최근 판, 내 대비 | 목록 **기존** `useRivalList`. 최근 판·비교는 **새**: `rival` 로 사용자 집합을 만들어 `listRecentActivity` 를 `user_id IN` 으로 | 라이벌 수 상한 **[미확인]**. `rival.service.ts` | `ScoreRow`(subtitle=라이벌 이름) |
| 코스 진행 | 코스 묶음별 클리어 n/전체, 획득 단위 | **새**: `course_best` 를 사용자로 집계 + `useCourseCatalog`(`course.query.ts:21-25`)로 전체 수 | 코스 수는 카탈로그 의존 | 새 막대 |
| 내 리플레이 | 최근 리플레이 N건 → `/replays/{id}` | **새**: `replay` 를 `user_id` 로 최근순 조회 + `chart` 조인 | 리플레이 본문(`data`)을 읽지 않도록 열 선택 필수(`schema.ts:167-198` 의 `data mediumtext`) | `ScoreRow` |
| 종합 순위 위치 | "N위 / 전체", 앞뒤 플레이어 | **기존** `usePlayerLeaderboard` 상위 N쪽을 받아 내 `login_id` 찾기, 못 찾으면 "N위 밖" | 새 순위 쿼리는 전 사용자 `GROUP BY`(`compose/fe.ts:73-103`)라 비싸다. 기존 캐시 응답 재사용 권장 | `StatTile` |
| AI 사용량·키 상태 | 오늘/이달 토큰, 키 등록 여부 | **기존** `useAiUsageSummary`, `useAiCredentialList` | 키 없으면 설정 링크(`AI_SETTINGS_HREF`, `navigation.ts:21-22`) | `AiUsageMeter` |
| 서버 현황 | 등록 차트·플레이어·누적 제출 | **기존** `useStatsSummary` | 캐시됨. 결정상 손님 홈을 유지한다면 회원 대시보드에는 작은 타일로만 | `StatTile` |
| 전체 최근 활동 | 커뮤니티 최근 제출 | **기존** `useActivityRecent({limit})` | 캐시됨 | `ScoreRow` |

---

## 7. 웹 규칙과 대시보드 위젯을 더할 때 지킬 것

### 7.1 파일 배치(FSD, `docs/web/architecture.md` 2·2-1절, 213-214·246행)

- 의존은 `app → widgets → features → entities → shared` 아래로만. barrel 금지. 경로 별칭 `@app @widgets @features @entities @shared @server`.
- `features/*` 는 props 만 받는 순수 UI(비즈니스 로직 없음), `widgets/*` 가 쿼리 훅·상태 분기·번역을 한다. 선례: `PlayerPlayDays`(위젯)가 `PlayDayChart`/`PlayDayTable`(features)에 props 를 넘긴다(`docs/web/components.md:260-264`).
- `@server/*` 는 `app/api/**` 와 **서버 컴포넌트**에서만 import 하고 파일 맨 위에 `import 'server-only'`(`public-reads.ts:1`, `table-snapshot-session.tsx:1`). `widgets/home/home-content.tsx` 는 서버 위젯이 `@server/dto/fe`·`@server/service/public-reads` 를 import 하는 선례다.
- 새 파일 제안 **[제안]**: `widgets/home/member-dashboard-content.tsx`(서버: 세션 + 프리페치 + `HydrationBoundary`), `widgets/home/member-*-panel.tsx`(클라이언트 패널, 1파일 1컴포넌트), `features/dashboard/*`(새 순수 UI), `entities/dashboard.query.ts`·`dashboard.type.ts`(새 훅·타입. 도메인 단위 파일 규칙 `<domain>.query.ts`, `architecture.md:213-214`), 서버 집계는 `server/service/domain/<d>/` + `compose/` + `route/fe.ts` + `app/api/fe/.../route.ts` 3~5줄 어댑터.
- 컴포넌트 규칙(전역 컨벤션 + 코드 관찰): arrow function, `FC<Props>`, `useMemo`/`useCallback` 금지(React Compiler `next.config.ts:26`), 한 파일 한 export, `useEffect` 최소.

### 7.2 쿼리 규칙

- 키는 `shared/constants/query-key.ts`(`QUERY_KEY`, 102줄)에 도메인 → 동작 → 파라미터 배열로 추가한다. 기존 도메인: `AUTH, CHART, PLAYER{PROFILE, SCORES, RECENT, STATS, PLAY_DAYS, RIVALS}, SCORE, REPLAY, COURSE, DAN{MINE}, TABLE, TABLE_SNAPSHOT{RECORDS}, FOLDER, STATS, ACTIVITY, LEADERBOARD, TOKEN, SETTING, AI{…DIGEST…}, ADMIN, SYSTEM`.
- 조회는 `queryOptions` 팩토리 + `use*` 훅(`player.query.ts` 형식). `entities` 에는 `*.api.ts` 가 없고 `fetchEnvelope`/`fetchEnvelopePage`/`fetchRaw` 를 직접 쓴다(`architecture.md:246-248`).
- 서버 프리페치: 요청마다 `getQueryClient()`(서버는 새 인스턴스, `get-query-client.ts:42-46`), `prefetchQuery({...xQueryOptions(…), queryFn: () => readX(…)})` 로 **서비스 직접 읽기**를 끼운다. `queryFn` 이 없는 프리페치가 있으면 시험이 실패한다(`W:tests/lib/ssr-no-self-fetch.test.ts:9,33-43`, 서버 위젯·앱 파일 전부를 훑고 호출 수 10 이상을 요구).
- 기본 `staleTime` 60초, `gcTime` 10분, `retry: 1`, `refetchOnWindowFocus: false`(`get-query-client.ts:5-6,27-31`). 서버 프리페치는 "첫 화면에 반드시 보이는 데이터에만"(`query.md` §6).
- 사용자 전환 시 사용자 범위 캐시를 비운다: `removeUserScopedQueries`(`W:entities/auth.query.ts:17-30`)의 `USER_SCOPED_QUERY_KEYS` 목록에 새 사용자 범위 키(`QUERY_KEY.X.ALL`)를 더해야 로그아웃 후 다음 계정에 남지 않는다 **[확인: 목록이 명시적이라 새 키가 빠지면 누수]**. 현재 목록은 AUTH, FOLDER, TABLE_SNAPSHOT.RECORDS_ALL, TOKEN, SETTING, AI, ADMIN.AI_LIMITS 이다 **[확인]**. 세션 전용인 `DAN.MINE`(`dan.query.ts:6-10`)은 이 목록에 없다 **[확인]**. 로그아웃 뒤 다음 계정에 남는지는 실행으로 확인하지 않았다 **[미확인]**. 새로 추가하는 세션 전용 키(대시보드의 표 진행도·라이벌 활동 등)는 이 목록에 더해야 한다.

### 7.3 캐시 태그와 무효화

- 서버 캐시는 `'use cache'` + `cacheLife` + `cacheTag`(`read-cache.ts`)이고 `'use cache'` 안에서 `cookies()`·`headers()`·`getLocale()` 을 부르지 않는다(`architecture.md:376`). 사용자 값은 **인자**로 넘긴다(캐시 키에 들어간다).
- 새 사용자별 읽기를 캐시하면 `CACHE_TAG` 에 태그를 더하고 `revalidateOnScoreSubmit`(`read-cache.ts:76-85`)에서 무효화해야 새 판이 올라와도 낡은 값이 남지 않는다. 안 하면 minutes 프로필의 revalidate 1분, expire 1시간 사이에 어긋난다(`cacheLife.md:143`). 태그 사전·무효화 표는 `architecture.md` 5-2·5-3절이므로 같은 작업에서 고친다(저장소 규칙).
- 개인화 데이터(표 기록, 폴더, 단위, AI digest)는 지금도 캐시하지 않는다(`architecture.md:376`, `server/route/ai.ts:71`).

### 7.4 i18n(세 언어)

- 문구는 `web/messages/{ko,en,ja}/{common,ai,settings,browse,pages,server}.json` 여섯 파일에만 둔다. 네임스페이스를 새로 만들려면 `MESSAGE_NAMESPACES`(`W:shared/i18n/messages.ts:21`)와 import 를 함께 고친다. 한국어가 원본이고 en·ja 는 키가 빠지면 `tsc` 가 막는다.
- `tests/lib/i18n-messages.test.ts` 가 세 언어의 파일 목록·키 집합·ICU 자리표 일치·빈 값 금지를 검사한다(`i18n-messages.test.ts:20-40`). 새 문구는 ko/en/ja 를 한꺼번에 추가한다. 일본어는 모델 초안이다(`HANDOFF.md:103,91`).
- 서버 컴포넌트는 `await getTranslations('pages.…')`, 클라이언트는 `useTranslations('browse.…')`. 현재 홈 문구 위치는 `browse.home.*`, `pages.route.home`.
- 알려진 어긋남: `formatCount` 가 `ko-KR` 고정(`W:shared/lib/format.ts:3`), 날짜는 KST 고정(`format.ts:7-24`), 램프 이름은 영어 상수(`clear-lamp.ts:20-32`). 대시보드 숫자가 영어·일본어 화면에서도 한국식 구분으로 나온다는 뜻이며 기존 화면과 같다.
- 시험 보조: `tests/support/browse-intl.tsx:7-11` 의 `withLocale(ui, locale)`.

### 7.5 디자인 토큰과 제약(`docs/web/components.md` 6절, `2026-10-14-design-direction.md`)

- 반경 0, 그림자 없음, 블록 분리는 부모의 `gap-px bg-border` 1px 틈. 글자 굵기 400/500/600 만(`font-bold` 금지). 색은 `var(--color-*)`·`var(--rbms-*)` 만이고 hex/rgb/Tailwind 팔레트 금지. 이모지 금지, 아이콘은 lucide. 로딩은 스켈레톤. **차트는 애니메이션하지 않는다**(`components.md:179`). 동적 className 은 `cn()`. 툴팁은 shadcn `Tooltip`(native `title` 금지 규정이나 `PlayDayChart` 는 `title` 속성을 쓴다, `play-day-chart.tsx:29-30` — 규칙과 구현이 이미 어긋나 있다).
- 램프 색 토큰: `--rbms-lamp-{noplay, failed, assist, light-assist, assist-easy, assist-clear, assist-hard, assist-exhard, easy, normal, hard, exhard, fullcombo, perfect, max}`(`W:app/globals.css:57-71`). 차트 보조색 `--chart-1..5`(113-117). 판정·타이밍 색 `--rbms-judge-*`, `--rbms-timing-fast/slow`(73-79). 모션 변수 `--motion-bar-duration: 0.24s`(153). 막대 너비 transition(`transition-[width] duration-(--motion-bar-duration)`)은 미사용 컴포넌트 `bar-count-list.tsx:23` 에만 있고 `PlayDayChart` 는 transition 이 없다. 새 막대에 transition 을 쓸지는 규칙(차트 비애니메이션, `components.md:179`)과 맞춰 정해야 한다 **[미결]**.
- 숫자는 `tabular-nums`, 우측 정렬. 해시는 mono(`components.md:167`). 라이트 테마 회색 글자는 `text-muted-foreground` 만(새 회색 단계 금지, `components.md:242`).
- 접근성: 패널 제목 `h2`, 시각 막대는 `role='img'` + `aria-label` 에 수치 문장(`lamp-stack-bar.tsx:19-21`), 색만으로 구분하지 않고 이름+개수 병기.

### 7.6 요청마다 렌더(`instant = false`)와 비용

**[확인]** 루트 `instant = false`(`app/layout.tsx:22`)이므로 `/` 는 이미 매 요청 렌더된다. 회원 대시보드를 같은 `/` 에서 분기하면 추가되는 것은 아래다. 질의 수는 코드에서 센 값이다(**측정 아님**).

| 단계 | 어디서 | 요청마다 DB | 서버 캐시 |
|---|---|---|---|
| 세션 읽기 | `readSessionUser()` | 세션(+사용자) 조회 1~2회 **[추정]** (`auth.ts:22`, 쿠키 캐시 없음) | 없음 |
| 플레이어 id → 사용자 id | `read*` 마다 `requireUserId`(`public-reads.ts:51-55`) | `read*` 호출마다 1회(통계·최근·일별 세 개면 3회 중복). 이미 세션에 `id` 가 있으므로 `getPlayerStatsCached({loginId, userId})` 를 직접 부르면 생략 가능 **[제안]** | 없음 |
| 통계 | `getPlayerStatsCached` | 적중 시 0, 실패 시 `countPlayerScores` 1 + `countLampsByUser`(`GROUP BY` + `NOT EXISTS` 형제 검사) 1 | minutes |
| 최근 판 20건 | `getPlayerRecentCached` | 적중 시 0, 실패 시 2(목록 조인 + count) | minutes |
| 일별 30일 | `getPlayerPlayDaysCached` | 적중 시 0, 실패 시 2(`played_at` 두 번) | minutes(키에 KST 오늘) |
| 발광 표 기록 | `tableSnapshotService.getRecords` | 표마다 1(약 1,033 md5 `IN`, 3표면 3) | 없음 |
| 내 단위·폴더 | 클라이언트 훅 | 각 1 | 없음 |
| AI digest | 클라이언트 훅 | 기능마다 표 기록 + 패턴 읽기 등 다수 **[미확인: 서비스 내부 질의 수는 읽지 않았다]** | 없음, 분당 30회 제한 |

- 서버에서 한꺼번에 프리페치하면 첫 HTML 에 값이 들어오지만 모든 질의가 TTFB 에 얹힌다. 첫 화면에 반드시 보이는 패널(통계, 최근, 일별)만 서버 프리페치하고 나머지(표 진행도, AI digest, 라이벌)는 클라이언트 지연 로딩(뷰포트 진입 후 `enabled`)으로 두는 구성이 규칙(`query.md` §6)과 맞다 **[제안]**.
- DB 풀은 `connectionLimit: 5`, 락 대기 5초(`W:server/db/index.ts:8,19` 의 `CONNECTION_LIMIT = 5` **[확인]**, 락 대기 5초는 `docs/reference/ai-features/server.md` 5.6절 기록이며 이 세션에서 다시 읽지 않았다). 회원 `/` 한 번이 연결을 여러 번 잡으면 동시 접속에서 병목이 될 수 있다 **[추정]**.
- 응답 본문 4.5MB 한도(Vercel)는 표 기록 150KB 수준에서는 문제없다(`server.md` 5.6).
- 공개 읽기는 IP 당 분당 120회(`with-rate-limit.ts:8`)지만 서버 프리페치는 서비스 계층을 직접 읽어 이 한도를 쓰지 않는다(`architecture.md:433`). 클라이언트 재조회(`staleTime` 60초 후)만 한도에 든다.

### 7.7 검증·문서 규칙

- 웹 게이트: `cd web && bun run verify && bun run format:check && bun run build`(`HANDOFF.md:109`). 새 SQL 이 있으면 DB 통합 `RBMS_DB_E2E=1 bun test tests/db`(같은 줄). web 시험은 bun 이 한 프로세스에서 파일을 돌려 sonner 토스트 같은 전역 상태가 샌다(시작 때도 비울 것, `HANDOFF.md:112`).
- 위젯 시험 선례: `tests/components/home-recent-activity-panel.test.tsx`(happy-dom 등록, `QueryClient` 에 `setQueryData(QUERY_KEY…)`, `withLocale`), 서버 서비스 시험 `tests/service/fe-player-history.test.ts`·`fe-player-play-days.test.ts`, 엔티티 와이어링 `tests/components/entity-query-wiring.test.ts`(fetch 를 가로채 경로 끝을 검사).
- 문서: 새 엔드포인트는 `docs/backend/api-spec.md`·`docs/web/architecture.md`(§1-3 표, §5-2 태그, §5-3 무효화, §5-4 페이지별 prefetch)를 같은 작업에서 갱신(`components.md:264-266` 이 일별 모드에서 데이터 계약·캐시·ADR 문서를 함께 가리키는 방식과 같다).
- 응답: 새 사용자 표시 문구는 세 언어, 용어집 `docs/reference/2026-10-24-i18n-glossary.md`. 다른 사이트의 흔적을 싫어한다는 사용자 방향(`HANDOFF.md:94`): 화면 문구에 IIDX 전용 용어를 새로 들이지 않는 편이 안전하다 **[제안]**. 이미 `getDjRank` 가 "DJ rank" 이름을 쓴다(`dj-rank.ts`).

---

## 8. 참고안(결정 아님)

### 8.1 `/` 분기 방식 세 가지

| 방식 | 내용 | 장점 | 단점 |
|---|---|---|---|
| A. 같은 페이지에서 서버 분기 | `HomePage` 의 `<Suspense>` 안에 서버 컴포넌트 `HomeBranch`: `readSessionUser()`(try/catch, `table-snapshot-session.tsx:12-18` 선례) → 회원이면 `MemberDashboardContent`, 아니면 기존 `HomeContent` | 기존 구조·선례와 같다. 로그인·로그아웃 후 `router.refresh()` 로 갱신된다 | `Suspense` 폴백은 정적 셸이라 viewer 를 모른다. 폴백을 회원/손님 중립으로 만들어야 한다. 손님도 쿠키가 없으면 DB 조회 없이 끝나도록 쿠키 확인을 앞에 둘 수 있다 **[추정]** |
| B. 프록시 재작성 | `proxy.ts` matcher 에 `/` 를 더하고 세션 쿠키가 있으면 내부 경로로 `NextResponse.rewrite`(404 재작성 선례 `proxy.ts:6,24-26`) | 정적 셸이 경로별로 따로 | 쿠키가 있다고 유효한 세션은 아니다(만료·위조). 내부 경로에서도 `readSessionUser` 로 다시 확인해야 한다. `/` 가 matcher 에 들어가면 모든 홈 요청이 프록시를 지난다. 내비 활성 표시(경로 접두사 계산)에 영향이 있는지 **[미확인]** |
| C. 클라이언트 분기 | iidx-rank 방식(`useViewerIdentity`) | 구현이 단순 | 하이드레이션 전에 viewer 를 모르므로 깜빡임. R-BMS 는 `useSessionUser` 가 하이드레이션 전 `isPending` 이다(`auth.query.ts:81-87`). 서버 프리페치와 섞기 어렵다 |

조사자 권고: **A**. 서버가 `initialUserId` 를 정하고 클라이언트 패널은 `useSessionUser()` 와 맞춰 불일치 순간(다른 탭에서 로그아웃)에는 스켈레톤을 보이는 iidx-rank 의 `isAligned` 장치(`use-viewer-identity.ts:11`)를 `table-snapshot-workspace.tsx:88` 형태로 가져온다.

### 8.2 패널 구성(1차: 새 쿼리 없음)

패널은 격자 한 칸씩이고 넓음은 62rem 이상에서 2칸이다. iidx-rank 와 같이 왼쪽 위부터 읽는 순서다.

1. 내 정보: 이름, `@login_id`, 대표 단위 배지, 총 제출 수(`usePlayer`), 마지막 제출 시각(`recent` 첫 항목), 내 페이지·설정 링크.
2. 램프 현황: `LampDistribution`(플레이한 차트 기준) + 클리어 비율 타일 + A-SCR 곡 수(`usePlayerStats`).
3. 최근 플레이 20판: `ScoreRow` 목록, "전체 보기" → `/players/{login_id}`.
4. 일별 플레이 시간 30일: `PlayDayChart` + 합계(`usePlayerPlayDays`).
5. 발광 표 진행도(넓음): 표 3종 탭 또는 3줄 요약 + 접힌 레벨별 막대(`TableProgressPanel`, 기존 표 상세 + 기록 사용. 서버 집계 전까지 무거움).
6. 단위: 체계별 보유·표시 단위(`useMyDans`).
7. 이번 주: 판 수, 활동일, 램프 갱신, 앞 주 대비(`useAiWeekly`) + 다음 도전 곡 3곡(`useAiRecommend`). 지연 로딩.
8. 내 폴더·AI: 최근 폴더 5개(`useGetFolderList`) + 목표 진행(`useAiGoalList`) + AI 사용량(`useAiUsageSummary`).

### 8.3 2차: 새 서버 집계

- 표 레벨별 램프 개수: `GET /api/fe/me/tables/{id}/progress` **[제안, 이름 미정]**. 기존 `listBestByMd5` + `groupTableByLevel`(`table-snapshot.service.ts`, `table-progress.ts:95-114` 의 `summarizeLevels` 와 비슷한 모양)을 합쳐 레벨당 `DISPLAY_LAMPS` 개수를 돌려준다. 응답이 수 KB 로 줄고 150KB+307KB 전송이 없어진다.
- DJ 랭크 분포: `chart_best.score_id → score.ex_score, score.notes` 로 `getRateStep` 별 개수. 27단계를 화면에서 8~9묶음(F..AAA, MAX)으로 줄일지 정해야 한다(**미결**).
- 활동 달력: `play-days` 확장(창·`GROUP BY` KST 일).
- 라이벌 최근 활동: `rival` → 사용자 집합 → 최근 판.
- 내 코스 진행, 내 리플레이, 종합 순위 위치(기존 응답 재사용 우선).

---

## 9. 열린 질문

1. 회원 `/` 에 손님 홈 패널(서버 현황, 전체 최근 활동)을 작게 남길지, 완전히 교체할지. "교체" 결정이므로 기본은 제거이나 서버 현황은 한 줄 타일로 남기는 안이 있다.
2. 노트레이더 자리를 어떻게 할지: 대체 후보 A(패턴 강약, trace 필요), B(판정 습관), 또는 새 6축 정의. 사용자가 레이더를 원하는지 확인이 필요하다.
3. "램프 현황" 분모: 플레이한 차트(기존 `stats`)로 충분한지, 표 기준(발광 1,033곡, 미플레이 포함)으로 보여야 하는지.
4. 최근 플레이를 판당(기존)으로 둘지, 곡당 최신으로 줄일지. DJ 랭크 표기를 원하면 `activitySelection` 에 `notes` 를 추가하는 서버 변경이 필요하다.
5. AI digest 패널을 대시보드에 자동 로딩해도 되는지(요청마다 DB, 분당 30회 제한). 기본은 지연 로딩 또는 "보기" 버튼.
6. 서버 분기 방식(A/B/C) 확정.
7. 활동 달력·DJ 랭크 분포·표 진행도 서버 집계를 1차에 넣을지 2차로 미룰지.
8. 일본어·영어 문구는 모델 초안으로 가는 기존 방침을 유지하는지(`HANDOFF.md:103`).
9. **[미확인]** 운영 응답 시간·질의 수 실측, `'use cache'` 저장소 위치, AI digest 서비스 내부 질의 수, better-auth `getSession` 의 DB 질의 수, `DAN.MINE` 이 로그아웃 때 비워지는지(`removeUserScopedQueries` 목록에 없음).
