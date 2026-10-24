import type { useTranslations } from 'next-intl'
import { IR_SERVER_BASE_URL } from '@shared/constants/server-info'

export type GuideTranslator = ReturnType<typeof useTranslations<'browse.guide'>>

export type GuideRow = {
    label: string
    description: string
}

export type GuideSection = {
    id: string
    title: string
    body: string
    steps?: string[]
    rows?: GuideRow[]
    code?: string
}

export const buildGuideSections = (t: GuideTranslator): GuideSection[] => [
    {
        id: 'connect',
        title: t('connect.title'),
        body: t('connect.body'),
        steps: [
            t('connect.steps.open'),
            t('connect.steps.network'),
            t('connect.steps.server', { url: IR_SERVER_BASE_URL }),
            t('connect.steps.player'),
            t('connect.steps.close'),
        ],
        code: t('connect.code', { url: IR_SERVER_BASE_URL }),
    },
    {
        id: 'account',
        title: t('account.title'),
        body: t('account.body'),
        rows: [
            { label: t('account.rows.playerId.label'), description: t('account.rows.playerId.description') },
            { label: t('account.rows.email.label'), description: t('account.rows.email.description') },
            { label: t('account.rows.password.label'), description: t('account.rows.password.description') },
            { label: t('account.rows.login.label'), description: t('account.rows.login.description') },
            { label: t('account.rows.register.label'), description: t('account.rows.register.description') },
            { label: t('account.rows.logout.label'), description: t('account.rows.logout.description') },
            { label: t('account.rows.account.label'), description: t('account.rows.account.description') },
        ],
    },
    {
        id: 'ranked',
        title: t('ranked.title'),
        body: t('ranked.body'),
        rows: [
            { label: t('ranked.rows.skipped.label'), description: t('ranked.rows.skipped.description') },
            { label: t('ranked.rows.widenedJudge.label'), description: t('ranked.rows.widenedJudge.description') },
            { label: t('ranked.rows.scratchAuto.label'), description: t('ranked.rows.scratchAuto.description') },
            { label: t('ranked.rows.assist.label'), description: t('ranked.rows.assist.description') },
        ],
        code: t('ranked.code'),
    },
    {
        id: 'ranking-panel',
        title: t('rankingPanel.title'),
        body: t('rankingPanel.body'),
        rows: [
            { label: t('rankingPanel.rows.layout.label'), description: t('rankingPanel.rows.layout.description') },
            { label: t('rankingPanel.rows.youRival.label'), description: t('rankingPanel.rows.youRival.description') },
            { label: t('rankingPanel.rows.replay.label'), description: t('rankingPanel.rows.replay.description') },
            { label: t('rankingPanel.rows.status.label'), description: t('rankingPanel.rows.status.description') },
        ],
    },
    {
        id: 'replay',
        title: t('replay.title'),
        body: t('replay.body'),
        rows: [
            { label: t('replay.rows.autoUpload.label'), description: t('replay.rows.autoUpload.description') },
            { label: t('replay.rows.condition.label'), description: t('replay.rows.condition.description') },
        ],
    },
    {
        id: 'sync',
        title: t('sync.title'),
        body: t('sync.body'),
        rows: [
            { label: t('sync.rows.uploaded.label'), description: t('sync.rows.uploaded.description') },
            { label: t('sync.rows.downloaded.label'), description: t('sync.rows.downloaded.description') },
            { label: t('sync.rows.requirement.label'), description: t('sync.rows.requirement.description') },
        ],
    },
    {
        id: 'rivals',
        title: t('rivals.title'),
        body: t('rivals.body'),
        rows: [
            { label: t('rivals.rows.controls.label'), description: t('rivals.rows.controls.description') },
            { label: t('rivals.rows.sync.label'), description: t('rivals.rows.sync.description') },
            { label: t('rivals.rows.display.label'), description: t('rivals.rows.display.description') },
        ],
    },
]

export const buildGuideAdvancedSections = (t: GuideTranslator): GuideSection[] => [
    {
        id: 'api-token',
        title: t('apiToken.title'),
        body: t('apiToken.body'),
        code: t('apiToken.code', { url: IR_SERVER_BASE_URL }),
    },
    {
        id: 'cli-overrides',
        title: t('cliOverrides.title'),
        body: t('cliOverrides.body'),
        code: t('cliOverrides.code', { url: IR_SERVER_BASE_URL }),
    },
]
