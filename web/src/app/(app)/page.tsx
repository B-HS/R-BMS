import { Suspense } from 'react'
import type { Metadata } from 'next'
import { getTranslations } from 'next-intl/server'
import { DASHBOARD_GRID_CLASS_NAME } from '@shared/constants/dashboard'
import { SITE_NAME } from '@shared/constants/site'
import { HomeBranch } from '@widgets/home/home-branch'
import { HomeBranchSkeleton } from '@widgets/home/home-branch-skeleton'
import { ShellPage } from '@widgets/shell/shell-page'

export const generateMetadata = async (): Promise<Metadata> => {
    const t = await getTranslations('pages')
    return {
        title: { absolute: `${SITE_NAME} — ${t('site.tagline')}` },
        description: t('site.description'),
        alternates: { canonical: '/' },
    }
}

const HomePage = async () => {
    const t = await getTranslations('pages.route.home')

    return (
        <ShellPage title={t('title')} description={t('description')} contentClassName='@container/dashboard gap-0'>
            <div className={DASHBOARD_GRID_CLASS_NAME}>
                <Suspense fallback={<HomeBranchSkeleton />}>
                    <HomeBranch />
                </Suspense>
            </div>
        </ShellPage>
    )
}

export default HomePage
