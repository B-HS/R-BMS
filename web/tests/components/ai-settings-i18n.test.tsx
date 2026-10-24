import { afterAll, afterEach, describe, expect, test } from 'bun:test'
import { GlobalRegistrator } from '@happy-dom/global-registrator'
import { AppRouterContext } from 'next/dist/shared/lib/app-router-context.shared-runtime'
import { SearchParamsContext } from 'next/dist/shared/lib/hooks-client-context.shared-runtime'

GlobalRegistrator.register({ url: 'http://localhost:3000/' })

afterAll(() => GlobalRegistrator.unregister())

const { page } = await import('../support/dom-harness')
const { LOCALES } = await import('../../src/shared/i18n/locale')
const { AiReportPanel } = await import('../../src/widgets/ai/ai-report-panel')
const { RecordQueryPanel } = await import('../../src/widgets/ai/record-query-panel')
const { DanPanel } = await import('../../src/widgets/settings/dan-panel')
const { ProfilePanel } = await import('../../src/widgets/settings/profile-panel')
const { RivalSharingPanel } = await import('../../src/widgets/settings/rival-sharing-panel')
const { SettingsTabs } = await import('../../src/widgets/settings/settings-tabs')
const { cleanupScreens, installFetchRoutes, jsonResponse, renderWithQueryClient } = await import('../support/ai-screen')

type Locale = import('../../src/shared/i18n/locale').Locale
type ReportView = import('../../src/entities/ai.type').AiReportView
type MyDans = import('../../src/entities/dan.type').MyDans
type SessionUser = import('../../src/entities/auth.type').SessionUser

const ADMIN: SessionUser = { id: 'user-1', login_id: 'alice', name: 'Alice', email: 'alice@example.test', role: 'admin' }
const ROUTER = { push: () => undefined, replace: () => undefined, back: () => undefined, forward: () => undefined, refresh: () => undefined }
const ANALYZED_PLAYS = 1234

const withRouter = (ui: React.ReactNode) => (
    <AppRouterContext.Provider value={ROUTER as never}>
        <SearchParamsContext.Provider value={new URLSearchParams('')}>{ui}</SearchParamsContext.Provider>
    </AppRouterContext.Provider>
)

const TAB_NAMES = {
    ko: ['프로필', 'API 토큰', '동기화 blob', '라이벌', '단위', 'AI 키'],
    en: ['Profile', 'API tokens', 'Sync blobs', 'Rivals', 'Dan', 'AI keys'],
    ja: ['プロフィール', 'APIトークン', '同期blob', 'ライバル', '段位', 'AIキー'],
} satisfies Record<Locale, string[]>

const ADMIN_LINKS = {
    ko: { builds: '빌드 관리', aiLimits: 'AI 한도' },
    en: { builds: 'Manage builds', aiLimits: 'AI limits' },
    ja: { builds: 'ビルド管理', aiLimits: 'AIの上限' },
} satisfies Record<Locale, { builds: string; aiLimits: string }>

const QUESTION_FORM = {
    ko: { label: '질문', placeholder: '예: 지난달 ★12 에서 하드 못 깬 곡', submit: 'AI 로 조건 만들기' },
    en: { label: 'Question', placeholder: 'e.g. Songs I could not HARD-clear at ★12 last month', submit: 'Make conditions with AI' },
    ja: { label: '質問', placeholder: '例: 先月★12でHARDをクリアできなかった曲', submit: 'AIで条件を作る' },
} satisfies Record<Locale, { label: string; placeholder: string; submit: string }>

const SHARING_TITLE = {
    ko: '라이벌에게 구간 요약 허용',
    en: 'Allow rivals to see segment summaries',
    ja: 'ライバルに区間サマリーを許可',
} satisfies Record<Locale, string>

const REPORT_INTRO = {
    ko: '판별 데이터가 있는 최근 1,234판 중 7키 단일 플레이 1판으로 계산했습니다(옛 분석 버전 2판은 다시 계산되면 더해집니다). 레인은 옵션을 적용한 뒤 실제로 친 레인이고, 손은 기본 손 배정 기준입니다.',
    en: 'Calculated from 1 single-player 7-key play out of the latest 1,234 plays that have note-by-note trace data (2 plays on an old analysis version will be added once recalculated). Lanes are the lanes actually played after options were applied, and hands follow the default hand assignment.',
    ja: 'ノーツ単位のトレースデータがある直近1,234プレーのうち、7KEYSのシングルプレー1プレーから計算しました（古い分析バージョンの2プレーは再計算されると加わります）。レーンはオプションを適用したあとに実際にプレーしたレーンで、手は標準の手の割り当てが基準です。',
} satisfies Record<Locale, string>

const COURSE_LIST_LINK = { ko: '코스 목록', en: 'course list', ja: 'コース一覧' } satisfies Record<Locale, string>

const reportView = (): ReportView => ({
    plays: { analyzed: ANALYZED_PLAYS, measured: 1, stale: 2, failed: 0, from: 0, to: 1 },
    tags: [],
    lanes: [],
    density: [],
    fatigue: { key: 'fatigue', plays: 1, mean_delta: -0.05 },
    timing: { key: 'timing.bias', notes: 30, mean_ms: 1.2 },
    options: [],
    preferred: [],
    lamps: null,
    score_rows: { plays: 1, fast: 10, slow: 5 },
    notice: null,
    ai: null,
})

