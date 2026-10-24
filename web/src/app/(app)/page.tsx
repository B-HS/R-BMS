import { Suspense } from 'react'
import type { Metadata } from 'next'
import { getTranslations } from 'next-intl/server'
import { DashboardPanel } from '@features/dashboard/dashboard-panel'
import { DashboardPanelLoading } from '@features/dashboard/dashboard-panel-loading'
import { DASHBOARD_FILL_PANEL_CLASS_NAME, DASHBOARD_FULL_PANEL_CLASS_NAME, DASHBOARD_GRID_CLASS_NAME } from '@shared/constants/dashboard'
import { SITE_NAME } from '@shared/constants/site'
import { HomeContent } from '@widgets/home/home-content'
import { HomeIntroPanel } from '@widgets/home/home-intro-panel'
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
                <HomeIntroPanel />
                <Suspense
                    fallback={
                        <>
                            <DashboardPanel title={t('serverStatus')} className={DASHBOARD_FILL_PANEL_CLASS_NAME}>
                                <DashboardPanelLoading label={t('serverStatusLoading')} />
                            </DashboardPanel>
                            <DashboardPanel title={t('recentActivity')} className={DASHBOARD_FULL_PANEL_CLASS_NAME}>
                                <DashboardPanelLoading label={t('recentActivityLoading')} className='min-h-64' />
                            </DashboardPanel>
                        </>
                    }>
                    <HomeContent />
                </Suspense>
            </div>
        </ShellPage>
    )
}

export default HomePage
