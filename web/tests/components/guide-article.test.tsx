import { describe, expect, test } from 'bun:test'
import { renderToStaticMarkup } from 'react-dom/server'
import { GuideArticle } from '../../src/widgets/guide/guide-article'
import { GUIDE_ADVANCED_SECTIONS, GUIDE_SECTIONS } from '../../src/widgets/guide/guide-sections'
import { IR_SERVER_BASE_URL } from '../../src/shared/constants/server-info'

const markup = () => renderToStaticMarkup(<GuideArticle />)

describe('연동 가이드 문서', () => {
    test('GUI 절차 순서대로 8개 절과 고급 2개 절을 렌더한다', () => {
        const html = markup()
        expect(GUIDE_SECTIONS.map((section) => section.id)).toEqual(['connect', 'account', 'ranked', 'ranking-panel', 'replay', 'sync', 'rivals'])
        expect(GUIDE_ADVANCED_SECTIONS.map((section) => section.id)).toEqual(['api-token', 'cli-overrides'])
        for (const section of [...GUIDE_SECTIONS, ...GUIDE_ADVANCED_SECTIONS]) expect(html).toContain(`id="${section.id}"`)
    })

    test('설정 화면 진입 경로와 서버 주소를 안내한다', () => {
        const html = markup()
        expect(html).toContain('SETTINGS')
        expect(html).toContain('NETWORK')
        expect(html).toContain('SERVER URL')
        expect(html).toContain(IR_SERVER_BASE_URL)
    })

    test('계정 조작은 CLI 가 아니라 NETWORK 탭 행으로 설명한다', () => {
        const html = markup()
        for (const row of ['PLAYER ID', 'EMAIL', 'PASSWORD', 'LOGIN', 'REGISTER', 'LOGOUT', 'ACCOUNT']) expect(html).toContain(row)
    })

    test('랭킹 제외 조건을 결정 12 기준으로 담는다', () => {
        const html = markup()
        for (const reason of ['autoplay', 'replay playback', 'judge window widened', 'scratch assist', 'UNRANKED']) expect(html).toContain(reason)
    })

    test('랭킹 패널 · 리플레이 · 설정 동기화 · 라이벌 조작을 담는다', () => {
        const html = markup()
        for (const token of ['IR RANKING', 'REP', 'AUTO UPLOAD REPLAY', 'SYNC SETTINGS', 'conflict: server newer, download first', 'RIVALS'])
            expect(html).toContain(token)
    })

    test('CLI 플래그는 고급 절에서만 언급한다', () => {
        const html = markup()
        const advancedIndex = html.indexOf('고급 (Advanced)')
        expect(advancedIndex).toBeGreaterThan(0)
        for (const flag of ['--server', '--player']) {
            expect(html).toContain(flag)
            expect(html.indexOf(flag)).toBeGreaterThan(advancedIndex)
        }
        expect(html.slice(0, advancedIndex)).not.toContain('rbms-player --')
    })

    test('Bearer 토큰 사용법을 고급 절에 담는다', () => {
        expect(markup()).toContain('Bearer rbms_')
    })
})
