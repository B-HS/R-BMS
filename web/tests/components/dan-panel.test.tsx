import { afterAll, afterEach, describe, expect, test } from 'bun:test'
import { GlobalRegistrator } from '@happy-dom/global-registrator'

GlobalRegistrator.register()

afterAll(() => GlobalRegistrator.unregister())

const { click, keyDown, page, waitFor, within } = await import('../support/dom-harness')
const { toast } = await import('sonner')
const { DanPanel } = await import('../../src/widgets/settings/dan-panel')
const { cleanupScreens, errorResponse, installFetchRoutes, jsonResponse, renderWithQueryClient, toastTexts } = await import('../support/ai-screen')

type MyDans = import('../../src/entities/dan.type').MyDans
type DanSystem = import('../../src/entities/dan.type').DanSystem
type AcquiredDan = import('../../src/entities/dan.type').AcquiredDan
type DisplayedDan = import('../../src/entities/dan.type').DisplayedDan
type FetchRoutes = Parameters<typeof installFetchRoutes>[0]

const MINE_PATH = '/api/fe/me/dans'
const DISPLAY_PATH = '/api/fe/me/dans/display'
const PLAYER_ID = 'alice'
const ACQUIRED_AT = 1_780_000_000_000
const SP_NAME = 'GENOSIDE 2018 SP'
const SP_SELECT_NAME = `${SP_NAME} 표시 단위`
const SYSTEM_COUNT = 4

const acquiredDan = (label: string, rank: number, courseName: string, assisted = false): AcquiredDan => ({
    label,
    assisted,
    rank,
    course_hash: `hash-${label}`,
    course_name: courseName,
    course_gauge: 'LR2',
    acquired_at: ACQUIRED_AT,
})

const SP_ACQUIRED = [acquiredDan('初段', 1, 'GENOSIDE 2018 段位認定 初段'), acquiredDan('二段', 2, 'GENOSIDE 2018 段位認定 二段')]
const SP_ASSISTED_ACQUIRED = acquiredDan('三段', 3, 'GENOSIDE 2018 段位認定 三段', true)
const SP_WITH_ASSISTED = [...SP_ACQUIRED, SP_ASSISTED_ACQUIRED]

const shownDan = (label: string, assisted = false): DisplayedDan => ({ system: 'genoside2018_sp', name: SP_NAME, label, assisted })

const emptySystem = (system: DanSystem['system'], name: string): DanSystem => ({
    system,
    name,
    display: { mode: 'auto', label: null, assisted: false },
    shown: [],
    acquired: [],
})

const myDans = (overrides: Partial<DanSystem> = {}): MyDans => ({
    systems: [
        { ...emptySystem('genoside2018_sp', SP_NAME), ...overrides },
        emptySystem('genoside2018_dp', 'GENOSIDE 2018 DP'),
        emptySystem('stella', 'Stella'),
        emptySystem('satellite', 'Satellite'),
    ],
})

const withSpDans = () => myDans({ acquired: SP_ACQUIRED, shown: [shownDan('二段')] })
const withSpAssistedDans = () => myDans({ acquired: SP_WITH_ASSISTED, shown: [shownDan('二段'), shownDan('A-三段', true)] })

let restoreFetch: () => void = () => undefined

const useRoutes = (routes: FetchRoutes) => {
    const installed = installFetchRoutes(routes)
    restoreFetch = installed.restore
    return installed.calls
}

const openSelect = (trigger: HTMLElement) => keyDown(trigger, 'ArrowDown')

afterEach(async () => {
    await cleanupScreens()
    restoreFetch()
    toast.dismiss()
})

