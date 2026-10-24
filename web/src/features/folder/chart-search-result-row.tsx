import type { FC } from 'react'
import { Check, Plus } from 'lucide-react'
import { useTranslations } from 'next-intl'
import { formatHash } from '@shared/lib/format'
import { Button } from '@shared/ui/button'

type ChartSearchResultRowProps = {
    md5: string
    title: string
    artist: string
    levelLabel?: string
    isKnownToServer: boolean
    isAdded: boolean
    isAddDisabled: boolean
    onAdd: () => void
}

export const ChartSearchResultRow: FC<ChartSearchResultRowProps> = ({
    md5,
    title,
    artist,
    levelLabel,
    isKnownToServer,
    isAdded,
    isAddDisabled,
    onAdd,
}) => {
    const t = useTranslations('pages.folder.searchResult')
    const tShared = useTranslations('pages.shared')
    const displayTitle = title.trim() || tShared('untitledWithHash', { hash: formatHash(md5) })

    return (
        <li className='border-border flex min-h-10 items-center gap-2 border-b py-1 pr-1.5 pl-3'>
            <div className='flex min-w-0 flex-1 flex-col'>
                <span className='truncate text-xs font-medium'>{displayTitle}</span>
                <span className='text-muted-foreground text-2xs flex min-w-0 items-center gap-1.5'>
                    {levelLabel && (
                        <span className='shrink-0 font-mono font-semibold' style={{ color: 'var(--rbms-difficulty-insane)' }}>
                            {levelLabel}
                        </span>
                    )}
                    {isKnownToServer && <span className='border-border shrink-0 border px-1 leading-tight'>IR</span>}
                    <span className='truncate'>{artist || tShared('artistMissing')}</span>
                </span>
            </div>
            {isAdded ? (
                <span className='text-muted-foreground flex shrink-0 items-center gap-1 px-2 text-xs'>
                    <Check className='size-3' aria-hidden />
                    {t('added')}
                </span>
            ) : (
                <Button variant='ghost' size='xs' aria-label={t('addAria', { title: displayTitle })} disabled={isAddDisabled} onClick={onAdd}>
                    <Plus />
                    {t('add')}
                </Button>
            )}
        </li>
    )
}
