import { Suspense } from 'react'
import { getTranslations } from 'next-intl/server'
import { SessionGate } from '@widgets/auth/session-gate'
import { ShellPageLoading } from '@widgets/shell/shell-page-loading'

const SettingsLayout = async ({ children }: LayoutProps<'/settings'>) => {
    const t = await getTranslations('pages.route.settings')

    return (
        <Suspense fallback={<ShellPageLoading title={t('title')} />}>
            <SessionGate next='/settings'>{children}</SessionGate>
        </Suspense>
    )
}

export default SettingsLayout
