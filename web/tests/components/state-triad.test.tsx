import { describe, expect, test } from 'bun:test'
import { renderToStaticMarkup } from 'react-dom/server'
import { EmptyState } from '../../src/features/state/empty-state'
import { LoadErrorState } from '../../src/features/state/load-error-state'
import { QueryErrorState } from '../../src/features/state/query-error-state'

describe('State Triad 3상태 분기', () => {
    test('empty 는 제목만 노출하고 알람 색을 쓰지 않는다', () => {
        const markup = renderToStaticMarkup(<EmptyState title='기록이 없습니다' />)
        expect(markup).toContain('기록이 없습니다')
        expect(markup).not.toContain('text-destructive')
    })

    test('load error 는 아이콘만 destructive 로 칠하고 재시도 안내를 붙인다', () => {
        const markup = renderToStaticMarkup(<LoadErrorState title='랭킹을 불러오지 못했습니다' />)
        expect(markup).toContain('랭킹을 불러오지 못했습니다')
        expect(markup).toContain('요청이 실패했습니다. 잠시 후 다시 시도하세요.')
        expect(markup).toContain('text-destructive')
    })

    test('query error 는 고정 제목 + mono 파서 메시지를 쓰고 알람 색이 없다', () => {
        const markup = renderToStaticMarkup(<QueryErrorState message='level:  <- 값이 비어 있습니다' />)
        expect(markup).toContain('검색식을 해석할 수 없습니다')
        expect(markup).toContain('font-mono')
        expect(markup).not.toContain('text-destructive')
    })
})
