import type { FC, ReactNode } from 'react'
import { LampLabel } from '@features/lamp/lamp-label'
import { Badge } from '@shared/ui/badge'
import { PERCENT_SCALE } from '@shared/constants/ui'
import { formatCount } from '@shared/lib/format'
import { aiPatternTagLabel, type AiChartCandidate } from '@entities/ai.type'

const EX_RATE_DIGITS = 2
const MISSING = '—'

type AiChartCandidateListProps = {
    label: string
    symbol: string
    candidates: readonly AiChartCandidate[]
    renderExtra?: (candidate: AiChartCandidate) => ReactNode
}

export const AiChartCandidateList: FC<AiChartCandidateListProps> = ({ label, symbol, candidates, renderExtra }) => (
    <ol aria-label={label} className='flex flex-col gap-1'>
        {candidates.map((candidate) => (
            <li key={candidate.md5} className='border-border flex flex-col gap-1 border-b py-1.5 text-xs last:border-b-0'>
                <span className='flex flex-wrap items-center gap-2'>
                    <Badge variant='outline' className='tabular-nums'>
                        {symbol}
                        {candidate.level}
                    </Badge>
                    <span className='min-w-0 truncate text-sm'>{candidate.title || '제목 미등록'}</span>
                    <span className='text-muted-foreground truncate'>{candidate.artist}</span>
                </span>
                <span className='text-muted-foreground flex flex-wrap items-center gap-3'>
                    <LampLabel lamp={candidate.lamp} assistGrade={candidate.assist_grade} />
                    <span className='tabular-nums'>EX {candidate.ex_rate === null ? MISSING : `${candidate.ex_rate.toFixed(EX_RATE_DIGITS)}%`}</span>
                    <span className='tabular-nums'>BP {candidate.bp === null ? MISSING : formatCount(candidate.bp)}</span>
                    {candidate.tags.map((tag) => (
                        <span key={tag.tag}>
                            {aiPatternTagLabel(tag.tag)} {Math.round(tag.ratio * PERCENT_SCALE)}%
                        </span>
                    ))}
                </span>
                {renderExtra?.(candidate)}
            </li>
        ))}
    </ol>
)
