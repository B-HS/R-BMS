import { describe, expect, test } from 'bun:test'
import { SETTING_NAMES_PER_USER_MAX, settingNameSchema, settingPutSchema } from '@server/dto/setting'
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
                writeSetting: async ({ baseUpdatedAt, ...row }) => {
                    const stored = rows.get(row.name)
                    if (baseUpdatedAt !== null && (stored?.updatedAt ?? 0) !== baseUpdatedAt) return false
                    rows.set(row.name, { name: row.name, format: row.format, content: row.content, size: row.size, updatedAt: row.updatedAt })
                    return true
                },
                countSettings: async () => rows.size,
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
        expect(result.status).toBe('stored')
        expect(rows.get('keyconfig')?.content).toBe('(k:2)')
    })

    test('base_updated_at 이 서버 값과 같으면 저장하고 새 updated_at 을 돌려준다', async () => {
        const { service } = createService([storedBlob])
        const result = await service.put({
            userId: USER_ID,
            name: 'keyconfig',
            input: settingPutSchema.parse({ content: '(k:3)', base_updated_at: STORED_UPDATED_AT }),
        })
        expect(result).toEqual({ status: 'stored', updated_at: NOW })
    })

    test('base_updated_at 이 어긋나면 충돌과 서버 최신본을 돌려준다', async () => {
        const { service, rows } = createService([storedBlob])
        const result = await service.put({
            userId: USER_ID,
            name: 'keyconfig',
            input: settingPutSchema.parse({ content: '(k:4)', base_updated_at: STORED_UPDATED_AT - 1 }),
        })
        expect(result).toEqual({ status: 'conflict', server: { name: 'keyconfig', format: 'ron', content: '(k:1)', updated_at: STORED_UPDATED_AT } })
        expect(rows.get('keyconfig')?.content).toBe('(k:1)')
    })

    test('없던 키에 base_updated_at 0 을 보내면 새로 만든다', async () => {
        const { service } = createService()
        const result = await service.put({ userId: USER_ID, name: 'tables', input: settingPutSchema.parse({ content: '()', base_updated_at: 0 }) })
        expect(result.status).toBe('stored')
    })

    test('같은 base_updated_at 으로 동시에 들어온 두 쓰기 중 하나만 저장되고 다른 하나는 충돌과 서버 최신본을 받는다', async () => {
        const { service, rows } = createService([storedBlob])
        const put = (content: string) =>
            service.put({
                userId: USER_ID,
                name: 'keyconfig',
                input: settingPutSchema.parse({ content, base_updated_at: STORED_UPDATED_AT }),
            })
        const [a, b] = await Promise.all([put('from-machine-a'), put('from-machine-b')])
        expect([a.status, b.status].toSorted()).toEqual(['conflict', 'stored'])
        const loser = a.status === 'conflict' ? a : b
        expect(loser).toMatchObject({ server: { content: rows.get('keyconfig')?.content } })
        expect(rows.get('keyconfig')?.content).toBe(a.status === 'stored' ? 'from-machine-a' : 'from-machine-b')
    })

    test('없던 키를 base_updated_at 0 으로 동시에 만들려 하면 하나만 만들어진다', async () => {
        const { service } = createService()
        const create = (content: string) =>
            service.put({ userId: USER_ID, name: 'tables', input: settingPutSchema.parse({ content, base_updated_at: 0 }) })
        const results = await Promise.all([create('first'), create('second')])
        expect(results.map((result) => result.status).toSorted()).toEqual(['conflict', 'stored'])
    })

    test('서버 시계가 저장된 값보다 뒤처져도 새 updated_at 은 base 보다 크다', async () => {
        const { service } = createService([{ ...storedBlob, updatedAt: NOW + 5_000 }])
        const result = await service.put({
            userId: USER_ID,
            name: 'keyconfig',
            input: settingPutSchema.parse({ content: '(k:5)', base_updated_at: NOW + 5_000 }),
        })
        expect(result).toEqual({ status: 'stored', updated_at: NOW + 5_001 })
    })

    test('이름 개수가 한도에 이르면 새 이름은 거절하고 있던 이름은 고칠 수 있다', async () => {
        const seed = Array.from({ length: SETTING_NAMES_PER_USER_MAX }, (_, index) => ({ ...storedBlob, name: `blob-${index}` }))
        const { service } = createService(seed)
        const added = await service.put({ userId: USER_ID, name: 'one-more', input: settingPutSchema.parse({ content: '()' }) })
        expect(added).toEqual({ status: 'too_many_names', limit: SETTING_NAMES_PER_USER_MAX })
        const edited = await service.put({ userId: USER_ID, name: 'blob-0', input: settingPutSchema.parse({ content: '(k:9)' }) })
        expect(edited.status).toBe('stored')
    })
})

describe('settingNameSchema', () => {
    test('64자 이하의 영문·숫자·밑줄·점·하이픈만 받는다', () => {
        expect(settingNameSchema.safeParse('keyconfig').success).toBe(true)
        expect(settingNameSchema.safeParse('player.v2-a_b').success).toBe(true)
        expect(settingNameSchema.safeParse('x'.repeat(64)).success).toBe(true)
        expect(settingNameSchema.safeParse('x'.repeat(65)).success).toBe(false)
        expect(settingNameSchema.safeParse('').success).toBe(false)
        expect(settingNameSchema.safeParse('a/b').success).toBe(false)
        expect(settingNameSchema.safeParse('a b').success).toBe(false)
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
