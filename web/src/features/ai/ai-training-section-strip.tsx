import type { FC } from 'react'
import { useTranslations } from 'next-intl'
import { cn } from '@shared/lib/cn'
import { PERCENT_SCALE } from '@shared/constants/ui'
import { useAiTrainingFocusLabel } from '@entities/ai.query'
import type { AiTrainingChartDetail, AiTrainingPatternKind } from '@entities/ai.type'

type AiTrainingSection = AiTrainingChartDetail['sections'][number]

const MS_PER_SECOND = 1000
const SECONDS_PER_MINUTE = 60
const NPS_DECIMALS = 1
const CLOCK_SECONDS_WIDTH = 2
const MIN_BAR_PERCENT = 4
const MIN_SCALE_NPS = 1

const KIND_FILL_CLASS = {
    stream: 'bg-chart-1/40 border-chart-1',
    chord_stream: 'bg-chart-1/40 border-chart-1',
    burst: 'bg-chart-1/40 border-chart-1',
    trill: 'bg-chart-2/40 border-chart-2',
    stair: 'bg-chart-2/40 border-chart-2',
    jack: 'bg-chart-3/40 border-chart-3',
    chord_jack: 'bg-chart-3/40 border-chart-3',
    scratch_solo: 'bg-chart-4/40 border-chart-4',
    scratch_mixed: 'bg-chart-4/40 border-chart-4',
    scratch_run: 'bg-chart-4/40 border-chart-4',
    ln_hold: 'bg-chart-5/40 border-chart-5',
    ln_release: 'bg-chart-5/40 border-chart-5',
    rest: 'bg-muted border-border',
} satisfies Record<AiTrainingPatternKind, string>

const clockOf = (ms: number) => {
    const seconds = Math.floor(ms / MS_PER_SECOND)
    return `${Math.floor(seconds / SECONDS_PER_MINUTE)}:${String(seconds % SECONDS_PER_MINUTE).padStart(CLOCK_SECONDS_WIDTH, '0')}`
}

/**
 * The sections of one training chart (plan 2026-11-04 7.3): a band whose segment widths follow the section lengths, whose heights
 * follow the notes per second the synthesised chart really has and whose line marks the target density, coloured by pattern kind,
 * and under it the same sections as a list. The band is a picture only; the list carries every number.
 */
export const AiTrainingSectionStrip: FC<{ sections: readonly AiTrainingSection[] }> = ({ sections }) => {
    const t = useTranslations('pages.ai.training')
    const focusLabel = useAiTrainingFocusLabel()
    const scaleNps = Math.max(MIN_SCALE_NPS, ...sections.flatMap((section) => [section.target_nps, section.actual_nps]))

    return (
        <div className='flex flex-col gap-3'>
            <ol aria-hidden className='flex h-24 w-full items-end gap-px'>
                {sections.map((section) => (
                    <li
                        key={section.index}
                        style={{ flexGrow: section.to_ms - section.from_ms }}
                        className='relative flex h-full min-w-0 basis-0 items-end'>
                        <span
                            className={cn('text-2xs flex w-full items-start justify-center border-b-2 tabular-nums', KIND_FILL_CLASS[section.kind])}
                            style={{ height: `${Math.max(MIN_BAR_PERCENT, (section.actual_nps / scaleNps) * PERCENT_SCALE)}%` }}>
                            {section.index + 1}
                        </span>
                        <span
                            className='border-foreground/70 absolute inset-x-0 border-t border-dashed'
                            style={{ bottom: `${(section.target_nps / scaleNps) * PERCENT_SCALE}%` }}
                        />
                    </li>
                ))}
            </ol>
            <p className='text-muted-foreground text-2xs break-keep'>{t('strip.hint')}</p>
            <ul aria-label={t('strip.listLabel')} className='flex flex-col'>
                {sections.map((section) => (
                    <li key={section.index} className='border-border flex flex-col gap-0.5 border-b py-1.5 text-xs'>
                        <p className='flex flex-wrap items-center gap-x-2 gap-y-0.5 tabular-nums'>
                            <span aria-hidden className={cn('inline-block size-2.5 border-b-2', KIND_FILL_CLASS[section.kind])} />
                            <span className='font-medium'>{section.index + 1}</span>
                            <span className='font-medium break-keep'>{focusLabel(section.focus_key)}</span>
                            <span className='text-muted-foreground'>
                                {t(`kind.${section.kind}`)} · {t(`laneHint.${section.lane_hint}`)} · {clockOf(section.from_ms)}–
                                {clockOf(section.to_ms)} ·{' '}
                                {t('strip.nps', {
                                    target: section.target_nps.toFixed(NPS_DECIMALS),
                                    actual: section.actual_nps.toFixed(NPS_DECIMALS),
                                })}
                            </span>
                        </p>
                        {section.intent && <p className='text-muted-foreground break-keep'>{section.intent}</p>}
                    </li>
                ))}
            </ul>
        </div>
    )
}
