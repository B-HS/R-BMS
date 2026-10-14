import type { FC } from 'react'
import Link from 'next/link'
import { EmptyState } from '@features/state/empty-state'
import { Button } from '@shared/ui/button'

type NotFoundNoticeProps = {
    title?: string
    description?: string
    actionHref?: string
    actionLabel?: string
}

export const NotFoundNotice: FC<NotFoundNoticeProps> = ({
    title = '페이지를 찾을 수 없습니다',
    description = '주소가 바뀌었거나 없는 페이지입니다.',
    actionHref = '/',
    actionLabel = '홈으로',
}) => (
    <EmptyState
        eyebrow='404'
        title={title}
        description={description}
        className='min-h-64'
        action={
            <Button asChild variant='outline' size='sm'>
                <Link href={actionHref}>{actionLabel}</Link>
            </Button>
        }
    />
)
