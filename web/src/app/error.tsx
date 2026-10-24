'use client'

import { useTranslations } from 'next-intl'
import { EmptyState } from '@features/state/empty-state'
import { Button } from '@shared/ui/button'
import { ShellPage } from '@widgets/shell/shell-page'

const ErrorPage = ({ retry }: { error: Error & { digest?: string }; retry: () => void }) => {
    const t = useTranslations('pages.route.error')
    const tShared = useTranslations('pages.shared')

    return (
        <ShellPage title={t('title')}>
            <EmptyState
                title={t('heading')}
                description={t('description')}
                className='min-h-64'
                action={
                    <Button variant='outline' size='sm' onClick={() => retry()}>
                        {tShared('retry')}
                    </Button>
                }
            />
        </ShellPage>
    )
}

export default ErrorPage