describe('마이페이지 단위 탭', () => {
    test('획득이 없으면 안내와 인정 조건을 보이고 네 체계 카드가 모두 있다', async () => {
        useRoutes({ [`GET ${MINE_PATH}`]: () => jsonResponse(myDans()) })
        await renderWithQueryClient(<DanPanel playerId={PLAYER_ID} />)

        expect(await page.findByText(/아직 획득한 단위가 없습니다/)).toBeTruthy()
        expect(page.getByLabelText('단위 인정 조건')).toBeTruthy()
        expect(page.getAllByRole('combobox')).toHaveLength(SYSTEM_COUNT)
        for (const name of [SP_NAME, 'GENOSIDE 2018 DP', 'Stella', 'Satellite']) {
            expect(page.getByRole('heading', { name })).toBeTruthy()
        }
    })

    test('자동이면 가장 높은 단위의 이름이 다른 플레이어에게 보이는 단위로 나온다', async () => {
        useRoutes({ [`GET ${MINE_PATH}`]: () => jsonResponse(withSpDans()) })
        await renderWithQueryClient(<DanPanel playerId={PLAYER_ID} />)

        expect(await page.findByText(`${SP_NAME} 二段`)).toBeTruthy()
        expect(page.queryByText(/아직 획득한 단위가 없습니다/)).toBeNull()
        expect(page.getByRole('link', { name: 'GENOSIDE 2018 段位認定 初段' }).getAttribute('href')).toBe(
            `/courses/${encodeURIComponent('hash-初段')}`,
        )
    })

    test('표시를 숨김으로 바꾸면 PUT 을 보내고 성공 토스트를 띄운 뒤 목록을 다시 받아 보인다', async () => {
        const toastsBefore = toastTexts().length
        let current = withSpDans()
        const calls = useRoutes({
            [`GET ${MINE_PATH}`]: () => jsonResponse(current),
            [`PUT ${DISPLAY_PATH}`]: () => {
                current = myDans({ acquired: SP_ACQUIRED, display: { mode: 'hidden', label: null, assisted: false }, shown: [] })
                return jsonResponse(current)
            },
        })
        await renderWithQueryClient(<DanPanel playerId={PLAYER_ID} />)

        await openSelect(await page.findByRole('combobox', { name: SP_SELECT_NAME }))
        await click(await page.findByRole('option', { name: '숨김' }))

        await waitFor(() => expect(calls.find((call) => call.method === 'PUT')?.body).toEqual({ system: 'genoside2018_sp', mode: 'hidden' }))
        await waitFor(() => expect(toastTexts().slice(toastsBefore)).toEqual(['표시할 단위를 바꿨습니다']))
        await waitFor(() => expect(page.queryByText(`${SP_NAME} 二段`)).toBeNull())
        expect(calls.filter((call) => call.method === 'GET' && call.path === MINE_PATH).length).toBeGreaterThanOrEqual(2)
    })

    test('획득한 단위 하나를 고르면 label 을 담아 보낸다', async () => {
        const calls = useRoutes({
            [`GET ${MINE_PATH}`]: () => jsonResponse(withSpDans()),
            [`PUT ${DISPLAY_PATH}`]: () => jsonResponse(withSpDans()),
        })
        await renderWithQueryClient(<DanPanel playerId={PLAYER_ID} />)

        await openSelect(await page.findByRole('combobox', { name: SP_SELECT_NAME }))
        expect(page.queryByRole('option', { name: '三段' })).toBeNull()
        await click(await page.findByRole('option', { name: '初段' }))

        await waitFor(() =>
            expect(calls.find((call) => call.method === 'PUT')?.body).toEqual({
                system: 'genoside2018_sp',
                mode: 'label',
                label: '初段',
                assisted: false,
            }),
        )
    })

    test('고른 단위가 있는 체계는 그 단위가 보이고 자동으로 되돌리면 auto 를 보낸다', async () => {
        const chosen = myDans({ acquired: SP_ACQUIRED, display: { mode: 'label', label: '初段', assisted: false }, shown: [shownDan('初段')] })
        const calls = useRoutes({
            [`GET ${MINE_PATH}`]: () => jsonResponse(chosen),
            [`PUT ${DISPLAY_PATH}`]: () => jsonResponse(withSpDans()),
        })
        await renderWithQueryClient(<DanPanel playerId={PLAYER_ID} />)

        expect(await page.findByText(`${SP_NAME} 初段`)).toBeTruthy()
        await openSelect(page.getByRole('combobox', { name: SP_SELECT_NAME }))
        await click(await page.findByRole('option', { name: '자동 (가장 높은 단위, A-단위가 더 높으면 함께)' }))

        await waitFor(() => expect(calls.find((call) => call.method === 'PUT')?.body).toEqual({ system: 'genoside2018_sp', mode: 'auto' }))
    })

    test('A-단위까지 가진 체계는 다른 플레이어에게 보이는 단위 줄에 정식과 A-단위를 함께 보인다', async () => {
        useRoutes({ [`GET ${MINE_PATH}`]: () => jsonResponse(withSpAssistedDans()) })
        await renderWithQueryClient(<DanPanel playerId={PLAYER_ID} />)

        const shown = await page.findByLabelText('표시 단위')
        expect([...shown.querySelectorAll('li')].map((item) => item.textContent)).toEqual([`${SP_NAME} 二段`, `${SP_NAME} A-三段`])
    })

    test('표시 단위 선택 목록은 정식과 A-단위를 따로 보이고 A-단위를 고르면 assisted 를 true 로 보낸다', async () => {
        const calls = useRoutes({
            [`GET ${MINE_PATH}`]: () => jsonResponse(withSpAssistedDans()),
            [`PUT ${DISPLAY_PATH}`]: () => jsonResponse(withSpAssistedDans()),
        })
        await renderWithQueryClient(<DanPanel playerId={PLAYER_ID} />)

        await openSelect(await page.findByRole('combobox', { name: SP_SELECT_NAME }))
        expect(page.getAllByRole('option').map((option) => option.textContent)).toEqual([
            '자동 (가장 높은 단위, A-단위가 더 높으면 함께)',
            '숨김',
            '初段',
            '二段',
            'A-三段',
        ])
        await click(await page.findByRole('option', { name: 'A-三段' }))

        await waitFor(() =>
            expect(calls.find((call) => call.method === 'PUT')?.body).toEqual({
                system: 'genoside2018_sp',
                mode: 'label',
                label: '三段',
                assisted: true,
            }),
        )
    })

    test('같은 label 의 정식과 A-단위를 둘 다 가지면 선택 목록에 따로 나오고 각각 다른 값을 보낸다', async () => {
        const bothTwo = [acquiredDan('二段', 2, 'GENOSIDE 2018 段位認定 二段'), acquiredDan('二段', 2, 'A 코스 二段', true)]
        const calls = useRoutes({
            [`GET ${MINE_PATH}`]: () => jsonResponse(myDans({ acquired: bothTwo, shown: [shownDan('二段')] })),
            [`PUT ${DISPLAY_PATH}`]: () => jsonResponse(myDans({ acquired: bothTwo })),
        })
        await renderWithQueryClient(<DanPanel playerId={PLAYER_ID} />)

        await openSelect(await page.findByRole('combobox', { name: SP_SELECT_NAME }))
        expect(page.getAllByRole('option').map((option) => option.textContent)).toEqual([
            '자동 (가장 높은 단위, A-단위가 더 높으면 함께)',
            '숨김',
            '二段',
            'A-二段',
        ])
        await click(await page.findByRole('option', { name: 'A-二段' }))

        await waitFor(() =>
            expect(calls.find((call) => call.method === 'PUT')?.body).toEqual({
                system: 'genoside2018_sp',
                mode: 'label',
                label: '二段',
                assisted: true,
            }),
        )
    })

    test('A-단위만 가진 체계에서 A-단위를 고르면 그 선택이 왕복해 선택창과 보이는 단위 줄에 A-단위로 남는다', async () => {
        const onlyAssisted = [SP_ASSISTED_ACQUIRED]
        let current = myDans({ acquired: onlyAssisted, shown: [shownDan('A-三段', true)] })
        const calls = useRoutes({
            [`GET ${MINE_PATH}`]: () => jsonResponse(current),
            [`PUT ${DISPLAY_PATH}`]: () => {
                current = myDans({
                    acquired: onlyAssisted,
                    display: { mode: 'label', label: '三段', assisted: true },
                    shown: [shownDan('A-三段', true)],
                })
                return jsonResponse(current)
            },
        })
        await renderWithQueryClient(<DanPanel playerId={PLAYER_ID} />)

        await openSelect(await page.findByRole('combobox', { name: SP_SELECT_NAME }))
        await click(await page.findByRole('option', { name: 'A-三段' }))

        await waitFor(() => expect(calls.find((call) => call.method === 'PUT')?.body).toMatchObject({ mode: 'label', label: '三段', assisted: true }))
        await waitFor(() => expect(page.getByRole('combobox', { name: SP_SELECT_NAME }).textContent).toBe('A-三段'))
        expect(page.getByText(`${SP_NAME} A-三段`)).toBeTruthy()
    })

    test('서버가 거절하면 오류 토스트를 띄우고 성공 토스트는 띄우지 않는다', async () => {
        const toastsBefore = toastTexts().length
        useRoutes({
            [`GET ${MINE_PATH}`]: () => jsonResponse(withSpDans()),
            [`PUT ${DISPLAY_PATH}`]: () => errorResponse(400, 'VALIDATION_ERROR', '입력값이 올바르지 않습니다.'),
        })
        await renderWithQueryClient(<DanPanel playerId={PLAYER_ID} />)

        await openSelect(await page.findByRole('combobox', { name: SP_SELECT_NAME }))
        await click(await page.findByRole('option', { name: '숨김' }))

        await waitFor(() => expect(toastTexts().slice(toastsBefore)).toEqual(['입력값이 올바르지 않습니다.']))
    })

    test('획득 목록은 단위·코스·게이지 모드를 보인다', async () => {
        useRoutes({ [`GET ${MINE_PATH}`]: () => jsonResponse(withSpDans()) })
        await renderWithQueryClient(<DanPanel playerId={PLAYER_ID} />)

        const caption = await page.findByText(`${SP_NAME} 획득 단위`)
        const table = caption.closest('table') as HTMLElement
        expect(within(table).getByText('二段')).toBeTruthy()
        expect(table.textContent).toContain('LR2')
        expect(table.querySelectorAll('tbody tr')).toHaveLength(SP_ACQUIRED.length)
    })

    test('획득 목록은 A-단위를 A- 글자와 함께 정식 단위 옆 행으로 보인다', async () => {
        useRoutes({ [`GET ${MINE_PATH}`]: () => jsonResponse(withSpAssistedDans()) })
        await renderWithQueryClient(<DanPanel playerId={PLAYER_ID} />)

        const caption = await page.findByText(`${SP_NAME} 획득 단위`)
        const table = caption.closest('table') as HTMLElement
        expect(within(table).getByText('A-三段')).toBeTruthy()
        expect(within(table).queryByText('三段')).toBeNull()
        expect(table.querySelectorAll('tbody tr')).toHaveLength(SP_WITH_ASSISTED.length)
    })

    test('인정 조건에 A-단위 규칙이 들어 있다', async () => {
        useRoutes({ [`GET ${MINE_PATH}`]: () => jsonResponse(myDans()) })
        await renderWithQueryClient(<DanPanel playerId={PLAYER_ID} />)

        const rules = await page.findByLabelText('단위 인정 조건')
        expect(rules.querySelectorAll('li')).toHaveLength(7)
        expect(within(rules).getByText(/A-十段/)).toBeTruthy()
    })

    test('불러오지 못하면 오류 상태를 보인다', async () => {
        useRoutes({ [`GET ${MINE_PATH}`]: () => errorResponse(500, 'INTERNAL_ERROR', '서버 오류') })
        await renderWithQueryClient(<DanPanel playerId={PLAYER_ID} />)

        expect(await page.findByText('단위를 불러오지 못했습니다')).toBeTruthy()
        expect(page.queryByLabelText('단위 인정 조건')).toBeNull()
    })
})
