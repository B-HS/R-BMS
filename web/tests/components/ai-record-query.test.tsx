import { afterAll, afterEach, describe, expect, test } from 'bun:test'
import { GlobalRegistrator } from '@happy-dom/global-registrator'

GlobalRegistrator.register()

afterAll(() => GlobalRegistrator.unregister())

const { changeValue, click, keyDown, page, waitFor, within } = await import('../support/dom-harness')
const { RecordQueryPanel } = await import('../../src/widgets/ai/record-query-panel')
const { cleanupScreens, errorResponse, installFetchRoutes, jsonResponse, renderWithQueryClient, toastTexts } = await import('../support/ai-screen')

type Answer = import('../../src/entities/ai.type').AiRecordQueryAnswer
type Row = import('../../src/entities/ai.type').AiRecordQueryRow

const ASK_PATH = '/api/fe/me/ai/record-query'
const RUN_PATH = `${ASK_PATH}/run`
const USAGE_PATH = '/api/fe/me/ai/usage'
const PLAYED_AT = Date.UTC(2026, 9, 1, 3)

const row = (overrides: Partial<Row> = {}): Row => ({
    score_id: 'score-1',
    chart_sha256: 'f'.repeat(64),
    chart_md5: 'a'.repeat(32),
    title: 'Alpha',
    artist: 'Artist A',
    table_level: '★12',
    lamp: 'Hard',
    assist_grade: 0,
    ex_score: 1800,
    ex_rate: 90,
    bp: 5,
    played_at: PLAYED_AT,
    random: 'Random',
    gauge: 'Hard',
    scratch_auto: false,
    ...overrides,
})

const askAnswer = (overrides: Partial<Answer> = {}): Answer => ({
    plan: {
        filters: [{ field: 'lamp', op: 'gte', value: 'Hard' }],
        sort_field: 'played_at',
        sort_dir: 'desc',
        limit: 20,
        explanation: '하드 이상인 판을 최근 순으로 보입니다.',
    },
    issues: [],
    result: { source: 'scores', rows: [row()] },
    ai: {
        cached: false,
        provider: 'anthropic',
        model: 'claude-test',
        usage: { inputTokens: 300, outputTokens: 50, cachedInputTokens: 0 },
        created_at: PLAYED_AT,
    },
    ...overrides,
})

let restoreFetch: () => void = () => undefined

const install = (answers: { ask: Answer; run?: (body: unknown) => Answer }) => {
    const installed = installFetchRoutes({
        [`POST ${ASK_PATH}`]: () => jsonResponse(answers.ask),
        [`POST ${RUN_PATH}`]: (call) => jsonResponse(answers.run?.(call.body) ?? answers.ask),
        [`GET ${USAGE_PATH}`]: () => jsonResponse(null),
    })
    restoreFetch = installed.restore
    return installed.calls
}

const askQuestion = async (question: string) => {
    await changeValue(page.getByLabelText('질문') as HTMLInputElement, question)
    await click(page.getByRole('button', { name: 'AI 로 조건 만들기' }))
}

afterEach(async () => {
    await cleanupScreens()
    restoreFetch()
})

describe('기록 질문 화면의 AI 실패', () => {
    const installFailure = (response: () => Response) => {
        const installed = installFetchRoutes({ [`POST ${ASK_PATH}`]: response, [`GET ${USAGE_PATH}`]: () => jsonResponse(null) })
        restoreFetch = installed.restore
    }

    test('공급자가 기다리라고 한 초를 화면에 보이고 토스트를 겹쳐 띄우지 않는다', async () => {
        installFailure(() =>
            errorResponse(429, 'AI_PROVIDER_RATE_LIMITED', 'AI 공급자가 요청을 잠시 막았습니다.', {
                reason: 'rate_limited',
                retry_after_seconds: 17,
            }),
        )
        await renderWithQueryClient(<RecordQueryPanel />)
        await askQuestion('하드 이상')

        const alert = await page.findByRole('alert')
        expect(within(alert).getByText('AI 공급자가 요청을 잠시 막았습니다.')).toBeTruthy()
        expect(within(alert).getByText(/17초 뒤에 다시 시도/)).toBeTruthy()
        expect(within(alert).queryByRole('link', { name: 'AI 설정 열기' })).toBeNull()
        expect(toastTexts()).not.toContain('AI 공급자가 요청을 잠시 막았습니다.')
    })

    test('모델이 없거나 은퇴했으면 모델을 다시 고르라고 하고 AI 설정으로 가는 링크를 준다', async () => {
        installFailure(() =>
            errorResponse(422, 'AI_MODEL_UNAVAILABLE', '선택한 AI 모델이 없거나 더 이상 제공되지 않습니다.', { reason: 'model_unavailable' }),
        )
        await renderWithQueryClient(<RecordQueryPanel />)
        await askQuestion('하드 이상')

        const alert = await page.findByRole('alert')
        expect(within(alert).getByText('설정에서 모델을 다시 고르세요.')).toBeTruthy()
        expect(within(alert).getByRole('link', { name: 'AI 설정 열기' }).getAttribute('href')).toBe('/settings?tab=ai')
    })
})

