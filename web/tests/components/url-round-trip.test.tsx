import { afterAll, describe, expect, test } from 'bun:test'
import { GlobalRegistrator } from '@happy-dom/global-registrator'

GlobalRegistrator.register()

afterAll(() => GlobalRegistrator.unregister())

const { fireEvent, render, screen, cleanup } = await import('@testing-library/react')
const { Pager } = await import('../../src/features/table/pager')
const { ColumnToggle } = await import('../../src/features/table/column-toggle')
const { serializeSearchParams, toggleListValue } = await import('../../src/shared/lib/search-params')

const COLUMN_OPTIONS = [
    { key: 'mode', label: '모드' },
    { key: 'level', label: '레벨' },
    { key: 'notes', label: '노트' },
]

describe('Pager 클릭 → URL 파라미터 왕복', () => {
    test('다음 페이지 클릭이 page=3 쿼리를 만든다', () => {
        let url = ''
        render(<Pager page={2} totalPages={5} onPageChange={(page) => (url = serializeSearchParams({ sort: 'popular', page }))} />)
        fireEvent.click(screen.getByLabelText('다음 페이지'))
        cleanup()
        expect(url).toBe('sort=popular&page=3')
    })

    test('이전 페이지 클릭이 page=1 쿼리를 만든다', () => {
        let url = ''
        render(<Pager page={2} totalPages={5} onPageChange={(page) => (url = serializeSearchParams({ page }))} />)
        fireEvent.click(screen.getByLabelText('이전 페이지'))
        cleanup()
        expect(url).toBe('page=1')
    })

    test('첫 페이지에서 이전 버튼은 pointer-events 가 꺼진다', () => {
        render(<Pager page={1} totalPages={5} onPageChange={() => undefined} />)
        const previous = screen.getByLabelText('이전 페이지')
        cleanup()
        expect(previous.className).toContain('pointer-events-none')
    })
})

describe('ColumnToggle 클릭 → cols 파라미터 왕복', () => {
    test('선택된 컬럼을 끄면 cols 에서 빠진다', () => {
        let url = ''
        const selected = ['mode', 'level', 'notes']
        render(
            <ColumnToggle
                options={COLUMN_OPTIONS}
                selected={selected}
                onToggle={(key) => (url = serializeSearchParams({ cols: toggleListValue(selected, key) }))}
            />,
        )
        fireEvent.click(screen.getByText('레벨'))
        cleanup()
        expect(url).toBe('cols=mode%2Cnotes')
    })

    test('꺼진 컬럼을 켜면 cols 뒤에 붙는다', () => {
        let url = ''
        const selected = ['mode']
        render(
            <ColumnToggle
                options={COLUMN_OPTIONS}
                selected={selected}
                onToggle={(key) => (url = serializeSearchParams({ cols: toggleListValue(selected, key) }))}
            />,
        )
        fireEvent.click(screen.getByText('노트'))
        cleanup()
        expect(url).toBe('cols=mode%2Cnotes')
    })

    test('선택 상태가 aria-pressed 로 노출된다', () => {
        render(<ColumnToggle options={COLUMN_OPTIONS} selected={['mode']} onToggle={() => undefined} />)
        const pressed = screen.getByText('모드').getAttribute('aria-pressed')
        const notPressed = screen.getByText('레벨').getAttribute('aria-pressed')
        cleanup()
        expect(pressed).toBe('true')
        expect(notPressed).toBe('false')
    })
})
