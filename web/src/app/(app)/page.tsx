import { Suspense } from 'react'
import type { Metadata } from 'next'
import { DashboardPanel } from '@features/dashboard/dashboard-panel'
import { DashboardPanelLoading } from '@features/dashboard/dashboard-panel-loading'
import { DASHBOARD_FILL_PANEL_CLASS_NAME, DASHBOARD_FULL_PANEL_CLASS_NAME, DASHBOARD_GRID_CLASS_NAME } from '@shared/constants/dashboard'
import { SITE_DESCRIPTION, SITE_NAME } from '@shared/constants/site'
import { HomeContent } from '@widgets/home/home-content'
import { HomeIntroPanel } from '@widgets/home/home-intro-panel'
import { ShellPage } from '@widgets/shell/shell-page'

export const metadata: Metadata = {
    title: { absolute: `${SITE_NAME} — BMS 플레이어의 점수 서버` },
    description: SITE_DESCRIPTION,
    alternates: { canonical: '/' },
}

const HomePage = () => (
    <ShellPage title='홈' description='R-BMS 플레이어의 점수 서버' contentClassName='@container/dashboard gap-0'>
        <div className={DASHBOARD_GRID_CLASS_NAME}>
            <HomeIntroPanel />
            <Suspense
                fallback={
                    <>
                        <DashboardPanel title='서버 현황' className={DASHBOARD_FILL_PANEL_CLASS_NAME}>
                            <DashboardPanelLoading label='서버 현황 불러오는 중' />
                        </DashboardPanel>
                        <DashboardPanel title='최근 활동' className={DASHBOARD_FULL_PANEL_CLASS_NAME}>
                            <DashboardPanelLoading label='최근 활동 불러오는 중' className='min-h-64' />
                        </DashboardPanel>
                    </>
                }>
                <HomeContent />
            </Suspense>
        </div>
    </ShellPage>
)

export default HomePage
