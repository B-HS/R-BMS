import { afterAll, afterEach, describe, expect, test } from 'bun:test'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { GlobalRegistrator } from '@happy-dom/global-registrator'
import { AppRouterContext } from 'next/dist/shared/lib/app-router-context.shared-runtime'
import { PathnameContext } from 'next/dist/shared/lib/hooks-client-context.shared-runtime'

GlobalRegistrator.register({ url: 'http://localhost:3000/' })

afterAll(() => GlobalRegistrator.unregister())

const { cleanup, render, within } = await import('@testing-library/react')
const { CourseDetailPanel } = await import('../../src/widgets/course/course-detail-panel')
const { QUERY_KEY } = await import('../../src/shared/constants/query-key')
const { SidebarProvider } = await import('../../src/shared/ui/sidebar')
const { DAN_SETTINGS_HREF } = await import('../../src/shared/constants/navigation')
const fixture = await import('../support/course-catalog-fixture')
const { createCourseCatalogService } = await import('../../src/server/service/domain/course-catalog/course-catalog.service')
const { createMemoryCourseCatalogDb } = await import('../support/course-catalog-memory-db')

type CourseMeta = import('../../src/entities/course.type').CourseMeta
type CourseCatalogEntry = import('../../src/entities/course.type').CourseCatalogEntry
type CatalogCourse = typeof fixture.GENOSIDE_FIRST
type CatalogDefinition = typeof fixture.GENOSIDE_DEFINITION

const RANKING_PARAMS = { limit: 50, include_scratch_auto: false }
const SHA256_OF_STAGE_ONE = 'f'.repeat(64)
const FIRST_STAGE = 0
const SECOND_STAGE = 1
const PATHNAME = '/courses/x'

const router = { push: () => undefined, back: () => undefined, forward: () => undefined, refresh: () => undefined }

const metaOf = (course: CatalogCourse, sha256s: string[]): CourseMeta => ({
    course_hash: course.courseHash,
    name: course.name,
    lntype: 0,
    charts: course.charts.map((chart, stage) => ({ md5: chart.md5, sha256: sha256s[stage] ?? '' })),
    constraint: course.constraint,
    trophy: [],
    extra: {},
})

const entryOf = async (definition: CatalogDefinition, courseHash: string): Promise<CourseCatalogEntry | null> => {
    const service = createCourseCatalogService({ db: createMemoryCourseCatalogDb().db })
    await service.applySync(definition)
    return service.getEntry(courseHash)
}

const renderDetail = (course: CatalogCourse, entry: CourseCatalogEntry | null, sha256s: string[] = []) => {
    const queryClient = new QueryClient({ defaultOptions: { queries: { staleTime: Infinity } } })
    queryClient.setQueryData(QUERY_KEY.COURSE.META(course.courseHash), metaOf(course, sha256s))
    queryClient.setQueryData(QUERY_KEY.COURSE.RANKING(course.courseHash, RANKING_PARAMS), [])
    queryClient.setQueryData(QUERY_KEY.COURSE.CATALOG_ENTRY(course.courseHash), entry)
    return render(
        <AppRouterContext.Provider value={router as never}>
            <PathnameContext.Provider value={PATHNAME}>
                <QueryClientProvider client={queryClient}>
                    <SidebarProvider>
                        <CourseDetailPanel courseHash={course.courseHash} rankingParams={RANKING_PARAMS} />
                    </SidebarProvider>
                </QueryClientProvider>
            </PathnameContext.Provider>
        </AppRouterContext.Provider>,
    )
}

afterEach(() => cleanup())

describe('코스 상세의 카탈로그 정보', () => {
    test('곡 제목을 차트 순서대로 보이고 sha256 이 없는 차트는 md5 로 차트 페이지에 잇는다', async () => {
        const entry = await entryOf(fixture.GENOSIDE_DEFINITION, fixture.GENOSIDE_FIRST.courseHash)
        const view = renderDetail(fixture.GENOSIDE_FIRST, entry, ['', SHA256_OF_STAGE_ONE])

        const [firstChart, secondChart] = fixture.GENOSIDE_FIRST.charts
        const first = view.getByRole('link', { name: firstChart.title })
        const second = view.getByRole('link', { name: secondChart.title })
        expect(first.getAttribute('href')).toBe(`/charts/${firstChart.md5}`)
        expect(second.getAttribute('href')).toBe(`/charts/${SHA256_OF_STAGE_ONE}`)
        const rows = view.getAllByRole('row').slice(1)
        expect(rows[FIRST_STAGE].textContent).toContain('—')
        expect(rows[SECOND_STAGE].textContent).toContain(SHA256_OF_STAGE_ONE.slice(0, 8))
    })

    test('이 코스가 주는 단위와 인정 조건 링크, 출처 블록을 보인다', async () => {
        const entry = await entryOf(fixture.GENOSIDE_DEFINITION, fixture.GENOSIDE_FIRST.courseHash)
        const view = renderDetail(fixture.GENOSIDE_FIRST, entry)

        expect(view.getByText(/GENOSIDE 2018 SP 初段 단위를 얻습니다/)).toBeTruthy()
        expect(view.getByRole('link', { name: '인정 조건' }).getAttribute('href')).toBe(DAN_SETTINGS_HREF)
        expect(view.getByText(/코스 정의 GENOCIDE 공식 코스 파일/)).toBeTruthy()
        expect(view.getByRole('link', { name: /^공식 페이지/ }).getAttribute('href')).toBe(fixture.GENOSIDE_PAGE_URL)
    })

    test('단위를 주지 않는 카탈로그 코스는 단위 줄 없이 출처만 보인다', async () => {
        const entry = await entryOf(fixture.STELLA_DEFINITION, fixture.STELLA_TRAINING.courseHash)
        const view = renderDetail(fixture.STELLA_TRAINING, entry)

        expect(view.queryByText(/단위를 얻습니다/)).toBeNull()
        expect(within(view.container).getByText(/내용 받은 날/)).toBeTruthy()
    })

    test('카탈로그에 없는 코스(플레이어가 가져온 코스)는 출처와 단위 줄 없이 해시로 차트를 보인다', () => {
        const view = renderDetail(fixture.GENOSIDE_FIRST, null)

        expect(view.queryByText(/단위를 얻습니다/)).toBeNull()
        expect(view.queryByText(/공식 코스 파일/)).toBeNull()
        expect(view.queryByText(fixture.GENOSIDE_FIRST_TITLE)).toBeNull()
        expect(view.getByRole('link', { name: fixture.GENOSIDE_FIRST.charts[FIRST_STAGE].md5.slice(0, 8) + '…' })).toBeTruthy()
    })
})
