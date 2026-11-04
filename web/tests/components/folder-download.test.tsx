import { afterAll, afterEach, describe, expect, test } from 'bun:test'
import { QueryClientProvider } from '@tanstack/react-query'
import { GlobalRegistrator } from '@happy-dom/global-registrator'
import { AppRouterContext } from 'next/dist/shared/lib/app-router-context.shared-runtime'

GlobalRegistrator.register({ url: 'http://localhost:3000/folders' })

afterAll(() => GlobalRegistrator.unregister())

const { mount, page, unmountAll, waitFor, within } = await import('../support/dom-harness')
const { cleanupScreens, installFetchRoutes, jsonResponse, renderWithQueryClient } = await import('../support/ai-screen')
const { createQueryClient } = await import('../../src/shared/utils/get-query-client')
const { FolderChartRow } = await import('../../src/features/folder/folder-chart-row')
const { FolderEditor } = await import('../../src/widgets/folder/folder-editor')
const { SidebarProvider } = await import('../../src/shared/ui/sidebar')
const { ShellSidebarSlotContext } = await import('../../src/shared/providers/shell-sidebar-slot-context')
const { QUERY_KEY } = await import('../../src/shared/constants/query-key')
const { folderDetailSchema } = await import('../../src/server/dto/folder')

type FolderDetail = import('../../src/entities/folder.type').FolderDetail
type FolderItem = import('../../src/entities/folder.type').FolderItem

const FOLDER_ID = `uf_${'a'.repeat(30)}`
const FOLDER_PATH = `/api/fe/me/folders/${FOLDER_ID}`
const DOWNLOAD_URL = `http://localhost:3000/api/training/tc_${'1'.repeat(32)}/chart.zip`
const UPDATED_AT = 1_780_000_000_000
const ROUTER = {
    push: () => undefined,
    replace: () => undefined,
    back: () => undefined,
    forward: () => undefined,
    refresh: () => undefined,
    prefetch: () => undefined,
}

const item = (md5Digit: string, title: string, overrides: Partial<FolderItem> = {}): FolderItem => ({
    md5: md5Digit.repeat(32),
    sha256: md5Digit.repeat(64),
    title,
    artist: 'R-BMS',
    note: '',
    ...overrides,
})

const folder = (withLinks: boolean): FolderDetail => ({
    id: FOLDER_ID,
    name: '훈련곡',
    description: '',
    is_public: false,
    position: 0,
    item_count: 2,
    updated_at: UPDATED_AT,
    items: [item('1', 'Training chart', withLinks ? { download_url: DOWNLOAD_URL } : {}), item('2', 'Plain chart')],
})

let restoreFetch: () => void = () => undefined

afterEach(async () => {
    await cleanupScreens()
    await unmountAll()
    restoreFetch()
})

describe('폴더 항목의 받기 링크', () => {
    test('서버가 주는 폴더 응답 예시가 서버 스키마를 통과한다', () => {
        expect(folderDetailSchema.safeParse(folder(true)).success).toBe(true)
        expect(folderDetailSchema.safeParse(folder(false)).success).toBe(true)
    })

    test('download_url 이 있는 항목에만 받기 링크를 달고, 링크는 내려받기 속성을 가져 폴더 이탈 경고에 걸리지 않는다', async () => {
        const noop = () => undefined
        const rowProps = { position: 1, isNew: false, isFirst: true, isLast: true, onMove: noop, onEditNote: noop, onRemove: noop }
        await renderWithQueryClient(
            <ol>
                <FolderChartRow item={item('1', 'Training chart', { download_url: DOWNLOAD_URL })} {...rowProps} />
                <FolderChartRow item={item('2', 'Plain chart')} {...rowProps} />
            </ol>,
        )
        const link = page.getByRole('link', { name: 'Training chart 받기' })
        expect(link.getAttribute('href')).toBe(DOWNLOAD_URL)
        expect(link.hasAttribute('download')).toBe(true)
        expect(page.queryByRole('link', { name: 'Plain chart 받기' })).toBeNull()
        expect(page.getAllByRole('link')).toHaveLength(1)
    })

    test('서버가 첫 폴더를 링크 없이 그려도 편집 화면이 열릴 때 폴더 경로를 다시 불러 받기 링크를 단다', async () => {
        const installed = installFetchRoutes({ [`GET ${FOLDER_PATH}`]: () => jsonResponse(folder(true)) })
        restoreFetch = installed.restore
        const queryClient = createQueryClient()
        queryClient.setQueryData(QUERY_KEY.FOLDER.DETAIL(FOLDER_ID), folder(false))
        await mount(
            <QueryClientProvider client={queryClient}>
                <AppRouterContext.Provider value={ROUTER as never}>
                    <SidebarProvider>
                        <ShellSidebarSlotContext.Provider value={{ element: null }}>
                            <FolderEditor folderId={FOLDER_ID} tables={[]} />
                        </ShellSidebarSlotContext.Provider>
                    </SidebarProvider>
                </AppRouterContext.Provider>
            </QueryClientProvider>,
        )

        const list = await waitFor(() => {
            const found = document.querySelector('ol[aria-label="훈련곡의 곡"]')
            if (!(found instanceof HTMLElement)) throw new Error('the folder list is not drawn yet')
            return found
        })
        expect(within(list).queryByRole('link', { name: 'Training chart 받기' })).toBeNull()
        await waitFor(() => expect(within(list).getByRole('link', { name: 'Training chart 받기' }).getAttribute('href')).toBe(DOWNLOAD_URL))
        expect(within(list).queryByRole('link', { name: 'Plain chart 받기' })).toBeNull()
        expect(installed.calls.filter((call) => call.method === 'GET' && call.path === FOLDER_PATH)).toHaveLength(1)
        queryClient.clear()
    })
})
