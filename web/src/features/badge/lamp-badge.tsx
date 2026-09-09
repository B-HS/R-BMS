import type { FC } from 'react'
import { Badge } from '@shared/ui/badge'
import { CLEAR_LAMP_LABEL, type ClearLamp } from '@shared/constants/clear-lamp'
import { cn } from '@shared/lib/cn'

const LAMP_CLASS: Record<ClearLamp, string> = {
    Max: 'bg-chart-1/20 text-chart-1',
    Perfect: 'bg-chart-1/20 text-chart-1',
    FullCombo: 'bg-chart-2/20 text-chart-2',
    ExHard: 'bg-chart-3/20 text-chart-3',
    Hard: 'bg-chart-4/20 text-chart-4',
    Normal: 'bg-chart-5/20 text-chart-5',
    Easy: 'border-border text-muted-foreground',
    LightAssistEasy: 'border-border text-muted-foreground',
    AssistEasy: 'border-border text-muted-foreground',
    Failed: 'bg-destructive/20 text-destructive',
    NoPlay: 'border-border text-muted-foreground',
}

export const LampBadge: FC<{ clear: ClearLamp }> = ({ clear }) => (
    <Badge variant='outline' className={cn(LAMP_CLASS[clear])}>
        {CLEAR_LAMP_LABEL[clear]}
    </Badge>
)
