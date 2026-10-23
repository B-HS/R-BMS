import { describe, expect, test } from 'bun:test'
import {
    TABLE_FIRST_SNAPSHOT_MIN_ENTRIES,
    TABLE_REFRESH_REASON,
    deriveLevelOrder,
    diffEntries,
    judgeChange,
    parseJsonBytes,
    validateRemoteEntries,
    validateRemoteHeader,
} from '@server/service/domain/table-refresh/table-refresh.validate'
import { encodeJson, remoteEntries, satelliteHeader, STELLA, stellaHeader } from '../support/remote-table-fixture'

const ENTRY_HUNDRED = 100
const PREVIOUS_THOUSAND = 1000

const reasonOf = (result: { ok: boolean; reason?: string }) => (result.ok ? null : result.reason)

describe('JSON 읽기', () => {
    test('BOM 이 붙은 머리도 읽는다', () => {
        const bytes = new Uint8Array([0xef, 0xbb, 0xbf, ...encodeJson(stellaHeader())])
        const parsed = parseJsonBytes(bytes)
        expect(parsed.ok).toBe(true)
        expect(validateRemoteHeader(STELLA, parsed.ok ? parsed.value : null).ok).toBe(true)
    })

    test('JSON 이 아니거나 UTF-8 이 아니면 not_json', () => {
        expect(reasonOf(parseJsonBytes(new TextEncoder().encode('<html></html>')))).toBe(TABLE_REFRESH_REASON.NOT_JSON)
        expect(reasonOf(parseJsonBytes(new Uint8Array([0x5b, 0xff, 0xfe, 0x5d])))).toBe(TABLE_REFRESH_REASON.NOT_JSON)
    })
})

describe('머리 검증', () => {
    test('이름이 다르면 name_mismatch', () => {
        expect(reasonOf(validateRemoteHeader(STELLA, stellaHeader({ name: 'Satellite' })))).toBe(TABLE_REFRESH_REASON.NAME_MISMATCH)
    })

    test('기호가 다르면 symbol_mismatch', () => {
        expect(reasonOf(validateRemoteHeader(STELLA, stellaHeader({ symbol: 'sl' })))).toBe(TABLE_REFRESH_REASON.SYMBOL_MISMATCH)
    })

    test('모양이 틀린 머리는 header_invalid', () => {
        expect(reasonOf(validateRemoteHeader(STELLA, { name: 'Stella' }))).toBe(TABLE_REFRESH_REASON.HEADER_INVALID)
        expect(reasonOf(validateRemoteHeader(STELLA, 'Stella'))).toBe(TABLE_REFRESH_REASON.HEADER_INVALID)
    })

    test('data_url 이 다른 호스트면 data_url_off_site', () => {
        for (const dataUrl of ['https://evil.example/score.json', 'http://stellabms.xyz/score.json', '//other.example/score.json']) {
            expect(reasonOf(validateRemoteHeader(STELLA, stellaHeader({ data_url: dataUrl })))).toBe(TABLE_REFRESH_REASON.DATA_URL_OFF_SITE)
        }
    })

    test('상대 data_url score.json 은 머리와 같은 폴더로 합쳐진다', () => {
        const result = validateRemoteHeader(STELLA, stellaHeader())
        expect(result.ok && result.dataUrl).toBe('https://stellabms.xyz/st/score.json')
        const satellite = validateRemoteHeader(
            { ...STELLA, expectedName: 'Satellite', symbol: 'sl', headerUrl: 'https://stellabms.xyz/sl/header.json' },
            satelliteHeader(),
        )
        expect(satellite.ok && satellite.dataUrl).toBe('https://stellabms.xyz/sl/score.json')
    })

    test('level_order 와 last_update 가 없어도 받아들이고 course 는 그대로 들고 간다', () => {
        const result = validateRemoteHeader(STELLA, stellaHeader())
        expect(result.ok && result.header.level_order).toBeUndefined()
        expect(result.ok && result.header.course).toEqual(stellaHeader().course)
    })
})

