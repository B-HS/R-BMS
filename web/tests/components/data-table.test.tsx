import { describe, expect, test } from 'bun:test'
import { renderToStaticMarkup } from 'react-dom/server'
import { DataTable, type DataTableColumn } from '../../src/features/table/data-table'

type Row = { id: string; name: string; score: number; ranked: boolean }

const ROWS: Row[] = [
    { id: 'sc_1', name: 'alice', score: 4390, ranked: true },
    { id: 'sc_2', name: 'bob', score: 3611, ranked: false },
]

const COLUMNS: DataTableColumn<Row>[] = [
    { key: 'name', label: '플레이어', flex: true, cell: (row) => row.name },
    { key: 'id', label: 'ID', width: 96, mono: true, cell: (row) => row.id },
    { key: 'score', label: 'EX', width: 96, align: 'right', cell: (row) => row.score },
]

const markup = () =>
    renderToStaticMarkup(<DataTable columns={COLUMNS} rows={ROWS} rowKey={(row) => row.id} rowMuted={(row) => !row.ranked} caption='테스트 표' />)

describe('DataTable 컬럼 디스크립터', () => {
    test('우측 정렬 컬럼은 tabular-nums 를 함께 갖는다', () => {
        expect(markup()).toContain('text-right tabular-nums')
    })

    test('flex 컬럼만 max-w-0 말줄임을 갖는다', () => {
        const html = markup()
        expect(html).toContain('max-w-0')
        expect(html.match(/max-w-0/g)?.length).toBe(ROWS.length)
    })

    test('mono 컬럼은 font-mono 를 갖는다', () => {
        expect(markup()).toContain('font-mono')
    })

    test('rowMuted 인 행만 muted 로 표시한다', () => {
        const bodyRows = markup().split('<tbody')[1].split('<tr').slice(1)
        expect(bodyRows.length).toBe(2)
        expect(bodyRows[0]).not.toContain('text-muted-foreground')
        expect(bodyRows[1]).toContain('text-muted-foreground')
    })

    test('테이블은 가로 스크롤 컨테이너로 감싸이고 caption 을 갖는다', () => {
        const html = markup()
        expect(html).toContain('overflow-x-auto')
        expect(html).toContain('테스트 표')
    })
})
