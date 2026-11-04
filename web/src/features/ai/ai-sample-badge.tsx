import type { FC } from 'react'
import { useTranslations } from 'next-intl'
import { Badge } from '@shared/ui/badge'
import type { AiEvidenceGrade, AiProfileSampleLevel } from '@entities/ai.type'

type AiSampleBadgeProps = { evidence: AiEvidenceGrade; sampleLevel: AiProfileSampleLevel }

/** The grade of a profile value (did it pass the test) and how much data stands behind it, in one text badge. */
export const AiSampleBadge: FC<AiSampleBadgeProps> = ({ evidence, sampleLevel }) => {
    const t = useTranslations('pages.ai')

    return (
        <Badge variant='outline'>
            {t(`evidence.${evidence}`)} · {t(`profile.sampleLevel.${sampleLevel}`)}
        </Badge>
    )
}
