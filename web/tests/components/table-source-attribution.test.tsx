import { afterAll, afterEach, describe, expect, test } from 'bun:test'
import { GlobalRegistrator } from '@happy-dom/global-registrator'

GlobalRegistrator.register({ url: 'http://localhost:3000/' })

afterAll(() => GlobalRegistrator.unregister())

const { mount, page, unmountAll } = await import('../support/dom-harness')
const { TableSourceAttribution } = await import('../../src/features/table/table-source-attribution')

type TableSource = import('../../src/server/dto/table-snapshot').TableSourceOutput

const DATA_SHA256 = '0123456789abcdef'.repeat(4)
const FETCHED_AT_UTC_MS = Date.UTC(2026, 9, 22, 15, 0)
const CHECKED_AT_UTC_MS = Date.UTC(2026, 9, 23, 15, 30)

const SOURCE: TableSource = {
    name: 'Stella',
    maker: 'stellabms.xyz',
    page_url: 'https://stellabms.xyz/st/table.html',
    fetched_at: FETCHED_AT_UTC_MS,
    checked_at: CHECKED_AT_UTC_MS,
    data_sha256: DATA_SHA256,
}

afterEach(async () => {
    await unmountAll()
})

describe('표 출처 표기', () => {
    test('출처 이름, 원본 링크(새 탭), KST 날짜, 해시 앞 12자를 보인다', async () => {
        await mount(<TableSourceAttribution source={SOURCE} />)

        const region = page.getByRole('region', { name: '표 출처' })
        expect(region.textContent).toContain('출처 Stella(stellabms.xyz)')
        expect(region.textContent).toContain('내용 받은 날 2026-10-23 00:00')
        expect(region.textContent).toContain('마지막 확인 2026-10-24 00:30')
        expect(region.textContent).toContain(`SHA-256 ${DATA_SHA256.slice(0, 12)}`)
        expect(region.textContent).not.toContain(DATA_SHA256.slice(0, 13))

        const link = page.getByRole('link', { name: /원본 표/ })
        expect(link.getAttribute('href')).toBe(SOURCE.page_url)
        expect(link.getAttribute('target')).toBe('_blank')
        expect(link.getAttribute('rel')).toBe('noopener noreferrer')
    })
})
