import { getTranslations } from 'next-intl/server'
import { buildPageMetadata } from '@shared/lib/metadata'
import { AuthCard } from '@widgets/auth/auth-card'
import { LoginForm } from '@widgets/auth/login-form'
import { ShellPage } from '@widgets/shell/shell-page'

export const generateMetadata = async () => {
    const t = await getTranslations('pages.route.login')
    return buildPageMetadata({ title: t('title'), description: t('metaDescription'), path: '/login' })
}

const LoginPage = async () => {
    const t = await getTranslations('pages.route.login')

    return (
        <ShellPage title={t('title')} contentClassName='gap-0'>
            <AuthCard title={t('title')} description={t('description')}>
                <LoginForm />
            </AuthCard>
        </ShellPage>
    )
}

export default LoginPage
