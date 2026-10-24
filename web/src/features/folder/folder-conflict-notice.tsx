import type { FC } from 'react'
import { TriangleAlert } from 'lucide-react'
import { useTranslations } from 'next-intl'
import { formatDateTime } from '@shared/lib/format'
import { Alert, AlertDescription, AlertTitle } from '@shared/ui/alert'
import { Button } from '@shared/ui/button'

type FolderConflictSummary = {
    serverName: string
    serverItemCount: number
    serverUpdatedAt: number
    onlyMine: number
    onlyServer: number
    isOrderDifferent: boolean
    isNameDifferent: boolean
    isDescriptionDifferent: boolean
}

type FolderConflictNoticeProps = {
    summary: FolderConflictSummary
    isSaving: boolean
    onUseServer: () => void
    onOverwrite: () => void
}

export const FolderConflictNotice: FC<FolderConflictNoticeProps> = ({ summary, isSaving, onUseServer, onOverwrite }) => {
    const t = useTranslations('pages.folder.conflict')
    const differences = [
        summary.onlyServer > 0 && t('onlyServer', { count: summary.onlyServer }),
        summary.onlyMine > 0 && t('onlyMine', { count: summary.onlyMine }),
        summary.isOrderDifferent && t('order'),
        summary.isNameDifferent && t('name', { name: summary.serverName }),
        summary.isDescriptionDifferent && t('description'),
    ].filter((value) => typeof value === 'string')

    return (
        <Alert role='alert' variant='destructive' className='rounded-none border-x-0 border-t-0'>
            <TriangleAlert aria-hidden />
            <AlertTitle>{t('title')}</AlertTitle>
            <AlertDescription className='flex flex-col gap-2'>
                <p className='text-foreground'>
                    {t('summary', { updatedAt: formatDateTime(summary.serverUpdatedAt), count: summary.serverItemCount })}{' '}
                    {differences.length > 0 ? t('differences', { list: differences.join(', ') }) : t('same')}
                </p>
                <div className='flex flex-wrap gap-1'>
                    <Button variant='outline' size='sm' disabled={isSaving} onClick={onUseServer}>
                        {t('useServer')}
                    </Button>
                    <Button variant='destructive' size='sm' disabled={isSaving} onClick={onOverwrite}>
                        {t('overwrite')}
                    </Button>
                </div>
            </AlertDescription>
        </Alert>
    )
}
