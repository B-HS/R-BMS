import type { FC } from 'react'
import { TriangleAlert } from 'lucide-react'
import { formatCount, formatDateTime } from '@shared/lib/format'
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
    const differences = [
        summary.onlyServer > 0 && `서버에만 있는 곡 ${formatCount(summary.onlyServer)}곡`,
        summary.onlyMine > 0 && `이 화면에만 있는 곡 ${formatCount(summary.onlyMine)}곡`,
        summary.isOrderDifferent && '곡 순서',
        summary.isNameDifferent && `이름(서버: ${summary.serverName})`,
        summary.isDescriptionDifferent && '설명',
    ].filter((value) => typeof value === 'string')

    return (
        <Alert role='alert' variant='destructive' className='rounded-none border-x-0 border-t-0'>
            <TriangleAlert aria-hidden />
            <AlertTitle>다른 곳에서 먼저 고친 폴더입니다</AlertTitle>
            <AlertDescription className='flex flex-col gap-2'>
                <p className='text-foreground'>
                    서버의 폴더는 {formatDateTime(summary.serverUpdatedAt)} 에 고쳐졌고 {formatCount(summary.serverItemCount)}곡입니다.{' '}
                    {differences.length > 0 ? `이 화면과 다른 곳: ${differences.join(', ')}.` : '곡 목록은 이 화면과 같습니다.'}
                </p>
                <div className='flex flex-wrap gap-1'>
                    <Button variant='outline' size='sm' disabled={isSaving} onClick={onUseServer}>
                        서버 내용 불러오기 (내 변경 버림)
                    </Button>
                    <Button variant='destructive' size='sm' disabled={isSaving} onClick={onOverwrite}>
                        내 변경으로 덮어쓰기
                    </Button>
                </div>
            </AlertDescription>
        </Alert>
    )
}
