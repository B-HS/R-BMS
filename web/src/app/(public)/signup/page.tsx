import { getTranslations } from 'next-intl/server'
import { buildPageMetadata } from '@shared/lib/metadata'
import { AuthCard } from '@widgets/auth/auth-card'
import { SignupForm } from '@widgets/auth/signup-form'
import { ShellPage } from '@widgets/shell/shell-page'

export const generateMetadata = async () => {
    const t = await getTranslations('pages.route.signup')
    return buildPageMetadata({ title: t('title'), description: t('metaDescription'), path: '/signup' })
}

const SignupPage = async () => {
    const t = await getTranslations('pages.route.signup')

    return (
        <ShellPage title={t('title')} contentClassName='gap-0'>
            <AuthCard title={t('title')} description={t('description')}>
                <SignupForm />
            </AuthCard>
        </ShellPage>
    )
}

export default SignupPage
