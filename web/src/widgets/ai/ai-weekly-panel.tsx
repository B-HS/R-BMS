'use client'

import { useState, type FC } from 'react'
import { useTranslations } from 'next-intl'
import { AiDataNotice } from '@features/ai/ai-data-notice'
import { AiInsightList } from '@features/ai/ai-insight-list'
import { AiInterpretSection } from '@features/ai/ai-interpret-section'
import { LampLabel } from '@features/lamp/lamp-label'
import { PanelCard } from '@features/shell/panel-card'
import { LoadErrorState } from '@features/state/load-error-state'
import { PanelSkeleton } from '@features/state/panel-skeleton'
import { Badge } from '@shared/ui/badge'
import { Button } from '@shared/ui/button'
import { PERCENT_SCALE } from '@shared/constants/ui'
import { formatCount } from '@shared/lib/format'
import { useAiWeekly, useInterpretAiWeekly } from '@entities/ai.query'
import type { AiWeeklyView } from '@entities/ai.type'

const DAY_MS = 86_400_000
const WEEK_MS = 7 * DAY_MS
const ISO_DATE_LENGTH = 10
const MISSING = '—'

type WeeklyTotals = AiWeeklyView['totals']

const TOTAL_ROWS = [
    { key: 'plays', labelKey: 'plays' },
    { key: 'charts', labelKey: 'charts' },
    { key: 'active_days', labelKey: 'activeDays' },
    { key: 'first_plays', labelKey: 'firstPlays' },
    { key: 'lamp_ups', labelKey: 'lampUps' },
    { key: 'new_clears', labelKey: 'newClears' },
    { key: 'ex_ups', labelKey: 'exUps' },
] as const satisfies readonly { key: keyof WeeklyTotals; labelKey: string }[]

const shiftWeek = (start: string, weeks: number) =>
    new Date(Date.parse(`${start}T00:00:00.000Z`) + weeks * WEEK_MS).toISOString().slice(0, ISO_DATE_LENGTH)

const formatShare = (share: number | null) => (share === null ? MISSING : `${Math.round(share * PERCENT_SCALE)}%`)

export const AiWeeklyPanel: FC = () => {
    const [week, setWeek] = useState<string | null>(null)

    const t = useTranslations('ai.weekly')
    const weekly = useAiWeekly(week)
    const interpret = useInterpretAiWeekly()

    const view = weekly.data
    const ai = interpret.data ?? view?.ai ?? null

    const changeWeek = (next: string) => {
        setWeek(next)
        interpret.reset()
    }

    return (
        <PanelCard title={t('title')} contentClassName='flex flex-col gap-4'>
            {weekly.isPending && <PanelSkeleton label={t('loading')} />}
            {weekly.isError && <LoadErrorState title={t('loadError')} isRetrying={weekly.isFetching} onRetry={() => void weekly.refetch()} />}
            {view && (
                <>
                    <div className='flex flex-wrap items-center gap-2'>
                        <Button type='button' variant='outline' size='xs' onClick={() => changeWeek(shiftWeek(view.week.start, -1))}>
                            {t('previousWeek')}
                        </Button>
                        <span className='text-sm tabular-nums'>
                            {view.week.start} ~ {view.week.end} (UTC)
                        </span>
                        {view.week.in_progress && <Badge variant='secondary'>{t('inProgress')}</Badge>}
                        <Button
                            type='button'
                            variant='outline'
                            size='xs'
                            disabled={view.week.in_progress}
                            onClick={() => changeWeek(shiftWeek(view.week.start, 1))}>
                            {t('nextWeek')}
                        </Button>
                    </div>
                    <table className='w-full max-w-md text-xs tabular-nums'>
                        <caption className='sr-only'>{t('caption')}</caption>
                        <thead>
                            <tr className='text-muted-foreground'>
                                <th scope='col' className='text-left font-normal'>
                                    {t('itemHeader')}
                                </th>
                                <th scope='col' className='text-right font-normal'>
                                    {t('thisWeekHeader')}
                                </th>
                                <th scope='col' className='text-right font-normal'>
                                    {t('previousWeekHeader')}
                                </th>
                            </tr>
                        </thead>
                        <tbody>
                            {TOTAL_ROWS.map((row) => (
                                <tr key={row.key}>
                                    <th scope='row' className='text-left font-normal'>
                                        {t(`totals.${row.labelKey}`)}
                                    </th>
                                    <td className='text-right'>{formatCount(view.totals[row.key])}</td>
                                    <td className='text-muted-foreground text-right'>{formatCount(view.previous[row.key])}</td>
                                </tr>
                            ))}
                            <tr>
                                <th scope='row' className='text-left font-normal'>
                                    {t('fastShare')}
                                </th>
                                <td className='text-right'>{formatShare(view.totals.fast_share)}</td>
                                <td className='text-muted-foreground text-right'>{formatShare(view.previous.fast_share)}</td>
                            </tr>
                        </tbody>
                    </table>
                    {view.lamp_changes.length > 0 && (
                        <ul aria-label={t('lampChangesLabel')} className='flex flex-col gap-1 text-xs'>
                            {view.lamp_changes.map((change) => (
                                <li key={`${change.key}-${String(change.scratch_auto)}`} className='flex flex-wrap items-center gap-2'>
                                    {change.level && <Badge variant='outline'>{change.level}</Badge>}
                                    <span className='truncate'>{change.title || t('untitled')}</span>
                                    <LampLabel lamp={change.from_lamp} assistGrade={change.from_assist_grade} />
                                    <span aria-hidden>→</span>
                                    <LampLabel lamp={change.to_lamp} assistGrade={change.to_assist_grade} />
                                    {change.scratch_auto && <Badge variant='secondary'>SCRATCH AUTO</Badge>}
                                </li>
                            ))}
                        </ul>
                    )}
                    {view.notice && <AiDataNotice need={view.notice.need} />}
                    <AiInterpretSection
                        label={t('aiLabel')}
                        meta={ai}
                        isPending={interpret.isPending}
                        error={interpret.error}
                        isBlocked={view.notice !== null}
                        onInterpret={() => interpret.mutate({ week: view.week.start })}>
                        {ai && <p className='text-sm font-medium break-keep'>{ai.output.headline}</p>}
                        {ai && <AiInsightList label={t('highlightsLabel')} insights={ai.output.highlights} />}
                        {ai && <p className='text-muted-foreground text-xs break-keep'>{t('nextWeekNote', { text: ai.output.next_week })}</p>}
                    </AiInterpretSection>
                </>
            )}
        </PanelCard>
    )
}
