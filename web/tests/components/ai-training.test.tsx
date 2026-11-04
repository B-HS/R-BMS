import { afterAll, afterEach, beforeEach, describe, expect, test } from 'bun:test'
import { GlobalRegistrator } from '@happy-dom/global-registrator'
import { AppRouterContext } from 'next/dist/shared/lib/app-router-context.shared-runtime'
import { SearchParamsContext } from 'next/dist/shared/lib/hooks-client-context.shared-runtime'
import { toast } from 'sonner'

GlobalRegistrator.register({ url: 'http://localhost:3000/ai' })

afterAll(() => GlobalRegistrator.unregister())

const { click, keyDown, page, waitFor, within } = await import('../support/dom-harness')
const { AiTrainingPanel } = await import('../../src/widgets/ai/ai-training-panel')
const { AiTrainingDetailPanel } = await import('../../src/widgets/ai/ai-training-detail-panel')
const { AiHubTabs } = await import('../../src/widgets/ai/ai-hub-tabs')
const { cleanupScreens, errorResponse, installFetchRoutes, jsonResponse, renderWithQueryClient, toastTexts } = await import('../support/ai-screen')
const fixture = await import('./ai-training-fixture')
const { trainingChartCreateSchema, trainingChartDetailSchema, trainingChartListSchema, trainingRemovalSchema } =
    await import('../../src/server/dto/training')
const { weaknessProfileViewSchema } = await import('../../src/server/dto/ai-play')

type TrainingList = import('../../src/entities/ai.type').AiTrainingChartList
type Profile = import('../../src/entities/ai.type').AiWeaknessProfile

const LIST_PATH = '/api/fe/me/ai/training-charts'
const PROFILE_PATH = '/api/fe/me/ai/profile'
const FIRST_TITLE = fixture.trainingChart(1).title
const LIMIT = 20

const routerReplaces: string[] = []
const ROUTER = {
    push: () => undefined,
    replace: (href: string) => void routerReplaces.push(href),
    back: () => undefined,
    forward: () => undefined,
    refresh: () => undefined,
    prefetch: () => undefined,
}

let restoreFetch: () => void = () => undefined

beforeEach(() => {
    toast.dismiss()
    routerReplaces.length = 0
})

afterEach(async () => {
    await cleanupScreens()
    restoreFetch()
    toast.dismiss()
})

const withRouter = (search: string, ui: React.ReactNode) => (
    <AppRouterContext.Provider value={ROUTER as never}>
        <SearchParamsContext.Provider value={new URLSearchParams(search)}>{ui}</SearchParamsContext.Provider>
    </AppRouterContext.Provider>
)

const element = (selector: string) => {
    const found = document.querySelector(selector)
    if (!(found instanceof HTMLElement)) throw new Error(`missing ${selector}`)
    return found
}

const svgElement = (selector: string) => {
    const found = document.querySelector(selector)
    if (!(found instanceof SVGElement)) throw new Error(`missing ${selector}`)
    return found
}

const useRoutes = (routes: Parameters<typeof installFetchRoutes>[0]) => {
    const installed = installFetchRoutes(routes)
    restoreFetch = installed.restore
    return installed.calls
}

const makeRoutes = (list: () => TrainingList, profile: () => Profile = () => fixture.trainingProfile()) => ({
    [`GET ${LIST_PATH}`]: () => jsonResponse(list()),
    [`GET ${PROFILE_PATH}`]: () => jsonResponse(profile()),
})

const pickFocus = async (name: string) => {
    await keyDown(await page.findByLabelText('훈련할 약점'), 'ArrowDown')
    await click(await page.findByRole('option', { name }))
}

describe('시험 예시가 서버의 응답 스키마를 통과한다', () => {
    test('목록·상세·삭제 개수·프로필', () => {
        expect(trainingChartListSchema.safeParse(fixture.trainingList()).success).toBe(true)
        expect(trainingChartDetailSchema.safeParse(fixture.trainingDetail()).success).toBe(true)
        expect(weaknessProfileViewSchema.safeParse(fixture.trainingProfile()).success).toBe(true)
        expect(
            trainingRemovalSchema.safeParse({
                removes: { training_chart: 1, folder_items: 2 },
                keeps: { chart_pattern: 1, scores: 3, traces: 2, replays: 1 },
            }).success,
        ).toBe(true)
    })
})

