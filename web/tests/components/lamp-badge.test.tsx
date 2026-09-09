import { describe, expect, test } from 'bun:test'
import { renderToStaticMarkup } from 'react-dom/server'
import { LampBadge } from '../../src/features/badge/lamp-badge'
import { CLEAR_LAMPS } from '../../src/shared/constants/clear-lamp'

const EXPECTED: Record<string, { label: string; token: string }> = {
    Max: { label: 'MAX', token: 'text-chart-1' },
    Perfect: { label: 'PERFECT', token: 'text-chart-1' },
    FullCombo: { label: 'FULL COMBO', token: 'text-chart-2' },
    ExHard: { label: 'EX HARD', token: 'text-chart-3' },
    Hard: { label: 'HARD', token: 'text-chart-4' },
    Normal: { label: 'NORMAL', token: 'text-chart-5' },
    Easy: { label: 'EASY', token: 'text-muted-foreground' },
    LightAssistEasy: { label: 'L-ASSIST EASY', token: 'text-muted-foreground' },
    AssistEasy: { label: 'ASSIST EASY', token: 'text-muted-foreground' },
    Failed: { label: 'FAILED', token: 'text-destructive' },
    NoPlay: { label: 'NO PLAY', token: 'text-muted-foreground' },
}

describe('LampBadge 램프 매핑', () => {
    test('11종 램프가 모두 매핑 표를 가진다', () => {
        expect(CLEAR_LAMPS.length).toBe(11)
        expect(Object.keys(EXPECTED).length).toBe(11)
    })

    for (const lamp of CLEAR_LAMPS) {
        test(`${lamp} 은(는) 라벨과 시맨틱 토큰을 함께 렌더한다`, () => {
            const markup = renderToStaticMarkup(<LampBadge clear={lamp} />)
            expect(markup).toContain(EXPECTED[lamp].label)
            expect(markup).toContain(EXPECTED[lamp].token)
        })
    }

    test('색만으로 구분하지 않도록 배지 텍스트에 램프명이 항상 들어간다', () => {
        const markups = CLEAR_LAMPS.map((lamp) => renderToStaticMarkup(<LampBadge clear={lamp} />))
        expect(markups.every((markup) => markup.replace(/<[^>]+>/g, '').trim().length > 0)).toBe(true)
    })
})