describe('기록 질문 화면', () => {
    test('질문을 보내면 해석 문장·사용량·조건 칩·결과 표를 보이고 AI 호출은 한 번이다', async () => {
        const calls = install({ ask: askAnswer() })
        await renderWithQueryClient(<RecordQueryPanel />)
        expect(page.getByRole('button', { name: 'AI 로 조건 만들기' }).hasAttribute('disabled')).toBe(true)
        await askQuestion('  하드 이상 최근 순  ')

        expect(await page.findByText('하드 이상인 판을 최근 순으로 보입니다.')).toBeTruthy()
        expect(page.getByText(/입력 300 · 출력 50 토큰/)).toBeTruthy()
        expect((page.getByLabelText('조건 1 값') as HTMLInputElement).value).toBe('Hard')
        expect(page.getByRole('link', { name: 'Alpha' }).getAttribute('href')).toBe(`/charts/${'f'.repeat(64)}`)
        expect(page.getByText('90.00%')).toBeTruthy()
        expect(page.queryByText('저장된 해석')).toBeNull()
        expect(calls.filter((call) => call.path === ASK_PATH)).toEqual([{ method: 'POST', path: ASK_PATH, body: { question: '하드 이상 최근 순' } }])
    })

    test('조건을 고쳐 다시 실행하면 AI 경로가 아니라 run 경로로 고친 조건만 보낸다', async () => {
        const calls = install({
            ask: askAnswer(),
            run: () =>
                askAnswer({
                    plan: { ...askAnswer().plan, explanation: null },
                    ai: null,
                    result: { source: 'scores', rows: [row({ lamp: 'Easy' })] },
                }),
        })
        await renderWithQueryClient(<RecordQueryPanel />)
        await askQuestion('하드 이상')
        await page.findByLabelText('조건 1 값')

        await changeValue(page.getByLabelText('조건 1 값') as HTMLInputElement, 'Easy')
        await click(page.getByRole('button', { name: '조건 추가' }))
        await changeValue(page.getByLabelText('조건 2 값') as HTMLInputElement, 'Failed')
        await click(page.getByRole('button', { name: '조건 2 지우기' }))
        await changeValue(page.getByLabelText('최대 건수') as HTMLInputElement, '7')
        await click(page.getByRole('button', { name: '이 조건으로 다시 실행' }))

        expect(await page.findByText('직접 고친 조건으로 실행했습니다(AI 호출 없음).')).toBeTruthy()
        expect(calls.filter((call) => call.path === ASK_PATH)).toHaveLength(1)
        expect(calls.filter((call) => call.path === RUN_PATH).map((call) => call.body)).toEqual([
            { filters: [{ field: 'lamp', op: 'gte', value: 'Easy' }], sort_field: 'played_at', sort_dir: 'desc', limit: 7 },
        ])
        expect((page.getByLabelText('조건 1 값') as HTMLInputElement).value).toBe('Easy')
    })

    test('검사에 실패한 조건이 있으면 결과 대신 그 칩에 이유를 보이고, 항목을 바꾸면 비교도 그 항목에 맞춘다', async () => {
        install({
            ask: askAnswer({
                plan: {
                    filters: [
                        { field: 'bp', op: 'lte', value: '5' },
                        { field: 'level', op: 'eq', value: '99' },
                    ],
                    sort_field: 'level',
                    sort_dir: 'asc',
                    limit: 20,
                    explanation: '99레벨에서 BP 5 이하',
                },
                issues: [{ index: 1, reason: 'value_invalid' }],
                result: null,
            }),
        })
        await renderWithQueryClient(<RecordQueryPanel />)
        await askQuestion('99레벨 BP 5 이하')

        expect(await page.findByText(/검사를 통과하지 못한 조건이 있어 실행하지 않았습니다/)).toBeTruthy()
        expect(within(page.getByLabelText('조건 2')).getByRole('alert').textContent).toBe('값을 읽을 수 없습니다')
        expect(within(page.getByLabelText('조건 1')).queryByRole('alert')).toBeNull()
        expect(page.queryByRole('region', { name: '질문 결과' })).toBeNull()

        await keyDown(page.getByLabelText('조건 1 항목'), 'ArrowDown')
        await click(await page.findByRole('option', { name: '곡 제목' }))
        await waitFor(() => expect(page.getByLabelText('조건 1 비교').textContent).toBe('포함'))
    })

    test('표 결과에서 안 친 곡은 링크 없이 NO PLAY 로 보인다', async () => {
        install({
            ask: askAnswer({
                result: {
                    source: 'table',
                    rows: [
                        row({
                            score_id: null,
                            chart_sha256: null,
                            title: 'Unplayed',
                            lamp: 'NoPlay',
                            ex_score: null,
                            ex_rate: null,
                            bp: null,
                            played_at: null,
                            random: null,
                            gauge: null,
                            scratch_auto: null,
                        }),
                    ],
                },
            }),
        })
        await renderWithQueryClient(<RecordQueryPanel />)
        await askQuestion('★12 안 깬 곡')

        expect(await page.findByText(/난이도표의 곡마다 가장 좋은 기록에서 찾았습니다/)).toBeTruthy()
        expect(page.getByText('Unplayed').tagName).toBe('SPAN')
        expect(page.queryByRole('link', { name: 'Unplayed' })).toBeNull()
        expect(page.getByText('NO PLAY')).toBeTruthy()
    })
})