describe('훈련곡 탭 - 만들기 폼', () => {
    test('AI 키가 없으면 만들기 폼 대신 키 등록을 안내하고 프로필도 부르지 않지만 목록은 그대로 보인다', async () => {
        const calls = useRoutes(makeRoutes(() => fixture.trainingList({ needs_key: true })))
        await renderWithQueryClient(<AiTrainingPanel />)

        expect(await page.findByText('등록된 AI 키가 없습니다')).toBeTruthy()
        expect(page.getByRole('link', { name: 'AI 키 등록하기' }).getAttribute('href')).toBe('/settings?tab=ai')
        expect(page.queryByRole('button', { name: '만들기' })).toBeNull()
        expect(calls.some((call) => call.path === PROFILE_PATH)).toBe(false)
        expect(element('ul[aria-label="내 훈련곡 목록"]').querySelectorAll(':scope > li')).toHaveLength(2)
    })

    test('20곡이 차면 만들기 대신 먼저 지우라는 안내를 보이고 목록은 그대로 둔다', async () => {
        const charts = Array.from({ length: LIMIT }, (_, index) => fixture.trainingChart(index + 1))
        useRoutes(makeRoutes(() => fixture.trainingList({ charts })))
        await renderWithQueryClient(<AiTrainingPanel />)

        expect(await page.findByText('훈련곡 보관함이 가득 찼습니다')).toBeTruthy()
        expect(page.getByText(/새로 만들려면 아래 목록에서 쓰지 않는 훈련곡을 먼저 지우세요/)).toBeTruthy()
        expect(page.getByText('보관 중인 훈련곡 20곡 / 최대 20곡')).toBeTruthy()
        expect(page.queryByRole('button', { name: '만들기' })).toBeNull()
        expect(element('ul[aria-label="내 훈련곡 목록"]').querySelectorAll(':scope > li')).toHaveLength(LIMIT)
    })

    test('목록 위에 구동기에서 받는 방법(내 폴더 훈련곡 폴더, 흐린 줄, GET CHART)을 적는다', async () => {
        useRoutes(makeRoutes(() => fixture.trainingList()))
        await renderWithQueryClient(<AiTrainingPanel />)

        await waitFor(() => element('ul[aria-label="내 훈련곡 목록"]'))
        const guide = await page.findByText(
            /받는 방법: 구동기 선곡 화면의 MY FOLDERS\(내 폴더\)에서 "훈련곡" 폴더 → 흐린 줄 → GET CHART\(차트 받기\)/,
        )
        const list = element('ul[aria-label="내 훈련곡 목록"]')
        expect(guide.compareDocumentPosition(list) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy()
    })

    test('초점은 자동과 훈련곡을 만들 수 있는 약점만 고르고, 강도를 바꿔 만들면 서버 입력 꼴로 보내 상세 링크를 보인다', async () => {
        const charts = [fixture.trainingChart(1), fixture.trainingChart(2)]
        const calls = useRoutes({
            ...makeRoutes(() => fixture.trainingList({ charts })),
            [`POST ${LIST_PATH}`]: () => {
                charts.unshift(fixture.trainingChart(3))
                const made = fixture.trainingChart(3)
                return jsonResponse(
                    fixture.trainingDetail({ id: made.id, sha256: made.sha256, title: made.title, download_url: made.download_url }),
                    201,
                )
            },
        })
        await renderWithQueryClient(<AiTrainingPanel />)

        expect((await page.findByLabelText('훈련할 약점')).textContent).toBe('자동(서버가 약점을 고름)')
        await keyDown(page.getByLabelText('훈련할 약점'), 'ArrowDown')
        const options = (await page.findByRole('option', { name: '트릴' })).parentElement
        expect([...(options?.querySelectorAll('[role="option"]') ?? [])].map((option) => option.textContent)).toEqual([
            '자동(서버가 약점을 고름)',
            '트릴',
            '1번 레인',
            '연타',
        ])
        await click(page.getByRole('option', { name: '트릴' }))
        expect(page.getByText(/EX 한계 밀도의 100~110%까지 밀어붙이는 기본 단계입니다/)).toBeTruthy()

        await click(page.getByRole('radio', { name: 'OVER' }))
        expect(page.getByText('EX 한계 밀도의 115~130%로 한계를 넘어서는 단계입니다.')).toBeTruthy()
        await click(page.getByRole('button', { name: '만들기' }))

        const post = await waitFor(() => {
            const found = calls.find((call) => call.method === 'POST')
            if (!found) throw new Error('no POST yet')
            return found
        })
        expect(post.body).toEqual({ focus_key: 'tag:trill', intensity: 'over' })
        expect(trainingChartCreateSchema.safeParse(post.body).success).toBe(true)
        const link = await page.findByRole('link', { name: '상세 보기' })
        expect(link.getAttribute('href')).toBe(`/ai/training/${fixture.trainingId(3)}`)
        await waitFor(() => expect(element('ul[aria-label="내 훈련곡 목록"]').querySelectorAll(':scope > li')).toHaveLength(3))
        expect(toastTexts()).toContain(`훈련곡을 만들었습니다: ${fixture.trainingChart(3).title}`)
    })

    test('레인·손 초점이면 RANDOM 계열 옵션을 끄라는 안내를 보이고, 링크의 약점이 프로필에 없으면 자동으로 두고 알린다', async () => {
        useRoutes(makeRoutes(() => fixture.trainingList()))
        await renderWithQueryClient(<AiTrainingPanel initialFocusKey='lane:1' />)
        expect((await page.findByLabelText('훈련할 약점')).textContent).toBe('1번 레인')
        expect(page.getByText(/RANDOM 계열 옵션을 끄고 치세요/)).toBeTruthy()

        await pickFocus('트릴')
        expect(page.queryByText(/RANDOM 계열 옵션을 끄고 치세요/)).toBeNull()
        await cleanupScreens()

        await renderWithQueryClient(<AiTrainingPanel initialFocusKey='tag:stair' />)
        expect((await page.findByLabelText('훈련할 약점')).textContent).toBe('자동(서버가 약점을 고름)')
        expect(page.getByText(/링크로 연 약점이 지금 프로필에 없거나/)).toBeTruthy()
    })

    test('표본이 너무 작은 약점만 있으면 자동은 없고 그 약점만 고를 수 있다', async () => {
        const profile = fixture.trainingProfile({
            weaknesses: [fixture.trainingWeakness({ key: 'tag:jack', kind: 'tag', sample_level: 'insufficient', training_kinds: ['jack'] })],
        })
        useRoutes(
            makeRoutes(
                () => fixture.trainingList(),
                () => profile,
            ),
        )
        await renderWithQueryClient(<AiTrainingPanel />)
        expect((await page.findByLabelText('훈련할 약점')).textContent).toBe('연타')
        await keyDown(page.getByLabelText('훈련할 약점'), 'ArrowDown')
        expect((await page.findByRole('option', { name: '연타' })).parentElement?.querySelectorAll('[role="option"]')).toHaveLength(1)
    })

    test('훈련곡을 만들 수 있는 약점이 없으면 이유만 보이고 만들기 단추를 두지 않는다', async () => {
        useRoutes(
            makeRoutes(
                () => fixture.trainingList(),
                () => fixture.trainingProfile({ weaknesses: [], notice: { need: 'traced_plays' } }),
            ),
        )
        await renderWithQueryClient(<AiTrainingPanel />)
        expect(await page.findByText(/판별 데이터\(새 클라이언트로 친 판의 노트 단위 기록/)).toBeTruthy()
        expect(page.queryByRole('button', { name: '만들기' })).toBeNull()
        expect(page.queryByLabelText('훈련할 약점')).toBeNull()
    })

    test('서버가 거절하는 이유마다 그 자리에 문구를 보이고 오류 토스트는 띄우지 않는다', async () => {
        let failure: Response = errorResponse(409, 'TRAINING_CHART_LIMIT', '한도', { limit: LIMIT, current: LIMIT })
        const toastsBefore = toastTexts().length
        const calls = useRoutes({ ...makeRoutes(() => fixture.trainingList()), [`POST ${LIST_PATH}`]: () => failure })
        await renderWithQueryClient(<AiTrainingPanel />)
        await page.findByLabelText('훈련할 약점')
        const submit = () => click(page.getByRole('button', { name: '만들기' }))
        const postCount = () => calls.filter((call) => call.method === 'POST').length

        await submit()
        await waitFor(() => expect(page.getByText(/다른 곳에서 먼저 훈련곡 보관 한도가 찼습니다/)).toBeTruthy())
        expect(page.getByText(/AI 호출은 이미 쓰였을 수 있습니다/)).toBeTruthy()
        await waitFor(() => expect(calls.filter((call) => call.method === 'GET' && call.path === LIST_PATH).length).toBeGreaterThan(1))

        failure = errorResponse(409, 'TRAINING_FOLDER_UNAVAILABLE', '폴더', { limit: 50 })
        await submit()
        await waitFor(() => expect(postCount()).toBe(2))
        expect(await page.findByText('훈련곡 폴더를 만들 수 없습니다. 폴더를 하나 지운 뒤 다시 시도해 주세요.')).toBeTruthy()

        failure = errorResponse(422, 'AI_NOT_ENOUGH_DATA', '모자람', { reason: 'not_enough_data', need: 'weakness_profile' })
        await submit()
        await waitFor(() => expect(postCount()).toBe(3))
        expect(await page.findByText(/약점 프로필에 이 약점이 없습니다/)).toBeTruthy()

        failure = errorResponse(404, 'AI_CREDENTIAL_NOT_FOUND', '등록된 AI 키가 없습니다.')
        await submit()
        await waitFor(() => expect(postCount()).toBe(4))
        expect(await page.findByText('AI 호출이 실패했습니다')).toBeTruthy()
        expect(page.getByRole('link', { name: 'AI 설정 열기' })).toBeTruthy()
        expect(toastTexts().slice(toastsBefore)).toEqual([])
    })
})

describe('훈련곡 탭 - 목록과 삭제', () => {
    const removalCounts = { removes: { training_chart: 1, folder_items: 2 }, keeps: { chart_pattern: 1, scores: 3, traces: 2, replays: 1 } }

    test('제목·초점·판 수·최고 EX·받기 주소 복사를 한 줄에 보이고 제목은 상세로 잇는다', async () => {
        let copied = ''
        Object.defineProperty(navigator, 'clipboard', { configurable: true, value: { writeText: async (text: string) => void (copied = text) } })
        useRoutes(makeRoutes(() => fixture.trainingList()))
        await renderWithQueryClient(<AiTrainingPanel />)

        const first = (await page.findByRole('link', { name: FIRST_TITLE })).closest('li')
        if (!first) throw new Error('missing row')
        expect(within(first).getByRole('link', { name: FIRST_TITLE }).getAttribute('href')).toBe(`/ai/training/${fixture.trainingId(1)}`)
        expect(first.textContent).toContain('트릴')
        expect(first.textContent).toContain('3판 플레이')
        expect(first.textContent).toContain('최고 EX 87.7%')
        expect(first.textContent).toContain('BPM 160 · 55초 · 노트 600개')

        const second = element('ul[aria-label="내 훈련곡 목록"]').querySelectorAll(':scope > li')[1]
        expect(second.textContent).toContain('연타, 1번 레인')
        expect(second.textContent).toContain('최고 EX 없음')

        await click(within(first).getByRole('button', { name: `${FIRST_TITLE} 받기 주소 복사` }))
        await waitFor(() => expect(copied).toBe(fixture.trainingDownloadUrl(1)))
    })

    test('지우기를 누르면 개수를 센 뒤에야 확인할 수 있고, 확인한 뒤에만 DELETE 를 보낸 다음 목록에서 사라진다', async () => {
        const charts = [fixture.trainingChart(1), fixture.trainingChart(2)]
        let release: () => void = () => undefined
        const gate = new Promise<void>((resolve) => void (release = resolve))
        const calls = useRoutes({
            ...makeRoutes(() => fixture.trainingList({ charts })),
            [`GET ${LIST_PATH}/${fixture.trainingId(1)}/removal`]: async () => {
                await gate
                return jsonResponse(removalCounts)
            },
            [`DELETE ${LIST_PATH}/${fixture.trainingId(1)}`]: () => {
                charts.shift()
                return jsonResponse({ ...removalCounts, removed: true })
            },
        })
        await renderWithQueryClient(<AiTrainingPanel />)

        await click(await page.findByRole('button', { name: `${FIRST_TITLE} 지우기` }))
        const dialog = await page.findByRole('alertdialog')
        expect(within(dialog).getByText('지워지는 것과 남는 것을 세는 중입니다')).toBeTruthy()
        expect((within(dialog).getByRole('button', { name: '지우기' }) as HTMLButtonElement).disabled).toBe(true)
        expect(calls.some((call) => call.method === 'DELETE')).toBe(false)

        release()
        expect(await within(dialog).findByText('지워지는 것: 훈련곡 1곡, 내 폴더의 폴더 항목 2개')).toBeTruthy()
        expect(within(dialog).getByText('남는 것: 이 훈련곡의 판 기록 3판, 판별 데이터 2개, 리플레이 1개, 곡 패턴 분석 1개')).toBeTruthy()
        expect(within(dialog).getByText(/이미 구동기로 받은 파일도 그대로 남습니다/)).toBeTruthy()
        expect((within(dialog).getByRole('button', { name: '지우기' }) as HTMLButtonElement).disabled).toBe(false)
        expect(calls.some((call) => call.method === 'DELETE')).toBe(false)

        await click(within(dialog).getByRole('button', { name: '지우기' }))
        await waitFor(() => expect(element('ul[aria-label="내 훈련곡 목록"]').querySelectorAll(':scope > li')).toHaveLength(1))
        expect(calls.filter((call) => call.method === 'DELETE')).toHaveLength(1)
        expect(toastTexts()).toContain('훈련곡을 지웠습니다')
    })

    test('취소하면 DELETE 를 보내지 않는다', async () => {
        const calls = useRoutes({
            ...makeRoutes(() => fixture.trainingList()),
            [`GET ${LIST_PATH}/${fixture.trainingId(1)}/removal`]: () => jsonResponse(removalCounts),
        })
        await renderWithQueryClient(<AiTrainingPanel />)
        await click(await page.findByRole('button', { name: `${FIRST_TITLE} 지우기` }))
        const dialog = await page.findByRole('alertdialog')
        await within(dialog).findByText('지워지는 것: 훈련곡 1곡, 내 폴더의 폴더 항목 2개')
        await click(within(dialog).getByRole('button', { name: '취소' }))
        expect(calls.some((call) => call.method === 'DELETE')).toBe(false)
    })
})

describe('훈련곡 상세', () => {
    const detailPath = `${LIST_PATH}/${fixture.trainingId(1)}`

    const renderDetail = async () => {
        await renderWithQueryClient(
            <AppRouterContext.Provider value={ROUTER as never}>
                <AiTrainingDetailPanel chartId={fixture.trainingId(1)} />
            </AppRouterContext.Provider>,
        )
    }

    test('구간 띠·구간 목록·보정·전후 비교·판 목록을 보이고 레인 초점이 있으면 옵션 안내를 보인다', async () => {
        useRoutes({ [`GET ${detailPath}`]: () => jsonResponse(fixture.trainingDetail()) })
        await renderDetail()

        expect(await page.findByRole('heading', { name: FIRST_TITLE })).toBeTruthy()
        expect(page.getByText(/RANDOM 계열 옵션을 끄고 치세요/)).toBeTruthy()
        expect(page.getByText('PLAYLEVEL 은 난이도표 레벨이 아니라 곡 본문의 2초 밀도(NPS) 상위 10% 지점을 정수로 적은 값입니다.')).toBeTruthy()
        expect(element('dl').textContent).toContain('BPM 160 · 55초 · 노트 600개')

        const strip = element('ol[aria-hidden="true"]')
        expect([...strip.children].map((item) => (item as HTMLElement).style.flexGrow)).toEqual(['12000', '12000', '12000', '12000'])

        const sections = element('ul[aria-label="구간 목록"]').querySelectorAll(':scope > li')
        expect(sections).toHaveLength(4)
        expect(sections[0].textContent).toContain('워밍업')
        expect(sections[1].textContent).toContain('트릴')
        expect(sections[2].textContent).toContain('1번 레인')
        expect(sections[2].textContent).toContain('왼손 쪽')
        expect(sections[1].textContent).toContain('목표 9.0 / 실제 9.4 NPS · 7.2 행/초')
        expect(sections[0].textContent).not.toContain('행/초')
        expect(sections[3].textContent).toContain('쿨다운')

        const corrections = [...element('ul[aria-label="서버가 바꾼 값 목록"]').querySelectorAll(':scope > li')].map((item) => item.textContent)
        expect(corrections).toEqual(['구간 2 패턴 종류: 연타 → 트릴', '구간 5 약점: unknown → 버림', '전체 기준 BPM: 151 → 152'])

        const comparison = element('ul[aria-label="약점별 전후 비교"]').querySelectorAll(':scope > li')
        expect(comparison[0].textContent).toContain('-8.0%p')
        expect(comparison[0].textContent).toContain('-4.0%p')
        expect(comparison[0].textContent).toContain('차이 +4.0%p')
        expect(comparison[0].textContent).toContain('불확실')
        expect(comparison[1].textContent).toContain('표본 조건을 채우는 판이 아직 모자랍니다')
        expect(comparison[1].textContent).not.toContain('차이')

        expect(page.getByText('최근 4판을 보입니다(전체 4판, 최대 50판까지 표시)')).toBeTruthy()
        const plays = element('ul[aria-label="훈련곡 판 목록"]').querySelectorAll(':scope > li')
        expect(plays).toHaveLength(4)
        expect(plays[0].textContent).toContain('MIRROR')
        expect(plays[3].textContent).toContain('구간 결과 없음')
        expect(
            within(plays[0] as HTMLElement)
                .getByRole('link', { name: '판 코치' })
                .getAttribute('href'),
        ).toBe('/ai/plays/score-newest')
    })

    test('레인·손 초점 구간의 추세에서는 RANDOM 계열 옵션으로 친 판을 흐린 점으로 그리고 선에서 뺀다', async () => {
        useRoutes({ [`GET ${detailPath}`]: () => jsonResponse(fixture.trainingDetail()) })
        await renderDetail()
        await page.findByRole('heading', { name: FIRST_TITLE })

        const pattern = svgElement('svg[aria-label^="구간 2 EX 추이"]')
        expect(pattern.getAttribute('aria-label')).toBe('구간 2 EX 추이. 처음 80.0% → 마지막 90.0% (3판)')
        expect(pattern.querySelectorAll('circle[data-dimmed="true"]')).toHaveLength(0)
        expect(pattern.querySelector('polyline')?.getAttribute('points')?.split(' ')).toHaveLength(3)

        const lane = svgElement('svg[aria-label^="구간 3 EX 추이"]')
        expect(lane.getAttribute('aria-label')).toBe('구간 3 EX 추이. 처음 75.0% → 마지막 80.0% (2판)')
        expect(lane.querySelectorAll('circle')).toHaveLength(3)
        expect(lane.querySelectorAll('circle[data-dimmed="true"]')).toHaveLength(1)
        expect(lane.querySelector('polyline')?.getAttribute('points')?.split(' ')).toHaveLength(2)

        expect(element('ul[aria-label="구간별 EX 추이"]').textContent).toContain('이 구간의 결과가 아직 없습니다.')
    })

    test('레인·손 초점이 없으면 옵션 안내를 보이지 않고, 판별 데이터가 있는 판이 없으면 추이를 그리지 않는다', async () => {
        const detail = fixture.trainingDetail({
            focus_keys: ['tag:trill'],
            sections: fixture.trainingDetail().sections.filter((section) => section.focus_key !== 'lane:1'),
            plays: [fixture.trainingPlay('score-only', { sections: null })],
        })
        useRoutes({ [`GET ${detailPath}`]: () => jsonResponse(detail) })
        await renderDetail()
        await page.findByRole('heading', { name: FIRST_TITLE })
        expect(page.queryByText(/RANDOM 계열 옵션을 끄고 치세요/)).toBeNull()
        expect(page.getByText('판별 데이터가 있는 판이 아직 없어 구간 결과를 그릴 수 없습니다.')).toBeTruthy()
        expect(document.querySelector('svg[aria-label*="EX 추이"]')).toBeNull()
    })

    test('없는 훈련곡은 찾을 수 없다는 안내와 목록으로 가는 링크를 보인다', async () => {
        useRoutes({ [`GET ${detailPath}`]: () => errorResponse(404, 'TRAINING_CHART_NOT_FOUND', '훈련곡을 찾을 수 없습니다.') })
        await renderDetail()
        expect(await page.findByText('훈련곡을 찾을 수 없습니다')).toBeTruthy()
        expect(page.getByRole('link', { name: '훈련곡 목록' }).getAttribute('href')).toBe('/ai?tab=training')
    })

    test('상세에서 지우면 개수를 확인한 뒤 DELETE 를 보내고 훈련곡 탭으로 돌아간다', async () => {
        const calls = useRoutes({
            [`GET ${detailPath}`]: () => jsonResponse(fixture.trainingDetail()),
            [`GET ${detailPath}/removal`]: () =>
                jsonResponse({ removes: { training_chart: 1, folder_items: 1 }, keeps: { chart_pattern: 0, scores: 4, traces: 3, replays: 0 } }),
            [`DELETE ${detailPath}`]: () =>
                jsonResponse({
                    removes: { training_chart: 1, folder_items: 1 },
                    keeps: { chart_pattern: 0, scores: 4, traces: 3, replays: 0 },
                    removed: true,
                }),
            [`GET ${LIST_PATH}`]: () => jsonResponse(fixture.trainingList()),
        })
        await renderDetail()
        await click(await page.findByRole('button', { name: '지우기' }))
        const dialog = await page.findByRole('alertdialog')
        expect(await within(dialog).findByText('지워지는 것: 훈련곡 1곡, 내 폴더의 폴더 항목 1개')).toBeTruthy()
        expect(calls.some((call) => call.method === 'DELETE')).toBe(false)

        await click(within(dialog).getByRole('button', { name: '지우기' }))
        await waitFor(() => expect(routerReplaces).toEqual(['/ai?tab=training']))
        expect(calls.filter((call) => call.method === 'DELETE')).toHaveLength(1)
    })
})

describe('AI 허브의 훈련곡 탭', () => {
    test('훈련곡 탭이 있고 focus 가 있는 주소는 그 약점을 고른 채 훈련곡 탭을 연다', async () => {
        useRoutes(makeRoutes(() => fixture.trainingList()))
        await renderWithQueryClient(withRouter('tab=training&focus=lane:1', <AiHubTabs />))

        expect(page.getByRole('tab', { name: '훈련곡' }).getAttribute('aria-selected')).toBe('true')
        expect((await page.findByLabelText('훈련할 약점')).textContent).toBe('1번 레인')
    })

    test('다른 탭에서는 훈련곡 목록과 프로필을 요청하지 않는다', async () => {
        const calls = useRoutes(makeRoutes(() => fixture.trainingList()))
        await renderWithQueryClient(withRouter('tab=weekly', <AiHubTabs />))
        expect(calls.some((call) => call.path === LIST_PATH)).toBe(false)
        expect(page.getByRole('tab', { name: '훈련곡' }).getAttribute('aria-selected')).toBe('false')
    })
})
