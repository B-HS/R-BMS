import type { FC } from 'react'
import { formatUrlHost } from '@features/table/format-url-host'
import { TableExternalLink } from '@features/table/table-external-link'
import { cn } from '@shared/lib/cn'
import { formatCount } from '@shared/lib/format'

type TableSourceNoticeProps = {
    variant: 'summary' | 'full'
    name: string
    mirrorUrl: string
    originalUrl: string
    lastUpdate: string
    snapshotDate: string
    entryCount: number
    contactUrl: string
    className?: string
}

export const TableSourceNotice: FC<TableSourceNoticeProps> = ({
    variant,
    name,
    mirrorUrl,
    originalUrl,
    lastUpdate,
    snapshotDate,
    entryCount,
    contactUrl,
    className,
}) => {
    if (variant === 'summary')
        return (
            <p className={cn('text-muted-foreground bg-card px-3 py-2 text-xs leading-5 break-keep', className)}>
                이 페이지는 <TableExternalLink href={mirrorUrl} label={formatUrlHost(mirrorUrl)} appearance='text' /> 가 보관한 {name}의
                사본입니다(원본 <TableExternalLink href={originalUrl} label={formatUrlHost(originalUrl)} appearance='text' />, 원본 갱신 {lastUpdate}
                ). 요청이 있으면 내립니다: <TableExternalLink href={contactUrl} label='삭제 요청' appearance='text' />
            </p>
        )

    return (
        <footer className={cn('text-muted-foreground bg-card flex flex-col gap-2 border-t px-3 py-3 text-xs leading-5', className)}>
            <p className='break-keep'>
                출처: {name} · <TableExternalLink href={mirrorUrl} label={formatUrlHost(mirrorUrl)} appearance='text' /> 보관본의 사본({snapshotDate}{' '}
                수집, {formatCount(entryCount)}곡, 원본 갱신 {lastUpdate}) · 처음 공개된 곳{' '}
                <TableExternalLink href={originalUrl} label={originalUrl} appearance='text' />
            </p>
            <p className='break-keep'>
                곡·레벨·링크는 원작자와 표를 만든 분들의 것이며 내용을 고치지 않았습니다. 곡 파일은 이 사이트가 올려 두지 않고 원래 주소로 잇기만
                합니다. 원 운영자가 요청하면 사본을 내립니다. 연락: <TableExternalLink href={contactUrl} label='GitHub 이슈' appearance='text' />
            </p>
            <p lang='en' className='break-words'>
                This page mirrors the archived {name} from {formatUrlHost(mirrorUrl)} (snapshot of {snapshotDate}, {formatCount(entryCount)} charts,
                last updated by the original on {lastUpdate}), first published at {originalUrl}. All charts, levels and links belong to their original
                authors and the table&apos;s compilers; the data is provided unchanged for use with BMS players and will be removed on request.
            </p>
        </footer>
    )
}
