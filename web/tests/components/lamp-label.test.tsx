import { describe, expect, test } from 'bun:test'
import { renderToStaticMarkup } from 'react-dom/server'
import { LampLabel } from '../../src/features/lamp/lamp-label'
import { LampStackBar } from '../../src/features/lamp/lamp-stack-bar'
import { CLEAR_LAMPS, LAMP_BACKGROUND } from '../../src/shared/constants/clear-lamp'

const EXPECTED: Record<string, { label: string; token: string }> = {
    Max: { label: 'MAX', token: 'var(--rbms-lamp-max)' },
    Perfect: { label: 'PERFECT', token: 'var(--rbms-lamp-perfect)' },
    FullCombo: { label: 'FULL COMBO', token: 'var(--rbms-lamp-fullcombo)' },
    ExHard: { label: 'EX HARD', token: 'var(--rbms-lamp-exhard)' },
    Hard: { label: 'HARD', token: 'var(--rbms-lamp-hard)' },
    Normal: { label: 'CLEAR', token: 'var(--rbms-lamp-normal)' },
    Easy: { label: 'EASY', token: 'var(--rbms-lamp-easy)' },
    LightAssistEasy: { label: 'L-ASSIST', token: 'var(--rbms-lamp-light-assist)' },
    AssistEasy: { label: 'ASSIST', token: 'var(--rbms-lamp-assist)' },
    Failed: { label: 'FAILED', token: 'var(--rbms-lamp-failed)' },
    NoPlay: { label: 'NO PLAY', token: 'var(--rbms-lamp-noplay)' },
}

describe('LampLabel 램프 매핑', () => {
    test('11종 램프가 모두 스킨 색 토큰을 가진다', () => {
        expect(CLEAR_LAMPS.length).toBe(11)
        expect(Object.keys(LAMP_BACKGROUND).length).toBe(11)
    })

    for (const lamp of CLEAR_LAMPS) {
        test(`${lamp} 은(는) 이름과 스킨 색 견본을 함께 렌더한다`, () => {
            const markup = renderToStaticMarkup(<LampLabel lamp={lamp} />)
            expect(markup).toContain(EXPECTED[lamp].label)
            expect(markup).toContain(EXPECTED[lamp].token)
        })
    }

    test('색은 견본에만 쓰고 글자는 foreground 로 둔다', () => {
        const markup = renderToStaticMarkup(<LampLabel lamp='Failed' />)
        expect(markup).toContain('text-foreground')
        expect(markup).not.toContain('text-destructive')
    })

    test('누적 막대는 값이 0 인 구간을 그리지 않고 전체를 글자로도 읽어 준다', () => {
        const markup = renderToStaticMarkup(
            <LampStackBar
                lamps={[
                    { lamp: 'Hard', count: 3 },
                    { lamp: 'Easy', count: 0 },
                ]}
            />,
        )
        expect(markup).toContain('role="img"')
        expect(markup).toContain('HARD 3, EASY 0')
        expect(markup).toContain('var(--rbms-lamp-hard)')
        expect(markup).not.toContain('var(--rbms-lamp-easy)')
    })
})
