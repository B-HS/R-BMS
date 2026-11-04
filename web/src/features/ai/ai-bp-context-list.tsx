import type { FC } from 'react'
import { useTranslations } from 'next-intl'
import { Badge } from '@shared/ui/badge'
import { formatCount } from '@shared/lib/format'
import { useAiWeaknessLabel } from '@entities/ai.query'
import { formatAiWeaknessValue, type AiWeaknessProfile, type AiWeaknessProfileCell } from '@entities/ai.type'

type AiBpContextListProps = {
    bpContexts: readonly AiWeaknessProfileCell[]
    gaugeContexts: readonly AiWeaknessProfileCell[]
    bpRuns: AiWeaknessProfile['bp_runs']
    gauge: AiWeaknessProfile['gauge']
}

const MISSING = '—'

/**
 * Where plays break: the patterns that come right before a BAD/POOR run and inside a gauge drop more often than among all notes
 * (their lift), with the mean longest run per chart, the median notes to recover and the gauge drops per play.
 */
export const AiBpContextList: FC<AiBpContextListProps> = ({ bpContexts, gaugeContexts, bpRuns, gauge }) => {
    const t = useTranslations('pages.ai')
    const weaknessLabel = useAiWeaknessLabel()
    const orNone = (value: number | null) => (value === null ? MISSING : String(value))

    const renderList = (label: string, cells: readonly AiWeaknessProfileCell[]) => (
        <section aria-label={label} className='flex flex-col gap-1'>
            <h3 className='text-xs font-medium'>{label}</h3>
            {cells.length === 0 && <p className='text-muted-foreground text-xs'>{t('profile.contextEmpty')}</p>}
            {cells.length > 0 && (
                <ul className='flex flex-col gap-0.5 text-xs tabular-nums'>
                    {cells.map((cell) => (
                        <li key={cell.key} className='flex flex-wrap items-center gap-2'>
                            <span className='min-w-28'>{weaknessLabel(cell.key)}</span>
                            <span>{formatAiWeaknessValue(cell.key, cell.unit, cell.value)}</span>
                            <span className='text-muted-foreground'>{t('profile.contextNotes', { notes: formatCount(cell.notes) })}</span>
                            <Badge variant='outline'>{t(`evidence.${cell.evidence}`)}</Badge>
                        </li>
                    ))}
                </ul>
            )}
        </section>
    )

    return (
        <div className='flex flex-col gap-3'>
            <p className='text-muted-foreground text-xs tabular-nums'>
                {t('profile.bpRunsLine', { longest: orNone(bpRuns.longest_mean), recovery: orNone(bpRuns.recovery_median) })} ·{' '}
                {t('profile.gaugeLine', {
                    drops: orNone(gauge.drops_per_play),
                    dropNps: orNone(gauge.drop_mean_nps),
                    overallNps: orNone(gauge.overall_mean_nps),
                })}
            </p>
            <div className='grid gap-3 md:grid-cols-2'>
                {renderList(t('profile.bpContextLabel'), bpContexts)}
                {renderList(t('profile.gaugeContextLabel'), gaugeContexts)}
            </div>
        </div>
    )
}
