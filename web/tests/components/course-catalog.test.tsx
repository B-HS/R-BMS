import { afterAll, afterEach, describe, expect, test } from 'bun:test'
import { GlobalRegistrator } from '@happy-dom/global-registrator'

GlobalRegistrator.register()

afterAll(() => GlobalRegistrator.unregister())

const { page, within } = await import('../support/dom-harness')
const { CourseCatalogPanel } = await import('../../src/widgets/course/course-catalog-panel')
const { cleanupScreens, errorResponse, installFetchRoutes, jsonResponse, renderWithQueryClient } = await import('../support/ai-screen')
const fixture = await import('../support/course-catalog-fixture')
const { createCourseCatalogService } = await import('../../src/server/service/domain/course-catalog/course-catalog.service')
const { createMemoryCourseCatalogDb } = await import('../support/course-catalog-memory-db')
const { DAN_SETTINGS_HREF } = await import('../../src/shared/constants/navigation')

const CATALOG_PATH = '/api/fe/courses'

let restoreFetch: () => void = () => undefined

const useCatalog = (respond: () => Response) => {
    const installed = installFetchRoutes({ [`GET ${CATALOG_PATH}`]: respond })
    restoreFetch = installed.restore
}

afterEach(async () => {
    await cleanupScreens()
    restoreFetch()
})

describe('코스 목록 패널', () => {
    test('묶음마다 제목과 코스 수, 키 수, 출처 블록을 보인다', async () => {
        const catalog = await (await fixture.createCatalogFixture()).listGroups()
        useCatalog(() => jsonResponse(catalog))
        await renderWithQueryClient(<CourseCatalogPanel />)

        expect(await page.findByRole('heading', { name: 'GENOSIDE 2018 段位認定 (1)' })).toBeTruthy()
        expect(page.getByRole('heading', { name: 'GENOSIDE 2018 DP段位認定 (1)' })).toBeTruthy()
        expect(page.getByRole('heading', { name: 'Stella (2)' })).toBeTruthy()
        expect(page.getByText('14키')).toBeTruthy()
        expect(page.getAllByRole('link', { name: '공식 페이지(외부 사이트, 새 탭)' })).toHaveLength(2)
        expect(page.getByRole('link', { name: '원본 표(외부 사이트, 새 탭)' }).getAttribute('href')).toBe(fixture.STELLA_PAGE_URL)
        expect(page.getByText(/DP 값의 원천이 없습니다/)).toBeTruthy()
    })

    test('코스 줄은 코스 페이지로 가는 링크와 단위 배지를 갖고 단위를 주지 않는 코스는 표시가 없다', async () => {
        const catalog = await (await fixture.createCatalogFixture()).listGroups()
        useCatalog(() => jsonResponse(catalog))
        await renderWithQueryClient(<CourseCatalogPanel />)

        const link = await page.findByRole('link', { name: fixture.GENOSIDE_FIRST_NAME })
        expect(link.getAttribute('href')).toBe(`/courses/${fixture.GENOSIDE_FIRST.courseHash}`)
        const firstRow = link.closest('tr') as HTMLElement
        expect(within(firstRow).getByText('初段')).toBeTruthy()
        expect(within(firstRow).getByText('4')).toBeTruthy()
        const trainingRow = page.getByRole('link', { name: fixture.STELLA_TRAINING_NAME }).closest('tr') as HTMLElement
        expect(trainingRow.textContent).toContain('—')
    })

    test('표에서 빠진 코스는 표에서 빠짐으로 표시하고 그대로 목록에 둔다', async () => {
        const service = createCourseCatalogService({ db: createMemoryCourseCatalogDb().db })
        const [stellaGroup] = fixture.STELLA_DEFINITION.groups
        await service.applySync(fixture.STELLA_DEFINITION)
        await service.applySync({ ...fixture.STELLA_DEFINITION, groups: [{ ...stellaGroup, courses: [fixture.STELLA_FIRST] }] })
        const catalog = await service.listGroups()
        useCatalog(() => jsonResponse(catalog))
        await renderWithQueryClient(<CourseCatalogPanel />)

        const trainingRow = (await page.findByRole('link', { name: fixture.STELLA_TRAINING_NAME })).closest('tr') as HTMLElement
        expect(within(trainingRow).getByText('표에서 빠짐')).toBeTruthy()
        const firstRow = page.getByRole('link', { name: fixture.STELLA_FIRST.name }).closest('tr') as HTMLElement
        expect(within(firstRow).queryByText('표에서 빠짐')).toBeNull()
    })

    test('마이페이지 단위 탭으로 가는 길을 둔다', async () => {
        const catalog = await (await fixture.createCatalogFixture()).listGroups()
        useCatalog(() => jsonResponse(catalog))
        await renderWithQueryClient(<CourseCatalogPanel />)

        expect((await page.findByRole('link', { name: '마이페이지 단위 탭' })).getAttribute('href')).toBe(DAN_SETTINGS_HREF)
    })

    test('등록된 코스가 없으면 빈 상태를 보인다', async () => {
        useCatalog(() => jsonResponse({ groups: [] }))
        await renderWithQueryClient(<CourseCatalogPanel />)

        expect(await page.findByText('등록된 코스가 아직 없습니다')).toBeTruthy()
    })

    test('불러오지 못하면 오류 상태를 보인다', async () => {
        useCatalog(() => errorResponse(500, 'INTERNAL_ERROR', '서버 오류'))
        await renderWithQueryClient(<CourseCatalogPanel />)

        expect(await page.findByText('코스를 불러오지 못했습니다')).toBeTruthy()
    })
})
