import { afterEach, describe, expect, test } from 'bun:test'
import { renderToStaticMarkup } from 'react-dom/server'
import { TableChartCard } from '../../src/features/table/table-chart-card'
import { TableChartGrid } from '../../src/features/table/table-chart-grid'
import { tableSnapshotRecordsQueryOptions } from '../../src/entities/table-snapshot.query'
import type { TableSnapshotEntry, TableSnapshotLevel, TableSnapshotRecord } from '../../src/entities/table-snapshot.type'
import {
    countActiveTableFilters,
    countTableLamps,
    DEFAULT_TABLE_FILTER,
    filterTableLevels,
    levelAnchorId,
    readTableFilter,
    summariseTableRecords,
    writeTableFilter,
} from '../../src/widgets/table/table-snapshot-filter'

const entry = (md5Seed: string, level: string, title: string, artist = 'artist'): TableSnapshotEntry => ({
    md5: md5Seed.padEnd(32, '0'),
    level,
    title,
    artist,
})

const record = (clear: TableSnapshotRecord['clear'], exScore: number, totalNotes = 1000): TableSnapshotRecord => ({
    clear,
    ex_score: exScore,
    minbp: 3,
    max_combo: 500,
    total_notes: totalNotes,
    played_at: 1_791_949_770_000,
    scratch_auto: false,
})

const LEVELS: TableSnapshotLevel[] = [
    { level: '1', entries: [entry('a1', '1', 'Zeta'), entry('a2', '1', 'Alpha'), entry('a3', '1', 'ＡＮＯＴＨＥＲ wide')] },
    { level: '2', entries: [entry('b1', '2', 'Beta'), entry('b2', '2', 'Gamma', 'xi')] },
    { level: '???', entries: [entry('c1', '???', 'Unknown')] },
]
const ORDER = LEVELS.map((group) => group.level)
const RECORDS = { [LEVELS[0].entries[0].md5]: record('Hard', 1800), [LEVELS[0].entries[1].md5]: record('Failed', 900) }

describe('발광 표 주소 필터', () => {
    test('주소의 값을 읽고 표에 없는 레벨과 모르는 값은 버린다', () => {
        const filter = readTableFilter(new URLSearchParams('q=xi&from=2&to=99&lamp=hard&sort=bogus'), ORDER)
        expect(filter).toEqual({ query: 'xi', from: '2', to: null, lamp: 'hard', sort: 'table' })
    })

    test('거꾸로 준 레벨 범위는 바로잡는다', () => {
        const filter = readTableFilter(new URLSearchParams('from=???&to=1'), ORDER)
        expect(filter.from).toBe('1')
        expect(filter.to).toBe('???')
    })

    test('기본값은 주소에 쓰지 않고 읽은 값과 왕복한다', () => {
        expect(writeTableFilter(DEFAULT_TABLE_FILTER)).toBe('')
        const query = writeTableFilter({ query: 'born ', from: '2', to: '???', lamp: 'easy', sort: 'rate' })
        expect(readTableFilter(new URLSearchParams(query), ORDER)).toEqual({ query: 'born ', from: '2', to: '???', lamp: 'easy', sort: 'rate' })
        expect(countActiveTableFilters(readTableFilter(new URLSearchParams(query), ORDER))).toBe(3)
    })

    test('레벨 앵커는 주소에 안전한 이름만 쓴다', () => {
        expect(levelAnchorId('12', 11)).toBe('lv-12')
        expect(levelAnchorId('???', 25)).toBe('lv-other-25')
    })
})