const emptyDans = (): MyDans => ({
    systems: [{ system: 'stella', name: 'Stella', display: { mode: 'auto', label: null, assisted: false }, shown: [], acquired: [] }],
})

const ASSISTED_DAN_LABEL = 'A-st3'

const assistedDans = (): MyDans => ({
    systems: [
        {
            system: 'stella',
            name: 'Stella',
            display: { mode: 'auto', label: null, assisted: false },
            shown: [{ system: 'stella', name: 'Stella', label: ASSISTED_DAN_LABEL, assisted: true }],
            acquired: [
                {
                    label: 'st3',
                    assisted: true,
                    rank: 3,
                    course_hash: 'hash-st3',
                    course_name: 'Stella st3',
                    course_gauge: 'LR2',
                    acquired_at: 1_780_000_000_000,
                },
            ],
        },
    ],
})

const ASSISTED_DAN_TITLE = {
    ko: '어시스트 판으로 얻은 단위(A-단위)',
    en: 'Dan earned with assist (A-Dan)',
    ja: 'アシストありで獲得した段位（A-段位）',
} satisfies Record<Locale, string>

const ASSISTED_DAN_RULE = {
    ko: /A-단위\(예: A-十段\)로 따로 기록합니다/,
    en: /recorded separately as an A-Dan \(for example A-十段\)/,
    ja: /A-段位（例: A-十段）として別に記録します/,
} satisfies Record<Locale, RegExp>

let restoreFetch: () => void = () => undefined

afterEach(async () => {
    await cleanupScreens()
    restoreFetch()
})

describe('AI·설정 위젯의 세 언어 렌더', () => {
    for (const locale of LOCALES) {
        test(`${locale}: 설정 탭 이름이 그 언어로 나온다`, async () => {
            await renderWithQueryClient(withRouter(<SettingsTabs user={ADMIN} />), locale)

            expect(page.getAllByRole('tab').map((tab) => tab.textContent)).toEqual(TAB_NAMES[locale])
        })

        test(`${locale}: 프로필 패널의 관리자 링크 이름이 그 언어로 나온다`, async () => {
            await renderWithQueryClient(withRouter(<ProfilePanel user={ADMIN} />), locale)

            expect(page.getByRole('link', { name: ADMIN_LINKS[locale].builds }).getAttribute('href')).toBe('/admin/builds')
            expect(page.getByRole('link', { name: ADMIN_LINKS[locale].aiLimits }).getAttribute('href')).toBe('/admin/ai')
        })

        test(`${locale}: 기록에 묻기 폼의 라벨·안내·버튼이 그 언어로 나온다`, async () => {
            await renderWithQueryClient(<RecordQueryPanel />, locale)

            const { label, placeholder, submit } = QUESTION_FORM[locale]
            expect(page.getByLabelText(label).getAttribute('placeholder')).toBe(placeholder)
            expect(page.getByRole('button', { name: submit })).toBeTruthy()
        })

        test(`${locale}: 라이벌 구간 공유 패널의 제목이 그 언어로 나온다`, async () => {
            const installed = installFetchRoutes({ 'GET /api/fe/me/traces/sharing': () => jsonResponse({ rival_segments: false, updated_at: null }) })
            restoreFetch = installed.restore
            await renderWithQueryClient(<RivalSharingPanel />, locale)

            expect(await page.findByRole('heading', { name: SHARING_TITLE[locale] })).toBeTruthy()
            expect(await page.findByRole('switch')).toBeTruthy()
        })

        test(`${locale}: 단위 패널의 안내 속 링크가 문장 안에서 그 언어로 나온다`, async () => {
            const installed = installFetchRoutes({ 'GET /api/fe/me/dans': () => jsonResponse(emptyDans()) })
            restoreFetch = installed.restore
            await renderWithQueryClient(<DanPanel playerId='alice' />, locale)

            expect((await page.findByRole('link', { name: COURSE_LIST_LINK[locale] })).getAttribute('href')).toBe('/courses')
        })

        test(`${locale}: A-단위는 어떤 언어에서도 같은 A- 글자로 나오고 설명과 인정 조건만 그 언어로 나온다`, async () => {
            const installed = installFetchRoutes({ 'GET /api/fe/me/dans': () => jsonResponse(assistedDans()) })
            restoreFetch = installed.restore
            await renderWithQueryClient(<DanPanel playerId='alice' />, locale)

            const badge = await page.findByText(`Stella ${ASSISTED_DAN_LABEL}`)
            expect(badge.getAttribute('title')).toBe(ASSISTED_DAN_TITLE[locale])
            expect(page.getByText(ASSISTED_DAN_RULE[locale])).toBeTruthy()
            expect(page.getByText(ASSISTED_DAN_LABEL)).toBeTruthy()
        })

        test(`${locale}: 리포트 안내 문장이 숫자 묶음과 단복수를 그 언어 규칙으로 쓴다`, async () => {
            const installed = installFetchRoutes({ 'GET /api/fe/me/ai/report': () => jsonResponse(reportView()) })
            restoreFetch = installed.restore
            await renderWithQueryClient(<AiReportPanel />, locale)

            expect(await page.findByText(REPORT_INTRO[locale])).toBeTruthy()
        })
    }
})
