import type { FC } from 'react'
import { useTranslations } from 'next-intl'
import { DanBadge } from '@features/dan/dan-badge'
import type { DisplayedDan } from '@entities/dan.type'

export const DanBadgeList: FC<{ dans: readonly DisplayedDan[] }> = ({ dans }) => {
    const t = useTranslations('pages.dan')

    if (dans.length === 0) return null

    return (
        <ul aria-label={t('displayed')} className='flex flex-wrap items-center gap-1'>
            {dans.map((dan) => (
                <li key={`${dan.system}:${dan.assisted}`}>
                    <DanBadge dan={dan} />
                </li>
            ))}
        </ul>
    )
}
