import type { FC } from 'react'
import { Check, Plus } from 'lucide-react'
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
    const displayTitle = title.trim() || `제목 미등록 ${formatHash(md5)}`

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
                    <span className='truncate'>{artist || '아티스트 미등록'}</span>
                </span>
            </div>
            {isAdded ? (
                <span className='text-muted-foreground flex shrink-0 items-center gap-1 px-2 text-xs'>
                    <Check className='size-3' aria-hidden />
                    추가됨
                </span>
            ) : (
                <Button variant='ghost' size='xs' aria-label={`${displayTitle} 추가`} disabled={isAddDisabled} onClick={onAdd}>
                    <Plus />
                    추가
                </Button>
            )}
        </li>
    )
}
