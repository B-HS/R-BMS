import { Suspense } from 'react'
import { getTranslations } from 'next-intl/server'
import { SessionGate } from '@widgets/auth/session-gate'
import { ShellPageLoading } from '@widgets/shell/shell-page-loading'

const AdminLayout = async ({ children }: LayoutProps<'/admin'>) => {
    const t = await getTranslations('pages.route.admin')

    return (
        <Suspense fallback={<ShellPageLoading title={t('title')} />}>
            <SessionGate next='/admin/builds' requireAdmin>
                {children}
            </SessionGate>
        </Suspense>
    )
}

export default AdminLayout
