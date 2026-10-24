import type { FC } from 'react'
import Link from 'next/link'
import { useTranslations } from 'next-intl'
import { EmptyState } from '@features/state/empty-state'
import { Button } from '@shared/ui/button'

type NotFoundNoticeProps = {
    title?: string
    description?: string
    actionHref?: string
    actionLabel?: string
}

export const NotFoundNotice: FC<NotFoundNoticeProps> = ({ title, description, actionHref = '/', actionLabel }) => {
    const t = useTranslations('pages.state.notFound')

    return (
        <EmptyState
            eyebrow='404'
            title={title ?? t('title')}
            description={description ?? t('description')}
            className='min-h-64'
            action={
                <Button asChild variant='outline' size='sm'>
                    <Link href={actionHref}>{actionLabel ?? t('home')}</Link>
                </Button>
            }
        />
    )
}
