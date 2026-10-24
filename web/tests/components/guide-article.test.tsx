import { describe, expect, test } from 'bun:test'
import { createTranslator } from 'next-intl'
import { renderToStaticMarkup } from 'react-dom/server'
import { GuideArticle } from '../../src/widgets/guide/guide-article'
import { buildGuideAdvancedSections, buildGuideSections } from '../../src/widgets/guide/guide-sections'
import { IR_SERVER_BASE_URL } from '../../src/shared/constants/server-info'
import { MESSAGES } from '../../src/shared/i18n/messages'
import { withLocale } from '../support/browse-intl'

type Locale = keyof typeof MESSAGES

const guideTranslator = (locale: Locale) => createTranslator({ locale, messages: MESSAGES[locale], namespace: 'browse.guide' })

const markup = (locale: Locale = 'ko') => renderToStaticMarkup(withLocale(<GuideArticle />, locale))

describe('연동 가이드 문서', () => {
    test('GUI 절차 순서대로 8개 절과 고급 2개 절을 렌더한다', () => {
        const html = markup()
        const sections = buildGuideSections(guideTranslator('ko'))
        const advancedSections = buildGuideAdvancedSections(guideTranslator('ko'))
        expect(sections.map((section) => section.id)).toEqual(['connect', 'account', 'ranked', 'ranking-panel', 'replay', 'sync', 'rivals'])
        expect(advancedSections.map((section) => section.id)).toEqual(['api-token', 'cli-overrides'])
        for (const section of [...sections, ...advancedSections]) expect(html).toContain(`id="${section.id}"`)
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
        for (const reason of ['autoplay', 'replay playback', 'widened judge', 'assist options', 'UNRANKED']) expect(html).toContain(reason)
    })

    test('SCRATCH AUTO 판은 랭킹에 오르고 웹 순위에 기본으로 포함되며 토글로 끌 수 있다고 안내한다', () => {
        const html = markup()
        expect(html).not.toContain('UNRANKED: auto scratch')
        for (const token of [
            'SCRATCH AUTO 포함',
            'A-SCR',
            'IR: RANK #&lt;순위&gt; (SCRATCH AUTO)',
            '기본으로 SCRATCH AUTO 포함이 켜져 있어',
            '끄면 일반 판만',
        ])
            expect(html).toContain(token)
    })

    test('어시스트 플레이는 제출 생략이 아니라 랭킹 제외로 안내한다', () => {
        const html = markup()
        for (const staleSkip of ['SKIPPED (judge window widened)', 'SKIPPED (scratch assist)']) expect(html).not.toContain(staleSkip)
        expect(html).toContain('코스')
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

    test('영어와 일본어로도 같은 절과 같은 명령 예시를 렌더한다', () => {
        for (const locale of ['en', 'ja'] as const) {
            const html = markup(locale)
            const sections = [...buildGuideSections(guideTranslator(locale)), ...buildGuideAdvancedSections(guideTranslator(locale))]
            for (const section of sections) {
                expect(html).toContain(`id="${section.id}"`)
                expect(html).toContain(section.title.replaceAll('&', '&amp;'))
            }
            expect(html).toContain(IR_SERVER_BASE_URL)
            expect(html).toContain('IR: RANK #&lt;')
            expect(html).toContain(`curl -H &#x27;Authorization: Bearer rbms_&lt;`)
            expect(html).not.toContain('가이드')
            expect(html).not.toContain('고급 (Advanced)')
        }
        expect(markup('en')).toContain('Create an account')
        expect(markup('ja')).toContain('アカウントを作成')
    })
})