describe('항목 검증', () => {
    test('md5 가 32자 hex 가 아니면 entries_invalid', () => {
        expect(reasonOf(validateRemoteEntries([{ ...remoteEntries(1)[0], md5: 'not-a-hash' }]))).toBe(TABLE_REFRESH_REASON.ENTRIES_INVALID)
        expect(reasonOf(validateRemoteEntries({ not: 'an array' }))).toBe(TABLE_REFRESH_REASON.ENTRIES_INVALID)
    })

    test('한 항목이라도 틀리면 전체가 거절된다', () => {
        const entries = [...remoteEntries(3), { ...remoteEntries(1, { offset: 9 })[0], sha256: 'short' }]
        expect(reasonOf(validateRemoteEntries(entries))).toBe(TABLE_REFRESH_REASON.ENTRIES_INVALID)
    })

    test('숫자 레벨도 글자로 읽고 md5 는 소문자로 바꾼다', () => {
        const [entry] = remoteEntries(1)
        const result = validateRemoteEntries([{ ...entry, md5: entry.md5.toUpperCase(), level: 7 }])
        expect(result.ok && result.entries[0].level).toBe('7')
        expect(result.ok && result.entries[0].md5).toBe(entry.md5)
    })

    test('sha256 이 없거나 비어 있어도 되고, 글자가 길면 거절된다', () => {
        const [entry] = remoteEntries(1)
        const withoutSha = Object.fromEntries(Object.entries(entry).filter(([key]) => key !== 'sha256'))
        expect(validateRemoteEntries([withoutSha]).ok).toBe(true)
        expect(validateRemoteEntries([{ ...entry, sha256: '' }]).ok).toBe(true)
        expect(reasonOf(validateRemoteEntries([{ ...entry, level: 'x'.repeat(33) }]))).toBe(TABLE_REFRESH_REASON.ENTRIES_INVALID)
        expect(reasonOf(validateRemoteEntries([{ ...entry, level: '' }]))).toBe(TABLE_REFRESH_REASON.ENTRIES_INVALID)
    })

    test('같은 md5 가 둘이면 duplicate_md5', () => {
        const [entry] = remoteEntries(1)
        expect(reasonOf(validateRemoteEntries([entry, { ...entry, md5: entry.md5.toUpperCase() }]))).toBe(TABLE_REFRESH_REASON.DUPLICATE_MD5)
    })
})

describe('레벨 순서', () => {
    test('level_order 가 없으면 레벨을 수 순서로 만든다', () => {
        const entries = ['0', '12', '3', '12'].map((level) => ({ level }))
        expect(deriveLevelOrder({}, entries)).toEqual(['0', '3', '12'])
    })

    test('글자 레벨이 섞이면 처음 나온 순서', () => {
        const entries = ['b', '3', 'a', '3'].map((level) => ({ level }))
        expect(deriveLevelOrder({}, entries)).toEqual(['b', '3', 'a'])
    })

    test('level_order 가 있으면 그것을 쓰고 빈 배열은 없는 것으로 본다', () => {
        const entries = ['1', '2'].map((level) => ({ level }))
        expect(deriveLevelOrder({ level_order: ['2', '1'] }, entries)).toEqual(['2', '1'])
        expect(deriveLevelOrder({ level_order: [] }, entries)).toEqual(['1', '2'])
    })
})

describe('변경 판정', () => {
    const previous = remoteEntries(PREVIOUS_THOUSAND)

    test('diffEntries 는 늘어난 곡, 사라진 곡, 레벨이 옮겨진 곡을 센다', () => {
        const next = [...previous.slice(2), ...remoteEntries(5, { offset: PREVIOUS_THOUSAND }), { ...previous[0], level: 'moved' }]
        expect(diffEntries(previous, next)).toEqual({ added: 5, removed: 1, releveled: 1 })
        expect(diffEntries([], next)).toEqual({ added: next.length, removed: 0, releveled: 0 })
    })

    test('첫 저장이 500곡 미만이면 too_few_entries', () => {
        const base = { previousCount: null, diff: null, allowLargeChange: false }
        expect(reasonOf(judgeChange({ ...base, nextCount: TABLE_FIRST_SNAPSHOT_MIN_ENTRIES - 1 }))).toBe(TABLE_REFRESH_REASON.TOO_FEW_ENTRIES)
        expect(judgeChange({ ...base, nextCount: TABLE_FIRST_SNAPSHOT_MIN_ENTRIES }).ok).toBe(true)
    })

    test('10% 넘게 줄면 shrank, 딱 10% 줄어든 것은 받는다', () => {
        const base = { previousCount: PREVIOUS_THOUSAND, diff: { added: 0, removed: 0, releveled: 0 }, allowLargeChange: false }
        expect(reasonOf(judgeChange({ ...base, nextCount: 899 }))).toBe(TABLE_REFRESH_REASON.SHRANK)
        expect(judgeChange({ ...base, nextCount: 900 }).ok).toBe(true)
    })

    test('25% 넘게 바뀌면 changed_too_much — 레벨 이동도 센다', () => {
        const base = { previousCount: PREVIOUS_THOUSAND, nextCount: PREVIOUS_THOUSAND, allowLargeChange: false }
        expect(reasonOf(judgeChange({ ...base, diff: { added: 0, removed: 0, releveled: 251 } }))).toBe(TABLE_REFRESH_REASON.CHANGED_TOO_MUCH)
        expect(judgeChange({ ...base, diff: { added: 100, removed: 0, releveled: 150 } }).ok).toBe(true)
    })

    test('강제 반영은 급감·변경량만 건너뛰고 빈 표는 여전히 거절한다', () => {
        const forced = { previousCount: PREVIOUS_THOUSAND, diff: { added: 0, removed: 800, releveled: 0 }, allowLargeChange: true }
        expect(judgeChange({ ...forced, nextCount: ENTRY_HUNDRED }).ok).toBe(true)
        expect(reasonOf(judgeChange({ ...forced, nextCount: 0 }))).toBe(TABLE_REFRESH_REASON.TOO_FEW_ENTRIES)
        expect(reasonOf(judgeChange({ ...forced, previousCount: null, nextCount: ENTRY_HUNDRED }))).toBe(TABLE_REFRESH_REASON.TOO_FEW_ENTRIES)
    })
})
