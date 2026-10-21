'use client'

import { useState, type FC } from 'react'
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
    { key: 'plays', label: '판 수' },
    { key: 'charts', label: '친 곡' },
    { key: 'active_days', label: '친 날' },
    { key: 'first_plays', label: '처음 친 곡' },
    { key: 'lamp_ups', label: '램프 갱신' },
    { key: 'new_clears', label: '첫 클리어' },
    { key: 'ex_ups', label: 'EX 갱신' },
] as const satisfies readonly { key: keyof WeeklyTotals; label: string }[]

const shiftWeek = (start: string, weeks: number) =>
    new Date(Date.parse(`${start}T00:00:00.000Z`) + weeks * WEEK_MS).toISOString().slice(0, ISO_DATE_LENGTH)

const formatShare = (share: number | null) => (share === null ? MISSING : `${Math.round(share * PERCENT_SCALE)}%`)

export const AiWeeklyPanel: FC = () => {
    const [week, setWeek] = useState<string | null>(null)

    const weekly = useAiWeekly(week)
    const interpret = useInterpretAiWeekly()

    const view = weekly.data
    const ai = interpret.data ?? view?.ai ?? null

    const changeWeek = (next: string) => {
        setWeek(next)
        interpret.reset()
    }

    return (
        <PanelCard title='주간 성장 요약' contentClassName='flex flex-col gap-4'>
            {weekly.isPending && <PanelSkeleton label='주간 기록 계산 중' />}
            {weekly.isError && (
                <LoadErrorState title='주간 기록을 불러오지 못했습니다' isRetrying={weekly.isFetching} onRetry={() => void weekly.refetch()} />
            )}
            {view && (
                <>
                    <div className='flex flex-wrap items-center gap-2'>
                        <Button type='button' variant='outline' size='xs' onClick={() => changeWeek(shiftWeek(view.week.start, -1))}>
                            지난주
                        </Button>
                        <span className='text-sm tabular-nums'>
                            {view.week.start} ~ {view.week.end} (UTC)
                        </span>
                        {view.week.in_progress && <Badge variant='secondary'>진행 중</Badge>}
                        <Button
                            type='button'
                            variant='outline'
                            size='xs'
                            disabled={view.week.in_progress}
                            onClick={() => changeWeek(shiftWeek(view.week.start, 1))}>
                            다음 주
                        </Button>
                    </div>
                    <table className='w-full max-w-md text-xs tabular-nums'>
                        <caption className='sr-only'>이번 주와 지난주</caption>
                        <thead>
                            <tr className='text-muted-foreground'>
                                <th scope='col' className='text-left font-normal'>
                                    항목
                                </th>
                                <th scope='col' className='text-right font-normal'>
                                    이 주
                                </th>
                                <th scope='col' className='text-right font-normal'>
                                    앞 주
                                </th>
                            </tr>
                        </thead>
                        <tbody>
                            {TOTAL_ROWS.map((row) => (
                                <tr key={row.key}>
                                    <th scope='row' className='text-left font-normal'>
                                        {row.label}
                                    </th>
                                    <td className='text-right'>{formatCount(view.totals[row.key])}</td>
                                    <td className='text-muted-foreground text-right'>{formatCount(view.previous[row.key])}</td>
                                </tr>
                            ))}
                            <tr>
                                <th scope='row' className='text-left font-normal'>
                                    FAST 비율
                                </th>
                                <td className='text-right'>{formatShare(view.totals.fast_share)}</td>
                                <td className='text-muted-foreground text-right'>{formatShare(view.previous.fast_share)}</td>
                            </tr>
                        </tbody>
                    </table>
                    {view.lamp_changes.length > 0 && (
                        <ul aria-label='램프가 오른 곡' className='flex flex-col gap-1 text-xs'>
                            {view.lamp_changes.map((change) => (
                                <li key={`${change.key}-${String(change.scratch_auto)}`} className='flex flex-wrap items-center gap-2'>
                                    {change.level && <Badge variant='outline'>{change.level}</Badge>}
                                    <span className='truncate'>{change.title || '제목 미등록'}</span>
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
                        label='AI 주간 요약'
                        meta={ai}
                        isPending={interpret.isPending}
                        error={interpret.error}
                        isBlocked={view.notice !== null}
                        onInterpret={() => interpret.mutate({ week: view.week.start })}>
                        {ai && <p className='text-sm font-medium break-keep'>{ai.output.headline}</p>}
                        {ai && <AiInsightList label='이번 주 하이라이트' insights={ai.output.highlights} />}
                        {ai && <p className='text-muted-foreground text-xs break-keep'>다음 주: {ai.output.next_week}</p>}
                    </AiInterpretSection>
                </>
            )}
        </PanelCard>
    )
}
