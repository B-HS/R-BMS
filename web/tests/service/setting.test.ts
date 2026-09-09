import { describe, expect, test } from 'bun:test'
import { settingPutSchema } from '@server/dto/setting'
import { createSettingService, isSettingConflict, type SettingRow } from '@server/service/domain/setting/setting.service'

const USER_ID = 'user-1'
const NOW = 1_700_000_050_000
const STORED_UPDATED_AT = 1_700_000_000_000

const createService = (seed: SettingRow[] = []) => {
    const rows = new Map(seed.map((row) => [row.name, row]))
    return {
        rows,
        service: createSettingService({
            now: () => NOW,
            db: {
                findSetting: async ({ name }) => rows.get(name) ?? null,
                upsertSetting: async (row) => {
                    rows.set(row.name, { name: row.name, format: row.format, content: row.content, size: row.size, updatedAt: row.updatedAt })
                },
                listSettings: async () => [...rows.values()],
            },
        }),
    }
}

const storedBlob: SettingRow = { name: 'keyconfig', format: 'ron', content: '(k:1)', size: 5, updatedAt: STORED_UPDATED_AT }

describe('settingService.put', () => {
    test('base_updated_at 이 없으면 마지막 쓰기가 이긴다', async () => {
        const { service, rows } = createService([storedBlob])
        const result = await service.put({ userId: USER_ID, name: 'keyconfig', input: settingPutSchema.parse({ content: '(k:2)' }) })
        expect(result.conflict).toBe(false)
        expect(rows.get('keyconfig')?.content).toBe('(k:2)')
    })

    test('base_updated_at 이 서버 값과 같으면 저장하고 새 updated_at 을 돌려준다', async () => {
        const { service } = createService([storedBlob])
        const result = await service.put({
            userId: USER_ID,
            name: 'keyconfig',
            input: settingPutSchema.parse({ content: '(k:3)', base_updated_at: STORED_UPDATED_AT }),
        })
        expect(result).toEqual({ conflict: false, updated_at: NOW })
    })

    test('base_updated_at 이 어긋나면 충돌과 서버 최신본을 돌려준다', async () => {
        const { service, rows } = createService([storedBlob])
        const result = await service.put({
            userId: USER_ID,
            name: 'keyconfig',
            input: settingPutSchema.parse({ content: '(k:4)', base_updated_at: STORED_UPDATED_AT - 1 }),
        })
        expect(result).toEqual({ conflict: true, server: { name: 'keyconfig', format: 'ron', content: '(k:1)', updated_at: STORED_UPDATED_AT } })
        expect(rows.get('keyconfig')?.content).toBe('(k:1)')
    })

    test('없던 키에 base_updated_at 0 을 보내면 새로 만든다', async () => {
        const { service } = createService()
        const result = await service.put({ userId: USER_ID, name: 'tables', input: settingPutSchema.parse({ content: '()', base_updated_at: 0 }) })
        expect(result.conflict).toBe(false)
    })
})

describe('settingService.get / listKeys', () => {
    test('없는 이름은 null 이다', async () => {
        const { service } = createService()
        expect(await service.get({ userId: USER_ID, name: 'settings' })).toBeNull()
    })

    test('키 목록은 content 대신 크기와 갱신시각을 노출한다', async () => {
        const { service } = createService([storedBlob])
        expect(await service.listKeys(USER_ID)).toEqual([{ key: 'keyconfig', format: 'ron', updated_at: STORED_UPDATED_AT, size: 5 }])
    })
})

describe('isSettingConflict', () => {
    test('base 가 없으면 충돌이 아니다', () => {
        expect(isSettingConflict(null, storedBlob)).toBe(false)
    })

    test('서버에 아직 없을 때 0 이 아닌 base 를 보내면 충돌이다', () => {
        expect(isSettingConflict(STORED_UPDATED_AT, null)).toBe(true)
        expect(isSettingConflict(0, null)).toBe(false)
    })
})
