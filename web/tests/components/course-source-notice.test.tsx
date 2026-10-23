import { describe, expect, test } from 'bun:test'
import { renderToStaticMarkup } from 'react-dom/server'
import { CourseSourceNotice } from '../../src/features/course/course-source-notice'
import type { CourseCatalogSource } from '../../src/entities/course.type'
import {
    CONSTRAINT_SOURCE_URL,
    GENOSIDE_FILE_SHA256,
    GENOSIDE_PAGE_URL,
    STELLA_HEADER_SHA256,
    STELLA_PAGE_URL,
    createCatalogFixture,
} from '../support/course-catalog-fixture'

const SP_KEYS = 7
const DP_KEYS = 14
const HASH_PREFIX_LENGTH = 12

const sourcesOf = async () => {
    const { groups } = await (await createCatalogFixture()).listGroups()
    const [genoside, , stella] = groups
    return { genoside: genoside.source, stella: stella.source }
}

const withUrls = (source: CourseCatalogSource, urls: Partial<Pick<CourseCatalogSource, 'page_url' | 'file_url'>>): CourseCatalogSource => ({
    ...source,
    ...urls,
})

describe('CourseSourceNotice', () => {
    test('GENOSIDE 코스는 제작 주체·파일 이름·받은 날·해시 앞 12자와 공식 페이지 링크를 보인다', async () => {
        const { genoside } = await sourcesOf()
        const markup = renderToStaticMarkup(<CourseSourceNotice source={genoside} keys={SP_KEYS} />)
        expect(markup).toContain('GENOCIDE')
        expect(markup).toContain('2018_course.zip')
        expect(markup).toContain('2026-10-23')
        expect(markup).toContain(GENOSIDE_FILE_SHA256.slice(0, HASH_PREFIX_LENGTH))
        expect(markup).not.toContain(GENOSIDE_FILE_SHA256.slice(0, HASH_PREFIX_LENGTH + 1))
        expect(markup).toContain(`href="${GENOSIDE_PAGE_URL}"`)
        expect(markup).toContain('target="_blank"')
        expect(markup).toContain('rel="noopener noreferrer"')
    })

    test('제약·트로피를 읽은 표 머리말은 호스트 이름과 받은 날로 보인다', async () => {
        const { genoside } = await sourcesOf()
        const markup = renderToStaticMarkup(<CourseSourceNotice source={genoside} keys={SP_KEYS} />)
        expect(markup).toContain('제약·트로피')
        expect(markup).toContain(`href="${CONSTRAINT_SOURCE_URL}"`)
        expect(markup).toContain('miraiscarlet.github.io')
        expect(markup).toContain('(2026-10-23 받음)')
    })

    test('제약 원천이 적히지 않은 출처에는 제약·트로피 줄이 없다', async () => {
        const { genoside } = await sourcesOf()
        const markup = renderToStaticMarkup(<CourseSourceNotice source={{ ...genoside, detail: {} }} keys={SP_KEYS} />)
        expect(markup).not.toContain('제약·트로피')
    })

    test('GENOSIDE 2018 은 DP 코스 묶음(14키)에서만 DP 값의 원천이 없다는 안내를 한다', async () => {
        const { genoside } = await sourcesOf()
        expect(renderToStaticMarkup(<CourseSourceNotice source={genoside} keys={DP_KEYS} />)).toContain('DP 값의 원천이 없습니다')
        expect(renderToStaticMarkup(<CourseSourceNotice source={genoside} keys={SP_KEYS} />)).not.toContain('DP 값의 원천이 없습니다')
    })

    test('Stella 표 코스는 표 이름·원본 표 링크·받은 날(KST)·해시 앞 12자를 보인다', async () => {
        const { stella } = await sourcesOf()
        const markup = renderToStaticMarkup(<CourseSourceNotice source={stella} keys={SP_KEYS} />)
        expect(markup).toContain('Stella')
        expect(markup).toContain('stellabms.xyz')
        expect(markup).toContain(`href="${STELLA_PAGE_URL}"`)
        expect(markup).toContain('2026-10-24 00:05')
        expect(markup).toContain('(KST)')
        expect(markup).toContain(STELLA_HEADER_SHA256.slice(0, HASH_PREFIX_LENGTH))
        expect(markup).not.toContain('DP 값의 원천이 없습니다')
    })

    test('http(s) 가 아닌 주소는 링크로 만들지 않고 글자로만 보인다', async () => {
        const { genoside, stella } = await sourcesOf()
        const unsafe = 'javascript:alert(1)'
        const genoside2018 = renderToStaticMarkup(<CourseSourceNotice source={withUrls(genoside, { page_url: unsafe })} keys={SP_KEYS} />)
        const table = renderToStaticMarkup(<CourseSourceNotice source={withUrls(stella, { page_url: unsafe })} keys={SP_KEYS} />)
        for (const markup of [genoside2018, table]) {
            expect(markup).not.toContain('javascript:')
            expect(markup).not.toContain('href="javascript')
        }
        expect(table).toContain('원본 표')
    })
})
