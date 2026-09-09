import { describe, expect, test } from 'bun:test'
import { renderToStaticMarkup } from 'react-dom/server'
import { ReplayWaterfall } from '@widgets/replay/replay-waterfall'

describe('ReplayWaterfall µs 좌표 변환', () => {
    test('이벤트 x 좌표를 레인 헤더 폭 + 초당 96px 로 배치한다', () => {
        const html = renderToStaticMarkup(
            <ReplayWaterfall
                events={[
                    { t_us: 0, lane: 0, press: true },
                    { t_us: 2_000_000, lane: 1, press: false },
                ]}
                durationUs={2_000_000}
            />,
        )
        expect(html).toContain('x="64"')
        expect(html).toContain('x="256"')
    })

    test('레인 수와 이벤트 수를 aria-label 로 요약한다', () => {
        const html = renderToStaticMarkup(
            <ReplayWaterfall
                events={[
                    { t_us: 0, lane: 0, press: true },
                    { t_us: 500_000, lane: 3, press: false },
                ]}
                durationUs={500_000}
            />,
        )
        expect(html).toContain('aria-label="레인 2개에 걸친 입력 이벤트 2건, 총 길이 1.0초"')
    })

    test('press 와 release 를 서로 다른 시리즈 토큰 색으로 칠한다', () => {
        const pressOnly = renderToStaticMarkup(<ReplayWaterfall events={[{ t_us: 0, lane: 0, press: true }]} durationUs={1_000_000} />)
        expect(pressOnly).toContain('var(--color-chart-2)')
        expect(pressOnly).not.toContain('var(--color-chart-4)')

        const releaseOnly = renderToStaticMarkup(<ReplayWaterfall events={[{ t_us: 0, lane: 0, press: false }]} durationUs={1_000_000} />)
        expect(releaseOnly).toContain('var(--color-chart-4)')
    })

    test('레인 0 은 스크래치로 표기한다', () => {
        const html = renderToStaticMarkup(<ReplayWaterfall events={[{ t_us: 0, lane: 0, press: true }]} durationUs={1_000_000} />)
        expect(html).toContain('>SC</text>')
    })
})
