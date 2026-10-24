import { Suspense } from 'react'
import { getTranslations } from 'next-intl/server'
import { SessionGate } from '@widgets/auth/session-gate'
import { ShellPageLoading } from '@widgets/shell/shell-page-loading'

const FoldersLayout = async ({ children }: LayoutProps<'/folders'>) => {
    const t = await getTranslations('pages.route.folders')

    return (
        <Suspense fallback={<ShellPageLoading title={t('title')} />}>
            <SessionGate next='/folders'>{children}</SessionGate>
        </Suspense>
    )
}

export default FoldersLayout