describe('발광 표 걸러내기', () => {
    test('검색은 전각과 대소문자를 접어 제목과 아티스트를 찾고 맞는 곡이 없는 레벨을 숨긴다', () => {
        const levels = filterTableLevels(LEVELS, { ...DEFAULT_TABLE_FILTER, query: 'another' }, null)
        expect(levels.map((group) => group.level)).toEqual(['1'])
        expect(levels[0].entries.map((item) => item.title)).toEqual(['ＡＮＯＴＨＥＲ wide'])
        expect(levels[0].levelEntries).toHaveLength(3)
        expect(filterTableLevels(LEVELS, { ...DEFAULT_TABLE_FILTER, query: 'XI' }, null)[0].entries[0].title).toBe('Gamma')
    })

    test('md5 앞 여섯 자 이상으로도 찾는다', () => {
        const levels = filterTableLevels(LEVELS, { ...DEFAULT_TABLE_FILTER, query: 'b20000' }, null)
        expect(levels.flatMap((group) => group.entries.map((item) => item.title))).toEqual(['Gamma'])
    })

    test('레벨 범위 밖은 빼고 조건이 없으면 빈 레벨도 남긴다', () => {
        const levels = filterTableLevels(LEVELS, { ...DEFAULT_TABLE_FILTER, from: '2' }, null)
        expect(levels.map((group) => group.level)).toEqual(['2', '???'])
    })

    test('램프 조건은 기록이 있을 때만 걸리고 기록 없는 곡은 NO PLAY 로 본다', () => {
        expect(filterTableLevels(LEVELS, { ...DEFAULT_TABLE_FILTER, lamp: 'easy' }, null).flatMap((group) => group.entries)).toHaveLength(6)
        const cleared = filterTableLevels(LEVELS, { ...DEFAULT_TABLE_FILTER, lamp: 'easy' }, RECORDS)
        expect(cleared.flatMap((group) => group.entries.map((item) => item.title))).toEqual(['Zeta'])
        const unplayed = filterTableLevels(LEVELS, { ...DEFAULT_TABLE_FILTER, lamp: 'noplay' }, RECORDS)
        expect(unplayed.flatMap((group) => group.entries)).toHaveLength(4)
    })

    test('곡명 순과 램프 낮은 순으로 묶음 안을 정렬한다', () => {
        const byTitle = filterTableLevels(LEVELS, { ...DEFAULT_TABLE_FILTER, sort: 'title' }, null)
        expect(byTitle[0].entries.map((item) => item.title)).toEqual(['Alpha', 'ＡＮＯＴＨＥＲ wide', 'Zeta'])
        const byLamp = filterTableLevels(LEVELS, { ...DEFAULT_TABLE_FILTER, sort: 'lamp' }, RECORDS)
        expect(byLamp[0].entries.map((item) => item.title)).toEqual(['ＡＮＯＴＨＥＲ wide', 'Alpha', 'Zeta'])
    })

    test('램프 분포는 좋은 램프부터 세고 기록 없는 곡을 NO PLAY 로 센다', () => {
        const lamps = countTableLamps(LEVELS[0].entries, RECORDS)
        expect(lamps[0].lamp).toBe('Max')
        expect(lamps.at(-1)).toEqual({ lamp: 'NoPlay', count: 1 })
        expect(lamps.find((item) => item.lamp === 'Hard')?.count).toBe(1)
        expect(summariseTableRecords(RECORDS, 6)).toMatchObject({ played: 2 })
        expect(summariseTableRecords(RECORDS, 6).lamps.at(-1)).toEqual({ lamp: 'NoPlay', count: 4 })
    })
})

describe('발광 표 카드', () => {
    const noop = () => undefined

    test('기록이 없으면 회색 띠와 곡 정보만 보인다', () => {
        const markup = renderToStaticMarkup(
            <TableChartCard entry={LEVELS[0].entries[0]} symbol='★' record={undefined} isRecordPending={false} onOpen={noop} />,
        )
        expect(markup).toContain('var(--rbms-lamp-noplay)')
        expect(markup).toContain('aria-label="Zeta · artist · ★1 · NO PLAY"')
        expect(markup).toContain('aria-haspopup="dialog"')
    })

    test('기록이 있으면 램프 띠, EX, 랭크를 그리고 읽어 준다', () => {
        const markup = renderToStaticMarkup(
            <TableChartCard entry={LEVELS[0].entries[0]} symbol='★' record={record('Hard', 1800)} isRecordPending={false} onOpen={noop} />,
        )
        expect(markup).toContain('var(--rbms-lamp-hard)')
        expect(markup).toContain('1,800')
        expect(markup).toContain('· HARD · AAA-')
    })

    test('격자는 마지막 줄을 열 수마다 빈 칸으로 채운다', () => {
        const markup = renderToStaticMarkup(
            <TableChartGrid label='★1 곡' symbol='★' entries={LEVELS[0].entries} records={null} isRecordPending={false} onOpen={noop} />,
        )
        expect(markup.match(/aria-hidden="true" class="bg-card min-h-14/g)?.length).toBe(1 + 0 + 1 + 2 + 3)
    })
})

const originalFetch = globalThis.fetch

const run = async <T,>(options: { queryFn?: unknown }) => (options.queryFn as () => Promise<T>)()

afterEach(() => {
    globalThis.fetch = originalFetch
})

describe('발광 표 쿼리', () => {
    test('내 기록은 봉투를 벗긴 md5 맵을 돌려주고 사용자별 키를 쓴다', async () => {
        const calls: string[] = []
        globalThis.fetch = Object.assign(async (input: RequestInfo | URL) => {
            calls.push(String(input))
            return new Response('{"success":true,"data":{"table_id":"insane1","records":{"abc":{"clear":"Hard"}}}}', { status: 200 })
        }, originalFetch)
        const options = tableSnapshotRecordsQueryOptions('insane1', 'user-1')
        expect([...options.queryKey]).toEqual(['table-snapshot', 'records', 'insane1', 'user-1'])
        const records = await run<Record<string, unknown>>(options)
        expect(Object.keys(records)).toEqual(['abc'])
        expect(calls[0]).toEndWith('/api/fe/tables/insane1/records')
    })
})
